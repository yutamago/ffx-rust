//! List `h4-glue-other`, second agent (h4b): `ffx_glue_other::h4b_*`
//! against the original, on the image alone and on a snapshot
//! (`FFX_SNAPSHOT`). Results and every byte written must agree. Needs
//! `FFX_GAME_DIR`.
//!
//! The harness is `h4_glue_other_e.rs`'s: callees that stay original are
//! replaced in both oracles alike by recording stubs (tag, `ecx` for
//! methods, stack words, in a log in memory; the result from a cell), so
//! the diff checks the calls too. Float callees' stubs return their
//! cell's `f64` in `ST0`.

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

use ffx_bridge::Mem;
use ffx_glue_other::{
    h4_e1 as e1, h4b_a as ba, h4b_b as bb, h4b_c as bc, h4b_d as bd, h4b_e as be, h4b_f as bf,
    h4b_g as bg, h4b_h as bh, h4b_i as bi,
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
    /// One of `v`, or a random word.
    fn pickr(&mut self, v: &[u32]) -> u32 {
        let r = self.next();
        let k = self.below(v.len() as u32 + 1) as usize;
        if k == v.len() { r } else { v[k] }
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

impl Twins {
    /// [`Twins::check_conv`] with `fix` run on both oracles before the
    /// diff (to blank stub log words the originals fill with stack junk).
    fn check_fix(
        &mut self,
        what: &str,
        sym: u32,
        args: &[u32],
        port: impl FnOnce(&mut G) -> Out,
        fix: impl Fn(&mut Oracle),
    ) -> u32 {
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let tramp = self.tramp;
        let out = port(&mut G(&mut self.b, tramp));
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        if let Out::U32(v) = out {
            assert_eq!(ra.eax, v, "{what}: eax");
        }
        fix(&mut self.a);
        fix(&mut self.b);
        self.diff(what);
        ra.eax
    }
}

// ---- batch 1 (ffx_glue_other::h4b_a)

/// A macro text: codes, `7 xx` pen steps, a terminator and zero padding.
fn macro_text(rng: &mut Rng) -> Vec<u8> {
    let n = rng.below(20) as usize;
    let mut v = Vec::new();
    for _ in 0..n {
        match rng.below(10) {
            0 | 1 => {
                v.push(7);
                let b = if rng.below(3) == 0 {
                    rng.next() as u8
                } else {
                    0xe0 + rng.below(16) as u8
                };
                v.push(b);
            }
            2 | 3 => v.push(1 + rng.below(0x2f) as u8),
            4 => v.push(rng.next() as u8),
            _ => v.push(0x30 + rng.below(0xc0) as u8),
        }
    }
    v.extend([0; 8]);
    v
}

/// `TODrawChangeMacro2`: the macro lookup is a hand-written stub (the
/// text after the code byte, so recursion ends; it hashes the ids into
/// a cell), `TOSkipCode` and `TOGetGroup` run as the original.
#[test]
fn ba_change_macro2() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b401);
        for (sym, n) in [
            (ba::MEMSET, 3),
            (ba::UNLOCK_ACHIEVEMENT, 1),
            (ba::GET_FONT_INFO, 2),
            (ba::MAKE_PKT_FONT, 2),
        ] {
            t.stub(sym, n, false);
        }
        let tags = [
            ba::MEMSET,
            ba::UNLOCK_ACHIEVEMENT,
            ba::GET_FONT_INFO,
            ba::MAKE_PKT_FONT,
        ];
        let hash = t.zeros(4);
        let mut c = vec![0x8b, 0x4c, 0x24, 0x04, 0xc1, 0x05];
        c.extend(hash.to_le_bytes());
        c.extend([0x05, 0x31, 0x0d]);
        c.extend(hash.to_le_bytes());
        c.extend([0x8b, 0x44, 0x24, 0x08, 0x85, 0xc0, 0x74, 0x01, 0x40, 0xc3]);
        let lookup = t.code(&c);
        t.jump(ba::GET_CHANGE_MACRO, lookup);
        let win = t.zeros(0xc0);
        let pen = t.zeros(8);
        let frame = t.ebp_for(3);
        let mut steps = 0;
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let text = macro_text(&mut rng);
            let at = t.alloc(&text);
            if text
                .windows(2)
                .any(|p| p[0] == 7 && (0xe0..0xf0).contains(&p[1]))
            {
                steps += 1;
            }
            t.w16(win + 4, rng.next() as u16);
            let count = rng.below(30);
            t.w32(win + 0x9c, count);
            let limit = count + rng.below(12);
            t.w32(win + 0xa0, limit);
            let max = count + rng.below(24);
            t.w32(win + 0xa8, max);
            for off in [0xac, 0xb4] {
                let r = rng.next();
                let v = rng.pick(&[0, 0, 1, r]);
                t.w32(win + off, v);
            }
            let f = rng.float();
            t.w32(pen + 4, f);
            let v = rng.pick(&[0x9e, 0x9e, 0x9d, 0x19e, 0]);
            t.w16(t.rebase(0x0112_ca90), v);
            let v = rng.pick(&[0x32, 0x32, 0x31, 0x33]);
            t.w8(t.rebase(0x0112_cddb), v);
            let v = rng.pick(&[0x47, 0x47, 0x46, 0x48]);
            t.w8(t.rebase(0x0112_cdda), v);
            let v = rng.pick(&[1, 1, 0, 2]);
            t.w8(t.rebase(0x00c5_a03d), v);
            let code = if rng.below(10) == 0 { 0 } else { at - 1 };
            t.check_framed(&w, 0x008b_5960, &[win, code, pen], frame, &[(8, 4)], |g| {
                g.0.write(frame + 8, &win.to_le_bytes()).unwrap();
                u(ba::change_macro2(g, win, code, pen, Some(frame + 8)))
            });
        }
        assert!(steps > 50, "{name}: pen steps {steps}");
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// `TODrawBtlWindow`: every callee a recording stub, the sub-windows'
/// callbacks too.
#[test]
fn ba_draw_btl_window() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b402);
        let mut stubs = vec![
            (ba::BATTLE_CURSOR_CALC, 0),
            (ba::BTL_INIT_GS, 0),
            (ba::MAKE_PKT_SCISSOR, 4),
            (ba::DRAW_LIMIT_BG, 0),
            (ba::SEQ_DRAW, 3),
            (ba::DAMAGE_VALUE2, 0),
            (ba::SIMPLE_HELP_MES, 1),
            (ba::BATTLE_GET_STATUS, 0),
            (ba::STATUS_WIN, 0),
            (ba::ACTIVE_CHR_CURSOR, 1),
            (ba::CTB_WINDOW, 2),
            (ba::HELP_WIN, 0),
            (ba::INFO_WIN, 3),
            (ba::INFO_WIN_MES, 1),
            (ba::SCISSOR_XYWH, 4),
            (ba::BTL_CURSORS, 0),
        ];
        stubs.extend(ba::BTL_DRAWERS.map(|f| (f, 0)));
        for &(sym, n) in &stubs {
            t.stub(sym, n, false);
        }
        let callbacks: Vec<u32> = (0..3)
            .map(|k| t.stub_code(0xcb00 + k, false, 1, 0, false, &[]))
            .collect();
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.extend(0xcb00..0xcb03);
        let mut deep = 0;
        for i in 0..500 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let r = rng.next();
            let st = rng.pick(&[0, 1, 2, 2, 1, 3, r]);
            t.ret(ba::BATTLE_GET_STATUS, st);
            let r = rng.next();
            let v = rng.pick(&[0, 1, 1, 1, r]);
            t.g32(0x0133_c948, v);
            for (a, n) in [
                (0x0133_d6f4, 2),
                (0x0133_d708, 2),
                (0x0133_d734, 2),
                (0x0133_c93f, 1),
            ] {
                let v = if rng.below(4) == 0 { rng.next() | 1 } else { 0 };
                let at = t.rebase(a);
                t.write(at, &v.to_le_bytes()[..n]);
            }
            let v = rng.pick(&[1, 0, 2]);
            t.w8(t.rebase(0x00c5_8ef4), v);
            let r = rng.next();
            let v = rng.pick(&[0, 1, r]);
            t.g32(0x023c_c08c, v);
            let v = rng.pick(&[1, 1, 0, 2]);
            t.g32(0x00c5_8ef8, v);
            let v = rng.pick(&[0, 0, 1, 2]);
            t.g32(0x0133_d6e0, v);
            for a in [0x023c_c088, 0x025d_0a88, 0x025d_0a8a, 0x0133_d1f4] {
                let v = rng.next() as u16;
                t.w16(t.rebase(a), v);
            }
            for k in 0..8 {
                let rec = t.rebase(ba::SUB_WINDOWS) + 0xf0 * k;
                let v = rng.pick(&[0, 2, 3, 4, 0x80, 0x7f]);
                t.w8(rec + 1, v);
                let f = if rng.below(4) == 0 {
                    0
                } else {
                    rng.pick(&callbacks)
                };
                t.w32(rec + 0x90, f);
            }
            if st == 1 || st == 2 {
                deep += 1;
            }
            t.check(&w, 0x0089_b360, &[], |g| {
                ba::draw_btl_window(g);
                Out::None
            });
        }
        assert!(deep > 100, "{name}: {deep}");
    }
}

/// `Sg_MFIFOSync`: the wait without and with a timeout (the VIF1
/// registers mapped and random in both oracles).
#[test]
fn ba_mfifo_sync() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b403);
        t.stub(ba::DBG_PRINTF, 3, false);
        t.stub(ba::SCE_PRINTF, 2, false);
        let tags = [ba::DBG_PRINTF, ba::SCE_PRINTF];
        t.a.map_zeroed(ba::VIF1_REGS & !0xfff, 0x1000).unwrap();
        t.b.map_zeroed(ba::VIF1_REGS & !0xfff, 0x1000).unwrap();
        let log = t.log;
        for i in 0..30 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let timeout = i % 10 == 9;
            let sink = rng.next();
            let sink = if timeout { sink | 0x100 } else { sink & !0x100 };
            t.g32(ba::GS_REGISTER_SINK, sink);
            let v = rng.next();
            t.g32(ba::SG_FRAME, v);
            for k in 0..16 {
                let v = rng.next();
                t.w32(ba::VIF1_REGS + 0x10 * k, v);
            }
            // the one- and two-word prints leave stack junk in the log
            let fix = move |o: &mut Oracle| {
                for (entry, from) in [(0, 1), (1, 2), (17, 1)] {
                    for word in from..3 {
                        let at = log + 4 + 64 * entry + 8 + 4 * word;
                        o.write(at, &[0; 4]).unwrap();
                    }
                }
                let at = log + 4 + 64 * 18 + 16;
                o.write(at, &[0; 4]).unwrap();
            };
            t.check_fix(&w, 0x0084_3360, &[], |g| u(ba::mfifo_sync(g)), fix);
            if timeout {
                assert_eq!(t.log_entries(), 19, "{w}: the report");
            }
        }
    }
}

/// `Sg_SetDispDrawEnv_008217d0`: both buffers, every video mode byte.
#[test]
fn ba_set_disp_draw_env() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b404);
        let stubs = [
            (ba::SET_SCREEN_WH, 2),
            (ba::SET_DEF_DISP_ENV, 6),
            (ba::SET_DEF_DRAW_ENV1, 6),
            (ba::SET_DEF_DRAW_ENV, 6),
            (ba::VRAM_GET_ADDR_Z, 0),
        ];
        for (sym, n) in stubs {
            t.stub(sym, n, false);
        }
        let tags = stubs.map(|s| s.0);
        let frame = t.ebp_for(1);
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let r = rng.next() as u8;
            let mode = rng.pick(&[0, 1, 2, 3, r]);
            t.w8(t.rebase(0x012f_b8fb), mode);
            for k in 0..4 {
                let v = rng.next();
                t.g32(0x00c4_94ac + 4 * k, v);
            }
            for k in 0..2 {
                let (a, b) = (rng.below(1024), rng.next());
                let v = rng.pick(&[a, b]);
                t.g32(ba::SCREEN_SIZE + 4 * k, v);
            }
            for k in 0..0x84 {
                let v = rng.next();
                t.g32(0x012f_b900 + 4 * k, v);
            }
            let r = rng.next();
            let buffer = rng.pick(&[0, 1, 2, 3, r]);
            t.check_framed(&w, 0x0082_17d0, &[buffer], frame, &[(8, 4)], |g| {
                g.0.write(frame + 8, &buffer.to_le_bytes()).unwrap();
                u(ba::set_disp_draw_env(g, buffer, Some(frame + 8)))
            });
        }
    }
}

/// The DMA and GS path stubs of the uploads; their tags.
fn dma_stubs(t: &mut Twins) -> Vec<u32> {
    let stubs = [
        (ba::DMA_GET_CHAN, 1),
        (ba::DMA_SEND, 2),
        (ba::DMA_SEND_N, 3),
        (ba::DMA_SYNC, 3),
        (ba::FLUSH_CACHE, 1),
        (ba::GS_SYNC_PATH, 2),
    ];
    for (sym, n) in stubs {
        t.stub(sym, n, false);
    }
    stubs.map(|s| s.0).to_vec()
}

/// `TOTransImageHL`: every pixel format, the DMA tag in the original's
/// aligned frame.
#[test]
fn ba_trans_image_hl() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b405);
        let tags = dma_stubs(&mut t);
        // the return address's slot, then the `and esp, -16` frame
        let ret = t.a.stack_range().end - 0x100 - 4 * 7 - 4;
        let ebp = (ret - 12) & !0xf;
        let tag = ebp - 0x30;
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let r = rng.next();
            let psm = if i < 0x30 {
                i
            } else {
                rng.pick(&[0, 1, 2, 10, 0x13, 0x1b, 0x14, 0x24, 0x2c, r])
            };
            let args: Vec<u32> = (0..7)
                .map(|k| {
                    let (a, b, c) = (rng.below(2048), rng.next(), rng.small());
                    if k == 2 { psm } else { rng.pick(&[a, a, b, c]) }
                })
                .collect();
            t.check_framed(&w, 0x008b_0b10, &args, tag, &[(0, 0x20)], |g| {
                u(ba::trans_image_hl(
                    g, tag, args[0], args[1], args[2], args[3], args[4], args[5], args[6],
                ))
            });
        }
    }
}

/// `yiPrintGs_0090aaa0`: texts with newlines and wide lines, centred or
/// not; `Sg_GetDBuffDC` runs as the original (it writes the frame word).
#[test]
fn ba_print_gs() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b406);
        let tags = dma_stubs(&mut t);
        let ebp = t.ebp_for(7);
        let colour = t.zeros(4);
        for i in 0..150 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let n = rng.below(14) as usize;
            let mut text: Vec<u8> = (0..n)
                .map(|_| match rng.below(8) {
                    0 => 0xa,
                    1 => rng.next() as u8 | 1,
                    _ => 0x20 + rng.below(0x60) as u8,
                })
                .collect();
            text.push(0);
            let at = t.alloc(&text);
            let c = rng.next();
            t.w32(colour, c);
            for k in 0..0x60 {
                let v = rng.next();
                t.g32(ba::DEBUG_FONT + 4 * k, v);
            }
            let v = rng.next();
            t.g32(0x023c_bbf0, v);
            let (x, y) = (rng.small(), rng.small());
            let r = rng.next();
            let cols = rng.pick(&[0, 1, 3, 40, r]);
            let r = rng.next();
            // no column limit and no row limit would loop for 2^32 lines
            let r = if cols == 0 { 7 } else { r };
            let rows = rng.pick(&[0, 1, 2, 40, r]);
            let r = rng.next();
            let center = rng.pick(&[0, 0, 1, r]);
            let args = [at, x, y, cols, rows, center, colour];
            t.check_framed(
                &w,
                0x0090_aaa0,
                &args,
                ebp - 0xc,
                &[(0, 4), (0x14, 0x1c)],
                |g| {
                    for (k, a) in args.iter().enumerate() {
                        g.0.write(ebp + 8 + 4 * k as u32, &a.to_le_bytes()).unwrap();
                    }
                    u(ba::print_gs(
                        g,
                        ebp - 0xc,
                        at,
                        x,
                        y,
                        cols,
                        rows,
                        center,
                        colour,
                        Some(ebp + 8),
                    ))
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

// ---- batch 2 (ffx_glue_other::h4b_b)

fn random_block(t: &mut Twins, rng: &mut Rng, n: u32) -> u32 {
    let at = t.zeros(n as usize);
    for k in 0..n / 4 {
        let v = rng.next();
        t.w32(at + 4 * k, v);
    }
    at
}

/// A stub at `sym`'s tag whose results alternate between a second cell
/// (`next`, the first call) and its own cell: (code address, `next`).
fn alternating_stub(
    t: &mut Twins,
    tag: u32,
    this: bool,
    n: u32,
    pops: u32,
    writes: &[Write],
) -> (u32, u32) {
    let next = t.zeros(4);
    let log = t.stub_code(tag, this, n, pops, false, writes);
    let cell = t.cells[&tag];
    let mut c = vec![0xa1];
    c.extend(next.to_le_bytes()); // mov eax, [next]
    c.extend([0x87, 0x05]);
    c.extend(cell.to_le_bytes()); // xchg [cell], eax
    c.push(0xa3);
    c.extend(next.to_le_bytes()); // mov [next], eax
    let at = t.cave_next;
    c.push(0xe9);
    c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // jmp log
    (t.code(&c), next)
}

/// `gdraw_makeTextureBegin_00a3a070`: sizes at and past the limit, RGBA
/// and single-channel, with and without mips, every allocation failing.
#[test]
fn bb_make_texture_begin() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b501);
        let malloc = t.stub_code(0x0bad_b500, false, 3, 3, false, &[]);
        let free = t.stub_code(0x0bad_b501, false, 1, 1, false, &[]);
        let warn = t.stub_code(0x0bad_b502, false, 4, 0, false, &[]);
        t.stub(bb::CONFIG_MEMORY, 3, false);
        let tags = [0x0bad_b500, 0x0bad_b501, 0x0bad_b502, bb::CONFIG_MEMORY];
        let state = t.zeros(0xac0);
        let mut ok = 0;
        for i in 0..800 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(bb::GDRAW_MALLOC.sym, malloc);
            t.g32(bb::GDRAW_FREE.sym, free);
            t.g32(bb::GDRAW_WARNING.sym, warn);
            t.g32(bb::GDRAW_STATE.sym, state);
            let pool = rng.next();
            t.w32(state + 0xab0, pool);
            let p = rng.pick(&[0, 0x1000, 0x1000, 0x1000]);
            t.ret(0x0bad_b500, p);
            let record = random_block(&mut t, &mut rng, 0x14);
            let record = rng.pick(&[0, record, record, record]);
            t.ret(bb::CONFIG_MEMORY, record);
            let info = random_block(&mut t, &mut rng, 0x58);
            let flags = rng.next();
            let dim = |rng: &mut Rng| {
                let (a, b, c) = (rng.below(0x4100), rng.below(64), rng.next());
                // a negative size with mips would never end (as in the game)
                let c = if flags & 1 != 0 { c & 0x7fff_ffff } else { c };
                rng.pick(&[a, b, b, 0x3fff, 0x4000, 1, 0, c])
            };
            let (dw, dh) = (dim(&mut rng), dim(&mut rng));
            let format = rng.pick(&[0, 0, 1, 0x1c]);
            let a = [rng.next(), dw, dh, format, flags, info, rng.next()];
            let r = t.check_conv(&w, 0x00a3_a070, Conv::Stdcall, &a, |g| {
                u(bb::make_texture_begin(
                    g, a[0], a[1], a[2], a[3], a[4], a[5], a[6],
                ))
            });
            ok += r;
        }
        assert!(ok > 100, "{name}: {ok}");
    }
}

/// `gdraw_makeVertexBufferEnd_00a3a5f0`: the device's `CreateBuffer`
/// (recording, results alternating between the two calls, the buffer
/// stored through the out pointer), the buffers' `Release`, the heap
/// ring relink on failure.
#[test]
fn bb_make_vertex_buffer_end() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b502);
        let free = t.stub_code(0x0bad_b511, false, 1, 1, false, &[]);
        let warn = t.stub_code(0x0bad_b512, false, 5, 0, false, &[]);
        let buffer_cell = t.zeros(4);
        let (create, next) =
            alternating_stub(&mut t, 0x0bad_b513, false, 4, 4, &[wr(3, buffer_cell)]);
        let release = t.stub_code(0x0bad_b514, false, 1, 1, false, &[]);
        t.stub(bb::MOVE_BLOCK, 4, false);
        let tags = [
            0x0bad_b511,
            0x0bad_b512,
            0x0bad_b513,
            0x0bad_b514,
            bb::MOVE_BLOCK,
        ];
        let device_vt = t.zeros(0x40);
        t.w32(device_vt + 0xc, create);
        let device = t.zeros(4);
        t.w32(device, device_vt);
        let buffer_vt = t.zeros(0x10);
        t.w32(buffer_vt + 8, release);
        let state = t.zeros(0x10);
        let ebp = t.ebp_for(2);
        let log = t.log;
        let mut failed = 0;
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(bb::GDRAW_FREE.sym, free);
            t.g32(bb::GDRAW_WARNING.sym, warn);
            t.g32(bb::GDRAW_STATE.sym, state);
            t.w32(state, device);
            let hrs = [0, 1, 0x8007_000e, 0x8000_4005, 0x7fff_ffff];
            let (h0, h1) = (rng.pick(&hrs), rng.pick(&hrs));
            t.w32(next, h0);
            t.ret(0x0bad_b513, h1);
            let buffers: Vec<u32> = (0..2)
                .map(|_| {
                    let b = t.zeros(4);
                    t.w32(b, buffer_vt);
                    b
                })
                .collect();
            let made = rng.pick(&[0, buffers[0]]);
            t.w32(buffer_cell, made);
            // the heap ring: the block between two neighbours, a heap
            let record = random_block(&mut t, &mut rng, 0x40);
            let prev = random_block(&mut t, &mut rng, 0x40);
            let after = random_block(&mut t, &mut rng, 0x40);
            let heap = random_block(&mut t, &mut rng, 0x40);
            let last = random_block(&mut t, &mut rng, 0x40);
            t.w32(record + 0x1c, after);
            t.w32(record + 0x20, prev);
            t.w32(record + 0x18, heap);
            t.w32(heap + 0x30, last);
            for off in [0, 4] {
                let v = rng.pick(&[0, buffers[1]]);
                t.w32(record + off, v);
            }
            let info = random_block(&mut t, &mut rng, 0x18);
            t.w32(info + 0x10, record);
            let a = [info, rng.next()];
            if (h0 as i32) < 0 || (h1 as i32) < 0 {
                failed += 1;
            }
            let fix = move |o: &mut Oracle| {
                // the out-of-memory warning pushes four words: the fifth
                // logged one is stack junk
                let n: u32 = Mem::read(o, log);
                for e in 0..n {
                    let at = log + 4 + 64 * e;
                    let tag: u32 = Mem::read(o, at);
                    let fmt: u32 = Mem::read(o, at + 12);
                    if tag == 0x0bad_b512 && fmt == o.rebase(0x00b7_a4ec) {
                        o.write(at + 8 + 16, &[0; 4]).unwrap();
                    }
                }
            };
            t.write(ebp - 0x4c, &[0xa5; 0x48]);
            t.b.set_call_top(Some(ebp - 0x200));
            let addr = t.rebase(0x00a3_a5f0);
            let ra =
                t.a.call(addr, Conv::Stdcall, &a)
                    .unwrap_or_else(|e| panic!("{w}: {e}"));
            let tramp = t.tramp;
            let port = bb::make_vertex_buffer_end(&mut G(&mut t.b, tramp), ebp - 0x4c, info);
            t.b.set_call_top(None);
            assert_eq!(ra.eax, port, "{w}: eax");
            fix(&mut t.a);
            fix(&mut t.b);
            t.diff(&w);
            let x = Oracle::read(&mut t.a, ebp - 0x4c, 0x48).unwrap();
            let y = Oracle::read(&mut t.b, ebp - 0x4c, 0x48).unwrap();
            assert_eq!(x, y, "{w}: frame");
        }
        assert!(failed > 100, "{name}: {failed}");
    }
}

/// A fake COM object whose vtable slots (`(offset, words, writes)`) are
/// recording stdcall stubs tagged `tag + offset`; the object's address.
fn com(t: &mut Twins, tag: u32, slots: &[(u32, u32, &[Write])]) -> u32 {
    let vt = t.zeros(0x100);
    for &(off, n, writes) in slots {
        let f = t.stub_code(tag + off, false, n, n, false, writes);
        t.w32(vt + off, f);
    }
    let obj = t.zeros(8);
    t.w32(obj, vt);
    obj
}

/// `gdraw_createRowTexture_00a3a9f0`: the device's texture and view
/// calls failing in turn, old objects released.
#[test]
fn bb_create_row_texture() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b503);
        let warn = t.stub_code(0x0bad_b522, false, 5, 0, false, &[]);
        let made = t.zeros(4);
        let device = com(
            &mut t,
            0xd000,
            &[(0x14, 4, &[wr(3, made)]), (0x1c, 4, &[wr(3, made)])],
        );
        let objs: Vec<u32> = (0..3)
            .map(|k| com(&mut t, 0xe000 + 0x100 * k, &[(8, 1, &[])]))
            .collect();
        let tags = [0x0bad_b522, 0xd014, 0xd01c, 0xe008, 0xe108, 0xe208];
        let state = t.zeros(0x800);
        let ebp = t.ebp_for(2);
        let log = t.log;
        for i in 0..500 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(bb::GDRAW_WARNING.sym, warn);
            t.g32(bb::GDRAW_STATE.sym, state);
            t.w32(state, device);
            let hrs = [0, 1, 0x8007_000e, 0x8000_4005];
            let h = rng.pick(&hrs);
            t.ret(0xd014, h);
            let h = rng.pick(&hrs);
            t.ret(0xd01c, h);
            for off in [0x798, 0x79c] {
                let v = rng.pick(&[0, objs[0], objs[1]]);
                t.w32(state + off, v);
            }
            let m = rng.pick(&[0, objs[2]]);
            t.w32(made, m);
            let a = [rng.pickr(&[1, 64, 256]), rng.next()];
            let fix = move |o: &mut Oracle| {
                let n: u32 = Mem::read(o, log);
                for e in 0..n {
                    let at = log + 4 + 64 * e;
                    let tag: u32 = Mem::read(o, at);
                    let fmt: u32 = Mem::read(o, at + 12);
                    if tag == 0x0bad_b522 && fmt == o.rebase(0x00b7_a4ec) {
                        o.write(at + 8 + 16, &[0; 4]).unwrap();
                    }
                }
            };
            let frame = ebp - 0x3c;
            t.write(frame, &[0xa5; 0x38]);
            t.b.set_call_top(Some(frame - 0x100));
            let addr = t.rebase(0x00a3_a9f0);
            let ra =
                t.a.call(addr, Conv::Stdcall, &a)
                    .unwrap_or_else(|e| panic!("{w}: {e}"));
            let tramp = t.tramp;
            let port = bb::create_row_texture(&mut G(&mut t.b, tramp), frame, a[0], a[1]);
            t.b.set_call_top(None);
            assert_eq!(ra.eax, port, "{w}: eax");
            fix(&mut t.a);
            fix(&mut t.b);
            t.diff(&w);
            let x = Oracle::read(&mut t.a, frame, 0x38).unwrap();
            let y = Oracle::read(&mut t.b, frame, 0x38).unwrap();
            assert_eq!(x, y, "{w}: frame");
        }
    }
}

/// The two quad draws: the context's state, shader, map (the data
/// pointer stored through the out pointer, 0 or a block), unmap and
/// constant-buffer calls; viewport, projection, memset and the quad as
/// recording stubs.
#[test]
fn bb_quads() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b504);
        let warn = t.stub_code(0x0bad_b532, false, 5, 0, false, &[]);
        let data = t.zeros(4);
        let ctx = com(
            &mut t,
            0xc000,
            &[
                (0x8c, 4, &[]),
                (0x90, 3, &[]),
                (0x24, 4, &[]),
                (0x38, 6, &[wr(5, data)]),
                (0x3c, 3, &[]),
                (0x40, 4, &[]),
            ],
        );
        for (sym, n) in [
            (bb::SET_VIEWPORT, 4),
            (bb::SET_PROJECTION, 4),
            (bb::DRAW_QUAD, 6),
            (bb::MEMSET, 3),
        ] {
            t.stub(sym, n, false);
        }
        let tags = [
            0x0bad_b532,
            0xc08c,
            0xc090,
            0xc024,
            0xc038,
            0xc03c,
            0xc040,
            bb::SET_VIEWPORT,
            bb::SET_PROJECTION,
            bb::DRAW_QUAD,
            bb::MEMSET,
        ];
        let state = t.zeros(0xa00);
        let log = t.log;
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(bb::GDRAW_WARNING.sym, warn);
            t.g32(bb::GDRAW_STATE.sym, state);
            for k in 0..0x280 {
                let v = rng.next();
                t.w32(state + 4 * k, v);
            }
            t.w32(state + 4, ctx);
            for off in [0x6c0, 0x6c4, 0x6c8, 0x6cc, 0x6d8, 0x6dc, 0x6e0, 0x6e4] {
                let v = rng.small();
                t.w32(state + off, v);
            }
            let h = rng.pick(&[0, 1, 0x8007_000e, 0x8000_4005]);
            t.ret(0xc038, h);
            let d = rng.pick(&[0, 0x1234, 0x5678]);
            t.w32(data, d);
            let fix = move |o: &mut Oracle| {
                let n: u32 = Mem::read(o, log);
                for e in 0..n {
                    let at = log + 4 + 64 * e;
                    let tag: u32 = Mem::read(o, at);
                    let fmt: u32 = Mem::read(o, at + 12);
                    if tag == 0x0bad_b532 && fmt == o.rebase(0x00b7_a4ec) {
                        o.write(at + 8 + 16, &[0; 4]).unwrap();
                    }
                }
            };
            let rect = t.zeros(16);
            for k in 0..4 {
                let (sm, nx) = (rng.small(), rng.next());
                let v = rng.pick(&[sm, nx]);
                t.w32(rect + 4 * k, v);
            }
            let a = rng.next();
            let ebp = t.ebp_for(2);
            run_framed(
                &mut t,
                &w,
                0x00a3_bd40,
                &[rect, a],
                ebp - 0x10,
                12,
                None,
                &fix,
                |g| bb::draw_rect_quad(g, ebp - 0x10, rect, a),
            );
            let depth = rng.next();
            let sub = rng.pickr(&[0, 1]);
            let ebp = t.ebp_for(3);
            run_framed(
                &mut t,
                &format!("{w} target"),
                0x00a3_bed0,
                &[a, depth, sub],
                ebp - 0xc,
                12,
                Some(ebp + 8),
                &fix,
                |g| bb::draw_target_quad(g, ebp - 0xc, a, depth, sub, Some(ebp + 8)),
            );
        }
    }
}

/// Runs a cdecl original and its port with a frame block (`frame`, `len`
/// bytes) at the same address, the port's calls below it; `slots` (the
/// first argument word) gets the arguments in the port's oracle and is
/// compared too (12 words). `fix` runs on both before the diff.
#[allow(clippy::too_many_arguments)]
fn run_framed(
    t: &mut Twins,
    w: &str,
    sym: u32,
    args: &[u32],
    frame: u32,
    len: usize,
    slots: Option<u32>,
    fix: &dyn Fn(&mut Oracle),
    port: impl FnOnce(&mut G) -> u32,
) {
    t.write(frame, &vec![0xa5; len]);
    if let Some(s) = slots {
        for (k, a) in args.iter().enumerate() {
            t.b.write(s + 4 * k as u32, &a.to_le_bytes()).unwrap();
        }
    }
    t.b.set_call_top(Some(frame - 0x100));
    let addr = t.rebase(sym);
    let ra =
        t.a.call(addr, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{w}: {e}"));
    let tramp = t.tramp;
    let v = port(&mut G(&mut t.b, tramp));
    t.b.set_call_top(None);
    assert!(ra.st0_raw.is_none(), "{w}: ST0 left set");
    assert_eq!(ra.eax, v, "{w}: eax");
    fix(&mut t.a);
    fix(&mut t.b);
    t.diff(w);
    let x = Oracle::read(&mut t.a, frame, len).unwrap();
    let y = Oracle::read(&mut t.b, frame, len).unwrap();
    assert_eq!(x, y, "{w}: frame");
    if let Some(s) = slots {
        let n = 4 * args.len();
        let x = Oracle::read(&mut t.a, s, n).unwrap();
        let y = Oracle::read(&mut t.b, s, n).unwrap();
        assert_eq!(x, y, "{w}: argument slots");
    }
}

// ---- batch 3 (ffx_glue_other::h4b_c)

/// The remap, time and shape stubs of the cursors; their tags.
fn cursor_stubs(t: &mut Twins) -> Vec<u32> {
    t.stub(e1::REMAP_X2, 1, true);
    t.stub(e1::REMAP_Y2, 1, true);
    t.stub(e1::GET_TIME, 0, true);
    t.stub(e1::SHAPE_UV, 9, false);
    vec![e1::REMAP_X2, e1::REMAP_Y2, e1::GET_TIME, e1::SHAPE_UV]
}

/// Fresh `ST0` results for the float stubs and random float constants.
fn cursor_floats(t: &mut Twins, rng: &mut Rng, f32s: &[u32], f64s: &[u32]) {
    for tag in [e1::REMAP_X2, e1::REMAP_Y2, e1::GET_TIME] {
        let v = rng.double();
        t.ret_f64(tag, v);
    }
    t.constants(rng, f32s, f64s);
}

/// `TkMn2DrawCrossCursor`: every kind, the argument slots compared;
/// `graphicFontGetScreenWH` runs as the original.
#[test]
fn bc_cross_cursor() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b601);
        let tags = cursor_stubs(&mut t);
        let f32s = [
            0x00b5_eec0,
            0x00b3_eb98,
            0x00b5_ee90,
            0x00b5_1b9c,
            0x00b6_5854,
            0x00b6_57fc,
            0x00b6_585c,
            0x00b3_04f8,
            0x00b5_ee64,
            0x00b5_f0fc,
            0x00b5_f0ec,
            0x00b5_f108,
            0x00b5_f100,
            0x00b5_f0cc,
            0x00b5_f0e8,
        ];
        let f64s = [
            0x00b9_22a8,
            0x00b2_c5c8,
            0x00b4_1cc0,
            0x00b4_1d30,
            0x00b9_22f0,
            0x00b5_4c70,
        ];
        let ebp = t.ebp_for(3);
        let fix = |_: &mut Oracle| {};
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            cursor_floats(&mut t, &mut rng, &f32s, &f64s);
            let (x, y) = (rng.float(), rng.float());
            let kind = rng.pickr(&[0, 4, 5, 0, 4, 5, 1, 6]);
            run_framed(
                &mut t,
                &w,
                0x008c_0670,
                &[x, y, kind],
                ebp - 0x40,
                0,
                Some(ebp + 8),
                &fix,
                |g| bc::cross_cursor(g, ebp - 0x40, x, y, kind, Some(ebp + 8)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// `TODrawSelectCursor`: armed and unarmed selections, number entry and
/// the cross cursor; `TOGetMessageWH` a hand-written stub storing
/// alternating widths through its out pointer.
#[test]
fn bc_select_cursor() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b602);
        let digits = t.zeros(4);
        t.stub_at(bc::INT_TO_MES, false, 4, 0, &[wr(1, digits)]);
        for (sym, n) in [
            (bc::CROSS_BOX, 6),
            (bc::SET_INVOKED, 1),
            (bc::CROSS_CURSOR, 3),
        ] {
            t.stub(sym, n, false);
        }
        // TOGetMessageWH: *arg2 = [a]; swap [a] and [b]; then log
        let (wa, wb) = (t.zeros(4), t.zeros(4));
        let log = t.stub_code(bc::GET_MESSAGE_WH, false, 6, 0, false, &[]);
        let mut c = vec![0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x15];
        c.extend(wa.to_le_bytes());
        c.extend([0x89, 0x10, 0x87, 0x15]);
        c.extend(wb.to_le_bytes());
        c.extend([0x89, 0x15]);
        c.extend(wa.to_le_bytes());
        let at = t.cave_next;
        c.push(0xe9);
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let wh = t.code(&c);
        t.jump(bc::GET_MESSAGE_WH, wh);
        let tags = [
            bc::INT_TO_MES,
            bc::CROSS_BOX,
            bc::SET_INVOKED,
            bc::CROSS_CURSOR,
            bc::GET_MESSAGE_WH,
        ];
        let f32s = [0x00b5_4f84];
        let f64s = [0x00b9_2270, 0x00b9_22c0, 0x00b4_b468];
        let ebp = t.ebp_for(1);
        let frame = ebp - 0x38;
        let mut armed = 0;
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(&mut rng, &f32s, &f64s);
            for cell in [digits, wa, wb] {
                let v = rng.float();
                t.w32(cell, v);
            }
            let window = rng.below(8);
            let win = random_block(&mut t, &mut rng, 0x60);
            let v = rng.pickr(&[2, 2, 0, 1]);
            t.w16(win + 0x2c, v as u16);
            let v = rng.pickr(&[2, 2, 0]);
            t.w16(win + 0x1c, v as u16);
            for off in [0x48, 0x4c] {
                let v = rng.float();
                t.w32(win + off, v);
            }
            let block = random_block(&mut t, &mut rng, 0x100);
            let sel = block + 0x40;
            let v = rng.pickr(&[0, 1, 1, 2]);
            t.w8(sel + 1, v as u8);
            let ch = rng.below(10) as i32 - 3;
            t.w8(sel + 2, ch as u8);
            let v = rng.below(10);
            t.w8(sel + 5, v as u8);
            t.g32(bc::MSG_WINDOWS + 4 * window, win);
            t.g32(bc::SELECTIONS + 4 * window, sel);
            let mut rec_ = [0u32; 7];
            for (k, r) in rec_.iter_mut().enumerate() {
                *r = rng.next();
                t.g32(bc::SELECT_RECORD + 4 * k as u32, *r);
            }
            if t.check_framed(
                &w,
                0x008b_5fb0,
                &[window],
                frame,
                &[(0, 4), (4, 4), (8, 4), (0x14, 0x20)],
                |g| u(bc::select_cursor(g, frame, window)),
            ) != win
            {
                armed += 1;
            }
        }
        assert!(armed > 100, "{name}: {armed}");
    }
}

/// The limit windows' words, kind set from `kinds`.
fn limit_words(t: &mut Twins, rng: &mut Rng, at: u32, kinds: &[u32]) {
    for k in 0..0x20 {
        let v = rng.small() as u16;
        t.w16(t.rebase(at) + 2 * k, v);
    }
    let kind = rng.pickr(kinds);
    t.w16(t.rebase(at), kind as u16);
}

/// `TOBtlDrawTidusLimitWindow` and `TOBtlDrawLuluLimitWindow`: the
/// drawers as recording stubs, the caption's width through the frame.
#[test]
fn bc_limit_windows() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b603);
        let mut tags = cursor_stubs(&mut t);
        let width = t.zeros(4);
        t.stub_at(bc::EASY_FONT_WIDTH, false, 5, 0, &[wr(1, width)]);
        let buf = t.zeros(4);
        t.stub_at(bc::LOC_TEXT, false, 3, 0, &[wr(1, buf)]);
        for (sym, n) in [
            (bc::SPECIAL_GAUGE, 3),
            (bc::CROSS_BOX, 6),
            (bc::ROM_BTL_TEXT, 2),
            (bc::EXT_MES_FONT, 7),
            (bc::LULU_COUNTER, 4),
            (bc::CROSS_BOX_FW, 7),
        ] {
            t.stub(sym, n, false);
            tags.push(sym);
        }
        tags.extend([bc::EASY_FONT_WIDTH, bc::LOC_TEXT]);
        let f32s = [
            0x00b5_ef98,
            0x00b5_ee6c,
            0x00b5_efa8,
            0x00b5_ede8,
            0x00b5_ee08,
            0x00b5_edd8,
            0x00b5_ee00,
            0x00b5_4cf8,
            0x00b5_83d8,
            0x00b5_ef90,
            0x00b5_4f90,
            0x00b5_ee10,
            0x00b5_ef10,
            0x00b4_ae10,
            0x00b9_2318,
        ];
        let f64s = [0x00b9_2220, 0x00b4_1d38, 0x00b5_ee50, 0x00b9_21e0];
        let ebp = t.ebp_for(0);
        let mut drawn = 0;
        for i in 0..500 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            cursor_floats(&mut t, &mut rng, &f32s, &f64s);
            for cell in [width, buf] {
                let v = rng.float();
                t.w32(cell, v);
            }
            // the Lulu block (0x0133d708, 0x40 bytes) overlaps the Tidus
            // block's start: written first
            limit_words(&mut t, &mut rng, 0x0133_d708, &[3, 4, 3, 4, 2, 5]);
            limit_words(&mut t, &mut rng, bc::LIMIT_WORDS, &[3, 4, 3, 4, 2, 5]);
            let mode = rng.pickr(&[0, 1, 0, 1]);
            t.g32(bc::CONTROL_MODE, mode);
            let kind: i16 = Mem::read(&t.b, t.rebase(bc::LIMIT_WORDS));
            if (3..=4).contains(&kind) {
                drawn += 1;
            }
            t.check_framed(&w, 0x0089_7240, &[], ebp - 0xc, &[(0, 4)], |g| {
                u(bc::tidus_limit_window(g, ebp - 0xc))
            });
            // the never-written width word: the same in both (the shim's
            // frame gives 0; brief 2026-10-24)
            let frame = ebp - 0x110;
            t.check_framed(
                &format!("{w} lulu"),
                0x0089_5660,
                &[],
                frame,
                &[(0, 4), (0xc, 0x100)],
                |g| u(bc::lulu_limit_window(g, frame)),
            );
        }
        assert!(drawn > 100, "{name}: {drawn}");
    }
}

/// `TOBtlDrawCrossLimitGauge`: full, partial and empty gauges; `cosf` a
/// float stub; the last argument slot compared. The original's `eax` is
/// the last call's or the FPU status word (the shim is void).
#[test]
fn bc_cross_limit_gauge() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b604);
        let mut tags = cursor_stubs(&mut t);
        t.stub(bc::COSF, 1, true);
        tags.push(bc::COSF);
        for (sym, n) in [
            (bc::CROSS_BOX, 6),
            (bc::SHAPE_UV_C2, 11),
            (bc::LIMIT_GAUGE_MAX, 1),
            (bc::LIMIT_GAUGE, 1),
        ] {
            t.stub(sym, n, false);
            tags.push(sym);
        }
        let f32s = [
            0x00b9_22f8,
            0x00b9_2314,
            0x00b4_7de8,
            0x00b9_2308,
            0x00b9_2268,
            0x00b5_eff8,
            0x00b5_ef38,
            0x00b5_f01c,
            0x00b5_ef3c,
        ];
        let f64s = [0x00b2_c5c8, 0x00b4_12f8, 0x00b5_ee40];
        let ebp = t.ebp_for(5);
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            cursor_floats(&mut t, &mut rng, &f32s, &f64s);
            let v = rng.double();
            t.ret_f64(bc::COSF, v);
            let sp = rng_small_pos(&mut rng);
            let max = rng.pickr(&[100, 0, 50, 255, sp]);
            let sp = rng_small_pos(&mut rng);
            let g = rng.pickr(&[100, 0, 50, 255, 0xffff_ffff, sp]);
            t.ret(bc::LIMIT_GAUGE_MAX, max);
            t.ret(bc::LIMIT_GAUGE, g);
            let a = [
                rng.next(),
                rng.float(),
                rng.float(),
                rng.float(),
                rng.float(),
            ];
            for (k, v) in a.iter().enumerate() {
                t.b.write(ebp + 8 + 4 * k as u32, &v.to_le_bytes()).unwrap();
            }
            t.b.set_call_top(Some(ebp - 0x100));
            t.check(&w, 0x0089_2910, &a, |g| {
                bc::cross_limit_gauge(g, a[0], a[1], a[2], a[3], a[4], Some(ebp + 8));
                Out::None
            });
            t.b.set_call_top(None);
            let x = Oracle::read(&mut t.a, ebp + 8, 20).unwrap();
            let y = Oracle::read(&mut t.b, ebp + 8, 20).unwrap();
            assert_eq!(x, y, "{w}: argument slots");
        }
    }
}

/// `TOBtlDrawStatusLimitGauge`: gauges at, above and below the maximum; `cosf` a
/// float stub; the last argument slot compared. The original's `eax` is
/// the last call's or the FPU status word (the shim is void).
#[test]
fn bc_status_limit_gauge() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b605);
        let mut tags = cursor_stubs(&mut t);
        t.stub(bc::COSF, 1, true);
        tags.push(bc::COSF);
        for (sym, n) in [
            (bc::CROSS_BOX, 6),
            (bc::SHAPE_UV_C2, 11),
            (bc::LIMIT_GAUGE_MAX, 1),
            (bc::LIMIT_GAUGE, 1),
        ] {
            t.stub(sym, n, false);
            tags.push(sym);
        }
        let f32s = [
            0x00b9_22f8,
            0x00b9_2314,
            0x00b4_7de8,
            0x00b9_2308,
            0x00b9_2268,
            0x00b5_eff8,
            0x00b5_ef38,
            0x00b5_f01c,
            0x00b5_ef3c,
        ];
        let f64s = [0x00b2_c5c8, 0x00b4_12f8, 0x00b5_ee40];
        let ebp = t.ebp_for(5);
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            cursor_floats(&mut t, &mut rng, &f32s, &f64s);
            let v = rng.double();
            t.ret_f64(bc::COSF, v);
            let sp = rng_small_pos(&mut rng);
            let max = rng.pickr(&[100, 0, 50, 255, sp]);
            let sp = rng_small_pos(&mut rng);
            let g = rng.pickr(&[100, 0, 50, 255, 0xffff_ffff, sp]);
            t.w16(t.rebase(bc::STATUS_GAUGE_MAX), max as u16);
            let a = [g, rng.float(), rng.float(), rng.float(), rng.float()];
            for (k, v) in a.iter().enumerate() {
                t.b.write(ebp + 8 + 4 * k as u32, &v.to_le_bytes()).unwrap();
            }
            t.b.set_call_top(Some(ebp - 0x100));
            t.check(&w, 0x0089_60e0, &a, |g| {
                bc::status_limit_gauge(g, a[0], a[1], a[2], a[3], a[4], Some(ebp + 8));
                Out::None
            });
            t.b.set_call_top(None);
            let x = Oracle::read(&mut t.a, ebp + 8, 20).unwrap();
            let y = Oracle::read(&mut t.b, ebp + 8, 20).unwrap();
            assert_eq!(x, y, "{w}: argument slots");
        }
    }
}

/// `TkBtlEndDrawLegendNotice_008bc100`: the size from a stub through the
/// frame, the plate, scissors and text as recording stubs.
#[test]
fn bc_legend_notice() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b606);
        let (cw, ch) = (t.zeros(4), t.zeros(4));
        t.stub_at(bc::MESSAGE_WH_1612, false, 8, 0, &[wr(2, cw), wr(3, ch)]);
        let mut tags = vec![bc::MESSAGE_WH_1612];
        for (sym, n) in [
            (bc::OPEN_PKT_TMP, 0),
            (bc::DRAW_KICK_TMP, 0),
            (bc::MENU_PLATE, 5),
            (bc::SCISSOR_XYWH, 4),
            (bc::EXT_MES_FONT, 7),
        ] {
            t.stub(sym, n, false);
            tags.push(sym);
        }
        let f32s = [0x00b5_ee10];
        let f64s = [
            0x00b6_5648,
            0x00b9_22b0,
            0x00b4_1d38,
            0x00b4_1d30,
            0x00b9_2220,
            0x00b5_0190,
            0x00b5_15c8,
        ];
        let ebp = t.ebp_for(0);
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            if i % 2 == 0 {
                t.constants(&mut rng, &f32s, &f64s);
            }
            for cell in [cw, ch] {
                let v = if rng.below(3) == 0 {
                    rng.float()
                } else {
                    (rng.below(600) as f32).to_bits()
                };
                t.w32(cell, v);
            }
            let fade = rng.pickr(&[0, 0x1000, 0x800, 0x200]);
            t.w16(t.rebase(bc::END_FADE), fade as u16);
            let text = rng.pickr(&[0, 0x1234]);
            t.g32(bc::END_TEXT, text);
            let drawn = fade as u16 != 0 && text != 0;
            t.check_framed(&w, 0x008b_c100, &[], ebp - 0xc, &[(0, 8)], |g| {
                let v = bc::legend_notice(g, ebp - 0xc);
                if drawn { u(v) } else { Out::None }
            });
        }
    }
}

/// `TkMn2DrawCrossLimitParam`: values at, above and below the maximum;
/// the pulse through `_CIcos`; the fourth argument slot compared.
#[test]
fn bc_cross_limit_param() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b607);
        let mut tags = cursor_stubs(&mut t);
        for (sym, n) in [(bc::CROSS_BOX, 6), (bc::SHAPE_UV_C2, 11)] {
            t.stub(sym, n, false);
            tags.push(sym);
        }
        let f32s = [
            0x00b9_22f8,
            0x00b9_2314,
            0x00b4_7de8,
            0x00b9_2308,
            0x00b9_2268,
            0x00b5_eff8,
            0x00b5_ef38,
            0x00b5_f01c,
            0x00b5_ef3c,
        ];
        let f64s = [0x00b2_c5c8, 0x00b4_12f8, 0x00b5_ee40];
        let ebp = t.ebp_for(6);
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            cursor_floats(&mut t, &mut rng, &f32s, &f64s);
            let sp = rng_small_pos(&mut rng);
            let max = rng.pickr(&[100, 0, 50, 255, sp]);
            let sp = rng_small_pos(&mut rng);
            let v = rng.pickr(&[100, 0, 50, 255, 0xffff_ffff, sp]);
            let a = [rng.float(), rng.float(), rng.float(), rng.float(), v, max];
            for (k, x) in a.iter().enumerate() {
                t.b.write(ebp + 8 + 4 * k as u32, &x.to_le_bytes()).unwrap();
            }
            t.b.set_call_top(Some(ebp - 0x100));
            t.check(&w, 0x008d_3690, &a, |g| {
                bc::cross_limit_param(g, a[0], a[1], a[2], a[3], a[4], a[5], Some(ebp + 8));
                Out::None
            });
            t.b.set_call_top(None);
            let x = Oracle::read(&mut t.a, ebp + 8, 24).unwrap();
            let y = Oracle::read(&mut t.b, ebp + 8, 24).unwrap();
            assert_eq!(x, y, "{w}: argument slots");
        }
    }
}

/// `TkMn2DrawCrossCharFace`: every face (Rikku's too) and some out of
/// range; the image size and the record in the frame.
#[test]
fn bc_cross_char_face() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b608);
        let (cw, ch) = (t.zeros(4), t.zeros(4));
        t.stub_at(bc::IMAGE_WH, false, 3, 0, &[wr(1, cw), wr(2, ch)]);
        let mut tags = vec![bc::IMAGE_WH];
        for (sym, n) in [
            (bc::SHAPE_TEXTURE_NAME, 1),
            (bc::ALBHED_RIKKU, 0),
            (bc::DRAW_UI_ELEMENT, 5),
        ] {
            t.stub(sym, n, false);
            tags.push(sym);
        }
        let ebp = t.ebp_for(5);
        let frame = ebp - 0xb8;
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let r = rng.pickr(&[0, 1, 2]);
            t.ret(bc::ALBHED_RIKKU, r);
            for cell in [cw, ch] {
                let v = if rng.below(4) == 0 {
                    rng.float()
                } else {
                    (rng.below(1000) as f32).to_bits()
                };
                t.w32(cell, v);
            }
            for k in 0..4 {
                let v = rng.next();
                t.g32(bc::FONT_SHADE + 8 * k, v);
            }
            let face = if i < 30 {
                i
            } else {
                rng.pickr(&[6, 6, 7, 8, 20, 0x8000_0000])
            };
            let a = [face, rng.float(), rng.float(), rng.float(), rng.float()];
            t.check_framed(&w, 0x008c_0250, &a, frame, &[(0xc, 8), (0x1c, 0x98)], |g| {
                u(bc::cross_char_face(g, frame, a[0], a[1], a[2], a[3], a[4]))
            });
        }
    }
}

/// `IggyMouseQueue_update_00a25ab0`: a fake mouse (buttons and axes from
/// tables), presses, releases, swapped buttons, wheel; the four-event
/// case that hangs the original is left out.
#[test]
fn bc_mouse_queue() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b609);
        t.stub(bc::GET_DEVICE, 2, false);
        let buttons = t.zeros(4);
        let axes = t.zeros(12);
        let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0x0f, 0xb6, 0x80];
        c.extend((buttons.wrapping_sub(0xb)).to_le_bytes());
        c.extend([0xc2, 0x04, 0x00]);
        let button = t.code(&c);
        let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0xd9, 0x04, 0x85];
        c.extend((axes - 4).to_le_bytes());
        c.extend([0xc2, 0x04, 0x00]);
        let axis_code = t.code(&c);
        let vt = t.zeros(0x20);
        t.w32(vt + 0xc, button);
        t.w32(vt + 0x10, axis_code);
        let mouse = t.zeros(8);
        t.w32(mouse, vt);
        let q = t.zeros(0x80);
        let mut events = 0;
        for i in 0..800 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &[bc::GET_DEVICE]);
            t.ret(bc::GET_DEVICE, mouse);
            for k in 0..0x20 {
                let v = rng.next();
                t.w32(q + 4 * k, v);
            }
            let prev: [u8; 3] = std::array::from_fn(|_| rng.pickr(&[0, 1, 0, 1, 2]) as u8);
            let now: [u8; 3] = std::array::from_fn(|_| rng.pickr(&[0, 1, 0, 1, 3]) as u8);
            t.write(q, &prev);
            t.write(buttons, &now);
            let swap = rng.pickr(&[0, 1, 0]) as u8;
            t.w8(q + 0x78, swap);
            let mut b = now;
            if swap != 0 {
                b.swap(0, 2);
            }
            let change = |k: usize| matches!((prev[k], b[k]), (0, 1) | (1, 0));
            for k in 0..3 {
                let v = if k == 2 && rng.below(2) == 0 {
                    0
                } else {
                    rng.float()
                };
                let v = if k == 2 && change(0) && change(2) {
                    0
                } else {
                    v
                };
                t.w32(axes + 4 * k, v);
            }
            if change(0) || change(2) {
                events += 1;
            }
            t.check_conv(&w, 0x00a2_5ab0, Conv::Thiscall(q), &[], |g| {
                bc::mouse_queue_update(g, q);
                Out::None
            });
        }
        assert!(events > 100, "{name}: {events}");
    }
}

/// `IggyMenuManager_dispatchMouse_00a26ce0`: queued events of every type
/// (and unknown ones) to fake menus (some slots empty); the lock and the
/// event makers through their import slots.
#[test]
fn bc_dispatch_mouse() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b60a);
        let slots = [
            (bc::ENTER_CS.sym, 1),
            (bc::LEAVE_CS.sym, 1),
            (bc::MAKE_EVENT_NONE.sym, 1),
            (bc::MAKE_EVENT_MOVE.sym, 3),
            (bc::MAKE_EVENT_BUTTON.sym, 2),
            (bc::MAKE_EVENT_WHEEL.sym, 2),
        ];
        let stubs: Vec<(u32, u32)> = slots
            .iter()
            .map(|&(slot, n)| (slot, t.stub_code(slot, false, n, n, false, &[])))
            .collect();
        let mut tags: Vec<u32> = slots.iter().map(|s| s.0).collect();
        let menus: Vec<u32> = (0..3)
            .map(|k| {
                let vt = t.zeros(8);
                let f0 = t.stub_code(0xf000 + k, true, 0, 0, false, &[]);
                let f4 = t.stub_code(0xf100 + k, true, 2, 2, false, &[]);
                t.w32(vt, f0);
                t.w32(vt + 4, f4);
                let m = t.zeros(4);
                t.w32(m, vt);
                m
            })
            .collect();
        tags.extend((0..3).flat_map(|k| [0xf000 + k, 0xf100 + k]));
        let ebp = t.ebp_for(0);
        let frame = ebp - 0x28;
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            for &(slot, code) in &stubs {
                t.g32(slot, code);
            }
            let n = rng.below(5);
            let mut next = 0;
            for _ in 0..n {
                let e = t.zeros(0x14);
                let kind = rng.pickr(&[0, 1, 2, 3, 4, 5, 6, 7, 6, 7]);
                t.w32(e, kind);
                for off in [4, 8, 0xc] {
                    let v = rng.float();
                    t.w32(e + off, v);
                }
                t.w32(e + 0x10, next);
                next = e;
            }
            let queue = t.zeros(0x80);
            t.w32(queue + 0xc, next);
            t.g32(0x0198_4c68, queue);
            let count = rng.below(4);
            let arr = t.zeros(16);
            for k in 0..4 {
                let m = rng.pick(&[0, menus[0], menus[1], menus[2]]);
                t.w32(arr + 4 * k, m);
            }
            let this = t.zeros(0x10);
            t.w32(this + 8, count | (rng.below(2) << 31));
            t.w32(this + 0xc, arr);
            t.check_framed_conv(
                &w,
                0x00a2_6ce0,
                Conv::Thiscall(this),
                &[],
                frame,
                &[(0, 0x24)],
                |g| u(bc::dispatch_mouse(g, frame, this)),
            );
        }
    }
}

/// A gauge-sized positive integer.
fn rng_small_pos(rng: &mut Rng) -> u32 {
    1 + rng.below(300)
}

// ---- batch 4 (ffx_glue_other::h4b_d)

/// `gdraw_allocRenderTarget_00a3b610`: a cached target, the budget, the
/// heap and each device call failing in turn.
#[test]
fn bd_alloc_render_target() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b701);
        let warn = t.stub_code(0x0bad_b702, false, 3, 0, false, &[]);
        let made = t.zeros(4);
        let device = com(
            &mut t,
            0xd100,
            &[
                (0x14, 4, &[wr(3, made)]),
                (0x1c, 4, &[wr(3, made)]),
                (0x24, 4, &[wr(3, made)]),
            ],
        );
        let objs: Vec<u32> = (0..3)
            .map(|k| com(&mut t, 0xe400 + 0x100 * k, &[(8, 1, &[])]))
            .collect();
        let mut tags = vec![0x0bad_b702, 0xd114, 0xd11c, 0xd124, 0xe408, 0xe508, 0xe608];
        for (sym, n) in [
            (bd::REQUEUE, 2),
            (bd::TAKE_FREE, 1),
            (bd::MOVE_BLOCK, 4),
            (bd::FREE_BLOCK, 1),
            (bd::REPORT_HR, 3),
        ] {
            t.stub(sym, n, false);
            tags.push(sym);
        }
        let state = t.zeros(0x800);
        let ebp = t.ebp_for(1);
        let desc = ebp - 0x30;
        let log = t.log;
        let mut made_ok = 0;
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(bb::GDRAW_WARNING.sym, warn);
            t.g32(bb::GDRAW_STATE.sym, state);
            t.w32(state, device);
            let cache_ring = state + 0x2d8;
            let cached = random_block(&mut t, &mut rng, 0x40);
            let c = rng.pick(&[cache_ring, 0, 0, 0, cached]);
            t.w32(cache_ring + 0x1c, c);
            let kind = rng.pick(&[1u32, 1, 5, 2]) << 28 | rng.below(0x1000);
            t.w32(cached + 0x30, kind);
            let owner = rng.pick(&[1, 1, 0]);
            t.w32(cached + 0x14, owner);
            let (sw, sh) = (rng.below(64), rng.below(64));
            t.w32(state + 0x6c0, sw);
            t.w32(state + 0x6c4, sh);
            let budget = rng.pick(&[0x10_0000, 0, sw * sh * 4, (sw * sh * 4).wrapping_sub(1)]);
            t.w32(state + 0x290, budget);
            let limit = rng.next();
            t.w32(state + 0x294, limit);
            let block = random_block(&mut t, &mut rng, 0x40);
            let b = rng.pick(&[block, block, block, 0]);
            t.ret(bd::TAKE_FREE, b);
            let hrs = [0, 0, 1, 0x8007_000e, 0x8000_4005];
            for tag in [0xd114, 0xd11c, 0xd124] {
                let v = rng.pick(&hrs);
                t.ret(tag, v);
            }
            let m = rng.pick(&[0, objs[0], objs[1], objs[2]]);
            t.w32(made, m);
            let stat_block = random_block(&mut t, &mut rng, 0x20);
            t.write(desc, &[0xa5; 0x2c]);
            t.b.set_call_top(Some(desc - 0x100));
            let addr = t.rebase(0x00a3_b610);
            let ra =
                t.a.call(addr, Conv::Cdecl, &[stat_block])
                    .unwrap_or_else(|e| panic!("{w}: {e}"));
            let tramp = t.tramp;
            let port = bd::alloc_render_target(&mut G(&mut t.b, tramp), desc, stat_block);
            t.b.set_call_top(None);
            assert_eq!(ra.eax, port, "{w}: eax");
            if port == block {
                made_ok += 1;
            }
            // the two-word warning's third logged word is stack junk
            let fix = |o: &mut Oracle| {
                let n: u32 = Mem::read(o, log);
                for e in 0..n {
                    let at = log + 4 + 64 * e;
                    let tag: u32 = Mem::read(o, at);
                    let fmt: u32 = Mem::read(o, at + 12);
                    if tag == 0x0bad_b702 && fmt == o.rebase(0x00b7_a73c) {
                        o.write(at + 16, &[0; 4]).unwrap();
                    }
                }
            };
            fix(&mut t.a);
            fix(&mut t.b);
            t.diff(&w);
            let x = Oracle::read(&mut t.a, desc, 0x2c).unwrap();
            let y = Oracle::read(&mut t.b, desc, 0x2c).unwrap();
            assert_eq!(x, y, "{w}: frame");
        }
        assert!(made_ok > 30, "{name}: {made_ok}");
    }
}

/// `IggyMenuManager_init_00a262e0`: arrays of the right and other sizes,
/// owned or not, allocations failing; `Config_InitDefaults` runs as the
/// original.
#[test]
fn bd_menu_manager_init() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b702);
        let (malloc, next) = alternating_stub(&mut t, 0x0bad_b710, false, 2, 0, &[]);
        t.jump(bd::PHYRE_MALLOC, malloc);
        t.stub(bd::PHYRE_FREE, 1, false);
        t.stub(bd::GDRAW_INIT, 3, false);
        t.stub_at(bd::SET_VIEWPORT, true, 2, 2, &[]);
        let cb = t.stub_code(0x0bad_b711, false, 2, 2, false, &[]);
        let tags = [
            0x0bad_b710,
            0x0bad_b711,
            bd::PHYRE_FREE,
            bd::GDRAW_INIT,
            bd::SET_VIEWPORT,
        ];
        let ebp = t.ebp_for(3);
        let frame = ebp - 0x24;
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(bd::SET_AS3_CALLBACK.sym, cb);
            let b0 = random_block(&mut t, &mut rng, 0x20);
            let b1 = random_block(&mut t, &mut rng, 0xb0);
            let first = rng.pick(&[b0, b0, 0]);
            t.w32(next, first);
            let second = rng.pick(&[b1, b1, 0]);
            t.ret(0x0bad_b710, second);
            let this = random_block(&mut t, &mut rng, 0x20);
            let old0 = random_block(&mut t, &mut rng, 0x40);
            let old1 = random_block(&mut t, &mut rng, 0x100);
            let n0 = rng.pick(&[5, 5, 3, 0, 0x8000_0003, 0x8000_0005]);
            t.w32(this + 8, n0);
            // a null old block only where the new one comes (the clearing
            // loop would write through it)
            let p0 = if first == 0 || n0 == 5 {
                rng.pick(&[old0, b0])
            } else {
                rng.pick(&[old0, old0, b0, 0])
            };
            t.w32(this + 0xc, p0);
            let n1 = rng.pick(&[10, 10, 4, 0, 0x8000_0002, 0x8000_000a]);
            t.w32(this + 0x10, n1);
            let p1 = rng.pick(&[old1, old1, b1]);
            t.w32(this + 0x14, p1);
            let v = rng.next();
            t.g32(0x01a8_58c4, v);
            let a = [rng.next(), rng.next(), rng.next()];
            t.check_framed_conv(
                &w,
                0x00a2_62e0,
                Conv::Thiscall(this),
                &a,
                frame,
                &[(0, 0x20), (0x23, 1)],
                |g| u(bd::menu_manager_init(g, frame, this, a[0], a[1], a[2])),
            );
        }
    }
}

/// `TODrawBtlCtbParts`: counts across every band edge, party members
/// (Rikku before and after the scenario mark), 8..17 and monsters of
/// every icon type; the argument slots compared.
#[test]
fn bd_btl_ctb_parts() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b703);
        let mut tags = cursor_stubs(&mut t);
        for (sym, n) in [
            (bd::CROSS_BOX, 6),
            (bd::CTB_ICON, 5),
            (bd::SCENARIO_FLAG, 0),
            (bd::CHR_ICON_TYPE, 1),
            (bd::CHR_ICON_NUMBER, 1),
        ] {
            t.stub(sym, n, false);
            tags.push(sym);
        }
        let f32s = [
            0x00b5_ee90,
            0x00b9_2268,
            0x00b5_f304,
            0x00b5_f314,
            0x00b5_67a8,
        ];
        let f64s = [0x00b5_af28];
        let ebp = t.ebp_for(4);
        for i in 0..800 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            cursor_floats(&mut t, &mut rng, &f32s, &f64s);
            let flag = rng.pick(&[0x433, 0x434, 0, 0x1000]);
            t.ret(bd::SCENARIO_FLAG, flag);
            let kind = rng.pickr(&[0x14, 0x15, 0x16, 0x17, 0x13]);
            t.ret(bd::CHR_ICON_TYPE, kind);
            let count = if i < 80 {
                i
            } else {
                rng.pickr(&[0, 19, 20, 39, 40, 55, 56, 59, 60, 100])
            };
            let chr = rng.pickr(&[0, 1, 6, 6, 7, 8, 17, 18, 19, 20, 30]);
            let a = [chr, count, rng.float(), rng.float()];
            for (k, v) in a.iter().enumerate() {
                t.b.write(ebp + 8 + 4 * k as u32, &v.to_le_bytes()).unwrap();
            }
            t.b.set_call_top(Some(ebp - 0x100));
            t.check(&w, 0x008a_16d0, &a, |g| {
                bd::btl_ctb_parts(g, a[0], a[1], a[2], a[3], Some(ebp + 8));
                Out::None
            });
            t.b.set_call_top(None);
            let x = Oracle::read(&mut t.a, ebp + 8, 16).unwrap();
            let y = Oracle::read(&mut t.b, ebp + 8, 16).unwrap();
            assert_eq!(x, y, "{w}: argument slots");
        }
    }
}

/// `TODrawChrFaceCell_008efd90`: every face and mode, Rikku, faces out
/// of range; the image size and the record in the frame, the face slot.
#[test]
fn bd_chr_face_cell() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b704);
        let (cw, ch) = (t.zeros(4), t.zeros(4));
        t.stub_at(bd::IMAGE_WH, false, 3, 0, &[wr(1, cw), wr(2, ch)]);
        let mut tags = vec![bd::IMAGE_WH];
        for (sym, n) in [
            (bd::SHAPE_TEXTURE_NAME, 1),
            (bd::ALBHED_RIKKU, 0),
            (bd::DRAW_UI_ELEMENT, 5),
        ] {
            t.stub(sym, n, false);
            tags.push(sym);
        }
        let f64s = [0x00b6_6ca8, 0x00b6_6cb8, 0x00b9_22d8];
        let ebp = t.ebp_for(9);
        let frame = ebp - 0xb4;
        for i in 0..800 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(&mut rng, &[], &f64s);
            let r = rng.pickr(&[0, 1, 2]);
            t.ret(bd::ALBHED_RIKKU, r);
            for cell in [cw, ch] {
                let v = if rng.below(4) == 0 {
                    rng.float()
                } else {
                    (rng.below(1000) as f32).to_bits()
                };
                t.w32(cell, v);
            }
            let face = if i < 40 {
                i % 20
            } else {
                rng.pickr(&[6, 6, 7, 8, 17, 18, 0x8000_0000])
            };
            let mode = rng.pickr(&[6, 6, 0, 1]);
            let a = [
                rng.next(),
                face,
                rng.small(),
                rng.small(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                mode,
            ];
            t.check_framed(&w, 0x008e_fd90, &a, frame, &[(0, 8), (0x18, 0x98)], |g| {
                for (k, v) in a.iter().enumerate() {
                    g.0.write(ebp + 8 + 4 * k as u32, &v.to_le_bytes()).unwrap();
                }
                let shades = [a[4], a[5], a[6], a[7]];
                u(bd::chr_face_cell(
                    g,
                    frame,
                    a[0],
                    a[1],
                    a[2],
                    a[3],
                    shades,
                    a[8],
                    Some(ebp + 8),
                ))
            });
            let x = Oracle::read(&mut t.a, ebp + 8, 36).unwrap();
            let y = Oracle::read(&mut t.b, ebp + 8, 36).unwrap();
            assert_eq!(x, y, "{w}: argument slots");
        }
    }
}

/// `gdraw_makeTextureEnd_00a3a200`: mip chains of several sizes and
/// formats, each device call failing in turn; the description and the
/// level array in the frame, the record's ring moves.
#[test]
fn bd_make_texture_end() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b705);
        let warn = t.stub_code(0x0bad_b720, false, 5, 0, false, &[]);
        let free = t.stub_code(0x0bad_b721, false, 1, 1, false, &[]);
        let made = t.zeros(4);
        let device = com(
            &mut t,
            0xd200,
            &[(0x14, 4, &[wr(3, made)]), (0x1c, 4, &[wr(3, made)])],
        );
        let objs: Vec<u32> = (0..2)
            .map(|k| com(&mut t, 0xe700 + 0x100 * k, &[(8, 1, &[])]))
            .collect();
        t.stub(bd::TEXELS_MIP, 7, false);
        let tags = [
            0x0bad_b720,
            0x0bad_b721,
            0xd214,
            0xd21c,
            0xe708,
            0xe808,
            bd::TEXELS_MIP,
        ];
        let state = t.zeros(0x10);
        let ebp = t.ebp_for(2);
        let frame = ebp - 0x150;
        let log = t.log;
        let mut ok = 0;
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(bb::GDRAW_WARNING.sym, warn);
            t.g32(bb::GDRAW_FREE.sym, free);
            t.g32(bb::GDRAW_STATE.sym, state);
            t.w32(state, device);
            let hrs = [0, 0, 1, 0x8007_000e, 0x8000_4005];
            for tag in [0xd214, 0xd21c] {
                let v = rng.pick(&hrs);
                t.ret(tag, v);
            }
            let m = rng.pick(&[0, objs[0], objs[1]]);
            t.w32(made, m);
            let record = random_block(&mut t, &mut rng, 0x40);
            let heap = random_block(&mut t, &mut rng, 0x180);
            let (prev, next) = (
                random_block(&mut t, &mut rng, 0x40),
                random_block(&mut t, &mut rng, 0x40),
            );
            t.w32(record + 0x18, heap);
            t.w32(record + 0x1c, next);
            t.w32(record + 0x20, prev);
            for k in 0..6 {
                let last = random_block(&mut t, &mut rng, 0x40);
                // ring heads at heap + 0x10 and + 8 * (7 kind + 2)
                t.w32(heap + 0x10 + 0x38 * k + 0x20, last);
            }
            let r0 = rng.pick(&[0, objs[0]]);
            t.w32(record, r0);
            let r1 = rng.pick(&[0, objs[1]]);
            t.w32(record + 4, r1);
            let info = random_block(&mut t, &mut rng, 0x58);
            t.w32(info + 0x18, record);
            let levels = rng.pick(&[0, 1, 2, 5, 9]);
            t.w32(info + 0x4c, levels);
            let (tw, th) = (rng.below(600), rng.below(600));
            t.w32(info + 0x38, tw);
            t.w32(info + 0x3c, th);
            let texel = rng.pick(&[1, 4]);
            t.w32(info + 0x50, texel);
            let stat_block = random_block(&mut t, &mut rng, 0x20);
            t.write(frame, &[0xa5; 0x14c]);
            t.b.set_call_top(Some(frame - 0x100));
            let addr = t.rebase(0x00a3_a200);
            let ra =
                t.a.call(addr, Conv::Stdcall, &[info, stat_block])
                    .unwrap_or_else(|e| panic!("{w}: {e}"));
            let tramp = t.tramp;
            let port = bd::make_texture_end(&mut G(&mut t.b, tramp), frame, info, stat_block);
            t.b.set_call_top(None);
            assert_eq!(ra.eax, port, "{w}: eax");
            if port != 0 {
                ok += 1;
            }
            let fix = |o: &mut Oracle| {
                let n: u32 = Mem::read(o, log);
                for e in 0..n {
                    let at = log + 4 + 64 * e;
                    let tag: u32 = Mem::read(o, at);
                    let fmt: u32 = Mem::read(o, at + 12);
                    if tag == 0x0bad_b720 && fmt == o.rebase(0x00b7_a4ec) {
                        o.write(at + 8 + 16, &[0; 4]).unwrap();
                    }
                }
            };
            fix(&mut t.a);
            fix(&mut t.b);
            t.diff(&w);
            let x = Oracle::read(&mut t.a, frame, 0x14c).unwrap();
            let y = Oracle::read(&mut t.b, frame, 0x14c).unwrap();
            assert_eq!(x, y, "{w}: frame");
        }
        assert!(ok > 50, "{name}: {ok}");
    }
}

// ---- batch 5 (ffx_glue_other::h4b_e)

/// `AbmapDrawScreenQuad_00a4b260`: the vertex block from a stub, the
/// divisor random; skipped frames draw nothing.
#[test]
fn be_screen_quad() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b801);
        t.stub(be::ABMAP_VERTEX_INFO, 2, false);
        let ebp = t.ebp_for(0);
        for i in 0..200 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &[be::ABMAP_VERTEX_INFO]);
            t.constants(&mut rng, &[], &[0x00b9_21f0]);
            let skip = rng.pick(&[0, 0, 0, 1]);
            t.g32(be::SKIP_FRAME, skip);
            let info = random_block(&mut t, &mut rng, 0x98);
            let v = random_block(&mut t, &mut rng, 0x24);
            t.w32(info + 0x94, v);
            for (off, n) in [
                (0x20, 0x10),
                (0xc, 0x30),
                (0x10, 0x30),
                (0x14, 0x40),
                (0x1c, 0x10),
            ] {
                let b = random_block(&mut t, &mut rng, n);
                t.w32(v + off, b);
            }
            t.ret(be::ABMAP_VERTEX_INFO, info);
            t.check_framed(&w, 0x00a4_b260, &[], ebp - 0xc, &[], |g| {
                be::screen_quad(g, ebp - 0x40);
                Out::None
            });
        }
    }
}

/// `MenuConfig_DrawRowFrame_008cb530`: the 13 boxes from float stubs and
/// random constants; the argument slots compared.
#[test]
fn be_row_frame() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_b802);
        let mut tags = cursor_stubs(&mut t);
        t.stub(be::CROSS_BOX, 6, false);
        tags.push(be::CROSS_BOX);
        let f32s = [
            0x00b5_f430,
            0x00b3_eb9c,
            0x00b9_2308,
            0x00b4_bf08,
            0x00b6_5c0c,
            0x00b5_ef78,
            0x00b6_5c14,
            0x00b6_5c18,
            0x00b9_2268,
            0x00b3_eb94,
        ];
        let ebp = t.ebp_for(2);
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            cursor_floats(&mut t, &mut rng, &f32s, &[]);
            let a = [rng.float(), rng.float()];
            for (k, v) in a.iter().enumerate() {
                t.b.write(ebp + 8 + 4 * k as u32, &v.to_le_bytes()).unwrap();
            }
            t.b.set_call_top(Some(ebp - 0x100));
            t.check(&w, 0x008c_b530, &a, |g| {
                u(be::row_frame(g, a[0], a[1], Some(ebp + 8)))
            });
            t.b.set_call_top(None);
            let x = Oracle::read(&mut t.a, ebp + 8, 8).unwrap();
            let y = Oracle::read(&mut t.b, ebp + 8, 8).unwrap();
            assert_eq!(x, y, "{w}: argument slots");
        }
    }
}

// ---- round 2, batch 1 (ffx_glue_other::h4b_f)

/// Runs an original (`conv`) and its port with the frame block
/// `[lo, hi)` filled alike (`fill`), the port's calls below it; then the
/// result, the diff and the frame bytes of `compare` (absolute ranges).
#[allow(clippy::too_many_arguments)]
fn run_parts(
    t: &mut Twins,
    w: &str,
    sym: u32,
    conv: Conv,
    args: &[u32],
    (lo, hi): (u32, u32),
    fill: u8,
    compare: &dyn Fn(u32) -> Vec<(u32, u32)>,
    port: impl FnOnce(&mut G) -> u32,
) -> u32 {
    run_zeroed(t, w, sym, conv, args, (lo, hi), fill, &[], compare, port)
}

/// [`run_parts`] with `zero` (absolute) zeroed after the fill: the
/// never-written words the port takes as 0.
#[allow(clippy::too_many_arguments)]
fn run_zeroed(
    t: &mut Twins,
    w: &str,
    sym: u32,
    conv: Conv,
    args: &[u32],
    (lo, hi): (u32, u32),
    fill: u8,
    zero: &[(u32, u32)],
    compare: &dyn Fn(u32) -> Vec<(u32, u32)>,
    port: impl FnOnce(&mut G) -> u32,
) -> u32 {
    t.write(lo, &vec![fill; (hi - lo) as usize]);
    for &(a, b) in zero {
        t.write(a, &vec![0; (b - a) as usize]);
    }
    t.b.set_call_top(Some(lo - 0x100));
    let addr = t.rebase(sym);
    let ra =
        t.a.call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{w}: the original: {e}"));
    let tramp = t.tramp;
    let v = port(&mut G(&mut t.b, tramp));
    t.b.set_call_top(None);
    assert!(ra.st0_raw.is_none(), "{w}: ST0 left set");
    assert_eq!(ra.eax, v, "{w}: eax");
    t.diff(w);
    for (a, b) in compare(v) {
        let x = Oracle::read(&mut t.a, a, (b - a) as usize).unwrap();
        let y = Oracle::read(&mut t.b, a, (b - a) as usize).unwrap();
        assert_eq!(x, y, "{w}: frame {a:#x}..{b:#x}");
    }
    v
}

/// The `sprintf` import slot pointed at a recording stub (4 words).
fn sprintf_stub(t: &mut Twins) -> u32 {
    let f = t.stub_code(0x0bad_f001, false, 4, 0, false, &[]);
    t.g32(bf::SPRINTF.sym, f);
    0x0bad_f001
}

/// A sphere grid block with random words around the fields the quads
/// read; the grid type one of 0..=3.
fn abmap_block(t: &mut Twins, rng: &mut Rng) -> u32 {
    let abmap = t.zeros(0x116c0);
    for off in (0x11680..0x116c0).step_by(4) {
        let v = rng.next();
        t.w32(abmap + off, v);
    }
    let kind = rng.pick(&[0, 0, 2, 2, 1, 3]);
    t.w32(abmap + 0x116a4, kind);
    t.g32(bf::ABMAP.sym, abmap);
    abmap
}

/// `AbmapDrawGridTexture_00a4c460`: every grid type and language, the
/// packet and the record; the name buffer zeroed alike (the junk read).
#[test]
fn bf_grid_texture() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f001);
        let sp = sprintf_stub(&mut t);
        t.stub(bf::GET_LANG, 0, false);
        t.stub(bf::DRAW_UI_ELEMENT, 5, false);
        let tags = [sp, bf::GET_LANG, bf::DRAW_UI_ELEMENT];
        let ebp = t.ebp_for(2);
        for i in 0..200 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let lang = rng.pickr(&[0xb, 0, 1, 0xb]);
            t.ret(bf::GET_LANG, lang);
            abmap_block(&mut t, &mut rng);
            let packet = random_block(&mut t, &mut rng, 0x70);
            t.g32(bf::PACKET.sym, packet);
            t.constants(
                &mut rng,
                &[
                    0x00b7_b618,
                    0x00b7_b5fc,
                    0x00b4_1d9c,
                    0x00b4_1d98,
                    0x00b7_b640,
                    0x00b7_b610,
                ],
                &[],
            );
            let level = rng.pickr(&[0, 1, 0x7f, 0x8000_0000, 0xffff_ff80]);
            let size = rng.next();
            run_zeroed(
                &mut t,
                &w,
                0x00a4_c460,
                Conv::Cdecl,
                &[level, size],
                (ebp - 0x19c, ebp - 4),
                0xa5,
                &[(ebp - 0x104, ebp - 4)],
                &|_| vec![(ebp - 0x19c, ebp - 4)],
                |g| bf::grid_texture(g, ebp - 0x19c, level, size),
            );
        }
    }
}

/// `AbmapDrawLangTitle_00a45ab0`: the cells of every language, random
/// constants.
#[test]
fn bf_lang_title() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f002);
        t.stub(bf::GET_LANG, 0, false);
        capture_stub(&mut t, bf::DRAW_ABMAP_ELEMENT, 3, 0, 0x80);
        let tags = [bf::GET_LANG, bf::DRAW_ABMAP_ELEMENT];
        let ebp = t.ebp_for(2);
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let lang = rng.pick(&[0, 1, 2, 3, 4, 5, 6, 0xb, 0x11, 0x12, 0x13, 0xffff_ffff]);
            t.ret(bf::GET_LANG, lang);
            t.constants(
                &mut rng,
                &[
                    0x00b7_b6d4,
                    0x00b7_b6e4,
                    0x00b7_b5e4,
                    0x00b5_ed84,
                    0x00b7_b5ec,
                    0x00b7_b6dc,
                    0x00b7_b6e0,
                    0x00b9_2268,
                ],
                &[
                    0x00b7_b6a8,
                    0x00b7_b690,
                    0x00b7_b698,
                    0x00b4_c170,
                    0x00b5_8b50,
                    0x00b7_b6a0,
                ],
            );
            let (a, x) = (rng.next(), rng.float());
            run_zeroed(
                &mut t,
                &w,
                0x00a4_5ab0,
                Conv::Cdecl,
                &[a, x],
                (ebp - 0x1ac, ebp - 4),
                0xa5,
                &[(ebp - 0x104, ebp - 4)],
                &|_| vec![(ebp - 0x1ac, ebp - 4)],
                |g| bf::lang_title(g, ebp - 0x1ac, x),
            );
        }
    }
}

/// `AbmapDrawPadArrow_00a4f290`: the bob's step on both sides and at its
/// turns, every grid type; `_CIsin` runs as the original.
#[test]
fn bf_pad_arrow() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f003);
        let sp = sprintf_stub(&mut t);
        t.stub(bf::GET_LANG, 0, false);
        capture_stub(&mut t, bf::DRAW_ABMAP_ELEMENT, 3, 0, 0x80);
        let tags = [sp, bf::GET_LANG, bf::DRAW_ABMAP_ELEMENT];
        let ebp = t.ebp_for(3);
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let lang = rng.pickr(&[0xb, 0, 0xb]);
            t.ret(bf::GET_LANG, lang);
            let abmap = abmap_block(&mut t, &mut rng);
            let bob = if rng.below(3) == 0 {
                rng.float()
            } else {
                (rng.below(400) as f32 / 9.0 - 20.0).to_bits()
            };
            t.w32(abmap + 0x116b4, bob);
            let step = rng.pickr(&[
                0,
                1,
                5,
                10,
                0xb,
                0xc,
                0xffff_ffff,
                0xffff_fffe,
                0xffff_fff5,
                0x8000_0000,
            ]);
            t.w32(abmap + 0x116b8, step);
            t.constants(
                &mut rng,
                &[0x00b5_f13c, 0x00b5_ed60, 0x00b6_6b7c, 0x00b7_b630],
                &[0x00b2_c5c8, 0x00b4_9968, 0x00b5_87d8, 0x00b9_2270],
            );
            // the sine's argument stays ordinary
            for (c, v) in [
                (0x00b4_b468u32, 10.0f64),
                (0x00b7_b688, std::f64::consts::FRAC_PI_2),
            ] {
                let at = t.rebase(c);
                t.write(at, &v.to_le_bytes());
            }
            let item = random_block(&mut t, &mut rng, 0x20);
            let (top, x) = (rng.small(), rng.small());
            run_zeroed(
                &mut t,
                &w,
                0x00a4_f290,
                Conv::Cdecl,
                &[item, top, x],
                (ebp - 0x1ac, ebp - 4),
                0xa5,
                &[(ebp - 0x104, ebp - 4)],
                &|_| vec![(ebp - 0x1ac, ebp - 4)],
                |g| bf::pad_arrow(g, ebp - 0x1ac, item, top, x),
            );
        }
    }
}

/// A `GDraw` state for the begins: random words, the context, the target
/// stack somewhere below `+0xaac` (sometimes full), the sizes small.
fn gdraw_state(t: &mut Twins, rng: &mut Rng, state: u32, ctx: u32, warn: u32) {
    t.g32(bb::GDRAW_WARNING.sym, warn);
    t.g32(bb::GDRAW_STATE.sym, state);
    for k in 0..0x2c0 {
        let v = rng.next();
        t.w32(state + 4 * k, v);
    }
    t.w32(state + 4, ctx);
    let plain = rng.below(2) == 0;
    for off in [0x6c0, 0x6c4, 0x6e0, 0x6e4, 0x6f0, 0x6f4] {
        let v = if !plain {
            rng.pick(&[0, 1, 2, 3, 100, 640, 1000, 0xffff_fffe, 0xffff_ff00])
        } else if off >= 0x6f0 {
            rng.below(8)
        } else {
            600 + rng.below(100)
        };
        t.w32(state + off, v);
    }
    for off in [0x6d8, 0x6dc] {
        let v = rng.pick(&[1, 1, 1, 1, 1, 0]);
        t.w32(state + off, v);
    }
    let flat = rng.pick(&[0, 1]);
    t.w32(state + 0x6f8, flat);
    let depth = rng.pick(&[1, 2, 5, 0x10, 0x11]);
    let top = state + 0xaac - 0x1c * depth;
    t.w32(state + 0xaac, top);
}

/// A target block for the allocation stub: inside the state (its slot
/// row stays inside it too), its pool word the state's or not.
fn gdraw_target(t: &mut Twins, rng: &mut Rng, state: u32) -> u32 {
    let j = rng.below(6);
    let target = state + 0x438 + 0x38 * j + rng.pick(&[0, 4, 0x34]);
    let other = rng.next();
    let pool = rng.pick(&[state + 0x290, other]);
    t.w32(target + 0x18, pool);
    target
}

/// Maps the page the slot row of target 0 lands on (`keep`).
fn map_slot_row(t: &mut Twins, state: u32) {
    let slot = (0u32.wrapping_sub(state).wrapping_sub(0x438) as i32 / 0x38) as u32;
    let at = state
        .wrapping_add(slot.wrapping_mul(16))
        .wrapping_add(0x630)
        & !0xfff;
    let _ = t.a.map_zeroed(at, 0x2000);
    let _ = t.b.map_zeroed(at, 0x2000);
}

fn gdraw_begin_stubs(t: &mut Twins) -> Vec<u32> {
    let mut tags = vec![];
    for (sym, n) in [
        (bf::ALLOC_TARGET, 1),
        (bf::BIND_TOP, 1),
        (bf::DRAW_RECT_QUAD, 2),
        (bf::REQUEUE_NODE, 1),
        (bf::RELEASE_TARGET, 2),
        (bf::TARGET_VIEWPORT, 0),
        (bf::TARGET_PROJECTION, 0),
        (bb::SET_VIEWPORT, 4),
        (bb::SET_PROJECTION, 4),
        (bf::MAP_BUFFER, 1),
    ] {
        t.stub(sym, n, false);
        tags.push(sym);
    }
    let zero = t.zeros(4);
    t.stub_at(bf::DRAW_QUAD, false, 6, 0, &[wr(0, zero)]);
    tags.push(bf::DRAW_QUAD);
    tags
}

/// A rectangle: small values, sometimes empty, reversed or negative.
fn gdraw_rect(t: &mut Twins, rng: &mut Rng) -> u32 {
    let rect = t.zeros(16);
    if rng.below(2) == 0 {
        let (x, y) = (rng.below(300), rng.below(300));
        let (w, h) = (rng.below(400), rng.below(400));
        for (k, v) in [x, y, x + w, y + h].iter().enumerate() {
            t.w32(rect + 4 * k as u32, *v);
        }
        return rect;
    }
    for k in 0..4 {
        let v = rng.pick(&[
            0,
            1,
            2,
            3,
            4,
            50,
            100,
            639,
            640,
            1200,
            0xffff_fffd,
            0xffff_ff00,
        ]);
        t.w32(rect + 4 * k, v);
    }
    rect
}

/// `gdraw_textureDrawBufferBegin_00a3ac90`: kept and new targets, the
/// allocation failing, empty and clipped rectangles, a full stack, a
/// zero-sized screen.
#[test]
fn bf_texture_draw_begin() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f004);
        let warn = t.stub_code(0x0bad_f004, false, 2, 0, false, &[]);
        let ctx = com(&mut t, 0xf000, &[(0xc8, 3, &[])]);
        let mut tags = gdraw_begin_stubs(&mut t);
        tags.extend([0x0bad_f004, 0xf0c8]);
        let state = t.zeros(0xb00);
        map_slot_row(&mut t, state);
        // a kept entry's target is 0: an empty rectangle reads its pool word
        let _ = t.a.map_zeroed(0, 0x1000);
        let _ = t.b.map_zeroed(0, 0x1000);
        let ebp = t.ebp_for(5);
        let mut ok = 0;
        for i in 0..800 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            gdraw_state(&mut t, &mut rng, state, ctx, warn);
            let target = gdraw_target(&mut t, &mut rng, state);
            let made = rng.pick(&[target, target, target, 0]);
            t.ret(bf::ALLOC_TARGET, made);
            let rect = if rng.below(4) == 0 {
                0
            } else {
                gdraw_rect(&mut t, &mut rng)
            };
            let keep = rng.pick(&[0, 0, 1, 0x100]);
            let (b, format, counters) = (rng.next(), rng.next(), rng.next());
            let args = [rect, b, format, keep, counters];
            if keep != 0 && rect == 0 {
                continue; // the original reads the null rectangle
            }
            let v = run_parts(
                &mut t,
                &w,
                0x00a3_ac90,
                Conv::Stdcall,
                &args,
                (ebp - 0x18, ebp),
                0xa5,
                &|_| {
                    if rect == 0 {
                        vec![]
                    } else {
                        vec![(ebp - 0x18, ebp - 8)]
                    }
                },
                |g| bf::texture_draw_begin(g, ebp - 0x18, rect, format, keep, counters),
            );
            ok += v;
        }
        assert!(ok > 100, "{name}: {ok}");
    }
}

/// `gdraw_filterPass_00a374e0`: as the texture begin, plus the filter's
/// input or none, and the pop at the end with the stack valid or not.
#[test]
fn bf_filter_pass() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f005);
        let warn = t.stub_code(0x0bad_f005, false, 2, 0, false, &[]);
        let ctx = com(
            &mut t,
            0xf100,
            &[
                (0xc8, 3, &[]),
                (0x20, 4, &[]),
                (0x28, 4, &[]),
                (0x24, 4, &[]),
                (0x3c, 3, &[]),
                (0x40, 4, &[]),
            ],
        );
        let mut tags = gdraw_begin_stubs(&mut t);
        tags.extend([0x0bad_f005, 0xf1c8, 0xf120, 0xf128, 0xf124, 0xf13c, 0xf140]);
        let state = t.zeros(0xb00);
        let ebp = t.ebp_for(4);
        let mut full = 0;
        for i in 0..800 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            gdraw_state(&mut t, &mut rng, state, ctx, warn);
            let target = gdraw_target(&mut t, &mut rng, state);
            let made = rng.pick(&[target, target, target, 0]);
            t.ret(bf::ALLOC_TARGET, made);
            let dst = t.zeros(0x50);
            t.ret(bf::MAP_BUFFER, dst);
            let rect = gdraw_rect(&mut t, &mut rng);
            let filter = random_block(&mut t, &mut rng, 0xe0);
            let block = random_block(&mut t, &mut rng, 0x30);
            let input = rng.pick(&[0, block]);
            t.w32(filter + 0x1c, input);
            let (colour, counters) = (rng.next(), rng.next());
            // the quad's stub clears the word `colour` points at: the
            // screen's size sometimes, so the pop sees a zero-sized screen
            let cell = t.zeros(16);
            t.w32(cell, colour);
            let colour = rng.pick(&[cell, cell, state + 0x6d8, state + 0x6dc]);
            run_parts(
                &mut t,
                &w,
                0x00a3_74e0,
                Conv::Cdecl,
                &[filter, rect, colour, counters],
                (ebp - 0x24, ebp),
                0xa5,
                &|_| vec![(ebp - 0x24, ebp - 0x10)],
                |g| bf::filter_pass(g, ebp - 0x24, filter, rect, colour, counters),
            );
            let n = t.log_entries();
            for e in 0..n {
                let tag: u32 = Mem::read(&t.b, t.log + 4 + 64 * e);
                if tag == bf::DRAW_QUAD {
                    full += 1;
                    let x = Oracle::read(&mut t.a, ebp - 0x10, 8).unwrap();
                    let y = Oracle::read(&mut t.b, ebp - 0x10, 8).unwrap();
                    assert_eq!(x, y, "{w}: the buffer word and the float");
                }
            }
        }
        assert!(full > 100, "{name}: {full}");
    }
}

/// A recording stub at `sym` (`n` words) that first copies `bytes` from
/// its pointer argument `arg` into a capture area (four slots, by call
/// count), so the diff sees what each call was handed.
fn capture_stub(t: &mut Twins, sym: u32, n: u32, arg: u8, bytes: u32) {
    capture_stub_n(t, sym, n, arg, bytes, 4);
}

/// [`capture_stub`] with `slots` (a power of two up to 128) capture slots.
fn capture_stub_n(t: &mut Twins, sym: u32, n: u32, arg: u8, bytes: u32, slots: u32) {
    let log = t.stub_code(sym, false, n, 0, false, &[]);
    let count = t.zeros(4);
    let area = t.zeros((slots * bytes) as usize);
    let mut c = vec![0x56, 0x57, 0x8b, 0x74, 0x24, 0x0c + 4 * arg]; // push esi; push edi; mov esi, [esp+0xc+4i]
    c.push(0xa1);
    c.extend(count.to_le_bytes()); // mov eax, [count]
    c.extend([0x83, 0xe0, (slots - 1) as u8, 0x69, 0xc0]); // and eax, slots - 1; imul eax, eax, bytes
    c.extend(bytes.to_le_bytes());
    c.extend([0x8d, 0xb8]);
    c.extend(area.to_le_bytes()); // lea edi, [eax+area]
    c.push(0xb9);
    c.extend((bytes / 4).to_le_bytes()); // mov ecx, bytes / 4
    c.extend([0xf3, 0xa5, 0xff, 0x05]); // rep movsd; inc dword [count]
    c.extend(count.to_le_bytes());
    c.extend([0x5f, 0x5e, 0xe9]); // pop edi; pop esi; jmp log
    let at = t.cave_next;
    c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
    let f = t.code(&c);
    t.jump(sym, f);
}

/// A stub that logs its one argument and returns `op` of it (`op` runs
/// after `mov eax, [esp+4]`).
fn arg_stub(t: &mut Twins, sym: u32, op: &[u8]) {
    let log = t.stub_code(sym, false, 1, 0, false, &[]);
    let at = t.cave_next;
    let mut c = vec![0xff, 0x74, 0x24, 0x04, 0xe8]; // push [esp+4]; call log
    c.extend(log.wrapping_sub(at + 9).to_le_bytes());
    c.extend([0x83, 0xc4, 0x04, 0x8b, 0x44, 0x24, 0x04]); // add esp, 4; mov eax, [esp+4]
    c.extend(op);
    c.push(0xc3);
    let f = t.code(&c);
    t.jump(sym, f);
}

/// `TOAdpCrossEasyStrFontSClutRGBA`: texts of digits, letters, the
/// mapped signs and others in every language that ends; line breaks and
/// glyphs by the character's low bits; the `y` slot compared.
#[test]
fn bf_cross_str() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f006);
        let wcell = t.zeros(4);
        let hcell = t.zeros(4);
        t.stub_at(
            bf::FONT_SCREEN_WH,
            false,
            2,
            0,
            &[wr(0, wcell), wr(1, hcell)],
        );
        t.stub(e1::REMAP_Y2, 1, true);
        t.stub(bf::GET_LANG, 0, false);
        // the group from a table: glyphs mostly, some line breaks; J and I glyphs
        let groups: Vec<u8> = (0..=255u32)
            .map(|c| match c {
                0x49 | 0x4a => 4,
                _ => [4, 4, 4, 1, 0][(c % 5) as usize],
            })
            .collect();
        let table = t.alloc(&groups);
        let mut op = vec![0x0f, 0xb6, 0xc0, 0x0f, 0xb6, 0x80]; // movzx eax, al; movzx eax, [eax+table]
        op.extend(table.to_le_bytes());
        arg_stub(&mut t, bf::GET_GROUP, &op);
        arg_stub(&mut t, bf::SKIP_CODE, &[0x40]); // inc eax
        let mut tags = vec![bf::FONT_SCREEN_WH, e1::REMAP_Y2, bf::GET_LANG];
        for (sym, n) in [
            (bf::MEMSET, 3),
            (bf::GET_FONT_INFO, 2),
            (bf::SET_DRAW_TYPE, 1),
            (bf::SET_SHADOW_TYPE, 1),
            (bf::FONT_SHADOW, 3),
            (bf::FONT, 3),
            (bf::FONT_WIDTH_SPACE, 3),
        ] {
            t.stub(sym, n, false);
            tags.push(sym);
        }
        let ebp = t.ebp_for(10);
        let chars =
            b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz --//::!.IJIJ\x80\xff";
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let r = rng.double();
            t.ret_f64(e1::REMAP_Y2, r);
            let lang = rng.pick(&[0, 1, 2, 5, 8, 9, 10, 11, 0x12, 0xffff_ffff]);
            t.ret(bf::GET_LANG, lang);
            let letters = lang <= 10;
            let n = rng.below(24) as usize;
            let mut text: Vec<u8> = (0..n)
                .map(|_| chars[rng.below(chars.len() as u32) as usize])
                .filter(|c| letters || !c.is_ascii_alphabetic())
                .collect();
            text.push(0);
            let at = t.alloc(&text);
            let (sw, sh) = (
                rng.pickr(&[640, 1280, 1920, 0]),
                rng.pickr(&[448, 720, 1080, 0]),
            );
            t.w32(wcell, sw);
            t.w32(hcell, sh);
            t.constants(
                &mut rng,
                &[0x00b5_5140],
                &[
                    0x00b4_3d60,
                    0x00b3_13a0,
                    0x00b9_22a8,
                    0x00b9_2220,
                    0x00b4_8328,
                ],
            );
            let glyph = t.rebase(bf::GLYPH);
            let gw = rng.float();
            t.w32(glyph + 0x1c, gw);
            let args = [
                at,
                rng.float(),
                rng.float(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.float(),
                rng.float(),
            ];
            for (k, v) in args.iter().enumerate() {
                t.b.write(ebp + 8 + 4 * k as u32, &v.to_le_bytes()).unwrap();
            }
            run_parts(
                &mut t,
                &w,
                0x008e_c2f0,
                Conv::Cdecl,
                &args,
                (ebp - 0x118, ebp - 4),
                0,
                &|_| vec![(ebp - 0x118, ebp - 4), (ebp + 8, ebp + 0x30)],
                |g| {
                    bf::cross_str(
                        g,
                        ebp - 0x118,
                        args[0],
                        args[1],
                        args[2],
                        args[3],
                        [args[4], args[5], args[6], args[7]],
                        args[8],
                        args[9],
                        Some(ebp + 8),
                    )
                },
            );
        }
    }
}

// ---- round 2, batch 2 (ffx_glue_other::h4b_g)

/// `TOMakePktShapeCursor`: the bobbing hand (`_CIcos` as the original)
/// and layout cells of every size for kinds 4, 5 and others, with and
/// without the select-cursor flag; the record each draw sees captured.
/// The never-written words (another kind's layout number, the record's
/// unwritten words, all of it for layouts of fewer than two entries) are
/// zero in both frames.
#[test]
fn bg_shape_cursor() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f101);
        t.stub(bg::GET_TIME, 0, true);
        t.stub(e1::REMAP_X2, 1, true);
        t.stub(e1::REMAP_Y2, 1, true);
        let (iw, ih) = (t.zeros(4), t.zeros(4));
        t.stub_at(bg::IMAGE_WH, false, 3, 0, &[wr(1, iw), wr(2, ih)]);
        capture_stub(&mut t, bg::DRAW_UI_ELEMENT, 5, 0, 0x98);
        let mut tags = vec![
            bg::GET_TIME,
            e1::REMAP_X2,
            e1::REMAP_Y2,
            bg::IMAGE_WH,
            bg::DRAW_UI_ELEMENT,
        ];
        for (sym, n) in [
            (bg::SHAPE_XYWHUV, 9),
            (bg::COPY_LAYOUT, 1),
            (bg::SHAPE_TEXTURE_NAME, 1),
        ] {
            t.stub(sym, n, false);
            tags.push(sym);
        }
        let ebp = t.ebp_for(3);
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            for tag in [bg::GET_TIME, e1::REMAP_X2, e1::REMAP_Y2] {
                let v = rng.double();
                t.ret_f64(tag, v);
            }
            let time = f64::from(rng.below(100_000)) / 7.0;
            t.ret_f64(bg::GET_TIME, time);
            let (a, b) = (rng.float(), rng.float());
            t.w32(iw, a);
            t.w32(ih, b);
            t.constants(
                &mut rng,
                &[
                    0x00b5_1b9c,
                    0x00b6_6c08,
                    0x00b6_6b74,
                    0x00b6_6c0c,
                    0x00b5_ee90,
                    0x00b5_eec0,
                    0x00b3_eb98,
                ],
                &[0x00b2_c5c8],
            );
            let k = 0.001 * f64::from(rng.below(100));
            let at = t.rebase(0x00b9_22a8);
            t.write(at, &k.to_le_bytes());
            let n = rng.pick(&[0, 1, 2, 3, 5]);
            let layout = random_block(&mut t, &mut rng, 4 + 12 * 6);
            t.w32(layout, n);
            t.ret(bg::COPY_LAYOUT, layout);
            let select = rng.pick(&[0, 1]);
            t.g32(bg::SELECT_CURSOR.sym, select);
            let kind = rng.pick(&[0, 0, 4, 5, 1, 7, 0xffff_ffff]);
            let (x, y) = (rng.small(), rng.small());
            let mut zero = vec![
                (ebp - 0x5c, ebp - 0x1c),
                (ebp - 0x18, ebp - 0x14),
                (ebp - 0x10, ebp - 0xc),
            ];
            if !matches!(kind, 0 | 4 | 5) {
                zero.push((ebp - 0xac, ebp - 0xa8));
            }
            if n < 2 {
                zero.push((ebp - 0x9c, ebp - 4));
            }
            run_zeroed(
                &mut t,
                &w,
                0x008f_eb00,
                Conv::Cdecl,
                &[kind, x, y],
                (ebp - 0xb8, ebp - 4),
                0xa5,
                &zero,
                &|_| vec![(ebp - 0xb8, ebp - 4)],
                |g| bg::shape_cursor(g, ebp - 0xb8, kind, x, y),
            );
        }
    }
}

/// `yiAssert`: with and without a draw context (either frame word), with
/// and without the abort, part and message, parts shorter and longer
/// than 46 characters. The never-written frame word holds `0xa5` bytes
/// in both frames: the result must not depend on it.
#[test]
fn bg_yi_assert() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f102);
        let flags = t.zeros(4);
        t.stub_at(bg::GET_DBUFF_DC, false, 1, 0, &[wr(0, flags)]);
        let vp = t.stub_code(0x0bad_f102, false, 2, 0, false, &[]);
        t.g32(bg::VPRINTF.sym, vp);
        let vs = t.stub_code(0x0bad_f103, false, 3, 0, false, &[]);
        t.g32(bg::VSPRINTF.sym, vs);
        let mut tags = vec![bg::GET_DBUFF_DC, 0x0bad_f102, 0x0bad_f103];
        for (sym, n) in [
            (bg::DBG_PRINTF, 1),
            (bg::GS_SYNC_V, 1),
            (bg::EXIT_TRACE, 1),
            (bg::GS_SYNC_PATH, 2),
            (bg::GS_RESET_PATH, 0),
            (bg::FLUSH_CACHE, 1),
            (bg::DMA_GET_CHAN, 3),
            (bg::DMA_SYNC, 3),
            (bg::DMA_SEND_N, 3),
            (bg::PRINT_GS, 7),
        ] {
            t.stub(sym, n, false);
            tags.push(sym);
        }
        let ebp = t.ebp_for(5);
        for i in 0..200 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let f = rng.next();
            t.w32(flags, f);
            let dc = random_block(&mut t, &mut rng, 0x40);
            let dc = rng.pick(&[dc, dc, dc, 0]);
            t.ret(bg::GET_DBUFF_DC, dc);
            let abort = rng.pick(&[0, 1, 0x100]);
            if dc == 0 && abort != 0 {
                continue; // the original goes on to draw with no context
            }
            let n = rng.pick(&[0, 5, 46, 47, 80]);
            let mut text: Vec<u8> = (0..n).map(|_| b'a' + rng.below(26) as u8).collect();
            text.push(0);
            let part = t.alloc(&text);
            let part = rng.pick(&[part, part, 0]);
            let fmt = rng.next();
            let format = rng.pick(&[0, fmt]);
            let args = [abort, part, format, rng.next(), rng.next()];
            run_parts(
                &mut t,
                &w,
                0x0090_ae50,
                Conv::Cdecl,
                &args,
                (ebp - 0x101c, ebp - 4),
                0xa5,
                // `[ebp-8]` holds `ecx` from `__alloca_probe`'s push
                &|_| vec![(ebp - 0x101c, ebp - 8)],
                |g| bg::yi_assert(g, ebp - 0x101c, abort, part, format, ebp + 0x14),
            );
        }
    }
}

/// `gdraw_filterPass3_00a38c40`: as `bf_filter_pass`, with each of the
/// three inputs present or not, random shader indices and constants.
#[test]
fn bg_filter_pass3() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f103);
        let warn = t.stub_code(0x0bad_f106, false, 2, 0, false, &[]);
        let ctx = com(
            &mut t,
            0xf200,
            &[
                (0xc8, 3, &[]),
                (0x20, 4, &[]),
                (0x28, 4, &[]),
                (0x24, 4, &[]),
                (0x3c, 3, &[]),
                (0x40, 4, &[]),
            ],
        );
        let mut tags = gdraw_begin_stubs(&mut t);
        t.stub(bg::TEXTURE_TRANSFORM, 2, false);
        tags.extend([
            bg::TEXTURE_TRANSFORM,
            0x0bad_f106,
            0xf2c8,
            0xf220,
            0xf228,
            0xf224,
            0xf23c,
            0xf240,
        ]);
        let state = t.zeros(0xb00);
        let ebp = t.ebp_for(5);
        let mut full = 0;
        for i in 0..800 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            gdraw_state(&mut t, &mut rng, state, ctx, warn);
            let target = gdraw_target(&mut t, &mut rng, state);
            let made = rng.pick(&[target, target, target, 0]);
            t.ret(bf::ALLOC_TARGET, made);
            let dst = t.zeros(0x50);
            t.ret(bf::MAP_BUFFER, dst);
            let rect = gdraw_rect(&mut t, &mut rng);
            let filter = t.zeros(0xc0);
            for k in 0..0x30 {
                let v = if (0x22..0x2e).contains(&k) {
                    rng.float()
                } else {
                    rng.next()
                };
                t.w32(filter + 4 * k, v);
            }
            let ins: Vec<u32> = (0..3)
                .map(|_| {
                    let b = random_block(&mut t, &mut rng, 0x30);
                    rng.pick(&[0, b])
                })
                .collect();
            for (k, v) in ins.iter().enumerate() {
                t.w32(filter + 0x1c + 4 * k as u32, *v);
            }
            let kind = rng.below(4);
            let (colour, counters) = (rng.next(), rng.next());
            let cell = t.zeros(16);
            t.w32(cell, colour);
            let colour = rng.pick(&[cell, cell, state + 0x6d8, state + 0x6dc]);
            run_parts(
                &mut t,
                &w,
                0x00a3_8c40,
                Conv::Cdecl,
                &[filter, rect, colour, kind, counters],
                (ebp - 0x2c, ebp),
                0xa5,
                &|_| vec![(ebp - 0x2c, ebp - 0x1c)],
                |g| bg::filter_pass3(g, ebp - 0x2c, filter, rect, colour, kind, counters),
            );
            let n = t.log_entries();
            let drawn = (0..n).any(|e| {
                let tag: u32 = Mem::read(&t.b, t.log + 4 + 64 * e);
                tag == bf::DRAW_QUAD
            });
            if drawn {
                full += 1;
                let mut parts = vec![(ebp - 0x1c, ebp - 0x10), (ebp - 8, ebp)];
                if ins[1] == 0 {
                    parts.push((ebp - 0x10, ebp - 0xc));
                }
                if ins[2] == 0 {
                    parts.push((ebp - 0xc, ebp - 8));
                }
                for (a, b) in parts {
                    let x = Oracle::read(&mut t.a, a, (b - a) as usize).unwrap();
                    let y = Oracle::read(&mut t.b, a, (b - a) as usize).unwrap();
                    assert_eq!(x, y, "{w}: frame {a:#x}..{b:#x}");
                }
            }
        }
        assert!(full > 100, "{name}: {full}");
    }
}

/// `gdraw_setRenderState_00a3c6c0`: both maps failing (out of memory or
/// another error), null or written; every flag, blend, kind, texture and
/// colour transform; the scissor on the screen's target or another and
/// the pending reset; both depth-stencil paths. The depth-stencil
/// helper's stub masks the two byte arguments (the original leaves stale
/// upper bytes there).
#[test]
fn bg_set_render_state() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f104);
        // four words: the out-of-memory warning passes no fifth
        let warn = t.stub_code(0x0bad_f107, false, 4, 0, false, &[]);
        let data = t.zeros(4);
        let (map, first) = alternating_stub(&mut t, 0xf338, false, 6, 6, &[wr(5, data)]);
        let ctx = com(
            &mut t,
            0xf300,
            &[
                (0x2c, 4, &[]),
                (0x3c, 3, &[]),
                (0x1c, 4, &[]),
                (0x8c, 4, &[]),
                (0x24, 4, &[]),
                (0x20, 4, &[]),
                (0x28, 4, &[]),
                (0x40, 4, &[]),
                (0xb4, 3, &[]),
                (0x90, 3, &[]),
            ],
        );
        let vt: u32 = Mem::read(&t.b, ctx);
        t.w32(vt + 0x38, map);
        let log = t.stub_code(bg::DEPTH_STENCIL_STATE, false, 4, 0, false, &[]);
        let mut c = vec![0x81, 0x64, 0x24, 0x0c, 0xff, 0, 0, 0]; // and dword [esp+0xc], 0xff
        c.extend([0x81, 0x64, 0x24, 0x10, 0xff, 0, 0, 0]); // and dword [esp+0x10], 0xff
        c.push(0xe9);
        let at = t.cave_next;
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let f = t.code(&c);
        t.jump(bg::DEPTH_STENCIL_STATE, f);
        let tags = [
            0x0bad_f107,
            0xf338,
            0xf32c,
            0xf33c,
            0xf31c,
            0xf38c,
            0xf324,
            0xf320,
            0xf328,
            0xf340,
            0xf3b4,
            0xf390,
            bg::DEPTH_STENCIL_STATE,
        ];
        let state = t.zeros(0xb00);
        let ebp = t.ebp_for(4);
        for i in 0..800 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(bb::GDRAW_WARNING.sym, warn);
            t.g32(bb::GDRAW_STATE.sym, state);
            for k in 0..0x2c0 {
                let v = rng.next();
                t.w32(state + 4 * k, v);
            }
            t.w32(state + 4, ctx);
            let hrs = [0, 0, 0, 1, 0x8007_000e, 0x8000_4005];
            let (h0, h1) = (rng.pick(&hrs), rng.pick(&hrs));
            t.w32(first, h0);
            t.ret(0xf338, h1);
            let buf = t.zeros(0x80);
            let d = rng.pick(&[buf, buf, buf, 0]);
            t.w32(data, d);
            for (off, v) in [
                (0x9c8, rng.pick(&[0, 1, 5, 6])),
                (0x77c, rng.pick(&[0, 1])),
                (0x9c4, rng.pick(&[0, 1])),
            ] {
                t.w32(state + off, v);
            }
            let screen = rng.pick(&[state + 0x9cc, state + 0x9d0]);
            t.w32(state + 0xaac, screen);
            let r = random_block(&mut t, &mut rng, 0x80);
            for k in [0x2c, 0x28] {
                let m = t.zeros(0x20);
                for j in 0..6 {
                    let v = rng.float();
                    t.w32(m + 4 * j, v);
                }
                t.w32(r + k, m);
            }
            for off in [0x7a4, 0x7a8] {
                let v = rng.float();
                t.w32(state + off, v);
            }
            let cx = t.zeros(8);
            for j in 0..4 {
                let v = rng.pick(&[0, 1, 255, 0x7fff, 0x8000, 0xff80]) as u16;
                t.w16(cx + 2 * j, v);
            }
            let cx = rng.pick(&[0, cx, cx]);
            t.w32(r + 0x18, cx);
            let tb = random_block(&mut t, &mut rng, 0x10);
            let tex = rng.pick(&[0, tb]);
            t.w32(r + 0x1c, tex);
            let flags = rng.next() & 0xf;
            t.w32(r + 4, flags);
            for (off, v) in [
                (8, rng.pick(&[0, 1])),
                (9, rng.pick(&[0, 1, 4, 5])),
                (0xa, rng.below(4)),
                (0xb, rng.below(4)),
                (0xc, rng.pick(&[0, 1, 5, 6])),
                (0xd, rng.below(8)),
                (0x10, rng.pick(&[0, 0, 0xff])),
                (0x11, rng.pick(&[0, 0, 0x7f])),
            ] {
                t.w8(r + off, v as u8);
            }
            let format = rng.below(4);
            let colour = random_block(&mut t, &mut rng, 0x10);
            let third = rng.next();
            t.constants(&mut rng, &[], &[0x00b7_a960, 0x00b1_70c0]);
            run_parts(
                &mut t,
                &w,
                0x00a3_c6c0,
                Conv::Cdecl,
                &[format, r, third, colour],
                (ebp - 0x64, ebp - 4),
                0xa5,
                &|_| {
                    vec![
                        (ebp - 0x64, ebp - 0x4c),
                        (ebp - 0x3c, ebp - 0x38),
                        (ebp - 0x34, ebp - 4),
                    ]
                },
                |g| bg::set_render_state(g, ebp - 0x64, format, r, colour),
            );
        }
    }
}

// ---- round 2, batch 3 (ffx_glue_other::h4b_h)

/// `TODrawMessageWaitNamesp`: random code streams (groups from a table:
/// line breaks, control codes with tabs, macros, glyphs) in every window
/// kind, on the maps with their own layouts; the window word's slot and
/// the frame compared.
#[test]
fn bh_message_wait_namesp() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f201);
        let groups: Vec<u8> = (0..=255u32)
            .map(|c| {
                if c == 9 {
                    2
                } else {
                    [4, 4, 4, 1, 2, 3, 4, 0, 4, 1][(c % 10) as usize]
                }
            })
            .collect();
        let table = t.alloc(&groups);
        let mut op = vec![0x0f, 0xb6, 0xc0, 0x0f, 0xb6, 0x80]; // movzx eax, al; movzx eax, [eax+table]
        op.extend(table.to_le_bytes());
        arg_stub(&mut t, bh::GET_GROUP, &op);
        arg_stub(&mut t, bh::SKIP_CODE, &[0x40]); // inc eax
        let albhed = t.zeros(4);
        t.stub_at(bh::CONV_ALBHED, false, 2, 0, &[wr(1, albhed)]);
        t.stub(e1::REMAP_X2, 1, true);
        t.stub(e1::REMAP_Y2, 1, true);
        t.stub(bh::MACRO_WIDTH, 3, true);
        let mut tags = vec![
            bh::GET_GROUP,
            bh::SKIP_CODE,
            bh::CONV_ALBHED,
            e1::REMAP_X2,
            e1::REMAP_Y2,
            bh::MACRO_WIDTH,
        ];
        for (sym, n) in [
            (bh::PKT_SCISSOR, 4),
            (bh::COLON_WIDTH, 2),
            (bh::CHANGE_MACRO2, 3),
            (bh::FIRST_INIT, 3),
            (bh::SET_DRAW_INFO, 3),
            (bh::MACRO_WORD_COUNT, 2),
            (bh::GET_FONT_INFO, 2),
            (bh::FONT1612, 2),
        ] {
            t.stub(sym, n, false);
            tags.push(sym);
        }
        let ebp = t.ebp_for(1);
        let mut lines7 = 0;
        for i in 0..500 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            for tag in [e1::REMAP_X2, e1::REMAP_Y2, bh::MACRO_WIDTH] {
                let v = rng.double();
                t.ret_f64(tag, v);
            }
            t.ret(bh::COLON_WIDTH, rng.below(40));
            t.ret(bh::MACRO_WORD_COUNT, rng.below(3));
            t.ret(bh::CONV_ALBHED, rng.below(4));
            let a = rng.next();
            t.w32(albhed, a);
            t.constants(
                &mut rng,
                &[
                    0x00c5_d784,
                    0x00b5_4c7c,
                    0x00b5_8cd8,
                    0x00b6_67e4,
                    0x00b6_58a4,
                    0x00b3_eb9c,
                    0x00b1_f2c8,
                    0x00b6_67e0,
                    0x00b6_67e8,
                    0x00b4_3dd4,
                ],
                &[
                    0x00b9_22c0,
                    0x00b9_2270,
                    0x00b3_13a0,
                    0x00b6_67d8,
                    0x00b4_1d20,
                    0x00b9_22b0,
                    0x00b2_c5c8,
                    0x00b5_af30,
                    0x00b4_9968,
                    0x00b9_2220,
                ],
            );
            let gmap = rng.pick(&[0xdf, 0x17e, 5]);
            t.g32(bh::GRAPHIC_MAP_ID.sym, gmap);
            let cmap = rng.pick(&[0xd4, 0x15b, 7]);
            t.g32(bh::CURRENT_MAP_NO.sym, cmap);
            let hide = rng.pick(&[0, 0, 0, 1]);
            t.g32(bh::HIDE_17E.sym, hide);
            let text: Vec<u8> = (0..600)
                .map(|_| match rng.below(8) {
                    0 => 9,
                    1 => rng.pick(&[0xbf, 0xc0, 0xc1]) as u8,
                    _ => rng.next() as u8,
                })
                .collect();
            let text = t.alloc(&text);
            let win = t.zeros(0x100);
            for k in 0..0x40 {
                let v = if (0x12..0x16).contains(&k) {
                    rng.float()
                } else {
                    rng.next()
                };
                t.w32(win + 4 * k, v);
            }
            t.w32(win + 0x94, text);
            let (x, y) = rng.pick(&[(0x1fe, 0x185), (-200, 0xb4), (-30, 0xb4), (3, 4)]);
            t.w16(win + 0x14, x as u16);
            t.w16(win + 0x16, y as u16);
            for (off, v) in [
                (0x22, rng.pick(&[0, 1, 2])),
                (0x24, rng.pick(&[0, 1, 0xffff])),
                (0x10, rng.pick(&[0x4000, 0])),
                (0xe, rng.pick(&[0, 1, 2])),
                (0x1a, rng.pick(&[0, 1, 3])),
                (4, rng.below(4)),
            ] {
                t.w16(win + off, v as u16);
            }
            for (off, v) in [
                (0xa0, rng.pick(&[0, 1, 5, 20, 40])),
                (0xa4, rng.below(12)),
                (0xa8, rng.below(50)),
                (0xac, rng.pick(&[0, 0, 1])),
                (0xb4, rng.pick(&[0, 0, 1])),
            ] {
                t.w32(win + off, v);
            }
            t.b.write(ebp + 8, &win.to_le_bytes()).unwrap();
            run_parts(
                &mut t,
                &w,
                0x008d_ee00,
                Conv::Cdecl,
                &[win],
                (ebp - 0x14, ebp),
                0xa5,
                &|_| vec![(ebp - 0x14, ebp), (ebp + 8, ebp + 0xc)],
                |g| bh::message_wait_namesp(g, ebp - 0x14, win, Some(ebp + 8)),
            );
            let lines: u32 = Mem::read(&t.b, ebp - 4);
            if lines >= 8 && cmap != 7 {
                lines7 += 1;
            }
        }
        assert!(lines7 > 10, "{name}: {lines7}");
    }
}

/// `AbmapDrawGridCells_00a497b0`: random marks and corner records
/// (ordinary floats, which serve as positions and sizes alike); each
/// draw's record captured.
#[test]
fn bh_grid_cells() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f202);
        t.stub(bh::SCREEN_QUAD, 0, false);
        capture_stub_n(&mut t, bh::DRAW_ABMAP_ELEMENT, 3, 0, 0x80, 128);
        let tags = [bh::SCREEN_QUAD, bh::DRAW_ABMAP_ELEMENT];
        let ebp = t.ebp_for(0);
        let abmap = t.zeros(0x13000);
        for i in 0..60 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.g32(bh::ABMAP.sym, abmap);
            for off in (0x11680..0x12e00).step_by(4) {
                let v = if rng.below(4) == 0 {
                    rng.float()
                } else {
                    rng.below(0x10000)
                };
                t.w32(abmap + off, v);
            }
            let density = rng.pick(&[2, 4, 10]);
            for k in 0..340 {
                let m = if rng.below(density) == 0 {
                    rng.pick(&[1, 2, 0x80, 0xff])
                } else {
                    0
                };
                t.w8(abmap + 0x12e60 + k, m);
            }
            let packet = t.zeros(0xd0);
            t.g32(bh::PACKET.sym, packet);
            t.constants(
                &mut rng,
                &[],
                &[0x00b9_21e0, 0x00b5_1bc0, 0x00b9_22f0, 0x00b5_4c70],
            );
            run_parts(
                &mut t,
                &w,
                0x00a4_97b0,
                Conv::Cdecl,
                &[],
                (ebp - 0x1a4, ebp - 4),
                0xa5,
                &|_| vec![(ebp - 0x19c, ebp - 4)],
                |g| bh::grid_cells(g, ebp - 0x1a4),
            );
        }
    }
}

/// A float stub at `sym` returning its `f32` argument times the `f64` in
/// the returned cell (so each call's result depends on what it was
/// handed); logs the argument.
fn scaled_stub(t: &mut Twins, sym: u32) -> u32 {
    let log = t.stub_code(sym, false, 1, 0, false, &[]);
    let k = t.zeros(8);
    let at = t.cave_next;
    let mut c = vec![0xff, 0x74, 0x24, 0x04, 0xe8]; // push [esp+4]; call log
    c.extend(log.wrapping_sub(at + 9).to_le_bytes());
    c.extend([0x83, 0xc4, 0x04, 0xd9, 0x44, 0x24, 0x04, 0xdc, 0x0d]); // add esp, 4; fld dword [esp+4]; fmul qword [k]
    c.extend(k.to_le_bytes());
    c.push(0xc3);
    let f = t.code(&c);
    t.jump(sym, f);
    k
}

/// `TODrawFrameShape9_008f9280`: random sizes, alphas (negative ones
/// through the unsigned fix-up) and constants; the remaps' results
/// follow their arguments; the three argument slots compared.
#[test]
fn bh_frame_shape9() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f203);
        let kx = scaled_stub(&mut t, e1::REMAP_X2);
        let ky = scaled_stub(&mut t, e1::REMAP_Y2);
        let mut tags = vec![e1::REMAP_X2, e1::REMAP_Y2];
        for (sym, n) in [
            (bh::SHAPE_UV_C2, 11),
            (bh::CROSS_BOX, 6),
            (bh::FRAME_CONTENTS, 6),
        ] {
            t.stub(sym, n, false);
            tags.push(sym);
        }
        let ebp = t.ebp_for(4);
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            // ordinary scales: a product past the f64 range stays finite
            // in the x87's registers but not in a port's f64
            for k in [kx, ky] {
                let v = rng.double();
                let v = if v.abs() > 1e100 { 1.5 } else { v };
                t.write(k, &v.to_le_bytes());
            }
            t.constants(
                &mut rng,
                &[
                    0x00b5_eff4,
                    0x00b5_83d8,
                    0x00b4_1d8c,
                    0x00b5_f384,
                    0x00b9_2308,
                    0x00b9_2268,
                    0x00b3_eb94,
                    0x00b5_f2f8,
                    0x00b5_ef68,
                    0x00b5_f348,
                ],
                &[
                    0x00b4_1d58,
                    0x00b4_1d30,
                    0x00b4_1d68,
                    0x00b4_1cc0,
                    0x00b5_4d60,
                    0x00b5_f1d8,
                    0x00b5_8cd0,
                    0x00b4_c170,
                ],
            );
            let k128 = rng.pick(&[1.0 / 128.0, 1.0 / 128.0, 0.5, 1e10]);
            let at = t.rebase(0x00b1_70b8);
            t.write(at, &f64::to_le_bytes(k128));
            let alpha = rng.pickr(&[0, 1, 0x40, 0x80, 0xff, 0x8000_0000, 0xffff_ffff]);
            let args = [rng.next(), rng.float(), rng.float(), alpha];
            for (k, v) in args.iter().enumerate() {
                t.b.write(ebp + 8 + 4 * k as u32, &v.to_le_bytes()).unwrap();
            }
            run_parts(
                &mut t,
                &w,
                0x008f_9280,
                Conv::Cdecl,
                &args,
                (ebp - 0x30, ebp),
                0xa5,
                &|_| vec![(ebp + 0xc, ebp + 0x18)],
                |g| bh::frame_shape9(g, args[0], args[1], args[2], args[3], Some(ebp + 8)),
            );
        }
    }
}

// ---- round 2, batch 4 (ffx_glue_other::h4b_i)

/// `TODrawFrameShape_008f9c20`: every kind, random frames and constants;
/// the remaps' results follow their arguments; the argument slots
/// compared.
#[test]
fn bi_frame_shape() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f301);
        let kx = scaled_stub(&mut t, e1::REMAP_X2);
        let ky = scaled_stub(&mut t, e1::REMAP_Y2);
        t.stub(bi::SHAPE_UV, 9, false);
        let tags = [e1::REMAP_X2, e1::REMAP_Y2, bi::SHAPE_UV];
        let ebp = t.ebp_for(5);
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            for k in [kx, ky] {
                let v = rng.double();
                let v = if v.abs() > 1e100 { 1.5 } else { v };
                t.write(k, &v.to_le_bytes());
            }
            t.constants(
                &mut rng,
                &[
                    0x00b3_eb94,
                    0x00b9_2308,
                    0x00b4_1d80,
                    0x00b6_5818,
                    0x00b6_5800,
                    0x00b6_581c,
                    0x00b6_5838,
                    0x00b6_5610,
                    0x00b5_f0d4,
                    0x00b6_6bd4,
                    0x00b6_5834,
                    0x00b6_561c,
                    0x00b6_6bdc,
                    0x00b6_6be0,
                    0x00b6_6e30,
                    0x00b6_6e34,
                    0x00b3_eb9c,
                    0x00b6_6be4,
                    0x00b6_5628,
                    0x00b4_936c,
                    0x00b6_580c,
                    0x00b4_1ca0,
                    0x00b6_5810,
                    0x00b6_5828,
                    0x00b6_6bd0,
                    0x00b6_5814,
                ],
                &[],
            );
            let kind = rng.pick(&[0, 1, 2, 3, 4, 5, 6, 0xffff_ffff]);
            let args = [rng.float(), rng.float(), rng.float(), rng.float(), kind];
            for (k, v) in args.iter().enumerate() {
                t.b.write(ebp + 8 + 4 * k as u32, &v.to_le_bytes()).unwrap();
            }
            run_parts(
                &mut t,
                &w,
                0x008f_9c20,
                Conv::Cdecl,
                &args,
                (ebp - 0x18, ebp),
                0xa5,
                &|_| vec![(ebp + 8, ebp + 0x1c)],
                |g| bi::frame_shape(g, args[0], args[1], args[2], args[3], kind, Some(ebp + 8)),
            );
        }
    }
}
