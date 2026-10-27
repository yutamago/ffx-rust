//! M4 list `h4-glue-other`, agent h4y (`ffx_glue_other::h4y_*`): the
//! particle fill-buffer jobs against the original, on the image alone and
//! on a snapshot (`FFX_SNAPSHOT`). Results and every byte written must
//! agree. Needs `FFX_GAME_DIR`. The helpers are those of
//! `h4_glue_other_g.rs`.
//!
//! Callees that stay original are replaced in both oracles alike by
//! recording stubs (tag, `ecx` for methods, stack words, in a log in
//! memory; the result from a cell), so the diff checks the calls too; the
//! `FFXVu0*` math runs as the original in both. Image float constants are
//! overwritten with random floats in both oracles, so a port that reads
//! the wrong one differs.

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
use ffx_glue_other::{h4y_dtt as dtt, h4y_shp as shp};
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

// ---- JobSchedule_pppPart_fillBuffer_drawShp_00727110 (ffx_glue_other::h4y_shp)

/// Where the original's `ebp` lands for a one-word call of a function that
/// aligns its frame to 16 bytes (`and esp, -16` after `push ebx`).
fn aligned_ebp(t: &Twins) -> u32 {
    let entry = t.a.stack_range().end - 0x100 - 4 - 4;
    (entry - 12) & !0xf
}

/// The blocks of [`shp::FRAME`] the callees see.
const SHP_BLOCKS: [(u32, usize); 9] = [
    (shp::FADE_MATRIX, 0x40),
    (shp::PART_SCREEN, 0x40),
    (shp::ZMAT_SCREEN, 0x40),
    (shp::QUAD_CORNERS, 0x40),
    (shp::QUAD_RECT, 0x40),
    (shp::SCALE, 0x10),
    (shp::LAST, 0x10),
    (shp::FADE, 0x10),
    (shp::COLOUR, 0x10),
];

/// Random floats over `n` words at `at`.
fn floats(t: &mut Twins, rng: &mut Rng, at: u32, n: u32) {
    for k in 0..n {
        let f = rng.float();
        t.w32(at + 4 * k, f);
    }
}

/// A matrix of ordinary floats (`-8..8`), or random float bits.
fn matrix(t: &mut Twins, rng: &mut Rng, at: u32) {
    if rng.below(4) == 0 {
        floats(t, rng, at, 16);
    } else {
        for k in 0..16 {
            let v = (rng.below(4001) as f32 - 2000.0) / 250.0;
            t.w32(at + 4 * k, v.to_bits());
        }
    }
}

/// The real values of the constants.
fn shp_real_constants(t: &mut Twins) {
    for (c, v) in [
        (shp::INV_16, 0.0625f64),
        (shp::INV_128, 1.0 / 128.0),
        (shp::INV_256, 1.0 / 256.0),
        (shp::INV_4096, 1.0 / 4096.0),
        (shp::EIGHT, 8.0),
        (shp::THIRTY_TWO, 32.0),
        (shp::MIN_W, 1e-5),
    ] {
        let at = t.rebase(c);
        t.write(at, &v.to_le_bytes());
    }
    for (c, v) in [
        (shp::F128, 128f32),
        (shp::HALF, 0.5),
        (shp::MINUS_ONE, -1.0),
    ] {
        t.g32(c, v.to_bits());
    }
}

/// `drawShp_00727110` on random parts: rectangles and four-corner shapes,
/// 0..5 primitives (and a negative count), slots by mode or not, with
/// and without the z matrix (clipped or not; level 0xea7's own matrix),
/// with and without the fade (below 0, inside, above 128), colours above
/// and below 0.5, render data counts 0..6 and their masks. On a snapshot,
/// also every live part of kind 1 in the six fill buffers.
#[test]
fn shp_draw_shp() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_7110);
        t.stub(shp::LEVEL_HASH, 0, false);
        t.stub(shp::ZCLIP, 6, false);
        t.stub(shp::UPDATE_PARAMETER, 4, false);
        t.stub(shp::USE_OPTIMIZED, 2, false);
        let tags = [shp::LEVEL_HASH, shp::ZCLIP];
        let ebp = aligned_ebp(&t);
        let frame = ebp - 0x1b8;
        let mut seen = [0u32; 4];
        for i in 0..600 {
            t.fresh(&mut rng, &tags);
            if rng.below(3) == 0 {
                t.constants(
                    &mut rng,
                    &[shp::F128, shp::HALF, shp::MINUS_ONE],
                    &[
                        shp::INV_16,
                        shp::INV_128,
                        shp::INV_256,
                        shp::INV_4096,
                        shp::EIGHT,
                        shp::THIRTY_TWO,
                        shp::MIN_W,
                    ],
                );
            } else {
                shp_real_constants(&mut t);
                if rng.below(4) == 0 {
                    // texture coordinates whose 1 - v is not an f32
                    let at = t.rebase(shp::INV_4096);
                    t.write(at, &(1.0f64 / 3001.0).to_le_bytes());
                }
            }
            let screen = t.rebase(shp::SCREEN_MATRIX);
            matrix(&mut t, &mut rng, screen);
            t.ret(shp::LEVEL_HASH, rng.pick(&[0, 0xea7, 0x25ca]));
            let clip = rng.below(3);
            t.ret(shp::ZCLIP, clip.saturating_sub(1));
            // the shape
            let corners = rng.below(2) == 0;
            let n = rng.pick(&[0i16, 1, 2, 3, 5, -1]);
            let room = u32::from(n.max(0).cast_unsigned());
            let stride = if corners { 0x18 } else { 0x10 };
            let shape = t.zeros((0x20 + stride * room) as usize);
            for k in 0..stride * room / 2 {
                let v = if rng.below(8) == 0 {
                    rng.next() as u16
                } else {
                    rng.small() as u16
                };
                t.w16(shape + 0x20 + 2 * k, v);
            }
            let flags = (rng.next() as u16 & !8) | if corners { 8 } else { 0 };
            t.w16(shape, flags);
            t.w16(shape + 2, n.cast_unsigned());
            let prims = random_block(&mut t, &mut rng, 8 * room + 4);
            for k in 0..room {
                let mode = rng.pick(&[0x48u8, 0x42, 0x46, 0x41, 0x88, 0x40, 0]);
                t.w8(prims + 8 * k + 4, mode);
                if rng.below(2) == 0 {
                    // dim colours: below 0.5 after the scaling
                    for j in 0..4 {
                        let c = rng.below(0x40) as u8;
                        t.w8(prims + 8 * k + j, c);
                    }
                }
            }
            t.w32(shape + 4, prims.wrapping_sub(shape));
            // the part
            let part = random_block(&mut t, &mut rng, 0x214);
            t.w32(part + 0x10, shape);
            t.w8(part + 1, rng.pick(&[0, 0, 1, 0x80]));
            let fade = rng.below(2) == 0;
            let pflags = (rng.next() as u16 & !0x400) | if fade { 0x400 } else { 0 };
            t.w16(part + 0x44, pflags);
            matrix(&mut t, &mut rng, part + 0x48);
            matrix(&mut t, &mut rng, part + 0x88);
            if fade && rng.below(2) == 0 {
                // w spread around the fade's 4..20
                let at = screen + 0x30;
                for k in 0..4 {
                    let v = rng.below(30) as f32 - 4.0;
                    t.w32(at + 4 * k, v.to_bits());
                }
            }
            if rng.below(4) == 0 {
                // diagonal matrices: w exactly 20 (the fade's top), 4 (its
                // bottom) or 1 (the depth limit set to 1); half colours
                let sw = rng.pick(&[20f32, 4.0, 1.0, 2.5]);
                for m in [screen, part + 0x48, part + 0x88] {
                    t.write(m, &[0; 0x40]);
                    for k in 0..4 {
                        let v = if m == screen && k == 3 { sw } else { 1.0 };
                        t.w32(m + 0x14 * k, v.to_bits());
                    }
                }
                // the ramp's ends exact, its scale random or not
                t.g32(shp::F128, 128f32.to_bits());
                for (c, v) in [(shp::EIGHT, 8f64), (shp::THIRTY_TWO, 32.0)] {
                    let at = t.rebase(c);
                    t.write(at, &v.to_le_bytes());
                }
                if rng.below(2) == 0 {
                    let at = t.rebase(shp::INV_128);
                    let v = rng.double();
                    t.write(at, &v.to_le_bytes());
                }
                let at = t.rebase(shp::MIN_W);
                t.write(at, &rng.pick(&[1.0f64, 1e-5, 20.0]).to_le_bytes());
                for j in 0..4 {
                    t.w8(part + 0xc8 + j, 0x80);
                }
                for k in 0..room {
                    for j in 0..4 {
                        t.w8(prims + 8 * k + j, rng.pick(&[0x80, 0x40, 0x81]));
                    }
                }
            }
            t.w32(part + 0xcc, rng.pick(&[0, 1, 0x100]));
            for k in 0..6 {
                let rd = random_block(&mut t, &mut rng, 0xd0);
                let seg = t.zeros(0x20);
                for (off, size) in [(0xc, 0x30), (0x14, 0x40), (0x18, 0x20), (0x1c, 0xc)] {
                    let b = random_block(&mut t, &mut rng, size * room.max(1));
                    t.w32(seg + off, b);
                }
                t.w32(rd + 0x94, seg);
                t.w16(rd + 0xc4, rng.below(7) as u16);
                t.w32(part + 0x2c + 4 * k, rd);
            }
            let w = format!("{name} {i}: n {n} corners {corners} fade {fade} clip {clip}");
            t.check_framed(&w, 0x0072_7110, &[part], frame, &SHP_BLOCKS, |g| {
                u(shp::draw_shp(g, frame, part))
            });
            seen[usize::from(corners) * 2 + usize::from(fade)] += 1;
        }
        eprintln!("{name}: {seen:?}");
        // the snapshot's own parts
        let mut live = 0;
        for b in 0..6 {
            let parts: u32 = Mem::read(&t.a, t.rebase(0x00ce_c1a8) + 4 * b);
            let count: u32 = Mem::read(&t.a, t.rebase(0x00ce_c1c0) + 4 * b);
            if parts == 0 {
                continue;
            }
            for k in 0..count.min(0x200) {
                let part = parts + 0x214 * k;
                if Mem::read::<u8>(&t.a, part) != 1 {
                    continue;
                }
                t.fresh(&mut rng, &tags);
                let w = format!("{name} buffer {b} part {k}");
                t.check_framed(&w, 0x0072_7110, &[part], frame, &SHP_BLOCKS, |g| {
                    u(shp::draw_shp(g, frame, part))
                });
                live += 1;
            }
        }
        eprintln!("{name}: {live} live parts");
    }
}

// ---- JobSchedule_pppPart_fillBuffer_drawMdlDtt (ffx_glue_other::h4y_dtt)

/// The real values of `drawMdlDtt`'s constants.
fn dtt_real_constants(t: &mut Twins) {
    for (c, v) in [
        (dtt::QUARTER, 0.25f64),
        (dtt::ZERO, 0.0),
        (dtt::THREE, 3.0),
        (dtt::FOUR, 4.0),
        (dtt::THIRD, 0.333_330_005_407_333_4),
        (dtt::INV_255, 0.003_921_568_859_368_563),
        (dtt::MIN_LENGTH, 1e-6),
        (dtt::CENTRE, 2048.5),
        (dtt::HEIGHT, 448.0),
        (dtt::HALF, 0.5),
        (dtt::CUT_SCALE, 1.333_333_015_441_894_5),
    ] {
        let at = t.rebase(c);
        t.write(at, &v.to_le_bytes());
    }
    for (c, v) in [
        (dtt::MIN_START, 16_777_216f32),
        (dtt::WIDTH_CUT, 512.0),
        (dtt::WIDTH, 682.0),
        (dtt::LIMIT, 4096.0),
        (dtt::BOUND_LOW, -1e6),
        (dtt::BOUND_HIGH, 1e6),
        (dtt::MINUS_ONE, -1.0),
    ] {
        t.g32(c, v.to_bits());
    }
}

/// A view matrix that puts points near the screen's middle (column-major,
/// translation 2048, 2048), scaled by `s`, w near 1; or random floats.
fn view_matrix(t: &mut Twins, rng: &mut Rng, at: u32) {
    if rng.below(8) == 0 {
        floats(t, rng, at, 16);
        return;
    }
    let s = rng.pick(&[1.0f32, 2.5, 0.25, -1.0]);
    let mut m = [0f32; 16];
    m[0] = s;
    m[5] = s;
    m[10] = s;
    m[12] = 2048.0;
    m[13] = 2048.0;
    m[14] = rng.below(200) as f32 - 100.0;
    m[15] = 1.0;
    if rng.below(2) == 0 {
        for v in &mut m {
            *v += (rng.below(201) as f32 - 100.0) / 1000.0;
        }
    }
    for (k, v) in (0u32..).zip(m) {
        t.w32(at + 4 * k, v.to_bits());
    }
}

/// Small ordinary floats over `n` words.
fn small_floats(t: &mut Twins, rng: &mut Rng, at: u32, n: u32, range: u32) {
    for k in 0..n {
        let v = (rng.below(2 * range + 1) as f32 - range as f32) / 7.0;
        t.w32(at + 4 * k, v.to_bits());
    }
}

/// `drawMdlDtt` on random parts: points from the shape or a model buffer
/// (missing or there, with or without its colour buffer), one to three
/// groups (a skipped first, an 0xff or other terminator), triangles in
/// and out of the screen, one instance or four bands, the extra matrix,
/// the table exchange both ways, the push with and without z, a
/// 2D cut (and a fading movie), the magic special cases; and on a
/// snapshot every live part of kind 3 in the six fill buffers.
#[test]
fn dtt_draw_mdl_dtt() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_56c0);
        for (sym, n) in [
            (dtt::IN_CUT_2D, 0),
            (dtt::SINGLE_INSTANCE, 0),
            (dtt::BUFFER_BY_INDEX, 1),
            (dtt::MEMSET, 3),
            (dtt::MAGIC_ID, 0),
            (dtt::UPDATE_PARAMETER, 4),
        ] {
            t.stub(sym, n, false);
        }
        t.stub(dtt::CUT_2D_BIAS, 0, true);
        let tags = [
            dtt::IN_CUT_2D,
            dtt::SINGLE_INSTANCE,
            dtt::BUFFER_BY_INDEX,
            dtt::MAGIC_ID,
        ];
        let ebp = t.ebp_for(1);
        let frame = ebp - 0x1_4114;
        // all but ebp-8, where __alloca_probe saved the caller's ecx (the
        // direction's w until the first triangle's)
        let blocks = [(0, dtt::FRAME as usize - 4)];
        let mut seen = [0u32; 4];
        for i in 0..400 {
            t.fresh(&mut rng, &tags);
            if rng.below(3) == 0 {
                t.constants(
                    &mut rng,
                    &[
                        dtt::MIN_START,
                        dtt::WIDTH_CUT,
                        dtt::WIDTH,
                        dtt::LIMIT,
                        dtt::BOUND_LOW,
                        dtt::BOUND_HIGH,
                        dtt::MINUS_ONE,
                    ],
                    &[
                        dtt::QUARTER,
                        dtt::ZERO,
                        dtt::THREE,
                        dtt::FOUR,
                        dtt::THIRD,
                        dtt::INV_255,
                        dtt::MIN_LENGTH,
                        dtt::CENTRE,
                        dtt::HEIGHT,
                        dtt::HALF,
                        dtt::CUT_SCALE,
                    ],
                );
            } else {
                dtt_real_constants(&mut t);
            }
            t.ret(dtt::IN_CUT_2D, rng.below(2));
            t.g32(dtt::MOVIE_FADING, rng.pick(&[0, 0, 1]));
            let bias = f64::from(rng.below(100) as f32 / 9.0);
            t.ret_f64(dtt::CUT_2D_BIAS, bias);
            let single = rng.below(3) == 0;
            t.ret(dtt::SINGLE_INSTANCE, u32::from(single));
            t.ret(dtt::MAGIC_ID, rng.pick(&[0x46, 0x23a, 0x10, 0x46]));
            // the shape: points and groups
            let count = rng.pick(&[0u16, 1, 3, 8, 8]);
            let pts = count.max(1);
            let groups_n = rng.below(3) + 1;
            let tris: Vec<u32> = (0..groups_n).map(|_| rng.pick(&[0, 1, 2, 4])).collect();
            let total: u32 = tris.iter().sum();
            let shape = t.zeros(0x20);
            t.w16(shape + 0x12, count);
            let points = t.zeros(6 * usize::from(pts));
            for k in 0..3 * u32::from(pts) {
                // sometimes on the screen's edges (scale 1: 0 or 4096)
                let v = if rng.below(6) == 0 {
                    rng.pick(&[2048i32, -2048, 0]) as u16
                } else {
                    (rng.below(1601) as i32 - 800) as u16
                };
                t.w16(points + 2 * k, v);
            }
            t.w32(shape + 8, points.wrapping_sub(shape));
            let gsize: u32 = tris.iter().map(|n| 0x10 + 0x14 * n).sum::<u32>() + 0x10;
            let groups = t.zeros(gsize as usize);
            let mut at = groups;
            for (g, &n) in tris.iter().enumerate() {
                let skip = g == 0 && rng.below(10) == 0;
                t.w8(at + 1, u8::from(skip));
                t.w16(at + 2, n as u16);
                for k in 0..n {
                    let rec = at + 0x10 + 0x14 * k;
                    for j in 0..12 {
                        let b = rng.next() as u8;
                        t.w8(rec + j, b);
                    }
                    for j in 0..3 {
                        let idx = rng.below(u32::from(pts)) as u16;
                        t.w16(rec + 0xc + 2 * j, idx);
                    }
                }
                at += 0x10 + 0x14 * n;
            }
            t.w8(at + 1, rng.pick(&[0xff, 0xff, 1]));
            t.w32(shape + 4, groups.wrapping_sub(shape));
            // the part
            let part = random_block(&mut t, &mut rng, 0x214);
            t.w32(part + 0x150, shape);
            matrix(&mut t, &mut rng, part + 0x30);
            if rng.below(2) == 0 {
                // the identity
                t.write(part + 0x30, &[0; 0x40]);
                for k in 0..4 {
                    t.w32(part + 0x30 + 0x14 * k, 1f32.to_bits());
                }
            }
            view_matrix(&mut t, &mut rng, part + 0x70);
            matrix(&mut t, &mut rng, part + 0xb0);
            view_matrix(&mut t, &mut rng, part + 0xf0);
            small_floats(&mut t, &mut rng, part + 0x60, 4, 7000);
            small_floats(&mut t, &mut rng, part + 0x130, 4, 2000);
            small_floats(&mut t, &mut rng, part + 0x140, 2, 3000);
            small_floats(&mut t, &mut rng, part + 0x164, 2, 20);
            for off in [0x154, 0x158, 0x160, 0x16c] {
                t.w32(part + off, rng.pick(&[0, 0, 1, 0x100]));
            }
            // float bits without signalling NaNs (the emulator keeps them)
            let table = t.zeros((0x30 * (total + 1) + 0x10) as usize);
            floats(&mut t, &mut rng, table, 0xc * (total + 1) + 4);
            t.w32(part + 0x15c, table);
            // the render data
            let mut room = 0;
            for k in 0..4 {
                let rd = random_block(&mut t, &mut rng, 0xd0);
                let seg = t.zeros(0x20);
                let filled = rng.below(4) * 3;
                t.w32(seg + 4, filled + rng.pick(&[0, 0, 3]));
                t.w32(seg + 8, filled);
                room = filled / 3 + total + 1;
                for (off, size) in [(0xc, 0x24), (0x14, 0x30), (0x18, 0x18), (0x1c, 6)] {
                    let b = random_block(&mut t, &mut rng, size * (room + 3));
                    t.w32(seg + off, b);
                }
                t.w32(rd + 0x94, seg);
                t.w32(part + 0x10 + 4 * k, rd);
            }
            let rd0: u32 = Mem::read(&t.a, part + 0x10);
            let buffered = rng.below(2) == 0;
            t.w16(rd0 + 0xc2, if buffered { 7 } else { 0xffff });
            let mb = if buffered && rng.below(4) != 0 {
                let mb = random_block(&mut t, &mut rng, 0xa0);
                let v = t.zeros(16 * usize::from(pts));
                small_floats(&mut t, &mut rng, v, 4 * u32::from(pts), 7000);
                t.w32(mb + 0x14, v);
                let colours = if rng.below(2) == 0 {
                    0
                } else {
                    let c = t.zeros((0x30 * (total + 1)) as usize);
                    small_floats(&mut t, &mut rng, c, 12 * (total + 1), 2000);
                    c
                };
                t.w32(mb + 0x9c, colours);
                mb
            } else {
                0
            };
            t.ret(dtt::BUFFER_BY_INDEX, mb);
            let edge = tris[0] > 0 && rng.below(3) == 0;
            if edge {
                // exact cases on the first triangle: identity model, a view
                // of scale 1 (x' = x + 2048), the screen's edges, the
                // first corner exactly 5 from the model's origin (the limit 5)
                t.write(part + 0x30, &[0; 0x40]);
                t.write(part + 0x70, &[0; 0x40]);
                for k in 0..4 {
                    t.w32(part + 0x30 + 0x14 * k, 1f32.to_bits());
                    t.w32(part + 0x70 + 0x14 * k, 1f32.to_bits());
                }
                t.w32(part + 0x70 + 0x30, 2048f32.to_bits());
                t.w32(part + 0x70 + 0x34, 2048f32.to_bits());
                t.w32(part + 0x16c, 0);
                t.g32(dtt::LIMIT, 4096f32.to_bits());
                t.w16(rd0 + 0xc2, 0xffff);
                t.w8(groups + 1, 0);
                for j in 0..3u32 {
                    let idx = if pts >= 3 { j as u16 } else { 0 };
                    t.w16(groups + 0x10 + 0xc + 2 * j, idx);
                }
                // the first point (3, 4, 0), 5 from the model's origin
                // (the matrix's translation, +0x60), which puts it on the
                // screen's edges or near them
                t.w16(points, 3);
                t.w16(points + 2, 4);
                t.w16(points + 4, 0);
                let ox = rng.pick(&[2045f32, 2044.0, -2051.0, 0.0, 2045.0]);
                let oy = rng.pick(&[2044f32, -2052.0, 2043.0, 0.0, -2051.0]);
                for (k, v) in [ox, oy, 0.0, 1.0].into_iter().enumerate() {
                    t.w32(part + 0x60 + 4 * k as u32, v.to_bits());
                }
                let at = t.rebase(dtt::MIN_LENGTH);
                t.write(at, &rng.pick(&[5.0f64, 1e-6]).to_le_bytes());
            }
            let w = format!(
                "{name} {i}: points {count} tris {tris:?} single {single} buffer {mb:#x} room {room}"
            );
            t.check_framed(&w, 0x0072_56c0, &[part], frame, &blocks, |g| {
                u(dtt::draw_mdl_dtt(g, frame, part))
            });
            seen[usize::from(single) * 2 + usize::from(mb != 0)] += 1;
        }
        eprintln!("{name}: {seen:?}");
        // the snapshot's own parts
        let mut live = 0;
        for b in 0..6 {
            let parts: u32 = Mem::read(&t.a, t.rebase(0x00ce_c1a8) + 4 * b);
            let count: u32 = Mem::read(&t.a, t.rebase(0x00ce_c1c0) + 4 * b);
            if parts == 0 {
                continue;
            }
            for k in 0..count.min(0x200) {
                let part = parts + 0x214 * k;
                if Mem::read::<u8>(&t.a, part) != 3 {
                    continue;
                }
                t.fresh(&mut rng, &tags);
                let w = format!("{name} buffer {b} part {k}");
                t.check_framed(&w, 0x0072_56c0, &[part], frame, &blocks, |g| {
                    u(dtt::draw_mdl_dtt(g, frame, part))
                });
                live += 1;
            }
        }
        eprintln!("{name}: {live} live parts");
    }
}
