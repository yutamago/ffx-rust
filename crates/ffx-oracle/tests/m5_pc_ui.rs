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
        // filled, so the zeros the callers store show
        let pool = self.alloc(&vec![0xcd; 0x4_0000]);
        let cur = self.words(&[pool, fail_after]);
        let mut c = vec![0x83, 0x3d];
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
        let ctr = self.words(&[0x0a00_0000]);
        let rec = self.stub_code(sym, false, 2, 0, 0x5e7);
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
