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
