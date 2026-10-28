//! M5 stage 1, list `battle` (`ffx_battle::m5_battle`) against the
//! original: on the image alone and on the snapshots (`FFX_SNAPSHOT`),
//! results and every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! None of these functions ran in a coverage run (debug menu helpers,
//! the PS2 CD layer's disc checks and sector read), so every branch is
//! reached with synthetic data, on the image and on the snapshots alike
//! (the snapshots add the game's real memory around it). Callees are
//! replaced in both oracles by recording stubs: each call appends its
//! tag and its arguments to a log in memory, so the diff checks that both
//! made the same calls in the same order with the same words; stubs pop
//! like the originals (all cdecl here). Callees the tests leave original
//! run as the game's code in both.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

use std::path::PathBuf;

use ffx_battle::m5_battle::bridge as mb;
use ffx_bridge::Mem;
use ffx_oracle::{Conv, Oracle, Pages};

/// `graphicFontDraw`'s body: no test here calls it, so the stubs are
/// written over it.
const CAVE: u32 = 0x0064_0660;
const CAVE_LEN: u32 = 1760;
/// After the stubs: `mov eax, POISON; ret`, called in the port's oracle
/// before each callee, so a callee leaving `eax` alone shows the poison.
const POISON_AT: u32 = CAVE + CAVE_LEN;
const POISON: u32 = 0xdead_beef;

/// Deterministic xorshift stream.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
    fn chance(&mut self, n: u32) -> bool {
        self.below(n) == 0
    }
    /// A word: small values, 0, 1, -1 and anything.
    fn word(&mut self) -> u32 {
        match self.below(5) {
            0 => 0,
            1 => 1,
            2 => self.below(8),
            3 => 0xffff_ffff,
            _ => self.next(),
        }
    }
    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next() as u8).collect()
    }
}

/// The bridge's view of an oracle: its memory, and its emulator for the
/// callees.
struct G<'a>(&'a mut Oracle, u32);

impl Mem for G<'_> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.0.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        Mem::write_bytes(self.0, addr, data);
    }
    fn image_base(&self) -> u32 {
        self.0.image_base()
    }
}

/// The shims' `call_cdecl`: at most 12 words.
fn oc(g: &mut G<'_>, f: u32, a: &[u32]) -> u32 {
    ffx_oracle::check_width("call_cdecl", f, a.len(), 12);
    g.0.call(g.1, Conv::Cdecl, &[]).unwrap();
    g.0.call(f, Conv::Cdecl, a).unwrap().eax
}

/// What a function returns, to compare.
#[derive(Debug, Clone, Copy)]
enum Out {
    /// Nothing the callers read.
    None,
    /// `eax`.
    U32(u32),
    /// `ST0` as `fst dword` stores it (bits).
    F32(u32),
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 16 words (tag, 15
    /// arguments).
    log: u32,
    cave: u32,
    cave_next: u32,
    /// Top bytes of a frame [`Twins::check_framed`] leaves out (the
    /// original's security cookie at `ebp - 4`).
    skip: u32,
    /// Bytes the stubs replaced, put back by [`Twins::reset`].
    patched: Vec<(u32, Vec<u8>)>,
}

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let cave = a.rebase(CAVE);
        let mut t = Self {
            a,
            b,
            log: 0,
            cave,
            cave_next: cave,
            skip: 0,
            patched: Vec::new(),
        };
        t.log = t.zeros(0x1_0000);
        let at = t.rebase(POISON_AT);
        let mut code = vec![0xb8];
        code.extend(POISON.to_le_bytes());
        code.push(0xc3);
        t.write(at, &code);
        t
    }

    fn image() -> Option<Self> {
        let exe = game_dir()?.join("FFX.exe");
        Some(Self::new(
            Oracle::load(&exe).unwrap(),
            Oracle::load(&exe).unwrap(),
        ))
    }

    fn snapshot() -> Option<Self> {
        let dir = game_dir()?;
        let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
            eprintln!("FFX_SNAPSHOT is not set; skipping");
            return None;
        };
        let exe = dir.join("FFX.exe");
        let pages = Pages::read(PathBuf::from(snap)).unwrap();
        Some(Self::new(
            Oracle::from_snapshot(&exe, &pages).unwrap(),
            Oracle::from_snapshot(&exe, &pages).unwrap(),
        ))
    }

    /// Both kinds: the image, then the snapshot when one is set.
    fn each(mut f: impl FnMut(&mut Self)) {
        if let Some(mut t) = Self::image() {
            f(&mut t);
        }
        if let Some(mut t) = Self::snapshot() {
            f(&mut t);
        }
    }

    fn rebase(&self, sym: u32) -> u32 {
        self.a.rebase(sym)
    }

    fn alloc(&mut self, bytes: &[u8]) -> u32 {
        let x = self.a.alloc_bytes(bytes).unwrap();
        assert_eq!(self.b.alloc_bytes(bytes).unwrap(), x);
        x
    }

    fn zeros(&mut self, n: usize) -> u32 {
        self.alloc(&vec![0; n])
    }

    fn words(&mut self, w: &[u32]) -> u32 {
        let bytes: Vec<u8> = w.iter().flat_map(|v| v.to_le_bytes()).collect();
        self.alloc(&bytes)
    }

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn w8(&mut self, at: u32, v: u8) {
        self.write(at, &[v]);
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    /// The `ffx.sym` global `sym` (a word) in both.
    fn g32(&mut self, sym: u32, v: u32) {
        let at = self.rebase(sym);
        self.w32(at, v);
    }

    fn g8(&mut self, sym: u32, v: u8) {
        let at = self.rebase(sym);
        self.w8(at, v);
    }

    /// Writes `bytes` at `at`, to be put back by [`Twins::reset`].
    fn patch(&mut self, at: u32, bytes: &[u8]) {
        let mut old = vec![0; bytes.len()];
        Mem::read_bytes(&self.b, at, &mut old);
        self.patched.push((at, old));
        self.write(at, bytes);
    }

    fn reset(&mut self) {
        for (at, old) in std::mem::take(&mut self.patched).into_iter().rev() {
            self.write(at, &old);
        }
        self.cave_next = self.cave;
        let log = self.log;
        self.w32(log, 0);
    }

    /// Code in the cave; returns its address.
    fn cave_code(&mut self, c: &[u8]) -> u32 {
        let addr = self.cave_next;
        assert!(addr + c.len() as u32 <= self.cave + CAVE_LEN, "cave full");
        self.write(addr, c);
        self.cave_next = addr + c.len() as u32;
        addr
    }

    /// The logging part of a stub: appends `tag` and `n` stack words.
    fn log_code(&self, tag: u32, n: u32) -> Vec<u8> {
        assert!(n <= 15);
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0x25, 0xff, 0x03, 0, 0]); // and eax, 0x3ff (a ring)
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (4 + 4 * i) as u8]); // mov [eax+4+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c
    }

    /// A recording cdecl stub over `sym`: logs `sym` and `n` words,
    /// returns `ret`.
    fn stub(&mut self, sym: u32, n: u32, ret: u32) {
        let mut c = self.log_code(sym, n);
        c.push(0xb8);
        c.extend(ret.to_le_bytes()); // mov eax, ret
        c.push(0xc3);
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }

    /// [`Twins::stub`] returning `seq[k]` on its `k`-th call (the last
    /// one after the end).
    fn stub_seq(&mut self, sym: u32, n: u32, seq: &[u32]) {
        let ctr = self.words(&[0]);
        let mut s = seq.to_vec();
        s.resize(s.len().max(1) + 256, *seq.last().unwrap_or(&0));
        let tbl = self.words(&s);
        let mut c = self.log_code(sym, n);
        c.push(0xa1);
        c.extend(ctr.to_le_bytes()); // mov eax, [ctr]
        c.extend([0x8b, 0x04, 0x85]);
        c.extend(tbl.to_le_bytes()); // mov eax, [tbl + eax*4]
        c.extend([0xff, 0x05]);
        c.extend(ctr.to_le_bytes()); // inc dword [ctr]
        c.push(0xc3);
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }

    /// A stub over `sym` that only counts its calls (in a word it
    /// allocates) and returns `ret`.
    fn stub_count(&mut self, sym: u32, ret: u32) {
        let ctr = self.words(&[0]);
        let mut c = vec![0xff, 0x05];
        c.extend(ctr.to_le_bytes()); // inc dword [ctr]
        c.push(0xb8);
        c.extend(ret.to_le_bytes());
        c.push(0xc3);
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }

    /// [`Twins::stub`] that also stores `seq[k]` at `at` on its `k`-th
    /// call (the last one after the end); returns 0.
    fn stub_store_seq(&mut self, sym: u32, at: u32, seq: &[u32]) {
        let ctr = self.words(&[0]);
        let mut s = seq.to_vec();
        s.resize(s.len().max(1) + 256, *seq.last().unwrap_or(&0));
        let tbl = self.words(&s);
        let mut c = self.log_code(sym, 0);
        c.push(0xa1);
        c.extend(ctr.to_le_bytes()); // mov eax, [ctr]
        c.extend([0x8b, 0x04, 0x85]);
        c.extend(tbl.to_le_bytes()); // mov eax, [tbl + eax*4]
        c.push(0xa3);
        c.extend(at.to_le_bytes()); // mov [at], eax
        c.extend([0xff, 0x05]);
        c.extend(ctr.to_le_bytes()); // inc dword [ctr]
        c.extend([0x31, 0xc0, 0xc3]); // xor eax, eax; ret
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }

    /// [`Twins::stub`] bound to the import table slot `slot`.
    fn stub_import(&mut self, slot: u32, n: u32, ret: u32) {
        let mut c = self.log_code(slot, n);
        c.push(0xb8);
        c.extend(ret.to_le_bytes());
        c.push(0xc3);
        let addr = self.cave_code(&c);
        let at = self.rebase(slot);
        self.patch(at, &addr.to_le_bytes());
    }

    /// A jump at `sym` to `to`.
    fn jump(&mut self, sym: u32, to: u32) {
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(to.wrapping_sub(at + 5).to_le_bytes());
        self.patch(at, &j);
    }

    fn diff(&mut self, what: &str) {
        assert_eq!(self.a.diff(&mut self.b).unwrap(), [], "{what}: memory");
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// The original's `ebp` for a call with `args` stack words.
    fn frame_ebp(&self, args: usize) -> u32 {
        self.a.stack_range().end - 0x100 - 4 * args as u32 - 8
    }

    /// Runs the original at `sym` (cdecl) in `a` and `bridge` on `b`;
    /// compares the result as `bridge` says and every byte written.
    fn check(&mut self, what: &str, sym: u32, args: &[u32], bridge: impl FnOnce(&mut G) -> Out) {
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let poison = self.rebase(POISON_AT);
        let out = bridge(&mut G(&mut self.b, poison));
        let st0 = ra.st0_f32().map(f32::to_bits);
        if !matches!(out, Out::F32(_)) {
            assert!(
                ra.st0_raw.is_none(),
                "{what}: the original leaves ST0 set ({:?})",
                ra.st0
            );
        }
        let bad = match out {
            Out::None => false,
            Out::U32(v) => ra.eax != v,
            Out::F32(v) => st0 != Some(v),
        };
        let bad = bad || !self.a.diff(&mut self.b).unwrap().is_empty();
        if bad {
            eprintln!("{what}: the call logs (original | port):");
            let (na, nb) = (
                Mem::read::<u32>(&self.a, self.log),
                Mem::read::<u32>(&self.b, self.log),
            );
            for k in 0..na.max(nb).min(40) {
                let at = self.log + 4 + 64 * k;
                let x: [u32; 6] = Mem::read(&self.a, at);
                let y: [u32; 6] = Mem::read(&self.b, at);
                eprintln!("  {k}: {x:x?} | {y:x?}");
            }
        }
        match out {
            Out::None => {}
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::F32(v) => assert_eq!(st0, Some(v), "{what}: ST0 (f32 bits)"),
        }
        self.diff(what);
    }

    /// [`Twins::check`] with the port's frame at the original's `ebp`
    /// (its callees log pointers into it; `low` bytes below `ebp`
    /// compared after the call) and its argument words at `ebp + 8` in
    /// the port's oracle too (the bridge gets `ebp`; the words are
    /// compared after the call: the originals that store into their own
    /// argument slots).
    fn check_framed(
        &mut self,
        what: &str,
        sym: u32,
        args: &[u32],
        low: u32,
        bridge: impl FnOnce(&mut G, u32) -> Out,
    ) {
        self.check_framed_init(what, sym, args, low, &[], bridge);
    }

    /// [`Twins::check_framed`] with the frame's lowest bytes set to `init`
    /// (the rest zero) in both oracles.
    fn check_framed_init(
        &mut self,
        what: &str,
        sym: u32,
        args: &[u32],
        low: u32,
        init: &[u8],
        bridge: impl FnOnce(&mut G, u32) -> Out,
    ) {
        let ebp = self.frame_ebp(args.len());
        let lo = ebp - low;
        let mut frame = vec![0; low as usize];
        frame[..init.len()].copy_from_slice(init);
        self.write(lo, &frame);
        let words: Vec<u8> = args.iter().flat_map(|v| v.to_le_bytes()).collect();
        self.b.write(ebp + 8, &words).unwrap();
        self.b.set_call_top(Some(lo - 0x100));
        self.check(what, sym, args, |m| bridge(m, ebp));
        self.b.set_call_top(None);
        let n = (low - self.skip) as usize;
        let x = Oracle::read(&mut self.a, lo, n).unwrap();
        let y = Oracle::read(&mut self.b, lo, n).unwrap();
        if let Some(i) = x.iter().zip(&y).position(|(u, v)| u != v) {
            let at = lo + i as u32;
            panic!(
                "{what}: frame byte ebp - {:#x} ({at:#x}): {} != {}",
                ebp - at,
                x[i],
                y[i]
            );
        }
        let x = Oracle::read(&mut self.a, ebp + 8, words.len()).unwrap();
        let y = Oracle::read(&mut self.b, ebp + 8, words.len()).unwrap();
        assert_eq!(x, y, "{what}: argument slots");
    }
}

// ---- jump thunks ----

#[test]
fn jump_thunks_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0bad_cafe);
        for (sym, target, n) in [
            (0x0078_26a0, mb::FFX_GET_ASIA_MODE, 0),
            (0x0078_2740, mb::MS_ET_EFFECT_READ, 0),
            (0x0092_e980, mb::MS_SET_CHR_POS, 2),
            (0x0092_e990, mb::MS_SET_CHR_VISIBLE, 2),
            (0x0092_eb10, mb::MS_EFFECT_SET_MOTION, 2),
        ] {
            for _ in 0..3 {
                t.reset();
                let ret = rng.word();
                t.stub(target, n, ret);
                let args: Vec<u32> = (0..n).map(|_| rng.word()).collect();
                t.check(&format!("thunk {sym:#x}"), sym, &args, |m| {
                    Out::U32(mb::jmp_thunk(m, &mut oc, target, &args))
                });
            }
        }
        // FFXGetAsiaMode itself: the byte it reads
        for v in [0u8, 1, 0x80, 0xff] {
            t.reset();
            t.g8(0x0133_0833, v);
            t.check("thunk 0x7826a0 (real)", 0x0078_26a0, &[], |m| {
                Out::U32(mb::jmp_thunk(m, &mut oc, mb::FFX_GET_ASIA_MODE, &[]))
            });
        }
    });
}

// ---- the debug menu's helpers ----

/// Inventory ids: some of the debug items already there, some 0x2065,
/// some empty (0xff), some random.
fn random_items(t: &mut Twins, rng: &mut Rng) {
    let mut ids = Vec::new();
    for _ in 0..0x70 {
        let v: u16 = match rng.below(5) {
            0 => 0x2000 + rng.below(0x70) as u16,
            1 => 0x2065,
            2 => 0xff,
            _ => rng.next() as u16,
        };
        ids.extend(v.to_le_bytes());
    }
    let at = t.rebase(mb::SAVE_ITEM_IDS);
    t.write(at, &ids);
    let counts = rng.bytes(0x70);
    let at = t.rebase(mb::SAVE_ITEM_COUNTS);
    t.write(at, &counts);
}

#[test]
fn debug_item_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x1234_5678);
        for i in 0..6 {
            for lang in [0, 0xb] {
                t.reset();
                random_items(t, &mut rng);
                t.stub_count(mb::TO_GET_FFX_LANG_JMP, lang);
                t.stub(mb::TK_MENU_ITEM_REFRESH, 1, 0);
                let tail = rng.word();
                t.stub(mb::FULL_ITEM_TAIL, 0, tail);
                t.check(
                    &format!("MsDebugFullItem #{i} lang {lang}"),
                    0x0078_49d0,
                    &[],
                    |m| Out::U32(mb::ms_debug_full_item(m, &mut oc)),
                );
            }
            t.reset();
            random_items(t, &mut rng);
            t.check("MsDebugAllItemsOne", 0x0078_4c20, &[], |m| {
                mb::ms_debug_all_items_one(m);
                Out::None
            });
            random_items(t, &mut rng);
            t.check("MsDebugClearItems", 0x0078_4cd0, &[], |m| {
                mb::ms_debug_clear_items(m);
                Out::None
            });
            // the save records: random words, many zero
            let mut rec = Vec::new();
            for _ in 0..(0x0113_2b00 - 0x0113_2044) / 2 {
                let v = if rng.chance(2) {
                    0u16
                } else {
                    rng.next() as u16
                };
                rec.extend(v.to_le_bytes());
            }
            let at = t.rebase(0x0113_2044);
            t.write(at, &rec);
            t.check("MsDebugFullStats", 0x0078_4b00, &[], |m| {
                mb::ms_debug_full_stats(m);
                Out::None
            });
            t.check("MsDebugAbilityOne", 0x0078_4c40, &[], |m| {
                mb::ms_debug_ability_one(m);
                Out::None
            });
        }
    });
}

#[test]
fn debug_full_member_matches_the_original() {
    Twins::each(|t| {
        for ret in [0, 1, 0x1234_5678] {
            t.reset();
            t.stub(mb::MS_SET_SAVE_PLY_JOIN, 2, 7);
            t.stub(mb::MS_SAVE_ENCOUNT_CHECK, 0, ret);
            t.check("MsDebugFullMember", 0x0078_4ac0, &[], |m| {
                Out::U32(mb::ms_debug_full_member(m, &mut oc))
            });
        }
    });
}

/// Actors for `MsGetChr`'s stub: `n` blocks of 0x600 random bytes.
fn actors(t: &mut Twins, rng: &mut Rng, n: usize) -> Vec<u32> {
    (0..n)
        .map(|_| {
            let b = rng.bytes(0x600);
            t.alloc(&b)
        })
        .collect()
}

#[test]
fn debug_weak_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0f0f_1234);
        for top in [0, 1] {
            for on in [0, 1, 0x100] {
                for (k, (name, sym)) in [
                    ("MsDebugWeakAll", 0x0078_4b80),
                    ("MsDebugWeakMonsters", 0x0078_4bc0),
                    ("MsDebugWeakParty", 0x0078_4c80),
                ]
                .into_iter()
                .enumerate()
                {
                    t.reset();
                    let list = actors(t, &mut rng, 0x1f);
                    t.stub(mb::MS_GET_CHR_TOP, 0, top);
                    // MsGetChr(i) for the i the function asks: the k-th
                    // call gets list[k] (their order is fixed)
                    t.stub_seq(mb::MS_GET_CHR, 1, &list);
                    t.stub(mb::MS_SET_CHR_WEAK, 2, 0);
                    let what = format!("{name} top {top} on {on}");
                    let args: &[u32] = if k == 0 { &[] } else { &[on] };
                    t.check(&what, sym, args, |m| {
                        match k {
                            0 => mb::ms_debug_weak_all(m, &mut oc),
                            1 => mb::ms_debug_weak_monsters(m, &mut oc, on),
                            _ => mb::ms_debug_weak_party(m, &mut oc, on),
                        }
                        Out::None
                    });
                }
            }
        }
    });
}

#[test]
fn check_chr_btl_pos_all_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5555_1234);
        for i in 0..12 {
            t.reset();
            let list = actors(t, &mut rng, 0x1f);
            // +0x508 set on a random few (or none)
            for (k, &a) in list.iter().enumerate() {
                let set = i % 4 != 0 && rng.chance(8 + k as u32 % 3);
                t.w8(a + 0x508, if set { rng.next() as u8 | 1 } else { 0 });
            }
            t.stub_seq(mb::MS_GET_CHR, 1, &list);
            t.check(
                &format!("MsCheckChrBtlPosAll #{i}"),
                0x0079_15f0,
                &[],
                |m| Out::U32(mb::ms_check_chr_btl_pos_all(m, &mut oc)),
            );
        }
    });
}

#[test]
fn sep_trans_check_wave_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7777_0001);
        for i in 0..20 {
            t.reset();
            let n = [0, 1, 5, 0x8000_0000, 0xffff_ffff][i % 5];
            t.g32(mb::SEP_TRANS, n);
            let idx = rng.below(15);
            t.g32(mb::SEP_TRANS_STATE, idx);
            let at = t.rebase(mb::SEP_TRANS_QUEUE) + 12 * idx;
            t.w8(at, if rng.chance(2) { 0 } else { rng.next() as u8 });
            t.check(
                &format!("MsSEPtransCheckWave #{i}"),
                0x007a_04d0,
                &[],
                |m| Out::U32(mb::ms_sep_trans_check_wave(m)),
            );
        }
    });
}

#[test]
fn pick_ground_group_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x2222_3333);
        for i in 0..40 {
            t.reset();
            let n: i8 = match i % 5 {
                0 => 0,
                1 => -3,
                _ => rng.below(12) as i8 + 1,
            };
            let mut recs = vec![0u8; 4 + 0x20 * 16];
            recs[1] = n as u8;
            for k in 0..16 {
                recs[4 + 0x20 * k] = rng.below(4) as u8;
            }
            let recs = t.alloc(&recs);
            t.g32(mb::BTL_SCRIPT_RECORDS, recs);
            let pos = rng.next();
            t.g32(mb::BTL_POS_FILE, pos);
            t.stub(mb::RND, 0, rng.next() & 0x7fff_ffff);
            t.stub(mb::MS_COPY_POINTS_OFFSET, 3, 0);
            let group = [0, 1, 2, 3, 0xffff_ffff][rng.below(5) as usize];
            t.check(
                &format!("MsPickGroundGroup #{i}"),
                0x007a_bf40,
                &[group],
                |m| {
                    mb::ms_pick_ground_group(m, &mut oc, group);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn atel_free_camera_id_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x4444_9999);
        for kind in [0, 1, 2, 3, 4, 0xffff_ffff] {
            for slot in [0, 1, 2, 3, 0x8000_0000] {
                t.reset();
                let ids: Vec<u8> = rng.bytes(0x30);
                let at = t.rebase(mb::ATEL_CAMERA);
                t.write(at, &ids);
                t.stub(mb::MS_CAMERA_FREE, 1, 0);
                t.check(
                    &format!("MsAtelFreeCameraID {kind} {slot}"),
                    0x007b_a6d0,
                    &[kind, slot],
                    |m| {
                        mb::ms_atel_free_camera_id(m, &mut oc, kind, slot);
                        Out::None
                    },
                );
            }
        }
    });
}

#[test]
fn camera_get_angle_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x1111_9999);
        for i in 0..4 {
            t.reset();
            let cam = t.zeros(0x700);
            t.g32(mb::CAMERA, cam);
            t.stub(mb::MS_VECTOR_RECT2_POLAR, 5, 0);
            let args = [rng.next(), rng.next(), rng.next()];
            t.check(&format!("MsCameraGetAngle #{i}"), 0x007b_d2d0, &args, |m| {
                mb::ms_camera_get_angle(m, &mut oc, args[0], args[1], args[2]);
                Out::None
            });
        }
    });
}

/// The scene picker's lists: a field record (i16 field number, 8 name
/// bytes at `+6`), an encounter list (count, group count) and group
/// records (size, then formation bytes at `5 + 2k`).
fn scene_lists(t: &mut Twins, rng: &mut Rng) -> (u32, u32, Vec<u32>) {
    let rec = {
        let b = rng.bytes(0x20);
        t.alloc(&b)
    };
    let groups = rng.below(5);
    let list = t.alloc(&[rng.below(12) as u8, groups as u8, 0, 0]);
    let recs: Vec<u32> = (0..=groups.max(1))
        .map(|_| {
            let mut b = rng.bytes(0x40);
            b[0] = rng.below(6) as u8;
            t.alloc(&b)
        })
        .collect();
    (rec, list, recs)
}

fn picker_state(t: &mut Twins, rng: &mut Rng) {
    let vals = [0, 1, 2, 3, 5, 0xffff_ffff, 0x7fff_ffff];
    for g in [
        mb::DBG_FIELD,
        mb::DBG_ENCOUNT,
        mb::DBG_GROUP,
        mb::DBG_IN_GROUP,
    ] {
        t.g32(g, vals[rng.below(vals.len() as u32) as usize]);
    }
    let fields = [0u16, 1, 3, 0x8001][rng.below(4) as usize];
    let at = t.rebase(mb::BTL_FIELD_COUNT);
    t.write(at, &fields.to_le_bytes());
}

#[test]
fn debug_scene_picker_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x3141_5926);
        for i in 0..40 {
            t.reset();
            picker_state(t, &mut rng);
            let (rec, list, groups) = scene_lists(t, &mut rng);
            t.stub(mb::MS_BTL_LIST_FIELD, 1, rec);
            t.stub(mb::MS_BTL_LIST_ENCOUNT, 1, list);
            t.stub_seq(
                mb::MS_BTL_LIST_GROUP,
                2,
                &[groups.clone(), groups.clone()].concat(),
            );
            t.check(&format!("MsDebugSceneWalk #{i}"), 0x007c_5dd0, &[], |m| {
                mb::ms_debug_scene_walk(m, &mut oc);
                Out::None
            });

            t.reset();
            picker_state(t, &mut rng);
            let (_, _, groups) = scene_lists(t, &mut rng);
            t.stub_seq(mb::MS_BTL_LIST_GROUP, 2, &groups);
            let g = rng.below(4);
            t.g32(mb::DBG_GROUP, if rng.chance(5) { 0xffff_ffff } else { g });
            t.check(
                &format!("MsDebugSceneGroupBase #{i}"),
                0x007c_5f00,
                &[],
                |m| Out::U32(mb::ms_debug_scene_group_base(m, &mut oc)),
            );

            t.reset();
            let (_, _, groups) = scene_lists(t, &mut rng);
            t.stub_seq(mb::MS_BTL_LIST_GROUP, 2, &groups);
            let args = [rng.word(), rng.below(4), rng.word(), rng.word(), rng.word()];
            t.check(&format!("MsDebugSetScene #{i}"), 0x007c_5f60, &args, |m| {
                Out::U32(mb::ms_debug_set_scene(m, &mut oc, args))
            });
        }
    });
}

#[test]
fn get_battle_name_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x2718_2818);
        for i in 0..16 {
            t.reset();
            let rec = {
                let b = rng.bytes(0x20);
                t.alloc(&b)
            };
            let formation = t.alloc(&[if rng.chance(2) { 0 } else { rng.next() as u8 }]);
            t.g32(mb::DBG_FIELD_REC, rec);
            let junk = rng.bytes(9);
            let name = t.rebase(mb::DBG_SCENE_NAME);
            t.write(name, &junk);
            t.g32(mb::DBG_FORMATION, formation);
            t.g32(mb::DBG_FIELD, if rng.chance(2) { 0 } else { rng.word() });
            t.stub(mb::MS_DEBUG_SCENE_WALK, 0, 0);
            let out = t.zeros(0x40);
            let full = [0, 1, 0x100][rng.below(3) as usize];
            // the fixed names pass three words, the scene name four
            let fixed = full != 0 && Mem::read::<u32>(&t.a, t.rebase(mb::DBG_FIELD)) == 0;
            t.stub_import(mb::SPRINTF_SLOT, if fixed { 3 } else { 4 }, 9);
            t.check(
                &format!("MsGetBattleName #{i}"),
                0x007c_5fc0,
                &[full, out],
                |m| Out::U32(mb::ms_get_battle_name(m, &mut oc, full, out)),
            );
        }
    });
}

#[test]
fn debug_next_member_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x6666_1111);
        for i in 0..60 {
            t.reset();
            let mut party = [0u8; 3];
            for p in &mut party {
                *p = match rng.below(4) {
                    0 => 0xff,
                    1 => 0x11,
                    2 => 0,
                    _ => rng.below(0x12) as u8,
                };
            }
            let slot = rng.below(3);
            let step = [0, 1, 0xffff_ffff, 5, 0x8000_0000][rng.below(5) as usize];
            if step == 0 && party[slot as usize] != 0xff {
                // a step of 0 on an unjoined or taken character loops
                // forever in both: keep the character free and joined
                let c = party[slot as usize];
                for (k, p) in party.iter_mut().enumerate() {
                    if k != slot as usize && *p == c {
                        *p = 0xfe;
                    }
                }
            }
            let at = t.rebase(mb::SAVE_IN_PARTY);
            t.write(at, &party);
            let mut joins: Vec<u32> = (0..8).map(|_| u32::from(rng.chance(2))).collect();
            if step == 0 {
                joins = vec![1];
            }
            joins.push(1);
            t.stub_seq(mb::MS_GET_SAVE_PLY_JOIN, 1, &joins);
            t.check(
                &format!("MsDebugNextMember #{i}"),
                0x007c_60a0,
                &[slot, step],
                |m| Out::U32(mb::ms_debug_next_member(m, &mut oc, slot, step)),
            );
        }
    });
}

#[test]
fn env_writer_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x9999_0000);
        for i in 0..30 {
            t.reset();
            let size = [1, 4, 16, 64][rng.below(4) as usize];
            let buf = t.zeros(0x100);
            let fill = if rng.chance(4) { size } else { rng.below(size) };
            let writer = t.words(&[buf, 0, rng.next(), fill, size]);
            let len = [0, 1, 3, 15, 40, 100][rng.below(6) as usize];
            let mut text: Vec<u8> = (0..len).map(|_| rng.below(255) as u8 + 1).collect();
            text.push(0);
            let text = t.alloc(&text);
            t.stub(mb::SCE_WRITE, 3, 0);
            t.check_framed(
                &format!("MsEnvWriteText #{i}"),
                0x007c_e9f0,
                &[writer, text],
                0,
                |m, ebp| {
                    mb::ms_env_write_text(m, &mut oc, ebp + 8, writer, text);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn env_find_key_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x8888_0000);
        for i in 0..30 {
            t.reset();
            let n = rng.below(6);
            let mut w = Vec::new();
            for _ in 0..n {
                w.extend([rng.next() | 1, rng.below(8), rng.next()]);
            }
            w.extend([0, rng.below(8), 0]);
            let table = t.words(&w);
            let key = rng.below(8);
            t.check(
                &format!("MsEnvFindKey #{i}"),
                0x007c_f140,
                &[table, key],
                |m| Out::U32(mb::ms_env_find_key(m, table, key)),
            );
        }
    });
}

#[test]
fn env_init_matches_the_original() {
    Twins::each(|t| {
        t.skip = 4;
        for debug in [0, 1] {
            for kind in [0u8, 1, 2, 3] {
                t.reset();
                t.stub(mb::FFX_IS_DEBUG, 0, debug);
                t.stub(mb::MS_SET_COUNTRY_NAME, 1, 0);
                t.stub_import(mb::SPRINTF_SLOT, 3, 5);
                t.stub(mb::PC_FILE_TBL_SET, 2, 0);
                t.g8(mb::ENV_KIND, kind);
                t.check_framed(
                    &format!("MsEnvInit {debug} {kind}"),
                    0x0078_2770,
                    &[],
                    0x84,
                    |m, ebp| {
                        mb::ms_env_init(m, &mut oc, ebp - 0x84);
                        Out::None
                    },
                );
            }
        }
        t.skip = 0;
    });
}

// ---- the CD layer and the password file ----

#[test]
fn password_file_read_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xabcd_0001);
        for i in 0..6 {
            t.reset();
            let f = if i % 3 == 0 { 0 } else { rng.next() | 1 };
            t.stub(mb::FIOS_OPEN, 2, f);
            let size = rng.below(0x100);
            t.stub_seq(mb::SCE_LSEEK, 3, &[size, 0]);
            let buf = rng.next();
            t.stub(mb::TK_MALLOC, 1, buf);
            t.stub(mb::SCE_READ, 3, size);
            t.stub(mb::SCE_CLOSE, 1, 0);
            t.stub(mb::MEMCPY, 3, 0);
            t.stub(mb::TK_FREE, 1, 0);
            let (name, dst) = (rng.next(), rng.next());
            t.check(
                &format!("PasswordFileRead #{i}"),
                0x0079_0430,
                &[name, dst],
                |m| Out::U32(mb::password_file_read(m, &mut oc, name, dst)),
            );
        }
    });
}

/// The CD layer's callees as stubs: the disc ready answers `ready`, the
/// status `status`, `sceCdRead` `read`.
fn cd_stubs(t: &mut Twins, ready: &[u32], status: &[u32], read: &[u32]) {
    t.stub_seq(mb::SCE_CD_DISK_READY, 1, ready);
    t.stub_seq(mb::SCE_CD_STATUS, 0, status);
    t.stub_seq(mb::SCE_CD_READ, 4, read);
    for (sym, n) in [
        // varargs: one word (the second is stack junk after a format
        // without arguments)
        (mb::DBG_PRINTF, 1),
        (mb::AUDIO_STOP, 1),
        (mb::READ_ERR_DISP_SET, 1),
        (mb::MOVIE_WAIT, 0),
        (mb::MOVIE_CSC, 0),
        (mb::TK_SYNC_V, 1),
        (mb::FLUSH_CACHE, 1),
        (mb::MOVIE_ERR, 3),
        (mb::SCE_CD_READ_RESET, 0),
    ] {
        t.stub(sym, n, 0x77);
    }
}

/// A context: `hd` at `+0x5075d`, the file at `+0x50544`, the kind at
/// `+0x50764`.
fn cd_ctx(t: &mut Twins, rng: &mut Rng, hd: bool) -> u32 {
    let ctx = t.zeros(0x5_0800);
    t.w8(ctx + 0x5_075d, if hd { rng.next() as u8 | 1 } else { 0 });
    let file = match rng.below(3) {
        0 => 0x8000_0000 | rng.next(),
        1 => 0,
        _ => rng.below(100),
    };
    t.w32(ctx + 0x5_0544, file);
    t.w8(ctx + 0x5_0764, rng.next() as u8);
    ctx
}

/// Disc answers that end the wait loops after a few rounds: ready is 2
/// at the end, the status 1 (tray open) a few times before.
fn disc_answers(rng: &mut Rng) -> (Vec<u32>, Vec<u32>) {
    let k = rng.below(4) as usize;
    let mut ready: Vec<u32> = (0..k)
        .map(|_| [0, 1, 2, 6][rng.below(4) as usize])
        .collect();
    ready.push(2);
    let mut status: Vec<u32> = (0..=k).map(|_| 1).collect();
    status.push([0, 2, 6][rng.below(3) as usize]);
    (ready, status)
}

#[test]
fn cd_disc_checks_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xcd00_0001);
        for i in 0..24 {
            t.reset();
            let (ready, mut status) = disc_answers(&mut rng);
            let first = [1, 1, 0, 2][i % 4];
            status.insert(0, first);
            cd_stubs(t, &ready, &status, &[1]);
            let ctx = cd_ctx(t, &mut rng, i % 6 == 5);
            t.g8(mb::CD_DISC_OPEN, rng.next() as u8);
            t.check(&format!("CdDiscWait #{i}"), 0x0077_6f60, &[ctx], |m| {
                Out::U32(mb::cd_disc_wait(m, &mut oc, ctx))
            });

            t.reset();
            cd_stubs(t, &ready, &status, &[1]);
            let sync = [0, 1, 0x100][rng.below(3) as usize];
            t.check(
                &format!("CdOpenCheck_sub #{i}"),
                0x0077_7000,
                &[sync],
                |m| Out::U32(mb::cd_open_check_sub(m, &mut oc, sync)),
            );
        }
    });
}

#[test]
fn cd_read_sectors_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xcd00_0002);
        for i in 0..40 {
            t.reset();
            let hd = i % 4 == 1;
            let ctx = cd_ctx(t, &mut rng, hd);
            // the retry rounds: ready 2 after a few tries; the read
            // succeeds at once, after a failure, or after a reset round
            let ready: Vec<u32> = match i % 5 {
                4 => {
                    let mut r = vec![0; 0x3c];
                    r.extend([1, 2]);
                    r
                }
                3 => vec![0, 2, 2],
                _ => vec![2],
            };
            let read = [vec![1], vec![0, 1], vec![3, 1]][rng.below(3) as usize].clone();
            cd_stubs(t, &ready, &[1], &read);
            t.stub(mb::HD_FILE_READ_NO_WAIT, 5, rng.next());
            // hdd_state_clear: the busy word it leaves (busy once, then
            // clear, in some cases)
            let busy = u32::from(rng.chance(3));
            let hdd = t.rebase(mb::HDD_STATE + 4);
            t.stub_store_seq(mb::HDD_STATE_CLEAR, hdd, &[busy, 0]);
            t.stub(mb::CD_DISC_WAIT, 1, 0x77);
            // the one-sector copy: right after the last read or not
            let block = {
                let b = rng.bytes(0x800);
                t.alloc(&b)
            };
            let last = rng.below(1000);
            let count_last = rng.below(5);
            t.g32(mb::CD_LAST_SECTOR, last);
            t.g32(mb::CD_LAST_COUNT, count_last);
            t.g32(mb::CD_LAST_BLOCK, block);
            t.g8(mb::CD_DISC_OPEN, u8::from(rng.chance(3)));
            let sector = if rng.chance(2) {
                count_last.wrapping_sub(1).wrapping_add(last)
            } else {
                rng.below(1000)
            };
            let count = rng.below(4) + 1;
            let buf = t.zeros(0x2800) + if rng.chance(3) { 4 } else { 0 };
            let mode = rng.word();
            let args = [sector, count, buf, mode, ctx];
            t.check_framed(
                &format!("CdReadSectors #{i}"),
                0x0077_7080,
                &args,
                0,
                |m, ebp| Out::U32(mb::cd_read_sectors(m, &mut oc, ebp + 8, args)),
            );
        }
    });
}

// ---- batch 2: the debug status, files, the position script reader ----

/// A cdecl stub over `sym` taking a string: logs `sym` and the
/// string's first 16 bytes (its text, not its address).
fn stub_text(t: &mut Twins, sym: u32) {
    stub_text_at(t, sym, 4);
}

/// [`stub_text`] for a string in the stack word at `esp + off`.
fn stub_text_at(t: &mut Twins, sym: u32, off: u8) {
    let mut c = t.log_code(sym, 0);
    c.extend([0x8b, 0x54, 0x24, off]); // mov edx, [esp+off]
    for k in 0..4u8 {
        c.extend([0x8b, 0x4a, 4 * k]); // mov ecx, [edx+4k]
        c.extend([0x89, 0x48, 4 + 4 * k]); // mov [eax+4+4k], ecx
    }
    c.extend([0x31, 0xc0, 0xc3]); // xor eax, eax; ret
    let addr = t.cave_code(&c);
    t.jump(sym, addr);
}

/// A recording cdecl stub's code alone (a function pointer's target):
/// logs `tag` and `n` words, returns `ret`.
fn stub_fn(t: &mut Twins, tag: u32, n: u32, ret: u32) -> u32 {
    let mut c = t.log_code(tag, n);
    c.push(0xb8);
    c.extend(ret.to_le_bytes());
    c.push(0xc3);
    t.cave_code(&c)
}

#[test]
fn battle_debug_status_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xdeb0_0001);
        t.skip = 4;
        for i in 0..16 {
            t.reset();
            let actor = {
                let b = rng.bytes(0xe00);
                t.alloc(&b)
            };
            let f8 = stub_fn(t, 0xcd08, 1, rng.below(5));
            let cd = t.words(&[0, 0, f8, 0]);
            for (sym, n, ret) in [
                (mb::MS_GET_BATTLE_END_STATUS, 1, rng.below(4)),
                (mb::TK_SND_REPORT_STATUS, 1, rng.below(4)),
                (mb::MS_SEP_TRANS_CHECK_WAVE, 0, rng.below(3)),
                (mb::GET_CD, 0, cd),
                (mb::MS_BTL_READ_CUE_LEVEL, 1, rng.below(9)),
                (mb::TO_DAPS_SET_POS, 2, 0),
                (mb::SND_SQS_IOP_SOUND_ERROR_STATUS, 1, rng.next()),
                (mb::SQS_IOP_SOUND_SYSTEM_STATUS_JMP, 1, rng.next()),
                (mb::MS_GET_SUMMON_ENERGY, 1, rng.below(200)),
                (mb::MS_GET_YOUJINBOU_COM, 0, rng.below(200)),
                (mb::MS_MENU_WEAK_ELEMENT, 1, rng.below(0x100)),
                (mb::MS_GET_CHR, 1, actor),
                (mb::MS_CHECK_CHR_MOTION_ALL, 1, rng.below(2)),
                (mb::SND_GET_STREAM_STATE, 0, rng.below(4)),
                (mb::SND_IS_READ_STATUS, 1, rng.below(2)),
                (mb::MS_CHECK_START_EFFECT, 2, rng.below(2)),
            ] {
                t.stub(sym, n, ret);
            }
            for g in [
                mb::BTL_STATUS,
                mb::BTL_END_HOLD,
                mb::BTL_DIR_FREE,
                mb::BTL_COMMAND_ACTOR,
                mb::BTL_CAMERA_STATE,
                mb::BTL_CAMERA_ONCE,
                mb::EFFECT_WAIT,
                mb::EFFECT_WAIT + 1,
                mb::BTL_RESET_WAIT2,
            ] {
                let v = if rng.chance(2) { 0 } else { rng.next() as u8 };
                t.g8(g, v);
            }
            stub_text(t, mb::TO_DAPS_STRING);
            t.g32(mb::EFFECT_CAMERA_BUF_20, rng.next());
            t.g32(mb::BTL_MESSAGE_LAST, rng.next());
            t.g32(
                mb::BTL_PARSED_COMMAND2,
                if rng.chance(2) {
                    0xff
                } else {
                    rng.below(0x200)
                },
            );
            t.g32(
                mb::BTL_LAST_MAGIC,
                [0x8000_0000 | rng.next(), 0, rng.below(1000)][rng.below(3) as usize],
            );
            t.g32(
                mb::BTL_STATUS_COUNTER,
                [0xffff_ffff, 0, rng.below(1000)][rng.below(3) as usize],
            );
            t.check_framed(
                &format!("DebugStatus #{i}"),
                0x0079_2f20,
                &[],
                0x44,
                |m, ebp| {
                    mb::ms_battle_debug_status(m, &mut oc, ebp - 0x44);
                    Out::None
                },
            );
        }
        t.skip = 0;
    });
}

#[test]
fn debug_file_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xf11e_0001);
        for i in 0..12 {
            t.reset();
            let fd = [0xffff_ffff, 0x8000_0000, 3, 0][i % 4];
            t.stub(mb::SCE_OPEN, 2, fd);
            let size = [0u32, 1, 3, 4, 0x7fff_fffe, 0xffff_fffd][rng.below(6) as usize];
            t.stub_seq(mb::SCE_LSEEK, 3, &[size, 0]);
            t.stub(mb::SCE_CLOSE, 1, 0);
            t.stub(mb::SCE_READ, 3, 0);
            t.stub(mb::SCE_WRITE, 3, 0);
            t.stub(mb::DBG_PRINTF, 2, 0);
            let name = rng.next();
            t.check(&format!("MsCheckFile #{i}"), 0x007c_8a80, &[name], |m| {
                Out::U32(mb::ms_check_file(m, &mut oc, name))
            });
            let (buf, n) = (rng.next(), rng.next());
            for (sym, out) in [(0x007c_8b00, false), (0x007c_8b50, true)] {
                t.check(&format!("MsFileIo {out} #{i}"), sym, &[name, buf, n], |m| {
                    mb::ms_file_io(m, &mut oc, out, name, buf, n);
                    Out::None
                });
            }
        }
    });
}

#[test]
fn debug_win_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0d1b_0001);
        for i in 0..20 {
            t.reset();
            // the in-use bytes: all set but one (or none free)
            let base = t.rebase(mb::DEBUG_WINS);
            let free = rng.below(0x90);
            for k in 0..0x80 {
                t.w8(
                    base + 0x20 + 0x34 * k,
                    if k == free {
                        0
                    } else {
                        1 + rng.below(255) as u8
                    },
                );
            }
            let slot = match i % 3 {
                0 => 0,
                1 => t.words(&[0]),
                _ => t.words(&[rng.next() | 1]),
            };
            t.check(
                &format!("MsDebugWinAlloc #{i}"),
                0x007c_8bb0,
                &[slot],
                |m| Out::U32(mb::ms_debug_win_alloc(m, slot)),
            );
            t.reset();
            let wins: Vec<u32> = (0..4)
                .map(|_| {
                    let b = rng.bytes(0x40);
                    t.alloc(&b)
                })
                .collect();
            t.stub_seq(mb::DEBUG_WIN_ALLOC, 1, &wins);
            t.check(&format!("MsInfoWinsInit #{i}"), 0x007c_fca0, &[], |m| {
                mb::ms_info_wins_init(m, &mut oc);
                Out::None
            });
            // MsInfoWinSet: the windows set or null, any index
            for (k, &w) in wins.iter().enumerate() {
                let at = t.rebase(mb::INFO_WINS) + 4 * k as u32;
                t.w32(at, if rng.chance(4) { 0 } else { w });
            }
            t.g8(mb::INFO_WIN_FLAGS, rng.next() as u8);
            // the word before the windows (index -1)
            let before = t.rebase(mb::INFO_WINS) - 4;
            let w0 = wins[0];
            t.w32(before, if rng.chance(2) { 0 } else { w0 });
            let pos = t.words(&[rng.next(), 0x3f80_0000, 0x7fc0_0001]);
            let idx = [0, 1, 2, 3, 4, 0xffff_ffff][rng.below(6) as usize];
            let mark = rng.next();
            t.check(
                &format!("MsInfoWinSet #{i}"),
                0x007c_fba0,
                &[idx, mark, pos],
                |m| {
                    mb::ms_info_win_set(m, idx, mark, pos);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn debug_memory_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x3e3e_0001);
        t.skip = 4;
        for i in 0..12 {
            t.reset();
            let block = if i % 4 == 0 { 0 } else { rng.next() };
            t.stub(mb::USER_MALLOC, 1, block);
            t.stub(mb::DBG_PRINTF, 1, 0);
            t.stub(mb::DEBUG_EXIT_TRACE, 1, 0);
            t.stub(mb::OP_FILE_PUTS, 2, 0);
            t.stub(mb::USER_FREE_THUNK, 1, 0);
            t.g32(mb::MEM_TRACE, u32::from(rng.chance(2)));
            let size = rng.next();
            t.check_framed(
                &format!("ms_malloc #{i}"),
                0x007c_f970,
                &[size],
                0x104,
                |m, ebp| Out::U32(mb::ms_malloc(m, &mut oc, ebp - 0x104, size)),
            );
            t.check_framed(
                &format!("ms_free #{i}"),
                0x007c_f8f0,
                &[block],
                0x104,
                |m, ebp| {
                    mb::ms_free(m, &mut oc, ebp - 0x104, block);
                    Out::None
                },
            );
        }
        t.skip = 0;
    });
}

#[test]
fn debug_battle_pos_init_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xb905_0001);
        t.skip = 4;
        for i in 0..8 {
            t.reset();
            let user = t.alloc(b"tester\0");
            t.stub(mb::MS_GET_BATTLE_NAME, 2, 0);
            t.stub(mb::FFX_IS_DEBUG, 0, u32::from(i % 2 == 0));
            t.stub(mb::SG_GET_USER_NAME, 0, if i % 4 < 2 { user } else { 0 });
            let out = t.zeros(0x200);
            let a = t.alloc(b"_a\0");
            let b = t.alloc(b".bin\0");
            let country = t.rebase(mb::COUNTRY_NAME);
            let c = rng.below(26) as u8 + b'a';
            t.write(country, &[c, c, 0]);
            t.check_framed(
                &format!("MsDebugBattlePosInit #{i}"),
                0x007c_90f0,
                &[out, a, b],
                0x104,
                |m, ebp| {
                    mb::ms_debug_battle_pos_init(m, &mut oc, ebp - 0x104, [out, a, b]);
                    Out::None
                },
            );
        }
        t.skip = 0;
    });
}

#[test]
fn btl_pos_free_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xf7ee_0001);
        for i in 0..12 {
            t.reset();
            t.stub(mb::MS_FREE, 1, 0);
            t.stub(mb::MS_BTL_POS_INIT, 0, 0);
            let n = rng.below(4);
            let mut recs = Vec::new();
            for _ in 0..n {
                let mut r = vec![0u32; 0x74 / 4];
                for w in [0x28, 0x2c, 0x4c, 0x50, 0x60, 0x64] {
                    r[w / 4] = if rng.chance(3) { 0 } else { rng.next() | 1 };
                }
                let k = rng.below(4);
                r[0x68 / 4] = k;
                if !rng.chance(4) {
                    let subs: Vec<u32> = (0..k * 5)
                        .map(|j| {
                            if j % 5 == 4 && !rng.chance(3) {
                                rng.next() | 1
                            } else {
                                0
                            }
                        })
                        .collect();
                    r[0x70 / 4] = t.words(&[subs, vec![0]].concat());
                }
                recs.extend(r);
            }
            let arr = if i % 5 == 4 {
                0
            } else {
                t.words(&[recs, vec![0]].concat())
            };
            let table = t.words(&[0, 0, 0, 0, 0, 0, 0, n, 0, 0, 0, rng.next() & 0xff, arr, 0]);
            t.g32(mb::POS_TABLE, if i % 6 == 5 { 0 } else { table });
            t.check(&format!("MsBtlPosFree #{i}"), 0x007c_9210, &[], |m| {
                mb::ms_btl_pos_free(m, &mut oc);
                Out::None
            });
        }
    });
}

#[test]
fn btl_pos_versions_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e75_0001);
        for i in 0..24 {
            t.reset();
            t.stub(mb::DBG_PRINTF, 2, 0);
            t.g32(
                mb::POS_VERSION,
                [0, 1, 2, 0xffff_ffff][rng.below(4) as usize],
            );
            t.g32(mb::POS_VERSION_SUB, rng.below(3));
            t.g32(
                mb::POS_FILE_NAME,
                if rng.chance(2) { 0 } else { rng.next() },
            );
            t.g32(mb::POS_LINE, if rng.chance(2) { 0 } else { rng.below(99) });
            let (name, v, sub) = (
                rng.next(),
                [0, 1, 2, 0x8000_0000][rng.below(4) as usize],
                rng.below(3),
            );
            t.check(
                &format!("check_version #{i}"),
                0x007c_c1f0,
                &[name, v],
                |m| Out::U32(mb::ms_btl_pos_check_version(m, &mut oc, name, v)),
            );
            t.check(
                &format!("check_version2 #{i}"),
                0x007c_c250,
                &[name, v, sub],
                |m| Out::U32(mb::ms_btl_pos_check_version2(m, &mut oc, name, v, sub)),
            );
        }
    });
}

/// A text reader over `text` (`+0` buffer, `+0xc` position, `+0x10`
/// length).
fn reader(t: &mut Twins, text: &[u8], pos: u32, len: u32) -> u32 {
    let mut b = text.to_vec();
    b.extend([0x55; 8]);
    let buf = t.alloc(&b);
    t.words(&[buf, 0, 0, pos, len])
}

/// Text for the readers: words, spaces and tabs, quotes, line ends,
/// Shift-JIS pairs, NULs and 0xff.
fn text(rng: &mut Rng, n: usize) -> Vec<u8> {
    (0..n)
        .map(|_| match rng.below(14) {
            0 => b' ',
            1 => b'\t',
            2 => b'"',
            3 => b'\'',
            4 => b'\n',
            5 => b'\r',
            6 => [0x80, 0x82, 0xe0][rng.below(3) as usize],
            7 => 0,
            8 => 0xff,
            9 => b'A' + rng.below(26) as u8,
            _ => b'a' + rng.below(26) as u8,
        })
        .collect()
}

#[test]
fn readers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7ead_0001);
        for i in 0..200 {
            t.reset();
            let n = 1 + rng.below(40) as usize;
            let body = text(&mut rng, n);
            let pos = if rng.chance(5) {
                n as u32 + rng.below(3)
            } else {
                rng.below(n as u32)
            };
            let len = if rng.chance(4) {
                rng.below(n as u32 + 1)
            } else {
                n as u32
            };
            let r = reader(t, &body, pos, len);
            match i % 3 {
                0 => t.check(&format!("MsReadSkipSpace #{i}"), 0x007c_f540, &[r], |m| {
                    Out::U32(mb::ms_read_skip_space(m, r))
                }),
                1 => {
                    let line = if rng.chance(3) {
                        0
                    } else {
                        t.words(&[rng.below(9)])
                    };
                    t.check(
                        &format!("MsReadSkipLine #{i}"),
                        0x007c_f380,
                        &[r, line],
                        |m| Out::U32(mb::ms_read_skip_line(m, r, line)),
                    );
                }
                _ => {
                    let size = [0, 1, 2, 5, 0x100][rng.below(5) as usize];
                    let out = t.alloc(&[0x55; 0x110]);
                    let real = rng.chance(2);
                    if !real {
                        t.stub(mb::READ_SKIP_SPACE, 1, 0);
                    }
                    t.check_framed(
                        &format!("MsReadToken #{i}"),
                        0x007c_f180,
                        &[r, out, size],
                        0,
                        |m, ebp| Out::U32(mb::ms_read_token(m, &mut oc, ebp + 8, r, out, size)),
                    );
                }
            }
        }
    });
}

#[test]
fn readers_on_lead_bytes_match_the_original() {
    Twins::each(|t| {
        let cases: [&[u8]; 7] = [
            b"\x82\xff",
            b"\x82\x00",
            b"ab\x82",
            b"\x80a b",
            b"a\x82b c",
            b"\x82\x82",
            b"\x80\nab\n",
        ];
        for (i, body) in cases.into_iter().enumerate() {
            for size in [1, 2, 3, 4] {
                t.reset();
                let r = reader(t, body, 0, body.len() as u32);
                let out = t.alloc(&[0x55; 0x10]);
                let what = format!("MsReadToken lead #{i} size {size}");
                t.check_framed(&what, 0x007c_f180, &[r, out, size], 0, |m, ebp| {
                    Out::U32(mb::ms_read_token(m, &mut oc, ebp + 8, r, out, size))
                });
            }
            t.reset();
            let r = reader(t, body, 0, body.len() as u32);
            t.check(
                &format!("MsReadSkipLine lead #{i}"),
                0x007c_f380,
                &[r, 0],
                |m| Out::U32(mb::ms_read_skip_line(m, r, 0)),
            );
        }
    });
}

#[test]
fn read_values_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5ca7_0001);
        t.skip = 4;
        for i in 0..16 {
            t.reset();
            let body: &[u8] = [b"12 x".as_slice(), b"\n", b"-3.5\n", b" 0x10 "][i % 4];
            let r = reader(t, body, 0, body.len() as u32);
            t.stub(mb::DBG_PRINTF, 2, 0);
            t.g32(
                mb::POS_FILE_NAME,
                if rng.chance(2) { 0 } else { rng.next() },
            );
            t.g32(mb::POS_LINE, if rng.chance(2) { 0 } else { rng.below(99) });
            let out = t.zeros(8);
            let name = rng.next();
            let float = i % 2 == 0;
            let sym = if float { 0x007c_d360 } else { 0x007c_d420 };
            t.check_framed(
                &format!("MsReadValue {float} #{i}"),
                sym,
                &[r, name, out],
                0x104,
                |m, ebp| {
                    mb::ms_read_value(m, &mut oc, ebp - 0x104, float, [r, name, out]);
                    Out::None
                },
            );
        }
        t.skip = 0;
    });
}

#[test]
fn btl_pos_alloc_points_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xa11c_0001);
        for i in 0..6 {
            t.reset();
            let rec = {
                let mut b = rng.bytes(0x74);
                b[0x54..0x58].copy_from_slice(&rng.below(50).to_le_bytes());
                t.alloc(&b)
            };
            t.g32(mb::POS_RECORD, rec);
            t.stub_seq(mb::MS_MALLOC, 1, &[rng.next(), rng.next()]);
            t.stub(mb::MS_STRUCT_CLEAR, 2, 0);
            t.check(
                &format!("MsBtlPosAllocPoints #{i}"),
                0x007c_d4e0,
                &[],
                |m| {
                    mb::ms_btl_pos_alloc_points(m, &mut oc);
                    Out::None
                },
            );
        }
    });
}

/// The keyword hash of `name` as `0x007ceeb0` computes it.
fn keyword_hash(name: &[u8]) -> u32 {
    let mut h = 0u32;
    for (k, &c) in name.iter().enumerate() {
        let c = if c.wrapping_sub(0x41) <= 0x19 {
            c + 0x20
        } else {
            c
        };
        h = h.wrapping_add(i32::from(c as i8) as u32 ^ (0x6f * k as u32));
    }
    h
}

#[test]
fn string_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x57e1_0001);
        let words: [&[u8]; 6] = [b"cameraz", b"CAMERAZ", b"Pos", b"pos\x82", b"", b"cameraX"];
        for i in 0..60 {
            t.reset();
            let a = words[rng.below(6) as usize];
            let b = words[rng.below(6) as usize];
            let pa = t.alloc(&[a, b"\0"].concat());
            let pb = t.alloc(&[b, b"\0"].concat());
            t.check(
                &format!("MsStrEqualNoCase #{i}"),
                0x007c_f330,
                &[pa, pb],
                |m| Out::U32(mb::ms_str_equal_no_case(m, pa, pb)),
            );
            // a keyword table: names, values, hashes (some wrong)
            let mut tbl = Vec::new();
            for w in &words[..rng.below(6) as usize] {
                let p = t.alloc(&[w, b"\0".as_slice()].concat());
                let h = if rng.chance(5) {
                    rng.next()
                } else {
                    keyword_hash(w)
                };
                tbl.extend([p, rng.next(), h]);
            }
            tbl.extend([0, 0, 0]);
            let table = t.words(&tbl);
            t.check(
                &format!("MsKeywordLookup #{i}"),
                0x007c_eeb0,
                &[pa, table],
                |m| Out::U32(mb::ms_keyword_lookup(m, pa, table)),
            );
            for kind in [0, 1, 2, 3, 5, 7, 8, 17, 19, 21, 23, 24, 0x125] {
                t.check(
                    &format!("MsCameraKindChar {kind}"),
                    0x007c_faa0,
                    &[kind],
                    |m| Out::U32(mb::ms_camera_kind_char(m, kind)),
                );
            }
        }
    });
}

#[test]
fn path_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x9a7b_0001);
        let paths: [&[u8]; 10] = [
            b"dir\\file.bin",
            b"dir/sub/file",
            b"file.",
            b"a\x82\\b.c",
            b".x",
            b"",
            b"d\\e.f\ng",
            b"x\x83.",
            b"q\\",
            b"ab\xff.c",
        ];
        let more: [&[u8]; 4] = [b"\x80\\x.y", b"a.b\rc", b".x", b"a\r.b"];
        for i in 0..60 {
            t.reset();
            let p = if rng.chance(3) {
                more[rng.below(4) as usize]
            } else {
                paths[rng.below(10) as usize]
            };
            let mut buf = p.to_vec();
            buf.extend([0, 0x77, 0x77, 0x77, 0]);
            let path = t.alloc(&buf);
            let ext = if rng.chance(3) {
                0
            } else {
                t.words(&[rng.next()])
            };
            t.check(
                &format!("MsPathBaseName #{i}"),
                0x007c_efb0,
                &[path, ext],
                |m| Out::U32(mb::ms_path_base_name(m, path, ext)),
            );
            t.reset();
            // a byte before the path: MsPathSetExt reads ext[-2]
            let path = t.alloc(&[&[0x90], buf.as_slice()].concat()) + 1;
            t.stub(mb::MS_STR_COPY, 3, 0);
            let (src, n) = (rng.next(), rng.below(20));
            let force = [0, 1, 2][rng.below(3) as usize];
            // MsPathBaseName stays the original: it writes the index into
            // the caller's argument slot
            t.check_framed(
                &format!("MsPathSetExt #{i}"),
                0x007c_f590,
                &[path, src, n, force],
                0,
                |m, ebp| {
                    Out::U32(mb::ms_path_set_ext(
                        m,
                        &mut oc,
                        ebp + 8,
                        [path, src, n, force],
                    ))
                },
            );
        }
    });
}

#[test]
fn read_open_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x09e1_0001);
        for i in 0..10 {
            t.reset();
            let fd = [0xffff_ffff, 0x8000_0000, 3, 0][i % 4];
            t.stub(mb::SCE_OPEN, 2, fd);
            let size = [0u32, 1, 15, 16, 17, 0xffff_ffe0][rng.below(6) as usize];
            t.stub_seq(mb::SCE_LSEEK, 3, &[size, 0]);
            let buf = t.alloc(&[0x66; 0x40]);
            t.stub_import(mb::MALLOC_SLOT, 1, buf);
            t.stub(mb::SCE_READ, 3, 0);
            t.stub(mb::SCE_CLOSE, 1, 0);
            let r = {
                let b = rng.bytes(0x14);
                t.alloc(&b)
            };
            let name = rng.next();
            if size > 0x40 && size < 0x8000_0000 {
                continue;
            }
            t.check(&format!("MsReadOpen #{i}"), 0x007c_d5a0, &[r, name], |m| {
                mb::ms_read_open(m, &mut oc, r, name);
                Out::None
            });
        }
    });
}

#[test]
fn debug_set_camera_mode_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xca3e_0001);
        for i in 0..8 {
            t.reset();
            t.g8(mb::DEBUG_CAMERA, rng.below(3) as u8);
            t.stub(mb::MS_CAMERA_GET_NUM, 1, rng.below(4));
            let (side, mode) = (rng.below(2), rng.next());
            t.check(
                &format!("MsDebugSetCameraMode #{i}"),
                0x007c_fb60,
                &[side, mode],
                |m| {
                    mb::ms_debug_set_camera_mode(m, &mut oc, side, mode);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn env_write_text_copy_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x9999_0002);
        for i in 0..20 {
            t.reset();
            let size = [1, 4, 16, 64][rng.below(4) as usize];
            let buf = t.zeros(0x100);
            let fill = rng.below(size);
            let writer = t.words(&[buf, 0, rng.next(), fill, size]);
            let len = [0, 1, 3, 15, 40][rng.below(5) as usize];
            let mut s: Vec<u8> = (0..len).map(|_| rng.below(255) as u8 + 1).collect();
            s.push(0);
            let s = t.alloc(&s);
            t.stub(mb::SCE_WRITE, 3, 0);
            t.check_framed(
                &format!("MsEnvWriteText copy #{i}"),
                0x007c_d680,
                &[writer, s],
                0,
                |m, ebp| {
                    mb::ms_env_write_text(m, &mut oc, ebp + 8, writer, s);
                    Out::None
                },
            );
        }
    });
}

// ---- batch 3: the camera and battle debug screens' helpers ----

/// Float bits for the x87 tests: ordinary values, both zeros, values at
/// and around the clamp limits, infinities and quiet NaNs (signalling
/// NaNs stay out: the emulator keeps them, hardware quiets them).
fn float_bits(rng: &mut Rng) -> u32 {
    match rng.below(12) {
        0 => 0,
        1 => 0x8000_0000,
        2 => 1.0f32.to_bits(),
        3 => 1.000_000_1f32.to_bits(),
        4 => 0.999_999_9f32.to_bits(),
        5 => (-0.5f32).to_bits(),
        6 => f32::INFINITY.to_bits(),
        7 => f32::NEG_INFINITY.to_bits(),
        8 => 0x7fc0_0000 | (rng.next() & 0x003f_ffff),
        9 => 1e-40f32.to_bits(),
        _ => {
            let v = (rng.next() % 4000) as f32 / 1000.0 - 1.5;
            v.to_bits()
        }
    }
}

#[test]
fn env_write_text_copy3_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x9999_0003);
        for i in 0..12 {
            t.reset();
            let size = [1, 4, 16][rng.below(3) as usize];
            let buf = t.zeros(0x100);
            let writer = t.words(&[buf, 0, rng.next(), rng.below(size), size]);
            let mut s: Vec<u8> = (0..[0, 2, 30][i % 3])
                .map(|_| rng.below(255) as u8 + 1)
                .collect();
            s.push(0);
            let s = t.alloc(&s);
            t.stub(mb::SCE_WRITE, 3, 0);
            t.check_framed(
                &format!("MsEnvWriteText copy 3 #{i}"),
                0x007d_e700,
                &[writer, s],
                0,
                |m, ebp| {
                    mb::ms_env_write_text(m, &mut oc, ebp + 8, writer, s);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn debug_screen_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xd5c7_0001);
        for i in 0..12 {
            t.reset();
            t.stub(mb::MS_FREE, 1, 0);
            t.stub(mb::MS_BTL_POS_FREE, 0, 0);
            t.g32(mb::POS_EDIT_BUF, if i % 2 == 0 { 0 } else { rng.next() });
            t.check(&format!("MsBtlPosEditFree #{i}"), 0x007d_7450, &[], |m| {
                mb::ms_btl_pos_edit_free(m, &mut oc);
                Out::None
            });
            t.reset();
            let dir = t.rebase(mb::DBG_SCENE_DIR);
            t.write(dir, if i % 2 == 0 { b"\0" } else { b"abc\0" });
            let root = t.rebase(mb::DBG_SCENE_ROOT);
            t.write(root, b"host0:/x/\0");
            t.g32(mb::DBG_SCENE_PATH_POS, rng.next() | 1);
            t.check(&format!("MsDebugScenePath #{i}"), 0x007d_70d0, &[], |m| {
                mb::ms_debug_scene_path(m, &mut oc);
                Out::None
            });
        }
        for kind in 0..14u32 {
            for k in [kind, kind | 0x100, 0xffff_ffff] {
                t.check(
                    &format!("MsDebugIsTextMode {k:#x}"),
                    0x007d_79e0,
                    &[k],
                    |_| Out::U32(mb::ms_debug_is_text_mode(k)),
                );
            }
        }
    });
}

#[test]
fn debug_place_all_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x91ac_0001);
        t.skip = 4;
        for i in 0..6 {
            t.reset();
            let actors: Vec<u32> = (0..0x1f)
                .map(|_| {
                    let model = if rng.chance(3) { 0 } else { rng.next() | 1 };
                    t.words(&[model])
                })
                .collect();
            t.stub_seq(mb::MS_GET_CHR, 1, &actors);
            t.stub(mb::CH_SET_POS3, 4, 0);
            // MsGetChrBtlPos writes no floats here: the frame's start
            // values (the same in both) go to Ch_SetPos3
            t.stub(mb::MS_GET_CHR_BTL_POS, 2, 0);
            let init: Vec<u8> = (0..3)
                .flat_map(|_| float_bits(&mut rng).to_le_bytes())
                .collect();
            t.check_framed_init(
                &format!("MsDebugPlaceAll #{i}"),
                0x007d_73e0,
                &[],
                0x14,
                &init,
                |m, ebp| {
                    mb::ms_debug_place_all(m, &mut oc, ebp - 0x14);
                    Out::None
                },
            );
        }
        t.skip = 0;
    });
}

#[test]
fn debug_find_name_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xf14d_0001);
        for i in 0..16 {
            t.reset();
            let n = rng.below(5);
            let mut list: Vec<u32> = (0..n).map(|_| rng.next() | 1).collect();
            list.push(0);
            let list = t.words(&list);
            let hit = rng.below(6);
            let answers: Vec<u32> = (0..6).map(|k| u32::from(k == hit)).collect();
            t.stub_seq(mb::MS_STR_EQUAL_NO_CASE, 2, &answers);
            let name = rng.next();
            t.check(
                &format!("MsDebugFindName #{i}"),
                0x007d_2f10,
                &[name, list],
                |m| Out::U32(mb::ms_debug_find_name(m, &mut oc, name, list)),
            );
        }
    });
}

#[test]
fn debug_ref_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7ef0_0001);
        for i in 0..80 {
            t.reset();
            let cam = rng.below(3);
            let rec = t.rebase(0x0113_7b8c) + 0xda0 * cam;
            let v: Vec<u8> = (0..0x10)
                .flat_map(|_| float_bits(&mut rng).to_le_bytes())
                .collect();
            t.write(rec + 0x20, &v[..0x20]);
            t.write(rec + 0x30, &v[0x20..0x30]);
            t.check(
                &format!("MsDebugRefFromCamera #{i}"),
                0x007d_36b0,
                &[cam],
                |m| {
                    mb::ms_debug_ref_from_camera(m, cam);
                    Out::None
                },
            );
            let args: [u32; 7] = std::array::from_fn(|_| float_bits(&mut rng));
            t.check_framed(
                &format!("MsDebugRefSet #{i}"),
                0x007d_3420,
                &args,
                0,
                |m, ebp| Out::F32(mb::ms_debug_ref_set(m, ebp + 8, args)),
            );
            t.g32(mb::REF_VALUE, float_bits(&mut rng));
            let mut delta = float_bits(&mut rng);
            // both NaNs: the x87 keeps the larger payload
            if i % 8 == 7 {
                t.g32(mb::REF_VALUE, 0x7fc0_0001 | (rng.next() & 0x003f_ffff));
                delta = 0x7fc0_0000 | (rng.next() & 0x003f_ffff);
            }
            t.check(&format!("MsDebugRefAdd #{i}"), 0x007d_2f50, &[delta], |m| {
                Out::F32(mb::ms_debug_ref_add(m, delta))
            });
            for sse2 in [0, 1] {
                t.g32(0x0193_ed20, sse2); // __sse2_available
                let v = match rng.below(4) {
                    0 => float_bits(&mut rng),
                    1 => (rng.next() as f32 * 1e-3).to_bits(),
                    _ => ((rng.next() % 40_000) as f32 / 1000.0 - 20.0).to_bits(),
                };
                t.g32(mb::REF_VALUE, v);
                t.check(
                    &format!("MsDebugRefRound #{i} sse2 {sse2}"),
                    0x007d_23d0,
                    &[],
                    |m| {
                        mb::ms_debug_ref_round(m);
                        Out::None
                    },
                );
            }
        }
    });
}

#[test]
fn debug_file_names_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xf9a3_0001);
        for i in 0..16 {
            t.reset();
            let debug = u32::from(i % 2 == 0);
            let user = t.alloc(b"tester\0");
            t.stub(mb::FFX_IS_DEBUG, 0, debug);
            t.stub(mb::SG_GET_USER_NAME, 0, user);
            // the user name may be null: sprintf stays a stub
            t.stub_import(mb::SPRINTF_SLOT, 6, 0);
            let out = rng.next();
            let n = rng.next();
            t.check(
                &format!("MsSetFileName #{i}"),
                0x007d_ce80,
                &[out, n],
                |m| {
                    mb::ms_set_file_name(m, &mut oc, out, n);
                    Out::None
                },
            );
            t.reset();
            t.stub(mb::FFX_IS_DEBUG, 0, debug);
            t.stub(mb::SG_GET_USER_NAME, 0, user);
            let program = rng.chance(2);
            t.stub(mb::MS_STR_EQUAL_NO_CASE, 2, u32::from(program));
            t.stub_import(mb::SPRINTF_SLOT, if program { 5 } else { 6 }, 0);
            t.g8(
                mb::BTL_UNDERWATER,
                if rng.chance(2) {
                    0
                } else {
                    rng.next() as u8 | 1
                },
            );
            let name = rng.next();
            t.check(
                &format!("MsGetPlySaveName #{i}"),
                0x007d_cec0,
                &[out, name],
                |m| {
                    mb::ms_get_ply_save_name(m, &mut oc, out, name);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn debug_actor_count_and_paths_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xac7c_0001);
        for i in 0..24 {
            t.reset();
            t.stub(
                mb::ATEL_GET_TOTAL_ACTOR_NUMBER2,
                0,
                [0, 1, 7, 0xffff_ffff][rng.below(4) as usize],
            );
            let id = [0x5_0000, 0x5_1234, 0x4_0000, 0x15_0000, rng.next()][rng.below(5) as usize];
            let kind = if rng.chance(3) {
                0
            } else {
                t.words(&[rng.next()])
            };
            t.check(
                &format!("MsDebugActorCount #{i}"),
                0x007d_9ab0,
                &[id, kind],
                |m| Out::U32(mb::ms_debug_actor_count(m, &mut oc, id, kind)),
            );
            let pairs: [(&[u8], &[u8]); 6] = [
                (b"a/b/c", b"a/b"),
                (b"a/b/c", b"x"),
                (b"abc", b"abc"),
                (b"", b"q"),
                (b"ab/", b"a"),
                (b"a/b", b"a/x/y"),
            ];
            let (p, b) = pairs[i % 6];
            let pp = t.alloc(&[p, b"\0"].concat());
            let pb = t.alloc(&[b, b"\0"].concat());
            t.check(
                &format!("MsDebugPathAfterBase #{i}"),
                0x007d_12a0,
                &[pp, pb],
                |m| Out::U32(mb::ms_debug_path_after_base(m, pp, pb)),
            );
        }
    });
}

#[test]
fn debug_camera_copy_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xcc0f_0001);
        for i in 0..12 {
            t.reset();
            let screen = rng.below(2);
            t.g32(mb::DBG_CAMERA_SCREEN, screen);
            let cam = rng.below(3);
            let at = t.rebase(mb::DEBUG_CAMERA) + 0x1c0 * screen;
            t.w8(at, cam as u8);
            let ch = rng.below(3);
            t.stub(mb::MS_CAMERA_GET_ID, 1, rng.next());
            t.stub(mb::MS_CAMERA_GET_NUM, 1, ch);
            t.stub(
                mb::MS_DEBUG_GET_CAMERA_MODE,
                1,
                [7, 8, 9, 10, 0][rng.below(5) as usize],
            );
            t.stub(mb::MS_DEBUG_CAMERA_SYNC, 0, 0);
            let data = {
                let b = rng.bytes(0x640);
                t.alloc(&b)
            };
            let rec = t.rebase(0x0113_7c4c) + 0xda0 * cam + 0x130 * ch;
            t.w32(rec + 8, data);
            t.g32(
                mb::DBG_CAMERA_FROM,
                if rng.chance(2) {
                    data + 0x630
                } else {
                    rng.next()
                },
            );
            t.check(&format!("MsDebugCameraCopy #{i}"), 0x007d_1220, &[], |m| {
                mb::ms_debug_camera_copy(m, &mut oc);
                Out::None
            });
        }
    });
}

#[test]
fn btl_pos_load_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x10ad_0001);
        for i in 0..12 {
            t.reset();
            t.skip = 4;
            // the parser: tokens, then empty ones until the end
            let toks: Vec<u32> = (0..4).map(|k| u32::from(k < i % 3)).collect();
            let tok_stub = {
                // MsReadToken's stub writes the token's first byte from a
                // sequence (1: "x", 0: empty) into the buffer it gets
                let ctr = t.words(&[0]);
                let mut s = toks.clone();
                s.resize(64, 0);
                let tbl = t.words(&s);
                let mut c = t.log_code(mb::READ_TOKEN, 3);
                c.push(0xa1);
                c.extend(ctr.to_le_bytes()); // mov eax, [ctr]
                c.extend([0x8b, 0x04, 0x85]);
                c.extend(tbl.to_le_bytes()); // mov eax, [tbl + eax*4]
                c.extend([0xff, 0x05]);
                c.extend(ctr.to_le_bytes()); // inc dword [ctr]
                c.extend([0x8b, 0x54, 0x24, 0x08]); // mov edx, [esp+8]
                c.extend([0x88, 0x02]); // mov [edx], al
                c.push(0xc3);
                t.cave_code(&c)
            };
            t.jump(mb::READ_TOKEN, tok_stub);
            t.stub(mb::MS_POS_PARSE_TOKEN, 2, 0);
            t.stub_seq(mb::READ_SKIP_LINE, 2, &[0, 3, 0xffff_ffff]);
            t.check_framed(
                &format!("MsBtlPosParse #{i}"),
                0x007c_d0f0,
                &[if i % 5 == 4 { 0 } else { 0x1234 }],
                0x104,
                |m, ebp| {
                    let r = if i % 5 == 4 { 0 } else { 0x1234 };
                    mb::ms_btl_pos_parse(m, &mut oc, ebp - 0x104, r);
                    Out::None
                },
            );

            t.reset();
            // MsReadOpen's stub fills the reader from a template
            let tmpl = [
                rng.next(),
                u32::from(rng.chance(2)),
                [0xffff_ffff, 0, 5][rng.below(3) as usize],
                [0, 1, 9][rng.below(3) as usize],
                0,
            ];
            let tmpl_at = t.words(&tmpl);
            let open = {
                let mut c = t.log_code(mb::READ_OPEN, 2);
                c.extend([0x56, 0x57]); // push esi; push edi
                c.extend([0x8b, 0x7c, 0x24, 0x0c]); // mov edi, [esp+0xc]
                c.push(0xbe);
                c.extend(tmpl_at.to_le_bytes()); // mov esi, tmpl
                c.extend([0xb9, 5, 0, 0, 0]); // mov ecx, 5
                c.extend([0xf3, 0xa5]); // rep movsd
                c.extend([0x5f, 0x5e, 0xc3]); // pop edi; pop esi; ret
                t.cave_code(&c)
            };
            t.jump(mb::READ_OPEN, open);
            t.stub(mb::MS_POS_PARSE, 1, 0);
            t.stub(mb::SCE_WRITE, 3, 0);
            t.stub(mb::SCE_CLOSE, 1, 0);
            t.stub(mb::DBG_PRINTF, 2, 0);
            t.stub_import(mb::FREE_SLOT, 1, 0);
            t.g32(mb::POS_FILE_NAME, rng.next());
            t.g32(mb::POS_LINE, rng.below(9));
            let name = rng.next();
            t.skip = 0; // no cookie in this frame
            t.check_framed(
                &format!("MsBtlPosLoad #{i}"),
                0x007c_a4a0,
                &[name],
                0x14,
                |m, ebp| Out::U32(mb::ms_btl_pos_load(m, &mut oc, ebp - 0x14, name)),
            );
        }
        t.skip = 0;
    });
}

// ---- batch 4: the position screens, the monster motion file ----

#[test]
fn btl_pos_write_all_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x3b17_0001);
        for i in 0..16 {
            t.reset();
            let n = [0, 1, 3, 0xffff_ffff][i % 4];
            let mut recs = Vec::new();
            for _ in 0..n.min(4) {
                let mut r = vec![0u32; 0x74 / 4];
                for o in [0xc, 0x30, 0x54] {
                    r[o / 4] = rng.below(100);
                }
                let k = rng.below(6);
                r[0x68 / 4] = k;
                let subs: Vec<u32> = (0..k * 5).map(|_| rng.below(50)).collect();
                r[0x70 / 4] = t.words(&[subs, vec![0]].concat());
                recs.extend(r);
            }
            let arr = t.words(&[recs, vec![0]].concat());
            let table = t.words(&[0, 0, 0, 0, 0, 0, 0, n, 0, rng.below(0x100), 0, 0, arr, 0]);
            t.g32(mb::POS_TABLE, table);
            t.stub(mb::MS_MALLOC, 1, 0x1234_5678);
            t.stub(mb::MS_BTL_POS_WRITE_BIN, 2, rng.next());
            let out = t.words(&[if i % 3 == 0 { rng.next() } else { 0 }]);
            t.check(
                &format!("MsBtlPosWriteAll #{i}"),
                0x007c_9bc0,
                &[out],
                |m| Out::U32(mb::ms_btl_pos_write_all(m, &mut oc, out)),
            );
        }
    });
}

/// The position screen's state: kind, group, picks, the position file.
fn pos_screen(t: &mut Twins, rng: &mut Rng) {
    let kinds = [0, 3, 4, 5, 6, 7, 8, 0xb, 0xc, 0xffff_ffff, 2, 9];
    t.g32(mb::POS_KIND, kinds[rng.below(12) as usize]);
    t.g8(mb::BTL_STATUS, u8::from(!rng.chance(5)));
    t.g8(mb::POS_GROUP, [0, 1, 2, 0xff, 0x80][rng.below(5) as usize]);
    let picks: Vec<u32> = (0..12)
        .map(|_| [0, 1, 5, 0xffff_ffff][rng.below(4) as usize])
        .collect();
    let at = t.rebase(mb::POS_PICK);
    let bytes: Vec<u8> = picks.iter().flat_map(|v| v.to_le_bytes()).collect();
    t.write(at, &bytes);
    let mut file = rng.bytes(0x20 * 13);
    file[0] = u8::from(rng.chance(2));
    file[1] = [0, 1, 3, 0x81][rng.below(4) as usize];
    for k in 0..13 {
        for o in [4, 5, 6, 9] {
            file[0x20 * k + o] = [0, 1, 3, 0xff][rng.below(4) as usize];
        }
    }
    let file = t.alloc(&file);
    t.g32(mb::BTL_POS_FILE, file);
}

#[test]
fn debug_pos_screen_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x905c_0001);
        for i in 0..60 {
            t.reset();
            pos_screen(t, &mut rng);
            t.check(&format!("MsDebugPosGroup #{i}"), 0x007d_72e0, &[], |m| {
                Out::U32(mb::ms_debug_pos_group(m))
            });
            pos_screen(t, &mut rng);
            let step = [1, 0xffff_ffff, 0, 3][rng.below(4) as usize];
            t.check(
                &format!("MsDebugPosNextKind #{i}"),
                0x007d_92d0,
                &[step],
                |m| {
                    mb::ms_debug_pos_next_kind(m, step);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn debug_pos_windows_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x3a1d_0001);
        for i in 0..24 {
            t.reset();
            let wins: Vec<u32> = (0..0x60)
                .map(|_| if rng.chance(4) { 0 } else { rng.next() & !3 })
                .collect();
            let at = t.rebase(mb::POS_WINS);
            let bytes: Vec<u8> = wins.iter().flat_map(|v| v.to_le_bytes()).collect();
            t.write(at, &bytes);
            // MsDebugWinAlloc: the slot gets a fresh record (some null)
            let recs: Vec<u32> = (0..0x60)
                .map(|_| {
                    let b = rng.bytes(0x40);
                    t.alloc(&b)
                })
                .collect();
            let fill = {
                let ctr = t.words(&[0]);
                let mut s = recs.clone();
                for (k, v) in s.iter_mut().enumerate() {
                    if k % 7 == 3 {
                        *v = 0;
                    }
                }
                let tbl = t.words(&s);
                let mut c = t.log_code(mb::DEBUG_WIN_ALLOC, 1);
                c.push(0xa1);
                c.extend(ctr.to_le_bytes()); // mov eax, [ctr]
                c.extend([0x8b, 0x04, 0x85]);
                c.extend(tbl.to_le_bytes()); // mov eax, [tbl + eax*4]
                c.extend([0xff, 0x05]);
                c.extend(ctr.to_le_bytes()); // inc dword [ctr]
                c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
                c.extend([0x89, 0x02, 0xc3]); // mov [edx], eax; ret
                t.cave_code(&c)
            };
            t.jump(mb::DEBUG_WIN_ALLOC, fill);
            let found: Vec<u32> = (0..8)
                .map(|k| u32::from(k > 2 + i % 4 || (k == 0 && i % 2 == 0)))
                .collect();
            t.stub_seq(mb::MS_BTL_GET_POS, 6, &found);
            let kind = rng.below(4);
            t.g32(mb::POS_KIND, if rng.chance(2) { kind } else { 9 });
            let at = t.rebase(mb::POS_PICK) + 4 * kind;
            t.w32(at, rng.below(4));
            t.g32(mb::POS_HIGHLIGHT, u32::from(rng.chance(2)));
            let all = u32::from(i % 3 == 0);
            let start = [0, 5, 0x5e, 0x60, 0xffff_ffff][rng.below(5) as usize];
            let args = [
                all,
                rng.below(3),
                kind,
                start,
                rng.next() & 0x1ff,
                0xffff_fff1,
                rng.below(256),
                rng.next(),
            ];
            t.check(
                &format!("MsDebugPosWindows #{i}"),
                0x007d_7a40,
                &args,
                |m| Out::U32(mb::ms_debug_pos_windows(m, &mut oc, args)),
            );
        }
    });
}

#[test]
fn mon_save_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x305a_0001);
        t.skip = 4;
        for i in 0..12 {
            t.reset();
            t.stub(mb::SCE_OPEN, 2, [0xffff_ffff, 0, 4, 0x8000_0000][i % 4]);
            let buf = if i % 5 == 4 { 0 } else { rng.next() | 1 };
            t.stub_import(mb::MALLOC_SLOT, 1, buf);
            t.stub_import(mb::FREE_SLOT, 1, 0);
            stub_text_at(t, mb::ENV_WRITE_TEXT_3, 8);
            t.stub(mb::SCE_WRITE, 3, 0);
            t.stub(mb::SCE_CLOSE, 1, 0);
            // the motion writer leaves a fill and the flag
            let w = {
                let fill = [0u32, 1, 0x80][i % 3];
                let flag = u32::from(i % 2 == 0 || i % 4 == 1);
                let mut c = t.log_code(mb::MS_MOTION_WRITE, 2);
                c.extend([0x8b, 0x54, 0x24, 0x08]); // mov edx, [esp+8]
                c.extend([0xc7, 0x42, 0x0c]);
                c.extend(fill.to_le_bytes()); // mov dword [edx+0xc], fill
                c.extend([0xc7, 0x42, 0x04]);
                c.extend(flag.to_le_bytes()); // mov dword [edx+4], flag
                c.push(0xc3);
                t.cave_code(&c)
            };
            t.jump(mb::MS_MOTION_WRITE, w);
            let scene = t.rebase(mb::BTL_SCENE_NAME);
            t.write(scene, b"bsil_00\0\0");
            let args = [rng.next(), rng.next(), rng.next()];
            t.check_framed(
                &format!("MsMonSave #{i}"),
                0x007d_d480,
                &args,
                0x118,
                |m, ebp| Out::U32(mb::ms_mon_save(m, &mut oc, ebp - 0x118, args)),
            );
        }
        t.skip = 0;
    });
}

#[test]
fn debug_print_rules_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7a1e_0001);
        t.skip = 4;
        for i in 0..4 {
            t.reset();
            t.stub(mb::GU_PRINT_SET_X, 1, 0);
            t.stub(mb::GU_PRINT_SET_Y, 1, 0);
            t.stub(mb::GU_PRINTF, 2, 0);
            let y = rng.next();
            t.check_framed(
                &format!("MsDebugPrintRules #{i}"),
                0x007d_10c0,
                &[y],
                0x104,
                |m, ebp| Out::U32(mb::ms_debug_print_rules(m, &mut oc, ebp - 0x104, y)),
            );
        }
        t.skip = 0;
    });
}

// ---- batch 5: actors by place ----

/// A synthetic actor table (`CHR_TOP`, 0x1f actors of 0xf90 bytes): the
/// places `+0x4fc..+0x4fe` drawn from a few values, models, the `+0xdc8`
/// flag, widths and directions as floats. The original `MsGetChr`,
/// `MsGetChrWidth` and `MsNearRag` run in both oracles (the ports call
/// the latter two's Rust directly).
fn actor_table(t: &mut Twins, rng: &mut Rng) {
    let mut b = vec![0u8; 0x1f * 0xf90];
    for k in 0..0x1f {
        let a = k * 0xf90;
        let model = if rng.chance(4) { 0 } else { rng.next() | 1 };
        b[a..a + 4].copy_from_slice(&model.to_le_bytes());
        for o in [0x4fc, 0x4fd, 0x4fe] {
            b[a + o] = [0, 1, 0xff][rng.below(3) as usize];
        }
        b[a + 0xdc8] = u8::from(rng.chance(2));
        for o in [0x3a8, 0x4e4, 0x514] {
            b[a + o..a + o + 4].copy_from_slice(&float_bits(rng).to_le_bytes());
        }
    }
    let top = t.alloc(&b);
    t.g32(0x0113_34d4, top); // CHR_TOP
}

#[test]
fn debug_actor_place_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x091a_ce01);
        for i in 0..60 {
            t.reset();
            actor_table(t, &mut rng);
            let pick = |rng: &mut Rng| [0, 1, 0xffff_ffff, 2][rng.below(4) as usize];
            let place = [pick(&mut rng), pick(&mut rng), pick(&mut rng)];
            let index = if rng.chance(3) {
                0
            } else {
                t.words(&[rng.next()])
            };
            let args = [place[0], place[1], place[2], index];
            t.check(&format!("MsDebugWidthAt #{i}"), 0x007d_93f0, &args, |m| {
                Out::F32(mb::ms_debug_width_at(m, &mut oc, args))
            });
            t.check(&format!("MsDebugDirAt #{i}"), 0x007d_9470, &place, |m| {
                Out::F32(mb::ms_debug_dir_at(m, &mut oc, place))
            });
            t.stub(mb::CH_SET_DIR_F, 2, 0);
            let deg = float_bits(&mut rng);
            let args = [place[0], place[1], place[2], deg];
            t.check(&format!("MsDebugSetDirAt #{i}"), 0x007d_9550, &args, |m| {
                mb::ms_debug_set_dir_at(m, &mut oc, args);
                Out::None
            });
        }
    });
}

// ---- batch 6 ----

#[test]
fn read_vec3_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7ec3_0001);
        t.skip = 4;
        for i in 0..12 {
            t.reset();
            let body: &[u8] = [b"1 2 3".as_slice(), b"1.5 -2\n", b"\n", b"0.25 x 7e3 "][i % 4];
            let r = reader(t, body, 0, body.len() as u32);
            t.stub(mb::DBG_PRINTF, 2, 0);
            t.g32(
                mb::POS_FILE_NAME,
                if rng.chance(2) { 0 } else { rng.next() },
            );
            t.g32(mb::POS_LINE, rng.below(9));
            let out = t.zeros(12);
            let args = [r, rng.next(), out, out + 4, out + 8];
            t.check_framed(
                &format!("MsReadVec3 #{i}"),
                0x007c_d170,
                &args,
                0x104,
                |m, ebp| {
                    mb::ms_read_vec3(m, &mut oc, ebp - 0x104, args);
                    Out::None
                },
            );
        }
        t.skip = 0;
    });
}

#[test]
fn debug_camera_keep_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xcae9_0001);
        for i in 0..40 {
            t.reset();
            let screen = rng.below(2);
            t.g32(mb::DBG_CAMERA_SCREEN, screen);
            let cam = rng.below(3);
            let at = t.rebase(mb::DEBUG_CAMERA) + 0x1c0 * screen;
            t.w8(at, cam as u8);
            let ch = rng.below(4);
            t.stub(mb::MS_CAMERA_GET_NUM, 1, ch);
            t.stub(mb::MS_CAMERA_RESET_MOVE, 2, 0);
            // the camera, the kept cameras and their flags
            let camera = t.rebase(mb::CAMERAS) + 0xda0 * cam;
            let mut c = Vec::new();
            for _ in 0..0x368 {
                c.extend(float_bits(&mut rng).to_le_bytes());
            }
            t.write(camera, &c);
            let mut k = Vec::new();
            for _ in 0..3 * 0x368 {
                k.extend(float_bits(&mut rng).to_le_bytes());
            }
            let kept = t.rebase(mb::CAM_KEPT);
            t.write(kept, &k);
            let flags: Vec<u8> = (0..3)
                .flat_map(|_| (rng.below(4) | if rng.chance(3) { 0x10 } else { 0 }).to_le_bytes())
                .collect();
            let fa = t.rebase(mb::CAM_KEPT_FLAGS);
            t.write(fa, &flags);
            t.g32(mb::CAM_RESTORE_WHOLE, u32::from(rng.chance(3)));
            t.g32(mb::CAM_SLOT_FLAG, rng.next() & 6);
            let slots = t.rebase(mb::CAMERA_SLOTS) + 0xda0 * cam;
            for s in 0..4 {
                let level = [0u32, 0x3, 0x13, 0xf0, 0x8][rng.below(5) as usize] << 16;
                let w = (rng.next() & 0xff00_ffff) | level;
                t.w32(slots + 8 * s, w);
                let b = rng.next() & 0x00ff_ffff & if rng.chance(3) { !0xff00 } else { !0 };
                t.w32(slots + 8 * s + 4, b);
            }
            let step = [0, 1, 0xffff_ffff, 2][i % 4];
            t.check_framed(
                &format!("MsDebugCameraKeep #{i}"),
                0x007c_8c50,
                &[step],
                0,
                |m, ebp| Out::U32(mb::ms_debug_camera_keep(m, &mut oc, ebp + 8, step)),
            );
        }
    });
}

// ---- batch 7 ----

#[test]
fn ply_save_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x91a5_0001);
        t.skip = 4;
        for i in 0..12 {
            t.reset();
            t.stub(mb::SCE_OPEN, 2, [0xffff_ffff, 0, 4, 0x8000_0000][i % 4]);
            let buf = if i % 5 == 4 { 0 } else { rng.next() | 1 };
            t.stub_import(mb::MALLOC_SLOT, 1, buf);
            t.stub_import(mb::FREE_SLOT, 1, 0);
            stub_text_at(t, mb::ENV_WRITE_TEXT_3, 8);
            t.stub(mb::SCE_WRITE, 3, 0);
            t.stub(mb::SCE_CLOSE, 1, 0);
            let summons: Vec<u32> = (0..0x20).map(|_| u32::from(rng.chance(3))).collect();
            t.stub_seq(mb::MS_GET_RAM_CHR_SUMMON, 1, &summons);
            // the motion writer leaves a fill and the flag
            let w = {
                let fill = [0u32, 1, 0x80][i % 3];
                let flag = u32::from(i % 2 == 0 || i % 4 == 1);
                let mut c = t.log_code(mb::MS_MOTION_WRITE, 2);
                c.extend([0x8b, 0x54, 0x24, 0x08]); // mov edx, [esp+8]
                c.extend([0xc7, 0x42, 0x0c]);
                c.extend(fill.to_le_bytes()); // mov dword [edx+0xc], fill
                c.extend([0xc7, 0x42, 0x04]);
                c.extend(flag.to_le_bytes()); // mov dword [edx+4], flag
                c.push(0xc3);
                t.cave_code(&c)
            };
            t.jump(mb::MS_MOTION_WRITE, w);
            let scene = t.rebase(mb::BTL_SCENE_NAME);
            t.write(scene, b"bsil_00\0\0");
            t.g8(
                mb::BTL_UNDERWATER,
                if i % 3 == 1 { rng.next() as u8 | 1 } else { 0 },
            );
            let name = rng.next();
            t.check_framed(
                &format!("MsPlySave #{i}"),
                0x007d_d5e0,
                &[name],
                0x418,
                |m, ebp| Out::U32(mb::ms_ply_save(m, &mut oc, ebp - 0x418, name)),
            );
        }
        t.skip = 0;
    });
}
