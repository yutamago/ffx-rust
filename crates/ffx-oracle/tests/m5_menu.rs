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
