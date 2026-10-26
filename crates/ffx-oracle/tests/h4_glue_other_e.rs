//! Continuation round 3 of list `h4-glue-other` (`ffx_glue_other::h4_e*`)
//! against the original: on the image alone and on a snapshot
//! (`FFX_SNAPSHOT`). Results and every byte written must agree. Needs
//! `FFX_GAME_DIR`.
//!
//! As in `h4_glue_other.rs`, callees that stay original are replaced in
//! both oracles alike by recording stubs (tag, `ecx` for methods, stack
//! words, in a log in memory; the result from a cell), so the diff checks
//! the calls too. Float callees' stubs return their cell's `f64` in `ST0`.
//! The x87 cursor code reads image constants: the tests overwrite them
//! with random floats in both oracles, so a port that reads the wrong one
//! differs.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_glue_other::{
    frames as fr, h4_c2 as c2, h4_c3 as c3, h4_d1 as d1, h4_d3 as d3, h4_e1 as e1, h4_e2 as e2,
    h4_e3 as e3, h4_e4 as e4, h4_e5 as e5, h4_f1 as f1, h4_f2 as f2, h4_f3 as f3, h4_f4 as f4,
    h4_f5 as f5, h4_f6 as f6, h4_f7 as f7, h4_g1 as g1, h4_g2 as g2, h4_h1 as h1, h4_h2 as h2,
    h4_h3 as h3, h4_h4 as h4, h4_h5 as h5,
};
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};

/// `graphicFontDraw`'s body: no test here calls it. The intrinsic
/// trampoline goes first (`fld qword [esp+4]; call [esp+0xc]; ret`, then
/// at `+16` `fstp st0; ret`), the recording stubs after it.
const CAVE: u32 = 0x0064_0660;
const CAVE_LEN: u32 = 1768;
const TRAMPOLINE: [u8; 19] = [
    0xdd, 0x44, 0x24, 0x04, 0xff, 0x54, 0x24, 0x0c, 0xc3, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
    0xdd, 0xd8, 0xc3,
];

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
    fn pick<T: Copy>(&mut self, v: &[T]) -> T {
        v[self.below(v.len() as u32) as usize]
    }
    /// Float bits that are not a signalling NaN (the emulator keeps
    /// those; the hardware quiets them): mostly ordinary values.
    fn float(&mut self) -> u32 {
        loop {
            let bits = match self.below(6) {
                0 => self.next(),
                1 => [
                    0,
                    0x8000_0000,
                    0x7f80_0000,
                    0xff80_0000,
                    0x7fc0_0000,
                    0xffc0_1234,
                    1,
                    0x7f7f_ffff,
                ][self.below(8) as usize],
                _ => (self.below(20_000) as f32 / 7.0 - 1000.0).to_bits(),
            };
            if !ffx_bridge::x87::is_snan(bits) {
                return bits;
            }
        }
    }
    /// An `f64` constant: mostly ordinary, sometimes 0, infinite or NaN.
    fn double(&mut self) -> f64 {
        match self.below(8) {
            0 => [0.0, -0.0, f64::INFINITY, f64::NAN, 1e300][self.below(5) as usize],
            _ => f64::from(self.below(100_000) as f32 / 37.0 - 1000.0) * 1.000_000_1,
        }
    }
    /// A small signed word.
    fn small(&mut self) -> u32 {
        (self.below(2000) as i32 - 1000) as u32
    }
}

/// The bridge's view of an oracle; `.1` is the intrinsic trampoline.
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

impl G<'_> {
    fn ret(&mut self, addr: u32, c: host::Conv, args: &[u32], st0: bool) -> ffx_oracle::Ret {
        ffx_oracle::check_width("Host::call", addr, args.len(), host::max_words(c, st0));
        self.0
            .call(addr, conv(c), args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
    }
    /// Pops the callee's `ST0`, as the caller's `fstp` does.
    fn pop(&mut self) {
        self.0.call(self.1 + 16, Conv::Cdecl, &[]).unwrap();
    }
}

impl Host for G<'_> {
    fn call(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> u32 {
        self.ret(addr, c, args, false).eax
    }
    fn call_st0(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> f32 {
        let r = self.ret(addr, c, args, true);
        let v = r.st0_f32().expect("the callee's ST0");
        self.pop();
        v
    }
    fn call_st0_f64(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> f64 {
        let r = self.ret(addr, c, args, true);
        let v = r.st0_f64().expect("the callee's ST0");
        self.pop();
        v
    }
    fn call_u64(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> u64 {
        let r = self.ret(addr, c, args, true);
        u64::from(r.edx) << 32 | u64::from(r.eax)
    }
    fn call_intrinsic(&mut self, addr: u32, x: f64) -> f32 {
        let b = x.to_bits();
        let r = self
            .0
            .call(self.1, Conv::Cdecl, &[b as u32, (b >> 32) as u32, addr])
            .unwrap_or_else(|e| panic!("intrinsic {addr:#x}: {e}"));
        let v = r.st0_f32().expect("the intrinsic's ST0");
        self.pop();
        v
    }
    fn scratch(&mut self) -> u32 {
        self.0.stack_range().start + 0x100
    }
}

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
}

/// What the original returns, to compare.
#[derive(Debug, Clone, Copy)]
enum Out {
    /// Nothing the callers read.
    None,
    /// `eax`.
    U32(u32),
    /// `edx:eax`.
    U64(u64),
}

fn u(v: u32) -> Out {
    Out::U32(v)
}

/// `Some(eax)`, or for `None` the `eax` the last (void) callee's stub
/// leaves: `last`.
fn opt(v: Option<u32>, last: u32) -> Out {
    Out::U32(v.unwrap_or(last))
}

/// A cell a stub stores through one of its pointer arguments before it
/// logs (a callee filling the caller's locals).
struct Write {
    arg: u8,
    cell: u32,
    /// Only the cell's first byte.
    byte: bool,
}

fn wr(arg: u8, cell: u32) -> Write {
    Write {
        arg,
        cell,
        byte: false,
    }
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 16 words (tag, `ecx`, up to
    /// 14 stack words).
    log: u32,
    tramp: u32,
    cave: u32,
    cave_next: u32,
    /// The result cell of each stub, by tag.
    cells: HashMap<u32, u32>,
    /// The `ST0` cell (`f64`) of each float stub, by tag.
    fcells: HashMap<u32, u32>,
    /// The `edx` cell of each 64-bit stub, by tag.
    hi_cells: HashMap<u32, u32>,
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
            tramp: cave,
            cave,
            cave_next: cave,
            cells: HashMap::new(),
            fcells: HashMap::new(),
            hi_cells: HashMap::new(),
        };
        t.log = t.zeros(0x4_0000);
        t.tramp = t.code(&TRAMPOLINE);
        t.cave_next = cave + 0x20;
        t
    }

    /// The image, then the snapshot when `FFX_SNAPSHOT` is set.
    fn all() -> Vec<(&'static str, Self)> {
        let Some(dir) = game_dir() else {
            return vec![];
        };
        let exe = dir.join("FFX.exe");
        let mut out = vec![(
            "image",
            Self::new(Oracle::load(&exe).unwrap(), Oracle::load(&exe).unwrap()),
        )];
        if let Some(snap) = std::env::var_os("FFX_SNAPSHOT") {
            let pages = Pages::read(PathBuf::from(snap)).unwrap();
            out.push((
                "snapshot",
                Self::new(
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
                ),
            ));
        } else {
            eprintln!("FFX_SNAPSHOT is not set; image only");
        }
        out
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

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    fn w16(&mut self, at: u32, v: u16) {
        self.write(at, &v.to_le_bytes());
    }

    fn w8(&mut self, at: u32, v: u8) {
        self.write(at, &[v]);
    }

    /// Writes global `sym` (an `ffx.sym` address).
    fn g32(&mut self, sym: u32, v: u32) {
        let at = self.rebase(sym);
        self.w32(at, v);
    }

    /// Random float constants at the image addresses `f32s` and `f64s`.
    fn constants(&mut self, rng: &mut Rng, f32s: &[u32], f64s: &[u32]) {
        for &c in f32s {
            let v = rng.float();
            self.g32(c, v);
        }
        for &c in f64s {
            let v = rng.double();
            let at = self.rebase(c);
            self.write(at, &v.to_le_bytes());
        }
    }

    /// A recording stub at the original `sym` (a `jmp` there): logs `sym`,
    /// `ecx` when `this`, and `n` stack words; pops `pops` words; returns
    /// its cell ([`Twins::ret`]), and with `float` its `f64` cell in `ST0`
    /// ([`Twins::ret_f64`]).
    fn stub(&mut self, sym: u32, n: u32, float: bool) {
        let addr = self.stub_code(sym, false, n, 0, float, &[]);
        self.jump(sym, addr);
    }

    /// [`Twins::stub_code`] jumped to from the original `sym`.
    fn stub_at(&mut self, sym: u32, this: bool, n: u32, pops: u32, writes: &[Write]) {
        let addr = self.stub_code(sym, this, n, pops, false, writes);
        self.jump(sym, addr);
    }

    /// A `jmp` from the original `sym` to `addr`.
    fn jump(&mut self, sym: u32, addr: u32) {
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
    }

    /// The stub of [`Twins::stub`] in the cave only, `tag` its log tag,
    /// first storing `writes` through its pointer arguments. Returns its
    /// address.
    fn stub_code(
        &mut self,
        tag: u32,
        this: bool,
        n: u32,
        pops: u32,
        float: bool,
        writes: &[Write],
    ) -> u32 {
        self.stub_code_edx(tag, this, n, pops, float, writes, None)
    }

    /// A 64-bit stub at the original `sym`: [`Twins::stub_at`] that also
    /// returns `edx` from a second cell ([`Twins::ret_u64`]).
    fn stub_u64(&mut self, sym: u32, this: bool, n: u32, pops: u32) {
        let hi = self.zeros(4);
        self.hi_cells.insert(sym, hi);
        let addr = self.stub_code_edx(sym, this, n, pops, false, &[], Some(hi));
        self.jump(sym, addr);
    }

    /// Sets the `edx:eax` result of the stub tagged `tag`.
    fn ret_u64(&mut self, tag: u32, v: u64) {
        self.ret(tag, v as u32);
        let hi = self.hi_cells[&tag];
        self.w32(hi, (v >> 32) as u32);
    }

    /// [`Twins::stub_code`], with `edx` loaded from `edx` at the end.
    #[allow(clippy::too_many_arguments)]
    fn stub_code_edx(
        &mut self,
        tag: u32,
        this: bool,
        n: u32,
        pops: u32,
        float: bool,
        writes: &[Write],
        edx: Option<u32>,
    ) -> u32 {
        assert!(n <= 14);
        let cell = self.zeros(4);
        self.cells.insert(tag, cell);
        let mut c = Vec::new();
        for w in writes {
            c.extend([0x8b, 0x44, 0x24, 4 + 4 * w.arg]); // mov eax, [esp+4+4i]
            if w.byte {
                c.extend([0x8a, 0x15]);
                c.extend(w.cell.to_le_bytes()); // mov dl, [cell]
                c.extend([0x88, 0x10]); // mov [eax], dl
            } else {
                c.extend([0x8b, 0x15]);
                c.extend(w.cell.to_le_bytes()); // mov edx, [cell]
                c.extend([0x89, 0x10]); // mov [eax], edx
            }
        }
        let log = self.log;
        c.push(0xa1);
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
        c.push(0xa1);
        c.extend(cell.to_le_bytes()); // mov eax, [cell]
        if float {
            let f = self.zeros(8);
            self.fcells.insert(tag, f);
            c.extend([0xdd, 0x05]);
            c.extend(f.to_le_bytes()); // fld qword [f]
        }
        if let Some(hi) = edx {
            c.extend([0x8b, 0x15]);
            c.extend(hi.to_le_bytes()); // mov edx, [hi]
        }
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        self.code(&c)
    }

    /// Machine code in the cave; its address.
    fn code(&mut self, c: &[u8]) -> u32 {
        let addr = self.cave_next;
        assert!(addr + c.len() as u32 <= self.cave + CAVE_LEN, "cave full");
        self.write(addr, c);
        self.cave_next = addr + c.len() as u32;
        addr
    }

    /// Sets the result of the stub tagged `tag`.
    fn ret(&mut self, tag: u32, v: u32) {
        let cell = self.cells[&tag];
        self.w32(cell, v);
    }

    /// Sets the `ST0` result of the float stub tagged `tag`.
    fn ret_f64(&mut self, tag: u32, v: f64) {
        let cell = self.fcells[&tag];
        self.write(cell, &v.to_le_bytes());
    }

    /// Resets the log and sets every listed stub's result to a random word.
    fn fresh(&mut self, rng: &mut Rng, tags: &[u32]) {
        let log = self.log;
        self.w32(log, 0);
        for &tag in tags {
            let v = rng.next();
            self.ret(tag, v);
        }
    }

    /// The result the stub tagged `tag` returns.
    fn cell(&self, tag: u32) -> u32 {
        Mem::read(&self.b, self.cells[&tag])
    }

    fn log_entries(&self) -> u32 {
        Mem::read(&self.b, self.log)
    }

    fn diff(&mut self, what: &str) {
        let d = self.a.diff(&mut self.b).unwrap();
        assert!(
            d.is_empty(),
            "{what}: memory differs at {} bytes, first {:x?}",
            d.len(),
            &d[..d.len().min(8)]
        );
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// Runs the original at `sym` in `a` and `port` on `b`; compares the
    /// result as `port` says and every byte written. Returns the
    /// original's `eax`.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        args: &[u32],
        port: impl FnOnce(&mut G) -> Out,
    ) -> u32 {
        self.check_conv(what, sym, Conv::Cdecl, args, port)
    }

    /// [`Twins::check`] with the original's convention.
    fn check_conv(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        port: impl FnOnce(&mut G) -> Out,
    ) -> u32 {
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let tramp = self.tramp;
        let out = port(&mut G(&mut self.b, tramp));
        assert!(
            ra.st0_raw.is_none(),
            "{what}: the original leaves ST0 set ({:?})",
            ra.st0
        );
        match out {
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::U64(v) => {
                assert_eq!(ra.eax, v as u32, "{what}: eax");
                assert_eq!(ra.edx, (v >> 32) as u32, "{what}: edx");
            }
            Out::None => {}
        }
        self.diff(what);
        ra.eax
    }

    /// Where the original's `ebp` lands for a call with `words` stack
    /// arguments.
    fn ebp_for(&self, words: u32) -> u32 {
        assert_eq!(self.a.stack_range(), self.b.stack_range());
        self.a.stack_range().end - 0x100 - 4 * words - 8
    }

    /// [`Twins::check`] for an original whose frame (`frame`) callees see:
    /// the port's oracle gets the same block, its own calls below it;
    /// afterwards the bytes of `compare` (offsets and lengths in the frame)
    /// must agree.
    fn check_framed(
        &mut self,
        what: &str,
        sym: u32,
        args: &[u32],
        frame: u32,
        compare: &[(u32, usize)],
        port: impl FnOnce(&mut G) -> Out,
    ) -> u32 {
        self.check_framed_conv(what, sym, Conv::Cdecl, args, frame, compare, port)
    }

    /// [`Twins::check_framed`] with the original's convention.
    #[allow(clippy::too_many_arguments)]
    fn check_framed_conv(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        frame: u32,
        compare: &[(u32, usize)],
        port: impl FnOnce(&mut G) -> Out,
    ) -> u32 {
        for &(off, len) in compare {
            let junk: Vec<u8> = (0..len).map(|k| 0xa5 ^ k as u8).collect();
            self.write(frame + off, &junk);
        }
        self.b.set_call_top(Some(frame - 0x100));
        let eax = self.check_conv(what, sym, conv, args, port);
        self.b.set_call_top(None);
        for &(off, len) in compare {
            let mut x = vec![0; len];
            let mut y = vec![0; len];
            self.a.read_bytes(frame + off, &mut x);
            self.b.read_bytes(frame + off, &mut y);
            assert_eq!(x, y, "{what}: frame +{off:#x}");
        }
        eax
    }
}

// ---- batch 1 (ffx_glue_other::h4_e1)

/// The list cursors: the equipment menu's, the weapon list's, the four
/// weapon-shop cursors and the item list's.
#[test]
fn e1_list_cursors() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e101);
        let stubs = [
            (e1::WEAPON_MES_POS, 0),
            (e1::CROSS_CMD_CURSOR, 3),
            (e1::CROSS_CURSOR, 3),
            (e1::MENU_TIMER, 0),
        ];
        for (sym, n) in stubs {
            t.stub(sym, n, false);
        }
        let tags = stubs.map(|s| s.0);
        let f32s = [
            0x00b6_5e48,
            0x00b6_64e0,
            0x00b6_64dc,
            0x00b5_eecc,
            0x00b6_5e4c,
            0x00b6_636c,
            0x00b6_6720,
            0x00b5_870c,
            0x00b9_2314,
            0x00b5_a480,
        ];
        let block = t.zeros(0x500);
        let list = block + 0x280;
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(&mut rng, &f32s, &[]);
            let timer = rng.below(256);
            t.ret(e1::MENU_TIMER, timer);
            let pos = rng.small();
            t.ret(e1::WEAPON_MES_POS, pos);
            for k in 0..0x140 {
                let v = rng.next();
                t.w32(block + 4 * k, v);
            }
            let cursor = rng.small() as u16;
            let half = rng.pick(&[0u16, 1, 5, 0x8000, 0xffff]);
            t.w16(list + 0x48, rng.pick(&[cursor, half]));
            let scroll = rng.small() as u16;
            t.w16(list + 0x32, scroll);
            let count = rng.pick(&[0u8, 1, 2, 2, 5, 10, 0xff]);
            t.w8(list + 0x42, count);
            let entry = rng.pick(&[0u8, 1, 3, 9, 0xff, 0x80, 0x7f, count]);
            t.w8(list + 0x45, entry);
            for k in 0..12 {
                let v = {
                    let r0_ = rng.next();
                    rng.pick(&[0, 0, 1, r0_])
                };
                t.w32(list + 0x58 + 4 * k, v);
            }
            let fade = rng.pick(&[0xfff, 0x1000, 0x2000, 0x8000_0000, 0x7fff_ffff]);
            t.g32(e1::EQUIP_FADE.sym, fade);
            let alt = {
                let r0_ = rng.next();
                rng.pick(&[0, 1, 3, r0_])
            };
            t.g32(e1::EQUIP_ALT_MODE.sym, alt);
            let open = rng.next();
            t.g32(e1::EQUIP_LIST_OPEN.sym, open);
            let mode = rng.pick(&[0, 1, 2, 2, 3]);
            t.g32(e1::ITEM_TARGET_MODE.sym, mode);
            let (x, y) = (rng.float(), rng.float());
            let cmd = t.cell(e1::CROSS_CMD_CURSOR);
            t.check(&format!("{w} equip"), 0x008c_f990, &[], |g| {
                e1::equip_cursor(g);
                Out::None
            });
            let (ix, iy) = (rng.small(), {
                let r0_ = rng.small();
                let r1_ = rng.next();
                rng.pick(&[r0_, r1_])
            });
            t.check(
                &format!("{w} weapon list"),
                0x008d_0100,
                &[list, ix, iy],
                |g| {
                    e1::weapon_list_cursor(g, list, ix, iy);
                    Out::None
                },
            );
            for sym in [0x008d_8bb0, 0x008d_8e60] {
                t.check(&format!("{w} shop list {sym:x}"), sym, &[list, x, y], |g| {
                    u(e1::shop_list_cursor(g, list, x, y))
                });
            }
            t.check(&format!("{w} shop buy"), 0x008d_ab70, &[list, x, y], |g| {
                u(e1::shop_buy_cursor(g, list, x, y))
            });
            t.check(&format!("{w} shop sell"), 0x008d_ae00, &[list, x, y], |g| {
                u(e1::shop_sell_cursor(g, list, x, y))
            });
            t.check(&format!("{w} item list"), 0x008c_7af0, &[list, x, y], |g| {
                opt(e1::item_list_cursor(g, list, x, y), cmd)
            });
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The shape cursors, the brushes and the backgrounds.
#[test]
fn e1_shapes() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e102);
        let stubs = [
            (e1::SHAPE_UV, 9, false),
            (e1::SHAPE_UV_C2, 11, false),
            (e1::MENU_TIMER, 0, false),
            (e1::GET_TIME, 0, true),
        ];
        for (sym, n, float) in stubs {
            t.stub(sym, n, float);
        }
        let tags = stubs.map(|s| s.0);
        let f32s = [
            0x00b5_eec0,
            0x00b5_ee90,
            0x00b5_1b9c,
            0x00b6_5854,
            0x00b6_57fc,
            0x00b6_585c,
            0x00b3_eb98,
            0x00b6_5820,
            0x00b6_5804,
            0x00b5_f0c8,
            0x00b9_2314,
            0x00b6_589c,
            0x00b5_5178,
            0x00b4_a1ac,
            0x00b4_1d98,
            0x00b4_1d9c,
            0x00b4_9988,
            0x00b6_61c8,
            0x00b6_6230,
            0x00b5_67a8,
        ];
        let f64s = [
            0x00b9_22a8,
            0x00b2_c5c8,
            0x00b6_6d08,
            0x00b5_8b50,
            0x00b5_1ba8,
            0x00b4_1d30,
        ];
        let brushes = t.rebase(e1::BRUSHES);
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(&mut rng, &f32s, &f64s);
            let timer = rng.below(256);
            t.ret(e1::MENU_TIMER, timer);
            let time = {
                let r0_ = rng.below(100_000);
                rng.pick(&[0.0, 1.5, 1e9, -3.25, f64::from(r0_) / 60.0])
            };
            t.ret_f64(e1::GET_TIME, time);
            let (x, y) = (rng.float(), rng.float());
            let blink = rng.pick(&[0, 1, 0x100]);
            let shape = t.cell(e1::SHAPE_UV);
            t.check(
                &format!("{w} cmd cursor"),
                0x008c_0580,
                &[x, y, blink],
                |g| opt(e1::cross_cmd_cursor(g, x, y, blink), shape),
            );
            let hidden = rng.pick(&[0, 0, 1, 0x8000_0000]);
            t.check(&format!("{w} stati"), 0x008c_13d0, &[x, y, hidden], |g| {
                e1::stati_cursor(g, x, y, hidden);
                Out::None
            });
            let (a, ix, iy) = (rng.next(), rng.small(), rng.next());
            t.check(&format!("{w} brush"), 0x008b_f830, &[a, ix, iy], |g| {
                e1::draw_brush(g, ix, iy);
                Out::None
            });
            for k in 0..4u32 {
                let e = brushes + 16 * k;
                t.w16(e, rng.pick(&[0, 1, 0x100]));
                for off in [2, 4, 6] {
                    let v = rng.small() as u16;
                    t.w16(e + off, v);
                }
                let steps = rng.pick(&[0u16, 1, 8, 30, 0x7fff, 0x8000]);
                t.w16(e + 0xc, steps);
                let step = rng.pick(&[0, 1, steps, steps.wrapping_sub(1), 0x7fff]);
                t.w16(e + 0xa, step);
                t.w16(e + 8, rng.next() as u16);
            }
            let k = rng.below(4);
            t.check(&format!("{w} brush2 {k}"), 0x008f_f260, &[k], |g| {
                e1::draw_brush2(g, k);
                Out::None
            });
            let (ca, cb) = (rng.next(), rng.next());
            t.check(&format!("{w} bg c2"), 0x008f_5c60, &[ca, cb], |g| {
                e1::draw_menu_bg_c2(g, ca, cb);
                Out::None
            });
            let colour = {
                let r0_ = rng.next();
                rng.pick(&[0, 0x1f, 0x20, 0xffff_ffff, r0_])
            };
            let kind = rng.pick(&[0, 1, 2, 4, 5]);
            t.check(
                &format!("{w} status bg {kind}"),
                0x008d_4170,
                &[colour, kind],
                |g| {
                    e1::draw_status_bg(g, colour, kind);
                    Out::None
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The weapon ability lists.
#[test]
fn e1_abilities() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e103);
        let stubs = [
            (e1::CALC_WEAPON_CAP, 1),
            (e1::MENU_PLATE, 5),
            (e1::ABILITY_NAME, 4),
            (e1::SAVE_WEAPON, 2),
        ];
        for (sym, n) in stubs {
            t.stub(sym, n, false);
        }
        let tags = stubs.map(|s| s.0);
        let f32s = [
            0x00b3_eb9c,
            0x00b5_eff4,
            0x00b5_eec0,
            0x00b9_2268,
            0x00b6_5724,
            0x00b5_efcc,
            0x00b6_5e48,
        ];
        let weapon = t.zeros(0x20);
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(&mut rng, &f32s, &[]);
            for k in 0..8 {
                let v = rng.next();
                t.w32(weapon + 4 * k, v);
            }
            let slots = rng.pick(&[0u8, 1, 2, 4, 7]);
            t.w8(weapon + 0xb, slots);
            for k in 0..4 {
                let id = {
                    let r0_ = rng.next();
                    rng.pick(&[0u16, 0xff, 0x100, r0_ as u16])
                };
                t.w16(weapon + 0xe + 2 * k, id);
            }
            t.ret(e1::SAVE_WEAPON, weapon);
            let (x, y) = (rng.float(), rng.float());
            t.check(
                &format!("{w} abilities"),
                0x008d_8aa0,
                &[x, y, weapon],
                |g| {
                    e1::weapon_abilities(g, x, y, weapon);
                    Out::None
                },
            );
            let item = rng.pick(&[0xff, 0, 3, 0x1ff]);
            t.g32(e1::EQUIP_ITEM.sym, item);
            let frame = t.ebp_for(0) - e1::EQUIP_ABILITIES_FRAME;
            t.check_framed(
                &format!("{w} equip abilities"),
                0x008d_02e0,
                &[],
                frame,
                &[(0, 4)],
                |g| match e1::equip_weapon_abilities(g, frame) {
                    // the remaps' eax otherwise
                    None if item != 0xff => Out::None,
                    r => u(r.unwrap_or(!item)),
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The battle target cursors and the enemies' limit gauges.
#[test]
fn e1_battle() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e104);
        let cx = t.zeros(4);
        let cy = t.zeros(4);
        let pos = [wr(1, cx), wr(2, cy)];
        let f = t.stub_code(e1::TARGET_POS, false, 3, 0, false, &pos);
        t.jump(e1::TARGET_POS, f);
        let f = t.stub_code(e1::GROUND_POS, false, 3, 0, false, &pos);
        t.jump(e1::GROUND_POS, f);
        let stubs = [
            (e1::SET_ENV_CURSOR, 0),
            (e1::CROSS_CURSOR, 3),
            (e1::BATTLE_END, 1),
            (e1::PKT_SCISSOR, 4),
            (e1::LIMIT_BAR_FLAG, 1),
            (e1::LIMIT_GAUGE, 5),
        ];
        for (sym, n) in stubs {
            t.stub(sym, n, false);
        }
        let mut tags = stubs.map(|s| s.0).to_vec();
        tags.extend([e1::TARGET_POS, e1::GROUND_POS]);
        let f32s = [0x00b9_2314, 0x00b5_ef78];
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(&mut rng, &f32s, &[0x00b9_2220]);
            let (x, y) = (rng.small(), {
                let r0_ = rng.small();
                let r1_ = rng.next();
                rng.pick(&[r0_, r1_])
            });
            t.w32(cx, x);
            t.w32(cy, y);
            let hide = rng.pick(&[0, 1, 2]);
            t.g32(e1::TARGET_HIDE.sym, hide);
            let mask = {
                let r0_ = rng.next();
                let r1_ = rng.below(32);
                rng.pick(&[0, u32::MAX, r0_, 1 << r1_])
            };
            t.g32(e1::TARGET_MASK.sym, mask);
            let frame = t.ebp_for(0) - e1::TARGET_FRAME;
            t.check_framed(
                &format!("{w} targets"),
                0x0089_c870,
                &[],
                frame,
                &[(4, 8)],
                |g| {
                    e1::target_cursor(g, frame);
                    Out::None
                },
            );
            let end = rng.pick(&[0, 1, 2]);
            t.ret(e1::BATTLE_END, end);
            let bar = rng.pick(&[1, 1, 0, 2]);
            t.ret(e1::LIMIT_BAR_FLAG, bar);
            let count = rng.pick(&[0, 1, 3, 8, u32::MAX]);
            t.g32(e1::ENEMY_COUNT.sym, count);
            for k in 0..8 {
                let id = {
                    let r0_ = rng.next();
                    rng.pick(&[0xffu16, 0, 5, r0_ as u16])
                };
                let at = t.rebase(e1::ENEMY_IDS) + 2 * k;
                t.w16(at, id);
                let v = rng.next();
                let at = t.rebase(e1::ENEMY_GAUGES) + 16 * k;
                t.w32(at, v);
            }
            let frame = t.ebp_for(0) - e1::ENEMY_LIMIT_FRAME;
            t.check_framed(
                &format!("{w} enemy limit"),
                0x0089_33c0,
                &[],
                frame,
                &[(8, 4), (0x10, 4)],
                |g| {
                    e1::enemy_limit(g, frame);
                    Out::None
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

// ---- batch 2 (ffx_glue_other::h4_e2)

/// A volume: in and out of 0..1, signed zeros, NaN.
fn volume(rng: &mut Rng) -> u32 {
    let r = rng.float();
    rng.pick(&[
        0,
        0x8000_0000,
        1f32.to_bits(),
        0.5f32.to_bits(),
        (-0.25f32).to_bits(),
        1.5f32.to_bits(),
        0x7fc0_0000,
        r,
        r,
    ])
}

/// Random music tracks (`n` records of 0x3c bytes): mostly unloaded,
/// playing or not, pending or not.
fn tracks(t: &mut Twins, rng: &mut Rng, at: u32, n: u32) {
    for i in 0..n {
        let e = at + e2::TRACK * i;
        for k in 0..e2::TRACK / 4 {
            let v = rng.next();
            t.w32(e + 4 * k, v);
        }
        let r = rng.next() | 1;
        let ev = rng.pick(&[0, 0, 0, r]);
        t.w32(e, ev);
        for off in [8, 0xc, 0x20] {
            let v = rng.pick(&[0, 1, 1, 2]);
            t.w32(e + off, v);
        }
        for off in [0x34, 0x38] {
            let v = volume(rng);
            t.w32(e + off, v);
        }
    }
}

/// The music's direct and whole volume.
#[test]
fn e2_music_volume() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e201);
        t.stub_at(fr::FMOD_SET_VOLUME, false, 2, 2, &[]);
        t.stub_at(fr::ERRCHECK, false, 3, 0, &[]);
        let tags = [fr::FMOD_SET_VOLUME, fr::ERRCHECK];
        let this = t.zeros(0x40);
        let arr = t.zeros((e2::TRACK * (e2::TRACKS + 1)) as usize);
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let d = rng.double();
            let k = rng.pick(&[100.0, 127.0, 0.0, d]);
            let at = t.rebase(e2::VOLUME_DIVISOR);
            t.write(at, &k.to_le_bytes());
            for k in 0..0x10 {
                let v = rng.next();
                t.w32(this + 4 * k, v);
            }
            t.w32(this + 0x10, arr);
            let mute = rng.pick(&[0, 1, 2]);
            t.w32(this + 0x24, mute);
            let scale = volume(&mut rng);
            t.w32(this + 0x30, scale);
            tracks(&mut t, &mut rng, arr, e2::TRACKS + 1);
            let r = rng.below(0xb6);
            let idx = rng.pick(&[0, 1, 0x82, 0xb5, 0xb6, r, u32::MAX]);
            let (v, keep) = (volume(&mut rng), rng.pick(&[0, 1, 2]));
            t.check_conv(
                &format!("{w} direct {idx:#x}"),
                0x0070_9dd0,
                Conv::Thiscall(this),
                &[idx, v, keep],
                |g| u(e2::music_volume_direct(g, this, idx, v, keep)),
            );
            let other = volume(&mut rng);
            let v = rng.pick(&[other, scale]);
            t.check_conv(
                &format!("{w} entire"),
                0x0070_93e0,
                Conv::Thiscall(this),
                &[v],
                |g| {
                    e2::music_set_entire_volume(g, this, v);
                    Out::None
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The music's fade setup and the old tracks' fade-out.
#[test]
fn e2_music_fades() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e202);
        let state = t.zeros(4);
        let vol = t.zeros(4);
        let paused = t.zeros(4);
        t.stub_at(fr::FMOD_GET_STATE, false, 2, 2, &[wr(1, state)]);
        t.stub_at(e2::EVENT_GET_VOLUME, false, 2, 2, &[wr(1, vol)]);
        let byte = Write {
            arg: 1,
            cell: paused,
            byte: true,
        };
        t.stub_at(e2::EVENT_GET_PAUSED, false, 2, 2, &[byte]);
        t.stub_at(fr::ERRCHECK, false, 3, 0, &[]);
        t.stub_at(e2::MUSIC_VOLUME_DIRECT, true, 3, 3, &[]);
        t.stub_at(e2::MUSIC_FADE_OUT, true, 2, 2, &[]);
        let tags = [
            fr::FMOD_GET_STATE,
            e2::EVENT_GET_VOLUME,
            e2::EVENT_GET_PAUSED,
            fr::ERRCHECK,
            e2::MUSIC_VOLUME_DIRECT,
            e2::MUSIC_FADE_OUT,
        ];
        let this = t.zeros(0x40);
        let arr = t.zeros((e2::TRACK * (e2::TRACKS + 1)) as usize);
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let d = rng.double();
            let k = rng.pick(&[100.0, 127.0, d]);
            let at = t.rebase(e2::VOLUME_DIVISOR);
            t.write(at, &k.to_le_bytes());
            for k in 0..0x10 {
                let v = rng.next();
                t.w32(this + 4 * k, v);
            }
            t.w32(this + 0x10, arr);
            let mode = rng.pick(&[0x84, 0x84, 0x15d, 0x15d, 0, 0x85]);
            t.w32(this + 0x28, mode);
            tracks(&mut t, &mut rng, arr, e2::TRACKS + 1);
            let flag = rng.pick(&[1, 1, 0, 2]);
            t.w32(arr + 0x1e80, flag);
            let st = rng.pick(&[8, 8, 0, 0xff, 0xf7]);
            t.w32(state, st);
            let v = volume(&mut rng);
            t.w32(vol, v);
            let p = rng.pick(&[0, 0, 1, 0x100]);
            t.w32(paused, p);
            let r = rng.below(0xb6);
            let idx = rng.pick(&[0x82, 0x82, 0, 0xb5, 0xb6, r]);
            let (r1, r2) = (rng.next() | 1, rng.next() | 1);
            let ev = rng.pick(&[0, r1, r2]);
            if idx < e2::TRACKS {
                t.w32(arr + e2::TRACK * idx, ev);
            }
            let r = rng.next();
            let frames = rng.pick(&[
                0,
                1,
                0x78,
                0x79,
                0xb4,
                0xb5,
                0x168,
                0x169,
                0x169,
                0x200,
                0xb5,
                u32::MAX,
                0x8000_0000,
                r,
            ]);
            let r = rng.next();
            let target = rng.pick(&[0, 1, 100, u32::MAX, r]);
            let frame = t.ebp_for(3) - e2::TRANSLATE_FRAME;
            t.check_framed_conv(
                &format!("{w} translate {idx:#x} {frames:#x}"),
                0x0070_9ff0,
                Conv::Thiscall(this),
                &[idx, frames, target],
                frame,
                &[(0, 4)],
                |g| {
                    u(e2::music_volume_translate(
                        g, frame, this, idx, frames, target,
                    ))
                },
            );
            let r = rng.below(0xb6);
            let keep = rng.pick(&[0, 1, 0xb5, 0xb6, r]);
            let frame = t.ebp_for(1) - e2::RELEASE_OLD_FRAME;
            t.check_framed_conv(
                &format!("{w} release old {keep:#x}"),
                0x0070_91b0,
                Conv::Thiscall(this),
                &[keep],
                frame,
                &[(0, 4), (7, 1)],
                |g| {
                    e2::release_old_music(g, frame, this, keep);
                    Out::None
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The voice's loop fade.
#[test]
fn e2_voice_loop() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e203);
        let state = t.zeros(4);
        t.stub_at(fr::FMOD_GET_STATE, false, 2, 2, &[wr(1, state)]);
        t.stub_at(fr::ERRCHECK, false, 3, 0, &[]);
        t.stub_at(e2::VOICE_VOLUME_DIRECT, true, 2, 2, &[]);
        let tags = [fr::FMOD_GET_STATE, fr::ERRCHECK, e2::VOICE_VOLUME_DIRECT];
        let this = t.zeros(0x40);
        for i in 0..500 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let d = rng.double();
            let k = rng.pick(&[100.0, 127.0, d]);
            let at = t.rebase(e2::VOLUME_DIVISOR);
            t.write(at, &k.to_le_bytes());
            let d = rng.double();
            let snap = rng.pick(&[0.001, 0.01, 0.5, d]);
            let at = t.rebase(e2::VOICE_SNAP);
            t.write(at, &snap.to_le_bytes());
            for k in 0..0x10 {
                let v = rng.next();
                t.w32(this + 4 * k, v);
            }
            let on = rng.pick(&[0, 1, 1, 0x80]);
            t.w8(this + 0x2c, on);
            let n = rng.pick(&[0, 1, 5, u32::MAX, 0x8000_0000]);
            t.w32(this + 0x30, n);
            let (r1, r2) = (rng.next() | 1, rng.next() | 1);
            let ch = rng.pick(&[0, r1, r2]);
            t.w32(this + 0x14, ch);
            let target = volume(&mut rng);
            t.w32(this + 0x3c, target);
            let other = volume(&mut rng);
            let cur = rng.pick(&[other, target]);
            t.w32(this + 0x38, cur);
            let (a, b) = (volume(&mut rng), rng.float());
            let step = rng.pick(&[0, a, b]);
            t.w32(this + 0x34, step);
            let st = rng.pick(&[8, 8, 0, 0xf7]);
            t.w32(state, st);
            let frame = t.ebp_for(0) - e2::VOICE_LOOP_FRAME;
            t.check_framed_conv(
                &format!("{w} voice loop"),
                0x0070_acf0,
                Conv::Thiscall(this),
                &[],
                frame,
                &[(4, 4)],
                |g| u(e2::voice_loop_volume(g, frame, this)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The sound events' pan and stop.
#[test]
fn e2_sfx_events() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e204);
        let (group, count, channel, vol) = (t.zeros(4), t.zeros(4), t.zeros(4), t.zeros(4));
        t.stub_at(d1::EVENT_GET_CHANNEL_GROUP, false, 2, 2, &[wr(1, group)]);
        t.stub_at(d1::GROUP_NUM_CHANNELS, false, 2, 2, &[wr(1, count)]);
        t.stub_at(d1::GROUP_GET_CHANNEL, false, 3, 3, &[wr(2, channel)]);
        t.stub_at(e2::CHANNEL_SET_PAN, false, 2, 2, &[]);
        t.stub_at(fr::ERRCHECK, false, 3, 0, &[]);
        t.stub_at(e2::EVENT_GET_VOLUME, false, 2, 2, &[wr(1, vol)]);
        t.stub_at(e2::EVENT_STOP, false, 2, 2, &[]);
        t.stub_at(fr::OPERATOR_DELETE, false, 1, 0, &[]);
        let tags = [
            d1::EVENT_GET_CHANNEL_GROUP,
            d1::GROUP_NUM_CHANNELS,
            d1::GROUP_GET_CHANNEL,
            e2::CHANNEL_SET_PAN,
            fr::ERRCHECK,
            e2::EVENT_GET_VOLUME,
            e2::EVENT_STOP,
            fr::OPERATOR_DELETE,
        ];
        let this = t.zeros(0x20);
        let events = t.zeros(8 * 8);
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            for k in 0..8 {
                let v = rng.next();
                t.w32(this + 4 * k, v);
            }
            t.w32(this + 8, events);
            let n = rng.pick(&[0, 1, 3, 8, u32::MAX]);
            t.w32(this, n);
            for k in 0..8 {
                let v = rng.next();
                t.w32(events + 8 * k, v);
                let v = rng.next();
                t.w32(events + 8 * k + 4, v);
                let live = rng.pick(&[0, 1, 1, 0x80]);
                t.w8(events + 8 * k + 4, live);
            }
            let fail = rng.pick(&[0, 0, 1, 2]);
            t.ret(d1::EVENT_GET_CHANNEL_GROUP, fail);
            let g = rng.next();
            t.w32(group, g);
            let n = rng.pick(&[0, 1, 3, u32::MAX]);
            t.w32(count, n);
            let c = rng.next();
            t.w32(channel, c);
            let pan = rng.float();
            let frame = t.ebp_for(1) - e2::APPLY_PAN_FRAME;
            t.check_framed_conv(
                &format!("{w} pan"),
                0x0070_eeb0,
                Conv::Thiscall(this),
                &[pan],
                frame,
                &[(4, 0xc)],
                |g| {
                    e2::sfx_event_apply_pan(g, frame, this, pan);
                    Out::None
                },
            );
            let f = rng.float();
            let v = rng.pick(&[
                0,
                0x8000_0000,
                0.5f32.to_bits(),
                1f32.to_bits(),
                3.5f32.to_bits(),
                (-1.5f32).to_bits(),
                0x7fc0_0000,
                f,
            ]);
            t.w32(vol, v);
            for off in [0x18, 0x1c] {
                let r = rng.next();
                let p = rng.pick(&[0, r]);
                t.w32(this + off, p);
            }
            let immediate = rng.pick(&[0, 0, 1, 0x100]);
            let frame = t.ebp_for(1) - e2::STOP_FRAME;
            t.check_framed_conv(
                &format!("{w} stop"),
                0x0070_f490,
                Conv::Thiscall(this),
                &[immediate],
                frame,
                &[(8, 4)],
                |g| {
                    e2::sfx_event_stop(g, frame, this, immediate);
                    Out::None
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// `SleepThreadInMillisecond`.
const SLEEP_MS: u32 = 0x006f_2d20;

/// The sync manager's data read.
#[test]
fn e2_sync_get_data() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e205);
        let this = t.zeros(0x24);
        // the sleep sets the loaded flag (another thread does in the game)
        let rest = t.stub_code(SLEEP_MS, false, 1, 0, false, &[]);
        let mut c = vec![0xc7, 0x05];
        c.extend((this + 0x20).to_le_bytes());
        c.extend(1u32.to_le_bytes()); // mov dword [this+0x20], 1
        c.push(0xe9);
        let at = t.cave_next + c.len() as u32 + 4;
        c.extend(rest.wrapping_sub(at).to_le_bytes()); // jmp rest
        let f = t.code(&c);
        t.jump(SLEEP_MS, f);
        let data = t.zeros(0x30);
        let outs = t.zeros(0x10);
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &[SLEEP_MS]);
            for k in 0..9 {
                let v = rng.next();
                t.w32(this + 4 * k, v);
            }
            let cursor = rng.pick(&[0, data, data + 4, data]);
            t.w32(this + 8, cursor);
            let has = rng.pick(&[0, 1, 1]);
            t.w32(this + 0x1c, has);
            let loaded = rng.pick(&[0, 1, 1]);
            t.w32(this + 0x20, loaded);
            for k in 0..0xc {
                let f = rng.float();
                let v = rng.pick(&[1f32.to_bits(), 1f32.to_bits(), 0x3e7, f, 0x3f80_0001]);
                t.w32(data + 4 * k, v);
            }
            let (x, y, id) = (outs, outs + 4, outs + 8);
            t.check_conv(
                &format!("{w} sync"),
                0x006f_b160,
                Conv::Thiscall(this),
                &[x, y, id],
                |g| u(e2::sync_get_data(g, this, x, y, id)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

// ---- batch 3 (ffx_glue_other::h4_e3)

/// The FMOD stubs the music's stop, restart and fade call: state and
/// paused queries that write their cells, the setters, `ERRCHECK`, the
/// music's own methods (thiscall). Returns the tags and the state,
/// paused cells.
fn music_stubs(t: &mut Twins) -> (Vec<u32>, u32, u32) {
    let state = t.zeros(4);
    let paused = t.zeros(4);
    t.stub_at(fr::FMOD_GET_STATE, false, 2, 2, &[wr(1, state)]);
    let byte = Write {
        arg: 1,
        cell: paused,
        byte: true,
    };
    t.stub_at(e2::EVENT_GET_PAUSED, false, 2, 2, &[byte]);
    let plain = [
        (EVENT_SET_PAUSED, 2),
        (EVENT_SET_CALLBACK, 3),
        (e2::EVENT_STOP, 2),
        (e3::EVENT_START, 1),
    ];
    for (sym, n) in plain {
        t.stub_at(sym, false, n, n, &[]);
    }
    t.stub_at(fr::ERRCHECK, false, 3, 0, &[]);
    let methods = [
        (e2::MUSIC_VOLUME_DIRECT, 3),
        (e3::MUSIC_VOLUME_TRANSLATE, 3),
        (e3::MUSIC_VOLUME_RESET, 3),
        (e3::MUSIC_STOP, 1),
        (e3::MUSIC_CLEAR, 1),
    ];
    for (sym, n) in methods {
        t.stub_at(sym, true, n, n, &[]);
    }
    let mut tags = vec![fr::FMOD_GET_STATE, e2::EVENT_GET_PAUSED, fr::ERRCHECK];
    tags.extend(plain.map(|p| p.0));
    tags.extend(methods.map(|m| m.0));
    (tags, state, paused)
}

/// `FMOD::Event::setPaused`'s thunk.
const EVENT_SET_PAUSED: u32 = 0x0094_8cbe;
/// `FMOD::Event::setCallback`'s thunk.
const EVENT_SET_CALLBACK: u32 = 0x0094_8cac;

/// The music's stop and stop-all (each in its own oracles: the stop-all
/// test keeps `FmodMusic_stop` original).
#[test]
fn e3_music_stop() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e301);
        let (stubs, state, paused) = music_stubs_without_stop(&mut t);
        let this = t.zeros(0x40);
        let arr = t.zeros((e2::TRACK * (e2::TRACKS + 1)) as usize);
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &stubs);
            for k in 0..0x10 {
                let v = rng.next();
                t.w32(this + 4 * k, v);
            }
            t.w32(this + 0x10, arr);
            tracks(&mut t, &mut rng, arr, e2::TRACKS + 1);
            for k in 0..=e2::TRACKS {
                let e = arr + e2::TRACK * k;
                let fade = rng.pick(&[0, 1, 1, 2]);
                t.w32(e + 0x24, fade);
                let held = rng.pick(&[0, 1, 1, 2]);
                t.w32(e + 4, held);
            }
            let st = rng.pick(&[8, 8, 0, 0xf7, 0x108, 0x100]);
            t.w32(state, st);
            let p = rng.pick(&[0, 0, 1, 0x100]);
            t.w32(paused, p);
            let r = rng.below(0xb6);
            let idx = rng.pick(&[0, 0x82, 0xb5, 0xb6, r, r, 0x0100_0005]);
            if idx <= e2::TRACKS {
                let (r1, r2) = (rng.next() | 1, rng.next() | 1);
                let ev = rng.pick(&[0, r1, r2]);
                t.w32(arr + e2::TRACK * idx, ev);
            }
            let frame = t.ebp_for(1) - 4;
            t.check_framed_conv(
                &format!("{w} stop {idx:#x}"),
                0x0070_99a0,
                Conv::Thiscall(this),
                &[idx],
                frame,
                &[(0, 4), (0xc, 4)],
                |g| u(e3::music_stop(g, frame, this, idx)),
            );
            let frame = t.ebp_for(0) - e3::STOP_ALL_FRAME;
            t.check_framed_conv(
                &format!("{w} stop all"),
                0x0070_9b10,
                Conv::Thiscall(this),
                &[],
                frame,
                &[(0, 4), (7, 1)],
                |g| u(e3::music_stop_all(g, frame, this)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// [`music_stubs`] without stubbing `FmodMusic_stop` (tested here).
fn music_stubs_without_stop(t: &mut Twins) -> (Vec<u32>, u32, u32) {
    let state = t.zeros(4);
    let paused = t.zeros(4);
    t.stub_at(fr::FMOD_GET_STATE, false, 2, 2, &[wr(1, state)]);
    let byte = Write {
        arg: 1,
        cell: paused,
        byte: true,
    };
    t.stub_at(e2::EVENT_GET_PAUSED, false, 2, 2, &[byte]);
    let plain = [
        (EVENT_SET_PAUSED, 2),
        (EVENT_SET_CALLBACK, 3),
        (e2::EVENT_STOP, 2),
    ];
    for (sym, n) in plain {
        t.stub_at(sym, false, n, n, &[]);
    }
    t.stub_at(fr::ERRCHECK, false, 3, 0, &[]);
    let mut tags = vec![fr::FMOD_GET_STATE, e2::EVENT_GET_PAUSED, fr::ERRCHECK];
    tags.extend(plain.map(|p| p.0));
    (tags, state, paused)
}

/// The music's restart and per-frame fade.
#[test]
fn e3_music_restart_loop() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e302);
        let (tags, state, _) = music_stubs(&mut t);
        let this = t.zeros(0x40);
        let arr = t.zeros((e2::TRACK * (e2::TRACKS + 1)) as usize);
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let d = rng.double();
            let k = rng.pick(&[100.0, 127.0, -1.0, d]);
            let at = t.rebase(e2::VOLUME_DIVISOR);
            t.write(at, &k.to_le_bytes());
            let full = rng.float();
            t.g32(e3::FULL_VOLUME, full);
            for k in 0..0x10 {
                let v = rng.next();
                t.w32(this + 4 * k, v);
            }
            t.w32(this + 0x10, arr);
            let flag = rng.pick(&[0, 1, 1, 2]);
            t.w32(this + 0x1c, flag);
            let mute = rng.pick(&[0, 1, 2]);
            t.w32(this + 0x24, mute);
            tracks(&mut t, &mut rng, arr, e2::TRACKS + 1);
            for k in 0..=e2::TRACKS {
                let e = arr + e2::TRACK * k;
                for off in [0xc, 0x18, 0x24] {
                    let v = rng.pick(&[0, 0, 1, 2]);
                    t.w32(e + off, v);
                }
                let n = rng.pick(&[0, 1, 5, u32::MAX, 0x8000_0000]);
                t.w32(e + 0x28, n);
                let f = rng.pick(&[0, 0x10, 0x7f]);
                t.w32(e + 0x1c, f);
                // floats the originals copy through fld/fstp: no
                // signalling NaNs (the emulator keeps them)
                for off in [0x10, 0x2c, 0x30] {
                    let s = rng.float();
                    t.w32(e + off, s);
                }
                // events mostly present: a fading track without one ends
                // the walk
                let ev = rng.next() | 1;
                let ev = rng.pick(&[ev, ev, ev, ev, ev, ev, 0]);
                t.w32(e, ev);
            }
            let st = rng.pick(&[1, 1, 8, 9, 0, 0x101]);
            t.w32(state, st);
            let r = rng.below(0xb6);
            let idx = rng.pick(&[0, 1, 0x82, 0xb5, 0xb6, r, r]);
            // the argument word itself
            let frame = t.ebp_for(1) + 8;
            t.check_framed_conv(
                &format!("{w} restart {idx:#x}"),
                0x0070_9270,
                Conv::Thiscall(this),
                &[idx],
                frame,
                &[(0, 4)],
                |g| u(e3::music_restart(g, frame, this, idx)),
            );
            let st = rng.pick(&[8, 8, 0, 0xf7]);
            t.w32(state, st);
            let frame = t.ebp_for(0) - e3::LOOP_FRAME;
            t.check_framed_conv(
                &format!("{w} loop"),
                0x0070_8e50,
                Conv::Thiscall(this),
                &[],
                frame,
                &[(0, 8)],
                |g| u(e3::music_loop_volume(g, frame, this)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// A sound event (`+0` count, `+8` events of 8 bytes) with its fades.
fn sfx_event(t: &mut Twins, rng: &mut Rng, this: u32, events: u32, fades: [u32; 2]) {
    for k in 0..8 {
        let v = rng.next();
        t.w32(this + 4 * k, v);
    }
    let n = rng.pick(&[0, 1, 3, 8, u32::MAX]);
    t.w32(this, n);
    t.w32(this + 8, events);
    for k in 0..16 {
        let v = rng.next();
        t.w32(events + 4 * k, v);
    }
    let f = rng.float();
    let delay = rng.pick(&[0, 0.5f32.to_bits(), 0.01f32.to_bits(), 0x7fc0_0000, f]);
    t.w32(this + 0xc, delay);
    for (k, f) in fades.into_iter().enumerate() {
        for w in 0..4 {
            let v = rng.float();
            t.w32(f + 4 * w, v);
        }
        let r = rng.float();
        let left = rng.pick(&[0, 0.01f32.to_bits(), 1f32.to_bits(), (-0.5f32).to_bits(), r]);
        t.w32(f + 8, left);
        let p = rng.pick(&[0, f, f]);
        t.w32(this + 0x18 + 4 * k as u32, p);
    }
}

/// The sound events' start.
#[test]
fn e3_sfx_start() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e303);
        let stubs = [
            (EVENT_SET_CALLBACK, false, 3),
            (e3::EVENT_START, false, 1),
            (e3::SFX_APPLY_VOLUME, true, 1),
            (e3::SFX_APPLY_PAN, true, 1),
        ];
        for (sym, this, n) in stubs {
            t.stub_at(sym, this, n, n, &[]);
        }
        t.stub_at(fr::ERRCHECK, false, 3, 0, &[]);
        t.stub_at(fr::OPERATOR_DELETE, false, 1, 0, &[]);
        let mut tags = stubs.map(|s| s.0).to_vec();
        tags.extend([fr::ERRCHECK, fr::OPERATOR_DELETE]);
        let this = t.zeros(0x20);
        let events = t.zeros(0x40);
        let fades = [t.zeros(0x10), t.zeros(0x10)];
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let d = rng.double();
            let k = rng.pick(&[127.0, 127.0, 128.0, 100.0, -127.0, d]);
            let at = t.rebase(e2::VOLUME_DIVISOR);
            t.write(at, &k.to_le_bytes());
            let d = rng.double();
            let one = rng.pick(&[1.0, 1.0, 0.5, d]);
            let at = t.rebase(e3::PAN_OFFSET);
            t.write(at, &one.to_le_bytes());
            let f = rng.float();
            let min = rng.pick(&[(-1f32).to_bits(), (-1f32).to_bits(), 0x8000_0000, f]);
            t.g32(e3::PAN_MIN, min);
            sfx_event(&mut t, &mut rng, this, events, fades);
            let r = rng.next();
            let volume = rng.pick(&[0, 1, 63, 127, 128, 200, u32::MAX, r]);
            let r = rng.next();
            let pan = rng.pick(&[0, 1, 63, 64, 127, 128, 200, u32::MAX, r]);
            t.check_conv(
                &format!("{w} start {volume} {pan}"),
                e3::SFX_START,
                Conv::Thiscall(this),
                &[volume, pan],
                |g| {
                    e3::sfx_event_start(g, this, volume, pan);
                    Out::None
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The sound events' delayed start and fades.
#[test]
fn e3_sfx_fades() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e304);
        let stubs = [
            (e3::SFX_START, true, 2),
            (e3::SFX_APPLY_VOLUME, true, 1),
            (e3::SFX_APPLY_PAN, true, 1),
        ];
        for (sym, this, n) in stubs {
            t.stub_at(sym, this, n, n, &[]);
        }
        t.stub_at(fr::OPERATOR_DELETE, false, 1, 0, &[]);
        let mut tags = stubs.map(|s| s.0).to_vec();
        tags.push(fr::OPERATOR_DELETE);
        let this = t.zeros(0x20);
        let events = t.zeros(0x40);
        let fades = [t.zeros(0x10), t.zeros(0x10)];
        for i in 0..500 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            sfx_event(&mut t, &mut rng, this, events, fades);
            let f = rng.float();
            let dt = rng.pick(&[0.5f32.to_bits(), 0.016f32.to_bits(), 0, f]);
            t.check_conv(
                &format!("{w} fades"),
                0x0070_f5c0,
                Conv::Thiscall(this),
                &[dt],
                |g| {
                    e3::sfx_event_update_fades(g, this, dt);
                    Out::None
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

// ---- batch 4 (ffx_glue_other::h4_e4)

/// The UI records: the active character's cursor, the sequence
/// rectangle, the mode-2 gradient box. The original's frame is zeroed
/// first in both oracles (the words it leaves unset are 0 in the port).
#[test]
fn e4_ui_records() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e401);
        let corners = [t.zeros(4), t.zeros(4), t.zeros(4), t.zeros(4)];
        let writes: Vec<Write> = corners
            .iter()
            .enumerate()
            .map(|(k, &c)| wr(k as u8, c))
            .collect();
        let scissor = t.stub_code(e4::BEGIN_SCISSOR, false, 4, 0, false, &writes);
        let plain = t.stub_code(0x0bad_58e0, false, 4, 0, false, &[]);
        t.stub_at(e4::DRAW_UI_ELEMENT, false, 5, 0, &[]);
        t.stub_at(e4::SCENE_STATE, false, 2, 0, &[]);
        let tags = [
            e4::BEGIN_SCISSOR,
            0x0bad_58e0,
            e4::DRAW_UI_ELEMENT,
            e4::SCENE_STATE,
        ];
        let p = t.zeros(0x14);
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            // active character cursor
            let xy: [u32; 6] = std::array::from_fn(|_| rng.small());
            let (c1, c0) = (rng.next(), rng.next());
            let ebp = t.ebp_for(8);
            let rec = ebp - 0x9c;
            t.write(rec, &[0; e4::RECORD as usize]);
            let mut args = xy.to_vec();
            args.extend([c1, c0]);
            t.check_framed(
                &format!("{w} active cursor"),
                0x0090_1190,
                &args,
                rec,
                &[],
                |g| u(e4::active_chr_cursor(g, rec, xy, c1, c0)),
            );
            let (mut x, mut y) = ([0; 0x98], [0; 0x98]);
            t.a.read_bytes(rec, &mut x);
            t.b.read_bytes(rec, &mut y);
            assert_eq!(x, y, "{w} active cursor record");
            // sequence rectangle
            for k in 0..5 {
                let v = rng.next();
                t.w32(p + 4 * k, v);
            }
            let st = rng.pick(&[3, 3, 0, 2]);
            t.ret(e4::SCENE_STATE, st);
            let wh = rng.pick(&[0, 0, 1, 2]);
            if wh != 0 {
                t.w16(p + 0xe, 7);
                t.w16(p + 0x12, if wh == 1 { 0x16 } else { 0x17 });
            }
            let ebp = t.ebp_for(1);
            let rec = ebp - 0x9c;
            t.write(rec, &[0; e4::RECORD as usize]);
            t.check_framed(&format!("{w} seq rect"), 0x008f_df20, &[p], rec, &[], |g| {
                u(e4::seq_draw_rect(g, rec, p))
            });
            let (mut x, mut y) = ([0; 0x98], [0; 0x98]);
            t.a.read_bytes(rec, &mut x);
            t.b.read_bytes(rec, &mut y);
            assert_eq!(x, y, "{w} seq rect record");
            // mode-2 box: the scissor clips (writes the corners) or not
            let clip = rng.pick(&[true, true, false]);
            t.jump(e4::BEGIN_SCISSOR, if clip { scissor } else { plain });
            for c in corners {
                let v = rng.float();
                t.w32(c, v);
            }
            let keep = rng.pick(&[1, 1, 0]);
            t.ret(e4::BEGIN_SCISSOR, keep);
            t.ret(0x0bad_58e0, keep);
            let a: [u32; 6] = [
                rng.float(),
                rng.float(),
                rng.float(),
                rng.float(),
                rng.next(),
                rng.next(),
            ];
            let ebp = t.ebp_for(6);
            let frame = ebp - 0xac;
            t.write(frame, &[0; e4::BOX_FRAME as usize]);
            t.check_framed(&format!("{w} box"), 0x008f_4df0, &a, frame, &[], |g| {
                u(e4::cross_box_mode2(g, frame, a))
            });
            if keep != 0 {
                let (mut x, mut y) = ([0; 0xa8], [0; 0xa8]);
                t.a.read_bytes(frame, &mut x);
                t.b.read_bytes(frame, &mut y);
                assert_eq!(x, y, "{w} box frame");
            }
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The distortion grid's weights and the button icons' UV remap.
#[test]
fn e4_grid_icon_uv() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e402);
        let this = t.zeros(0x70);
        let grid = t.zeros(0x600);
        let rec = t.zeros(0x98);
        for i in 0..400 {
            let w = format!("{name} {i}");
            for k in 0..0x1c {
                let v = rng.next();
                t.w32(this + 4 * k, v);
            }
            for k in 0..0x180 {
                let v = rng.next();
                t.w32(grid + 4 * k, v);
            }
            t.w32(this + 0x6c, grid + 4 * rng.below(4));
            t.check_conv(
                &format!("{w} grid"),
                0x006c_28e0,
                Conv::Thiscall(this),
                &[],
                |g| u(e4::distortion_fill_ones(g, this)),
            );
            let du = {
                let r0_ = rng.double();
                rng.pick(&[1.0 / 12.0, 1.0 / 12.0, r0_])
            };
            let at = t.rebase(e4::ICON_U_SIZE);
            t.write(at, &du.to_le_bytes());
            let dv = {
                let r0_ = rng.double();
                rng.pick(&[0.125, 0.125, r0_])
            };
            let at = t.rebase(e4::ICON_V_SIZE);
            t.write(at, &dv.to_le_bytes());
            for k in 0..0x26 {
                let v = rng.float();
                t.w32(rec + 4 * k, v);
            }
            // the left-to-right test: equal v, ordered u, or not
            let a = rng.float();
            let shape = rng.below(4);
            if shape < 3 {
                let b = {
                    let r0_ = rng.float();
                    rng.pick(&[(f32::from_bits(a) + 0.25).to_bits(), a, r0_])
                };
                t.w32(rec + 8, a);
                t.w32(rec + 0x28, b);
                let c = rng.float();
                t.w32(rec + 0xc, c);
                let d = {
                    let r0_ = rng.float();
                    rng.pick(&[c, c, r0_])
                };
                t.w32(rec + 0x2c, d);
            }
            let kind = {
                let r0_ = rng.next();
                rng.pick(&[4, 4, 0x8000_0004, 6, 0xb, 0x8000_0006, 0, 5, r0_])
            };
            let (iu, iv) = (rng.float(), rng.float());
            t.check_conv(
                &format!("{w} uv {kind:#x}"),
                0x006e_6f20,
                Conv::Stdcall,
                &[kind, rec, iu, iv],
                |g| {
                    e4::remap_icon_uv(g, kind, rec, iu, iv);
                    Out::None
                },
            );
        }
    }
}

/// The button icons' size remap.
#[test]
fn e4_icon_size() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e403);
        t.stub_at(e4::INPUT_INSTANCE, false, 0, 0, &[]);
        t.stub_at(e4::INPUT_VIBRATION, true, 0, 0, &[]);
        let tags = [e4::INPUT_INSTANCE, e4::INPUT_VIBRATION];
        let this = t.zeros(0x170);
        let rec = t.zeros(0x98);
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let r = rng.double();
            let kw = rng.pick(&[9.0, 9.0, r]);
            let at = t.rebase(e4::ICON_W_SCALE);
            t.write(at, &kw.to_le_bytes());
            let r = rng.double();
            let kh = rng.pick(&[12.0, 12.0, r]);
            let at = t.rebase(e4::ICON_H_SCALE);
            t.write(at, &kh.to_le_bytes());
            for off in [0x160, 0x164] {
                let v = rng.float();
                t.w32(this + off, v);
            }
            for k in 0..0x26 {
                let v = rng.float();
                t.w32(rec + 4 * k, v);
            }
            let a = rng.float();
            if rng.below(4) < 3 {
                let r = rng.float();
                let b = rng.pick(&[(f32::from_bits(a) + 0.25).to_bits(), a, r]);
                t.w32(rec + 8, a);
                t.w32(rec + 0x28, b);
                let c = rng.float();
                t.w32(rec + 0xc, c);
                let r = rng.float();
                let d = rng.pick(&[c, c, r]);
                t.w32(rec + 0x2c, d);
            }
            let r = rng.next();
            let kind = rng.pick(&[4, 4, 0x8000_0004, 6, 0xb, 1, 0x8000_0001, 0, 5, r]);
            t.check_conv(
                &format!("{w} size {kind:#x}"),
                0x006e_6db0,
                Conv::Thiscall(this),
                &[kind, rec],
                |g| {
                    e4::remap_icon_size(g, this, kind, rec);
                    Out::None
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The UI buttons' position remap.
#[test]
fn e4_button_position() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e404);
        let rec = t.zeros(0x98);
        for i in 0..600 {
            let w = format!("{name} {i}");
            for sym in e4::BUTTON_OFFSETS {
                let v = rng.double();
                let at = t.rebase(sym);
                t.write(at, &v.to_le_bytes());
            }
            for sym in e4::BUTTON_OFFSET_GLOBALS {
                for k in 0..2 {
                    let v = rng.float();
                    t.g32(sym + 4 * k, v);
                }
            }
            for k in 0..0x26 {
                let r = rng.float();
                let v = rng.pick(&[r, r, 0x8000_0000, 0]);
                t.w32(rec + 4 * k, v);
            }
            let r = rng.next();
            let kind = rng.pick(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0xa, 0xb, 0xc, 0xd, 0xe, r]);
            let (a, b) = (rng.next(), rng.next());
            t.check_conv(
                &format!("{w} button {kind:#x}"),
                0x006e_7470,
                Conv::Stdcall,
                &[a, rec, b, kind],
                |g| u(e4::remap_ui_button_position(g, rec, kind)),
            );
        }
    }
}

// ---- batch 5 (ffx_glue_other::h4_e5)

/// Random music tracks for batch 5: [`tracks`] plus the held, queued and
/// fade words, no signalling NaNs where the originals copy floats.
fn tracks5(t: &mut Twins, rng: &mut Rng, arr: u32) {
    tracks(t, rng, arr, e2::TRACKS + 1);
    for k in 0..=e2::TRACKS {
        let e = arr + e2::TRACK * k;
        for off in [4, 0x18, 0x24] {
            let v = rng.pick(&[0, 0, 1, 2]);
            t.w32(e + off, v);
        }
        let f = rng.pick(&[0, 0x10, 0x7f]);
        t.w32(e + 0x1c, f);
        for off in [0x10, 0x2c, 0x30] {
            let s = rng.float();
            t.w32(e + off, s);
        }
    }
}

/// The music's start, read and data send.
#[test]
fn e5_music_start_read() {
    e5_music(false);
}

/// The music's cross-play and set switch (`FmodMusic_read` and
/// `FmodMusic_start` stubbed: the originals hand FMOD their own argument
/// words, whose addresses differ under a port's frame).
#[test]
fn e5_music_cross_switch() {
    e5_music(true);
}

fn e5_music(cross: bool) {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(if cross { 0x5eed_e502 } else { 0x5eed_e501 });
        let state = t.zeros(4);
        t.stub_at(fr::FMOD_GET_STATE, false, 2, 2, &[wr(1, state)]);
        let plain = [
            (EVENT_SET_PAUSED, 2),
            (EVENT_SET_CALLBACK, 3),
            (e3::EVENT_START, 1),
        ];
        for (sym, n) in plain {
            t.stub_at(sym, false, n, n, &[]);
        }
        t.stub_at(fr::ERRCHECK, false, 3, 0, &[]);
        let methods = [
            (e2::MUSIC_VOLUME_DIRECT, 3),
            (e3::MUSIC_VOLUME_TRANSLATE, 3),
            (e3::MUSIC_STOP, 1),
            (e3::MUSIC_CLEAR, 1),
            (e5::RELEASE_OLD, 1),
            (e2::MUSIC_FADE_OUT, 2),
            (e5::MUSIC_STOP_ALL, 0),
            (e5::MUSIC_FORCE_CLEAR_ALL, 0),
        ];
        for (sym, n) in methods {
            t.stub_at(sym, true, n, n, &[]);
        }
        t.stub_at(e5::MUSIC_GROUP, false, 1, 0, &[]);
        if cross {
            t.stub_at(e5::MUSIC_READ, true, 2, 2, &[]);
            t.stub_at(e5::MUSIC_START, true, 1, 1, &[]);
        }
        t.stub_at(e5::OPERATOR_DELETE_ARRAY, false, 1, 0, &[]);
        // operator new: a fresh block per call
        let heap = t.zeros(0x1000);
        let next = t.zeros(4);
        let rest = t.stub_code(e5::OPERATOR_NEW, false, 1, 0, false, &[]);
        let cell = t.cells[&e5::OPERATOR_NEW];
        let mut c = vec![0xa1];
        c.extend(next.to_le_bytes()); // mov eax, [next]
        c.extend([0xa3]);
        c.extend(cell.to_le_bytes()); // mov [cell], eax
        c.extend([0x81, 0x05]);
        c.extend(next.to_le_bytes());
        c.extend(0x400u32.to_le_bytes()); // add dword [next], 0x400
        c.push(0xe9);
        let at = t.cave_next + c.len() as u32 + 4;
        c.extend(rest.wrapping_sub(at).to_le_bytes()); // jmp rest
        let f = t.code(&c);
        t.jump(e5::OPERATOR_NEW, f);
        // the event project's getEvent, __iob_func, fprintf
        let get_event = t.stub_code(0x0bad_0024, false, 4, 4, false, &[]);
        let vtable = t.zeros(0x40);
        t.w32(vtable + 0x24, get_event);
        let iob = t.stub_code(0x0bad_c350, false, 0, 0, false, &[]);
        t.g32(e5::IOB_FUNC.sym, iob);
        let fprintf = t.stub_code(0x0bad_c38c, false, 4, 0, false, &[]);
        t.g32(e5::FPRINTF.sym, fprintf);
        let mut tags = vec![fr::FMOD_GET_STATE, fr::ERRCHECK, e5::MUSIC_GROUP];
        tags.extend(plain.map(|p| p.0));
        tags.extend(methods.map(|m| m.0));
        tags.extend([
            e5::OPERATOR_DELETE_ARRAY,
            0x0bad_0024,
            0x0bad_c350,
            0x0bad_c38c,
        ]);
        let this = t.zeros(0x40);
        let arr = t.zeros((e2::TRACK * (e2::TRACKS + 1)) as usize);
        let projects = [t.zeros(8), t.zeros(8)];
        for p in projects {
            t.w32(p, vtable);
        }
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.w32(next, heap);
            let full = rng.float();
            t.g32(e3::FULL_VOLUME, full);
            for k in 0..0x10 {
                let v = rng.next();
                t.w32(this + 4 * k, v);
            }
            t.w32(this + 0x10, arr);
            let p0 = rng.pick(&[projects[0], projects[0], 0]);
            t.w32(this, p0);
            let p1 = rng.pick(&[projects[1], projects[1], 0]);
            t.w32(this + 4, p1);
            for (off, vals) in [
                (0x18, [0, 1, 0x10, 0x82]),
                (0x24, [0, 1, 1, 2]),
                (0x28, [0x79, 0xd0, 0, 0x79]),
                (0x2c, [0, 1, 1, 2]),
            ] {
                let v = rng.pick(&vals);
                t.w32(this + off, v);
            }
            tracks5(&mut t, &mut rng, arr);
            let st = rng.pick(&[1, 1, 8, 9, 0, 0x108]);
            t.w32(state, st);
            let nl = rng.pick(&[0, 0, 1, 2]);
            t.g32(e5::NO_LOAD.sym, nl);
            let g = rng.pick(&[0, 5, 0xb5, 0xb6]);
            t.ret(e5::MUSIC_GROUP, g);
            let r = rng.pick(&[0, 0x59, 0x59, 1]);
            t.ret(0x0bad_0024, r);
            let r = rng.below(0xb6);
            let idx = rng.pick(&[0, 0xc, 0xc, 0x10, 0x82, 0xb5, 0xb6, r, r]);
            let flag = rng.pick(&[0, 1, 1]);
            let a = rng.next();
            let frames = rng.next();
            let r = rng.next();
            let bgm = rng.pick(&[0, 1, 2, r]);
            if cross {
                t.check_conv(
                    &format!("{w} cross {idx:#x}"),
                    0x0070_8900,
                    Conv::Thiscall(this),
                    &[idx, frames, flag],
                    |g| u(e5::music_cross_play(g, this, idx, frames, flag)),
                );
                t.check_conv(
                    &format!("{w} switch {bgm:#x}"),
                    0x0070_9c80,
                    Conv::Thiscall(this),
                    &[bgm, flag],
                    |g| {
                        e5::music_switch_bgm(g, this, bgm, flag);
                        Out::None
                    },
                );
                continue;
            }
            let frame = t.ebp_for(1) + 8;
            t.check_framed_conv(
                &format!("{w} start {idx:#x}"),
                0x0070_96f0,
                Conv::Thiscall(this),
                &[idx],
                frame,
                &[(0, 4)],
                |g| u(e5::music_start(g, frame, this, idx)),
            );
            t.check_conv(
                &format!("{w} read {idx:#x}"),
                0x0070_9080,
                Conv::Thiscall(this),
                &[idx, flag],
                |g| u(e5::music_read(g, this, idx, flag)),
            );
            t.check_conv(
                &format!("{w} data send"),
                0x0070_8a00,
                Conv::Thiscall(this),
                &[a, flag],
                |g| u(e5::music_data_send(g, this, flag)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

// ---- continuation round 4, batch 1 (ffx_glue_other::h4_f1)

/// `graphicGetTimeInMicroseconds`.
const TIME_US: u32 = 0x0064_1420;

/// Random words over `n` bytes at a fresh block; its address.
fn random_block(t: &mut Twins, rng: &mut Rng, n: u32) -> u32 {
    let at = t.zeros(n as usize);
    for k in 0..n / 4 {
        let v = rng.next();
        t.w32(at + 4 * k, v);
    }
    at
}

fn rand_u64(rng: &mut Rng) -> u64 {
    (u64::from(rng.next()) << 32) | u64::from(rng.next())
}

/// The voice's start and restart.
#[test]
fn f1_voice() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f101);
        let state = t.zeros(4);
        t.stub_at(fr::FMOD_GET_STATE, false, 2, 2, &[wr(1, state)]);
        t.stub_at(EVENT_SET_CALLBACK, false, 3, 3, &[]);
        t.stub_at(e3::EVENT_START, false, 1, 1, &[]);
        t.stub_at(fr::ERRCHECK, false, 3, 0, &[]);
        t.stub_at(e2::VOICE_VOLUME_DIRECT, true, 2, 2, &[]);
        t.stub_u64(TIME_US, false, 0, 0);
        let release = t.stub_code(0x0bad_f108, true, 3, 3, false, &[]);
        let vtable = t.zeros(0x10);
        t.w32(vtable + 8, release);
        let group = t.zeros(4);
        t.w32(group, vtable);
        let tags = [
            fr::FMOD_GET_STATE,
            EVENT_SET_CALLBACK,
            e3::EVENT_START,
            fr::ERRCHECK,
            e2::VOICE_VOLUME_DIRECT,
            0x0bad_f108,
        ];
        let this = t.zeros(0x90);
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(&mut rng, &[e3::FULL_VOLUME], &[]);
            let now = rand_u64(&mut rng);
            t.ret_u64(TIME_US, now);
            for k in 0..0x24 {
                let v = rng.next();
                t.w32(this + 4 * k, v);
            }
            let ev = rng.next() | 1;
            t.w32(this + 0x14, rng.pick(&[0, ev, ev, ev]));
            t.w32(this + 0x58, rng.pick(&[0, 1, 1, 2]));
            t.w32(
                this + 0x64,
                rng.pick(&[0, 4, 5, 6, 0x8000_0000, 0x7fff_ffff]),
            );
            t.w32(this + 0x10, rng.pick(&[0, group, group]));
            t.w32(this + 0x80, rng.pick(&[0, 1, 2]));
            let flag = rng.pick(&[0, 1, 2]);
            t.g32(f1::VOICE_NO_PLAY, flag);
            let st = rng.pick(&[1, 1, 0, 2, 8]);
            t.w32(state, st);
            let r = rng.pick(&[0, 0, 1, 0x24]);
            t.ret(fr::FMOD_GET_STATE, r);
            let (a, b) = (rng.next(), rng.next());
            let frame = t.ebp_for(2) - f1::VOICE_FRAME;
            t.check_framed_conv(
                &format!("{w} start"),
                0x0070_b5c0,
                Conv::Thiscall(this),
                &[a, b],
                frame,
                &[(0, 4)],
                |g| u(f1::voice_start(g, frame, this)),
            );
            let frame = t.ebp_for(0) - f1::VOICE_FRAME;
            t.check_framed_conv(
                &format!("{w} restart"),
                0x0070_b160,
                Conv::Thiscall(this),
                &[],
                frame,
                &[(0, 4)],
                |g| u(f1::voice_restart(g, frame, this)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The sound loop.
#[test]
fn f1_sound_loop() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f102);
        t.stub_u64(TIME_US, false, 0, 0);
        let stubs = [
            (f1::SYSTEM_UPDATE, false, 1, 1),
            (fr::ERRCHECK, false, 3, 0),
            (f1::VOICE_UPDATE_STATE, true, 1, 1),
            (f1::MUSIC_LOOP_VOLUME, true, 0, 0),
            (f1::VOICE_LOOP_VOLUME, true, 0, 0),
            (f1::VOICE_RESTART, true, 0, 0),
            (f1::MUSIC_RESTART, true, 1, 1),
            (f1::SFX_UPDATE, true, 1, 1),
        ];
        for (sym, this, n, pops) in stubs {
            t.stub_at(sym, this, n, pops, &[]);
        }
        let tags = stubs.map(|s| s.0);
        let this = t.zeros(0x18);
        let music = t.zeros(0x20);
        let last_at = t.rebase(f1::LAST_LOOP);
        for i in 0..800 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let d = rng.double();
            let us = rng.pick(&[1e6, 1e6, d]);
            let at = t.rebase(f1::US_PER_S);
            t.write(at, &us.to_le_bytes());
            let last = rand_u64(&mut rng);
            let r = u64::from(rng.next());
            let big = (u64::from(rng.next()) << 32) | 0x1234;
            let step = rng.pick(&[
                0,
                1,
                0x80e8,
                0x80e9,
                16_667,
                33_334,
                1_000_000,
                r,
                big,
                u64::MAX,
                u64::MAX - 0x80e8,
                1 << 63,
                (1 << 63) + 0x801,
                (1 << 53) + 1,
            ]);
            let now = last.wrapping_add(step);
            t.ret_u64(TIME_US, now);
            t.w32(last_at, last as u32);
            t.w32(last_at + 4, (last >> 32) as u32);
            let sys = rng.next();
            t.w32(this + 4, rng.pick(&[0, sys]));
            let (v, s) = (rng.next(), rng.next());
            t.w32(this + 0xc, music);
            t.w32(this + 0x10, v);
            t.w32(this + 0x14, s);
            let track = rng.next();
            t.w32(music + 0x18, track);
            t.check_conv(
                &format!("{w} step {step:#x}"),
                0x0070_70d0,
                Conv::Thiscall(this),
                &[],
                |g| u(f1::sound_loop(g, this)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// `Phyre_PRedBlackTree_successor`'s stub: logs, returns the node's `+0x1c`.
fn stub_successor_e(t: &mut Twins) {
    let sym = c2::TREE_SUCCESSOR;
    let rest = t.stub_code(sym, true, 1, 1, false, &[]);
    let cell = t.cells[&sym];
    let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x40, 0x1c, 0xa3];
    c.extend(cell.to_le_bytes()); // mov eax, [esp+4]; mov eax, [eax+0x1c]; mov [cell], eax
    c.push(0xe9);
    let at = t.cave_next;
    c.extend(rest.wrapping_sub(at + 17).to_le_bytes()); // jmp rest
    let f = t.code(&c);
    t.jump(sym, f);
}

/// A stub at `sym` (thiscall, `n` words) whose result is `[ecx + off]`.
fn stub_from_this(t: &mut Twins, sym: u32, n: u32, off: u8) {
    let rest = t.stub_code(sym, true, n, n, false, &[]);
    let cell = t.cells[&sym];
    let mut c = vec![0x8b, 0x41, off, 0xa3];
    c.extend(cell.to_le_bytes()); // mov eax, [ecx+off]; mov [cell], eax
    c.push(0xe9);
    let at = t.cave_next;
    c.extend(rest.wrapping_sub(at + 13).to_le_bytes()); // jmp rest
    let f = t.code(&c);
    t.jump(sym, f);
}

/// A `PRedBlackTree` at `tree` over `nodes` (in order: `+0x1c` the next
/// for the successor stub; the leftmost chain from `tree + 0xc`).
fn tree_of(t: &mut Twins, rng: &mut Rng, tree: u32, nodes: &[u32]) {
    for (k, &nd) in nodes.iter().enumerate() {
        t.w32(nd + 0x1c, nodes.get(k + 1).copied().unwrap_or(tree));
    }
    if let Some(&first) = nodes.first() {
        if rng.below(2) == 0 {
            t.w32(tree + 0xc, first);
        } else {
            let top = t.zeros(0x20);
            t.w32(tree + 0xc, top);
            t.w32(top, first);
        }
        t.w32(first, tree);
    } else {
        // a null link would hand the successor stub address 0
        t.w32(tree + 0xc, tree);
    }
}

/// The sound effects' whole volume and project unload.
#[test]
fn f1_sfx() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f103);
        stub_successor_e(&mut t);
        stub_from_this(&mut t, f1::SFX_ANY_EVENT_FLAG, 0, 0);
        stub_from_this(&mut t, f1::SFX_BANK_PLAYING, 0, 4);
        let stubs = [
            (f1::SFX_EVENT_ENTIRE_VOLUME, true, 1, 1),
            (f1::SFX_BANK_DTOR, true, 0, 0),
            (fr::OPERATOR_DELETE, false, 1, 0),
            (c2::TREE_ORPHAN_ALL, true, 0, 0),
            (c2::FREE_LIST_RESET, true, 0, 0),
            (VFX_INT_TREE_INSERT, true, 1, 1),
            (c2::TREE_POST_INSERT, true, 1, 1),
            (c2::FREE_LIST_DEALLOCATE, true, 1, 1),
        ];
        for (sym, this, n, pops) in stubs {
            t.stub_at(sym, this, n, pops, &[]);
        }
        // allocateNewBlock: the list head from a cell
        let refill = t.zeros(4);
        let rest = t.stub_code(c2::FREE_LIST_ALLOCATE_BLOCK, true, 1, 1, false, &[]);
        let mut c = vec![0xa1];
        c.extend(refill.to_le_bytes()); // mov eax, [refill]
        c.extend([0x89, 0x01, 0xe9]); // mov [ecx], eax; jmp rest
        let at = t.cave_next;
        c.extend(rest.wrapping_sub(at + 12).to_le_bytes());
        let f = t.code(&c);
        t.jump(c2::FREE_LIST_ALLOCATE_BLOCK, f);
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.extend([
            c2::TREE_SUCCESSOR,
            f1::SFX_ANY_EVENT_FLAG,
            f1::SFX_BANK_PLAYING,
            c2::FREE_LIST_ALLOCATE_BLOCK,
        ]);
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let this = random_block(&mut t, &mut rng, 0x100);
            // the sound tree at +0x64
            let n = rng.below(5) as usize;
            let nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x20)).collect();
            for &nd in &nodes {
                let events = t.zeros(0x10);
                for e in 0..4 {
                    let flagged = t.zeros(4);
                    let hi = rng.next() & !0xff;
                    t.w32(flagged, hi | rng.pick(&[0, 1, 0x80]));
                    let v = rng.pick(&[0, flagged, flagged]);
                    t.w32(events + 4 * e, v);
                }
                t.w32(nd + 0x10, rng.below(5));
                t.w32(nd + 0x18, events);
            }
            tree_of(&mut t, &mut rng, this + 0x64, &nodes);
            let flagged = t.zeros(4);
            t.w8(flagged, rng.pick(&[0, 1]));
            t.w32(this + 0xc8, rng.pick(&[0, flagged, flagged]));
            let cur = rng.float();
            let cur = rng.pick(&[cur, 0, 0x3f00_0000, 0x3f80_0000]);
            t.w32(this + 0xd0, cur);
            let v = rng.float();
            let volume = rng.pick(&[
                v,
                cur,
                0x3f00_0000,
                0xbf80_0000,
                0x4000_0000,
                0,
                0x8000_0000,
                0x7fc0_0000,
                0x3f80_0000,
            ]);
            t.check_conv(
                &format!("{w} entire volume {volume:#x} of {n}"),
                0x0070_d500,
                Conv::Thiscall(this),
                &[volume],
                |g| {
                    f1::sfx_set_entire_volume(g, this, volume);
                    Out::None
                },
            );
            // the bank trees at +4 and +0x34
            for tree in [this + 4, this + 0x34] {
                let n = rng.below(6) as usize;
                let nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x20)).collect();
                for &nd in &nodes {
                    let bank = t.zeros(8);
                    t.w32(bank + 4, rng.pick(&[0, 0, 1, 3]));
                    // the +0x34 tree's banks are asked for their playing
                    // events (a null bank faults the original there too)
                    let none = if tree == this + 4 { 0 } else { bank };
                    t.w32(nd + 0x10, rng.pick(&[none, bank, bank, bank]));
                    let key = rng.next();
                    t.w32(nd + 0xc, key);
                }
                tree_of(&mut t, &mut rng, tree, &nodes);
            }
            let free = t.zeros(0x20);
            let next = t.zeros(0x20);
            t.w32(free, rng.pick(&[0, next]));
            t.w32(this + 0xac, rng.pick(&[0, free, free]));
            let more = t.zeros(0x20);
            t.w32(refill, rng.pick(&[0, more]));
            let dup = rng.pick(&[0, 0, 1, 0x100]);
            t.ret(VFX_INT_TREE_INSERT, dup);
            let all = rng.pick(&[0, 1, 0x100, 0x101]);
            let b = rng.next();
            t.check_conv(
                &format!("{w} unload {all:#x}"),
                0x0070_dd10,
                Conv::Thiscall(this),
                &[all, b],
                |g| u(f1::sfx_unload_projects(g, this, all)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// `vfxIntTree_Insert_0070bb20`.
const VFX_INT_TREE_INSERT: u32 = 0x0070_bb20;

/// A sound event's volume fade.
#[test]
fn f1_set_volume2() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f104);
        let current = t.zeros(4);
        t.stub_at(f1::EVENT_GET_VOLUME, false, 2, 2, &[wr(1, current)]);
        t.stub_at(fr::ERRCHECK, false, 3, 0, &[]);
        t.stub_at(f1::SFX_EVENT_SET_VOLUME, true, 1, 1, &[]);
        t.stub_at(fr::OPERATOR_DELETE, false, 1, 0, &[]);
        t.stub_at(f1::OPERATOR_NEW, false, 1, 0, &[]);
        let from = t.stub_code(f1::SFX_VOLUME_FROM_127, true, 1, 1, true, &[]);
        t.jump(f1::SFX_VOLUME_FROM_127, from);
        let tags = [
            f1::EVENT_GET_VOLUME,
            fr::ERRCHECK,
            f1::SFX_EVENT_SET_VOLUME,
            fr::OPERATOR_DELETE,
            f1::OPERATOR_NEW,
            f1::SFX_VOLUME_FROM_127,
        ];
        let this = t.zeros(0x20);
        let first = t.zeros(8);
        for i in 0..800 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let fade = t.zeros(0x10);
            t.ret(f1::OPERATOR_NEW, rng.pick(&[0, fade, fade, fade]));
            let d = rng.double();
            t.ret_f64(
                f1::SFX_VOLUME_FROM_127,
                rng.pick(&[d, 0.5, 1.0, 0.0, 1.0 / 3.0]),
            );
            let d = rng.double();
            let fps = rng.pick(&[30.0, 30.0, d, 0.0]);
            let at = t.rebase(f1::FADE_FPS);
            t.write(at, &fps.to_le_bytes());
            let cur = rng.float();
            t.w32(current, rng.pick(&[cur, 0x3f80_0000, 0]));
            for k in 0..8 {
                let v = rng.next();
                t.w32(this + 4 * k, v);
            }
            let old = rng.next();
            t.w32(this + 0x18, rng.pick(&[0, old]));
            t.w32(this + 8, first);
            let ev = rng.next();
            t.w32(first, ev);
            let hi = rng.next() & !0xff;
            t.w32(first + 4, hi | rng.pick(&[0, 1, 0x80]));
            let r = rng.next();
            let volume = rng.pick(&[0, 64, 127, 200, r]);
            let r = rng.next();
            let frames = rng.pick(&[0, 0, 1, 30, 90, 0xffff_ffff, 0x8000_0000, r]);
            let frame = t.ebp_for(2) - 0x10;
            t.check_framed_conv(
                &format!("{w} {volume} over {frames}"),
                0x0070_f210,
                Conv::Thiscall(this),
                &[volume, frames],
                frame,
                &[(0, 4)],
                |g| match f1::sfx_event_set_volume2(g, frame, this, volume, frames) {
                    Some(v) => u(v),
                    None => Out::None,
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The menu manager's instance setup.
#[test]
fn f1_menu_instance_setup() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f105);
        t.stub_at(f1::MENU_GEOMETRY_CTOR, true, 0, 0, &[]);
        t.stub_at(f1::MENU_GEOMETRY_SET_MATRIX, true, 0, 0, &[]);
        // the material call also notes the counts the record's vertex
        // info lends meanwhile (geometry +0x24 -> record +0x94 -> +4, +8)
        let counts = t.zeros(8);
        let rest = t.stub_code(0x0bad_f104, true, 2, 2, false, &[]);
        let mut c = vec![
            0x8b, 0x41, 0x24, 0x8b, 0x80, 0x94, 0, 0, 0, 0x8b, 0x50, 0x04, 0x89, 0x15,
        ];
        c.extend(counts.to_le_bytes()); // mov eax, [ecx+0x24]; mov eax, [eax+0x94]; mov edx, [eax+4]; mov [counts], edx
        c.extend([0x8b, 0x50, 0x08, 0x89, 0x15]);
        c.extend((counts + 4).to_le_bytes()); // mov edx, [eax+8]; mov [counts+4], edx
        c.push(0xe9);
        let at = t.cave_next;
        c.extend(rest.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // jmp rest
        let set = t.code(&c);
        let tags = [
            f1::MENU_GEOMETRY_CTOR,
            f1::MENU_GEOMETRY_SET_MATRIX,
            0x0bad_f104,
        ];
        let vtable = t.zeros(8);
        t.w32(vtable + 4, set);
        let scene = t.zeros(0xfaa0);
        t.g32(f1::PHYRE_SCENE, scene);
        let names: Vec<u32> = [
            &b"RealFrameBuffer\0"[..],
            b"NoTexture\0",
            b"NoTexturf\0",
            b"RealFrameBuffe\0",
            b"\0",
            b"FrameBuffer\0",
        ]
        .iter()
        .map(|s| t.alloc(s))
        .collect();
        let this = t.zeros(0x2ec40);
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            for off in [0xfa80, 0xfa84, 0xfa88] {
                let v = rng.next();
                t.w32(scene + off, v);
            }
            t.w8(scene + 0xfa9c, rng.pick(&[0, 0, 1, 2]));
            t.w8(scene + 0xfa9d, rng.pick(&[0, 0, 1, 2]));
            let n = rng.pick(&[0, 1, 5, 0x10, 0x31]);
            t.w32(this + 0x2ec30, n);
            let geom = t.zeros(0x40);
            t.w32(geom, vtable);
            t.ret(f1::MENU_GEOMETRY_CTOR, geom);
            let rec = random_block(&mut t, &mut rng, 0xc0);
            let info = random_block(&mut t, &mut rng, 0x30);
            t.w32(rec + 0x94, info);
            let real = t.rebase(f1::REAL_FRAME_BUFFER);
            let none = t.rebase(f1::NO_TEXTURE);
            let k = rng.below(names.len() as u32) as usize;
            let nm = rng.pick(&[names[k], names[k], real, none]);
            let big = rng.pick(&[0, 1, 2, 0x100]);
            t.check_conv(
                &format!("{w} setup n {n} name {nm:#x} big {big}"),
                0x0067_d760,
                Conv::Thiscall(this),
                &[rec, nm, big],
                |g| Out::U64(f1::menu_instance_setup(g, this, rec, nm, big)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The menu manager's setup of every instance and its buffer chain swap.
#[test]
fn f1_menu_all_swap() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f106);
        t.stub_u64(f1::MENU_INSTANCE_SETUP, true, 3, 3);
        t.stub_at(f1::MENU_GATHER_BUFFERS, true, 1, 1, &[]);
        t.stub_at(f1::MENU_UPDATE_MATERIAL, true, 2, 2, &[]);
        let tags = [
            f1::MENU_INSTANCE_SETUP,
            f1::MENU_GATHER_BUFFERS,
            f1::MENU_UPDATE_MATERIAL,
        ];
        let this = t.zeros(0x2ec40);
        let lists = [
            (0x2bd4, 0x2580),
            (0x2bdc, 0x3200),
            (0x582c, 0x2580),
            (0x5834, 0x3200),
        ];
        let blocks: Vec<u32> = lists.iter().map(|&(_, n)| t.zeros(n)).collect();
        for i in 0..24 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let r = rand_u64(&mut rng);
            t.ret_u64(f1::MENU_INSTANCE_SETUP, r);
            for nm in f1::MENU_NAMES {
                let v = rng.next();
                t.g32(nm, v);
            }
            for (&(off, _), &b) in lists.iter().zip(&blocks) {
                t.w32(this + off, b);
            }
            for off in [0x15e4, 0x15ec] {
                let recs = t.zeros(4);
                let v = rng.next();
                t.w32(recs, v);
                t.w32(this + off, recs);
            }
            let n = rng.next();
            t.w32(this + 0x2ec30, n);
            let v = rng.next();
            t.w32(this + 0x5b9c, v);
            if i % 3 == 0 {
                t.check_conv(
                    &format!("{w} set up all"),
                    0x0067_e6c0,
                    Conv::Thiscall(this),
                    &[],
                    |g| Out::U64(f1::menu_set_up_all(g, this)),
                );
            }
            for _ in 0..20 {
                t.fresh(&mut rng, &tags);
                t.w32(this + 0x5b7c, rng.pick(&[0, 0, 1, 0x100]));
                let c = rng.pick(&[0, 1, 3, 8]);
                t.w32(this + 0x2c50, c);
                let c = rng.pick(&[0, 1, 3, 8]);
                t.w32(this + 0x5840, c);
                t.w32(this + 0x5844, rng.pick(&[0, 1]));
                for base in [0u32, 0x2c58] {
                    for k in 0..8 {
                        let rec = t.zeros(0xb4);
                        t.w32(rec + 0xb0, rng.pick(&[0, 0, 1, 0x100]));
                        t.w32(this + base + 4 * k, rec);
                    }
                }
                for &(off, _) in &lists {
                    let v = rng.next();
                    t.w32(this + off, v);
                }
                t.check_conv(
                    &format!("{w} swap"),
                    0x0067_e7f0,
                    Conv::Thiscall(this),
                    &[],
                    |g| {
                        f1::menu_swap_buffer_chain(g, this);
                        Out::None
                    },
                );
            }
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

// ---- continuation round 4, batch 2 (ffx_glue_other::h4_f2)

/// The render thread's shader-pass switch.
#[test]
fn f2_shader_pass() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f201);
        let stubs = [
            (f2::PROGRAM_SET, 1),
            (f2::PASS_APPLY, 1),
            (f2::RETURN_ZERO3, 3),
            (f2::UNBIND_ALL, 3),
            (f2::RETURN_ZERO2, 2),
            (f2::RETURN_ZERO1, 1),
            (f2::RELEASE_PASS, 1),
            (f2::BIND_TABLES, 3),
            (f2::BIND_TEXTURES, 3),
            (f2::FORWARD_IF_BOUND, 2),
            (f2::BIND_ALL, 2),
        ];
        for (sym, n) in stubs {
            t.stub_at(sym, true, n, n, &[]);
        }
        let tags = stubs.map(|s| s.0);
        let this = t.zeros(0x20);
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let (p1, p2) = (rng.next(), rng.next());
            let prev = random_block(&mut t, &mut rng, 0x30);
            let pass = random_block(&mut t, &mut rng, 0x30);
            t.w32(prev + 0x14, p1);
            t.w32(pass + 0x14, rng.pick(&[p1, p1, p2]));
            let tables = [rng.next(), rng.next()];
            for k in 0..4 {
                t.w32(prev + 0x20 + 4 * k, tables[0]);
                t.w32(pass + 0x20 + 4 * k, rng.pick(&tables));
            }
            for k in 0..8 {
                let v = rng.next();
                t.w32(this + 4 * k, v);
            }
            t.w32(this, rng.pick(&[0, prev, prev, prev]));
            t.check_conv(
                &format!("{w} switch"),
                0x0061_c400,
                Conv::Thiscall(this),
                &[pass],
                |g| u(f2::set_shader_pass(g, this, pass)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The menu manager's material update.
#[test]
fn f2_menu_material() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f202);
        t.stub_at(fr::PHYRE_PRINTF, false, 2, 0, &[]);
        t.stub_at(f2::GEOMETRY_HIDE, true, 0, 0, &[]);
        t.stub_at(fr::FIOS_UNIFY, false, 3, 0, &[]);
        t.stub_at(f2::GEOMETRY_SET_MATERIAL, true, 3, 3, &[]);
        let tags = [
            fr::PHYRE_PRINTF,
            f2::GEOMETRY_HIDE,
            fr::FIOS_UNIFY,
            f2::GEOMETRY_SET_MATERIAL,
        ];
        let scene = t.zeros(0xfaa4);
        t.g32(f1::PHYRE_SCENE, scene);
        let mut names: Vec<u32> = [
            &b"RealFrameBuffer\0"[..],
            b"NoTexture\0",
            b"FrameBuffer\0",
            b"menu/tex.dds\0",
            b"a\0",
        ]
        .iter()
        .map(|s| t.alloc(s))
        .collect();
        let real = t.rebase(f1::REAL_FRAME_BUFFER);
        let none = t.rebase(f1::NO_TEXTURE);
        let fb = t.rebase(f2::FRAME_BUFFER);
        names.extend([real, none, fb]);
        let mut ran = 0;
        for i in 0..800 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            for off in [0xfa80, 0xfa84, 0xfa88, 0xfa94] {
                let v = rng.next();
                t.w32(scene + off, v);
            }
            for off in [0xfa9c, 0xfa9d, 0xfaa0] {
                t.w8(scene + off, rng.pick(&[0, 0, 1, 2]));
            }
            let failed = rng.next();
            t.g32(f2::FAILED_NAME, failed);
            t.ret(f2::GEOMETRY_SET_MATERIAL, rng.pick(&[0, 0, 1, 0x100]));
            let geom = random_block(&mut t, &mut rng, 0x30);
            let slot = t.zeros(0xc);
            let g = rng.next();
            let g0 = rng.pick(&[0, geom, geom, geom]);
            t.w32(slot, g0);
            let rec_on = rng.below(6) != 0;
            // a geometry of 0 with a second word would be written at 0x24
            t.w32(
                slot + 4,
                if g0 == 0 && rec_on {
                    0
                } else {
                    rng.pick(&[0, g])
                },
            );
            let k = rng.below(names.len() as u32) as usize;
            t.w32(slot + 8, names[k]);
            let rec = random_block(&mut t, &mut rng, 0xc0);
            let info = random_block(&mut t, &mut rng, 0x30);
            t.w32(rec + 0x94, info);
            let k2 = rng.below(names.len() as u32) as usize;
            let n2 = rng.pick(&[names[k2], names[k2], names[k]]);
            t.w32(info + 0x20, n2);
            t.w32(rec + 0xbc, rng.pick(&[0, 0, 0, 1, 0x100]));
            let rec = if rec_on { rec } else { 0 };
            let frame = t.ebp_for(2) - 0x104;
            let this = rng.next();
            t.check_framed_conv(
                &format!("{w} material"),
                0x0067_d920,
                Conv::Thiscall(this),
                &[slot, rec],
                frame,
                &[(0, 0x100)],
                |g| u(f2::menu_update_material(g, frame, slot, rec)),
            );
            ran += t.log_entries();
        }
        assert!(ran > 0, "{name}: the stubs ran");
    }
}

/// The menu manager's buffer reset.
#[test]
fn f2_menu_reset() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f203);
        let this = t.zeros(0x5ba0);
        for i in 0..300 {
            let w = format!("{name} {i}");
            for k in 0..0x5ba0 / 0x40 {
                let v = rng.next();
                t.w32(this + 0x40 * k, v);
            }
            for &(recs, n) in &f2::RESET_LISTS {
                let count = rng.pick(&[0, 1, 3, 5]);
                t.w32(this + n, count);
                let arr = t.zeros(0x20);
                for k in 0..count {
                    let rec = random_block(&mut t, &mut rng, 0xc0);
                    let info = random_block(&mut t, &mut rng, 0x30);
                    t.w32(rec + 0x94, info);
                    t.w32(arr + 4 * k, rec);
                }
                t.w32(this + recs, arr);
            }
            for off in [0x2c50, 0x5840, 0x5b88, 0x5b78, 0x5b74, 0x5844] {
                let v = rng.next();
                t.w32(this + off, v);
            }
            let state = rng.pick(&[0, 1, 1, 0, 2, 0x100]);
            t.w32(this + 0x5b7c, state);
            t.check_conv(
                &format!("{w} reset {state}"),
                0x0067_dfc0,
                Conv::Thiscall(this),
                &[],
                |g| {
                    f2::menu_reset_buffer_chain(g, this);
                    Out::None
                },
            );
        }
    }
}

/// The button-icon vertex info lookup.
#[test]
fn f2_abmap_vertex_info() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f204);
        t.stub_at(f2::REMAP_INSTANCE, false, 0, 0, &[]);
        t.stub_at(f2::REMAP_CONTROL_MACHINE, true, 0, 0, &[]);
        let strstr = t.stub_code(0x0bad_f2a0, false, 2, 0, false, &[]);
        let tags = [f2::REMAP_INSTANCE, f2::REMAP_CONTROL_MACHINE, 0x0bad_f2a0];
        let this = t.zeros(0xe4);
        let mut ran = 0;
        for i in 0..800 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(f2::STRSTR_IAT, strstr);
            t.ret(f2::REMAP_CONTROL_MACHINE, rng.pick(&[0, 1, 1]));
            t.ret(0x0bad_f2a0, rng.pick(&[0, 1, 0x1234_5678]));
            for k in 0..0x39 {
                let v = rng.next();
                t.w32(this + 4 * k, v);
            }
            t.w32(this + 4, rng.pick(&[0, 1, 0x8000_0000, 5]));
            t.w32(this + 8, rng.pick(&[0, 1, 0x8000_0000, 5]));
            let nm = rng.next();
            let nm = rng.pick(&[0, nm, nm]);
            let any = rng_kind(&mut rng);
            let kind = rng.pick(&[any, 6, 11, 0x8000_0006]);
            t.check_conv(
                &format!("{w} kind {kind}"),
                0x0068_4cd0,
                Conv::Thiscall(this),
                &[nm, kind],
                |g| u(f2::abmap_vertex_info(g, this, nm, kind)),
            );
            ran += t.log_entries();
        }
        assert!(ran > 0, "{name}: the stubs ran");
    }
}

fn rng_kind(rng: &mut Rng) -> u32 {
    rng.below(17)
}

/// A sound event's constructor and release.
#[test]
fn f2_sfx_event() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f205);
        // Array8_reserve: capacity and entries from cells
        let (cap, buf) = (t.zeros(4), t.zeros(4));
        let rest = t.stub_code(f2::ARRAY8_RESERVE, true, 1, 1, false, &[]);
        let mut c = vec![0xa1];
        c.extend(cap.to_le_bytes()); // mov eax, [cap]
        c.extend([0x89, 0x41, 0x04, 0xa1]); // mov [ecx+4], eax
        c.extend(buf.to_le_bytes()); // mov eax, [buf]
        c.extend([0x89, 0x41, 0x08, 0xe9]); // mov [ecx+8], eax; jmp rest
        let at = t.cave_next + c.len() as u32 + 4;
        c.extend(rest.wrapping_sub(at).to_le_bytes());
        let f = t.code(&c);
        t.jump(f2::ARRAY8_RESERVE, f);
        // user_malloc: a fresh block per call
        let heap = random_block(&mut t, &mut rng, 0x10000);
        let next = t.zeros(4);
        let rest = t.stub_code(USER_MALLOC, false, 1, 0, false, &[]);
        let cell = t.cells[&USER_MALLOC];
        let mut c = vec![0xa1];
        c.extend(next.to_le_bytes()); // mov eax, [next]
        c.extend([0xa3]);
        c.extend(cell.to_le_bytes()); // mov [cell], eax
        c.extend([0x81, 0x05]);
        c.extend(next.to_le_bytes());
        c.extend(0x100u32.to_le_bytes()); // add dword [next], 0x100
        c.push(0xe9);
        let at = t.cave_next + c.len() as u32 + 4;
        c.extend(rest.wrapping_sub(at).to_le_bytes()); // jmp rest
        let f = t.code(&c);
        t.jump(USER_MALLOC, f);
        let stubs = [
            (MEMCPY, 3, 0),
            (c2::USER_FREE_THUNK, 1, 0),
            (EVENT_SET_CALLBACK, 3, 3),
            (fr::OPERATOR_DELETE, 1, 0),
        ];
        for (sym, n, pops) in stubs {
            t.stub_at(sym, false, n, pops, &[]);
        }
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.extend([f2::ARRAY8_RESERVE, USER_MALLOC]);
        let this = t.zeros(0x28);
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.w32(next, heap);
            for k in 0..10 {
                let v = rng.next();
                t.w32(this + 4 * k, v);
            }
            let src = t.zeros(0xc);
            let n = rng.pick(&[0, 1, 2, 5, 9, 17, 30]);
            t.w32(src, n);
            let events = random_block(&mut t, &mut rng, 4 * 32);
            t.w32(src + 8, events);
            let block = random_block(&mut t, &mut rng, 8 * 40);
            let half = n / 2;
            let (c, b) = rng.pick(&[(n, block), (0, 0), (half, block), (half + 1, block)]);
            t.w32(cap, c);
            t.w32(buf, b);
            let volume = rng.float();
            let ebp = t.ebp_for(2);
            t.write(ebp - 0xc, &[0; 4]);
            t.check_conv(
                &format!("{w} ctor of {n} into cap {c}"),
                0x0070_e710,
                Conv::Thiscall(this),
                &[src, volume],
                |g| u(f2::sfx_event_ctor(g, this, src, volume)),
            );
            let n = rng.pick(&[0, 1, 3, 0x8000_0000]);
            t.w32(this, n);
            let v = rng.next();
            t.w32(this + 0x14, v);
            let data = random_block(&mut t, &mut rng, 8 * 4);
            // no entries only without instances (the original reads them)
            let none = if n == 1 || n == 3 { data } else { 0 };
            t.w32(this + 8, rng.pick(&[none, data, data]));
            for off in [0x18, 0x1c] {
                let p = rng.next();
                t.w32(this + off, rng.pick(&[0, p]));
            }
            t.check_conv(
                &format!("{w} dtor of {n}"),
                0x0070_e850,
                Conv::Thiscall(this),
                &[],
                |g| {
                    f2::sfx_event_dtor(g, this);
                    Out::None
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// `user_malloc` (`0x00687020`).
const USER_MALLOC: u32 = 0x0068_7020;
/// `memcpy`'s thunk.
const MEMCPY: u32 = 0x0094_9256;

// ---- continuation round 4, batch 3 (ffx_glue_other::h4_f3)

/// A search tree at `tree` over `nodes` sorted by key (`+0xc`, signed):
/// children `+0`/`+4` (the tree itself for none), root `tree + 0xc`,
/// `+0x1c` the next in order for the successor stub.
fn bst(t: &mut Twins, tree: u32, nodes: &[u32]) {
    fn build(t: &mut Twins, tree: u32, nodes: &[u32]) -> u32 {
        if nodes.is_empty() {
            return tree;
        }
        let mid = nodes.len() / 2;
        let left = build(t, tree, &nodes[..mid]);
        let right = build(t, tree, &nodes[mid + 1..]);
        t.w32(nodes[mid], left);
        t.w32(nodes[mid] + 4, right);
        nodes[mid]
    }
    for (k, &nd) in nodes.iter().enumerate() {
        t.w32(nd + 0x1c, nodes.get(k + 1).copied().unwrap_or(tree));
    }
    let root = build(t, tree, nodes);
    t.w32(tree + 0xc, root);
}

/// `n` tree nodes (0x20 bytes) with distinct ascending keys from `rng`.
fn keyed_nodes(t: &mut Twins, rng: &mut Rng, n: usize) -> Vec<(u32, u32)> {
    let mut keys: Vec<i32> = (0..n).map(|_| rng.small() as i32).collect();
    keys.sort_unstable();
    keys.dedup();
    keys.iter()
        .map(|&k| {
            let nd = t.zeros(0x20);
            t.w32(nd + 0xc, k as u32);
            (nd, k as u32)
        })
        .collect()
}

/// A stub at `sym` (thiscall, `n` words) that returns `ecx`.
fn stub_ret_this(t: &mut Twins, sym: u32, n: u32) {
    let rest = t.stub_code(sym, true, n, n, false, &[]);
    let cell = t.cells[&sym];
    let mut c = vec![0x89, 0x0d];
    c.extend(cell.to_le_bytes()); // mov [cell], ecx
    c.push(0xe9);
    let at = t.cave_next + c.len() as u32 + 4;
    c.extend(rest.wrapping_sub(at).to_le_bytes()); // jmp rest
    let f = t.code(&c);
    t.jump(sym, f);
}

/// `user_malloc`'s stub: a fresh block from `heap` per call (the bump
/// pointer in a cell). Returns the cell.
fn stub_malloc(t: &mut Twins, heap: u32) -> u32 {
    let next = t.zeros(4);
    t.w32(next, heap);
    let rest = t.stub_code(USER_MALLOC, false, 1, 0, false, &[]);
    let cell = t.cells[&USER_MALLOC];
    let mut c = vec![0xa1];
    c.extend(next.to_le_bytes()); // mov eax, [next]
    c.extend([0xa3]);
    c.extend(cell.to_le_bytes()); // mov [cell], eax
    c.extend([0x81, 0x05]);
    c.extend(next.to_le_bytes());
    c.extend(0x100u32.to_le_bytes()); // add dword [next], 0x100
    c.push(0xe9);
    let at = t.cave_next + c.len() as u32 + 4;
    c.extend(rest.wrapping_sub(at).to_le_bytes()); // jmp rest
    let f = t.code(&c);
    t.jump(USER_MALLOC, f);
    next
}

/// The mutex imports' stubs (tags `0x0bad_f3a0`, `0x0bad_f3a1`).
fn mutex_stubs(t: &mut Twins) -> [u32; 2] {
    let wait = t.stub_code(0x0bad_f3a0, false, 2, 2, false, &[]);
    let release = t.stub_code(0x0bad_f3a1, false, 1, 1, false, &[]);
    [wait, release]
}

/// The sound effects' per-frame update.
#[test]
fn f3_update() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f301);
        stub_successor_e(&mut t);
        stub_from_this(&mut t, f1::SFX_ANY_EVENT_FLAG, 0, 0);
        stub_from_this(&mut t, f1::SFX_BANK_PLAYING, 0, 4);
        let heap = random_block(&mut t, &mut rng, 0x8000);
        let next = stub_malloc(&mut t, heap);
        let [wait, release] = mutex_stubs(&mut t);
        let stubs = [
            (f3::SFX_EVENT_UPDATE_FADES, true, 1, 1),
            (f3::SFX_EVENT_DTOR, true, 0, 0),
            (fr::OPERATOR_DELETE, false, 1, 0),
            (f3::TREE_REMOVE, true, 1, 1),
            (c2::USER_FREE_THUNK, false, 1, 0),
            (c2::FREE_LIST_DEALLOCATE, true, 1, 1),
            (MEMCPY, false, 3, 0),
            (f1::SFX_BANK_DTOR, true, 0, 0),
        ];
        for (sym, this, n, pops) in stubs {
            t.stub_at(sym, this, n, pops, &[]);
        }
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.extend([
            c2::TREE_SUCCESSOR,
            f1::SFX_ANY_EVENT_FLAG,
            f1::SFX_BANK_PLAYING,
            USER_MALLOC,
            0x0bad_f3a0,
            0x0bad_f3a1,
        ]);
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.w32(next, heap);
            t.g32(fr::WAIT_FOR_SINGLE_OBJECT, wait);
            t.g32(fr::RELEASE_MUTEX, release);
            let this = random_block(&mut t, &mut rng, 0x100);
            t.w8(this, rng.pick(&[0, 0, 1]));
            // the id tree: some sounds with every slot empty
            let n = rng.below(12) as usize;
            let nodes = keyed_nodes(&mut t, &mut rng, n);
            for &(nd, _) in &nodes {
                let count = rng.pick(&[0, 1, 2, 4]);
                let events = t.zeros(0x10);
                for e in 0..count {
                    let ev = t.zeros(4);
                    let flag = rng.next() & !0xff | rng.pick(&[0, 0, 1]);
                    t.w32(ev, flag);
                    let v = rng.pick(&[0, ev, ev]);
                    t.w32(events + 4 * e, v);
                }
                t.w32(nd + 0x10, count);
                t.w32(nd + 0x18, events);
            }
            let list: Vec<u32> = nodes.iter().map(|p| p.0).collect();
            bst(&mut t, this + 0x64, &list);
            // the bank tree
            let n = rng.below(5) as usize;
            let banks = keyed_nodes(&mut t, &mut rng, n);
            for &(nd, _) in &banks {
                let bank = t.zeros(8);
                t.w32(bank + 4, rng.pick(&[0, 1, 1, 3]));
                t.w32(nd + 0x10, bank);
            }
            let list: Vec<u32> = banks.iter().map(|p| p.0).collect();
            bst(&mut t, this + 0x94, &list);
            let stream = t.zeros(4);
            let flag = rng.next() & !0xff | rng.pick(&[0, 1]);
            t.w32(stream, flag);
            t.w32(this + 0xc8, rng.pick(&[0, stream, stream]));
            let dt = rng.float();
            t.check_conv(
                &format!("{w} update"),
                0x0070_dec0,
                Conv::Thiscall(this),
                &[dt],
                |g| {
                    f3::sfx_update(g, this, dt);
                    Out::None
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// `SfxBank_appendEventById`'s stub: `[list + 8]` = the word in `fill`,
/// `al` = the bank's first word.
fn stub_append(t: &mut Twins, fill: u32) {
    let sym = f3::BANK_APPEND_EVENT;
    let rest = t.stub_code(sym, true, 2, 2, false, &[]);
    let cell = t.cells[&sym];
    let mut c = vec![0x8b, 0x44, 0x24, 0x08, 0x8b, 0x15];
    c.extend(fill.to_le_bytes()); // mov eax, [esp+8]; mov edx, [fill]
    c.extend([0x89, 0x50, 0x08, 0x8b, 0x01, 0xa3]); // mov [eax+8], edx; mov eax, [ecx]
    c.extend(cell.to_le_bytes()); // mov [cell], eax
    c.push(0xe9);
    let at = t.cave_next + c.len() as u32 + 4;
    c.extend(rest.wrapping_sub(at).to_le_bytes()); // jmp rest
    let f = t.code(&c);
    t.jump(sym, f);
}

/// The sound effects' event creation.
#[test]
fn f3_create_event() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f302);
        stub_successor_e(&mut t);
        let fill = t.zeros(4);
        stub_append(&mut t, fill);
        stub_ret_this(&mut t, f3::SFX_EVENT_CTOR, 2);
        let heap = random_block(&mut t, &mut rng, 0x8000);
        let next = stub_malloc(&mut t, heap);
        let [wait, release] = mutex_stubs(&mut t);
        let stubs = [
            (f1::OPERATOR_NEW, false, 1, 0),
            (fr::PRINT_TO_OUTPUT, false, 3, 0),
            (MEMCPY, false, 3, 0),
            (c2::USER_FREE_THUNK, false, 1, 0),
            (f3::ID_TREE_INSERT, true, 2, 2),
        ];
        for (sym, this, n, pops) in stubs {
            t.stub_at(sym, this, n, pops, &[]);
        }
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.extend([
            c2::TREE_SUCCESSOR,
            f3::BANK_APPEND_EVENT,
            f3::SFX_EVENT_CTOR,
            USER_MALLOC,
            0x0bad_f3a0,
            0x0bad_f3a1,
        ]);
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let entries = rng.next();
            t.w32(fill, rng.pick(&[0, entries]));
            t.w32(next, heap);
            t.g32(fr::WAIT_FOR_SINGLE_OBJECT, wait);
            t.g32(fr::RELEASE_MUTEX, release);
            let this = random_block(&mut t, &mut rng, 0x100);
            let block = t.zeros(0x28);
            t.ret(f1::OPERATOR_NEW, rng.pick(&[0, block, block, block]));
            for tree in [this + 0x34, this + 4] {
                let n = rng.below(4) as usize;
                let nodes = keyed_nodes(&mut t, &mut rng, n);
                for &(nd, _) in &nodes {
                    let bank = t.zeros(4);
                    let has = rng.next() & !0xff | rng.pick(&[0, 0, 0, 1]);
                    t.w32(bank, has);
                    t.w32(nd + 0x10, bank);
                }
                let list: Vec<u32> = nodes.iter().map(|p| p.0).collect();
                bst(&mut t, tree, &list);
            }
            // the id tree's sounds and their event arrays
            let n = rng.below(8) as usize;
            let mut nodes = keyed_nodes(&mut t, &mut rng, n);
            if rng.below(3) == 0 {
                let nd = t.zeros(0x20);
                t.w32(nd + 0xc, 0x72);
                nodes.push((nd, 0x72));
                nodes.sort_by_key(|p| p.1 as i32);
                nodes.dedup_by_key(|p| p.1);
            }
            for &(nd, key) in &nodes {
                let count = if key == 0x72 {
                    rng.pick(&[30, 31, 32])
                } else {
                    rng.pick(&[0, 1, 3, 5, 6, 7, 31])
                };
                let cap = rng.pick(&[count, count + 2, 0]);
                let data = random_block(&mut t, &mut rng, 4 * 40);
                for k in 0..40 {
                    let v = rng.pick(&[0, 1, 1, 1]);
                    // sound 0x72: exactly its limit live, then one more
                    let gone = if key == 0x72 { k == 0 } else { v == 0 };
                    if gone {
                        t.w32(data + 4 * k, 0);
                    }
                }
                let data = if count == 0 && cap == 0 {
                    rng.pick(&[0, data])
                } else {
                    data
                };
                t.w32(nd + 0x10, count);
                t.w32(nd + 0x14, cap);
                t.w32(nd + 0x18, data);
            }
            let list: Vec<u32> = nodes.iter().map(|p| p.0).collect();
            bst(&mut t, this + 0x64, &list);
            let keys: Vec<u32> = nodes.iter().map(|p| p.1).collect();
            let fresh = rng.small();
            let id = if keys.is_empty() {
                fresh
            } else {
                let k = keys[rng.below(keys.len() as u32) as usize];
                rng.pick(&[k, k, fresh, 0x72])
            };
            let frame = t.ebp_for(1) - 0x2c;
            t.check_framed_conv(
                &format!("{w} create {id:#x}"),
                0x0070_c8f0,
                Conv::Thiscall(this),
                &[id],
                frame,
                &[(0, 0x18), (0x34, 4)],
                |g| u(f3::sfx_create_event(g, frame, this, id)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The battle stream.
#[test]
fn f3_battle_streaming() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f303);
        let fill = t.zeros(4);
        stub_append(&mut t, fill);
        stub_ret_this(&mut t, f3::SFX_EVENT_CTOR, 2);
        let stubs = [
            (f1::OPERATOR_NEW, false, 1, 0),
            (fr::PRINT_TO_OUTPUT, false, 3, 0),
            (f3::MEMSET, false, 3, 0),
            (f3::FULL_FILENAME, false, 2, 0),
            (f3::SFX_BANK_CTOR, true, 2, 2),
            (f3::SFX_EVENT_STOP, true, 1, 1),
            (f3::SFX_EVENT_DTOR, true, 0, 0),
            (fr::OPERATOR_DELETE, false, 1, 0),
            (f3::SFX_EVENT_START, true, 2, 2),
            (c2::USER_FREE_THUNK, false, 1, 0),
        ];
        for (sym, this, n, pops) in stubs {
            t.stub_at(sym, this, n, pops, &[]);
        }
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.extend([f3::BANK_APPEND_EVENT, f3::SFX_EVENT_CTOR]);
        let mut ran = 0;
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let entries = rng.next();
            t.w32(fill, rng.pick(&[0, entries]));
            let this = random_block(&mut t, &mut rng, 0x100);
            t.w8(this, rng.pick(&[0, 0, 0, 1]));
            t.w8(this + 2, rng.pick(&[0, 1, 0x80]));
            let bank = t.zeros(4);
            let has = rng.next() & !0xff | rng.pick(&[0, 1, 1]);
            t.w32(bank, has);
            let have = rng.pick(&[0, bank, bank]);
            t.w32(this + 0xc4, have);
            t.ret(f3::SFX_BANK_CTOR, bank);
            let block = t.zeros(0x28);
            // a failed new for the bank would hand bank 0 to the lookup
            let none = if have == 0 { block } else { 0 };
            t.ret(f1::OPERATOR_NEW, rng.pick(&[none, block, block, block]));
            let stream = rng.next();
            t.w32(this + 0xc8, rng.pick(&[0, stream]));
            let args = [rng.next(), rng.small(), rng.next(), rng.next()];
            let frame = t.ebp_for(4) - 0x11c;
            t.check_framed_conv(
                &format!("{w} stream"),
                0x0070_d190,
                Conv::Thiscall(this),
                &args,
                frame,
                &[(0, 0x10c)],
                |g| {
                    f3::sfx_play_battle_streaming(g, frame, this, args);
                    Out::None
                },
            );
            ran += t.log_entries();
        }
        assert!(ran > 0, "{name}: the stubs ran");
    }
}

/// The sound effects' project load.
#[test]
fn f3_load_project() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f304);
        let found = t.zeros(4);
        // SfxEventTree_Find: the result from a cell; notes the key it reads
        let rest = t.stub_code(f3::TREE_FIND, true, 1, 1, false, &[]);
        let cell = t.cells[&f3::TREE_FIND];
        let keyed = t.zeros(4);
        let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x00, 0xa3];
        c.extend(keyed.to_le_bytes()); // mov eax, [esp+4]; mov eax, [eax]; mov [keyed], eax
        c.extend([0xa1]);
        c.extend(found.to_le_bytes()); // mov eax, [found]
        c.extend([0x8b, 0x00, 0x83, 0x05]); // mov eax, [eax]
        c.extend(found.to_le_bytes());
        c.push(4); // add dword [found], 4: the next call's result
        c.extend([0xa3]);
        c.extend(cell.to_le_bytes()); // mov [cell], eax
        c.push(0xe9);
        let at = t.cave_next + c.len() as u32 + 4;
        c.extend(rest.wrapping_sub(at).to_le_bytes());
        let f = t.code(&c);
        t.jump(f3::TREE_FIND, f);
        let sprintf = t.stub_code(0x0bad_f3b0, false, 3, 0, false, &[]);
        let stubs = [
            (f3::TREE_ADD, true, 2, 2),
            (f3::TREE_REMOVE, true, 1, 1),
            (c2::FREE_LIST_DEALLOCATE, true, 1, 1),
            (f3::MEMSET, false, 3, 0),
            (f3::FULL_FILENAME, false, 2, 0),
            (f1::OPERATOR_NEW, false, 1, 0),
            (f3::SFX_BANK_CTOR, true, 2, 2),
        ];
        for (sym, this, n, pops) in stubs {
            t.stub_at(sym, this, n, pops, &[]);
        }
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.extend([f3::TREE_FIND, 0x0bad_f3b0]);
        let mut ran = 0;
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(f3::SPRINTF_IAT, sprintf);
            let this = random_block(&mut t, &mut rng, 0x100);
            t.w8(this + 2, rng.pick(&[0, 1, 0x80]));
            let results = t.zeros(0x10);
            for k in 0..3 {
                let node = rng.next();
                t.w32(results + 4 * k, rng.pick(&[0, 0, node, 0xffff_fff0]));
            }
            t.w32(found, results);
            let block = t.zeros(0xc);
            t.ret(f1::OPERATOR_NEW, rng.pick(&[0, block, block]));
            let a = rng.next();
            let a = rng.pick(&[0, a, a, a]);
            let id = rng.small();
            let resident = rng.pick(&[0, 1, 0x100]);
            let args = [a, id, resident];
            let frame = t.ebp_for(3) - 0x214;
            t.check_framed_conv(
                &format!("{w} load"),
                0x0070_cf10,
                Conv::Thiscall(this),
                &args,
                frame,
                &[(0, 0x204), (0x220, 4)],
                |g| {
                    f3::sfx_load_project(g, frame, this, args);
                    Out::None
                },
            );
            ran += t.log_entries();
        }
        assert!(ran > 0, "{name}: the stubs ran");
    }
}

/// The sound effects' constructor.
#[test]
fn f3_sfx_ctor() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f305);
        t.stub_at(f3::FREE_LIST_CTOR, true, 4, 4, &[]);
        let create = t.stub_code(0x0bad_f3c0, false, 3, 3, false, &[]);
        let tags = [f3::FREE_LIST_CTOR, 0x0bad_f3c0];
        for i in 0..50 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(f3::CREATE_MUTEX_IAT, create);
            let this = random_block(&mut t, &mut rng, 0xd8);
            t.check_conv(
                &format!("{w} ctor"),
                0x0070_bfc0,
                Conv::Thiscall(this),
                &[],
                |g| u(f3::sfx_ctor(g, this)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

// ---- continuation round 4, batch 4 (ffx_glue_other::h4_f4)

/// The FMOD event system's setup.
#[test]
fn f4_init_event_system() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f401);
        let (system, object) = (t.zeros(4), t.zeros(4));
        t.stub_at(f4::EVENT_SYSTEM_CREATE, false, 1, 1, &[wr(0, system)]);
        t.stub_at(f4::GET_SYSTEM_OBJECT, false, 2, 2, &[wr(1, object)]);
        let stubs = [
            (f4::MEMORY_INITIALIZE, 6, 6),
            (f4::SET_STREAM_BUFFER_SIZE, 3, 3),
            (f4::EVENT_SYSTEM_INIT, 5, 5),
            (f4::SET_FILE_SYSTEM, 8, 8),
            (fr::ERRCHECK, 3, 0),
            (fr::PRINT_TO_OUTPUT, 2, 0),
            (f4::CLEAR_GLOBAL, 0, 0),
        ];
        for (sym, n, pops) in stubs {
            t.stub_at(sym, false, n, pops, &[]);
        }
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.extend([f4::EVENT_SYSTEM_CREATE, f4::GET_SYSTEM_OBJECT]);
        for i in 0..100 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let this = random_block(&mut t, &mut rng, 0xc);
            let s = rng.next();
            t.w32(system, rng.pick(&[0, s, s]));
            let o = rng.next();
            t.w32(object, o);
            t.check_conv(
                &format!("{w} init"),
                0x0070_6820,
                Conv::Thiscall(this),
                &[],
                |g| u(f4::init_event_system(g, this)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// A sound bank's constructor.
#[test]
fn f4_sfx_bank_ctor() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f402);
        let project = t.zeros(4);
        t.stub_at(f4::EVENT_SYSTEM_LOAD, false, 4, 4, &[wr(3, project)]);
        let sprintf = t.stub_code(0x0bad_f4a0, false, 3, 0, false, &[]);
        let stubs = [
            (fr::PRINT_TO_OUTPUT, false, 3, 0),
            (f3::MEMSET, false, 3, 0),
            (MEMCPY, false, 3, 0),
            (f1::OPERATOR_NEW, false, 1, 0),
            (f4::EVENT_TREE_CTOR, true, 2, 2),
            (f4::EVENT_GROUP_CTOR, true, 2, 2),
        ];
        for (sym, this, n, pops) in stubs {
            t.stub_at(sym, this, n, pops, &[]);
        }
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.extend([f4::EVENT_SYSTEM_LOAD, 0x0bad_f4a0]);
        let mut ran = 0;
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(f3::SPRINTF_IAT, sprintf);
            let this = random_block(&mut t, &mut rng, 0xc);
            let p = rng.next();
            t.w32(project, p);
            t.ret(f4::EVENT_SYSTEM_LOAD, rng.pick(&[0, 0, 0, 0x24]));
            let block = t.zeros(0x34);
            t.ret(f1::OPERATOR_NEW, rng.pick(&[0, block, block]));
            let len = rng.pick(&[4, 5, 15, 0x40, 0xff, 0x103]) as usize;
            let mut path: Vec<u8> = (0..len).map(|_| b'a' + rng.below(26) as u8).collect();
            path.push(0);
            let path = t.alloc(&path);
            let s = rng.next();
            let system = rng.pick(&[0, s, s, s]);
            let frame = t.ebp_for(2) - 0x210;
            t.check_framed_conv(
                &format!("{w} bank of {len}"),
                0x0070_f760,
                Conv::Thiscall(this),
                &[system, path],
                frame,
                &[(0, 0x200)],
                |g| u(f4::sfx_bank_ctor(g, frame, this, system, path)),
            );
            ran += t.log_entries();
        }
        assert!(ran > 0, "{name}: the stubs ran");
    }
}

/// The event tree's load.
#[test]
fn f4_event_tree_load() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f403);
        // allocateNewBlock: the list head from a cell
        let refill = t.zeros(4);
        let rest = t.stub_code(c2::FREE_LIST_ALLOCATE_BLOCK, true, 1, 1, false, &[]);
        let mut c = vec![0xa1];
        c.extend(refill.to_le_bytes()); // mov eax, [refill]
        c.extend([0x89, 0x01, 0xe9]); // mov [ecx], eax; jmp rest
        let at = t.cave_next;
        c.extend(rest.wrapping_sub(at + 12).to_le_bytes());
        let f = t.code(&c);
        t.jump(c2::FREE_LIST_ALLOCATE_BLOCK, f);
        let stubs = [
            (FIOS_EXISTS, false, 1, 0),
            (FIOS_OPEN, false, 2, 0),
            (FIOS_SIZE, false, 1, 0),
            (OPERATOR_NEW_2, false, 1, 0),
            (FIOS_READ, false, 3, 0),
            (FIOS_CLOSE, false, 1, 0),
            (f4::INT_TREE_INSERT, true, 1, 1),
            (c2::TREE_POST_INSERT, true, 1, 1),
            (c2::FREE_LIST_DEALLOCATE, true, 1, 1),
            (e5::OPERATOR_DELETE_ARRAY, false, 1, 0),
        ];
        for (sym, this, n, pops) in stubs {
            t.stub_at(sym, this, n, pops, &[]);
        }
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.push(c2::FREE_LIST_ALLOCATE_BLOCK);
        let mut ran = 0;
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let this = random_block(&mut t, &mut rng, 0x28);
            t.ret(FIOS_EXISTS, rng.pick(&[0, 1, 1, 1]));
            let file = rng.next() | 1;
            t.ret(FIOS_OPEN, rng.pick(&[0, file, file, file]));
            let size = rng.pick(&[0, 7, 8, 16, 40, 41]);
            t.ret(FIOS_SIZE, size);
            let buf = random_block(&mut t, &mut rng, 0x40);
            t.ret(OPERATOR_NEW_2, buf);
            t.ret(f4::INT_TREE_INSERT, rng.pick(&[0, 0, 1, 0x1234_5678]));
            // the free list: a chain of nodes
            let mut head = 0;
            for _ in 0..rng.below(6) {
                let nd = random_block(&mut t, &mut rng, 0x14);
                t.w32(nd, head);
                head = nd;
            }
            t.w32(this + 0x1c, head);
            let more = t.zeros(0x14);
            t.w32(refill, rng.pick(&[0, more]));
            let path = rng.next();
            t.check_conv(
                &format!("{w} load {size}"),
                0x0071_0370,
                Conv::Thiscall(this),
                &[path],
                |g| u(f4::event_tree_load(g, this, path)),
            );
            ran += t.log_entries();
        }
        assert!(ran > 0, "{name}: the stubs ran");
    }
}

/// `fiosIsFileExist`.
const FIOS_EXISTS: u32 = 0x0067_96f0;
/// `fiosOpen`.
const FIOS_OPEN: u32 = 0x0067_9730;
/// `fiosFileSize`.
const FIOS_SIZE: u32 = 0x0067_9570;
/// `fiosRead`.
const FIOS_READ: u32 = 0x0067_9800;
/// `fiosClose`.
const FIOS_CLOSE: u32 = 0x0067_9510;
/// `operator_new_006304c0`.
const OPERATOR_NEW_2: u32 = 0x0063_04c0;

// ---- continuation round 4, batch 5 (ffx_glue_other::h4_f5)

/// The voice's event read.
#[test]
fn f5_voice_read() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f501);
        t.stub_at(c3::VOICE_STOP, true, 0, 0, &[]);
        t.stub_at(fr::ERRCHECK, false, 3, 0, &[]);
        // getInfo: the info's +8 from a cell
        let info8 = t.zeros(4);
        let rest = t.stub_code(d1::EVENT_GET_INFO, false, 4, 4, false, &[]);
        let mut c = vec![0x8b, 0x44, 0x24, 0x10, 0x8b, 0x15];
        c.extend(info8.to_le_bytes()); // mov eax, [esp+0x10]; mov edx, [info8]
        c.extend([0x89, 0x50, 0x08, 0xe9]); // mov [eax+8], edx; jmp rest
        let at = t.cave_next + c.len() as u32 + 4;
        c.extend(rest.wrapping_sub(at).to_le_bytes());
        let f = t.code(&c);
        t.jump(d1::EVENT_GET_INFO, f);
        // getEvent: logs, writes its event cell through the out pointer
        let event = t.zeros(4);
        let get = t.stub_code(0x0bad_f520, true, 4, 4, false, &[wr(3, event)]);
        let vtable = t.zeros(0x24);
        t.w32(vtable + 0x20, get);
        let iob = t.stub_code(0x0bad_f521, false, 0, 0, false, &[]);
        let fprintf = t.stub_code(0x0bad_f522, false, 3, 0, false, &[]);
        let tags = [
            c3::VOICE_STOP,
            fr::ERRCHECK,
            d1::EVENT_GET_INFO,
            0x0bad_f520,
            0x0bad_f521,
            0x0bad_f522,
        ];
        let ids = [
            0x0290_d1c1u32,
            0x0291_01c1,
            0x0834_1504,
            0x504,
            0x3300_0001,
            0x3300_0000,
            0x1234,
            0x0101_0101,
        ];
        let mut ran = 0;
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(e5::IOB_FUNC.sym, iob);
            t.g32(e5::FPRINTF.sym, fprintf);
            t.ret(0x0bad_f520, rng.pick(&[0, 0, 0x59, 1]));
            let ev = rng.next();
            t.w32(event, rng.pick(&[0, ev, ev]));
            let this = random_block(&mut t, &mut rng, 0x70);
            t.w32(this + 8, rng.pick(&[0, 1]));
            t.w32(this + 0x14, rng.pick(&[0, 0, ev]));
            t.w32(this + 0x60, rng.pick(&[0, 1, 2]));
            t.w32(this, rng.pick(&[0x10e, 3, 3]));
            let flag = rng.pick(&[0, 0, 1]);
            t.g32(f1::VOICE_NO_PLAY, flag);
            let info = random_block(&mut t, &mut rng, 0x38);
            let v = rng.next();
            t.w32(info8, v);
            t.w32(this + 0x68, info);
            // the groups
            let group = t.zeros(4);
            t.w32(group, vtable);
            t.w32(this + 0x1c, rng.pick(&[0, group, group]));
            let groups = t.zeros(4 * 100);
            for k in 0..100 {
                t.w32(groups + 4 * k, rng.pick(&[0, group, group]));
            }
            t.w32(this + 0xc, groups);
            // the table: "ML\0\0", count, (id, offset) entries, names
            let table = t.zeros(0x100);
            t.write(
                table,
                rng.pick(&[&b"ML\0\0"[..], b"ML\0\0", b"MX\0\0", b"ML\0a"]),
            );
            let n = rng.pick(&[0, 3, 8]);
            t.w32(table + 4, n);
            for k in 0..n {
                let id = ids[rng.below(ids.len() as u32) as usize];
                t.w32(table + 8 + 8 * k, id);
                let off = 0x80 + 8 * k;
                t.w32(table + 12 + 8 * k, off);
                let d0 = b'0' + rng.below(10) as u8;
                let d1_ = b'0' + rng.below(10) as u8;
                t.write(table + off, &[d0, d1_, b'x', 0]);
            }
            t.w32(this + 0x24, table);
            let id = ids[rng.below(ids.len() as u32) as usize];
            let flag = rng.pick(&[0, 1, 1, 1]);
            t.check_conv(
                &format!("{w} read {id:#x}"),
                0x0070_add0,
                Conv::Thiscall(this),
                &[id, flag],
                |g| u(f5::voice_read(g, this, id, flag)),
            );
            ran += t.log_entries();
        }
        assert!(ran > 0, "{name}: the stubs ran");
    }
}

/// The sync manager's setup.
#[test]
fn f5_sync_init() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f502);
        let sprintf = t.stub_code(0x0bad_f530, false, 3, 0, false, &[]);
        let stubs = [
            (f3::MEMSET, false, 3, 0),
            (fr::FIOS_UNIFY, false, 3, 0),
            (f5::READ_SYNC_FILE, true, 1, 1),
            (f5::DBG_PRINTF, false, 1, 0),
            (f5::USER_ALIGNED_FREE, false, 1, 0),
        ];
        for (sym, this, n, pops) in stubs {
            t.stub_at(sym, this, n, pops, &[]);
        }
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.push(0x0bad_f530);
        let mut ran = 0;
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(f3::SPRINTF_IAT, sprintf);
            let this = random_block(&mut t, &mut rng, 0x44);
            let list = t.zeros(0x100);
            t.w32(this + 0x40, list);
            t.w32(this + 0x18, rng.below(8));
            let data = random_block(&mut t, &mut rng, 0x40);
            t.write(
                data,
                rng.pick(&[&b"SL\0\0"[..], b"SL\0\0", b"SX\0\0", b"SL\x01\0"]),
            );
            t.w32(data + 4, rng.pick(&[0, 1, 5, 12]));
            t.ret(f5::READ_SYNC_FILE, rng.pick(&[0, data, data]));
            let frame = t.ebp_for(0) - 0x204;
            t.check_framed_conv(
                &format!("{w} init"),
                0x006f_b210,
                Conv::Thiscall(this),
                &[],
                frame,
                &[(0, 0x200)],
                |g| u(f5::sync_init(g, frame, this)),
            );
            ran += t.log_entries();
        }
        assert!(ran > 0, "{name}: the stubs ran");
    }
}

// ---- continuation round 4, batch 6 (ffx_glue_other::h4_f6)

/// The menu message file read.
#[test]
fn f6_read_menu_message() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f601);
        // the first sprintf passes 3 words (a fourth would log the
        // caller's stack): the stub logs 3 and, for the menu file format,
        // stores the fourth (the name) into the path
        let rest = t.stub_code(0x0bad_f610, false, 3, 0, false, &[]);
        let fmt = t.rebase(f6::MENU_FILE_FORMAT);
        let mut c = vec![0x81, 0x7c, 0x24, 0x08];
        c.extend(fmt.to_le_bytes()); // cmp dword [esp+8], fmt
        c.extend([0x75, 0x0a]); // jne +10
        c.extend([0x8b, 0x44, 0x24, 0x10]); // mov eax, [esp+0x10]
        c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
        c.extend([0x89, 0x02]); // mov [edx], eax
        c.push(0xe9);
        let at = t.cave_next + c.len() as u32 + 4;
        c.extend(rest.wrapping_sub(at).to_le_bytes()); // jmp rest
        let sprintf = t.code(&c);
        let sprintf_s = t.stub_code(0x0bad_f611, false, 4, 0, false, &[]);
        let stubs = [
            (d3::GET_LANG, 0),
            (fr::FIOS_UNIFY, 3),
            (FIOS_EXISTS, 1),
            (f5::DBG_PRINTF, 2),
            (FIOS_OPEN, 2),
            (FIOS_SIZE, 1),
            (FIOS_READ, 3),
            (FIOS_CLOSE, 1),
        ];
        for (sym, n) in stubs {
            t.stub_at(sym, false, n, 0, &[]);
        }
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.extend([0x0bad_f610, 0x0bad_f611]);
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(f3::SPRINTF_IAT, sprintf);
            t.g32(f6::SPRINTF_S_IAT, sprintf_s);
            let lang = rng.below(13);
            t.ret(d3::GET_LANG, rng.pick(&[lang, lang, 0xffff_ffff]));
            t.ret(FIOS_EXISTS, rng.pick(&[0, 1, 1]));
            let size = rng.pick(&[0, 1, 0x10, 0x11, 0x1234]);
            t.ret(FIOS_SIZE, size);
            let out = random_block(&mut t, &mut rng, 8);
            let nm = rng.next();
            let frame = t.ebp_for(2) - 0x1c4;
            t.check_framed(
                &format!("{w} lang {lang}"),
                0x0088_ccf0,
                &[nm, out],
                frame,
                &[(0, 0x1c0)],
                |g| u(f6::read_menu_message(g, frame, nm, out)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

// ---- continuation round 4, batch 7 (ffx_glue_other::h4_f7)

/// The sound event group's list load.
#[test]
fn f7_event_group_load() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f701);
        // Array_Reserve: capacity and entries from cells (a capacity below
        // n makes the appends grow the array)
        let (cap_cell, buf_cell) = (t.zeros(4), t.zeros(4));
        let rest = t.stub_code(f7::ARRAY_RESERVE, true, 1, 1, false, &[]);
        let mut c = vec![0xa1];
        c.extend(cap_cell.to_le_bytes());
        c.extend([0x89, 0x41, 0x04, 0xa1]);
        c.extend(buf_cell.to_le_bytes()); // mov eax, [cap]; mov [ecx+4], eax; mov eax, [buf]
        c.extend([0x89, 0x41, 0x08, 0xe9]); // mov [ecx+8], eax; jmp rest
        let at = t.cave_next + c.len() as u32 + 4;
        c.extend(rest.wrapping_sub(at).to_le_bytes());
        let f = t.code(&c);
        t.jump(f7::ARRAY_RESERVE, f);
        let heap = random_block(&mut t, &mut rng, 0x8000);
        let next = stub_malloc(&mut t, heap);
        let stubs = [
            (FIOS_EXISTS, false, 1, 0),
            (FIOS_OPEN, false, 2, 0),
            (FIOS_SIZE, false, 1, 0),
            (OPERATOR_NEW_2, false, 1, 0),
            (FIOS_READ, false, 3, 0),
            (FIOS_CLOSE, false, 1, 0),
            (MEMCPY, false, 3, 0),
            (c2::USER_FREE_THUNK, false, 1, 0),
            (f7::ID_TREE_INSERT_ARRAY, true, 2, 2),
            (e5::OPERATOR_DELETE_ARRAY, false, 1, 0),
        ];
        for (sym, this, n, pops) in stubs {
            t.stub_at(sym, this, n, pops, &[]);
        }
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.extend([f7::ARRAY_RESERVE, USER_MALLOC]);
        let mut ran = 0;
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.w32(next, heap);
            let this = random_block(&mut t, &mut rng, 0x10);
            t.ret(FIOS_EXISTS, rng.pick(&[0, 1, 1, 1]));
            let file = rng.next() | 1;
            t.ret(FIOS_OPEN, rng.pick(&[0, file, file, file]));
            // the records: key, count, ids
            let buf = t.zeros(0x100);
            let mut words = Vec::new();
            for _ in 0..rng.below(5) {
                words.push(rng.next());
                let n = rng.pick(&[0, 1, 2, 5, 9]);
                words.push(n);
                for _ in 0..n {
                    words.push(rng.next());
                }
            }
            let cut = rng.below(4) as usize;
            let len = words.len().saturating_sub(cut);
            for (k, v) in words[..len].iter().enumerate() {
                t.w32(buf + 4 * k as u32, *v);
            }
            let size = 4 * len as u32 + rng.pick(&[0, 0, 1, 3]);
            t.ret(FIOS_SIZE, size);
            t.ret(OPERATOR_NEW_2, buf);
            let block = random_block(&mut t, &mut rng, 0x40);
            // a reserve always has entries (a null one would write near 0)
            t.w32(buf_cell, block);
            t.w32(cap_cell, rng.pick(&[1, 3, 100]));
            let path = rng.next();
            let frame = t.ebp_for(1) - 0x34;
            t.check_framed_conv(
                &format!("{w} load {size}"),
                0x0071_0c50,
                Conv::Thiscall(this),
                &[path],
                frame,
                &[(0, 0xc), (0x10, 4)],
                |g| u(f7::event_group_load(g, frame, this, path)),
            );
            ran += t.log_entries();
        }
        assert!(ran > 0, "{name}: the stubs ran");
    }
}

// ---- continuation round 5, batch 1 (ffx_glue_other::h4_g1)

/// The remap and shape stubs of the g1 drawers; their tags.
fn g1_shape_stubs(t: &mut Twins) -> Vec<u32> {
    t.stub(e1::REMAP_X2, 1, true);
    t.stub(e1::REMAP_Y2, 1, true);
    t.stub(e1::GET_TIME, 0, true);
    let stubs = [
        (e1::SHAPE_UV, 9),
        (e1::SHAPE_UV_C2, 11),
        (g1::SHAPE_XY_C2, 5),
        (g1::CTRL_PKT, 0),
        (g1::MES_FONT_DRAW, 12),
        (g1::COMMAND_WINDOW, 1),
        (g1::SET_CUR_POS, 3),
    ];
    for (sym, n) in stubs {
        t.stub(sym, n, false);
    }
    let mut tags = stubs.map(|s| s.0).to_vec();
    tags.extend([e1::REMAP_X2, e1::REMAP_Y2, e1::GET_TIME]);
    tags
}

/// Fresh `ST0` results for the float stubs.
fn g1_floats(t: &mut Twins, rng: &mut Rng) {
    for tag in [e1::REMAP_X2, e1::REMAP_Y2, e1::GET_TIME] {
        let v = rng.double();
        t.ret_f64(tag, v);
    }
}

/// The damage digits, the battle icons, the limit plates and gauge.
#[test]
fn g1_shapes() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_9101);
        let tags = g1_shape_stubs(&mut t);
        let f32s = [
            0x00b5_ee90,
            0x00b6_56c4,
            0x00b5_9830,
            0x00b4_9988,
            0x00b6_67e0,
            0x00b6_6dc0,
            0x00b6_56e4,
            0x00b6_6dd8,
            0x00b3_eb9c,
            0x00b6_56d8,
            0x00b6_6df4,
            0x00b6_6db8,
            0x00b6_6e1c,
            0x00b6_6dd0,
            0x00b6_6d90,
            0x00b6_6db4,
            0x00b6_6dbc,
            0x00b6_6dc8,
            0x00b6_67e8,
            0x00b6_6d80,
            0x00b6_5868,
            0x00b5_ef64,
            0x00b4_1d88,
            0x00b5_f434,
            0x00b4_3dc0,
            0x00b5_ef1c,
            0x00b5_f38c,
            0x00b5_f3a0,
            0x00b5_f348,
            0x00b5_f3c4,
            0x00b5_f3a8,
            0x00b5_efa0,
            0x00b5_f31c,
            0x00b5_f42c,
            0x00b5_f3d0,
            0x00b5_f368,
            0x00b9_2308,
            0x00b5_4c78,
            0x00b5_f430,
            0x00b5_f374,
            0x00b5_ee98,
            0x00b6_56c0,
            0x00b6_6b70,
            0x00b6_6b5c,
            0x00b6_6b7c,
            0x00b5_f0a8,
            0x00b6_6ba0,
            0x00b6_6ba4,
        ];
        let f64s = [
            0x00b6_5ad0,
            0x00b6_6d28,
            0x00b5_8b50,
            0x00b6_6cf8,
            0x00b6_6c48,
            0x00b6_6c40,
        ];
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(&mut rng, &f32s, &f64s);
            g1_floats(&mut t, &mut rng);
            let n = {
                let r0_ = rng.small();
                let r1_ = rng.next();
                rng.pick(&[r0_ % 30, r0_, r1_, 0x8000_0000, 0xffff_fffb])
            };
            let (x, y) = (rng.small(), rng.small());
            let rgba: [u32; 4] = std::array::from_fn(|_| {
                let r0_ = rng.below(256);
                let r1_ = rng.next();
                rng.pick(&[r0_, r0_, r1_])
            });
            let mut args = vec![n, x, y];
            args.extend(rgba);
            t.check(&format!("{w} damage"), 0x008f_b840, &args, |g| {
                u(g1::damage_num(g, n, x, y, rgba))
            });
            let kind = rng.pick(&[0, 1, 2, 3, 4, 0xffff_ffff]);
            let alpha = {
                let r0_ = rng.next();
                rng.pick(&[0x80, 0, r0_])
            };
            t.check(
                &format!("{w} icon"),
                0x008f_bcc0,
                &[kind, x, y, alpha],
                |g| u(g1::btl_icon(g, kind, x, y, alpha)),
            );
            t.check(&format!("{w} limit bg"), 0x008a_1ce0, &[], |g| {
                u(g1::limit_bg(g))
            });
            let gw = {
                let r0_ = rng.small();
                let r1_ = rng.next();
                rng.pick(&[r0_, r1_])
            };
            t.check(&format!("{w} gauge"), 0x0090_1300, &[x, y, gw], |g| {
                u(g1::limit_special_gauge(g, x, y, gw))
            });
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The ratio number, the simple help and the wait window's cursor.
#[test]
fn g1_numbers_help_cursor() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_9102);
        let mut tags = g1_shape_stubs(&mut t);
        let width = t.zeros(4);
        let f = t.stub_code(g1::MES_FONT_WIDTH, false, 5, 0, false, &[wr(2, width)]);
        t.jump(g1::MES_FONT_WIDTH, f);
        tags.push(g1::MES_FONT_WIDTH);
        let f32s = [
            0x00b5_eea8,
            0x00b5_f030,
            0x00b4_3db8,
            0x00b5_eef0,
            0x00b5_efb0,
            0x00b5_efcc,
            0x00b5_ee64,
            0x00b5_eedc,
            0x00b5_eed0,
            0x00b5_edf4,
            0x00b5_ee20,
            0x00b5_ee0c,
            0x00b5_ee2c,
            0x00b5_ee10,
            0x00b5_eee4,
        ];
        let f64s = [0x00b9_22e0, 0x00b4_1d38, 0x00b9_2220];
        let win = t.zeros(0x100);
        let mut shown = 0;
        let mut cursors = 0;
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(&mut rng, &f32s, &f64s);
            if rng.below(2) == 0 {
                let v = rng.pick(&[100.0f64, 10.0, 1000.0]);
                let at = t.rebase(0x00b9_22e0);
                t.write(at, &v.to_le_bytes());
            }
            g1_floats(&mut t, &mut rng);
            // the ratio number
            let ratio = {
                let a = (rng.below(10_000) as f32 / 100.0).to_bits();
                let b = rng.float();
                let c = (rng.below(100_000) as f32 / 3.0 - 20_000.0).to_bits();
                rng.pick(&[a, a, b, c])
            };
            let (x, y, c) = (rng.small(), rng.small(), rng.next());
            let sse = rng.pick(&[0, 1]);
            t.g32(ffx_bridge::x87::SSE2_AVAILABLE.sym, sse);
            t.check(&format!("{w} ratio"), 0x0090_1490, &[ratio, x, y, c], |g| {
                u(g1::ratio_number(g, ratio, x, y, c))
            });
            // the simple help
            let idx = rng.below(4);
            for k in 0..4 {
                let at = t.rebase(g1::SIMPLE_HELP) + 16 * k;
                let on = rng.pick(&[0, 1, 1]);
                t.w32(at, on);
                let (a, b) = (rng.next(), rng.next());
                t.w32(at + 4, a);
                t.w32(at + 8, b);
            }
            let wv = {
                let r0_ = rng.float();
                let r1_ = (rng.below(600) as f32).to_bits();
                rng.pick(&[r0_, r1_])
            };
            t.w32(width, wv);
            let slot = t.ebp_for(1) + 8;
            t.check_framed(
                &format!("{w} help"),
                0x0089_d9b0,
                &[idx],
                slot,
                &[(0, 4)],
                |g| match g1::simple_help(g, slot, idx) {
                    Some(v) => {
                        shown += 1;
                        u(v)
                    }
                    None => Out::None,
                },
            );
            // the wait window's cursor
            for k in 0..0x40 {
                let v = rng.next();
                t.w32(win + 4 * k, v);
            }
            let start = {
                let r0_ = rng.float();
                rng.pick(&[
                    0,
                    0x8000_0000,
                    1f32.to_bits(),
                    0xbf80_0000,
                    0x7fc0_0000,
                    r0_,
                ])
            };
            t.w32(win + 0xdc, start);
            let cols = rng.pick(&[0u16, 1, 2, 3, 5, 0xffff]);
            let rows = rng.pick(&[0u16, 1, 2, 4, 0xfffe]);
            let first = rng.pick(&[0u16, 1, 3, 0xffff]);
            let cursor = (i32::from(first as i16) + rng.below(12) as i32 - 2) as u16;
            t.w16(win + 0x38, cols);
            t.w16(win + 0x3a, rows);
            t.w16(win + 0x40, first);
            t.w16(win + 0x42, cursor);
            let step = rng.small() as u16;
            t.w16(win + 0x62, step);
            let fx = {
                let r0_ = rng.float();
                let r1_ = (rng.below(500) as f32).to_bits();
                rng.pick(&[r0_, r1_])
            };
            let fy = rng.float();
            t.w32(win + 0x78, fx);
            t.w32(win + 0x7c, fy);
            let skip = rng.pick(&[0, 0, 1, 2]);
            t.g32(g1::WAIT_WIN_SKIP.sym, skip);
            let before = t.log_entries();
            t.check(
                &format!("{w} wait sub"),
                0x008a_7520,
                &[win],
                |g| match g1::wait_win_sub(g, win) {
                    Some(v) => u(v),
                    None => Out::None,
                },
            );
            if t.log_entries() > before + 1 {
                cursors += 1;
            }
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
        assert!(
            shown > 50 && cursors > 20,
            "{name}: {shown} helps, {cursors} cursors"
        );
    }
}

/// The three UI records and `movie_disp2`.
#[test]
fn g1_records_movie() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_9103);
        let (cw, ch) = (t.zeros(4), t.zeros(4));
        let f = t.stub_code(g1::IMAGE_WH, false, 3, 0, false, &[wr(1, cw), wr(2, ch)]);
        t.jump(g1::IMAGE_WH, f);
        let stubs = [
            (g1::SHAPE_TEXTURE, 1),
            (e4::DRAW_UI_ELEMENT, 5),
            (g1::BEGIN_SCISSOR_UV, 8),
        ];
        for (sym, n) in stubs {
            t.stub(sym, n, false);
        }
        t.stub_at(g1::MATRIX_COPY, true, 1, 1, &[]);
        t.stub_at(g1::RENDER_TO_TEXTURE, true, 11, 11, &[]);
        let mut tags = stubs.map(|s| s.0).to_vec();
        tags.extend([g1::IMAGE_WH, g1::MATRIX_COPY, g1::RENDER_TO_TEXTURE]);
        let f64s = [
            0x00b4_1d38,
            0x00b4_1d68,
            0x00b4_1d30,
            0x00b4_1d58,
            0x00b9_22b0,
            0x00b3_13a0,
        ];
        let vecs = t.zeros(0x20);
        let compare = |t: &mut Twins, w: &str, at: u32, len: usize| {
            let (mut x, mut y) = (vec![0; len], vec![0; len]);
            t.a.read_bytes(at, &mut x);
            t.b.read_bytes(at, &mut y);
            assert_eq!(x, y, "{w}: frame +{at:#x}");
        };
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(&mut rng, &[], &f64s);
            let size = {
                let r0_ = rng.float();
                rng.pick(&[512f32.to_bits(), 256f32.to_bits(), r0_])
            };
            t.w32(cw, size);
            let size = {
                let r0_ = rng.float();
                rng.pick(&[512f32.to_bits(), 1024f32.to_bits(), r0_])
            };
            t.w32(ch, size);
            let keep = rng.pick(&[0, 1, 1]);
            t.ret(g1::BEGIN_SCISSOR_UV, keep);
            // the shape cell
            let a: [u32; 6] = [
                {
                    let r0_ = rng.below(40);
                    let r1_ = rng.next();
                    rng.pick(&[r0_, r0_, r1_, 0x8000_0000])
                },
                rng.float(),
                rng.float(),
                rng.float(),
                rng.float(),
                rng.next(),
            ];
            let frame = t.ebp_for(6) - 0xa8;
            t.write(frame, &[0; g1::CELL_FRAME as usize]);
            t.check_framed(&format!("{w} cell"), 0x008f_8d50, &a, frame, &[], |g| {
                u(g1::shape_cell(g, frame, a))
            });
            compare(&mut t, &format!("{w} cell"), frame, 8);
            compare(&mut t, &format!("{w} cell"), frame + 0xc, 0x98);
            // the layout image
            let a: [u32; 8] = std::array::from_fn(|k| {
                if k < 6 {
                    let r0_ = rng.small();
                    let r1_ = rng.next();
                    rng.pick(&[r0_, r0_, r1_])
                } else {
                    rng.next()
                }
            });
            let frame = t.ebp_for(8) - 0xa8;
            t.write(frame, &[0; g1::CELL_FRAME as usize]);
            t.check_framed(&format!("{w} layout"), 0x008f_59c0, &a, frame, &[], |g| {
                u(g1::seq_layout_image(g, frame, a))
            });
            compare(&mut t, &format!("{w} layout"), frame, 8);
            compare(&mut t, &format!("{w} layout"), frame + 0xc, 0x98);
            // the menu icon
            let a: [u32; 8] = std::array::from_fn(|k| match k {
                1 | 2 => rng.small(),
                _ => rng.next(),
            });
            let frame = t.ebp_for(8) - 0xac;
            t.write(frame, &[0; g1::ICON_FRAME as usize]);
            t.check_framed(&format!("{w} icon"), 0x008e_b270, &a, frame, &[], |g| {
                u(g1::menu_icon(g, frame, a))
            });
            compare(&mut t, &format!("{w} icon"), frame, 8);
            compare(&mut t, &format!("{w} icon"), frame + 0x10, 0x98);
            // movie_disp2
            for k in 0..8 {
                let v = rng.float();
                t.w32(vecs + 4 * k, v);
            }
            let scene = rng.next();
            t.g32(g1::PHYRE_SCENE.sym, scene);
            let a = [
                rng.pick(&[0, 1, 2]),
                {
                    let r0_ = rng.next();
                    rng.pick(&[0, r0_])
                },
                rng.pick(&[0, vecs]),
                rng.pick(&[0, vecs + 0x10]),
                rng.next(),
            ];
            let frame = t.ebp_for(5) - 0x60;
            t.write(frame, &[0x5a; 0x40]);
            t.check_framed(&format!("{w} disp2"), 0x0064_2fe0, &a, frame, &[], |g| {
                u(g1::movie_disp2(g, frame, a))
            });
            compare(&mut t, &format!("{w} disp2"), frame, 0x40);
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

// ---- continuation round 5, batch 2 (ffx_glue_other::h4_g2)

/// The distortion grid's UVs and positions.
#[test]
fn g2_grid() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_9201);
        let this = t.zeros(0x80);
        let uv = t.zeros(0x400);
        let pos = t.zeros(0x400);
        t.w32(this + 0x70, uv);
        t.w32(this + 0x68, pos);
        let f32s = [
            0x00b4_9368,
            0x00b9_2180,
            0x00b4_936c,
            0x00b9_2194,
            0x00b4_9370,
            0x00b4_3d18,
            0x00b4_9374,
            0x00b4_7c9c,
            0x00b9_2338,
            0x00b4_9378,
        ];
        for i in 0..200 {
            let w = format!("{name} {i}");
            t.constants(&mut rng, &f32s, &[0x00b9_2200, 0x00b3_4f90]);
            if rng.below(2) == 0 {
                for (k, v) in [(0x00b9_2200, 0.125f64), (0x00b3_4f90, 0.25)] {
                    let at = t.rebase(k);
                    t.write(at, &v.to_le_bytes());
                }
            }
            t.check_conv(
                &format!("{w} uv"),
                0x006c_2bb0,
                Conv::Thiscall(this),
                &[],
                |g| u(g2::grid_uv(g, this)),
            );
            t.check_conv(
                &format!("{w} positions"),
                0x006c_2ca0,
                Conv::Thiscall(this),
                &[],
                |g| u(g2::grid_positions(g, this)),
            );
        }
    }
}

/// The extended blur's two constructors.
#[test]
fn g2_blur_ctors() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_9202);
        t.stub_at(g2::SAMPLER_CTOR, true, 0, 0, &[]);
        t.stub(g2::OPERATOR_NEW, 1, false);
        let tags = [g2::SAMPLER_CTOR, g2::OPERATOR_NEW];
        let params = t.zeros(0x88);
        for i in 0..200 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(&mut rng, &[0x00b9_2194], &[]);
            let this = random_block(&mut t, &mut rng, 0x90);
            t.check_conv(
                &format!("{w} params"),
                0x006c_4020,
                Conv::Thiscall(this),
                &[],
                |g| u(g2::blur_params_ctor(g, this)),
            );
            for k in 0..0x88 / 4 {
                let v = rng.next();
                t.w32(params + 4 * k, v);
            }
            let p = rng.pick(&[0, params, params]);
            t.ret(g2::OPERATOR_NEW, p);
            t.check_conv(
                &format!("{w} blur ex"),
                0x006c_4820,
                Conv::Thiscall(this),
                &[],
                |g| u(g2::blur_ex_ctor(g, this)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// `TextUtil`'s and the post-process tweak tool's constructors.
#[test]
fn g2_text_tweak_ctors() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_9203);
        t.stub_at(g2::MATRIX_COPY, true, 1, 1, &[]);
        let tags = [g2::MATRIX_COPY];
        for i in 0..200 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(&mut rng, &[0x00b9_2194, 0x00b3_eb94], &[]);
            let this = random_block(&mut t, &mut rng, 0x90);
            let frame = t.ebp_for(0) - g2::IDENTITY_FRAME;
            t.write(frame, &[0x5a; g2::IDENTITY_FRAME as usize]);
            t.check_framed_conv(
                &format!("{w} text util"),
                0x006f_5b50,
                Conv::Thiscall(this),
                &[],
                frame,
                &[],
                |g| u(g2::text_util_ctor(g, frame, this)),
            );
            let (mut x, mut y) = ([0; 0x40], [0; 0x40]);
            t.a.read_bytes(frame, &mut x);
            t.b.read_bytes(frame, &mut y);
            assert_eq!(x, y, "{w}: identity");
        }
        // the tweak tool, with TextUtil's constructor a stub
        t.stub_at(g2::TEXT_UTIL_CTOR, true, 0, 0, &[]);
        let tags = [g2::TEXT_UTIL_CTOR];
        for i in 0..200 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(&mut rng, &[0x00b1_f2b0], &[]);
            let this = random_block(&mut t, &mut rng, 0x190);
            t.check_conv(
                &format!("{w} tweak"),
                0x0069_2410,
                Conv::Thiscall(this),
                &[],
                |g| u(g2::tweak_util_ctor(g, this)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// An Iggy menu's viewport and a `GDraw` rectangle's UVs.
#[test]
fn g2_iggy_gdraw() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_9204);
        t.stub_at(g2::SET_VIEWPORT, true, 4, 4, &[]);
        let tags = [g2::SET_VIEWPORT];
        let state = t.zeros(0x700);
        let out = t.zeros(0x10);
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let sse = rng.pick(&[0, 1]);
            t.g32(ffx_bridge::x87::SSE2_AVAILABLE.sym, sse);
            // viewport
            let this = random_block(&mut t, &mut rng, 0xa0);
            for k in 0..4 {
                let v = {
                    let r0_ = rng.float();
                    let r1_ = (rng.below(300) as f32 / 100.0).to_bits();
                    rng.pick(&[r0_, r1_, r1_])
                };
                t.w32(this + 0x74 + 4 * k, v);
            }
            let a: [u32; 4] = std::array::from_fn(|_| {
                let r0_ = rng.small();
                let r1_ = rng.next();
                rng.pick(&[r0_, r0_, r1_])
            });
            t.check_conv(
                &format!("{w} viewport"),
                0x00a2_8f40,
                Conv::Thiscall(this),
                &a,
                |g| u(g2::iggy_set_viewport(g, this, a)),
            );
            // GDraw sub-rectangle
            t.g32(g2::GDRAW_STATE.sym, state);
            for k in 0..0x700 / 4 {
                let v = rng.small();
                t.w32(state + 4 * k, v);
            }
            for off in [0x6c0, 0x6c4] {
                let v = {
                    let r0_ = rng.next();
                    rng.pick(&[0, 1280, 720, 1, r0_, 0xffff_fff0])
                };
                t.w32(state + off, v);
            }
            t.constants(&mut rng, &[], &[0x00b9_2220]);
            let slot = state + 0x438 + 0x38 * rng.below(5) + rng.below(0x38);
            t.check(&format!("{w} gdraw"), 0x00a3_c1b0, &[out, slot], |g| {
                u(g2::gdraw_sub_rect_uv(g, out, slot))
            });
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

// ---- continuation round 6, batch 1 (ffx_glue_other::h4_h1)

/// A random `PString` word: 0, a text (non-zero bytes, 0..24 of them), or
/// a static one (bit 0 set).
fn h1_string(t: &mut Twins, rng: &mut Rng) -> u32 {
    if rng.below(5) == 0 {
        return 0;
    }
    let len = rng.below(24) as usize;
    let mut text: Vec<u8> = (0..len).map(|_| 1 + rng.below(255) as u8).collect();
    text.push(0);
    let at = t.alloc(&text);
    assert_eq!(at & 1, 0);
    at | rng.below(2)
}

/// A random word array `(count, block)` at `at`: a count of 0..5 with or
/// without the top bit, a block of random words.
fn h1_words(t: &mut Twins, rng: &mut Rng, at: u32) {
    let n = rng.below(6);
    let top = rng.pick(&[0, 0, 0x8000_0000]);
    let data = random_block(t, rng, 4 * 8);
    t.w32(at, n | top);
    t.w32(at + 4, data);
}

/// A random source of `n` records of `size` bytes: random words, floats
/// at `floats`, `PString`s at `strings`, word arrays at `arrays`.
fn h1_records(
    t: &mut Twins,
    rng: &mut Rng,
    n: u32,
    size: u32,
    floats: &[u32],
    strings: &[u32],
    arrays: &[u32],
) -> u32 {
    let src = random_block(t, rng, n.max(1) * size);
    for k in 0..n {
        let r = src + k * size;
        for &off in floats {
            let v = rng.float();
            t.w32(r + off, v);
        }
        for &off in strings {
            let s = h1_string(t, rng);
            t.w32(r + off, s);
        }
        for &off in arrays {
            h1_words(t, rng, r + off);
        }
    }
    src
}

/// The allocation stub's next block: null sometimes, else a fresh block.
fn h1_block(t: &mut Twins, rng: &mut Rng, tag: u32, bytes: u32) {
    let b = random_block(t, rng, bytes);
    let v = rng.pick(&[0, b, b, b]);
    t.ret(tag, v);
}

/// A destination of `n` records of `size` bytes: null sometimes for one
/// record (skipped), else random bytes.
fn h1_dst(t: &mut Twins, rng: &mut Rng, n: u32, size: u32) -> u32 {
    if n == 1 && rng.below(4) == 0 {
        0
    } else {
        random_block(t, rng, n.max(1) * size)
    }
}

/// `TextureAnimationInfo`'s and `TextureAnimationMaterial`'s copies, one
/// and as arrays (the `Char` array copy and the copy helpers stubs).
#[test]
fn h1_info_material_copies() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a101);
        t.stub(h1::PHYRE_MALLOC, 2, false);
        t.stub(h1::MEMCPY, 3, false);
        t.stub(h1::COPY_WORDS, 2, false);
        t.stub(h1::COPY_NEW_A, 2, false);
        t.stub(h1::COPY_NEW_B, 2, false);
        t.stub(h1::CHAR_COPY_ARRAY, 3, false);
        let tags = [
            h1::PHYRE_MALLOC,
            h1::MEMCPY,
            h1::COPY_WORDS,
            h1::COPY_NEW_A,
            h1::COPY_NEW_B,
            h1::CHAR_COPY_ARRAY,
        ];
        for i in 0..200 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            for tag in [
                h1::PHYRE_MALLOC,
                h1::COPY_WORDS,
                h1::COPY_NEW_A,
                h1::COPY_NEW_B,
            ] {
                h1_block(&mut t, &mut rng, tag, 0x100);
            }
            // one Info
            let src = h1_records(&mut t, &mut rng, 1, 0x20, &[0x10, 0x14], &[4], &[8]);
            let this = random_block(&mut t, &mut rng, 0x20);
            t.check_conv(
                &format!("{w} info"),
                0x006a_f0d0,
                Conv::Thiscall(this),
                &[src],
                |g| u(h1::info_copy(g, this, src)),
            );
            // Info array
            let n = rng.below(4);
            let src = h1_records(&mut t, &mut rng, n, 0x20, &[0x10, 0x14], &[4], &[8]);
            let dst = h1_dst(&mut t, &mut rng, n, 0x20);
            t.check(
                &format!("{w} info array"),
                0x006a_e9c0,
                &[dst, src, n],
                |g| {
                    let r = h1::info_copy_array(g, dst, src, n);
                    if n == 0 { Out::None } else { u(r) }
                },
            );
            // one Material
            let src = h1_records(&mut t, &mut rng, 1, 0x20, &[], &[0x18, 0x1c], &[0, 8, 0x10]);
            let this = random_block(&mut t, &mut rng, 0x20);
            t.check_conv(
                &format!("{w} material"),
                0x006a_eee0,
                Conv::Thiscall(this),
                &[src],
                |g| u(h1::material_copy(g, this, src)),
            );
            // Material array
            let n = rng.below(4);
            let src = h1_records(&mut t, &mut rng, n, 0x20, &[], &[0x18, 0x1c], &[0, 8, 0x10]);
            let dst = h1_dst(&mut t, &mut rng, n, 0x20);
            t.check(
                &format!("{w} material array"),
                0x006a_e730,
                &[dst, src, n],
                |g| {
                    let r = h1::material_copy_array(g, dst, src, n);
                    if n == 0 { Out::None } else { u(r) }
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// `TextureAnimationInfoChar`'s copies, one and as an array (the base
/// copy a stub).
#[test]
fn h1_info_char_copies() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a102);
        t.stub(h1::PHYRE_MALLOC, 2, false);
        t.stub(h1::MEMCPY, 3, false);
        t.stub_at(h1::INFO_COPY, true, 1, 1, &[]);
        let tags = [h1::PHYRE_MALLOC, h1::MEMCPY, h1::INFO_COPY];
        let floats = [0x24, 0x28, 0x2c, 0x30];
        for i in 0..200 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            h1_block(&mut t, &mut rng, h1::PHYRE_MALLOC, 0x40);
            let src = h1_records(&mut t, &mut rng, 1, 0x40, &floats, &[0x20], &[]);
            let this = random_block(&mut t, &mut rng, 0x40);
            t.check_conv(
                &format!("{w} char"),
                0x006a_f270,
                Conv::Thiscall(this),
                &[src],
                |g| u(h1::info_char_copy(g, this, src)),
            );
            let n = rng.below(4);
            let src = h1_records(&mut t, &mut rng, n, 0x40, &floats, &[0x20], &[]);
            let dst = h1_dst(&mut t, &mut rng, n, 0x40);
            t.check(
                &format!("{w} char array"),
                0x006a_eb20,
                &[dst, src, n],
                |g| {
                    let r = h1::info_char_copy_array(g, dst, src, n);
                    if n == 0 { Out::None } else { u(r) }
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The small constructors: the semaphore queues, the post-process state,
/// the dynamic-geometry pool, the shadow manager, the shader-source
/// reader and the escape menu's creation.
#[test]
fn h1_ctors() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a103);
        t.stub_at(h1::EH_VECTOR_CTOR, false, 5, 5, &[]);
        t.stub(h1::OPERATOR_NEW, 1, false);
        t.stub(h1::PHYRE_MALLOC, 2, false);
        t.stub(h1::PHYRE_FREE, 1, false);
        t.stub(h1::MEMSET, 3, false);
        t.stub(h1::NEW_TREE_HEAD, 0, false);
        for sym in [
            h1::PAIR_INIT,
            h1::PCLUSTER_CTOR,
            h1::SAMPLER_STATE_CTOR,
            h1::ASSET_LIST_CLEAR,
        ] {
            t.stub_at(sym, true, 0, 0, &[]);
        }
        t.stub_at(h1::STRING_ASSIGN, true, 2, 2, &[]);
        t.stub_at(h1::ESC_MENU_CTOR, true, 5, 5, &[]);
        let semaphore = t.stub_code(0x0bad_a1c0, false, 4, 4, false, &[]);
        let mutex = t.stub_code(0x0bad_a1c1, false, 3, 3, false, &[]);
        let tags = [
            h1::EH_VECTOR_CTOR,
            h1::OPERATOR_NEW,
            h1::PHYRE_MALLOC,
            h1::PHYRE_FREE,
            h1::MEMSET,
            h1::NEW_TREE_HEAD,
            h1::PAIR_INIT,
            h1::PCLUSTER_CTOR,
            h1::SAMPLER_STATE_CTOR,
            h1::ASSET_LIST_CLEAR,
            h1::STRING_ASSIGN,
            h1::ESC_MENU_CTOR,
            0x0bad_a1c0,
            0x0bad_a1c1,
        ];
        for i in 0..200 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(h1::CREATE_SEMAPHORE_IAT, semaphore);
            t.g32(h1::CREATE_MUTEX_IAT, mutex);
            // the queue's boxes are never null here: the original reads
            // address 0 then, as the port does
            let b = random_block(&mut t, &mut rng, 8);
            t.ret(h1::OPERATOR_NEW, b);
            let this = random_block(&mut t, &mut rng, 0x1f0);
            t.check_conv(
                &format!("{w} queue"),
                0x006d_b560,
                Conv::Thiscall(this),
                &[],
                |g| u(h1::semaphore_queue_ctor(g, this)),
            );
            let this = random_block(&mut t, &mut rng, 0x240);
            t.check_conv(
                &format!("{w} queues"),
                0x006d_b670,
                Conv::Thiscall(this),
                &[],
                |g| u(h1::semaphore_queues_ctor(g, this)),
            );
            h1_block(&mut t, &mut rng, h1::PHYRE_MALLOC, 8);
            let this = random_block(&mut t, &mut rng, 0x110);
            t.check_conv(
                &format!("{w} post"),
                0x006d_dbf0,
                Conv::Thiscall(this),
                &[],
                |g| u(h1::post_process_state_init(g, this)),
            );
            let this = random_block(&mut t, &mut rng, 0x18);
            t.check_conv(
                &format!("{w} pool"),
                0x006d_e2c0,
                Conv::Thiscall(this),
                &[],
                |g| u(h1::dyn_geo_pool_ctor(g, this)),
            );
            let this = random_block(&mut t, &mut rng, 0x4068);
            t.check_conv(
                &format!("{w} shadow"),
                0x006c_7d80,
                Conv::Thiscall(this),
                &[],
                |g| u(h1::shadow_manager_ctor(g, this)),
            );
            let this = random_block(&mut t, &mut rng, 0x44);
            t.check_conv(
                &format!("{w} reader"),
                0x0069_04c0,
                Conv::Thiscall(this),
                &[],
                |g| u(h1::shader_source_reader_ctor(g, this)),
            );
            // the escape menu
            let other = rng.next();
            let menu = rng.pick(&[0, 0, other]);
            t.g32(h1::ESC_MENU.sym, menu);
            let b = random_block(&mut t, &mut rng, 8);
            let v = rng.pick(&[0, b, b]);
            t.ret(h1::OPERATOR_NEW, v);
            let a = rng.next();
            let f: [u32; 4] = std::array::from_fn(|_| rng.float());
            t.check(
                &format!("{w} esc menu"),
                0x0068_e400,
                &[a, f[0], f[1], f[2], f[3]],
                |g| match h1::esc_menu_create(g, a, f) {
                    Some(v) => u(v),
                    None => Out::None,
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

// ---- continuation round 6, batch 2 (ffx_glue_other::h4_h2)

/// The loading screen's constructor and the broken-screen render target.
#[test]
fn h2_loading_broken_screen() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a201);
        t.stub(h2::PHYRE_MALLOC1, 1, false);
        t.stub_at(h2::PCLUSTER_CTOR, true, 0, 0, &[]);
        for sym in [h2::SYNC_RENDER, h2::CLUSTER_CLEAR, h2::UPDATE_MAX_LEVEL] {
            t.stub_at(sym, true, 0, 0, &[]);
        }
        t.stub_at(h2::CLUSTER_CREATE, true, 3, 3, &[]);
        t.stub_at(h2::ALLOCATE_INSTANCE_LIST, true, 3, 3, &[]);
        t.stub_at(h2::SET_DIMENSIONS, true, 4, 4, &[]);
        t.stub_at(h2::RENDER_TARGET_CTOR, true, 1, 1, &[]);
        let tags = [
            h2::PHYRE_MALLOC1,
            h2::PCLUSTER_CTOR,
            h2::SYNC_RENDER,
            h2::CLUSTER_CLEAR,
            h2::UPDATE_MAX_LEVEL,
            h2::CLUSTER_CREATE,
            h2::ALLOCATE_INSTANCE_LIST,
            h2::SET_DIMENSIONS,
            h2::RENDER_TARGET_CTOR,
        ];
        for i in 0..200 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            h1_block(&mut t, &mut rng, h2::PHYRE_MALLOC1, 0x50);
            let this = random_block(&mut t, &mut rng, 0x48);
            t.check_conv(
                &format!("{w} loading"),
                0x006e_45e0,
                Conv::Thiscall(this),
                &[],
                |g| u(h2::loading_screen_ctor(g, this)),
            );
            // the render target
            let this = random_block(&mut t, &mut rng, 0x78);
            let flag = rng.pick(&[0, 1, 0x80]);
            t.w8(this + 0x74, flag);
            h1_block(&mut t, &mut rng, h2::CLUSTER_CREATE, 0x20);
            let list = random_block(&mut t, &mut rng, 0x38);
            let target = random_block(&mut t, &mut rng, 0x10);
            let target = rng.pick(&[0, target, target]);
            t.w32(list + 0x34, target);
            let list = rng.pick(&[0, list, list, list]);
            t.ret(h2::ALLOCATE_INSTANCE_LIST, list);
            t.check_conv(
                &format!("{w} broken"),
                0x006d_5050,
                Conv::Thiscall(this),
                &[],
                |g| u(h2::broken_screen_create_render_target(g, this)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The boot text's creation.
#[test]
fn h2_boot_text() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a202);
        t.stub(h1::OPERATOR_NEW, 1, false);
        t.stub_at(h2::TEXT_UTIL_CTOR, true, 0, 0, &[]);
        t.stub_at(h2::TEXT_UTIL_INIT, true, 0, 0, &[]);
        t.stub_at(h2::SET_TITLE_TEXT, true, 1, 1, &[]);
        t.stub_at(h2::SET_POSITION, true, 2, 2, &[]);
        t.stub_at(h2::SET_ENABLED, true, 1, 1, &[]);
        let sprintf = t.stub_code(0x0bad_a2c0, false, 8, 0, false, &[]);
        let tags = [
            h1::OPERATOR_NEW,
            h2::TEXT_UTIL_CTOR,
            h2::TEXT_UTIL_INIT,
            h2::SET_TITLE_TEXT,
            h2::SET_POSITION,
            h2::SET_ENABLED,
            0x0bad_a2c0,
        ];
        for i in 0..200 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(h2::SPRINTF_S.sym, sprintf);
            t.constants(&mut rng, &h2::POSITION, &[]);
            let b = random_block(&mut t, &mut rng, 8);
            let v = rng.pick(&[0, b, b, b]);
            t.ret(h1::OPERATOR_NEW, v);
            let other = rng.next();
            let v = rng.pick(&[0, other, other, other]);
            t.ret(h2::TEXT_UTIL_CTOR, v);
            let other = rng.next();
            let v = rng.pick(&[0, 0, other]);
            t.ret(h2::TEXT_UTIL_INIT, v);
            let this = random_block(&mut t, &mut rng, 0x160);
            t.check_conv(
                &format!("{w} boot text"),
                0x006d_dc80,
                Conv::Thiscall(this),
                &[],
                |g| u(h2::boot_text_create(g, this)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The distortion effect's setup.
#[test]
fn h2_distortion_initialize() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a203);
        t.stub(h2::PHYRE_MALLOC1, 1, false);
        t.stub_at(h2::PCLUSTER_CTOR, true, 0, 0, &[]);
        t.stub_at(h2::TEXTURE_REGISTER, true, 0, 0, &[]);
        t.stub(h2::POST_PROCESS_MANAGER, 0, false);
        t.stub(h2::FIOS_UNIFY, 3, false);
        t.stub(h2::ASYNC_LOADER, 0, false);
        t.stub(h2::LOAD_CLUSTER, 7, false);
        let tags = [
            h2::PHYRE_MALLOC1,
            h2::PCLUSTER_CTOR,
            h2::TEXTURE_REGISTER,
            h2::POST_PROCESS_MANAGER,
            h2::FIOS_UNIFY,
            h2::ASYNC_LOADER,
            h2::LOAD_CLUSTER,
        ];
        let manager = t.zeros(0x1_2400);
        for i in 0..200 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            h1_block(&mut t, &mut rng, h2::PHYRE_MALLOC1, 0x50);
            t.ret(h2::POST_PROCESS_MANAGER, manager);
            let on = rng.pick(&[0, 1, 0x40]);
            t.w8(manager + 0x1_23b4, on);
            let this = random_block(&mut t, &mut rng, 0x70);
            let a: [u32; 5] = std::array::from_fn(|_| rng.next());
            let frame = t.ebp_for(5) - 0x110;
            t.check_framed_conv(
                &format!("{w} init"),
                0x006c_2710,
                Conv::Thiscall(this),
                &a,
                frame,
                &[],
                |g| u(h2::distortion_initialize(g, frame, this, a)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The distortion effect's shader callback (the cursor seek a stub that
/// fills the cursor's result words).
#[test]
fn h2_distortion_on_shaders() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a204);
        let seek = t.stub_code(h2::CURSOR_SEEK, true, 0, 0, false, &[]);
        let cells = [t.zeros(4), t.zeros(4), t.zeros(4)];
        let mut c = Vec::new();
        for (cell, off) in cells.iter().zip([0u8, 0xc, 0x10]) {
            c.push(0xa1);
            c.extend(cell.to_le_bytes()); // mov eax, [cell]
            c.extend([0x89, 0x41, off]); // mov [ecx+off], eax
        }
        let at = t.cave_next;
        c.push(0xe9);
        c.extend(seek.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // jmp seek
        let fill = t.code(&c);
        t.jump(h2::CURSOR_SEEK, fill);
        t.stub_at(h2::DISTORTION_CREATE_RT, true, 0, 0, &[]);
        t.stub_at(h2::CREATE_MATERIAL, true, 3, 3, &[]);
        t.stub_at(h2::CLUSTER_CREATE, true, 3, 3, &[]);
        // the grid stub sets the third create's result
        let grid = t.stub_code(h2::SETUP_GRID, true, 0, 0, false, &[]);
        let third_cell = t.zeros(4);
        let create_cell = t.cells[&h2::CLUSTER_CREATE];
        let mut c = vec![0xa1];
        c.extend(third_cell.to_le_bytes()); // mov eax, [third]
        c.push(0xa3);
        c.extend(create_cell.to_le_bytes()); // mov [create], eax
        let at = t.cave_next;
        c.push(0xe9);
        c.extend(grid.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // jmp grid
        let set = t.code(&c);
        t.jump(h2::SETUP_GRID, set);
        t.stub_at(h2::SET_WORD, true, 2, 2, &[]);
        t.stub_at(h2::SET_STREAMS, true, 2, 2, &[]);
        t.stub_at(h2::GRID_INSTANCE, true, 1, 1, &[]);
        t.stub_at(h2::MESH_INSTANCE, true, 1, 1, &[]);
        t.stub(h2::PHYRE_FREE, 1, false);
        let tags = [
            h2::CURSOR_SEEK,
            h2::DISTORTION_CREATE_RT,
            h2::SETUP_GRID,
            h2::CREATE_MATERIAL,
            h2::CLUSTER_CREATE,
            h2::SET_WORD,
            h2::SET_STREAMS,
            h2::GRID_INSTANCE,
            h2::MESH_INSTANCE,
            h2::PHYRE_FREE,
        ];
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            // the cursor's results: base, hit, offset
            let hit = rng.pick(&[0, 1, 1, 1]);
            let base = rng.next();
            let other = rng.next();
            let off = rng.pick(&[base.wrapping_neg(), 0x40, other]);
            t.w32(cells[0], base);
            t.w32(cells[1], hit);
            t.w32(cells[2], off);
            // the shader cluster, its list empty or not
            let cluster = random_block(&mut t, &mut rng, 0x24);
            let list = cluster + 0x1c;
            let other = rng.next();
            let first = rng.pick(&[list, other]);
            t.w32(list, first);
            let this = random_block(&mut t, &mut rng, 0x44);
            t.w32(this + 0x20, cluster);
            // the material list: a count with or without its top bit
            let array = random_block(&mut t, &mut rng, 0x10);
            let n = rng.pick(&[0, 1, 1, 2, 5]) | rng.pick(&[0, 0, 0x8000_0000]);
            t.w32(array, n);
            if rng.below(4) == 0 {
                // a block word pointing at the inline slot itself
                t.w32(array + 4, array + 4);
            }
            let array = rng.pick(&[0, array, array, array]);
            t.ret(h2::CLUSTER_CREATE, array);
            // what the third create returns (the grid stub sets it)
            let other = rng.next();
            let third = rng.pick(&[0, other, other]);
            t.w32(third_cell, third);
            let mi = random_block(&mut t, &mut rng, 0x20);
            t.ret(h2::MESH_INSTANCE, mi);
            let frame = t.ebp_for(0) - 0x20;
            t.check_framed_conv(
                &format!("{w} shaders"),
                0x006c_2520,
                Conv::Thiscall(this),
                &[],
                frame,
                &[(0, 0x20)],
                |g| u(h2::distortion_on_shaders(g, frame, this)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

// ---- continuation round 6, batch 3 (ffx_glue_other::h4_h3)

/// A recording stub at `sym` (`this`, `n` words, `pops`) that first
/// ORs each cell's word into `[ecx + off]` (a cell of 0 leaves the word
/// as the caller set it).
fn h3_fill_stub(t: &mut Twins, sym: u32, n: u32, pops: u32, fills: &[(u32, u8)]) {
    let log = t.stub_code(sym, true, n, pops, false, &[]);
    let mut c = Vec::new();
    for &(cell, off) in fills {
        c.push(0xa1);
        c.extend(cell.to_le_bytes()); // mov eax, [cell]
        c.extend([0x09, 0x41, off]); // or [ecx+off], eax
    }
    let at = t.cave_next;
    c.push(0xe9);
    c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // jmp log
    let code = t.code(&c);
    t.jump(sym, code);
}

/// A random C string (0..12 bytes, sometimes empty) of the bytes in
/// `alphabet`.
fn h3_text(t: &mut Twins, rng: &mut Rng, alphabet: &[u8]) -> u32 {
    let len = rng.pick(&[0, 1, 5, 12]) as usize;
    let mut text: Vec<u8> = (0..len).map(|_| rng.pick(alphabet)).collect();
    text.push(0);
    t.alloc(&text)
}

/// A random tree node for the string-keyed lookups: its key `+0x10` (a
/// `std::string`, inline or on the heap), its value `+0x28`.
fn h3_node(t: &mut Twins, rng: &mut Rng) -> u32 {
    let node = random_block(t, rng, 0x30);
    let heap = random_block(t, rng, 0x20);
    let cap = rng.pick(&[0xf, 0x10, 0x1f]);
    t.w32(node + 0x24, cap);
    if cap >= 0x10 {
        t.w32(node + 0x10, heap);
    }
    let len = rng.below(0x20);
    t.w32(node + 0x20, len);
    node
}

/// The loaded-character table's lookup and insertion, and the Iggy
/// callback's name lookup.
#[test]
fn h3_string_lookups() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a301);
        let (cap, size) = (t.zeros(4), t.zeros(4));
        h3_fill_stub(
            &mut t,
            h3::STRING_ASSIGN,
            2,
            2,
            &[(cap, 0x14), (size, 0x10)],
        );
        t.stub_at(h3::TABLE_FIND, true, 1, 1, &[]);
        t.stub_at(h3::NAME_MAP_FIND, true, 1, 1, &[]);
        t.stub_at(h3::STRING_COMPARE, true, 4, 4, &[]);
        t.stub(h3::OPERATOR_DELETE, 1, false);
        t.stub(h3::NAME_FOUND, 2, false);
        let tags = [
            h3::STRING_ASSIGN,
            h3::TABLE_FIND,
            h3::NAME_MAP_FIND,
            h3::STRING_COMPARE,
            h3::OPERATOR_DELETE,
            h3::NAME_FOUND,
        ];
        let map_head = t.rebase(h3::NAME_MAP.sym);
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let c = rng.pick(&[0, 0, 0x10, 0x40]);
            t.w32(cap, c);
            let s = rng.below(0x40);
            t.w32(size, s);
            let r = rng.pick(&[0, 1, 0xffff_ffff, 0x8000_0000, 7]);
            t.ret(h3::STRING_COMPARE, r);
            let text = h3_text(&mut t, &mut rng, b"abc/");
            // the table: its head node at +0
            let head = random_block(&mut t, &mut rng, 0x30);
            let this = random_block(&mut t, &mut rng, 8);
            t.w32(this, head);
            let node = h3_node(&mut t, &mut rng);
            let found = rng.pick(&[head, node, node]);
            t.ret(h3::TABLE_FIND, found);
            let frame = t.ebp_for(1) - h3::GET_FRAME - 0x10;
            t.check_framed_conv(
                &format!("{w} get"),
                0x006a_2110,
                Conv::Thiscall(this),
                &[text],
                frame,
                &[(0, h3::GET_FRAME as usize)],
                |g| u(h3::get_prototype(g, frame, this, text)),
            );
            // the Iggy lookup in the global map
            t.w32(map_head, head);
            let found = rng.pick(&[head, node, node]);
            t.ret(h3::NAME_MAP_FIND, found);
            let args = random_block(&mut t, &mut rng, 0x10);
            t.w32(args + 8, text);
            let a = rng.next();
            let frame = t.ebp_for(2) - h3::LOOKUP_FRAME - 0x10;
            t.check_framed(
                &format!("{w} iggy"),
                0x006e_9590,
                &[a, args],
                frame,
                &[(0, h3::LOOKUP_FRAME as usize)],
                |g| u(h3::iggy_lookup(g, frame, a, args)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The loaded-character table's insertion (the lookup a stub).
#[test]
fn h3_insert_prototype() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a302);
        let (cap, size) = (t.zeros(4), t.zeros(4));
        h3_fill_stub(
            &mut t,
            h3::STRING_ASSIGN,
            2,
            2,
            &[(cap, 0x14), (size, 0x10)],
        );
        t.stub_at(h3::GET_PROTOTYPE, true, 1, 1, &[]);
        t.stub(h3::MAKE_ENTRY, 3, false);
        t.stub_at(h3::NEW_NODE, true, 1, 1, &[]);
        t.stub_at(h3::INSERT_NODE, true, 4, 4, &[]);
        t.stub(h3::OPERATOR_DELETE, 1, false);
        let tags = [
            h3::STRING_ASSIGN,
            h3::GET_PROTOTYPE,
            h3::MAKE_ENTRY,
            h3::NEW_NODE,
            h3::INSERT_NODE,
            h3::OPERATOR_DELETE,
        ];
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let c = rng.pick(&[0, 0, 0x10, 0x40]);
            t.w32(cap, c);
            let s = rng.below(0x40);
            t.w32(size, s);
            let other = rng.next();
            let r = rng.pick(&[0, 0, other]);
            t.ret(h3::GET_PROTOTYPE, r);
            let text = h3_text(&mut t, &mut rng, b"abc/");
            let this = random_block(&mut t, &mut rng, 8);
            let proto = rng.next();
            let frame = t.ebp_for(2) - h3::INSERT_FRAME - 0x10;
            // the entry's capacity: junk the stub leaves
            let junk: Vec<u8> = (0..h3::INSERT_FRAME).map(|_| rng.next() as u8).collect();
            t.write(frame, &junk);
            let ec = rng.pick(&[0xf, 0x10, 0x7fff]);
            t.w32(frame + 0x1c, ec);
            t.check_framed_conv(
                &format!("{w} insert"),
                0x006a_28b0,
                Conv::Thiscall(this),
                &[text, proto],
                frame,
                &[],
                |g| u(h3::insert_prototype(g, frame, this, text, proto)),
            );
            let (mut x, mut y) = (
                vec![0; h3::INSERT_FRAME as usize],
                vec![0; h3::INSERT_FRAME as usize],
            );
            t.a.read_bytes(frame, &mut x);
            t.b.read_bytes(frame, &mut y);
            assert_eq!(x, y, "{w}: frame");
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// A random cluster path: pieces of the patterns the cache looks for.
fn h3_path(t: &mut Twins, rng: &mut Rng, costume: &[u8]) -> u32 {
    let pieces: [&[u8]; 9] = [
        b".dae",
        b"chr",
        b"/chr/",
        b"PhyreChrLitShader",
        b"PhyreDefaultLitShader",
        b"/Shaders/",
        b"x",
        b"map/",
        costume,
    ];
    let mut text = Vec::new();
    for _ in 0..rng.below(5) {
        text.extend_from_slice(rng.pick(&pieces));
    }
    text.push(0);
    t.alloc(&text)
}

/// The image's C string at `sym`.
fn h3_image_string(t: &Twins, sym: u32) -> Vec<u8> {
    let mut at: u32 = Mem::read(&t.a, t.rebase(sym));
    let mut out = Vec::new();
    loop {
        let b: u8 = Mem::read(&t.a, at);
        if b == 0 {
            return out;
        }
        out.push(b);
        at += 1;
    }
}

/// `strstr` for the tests: a cdecl routine in the cave.
fn h3_strstr(t: &mut Twins) -> u32 {
    // push esi; push edi; mov esi,[esp+0xc]; mov edi,[esp+0x10]
    // outer: xor ecx,ecx
    // inner: mov al,[edi+ecx]; test al,al; jz found; cmp al,[esi+ecx]; jne next; inc ecx; jmp inner
    // next: cmp byte [esi],0; jz none; inc esi; jmp outer
    // found: mov eax,esi; pop edi; pop esi; ret
    // none: xor eax,eax; pop edi; pop esi; ret
    let code: [u8; 46] = [
        0x56, 0x57, 0x8b, 0x74, 0x24, 0x0c, 0x8b, 0x7c, 0x24, 0x10, // 0..10
        0x31, 0xc9, // 10 outer
        0x8a, 0x04, 0x0f, // 12 inner: mov al,[edi+ecx]
        0x84, 0xc0, // 15
        0x74, 0x10, // 17 jz found (19 + 16 = 35)
        0x3a, 0x04, 0x0e, // 19 cmp al,[esi+ecx]
        0x75, 0x03, // 22 jne next (+3 -> 27)
        0x41, // 24 inc ecx
        0xeb, 0xf1, // 25 jmp inner (27 - 15 = 12)
        0x80, 0x3e, 0x00, // 27 next: cmp byte [esi],0
        0x74, 0x08, // 30 jz none (32 + 8 = 40)
        0x46, // 32 inc esi
        0xeb, 0xe7, // 33 jmp outer (35 - 25 = 10)
        0x89, 0xf0, // 35 found: mov eax,esi
        0x5f, 0x5e, 0xc3, // 37
        0x31, 0xc0, 0x5f, 0x5e, 0xc3, // 40 none
        0x90, // pad
    ];
    t.code(&code)
}

/// A balanced tree of `nodes` (keys in order) under `head` (the nil node);
/// returns its root.
fn h3_build(t: &mut Twins, nodes: &[u32], keys: &[u32], head: u32, rng: &mut Rng) -> u32 {
    if nodes.is_empty() {
        return head;
    }
    let m = nodes.len() / 2;
    let node = nodes[m];
    let l = h3_build(t, &nodes[..m], &keys[..m], head, rng);
    let r = h3_build(t, &nodes[m + 1..], &keys[m + 1..], head, rng);
    t.w32(node, l);
    t.w32(node + 8, r);
    t.w8(node + 0xd, 0);
    t.w32(node + 0x10, keys[m]);
    let rec = random_block(t, rng, 8);
    t.w32(node + 0x14, rec);
    node
}

/// The character resource cache's `onLoadPCluster` and the cluster
/// manager's `loadPCluster`.
#[test]
fn h3_cluster_loads() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a303);
        let strstr = h3_strstr(&mut t);
        let costume = h3_image_string(&t, h3::COSTUME_PATTERN.sym);
        let assign_cell = t.zeros(4);
        h3_fill_stub(&mut t, h3::PSTRING_ASSIGN, 1, 1, &[(assign_cell, 0)]);
        t.stub(h3::PHYRE_MALLOC, 2, false);
        t.stub(h3::PHYRE_FREE, 1, false);
        t.stub_at(h3::APPEND_PAIR, true, 2, 2, &[]);
        let wait = t.stub_code(0x0bad_a3c0, false, 2, 2, false, &[]);
        let release = t.stub_code(0x0bad_a3c1, false, 1, 1, false, &[]);
        let tags = [
            h3::PSTRING_ASSIGN,
            h3::PHYRE_MALLOC,
            h3::PHYRE_FREE,
            h3::APPEND_PAIR,
            0x0bad_a3c0,
            0x0bad_a3c1,
        ];
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(h3::STRSTR.sym, strstr);
            let word = rng.next() & !1 | rng.below(2);
            t.w32(assign_cell, word);
            let path = h3_path(&mut t, &mut rng, &costume);
            let path = if rng.below(20) == 0 { 0 } else { path };
            // the (hash, string) array
            let n = rng.below(4);
            let block = random_block(&mut t, &mut rng, 8 * n.max(1));
            let hash = rng.next();
            if n > 0 && rng.below(2) == 0 {
                let k = rng.below(n);
                t.w32(block + 8 * k, hash);
            }
            let this = random_block(&mut t, &mut rng, 0x10);
            let first = rng.pick(&[0, 0, 5]);
            t.w32(this, first);
            t.w32(this + 8, n | rng.pick(&[0, 0x8000_0000]));
            t.w32(this + 0xc, block);
            let record = random_block(&mut t, &mut rng, 0x18);
            let frame = t.ebp_for(3) - 0x18;
            if path != 0 {
                t.check_framed_conv(
                    &format!("{w} on load"),
                    0x0069_bb80,
                    Conv::Thiscall(this),
                    &[path, hash, record],
                    frame,
                    &[(0, 8)],
                    |g| u(h3::on_load_pcluster(g, frame, this, path, hash, record)),
                );
            }
        }
        // loadPCluster, with onLoadPCluster a stub
        t.stub(h3::FIOS_UNIFY, 3, false);
        t.stub(h3::DJB_HASH, 1, false);
        t.stub_at(h3::TAKE_RECORD, true, 0, 0, &[]);
        t.stub_at(h3::GIVE_RECORD, true, 1, 1, &[]);
        // the cache stub changes what the second hash returns
        let on_load = t.stub_code(h3::ON_LOAD_PCLUSTER, true, 3, 3, false, &[]);
        let hash2_cell = t.zeros(4);
        let djb_cell = t.cells[&h3::DJB_HASH];
        let mut c = vec![0xa1];
        c.extend(hash2_cell.to_le_bytes()); // mov eax, [hash2]
        c.push(0xa3);
        c.extend(djb_cell.to_le_bytes()); // mov [djb], eax
        let at = t.cave_next;
        c.push(0xe9);
        c.extend(on_load.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // jmp on_load
        let set = t.code(&c);
        t.jump(h3::ON_LOAD_PCLUSTER, set);
        t.stub(h3::LOAD_ASSET_FILE, 3, false);
        t.stub_at(h3::NEW_ENTRY, true, 1, 1, &[]);
        t.stub_at(h3::INSERT_ENTRY, true, 4, 4, &[]);
        let tags = [
            h3::FIOS_UNIFY,
            h3::DJB_HASH,
            h3::TAKE_RECORD,
            h3::GIVE_RECORD,
            h3::ON_LOAD_PCLUSTER,
            h3::LOAD_ASSET_FILE,
            h3::NEW_ENTRY,
            h3::INSERT_ENTRY,
            0x0bad_a3c0,
            0x0bad_a3c1,
        ];
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(h3::STRSTR.sym, strstr);
            t.g32(h3::WAIT_FOR_SINGLE_OBJECT.sym, wait);
            t.g32(h3::RELEASE_MUTEX.sym, release);
            let hash = rng.below(16) * 0x10;
            t.ret(h3::DJB_HASH, hash);
            let hash2 = rng.next();
            t.w32(hash2_cell, hash2);
            // a tree of up to 7 nodes keyed 0x00..0xf0 (in order)
            let head = random_block(&mut t, &mut rng, 0x18);
            t.w8(head + 0xd, 1);
            let n = rng.below(8) as usize;
            let mut keys: Vec<u32> = (0..n).map(|_| rng.below(16) * 0x10).collect();
            keys.sort_unstable();
            let nodes: Vec<u32> = (0..n)
                .map(|_| random_block(&mut t, &mut rng, 0x18))
                .collect();
            let root = h3_build(&mut t, &nodes, &keys, head, &mut rng);
            t.w32(head + 4, root);
            let mutex_at = random_block(&mut t, &mut rng, 4);
            let this = random_block(&mut t, &mut rng, 0x20);
            t.w32(this + 0x10, head);
            t.w32(this + 0x18, mutex_at);
            let record = random_block(&mut t, &mut rng, 0x18);
            t.ret(h3::TAKE_RECORD, record);
            let r = rng.pick(&[0, 0, 1]);
            t.ret(h3::LOAD_ASSET_FILE, r);
            let name_text = h3_text(&mut t, &mut rng, b"ab");
            let frame = t.ebp_for(1) - 0x10c;
            // the stub doesn't unify: put a path into both frames
            let path = h3_path(&mut t, &mut rng, &costume);
            let mut bytes = vec![0u8; 0x100];
            t.a.read_bytes(path, &mut bytes[..0x40]);
            t.write(frame + 8, &bytes);
            t.check_framed_conv(
                &format!("{w} load"),
                0x0069_b940,
                Conv::Thiscall(this),
                &[name_text],
                frame,
                &[(0, 8)],
                |g| u(h3::load_pcluster(g, frame, this, name_text)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

// ---- continuation round 6, batch 4 (ffx_glue_other::h4_h4)

/// [`h3_fill_stub`] storing (`mov`) instead of OR-ing: for a callee that
/// sets the words whatever they held.
fn h4_set_stub(t: &mut Twins, sym: u32, n: u32, pops: u32, fills: &[(u32, u8)]) {
    let log = t.stub_code(sym, true, n, pops, false, &[]);
    let mut c = Vec::new();
    for &(cell, off) in fills {
        c.push(0xa1);
        c.extend(cell.to_le_bytes()); // mov eax, [cell]
        c.extend([0x89, 0x41, off]); // mov [ecx+off], eax
    }
    let at = t.cave_next;
    c.push(0xe9);
    c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // jmp log
    let code = t.code(&c);
    t.jump(sym, code);
}

/// A recording stub at `sym` whose result alternates between its cell
/// and a second cell (returned): each call first swaps the two.
fn h4_swap_stub(t: &mut Twins, sym: u32, n: u32, pops: u32) -> u32 {
    let log = t.stub_code(sym, true, n, pops, false, &[]);
    let cell = t.cells[&sym];
    let next = t.zeros(4);
    let mut c = vec![0xa1];
    c.extend(next.to_le_bytes()); // mov eax, [next]
    c.extend([0x87, 0x05]);
    c.extend(cell.to_le_bytes()); // xchg [cell], eax
    c.push(0xa3);
    c.extend(next.to_le_bytes()); // mov [next], eax
    let at = t.cave_next;
    c.push(0xe9);
    c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // jmp log
    let code = t.code(&c);
    t.jump(sym, code);
    next
}

/// The cursor seek as a stub that fills the cursor's base `+0`, hit
/// `+0xc` and offset `+0x10` from three cells (returned).
fn h4_seek_stub(t: &mut Twins) -> [u32; 3] {
    let cells = [t.zeros(4), t.zeros(4), t.zeros(4)];
    h3_fill_stub(
        t,
        h2::CURSOR_SEEK,
        0,
        0,
        &[(cells[0], 0), (cells[1], 0xc), (cells[2], 0x10)],
    );
    cells
}

/// Random cursor results.
fn h4_seek_results(t: &mut Twins, rng: &mut Rng, cells: [u32; 3]) {
    let base = rng.next();
    let other = rng.next();
    let off = rng.pick(&[base.wrapping_neg(), 0x40, other]);
    let hit = rng.pick(&[0, 1, 1, 1]);
    t.w32(cells[0], base);
    t.w32(cells[1], hit);
    t.w32(cells[2], off);
}

/// A shader cluster whose list (`+0x1c`) is empty or not.
fn h4_cluster(t: &mut Twins, rng: &mut Rng) -> u32 {
    let cluster = random_block(t, rng, 0x24);
    let list = cluster + 0x1c;
    let other = rng.next();
    let first = rng.pick(&[list, other]);
    t.w32(list, first);
    cluster
}

/// A material list for the create stub: a count with or without its top
/// bit, its block word sometimes pointing at the inline slot.
fn h4_list(t: &mut Twins, rng: &mut Rng) -> u32 {
    let array = random_block(t, rng, 0x10);
    let n = rng.pick(&[0, 1, 1, 2, 5]) | rng.pick(&[0, 0, 0x8000_0000]);
    t.w32(array, n);
    if rng.below(4) == 0 {
        t.w32(array + 4, array + 4);
    }
    array
}

const H4_TAGS: [u32; 9] = [
    h2::CURSOR_SEEK,
    h2::CREATE_MATERIAL,
    h2::CLUSTER_CREATE,
    h2::PHYRE_FREE,
    h4::FULLSCREEN_MESH,
    h4::FIND_PARAMETER,
    h4::SET_PARAM48,
    h2::TEXTURE_REGISTER,
    h4::BLUR_PASS_INIT,
];

fn h4_stubs(t: &mut Twins) -> [u32; 3] {
    let cells = h4_seek_stub(t);
    t.stub_at(h2::CREATE_MATERIAL, true, 3, 3, &[]);
    t.stub_at(h2::CLUSTER_CREATE, true, 3, 3, &[]);
    t.stub(h2::PHYRE_FREE, 1, false);
    t.stub(h4::FULLSCREEN_MESH, 1, false);
    t.stub_at(h4::FIND_PARAMETER, true, 1, 1, &[]);
    t.stub_at(h4::SET_PARAM48, true, 4, 4, &[]);
    t.stub_at(h2::TEXTURE_REGISTER, true, 0, 0, &[]);
    cells
}

/// The blur pass's setup.
#[test]
fn h4_blur_pass_init() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a401);
        let cells = h4_stubs(&mut t);
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &H4_TAGS[..8]);
            h4_seek_results(&mut t, &mut rng, cells);
            let mesh = random_block(&mut t, &mut rng, 0x20);
            t.ret(h4::FULLSCREEN_MESH, mesh);
            let list = h4_list(&mut t, &mut rng);
            t.ret(h2::CLUSTER_CREATE, list);
            let this = random_block(&mut t, &mut rng, 0x60);
            let cluster = h4_cluster(&mut t, &mut rng);
            let size = |rng: &mut Rng| {
                let r = rng.next();
                rng.pick(&[1280, 720, 0, 0xffff_fff0, r])
            };
            let a = [
                rng.next(),
                cluster,
                rng.next(),
                size(&mut rng),
                size(&mut rng),
            ];
            let frame = t.ebp_for(5) - 0x20;
            t.check_framed_conv(
                &format!("{w} pass"),
                0x006c_40e0,
                Conv::Thiscall(this),
                &a,
                frame,
                &[(0, 0x20)],
                |g| u(h4::blur_pass_init(g, frame, this, a)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The extended blur's post-initialisation (the blur pass a stub).
#[test]
fn h4_blur_ex_post_initialize() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a402);
        let cells = h4_stubs(&mut t);
        t.stub(h4::BLUR_EFFECT, 0, false);
        t.stub_at(h4::BLUR_PASS_INIT, true, 5, 5, &[]);
        let mut tags = H4_TAGS.to_vec();
        tags.push(h4::BLUR_EFFECT);
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            h4_seek_results(&mut t, &mut rng, cells);
            let blur = random_block(&mut t, &mut rng, 0x24);
            let cluster = h4_cluster(&mut t, &mut rng);
            // a cluster, unless BlurEffect has none at all (the original
            // seeks a null cluster then)
            t.w32(blur + 0x20, cluster);
            let other = rng.next();
            let c0 = rng.pick(&[0, other, other]);
            t.w32(blur, c0);
            t.ret(h4::BLUR_EFFECT, blur);
            let mesh = random_block(&mut t, &mut rng, 0x20);
            t.ret(h4::FULLSCREEN_MESH, mesh);
            let list = h4_list(&mut t, &mut rng);
            t.ret(h2::CLUSTER_CREATE, list);
            let this = random_block(&mut t, &mut rng, 0x38);
            let done = rng.pick(&[0, 0, 0, 7]);
            t.w32(this + 0x10, done);
            let frame = t.ebp_for(0) - 0x24;
            t.check_framed_conv(
                &format!("{w} post"),
                0x006c_4b60,
                Conv::Thiscall(this),
                &[],
                frame,
                &[(0, 0x24)],
                |g| u(h4::blur_ex_post_initialize(g, frame, this)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// `BlurEffect`'s shader-cluster callback (the radial effects stubs; the
/// render-target stub returns a fresh target or null).
#[test]
fn h4_blur_effect_on_shaders() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a403);
        let cells = h4_stubs(&mut t);
        // the list first, then the textures alternate with it
        let next = h4_swap_stub(&mut t, h2::CLUSTER_CREATE, 3, 3);
        t.stub(h4::GET_APPLICATION, 0, false);
        t.stub(h4::FIXUP_CLUSTERS, 2, false);
        t.stub_at(h2::SET_DIMENSIONS, true, 4, 4, &[]);
        t.stub_at(h2::UPDATE_MAX_LEVEL, true, 0, 0, &[]);
        t.stub_at(h4::CREATE_TARGET, true, 1, 1, &[]);
        t.stub_at(h4::RADIAL_BLUR_INIT, true, 5, 5, &[]);
        t.stub_at(h4::RADIAL_LINE_INIT, true, 6, 6, &[]);
        let mut tags = H4_TAGS[..8].to_vec();
        tags.extend([
            h4::GET_APPLICATION,
            h4::FIXUP_CLUSTERS,
            h2::SET_DIMENSIONS,
            h2::UPDATE_MAX_LEVEL,
            h4::CREATE_TARGET,
            h4::RADIAL_BLUR_INIT,
            h4::RADIAL_LINE_INIT,
        ]);
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            h4_seek_results(&mut t, &mut rng, cells);
            let other = rng.next();
            let m = rng.pick(&[0, other, other, other, other]);
            t.ret(h2::CREATE_MATERIAL, m);
            let mesh = random_block(&mut t, &mut rng, 0x20);
            t.ret(h4::FULLSCREEN_MESH, mesh);
            // the list (also every second texture), then a texture or null
            let list = h4_list(&mut t, &mut rng);
            let list = rng.pick(&[0, list, list, list, list, list]);
            t.w32(next, list);
            let texture = random_block(&mut t, &mut rng, 0x18);
            let texture = rng.pick(&[0, texture, texture, texture]);
            t.ret(h2::CLUSTER_CREATE, texture);
            let target = random_block(&mut t, &mut rng, 0x10);
            let tex = rng.next();
            t.w32(target, tex);
            let flag = rng.pick(&[0, 0, 1]);
            t.w8(target + 4, flag);
            let target = rng.pick(&[0, target, target, target, target, target, target]);
            t.ret(h4::CREATE_TARGET, target);
            let r = rng.pick(&[0, 0, 0x13]);
            t.ret(h4::RADIAL_BLUR_INIT, r);
            let this = random_block(&mut t, &mut rng, 0x40);
            let cluster = h4_cluster(&mut t, &mut rng);
            t.w32(this + 0x20, cluster);
            let frame = t.ebp_for(0) - 0x24;
            t.check_framed_conv(
                &format!("{w} effect"),
                0x006c_35d0,
                Conv::Thiscall(this),
                &[],
                frame,
                &[(0, 0x24)],
                |g| u(h4::blur_effect_on_shaders(g, frame, this)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The radial blur's and radial lines' setup.
#[test]
fn h4_radial_init() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a404);
        let cells = h4_stubs(&mut t);
        // the radial lines' cursor: a constructor that fills the cursor
        let line_cells = [t.zeros(4), t.zeros(4), t.zeros(4)];
        h4_set_stub(
            &mut t,
            h4::CURSOR_INIT,
            1,
            1,
            &[
                (line_cells[0], 0),
                (line_cells[1], 0xc),
                (line_cells[2], 0x10),
            ],
        );
        t.stub_at(h2::SET_DIMENSIONS, true, 4, 4, &[]);
        t.stub_at(h2::UPDATE_MAX_LEVEL, true, 0, 0, &[]);
        t.stub_at(h4::CREATE_TARGET, true, 1, 1, &[]);
        t.stub_at(h2::SET_WORD, true, 2, 2, &[]);
        t.stub_at(h4::RADIAL_SET_PARAM54, true, 4, 4, &[]);
        t.stub_at(h4::SET_MIN_FILTER, true, 1, 1, &[]);
        t.stub(h2::POST_PROCESS_MANAGER, 0, false);
        t.stub_at(h4::SET_TEXTURE, true, 2, 2, &[]);
        t.stub_at(h4::RADIAL_LINE_SETUP, true, 0, 0, &[]);
        t.stub_at(h4::CREATE_IN_ROOT, true, 1, 1, &[]);
        t.stub_at(h4::SET_LINE_STREAMS, true, 3, 3, &[]);
        t.stub_at(h4::LINE_INSTANCE, true, 1, 1, &[]);
        t.stub_at(h2::MESH_INSTANCE, true, 1, 1, &[]);
        let mut tags = H4_TAGS[..8].to_vec();
        tags.extend([
            h4::CURSOR_INIT,
            h2::SET_DIMENSIONS,
            h2::UPDATE_MAX_LEVEL,
            h4::CREATE_TARGET,
            h2::SET_WORD,
            h4::RADIAL_SET_PARAM54,
            h4::SET_MIN_FILTER,
            h2::POST_PROCESS_MANAGER,
            h4::SET_TEXTURE,
            h4::RADIAL_LINE_SETUP,
            h4::CREATE_IN_ROOT,
            h4::SET_LINE_STREAMS,
            h4::LINE_INSTANCE,
            h2::MESH_INSTANCE,
        ]);
        let manager = t.zeros(0x1_2400);
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            h4_seek_results(&mut t, &mut rng, cells);
            h4_seek_results(&mut t, &mut rng, line_cells);
            let other = rng.next();
            let m = rng.pick(&[0, other, other, other, other]);
            let material = random_block(&mut t, &mut rng, 8);
            let m = if m == 0 { 0 } else { material };
            t.ret(h2::CREATE_MATERIAL, m);
            let mesh = random_block(&mut t, &mut rng, 0x20);
            let mesh = rng.pick(&[0, mesh, mesh, mesh, mesh, mesh]);
            t.ret(h4::FULLSCREEN_MESH, mesh);
            let list = h4_list(&mut t, &mut rng);
            let list = rng.pick(&[0, list, list, list, list, list, list]);
            t.ret(h2::CLUSTER_CREATE, list);
            let target = random_block(&mut t, &mut rng, 0x10);
            let target = rng.pick(&[0, target, target, target, target, target]);
            t.ret(h4::CREATE_TARGET, target);
            let r = rng.pick(&[0, 0, 0, 0, 0, 7]);
            t.ret(h2::SET_DIMENSIONS, r);
            let r = rng.pick(&[0, 0, 0, 0, 0, 9, 1]);
            t.ret(h4::RADIAL_SET_PARAM54, r);
            let p = rng.pick(&[0, 0x1234, 0x1234]);
            t.ret(h4::FIND_PARAMETER, p);
            let mi = random_block(&mut t, &mut rng, 0x20);
            t.ret(h2::MESH_INSTANCE, mi);
            t.ret(h2::POST_PROCESS_MANAGER, manager);
            let on = rng.pick(&[0, 1, 1, 1]);
            t.w8(manager + 0x1_23b4, on);
            let size = |rng: &mut Rng| {
                let r = rng.next();
                rng.pick(&[1280, 720, 0, 0xffff_fff0, r])
            };
            let cluster = h4_cluster(&mut t, &mut rng);
            let cluster = rng.pick(&[0, cluster, cluster, cluster, cluster]);
            let a = [
                cluster,
                rng.next(),
                rng.next(),
                size(&mut rng),
                size(&mut rng),
            ];
            let this = random_block(&mut t, &mut rng, 0x80);
            let frame = t.ebp_for(5) - 0x20;
            t.check_framed_conv(
                &format!("{w} blur"),
                0x0070_3230,
                Conv::Thiscall(this),
                &a,
                frame,
                &[(0, 0x20)],
                |g| u(h4::radial_blur_init(g, frame, this, a)),
            );
            // the radial lines use the material without a null test
            t.ret(h2::CREATE_MATERIAL, material);
            let b = [a[0], a[1], a[2], a[3], a[4], rng.next()];
            let this = random_block(&mut t, &mut rng, 0x80);
            let frame = t.ebp_for(6) - 0x20;
            t.check_framed_conv(
                &format!("{w} line"),
                0x006f_7c10,
                Conv::Thiscall(this),
                &b,
                frame,
                &[(0, 0x20)],
                |g| u(h4::radial_line_init(g, frame, this, b)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

// ---- continuation round 6, batch 5 (ffx_glue_other::h4_h5)

/// The motion blur's shader setup.
#[test]
fn h5_motion_blur_setup() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a501);
        let cells = h4_stubs(&mut t);
        t.stub(h4::GET_APPLICATION, 0, false);
        t.stub(h4::FIXUP_CLUSTERS, 2, false);
        t.stub_at(h4::SET_TEXTURE, true, 2, 2, &[]);
        t.stub_at(h5::SET_VALUE2, true, 2, 2, &[]);
        t.stub_at(h5::SET_VALUE, true, 2, 2, &[]);
        t.stub_at(h2::SET_WORD, true, 2, 2, &[]);
        // the parameter lookups alternate between two results
        let next_p = h4_swap_stub(&mut t, h4::FIND_PARAMETER, 1, 1);
        let mut tags = H4_TAGS[..8].to_vec();
        tags.extend([
            h4::GET_APPLICATION,
            h4::FIXUP_CLUSTERS,
            h4::SET_TEXTURE,
            h5::SET_VALUE2,
            h5::SET_VALUE,
            h2::SET_WORD,
        ]);
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            h4_seek_results(&mut t, &mut rng, cells);
            let material = random_block(&mut t, &mut rng, 8);
            t.ret(h2::CREATE_MATERIAL, material);
            let p = rng.pick(&[0, 0x77, 0x77]);
            t.ret(h4::FIND_PARAMETER, p);
            let p = rng.pick(&[0, 0x88, 0x88]);
            t.w32(next_p, p);
            let mesh = random_block(&mut t, &mut rng, 0x20);
            t.ret(h4::FULLSCREEN_MESH, mesh);
            let list = h4_list(&mut t, &mut rng);
            let list = rng.pick(&[0, list, list, list]);
            t.ret(h2::CLUSTER_CREATE, list);
            let this = random_block(&mut t, &mut rng, 0x90);
            let cluster = h4_cluster(&mut t, &mut rng);
            t.w32(this + 0x50, cluster);
            let (a, b) = (rng.next(), rng.next());
            let frame = t.ebp_for(2) - 0x20;
            t.check_framed_conv(
                &format!("{w} motion"),
                0x006c_5150,
                Conv::Thiscall(this),
                &[a, b],
                frame,
                &[(0, 0x20)],
                |g| u(h5::motion_blur_setup(g, frame, this, a, b)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The movie player's shader setup.
#[test]
fn h5_fmv_on_shaders() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a502);
        let cells = [t.zeros(4), t.zeros(4), t.zeros(4)];
        h3_fill_stub(
            &mut t,
            h5::CURSOR_SEEK4,
            0,
            0,
            &[(cells[0], 0), (cells[1], 0xc), (cells[2], 0x10)],
        );
        t.stub(h4::GET_APPLICATION, 0, false);
        t.stub(h4::FIXUP_CLUSTERS, 2, false);
        t.stub_at(h5::BIND_PLANE, true, 2, 2, &[]);
        t.stub_at(h5::SET_VALUE2, true, 2, 2, &[]);
        t.stub_at(h2::CLUSTER_CREATE, true, 3, 3, &[]);
        t.stub(h2::PHYRE_FREE, 1, false);
        t.stub_at(h2::SET_WORD, true, 2, 2, &[]);
        t.stub(h4::FULLSCREEN_MESH, 1, false);
        let tags = [
            h5::CURSOR_SEEK4,
            h4::GET_APPLICATION,
            h4::FIXUP_CLUSTERS,
            h4::FIND_PARAMETER,
            h5::BIND_PLANE,
            h5::SET_VALUE2,
            h2::CLUSTER_CREATE,
            h2::PHYRE_FREE,
            h2::SET_WORD,
            h4::FULLSCREEN_MESH,
        ];
        // the find stub's result alternates between its two cells
        let next = h4_swap_stub(&mut t, h4::FIND_PARAMETER, 1, 1);
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let r = rng.pick(&[0, 0, 0, 0, 0, 3]);
            t.ret(h4::FIXUP_CLUSTERS, r);
            // the material: base + offset, a block with its params and buffer
            let material = random_block(&mut t, &mut rng, 8);
            let base = rng.next();
            t.w32(cells[0], base);
            t.w32(cells[2], material.wrapping_sub(base));
            let hit = rng.pick(&[0, 1, 1, 1]);
            t.w32(cells[1], hit);
            let p0 = rng.pick(&[0, 0x55, 0x55]);
            let p1 = rng.pick(&[0, 0x66, 0x66]);
            t.ret(h4::FIND_PARAMETER, p0);
            t.w32(next, p1);
            let r = rng.pick(&[0, 0, 0, 0, 0, 2]);
            t.ret(h5::BIND_PLANE, r);
            let r = rng.pick(&[0, 0, 0, 0, 0, 4]);
            t.ret(h2::SET_WORD, r);
            let mesh = random_block(&mut t, &mut rng, 0x20);
            t.ret(h4::FULLSCREEN_MESH, mesh);
            let list = h4_list(&mut t, &mut rng);
            let list = rng.pick(&[0, list, list, list]);
            t.ret(h2::CLUSTER_CREATE, list);
            let this = random_block(&mut t, &mut rng, 0x700);
            let cluster = h4_cluster(&mut t, &mut rng);
            t.w32(this + 0x360, cluster);
            let frame = t.ebp_for(0) - 0x28;
            t.check_framed_conv(
                &format!("{w} fmv"),
                0x006d_86e0,
                Conv::Thiscall(this),
                &[],
                frame,
                &[(0, 0x28)],
                |g| u(h5::fmv_on_shaders(g, frame, this)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The broken-screen effect's setup (the render target a stub).
#[test]
fn h5_broken_screen_initialize() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a503);
        let cells = h4_stubs(&mut t);
        // the list create alternates: the first list, then the second
        let next = h4_swap_stub(&mut t, h2::CLUSTER_CREATE, 3, 3);
        // and so does the material create
        let next_m = h4_swap_stub(&mut t, h2::CREATE_MATERIAL, 3, 3);
        t.stub(h2::FIOS_UNIFY, 3, false);
        t.stub(h5::LOAD_ASSET_FILE, 3, false);
        t.stub(h4::GET_APPLICATION, 0, false);
        t.stub(h4::FIXUP_CLUSTERS, 2, false);
        t.stub_at(h4::SET_TEXTURE, true, 2, 2, &[]);
        t.stub_at(h2::SET_WORD, true, 2, 2, &[]);
        t.stub_at(h5::BROKEN_CREATE_RT, true, 0, 0, &[]);
        let mut tags = H4_TAGS[..8].to_vec();
        tags.extend([
            h2::FIOS_UNIFY,
            h5::LOAD_ASSET_FILE,
            h4::GET_APPLICATION,
            h4::FIXUP_CLUSTERS,
            h4::SET_TEXTURE,
            h2::SET_WORD,
            h5::BROKEN_CREATE_RT,
        ]);
        let shader = random_block(&mut t, &mut rng, 0x10);
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(h5::BROKEN_SCREEN_SHADER.sym, shader);
            h4_seek_results(&mut t, &mut rng, cells);
            let r = rng.pick(&[0, 0, 0, 0, 0, 1]);
            t.ret(h5::LOAD_ASSET_FILE, r);
            let material = random_block(&mut t, &mut rng, 8);
            let material = rng.pick(&[0, material, material, material, material]);
            t.w32(next_m, material);
            let second = random_block(&mut t, &mut rng, 8);
            t.ret(h2::CREATE_MATERIAL, second);
            let p = rng.pick(&[0, 0x77, 0x77]);
            t.ret(h4::FIND_PARAMETER, p);
            let mesh = random_block(&mut t, &mut rng, 0x20);
            let mesh = rng.pick(&[0, mesh, mesh, mesh, mesh, mesh]);
            t.ret(h4::FULLSCREEN_MESH, mesh);
            // first create: `next`; second: the cell
            let first = h4_list(&mut t, &mut rng);
            let first = rng.pick(&[0, first, first, first, first]);
            t.w32(next, first);
            let second = h4_list(&mut t, &mut rng);
            t.ret(h2::CLUSTER_CREATE, second);
            let this = random_block(&mut t, &mut rng, 0x78);
            let cluster = h4_cluster(&mut t, &mut rng);
            t.w32(this + 0x3c, cluster);
            let a: [u32; 7] = std::array::from_fn(|_| rng.next());
            let frame = t.ebp_for(7) - 0x12c;
            t.check_framed_conv(
                &format!("{w} broken"),
                0x006d_51d0,
                Conv::Thiscall(this),
                &a,
                frame,
                &[(0, 0x28)],
                |g| u(h5::broken_screen_initialize(g, frame, this, a)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}
