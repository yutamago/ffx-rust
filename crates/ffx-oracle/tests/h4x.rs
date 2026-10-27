//! Round h4x of list `h4-glue-other` (`ffx_glue_other::h4x_*`)
//! against the original: on the image alone and on a snapshot
//! (`FFX_SNAPSHOT`). Results and every byte written must agree. Needs
//! `FFX_GAME_DIR`. The helpers are those of `h4_glue_other_g.rs`.
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
use ffx_glue_other::{h4_j2 as j2, h4_j4 as j4, h4x_video as xv};
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

// ---- batch 1 (ffx_glue_other::h4x_video)

/// A thiscall stub at `sym` returning its `ecx` (as `LocString_Load`
/// does), logging `ecx` and `n` words and popping `pops`.
fn this_stub(t: &mut Twins, sym: u32, n: u32, pops: u32) {
    let log = t.stub_code(sym, true, n, pops, false, &[]);
    let cell = t.cells[&sym];
    let mut c = vec![0x89, 0x0d]; // mov [cell], ecx
    c.extend(cell.to_le_bytes());
    jmp_to(t, &mut c, log);
    let f = t.code(&c);
    t.jump(sym, f);
}

/// A thiscall stub at `sym` that copies the four words at `cells` through
/// its first stack argument and counts the first one up, then logs `ecx`
/// and `n` words and pops `pops`.
fn out4_this_stub(t: &mut Twins, sym: u32, n: u32, pops: u32, cells: u32) {
    let log = t.stub_code(sym, true, n, pops, false, &[]);
    let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
    for k in 0..4u32 {
        c.extend([0x8b, 0x15]);
        c.extend((cells + 4 * k).to_le_bytes()); // mov edx, [cell k]
        c.extend([0x89, 0x50, (4 * k) as u8]); // mov [eax+4k], edx
    }
    c.extend([0xff, 0x05]);
    c.extend(cells.to_le_bytes()); // inc dword [cells]: each call's value differs
    jmp_to(t, &mut c, log);
    let f = t.code(&c);
    t.jump(sym, f);
}

/// A cdecl stub at `sym` that appends its `n` stack words to a list
/// (`list` holds the next address) and returns `ret`'s word.
fn wide_stub(t: &mut Twins, sym: u32, n: u32, list: u32, ret: u32) {
    let mut c = vec![0x56, 0x57, 0x8d, 0x74, 0x24, 0x0c, 0x8b, 0x3d]; // push esi; push edi; lea esi, [esp+0xc]; mov edi, [list]
    c.extend(list.to_le_bytes());
    c.push(0xb9);
    c.extend(n.to_le_bytes()); // mov ecx, n
    c.extend([0xf3, 0xa5, 0x89, 0x3d]); // rep movsd; mov [list], edi
    c.extend(list.to_le_bytes());
    c.extend([0x5f, 0x5e, 0xa1]); // pop edi; pop esi; mov eax, [ret]
    c.extend(ret.to_le_bytes());
    c.push(0xc3);
    let f = t.code(&c);
    t.jump(sym, f);
}

/// A `sprintf` stand-in in import slot `iat`: copies the next string of a
/// list (`seq` holds its address) into its first argument, then logs four
/// words when the format is `four` (tag `tag4`), else three (tag `tag3`).
fn sprintf_stub(t: &mut Twins, iat: u32, seq: u32, four: u32, tags: (u32, u32)) {
    let log3 = t.stub_code(tags.0, false, 3, 0, false, &[]);
    let log4 = t.stub_code(tags.1, false, 4, 0, false, &[]);
    let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x15]; // mov eax, [esp+4]; mov edx, [seq]
    c.extend(seq.to_le_bytes());
    // copy: mov cl, [edx]; mov [eax], cl; inc eax; inc edx; test cl, cl; jnz copy
    c.extend([0x8a, 0x0a, 0x88, 0x08, 0x40, 0x42, 0x84, 0xc9, 0x75, 0xf6]);
    c.extend([0x89, 0x15]);
    c.extend(seq.to_le_bytes()); // mov [seq], edx
    c.extend([0x81, 0x7c, 0x24, 0x08]);
    c.extend(four.to_le_bytes()); // cmp dword [esp+8], four
    c.extend([0x0f, 0x84]);
    let at = t.cave_next + c.len() as u32 + 4;
    c.extend(log4.wrapping_sub(at).to_le_bytes()); // je log4
    jmp_to(t, &mut c, log3);
    let addr = t.code(&c);
    t.w32(iat, addr);
}

/// `operator new` as a bump allocator over the block `heap` points at
/// (16-byte steps), logging its word.
fn new_stub(t: &mut Twins, sym: u32, heap: u32) {
    let log = t.stub_code(sym, false, 1, 0, false, &[]);
    let cell = t.cells[&sym];
    let mut c = vec![0xa1];
    c.extend(heap.to_le_bytes()); // mov eax, [heap]
    c.push(0xa3);
    c.extend(cell.to_le_bytes()); // mov [cell], eax
    c.extend([0x8b, 0x54, 0x24, 0x04, 0x83, 0xc2, 0x0f, 0x83, 0xe2, 0xf0]); // mov edx, [esp+4]; add edx, 15; and edx, -16
    c.extend([0x01, 0x15]);
    c.extend(heap.to_le_bytes()); // add [heap], edx
    jmp_to(t, &mut c, log);
    let f = t.code(&c);
    t.jump(sym, f);
}

/// `IggyStringArray_ctor` as a stub: `[ecx]` and `[ecx + 4]` from the two
/// cells at `cells`, returns `ecx`; logs `ecx` and its three words, pops
/// them.
fn string_array_stub(t: &mut Twins, sym: u32, cells: u32) {
    let log = t.stub_code(sym, true, 3, 3, false, &[]);
    let cell = t.cells[&sym];
    let mut c = vec![0xa1];
    c.extend(cells.to_le_bytes()); // mov eax, [cells]
    c.extend([0x89, 0x01, 0xa1]); // mov [ecx], eax
    c.extend((cells + 4).to_le_bytes()); // mov eax, [cells + 4]
    c.extend([0x89, 0x41, 0x04, 0x89, 0x0d]); // mov [ecx+4], eax; mov [cell], ecx
    c.extend(cell.to_le_bytes());
    jmp_to(t, &mut c, log);
    let f = t.code(&c);
    t.jump(sym, f);
}

/// A random lower-case string of up to `max` letters, NUL-terminated.
fn word(rng: &mut Rng, max: u32) -> Vec<u8> {
    let mut s: Vec<u8> = (0..rng.below(max + 1))
        .map(|_| b'a' + rng.below(26) as u8)
        .collect();
    s.push(0);
    s
}

/// The escape menu's video setting page: with and without the resolution
/// list, every setting's range (row 7's value missing from its table
/// too), the render interface made or already there, its level on both
/// sides of `0xa000`.
#[test]
fn x1_video_setting_page() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_4a01);
        let (out, flash6, flash3, arrays) = (t.zeros(16), t.zeros(4), t.zeros(4), t.zeros(8));
        let stubs = [
            (xv::OPERATOR_DELETE, false, 1, 0),
            (xv::IGGY_ROOT, true, 0, 0),
            (xv::VECTOR_DTOR, false, 4, 4),
            (xv::LOCK_ENTER, true, 0, 0),
            (xv::LOCK_LEAVE, true, 0, 0),
            (xv::RENDER_CONSTRUCT, true, 0, 0),
            (xv::ATEXIT, false, 1, 0),
        ];
        for (sym, this, n, pops) in stubs {
            t.stub_at(sym, this, n, pops, &[]);
        }
        this_stub(&mut t, j2::LOC_LOAD, 1, 1);
        out4_this_stub(&mut t, j2::LOC_TO_VALUE, 1, 1, out);
        let (list6, list3) = (t.zeros(4), t.zeros(4));
        wide_stub(&mut t, j2::CALL_FLASH_6, 26, list6, flash6);
        wide_stub(&mut t, j4::CALL_FLASH_3, 14, list3, flash3);
        let seq = t.zeros(4);
        let iat = t.rebase(j2::SPRINTF_IAT);
        let four = t.rebase(xv::RESOLUTION_FORMAT);
        sprintf_stub(&mut t, iat, seq, four, (0x0bad_4a03, 0x0bad_4a04));
        let heap = t.zeros(4);
        new_stub(&mut t, xv::OPERATOR_NEW, heap);
        string_array_stub(&mut t, xv::STRING_ARRAY, arrays);
        let free = t.stub_code(xv::VALUE_REF_FREE_IAT, false, 2, 2, false, &[]);
        t.g32(xv::VALUE_REF_FREE_IAT, free);
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.extend([j2::LOC_LOAD, xv::STRING_ARRAY, xv::VALUE_REF_FREE_IAT]);
        let (mut made, mut missing, mut listed) = (0, 0, 0);
        for i in 0..48 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            for cell in [
                out,
                out + 4,
                out + 8,
                out + 12,
                flash6,
                flash3,
                arrays,
                arrays + 4,
            ] {
                let v = rng.next();
                t.w32(cell, v);
            }
            // the resolution list
            let this = random_block(&mut t, &mut rng, 0xe0);
            let count = rng.below(6);
            let resolutions = if i % 4 == 3 {
                0
            } else {
                listed += 1;
                random_block(&mut t, &mut rng, 0x1c * count.max(1))
            };
            t.w32(this + 0xdc, resolutions);
            t.w32(this + 0xd4, count);
            let current = rng.below(count.max(1));
            t.w32(this + 0xd8, current);
            let mut text = Vec::new();
            for _ in 0..8 {
                text.extend(word(&mut rng, 9));
            }
            let src = t.alloc(&text);
            t.w32(seq, src);
            let block = t.zeros(0x200);
            t.w32(heap, block);
            // the settings, each in its list's range (row 7: 0, 2, 4, 8 or
            // something else)
            let settings = random_block(&mut t, &mut rng, 0x50);
            for (off, n) in [
                (0x08, 3),
                (0x28, 2),
                (0x34, 3),
                (0x2c, 5),
                (0x30, 4),
                (0x3c, 2),
                (0x40, 6),
                (0x44, 2),
                (0x48, 4),
            ] {
                let v = rng.below(n);
                t.w32(settings + off, v);
            }
            let quality = rng.pick(&[0, 2, 4, 8, 1, 3, 16, u32::MAX]);
            missing += u32::from(![0, 2, 4, 8].contains(&quality));
            t.w32(settings + 0x38, quality);
            t.g32(j2::SETTINGS, settings);
            // the multipliers' image bytes ("1x", "2x", "4x", "8x" and their
            // NULs; any non-zero bytes here)
            let multipliers = t.rebase(xv::MULTIPLIERS);
            for k in 0..16 {
                let b = 1 + rng.below(255) as u8;
                t.w8(multipliers + k, b);
            }
            // the render interface
            let guard = t.rebase(xv::RENDER_GUARD);
            let bits = rng.next() & !1 | u32::from(i % 3 == 0);
            made += u32::from(bits & 1 == 0);
            t.w32(guard, bits);
            let level = rng.pick(&[0x9100, 0x9300, 0xa000, 0xa001, 0xa100, 0xb000, 0x8000_0000]);
            t.g32(xv::RENDER_LEVEL, level);
            let calls = t.zeros(13 * 26 * 4);
            t.w32(list6, calls);
            let popup = t.zeros(14 * 4);
            t.w32(list3, popup);
            let frame = t.ebp_for(0) - xv::VIDEO_FRAME;
            let ebp = frame + xv::VIDEO_FRAME;
            // the LocStrings' texts (their constructor is a stub)
            for slot in [0x6e0u32, 0x5d0, 0x4c0, 0x3b0, 0x2a0, 0x190] {
                let s = word(&mut rng, 12);
                t.write(ebp - slot, &s);
            }
            let mut compare: Vec<(u32, usize)> =
                [0xa4cu32, 0xa5c, 0xa6c, 0xa7c, 0xa8c, 0xaa8, 0xab8, 0xac8]
                    .iter()
                    .map(|&off| (xv::VIDEO_FRAME - off, 16))
                    .collect();
            compare.push((xv::VIDEO_FRAME - 0x80, 0x70));
            // `ebp - 0xa38` (the two-text lists' second entry) is a value copy
            // too: it reaches the Flash call through the current choice
            compare.push((xv::VIDEO_FRAME - 0xa3c, 4));
            compare.push((xv::VIDEO_FRAME - 0xa90, 4));
            t.check_framed_conv(
                &w,
                0x006f_cca0,
                Conv::Thiscall(this),
                &[],
                frame,
                &compare,
                |g| u(xv::video_setting_page(g, frame, this)),
            );
            let rows = u32::from(resolutions != 0) + 11;
            let end: u32 = Mem::read(&t.b, list6);
            assert_eq!(end - calls, rows * 26 * 4, "{w}: rows");
            let end: u32 = Mem::read(&t.b, list3);
            assert_eq!(end - popup, 14 * 4, "{w}: pop-up");
        }
        assert!(
            made > 0 && missing > 0 && listed > 0,
            "{name}: {made} {missing} {listed}"
        );
    }
}
