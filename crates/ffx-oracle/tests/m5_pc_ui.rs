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
