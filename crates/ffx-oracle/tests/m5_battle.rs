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
        assert!(
            ra.st0_raw.is_none(),
            "{what}: the original leaves ST0 set ({:?})",
            ra.st0
        );
        let bad = match out {
            Out::None => false,
            Out::U32(v) => ra.eax != v,
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
        let ebp = self.frame_ebp(args.len());
        let lo = ebp - low;
        self.write(lo, &vec![0; low as usize]);
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
