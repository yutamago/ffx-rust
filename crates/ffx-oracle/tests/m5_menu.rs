//! M5 stage 1, list `menu` (`ffx_menu::m5m_*`) against the original: on
//! the image alone with synthetic objects, and on the snapshots
//! (`FFX_SNAPSHOT`) over the game's memory. Results and every byte written
//! must agree. Needs `FFX_GAME_DIR`.
//!
//! None of these functions ran in a coverage run (most are the `Gu_` debug
//! GUI behind `Sg_MainDebug`), so every branch is reached with synthetic
//! data; the snapshots add the game's own globals underneath. Callees that
//! stay original are either run as they are in both oracles (pure memory
//! helpers such as `Gu_SetTest`, `Gu_MouseInRect`) or replaced in both
//! alike by recording stubs: each call appends its tag, `ecx` and its
//! arguments to a log in memory, so the diff checks that both made the
//! same calls in the same order.

#![allow(
    dead_code,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_menu::m5m_a::bridge as ma;
use ffx_menu::m5m_b::bridge as mb;
use ffx_menu::m5m_c::{self as mc_fields, bridge as mc};
use ffx_menu::m5m_d::bridge as md;
use ffx_menu::m5m_e::bridge as me;
use ffx_menu::m5m_f::bridge as mf;
use ffx_menu::m5m_g::bridge as mg;
use ffx_menu::m5m_h::bridge as mh;
use ffx_menu::m5m_i::bridge as mi;
use ffx_menu::m5m_j::bridge as mj;
use ffx_menu::m5m_k::bridge as mk;
use ffx_menu::m5m_l::bridge as ml;
use ffx_menu::m5m_m::bridge as mm;
use ffx_oracle::{Conv, Oracle, Pages};

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
    fn call_u64(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> u64 {
        ffx_oracle::check_width(
            "Host::call_u64",
            addr,
            args.len(),
            host::max_words(c, false),
        );
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        let r = self.0.call(addr, conv(c), args).unwrap();
        u64::from(r.eax) | u64::from(r.edx) << 32
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

// ---- batch 1: the Gu debug GUI's frame (ffx_menu::m5m_a) ----

/// Symbols of the GUI globals these tests set.
mod g {
    pub const PACKET_COUNT: u32 = 0x0130_36c0;
    pub const PACKET: u32 = 0x0130_36c4;
    pub const PACKET_DATA: u32 = 0x0130_36c8;
    pub const FADE: u32 = 0x0130_3730;
    pub const WINDOW_LIST: u32 = 0x0130_3734;
    pub const DRAG_WINDOW: u32 = 0x0130_3738;
    pub const RESIZE_WINDOW: u32 = 0x0130_373c;
    pub const POPUP_WINDOW: u32 = 0x0130_3740;
    pub const DRAG_GRAB: u32 = 0x0130_3744;
    pub const CAPTURE: u32 = 0x0130_3754;
    pub const CAPTURE_ID: u32 = 0x0130_3758;
    pub const HIDDEN: u32 = 0x0130_375c;
    pub const MOUSE_X: u32 = 0x0132_5240;
    pub const MOUSE_Y: u32 = 0x0132_5244;
    pub const SCREEN_H: u32 = 0x023c_bbe0;
    pub const SCREEN_W: u32 = 0x023c_bbe4;
    pub const WHEEL: u32 = 0x023c_bc04;
    pub const BUTTONS: u32 = 0x023c_bc10;
    pub const H_SCALE: u32 = 0x00c4_df54;
}

fn put16(w: &mut [u8], o: usize, v: i16) {
    w[o..o + 2].copy_from_slice(&v.to_le_bytes());
}

impl Twins {
    /// A packet buffer of 64 quadwords for both streams, `count` used.
    fn packet(&mut self, count: u32) {
        let fill: Vec<u8> = (0..0x400u32)
            .map(|i| (i.wrapping_mul(0x9e37_79b9) >> 24) as u8)
            .collect();
        let p = self.alloc(&fill);
        let d = self.alloc(&fill);
        self.g32(g::PACKET, p);
        self.g32(g::PACKET_DATA, d);
        self.g32(g::PACKET_COUNT, count);
    }

    /// A window of `n` controls with random rectangles near the mouse,
    /// random kinds 0..=9 and flags; its callback (unless null) logs
    /// `(window, msg, id)`.
    fn window(&mut self, rng: &mut Rng, n: u32) -> u32 {
        let cb = if rng.chance(5) {
            0
        } else {
            let ret = rng.word();
            self.stub_code(0x7c, false, 3, 0, ret)
        };
        let label = self.alloc(b"Label\0");
        let mut c = vec![0u8; 0x44 * n as usize];
        for r in c.chunks_mut(0x44) {
            for b in &mut r[0x8..0x18] {
                *b = rng.next() as u8;
            }
            r[0x28..0x2c].copy_from_slice(&label.to_le_bytes());
            put16(r, 0, rng.below(40) as i16 - 8);
            put16(r, 2, rng.below(40) as i16 - 8);
            put16(r, 4, rng.below(60) as i16);
            put16(r, 6, rng.below(60) as i16);
            r[0x1a] = rng.next() as u8;
            put16(r, 0x20, rng.below(3) as i16);
            put16(r, 0x2c, rng.below(10) as i16);
            put16(r, 0x2e, rng.below(4) as i16);
            r[0x40] = if rng.chance(4) { rng.below(4) as u8 } else { 0 };
        }
        let controls = if n == 0 { 0 } else { self.alloc(&c) };
        let mut w = vec![0u8; 0xa0];
        put16(&mut w, 0xc, rng.below(60) as i16 - 10);
        put16(&mut w, 0xe, rng.below(60) as i16 - 10);
        put16(&mut w, 0x10, rng.below(120) as i16);
        put16(&mut w, 0x12, rng.below(120) as i16);
        w[0x7c..0x80].copy_from_slice(&cb.to_le_bytes());
        put16(&mut w, 0x80, rng.below(8) as i16);
        put16(&mut w, 0x82, rng.below(12) as i16);
        put16(&mut w, 0x84, rng.below(100) as i16);
        put16(&mut w, 0x86, rng.below(100) as i16);
        let flags = match rng.below(4) {
            0 => 0,
            1 => rng.next() & 0x81,
            _ => rng.next() & 0x9f,
        };
        w[0x88..0x8c].copy_from_slice(&flags.to_le_bytes());
        put16(&mut w, 0x8c, rng.below(0x100) as i16 - 0x40);
        put16(&mut w, 0x8e, rng.below(0x200) as i16);
        w[0x90..0x94].copy_from_slice(&n.to_le_bytes());
        w[0x94..0x98].copy_from_slice(&controls.to_le_bytes());
        self.alloc(&w)
    }

    /// Random mouse state: position, buttons, wheel; the capture id one
    /// of the controls' ids.
    fn mouse(&mut self, rng: &mut Rng) {
        let (x, y) = (rng.below(100), rng.below(100));
        self.g32(g::MOUSE_X, x);
        self.g32(g::MOUSE_Y, y);
        let mut b = 0;
        for bit in [
            1, 2, 4, 8, 0x100, 0x200, 0x400, 0x800, 0x1_0000, 0x2_0000, 0x4_0000, 0x8_0000,
        ] {
            if rng.chance(3) {
                b |= bit;
            }
        }
        self.g32(g::BUTTONS, b);
        let wheel = rng.below(5).wrapping_sub(2);
        self.g32(g::WHEEL, wheel);
        let id = rng.below(4);
        self.g32(g::CAPTURE_ID, id);
        let fade = rng.below(0x100);
        self.g32(g::FADE, fade);
    }

    /// The window callees that are not pure memory code, as recording
    /// stubs: `Gu_WindowToFront`, `Gu_WindowControlSetRect`,
    /// `Gu_WindowStartDrag`, `Sg_ControlMouseCamera`.
    fn window_stubs(&mut self) {
        self.stubs(&[
            (ma::GU_WINDOW_TO_FRONT, false, 1, 0, 0x11),
            (ma::GU_WINDOW_CONTROL_SET_RECT, false, 6, 0, 0x12),
            (ma::GU_WINDOW_START_DRAG, false, 1, 0, 0x13),
            (ma::SG_CONTROL_MOUSE_CAMERA, false, 0, 0, 0x14),
        ]);
    }
}

#[test]
fn gu_draw_end_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x1357_9bdf);
        for i in 0..8 {
            t.reset();
            t.packet(rng.below(40));
            let ret = rng.word();
            t.stubs(&[
                (ma::FLUSH_CACHE, false, 1, 0, 0x21),
                (ma::SCE_DMA_SEND, false, 2, 0, 0x22),
                (ma::SCE_GS_SYNC_PATH, false, 2, 0, ret),
            ]);
            let ch = rng.word();
            t.check(
                &format!("Gu_DrawEnd #{i}"),
                0x0084_79d0,
                Conv::Cdecl,
                &[ch],
                |m| Out::U32(ma::draw_end(m, ch)),
            );
        }
    });
}

#[test]
fn gu_draw_mouse_cursor_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x2468_1357);
        for i in 0..8 {
            t.reset();
            t.packet(rng.below(40));
            t.mouse(&mut rng);
            let ret = rng.word();
            t.stubs(&[
                (ma::SG_SET_DEBUG_TEX_0, false, 1, 0, 0x31),
                (ma::GU_DRAW_SPRITE, false, 9, 0, ret),
            ]);
            let what = format!("Gu_DrawMouseCursor #{i}");
            t.check(&what, 0x0084_7a40, Conv::Cdecl, &[], |m| {
                Out::U32(ma::draw_mouse_cursor(m))
            });
        }
    });
}

#[test]
fn gu_client_mouse_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0bad_cafe);
        for i in 0..60 {
            t.reset();
            t.mouse(&mut rng);
            let w = t.window(&mut rng, 0);
            t.check(
                &format!("0x00849510 #{i}"),
                0x0084_9510,
                Conv::Cdecl,
                &[w],
                |m| {
                    ma::client_mouse(m, w);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn gu_control_mouse_button_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7777_1111);
        for i in 0..60 {
            t.reset();
            t.mouse(&mut rng);
            let w = t.window(&mut rng, 0);
            let id = rng.word();
            let what = format!("Gu_ControlMouseButtonHandle #{i}");
            t.check(&what, 0x0084_b960, Conv::Cdecl, &[w, id], |m| {
                Out::U32(ma::control_mouse_button(m, w, id))
            });
        }
    });
}

#[test]
fn gu_control_mouse_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5151_aaaa);
        for i in 0..150 {
            t.reset();
            t.window_stubs();
            t.mouse(&mut rng);
            let w = t.window(&mut rng, 1);
            let c: u32 = Mem::read(&t.a, w + 0x94);
            t.check(
                &format!("0x0084b7f0 #{i}"),
                0x0084_b7f0,
                Conv::Cdecl,
                &[w, c],
                |m| Out::U32(ma::control_mouse(m, w, c)),
            );
        }
    });
}

#[test]
fn gu_window_mouse_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x4242_0042);
        for i in 0..200 {
            t.reset();
            t.window_stubs();
            t.mouse(&mut rng);
            let capture = match rng.below(4) {
                0 => 0xffff_ffff,
                1 => 0,
                _ => 0x99,
            };
            t.g32(g::CAPTURE, capture);
            let n = rng.below(6);
            let w = t.window(&mut rng, n);
            if n > 0 && rng.chance(3) {
                // the mouse on a control's left or top edge
                let c: u32 = Mem::read(&t.a, w + 0x94);
                let wx = i32::from(Mem::read::<i16>(&t.a, w + 0xc));
                let wy = i32::from(Mem::read::<i16>(&t.a, w + 0xe));
                let cx = i32::from(Mem::read::<i16>(&t.a, c));
                let cy = i32::from(Mem::read::<i16>(&t.a, c + 2));
                let (dx, dy) = (rng.below(3) as i32, rng.below(3) as i32);
                t.g32(g::MOUSE_X, (wx + cx + dx) as u32);
                t.g32(g::MOUSE_Y, (wy + cy + dy) as u32);
            }
            t.check(
                &format!("0x0084afd0 #{i}"),
                0x0084_afd0,
                Conv::Cdecl,
                &[w],
                |m| Out::U32(ma::window_mouse(m, w)),
            );
        }
    });
}

/// `n` random windows linked through `+0x04` from `g_GuWindowList` (and
/// `+0x00` alike), or none.
fn window_list(t: &mut Twins, rng: &mut Rng, n: u32) -> Vec<u32> {
    let ws: Vec<u32> = (0..n)
        .map(|_| {
            let k = rng.below(5);
            t.window(rng, k)
        })
        .collect();
    for (i, &w) in ws.iter().enumerate() {
        let next = ws.get(i + 1).copied().unwrap_or(0);
        t.w32(w, next);
        t.w32(w + 4, next);
    }
    t.g32(g::WINDOW_LIST, ws.first().copied().unwrap_or(0));
    ws
}

#[test]
fn gu_windows_mouse_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0dd0_0dd0);
        for i in 0..150 {
            t.reset();
            t.window_stubs();
            t.mouse(&mut rng);
            let capture = match rng.below(5) {
                0 => 0xffff_ffff,
                1 | 2 => 0,
                _ => 0x99,
            };
            t.g32(g::CAPTURE, capture);
            let n = rng.below(4);
            // the hit test follows +4 (Gu_ListCopyNext's copy), not +0
            for w in window_list(t, &mut rng, n) {
                t.w32(w, 0);
            }
            t.check(
                &format!("0x0084b2e0 #{i}"),
                0x0084_b2e0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(ma::windows_mouse(m)),
            );
        }
    });
}

fn pick(rng: &mut Rng, ws: &[u32]) -> u32 {
    if ws.is_empty() || rng.chance(2) {
        0
    } else {
        ws[rng.below(ws.len() as u32) as usize]
    }
}

#[test]
fn gu_vsync_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e57_7e57);
        for i in 0..200 {
            t.reset();
            t.window_stubs();
            let moved = rng.below(2);
            t.stub(ma::SG_MOUSE_GET_MOVE, false, 0, 0, moved);
            t.mouse(&mut rng);
            t.g32(g::HIDDEN, u32::from(rng.chance(4)));
            let n = rng.below(4);
            let ws = window_list(t, &mut rng, n);
            let drag = pick(&mut rng, &ws);
            let resize = pick(&mut rng, &ws);
            let popup = if rng.chance(4) {
                pick(&mut rng, &ws)
            } else {
                0
            };
            t.g32(g::DRAG_WINDOW, drag);
            t.g32(g::RESIZE_WINDOW, resize);
            t.g32(g::POPUP_WINDOW, popup);
            for k in 0..4 {
                let v = rng.below(30);
                t.g32(g::DRAG_GRAB + 4 * k, v);
            }
            let capture = match rng.below(5) {
                0 => 0xffff_ffff,
                1 => pick(&mut rng, &ws),
                _ => 0,
            };
            t.g32(g::CAPTURE, capture);
            t.check(
                &format!("Gu_VSync #{i}"),
                0x0084_a260,
                Conv::Cdecl,
                &[],
                |m| {
                    ma::vsync(m);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn gu_cursor_draw_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xc0c0_a5a5);
        for i in 0..80 {
            t.reset();
            t.packet(rng.below(40));
            let fade = rng.next() % 0x200;
            t.g32(g::FADE, fade);
            let (sw, sh, hs) = (rng.below(0x800), rng.below(0x800), rng.below(0x200));
            t.g32(g::SCREEN_W, sw);
            t.g32(g::SCREEN_H, sh);
            t.g32(g::H_SCALE, hs);
            let mut c = vec![0u8; 0x28];
            for b in &mut c {
                *b = rng.next() as u8;
            }
            if rng.chance(2) {
                c[0x24] = 0;
                c[0x25] = 0;
            }
            if rng.chance(3) {
                let head: Vec<u8> = c[0x8..0x10].to_vec();
                c[0x10..0x18].copy_from_slice(&head);
            }
            let cursor = t.alloc(&c);
            let (dx, dy) = (rng.word(), rng.word());
            let what = format!("0x00847420 #{i}");
            t.check(&what, 0x0084_7420, Conv::Cdecl, &[cursor, dx, dy], |m| {
                Out::U32(ma::cursor_draw(m, cursor, dx, dy))
            });
        }
    });
}

/// More GUI globals.
mod g2 {
    pub const PRINT_BASE_X: u32 = 0x0130_36e0;
    pub const PRINT_BASE_Y: u32 = 0x0130_36e8;
    pub const PRINT_ADVANCE: u32 = 0x0130_36f0;
    pub const PRINT_COLOR: u32 = 0x0130_370c;
    pub const PRINT_ITALIC: u32 = 0x0130_3718;
    pub const PRINT_BOLD: u32 = 0x0130_3720;
    pub const PRINT_SIZE: u32 = 0x0130_3728;
    pub const MOUSE_ENABLE: u32 = 0x0130_3760;
}

/// A NUL-terminated string in both oracles.
fn cstr(t: &mut Twins, s: &str) -> u32 {
    let mut b = s.as_bytes().to_vec();
    b.push(0);
    t.alloc(&b)
}

#[test]
fn gu_print_begin_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0f0f_1234);
        for i in 0..12 {
            t.reset();
            t.packet(rng.below(40));
            for k in 0..3 {
                let v = rng.word();
                t.g32(g2::PRINT_COLOR + 4 * k, v);
            }
            t.stub(ma::SG_SET_DEBUG_TEX_0, false, 1, 0, 0x31);
            t.check(
                &format!("Gu_PrintBegin #{i}"),
                0x0084_9170,
                Conv::Cdecl,
                &[],
                |m| Out::U32(ma::print_begin(m)),
            );
        }
    });
}

/// Texts for `Gu_PrintGetWidth`: plain, every code, and the stub's
/// continuation (`0x00848f60` / `0x00848fe0` return `next`).
const TEXTS: [&str; 12] = [
    "",
    "plain text",
    "%",
    "%x~b",
    "%~",
    "%~.abc",
    "%~b~i",
    "%~bxyz",
    "%~ihello",
    "%~c00ff00",
    "%~w12rest",
    "%~zq",
];

#[test]
fn gu_print_get_width_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7a7a_0001);
        for i in 0..60 {
            t.reset();
            let next = cstr(
                t,
                ["~b~.tail", "after", "~i", "~~x", ""][rng.below(5) as usize],
            );
            let s = TEXTS[i % TEXTS.len()];
            let text = cstr(t, s);
            t.stubs(&[
                (ma::GU_PARSE_HEX, false, 2, 0, next),
                (ma::GU_PARSE_DEC, false, 2, 0, next),
            ]);
            for (sym, v) in [
                (g2::PRINT_ADVANCE, rng.below(20)),
                (g2::PRINT_SIZE, rng.word()),
                (g2::PRINT_BOLD, rng.below(3)),
                (g2::PRINT_ITALIC, rng.below(3)),
            ] {
                t.g32(sym, v);
            }
            let what = format!("Gu_PrintGetWidth #{i} {s:?}");
            t.check_framed(&what, 0x0084_8d60, Conv::Cdecl, &[text], 0, |m, ebp| {
                Out::U32(ma::print_get_width(m, text, ebp + 8))
            });
        }
    });
}

impl Twins {
    /// Stubs for what the window drawing calls that is not pure memory
    /// code: the frame and client drawing, `Gu_Printf`, the text width
    /// (returns `width`), `Gu_FontSetupWindow`, `Gu_WindowCloseTree`.
    fn draw_stubs(&mut self, width: u32) {
        self.stubs(&[
            (ma::GU_DRAW_FRAME, false, 3, 0, 0x41),
            (ma::GU_DRAW_CLIENT, false, 3, 0, 0x42),
            (ma::GU_PRINTF, false, 1, 0, 0x43),
            (ma::GU_PRINT_GET_WIDTH, false, 1, 0, width),
            (ma::GU_FONT_SETUP_WINDOW, false, 1, 0, 0x44),
            (ma::GU_WINDOW_CLOSE_TREE, false, 1, 0, 0x45),
        ]);
    }

    /// Random print globals.
    fn print_state(&mut self, rng: &mut Rng) {
        for sym in [g2::PRINT_BASE_X, g2::PRINT_BASE_Y] {
            let v = rng.below(400);
            self.g32(sym, v);
        }
        for k in 0..3 {
            let v = rng.below(0x100);
            self.g32(g2::PRINT_COLOR + 4 * k, v);
        }
    }
}

#[test]
fn gu_draw_control_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xd1a0_c0de);
        for i in 0..200 {
            t.reset();
            let width = rng.below(200);
            t.draw_stubs(width);
            t.print_state(&mut rng);
            let w = t.window(&mut rng, 1);
            let title = cstr(t, "title");
            t.w32(w + 0x5c, title);
            let c: u32 = Mem::read(&t.a, w + 0x94);
            let label = cstr(
                t,
                ["", "OK", "Cancel", "a longer label"][rng.below(4) as usize],
            );
            t.w32(c + 0x28, label);
            let kind = rng.below(9);
            t.w32(c + 0x2c, kind | (rng.below(4) << 16));
            let flags = rng.next() & 0x1f;
            t.w32(c + 0x40, flags);
            if rng.chance(2) {
                t.write(c + 0x20, &[0, 0]);
            }
            let (dx, dy) = (rng.below(300), rng.below(300));
            let what = format!("Gu_DrawControl #{i}");
            t.check(&what, 0x0084_7720, Conv::Cdecl, &[c, dx, dy, w], |m| {
                ma::draw_control(m, c, dx, dy, w);
                Out::None
            });
        }
    });
}

#[test]
fn gu_draw_window_tree_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7ee0_7ee0);
        for i in 0..120 {
            t.reset();
            let width = rng.below(200);
            t.draw_stubs(width);
            t.print_state(&mut rng);
            t.packet(rng.below(8));
            let fade = rng.below(0x100);
            t.g32(g::FADE, fade);
            let n = rng.below(4);
            let ws = window_list(t, &mut rng, n);
            for &w in &ws {
                let title = cstr(t, "t");
                t.w32(w + 0x5c, title);
                // the target size near the size, the fade-out and growth cases
                let (cw, ch) = (rng.below(12) as u16, rng.below(12) as u16);
                t.write(w + 0x18, &cw.to_le_bytes());
                t.write(w + 0x1a, &ch.to_le_bytes());
                let (tw, th) = if rng.chance(3) {
                    (0, rng.below(12) as u16)
                } else {
                    (rng.below(12) as u16, rng.below(12) as u16)
                };
                if rng.chance(3) {
                    t.write(w + 0x18, &[7 + rng.below(3) as u8, 0]);
                }
                t.write(w + 0x20, &tw.to_le_bytes());
                t.write(w + 0x22, &th.to_le_bytes());
                let flags = rng.next() & 0xbb;
                t.w32(w + 0x88, flags);
            }
            let first = ws.first().copied().unwrap_or(0);
            let what = format!("Gu_DrawWindowTree #{i}");
            t.check(&what, 0x0084_ac80, Conv::Cdecl, &[first], |m| {
                ma::draw_window_tree(m, first);
                Out::None
            });
        }
    });
}

#[test]
fn gu_main_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x6a17_0001);
        for i in 0..40 {
            t.reset();
            t.window_stubs();
            t.draw_stubs(0x20);
            let buttons = rng.next() & 0xf_0f0f;
            let moved = rng.below(2);
            t.stubs(&[
                (ma::SG_MOUSE_BUTTON_MAIN, false, 0, 0, buttons),
                (ma::SG_MOUSE_GET_MOVE, false, 0, 0, moved),
                (ma::SEND_DEBUG_FONT, false, 0, 0, 0x51),
                (ma::GU_DRAW_START, false, 1, 0, 0x52),
                (ma::GU_SET_TEX_FLUSH, false, 0, 0, 0x53),
                (ma::GU_WINDOW_FREE_CLOSED, false, 0, 0, 0x54),
                (ma::FLUSH_CACHE, false, 1, 0, 0x21),
                (ma::SCE_DMA_SEND, false, 2, 0, 0x22),
                (ma::SCE_GS_SYNC_PATH, false, 2, 0, 0x23),
                (ma::SG_SET_DEBUG_TEX_0, false, 1, 0, 0x31),
                (ma::GU_DRAW_SPRITE, false, 9, 0, 0x32),
            ]);
            t.packet(rng.below(8));
            t.mouse(&mut rng);
            t.g32(g::HIDDEN, u32::from(rng.chance(4)));
            t.g32(g2::MOUSE_ENABLE, rng.below(2));
            t.g32(g::CAPTURE, 0);
            let n = rng.below(3);
            let ws = window_list(t, &mut rng, n);
            for &w in &ws {
                let title = cstr(t, "t");
                t.w32(w + 0x5c, title);
            }
            let (ch, buf) = (rng.word(), rng.word());
            t.check(
                &format!("Gu_Main #{i}"),
                0x0084_9850,
                Conv::Cdecl,
                &[ch, buf],
                |m| {
                    ma::gu_main(m, ch, buf);
                    Out::None
                },
            );
        }
    });
}

// ---- batch 2 (ffx_menu::m5m_b) ----

mod g3 {
    pub const VSYNC_THREAD: u32 = 0x0133_0268;
    pub const BG_THREAD_SEMA: u32 = 0x0133_0250;
    pub const MENU_TEXTURE_MANAGER: u32 = 0x00cc_c820;
}

#[test]
fn tk_set_vsync_handler_matches_the_original() {
    Twins::each(|t| {
        for (i, (thread, handler)) in [
            (u32::MAX, 0),
            (u32::MAX, 0x0040_1000),
            (7, 0),
            (7, 0x0040_1000),
            (0, 0x1234),
        ]
        .into_iter()
        .enumerate()
        {
            for clear in [false, true] {
                t.reset();
                t.stubs(&[
                    (mb::END_VSYNC_EMU_THREAD, false, 0, 0, 0x61),
                    (mb::CREATE_VSYNC_EMU_THREAD, false, 1, 0, 0x62),
                    (mb::THREAD_CREATE, false, 3, 0, 0x63),
                    (mb::THREAD_STOP, false, 1, 0, 0x64),
                    (mb::THREAD_START, false, 1, 0, 0x65),
                    (mb::THREAD_DELETE, false, 2, 0, 0x66),
                ]);
                t.g32(g3::VSYNC_THREAD, thread);
                if clear {
                    let what = format!("TkClearVsyncHandler #{i}");
                    t.check(&what, 0x0088_8830, Conv::Cdecl, &[], |m| {
                        Out::U32(mb::clear_vsync_handler(m))
                    });
                } else {
                    // the tail call's argument word is the caller's: compared
                    // through the frame (ebp + 8)
                    let what = format!("TkSetVsyncHandler #{i}");
                    t.check(&what, 0x0088_8680, Conv::Cdecl, &[handler], |m| {
                        let r = mb::set_vsync_handler(m, handler);
                        if handler != 0 {
                            let slot = m.0.stack_range().end - 0x100 - 4;
                            m.0.write(slot, &2u32.to_le_bytes()).unwrap();
                        }
                        Out::U32(r)
                    });
                }
            }
        }
    });
}

#[test]
fn tk_small_forwarders_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0002);
        for i in 0..6 {
            t.reset();
            let ret = rng.word();
            t.stubs(&[
                (mb::SCE_GS_SYNC_PATH, false, 2, 0, ret),
                (mb::SIGNAL_SEMA, false, 1, 0, ret ^ 1),
            ]);
            let (a, b) = (rng.word(), rng.word());
            let what = format!("escMenuSetAudioByConfig #{i}");
            t.check(&what, 0x0088_ec70, Conv::Cdecl, &[a, b], |m| {
                Out::U32(mb::esc_menu_set_audio_by_config(m, a, b))
            });
            let sema = rng.word();
            t.g32(g3::BG_THREAD_SEMA, sema);
            let what = format!("TkSignalSemaBGThread #{i}");
            t.check(&what, 0x0088_8820, Conv::Cdecl, &[], |m| {
                Out::U32(mb::signal_sema_bg_thread(m))
            });
            let lang = rng.next() as u8;
            t.g32(0x0133_0830, u32::from(lang));
            let what = format!("TkGetLang_thunk #{i}");
            t.check(&what, 0x0088_7c30, Conv::Cdecl, &[], |m| {
                Out::U32(ffx_menu::tk::bridge::tk_get_lang(m) as u32)
            });
        }
    });
}

#[test]
fn menu_record_inits_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0e0e_0e0e);
        for i in 0..4 {
            for multi in [false, true] {
                t.reset();
                let junk: Vec<u8> = (0..0xa00).map(|_| rng.next() as u8).collect();
                let rec = t.alloc(&junk);
                let what = format!("MenuRecord_Init multi {multi} #{i}");
                let sym = if multi { 0x0067_c030 } else { 0x0067_c2d0 };
                t.check(&what, sym, Conv::Stdcall, &[rec], |m| {
                    Out::U32(if multi {
                        mb::record_init_multi(m, rec)
                    } else {
                        mb::record_init_single(m, rec)
                    })
                });
            }
        }
    });
}

#[test]
fn menu_textures_reset_buffers_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xbeef_0101);
        for i in 0..24 {
            t.reset();
            let fresh = if rng.chance(4) { 0 } else { t.zeros(0x10) };
            t.stubs(&[
                (mb::PHYRE_MALLOC, false, 2, 0, fresh),
                (mb::PHYRE_FREE, false, 1, 0, 0x71),
                (mb::MEMSET, false, 3, 0, 0x72),
                (mb::MAP_PART_INIT_LANG_OVERRIDES, true, 0, 0, 0x73),
                (mb::MENU_TEXTURES_LOAD_NAMES, true, 0, 0, 0x74),
            ]);
            let manager = rng.word();
            t.g32(g3::MENU_TEXTURE_MANAGER, manager);
            let this = t.zeros(0x5840);
            for cap_at in [0x2bd0, 0x2bd8, 0x5828, 0x5830] {
                let cap = match rng.below(4) {
                    0 => 600,
                    1 => 800,
                    2 => 0x8000_0010,
                    _ => rng.below(1000),
                };
                let data = match rng.below(3) {
                    0 => 0,
                    1 => fresh,
                    _ => rng.word() | 1,
                };
                t.w32(this + cap_at, cap);
                t.w32(this + cap_at + 4, data);
            }
            let what = format!("0x0067c440 #{i}");
            t.check(&what, 0x0067_c440, Conv::Thiscall(this), &[], |m| {
                Out::U32(mb::reset_buffers(m, this))
            });
        }
    });
}

impl Twins {
    /// Stubs for the window constructors' callees: `SgReallocD` returns
    /// `grown`, `Gu_ControlSetText`, `Gu_WindowOpen`, `user_malloc`
    /// returns `fresh`; `memclear` runs as it is.
    fn create_stubs(&mut self, grown: u32, fresh: u32) {
        self.stubs(&[
            (mb::SG_REALLOC_D, false, 2, 0, grown),
            (mb::GU_CONTROL_SET_TEXT, false, 2, 0, 0x81),
            (mb::GU_WINDOW_OPEN, false, 1, 0, 0x82),
            (mb::USER_MALLOC, false, 1, 0, fresh),
        ]);
    }
}

#[test]
fn gu_window_add_controls_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xadd0_c0de);
        for i in 0..40 {
            t.reset();
            let n = rng.below(4);
            let w = t.window(&mut rng, n);
            // the grown array: the old controls copied, the new slot junk
            let old: u32 = Mem::read(&t.a, w + 0x94);
            let mut bytes = vec![0u8; 0x44 * (n as usize + 1)];
            Mem::read_bytes(&t.a, old, &mut bytes[..0x44 * n as usize]);
            for (k, b) in bytes.iter_mut().enumerate().skip(0x44 * n as usize) {
                *b = k as u8;
            }
            for c in bytes.chunks_mut(0x44) {
                if rng.chance(2) {
                    c[0x26] = 0;
                    c[0x27] = 0;
                }
            }
            let grown = t.alloc(&bytes);
            t.create_stubs(grown, 0);
            let args = [
                w,
                rng.word(),
                rng.word(),
                rng.word(),
                rng.word(),
                rng.word(),
                rng.word(),
            ];
            let [_, id, text, x, y, cw, ch] = args;
            match i % 4 {
                0 => t.check(
                    &format!("Gu_WindowAddControl #{i}"),
                    0x0084_bae0,
                    Conv::Cdecl,
                    &args,
                    |m| Out::U32(mb::window_add_control(m, w, id, text, x, y, cw, ch)),
                ),
                1 => t.check(
                    &format!("Gu_WindowAddButton #{i}"),
                    0x0084_a470,
                    Conv::Cdecl,
                    &args,
                    |m| Out::U32(mb::window_add_button(m, w, id, text, x, y, cw, ch)),
                ),
                2 => t.check(
                    &format!("Gu_WindowAddStatic #{i}"),
                    0x0084_a4d0,
                    Conv::Cdecl,
                    &args,
                    |m| Out::U32(mb::window_add_static(m, w, id, text, x, y, cw, ch)),
                ),
                _ => t.check(
                    &format!("Gu_WindowAddClient #{i}"),
                    0x0084_a4a0,
                    Conv::Cdecl,
                    &args[..6],
                    |m| Out::U32(mb::window_add_client(m, w, id, text, x, y, cw)),
                ),
            }
        }
    });
}

#[test]
fn gu_vsync_main_and_local_trans_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x1001_7001);
        for i in 0..20 {
            t.reset();
            let ret = rng.word();
            t.stubs(&[
                (mb::SG_MOUSE_MAIN, false, 0, 0, 0x91),
                (mb::SG_CLOCK_TICK, false, 0, 0, ret),
            ]);
            t.check(
                &format!("Gu_VSyncMain #{i}"),
                0x0084_a460,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mb::vsync_main(m)),
            );
            let fade = rng.below(0x200);
            t.g32(g::FADE, fade);
            let w = t.window(&mut rng, 0);
            let on = rng.word();
            let v = if rng.chance(2) { u32::MAX } else { rng.word() };
            let what = format!("Gu_WindowSetLocalTrans #{i}");
            t.check(&what, 0x0084_b620, Conv::Cdecl, &[w, on, v], |m| {
                Out::U32(mb::window_set_local_trans(m, w, on, v))
            });
        }
    });
}

#[test]
fn gu_window_create_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xc4ea_7e00);
        for i in 0..80 {
            t.reset();
            let n = rng.below(5);
            let ws = window_list(t, &mut rng, n);
            // some windows on the cascade's grid
            let (x0, y0) = (rng.below(3) * 16, rng.below(3) * 8);
            let reverse = rng.chance(2);
            for (k, &w) in ws.iter().enumerate() {
                if rng.chance(2) {
                    // front to back or back to front (the search restarts
                    // from the front after each hit)
                    let k = if reverse { n - 1 - k as u32 } else { k as u32 };
                    t.write(w + 0xc, &((x0 + 16 * k) as u16).to_le_bytes());
                    t.write(w + 0xe, &((y0 + 8 * k) as u16).to_le_bytes());
                }
            }
            let fresh = t.zeros(0x9c);
            let grown = t.zeros(0x44 * 4);
            t.create_stubs(grown, fresh);
            let cb = if rng.chance(3) {
                0
            } else {
                t.stub_code(0x7c, false, 3, 0, 0)
            };
            let (w, h) = (rng.below(0x220), rng.below(0x1b0));
            // the bottom edge sends the cascade to a new column at the top,
            // onto a window that is there: more hits than windows
            let (x0, y0, w, h) = if i < 8 && n >= 2 {
                t.write(ws[0] + 0xc, &[0, 0, 0, 0]);
                t.write(ws[1] + 0xc, &[0x20, 0, 0, 0]);
                (0, 0, 0x40, 0x19c)
            } else if i < 16 && n >= 1 {
                // the new column wraps back onto the same window: only the
                // hit count (one per window) ends the search
                t.write(ws[0] + 0xc, &[0, 0, 0, 0]);
                (0, 0, 0x1f0, 0x19c)
            } else {
                (x0, y0, w, h)
            };
            let flags = rng.below(16);
            let args = [x0, y0, w, h, flags, cb];
            let what = format!("Gu_WindowCreate #{i}");
            t.check(&what, 0x0084_a920, Conv::Cdecl, &args, |m| {
                Out::U32(mb::window_create(m, x0, y0, w, h, flags, cb))
            });
        }
    });
}

// ---- batch 3 (ffx_menu::m5m_c) ----

mod g4 {
    pub const FONT_COLOR: u32 = 0x0132_8a74;
    pub const FONT_POS: u32 = 0x0133_c8d4;
}

impl Twins {
    /// Stubs for the printers: `sprintf` (logging `n` words), the three
    /// font entries, `AtelGetPosWorkAdrs` returning `pos`,
    /// `Script_WorkerLevel` returning `level`, `Ch_GetFieldMode` returning
    /// `mode`, and `Math_AngleToDegrees` as `x * 57.25` from its argument.
    fn printer_stubs(&mut self, n: u32, pos: u32, level: u32, mode: u32) {
        self.stub_import(mc::SPRINTF_SLOT, false, n, 0, 0xa1);
        self.stubs(&[
            (mc::TK_PUT_DEBUG_FONT_STRING, false, 3, 0, 0xa2),
            (mc::TK_PUT_DEBUG_FONT_STRING_P, false, 3, 0, 0xa3),
            (mc::TK_PUT_DEBUG_FONT_STRING_P_EUC, false, 3, 0, 0xa4),
            (mc::ATEL_GET_POS_WORK_ADRS, false, 1, 0, pos),
            (mc::SCRIPT_WORKER_LEVEL, false, 2, 0, level),
            (mc::CH_GET_FIELD_MODE, false, 1, 0, mode),
        ]);
        let k = self.words(&[57.25f32.to_bits()]);
        let mut c = vec![0xd9, 0x44, 0x24, 0x04, 0xd8, 0x0d]; // fld [esp+4]; fmul [k]
        c.extend(k.to_le_bytes());
        c.push(0xc3);
        let f = self.cave_code(&c);
        self.jump(mc::MATH_ANGLE_TO_DEGREES, f);
    }

    /// 0x60 bytes of floats (no signalling NaNs) and `i16`s.
    fn float_block(&mut self, rng: &mut Rng) -> u32 {
        let w: Vec<u32> = (0..24)
            .map(|_| match rng.below(4) {
                0 => (rng.below(2000) as f32 - 1000.0).to_bits(),
                1 => rng.next() & 0x7fbf_ffff,
                2 => rng.below(0x1_0000),
                _ => rng.next(),
            })
            .map(|v| {
                if v & 0x7fc0_0000 == 0x7f80_0000 && v & 0x003f_ffff != 0 {
                    v | 0x0040_0000
                } else {
                    v
                }
            })
            .collect();
        self.words(&w)
    }
}

#[test]
fn tk_value_printers_match_the_original() {
    Twins::each(|t| {
        t.skip = 4;
        let mut rng = Rng(0x9e1e_7e21);
        for (addr, field) in mc_fields::PRINTERS {
            for i in 0..4 {
                t.reset();
                let pos = t.float_block(&mut rng);
                let b44 = t.float_block(&mut rng);
                let b48 = t.float_block(&mut rng);
                let level = t.words(
                    &[0; 17]
                        .iter()
                        .chain(&[b44, b48])
                        .copied()
                        .collect::<Vec<u32>>(),
                );
                let n = if matches!(field, mc_fields::Field::LevelI16(..)) {
                    3
                } else {
                    4
                };
                t.printer_stubs(n, pos, level, 0);
                let color = rng.below(8);
                t.g32(g4::FONT_COLOR, color);
                let worker = t.zeros(0x100);
                t.write(worker + 0x32, &[rng.next() as u8]);
                let what = format!("printer {addr:#x} #{i}");
                // the angle printers keep the degrees at ebp - 0x28
                let low = if matches!(field, mc_fields::Field::PosAngle(_)) {
                    0x28
                } else {
                    0x24
                };
                t.check_framed(&what, addr, Conv::Cdecl, &[worker], low, |m, ebp| {
                    Out::U32(mc::print_field(m, ebp, field, worker))
                });
            }
        }
    });
}

#[test]
fn tk_worker_printers_match_the_original() {
    Twins::each(|t| {
        t.skip = 4;
        let mut rng = Rng(0x0dd5_0001);
        for i in 0..40 {
            t.reset();
            let b44 = t.float_block(&mut rng);
            let level = t.words(
                &[0; 17]
                    .iter()
                    .chain(&[b44, 0])
                    .copied()
                    .collect::<Vec<u32>>(),
            );
            let mode = rng.below(4);
            t.printer_stubs(3, 0, level, mode);
            let color = rng.below(8);
            t.g32(g4::FONT_COLOR, color);
            let mut w = vec![0u8; 0x100];
            for b in &mut w {
                *b = rng.next() as u8;
            }
            w[0xaa] = rng.below(4) as u8;
            if rng.chance(3) {
                w[0x9c..0xa0].copy_from_slice(&[0; 4]);
            }
            let head = t.alloc(&[rng.below(3) as u8, 0, 0, 0]);
            w[0..4].copy_from_slice(&head.to_le_bytes());
            // a string table: +0x50 the offset of the offsets
            let mut tbl = vec![0u8; 0x200];
            tbl[0x50..0x54].copy_from_slice(&0x60u32.to_le_bytes());
            for k in 0..0x20usize {
                tbl[0x60 + 4 * k..0x64 + 4 * k].copy_from_slice(&(0x100 + k as u32).to_le_bytes());
            }
            let tbl = t.alloc(&tbl);
            w[8..12].copy_from_slice(&(if rng.chance(3) { 0 } else { tbl }).to_le_bytes());
            w[0x2e..0x30].copy_from_slice(&(rng.below(0x20) as u16).to_le_bytes());
            let worker = t.alloc(&w);
            let what = format!("worker printers #{i}");
            t.check_framed(
                &format!("{what} kind"),
                0x0087_ae60,
                Conv::Cdecl,
                &[worker],
                0x24,
                |m, ebp| Out::U32(mc::print_kind(m, ebp, worker)),
            );
            t.check_framed(
                &format!("{what} model"),
                0x0087_af50,
                Conv::Cdecl,
                &[worker],
                0x24,
                |m, ebp| Out::U32(mc::print_model(m, ebp, worker)),
            );
            t.check_framed(
                &format!("{what} entry"),
                0x0087_afa0,
                Conv::Cdecl,
                &[worker],
                0x24,
                |m, ebp| Out::U32(mc::print_entry(m, ebp, worker)),
            );
            t.check(
                &format!("{what} motion"),
                0x0087_b040,
                Conv::Cdecl,
                &[worker],
                |m| Out::U32(mc::print_motion(m, worker)),
            );
        }
    });
}

#[test]
fn tk_put_debug_font_entries_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xf0f0_0303);
        for i in 0..8 {
            t.reset();
            let width = rng.below(100);
            t.stubs(&[
                (mc::TK_PUT_DEBUG_FONT_CORE, false, 4, 0, rng.word()),
                (mc::TK_GET_STR_WIDTH, false, 2, 0, width),
            ]);
            let x = rng.below(400);
            t.g32(g4::FONT_POS, x);
            let (c, f, s) = (rng.word(), rng.word(), rng.word());
            t.check(
                &format!("TkPutDebugFontString #{i}"),
                0x0088_f140,
                Conv::Cdecl,
                &[c, f, s],
                |m| Out::U32(mc::put_string(m, c, f, s)),
            );
            t.check(
                &format!("TkPutDebugFontStringP #{i}"),
                0x0088_f4c0,
                Conv::Cdecl,
                &[c, f, s],
                |m| Out::U32(mc::put_string_p(m, c, f, s)),
            );
            t.check(
                &format!("TkPutDebugFontStringPR #{i}"),
                0x0088_f5c0,
                Conv::Cdecl,
                &[c, f, s],
                |m| Out::U32(mc::put_string_pr(m, c, f, s)),
            );
        }
    });
}

// ---- batch 4 (ffx_menu::m5m_d) ----

impl Twins {
    /// A `sprintf`-like recording stub (cdecl, 3 words logged) bound to
    /// import slot `slot`: copies `n` bytes from `src` to its first
    /// argument, returns `ret`.
    fn stub_import_copy(&mut self, slot: u32, src: u32, n: u32, ret: u32) {
        let log_stub = self.stub_code(slot, false, 3, 0, ret);
        let mut c = vec![0x56, 0x57]; // push esi; push edi
        c.extend([0x8b, 0x7c, 0x24, 0x0c]); // mov edi, [esp+0xc]
        c.push(0xb9);
        c.extend(n.to_le_bytes()); // mov ecx, n
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

    /// A fake CD reader table for `GetCD`: entries `+0x00` read (4
    /// words), `+0x08` sync (1), `+0x10` open (1), `+0x14` size (1,
    /// returns `size`).
    fn cd_table(&mut self, size: u32) {
        let read = self.stub_code(0xcd00, false, 4, 0, 0xc0);
        let sync = self.stub_code(0xcd08, false, 1, 0, 0xc8);
        let open = self.stub_code(0xcd10, false, 1, 0, 0xd0);
        let sz = self.stub_code(0xcd14, false, 1, 0, size);
        let table = self.words(&[read, 0, sync, 0, open, sz]);
        self.stubs(&[
            (md::GET_CD, false, 0, 0, table),
            (md::FLUSH_CACHE, false, 1, 0, 0xf1),
        ]);
    }
}

#[test]
fn tk_cd_entries_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xcd00_cd00);
        for i in 0..12 {
            t.reset();
            let size = match i % 4 {
                0 => 0,
                1 => 0x800,
                2 => 0xffff_f000 | rng.below(0x1000),
                _ => rng.below(0x10_0000),
            };
            t.cd_table(size);
            let fresh = t.words(&[0xdead_0001, 2, 3, 4]);
            t.stubs(&[
                (md::SG_MEM_SET_ALIGN, false, 1, 0, 0xe1),
                (md::USER_MALLOC, false, 1, 0, fresh),
            ]);
            let (f, s, b) = (rng.word(), rng.word(), rng.word());
            t.check(
                &format!("TkCdRead #{i}"),
                0x0088_d140,
                Conv::Cdecl,
                &[f, s, b],
                |m| Out::U32(md::cd_read(m, f, s, b)),
            );
            t.check(
                &format!("TkCdReadSync #{i}"),
                0x0088_d1c0,
                Conv::Cdecl,
                &[f],
                |m| Out::U32(md::cd_read_sync(m, f)),
            );
            t.check(
                &format!("TkCdReadNB #{i}"),
                0x0088_d190,
                Conv::Cdecl,
                &[f, s, b],
                |m| Out::U32(md::cd_read_nb(m, f, s, b)),
            );
            t.check(
                &format!("TkCdSize #{i}"),
                0x0088_d390,
                Conv::Cdecl,
                &[f, s],
                |m| Out::U32(md::cd_size(m, f, s)),
            );
            t.check(
                &format!("TkCdReadWithMalloc #{i}"),
                0x0088_d1d0,
                Conv::Cdecl,
                &[f, s],
                |m| Out::U32(md::cd_read_with_malloc(m, f, s)),
            );
        }
    });
}

#[test]
fn tk_platform_glue_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e1e_0004);
        for i in 0..8 {
            t.reset();
            let ret = rng.word();
            t.stubs(&[
                (md::SG_MEM_CHECK, false, 3, 0, ret),
                (md::POST_RESET, false, 1, 0, 0x11),
                (md::GRAPHIC_SET_SCREEN_FADE_COLOR, false, 4, 0, 0x12),
                (md::GRAPHIC_SET_SCREEN_FADE_LAST_REASON, false, 1, 0, 0x13),
                (md::GRAPHIC_LOCK_SCREEN_FADE_COLOR, false, 0, 0, ret ^ 2),
                (md::TK_SYNC_V_DI, false, 0, 0, 0x14),
                (md::TK_SYNC_V_EI, false, 0, 0, 0x15),
                (md::TK_SET_VSYNC_HANDLER, false, 1, 0, ret ^ 3),
                (md::SCE_PAD_INFO_ACT, false, 4, 0, rng.below(2)),
            ]);
            t.check(
                &format!("TkMemLock #{i}"),
                0x0088_e680,
                Conv::Cdecl,
                &[],
                |m| Out::U32(md::mem_check_pair(m, 0x1c5)),
            );
            t.check(
                &format!("TkMemUnlock #{i}"),
                0x0088_e6b0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(md::mem_check_pair(m, 0x1d0)),
            );
            t.check(
                &format!("TkSoftWareReset #{i}"),
                0x0087_2550,
                Conv::Cdecl,
                &[],
                |m| Out::U32(md::software_reset(m)),
            );
            let (thread, count) = (rng.word(), rng.word());
            t.g32(0x0133_0268, thread);
            t.g32(0x0133_0254, count);
            t.check(
                &format!("TkInitAllSyncVCallback #{i}"),
                0x0088_86f0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(md::init_all_sync_v_callback(m)),
            );
            let act = if rng.chance(3) { 0xff } else { rng.word() };
            let (p, s) = (rng.word(), rng.word());
            t.check(
                &format!("TkPadInfoAct #{i}"),
                0x0088_8a40,
                Conv::Cdecl,
                &[p, s, act],
                |m| Out::U32(md::pad_info_act(m, p, s, act)),
            );
        }
    });
}

#[test]
fn tk_mc_sync_matches_the_original() {
    Twins::each(|t| {
        t.skip = 0;
        let mut rng = Rng(0x3c5c_0001);
        for i in 0..60 {
            t.reset();
            let r = if rng.chance(2) { 1 } else { rng.word() };
            t.stubs(&[
                (md::TK_GET_TGS, false, 0, 0, u32::from(rng.chance(5))),
                (
                    md::TK_MC_GET_LOAD_FILE_POS,
                    false,
                    1,
                    0,
                    [5, 0, 0xffff_fff0][rng.below(3) as usize],
                ),
                (md::TK_MC_SET_LOAD_FILE_POS, false, 2, 0, 0x16),
            ]);
            // sceMcSync writing its two out words: cmd 1 or other, result
            let cmd = if rng.chance(2) { 1 } else { rng.below(4) };
            let res = if rng.chance(2) {
                0xffff_fffe
            } else {
                rng.below(4)
            };
            let log = t.stub_code(md::SCE_MC_SYNC, false, 3, 0, r);
            let mut c = vec![0x8b, 0x44, 0x24, 0x08]; // mov eax, [esp+8]
            c.extend([0xc7, 0x00]);
            c.extend(cmd.to_le_bytes()); // mov dword [eax], cmd
            c.extend([0x8b, 0x44, 0x24, 0x0c]); // mov eax, [esp+0xc]
            c.extend([0xc7, 0x00]);
            c.extend(res.to_le_bytes()); // mov dword [eax], res
            c.push(0xe9);
            let here = t.cave_next + c.len() as u32 + 4;
            c.extend(log.wrapping_sub(here).to_le_bytes()); // jmp log
            let f = t.cave_code(&c);
            t.jump(md::SCE_MC_SYNC, f);
            let slot = rng.below(2);
            t.g32(0x0133_0234, slot);
            let outs = t.zeros(8);
            let (co, ro) = (
                if rng.chance(3) { 0 } else { outs },
                if rng.chance(3) { 0 } else { outs + 4 },
            );
            let mode = rng.word();
            let what = format!("TkMcSync #{i}");
            t.check_framed(
                &what,
                0x0088_84e0,
                Conv::Cdecl,
                &[mode, co, ro],
                0xc,
                |m, ebp| Out::U32(md::mc_sync(m, ebp, mode, co, ro)),
            );
        }
    });
}

#[test]
fn tk_mc_make_path_matches_the_original() {
    Twins::each(|t| {
        t.skip = 0;
        let mut rng = Rng(0x9a7b_0001);
        for i in 0..20 {
            t.reset();
            // sprintf as a copy of the format into the buffer (logged)
            let text: Vec<u8> = (0..rng.below(300))
                .map(|_| b'a' + rng.below(26) as u8)
                .collect();
            let mut src = text.clone();
            src.push(0);
            let src = t.alloc(&src);
            t.stub_import_copy(md::SPRINTF_SLOT, src, text.len() as u32 + 1, 0x99);
            let dst = t.zeros(0x200);
            let name = rng.word();
            let what = format!("TkMcMakePath #{i}");
            t.check_framed(
                &what,
                0x0088_8030,
                Conv::Cdecl,
                &[dst, name],
                0x400,
                |m, ebp| Out::U32(md::mc_make_path(m, ebp, dst, name)),
            );
        }
    });
}

#[test]
fn tk_deci2_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xdec1_0002);
        for i in 0..24 {
            t.reset();
            let socket = rng.word();
            t.stubs(&[
                (md::SCE_DECI2_OPEN, false, 3, 0, socket),
                (
                    md::SCE_DECI2_REQ_SEND,
                    false,
                    2,
                    0,
                    if rng.chance(2) { 0 } else { 0xffff_ffff },
                ),
                (md::FLUSH_CACHE, false, 1, 0, 0xf1),
                (md::TK_IS_DEBUG, false, 0, 0, rng.below(2)),
                (md::TK_IS_DECI2_SEND, false, 0, 0, 0),
            ]);
            for (k, sym) in [0x0132_f578u32, 0x0132_f584, 0x0132_f588, 0x0132_d56c]
                .into_iter()
                .enumerate()
            {
                t.g32(sym, 0x100 + k as u32);
            }
            t.check(
                &format!("TkInitDeci2 #{i}"),
                0x0088_5240,
                Conv::Cdecl,
                &[],
                |m| Out::U32(md::init_deci2(m)),
            );
            let packet = t.words(&[rng.word(), rng.word(), 0]);
            let len = rng.word();
            t.check(
                &format!("TkDeci2Send #{i}"),
                0x0088_54c0,
                Conv::Cdecl,
                &[packet, len],
                |m| Out::U32(md::deci2_send(m, packet, len)),
            );
            let name: Vec<u8> = (0..rng.below(40))
                .map(|_| b'a' + rng.below(26) as u8)
                .chain([0])
                .collect();
            let name = t.alloc(&name);
            t.check(
                &format!("TkRemove #{i}"),
                0x0088_c740,
                Conv::Cdecl,
                &[name],
                |m| Out::U32(md::remove(m, name)),
            );
        }
    });
}

// ---- batch 5 (ffx_menu::m5m_e) ----

impl Twins {
    /// A selector of `n` items (names "item<k>" in a random order, values
    /// k), `rows` visible, scrolled by `scroll`.
    fn selector(&mut self, rng: &mut Rng, n: u32, rows: u32, scroll: u16) -> u32 {
        // eight more names on both sides: rows a scroll past either end
        // shows (more rows than items scroll to a negative offset)
        let names: Vec<u32> = (0..n + 16)
            .map(|_| {
                let s = format!("item{}\0", rng.below(50));
                self.alloc(s.as_bytes())
            })
            .collect();
        let names = self.words(&names) + 32;
        let values: Vec<u32> = (0..n.max(1)).collect();
        let values = self.words(&values);
        let mut s = vec![0u8; 0x30];
        s[0..2].copy_from_slice(&scroll.to_le_bytes());
        s[2..4].copy_from_slice(&(rng.below(8) as u16).to_le_bytes());
        s[4..6].copy_from_slice(&(n as u16).to_le_bytes());
        s[8..12].copy_from_slice(&names.to_le_bytes());
        s[0xc..0x10].copy_from_slice(&values.to_le_bytes());
        s[0x10..0x14].copy_from_slice(&rows.to_le_bytes());
        s[0x14..0x18].copy_from_slice(&rng.below(2).to_le_bytes());
        s[0x18..0x20].copy_from_slice(&[7, 0, 0, 0, 9, 0, 0, 0]);
        self.alloc(&s)
    }
}

#[test]
fn gu_selector_paint_and_sort_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5e1e_c7ed);
        for i in 0..80 {
            t.reset();
            t.stubs(&[
                (me::GU_PRINT_SET_COLOR, false, 4, 0, 0xb1),
                (me::GU_PRINT_SET_X, false, 1, 0, 0xb2),
                (me::GU_PRINT_SET_Y, false, 1, 0, 0xb3),
                (me::GU_PRINTF, false, 1, 0, 0xb4),
            ]);
            t.mouse(&mut rng);
            let my = rng.below(200);
            t.g32(g::MOUSE_Y, my.wrapping_sub(40));
            let n = rng.below(12);
            let rows = rng.below(n + 3);
            let scroll = (rng.below(120) as u16).wrapping_sub(20);
            let scroll = if rng.chance(8) { 0xffff } else { scroll };
            let sel = t.selector(&mut rng, n, rows, scroll);
            let w = t.window(&mut rng, 0);
            let wy = i32::from(Mem::read::<i16>(&t.a, w + 0xe));
            // the mouse exactly one list height below the window top, or
            // one pixel in
            let my = match rng.below(16) {
                0..=3 => Some(wy + 9 * rows as i32),
                4..=6 => Some(wy + 1),
                _ => None,
            };
            if let Some(my) = my {
                t.g32(g::MOUSE_Y, my as u32);
            }
            t.check(
                &format!("Gu_SelectorPaint #{i}"),
                0x0084_9c00,
                Conv::Cdecl,
                &[w, sel],
                |m| {
                    me::selector_paint(m, w, sel);
                    Out::None
                },
            );
            t.check(
                &format!("Gu_SelectorSortItem #{i}"),
                0x0084_a180,
                Conv::Cdecl,
                &[sel],
                |m| {
                    me::selector_sort_item(m, sel);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn gu_selector_procs_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x9e0c_0005);
        for i in 0..120 {
            t.reset();
            t.window_stubs();
            t.stubs(&[
                (me::GU_PRINT_SET_COLOR, false, 4, 0, 0xb1),
                (me::GU_PRINT_SET_X, false, 1, 0, 0xb2),
                (me::GU_PRINT_SET_Y, false, 1, 0, 0xb3),
                (me::GU_PRINTF, false, 1, 0, 0xb4),
                (me::SG_SET_MOUSE, false, 2, 0, 0xb5),
                (me::GU_WINDOW_CLOSE_TREE, false, 1, 0, 0xb6),
                (me::GU_WINDOW_MOUSE_IN, false, 1, 0, rng.below(2)),
            ]);
            t.mouse(&mut rng);
            let n = 3 + rng.below(4);
            let sel = t.selector(&mut rng, n, 3, 0);
            let owner = t.window(&mut rng, 0);
            t.w32(sel + 0x20, owner);
            t.w32(sel + 0x24, rng.word());
            if rng.chance(2) {
                let cb = t.stub_code(0x28, false, 3, 0, 0);
                t.w32(sel + 0x28, cb);
            }
            let w = t.window(&mut rng, 0);
            t.w32(w + 0x98, sel);
            let rect: Vec<u8> = (0..0x10).map(|_| rng.next() as u8).collect();
            t.write(w + 0x14, &rect);
            let popup = rng.word();
            t.g32(g::POPUP_WINDOW, popup);
            let msg = [
                0x8001, 0x8003, 0x8006, 0x8006, 0x800a, 0x8011, 0x8004, 0x8007,
            ][rng.below(8) as usize];
            let arg = [u32::MAX, u32::MAX, 0x4d2, 3][rng.below(4) as usize];
            t.check(
                &format!("Gu_SelectorProc #{i}"),
                0x0084_9e30,
                Conv::Cdecl,
                &[w, msg, arg],
                |m| Out::U32(me::selector_proc(m, w, msg, arg)),
            );
            t.check(
                &format!("Gu_MessageBoxProc #{i}"),
                0x0084_9a10,
                Conv::Cdecl,
                &[w, msg, arg],
                |m| Out::U32(me::message_box_proc(m, w, msg, arg)),
            );
        }
    });
}

#[test]
fn gu_selector_open_and_message_box_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0be7_0005);
        for i in 0..24 {
            t.reset();
            let fresh = t.zeros(0x9c);
            let grown = t.zeros(0x44 * 4);
            t.create_stubs(grown, fresh);
            let width = if rng.chance(4) {
                0x7ff0 + rng.below(0x40)
            } else {
                rng.below(0x240)
            };
            t.stubs(&[
                (me::GU_SELECTOR_FIT_WINDOW, false, 2, 0, rng.word()),
                (me::GU_PRINT_GET_WIDTH, false, 1, 0, width),
            ]);
            t.mouse(&mut rng);
            t.g32(g::WINDOW_LIST, 0);
            let sel = t.selector(&mut rng, 3, 3, 0);
            let (owner, arg) = (rng.word(), rng.word());
            match i % 3 {
                0 => t.check(
                    &format!("Gu_SelectorOpen #{i}"),
                    0x0084_9d60,
                    Conv::Cdecl,
                    &[sel, owner, arg],
                    |m| Out::U32(me::selector_open(m, sel, owner, arg)),
                ),
                1 => {
                    let (x, y, w, h) = (
                        rng.below(300),
                        rng.below(300),
                        rng.below(200),
                        rng.below(200),
                    );
                    let args = [sel, owner, arg, x, y, w, h];
                    t.check(
                        &format!("Gu_SelectorOpenXYWH #{i}"),
                        0x0084_9dc0,
                        Conv::Cdecl,
                        &args,
                        |m| Out::U32(me::selector_open_xywh(m, sel, owner, arg, x, y, w, h)),
                    );
                }
                _ => {
                    let text = rng.word();
                    t.check(
                        &format!("Gu_MessageBox #{i}"),
                        0x0084_9970,
                        Conv::Cdecl,
                        &[text],
                        |m| Out::U32(me::message_box(m, text)),
                    );
                }
            }
        }
    });
}

#[test]
fn gu_make_file_list_matches_the_original() {
    Twins::each(|t| {
        t.skip = 4;
        let mut rng = Rng(0xf11e_0005);
        for i in 0..20 {
            t.reset();
            let path: Vec<u8> = (0..rng.below(80))
                .map(|_| b"a/\\x."[rng.below(5) as usize])
                .chain([0])
                .collect();
            let path = t.alloc(&path);
            t.stub_import(me::SPRINTF_SLOT, false, 4, 0, 0xc1);
            t.stub(me::SG_MAKE_FILELIST, false, 3, 0, rng.word());
            let list = rng.word();
            let what = format!("Gu_MakeFileList #{i}");
            t.check_framed(
                &what,
                0x0084_98e0,
                Conv::Cdecl,
                &[list, path],
                0x204,
                |m, ebp| Out::U32(me::make_file_list(m, ebp, list, path)),
            );
        }
    });
}

// ---- batch 6 (ffx_menu::m5m_f) ----

/// A float word for the rectangle tests: ordinary values, no NaNs.
fn ui_float(rng: &mut Rng) -> u32 {
    match rng.below(3) {
        0 => (rng.below(1000) as f32 / 3.0).to_bits(),
        1 => (rng.next() & 0x3fff_ffff) | (rng.below(2) << 31),
        _ => ((rng.below(4000) as f32) - 2000.0).to_bits(),
    }
}

#[test]
fn tk_menu_packet_wrappers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7a1e_0006);
        for i in 0..12 {
            t.reset();
            let ret = rng.word();
            t.stubs(&[
                (mf::PKT_SP3R_008EBBD0, false, 4, 0, ret),
                (mf::TO_GET_WIDTH_EXT_MES_1612, false, 2, 0, ret ^ 1),
                (mf::PKT_MES_008E9FD0, false, 4, 0, ret ^ 2),
                (mf::PKT_MES_MACRO_008EA5C0, false, 5, 0, ret ^ 3),
                (mf::PKT_WINDOW_008EADF0, false, 5, 0, ret ^ 4),
                (mf::TK_MENU_DRAW_SCISSOR_RECT, false, 4, 0, 0x55),
                (mf::DRAW_CROSS_BUTTON_ICON_RGBA, false, 7, 0, ret ^ 5),
            ]);
            let p = rng.word();
            t.g32(0x0187_1530, p);
            let (a, b, c, d) = (rng.word(), rng.word(), rng.word(), rng.word());
            // around the clip limits 36 x 10
            let (w, ht) = match i {
                0 => (0x24, 0xb),
                1 => (0x25, 0xa),
                2 => (0x25, 0xb),
                _ => (0x22 + rng.below(5), 8 + rng.below(5)),
            };
            t.check(
                &format!("Sp3R0 #{i}"),
                0x008d_ca60,
                Conv::Cdecl,
                &[a, b, c],
                |m| Out::U32(mf::draw_sp3_r0(m, a, b, c)),
            );
            t.check(
                &format!("WidthHeight #{i}"),
                0x008d_c9f0,
                Conv::Cdecl,
                &[a, b],
                |m| Out::U32(mf::draw_1612_width_height(m, a, b)),
            );
            t.check(
                &format!("EasyMes #{i}"),
                0x008d_cb80,
                Conv::Cdecl,
                &[a, b, c],
                |m| Out::U32(mf::draw_easy_mes_1612(m, a, b, c)),
            );
            t.check(
                &format!("EasyMesMacro #{i}"),
                0x008d_cc70,
                Conv::Cdecl,
                &[a, b, c, d],
                |m| Out::U32(mf::draw_easy_mes_1612_with_macro(m, a, b, c, d)),
            );
            t.check(
                &format!("Window1612 #{i}"),
                0x008d_cf40,
                Conv::Cdecl,
                &[a, b, w, ht],
                |m| Out::U32(mf::draw_window_1612(m, a, b, w, ht)),
            );
            let (x, y, fw, fh) = (
                ui_float(&mut rng),
                ui_float(&mut rng),
                ui_float(&mut rng),
                ui_float(&mut rng),
            );
            t.check(
                &format!("DrawCrossButtonIcon #{i}"),
                0x008e_6030,
                Conv::Cdecl,
                &[a, x, y, fw, fh],
                |m| Out::U32(mf::draw_cross_button_icon(m, a, x, y, fw, fh)),
            );
        }
    });
}

#[test]
fn tk_ui_rectangles_match_the_original() {
    Twins::each(|t| {
        t.skip = 4;
        let mut rng = Rng(0x0e1e_0006);
        for i in 0..30 {
            t.reset();
            let ret = rng.word();
            t.stubs(&[
                (mf::GRAPHIC_DRAW_UI_ELEMENT, false, 5, 0, ret),
                (mf::GRAPHIC_GET_ABMAP, false, 0, 0, rng.below(3)),
            ]);
            let (c0, c1) = (rng.word(), rng.word());
            let (x, y, w, h) = (
                ui_float(&mut rng),
                ui_float(&mut rng),
                ui_float(&mut rng),
                ui_float(&mut rng),
            );
            let args = [x, y, w, h, c0, c1];
            t.check_framed(
                &format!("TkMn2DrawCrossRect #{i}"),
                0x008c_0930,
                Conv::Cdecl,
                &args,
                0x9c,
                |m, ebp| Out::U32(mf::draw_cross_rect(m, ebp, x, y, w, h, c0, c1)),
            );
            let (x, y, w, h) = (rng.word(), rng.word(), rng.word(), rng.word());
            let args = [x, y, w, h, c0, c1];
            t.check_framed(
                &format!("TkMn2DrawRect #{i}"),
                0x008c_1280,
                Conv::Cdecl,
                &args,
                0xa0,
                |m, ebp| Out::U32(mf::draw_rect(m, ebp, x, y, w, h, c0, c1)),
            );
            t.check_framed(
                &format!("DrawBlackBackGround #{i}"),
                0x008d_b870,
                Conv::Cdecl,
                &[],
                0x9c,
                |m, ebp| Out::U32(mf::draw_black_background(m, ebp)),
            );
        }
    });
}

#[test]
fn gui_eve_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x9e7e_0006);
        for i in 0..8 {
            t.reset();
            let ret = rng.word();
            t.stubs(&[
                (mf::DBG_PRINTF, false, 1, 0, 0x61),
                (mf::SG_GET_RESOURCE, false, 2, 0, ret),
                (mf::SG_SET_RESOURCE, false, 2, 0, 0x62),
                (mf::SG_SET_RESOURCE_STR, false, 2, 0, 0x63),
                (mf::GU_SELECTOR_GET_ITEM_VALUE, false, 1, 0, ret ^ 7),
                (mf::GU_SELECTOR_GET_ITEM_NAME, false, 1, 0, ret ^ 8),
                (mf::GU_WINDOW_CONTROL_SET_TEXT, false, 3, 0, 0x64),
                (mf::ATEL_SET_EVENT_JUMP, false, 2, 0, ret ^ 9),
            ]);
            t.check(
                &format!("gui_eve_Init #{i}"),
                0x0085_4e60,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mf::eve_init(m)),
            );
            let sel = rng.word();
            t.g32(0x0132_592c, sel);
            let w = rng.word();
            let msg = if rng.chance(2) { 0x2c } else { rng.below(0x40) };
            t.check(
                &format!("gui_eve_GWM_SELECTOR #{i}"),
                0x0085_4df0,
                Conv::Cdecl,
                &[w, msg],
                |m| match mf::eve_gwm_selector(m, w, msg) {
                    Some(v) => Out::U32(v),
                    None => Out::None,
                },
            );
        }
    });
}

#[test]
fn tk_mn2_draw_screen_strips_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5c4e_0006);
        for i in 0..4 {
            t.reset();
            let ret = rng.word();
            t.stubs(&[
                (mf::TK_MENU_DRAW_KICK, false, 0, 0, 0x71),
                (mf::TK_MN2_DRAW_INIT_PACKET, false, 0, 0, 0x73),
                (mf::TK_MN2_DRAW_KICK_SYNC_PACKET, false, 0, 0, 0x74),
                (mf::MENU_MN2_STEP_DRAW_Y80, false, 4, 0, 0x75),
                (mf::SCE_GS_SYNC_PATH, false, 2, 0, ret),
                (mf::SCE_GS_SET_DEF_LOAD_IMAGE, false, 8, 0, 0x76),
                (mf::SCE_GS_EXEC_LOAD_IMAGE, false, 2, 0, 0x77),
                (mf::FLUSH_CACHE, false, 1, 0, 0x78),
            ]);
            // the VU1 thunk is a bare jmp (not a function in the dump)
            let vu1 = t.stub_code(mf::TK_VU1_SYNC_PATH_THUNK, false, 0, 0, 0x72);
            t.jump(mf::TK_VU1_SYNC_PATH_THUNK, vu1);
            let (a, b, y) = (rng.word(), rng.word(), rng.word());
            let what = format!("TkMn2DrawScreenStrips #{i}");
            t.check(&what, 0x008a_9510, Conv::Cdecl, &[a, b, y], |m| {
                Out::U32(mf::draw_screen_strips(m, a, b, y))
            });
        }
    });
}

// ---- batch 7 (ffx_menu::m5m_g) ----

#[test]
fn tk_put_debug_font_string_p_euc_matches_the_original() {
    Twins::each(|t| {
        t.skip = 4;
        let mut rng = Rng(0xe0c0_0007);
        for i in 0..60 {
            t.reset();
            let ret = rng.word();
            // the core logs its words and copies each piece (32 bytes)
            // into a list, so every piece is compared, not only the last
            let log_stub = t.stub_code(mg::TK_PUT_DEBUG_FONT_CORE, false, 4, 0, ret);
            let pieces = t.zeros(32 * 8);
            let cursor = t.words(&[pieces]);
            let mut c = vec![0x56, 0x57]; // push esi; push edi
            c.extend([0x8b, 0x74, 0x24, 0x18]); // mov esi, [esp+0x18]
            c.extend([0x8b, 0x3d]);
            c.extend(cursor.to_le_bytes()); // mov edi, [cursor]
            c.extend([0xb9, 32, 0, 0, 0]); // mov ecx, 32
            c.extend([0xf3, 0xa4]); // rep movsb
            c.extend([0x89, 0x3d]);
            c.extend(cursor.to_le_bytes()); // mov [cursor], edi
            c.extend([0x5f, 0x5e]); // pop edi; pop esi
            c.push(0xe9);
            let here = t.cave_next + c.len() as u32 + 4;
            c.extend(log_stub.wrapping_sub(here).to_le_bytes()); // jmp log_stub
            let f = t.cave_code(&c);
            t.jump(mg::TK_PUT_DEBUG_FONT_CORE, f);
            let len = rng.below(90);
            let mut text: Vec<u8> = (0..len)
                .map(|_| match rng.below(3) {
                    0 => [b'a' + rng.below(26) as u8, 0x7f][rng.below(2) as usize],
                    _ => 0x80 | rng.next() as u8,
                })
                .collect();
            text.extend([0, 0]);
            let text = if i % 10 == 9 { 0 } else { t.alloc(&text) };
            let (c, f) = (rng.word(), rng.word());
            let what = format!("TkPutDebugFontStringPEuc #{i}");
            t.check_framed(
                &what,
                0x0088_f4e0,
                Conv::Cdecl,
                &[c, f, text],
                0x2c,
                |m, ebp| Out::U32(mg::put_string_p_euc(m, ebp, c, f, text)),
            );
        }
    });
}

#[test]
fn gui_eve_buttonup_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xb07a_0007);
        for i in 0..20 {
            t.reset();
            let n = rng.below(6);
            // a negative size (a broken file) rounds towards zero
            let size = if i % 7 == 6 {
                0u32.wrapping_sub(rng.below(20))
            } else {
                n * 8 + rng.below(8)
            };
            let mut file = vec![size.to_le_bytes()];
            for k in 0..n {
                file.push((0x40 + 8 * k).to_le_bytes());
                file.push(0u32.to_le_bytes());
            }
            let file: Vec<u8> = file.concat();
            let ids = if i % 5 == 4 { 0 } else { t.alloc(&file) };
            let sel = rng.word();
            let ret = rng.word();
            t.stubs(&[
                (mg::USER_FREE, false, 1, 0, 0x81),
                (mg::SG_PC_READ, false, 1, 0, ids),
                (mg::SG_GET_RESOURCE_STR, false, 1, 0, 0x82),
                (mg::GU_SELECTOR_ALLOCATE, false, 0, 0, sel),
                (mg::GU_SELECTOR_RESET_NB_ITEM, false, 2, 0, 0x83),
                (mg::GU_SELECTOR_SET_ITEM, false, 4, 0, 0x84),
                (mg::GU_SELECTOR_SET_SELECT_STR, false, 2, 0, 0x85),
                (mg::GU_SELECTOR_SORT_ITEM, false, 1, 0, 0x86),
                (mg::GU_SELECTOR_OPEN_XYWH, false, 7, 0, ret),
            ]);
            let old = if rng.chance(2) { 0 } else { rng.word() };
            t.g32(0x0132_5924, old);
            let w = rng.word();
            t.check(
                &format!("gui_eve_GWM_BUTTONUP #{i}"),
                0x0085_4e80,
                Conv::Cdecl,
                &[w],
                |m| Out::U32(mg::eve_gwm_buttonup(m, w)),
            );
        }
    });
}

// ---- round 3, batch 1 (ffx_menu::m5m_h) ----

impl Twins {
    /// A recording stub over `sym` (cdecl, `n` words) that returns
    /// `value` in `ST0` (`fld dword`) after logging.
    fn stub_float_logged(&mut self, sym: u32, n: u32, value: f32) {
        let cell = self.words(&[value.to_bits()]);
        let addr = self.stub_code(sym, false, n, 0, 0);
        // the final `ret` becomes `fld dword [cell]; ret`
        self.cave_next -= 1;
        let mut c = vec![0xd9, 0x05];
        c.extend(cell.to_le_bytes());
        c.push(0xc3);
        self.cave_code(&c);
        self.jump(sym, addr);
    }
}

/// The 5-byte jump thunks: each target stubbed (logging the words the
/// thunk forwards); the thunk's `eax` is the target's.
#[test]
fn r3_jump_thunks_match_the_original() {
    type Bridge = fn(&mut G, &[u32]) -> u32;
    let list: &[(u32, u32, u32, Bridge)] = &[
        (0x0088_d500, mh::ATEL_DRAW_SEMI_TRANS, 0, |m, _| {
            mh::atel_draw_semi_trans_thunk(m)
        }),
        (0x0088_7ae0, mh::TK_SND_IS_READ, 0, |m, _| {
            mh::tk_snd_is_read_thunk(m)
        }),
        (0x0088_7c30, mh::TK_GET_LANG, 0, |m, _| {
            mh::tk_get_lang_thunk(m)
        }),
        (0x0086_c1f0, mh::TK_GET_NOW_EVENT_JUMP_MAP_NO, 0, |m, _| {
            mh::tk_get_now_event_jump_map_no_thunk(m)
        }),
        (0x0086_c1d0, mh::TK_GET_NOW_EVENT_JUMP_INDEX, 0, |m, _| {
            mh::tk_get_now_event_jump_index_thunk(m)
        }),
        (0x0088_c830, mh::MS_GET_SAVE_EVENT_ADDRESS, 0, |m, _| {
            mh::tk_get_save_event_address(m)
        }),
        (0x0088_e780, mh::MS_IMPORTANT_RESET, 1, |m, a| {
            mh::ms_important_reset_thunk(m, a[0])
        }),
        (0x008c_1790, mh::TK_VU1_SYNC_PATH, 0, |m, _| {
            mh::tk_vu1_sync_path_thunk(m)
        }),
        (0x008a_98b0, mh::MS_GET_GIL, 0, |m, _| {
            mh::ms_get_gil_thunk(m)
        }),
        (0x008b_da30, mh::MS_GET_GIL, 0, |m, _| {
            mh::ms_get_gil_thunk(m)
        }),
        (0x008c_1390, mh::TO_DRAW_SCISSOR_XYWH, 4, |m, a| {
            mh::draw_scissor_rect_thunk(m, a[0], a[1], a[2], a[3])
        }),
        (
            0x008c_1b10,
            mh::TO_MENU_GET_ABILITYMAP_WORK_ADRS,
            0,
            |m, _| mh::get_abilitymap_work_thunk(m),
        ),
        (0x008c_2380, mh::TK_MN2_BUILD_ITEM_LIST, 2, |m, a| {
            mh::build_item_list_thunk(m, a[0], a[1])
        }),
        (0x008c_30d0, mh::MS_SET_SAVE_PARAM_ALL, 0, |m, _| {
            mh::tk_ms_set_save_param_all(m)
        }),
    ];
    Twins::each(|t| {
        let mut rng = Rng(0x7e3a_0001);
        for &(thunk, target, n, bridge) in list {
            for i in 0..3 {
                t.reset();
                let ret = rng.word();
                t.stub(target, false, n, 0, ret);
                let args: Vec<u32> = (0..n).map(|_| rng.word()).collect();
                let what = format!("thunk {thunk:#x} #{i}");
                t.check(&what, thunk, Conv::Cdecl, &args, |m| {
                    Out::U32(bridge(m, &args))
                });
            }
        }
    });
}

#[test]
fn r3_constants_and_flags_match_the_original() {
    Twins::each(|t| {
        t.reset();
        t.check("0x0087aa30", 0x0087_aa30, Conv::Cdecl, &[], |m| {
            Out::U32(mh::vibrate_curve_0087aa30(m))
        });
        t.check("0x008a9850", 0x008a_9850, Conv::Cdecl, &[], |m| {
            Out::U32(mh::vibrate_curve_008a9850(m))
        });
        for (sym, on) in [(0x008c_3380, 1), (0x008c_33e0, 0)] {
            for old in [0, 1, 0xdead_0001] {
                t.g32(0x0187_1538, old);
                t.check(&format!("help flag {sym:#x}"), sym, Conv::Cdecl, &[], |m| {
                    mh::set_help_flag(m, on);
                    Out::None
                });
            }
        }
    });
}

#[test]
fn r3_set_near_plane_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e3a_0002);
        let values = [
            0f32.to_bits(),
            1f32.to_bits(),
            (-0.5f32).to_bits(),
            (-0.0f32).to_bits(),
            f32::INFINITY.to_bits(),
            0x7fc0_1234, // a quiet NaN (signalling NaNs are left out)
            0x0000_0001, // a denormal
        ];
        for (i, &near) in values.iter().enumerate() {
            t.reset();
            let ret = rng.word();
            // a camera of 0xc0 bytes whose vtable's slot 1 logs `ecx`
            let fake = t.object(&[(0, 0x11), (0, ret)]);
            let vt = t.r32(fake);
            let camera = t.zeros(0xc0);
            t.w32(camera, vt);
            let scene = t.zeros(0x100);
            t.w32(scene + 0xdc, camera);
            t.g32(0x00cc_b9d8, scene);
            let what = format!("setNearPlane #{i}");
            t.check(&what, 0x0064_5f00, Conv::Cdecl, &[near], |m| {
                Out::U32(mh::set_near_plane(m, near))
            });
        }
    });
}

#[test]
fn r3_tk_boot_steps_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e3a_0003);
        for i in 0..8 {
            t.reset();
            let ret = rng.word();
            let data = if i % 3 == 2 { 0 } else { rng.word() | 1 };
            let hook10 = t.stub_code(0x10, false, 1, 0, 0x91);
            let hook14 = t.stub_code(0x14, false, 1, 0, rng.word());
            let cd = t.zeros(0x20);
            t.w32(cd + 0x10, hook10);
            t.w32(cd + 0x14, hook14);
            t.stubs(&[
                (mh::TK_MC_SET_EXEC_PORT, false, 1, 0, 0x81),
                (mh::TK_MC_SET_DEBUG_LOCK, false, 1, 0, ret),
                (mh::TK_SYNC_VDI, false, 0, 0, 0x82),
                (mh::TK_SYNC_VEI, false, 0, 0, 0x83),
                (mh::TK_SET_VSYNC_HANDLER, false, 1, 0, ret ^ 1),
                (mh::GET_CD, false, 0, 0, cd),
                (mh::TK_CD_READ_WITH_MALLOC, false, 2, 0, data),
                (mh::SND_STANDBY_SHOUT_STREAM, false, 1, 0, 0x84),
                (mh::SND_DATA_SEND_SYNC, false, 1, 0, ret ^ 2),
                (mh::SQS_REPORT_STANDBY_STATUS, false, 1, 0, 0x85),
                (mh::SQS_SHOUT_STREAMING_DATA_SEND_THUNK, false, 2, 0, 0x86),
                (mh::USER_FREE, false, 1, 0, 0x87),
                (mh::SG_MEM_CHECK, false, 3, 0, ret ^ 3),
            ]);
            t.g32(0x0133_022c, rng.word());
            t.check(
                &format!("0x00888310 #{i}"),
                0x0088_8310,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mh::mc_reset_ports(m)),
            );
            t.check(
                &format!("0x00888760 #{i}"),
                0x0088_8760,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mh::sync_and_clear_vsync(m)),
            );
            t.check(
                &format!("0x0088de60 #{i}"),
                0x0088_de60,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mh::load_shout_stream(m)),
            );
            t.check(
                &format!("0x0088ded0 #{i}"),
                0x0088_ded0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mh::send_shout_stream(m)),
            );
        }
    });
}

#[test]
fn r3_pause_leave_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e3a_0004);
        for i in 0..12 {
            t.reset();
            let now = u64::from(rng.next()) << 32 | u64::from(rng.next());
            // mov eax, lo; mov edx, hi; ret
            let mut c = vec![0xb8];
            c.extend((now as u32).to_le_bytes());
            c.push(0xba);
            c.extend(((now >> 32) as u32).to_le_bytes());
            c.push(0xc3);
            let clock = t.cave_code(&c);
            t.jump(mh::GRAPHIC_GET_TIME_US, clock);
            let movie = [0, 1, 0x48][i % 3];
            let ret = rng.word();
            t.stubs(&[
                (mh::GRAPHIC_SET_FREE_CAMERA_MODE, false, 1, 0, 0x81),
                (mh::SG_SET_PAUSE_ALL_SOUNDS, false, 1, 0, 0x82),
                (mh::PAUSE_INPUT_RESUME, false, 0, 0, 0x83),
                (mh::INPUT_UPDATE_ALL, false, 0, 0, 0x84),
                (mh::MOVIE_WITH_3D_SCENE, false, 0, 0, movie),
                (mh::GRAPHIC_VIDEO_PAUSE, false, 0, 0, ret),
            ]);
            for (g, v) in [
                (0x0134_084c, 1),
                (0x0134_0850, 1),
                (0x0134_0828, 1),
                (0x012f_b818, rng.next()),
                (0x012f_b81c, rng.next()),
                (0x012f_b820, rng.next()),
                (0x012f_b824, rng.next()),
                (0x0112_a00c, u32::from((i / 3) % 2 == 1)),
            ] {
                t.g32(g, v);
            }
            t.check(
                &format!("Pause_Leave #{i}"),
                0x008a_b2d0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mh::pause_leave(m)),
            );
        }
    });
}

#[test]
fn r3_btl_end_steps_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e3a_0005);
        for i in 0..40 {
            t.reset();
            let (sin, cos) = (rng.next(), rng.next());
            t.stubs(&[
                (mh::MATH_SIN_LOOKUP, false, 1, 0, sin),
                (mh::MATH_COS_LOOKUP, false, 1, 0, cos),
                (mh::BTL_END_DRAW_ITEM_PAGE, false, 1, 0, 0x81),
                (mh::BTL_END_DRAW_WEAPON_PAGE, false, 1, 0, 0x82),
                (mh::BTL_END_DRAW_GIL_PAGE, false, 1, 0, 0x83),
                (mh::BTL_END_DRAW_SPOIL_LIST_PAGE, false, 1, 0, 0x84),
                (mh::BTL_END_DRAW_EXCHANGE_PAGE, false, 1, 0, 0x85),
                (mh::BTL_END_DRAW_SPOIL_PAGE, false, 1, 0, 0x86),
                (mh::MS_SAVE_ITEM_USE, false, 2, 0, 0x87),
            ]);
            let fade = match i % 5 {
                0 => 0,
                1 => 0x1000,
                2 => rng.below(0x1000),
                _ => rng.next() & 0xffff,
            };
            // the two fades get opposite speeds (each reads its own)
            let speed = [0x80, 0xff80, 0, rng.next() & 0xffff][i % 4];
            for (k, (f, s)) in [(0x0186_a160, 0x0186_a164), (0x0186_a168, 0x0186_a16c)]
                .into_iter()
                .enumerate()
            {
                t.g32(f, fade | (rng.next() & 0xffff_0000));
                t.g32(s, if k == 0 { speed } else { speed ^ 0xff00 });
            }
            t.check(
                &format!("0x008bc080 #{i}"),
                0x008b_c080,
                Conv::Cdecl,
                &[],
                |m| {
                    mh::draw_gil_pages(m);
                    Out::None
                },
            );
            t.check(
                &format!("0x008bd940 #{i}"),
                0x008b_d940,
                Conv::Cdecl,
                &[],
                |m| {
                    mh::draw_spoil_pages(m);
                    Out::None
                },
            );
            let count = [0u8, 1, 3, 9, 0x80, 0xff][i % 6];
            let spoils: Vec<u8> = (0..0x20).map(|_| rng.next() as u8).collect();
            let at = t.rebase(0x0231_0f74);
            t.write(at, &spoils);
            let at = t.rebase(0x0231_0f70);
            t.write(at, &[count]);
            t.check(
                &format!("0x008bd9c0 #{i}"),
                0x008b_d9c0,
                Conv::Cdecl,
                &[],
                |m| {
                    mh::take_spoils(m);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn r3_menu_screen_steps_match_the_original() {
    type Step = fn(&mut G) -> u32;
    let steps: &[(u32, Step)] = &[
        (0x008b_eff0, |m| mh::help_draw(m)),
        (0x008c_3340, |m| mh::help_close(m)),
        (0x008c_3360, |m| mh::draw_empty_screen(m)),
        (0x008c_5ee0, |m| mh::item_step_draw_bg(m)),
        (0x008c_5ef0, |m| mh::item_open(m)),
        (0x008c_6050, |m| mh::item_close(m)),
        (0x008c_60a0, |m| mh::screen_x2_off(m)),
        (0x008c_64a0, |m| mh::item_draw_plate(m)),
        (0x008c_64f0, |m| mh::shop_open_rect(m)),
    ];
    Twins::each(|t| {
        let mut rng = Rng(0x7e3a_0006);
        for i in 0..6 {
            t.reset();
            let ret = rng.word();
            let list = t.stub_code(0x00c4_16b0, false, 0, 0, 0x90);
            let slot = t.rebase(0x00c4_16b0);
            t.patch(slot, &list.to_le_bytes());
            let who = rng.below(8);
            t.stubs(&[
                (mh::TK_VU1_SYNC_PATH, false, 0, 0, 0x81),
                (mh::HELP_DRAW_PREPARE, false, 0, 0, 0x82),
                (mh::TO_DRAW_SCISSOR_XYWH, false, 4, 0, 0x83),
                (mh::TO_DRAW_BOX_XYWHC2, false, 6, 0, 0x84),
                (mh::TO_MENU_DRAW_KICK_TMP, false, 0, 0, 0x85),
                (mh::TO_DAPS_DRAW, false, 0, 0, ret),
                (mh::TK_MENU_MAIN_SLEEP_MENU, false, 1, 0, 0x86),
                (mh::TO_MENU_INIT_BRUSH, false, 0, 0, ret ^ 1),
                (mh::TK_MENU_DRAW_KICK, false, 0, 0, 0x87),
                (mh::TK_VU1_SYNC_PATH_THUNK, false, 0, 0, 0x88),
                (mh::TK_MN2_DRAW_INIT_PACKET, false, 0, 0, 0x89),
                (mh::TK_MN2_DRAW_KICK_SYNC_PACKET, false, 0, 0, 0x8a),
                (mh::TO_DRAW_MENU_BG, false, 0, 0, ret ^ 2),
                (mh::TK_MENU_SET_SCREEN_X2_FLAG, false, 1, 0, ret ^ 3),
                (mh::MENU_ITEM_GET_LINE_POS, false, 0, 0, 0x8b),
                (mh::TO_SEQ_GET_RECT, false, 6, 0, ret ^ 4),
                (mh::TK_MENU_ITEM_CLOSE_WINDOW, false, 0, 0, 0x8c),
                (mh::TK_MENU_ITEM_CLOSE_WINDOW2, false, 0, 0, 0x8d),
                (mh::TK_MENU_ITEM_CLOSE_WINDOW3, false, 0, 0, 0x8e),
                (mh::TK_MENU_ITEM_CLOSE_WINDOW4, false, 0, 0, 0x8f),
                (mh::TO_SEQ_STOP_SCENE, false, 2, 0, 0x90),
                (mh::TK_MENU_CLOSE_SUB_CMD_WINDOW, false, 0, 0, ret ^ 5),
                (mh::TK_MENU_GET_CURRENT_PLAYER, false, 0, 0, who),
                (mh::TO_MENU_DRAW_CHR_STATUS_PLATE, false, 3, 0, ret ^ 6),
                (mh::TK_MN2_GET_WSHOP_POS, false, 0, 0, 0x91),
                (mh::TK_MN2_GET_ISHOP_POS, false, 0, 0, 0x92),
            ]);
            t.stub_float_logged(mh::GRAPHIC_UI_REMAP_X2, 1, 1.25 + i as f32);
            t.stub_float_logged(mh::GRAPHIC_UI_REMAP_Y2, 1, -3.5 * i as f32);
            let w = t.zeros(0x40);
            let busy = [0u16, 1, 0x8000][i % 3];
            t.write(w + 0x30, &busy.to_le_bytes());
            t.g32(0x0186_a278, w);
            let at = t.rebase(0x0186_a390);
            t.write(at, &[rng.next() as u8]);
            for &(sym, step) in steps {
                t.check(&format!("{sym:#x} #{i}"), sym, Conv::Cdecl, &[], |m| {
                    Out::U32(step(m))
                });
            }
        }
    });
}

// ---- round 3, batch 2 (ffx_menu::m5m_i) ----

#[test]
fn r3b2_jump_thunks_match_the_original() {
    type Bridge = fn(&mut G, &[u32]) -> u32;
    let list: &[(u32, u32, u32, Bridge)] = &[
        (0x008c_a470, mi::TK_SND_UPDATE, 0, |m, _| {
            mi::tk_snd_update_thunk(m)
        }),
        (0x008c_c1a0, mi::SUMMON_INIT, 0, |m, _| {
            mi::summon_init_thunk(m)
        }),
        (0x008c_f820, mi::WEAPON_INIT, 0, |m, _| {
            mi::weapon_init_thunk(m)
        }),
        (0x008d_0a10, mi::LIMIT_INIT, 0, |m, _| {
            mi::limit_init_thunk(m)
        }),
        (0x008d_55d0, mi::KAIZOU_INIT, 0, |m, _| {
            mi::kaizou_init_thunk(m)
        }),
        (0x008d_d460, mh::TO_DRAW_SCISSOR_XYWH, 4, |m, a| {
            mi::menu_draw_scissor_rect(m, a[0], a[1], a[2], a[3])
        }),
    ];
    Twins::each(|t| {
        let mut rng = Rng(0x7e3b_0001);
        for &(thunk, target, n, bridge) in list {
            for i in 0..3 {
                t.reset();
                let ret = rng.word();
                t.stub(target, false, n, 0, ret);
                let args: Vec<u32> = (0..n).map(|_| rng.word()).collect();
                let what = format!("thunk {thunk:#x} #{i}");
                t.check(&what, thunk, Conv::Cdecl, &args, |m| {
                    Out::U32(bridge(m, &args))
                });
            }
        }
        t.reset();
        t.check("0x008dde90", 0x008d_de90, Conv::Cdecl, &[], |m| {
            Out::U32(mi::vibrate_curve_008dde90(m))
        });
        for old in [0, 1, 0xdead_0001] {
            t.g32(0x025d_16bc, old);
            t.check("0x008db7d0", 0x008d_b7d0, Conv::Cdecl, &[], |m| {
                mi::tuto_clear(m);
                Out::None
            });
        }
    });
}

#[test]
fn r3b2_menu_screen_steps_match_the_original() {
    type Step = fn(&mut G) -> u32;
    let steps: &[(u32, Step)] = &[
        (0x008c_b350, |m| mi::config_open(m)),
        (0x008c_c300, |m| mi::summon_close(m)),
        (0x008c_f910, |m| mi::weapon_stop_scenes(m)),
        (0x008d_09b0, |m| mi::limit_draw(m)),
        (0x008d_0c70, |m| mi::limit_close(m)),
        (0x008d_1d30, |m| mi::status_open(m)),
        (0x008d_1dd0, |m| mi::status_close(m)),
        (0x008d_55c0, |m| mi::draw_sync_only(m)),
        (0x008d_5650, |m| mi::kaizou_close(m)),
        (0x008d_7560, |m| mh::draw_empty_screen(m)),
        (0x008d_7580, |m| mi::shop_get_pos(m)),
        (0x008d_75b0, |m| mi::wshop_close(m)),
        (0x008d_9aa0, |m| mh::draw_empty_screen(m)),
        (0x008d_9ac0, |m| mi::shop_get_pos(m)),
        (0x008d_9ae0, |m| mi::wshop_close3(m)),
    ];
    Twins::each(|t| {
        let mut rng = Rng(0x7e3b_0002);
        for i in 0..8 {
            t.reset();
            let ret = rng.word();
            let who = rng.below(8);
            let lang = [0, 1, 0xb][i % 3];
            let layout = [1, 0, 2, 1][i % 4];
            t.stubs(&[
                (mi::TO_GET_FFX_LANG_JMP, false, 0, 0, lang),
                (mi::GET_U32_023CBEB0, false, 0, 0, layout),
                (mi::CONFIG_OPEN, false, 0, 0, ret),
                (mh::TK_MENU_CLOSE_SUB_CMD_WINDOW, false, 0, 0, 0x81),
                (mi::TK_MENU_SUMMON_CLOSE_WINDOW, false, 0, 0, ret ^ 1),
                (mi::TK_MN2_SEQ_STOP_SCENE, false, 1, 0, ret ^ 2),
                (mh::TK_MENU_GET_CURRENT_PLAYER, false, 0, 0, who),
                (mh::TK_MENU_DRAW_KICK, false, 0, 0, 0x82),
                (mh::TK_VU1_SYNC_PATH_THUNK, false, 0, 0, 0x83),
                (mh::TK_MN2_DRAW_INIT_PACKET, false, 0, 0, 0x84),
                (mh::TO_DRAW_MENU_BG, false, 0, 0, 0x85),
                (mi::MENU_DRAW_DIVIDER_SHAPES, false, 1, 0, 0x86),
                (mh::TO_MENU_DRAW_CHR_STATUS_PLATE, false, 3, 0, 0x87),
                (mi::DRAW_LIMIT_PAGE_OVER_DRIVE, false, 1, 0, 0x88),
                (mh::TK_MN2_DRAW_KICK_SYNC_PACKET, false, 0, 0, ret ^ 3),
                (mi::TK_MENU_LIMIT_CLOSE_WINDOW, false, 0, 0, 0x89),
                (mi::TK_MENU_LIMIT_CLOSE_WINDOW2, false, 0, 0, ret ^ 4),
                (mi::TK_MN2_GET_STATUS_POS, false, 0, 0, ret ^ 5),
                (mi::MENU_STATUS_CLOSE_WINDOW, false, 1, 0, 0x8a),
                (mi::TK_MN2_INIT_STATUS_BG, false, 0, 0, ret ^ 6),
                (mi::TK_MS_SET_SAVE_PARAM_ALL, false, 0, 0, 0x8b),
                (mi::TK_MENU_KAIZOU_CLOSE_WINDOW, false, 0, 0, 0x8c),
                (mi::ATEL_REGIST_CALLBACK_QUE2, false, 3, 0, ret ^ 7),
                (mh::TK_MN2_GET_ISHOP_POS, false, 0, 0, 0x8d),
                (mh::TK_MN2_GET_WSHOP_POS, false, 0, 0, 0x8e),
                (mi::MENU_WSHOP_CLOSE_SUB_WINDOW4, false, 0, 0, 0x8f),
                (mi::TK_MENU_WSHOP_CLOSE_WINDOW3, false, 0, 0, 0x90),
            ]);
            t.stub_float_logged(mh::GRAPHIC_UI_REMAP_X2, 1, 2.75 * i as f32);
            t.stub_float_logged(mh::GRAPHIC_UI_REMAP_Y2, 1, 0.5 - i as f32);
            let restart = [0, 1, 2, 0xffff_ffff, 7][i % 5];
            t.g32(0x0186_a5fc, restart);
            for k in 0..6 {
                t.g32(0x0186_a820 + 4 * k, rng.word());
            }
            t.g32(0x0186_a448, rng.word());
            t.g32(0x0186_a44c, rng.word());
            for &(sym, step) in steps {
                t.check(&format!("{sym:#x} #{i}"), sym, Conv::Cdecl, &[], |m| {
                    Out::U32(step(m))
                });
            }
        }
    });
}

/// The x87 row and cursor drawers: the remaps stubbed with awkward
/// floats (so every product and sum rounds), both `ftol` paths
/// (`__sse2_available` 0 and 1), rows in and out of range, the row table
/// random.
#[test]
#[allow(clippy::too_many_lines)]
fn r3b2_menu_row_drawers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e3b_0003);
        for i in 0..48u32 {
            t.reset();
            let ret = rng.word();
            t.stubs(&[
                (mi::TK_MN2_DRAW_CROSS_CMD_CURSOR, false, 3, 0, ret),
                (mi::TK_MN2_DRAW_CROSS_CURSOR, false, 3, 0, ret ^ 1),
                (mi::TO_DRAW_MENU_PLATE_XYWH_TYPE, false, 5, 0, 0x81),
                (mi::TO_DRAW_CROSS_MENU_ICON_COMMAND_NAME, false, 4, 0, 0x82),
                (mi::TO_DRAW_CROSS_MENU_ICON_ABILITY_NAME, false, 4, 0, 0x83),
            ]);
            let big = if i % 11 == 10 { 3.0e9 } else { 1.0 };
            let rx = (rng.below(2000) as f32 - 300.0) / 7.0 * big;
            let ry = (rng.below(1200) as f32 - 100.0) / 3.0;
            t.stub_float_logged(mh::GRAPHIC_UI_REMAP_X2, 1, rx);
            t.stub_float_logged(mh::GRAPHIC_UI_REMAP_Y2, 1, ry);
            t.g32(0x0193_ed20, i % 2);
            let w = t.zeros(0x60);
            let rows = rng.below(14) as u16;
            t.write(w + 0x30, &rows.to_le_bytes());
            for off in [0x32, 0x46, 0x48] {
                let v = (rng.next() as i16) >> (rng.below(12) as i16);
                t.write(w + off, &v.to_le_bytes());
            }
            let table: Vec<u8> = (0..0x40)
                .map(|k| match rng.below(4) {
                    0 => 0,
                    1 => [4u8, 0xb, 0xff][k % 3],
                    _ => rng.next() as u8,
                })
                .collect();
            let at = t.rebase(mi::ROW_TABLE);
            t.write(at, &table);
            // past 2^24 some integers don't fit an f32 (fild/fstp rounds)
            let wide = |r: &mut Rng| match r.below(3) {
                0 => r.next() | 0x0100_0001,
                _ => (r.next() % 2000).wrapping_sub(500),
            };
            let x = wide(&mut rng);
            let y = wide(&mut rng);
            for sym in [0x008d_5df0, 0x008d_65f0] {
                t.check(
                    &format!("{sym:#x} #{i}"),
                    sym,
                    Conv::Cdecl,
                    &[w, x, y],
                    |m| Out::U32(mi::row_cursor(m, w, x, y)),
                );
            }
            t.check(
                &format!("0x008c9940 #{i}"),
                0x008c_9940,
                Conv::Cdecl,
                &[w, x, y],
                |m| Out::U32(mi::list_cursor(m, w, x, y)),
            );
            t.check(
                &format!("0x008cda20 #{i}"),
                0x008c_da20,
                Conv::Cdecl,
                &[w, x, y],
                |m| Out::U32(mi::summon_cursor(m, w)),
            );
            let fx = ((rng.below(4000) as f32 - 1000.0) / 9.0).to_bits();
            let fy = ((rng.below(4000) as f32 - 1000.0) / 13.0).to_bits();
            let row = [0, 1, 5, 13, 0xffff_ffff, rng.below(16)][i as usize % 6];
            let fixed = i / 2 % 2;
            for (sym, which) in [
                (0x008c_dab0, mi::Rows::Summon),
                (0x008d_5e60, mi::Rows::Kaizou),
            ] {
                let args = [w, fx, fy, row, fixed];
                t.check(&format!("{sym:#x} #{i}"), sym, Conv::Cdecl, &args, |m| {
                    mi::row_draw(m, which, w, fx, fy, row, fixed);
                    Out::None
                });
            }
            // the lists: their row drawers run as they are, then stubbed
            let b = rng.below(6);
            for stub_rows in [false, true] {
                if stub_rows {
                    t.stubs(&[
                        (mi::SUMMON_ROW_DRAW, false, 5, 0, 0x84),
                        (mi::KAIZOU_ROW_DRAW, false, 5, 0, 0x85),
                    ]);
                }
                let s = [w, fixed, b, fx, fy];
                t.check(
                    &format!("0x008cd990 #{i}"),
                    0x008c_d990,
                    Conv::Cdecl,
                    &s,
                    |m| {
                        mi::list_draw(m, mi::Rows::Summon, w, fixed, b, fx, fy);
                        Out::None
                    },
                );
                let k = [w, fixed, b, x, y];
                t.check(
                    &format!("0x008d5d50 #{i}"),
                    0x008d_5d50,
                    Conv::Cdecl,
                    &k,
                    |m| {
                        mi::list_draw(m, mi::Rows::Kaizou, w, fixed, b, x, y);
                        Out::None
                    },
                );
            }
        }
    });
}

// ---- round 3, batch 3 (ffx_menu::m5m_j) ----

#[test]
fn r3b3_malloc_and_read_gamecode_matches_the_original() {
    Twins::each(|t| {
        t.skip = 4;
        let mut rng = Rng(0x7e3c_0001);
        let langs = [0u32, 1, 2, 5, 6, 8, 9, 10, 11, 0x12, 0x13, 0xffff_ffff];
        let ends = [
            0u32,
            1,
            15,
            16,
            0x1e1,
            0xffff_ffff,
            0u32.wrapping_sub(20),
            0u32.wrapping_sub(100),
        ];
        for (i, &lang) in langs.iter().enumerate() {
            for (k, &end) in ends.iter().enumerate() {
                t.reset();
                // a negative descriptor counts as open (only 0 fails)
                let fd = match (i + k) % 5 {
                    4 => 0,
                    3 => 0xffff_fff0,
                    _ => 7 + rng.below(4),
                };
                let pad = t.zeros(0x80);
                // 0xff fill: the cleared first word and the 0 byte after the data show
                let data = t.alloc(&[0xff; 0x400]) + 0x100;
                let _ = pad;
                t.stub_import(mj::SPRINTF_IAT, false, 3, 0, 0x20);
                t.stubs(&[
                    (mj::SCE_OPEN, false, 2, 0, fd),
                    (mj::SCE_LSEEK, false, 3, 0, end),
                    (mj::SCE_READ, false, 3, 0, 0x31),
                    (mj::SCE_CLOSE, false, 1, 0, 0),
                    (mj::DBG_PRINTF, false, 2, 0, 0x32),
                    (mj::TK_MALLOC, false, 1, 0, data),
                    (mj::TK_FREE, false, 1, 0, 0x33),
                ]);
                for g in [0x00cc_b95c, 0x00cc_b960] {
                    let old = if rng.chance(2) { 0 } else { rng.word() | 1 };
                    t.g32(g, old);
                }
                let what = format!("MallocAndReadGamecode {lang:#x} end {end:#x}");
                t.check_framed(&what, 0x0064_6920, Conv::Cdecl, &[lang], 0x88, |m, ebp| {
                    Out::U32(mj::malloc_and_read_gamecode(m, ebp, lang))
                });
            }
        }
    });
}

#[test]
fn r3b3_kaizou_fade_band_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e3c_0002);
        for i in 0..400 {
            t.reset();
            let ret = rng.word();
            t.stub(mj::TO_MKP_SHAPE_XYWHUVC2, false, 11, 0, ret);
            let c = match i % 4 {
                0 => rng.below(0x20),
                1 => 0x20 << rng.below(8),
                _ => rng.next(),
            };
            let y = (rng.next() as i16) >> rng.below(14);
            let at = t.rebase(0x0186_aa42);
            t.write(at, &y.to_le_bytes());
            t.check(
                &format!("0x008d6340 #{i}"),
                0x008d_6340,
                Conv::Cdecl,
                &[c],
                |m| Out::U32(mj::kaizou_fade_band(m, c)),
            );
        }
    });
}

// ---- round 4, batch 1 (ffx_menu::m5m_k) ----

impl Twins {
    /// A recording stub (`tag`, `n` words, `pops`, `ret`) that first copies
    /// `bytes` to the pointer in argument `arg`. Returns its address.
    fn stub_writing(
        &mut self,
        tag: u32,
        n: u32,
        pops: u32,
        ret: u32,
        arg: u32,
        bytes: &[u8],
    ) -> u32 {
        let src = self.alloc(bytes);
        let log = self.stub_code(tag, false, n, pops, ret);
        let mut c = vec![0x56, 0x57]; // push esi; push edi
        c.extend([0x8b, 0x7c, 0x24, (0xc + 4 * arg) as u8]); // mov edi, [esp+0xc+4arg]
        c.push(0xbe);
        c.extend(src.to_le_bytes()); // mov esi, src
        c.push(0xb9);
        c.extend((bytes.len() as u32).to_le_bytes()); // mov ecx, len
        c.extend([0xf3, 0xa4]); // rep movsb
        c.extend([0x5f, 0x5e]); // pop edi; pop esi
        c.push(0xe9);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(log.wrapping_sub(here).to_le_bytes()); // jmp log
        self.cave_code(&c)
    }

    /// [`Twins::stub_writing`] over the original at `sym`.
    fn stub_write(&mut self, sym: u32, n: u32, ret: u32, arg: u32, bytes: &[u8]) {
        let addr = self.stub_writing(sym, n, 0, ret, arg, bytes);
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        self.patch(at, &j);
    }

    /// [`Twins::stub_writing`] bound to the import slot `slot` (cdecl).
    fn stub_import_write(&mut self, slot: u32, n: u32, ret: u32, arg: u32, bytes: &[u8]) {
        let addr = self.stub_writing(slot, n, 0, ret, arg, bytes);
        let at = self.rebase(slot);
        self.patch(at, &addr.to_le_bytes());
    }

    /// A stub over `sym` whose result depends on its argument `arg`: `hit`
    /// when it equals `value`, else `miss`; logs `n` words under one tag.
    fn stub_branch(&mut self, sym: u32, n: u32, arg: u32, value: u32, hit: u32, miss: u32) -> u32 {
        let a = self.stub_code(sym, false, n, 0, hit);
        let b = self.stub_code(sym, false, n, 0, miss);
        let mut c = vec![0x81, 0x7c, 0x24, (4 + 4 * arg) as u8];
        c.extend(value.to_le_bytes()); // cmp dword [esp+4+4arg], value
        c.extend([0x0f, 0x84]); // je a
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(a.wrapping_sub(here).to_le_bytes());
        c.push(0xe9); // jmp b
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(b.wrapping_sub(here).to_le_bytes());
        self.cave_code(&c)
    }

    /// Runs the original at `sym` with its frame's `len` bytes below
    /// `ebp` (the frame base the caller computed, for aligned frames)
    /// zeroed in both oracles, the port's calls below it, and compares
    /// the frame (less its top `skip` bytes) after the call.
    #[allow(clippy::too_many_arguments)]
    fn check_frame_at(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        ebp: u32,
        len: u32,
        bridge: impl FnOnce(&mut G, u32) -> Out,
    ) {
        let lo = ebp - len;
        self.write(lo, &vec![0; len as usize]);
        self.b.set_call_top(Some(lo - 0x100));
        self.check(what, sym, conv, args, |m| bridge(m, ebp));
        self.b.set_call_top(None);
        let n = (len - self.skip) as usize;
        let mut x = vec![0; n];
        let mut y = vec![0; n];
        Mem::read_bytes(&self.a, lo, &mut x);
        Mem::read_bytes(&self.b, lo, &mut y);
        if let Some(i) = x.iter().zip(&y).position(|(u, v)| u != v) {
            panic!(
                "{what}: frame byte ebp - {:#x}: {} != {}",
                len - i as u32,
                x[i],
                y[i]
            );
        }
    }
}

impl Twins {
    /// The event loaders' `sprintf`: logs as many words as each format
    /// takes (3 by default).
    fn sprintf_by_format(&mut self, kind: mk::EventFile) {
        let five: &[u32] = match kind {
            mk::EventFile::Ftc => &[0x00b5_eb54, 0x00b5_ebb0],
            mk::EventFile::Message => &[0x00b5_eb00, 0x00b5_eb38],
        };
        let four: &[u32] = match kind {
            mk::EventFile::Ftc => &[],
            mk::EventFile::Message => &[0x00b5_eaf0, 0x00b5_eb20],
        };
        let mut cases = Vec::new();
        for &f in five {
            cases.push((
                self.rebase(f),
                self.stub_code(mk::SPRINTF_IAT, false, 5, 0, 0x72),
            ));
        }
        for &f in four {
            cases.push((
                self.rebase(f),
                self.stub_code(mk::SPRINTF_IAT, false, 4, 0, 0x72),
            ));
        }
        let other = self.stub_code(mk::SPRINTF_IAT, false, 3, 0, 0x72);
        let mut c = Vec::new();
        for (v, to) in cases {
            c.extend([0x81, 0x7c, 0x24, 8]);
            c.extend(v.to_le_bytes()); // cmp dword [esp+8], v
            c.extend([0x0f, 0x84]);
            let here = self.cave_next + c.len() as u32 + 4;
            c.extend(to.wrapping_sub(here).to_le_bytes()); // je to
        }
        c.push(0xe9);
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(other.wrapping_sub(here).to_le_bytes());
        let d = self.cave_code(&c);
        let at = self.rebase(mk::SPRINTF_IAT);
        self.patch(at, &d.to_le_bytes());
    }
}

/// A client box: rectangle, target, colours and the divisor word.
fn client_box(rng: &mut Rng) -> Vec<u8> {
    let mut c = vec![0u8; 0x28];
    for o in (8..0x18).step_by(2) {
        put16(&mut c, o, (rng.below(600) as i16) - 100);
    }
    for b in &mut c[0x18..0x20] {
        *b = rng.next() as u8;
    }
    let d = match rng.below(4) {
        0 => 0,
        1 => rng.below(9) as i16 - 4,
        _ => rng.below(12) as i16 + 1,
    };
    put16(
        &mut c,
        0x24,
        if d == 0 && rng.chance(2) { 0 } else { d.max(1) },
    );
    if rng.chance(3) {
        put16(&mut c, 0x14, 0);
    }
    c
}

#[test]
fn r4b1_draw_client_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0001);
        for i in 0..300 {
            t.reset();
            t.packet(rng.below(30));
            let c = client_box(&mut rng);
            let c = t.alloc(&c);
            let (w, hh) = if i % 4 == 0 {
                (0x1001 + 2 * rng.below(0x80), 0x1003 + 2 * rng.below(0x80))
            } else {
                (0x200 + rng.below(0x300), 0x180 + rng.below(0x200))
            };
            t.g32(g::SCREEN_W, w);
            t.g32(g::SCREEN_H, hh);
            let s = 0x80 + rng.below(0x100);
            t.g32(g::H_SCALE, s);
            let (x, y) = (rng.below(400), rng.below(300));
            t.check(
                &format!("Gu_DrawClient #{i}"),
                0x0084_7b50,
                Conv::Cdecl,
                &[c, x, y],
                |m| Out::U32(mk::draw_client(m, c, x, y)),
            );
        }
    });
}

#[test]
fn r4b1_gu_printf_matches_the_original() {
    let texts: [&[u8]; 12] = [
        b"plain %d\0",
        b"%d percent\0",
        b"%x\0",
        b"%~\0",
        b"%~b~i~.text\0",
        b"%~btext\0",
        b"%~itext\0",
        b"%~cff~w4\0",
        b"%~w12x\0",
        b"%~zz\0",
        b"%~\x90a\0",
        b"%~.\0",
    ];
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0002);
        for (i, text) in texts.iter().enumerate() {
            for k in 0..6 {
                t.reset();
                t.skip = 0;
                t.packet(rng.below(20));
                for (sym, v) in [
                    (0x0130_370c, rng.below(0x80)),
                    (0x0130_3710, rng.below(0x80)),
                    (0x0130_3714, rng.below(0x80)),
                    (0x0130_3718, rng.below(2)),
                    (0x0130_3720, rng.below(2)),
                    (0x0130_3728, rng.below(0x400)),
                    (0x0130_36bc, rng.word()),
                ] {
                    t.g32(sym, v);
                }
                let s = t.alloc(text);
                // the parsers end at a continuation: more codes or text
                let cont = t.alloc(if k % 2 == 0 { b"~bxy\0" } else { b"zz\0" });
                let v = rng.next();
                t.stub_write(mk::GU_PARSE_HEX, 2, cont, 1, &v.to_le_bytes());
                let v = rng.below(0x800);
                t.stub_write(mk::GU_PARSE_NUMBER, 2, cont, 1, &v.to_le_bytes());
                let chars = rng.next();
                let log = t.stub_code(mk::SG_VFPRINTF, false, 3, 0, 0x55);
                let at = t.rebase(0x0130_36bc);
                let mut c = vec![0xc7, 0x05];
                c.extend(at.to_le_bytes());
                c.extend(chars.to_le_bytes()); // mov dword [chars], v
                // the style the text is printed with, kept in a record
                let rec = t.zeros(0x20);
                let style = [
                    0x0130_370c,
                    0x0130_3710,
                    0x0130_3714,
                    0x0130_3718,
                    0x0130_3720,
                    0x0130_3728,
                ];
                for (k, sym) in style.into_iter().enumerate() {
                    c.push(0xa1);
                    c.extend(t.rebase(sym).to_le_bytes()); // mov eax, [sym]
                    c.push(0xa3);
                    c.extend((rec + 4 * k as u32).to_le_bytes()); // mov [rec+4k], eax
                }
                c.push(0xe9);
                let here = t.cave_next + c.len() as u32 + 4;
                c.extend(log.wrapping_sub(here).to_le_bytes());
                let vf = t.cave_code(&c);
                t.jump(mk::SG_VFPRINTF, vf);
                t.stubs(&[
                    (mk::GU_PRINT_BEGIN, false, 0, 0, 0x51),
                    (mk::SET_PRINTF_FUNC, false, 1, 0, 0x5200 + i as u32),
                ]);
                let args = [s, rng.word(), rng.word()];
                let va = t.frame_ebp(3) + 0xc;
                t.check_framed(
                    &format!("Gu_Printf {:?} #{k}", String::from_utf8_lossy(text)),
                    0x0084_8880,
                    Conv::Cdecl,
                    &args,
                    4,
                    |m, ebp| Out::U32(mk::gu_printf(m, ebp - 4, s, va)),
                );
            }
        }
    });
}

#[test]
fn r4b1_slot_files_match_the_original() {
    let kinds = [
        (0x0064_6b30, mk::SlotFile::Load, 0x40c),
        (0x0064_6ca0, mk::SlotFile::LoadBare, 0x40c),
        (0x0064_6df0, mk::SlotFile::Save, 0x408),
    ];
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0003);
        for (sym, kind, low) in kinds {
            for k in 0..8 {
                t.reset();
                t.skip = 4;
                let zeros = vec![0u8; 0x100];
                t.stub_write(mk::CRT_MEMSET, 3, 0x41, 0, &zeros);
                let num: &[u8] = if k % 2 == 0 { b"07\0" } else { b"123456\0" };
                t.stub_import_write(mk::SPRINTF_IAT, 3, 0x42, 0, num);
                let dir: &[u8] = if k % 3 == 0 {
                    b"\0"
                } else {
                    b"C:/game/saves/\0"
                };
                t.stub_write(mk::DATA_DIRECTORY, 3, 0x43, 1, dir);
                let f = if k % 4 == 3 { 0 } else { 0x4400 + k };
                t.stub_import(mk::FOPEN_IAT, false, 2, 0, f);
                t.stub_import(mk::FREAD_IAT, false, 4, 0, 0x45);
                t.stub_import(mk::FWRITE_IAT, false, 4, 0, 0x46);
                t.stub_import(mk::FCLOSE_IAT, false, 1, 0, 0x47);
                t.stubs(&[
                    (mk::CROSS_ASSERT_OUTPUT, false, 1, 0, 0x48),
                    (mk::SAVE_DATA_CHECK_CRC, false, 0, 0, rng.word()),
                ]);
                let fill: Vec<u8> = (0..0x6500u32).map(|i| (i * 7) as u8).collect();
                let buf = t.alloc(&fill);
                let (slot, size) = (rng.below(100), rng.below(0x6500));
                t.check_framed(
                    &format!("{kind:?} #{k}"),
                    sym,
                    Conv::Cdecl,
                    &[slot, buf, size],
                    low,
                    |m, ebp| Out::U32(mk::slot_file(m, ebp, kind, slot, buf, size)),
                );
            }
        }
    });
}

#[test]
fn r4b1_kabe_packets_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0004);
        for i in 0..200 {
            t.reset();
            let p = t.zeros(0x100);
            let w = match i % 3 {
                0 => rng.below(0x300),
                1 => 1 << rng.below(11),
                _ => rng.word(),
            };
            let hh = match i % 4 {
                0 => rng.below(0x300),
                1 => (1 << rng.below(11)) + rng.below(2),
                _ => rng.word(),
            };
            let (tbp, psm, x, y) = (rng.word(), rng.word(), rng.word(), rng.word());
            t.check(
                &format!("GsPkt_TexturedSprite #{i}"),
                0x0077_4ff0,
                Conv::Cdecl,
                &[p, tbp, psm, x, y, w, hh],
                |m| Out::U32(mk::textured_sprite(m, p, tbp, psm, x, y, w, hh)),
            );
            let p = t.zeros(0x100);
            t.check(
                &format!("frame packet #{i}"),
                0x0077_5330,
                Conv::Cdecl,
                &[p, x, y, w, hh],
                |m| Out::U32(mk::frame_packet(m, p, x, y, w, hh)),
            );
        }
    });
}

#[test]
fn r4b1_kabe_load_image_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0005);
        for i in 0..40 {
            t.reset();
            t.skip = 4;
            t.stubs(&[
                (mk::SCE_GS_SET_DEF_LOAD_IMAGE, false, 8, 0, 0x61),
                (mk::FLUSH_CACHE, false, 1, 0, 0x62),
                (mk::SCE_GS_EXEC_LOAD_IMAGE, false, 2, 0, 0x63),
                (mk::SCE_GS_SYNC_PATH, false, 2, 0, rng.word()),
            ]);
            let mut img = vec![0u8; 0x20];
            img[..4].copy_from_slice(&rng.word().to_le_bytes());
            img[4..8].copy_from_slice(&rng.word().to_le_bytes());
            let img = t.alloc(&img);
            let out = t.zeros(8);
            let base = rng.word();
            t.g32(0x00c4_16cc, base);
            // the aligned frame: entry esp at the return address R, `push
            // ebx`, `sub esp, 8`, `and esp, -16`, `add esp, 4`, `push ebp`
            let r = t.a.stack_range().end - 0x100 - 12 - 4;
            let ebp = (r - 12) & !15;
            t.check_frame_at(
                &format!("kabe load image #{i}"),
                0x0077_5720,
                Conv::Cdecl,
                &[img, out, out + 4],
                ebp,
                0x60,
                |m, ebp| Out::U32(mk::kabe_load_image(m, ebp - 0x60, img, out, out + 4)),
            );
        }
    });
}

#[test]
fn r4b1_event_files_match_the_original() {
    let ftc: [u32; 5] = [0x00b5_5e74, 0x00b5_eb6c, 0x00b5_eb84, 0x00b4_04a0, 0];
    let msg: [u32; 4] = [0x00b5_eae8, 0x00b5_5e74, 0x00b5_eb18, 0];
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0006);
        for (sym, kind, needles) in [
            (0x0088_c860, mk::EventFile::Ftc, &ftc[..]),
            (0x0088_cac0, mk::EventFile::Message, &msg[..]),
        ] {
            for (n, &needle) in needles.iter().enumerate() {
                for k in 0..6 {
                    t.reset();
                    t.skip = 4;
                    let lang = [0u32, 1, 2, 3, 4, 5, 6, 9, 10, 11, 0xffff_ffff][(n + k) % 11];
                    let hit = if needle == 0 { 0 } else { t.rebase(needle) };
                    let s = t.stub_branch(mk::STRSTR_IAT, 2, 1, hit, 0x7100, 0);
                    let at = t.rebase(mk::STRSTR_IAT);
                    t.patch(at, &s.to_le_bytes());
                    t.sprintf_by_format(kind);
                    t.stub_import(mk::SPRINTF_S_IAT, false, 4, 0, 0x73);
                    let exists = u32::from(k % 3 != 2);
                    let size = [0u32, 1, 0x10, 0xffff_f000, 0x800, 0xffff_fff8][k];
                    let data = t.alloc(&[0xff; 0x40]);
                    t.stubs(&[
                        (mk::TO_GET_FFX_LANG, false, 0, 0, lang),
                        (mk::FIOS_UNIFY_FILENAME, false, 3, 0, 0x74),
                        (mk::FIOS_IS_FILE_EXIST, false, 1, 0, exists),
                        (mk::FIOS_OPEN, false, 2, 0, 0x7500 + k as u32),
                        (mk::FIOS_FILE_SIZE, false, 1, 0, size),
                        (mk::SG_MEM_SET_ALIGN, false, 1, 0, 0x76),
                        (mk::USER_MALLOC, false, 1, 0, data),
                        (mk::FIOS_READ, false, 3, 0, 0x77),
                        (mk::FIOS_CLOSE, false, 1, 0, 0x78),
                        (mk::DBG_PRINTF, false, 2, 0, 0x79),
                    ]);
                    let name = t.alloc(b"bsil0300/name\0");
                    let _ = rng.next();
                    t.check_framed(
                        &format!("{kind:?} rule {n} #{k}"),
                        sym,
                        Conv::Cdecl,
                        &[name],
                        0x1c4,
                        |m, ebp| Out::U32(mk::event_file(m, ebp, kind, name)),
                    );
                }
            }
        }
    });
}

/// The motion window's argument and character: frame, speed, count.
fn motion_objects(t: &mut Twins, rng: &mut Rng, count: Option<i16>) -> u32 {
    let name = t.alloc(b"c001\0");
    let mut c = vec![0u8; 0x810];
    c[4..8].copy_from_slice(&name.to_le_bytes());
    c[0x740..0x744].copy_from_slice(&rng.word().to_le_bytes());
    put16(&mut c, 0x754, rng.next() as i16);
    if let Some(n) = count {
        let cnt = t.alloc(&n.to_le_bytes());
        c[0x800..0x804].copy_from_slice(&cnt.to_le_bytes());
    }
    let chr = t.alloc(&c);
    let mut a = vec![0u8; 0x50];
    a[4..8].copy_from_slice(&chr.to_le_bytes());
    for o in (0x38..0x50).step_by(4) {
        a[o..o + 4].copy_from_slice(&rng.word().to_le_bytes());
    }
    t.alloc(&a)
}

#[test]
fn r4b1_chr_view_motion_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0007);
        let msgs = [0x8001u32, 0x8005, 0x8006, 0x8011, 0x8000, 0x8012, 0x8007];
        for (m, &msg) in msgs.iter().enumerate() {
            for id in 0..11u32 {
                t.reset();
                t.skip = 4;
                let count = match (id + m as u32) % 5 {
                    0 => None,
                    1 => Some(-3),
                    _ => Some(rng.below(40) as i16),
                };
                let arg = motion_objects(t, &mut rng, count);
                let mut width = 20 + rng.below(200);
                if width == 15 {
                    width = 16;
                }
                let over = rng.below(2);
                t.stub_import(mk::SPRINTF_IAT, false, 7, 0, 0x81);
                t.stubs(&[
                    (mk::GU_WINDOW_GET_ARG, false, 1, 0, arg),
                    (mk::GU_MOUSE_SET_CAPTURE, false, 2, 0, 0x82),
                    (mk::GU_WINDOW_ADD_BUTTON, false, 7, 0, 0x83),
                    (mk::GU_WINDOW_ADD_CLIENT, false, 6, 0, 0x84),
                    (mk::GU_WINDOW_SET_TITLE, false, 2, 0, 0x85),
                    (mk::GU_WINDOW_GET_WIDTH, false, 1, 0, width),
                    (mk::GU_MOUSE_GET_WINDOW_Y, false, 1, 0, rng.below(300)),
                    (mk::GU_WINDOW_CONTROL_SET_RECT, false, 6, 0, 0x86),
                    (mk::GU_PACKET_BEGIN_TAG, false, 1, 0, 0x87),
                    (mk::GU_SET_TEST, false, 8, 0, 0x88),
                    (mk::GU_SET_PRIM, false, 1, 0, 0x89),
                    (mk::GU_SET_RGBAQ, false, 4, 0, 0x8a),
                    (mk::GU_SET_XYZF2, false, 2, 0, 0x8b),
                    (mk::GU_MOUSE_IN_CLIENT_RECT, false, 5, 0, over),
                    (mk::GU_PACKET_END_TAG, false, 0, 0, 0x8c),
                ]);
                let (msg_id, w) = (
                    if msg == 0x8006 {
                        [4, 8, id][id as usize % 3]
                    } else {
                        id
                    },
                    0x9100,
                );
                t.check_framed(
                    &format!("ChrViewMotionCtlProc {msg:#x} {msg_id}"),
                    0x0084_bc00,
                    Conv::Cdecl,
                    &[w, msg, msg_id],
                    0x44,
                    |m, ebp| Out::U32(mk::chr_view_motion(m, ebp, w, msg, msg_id)),
                );
            }
        }
    });
}

#[test]
fn r4b1_debug_menu_command_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0008);
        let mut ids: Vec<u32> = (0x28..0x70).collect();
        ids.extend([0, 0xffff_ffff, 0x6c, 0x100]);
        ids.extend([0x55, 0x55, 0x55, 0x55, 0x55, 0x6b, 0x6b]);
        for (n, id) in ids.into_iter().enumerate() {
            t.reset();
            t.stubs(&[
                (mk::GUI_EVE_GWM_BUTTONUP, false, 1, 0, 0x91),
                (mk::ATEL_DEBUG_SET_0086FEC0, false, 2, 0, 0x92),
                (mk::SG_TOGGLE_ENABLE_TK_ANTI, false, 2, 0, 0x93),
                (mk::GU_WINDOW_CREATE, false, 6, 0, 0x94),
                (mk::ATEL_INIT_SAVE_WORK, false, 2, 0, 0x95),
                (
                    mk::SG_GET_KEEP_FPS,
                    false,
                    0,
                    0,
                    (n as u32 % 5).wrapping_sub(1),
                ),
                (mk::SG_SET_KEEP_FPS, false, 1, 0, 0x96),
                (mk::SG_GUI_GET_DRAW_CHR, false, 0, 0, rng.below(2)),
                (mk::SG_GUI_SET_DRAW_CHR, false, 1, 0, 0x97),
                (mk::SG_GUI_GET_TEXTURE_ANIM, false, 0, 0, rng.word()),
                (mk::SG_GUI_SET_TEXTURE_ANIM, false, 1, 0, 0x98),
                (mk::GU_BUTTON_SET_TOGGLE_VALUE, false, 3, 0, 0x99),
            ]);
            t.g32(0x0132_5928, rng.word());
            let w = 0x9200;
            let extra = 0x5a5a_0000 + n as u32;
            // the caller's argument words, in `b` where `a`'s call puts them
            let slots = t.frame_ebp(3) + 8;
            let words: Vec<u8> = [w, id, extra]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            t.b.write(slots, &words).unwrap();
            t.b.set_call_top(Some(slots - 0x200));
            t.check(
                &format!("debug menu command {id:#x}"),
                0x0085_4c60,
                Conv::Cdecl,
                &[w, id, extra],
                |m| Out::U32(mk::debug_menu_command(m, w, id, slots)),
            );
            t.b.set_call_top(None);
            let x: [u32; 3] = Mem::read(&t.a, slots);
            let y: [u32; 3] = Mem::read(&t.b, slots);
            assert_eq!(x, y, "debug menu command {id:#x}: argument slots");
        }
    });
}

#[test]
fn r4b1_focus_proc_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0009);
        let msgs = [
            0x8001u32, 0x8005, 0x8006, 0x8007, 0x8009, 0x800a, 0x800b, 0x8011, 0x8000, 0x8012,
        ];
        for msg in msgs {
            for k in 0..8u32 {
                t.reset();
                t.skip = 4;
                let id = match k {
                    0 => 0x4d2,
                    1 => 0,
                    2 => 2,
                    3 => 0x10,
                    4 => 0x12,
                    5 => 0x11,
                    6 => 0x00ff_fff0,
                    _ => rng.next() & 0xff,
                };
                let focus = match rng.below(4) {
                    0 => rng.below(0x20),
                    1 => 0x4_fff0 + rng.below(0x200),
                    _ => rng.below(0x6_0000),
                };
                let held = [0x3c, 0x3d, 0x3b, 0][k as usize % 4];
                t.g32(
                    0x0132_5940,
                    if msg & 1 == 0 { rng.below(0x80) } else { held },
                );
                t.stub_import(mk::SPRINTF_IAT, false, 3, 0, 0xa1);
                t.stubs(&[
                    (mk::GU_WINDOW_SET_TITLE, false, 2, 0, 0xa2),
                    (mk::GU_WINDOW_ADD_BUTTON, false, 7, 0, 0xa3),
                    (mk::GU_WINDOW_ADD_STATIC, false, 7, 0, 0xa4),
                    (mk::SG_GET_FOCUS, false, 1, 0, focus),
                    (mk::SG_SET_FOCUS, false, 2, 0, 0xa5),
                    (mk::SG_TOGGLE_FOCUS, false, 0, 0, 0xa6),
                    (mk::SG_GET_FOCUS_ENABLE, false, 0, 0, rng.below(2)),
                    (mk::GU_PRINTF, false, 2, 0, 0xa7),
                    (mk::GU_WINDOW_CONTROL_SET_TEXT, false, 3, 0, 0xa8),
                ]);
                let w = 0x9300;
                t.check_framed(
                    &format!("focus proc {msg:#x} {id:#x}"),
                    0x0085_4980,
                    Conv::Cdecl,
                    &[w, msg, id],
                    0x18,
                    |m, ebp| Out::U32(mk::focus_proc(m, ebp, w, msg, id)),
                );
            }
        }
    });
}

#[test]
fn r4b1_put_string_core_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_000a);
        for i in 0..300 {
            t.reset();
            let len = match i % 4 {
                0 => rng.below(4),
                1 => 30 + rng.below(8),
                _ => rng.below(80),
            };
            let mut s: Vec<u8> = (0..len)
                .map(|_| match rng.below(8) {
                    0 => 9,
                    1 => rng.below(0x20) as u8 | 1,
                    2 => 0x80 + rng.below(0x80) as u8,
                    _ => 0x20 + rng.below(0x60) as u8,
                })
                .collect();
            s.extend([0, 0]);
            let s = t.alloc(&s);
            let buf = t.zeros(0x1800);
            let end = buf + [0x1800, 0x100, 0x20][i % 3];
            let ctl = t.words(&[buf, 0, 0, end]);
            t.g32(0x0133_c8cc, ctl);
            t.g32(0x0133_c8d0, rng.word());
            let x = match i % 5 {
                0 => 0x2001,
                1 => 0x1f00 + rng.below(0x100),
                _ => rng.below(0x1000),
            };
            t.g32(0x0133_c8d4, x);
            t.g32(0x0133_c8d8, rng.word());
            t.g32(0x0133_c8dc, rng.below(0x400));
            let ret = rng.word();
            t.stubs(&[
                (mk::TK_SET_DEBUG_FONT_BUFFER, false, 1, 0, 0xb1),
                (mk::SET_PTR32, false, 1, 0, ret),
                (mk::DBG_PRINTF, false, 1, 0, 0xb2),
            ]);
            let prop = rng.below(2);
            let colour = rng.below(12).wrapping_sub(2);
            let scale = [rng.below(0x500), 0xffff_fff0, 0x3ff, 0x400][i % 4];
            t.check(
                &format!("TkPutDebugFontStringCore #{i}"),
                0x0088_f240,
                Conv::Cdecl,
                &[prop, colour, scale, s],
                |m| Out::U32(mk::put_string_core(m, prop, colour, scale, s)),
            );
        }
    });
}

#[test]
fn r4b1_mc_callback_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_000b);
        let mirror = (t.rebase(mk::MC_SUM_A) | 0x2000_0000) & !0xfff;
        t.a.map_zeroed(mirror, 0x3000).unwrap();
        t.b.map_zeroed(mirror, 0x3000).unwrap();
        for i in 0..900 {
            t.reset();
            let op = [0u32, 1, 1, 1, 1, 2, 3, 3, 4, 5, 6, 6, 7][i % 13];
            let r = if rng.chance(3) {
                0xffff_fff0
            } else {
                rng.below(0x40)
            };
            let dest = if rng.chance(4) { 0 } else { t.zeros(0x1100) };
            t.stubs(&[
                (mk::SCE_MC_READ_0062F650, false, 3, 0, r),
                (mk::SCE_MC_0062F670, false, 3, 0, r),
                (mk::CRT_MEMCPY, false, 3, 0, 0xc1),
            ]);
            let mut req = vec![0u8; 0x14];
            for b in &mut req {
                *b = rng.next() as u8;
            }
            req[0x10] = rng.below(2) as u8;
            req[0x11] = [0, 1, 2][rng.below(3) as usize];
            let req = t.alloc(&req);
            let buf = t.rebase(mk::MC_BUFFER);
            let head: [u8; 0x14] = std::array::from_fn(|_| rng.next() as u8);
            t.write(buf, &head);
            let b8 = [2u8, 1][rng.below(2) as usize];
            t.write(buf + 8, &[b8]);
            t.g32(0x0132_f578, dest);
            let left = [0u16, 1, 0x80, 0x1200, 0xff80][rng.below(5) as usize];
            let at = t.rebase(0x0132_f57e);
            t.write(at, &left.to_le_bytes());
            let size = [0u32, 0x10, 0x900, 0x1100, 0xffff_ff00][rng.below(5) as usize];
            t.check(
                &format!("mc callback {op} #{i}"),
                0x0088_52a0,
                Conv::Cdecl,
                &[op, size, req],
                |m| Out::U32(mk::mc_callback(m, op, size, req)),
            );
        }
    });
}

// ---- round 4, batch 2 (ffx_menu::m5m_k, the battle-end and menu screens) ----

impl Twins {
    /// A stub over `sym` that logs `n` words under its tag, then returns
    /// `code`'s result (machine code ending in `ret`, run after the log).
    fn stub_then(&mut self, sym: u32, n: u32, code: &[u8]) -> u32 {
        let log = self.stub_code(sym, false, n, 0, 0);
        let mut c = Vec::new();
        for _ in 0..n {
            c.extend([0xff, 0x74, 0x24, (4 * n) as u8]); // push dword [esp+4n]
        }
        c.push(0xe8); // call log
        let here = self.cave_next + c.len() as u32 + 4;
        c.extend(log.wrapping_sub(here).to_le_bytes());
        c.extend([0x83, 0xc4, (4 * n) as u8]); // add esp, 4n
        c.extend(code);
        let addr = self.cave_code(&c);
        self.jump(sym, addr);
        addr
    }
}

#[test]
fn r4b2_btl_end_draw_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0101);
        for i in 0..200u32 {
            t.reset();
            let count = [0u32, 1, 3, 9][i as usize % 4];
            t.stubs(&[
                (mk::TK_VU1_SYNC_PATH, false, 0, 0, 0x11),
                (mk::TK_MENU_DRAW_SCISSOR_CLEAR, false, 0, 0, 0x12),
                (mk::TO_DRAW_MENU_BG, false, 0, 0, 0x13),
                (mk::BTL_END_DRAW_GIL_PAGES, false, 0, 0, 0x15),
                (mk::BTL_END_DRAW_SPOIL_PAGES, false, 0, 0, 0x16),
                (mk::SND_SEP_STOP, false, 1, 0, 0x17),
                (mk::SND_SEP_PLAY_SIMPLE, false, 1, 0, 0x18),
                (mk::BTL_END_DRAW_CAPTION, false, 0, 0, 0x19),
                (mk::TK_BTL_END_DRAW_LEGEND_NOTICE, false, 0, 0, 0x1a),
                (mk::BTL_END_DRAW_FRAME, false, 0, 0, 0x1b),
                (mk::BTL_END_DRAW_CURSOR, false, 0, 0, 0x1c),
                (mk::ATEL_GET_KEEP_PLAY_STATUS, false, 0, 0, rng.below(2)),
                (mk::TK_MENU_DRAW_RECT, false, 6, 0, 0x1d),
                (mk::TK_MENU_DRAW_KICK, false, 0, 0, 0x1e),
                (mk::TO_MENU_CLEAR_MENU_TYPE, false, 1, 0, 0x1f),
                (mk::BTL_END_TAKE_SPOILS, false, 0, 0, 0x20),
                (mk::MS_BTL_GET_INIT, false, 0, 0, 0x21),
                (mk::TK_MENU_MAIN_REMOVE_MENU, false, 1, 0, 0x22),
            ]);
            t.g32(0x0186_9f0c, [0, 1, 2, 0xffff_ffff][rng.below(4) as usize]);
            // each row drawn may shrink the list (the count is re-read)
            let cnt = t.rebase(0x0186_9f18);
            let shrink = if i % 3 == 0 { 0xfe } else { 0 }; // dec byte / nothing
            let mut c = vec![0x80, 0x05];
            c.extend(cnt.to_le_bytes());
            c.push(shrink); // add byte [count], -2 or 0
            c.push(0xc3);
            t.stub_then(mk::TK_BTL_END_DRAW, 1, &c);
            let at = t.rebase(0x0186_9f18);
            t.write(at, &[count as u8]);
            t.g32(0x0186_9fbc, rng.below(2));
            let fade: i16 = match i % 5 {
                0 => 0x1000,
                1 => -(rng.below(0x2000) as i16),
                _ => rng.below(0x1100) as i16,
            };
            let at = t.rebase(0x0186_a0b4);
            t.write(at, &fade.to_le_bytes());
            let at = t.rebase(0x00c5_a068);
            t.write(at, &[rng.below(2) as u8]);
            t.check(
                &format!("BtlEnd_Draw #{i}"),
                0x008b_b7a0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mk::btl_end_draw(m)),
            );
        }
    });
}

#[test]
fn r4b2_btl_end_draw_cursor_and_frame_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0102);
        for i in 0..40u32 {
            t.reset();
            t.stubs(&[
                (mk::MATH_COS_LOOKUP, false, 1, 0, 0x31),
                (mk::MATH_SIN_LOOKUP, false, 1, 0, 0x32),
                (mk::TO_DRAW_MENU_PLATE_XYWH_TYPE, false, 5, 0, 0x33),
                (mk::TO_DRAW_SHEET_CELL, false, 5, 0, rng.word()),
                (mk::TO_MKP_SHAPE_XYWH_UVC2, false, 11, 0, 0x34),
                (mk::TO_MKP_SHAPE_XYWH_UV, false, 9, 0, rng.word()),
                (
                    mk::TO_MKP_CROSS_EXT_MES_FONT_L_CLUT,
                    false,
                    7,
                    0,
                    rng.word(),
                ),
            ]);
            let state = (i % 8) as i16 - 1;
            let at = t.rebase(0x0186_9f24);
            t.write(at, &state.to_le_bytes());
            let at = t.rebase(0x0186_9f22);
            t.write(at, &(rng.next() as i16).to_le_bytes());
            t.check(
                &format!("BtlEnd_DrawCursor {state}"),
                0x008b_b8c0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mk::btl_end_draw_cursor(m)),
            );
            t.g32(0x025c_c588, if i % 2 == 0 { 0 } else { rng.next() | 1 });
            t.check(
                &format!("BtlEnd_DrawFrame #{i}"),
                0x008b_b9c0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mk::btl_end_draw_frame(m)),
            );
        }
    });
}

/// A weapon record for `MsGetSaveWeapon`: exists, slot count, slots.
fn weapon(t: &mut Twins, rng: &mut Rng, i: u32) -> u32 {
    let mut w = vec![0u8; 0x20];
    w[2] = u8::from(!i.is_multiple_of(5));
    w[0xb] = [0u8, 1, 2, 4, 3][(i / 5) as usize % 5];
    for k in 0..4 {
        let a: u16 = [0, 0xff, 0x100 + rng.below(0x80) as u16, 0x2a][rng.below(4) as usize];
        w[0xe + 2 * k..0x10 + 2 * k].copy_from_slice(&a.to_le_bytes());
    }
    t.alloc(&w)
}

#[test]
fn r4b2_weapon_abilities_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0103);
        for i in 0..60u32 {
            t.reset();
            // only the lookup's out word: the rest are the original's f32 temps
            t.skip = 0xc;
            let w = weapon(t, &mut rng, i);
            t.stubs(&[
                (mk::TO_DRAW_SCISSOR_XYWH, false, 4, 0, 0x41),
                (mk::TO_DRAW_MENU_PLATE_XYWH_TYPE, false, 5, 0, 0x42),
                (mk::TO_DRAW_SHEET_CELL, false, 5, 0, 0x43),
                (mk::MS_GET_SAVE_WEAPON, false, 2, 0, w),
                (mk::MS_CALC_WEAPON_CAP, false, 1, 0, 0x44),
                (
                    mk::TO_DRAW_CROSS_MENU_ICON_ABILITY_NAME,
                    false,
                    4,
                    0,
                    0x4500 + i,
                ),
            ]);
            let items: Vec<u8> = (0..0x40).map(|_| rng.next() as u8).collect();
            let items = t.alloc(&items);
            t.g32(0x025c_c580, items);
            t.g32(0x0186_a19c, rng.below(2));
            t.g32(0x0186_a1a8, rng.below(2));
            t.g32(0x0186_a190, rng.below(8));
            t.g32(0x0186_a1a0, rng.below(8));
            t.g32(0x0186_a1a4, rng.below(8));
            let x = match i % 3 {
                0 => rng.below(0x200),
                1 => rng.below(0x200).wrapping_neg(),
                _ => rng.next(),
            };
            t.check_framed(
                &format!("BtlEnd_DrawWeaponAbilities #{i}"),
                0x008b_d020,
                Conv::Cdecl,
                &[x],
                0x10,
                |m, ebp| Out::U32(mk::btl_end_weapon_abilities(m, ebp, x)),
            );
            t.reset();
            t.stubs(&[
                (mk::TO_DRAW_MENU_PLATE_XYWH_TYPE, false, 5, 0, 0x46),
                (mk::MS_GET_SAVE_WEAPON, false, 2, 0, w),
                (mk::MS_CALC_WEAPON_CAP, false, 1, 0, 0x47),
                (
                    mk::TO_DRAW_CROSS_MENU_ICON_ABILITY_NAME,
                    false,
                    4,
                    0,
                    0x4800 + i,
                ),
            ]);
            t.g32(0x0186_aa38, items);
            let row = rng.below(16);
            t.check_framed(
                &format!("MenuKaizou_DrawWeaponAbilities #{i}"),
                0x008d_63f0,
                Conv::Cdecl,
                &[row],
                0x10,
                |m, ebp| Out::U32(mk::kaizou_weapon_abilities(m, ebp, row)),
            );
        }
    });
}

#[test]
fn r4b2_equip_draw_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0104);
        for i in 0..10 {
            t.reset();
            t.stubs(&[
                (mk::TO_DRAW_MENU_BG, false, 0, 0, 0x51),
                (mk::MENU_DRAW_DIVIDER_SHAPES, false, 1, 0, 0x52),
                (mk::TK_MENU_DRAW_KICK, false, 0, 0, 0x53),
                (mk::TK_VU1_SYNC_PATH_THUNK, false, 0, 0, 0x54),
                (mk::TK_MN2_DRAW_INIT_PACKET, false, 0, 0, 0x55),
                (mk::TK_MENU_GET_CURRENT_PLAYER, false, 0, 0, rng.below(8)),
                (mk::MENU_STATUS_DRAW_PAGE, false, 9, 0, 0x56),
                (mk::TO_DRAW_MENU_PLATE_XYWH_TYPE, false, 5, 0, 0x57),
                (mk::TO_DRAW_SHEET_CELL, false, 5, 0, 0x58),
                (mk::TK_MENU_EQUIP_DRAW_CURSOR, false, 0, 0, 0x59),
                (mk::MENU_EQUIP_DRAW_WEAPON_ABILITIES, false, 0, 0, 0x5a),
                (mk::TK_MN2_DRAW_KICK_SYNC_PACKET, false, 0, 0, rng.word()),
            ]);
            t.g32(0x0186_a614, rng.word());
            t.check(
                &format!("MenuEquip_Draw #{i}"),
                0x008c_f670,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mk::equip_draw(m)),
            );
        }
    });
}

/// A message window: kind, flags, the pen's floats, counts.
fn message_window(t: &mut Twins, rng: &mut Rng, i: u32) -> u32 {
    let _ = i;
    let text: Vec<u8> = (0..0x400)
        .map(|_| match rng.below(6) {
            0 => 9,
            1 => 0x10,
            2 => 0x50 + rng.below(0x10) as u8,
            3 => 0xb0 + rng.below(0x50) as u8,
            _ => rng.next() as u8,
        })
        .collect();
    let text = t.alloc(&text);
    let mut w = vec![0u8; 0x100];
    for b in &mut w {
        *b = rng.next() as u8;
    }
    put16(&mut w, 4, rng.below(4) as i16);
    put16(&mut w, 0xe, rng.below(3) as i16);
    put16(&mut w, 0x10, if rng.chance(2) { 0x4000 } else { 0x40 });
    let right = 100.0f32 + rng.below(200) as f32;
    for (o, v) in [
        (
            0x48,
            right - 5.0 + [-1.0f32, 0.0, 0.5, -20.0][rng.below(4) as usize],
        ),
        (0x4c, rng.below(300) as f32),
        (0x50, right),
        (0x54, rng.below(100) as f32 * 0.5),
        (0x58, 1.5),
    ] {
        w[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    w[0x94..0x98].copy_from_slice(&text.to_le_bytes());
    for (o, v) in [
        (0xa0, rng.below(7)),
        (0xa4, rng.below(7)),
        (0xa8, rng.below(9)),
        (0xac, u32::from(rng.chance(3))),
        (0xb4, u32::from(rng.chance(3))),
    ] {
        w[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    if i.is_multiple_of(9) {
        w[0xa0..0xa4].copy_from_slice(&0xffff_fffeu32.to_le_bytes());
    }
    t.alloc(&w)
}

#[test]
fn r4b2_message_text_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0105);
        for i in 0..150u32 {
            t.reset();
            t.skip = 4;
            let win = message_window(t, &mut rng, i);
            for kind in 0..4u32 {
                t.g32(0x025d_0b20 + 4 * kind, rng.below(2));
            }
            t.stub(mk::HARD_CODE_TO_FIX_THE_WRONG_DATA, false, 2, 0, rng.next());
            // the group from the code byte: 9 and 0x10 controls (2), 0xff
            // none (5), else (b & 3) + 1
            let groups: Vec<u8> = (0..=255u8)
                .map(|b| match b {
                    9 | 0x10 => 2,
                    0xff => 5,
                    _ => (b & 3) + 1,
                })
                .collect();
            let groups = t.alloc(&groups);
            let mut c = vec![0x0f, 0xb6, 0x44, 0x24, 0x04]; // movzx eax, byte [esp+4]
            c.extend([0x0f, 0xb6, 0x80]);
            c.extend(groups.to_le_bytes()); // movzx eax, byte [eax+groups]
            c.push(0xc3);
            t.stub_then(mk::TO_GET_GROUP, 1, &c);
            // the next code: one byte on
            t.stub_then(mk::TO_SKIP_CODE, 1, &[0x8b, 0x44, 0x24, 0x04, 0x40, 0xc3]);
            t.stubs(&[
                (mk::TO_CONV_ALBHED_WORD, false, 2, 0, rng.below(4)),
                (mk::TO_GET_FONT_INFO, false, 2, 0, 0x61),
                (mk::TO_MAKE_PKT_FONT_1612, false, 2, 0, 0x62),
                (mk::TO_FIRST_INIT_MESSAGE_NORMAL, false, 3, 0, 0x63),
                (mk::TO_SET_DRAW_INFO, false, 3, 0, 0x64),
                (mk::TO_DRAW_CHANGE_MACRO2, false, 3, 0, 0x65),
            ]);
            let r = t.a.stack_range().end - 0x100 - 4 - 4;
            let ebp = (r - 12) & !15;
            t.check_frame_at(
                &format!("TODrawMessageText #{i}"),
                0x008d_ea40,
                Conv::Cdecl,
                &[win],
                ebp,
                0x64,
                |m, ebp| Out::U32(mk::message_text(m, ebp, win)),
            );
        }
    });
}

#[test]
fn r4b2_ask_yes_no_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0106);
        for i in 0..300u32 {
            t.reset();
            let trg = match i % 4 {
                0 => 0x20,
                1 => 0x40,
                2 => 0,
                _ => rng.next(),
            };
            t.stubs(&[
                (mk::TO_MENU_GET_CONTROL_PAD, false, 0, 0, 0x71),
                (mk::TO_MENU_GET_CONTROL_PAD_REP, false, 0, 0, rng.next()),
                (
                    mk::TO_MENU_GET_CONTROL_PAD_TRG,
                    false,
                    0,
                    0,
                    trg | (rng.next() << 16),
                ),
                (mk::SND_SEP_PLAY_SIMPLE, false, 1, 0, 0x7200 + i),
            ]);
            let mut w = vec![0u8; 0xa0];
            for b in &mut w {
                *b = rng.next() as u8;
            }
            let state = (i / 4) % 10;
            w[0x28..0x2c].copy_from_slice(&state.to_le_bytes());
            put16(&mut w, 0x48, rng.below(2) as i16);
            w[0x37] = rng.below(2) as u8;
            if rng.chance(2) {
                w[0x58..0x5c].fill(0);
            }
            w[0x90..0x94].copy_from_slice(&rng.below(3).to_le_bytes());
            let win = t.alloc(&w);
            t.check(
                &format!("TkMenuCtrlAskYesNoWindow {state} #{i}"),
                0x008e_3730,
                Conv::Cdecl,
                &[win],
                |m| Out::U32(mk::ask_yes_no(m, win)),
            );
        }
    });
}

#[test]
fn r4b2_ask_yes_no_draw_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0107);
        for i in 0..120u32 {
            t.reset();
            let mut w = vec![0u8; 0x70];
            for b in &mut w {
                *b = rng.next() as u8;
            }
            let c1 = if i % 3 == 0 { 0 } else { 0x0010_0000 + i };
            w[0x58..0x5c].copy_from_slice(&c1.to_le_bytes());
            put16(&mut w, 0x48, rng.below(3) as i16);
            w[0x60..0x64].copy_from_slice(&(rng.below(0x40) as i32 - 0x20).to_le_bytes());
            let win = t.alloc(&w);
            let s = t.stub_branch(
                mk::TK_MENU_DRAW_1612_WIDTH_HEIGHT,
                2,
                0,
                c1,
                rng.next() & 0x00ff_01ff,
                rng.next() & 0x00ff_01ff,
            );
            t.jump(mk::TK_MENU_DRAW_1612_WIDTH_HEIGHT, s);
            t.stubs(&[
                (
                    mk::TK_MENU_DRAW_WINDOW_1612,
                    false,
                    4,
                    0,
                    u32::from(i % 4 != 1),
                ),
                (mk::TK_MENU_DRAW_EASY_MES_1612_WITH_MACRO, false, 4, 0, 0x81),
                (mk::TK_MENU_DRAW_EASY_MES_1612, false, 3, 0, 0x82),
                (mk::TK_MENU_DRAW_SP30, false, 3, 0, 0x83),
                (mk::TK_MENU_DRAW_SCISSOR_CLEAR, false, 0, 0, rng.word()),
            ]);
            t.check(
                &format!("TkMenuDrawAskYesNoWindow #{i}"),
                0x008e_3bc0,
                Conv::Cdecl,
                &[win],
                |m| Out::U32(mk::ask_yes_no_draw(m, win)),
            );
        }
    });
}

#[test]
fn r4b2_draw_ui_element_matches_the_original() {
    Twins::each(|t| {
        t.reset();
        t.skip = 4;
        t.stub(mk::GRAPHIC_DRAW_UI_ELEMENT, false, 5, 0, 0x91);
        t.check_framed(
            "MenuDrawUiElement_008e5f60",
            0x008e_5f60,
            Conv::Cdecl,
            &[],
            0x9c,
            |m, ebp| Out::U32(mk::draw_ui_element_008e5f60(m, ebp)),
        );
    });
}

// ---- round 4, batch 3 (ffx_menu::m5m_l, the event debugger's printers) ----

/// What a printer test runs: the original's address and its port.
enum Printer {
    /// `(ebp, worker)` with a frame.
    Framed(fn(&mut G, u32, u32) -> u32),
    /// `(worker)` without one.
    Plain(fn(&mut G, u32) -> u32),
}

/// A debugger worker: random bytes with a head (its first word pointing
/// at a kind byte), a model or none, and a short list at `+0x90`.
fn debug_worker(t: &mut Twins, rng: &mut Rng, i: u32) -> u32 {
    let head = t.alloc(&[[2u8, 1, 0, 1][i as usize % 4], 0, 0, 0]);
    let mut w = vec![0u8; 0xb20];
    for b in &mut w {
        *b = rng.next() as u8;
    }
    w[0..4].copy_from_slice(&head.to_le_bytes());
    let model = if i.is_multiple_of(4) { 0 } else { 0x7700 + i };
    w[0x9c..0xa0].copy_from_slice(&model.to_le_bytes());
    w[0x40..0x44].copy_from_slice(&(i % 0x20).wrapping_sub(1).to_le_bytes());
    w[0xaf] = [0xf, 3, 0x80][i as usize % 3];
    // the list: i % 14 nodes
    let mut next = 0u32;
    for _ in 0..(i % 14) {
        next = t.words(&[next, 0]);
    }
    w[0x90..0x94].copy_from_slice(&next.to_le_bytes());
    t.alloc(&w)
}

impl Twins {
    /// The debugger printers' callees: `sprintf` logging `fmt_words` words,
    /// the font prints, and the lookups.
    fn debugger_stubs(&mut self, rng: &mut Rng, fmt_words: u32) {
        let level = {
            let block = self.alloc(&[0, 0, rng.next() as u8, rng.next() as u8, 0, 0, 0, 0]);
            let mut l = vec![0u8; 0x50];
            l[0x48..0x4c].copy_from_slice(&block.to_le_bytes());
            l[0x1f] = rng.below(4) as u8;
            self.alloc(&l)
        };
        let pos = {
            let p: Vec<u8> = (0..0x40).map(|_| rng.next() as u8).collect();
            let mut p = p;
            p[0x2c..0x30].copy_from_slice(&(rng.below(1000) as f32 * 0.25).to_bits().to_le_bytes());
            self.alloc(&p)
        };
        let ctrl = {
            let mut c: Vec<u8> = (0..0x240).map(|_| rng.next() as u8).collect();
            let id: u16 = if rng.chance(3) {
                0xffff
            } else {
                rng.below(8) as u16
            };
            c[0xa..0xc].copy_from_slice(&id.to_le_bytes());
            for o in [0x218usize, 0x21c, 0x220, 0x228, 0x22c, 0x230] {
                c[o..o + 4]
                    .copy_from_slice(&(rng.below(4000) as f32 / 8.0).to_bits().to_le_bytes());
            }
            self.alloc(&c)
        };
        self.stub_import(ml::SPRINTF_SLOT, false, fmt_words, 0, 0x5c);
        self.stubs(&[
            (ml::TK_PUT_DEBUG_FONT_STRING, false, 3, 0, 0xc1),
            (ml::TK_PUT_DEBUG_FONT_STRING_P, false, 3, 0, 0xc2),
            (ml::TK_PUT_DEBUG_FONT_STRING_P_EUC, false, 3, 0, 0xc3),
            (ml::TK_PUT_DEBUG_FONT_STRING_PR, false, 3, 0, 0xc4),
            (ml::SCRIPT_WORKER_LEVEL, false, 2, 0, level),
            (ml::SCRIPT_PC_OFFSET, false, 2, 0, rng.word()),
            (ml::ATEL_GET_POS_WORK_ADRS, false, 1, 0, pos),
            (ml::ATEL_GET_CTRL_WORK, false, 1, 0, ctrl),
            (ml::ATEL_GET_BASIC_WORK, false, 1, 0, 0xc5),
            (ml::ATEL_DEBUG_PRINT_ENTRY, false, 1, 0, 0xc6),
            (ml::CH_GET_BODY_HIT, false, 1, 0, rng.below(2)),
            (ml::ATEL_GET_WORKER_BITS, false, 1, 0, rng.below(6)),
        ]);
        self.stub_float(ml::CH_GET_THICKNESS, rng.below(100) as f32 * 0.125);
        self.g32(0x0132_8a74, rng.below(8));
        self.g32(0x0132_8a70, rng.below(4));
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one table of printers, then the odd ones
fn r4b3_debugger_printers_match_the_original() {
    let mut printers: Vec<(u32, u32, Printer)> = vec![
        (
            0x0087_b0c0,
            3,
            Printer::Framed(|m, ebp, w| ml::print_motion_no(m, ebp, w)),
        ),
        (
            0x0087_b170,
            3,
            Printer::Framed(|m, ebp, w| ml::print_state(m, ebp, w)),
        ),
        (
            0x0087_b360,
            3,
            Printer::Framed(|m, ebp, w| ml::print_byte_af(m, ebp, w)),
        ),
        (
            0x0087_b3e0,
            3,
            Printer::Plain(|m, w| ml::print_slot_n(m, w, 0)),
        ),
        (
            0x0087_b400,
            3,
            Printer::Plain(|m, w| ml::print_slot_n(m, w, 1)),
        ),
        (
            0x0087_b420,
            3,
            Printer::Plain(|m, w| ml::print_body_hit(m, w)),
        ),
        (
            0x0087_b490,
            4,
            Printer::Framed(|m, ebp, w| ml::print_thickness(m, ebp, w)),
        ),
        (
            0x0087_b520,
            4,
            Printer::Framed(|m, ebp, w| ml::print_pos_2c(m, ebp, w)),
        ),
        (
            0x0087_b610,
            3,
            Printer::Framed(|m, ebp, w| ml::print_level(m, ebp, w)),
        ),
        (
            0x0087_b8f0,
            3,
            Printer::Plain(|m, w| ml::print_bits_name(m, w)),
        ),
        (
            0x0087_b920,
            3,
            Printer::Plain(|m, w| ml::print_flags_34(m, w)),
        ),
        (
            0x0087_b980,
            3,
            Printer::Plain(|m, w| ml::print_flags_38(m, w)),
        ),
        (
            0x0087_b9c0,
            3,
            Printer::Framed(|m, ebp, w| ml::print_pc(m, ebp, w)),
        ),
        (
            0x0087_baa0,
            3,
            Printer::Framed(|m, ebp, w| ml::print_byte_33(m, ebp, w)),
        ),
        (
            0x0087_baf0,
            3,
            Printer::Plain(|m, w| ml::print_flags_ctrl(m, w)),
        ),
        (
            0x0087_bb40,
            3,
            Printer::Framed(|m, ebp, w| ml::print_word_c4(m, ebp, w)),
        ),
        (
            0x0087_bb90,
            3,
            Printer::Framed(|m, ebp, w| ml::print_list_len(m, ebp, w)),
        ),
        (0x0087_e6d0, 3, Printer::Plain(|m, w| ml::print_right(m, w))),
    ];
    for (k, &(addr, _)) in ffx_menu::m5m_l::FLAGS.iter().enumerate() {
        let f: fn(&mut G, u32) -> u32 = match k {
            0 => |m, w| ml::print_flag(m, ffx_menu::m5m_l::FLAGS[0].1, w),
            1 => |m, w| ml::print_flag(m, ffx_menu::m5m_l::FLAGS[1].1, w),
            2 => |m, w| ml::print_flag(m, ffx_menu::m5m_l::FLAGS[2].1, w),
            3 => |m, w| ml::print_flag(m, ffx_menu::m5m_l::FLAGS[3].1, w),
            _ => |m, w| ml::print_flag(m, ffx_menu::m5m_l::FLAGS[4].1, w),
        };
        printers.push((addr, 3, Printer::Plain(f)));
    }
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0201);
        for (addr, words, p) in &printers {
            for i in 0..40u32 {
                t.reset();
                t.skip = 4;
                let w = debug_worker(t, &mut rng, i);
                t.debugger_stubs(&mut rng, *words);
                let what = format!("printer {addr:#x} #{i}");
                match p {
                    Printer::Framed(f) => {
                        let low = if *addr == 0x0087_b490 { 0x28 } else { 0x24 };
                        t.check_framed(&what, *addr, Conv::Cdecl, &[w], low, |m, ebp| {
                            Out::U32(f(m, ebp, w))
                        });
                    }
                    Printer::Plain(f) => {
                        // the slot wrappers call the printer: its frame where the
                        // original's call puts it
                        let wraps = matches!(*addr, 0x0087_b3e0 | 0x0087_b400);
                        if wraps {
                            let top = t.frame_ebp(1);
                            t.b.set_call_top(Some(top));
                        }
                        t.check(&what, *addr, Conv::Cdecl, &[w], |m| Out::U32(f(m, w)));
                        t.b.set_call_top(None);
                    }
                }
            }
        }
        // the slot printer's other slots
        for i in 0..40u32 {
            t.reset();
            t.skip = 4;
            let w = debug_worker(t, &mut rng, i);
            t.debugger_stubs(&mut rng, 3);
            let slot = i % 4;
            t.check_framed(
                &format!("print slot {slot} #{i}"),
                0x0087_ee10,
                Conv::Cdecl,
                &[w, slot],
                0x24,
                |m, ebp| Out::U32(ml::print_slot(m, ebp, w, slot)),
            );
        }
        // the control work's player and floats
        for i in 0..30u32 {
            t.reset();
            t.skip = 4;
            t.debugger_stubs(&mut rng, 4);
            t.check("print ctrl player", 0x0087_b660, Conv::Cdecl, &[], |m| {
                Out::U32(ml::print_ctrl_player(m))
            });
            let (addr, off) = ffx_menu::m5m_l::CTRL_FLOATS[i as usize % 6];
            t.check_framed(
                &format!("ctrl float {addr:#x}"),
                addr,
                Conv::Cdecl,
                &[],
                0x24,
                |m, ebp| Out::U32(ml::print_ctrl_float(m, ebp, off)),
            );
        }
    });
}

#[test]
fn r4b3_set_up_source_debug_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0202);
        for i in 0..200u32 {
            t.reset();
            let len = rng.below(0x3000);
            let src: Vec<u8> = (0..0x3000)
                .map(|_| match rng.below(20) {
                    0..=5 => b'\n',
                    6 if rng.chance(30) => 0x1a,
                    7 if rng.chance(60) => 0,
                    _ => b'a',
                })
                .collect();
            let src = if i % 10 == 0 { 0 } else { t.alloc(&src) };
            let pages = t.zeros(0x400);
            let on = u32::from(i % 7 != 1);
            let size = if i % 4 == 0 {
                0x10 + rng.below(0x20)
            } else {
                rng.below(0x400)
            };
            t.check(
                &format!("AtelSetUpSourceDebug #{i}"),
                0x0088_5b00,
                Conv::Cdecl,
                &[src, len, on, pages, size],
                |m| Out::U32(ml::set_up_source_debug(m, src, len, on, pages, size)),
            );
        }
    });
}

// ---- round 4, batch 3 (ffx_menu::m5m_m, platform, host files, helpers) ----

impl Twins {
    /// A stub over `sym` logging `n` words whose result counts down: it
    /// returns `busy` (and decrements a counter) while the counter, set to
    /// `times`, is above 0, then `done`.
    fn stub_countdown(&mut self, sym: u32, n: u32, times: u32, busy: u32, done: u32) {
        let cnt = self.words(&[times]);
        let mut c = vec![0x83, 0x3d];
        c.extend(cnt.to_le_bytes());
        c.push(0); // cmp dword [cnt], 0
        c.extend([0x74, 0x0c]); // je +12
        c.extend([0xff, 0x0d]);
        c.extend(cnt.to_le_bytes()); // dec dword [cnt]
        c.push(0xb8);
        c.extend(busy.to_le_bytes()); // mov eax, busy
        c.push(0xc3);
        c.push(0xb8);
        c.extend(done.to_le_bytes()); // mov eax, done
        c.push(0xc3);
        self.stub_then(sym, n, &c);
    }

    /// The caller's argument words of a call with `args`, written in `b`
    /// where `a`'s call puts them (for ports that write their slots); `b`'s
    /// own calls go below them.
    fn arg_slots(&mut self, args: &[u32]) -> u32 {
        let slots = self.frame_ebp(args.len()) + 8;
        let words: Vec<u8> = args.iter().flat_map(|v| v.to_le_bytes()).collect();
        self.b.write(slots, &words).unwrap();
        self.b.set_call_top(Some(slots - 0x200));
        slots
    }

    fn same_slots(&mut self, what: &str, slots: u32, n: usize) {
        self.b.set_call_top(None);
        let mut x = vec![0; 4 * n];
        let mut y = vec![0; 4 * n];
        Mem::read_bytes(&self.a, slots, &mut x);
        Mem::read_bytes(&self.b, slots, &mut y);
        assert_eq!(x, y, "{what}: argument slots");
    }
}

#[test]
fn r4b3_mouse_rpcs_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0301);
        for i in 0..12u32 {
            t.reset();
            t.stub_countdown(mm::FFX_SIF_CHECK_STAT_RPC, 1, i % 4, 1 + i, 0);
            let mode = i % 3;
            let slots = t.arg_slots(&[mode, 0x5a5a_0000 + i]);
            t.check(
                &format!("FFXUsbMouseSync {mode} #{i}"),
                0x0085_5bf0,
                Conv::Cdecl,
                &[mode, 0x5a5a_0000 + i],
                |m| Out::U32(mm::usb_mouse_sync(m, mode, slots)),
            );
            t.same_slots("FFXUsbMouseSync", slots, 2);
            t.stub(mm::FFX_SIF_CALL_RPC, false, 9, 0, rng.word());
            let (a, d) = (rng.word(), rng.word());
            t.check("IopMemGet", 0x0085_5c40, Conv::Cdecl, &[d], |m| {
                Out::U32(mm::usb_mouse_iop_mem_get(m, d))
            });
            t.check("Read2", 0x0085_5bc0, Conv::Cdecl, &[a, d], |m| {
                Out::U32(mm::usb_mouse_read2(m, a, d))
            });
        }
    });
}

#[test]
#[allow(clippy::too_many_lines)] // one block per wrapper
fn r4b3_threads_and_semaphores_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0302);
        for i in 0..24u32 {
            t.reset();
            let r = rng.word();
            t.stub_import(mm::RESUME_THREAD, false, 1, 1, r);
            t.stub_import(mm::TERMINATE_THREAD, false, 2, 2, 0xd1);
            t.stub_import(mm::CLOSE_HANDLE, false, 1, 1, 0xd2);
            t.stub_import(mm::CREATE_SEMAPHORE_W, false, 4, 4, 0xd300 + i);
            t.stub_import(mm::SET_THREAD_PRIORITY, false, 2, 2, r);
            t.stub_import(mm::RELEASE_SEMAPHORE, false, 3, 3, rng.word());
            t.stub_import(mm::CREATE_MUTEX_W, false, 3, 3, 0xd400 + i);
            t.stub_import(mm::RELEASE_MUTEX, false, 1, 1, r);
            let a = t.alloc(&[0xff; 0x20]);
            let b = t.alloc(&[0xff; 0x20]);
            let new = t.stub_branch(mm::OPERATOR_NEW, 1, 0, 4, if i % 5 == 4 { 0 } else { a }, b);
            t.jump(mm::OPERATOR_NEW, new);
            t.stubs(&[
                (mm::OPERATOR_DELETE, false, 1, 0, 0xd5),
                (mm::SYS_THREAD_CREATE, true, 4, 4, 0xd6),
                (mm::EH_VECTOR_ITERATOR, false, 4, 4, rng.word()),
            ]);
            // a thread handle: its object, the thread handle at +4
            let obj = t.object(&[(1, 0xd7)]);
            t.w32(obj + 4, if i % 2 == 0 { 0 } else { 0x0e00 + i });
            t.w32(obj + 8, 0x77);
            let handle = t.words(&[obj]);
            let arg = rng.word();
            t.check(
                "StartThread_ps2",
                0x006f_2d30,
                Conv::Cdecl,
                &[handle, arg],
                |m| Out::U32(mm::start_thread(m, handle, arg)),
            );
            t.check(
                "SetThreadPriority",
                0x006f_3050,
                Conv::Cdecl,
                &[handle, arg],
                |m| Out::U32(mm::set_thread_priority(m, handle, arg)),
            );
            let params = t.words(&[0, rng.word(), 0, rng.word()]);
            let name = if i % 3 == 0 { 0 } else { 0x0b00 + i };
            if i % 5 != 4 {
                t.check(
                    "CreateThread_ps2",
                    0x006f_2e10,
                    Conv::Cdecl,
                    &[params, name],
                    |m| Out::U32(mm::create_thread(m, params, name)),
                );
            }
            let count = rng.below(4);
            t.check("CreateSema", 0x006f_2b10, Conv::Cdecl, &[count], |m| {
                Out::U32(mm::create_sema(m, count))
            });
            // the handle allocated, its object not
            let p = t.alloc(&[0xff; 4]);
            t.stub_countdown(mm::OPERATOR_NEW, 1, 1, p, 0);
            t.check("CreateSema half", 0x006f_2b10, Conv::Cdecl, &[count], |m| {
                Out::U32(mm::create_sema(m, count))
            });
            t.check("Mutex_Create", 0x006f_2de0, Conv::Cdecl, &[], |m| {
                Out::U32(mm::mutex_create(m))
            });
            let mx = t.words(&[0x0f00 + i]);
            t.stub_import(mm::WAIT_FOR_SINGLE_OBJECT, false, 2, 2, r);
            t.check("Mutex_Lock", 0x006f_2ed0, Conv::Cdecl, &[mx], |m| {
                Out::U32(mm::mutex_lock(m, mx))
            });
            t.check("Mutex_Unlock", 0x006f_2f50, Conv::Cdecl, &[mx], |m| {
                Out::U32(mm::mutex_unlock(m, mx))
            });
            let sema = t.words(&[mx]);
            let s = if i % 4 == 0 { 0 } else { sema };
            t.check("SignalSema", 0x006f_3000, Conv::Cdecl, &[s], |m| {
                Out::U32(mm::signal_sema(m, s))
            });
            // poll: -1 a few times, then the result
            t.reset();
            t.stub_import(mm::RELEASE_SEMAPHORE, false, 3, 3, 0xd8);
            let done = [0u32, 1, 0x80, 0x102][i as usize % 4];
            let cnt = t.words(&[i % 3]);
            let log = t.stub_code(mm::WAIT_FOR_SINGLE_OBJECT, false, 2, 0, 0);
            let mut c = vec![0x83, 0x3d];
            c.extend(cnt.to_le_bytes());
            c.push(0); // cmp dword [cnt], 0
            c.extend([0x74, 0x1c]); // je to the result
            c.extend([0xff, 0x0d]);
            c.extend(cnt.to_le_bytes()); // dec dword [cnt]
            for _ in 0..2 {
                c.extend([0xff, 0x74, 0x24, 8]); // push dword [esp+8]
            }
            c.push(0xe8);
            let here = t.cave_next + c.len() as u32 + 4;
            c.extend(log.wrapping_sub(here).to_le_bytes()); // call log
            c.extend([0x83, 0xc4, 8]); // add esp, 8
            c.extend([0x83, 0xc8, 0xff]); // or eax, -1
            c.extend([0xc2, 8, 0]); // ret 8
            for _ in 0..2 {
                c.extend([0xff, 0x74, 0x24, 8]); // push dword [esp+8]
            }
            c.push(0xe8);
            let here = t.cave_next + c.len() as u32 + 4;
            c.extend(log.wrapping_sub(here).to_le_bytes()); // call log
            c.extend([0x83, 0xc4, 8]); // add esp, 8
            c.push(0xb8);
            c.extend(done.to_le_bytes()); // mov eax, done
            c.extend([0xc2, 8, 0]); // ret 8
            let code = t.cave_code(&c);
            let slot = t.rebase(mm::WAIT_FOR_SINGLE_OBJECT);
            t.patch(slot, &code.to_le_bytes());
            let s = if i % 6 == 5 { 0 } else { sema };
            t.check("PollSema", 0x006f_2d50, Conv::Cdecl, &[s], |m| {
                Out::U32(mm::poll_sema(m, s))
            });
        }
    });
}

#[test]
fn r4b3_thread_objects_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0303);
        for i in 0..16u32 {
            t.reset();
            let menu = t.zeros(0xa0);
            t.stub_import(mm::TERMINATE_THREAD, false, 2, 2, 0xe1);
            t.stub_import(mm::CLOSE_HANDLE, false, 1, 1, 0xe2);
            t.stub_import(mm::WAIT_FOR_SINGLE_OBJECT, false, 2, 2, 0xe3);
            t.stub_import(mm::RELEASE_SEMAPHORE, false, 3, 3, rng.word());
            t.stubs(&[
                (mm::OPERATOR_DELETE, false, 1, 0, 0xe4),
                (mm::EH_VECTOR_ITERATOR, false, 4, 4, rng.word()),
                (
                    mm::OPERATOR_NEW,
                    false,
                    1,
                    0,
                    if i % 4 == 0 { 0 } else { menu },
                ),
                (mm::IGGY_MENU_CTOR, true, 6, 6, 0xe5),
                (mm::IGGY_MENU_SET_ENABLE, true, 1, 1, rng.word()),
            ]);
            let obj = t.object(&[(1, 0xe6)]);
            t.w32(obj + 4, if i % 2 == 0 { 0 } else { 0x0e00 + i });
            let handle = t.words(&[if i % 5 == 0 { 0 } else { obj }]);
            if i % 5 != 0 {
                t.check(
                    "DeleteThread_ps2",
                    0x006f_2f00,
                    Conv::Cdecl,
                    &[handle],
                    |m| Out::U32(mm::delete_thread(m, handle)),
                );
            }
            let th = t.words(&[0x55, if i % 3 == 0 { 0 } else { 0x0e10 + i }, 0x66]);
            let flags = rng.below(4);
            t.check(
                "Ps2Thread_dtor",
                0x006f_29a0,
                Conv::Thiscall(th),
                &[flags],
                |m| Out::U32(mm::thread_dtor(m, th, flags)),
            );
            let q = t.zeros(0x1e8 * 8 + 0x40);
            for k in 0..8u32 {
                let o = q + k * 0x1e8;
                t.w32(o + 0x1f4, rng.below(16).wrapping_sub(8));
                t.w32(o + 0x1f8, rng.below(8));
                t.w32(o + 0x220, rng.below(16).wrapping_sub(8));
                t.w32(o + 0x228, rng.below(8));
                for j in 0..8 {
                    t.w32(o + 0x200 + 4 * j, rng.word());
                }
            }
            let k = rng.below(8);
            let job = rng.word();
            t.check("job push", 0x006d_b800, Conv::Thiscall(q), &[job, k], |m| {
                Out::U32(mm::job_push(m, q, job, k))
            });
            t.check("job pop", 0x006d_b8a0, Conv::Thiscall(q), &[k], |m| {
                Out::U32(mm::job_pop(m, q, k))
            });
            t.check("job queue dtor", 0x006d_b730, Conv::Thiscall(q), &[], |m| {
                Out::U32(mm::job_queue_dtor(m, q))
            });
            t.g32(0x00ce_81fc, if i % 3 == 0 { 0x1234 } else { 0 });
            let path = rng.word();
            t.check("overlay menu", 0x006f_3130, Conv::Cdecl, &[path], |m| {
                mm::overlay_menu_create(m, path);
                Out::None
            });
        }
    });
}

#[test]
fn r4b3_host_files_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0304);
        for i in 0..24u32 {
            t.reset();
            t.skip = 4;
            let fd = [0u32, 7, 0xffff_ffff, 9][i as usize % 4];
            let fd2 = [0u32, 7, 0, 0xffff_fff0][(i / 4) as usize % 4];
            let s = t.stub_branch(mm::SCE_OPEN, 2, 1, 0x203, fd2, fd);
            t.jump(mm::SCE_OPEN, s);
            t.stubs(&[
                (mm::SCE_LSEEK, false, 3, 0, 0x100 + rng.below(0x40_0000)),
                (mm::SCE_READ, false, 3, 0, rng.word()),
                (mm::SCE_WRITE, false, 3, 0, rng.word()),
                (mm::SCE_CLOSE, false, 1, 0, 0xf1),
                (mm::FLUSH_CACHE, false, 1, 0, rng.word()),
                (mm::DBG_PRINTF, false, 1, 0, 0xf2),
                (mm::TK_SYNC_V, false, 1, 0, 0xf3),
                (mm::TK_MEM_LOCK, false, 1, 0, 0xf4),
                (mm::TK_MEM_UNLOCK, false, 1, 0, 0xf5),
            ]);
            let first = rng.below(2) as u8;
            let at = t.rebase(0x0112_8aae);
            t.write(at, &[first]);
            let (buf, n) = (rng.word(), rng.word());
            t.check("dump write", 0x0076_e890, Conv::Cdecl, &[buf, n], |m| {
                Out::U32(mm::dump_write(m, buf, n))
            });
            let name = t.alloc(b"event/x.bin\0");
            for (k, sym) in [0x0087_9c60u32, 0x0087_9cf0, 0x0087_9d60]
                .into_iter()
                .enumerate()
            {
                let f = |m: &mut G, buf: u32, name: u32| match k {
                    0 => mm::pc_create(m, buf, name),
                    1 => mm::pc_open(m, buf, name),
                    _ => mm::pc_open2(m, buf, name),
                };
                t.check_framed(
                    &format!("{sym:#x} #{i}"),
                    sym,
                    Conv::Cdecl,
                    &[name],
                    0x104,
                    |m, ebp| Out::U32(f(m, ebp - 0x104, name)),
                );
            }
            t.check("AtelPCRead", 0x0087_9de0, Conv::Cdecl, &[fd, buf, n], |m| {
                Out::U32(mm::pc_read(m, fd, buf, n))
            });
            t.check("AtelPCSize", 0x0087_9e10, Conv::Cdecl, &[fd], |m| {
                Out::U32(mm::pc_size(m, fd))
            });
            t.check(
                "AtelPCWrite",
                0x0087_9e50,
                Conv::Cdecl,
                &[fd, buf, n],
                |m| Out::U32(mm::pc_forward(m, mm::SCE_WRITE, fd, buf, n)),
            );
            t.check(
                "AtelPClseek",
                0x0087_9e60,
                Conv::Cdecl,
                &[fd, buf, n],
                |m| Out::U32(mm::pc_forward(m, mm::SCE_LSEEK, fd, buf, n)),
            );
            let slots = t.arg_slots(&[fd]);
            t.check("AtelPCClose", 0x0087_9c30, Conv::Cdecl, &[fd], |m| {
                Out::U32(mm::pc_close(m, fd, slots))
            });
            t.same_slots("AtelPCClose", slots, 1);
        }
    });
}

#[test]
fn r4b3_debug_files_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0305);
        for i in 0..24u32 {
            t.reset();
            t.skip = 4;
            let fd = [0u32, 7, 0xffff_fff0][i as usize % 3];
            t.stubs(&[
                (mm::ATEL_PC_OPEN, false, 1, 0, fd),
                (mm::ATEL_PC_OPEN2, false, 1, 0, fd),
                (mm::ATEL_PC_CLOSE, false, 1, 0, rng.word()),
                (mm::ATEL_PC_WRITE, false, 3, 0, 0xf6),
                (mm::ATEL_PC_LSEEK, false, 3, 0, 0xf7),
                (
                    mm::SCE_LSEEK,
                    false,
                    3,
                    0,
                    [0x10u32, 0x40_0000, 0x10_0001][i as usize % 3],
                ),
                (mm::SCE_CLOSE, false, 1, 0, 0xf8),
                (mm::FLUSH_CACHE, false, 1, 0, 0xf9),
                (mm::DBG_PRINTF, false, 3, 0, 0xfa),
                (mm::TK_MEM_LOCK, false, 1, 0, rng.word()),
                (mm::TK_MEM_UNLOCK, false, 1, 0, 0xfb),
                (mm::ATEL_EVENT_GET_FILE_PATH2, false, 5, 0, 0xfd),
            ]);
            let entry: Vec<u8> = [rng.below(0x100), rng.below(0x40)]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            t.stub_write(mm::SCE_READ, 3, 0xfe, 1, &entry);
            t.stub_import(mm::SPRINTF, false, 2, 0, 0xff);
            t.check_framed(
                "AtelOutputEncountStart",
                0x0088_5a80,
                Conv::Cdecl,
                &[],
                0x404,
                |m, ebp| Out::U32(mm::output_encount_start(m, ebp - 0x404)),
            );
            let ev = rng.below(0x100);
            // only the index entry: the rest is the stack probe's
            t.skip = 0x2004 - 8;
            t.check_framed(
                "AtelGetEventName",
                0x0087_9760,
                Conv::Cdecl,
                &[ev],
                0x2004,
                |m, ebp| Out::U32(mm::get_event_name(m, ebp - 0x2004, ev)),
            );
            t.skip = 4;
            for k in 0..7u32 {
                let b = t.zeros(0x10);
                t.g32(0x0132_8598 + 4 * k, b);
            }
            for k in 0..64u32 {
                let b = t.zeros(0x10);
                t.g32(0x0132_8698 + 4 * k, b);
            }
            t.stub(mm::ATEL_GET_EVENT_NAME, false, 1, 0, 0xfc);
            let slot = rng.below(7);
            t.check(
                "AtelLoadDebugData",
                0x0087_99b0,
                Conv::Cdecl,
                &[slot, ev],
                |m| Out::U32(mm::load_debug_data(m, slot, ev)),
            );
            let (a, b) = (rng.below(7), rng.below(8));
            let name = rng.word();
            t.check(
                "Atel_LoadSourceFile",
                0x0087_9b60,
                Conv::Cdecl,
                &[a, 0x1234, b, name],
                |m| Out::U32(mm::load_source_file(m, a, b, name)),
            );
        }
    });
}

#[test]
fn r4b3_debugger_setup_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0306);
        for i in 0..6u32 {
            t.reset();
            for k in 0..7u32 {
                t.g32(
                    0x0132_8598 + 4 * k,
                    if rng.chance(2) { 0 } else { rng.word() | 1 },
                );
            }
            for k in 0..56u32 {
                t.g32(
                    0x0132_8698 + 4 * k,
                    if rng.chance(2) { 0 } else { rng.word() | 1 },
                );
                t.g32(
                    0x0132_85b8 + 4 * k,
                    if rng.chance(2) { 0 } else { rng.word() | 1 },
                );
            }
            t.stub(mm::USER_FREE_THUNK, false, 1, 0, rng.word());
            t.check("AtelRemovePS2Debug", 0x0087_9e70, Conv::Cdecl, &[], |m| {
                Out::U32(mm::remove_ps2_debug(m))
            });
            t.reset();
            t.stub_countdown(mm::USER_MALLOC, 1, 1000, 0x1000_0000 + i, 0);
            t.stubs(&[
                (mm::TK_MEM_LOCK, false, 1, 0, 0x101),
                (mm::INIT_DEBUGGER, false, 0, 0, rng.word()),
            ]);
            t.check("AtelInitPS2Debug", 0x0087_98f0, Conv::Cdecl, &[], |m| {
                Out::U32(mm::init_ps2_debug(m))
            });
            t.reset();
            t.stubs(&[
                (mm::TK_IS_DEBUG, false, 0, 0, i % 2),
                (mm::ATEL_INIT_PS2_DEBUG, false, 0, 0, 0x102),
            ]);
            t.check("init total debug", 0x0087_3ba0, Conv::Cdecl, &[], |m| {
                Out::U32(mm::init_total_debug(m))
            });
            t.check("clear debugger", 0x0087_f1c0, Conv::Cdecl, &[], |m| {
                Out::U32(mm::clear_debugger(m))
            });
            t.reset();
            t.stubs(&[
                (mm::CLEAR_DEBUGGER, false, 0, 0, 0x103),
                (mm::SET_GLOBALS_0086E970, false, 2, 0, 0x104),
                (mm::TK_SET_START_MAP_INDEX, false, 2, 0, rng.word()),
            ]);
            t.check("init debugger", 0x0087_f0d0, Conv::Cdecl, &[], |m| {
                Out::U32(mm::init_debugger(m))
            });
        }
    });
}

#[test]
fn r4b3_script_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0307);
        for i in 0..120u32 {
            t.reset();
            let mut ctx = vec![0u8; 0x20];
            ctx[0] = rng.next() as u8;
            ctx[1] = if rng.chance(4) { 4 } else { 0 };
            ctx[3] = if rng.chance(4) { 0x80 } else { 0 };
            let p: u16 = if rng.chance(4) {
                0xffff
            } else {
                rng.below(8) as u16
            };
            ctx[0xa..0xc].copy_from_slice(&p.to_le_bytes());
            ctx[0xe..0x10].copy_from_slice(&(rng.below(8) as u16).to_le_bytes());
            let ctx = t.alloc(&ctx);
            t.g32(0x0132_6b28, ctx);
            let work = t.alloc(&[0x5a; 0x40]);
            let save = t.zeros(0x100);
            let words: Vec<u32> = (0..0x40).map(|_| rng.next()).collect();
            let words = t.words(&words);
            let table = t.words(&[0, 0, 0, 0, 0, 0, 0, 0, words - 0x10]);
            let wk = t.words(&[table, 0x10]);
            let wtab = t.stub_branch(mm::ATEL_GET_BASIC_WORK, 1, 0, 0x7777, wk, work);
            t.jump(mm::ATEL_GET_BASIC_WORK, wtab);
            t.stubs(&[
                (mm::ATEL_UC_OFF, false, 0, 0, 0x111),
                (mm::ATEL_UC_ON, false, 0, 0, 0x112),
                (mm::EVENT_SET_PLAYER, false, 1, 0, 0x113),
                (mm::TK_GET_SAVE_EVENT_ADDRESS, false, 0, 0, save),
                (mm::ATEL_REGIST_CALLBACK_QUE2, false, 3, 0, rng.word()),
            ]);
            let id = match i % 5 {
                0 => 0xffff_ffff,
                1 => 0xffff,
                2 => 0x8000_0000,
                _ => rng.below(8),
            };
            t.check(
                &format!("AtelSetCtrlChar {id:#x}"),
                0x0086_f580,
                Conv::Cdecl,
                &[id],
                |m| Out::U32(mm::set_ctrl_char(m, id)),
            );
            t.w32(save + 0xe0, rng.below(8));
            t.check("save step 6", 0x0087_3db0, Conv::Cdecl, &[], |m| {
                Out::U32(mm::save_event_step6(m))
            });
            for (sym, to) in [(0x0087_3c40u32, 4u32), (0x0087_3d30, 1)] {
                t.w32(save + 0xe0, rng.below(8));
                t.check(&format!("save step {to}"), sym, Conv::Cdecl, &[], |m| {
                    Out::U32(mm::save_event_step(m, to))
                });
            }
            let k = rng.below(8);
            t.check(
                "worker table word",
                0x0086_c250,
                Conv::Cdecl,
                &[0x7777, k],
                |m| Out::U32(mm::worker_table_word(m, 0x7777, k)),
            );
            let keep = rng.below(3);
            t.check("regist callbacks", 0x0086_1440, Conv::Cdecl, &[keep], |m| {
                Out::U32(mm::regist_debug_callbacks(m, keep))
            });
        }
    });
}

#[test]
fn r4b3_battle_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0308);
        for i in 0..40u32 {
            t.reset();
            let name = t.rebase(mm::BTL_FILE_NAME);
            t.write(name, b"ffx/battle/btl/abcd/efgh/x.bin\0");
            t.stub_import(mm::STRSTR, false, 2, 0, name + 4);
            t.stub_import(mm::STRRCHR, false, 2, 0, name + 24);
            let text = if i % 3 == 0 { 0 } else { 0x0a10_0000 + i };
            t.stubs(&[
                (mm::TK_FREE, false, 1, 0, 0x121),
                (mm::TK_MALLOC_AND_READ_EVENT_MESSAGE, false, 1, 0, text),
                (mm::TK_MALLOC_AND_READ_EVENT_FTC, false, 1, 0, rng.word()),
                (mm::CHN_SYSTEM_MGRP_LOADED, false, 0, 0, rng.below(2)),
                (mm::CHN_SYSTEM_MGRP_RESET_TAIL, false, 0, 0, 0x122),
            ]);
            t.g32(0x0231_12c4, if rng.chance(2) { 0 } else { rng.word() | 1 });
            t.g32(0x0231_12c0, if rng.chance(2) { 0 } else { rng.word() | 1 });
            t.g32(0x0231_12bc, rng.below(2));
            t.check("MsBtlReadAsyncTask", 0x0078_4500, Conv::Cdecl, &[], |m| {
                Out::U32(mm::btl_read_async(m))
            });
            t.g32(0x0113_606c, rng.below(2));
            t.check("ChnSystemMgrpReset", 0x0079_cca0, Conv::Cdecl, &[], |m| {
                mm::chn_system_mgrp_reset(m);
                Out::None
            });
        }
    });
}

#[test]
fn r4b3_one_liners_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0309);
        for (sym, value) in [
            (0x0078_29a0u32, 0x00b5_46a0u32),
            (0x0086_0770, 0x00b5_b9fc),
            (0x0087_8130, 0x00b5_b9fc),
            (0x0087_9420, 0x00b5_b9fc),
        ] {
            t.check(&format!("{sym:#x}"), sym, Conv::Cdecl, &[], |m| {
                Out::U32(mm::rdata_address(m, value))
            });
        }
        for _ in 0..4 {
            t.g32(0x0132_6b58, rng.word());
            t.check("0x0086b060", 0x0086_b060, Conv::Cdecl, &[], |m| {
                Out::U32(mm::global_01326b58(m))
            });
            let at = t.rebase(0x0112_a8e0);
            t.write(at, &[rng.next() as u8]);
            t.check("MsBtlReadFileDone", 0x0078_2fe0, Conv::Cdecl, &[], |m| {
                mm::btl_read_file_done(m);
                Out::None
            });
            let at = t.rebase(0x0132_6b81);
            t.write(at, &[rng.next() as u8]);
            t.check("0x008708d0", 0x0087_08d0, Conv::Cdecl, &[], |m| {
                mm::set_global_01326b81(m);
                Out::None
            });
        }
    });
}

// ---- round 4, batch 4 (ffx_menu::m5m_m, the battle debug camera) ----

#[test]
fn r4b4_debug_camera_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0401);
        for i in 0..20u32 {
            t.reset();
            let recs = t.alloc(&[0xff; 0x40 * 33]);
            t.stub_countdown(mm::MS_DEBUG_WIN_ALLOC, 1, 100, recs + 0x40 * (i % 3), 0);
            t.stub(mm::MS_DEBUG_CAMERA_KEEP_007D7130, false, 0, 0, rng.word());
            t.check("MsDebugWinAllocAll", 0x007d_6440, Conv::Cdecl, &[], |m| {
                Out::U32(mm::debug_win_alloc_all(m))
            });
            t.check("MsDebugCameraInit", 0x007d_62c0, Conv::Cdecl, &[], |m| {
                Out::U32(mm::debug_camera_init(m))
            });
            let at = t.rebase(0x0113_f528);
            t.write(at, &[0xa5; 0x180]);
            t.check("MsDebugWinStateClear", 0x007d_7a20, Conv::Cdecl, &[], |m| {
                Out::U32(mm::debug_win_state_clear(m))
            });
            // the cameras' positions
            let cams: Vec<u8> = (0..0x4000).map(|_| rng.next() as u8).collect();
            let at = t.rebase(0x0113_7b8c);
            t.write(at, &cams);
            for k in 0..30u32 {
                // a keep: the count below, at and past 32
                let n = [0u32, 5, 31, 32, 40, 1][(i + k) as usize % 6];
                t.g32(0x0113_ed7c, n);
                let screen = rng.below(2);
                t.g32(0x0113_9f10, screen);
                let at = t.rebase(0x0113_70c1 + screen * 0x1c0);
                t.write(at, &[rng.below(4) as u8]);
                for sym in [
                    0x0113_ed64u32,
                    0x0113_ed68,
                    0x0113_ed6c,
                    0x0113_ed70,
                    0x0113_ed74,
                ] {
                    let v = if i % 2 == 0 {
                        (rng.below(7200) as f32 / 10.0 - 360.0).to_bits()
                    } else {
                        (rng.next() & 0x3fff_ffff) | 0x3000_0000
                    };
                    t.g32(sym, v);
                }
                t.check("MsDebugCameraKeepAdd", 0x007d_0ee0, Conv::Cdecl, &[], |m| {
                    Out::U32(mm::debug_camera_keep_add(m))
                });
            }
        }
    });
}

#[test]
fn r4b4_btl_end_menu_step_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0402);
        for i in 0..64u32 {
            t.reset();
            let trg = (i & 1) | if i & 2 != 0 { 0x40 } else { 0 } | (rng.next() & 0xffbe);
            let at = t.rebase(0x025d_09d4);
            t.write(at, &(trg as u16).to_le_bytes());
            let rep =
                [0u32, 0x1000, 0x4000, 0x5000][(i / 4) as usize % 4] | (rng.next() & 0xffff_afff);
            t.stubs(&[
                (mm::SND_PLAY_CLICK, false, 1, 0, rng.word()),
                (mm::TO_INIT_MESSAGE_SLOT, false, 1, 0, 0x131),
                (mm::TO_MENU_GET_CONTROL_PAD_REP2, false, 0, 0, rep),
            ]);
            t.check("BtlEnd menu step", 0x008b_e1d0, Conv::Cdecl, &[], |m| {
                Out::U32(mm::btl_end_menu_step(m))
            });
        }
    });
}

// ---- round 4, batch 5 (ffx_menu::m5m_m, the battle debug sliders) ----

/// A slider's object: floats at `+0x30..+0x4c`, indices at `+0xc`/`+0xe`.
fn slider_object(t: &mut Twins, rng: &mut Rng, i: u32) -> u32 {
    let mut o = vec![0u8; 0x50];
    for k in (0x30..0x50).step_by(4) {
        let v: f32 = match (i + k as u32) % 4 {
            0 => rng.below(2000) as f32 / 7.0 - 140.0,
            1 => (rng.below(720) as f32 - 360.0) * 0.017_453_3,
            2 => rng.below(1_000_000) as f32 * 3.3,
            _ => -(rng.below(5000) as f32) / 3.0,
        };
        o[k..k + 4].copy_from_slice(&v.to_bits().to_le_bytes());
    }
    for k in [0xcusize, 0xe] {
        let v = rng.below(40) as i16 - 4;
        o[k..k + 2].copy_from_slice(&v.to_le_bytes());
    }
    t.alloc(&o)
}

#[test]
fn r4b5_debug_sliders_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0501);
        // the CRT's SSE2 flag both ways (the image has not run its start-up)
        for sse in [0u32, 1] {
            t.g32(0x0193_ed20, sse);
            for id in 0x25..0x36u32 {
                for k in 0..6u32 {
                    t.reset();
                    let obj = slider_object(t, &mut rng, k);
                    let p = match k % 3 {
                        0 => (rng.below(100) as f32 / 3.0).to_bits(),
                        1 => (-(rng.below(1000) as f32)).to_bits(),
                        _ => 2.5f32.to_bits(),
                    };
                    let flags = match k % 3 {
                        0 => 0x5_0000,
                        1 => 0x7_0000 | (rng.next() & 0xfff0_ffff),
                        _ => rng.next() & 0xfff0_ffff,
                    };
                    t.stub(
                        mm::ATEL_GET_TOTAL_ACTOR_NUMBER2,
                        false,
                        0,
                        0,
                        [0u32, 3, 0x40, 0xffff_ffff][(k / 2) as usize % 4],
                    );
                    let args = [obj, id, p, flags];
                    for (sym, n) in [
                        (0x007d_6dc0u32, 3usize),
                        (0x007d_abc0, 3),
                        (0x007d_0af0, 3),
                        (0x007d_6470, 3),
                        (0x007d_95e0, 4),
                    ] {
                        let slots = t.arg_slots(&args[..n]);
                        t.check(
                            &format!("slider {sym:#x} id {id:#x} #{k} sse {sse}"),
                            sym,
                            Conv::Cdecl,
                            &args[..n],
                            |m| {
                                Out::U32(match sym {
                                    0x007d_0af0 => mm::slider_camera(m, obj, id, p, slots),
                                    0x007d_6470 => mm::slider_keep_angle(m, obj, id, p, slots),
                                    0x007d_95e0 => mm::slider_actors(m, obj, id, p, flags, slots),
                                    _ => mm::slider_xyz(m, obj, id, p, slots),
                                })
                            },
                        );
                        t.same_slots(&format!("slider {sym:#x} id {id:#x}"), slots, n);
                    }
                }
            }
        }
    });
}

#[test]
fn r4b5_debug_keep_windows_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7e4b_0502);
        for i in 0..24u32 {
            t.reset();
            let wins: Vec<u8> = (0..0x40 * 32).map(|_| rng.next() as u8).collect();
            let wins = t.alloc(&wins);
            for k in 0..32u32 {
                t.g32(0x0113_ec98 + 4 * k, wins + 0x40 * k);
            }
            // finite floats: the emulator keeps signalling NaNs the CPU quiets
            let keeps: Vec<u8> = (0..0xc * 32)
                .flat_map(|_| ((rng.next() & 0x3fff_ffff) | 0x0080_0000).to_le_bytes())
                .collect();
            let at = t.rebase(0x0113_ed8c);
            t.write(at, &keeps);
            t.g32(0x0113_ed7c, [0u32, 5, 32, 40][i as usize % 4]);
            t.g32(0x0113_ed5c, rng.below(34));
            t.g32(0x0113_9eec, rng.below(2));
            t.stub(
                mm::MS_DEBUG_GET_CAMERA_MODE,
                false,
                1,
                0,
                [7u32, 8, 9, 0][(i / 4) as usize % 4],
            );
            let all = rng.below(2);
            t.check("keep windows", 0x007d_71d0, Conv::Cdecl, &[all], |m| {
                Out::U32(mm::debug_keep_windows(m, all))
            });
        }
    });
}
