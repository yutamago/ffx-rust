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
use ffx_system::m5a_savedata::{self as sv, bridge as sa};
use ffx_system::m5a_settings::{self as settings, bridge as st};
use ffx_system::m5a_std::{self as sd, bridge as sb};

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
        let ebp = self.frame_ebp(args.len());
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
