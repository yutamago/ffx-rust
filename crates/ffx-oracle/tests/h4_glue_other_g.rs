//! Continuation round 11 of list `h4-glue-other` (`ffx_glue_other::h4_m*`)
//! against the original: on the image alone and on a snapshot
//! (`FFX_SNAPSHOT`). Results and every byte written must agree. Needs
//! `FFX_GAME_DIR`. The helpers are those of `h4_glue_other_f.rs`.
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

use ffx_bridge::Mem;
use ffx_glue_other::{
    h4_m1 as m1, h4_m2 as m2, h4_m3 as m3, h4_m4 as m4, h4_m5 as m5, h4_m6 as m6, h4_m7 as m7,
    h4_m8 as m8, h4_m9 as m9,
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
            if let Some(k) = (0..len).find(|&k| x[k] != y[k]) {
                let end = (k + 16).min(len);
                panic!(
                    "{what}: frame +{:#x} differs: original {:x?}, port {:x?}",
                    off as usize + k,
                    &x[k..end],
                    &y[k..end]
                );
            }
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

/// A string in fresh memory; its address.
fn string(t: &mut Twins, s: &[u8]) -> u32 {
    let mut b = s.to_vec();
    b.push(0);
    t.alloc(&b)
}

/// A `jmp rel32` from the cave's next byte (after `c`'s bytes) to `to`.
fn jmp_to(t: &Twins, c: &mut Vec<u8>, to: u32) {
    c.push(0xe9);
    let at = t.cave_next + c.len() as u32 + 4;
    c.extend(to.wrapping_sub(at).to_le_bytes());
}

/// A recording stub at `sym` that, before logging, stores fresh cells at
/// `[ecx + offsets[i]]` (a method filling its object). Returns the cells.
fn stub_filling(t: &mut Twins, sym: u32, n: u32, pops: u32, offsets: &[u8]) -> Vec<u32> {
    let rest = t.stub_code(sym, true, n, pops, false, &[]);
    let cells: Vec<u32> = offsets.iter().map(|_| t.zeros(4)).collect();
    let mut c = Vec::new();
    for (&off, &cell) in offsets.iter().zip(&cells) {
        c.push(0xa1);
        c.extend(cell.to_le_bytes()); // mov eax, [cell]
        c.extend([0x89, 0x41, off]); // mov [ecx+off], eax
    }
    jmp_to(t, &mut c, rest);
    let f = t.code(&c);
    t.jump(sym, f);
    cells
}

/// A recording stub at `sym` that picks its stub by its stack word `word`:
/// `cases[i].0` goes to a stub tagged `cases[i].1`, anything else to one
/// tagged `sym`.
#[allow(clippy::too_many_arguments)]
fn stub_by_arg(
    t: &mut Twins,
    sym: u32,
    this: bool,
    n: u32,
    pops: u32,
    word: u8,
    cases: &[(u32, u32)],
) {
    let other = t.stub_code(sym, this, n, pops, false, &[]);
    let targets: Vec<(u32, u32)> = cases
        .iter()
        .map(|&(v, tag)| (v, t.stub_code(tag, this, n, pops, false, &[])))
        .collect();
    let mut c = Vec::new();
    for (v, to) in targets {
        c.extend([0x81, 0x7c, 0x24, 4 + 4 * word]);
        c.extend(v.to_le_bytes()); // cmp dword [esp+4+4*word], v
        c.extend([0x0f, 0x84]);
        let at = t.cave_next + c.len() as u32 + 4;
        c.extend(to.wrapping_sub(at).to_le_bytes()); // je to
    }
    jmp_to(t, &mut c, other);
    let f = t.code(&c);
    t.jump(sym, f);
}

/// `Phyre_PRedBlackTree_successor` as a stub: returns the node's `+0x1c`.
fn stub_successor(t: &mut Twins, sym: u32) {
    let rest = t.stub_code(sym, true, 1, 1, false, &[]);
    let cell = t.cells[&sym];
    let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x40, 0x1c, 0xa3];
    c.extend(cell.to_le_bytes()); // mov eax, [esp+4]; mov eax, [eax+0x1c]; mov [cell], eax
    jmp_to(t, &mut c, rest);
    let f = t.code(&c);
    t.jump(sym, f);
}

/// The original `DJB_hash` of the string at `s`, run in both oracles.
fn djb(t: &mut Twins, s: u32) -> u32 {
    let f = t.rebase(m1::DJB_HASH);
    let x = t.a.call(f, Conv::Cdecl, &[s]).unwrap().eax;
    assert_eq!(t.b.call(f, Conv::Cdecl, &[s]).unwrap().eax, x);
    x
}

// ---- batch 1 (ffx_glue_other::h4_m1)

/// `MenuDynamicGeometry_setShaderClusterAndMaterial`: the cursor finding
/// the shader or not; the name in the tree (loaded or not, the first of
/// two alike), not there, `"FrameBuffer"` (whose hash never matches) and
/// `"RealFrameBuffer"` (the post-process buffers, set or flagged); `load`
/// 0 or not; texture clusters found or not; each parameter missing; the
/// texture binding failing; material arrays empty, inline and pointed.
#[test]
fn m1_set_shader_cluster_and_material() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_a311);
        let cursor = stub_filling(&mut t, m1::CURSOR_SEEK, 0, 0, &[0, 0xc, 0x10]);
        let iter = stub_filling(&mut t, m1::TEXTURE_ITERATOR, 1, 1, &[0, 0x10]);
        stub_successor(&mut t, m1::TREE_SUCCESSOR);
        let texture = t.rebase(m1::TEXTURE_PARAM);
        let sampler = t.rebase(m1::SAMPLER_PARAM);
        stub_by_arg(
            &mut t,
            m1::FIND_PARAM,
            true,
            1,
            1,
            0,
            &[(texture, m1::FIND_PARAM + 1), (sampler, m1::FIND_PARAM + 2)],
        );
        for (sym, this, n, pops) in [
            (m1::CLUSTER_MANAGER, false, 0, 0),
            (m1::CLUSTER_BY_NAME, true, 1, 1),
            (m1::LINEAR_SAMPLER, true, 1, 1),
            (m1::SET_VALUE_DESC, true, 2, 2),
            (m1::SURFACE_FORMAT, true, 2, 2),
            (m1::CREATE_MATERIAL, true, 4, 4),
            (m1::SET_VALUE, true, 2, 2),
            (m1::POST_PROCESS, false, 0, 0),
            (m1::OPERATOR_NEW, false, 1, 0),
            (m1::TREE_ADD, true, 2, 2),
        ] {
            t.stub_at(sym, this, n, pops, &[]);
        }
        let tags = [
            m1::CURSOR_SEEK,
            m1::TEXTURE_ITERATOR,
            m1::TREE_SUCCESSOR,
            m1::FIND_PARAM,
            m1::FIND_PARAM + 1,
            m1::FIND_PARAM + 2,
            m1::CLUSTER_MANAGER,
            m1::CLUSTER_BY_NAME,
            m1::LINEAR_SAMPLER,
            m1::SET_VALUE_DESC,
            m1::SURFACE_FORMAT,
            m1::CREATE_MATERIAL,
            m1::SET_VALUE,
            m1::TREE_ADD,
        ];
        let manager = t.zeros(0x1_2340);
        let tree = t.rebase(ffx_glue_other::h4_c3::MENU_TEXTURE_TREE);
        let names: Vec<u32> = [
            &b"menu_tex_a"[..],
            b"menu_tex_b",
            b"menu_tex_c",
            b"FrameBuffer",
            b"RealFrameBuffer",
        ]
        .iter()
        .map(|s| string(&mut t, s))
        .collect();
        let hashes: Vec<u32> = names.clone().iter().map(|&s| djb(&mut t, s)).collect();
        let mut seen = [0u32; 6];
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            // the tree: 0..4 nodes over names 0..2 and FrameBuffer's hash
            let n = rng.below(5) as usize;
            let nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x20)).collect();
            for (k, &nd) in nodes.iter().enumerate() {
                let record = t.zeros(8);
                let h = hashes[rng.pick(&[0, 1, 2, 3])];
                t.w32(record, h);
                t.w32(record + 4, rng.pick(&[0, 1, 1, 2]));
                t.w32(nd + 0xc, record);
                let material = random_block(&mut t, &mut rng, 8);
                t.w32(nd + 0x10, material);
                t.w32(nd + 0x1c, nodes.get(k + 1).copied().unwrap_or(tree));
            }
            match nodes.first() {
                Some(&first) => {
                    let top = t.zeros(0x20);
                    t.w32(tree + 0xc, rng.pick(&[first, top]));
                    t.w32(top, first);
                    t.w32(first, tree);
                }
                None => t.w32(tree + 0xc, tree),
            }
            // the geometry
            let this = random_block(&mut t, &mut rng, 0x30);
            let count = rng.pick(&[0, 1, 2, 0x8000_0001, 0x8000_0003, 0x8000_0000]);
            let array = random_block(&mut t, &mut rng, 8);
            t.w32(array, count);
            let elements = random_block(&mut t, &mut rng, 8);
            t.w32(array + 4, elements);
            t.w32(this + 0x14, array);
            let owner = t.zeros(0x20);
            let other = t.zeros(0x20);
            let head = rng.pick(&[owner + 0x1c, other]);
            t.w32(owner + 0x1c, head);
            t.w32(this + 0x20, owner);
            let instance = t.zeros(0x98);
            let info = random_block(&mut t, &mut rng, 0x28);
            t.w32(instance + 0x94, info);
            t.w32(this + 0x24, instance);
            let scene = t.zeros(4);
            t.g32(m1::PHYRE_SCENE, scene);
            let new_owner = t.zeros(0x20);
            t.w32(new_owner + 0x1c, rng.pick(&[new_owner + 0x1c, other]));
            let cluster = rng.pick(&[0, 0, new_owner]);
            // the cursor's find
            let base = t.zeros(0x40);
            let off = 4 * rng.below(8);
            let shader = rng.next();
            t.w32(base + off, shader);
            let found = rng.pick(&[0, 1, 1, 1, 1, 1, 1, 1]);
            t.w32(cursor[0], off);
            t.w32(cursor[1], found);
            t.w32(cursor[2], base);
            let (a, b) = (rng.next(), rng.next());
            t.w32(iter[0], a);
            t.w32(iter[1], b);
            // the callees' results
            let (x, y) = (rng.next(), rng.next());
            let texture_cluster = rng.pick(&[0, x, y]);
            t.ret(m1::CLUSTER_BY_NAME, texture_cluster);
            for k in 0..3 {
                let (x, y) = (rng.next(), rng.next());
                let v = rng.pick(&[0, x, y]);
                t.ret(m1::FIND_PARAM + k, v);
            }
            t.ret(m1::SURFACE_FORMAT, rng.pick(&[0, 0, 0, 5, 0x15]));
            let material = random_block(&mut t, &mut rng, 8);
            t.ret(m1::CREATE_MATERIAL, material);
            let new = random_block(&mut t, &mut rng, 8);
            t.ret(m1::OPERATOR_NEW, new);
            t.ret(m1::POST_PROCESS, manager);
            for field in [0x1_232c, 0x9c] {
                let buffer = random_block(&mut t, &mut rng, 8);
                let flag = rng.pick(&[0, 0, 1, 0x100]);
                t.w32(buffer + 4, flag);
                t.w32(manager + field, buffer);
            }
            let which = rng.below(5) as usize;
            let text = names[which];
            let load = rng.pick(&[0, 1, 1, 0x100]);
            let ebp = t.ebp_for(3);
            let frame = ebp - 0x4c;
            let r = t.check_framed_conv(
                &format!("{w} name {which} load {load:#x} cluster {cluster:#x} nodes {n}"),
                0x006a_3100,
                Conv::Thiscall(this),
                &[cluster, text, load],
                frame,
                &[(0, 0x4c), (0x54, 0xc)],
                |g| {
                    u(m1::set_shader_cluster_and_material(
                        g, frame, this, cluster, text, load,
                    ))
                },
            );
            let slot = match r {
                0 => 0,
                1 => 1,
                3 => 2,
                5 | 0x15 => 3,
                0x13 => 4,
                _ => 5,
            };
            seen[slot] += 1;
        }
        eprintln!("{name}: results {seen:?}");
        assert!(seen[..5].iter().all(|&k| k > 0), "{name}: {seen:?}");
    }
}

/// A ring map stub at `sym` (thiscall, one word, ret 4): stores `cells.0`
/// through its pointer argument, or `cells.1` when that is `second`.
fn stub_map(t: &mut Twins, sym: u32, second: u32) -> (u32, u32) {
    let rest = t.stub_code(sym, true, 1, 1, false, &[]);
    let (a, b) = (t.zeros(4), t.zeros(4));
    let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x15];
    c.extend(a.to_le_bytes()); // mov eax, [esp+4]; mov edx, [a]
    c.push(0x3d);
    c.extend(second.to_le_bytes()); // cmp eax, second
    c.extend([0x75, 0x06, 0x8b, 0x15]);
    c.extend(b.to_le_bytes()); // jne +6; mov edx, [b]
    c.extend([0x89, 0x10]); // mov [eax], edx
    jmp_to(t, &mut c, rest);
    let f = t.code(&c);
    t.jump(sym, f);
    (a, b)
}

/// A `PArray` header with a random count (none, one owned or not, three)
/// over random words; its address.
fn parray(t: &mut Twins, rng: &mut Rng) -> u32 {
    let a = random_block(t, rng, 8);
    let count = rng.pick(&[0, 1, 1, 3, 0x8000_0001, 0x8000_0003]);
    t.w32(a, count);
    a
}

/// `SkipPrompt_buildQuad_006e7940`: mesh, material list and instance
/// there or created (arrays of 0, 1 (owned or not) or 3 elements, failed
/// allocations), the segment's streams there or made, the sizes changed
/// or not (bound or not), the unmaps failing; random rectangles and
/// screen constants (NaN, infinities).
#[test]
fn m2_build_quad() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e794);
        let materials_type = t.rebase(m2::MATERIALS_TYPE);
        stub_by_arg(
            &mut t,
            m2::CLUSTER_CREATE,
            true,
            3,
            3,
            1,
            &[(materials_type, m2::CLUSTER_CREATE + 1)],
        );
        stub_by_arg(
            &mut t,
            m2::MALLOC,
            false,
            2,
            0,
            0,
            &[(0x80, m2::MALLOC + 1)],
        );
        let ebp = t.ebp_for(8);
        let frame = ebp - 0x20;
        let maps = stub_map(&mut t, m2::MAP, frame);
        for (sym, this, n, pops) in [
            (m2::SEGMENTS_CONSTRUCT, false, 2, 0),
            (m2::SEGMENTS_DESTROY, false, 2, 0),
            (m2::FREE, false, 1, 0),
            (m2::ALLOCATE_INSTANCES, true, 3, 3),
            (m2::MESH_INSTANCE_CTOR, true, 1, 1),
            (m2::STREAM_CTOR, true, 0, 0),
            (m2::SET_BUFFER, true, 2, 2),
            (m2::BUFFER_CTOR, true, 3, 3),
            (m2::IS_BOUND, true, 0, 0),
            (m2::RELEASE, true, 0, 0),
            (m2::UNMAP, true, 2, 2),
        ] {
            t.stub_at(sym, this, n, pops, &[]);
        }
        let tags = [
            m2::CLUSTER_CREATE,
            m2::CLUSTER_CREATE + 1,
            m2::MALLOC,
            m2::MALLOC + 1,
            m2::MAP,
            m2::SEGMENTS_CONSTRUCT,
            m2::SEGMENTS_DESTROY,
            m2::FREE,
            m2::ALLOCATE_INSTANCES,
            m2::MESH_INSTANCE_CTOR,
            m2::STREAM_CTOR,
            m2::SET_BUFFER,
            m2::BUFFER_CTOR,
            m2::IS_BOUND,
            m2::RELEASE,
            m2::UNMAP,
        ];
        let mut made = 0;
        for i in 0..500 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(
                &mut rng,
                &[],
                &[m2::SCREEN_WIDTH, m2::SCREEN_HEIGHT, m2::TWO],
            );
            if rng.below(2) == 0 {
                for (c, v) in [
                    (m2::SCREEN_WIDTH, 1920.0f64),
                    (m2::SCREEN_HEIGHT, 1080.0),
                    (m2::TWO, 2.0),
                ] {
                    let at = t.rebase(c);
                    t.write(at, &v.to_le_bytes());
                }
            }
            let scene = t.zeros(0x1300);
            t.g32(m2::PHYRE_SCENE, scene);
            // the segment and its streams
            let segment = random_block(&mut t, &mut rng, 0x30);
            if rng.below(3) == 0 {
                t.w32(segment + 0x28, 2);
            }
            let streams = random_block(&mut t, &mut rng, 0x80);
            for s in [streams, streams + 0x40] {
                let desc = random_block(&mut t, &mut rng, 0xc);
                t.w32(s + 0xc, desc);
                let (a, b) = (rng.below(8), rng.below(8));
                t.w32(s, a);
                t.w32(s + 4, b);
                let size = rng.pick(&[a * b, a * b, 7]);
                t.w32(s + 0x38, size);
                t.w32(s + 0x18, rng.pick(&[0, 1, 2, 0x8000_0002]));
                t.w32(s + 0x10, rng.pick(&[0, 1, 0xffff_ffff]));
            }
            t.w32(segment + 0x2c, streams);
            // the mesh: there, or created (the new data the segment)
            let mesh = t.zeros(4);
            let mesh_array = parray(&mut t, &mut rng);
            t.w32(mesh_array + 4, segment);
            let created = parray(&mut t, &mut rng);
            let x = rng.next();
            t.w32(created + 4, rng.pick(&[segment, x]));
            if rng.below(2) == 0 {
                t.w32(mesh, mesh_array);
            }
            t.ret(m2::CLUSTER_CREATE, created);
            t.ret(m2::MALLOC, rng.pick(&[segment, segment, segment, 0]));
            let x = rng.next() | 0x10_0000;
            t.ret(m2::MALLOC + 1, rng.pick(&[0, x]));
            // the material list
            let materials = t.zeros(4);
            let list = parray(&mut t, &mut rng);
            let elements = random_block(&mut t, &mut rng, 12);
            t.w32(list + 4, elements);
            if rng.below(2) == 0 {
                t.w32(materials, list);
                let x = rng.next();
                t.ret(m2::CLUSTER_CREATE + 1, x);
            } else {
                t.ret(m2::CLUSTER_CREATE + 1, list);
            }
            // the instance
            let instance = t.zeros(4);
            let inst = random_block(&mut t, &mut rng, 0x14);
            let made_list = random_block(&mut t, &mut rng, 0x38);
            let ctor_this = rng.next();
            t.w32(
                made_list + 0x34,
                rng.pick(&[ctor_this, ctor_this, ctor_this, 0]),
            );
            let mut ok = true;
            if rng.below(2) == 0 {
                t.w32(instance, inst);
            } else {
                let list = rng.pick(&[made_list, made_list, made_list, 0]);
                t.ret(m2::ALLOCATE_INSTANCES, list);
                t.ret(m2::MESH_INSTANCE_CTOR, inst);
                // no instance: the original reads address 0
                ok = list != 0 && Mem::read::<u32>(&t.b, made_list + 0x34) != 0;
            }
            // the mesh's segment must be readable: the original reads it
            let mesh_now: u32 = Mem::read(&t.b, mesh);
            if mesh_now == 0 {
                let c: u32 = Mem::read(&t.b, created);
                let seg: u32 = Mem::read(&t.b, created + 4);
                let m: u32 = Mem::read(&t.b, t.cells[&m2::MALLOC]);
                ok &= if c == 1 {
                    seg == segment
                } else {
                    m == segment || (m == 0 && seg == segment)
                };
            }
            if !ok {
                continue;
            }
            let pos = t.zeros(0x48);
            let uv = t.zeros(0x30);
            t.w32(maps.0, pos);
            t.w32(maps.1, uv);
            t.ret(m2::IS_BOUND, rng.pick(&[0, 1, 0x100, 0xffff_ff00]));
            t.ret(m2::UNMAP, rng.pick(&[0, 0, 1]));
            let rect: [u32; 4] = std::array::from_fn(|_| rng.float());
            let material = rng.next();
            let args = [
                rect[0], rect[1], rect[2], rect[3], mesh, instance, materials, material,
            ];
            t.check_framed_conv(
                &w,
                0x006e_7940,
                Conv::Stdcall,
                &args,
                frame,
                &[(0, 4), (8, 4)],
                |g| {
                    u(m2::build_quad(
                        g, frame, rect, mesh, instance, materials, material,
                    ))
                },
            );
            made += 1;
        }
        assert!(made > 150, "{name}: {made}");
    }
}

/// `PhyFMVPlayerManager_initialize`: a player running or not, window
/// sizes above 2^31, the map 0xb8 or another, ids listed as unskippable
/// or not (and the three always-skippable ones), names empty or not, the
/// texture registered or failing, wide or narrow, textures holding 3
/// levels or not (bound, allocations failing); random size constants.
#[test]
fn m3_initialize() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_d9c8);
        let path_cell = t.zeros(4);
        for (sym, this, n, pops, writes) in [
            (m3::DESTROY_PLAYER, true, 0, 0, vec![]),
            (m3::GET_APPLICATION, false, 0, 0, vec![]),
            (m3::VIDEO_NAME, true, 2, 2, vec![]),
            (m3::SAVE_CONFIG, false, 1, 1, vec![]),
            (m3::REGISTER_TEXTURE, true, 0, 0, vec![]),
            (m3::SET_DIMENSIONS, true, 4, 4, vec![]),
            (m3::UPDATE_MAX_LEVEL, true, 0, 0, vec![]),
            (m3::IS_BOUND, true, 0, 0, vec![]),
            (m3::ALLOCATE_LEVELS, true, 1, 1, vec![]),
            (m3::UNIFY_FILENAME, false, 3, 0, vec![wr(1, path_cell)]),
            (m3::ASYNC_LOAD_MANAGER, false, 0, 0, vec![]),
            (m3::LOAD_CLUSTER, false, 7, 0, vec![]),
            (m3::PLAYER_INITIALIZE, false, 1, 1, vec![]),
            (m3::SET_UVS, true, 1, 1, vec![]),
        ] {
            t.stub_at(sym, this, n, pops, &writes);
        }
        stub_import(&mut t, m3::SPRINTF.sym, 2);
        // the printf logs the id after the missing-name message only
        let no_name = t.rebase(m3::NO_NAME_MESSAGE);
        stub_by_format(&mut t, m3::PHYRE_PRINTF, 1, no_name, 3, 2);
        let tags = [
            m3::PHYRE_PRINTF,
            m3::DESTROY_PLAYER,
            m3::VIDEO_NAME,
            m3::SAVE_CONFIG,
            m3::UPDATE_MAX_LEVEL,
            m3::ALLOCATE_LEVELS,
            m3::UNIFY_FILENAME,
            m3::ASYNC_LOAD_MANAGER,
            m3::LOAD_CLUSTER,
            m3::PLAYER_INITIALIZE,
            m3::SET_UVS,
            m3::SET_DIMENSIONS,
            m3::SPRINTF.sym,
        ];
        let listed: Vec<u32> = (0..16)
            .map(|k| {
                let at = t.rebase(m3::UNSKIPPABLE) + 4 * k;
                Mem::read(&t.b, at)
            })
            .collect();
        let mut seen = [0u32; 3];
        let high = 0x9000_0000u32;
        t.a.map_zeroed(high, 0x1000).unwrap();
        t.b.map_zeroed(high, 0x1000).unwrap();
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(
                &mut rng,
                &[
                    m3::TWO_POW_32,
                    m3::WIDTH_WIDE,
                    m3::HEIGHT_WIDE,
                    m3::WIDTH_NARROW,
                    m3::HEIGHT_NARROW,
                ],
                &[m3::HALF],
            );
            if rng.below(2) == 0 {
                for (c, v) in [
                    (m3::TWO_POW_32, 4_294_967_296.0f32),
                    (m3::WIDTH_WIDE, 1920.0),
                    (m3::HEIGHT_WIDE, 1080.0),
                    (m3::WIDTH_NARROW, 1280.0),
                    (m3::HEIGHT_NARROW, 720.0),
                ] {
                    t.g32(c, v.to_bits());
                }
                let at = t.rebase(m3::HALF);
                t.write(at, &0.5f64.to_le_bytes());
            }
            let mut this = random_block(&mut t, &mut rng, 0x750);
            if rng.below(4) == 0 {
                // a manager above 2^31: the load's argument is sign-extended
                for k in 0..0x750 / 4 {
                    let v = rng.next();
                    t.w32(high + 4 * k, v);
                }
                this = high;
            }
            t.w8(this + 0x6d2, rng.pick(&[0, 0, 1, 0x80]));
            t.w8(this + 0x4f0, rng.pick(&[0, b'm', b'm', b'm']));
            t.w32(this + 0x380, rng.pick(&[0, 0x80, 0xffff_ff7f, 0xffff_ffff]));
            for tex in [this + 0x38c, this + 0x400, this + 0x474] {
                t.w32(
                    tex + 0x24,
                    rng.pick(&[0, 3, 3, 0x8000_0003, 2, 0x8000_0000]),
                );
            }
            let app = random_block(&mut t, &mut rng, 0x280);
            for off in [0x26c, 0x270] {
                let x = rng.next();
                let v = rng.pick(&[1920, 1080, 0x8000_0000, 0xffff_ffff, x]);
                t.w32(app + off, v);
            }
            t.ret(m3::GET_APPLICATION, app);
            let map = rng.pick(&[0xb8, 0xb8, 0x41, 0x1b8]);
            t.g32(m3::CURRENT_MAP, map);
            let (l, x) = (listed[rng.below(16) as usize], rng.below(0x80));
            let id = rng.pick(&[0x2e, 0x48, 0x20, l, x]);
            t.ret(m3::REGISTER_TEXTURE, rng.pick(&[0, 0, 0, 0, 7]));
            t.ret(m3::IS_BOUND, rng.pick(&[0, 1, 0x100]));
            let x = rng.next();
            t.ret(m3::ALLOCATE_LEVELS, rng.pick(&[0, x]));
            let x = rng.next();
            t.w32(path_cell, x);
            let scene = t.zeros(0x10de4);
            let icon = rng.next();
            t.w32(scene + 0x10de0, icon);
            t.g32(m3::PHYRE_SCENE, scene);
            let path = rng.next();
            t.g32(m3::SHADER_CLUSTER_PATH, path);
            let wide = rng.pick(&[0, 1, 0x100, 0xffff_ff00]);
            let flag = rng.pick(&[0, 1, 0x100]);
            let ebp = t.ebp_for(3);
            let frame = ebp - 0x104;
            let r = t.check_framed_conv(
                &format!("{w} id {id:#x} wide {wide:#x}"),
                0x006d_9c80,
                Conv::Thiscall(this),
                &[id, wide, flag],
                frame,
                &[(0, 0x100)],
                |g| u(m3::initialize(g, frame, this, id, wide, flag)),
            );
            let n = t.log_entries();
            seen[if r != 0 {
                0
            } else if n > 12 {
                1
            } else {
                2
            }] += 1;
        }
        eprintln!("{name}: {seen:?}");
        assert!(seen.iter().all(|&k| k > 0), "{name}: {seen:?}");
    }
}

/// A recording stub at `sym` (a varargs cdecl function) that logs
/// `n_match` words when its stack word `word` is `value`, else `n_else`.
fn stub_by_format(t: &mut Twins, sym: u32, word: u8, value: u32, n_match: u32, n_else: u32) {
    let other = t.stub_code(sym, false, n_else, 0, false, &[]);
    let matched = t.stub_code(sym, false, n_match, 0, false, &[]);
    let mut c = vec![0x81, 0x7c, 0x24, 4 + 4 * word];
    c.extend(value.to_le_bytes()); // cmp dword [esp+4+4*word], value
    c.extend([0x0f, 0x85]);
    let at = t.cave_next + c.len() as u32 + 4;
    c.extend(other.wrapping_sub(at).to_le_bytes()); // jne other
    jmp_to(t, &mut c, matched);
    let f = t.code(&c);
    t.jump(sym, f);
}

/// `PSharray_setWord_0048c540` as a stub that also stores its value in the
/// array's inline element (`[ecx+4]`; the lists here hold one).
fn stub_set_word(t: &mut Twins, sym: u32) {
    let rest = t.stub_code(sym, true, 2, 2, false, &[]);
    let mut c = vec![0x8b, 0x44, 0x24, 0x08, 0x89, 0x41, 0x04]; // mov eax, [esp+8]; mov [ecx+4], eax
    jmp_to(t, &mut c, rest);
    let f = t.code(&c);
    t.jump(sym, f);
}

/// `LoadingScreen_prepareMesh`: inputs missing (an empty texture list, no
/// texture, no variant) or there; created arrays already of one element
/// or not (owned or not), failed allocations, streams there or made,
/// sizes changed (bound or not), unmaps failing; random constants.
#[test]
fn m4_prepare_mesh() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_e492);
        let materials_type = t.rebase(m2::MATERIALS_TYPE);
        stub_by_arg(
            &mut t,
            m2::CLUSTER_CREATE,
            true,
            3,
            3,
            1,
            &[(materials_type, m2::CLUSTER_CREATE + 1)],
        );
        stub_by_arg(
            &mut t,
            m2::MALLOC,
            false,
            2,
            0,
            0,
            &[(0x80, m2::MALLOC + 1)],
        );
        let texture = t.rebase(m4::TEXTURE_PARAM);
        let colour = t.rebase(m4::COLOUR_PARAM);
        stub_by_arg(
            &mut t,
            m4::FIND_PARAM,
            true,
            1,
            1,
            0,
            &[(texture, m4::FIND_PARAM + 1), (colour, m4::FIND_PARAM + 2)],
        );
        stub_set_word(&mut t, m4::SET_WORD);
        let ebp = t.ebp_for(0);
        let frame = ebp - 0x60;
        let maps = stub_map(&mut t, m2::MAP, frame + 0x30);
        // the records as the callees see them (the frame reuses them)
        stub_copying(&mut t, m4::SET_TEXTURE, true, 2, 2, &[1], 4);
        stub_copying(&mut t, m4::SET_VALUE, true, 2, 2, &[1], 4);
        for (sym, this, n, pops) in [
            (m2::SEGMENTS_CONSTRUCT, false, 2, 0),
            (m2::SEGMENTS_DESTROY, false, 2, 0),
            (m2::FREE, false, 1, 0),
            (m2::STREAM_CTOR, true, 0, 0),
            (m2::SET_BUFFER, true, 2, 2),
            (m2::BUFFER_CTOR, true, 3, 3),
            (m2::IS_BOUND, true, 0, 0),
            (m2::RELEASE, true, 0, 0),
            (m2::UNMAP, true, 2, 2),
            (m4::CREATE_TRIANGLE, false, 1, 0),
            (m4::CREATE_MATERIAL, true, 3, 3),
            (m4::ALLOCATE_INSTANCE, true, 1, 1),
            (m4::LINEAR_SAMPLER, true, 1, 1),
        ] {
            t.stub_at(sym, this, n, pops, &[]);
        }
        let tags = [
            m2::SEGMENTS_CONSTRUCT,
            m2::SEGMENTS_DESTROY,
            m2::FREE,
            m2::STREAM_CTOR,
            m2::SET_BUFFER,
            m2::BUFFER_CTOR,
            m2::RELEASE,
            m4::FIND_PARAM,
            m4::FIND_PARAM + 1,
            m4::FIND_PARAM + 2,
            m4::SET_TEXTURE,
            m4::SET_VALUE,
            m4::LINEAR_SAMPLER,
            m4::SET_WORD,
        ];
        let mut full = 0;
        for i in 0..400 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(
                &mut rng,
                &[
                    m4::ALPHA,
                    m4::MINUS_ONE,
                    m4::LEFT,
                    m4::RIGHT,
                    m4::TOP,
                    m4::BOTTOM,
                ],
                &[],
            );
            let this = random_block(&mut t, &mut rng, 0x48);
            let x = rng.next();
            t.w32(this + 0x20, rng.pick(&[0, 0x8000_0000, 1, 3, x]));
            let x = rng.next();
            t.w32(this + 0x2c, rng.pick(&[0, x, x, x, x]));
            let x = rng.next();
            t.w32(this + 0x30, rng.pick(&[0, x, x, x, x]));
            let textures = random_block(&mut t, &mut rng, 4);
            t.w32(this + 0x24, textures);
            let scene = t.zeros(4);
            t.g32(m4::PHYRE_SCENE, scene);
            let triangle = random_block(&mut t, &mut rng, 0x14);
            t.ret(m4::CREATE_TRIANGLE, triangle);
            let list = parray(&mut t, &mut rng);
            t.ret(m2::CLUSTER_CREATE + 1, list);
            let material = random_block(&mut t, &mut rng, 8);
            t.ret(m4::CREATE_MATERIAL, material);
            // the mesh and its segment
            let segment = random_block(&mut t, &mut rng, 0x30);
            if rng.below(3) == 0 {
                t.w32(segment + 0x28, 2);
            }
            let streams = random_block(&mut t, &mut rng, 0x80);
            for s in [streams, streams + 0x40] {
                let desc = random_block(&mut t, &mut rng, 0xc);
                t.w32(s + 0xc, desc);
                let (a, b) = (rng.below(8), rng.below(8));
                t.w32(s, a);
                t.w32(s + 4, b);
                let size = rng.pick(&[a * b, a * b, 7]);
                t.w32(s + 0x38, size);
                t.w32(s + 0x18, rng.pick(&[0, 1, 2, 0x8000_0002]));
                t.w32(s + 0x10, rng.pick(&[0, 1, 0xffff_ffff]));
            }
            t.w32(segment + 0x2c, streams);
            let mesh = parray(&mut t, &mut rng);
            t.w32(mesh + 4, segment);
            t.ret(m2::CLUSTER_CREATE, mesh);
            t.ret(m2::MALLOC, rng.pick(&[segment, segment, 0]));
            if Mem::read::<u32>(&t.b, mesh) != 1
                && Mem::read::<u32>(&t.b, t.cells[&m2::MALLOC]) == 0
            {
                // the old data stays: the segment
                t.w32(mesh + 4, segment);
            }
            let x = rng.next() | 0x10_0000;
            t.ret(m2::MALLOC + 1, rng.pick(&[0, x]));
            let pos = t.zeros(0x48);
            let uv = t.zeros(0x30);
            t.w32(maps.0, pos);
            t.w32(maps.1, uv);
            t.ret(m2::IS_BOUND, rng.pick(&[0, 1, 0x100]));
            t.ret(m2::UNMAP, rng.pick(&[0, 0, 1]));
            let instance = random_block(&mut t, &mut rng, 0x14);
            t.ret(m4::ALLOCATE_INSTANCE, instance);
            let r = t.check_framed_conv(
                &w,
                0x006e_4920,
                Conv::Thiscall(this),
                &[],
                frame,
                &[(0, 0x30), (0x30, 4), (0x38, 4)],
                |g| match m4::prepare_mesh(g, frame, this) {
                    Some(v) => u(v),
                    None => Out::None,
                },
            );
            let _ = r;
            if t.log_entries() > 20 {
                full += 1;
            }
        }
        assert!(full > 100, "{name}: {full}");
    }
}

/// A recording stub at `sym` that also copies `words` words from each
/// pointer among its stack words `args` into a ring of eight 0x40-byte
/// records (what a callee reads through pointers to the caller's frame,
/// as it was at the call). Returns the ring (a count word, then the
/// records).
fn stub_copying(
    t: &mut Twins,
    sym: u32,
    this: bool,
    n: u32,
    pops: u32,
    args: &[u8],
    words: u8,
) -> u32 {
    assert!(args.len() <= 4 && words <= 4);
    let rest = t.stub_code(sym, this, n, pops, false, &[]);
    let ring = t.zeros(4 + 8 * 0x40);
    let mut c = vec![0x51, 0xa1]; // push ecx; mov eax, [ring]
    c.extend(ring.to_le_bytes());
    c.extend([0x83, 0xe0, 0x07, 0xc1, 0xe0, 0x06, 0x05]); // and eax, 7; shl eax, 6; add eax,
    c.extend((ring + 4).to_le_bytes()); // ring + 4
    for (j, &arg) in (0u8..).zip(args) {
        c.extend([0x8b, 0x54, 0x24, 8 + 4 * arg]); // mov edx, [esp+8+4*arg]
        for k in 0..words {
            c.extend([0x8b, 0x4a, 4 * k, 0x89, 0x48, 16 * j + 4 * k]); // mov ecx, [edx+4k]; mov [eax+16j+4k], ecx
        }
    }
    c.push(0x59); // pop ecx
    c.extend([0xff, 0x05]);
    c.extend(ring.to_le_bytes()); // inc dword [ring]
    jmp_to(t, &mut c, rest);
    let f = t.code(&c);
    t.jump(sym, f);
    ring
}

/// `GraphicAbmap_update_00685840`: every state (negative, 0, 1, 2, 3, 4,
/// large), `+8`'s sign, the draw detached or not, handles empty, half
/// empty (only the high word) or set, render data missing or there; in
/// state 3 the 23 render data with their segments freed and the fifteen
/// geometries released and destroyed through their vtables.
#[test]
fn m5_abmap_update() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_6858);
        for (sym, this, n) in [
            (m5::DELETE, false, 1),
            (m5::DELETE_ARRAY, false, 1),
            (m5::RELEASE_GEOMETRY, true, 0),
            (m5::PHYRE_PRINTF, false, 2),
        ] {
            t.stub_at(sym, this, n, 0, &[]);
        }
        let destroy = t.stub_code(0x00ab_0000, true, 1, 1, false, &[]);
        let vtable = t.zeros(4);
        t.w32(vtable, destroy);
        let tags = [
            m5::DELETE,
            m5::DELETE_ARRAY,
            m5::RELEASE_GEOMETRY,
            m5::PHYRE_PRINTF,
            0x00ab_0000,
        ];
        let mut seen = [0u32; 4];
        for i in 0..300 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let this = random_block(&mut t, &mut rng, 0xe8);
            let state = rng.pick(&[
                0xffff_fffe,
                0xffff_ffff,
                0,
                1,
                2,
                3,
                3,
                3,
                4,
                0x8000_0000,
                0x7fff_ffff,
            ]);
            t.w32(this + 4, state);
            let x = rng.next();
            t.w32(this + 8, rng.pick(&[0, 1, 0xffff_ffff, x]));
            let detached = rng.pick(&[0, 0, 0, 1]);
            t.g32(m5::DRAW_DETACHED, detached);
            for k in 0..15 {
                let geometry = random_block(&mut t, &mut rng, 0x30);
                t.w32(geometry, vtable);
                let (lo, hi) = match rng.below(6) {
                    0 => (0, 0),
                    _ => (geometry, rng.pick(&[0, 0, 0xffff_ffff])),
                };
                t.w32(this + 0x10 + 8 * k, lo);
                t.w32(this + 0x14 + 8 * k, hi);
            }
            for k in 0..23 {
                let data = random_block(&mut t, &mut rng, 0x98);
                let segments = random_block(&mut t, &mut rng, 4 * 0x6c);
                t.w32(data + 0x94, segments);
                let none = state != 3 && rng.below(4) == 0;
                t.w32(this + 0x88 + 4 * k, if none { 0 } else { data });
            }
            t.check_conv(&w, 0x0068_5840, Conv::Thiscall(this), &[], |g| {
                u(m5::abmap_update(g, this))
            });
            let s = state.cast_signed();
            seen[if s < 2 {
                usize::from(detached == 0)
            } else if s == 3 {
                2
            } else {
                3
            }] += 1;
        }
        eprintln!("{name}: {seen:?}");
        assert!(seen.iter().all(|&k| k > 10), "{name}: {seen:?}");
    }
}

/// `ClassDynamicGeometry_setupMaterial`: no owner, the cursor finding no
/// shader, the resize failing, 0..3 segments (and a negative count),
/// material arrays too short, handle tables there or made (inline for one
/// segment, else allocated), each parameter missing, the fade with and
/// without the geometry's depth, clusters missing, texture clusters found
/// or the frame buffer names (and an unknown one), bindings failing.
#[test]
fn m6_setup_material() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_6a3b);
        let cursor = stub_filling(&mut t, m1::CURSOR_SEEK, 0, 0, &[0, 0xc, 0x10]);
        let iter = stub_filling(&mut t, m1::TEXTURE_ITERATOR, 1, 1, &[0, 0x10]);
        let params = [
            m1::TEXTURE_PARAM,
            m1::SAMPLER_PARAM,
            m6::TARGET_PARAM,
            m6::UV_SCALE_PARAM,
            m6::CC_PARAM,
            m1::COLOUR_PARAM,
            m6::FADE_PARAM,
            m6::FADE_FACTOR_PARAM,
        ];
        let cases: Vec<(u32, u32)> = params
            .iter()
            .zip(1..)
            .map(|(&p, k)| (t.rebase(p), m1::FIND_PARAM + k))
            .collect();
        stub_by_arg(&mut t, m1::FIND_PARAM, true, 1, 1, 0, &cases);
        let a_cell = t.zeros(4);
        let b_cell = t.zeros(4);
        t.stub_at(
            m6::CREATE_MATERIAL,
            true,
            4,
            4,
            &[wr(2, a_cell), wr(3, b_cell)],
        );
        for sym in [m1::SET_VALUE, m1::SURFACE_FORMAT, m6::SET_TEXTURE] {
            stub_copying(&mut t, sym, true, 2, 2, &[1], 4);
        }
        // the fade words sit next to the original's own locals: one word
        stub_copying(&mut t, m6::SET_VALUE2, true, 2, 2, &[1], 1);
        for (sym, this, n, pops) in [
            (m6::RESIZE, true, 1, 1),
            (m6::MALLOC, false, 1, 0),
            (m6::MEMSET, false, 3, 0),
            (m1::SET_VALUE_DESC, true, 2, 2),
            (m1::POST_PROCESS, false, 0, 0),
            (m1::CLUSTER_MANAGER, false, 0, 0),
            (m1::CLUSTER_BY_NAME, true, 1, 1),
            (m6::LIVE_POINTER, true, 0, 0),
            (m6::READY_A8, true, 0, 0),
            (m1::LINEAR_SAMPLER, true, 1, 1),
            (m6::SOFT_PARTICLE, false, 5, 0),
        ] {
            t.stub_at(sym, this, n, pops, &[]);
        }
        let mut tags = vec![
            m1::CURSOR_SEEK,
            m1::TEXTURE_ITERATOR,
            m1::FIND_PARAM,
            m6::CREATE_MATERIAL,
            m1::SET_VALUE,
            m6::SET_VALUE2,
            m6::SET_TEXTURE,
            m6::MALLOC,
            m6::MEMSET,
            m1::SET_VALUE_DESC,
            m1::CLUSTER_MANAGER,
            m6::LIVE_POINTER,
            m6::READY_A8,
            m1::LINEAR_SAMPLER,
            m6::SOFT_PARTICLE,
        ];
        tags.extend(cases.iter().map(|c| c.1));
        let texts: Vec<u32> = [
            &b"menu_tex"[..],
            b"D3D11FrameBuffer",
            b"FrameBuffer",
            b"RWTT",
            b"other",
        ]
        .iter()
        .map(|s| string(&mut t, s))
        .collect();
        let manager = t.zeros(0x100);
        let scene = t.zeros(0xfc80);
        let mut seen = [0u32; 6];
        for i in 0..500 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(&mut rng, &[m6::SOFT_A, m6::SOFT_B], &[]);
            t.g32(m1::PHYRE_SCENE, scene);
            t.ret(m1::POST_PROCESS, manager);
            for off in [0x9c, 0xac] {
                let buffer = random_block(&mut t, &mut rng, 8);
                t.w32(buffer + 4, rng.pick(&[0, 0, 1]));
                t.w32(manager + off, buffer);
            }
            let rwtt = random_block(&mut t, &mut rng, 8);
            t.w32(rwtt + 4, rng.pick(&[0, 0, 1]));
            t.w32(scene + 0xfc70, rng.pick(&[0, rwtt, rwtt]));
            for k in 1..=8 {
                if rng.below(5) == 0 {
                    t.ret(m1::FIND_PARAM + k, 0);
                }
            }
            let n = rng.pick(&[0, 1, 1, 2, 3, 3, 0xffff_ffff]);
            let room: u32 = if n <= 3 { n } else { 0 };
            let this = random_block(&mut t, &mut rng, 0x40);
            let owner = t.zeros(0x20);
            t.w32(owner + 0x1c, rng.pick(&[owner + 0x1c, 0x1234]));
            t.w32(
                this + 0x20,
                rng.pick(&[0, owner, owner, owner, owner, owner]),
            );
            let geometry = random_block(&mut t, &mut rng, 0xb8);
            let records = random_block(&mut t, &mut rng, 0x6c * 3);
            t.w32(geometry + 0x94, records);
            t.w8(geometry + 0xb7, rng.pick(&[0, 0, 3, 0xff]));
            t.w32(this + 0x24, geometry);
            let array = random_block(&mut t, &mut rng, 8);
            let elements = random_block(&mut t, &mut rng, 16);
            t.w32(array + 4, elements);
            let count = rng.pick(&[room, room, room, room + 1, room.saturating_sub(1)]);
            let count = count | rng.pick(&[0, 0x8000_0000]);
            t.w32(array, count);
            t.w32(this + 0x14, array);
            for off in [0x18, 0x1c] {
                let table = random_block(&mut t, &mut rng, 16);
                t.w32(this + off, rng.pick(&[0, table, table]));
            }
            let tables = random_block(&mut t, &mut rng, 16);
            t.ret(m6::MALLOC, tables);
            t.ret(m6::RESIZE, rng.pick(&[0, 0, 0, 0, 7]));
            let names = t.zeros(16);
            let clusters = t.zeros(16);
            for k in 0..room {
                let s = texts[rng.below(5) as usize];
                t.w32(names + 4 * k, s);
                let x = rng.next();
                t.w32(clusters + 4 * k, rng.pick(&[0, x, x]));
            }
            let x = rng.next();
            t.ret(m1::CLUSTER_BY_NAME, rng.pick(&[0, 0, x]));
            t.ret(m1::SURFACE_FORMAT, rng.pick(&[0, 0, 0, 0, 5]));
            t.ret(m6::SET_TEXTURE, rng.pick(&[0, 0, 0, 0, 5]));
            t.ret(m1::SET_VALUE, rng.pick(&[0, 0, 0, 0, 3]));
            let x = rng.next();
            t.w32(a_cell, rng.pick(&[0, x]));
            let x = rng.next();
            t.w32(b_cell, rng.pick(&[0, x]));
            let material = random_block(&mut t, &mut rng, 8);
            t.ret(m6::CREATE_MATERIAL, material);
            let base = t.zeros(0x40);
            let off = 4 * rng.below(8);
            let shader = rng.next();
            t.w32(base + off, shader);
            t.w32(cursor[0], off);
            t.w32(cursor[1], rng.pick(&[0, 1, 1, 1, 1, 1]));
            t.w32(cursor[2], base);
            let (a, b) = (rng.next(), rng.next());
            t.w32(iter[0], a);
            t.w32(iter[1], b);
            let ebp = t.ebp_for(3);
            let frame = ebp - 0xf8;
            let r = t.check_framed_conv(
                &format!("{w} n {n:#x} count {count:#x} this {this:#x} geometry {geometry:#x} array {array:#x} names {names:#x} clusters {clusters:#x} material {material:#x}"),
                0x006a_3b50,
                Conv::Thiscall(this),
                &[n, names, clusters],
                frame,
                &[(0, 0xc0), (0xc4, 4), (0xd0, 4), (0xd8, 4), (0xe8, 4)],
                |g| u(m6::setup_material(g, frame, this, n, names, clusters)),
            );
            seen[match r {
                0 => 0,
                0xa => 1,
                3 => 2,
                0x13 => 3,
                5 | 7 => 4,
                _ => 5,
            }] += 1;
        }
        eprintln!("{name}: {seen:?}");
        assert!(seen[..5].iter().all(|&k| k > 5), "{name}: {seen:?}");
    }
}

/// `ClassDynamicGeometry_updateRenderData`: no geometry, no instance flag,
/// no or empty materials; mesh, instance and bounds there or not; the
/// segment textures bound once (names missing, materials missing,
/// clusters or parameters missing, bindings failing); the depth range,
/// z matrix (offsets out of the buffer or below 4), screen, soft particle
/// and fade passes on and off; random floats (NaN, infinities).
#[test]
fn m7_update_render_data() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_6a4a);
        let iter = stub_filling(&mut t, m1::TEXTURE_ITERATOR, 1, 1, &[0, 0x10]);
        let params = [
            m1::TEXTURE_PARAM,
            m1::SAMPLER_PARAM,
            m7::Z_RANGE_PARAM,
            m7::Z_MATRIX_PARAM,
            m7::SCREEN_PARAM,
            m6::FADE_PARAM,
            m6::FADE_FACTOR_PARAM,
        ];
        let cases: Vec<(u32, u32)> = params
            .iter()
            .zip(1..)
            .map(|(&p, k)| (t.rebase(p), m1::FIND_PARAM + k))
            .collect();
        stub_by_arg(&mut t, m1::FIND_PARAM, true, 1, 1, 0, &cases);
        stub_copying(&mut t, m1::SURFACE_FORMAT, true, 2, 2, &[1], 4);
        stub_copying(&mut t, m6::SET_VALUE2, true, 2, 2, &[1], 1);
        stub_copying(&mut t, m7::SET_MIN_MAX, true, 2, 2, &[0, 1], 3);
        stub_copying(&mut t, m7::MEMCPY, false, 3, 0, &[1], 4);
        for (sym, this, n, pops) in [
            (m7::FILL_DRAW_TABLE, true, 1, 1),
            (m7::SET_WORLD_MATRIX, false, 2, 2),
            (m1::SET_VALUE_DESC, true, 2, 2),
            (m1::CLUSTER_MANAGER, false, 0, 0),
            (m1::CLUSTER_BY_NAME, true, 1, 1),
            (m1::LINEAR_SAMPLER, true, 1, 1),
            (m6::SOFT_PARTICLE, false, 5, 0),
        ] {
            t.stub_at(sym, this, n, pops, &[]);
        }
        let mut tags = vec![
            m1::TEXTURE_ITERATOR,
            m1::FIND_PARAM,
            m6::SET_VALUE2,
            m7::SET_MIN_MAX,
            m7::MEMCPY,
            m7::FILL_DRAW_TABLE,
            m7::SET_WORLD_MATRIX,
            m1::SET_VALUE_DESC,
            m1::CLUSTER_MANAGER,
            m1::LINEAR_SAMPLER,
            m6::SOFT_PARTICLE,
        ];
        tags.extend(cases.iter().map(|c| c.1));
        let scene = t.zeros(4);
        let mut seen = [0u32; 4];
        for i in 0..500 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(&mut rng, &[m6::SOFT_A, m6::SOFT_B], &[]);
            t.g32(m1::PHYRE_SCENE, scene);
            // the parameters: offsets and sizes in and out of the buffers
            for k in 1..=7 {
                let p = t.zeros(0xc);
                t.w16(p + 8, rng.pick(&[0, 2, 4, 8, 0x40, 0x100]));
                t.w16(p + 0xa, rng.pick(&[4, 8, 0x40, 0xe010, 0x1ff0]));
                t.ret(m1::FIND_PARAM + k, rng.pick(&[0, p, p, p]));
            }
            // the geometry
            let g = random_block(&mut t, &mut rng, 0xb8);
            for k in 0..0x30 {
                let f = rng.float();
                t.w32(g + 0x40 + 4 * k, f);
            }
            let x = rng.next();
            t.w32(g + 0x80, rng.pick(&[0, 0, 5, 0xffff_fff0, x]));
            t.w32(g + 0x84, rng.pick(&[0, 1]));
            let segments = rng.pick(&[0, 1, 2, 3, 0xffff_ffff]);
            t.w32(g + 0x90, segments);
            let records = random_block(&mut t, &mut rng, 3 * 0x6c);
            for k in 0..3 {
                let x = rng.next();
                t.w32(records + 0x6c * k + 0x20, rng.pick(&[0, x, x]));
            }
            t.w32(g + 0x94, records);
            t.w8(g + 0xb6, rng.pick(&[0, 0, 2]));
            t.w8(g + 0xb7, rng.pick(&[0, 0, 4, 0xff]));
            // the materials
            let n = rng.pick(&[0, 1, 2, 3, 0x8000_0002]);
            let room = n & 0x7fff_ffff;
            let array = t.zeros(16);
            t.w32(array, n);
            let elements = t.zeros(16);
            for k in 0..3 {
                let material = random_block(&mut t, &mut rng, 8);
                let buffer = t.zeros(0x240);
                t.w32(buffer, rng.pick(&[0x40, 0x200, 0x110]));
                t.w32(material + 4, buffer);
                let at = if room == 1 {
                    array + 4
                } else {
                    elements + 4 * k
                };
                // later passes read every material's buffer
                let range: u32 = Mem::read(&t.b, g + 0x80);
                let screen: u32 = Mem::read(&t.b, g + 0x84);
                let fade: u8 = Mem::read(&t.b, g + 0xb7);
                let zero_ok = range == 0 && screen == 0 && fade == 0;
                if k < room.max(1) {
                    let m = if zero_ok {
                        rng.pick(&[0, material, material, material])
                    } else {
                        material
                    };
                    t.w32(at, m);
                }
            }
            if room > 1 {
                t.w32(array + 4, elements);
            }
            let this = random_block(&mut t, &mut rng, 0x34);
            t.w32(this + 0x24, rng.pick(&[0, g, g, g, g, g, g, g]));
            t.w32(this + 4, rng.pick(&[0, 1, 1, 1, 1]));
            t.w32(
                this + 0x14,
                rng.pick(&[0, array, array, array, array, array]),
            );
            let x = rng.next();
            t.w32(this + 8, rng.pick(&[0, x]));
            let instance = random_block(&mut t, &mut rng, 0x20);
            let x = rng.next();
            t.w32(instance + 0x1c, rng.pick(&[0, x]));
            t.w32(this + 0xc, rng.pick(&[0, instance]));
            t.w8(this + 0x30, rng.pick(&[0, 0, 1]));
            let x = rng.next();
            t.ret(m1::CLUSTER_BY_NAME, rng.pick(&[0, x, x]));
            t.ret(m1::SURFACE_FORMAT, rng.pick(&[0, 0, 0, 0, 5]));
            let (a, b) = (rng.next(), rng.next());
            t.w32(iter[0], a);
            t.w32(iter[1], b);
            let ebp = t.ebp_for(1);
            let frame = ebp - 0x100;
            let unused = rng.next();
            let r = t.check_framed_conv(
                &format!("{w} n {n:#x} segments {segments:#x}"),
                0x006a_4a50,
                Conv::Thiscall(this),
                &[unused],
                frame,
                &[(0, 0x6c), (0x6c, 0x40), (0xb0, 8), (0xb8, 0x30)],
                |g| u(m7::update_render_data(g, frame, this)),
            );
            let calls = t.log_entries();
            seen[match r {
                0 if calls > 10 => 0,
                0 | 1 => 1,
                5 => 2,
                _ => 3,
            }] += 1;
        }
        eprintln!("{name}: {seen:?}");
        assert!(seen[..3].iter().all(|&k| k > 2), "{name}: {seen:?}");
    }
}

/// `createVFXRenderDataBuffer`: models of 1..6 segments of every kind
/// (and unknown kinds), counts 0, negative and large (several index
/// groups), the texture numbers' special cases (the frame buffers,
/// monsters with and without an instance or a name, magic 0x122's
/// remaps, the flag numbers), textures missing, 30 segments, the magic
/// and map cases of the flags, one, several or no instances, render data
/// missing.
#[test]
fn m8_create_render_data_buffer() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_7137);
        for (sym, n) in [
            (m8::MEMSET, 3),
            (m8::LEVEL_HASH, 0),
            (m8::MAGIC_ID, 0),
            (m8::PTR32, 1),
            (m8::IS_TEXTURE_BP_LOADED, 7),
            (m8::IS_TEXTURE_LOADED, 6),
            (m8::CHR_INSTANCE, 1),
            (m8::MONSTER_TEXTURE_NAME, 2),
            (m8::BATTLE_SCENE, 0),
            (m8::DBG_PRINTF, 2),
            (m8::VFX_CREATE, 3),
            (m8::USE_MULTIPLE, 2),
            (m8::CREATE_DATA_BUFFER, 7),
            (m8::RENDER_DATA, 2),
            (m8::ALLOCATE_BUFFER, 1),
        ] {
            t.stub_at(sym, false, n, 0, &[]);
        }
        stub_import(&mut t, m8::SPRINTF.sym, 2);
        stub_import(&mut t, m8::STRSTR.sym, 2);
        let tags = [
            m8::MEMSET,
            m8::IS_TEXTURE_LOADED,
            m8::DBG_PRINTF,
            m8::VFX_CREATE,
            m8::CREATE_DATA_BUFFER,
            m8::ALLOCATE_BUFFER,
            m8::SPRINTF.sym,
        ];
        let monster_name = string(&mut t, b"m_tex_012");
        let numbers = [
            0x3680u32, 0x3420, 0x3400, 0x3600, 0xd00, 0x1a00, 0x3f10, 0x3f00, 0x100, 0x3b00,
            0x3800, 0, 0x3a00, 0x3b40, 0x3b41,
        ];
        let mut seen = [0u32; 4];
        for i in 0..600 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let level = rng.pick(&[0x25ca, 0x25b0, 0x1234, 0x25ca]);
            t.ret(m8::LEVEL_HASH, level);
            let magic = rng.pick(&[
                0x122, 0x12a, 0x111, 0xc9, 0xcb, 0x203, 0x150, 0x29b, 0x66, 0x67, 0x1ba, 0x18d,
                0x207, 0xbc, 0x15f, 0x241, 0x241, 0x241,
            ]);
            t.ret(m8::MAGIC_ID, magic);
            t.g32(m8::MAGIC_CASES, rng.pick(&[0, 1]));
            t.g32(m8::MAP_188_WORD, rng.pick(&[0xffff_ffff, 3]));
            t.g32(m8::CURRENT_MAP, rng.pick(&[0x188, 0x46, 0x71, 0x46]));
            t.ret(m8::BATTLE_SCENE, rng.pick(&[0x0259_0069, 0x0259_0069, 7]));
            t.ret(
                m8::CHR_INSTANCE,
                rng.pick(&[0, 0xffff_ffff, 0x1234, 0x1234]),
            );
            t.ret(
                m8::MONSTER_TEXTURE_NAME,
                rng.pick(&[0, monster_name, monster_name]),
            );
            t.ret(m8::IS_TEXTURE_BP_LOADED, rng.pick(&[0, 1, 1, 1, 1, 1, 1]));
            t.ret(m8::IS_TEXTURE_LOADED, rng.pick(&[0, 1, 1]));
            t.ret(m8::STRSTR.sym, rng.pick(&[0, 1, 1, 1]));
            t.ret(m8::USE_MULTIPLE, rng.pick(&[0, 1, 2, 2]));
            // the model: segments, then a terminator that ends the count
            let segments = rng.pick(&[1, 2, 3, 4, 6, 31]);
            let mut bytes = Vec::new();
            let mut total = 0i32;
            for s in 0..segments {
                let kind: u8 = rng.pick(&[0, 1, 2, 3, 4, 5, 6, 7, 2, 3, 6, 7, 0xff, 8]);
                let kind = if rng.below(8) == 0 { kind } else { kind & 7 };
                let last = s + 1 == segments;
                let n: i16 = if last && rng.below(4) == 0 {
                    rng.pick(&[1000, 1500, 2000, 3001, 4500])
                } else {
                    rng.pick(&[0, 1, 2, 3, 1, 2, -1])
                };
                total += i32::from(n);
                let number = numbers[rng.below(numbers.len() as u32) as usize];
                let d = u64::from(number)
                    | u64::from(rng.below(0x40)) << 20
                    | u64::from(rng.below(16)) << 26
                    | u64::from(rng.below(16)) << 30
                    | u64::from(rng.next() & 0xffff_fffc) << 34;
                let mut rec = vec![0u8; 0x10];
                rec[0] = rng.next() as u8;
                rec[1] = kind;
                rec[2..4].copy_from_slice(&n.to_le_bytes());
                rec[8..16].copy_from_slice(&d.to_le_bytes());
                let stride = [20, 28, 32, 40, 24, 32, 40, 48][usize::from(kind & 7)];
                rec.extend(vec![0u8; stride * usize::from(n.max(0).cast_unsigned())]);
                bytes.extend(rec);
            }
            // a terminator whose count ends any remaining count (its data is
            // never read)
            let mut rec = vec![0u8; 0x10];
            rec[1] = 1;
            rec[2..4].copy_from_slice(&0x200i16.to_le_bytes());
            bytes.extend(rec);
            let total = if total <= 0 { 0 } else { total };
            let model = t.zeros(0x20);
            t.w32(model, rng.pick(&[0, 4, 0x44]));
            let records = t.alloc(&bytes);
            t.w32(model + 4, records.wrapping_sub(model));
            t.w16(model + 0x10, rng.pick(&[total as u16, total as u16, 0x96]));
            t.ret(m8::PTR32, model);
            let effect = random_block(&mut t, &mut rng, 0x20);
            let data = random_block(&mut t, &mut rng, 0x40);
            t.w8(data, rng.pick(&[0, 0x40]));
            t.w8(data + 8, rng.pick(&[0x44, 0x42, 0x10]));
            let x = rng.next();
            t.w32(data + 0x38, rng.pick(&[0, x]));
            let rd = random_block(&mut t, &mut rng, 0xc8);
            let segs = t.zeros(31 * 0x6c);
            t.w32(rd + 0x94, segs);
            t.ret(m8::RENDER_DATA, rng.pick(&[rd, rd, rd, 0]));
            let flags = rng.pick(&[0, 1, 0x2000, 0x1000_0001, 0x8000_0000, 0x2001]);
            let (lo, hi) = (rng.next(), rng.pick(&[0, 7, 0xffff_ffff]));
            let ebp = t.ebp_for(5);
            let frame = ebp - 0x23dc;
            let r = t.check_framed(
                &format!("{w} magic {magic:#x} level {level:#x} segments {segments}"),
                0x0071_3790,
                &[lo, hi, effect, data, flags],
                frame,
                // up to ebp - 8: _alloca_probe keeps ecx there (names[29], never used)
                &[(0x38, m8::BUFFER_FRAME as usize - 0x3c)],
                |g| {
                    u(m8::create_render_data_buffer(
                        g, frame, lo, hi, effect, data, flags,
                    ))
                },
            );
            let calls = t.log_entries();
            seen[if r == 0 {
                0
            } else if calls > 12 {
                1
            } else {
                2
            }] += 1;
        }
        eprintln!("{name}: {seen:?}");
        assert!(seen[..3].iter().all(|&k| k > 10), "{name}: {seen:?}");
    }
}

/// `updateVFXRenderDataAtelShp`: flagged and unflagged shapes of 0..4
/// primitives (and a negative count) spread over the six alpha buckets
/// (and unknown alpha functions), render data and buffers missing,
/// objects with and without a tint and a second matrix, screens below
/// and from 720 high with unscaled or scaled matrices; random floats and
/// constants.
#[test]
fn m9_update_atel_shp() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_7196);
        let w_cell = t.zeros(4);
        let h_cell = t.zeros(4);
        t.stub_at(m9::SCREEN_WH, false, 2, 0, &[wr(0, w_cell), wr(1, h_cell)]);
        // the alpha function by a table of the 256 modes
        let table = t.zeros(0x400);
        let rest = t.stub_code(m9::ALPHA_FUNCTION, false, 1, 0, false, &[]);
        let cell = t.cells[&m9::ALPHA_FUNCTION];
        let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x04, 0x85];
        c.extend(table.to_le_bytes()); // eax = table[[esp+4]]
        c.push(0xa3);
        c.extend(cell.to_le_bytes()); // mov [cell], eax
        jmp_to(&t, &mut c, rest);
        let f = t.code(&c);
        t.jump(m9::ALPHA_FUNCTION, f);
        let lo = 0x1234_0000u32;
        let cases: Vec<(u32, u32)> = (0..6)
            .map(|k| (lo + 10000 * k, m9::RENDER_DATA + 1 + k))
            .collect();
        stub_by_arg(&mut t, m9::RENDER_DATA, false, 2, 0, 0, &cases);
        stub_copying(&mut t, m9::APPLY_MATRIX, false, 3, 0, &[2], 4);
        for (sym, n) in [
            (m9::GET_BUFFER, 4),
            (m9::MUL_MATRIX, 3),
            (m9::SET_VISIBLE, 3),
        ] {
            t.stub_at(sym, false, n, 0, &[]);
        }
        let mut tags = vec![
            m9::SCREEN_WH,
            m9::MUL_MATRIX,
            m9::SET_VISIBLE,
            m9::APPLY_MATRIX,
        ];
        tags.extend(cases.iter().map(|c| c.1));
        let mut seen = [0u32; 3];
        for i in 0..500 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            for k in 0..0x100 {
                let a = rng.pick(&[0, 2, 0x10, 0x20, 0x100, 0x200, 0x40]);
                t.w32(table + 4 * k, a);
            }
            t.constants(
                &mut rng,
                &[m9::MIN_START, m9::MAX_START, m9::ALPHA_SCALE],
                &[
                    m9::INV_512,
                    m9::HEIGHT_416,
                    m9::FIFTEEN,
                    m9::INV_256,
                    m9::INV_128,
                    m9::INV_4096,
                    m9::HALF,
                    m9::MARGIN,
                ],
            );
            if rng.below(2) == 0 {
                for (c, v) in [
                    (m9::MIN_START, f32::MAX),
                    (m9::MAX_START, -f32::MAX),
                    (m9::ALPHA_SCALE, 256.0),
                ] {
                    t.g32(c, v.to_bits());
                }
                for (c, v) in [
                    (m9::INV_512, 1.0 / 512.0),
                    (m9::HEIGHT_416, 416.0f64),
                    (m9::FIFTEEN, 15.0),
                    (m9::INV_256, 1.0 / 256.0),
                    (m9::INV_128, 1.0 / 128.0),
                    (m9::INV_4096, 1.0 / 4096.0),
                    (m9::HALF, 0.5),
                    (m9::MARGIN, 0.850_000_023_841_857_9),
                ] {
                    let at = t.rebase(c);
                    t.write(at, &v.to_le_bytes());
                }
                if rng.below(3) == 0 {
                    // texture coordinates whose 1 - v is not an f32
                    let at = t.rebase(m9::INV_4096);
                    t.write(at, &(1.0f64 / 3001.0).to_le_bytes());
                }
            }
            // the object
            let obj = if rng.below(4) == 0 {
                0
            } else {
                let o = random_block(&mut t, &mut rng, 0x20);
                let m = t.zeros(0x40);
                for k in 0..16 {
                    let f = rng.float();
                    t.w32(m + 4 * k, f);
                }
                // unscaled (both 1), or only the first 1
                if rng.below(2) == 0 {
                    t.w32(m, 1f32.to_bits());
                    if rng.below(4) != 0 {
                        t.w32(m + 0x14, 1f32.to_bits());
                    }
                }
                t.w32(o + 0x10, m);
                let m2 = random_block(&mut t, &mut rng, 0x40);
                t.w32(o + 0x1c, rng.pick(&[0, m2]));
                o
            };
            let width = rng.pick(&[1920, 1280, 640, 0xffff_fc00]);
            let height = if obj == 0 {
                rng.pick(&[480, 719, 0])
            } else {
                rng.pick(&[1080, 720, 719, 480])
            };
            t.w32(w_cell, width);
            t.w32(h_cell, height);
            // the shapes
            let n = rng.pick(&[0i16, 1, 2, 3, 4, -1]);
            let room = u32::from(n.max(0).cast_unsigned());
            let shapes = t.zeros(0x30);
            let record = t.zeros(0x20 + 16 * room as usize);
            for k in 0..room * 8 {
                let v = rng.small() as u16;
                t.w16(record + 0x20 + 2 * k, v);
            }
            let prims = random_block(&mut t, &mut rng, 8 * room + 4);
            t.w8(record, rng.pick(&[0, 0, 0, 8, 0x80]));
            t.w16(record + 2, n.cast_unsigned());
            t.w32(record + 4, prims.wrapping_sub(record));
            let index = rng.below(4);
            t.w16(
                shapes + 0x10 + 8 * index,
                (record.wrapping_sub(shapes)) as u16,
            );
            // the instances
            for k in 0..6 {
                let rd = random_block(&mut t, &mut rng, 0xb0);
                let seg = t.zeros(0x20);
                for (off, size) in [(0xc, 48), (0x14, 64), (0x18, 32), (0x1c, 12)] {
                    let b = t.zeros((size * room.max(1)) as usize);
                    t.w32(seg + off, b);
                }
                t.w32(rd + 0x94, seg);
                t.ret(m9::RENDER_DATA + 1 + k, rng.pick(&[rd, rd, rd, rd, 0]));
            }
            t.ret(m9::GET_BUFFER, rng.pick(&[1, 1, 1, 1, 0]));
            let hi = rng.pick(&[0, 3]);
            let ebp = t.ebp_for(5);
            let frame = ebp - 0x94;
            let index_word = index | rng.pick(&[0, 0x5_0000]);
            let r = t.check_framed(
                &format!("{w} n {n} obj {obj:#x} {width}x{height}"),
                0x0071_96f0,
                &[lo, hi, shapes, obj, index_word],
                frame,
                &[(0x14, 4), (0x20, 4), (0x70, 0x10), (0x80, 0x10)],
                |g| {
                    u(m9::update_atel_shp(
                        g, frame, lo, hi, shapes, obj, index_word,
                    ))
                },
            );
            seen[if r == 0 {
                0
            } else if room > 0 {
                1
            } else {
                2
            }] += 1;
        }
        eprintln!("{name}: {seen:?}");
        assert!(seen.iter().all(|&k| k > 10), "{name}: {seen:?}");
    }
}
