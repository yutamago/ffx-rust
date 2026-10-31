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

// ---- round 2: the movie frame queue (`ffx_system::m5b_movie`) ----
//
// The movie player's queue and reader never ran in a coverage run: every
// branch runs on synthetic queues, and the snapshots repeat the cases over
// their memory.

use ffx_system::m5b_movie::{self as mov, bridge as mv};
use ffx_system::m5b_pc::{self as pcs, bridge as pcb};

/// The event methods as recording thiscall stubs (signal/close return 0
/// like the originals; initialize pops its flags word).
fn event_stubs(t: &mut Twins) {
    t.stub(mov::EVENT_CTOR, true, 0, 0, 0x0e00_0001);
    t.stub(mov::EVENT_INITIALIZE, true, 1, 1, 0x0e00_0002);
    t.stub(mov::EVENT_SIGNAL, true, 0, 0, 0);
    t.stub(mov::EVENT_CLOSE, true, 0, 0, 0);
    t.stub(mov::OPERATOR_DELETE, false, 1, 0, 0x0e00_0003);
}

/// `PEventFlagWin32_wait` as a stub that logs `ecx`, adds `step` to the
/// word at `limit` and after `calls` calls clears the running byte `run`.
fn wait_stub(t: &mut Twins, limit: u32, step: u32, calls: u32, run: u32) {
    let log_stub = t.stub_code(mov::EVENT_WAIT, true, 0, 0, 0);
    let ctr = t.words(&[calls]);
    let mut c = vec![0x81, 0x05];
    c.extend(limit.to_le_bytes());
    c.extend(step.to_le_bytes()); // add dword [limit], step
    c.extend([0xff, 0x0d]);
    c.extend(ctr.to_le_bytes()); // dec dword [ctr]
    c.extend([0x75, 0x07]); // jnz +7
    c.extend([0xc6, 0x05]);
    c.extend(run.to_le_bytes());
    c.push(0); // mov byte [run], 0
    c.push(0xe9);
    let here = t.cave_next + c.len() as u32 + 4;
    c.extend(log_stub.wrapping_sub(here).to_le_bytes()); // jmp log_stub
    let addr = t.cave_code(&c);
    t.jump(mov::EVENT_WAIT, addr);
}

/// A queue with ring size `size`, read index `r`, write index `w`, the
/// open and running bytes, random counters and a size word at `r`.
fn queue(t: &mut Twins, rng: &mut Rng, size: u32, r: u32, w: u32, open: bool, run: bool) -> u32 {
    let buf = t.zeros(size as usize + 0x20);
    let q = t.zeros(0x40);
    let words = [
        0x00b7_a9e4,
        buf,
        r,
        w,
        rng.word(),
        rng.word(),
        u32::from(open),
        0x0e1e_0001,
        1,
        0x0e1e_0002,
        1,
        u32::from(run),
        size,
        rng.below(9),
        rng.below(9) + 9,
        0,
    ];
    t.put(q, &words);
    let len = rng.below(size / 2 + 1);
    t.w32(buf + r, len);
    q
}

fn out_opt(v: Option<u32>) -> Out {
    v.map_or(Out::None, Out::U32)
}

#[test]
fn r2_data_frame_queue_simple_methods_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0201);
        for case in 0..40u32 {
            t.reset();
            event_stubs(t);
            let size = 0x40 + rng.below(4) * 0x10;
            let (r, w) = (rng.below(size) & !3, rng.below(size) & !3);
            let q = queue(t, &mut rng, size, r, w, case & 1 == 0, case & 2 == 0);
            let what = format!("queue #{case}");
            let (a, b, c, d) = (rng.word(), rng.word(), rng.word(), rng.word());
            t.check(
                &format!("{what} SetFields"),
                0x00a3_dd00,
                Conv::Thiscall(q),
                &[a, b, c, d],
                |m| Out::U32(mv::set_fields(m, q, [a, b, c, d])),
            );
            t.check(
                &format!("{what} Count"),
                0x00a3_dde0,
                Conv::Thiscall(q),
                &[],
                |m| Out::U32(mv::queue_count(m, q)),
            );
            // the reserve end and the front end around the indices, and
            // on them (the comparisons' equal cases)
            let fe = match case % 4 {
                0 => w,
                1 => w + size,
                _ => rng.below(size + 8),
            };
            let re = match case / 4 % 3 {
                0 => fe,
                1 if w > fe => fe + size,
                _ => rng.below(size + 8),
            };
            // (SetFields wrote over the write index)
            t.w32(q + 0xc, w);
            t.w32(q + 0x10, re);
            t.w32(q + 0x14, fe);
            t.check(
                &format!("{what} Pop"),
                0x00a3_ddc0,
                Conv::Thiscall(q),
                &[],
                |m| out_opt(mv::queue_pop(m, q)),
            );
            t.check(
                &format!("{what} Commit"),
                0x00a3_df00,
                Conv::Thiscall(q),
                &[],
                |m| Out::U32(mv::queue_commit(m, q)),
            );
            t.check(
                &format!("{what} Close"),
                0x00a3_dea0,
                Conv::Thiscall(q),
                &[],
                |m| Out::U32(mv::queue_close(m, q)),
            );
            let free = t.stub_code(0x0e0f_0001, false, 1, 0, 0x0e00_0004);
            t.w32(q + 0x3c, free);
            t.check(
                &format!("{what} Destroy"),
                0x00a3_df20,
                Conv::Thiscall(q),
                &[],
                |m| out_opt(mv::queue_destroy(m, q)),
            );
            let flags = case & 3;
            t.check(
                &format!("{what} delete"),
                0x00a3_dd60,
                Conv::Thiscall(q),
                &[flags],
                |m| Out::U32(mv::queue_deleting_dtor(m, q, flags)),
            );
            let fresh = t.alloc(&[0xa5; 0x40]);
            t.check(
                &format!("{what} ctor"),
                0x00a3_dd20,
                Conv::Thiscall(fresh),
                &[],
                |m| Out::U32(mv::queue_construct(m, fresh)),
            );
        }
    });
}

#[test]
fn r2_data_frame_queue_init_matches_the_original() {
    // A failed allocation is left out: the original then stores at
    // `size - 4` (no buffer) and faults there, as the port does.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0202);
        for case in 0..16u32 {
            t.reset();
            event_stubs(t);
            let block = t.alloc(&[0xa5; 0x200]);
            t.stub(mov::PHYRE_MALLOC, false, 1, 0, block);
            let mine = t.stub_code(0x0e0f_0002, false, 1, 0, block);
            let alloc = if case & 1 == 0 { 0 } else { mine };
            let free = if case & 2 == 0 { 0 } else { 0x0012_3450 };
            let q = t.zeros(0x40);
            let capacity = rng.below(0x100);
            let extra = rng.below(0x40);
            t.check(
                &format!("Init #{case}"),
                0x00a3_ddf0,
                Conv::Thiscall(q),
                &[capacity, extra, alloc, free],
                |m| Out::U32(mv::queue_init(m, q, capacity, extra, alloc, free)),
            );
        }
    });
}

#[test]
fn r2_data_frame_queue_front_and_reserve_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0203);
        for case in 0..120u32 {
            t.reset();
            event_stubs(t);
            let size = 0x40 + rng.below(4) * 0x10;
            let (r, w) = (rng.below(size - 4) & !3, rng.below(size - 4) & !3);
            let q = queue(t, &mut rng, size, r, w, true, case % 5 != 0);
            let reserve = case & 1 == 1;
            // waits move the other side's index on, or stop the queue
            let limit = q + if reserve { 8 } else { 0xc };
            let step = rng.below(3) * 0x10;
            let calls = 1 + rng.below(4);
            wait_stub(t, limit, step, calls, q + 0x2c);
            let what = format!("{} #{case}", if reserve { "Reserve" } else { "Front" });
            if reserve {
                let len = rng.below(size);
                t.check(&what, 0x00a3_dec0, Conv::Thiscall(q), &[len], |m| {
                    Out::U32(mv::queue_reserve(m, q, len))
                });
            } else {
                t.check(&what, 0x00a3_dd90, Conv::Thiscall(q), &[], |m| {
                    Out::U32(mv::queue_front(m, q))
                });
            }
            // the helper itself, with its own arguments
            t.reset();
            let out = t.zeros(4);
            // high bits on the indices: they show in `eax`
            let high = (case % 3) * 0x1234_5600;
            let lim = t.words(&[rng.below(size + 0x10) + high]);
            let start = rng.below(size + 0x10) + high;
            let len = rng.below(size);
            let ev = t.zeros(4);
            t.write(q + 0x2c, &[u8::from(case % 3 != 0)]);
            let step = rng.below(3) * 8;
            let calls = 1 + rng.below(3);
            wait_stub(t, lim, step, calls, q + 0x2c);
            let args = [out, start, len, lim, ev];
            t.check(
                &format!("{what} waitSpace"),
                0x00a3_dfb0,
                Conv::Thiscall(q),
                &args,
                |m| Out::U32(mv::queue_wait_space(m, q, out, start, len, lim, ev)),
            );
        }
    });
}

#[test]
fn r2_vbf_mkv_reader_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0204);
        for case in 0..24u32 {
            t.reset();
            t.stub(mov::STREAM_FILE_DTOR, true, 0, 0, 0x0e00_0011);
            t.stub(mov::OPERATOR_DELETE, false, 1, 0, 0x0e00_0012);
            t.stub(mov::MKV_READER_BASE_DTOR, true, 0, 0, 0x0e00_0013);
            let file = t.zeros(8);
            let block = if case % 4 == 1 { 0 } else { t.zeros(8) };
            t.stub(mov::OPERATOR_NEW, false, 1, 0, block);
            let made = if case % 8 == 2 { 0 } else { block };
            t.stub(mov::STREAM_FILE_CTOR, true, 5, 5, made);
            let size = rng.next();
            t.stub(mov::STREAM_FILE_SIZE, true, 0, 0, size);
            let has = case & 2 == 0;
            let this = t.words(&[0, 1, rng.next(), rng.next(), if has { file } else { 0 }]);
            let what = format!("reader #{case}");
            let a = if case & 4 == 0 { t.zeros(8) } else { 0 };
            let b = if case & 8 == 0 { t.zeros(8) } else { 0 };
            t.check(
                &format!("{what} Length"),
                0x00a3_e1b0,
                Conv::Thiscall(this),
                &[a, b],
                |m| Out::U32(mv::reader_length(m, this, a, b)),
            );
            let name = if case % 6 == 5 {
                0
            } else {
                t.alloc(b"movie.vbf\0")
            };
            t.check(
                &format!("{what} Open"),
                0x00a3_e1f0,
                Conv::Thiscall(this),
                &[name],
                |m| Out::U32(mv::reader_open(m, this, name)),
            );
            t.check(
                &format!("{what} Close"),
                0x00a3_e150,
                Conv::Thiscall(this),
                &[],
                |m| {
                    mv::reader_close(m, this);
                    Out::None
                },
            );
            t.w32(this + 0x10, if case & 1 == 0 { file } else { 0 });
            let flags = case & 3;
            t.check(
                &format!("{what} delete"),
                0x00a3_e0c0,
                Conv::Thiscall(this),
                &[flags],
                |m| Out::U32(mv::reader_deleting_dtor(m, this, flags)),
            );
            t.w32(this + 0x10, 0x0e1e_2001);
            t.check(
                &format!("{what} ctor"),
                0x00a3_e030,
                Conv::Thiscall(this),
                &[],
                |m| Out::U32(mv::reader_construct(m, this)),
            );
        }
    });
}

// ---- round 2: PC glue (`ffx_system::m5b_pc`) ----
//
// None of these ran in a coverage run except the job pushes and the focus
// handler's registration; synthetic objects reach every branch.

#[test]
fn r2_occluder_descriptor_methods_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0205);
        for case in 0..16u32 {
            t.reset();
            let r = rng.next();
            t.stub(pcs::OCCLUDER_GET, false, 1, 0, r);
            t.stub(pcs::PHYRE_PRINTF, false, 2, 0, r ^ 1);
            t.stub(pcs::PUSH_OBJECT, false, 3, 0, r ^ 2);
            t.stub(pcs::LUA_94AC00, false, 2, 0, r ^ 3);
            t.stub(pcs::LUA_PUSHNIL, false, 1, 0, r ^ 4);
            t.stub(pcs::REGISTRY_SET, true, 2, 2, r ^ 5);
            let (l, v) = (rng.word(), rng.word());
            let out = t.zeros(4);
            t.check("get", 0x00a4_24b0, Conv::Stdcall, &[v, out], |m| {
                Out::U32(pcb::occluder_get(m, v, out))
            });
            t.check("not supported", 0x00a4_24e0, Conv::Stdcall, &[l, v], |m| {
                Out::U32(pcb::not_supported(m))
            });
            let slot = t.words(&[if case & 1 == 0 { 0 } else { v | 1 }]);
            t.check("push slot", 0x00a4_2700, Conv::Stdcall, &[l, slot], |m| {
                Out::U32(pcb::push_slot(m, l, slot))
            });
            for sym in [0x00a4_2740u32, 0x00a4_2760] {
                t.check("push", sym, Conv::Stdcall, &[l, v], |m| {
                    Out::U32(pcb::push_value(m, l, v))
                });
            }
            let flag = match case % 4 {
                0 => 0,
                1 => 1,
                2 => 0x100,
                _ => rng.next(),
            };
            t.check(
                "registry set",
                0x00a4_27e0,
                Conv::Stdcall,
                &[v, flag],
                |m| Out::U32(pcb::registry_set(m, v, flag)),
            );
            let f = t.stub_code(0x0e0f_0003, false, 2, 0, r ^ 6);
            let this = t.zeros(0xa0);
            t.w32(this + 0x94, f);
            t.check(
                "call field",
                0x00a4_2810,
                Conv::Thiscall(this),
                &[l, v],
                |m| Out::U32(pcb::call_field(m, this, l, v)),
            );
        }
    });
}

#[test]
fn r2_pc_read_hooks_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0206);
        for case in 0..8u32 {
            t.reset();
            let (r1, r2) = (rng.next(), rng.next());
            t.stub(pcs::CD_READ_SECTOR, false, 5, 0, r1);
            t.stub(pcs::CD_FILE_SECTOR, false, 1, 0, r2);
            let base = t.rebase(pcs::CD_FILE_BASE);
            let b0 = rng.below(0x1000);
            t.w32(base, b0);
            let w: Vec<u32> = (0..5).map(|_| rng.word()).collect();
            t.check("read sector", 0x00a4_2870, Conv::Cdecl, &w, |m| {
                Out::U32(pcb::read_sector(m, w[0], w[1], w[2], w[3], w[4]))
            });
            let file = rng.below(0x4000) + case;
            let slot = t.b_stack(0x40);
            t.check("file sector", 0x00a4_2880, Conv::Cdecl, &[file], |m| {
                Out::U32(pcb::file_sector(m, file, slot))
            });
        }
    });
}

#[test]
fn r2_read_whole_file_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0207);
        for case in 0..16u32 {
            t.reset();
            let opened = case % 3 != 0;
            t.stub(pcs::SCE_OPEN, false, 2, 0, if opened { 7 } else { 0 });
            // the empty name's message takes one word
            t.stub(
                pcs::DBG_PRINTF,
                false,
                if case & 4 == 0 { 1 } else { 2 },
                0,
                1,
            );
            t.stub(pcs::MOVIE_ERR, false, 3, 0, 2);
            let size = match case % 4 {
                0 => 0,
                1 => rng.below(0x100),
                2 => 0xffff_fffb,
                _ => 0x8000_0001,
            };
            t.stub_seq(pcs::SCE_LSEEK, 3, 0, &[size, 0]);
            t.stub(pcs::USER_MALLOC, false, 1, 0, 0x0e00_0100);
            t.stub(pcs::FLUSH_CACHE, false, 1, 0, 3);
            t.stub(pcs::SCE_READ, false, 3, 0, size);
            t.stub(pcs::SCE_CLOSE, false, 1, 0, 0);
            let name = t.alloc(if case & 4 == 0 { b"\0" } else { b"map.bin\0" });
            let out = if case & 8 == 0 { t.zeros(4) } else { 0 };
            t.check(
                &format!("read whole file #{case}"),
                0x00a4_2a00,
                Conv::Cdecl,
                &[name, out],
                |m| Out::U32(pcb::read_whole_file(m, name, out)),
            );
        }
    });
}

#[test]
fn r2_focus_handler_matches_the_original() {
    Twins::each(|t| {
        for case in 0..96u32 {
            t.reset();
            let cam = t.zeros(0x340);
            t.w32(cam, [0, 1, 2][(case % 3) as usize]);
            t.write(cam + 0x338, &[u8::from(case / 3 % 4 == 1)]);
            t.write(cam + 0x33c, &[u8::from(case / 3 % 4 == 2)]);
            let made = t.zeros(0x340);
            // without a camera it is made on demand; a failed `new` makes
            // the original read through 0 and fault, so only the
            // successful one runs
            let have = case / 12 % 2 == 0;
            t.stub(pcs::OPERATOR_NEW, false, 1, 0, made);
            t.stub(pcs::FREE_CAMERA_CTOR, true, 0, 0, made);
            let cell = t.rebase(pcs::FREE_CAMERA);
            t.w32(cell, if have { cam } else { 0 });
            let esc = t.zeros(0x80);
            t.write(esc + 0x70, &[u8::from(case / 24 % 2 == 1)]);
            let esc_cell = t.rebase(pcs::ESC_MENU);
            t.w32(esc_cell, if case % 5 == 0 { 0 } else { esc });
            let fmv = t.zeros(0x700);
            t.write(fmv + 0x6fc, &[u8::from(case % 7 == 3)]);
            let manager = if case % 11 == 4 { 0 } else { fmv };
            t.stub(pcs::FMV_MANAGER, false, 0, 0, manager);
            t.stub(pcs::FMV_PAUSE, true, 0, 0, 0x0e00_0200);
            t.stub(pcs::PAUSE_ALL_SOUNDS, false, 1, 0, 0x0e00_0201);
            let this = t.words(&[0x00b7_af94, if case / 48 == 0 { 0 } else { 0x0101 }]);
            let what = format!("focus #{case}");
            if case & 1 == 0 {
                t.check(
                    &format!("{what} lost"),
                    0x00a4_4890,
                    Conv::Thiscall(this),
                    &[],
                    |m| Out::U32(pcb::focus_lost(m, this)),
                );
            } else {
                t.check(
                    &format!("{what} gained"),
                    0x00a4_4960,
                    Conv::Thiscall(this),
                    &[],
                    |m| Out::U32(pcb::focus_gained(m, this)),
                );
            }
        }
        t.reset();
        t.stub(pcs::HANDLER_INSERT, false, 1, 0, 0x0e00_0202);
        let h = t.rebase(pcs::FOCUS_HANDLER);
        t.write(h + 4, &[1]);
        t.check("register", 0x00a4_4a30, Conv::Cdecl, &[], |m| {
            Out::U32(pcb::register_focus_handler(m))
        });
    });
}

/// A scheduler with queue `queue` set up: handles, a free ring of entries
/// and a run ring. The indices stay 0..=15: the game keeps them in 0..=7,
/// and a negative one (the signed modulo's other branch) would index
/// before the ring in the original as in the port.
fn scheduler(t: &mut Twins, rng: &mut Rng, queue: u32) -> u32 {
    let s = t.zeros(0x1e8 * 2 + 0x40);
    let q = s + queue * 0x1e8;
    for i in 0..8 {
        let e = t.zeros(0x30);
        t.w32(q + 0x1d0 + 4 * i, e);
    }
    let free = rng.below(16);
    t.w32(q + 0x1f0, free);
    let words = [
        (0x1f8, rng.below(9)),
        (0x1fc, 0x0e1e_1001),
        (
            0x224,
            rng.below(8) + if rng.chance(3) { 0xffff_fff0 } else { 0 },
        ),
        (0x228, rng.below(8)),
        (0x22c, 0x0e1e_1002),
        (0x230, 0x0e1e_1003),
        (0x234, 0x0e1e_1004),
    ];
    for (off, v) in words {
        t.w32(q + off, v);
    }
    s
}

#[test]
fn r2_job_pushes_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0208);
        for case in 0..24u32 {
            t.reset();
            imp(t, pcs::WAIT_FOR_SINGLE_OBJECT_IAT, 2, true, 0);
            imp(t, pcs::RELEASE_SEMAPHORE_IAT, 3, true, 0x0e00_0300 + case);
            let queue = case % 2;
            let s = scheduler(t, &mut rng, queue);
            let (f, a, b, c) = (rng.next(), rng.word(), rng.word(), rng.word());
            let slot = t.b_stack(0x40);
            if case & 2 == 0 {
                t.check(
                    "push job 2",
                    0x00a4_4a50,
                    Conv::Thiscall(s),
                    &[f, a, b, queue],
                    |m| Out::U32(pcb::push_job(m, s, f, &[a, b], queue, slot)),
                );
            } else {
                t.check(
                    "push job 3",
                    0x0071_e720,
                    Conv::Thiscall(s),
                    &[f, a, b, c, queue],
                    |m| Out::U32(pcb::push_job(m, s, f, &[a, b, c], queue, slot)),
                );
            }
            let s = scheduler(t, &mut rng, 0);
            let cell = t.rebase(pcs::SCHEDULER);
            t.w32(cell, s);
            t.check("schedule cloth", 0x00a4_4b40, Conv::Cdecl, &[a, b], |m| {
                let job = Mem::read(&*m, Mem::rebase(&*m, pcs::CLOTH_JOB_PUSH));
                Out::U32(pcb::schedule_cloth(m, a, b, job, slot))
            });
            t.check(
                "schedule calc middle",
                0x00a4_4b90,
                Conv::Cdecl,
                &[a, b, c],
                |m| {
                    let job = Mem::read(&*m, Mem::rebase(&*m, pcs::CALC_MIDDLE_JOB_PUSH));
                    Out::U32(pcb::schedule_calc_middle(m, [a, b, c], job, slot))
                },
            );
            t.check("wait calc middle", 0x00a4_4b80, Conv::Cdecl, &[], |m| {
                ffx_system::m5a_platform::bridge::wait_cloth(m);
                Out::None
            });
        }
    });
}

#[test]
fn r2_str_replace_matches_the_original() {
    let cases: &[(&[u8], &[u8], &[u8])] = &[
        (b"", b"a", b"b"),
        (b"abc", b"x", b"yy"),
        (b"abc", b"b", b"XYZ"),
        (b"aaaa", b"a", b""),
        (b"aaaa", b"aa", b"b"),
        (b"%s/%s.bin", b"%s", b"field"),
        (b"host0:/ffx/master/", b"/", b"\\\\"),
        (b"abab", b"ab", b"ab"),
    ];
    let z = |b: &[u8]| {
        let mut v = b.to_vec();
        v.push(0);
        v
    };
    Twins::each(|t| {
        for (i, &(s, what, with)) in cases.iter().enumerate() {
            for fail in 0..3u32 {
                t.reset();
                let buf = t.zeros(0x100);
                let grown = t.zeros(0x100);
                t.stub(
                    pcs::USER_MALLOC,
                    false,
                    1,
                    0,
                    if fail == 1 { 0 } else { buf },
                );
                t.stub(
                    pcs::USER_REALLOC,
                    false,
                    2,
                    0,
                    if fail == 2 { 0 } else { grown },
                );
                t.stub(pcs::USER_FREE, false, 1, 0, 0);
                let (ps, pw, pt) = (t.alloc(&z(s)), t.alloc(&z(what)), t.alloc(&z(with)));
                t.check(
                    &format!("str_replace #{i}/{fail}"),
                    0x00a4_4bd0,
                    Conv::Cdecl,
                    &[ps, pw, pt],
                    |m| Out::U32(pcb::str_replace(m, ps, pw, pt)),
                );
            }
        }
    });
}

// ---- round 2: the HDD layer and one-jump thunks (`ffx_system::m5b_hdd`) ----
//
// The PS2 HDD-install screens never ran in a coverage run (the PC build
// installs nothing): every branch runs on synthetic globals, the
// snapshots repeat the cases over their memory.

use ffx_system::m5b_hdd::{self as hdc, bridge as hdb};

type Thunk0 = fn(&mut G) -> u32;
type Thunk1 = fn(&mut G, u32) -> u32;

/// The callees in `only` the thunks and the HDD layer reach, as
/// recording cdecl stubs returning `r ^ k` (the cave holds about 30).
fn hdd_stubs(t: &mut Twins, r: u32, only: &[u32]) {
    let list = [
        (hdc::SCENARIO_FLAG, 0),
        (hdc::SPHERE_MONITOR, 0),
        (hdc::RMODE_GET, 1),
        (hdc::HEAD_SECTORS, 1),
        (hdc::SET_RMODE, 2),
        (hdc::PAY_GIL, 1),
        (hdc::HDD_STATUS, 0),
        (hdc::SET_LANG, 1),
        (hdc::ALBHED_LOADED, 0),
        (hdc::SAVE_BUSY, 0),
        (hdc::HDD_MOUNT, 0),
        (hdc::REFRESH_HDD_MODE, 0),
        (hdc::CONFIG_ADDRESS, 0),
        (hdc::ASSERT_TARGET, 1),
        (hdc::MODULE_FILE_SECTOR, 2),
        (hdc::QUEUE_SECTOR_READ, 3),
        (hdc::MEM_SET_ALIGN, 1),
        (hdc::SET_GLOBAL_A018, 1),
        (hdc::REGIST_BG_THREAD, 1),
        (hdc::SIGNAL_BG_THREAD, 0),
        (hdc::DISK_OPEN_CHECK_SET, 1),
        (hdc::READ_THREAD_INIT, 0),
        (hdc::DRAW_BAR, 2),
        (hdc::USER_FREE, 1),
        (hdc::SCE_READ, 3),
        (hdc::CD_RESET, 0),
        (hdc::READ_DIRECT, 4),
        (hdc::MEMSET, 3),
        (hdc::PACKET_PTR, 0),
        (hdc::DMA_GET_CHAN, 1),
        (hdc::TEXTURED_SPRITE, 7),
        (hdc::FLUSH_CACHE, 1),
        (hdc::DMA_SEND, 2),
        (hdc::GS_SYNC_PATH, 2),
        (hdc::USER_MALLOC, 1),
        (hdc::DBG_PRINTF, 1),
    ];
    for (k, (sym, n)) in list.into_iter().enumerate() {
        if only.contains(&sym) {
            t.stub(sym, false, n, 0, r ^ (k as u32));
        }
    }
}

#[test]
fn r2_hdd_thunks_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0301);
        for case in 0..16u32 {
            t.reset();
            let r = rng.next();
            hdd_stubs(
                t,
                r,
                &[
                    hdc::SCENARIO_FLAG,
                    hdc::SPHERE_MONITOR,
                    hdc::RMODE_GET,
                    hdc::HEAD_SECTORS,
                    hdc::SET_RMODE,
                    hdc::PAY_GIL,
                    hdc::HDD_STATUS,
                    hdc::SET_LANG,
                    hdc::ALBHED_LOADED,
                    hdc::SAVE_BUSY,
                    hdc::HDD_MOUNT,
                    hdc::REFRESH_HDD_MODE,
                    hdc::CONFIG_ADDRESS,
                    hdc::ASSERT_TARGET,
                    hdc::MODULE_FILE_SECTOR,
                    hdc::QUEUE_SECTOR_READ,
                    hdc::DISK_OPEN_CHECK_SET,
                    hdc::SET_GLOBAL_A018,
                    hdc::READ_THREAD_INIT,
                    hdc::DBG_PRINTF,
                    hdc::PACKET_PTR,
                    hdc::DMA_GET_CHAN,
                    hdc::TEXTURED_SPRITE,
                    hdc::FLUSH_CACHE,
                    hdc::DMA_SEND,
                    hdc::GS_SYNC_PATH,
                ],
            );
            let (a, b) = (rng.word(), rng.word());
            let none: [(u32, Thunk0); 9] = [
                (0x0077_26d0, |m| hdb::scenario_flag(m)),
                (0x0077_2730, |m| hdb::sphere_monitor(m)),
                (0x0077_3b80, |m| hdb::hdd_status(m)),
                (0x0082_2540, |m| hdb::albhed_loaded(m)),
                (0x0082_2560, |m| hdb::save_busy(m)),
                (0x0077_89d0, |m| hdb::head_init(m)),
                (0x0077_92c0, |m| hdb::install_end(m)),
                (0x0077_66e0, |m| hdb::draw_background(m)),
                (0x0077_6740, |m| hdb::draw_wall(m)),
            ];
            let work = t.zeros(0x50);
            let cell = t.rebase(hdc::MENU_WORK);
            t.w32(cell, if case & 1 == 0 { work } else { rng.next() });
            for (sym, f) in none {
                t.check(
                    &format!("thunk {sym:#x} #{case}"),
                    sym,
                    Conv::Cdecl,
                    &[],
                    |m| Out::U32(f(m)),
                );
            }
            let one: [(u32, Thunk1); 4] = [
                (0x0077_27f0, |m, x| hdb::rmode_get(m, x)),
                (0x0077_3530, |m, x| hdb::head_sectors(m, x)),
                (0x0077_3990, |m, x| hdb::pay_gil(m, x)),
                (0x0088_7ca0, |m, x| hdb::set_lang(m, x)),
            ];
            for (sym, f) in one {
                t.check(
                    &format!("thunk {sym:#x} #{case}"),
                    sym,
                    Conv::Cdecl,
                    &[a],
                    |m| Out::U32(f(m, a)),
                );
            }
            t.check("set rmode", 0x0077_3940, Conv::Cdecl, &[a, b], |m| {
                Out::U32(hdb::set_rmode(m, a, b))
            });
            t.check(
                "queue module read",
                0x0076_cf60,
                Conv::Cdecl,
                &[a, b],
                |m| Out::U32(hdb::queue_module_read(m, a, b)),
            );
            let on = [0, 1, 0x100, a][(case % 4) as usize];
            t.check("set hdd mode", 0x0077_3c60, Conv::Cdecl, &[on], |m| {
                Out::U32(hdb::set_hdd_mode(m, on))
            });
            // TkyiAssert's target is a bare `ret`: both paths leave `eax`
            let off = t.rebase(hdc::ASSERT_OFF);
            t.w32(off, u32::from(case & 2 == 0));
            t.check("TkyiAssert", 0x0077_7680, Conv::Cdecl, &[a], |m| {
                hdb::assert(m, a);
                Out::None
            });
            t.check("heads assert", 0x0077_8980, Conv::Cdecl, &[], |m| {
                Out::U32(hdb::heads_assert(m))
            });
            let block = t.zeros(4);
            t.stub(hdc::RETURN_ADDRESS, false, 0, 0, block);
            let n = [0, 1, 2, a][(case % 4) as usize];
            let slot = t.b_stack(0x40);
            t.check("set block flag", 0x0076_cf80, Conv::Cdecl, &[n], |m| {
                Out::U32(hdb::set_block_flag(m, n, slot))
            });
        }
    });
}

#[test]
fn r2_hdd_state_readers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0302);
        for case in 0..48u32 {
            t.reset();
            let r = rng.next();
            hdd_stubs(
                t,
                r,
                &[
                    hdc::DRAW_BAR,
                    hdc::PACKET_PTR,
                    hdc::DMA_GET_CHAN,
                    hdc::TEXTURED_SPRITE,
                    hdc::FLUSH_CACHE,
                    hdc::DMA_SEND,
                    hdc::GS_SYNC_PATH,
                    hdc::MEM_SET_ALIGN,
                    hdc::USER_MALLOC,
                    hdc::SET_GLOBAL_A018,
                    hdc::REGIST_BG_THREAD,
                    hdc::SIGNAL_BG_THREAD,
                ],
            );
            let buttons = t.rebase(hdc::PAD_BUTTONS);
            t.w32(buttons, rng.next());
            let held = t.rebase(hdc::PAD_HELD);
            t.w32(held, if case & 1 == 0 { 0 } else { rng.next() });
            if case & 2 == 0 {
                t.w32(buttons, rng.next() & 0xffff_0fff);
            }
            if case % 16 == 6 {
                // bit 15 alone
                t.w32(buttons, 0x8000);
            }
            let on = t.rebase(hdc::PAD_ON);
            t.write(on, &[u8::from(case & 4 != 0)]);
            let w78 = t.rebase(hdc::PAD_WORD_78);
            t.w32(w78, rng.next());
            let w518 = t.rebase(hdc::WORD_A518);
            t.w32(w518, rng.next());
            let state = t.rebase(hdc::INSTALL_STATE);
            t.write(state, &[[0, 1, 2, 0xfe, 0xff, 0x80][(case % 6) as usize]]);
            let total = t.rebase(hdc::TOTAL);
            let done = t.rebase(hdc::DONE);
            let tot = [0, 1000, 0xffff_fff0, 7][(case / 6 % 4) as usize];
            t.w32(total, tot);
            t.w32(
                done,
                [0, 500, 1000, 3000, 0x0100_0000, 0xffff_fffe][(case % 6) as usize],
            );
            let last = t.rebase(hdc::LAST_PROGRESS);
            t.w32(last, if case & 8 == 0 { 0 } else { 32 });
            let readers: [(u32, &str, Thunk0); 7] = [
                (0x0077_2800, "bit 5", |m| hdb::pad_bit(m, 5)),
                (0x0077_2970, "bit 6", |m| hdb::pad_bit(m, 6)),
                (0x0077_2860, "shoulder", |m| hdb::pad_any_shoulder(m)),
                (0x0077_29d0, "word 78", |m| hdb::pad_word(m)),
                (0x0077_93a0, "word a518", |m| hdb::word_a518(m)),
                (0x0077_9880, "install result", |m| hdb::install_result(m)),
                (0x0077_9290, "install percent", |m| hdb::install_percent(m)),
            ];
            for (sym, what, f) in readers {
                t.check(&format!("{what} #{case}"), sym, Conv::Cdecl, &[], |m| {
                    Out::U32(f(m))
                });
            }
            t.check(
                &format!("draw progress #{case}"),
                0x0077_9450,
                Conv::Cdecl,
                &[],
                |m| Out::U32(hdb::draw_progress(m)),
            );
            let err = t.rebase(hdc::HDD_ERR);
            t.write(err, &[0x5a]);
            let started = case % 6 != 0;
            t.write(state, &[u8::from(started)]);
            t.check(
                &format!("install start #{case}"),
                0x0077_9120,
                Conv::Cdecl,
                &[],
                |m| hdb::install_start(m).map_or(Out::None, Out::U32),
            );
        }
    });
}

#[test]
fn r2_hdd_mount_and_heads_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0303);
        for case in 0..24u32 {
            t.reset();
            let r = rng.next();
            hdd_stubs(
                t,
                r,
                &[
                    hdc::ASSERT_TARGET,
                    hdc::DBG_PRINTF,
                    hdc::USER_FREE,
                    hdc::SCE_READ,
                    hdc::CD_RESET,
                    hdc::READ_DIRECT,
                    hdc::MEMSET,
                ],
            );
            // umount's result and the asserts' switch
            t.stub(
                hdc::UMOUNT,
                false,
                1,
                0,
                if case & 1 == 0 { 0 } else { r | 1 },
            );
            let mounted = t.rebase(hdc::HDD_MOUNTED);
            t.write(mounted, &[u8::from(case & 2 == 0)]);
            t.check(
                &format!("unmount #{case}"),
                0x0077_9730,
                Conv::Cdecl,
                &[],
                |m| Out::U32(hdb::unmount(m)),
            );
            // the heads: sizes, a count (negative too), which are installed
            let heads = t.rebase(hdc::HEADS);
            for i in 0..8 {
                t.w32(heads + 4 + 12 * i, rng.next() >> (case % 8));
            }
            let count = t.rebase(hdc::HEAD_COUNT);
            t.w32(count, [0, 1, 5, 8, 0xffff_ffff, 3][(case % 6) as usize]);
            let installed: Vec<u32> = (0..8).map(|_| rng.below(2)).collect();
            t.stub_seq(hdc::RMODE_GET, 1, 0, &installed);
            t.check(
                &format!("installed MB #{case}"),
                0x0077_8f60,
                Conv::Cdecl,
                &[],
                |m| Out::U32(hdb::installed_mb(m, 1)),
            );
            t.stub_seq(hdc::RMODE_GET, 1, 0, &installed);
            t.check(
                &format!("installed MB 2 #{case}"),
                0x0077_8fb0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(hdb::installed_mb(m, 2)),
            );
            // the format: a negative result printed
            let fr = [0, 1, 0xffff_fffb, 0x8000_0000][(case % 4) as usize];
            t.stub(hdc::SCE_FORMAT, false, 4, 0, fr);
            // the message takes one word, the failure two
            t.stub_two(hdc::DBG_PRINTF, 1, 2, r ^ 0x56);
            t.check(
                &format!("format #{case}"),
                0x0077_89a0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(hdb::format(m)),
            );
            // the trophy manager
            let cell = t.rebase(hdc::TROPHY_MANAGER);
            t.w32(cell, if case & 1 == 0 { 0 } else { rng.next() });
            t.check(
                &format!("trophy destroy #{case}"),
                0x007a_0cb0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(hdb::trophy_destroy(m)),
            );
            let head = rng.below(8);
            let args = [rng.word(), rng.next(), rng.next(), head, rng.next()];
            t.check(
                &format!("read no wait #{case}"),
                0x0077_85e0,
                Conv::Cdecl,
                &args,
                |m| Out::U32(hdb::read_no_wait(m, args[1], args[2], args[3], args[4])),
            );
            // the disk check: changed state 0, -1 or other; ready or not
            let changed = [0, u32::MAX, 1, 5][(case % 4) as usize];
            t.stub(hdc::DISK_CHANGED, false, 0, 0, changed);
            t.stub(
                hdc::DISK_READY,
                false,
                1,
                0,
                if case & 4 == 0 { 2 } else { 1 },
            );
            t.stub(hdc::DBG_PRINTF, false, 1, 0, r ^ 0x57);
            let pend = t.rebase(hdc::PENDING_READ);
            t.put(pend, &[rng.next(), rng.next(), rng.next()]);
            t.check(
                &format!("check disk #{case}"),
                0x0076_d5a0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(hdb::check_disk(m)),
            );
            t.stub(hdc::USER_MALLOC, false, 1, 0, rng.next());
            let w = t.rebase(hdc::WORD_8AB8);
            t.w32(w, 0x5a5a_5a5a);
            t.stub(hdc::DBG_PRINTF, false, 2, 0, r ^ 0x58);
            t.check(
                &format!("read buffer #{case}"),
                0x0076_dc70,
                Conv::Cdecl,
                &[],
                |m| Out::U32(hdb::alloc_read_buffer(m)),
            );
        }
    });
}

// ---- round 3: menu wrappers and table entries (`ffx_system::m5b_menu`) ----
//
// The packet wrappers ran in the menus; the table entries (the HDD menu,
// the option menu, the sphere grid's) mostly never did. Every branch runs
// on synthetic globals; the snapshots repeat the cases over their memory.

use ffx_system::m5b_menu::{self as mnc, bridge as mnb};

/// `TOCtrlPkt`'s state by `case % 4`: packets not added; added with the
/// cursor below the limit; added with the limit 0 (`TODrawKick`); added
/// with the limit at `cursor`, the cursor the call leaves (`TODrawKick`).
fn ctrl_state(t: &mut Twins, case: u32, cursor: u32) {
    let (add, limit) = [(0, 0), (1, 0xffff_ffff), (1, 0), (1, cursor)][(case % 4) as usize];
    t.g32(mnc::ADD_PKT, add);
    t.g32(mnc::PKT_LIMIT, limit);
}

#[test]
fn r3_menu_packet_wrappers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0401);
        for case in 0..12u32 {
            t.reset();
            let r = rng.next();
            for (k, (sym, n)) in [
                (mnc::VALUE_FONT_M, 5),
                (mnc::FONT_8F2BF0, 5),
                (mnc::EASY_SHAPE, 8),
                (mnc::BASE_MES_FONT, 5),
                (mnc::MES_FONT_8ED120, 6),
                (mnc::STR_FONT_M, 9),
                (mnc::DRAW_KICK, 0),
                (mnc::CHANGE_SEQ_ANM, 3),
                (mnc::TRANS_IMAGE_HL, 7),
            ]
            .into_iter()
            .enumerate()
            {
                // one cursor for all, so that the limit can equal it
                t.stub(
                    sym,
                    false,
                    n,
                    0,
                    if sym == mnc::DRAW_KICK {
                        r ^ (k as u32 * 0x111)
                    } else {
                        r
                    },
                );
            }
            let cell = t.rebase(mnc::PKT_CURSOR);
            t.w32(cell, r);
            ctrl_state(t, case, r);
            let w: Vec<u32> = (0..7).map(|_| rng.word()).collect();
            let w4 = [w[0], w[1], w[2], w[3]];
            t.check("value font", 0x0090_42e0, Conv::Cdecl, &w4, |m| {
                Out::U32(mnb::value_font_m_clut_right(m, w4))
            });
            t.check("font 902810", 0x0090_2810, Conv::Cdecl, &w4, |m| {
                Out::U32(mnb::font_902810(m, w4))
            });
            let w7 = [w[0], w[1], w[2], w[3], w[4], w[5], w[6]];
            t.check("shape", 0x0090_3b70, Conv::Cdecl, &w7, |m| {
                Out::U32(mnb::shape_xywhc2(m, w7))
            });
            t.check("base mes font", 0x0090_1cb0, Conv::Cdecl, &w4, |m| {
                Out::U32(mnb::base_mes_font(m, w4))
            });
            let w5 = [w[0], w[1], w[2], w[3], w[4]];
            t.check("mes font", 0x0090_1d10, Conv::Cdecl, &w5, |m| {
                Out::U32(mnb::mes_font_901d10(m, w5))
            });
            t.check("str font 4", 0x0090_1f80, Conv::Cdecl, &w4, |m| {
                Out::U32(mnb::str_font_m4(m, w4))
            });
            t.check("easy str font", 0x0090_1f40, Conv::Cdecl, &w4[..3], |m| {
                Out::U32(mnb::easy_str_font_m(m, w[0], w[1], w[2]))
            });
            let prim = t.zeros(0x20);
            let halves: Vec<u8> = (0..0x20).map(|_| rng.next() as u8).collect();
            t.write(prim, &halves);
            t.check("seq prim func", 0x008f_e040, Conv::Cdecl, &[prim], |m| {
                Out::U32(mnb::seq_prim_func(m, prim))
            });
            t.check("seq prim mes", 0x008f_e080, Conv::Cdecl, &[prim], |m| {
                Out::U32(mnb::seq_prim_mes(m, prim))
            });
            t.check("small face", 0x0090_1160, Conv::Cdecl, &[], |m| {
                Out::U32(mnb::trans_small_face(m))
            });
        }
    });
}

#[test]
fn r3_menu_str_font_m_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0402);
        for (case, text) in [&b"\0"[..], b"A\0", b"Gil 123\0", b"\xff\x01\0"]
            .into_iter()
            .enumerate()
        {
            t.reset();
            t.stub(mnc::IDENTITY, false, 1, 0, rng.next());
            t.stub(mnc::FONT_M_ASCII, false, 9, 0, rng.next());
            let s = t.alloc(text);
            let out = t.zeros(0x80);
            let rest: [u32; 6] = std::array::from_fn(|_| rng.word());
            let cur = rng.next();
            let mut args = vec![cur, s, out];
            args.extend(rest);
            t.check(
                &format!("str font #{case}"),
                0x008e_dfc0,
                Conv::Cdecl,
                &args,
                |m| Out::U32(mnb::str_font_m(m, cur, s, out, rest)),
            );
        }
    });
}

#[test]
fn r3_menu_table_entries_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0403);
        for case in 0..8u32 {
            t.reset();
            let r = rng.next();
            for (k, (sym, n)) in [
                (mnc::CD_SIZE_KB, 0),
                (mnc::MC_SET_DEBUG_LOCK, 1),
                (mnc::PKT_8EB4E0, 1),
                (mnc::DRAW_KICK, 0),
                (mnc::CLOSE_WINDOW, 0),
                (mnc::CLOSE_WINDOW3, 0),
                (mnc::CLOSE_WINDOW2, 0),
                (mnc::CLOSE_SUB_CMD, 0),
                (mnc::SEQ_STOP_SCENE, 2),
                (mnc::ITEM_LIST_MAX, 0),
                (mnc::SHADE_RATIO_DIFF, 1),
                (mnc::SHADE_RATIO_STEP, 1),
                (mnc::OPTION_MENU, 1),
                (mnc::MAIN_HELP, 1),
                (mnc::SET_HELP, 1),
                (mnc::SCREEN_X2, 1),
                (mnc::IGNORE_JOIN, 1),
                (mnc::SEQ_INIT_SCENE, 1),
                (mnc::SEQ_STOP_EXEC, 1),
                (mnc::ABM_EXIT, 0),
                (mnc::ET_CHR_SHOW, 1),
                (mnc::TEX_VDISP_LOAD, 1),
                (mnc::PPP_INIT_DATA, 3),
                (mnc::FONT_1612_EDGE, 2),
                (mnc::DBG_PRINTF, 1),
                (mnc::GET_FFX_LANG_JMP, 0),
            ]
            .into_iter()
            .enumerate()
            {
                t.stub(sym, false, n, 0, r ^ (k as u32 * 0x1001));
            }
            let consts: [(u32, u32); 5] = [
                (0x008a_c7a0, mnc::STR_B62CF4),
                (0x008b_7c90, mnc::STR_B62CF4),
                (0x008d_f680, mnc::STR_B62CF4),
                (0x008f_a4e0, mnc::STR_B66B30),
                (0x0090_6c10, mnc::STR_B66B30),
            ];
            for (sym, v) in consts {
                t.check("constant", sym, Conv::Cdecl, &[], |m| {
                    Out::U32(mnb::constant(m, v))
                });
            }
            let none: [(u32, Thunk0); 14] = [
                (0x008b_34f0, |m| mnb::update_save_need(m)),
                (0x008b_3600, |m| mnb::mc_unlock(m)),
                (0x008c_66c0, |m| mnb::close_item_windows(m)),
                (0x008c_6700, |m| mnb::item_list_max(m)),
                (0x008e_1910, |m| mnb::option_enter(m)),
                (0x008e_1950, |m| mnb::option_open(m)),
                (0x008e_19b0, |m| mnb::option_close(m)),
                (0x008e_19d0, |m| mnb::option_leave(m)),
                (0x008e_27b0, |m| mnb::abm_exit(m)),
                (0x008e_27c0, |m| mnb::abm_enter(m)),
                (0x008e_27d0, |m| mnb::abm_leave(m)),
                (0x0076_ef90, |m| mnb::print_b53060(m)),
                (0x008d_fe10, |m| mnb::get_ffx_lang(m)),
                (0x008b_f0a0, |m| mnb::hdd_menu_func(m)),
            ];
            // the option window (or none), the words the entries write
            let win = t.alloc(&[0xa5; 0x50]);
            let wcell = t.rebase(mnc::OPT_WINDOW);
            t.w32(wcell, if case & 1 == 0 { win } else { 0 });
            for sym in [
                mnc::OPT_WORD_44,
                mnc::OPT_WORD_54,
                mnc::OPT_WORD_5C,
                mnc::OPT_WORD_60,
                mnc::WORD_134085C,
                mnc::BYTE_1340858,
                mnc::WORD_186A22C,
                mnc::WORD_186A230,
            ] {
                let at = t.rebase(sym);
                t.w32(at, 0x5a5a_5a5a);
            }
            let item = t.rebase(mnc::OPT_ITEM);
            t.w32(item, rng.below(8));
            // the sphere grid's leave: its once-flag, the record's first byte
            t.g32(mnc::ABM_RUNNING, 0x5a5a_5a5a);
            t.g32(mnc::ABM_LEAVE_DONE, u32::from(case & 2 != 0));
            let mut rec = [0xa5u8; 0x10];
            rec[0] = u8::from(case & 4 != 0);
            let rec = t.alloc(&rec);
            t.g32(mnc::ABM_PPP_RECORD, rec);
            // PKT_8EB4E0 is stub 2 above
            ctrl_state(t, case, r ^ 0x2002);
            t.g32(mnc::PKT_CURSOR, rng.next());
            // the two HDD menu pointers: recording stubs
            let f1 = t.stub_code(0x0e0f_0101, false, 0, 0, r ^ 0x77);
            let f2 = t.stub_code(0x0e0f_0102, false, 0, 0, r ^ 0x78);
            let p1 = t.rebase(mnc::HDD_MENU_ENTER);
            t.patch(p1, &f1.to_le_bytes());
            let p2 = t.rebase(mnc::HDD_MENU_FUNC);
            t.patch(p2, &f2.to_le_bytes());
            for (sym, f) in none {
                t.check(
                    &format!("entry {sym:#x} #{case}"),
                    sym,
                    Conv::Cdecl,
                    &[],
                    |m| Out::U32(f(m)),
                );
            }
            t.check("hdd menu enter", 0x008b_f050, Conv::Cdecl, &[], |m| {
                Out::U32(mnb::hdd_menu_enter(m))
            });
            let w6: [u32; 6] = std::array::from_fn(|_| rng.word());
            t.check("pkt 8eb4e0", 0x008b_e250, Conv::Cdecl, &[], |m| {
                Out::U32(mnb::pkt_8eb4e0(m))
            });
            t.check("font 1612 edge", 0x0090_6260, Conv::Cdecl, &w6[..2], |m| {
                Out::U32(mnb::font_1612_edge(m, w6[0], w6[1]))
            });
            // the callback object
            let cb = t.stub_code(0x0e0f_0103, false, 1, 0, r ^ 0x79);
            let obj = t.words(&[0, 0, cb]);
            let ocell = t.rebase(mnc::CALLBACK_OBJECT);
            t.w32(ocell, obj);
            t.check("callback one", 0x008a_bb00, Conv::Cdecl, &[], |m| {
                Out::U32(mnb::callback_one(m))
            });
        }
    });
}

#[test]
fn r3_menu_font_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0404);
        for case in 0..16u32 {
            t.reset();
            let r = rng.next();
            t.stub(mnc::MES_FONT_8ED190, false, 11, 0, r);
            t.stub(mnc::FONT_8F1880, false, 8, 0, r ^ 1);
            t.stub(mnc::UNLOAD_TEXTURE, false, 1, 0, r ^ 2);
            t.stub(mnc::GLYPH_A, false, 2, 0, r ^ 3);
            t.stub(mnc::GLYPH_B, false, 2, 0, r ^ 4);
            t.stub(mnc::GLYPH_C, false, 2, 0, r ^ 5);
            let w6: [u32; 6] = std::array::from_fn(|_| rng.word());
            t.check("mes font 8ed120", 0x008e_d120, Conv::Cdecl, &w6, |m| {
                Out::U32(mnb::mes_font_8ed120(m, w6))
            });
            let cell = t.rebase(mnc::PKT_CURSOR);
            t.w32(cell, rng.next());
            t.check("font 8f8870", 0x008f_8870, Conv::Cdecl, &w6[..3], |m| {
                Out::U32(mnb::font_8f8870(m, w6[0], w6[1], w6[2]))
            });
            let count = t.rebase(mnc::WAKKA_COUNT);
            t.w32(count, [0, 1, 3, 0xffff_ffff][(case % 4) as usize]);
            t.check("wakka unloaded", 0x008e_89d0, Conv::Cdecl, &[], |m| {
                mnb::wakka_textures_unloaded(m).map_or(Out::None, Out::U32)
            });
            let glyph = t.zeros(0x28);
            // 2^60 and -2^60: the sum's order shows (the pad is lost in 2^60)
            let adv = [0.0f32, 1.5, -3.25, 1_152_921_504_606_846_976.0][(case % 4) as usize];
            t.w32(glyph + 0x1c, adv.to_bits());
            let flags = [0u16, 1, 2, 4, 0xfffc][(case % 5) as usize];
            t.write(glyph + 0x26, &flags.to_le_bytes());
            let pos = t.zeros(8);
            let x = [0.0f32, 100.25, -7.0, -1_152_921_504_606_846_976.0][(case / 4 % 4) as usize];
            t.w32(pos + 4, x.to_bits());
            t.check(
                "glyph advance",
                0x008f_c770,
                Conv::Cdecl,
                &[pos, glyph],
                |m| Out::U32(mnb::glyph_advance(m, pos, glyph)),
            );
        }
    });
}

#[test]
fn r3_menu_shape_pairs_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0405);
        for _ in 0..8u32 {
            t.reset();
            let r = rng.next();
            t.stub_seq(mnc::EASY_SHAPE, 8, 0, &[r, r ^ 1]);
            let info = t.zeros(0x10);
            let ww = (rng.below(0x80) as u16).to_le_bytes();
            t.write(info + 8, &ww);
            t.stub(mnc::SHAPE_INFO, false, 1, 0, info);
            t.stub(mnc::SHAPE_INFO_ID, false, 1, 0, info);
            let w8: [u32; 8] = std::array::from_fn(|_| rng.word());
            let w6 = [w8[0], w8[1], w8[2], w8[3], w8[4], w8[5]];
            for sym in [0x008f_1a90u32, 0x008f_1af0] {
                t.stub_seq(mnc::EASY_SHAPE, 8, 0, &[r, r ^ 1]);
                t.check("shape da db", sym, Conv::Cdecl, &w6, |m| {
                    Out::U32(mnb::shape_da_db(m, w6))
                });
            }
            t.stub_seq(mnc::EASY_SHAPE, 8, 0, &[r, r ^ 1]);
            t.check("shape ids", 0x008f_0990, Conv::Cdecl, &w8, |m| {
                Out::U32(mnb::shape_ids(m, w8))
            });
        }
    });
}

// ---- round 3: magic-file preloads, VFX vector cells (`ffx_system::m5b_pc`) ----

#[test]
fn r3_magic_preloads_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0501);
        for case in 0..24u32 {
            t.reset();
            imp(t, pcs::IOB_FUNC_IAT, 0, false, 0x0e10_0000);
            imp(t, pcs::FPRINTF_IAT, 6, false, 0x11);
            t.stub(pcs::UNLOAD_PRX, false, 2, 0, 0x12);
            let on = t.rebase(pcs::PRELOAD_ON);
            t.w32(on, u32::from(case % 6 != 0));
            let table = t.rebase(pcs::PRELOAD_TABLE);
            let counts = t.rebase(pcs::PRELOAD_COUNTS);
            let handles = t.rebase(pcs::PRELOAD_HANDLES);
            for i in 0..12 {
                t.put(table + 12 * i, &[rng.below(4), rng.next(), rng.below(3)]);
                let c = [0, 1, 3, 0xffff_ffff][rng.below(4) as usize];
                t.w32(counts + 4 * i, c);
                t.w32(handles + 4 * i, rng.next());
            }
            let magic = [0, 1, 3, 0xffff_ffff][(case % 4) as usize];
            let file = [0, 2, 0x8000_0000][(case / 4 % 3) as usize];
            if case & 1 == 0 {
                t.check(
                    &format!("clear #{case}"),
                    0x009d_acc0,
                    Conv::Cdecl,
                    &[magic, file],
                    |m| pcb::clear_preloaded(m, magic, file).map_or(Out::None, Out::U32),
                );
            } else {
                t.check(
                    &format!("clear all #{case}"),
                    0x009d_aca0,
                    Conv::Cdecl,
                    &[magic],
                    |m| pcb::clear_all_preloaded(m, magic).map_or(Out::None, Out::U32),
                );
            }
        }
    });
}

#[test]
fn r3_magic_dll_loaded_matches_the_original() {
    Twins::each(|t| {
        for module in [0u32, 0x1000_0000] {
            t.reset();
            imp(t, pcs::GET_LAST_ERROR_IAT, 0, true, 0x7e);
            imp(t, pcs::PRINTF_IAT, 3, false, 0x21);
            imp(t, pcs::GET_PROC_ADDRESS_IAT, 2, true, 0x1000_1234);
            t.check(
                &format!("dll loaded {module:#x}"),
                0x009d_b110,
                Conv::Cdecl,
                &[module],
                |m| Out::U32(pcb::on_magic_dll_loaded(m, module)),
            );
        }
    });
}

#[test]
fn r3_vfx_vector_cells_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0502);
        for _ in 0..4 {
            t.reset();
            let src = t.words(&[rng.next(), rng.next(), rng.next(), rng.next()]);
            let dst = t.zeros(16);
            t.check("copy c0a004", 0x0092_cc20, Conv::Cdecl, &[dst], |m| {
                Out::U32(pcb::copy_c0a004(m, dst))
            });
            t.check(
                "copy via c8f980",
                0x0092_cbf0,
                Conv::Cdecl,
                &[dst, src],
                |m| Out::U32(pcb::copy_via_c8f980(m, dst, src)),
            );
        }
    });
}

// ---- round 3, batch 2: packet builders and menu draws (`ffx_system::m5b_menu`) ----
//
// None of these ran in a coverage run but the battle plate and the
// windows; every branch runs on synthetic globals and recording stubs
// (the face slot past 7, a window under 16 wide, a save slot not in use,
// each `TOCtrlPkt` state, the languages), and the snapshots repeat them
// over their memory.

impl Twins {
    /// A recording stub (cdecl) that writes a string into the buffer its
    /// word `buf` points to: `count` bytes of `src` (its word `count_word`
    /// masked to 0..15 when given, else `fixed`), then a NUL. Logs `n`
    /// words, returns `ret`; returns its address.
    #[allow(clippy::too_many_arguments)]
    fn fill_code(
        &mut self,
        tag: u32,
        n: u32,
        buf: u8,
        count_word: Option<u8>,
        fixed: u32,
        src: u32,
        ret: u32,
    ) -> u32 {
        let log_stub = self.stub_code(tag, false, n, 0, ret);
        let mut c = vec![0x56, 0x57]; // push esi; push edi
        c.extend([0x8b, 0x7c, 0x24, 0xc + 4 * buf]); // mov edi, [esp+0xc+4buf]
        if let Some(w) = count_word {
            c.extend([0x8b, 0x4c, 0x24, 0xc + 4 * w]); // mov ecx, [esp+0xc+4w]
            c.extend([0x83, 0xe1, 0x0f]); // and ecx, 0xf
        } else {
            c.push(0xb9);
            c.extend(fixed.to_le_bytes()); // mov ecx, fixed
        }
        c.push(0xbe);
        c.extend(src.to_le_bytes()); // mov esi, src
        c.extend([0xf3, 0xa4]); // rep movsb
        c.extend([0xc6, 0x07, 0x00]); // mov byte [edi], 0
        c.extend([0x5f, 0x5e]); // pop edi; pop esi
        c.push(0xe9);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(log_stub.wrapping_sub(here).to_le_bytes()); // jmp log_stub
        self.cave_code(&c)
    }

    /// A recording stub over `sym` (no words) that first sets the word at
    /// the `ffx.sym` address `global` to `value`; returns `ret`.
    fn stub_store_global(&mut self, sym: u32, global: u32, value: u32, ret: u32) {
        let log_stub = self.stub_code(sym, false, 0, 0, ret);
        let mut c = vec![0xc7, 0x05];
        c.extend(self.rebase(global).to_le_bytes());
        c.extend(value.to_le_bytes()); // mov dword [global], value
        c.push(0xe9);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(log_stub.wrapping_sub(here).to_le_bytes()); // jmp log_stub
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }

    /// [`Twins::fill_code`] bound to the import slot `slot` (tag `slot`).
    #[allow(clippy::too_many_arguments)]
    fn fill_import(
        &mut self,
        slot: u32,
        n: u32,
        buf: u8,
        count_word: Option<u8>,
        fixed: u32,
        src: u32,
        ret: u32,
    ) {
        let addr = self.fill_code(slot, n, buf, count_word, fixed, src, ret);
        let at = self.rebase(slot);
        self.patch(at, &addr.to_le_bytes());
    }

    /// [`Twins::fill_code`] over the original at `sym`.
    #[allow(clippy::too_many_arguments)]
    fn fill_stub(
        &mut self,
        sym: u32,
        n: u32,
        buf: u8,
        count_word: Option<u8>,
        fixed: u32,
        src: u32,
        ret: u32,
    ) {
        let addr = self.fill_code(sym, n, buf, count_word, fixed, src, ret);
        self.jump(sym, addr);
    }

    /// [`Twins::check`] of a cdecl original with a frame of `low` bytes
    /// below its `ebp` (zeroed in both; the top `skip` bytes, the
    /// security cookie, left out of the comparison) and its argument
    /// slots: the bridge gets `ebp` and the slots' address (B holds the
    /// same argument words there, its own calls go below the frame), and
    /// both the frame and the slots must agree afterwards.
    fn check_fs(
        &mut self,
        what: &str,
        sym: u32,
        args: &[u32],
        low: u32,
        skip: u32,
        bridge: impl FnOnce(&mut G, u32, u32) -> Out,
    ) {
        let ebp = self.frame_ebp(args.len());
        let slots = ebp + 8;
        let lo = ebp - low;
        self.write(lo, &vec![0; low as usize]);
        let words: Vec<u8> = args.iter().flat_map(|v| v.to_le_bytes()).collect();
        self.b.write(slots, &words).unwrap();
        self.b.set_call_top(Some(lo - 0x100));
        self.check(what, sym, Conv::Cdecl, args, |m| bridge(m, ebp, slots));
        self.b.set_call_top(None);
        for (from, n) in [(lo, low - skip), (slots, 4 * args.len() as u32)] {
            let mut x = vec![0; n as usize];
            let mut y = vec![0; n as usize];
            Mem::read_bytes(&self.a, from, &mut x);
            Mem::read_bytes(&self.b, from, &mut y);
            if let Some(i) = x.iter().zip(&y).position(|(u, v)| u != v) {
                panic!(
                    "{what}: stack byte {:#x}: {} != {}",
                    from + i as u32,
                    x[i],
                    y[i]
                );
            }
        }
    }
}

#[test]
fn r3b2_packet_words_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0601);
        for case in 0..8u32 {
            t.reset();
            let r = rng.next();
            let buf = t.zeros(0x200);
            t.g32(mnc::PKT_CURSOR, buf);
            t.stub(mnc::DRAW_KICK, false, 0, 0, r ^ 0x99);
            // TOGsTEXFLUSH moves the cursor: a stub that sets it
            let moved = buf + 0x40;
            t.stub_store_global(mnc::GS_TEXFLUSH, mnc::PKT_CURSOR, moved, r ^ 1);
            t.check(
                &format!("tex flush #{case}"),
                0x008e_b4e0,
                Conv::Cdecl,
                &[buf],
                |m| Out::U32(mnb::tex_flush_alpha(m, buf)),
            );
            t.g32(mnc::PKT_CURSOR, buf);
            ctrl_state(t, case, buf + 0x60);
            t.check(
                &format!("plain regs #{case}"),
                0x0090_46e0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mnb::plain_draw_regs(m)),
            );
            t.g32(mnc::PKT_CURSOR, buf);
            for (sym, n) in [(mnc::GS_ALPHA, 5), (mnc::GS_WORDS4, 4)] {
                t.stub(sym, false, n, 0, r ^ sym);
            }
            t.stub_seq(mnc::GS_XYZ2_OFFSET, 2, 0, &[r ^ 2, r ^ 3, r ^ 4, r ^ 5]);
            ctrl_state(t, case, r ^ 5);
            let w4: [u32; 4] = std::array::from_fn(|_| rng.word());
            t.check(
                &format!("blue window #{case}"),
                0x008f_de50,
                Conv::Cdecl,
                &w4,
                |m| Out::U32(mnb::sample_blue_window(m, w4)),
            );
        }
    });
}

#[test]
fn r3b2_face_sprite_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0602);
        for slot in [0u32, 1, 2, 3, 4, 5, 6, 7, 8, 0xffff_ffff, 0x8000_0003] {
            t.reset();
            let r = rng.next();
            t.stub(mnc::ADP_TEST, false, 9, 0, r);
            let p = t.zeros(0x80);
            t.stub(mnc::ADP_TEX0, false, 13, 0, p + 0x20);
            t.stub(mnc::ADP_WORDS4, false, 5, 0, r ^ 1);
            t.stub_seq(mnc::ADP_XY2, 3, 0, &[r ^ 2, r ^ 3]);
            t.stub_seq(mnc::ADP_XYZ2_OFFSET, 3, 0, &[r ^ 4, r ^ 5]);
            let mut args = [p, slot, 0, 0, 0, 0, 0, 0];
            for a in &mut args[2..] {
                *a = rng.word();
            }
            t.check_fs(
                &format!("face sprite {slot:#x}"),
                0x008f_1880,
                &args,
                0,
                0,
                |m, _, s| Out::U32(mnb::face_sprite(m, args, s)),
            );
        }
    });
}

#[test]
fn r3b2_menu_shapes_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0603);
        for case in 0..12u32 {
            t.reset();
            let r = rng.next();
            let info = t.zeros(0x10);
            let ww = [0u16, 4, 0x20, 0xffff][(case % 4) as usize];
            let hh = [0u16, 6, 0x30, 0x8000][(case / 4 % 3) as usize];
            t.write(info + 8, &ww.to_le_bytes());
            t.write(info + 0xa, &hh.to_le_bytes());
            t.stub(mnc::SHAPE_INFO, false, 1, 0, info);
            t.stub(mnc::WINDOW_LAYOUT, false, 1, 0, info);
            t.stub_seq(mnc::EASY_SHAPE, 8, 0, &[r, r ^ 1, r ^ 2]);
            let w: [u32; 7] = std::array::from_fn(|_| rng.word());
            let w6 = [w[0], w[1], w[2], w[3], w[4], w[5]];
            for (sym, id) in [(0x008f_1990u32, 0x10b), (0x008f_1a10, 0x10a)] {
                t.stub_seq(mnc::EASY_SHAPE, 8, 0, &[r, r ^ 1, r ^ 2]);
                t.check(
                    &format!("da db icon {sym:#x} #{case}"),
                    sym,
                    Conv::Cdecl,
                    &w6,
                    |m| Out::U32(mnb::shape_da_db_icon(m, w6, id)),
                );
            }
            t.stub_seq(mnc::EASY_SHAPE, 8, 0, &[r, r ^ 1, r ^ 2]);
            let cell = t.rebase(mnc::PKT_CURSOR);
            t.w32(cell, rng.next());
            t.check(
                &format!("left right #{case}"),
                0x008f_5e90,
                Conv::Cdecl,
                &w,
                |m| Out::U32(mnb::shape_left_right(m, w)),
            );
            // the windows: widths below, at and past 16
            t.stub_seq(mnc::MKP_SHAPE_XYWHC2, 7, 0, &[r ^ 3, r ^ 4, r ^ 5, r ^ 6]);
            t.stub_seq(mnc::SHAPE_901C10, 7, 0, &[r ^ 7, r ^ 8]);
            t.g32(mnc::WINDOW_COLOUR, rng.next());
            t.g32(mnc::WINDOW_COLOUR2, rng.next());
            ctrl_state(t, case, r ^ 6);
            let width = [0u32, 0xf, 0x10, 0x40, 0xffff_fff0, 0x7fff_ffff][(case % 6) as usize];
            let fr = [w[0], w[1], width, w[3], w[4], w[5]];
            t.check_fs(
                &format!("window frame #{case}"),
                0x008f_9b60,
                &fr,
                0,
                0,
                |m, _, s| Out::U32(mnb::window_frame(m, fr, s)),
            );
            t.stub_seq(mnc::MKP_SHAPE_XYWHC2, 7, 0, &[r ^ 3, r ^ 4, r ^ 5, r ^ 6]);
            let wp = [w[0], w[1], width, w[3]];
            let last = if width.cast_signed() < 0x10 {
                r ^ 8
            } else {
                r ^ 6
            };
            ctrl_state(t, case, last);
            t.check_fs(
                &format!("window plain #{case}"),
                0x008f_9a40,
                &wp,
                0,
                0,
                |m, _, s| Out::U32(mnb::window_plain(m, wp, s)),
            );
        }
    });
}

#[test]
fn r3b2_battle_plate_and_save_faces_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0604);
        for case in 0..8u32 {
            t.reset();
            let r = rng.next();
            for (k, (sym, n)) in [
                (mnc::BLUE_WINDOW, 4),
                (mnc::MKP_SHAPE_XY, 3),
                (mnc::SAVE_CHR_NAME, 1),
                (mnc::MES_FONT_L_CENTER, 3),
                (mnc::PARAM_PLATE, 3),
                (mnc::SMALL_FACE, 0),
                (mnc::PKT_SCISSOR, 4),
            ]
            .into_iter()
            .enumerate()
            {
                t.stub(sym, false, n, 0, r ^ (k as u32 * 0x101));
            }
            let w3: [u32; 3] = std::array::from_fn(|_| rng.word());
            t.check(
                &format!("change seq anm #{case}"),
                0x0089_28b0,
                Conv::Cdecl,
                &w3,
                |m| Out::U32(mnb::change_seq_anm(m, w3[0], w3[1], w3[2])),
            );
            // the save file: its index, info (faces from +5, the flag +0x1e)
            let idx = [0u16, 3, 0x62, 0xffff][(case % 4) as usize];
            let save = t.zeros(0x50);
            t.write(save + 0x48, &idx.to_le_bytes());
            let mut faces = [0u8; 0x20];
            for (i, f) in faces[5..12].iter_mut().enumerate() {
                *f = [6, 0xff, 1, 6, 0xff, 7, 0xff][(i + case as usize) % 7];
            }
            faces[0x1e] = u8::from(case & 2 != 0);
            if case == 7 {
                faces[11] = 2; // the last face drawn
            }
            let info = t.alloc(&faces);
            t.stub(mnc::SAVE_FILE_INFO, false, 1, 0, info);
            let slots_at = t.rebase(mnc::SAVE_SLOTS);
            let state = i32::from(idx.cast_signed()).cast_unsigned();
            t.write(
                slots_at.wrapping_add(state),
                &[if case & 4 == 0 { 1 } else { 3 }],
            );
            t.stub_seq(
                mnc::FONT_8F8870,
                3,
                0,
                &[
                    r ^ 0x10,
                    r ^ 0x11,
                    r ^ 0x12,
                    r ^ 0x13,
                    r ^ 0x14,
                    r ^ 0x15,
                    r ^ 0x16,
                ],
            );
            let args = [save, rng.word(), rng.word()];
            t.check_fs(
                &format!("save faces #{case}"),
                0x008b_37d0,
                &args,
                0,
                0,
                |m, _, s| Out::U32(mnb::save_party_faces(m, args[0], args[1], args[2], s)),
            );
        }
    });
}

#[test]
fn r3b2_number_printers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0605);
        // digits with a letter and a sign among them
        let src = t.alloc(b"19a4:-/7890123456789");
        for case in 0..16u32 {
            t.reset();
            let r = rng.next();
            t.fill_import(mnc::SPRINTF_IAT, 3, 0, Some(2), 0, src, r ^ 0x55);
            t.stub(mnc::STR_FONT_M, false, 9, 0, r);
            t.stub(mnc::FONT3D80_STRING, false, 9, 0, r ^ 1);
            t.stub(mnc::MKP_SHAPE_XYC2, false, 5, 0, r ^ 2);
            t.stub(mnc::DRAW_KICK, false, 0, 0, r ^ 3);
            let value = [0u32, 1, 3, 8, 9, 11, 15, 0x1234_5670][(case % 8) as usize];
            let w5 = [rng.next(), value, rng.word(), rng.word(), rng.word()];
            t.check_fs(
                &format!("value font m #{case}"),
                0x008f_3280,
                &w5,
                0x24,
                4,
                |m, ebp, _| Out::U32(mnb::value_font_m(m, ebp, w5)),
            );
            t.check_fs(
                &format!("value font 3d80 #{case}"),
                0x008f_3390,
                &w5,
                0x24,
                4,
                |m, ebp, _| Out::U32(mnb::value_font_3d80(m, ebp, w5)),
            );
            ctrl_state(t, case, 0);
            t.g32(mnc::PKT_CURSOR, r ^ 2);
            // at most 7 characters: an eighth and its NUL would reach the
            // security cookie at ebp - 4 (the original's 8-character path
            // needs a 9-byte "%02d", which breaks the cookie in the game too)
            let w4 = [case % 8, rng.word(), rng.word(), rng.word()];
            t.check_fs(
                &format!("lulu counter #{case}"),
                0x0090_1280,
                &w4,
                0xc,
                4,
                |m, ebp, _| Out::U32(mnb::lulu_counter(m, ebp, w4)),
            );
            // TOValueToMesFontL writes the digits; the language and the
            // English config choose the width
            t.fill_stub(mnc::VALUE_TO_MES_FONT_L, 3, 1, Some(0), 0, src, r ^ 4);
            t.stub(
                mnc::GET_FFX_LANG_JMP,
                false,
                0,
                0,
                [0, 0, 9, 1][(case % 4) as usize],
            );
            // MsGetSaveConfigEnglish is `xor eax, eax; ret`: the stub's jump
            // covers two bytes past it, put back by the reset
            t.stub(
                mnc::SAVE_CONFIG_ENGLISH,
                false,
                0,
                0,
                u32::from(case & 4 != 0),
            );
            t.stub(mnc::BASE_MES_FONT_RGBA, false, 11, 0, r ^ 5);
            t.check_fs(
                &format!("value font l #{case}"),
                0x008f_30b0,
                &w5,
                0x24,
                4,
                |m, ebp, _| Out::U32(mnb::value_font_l_clut_right(m, ebp, w5)),
            );
        }
    });
}

#[test]
fn r3b2_string_wrappers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0606);
        let path = t.alloc(b"ffx/font/abc/name.dat\0");
        let replaced = t.alloc(b"/FFX_Data/new.dat\0");
        for case in 0..12u32 {
            t.reset();
            let r = rng.next();
            t.stub(mnc::CROSS_STR_FONT_S, false, 10, 0, r);
            let xy = [
                (0u32, 0u32),
                (5, 0xffff_fffb),
                (0x7fff_ffff, 0x8000_0000),
                (0x0123_4567, 3),
            ][(case % 4) as usize];
            let args = [rng.next(), xy.0, xy.1];
            t.check_fs(
                &format!("str font s #{case}"),
                0x0090_2040,
                &args,
                0,
                0,
                |m, _, s| Out::U32(mnb::easy_str_font_s(m, args[0], args[1], args[2], s)),
            );
            // the width TOGetMessageWH writes, both x87 and SSE truncation
            let width = [0.0f32, 13.0, 7.5, -3.25, 1e10, 3.0][(case % 6) as usize];
            t.stub_store(
                mnc::MESSAGE_WH,
                None,
                2,
                width.to_bits(),
                false,
                6,
                0,
                r ^ 1,
            );
            t.stub(mnc::BASE_MES_FONT_PLAIN, false, 4, 0, r ^ 2);
            t.stub(mnc::DRAW_KICK, false, 0, 0, r ^ 3);
            ctrl_state(t, case, r ^ 2);
            let sse = t.rebase(0x0193_ed20); // __sse2_available
            // 1e10 (case 4 and 10) under both truncations
            t.w32(sse, (case >> 1) & 1);
            t.check_fs(
                &format!("mes font l center #{case}"),
                0x0090_1c50,
                &args,
                8,
                0,
                |m, ebp, s| Out::U32(mnb::easy_mes_font_l_center(m, ebp, args, s)),
            );
            // the font paths: languages 0, 9, 10 and another
            t.stub(
                mnc::GET_FFX_LANG_JMP,
                false,
                0,
                0,
                [0, 9, 10, 1][(case % 4) as usize],
            );
            t.stub(mnc::STR_REPLACE, false, 3, 0, replaced);
            t.fill_stub(mnc::UNIFY_FILENAME, 3, 1, None, 22, path, r ^ 4);
            let ebp = t.frame_ebp(1);
            // strrchr: the last '/' of the unified path
            imp(t, mnc::STRRCHR_IAT, 2, false, ebp - 0x204 + 12);
            t.stub(mnc::USER_FREE, false, 1, 0, r ^ 5);
            let (sym, table) = if case & 1 == 0 {
                (0x008a_d780u32, &mnc::BATTLE_FONTS)
            } else {
                (0x008a_d840, &mnc::EVENT_FONTS)
            };
            let dest = t.rebase(table.dest);
            t.write(dest, &[0xa5; 0x20]);
            t.check_fs(
                &format!("font path #{case}"),
                sym,
                &[path],
                0x204,
                4,
                |m, ebp, _| Out::U32(mnb::init_font_path(m, ebp, path, table)),
            );
        }
    });
}

// ---- round 3, batch 3: glyph packets, UI quads (`ffx_system::m5b_menu`) ----

/// A glyph position record (7 words: colour bytes, x, y, ..., the
/// nibbles at `+0x1b`) and a glyph (UV and size floats, `i16 +2`, `+4`,
/// the flags at `+0x26`) from `rng`, with some awkward floats.
fn glyph_pair(t: &mut Twins, rng: &mut Rng, case: u32) -> (u32, u32) {
    let fl = [0.0f32, 1.5, -2.75, 300.25, 1e10, -0.0, 7.0, 0.49];
    let mut pos = [0u32; 7];
    pos[0] = rng.next();
    pos[1] = fl[(case % 8) as usize].to_bits();
    pos[2] = fl[((case + 3) % 8) as usize].to_bits();
    pos[6] = rng.next();
    let pos = t.words(&pos);
    let mut g = [0u32; 10];
    g[0] = rng.next();
    g[1] = [0x8000_7fff, 0x0001_fffe, 0, rng.next()][(case % 4) as usize];
    for k in 2..9 {
        g[k] = fl[((case as usize) + k) % 8].to_bits();
    }
    g[9] = u32::from([0u16, 1, 2, 0xffff][(case / 2 % 4) as usize]) << 16;
    let glyph = t.words(&g);
    (pos, glyph)
}

#[test]
fn r3b3_glyph_packets_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0701);
        for case in 0..16u32 {
            t.reset();
            let r = rng.next();
            let buf = t.zeros(0x80);
            t.g32(mnc::PKT_CURSOR, buf);
            t.g32(0x0193_ed20, case & 1); // __sse2_available
            t.stub(mnc::GS_WORDS4, false, 4, 0, r);
            t.stub(mnc::GS_XY2, false, 2, 0, r ^ 1);
            t.stub_seq(mnc::GS_XYZ2_OFFSET, 2, 0, &[r ^ 2, r ^ 3]);
            t.stub(mnc::DRAW_KICK, false, 0, 0, r ^ 4);
            ctrl_state(t, case, buf + 0x50);
            let (pos, glyph) = glyph_pair(t, &mut rng, case);
            t.check(
                &format!("glyph edge #{case}"),
                0x0090_5ef0,
                Conv::Cdecl,
                &[pos, glyph],
                |m| Out::U32(mnb::glyph_edge(m, pos, glyph)),
            );
            t.g32(mnc::PKT_CURSOR, buf);
            t.stub_seq(mnc::GS_XYZ2_OFFSET, 2, 0, &[r ^ 2, r ^ 3]);
            t.check(
                &format!("glyph fill #{case}"),
                0x0090_60f0,
                Conv::Cdecl,
                &[pos, glyph],
                |m| Out::U32(mnb::glyph_fill(m, pos, glyph)),
            );
            // the two passes as stubs: the copy they get is on the frame
            t.stub(mnc::GLYPH_A, false, 2, 0, r ^ 5);
            t.stub(mnc::GLYPH_B, false, 2, 0, r ^ 6);
            t.check_fs(
                &format!("glyph draw #{case}"),
                0x0090_6050,
                &[pos, glyph],
                0x1c,
                0,
                |m, ebp, _| Out::U32(mnb::glyph_draw(m, ebp, pos, glyph)),
            );
            for sym in [
                mnc::ITALIC_EDGE,
                mnc::ITALIC_FONT,
                mnc::MOVIE_EDGE,
                mnc::MOVIE_FONT,
            ] {
                t.stub(sym, false, 2, 0, r ^ sym);
            }
            let kind = [0u8, 1, 3, 2, 0xff, 3, 4, 3][(case % 8) as usize];
            t.write(pos + 0x1a, &[kind]);
            t.check_fs(
                &format!("movie font #{case}"),
                0x008f_d960,
                &[pos, glyph],
                0x1c,
                0,
                |m, ebp, _| Out::U32(mnb::movie_font_1612(m, ebp, pos, glyph)),
            );
        }
    });
}

#[test]
fn r3b3_ui_quads_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0702);
        for case in 0..12u32 {
            t.reset();
            let r = rng.next();
            t.g32(0x0193_ed20, case & 1); // __sse2_available
            t.stub(mnc::DRAW_UI_ELEMENT, false, 5, 0, r);
            t.stub(
                mnc::BEGIN_SCISSOR,
                false,
                4,
                0,
                [0, 1, 0xffff_ffff][(case % 3) as usize],
            );
            let w6: [u32; 6] = std::array::from_fn(|_| rng.word());
            t.check_fs(
                &format!("ui box #{case}"),
                0x008f_d390,
                &w6,
                0xa0,
                4,
                |m, ebp, _| Out::U32(mnb::ui_box(m, ebp, w6)),
            );
            let w8: [u32; 8] = std::array::from_fn(|_| rng.word());
            t.check_fs(
                &format!("ui rect #{case}"),
                0x008f_b090,
                &w8,
                0x9c,
                4,
                |m, ebp, _| Out::U32(mnb::ui_rect(m, ebp, w8)),
            );
            // the picture: its size as TOGetImageWH writes it (0 and
            // negatives for the x87 division's NaN and sign rules)
            let wh =
                [(512.0f32, 416.0f32), (0.0, 2.0), (-3.5, 0.0), (1e-30, 7.0)][(case % 4) as usize];
            t.stub(mnc::SHAP_TEXTURE_NAME, false, 1, 0, r ^ 1);
            image_wh_stub(t, wh.0.to_bits(), wh.1.to_bits(), r ^ 2);
            let w5 = [rng.next(), rng.word(), rng.word(), rng.next(), rng.next()];
            t.check_fs(
                &format!("ui picture #{case}"),
                0x008f_2bf0,
                &w5,
                0xa8,
                4,
                |m, ebp, _| Out::U32(mnb::ui_picture(m, ebp, w5)),
            );
        }
    });
}

/// A `TOGetImageWH(id, &w, &h)` stub: writes `w` and `h`, logs the three
/// words, returns `ret`.
fn image_wh_stub(t: &mut Twins, w: u32, h: u32, ret: u32) {
    let log_stub = t.stub_code(mnc::IMAGE_WH, false, 3, 0, ret);
    let mut c = vec![0x8b, 0x44, 0x24, 0x08]; // mov eax, [esp+8]
    c.extend([0xc7, 0x00]);
    c.extend(w.to_le_bytes()); // mov dword [eax], w
    c.extend([0x8b, 0x44, 0x24, 0x0c]); // mov eax, [esp+0xc]
    c.extend([0xc7, 0x00]);
    c.extend(h.to_le_bytes()); // mov dword [eax], h
    c.push(0xe9);
    let here = t.cave_next + c.len() as u32 + 4;
    c.extend(log_stub.wrapping_sub(here).to_le_bytes()); // jmp log_stub
    let addr = t.cave_code(&c);
    t.jump(mnc::IMAGE_WH, addr);
}

#[test]
fn r3b3_scroll_bar_and_vector_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0703);
        for case in 0..16u32 {
            t.reset();
            let r = rng.next();
            t.g32(0x0193_ed20, (case >> 1) & 1); // __sse2_available
            t.stub_seq(mnc::UI_RECT, 8, 0, &[r, r ^ 1]);
            t.stub(mnc::DRAW_KICK, false, 0, 0, r ^ 2);
            ctrl_state(t, case, 0);
            // first, count, total: empty, partial, to the end, past it,
            // count >= total, a zero total under a negative count
            let (first, count, total) = [
                (0u32, 3u32, 10u32),
                (2, 3, 10),
                (7, 3, 10),
                (9, 5, 10),
                (0, 10, 10),
                (0, 0xffff_fffe, 0),
                (1, 5, 6),
                (0x7fff_ffff, 1, 0x7fff_fff0),
            ][(case % 8) as usize];
            let mut args = [
                rng.word(),
                rng.word(),
                rng.word(),
                rng.word(),
                first,
                count,
                total,
            ];
            if total == 6 {
                // y 0, h 1: the thumb's end sums to 0.9999999999999999,
                // so only the full-length rule gives 1
                args[1] = 0;
                args[3] = 1;
            }
            t.check_fs(
                &format!("scroll bar #{case}"),
                0x0090_2700,
                &args,
                0,
                0,
                |m, _, s| Out::U32(mnb::scroll_bar(m, args, s)),
            );
            // the by-value vector
            t.stub(mnc::COPY16_FROM_SYM, false, 2, 0, r ^ 3);
            t.stub_copy16(mnc::COPY16_TO_B, r ^ 4);
            t.stub(mnc::VU0_MUL_X, false, 6, 0, r ^ 5);
            t.stub(mnc::COPY16_TO_SYM, false, 2, 0, r ^ 6);
            let args = [
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
            ];
            t.check_fs(
                &format!("vec mul x #{case}"),
                0x0092_cc40,
                &args,
                0,
                0,
                |m, _, s| Out::U32(mnb::vec_mul_x(m, args[0], args[1], s)),
            );
        }
    });
}

impl Twins {
    /// A `(src, dst)` 16-byte copy stub over `sym` that logs both words;
    /// returns `ret`.
    fn stub_copy16(&mut self, sym: u32, ret: u32) {
        let log_stub = self.stub_code(sym, false, 2, 0, ret);
        let mut c = vec![0x56, 0x57]; // push esi; push edi
        c.extend([0x8b, 0x74, 0x24, 0x0c]); // mov esi, [esp+0xc] (src)
        c.extend([0x8b, 0x7c, 0x24, 0x10]); // mov edi, [esp+0x10] (dst)
        c.extend([0xb9, 4, 0, 0, 0]); // mov ecx, 4
        c.extend([0xf3, 0xa5]); // rep movsd
        c.extend([0x5f, 0x5e]); // pop edi; pop esi
        c.push(0xe9);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(log_stub.wrapping_sub(here).to_le_bytes()); // jmp log_stub
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }
}

// ---- round 3, batch 3: the HDD reader's requests (`ffx_system::m5b_hdd`) ----

const BG_STATE: u32 = 0x0112_8aad;
const TASK_WAIT: u32 = 0x0077_a4e0;

impl Twins {
    /// A recording stub over `sym` (`n` words) that first clears the byte
    /// at the `ffx.sym` address `at`; returns `ret`.
    fn stub_clear_byte(&mut self, sym: u32, at: u32, n: u32, ret: u32) {
        let log_stub = self.stub_code(sym, false, n, 0, ret);
        let mut c = vec![0xc6, 0x05];
        c.extend(self.rebase(at).to_le_bytes());
        c.push(0); // mov byte [at], 0
        c.push(0xe9);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(log_stub.wrapping_sub(here).to_le_bytes()); // jmp log_stub
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }

    /// A recording stub over `sym` (`n` words) that first decrements the
    /// byte at the `ffx.sym` address `at`; returns `ret`.
    fn stub_dec_byte(&mut self, sym: u32, at: u32, n: u32, ret: u32) {
        let log_stub = self.stub_code(sym, false, n, 0, ret);
        let mut c = vec![0xfe, 0x0d];
        c.extend(self.rebase(at).to_le_bytes()); // dec byte [at]
        c.push(0xe9);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(log_stub.wrapping_sub(here).to_le_bytes()); // jmp log_stub
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }
}

#[test]
fn r3b3_hdd_requests_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0801);
        let name = t.alloc(b"/hdd0/ffx/data0\0");
        for case in 0..8u32 {
            t.reset();
            let r = rng.next();
            t.stub_two(hdc::DBG_PRINTF, 2, 5, r);
            t.stub(hdc::HDD_NAME, false, 1, 0, name);
            // the waits clear the state; the signal does in some cases
            // each wait takes the state one step down: 3 after the signal
            // needs three
            t.stub_dec_byte(TASK_WAIT, BG_STATE, 1, r ^ 1);
            if case & 2 == 0 {
                t.stub(hdc::SIGNAL_BG_THREAD, false, 0, 0, r ^ 2);
            } else {
                t.stub_clear_byte(hdc::SIGNAL_BG_THREAD, BG_STATE, 0, r ^ 2);
            }
            let head = case % 3;
            let heads = t.rebase(hdc::HEADS);
            t.put(heads + 12 * head, &[0x100, u32::from(case & 4 == 0), 7]);
            t.write(t.rebase(BG_STATE), &[[0, 2][(case & 1) as usize]]);
            // the path's old bytes: the copy's NUL must land
            t.write(t.rebase(0x0112_a7b0), &[0xa5; 0x20]);
            let args = [0x100 + rng.below(0x40), rng.next(), rng.next(), head];
            t.check(
                &format!("read th #{case}"),
                0x0077_8610,
                Conv::Cdecl,
                &args,
                |m| Out::U32(hdb::read_th(m, args[0], args[1], args[2], args[3])),
            );
            t.write(t.rebase(BG_STATE), &[[0, 2][(case & 1) as usize]]);
            t.write(t.rebase(0x0112_a7b0), &[0xa5; 0x20]);
            let args = [name, rng.next(), rng.next(), rng.next()];
            t.check(
                &format!("read ex th #{case}"),
                0x0077_8550,
                Conv::Cdecl,
                &args,
                |m| Out::U32(hdb::read_ex_th(m, args[0], args[1], args[2], args[3])),
            );
        }
    });
}

#[test]
fn r3b3_hdd_read_main_and_space_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0802);
        for case in 0..16u32 {
            t.reset();
            let r = rng.next();
            t.stub(hdc::DBG_PRINTF, false, 2, 0, r);
            // open fails, seek fails, read fails, all fine; under and
            // over 2 GB
            let (open, seek, read) = [
                (0xffff_ffffu32, 0u32, 0u32),
                (3, 0xffff_fffe, 0),
                (4, 0, 0x8000_0000),
                (5, 0x10, 0x20),
            ][(case % 4) as usize];
            t.stub(hdc::SCE_OPEN, false, 2, 0, open);
            t.stub(hdc::SCE_LSEEK, false, 3, 0, seek);
            t.stub(hdc::SCE_LSEEK64, false, 3, 0, seek);
            t.stub(hdc::SCE_READ, false, 3, 0, read);
            t.stub(hdc::SCE_CLOSE, false, 1, 0, r ^ 1);
            t.write(t.rebase(hdc::HDD_ERR), &[0]);
            let offset = [0x800u32, 0x7fff_ffff, 0x8000_0000, 0xffff_f800][(case / 4) as usize];
            t.put(t.rebase(0x0112_a7f0), &[rng.next(), rng.next(), offset]);
            t.check(
                &format!("read th main #{case}"),
                0x0077_86f0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(hdb::read_th_main(m)),
            );
            // free space: devctl fails or writes the free sectors
            let free = [0u32, 0x8_0000, 0x7f_ffff, 0xffff_ffff][(case % 4) as usize];
            let fails = case & 4 != 0;
            t.stub_store(
                hdc::SCE_DEVCTL,
                None,
                4,
                free,
                false,
                6,
                0,
                if fails { 0x8000_0002 } else { 0 },
            );
            let need = [0u32, 1, 2, 0xffff_ffff][(case / 4 % 4) as usize];
            let args = [rng.next(), need];
            t.check_fs(
                &format!("free space #{case}"),
                0x0077_7610,
                &args,
                4,
                0,
                |m, ebp, _| Out::U32(hdb::free_space_ok(m, ebp, args[0], args[1])),
            );
            // the read-function table
            t.stub(hdc::PC_READ_FUNC_SET, false, 0, 0, r ^ 2);
            let cd = [0u32, 1, 0xffff_ffff, 0x100][(case % 4) as usize];
            t.check(
                &format!("read funcs #{case}"),
                0x0076_cff0,
                Conv::Cdecl,
                &[cd],
                |m| Out::U32(hdb::set_read_funcs(m, cd)),
            );
        }
    });
}

// ---- round 3, batch 3: RadialBlur's draws, the sphere grid's item check (`ffx_system::m5b_pc`) ----

#[test]
fn r3b3_blur_draws_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0901);
        for case in 0..12u32 {
            t.reset();
            let r = rng.next();
            t.stub(pcs::SET_PARAM58, true, 4, 4, r);
            t.stub(pcs::SET_TEXTURE_PARAM, true, 2, 2, r ^ 1);
            t.stub(pcs::SET_PARAM5C, true, 4, 4, r ^ 2);
            t.stub(pcs::SET_COLOR_TARGET, true, 2, 2, r ^ 3);
            t.stub(pcs::SET_DEPTH_TARGET, true, 1, 1, r ^ 4);
            t.stub(pcs::BEGIN_SCENE, true, 1, 1, [0, 7][(case & 1) as usize]);
            t.stub(pcs::RENDER_MESH_INSTANCE, true, 4, 4, r ^ 5);
            t.stub(pcs::END_SCENE, true, 1, 1, r ^ 6);
            let words: Vec<u32> = (0..0x50).map(|i| 0x5a5a_0000 + i).collect();
            let renderer = t.words(&words);
            let mesh = t.words(&[1, 2, 3, 4, rng.next()]);
            // the scales: plain, zero (x87 division by zero), negative,
            // a NaN
            let (sx, sy) = [
                (2.0f32.to_bits(), 4.0f32.to_bits()),
                (0, 1.5f32.to_bits()),
                ((-3.0f32).to_bits(), 0x8000_0000),
                (0x7fc0_1234, 0.1f32.to_bits()),
            ][(case / 2 % 4) as usize];
            let mut this = [0u32; 0x1c];
            this[0] = u32::from(case % 6 != 5);
            for w in &mut this[1..5] {
                *w = rng.next();
            }
            this[5] = sx;
            this[6] = sy;
            this[7] = renderer;
            this[0x19] = rng.next();
            this[0x1a] = rng.next();
            this[0x1b] = mesh;
            let this = t.words(&this);
            let tex = rng.next();
            let check = |t: &mut Twins,
                         what: &str,
                         sym: u32,
                         args: &[u32],
                         low: u32,
                         bridge: &dyn Fn(&mut G, u32) -> u32| {
                let ebp = t.frame_ebp(args.len());
                let lo = ebp - low;
                t.write(lo, &vec![0; low as usize]);
                t.b.set_call_top(Some(lo - 0x100));
                t.check(what, sym, Conv::Thiscall(this), args, |m| {
                    Out::U32(bridge(m, ebp))
                });
                t.b.set_call_top(None);
                let n = (low - 0xc) as usize;
                let mut x = vec![0; n];
                let mut y = vec![0; n];
                Mem::read_bytes(&t.a, lo, &mut x);
                Mem::read_bytes(&t.b, lo, &mut y);
                assert_eq!(x, y, "{what}: frame");
            };
            check(
                t,
                &format!("blur draw #{case}"),
                0x0070_30e0,
                &[tex, rng.next()],
                0x2c,
                &|m, ebp| pcb::blur_draw(m, ebp, this, tex),
            );
            check(
                t,
                &format!("blur draw scaled #{case}"),
                0x0070_3480,
                &[tex],
                0x3c,
                &|m, ebp| pcb::blur_draw_scaled(m, ebp, this, tex),
            );
        }
    });
}

#[test]
fn r3b3_abmap_item_check_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0902);
        for case in 0..10u32 {
            t.reset();
            let r = rng.next();
            // the grid: the character byte, its node table
            let abmap = t.zeros(0x11600);
            let chr = [0u8, 2, 6, 0x21][(case % 4) as usize];
            t.write(abmap + 0x115bc, &[chr]);
            t.write(
                abmap + u32::from(chr) * 80 + 0x110cc,
                &(0x1234u16 + case as u16).to_le_bytes(),
            );
            t.g32(pcs::ABMAP, abmap);
            let count = [0i16, 1, 3, 5, -1][(case % 5) as usize];
            let pad = t.zeros(0x1a44 + 12 * 6);
            t.write(pad + 0x1a1e, &count.to_le_bytes());
            for k in 0..6u32 {
                t.write(pad + 0x1a44 + 12 * k, &[0xa5, 0, 0, 0]);
                t.write(pad + 0x1a48 + 12 * k, &(0x2000u16 + k as u16).to_le_bytes());
            }
            t.g32(pcs::ABMAP_PAD, pad);
            t.g32(pcs::ABMAP_RUNNING, u32::from(case != 9));
            // stock, the item's ability byte, its record's kind
            t.stub_seq(pcs::SAVE_ITEM_NUM, 1, 0, &[1, 0, 3, 2, 1, 1]);
            let rom = t.zeros(0x60);
            t.write(rom + 0x5d, &[[0xff, 3][(case & 1) as usize]]);
            t.stub(pcs::ROM_ITEM, false, 2, 0, rom);
            let rec = t.zeros(0x10);
            t.write(rec + 0xc, &[[0x20, 1][((case / 2) & 1) as usize]]);
            t.stub_seq(pcs::EXCEL_DATA, 3, 0, &[rec, 0, rec, rec, 0, rec]);
            // the walks set the found flag (ctx + 0xc), one of them not
            let found = u32::from(case % 3 != 0);
            t.stub_store_into(pcs::FOR_EACH_NODE, 1, 0xc, found, 2, r ^ 7);
            t.stub_store_into(pcs::FOR_NODE_AND_LINKED, 2, 0xc, 1 - found, 3, r ^ 8);
            // the menu refill puts the count back, or leaves it emptied
            if case & 1 == 0 {
                t.stub_store_abs(
                    pcs::FILL_USE_ITEM_MENU,
                    pad + 0x1a1e,
                    u32::from(count.cast_unsigned()),
                    2,
                    r ^ 9,
                );
            } else {
                t.stub(pcs::FILL_USE_ITEM_MENU, false, 2, 0, r ^ 9);
            }
            t.check_fs(
                &format!("item usable #{case}"),
                0x00a4_4d30,
                &[],
                0x24,
                0,
                |m, ebp, _| Out::U32(pcb::abmap_item_usable(m, ebp)),
            );
            t.write(pad + 0x1a1e, &count.to_le_bytes());
            t.check_fs(
                &format!("refresh items #{case}"),
                0x00a4_4ea0,
                &[],
                // the frame holds AbmapFillUseItemMenu's call words in the original: not compared
                0,
                0,
                |m, ebp, _| pcb::abmap_refresh_items(m, ebp).map_or(Out::None, Out::U32),
            );
        }
    });
}

impl Twins {
    /// A recording stub over `sym` (`n` words) that first stores `value`
    /// at `[word + off]` (its stack word `word` a pointer); returns `ret`.
    fn stub_store_into(&mut self, sym: u32, word: u8, off: u8, value: u32, n: u32, ret: u32) {
        let log_stub = self.stub_code(sym, false, n, 0, ret);
        let mut c = vec![0x8b, 0x44, 0x24, 4 + 4 * word]; // mov eax, [esp+4+4word]
        c.extend([0xc7, 0x40, off]);
        c.extend(value.to_le_bytes()); // mov dword [eax+off], value
        c.push(0xe9);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(log_stub.wrapping_sub(here).to_le_bytes()); // jmp log_stub
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }

    /// A recording stub over `sym` (`n` words) that first stores the word
    /// `value` at the absolute address `at`; returns `ret`.
    fn stub_store_abs(&mut self, sym: u32, at: u32, value: u32, n: u32, ret: u32) {
        let log_stub = self.stub_code(sym, false, n, 0, ret);
        let mut c = vec![0xc7, 0x05];
        c.extend(at.to_le_bytes());
        c.extend(value.to_le_bytes()); // mov dword [at], value
        c.push(0xe9);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(log_stub.wrapping_sub(here).to_le_bytes()); // jmp log_stub
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }
}

// ---- round 3, batch 4: small menu wrappers, disk checks, sector reads, the install wall ----

#[test]
fn r3b4_menu_wrappers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0a01);
        for case in 0..8u32 {
            t.reset();
            let r = rng.next();
            for (k, (sym, n)) in [
                (mnc::WAIT_WIN_MAIN, 1),
                (mnc::MENU_ICON, 4),
                (mnc::MOVIE_MESSAGE_IMAGE, 6),
                (mnc::FONT_8EBC00, 8),
                (mnc::MES_FONT_1612, 9),
                (mnc::FONT_8EA680, 10),
                (mnc::DAPS_STRING, 1),
                (mnc::GS_TEST, 8),
                (mnc::GS_WORDS4, 4),
                (mnc::DRAW_KICK, 0),
                (mnc::SET_STORE_IMAGE, 8),
                (mnc::GS_FLUSH_CACHE, 1),
                (mnc::EXEC_STORE_IMAGE, 2),
            ]
            .into_iter()
            .enumerate()
            {
                t.stub(sym, false, n, 0, r ^ (k as u32 * 0x10101));
            }
            t.stub_seq(mnc::GS_XYZ2_OFFSET, 2, 0, &[r ^ 0x55, r ^ 0x66]);
            t.stub_seq(mnc::GS_SYNC_PATH, 2, 0, &[r ^ 0x77, r ^ 0x88]);
            let w: [u32; 8] = std::array::from_fn(|_| rng.word());
            let win = t.zeros(0x90);
            t.check("wait win start", 0x008a_8750, Conv::Cdecl, &[win], |m| {
                Out::U32(mnb::wait_win_start(m, win))
            });
            let buf = t.zeros(0x40);
            t.g32(mnc::PKT_CURSOR, buf);
            ctrl_state(t, case, r ^ 0x10101);
            let w3 = [w[0], w[1], w[2]];
            t.check("menu icon", 0x008f_d4d0, Conv::Cdecl, &w3, |m| {
                Out::U32(mnb::menu_icon(m, w3))
            });
            let w5 = [w[0], w[1], w[2], w[3], w[4]];
            t.check("movie image", 0x0090_2840, Conv::Cdecl, &w5, |m| {
                Out::U32(mnb::movie_message_image(m, w5))
            });
            let w4 = [w[0], w[1], w[2], w[3]];
            t.check("font 8ebc00", 0x008e_bbd0, Conv::Cdecl, &w4, |m| {
                Out::U32(mnb::font_8ebc00(m, w4))
            });
            t.check("mes font 1612", 0x008e_9fd0, Conv::Cdecl, &w4, |m| {
                Out::U32(mnb::mes_font_1612(m, w4))
            });
            t.check("font 8ea680", 0x008e_a5c0, Conv::Cdecl, &w5, |m| {
                Out::U32(mnb::font_8ea680(m, w5))
            });
            t.g32(mnc::PKT_CURSOR, buf);
            ctrl_state(t, case, r ^ 0x66);
            t.check("flat rect", 0x008f_c6a0, Conv::Cdecl, &w, |m| {
                Out::U32(mnb::flat_rect(m, w))
            });
            // the message width: TOGetMessageWH writes the float width;
            // and the height word (its low half signed)
            let width = [0.0f32, 13.5, -7.25, 70000.0, 1e10][(case % 5) as usize];
            let height = [0x0000_0010u32, 0xfffe_8123, 0x7fff][(case % 3) as usize];
            t.stub_store2(mnc::MESSAGE_WH, [width.to_bits(), height], 6, r ^ 1);
            // 1e10 (case 4) under SSE, past i32 under both rules
            t.g32(0x0193_ed20, (case >> 2) & 1); // __sse2_available
            let w2 = [w[0], w[1]];
            t.check_fs("width ext", 0x008b_8070, &w2, 8, 0, |m, ebp, _| {
                Out::U32(mnb::width_ext_mes_1612(m, ebp, w2[0], w2[1]))
            });
            // TODapsPrintf: vsprintf gets the caller's word list
            imp(t, mnc::VSPRINTF_IAT, 3, false, r ^ 2);
            let daps = [w[0], w[1], w[2], w[3]];
            t.check_fs("daps printf", 0x0089_dde0, &daps, 0x104, 4, |m, ebp, s| {
                Out::U32(mnb::daps_printf(m, ebp, daps[0], s + 4))
            });
            // TOTransImageLH: its frame is aligned to 16 bytes
            let a5 = [0u32, 64, 100, 0xffff_ffc1, 0x8000_0000][(case % 5) as usize];
            let lh = [w[0], w[1], w[2], w[3], w[4], a5, w[5]];
            let entry = t.frame_ebp(lh.len()) + 4;
            let ebp = (entry - 0xc) & !0xf;
            t.check_fs("trans image lh", 0x008b_0da0, &lh, 0, 0, |m, _, _| {
                Out::U32(mnb::trans_image_lh(m, ebp, lh))
            });
        }
    });
}

#[test]
fn r3b4_disk_stream_and_wall_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0a02);
        for case in 0..16u32 {
            t.reset();
            let r = rng.next();
            // the cached disk check
            let ctx = t.zeros(0x5_0760);
            t.write(ctx + 0x5_075d, &[u8::from(case & 1 != 0)]);
            t.write(
                t.rebase(hdc::DISK_RESULT),
                &[[0x80u8, 1, 0xff, 0][((case >> 1) % 4) as usize]],
            );
            t.stub(
                hdc::DISK_CHANGED,
                false,
                0,
                0,
                [0, 1, 0, 0x100][(case / 2 % 4) as usize],
            );
            t.check("disk changed", 0x0077_75e0, Conv::Cdecl, &[ctx], |m| {
                Out::U32(hdb::disk_changed_cached(m, ctx))
            });
            // the sector stream: used bytes and offsets around sector ends
            let used = [0u32, 1, 0x7ff, 0x400][(case % 4) as usize];
            let offset = [0u32, 0x800, 0x801, 0x7ff, 0xffff_f000, 0x1234][(case % 6) as usize];
            t.g32(hdc::STREAM_USED, used);
            t.g32(hdc::STREAM_SECTOR, 0x100 + case);
            t.stub(hdc::READ_SECTORS, false, 5, 0, r);
            let skip = t.words(&[0xa5a5_a5a5]);
            let args = [offset, rng.next(), skip, rng.next()];
            t.check("stream read", 0x0077_7380, Conv::Cdecl, &args, |m| {
                Out::U32(hdb::stream_read(m, args))
            });
            t.stub(hdc::READ_MODULE_FILES, false, 0, 0, r ^ 1);
            t.check("read module files", 0x0077_7970, Conv::Cdecl, &[], |m| {
                Out::U32(hdb::read_module_files(m))
            });
            // the wall: progress at, past and before the mark
            let (done, total) =
                [(0u32, 0u32), (5, 10), (10, 10), (0xffff_fff0, 3)][(case % 4) as usize];
            t.g32(hdc::DONE, done);
            t.g32(hdc::TOTAL, total);
            let progress = if total == 0 {
                0
            } else {
                ((done << 6) as i32 / total as i32) as u32
            };
            let mark = if case & 4 == 0 {
                progress
            } else {
                progress + 1
            };
            t.g32(hdc::WALL_MARK, mark);
            t.g32(
                hdc::WALL_FRAME,
                [0u32, 3, 4, 0xffff_fffe][(case % 4) as usize],
            );
            t.g32(hdc::MENU_WORK, rng.next());
            t.stub(hdc::WALL_DRAW, false, 2, 0, r ^ 2);
            t.stub(hdc::DBG_PRINTF, false, 2, 0, r ^ 3);
            t.stub(hdc::WALL_SPRITE, false, 0, 0, r ^ 4);
            t.check("kabe disp", 0x0077_93c0, Conv::Cdecl, &[], |m| {
                Out::U32(hdb::kabe_disp(m))
            });
        }
    });
}

impl Twins {
    /// A recording stub over `sym` (cdecl, `n` words) that first writes
    /// `v[0]` through its third word and `v[1]` through its fourth (the
    /// `&w`, `&h` of `TOGetMessageWH`); returns `ret`.
    fn stub_store2(&mut self, sym: u32, v: [u32; 2], n: u32, ret: u32) {
        let log_stub = self.stub_code(sym, false, n, 0, ret);
        let mut c = vec![0x8b, 0x44, 0x24, 0x0c]; // mov eax, [esp+0xc]
        c.extend([0xc7, 0x00]);
        c.extend(v[0].to_le_bytes()); // mov dword [eax], v0
        c.extend([0x8b, 0x44, 0x24, 0x10]); // mov eax, [esp+0x10]
        c.extend([0xc7, 0x00]);
        c.extend(v[1].to_le_bytes()); // mov dword [eax], v1
        c.push(0xe9);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(log_stub.wrapping_sub(here).to_le_bytes()); // jmp log_stub
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }
}

#[test]
fn r3b4_frame_buffer_install_mount_main_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0a03);
        for case in 0..8u32 {
            t.reset();
            let r = rng.next();
            t.stub(mnc::FRAME_BUFFER_ARG, false, 0, 0, r);
            t.stub(mnc::DRAW_UI_ELEMENT, false, 5, 0, r ^ 1);
            t.g32(mnc::FRAME_BUFFER_RGB, rng.next());
            t.check_fs("frame buffer", 0x008a_92a0, &[], 0x9c, 4, |m, ebp, _| {
                Out::U32(mnb::real_frame_buffer(m, ebp))
            });
            // the install mode: each kind, negative ones, 0x0112a514
            let kind = [0u32, 1, 2, 3, 4, 0xffff_fffe, 0x8000_0000, 0x13][(case % 8) as usize];
            t.stub(hdc::INSTALL_KIND, false, 0, 0, kind);
            t.stub(hdc::DBG_PRINTF, false, 2, 0, r ^ 0x200);
            t.g32(hdc::WORD_A514, [0u32, 0xffff_ffff][(case / 4 % 2) as usize]);
            t.write(t.rebase(hdc::HDD_MODE), &[[0x80u8, 1][(case & 1) as usize]]);
            t.g32(hdc::WORD_A518, 0x77);
            t.check("install init", 0x0077_92e0, Conv::Cdecl, &[], |m| {
                Out::U32(hdb::install_init(m))
            });
            // the mount: mounted, failing, succeeding
            t.write(t.rebase(hdc::HDD_MOUNTED), &[u8::from(case % 3 == 0)]);
            t.stub(hdc::MOUNT_PREPARE, false, 0, 0, r ^ 2);
            t.stub(hdc::PARTITION_NAME, false, 1, 0, r ^ 3);
            imp(t, hdc::SPRINTF_IAT, 4, false, r ^ 4);
            t.stub(hdc::MOUNT_PRE2, false, 0, 0, r ^ 5);
            t.stub(hdc::MOUNT_LOG, false, 1, 0, r ^ 6);
            t.stub_two(hdc::DBG_PRINTF, 1, 2, r ^ 7);
            t.stub(
                hdc::SCE_MOUNT,
                false,
                5,
                0,
                [0u32, 0xffff_fffb, 7][(case % 3) as usize],
            );
            t.check_fs("mount", 0x0077_9490, &[], 0x84, 4, |m, ebp, _| {
                Out::U32(hdb::mount(m, ebp))
            });
            // main: count 0..2, "debug", "DEBUG", "debu", "debugs"
            let words = [&b"debug\0"[..], b"DEBUG\0", b"debu\0", b"debugs\0"];
            let arg = t.alloc(words[((case / 2 + case) % 4) as usize]);
            let argv = t.words(&[rng.next(), arg]);
            let count = [2u32, 1, 3, 0xffff_ffff][(case / 2 % 4) as usize];
            t.stub(pcs::MAIN_PREPARE, false, 0, 0, r ^ 8);
            t.stub(pcs::MAIN_INIT, false, 2, 0, r ^ 9);
            t.stub(pcs::MAIN_LOOP, false, 2, 0, r ^ 0xa);
            t.check("main", 0x0082_2490, Conv::Cdecl, &[count, argv], |m| {
                Out::U32(pcb::game_main(m, count, argv))
            });
        }
    });
}

#[test]
fn r3b5_capture_offsets_install_check_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0b01);
        for case in 0..18u32 {
            t.reset();
            let r = rng.next();
            // the capture: off, on by the menu flag or the option, the
            // display modes 0, 6 and others
            let menu = t.words(&[[0u32, 0x200, 0xffff_fdff][(case % 3) as usize]]);
            t.stub(mnc::SAVE_MENU, false, 0, 0, menu);
            t.g32(mnc::CAPTURE_OPTION, u32::from(case % 4 == 3));
            t.stub_seq(
                mnc::DISP_MODE,
                0,
                0,
                &[
                    [0u32, 1, 6, 2][(case / 3 % 4) as usize],
                    [3, 6, 1, 6][(case % 4) as usize],
                ],
            );
            t.stub(hdc::DBG_PRINTF, false, 2, 0, r);
            t.stub(mnc::TRANS_IMAGE_LH, false, 7, 0, r ^ 1);
            t.stub(mnc::MESSAGE_WINDOW_MODE, false, 1, 0, r ^ 2);
            t.g32(mnc::CAPTURE_BUF[0], rng.next());
            let xy = [rng.next(), rng.next()];
            t.check(
                &format!("capture #{case}"),
                0x008a_daf0,
                Conv::Cdecl,
                &xy,
                |m| Out::U32(mnb::capture_screen(m, xy[0], xy[1])),
            );
            // the CD offsets: found at once or after a wait
            t.stub_seq(
                hdc::CD_SEARCH_FILE,
                2,
                0,
                &[1, 0, 1, u32::from(case & 1 == 0), 1, 1],
            );
            t.stub(hdc::SYNC_V, false, 1, 0, r ^ 3);
            for sym in hdc::FILE_OFFSETS {
                t.g32(sym, 0x5a5a_5a5a);
            }
            let names = if case % 5 == 4 {
                0
            } else {
                t.words(&[rng.next(), rng.next(), rng.next()])
            };
            t.check_fs(
                &format!("file offsets #{case}"),
                0x0076_db40,
                &[names],
                0x28,
                4,
                |m, ebp, _| hdb::file_offset_get(m, ebp, names).map_or(Out::None, Out::U32),
            );
            // the installed files: heads with sizes, read modes, opens
            let heads = t.rebase(hdc::HEADS);
            let n = [0u32, 2, 3, 0xffff_ffff][(case % 4) as usize];
            t.g32(hdc::HEAD_COUNT, n);
            for k in 0..3u32 {
                t.put(
                    heads + 12 * k,
                    &[0x10, [0, 0x800, 0x1000][((k + case) % 3) as usize], 0],
                );
            }
            t.stub_seq(hdc::RMODE_GET, 1, 0, &[1, u32::from(case & 2 != 0), 1]);
            t.stub(hdc::HDD_NAME, false, 1, 0, r ^ 4);
            t.stub_seq(
                hdc::SCE_OPEN,
                2,
                0,
                &[3, [0x8000_0001, 4][(case & 1) as usize], 5],
            );
            t.stub_seq(hdc::SCE_LSEEK64, 3, 0, &[0x800, 0x1000, 0x7ff]);
            t.stub(hdc::SCE_CLOSE, false, 1, 0, r ^ 5);
            t.stub(hdc::MOUNT_LOG, false, 1, 0, r ^ 6);
            // the format alone: the calls pass one to three words
            t.stub(hdc::DBG_PRINTF, false, 1, 0, r ^ 7);
            t.check(
                &format!("verify files #{case}"),
                0x0077_87c0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(hdb::verify_files(m)),
            );
            // the install check
            let kind = [0u32, 0x10, 0xffff_fff1, 3, 4, 0][(case % 6) as usize];
            t.stub(hdc::INSTALL_KIND, false, 0, 0, kind);
            t.stub(hdc::HDD_MOUNT, false, 0, 0, u32::from(case % 4 == 1));
            t.stub(hdc::VERIFY_FILES, false, 0, 0, u32::from(case % 4 == 2));
            t.stub(hdc::INS_CHECK, false, 1, 0, r ^ 8);
            t.stub(hdc::HDD_UNMOUNT, false, 0, 0, r ^ 9);
            t.stub(hdc::REFRESH_HDD_MODE, false, 0, 0, r ^ 0xa);
            // kind 0 reaching hdd_ins_check: cases 0 (both set), 11 (CD 0),
            // 12 (byte 0)
            t.write(t.rebase(hdc::BYTE_8A86), &[u8::from(case != 12)]);
            t.g32(hdc::READ_FUNCS_CD, u32::from(case != 11));
            t.g32(hdc::WORD_A514, 0x5a5a);
            t.write(t.rebase(hdc::HDD_MODE), &[0x55]);
            t.stub(hdc::DBG_PRINTF, false, 2, 0, r ^ 0xb);
            t.check(
                &format!("install check #{case}"),
                0x0077_91a0,
                Conv::Cdecl,
                &[r],
                |m| Out::U32(hdb::install_check(m, r)),
            );
        }
    });
}

#[test]
fn r3b5_partition_help_picture_map_reload_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0b02);
        for case in 0..8u32 {
            t.reset();
            let r = rng.next();
            // the partition: mounted or not, open failing, ioctl failing,
            // space or not
            t.write(t.rebase(hdc::HDD_MOUNTED), &[u8::from(case & 1 != 0)]);
            t.g32(hdc::WORD_A514, 0x5a5a);
            for (k, sym) in [
                hdc::HDD_UNMOUNT,
                hdc::MOUNT_PREPARE,
                hdc::MOUNT_PRE2,
                hdc::HDD_MOUNT,
            ]
            .into_iter()
            .enumerate()
            {
                t.stub(sym, false, 0, 0, r ^ (k as u32 + 1));
            }
            t.stub(hdc::PARTITION_NAME, false, 1, 0, r ^ 5);
            imp(t, hdc::SPRINTF_IAT, 4, false, r ^ 6);
            t.stub(
                hdc::SCE_OPEN,
                false,
                2,
                0,
                [3u32, 0x8000_0000][((case >> 1) & 1) as usize],
            );
            t.stub(
                hdc::SCE_IOCTL2,
                false,
                6,
                0,
                [0x40u32, 0xffff_fff0][((case >> 2) & 1) as usize],
            );
            t.stub(hdc::SCE_DCLOSE, false, 1, 0, r ^ 7);
            t.stub(hdc::INSTALLED_MB, false, 1, 0, 0x1234_5678 ^ r);
            t.stub(hdc::DBG_PRINTF, false, 1, 0, r ^ 8);
            t.stub(hdc::FREE_SPACE_OK, false, 2, 0, u32::from(case % 3 != 0));
            t.check_fs(
                &format!("open partition #{case}"),
                0x0077_9000,
                &[],
                0x44,
                4,
                |m, ebp, _| Out::U32(hdb::open_ffx_partition(m, ebp)),
            );
            // the help picture: tables of known bytes
            let table = t.zeros(0x1_0400 + 0x24c1 * 3);
            let bytes: Vec<u8> = (0..0x24c1 * 3).map(|_| rng.next() as u8).collect();
            t.write(table + 0x1_0400, &bytes);
            t.g32(hdc::HELP_TABLE, table);
            let picture = t.alloc(&[0xa5; 0x4000]);
            t.g32(hdc::HELP_PICTURE, picture);
            t.stub_copy_fill(hdc::MEMSET);
            t.stub(hdc::SET_LOAD_IMAGE, false, 8, 0, r ^ 9);
            t.stub(hdc::FLUSH_CACHE, false, 1, 0, r ^ 0xa);
            t.stub(hdc::EXEC_LOAD_IMAGE, false, 2, 0, r ^ 0xb);
            t.stub_seq(hdc::GS_SYNC_PATH, 2, 0, &[r ^ 0xc, r ^ 0xd]);
            let args = [case % 3, rng.next()];
            let entry = t.frame_ebp(args.len()) + 4;
            let ebp = (entry - 0xc) & !0xf;
            t.check_fs(
                &format!("help picture #{case}"),
                0x0077_9ad0,
                &args,
                0,
                0,
                |m, _, _| Out::U32(hdb::help_picture(m, ebp, args[0], args[1])),
            );
            // the map objects: unknown map, a chunk held, loaded or not
            t.stub_store(
                pcs::EVENT_JUMP_MAP,
                None,
                0,
                0x8061_7a41 ^ (case << 8),
                false,
                1,
                0,
                r ^ 0xe,
            );
            t.stub(pcs::STAGE_NAME, false, 2, 0, u32::from(case % 4 != 3));
            imp(t, pcs::SPRINTF_IAT, 7, false, r ^ 0xf);
            let map = t.words(&[0, 0, 0]);
            let guide = t.words(&[0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67]);
            t.put(map + 8, &[guide]);
            let system = t.words(&[map, rng.next(), 0, 0, 0, [0u32, 5][(case & 1) as usize]]);
            t.g32(pcs::YN_MAP_SYSTEM, system);
            t.stub(pcs::RC_PRINT, false, 1, 0, r ^ 0x10);
            let chunk = if case & 4 == 0 {
                t.words(&[rng.next(), 0])
            } else {
                0
            };
            t.stub(pcs::CHUNK_NEW, false, 0, 0, chunk);
            t.stub(
                pcs::CHUNK_LOAD,
                false,
                2,
                0,
                u32::from(case & 2 != 0 && chunk != 0),
            );
            t.stub(pcs::BG_LOAD_BIN, false, 1, 0, r ^ 0x11);
            t.stub(pcs::GUIDE_SET_MODEL, false, 2, 0, r ^ 0x12);
            t.stub(pcs::CHUNK_DELETE, false, 1, 0, r ^ 0x13);
            t.check_fs(
                &format!("map reload #{case}"),
                0x0092_cdd0,
                &[],
                0x10c,
                4,
                |m, ebp, _| Out::U32(pcb::reload_map_objects(m, ebp)),
            );
        }
    });
}

impl Twins {
    /// A `memset(dst, c, n)` stub over `sym` (cdecl): fills and logs the
    /// three words; returns `dst`.
    fn stub_copy_fill(&mut self, sym: u32) {
        let log_stub = self.stub_code(sym, false, 3, 0, 0);
        let mut c = vec![0x57]; // push edi
        c.extend([0x8b, 0x7c, 0x24, 0x08]); // mov edi, [esp+8] (dst)
        c.extend([0x8b, 0x44, 0x24, 0x0c]); // mov eax, [esp+0xc] (c)
        c.extend([0x8b, 0x4c, 0x24, 0x10]); // mov ecx, [esp+0x10] (n)
        c.extend([0xf3, 0xaa]); // rep stosb
        c.push(0x5f); // pop edi
        c.push(0xe9);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(log_stub.wrapping_sub(here).to_le_bytes()); // jmp log_stub
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }
}

#[test]
fn r3b5_printf2_install_thread_disc_wait_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0b03);
        let text = t.alloc(b"MAP: loaded field ffx_base 0123456789 abcdefghij\0");
        for case in 0..16u32 {
            t.reset();
            let r = rng.next();
            // printf2: vsprintf writes the line; the switches and filters
            t.fill_import(mnc::VSPRINTF_IAT, 3, 0, Some(2), 0, text, r);
            t.stub_import_copy(mnc::STRNCPY_IAT);
            t.g32(mnc::LOG_INDEX, [0u32, 5, 0x7f, 0x80][(case % 4) as usize]);
            t.stub(
                mnc::DEBUG_SWITCH,
                false,
                0,
                0,
                // mode 4 (cases 2, 3, 10, 11) sees 1 and 2
                [1u32, 0, 1, 2][((case / 4 + case) % 4) as usize],
            );
            let config = t.words(&[0, 0, 0x0100_0000 * (case & 1), 0]);
            t.stub(mnc::FFX_CONFIG, false, 0, 0, config);
            let filter: &[u8] = [
                &b"\0"[..],
                b"MAP\0",
                b"MAP: loaded field ffx_base 0123456789 abcdefghij\0",
                b"XX\0",
            ][(case % 4) as usize];
            t.write(t.rebase(mnc::LOG_FILTER), filter);
            t.write(t.rebase(mnc::LOG_FILTER_SENSE), &[u8::from(case & 2 != 0)]);
            t.write(
                t.rebase(mnc::LOG_MODE),
                &[[3u8, 4, 0, 0xff][(case / 2 % 4) as usize]],
            );
            t.stub(hdc::DBG_PRINTF, false, 1, 0, r ^ 1);
            t.stub(mnc::LOG_SEND, false, 3, 0, r ^ 2);
            t.stub(mnc::LOG_DUMP, false, 2, 0, r ^ 3);
            let args = [rng.next(), [0u32, 3, 15, 0x1f][(case % 4) as usize]];
            t.check_fs(
                &format!("printf2 #{case}"),
                0x0076_dd10,
                &args,
                0x204,
                4,
                |m, ebp, _| Out::U32(mnb::printf2(m, ebp, args[0], args[1])),
            );
            // the install thread: each setup and copy result
            let setup = [0u32, 1, 2, 3][(case % 4) as usize];
            let copy = [0u32, 1, 2, 0][(case / 4 % 4) as usize];
            t.stub(hdc::INSTALL_SETUP, false, 0, 0, setup);
            let setter = t.stub_code(0x0e0f_0201, false, 1, 0, r ^ 4);
            t.g32(hdc::SET_READ_FUNCS_CELL, setter);
            t.g32(hdc::READ_FUNCS_CD, rng.next());
            for (k, sym) in [
                hdc::HDD_MOUNT,
                hdc::HDD_UNMOUNT,
                hdc::MOUNT_PREPARE,
                hdc::MOUNT_PRE2,
                hdc::DISK_OPEN_CHECK_SET,
            ]
            .into_iter()
            .enumerate()
            {
                t.stub(
                    sym,
                    false,
                    u32::from(sym == hdc::DISK_OPEN_CHECK_SET),
                    0,
                    r ^ (0x10 + k as u32),
                );
            }
            t.stub(hdc::INSTALL_COPY, false, 0, 0, copy);
            t.stub(hdc::PARTITION_NAME, false, 1, 0, r ^ 5);
            imp(t, hdc::SPRINTF_IAT, 5, false, r ^ 6);
            t.stub(
                hdc::SCE_OPEN,
                false,
                2,
                0,
                [3u32, 0xffff_fff0][((case / 4) & 1) as usize],
            );
            t.stub(hdc::INSTALL_FINISH, false, 1, 0, r ^ 7);
            t.stub(hdc::SCE_CLOSE, false, 1, 0, r ^ 8);
            t.stub(hdc::DBG_PRINTF, false, 1, 0, r ^ 9);
            t.stub(hdc::USER_FREE, false, 1, 0, r ^ 0xa);
            t.put(
                t.rebase(hdc::BUFFERS[0]),
                &[[0u32, 0x1234][(case & 1) as usize]],
            );
            t.put(
                t.rebase(hdc::BUFFERS[1]),
                &[[0x5678u32, 0][(case & 1) as usize]],
            );
            t.check_fs(
                &format!("install thread #{case}"),
                0x0077_95b0,
                &[],
                0x44,
                4,
                |m, ebp, _| Out::U32(hdb::install_thread(m, ebp)),
            );
            // the disc wait: status, readiness, read and compare outcomes
            let off = [[0u32, 2][(case / 4 % 2) as usize]];
            let status: &[u32] =
                [&off[..], &[1, 0, 0, 0], &[1, 1, 0, 0, 0], &[1, 0, 1, 0, 0]][(case % 4) as usize];
            t.stub_seq(hdc::CD_STATUS, 0, 0, &[status, &[0; 8][..]].concat());
            t.stub_seq(hdc::DISK_READY, 1, 0, &[0, 2, 2, 2, 2, 2, 2, 2]);
            t.stub(hdc::TASK_WAIT_SYM, false, 1, 0, r ^ 0xb);
            t.write(t.rebase(hdc::DISK_CHECK_ON), &[u8::from(case % 8 != 1)]);
            let header: Vec<u32> = (0..8).map(|k| 0x4000 + k).collect();
            let expected = if case % 8 == 2 { 0 } else { t.words(&header) };
            t.g32(hdc::DISK_HEADER, expected);
            let mut read = header.clone();
            if case & 4 != 0 {
                read[7] ^= 1;
            }
            let buffer = t.words(&[read, vec![0xa5a5_a5a5; 4]].concat());
            t.g32(hdc::DISK_BUFFER, buffer);
            t.stub(hdc::STATE_CLEAR, false, 0, 0, r ^ 0xc);
            t.stub(hdc::MODULE_FILE_SECTOR, false, 2, 0, 0x99);
            t.stub(hdc::FLUSH_CACHE, false, 1, 0, r ^ 0xd);
            t.stub_seq(hdc::CD_READ, 4, 0, &[u32::from(case & 8 == 0), 1, 1, 1]);
            t.stub_seq(
                hdc::DISK_CHANGED,
                0,
                0,
                &[
                    0xffff_ffff,
                    0xffff_ffff,
                    [0u32, 5][(case & 1) as usize],
                    0,
                    0,
                    0,
                ],
            );
            // a mismatch would wait forever: the mismatch message (the
            // n-th print of the case) puts the right word in
            let nth = match case {
                6 => 3,
                5 | 7 | 14 => 5,
                13 | 15 => 7,
                _ => 0,
            };
            t.stub_store_nth(hdc::DBG_PRINTF, buffer + 28, header[7], nth, r ^ 0xe);
            t.check(
                &format!("disc wait #{case}"),
                0x0076_ccb0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(hdb::wait_disc(m)),
            );
        }
    });
}

impl Twins {
    /// A recording stub over `sym` (cdecl, one word logged) that on its
    /// `nth` call (1-based; 0 never) first stores `value` at the absolute
    /// address `at`; returns `ret`.
    fn stub_store_nth(&mut self, sym: u32, at: u32, value: u32, nth: u32, ret: u32) {
        let log_stub = self.stub_code(sym, false, 1, 0, ret);
        let ctr = self.words(&[0]);
        let mut c = vec![0xff, 0x05];
        c.extend(ctr.to_le_bytes()); // inc dword [ctr]
        c.extend([0x81, 0x3d]);
        c.extend(ctr.to_le_bytes());
        c.extend(nth.to_le_bytes()); // cmp dword [ctr], nth
        c.extend([0x75, 0x0a]); // jne +10
        c.extend([0xc7, 0x05]);
        c.extend(at.to_le_bytes());
        c.extend(value.to_le_bytes()); // mov dword [at], value
        c.push(0xe9);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(log_stub.wrapping_sub(here).to_le_bytes()); // jmp log_stub
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }
}

#[test]
fn r3b6_box_textured_wall_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0c01);
        for case in 0..12u32 {
            t.reset();
            let r = rng.next();
            t.stub(mnc::DRAW_UI_ELEMENT, false, 5, 0, r);
            let full = [1u32, 0x40, 3, 0xffff_ffff][(case % 4) as usize];
            let w = [0u32, 0x20, 0x40, 0xffff_fff0][(case / 4 % 3) as usize];
            let args = [
                rng.word(),
                rng.word(),
                w,
                rng.word(),
                rng.next(),
                rng.next(),
                full,
            ];
            t.check_fs(
                &format!("box fw #{case}"),
                0x008f_3eb0,
                &args,
                0xa4,
                4,
                |m, ebp, _| Out::U32(mnb::box_fill_width(m, ebp, args)),
            );
            let size = [1u32, 0x10, 0xffff_ffff, 0x7fff_ffff][(case % 4) as usize];
            let args = [
                rng.word(),
                rng.word(),
                size,
                [3u32, 1, 0x8000_0000][(case % 3) as usize],
                rng.next(),
            ];
            t.check_fs(
                &format!("ui textured #{case}"),
                0x0077_4520,
                &args,
                0xa0,
                4,
                |m, ebp, _| Out::U32(mnb::ui_textured(m, ebp, args)),
            );
            // the wall: 6 frames of small sizes
            let widths = [4i16, 3, 5, 2, 1, 6];
            let heights = [2i16, 3, 1, 4, 0, 2];
            for k in 0..6u32 {
                t.write(
                    t.rebase(hdc::WALL_WIDTHS) + 2 * k,
                    &widths[k as usize].to_le_bytes(),
                );
                t.write(
                    t.rebase(hdc::WALL_HEIGHTS) + 2 * k,
                    &heights[k as usize].to_le_bytes(),
                );
            }
            let bytes: Vec<u8> = (0..0x1500).map(|_| rng.next() as u8).collect();
            let pictures = t.alloc(&bytes);
            t.g32(hdc::WALL_PICTURES, pictures);
            let picture = t.alloc(&vec![0xa5; 0x4_0000]);
            t.g32(hdc::HELP_PICTURE, picture);
            t.stub_copy_fill(hdc::MEMSET);
            t.stub(hdc::SET_LOAD_IMAGE, false, 8, 0, r ^ 1);
            t.stub(hdc::FLUSH_CACHE, false, 1, 0, r ^ 2);
            t.stub(hdc::EXEC_LOAD_IMAGE, false, 2, 0, r ^ 3);
            t.stub_seq(hdc::GS_SYNC_PATH, 2, 0, &[r ^ 4, r ^ 5]);
            let args = [case % 6, rng.next()];
            let entry = t.frame_ebp(args.len()) + 4;
            let ebp = (entry - 0xc) & !0xf;
            t.check_fs(
                &format!("wall frame #{case}"),
                0x0077_98b0,
                &args,
                0,
                0,
                |m, _, _| Out::U32(hdb::wall_frame(m, ebp, args[0], args[1])),
            );
        }
    });
}

#[test]
fn r3b6_limit_gauge_param_plate_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0c02);
        for case in 0..12u32 {
            t.reset();
            let r = rng.next();
            let layout: Vec<u8> = (0..0x30).map(|_| rng.next() as u8).collect();
            t.write(t.rebase(mnc::PLATE_LAYOUT), &layout);
            t.stub_seq(mnc::BOX_RGBA, 8, 0, &[r, r ^ 1]);
            t.stub(mnc::BOX_FILL_WIDTH, false, 7, 0, r ^ 2);
            // max, value: zero ones, full, past full, partial
            let (max, now) = [
                (0u32, 5u32),
                (7, 0),
                (7, 7),
                (7, 9),
                (9, 4),
                (0x7fff_ffff, 3),
            ][(case % 6) as usize];
            t.stub(mnc::LIMIT_MAX, false, 1, 0, max);
            t.stub(mnc::LIMIT_NOW, false, 1, 0, now);
            t.g32(0x0193_ed20, (case / 6) & 1); // __sse2_available
            let args = [rng.next(), rng.next(), rng.next()];
            t.check_fs(
                &format!("limit gauge #{case}"),
                0x0089_5480,
                &args,
                0,
                0,
                |m, _, s| {
                    mnb::limit_gauge(m, args[0], args[1], args[2], s).map_or(Out::None, Out::U32)
                },
            );
            t.stub(mnc::MKP_SHAPE_XY, false, 3, 0, r ^ 3);
            t.stub(mnc::HP_MAX, false, 1, 0, r ^ 4);
            t.stub(mnc::HP_NOW, false, 1, 0, r ^ 5);
            t.stub(
                mnc::WEAK_LEVEL,
                false,
                2,
                0,
                [0u32, 1, 2, 3, 4, 0xffff_ffff][(case % 6) as usize],
            );
            t.stub(mnc::MP_NOW, false, 1, 0, r ^ 6);
            t.stub(mnc::VALUE_FONT_M_CLUT_RIGHT, false, 4, 0, r ^ 7);
            t.stub(mnc::LIMIT_GAUGE, false, 3, 0, r ^ 8);
            t.check_fs(
                &format!("param plate #{case}"),
                0x0089_5940,
                &args,
                0,
                0,
                |m, _, s| Out::U32(mnb::param_plate(m, args[0], args[1], args[2], s)),
            );
        }
    });
}

#[test]
fn r3b6_movie_image_bar_caps_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0c03);
        for case in 0..6u32 {
            t.reset();
            let r = rng.next();
            let p = t.zeros(0x80);
            t.stub(mnc::ADP_TEX0, false, 13, 0, p + 0x40);
            t.stub_seq(mnc::ADP_WORDS4, 5, 0, &[r, r ^ 1, r ^ 2, r ^ 3]);
            t.stub_seq(mnc::ADP_XY2, 3, 0, &[r ^ 4, r ^ 5, r ^ 6, r ^ 7]);
            t.stub_seq(
                mnc::ADP_XYZ2_OFFSET,
                3,
                0,
                &[r ^ 8, r ^ 9, r ^ 0xa, r ^ 0xb],
            );
            let args = [
                p,
                rng.next(),
                rng.word(),
                rng.word(),
                rng.word(),
                rng.word(),
            ];
            t.check(
                &format!("movie image #{case}"),
                0x008f_2e10,
                Conv::Cdecl,
                &args,
                |m| Out::U32(mnb::movie_image(m, args)),
            );
            // the bar: the alpha call moves the cursor in some cases
            let q = t.zeros(0x80);
            if case & 1 == 0 {
                t.stub(mnc::GS_ALPHA, false, 5, 0, r ^ 0xc);
            } else {
                t.stub_store_global(mnc::GS_ALPHA, mnc::PKT_CURSOR, q + 0x40, r ^ 0xc);
            }
            t.stub(mnc::GS_WORDS4, false, 4, 0, r ^ 0xd);
            // the real TOGs* calls move the cursor: odd cases
            if case & 1 == 0 {
                t.stub(mnc::GS_XY2, false, 2, 0, r ^ 0xe);
            } else {
                t.stub_store_global(mnc::GS_XY2, mnc::PKT_CURSOR, r, r ^ 0xe);
            }
            t.stub(mnc::GS_XYZ2_OFFSET, false, 2, 0, r ^ 0xf);
            let args = [
                q,
                rng.word(),
                rng.word(),
                rng.word(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
            ];
            t.check_fs(
                &format!("bar caps #{case}"),
                0x008e_bc00,
                &args,
                0,
                0,
                |m, _, s| Out::U32(mnb::bar_caps(m, args, s)),
            );
        }
    });
}

#[test]
fn r3b6_mes_font_inter_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0c04);
        for case in 0..8u32 {
            t.reset();
            let r = rng.next();
            t.stub_copy_fill(mnc::CRT_MEMSET);
            let scale_x = [1.0f32, 0.75, 2.5, 0.0][(case % 4) as usize];
            t.stub_store_into(mnc::GLYPH_LOOKUP, 1, 0x1c, scale_x.to_bits(), 3, r);
            t.stub(mnc::PASS_EDGE, false, 3, 0, r ^ 1);
            // the real passes move the cursor: odd cases
            if case & 1 == 0 {
                t.stub(mnc::PASS_FONT, false, 3, 0, r ^ 2);
            } else {
                t.stub_store_global(mnc::PASS_FONT, mnc::PKT_CURSOR, r ^ 3, r ^ 2);
            }
            let text = t.alloc(&[[0u8, b'A'][usize::from(case != 0)], b'B', b'C', 0]);
            t.stub_seq(mnc::NEXT_CHAR, 1, 0, &[text + 1, text + 2, text + 3]);
            t.g32(
                mnc::FONT_SPACING,
                [0.0f32, 1.5, -2.0, 3.25][(case % 4) as usize].to_bits(),
            );
            t.g32(
                mnc::WIDTH_FACTOR,
                [12.0f32, 7.0, 0.5, 9.75][(case / 2 % 4) as usize].to_bits(),
            );
            let scale = [0u32, 0x10, 0xffff_ffff, 7][(case % 4) as usize];
            let args = [
                rng.next(),
                text,
                rng.word(),
                rng.word(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.word(),
                scale,
            ];
            t.check_fs(
                &format!("mes font inter #{case}"),
                0x008e_d190,
                &args,
                0,
                0,
                |m, _, s| Out::U32(mnb::mes_font_inter(m, args, s)),
            );
        }
    });
}

#[test]
fn r3b7_movie_font_glyph_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0d01);
        for case in 0..16u32 {
            t.reset();
            let r = rng.next();
            let buf = t.zeros(0x40);
            t.g32(mnc::PKT_CURSOR, buf);
            t.g32(0x0193_ed20, (case >> 3) & 1); // __sse2_available
            for (sym, n) in [
                (mnc::GS_TEST, 8),
                (mnc::GS_ALPHA, 5),
                (mnc::GS_WORDS4, 4),
                (mnc::DRAW_KICK, 0),
            ] {
                t.stub(sym, false, n, 0, r ^ sym);
            }
            t.stub(mnc::GS_TEX0, false, 12, 0, r ^ 1);
            t.stub(mnc::GS_XY2, false, 2, 0, r ^ 2);
            t.stub(mnc::GS_XYZ_OFFSET, false, 3, 0, r ^ 3);
            t.stub(mnc::GS_XYZ2_OFFSET, false, 2, 0, r ^ 4);
            ctrl_state(t, case, r ^ 4);
            let (pos, glyph) = glyph_pair(t, &mut rng, case);
            t.write(
                pos + 0x1b,
                &[[0u8, 7, 8, 0x1f, 0xff, 0x9a][(case % 6) as usize]],
            );
            t.check_fs(
                &format!("movie font glyph #{case}"),
                0x008f_dc00,
                &[pos, glyph],
                0,
                0,
                |m, _, s| Out::U32(mnb::movie_font_glyph(m, pos, glyph, s)),
            );
        }
    });
}

#[test]
fn r3b7_shape_by_kind_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0d02);
        for kind in 0..0x22u32 {
            t.reset();
            let r = rng.next();
            for (k, sym) in mnc::KIND_SHAPES.into_iter().enumerate() {
                t.stub(sym, false, 6, 0, r ^ k as u32);
            }
            for sym in mnc::KIND_COLOURS {
                t.g32(sym, rng.next());
                t.g32(sym - 4, rng.next());
            }
            let args = [
                rng.next(),
                rng.word(),
                rng.word(),
                if kind == 0x21 { 0xffff_ffff } else { kind },
            ];
            t.check(
                &format!("shape by kind {kind:#x}"),
                0x008f_1b50,
                Conv::Cdecl,
                &args,
                |m| Out::U32(mnb::shape_by_kind(m, args)),
            );
        }
    });
}

#[test]
fn r3b8_cross_box_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0e01);
        for case in 0..13u32 {
            t.reset();
            let r = rng.next();
            t.stub(mnc::DRAW_UI_ELEMENT, false, 5, 0, r);
            t.g32(0x0193_ed20, (case >> 1) & 1); // __sse2_available
            let fl = [0.0f32, 12.5, -3.25, 1e9, 640.0, 0.1];
            let full = [1u32, 0x40, 3, 0xffff_ffff, 0][(case % 5) as usize];
            let mut args = [
                fl[(case % 6) as usize].to_bits(),
                fl[((case + 1) % 6) as usize].to_bits(),
                fl[((case + 2) % 6) as usize].to_bits(),
                fl[((case + 3) % 6) as usize].to_bits(),
                rng.next(),
                rng.next(),
                full,
            ];
            if case == 12 {
                // (1 * 49) / 49 is 1, (1 / 49) * 49 just under: the order shows
                args = [0, 0, 49.0f32.to_bits(), 0, 0, 1, 49];
            }
            t.check_fs(
                &format!("cross box #{case}"),
                0x008f_4c40,
                &args,
                0xb0,
                4,
                |m, ebp, _| Out::U32(mnb::cross_box_fill_width(m, ebp, args)),
            );
        }
    });
}

#[test]
fn r3b8_important_icon_name_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0e02);
        for case in 0..8u32 {
            t.reset();
            let r = rng.next();
            let record = t.zeros(0x20);
            t.write(record, &(0x120u16 + case as u16).to_le_bytes());
            t.write(record + 0x12, &[case as u8 * 7]);
            t.stub_store(
                mnc::IMPORTANT_PTR,
                None,
                1,
                0x1000 * case,
                false,
                2,
                0,
                record,
            );
            t.stub_float(
                mnc::REMAP_Y2,
                [0.5f32, -12.25, 3e6, 0.0][(case % 4) as usize],
            );
            t.stub_float(
                mnc::REMAP_X2,
                [1.75f32, 7.0, -0.125, 1e-3][(case / 2 % 4) as usize],
            );
            t.stub(mnc::CROSS_MENU_ICON, false, 9, 0, r);
            t.stub(mnc::BTL_EASY_FONT, false, 6, 0, r ^ 1);
            let fl = [0.0f32, 100.5, -33.25, 1e7];
            let args = [
                rng.next(),
                fl[(case % 4) as usize].to_bits(),
                fl[((case + 1) % 4) as usize].to_bits(),
                rng.next(),
            ];
            t.check_fs(
                &format!("important icon #{case}"),
                0x008f_50f0,
                &args,
                8,
                0,
                |m, ebp, s| Out::U32(mnb::important_icon_name(m, ebp, args, s)),
            );
        }
    });
}

#[test]
fn r3b8_hdd_ins_check_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0e03);
        for case in 0..8u32 {
            t.reset();
            let r = rng.next();
            let n = [0u32, 1, 3, 2][(case % 4) as usize];
            let mut list = vec![0u8; 0x34 * 8];
            for (k, e) in list.chunks_mut(0x34).enumerate() {
                e[..0x24].fill(0x10 + k as u8);
                e[0x24..].fill(0xee);
                let name = format!("file{k}.bin");
                e[0x24..0x24 + name.len()].copy_from_slice(name.as_bytes());
                e[0x24 + name.len()] = 0;
            }
            let list = t.alloc(&list);
            let copy = t.zeros(0x34 * 8);
            t.stub(hdc::MOUNT_LOG, false, 1, 0, r);
            t.stub_store(hdc::READ_LIST, None, 1, n, false, 3, 0, r ^ 1);
            t.stub(hdc::USER_MALLOC, false, 1, 0, copy);
            t.stub_copy(hdc::MEMCPY);
            t.stub(hdc::DBG_PRINTF, false, 2, 0, r ^ 2);
            imp(t, hdc::SPRINTF_IAT, 3, false, r ^ 3);
            t.stub_seq(
                hdc::SCE_OPEN,
                2,
                0,
                &[3, [0xffff_ffff, 4][(case / 4) as usize], 5],
            );
            // the count word, and the entries' first words (both sides alike)
            t.stub_store(
                hdc::SCE_READ,
                None,
                1,
                [2u32, 0, 1, 0xffff_ffff][(case % 4) as usize],
                false,
                3,
                0,
                r ^ 4,
            );
            t.stub(hdc::SCE_CLOSE, false, 1, 0, r ^ 5);
            t.stub(hdc::USER_FREE, false, 1, 0, r ^ 6);
            t.check_fs(
                &format!("ins check #{case}"),
                0x0077_8d70,
                &[list],
                0x120,
                4,
                |m, ebp, _| Out::U32(hdb::ins_check(m, ebp, list)),
            );
        }
    });
}

#[test]
fn r3b9_wakka_frame_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0f01);
        for case in 0..4u32 {
            t.reset();
            let r = rng.next();
            // constant remaps, or ones that hand back their argument (the
            // results then tell the constants apart)
            if case < 2 {
                t.stub_float(mnc::REMAP_Y2, [0.5f32, -12.25][case as usize]);
                t.stub_float(mnc::REMAP_X2, [1.75f32, 7.0][case as usize]);
            } else {
                for sym in [mnc::REMAP_Y2, mnc::REMAP_X2] {
                    let at = t.cave_code(&[0xd9, 0x44, 0x24, 0x04, 0xc3]); // fld dword [esp+4]; ret
                    t.jump(sym, at);
                }
            }
            t.stub_seq(mnc::SHAPE_XYWHUV, 9, 0, &[r, r ^ 1, r ^ 2, r ^ 3]);
            t.check(
                &format!("wakka frame #{case}"),
                0x008a_1e50,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mnb::wakka_frame(m)),
            );
        }
    });
}

#[test]
fn r3b9_movie_change_macro_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0f02);
        for case in 0..15u32 {
            t.reset();
            let r = rng.next();
            t.stub_copy_fill(mnc::CRT_MEMSET);
            // the text: codes 7 + step, nested macros, glyphs; empty, none
            let text = t.alloc(&[0x41, 7, 0xe3, 0x42, 7, 0xf0, 0x43, 0x44, 0]);
            let empty = t.alloc(&[0]);
            let found = match case % 5 {
                _ if case >= 10 => text,
                0 => 0,
                1 => empty,
                _ => text,
            };
            t.stub(mnc::CHANGE_MACRO_ADDRESS, false, 2, 0, found);
            t.stub_seq(
                mnc::NEXT_CHAR,
                1,
                0,
                &[text + 1, text + 3, text + 4, text + 6, text + 7, text + 8],
            );
            // the class: the argument's upper bytes are the caller's, so not logged
            t.stub_seq(
                mnc::CHAR_CLASS,
                0,
                0,
                &[[1u32, 3][(case & 1) as usize], 1, 2, 3, 1],
            );
            t.stub(mnc::CHANGE_MACRO2, false, 3, 0, r);
            t.stub(mnc::FONT_INFO, false, 2, 0, r ^ 1);
            t.stub(mnc::MOVIE_FONT_1612, false, 2, 0, r ^ 2);
            let win = t.zeros(0xc0);
            let (count, limit, most) = [
                (0u32, 9u32, 9u32),
                (0, 2, 9),
                (1, 1, 2),
                (0, 1, 3),
                (3, 2, 1),
            ][(case % 5) as usize];
            // cases 10..14: the limit's growth: stopped by +0xb4, by +0xac,
            // free up to 3, held at 1, free
            let (count, limit, most) = if case >= 10 {
                (0, 1, if case == 13 { 1 } else { 3 })
            } else {
                (count, limit, most)
            };
            t.put(win + 0x9c, &[count, limit]);
            t.put(
                win + 0xa8,
                &[most, u32::from(case == 11), 0, u32::from(case == 10)],
            );
            let pos = t.words(&[0, 2.5f32.to_bits(), 0, 0, 0, 0, 0]);
            let args = [win, rng.next(), pos];
            t.check_fs(
                &format!("change macro #{case}"),
                0x008b_5ea0,
                &args,
                0,
                0,
                |m, _, s| Out::U32(mnb::movie_change_macro(m, args[0], args[1], args[2], s)),
            );
        }
    });
}

#[test]
fn r3b10_font_1612_text_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_1001);
        for case in 0..6u32 {
            t.reset();
            let r = rng.next();
            t.stub_copy_fill(mnc::CRT_MEMSET);
            // the characters and their classes: a glyph, a new line, the
            // control codes (macro, colour, icon, other), a macro
            let text = t.alloc(&[
                0x41,
                0x0d,
                0x10,
                0x52,
                0x0a,
                [0x35, 0x40, 0x7f, 0x00, 0x91, 0x0f][case as usize],
                0x0b,
                0x07,
                0x09,
                0x12,
                0x10,
                0x20,
                0x42,
                0,
            ]);
            let steps = [1u32, 2, 4, 6, 8, 9, 10, 12, 13];
            let next: Vec<u32> = steps.iter().map(|k| text + k).collect();
            t.stub_seq(mnc::NEXT_CHAR, 1, 0, &next);
            t.stub_seq(
                mnc::CHAR_CLASS,
                0,
                0,
                &[4, 1, 2, 2, 2, 2, 3, 2, [4, 0][(case & 1) as usize]],
            );
            let sx = [1.0f32, 0.75, 12.5][(case % 3) as usize];
            t.stub_store_into(mnc::FONT_INFO, 1, 0x1c, sx.to_bits(), 2, r);
            t.stub(mnc::FONT1612_SHADOW, false, 2, 0, r ^ 1);
            t.stub(mnc::FONT1612_FONT, false, 2, 0, r ^ 2);
            t.stub(mnc::SET_BUTTON_DATA, false, 2, 0, r ^ 3);
            t.stub(mnc::RESET_BUTTON_DATA, false, 0, 0, r ^ 4);
            t.stub(mnc::BUTTON_ICON, false, 4, 0, r ^ 5);
            t.stub(mnc::CHANGE_MACRO, false, 3, 0, r ^ 6);
            t.g32(0x0193_ed20, case & 1); // __sse2_available
            let fl = [0u32, 30, 0xffff_ffe0, 0x7fff_ffff];
            let args = [
                rng.next(),
                rng.next(),
                text,
                fl[(case % 4) as usize],
                fl[((case + 1) % 4) as usize],
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
            ];
            t.check_fs(
                &format!("font 1612 text #{case}"),
                0x008e_a680,
                &args,
                0,
                0,
                |m, _, s| Out::U32(mnb::font_1612_text(m, args, s)),
            );
        }
    });
}

#[test]
fn r3b11_job_worker_loop_and_file_read_main_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_1101);
        for passes in 1..5u32 {
            t.reset();
            let r = rng.next();
            let sched = t.zeros(0x40);
            t.g32(pcs::JOB_SCHEDULER, sched);
            let jobs: Vec<u32> = (0..4).map(|k| r ^ (k << 4)).collect();
            t.stub(pcs::THREAD_CALL_RUN, true, 0, 0, r ^ 1);
            t.stub(pcs::QUEUE_PUSH, true, 2, 2, r ^ 2);
            // the pop stub ends the original at pop number passes + 1,
            // before logging it: dec [count]; jg pop; add esp, 8; pop edi;
            // pop esi; pop ebx; ret (out of the loop to the test's caller)
            let pop = t.stub_seq_code(pcs::QUEUE_POP, 1, 1, &jobs);
            let count = t.words(&[passes + 1]);
            let mut c = vec![0xff, 0x0d];
            c.extend(count.to_le_bytes());
            c.extend([0x0f, 0x8f]);
            let here = t.cave_next + c.len() as u32 + 4;
            c.extend(pop.wrapping_sub(here).to_le_bytes()); // jg pop
            c.extend([0x83, 0xc4, 0x08, 0x5f, 0x5e, 0x5b, 0xc3]);
            let at = t.cave_code(&c);
            t.jump(pcs::QUEUE_POP, at);
            let this = t.words(&[0, 0, 0, 0, rng.below(3)]);
            // `before` pushes the original's queue log alike: the bridge
            // runs the same passes
            let poison = t.rebase(POISON_AT);
            let ra =
                t.a.call(t.rebase(0x0070_3a20), Conv::Thiscall(this), &[])
                    .unwrap();
            let _ = ra;
            {
                let mut g = G(&mut t.b, poison);
                let sched: u32 = Mem::read(&g, g.rebase(pcs::JOB_SCHEDULER));
                for _ in 0..passes {
                    pcb::job_worker_pass(&mut g, sched, this);
                }
            }
            // the count cell: the original's stub counted it down
            t.b.write(count, &0u32.to_le_bytes()).unwrap();
            // b's pops ran the sequence stub directly: its counter cell
            // is the only other difference, written alike above
            t.diff(&format!("job worker loop x{passes}"));
            // FileReadMain: the step through the task record
            let step = t.stub_code(0x0e0f_1101, false, 2, 0, r ^ 3);
            let task = t.words(&[step, rng.next()]);
            t.g32(pcs::READ_TASK, task);
            t.check(
                &format!("file read main #{passes}"),
                0x0076_cfd0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(pcb::file_read_main(m)),
            );
        }
    });
}
impl Twins {
    /// [`Twins::stub_seq`]'s code without the jump over `sym`; its address.
    fn stub_seq_code(&mut self, sym: u32, n: u32, pops: u32, seq: &[u32]) -> u32 {
        self.stub_seq(sym, n, pops, seq);
        let at = self.rebase(sym);
        let rel: u32 = Mem::read(&self.a, at + 1);
        at.wrapping_add(5).wrapping_add(rel)
    }
}

#[test]
fn r3b12_cd_file_sizes_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_1201);
        for case in 0..24u32 {
            t.reset();
            let r = rng.next();
            let numbers = [0u32, 3, 21, 36, 5, 0xffff_fff1, 18 * 7 + 3, 0xffff_ffdc];
            let number = numbers[(case % 8) as usize];
            t.g32(hdc::FILE_BASE, [0u32, 0x10, 2][(case % 3) as usize]);
            let module = [0x10u32, 4, 0x29, 0x1b, 0xe][(case / 3 % 5) as usize];
            t.g32(hdc::MODULE, module);
            t.stub(
                hdc::EX_FILE_SIZE,
                false,
                2,
                0,
                if case % 8 == 6 { 0x4321 } else { 0 },
            );
            t.stub(hdc::IS_CALM_MGRP, false, 1, 0, case & 1);
            t.stub(hdc::CALM_MGRP_SIZE, false, 1, 0, r);
            t.stub(hdc::IS_CALM_SCENE, false, 1, 0, (case >> 1) & 1);
            t.stub(hdc::CALM_SCENE_SIZE, false, 1, 0, r ^ 1);
            // the sector table and the 24-bit sizes around every index used
            let words: Vec<u32> = (0..0x100)
                .map(|k| {
                    let flag = [0u32, 0x40_0000, 0x80_0000, 0xc0_0000][((k + case) % 4) as usize];
                    flag | (rng.next() & 0xff3f_ffff)
                })
                .collect();
            let table = t.words(&words);
            // a negative number reaches below the table: point it inside
            t.g32(hdc::SECTOR_TABLE, table + 0x200);
            let sizes: Vec<u8> = (0..0x300).map(|_| rng.next() as u8).collect();
            let sizes_at = t.alloc(&sizes);
            t.g32(hdc::SIZE_TABLE, sizes_at + 0x180);
            t.check(
                &format!("size cd #{case}"),
                0x0076_c320,
                Conv::Cdecl,
                &[number],
                |m| Out::U32(hdb::file_size_cd(m, number)),
            );
            // the PC size
            let cd = t.words(&[0; 16]);
            t.put(cd + 0x20, &[rng.below(4)]);
            t.put(cd + 0x30, &[u32::from(case % 5 != 4)]);
            t.stub(hdc::GET_CD, false, 0, 0, cd);
            t.write(t.rebase(hdc::HDD_MODE), &[u8::from(case % 6 == 5)]);
            t.stub(hdc::FILE_SIZE_CD, false, 1, 0, r ^ 2);
            t.stub(
                hdc::PC_FILE_NAME,
                false,
                1,
                0,
                if case % 9 == 8 { 0 } else { r ^ 3 },
            );
            t.stub(
                hdc::FILE_SIZE_CD_EX,
                false,
                1,
                0,
                [0xffff_ffffu32, 0x777][(case & 1) as usize],
            );
            t.stub(
                hdc::SCE_OPEN,
                false,
                2,
                0,
                [0u32, 5, 0xffff_ffff][(case % 3) as usize],
            );
            t.stub(
                hdc::SCE_FILE_SIZE,
                false,
                1,
                0,
                [0u32, 1, 0x10, 0xffff_fff8, 0x7fff_fff5, 0x8000_0000][(case % 6) as usize],
            );
            t.stub(hdc::SCE_CLOSE, false, 1, 0, r ^ 4);
            t.stub(hdc::DBG_PRINTF, false, 2, 0, r ^ 5);
            t.check(
                &format!("size pc #{case}"),
                0x00a4_28a0,
                Conv::Cdecl,
                &[number],
                |m| Out::U32(hdb::file_size_pc(m, number)),
            );
        }
    });
}

#[test]
fn r3b13_movie_message_end_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_1301);
        t.a.map_zeroed(mnc::GS_PMODE, 0x1000).unwrap();
        t.b.map_zeroed(mnc::GS_PMODE, 0x1000).unwrap();
        for case in 0..8u32 {
            t.reset();
            let r = rng.next();
            t.stub(mnc::CLEAR_MESSAGE_SLOT, false, 1, 0, r);
            t.stub(mnc::TRANS_IMAGE_HL, false, 7, 0, r ^ 1);
            t.stub(mnc::MESSAGE_WINDOW_MODE, false, 1, 0, r ^ 2);
            t.g32(mnc::MOVIE_IMAGE, rng.next());
            t.w32(mnc::GS_PMODE, rng.next());
            t.w32(mnc::GS_PMODE + 4, rng.next());
            for (k, sym) in mnc::CAPTURE_FLAGS.into_iter().enumerate() {
                let v = [0u8, 1, 2][((case >> k) & 1) as usize + usize::from(k == 2 && case == 7)];
                t.write(t.rebase(sym), &[v]);
            }
            // the frame's two never-written words are 0 in both (brief 2026-10-24)
            t.check_fs(
                &format!("movie message end #{case}"),
                0x008a_b860,
                &[],
                8,
                0,
                |m, ebp, _| Out::U32(mnb::movie_message_end(m, ebp)),
            );
        }
    });
}

#[test]
fn r3b13_movie_disc_wait_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_1302);
        for case in 0..12u32 {
            t.reset();
            let r = rng.next();
            let ctx = t.zeros(0x5_0760);
            let streaming = case < 4;
            t.write(ctx + 0x5_075d, &[u8::from(streaming)]);
            t.put(ctx + 0x5_0544, &[rng.next()]);
            for (k, sym) in [hdc::MOVIE_CSC, hdc::PAD_PRINT_FLAG, hdc::STATE_CLEAR]
                .into_iter()
                .enumerate()
            {
                t.stub(sym, false, 0, 0, r ^ k as u32);
            }
            t.stub(hdc::AUDIO_STOP, false, 1, 0, r ^ 4);
            t.stub(hdc::DISC_WAIT, false, 1, 0, r ^ 5);
            t.stub(hdc::READ_SECTORS, false, 5, 0, r ^ 6);
            t.stub(hdc::DBG_PRINTF, false, 1, 0, r ^ 7);
            for sym in hdc::PENDING_SECTOR {
                t.g32(sym, rng.next());
            }
            t.write(t.rebase(hdc::DISK_RESULT), &[0x55]);
            let play = u32::from(case & 1 != 0);
            if streaming {
                // the ioctl writes 3, 2, 1, then 0 through its third word
                let left = t.words(&[[0u32, 2, 3, 1][case as usize]]);
                let log = t.stub_code(hdc::SCE_IOCTL, false, 3, 0, r ^ 8);
                let mut c = vec![0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x0d]; // mov eax, [esp+0xc]; mov ecx, [left]
                c.extend(left.to_le_bytes());
                c.extend([0x89, 0x08, 0x85, 0xc9, 0x74, 0x06, 0xff, 0x0d]); // mov [eax], ecx; test; jz +6; dec [left]
                c.extend(left.to_le_bytes());
                c.push(0xe9);
                let here = t.cave_next + c.len() as u32 + 4;
                c.extend(log.wrapping_sub(here).to_le_bytes());
                let at = t.cave_code(&c);
                t.jump(hdc::SCE_IOCTL, at);
                t.stub(hdc::MOVIE_WAIT, false, 0, 0, r ^ 9);
                let args = [play, ctx];
                t.check_fs(
                    &format!("disc wait stream #{case}"),
                    0x0077_7440,
                    &args,
                    0,
                    0,
                    |m, _, s| Out::U32(hdb::movie_disc_wait(m, play, ctx, s)),
                );
                continue;
            }
            let changed = [0u32, 0xffff_ffff, 5][(case % 3) as usize];
            t.stub(hdc::DISK_CHANGED, false, 0, 0, changed);
            if changed == 0 {
                t.stub(hdc::MOVIE_WAIT, false, 0, 0, r ^ 9);
                t.check_fs(
                    &format!("disc wait ok #{case}"),
                    0x0077_7440,
                    &[play, ctx],
                    0,
                    0,
                    |m, _, s| Out::U32(hdb::movie_disc_wait(m, play, ctx, s)),
                );
                continue;
            }
            // the never-ending wait: the original stopped at its k-th
            // movie_wait (out of the function through its frame), the
            // bridge's passes run up to the same point
            let k = [1u32, 2, 151, 152, 300][(case % 5) as usize];
            let log = t.stub_code(hdc::MOVIE_WAIT, false, 0, 0, r ^ 9);
            let count = t.words(&[k]);
            let mut c = vec![0xff, 0x0d];
            c.extend(count.to_le_bytes());
            c.extend([0x0f, 0x85]);
            let here = t.cave_next + c.len() as u32 + 4;
            c.extend(log.wrapping_sub(here).to_le_bytes()); // jnz log
            // lea esp, [ebp - 0x18]; pop edi; pop esi; pop ebx; mov esp, ebp; pop ebp; ret
            c.extend([0x8d, 0x65, 0xe8, 0x5f, 0x5e, 0x5b, 0x8b, 0xe5, 0x5d, 0xc3]);
            let at = t.cave_code(&c);
            t.jump(hdc::MOVIE_WAIT, at);
            let at = t.rebase(0x0077_7440);
            t.a.call(at, Conv::Cdecl, &[play, ctx]).unwrap();
            {
                let poison = t.rebase(POISON_AT);
                let mut g = G(&mut t.b, poison);
                let changed2 = hdb::disc_wait_first(&mut g);
                let mut st = hdb::DiscWaitState::default();
                for _ in 1..k {
                    hdb::disc_wait_pre(&mut g, ctx, changed2, &mut st);
                    hdb::disc_wait_post(&mut g, play, ctx, &mut st);
                }
                hdb::disc_wait_pre(&mut g, ctx, changed2, &mut st);
            }
            t.b.write(count, &0u32.to_le_bytes()).unwrap();
            t.diff(&format!("disc wait forever #{case} k {k}"));
        }
    });
}
