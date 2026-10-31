//! M5 stage 1, list `system-a` (`ffx_system::m5a_*`) against the
//! original: on the image alone with synthetic objects, and on the
//! snapshots (`FFX_SNAPSHOT`) over the game's memory. Results and every
//! byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! Most of these functions never ran in a coverage run, so every branch
//! is reached with synthetic data (the snapshots add the game's own
//! tables where a function reads them). Callees that are still original
//! (`operator delete`, the string assign, the `MSVCP110` imports, virtual
//! methods of fake objects) are replaced in both oracles alike by
//! recording stubs: each call appends its tag, `ecx` and its arguments to
//! a log in memory, so the diff checks that both made the same calls in
//! the same order. Imports are stubbed through their import table slots.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_system::m5a_files::bridge as fl;
use ffx_system::m5a_fixup::bridge as fx;
use ffx_system::m5a_glue::bridge as gl;
use ffx_system::m5a_platform::{self as platform, bridge as pf};
use ffx_system::m5a_savedata::{self as sv, bridge as sa};
use ffx_system::m5a_savedata2::{self as s2, bridge as s2b};
use ffx_system::m5a_settings::{self as settings, bridge as st};
use ffx_system::m5a_std::{self as sd, bridge as sb};
use ffx_system::m5a_vfs::bridge as vf;

/// `graphicFontDraw`'s body: no test here calls it, so the recording
/// stubs are written over it.
const CAVE: u32 = 0x0064_0660;
const CAVE_LEN: u32 = 1760;
/// After the stubs: `mov eax, POISON; ret`, which [`G`] calls before each
/// original so that a callee leaving `eax` alone shows the poison.
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
}

/// The bridge's view of an oracle: its memory, and its emulator for the
/// original callees.
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

fn conv(c: host::Conv) -> Conv {
    match c {
        host::Conv::Cdecl => Conv::Cdecl,
        host::Conv::Stdcall => Conv::Stdcall,
        host::Conv::Thiscall(t) => Conv::Thiscall(t),
        host::Conv::Fastcall => Conv::Fastcall,
    }
}

impl Host for G<'_> {
    fn call(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> u32 {
        ffx_oracle::check_width("Host::call", addr, args.len(), host::max_words(c, false));
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        self.0.call(addr, conv(c), args).unwrap().eax
    }
    fn call_st0(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> f32 {
        ffx_oracle::check_width("Host::call_st0", addr, args.len(), host::max_words(c, true));
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        let r = self.0.call(addr, conv(c), args).unwrap();
        r.st0_f32().expect("the callee leaves ST0 set")
    }
    fn call_st0_f64(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> f64 {
        ffx_oracle::check_width(
            "Host::call_st0_f64",
            addr,
            args.len(),
            host::max_words(c, true),
        );
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        let r = self.0.call(addr, conv(c), args).unwrap();
        r.st0_f64().expect("the callee leaves ST0 set")
    }
    fn scratch(&mut self) -> u32 {
        self.0.stack_range().start + 0x100
    }
}

/// What a function returns, to compare.
#[derive(Debug, Clone, Copy)]
enum Out {
    /// Nothing the callers read.
    None,
    /// `eax`.
    U32(u32),
    /// `al`.
    U8(u8),
    /// `edx:eax`.
    U64(u64),
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 16 words (tag, `this`, up
    /// to 14 arguments).
    log: u32,
    cave: u32,
    cave_next: u32,
    snapshot: bool,
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
    fn new(mut a: Oracle, mut b: Oracle, snapshot: bool) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let cave = a.rebase(CAVE);
        let mut t = Self {
            a,
            b,
            log: 0,
            cave,
            cave_next: cave,
            snapshot,
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
            false,
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
            true,
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

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    fn r32(&self, at: u32) -> u32 {
        Mem::read(&self.a, at)
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

    /// A recording stub in the cave: logs `tag`, `ecx` when `this`, and
    /// `n` stack arguments; pops `pops` words; returns `ret`. Returns its
    /// address.
    fn stub_code(&mut self, tag: u32, this: bool, n: u32, pops: u32, ret: u32) -> u32 {
        assert!(n <= 14);
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        if this {
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        }
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xb8);
        c.extend(ret.to_le_bytes()); // mov eax, ret
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        let addr = self.cave_next;
        assert!(addr + c.len() as u32 <= self.cave + CAVE_LEN, "cave full");
        self.write(addr, &c);
        self.cave_next = addr + c.len() as u32;
        addr
    }

    /// A recording cdecl stub bound to the import slot `slot` like
    /// [`Twins::stub_import`] (tag `slot`, `n` words, no `ecx`, `pops`
    /// words popped), returning `seq[k]` on its `k`-th call (`seq` must be
    /// long enough).
    fn stub_import_seq(&mut self, slot: u32, n: u32, pops: u32, seq: &[u32]) {
        let ctr = self.words(&[0]);
        let tbl = self.words(seq);
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(slot.to_le_bytes()); // mov [eax], tag
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xa1);
        c.extend(ctr.to_le_bytes()); // mov eax, [ctr]
        c.extend([0x8b, 0x04, 0x85]);
        c.extend(tbl.to_le_bytes()); // mov eax, [tbl + eax*4]
        c.extend([0xff, 0x05]);
        c.extend(ctr.to_le_bytes()); // inc dword [ctr]
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        let addr = self.cave_code(&c);
        let at = self.rebase(slot);
        self.patch(at, &addr.to_le_bytes());
    }

    /// Code in the cave; returns its address.
    fn cave_code(&mut self, c: &[u8]) -> u32 {
        let addr = self.cave_next;
        assert!(addr + c.len() as u32 <= self.cave + CAVE_LEN, "cave full");
        self.write(addr, c);
        self.cave_next = addr + c.len() as u32;
        addr
    }

    /// A jump at `sym` to `to`.
    fn jump(&mut self, sym: u32, to: u32) {
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(to.wrapping_sub(at + 5).to_le_bytes());
        self.patch(at, &j);
    }

    /// A stub over `sym` that returns `value` in `ST0` (`fld dword`).
    fn stub_float(&mut self, sym: u32, value: f32) {
        let cell = self.words(&[value.to_bits()]);
        let mut c = vec![0xd9, 0x05];
        c.extend(cell.to_le_bytes());
        c.push(0xc3);
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }

    /// An `fread(buf, 1, n, f)`-like recording stub (cdecl) bound to
    /// import slot `slot`: copies `n` bytes (at most `max`) from `src`
    /// to `buf`, logs its four words, returns `ret`.
    fn stub_import_fill(&mut self, slot: u32, src: u32, max: u32, ret: u32) {
        let log_stub = self.stub_code(slot, false, 4, 0, ret);
        let mut c = vec![0x56, 0x57]; // push esi; push edi
        c.extend([0x8b, 0x7c, 0x24, 0x0c]); // mov edi, [esp+0xc] (buf)
        c.extend([0x8b, 0x4c, 0x24, 0x14]); // mov ecx, [esp+0x14] (n)
        c.push(0xb8);
        c.extend(max.to_le_bytes()); // mov eax, max
        c.extend([0x39, 0xc1]); // cmp ecx, eax
        c.extend([0x76, 0x02]); // jbe +2
        c.extend([0x89, 0xc1]); // mov ecx, eax
        c.push(0xbe);
        c.extend(src.to_le_bytes()); // mov esi, src
        c.extend([0xf3, 0xa4]); // rep movsb
        c.extend([0x5f, 0x5e]); // pop edi; pop esi
        c.push(0xe9);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(log_stub.wrapping_sub(here).to_le_bytes()); // jmp log_stub
        let addr = self.cave_code(&c);
        let at = self.rebase(slot);
        self.patch(at, &addr.to_le_bytes());
    }

    /// A recording stub (`tag`, `this`, `n` words, `pops`, `ret` as
    /// [`Twins::stub_code`]) that first copies `len` bytes from `src` to
    /// the pointer in its stack word `arg` (what the callee writes there).
    /// Returns its address.
    #[allow(clippy::too_many_arguments)]
    fn stub_code_writes(
        &mut self,
        tag: u32,
        this: bool,
        n: u32,
        pops: u32,
        ret: u32,
        arg: u32,
        src: u32,
        len: u32,
    ) -> u32 {
        let log_stub = self.stub_code(tag, this, n, pops, ret);
        let mut c = vec![0x51, 0x56, 0x57]; // push ecx; push esi; push edi
        c.extend([0x8b, 0x7c, 0x24, (0x10 + 4 * arg) as u8]); // mov edi, [esp+0x10+4arg]
        c.push(0xb9);
        c.extend(len.to_le_bytes()); // mov ecx, len
        c.push(0xbe);
        c.extend(src.to_le_bytes()); // mov esi, src
        c.extend([0xf3, 0xa4]); // rep movsb
        c.extend([0x5f, 0x5e, 0x59]); // pop edi; pop esi; pop ecx
        c.push(0xe9);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(log_stub.wrapping_sub(here).to_le_bytes()); // jmp log_stub
        self.cave_code(&c)
    }

    /// Recording stubs over the originals `(sym, this, words, pops, ret)`.
    fn stubs(&mut self, list: &[(u32, bool, u32, u32, u32)]) {
        for &(sym, this, n, pops, ret) in list {
            self.stub(sym, this, n, pops, ret);
        }
    }

    /// Writes the word at the `ffx.sym` address `sym`.
    fn g32(&mut self, sym: u32, v: u32) {
        let at = self.rebase(sym);
        self.w32(at, v);
    }

    /// [`Twins::stub_code`] written over the original at `sym`.
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
        let addr = self.stub_code(sym, this, n, pops, ret);
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        self.patch(at, &j);
    }

    /// [`Twins::stub_code`] bound to the import table slot `slot`.
    fn stub_import(&mut self, slot: u32, this: bool, n: u32, pops: u32, ret: u32) {
        let addr = self.stub_code(slot, this, n, pops, ret);
        let at = self.rebase(slot);
        self.patch(at, &addr.to_le_bytes());
    }

    /// A fake object whose vtable's slots are recording thiscall stubs
    /// `(words, ret)`, tagged `0x5100 + slot`. Returns the object (16
    /// bytes, the vtable pointer first).
    fn object(&mut self, slots: &[(u32, u32)]) -> u32 {
        let mut vt = Vec::new();
        for (i, &(n, ret)) in slots.iter().enumerate() {
            vt.push(self.stub_code(0x5100 + i as u32, true, n, n, ret));
        }
        let vt = self.words(&vt);
        self.words(&[vt, 0, 0, 0])
    }

    fn diff(&mut self, what: &str) {
        assert_eq!(self.a.diff(&mut self.b).unwrap(), [], "{what}: memory");
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// The original's `ebp` for a call with `args` stack words: its
    /// arguments, the return address and the saved `ebp` below where the
    /// call pushes.
    fn frame_ebp(&self, args: usize) -> u32 {
        self.a.stack_range().end - 0x100 - 4 * args as u32 - 8
    }

    /// Runs the original at `sym` in `a` and `bridge` on `b`; compares the
    /// result as `bridge` says and every byte written.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        bridge: impl FnOnce(&mut G) -> Out,
    ) {
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let poison = self.rebase(POISON_AT);
        let out = bridge(&mut G(&mut self.b, poison));
        assert!(
            ra.st0_raw.is_none(),
            "{what}: the original leaves ST0 set ({:?})",
            ra.st0
        );
        match out {
            Out::None => {}
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::U8(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
            Out::U64(v) => assert_eq!(
                u64::from(ra.edx) << 32 | u64::from(ra.eax),
                v,
                "{what}: edx:eax"
            ),
        }
        self.diff(what);
    }

    /// [`Twins::check`] with the port's frame at the original's `ebp`
    /// (its callees log pointers into it): `low` bytes below `ebp` are
    /// compared after the call.
    fn check_framed(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        low: u32,
        bridge: impl FnOnce(&mut G, u32) -> Out,
    ) {
        self.check_framed_init(what, sym, conv, args, low, &[], bridge);
    }

    /// [`Twins::check_framed`] with the frame's lowest bytes set to `init`
    /// (the rest zero) in both oracles.
    #[allow(clippy::too_many_arguments)]
    fn check_framed_init(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        low: u32,
        init: &[u8],
        bridge: impl FnOnce(&mut G, u32) -> Out,
    ) {
        // the stack words: fastcall's first two are in ecx and edx
        let words = match conv {
            Conv::Fastcall | Conv::FastcallCallerPops => args.len().saturating_sub(2),
            _ => args.len(),
        };
        let ebp = self.frame_ebp(words);
        let lo = ebp - low;
        let mut frame = vec![0; low as usize];
        frame[..init.len()].copy_from_slice(init);
        self.write(lo, &frame);
        self.b.set_call_top(Some(lo - 0x100));
        self.check(what, sym, conv, args, |m| bridge(m, ebp));
        self.b.set_call_top(None);
        let n = (low - self.skip) as usize;
        let mut x = vec![0; n];
        let mut y = vec![0; n];
        Mem::read_bytes(&self.a, lo, &mut x);
        Mem::read_bytes(&self.b, lo, &mut y);
        if let Some(i) = x.iter().zip(&y).position(|(u, v)| u != v) {
            let at = lo + i as u32;
            panic!(
                "{what}: frame byte ebp - {:#x} ({at:#x}): {} != {}",
                ebp - at,
                x[i],
                y[i]
            );
        }
    }
}

// ---- <system_error> ----

#[test]
fn release_facet_matches_the_original() {
    Twins::each(|t| {
        for case in 0..3 {
            t.reset();
            let dead = t.object(&[(1, 0x77)]);
            let facet = t.object(&[(0, 0), (0, 0), (0, if case == 2 { dead } else { 0 })]);
            let holder = t.words(&[if case == 0 { 0 } else { facet }]);
            let what = format!("release_facet #{case}");
            t.check(&what, 0x0040_11d0, Conv::Thiscall(holder), &[], |m| {
                sb::release_facet(m, holder);
                Out::None
            });
        }
    });
}

#[test]
fn category_deleting_dtors_match_the_original() {
    Twins::each(|t| {
        for sym in [0x0040_1260, 0x0040_1430, 0x0040_14e0, 0x0040_1600] {
            for flags in [0, 1, 2, 3, 0xffff_ff00, 0x0000_0101] {
                t.reset();
                t.stub(sd::OPERATOR_DELETE, false, 1, 0, 0x55);
                let this = t.words(&[0x1234_5678, 9]);
                let what = format!("deleting dtor {sym:#x} flags {flags:#x}");
                t.check(&what, sym, Conv::Thiscall(this), &[flags], |m| {
                    Out::U32(sb::category_deleting_dtor(m, this, flags))
                });
            }
        }
    });
}

#[test]
fn equivalent_value_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x2468_ace1);
        for i in 0..40 {
            t.reset();
            let value = rng.word();
            let cat = rng.next();
            let pair = t.words(&[value, cat]);
            let cond_value = if rng.chance(2) { value } else { rng.word() };
            let cond_cat = if rng.chance(3) { rng.next() } else { cat };
            let cond = t.words(&[cond_value, cond_cat]);
            let this = t.object(&[(0, 0), (0, 0), (0, 0), (2, pair)]);
            let code = rng.word();
            let what = format!("equivalent_value #{i}");
            t.check_framed(
                &what,
                0x0040_1330,
                Conv::Thiscall(this),
                &[code, cond],
                8,
                |m, ebp| Out::U32(sb::equivalent_code(m, ebp, this, code, cond)),
            );
        }
    });
}

#[test]
fn equivalent_code_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x1357_2468);
        for i in 0..40 {
            t.reset();
            let this = rng.next();
            let value = rng.word();
            let code = t.words(&[value, if rng.chance(3) { rng.next() } else { this }]);
            let cond = if rng.chance(2) { value } else { rng.word() };
            let what = format!("equivalent_code #{i}");
            t.check(
                &what,
                0x0040_1370,
                Conv::Thiscall(this),
                &[code, cond],
                |m| Out::U32(sb::equivalent_cond(m, this, code, cond)),
            );
        }
    });
}

/// The message functions: the map import answers `text` (0: unknown).
#[test]
fn category_messages_match_the_original() {
    Twins::each(|t| {
        for (sym, slot) in [
            (0x0040_13b0u32, sd::SYSERROR_MAP_IAT),
            (0x0040_1540, sd::WINERROR_MAP_IAT),
            (0x0040_1490, sd::SYSERROR_MAP_IAT),
        ] {
            for (case, code) in [0u32, 1, 2, 5, 0xffff_ffff].into_iter().enumerate() {
                for text in ["", "x", "file not found", "-"] {
                    t.reset();
                    let s = if text == "-" {
                        0
                    } else {
                        let mut b = text.as_bytes().to_vec();
                        b.push(0);
                        t.alloc(&b)
                    };
                    t.stub_import(slot, false, 1, 0, s);
                    t.stub(sd::STRING_ASSIGN, true, 2, 2, 0x99);
                    let out = t.words(&[0x1111_1111; 8]);
                    let what = format!("message {sym:#x} #{case} {text:?}");
                    t.check(&what, sym, Conv::Stdcall, &[out, code], |m| {
                        Out::U32(match sym {
                            0x0040_13b0 => sb::generic_message(m, out, code),
                            0x0040_1540 => sb::system_message(m, out, code),
                            _ => sb::iostream_message(m, out, code),
                        })
                    });
                }
            }
        }
    });
}

#[test]
fn default_condition_matches_the_original() {
    Twins::each(|t| {
        for (code, known) in [(0u32, 0u32), (2, 0x1000), (0xffff_ffff, 0), (5, 1)] {
            t.reset();
            t.stub_import(sd::SYSERROR_MAP_IAT, false, 1, 0, known);
            let out = t.words(&[7, 7]);
            let what = format!("default_condition {code:#x} {known:#x}");
            t.check(&what, 0x0040_15c0, Conv::Stdcall, &[out, code], |m| {
                Out::U32(sb::default_condition(m, out, code))
            });
        }
    });
}

// ---- the streams ----

#[test]
fn sentry_unlock_matches_the_original() {
    Twins::each(|t| {
        for (case, dist) in [0u32, 8, 0x68, 0x68].into_iter().enumerate() {
            t.reset();
            let buf = t.object(&[(0, 0), (0, 0), (0, 0x31)]);
            let vbtable = t.words(&[0, dist]);
            let stream = t.zeros(0x100);
            t.w32(stream, vbtable);
            let rdbuf = if case == 3 { 0 } else { buf };
            t.w32(stream + dist + 0x38, rdbuf);
            let holder = t.words(&[stream]);
            let what = format!("sentry_unlock #{case}");
            t.check(&what, 0x0040_1660, Conv::Thiscall(holder), &[], |m| {
                sb::sentry_unlock(m, holder);
                Out::None
            });
        }
    });
}

#[test]
fn stream_dtors_match_the_original() {
    Twins::each(|t| {
        for (sym, d) in [
            (0x0040_1840u32, sd::STRINGSTREAM_DTOR),
            (0x0040_22b0, sd::ISTRINGSTREAM_DTOR),
            (0x0040_29c0, sd::OSTRINGSTREAM_DTOR),
        ] {
            // the real distance, 0 (the vtable lands on the vbase
            // pointer: the original reads it again) and another
            for dist in [d.ios, 0, 0x10] {
                t.reset();
                t.stub(sd::STRINGBUF_DTOR, true, 0, 0, 0x41);
                t.stub_import(d.base_dtor, true, 0, 0, 0x42);
                t.stub_import(sd::IOS_DTOR_IAT, true, 0, 0, 0x43);
                let vbtable = t.words(&[0, dist]);
                let this = t.words(&[0x5555_5555; 0x40]);
                t.w32(this, vbtable);
                let what = format!("stream dtor {sym:#x} dist {dist:#x}");
                t.check(&what, sym, Conv::Thiscall(this), &[], |m| {
                    Out::U32(sb::stream_dtor(m, this, d))
                });
            }
        }
    });
}

#[test]
fn vtordisp_thunks_match_the_original() {
    Twins::each(|t| {
        for disp in [0u32, 4, 0x60, 0xffff_fff0] {
            t.reset();
            t.stub_import(sd::ISTREAM_VTORDISP_IAT, true, 0, 0, 0x61);
            t.stub(sd::STRINGSTREAM_DELETING, true, 1, 1, 0x62);
            let obj = t.words(&[disp, 0, 0, 0]);
            let this = obj + 4;
            let what = format!("vtordisp_istream {disp:#x}");
            t.check(&what, 0x0040_5a77, Conv::Thiscall(this), &[], |m| {
                Out::U32(sb::vtordisp_istream(m, this))
            });
            for flags in [0, 1] {
                let what = format!("vtordisp_delete {disp:#x} {flags}");
                t.check(&what, 0x0040_5a83, Conv::Thiscall(this), &[flags], |m| {
                    Out::U32(sb::vtordisp_stringstream_delete(m, this, flags))
                });
            }
        }
    });
}

// ---- the settings ----

#[test]
fn settings_construct_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0bad_f00d);
        for case in 0..4 {
            t.reset();
            let fill: Vec<u32> = (0..0x190).map(|_| rng.next()).collect();
            let this = t.words(&fill);
            let path = match case {
                0 => 0,
                1 => t.words(&[0]),
                _ => {
                    let w: Vec<u16> = "C:\\x\\GameSetting.ini\0".encode_utf16().collect();
                    let b: Vec<u8> = w.iter().flat_map(|c| c.to_le_bytes()).collect();
                    t.alloc(&b)
                }
            };
            let what = format!("Settings_Construct #{case}");
            t.check(&what, 0x0040_1880, Conv::Thiscall(this), &[path], |m| {
                Out::U32(st::construct(m, this, path))
            });
        }
    });
}

#[test]
fn settings_detect_quality_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7777_1234);
        let presets = t.rebase(settings::PRESETS);
        let mask = t.rebase(settings::PRESET_MASK);
        let masked: Vec<u32> = (0..24)
            .filter(|&k| Mem::read::<u8>(&t.a, mask + k) != 0)
            .collect();
        assert!(!masked.is_empty(), "the preset mask fixes some values");
        for case in 0..40 {
            t.reset();
            // a preset's values (0..2), then now and then one changed
            let level = rng.below(4);
            let mut v: Vec<u32> = (0..24)
                .map(|k| {
                    if level < 3 {
                        t.r32(presets + 16 * k + 4 * level)
                    } else {
                        rng.word()
                    }
                })
                .collect();
            if rng.chance(3) {
                let k = masked[rng.below(masked.len() as u32) as usize];
                v[k as usize] ^= 1 << rng.below(32);
            }
            if rng.chance(4) {
                // an unmasked value never matters
                let k = (0..24).find(|k| !masked.contains(k)).unwrap_or(0);
                v[k as usize] = rng.next();
            }
            let this = t.words(&v);
            let what = format!("Settings_DetectQuality #{case}");
            t.check(&what, 0x0040_2a20, Conv::Thiscall(this), &[], |m| {
                Out::U32(st::detect_quality(m, this))
            });
        }
    });
}

fn slots() -> Vec<u32> {
    let mut v: Vec<u32> = (0..0x60).collect();
    v.extend([
        0x7f,
        0x80,
        0x81,
        0xff,
        0x100,
        0xffff_ffff,
        0x8000_0000,
        0x1234_5629,
    ]);
    v
}

#[test]
fn key_name_tables_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0f0f_1234);
        for (table, has, get, set) in [
            (st::KEYBOARD, 0x0040_5a90u32, 0x0040_5af0u32, 0x0040_5b30u32),
            (st::CONTROLLER, 0x0040_5ac0, 0x0040_5b10, 0x0040_5b60),
        ] {
            let base = t.rebase(table);
            if !t.snapshot {
                // the image's tables are empty until the menus fill them
                for i in 0..0x80 {
                    let v = if rng.chance(3) { 0 } else { rng.next() };
                    t.w32(base + 4 * i, v);
                }
            }
            for slot in slots() {
                t.reset();
                let what = format!("has {has:#x} {slot:#x}");
                t.check(&what, has, Conv::Cdecl, &[slot], |m| {
                    Out::U32(st::has_name(m, table, slot))
                });
                let what = format!("get {get:#x} {slot:#x}");
                t.check(&what, get, Conv::Cdecl, &[slot], |m| {
                    Out::U32(st::get_name(m, table, slot))
                });
                let name = rng.next();
                let what = format!("set {set:#x} {slot:#x}");
                t.check(&what, set, Conv::Cdecl, &[slot, name], |m| {
                    Out::U8(st::set_name(m, table, slot, name))
                });
            }
        }
        let what = "ReturnConst80";
        t.check(what, 0x0040_5b90, Conv::Cdecl, &[], |_| Out::U32(0x80));
    });
}

#[test]
fn binding_slot_filters_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a5a_1111);
        let mut all = slots();
        all.extend((0..64).map(|_| rng.next()));
        all.extend((0..64).map(|_| rng.next() & 0xffff_ff7f));
        for slot in all {
            for (sym, f) in [
                (0x0040_5cb0u32, st::is_ffx_slot as fn(u32) -> u32),
                (0x0040_5d00, st::is_ffx2_slot),
                (0x0040_5d80, st::is_lm_slot),
            ] {
                let what = format!("filter {sym:#x} {slot:#x}");
                t.check(&what, sym, Conv::Cdecl, &[slot], |_| Out::U32(f(slot)));
            }
        }
    });
}

/// `0x00401690(text; out, max)` (`Conv::FastcallCallerPops`): digits,
/// commas, other bytes (high ones too), empty and null texts, `max` 0, 1,
/// small, 81 and negative. The pointer wrapping to 0 (`inc edi; jne`) is
/// not reached: no text ends at 0xffffffff.
#[test]
fn custom_conv_parse_list_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0169_0001);
        let chars = b"0123456789,,, x-/:\x80\xb0\xff";
        for case in 0..300 {
            t.reset();
            let len = rng.below(48) as usize;
            let mut s: Vec<u8> = (0..len)
                .map(|_| chars[rng.below(chars.len() as u32) as usize])
                .collect();
            if rng.chance(4) {
                // long numbers: wrapping accumulators
                s.extend(b"98765432109876543210,4294967295,4294967296");
            }
            s.push(0);
            let text = if case % 37 == 5 { 0 } else { t.alloc(&s) };
            let out = t.zeros(0x200);
            let max = [0u32, 1, 2, 3, 7, 0x51, 0xffff_ffff, 0x8000_0000][rng.below(8) as usize];
            let what = format!(
                "0x00401690 #{case} {:?} max {max:#x}",
                String::from_utf8_lossy(&s)
            );
            t.check(
                &what,
                0x0040_1690,
                Conv::FastcallCallerPops,
                &[text, out, max],
                |m| Out::U32(st::parse_list(m, text, out, max)),
            );
        }
    });
}

/// `Settings_ValidateBindings(list; n, filter)` (`Conv::FastcallCallerPops`)
/// with the three real slot filters and recording stubs that take every
/// slot or none (the log checks the order of the filter calls): valid
/// distinct codes (the 1 path), a code without a name, codes 0x80 and
/// negative, duplicates, zero slots below and above 0x31, `n` 0, small, 81,
/// 0x60 and negative. On the image the key-name tables are random; on the
/// snapshots they are the game's own.
#[test]
fn custom_conv_validate_bindings_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x05bb_0002);
        let (kb, pad) = (t.rebase(st::KEYBOARD), t.rebase(st::CONTROLLER));
        if !t.snapshot {
            for i in 0..0x80 {
                let v = if rng.chance(4) { 0 } else { rng.next() | 1 };
                t.w32(kb + 4 * i, v);
                let v = if rng.chance(4) { 0 } else { rng.next() | 1 };
                t.w32(pad + 4 * i, v);
            }
            // code 0 named, so that zero slots below 0x31 pass; the words
            // past both tables: a code 0x80 read as a name would find one
            t.w32(kb, 0x5555_0002);
            t.w32(pad, 0x5555_0003);
            t.w32(kb + 0x200, 0x5555_0001);
            t.w32(pad + 0x200, 0x5555_0001);
        }
        let named = |t: &Twins, base: u32| -> Vec<u32> {
            (1..0x80).filter(|&c| t.r32(base + 4 * c) != 0).collect()
        };
        let (kb_named, pad_named) = (named(t, kb), named(t, pad));
        for case in 0..400 {
            t.reset();
            let n = [
                0u32,
                1,
                3,
                0x10,
                0x1d,
                0x30,
                0x40,
                0x51,
                0x51,
                0x51,
                0x60,
                0xffff_ffff,
            ][rng.below(12) as usize];
            let words = n.min(0x60) as usize;
            // codes distinct across both tables, each named in its own:
            // a lookup in the wrong table finds the other's zeros
            let mut used = [false; 0x80];
            let mut list = vec![0u32; 0x60];
            for (i, w) in list.iter_mut().enumerate().take(words) {
                let pad_slot = (14..=27).contains(&i) || i == 44 || i == 45;
                let pool: Vec<u32> = if pad_slot { &pad_named } else { &kb_named }
                    .iter()
                    .copied()
                    .filter(|&c| !used[c as usize])
                    .collect();
                *w = if rng.chance(6) || pool.is_empty() {
                    0
                } else {
                    let c = pool[rng.below(pool.len() as u32) as usize];
                    used[c as usize] = true;
                    c
                };
            }
            // some cases broken in one slot
            let fault = rng.below(6);
            if words > 0 && fault < 4 {
                let i = rng.below(words as u32) as usize;
                list[i] = match fault {
                    0 => [0x80, 0xffff_ffff, 0x8000_0000, 0x7fff_ffff][rng.below(4) as usize],
                    // a code without a name (when there is one)
                    1 => (0..0x80).find(|&c| t.r32(kb + 4 * c) == 0).unwrap_or(0),
                    2 => (0..0x80).find(|&c| t.r32(pad + 4 * c) == 0).unwrap_or(0),
                    // a duplicate of another slot's code
                    _ => list[rng.below(words as u32) as usize],
                };
            }
            let at = t.words(&list);
            let filter = match rng.below(5) {
                0 => t.rebase(0x0040_5cb0),
                1 => t.rebase(0x0040_5d00),
                2 => t.rebase(0x0040_5d80),
                3 => t.stub_code(0x5f00, false, 1, 0, 0x7fff_ff01),
                _ => t.stub_code(0x5f01, false, 1, 0, 0x0000_0100),
            };
            let what = format!("Settings_ValidateBindings #{case} n {n:#x} fault {fault}");
            t.check(
                &what,
                0x0040_5bb0,
                Conv::FastcallCallerPops,
                &[at, n, filter],
                |m| Out::U32(st::validate_bindings(m, at, n, filter)),
            );
        }
    });
}

/// `Settings_JoinBindings(out; list, n)` (`Conv::FastcallCallerPops`)
/// with every callee stubbed: the stream constructor, the comma writer,
/// `operator<<(int)`, `str()`, `operator delete`, `setg` and the three
/// destructors log their calls. The frame (`ebp - 0xcc` up) holds what
/// the constructor would have built: the vbase table pointer (offset
/// 0x68 or 0x60), the buffer-owned flag at `ebp - 0x78` (both values),
/// the buffer pointer cell at `ebp - 0xa8` and the three pointer cells
/// the destructor clears; the rest random. `n` 0, 1, several and
/// negative. The SEH registration and the cookie (`ebp - 0x10` up) are
/// left out of the frame compare.
#[test]
fn custom_conv_join_bindings_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x16e0_0003);
        t.skip = 0x10;
        for case in 0..40 {
            t.reset();
            let n = [0u32, 1, 2, 5, 0x51, 0xffff_ffff][rng.below(6) as usize];
            let list: Vec<u32> = (0..0x51).map(|_| rng.word()).collect();
            let list = t.words(&list);
            let out = t.zeros(0x20);
            let dist = [0x68u32, 0x60][rng.below(2) as usize];
            let vbtable = t.words(&[0, dist]);
            let block = t.words(&[rng.next()]);
            let cells = t.words(&[rng.next(), rng.next(), rng.next()]);
            let mut frame: Vec<u32> = (0..0xcc / 4).map(|_| rng.next()).collect();
            // word k of the frame is at ebp - 0xcc + 4k
            let mut set = |off: u32, v: u32| frame[((0xcc - off) / 4) as usize] = v;
            set(0xcc, vbtable);
            set(0x78, rng.next() & !1 | u32::from(rng.chance(2)));
            set(0xa8, block);
            set(0xa4, cells);
            set(0x94, cells + 4);
            set(0x84, cells + 8);
            let init: Vec<u8> = frame.iter().flat_map(|w| w.to_le_bytes()).collect();
            let list_stubs = [
                (st::STRINGSTREAM_CTOR, true, 0, 2, 0x5a5a_0001),
                (st::PUT_COMMA, true, 0, 0, 0x0c0c_0002),
                (st::STRINGSTREAM_STR, true, 1, 1, 0x5a5a_0003),
                (st::OPERATOR_DELETE, false, 1, 0, 0x5a5a_0004),
            ];
            t.stubs(&list_stubs);
            for (slot, n, ret) in [
                (st::OSTREAM_INT_IAT, 1, 0x1111_0001),
                (st::SETG_IAT, 3, 0x1111_0002),
                (st::STREAMBUF_DTOR_IAT, 0, 0x1111_0003),
                (st::IOSTREAM_DTOR_IAT, 0, 0x1111_0004),
                (st::IOS_DTOR_IAT, 0, 0x1111_0005),
            ] {
                t.stub_import(slot, true, n, n, ret);
            }
            let what = format!("Settings_JoinBindings #{case} n {n:#x} dist {dist:#x}");
            t.check_framed_init(
                &what,
                0x0040_16e0,
                Conv::FastcallCallerPops,
                &[out, list, n],
                0xcc,
                &init,
                |m, ebp| Out::U32(st::join_bindings(m, ebp, out, list, n)),
            );
        }
        t.skip = 0;
    });
}

/// A by-value `std::string` (six words) holding `text`: in place below
/// 16 bytes, else on the heap (`alloc`) with capacity `text.len()` or more.
fn by_value_string(t: &mut Twins, text: &[u8], heap: bool) -> [u32; 6] {
    let mut w = [0u32; 6];
    if heap || text.len() >= 16 {
        let mut z = text.to_vec();
        z.push(0);
        let p = t.alloc(&z);
        w[0] = p;
        w[5] = (text.len() as u32).max(0x10);
    } else {
        let mut b = [0u8; 16];
        b[..text.len()].copy_from_slice(text);
        for (i, c) in b.chunks(4).enumerate() {
            w[i] = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
        }
        w[5] = 0xf;
    }
    w[4] = text.len() as u32;
    w
}

/// `Settings_SetValue(this; key, value)` (thiscall, `ret 0x1c`): key rows
/// of 0..8 strings (set here in both oracles) so that every path runs:
/// the `_stricmp` search (a stub answering a random sequence: a match at
/// each index, none, an empty row), the one-string keys 1 (`find` hit or
/// miss, the two substrings in the frame with in-place or heap buffers,
/// the frame state byte), 0x15..=0x17 (the binding lists through the
/// real `Settings_ValidateBindings` and slot filters) and any other
/// (`atoi`); the value in place and on the heap (`operator delete`). The
/// string's words sit at the original's argument slots in both oracles.
#[test]
#[allow(clippy::too_many_lines)] // one case setup, every path
fn custom_conv_set_value_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x1930_0004);
        t.skip = 0x14;
        let (kb, pad) = (t.rebase(st::KEYBOARD), t.rebase(st::CONTROLLER));
        if !t.snapshot {
            // the key-name tables as in the validate test: code 0 named
            for i in 0..0x80 {
                let v = if i > 0 && rng.chance(4) {
                    0
                } else {
                    rng.next() | 1
                };
                t.w32(kb + 4 * i, v);
                let v = if i > 0 && rng.chance(4) {
                    0
                } else {
                    rng.next() | 1
                };
                t.w32(pad + 4 * i, v);
            }
        }
        let texts: [&[u8]; 6] = [
            b"1920*1080",
            b"1280x720",
            b"3,4,5,6,0,0,7",
            b"1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29",
            b"HIGH",
            b"",
        ];
        for case in 0..160 {
            t.reset();
            let key = [0u32, 1, 1, 2, 5, 13, 0x15, 0x16, 0x17, 0x17, 0x0b][rng.below(11) as usize];
            let count = [0u32, 1, 1, 1, 2, 3, 8][rng.below(7) as usize];
            let row = t.rebase(st::VALUE_STRINGS) + key * 0x20;
            let mut words = [0u32; 8];
            for w in words.iter_mut().take(count as usize) {
                *w = 0x0012_0000 | rng.below(0x1000);
            }
            let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
            t.patch(row, &bytes);
            let this = t.alloc(&(0..0x640).map(|_| rng.next() as u8).collect::<Vec<_>>());
            // or a long list: 0x58 codes, past the tables' 0x51 words,
            // distinct and named in their slot's table (the validation's
            // 1 path), but sometimes one unnamed code in a slot only some
            // of the filters take (0x24: FFX's, not FFX-2's)
            let mut used = [false; 0x80];
            let mut codes = Vec::new();
            for i in 0..0x58u32 {
                let table = if (14..=27).contains(&i) || i == 44 || i == 45 {
                    pad
                } else {
                    kb
                };
                let pool: Vec<u32> = (1..0x80u32)
                    .filter(|&c| !used[c as usize] && t.r32(table + 4 * c) != 0)
                    .collect();
                let c = if pool.is_empty() {
                    0
                } else {
                    pool[rng.below(pool.len() as u32) as usize]
                };
                used[c as usize] = true;
                codes.push(c);
            }
            let unnamed = if rng.chance(2) {
                (1..0x80u32).find(|&c| !used[c as usize] && t.r32(kb + 4 * c) == 0)
            } else {
                None
            };
            if let Some(c) = unnamed {
                codes[0x24] = c;
            }
            let long: Vec<u8> = codes
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",")
                .into_bytes();
            let k = rng.below(texts.len() as u32 + 2) as usize;
            let text: &[u8] = if k < texts.len() { texts[k] } else { &long };
            let value = by_value_string(t, text, rng.chance(3));
            let seq: Vec<u32> = (0..8).map(|_| u32::from(!rng.chance(3))).collect();
            t.stub_import_seq(st::STRICMP_IAT, 2, 0, &seq);
            // printf: the Resolution message has one argument, the other two
            let printf_words = if key == 1 && count == 1 { 2 } else { 3 };
            t.stub_import(st::PRINTF_IAT, false, printf_words, 0, 0x2222_0001);
            let numbers = [rng.next(), rng.next()];
            t.stub_import_seq(st::ATOI_IAT, 1, 0, &numbers);
            let pos = [u32::MAX, 0, 4, 0x10][rng.below(4) as usize];
            t.stubs(&[
                (st::STRING_FIND, true, 3, 3, pos),
                (st::STRING_SUBSTR, true, 3, 3, 0x2222_0002),
                (st::OPERATOR_DELETE, false, 1, 0, 0x2222_0003),
            ]);
            // the frame: the two substrings' words (in place or heap)
            let mut frame: Vec<u32> = (0..0x50 / 4).map(|_| rng.next()).collect();
            for off in [0x44u32, 0x2c] {
                let cap = if rng.chance(2) { 0xf } else { 0x20 };
                frame[((0x50 - off + 0x14) / 4) as usize] = cap;
            }
            let init: Vec<u8> = frame.iter().flat_map(|w| w.to_le_bytes()).collect();
            let mut args = vec![key];
            args.extend(value);
            let ebp = t.frame_ebp(args.len());
            let slots: Vec<u8> = args.iter().flat_map(|w| w.to_le_bytes()).collect();
            t.write(ebp + 8, &slots);
            let what = format!(
                "Settings_SetValue #{case} key {key:#x} row {count} {:?} this {this:#x} ebp {ebp:#x} value {value:x?}",
                String::from_utf8_lossy(text)
            );
            t.check_framed_init(
                &what,
                0x0040_1930,
                Conv::Thiscall(this),
                &args,
                0x50,
                &init,
                |m, ebp| Out::U32(st::set_value(m, ebp, this, key, ebp + 0xc)),
            );
        }
        t.skip = 0;
    });
}

/// `Settings_Save(this)` and `Settings_Load(this)` (thiscall) with the
/// CRT's file functions, `Settings_Format`, `Settings_Parse`,
/// `Settings_CopyDefaultTables` and (for the load without a file)
/// `Settings_Save` stubbed: lengths 0, small and large, `_wfopen`
/// failing and not. The `alloca` buffer is where the original's
/// `_alloca_probe_16` puts it (16-aligned below `ebp - 0x14`), so the
/// logged pointers agree.
#[test]
fn settings_save_load_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x1d80_0005);
        for case in 0..48 {
            t.reset();
            let this = t.alloc(&(0..0x640).map(|_| rng.next() as u8).collect::<Vec<_>>());
            let file = if rng.chance(3) { 0 } else { 0x0bad_f11e };
            let len = [0u32, 1, 0x35, 0x400, 0xfff][rng.below(5) as usize];
            t.stub_import(st::WFOPEN_IAT, false, 2, 0, file);
            for (slot, n) in [
                (st::FWRITE_IAT, 4),
                (st::FCLOSE_IAT, 1),
                (st::FSEEK_IAT, 3),
                (st::FREAD_IAT, 4),
            ] {
                t.stub_import(slot, false, n, 0, 0x3333_0000 + n);
            }
            t.stub_import(st::FTELL_IAT, false, 1, 0, len);
            t.stubs(&[
                (st::FORMAT, true, 1, 1, len),
                (st::PARSE, true, 1, 1, 0x3333_0010),
                (st::COPY_DEFAULT_TABLES, true, 0, 0, 0x3333_0011),
            ]);
            let ebp = t.frame_ebp(0);
            let alloca = move |n: u32| (ebp - 0x14).wrapping_sub(n) & !0xf;
            let what = format!("Settings_Save #{case} file {file:#x} len {len:#x}");
            t.check(&what, st::SAVE, Conv::Thiscall(this), &[], |m| {
                Out::U32(st::save(m, this, alloca))
            });
            // Settings_Load's own call of Settings_Save, stubbed
            t.stub(st::SAVE, true, 0, 0, 0x3333_0012);
            let what = format!("Settings_Load #{case} file {file:#x} len {len:#x}");
            t.check(&what, 0x0040_1cb0, Conv::Thiscall(this), &[], |m| {
                Out::U32(st::load(m, this, alloca))
            });
        }
    });
}

// ---- the platform glue (m5a_platform) ----

/// `fiosClose`, `sceClose`, `sceWrite` and `SaveDataIsSaveEnabled` with
/// their callees stubbed: null and real files, `closeFile` answering 0, 1,
/// 2 and 0x101 (only `al` counts).
#[test]
fn platform_files_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0679_0006);
        for case in 0..32 {
            t.reset();
            let closed = [0u32, 1, 2, 0x101][rng.below(4) as usize];
            t.stubs(&[
                (pf::CLOSE_FILE, true, 0, 0, closed),
                (pf::OPERATOR_DELETE, false, 1, 0, 0x4444_0001),
                (pf::DBG_PRINTF, false, 2, 0, 0x4444_0002),
                (pf::SAVE_MANAGER, false, 0, 0, 0x4444_0003),
                (pf::SAVE_IS_ENABLED, true, 0, 0, rng.word()),
            ]);
            let f = if rng.chance(4) { 0 } else { t.zeros(0x40) };
            let what = format!("fiosClose #{case} {f:#x} closed {closed:#x}");
            t.check(&what, 0x0067_9510, Conv::Cdecl, &[f], |m| {
                Out::U32(pf::fios_close(m, f))
            });
            let what = format!("sceClose #{case} {f:#x} closed {closed:#x}");
            t.check(&what, 0x0062_f610, Conv::Cdecl, &[f], |m| {
                Out::U32(pf::sce_close(m, f))
            });
            let buf = rng.next();
            let what = format!("sceWrite #{case}");
            t.check(&what, 0x0062_fdc0, Conv::Cdecl, &[f, buf, 7], |m| {
                Out::U32(pf::sce_write(m, buf))
            });
            let what = format!("SaveDataIsSaveEnabled #{case}");
            t.check(&what, 0x0064_80e0, Conv::Cdecl, &[], |m| {
                Out::U32(pf::is_save_enabled(m))
            });
        }
    });
}

/// `0x0061b3c0(module, info, id)` with `LoadResource`, `LockResource` and
/// `SizeofResource` stubbed (stdcall): a block of 16 length-prefixed
/// UTF-16 strings (empty ones too), sizes that cut it short, both
/// failures, ids with every low nibble and high bits.
#[test]
fn platform_string_resource_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x061b_0007);
        for case in 0..200 {
            t.reset();
            let mut block = Vec::new();
            for _ in 0..16 {
                let n = if rng.chance(3) {
                    0
                } else {
                    rng.below(6) as u16
                };
                block.extend(n.to_le_bytes());
                for _ in 0..n {
                    block.extend((rng.next() as u16).to_le_bytes());
                }
            }
            let full = block.len() as u32;
            let at = t.alloc(&block);
            let global = if rng.chance(8) { 0 } else { 0x5151_0000 };
            let locked = if rng.chance(8) { 0 } else { at };
            // string starts, for sizes that end exactly at one
            let mut starts = vec![0u32];
            let mut q = 0usize;
            while q < block.len() {
                q += 2 + 2 * usize::from(u16::from_le_bytes([block[q], block[q + 1]]));
                starts.push(q as u32);
            }
            let size = match rng.below(4) {
                0 => full,
                1 => rng.below(full + 1),
                2 => starts[rng.below(starts.len() as u32) as usize],
                _ => full + 8,
            };
            for (slot, n, ret) in [
                (pf::LOAD_RESOURCE_IAT, 2, global),
                (pf::LOCK_RESOURCE_IAT, 1, locked),
                (pf::SIZEOF_RESOURCE_IAT, 2, size),
            ] {
                t.stub_import(slot, false, n, n, ret);
            }
            let id = rng.below(16) | if rng.chance(3) { rng.next() & !0xf } else { 0 };
            let (module, info) = (0x0040_0000, rng.next());
            let what = format!("0x0061b3c0 #{case} id {id:#x} size {size}/{full}");
            t.check(&what, 0x0061_b3c0, Conv::Cdecl, &[module, info, id], |m| {
                Out::U32(pf::string_resource_image(m, module, info, id))
            });
        }
    });
}

/// The achievement records (`g_AchievementRecords`, set in both oracles:
/// all unlocked but a few, so that the platinum's path runs too) with
/// `SteamUserStats` returning a fake object whose vtable slots 7
/// (`SetAchievement`) and 10 (`StoreStats`) log, the manager's flag
/// `+0x4d` on and off, ids 0..=0x25 (past 0x21 too), `g_SteamReady` on and
/// off.
#[test]
fn platform_achievements_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x06f1_0008);
        let records = t.rebase(platform::RECORDS);
        for case in 0..160 {
            t.reset();
            let mut bytes: Vec<u8> = (0..34 * platform::RECORD + 0x400)
                .map(|_| rng.next() as u8)
                .collect();
            let missing = rng.below(4);
            for id in 0..34u32 {
                let at = (id * platform::RECORD + 0x184) as usize;
                bytes[at] = u8::from(id != 0 || rng.chance(4));
            }
            for _ in 0..missing {
                let id = rng.below(34);
                bytes[(id * platform::RECORD + 0x184) as usize] = 0;
            }
            t.patch(records, &bytes);
            let slots: Vec<(u32, u32)> = (0..11)
                .map(|i| (u32::from(i == 7), 0x4545_0000 + i))
                .collect();
            let stats = t.object(&slots);
            t.stub_import(pf::STEAM_USER_STATS_IAT, false, 0, 0, stats);
            let manager = t.zeros(0x80);
            t.write(manager + 0x4d, &[u8::from(!rng.chance(5))]);
            t.stubs(&[(pf::ACHIEVEMENTS, false, 0, 0, manager)]);
            let steam = t.rebase(platform::STEAM_READY);
            t.patch(steam, &[u8::from(!rng.chance(5))]);
            let id = rng.below(0x26);
            let what = format!("SetAchievement #{case} id {id:#x}");
            t.check(&what, 0x006f_1900, Conv::Thiscall(manager), &[id], |m| {
                pf::set_achievement(m, manager, id);
                Out::None
            });
            // half the cases: only this id and the platinum missing, so
            // that the unlock goes on to the platinum (0x21 often)
            let id = if rng.chance(2) {
                let id = [0x21, 0x20, 1, 0x22, rng.below(0x26)][rng.below(5) as usize];
                let mut set =
                    |id: u32, v: u8| t.write(records + id * platform::RECORD + 0x184, &[v]);
                for k in 0..34 {
                    set(k, 1);
                }
                set(0, 0);
                if id < 34 {
                    set(id, 0);
                }
                id
            } else {
                rng.below(0x26)
            };
            let what = format!("Unlock #{case} id {id:#x}");
            t.check(&what, 0x006f_1510, Conv::Thiscall(manager), &[id], |m| {
                pf::unlock(m, manager, id);
                Out::None
            });
            if rng.chance(2) {
                // the platinum and at most one other missing (1 and 0x21
                // often: the ends of the scan)
                for k in 0..34u32 {
                    t.write(records + k * platform::RECORD + 0x184, &[u8::from(k != 0)]);
                }
                let k = [1u32, 0x21, 0x22, 0, rng.below(34)][rng.below(5) as usize];
                t.write(records + k * platform::RECORD + 0x184, &[0]);
            }
            let what = format!("AchievementTryObtainPlat #{case}");
            t.check(&what, 0x006f_1550, Conv::Thiscall(manager), &[], |m| {
                pf::try_obtain_platinum(m, manager);
                Out::None
            });
            let id = rng.below(0x26);
            let what = format!("SaveDataUnlockAchievement #{case} id {id:#x}");
            t.check(&what, 0x0064_88c0, Conv::Cdecl, &[id], |m| {
                pf::unlock_achievement(m, id);
                Out::None
            });
        }
    });
}

/// `fiosOpen(name, mode)` with its callees stubbed: `operator new`
/// failing or not, `isOpen` 0, 1 and 0x100 (only `al`), the error path
/// with and without the string resource and with the `CString`'s
/// reference count reaching 0 or not (a fake string manager: its vtable
/// slot 3 gives a nil string block whose owner's slot 1 frees it). The
/// `CString` lives in the `mode` argument slot in both oracles.
#[test]
fn platform_fios_open_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0679_0009);
        let manager = t.rebase(pf::STRING_MANAGER);
        for case in 0..80 {
            t.reset();
            let owner = t.object(&[(0, 0), (1, 0x5252_0001)]);
            let refs = [1u32, 2, 0, 0xffff_fffb][rng.below(4) as usize];
            let nil = t.words(&[owner, 0, 0, refs, 0, 0]);
            let fake = t.object(&[(0, 0), (0, 0), (0, 0), (0, nil)]);
            let vt = t.r32(fake);
            t.patch(manager, &vt.to_le_bytes());
            let file = if rng.chance(5) { 0 } else { t.zeros(8) };
            let open = [0u32, 1, 0x100, 0x7f01][rng.below(4) as usize];
            let info = if rng.chance(2) { 0 } else { 0x5252_0002 };
            t.stubs(&[
                (pf::OPERATOR_NEW, false, 1, 0, file),
                (pf::STREAM_FILE_CTOR, true, 5, 5, 0x5252_0003),
                (pf::STREAM_FILE_IS_OPEN, true, 0, 0, open),
                (pf::OPERATOR_DELETE, false, 1, 0, 0x5252_0004),
                (pf::FIND_STRING_RESOURCE, false, 2, 0, info),
                (pf::LOAD_STRING_RESOURCE, true, 2, 2, 0x5252_0005),
                (pf::RESET_TO_LAUNCHER, false, 0, 0, 0x5252_0006),
            ]);
            t.stub_import(pf::SHOW_CURSOR_IAT, false, 1, 1, 0x5252_0007);
            t.stub_import(pf::MESSAGE_BOX_IAT, false, 4, 4, 0x5252_0008);
            let name = rng.next();
            let mode = rng.word();
            let what = format!(
                "fiosOpen #{case} file {file:#x} open {open:#x} info {info:#x} refs {refs:#x}"
            );
            t.check_framed(
                &what,
                0x0067_9730,
                Conv::Cdecl,
                &[name, mode],
                0,
                |m, ebp| Out::U32(pf::fios_open(m, name, mode, ebp + 0xc)),
            );
        }
    });
}

/// `sceOpen(name, mode)` with `memset`, `memcpy`, `fiosUnifyFilename`,
/// `fiosIsFileExist` and `fiosOpen` stubbed: names of 0..40 bytes after
/// the 7-byte device, the file there or not, `fiosOpen` failing or not.
/// The frame (two 0x400-byte paths) is compared but for the cookie.
#[test]
fn platform_sce_open_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x062f_000a);
        t.skip = 4;
        for case in 0..40 {
            t.reset();
            let mut name = b"cdrom0:".to_vec();
            for _ in 0..rng.below(40) {
                name.push(b'a' + rng.below(26) as u8);
            }
            name.push(0);
            let name = t.alloc(&name);
            let exists = u32::from(!rng.chance(3));
            let file = if rng.chance(3) { 0 } else { 0x5353_0001 };
            t.stubs(&[
                (pf::MEMSET, false, 3, 0, 0x5353_0002),
                (pf::MEMCPY, false, 3, 0, 0x5353_0003),
                (pf::UNIFY_FILENAME, false, 3, 0, 0x5353_0004),
                (pf::FILE_EXISTS, false, 1, 0, exists),
                (pf::FIOS_OPEN, false, 2, 0, file),
            ]);
            let mode = rng.word();
            let what = format!("sceOpen #{case} exists {exists} file {file:#x}");
            // a random frame: the stubs write nothing, so every byte the
            // two write shows (the prefix's terminator stays the frame's)
            let init: Vec<u8> = (0..pf::SCE_OPEN_FRAME).map(|_| rng.next() as u8).collect();
            t.check_framed_init(
                &what,
                0x0062_fa30,
                Conv::Cdecl,
                &[name, mode],
                pf::SCE_OPEN_FRAME,
                &init,
                |m, ebp| Out::U32(pf::sce_open(m, ebp, name, mode)),
            );
        }
        t.skip = 0;
    });
}

/// `TOSetEnvMovieMessage`, `TOGetFFXLangJmp` and
/// `ToDrawEternalCalmCopyRight` with `Sg_GetDispMode`, `TOGetFFXLang` and
/// `graphicDrawUIElement` stubbed: the emulated GS register page mapped in
/// both oracles, the fade at every edge (`frame` negative, 0x1f, 0x20,
/// 0x10f, 0x110 and up; the fade's word from -8 to 0x90). The copyright
/// quad's frame is compared (zero where the original never writes: the
/// junk-read words) but for the cookie.
#[test]
fn platform_title_glue_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x08af_000b);
        t.a.map_zeroed(pf::GS_PMODE, 0x1000).unwrap();
        t.b.map_zeroed(pf::GS_PMODE, 0x1000).unwrap();
        for case in 0..60 {
            t.reset();
            let mode = [0u32, 1, 5, 0x100][rng.below(4) as usize];
            let lang = [0u32, 1, 2, 9][rng.below(4) as usize];
            t.stubs(&[
                (pf::GET_DISP_MODE, false, 0, 0, mode),
                (pf::GET_FFX_LANG, false, 0, 0, lang),
                (pf::DRAW_UI_ELEMENT, false, 5, 0, 0x5454_0001),
            ]);
            t.w32(pf::GS_PMODE, rng.next());
            t.w32(pf::GS_PMODE + 4, rng.next());
            let what = format!("TOSetEnvMovieMessage #{case} mode {mode:#x}");
            t.check(&what, 0x008a_f590, Conv::Cdecl, &[], |m| {
                Out::U32(pf::set_env_movie_message(m))
            });
            let what = format!("TOGetFFXLangJmp #{case}");
            t.check(&what, 0x008a_c2f0, Conv::Cdecl, &[], |m| {
                Out::U32(pf::get_ffx_lang_jmp(m))
            });
            let alpha =
                [0xffff_fff8u32, 0, 3, 4, 0x7c, 0x7d, 0x80, 0x84, 0x90][rng.below(9) as usize];
            t.g32(pf::COPYRIGHT_ALPHA, alpha);
            let frame = [0xffff_fff0u32, 0, 0x1f, 0x20, 0x10f, 0x110, 0x400][rng.below(7) as usize];
            t.skip = 4;
            let what = format!(
                "ToDrawEternalCalmCopyRight #{case} frame {frame:#x} alpha {alpha:#x} lang {lang}"
            );
            t.check_framed(
                &what,
                0x0090_4a70,
                Conv::Cdecl,
                &[frame],
                pf::COPYRIGHT_FRAME,
                |m, ebp| Out::U32(pf::draw_copyright(m, ebp, frame)),
            );
            t.skip = 0;
        }
    });
}

/// `JobSchedule_JobScheduler_waitForComplete(this; queue)` and
/// `JobScheduleWait_Ch_Cloth_PerCharacter` with the semaphore imports
/// stubbed (stdcall): queues 0..2, counts below, at and above 8 and
/// negative.
#[test]
fn platform_job_waits_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x06db_000c);
        for case in 0..40 {
            t.reset();
            t.stub_import(pf::WAIT_FOR_SINGLE_OBJECT_IAT, false, 2, 2, 0);
            t.stub_import(pf::RELEASE_SEMAPHORE_IAT, false, 3, 3, 1);
            let this = t.alloc(
                &(0..0x1e8 * 3 + 0x300)
                    .map(|_| rng.next() as u8)
                    .collect::<Vec<_>>(),
            );
            for q in 0..3 {
                let count = [0u32, 7, 8, 9, 0xffff_ffff][rng.below(5) as usize];
                t.w32(this + q * 0x1e8 + 0x1f8, count);
            }
            let queue = rng.below(3);
            let what = format!("waitForComplete #{case} queue {queue}");
            t.check(&what, 0x006d_bf50, Conv::Thiscall(this), &[queue], |m| {
                pf::wait_for_complete(m, this, queue);
                Out::None
            });
            t.g32(pf::CLOTH_SCHEDULER, this);
            let what = format!("JobScheduleWait_Ch_Cloth_PerCharacter #{case}");
            t.check(&what, 0x00a4_4b30, Conv::Cdecl, &[], |m| {
                pf::wait_cloth(m);
                Out::None
            });
        }
    });
}

/// The loading screen's `unloadTextures`, `terminate` and destructor
/// (`0x00634790`), `CDataFrameQueue_dtor` and `VbfMkvReader_dtor`, with
/// the cluster manager, `PCluster`'s destructor, `PhyreFree` and
/// `operator delete` stubbed: the clusters at `+0x28` and `+0x34` set or
/// not, the arrays' counts negative or not, the reader's file set or not.
#[test]
fn platform_dtors_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x06e5_000d);
        for case in 0..40 {
            t.reset();
            t.stubs(&[
                (pf::CLUSTER_MANAGER, false, 0, 0, 0x5555_0001),
                (pf::RELEASE_CLUSTER, true, 1, 1, 0x5555_0002),
                (pf::CLUSTER_DTOR, true, 0, 0, 0x5555_0003),
                (pf::PHYRE_FREE, false, 1, 0, 0x5555_0004),
                (pf::OPERATOR_DELETE, false, 1, 0, 0x5555_0005),
            ]);
            let clusters: Vec<u32> = (0..20).map(|_| rng.next()).collect();
            let clusters = t.words(&clusters);
            let mut words: Vec<u32> = (0..16).map(|_| rng.next()).collect();
            words[0x1c / 4] = clusters;
            words[0x28 / 4] = if rng.chance(2) { 0 } else { rng.next() };
            words[0x34 / 4] = if rng.chance(2) { 0 } else { rng.next() };
            words[0x18 / 4] = [0u32, 3, 0x8000_0000, 0xffff_ffff][rng.below(4) as usize];
            words[0x20 / 4] = [0u32, 3, 0x8000_0000, 0xffff_ffff][rng.below(4) as usize];
            for (sym, name) in [
                (0x006e_5050u32, "unloadTextures"),
                (0x006e_5020, "terminate"),
                (0x0063_4790, "dtor"),
            ] {
                let this = t.words(&words);
                let what = format!("LoadingScreen {name} #{case}");
                t.check(&what, sym, Conv::Thiscall(this), &[], |m| match sym {
                    0x006e_5050 => Out::U32(pf::unload_textures(m, this)),
                    0x006e_5020 => Out::U32(pf::terminate(m, this)),
                    _ => {
                        pf::loading_screen_dtor(m, this);
                        Out::None
                    }
                });
            }
            let this = t.words(&[
                rng.next(),
                0,
                0,
                0,
                if rng.chance(2) { 0 } else { rng.next() },
            ]);
            let what = format!("CDataFrameQueue_dtor #{case}");
            t.check(&what, 0x00a3_dd50, Conv::Thiscall(this), &[], |m| {
                pf::data_frame_queue_dtor(m, this);
                Out::None
            });
            let what = format!("VbfMkvReader_dtor #{case}");
            t.check(&what, 0x00a3_e040, Conv::Thiscall(this), &[], |m| {
                pf::vbf_mkv_reader_dtor(m, this);
                Out::None
            });
        }
    });
}

/// `0x00633fe0`, `0x00634220`, `0x006e3f10`, `0x00691b00`, `0x0069a0e0`
/// and `code_00686fe0` with their callees stubbed. The tree's remove stub
/// unlinks nothing, so the first node is moved by hand: a stub that
/// writes the next node into `+0xc` (a chain of 0..4 nodes ending at the
/// tree itself or at null; buffers set or not). The list: 0..4 nodes; the
/// vector's block set or not. The managers: present or not.
#[test]
fn platform_teardown_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0633_000e);
        for case in 0..60 {
            t.reset();
            // the tree: nodes chained through +0 (the remove stub follows it)
            let n = rng.below(5) as usize;
            let tree = t.zeros(0x40);
            let mut nodes = Vec::new();
            for _ in 0..n {
                let buf = if rng.chance(3) { 0 } else { rng.next() };
                let mut w = vec![0u32; 8];
                w[6] = buf;
                nodes.push(t.words(&w));
            }
            let end = if rng.chance(2) { tree } else { 0 };
            for i in 0..n {
                let next = if i + 1 < n { nodes[i + 1] } else { end };
                t.w32(nodes[i], next);
            }
            t.w32(tree + 0xc, nodes.first().copied().unwrap_or(end));
            // remove(this; node): this+0xc = [node], then the logging
            // stub (ret 4)
            let log = t.stub_code(pf::TREE_REMOVE, true, 1, 1, 0);
            let mut c = vec![0x60]; // pushad
            c.extend([0x8b, 0x44, 0x24, 0x24]); // mov eax, [esp+0x24]
            c.extend([0x8b, 0x00]); // mov eax, [eax]
            c.extend([0x89, 0x41, 0x0c]); // mov [ecx+0xc], eax
            c.push(0x61); // popad
            c.push(0xe9);
            let here = t.cave_next + c.len() as u32 + 4;
            c.extend(log.wrapping_sub(here).to_le_bytes()); // jmp log (ret 4)
            let remove = t.cave_code(&c);
            t.jump(pf::TREE_REMOVE, remove);
            t.stubs(&[
                (pf::USER_FREE_THUNK, false, 1, 0, 0x5656_0001),
                (pf::FREE_LIST_RESET, true, 0, 0, 0x5656_0002),
                (pf::FREE_LIST_DTOR, true, 0, 0, 0x5656_0003),
                (pf::TREE_CLEAR, true, 0, 0, 0x5656_0004),
                (pf::OPERATOR_DELETE, false, 1, 0, 0x5656_0005),
                (pf::FOG_RESET, true, 0, 0, 0x5656_0006),
                (pf::SIGNAL_SEMA, false, 1, 0, 1),
                (pf::SLEEP_THREAD, false, 0, 0, 0),
                (pf::CLUSTER_MANAGER_DTOR, true, 0, 0, 0x5656_0007),
                (pf::USER_ALIGNED_FREE, false, 1, 0, 0x5656_0008),
            ]);
            let what = format!("0x00633fe0 #{case} nodes {n} end {end:#x}");
            t.check(&what, 0x0063_3fe0, Conv::Thiscall(tree), &[], |m| {
                pf::buffer_tree_dtor(m, tree);
                Out::None
            });
            // the list and the vector
            let this = t.zeros(0x20);
            let head = t.zeros(8);
            let k = rng.below(5) as usize;
            let list: Vec<u32> = (0..k).map(|_| t.zeros(0x10)).collect();
            for i in 0..k {
                let next = if i + 1 < k { list[i + 1] } else { head };
                t.w32(list[i], next);
            }
            t.w32(head, list.first().copied().unwrap_or(head));
            t.w32(this, head);
            t.w32(this + 4, k as u32);
            if !rng.chance(3) {
                t.w32(this + 8, rng.next());
                t.w32(this + 0xc, rng.next());
                t.w32(this + 0x10, rng.next());
            }
            let what = format!("0x00634220 #{case} nodes {k}");
            t.check(&what, 0x0063_4220, Conv::Thiscall(this), &[], |m| {
                Out::U32(pf::list_vector_dtor(m, this))
            });
            let fog = t.alloc(&(0..0xb0).map(|_| rng.next() as u8).collect::<Vec<_>>());
            let what = format!("0x006e3f10 #{case}");
            t.check(&what, 0x006e_3f10, Conv::Thiscall(fog), &[], |m| {
                Out::U32(pf::fog_reset_all(m, fog))
            });
            let loader = if rng.chance(3) {
                0
            } else {
                let o = t.object(&[(1, 0x5656_0009)]);
                let vt = t.r32(o);
                let mut w: Vec<u32> = (0..8).map(|_| rng.next() | 1).collect();
                w[0] = vt;
                t.words(&w)
            };
            t.g32(pf::ASYNC_LOAD_MANAGER, loader);
            let what = format!("0x00691b00 #{case} {loader:#x}");
            t.check(&what, 0x0069_1b00, Conv::Cdecl, &[], |m| {
                Out::U32(pf::async_load_shutdown(m))
            });
            let clusters = if rng.chance(3) { 0 } else { t.zeros(0x20) };
            t.g32(pf::CLUSTER_MANAGER_CELL, clusters);
            let what = format!("0x0069a0e0 #{case} {clusters:#x}");
            t.check(&what, 0x0069_a0e0, Conv::Cdecl, &[], |m| {
                pf::cluster_manager_shutdown(m);
                Out::None
            });
            let p = rng.next();
            let what = format!("code_00686fe0 #{case}");
            t.check(&what, 0x0068_6fe0, Conv::Cdecl, &[p], |m| {
                Out::U32(pf::aligned_free_thunk(m, p))
            });
        }
    });
}

/// `SaveDataTtyWrite2(fd, fmt, ...)` with `memset`, `vsprintf`,
/// `__iob_func` and `fprintf` stubbed: the logged `va_list` is the
/// original's argument slots (`ebp + 0x10`) in both oracles; the line's
/// frame is compared but for the cookie.
#[test]
fn platform_tty_write_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0648_000f);
        t.skip = 4;
        for case in 0..16 {
            t.reset();
            t.stubs(&[(pf::MEMSET, false, 3, 0, 0x5757_0001)]);
            t.stub_import(pf::VSPRINTF_IAT, false, 3, 0, 0x5757_0002);
            t.stub_import(pf::IOB_FUNC_IAT, false, 0, 0, 0x0123_4000);
            t.stub_import(pf::FPRINTF_IAT, false, 3, 0, 0x5757_0003);
            let args = [rng.next(), rng.next(), rng.next(), rng.next()];
            let what = format!("SaveDataTtyWrite2 #{case}");
            t.check_framed(
                &what,
                0x0064_8850,
                Conv::Cdecl,
                &args,
                pf::TTY_FRAME,
                |m, ebp| Out::U32(pf::tty_write2(m, ebp, args[1], ebp + 0x10)),
            );
        }
        t.skip = 0;
    });
}

/// A synthetic MSVC `_Tree` (head plus `keys.len()` nodes, a valid
/// red-black-free binary search tree built by insertion; each node's
/// value block `[id, count]`): left `+0`, parent `+4`, right `+8`, nil
/// byte `+0xd`, value pointer `+0x14`. Returns the head.
fn msvc_tree(t: &mut Twins, values: &[(u32, u32)], order: &[u32]) -> u32 {
    let head = t.zeros(0x18);
    t.write(head + 0xd, &[1]);
    let nodes: Vec<u32> = values
        .iter()
        .map(|&(id, count)| {
            let v = t.words(&[id, count]);
            let n = t.zeros(0x18);
            t.w32(n, head);
            t.w32(n + 4, head);
            t.w32(n + 8, head);
            t.w32(n + 0x14, v);
            n
        })
        .collect();
    let mut root = head;
    for (i, &n) in nodes.iter().enumerate() {
        let key = order[i];
        if root == head {
            root = n;
            continue;
        }
        let mut at = root;
        loop {
            let at_key = order[nodes.iter().position(|&x| x == at).unwrap()];
            let side = if key < at_key { 0 } else { 8 };
            let next = t.r32(at + side);
            if next == head {
                t.w32(at + side, n);
                t.w32(n + 4, at);
                break;
            }
            at = next;
        }
    }
    t.w32(head + 4, root);
    // the head's left is the leftmost node, its right the rightmost
    let mut lo = root;
    let mut hi = root;
    if root != head {
        while t.r32(lo) != head {
            lo = t.r32(lo);
        }
        while t.r32(hi + 8) != head {
            hi = t.r32(hi + 8);
        }
    }
    t.w32(head, if root == head { head } else { lo });
    t.w32(head + 8, if root == head { head } else { hi });
    head
}

/// `0x0069b890(this; id)` with the mutex imports stubbed (stdcall;
/// `ReleaseMutex` answering with upper bytes set): trees of 0..12 nodes
/// in random shapes, the id present with a count of 1 or more, present
/// with 0 or a negative one, absent.
#[test]
fn platform_map_has_active_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x069b_0010);
        for case in 0..120 {
            t.reset();
            t.stub_import(pf::WAIT_FOR_SINGLE_OBJECT_IAT, false, 2, 2, 0);
            t.stub_import(pf::RELEASE_MUTEX_IAT, false, 1, 1, 0x1234_5601);
            let n = rng.below(13) as usize;
            let values: Vec<(u32, u32)> = (0..n)
                .map(|_| {
                    (
                        rng.below(8),
                        [0u32, 1, 2, 0xffff_ffff][rng.below(4) as usize],
                    )
                })
                .collect();
            let order: Vec<u32> = (0..n).map(|_| rng.below(100)).collect();
            let head = msvc_tree(t, &values, &order);
            let handle = t.words(&[rng.next()]);
            let this = t.zeros(0x20);
            t.w32(this + 0x10, head);
            t.w32(this + 0x18, handle);
            let id = rng.below(9);
            let what = format!("0x0069b890 #{case} nodes {n} id {id}");
            t.check(&what, 0x0069_b890, Conv::Thiscall(this), &[id], |m| {
                Out::U32(pf::map_has_active(m, this, id))
            });
        }
    });
}

/// `0x00703070`, `0x006db040`, `fiosFinalize`, `0x0067a900`,
/// `0x0067ac40` and `0x006f5450` with their callees stubbed; the globals
/// set and not.
#[test]
fn platform_shutdowns_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0703_0011);
        for case in 0..24 {
            t.reset();
            t.stubs(&[
                (pf::SAMPLER_STATE_DTOR, true, 0, 0, 0x5858_0001),
                (pf::LOCALIZATION_DTOR, true, 0, 0, 0x5858_0002),
                (pf::OPERATOR_DELETE, false, 1, 0, 0x5858_0003),
                (pf::FIOS_RESOURCES_TERM, false, 0, 0, 0x5858_0004),
                (pf::SYNC_MANAGER_DTOR, true, 0, 0, 0x5858_0005),
                (pf::SAMPLER_RELEASE, true, 0, 0, 0x5858_0006),
            ]);
            let this = t.alloc(&(0..0x80).map(|_| rng.next() as u8).collect::<Vec<_>>());
            let what = format!("0x00703070 #{case}");
            t.check(&what, 0x0070_3070, Conv::Thiscall(this), &[], |m| {
                Out::U32(pf::clear_sampler_holder(m, this))
            });
            for cell in [pf::LOCALIZATION_MANAGER, pf::SYNC_MANAGER, pf::BLOCK_CCC7CC] {
                let v = if rng.chance(2) { 0 } else { t.zeros(0x10) };
                t.g32(cell, v);
            }
            let what = format!("0x006db040 #{case}");
            t.check(&what, 0x006d_b040, Conv::Cdecl, &[], |m| {
                pf::localization_shutdown(m);
                Out::None
            });
            let v = if rng.chance(2) { 0 } else { t.zeros(0x10) };
            t.g32(pf::LOCALIZATION_MANAGER, v);
            let what = format!("fiosFinalize #{case}");
            t.check(&what, 0x0067_9590, Conv::Cdecl, &[], |m| {
                Out::U32(pf::fios_finalize(m))
            });
            let what = format!("0x0067a900 #{case}");
            t.check(&what, 0x0067_a900, Conv::Cdecl, &[], |m| {
                Out::U32(pf::sync_manager_shutdown(m))
            });
            let what = format!("0x0067ac40 #{case}");
            t.check(&what, 0x0067_ac40, Conv::Cdecl, &[], |m| {
                Out::U32(pf::free_block_ccc7cc(m))
            });
            let what = format!("0x006f5450 #{case}");
            t.check(&what, 0x006f_5450, Conv::Cdecl, &[], |m| {
                Out::U32(pf::release_samplers(m))
            });
        }
    });
}

/// The job scheduler's push (`0x006dbd10`) and shutdown (`0x006dbe80`)
/// with the semaphore imports and `Thread_JoinClose` stubbed: queues 0..2
/// with ring indices 0..7 and beyond (negative too: the signed `% 8`),
/// entries pointing at scratch blocks. The push's queue slot is the
/// original's argument slot in both oracles.
#[test]
fn platform_job_queue_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x06db_0012);
        for case in 0..40 {
            t.reset();
            t.stub_import(pf::WAIT_FOR_SINGLE_OBJECT_IAT, false, 2, 2, 0);
            t.stub_import(pf::RELEASE_SEMAPHORE_IAT, false, 3, 3, 0x5959_0001);
            t.stubs(&[(pf::THREAD_JOIN_CLOSE, true, 0, 0, 0x5959_0002)]);
            let this = t.alloc(
                &(0..0x1e8 * 3 + 0x40)
                    .map(|_| rng.next() as u8)
                    .collect::<Vec<_>>(),
            );
            for q in 0..3u32 {
                let base = this + q * 0x1e8;
                // entries for ring indices -7..=7 (a negative index from
                // the signed % 8 reads below the ring)
                for k in 0..15 {
                    let e = t.alloc(&(0..0x20).map(|_| rng.next() as u8).collect::<Vec<_>>());
                    t.w32(base + 0x1d0 - 0x1c + 4 * k, e);
                }
                for off in [0x1f0u32, 0x224] {
                    let i = [0u32, 1, 7, 8, 0xffff_fff9, 0x8000_0000][rng.below(6) as usize];
                    t.w32(base + off, i);
                }
            }
            let job = rng.next();
            let queue = rng.below(3);
            let what = format!("0x006dbd10 #{case} queue {queue}");
            t.check_framed(
                &what,
                0x006d_bd10,
                Conv::Thiscall(this),
                &[job, queue],
                0,
                |m, ebp| Out::U32(pf::push_job(m, this, job, queue, Some(ebp + 0xc))),
            );
            // the shutdown pushes onto queue 0: its free ring indices in range
            t.w32(this + 0x1f0, rng.below(8));
            t.w32(this + 0x224, rng.below(8));
            let what = format!("0x006dbe80 #{case}");
            t.check(&what, 0x006d_be80, Conv::Thiscall(this), &[], |m| {
                Out::U32(pf::scheduler_shutdown(m, this))
            });
        }
    });
}

/// `0x0061b4a0(this; timeout)` with `WaitForSingleObject` and
/// `InterlockedCompareExchange` stubbed (a stub answering a random
/// sequence: the first slot free, a later one, none): 0..5 slots, the wait
/// timing out or not.
#[test]
fn platform_claim_slot_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x061b_0013);
        for case in 0..60 {
            t.reset();
            let waited = if rng.chance(4) { 0x102 } else { 0 };
            t.stub_import(pf::WAIT_FOR_SINGLE_OBJECT_IAT, false, 2, 2, waited);
            let seq: Vec<u32> = (0..8).map(|_| u32::from(!rng.chance(3))).collect();
            t.stub_import_seq(pf::COMPARE_EXCHANGE_IAT, 3, 3, &seq);
            let n = rng.below(6);
            let slots = t.zeros(0x60 * 6);
            let this = t.words(&[rng.next(), n, slots]);
            let timeout = rng.word();
            let what = format!("0x0061b4a0 #{case} slots {n} waited {waited:#x}");
            t.check(&what, 0x0061_b4a0, Conv::Thiscall(this), &[timeout], |m| {
                Out::U32(pf::claim_slot(m, this, timeout))
            });
        }
    });
}

/// The big-file teardown: `0x0061d5c0` (the tree's remove stub moves the
/// front node like the one in `platform_teardown`), `BigFileHandle_deinitialize`,
/// `BigFileHandle_dtor_BigFileHandle`, `0x0061b620` and `0x0061b690`
/// with the CRT's `delete[]` and vector destructor, the tree and free
/// list methods, `operator delete` and the Win32 imports stubbed: blocks
/// set or not, 0..5 handles, the stream global set or not.
#[test]
#[allow(clippy::too_many_lines)] // one case setup, five functions
fn platform_big_file_teardown_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x061d_0014);
        for case in 0..40 {
            t.reset();
            let log = t.stub_code(pf::TREE_REMOVE, true, 1, 1, 0);
            let mut c = vec![0x60]; // pushad
            c.extend([0x8b, 0x44, 0x24, 0x24]); // mov eax, [esp+0x24]
            c.extend([0x8b, 0x00]); // mov eax, [eax]
            c.extend([0x89, 0x41, 0x0c]); // mov [ecx+0xc], eax
            c.push(0x61); // popad
            c.push(0xe9);
            let here = t.cave_next + c.len() as u32 + 4;
            c.extend(log.wrapping_sub(here).to_le_bytes()); // jmp log (ret 4)
            let remove = t.cave_code(&c);
            t.jump(pf::TREE_REMOVE, remove);
            t.stubs(&[
                (pf::FREE_LIST_RESET, true, 0, 0, 0x6060_0001),
                (pf::FREE_LIST_DTOR, true, 0, 0, 0x6060_0002),
                (pf::TREE_CLEAR, true, 0, 0, 0x6060_0003),
                (pf::OPERATOR_DELETE, false, 1, 0, 0x6060_0004),
                (pf::OPERATOR_DELETE_ARRAY, false, 1, 0, 0x6060_0005),
                (pf::EH_VECTOR_DTOR, false, 4, 4, 0x6060_0006),
            ]);
            for (slot, n) in [
                (pf::DELETE_CRITICAL_SECTION_IAT, 1),
                (pf::CLOSE_HANDLE_IAT, 1),
                (pf::CRYPT_RELEASE_CONTEXT_IAT, 2),
            ] {
                t.stub_import(slot, false, n, n, 0x6060_0010 + n);
            }
            // a tree of 0..3 nodes chained through +0, ending at itself or null
            let tree = |t: &mut Twins, rng: &mut Rng, at: u32| {
                let n = rng.below(4);
                let end = if rng.chance(2) { at } else { 0 };
                let mut next = end;
                for _ in 0..n {
                    let node = t.words(&[next, 0, 0, 0]);
                    next = node;
                }
                t.w32(at + 0xc, next);
            };
            let handle = |t: &mut Twins, rng: &mut Rng| {
                let words: Vec<u32> = (0..0x18).map(|_| rng.next()).collect();
                let h = t.words(&words);
                for off in [0x40u32, 0, 0x3c, 0x5c] {
                    let v = if rng.chance(3) {
                        0
                    } else {
                        t.words(&[3, 0, 0, 0]) + 4
                    };
                    t.w32(h + off, v);
                }
                tree(t, rng, h + 4);
                h
            };
            let one = t.zeros(0x30);
            tree(t, &mut rng, one);
            let what = format!("0x0061d5c0 #{case}");
            t.check(&what, 0x0061_d5c0, Conv::Thiscall(one), &[], |m| {
                pf::tree_dtor(m, one);
                Out::None
            });
            let h1 = handle(t, &mut rng);
            let what = format!("BigFileHandle_deinitialize #{case}");
            t.check(&what, 0x0061_d910, Conv::Thiscall(h1), &[], |m| {
                pf::big_file_deinitialize(m, h1);
                Out::None
            });
            let h2 = handle(t, &mut rng);
            let what = format!("BigFileHandle_dtor_BigFileHandle #{case}");
            t.check(&what, 0x0061_d670, Conv::Thiscall(h2), &[], |m| {
                pf::big_file_handle_dtor(m, h2);
                Out::None
            });
            let stream = |t: &mut Twins, rng: &mut Rng| {
                let words: Vec<u32> = (0..0x12).map(|_| rng.next()).collect();
                let s = t.words(&words);
                let v = if rng.chance(3) { 0 } else { rng.next() };
                t.w32(s + 0x40, v);
                let v = if rng.chance(3) {
                    0
                } else {
                    t.words(&[2, 0, 0, 0]) + 4
                };
                t.w32(s + 0xc, v);
                for k in 0..5 {
                    let v = if rng.chance(2) { 0 } else { handle(t, rng) };
                    t.w32(s + 0x10 + 4 * k, v);
                }
                s
            };
            let s1 = stream(t, &mut rng);
            let what = format!("0x0061b620 #{case}");
            t.check(&what, 0x0061_b620, Conv::Thiscall(s1), &[], |m| {
                Out::U32(pf::big_file_stream_term(m, s1))
            });
            let s2 = if rng.chance(4) {
                0
            } else {
                stream(t, &mut rng)
            };
            t.g32(pf::BIG_FILE_STREAM, s2);
            let what = format!("0x0061b690 #{case} {s2:#x}");
            t.check(&what, 0x0061_b690, Conv::Cdecl, &[], |m| {
                pf::big_file_stream_shutdown(m);
                Out::None
            });
        }
    });
}

/// `SyncManager_releaseData` and its thunk `code_006fa8c0` with
/// `user_free_thunk` stubbed: every pointer set or not (the table's 0x80
/// words too).
#[test]
fn platform_sync_release_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x06fa_0015);
        for case in 0..32 {
            t.reset();
            t.stubs(&[(pf::USER_FREE_THUNK, false, 1, 0, 0x6161_0001)]);
            let table: Vec<u32> = (0..0x80)
                .map(|_| if rng.chance(2) { 0 } else { rng.next() })
                .collect();
            // never null: the original indexes it unchecked (a null table
            // faults there, in the port alike)
            let table = t.words(&table);
            let mut w: Vec<u32> = (0..0x12)
                .map(|_| if rng.chance(3) { 0 } else { rng.next() })
                .collect();
            w[1] = table;
            let this = t.words(&w);
            let sym = if case % 2 == 0 {
                0x006f_ad90
            } else {
                0x006f_a8c0
            };
            let what = format!("SyncManager_releaseData {sym:#x} #{case}");
            t.check(&what, sym, Conv::Thiscall(this), &[], |m| {
                Out::U32(pf::sync_release_data(m, this))
            });
        }
    });
}

/// The input glue: `PInputMapper_setKey` (keyboards 0..8, null boards,
/// keys around 0x1c and 0x11c), `inputGetCurTime` (a stubbed `GetTime`
/// returning doubles; the start time random), `ClearHoldTimers` and
/// `UpdateHoldTimers` (a stubbed `inputGetCurTime`; pad words and timers
/// random, no NaNs) and `inputODUpdate` (its callees stubbed, the flag on
/// and off).
#[test]
fn platform_input_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0630_0016);
        let float = |rng: &mut Rng| -> f32 {
            match rng.below(4) {
                0 => 0.0,
                1 => f32::from(rng.below(2000) as u16) / 16.0,
                2 => -(f32::from(rng.below(100) as u16) / 3.0),
                _ => f32::from_bits(rng.next() & 0x3fff_ffff),
            }
        };
        for case in 0..60 {
            t.reset();
            // setKey
            let boards: Vec<u32> = (0..7)
                .map(|_| if rng.chance(4) { 0 } else { t.zeros(0x120) })
                .collect();
            let mut mw = vec![0u32; 0x40];
            mw[0x20..0x27].copy_from_slice(&boards);
            // a board past the seven: kbd 7 must not reach it
            mw[0x27] = t.zeros(0x120);
            let mapper = t.words(&mw);
            let this = t.zeros(0x2c0);
            t.w32(this + 0x2b8, mapper);
            let key = [0x1b, 0x1c, 0x20, 0x11b, 0x11c, 0, rng.next()][rng.below(7) as usize];
            let kbd = rng.below(9);
            let v = rng.next();
            let what = format!("PInputMapper_setKey #{case} key {key:#x} kbd {kbd}");
            t.check(
                &what,
                0x0062_c990,
                Conv::Thiscall(this),
                &[key, v, kbd],
                |m| Out::U32(pf::set_key(m, this, key, v, kbd)),
            );
            // inputGetCurTime over a stubbed GetTime
            let now =
                f64::from(rng.below(1 << 20)) + f64::from(rng.below(1000)) / 1024.0 + 1_048_576.0;
            let cell = t.words(&[now.to_bits() as u32, (now.to_bits() >> 32) as u32]);
            let mut c = vec![0xdd, 0x05];
            c.extend(cell.to_le_bytes()); // fld qword [cell]
            c.push(0xc3);
            let get = t.cave_code(&c);
            t.jump(pf::GET_TIME, get);
            let start = 1_048_576.0 + f64::from(rng.below(1 << 16));
            t.patch(t.rebase(pf::APP_START_TIME), &start.to_bits().to_le_bytes());
            let what = format!("inputGetCurTime #{case}");
            let addr = t.rebase(pf::CUR_TIME);
            let ra = t.a.call(addr, Conv::Cdecl, &[]).unwrap();
            let poison = t.rebase(POISON_AT);
            let port = pf::cur_time(&mut G(&mut t.b, poison));
            assert_eq!(
                ra.st0_f32().map(f32::to_bits),
                Some(port.to_bits()),
                "{what}"
            );
            t.diff(&what);
            // the hold timers over a stubbed inputGetCurTime
            t.stub_float(pf::CUR_TIME, float(&mut rng));
            let mut w: Vec<u32> = (0..0x24).map(|_| float(&mut rng).to_bits()).collect();
            w[0] = rng.next();
            w[1] = rng.next();
            let input = t.words(&w);
            let what = format!("InputManager_UpdateHoldTimers #{case}");
            t.check(&what, 0x0063_30e0, Conv::Thiscall(input), &[], |m| {
                Out::U32(pf::update_hold_timers(m, input))
            });
            let what = format!("InputManager_ClearHoldTimers #{case}");
            t.check(&what, 0x0063_1270, Conv::Thiscall(input), &[], |m| {
                pf::clear_hold_timers(m, input);
                Out::None
            });
            // inputODUpdate
            t.stubs(&[
                (pf::INPUT_MANAGER, false, 0, 0, 0x6262_0001),
                (pf::UPDATE_INPUT, true, 0, 0, 0x6262_0002),
                (pf::DISPATCH_MOUSE, true, 0, 0, 0x6262_0003),
            ]);
            t.g32(pf::INPUT_READY, u32::from(!rng.chance(3)));
            t.g32(pf::IGGY_MENU_MANAGER, rng.next());
            let ready = t.r32(t.rebase(pf::INPUT_READY));
            let what = format!("inputODUpdate #{case} ready {ready}");
            t.check(&what, 0x0063_0d50, Conv::Cdecl, &[], |m| {
                let r = pf::od_update(m);
                // the original leaves the caller's eax when not ready
                if ready == 0 { Out::None } else { Out::U32(r) }
            });
        }
    });
}

/// The pair array's reserve (`0x00631450`) and assign (`0x00631110`) with
/// `user_malloc` (a fresh block or 0), `memcpy` and `user_free_thunk`
/// stubbed; counts 0..5 and negative, capacities 0..6 and negative, no
/// data yet or some, self-assignment. Then `SaveData_Init`,
/// `0x0067ac10` and `0x006f1c10` with their callees stubbed, `operator
/// new` failing or not.
#[test]
fn platform_arrays_and_init_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0631_0017);
        for case in 0..60 {
            t.reset();
            let block = if rng.chance(5) { 0 } else { t.zeros(0x80) };
            t.stubs(&[
                (pf::USER_MALLOC, false, 1, 0, block),
                (pf::MEMCPY, false, 3, 0, 0x6363_0001),
                (pf::USER_FREE_THUNK, false, 1, 0, 0x6363_0002),
            ]);
            let pairs = |t: &mut Twins, rng: &mut Rng| {
                let count = [0u32, 1, 3, 5, 0xffff_ffff][rng.below(5) as usize];
                let data = if rng.chance(3) {
                    0
                } else {
                    t.alloc(&(0..0x30).map(|_| rng.next() as u8).collect::<Vec<_>>())
                };
                t.words(&[count, rng.below(8), data])
            };
            let a = pairs(t, &mut rng);
            let n = [0u32, 1, 4, 6, 0xffff_fffe][rng.below(5) as usize];
            let what = format!("0x00631450 #{case} n {n:#x} block {block:#x}");
            t.check(&what, 0x0063_1450, Conv::Thiscall(a), &[n], |m| {
                Out::U32(pf::pair_array_reserve(m, a, n))
            });
            let b = pairs(t, &mut rng);
            let c = if rng.chance(6) { b } else { pairs(t, &mut rng) };
            // the source's data must exist for its count
            if t.r32(c + 8) == 0 {
                let d = t.zeros(0x30);
                t.w32(c + 8, d);
            }
            // a failing user_malloc makes the original write pairs at
            // 8 i from 0 (it faults; the port alike): a block here
            let fresh = t.zeros(0x80);
            t.stubs(&[(pf::USER_MALLOC, false, 1, 0, fresh)]);
            let what = format!("0x00631110 #{case}");
            t.check(&what, 0x0063_1110, Conv::Thiscall(b), &[c], |m| {
                Out::U32(pf::pair_array_assign(m, b, c))
            });
            let object = if rng.chance(4) {
                0
            } else {
                t.words(&[rng.next() | 0x0101_0101, rng.next()])
            };
            t.stubs(&[
                (pf::SAVE_MANAGER, false, 0, 0, 0x6363_0003),
                (pf::SAVE_INIT_SYS, true, 2, 2, 0x6363_0004),
                (pf::TROPHY_MANAGER, false, 0, 0, 0x6363_0005),
                (pf::TROPHY_CLEAR, true, 0, 0, 0x6363_0006),
                (pf::OPERATOR_NEW, false, 1, 0, object),
                (pf::SAVE_MANAGER_CTOR, true, 0, 0, 0x6363_0007),
            ]);
            let what = format!("SaveData_Init #{case}");
            t.check(&what, 0x0064_7fb0, Conv::Cdecl, &[], |m| {
                Out::U32(pf::save_data_init(m))
            });
            let what = format!("0x0067ac10 #{case} {object:#x}");
            t.check(&what, 0x0067_ac10, Conv::Cdecl, &[], |m| {
                Out::U32(pf::trophy_manager_create(m))
            });
            let what = format!("0x006f1c10 #{case} {object:#x}");
            t.check(&what, 0x006f_1c10, Conv::Cdecl, &[], |m| {
                Out::U32(pf::save_manager_create(m))
            });
        }
    });
}

/// `StatsAndAchievements_Create`, the save manager's constructor
/// `0x006f1980`, `0x0061d8e0`, `VbfStream_Open` and `iSyncInitialize`
/// with their callees stubbed (`operator new` failing or not, the sync
/// manager already there or not, the tree remove stub moving the front
/// node, `size` 0 or not).
#[test]
fn platform_creation_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x06f1_0018);
        for case in 0..40 {
            t.reset();
            let object = if rng.chance(4) { 0 } else { t.zeros(0x40) };
            t.stubs(&[
                (pf::OPERATOR_NEW, false, 1, 0, object),
                (pf::STATS_CTOR, true, 0, 0, 0x6464_0001),
                (pf::TO_BLOCK, false, 0, 0, 0x6464_0002),
                (pf::TO_CLEAR_BLOCK, false, 0, 0, 0x6464_0003),
                (pf::FREE_LIST_RESET, true, 0, 0, 0x6464_0004),
                (pf::SYNC_MANAGER_CTOR, true, 0, 0, 0x6464_0005),
            ]);
            t.stub_import(pf::STRNCPY_IAT, false, 3, 0, 0x6464_0006);
            t.stub_import(pf::FOPEN_IAT, false, 2, 0, 0x6464_0007);
            let what = format!("StatsAndAchievements_Create #{case} {object:#x}");
            t.check(&what, 0x006f_1430, Conv::Cdecl, &[], |m| {
                Out::U32(pf::stats_create(m))
            });
            let this = t.alloc(&(0..0x400).map(|_| rng.next() as u8).collect::<Vec<_>>());
            let what = format!("0x006f1980 #{case}");
            t.check(&what, 0x006f_1980, Conv::Thiscall(this), &[], |m| {
                Out::U32(pf::save_manager_ctor(m, this))
            });
            // the tree: remove(this; node) sets this+0xc = [node]
            let log = t.stub_code(pf::TREE_REMOVE, true, 1, 1, 0);
            let mut c = vec![
                0x60, 0x8b, 0x44, 0x24, 0x24, 0x8b, 0x00, 0x89, 0x41, 0x0c, 0x61, 0xe9,
            ];
            let here = t.cave_next + c.len() as u32 + 4;
            c.extend(log.wrapping_sub(here).to_le_bytes());
            let remove = t.cave_code(&c);
            t.jump(pf::TREE_REMOVE, remove);
            let tree = t.zeros(0x30);
            let end = if rng.chance(2) { tree } else { 0 };
            let mut next = end;
            for _ in 0..rng.below(4) {
                next = t.words(&[next, 0, 0, 0]);
            }
            t.w32(tree + 0xc, next);
            let what = format!("0x0061d8e0 #{case}");
            t.check(&what, 0x0061_d8e0, Conv::Thiscall(tree), &[], |m| {
                Out::U32(pf::tree_reset(m, tree))
            });
            let stream = t.alloc(&(0..0x10).map(|_| rng.next() as u8).collect::<Vec<_>>());
            let (path, size) = (rng.next(), if rng.chance(2) { 0 } else { rng.next() });
            let what = format!("VbfStream_Open #{case}");
            t.check(
                &what,
                0x0061_acc0,
                Conv::Thiscall(stream),
                &[path, size],
                |m| Out::U32(pf::vbf_stream_open(m, stream, path, size)),
            );
            let old = if rng.chance(3) { rng.next() } else { 0 };
            t.g32(pf::SYNC_MANAGER, old);
            let what = format!("iSyncInitialize #{case} old {old:#x}");
            t.check(&what, 0x0067_a960, Conv::Cdecl, &[], |m| {
                Out::U32(pf::sync_initialize(m))
            });
        }
    });
}
/// `0x006dafc0`, `Vbf_Contains` (a stubbed lookup: 0, a node, the node
/// whose entry wraps to 0; the digest copy in the frame), `0x0059e530`
/// (the guard set or not; `atexit` stubbed) and `BigFileStream_ctor`
/// (its callees stubbed).
#[test]
fn platform_creation2_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x06da_0019);
        for case in 0..40 {
            t.reset();
            let object = if rng.chance(4) { 0 } else { t.zeros(0x20) };
            let node = [0u32, 0x0012_3400, 0xffff_ffe0][rng.below(3) as usize];
            t.stubs(&[
                (pf::OPERATOR_NEW, false, 1, 0, object),
                (pf::LOCALIZATION_CTOR, true, 0, 0, 0x6565_0001),
                (pf::VBF_MAP_LOOKUP, true, 1, 1, node),
                (pf::ATEXIT, false, 1, 0, 0x6565_0002),
                (pf::STREAM_SLOTS_INIT, true, 1, 1, 0x6565_0003),
            ]);
            t.stub_import(pf::CRYPT_ACQUIRE_IAT, false, 5, 5, 1);
            t.stub_import(pf::INIT_CRITICAL_SECTION_IAT, false, 1, 1, 0x6565_0004);
            let old = if rng.chance(3) { rng.next() } else { 0 };
            t.g32(pf::LOCALIZATION_MANAGER, old);
            let what = format!("0x006dafc0 #{case} old {old:#x} new {object:#x}");
            t.check(&what, 0x006d_afc0, Conv::Cdecl, &[], |m| {
                let r = pf::localization_create(m);
                // junk (the cookie's xor) when the manager exists
                if old == 0 { Out::U32(r) } else { Out::None }
            });
            let digest = t.words(&[rng.next(), rng.next(), rng.next(), rng.next()]);
            let this = t.zeros(0x20);
            let what = format!("Vbf_Contains #{case} node {node:#x}");
            t.check_framed(
                &what,
                0x0061_dec0,
                Conv::Thiscall(this),
                &[digest],
                pf::CONTAINS_FRAME,
                |m, ebp| Out::U32(pf::vbf_contains(m, ebp, this, digest)),
            );
            t.g32(pf::LAYOUT_GUARD, rng.next());
            let what = format!("0x0059e530 #{case}");
            t.check(&what, 0x0059_e530, Conv::Cdecl, &[], |m| {
                Out::U32(pf::layout_interface(m))
            });
            let s = t.alloc(&(0..0x60).map(|_| rng.next() as u8).collect::<Vec<_>>());
            let what = format!("BigFileStream_ctor #{case}");
            t.check(&what, 0x0061_bdd0, Conv::Thiscall(s), &[], |m| {
                Out::U32(pf::big_file_stream_ctor(m, s))
            });
        }
    });
}
/// `0x0061c2c0(this; n)` and `BigFileStream_createInstance` with their
/// callees stubbed: an old slot array or none, an old semaphore or none,
/// `operator new[]` failing or not, `n` 0, small and overflowing (the
/// saturated size), the stream there already or not, `operator new`
/// failing or not.
#[test]
fn platform_stream_slots_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x061c_001a);
        for case in 0..40 {
            t.reset();
            let block = if rng.chance(4) { 0 } else { t.zeros(0x40) };
            let object = if rng.chance(4) {
                0
            } else {
                t.alloc(&(0..0x60).map(|_| rng.next() as u8 | 1).collect::<Vec<_>>())
            };
            t.stubs(&[
                (pf::OPERATOR_DELETE_ARRAY, false, 1, 0, 0x6666_0001),
                (pf::EH_VECTOR_DTOR, false, 4, 4, 0x6666_0002),
                (pf::OPERATOR_NEW_ARRAY, false, 1, 0, block),
                (pf::EH_VECTOR_CTOR, false, 5, 5, 0x6666_0003),
                (pf::OPERATOR_NEW, false, 1, 0, object),
            ]);
            t.stub_import(pf::CLOSE_HANDLE_IAT, false, 1, 1, 1);
            t.stub_import(pf::CREATE_SEMAPHORE_IAT, false, 4, 4, 0x6666_0005);
            t.stub_import(pf::CRYPT_ACQUIRE_IAT, false, 5, 5, 1);
            t.stub_import(pf::INIT_CRITICAL_SECTION_IAT, false, 1, 1, 0x6666_0006);
            let old = if rng.chance(3) {
                0
            } else {
                t.words(&[2, 0, 0]) + 4
            };
            let sem = if rng.chance(3) { 0 } else { rng.next() };
            let this = t.words(&[sem, rng.next(), old]);
            let n = [0u32, 1, 0x1e, 0x02aa_aaab, 0xffff_ffff][rng.below(5) as usize];
            let what = format!("0x0061c2c0 #{case} n {n:#x} block {block:#x}");
            t.check(&what, 0x0061_c2c0, Conv::Thiscall(this), &[n], |m| {
                Out::U32(pf::stream_slots_init(m, this, n))
            });
            t.stubs(&[(pf::STREAM_SLOTS_INIT, true, 1, 1, 0x6666_0004)]);
            let existing = if rng.chance(3) { rng.next() } else { 0 };
            t.g32(pf::BIG_FILE_STREAM, existing);
            let what = format!(
                "BigFileStream_createInstance #{case} existing {existing:#x} new {object:#x}"
            );
            t.check(&what, 0x0061_b590, Conv::Cdecl, &[], |m| {
                let r = pf::big_file_stream_create(m);
                if existing == 0 {
                    Out::U32(r)
                } else {
                    Out::None
                }
            });
        }
    });
}
/// The VBF digest tree: `0x0061d210` grows one tree in both oracles, a
/// node at a time (each insert compared), keys drawn from small sets so
/// that both halves compare signed and unsigned words, ties fall to the
/// second half and duplicates come back. Then `Vbf_InsertEntry` with the
/// free list's methods and the insert fix-up stubbed (a free node, an
/// empty list refilled or not) on the same kind of tree.
#[test]
fn platform_digest_tree_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x061d_001b);
        let word =
            |rng: &mut Rng| [0u32, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff][rng.below(5) as usize];
        for round in 0..16 {
            t.reset();
            let this = t.zeros(0x30);
            t.w32(this + 0xc, this);
            for k in 0..30 {
                let mut w: Vec<u32> = (0..9).map(|_| rng.next()).collect();
                for x in &mut w[4..8] {
                    *x = word(&mut rng);
                }
                let node = t.words(&w);
                let what = format!("0x0061d210 round {round} #{k}");
                t.check(&what, 0x0061_d210, Conv::Thiscall(this), &[node], |m| {
                    Out::U32(pf::digest_tree_link(m, this, node))
                });
            }
            // Vbf_InsertEntry on the grown tree
            let free = if rng.chance(3) {
                0
            } else {
                t.alloc(&(0..0x30).map(|_| rng.next() as u8).collect::<Vec<_>>())
            };
            t.w32(this + 0x18, free);
            t.w32(this + 0x20, 0x30);
            let refill = if rng.chance(2) { 0 } else { t.zeros(0x30) };
            // allocateNewBlock(this; size): [this] = refill, then the log
            let log = t.stub_code(pf::FREE_LIST_NEW_BLOCK, true, 1, 1, 0);
            let mut c = vec![0xc7, 0x01];
            c.extend(refill.to_le_bytes()); // mov dword [ecx], refill
            c.push(0xe9);
            let here = t.cave_next + c.len() as u32 + 4;
            c.extend(log.wrapping_sub(here).to_le_bytes());
            let new_block = t.cave_code(&c);
            t.jump(pf::FREE_LIST_NEW_BLOCK, new_block);
            t.stubs(&[
                (pf::FREE_LIST_DEALLOCATE, true, 1, 1, 0x6767_0001),
                (pf::TREE_POST_INSERT, true, 1, 1, 0x6767_0002),
            ]);
            // half the time an existing node's key: the duplicate path
            let root = t.r32(this + 0xc);
            let key: Vec<u32> = if root != this && rng.chance(2) {
                (0..4).map(|k| t.r32(root + 0x10 + 4 * k)).collect()
            } else {
                (0..4).map(|_| word(&mut rng)).collect()
            };
            let digest = t.words(&key);
            let entry = t.words(&[rng.next()]);
            let what = format!("Vbf_InsertEntry round {round} free {free:#x} refill {refill:#x}");
            t.check(
                &what,
                0x0061_de30,
                Conv::Thiscall(this),
                &[digest, entry],
                |m| Out::U32(pf::vbf_insert_entry(m, this, digest, entry)),
            );
        }
    });
}

/// Fourteen small glue functions (sort predicates, fixup record setters,
/// deleting destructors, a string length helper, the enum lookup, the
/// `VFile` close, the stream slot constructor, the BSTR free, a forwarder,
/// a flag setter, the pad enumeration callback) with their callees
/// stubbed: words equal, above and below; flags with and without the bit;
/// lookups found or not; every device type around the three taken.
#[test]
#[allow(clippy::too_many_lines)] // fourteen functions, one case setup
fn platform_small_glue_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x060a_001c);
        for case in 0..60 {
            t.reset();
            t.stubs(&[
                (pf::READ_LEB128, false, 1, 0, rng.next()),
                (pf::READ_RECORD, true, 2, 2, 0x6868_0001),
                (pf::DTOR_5A0270, true, 0, 0, 0x6868_0002),
                (pf::PHYRE_FREE, false, 1, 0, 0x6868_0003),
                (
                    pf::TYPE_ENUM_FIND,
                    false,
                    1,
                    0,
                    if rng.chance(2) { 0 } else { rng.next() },
                ),
                (pf::RELEASE_VFILE, true, 1, 1, 0x6868_0004),
                (pf::SLOT_MEMBER_CTOR, true, 0, 0, 0x6868_0005),
                (pf::CALL_61D030, true, 3, 3, 0x6868_0006),
                (pf::ADD_PAD_DEVICE, false, 1, 0, 0x6868_0007),
            ]);
            t.stub_import(pf::OLEAUT_FREE_IAT, false, 1, 1, 0x6868_0008);
            let small = |rng: &mut Rng| [0u32, 1, 2, 0xffff_ffff][rng.below(4) as usize];
            let a = t.words(&[rng.next(), small(&mut rng)]);
            let b = t.words(&[rng.next(), small(&mut rng)]);
            for sym in [0x0060_add0u32, 0x0060_ae00] {
                let what = format!("{sym:#x} #{case}");
                t.check(&what, sym, Conv::Cdecl, &[a, b], |m| {
                    Out::U32(pf::word4_greater(m, a, b))
                });
            }
            let rec = t.zeros(0x10);
            let (object, cursor, flags) = (rng.next(), rng.next(), rng.next());
            let what = format!("Fixup_SetArrayObjectAndRead #{case} flags {flags:#x}");
            t.check(
                &what,
                0x0060_ae50,
                Conv::Cdecl,
                &[rec, object, cursor, flags],
                |m| Out::U32(pf::set_array_object_and_read(m, rec, object, cursor, flags)),
            );
            let rec = t.zeros(0x10);
            let what = format!("Fixup_SetObjectAndRead #{case}");
            t.check(
                &what,
                0x0060_ae90,
                Conv::Cdecl,
                &[rec, object, cursor, flags],
                |m| Out::U32(pf::set_object_and_read(m, rec, object, cursor, flags)),
            );
            let this = t.zeros(0x60);
            let f = rng.next();
            let what = format!("0x0060e8a0 #{case}");
            t.check(&what, 0x0060_e8a0, Conv::Thiscall(this), &[f], |m| {
                Out::U32(pf::deleting_dtor_60e8a0(m, this, f))
            });
            let what = format!("0x0061a740 #{case}");
            t.check(&what, 0x0061_a740, Conv::Thiscall(this), &[f], |m| {
                Out::U32(pf::deleting_dtor_61a740(m, this, f))
            });
            let mut text: Vec<u8> = (0..rng.below(20))
                .map(|_| 1 + rng.below(255) as u8)
                .collect();
            text.push(0);
            let s = t.alloc(&text);
            let sp = t.words(&[s]);
            let (out, len) = (t.zeros(4), t.zeros(4));
            let what = format!("0x0060e8e0 #{case}");
            t.check(&what, 0x0060_e8e0, Conv::Cdecl, &[sp, out, len], |m| {
                Out::U32(pf::string_with_length(m, sp, out, len))
            });
            let name = rng.next();
            let what = format!("0x0060e9b0 #{case}");
            t.check(&what, 0x0060_e9b0, Conv::Cdecl, &[out, name], |m| {
                Out::U32(pf::find_enum(m, out, name))
            });
            let vfile = t.words(&[rng.next(), rng.next(), rng.next()]);
            let what = format!("VFile_fileClose #{case}");
            t.check(&what, 0x0061_aa50, Conv::Thiscall(vfile), &[], |m| {
                Out::U32(pf::vfile_close(m, vfile))
            });
            let this = t.alloc(&(0..0x60).map(|_| rng.next() as u8 | 1).collect::<Vec<_>>());
            let what = format!("0x0061af20 #{case}");
            t.check(&what, 0x0061_af20, Conv::Thiscall(this), &[], |m| {
                Out::U32(pf::slot_ctor(m, this))
            });
            let what = format!("0x0061b0b0 #{case}");
            t.check(&what, 0x0061_b0b0, Conv::Thiscall(vfile), &[], |m| {
                Out::U32(pf::free_bstr(m, vfile))
            });
            let (x, y, z, w) = (rng.next(), small(&mut rng), rng.next(), rng.next());
            let what = format!("0x0061d0b0 #{case}");
            t.check(&what, 0x0061_d0b0, Conv::Cdecl, &[x, y, z, w], |m| {
                Out::U32(pf::forward_61d030(m, x, y, z, w))
            });
            let what = format!("0x00624d30 #{case}");
            t.check(&what, 0x0062_4d30, Conv::Cdecl, &[this], |m| {
                Out::U32(pf::set_flag39(m, this))
            });
            let kind = [0x13u8, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x94][rng.below(8) as usize];
            t.write(this + 0x24, &[kind]);
            let ctx = rng.next();
            let what = format!("Input_EnumPadsCallback #{case} kind {kind:#x}");
            t.check(&what, 0x0062_4d40, Conv::Stdcall, &[this, ctx], |m| {
                Out::U32(pf::enum_pads_callback(m, this))
            });
        }
    });
}

/// The `VFile` set: `VFile_dtor_VFile` (buffer record and file set or
/// not), `BigFileHandle_releaseVFile` (null, one of the 15 inline files,
/// a heap file), `0x0061af40`, `BigFileHandle_getNewVFile` (lookup 0, a
/// node, a node whose entry wraps to 0) and `0x0061b330` (the resource
/// there or not; the allocation failing or not), their callees stubbed.
#[test]
fn platform_vfiles_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x061a_001d);
        for case in 0..60 {
            t.reset();
            let node = [0u32, 0, 0xffff_ffe0][rng.below(3) as usize];
            let node = if node == 0 && rng.chance(2) {
                t.words(&[0; 9])
            } else {
                node
            };
            let res = if rng.chance(3) {
                0
            } else {
                t.alloc(&[3, 0, 0x41, 0, 0x42, 0, 0x43, 0])
            };
            let alloc = if rng.chance(4) { 0 } else { rng.next() };
            t.stubs(&[
                (pf::OPERATOR_DELETE_ARRAY, false, 1, 0, 0x6969_0001),
                (pf::OPERATOR_DELETE, false, 1, 0, 0x6969_0002),
                (pf::VFILE_RESET, true, 0, 0, 0x6969_0003),
                (pf::EH_VECTOR_DTOR, false, 4, 4, 0x6969_0004),
                (pf::VBF_MAP_LOOKUP, true, 1, 1, node),
                (pf::VFILE_FOR_ENTRY, true, 1, 1, 0x6969_0005),
                (pf::STRING_RESOURCE, false, 1, 0, res),
            ]);
            t.stub_import(pf::FCLOSE_IAT, false, 1, 0, 0x6969_0006);
            t.stub_import(pf::CLOSE_HANDLE_IAT, false, 1, 1, 1);
            t.stub_import(pf::OLEAUT_FREE_IAT, false, 1, 1, 0x6969_0007);
            t.stub_import(pf::OLEAUT_ALLOC_IAT, false, 2, 2, alloc);
            let vfile = |t: &mut Twins, rng: &mut Rng| {
                let rec = if rng.chance(3) {
                    0
                } else {
                    t.words(&[if rng.chance(2) { 0 } else { rng.next() }])
                };
                let f = if rng.chance(3) { 0 } else { rng.next() };
                t.words(&[rng.next(), f, 0, 0, 0, 0, rec, 0])
            };
            let v = vfile(t, &mut rng);
            let what = format!("VFile_dtor_VFile #{case}");
            t.check(&what, 0x0061_a960, Conv::Thiscall(v), &[], |m| {
                Out::U32(pf::vfile_dtor(m, v))
            });
            let inline = t.zeros(0x20 * 15);
            let handle = t.zeros(0x60);
            t.w32(handle + 0x5c, inline);
            let which = match rng.below(3) {
                0 => 0,
                1 => inline + 0x20 * [0, 14, rng.below(15)][rng.below(3) as usize],
                _ => vfile(t, &mut rng),
            };
            let what = format!("BigFileHandle_releaseVFile #{case} {which:#x}");
            t.check(&what, 0x0061_df70, Conv::Thiscall(handle), &[which], |m| {
                let r = pf::release_vfile(m, handle, which);
                if which == 0 { Out::None } else { Out::U32(r) }
            });
            let slots = if rng.chance(3) {
                0
            } else {
                t.words(&[2, 0, 0]) + 4
            };
            let pool = t.words(&[rng.next(), rng.next(), slots]);
            let what = format!("0x0061af40 #{case}");
            t.check(&what, 0x0061_af40, Conv::Thiscall(pool), &[], |m| {
                Out::U32(pf::slot_pool_term(m, pool))
            });
            let digest = t.words(&[rng.next(), rng.next(), rng.next(), rng.next()]);
            let what = format!("BigFileHandle_getNewVFile #{case} node {node:#x}");
            t.check_framed(
                &what,
                0x0061_df10,
                Conv::Thiscall(handle),
                &[digest],
                pf::GET_VFILE_FRAME,
                |m, ebp| Out::U32(pf::get_new_vfile(m, ebp, handle, digest)),
            );
            let bstr = t.words(&[rng.next()]);
            let id = rng.next();
            let what = format!("0x0061b330 #{case} res {res:#x} alloc {alloc:#x}");
            t.check(&what, 0x0061_b330, Conv::Thiscall(bstr), &[id], |m| {
                Out::U32(pf::load_bstr(m, bstr, id))
            });
        }
    });
}

// ---- the save/load screen ----

/// Random machine globals: state, timer, list position, counts, choice,
/// a save buffer of 0x4000 bytes, the file table and the menu.
fn machine(t: &mut Twins, rng: &mut Rng) {
    let small = |rng: &mut Rng| match rng.below(4) {
        0 => 0,
        1 => 1,
        2 => rng.below(6),
        _ => rng.word(),
    };
    for g in [
        sv::STATE,
        sv::TIMER,
        sv::TOP,
        sv::CURSOR,
        sv::COUNT,
        sv::LIST_LEN,
        sv::FULL,
        sv::CHOICE,
    ] {
        let v = small(rng);
        t.g32(g, v);
    }
    let v = rng.below(5);
    t.g32(sv::CURSOR, v);
    let v = 0x100 + rng.below(0x3c00);
    t.g32(sv::SIZE, v);
    let fill: Vec<u32> = (0..0x1000).map(|_| rng.next()).collect();
    let buf = t.words(&fill);
    t.g32(sv::BUFFER, buf);
    for g in [
        sv::TICK_BASE,
        sv::BUSY,
        sv::RENAME,
        sv::MERGED_FLAGS[0],
        sv::MERGED_FLAGS[1],
    ] {
        let v = small(rng);
        t.g32(g, v);
    }
    let at = t.rebase(sv::FILES);
    for i in 0..600u32 {
        let v = if i == 0 && rng.chance(2) {
            0xffff_ffff
        } else {
            rng.below(300)
        };
        t.w32(at + 4 * i, v);
    }
    let at = t.rebase(sv::EXISTS);
    let free = rng.below(250);
    // existing (1), missing (-1) and 0 (which counts as existing: the
    // free-number search wants a negative word); now and then number 0
    for i in 0..200 {
        let v = if i == free || rng.chance(40) || (i == 0 && rng.chance(3)) {
            0xffff_ffff
        } else {
            u32::from(!rng.chance(8))
        };
        t.w32(at + 4 * i, v);
    }
    let menu = if rng.chance(5) { 0 } else { t.zeros(0x40) };
    t.g32(sv::MENU, menu);
    let dialog = t.zeros(0x20);
    t.g32(sv::DIALOG_OBJ, dialog);
}

/// Recording stubs for every callee the machine's handlers make, but
/// `except` (the handler under test).
fn machine_stubs(t: &mut Twins, code: u32, except: u32, rng: &mut Rng) {
    let list = [
        (sv::MENU_INPUT, 0, code),
        (sv::CLICK, 1, rng.word()),
        (sv::YES_NO, 0, rng.word()),
        (sv::ENABLE, 0, rng.word()),
        (sv::DISABLE, 0, rng.word()),
        (sv::CLEAR, 0, rng.word()),
        (sv::SHOW_POPUP, 1, rng.word()),
        (sv::SHOW_POPUP2, 2, rng.word()),
        (sv::SHOW_POPUP4, 1, rng.word()),
        (sv::SET_FOCUS, 1, rng.word()),
        (sv::MOVE_CURSOR, 1, u32::from(rng.chance(3))),
        (sv::CURSOR_DOWN, 0, rng.word()),
        (
            sv::SAVE_CHECK_CRC,
            0,
            if rng.chance(2) { 0 } else { rng.word() | 1 },
        ),
        (sv::MEMSET, 3, rng.word()),
        (sv::MENU_DISABLE, 0, rng.word()),
        (sv::MENU_RELEASE, 0, rng.word()),
        (sv::MENU_OPEN, 0, rng.word()),
        (sv::SAVE_LOAD_SUCCEED, 1, rng.word()),
        (sv::SET_DIALOG_STATE, 1, rng.word()),
        (sv::SET_SAVE_MODE, 1, rng.word()),
        (sv::ENABLE_SAVE, 0, rng.word()),
        (sv::OPERATOR_DELETE_ARRAY, 1, rng.word()),
        (sv::OPERATOR_DELETE, 1, rng.word()),
        (sv::SCAN_FILES, 0, rng.word()),
        (sv::SORT_FILES, 0, rng.word()),
        (sv::READ_HEADERS, 2, rng.word()),
        (sv::SE_PLAY, 1, rng.word()),
        (sv::NEED_RENAME, 1, rng.word()),
        (sv::IGGY_SET_VALUE, 2, rng.word()),
        (sv::IGGY_SET_STRING, 2, rng.word()),
        (sv::GET_LOC, 2, rng.word()),
    ];
    for (sym, n, ret) in list {
        if sym != except {
            t.stub(sym, false, n, 0, ret);
        }
    }
    let ret = rng.word();
    t.stub(sv::IGGY_CALL, true, 5, 5, ret);
    let ret = rng.word();
    t.stub(sv::IGGY_DTOR, true, 0, 0, ret);
}

/// The handlers without arguments: `(original, port)`.
type Handler = (u32, fn(&mut G) -> u32);

fn handlers() -> Vec<Handler> {
    vec![
        (0x006e_fbd0, |m| sa::check_file(m)),
        (0x006e_fc50, |m| sa::yes_no(m, sv::YES_NO_OVERWRITE)),
        (0x006e_ffa0, |m| sa::yes_no(m, sv::YES_NO_LOAD)),
        (0x006f_05b0, |m| sa::yes_no(m, sv::YES_NO_SAVE)),
        (0x006e_fcd0, |m| sa::save_done(m, 1)),
        (0x006e_fde0, |m| sa::save_done(m, 0)),
        (0x006f_0020, |m| sa::load_done(m)),
        (0x006f_0630, |m| sa::save_finished(m)),
        (0x006f_0290, |m| sa::cursor_down(m)),
        (0x006e_fe20, |m| sa::list_input(m, sv::LIST_OVERWRITE)),
        (0x006f_0140, |m| sa::list_input(m, sv::LIST_LOAD)),
        (0x006f_06e0, |m| sa::save_list_input(m)),
        (0x006e_ff00, |m| sa::build_list(m, sv::BUILD_OVERWRITE)),
        (0x006f_0220, |m| sa::build_list(m, sv::BUILD_LOAD)),
        (0x006f_0800, |m| sa::build_list(m, sv::BUILD_SAVE)),
        (0x006e_ff70, |m| sa::wait_then_enable(m)),
        (0x006f_0050, |m| sa::confirm_popup(m)),
        (0x006f_04c0, |m| sa::saved(m)),
        (0x0070_2a30, |m| sa::as3_call(m, sv::AS3_ENABLE)),
        (0x0070_2a50, |m| sa::as3_call(m, sv::AS3_DISABLE)),
        (0x0070_2f80, |m| sa::as3_call(m, sv::AS3_CLEAR)),
        (0x0070_2be0, |m| sa::new_file_row(m)),
    ]
}

#[test]
fn save_machine_handlers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a7e_0001);
        for (sym, port) in handlers() {
            for case in 0..120 {
                t.reset();
                machine(t, &mut rng);
                let code = match case % 10 {
                    0..=7 => case % 10 + 1,
                    8 => 0,
                    _ => rng.word(),
                };
                machine_stubs(t, code, sym, &mut rng);
                let what = format!("handler {sym:#x} #{case} code {code}");
                t.check(&what, sym, Conv::Cdecl, &[], |m| Out::U32(port(m)));
            }
        }
    });
}

#[test]
fn save_machine_cursor_and_rows_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a7e_0002);
        for case in 0..80 {
            t.reset();
            machine(t, &mut rng);
            machine_stubs(t, 0, sv::MOVE_CURSOR, &mut rng);
            let code = rng.below(9);
            let what = format!("move_cursor #{case} code {code}");
            t.check(&what, 0x006e_f630, Conv::Cdecl, &[code], |m| {
                Out::U32(sa::move_cursor(m, code))
            });
            let mut words = [0u32; 11];
            for w in &mut words {
                *w = rng.word();
            }
            words[10] = match rng.below(3) {
                0 => 0,
                1 => 0xffff_ffff,
                _ => rng.word(),
            };
            let what = format!("file_row #{case}");
            let mut strings = [0u32; 9];
            strings.copy_from_slice(&words[1..10]);
            t.check(&what, 0x0070_2d90, Conv::Cdecl, &words, |m| {
                Out::U32(sa::file_row(m, words[0], strings, words[10]))
            });
        }
    });
}

#[test]
fn save_machine_update_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a7e_0003);
        let targets = [
            0x006f_0800u32,
            0x006f_06e0,
            0x006f_05b0,
            0x006f_04c0,
            0x006f_0630,
            0x006f_0220,
            0x006f_0140,
            0x006e_ffa0,
            0x006e_fbd0,
            0x006f_0050,
            0x006f_0020,
            0x006e_ff00,
            0x006e_fe20,
            0x006e_fc50,
            0x006e_ff70,
            0x006e_fcd0,
            0x006e_fde0,
        ];
        for state in 0..32u32 {
            t.reset();
            machine(t, &mut rng);
            t.g32(sv::STATE, state);
            for &f in &targets {
                t.stub(f, false, 0, 0, f ^ 0x5555);
            }
            for f in [0x006f_0860u32, 0x006f_0070, 0x006e_fd10] {
                t.stub(f, false, 1, 0, f ^ 0xaaaa);
            }
            let what = format!("SaveDataMachine_Update state {state}");
            t.check(&what, 0x006f_0950, Conv::Cdecl, &[], |m| {
                Out::U32(sa::machine_update(m))
            });
        }
        // code_006f2740: the 5-byte jump thunk to it (SaveData_Update
        // calls it with ecx = the manager, which the machine never reads)
        for ret in [0u32, 1, 0x1234_5678] {
            t.reset();
            t.stub(0x006f_0950, false, 0, 0, ret);
            let what = format!("code_006f2740 -> {ret:#x}");
            t.check(&what, 0x006f_2740, Conv::Thiscall(0x0bad_f00d), &[], |m| {
                Out::U32(sa::machine_update_thunk(m))
            });
        }
    });
}

#[test]
fn save_files_match_the_original() {
    Twins::each(|t| {
        t.skip = 4;
        let mut rng = Rng(0x5a7e_0004);
        for case in 0..36 {
            t.reset();
            machine(t, &mut rng);
            let opens = !rng.chance(4);
            let file = if opens { 0x7700 + rng.below(0x100) } else { 0 };
            let r = rng.word();
            t.stub_import(sv::WSPRINTFW_IAT, false, 5, 0, r);
            t.stub_import(sv::WFOPEN_IAT, false, 2, 0, file);
            let r = rng.word();
            t.stub_import(sv::FWRITE_IAT, false, 4, 0, r);
            let r = rng.word();
            t.stub_import(sv::FCLOSE_IAT, false, 1, 0, r);
            let data: Vec<u32> = (0..0x1c00).map(|_| rng.next()).collect();
            let src = t.words(&data);
            // a short file now and then: the flag word past its end
            let max = if rng.chance(3) {
                rng.below(0x3d60)
            } else {
                0x7000
            };
            let r = rng.word();
            t.stub_import_fill(sv::FREAD_IAT, src, max, r);
            let r = rng.word();
            t.stub(sv::NEED_RENAME, false, 1, 0, r);
            let slot = rng.below(6);
            let what = format!("ReadFile #{case}");
            t.check_framed(
                &what,
                0x006f_0070,
                Conv::Cdecl,
                &[slot],
                sv::bridge::READ_FRAME,
                |m, ebp| Out::U32(sa::read_file(m, ebp, slot)),
            );
            let what = format!("MergeFileFlags #{case}");
            // _chkstk leaves its own words at ebp - 8 (the cookie goes
            // over ebp - 4): stack junk the port has no counterpart of
            t.skip = 8;
            t.check_framed(
                &what,
                0x006e_fd10,
                Conv::Cdecl,
                &[slot],
                sv::bridge::MERGE_FRAME,
                |m, ebp| Out::U32(sa::merge_file_flags(m, ebp, slot)),
            );
            t.skip = 4;
            if opens {
                // a file that doesn't open stores to address 0: both fault
                let what = format!("WriteFile #{case}");
                t.check_framed(
                    &what,
                    0x006f_0860,
                    Conv::Cdecl,
                    &[slot],
                    sv::bridge::WRITE_FRAME,
                    |m, ebp| Out::U32(sa::write_file(m, ebp, slot)),
                );
            }
        }
    });
}

#[test]
fn save_glue_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a7e_0005);
        for case in 0..24 {
            t.reset();
            machine(t, &mut rng);
            let obj = t.zeros(0x400);
            let byte = u8::from(rng.chance(2));
            t.write(obj + 1, &[byte]);
            let mgr = if case % 4 == 0 { 0 } else { obj };
            let list = [
                (sv::TROPHY_MANAGER, false, 0, 0, mgr),
                (sv::SAVE_MANAGER, false, 0, 0, obj),
                (sv::SAVE_TAKE_BYTE364, true, 0, 0, rng.next()),
                (sv::SAVE_SET_MEMBER_C, true, 1, 1, rng.word()),
                (sv::INPUT_MANAGER, false, 0, 0, obj + 0x100),
                (sv::INPUT_STICK_INFO, true, 0, 0, rng.next()),
                (sv::INPUT_IS_HOLD, true, 1, 1, rng.next()),
                (sv::INPUT_IS_PRESS, true, 1, 1, rng.next()),
                (sv::LOC_MANAGER, false, 0, 0, obj + 0x200),
                (sv::LOC_TEXT, true, 3, 3, rng.word()),
                (sv::USER_FREE, false, 1, 0, rng.word()),
                (sv::VIDEO_INFO, false, 2, 0, rng.word()),
                (sv::IGGY_DTOR, true, 0, 0, rng.word()),
                (sv::OPERATOR_DELETE, false, 1, 0, rng.word()),
            ];
            t.stubs(&list);
            let what = format!("glue #{case}");
            t.check(&what, 0x0064_8ff0, Conv::Cdecl, &[], |m| {
                Out::U32(sa::trophy_get_error(m))
            });
            if byte == 0 {
                // with the byte set the original returns the eax it was
                // called with (its only caller passes eax = ecx), which a
                // direct call doesn't set: TrophyGetError covers that path
                t.check(&what, 0x0067_ac60, Conv::Thiscall(obj), &[], |m| {
                    Out::U32(sa::trophy_flag(m, obj))
                });
            }
            t.check(&what, 0x0064_8010, Conv::Cdecl, &[], |m| {
                Out::U32(sa::save_take_byte364(m))
            });
            t.check(&what, 0x0064_1680, Conv::Cdecl, &[], |m| {
                sa::trophy_manager_update(m);
                Out::None
            });
            t.check(&what, 0x0063_0c90, Conv::Cdecl, &[], |m| {
                Out::U32(sa::stick_info(m))
            });
            let b = rng.word();
            t.check(&what, 0x0063_0cd0, Conv::Cdecl, &[b], |m| {
                Out::U32(sa::input_button(m, sv::INPUT_IS_HOLD, b))
            });
            t.check(&what, 0x0063_0cf0, Conv::Cdecl, &[b], |m| {
                Out::U32(sa::input_button(m, sv::INPUT_IS_PRESS, b))
            });
            t.check(&what, 0x0064_7d40, Conv::Cdecl, &[], |m| {
                Out::U32(sa::enable_save(m))
            });
            t.check(&what, 0x0064_7d50, Conv::Cdecl, &[b], |m| {
                Out::U32(sa::set_save_mode(m, b))
            });
            let st = [1u32, 3, 5, 0, 2, 7][rng.below(6) as usize];
            t.w32(obj + 8, st);
            t.check(&what, 0x006f_1aa0, Conv::Thiscall(obj), &[b], |m| {
                Out::U32(sa::set_result(m, obj, b))
            });
            let out = t.zeros(0x40);
            t.check(&what, 0x0064_7f30, Conv::Cdecl, &[b, out], |m| {
                Out::U32(sa::get_loc(m, b, out))
            });
            t.check(&what, 0x0070_2fa0, Conv::Cdecl, &[], |m| {
                sa::release_menu(m);
                Out::None
            });
            t.check(&what, 0x0068_7010, Conv::Cdecl, &[b], |m| {
                Out::U32(sa::user_free_thunk(m, b))
            });
        }
    });
}

#[test]
fn save_load_succeed_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a7e_0006);
        for case in 0..24 {
            t.reset();
            machine(t, &mut rng);
            let obj = t.zeros(0x400);
            let buf = t.zeros(0x100);
            let flags = rng.next();
            t.w32(buf + 0xe8, flags);
            let v = [0u16, 1, 0x100, 0x8000, 0xffff][rng.below(5) as usize];
            t.write(buf + 0xea, &v.to_le_bytes());
            let list = [
                (sv::SAVE_MANAGER, false, 0, 0, obj),
                (sv::SAVE_SET_RESULT, true, 1, 1, rng.word()),
                (sv::SAVE_REF_BUFFER, true, 0, 0, buf),
                (sv::VIDEO_INFO, false, 2, 0, rng.word()),
            ];
            t.stubs(&list);
            if rng.chance(2) {
                // nothing merged yet: the load's flag and video words
                t.g32(sv::MERGED_FLAGS[0], 0);
            }
            let kind = [3u32, 3, 1, 0][rng.below(4) as usize];
            let what = format!("SaveDataSaveLoadSucceed #{case} kind {kind}");
            t.check(&what, 0x0064_8540, Conv::Cdecl, &[kind], |m| {
                Out::U32(sa::save_load_succeed(m, kind))
            });
        }
    });
}

#[test]
fn save_timers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a7e_0007);
        for case in 0..40 {
            t.reset();
            let base = rng.next();
            t.g32(sv::TICK_BASE, base);
            let d = match rng.below(4) {
                0 => rng.below(4000),
                1 => 0x8000_0000 + rng.below(100),
                2 => rng.next(),
                _ => rng.below(100_000),
            };
            t.stub_import(sv::GET_TICK_COUNT_IAT, false, 0, 0, base.wrapping_add(d));
            let what = format!("wait_fraction #{case}");
            let addr = t.rebase(0x006f_0670);
            let ra = t.a.call(addr, Conv::Cdecl, &[]).unwrap();
            let poison = t.rebase(POISON_AT);
            let port = sa::wait_fraction(&mut G(&mut t.b, poison));
            assert_eq!(
                ra.st0_f32().map(f32::to_bits),
                Some(port.to_bits()),
                "{what}"
            );
            t.diff(&what);
            // SaveData_Update over it
            let limit = Mem::read::<f32>(&t.a, t.rebase(sv::TROPHY_INTERVAL));
            let f = [0.0f32, 1.0, limit, limit - 0.01, 0.5][rng.below(5) as usize];
            t.reset();
            t.stub_float(0x006f_0670, f);
            let obj = t.zeros(0x40);
            t.stubs(&[
                (sv::TROPHY_MANAGER_UPDATE, false, 0, 0, 1),
                (sv::PLAYER_POS, false, 1, 0, 2),
                (sv::SAVE_MANAGER, false, 0, 0, obj),
                (sv::MACHINE_UPDATE_THUNK, true, 0, 0, 3),
                (sv::TROPHY_MANAGER, false, 0, 0, obj + 8),
            ]);
            let what = format!("SaveData_Update #{case} f {f}");
            t.check(&what, 0x0064_8480, Conv::Cdecl, &[], |m| {
                Out::U32(sa::save_data_update(m))
            });
        }
    });
}

#[test]
fn set_input_info_matches_the_original() {
    Twins::each(|t| {
        t.skip = 4;
        let mut rng = Rng(0x5a7e_0008);
        for case in 0..24 {
            t.reset();
            let obj = t.zeros(0x200);
            t.stubs(&[
                (sv::CONTROLLER_MANAGER, false, 0, 0, 0x1234),
                (sv::CONTROLLER_INFO, true, 2, 2, 0),
            ]);
            let mut init = Vec::new();
            init.extend(rng.next().to_le_bytes());
            for _ in 0..4 {
                let bits = match rng.below(4) {
                    // a quiet NaN (the emulator keeps signalling ones signalling)
                    0 => 0x7fc0_0001u32,
                    1 => 0xff80_0000,
                    _ => rng.next(),
                };
                init.extend(bits.to_le_bytes());
            }
            let old = rng.next();
            t.w32(obj, old);
            let what = format!("setInputInfo #{case}");
            t.check_framed_init(
                &what,
                0x0063_1550,
                Conv::Thiscall(obj),
                &[],
                0x18,
                &init,
                |m, ebp| Out::U32(sa::set_input_info(m, ebp, obj)),
            );
        }
    });
}

// ---- round 3 (ffx_system::m5a_glue) ----

/// The forwarders: the input manager's (`inputClear`,
/// `inputClearAllInputInfo`, `inputGetCurrDeviceButtonInfo`,
/// `Input_AsyncPoll`, `inputIsReleaseButton`, `inputIsRepButton`),
/// `SaveDataGetLoc`, the voice-volume and voice-table ones, `fiosLseek`
/// and `sceLseek` (null file or not), the `ListVector_dtor` thunk and
/// `vsnprintf1K` (its `va_list` the caller's fourth slot): stubbed
/// callees, then the button reads again on the real callees.
#[test]
#[allow(clippy::too_many_lines)] // eighteen forwarders, one case setup
fn glue_forwarders_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a30_0001);
        for case in 0..40 {
            t.reset();
            let (m, c, app, loc, fmod, voices) = (
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
            );
            t.stubs(&[
                (gl::INPUT_MANAGER, false, 0, 0, m),
                (gl::INPUT_BUTTON_INFO, true, 0, 0, rng.next()),
                (gl::INPUT_IS_RELEASE, true, 1, 1, rng.next()),
                (gl::INPUT_IS_REP, true, 1, 1, rng.next()),
                (gl::CONTROLLER_MANAGER, false, 0, 0, c),
                (gl::CLEAR_CONTROLLER_INFO, true, 0, 0, rng.next()),
                (gl::GET_APPLICATION, false, 0, 0, app),
                (gl::POLL_INPUT_ASYNC, true, 0, 0, rng.next()),
                (gl::LOC_MANAGER, false, 0, 0, loc),
                (gl::LOC_TEXT, true, 3, 3, rng.next()),
                (gl::FMOD_COMMAND3, true, 3, 3, rng.next()),
                (gl::VOICE_SELECT, true, 4, 4, rng.next()),
                (gl::VOICE_FIND, true, 2, 2, rng.next()),
                (gl::STREAM_SEEK, true, 2, 2, rng.next()),
                (gl::LIST_VECTOR_DTOR, true, 0, 0, rng.next()),
            ]);
            t.stub_import(gl::VSNPRINTF_S_IAT, false, 5, 0, rng.next());
            t.g32(gl::FMOD_MANAGER, fmod);
            t.g32(gl::VOICE_TABLE, voices);
            let none = |name: &str| format!("{name} #{case}");
            t.check(&none("inputClear"), 0x0063_0c30, Conv::Cdecl, &[], |m| {
                Out::U32(gl::input_clear(m))
            });
            t.check(
                &none("inputClearAllInputInfo"),
                0x0063_0c40,
                Conv::Cdecl,
                &[],
                |m| Out::U32(gl::clear_all_input_info(m)),
            );
            t.check(
                &none("inputGetCurrDeviceButtonInfo"),
                0x0063_0c80,
                Conv::Cdecl,
                &[],
                |m| Out::U32(gl::button_info(m)),
            );
            t.check(
                &none("Input_AsyncPoll"),
                0x0063_0cb0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(gl::async_poll(m)),
            );
            let b = rng.word();
            for (sym, method) in [
                (0x0063_0d10u32, gl::INPUT_IS_RELEASE),
                (0x0063_0d30, gl::INPUT_IS_REP),
            ] {
                t.check(&none(&format!("{sym:#x}")), sym, Conv::Cdecl, &[b], |m| {
                    Out::U32(gl::input_button(m, method, b))
                });
            }
            let (id, out) = (rng.word(), rng.next());
            t.check(
                &none("SaveDataGetLoc"),
                0x0064_12d0,
                Conv::Cdecl,
                &[id, out],
                |m| Out::U32(gl::save_data_get_loc(m, id, out)),
            );
            let (v, w) = (rng.word(), rng.word());
            t.check(
                &none("iVoiceVolumeDirect"),
                0x0067_a7b0,
                Conv::Cdecl,
                &[v],
                |m| Out::U32(gl::voice_volume_direct(m, v)),
            );
            t.check(
                &none("iVoiceVolumeTranslate"),
                0x0067_a7d0,
                Conv::Cdecl,
                &[v, w],
                |m| Out::U32(gl::voice_volume_translate(m, v, w)),
            );
            let a: Vec<u32> = (0..4).map(|_| rng.word()).collect();
            t.check(&none("0x0067a9b0"), 0x0067_a9b0, Conv::Cdecl, &a, |m| {
                Out::U32(gl::voice_select_forward(m, a[0], a[1], a[2], a[3]))
            });
            t.check(
                &none("0x0067a9d0"),
                0x0067_a9d0,
                Conv::Cdecl,
                &a[..2],
                |m| Out::U32(gl::voice_find_forward(m, a[0], a[1])),
            );
            let f = if rng.chance(3) { 0 } else { rng.next() };
            for sym in [0x0067_9710u32, 0x0062_f8e0] {
                t.check(
                    &none(&format!("{sym:#x} f {f:#x}")),
                    sym,
                    Conv::Cdecl,
                    &[f, a[0], a[1]],
                    |m| Out::U64(gl::fios_lseek(m, f, a[0], a[1])),
                );
            }
            let this = rng.next();
            t.check(
                &none("0x00634360"),
                0x0063_4360,
                Conv::Thiscall(this),
                &[],
                |m| Out::U32(gl::list_vector_dtor_thunk(m, this)),
            );
            // the va_list is the address of the caller's fourth slot
            let slots = t.frame_ebp(4) + 8;
            t.check(&none("vsnprintf1K"), 0x0068_7070, Conv::Cdecl, &a, |m| {
                Out::U32(gl::vsnprintf_1k(m, a[0], a[1], a[2], slots + 12))
            });
            // the real input manager
            t.reset();
            let b = 1 << rng.below(32);
            for (sym, method) in [
                (0x0063_0d10u32, gl::INPUT_IS_RELEASE),
                (0x0063_0d30, gl::INPUT_IS_REP),
            ] {
                let what = format!("{sym:#x} real #{case}");
                t.check(&what, sym, Conv::Cdecl, &[b], |m| {
                    Out::U32(gl::input_button(m, method, b))
                });
            }
            t.check(
                &none("inputGetCurrDeviceButtonInfo real"),
                0x0063_0c80,
                Conv::Cdecl,
                &[],
                |m| Out::U32(gl::button_info(m)),
            );
            t.check(
                &none("inputClear real"),
                0x0063_0c30,
                Conv::Cdecl,
                &[],
                |m| Out::U32(gl::input_clear(m)),
            );
        }
    });
}

/// `Virtuos_InputManager_setVibration`: pads with and without a device,
/// vibration on and off; `0x0062f5b0` (`GetSystemTime` stubbed to write a
/// random `SYSTEMTIME`).
#[test]
fn glue_vibration_and_clock_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a30_0002);
        for case in 0..40 {
            t.reset();
            let table = t.zeros(0x60);
            let app = t.zeros(0x300);
            t.w32(app + 0x2b8, table);
            let pad = rng.below(10);
            if !rng.chance(3) {
                let dev = rng.next();
                t.w32(table + 0x38 + 4 * pad, dev);
            }
            let this = t.zeros(0x100);
            let on = if rng.chance(2) { 0 } else { rng.word() };
            t.w32(this + 0xa8, on);
            t.stubs(&[
                (gl::GET_APPLICATION, false, 0, 0, app),
                (gl::PAD_SET_VIBRATION, true, 2, 2, rng.next()),
            ]);
            let (small, large) = (rng.word(), rng.word());
            let what = format!("setVibration #{case} pad {pad}");
            t.check(
                &what,
                0x0063_29c0,
                Conv::Thiscall(this),
                &[pad, small, large],
                |m| Out::U32(gl::set_vibration(m, this, pad, small, large)),
            );
            let time: Vec<u8> = (0..16).map(|_| rng.next() as u8).collect();
            let src = t.alloc(&time);
            let r = rng.next();
            let stub = t.stub_code_writes(gl::GET_SYSTEM_TIME_IAT, false, 1, 1, r, 0, src, 16);
            let at = t.rebase(gl::GET_SYSTEM_TIME_IAT);
            t.patch(at, &stub.to_le_bytes());
            let out = t.alloc(&(0..8).map(|_| rng.next() as u8).collect::<Vec<_>>());
            t.skip = 4;
            let what = format!("0x0062f5b0 #{case}");
            t.check_framed(
                &what,
                0x0062_f5b0,
                Conv::Cdecl,
                &[out],
                gl::SYSTEM_TIME_FRAME,
                |m, ebp| Out::U32(gl::system_clock(m, ebp, out)),
            );
            t.skip = 0;
        }
    });
}

/// A circular `std::list` of `n` nodes after the head; returns the owner
/// (`+0` the head, `+4` the size).
fn std_list(t: &mut Twins, n: u32) -> u32 {
    let head = t.zeros(0x10);
    let mut prev = head;
    for _ in 0..n {
        let node = t.zeros(0x10);
        t.w32(prev, node);
        t.w32(node + 4, prev);
        prev = node;
    }
    t.w32(prev, head);
    t.w32(head + 4, prev);
    t.words(&[head, n])
}

/// The unwinding destructors and small teardowns: `0x00631230` (kept or
/// not), `0x00631060` (buffer set or not), `0x00634200` (tagged or not),
/// `0x00634340`, `0x00634370` (0..4 nodes), `0x006343b0`/`0x006343e0`
/// (framed: the erase's iterator at `ebp - 4`), `0x00634500` (empty or
/// not), `0x00633f70`, `0x00630b10` (device set or not), `0x00686bb0`
/// (`next` 0 or not), `0x00630f40` (`inputGetCurTime` a random float).
#[test]
#[allow(clippy::too_many_lines)] // thirteen functions, one case setup
fn glue_dtors_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a30_0003);
        for case in 0..40 {
            t.reset();
            t.stubs(&[
                (gl::USER_FREE_THUNK, false, 1, 0, rng.next()),
                (gl::PHYRE_FREE, false, 1, 0, rng.next()),
                (gl::OPERATOR_DELETE, false, 1, 0, rng.next()),
                (gl::TREE_ERASE_636D60, true, 3, 3, rng.next()),
                (gl::TREE_ERASE_6370D0, true, 3, 3, rng.next()),
                (gl::DESTROY_RANGE, false, 2, 2, rng.next()),
                (gl::TEARDOWN_636970, true, 0, 0, rng.next()),
                (gl::FREE_LIST_DTOR, true, 0, 0, rng.next()),
                (gl::TREE_CLEAR, true, 0, 0, rng.next()),
                (gl::INPUT_TEARDOWN, false, 0, 0, rng.next()),
                (gl::TOUCH_RECORD_CREATE, false, 0, 0, rng.next()),
                (gl::TOUCH_MANAGER, false, 0, 0, rng.next()),
                (gl::GUI_TOUCH_RESET, true, 0, 0, rng.next()),
                (gl::MOUSE_STATE_CREATE, false, 0, 0, rng.next()),
                (gl::ARRAY16_CREATE, false, 0, 0, rng.next()),
            ]);
            let words = |rng: &mut Rng| -> Vec<u32> {
                (0..0x30)
                    .map(|_| if rng.chance(3) { 0 } else { rng.next() })
                    .collect()
            };
            let w = words(&mut rng);
            let this = t.words(&w);
            let keep = [0, 1, 0x100, 0xff00, rng.word()][rng.below(5) as usize];
            let what = format!("0x00631230 #{case} keep {keep:#x}");
            t.check(&what, 0x0063_1230, Conv::Thiscall(this), &[keep], |m| {
                gl::buffer_reset(m, this, keep);
                Out::None
            });
            let w = words(&mut rng);
            let this = t.words(&w);
            let what = format!("0x00631060 #{case}");
            t.check(&what, 0x0063_1060, Conv::Thiscall(this), &[], |m| {
                Out::U32(gl::free_member8(m, this))
            });
            let what = format!("0x00634200 #{case}");
            t.check(&what, 0x0063_4200, Conv::Thiscall(this), &[], |m| {
                gl::phyre_free_member_c(m, this);
                Out::None
            });
            let what = format!("0x00634340 #{case}");
            t.check(&what, 0x0063_4340, Conv::Thiscall(this), &[], |m| {
                gl::delete_member0(m, this);
                Out::None
            });
            let list = std_list(t, rng.below(5));
            let what = format!("0x00634370 #{case}");
            t.check(&what, 0x0063_4370, Conv::Thiscall(list), &[], |m| {
                gl::list_dtor(m, list);
                Out::None
            });
            for (sym, erase) in [
                (0x0063_43b0u32, gl::TREE_ERASE_636D60),
                (0x0063_43e0, gl::TREE_ERASE_6370D0),
            ] {
                let head = t.words(&[rng.next(), rng.next(), rng.next()]);
                let tree = t.words(&[head, rng.next()]);
                let what = format!("{sym:#x} #{case}");
                t.check_framed(
                    &what,
                    sym,
                    Conv::Thiscall(tree),
                    &[],
                    gl::TREE_DTOR_FRAME,
                    |m, ebp| {
                        gl::tree_dtor(m, ebp, erase, tree);
                        Out::None
                    },
                );
            }
            let v = t.words(&words(&mut rng)[..3]);
            let what = format!("0x00634500 #{case}");
            t.check(&what, 0x0063_4500, Conv::Thiscall(v), &[], |m| {
                Out::U32(gl::vector_dtor(m, v))
            });
            let what = format!("0x00633f70 #{case}");
            t.check(&what, 0x0063_3f70, Conv::Thiscall(this), &[], |m| {
                gl::tree_set_dtor(m, this);
                Out::None
            });
            let dev = if rng.chance(3) {
                0
            } else {
                let r = rng.next();
                t.object(&[(1, r)])
            };
            t.g32(gl::INPUT_DEVICE, dev);
            let what = format!("0x00630b10 #{case}");
            t.check(&what, 0x0063_0b10, Conv::Cdecl, &[], |m| {
                Out::U32(gl::input_shutdown(m))
            });
            let node = t.zeros(0x24);
            t.stub(gl::OPERATOR_NEW, false, 1, 0, node);
            let next = if rng.chance(2) { 0 } else { rng.next() };
            let prev = rng.next();
            let what = format!("0x00686bb0 #{case} next {next:#x}");
            t.check(&what, 0x0068_6bb0, Conv::Stdcall, &[next, prev], |m| {
                Out::U32(gl::buy_list_node(m, next, prev))
            });
            let now = f32::from_bits(rng.next() & 0x7f7f_ffff);
            t.stub_float(gl::CUR_TIME, now);
            let w = words(&mut rng);
            let this = t.words(&w);
            let what = format!("0x00630f40 #{case}");
            t.check(&what, 0x0063_0f40, Conv::Thiscall(this), &[], |m| {
                Out::U32(gl::controller_ctor(m, this))
            });
        }
    });
}

/// The voice table's lookups `0x006fb380` and `0x006fb410`: no table, a
/// table still loading (the sleep stub marks it loaded), 0..6 rows of
/// small values so that some rows match (several for the find, whose
/// last match counts), a negative count.
#[test]
fn glue_voice_table_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a30_0004);
        for case in 0..120 {
            t.reset();
            let this = t.zeros(0x40);
            let n = rng.below(7);
            for off in [4u32, 0x24, 0x28, 0x2c, 0x30, 0x34, 0x38, 0x3c] {
                let col: Vec<u32> = (0..n.max(1)).map(|_| rng.below(2)).collect();
                let col = t.words(&col);
                t.w32(this + off, col);
            }
            let count = if rng.chance(8) { 0xffff_fffe } else { n };
            t.w32(this + 0x10, count);
            t.w32(this + 0x14, count);
            t.w32(this + 0x1c, u32::from(!rng.chance(5)));
            let loaded = u32::from(!rng.chance(3));
            t.w32(this + 0x20, loaded);
            // the sleep stub: logs, then marks the table loaded
            let log = t.stub_code(gl::SLEEP_MS, false, 1, 0, 0);
            let mut c = vec![0xc7, 0x05];
            c.extend((this + 0x20).to_le_bytes());
            c.extend(1u32.to_le_bytes()); // mov dword [this+0x20], 1
            c.push(0xe9);
            let here = t.cave_next + c.len() as u32 + 4;
            c.extend(log.wrapping_sub(here).to_le_bytes());
            let sleep = t.cave_code(&c);
            t.jump(gl::SLEEP_MS, sleep);
            // bits 8 and 16 of the id: only its low half counts
            let id = rng.below(2)
                | ((rng.below(2) << 8) * u32::from(rng.chance(4)))
                | (rng.below(2) << 16);
            let (b, c, d) = (rng.below(2), rng.below(2), rng.below(2));
            let what = format!("0x006fb380 #{case} n {count:#x}");
            t.check(
                &what,
                0x006f_b380,
                Conv::Thiscall(this),
                &[id, b, c, d],
                |m| Out::U32(gl::voice_select(m, this, id, b, c, d)),
            );
            t.w32(this + 0x20, loaded);
            let what = format!("0x006fb410 #{case} n {count:#x}");
            t.check(&what, 0x006f_b410, Conv::Thiscall(this), &[b, c], |m| {
                Out::U32(gl::voice_find(m, this, b, c))
            });
        }
    });
}

/// The three `xor eax, eax; ret` stubs (host-only table entries) and the
/// PRX-state jump thunks `code_006793b0`/`c0`/`d0`: on the real targets
/// with the PRX state words random, and on recording stubs.
#[test]
fn glue_stubs_and_thunks_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a30_0005);
        for case in 0..24 {
            t.reset();
            for sym in [0x0062_f640u32, 0x0062_f980, 0x0062_fb00] {
                let what = format!("{sym:#x} #{case}");
                t.check(&what, sym, Conv::Cdecl, &[], |_| {
                    Out::U32(ffx_pcore::m5p_stubs::bridge::zero())
                });
            }
            // PhyreFIOS's PRX load state and error words
            t.g32(0x00ce_83c4, rng.below(5));
            t.g32(0x00ce_83c0, rng.below(2));
            let thunks = [
                (0x0067_93b0u32, gl::PRX_IDLE),
                (0x0067_93c0, gl::PRX_IN_PROGRESS),
                (0x0067_93d0, gl::PRX_LOADED),
            ];
            for (sym, target) in thunks {
                let what = format!("{sym:#x} #{case}");
                t.check(&what, sym, Conv::Cdecl, &[], |m| {
                    Out::U32(gl::jump_thunk(m, target))
                });
            }
            for (sym, target) in thunks {
                let r = rng.next();
                t.stub(target, false, 0, 0, r);
                let what = format!("{sym:#x} stubbed #{case}");
                t.check(&what, sym, Conv::Cdecl, &[], |m| {
                    Out::U32(gl::jump_thunk(m, target))
                });
            }
        }
    });
}

// ---- round 3 (ffx_system::m5a_savedata2) ----

/// A recording stub over the original at `sym` (`this`, `n` words, `pops`
/// as [`Twins::stub_code`]) returning `seq[k]` on its `k`-th call (`seq`
/// must be long enough).
fn stub_seq(t: &mut Twins, sym: u32, this: bool, n: u32, pops: u32, seq: &[u32]) {
    let ctr = t.words(&[0]);
    let tbl = t.words(seq);
    let mut c = Vec::new();
    let logw = t.log;
    c.push(0xa1);
    c.extend(logw.to_le_bytes()); // mov eax, [log]
    c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
    c.push(0x05);
    c.extend((logw + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(sym.to_le_bytes()); // mov [eax], tag
    if this {
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
    }
    for i in 0..n {
        c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
        c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
    }
    c.extend([0xff, 0x05]);
    c.extend(logw.to_le_bytes()); // inc dword [log]
    c.push(0xa1);
    c.extend(ctr.to_le_bytes()); // mov eax, [ctr]
    c.extend([0x8b, 0x04, 0x85]);
    c.extend(tbl.to_le_bytes()); // mov eax, [tbl + eax*4]
    c.extend([0xff, 0x05]);
    c.extend(ctr.to_le_bytes()); // inc dword [ctr]
    if pops == 0 {
        c.push(0xc3);
    } else {
        c.push(0xc2);
        c.extend(((pops * 4) as u16).to_le_bytes());
    }
    let addr = t.cave_code(&c);
    t.jump(sym, addr);
}

/// A recording cdecl stub bound to the import slot `slot` for a variadic
/// callee (`sprintf`, `wsprintfW`): logs `long` words, or `short` when
/// its word `which` is `value` (a call with fewer arguments: the words
/// past them are the caller's stack). Returns `value_out`.
#[allow(clippy::too_many_arguments)]
fn stub_import_varargs(
    t: &mut Twins,
    slot: u32,
    which: u32,
    value: u32,
    short: u32,
    most: u32,
    value_out: u32,
) {
    let log = t.log;
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(slot.to_le_bytes()); // mov [eax], tag
    for i in 0..short {
        c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
        c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
    }
    c.extend([0x81, 0x7c, 0x24, (4 + 4 * which) as u8]);
    c.extend(value.to_le_bytes()); // cmp dword [esp+4+4which], value
    let tail: Vec<u8> = (short..most)
        .flat_map(|i| {
            [
                0x8b,
                0x54,
                0x24,
                (4 + 4 * i) as u8,
                0x89,
                0x50,
                (8 + 4 * i) as u8,
            ]
        })
        .collect();
    c.extend([0x74, tail.len() as u8]); // je past the tail
    c.extend(tail);
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    c.push(0xb8);
    c.extend(value_out.to_le_bytes()); // mov eax, value_out
    c.push(0xc3);
    let addr = t.cave_code(&c);
    let at = t.rebase(slot);
    t.patch(at, &addr.to_le_bytes());
}

/// The save layer's forwarders: `SaveDataManager_MarkAlbhedLoaded`,
/// `SaveDataIsAlbhedLoaded` (no manager or one), the three
/// `SaveDataIs*Finished` and `SaveDataResetToIdle` (stubbed and on the real
/// methods), `SaveDataSetState`, `SaveDataSetSaveDataInfo`,
/// `SaveDataPreDebugSave`, `AutoSave_DebugSave_00647e10` (`al` 0 or not),
/// `SaveDataToLoad`, `SaveDataToLoadAllAlBhed`, `SaveDataToSave` (saving
/// enabled or not), the debug-load/save thunks and `debugSavedataLoadAll`,
/// the Eternal Calm sizes (outside the 7 maps, cached, measured) and
/// `EternalCalm_FileSize_0067a9f0` (open or not; sizes around the
/// signed rounding).
#[test]
#[allow(clippy::too_many_lines)] // two dozen forwarders, one case setup
fn save2_forwarders_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a30_0101);
        for case in 0..40 {
            t.reset();
            let obj = t.zeros(0x400);
            let dialog = t.zeros(0x40);
            t.g32(s2::MANAGER, dialog);
            let none = |name: &str| format!("{name} #{case}");
            // the real methods on a manager object
            t.stub(s2b::INSTANCE, false, 0, 0, obj);
            t.w32(obj + 8, rng.below(8));
            t.w32(obj + 0xc, rng.word());
            for (sym, method) in [
                (0x0064_8060u32, s2b::IS_DELETING),
                (0x0064_8090, s2b::IS_LOADING),
                (0x0064_80f0, s2b::IS_SAVING),
                (0x0064_8530, s2b::RESET_IDLE),
            ] {
                t.check(
                    &none(&format!("{sym:#x} real")),
                    sym,
                    Conv::Cdecl,
                    &[],
                    |m| Out::U32(s2b::manager_call(m, method)),
                );
            }
            t.check(
                &none("IsAlbhedLoaded real"),
                0x0064_8020,
                Conv::Cdecl,
                &[],
                |m| Out::U32(s2b::is_albhed_loaded(m)),
            );
            t.check(
                &none("MarkAlbhedLoaded real"),
                0x0064_7e40,
                Conv::Cdecl,
                &[],
                |m| Out::U32(s2b::mark_albhed_loaded(m)),
            );
            let v = rng.word();
            t.check(&none("SetState"), 0x0064_8710, Conv::Cdecl, &[v], |m| {
                Out::U32(s2b::set_state(m, v))
            });
            // stubbed methods
            t.reset();
            let m0 = if rng.chance(4) { 0 } else { rng.next() };
            let pad: Vec<u8> = (0..4).map(|_| rng.next() as u8).collect();
            let pad_at = t.rebase(s2::PAD_WORDS[0]);
            t.write(pad_at, &pad);
            t.stubs(&[
                (s2b::INSTANCE, false, 0, 0, m0),
                // SetWord0 leaves eax (the manager) alone
                (s2b::MARK_ALBHED, true, 0, 0, m0),
                (s2b::IS_ALBHED, true, 0, 0, rng.next()),
                (s2b::IS_DELETING, true, 0, 0, rng.next()),
                (s2b::IS_LOADING, true, 0, 0, rng.next()),
                (s2b::IS_SAVING, true, 0, 0, rng.next()),
                (s2b::RESET_IDLE, true, 0, 0, rng.next()),
                (s2b::SET_CB_RESULT, true, 1, 1, rng.next()),
                (
                    s2b::IS_SAVE_ENABLED,
                    true,
                    0,
                    0,
                    if rng.chance(2) { 0 } else { rng.next() },
                ),
                (s2b::SET_INFO, true, 3, 3, rng.next()),
                (s2b::PRE_DEBUG_SAVE, true, 1, 1, rng.next()),
                (
                    s2b::CHECK_DEBUG_SAVE,
                    true,
                    1,
                    1,
                    rng.next() & !(0xff * u32::from(rng.chance(2))),
                ),
                (s2b::DO_LOAD, true, 0, 0, rng.next()),
                (s2b::LOAD_ALL, true, 0, 0, rng.next()),
                (s2b::DEBUG_LOAD, false, 3, 0, rng.next()),
                (s2b::DEBUG_SAVE, false, 3, 0, rng.next()),
                (s2b::DEBUG_LOAD_ALL, false, 3, 0, rng.next()),
            ]);
            // doSave's stub keeps the pad words it sees (set for the call)
            let r = rng.next();
            let log = t.stub_code(s2b::DO_SAVE, true, 0, 0, r);
            let snoop = t.zeros(4);
            let mut c = vec![0xa1];
            c.extend(pad_at.to_le_bytes()); // mov eax, [pad words]
            c.push(0xa3);
            c.extend(snoop.to_le_bytes()); // mov [snoop], eax
            c.push(0xe9);
            let here = t.cave_next + c.len() as u32 + 4;
            c.extend(log.wrapping_sub(here).to_le_bytes()); // jmp log
            let at = t.cave_code(&c);
            t.jump(s2b::DO_SAVE, at);
            t.check(
                &none("MarkAlbhedLoaded"),
                0x0064_7e40,
                Conv::Cdecl,
                &[],
                |m| Out::U32(s2b::mark_albhed_loaded(m)),
            );
            t.check(
                &none("IsAlbhedLoaded"),
                0x0064_8020,
                Conv::Cdecl,
                &[],
                |m| Out::U32(s2b::is_albhed_loaded(m)),
            );
            let a: Vec<u32> = (0..3).map(|_| rng.word()).collect();
            t.check(
                &none("SetSaveDataInfo"),
                0x0064_86c0,
                Conv::Cdecl,
                &a,
                |m| Out::U32(s2b::set_save_data_info_forward(m, a[0], a[1], a[2])),
            );
            t.check(
                &none("PreDebugSave"),
                0x0064_84d0,
                Conv::Cdecl,
                &a[..1],
                |m| Out::U32(s2b::pre_debug_save_forward(m, a[0])),
            );
            t.check(
                &none("AutoSave_DebugSave"),
                0x0064_7e10,
                Conv::Cdecl,
                &a[..1],
                |m| Out::U32(s2b::auto_debug_save(m, a[0])),
            );
            t.check(&none("ToLoad"), 0x0064_8760, Conv::Cdecl, &[], |m| {
                Out::U32(s2b::to_load(m))
            });
            t.check(&none("ToLoadAll"), 0x0064_8790, Conv::Cdecl, &[], |m| {
                Out::U32(s2b::to_load_all(m))
            });
            t.check(&none("ToSave"), 0x0064_87a0, Conv::Cdecl, &[], |m| {
                Out::U32(s2b::to_save(m))
            });
            for (sym, target) in [
                (0x0064_92b0u32, s2b::DEBUG_LOAD),
                (0x0064_92e0, s2b::DEBUG_SAVE),
            ] {
                t.check(&none(&format!("{sym:#x}")), sym, Conv::Cdecl, &a, |m| {
                    Out::U32(s2b::debug_thunk(m, target, a[0], a[1], a[2]))
                });
            }
            t.check(
                &none("debugSavedataLoadAll"),
                0x0064_92c0,
                Conv::Cdecl,
                &a[..1],
                |m| Out::U32(s2b::debug_load_all(m, a[0])),
            );
            // the Eternal Calm sizes
            let first = rng.below(0x100);
            t.g32(s2::CALM_FIRST, first);
            for tbl in [s2::CALM_MGRP_SIZES, s2::CALM_SCENE_SIZES] {
                let sizes: Vec<u32> = (0..7)
                    .map(|_| {
                        if rng.chance(2) {
                            0xffff_ffff
                        } else {
                            rng.below(0x10000)
                        }
                    })
                    .collect();
                let at = t.rebase(tbl);
                let bytes: Vec<u8> = sizes.iter().flat_map(|v| v.to_le_bytes()).collect();
                t.write(at, &bytes);
            }
            t.stub(s2b::CALM_FILE_SIZE, false, 1, 0, rng.word());
            let map = first.wrapping_add(rng.below(10)).wrapping_sub(1);
            for (sym, sizes, names) in [
                (0x0067_aa70u32, s2::CALM_MGRP_SIZES, s2::CALM_MGRP_NAMES),
                (0x0067_aae0, s2::CALM_SCENE_SIZES, s2::CALM_SCENE_NAMES),
            ] {
                let what = none(&format!("{sym:#x} map {map:#x}"));
                t.check(&what, sym, Conv::Cdecl, &[map], |m| {
                    Out::U32(s2b::calm_size(m, sizes, names, map))
                });
            }
            // the measuring itself (its stub above put back)
            t.reset();
            let f = if rng.chance(3) { 0 } else { rng.next() };
            let size = [
                0,
                1,
                15,
                16,
                17,
                0xffff_fff0,
                0xffff_ffef,
                0x7fff_fff5,
                rng.next(),
            ][rng.below(9) as usize];
            t.stubs(&[
                (s2b::SCE_OPEN, false, 2, 0, f),
                (s2b::SCE_FILE_SIZE, false, 1, 0, size),
                (s2b::SCE_CLOSE, false, 1, 0, rng.next()),
            ]);
            t.stub_import(s2b::IOB_FUNC_IAT, false, 0, 0, rng.next());
            t.stub_import(s2b::FPRINTF_IAT, false, 3, 0, rng.next());
            let name = rng.next();
            let what = none(&format!("EternalCalm_FileSize f {f:#x} size {size:#x}"));
            t.check(&what, 0x0067_a9f0, Conv::Cdecl, &[name], |m| {
                Out::U32(s2b::calm_file_size(m, name))
            });
        }
    });
}

/// The manager's own methods: `setSaveDataInfo` (each string null or
/// not), `setSaveDataIconString`, `preDebugSave`, `0x006f1c40` (word 0x1b
/// or not; the debug save's answer 0 or not), `doLoad`, `0x006f1d10`,
/// `doSave` (enabled or not), `0x006f2630` (synthetic tables: ids found,
/// appended), the `std::string` moves `0x006494d0`/`0x00649540` (short
/// and long), `SaveDataLocRemoveNumTag` (tags at random places, short
/// strings) and the load/save starts.
#[test]
#[allow(clippy::too_many_lines)] // one case setup for the methods
fn save2_manager_methods_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a30_0102);
        for case in 0..50 {
            t.reset();
            let dialog = t.zeros(0x40);
            t.g32(s2::MANAGER, dialog);
            let dbg = t.zeros(0x40);
            t.g32(s2::SAVE_BUFFER, dbg);
            t.stub_import(s2b::STRNCPY_IAT, false, 3, 0, rng.next());
            t.stub_import(s2b::MEMMOVE_IAT, false, 3, 0, rng.next());
            t.stubs(&[
                (s2b::MEMCPY, false, 3, 0, rng.next()),
                (
                    s2b::DEBUG_SAVE_INTERNAL,
                    false,
                    2,
                    0,
                    if rng.chance(2) { 0 } else { rng.next() },
                ),
                (s2b::START_LOAD, false, 2, 0, rng.next()),
                (s2b::START_LOAD_ALL, false, 2, 0, rng.next()),
                (s2b::START_SAVE, false, 2, 0, rng.next()),
                (s2b::SET_UP_FOLDER, false, 0, 0, rng.next()),
                (s2b::MENU_OPEN, false, 0, 0, rng.next()),
                (s2b::MENU_CLEAR, false, 0, 0, rng.next()),
                (s2b::SHOW_POPUP, false, 1, 0, rng.next()),
                (s2b::WRITE_CRC, false, 1, 0, rng.next()),
                (s2b::NEW_ARRAY, false, 1, 0, rng.next()),
            ]);
            let none = |name: &str| format!("{name} #{case}");
            let this = t.zeros(0x800);
            let save = t.zeros(0x40);
            t.write(
                save + 0x18,
                &(if rng.chance(3) {
                    0x1bu16
                } else {
                    rng.next() as u16
                })
                .to_le_bytes(),
            );
            t.w32(this, if rng.chance(3) { 0 } else { rng.next() });
            t.w32(this + 0x350, save);
            t.w32(this + 0x354, rng.next());
            let s: Vec<u32> = (0..3)
                .map(|_| if rng.chance(3) { 0 } else { rng.next() })
                .collect();
            t.check(
                &none("setSaveDataInfo"),
                0x006f_2560,
                Conv::Thiscall(this),
                &s,
                |m| Out::U32(s2b::set_save_data_info(m, this, s[0], s[1], s[2])),
            );
            t.check(
                &none("setIconString"),
                0x006f_2530,
                Conv::Thiscall(this),
                &s[..1],
                |m| Out::U32(s2b::set_icon_string(m, this, s[0])),
            );
            t.check(
                &none("preDebugSave"),
                0x006f_2410,
                Conv::Thiscall(this),
                &s[..1],
                |m| Out::U32(s2b::pre_debug_save(m, this)),
            );
            t.check(
                &none("0x006f1c40"),
                0x006f_1c40,
                Conv::Thiscall(this),
                &s[..1],
                |m| Out::U32(s2b::check_debug_save(m, this)),
            );
            t.check(
                &none("doLoad"),
                0x006f_1ce0,
                Conv::Thiscall(this),
                &[],
                |m| Out::U32(s2b::do_load(m, this)),
            );
            t.check(
                &none("0x006f1d10"),
                0x006f_1d10,
                Conv::Thiscall(this),
                &[],
                |m| Out::U32(s2b::load_all(m, this)),
            );
            t.check(
                &none("doSave"),
                0x006f_1d70,
                Conv::Thiscall(this),
                &[],
                |m| Out::U32(s2b::do_save(m, this)),
            );
            // the starts themselves, their callees stubbed again
            t.reset();
            t.stub_import(s2b::STRNCPY_IAT, false, 3, 0, rng.next());
            t.stub_import(s2b::MEMMOVE_IAT, false, 3, 0, rng.next());
            t.stubs(&[
                (s2b::MEMCPY, false, 3, 0, rng.next()),
                (s2b::SET_UP_FOLDER, false, 0, 0, rng.next()),
                (s2b::MENU_OPEN, false, 0, 0, rng.next()),
                (s2b::MENU_CLEAR, false, 0, 0, rng.next()),
                (s2b::SHOW_POPUP, false, 1, 0, rng.next()),
                (s2b::WRITE_CRC, false, 1, 0, rng.next()),
                (s2b::NEW_ARRAY, false, 1, 0, rng.next()),
            ]);
            let (size, buf) = (rng.next(), rng.next());
            for (sym, state) in [(0x006f_01d0u32, 0xb), (0x006e_feb0, 0x15)] {
                t.check(
                    &none(&format!("{sym:#x}")),
                    sym,
                    Conv::Cdecl,
                    &[size, buf],
                    |m| Out::U32(s2b::start_load(m, state, size, buf)),
                );
            }
            t.check(
                &none("0x006f07a0"),
                0x006f_07a0,
                Conv::Cdecl,
                &[size, buf],
                |m| Out::U32(s2b::start_save(m, size, buf)),
            );
            // 0x006f2630: a table of up to 8 entries with small ids
            let n = rng.below(8);
            let mut table = Vec::new();
            for _ in 0..n * 8 {
                table.push(rng.next());
            }
            for k in 0..n as usize {
                let id = rng.below(12);
                table[k * 8 + 1] = (table[k * 8 + 1] & 0xffff) | (id << 16);
            }
            table.extend(std::iter::repeat_n(0, 0x80));
            let table = t.words(&table);
            let mgr = t.alloc(&(0..0x500).map(|_| rng.next() as u8).collect::<Vec<_>>());
            let flags: Vec<u8> = (0..200)
                .map(|i| u8::from((i < 12 || i == 199) && rng.chance(3)) * (1 + rng.below(3) as u8))
                .collect();
            t.write(mgr + 0x3bc, &flags);
            t.w32(mgr + 0x368, table);
            t.w32(mgr + 0x3a0, if rng.chance(8) { 0xffff_ffff } else { n });
            t.check(
                &none("0x006f2630"),
                0x006f_2630,
                Conv::Thiscall(mgr),
                &[],
                |m| {
                    s2b::rebuild_table(m, mgr);
                    Out::None
                },
            );
            // the string moves
            let long = rng.chance(2);
            let src = t.words(&[
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                [0, 1, 0xf, rng.next(), 0xffff_ffff][rng.below(5) as usize],
                if long {
                    0x10 + rng.below(0x100)
                } else {
                    rng.below(0x10)
                },
                rng.next(),
            ]);
            let dst = t.zeros(0x20);
            for sym in [0x0064_94d0u32, 0x0064_9540] {
                let x = rng.next();
                t.check(
                    &none(&format!("{sym:#x} long {long}")),
                    sym,
                    Conv::Thiscall(dst),
                    &[src, x],
                    |m| Out::U32(s2b::string_move(m, dst, src)),
                );
            }
            // SaveDataLocRemoveNumTag
            // a one-character tag now and then (its NUL matches the end)
            let second = if rng.chance(4) {
                0
            } else {
                b'0' + rng.below(2) as u8
            };
            let tag = t.alloc(&[b'#', second, 0]);
            t.g32(s2::NUM_TAG, tag);
            let text: Vec<u8> = (0..rng.below(20))
                .map(|_| b"#01a"[rng.below(4) as usize])
                .chain([0])
                .collect();
            let text = t.alloc(&text);
            let out = t.alloc(&(0..0x20).map(|_| rng.next() as u8 | 1).collect::<Vec<_>>());
            t.check(
                &none("LocRemoveNumTag"),
                0x0064_8150,
                Conv::Cdecl,
                &[text, out],
                |m| Out::U32(s2b::loc_remove_num_tag(m, text, out)),
            );
        }
    });
}

/// The framed ones: `SaveDataManager_debugSave_Internal_6f0650` (the file
/// opens or not), `SetUpDefaultSaveFolder` (already set; the folder
/// lookup failing; each directory there, made, or failing) and
/// `SaveDataManager_initSaveDataSys`: the imports stubbed.
#[test]
fn save2_framed_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a30_0103);
        for case in 0..40 {
            t.reset();
            let dialog = t.alloc(&(0..0x40).map(|_| rng.next() as u8 | 1).collect::<Vec<_>>());
            t.g32(s2::MANAGER, dialog);
            t.stubs(&[
                (s2b::WRITE_CRC, false, 1, 0, rng.next()),
                (s2b::SET_UP_FOLDER, false, 0, 0, rng.next()),
                (s2b::MEMSET, false, 3, 0, rng.next()),
                (s2b::STREAM_PREFIX, false, 0, 0, rng.next()),
                (s2b::NEW_ARRAY, false, 1, 0, rng.next()),
            ]);
            t.stub_import(s2b::GET_TICK_COUNT_IAT, false, 0, 0, rng.next());
            t.stub_import(s2b::WSPRINTFW_IAT, false, 5, 0, rng.next());
            let f = if rng.chance(3) { 0 } else { rng.next() };
            t.stub_import(s2b::WFOPEN_IAT, false, 2, 0, f);
            t.stub_import(s2b::FWRITE_IAT, false, 4, 0, rng.next());
            t.stub_import(s2b::FCLOSE_IAT, false, 1, 0, rng.next());
            t.stub_import(s2b::STRNCPY_IAT, false, 3, 0, rng.next());
            let fmt = t.rebase(s2::PREFIX_FORMAT);
            let r = rng.next();
            stub_import_varargs(t, s2b::SPRINTF_IAT, 1, fmt, 3, 5, r);
            let (size, buf) = (rng.next(), rng.next());
            t.skip = 4;
            let what = format!("debugSave_Internal #{case} f {f:#x}");
            t.check_framed(
                &what,
                0x006f_0510,
                Conv::Cdecl,
                &[size, buf],
                s2b::DEBUG_SAVE_FRAME,
                |m, ebp| Out::U32(s2b::debug_save_internal(m, ebp, size, buf)),
            );
            let this = t.zeros(0x400);
            let what = format!("initSaveDataSys #{case}");
            t.check_framed(
                &what,
                0x006f_1fa0,
                Conv::Thiscall(this),
                &[buf, size],
                s2b::INIT_FRAME,
                |m, ebp| Out::U32(s2b::init_save_data_sys(m, ebp, this, buf, size)),
            );
            // SetUpDefaultSaveFolder (the real one: the stub above is gone)
            t.reset();
            t.g32(s2::FOLDER_SET, u32::from(rng.chance(6)));
            let hr = if rng.chance(5) {
                0x8000_4005
            } else {
                rng.below(2)
            };
            t.stub_import(s2b::SH_GET_FOLDER_PATH_IAT, false, 5, 5, hr);
            let fmt = t.rebase(s2::DIR_FORMAT);
            let r = rng.next();
            stub_import_varargs(t, s2b::WSPRINTFW_IAT, 1, fmt, 3, 4, r);
            let attrs: Vec<u32> = (0..3)
                .map(|_| [0xffff_ffff, 0x10, 0x30, 0x20, rng.next()][rng.below(5) as usize])
                .collect();
            let made: Vec<u32> = (0..3).map(|_| u32::from(!rng.chance(5))).collect();
            t.stub_import_seq(s2b::GET_FILE_ATTRIBUTES_IAT, 1, 1, &attrs);
            t.stub_import_seq(s2b::CREATE_DIRECTORY_IAT, 2, 2, &made);
            // _chkstk leaves its own words at ebp - 8 (the cookie at ebp - 4)
            t.skip = 8;
            let what = format!("SetUpDefaultSaveFolder #{case} hr {hr:#x} {attrs:x?} {made:?}");
            t.check_framed(
                &what,
                0x006f_0330,
                Conv::Cdecl,
                &[],
                s2b::FOLDER_FRAME,
                |m, ebp| {
                    s2b::set_up_default_save_folder(m, ebp);
                    Out::None
                },
            );
            t.skip = 0;
        }
    });
}

/// The water-wave shader setters: the parameter lookups found or not (a
/// sequence per call), the values by the caller's slots (logged as
/// pointers); `setScrollSpeed` also leaves three lookups in its argument
/// slots, compared after the call.
#[test]
fn save2_water_wave_setters_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a30_0104);
        for case in 0..60 {
            t.reset();
            let seq: Vec<u32> = (0..8)
                .map(|_| if rng.chance(5) { 0 } else { rng.next() })
                .collect();
            stub_seq(t, s2b::FIND_PARAM, true, 1, 1, &seq);
            t.stub(s2b::SET_VALUE, true, 2, 2, rng.next());
            let c = t.words(&[rng.next(), rng.next()]);
            let none = |name: &str| format!("{name} #{case}");
            let v = rng.next();
            let slots = t.frame_ebp(2) + 8;
            for (sym, name) in [
                (0x006f_5760u32, s2::BUOYANCY_FIX),
                (0x006f_57e0, s2::DRAUGHT_OFFSET),
                (0x006f_5810, s2::FOAM_STRENGTH),
                (0x006f_5840, s2::MULTI_TEX_BLEND),
            ] {
                t.check(
                    &none(&format!("{sym:#x}")),
                    sym,
                    Conv::Stdcall,
                    &[c, v],
                    |m| {
                        s2b::set_one(m, name, c, slots);
                        Out::None
                    },
                );
            }
            let ab: Vec<u32> = (0..2)
                .map(|_| if rng.chance(3) { 0 } else { rng.next() })
                .collect();
            for (sym, names) in [
                (0x006f_5790u32, s2::CIRCLE_WAVE),
                (0x006f_59a0, s2::SINE_WAVE),
            ] {
                t.check(
                    &none(&format!("{sym:#x}")),
                    sym,
                    Conv::Stdcall,
                    &[c, ab[0], ab[1]],
                    |m| {
                        s2b::set_two(m, names, c, ab[0], ab[1]);
                        Out::None
                    },
                );
            }
            t.reset();
            let seq: Vec<u32> = (0..8)
                .map(|_| if rng.chance(8) { 0 } else { rng.next() })
                .collect();
            stub_seq(t, s2b::FIND_PARAM, true, 1, 1, &seq);
            t.stub(s2b::SET_VALUE, true, 2, 2, rng.next());
            let args: Vec<u32> = std::iter::once(c)
                .chain((0..4).map(|_| rng.next()))
                .collect();
            let slots = t.frame_ebp(5) + 8;
            // the caller's slots as the call leaves them before the body runs
            let bytes: Vec<u8> = args.iter().flat_map(|w| w.to_le_bytes()).collect();
            Mem::write_bytes(&mut t.b, slots, &bytes);
            t.b.set_call_top(Some(slots - 0x200));
            t.check(
                &none("setScrollSpeed"),
                0x006f_5870,
                Conv::Stdcall,
                &args,
                |m| {
                    s2b::set_scroll_speed(m, c, slots);
                    Out::None
                },
            );
            t.b.set_call_top(None);
            let mut x = [0; 20];
            let mut y = [0; 20];
            Mem::read_bytes(&t.a, slots, &mut x);
            Mem::read_bytes(&t.b, slots, &mut y);
            assert_eq!(x, y, "setScrollSpeed #{case}: the argument slots");
        }
    });
}

// ---- round 3 (ffx_system::m5a_fixup) ----

/// `v` as LEB128, now and then padded with empty continuation bytes (the
/// decoder's shift past 31 bits included).
fn leb_bytes(rng: &mut Rng, v: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let mut x = v;
    loop {
        let b = (x & 0x7f) as u8;
        x >>= 7;
        if x == 0 {
            if rng.chance(6) {
                out.push(b | 0x80);
                let pad = rng.below(5) as usize;
                out.extend(std::iter::repeat_n(0x80, pad));
                out.push(0);
            } else {
                out.push(b);
            }
            return out;
        }
        out.push(b | 0x80);
    }
}

/// The record runs on synthetic streams, both record sizes: bitmasks of
/// 0..40 bits, except/listed counts with byte indices (`N < 0x100`) and
/// LEB128 ones (`N` 0x100..0x120, a null cursor now and then: count 0),
/// indices out of order and out of range, arithmetic progressions; the
/// callback a recording stub.
#[test]
#[allow(clippy::too_many_lines)] // eight runs, one case setup
fn fixup_runs_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a30_0201);
        for case in 0..40 {
            for (size, syms) in [
                (
                    16u32,
                    [0x0060_cac0u32, 0x0060_cc20, 0x0060_ce70, 0x0060_d000],
                ),
                (28, [0x0060_cb70, 0x0060_cd50, 0x0060_cf40, 0x0060_d080]),
            ] {
                for (which, sym) in syms.into_iter().enumerate() {
                    t.reset();
                    let big = which != 0 && rng.chance(3);
                    let n = if big {
                        0x100 + rng.below(0x20) * u32::from(rng.chance(2))
                    } else if rng.chance(8) {
                        0xff
                    } else {
                        rng.below(40)
                    };
                    let desc = t.words(&[rng.next(), n]);
                    let r = rng.next();
                    let cb = t.stub_code(0x7700, false, 4, 0, r);
                    let tmpl: Vec<u32> = (0..7).map(|_| rng.next()).collect();
                    let tmpl = t.words(&tmpl);
                    let mut stream = Vec::new();
                    match which {
                        0 => stream.extend((0..n / 8 + 2).map(|_| rng.next() as u8)),
                        1 | 2 => {
                            let count = rng.below(6);
                            stream.extend(leb_bytes(&mut rng, count));
                            // ascending, one pair out of order now and then
                            // (the except run's records stay under 2N)
                            let mut idx: Vec<u32> = (0..count).map(|_| rng.below(n + 3)).collect();
                            idx.sort_unstable();
                            if count > 1 && rng.chance(3) {
                                idx.swap(0, 1);
                            }
                            for i in idx {
                                if big {
                                    stream.extend(leb_bytes(&mut rng, i));
                                } else {
                                    stream.push(i as u8);
                                }
                            }
                        }
                        _ => {
                            let start = rng.below(n + 5);
                            let step = [0, 1, 2, 0xffff_ffff, rng.next()][rng.below(5) as usize];
                            let count = rng.below(6);
                            for v in [start, step, count] {
                                stream.extend(leb_bytes(&mut rng, v));
                            }
                        }
                    }
                    stream.extend([0; 8]);
                    let stream = t.alloc(&stream);
                    let null = which != 0 && big && rng.chance(4);
                    let cursor = t.words(&[if null { 0 } else { stream }]);
                    let max = if which == 1 { 2 * n + 16 } else { n.max(8) };
                    let buf = t.zeros((max * size) as usize);
                    let out = t.words(&[buf]);
                    let run = fx::Run {
                        desc,
                        cb,
                        tmpl,
                        cursor,
                        out,
                        size,
                    };
                    let what = format!("{sym:#x} #{case} n {n:#x} null {null}");
                    t.check(
                        &what,
                        sym,
                        Conv::Thiscall(desc),
                        &[cb, tmpl, cursor, out],
                        |m| match which {
                            0 => {
                                fx::run_bitmask(m, run);
                                Out::None
                            }
                            1 => Out::U32(fx::run_except(m, run)),
                            2 => Out::U32(fx::run_listed(m, run)),
                            _ => Out::U32(fx::run_arithmetic(m, run)),
                        },
                    );
                }
            }
        }
    });
}

/// A run stub over `sym` (thiscall, 4 words, `ret 0x10`): moves the
/// output cursor (`*arg 3`) on by `advance` bytes, logs, returns `ret`.
fn stub_run(t: &mut Twins, sym: u32, advance: u32, ret: u32) {
    let log = t.stub_code(sym, true, 4, 4, ret);
    let mut c = vec![0x8b, 0x44, 0x24, 0x10]; // mov eax, [esp+0x10]
    c.extend([0x81, 0x00]);
    c.extend(advance.to_le_bytes()); // add dword [eax], advance
    c.push(0xe9);
    let here = t.cave_next + c.len() as u32 + 4;
    c.extend(log.wrapping_sub(here).to_le_bytes()); // jmp log
    let at = t.cave_code(&c);
    t.jump(sym, at);
}

/// A synthetic fixup stream for `Fixup_UnpackList` (`array` false) or
/// `Fixup_UnpackArrayList`: `lists` lists for `count` objects with the
/// runs stubbed to make one record each ([`stub_run`]); returns the
/// stream and the number of records it makes.
#[allow(clippy::too_many_lines)] // every list kind's stream
fn fixup_stream(rng: &mut Rng, array: bool, count: u32, lists: u32, k: u32) -> (Vec<u8>, u32) {
    let mut s = Vec::new();
    let mut made = 0;
    for _ in 0..lists {
        // a nested list only with objects to fill
        let kind = loop {
            let k = rng.below(8);
            if k != 1 || count > 0 {
                break k;
            }
        };
        let extra = [0u8, 0x08, 0x10, 0x20, 0x40, 0x78, 0x58, 0x80, 0xf8][rng.below(9) as usize];
        // bits 0-2 are the kind (the flags drop bits 1 and 2)
        let b = kind as u8 | extra;
        s.push(b);
        let flags = u32::from(b & !6) | 1 | if count == 1 { 2 } else { 0 };
        let v = rng.next() >> rng.below(32);
        s.extend(leb_bytes(rng, v));
        let leb = |rng: &mut Rng, s: &mut Vec<u8>| {
            let v = rng.below(300);
            s.extend(leb_bytes(rng, v));
        };
        if !array && flags & 0x20 != 0 {
            leb(rng, &mut s);
        }
        match kind {
            0 | 4 | 6 => made += 1,
            2 | 3 => {
                // the array form reads `+8` (unless flag 8) and `+0xc` of
                // each record the run made
                if array {
                    for _ in 0..k {
                        if flags & 8 == 0 {
                            leb(rng, &mut s);
                        }
                        leb(rng, &mut s);
                    }
                }
                made += k;
            }
            5 => {
                let c = rng.below(4);
                s.extend(leb_bytes(rng, c));
                for _ in 0..c {
                    let reads = if array {
                        u32::from(flags & 1 == 0)
                            + u32::from(flags & 2 == 0)
                            + u32::from(flags & 8 == 0)
                            + 1
                    } else {
                        u32::from(flags & 1 == 0) + u32::from(flags & 2 == 0)
                    };
                    for _ in 0..reads {
                        leb(rng, &mut s);
                    }
                }
                made += c;
            }
            1 => {
                // `count` sub-lists that make a record each, a reported one
                // between them now and then
                let mut left = count;
                while left > 0 {
                    let bad = rng.chance(6);
                    // listed and except sub-lists make `k` records, the
                    // others one: never past `count`
                    let kind2 = if bad {
                        [0u8, 1, 5, 7, 0x20][rng.below(5) as usize]
                    } else if left >= k {
                        [2u8, 3, 4, 6][rng.below(4) as usize]
                    } else {
                        [4u8, 6][rng.below(2) as usize]
                    };
                    s.push(kind2);
                    if array {
                        if flags & 8 == 0 {
                            leb(rng, &mut s);
                        }
                        leb(rng, &mut s);
                    } else {
                        let mut more = true;
                        if flags & 0x10 == 0 {
                            let x = if rng.chance(2) { 0 } else { 1 + rng.below(9) };
                            s.extend(leb_bytes(rng, x));
                            more = x == 0;
                        }
                        if more {
                            leb(rng, &mut s);
                            if flags & 0x20 == 0 {
                                leb(rng, &mut s);
                            }
                            if flags & 0x40 == 0 {
                                leb(rng, &mut s);
                            }
                        }
                        if flags & 8 == 0 {
                            leb(rng, &mut s);
                        }
                    }
                    if !bad {
                        left -= if kind2 == 2 || kind2 == 3 { k } else { 1 };
                    }
                }
                made += count;
            }
            _ => {}
        }
    }
    s.extend([0; 16]);
    (s, made)
}

/// `Fixup_UnpackList` and `Fixup_UnpackArrayList` over synthetic streams
/// of every list kind and nested kind (the runs stubbed to make one
/// record each, `Fixup_ReadRecord`, `Fixup_ReadLeb128` the original),
/// one object or several; framed (the templates and the descriptor the
/// runs log, compared after the call) with the output cursor in the
/// caller's `records` slot (compared too). Not reached: the inline
/// value read's null check (the cursor was just read through) and, in the
/// array form, a null cursor before a nested list's `+0xc`.
#[test]
#[allow(clippy::too_many_lines)] // two unpackers, one stream generator
fn fixup_unpack_lists_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a30_0202);
        for case in 0..80 {
            for array in [false, true] {
                t.reset();
                let size = if array { 16 } else { 28 };
                let count = [0, 1, 2, 3, rng.below(6)][rng.below(5) as usize];
                let lists = 1 + rng.below(5);
                let k = 1 + rng.below(2);
                let (stream, made) = fixup_stream(&mut rng, array, count, lists, k);
                let stream = t.alloc(&stream);
                let cursor = t.words(&[stream]);
                let records = t.zeros(((made + 2) * size) as usize);
                let stubs: [(u32, u32); 5] = if array {
                    [
                        (fx::RUN_ALL16, 0),
                        (fx::RUN_LISTED16, k),
                        (fx::RUN_EXCEPT16, count.wrapping_sub(k)),
                        (fx::RUN_BITMASK16, 0),
                        (fx::RUN_ARITHMETIC16, 0),
                    ]
                } else {
                    [
                        (fx::RUN_ALL, 0),
                        (fx::RUN_LISTED, k),
                        (fx::RUN_EXCEPT, count.wrapping_sub(k)),
                        (fx::RUN_BITMASK, 0),
                        (fx::RUN_ARITHMETIC, 0),
                    ]
                };
                for (j, (sym, ret)) in stubs.into_iter().enumerate() {
                    // listed and except make `k` records and answer how many
                    // follow; the others make one
                    let (made, r) = if j == 1 || j == 2 {
                        (k, ret)
                    } else {
                        (1, rng.next())
                    };
                    stub_run(t, sym, made * size, r);
                }
                t.stub(fx::READ_RECORD, true, 2, 2, rng.next());
                t.stub(fx::PRINTF, false, 3, 0, rng.next());
                let n = made;
                let args = [cursor, records, n, count];
                let slots = t.frame_ebp(4) + 8;
                // the caller's records slot, which the runs advance
                Mem::write(&mut t.b, slots + 4, records);
                let (sym, low) = if array {
                    (0x0060_a340u32, fx::ARRAY_FRAME)
                } else {
                    (0x0060_a750, fx::LIST_FRAME)
                };
                t.skip = 0x1c;
                let what = format!("{sym:#x} #{case} count {count} lists {lists} made {made}");
                t.check_framed(&what, sym, Conv::Cdecl, &args, low, |m, ebp| {
                    if array {
                        fx::unpack_array_list(m, ebp, cursor, records, slots + 4, n, count);
                    } else {
                        fx::unpack_list(m, ebp, cursor, records, slots + 4, n, count);
                    }
                    Out::None
                });
                t.skip = 0;
                let x: u32 = Mem::read(&t.a, slots + 4);
                let y: u32 = Mem::read(&t.b, slots + 4);
                assert_eq!(x, y, "{what}: the records slot");
            }
        }
    });
}

// ---- round 3 (ffx_system::m5a_files) ----

/// Code that copies `len` bytes from `src` to the pointer in stack word
/// `arg` (as the caller pushed it), then jumps to `next` (a stub from
/// [`Twins::stub_code_writes`] or [`Twins::stub_code`]). Returns its
/// address.
fn write_then(t: &mut Twins, arg: u32, src: u32, len: u32, next: u32) -> u32 {
    let mut c = vec![0x51, 0x56, 0x57]; // push ecx; push esi; push edi
    c.extend([0x8b, 0x7c, 0x24, (0x10 + 4 * arg) as u8]); // mov edi, [esp+0x10+4arg]
    c.push(0xb9);
    c.extend(len.to_le_bytes()); // mov ecx, len
    c.push(0xbe);
    c.extend(src.to_le_bytes()); // mov esi, src
    c.extend([0xf3, 0xa4]); // rep movsb
    c.extend([0x5f, 0x5e, 0x59]); // pop edi; pop esi; pop ecx
    c.push(0xe9);
    let here = t.cave_next + c.len() as u32 + 4;
    c.extend(next.wrapping_sub(here).to_le_bytes()); // jmp next
    t.cave_code(&c)
}

/// The member caches `0x0057a200` and `Fixup_FindMember`: hits, misses
/// (the lookup found or not, moving the cursor and writing the element),
/// indices inside and outside the class's member table.
#[test]
fn files_member_caches_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a30_0301);
        for case in 0..80 {
            t.reset();
            let small = |rng: &mut Rng| rng.below(3);
            let class = t.words(&[0, 0, 0, 0, rng.below(6)]);
            let members = if rng.chance(5) { 0 } else { rng.next() & !0x1f };
            let view = t.words(&[class, 0, 0, 0, 0, 0, members]);
            let cache: Vec<u32> = (0..9)
                .map(|k| match k {
                    0 => view,
                    1..=3 => small(&mut rng),
                    _ => rng.next(),
                })
                .collect();
            let cache = t.words(&cache);
            let cursor = t.words(&[rng.next()]);
            let moved = t.words(&[rng.next()]);
            let element = t.words(&[rng.next()]);
            let elem = t.words(&[rng.next()]);
            let found = if rng.chance(3) { 0 } else { rng.next() };
            // the lookups move the cursor (arg 1) and the second writes the
            // element (arg 3)
            let next = t.stub_code(fl::FIND_MEMBER, true, 4, 4, found);
            let at = write_then(t, 1, moved, 4, next);
            t.jump(fl::FIND_MEMBER, at);
            let found2 = if rng.chance(3) { 0 } else { rng.next() };
            let next = t.stub_code(fl::FIND_MEMBER_BY_OFFSET, true, 4, 4, found2);
            let next = write_then(t, 3, elem, 4, next);
            let at = write_then(t, 1, moved, 4, next);
            t.jump(fl::FIND_MEMBER_BY_OFFSET, at);
            let (a, b, c) = (small(&mut rng), small(&mut rng), small(&mut rng));
            let what = format!("0x0057a200 #{case} {a} {b} {c}");
            t.check(
                &what,
                0x0057_a200,
                Conv::Thiscall(cache),
                &[cursor, a, b, c],
                |m| Out::U32(fl::cached_find_member(m, cache, cursor, a, b, c)),
            );
            let index = [rng.below(8), 0xffff_ffff, rng.next()][rng.below(3) as usize];
            let what = format!("Fixup_FindMember #{case} index {index:#x} {a} {b}");
            t.check(
                &what,
                0x0057_a280,
                Conv::Thiscall(cache),
                &[index, cursor, a, element, b],
                |m| {
                    Out::U32(fl::fixup_find_member(
                        m, cache, index, cursor, a, element, b,
                    ))
                },
            );
        }
    });
}

/// The sort predicates `0x0060ac50` and `0x0060ad00` on records of small
/// words (ties down to the last field) and top bits set or not; the tree
/// set destructors `0x0060d460`, `0x0060d550`, `0x0060d5d0` (array owned
/// or not, set or not), `VFile_VFile` (the record allocation failing or
/// not) and `0x0061b030` (each record and block there or not).
#[test]
fn files_small_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a30_0302);
        for case in 0..300 {
            t.reset();
            let word = |rng: &mut Rng| rng.below(2) | (rng.below(2) << 31);
            let a: Vec<u32> = (0..7).map(|_| word(&mut rng)).collect();
            // b a copy of a with two fields changed (late ties), or
            // anything
            let mut b = a.clone();
            for _ in 0..2 {
                let k = rng.below(7) as usize;
                b[k] = word(&mut rng);
            }
            if rng.chance(3) {
                b = (0..7).map(|_| word(&mut rng)).collect();
            }
            let (a, b) = (t.words(&a), t.words(&b));
            for (sym, fields) in [
                (0x0060_ac50u32, &fl::LESS_60AC50[..]),
                (0x0060_ad00, &fl::LESS_60AD00[..]),
            ] {
                let what = format!("{sym:#x} #{case}");
                t.check(&what, sym, Conv::Cdecl, &[a, b], |m| {
                    Out::U32(fl::record_less(m, fields, a, b))
                });
            }
            t.stubs(&[
                (fl::TREE_CLEAR, true, 0, 0, rng.next()),
                (fl::TREE_ORPHAN_ALL, true, 0, 0, rng.next()),
                (fl::FREE_LIST_RESET, true, 0, 0, rng.next()),
                (fl::FREE_LIST_DTOR, true, 0, 0, rng.next()),
                (fl::PHYRE_FREE, false, 1, 0, rng.next()),
                (fl::DELETE_ARRAY, false, 1, 0, rng.next()),
                (fl::OPERATOR_DELETE, false, 1, 0, rng.next()),
            ]);
            let mut w: Vec<u32> = (0..0x20).map(|_| rng.next()).collect();
            w[3] = match rng.below(3) {
                0 => rng.below(0x100),
                1 => 0x4000_0000 | rng.below(0x100),
                _ => 0x8000_0000 | rng.next(),
            };
            w[4] = if rng.chance(4) { 0 } else { rng.next() };
            let this = t.words(&w);
            for (k, sym) in [0x0060_d460u32, 0x0060_d550, 0x0060_d5d0]
                .into_iter()
                .enumerate()
            {
                let what = format!("{sym:#x} #{case}");
                t.check(&what, sym, Conv::Thiscall(this), &[], |m| {
                    match k {
                        0 => fl::tree_dtor(m, this),
                        1 => fl::pooled_tree_dtor(m, this),
                        _ => fl::owned_array_dtor(m, this),
                    }
                    Out::None
                });
            }
            let rec = if rng.chance(4) {
                0
            } else {
                t.words(&(0..6).map(|_| rng.next() | 1).collect::<Vec<_>>())
            };
            let block = rng.next();
            t.stub(fl::OPERATOR_NEW, false, 1, 0, rec);
            t.stub(fl::NEW_ARRAY, false, 1, 0, block);
            let vfile = t.words(&(0..8).map(|_| rng.next()).collect::<Vec<_>>());
            let what = format!("VFile_VFile #{case}");
            t.check(&what, 0x0061_a860, Conv::Thiscall(vfile), &[], |m| {
                Out::U32(fl::vfile_ctor(m, vfile))
            });
            let mut recs = Vec::new();
            for _ in 0..3 {
                let r = if rng.chance(4) {
                    0
                } else {
                    let blk = if rng.chance(3) { 0 } else { rng.next() };
                    t.words(&[rng.next(), blk])
                };
                recs.push(r);
            }
            let v = t.words(&[0, 0, recs[0], recs[1], recs[2]]);
            let what = format!("0x0061b030 #{case}");
            t.check(&what, 0x0061_b030, Conv::Thiscall(v), &[], |m| {
                fl::vfile_free_buffers(m, v);
                Out::None
            });
        }
    });
}

/// `0x0061a690` (the world-matrix controller's constructor, the real
/// `PMatrix4_toWorldMatrix3x4` on the identity in its frame) and the
/// controller's `0x0061a770` (the real `PMatrix4_FromMatrix3x4` into its
/// frame, then the normalised axis): random finite 3x4 matrices, zero
/// axes (an infinite inverse, NaN products) now and then.
#[test]
fn files_world_matrix_controller_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a30_0303);
        for case in 0..60 {
            t.reset();
            let this = t.alloc(&(0..0x80).map(|_| rng.next() as u8).collect::<Vec<_>>());
            let what = format!("0x0061a690 #{case}");
            t.check_framed(
                &what,
                0x0061_a690,
                Conv::Thiscall(this),
                &[],
                fl::MATRIX_FRAME,
                |m, ebp| Out::U32(fl::controller_ctor(m, ebp, this)),
            );
            let zero = rng.chance(6);
            for k in 0..12u32 {
                let v = if zero && (8..11).contains(&k) {
                    0.0f32
                } else {
                    f32::from(rng.next() as i16) / 16.0
                };
                t.write(this + 4 + 4 * k, &v.to_le_bytes());
            }
            let out = t.zeros(12);
            // the original's float temporary at ebp - 4 has no counterpart
            t.skip = 4;
            let what = format!("0x0061a770 #{case} zero {zero}");
            t.check_framed(
                &what,
                0x0061_a770,
                Conv::Thiscall(this),
                &[out],
                fl::AXIS_FRAME,
                |m, ebp| Out::U32(fl::controller_axis(m, ebp, this, out)),
            );
            t.skip = 0;
        }
    });
}

/// `0x0061b270` (no module, a module without the block, the second
/// module's string), `0x0061b740` (offsets 0, inside and past the first
/// block, 64-bit ones; totals around the block size) and
/// `FileSys_NewReadRequest` (offsets of 0..4 blocks, packed sizes of 0
/// and others, 64-bit stored offsets), the callees stubbed.
#[test]
#[allow(clippy::too_many_lines)] // three functions, one case setup
fn files_read_requests_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a30_0304);
        for case in 0..80 {
            t.reset();
            let mods: Vec<u32> = (0..4)
                .map(|k| if k >= rng.below(4) { 0 } else { rng.next() | 1 })
                .collect();
            stub_seq(t, fl::RESOURCE_MODULE, true, 1, 1, &mods);
            let infos: Vec<u32> = (0..4)
                .map(|_| if rng.chance(2) { 0 } else { rng.next() })
                .collect();
            t.stub_import_seq(fl::FIND_RESOURCE_IAT, 3, 3, &infos);
            let strs: Vec<u32> = (0..4)
                .map(|_| if rng.chance(3) { 0 } else { rng.next() })
                .collect();
            stub_seq(t, fl::STRING_RESOURCE, false, 3, 0, &strs);
            let id = [rng.next() & 0xf_ffff, 0xffff0 | rng.below(0x10), rng.next()]
                [rng.below(3) as usize];
            let what = format!("0x0061b270 #{case}");
            t.check(&what, 0x0061_b270, Conv::Cdecl, &[id], |m| {
                Out::U32(fl::find_string_resource(m, id))
            });
            // 0x0061b740
            t.reset();
            let recs: Vec<u32> = (0..3).map(|_| t.zeros(0x10)).collect();
            let mut slot: Vec<u32> = (0..0x18).map(|_| rng.next()).collect();
            slot[2] = recs[0];
            slot[3] = recs[1];
            slot[4] = recs[2];
            slot[9] = rng.below(5);
            let slot = t.words(&slot);
            t.stub(fl::CLAIM_SLOT, true, 1, 1, slot);
            let size = [0x10000, 0x10, 1, 0x7fff_ffff, 0xffff_ff00][rng.below(5) as usize];
            let lo = [0, 1, size / 2, size, size + 1, rng.next()][rng.below(6) as usize];
            let hi = [0, 0, 0, 1, 0xffff_ffff][rng.below(5) as usize];
            let total = [
                0,
                1,
                size - 1,
                size,
                size + 1,
                size.wrapping_mul(3).wrapping_add(5),
                rng.next(),
            ][rng.below(7) as usize];
            let stream = rng.next();
            let what =
                format!("0x0061b740 #{case} lo {lo:#x} hi {hi:#x} size {size:#x} total {total:#x}");
            t.check(
                &what,
                0x0061_b740,
                Conv::Thiscall(stream),
                &[lo, hi, size, total],
                |m| Out::U32(fl::set_up_slot(m, stream, lo, hi, size, total)),
            );
            // FileSys_NewReadRequest
            t.reset();
            let slot = t.zeros(0x60);
            t.stub(fl::SET_UP_SLOT, true, 4, 4, slot);
            let sizes: Vec<u8> = (0..0x20)
                .flat_map(|_| {
                    let n = if rng.chance(3) {
                        0
                    } else {
                        rng.below(0x10000) as u16
                    };
                    n.to_le_bytes()
                })
                .collect();
            let sizes = t.alloc(&sizes);
            let info = t.zeros(0x40);
            t.w32(info + 0x38, sizes);
            let entry = t.words(&[
                rng.below(8),
                0,
                0,
                0,
                rng.next(),
                [0, 1, 0xffff_ffff][rng.below(3) as usize],
            ]);
            let req = t.words(&[0, rng.next(), 0, entry, 0, 0, rng.next()]);
            let lo =
                rng.below(5) * 0x10000 + [0, 1, 0xffff, rng.below(0x10000)][rng.below(4) as usize];
            let hi = 0;
            let (file, length) = (rng.next(), rng.next());
            let what = format!("FileSys_NewReadRequest #{case} lo {lo:#x}");
            t.check(
                &what,
                0x0061_b7e0,
                Conv::Thiscall(stream),
                &[file, length, req, lo, hi, info],
                |m| {
                    Out::U32(fl::new_read_request(
                        m, stream, file, length, req, lo, hi, info,
                    ))
                },
            );
        }
    });
}

/// `0x00573b70` (the Lua vector negation): not userdata, the wrong type,
/// a vector (signs, zeros, infinities, quiet NaNs), the Lua API stubbed.
#[test]
fn files_lua_vector_negate_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a30_0305);
        for case in 0..60 {
            t.reset();
            let comps: Vec<u32> = (0..4)
                .map(|_| match rng.below(6) {
                    0 => 0,
                    1 => 0x8000_0000,
                    2 => 0x7f80_0000,
                    3 => 0xffc0_0001,
                    _ => rng.next() & 0xbfff_ffff,
                })
                .collect();
            let v = t.words(&comps);
            let ud = t.zeros(0x10);
            t.stubs(&[
                (
                    fl::LUA_TOUSERDATA,
                    false,
                    2,
                    0,
                    if rng.chance(5) { 0 } else { v },
                ),
                (fl::LUA_IS_USERDATA, false, 2, 0, u32::from(!rng.chance(5))),
                (fl::LUA_GETFIELD, false, 3, 0, rng.next()),
                (fl::LUA_COMPARE, false, 3, 0, u32::from(!rng.chance(4))),
                (fl::LUA_SETTOP, false, 2, 0, rng.next()),
                (fl::LUA_NEWUSERDATA, false, 2, 0, ud),
                (fl::LUA_SETMETATABLE, false, 2, 0, rng.next()),
            ]);
            let lua = rng.next();
            let what = format!("0x00573b70 #{case}");
            t.check(&what, 0x0057_3b70, Conv::Cdecl, &[lua], |m| {
                Out::U32(fl::lua_vector_negate(m, lua))
            });
        }
    });
}

// ---- round 3 (ffx_system::m5a_vfs) ----

/// `FileSys_Exists` and `BigFileStream_openFile` (framed: the hash size,
/// handle and digest the `CryptoAPI` stubs fill): no stream, archives set
/// or not, the name found in none, the first or a later one;
/// `BigFileStream_readVFile` (no request, done or not, the slot's last
/// use or not) and `0x0061d9c0` (a free inline file, none free with the
/// allocation and construction failing or not), the callees stubbed.
#[test]
#[allow(clippy::too_many_lines)] // four functions, one case setup
fn vfs_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a30_0401);
        for case in 0..60 {
            t.reset();
            t.g32(0x00cc_9c48, if rng.chance(6) { 0 } else { rng.next() });
            let handle = t.words(&[rng.next()]);
            let r = rng.next();
            let s = t.stub_code_writes(vf::CRYPT_CREATE_HASH_IAT, false, 5, 5, r, 4, handle, 4);
            let at = t.rebase(vf::CRYPT_CREATE_HASH_IAT);
            t.patch(at, &s.to_le_bytes());
            t.stub_import(vf::CRYPT_HASH_DATA_IAT, false, 4, 4, rng.next());
            let md5 = t.alloc(&(0..16).map(|_| rng.next() as u8).collect::<Vec<_>>());
            let r = rng.next();
            let s = t.stub_code_writes(vf::CRYPT_GET_HASH_PARAM_IAT, false, 5, 5, r, 2, md5, 16);
            let at = t.rebase(vf::CRYPT_GET_HASH_PARAM_IAT);
            t.patch(at, &s.to_le_bytes());
            t.stub_import(vf::CRYPT_DESTROY_HASH_IAT, false, 1, 1, rng.next());
            let hits: Vec<u32> = (0..5)
                .map(|_| {
                    if rng.chance(3) {
                        rng.next() | 1
                    } else {
                        rng.next() & !0xff
                    }
                })
                .collect();
            stub_seq(t, vf::VBF_CONTAINS, true, 1, 1, &hits);
            let files: Vec<u32> = (0..5)
                .map(|_| if rng.chance(2) { 0 } else { rng.next() })
                .collect();
            stub_seq(t, vf::GET_NEW_VFILE, true, 1, 1, &files);
            let mut w: Vec<u32> = (0..0x10).map(|_| rng.next()).collect();
            w[0xf] = rng.below(4);
            for k in 0..5 {
                w[4 + k] = if rng.chance(4) { 0 } else { rng.next() };
            }
            let this = t.words(&w);
            let name: Vec<u8> = (0..rng.below(12) + 4)
                .map(|_| b'a' + rng.below(26) as u8)
                .chain([0])
                .collect();
            let name = t.alloc(&name);
            t.skip = 4;
            let what = format!("FileSys_Exists #{case}");
            t.check_framed(
                &what,
                0x0061_be40,
                Conv::Thiscall(this),
                &[name],
                vf::DIGEST_FRAME,
                |m, ebp| Out::U8(vf::exists(m, ebp, this, name)),
            );
            let what = format!("BigFileStream_openFile #{case}");
            t.check_framed(
                &what,
                0x0061_bf10,
                Conv::Thiscall(this),
                &[name],
                vf::DIGEST_FRAME,
                |m, ebp| Out::U32(vf::open_file(m, ebp, this, name)),
            );
            t.skip = 0;
            // readVFile
            t.reset();
            let mut sw: Vec<u32> = (0..0x20).map(|_| rng.next()).collect();
            sw[0x14] = if rng.chance(2) { 5 } else { rng.below(8) };
            sw[9] = 1 + rng.below(2);
            let slot = t.words(&sw);
            t.stubs(&[
                (
                    vf::NEW_READ_REQUEST,
                    true,
                    6,
                    6,
                    if rng.chance(5) { 0 } else { slot },
                ),
                (vf::TASK_READ, false, 3, 3, rng.next()),
                (vf::TASK_UNCOMPRESS, false, 4, 4, rng.next()),
                (vf::SLOT_RESET, true, 0, 0, rng.next()),
            ]);
            t.stub_import(vf::ENTER_CS_IAT, false, 1, 1, rng.next());
            t.stub_import(vf::LEAVE_CS_IAT, false, 1, 1, rng.next());
            t.stub_import(vf::RELEASE_SEMAPHORE_IAT, false, 3, 3, rng.next());
            let a: Vec<u32> = (0..6).map(|_| rng.next()).collect();
            let what = format!("BigFileStream_readVFile #{case}");
            t.check(&what, 0x0061_bfe0, Conv::Thiscall(this), &a, |m| {
                Out::U32(vf::read_vfile(m, this, a[0], a[1], a[2], a[3], a[4], a[5]))
            });
            // 0x0061d9c0
            let mut inline = vec![0u8; 15 * 0x20];
            let used = if rng.chance(3) { 15 } else { rng.below(15) };
            for i in 0..15 {
                inline[i * 0x20] = if i < used as usize || rng.chance(3) {
                    1 + rng.below(255) as u8
                } else {
                    0
                };
            }
            let inline = t.alloc(&inline);
            let mut hw: Vec<u32> = (0..0x18).map(|_| rng.next()).collect();
            hw[0x17] = inline;
            let handle_obj = t.words(&hw);
            let p = if rng.chance(4) { 0 } else { t.zeros(0x20) };
            let made = if rng.chance(4) { 0 } else { p };
            t.stubs(&[
                (vf::VFILE_ASSIGN, true, 1, 1, rng.next()),
                (vf::NEW_NOTHROW, false, 2, 0, p),
                (vf::VFILE_CTOR, true, 0, 0, made),
                (vf::VBF_STREAM_OPEN, true, 2, 2, rng.next()),
            ]);
            let digest = rng.next();
            let what = format!("0x0061d9c0 #{case} used {used}");
            t.check(
                &what,
                0x0061_d9c0,
                Conv::Thiscall(handle_obj),
                &[digest],
                |m| Out::U32(vf::new_vfile(m, handle_obj, digest)),
            );
        }
    });
}

/// `BigFileStream_fileTaskRead` (head, body and tail records flagged or
/// not; bodies of 1..4 blocks; packed sizes of 0 and others) and the slot
/// constructor `0x0061add0`, the CRT and allocator stubbed.
#[test]
fn vfs_task_read_and_slot_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a30_0402);
        for case in 0..80 {
            t.reset();
            t.stub_import(vf::FSEEKI64_IAT, false, 4, 0, rng.next());
            t.stub_import(vf::FREAD_IAT, false, 4, 0, rng.next());
            let sizes: Vec<u8> = (0..0x20)
                .flat_map(|_| {
                    let n = if rng.chance(3) {
                        0
                    } else {
                        rng.below(0x10000) as u16
                    };
                    n.to_le_bytes()
                })
                .collect();
            let sizes = t.alloc(&sizes);
            let info = t.zeros(0x40);
            t.w32(info + 0x38, sizes);
            let mut recs = Vec::new();
            for _ in 0..3 {
                let flag = u32::from(rng.chance(2));
                recs.push(t.words(&[flag, rng.next(), rng.below(0x10000), rng.next()]));
            }
            let mut sw: Vec<u32> = (0..0x18).map(|_| rng.next()).collect();
            sw[1] = rng.below(8);
            sw[2] = recs[0];
            sw[3] = recs[1];
            sw[4] = recs[2];
            sw[0xe] = rng.below(5) * 0x10000 + rng.below(0x10000);
            sw[0x10] = info;
            let slot = t.words(&sw);
            let what = format!("fileTaskRead #{case} len {:#x}", sw[0xe]);
            let (a, c) = (rng.next(), rng.next());
            t.check(&what, 0x0061_b8d0, Conv::Stdcall, &[a, slot, c], |m| {
                vf::task_read(m, slot);
                Out::None
            });
            // the slot constructor
            t.reset();
            let rec = t.words(&[rng.next() | 1, rng.next(), 0, 0]);
            t.stubs(&[
                (vf::OPERATOR_NEW, false, 1, 0, rec),
                (vf::NEW_ARRAY, false, 1, 0, rng.next()),
                (vf::OPERATOR_DELETE, false, 1, 0, rng.next()),
            ]);
            let this = t.words(&(0..0x18).map(|_| rng.next()).collect::<Vec<_>>());
            let what = format!("0x0061add0 #{case}");
            t.check(&what, 0x0061_add0, Conv::Thiscall(this), &[], |m| {
                Out::U32(vf::slot_ctor(m, this))
            });
        }
    });
}
