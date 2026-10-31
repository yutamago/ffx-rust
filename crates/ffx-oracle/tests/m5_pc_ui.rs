//! M5 stage 1, list `pc-ui` (`ffx_pcui::m5u_*`) against the original: on the image alone with synthetic objects, and on the
//! snapshots (`FFX_SNAPSHOT`) over the game's memory. Results and every
//! byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! Most of these functions never ran in a coverage run, so every branch
//! is reached with synthetic data (the snapshots add the game's own
//! tables where a function reads them). Callees that are still original
//! (`operator new`/`delete`, the Iggy and texture calls, the `KERNEL32`
//! imports, virtual methods of fake objects) are replaced in both oracles alike by
//! recording stubs: each call appends its tag, `ecx` and its arguments to
//! a log in memory, so the diff checks that both made the same calls in
//! the same order. Imports are stubbed through their import table slots;
//! the C runtime (`sscanf`, `atoi`, `memmove`) is the game's own
//! `MSVCR110` in the oracle. Allocations come from a bump allocator stub
//! ([`Twins::stub_alloc`]), so both sides get the same addresses.

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
use ffx_pcui::m5u_a::bridge as a;
use ffx_pcui::m5u_b::bridge as b;
use ffx_pcui::m5u_c::bridge as c;
use ffx_pcui::m5u_d::bridge as d;
use ffx_pcui::m5u_e::bridge as e;
use ffx_pcui::m5u_f::bridge as ff;
use ffx_pcui::m5u_g::bridge as gg;
use ffx_pcui::m5u_h::bridge as hh;
use ffx_pcui::m5u_i::bridge as ii;
use ffx_pcui::m5u_j::bridge as jj;
use ffx_pcui::m5u_k::bridge as kk;
use ffx_pcui::m5u_l::bridge as ll;

/// `graphicFontDraw`'s body: no test here calls it, so the recording
/// stubs are written over it.
const CAVE: u32 = 0x0064_0660;
const CAVE_LEN: u32 = 1760;
/// After the stubs: `mov eax, POISON; ret`, which [`G`] calls before each
/// original so that a callee leaving `eax` alone shows the poison.
const POISON_AT: u32 = CAVE + CAVE_LEN;
const POISON: u32 = 0xdead_beef;
/// After the poison (`+8`): `fld qword [esp+4]; call [esp+0xc]; ret`,
/// for [`Host::call_intrinsic`] (a C runtime intrinsic with its argument
/// in `ST0`; the cave function is 1864 bytes long).
const TRAMPOLINE: [u8; 9] = [0xdd, 0x44, 0x24, 0x04, 0xff, 0x54, 0x24, 0x0c, 0xc3];

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
    fn call_intrinsic(&mut self, addr: u32, x: f64) -> f32 {
        let b = x.to_bits();
        let r = self
            .0
            .call(self.1 + 8, Conv::Cdecl, &[b as u32, (b >> 32) as u32, addr])
            .unwrap_or_else(|e| panic!("intrinsic {addr:#x}: {e}"));
        r.st0_f32().expect("the intrinsic's ST0")
    }
    fn call_intrinsic_f64(&mut self, addr: u32, x: f64) -> f64 {
        let b = x.to_bits();
        let r = self
            .0
            .call(self.1 + 8, Conv::Cdecl, &[b as u32, (b >> 32) as u32, addr])
            .unwrap_or_else(|e| panic!("intrinsic {addr:#x}: {e}"));
        r.st0_f64().expect("the intrinsic's ST0")
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
    /// `al` (the callers read no more).
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
        t.write(at + 8, &TRAMPOLINE);
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

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
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
}

impl Twins {
    /// A bump allocator over `sym` (cdecl `(size)`): 16-byte aligned
    /// blocks from a fresh zeroed pool, the same addresses on both sides;
    /// returns 0 once `fail_after` blocks were handed out.
    fn stub_alloc(&mut self, sym: u32, fail_after: u32) {
        self.stub_alloc_with(sym, fail_after, false);
    }

    /// [`Twins::stub_alloc`] for `PhyreMalloc(size, align)`: the last
    /// alignment is kept in memory too, so the diff sees it.
    fn stub_alloc2(&mut self, sym: u32, fail_after: u32) {
        self.stub_alloc_with(sym, fail_after, true);
    }

    fn stub_alloc_with(&mut self, sym: u32, fail_after: u32, align: bool) {
        // filled, so the zeros the callers store show
        let pool = self.alloc(&vec![0xcd; 0x4_0000]);
        let cur = self.words(&[pool, fail_after, 0]);
        let mut c = Vec::new();
        if align {
            c.extend([0x8b, 0x4c, 0x24, 0x08]); // mov ecx, [esp+8]
            c.extend([0x89, 0x0d]);
            c.extend((cur + 8).to_le_bytes()); // mov [cur+8], ecx
        }
        c.extend([0x83, 0x3d]);
        c.extend((cur + 4).to_le_bytes());
        c.push(0x00); // cmp dword [left], 0
        c.extend([0x75, 0x03]); // jne +3
        c.extend([0x31, 0xc0, 0xc3]); // xor eax, eax; ret
        c.extend([0xff, 0x0d]);
        c.extend((cur + 4).to_le_bytes()); // dec dword [left]
        c.push(0xa1);
        c.extend(cur.to_le_bytes()); // mov eax, [cur]
        c.extend([0x8b, 0x4c, 0x24, 0x04]); // mov ecx, [esp+4]
        c.extend([0x83, 0xc1, 0x0f, 0x83, 0xe1, 0xf0]); // round up to 16
        c.extend([0x01, 0xc1]); // add ecx, eax
        c.extend([0x89, 0x0d]);
        c.extend(cur.to_le_bytes()); // mov [cur], ecx
        c.push(0xc3);
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }

    /// Calls the original at `sym` and the bridge with no frame, comparing
    /// `eax` when `ret`.
    fn run(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        ret: bool,
        bridge: impl FnOnce(&mut G) -> u32,
    ) {
        self.check(what, sym, conv, args, |m| {
            let v = bridge(m);
            if ret { Out::U32(v) } else { Out::None }
        });
    }
}

/// Recording stubs for the frees.
fn delete_stubs(t: &mut Twins) {
    t.stubs(&[
        (a::OPERATOR_DELETE, false, 1, 0, 0x0d0d),
        (a::OPERATOR_DELETE_ARRAY, false, 1, 0, 0x0d0e),
        (a::PHYRE_FREE, false, 1, 0, 0x0f0f),
    ]);
}

// ---- the controller queues ----

/// A deque at `at` (20 bytes) with `map_size` blocks of 16 bytes (each
/// missing with chance 1 in `hole`, 0: none), `off`, `size`, its proxy
/// pointing back.
#[allow(clippy::too_many_arguments)]
fn made_up_deque(
    t: &mut Twins,
    rng: &mut Rng,
    at: u32,
    map_size: u32,
    off: u32,
    size: u32,
    hole: u32,
) {
    let mut map = Vec::new();
    for _ in 0..map_size {
        if hole != 0 && rng.chance(hole) {
            map.push(0);
        } else {
            let w: Vec<u32> = (0..4).map(|_| rng.next()).collect();
            map.push(t.words(&w));
        }
    }
    let map = if map_size == 0 { 0 } else { t.words(&map) };
    let proxy = t.words(&[at]);
    let d = [proxy, map, map_size, off, size];
    for (i, w) in (0u32..).zip(d) {
        t.w32(at + 4 * i, w);
    }
}

/// A manager: `+0x10`, `+0x24` deques, `+0x40` a mutex handle.
fn made_up_manager(t: &mut Twins, rng: &mut Rng, n_words: u32, n_sticks: u32) -> u32 {
    let m = t.zeros(0x48);
    let ms = 1 << rng.below(4);
    let off = rng.below(ms * 4);
    made_up_deque(t, rng, m + 0x10, ms, off, n_words, 0);
    let ms = 1 << rng.below(4);
    let off = rng.below(ms);
    made_up_deque(t, rng, m + 0x24, ms, off, n_sticks, 0);
    t.w32(m + 0x40, 0x4d00 + rng.below(16));
    t.w32(m + 0x3c, 1);
    m
}

fn mutex_stubs(t: &mut Twins) {
    t.stub_import(a::WAIT_IAT, false, 2, 2, 0);
    t.stub_import(a::RELEASE_MUTEX_IAT, false, 1, 1, 0x4e11);
}

/// Every path: off, empty, the `+0x20`/`+0x34` counts at 1 (offset
/// reset) and above.
#[test]
fn get_controller_info_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0c0f_fee1);
        for i in 0..40 {
            t.reset();
            mutex_stubs(t);
            let on = u32::from(i % 8 != 0);
            t.g32(a::ASYNC_CONTROLLER_ON, on);
            let words = [0, 1, 2, 3, 7][rng.below(5) as usize];
            let sticks = [0, 1, 2, 5][rng.below(4) as usize];
            let m = made_up_manager(t, &mut rng, words, sticks);
            let x = rng.word();
            t.w32(m + 0x30, x);
            let x = rng.below(64);
            t.w32(m + 0x1c, x);
            let out = t.words(&[0x7777_7777; 5]);
            let what = format!("get_controller_info #{i} on {on} words {words} sticks {sticks}");
            t.check_framed(
                &what,
                0x0068_ccd0,
                Conv::Thiscall(m),
                &[out, out + 4],
                0x10,
                |g, ebp| Out::U32(a::get_controller_info(g, m, out, out + 4, ebp - 0x10)),
            );
        }
    });
}

/// With recording pushes (the pointers handed on) and with the original
/// pushes (the queues' contents).
#[test]
fn push_controller_info_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0bad_cafe);
        // the security cookie at ebp - 4
        t.skip = 4;
        for i in 0..40 {
            t.reset();
            mutex_stubs(t);
            let real = i % 2 == 1;
            if real {
                t.stub_alloc(a::OPERATOR_NEW, 1000);
                delete_stubs(t);
            } else {
                t.stub(a::PUSH_U32, true, 1, 1, 0x11);
                t.stub(a::PUSH_V4, true, 1, 1, 0x12);
            }
            let on = u32::from(i % 6 != 0);
            t.g32(a::ASYNC_CONTROLLER_ON, on);
            let (nw, ns) = (rng.below(12), rng.below(5));
            let m = made_up_manager(t, &mut rng, nw, ns);
            let args: Vec<u32> = (0..5).map(|_| rng.next()).collect();
            // the port reads the word from the caller's slot
            let ebp = t.frame_ebp(5);
            for (k, w) in (0u32..).zip(&args) {
                Mem::write(&mut t.b, ebp + 8 + 4 * k, *w);
            }
            let what = format!("push_controller_info #{i} on {on} real {real}");
            t.check_framed(
                &what,
                0x0068_cc10,
                Conv::Thiscall(m),
                &args,
                0x14,
                |g, ebp| {
                    let s = [args[1], args[2], args[3], args[4]];
                    Out::U32(a::push_controller_info(g, m, ebp + 8, s, ebp - 0x14))
                },
            );
        }
        t.skip = 0;
    });
}

#[test]
fn clear_controller_info_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x00c1_ea12);
        for on in [0, 1, 2] {
            t.reset();
            mutex_stubs(t);
            t.stub(a::CLEAR_U32, true, 0, 0, 1);
            t.stub(a::CLEAR_V4, true, 0, 0, 2);
            t.g32(a::ASYNC_CONTROLLER_ON, on);
            let m = made_up_manager(t, &mut rng, 2, 2);
            let what = format!("clear_controller_info on {on}");
            t.run(&what, 0x0068_cc90, Conv::Thiscall(m), &[], false, |g| {
                a::clear_controller_info(g, m);
                0
            });
        }
    });
}

#[test]
fn deque_tidy_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7d1d_0001);
        for sym in [0x0068_d2d0, 0x0068_d330] {
            for i in 0..20 {
                t.reset();
                delete_stubs(t);
                let d = t.zeros(0x14);
                let ms = [0, 1, 2, 8, 16][rng.below(5) as usize];
                let size = if rng.chance(3) { 0 } else { rng.below(40) + 1 };
                let off = rng.below(64);
                made_up_deque(t, &mut rng, d, ms, off, size, 3);
                let what = format!("deque tidy {sym:#x} #{i}");
                t.run(&what, sym, Conv::Thiscall(d), &[], false, |g| {
                    a::deque_tidy(g, d);
                    0
                });
            }
        }
    });
}

/// `_Growmap`: map sizes 0..16, counts 0..100. A count of 0 on a map of
/// 8 or more is the only way into the branch with more leading blocks
/// than new slots (the game always asks for 1); smaller maps with count
/// 0 would grow anyway.
#[test]
fn deque_grow_map_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x6e0f_a1b2);
        for (sym, shift) in [(0x0068_cf60, 2), (0x0068_d0e0, 0)] {
            for i in 0..60 {
                t.reset();
                t.stub_alloc(a::OPERATOR_NEW, 1000);
                delete_stubs(t);
                let d = t.zeros(0x14);
                let ms = [0, 1, 2, 3, 4, 8, 16, 9][rng.below(8) as usize];
                let first = if ms == 0 { 0 } else { rng.below(ms) };
                let off = (first << shift) | rng.below(1 << shift);
                let count = match i % 4 {
                    0 => 0,
                    1 => 1,
                    2 => rng.below(20),
                    _ => rng.below(100),
                };
                let size = rng.below(ms * 4 + 1);
                made_up_deque(t, &mut rng, d, ms, off, size, 4);
                let what = format!("grow_map {sym:#x} #{i} ms {ms} first {first} count {count}");
                t.run(&what, sym, Conv::Thiscall(d), &[count], false, |g| {
                    a::grow_map(g, d, count, shift);
                    0
                });
            }
        }
    });
}

/// The pushes with the original `_Growmap`: empty maps, full blocks,
/// offsets past the map (masked), missing blocks.
#[test]
fn deque_push_back_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x9b5c_0003);
        for (sym, v4) in [(0x0068_d9f0, false), (0x0068_da80, true)] {
            for i in 0..60 {
                t.reset();
                t.stub_alloc(a::OPERATOR_NEW, 1000);
                delete_stubs(t);
                let d = t.zeros(0x14);
                let ms = [0, 1, 2, 4, 8][rng.below(5) as usize];
                let cap = if v4 { ms } else { ms * 4 };
                let size = if cap == 0 { 0 } else { rng.below(cap + 1) };
                // The game's states: an empty map has offset 0, and the
                // offset's block stays inside the map when the push grows
                // it (offsets past the map, masked by the push, only
                // with room left; else the original's _Growmap moves a
                // negative length)
                let room = if v4 { size + 1 < ms } else { size + 4 < cap };
                let off = if cap == 0 {
                    0
                } else if room && rng.chance(3) {
                    rng.next() & 0xff
                } else {
                    rng.below(cap - size + 1)
                };
                made_up_deque(t, &mut rng, d, ms, off, size, 3);
                let w: Vec<u32> = (0..4).map(|_| rng.next()).collect();
                let v = t.words(&w);
                let what = format!("push_back {sym:#x} #{i} ms {ms} size {size} off {off}");
                t.run(&what, sym, Conv::Thiscall(d), &[v], false, |g| {
                    if v4 {
                        a::push_v4(g, d, v);
                    } else {
                        a::push_u32(g, d, v);
                    }
                    0
                });
            }
        }
    });
}

/// No manager, stopped already, and one `SleepThread` (its stub stops
/// the thread).
#[test]
fn async_controller_destroy_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0de5_7001);
        for case in 0..3 {
            t.reset();
            delete_stubs(t);
            t.stub_import(a::CLOSE_HANDLE_IAT, false, 1, 1, 1);
            t.stub(a::CLEAR_U32, true, 0, 0, 1);
            t.stub(a::CLEAR_V4, true, 0, 0, 2);
            let g = t.rebase(a::ASYNC_CONTROLLER);
            let mut c = vec![0xa1];
            c.extend(g.to_le_bytes()); // mov eax, [manager]
            c.extend([0xc7, 0x40, 0x3c, 1, 0, 0, 0, 0xc3]); // mov [eax+0x3c], 1; ret
            let at = t.cave_code(&c);
            t.jump(a::SLEEP_THREAD, at);
            let m = if case == 0 {
                0
            } else {
                made_up_manager(t, &mut rng, 3, 3)
            };
            if case == 2 {
                t.w32(m + 0x3c, 0);
            }
            t.g32(a::ASYNC_CONTROLLER, m);
            let what = format!("async_controller_destroy #{case}");
            t.run(&what, 0x0068_db80, Conv::Cdecl, &[], false, |g| {
                a::async_controller_destroy(g);
                0
            });
        }
    });
}

// ---- input forwarders and singletons ----

#[test]
fn input_forwarders_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x1a9a_7b3c);
        for i in 0..6 {
            t.reset();
            let inst = rng.next() | 1;
            t.stub(a::INPUT_MANAGER, false, 0, 0, inst);
            t.stub(a::INPUT_SET_INFO, true, 0, 0, 0x5e70 + i);
            t.stub(a::INPUT_SWAP_STATE, true, 4, 4, 0x5e80 + i);
            t.stub(a::INPUT_SET_VIBRATION, true, 3, 3, 0x5e90 + i);
            t.run("set_input_info", 0x0063_0de0, Conv::Cdecl, &[], true, |g| {
                a::set_input_info_to_current_frame(g)
            });
            let w: Vec<u32> = (0..4).map(|_| rng.next()).collect();
            t.run("swap_state", 0x0063_0df0, Conv::Cdecl, &w, true, |g| {
                a::input_swap_state(g, w[0], w[1], w[2], w[3])
            });
            t.run(
                "set_vibration",
                0x0063_0e20,
                Conv::Cdecl,
                &w[..3],
                true,
                |g| a::input_set_vibration(g, w[0], w[1], w[2]),
            );
        }
    });
}

#[test]
fn singleton_create_and_destroy_match_the_original() {
    Twins::each(|t| {
        for case in 0..3u32 {
            // set already, new fails, new succeeds
            t.reset();
            t.stub_alloc(a::OPERATOR_NEW, if case == 1 { 0 } else { 10 });
            t.stub(a::TOUCH_RECORD_RESET, true, 0, 0, 3);
            t.stub(a::PARRAY_RESET16, true, 0, 0, 4);
            let cur = if case == 0 { 0x1234 } else { 0 };
            t.g32(a::TOUCH_RECORD, cur);
            t.g32(a::PARRAY16, cur);
            let what = format!("touch_record_create #{case}");
            t.run(&what, 0x0068_97d0, Conv::Cdecl, &[], false, |g| {
                a::touch_record_create(g);
                0
            });
            let what = format!("parray16_create #{case}");
            t.run(&what, 0x0068_ba20, Conv::Cdecl, &[], false, |g| {
                a::parray16_create(g);
                0
            });
            delete_stubs(t);
            for (sym, g) in [
                (0x006e_56d0, a::REMAP_BUTTON_ICON),
                (0x006e_76f0, a::SINGLETON_CE6DC0),
            ] {
                t.g32(g, if case == 1 { 0 } else { 0x5550 + case });
                let what = format!("delete singleton {sym:#x} #{case}");
                t.run(&what, sym, Conv::Cdecl, &[], false, |m| {
                    a::delete_singleton(m, g);
                    0
                });
            }
        }
    });
}

#[test]
fn save_menu_call_as3_matches_the_original() {
    Twins::each(|t| {
        for menu in [0, 0x00ab_cdef] {
            t.reset();
            t.stub(a::IGGY_CALL_AS3, true, 5, 5, 0x0a53);
            t.g32(a::IGGY_MENU_CEB248, menu);
            t.run(
                "changeSelectedBtn",
                0x0070_2a10,
                Conv::Cdecl,
                &[],
                true,
                |g| a::save_menu_call_as3(g),
            );
        }
    });
}

// ---- the debug GUI ----

#[test]
fn gui_window_set_global_trans_matches_the_original() {
    Twins::each(|t| {
        for v in [
            0u32,
            1,
            0x7f,
            0x80,
            0x81,
            0x1000,
            0xffff_ffff,
            0x8000_0000,
            0x7fff_ffff,
        ] {
            t.reset();
            let what = format!("set_global_trans {v:#x}");
            t.run(&what, 0x0084_8f20, Conv::Cdecl, &[v], false, |g| {
                a::gui_window_set_global_trans(g, v);
                0
            });
        }
    });
}

/// The parsers on strings up to 15 digits. Sixteen or more end the
/// process through `report_rangecheckfailure` after the original
/// overran its buffer: not run.
#[test]
fn gu_parsers_match_the_original() {
    Twins::each(|t| {
        t.skip = 4;
        let strings: [&[u8]; 16] = [
            b"",
            b"0",
            b"1f",
            b"ABCdef12",
            b"12g",
            b"0x1",
            b"fffffffff",
            b"123456789abcdef",
            b"G",
            b"9a",
            b"$",
            b"$7fz",
            b"$123456789abcdef",
            b"42 ",
            b"-5",
            b"\x8012",
        ];
        for s in strings {
            for sym in [0x0084_8f60u32, 0x0084_8fe0] {
                t.reset();
                let mut b = s.to_vec();
                b.push(0);
                let p = t.alloc(&b);
                let out = t.words(&[0x5a5a_5a5a]);
                let what = format!("parser {sym:#x} {:?}", String::from_utf8_lossy(s));
                // the buffer starts as hex digits: a missing terminator shows
                let init = [b'a'; 16];
                t.check_framed_init(&what, sym, Conv::Cdecl, &[p, out], 0x14, &init, |g, ebp| {
                    Out::U32(if sym == 0x0084_8f60 {
                        a::hex_parse(g, p, out, ebp - 0x14)
                    } else {
                        a::number_parse(g, p, out, ebp - 0x14)
                    })
                });
            }
        }
        t.skip = 0;
    });
}

// ---- teardown ----

#[test]
fn video_playback_dtor_matches_the_original() {
    Twins::each(|t| {
        t.reset();
        t.stub(a::MKV_READER_DTOR, true, 0, 0, 1);
        t.stub(a::FRAME_QUEUE_DTOR, true, 0, 0, 2);
        let this = t.zeros(0x200);
        t.run(
            "video dtor",
            0x00a2_6f30,
            Conv::Thiscall(this),
            &[],
            true,
            |g| a::video_playback_dtor(g, this),
        );
    });
}

/// Registered or not (the `stderr` message), asset and object set or
/// not.
#[test]
fn text_util_terminate_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e47_0001);
        for i in 0..12 {
            t.reset();
            delete_stubs(t);
            t.stub_import(a::IOB_FUNC_IAT, false, 0, 0, 0x0010_0000);
            t.stub_import(a::FPRINTF_IAT, false, 3, 0, 9);
            t.stub(a::TEXT_ASSET_RELEASE, true, 0, 0, 1);
            t.stub(a::FUN_00A295A0, true, 0, 0, 2);
            t.stub(a::CLUSTER_MANAGER, false, 0, 0, 0x00c1_0000);
            t.stub(a::RELEASE_PCLUSTER, true, 1, 1, 0x77 + i);
            let asset = if rng.chance(2) { 0 } else { 0x0a55 };
            let obj = if rng.chance(2) { 0 } else { 0x0b55 };
            let c = [rng.next(), rng.next()];
            let this = t.words(&[c[0], c[1], 0, asset, obj]);
            let mut list: Vec<u32> = (0..16).map(|_| rng.word()).collect();
            if i % 3 != 0 {
                list[rng.below(16) as usize] = this;
            }
            let bytes: Vec<u8> = list.iter().flat_map(|w| w.to_le_bytes()).collect();
            let at = t.rebase(a::TEXT_UTILS);
            t.patch(at, &bytes);
            let what = format!("text_util_terminate #{i}");
            t.run(&what, 0x006f_6950, Conv::Thiscall(this), &[], true, |g| {
                a::text_util_terminate(g, this)
            });
        }
    });
}

/// The texture-unload callees, recording; the UI object's material
/// chain and the scene set up.
fn texture_stubs(t: &mut Twins, rng: &mut Rng) {
    t.stubs(&[
        (a::SET_UI_INSTANCE_UNLOADED, true, 1, 1, 0x71),
        (a::UNIFY_FILENAME, false, 3, 0, 0x72),
        (a::SET_TEXTURE_UNLOADED, true, 1, 1, 0x73),
        (a::SCENE_UNLOAD_MENU_TEXTURE, true, 1, 1, 0x74),
        (a::MENU_UNLOAD_TEXTURE, true, 1, 1, 0x75),
        (a::MEMSET_THUNK, false, 3, 0, 0x76),
        (a::SA_MENU_TEXTURE_NAME, false, 2, 0, 0x77),
        (a::UNLOAD_DEFAULT_TEXTURES, false, 0, 0, 0x7d),
    ]);
    let geo = t.words(&[0x6e0_0000 | rng.below(256)]);
    let ui = t.zeros(0x2be0);
    t.w32(ui + 0x2bd4, geo);
    t.g32(a::GRAPHIC_UI, ui);
    t.g32(a::PHYRE_SCENE, 0x5ce0_0000 | rng.below(256));
}

/// Sets the name globals `syms` to strings or 0.
fn names(t: &mut Twins, rng: &mut Rng, syms: &[u32]) {
    for &s in syms {
        let v = if rng.chance(3) {
            0
        } else {
            t.alloc(b"menu/tex.dds\0")
        };
        t.g32(s, v);
    }
}

/// Every language of the three jump tables (0..=11), each name global
/// set or 0. The composite unloads run with their callees recording
/// (each is checked on its own here).
#[test]
#[allow(clippy::too_many_lines)] // one case runs all seven
fn menu_texture_unloads_match_the_original() {
    Twins::each(|t| {
        t.skip = 4;
        let mut rng = Rng(0x7e70_0001);
        for i in 0..24u32 {
            t.reset();
            texture_stubs(t, &mut rng);
            names(
                t,
                &mut rng,
                &[
                    0x00c5_9d64,
                    0x00c5_9d68,
                    0x00c5_9d74,
                    0x00c5_9d78,
                    0x00c5_9d7c,
                    0x00c5_9d6c,
                    0x00c5_9d84,
                    0x025d_16c8,
                    0x025d_16cc,
                ],
            );
            let lang = i % 12;
            t.stub(a::FFX_LANG, false, 0, 0, lang);
            t.stub(a::FFX_LANG_JMP, false, 0, 0, lang);
            let loc = t.words(&[0, lang]);
            t.stub(a::LOCALIZATION_MANAGER, false, 0, 0, loc);
            let this = t.zeros(0x210);
            if rng.chance(2) {
                let n = t.alloc(b"x.dds\0");
                t.w32(this + 0x20c, n);
            }
            let cd = Conv::Cdecl;
            let tc = Conv::Thiscall(this);
            t.check_framed(
                &format!("bg water #{i}"),
                0x0067_ecf0,
                cd,
                &[],
                0x104,
                |g, ebp| {
                    a::unload_bg_water(g, ebp - 0x104);
                    Out::None
                },
            );
            t.check_framed(
                &format!("game menu #{i}"),
                0x0067_ee70,
                cd,
                &[],
                0x104,
                |g, ebp| {
                    a::unload_game_menu(g, ebp - 0x104);
                    Out::None
                },
            );
            t.check_framed(
                &format!("sa names #{i}"),
                0x0067_de60,
                cd,
                &[],
                0x234,
                |g, ebp| {
                    a::unload_sa_names(g, ebp - 0x234);
                    Out::None
                },
            );
            t.run(
                &format!("sa lang {lang}"),
                0x0067_f030,
                tc,
                &[],
                true,
                |g| a::unload_sa_lang(g, this),
            );
            // the composites, their parts (checked above) recording
            t.stubs(&[
                (a::UNLOAD_SA_NAMES, false, 0, 0, 0x78),
                (a::UNLOAD_SA_LANG, true, 0, 0, 0x79),
            ]);
            let what = format!("sa menu lang {lang}");
            t.check_framed(&what, 0x0067_f230, tc, &[], 0x104, |g, ebp| {
                Out::U32(a::unload_sa_menu(g, this, ebp - 0x104))
            });
            t.stubs(&[
                (a::UNLOAD_GAME_MENU, false, 0, 0, 0x7a),
                (a::UNLOAD_BG_WATER, false, 0, 0, 0x7b),
                (a::UNLOAD_SA_MENU, true, 0, 0, 0x7c),
            ]);
            let what = format!("default menu lang {lang}");
            t.check_framed(&what, 0x0067_e9d0, tc, &[], 0x104, |g, ebp| {
                Out::U32(a::unload_default_menu(g, this, ebp - 0x104))
            });
            t.w32(this, i & 1);
            t.run(
                &format!("extra menu #{i}"),
                0x0067_ec50,
                tc,
                &[],
                true,
                |g| a::unload_extra_menu(g, this),
            );
        }
        t.skip = 0;
    });
}

#[test]
fn menu_manager_teardown_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7ea4_d0e1);
        for i in 0..16 {
            t.reset();
            delete_stubs(t);
            t.stubs(&[
                (a::MENU_TREE_RELEASE, true, 0, 0, 1),
                (a::FUN_006A1550, true, 0, 0, 2),
                (a::FUN_0067B850, true, 0, 0, 4),
                (a::UNLOAD_DEFAULT_MENU, true, 0, 0, 5),
                (a::MENU_UNLOAD_EXTRA, false, 0, 0, 6),
            ]);
            t.g32(a::MENU_TEXTURE_MANAGER, 0x00e7_0000 + i);
            let this = t.zeros(0x5b90);
            // the mesh table: a few entries, null objects among them
            if i % 4 != 0 {
                for _ in 0..30 {
                    let k = rng.below(0x578);
                    let obj = if rng.chance(3) { 0 } else { rng.next() | 1 };
                    let e = t.words(&[obj]);
                    t.w32(this + 0x15f0 + 4 * k, e);
                }
                let tree = if rng.chance(2) { 0 } else { 0x0071_0000 };
                let e = t.words(&[tree]);
                t.w32(this + 0x15f0, e);
            }
            for off in [
                0x5830, 0x5828, 0x4240, 0x4238, 0x2bd8, 0x2bd0, 0x15e8, 0x15e0,
            ] {
                // owned (bit 31 clear, bit 30 either way) or not
                let cap = match rng.below(3) {
                    0 => rng.below(16),
                    1 => 0x4000_0000 | rng.below(16),
                    _ => 0x8000_0000 | rng.below(16),
                };
                let data = if rng.chance(3) { 0 } else { rng.next() };
                t.w32(this + off, cap);
                t.w32(this + off + 4, data);
            }
            for off in [0x5b80, 0x5b84, 0x5b88] {
                let x = rng.next();
                t.w32(this + off, x);
            }
            let tc = Conv::Thiscall(this);
            t.run(
                &format!("menu meshes #{i}"),
                0x0067_b680,
                tc,
                &[],
                false,
                |g| {
                    a::menu_meshes_release(g, this);
                    0
                },
            );
            t.stub(a::MENU_MESHES_RELEASE, true, 0, 0, 3);
            t.run(
                &format!("menu teardown #{i}"),
                0x0067_b230,
                tc,
                &[],
                false,
                |g| {
                    a::menu_manager_teardown(g, this);
                    0
                },
            );
        }
    });
}

// ---- the Iggy menu manager and GDraw ----

#[test]
fn gdraw_context_matches_the_original() {
    Twins::each(|t| {
        for case in 0..8u32 {
            t.reset();
            delete_stubs(t);
            t.stubs(&[
                (a::QUEUE_CALLBACK, true, 2, 2, 0x0c0c),
                (a::SYNC_RENDER, true, 0, 0, 0x0c0d),
                (a::RING_JOB_DTOR, true, 0, 0, 0x0c0e),
            ]);
            let q = t.object(&[(0, 0), (0, 0), (0, 0x0c0f)]);
            let ctx = match case % 3 {
                0 => 0,
                1 => t.words(&[0, 0]),
                _ => t.words(&[0, q]),
            };
            t.g32(a::GDRAW_CONTEXT, ctx);
            t.g32(a::GDRAW_CONTEXT_REFS, [0, 1, 2, 1][case as usize % 4]);
            let what = format!("gdraw flush #{case}");
            t.run(&what, 0x00a3_d240, Conv::Cdecl, &[], false, |g| {
                a::gdraw_flush(g);
                0
            });
            let what = format!("end draw #{case}");
            t.run(&what, 0x00a3_62e0, Conv::Cdecl, &[], true, |g| {
                a::end_draw(g)
            });
            let what = format!("gdraw release #{case}");
            t.run(&what, 0x00a3_d570, Conv::Cdecl, &[], true, |g| {
                a::gdraw_release(g)
            });
        }
    });
}

#[test]
fn iggy_manager_teardown_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x1997_0001);
        for i in 0..16 {
            t.reset();
            delete_stubs(t);
            t.stubs(&[
                (a::IGGY_MENU_DTOR, true, 0, 0, 1),
                (a::GDRAW_RELEASE_QUEUE, false, 0, 0, 2),
                (a::GDRAW_RELEASE, false, 0, 0, 0),
            ]);
            // one list past 0xffff entries (the count's mask), mostly empty
            let n = if i == 5 { 0x1_0003 } else { rng.below(6) };
            let list: Vec<u32> = (0..n)
                .map(|k| {
                    if rng.chance(3) || (n > 6 && k < 0x1_0000) {
                        0
                    } else {
                        rng.next() | 1
                    }
                })
                .collect();
            let list = if n == 0 { 0 } else { t.words(&list) };
            let flag = if rng.chance(3) { 0x8000_0000 } else { 0 };
            let ctx = if rng.chance(2) { 0 } else { 0x00a1_0000 };
            let cap = if rng.chance(2) { 3 } else { 0x8000_0003 };
            let data = if rng.chance(3) { 0 } else { 0x00b1_0000 };
            let this = t.words(&[0, ctx, n | flag, list, cap, data]);
            let what = format!("iggy manager dtor #{i}");
            t.run(&what, 0x00a2_5eb0, Conv::Thiscall(this), &[], false, |g| {
                a::iggy_manager_dtor(g, this);
                0
            });
            t.stub(a::IGGY_MANAGER_DTOR, true, 0, 0, 3);
            t.g32(a::IGGY_MENU_MANAGER, if i % 3 == 0 { 0 } else { this });
            let what = format!("iggy manager destroy #{i}");
            t.run(&what, 0x00a2_6270, Conv::Cdecl, &[], false, |g| {
                a::iggy_manager_destroy(g);
                0
            });
        }
    });
}

// ---- batch 2 (ffx_pcui::m5u_b) ----

impl Twins {
    /// A stub over `sym` that copies `n` bytes from its first stack
    /// argument into a log of its own (so the test sees the buffer as it
    /// was at each call), pops `pops` words and returns 0.
    fn stub_snapshot(&mut self, sym: u32, n: u32, pops: u32) {
        let buf = self.zeros(0x2000);
        let cur = self.words(&[buf]);
        let mut c = vec![0x56, 0x57]; // push esi; push edi
        c.extend([0x8b, 0x74, 0x24, 0x0c]); // mov esi, [esp+0xc]
        c.extend([0x8b, 0x3d]);
        c.extend(cur.to_le_bytes()); // mov edi, [cur]
        c.push(0xb9);
        c.extend(n.to_le_bytes()); // mov ecx, n
        c.extend([0xf3, 0xa4]); // rep movsb
        c.extend([0x89, 0x3d]);
        c.extend(cur.to_le_bytes()); // mov [cur], edi
        c.extend([0x5f, 0x5e, 0x31, 0xc0]); // pop edi; pop esi; xor eax, eax
        c.push(0xc2);
        c.extend(((pops * 4) as u16).to_le_bytes());
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }

    fn cstr(&mut self, s: &str) -> u32 {
        let mut b = s.as_bytes().to_vec();
        b.push(0);
        self.alloc(&b)
    }
}

#[test]
fn mouse_state_create_matches_the_original() {
    Twins::each(|t| {
        for case in 0..4u32 {
            t.reset();
            t.stub_alloc(a::OPERATOR_NEW, if case == 1 { 0 } else { 4 });
            t.stub_import(b::GET_SYSTEM_METRICS_IAT, false, 1, 1, case & 2);
            t.g32(b::MOUSE_STATE, if case == 0 { 0x0123_4560 } else { 0 });
            let what = format!("mouse_state_create #{case}");
            t.run(&what, 0x00a2_5990, Conv::Cdecl, &[], false, |g| {
                b::mouse_state_create(g);
                0
            });
        }
    });
}

#[test]
fn button_icon_scale_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x1c0e_5ca1);
        for i in 0..20 {
            t.reset();
            let this = t.zeros(0x168);
            let (x, y) = match i {
                0 => (0x7fc0_0000, 0xffc0_1234), // quiet NaNs
                1 => (0x8000_0000, 0x0000_0001), // -0, a denormal
                2 => (0x7f80_0000, 0xff80_0000), // infinities
                _ => (rng.next() & 0x3fff_ffff, rng.next() | 0x8000_0000),
            };
            let tc = Conv::Thiscall(this);
            t.run("reset scale", 0x006e_76b0, tc, &[], false, |g| {
                b::reset_button_icon_scale(g, this);
                0
            });
            t.run(
                &format!("set scale #{i}"),
                0x006e_7720,
                tc,
                &[x, y],
                false,
                |g| {
                    b::set_button_icon_scale(g, this, x, y);
                    0
                },
            );
        }
    });
}

#[test]
fn shader_params_and_world_matrix_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5ade_0001);
        for case in 0..6u32 {
            t.reset();
            t.stub(b::SHADER_PARAM_WRITE, false, 3, 0, 0x5a00 + case);
            t.stub(b::SET_WORLD_MATRIX, false, 2, 0, 0x5b00 + case);
            let words: Vec<u32> = (0..0x48).map(|_| rng.next()).collect();
            let rec = t.words(&words);
            let block = if case == 1 { 0 } else { 0x00b1_0c00 + case };
            t.w32(rec, block);
            let rec = if case == 0 { 0 } else { rec };
            // without a record the original returns its caller's eax
            let ret = case != 0;
            let what = format!("shader params #{case}");
            t.run(&what, 0x0063_7c80, Conv::Cdecl, &[rec], ret, |g| {
                b::write_shader_params(g, rec)
            });
            t.skip = 4;
            let what = format!("world matrix #{case}");
            t.check_framed(&what, 0x0063_7d40, Conv::Cdecl, &[rec], 0x44, |g, ebp| {
                b::set_world_matrix(g, rec, ebp - 0x44);
                Out::None
            });
            t.skip = 0;
        }
    });
}

#[test]
fn menu_manager_small_methods_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x3e5a_0002);
        for i in 0..24u32 {
            t.reset();
            t.stubs(&[
                (a::SET_TEXTURE_UNLOADED, true, 1, 1, 0x61),
                (b::DYNGEO_BUFFER_ID, false, 0, 0, 0x62 + i),
                (b::PSTRING_ASSIGN, true, 1, 1, 0x63),
            ]);
            let this = t.zeros(0x5ba0);
            let geo = t.words(&[0x0060_0000 + i]);
            t.w32(this + 0x2c50, 0x5555_5555);
            t.w32(this + 0x2c54, 0x5555_5555);
            t.w32(this + 0x2bd4, geo);
            let tc = Conv::Thiscall(this);
            let name = rng.next();
            t.run(
                "set material unloaded",
                0x0067_e6a0,
                tc,
                &[name],
                true,
                |g| b::set_menu_ui_material_unloaded(g, this, name),
            );
            t.run("reset pause vertex", 0x0067_dfa0, tc, &[], true, |g| {
                b::reset_pause_vertex_info(g, this)
            });
            // the mesh lookup: gate, table choice, missing entries, pass
            t.write(this + 0x5b9d, &[u8::from(i % 6 != 0)]);
            t.w32(this + 0x5b7c, u32::from(i % 3 == 1));
            let k = rng.below(8);
            for table in [0x15f0, 0x4248] {
                if rng.chance(3) {
                    continue;
                }
                let mat = t.zeros(0xb8);
                t.write(mat + 0xb4, &(rng.below(3) as u16).to_le_bytes());
                let inner = t.zeros(0x40);
                let mesh = t.words(&[0, 0, 0, inner, 0, 0, 0, 0, 0, mat]);
                let e = t.words(&[mesh]);
                t.w32(this + table + 4 * k, e);
            }
            t.run(
                &format!("mesh instance #{i}"),
                0x0067_b9d0,
                tc,
                &[k],
                true,
                |g| b::get_menu_mesh_instance(g, this, k),
            );
        }
    });
}

#[test]
fn font_names_match_the_original() {
    Twins::each(|t| {
        for i in 0..6u32 {
            t.reset();
            let this = t.alloc(&[0x5a; 0x210]);
            t.run(
                "font names init",
                0x0067_b180,
                Conv::Thiscall(this),
                &[],
                true,
                |g| b::font_names_init(g, this),
            );
            // the event font: four loads (each buffer seen by the stub)
            t.stub_snapshot(b::SCENE_LOAD_MENU_TEXTURE, 40, 1);
            let suffixes = ["_a", "", "_long_suffix", "x"];
            for (k, sym) in b::FONT_SUFFIXES.into_iter().enumerate() {
                let s = t.cstr(suffixes[(k + i as usize) % 4]);
                t.g32(sym, s);
            }
            let name = t.cstr(["font", "", "scene_event_00", "a\u{1}b"][i as usize % 4]);
            let what = format!("event font #{i}");
            t.run(
                &what,
                0x0067_d370,
                Conv::Thiscall(this),
                &[name],
                false,
                |g| {
                    b::load_scene_event_font(g, this, name);
                    0
                },
            );
        }
    });
}

/// The load-side callees, recording.
fn load_stubs(t: &mut Twins, lang: u32) {
    t.stubs(&[
        (b::SCENE_LOAD_MENU_TEXTURE, true, 1, 1, 0x81),
        (b::LOAD_PCLUSTER_JUMP, true, 1, 1, 0x82),
        (a::CLUSTER_MANAGER, false, 0, 0, 0x00c1_0000),
        (b::LOAD_SA_MENU, true, 1, 1, 0x84),
        (b::LOAD_TEXTURE_CLUSTERS, false, 0, 0, 0x85),
        (b::LOAD_PCLUSTER, true, 1, 1, 0x00c1_5000),
        (b::GET_APPLICATION, false, 0, 0, 0x00a9_0000),
        (b::FIXUP_CLUSTERS, false, 2, 0, 0x87),
        (a::MENU_UNLOAD_TEXTURE, true, 1, 1, 0x88),
        (a::FFX_LANG_JMP, false, 0, 0, lang),
    ]);
    t.g32(a::PHYRE_SCENE, 0x5ce0_0000 + lang);
}

#[test]
#[allow(clippy::too_many_lines)] // one case runs every load
fn menu_texture_loads_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x10ad_0002);
        for i in 0..24u32 {
            t.reset();
            let lang = i % 12;
            load_stubs(t, lang);
            let loc = t.words(&[0, lang]);
            t.stub(a::LOCALIZATION_MANAGER, false, 0, 0, loc);
            t.stub(b::MENU_LOAD_TEXTURE, true, 1, 1, 0x89);
            let this = t.zeros(0x210);
            let tc = Conv::Thiscall(this);
            let cd = Conv::Cdecl;
            t.run("load default", 0x0067_c7a0, tc, &[], true, |g| {
                b::load_default_menu(g, this)
            });
            t.run("load bg water", 0x0067_c6f0, cd, &[], true, b_load_bg_water);
            for (sym, g) in [
                (0x0067_d650, 0x00c5_9d84),
                (0x0067_d670, 0x00c5_9d74),
                (0x0067_d690, 0x00c5_9d6c),
            ] {
                t.run(&format!("load one {sym:#x}"), sym, cd, &[], true, |m| {
                    b::load_one(m, g)
                });
            }
            t.run(
                "load dream map",
                0x0067_c8c0,
                cd,
                &[],
                true,
                b_load_dream_map,
            );
            t.run(
                &format!("copyright {lang}"),
                0x0067_c8f0,
                cd,
                &[],
                true,
                b_load_copyright,
            );
            t.run(
                &format!("unload copyright {lang}"),
                0x0067_ee40,
                tc,
                &[],
                true,
                |g| b::unload_copyright(g, this),
            );
            let kind = i % 5;
            t.run(
                &format!("game menu {kind}"),
                0x0067_cb30,
                Conv::Stdcall,
                &[kind],
                true,
                |g| b::load_game_menu(g, kind),
            );
            let screen = 9 + i % 9;
            t.run(
                &format!("cloud load {screen}"),
                0x0067_c720,
                tc,
                &[screen],
                true,
                |g| b::load_cloud_save_ui(g, this, screen),
            );
            t.run(
                &format!("cloud unload {screen}"),
                0x0067_edc0,
                tc,
                &[screen],
                true,
                |g| b::unload_cloud_save_ui(g, this, screen),
            );
            // MenuLoadTexture itself (the stub above is gone after reset)
            t.reset();
            load_stubs(t, lang);
            let id = rng.next();
            let what = format!("menu load texture #{i}");
            t.check_framed(&what, 0x0067_dbb0, tc, &[id], 4, |g, ebp| {
                Out::U32(b::menu_load_texture(g, id, ebp - 4))
            });
            // the save sub-menu: none, the same name, another name
            let name = t.cstr("menu/sa_sub.dds");
            let cur = match i % 3 {
                0 => 0,
                1 => t.cstr("menu/sa_sub.dds"),
                _ => t.cstr(["menu/sa_sub.dd", "menu/sa_sub.ddsx", "x"][i as usize % 3]),
            };
            t.w32(this + 0x20c, cur);
            let ebp = t.frame_ebp(1);
            Mem::write(&mut t.b, ebp + 8, name);
            let what = format!("sa sub menu #{i}");
            // ebp - 4 is `push ecx` (this) unless the cluster goes there
            let init = this.to_le_bytes();
            t.check_framed_init(&what, 0x0067_d0b0, tc, &[name], 4, &init, |g, ebp| {
                Out::U32(b::load_sa_sub_menu(g, this, name, ebp + 8, ebp - 4))
            });
            t.run(
                &format!("unload sa sub #{i}"),
                0x0067_f3c0,
                tc,
                &[],
                false,
                |g| {
                    b::unload_sa_sub_menu(g, this);
                    0
                },
            );
        }
    });
}

fn b_load_bg_water(g: &mut G) -> u32 {
    b::load_bg_water(g)
}

fn b_load_dream_map(g: &mut G) -> u32 {
    b::load_dream_map(g)
}

fn b_load_copyright(g: &mut G) -> u32 {
    b::load_copyright(g)
}

#[test]
fn menu_texture_unloads_b_match_the_original() {
    Twins::each(|t| {
        t.skip = 4;
        let mut rng = Rng(0x0d1e_0002);
        for _ in 0..12u32 {
            t.reset();
            texture_stubs(t, &mut rng);
            let syms: Vec<u32> = (0..5).map(|k| b::DREAM_MAP_NAMES + 4 * k).collect();
            names(t, &mut rng, &syms);
            names(t, &mut rng, &[0x00c5_9d84, 0x00c5_9d74, 0x00c5_9d6c]);
            t.check_framed(
                "dream map unload",
                0x0067_ebc0,
                Conv::Cdecl,
                &[],
                0x104,
                |g, ebp| {
                    b::unload_dream_map(g, ebp - 0x104);
                    Out::None
                },
            );
            for (sym, g) in [
                (0x0067_f920, 0x00c5_9d84),
                (0x0067_f9a0, 0x00c5_9d74),
                (0x0067_fa20, 0x00c5_9d6c),
            ] {
                t.check_framed(
                    &format!("unload one {sym:#x}"),
                    sym,
                    Conv::Cdecl,
                    &[],
                    0x104,
                    |m, ebp| {
                        b::unload_one(m, g, ebp - 0x104);
                        Out::None
                    },
                );
            }
        }
        t.skip = 0;
    });
}

#[test]
fn string_tidy_matches_the_original() {
    Twins::each(|t| {
        for cap in [0, 15, 16, 17, 0x100, 0xffff_ffff] {
            t.reset();
            delete_stubs(t);
            let this = t.words(&[0x6161_6161, 0x62, 0x63, 0x64, 7, cap]);
            let what = format!("string tidy cap {cap:#x}");
            t.run(&what, 0x0068_1800, Conv::Thiscall(this), &[], false, |g| {
                b::string_tidy(g, this);
                0
            });
        }
    });
}

// ---- batch 3 (ffx_pcui::m5u_c) ----

impl Twins {
    /// A stub over `sym` that returns `value` in `ST0` (`fld dword`).
    fn stub_float(&mut self, sym: u32, value: f32) {
        let cell = self.words(&[value.to_bits()]);
        let mut code = vec![0xd9, 0x05];
        code.extend(cell.to_le_bytes());
        code.push(0xc3);
        let addr = self.cave_code(&code);
        self.jump(sym, addr);
    }

    /// A recording stub over `sym` (`n` words, `pops` popped) that also
    /// stops the input thread: the manager's run flag `+4` cleared.
    fn stub_stop(&mut self, sym: u32, n: u32, pops: u32) {
        let rec = self.stub_code(sym, false, n, pops, 0);
        let g = self.rebase(a::ASYNC_CONTROLLER);
        let mut code = vec![0xa1];
        code.extend(g.to_le_bytes()); // mov eax, [manager]
        code.extend([0xc7, 0x40, 0x04, 0, 0, 0, 0]); // mov dword [eax+4], 0
        code.push(0xe9);
        let here = self.cave_next + code.len() as u32 + 4;
        code.extend(rec.wrapping_sub(here).to_le_bytes()); // jmp rec
        let addr = self.cave_code(&code);
        self.jump(sym, addr);
    }
}

#[test]
fn touch_and_parray_teardown_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7ea2_0003);
        for i in 0..16u32 {
            t.reset();
            delete_stubs(t);
            t.stub(c::RESET_ARRAYS, true, 0, 0, 1);
            t.stub(c::PARRAY_RESET, true, 0, 0, 2);
            let block = t.alloc(&[0xcd; 0x40]);
            t.g32(
                a::TOUCH_RECORD,
                if i % 3 == 0 { 0 } else { 0x0071_0000 + i },
            );
            t.run("touch destroy", 0x0068_9830, Conv::Cdecl, &[], false, |g| {
                c::touch_record_destroy(g);
                0
            });
            // PArray_free and the 16-array: owned or not, 16 already or not
            let cap = match i % 5 {
                0 => 0x10,
                1 => 3,
                2 => 0x8000_0010,
                3 => 0x4000_0003,
                _ => 0x8000_0002,
            };
            let entries: Vec<u32> = (0..16)
                .map(|_| if rng.chance(3) { 0 } else { rng.next() | 1 })
                .collect();
            let buf = t.words(&entries);
            // PhyreMalloc(0x40, 4): a block, the array's own buffer, or 0
            let got = match i % 3 {
                0 => block,
                1 => buf,
                _ => 0,
            };
            t.stub(c::PHYRE_MALLOC, false, 2, 0, got);
            // no buffer only where the original replaces it (else it reads
            // entries of a null array)
            let none = cap != 0x10 && got != 0 && rng.chance(3);
            let this = t.words(&[cap, if none { 0 } else { buf }]);
            let tc = Conv::Thiscall(this);
            t.run(
                &format!("parray16 clear #{i}"),
                0x0068_bae0,
                tc,
                &[],
                false,
                |g| {
                    c::parray16_clear(g, this);
                    0
                },
            );
            let x = t.words(&[cap, buf]);
            t.run(
                &format!("parray free #{i}"),
                0x0068_b830,
                Conv::Thiscall(x),
                &[],
                false,
                |g| {
                    c::parray_free(g, x);
                    0
                },
            );
            t.stub(c::PARRAY16_CLEAR, true, 0, 0, 3);
            let p = t.words(&[cap, buf]);
            t.g32(a::PARRAY16, if i % 5 == 0 { 0 } else { p });
            t.run(
                &format!("parray16 destroy #{i}"),
                0x0068_bba0,
                Conv::Cdecl,
                &[],
                false,
                |g| {
                    c::parray16_destroy(g);
                    0
                },
            );
        }
    });
}

#[test]
fn controller_ctor_and_create_match_the_original() {
    Twins::each(|t| {
        for i in 0..6u32 {
            t.reset();
            delete_stubs(t);
            t.stub_alloc(a::OPERATOR_NEW, 10);
            t.stub_import(c::CREATE_MUTEX_IAT, false, 3, 3, 0x4d00 + i);
            t.stub_float(
                c::INPUT_CUR_TIME,
                [1.5, -0.0, 1e30, 7.25, f32::INFINITY, 3.0][i as usize],
            );
            let id = if i % 2 == 0 {
                0x0003_0000 + i
            } else {
                0x8000_0000 + i
            };
            t.stub(c::CREATE_THREAD, false, 2, 0, id);
            t.stub(c::START_THREAD, false, 2, 0, 5);
            let this = t.alloc(&[0x33; 0x44]);
            // the parameter block only (the rest is the SEH frame)
            t.skip = 0x1c;
            let what = format!("controller ctor #{i}");
            t.check_framed(
                &what,
                0x0068_c830,
                Conv::Thiscall(this),
                &[],
                0x40,
                |g, ebp| Out::U32(c::controller_ctor(g, this, ebp - 0x40)),
            );
            t.skip = 0;
            // sCreateInstance: set, new fails, new succeeds
            t.reset();
            t.stub_alloc(a::OPERATOR_NEW, u32::from(i % 3 != 1));
            t.stub(c::CONTROLLER_CTOR, true, 0, 0, 0x00c7_0000 + i);
            t.g32(
                a::ASYNC_CONTROLLER,
                if i % 3 == 0 { 0x0012_0000 } else { 0 },
            );
            t.run(
                &format!("controller create #{i}"),
                0x0068_db00,
                Conv::Cdecl,
                &[],
                false,
                |g| {
                    c::controller_create(g);
                    0
                },
            );
            // the button deque's destructor
            delete_stubs(t);
            t.stub(a::CLEAR_U32, true, 0, 0, 1);
            let d = t.words(&[0x0044_0000 + i, 0, 0, 0, 0]);
            t.run(
                "deque dtor",
                0x0068_ca50,
                Conv::Thiscall(d),
                &[],
                false,
                |g| {
                    c::deque_u32_dtor(g, d);
                    0
                },
            );
        }
    });
}

/// One pass of the thread loop (the sleep and the poll stop the thread),
/// then its exit: the queues off, not due, due with the free camera
/// absent (made), idle, busy, `g_AsyncControllerOn` 2, NaN times.
#[test]
fn controller_thread_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7d1e_0003);
        let interval = f32::from_bits(0x3c88_8f86);
        // (now, prev): exactly the interval; far in the past (a long sleep);
        // an elapsed time only the f32 rounding makes whole
        let special = [(interval, 0.0), (0.0, 5.0), (0.75, 33_554_432.0)];
        for i in 0..46u32 {
            t.reset();
            t.stub_alloc(a::OPERATOR_NEW, 4);
            t.stub_stop(c::SLEEP_MS, 1, 0);
            t.stub_stop(c::ASYNC_POLL, 0, 0);
            t.stub(c::THREAD_EXIT, false, 1, 0, 0x7e00 + i);
            let now = match i % 8 {
                _ if i >= 40 => special[(i as usize - 40) % 3].0,
                0 => f32::NAN,
                1 => 0.0,
                _ => f32::from(rng.below(1000) as u16) / 997.0,
            };
            t.stub_float(c::INPUT_CUR_TIME, now);
            let cam = t.zeros(0x344);
            t.stub(c::FREE_CAMERA_CTOR, true, 0, 0, cam);
            let on = if i >= 40 {
                1
            } else {
                [0, 1, 1, 1, 2][(i as usize / 2) % 5]
            };
            t.g32(a::ASYNC_CONTROLLER_ON, on);
            let fc = match i % 5 {
                _ if (40..43).contains(&i) => cam,
                4 => {
                    t.write(cam + 0x33c, &[1]);
                    cam
                }
                0 => 0,
                1 => cam,
                2 => {
                    t.write(cam + 0x338, &[1]);
                    cam
                }
                _ => {
                    t.w32(cam, 1);
                    cam
                }
            };
            t.g32(c::FREE_CAMERA, fc);
            let prev = match i % 3 {
                _ if i >= 40 => special[(i as usize - 40) % 3].1,
                0 => now,
                1 => now - 0.5,
                _ => now - f32::from(rng.below(40) as u16) / 1000.0,
            };
            let m = t.words(&[
                0x0005_0000 + i,
                1,
                prev.to_bits(),
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ]);
            t.g32(a::ASYNC_CONTROLLER, m);
            let what = format!("controller thread #{i} on {on} now {now} prev {prev}");
            t.run(&what, 0x0068_d740, Conv::Cdecl, &[], true, |g| {
                c::controller_thread(g)
            });
        }
    });
}

#[test]
fn jump_thunks_and_esc_menu_ctor_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xe5c0_0003);
        for i in 0..8u32 {
            t.reset();
            t.stub(c::ESC_UPDATE_SPECIAL_FEATURE, true, 0, 0, 0x6600 + i);
            t.stub(c::CONVERT_LINE_BREAKS, false, 3, 0, 0x6700 + i);
            let this = rng.next();
            t.run(
                "special feature jump",
                0x0070_2560,
                Conv::Thiscall(this),
                &[],
                true,
                |g| c::esc_update_special_feature_jump(g, this),
            );
            let w = [rng.next(), rng.next(), rng.next()];
            t.run(
                "line breaks jump",
                0x006e_96b0,
                Conv::Cdecl,
                &w,
                true,
                |g| c::convert_line_breaks_jump(g, w[0], w[1], w[2]),
            );
            // the Esc menu (the jump thunk above is stubbed again here)
            t.reset();
            t.stub(c::IGGY_MENU_CTOR, true, 6, 6, 1);
            for (k, f) in c::ESC_MENU_INIT.into_iter().enumerate() {
                t.stub(f, true, 0, 0, 0x10 + k as u32);
            }
            t.stub(c::IGGY_MENU_SET_ENABLE, true, 1, 1, 2);
            t.stub(c::WINDOW_EVENT_INSERT, false, 1, 0, 3);
            let menu = t.alloc(&[0x77; 0x240]);
            let rect = match i {
                0 => [0x7fc0_0000, 0xffc1_0000, 0x8000_0000, 0x0000_0001],
                _ => [
                    rng.next() & 0x7f7f_ffff,
                    rng.next() & 0x7f7f_ffff,
                    rng.next() & 0xff7f_ffff,
                    0x4400_0000,
                ],
            };
            let path = rng.next();
            let args = [path, rect[0], rect[1], rect[2], rect[3]];
            t.run(
                &format!("esc menu ctor #{i}"),
                0x0068_dfd0,
                Conv::Thiscall(menu),
                &args,
                true,
                |g| c::esc_menu_ctor(g, menu, path, rect),
            );
        }
    });
}

impl Twins {
    /// A recording stub over `sym` (`this`, `n` words, `pops`) returning
    /// `seq[k]` on its `k`-th call.
    fn stub_seq(&mut self, sym: u32, this: bool, n: u32, pops: u32, seq: &[u32]) {
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
        if this {
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        }
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

    /// A recording stub over `sym` (cdecl `(value, text)`) that fills the
    /// four words of `value` with a running count, as `iggyDataValueSet`
    /// fills its value.
    fn stub_fill_value(&mut self, sym: u32) {
        self.stub_fill_value_with(sym, false, 2, 0);
    }

    /// [`Twins::stub_fill_value`] for other conventions (`LocString_ToValue
    /// (this; value)`: `this`, 1 word, 1 popped).
    fn stub_fill_value_with(&mut self, sym: u32, this: bool, n: u32, pops: u32) {
        let ctr = self.words(&[0x0a00_0000]);
        let rec = self.stub_code(sym, this, n, pops, 0x5e7);
        let mut c = vec![0x57]; // push edi
        c.extend([0x8b, 0x7c, 0x24, 0x08]); // mov edi, [esp+8]
        c.push(0xa1);
        c.extend(ctr.to_le_bytes()); // mov eax, [ctr]
        for off in [0u8, 4, 8, 0xc] {
            c.extend([0x89, 0x47, off]); // mov [edi+off], eax
            c.push(0x40); // inc eax
        }
        c.push(0xa3);
        c.extend(ctr.to_le_bytes()); // mov [ctr], eax
        c.push(0x5f); // pop edi
        c.push(0xe9);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(rec.wrapping_sub(here).to_le_bytes()); // jmp rec
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }
}

#[test]
fn esc_menu_callbacks_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xe5ca_0b03);
        for i in 0..8u32 {
            t.reset();
            t.stubs(&[
                (c::GET_DEREF, true, 0, 0, 0x00d0_0000 + i),
                (c::VALUE_REF_CREATE, true, 2, 2, 0x11),
                (c::HANDLERS_ADD_CALL, true, 2, 2, 0x12),
                (c::VALUE_REF_FREE, true, 0, 0, 0x13),
                (c::IGGY_CALL_AS3, true, 5, 5, 0x14),
            ]);
            let kind = [0xd, 0x1_000d, 0xc, 0xffff_000d][i as usize % 4];
            t.stub_seq(c::GET_ARRAY_S32, true, 1, 1, &[kind, 0x5150 + i]);
            let menu = rng.next();
            let args = t.words(&[0, rng.next(), 0, 0]);
            t.skip = 0xc;
            let what = format!("parameter button #{i}");
            t.check_framed(
                &what,
                0x0068_f610,
                Conv::Cdecl,
                &[menu, args],
                0x48,
                |g, ebp| {
                    c::on_parameter_button(g, menu, args, ebp - 0x48);
                    Out::None
                },
            );
            t.skip = 0;
            let yes = i & 1;
            t.run(
                "queue default key",
                0x0068_f6b0,
                Conv::Cdecl,
                &[yes, menu],
                // on no the original returns its caller's eax
                yes != 0,
                |g| c::queue_default_key(g, yes, menu),
            );
            t.run(
                "on default key",
                0x0068_f6d0,
                Conv::Cdecl,
                &[menu],
                true,
                |g| c::call_on_default_key(g, menu),
            );
            t.run(
                "special help",
                0x0068_f6f0,
                Conv::Cdecl,
                &[menu],
                true,
                |g| c::on_special_feature_help(g, menu),
            );
            t.run("on quit", 0x0068_fb00, Conv::Cdecl, &[menu], true, |g| {
                c::on_quit(g, menu)
            });
        }
    });
}

#[test]
fn esc_menu_popups_match_the_original() {
    Twins::each(|t| {
        for i in 0..6u32 {
            t.reset();
            t.stubs(&[
                (c::GET_LOC_TEXT, false, 3, 0, 0x21),
                (c::LINE_BREAKS_JUMP, false, 3, 0, 0x22),
                (c::CALL_FLASH_3, false, 14, 0, 0x23),
                (c::IGGY_MENU_SET_ENABLE, true, 1, 1, 0x24),
                (c::SETTINGS_SAVE, true, 0, 0, 0x25),
                (c::RESET_TO_LAUNCHER, false, 0, 0, 0x26),
            ]);
            t.stub_fill_value(c::DATA_VALUE_SET);
            t.stub_import(c::SHOW_CURSOR_IAT, false, 1, 1, 0x27);
            let esc = t.alloc(&[0x44; 0x140]);
            t.g32(c::ESC_MENU, esc);
            t.g32(c::SETTINGS, 0x00e5_0000 + i);
            let menu = 0x00a0_0000 + i;
            t.skip = 4;
            let what = format!("default key pop-up #{i}");
            t.check_framed(&what, 0x0068_f710, Conv::Cdecl, &[menu], 0x44c, |g, ebp| {
                c::show_default_key_popup(g, menu, ebp - 0x44c);
                Out::None
            });
            t.skip = 0;
            // the pop-up's button: the answer callback (a recording stub)
            let cb = t.stub_code(0xcb, false, 2, 0, 0x28 + i);
            t.w32(esc + 0x10c, if i % 3 == 0 { 0 } else { cb });
            let x = [1.7f64, -2.5, f64::NAN, 3e9, 0.0, -0.99][i as usize];
            let args = t.words(&[0, 0, x.to_bits() as u32, (x.to_bits() >> 32) as u32]);
            // without a callback the original returns its caller's eax
            let ret = i % 3 != 0;
            t.run(
                &format!("pop-up button #{i}"),
                0x0068_f8f0,
                Conv::Cdecl,
                &[menu, args],
                ret,
                |g| c::on_popup_button(g, args),
            );
            // the quit answer: no, yes with the cursor shown or hidden
            t.write(esc + 0x130, &[[0u8, 1, 2][(i as usize / 2) % 3]]);
            let yes = u32::from(i % 3 != 2);
            let ret = yes != 0;
            t.run(
                &format!("quit confirmed #{i}"),
                0x0068_fcf0,
                Conv::Cdecl,
                &[yes],
                ret,
                |g| c::quit_confirmed(g, yes),
            );
        }
    });
}

// ---- batch 4 (ffx_pcui::m5u_d) ----

/// The text on and off, glyph counts 0..big, screen sizes (0, odd, huge,
/// past 2^31), scales and offsets including NaN and infinities. The
/// matrix helpers run as the original's (they are library code).
#[test]
fn text_resize_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e5e_0004);
        for i in 0..40u32 {
            t.reset();
            t.stub(d::TEXT_SCROLL_TO_VISIBLE, true, 1, 1, 0x31);
            let obj = t.zeros(0x200);
            let counts = t.words(&[0, [0, 1, 7, 100, 0x3fff_ffff][i as usize % 5]]);
            let asset = t.zeros(0x40);
            t.w32(asset + 0x30, counts);
            let this = t.alloc(&[0x66; 0x90]);
            t.write(this + 0x80, &[u8::from(i % 7 != 0)]);
            t.w32(this + 0x10, if i % 11 == 3 { 0 } else { obj });
            t.w32(this + 0xc, if i % 13 == 5 { 0 } else { asset });
            let f = |r: &mut Rng| match r.below(6) {
                0 => 0x7fc0_0000,
                1 => 0x7f80_0000,
                2 => 0,
                _ => (r.next() & 0x3fff_ffff) | 0x3000_0000,
            };
            for off in [0x84, 0x88, 0x8c] {
                let v = f(&mut rng);
                t.w32(this + off, v);
            }
            let top = rng.next();
            t.w32(this + 0x74, top);
            let w = [1280, 0, 1, 0xffff_ffff, 0x8000_0000, 1920][i as usize % 6];
            let hh = [720, 0, 3, 1080, 0x9000_0000, 1][(i as usize / 6) % 6];
            t.skip = 0;
            let what = format!("text resize #{i} {w}x{hh}");
            t.check_framed(
                &what,
                0x006f_6670,
                Conv::Thiscall(this),
                &[w, hh],
                0xc8,
                |g, ebp| {
                    d::text_resize(g, this, w, hh, ebp - 0xc8);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn vec_push_back_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0ec7_0004);
        for i in 0..30u32 {
            t.reset();
            t.stub_alloc(a::OPERATOR_NEW, 100);
            delete_stubs(t);
            let below = t.words(&[rng.next()]);
            let n = rng.below(6);
            let cap = n + rng.below(3);
            let words: Vec<u32> = (0..cap.max(1)).map(|_| rng.next()).collect();
            let buf = t.words(&words);
            let (begin, end, last) = if i % 5 == 0 {
                (0, 0, 0)
            } else {
                (buf, buf + 4 * n, buf + 4 * cap)
            };
            let vec = t.words(&[begin, end, last]);
            // an element of its own, or a word elsewhere
            let v = match i % 3 {
                0 if n > 0 => buf + 4 * rng.below(n),
                1 => below,
                _ => t.words(&[rng.next()]),
            };
            let what = format!("vec push #{i} n {n} cap {cap}");
            t.run(&what, 0x006a_2b20, Conv::Thiscall(vec), &[v], false, |g| {
                d::vec_push_back(g, vec, v);
                0
            });
        }
    });
}

#[test]
fn iggy_tokenizer_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x70ce_0004);
        let texts = [
            "",
            "a",
            " a b  c ",
            ",,x,,y",
            "one",
            "  ",
            "a,b c",
            "\u{80}x y",
            "trailing ",
            "a\u{1}b c\u{1}d",
        ];
        for i in 0..36u32 {
            t.reset();
            t.stub_alloc2(c::PHYRE_MALLOC, if i % 9 == 4 { 1 } else { 100 });
            delete_stubs(t);
            t.stub(d::RELEASE_TAGGED_WORDS, false, 2, 0, 0x41);
            let text = texts[i as usize % texts.len()];
            let s = if i % 12 == 11 { 0 } else { t.cstr(text) };
            let delims = match i % 4 {
                0 => t.cstr(" "),
                1 => t.cstr(" ,"),
                2 => t.cstr(","),
                _ => 0,
            };
            let cd = Conv::Cdecl;
            t.run("skip delims", 0x00a2_9030, cd, &[s, delims], true, |g| {
                d::skip_delims(g, s, delims)
            });
            t.run("count tokens", 0x00a2_90d0, cd, &[s, delims], true, |g| {
                d::count_tokens(g, s, delims)
            });
            // an array of 0..4 slots, owned or not, with tagged pointers
            // the token count (as the original counts) for half the arrays,
            // so their old strings are freed; else a different count
            let set: &[u8] = match i % 4 {
                0 => b" ",
                1 => b" ,",
                2 => b",",
                _ => b"",
            };
            let tokens = if s == 0 || set.is_empty() {
                0
            } else {
                text.as_bytes()
                    .split(|b| set.contains(b))
                    .filter(|w| !w.is_empty())
                    .count() as u32
            };
            let k = if i % 2 == 0 { tokens } else { rng.below(5) };
            let slots: Vec<u32> = (0..k.max(1))
                .map(|_| [0, 0x1234_5679, 0x00ab_0000][rng.below(3) as usize])
                .collect();
            let words = t.words(&slots);
            let owned = [0, 0x8000_0000, 0x4000_0000][rng.below(3) as usize];
            let arr = t.words(&[k | owned, if k == 0 { 0 } else { words }]);
            let what = format!("tokenize #{i}");
            t.run(&what, 0x00a2_9140, cd, &[s, delims, arr], true, |g| {
                d::tokenize(g, s, delims, arr)
            });
        }
    });
}

#[test]
fn remap_icon_and_skip_prompt_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x1c0b_0004);
        for i in 0..6u32 {
            t.reset();
            t.stub_float(
                d::GRAPHIC_GET_TIME,
                [0.0, 1.5, -2.0, 1e20, 3.25, 7.0][i as usize],
            );
            t.stub(d::ICON_SLOT_SET, true, 5, 5, 0x51);
            let this = t.alloc(&[0x66; 0x70]);
            let tc = Conv::Thiscall(this);
            t.run("icon ctor", 0x006e_77d0, tc, &[], true, |g| {
                d::icon_state_init(g, this)
            });
            t.run("skip show", 0x006e_78a0, tc, &[], false, |g| {
                d::skip_prompt_show(g, this);
                0
            });
            t.run("skip hide", 0x006e_78c0, tc, &[], false, |g| {
                d::skip_prompt_hide(g, this);
                0
            });
            let b = [rng.next(), rng.next(), rng.next()];
            t.run("icon buttons", 0x006e_78e0, tc, &b, false, |g| {
                d::icon_set_buttons(g, this, b);
                0
            });
        }
    });
}

/// A small tree: head (nil) with root, and every node's predecessor,
/// the head included.
#[test]
fn tree_dec_matches_the_original() {
    Twins::each(|t| {
        // nodes: [left, parent, right, (byte +0xd nil)]
        let head = t.zeros(0x10);
        let nodes: Vec<u32> = (0..8).map(|_| t.zeros(0x10)).collect();
        t.write(head + 0xd, &[1]);
        // shape: 3 is the root; 1 (0, 2) on the left, 5 (4, 6) on the right
        let link = |t: &mut Twins, n: u32, l: u32, p: u32, r: u32| {
            t.w32(n, l);
            t.w32(n + 4, p);
            t.w32(n + 8, r);
        };
        let nd = |k: usize| nodes[k];
        link(t, head, nd(0), nd(3), nd(6));
        link(t, nd(3), nd(1), head, nd(5));
        link(t, nd(1), nd(0), nd(3), nd(2));
        link(t, nd(5), nd(4), nd(3), nd(6));
        for k in [0, 2, 4, 6] {
            let p = if k < 3 { nd(1) } else { nd(5) };
            link(t, nd(k), head, p, head);
        }
        // node 7 between 1 and 2: node 2's left child
        link(t, nd(2), nd(7), nd(1), head);
        link(t, nd(7), head, nd(2), head);
        for start in std::iter::once(head).chain(nodes.iter().copied()) {
            t.reset();
            let it = t.words(&[start]);
            t.run(
                "tree dec",
                0x006e_91f0,
                Conv::Thiscall(it),
                &[],
                true,
                |g| d::tree_dec(g, it),
            );
        }
    });
}

#[test]
fn iggy_helpers_and_debug_fill_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x1990_0004);
        for i in 0..8u32 {
            t.reset();
            delete_stubs(t);
            t.stubs(&[
                (d::HELPER_FIND, true, 1, 1, 0x00f0_0000 + i),
                (d::HELPER_INSERT, true, 4, 4, 0x61),
                (d::UNLOCK_ACHIEVEMENT, false, 1, 0, 0x62),
                (d::DEBUG_FULL_ITEM, false, 0, 0, 0x63),
                (d::DEBUG_FULL_COMMAND, false, 0, 0, 0x64),
                (d::DEBUG_FULL_GILL, false, 0, 0, 0x65),
                (d::GET_DEREF, true, 0, 0, 0x00d1_0000 + i),
            ]);
            t.stub_import(d::VALUE_REF_FREE_IAT, false, 2, 2, 0x66);
            t.stub_import(d::VALUE_REF_FROM_TEMP_IAT, false, 3, 3, 0x00d2_0000 + i);
            t.stub_import(d::VALUE_PATH_FROM_REF_IAT, false, 3, 3, 0x67);
            t.stub_import(d::VALUE_PATH_ARRAY_IAT, false, 3, 3, 0x68);
            t.stub_import(d::VALUE_PATH_SET_INDEX_IAT, false, 2, 2, 0x69);
            t.stub_import(d::VALUE_GET_S32_IAT, false, 4, 4, 0x6a);
            t.skip = 0;
            t.check_framed(
                "init iggy helper",
                0x006e_96c0,
                Conv::Cdecl,
                &[],
                8,
                |g, ebp| Out::U32(d::init_iggy_helper(g, ebp - 8)),
            );
            t.g32(d::CAMERA_DATA, if i % 3 == 0 { 0 } else { 0x00ca_0000 + i });
            t.run("unload camera", 0x006f_6f80, Conv::Cdecl, &[], false, |g| {
                d::unload_camera_data(g);
                0
            });
            let yes = u32::from(i % 4 != 3);
            let choice = t.words(&[[0, 1, 2, 3, 0xffff_ffff, 7, 1, 0][i as usize]]);
            t.run(
                "debug fill",
                0x006f_c970,
                Conv::Cdecl,
                &[yes, choice],
                yes != 0,
                |g| d::on_debug_fill(g, yes, choice),
            );
            let r = t.words(&[rng.next(), rng.next()]);
            t.run(
                "value ref free",
                0x006f_cc60,
                Conv::Thiscall(r),
                &[],
                true,
                |g| d::value_ref_free(g, r),
            );
            let this = rng.next();
            let out = t.zeros(0x30);
            let args = t.words(&[0, rng.next()]);
            // ebp - 4 holds `this` (the original keeps it there)
            t.skip = 4;
            t.check_framed(
                "read s32 x12",
                0x006f_ed60,
                Conv::Thiscall(this),
                &[out, args],
                0x34,
                |g, ebp| Out::U32(d::read_s32_array12(g, this, out, args, ebp - 0x34)),
            );
        }
    });
}

// ---- batch 5 (ffx_pcui::m5u_e) ----

#[test]
fn iggy_player_open_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0be5_0005);
        for i in 0..10u32 {
            t.reset();
            // operator new logged (its size), 0 for a failure
            let block = if i % 4 == 3 { 0 } else { t.zeros(0xa0) };
            t.stubs(&[
                (a::OPERATOR_NEW, false, 1, 0, block),
                (a::UNIFY_FILENAME, false, 3, 0, 0x71),
                (c::IGGY_MENU_CTOR, true, 6, 6, 0x00a5_0000 + i),
                (e::IGGY_ADD_CALLBACK, true, 2, 2, 0x72),
            ]);
            let path = t.cstr("menu/escmenu.swf");
            let ints = [
                rng.word(),
                rng.below(1920),
                [0, 1080, 0x8000_0000, 7][i as usize % 4],
                rng.next(),
            ];
            let floats = [
                0x3f80_0000,
                rng.next() & 0x7f7f_ffff,
                if i == 0 { 0x7fc0_0000 } else { 0x8000_0000 },
                0x4480_0000,
            ];
            // the original's frame beyond the path buffer holds its locals
            t.skip = 0x10;
            let args = [path, ints[0], ints[1], ints[2], ints[3]];
            t.check_framed(
                "open scaled",
                0x006e_9250,
                Conv::Cdecl,
                &args,
                0x110,
                |g, ebp| Out::U32(e::open_scaled(g, path, ints, ebp - 0x110)),
            );
            let args = [path, floats[0], floats[1], floats[2], floats[3]];
            t.check_framed("open", 0x006e_93f0, Conv::Cdecl, &args, 0x110, |g, ebp| {
                Out::U32(e::open_at(g, path, floats, ebp - 0x110))
            });
            t.skip = 0;
        }
    });
}

/// The Esc menu's pop-up callees, recording.
fn esc_stubs(t: &mut Twins, i: u32) -> u32 {
    t.stubs(&[
        (
            e::SCENARIO_FLAG,
            false,
            0,
            0,
            [0, 1, 0x1_0000, 0xffff][i as usize % 4],
        ),
        (e::IS_ETERNAL_CALM, false, 1, 0, u32::from(i % 5 == 4)),
        (c::CALL_FLASH_3, false, 14, 0, 0x75),
        (e::SET_POPUP_CALLBACK, true, 2, 2, 0x76),
        (e::CALL_FLASH_1, false, 6, 0, 0x77),
        (e::PLAY_MENU_SOUND, true, 1, 1, 0x78),
        (c::IGGY_CALL_AS3, true, 5, 5, 0x79),
        (c::HANDLERS_ADD_CALL, true, 2, 2, 0x7a),
        (e::HANDLERS_ADD, true, 4, 4, 0x7b),
        (e::SET_WORD_AT, true, 2, 2, 0x7c),
        (e::READ_S32_ARRAY12, true, 2, 2, 0x7d),
    ]);
    // LocString_Load(buf; id): logged, its string a constant
    t.stub(e::LOC_STRING_LOAD, true, 1, 1, 0x5151);
    t.stub_fill_value(c::DATA_VALUE_SET);
    t.stub_fill_value_with(e::LOC_STRING_TO_VALUE, true, 1, 1);
    let esc = t.alloc(&[0x44; 0x140]);
    t.g32(c::ESC_MENU, esc);
    t.g32(e::MOVIE_FADING, u32::from(i % 3 == 2));
    t.g32(c::SETTINGS, 0x00e5_0000 + i);
    esc
}

#[test]
fn esc_menu_settings_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5e77_0005);
        for i in 0..12u32 {
            t.reset();
            let esc = esc_stubs(t, i);
            t.stub_alloc(e::OPERATOR_NEW_ARRAY, 10);
            delete_stubs(t);
            t.skip = 4;
            let arg = rng.next();
            t.check_framed(
                "parameter clicked",
                0x006f_c7b0,
                Conv::Cdecl,
                &[arg],
                0x364,
                |g, ebp| {
                    e::on_parameter_button_clicked(g, arg, ebp - 0x364);
                    Out::None
                },
            );
            let changed = i & 1;
            let tc = Conv::Thiscall(esc);
            t.check_framed(
                "confirm video",
                0x006f_f060,
                tc,
                &[changed],
                0x364,
                |g, ebp| Out::U32(e::confirm_video_settings(g, esc, changed, ebp - 0x364)),
            );
            t.skip = 0;
            let yes = [0, 1, 2][i as usize % 3];
            t.run(
                "video applied",
                0x006f_f1b0,
                Conv::Cdecl,
                &[yes],
                yes != 0,
                |g| e::on_video_applied(g, yes),
            );
            t.run(
                "video answer",
                0x006f_f490,
                Conv::Cdecl,
                &[yes],
                yes != 0,
                |g| e::on_video_answer(g, yes),
            );
            // the 12 values: the stub leaves the new buffer as allocated
            // (0xcd), so they differ from the menu's unless it holds 0xcd too
            let menu = t.alloc(&[0x11; 0xe0]);
            if i % 2 == 0 {
                t.write(menu + 0xa4, &[0xcd; 0x30]);
                if i % 4 == 2 {
                    t.w32(menu + 0xa4 + 4 * (i % 12), 0);
                }
            }
            let args = t.words(&[0, rng.next(), 0, 0]);
            t.run(
                "video changed",
                0x006f_f1e0,
                Conv::Cdecl,
                &[menu, args],
                true,
                |g| e::on_video_settings_changed(g, menu, args),
            );
            t.run(
                "video refresh",
                0x006f_f4b0,
                Conv::Cdecl,
                &[menu, args],
                true,
                |g| e::on_video_setting_refresh(g, menu, args),
            );
            let x = [0.0f64, 55.7, -3.2, f64::NAN, 1e12, 100.0][i as usize % 6];
            let sargs = t.words(&[0, 0, x.to_bits() as u32, (x.to_bits() >> 32) as u32]);
            for (k, sym) in [0x006f_f4f0u32, 0x006f_f530, 0x006f_f570, 0x006f_f5b0]
                .into_iter()
                .enumerate()
            {
                let which = e::VOLUMES[k];
                t.run(
                    &format!("volume {sym:#x}"),
                    sym,
                    Conv::Cdecl,
                    &[menu, sargs],
                    true,
                    |g| e::on_volume_changed(g, menu, sargs, which),
                );
            }
        }
    });
}

// ---- batch 6 (ffx_pcui::m5u_f) ----

#[test]
fn esc_menu_audio_and_keys_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xa0d1_0006);
        for i in 0..8u32 {
            t.reset();
            delete_stubs(t);
            let cast = 0x00ca_0000 + i;
            t.stubs(&[
                (ff::RT_DYNAMIC_CAST, false, 5, 0, cast),
                (c::GET_DEREF, true, 0, 0, 0x00d1_0000 + i),
                (c::VALUE_REF_CREATE, true, 2, 2, 0x81),
                (ff::SET_WORD_AT, true, 2, 2, 0x82),
                (c::HANDLERS_ADD_CALL, true, 2, 2, 0x83),
                (c::VALUE_REF_FREE, true, 0, 0, 0x84),
                (ff::GET_APPLICATION, false, 0, 0, 0x00a9_0000),
                (ff::INPUT_SET_KEY, true, 3, 3, 0x85),
                (ff::SWAP_STATE, false, 4, 0, 0x86),
                (c::IGGY_MENU_SET_ENABLE, true, 1, 1, 0x87),
            ]);
            let vals: Vec<u32> = (0..4).map(|_| rng.next()).collect();
            t.stub_seq(ff::GET_ARRAY_S32, true, 1, 1, &vals);
            t.g32(c::SETTINGS, 0x00e5_0000 + i);
            let menu = rng.next();
            let args = t.words(&[0, rng.next()]);
            // the values by index and the value ref (the rest is SEH)
            t.skip = 0x10;
            t.check_framed(
                "audio refresh",
                0x006f_f660,
                Conv::Cdecl,
                &[menu, args],
                0x5c,
                |g, ebp| Out::U32(ff::on_audio_setting_refresh(g, menu, args, ebp - 0x5c)),
            );
            t.skip = 0;
            // the member trampolines: recording member stubs
            let m1 = t.stub_code(0xf1, true, 1, 1, 0x88 + i);
            let this = rng.next();
            let arg = rng.next();
            t.check_framed_init(
                "call member",
                0x006f_fbb0,
                Conv::Cdecl,
                &[this, m1, arg],
                8,
                &[0x5a; 8],
                |g, ebp| Out::U32(ff::call_member1(g, this, m1, arg, ebp - 8)),
            );
            let m0 = t.stub_code(0xf0, true, 0, 0, 0x89);
            let bound = t.words(&[m0, rng.below(64), rng.next()]);
            t.run(
                "bound member",
                0x006f_fc30,
                Conv::Cdecl,
                &[bound],
                false,
                |g| {
                    ff::call_bound_member(g, bound);
                    0
                },
            );
            t.run("clear keys", 0x006f_fc50, Conv::Cdecl, &[], true, |g| {
                ff::clear_keys(g)
            });
            let obj = t.alloc(&[0x33; 0x100]);
            t.check_framed(
                "reset input",
                0x0070_1930,
                Conv::Thiscall(obj),
                &[],
                0xc,
                |g, ebp| Out::U32(ff::reset_input_state(g, obj, ebp - 0xc)),
            );
            t.run(
                "key incomplete",
                0x0070_1660,
                Conv::Cdecl,
                &[menu],
                true,
                |g| ff::on_key_incomplete(g, menu),
            );
            let m = if i % 3 == 0 { 0 } else { 0x00ab_0000 + i };
            t.g32(ff::SAVE_LOAD_MENU, m);
            let on = [0, 1, 7][i as usize % 3];
            t.run(
                "save menu enable",
                0x0070_29e0,
                Conv::Cdecl,
                &[on],
                m != 0,
                |g| ff::save_menu_set_enable(g, on),
            );
        }
    });
}

/// The string read: the stub writes a length through the pointer it gets
/// (the original's own argument slot).
#[test]
fn iggy_get_string_utf8_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5718_0006);
        for i in 0..6u32 {
            t.reset();
            t.stub_import(ff::PATH_ARRAY_IAT, false, 3, 3, 0x91);
            // GetStringUTF8RS(.., &len): *len = 0x40 + i, logged
            let rec = t.stub_code(ff::GET_STRING_UTF8_IAT, false, 6, 6, 0x92);
            let mut code = vec![0x8b, 0x44, 0x24, 0x18]; // mov eax, [esp+0x18]
            code.extend([0xc7, 0x00]);
            code.extend((0x40 + i).to_le_bytes()); // mov dword [eax], len
            code.push(0xe9);
            let here = t.cave_next + code.len() as u32 + 4;
            code.extend(rec.wrapping_sub(here).to_le_bytes()); // jmp rec
            let stub = t.cave_code(&code);
            let slot = t.rebase(ff::GET_STRING_UTF8_IAT);
            t.patch(slot, &stub.to_le_bytes());
            let this = rng.next() & 0x7fff_fff0;
            let (k, buf, size) = (rng.below(9), rng.next(), rng.below(512));
            let tc = Conv::Thiscall(this);
            t.check_framed(
                "get string",
                0x00a2_8b60,
                tc,
                &[k, buf, size],
                0x18,
                |g, ebp2| {
                    Out::U32(ff::get_string_utf8(
                        g,
                        this,
                        k,
                        buf,
                        size,
                        ebp2 + 8,
                        ebp2 - 0x18,
                    ))
                },
            );
        }
    });
}

// ---- batch 7 (ffx_pcui::m5u_f) ----

impl Twins {
    /// `inputSwapState_00630df0(out, a, b, c)` -> `out`: `out` gets three
    /// words of a running count, `a`, `b`, `c` are kept in memory (the
    /// diff sees them).
    fn stub_swap_state(&mut self, sym: u32) {
        let cell = self.words(&[0x0b00_0000, 0, 0, 0]);
        let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
        c.extend([0x8b, 0x0d]);
        c.extend(cell.to_le_bytes()); // mov ecx, [cell]
        for off in [0u8, 4, 8] {
            c.extend([0x89, 0x48, off]); // mov [eax+off], ecx
            c.push(0x41); // inc ecx
        }
        c.extend([0x89, 0x0d]);
        c.extend(cell.to_le_bytes()); // mov [cell], ecx
        for (k, sp) in [(1u32, 8u8), (2, 0xc), (3, 0x10)] {
            c.extend([0x8b, 0x4c, 0x24, sp]); // mov ecx, [esp+sp]
            c.extend([0x89, 0x0d]);
            c.extend((cell + 4 * k).to_le_bytes()); // mov [cell+4k], ecx
        }
        c.push(0xc3);
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
    }
}

#[test]
fn key_capture_matches_the_original() {
    Twins::each(|t| {
        for i in 0..4u32 {
            t.reset();
            let obj = t.alloc(&[0x22; 0x110]);
            t.stubs(&[
                (ff::RT_DYNAMIC_CAST, false, 5, 0, obj),
                (ff::GET_APPLICATION, false, 0, 0, 0x00a9_0000),
                (ff::INPUT_SET_KEY, true, 3, 3, 0x91 + i),
                (ff::CLEAR_KEYS, false, 0, 0, 0x92 + i),
                (c::HANDLERS_ADD_CALL, true, 2, 2, 0x93),
            ]);
            t.stub_swap_state(ff::SWAP_STATE);
            let menu = 0x00ee_0000 + i;
            t.run(
                "pad incomplete",
                0x0070_1e40,
                Conv::Cdecl,
                &[menu],
                true,
                |g| ff::on_pad_incomplete(g, menu),
            );
            t.check_framed(
                "pad capture",
                0x0070_1be0,
                Conv::Cdecl,
                &[menu],
                0xc,
                |g, ebp| Out::U32(ff::begin_pad_capture(g, menu, ebp - 0xc)),
            );
            t.check_framed(
                "key capture",
                0x0070_13e0,
                Conv::Cdecl,
                &[menu],
                0xc,
                |g, ebp| Out::U32(ff::begin_keyboard_capture(g, menu, ebp - 0xc)),
            );
        }
    });
}

// ---- batch 8 (ffx_pcui::m5u_g) ----

#[test]
fn debug_gui_windows_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x6d1e_0008);
        for i in 0..6u32 {
            t.reset();
            t.stubs(&[
                (gg::WINDOW_CREATE, false, 6, 0, 0x00a1_0000 + i),
                (gg::WINDOW_SET_LOCAL_TRANS, false, 3, 0, 0xa2),
                (gg::SET_RESOURCE, false, 2, 0, 0xa3),
                (gg::RCBG_INIT, false, 0, 0, 0xa4),
                (gg::RCUI_INIT, false, 0, 0, 0xa5),
                (gg::RCUI_CLOSE_WINDOW, false, 1, 0, 0xa6),
                (
                    gg::ENABLE_GET_TEXTURE,
                    false,
                    0,
                    0,
                    [0, 1, 2][i as usize % 3],
                ),
                (gg::BUTTON_SET_TOGGLE, false, 3, 0, 0xa7),
                (gg::MAKE_FILE_LIST, false, 3, 0, 0xa8),
                (gg::SELECTOR_RESET, false, 2, 0, 0xa9),
                (gg::SELECTOR_SET_ITEM, false, 4, 0, 0xaa),
            ]);
            let cd = Conv::Cdecl;
            let open = i % 2 == 1;
            let flag = t.rebase(gg::SPRITE_WINDOW_OPEN);
            t.write(flag, &[u8::from(open)]);
            t.run("sprite window", 0x0085_3780, cd, &[], !open, |g| {
                gg::sprite_window_init(g)
            });
            t.g32(gg::MEM_GRAPH_OPEN, u32::from(open));
            t.run("mem graph", 0x0085_4fd0, cd, &[], !open, |g| {
                gg::mem_graph(g)
            });
            t.run("chr info", 0x0085_37b0, cd, &[], true, |g| gg::chr_info(g));
            t.run("clock", 0x0085_3ea0, cd, &[], true, |g| gg::clock_init(g));
            t.g32(gg::RCUI_WORK, if i % 3 == 0 { 0x00b2_0000 } else { 0 });
            t.g32(gg::YN_MAP_SYSTEM, u32::from(i % 2 == 0));
            t.run("yanagi", 0x0093_0d80, cd, &[], true, |g| gg::yanagi_init(g));
            let has = i % 4 != 0;
            t.g32(gg::TEXTURE_BUTTON, if has { 0x00b3_0000 } else { 0 });
            t.run("button texture", 0x0084_c9d0, cd, &[], has, |g| {
                gg::button_set_texture(g)
            });
            // the byte writer
            let buf = t.zeros(0x10);
            t.g32(gg::BYTE_CURSOR, buf + i);
            let c = rng.next();
            t.run("put byte", 0x0085_3400, cd, &[c], false, |g| {
                gg::put_byte(g, c);
                0
            });
            // the selector fill: 0..3 names
            let n = [0, 1, 3, 0xffff_ffff][i as usize % 4];
            let names = t.words(&[rng.next(), rng.next(), rng.next()]);
            let list = t.words(&[n, 0, names]);
            let (sel, dir) = (rng.next(), rng.next());
            t.run(
                "selector files",
                0x0084_d3a0,
                cd,
                &[list, sel, dir],
                false,
                |g| {
                    gg::selector_fill_files(g, list, sel, dir);
                    0
                },
            );
        }
    });
}

#[test]
fn debug_gui_packet_args_match_the_original() {
    Twins::each(|t| {
        for i in 0..8u32 {
            t.reset();
            let strs = ["12", "-7", "x", "", "99999999999"];
            let ptrs: Vec<u32> = strs.iter().map(|s| t.cstr(s)).collect();
            let pk = t.words(&ptrs);
            t.stub(gg::PACKET_GET_PTR, false, 0, 0, pk);
            t.g32(gg::PACKET_ARG_COUNT, [5, 3, 0, 0x8000_0000][i as usize % 4]);
            let k = [0, 1, 2, 4, 5, 0xffff_ffff, 3, 2][i as usize];
            let cd = Conv::Cdecl;
            t.run("packet arg", 0x0085_3140, cd, &[k], true, |g| {
                gg::packet_arg(g, k)
            });
            t.run("packet arg int", 0x0085_3170, cd, &[k], true, |g| {
                gg::packet_arg_int(g, k)
            });
        }
    });
}

// ---- batch 9 (ffx_pcui::m5u_g) ----

#[test]
fn debug_gui_printers_match_the_original() {
    Twins::each(|t| {
        for i in 0..4u32 {
            t.reset();
            t.stubs(&[
                (gg::SET_PRINTF_FUNC, false, 1, 0, 0x00c1_0000 + i),
                (gg::SG_VFPRINTF, false, 3, 0, 0x20 + i),
                (gg::PRINT_SET_XY, false, 2, 0, 0xb1),
                (gg::GU_PRINTF, false, 1, 0, 0xb2),
            ]);
            let buf = t.zeros(0x20);
            t.g32(gg::BYTE_CURSOR, buf + i);
            t.g32(gg::LOG_Y, 0x30 + i);
            let fmt = t.cstr("v=%d s=%s");
            let s = t.cstr(["abc", "", "longer text"][i as usize % 3]);
            let args = [fmt, i.wrapping_mul(0x1234_5677), s];
            // the port reads the arguments from the caller's slots
            let ebp = t.frame_ebp(3);
            for (k, w) in (0u32..).zip(args) {
                Mem::write(&mut t.b, ebp + 8 + 4 * k, w);
            }
            t.skip = 4;
            t.check_framed(
                "line printf",
                0x0085_33b0,
                Conv::Cdecl,
                &args,
                4,
                |g, ebp| Out::U32(gg::line_printf(g, fmt, ebp + 0xc)),
            );
            t.check_framed(
                "log printf",
                0x0085_3e40,
                Conv::Cdecl,
                &args,
                0x404,
                |g, ebp| Out::U32(gg::log_printf(g, fmt, ebp + 0xc, ebp - 0x404)),
            );
            t.skip = 0;
        }
    });
}

#[test]
fn debug_gui_light_file_and_chr_selector_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x119e_0009);
        for i in 0..6u32 {
            t.reset();
            let pk = t.zeros(0x100);
            let user = t.cstr("tidus");
            t.stubs(&[
                (gg::PACKET_GET_PTR, false, 0, 0, pk),
                // one word logged: the first call passes only the format
                (gg::LINE_PRINTF, false, 1, 0, 0xc1),
                (gg::GET_USER_NAME, false, 0, 0, user),
                (gg::PC_WRITE, false, 3, 0, 0xc2 + i),
                (gg::CHR_NB_USE, false, 0, 0, [0, 1, 3][i as usize % 3]),
                (gg::SELECTOR_RESET, false, 2, 0, 0xc3),
                (gg::SELECTOR_SET_ITEM, false, 4, 0, 0xc4),
            ]);
            // Gui_SettingSaveLight prints k bytes (moves the cursor)
            let cursor = t.rebase(gg::BYTE_CURSOR);
            let mut code = vec![0x81, 0x05];
            code.extend(cursor.to_le_bytes());
            code.extend((i % 3).to_le_bytes()); // add dword [cursor], k
            code.extend([0xb8, 0x55, 0, 0, 0, 0xc3]); // mov eax, 0x55; ret
            let at = t.cave_code(&code);
            t.jump(gg::SETTING_SAVE_LIGHT, at);
            t.run(
                "save light file",
                0x0085_3310,
                Conv::Cdecl,
                &[],
                true,
                |g| gg::save_light_file(g),
            );
            // characters: 0..4 records, some used
            let n = rng.below(5);
            let mut recs = vec![0u8; (n as usize).max(1) * 0x880];
            for k in 0..n as usize {
                recs[k * 0x880 + 2] = u8::from(rng.chance(2));
                // a used-looking byte mid-record (a wrong stride would see it)
                recs[k * 0x880 + 0x442] = 1;
                recs[k * 0x880 + 4..k * 0x880 + 8].copy_from_slice(&rng.next().to_le_bytes());
            }
            let arr = t.alloc(&recs);
            t.g32(gg::CHR_ARRAY, arr);
            t.g32(gg::CHR_COUNT, n);
            t.g32(gg::CHR_SELECTOR, 0x00c5_0000 + i);
            t.run("chr selector", 0x0085_3dc0, Conv::Cdecl, &[], true, |g| {
                gg::chr_selector_fill(g)
            });
        }
    });
}

// ---- batch 10 (ffx_pcui::m5u_g) ----

/// The three copies of the angle wrap: inside, above, below, far
/// outside, the bounds, infinities, NaN (quiet); `ST0` bit for bit.
#[test]
fn wrap_angle_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0a1e_0010);
        let pi = std::f32::consts::PI;
        let mut xs = vec![
            0.0f32,
            -0.0,
            1.0,
            pi,
            -pi,
            f32::from_bits(0x4049_0fdc), // just above pi
            3.5,
            -3.5,
            7.0,
            -7.0,
            100.0,
            -1000.0,
            1e6,
            f32::NAN,
        ];
        // exactly the bounds (pi and -pi with their low words zero)
        let hi = f64::from_bits(0x4009_21fb_0000_0000) as f32;
        xs.extend([hi, -hi]);
        for _ in 0..30 {
            xs.push(
                f32::from_bits(rng.next() & 0x447f_ffff) * if rng.chance(2) { -1.0 } else { 1.0 },
            );
        }
        for sym in [0x0084_d330u32, 0x0085_3000, 0x0085_4200] {
            for &x in &xs {
                let at = t.rebase(sym);
                let ra = t.a.call(at, Conv::Cdecl, &[x.to_bits()]).unwrap();
                let port = gg::wrap_angle(&G(&mut t.b, 0), x.to_bits());
                let orig = ra.st0_f32().expect("the original leaves ST0 set");
                assert_eq!(orig.to_bits(), port.to_bits(), "wrap {sym:#x} {x}");
                t.diff("wrap angle");
            }
        }
    });
}

#[test]
fn help_window_matches_the_original() {
    Twins::each(|t| {
        for i in 0..6u32 {
            t.reset();
            t.stubs(&[
                (gg::WINDOW_CREATE, false, 6, 0, 0x00a1_0000 + i),
                (gg::WINDOW_TO_FRONT, false, 1, 0, 0xd1),
                (gg::WINDOW_CLOSE_TREE, false, 1, 0, 0xd2),
            ]);
            t.g32(
                gg::HELP_WINDOW,
                if i % 3 == 0 { 0 } else { 0x00a2_0000 + i },
            );
            let at = t.rebase(gg::HELP_TEXT);
            t.write(at, &[0x5a; 0x40]);
            t.g32(gg::HELP_X, 10 + i);
            t.g32(gg::HELP_Y, 20 + i);
            let text = if i % 2 == 0 {
                0
            } else {
                t.cstr("Press the button to continue")
            };
            t.run(
                "help window",
                0x0085_5360,
                Conv::Cdecl,
                &[text],
                true,
                |g| gg::help_window(g, text),
            );
        }
    });
}

// ---- render-pcui batch 3 (ffx_pcui::m5u_h) ----

#[test]
#[allow(clippy::too_many_lines)] // one case per function
fn rp3_small_functions_match_the_original() {
    // Every branch on synthetic data: the thunks and import wrappers with
    // recording stubs (the imports through their slots), the pointer
    // test on set and clear words, the list walk over 0..4 nodes, the
    // movie player's CRT callbacks on real buffers (the game's
    // MSVCR110), the GDraw and Iggy wrappers with and without their
    // objects.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0b31);
        for case in 0..24u32 {
            let r = rng.word();
            t.stub(hh::WINDOW_CONTROL_SET_TEXT, false, 3, 0, r);
            let w = [rng.word(), rng.word(), rng.word()];
            t.check(
                "Gu_WindowControlSetText_thunk_009411f0",
                0x0094_11f0,
                Conv::Cdecl,
                &w,
                |g| {
                    Out::U32(hh::forward(
                        g,
                        hh::WINDOW_CONTROL_SET_TEXT,
                        host::Conv::Cdecl,
                        &w,
                    ))
                },
            );
            t.stub_import(hh::LEAVE_CRITICAL_SECTION_IAT, false, 1, 1, r ^ 1);
            let cs = rng.word();
            t.check(
                "LeaveCriticalSection_ecx_009453c0",
                0x0094_53c0,
                Conv::Thiscall(cs),
                &[],
                |g| Out::U32(hh::leave(g, cs)),
            );
            let pm = t.zeros(0x2000);
            let at = t.rebase(hh::PAGE_MANAGER);
            t.patch(at, &pm.to_le_bytes());
            t.check(
                "PageManager_unlock_00947ea0",
                0x0094_7ea0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(hh::page_unlock(g)),
            );
            t.stub_import(hh::CREATE_THREAD_IAT, false, 6, 6, r ^ 2);
            let arg = rng.word();
            t.check(
                "Thread_create_00948c10",
                0x0094_8c10,
                Conv::Cdecl,
                &[arg],
                |g| Out::U32(hh::create_thread(g, arg)),
            );
            t.stub(hh::CREATE_THREAD, false, 1, 0, r ^ 3);
            t.check(
                "PageManager_startThread_00947630",
                0x0094_7630,
                Conv::Cdecl,
                &[],
                |g| Out::U32(hh::page_start_thread(g)),
            );
            let f = t.stub_code(0x77, false, 0, 0, r ^ 4);
            t.check(
                "Thread_entry_00948c30",
                0x0094_8c30,
                Conv::Stdcall,
                &[f],
                |g| Out::U32(hh::thread_entry(g, f)),
            );
            // the pointer test
            let data: Vec<u32> = (0..8)
                .map(|_| if rng.chance(2) { 0 } else { rng.next() })
                .collect();
            let data = t.words(&data);
            let arr = t.words(&[0, data]);
            let i = rng.below(8);
            t.check(
                "PtrArray_isSet_00947da0",
                0x0094_7da0,
                Conv::Thiscall(arr),
                &[i],
                |g| Out::U32(hh::is_set(g, arr, i)),
            );
            // the list walk
            let mut next = 0;
            for _ in 0..rng.below(5) {
                let node = t.zeros(0x10);
                t.w32(node + 0xc, next);
                next = node;
            }
            let head = t.zeros(0x20);
            t.w32(head + 0x14, next);
            // the neighbouring word would fault as a node: the walk starts at +0x14
            t.w32(head + 0x10, 0xffff_fff0);
            t.check(
                "List_walkToEnd_00948a80",
                0x0094_8a80,
                Conv::Thiscall(head),
                &[],
                |g| Out::U32(hh::walk_to_end(g, head)),
            );
            // the CRT callbacks
            let n = rng.below(64);
            let src: Vec<u8> = (0..64).map(|_| rng.next() as u8).collect();
            let src = t.alloc(&src);
            let dst = t.zeros(128);
            t.check(
                "PVideoPlaybackWin32_memcpy_00a26fb0",
                0x00a2_6fb0,
                Conv::Cdecl,
                &[dst, src, n],
                |g| Out::U32(hh::crt3(g, hh::MEMCPY_THUNK, dst, src, n)),
            );
            let (d2, s2) = if case % 2 == 0 {
                (dst + 3, dst)
            } else {
                (dst, dst + 5)
            };
            t.check(
                "PVideoPlaybackWin32_memmove_00a26fc0",
                0x00a2_6fc0,
                Conv::Cdecl,
                &[d2, s2, n],
                |g| Out::U32(hh::crt3(g, hh::MEMMOVE_IAT, d2, s2, n)),
            );
            let c = rng.word();
            t.check(
                "PVideoPlaybackWin32_memset_00a26fd0",
                0x00a2_6fd0,
                Conv::Cdecl,
                &[dst, c, n],
                |g| Out::U32(hh::crt3(g, hh::MEMSET_THUNK, dst, c, n)),
            );
            // Iggy
            let player = if case % 3 == 0 { 0 } else { rng.next() | 1 };
            let holder = t.words(&[player]);
            t.stub_import(hh::IGGY_GOTO_FRAME_IAT, false, 3, 3, r ^ 5);
            t.check(
                "IggyPlayer_replay_00a3db30",
                0x00a3_db30,
                Conv::Thiscall(holder),
                &[],
                |g| Out::U32(hh::iggy_replay(g, holder)),
            );
            t.stub_import(hh::IGGY_DESTROY_IAT, false, 1, 1, r ^ 6);
            let x = rng.word();
            t.check(
                "IggyPlayerHolder_destroy_00a3d780",
                0x00a3_d780,
                Conv::Thiscall(holder),
                &[x],
                |g| Out::U32(hh::iggy_destroy(g, holder)),
            );
            let menu = t.zeros(0x80);
            t.w32(menu + 0x6c, holder);
            t.stub(hh::IGGY_REPLAY, true, 0, 0, r ^ 7);
            t.check(
                "IggyMenu_replay",
                0x00a2_8ee0,
                Conv::Thiscall(menu),
                &[],
                |g| Out::U32(hh::iggy_menu_replay(g, menu)),
            );
            // GDraw
            let method = t.stub_code(0x78, false, 3, 3, r ^ 8);
            let mut vt = vec![0; 16];
            vt[15] = method;
            let vt = t.words(&vt);
            let dev = t.words(&[vt, 0]);
            let gd = t.words(&[0, dev]);
            let at = t.rebase(hh::GDRAW);
            t.patch(at, &gd.to_le_bytes());
            let handle = t.words(&[rng.word(), rng.word(), rng.word()]);
            t.check(
                "GDraw_releaseHandle_00a362c0",
                0x00a3_62c0,
                Conv::Cdecl,
                &[handle],
                |g| Out::U32(hh::gdraw_release_handle(g, handle)),
            );
            let key = rng.word();
            let a = t.zeros(0x20);
            t.w32(a + 0x14, if rng.chance(2) { key } else { key ^ 1 });
            t.stub(hh::GDRAW_CALLBACK, false, 2, 0, r ^ 9);
            let x = rng.word();
            t.check(
                "GDraw_callbackIf_00a398e0",
                0x00a3_98e0,
                Conv::Stdcall,
                &[a, key, x],
                |g| Out::U32(hh::callback_if(g, a, key, x)),
            );
            t.stub(hh::GDRAW_RELEASE, true, 0, 0, r ^ 0xa);
            let this = rng.word();
            t.check(
                "GDraw_releaseThis_00a3d3c0",
                0x00a3_d3c0,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(hh::release_this(g, this)),
            );
            t.stub(hh::GDRAW_TEARDOWN, true, 0, 0, r ^ 0xb);
            t.check(
                "GDraw_teardown_thunk_00a3d4d0",
                0x00a3_d4d0,
                Conv::Thiscall(this),
                &[],
                |g| {
                    Out::U32(hh::forward(
                        g,
                        hh::GDRAW_TEARDOWN,
                        host::Conv::Thiscall(this),
                        &[],
                    ))
                },
            );
            let cb = t.stub_code(0x79, true, 1, 1, r ^ 0xc);
            let obj = rng.word();
            let pair = t.words(&[obj, cb]);
            let p = t.words(&[rng.word(), pair]);
            t.check(
                "Callback_invoke_00a3d6d0",
                0x00a3_d6d0,
                Conv::Cdecl,
                &[p],
                |g| Out::U32(hh::invoke(g, p)),
            );
            t.check(
                "AddrOfGlobal01a858cc_00a3d7c0",
                0x00a3_d7c0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(hh::gdraw_block(g)),
            );
            t.reset();
        }
    });
}

// ---- render-pcui batch 4 (ffx_pcui::m5u_i) ----

impl Twins {
    /// A COM-style object whose vtable `+8` is a recording stdcall stub
    /// taking the object (tag `tag`, `eax` `ret`).
    fn com_object(&mut self, tag: u32, ret: u32) -> u32 {
        let release = self.stub_code(tag, false, 1, 1, ret);
        let vt = self.words(&[0, 0, release]);
        self.words(&[vt, 0])
    }

    /// `queueDataCallback`'s stub: writes `block` (or nothing for 0)
    /// through its third word, logs and returns `ret` (`ret 0xc`).
    fn stub_queue(&mut self, block: u32, ret: u32) {
        let log = self.stub_code(0x5b59, true, 3, 3, ret);
        let at = self.cave_next;
        let mut c = vec![0x8b, 0x44, 0x24, 0x0c]; // mov eax, [esp+0xc]
        c.extend([0xc7, 0x00]);
        c.extend(block.to_le_bytes()); // mov dword [eax], block
        c.push(0xe9);
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let code = self.cave_code(&c);
        self.jump(ii::QUEUE_DATA_CALLBACK, code);
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one case per function
fn rp4_functions_match_the_original() {
    // Every branch on synthetic data: owned and borrowed arrays with and
    // without data; the rectangle call (its kept argument slot); both
    // render-thread queues queued or refused, with the renderer's flush
    // and sync results; the keyed release (key 0, matching, not; pool
    // node or not); the bucket fill over 0..3 steps; the page chain of
    // 0..3 pages; the page-bits constructor; the plane copy (no rows, no
    // width, equal and different pitches); the GDraw teardowns with and
    // without objects; the bucket locks over random bit sets; the bucket
    // setup (nonzero divisors); the hash lookup (empty, hit, miss, wrap);
    // the value readers (no value, no out, the class matching or not).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0b41);
        for case in 0..30u32 {
            // the array releases
            for (sym, checked) in [
                (0x00a2_5e50_u32, false),
                (0x00a2_5e80, true),
                (0x00a3_d3d0, true),
            ] {
                let owner = if rng.chance(2) { 0x8000_0000 } else { 0 };
                let data = if rng.chance(3) { 0 } else { rng.next() | 1 };
                let arr = t.words(&[owner | rng.below(9), data]);
                t.stub(ii::PHYRE_FREE, false, 1, 0, 0);
                t.check("array release", sym, Conv::Thiscall(arr), &[], |g| {
                    ii::array_release(g, arr, checked);
                    Out::None
                });
            }
            // the rectangle call
            let rect: Vec<u32> = (0..4).map(|_| rng.word()).collect();
            let rect = t.words(&rect);
            let (a, b) = (rng.word(), rng.word());
            t.stub(ii::GDRAW_RECT, false, 6, 0, rng.word());
            let slot = t.frame_ebp(3) + 8;
            t.w32(slot, rect);
            // the port's callees below the slot it writes
            t.b.set_call_top(Some(slot - 0x200));
            t.check(
                "GDraw_rectCall_00a36040",
                0x00a3_6040,
                Conv::Cdecl,
                &[rect, a, b],
                |g| Out::U32(ii::rect_call(g, slot, rect, a, b)),
            );
            t.b.set_call_top(None);
            assert_eq!(
                Mem::read::<u32>(&t.a, slot),
                Mem::read::<u32>(&t.b, slot),
                "rect slot"
            );
            // the render-thread queues
            let block = t.zeros(8);
            let refused = case % 4 == 0;
            t.stub_queue(
                if refused { 0 } else { block },
                if refused { 0xd } else { 0 },
            );
            let v = if rng.chance(2) { 0 } else { rng.word() };
            let renderer = t.object(&[(0, 0), (0, 0), (0, v)]);
            t.stub(
                ii::SYNC_RENDER,
                true,
                0,
                0,
                if rng.chance(2) { 0 } else { 7 },
            );
            let holder = rng.word();
            t.check_framed(
                "IggyPlayerHolder_queueDestroy_00a3d720",
                0x00a3_d720,
                Conv::Thiscall(holder),
                &[renderer],
                4,
                |g, ebp| Out::U32(ii::queue_destroy(g, ebp - 4, holder, renderer)),
            );
            t.check_framed(
                "IggyPlayer_QueueDraw_00a3d9b0",
                0x00a3_d9b0,
                Conv::Thiscall(holder),
                &[renderer],
                4,
                |g, ebp| Out::U32(ii::queue_draw(g, ebp - 4, holder, renderer)),
            );
            // GDraw
            let gd = t.zeros(0xa00);
            let at = t.rebase(ii::GDRAW);
            t.patch(at, &gd.to_le_bytes());
            let node = t.zeros(0x20);
            let key = rng.word() | 1;
            t.w32(node + 0x14, if rng.chance(2) { key } else { key ^ 2 });
            t.w32(
                node + 0x18,
                if rng.chance(2) {
                    gd + 0x290
                } else {
                    rng.word()
                },
            );
            let key = if case % 5 == 0 { 0 } else { key };
            t.stub(ii::NODE_REQUEUE, false, 1, 0, rng.word());
            t.stub(ii::GDRAW_CALLBACK, false, 2, 0, rng.word());
            let x = rng.word();
            t.check(
                "GDraw_releaseIf_00a398a0",
                0x00a3_98a0,
                Conv::Stdcall,
                &[node, key, x],
                |g| Out::U32(ii::release_if(g, node, key, x)),
            );
            for off in [0x788_u32, 0x794] {
                let o = if rng.chance(3) {
                    0
                } else {
                    t.com_object(0x60 + off, 0)
                };
                t.w32(gd + off, o);
            }
            t.stub(ii::GDRAW_POOL_FREE_ALL, false, 2, 0, 0);
            let stats = rng.word();
            t.check(
                "GDraw_freeResources_00a36300",
                0x00a3_6300,
                Conv::Cdecl,
                &[stats],
                |g| {
                    ii::free_resources(g, stats);
                    Out::None
                },
            );
            for k in 0..0x60 {
                let w = match (0x800 + 4 * k, rng.below(3)) {
                    (o, 0) if (0x880..0x900).contains(&o) => t.com_object(0x70, 0),
                    (o, _) if (0x880..0x900).contains(&o) => 0,
                    _ => rng.next(),
                };
                t.w32(gd + 0x800 + 4 * k, w);
            }
            let w = rng.next() | 1;
            t.w32(gd + 0x980, w);
            t.check(
                "GDraw_releaseSlots_00a3cfc0",
                0x00a3_cfc0,
                Conv::Cdecl,
                &[],
                |g| {
                    ii::release_slots(g);
                    Out::None
                },
            );
            // the page heap
            let heap = t.zeros(0x800);
            let base = rng.below(0x100) * 0x10;
            t.w32(heap + 8, base);
            t.w32(heap + 0x14, base + 0x10 * rng.below(4));
            t.stub(ii::PAGE_BUCKET_FILL, true, 2, 2, 0);
            t.check(
                "PageHeap_fillBuckets_00946100",
                0x0094_6100,
                Conv::Thiscall(heap),
                &[],
                |g| {
                    ii::fill_buckets(g, heap);
                    Out::None
                },
            );
            let mut next = 0;
            for _ in 0..rng.below(4) {
                let p = t.words(&[next, 0]);
                next = p;
            }
            let list = t.words(&[0, next]);
            t.stub(ii::DECOMMIT, false, 2, 0, 0);
            t.stub(ii::RELEASE, false, 2, 0, 0x1000);
            t.check(
                "PageList_free_009470f0",
                0x0094_70f0,
                Conv::Thiscall(list),
                &[],
                |g| {
                    ii::free_pages(g, list);
                    Out::None
                },
            );
            // dirty: the memset must clear all 0x400 bytes
            let dirty: Vec<u8> = (0..0x500).map(|_| rng.next() as u8 | 1).collect();
            let bits = t.alloc(&dirty);
            t.stub(ii::PAGE_BITS_INIT, true, 2, 2, 0);
            let (owner, n) = (rng.word(), rng.word());
            t.check(
                "PageBits_ctor_00946520",
                0x0094_6520,
                Conv::Thiscall(bits),
                &[owner, n],
                |g| Out::U32(ii::page_bits_ctor(g, bits, owner, n)),
            );
            let set: Vec<u32> = (0..8)
                .map(|_| if rng.chance(2) { 0 } else { rng.next() })
                .collect();
            let set = t.words(&set);
            t.w32(heap + 0x40c, set);
            for k in 0..0x100 {
                let b = rng.next();
                t.w32(heap + 4 + 4 * k, b);
            }
            t.stub_import(ii::ENTER_CRITICAL_SECTION_IAT, false, 1, 1, 0);
            t.stub_import(ii::LEAVE_CRITICAL_SECTION_IAT, false, 1, 1, 0);
            t.check(
                "PageHeap_lockAll_00946c40",
                0x0094_6c40,
                Conv::Thiscall(heap),
                &[],
                |g| Out::U32(ii::lock_all(g, heap, ii::ENTER_CRITICAL_SECTION_IAT)),
            );
            t.check(
                "PageHeap_unlockAll_00946cc0",
                0x0094_6cc0,
                Conv::Thiscall(heap),
                &[],
                |g| Out::U32(ii::lock_all(g, heap, ii::LEAVE_CRITICAL_SECTION_IAT)),
            );
            let bucket = t.zeros(0x20);
            let size = rng.below(0x400);
            let align = [1u32, 4, 8, 16, 3][rng.below(5) as usize];
            let room = t.zeros(0x20);
            // a size of 0 gives a rounded size of 0 for align > 1: a divide fault
            let size = if size < 2 { 2 } else { size };
            // half the time exactly the header plus whole buckets: the
            // quotient's edge
            let rounded = ((size - 1) / align + 1) * align;
            let header = (15 / align + 1) * align;
            let space = if rng.chance(2) {
                header + rounded * rng.below(8)
            } else {
                rng.below(0x10000)
            };
            t.w32(room + 0x18, space);
            t.check(
                "PageBucket_init_00948900",
                0x0094_8900,
                Conv::Thiscall(bucket),
                &[size, align, room],
                |g| Out::U32(ii::bucket_init(g, bucket, size, align, room)),
            );
            // the hash lookup
            let size = [4u32, 8, 16][(case % 3) as usize];
            let slots = if case % 6 == 0 {
                0
            } else {
                t.zeros(4 * size as usize)
            };
            let mut keys = vec![];
            if slots != 0 {
                for i in 0..size {
                    if rng.chance(2) {
                        let k = rng.below(4);
                        let e = t.words(&[k, 0, rng.next()]);
                        t.w32(slots + 4 * i, e);
                        keys.push(k);
                    }
                }
                // one empty slot at least: the probe stops there
                let hole = rng.below(size);
                t.w32(slots + 4 * hole, 0);
            }
            let table = t.words(&[size, slots]);
            // several keys per table: hits, misses and wrapped probes
            for _ in 0..6 {
                let key = t.words(&[rng.below(4), rng.next()]);
                let out = t.zeros(4);
                t.check(
                    "PageHash_find_009455d0",
                    0x0094_55d0,
                    Conv::Thiscall(table),
                    &[key, out],
                    |g| Out::U8(u8::from(ii::hash_find(g, table, key, out))),
                );
            }
            // the value readers
            for (sym, class_call) in [(0x00a2_5860_u32, true), (0x00a2_58c0, false)] {
                let f = t.stub_code(0x80, true, 1, 1, rng.word());
                let reader = t.words(&[0, f]);
                let v = if rng.chance(5) { 0 } else { rng.next() | 1 };
                let src = t.words(&[rng.next(), rng.next()]);
                let cursor = t.words(&[src]);
                let out = if rng.chance(5) { 0 } else { t.zeros(8) };
                let desc = rng.next() & !1;
                t.stub(ii::GET_CLASS_DESCRIPTOR, false, 0, 0, desc);
                let want = if class_call {
                    desc
                } else {
                    t.rebase(ii::VALUE_CLASS)
                } | 1;
                let class = if rng.chance(3) { rng.word() } else { want };
                t.check(
                    "IggyValue read",
                    sym,
                    Conv::Thiscall(reader),
                    &[v, class, out, cursor],
                    |g| Out::U32(ii::value_read(g, reader, v, class, out, cursor, class_call)),
                );
            }
            t.reset();
        }
        // the plane copy on real buffers (the game's memcpy)
        for case in 0..40u32 {
            let rows = [0u32, 1, 3, 5][(case % 4) as usize];
            let w = [0u32, 1, 7, 16][((case / 4) % 4) as usize];
            let spitch = 16 + 4 * rng.below(3);
            let dpitch = if case % 3 == 0 {
                spitch
            } else {
                16 + 4 * rng.below(3)
            };
            let src: Vec<u8> = (0..spitch * 6).map(|_| rng.next() as u8).collect();
            let src = t.alloc(&src);
            let dst = t.zeros((dpitch * 6 + spitch * 6) as usize);
            t.check(
                "PVideoPlaybackWin32_CopyPlane_00a26fe0",
                0x00a2_6fe0,
                Conv::Cdecl,
                &[dst, dpitch, src, spitch, w, rows],
                |g| {
                    ii::copy_plane(g, dst, dpitch, src, spitch, w, rows);
                    Out::None
                },
            );
        }
    });
}

// ---- render-pcui batch 5 (ffx_pcui::m5u_j) ----

#[test]
fn rp5_functions_match_the_original() {
    // Every branch on synthetic data: the selector handler on other
    // messages and on 0x8011 with and without a chosen item; the utility
    // destructor with an empty list, the utility first, later or absent,
    // with and without the delete flag; the locked call, the lock
    // constructor and the statistics pass with and without a page
    // manager (the imports and callees stubbed).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0b51);
        for case in 0..30u32 {
            let item = if case % 3 == 0 {
                0
            } else {
                t.words(&[0, rng.next()])
            };
            t.stub(jj::SELECTOR_GET_ITEM_VALUE, false, 1, 0, item);
            t.stub(jj::WINDOW_SET_ARG, false, 2, 0, 0);
            t.stub(jj::WINDOW_CONTROL_SET_TEXT, false, 3, 0, 0);
            t.stub(jj::WINDOW_SET_WORD30, false, 2, 0, 0);
            for (sym, v) in [(jj::CHR_SELECTOR, rng.word()), (jj::CHR_WINDOW, rng.word())] {
                let at = t.rebase(sym);
                t.patch(at, &v.to_le_bytes());
            }
            let window = rng.word();
            let message = if case % 4 == 0 { rng.word() } else { 0x8011 };
            t.check(
                "Gu_ChrSelectorProc_00853d40",
                0x0085_3d40,
                Conv::Cdecl,
                &[window, message],
                |g| Out::U32(jj::chr_selector_proc(g, window, message)),
            );
            // the utility list
            let n = rng.below(4);
            let mut nodes = vec![];
            for _ in 0..n {
                nodes.push(t.zeros(8));
            }
            let this = match case % 4 {
                0 => t.zeros(8),
                _ if n > 0 => nodes[rng.below(n) as usize],
                _ => t.zeros(8),
            };
            for k in 0..n as usize {
                let next = nodes.get(k + 1).copied().unwrap_or(0);
                t.w32(nodes[k] + 4, next);
            }
            let w = rng.next();
            t.w32(this, w);
            let head = t.rebase(jj::UTILITY_LIST);
            let first = nodes.first().copied().unwrap_or(0);
            t.patch(head, &first.to_le_bytes());
            t.stub(jj::OPERATOR_DELETE, false, 1, 0, 0);
            let flags = rng.word();
            t.check(
                "PUtility_scalarDeletingDtor_00a3e3e0",
                0x00a3_e3e0,
                Conv::Thiscall(this),
                &[flags],
                |g| Out::U32(jj::utility_delete(g, this, flags)),
            );
            // the page heap pieces
            t.stub_import(jj::ENTER_CRITICAL_SECTION_IAT, false, 1, 1, rng.word());
            t.stub_import(jj::LEAVE_CRITICAL_SECTION_IAT, false, 1, 1, rng.word());
            t.stub_import(jj::INIT_CRITICAL_SECTION_IAT, false, 1, 1, rng.word());
            t.stub(jj::BUCKET_CALL, true, 1, 1, rng.word());
            t.stub(jj::STORE_COMPUTED, true, 1, 1, rng.word());
            let bucket = t.words(&[0, rng.word(), 0, 0, rng.word()]);
            t.check(
                "PageBucket_lockedCall_00948820",
                0x0094_8820,
                Conv::Thiscall(bucket),
                &[],
                |g| Out::U32(jj::locked_call(g, bucket)),
            );
            let lock = rng.word();
            t.check(
                "PageLock_ctor_009488a0",
                0x0094_88a0,
                Conv::Thiscall(lock),
                &[],
                |g| Out::U32(jj::lock_ctor(g, lock)),
            );
            t.stub(jj::COMMITTED_BYTES, false, 0, 0, rng.word());
            t.stub(jj::RESERVED_BYTES, false, 0, 0, rng.word());
            t.stub(jj::PAGE_MANAGER_STATS, true, 0, 0, rng.word());
            let pm = if case % 3 == 0 { 0 } else { rng.word() | 1 };
            let at = t.rebase(jj::PAGE_MANAGER);
            t.patch(at, &pm.to_le_bytes());
            t.check(
                "PageStats_update_00947cc0",
                0x0094_7cc0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(jj::stats_update(g)),
            );
            t.reset();
        }
    });
}

// ---- render-pcui batch 6 (ffx_pcui::m5u_k) ----

#[test]
fn rp6_page_hash_matches_the_original() {
    // The bucket table's thirteen fills; the hash constructor without a
    // count, with small counts (the minimum of 2 slots) and larger ones
    // (one or more doublings, tables of one and several pages; the
    // reserve stub returns a real buffer for the memset); the release
    // with 0..3 overflow pages.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0b61);
        for case in 0..30u32 {
            let heap = t.zeros(0x100);
            t.stub(kk::PAGE_BUCKET_FILL, true, 2, 2, rng.word());
            t.check(
                "PageHeap_initBuckets_00946060",
                0x0094_6060,
                Conv::Thiscall(heap),
                &[],
                |g| Out::U32(kk::init_buckets(g, heap)),
            );
            let this = t.words(&[rng.next(); 8]);
            let n = match case % 5 {
                0 => 0,
                1 => rng.below(2),
                2 => rng.below(8),
                // 13 * 256 / 102 is 32 exactly: the doubling stops there
                3 if case % 2 == 0 => 13,
                3 => rng.below(200),
                _ => 1000 + rng.below(3000),
            };
            // dirty: the memset clears the slots, not the whole pages
            let dirty: Vec<u8> = (0..0x1_1000).map(|_| rng.next() as u8 | 1).collect();
            let table = t.alloc(&dirty);
            t.stub(kk::RESERVE, false, 1, 0, table);
            t.stub(kk::COMMIT, false, 2, 0, rng.word());
            let owner = rng.word();
            t.check(
                "PageHash_ctor_00945630",
                0x0094_5630,
                Conv::Thiscall(this),
                &[n, owner],
                |g| Out::U32(kk::hash_ctor(g, this, n, owner)),
            );
            let mut next = 0;
            for _ in 0..rng.below(4) {
                next = t.words(&[next, 0]);
            }
            let hash = t.words(&[
                rng.below(0x2000),
                rng.next(),
                rng.next(),
                0,
                next,
                rng.next(),
                rng.next(),
            ]);
            t.stub(kk::DECOMMIT, false, 2, 0, 0);
            t.stub(kk::RELEASE, false, 2, 0, 0x1000);
            t.check(
                "PageHash_release_009474f0",
                0x0094_74f0,
                Conv::Thiscall(hash),
                &[],
                |g| {
                    kk::hash_release(g, hash);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

// ---- round 6 (render-pcui4, ffx_pcui::m5u_l) ----

#[test]
fn rp4_crash_read_and_video_release_match_the_original() {
    // The memory reader with and without an override (ReadProcessMemory
    // stubbed; the count cell in the frame). The video texture release
    // with every slot set or empty.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_4081);
        for case in 0..8 {
            let hook = if case % 2 == 0 {
                0
            } else {
                t.stub_code(0x4801, false, 7, 0, rng.next())
            };
            t.g32(ll::READ_MEMORY_HOOK, hook);
            t.g32(ll::READ_MEMORY_CONTEXT, rng.next());
            // ReadProcessMemory writes the count through its fifth word
            let n = rng.next();
            let mut c = vec![0x8b, 0x44, 0x24, 0x14, 0xc7, 0x00];
            c.extend(n.to_le_bytes()); // mov eax, [esp+0x14]; mov [eax], n
            c.push(0xb8);
            c.extend(rng.next().to_le_bytes()); // mov eax, result
            c.extend([0xc2, 0x14, 0x00]);
            let rpm = t.cave_code(&c);
            let slot = t.rebase(ll::READ_PROCESS_MEMORY);
            t.patch(slot, &rpm.to_le_bytes());
            let read = t.zeros(4);
            let args = [
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                read,
            ];
            // with the override the cell is only the original's frame slot
            t.skip = if hook == 0 { 0 } else { 4 };
            t.check_framed(
                "CrashDump_readMemory_00689370",
                0x0068_9370,
                Conv::Stdcall,
                &args,
                4,
                |m, ebp| Out::U32(ll::crash_read_memory(m, ebp - 4, args)),
            );
            t.skip = 0;
            t.reset();
        }
        for _ in 0..20 {
            let this = t.zeros(0x80);
            for off in (0x18..0x6c).step_by(4) {
                let v = if rng.chance(3) { 0 } else { rng.next() };
                t.w32(this + off, v);
            }
            let scene = t.zeros(0x1300);
            t.g32(ll::SCENE, scene);
            t.stubs(&[
                (ll::CLUSTER_MANAGER, false, 0, 0, rng.next()),
                (ll::RELEASE_CLUSTER, true, 1, 1, rng.next()),
                (ll::DELETE_MATERIAL, true, 4, 4, rng.next()),
                (ll::RELEASE_POOL_ARRAY, true, 2, 2, rng.next()),
                (ll::DELETE_POOL_OBJECT56, true, 2, 2, rng.next()),
                (ll::DELETE_MESH_INSTANCE, true, 2, 2, rng.next()),
            ]);
            t.check(
                "VideoTexture_release_006e7d30",
                0x006e_7d30,
                Conv::Thiscall(this),
                &[],
                |m| Out::U32(ll::video_texture_release(m, this)),
            );
            t.reset();
        }
    });
}

impl Twins {
    fn w8(&mut self, at: u32, v: u8) {
        self.write(at, &[v]);
    }

    /// `getDisplayModeList(this; count, buf)` (`ret 8`): without a buffer
    /// `*count = n`, else the `n` 0x1c-byte modes of `table` copied in.
    fn stub_display_modes(&mut self, table: u32, n: u32) {
        let mut c = vec![
            0x8b, 0x44, 0x24, 0x04, 0x8b, 0x54, 0x24, 0x08, 0x85, 0xd2, 0x75, 0x09,
        ];
        c.extend([0xc7, 0x00]);
        c.extend(n.to_le_bytes());
        c.extend([0xc2, 0x08, 0x00, 0x56, 0x57, 0xbe]);
        c.extend(table.to_le_bytes());
        c.extend([0x89, 0xd7, 0xb9]);
        c.extend((n * 7).to_le_bytes());
        c.extend([0xf3, 0xa5, 0x5f, 0x5e, 0xc2, 0x08, 0x00]);
        let addr = self.cave_code(&c);
        self.jump(ll::DISPLAY_MODES, addr);
    }
}

#[test]
#[allow(clippy::too_many_lines)] // three functions
fn rp4_video_menu_matches_the_original() {
    // The resolution list: 0..12 modes with duplicate sizes and every
    // order, the render interface made or not, the settings' resolution
    // listed or not. The apply: every choice changed or not, the screen
    // mode 1 or another (SetWordAt as it is). The key binding prompt:
    // escape held and released, a key held and released, no key, the help
    // text shown or not.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_4082);
        for _ in 0..40 {
            let n = rng.below(13);
            let sizes = [
                (640, 480),
                (800, 600),
                (1280, 720),
                (1920, 1080),
                (1280, 1024),
            ];
            let mut modes = vec![];
            for _ in 0..n {
                let (w, h) = sizes[rng.below(sizes.len() as u32) as usize];
                modes.extend([
                    w,
                    h,
                    rng.next(),
                    rng.next(),
                    rng.next(),
                    rng.next(),
                    rng.next(),
                ]);
            }
            modes.extend([0; 7]);
            let table = t.words(&modes);
            t.stub_display_modes(table, n);
            t.stub_alloc(ll::NEW_ARRAY, 99);
            t.stubs(&[
                (ll::SCOPE_LOCK_ENTER, true, 0, 0, rng.next()),
                (ll::SCOPE_LOCK_LEAVE, true, 0, 0, rng.next()),
                (ll::RENDER_INTERFACE_CTOR, true, 0, 0, rng.next()),
                (ll::ATEXIT, false, 1, 0, 0),
                (ll::DELETE_ARRAY, false, 1, 0, 0),
            ]);
            let guard = t.rebase(ll::RENDER_INTERFACE_GUARD);
            let g: u32 = Mem::read(&t.b, guard);
            let g = (g & !1) | rng.below(2);
            t.g32(ll::RENDER_INTERFACE_GUARD, g);
            let settings = t.zeros(0x80);
            let (w, h) = sizes[rng.below(sizes.len() as u32) as usize];
            t.w32(settings + 4, w << 16 | h);
            t.g32(ll::SETTINGS, settings);
            let this = t.zeros(0x100);
            t.w32(this + 0xd8, rng.next());
            // the lock object only: the rest are the original's spills
            t.skip = 0x20;
            t.check_framed(
                "FFEscMenu_buildResolutionList",
                0x006f_edf0,
                Conv::Thiscall(this),
                &[],
                0x24,
                |m, ebp| {
                    ll::build_resolution_list(m, ebp - 0x24, this);
                    Out::None
                },
            );
            t.skip = 0;
            t.reset();
        }
        for case in 0..40 {
            let this = t.zeros(0x100);
            let modes = t.zeros(0x1c * 4);
            for k in 0..8 {
                t.w32(modes + 4 * k, rng.next());
            }
            t.w32(this + 0xdc, modes);
            t.w32(this + 0xd8, rng.below(3));
            t.w32(this + 0xa8, rng.below(3));
            let settings = t.zeros(0x80);
            t.w32(settings + 8, if case % 3 == 0 { 1 } else { rng.below(3) });
            t.g32(ll::SETTINGS, settings);
            let app = t.zeros(0x40);
            t.stubs(&[
                (ll::GET_APPLICATION, false, 0, 0, app),
                (ll::APPLY_SCREEN_MODE, false, 0, 0, rng.next()),
                (ll::APPLY_SETTINGS, true, 0, 0, rng.next()),
                (ll::DETECT_QUALITY, true, 0, 0, rng.next()),
                (ll::DELETE_ARRAY, false, 1, 0, rng.next()),
            ]);
            let mut s = vec![rng.below(3), rng.below(3)];
            for k in 2..12 {
                s.push(match k {
                    4 => rng.below(3),
                    6 | 7 => rng.below(4),
                    _ => rng.next(),
                });
            }
            let rec = t.words(&s);
            t.check(
                "FFEscMenu_applyVideoSettings",
                0x006f_f290,
                Conv::Thiscall(this),
                &[rec],
                |m| Out::U32(ll::apply_video_settings(m, this, rec)),
            );
            t.reset();
        }
        for case in 0..60 {
            let this = t.zeros(0x100);
            t.w8(this + 0xfc, u8::from(rng.chance(2)));
            let k = match case % 4 {
                0 => 0x40,
                1 => u32::MAX,
                _ => rng.below(8),
            };
            t.w32(this + 0xf8, k);
            let app = t.zeros(0x40);
            let held: Vec<u32> = (0..20).map(|_| u32::from(rng.chance(3)) | 0x100).collect();
            let names: Vec<u32> = (0..20).map(|_| u32::from(!rng.chance(3))).collect();
            t.stub_seq(ll::KEY_HELD, true, 2, 2, &held);
            t.stub_seq(ll::HAS_KEY_NAME, false, 1, 0, &names);
            t.stubs(&[
                (ll::GET_APPLICATION, false, 0, 0, app),
                (ll::LOC_STRING_LOAD, true, 1, 1, rng.next()),
                (ll::LOC_STRING_TO_VALUE, true, 1, 1, rng.next()),
                (ll::CALL_FLASH, false, 6, 0, rng.next()),
                (ll::INPUT_SWAP_STATE, false, 4, 0, rng.next()),
                (ll::CALL_AS3, true, 5, 5, rng.next()),
                (ll::KEY_NAME, false, 1, 0, rng.next()),
                (ll::IGGY_SET_UTF8, false, 2, 0, rng.next()),
                (ll::RESET_INPUT_STATE, true, 0, 0, rng.next()),
                (ll::KEY_SLOTS, false, 0, 0, 8),
            ]);
            let done = t.zeros(4);
            t.skip = 4;
            t.check_framed(
                "FFEscMenu_keyBindingInput_00701190",
                0x0070_1190,
                Conv::Thiscall(this),
                &[done],
                0x124,
                |m, ebp| {
                    ll::key_binding_input(m, ebp - 0x124, this, done);
                    Out::None
                },
            );
            t.skip = 0;
            t.reset();
        }
    });
}

impl Twins {
    /// `IggyValue_getStringUTF8(ref; i, buf, size)` (`ret 0xc`): copies
    /// entry `i` of `texts` (28 bytes) into `buf` and returns its length.
    fn stub_texts(&mut self, texts: &[String]) {
        let mut t = vec![];
        for s in texts {
            let mut b = s.as_bytes().to_vec();
            b.resize(28, 0);
            t.extend(b);
            t.extend((s.len() as u32).to_le_bytes());
        }
        let table = self.alloc(&t);
        let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0xc1, 0xe0, 0x05, 0x05];
        c.extend(table.to_le_bytes());
        c.extend([
            0x56, 0x57, 0x89, 0xc6, 0x8b, 0x7c, 0x24, 0x10, 0xb9, 7, 0, 0, 0,
        ]);
        c.extend([0xf3, 0xa5, 0x8b, 0x06, 0x5f, 0x5e, 0xc2, 0x0c, 0x00]);
        let addr = self.cave_code(&c);
        self.jump(ll::VALUE_GET_UTF8, addr);
    }

    /// A name table stub at `sym` (`cdecl(i)` -> the name of slot `i`)
    /// and a "has a name" stub at `has` (slots below 6).
    fn stub_names(&mut self, sym: u32, has: u32, names: &[&str]) {
        let ptrs: Vec<u32> = names.iter().map(|s| self.cstr(s)).collect();
        let table = self.words(&ptrs);
        let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x04, 0x85];
        c.extend(table.to_le_bytes());
        c.push(0xc3);
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
        let c = [
            0x8b, 0x44, 0x24, 0x04, 0x83, 0xf8, 0x06, 0x0f, 0x92, 0xc0, 0xc3,
        ];
        let addr = self.cave_code(&c);
        self.jump(has, addr);
    }
}

#[test]
#[allow(clippy::too_many_lines)] // three functions
fn rp4_binding_pages_match_the_original() {
    // The keyboard and controller binding loads: each record's text a
    // known name, an unknown one, two names or one (keyboard), the menu
    // calls stubbed, strchr as it is. The controller prompt as the
    // keyboard one's test.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_4091);
        let names = ["A", "Bb", "Ccc", "D", "Esc", "F1", "G", "H"];
        for round in 0..6 {
            let pick = |rng: &mut Rng| {
                if rng.chance(5) {
                    "Zz".to_string()
                } else {
                    names[rng.below(8) as usize].to_string()
                }
            };
            let texts: Vec<String> = (0..23)
                .map(|_| {
                    let a = pick(&mut rng);
                    if rng.chance(3) {
                        format!("{a}\n")
                    } else {
                        format!("{a}\n{}", pick(&mut rng))
                    }
                })
                .collect();
            t.stub_texts(&texts);
            t.stub_names(ll::KEY_NAME, ll::HAS_KEY_NAME, &names);
            let menu = t.zeros(0x80);
            let obj = t.zeros(0x40);
            t.stub_alloc(ll::OPERATOR_NEW, 99);
            t.stubs(&[
                (ll::MENU_CONTEXT, true, 0, 0, rng.next()),
                (ll::VALUE_REF_CREATE, true, 2, 2, rng.next()),
                (ll::KEY_SLOTS, false, 0, 0, if round == 5 { 0 } else { 8 }),
                (ll::DYNAMIC_CAST, false, 5, 0, obj),
                (ll::ADD_CALL, true, 2, 2, rng.next()),
                (ll::INPUT_MANAGER, false, 0, 0, rng.next()),
                (ll::BUILD_PAD_MAP, true, 0, 0, rng.next()),
                (ll::VALUE_REF_FREE, true, 0, 0, rng.next()),
            ]);
            let settings = t.zeros(0x400);
            t.g32(ll::SETTINGS, settings);
            let args = t.words(&[0, rng.next()]);
            t.skip = 0xa0; // the stubs write the frame alike; the rest are spills
            t.check_framed(
                "FFEscMenu_loadKeyBindings_00701680",
                0x0070_1680,
                Conv::Cdecl,
                &[menu, args],
                0xa0,
                |m, ebp| Out::U32(ll::load_key_bindings(m, ebp - 0xa0, menu, args)),
            );
            t.skip = 0;
            t.reset();
        }
        for round in 0..6 {
            let texts: Vec<String> = (0..23)
                .map(|_| {
                    if rng.chance(5) {
                        "Zz".to_string()
                    } else {
                        names[rng.below(8) as usize].to_string()
                    }
                })
                .collect();
            t.stub_texts(&texts);
            t.stub_names(ll::PAD_NAME, ll::HAS_PAD_NAME, &names);
            let menu = t.zeros(0x80);
            t.stubs(&[
                (ll::MENU_CONTEXT, true, 0, 0, rng.next()),
                (ll::VALUE_REF_CREATE, true, 2, 2, rng.next()),
                (ll::PAD_SLOTS, false, 0, 0, if round == 5 { 0 } else { 8 }),
                (ll::INPUT_MANAGER, false, 0, 0, rng.next()),
                (ll::BUILD_PAD_MAP, true, 0, 0, rng.next()),
                (ll::VALUE_REF_FREE, true, 0, 0, rng.next()),
            ]);
            let settings = t.zeros(0x400);
            t.g32(ll::SETTINGS, settings);
            let args = t.words(&[0, rng.next()]);
            t.skip = 0x94;
            t.check_framed(
                "FFEscMenu_loadPadBindings_00701e60",
                0x0070_1e60,
                Conv::Cdecl,
                &[menu, args],
                0x94,
                |m, ebp| Out::U32(ll::load_pad_bindings(m, ebp - 0x94, menu, args)),
            );
            t.skip = 0;
            t.reset();
        }
        for case in 0..60 {
            let this = t.zeros(0x100);
            t.w8(this + 0xfc, u8::from(rng.chance(2)));
            let k = match case % 4 {
                0 => 0x40,
                1 => u32::MAX,
                _ => rng.below(8),
            };
            t.w32(this + 0xf8, k);
            let app = t.zeros(0x40);
            let held: Vec<u32> = (0..20).map(|_| u32::from(rng.chance(3)) | 0x100).collect();
            let down: Vec<u32> = (0..20).map(|_| u32::from(rng.chance(3)) | 0x200).collect();
            let names: Vec<u32> = (0..20).map(|_| u32::from(!rng.chance(3))).collect();
            t.stub_seq(ll::KEY_HELD, true, 2, 2, &held);
            t.stub_seq(ll::KEY_DOWN, true, 2, 2, &down);
            t.stub_seq(ll::HAS_PAD_NAME, false, 1, 0, &names);
            t.stubs(&[
                (ll::GET_APPLICATION, false, 0, 0, app),
                (ll::LOC_STRING_LOAD, true, 1, 1, rng.next()),
                (ll::LOC_STRING_TO_VALUE, true, 1, 1, rng.next()),
                (ll::CALL_FLASH, false, 6, 0, rng.next()),
                (ll::CALL_AS3, true, 5, 5, rng.next()),
                (ll::PAD_NAME, false, 1, 0, rng.next()),
                (ll::IGGY_SET_UTF8, false, 2, 0, rng.next()),
                (ll::RESET_INPUT_STATE, true, 0, 0, rng.next()),
                (ll::PAD_SLOTS, false, 0, 0, 8),
            ]);
            let done = t.zeros(4);
            t.skip = 4;
            t.check_framed(
                "FFEscMenu_padBindingInput_007019b0",
                0x0070_19b0,
                Conv::Thiscall(this),
                &[done],
                0x124,
                |m, ebp| {
                    ll::pad_binding_input(m, ebp - 0x124, this, done);
                    Out::None
                },
            );
            t.skip = 0;
            t.reset();
        }
    });
}

impl Twins {
    /// `sceIoctl(fd, cmd, out)` (`cdecl`): logs its three words and writes
    /// the next value of `seq` to `*out`, returning 0.
    fn stub_ioctl(&mut self, seq: &[u32]) {
        let ctr = self.words(&[0]);
        let tbl = self.words(seq);
        let log = self.stub_code(ll::SCE_IOCTL, false, 3, 0, 0);
        let mut c = vec![0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x15];
        c.extend(ctr.to_le_bytes());
        c.extend([0x8b, 0x14, 0x95]);
        c.extend(tbl.to_le_bytes());
        c.extend([0xff, 0x05]);
        c.extend(ctr.to_le_bytes());
        c.extend([0x89, 0x10, 0xe9]);
        let at = self.cave_next;
        let end = at + c.len() as u32 + 4;
        c.extend(log.wrapping_sub(end).to_le_bytes());
        let addr = self.cave_code(&c);
        self.jump(ll::SCE_IOCTL, addr);
    }
}

#[test]
#[allow(clippy::too_many_lines)] // five functions
fn rp4_movie_sync_and_debug_print_match_the_original() {
    // The stream sync: no stream, done at once, a few frames' wait,
    // playing or not. The debug print character: a newline, characters,
    // bold, italic. The clock reset. The debug script reader and
    // tokenizer on scripts of empty lines, carriage returns, ';'
    // separators and the end mid-line (strtok as it is).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_40a1);
        for case in 0..12 {
            let state = t.zeros(0x5_0600);
            let fd = if case % 4 == 0 {
                0x8000_0000
            } else {
                rng.below(8)
            };
            t.w32(state + 0x5_0544, fd);
            let waits = rng.below(4);
            let mut seq: Vec<u32> = (0..waits).map(|_| 1 + rng.below(9)).collect();
            seq.push(0);
            t.stub_ioctl(&seq);
            t.stubs(&[
                // one word: the others are stack (the frame count of the
                // last message is the waits seen above)
                (ll::DBG_PRINTF, false, 1, 0, rng.next()),
                (ll::MOVIE_WAIT, false, 0, 0, rng.next()),
                (ll::AUDIO_STOP, false, 1, 0, rng.next()),
                (ll::MOVIE_CSC, false, 0, 0, rng.next()),
                (ll::PAD_PRINT_FLAG, false, 0, 0, rng.next()),
            ]);
            let play = rng.below(2);
            t.check_framed(
                "syncStrPc",
                0x0093_db20,
                Conv::Cdecl,
                &[play, state],
                4,
                |m, ebp| Out::U32(ll::sync_str_pc(m, ebp - 4, play, state)),
            );
            t.reset();
        }
        for case in 0..30 {
            for (sym, v) in [
                (ll::GU_BASE_X, rng.below(100)),
                (ll::GU_X, rng.below(600)),
                (ll::GU_Y, rng.below(400)),
                (ll::GU_ADVANCE, rng.below(12)),
                (ll::GU_NEXT_LINE, rng.below(20)),
                (ll::GU_LINE, rng.below(20)),
                (ll::GU_TEX_A, rng.next()),
                (ll::GU_TEX_B, rng.next()),
                (ll::GU_ITALIC, rng.below(2)),
                (ll::GU_SLANT, rng.next()),
                (ll::GU_BOLD, rng.below(2)),
                (ll::GU_BOLD_OFFSET, rng.below(3)),
                (ll::GU_COUNT, rng.below(100)),
            ] {
                t.g32(sym, v);
            }
            t.stub(ll::GU_DRAW_CHAR, false, 9, 0, rng.next());
            let c = if case % 5 == 0 {
                0xa
            } else {
                0x20 + rng.below(0x60)
            };
            t.check(
                "GuPrintChar_00849070",
                0x0084_9070,
                Conv::Cdecl,
                &[c],
                |m| Out::U32(ll::gu_print_char(m, c)),
            );
            t.reset();
        }
        t.check(
            "SgClock_resetState_0084ca00",
            0x0084_ca00,
            Conv::Cdecl,
            &[],
            |m| Out::U32(ll::sg_clock_reset_state(m)),
        );
        let scripts = [
            "warp 1 2\nsay hi;x y\r\n\n;;z",
            "\n\n\r\n",
            "",
            "one two three four five six",
            "a\tb\rc;d",
        ];
        for s in scripts {
            let buf = t.cstr(s);
            for tokenize in [false, true] {
                t.g32(ll::DEBUG_CURSOR, buf);
                t.g32(ll::DEBUG_END, buf + s.len() as u32);
                let packet = t.zeros(0x40);
                t.stub(ll::PACKET_PTR, false, 0, 0, packet);
                for _ in 0..4 {
                    if tokenize {
                        t.check(
                            "DebugScript_tokenizeLine_00853070",
                            0x0085_3070,
                            Conv::Cdecl,
                            &[],
                            |m| Out::U32(ll::debug_tokenize_line(m)),
                        );
                    } else {
                        t.check(
                            "DebugScript_readLine_008530d0",
                            0x0085_30d0,
                            Conv::Cdecl,
                            &[],
                            |m| Out::U32(ll::debug_read_line(m)),
                        );
                    }
                }
                t.reset();
            }
        }
    });
}

#[test]
fn rp4_debug_windows_and_light_script_match_the_original() {
    // Gui_MasakiInit for every index (open or not) and past the table. The
    // light reader on LGT lines and others, every light kind, groups and
    // lights at and past the limits (negative too). The interpreter on
    // scripts with VERSION, #LIGHT n, #skip n and other lines (the line
    // reader, tokenizer and atoi as they are; the light reader stubbed).
    // The loader with and without the file.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_40b1);
        for which in 0..12_u32 {
            for open in [0, 1] {
                if let Some(&(flag, _, _)) = ll::MASAKI_WINDOWS.get(which as usize) {
                    t.g32(flag, open);
                }
                t.stub(ll::GU_WINDOW_CREATE, false, 6, 0, rng.next());
                t.check("Gui_MasakiInit", 0x007c_86e0, Conv::Cdecl, &[which], |m| {
                    Out::U32(ll::gui_masaki_init(m, which))
                });
                t.reset();
            }
        }
        for _ in 0..30 {
            let lgt = t.cstr("LGT");
            let other = t.cstr("LGTX");
            let n = rng.below(5);
            let firsts: Vec<u32> = (0..n)
                .map(|_| if rng.chance(4) { other } else { lgt })
                .collect();
            let ints: Vec<u32> = (0..n * 8)
                .map(|k| match k % 8 {
                    0 => rng.below(9).wrapping_sub(1),
                    1 => rng.below(8).wrapping_sub(1),
                    _ => rng.next(),
                })
                .collect();
            t.stub_seq(
                ll::PACKET_ARG,
                false,
                1,
                0,
                &[firsts.as_slice(), &[0]].concat(),
            );
            t.stub_seq(
                ll::PACKET_ARG_INT,
                false,
                1,
                0,
                &[ints.as_slice(), &[0]].concat(),
            );
            t.stub(ll::DEBUG_TOKENIZE, false, 0, 0, rng.next());
            t.check(
                "DebugScript_readLights_0084d790",
                0x0084_d790,
                Conv::Cdecl,
                &[n],
                |m| {
                    ll::debug_read_lights(m, n);
                    Out::None
                },
            );
            t.reset();
        }
        let scripts = [
            "VERSION 3\n#LIGHT 2\nLGT 0 1\nLGT 1 2\n#skip 1\nx y\nz",
            "#LIGHT 0\n#x 0\n#y 2\na\nb\nc",
            "",
            "plain words\nVERSION\n#LIGHT\n",
            "xa 3\n#b\nl1\nl2\nl3\nl4\n#c 1\nVERSION",
        ];
        for s in scripts {
            let buf = t.cstr(s);
            t.g32(ll::DEBUG_CURSOR, buf);
            t.g32(ll::DEBUG_END, buf + s.len() as u32);
            let packet = t.zeros(0x40);
            t.stub(ll::PACKET_PTR, false, 0, 0, packet);
            t.stub(ll::DEBUG_LIGHTS, false, 1, 0, rng.next());
            t.check(
                "DebugScript_interpret_00853210",
                0x0085_3210,
                Conv::Cdecl,
                &[],
                |m| Out::U32(ll::debug_interpret(m)),
            );
            t.reset();
        }
        for case in 0..4 {
            let user = t.cstr("user");
            let file = if case % 2 == 0 { 0 } else { t.zeros(0x40) };
            t.stubs(&[
                (ll::USER_NAME, false, 0, 0, user),
                (ll::PC_READ, false, 1, 0, file),
                (ll::PC_READ_LEN, false, 0, 0, rng.below(0x40)),
                (ll::DBG_PRINTF, false, 3, 0, rng.next()),
                (ll::DEBUG_INTERPRET, false, 0, 0, rng.next()),
                (ll::USER_FREE, false, 1, 0, rng.next()),
            ]);
            t.check(
                "DebugScript_loadLights_008531a0",
                0x0085_31a0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(ll::debug_load_lights(m)),
            );
            t.reset();
        }
    });
}

#[test]
fn rp4_debug_gui_pages_match_the_original() {
    // The progress window (open or not), the character view's controls
    // (flag bit 6 set or not), the light page texts (mode on or off).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_40c1);
        for open in [0, 1, 0, 1] {
            t.g32(ll::PROG_OPEN, open);
            let creates = [rng.next(), rng.next()];
            t.stub_seq(ll::GU_WINDOW_CREATE, false, 6, 0, &creates);
            t.stubs(&[
                (ll::GU_WINDOW_SET_LOCAL_TRANS, false, 3, 0, rng.next()),
                (ll::GU_WINDOW_SET_PARENT, false, 2, 0, rng.next()),
                (ll::SG_SET_RESOURCE, false, 2, 0, rng.next()),
            ]);
            t.check(
                "Gui_ProgAvarage",
                0x0085_5430,
                Conv::Cdecl,
                &[],
                |m| match ll::gui_prog_average(m) {
                    Some(v) => Out::U32(v),
                    None => Out::None,
                },
            );
            t.reset();
        }
        for _ in 0..8 {
            let chr = t.zeros(0x200);
            t.w32(chr + 0x194, rng.next());
            let window = rng.next();
            t.stubs(&[
                (ll::GU_CONTROL_SET_DISABLE, false, 3, 0, rng.next()),
                (ll::GU_CONTROL_SET_TEXT, false, 3, 0, rng.next()),
            ]);
            t.check(
                "ChrView_updateControls_0084cc30",
                0x0084_cc30,
                Conv::Cdecl,
                &[window, chr],
                |m| Out::U32(ll::chrview_update_controls(m, window, chr)),
            );
            t.reset();
        }
        for _ in 0..6 {
            let at = t.rebase(ll::LIGHT_MODE);
            t.write(at, &[u8::from(rng.chance(2))]);
            t.g32(ll::LIGHT_NAME_ON, rng.next());
            t.g32(ll::LIGHT_NAME_OFF, rng.next());
            let window = rng.next();
            t.stub(ll::GU_CONTROL_SET_TEXT, false, 3, 0, rng.next());
            t.check(
                "ChrMenu_updateLightTexts_0084d540",
                0x0084_d540,
                Conv::Cdecl,
                &[window],
                |m| Out::U32(ll::chrmenu_update_light_texts(m, window)),
            );
            t.reset();
        }
    });
}

#[test]
#[allow(clippy::too_many_lines)] // five functions
fn rp4_character_menu_matches_the_original() {
    // The character selector (none in use, 0..5 characters with some
    // unused), the character view window (the strings copied, the clip
    // toggle), the two file listings (light mode on and off, lists of
    // 0..4 entries; sprintf as it is), and the light script saver.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_40d1);
        for case in 0..12 {
            let n = rng.below(6);
            let chrs = t.zeros(0x880 * n as usize + 4);
            for k in 0..n {
                t.w8(chrs + 0x880 * k + 2, u8::from(!rng.chance(3)));
                t.w32(chrs + 0x880 * k + 4, rng.next());
            }
            t.g32(ll::CHR_COUNT, n);
            t.g32(ll::CHR_ARRAY, chrs);
            let sel = t.words(&[rng.next()]);
            let used = if case % 4 == 0 { 0 } else { n };
            t.stubs(&[
                (ll::CH_SYS_NB_USE, false, 0, 0, used),
                (ll::GU_SELECTOR_RESET, false, 2, 0, rng.next()),
                (ll::GU_SELECTOR_SET_ITEM, false, 4, 0, rng.next()),
            ]);
            t.check(
                "ChrMenu_fillSelector_0084ce10",
                0x0084_ce10,
                Conv::Cdecl,
                &[sel],
                |m| Out::U32(ll::chrmenu_fill_selector(m, sel)),
            );
            t.reset();
        }
        for _ in 0..6 {
            let chr = t.zeros(0x20);
            t.write(chr, &(rng.below(0x10000) as u16).to_le_bytes());
            let name = t.cstr("Tidus");
            t.w32(chr + 4, name);
            let arg = t.zeros(0x100);
            let db = t.cstr("c001");
            let cell = t.words(&[0, rng.next()]);
            t.stubs(&[
                (ll::SG_SET_PLAYER, false, 1, 0, rng.next()),
                (ll::GU_WINDOW_CREATE, false, 6, 0, rng.next()),
                (ll::GU_WINDOW_GET_ARG, false, 1, 0, arg),
                (ll::SET_PTR32, false, 1, 0, rng.next()),
                (ll::SG_DB_NAME, false, 1, 0, db),
                (ll::GET_PTR32, false, 1, 0, cell),
                (ll::GU_WINDOW_SET_TITLE, false, 2, 0, rng.next()),
                (ll::CHRVIEW_UPDATE_CONTROLS, false, 2, 0, rng.next()),
                (ll::GU_BUTTON_SET_TOGGLE, false, 3, 0, rng.next()),
                (ll::CH_GET_CLIP, false, 1, 0, rng.below(2)),
                (ll::GU_BUTTON_SET_TOGGLE_VALUE, false, 3, 0, rng.next()),
            ]);
            t.check("create_chrview", 0x0084_cea0, Conv::Cdecl, &[chr], |m| {
                Out::U32(ll::create_chrview(m, chr))
            });
            t.reset();
        }
        for case in 0..8 {
            let at = t.rebase(ll::LIGHT_MODE);
            t.write(at, &[u8::from(case % 2 == 0)]);
            let (a, b) = (t.cstr("c001"), t.cstr("m002"));
            t.g32(ll::LIGHT_NAME_ON, a);
            t.g32(ll::LIGHT_NAME_OFF, b);
            let n = rng.below(5);
            let items: Vec<u32> = (0..n).map(|_| rng.next()).collect();
            let data = t.words(&[items.as_slice(), &[0]].concat());
            for list in [ll::MOTION_LIST, ll::CHR_LIST] {
                t.g32(list, n);
                t.g32(list + 8, data);
            }
            t.stubs(&[
                (ll::MAKE_FILELIST, false, 3, 0, rng.next()),
                (ll::GU_SELECTOR_RESET, false, 2, 0, rng.next()),
                (ll::GU_SELECTOR_SET_ITEM, false, 4, 0, rng.next()),
            ]);
            let sel = rng.next();
            t.skip = 4;
            t.check_framed(
                "ChrMenu_listMotions_0084d3f0",
                0x0084_d3f0,
                Conv::Cdecl,
                &[sel],
                0x84,
                |m, ebp| Out::U32(ll::chrmenu_list_motions(m, ebp - 0x84, sel)),
            );
            t.check_framed(
                "ChrMenu_listCharacters_0084d4a0",
                0x0084_d4a0,
                Conv::Cdecl,
                &[sel],
                0x84,
                |m, ebp| Out::U32(ll::chrmenu_list_characters(m, ebp - 0x84, sel)),
            );
            t.skip = 0;
            t.reset();
        }
        let table = t.rebase(ll::DEBUG_LIGHT_TABLE);
        let bytes: Vec<u8> = (0..0x1c * 42).map(|_| rng.next() as u8).collect();
        t.write(table, &bytes);
        // the lines printed as they are into a buffer (the script cursor)
        let out = t.zeros(0x2000);
        t.g32(ll::DEBUG_CURSOR, out);
        t.check("Gui_SettingSaveLight", 0x0084_d8a0, Conv::Cdecl, &[], |m| {
            Out::U32(ll::gui_setting_save_light(m))
        });
        t.reset();
    });
}

#[test]
#[allow(clippy::too_many_lines)] // three functions
fn rp4_character_menu_windows_match_the_original() {
    // The specular window (build, steps both ways by 1 and 10 with the
    // clamps, other buttons and messages; sprintf as it is), the
    // character selector messages (3 in both light modes, 0xf, 0x3e,
    // others; _strdup stubbed), the character load (light mode with a
    // numbered name, the other mode, allocation failing; sscanf and
    // sprintf as they are).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_40e1);
        for case in 0..90 {
            let v = t.words(&[[0, 3, 0x100, 0xfe, 9, rng.below(0x110)][case % 6]]);
            let msg =
                [0x8001, 0x8005, 0x8009, 0x8005, 0x8009, 7][(case / 6 + rng.below(2) as usize) % 6];
            let arg = [0, 2, 1][(case / 2 + rng.below(2) as usize) % 3];
            // one step past the top
            let (msg, arg) = if case == 89 {
                t.w32(v, 0x100);
                (0x8005, 2)
            } else {
                (msg, arg)
            };
            t.stubs(&[
                (ll::GU_WINDOW_GET_ARG, false, 1, 0, v),
                (ll::GU_WINDOW_SET_ARG, false, 2, 0, rng.next()),
                (ll::GU_WINDOW_SET_TITLE, false, 2, 0, rng.next()),
                (ll::GU_WINDOW_ADD_STATIC, false, 7, 0, rng.next()),
                (ll::GU_WINDOW_ADD_BUTTON, false, 7, 0, rng.next()),
                (ll::GU_CONTROL_SET_TEXT, false, 3, 0, rng.next()),
                (ll::SET_SPECULAR_POWER, false, 1, 0, rng.next()),
            ]);
            let window = rng.next();
            t.skip = 4;
            t.check_framed(
                "SpecWindowProc_0084ccc0",
                0x0084_ccc0,
                Conv::Cdecl,
                &[window, msg, arg],
                0x18,
                |m, ebp| Out::U32(ll::spec_window_proc(m, ebp - 0x18, window, msg, arg)),
            );
            t.skip = 0;
            t.reset();
        }
        for case in 0..16 {
            let at = t.rebase(ll::LIGHT_MODE);
            t.write(at, &[u8::from(rng.chance(2))]);
            let msg = [3, 0xf, 0x3e, 5][case % 4];
            let sel = t.words(&[rng.next(), rng.next()]);
            t.stubs(&[
                (ll::GU_SELECTOR_ITEM_NAME, false, 1, 0, rng.next()),
                (ll::GU_CONTROL_SET_TEXT, false, 3, 0, rng.next()),
                (ll::SG_SET_RESOURCE_STR, false, 2, 0, rng.next()),
                (ll::CHR_LOAD_SELECTED, false, 1, 0, rng.next()),
                (ll::GU_SELECTOR_ITEM_VALUE, false, 1, 0, rng.next()),
                (ll::CREATE_CHRVIEW, false, 1, 0, rng.next()),
                (ll::GU_SELECTOR_MOUSE_FUKKI, false, 2, 0, rng.next()),
            ]);
            t.stub_import(ll::STRDUP, false, 1, 0, rng.next());
            let window = rng.next();
            t.check(
                "gui_chr_GWM_SELECTOR",
                0x0084_d270,
                Conv::Cdecl,
                &[window, msg, sel],
                |m| Out::U32(ll::gui_chr_selector(m, window, msg, sel)),
            );
            t.reset();
        }
        for case in 0..60 {
            let at = t.rebase(ll::LIGHT_MODE);
            t.write(at, &[u8::from(case % 3 != 0)]);
            let names = ["c001", "m123", "x", "a7"];
            let name = t.cstr(names[rng.below(4) as usize]);
            let (fa, fb) = (t.cstr("data"), t.cstr("prot"));
            t.g32(ll::LIGHT_NAME_ON, fa);
            t.g32(ll::LIGHT_NAME_OFF, fb);
            let sel = t.words(&[rng.next(), rng.next()]);
            let chr = if case % 5 == 2 { 0 } else { rng.next() };
            t.stubs(&[
                (ll::GU_SELECTOR_ITEM_NAME, false, 1, 0, name),
                (ll::SG_DB_NUM, false, 1, 0, rng.below(5)),
                (ll::CH_ID_FROM_NAME, false, 2, 0, rng.next()),
                (ll::SG_SET_RESOURCE_STR, false, 2, 0, rng.next()),
                (ll::CH_SET_ALLOCATE_ERR, false, 0, 0, rng.next()),
                (ll::CH_ALLOCATE, false, 1, 0, chr),
                (ll::OP_ET_BINDEFF, false, 1, 0, rng.next()),
                (ll::CREATE_CHRVIEW, false, 1, 0, rng.next()),
                (ll::CHN_READ_SYSTEM_MGRP, false, 2, 0, rng.next()),
                (ll::CH_SET_MOTION_TYPE, false, 2, 0, rng.next()),
                (ll::CH_SET_SYS_MOTION, false, 3, 0, rng.next()),
                (ll::CH_MOTION_SET_LOOP, false, 2, 0, rng.next()),
                (ll::GU_MESSAGE_BOX, false, 1, 0, rng.next()),
                (ll::GU_SELECTOR_MOUSE_FUKKI, false, 2, 0, rng.next()),
            ]);
            t.skip = 4;
            t.check_framed(
                "ChrMenu_loadSelected_0084d5e0",
                0x0084_d5e0,
                Conv::Cdecl,
                &[sel],
                0x48,
                |m, ebp| Out::U32(ll::chrmenu_load_selected(m, ebp - 0x48, sel)),
            );
            t.skip = 0;
            t.reset();
        }
    });
}

#[test]
#[allow(clippy::too_many_lines)] // three functions
fn rp4_light_windows_match_the_original() {
    // The character menu set-up (an empty folder resource or not, the
    // window known or not), the light value editor (every message and
    // button, values around the limits and the angles' wrap), and the
    // light window (each message, the back colour light or another, on
    // and off; sprintf as it is).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_40f1);
        for case in 0..8 {
            let empty = t.cstr("");
            let folder = t.cstr("c001");
            let res = [if case % 2 == 0 { empty } else { folder }, rng.next()];
            t.stub_seq(ll::SG_GET_RESOURCE_STR, false, 1, 0, &res);
            t.stubs(&[
                (ll::USER_NAME, false, 0, 0, rng.next()),
                (ll::SG_SET_RESOURCE_STR, false, 2, 0, rng.next()),
                (ll::SG_ADD_BUTTON, false, 3, 0, rng.next()),
                (ll::SG_ADD_STATIC, false, 3, 0, rng.next()),
                (ll::SG_ADD_Y, false, 1, 0, rng.next()),
                (ll::GU_BUTTON_SET_TOGGLE, false, 3, 0, rng.next()),
                (ll::GU_BUTTON_SET_TOGGLE_VALUE, false, 3, 0, rng.next()),
                (ll::SG_DISP_ENABLE, false, 0, 0, rng.below(2)),
                (ll::CH_CALC_NORMAL, false, 0, 0, rng.below(2)),
                (ll::CH_MOTION_HALF, false, 0, 0, rng.below(2)),
                (ll::SG_GET_RESOURCE, false, 2, 0, rng.next()),
                (ll::CHRMENU_LIGHT_TEXTS, false, 1, 0, rng.next()),
                (ll::CH_ENABLE_TEXTURE, false, 0, 0, rng.below(2)),
                (ll::SG_GET_Y, false, 0, 0, rng.next()),
            ]);
            let window = if case % 4 == 1 { 0 } else { rng.next() };
            let info = t.zeros(0x100);
            t.check(
                "Gui_ChrMenuInit",
                0x0084_ca30,
                Conv::Cdecl,
                &[window, info],
                |m| Out::U32(ll::gui_chr_menu_init(m, window, info)),
            );
            t.reset();
        }
        for case in 0..200 {
            let vals: Vec<u32> = (0..6)
                .map(|k| match rng.below(5) {
                    0 => 0,
                    1 => 0xff,
                    2 => {
                        if k >= 4 {
                            359
                        } else {
                            0x100
                        }
                    }
                    3 => rng.below(10).wrapping_sub(5),
                    _ => rng.below(400),
                })
                .collect();
            let v = t.words(&vals);
            let msg = [0x8005, 0x8009, 0x800d, 0x8007][case % 4];
            let button = rng.below(20);
            let window = rng.next();
            t.check(
                "ChrMenu_editLight_0084d940",
                0x0084_d940,
                Conv::Cdecl,
                &[v, window, msg, button],
                |m| Out::U32(ll::chrmenu_edit_light(m, v, msg, button)),
            );
        }
        for case in 0..60 {
            let light = t.zeros(0x20);
            for k in 0..3 {
                t.w32(light + 4 * k, rng.below(256));
            }
            let kind = if rng.chance(3) {
                4
            } else {
                rng.below(6) as u16
            }; // past 5 the range check aborts
            t.write(light + 0x18, &kind.to_le_bytes());
            t.w8(light + 0x1a, u8::from(rng.chance(2)));
            t.g32(ll::LIGHT_WINDOW_LIGHT, light);
            t.stubs(&[
                (ll::GU_WINDOW_GET_ARG, false, 1, 0, light),
                (ll::GU_WINDOW_SET_ARG, false, 2, 0, rng.next()),
                (ll::GU_WINDOW_ADD_BUTTON, false, 7, 0, rng.next()),
                (ll::GU_WINDOW_ADD_STATIC, false, 7, 0, rng.next()),
                (ll::GU_BUTTON_SET_TOGGLE, false, 3, 0, rng.next()),
                (ll::GU_BUTTON_SET_TOGGLE_VALUE, false, 3, 0, rng.next()),
                (ll::GU_CONTROL_SET_TEXT, false, 3, 0, rng.next()),
                (ll::CHRMENU_EDIT_LIGHT, false, 4, 0, rng.next()),
                (ll::LIGHT_APPLY, false, 1, 0, rng.next()),
                (ll::YI_SET_BACK_COLOR, false, 1, 0, rng.next()),
            ]);
            let msg = [
                0x8001, 0x8003, 0x8005, 0x8009, 0x800d, 0x8007, 0x8011, 0x8002, 0x9000,
            ][case % 9];
            let button = if rng.chance(2) { 0x4d2 } else { rng.below(12) };
            let window = rng.next();
            t.skip = 4;
            // a frame not zero, so the black colour's four words show
            t.check_framed_init(
                "LightWindowProc_0084e100",
                0x0084_e100,
                Conv::Cdecl,
                &[window, msg, button],
                0x28,
                &[0x5a; 0x24],
                |m, ebp| Out::U32(ll::light_window_proc(m, ebp - 0x28, window, msg, button)),
            );
            t.skip = 0;
            t.reset();
        }
    });
}

#[test]
fn rp4_dir_light_window_matches_the_original() {
    // Each message (build, flag, edits, toggle, draw) on and off, the
    // drawing for camera yaws all round and light angles past a turn
    // either way (Angle_Wrap and the trig intrinsics as they are; the
    // camera's yaw from a patched Sg_CameraGetYaw).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_dac0);
        for case in 0..120 {
            let light = t.zeros(0x20);
            for k in 0..3 {
                t.w32(light + 4 * k, rng.below(256));
            }
            for k in 4..6 {
                t.w32(light + 4 * k, rng.below(1500).wrapping_sub(750));
            }
            let kind = rng.below(6) as u16; // past 5 the range check aborts
            t.write(light + 0x18, &kind.to_le_bytes());
            t.w8(light + 0x1a, u8::from(rng.chance(2)));
            t.g32(ll::LIGHT_WINDOW_LIGHT, light);
            t.g32(ll::DIR_WINDOW_BOTTOM, rng.below(200));
            let cell = t.zeros(4);
            let yaw = (f32::from(rng.below(20_000) as u16) / 10_000.0 - 1.0) * 3.5;
            t.w32(cell, yaw.to_bits());
            let mut code = vec![0xd9, 0x05];
            code.extend(cell.to_le_bytes());
            code.push(0xc3); // fld dword [cell]; ret
            let at = t.rebase(ll::SG_CAMERA_GET_YAW);
            t.patch(at, &code);
            t.stubs(&[
                (ll::GU_WINDOW_GET_ARG, false, 1, 0, light),
                (ll::GU_WINDOW_SET_ARG, false, 2, 0, rng.next()),
                (ll::GU_WINDOW_ADD_BUTTON, false, 7, 0, rng.next()),
                (ll::GU_WINDOW_ADD_STATIC, false, 7, 0, rng.next()),
                (ll::GU_BUTTON_SET_TOGGLE, false, 3, 0, rng.next()),
                (ll::GU_BUTTON_SET_TOGGLE_VALUE, false, 3, 0, rng.next()),
                (ll::GU_CONTROL_SET_TEXT, false, 3, 0, rng.next()),
                (ll::CHRMENU_EDIT_LIGHT, false, 4, 0, rng.next()),
                (ll::LIGHT_APPLY, false, 1, 0, rng.next()),
                (ll::GU_PACKET_BEGIN_TAG, false, 1, 0, rng.next()),
                (ll::GU_PACKET_END_TAG, false, 0, 0, rng.next()),
                (ll::GU_SET_TEST, false, 8, 0, rng.next()),
                (ll::GU_SET_PRIM, false, 1, 0, rng.next()),
                (ll::GU_SET_RGBAQ, false, 4, 0, rng.next()),
                (ll::GU_SET_XYZF2, false, 2, 0, rng.next()),
            ]);
            let msg = if case % 2 == 0 {
                0x8011
            } else {
                [
                    0x8001, 0x8003, 0x8005, 0x8009, 0x800d, 0x8007, 0x8002, 0x9000,
                ][case / 2 % 8]
            };
            let button = if rng.chance(2) { 0x4d2 } else { rng.below(18) };
            let window = rng.next();
            t.skip = 4;
            t.check_framed(
                "FUN_0084dac0",
                0x0084_dac0,
                Conv::Cdecl,
                &[window, msg, button],
                0x18,
                |m, ebp| {
                    Out::U32(ll::dir_light_window_proc(
                        m,
                        ebp - 0x18,
                        window,
                        msg,
                        button,
                    ))
                },
            );
            t.skip = 0;
            t.reset();
        }
    });
}

#[test]
fn rp4_light_group_load_save_matches_the_original() {
    // Saving and loading each group, lights on and off with angles past
    // a turn either way (the light setters, the apply and the back
    // colour stubbed; Angle_Wrap and the intrinsics as they are); the
    // colour/direction vector in the original's aligned frame compared.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_e3c0);
        for case in 0..40 {
            let first = t.rebase(ll::EDIT_LIGHTS);
            let table = t.rebase(ll::DEBUG_LIGHT_TABLE);
            for at in (0..0xa8 * 8).step_by(4) {
                let v = rng.below(1500).wrapping_sub(500);
                t.w32(table + at, v);
            }
            for i in 0..6 {
                let e = table + (case % 8) * 0xa8 + 0x1c * i;
                t.w8(e + 0x1a, u8::from(rng.chance(2)));
                let k = rng.below(6) as u16;
                t.write(e + 0x18, &k.to_le_bytes());
                for at in (0..0x18).step_by(4) {
                    let v = rng.next();
                    t.w32(first + 0x1c * i + at, v);
                }
            }
            // Sg_LightSetColor copying each colour into a buffer (the vector is
            // overwritten by the direction after it)
            let colours = t.zeros(0x100);
            let cursor = t.words(&[colours]);
            let mut c = vec![0x8b, 0x44, 0x24, 0x08, 0x8b, 0x15];
            c.extend(cursor.to_le_bytes());
            c.extend([0x8b, 0x08, 0x89, 0x0a, 0x8b, 0x48, 0x04, 0x89, 0x4a, 0x04]);
            c.extend([
                0x8b, 0x48, 0x08, 0x89, 0x4a, 0x08, 0x83, 0xc2, 0x0c, 0x89, 0x15,
            ]);
            c.extend(cursor.to_le_bytes());
            c.push(0xc3);
            let set_colour = t.cave_code(&c);
            t.jump(ll::SG_LIGHT_SET_COLOR, set_colour);
            t.stubs(&[
                (ll::SG_LIGHT_SET_DIR, false, 2, 0, rng.next()),
                (ll::LIGHT_APPLY, false, 1, 0, rng.next()),
                (ll::YI_SET_BACK_COLOR, false, 1, 0, rng.next()),
            ]);
            let (group, load) = (case % 8, u32::from(case % 3 != 0));
            // the original realigns its frame: ebp = (entry ebp - 8) & -16
            let std = t.frame_ebp(2);
            let low = std - ((std - 8) & !0xf) + 0x20;
            t.skip = low - 0x10;
            t.check_framed(
                "FUN_0084e3c0",
                0x0084_e3c0,
                Conv::Cdecl,
                &[group, load],
                low,
                |m, ebp| Out::U32(ll::light_group_load_save(m, ebp - low, group, load)),
            );
            t.skip = 0;
            t.reset();
        }
    });
}

#[test]
fn rp4_spec_and_capture_windows_match_the_original() {
    // The specular window: each message, the toggles from every state
    // (the getters stubbed by the case), spin values either sign for the
    // tenths; the capture window with its settings bytes all ways.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_2a30);
        for case in 0..160 {
            let st = [rng.below(2), rng.below(2), rng.below(2)];
            for spin in [
                ll::SPIN_SIZE,
                ll::SPIN_COEF,
                ll::SPIN_RED,
                ll::SPIN_GREEN,
                ll::SPIN_BLUE,
            ] {
                let at = t.rebase(spin);
                for k in 0..12 {
                    let v = if rng.chance(2) {
                        rng.below(300).wrapping_sub(100)
                    } else {
                        rng.next()
                    };
                    t.w32(at + 4 * k, v);
                }
            }
            t.stubs(&[
                (ll::GU_SPIN_MSG, false, 4, 0, rng.next()),
                (ll::GU_SPIN_ADD, false, 8, 0, rng.next()),
                (ll::GU_WINDOW_SET_TITLE, false, 2, 0, rng.next()),
                (ll::GU_WINDOW_ADD_BUTTON, false, 7, 0, rng.next()),
                (ll::GU_BUTTON_SET_TOGGLE, false, 3, 0, rng.next()),
                (ll::GU_BUTTON_SET_TOGGLE_VALUE, false, 3, 0, rng.next()),
                (ll::GU_CONTROL_SET_DISABLE, false, 3, 0, rng.next()),
                (ll::GU_CONTROL_SET_TEXTF, false, 5, 0, rng.next()),
                (ll::CH_GET_FOURCE_SPEC, false, 0, 0, st[0]),
                (ll::CH_GET_DISABLE_SPEC, false, 0, 0, st[1]),
                (ll::CH_GET_PARA_EDIT, false, 0, 0, st[2]),
                (ll::CH_SET_FOURCE_SPEC, false, 1, 0, rng.next()),
                (ll::CH_SET_DISABLE_SPEC, false, 1, 0, rng.next()),
                (ll::CH_SET_PARA_EDIT, false, 1, 0, rng.next()),
                (ll::CH_SET_SPEC_SIZE, false, 1, 0, rng.next()),
                (ll::CH_SET_SPEC_COEFFICIENT, false, 1, 0, rng.next()),
                (ll::CH_SET_SPEC_COLOR, false, 4, 0, rng.next()),
            ]);
            let msg = [
                0x8001, 0x8003, 0x8007, 0x8011, 0x8007, 0x8005, 0x8000, 0x8012,
            ][case % 8];
            let button = rng.below(10);
            let window = rng.next();
            t.check(
                "FUN_00852a30",
                0x0085_2a30,
                Conv::Cdecl,
                &[window, msg, button],
                |m| Out::U32(ll::spec_edit_window_proc(m, window, msg, button)),
            );
            t.reset();
        }
        for _ in 0..40 {
            for at in [
                ll::CAPTURE_SELECT,
                ll::CAPTURE_GUI,
                ll::CAPTURE_MODE,
                ll::CAPTURE_FILTER,
            ] {
                let v = [0, 1, 2, 0xff, 0x80][rng.below(5) as usize];
                t.w8(t.rebase(at), v);
            }
            t.stubs(&[
                (ll::GU_BUTTON_SET_TOGGLE_VALUE, false, 3, 0, rng.next()),
                (ll::GU_CONTROL_SET_DISABLE, false, 3, 0, rng.next()),
                (ll::SG_CAPTURE_SET_FILTER, false, 1, 0, rng.next()),
                (ll::SG_CAPTURE_SET_SELECT, false, 1, 0, rng.next()),
                (ll::SG_CAPTURE_SET_MODE, false, 1, 0, rng.next()),
                (ll::SG_CAPTURE_SET_GUI, false, 1, 0, rng.next()),
            ]);
            let window = rng.next();
            t.check("FUN_00853420", 0x0085_3420, Conv::Cdecl, &[window], |m| {
                Out::U32(ll::capture_window_update(m, window))
            });
            t.reset();
        }
    });
}

#[test]
fn rp4_capture_window_proc_matches_the_original() {
    // Each message and button (the toggles read back either way), the
    // settings bytes all ways; FUN_00853420 as it is (its callees
    // stubbed).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_3520);
        for case in 0..120 {
            for at in [
                ll::CAPTURE_SELECT,
                ll::CAPTURE_GUI,
                ll::CAPTURE_MODE,
                ll::CAPTURE_FILTER,
                ll::CAPTURE_WINDOW_FLAG,
            ] {
                let v = [0, 1, 2, 0xff][rng.below(4) as usize];
                t.w8(t.rebase(at), v);
            }
            t.stubs(&[
                (ll::GU_BUTTON_SET_TOGGLE_VALUE, false, 3, 0, rng.next()),
                (
                    ll::GU_BUTTON_GET_TOGGLE_VALUE,
                    false,
                    2,
                    0,
                    [0, 1, 0x100, 0x2ff][rng.below(4) as usize],
                ),
                (ll::GU_BUTTON_SET_TOGGLE, false, 3, 0, rng.next()),
                (ll::GU_CONTROL_SET_DISABLE, false, 3, 0, rng.next()),
                (ll::GU_CONTROL_SET_LEFT_TEXT, false, 3, 0, rng.next()),
                (ll::GU_WINDOW_ADD_BUTTON, false, 7, 0, rng.next()),
                (ll::GU_WINDOW_ADD_STATIC, false, 7, 0, rng.next()),
                (ll::GU_WINDOW_SET_TITLE, false, 2, 0, rng.next()),
                (ll::SG_CAPTURE_ENABLE, false, 0, 0, rng.next()),
                (ll::SG_CAPTURE_SET_FILTER, false, 1, 0, rng.next()),
                (ll::SG_CAPTURE_SET_SELECT, false, 1, 0, rng.next()),
                (ll::SG_CAPTURE_SET_MODE, false, 1, 0, rng.next()),
                (ll::SG_CAPTURE_SET_GUI, false, 1, 0, rng.next()),
            ]);
            let msg = [
                0x8001, 0x8003, 0x8006, 0x8007, 0x8007, 0x8007, 0x8002, 0x8008, 0x8000,
            ][case % 9];
            let button = match rng.below(4) {
                0 => 0x4d2,
                1 => 1000,
                2 => 0x4d3,
                _ => rng.below(7),
            };
            let window = rng.next();
            t.check(
                "SpriteFrameRegion_Load",
                0x0085_3520,
                Conv::Cdecl,
                &[window, msg, button],
                |m| Out::U32(ll::capture_window_proc(m, window, msg, button)),
            );
            t.reset();
        }
    });
}

#[test]
fn rp4_chr_info_window_matches_the_original() {
    // Each message; a refused, missing, hidden or shown character with
    // floats, small and odd words everywhere; Gu_LogPrintf as it is
    // (vsprintf the game's), its Gu_Printf replaced by a stub appending
    // each line to a buffer, so the formatted text is compared.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_37d0);
        for case in 0..90 {
            let out = t.zeros(0x4000);
            let cursor = t.words(&[out]);
            // mov edx, [esp+4]; mov ecx, [cursor]; copy the string with
            // its NUL; mov [cursor], ecx; ret
            let mut c = vec![0x8b, 0x54, 0x24, 0x04, 0x8b, 0x0d];
            c.extend(cursor.to_le_bytes());
            c.extend([
                0x8a, 0x02, 0x88, 0x01, 0x42, 0x41, 0x84, 0xc0, 0x75, 0xf6, 0x89, 0x0d,
            ]);
            c.extend(cursor.to_le_bytes());
            c.push(0xc3);
            let printf = t.cave_code(&c);
            t.jump(ll::GU_PRINTF, printf);
            let chr = t.zeros(0x900);
            for at in (0..0x900).step_by(4) {
                let v = match rng.below(6) {
                    0 => 0,
                    1 => rng.below(0x200),
                    2 => rng.next(),
                    _ => ((f32::from(rng.below(20_000) as u16) - 10_000.0) / 7.0).to_bits(),
                };
                t.w32(chr + at, v);
            }
            let name = t.cstr("chr_name");
            t.w32(chr + 4, name);
            t.w8(chr + 2, u8::from(!rng.chance(5)));
            let motion = if rng.chance(2) { 0 } else { t.cstr("motion%d") };
            t.stubs(&[
                (
                    ll::GU_WINDOW_GET_ARG,
                    false,
                    1,
                    0,
                    if rng.chance(6) { 0 } else { chr },
                ),
                (ll::CH_CHECK_HANDLE, false, 1, 0, u32::from(rng.chance(6))),
                (ll::GU_WINDOW_SET_ARG, false, 2, 0, rng.next()),
                (ll::GU_WINDOW_SET_TITLE, false, 2, 0, rng.next()),
                (ll::GU_MOUSE_SET_WINDOW, false, 1, 0, rng.next()),
                (ll::GU_LOCATE, false, 2, 0, rng.next()),
                (ll::GU_PRINT_SET_SIZE_ONLY, false, 1, 0, rng.next()),
                (ll::GU_PRINT_SET_ITALIC, false, 1, 0, rng.next()),
                (ll::GU_PRINT_SET_BOLD, false, 1, 0, rng.next()),
                (ll::GU_SET_HSCALE, false, 1, 0, rng.next()),
                (ll::MOTION_NAME, false, 1, 0, motion),
                (ll::CH_ID_GET_MAP, false, 1, 0, rng.next()),
                (ll::CH_ID_GET_MAP_B, false, 1, 0, rng.next()),
                (ll::CH_ID_GET_WATER, false, 1, 0, rng.next()),
                (ll::CH_ID_GET_ENCOUNT, false, 1, 0, rng.next()),
                (ll::CH_ID_GET_DICTIONARY, false, 1, 0, rng.next()),
                (ll::CH_ID_GET_EFFECT, false, 1, 0, rng.next()),
                (ll::CH_ID_GET_SOUND, false, 1, 0, rng.next()),
                (ll::CH_ID_GET_SHADE, false, 1, 0, rng.next()),
                (ll::CH_ID_GET_SHADE0, false, 1, 0, rng.next()),
                (ll::CH_ID_GET_SHADE1, false, 1, 0, rng.next()),
                (ll::CH_ID_GET_SHADE2, false, 1, 0, rng.next()),
                (
                    ll::GU_WINDOW_GET_LEFT,
                    false,
                    1,
                    0,
                    if rng.chance(3) { 0x46 } else { rng.below(0x90) },
                ),
                (ll::GU_WINDOW_GET_X, false, 1, 0, rng.next()),
                (ll::GU_WINDOW_GET_WIDTH, false, 1, 0, rng.next()),
                (ll::GU_SELECTOR_ALLOCATE, false, 0, 0, rng.next()),
                (
                    ll::GU_CHR_SELECTOR_FILL,
                    false,
                    0,
                    0,
                    u32::from(rng.chance(3)),
                ),
                (ll::GU_SELECTOR_SET_CALLBACK, false, 2, 0, rng.next()),
                (ll::GU_SELECTOR_OPEN_XYWH, false, 7, 0, rng.next()),
            ]);
            let msg = if case % 3 == 0 {
                [0x8001, 0x8006, 0x8000, 0x8007, 0x8012][case / 3 % 5]
            } else {
                0x8011
            };
            let window = rng.next();
            t.skip = 4;
            t.check_framed(
                "FUN_008537d0",
                0x0085_37d0,
                Conv::Cdecl,
                &[window, msg],
                0x104,
                |m, ebp| Out::U32(ll::chr_info_window_proc(m, ebp - 0x104, window, msg)),
            );
            t.skip = 0;
            t.reset();
        }
    });
}

#[test]
fn rp4_clock_window_matches_the_original() {
    // Other messages, then times of day all round (and odd words: past a
    // day, negative), each weekday; sprintf, Gu_WrapAngle and the trig
    // intrinsics as they are, the drawing and printing stubbed.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_3ec0);
        for case in 0..150 {
            let (hh, mm, ss) = if case % 10 == 9 {
                (
                    rng.next() % 1000,
                    rng.next() % 1000,
                    rng.below(200).wrapping_sub(100),
                )
            } else {
                (rng.below(24), rng.below(60), rng.below(60))
            };
            for (sym, v) in [
                (ll::CLOCK_HOUR, hh),
                (ll::CLOCK_MINUTE, mm),
                (ll::CLOCK_SECOND, ss),
                (ll::CLOCK_WEEKDAY, rng.below(7)),
                (ll::CLOCK_DAY, rng.below(32)),
                (ll::CLOCK_MONTH, rng.below(13)),
            ] {
                t.g32(sym, v);
            }
            t.stubs(&[
                (ll::GU_PRINT_SET_ITALIC, false, 1, 0, rng.next()),
                (ll::GU_PRINT_SET_BOLD, false, 1, 0, rng.next()),
                (ll::GU_PRINT_SET_COLOR, false, 4, 0, rng.next()),
                (ll::GU_PRINT_SET_X, false, 1, 0, rng.next()),
                (ll::GU_PRINT_SET_XY, false, 2, 0, rng.next()),
                (ll::GU_PRINT_SET_Y, false, 1, 0, rng.next()),
                (
                    ll::GU_PRINT_GET_WIDTH,
                    false,
                    1,
                    0,
                    rng.below(200).wrapping_sub(50),
                ),
                (ll::GU_PRINTF, false, 1, 0, rng.next()),
                (ll::GU_WINDOW_SET_TITLE, false, 2, 0, rng.next()),
                (ll::GU_PACKET_BEGIN_TAG, false, 1, 0, rng.next()),
                (ll::GU_PACKET_END_TAG, false, 0, 0, rng.next()),
                (ll::GU_SET_TEST, false, 8, 0, rng.next()),
                (ll::GU_SET_PRIM, false, 1, 0, rng.next()),
                (ll::GU_SET_RGBAQ, false, 4, 0, rng.next()),
                (ll::GU_SET_XYZF2, false, 2, 0, rng.next()),
            ]);
            let msg = if case < 5 {
                0x8001 + case as u32
            } else {
                0x8011
            };
            let window = rng.next();
            t.skip = 4;
            t.check_framed(
                "FUN_00853ec0",
                0x0085_3ec0,
                Conv::Cdecl,
                &[window, msg],
                0x24,
                |m, ebp| Out::U32(ll::clock_window_proc(m, ebp - 0x24, window, msg)),
            );
            t.skip = 0;
            t.reset();
        }
    });
}

#[test]
fn rp4_value_windows_match_the_original() {
    // The AccBuf and Blur windows: every message, the buttons around
    // 0..2 and "Current", values near and past both limits (the getters
    // stubbed; sprintf as it is).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_4270);
        for case in 0..160 {
            let v = match rng.below(4) {
                0 => rng.below(12),
                1 => 0x7a + rng.below(12),
                2 => rng.next(),
                _ => rng.below(0x90),
            };
            t.stubs(&[
                (ll::GU_WINDOW_SET_TITLE, false, 2, 0, rng.next()),
                (ll::GU_WINDOW_ADD_BUTTON, false, 7, 0, rng.next()),
                (ll::GU_WINDOW_ADD_STATIC, false, 7, 0, rng.next()),
                (ll::GU_CONTROL_SET_TEXT, false, 3, 0, rng.next()),
                (ll::SG_ACC_GET_ALPHA, false, 0, 0, v),
                (ll::SG_ACC_SET_ALPHA, false, 2, 0, rng.next()),
                (ll::SG_ACC_REQUEST_CURRENT, false, 1, 0, rng.next()),
                (ll::SG_GET_BLUR, false, 0, 0, v),
                (ll::SG_BLUR, false, 1, 0, rng.next()),
            ]);
            let msg = [
                0x8001, 0x8005, 0x8006, 0x8009, 0x8011, 0x8003, 0x8007, 0x8012,
            ][case % 8];
            let button = if rng.chance(4) { 0x4d2 } else { rng.below(4) };
            let window = rng.next();
            let (what, sym) = if case % 16 < 8 {
                ("FUN_00854270", 0x0085_4270)
            } else {
                ("FUN_00854420", 0x0085_4420)
            };
            t.skip = 4;
            t.check_framed(
                what,
                sym,
                Conv::Cdecl,
                &[window, msg, button],
                0x18,
                |m, ebp| {
                    let buf = ebp - 0x18;
                    Out::U32(if sym == 0x0085_4270 {
                        ll::acc_buf_window_proc(m, buf, window, msg, button)
                    } else {
                        ll::blur_window_proc(m, buf, window, msg, button)
                    })
                },
            );
            t.skip = 0;
            t.reset();
        }
    });
}

#[test]
fn rp4_flash_window_matches_the_original() {
    // Every message and button 0..13 and "Flash!", settings near and past
    // the limits either side.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_4590);
        for case in 0..200 {
            for sym in [
                ll::FLASH_TIME,
                ll::FLASH_RGB[0],
                ll::FLASH_RGB[1],
                ll::FLASH_RGB[2],
            ] {
                let v = match rng.below(5) {
                    0 => rng.below(14).wrapping_sub(4),
                    1 => 0xf8 + rng.below(14),
                    2 => 0x250 + rng.below(16),
                    3 => rng.next(),
                    _ => rng.below(0x300),
                };
                t.g32(sym, v);
            }
            t.stubs(&[
                (ll::GU_WINDOW_SET_TITLE, false, 2, 0, rng.next()),
                (ll::GU_WINDOW_ADD_BUTTON, false, 7, 0, rng.next()),
                (ll::GU_WINDOW_ADD_STATIC, false, 7, 0, rng.next()),
                (ll::GU_CONTROL_SET_TEXT, false, 3, 0, rng.next()),
                (ll::SG_ACC_GET_ALPHA, false, 0, 0, rng.next()),
                (ll::SG_FLASH, false, 4, 0, rng.next()),
            ]);
            let msg = [
                0x8001, 0x8005, 0x8007, 0x8009, 0x8011, 0x8005, 0x8009, 0x8003,
            ][case % 8];
            let button = if rng.chance(5) { 0x4d2 } else { rng.below(14) };
            let window = rng.next();
            t.skip = 4;
            t.check_framed(
                "FUN_00854590",
                0x0085_4590,
                Conv::Cdecl,
                &[window, msg, button],
                0x18,
                |m, ebp| Out::U32(ll::flash_window_proc(m, ebp - 0x18, window, msg, button)),
            );
            t.skip = 0;
            t.reset();
        }
    });
}

#[test]
fn rp4_help_windows_match_the_original() {
    // The help and tip windows: 0x8011 and others, the tip's text empty
    // or not, colours anything.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_5290);
        for case in 0..40 {
            let c = t.rebase(ll::GU_HELP_COLOUR);
            let v = rng.next();
            t.w32(c, v);
            let record = t.words(&[rng.next(), rng.next()]);
            t.g32(ll::GU_TIP_RECORD, record);
            t.g32(ll::GU_TIP_OWNER, rng.next());
            let tip = t.rebase(ll::GU_TIP_TEXT);
            t.w8(tip, if rng.chance(3) { 0 } else { b'x' });
            t.stubs(&[
                (ll::GU_WINDOW_SET_FLAG2, false, 2, 0, rng.next()),
                (ll::GU_PRINT_SET_BOLD, false, 1, 0, rng.next()),
                (ll::GU_PRINT_GET_WIDTH, false, 1, 0, rng.next()),
                (ll::GU_WINDOW_SET_SIZE, false, 3, 0, rng.next()),
                (ll::GU_PRINT_SET_X, false, 1, 0, rng.next()),
                (ll::GU_PRINTF, false, 1, 0, rng.next()),
                (ll::GU_PACKET_BEGIN_TAG, false, 1, 0, rng.next()),
                (ll::GU_PACKET_END_TAG, false, 0, 0, rng.next()),
                (ll::GU_SET_TEST, false, 8, 0, rng.next()),
                (ll::GU_SET_PRIM, false, 1, 0, rng.next()),
                (ll::GU_SET_RGBAQ, false, 4, 0, rng.next()),
                (ll::GU_SET_XYZF2, false, 2, 0, rng.next()),
            ]);
            let msg = if case % 3 == 0 {
                0x8001 + rng.below(0x20)
            } else {
                0x8011
            };
            let window = rng.next();
            if case % 2 == 0 {
                t.check(
                    "FUN_00855290",
                    0x0085_5290,
                    Conv::Cdecl,
                    &[window, msg],
                    |m| Out::U32(ll::help_window_proc(m, window, msg)),
                );
            } else {
                t.check(
                    "FUN_008554b0",
                    0x0085_54b0,
                    Conv::Cdecl,
                    &[window, msg],
                    |m| Out::U32(ll::tip_window_proc(m, window, msg)),
                );
            }
            t.reset();
        }
    });
}

#[test]
fn rp4_mem_graph_window_matches_the_original() {
    // Closing with and without a help window; drawing with the mouse
    // outside, over each band or between, a help window or none (the
    // usage words in the frame as Sg_MemUsage's stub leaves them, its
    // total never below 2).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_5020);
        for case in 0..120 {
            let mut init = vec![0_u8; 0x18];
            let mut total = 0_u32;
            for _ in 0..9 {
                let n = if rng.chance(3) {
                    rng.next()
                } else {
                    rng.below(0x40_0000)
                };
                total = total.wrapping_add(n);
                init.extend(n.to_le_bytes());
            }
            let total = if rng.chance(2) {
                total
            } else {
                rng.below(0x100_0000)
            };
            let total = if total.cast_signed() / 2 == 0 {
                2
            } else {
                total
            };
            let help = if rng.chance(2) { 0 } else { rng.next() };
            t.g32(ll::GU_HELP_WINDOW, help);
            t.stubs(&[
                (ll::SG_MEM_USAGE, false, 1, 0, total),
                (ll::SG_SET_RESOURCE, false, 2, 0, rng.next()),
                (ll::GU_WINDOW_GET_CLIENT_W, false, 1, 0, rng.below(0x200)),
                (ll::GU_WINDOW_GET_CLIENT_H, false, 1, 0, rng.below(0x200)),
                (ll::GU_WINDOW_MOUSE_IN, false, 1, 0, rng.below(3)),
                (ll::GU_MOUSE_GET_WINDOW_X, false, 1, 0, rng.below(0x200)),
                (ll::GU_WINDOW_GET_CLIENT_TOP, false, 1, 0, rng.below(0x20)),
                (ll::GU_MOUSE_GET_X, false, 0, 0, rng.next()),
                (ll::GU_MOUSE_GET_Y, false, 0, 0, rng.next()),
                (ll::GU_WINDOW_SET_POS, false, 3, 0, rng.next()),
                (ll::GU_WINDOW_CLOSE_TREE, false, 1, 0, rng.next()),
                (ll::GU_HELP_WINDOW_SHOW, false, 1, 0, rng.next()),
                (ll::GU_DRAW_RECT_STRIP, false, 1, 0, rng.next()),
                (ll::GU_PRINT_SET_SIZE_ONLY, false, 1, 0, rng.next()),
                (ll::GU_PACKET_BEGIN_TAG, false, 1, 0, rng.next()),
                (ll::GU_PACKET_END_TAG, false, 0, 0, rng.next()),
                (ll::GU_SET_PRIM, false, 1, 0, rng.next()),
                (ll::GU_SET_RGBAQ, false, 4, 0, rng.next()),
                (ll::GU_SET_XYZF2, false, 2, 0, rng.next()),
            ]);
            let msg = [0x8011, 0x8011, 0x8011, 0x8003, 0x8001][case % 5];
            let window = rng.next();
            t.skip = 4;
            t.check_framed_init(
                "FUN_00855020",
                0x0085_5020,
                Conv::Cdecl,
                &[window, msg],
                0x80,
                &init,
                |m, ebp| Out::U32(ll::mem_graph_window_proc(m, ebp - 0x80, window, msg)),
            );
            t.skip = 0;
            t.reset();
        }
    });
}
