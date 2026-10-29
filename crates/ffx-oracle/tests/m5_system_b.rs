//! M5 stage 1, list `system-b` (`ffx_system::m5b_*`) against the
//! original: on the image alone with synthetic objects, and on the
//! snapshots (`FFX_SNAPSHOT`) over the game's memory. Results and every
//! byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! These functions never ran in a coverage run (the crash reporter, the
//! loader thread's set-up and teardown, the request pools' edge cases), so
//! every branch is reached with synthetic objects; the snapshots run the
//! same cases over the game's memory. Callees that are still original
//! (the allocators, the CRT and Win32 imports, the PS2 kernel stand-ins,
//! virtual methods of fake objects) are replaced in both oracles alike by
//! recording stubs: each call appends its tag, `ecx` and its arguments to
//! a log in memory, so the diff checks that both made the same calls in
//! the same order. Imports are stubbed through their import table slots;
//! `memcpy`/`memmove` stubs copy and log only the destination, since the
//! source is the original's own stack.

// the shared harness (as in m5_system_a.rs) keeps helpers later batches use
#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_system::m5b_async::{self as al, bridge as ab};
use ffx_system::m5b_misc::{self as ms, bridge as mb};
use ffx_system::m5b_stackwalker::{self as swk, bridge as sw};
use ffx_system::m5b_steam::{self as steam, bridge as sm};
use ffx_system::m5b_stl::{self as stl, bridge as st};
use ffx_system::m5b_sync::{self as sy, bridge as sb3};

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
    fn call_wide(&mut self, addr: u32, args: &[u32]) -> u32 {
        ffx_oracle::check_width("Host::call_wide", addr, args.len(), host::WIDE_WORDS);
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        self.0.call(addr, Conv::Cdecl, args).unwrap().eax
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

    /// A recording stub over `sym` (`n` words, `pops`) returning `seq[k]`
    /// on its `k`-th call.
    fn stub_seq(&mut self, sym: u32, n: u32, pops: u32, seq: &[u32]) {
        let ctr = self.words(&[0]);
        let tbl = self.words(seq);
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], tag
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
        self.jump(sym, addr);
    }

    /// A copying `memcpy(dst, src, n)` stub (cdecl): copies `n` bytes and
    /// logs only the destination (tag `tag`). Returns its address.
    fn copy_stub(&mut self, tag: u32) -> u32 {
        let log_stub = self.stub_code(tag, false, 1, 0, 0);
        let mut c = vec![0x56, 0x57]; // push esi; push edi
        c.extend([0x8b, 0x7c, 0x24, 0x0c]); // mov edi, [esp+0xc] (dst)
        c.extend([0x8b, 0x74, 0x24, 0x10]); // mov esi, [esp+0x10] (src)
        c.extend([0x8b, 0x4c, 0x24, 0x14]); // mov ecx, [esp+0x14] (n)
        c.extend([0xf3, 0xa4]); // rep movsb
        c.extend([0x5f, 0x5e]); // pop edi; pop esi
        c.push(0xe9);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(log_stub.wrapping_sub(here).to_le_bytes()); // jmp log_stub
        self.cave_code(&c)
    }

    /// [`Twins::copy_stub`] over the original at `sym`.
    fn stub_copy(&mut self, sym: u32) {
        let addr = self.copy_stub(sym);
        self.jump(sym, addr);
    }

    /// [`Twins::copy_stub`] bound to the import slot `slot`.
    fn stub_import_copy(&mut self, slot: u32) {
        let addr = self.copy_stub(slot);
        let at = self.rebase(slot);
        self.patch(at, &addr.to_le_bytes());
    }

    /// A recording stub over `sym` (cdecl) logging `first` words on its
    /// first call and `rest` words on the later ones.
    fn stub_two(&mut self, sym: u32, first: u32, rest: u32, eax: u32) {
        let a = self.stub_code(sym, false, first, 0, eax);
        let b = self.stub_code(sym, false, rest, 0, eax);
        let ctr = self.words(&[0]);
        let mut c = vec![0xa1];
        c.extend(ctr.to_le_bytes()); // mov eax, [ctr]
        c.extend([0xff, 0x05]);
        c.extend(ctr.to_le_bytes()); // inc dword [ctr]
        c.extend([0x85, 0xc0]); // test eax, eax
        c.extend([0x0f, 0x84]);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(a.wrapping_sub(here).to_le_bytes()); // je a
        c.push(0xe9);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(b.wrapping_sub(here).to_le_bytes()); // jmp b
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }

    /// Writes `w` at `at` in both oracles.
    fn put(&mut self, at: u32, w: &[u32]) {
        let bytes: Vec<u8> = w.iter().flat_map(|v| v.to_le_bytes()).collect();
        self.write(at, &bytes);
    }

    /// Scratch on `b`'s stack (outside the diff) for what the original
    /// keeps on its own stack.
    fn b_stack(&self, off: u32) -> u32 {
        self.b.stack_range().start + off
    }
}

/// Stubs an import slot that takes `n` words, popping them when
/// `stdcall`.
fn imp(t: &mut Twins, slot: u32, n: u32, stdcall: bool, ret: u32) {
    t.stub_import(slot, false, n, if stdcall { n } else { 0 }, ret);
}

// ---- StackWalker ----

#[test]
fn stack_walker_ctor_matches_the_original() {
    Twins::each(|t| {
        for case in 0..4u32 {
            t.reset();
            let helper = if case & 1 == 0 { 0 } else { t.zeros(0x44) };
            t.stub(swk::OPERATOR_NEW, false, 1, 0, helper);
            t.stub(swk::INIT_HELPER, true, 2, 2, 0x4444_0000);
            imp(t, swk::STRDUP_IAT, 1, false, 0x5555_0000);
            let this = t.words(&[0x1111_1111; 8]);
            let path = if case & 2 == 0 {
                0
            } else {
                t.alloc(b"C:\\sym\0")
            };
            let what = format!("StackWalker ctor #{case}");
            t.check(
                &what,
                0x0068_70b0,
                Conv::Thiscall(this),
                &[0x2f, path, 77, 0x99],
                |m| Out::U32(sw::construct(m, this, 0x2f, path, 77, 0x99)),
            );
        }
    });
}

#[test]
fn stack_walker_dtors_match_the_original() {
    Twins::each(|t| {
        for case in 0..32u32 {
            t.reset();
            t.stub(swk::OPERATOR_DELETE, false, 1, 0, 0x1d);
            imp(t, swk::FREE_IAT, 1, false, 0x1e);
            imp(t, swk::FREE_LIBRARY_IAT, 1, true, 0x1f);
            let cleanup = t.stub_code(0xc1ea, false, 1, 1, 0x20);
            let helper = t.words(&[
                0x77,
                if case & 2 == 0 { 0 } else { 0x0bad_0001 },
                0x4242,
                if case & 4 == 0 { 0 } else { 0x0bad_0002 },
                if case & 8 == 0 { 0 } else { cleanup },
            ]);
            let this = t.words(&[
                0,
                if case & 1 == 0 { 0 } else { helper },
                1,
                2,
                3,
                if case & 16 == 0 { 0 } else { 0x0bad_0003 },
                5,
                6,
            ]);
            let flags = case % 3;
            let what = format!("StackWalker dtor #{case}");
            t.check(&what, 0x0068_7210, Conv::Thiscall(this), &[], |m| {
                sw::destruct(m, this);
                Out::None
            });
            let what = format!("StackWalker deleting dtor #{case}");
            t.check(&what, 0x0068_72e0, Conv::Thiscall(this), &[flags], |m| {
                Out::U32(sw::deleting_dtor(m, this, flags))
            });
        }
    });
}

/// Fills a 1 KB string field with `len` letters (no NUL when `len` is
/// 0x400 or more: the string runs into the next field).
fn name(t: &mut Twins, at: u32, len: usize, c: u8) {
    let mut b = vec![c; len.min(0x400)];
    if len < 0x400 {
        b.push(0);
    }
    t.write(at, &b);
}

#[test]
fn stack_walker_callstack_entry_matches_the_original() {
    Twins::each(|t| {
        t.skip = 4;
        let mut case = 0;
        for kind in [0u32, 1, 2] {
            for offset in [0u64, 0x0040_1234, 0x1_0000_0000] {
                for names in 0..16u32 {
                    case += 1;
                    if kind == 2 && names > 0 || offset == 0 && names > 1 {
                        continue;
                    }
                    t.reset();
                    t.stub_import(swk::STRCPY_S_IAT, false, 3, 0, 0);
                    t.stub_import(swk::STRNCPY_S_IAT, false, 4, 0, 0);
                    // the file-name line passes one word less
                    let words = if names & 4 == 0 { 7 } else { 6 };
                    t.stub(swk::VSNPRINTF_1K, false, words, 0, 0x30);
                    let this = t.object(&[(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (1, 0x61)]);
                    let entry = t.zeros(0x1440);
                    t.put(entry, &[offset as u32, (offset >> 32) as u32]);
                    let lens = [0usize, 3, 0x3ff, 0x400];
                    name(t, entry + 8, lens[(names % 4) as usize], b'n');
                    name(t, entry + 0x408, lens[((names / 2) % 4) as usize], b'u');
                    name(t, entry + 0x808, lens[((names + 1) % 4) as usize], b'f');
                    t.put(entry + 0xc14, &[42]);
                    name(t, entry + 0xc18, if names & 4 == 0 { 0 } else { 5 }, b'l');
                    name(t, entry + 0x1020, if names & 8 == 0 { 0 } else { 6 }, b'm');
                    let what = format!("OnCallstackEntry #{case}");
                    let fill = vec![0xcc; swk::BUFFER as usize];
                    t.check_framed_init(
                        &what,
                        0x0068_82a0,
                        Conv::Thiscall(this),
                        &[kind, entry],
                        swk::BUFFER,
                        &fill,
                        |m, ebp| {
                            sw::on_callstack_entry(m, ebp, this, kind, entry);
                            Out::None
                        },
                    );
                }
            }
        }
        t.skip = 0;
    });
}

#[test]
fn stack_walker_printers_match_the_original() {
    Twins::each(|t| {
        t.skip = 4;
        for case in 0..6u32 {
            t.reset();
            t.stub(swk::VSNPRINTF_1K, false, 6, 0, 0x30);
            let this = t.object(&[(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (1, 0x61)]);
            let what = format!("OnDbgHelpErr #{case}");
            let args = [0x00b4_6b3c, case, 0x1234_0000 + case, 7];
            t.check_framed(
                &what,
                0x0068_8470,
                Conv::Thiscall(this),
                &args,
                swk::BUFFER,
                |m, ebp| {
                    sw::on_dbghelp_err(m, ebp, this, args[0], args[1], args[2], args[3]);
                    Out::None
                },
            );
            let ver = [
                (0u32, 0u32),
                (1, 0),
                (0, 1),
                (0x0005_0006, 0x0007_0008),
                (0xffff_ffff, 0x8000_0001),
                (0x10000, 0),
            ][case as usize];
            let w = [1, 2, 0x0040_0000, 3, 0x0010_0000, 1, 5, 6, ver.0, ver.1];
            let module = sw::Module {
                image: w[0],
                module: w[1],
                base_lo: w[2],
                base_hi: w[3],
                size: w[4],
                result: w[5],
                sym_type: w[6],
                pdb: w[7],
                version_lo: w[8],
                version_hi: w[9],
            };
            t.reset();
            let words = if ver == (0, 0) { 10 } else { 14 };
            t.stub(swk::VSNPRINTF_1K, false, words, 0, 0x30);
            let this = t.object(&[(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (1, 0x61)]);
            let what = format!("OnLoadModule #{case}");
            t.check_framed(
                &what,
                0x0068_84e0,
                Conv::Thiscall(this),
                &w,
                swk::BUFFER,
                |m, ebp| {
                    sw::on_load_module(m, ebp, this, &module);
                    Out::None
                },
            );
        }
        // OnSymInit: GetVersionExA fails or succeeds; the info's fields
        // are the frame's (the memset stub clears nothing)
        for ok in [0u32, 1, 0x100] {
            t.reset();
            t.stub_two(swk::VSNPRINTF_1K, 6, 9, 0x30);
            t.stub(swk::MEMSET, false, 3, 0, 0);
            imp(t, swk::GET_VERSION_EX_IAT, 1, true, ok);
            let this = t.object(&[(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (1, 0x61)]);
            let init: Vec<u8> = (0..0x9c).map(|i| (i * 7 + 3) as u8).collect();
            let what = format!("OnSymInit {ok}");
            t.check_framed_init(
                &what,
                0x0068_85f0,
                Conv::Thiscall(this),
                &[0x11, 0x22, 0x33],
                swk::VERSION_INFO,
                &init,
                |m, ebp| {
                    sw::on_sym_init(m, ebp, this, 0x11, 0x22, 0x33);
                    Out::None
                },
            );
        }
        t.skip = 0;
    });
}

#[test]
fn esc_menu_dtor_matches_the_original() {
    Twins::each(|t| {
        t.reset();
        t.stub(ms::REMOVE_LISTENER, false, 1, 0, 0x12);
        t.stub(ms::IGGY_MENU_DTOR, true, 0, 0, 0x31);
        let this = t.zeros(0xa0);
        t.check(
            "FFEscMenu dtor",
            0x0068_e1f0,
            Conv::Thiscall(this),
            &[],
            |m| Out::U32(mb::esc_menu_destruct(m, this)),
        );
    });
}

// ---- AsyncLoadManager ----

/// The pool, list and request callees as recording stubs.
fn async_stubs(t: &mut Twins, block: u32) {
    t.stub(al::USER_MALLOC, false, 1, 0, block);
    t.stub(al::USER_FREE, false, 1, 0, 0x16);
    t.stub_copy(al::MEMCPY);
    t.stub(al::MEMSET, false, 3, 0, 0);
    t.stub(al::OPERATOR_NEW, false, 1, 0, block);
    t.stub(al::OPERATOR_DELETE, false, 1, 0, 0x17);
    t.stub(al::NEW_ARRAY, false, 1, 0, block);
    t.stub(al::DELETE_ARRAY, false, 1, 0, 0x18);
    imp(t, al::CREATE_MUTEX_IAT, 3, true, 0x00d0_0001);
    imp(t, al::CLOSE_HANDLE_IAT, 1, true, 0x77);
    imp(t, al::WAIT_IAT, 2, true, 0);
    imp(t, al::RELEASE_MUTEX_IAT, 1, true, 0x78);
    t.stub(al::SIGNAL_SEMA, false, 1, 0, 0x79);
    t.stub(al::WAIT_SEMA, false, 1, 0, 1);
}

#[test]
fn list_reserve_and_ctor_match_the_original() {
    Twins::each(|t| {
        for case in 0..24u32 {
            for sym in [0x0069_1770u32, 0x0069_17f0] {
                t.reset();
                let block = if case % 3 == 0 { 0 } else { t.zeros(0x100) };
                async_stubs(t, block);
                let old = t.words(&[1, 2, 3, 4, 5, 6]);
                let list = t.words(&[case % 5, 3, if case & 4 == 0 { 0 } else { old }]);
                let cap = [0u32, 1, 6, 0xffff_ffff, 0x8000_0000, 2][(case / 4) as usize];
                let what = format!("list reserve {sym:#x} #{case}");
                t.check(&what, sym, Conv::Thiscall(list), &[cap], |m| {
                    ab::list_reserve(m, list, cap);
                    Out::None
                });
            }
            let list = t.words(&[9, 9, 9]);
            let cap = [0u32, 1, 6, 0xffff_ffff][(case % 4) as usize];
            let what = format!("list ctor #{case}");
            t.check(&what, 0x0069_0680, Conv::Thiscall(list), &[cap], |m| {
                Out::U32(ab::list_construct(m, list, cap))
            });
        }
    });
}

#[test]
fn pool_ctor_and_dtors_match_the_original() {
    Twins::each(|t| {
        for cap in [0u32, 1, 200] {
            t.reset();
            let block = t.zeros(0x100);
            async_stubs(t, block);
            let this = t.words(&[0x5a5a_5a5a; 6]);
            let what = format!("cluster pool ctor {cap}");
            t.check(&what, 0x0069_06d0, Conv::Thiscall(this), &[cap], |m| {
                Out::U32(ab::cluster_pool_construct(m, this, cap))
            });
        }
        for flags in [0u32, 1, 2, 0xff] {
            for (sym, vft) in [
                (0x0069_0c40u32, al::CLUSTER_POOL_VFTABLE),
                (0x0069_0c80, al::TASK_POOL_VFTABLE),
            ] {
                t.reset();
                async_stubs(t, 0);
                let this = t.words(&[0, 0x0bad_0001, 0x0bad_0002, 3, 4, 0x0bad_0005]);
                let what = format!("pool dtor {sym:#x} {flags}");
                t.check(&what, sym, Conv::Thiscall(this), &[flags], |m| {
                    Out::U32(ab::pool_deleting_dtor(m, this, flags, vft))
                });
            }
            // requests: inline and heap names and arguments
            for case in 0..6u32 {
                t.reset();
                async_stubs(t, 0);
                let this = t.zeros(0x94);
                let name = if case & 1 == 0 {
                    this + 0x10
                } else {
                    0x0bad_0010
                };
                let args = [
                    this + 0x20,
                    this + 0x10,
                    this + 0x94,
                    this + 0xf,
                    this + 0x95,
                    0,
                ][case as usize];
                t.put(this + 4, &[name, 0, args]);
                let what = format!("cluster request dtor #{case} {flags}");
                t.check(&what, 0x0069_0cf0, Conv::Thiscall(this), &[flags], |m| {
                    Out::U32(ab::cluster_request_deleting_dtor(m, this, flags))
                });
                let this = t.zeros(0x20);
                let args = [this + 0xc, 0x0bad_0020, this + 0x10, 0, this + 8, this][case as usize];
                t.put(this + 8, &[args]);
                let what = format!("task request dtor #{case} {flags}");
                t.check(&what, 0x0069_0d50, Conv::Thiscall(this), &[flags], |m| {
                    Out::U32(ab::task_request_deleting_dtor(m, this, flags))
                });
            }
            t.reset();
            async_stubs(t, 0);
            let this = t.zeros(8);
            let what = format!("request dtor {flags}");
            t.check(&what, 0x0069_0d90, Conv::Thiscall(this), &[flags], |m| {
                Out::U32(ab::request_deleting_dtor(m, this, flags))
            });
        }
    });
}

/// A manager with `pools` at `+0x18`/`+0x1c`, request lists `lists`,
/// the current index `cur` and children `children` (`{count, capacity,
/// data}` each).
fn manager(
    t: &mut Twins,
    pools: [u32; 2],
    lists: [[u32; 3]; 2],
    cur: u32,
    children: [u32; 3],
) -> u32 {
    let m = t.zeros(0x4c);
    t.put(
        m,
        &[
            0x00b4_6fa4,
            0x0bad_7777,
            0x0bad_5e01,
            0x0bad_5e02,
            0x0bad_0a11,
            1,
        ],
    );
    t.put(m + 0x18, &pools);
    t.put(m + 0x20, &lists[0]);
    t.put(m + 0x2c, &lists[1]);
    t.put(m + 0x38, &[cur]);
    t.put(m + 0x3c, &children);
    m
}

#[test]
fn manager_ctor_and_dtor_match_the_original() {
    Twins::each(|t| {
        for case in 0..4u32 {
            t.reset();
            async_stubs(t, 0);
            let pools = [t.zeros(0x18), t.zeros(0x18)];
            let new = [
                if case & 1 == 0 { 0 } else { pools[0] },
                if case & 2 == 0 { 0 } else { pools[1] },
            ];
            t.stub_seq(al::OPERATOR_NEW, 1, 0, &new);
            let block = t.zeros(0x40);
            t.stub(al::USER_MALLOC, false, 1, 0, block);
            t.stub(al::NEW_ARRAY, false, 1, 0, block + 0x20);
            t.stub(al::VECTOR_CTOR, false, 5, 5, 0);
            t.stub(al::CREATE_SEMA, false, 1, 0, 0x5e5e_0000 + case);
            // the thread parameters are on the original's stack: log none
            t.stub(
                al::CREATE_THREAD,
                false,
                0,
                0,
                if case == 3 { 0 } else { 0x7e7e },
            );
            t.stub(al::START_THREAD, false, 2, 0, 0);
            t.stub(al::THREAD_PRIORITY, false, 2, 0, 0);
            let this = t.words(&[0x3333_3333; 0x13]);
            let what = format!("manager ctor #{case}");
            let frame = t.b_stack(0x1000);
            t.check(&what, 0x0069_0830, Conv::Thiscall(this), &[], |m| {
                Out::U32(ab::construct(m, frame, this))
            });
        }
        for case in 0..8u32 {
            t.reset();
            async_stubs(t, 0);
            t.stub(al::VECTOR_DTOR, false, 4, 4, 0);
            let p0 = t.object(&[(1, 0x41)]);
            let p1 = t.object(&[(1, 0x42)]);
            let m = manager(
                t,
                [
                    if case & 1 == 0 { 0 } else { p0 },
                    if case & 2 == 0 { 0 } else { p1 },
                ],
                [[1, 2, 3], [4, 5, 6]],
                0,
                [0, 0, if case & 4 == 0 { 0 } else { 0x0bad_c41d }],
            );
            let flags = case % 3;
            let what = format!("manager dtor #{case}");
            t.check(&what, 0x0069_0ac0, Conv::Thiscall(m), &[], |h| {
                Out::U32(ab::destruct(h, m))
            });
            let what = format!("manager deleting dtor #{case}");
            t.check(&what, 0x0069_0cc0, Conv::Thiscall(m), &[flags], |h| {
                Out::U32(ab::deleting_dtor(h, m, flags))
            });
        }
    });
}

#[test]
fn create_thread_matches_the_original() {
    Twins::each(|t| {
        for thread in [0u32, 0x7e7e] {
            t.reset();
            t.stub_seq(al::CREATE_SEMA, 1, 0, &[0x5e01, 0x5e02]);
            t.stub(al::CREATE_THREAD, false, 2, 0, thread);
            t.stub(al::START_THREAD, false, 2, 0, 0);
            t.stub(al::THREAD_PRIORITY, false, 2, 0, 0);
            let this = t.words(&[0x3333_3333; 0x13]);
            let what = format!("createThread {thread:#x}");
            t.check_framed(
                &what,
                0x0069_1180,
                Conv::Thiscall(this),
                &[],
                al::THREAD_PARAM,
                |m, ebp| {
                    ab::create_thread(m, ebp, this);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn queue_and_children_match_the_original() {
    Twins::each(|t| {
        for case in 0..30u32 {
            t.reset();
            // (a failed allocation while growing makes the original write
            // through a null list: list_reserve covers the failure)
            let block = t.zeros(0x100);
            async_stubs(t, block);
            let data = t.words(&[0xd1, 0xd2, 0xd3, 0xd4]);
            let count = [0u32, 1, 3, 4, 7, 0xffff_ffff][(case % 6) as usize];
            let cap = [4u32, 0, 0x8000_0000][((case / 6) % 3) as usize];
            // null data only where the slot's address is 0 (else the
            // original writes through it)
            let shape = [
                count,
                cap,
                if case % 5 == 4 && count == 0 { 0 } else { data },
            ];
            let list = t.words(&shape);
            let m = manager(t, [0, 0], [shape, shape], case & 1, [count, cap, data]);
            let what = format!("queue list #{case}");
            t.check(
                &what,
                0x0069_0e60,
                Conv::Thiscall(m),
                &[list, 0x0eee],
                |h| Out::U32(ab::queue(h, m, list, 0x0eee)),
            );
            let what = format!("queue manager #{case}");
            t.check(&what, 0x0069_0e60, Conv::Thiscall(m), &[0, 0x0eee], |h| {
                Out::U32(ab::queue(h, m, 0, 0x0eee))
            });
            let capacity = [0u32, 2, 0xffff_ffff][(case % 3) as usize];
            let what = format!("children list #{case}");
            t.check(&what, 0x0069_0f70, Conv::Thiscall(m), &[capacity], |h| {
                Out::U32(ab::create_children_list(h, m, capacity))
            });
        }
        // the slot address 0: the data pointer comes back in eax
        for data in [0u32, 0xffff_fffc] {
            t.reset();
            async_stubs(t, 0);
            let list = t.words(&[u32::from(data != 0), 4, data]);
            let m = manager(t, [0, 0], [[0; 3], [0; 3]], 0, [0; 3]);
            t.check(
                "queue slot 0",
                0x0069_0e60,
                Conv::Thiscall(m),
                &[list, 5],
                |h| Out::U32(ab::queue(h, m, list, 5)),
            );
        }
        // a negative count past a negative capacity grows by count / 8
        // rounded towards 0 (the slot lands below the new block)
        for count in [0xffff_fff1u32, 0xffff_ffe9] {
            t.reset();
            let block = t.zeros(0x100) + 0x80;
            async_stubs(t, block);
            let list = t.words(&[count, 0x8000_0000, 0]);
            let m = manager(t, [0, 0], [[0; 3], [0; 3]], 0, [0; 3]);
            let what = format!("queue negative {count:#x}");
            t.check(&what, 0x0069_0e60, Conv::Thiscall(m), &[list, 5], |h| {
                Out::U32(ab::queue(h, m, list, 5))
            });
        }
    });
}

/// A pool of `cap` requests of `size` bytes, `used` marking the used
/// ones, the free index `first`; each request gets a fake vtable whose
/// slot 0 is a recording stub (1 word).
fn pool(t: &mut Twins, size: u32, cap: u32, used: &[u8], first: u32) -> u32 {
    let n = cap.max(1);
    // two spare requests: a free index past the capacity hands one out
    let requests = t.alloc(&vec![0xee; (size * (n + 2)) as usize]);
    let vt = t.stub_code(0x5100, true, 1, 1, 0x51);
    let vt = t.words(&[vt]);
    for i in 0..n {
        t.put(requests + i * size, &[vt]);
    }
    let mut u = used.to_vec();
    u.resize(n as usize + 4, 1);
    let used = t.alloc(&u);
    t.words(&[0x00b4_6fac, requests, used, cap, first, 0x0bad_0a11])
}

#[test]
fn pool_alloc_and_free_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5157_eb01);
        for case in 0..40u32 {
            for (size, alloc, free) in [
                (0x94u32, 0x0069_0ff0u32, 0x0069_11f0u32),
                (0x20, 0x0069_10c0, 0),
            ] {
                t.reset();
                let block = if case % 5 == 0 {
                    0
                } else {
                    t.alloc(&[0xdd; 0x100])
                };
                async_stubs(t, block);
                imp(t, al::IOB_FUNC_IAT, 0, false, 0x00e0_0000);
                imp(t, al::FPRINTF_IAT, 3, false, 0);
                let cap = [0u32, 1, 3, 8][(case % 4) as usize];
                let used: Vec<u8> = (0..cap).map(|_| u8::from(rng.chance(2))).collect();
                let below = t.object(&[(1, 0x53)]);
                let first = match rng.below(6) {
                    0 | 1 => cap,
                    2 => cap + 1,
                    _ => rng.below(cap.max(1)),
                };
                let p = pool(t, size, cap, &used, first);
                let what = format!("pool alloc {alloc:#x} #{case}");
                t.check(&what, alloc, Conv::Thiscall(p), &[], |h| {
                    Out::U32(if size == 0x94 {
                        ab::cluster_pool_alloc(h, p)
                    } else {
                        ab::task_pool_alloc(h, p)
                    })
                });
                // free: inside, outside (a fake object) and null
                let requests = t.r32(p + 4);
                let outside = t.object(&[(1, 0x52)]);
                let request = match rng.below(5) {
                    0 => 0,
                    1 => outside,
                    2 => below,
                    _ => requests + size * rng.below(cap.max(1)),
                };
                if free != 0 {
                    let what = format!("pool free #{case}");
                    t.check(&what, free, Conv::Thiscall(p), &[request], |h| {
                        Out::U32(ab::cluster_pool_free(h, p, request))
                    });
                    t.g32(al::CLUSTER_POOL, p);
                    let what = format!("cluster release #{case}");
                    t.check(&what, 0x0069_12c0, Conv::Thiscall(request), &[], |h| {
                        Out::U32(ab::free_cluster_request(h, request))
                    });
                } else {
                    t.g32(al::TASK_POOL, p);
                    let what = format!("task release #{case}");
                    let addr = t.rebase(0x0069_12d0);
                    let ra = t
                        .a
                        .call(addr, Conv::Thiscall(request), &[])
                        .unwrap_or_else(|e| panic!("{what}: {request:#x} {:#x} {e}", t.r32(p + 4)));
                    let poison = t.rebase(POISON_AT);
                    let got = ab::free_task_request(&mut G(&mut t.b, poison), request);
                    if let Some(v) = got {
                        assert_eq!(ra.eax, v, "{what}: eax");
                    }
                    t.diff(&what);
                }
            }
        }
    });
}

#[test]
fn thread_main_matches_the_original() {
    Twins::each(|t| {
        for case in 0..12u32 {
            t.reset();
            async_stubs(t, 0);
            t.stub(al::EXIT_THREAD, false, 1, 0, 0x99);
            let r: Vec<u32> = (0..4)
                .map(|i| t.object(&[(0, 0), (0, 0x60 + i), (0, 0x70 + i)]))
                .collect();
            let d0 = t.words(&[r[0], r[1]]);
            let d1 = t.words(&[r[2]]);
            let kids = t.words(&[r[3], r[1]]);
            let head = t.words(&[
                case % 3,
                2,
                if case % 3 != 0 || case & 4 == 0 {
                    kids
                } else {
                    0
                },
            ]);
            // (a null head faults in the original: it reads the count first)
            let heads = t.words(&[head]);
            let lists = [[case % 3, 2, d0], [case & 1, 1, d1]];
            let children = match case % 4 {
                0 => [0, 0, heads],
                1 => [1, 1, heads],
                2 => [0xffff_ffff, 0xffff_fffe, heads],
                _ => [0xffff_ffff, 1, heads],
            };
            let m = manager(t, [0, 0], lists, case & 1, children);
            // running: 0 (case 10: no pass), else 1 or 2 passes
            t.put(m + 0x14, &[if case == 10 { 0 } else { 1 + case % 2 }]);
            // WaitSema: logs, then counts the running flag down
            let log_stub = t.stub_code(al::WAIT_SEMA, false, 1, 0, 1);
            let mut c = vec![0xff, 0x0d];
            c.extend((m + 0x14).to_le_bytes()); // dec dword [m + 0x14]
            c.push(0xe9);
            let here = t.cave_next + c.len() as u32 + 4;
            c.extend(log_stub.wrapping_sub(here).to_le_bytes());
            let wait = t.cave_code(&c);
            t.jump(al::WAIT_SEMA, wait);
            t.g32(al::MANAGER, m);
            let what = format!("threadMain #{case}");
            t.check(&what, al::THREAD_MAIN, Conv::Cdecl, &[], |h| {
                Out::U32(ab::thread_main(h))
            });
        }
    });
}

#[test]
fn cluster_request_init_matches_the_original() {
    Twins::each(|t| {
        for case in 0..20u32 {
            t.reset();
            let block = t.zeros(0x400);
            async_stubs(t, block);
            let len = [0usize, 5, 0x83, 0x84, 0x100][(case % 5) as usize];
            let mut n = vec![b'c'; len];
            n.push(0);
            let name = t.alloc(&n);
            let count = [0u32, 1, 5, 0x20][(case / 5) as usize];
            let mut w = vec![count];
            w.extend((0..count).map(|i| 0xa000 + i));
            let args = t.words(&w);
            let this = t.zeros(0x94);
            let what = format!("LoadClusterRequest init #{case}");
            t.check(
                &what,
                0x0069_1550,
                Conv::Thiscall(this),
                &[name, 0x77, args],
                |h| {
                    ab::cluster_request_init(h, this, name, 0x77, args);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn load_cluster_and_regist_task_match_the_original() {
    Twins::each(|t| {
        for case in 0..12u32 {
            t.reset();
            let block = t.zeros(0x400);
            async_stubs(t, block);
            // (a negative count makes the original copy 4 * count bytes)
            let count = [0u32, 1, 2, 5, 6, 3][(case % 6) as usize];
            let n = count;
            let mut va = Vec::new();
            for i in 0..n {
                va.extend([0xb000 + i, 0xdead_0000 + i]);
            }
            let varargs = t.words(&[&va[..], &[0, 0]].concat());
            let cp = pool(t, 0x94, 4, &[1, 0, 0, 0], case % 3);
            let tp = pool(t, 0x20, 4, &[0, 1, 0, 0], case % 2);
            let l0 = t.zeros(8);
            let l1 = t.zeros(8);
            let m = manager(t, [cp, tp], [[0, 2, l0], [1, 1, l1]], case & 1, [0, 0, 0]);
            let list = if case & 2 == 0 {
                0
            } else {
                let d = t.zeros(16);
                t.words(&[0, 4, d])
            };
            let name = t.alloc(if case & 4 == 0 { b"bsil0000\0" } else { b"\0" });
            let mut args = vec![m, list, name, 0x55, count];
            args.extend(&va);
            let array = t.b_stack(0x2000);
            let what = format!("loadCluster #{case}");
            t.check(&what, 0x0069_1660, Conv::Cdecl, &args, |h| {
                Out::U32(ab::load_cluster(
                    h, array, m, list, name, 0x55, count, varargs,
                ))
            });
            let mut args = vec![m, 0x7a5c, count];
            args.extend(&va);
            let what = format!("registAsyncLoadTask #{case}");
            t.check(&what, 0x0069_18b0, Conv::Cdecl, &args, |h| {
                Out::U32(ab::regist_task(h, array, m, 0x7a5c, count, varargs))
            });
        }
    });
}

#[test]
fn platform_specific_path_matches_the_original() {
    Twins::each(|t| {
        let cases = [
            (0u32, 15u32),
            (5, 15),
            (15, 15),
            (20, 31),
            (0xffff_ffff, 3),
            (3, 16),
        ];
        for (case, (size, cap)) in cases.into_iter().enumerate() {
            t.reset();
            t.stub_import_copy(al::MEMMOVE_IAT);
            let words = [0x6c69_7362, 0x3030_3030, 0x0063_6261, 0, size, cap];
            let arg = t.words(&words);
            let out = t.words(&[0x9999_9999; 6]);
            let mut args = vec![out];
            args.extend(words);
            let what = format!("getPlatformSpecificPath #{case}");
            t.check(&what, 0x0069_0590, Conv::Cdecl, &args, |h| {
                Out::U32(ab::platform_specific_path(h, out, arg))
            });
        }
    });
}

// ---- batch 2: std::_Tree, strings, vectors, the asset map, Steam ----

impl Twins {
    /// A recording stub over `sym` that first stores `value` at `[ecx +
    /// off]` (when `ecx_off`) or at `*[esp + 4 + 4 * word]`.
    #[allow(clippy::too_many_arguments)]
    fn stub_store(
        &mut self,
        sym: u32,
        ecx_off: Option<u8>,
        word: u8,
        value: u32,
        this: bool,
        n: u32,
        pops: u32,
        ret: u32,
    ) {
        let log_stub = self.stub_code(sym, this, n, pops, ret);
        let mut c = Vec::new();
        if let Some(off) = ecx_off {
            c.extend([0xc7, 0x41, off]); // mov dword [ecx + off], value
            c.extend(value.to_le_bytes());
        } else {
            c.extend([0x8b, 0x44, 0x24, 4 + 4 * word]); // mov eax, [esp + 4 + 4w]
            c.extend([0xc7, 0x00]); // mov dword [eax], value
            c.extend(value.to_le_bytes());
        }
        c.push(0xe9);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(log_stub.wrapping_sub(here).to_le_bytes());
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }
}

/// A tree head (nil, black, its links at itself) and the tree object
/// `{head, size}`.
fn tree(t: &mut Twins) -> (u32, u32) {
    let head = t.zeros(0x30);
    t.put(head, &[head, head, head]);
    t.write(head + 0xc, &[1, 1]);
    let this = t.words(&[head, 0]);
    (this, head)
}

/// Where a key goes in the tree (the BST search the callers do):
/// `(where, addLeft)`.
fn insert_site(t: &Twins, head: u32, key: u32) -> (u32, u32) {
    let mut at = head;
    let mut left = true;
    let mut n = t.r32(head + 4);
    while Mem::read::<u8>(&t.a, n + 0xd) == 0 {
        at = n;
        left = key < t.r32(n + 0x10);
        n = t.r32(n + if left { 0 } else { 8 });
    }
    (at, u32::from(left))
}

/// Inserts `keys` with the original in `a` and the port in `b`, each
/// insert checked; returns the tree, its head and the nodes.
fn build_tree(t: &mut Twins, keys: &[u32]) -> (u32, u32, Vec<u32>) {
    let (this, head) = tree(t);
    let out = t.zeros(4);
    let mut nodes = Vec::new();
    for (i, &k) in keys.iter().enumerate() {
        // as _Buynode leaves it: both children the head, red, not nil
        let node = t.zeros(0x18);
        t.put(node, &[head, 0, head, 0, k]);
        let (at, left) = insert_site(t, head, k);
        let what = format!("tree insert #{i} ({k})");
        t.check(
            &what,
            0x0069_93d0,
            Conv::Thiscall(this),
            &[out, left, at, 0x1234, node],
            |m| Out::U32(st::tree_insert_at(m, this, out, left, at, node)),
        );
        nodes.push(node);
    }
    (this, head, nodes)
}

#[test]
fn tree_insert_and_erase_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7ee5_0001);
        for round in 0..6u32 {
            t.reset();
            t.stub(stl::OPERATOR_DELETE, false, 1, 0, 0x1d);
            let n = [1usize, 2, 5, 17, 40, 64][round as usize];
            let keys: Vec<u32> = (0..n)
                .map(|i| match round {
                    // ascending and descending runs, then random
                    2 => i as u32,
                    3 => 1000 - i as u32,
                    _ => rng.below(500),
                })
                .collect();
            let (this, head, mut nodes) = build_tree(t, &keys);
            let out = t.zeros(4);
            let slot = t.b_stack(0x3000);
            while !nodes.is_empty() {
                let k = rng.below(nodes.len() as u32) as usize;
                let node = nodes.swap_remove(k);
                Mem::write_bytes(&mut t.b, slot, &node.to_le_bytes());
                let what = format!("tree erase round {round} ({} left)", nodes.len());
                t.check(
                    &what,
                    0x0069_b010,
                    Conv::Thiscall(this),
                    &[out, node],
                    |m| Out::U32(st::tree_erase(m, this, out, slot)),
                );
            }
            assert_eq!(t.r32(head + 4), head, "round {round}: the tree is empty");
        }
    });
}

#[test]
fn hash_set_contains_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x4a54_0002);
        t.reset();
        t.stub(stl::OPERATOR_DELETE, false, 1, 0, 0x1d);
        let keys: Vec<u32> = (0..20).map(|_| rng.next()).collect();
        let (_, head, _) = build_tree(t, &keys);
        let lock = t.words(&[0x0bad_0a11]);
        let this = t.zeros(0x20);
        t.put(this + 0x10, &[head]);
        t.put(this + 0x18, &[lock]);
        let probes: Vec<u32> = keys
            .iter()
            .copied()
            .chain([
                0,
                1,
                0xffff_ffff,
                keys[0].wrapping_add(1),
                keys[3].wrapping_sub(1),
            ])
            .collect();
        for (i, key) in probes.into_iter().enumerate() {
            t.reset();
            t.stub(stl::DJB_HASH, false, 1, 0, key);
            imp(t, stl::WAIT_IAT, 2, true, 0);
            imp(t, stl::RELEASE_MUTEX_IAT, 1, true, 0x78);
            let what = format!("hash set contains #{i}");
            t.check(
                &what,
                0x0069_b7c0,
                Conv::Thiscall(this),
                &[0x00b4_6fb8],
                |m| Out::U32(st::hash_set_contains(m, this, 0x00b4_6fb8)),
            );
        }
    });
}

#[test]
fn string_move_with_word_matches_the_original() {
    Twins::each(|t| {
        for (case, (size, cap)) in [
            (0u32, 15u32),
            (5, 15),
            (15, 15),
            (20, 31),
            (0xffff_ffff, 3),
            (3, 16),
        ]
        .into_iter()
        .enumerate()
        {
            t.reset();
            t.stub_import_copy(stl::MEMMOVE_IAT);
            let src = t.words(&[
                0x6c69_7362,
                0x3030_3030,
                0x0063_6261,
                0x0102_0304,
                size,
                cap,
                0x5eed,
            ]);
            let this = t.words(&[0x9999_9999; 8]);
            let what = format!("string move #{case}");
            t.check(&what, 0x0069_c200, Conv::Thiscall(this), &[src, 7], |m| {
                Out::U32(st::string_move_with_word(m, this, src))
            });
        }
    });
}

#[test]
fn wait_ppp_part_matches_the_original() {
    use ffx_system::m5a_platform::bridge as pf;
    Twins::each(|t| {
        let mut rng = Rng(0x71e8_1004);
        for case in 0..20 {
            t.reset();
            t.stub_import(pf::WAIT_FOR_SINGLE_OBJECT_IAT, false, 2, 2, 0);
            t.stub_import(pf::RELEASE_SEMAPHORE_IAT, false, 3, 3, 1);
            let this = t.alloc(
                &(0..0x1e8 * 3 + 0x300)
                    .map(|_| rng.next() as u8)
                    .collect::<Vec<_>>(),
            );
            let count = [0u32, 7, 8, 9, 0xffff_ffff][rng.below(5) as usize];
            t.w32(this + 0x1f8, count);
            t.g32(pf::CLOTH_SCHEDULER, this);
            let what = format!("JobScheduleWait_pppPart #{case}");
            t.check(&what, 0x0071_e810, Conv::Cdecl, &[], |m| {
                pf::wait_cloth(m);
                Out::None
            });
        }
    });
}

#[test]
fn read_localization_kit_matches_the_original() {
    Twins::each(|t| {
        t.skip = 4;
        for lang in 0..13u32 {
            for case in 0..3u32 {
                t.reset();
                t.stub(ms::MEMSET, false, 3, 0, 0);
                imp(t, ms::SPRINTF_IAT, 4, false, 9);
                imp(t, ms::PRINTF_IAT, 2, false, 9);
                t.stub(ms::FIOS_UNIFY, false, 3, 0, 0);
                t.stub(
                    ms::FIOS_OPEN,
                    false,
                    2,
                    0,
                    if case == 0 { 0 } else { 0x0f11_e000 },
                );
                t.stub_seq(ms::FIOS_LSEEK, 3, 0, &[0x40, 0]);
                let buf = t.zeros(0x40);
                t.stub(ms::NEW_ARRAY, false, 1, 0, buf);
                let reads: &[u32] = if case == 2 { &[3, 0x3f, 0x40] } else { &[0x41] };
                t.stub_seq(ms::FIOS_READ, 3, 0, reads);
                t.stub(ms::FIOS_CLOSE, false, 1, 0, 0);
                let this = t.words(&[0, lang, 0, 0, 0, 0x77, 0x88]);
                let what = format!("readLocalizationKit lang {lang} #{case}");
                t.check_framed(
                    &what,
                    0x006d_b2f0,
                    Conv::Thiscall(this),
                    &[],
                    ms::LOC_KIT_FRAME,
                    |m, ebp| {
                        mb::read_localization_kit(m, ebp, this);
                        Out::None
                    },
                );
            }
        }
        t.skip = 0;
    });
}

#[test]
fn depth_bias_on_scene_loaded_matches_the_original() {
    Twins::each(|t| {
        for case in 0..8u32 {
            t.reset();
            let level = t.alloc(b"djyt04\0");
            t.stub(ms::LEVEL_NAME, true, 0, 0, level);
            let hits: Vec<u32> = (0..6)
                .map(|i| u32::from((case >> (i % 3)) & 1 != 0) * (level + 1))
                .collect();
            t.stub_import_seq(ms::STRSTR_IAT, 2, 0, &hits);
            t.stub(ms::SET_MAP_ALPHA_FAST, false, 1, 0, 0);
            t.stub(ms::ASSET_LIST_DTOR, true, 0, 0, 0);
            t.stub(ms::USER_FREE, false, 1, 0, 0);
            t.g32(ms::PHYRE_SCENE, 0x0c0c_0000 + case);
            let mut args = vec![0x5151_5151u32];
            args.extend((0..40).map(|i| 0x100 + i));
            args[0x50 / 4] = if case & 1 == 0 { 0 } else { 0x0b10_c000 };
            args[0xa0 / 4] = if case & 2 == 0 { 0 } else { 0x0b10_c001 };
            // the port's view of the argument words: where the original's are
            let ebp = t.frame_ebp(args.len());
            let slots = ebp + 8;
            let bytes: Vec<u8> = args.iter().flat_map(|v| v.to_le_bytes()).collect();
            Mem::write_bytes(&mut t.b, slots, &bytes);
            t.b.set_call_top(Some(slots - 0x200));
            let what = format!("DepthBias onSceneLoaded #{case}");
            t.check(&what, 0x006e_4470, Conv::Cdecl, &args, |m| {
                mb::depth_bias_on_scene_loaded(m, slots);
                Out::None
            });
            t.b.set_call_top(None);
        }
    });
}

#[test]
fn steam_callbacks_match_the_original() {
    Twins::each(|t| {
        for flags in [0u32, 1, 2, 3] {
            for (sym, vft) in [
                (0x006f_1310u32, steam::ACHIEVEMENT_STORED_VFTABLE),
                (0x006f_1330, steam::STATS_RECEIVED_VFTABLE),
                (0x006f_1350, steam::STATS_STORED_VFTABLE),
            ] {
                t.reset();
                imp(t, steam::UNREGISTER_IAT, 1, false, 0x31);
                let this = t.words(&[0, flags, 0, 0, 0]);
                let what = format!("CCallback dtor {sym:#x} {flags}");
                t.check(&what, sym, Conv::Thiscall(this), &[], |m| {
                    sm::callback_dtor(m, this, vft);
                    Out::None
                });
            }
        }
        for sym in [0x006f_1820u32, 0x006f_1850, 0x006f_1880] {
            t.reset();
            let target = t.stub_code(0x7a46, true, 1, 1, 0x61);
            let this = t.words(&[0, 0, 0, 0x0b1e_c700, target]);
            let what = format!("CCallback run {sym:#x}");
            t.check(&what, sym, Conv::Thiscall(this), &[0x9a9a], |m| {
                Out::U32(sm::callback_run(m, this, 0x9a9a))
            });
            let what = format!("CCallback run call {:#x}", sym + 0x10);
            t.check(
                &what,
                sym + 0x10,
                Conv::Thiscall(this),
                &[0x9b9b, 1, 2, 3],
                |m| Out::U32(sm::callback_run_call(m, this, 0x9b9b)),
            );
        }
    });
}

#[test]
fn on_user_stats_received_matches_the_original() {
    Twins::each(|t| {
        for case in 0..8u32 {
            t.reset();
            let text = t.alloc(b"Sphere Grid\0");
            let mut slots = vec![(0u32, 0u32); 13];
            slots[6] = (2, 1);
            slots[12] = (2, text);
            let stats = t.object(&slots);
            imp(t, steam::USER_STATS_IAT, 0, false, stats);
            imp(t, steam::SNPRINTF_IAT, 4, false, 0);
            let count = [0u32, 1, 3, 0xffff_ffff][(case % 4) as usize];
            let records = t.zeros(0x190 * 3);
            for i in 0..3 {
                t.put(records + 0x190 * i + 4, &[0x00b4_1000 + i]);
            }
            let this = t.zeros(0x50);
            t.put(this + 0x3c, &[0x1111, 0x2222, records, count]);
            let id = if case & 4 == 0 {
                [0x1111u32, 0x2222]
            } else {
                [0x1111, 0x2223]
            };
            let result = [1u32, 1, 1, 2][(case % 4) as usize];
            let cb = t.words(&[id[0], id[1], result]);
            let what = format!("OnUserStatsReceived #{case}");
            t.check(&what, 0x006f_15f0, Conv::Thiscall(this), &[cb], |m| {
                sm::on_user_stats_received(m, this, cb);
                Out::None
            });
        }
    });
}

/// A `std::string` in a node at `at + 0x10` (inline below 16 characters).
fn put_string(t: &mut Twins, at: u32, s: &[u8]) {
    let len = s.len() as u32;
    if s.len() < 16 {
        let mut b = s.to_vec();
        b.resize(16, 0);
        t.write(at, &b);
        t.put(at + 0x10, &[len, 15]);
    } else {
        let mut b = s.to_vec();
        b.push(0);
        let p = t.alloc(&b);
        t.put(at, &[p]);
        t.put(at + 0x10, &[len, len.max(16)]);
    }
}

/// A balanced search tree of string-keyed nodes (`keys` sorted), its
/// head; colours are not read by these functions.
fn string_tree(t: &mut Twins, keys: &[Vec<u8>]) -> (u32, u32) {
    fn build(t: &mut Twins, head: u32, parent: u32, keys: &[Vec<u8>]) -> u32 {
        if keys.is_empty() {
            return head;
        }
        let mid = keys.len() / 2;
        let node = t.zeros(0x30);
        put_string(t, node + 0x10, &keys[mid]);
        t.write(node + 0xc, &[1, 0]);
        let left = build(t, head, node, &keys[..mid]);
        let right = build(t, head, node, &keys[mid + 1..]);
        t.put(node, &[left, parent, right]);
        node
    }
    let (this, head) = tree(t);
    let root = build(t, head, head, keys);
    t.put(head + 4, &[root]);
    t.put(this + 4, &[keys.len() as u32]);
    (this, head)
}

fn string_keys() -> Vec<Vec<u8>> {
    let mut keys: Vec<Vec<u8>> = [
        &b"a"[..],
        b"ab",
        b"abc",
        b"b",
        b"bsil0000",
        b"bsil0300",
        b"kami0100",
        b"kami0100_long_name_here",
        b"sixteen_chars_ab",
        b"z",
        b"\xff",
        b"",
    ]
    .iter()
    .map(|s| s.to_vec())
    .collect();
    keys.sort();
    keys
}

#[test]
fn string_map_equal_range_and_erase_match_the_original() {
    Twins::each(|t| {
        let probes: Vec<&[u8]> = vec![
            b"",
            b"a",
            b"aa",
            b"abc",
            b"abcd",
            b"b",
            b"bsil",
            b"bsil0000",
            b"bsil0200",
            b"kami0100_long_name_here",
            b"kami0100_long_name_herf",
            b"zz",
            b"\xff",
            b"\xff\xff",
        ];
        for (i, p) in probes.iter().enumerate() {
            for keys in [string_keys(), Vec::new(), string_keys()[..1].to_vec()] {
                t.reset();
                let (this, _) = string_tree(t, &keys);
                let key = t.zeros(0x18);
                put_string(t, key, p);
                let out = t.zeros(8);
                let what = format!("equal_range #{i} ({} keys)", keys.len());
                t.check(&what, 0x006f_3c60, Conv::Thiscall(this), &[out, key], |m| {
                    Out::U32(st::string_map_equal_range(m, this, out, key))
                });
                if i % 2 == 0 {
                    t.stub_store(stl::RANGE_COUNT, None, 2, 7, false, 4, 0, 0);
                } else {
                    // stores nothing: the 0 the erase writes first comes back
                    t.stub(stl::RANGE_COUNT, false, 4, 0, 0);
                    let ebp = t.frame_ebp(1);
                    Mem::write_bytes(&mut t.b, ebp + 8, &0x5555_u32.to_le_bytes());
                }
                t.stub(stl::TREE_ERASE_RANGE, true, 3, 3, 0);
                let what = format!("erase key #{i} ({} keys)", keys.len());
                t.check_framed(
                    &what,
                    0x006f_40e0,
                    Conv::Thiscall(this),
                    &[key],
                    8,
                    |m, ebp| {
                        Out::U32(st::string_map_erase_key(
                            m,
                            ebp - 8,
                            ebp - 4,
                            ebp + 8,
                            this,
                            key,
                        ))
                    },
                );
            }
        }
    });
}

#[test]
fn vector_growth_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7ec7_0003);
        for case in 0..40u32 {
            t.reset();
            let fail = case % 9 == 4;
            let block = if fail { 0 } else { t.zeros(0x400) };
            t.stub(stl::OPERATOR_NEW, false, 1, 0, block);
            // a freed block's first word changes (a value inside is re-read)
            t.stub_store(
                stl::OPERATOR_DELETE,
                None,
                0,
                0xdead_beef,
                false,
                1,
                0,
                0x1d,
            );
            imp(t, stl::XBAD_ALLOC_IAT, 0, false, 0);
            imp(t, stl::XLENGTH_IAT, 1, false, 0);
            t.stub_import_copy(stl::MEMMOVE_IAT);
            let size = rng.below(6);
            let cap = size + rng.below(3);
            let data = if size == 0 && rng.chance(2) {
                0
            } else {
                let w: Vec<u32> = (0..cap.max(1)).map(|i| 0xd000 + i).collect();
                t.words(&w)
            };
            let v = t.words(&[data, data + 4 * size, data + 4 * cap]);
            let n = [0u32, 1, 3, 0x4000_0000, 0x3fff_ffff, 7][(case % 6) as usize];
            // (a block of 0 for a vector with words makes the original copy
            // to address 0)
            if (n != 0 && !fail || size == 0) && (n < 0x100 || n == 0x3fff_ffff && !fail) {
                let what = format!("vector reallocate #{case}");
                t.check(&what, 0x006f_3ee0, Conv::Thiscall(v), &[n], |m| {
                    Out::U32(st::vector_reallocate(m, v, n))
                });
            }
            t.put(v, &[data, data + 4 * size, data + 4 * cap]);
            // (past the limit the original calls _Xlength_error without
            // popping its argument: it never returns, and after a stub's
            // return the stack is off, so that path is left out)
            if !fail && n < 0x100 {
                let what = format!("vector reserve #{case}");
                t.check(&what, 0x006f_3f60, Conv::Thiscall(v), &[n], |m| {
                    Out::U32(st::vector_reserve(m, v, n))
                });
            }
            if !fail {
                t.put(v, &[data, data + 4 * size, data + 4 * cap]);
                let inside = size > 0 && rng.chance(2);
                let value = if inside {
                    data + 4 * rng.below(size)
                } else {
                    t.words(&[0xabcd_0000 + case])
                };
                let what = format!("vector push_back #{case}");
                t.check(&what, 0x006f_4890, Conv::Thiscall(v), &[value], |m| {
                    st::vector_push_back(m, v, value);
                    Out::None
                });
            }
        }
    });
}

#[test]
fn asset_list_clear_matches_the_original() {
    Twins::each(|t| {
        t.reset();
        t.stub(stl::VECTOR_DESTROY, true, 2, 2, 0);
        t.stub(stl::TREE_ERASE_SUBTREE, true, 1, 1, 0);
        let (map, head) = tree(t);
        let root = t.zeros(0x18);
        t.put(head + 4, &[root]);
        let mapw = t.r32(map);
        let this = t.words(&[
            mapw, 5, 0x111, 0x222, 0x333, 0x444, 0x555, 0x666, 0x777, 0x888,
        ]);
        t.check(
            "AssetListReader clear",
            0x006f_4030,
            Conv::Thiscall(this),
            &[],
            |m| Out::U32(st::asset_list_clear(m, this)),
        );
    });
}

#[test]
fn has_key_and_symbol_id_match_the_original() {
    Twins::each(|t| {
        for case in 0..16u32 {
            t.reset();
            let (this, head) = tree(t);
            let node = t.zeros(0x40);
            put_string(
                t,
                node + 0x10,
                if case & 8 == 0 {
                    b"bsil"
                } else {
                    b"a_long_key_name_over_16"
                },
            );
            t.stub(stl::OPERATOR_DELETE, false, 1, 0, 0x1d);
            // the assign leaves capacity 15 or a heap buffer's
            t.stub_store(
                stl::STRING_ASSIGN,
                Some(0x14),
                0,
                [15, 0x1f, 15, 0x10][(case % 4) as usize],
                true,
                2,
                2,
                0,
            );
            t.stub(
                stl::LOWER_BOUND,
                true,
                1,
                1,
                if case & 2 == 0 { head } else { node },
            );
            t.stub(
                stl::STRING_COMPARE,
                true,
                4,
                4,
                if case & 4 == 0 { 0 } else { 0xffff_ffff },
            );
            let value = t.words(&[0x5150_0000 + case]);
            t.stub(stl::MAP_INDEX, true, 1, 1, value);
            let name = t.alloc(if case % 3 == 0 { b"\0" } else { b"bsil\0" });
            t.skip = 0x10;
            let what = format!("hasKey #{case}");
            t.check_framed(
                &what,
                0x006f_4410,
                Conv::Thiscall(this),
                &[name],
                stl::HAS_KEY_FRAME,
                |m, ebp| Out::U8(st::string_map_has_key(m, ebp, this, name)),
            );
            // (a heap capacity would make the nested hasKey free its
            // string's first word, which the stub leaves as stack junk)
            if case & 1 != 0 {
                t.skip = 0;
                continue;
            }
            let what = format!("getSymbolId #{case}");
            t.check_framed(
                &what,
                0x006f_4350,
                Conv::Thiscall(this),
                &[name],
                stl::SYMBOL_FRAME,
                |m, ebp| Out::U32(st::get_symbol_id(m, ebp, this, name)),
            );
            t.skip = 0;
        }
    });
}

impl Twins {
    /// A recording cdecl stub bound to the import slot `slot` that logs
    /// `widths[k]` words on its `k`-th call (the last width from then on).
    fn stub_import_widths(&mut self, slot: u32, widths: &[u32], eax: u32) {
        let stubs: Vec<u32> = widths
            .iter()
            .map(|&n| self.stub_code(slot, false, n, 0, eax))
            .collect();
        let tbl = self.words(&stubs);
        let ctr = self.words(&[0]);
        let last = (widths.len() - 1) as u8;
        let mut c = vec![0xa1];
        c.extend(ctr.to_le_bytes()); // mov eax, [ctr]
        c.extend([0xff, 0x05]);
        c.extend(ctr.to_le_bytes()); // inc dword [ctr]
        c.extend([0x83, 0xf8, last]); // cmp eax, last
        c.extend([0x76, 0x05]); // jbe +5
        c.push(0xb8);
        c.extend(u32::from(last).to_le_bytes()); // mov eax, last
        c.extend([0xff, 0x24, 0x85]);
        c.extend(tbl.to_le_bytes()); // jmp [tbl + eax*4]
        let addr = self.cave_code(&c);
        let at = self.rebase(slot);
        self.patch(at, &addr.to_le_bytes());
    }
}

#[test]
fn fmod_flush_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xf10d_0005);
        for case in 0..30u32 {
            t.reset();
            imp(t, sy::SET_EVENT_IAT, 1, true, 1);
            // the 70 handlers: one recording stub (the argument block it
            // logs tells the commands apart)
            let handler = t.stub_code(0x7000, false, 2, 0, 0x100 + case);
            let table = t.rebase(sy::FMOD_HANDLERS);
            for k in 0..70u32 {
                t.patch(table + 4 * k, &handler.to_le_bytes());
            }
            let commands = t.rebase(sy::FMOD_RING);
            for i in 0..150u32 {
                t.patch(commands + 20 * i, &rng.below(70).to_le_bytes());
            }
            // (write index 0 with the read index just below it: the last
            // step wraps through 150, its quotient 1 comes back)
            let write = if case % 5 == 2 { 0 } else { rng.below(150) };
            let read = match case % 3 {
                0 => write,
                1 => (write + 150 - 1 - rng.below(5)) % 150,
                _ => rng.below(150),
            };
            let this = t.zeros(0x50);
            t.put(this + 8, &[0xf00d]);
            t.put(this + 0x1c, &[0xe7e7, 0, 0, write, read, rng.below(70)]);
            t.write(this + 0x44, &[u8::from(case % 4 == 0)]);
            let what = format!("FMOD flush #{case}");
            t.check(&what, 0x006f_a7e0, Conv::Thiscall(this), &[], |m| {
                Out::U32(sb3::fmod_flush(m, this))
            });
        }
    });
}

#[test]
fn fmod_objects_ctor_matches_the_original() {
    Twins::each(|t| {
        for case in 0..8u32 {
            t.reset();
            let blocks = [t.zeros(0x34), t.zeros(0x90), t.zeros(0xd4)];
            let seq: Vec<u32> = (0..3)
                .map(|i| {
                    if case >> i & 1 == 0 {
                        0
                    } else {
                        blocks[i as usize]
                    }
                })
                .collect();
            t.stub_seq(sy::OPERATOR_NEW, 1, 0, &seq);
            t.stub(sy::MUSIC_CTOR, true, 0, 0, 0x3001);
            t.stub(sy::SOUND_CTOR, true, 0, 0, 0x3002);
            t.stub(sy::SFX_CTOR, true, 0, 0, 0x3003);
            let this = t.words(&[0x7777_7777; 10]);
            let what = format!("FMOD objects ctor #{case}");
            t.check(&what, 0x0070_6490, Conv::Thiscall(this), &[], |m| {
                Out::U32(sb3::fmod_objects_ctor(m, this))
            });
        }
    });
}

#[test]
fn radial_blur_setters_match_the_original() {
    use ffx_glue_other::frames::{ParamSlot, post_set_param};
    Twins::each(|t| {
        let mut rng = Rng(0x7036_1006);
        for case in 0..40u32 {
            t.reset();
            t.stub_copy(ms::MEMCPY);
            t.stub(
                ms::BIND_TEXTURE,
                true,
                1,
                1,
                if case % 7 == 3 { 0x22 } else { 0 },
            );
            let block = t.zeros(0x80);
            t.put(block, &[[0x40u32, 0x10, 0x80, 0x3c][(case % 4) as usize]]);
            let holder = t.words(&[0, block]);
            let kind = [0x42u8, 0xd9, 0xdc, 0x12, 0x13, 0x41][rng.below(6) as usize];
            let offset = [0u16, 3, 4, 0x10, 0x30][rng.below(5) as usize];
            // (sizes past 0x10 copy the bytes after the value block or the
            // argument words: the callers' stack, which differs here)
            let size = [0x10u16, 0xe010, 8, 0][rng.below(4) as usize];
            let mut rec = vec![0u8, 0, kind, 0, 0, 0, 0, 0];
            rec.extend(offset.to_le_bytes());
            rec.extend(size.to_le_bytes());
            let rec = t.alloc(&rec);
            let this = t.zeros(0x70);
            t.put(this + 0x2c, &[holder]);
            let present = !rng.chance(5);
            for off in [0x58u32, 0x5c, 0x60] {
                t.put(this + off, &[if present { rec } else { 0 }]);
            }
            let sampler = if rng.chance(4) { 0 } else { 0x5a5a };
            let texture = if rng.chance(4) { 0 } else { 0x7e7e };
            let what = format!("RadialBlur texture param #{case}");
            t.skip = 0;
            t.check_framed(
                &what,
                0x0070_3610,
                Conv::Thiscall(this),
                &[sampler, texture],
                0x10,
                |m, ebp| {
                    Out::U32(mb::post_set_texture_param(
                        m,
                        ebp - 0x10,
                        this,
                        0x60,
                        0x2c,
                        sampler,
                        texture,
                    ))
                },
            );
            let words = [rng.next(), rng.next(), rng.next(), rng.next()];
            for (sym, slot) in [
                (
                    0x0070_3780u32,
                    ParamSlot {
                        guard: 0x58,
                        param: 0x58,
                        holder: 0x2c,
                    },
                ),
                (
                    0x0070_3820,
                    ParamSlot {
                        guard: 0x60,
                        param: 0x5c,
                        holder: 0x2c,
                    },
                ),
            ] {
                // the copy reads the original's own argument words: the
                // port reads the same words at the same address
                let ebp = t.frame_ebp(4);
                let args = ebp + 8;
                let bytes: Vec<u8> = words.iter().flat_map(|v| v.to_le_bytes()).collect();
                Mem::write_bytes(&mut t.b, args, &bytes);
                t.b.set_call_top(Some(args - 0x200));
                let what = format!("RadialBlur param {sym:#x} #{case}");
                t.check(&what, sym, Conv::Thiscall(this), &words, |m| {
                    Out::U32(post_set_param(m, slot, this, args))
                });
                t.b.set_call_top(None);
            }
        }
    });
}

#[test]
fn threads_match_the_original() {
    Twins::each(|t| {
        for case in 0..6u32 {
            t.reset();
            imp(t, ms::TERMINATE_THREAD_IAT, 2, true, 1);
            imp(t, ms::CLOSE_HANDLE_IAT, 1, true, 1);
            imp(t, ms::SET_THREAD_PRIORITY_IAT, 2, true, 1);
            imp(t, ms::RESUME_THREAD_IAT, 1, true, 0x55);
            t.stub(ms::OPERATOR_DELETE, false, 1, 0, 0);
            t.stub(ms::THREAD_CREATE, true, 4, 4, 0);
            let this = t.words(&[0, if case & 1 == 0 { 0 } else { 0x7a7a }, 9, 0, 0]);
            let what = format!("Sys::Thread dtor #{case}");
            t.check(&what, 0x0070_3990, Conv::Thiscall(this), &[], |m| {
                mb::thread_destruct(m, this);
                Out::None
            });
            t.put(this + 4, &[if case & 1 == 0 { 0 } else { 0x7a7a }]);
            let flags = case % 3;
            let what = format!("Sys::Thread deleting dtor #{case}");
            t.check(&what, 0x0070_39d0, Conv::Thiscall(this), &[flags], |m| {
                Out::U32(mb::thread_deleting_dtor(m, this, flags))
            });
            t.put(this + 4, &[0x4444]);
            let what = format!("JobWorker start #{case}");
            t.check(
                &what,
                0x0070_3a50,
                Conv::Thiscall(this),
                &[case, 0xa5a5],
                |m| Out::U32(mb::job_worker_start(m, this, case, 0xa5a5)),
            );
        }
    });
}

// ---- batch 3: SyncManager, the FMOD ring, post-effect setters, threads ----

/// A `SyncManager` with its columns and file table (128 words each).
fn sync_manager(t: &mut Twins, event: u32, enabled: u32) -> u32 {
    let platform = t.alloc(b"PS3\0");
    let this = t.zeros(0x40);
    t.put(this, &[platform]);
    let files = t.alloc(&[0x11; 0x200]);
    t.put(this + 4, &[files]);
    t.put(this + 0xc, &[event, 0, 0, 0, enabled]);
    for col in [0x24u32, 0x28, 0x2c, 0x30, 0x34, 0x38, 0x3c] {
        let c = t.zeros(0x200);
        t.put(this + col, &[c]);
    }
    this
}

/// A list file: the magic (`magic`), a count and records of `width`
/// words, every other one for `event`.
fn sync_list(t: &mut Twins, magic: [u8; 4], count: u32, width: u32, id_at: u32, event: u32) -> u32 {
    let mut w: Vec<u32> = vec![u32::from_le_bytes(magic), count];
    for r in 0..count {
        for k in 0..width {
            w.push(if k == id_at {
                if r % 2 == 0 { event } else { event + 1 }
            } else {
                0x100 * (r + 1) + k
            });
        }
    }
    t.words(&w)
}

fn sync_stubs(t: &mut Twins) {
    t.stub(sy::MEMSET, false, 3, 0, 0);
    t.stub(sy::FIOS_UNIFY, false, 3, 0, 0);
    t.stub(sy::ALIGNED_FREE, false, 1, 0, 0);
    t.stub(sy::DBG_PRINTF, false, 1, 0, 0);
    t.stub(sy::ASYNC_INSTANCE, false, 0, 0, 0x0a51_0000);
}

#[test]
fn sync_lists_match_the_original() {
    Twins::each(|t| {
        t.skip = 4;
        for case in 0..15u32 {
            for voice in [false, true] {
                t.reset();
                sync_stubs(t);
                let (fmt_words, width, id_at) = if voice { (3, 4, 1) } else { (3, 5, 0) };
                imp(t, sy::SPRINTF_IAT, fmt_words, false, 0);
                let magic: [u8; 4] = match case % 5 {
                    1 => *b"SX\0\0",
                    2 => *b"SL\0x",
                    3 => *b"SLx\0",
                    _ => *b"SL\0\0",
                };
                let count = [1u32, 4, 7][(case / 5) as usize];
                let list = sync_list(t, magic, count, width, id_at, 0x42);
                t.stub(
                    sy::READ_SYNC_FILE,
                    true,
                    1,
                    1,
                    if case == 13 { 0 } else { list },
                );
                let this = sync_manager(t, 0x42, 1);
                let (sym, what) = if voice {
                    (0x006f_aba0u32, format!("initSyncVoiceList #{case}"))
                } else {
                    (0x006f_aa30, format!("initSyncFileList #{case}"))
                };
                t.check_framed(&what, sym, Conv::Thiscall(this), &[], 0x204, |m, ebp| {
                    Out::U32(if voice {
                        sb3::init_sync_voice_list(m, ebp, this)
                    } else {
                        sb3::init_sync_file_list(m, ebp, this)
                    })
                });
            }
        }
        t.skip = 0;
    });
}

#[test]
fn sync_release_and_change_match_the_original() {
    Twins::each(|t| {
        for case in 0..6u32 {
            t.reset();
            sync_stubs(t);
            let this = sync_manager(t, 7, u32::from(case != 0));
            t.put(this + 0x10, &[3, 4]);
            let files = t.r32(this + 4);
            for i in 0..128 {
                t.put(
                    files + 4 * i,
                    &[if (i + case) % 3 == 0 { 0 } else { 0xf000 + i }],
                );
            }
            let what = format!("releaseSyncData #{case}");
            t.check(&what, 0x006f_aee0, Conv::Thiscall(this), &[], |m| {
                Out::U32(sb3::release_sync_data(m, this))
            });
            // syncDataChange: the two lists read nothing (null data), so
            // only the preset entries are loaded
            let reads: Vec<u32> = [0, 0]
                .into_iter()
                .chain((0..8).map(|i| 0x5f00 + i))
                .collect();
            t.stub_seq(sy::READ_SYNC_FILE, 1, 1, &reads);
            let entries = [0u32, 1, 3, 0xffff_ffff, 2, 5][case as usize];
            t.put(this + 0x10, &[entries]);
            for col in [0x28u32, 0x30, 0x34, 0x38] {
                let c = t.r32(this + col);
                for i in 0..8 {
                    t.put(c + 4 * i, &[col * 0x100 + i]);
                }
            }
            // the list readers' sprintf (3 words) and the entries' (8)
            t.stub_import_widths(sy::SPRINTF_IAT, &[3, 3, 8], 0);
            t.skip = 4;
            let what = format!("syncDataChange #{case}");
            t.check_framed(
                &what,
                0x006f_af90,
                Conv::Thiscall(this),
                &[0x99],
                sy::CHANGE_FRAME,
                |m, ebp| Out::U32(sb3::sync_data_change(m, ebp, this)),
            );
            t.skip = 0;
        }
    });
}

#[test]
fn sync_data_change_task_matches_the_original() {
    Twins::each(|t| {
        for case in 0..3u32 {
            t.reset();
            sync_stubs(t);
            let this = sync_manager(t, 7, u32::from(case != 0));
            t.put(this + 0x10, &[case]);
            let reads: Vec<u32> = [0, 0, 0x5f01, 0x5f02].to_vec();
            t.stub_seq(sy::READ_SYNC_FILE, 1, 1, &reads);
            t.stub_import_widths(sy::SPRINTF_IAT, &[3, 3, 8], 0);
            let args = t.words(&[this, 0x99]);
            // syncDataChange's frame pointer is 0x10 below the task's; the
            // frames hold the original's saved registers, so only the
            // calls (their logged pointers) and memory are compared
            let ebp = t.frame_ebp(1);
            let low = 0x10 + sy::CHANGE_LIST_EBP + sy::LIST_FRAME;
            t.b.set_call_top(Some(ebp - low - 0x100));
            let what = format!("syncDataChangeAsyncTask #{case}");
            t.check(&what, 0x006f_b130, Conv::Cdecl, &[args], |m| {
                Out::U32(sb3::sync_data_change_task(m, ebp - 0x10, args))
            });
            t.b.set_call_top(None);
        }
    });
}
