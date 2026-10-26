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
use ffx_glue_other::{frames as fr, h4_d1 as d1, h4_e1 as e1, h4_e2 as e2, h4_e3 as e3};
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
        if let Out::U32(v) = out {
            assert_eq!(ra.eax, v, "{what}: eax");
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
