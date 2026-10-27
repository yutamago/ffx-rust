//! Continuation round 9 of list `h4-glue-other` (`ffx_glue_other::h4_k*`)
//! against the original: on the image alone and on a snapshot
//! (`FFX_SNAPSHOT`). Results and every byte written must agree. Needs
//! `FFX_GAME_DIR`. The helpers are those of `h4_glue_other_e.rs` (that file
//! is full).
//!
//! Callees that stay original are replaced in both oracles alike by
//! recording stubs (tag, `ecx` for methods, stack words, in a log in
//! memory; the result from a cell), so the diff checks the calls too.
//! Image float constants are overwritten with random floats in both
//! oracles, so a port that reads the wrong one differs.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    dead_code
)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::{Mem, x87};
use ffx_glue_other::{h4_k1 as k1, h4_k2 as k2};
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
    fn call_wide(&mut self, addr: u32, args: &[u32]) -> u32 {
        ffx_oracle::check_width("Host::call_wide", addr, args.len(), host::WIDE_WORDS);
        self.0
            .call(addr, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .eax
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

/// Random words over `n` bytes at a fresh block; its address.
fn random_block(t: &mut Twins, rng: &mut Rng, n: u32) -> u32 {
    let at = t.zeros(n as usize);
    for k in 0..n / 4 {
        let v = rng.next();
        t.w32(at + 4 * k, v);
    }
    at
}

/// An import slot (an `ffx.sym` address) pointing at a recording stub
/// tagged `slot` that logs `n` words.
fn stub_import(t: &mut Twins, slot: u32, n: u32) {
    let addr = t.stub_code(slot, false, n, 0, false, &[]);
    t.g32(slot, addr);
}

// ---- batch 1 (ffx_glue_other::h4_k1)

/// `updateVFXRenderDataLine`: no render data, no buffer, 0..5 lines and
/// counts past `MAX_LINE_COUNT` (the warning), negative counts, random
/// floats (NaN, infinities) for the bounds' compares.
#[test]
fn k1_update_line() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a0c0);
        for (sym, n) in [
            (k1::GET_RENDER_DATA, 2),
            (k1::GET_BUFFER, 4),
            (k1::SET_VISIBLE, 3),
        ] {
            t.stub_at(sym, false, n, 0, &[]);
        }
        stub_import(&mut t, k1::IOB_FUNC.sym, 0);
        stub_import(&mut t, k1::FPRINTF.sym, 2);
        let tags = [
            k1::GET_RENDER_DATA,
            k1::GET_BUFFER,
            k1::SET_VISIBLE,
            k1::IOB_FUNC.sym,
            k1::FPRINTF.sym,
        ];
        let mut lines_seen = 0;
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(
                &mut rng,
                &[k1::LOW_START, k1::HIGH_START, k1::ALPHA_SCALE],
                &[k1::COLOUR_SCALE],
            );
            if rng.below(3) == 0 {
                // ordinary bounds' starts
                t.g32(k1::LOW_START, f32::MAX.to_bits());
                t.g32(k1::HIGH_START, (-f32::MAX).to_bits());
            }
            let n = rng.pick(&[0, 1, 2, 3, 5, 0x9c5, 0x8000_0000, 0xffff_ffff]);
            let room = if n <= 5 { n } else { 0 };
            let src = t.zeros(0x38 * room as usize + 4);
            for k in 0..room * 14 {
                // signed zeros: equal for the compares, not for the bytes
                let f = rng.float();
                let f = rng.pick(&[f, f, f, 0, 0x8000_0000]);
                t.w32(src + 4 * k, f);
            }
            let lines = random_block(&mut t, &mut rng, 0x48);
            t.w32(lines, src);
            t.w32(lines + 4, n);
            let data = random_block(&mut t, &mut rng, 0xb0);
            let segment = random_block(&mut t, &mut rng, 0x20);
            t.w32(data + 0x94, segment);
            let pos = t.zeros(0x18 * room as usize + 4);
            let colour = t.zeros(0x20 * room as usize + 4);
            let index = t.zeros(4 * room as usize + 4);
            t.w32(segment + 0xc, pos);
            t.w32(segment + 0x14, colour);
            t.w32(segment + 0x1c, index);
            t.ret(k1::GET_RENDER_DATA, rng.pick(&[data, data, data, 0]));
            let r = rng.next();
            let buffer = if n > 5 && n < 0x8000_0000 {
                0
            } else {
                rng.pick(&[r, r, r, 0])
            };
            t.ret(k1::GET_BUFFER, buffer);
            let (a, b) = (rng.next(), rng.next());
            t.check(&w, 0x0071_a0c0, &[a, b, lines], |g| {
                u(k1::update_line(g, a, b, lines))
            });
            lines_seen += t.log_entries();
        }
        assert!(lines_seen > 0, "{name}");
    }
}

/// `BlurEffectEx_blur_006c4290`: the centre given or projected (random
/// projected points, w 0, NaN), the scale (0, NaN, infinities), the scene
/// started or not, the pass finished or not, mode 2 or other.
#[test]
fn k1_blur() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_4290);
        let stubs = [
            (k1::VIEW_PROJECTION, 1),
            (k1::ROW_TIMES, 2),
            (k1::SET_PARAM_4C, 4),
            (k1::SET_TEXTURE, 2),
            (k1::SET_PARAM_50, 4),
            (k1::SURFACE_CTOR, 2),
            (k1::SET_COLOR_TARGET, 2),
            (k1::BEGIN_SCENE, 1),
            (k1::RENDER_MESH_INSTANCE, 4),
            (k1::END_SCENE, 1),
            (k1::SURFACE_DTOR, 0),
        ];
        for (sym, n) in stubs {
            t.stub_at(sym, true, n, n, &[]);
        }
        // RowTimes copies the point to the result while `copy` is set
        // (so the point's words show in the centre), then logs
        let copy = t.zeros(4);
        let rest = t.stub_code(k1::ROW_TIMES, true, 2, 2, false, &[]);
        let mut c = vec![0x83, 0x3d];
        c.extend(copy.to_le_bytes());
        c.extend([0x00, 0x74, 0x22]); // cmp dword [copy], 0; je +0x22 (to the jmp)
        c.extend([0x51, 0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x54, 0x24, 0x08]); // push ecx; eax = v; edx = out
        for k in 0..4u8 {
            c.extend([0x8b, 0x48, 4 * k, 0x89, 0x4a, 4 * k]); // mov ecx, [eax+4k]; mov [edx+4k], ecx
        }
        c.push(0x59); // pop ecx
        c.push(0xe9);
        let at = t.cave_next + c.len() as u32 + 4;
        c.extend(rest.wrapping_sub(at).to_le_bytes()); // jmp rest
        let f = t.code(&c);
        t.jump(k1::ROW_TIMES, f);
        let tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        let ebp = t.ebp_for(2);
        let frame = ebp - 0x90;
        let mut scenes = 0;
        for i in 0..500 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(&mut rng, &[], &[k1::HALF]);
            if rng.below(2) == 0 {
                let at = t.rebase(k1::HALF);
                t.write(at, &0.5f64.to_le_bytes());
            }
            t.ret(k1::BEGIN_SCENE, rng.pick(&[0, 0, 0, 1, 5]));
            let copying = rng.below(2);
            t.w32(copy, copying);
            t.ret(k1::END_SCENE, rng.pick(&[0, 0, 5]));
            let scene = rng.next();
            t.g32(k1::PHYRE_SCENE, scene);
            for k in 0..4 {
                let f = if k == 3 && rng.below(5) == 0 {
                    0
                } else {
                    rng.float()
                };
                t.w32(frame + 0x50 + 4 * k, f);
            }
            let this = random_block(&mut t, &mut rng, 0x88);
            for off in (0x5c..0x84).step_by(4) {
                let f = rng.float();
                t.w32(this + off, f);
            }
            let c = rng.float();
            t.w32(this + 0x80, rng.pick(&[0, 0x8000_0000, c, c]));
            t.w32(this + 0x84, rng.pick(&[0, 1, 2, 2, 3]));
            let renderer = random_block(&mut t, &mut rng, 0x140);
            t.w32(this, renderer);
            let instance = random_block(&mut t, &mut rng, 0x20);
            t.w32(this + 0x18, instance);
            let (texture, target) = (rng.next(), rng.next());
            t.check_framed_conv(
                &w,
                0x006c_4290,
                Conv::Thiscall(this),
                &[texture, target],
                frame,
                &[(0x60, 0x18)],
                |g| {
                    let r = k1::blur(g, frame, this, texture, target);
                    // the original's spill to its own argument slot
                    let a: u32 = g.read(this + 0x6c);
                    g.write(ebp + 8, ffx_glue_other::h4_e1::pass(a));
                    u(r)
                },
            );
            scenes += t.log_entries();
        }
        assert!(scenes > 0, "{name}");
    }
}

/// A buffer object: its texture word and byte `+4` (set: no texture).
fn buffer_object(t: &mut Twins, rng: &mut Rng) -> u32 {
    let b = random_block(t, rng, 8);
    t.w8(b + 4, rng.pick(&[0, 0, 1]));
    b
}

/// `BlurEffect_render_006c3970`: inactive, modes 0..3 and negative,
/// buffers with and without textures, one or several materials, each
/// parameter found or not, fade times around the start (NaN, infinities).
#[test]
fn k1_render() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_3970);
        let stubs = [
            (k1::POST_GET_INSTANCE, false, 0, 0),
            (k1::COPY_BUFFER, true, 6, 6),
            (k1::COPY_FLOATS_4, true, 1, 1),
            (k1::COPY_FLOATS_2, true, 1, 1),
            (k1::PASS_SET_TEXTURE, true, 2, 2),
            (k1::PASS_RUN, true, 1, 1),
            (k1::SECOND_FLOATS_4, true, 1, 1),
            (k1::SECOND_FLOATS_2, true, 1, 1),
            (k1::SECOND_RUN, true, 1, 1),
            (k1::FIND_BY_NAME, true, 1, 1),
            (k1::SET_TEXTURE_CHECKED, true, 2, 2),
            (k1::SET_VALUE, true, 2, 2),
            (k1::SURFACE_CTOR, true, 2, 2),
            (k1::SET_COLOR_TARGET, true, 2, 2),
            (k1::BEGIN_SCENE, true, 1, 1),
            (k1::RENDER_MESH_INSTANCE, true, 4, 4),
            (k1::END_SCENE, true, 1, 1),
            (k1::SURFACE_DTOR, true, 0, 0),
        ];
        for (sym, this, n, pops) in stubs {
            t.stub_at(sym, this, n, pops, &[]);
        }
        // FindByName finds the name address masked with `mask`: the
        // names' low bits pick which parameters exist (0x10: ColorBuffer
        // only; 8: it and OutputScale; 4: OutputScale and UvScaleBias), so
        // the value block ends with each one's words
        let mask = t.zeros(4);
        let rest = t.stub_code(k1::FIND_BY_NAME, true, 1, 1, false, &[]);
        let cell = t.cells[&k1::FIND_BY_NAME];
        let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0x23, 0x05]; // mov eax, [esp+4]; and eax, [mask]
        c.extend(mask.to_le_bytes());
        c.push(0xa3);
        c.extend(cell.to_le_bytes()); // mov [cell], eax
        c.push(0xe9);
        let at = t.cave_next + c.len() as u32 + 4;
        c.extend(rest.wrapping_sub(at).to_le_bytes()); // jmp rest
        let f = t.code(&c);
        t.jump(k1::FIND_BY_NAME, f);
        let tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        let frame = t.ebp_for(0) - 0x2c;
        let mut calls = 0;
        for i in 0..500 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(
                &mut rng,
                &[k1::FADE_START],
                &[k1::FADE_OFFSET, k1::FADE_SCALE],
            );
            let manager = random_block(&mut t, &mut rng, 0x90);
            let target = buffer_object(&mut t, &mut rng);
            t.w32(manager + 0x8c, target);
            t.ret(k1::POST_GET_INSTANCE, manager);
            t.w32(mask, rng.pick(&[0, 0x10, 8, 8, 4, 0xffff_ffff]));
            let this = random_block(&mut t, &mut rng, 0x88);
            t.w8(this + 0xc, rng.pick(&[0, 1, 1, 1, 0x80]));
            t.w32(this + 0x10, rng.pick(&[0, 1, 2, 2, 3, 0xffff_ffff]));
            for off in [0, 4, 8] {
                let b = buffer_object(&mut t, &mut rng);
                t.w32(this + off, b);
            }
            let start: u32 = Mem::read(&t.a, t.rebase(k1::FADE_START));
            let f = rng.float();
            t.w32(this + 0x84, rng.pick(&[f, f, start]));
            // the mesh instance's materials: one inline or a list
            let material = random_block(&mut t, &mut rng, 8);
            let materials = t.zeros(0x10);
            let n = rng.pick(&[1, 1, 2, 3]);
            let top = rng.pick(&[0, 0x8000_0000]);
            t.w32(materials, n | top);
            if n == 1 {
                t.w32(materials + 4, material);
            } else {
                let list = t.zeros(16);
                t.w32(list, material);
                t.w32(materials + 4, list);
            }
            let instance = random_block(&mut t, &mut rng, 0x20);
            t.w32(instance + 0x10, materials);
            t.w32(this + 0x28, instance);
            let renderer = random_block(&mut t, &mut rng, 0x140);
            t.w32(this + 0x14, renderer);
            t.check_framed_conv(
                &w,
                0x006c_3970,
                Conv::Thiscall(this),
                &[],
                frame,
                &[(0x10, 0x10)],
                |g| u(k1::render(g, frame, this)),
            );
            calls += t.log_entries();
        }
        assert!(calls > 0, "{name}");
    }
}

/// `fld qword [esp+4]; fld qword [esp+0xc]; call [esp+0x14]; ret`: a
/// two-operand x87 intrinsic (`ST1`, `ST0`) called from a test.
const TRAMPOLINE2: [u8; 13] = [
    0xdd, 0x44, 0x24, 0x04, 0xdd, 0x44, 0x24, 0x0c, 0xff, 0x54, 0x24, 0x14, 0xc3,
];

/// `BlurEffect_update_006c3d40`: inactive, times before and after the
/// turn and around the clamps and the end (NaN, infinities), held or
/// not; `fmod` is the original `_CIfmod` (the oracle's `fprem` loop).
#[test]
fn k1_update() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_3d40);
        let tramp2 = t.code(&TRAMPOLINE2);
        let fmod_thunk = t.rebase(k1::CI_FMOD);
        let f32s = [k1::TURN, k1::CLAMP_F32, k1::LATE_FLOOR, k1::END];
        let f64s = [
            k1::STEP,
            k1::TURN_F64,
            k1::EARLY_SLOPE,
            k1::EARLY_START,
            k1::CLAMP,
            k1::LATE_START,
            k1::LATE_SLOPE_48,
            k1::LATE_SLOPE_68,
            k1::WOBBLE_SCALE_A,
            k1::WOBBLE_SCALE_B,
            k1::WOBBLE_PERIOD_A,
            k1::WOBBLE_PERIOD_B,
            k1::ZOOM_SLOPE_A,
            k1::ZOOM_SLOPE_B,
            k1::ZOOM_BASE_A,
            k1::ZOOM_BASE_B,
        ];
        // the image's own values, for runs near the real thresholds
        let mut real32 = Vec::new();
        for &c in &f32s {
            let at = t.rebase(c);
            real32.push(Mem::read::<u32>(&t.a, at));
        }
        let mut real64 = Vec::new();
        for &c in &f64s {
            let at = t.rebase(c);
            real64.push(Mem::read::<u64>(&t.a, at));
        }
        let mut ran = 0;
        for i in 0..800 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &[]);
            if rng.below(3) == 0 {
                t.constants(&mut rng, &f32s, &f64s);
            } else {
                for (k, &c) in f32s.iter().enumerate() {
                    t.g32(c, real32[k]);
                }
                for (k, &c) in f64s.iter().enumerate() {
                    let at = t.rebase(c);
                    t.write(at, &real64[k].to_le_bytes());
                }
            }
            let this = random_block(&mut t, &mut rng, 0x88);
            let active = rng.pick(&[0, 1, 1, 1]);
            t.w8(this + 0xc, active);
            t.w32(this + 0x10, rng.pick(&[0, 0, 1]));
            let time = match rng.below(4) {
                0 => rng.float(),
                _ => (rng.below(1200) as f32 / 100.0 - 0.5).to_bits(),
            };
            t.w32(this + 0x84, time);
            // equal operands for the compares: the turn or the end at the
            // new time; a fade equal to its clamp (+0x48/+0x68 early) or
            // floor (late, signed zeros)
            let step = f64::from_bits(Mem::read::<u64>(&t.a, t.rebase(k1::STEP)));
            let x = x87::store(x87::add(x87::load(time), step));
            match rng.below(6) {
                0 => t.g32(k1::TURN, x),
                1 => t.g32(k1::END, x),
                2 => {
                    // the fade at the clamp or just above it
                    let s = rng.pick(&[0.5f64, 0.0, -0.0, 0.25]);
                    let start = rng.pick(&[s, s, s + 0.0005]);
                    for (c, v) in [
                        (k1::EARLY_SLOPE, 0.0),
                        (k1::EARLY_START, start),
                        (k1::CLAMP, s),
                    ] {
                        let at = t.rebase(c);
                        t.write(at, &f64::to_le_bytes(v));
                    }
                }
                3 => {
                    for (c, v) in [(k1::LATE_SLOPE_68, 0.0f64), (k1::LATE_START, 0.0)] {
                        let at = t.rebase(c);
                        t.write(at, &v.to_le_bytes());
                    }
                    t.g32(k1::LATE_FLOOR, rng.pick(&[0, 0x8000_0000]));
                }
                _ => {}
            }
            ran += u32::from(active);
            let dt = rng.next();
            t.check_conv(&w, 0x006c_3d40, Conv::Thiscall(this), &[dt], |g| {
                u(k1::update(g, this, &mut |g: &mut G, a: f64, b: f64| {
                    let (a, b) = (a.to_bits(), b.to_bits());
                    let r =
                        g.0.call(
                            tramp2,
                            Conv::Cdecl,
                            &[
                                a as u32,
                                (a >> 32) as u32,
                                b as u32,
                                (b >> 32) as u32,
                                fmod_thunk,
                            ],
                        )
                        .unwrap();
                    let v = r.st0_f64().expect("_CIfmod's ST0");
                    g.pop();
                    v
                }))
            });
        }
        assert!(ran > 0, "{name}");
    }
}

// ---- batch 2 (ffx_glue_other::h4_k2)

/// A bump allocator at the original `sym` (cdecl, logging `n` words):
/// 0x40 bytes from `heap` per call, or 0 while the cell `fail` is set.
/// Returns `fail`.
fn stub_bump(t: &mut Twins, sym: u32, n: u32, heap: u32) -> u32 {
    let next = t.zeros(4);
    t.w32(next, heap);
    let fail = t.zeros(4);
    let rest = t.stub_code(sym, false, n, 0, false, &[]);
    let cell = t.cells[&sym];
    let mut c = vec![0x31, 0xc0, 0x83, 0x3d]; // xor eax, eax; cmp dword [fail], 0
    c.extend(fail.to_le_bytes());
    c.extend([0x00, 0x75, 0x0c]); // jne +12 (store 0)
    c.push(0xa1);
    c.extend(next.to_le_bytes()); // mov eax, [next]
    c.extend([0x83, 0x05]);
    c.extend(next.to_le_bytes());
    c.push(0x40); // add dword [next], 0x40
    c.push(0xa3);
    c.extend(cell.to_le_bytes()); // mov [cell], eax
    c.push(0xe9);
    let at = t.cave_next + c.len() as u32 + 4;
    c.extend(rest.wrapping_sub(at).to_le_bytes()); // jmp rest
    let f = t.code(&c);
    t.jump(sym, f);
    fail
}

/// A string in fresh memory; its address.
fn string(t: &mut Twins, s: &[u8]) -> u32 {
    let mut b = s.to_vec();
    b.push(0);
    t.alloc(&b)
}

/// `TextUtil_appendLine`: lines with an empty slot (the empty string
/// itself, another empty string, owned or not) or none, null slots, no
/// text, failed allocations, the bitmap text there or not.
#[test]
fn k2_append_line() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_5e30);
        let heap = t.zeros(0x4_0000);
        let fail = stub_bump(&mut t, k2::MALLOC, 2, heap);
        let stubs = [
            (k2::FREE, false, 1, 0),
            (k2::PUSH_INTO, true, 2, 2),
            (k2::BUILDER_APPEND, true, 1, 1),
            (k2::SET_TEXT, true, 1, 1),
            (k2::SCROLL_TO_VISIBLE, true, 1, 1),
        ];
        for (sym, this, n, pops) in stubs {
            t.stub_at(sym, this, n, pops, &[]);
        }
        // pushInto counts the new line, as the array's own does
        let rest = t.stub_code(k2::PUSH_INTO, true, 2, 2, false, &[]);
        let mut c = vec![0xff, 0x01, 0xe9]; // inc dword [ecx]; jmp rest
        let at = t.cave_next + c.len() as u32 + 4;
        c.extend(rest.wrapping_sub(at).to_le_bytes());
        let f = t.code(&c);
        t.jump(k2::PUSH_INTO, f);
        let tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        let slot = t.ebp_for(1) + 8;
        let empty = t.rebase(k2::EMPTY);
        let mut found = 0;
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.w32(fail, rng.pick(&[0, 0, 0, 1]));
            let n = rng.pick(&[0, 1, 2, 4]);
            let slots = t.zeros(4 * n as usize + 8);
            let mut has_empty = false;
            for k in 0..n {
                let s = match rng.below(7) {
                    0 => empty,
                    1 => empty | 1,
                    2 => string(&mut t, b""),
                    3 => 0,
                    4 => 1,
                    _ => {
                        let len = rng.below(6) as usize;
                        let text: Vec<u8> = (0..len).map(|_| b'a' + rng.below(3) as u8).collect();
                        string(&mut t, &text)
                    }
                };
                let s = if rng.below(4) == 0 { s | 1 } else { s };
                let line = s & !1;
                has_empty |= line == empty || (line > 1 && Mem::read::<u8>(&t.a, line) == 0);
                t.w32(slots + 4 * k, s);
            }
            // the pushed line's slot
            let pushed = string(&mut t, b"new");
            t.w32(slots + 4 * n, pushed);
            let this = random_block(&mut t, &mut rng, 0x80);
            let top = rng.pick(&[0, 0x8000_0000]);
            t.w32(this + 0x74, n | top);
            t.w32(this + 0x78, slots);
            let r = rng.next();
            t.w32(this + 0x10, rng.pick(&[0, r]));
            t.w32(this + 0x1c, rng.pick(&[0, 1, 5, 0x8000_0000]));
            let len = rng.below(8) as usize;
            let text: Vec<u8> = (0..len).map(|_| b'A' + rng.below(26) as u8).collect();
            let s = string(&mut t, &text);
            let text = rng.pick(&[s, s, s, 0]);
            found += u32::from(has_empty);
            t.check_framed_conv(
                &w,
                0x006f_5e30,
                Conv::Thiscall(this),
                &[text],
                slot,
                &[(0, 4)],
                |g| u(k2::append_line(g, slot, this, text)),
            );
        }
        assert!(found > 0, "{name}");
    }
}

/// `RemapButtonIcon_remapIconPosition`: every button with kind 6 and
/// others, the countdown 0..4, the machine 0..4, quads with signed zeros,
/// NaNs and infinities, the shifts random or the image's.
#[test]
fn k2_remap_icon_position() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_6a50);
        let shifts = [
            k2::SHIFT_3,
            k2::SHIFT_5,
            k2::SHIFT_19,
            k2::SHIFT_3_9,
            k2::SHIFT_13,
            k2::SHIFT_6_5,
            k2::SHIFT_4,
            k2::SHIFT_2_5,
            k2::SHIFT_2,
            k2::SHIFT_0_5,
            k2::SHIFT_MINUS_7,
            k2::SHIFT_MINUS_5,
        ];
        let real: Vec<u64> = shifts
            .iter()
            .map(|&c| Mem::read::<u64>(&t.a, t.rebase(c)))
            .collect();
        for i in 0..1500 {
            let w = format!("{name} {i}");
            if rng.below(2) == 0 {
                t.constants(&mut rng, &[], &shifts);
            } else {
                for (k, &c) in shifts.iter().enumerate() {
                    let at = t.rebase(c);
                    t.write(at, &real[k].to_le_bytes());
                }
            }
            t.g32(k2::REMAP_COUNTDOWN, rng.below(5));
            let this = random_block(&mut t, &mut rng, 0x154);
            t.w32(this + 0x150, rng.below(5));
            let quad = t.zeros(0x80);
            for k in 0..0x20 {
                let f = rng.float();
                t.w32(quad + 4 * k, rng.pick(&[f, f, f, 0, 0x8000_0000]));
            }
            let kind = rng.pick(&[6, 6, 0, 5, 7]);
            let button = rng.pick(&[
                0x20, 0x38, 0x39, 0x40, 0x40, 0x41, 0x42, 0x43, 0x44, 0x46, 0x48, 0x49, 0x4c, 0x45,
                0x50, 0,
            ]);
            t.check_conv(
                &w,
                0x006e_6a50,
                Conv::Thiscall(this),
                &[kind, quad, button],
                |g| u(k2::remap_icon_position(g, this, kind, quad, button)),
            );
        }
    }
}

/// `LoadingScreen_update_006e50a0`: no frames, mesh or shader; every
/// state with and without show flags; times around the hold time and
/// the fades around their ends (equal operands, NaN); frame clocks over
/// several frame times; the constants random or the image's.
#[test]
fn k2_loading_update() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_50a0);
        let stubs = [
            (k2::GET_APPLICATION, false, 0, 0),
            (k2::SLEEP_MS, false, 1, 0),
            (k2::LINEAR_SAMPLER, true, 1, 1),
            (k2::FIND_BY_NAME, true, 1, 1),
            (k2::SET_TEXTURE_CHECKED, true, 2, 2),
        ];
        for (sym, this, n, pops) in stubs {
            t.stub_at(sym, this, n, pops, &[]);
        }
        // SetValue records its value's fourth word in sequence
        let seq = t.zeros(4);
        let rest = t.stub_code(k2::SET_VALUE, true, 2, 2, false, &[]);
        let mut c = vec![0x8b, 0x44, 0x24, 0x08, 0x8b, 0x40, 0x0c, 0x8b, 0x15];
        c.extend(seq.to_le_bytes()); // eax = [[esp+8]+0xc]; edx = [seq]
        c.extend([0x89, 0x02, 0x83, 0x05]); // mov [edx], eax; add dword [seq], 4
        c.extend(seq.to_le_bytes());
        c.extend([0x04, 0xe9]);
        let at = t.cave_next + c.len() as u32 + 4;
        c.extend(rest.wrapping_sub(at).to_le_bytes()); // jmp rest
        let f = t.code(&c);
        t.jump(k2::SET_VALUE, f);
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.push(k2::SET_VALUE);
        // the frame times stay positive (the clock loop subtracts one
        // until the clock is below the other, as long as that takes)
        let f32s = [k2::HOLD, k2::FADE_FULL];
        let f64s = [k2::STEP_LIMIT, k2::FADE_RATE, k2::SCALE_DIVISOR];
        let real32: Vec<u32> = f32s.iter().map(|&c| Mem::read(&t.a, t.rebase(c))).collect();
        let real64: Vec<u64> = f64s.iter().map(|&c| Mem::read(&t.a, t.rebase(c))).collect();
        let ebp = t.ebp_for(1);
        let frame = ebp - 0x14;
        let mut sleeps = 0;
        for i in 0..2000 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let records = t.zeros(0x10);
            t.w32(seq, records);
            if rng.below(4) == 0 {
                t.constants(&mut rng, &f32s, &f64s);
            } else {
                for (k, &c) in f32s.iter().enumerate() {
                    t.g32(c, real32[k]);
                }
                for (k, &c) in f64s.iter().enumerate() {
                    let at = t.rebase(c);
                    t.write(at, &real64[k].to_le_bytes());
                }
            }
            let frame_time = rng.pick(&[0.04f32, 0.04, 0.03, 0.05]);
            t.g32(k2::FRAME_TIME, frame_time.to_bits());
            let ft64 = rng.pick(&[0.04f64, 0.04, 0.03, 0.05]);
            let at = t.rebase(k2::FRAME_TIME_F64);
            t.write(at, &ft64.to_le_bytes());
            let app = t.zeros(0x3a4);
            t.w8(app + 0x3a0, 1);
            t.ret(k2::GET_APPLICATION, app);
            let hold: u32 = Mem::read(&t.a, t.rebase(k2::HOLD));
            let rate: f64 = f64::from_bits(Mem::read(&t.a, t.rebase(k2::FADE_RATE)));
            let this = random_block(&mut t, &mut rng, 0x48);
            t.w8(this, rng.pick(&[0, 0, 1]));
            t.w8(this + 1, rng.pick(&[0, 0, 2]));
            let step_max = rng.pick(&[0.05f32, 0.05, 0.1, -0.5]);
            t.g32(k2::STEP_MAX, step_max.to_bits());
            // bounded steps (the clock loop runs once per frame time)
            let f = rng.float();
            let dt = match rng.below(6) {
                0 if f32::from_bits(f).abs() < 10.0 || f32::from_bits(f).is_nan() => f,
                1 => 0,
                2 => (-(rng.below(500) as f32) / 1000.0).to_bits(),
                _ => (rng.below(120) as f32 / 1000.0).to_bits(),
            };
            // finite clocks only (an infinite one never leaves the loop)
            let f = rng.float();
            let clock = match rng.below(5) {
                0 if f32::from_bits(f).abs() < 10.0 || f32::from_bits(f).is_nan() => f,
                1 => frame_time.to_bits(),
                _ => (rng.below(200) as f32 / 1000.0).to_bits(),
            };
            t.w32(this + 4, clock);
            let time = match rng.below(5) {
                0 => rng.float(),
                1 => hold,
                _ => (1.3 + rng.below(400) as f32 / 1000.0).to_bits(),
            };
            t.w32(this + 8, time);
            let count = rng.pick(&[0, 1, 3, 7]);
            t.w32(this + 0xc, rng.below(count.max(1)));
            t.w32(this + 0x10, rng.pick(&[0, 0, 1, 1, 2, 3, 3, 4]));
            let fade = match rng.below(5) {
                0 => rng.float(),
                1 => store_f32(rate),
                2 => 0,
                _ => (rng.below(1000) as f32 / 1000.0).to_bits(),
            };
            t.w32(this + 0x14, fade);
            t.w32(this + 0x20, count | rng.pick(&[0, 0x8000_0000]));
            let textures = random_block(&mut t, &mut rng, 0x20);
            t.w32(this + 0x24, textures);
            let r = rng.next();
            t.w32(this + 0x2c, rng.pick(&[r, r, r, 0]));
            let r = rng.next();
            t.w32(this + 0x30, rng.pick(&[r, r, r, 0]));
            for off in [0x38, 0x40] {
                let material = random_block(&mut t, &mut rng, 8);
                let materials = t.zeros(0x10);
                let n = rng.pick(&[1, 2]);
                t.w32(materials, n);
                if n == 1 {
                    t.w32(materials + 4, material);
                } else {
                    let list = t.zeros(8);
                    t.w32(list, material);
                    t.w32(materials + 4, list);
                }
                let instance = random_block(&mut t, &mut rng, 0x20);
                t.w32(instance + 0x10, materials);
                t.w32(this + off, instance);
            }
            t.check_framed_conv(
                &w,
                0x006e_50a0,
                Conv::Thiscall(this),
                &[dt],
                frame,
                &[(0, 0x10), (0x1c, 4)],
                |g| {
                    k2::loading_update(g, frame, this, dt);
                    Out::None
                },
            );
            let log = t.log_entries();
            for k in 0..log {
                let tag: u32 = Mem::read(&t.b, t.log + 4 + 64 * k);
                sleeps += u32::from(tag == k2::SLEEP_MS);
            }
        }
        assert!(sleeps > 0, "{name}");
    }
}

/// An `f64` as `fstp dword` stores it.
fn store_f32(x: f64) -> u32 {
    x87::store(x)
}
