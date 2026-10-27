//! M4 list `h4-glue-other`, agent h4z (`ffx_glue_other::h4z_*`): the
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
use ffx_glue_other::h4z_mdl as mdl;
use ffx_glue_other::h4z_quad as quad;
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
        let tramp = self.tramp;
        let ra = match self.a.call(addr, conv, args) {
            Ok(r) => r,
            Err(e) => {
                // the port on the same state, for its view of the fault
                port(&mut G(&mut self.b, tramp));
                panic!("{what}: the original: {e}");
            }
        };
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
// ---- JobSchedule_pppPart_fillBuffer_drawMdl (ffx_glue_other::h4z_*)

/// The original's address.
const DRAW_MDL: u32 = 0x0071_f320;

/// Where the original's `ebp` lands for a one-word call of a function that
/// aligns its frame to 16 bytes (`and esp, -16` after `push ebx`).
fn aligned_ebp(t: &Twins) -> u32 {
    let entry = t.a.stack_range().end - 0x100 - 4 - 4;
    (entry - 12) & !0xf
}

/// The image constants `drawMdl` reads.
const F64S: [u32; 13] = [
    mdl::INV_4096,
    mdl::THIRD,
    mdl::QUARTER,
    mdl::ZERO,
    mdl::THREE,
    mdl::FOUR,
    mdl::HALF,
    mdl::MIN_W,
    mdl::TENTH,
    mdl::THOUSAND,
    mdl::X_SCALE,
    quad::EIGHTH,
    quad::THIRD_F32,
];
/// See [`F64S`].
const F32S: [u32; 6] = [
    mdl::RANGE_MIN,
    mdl::RANGE_MAX,
    mdl::OPAQUE,
    mdl::MINUS_ONE,
    mdl::BOUND_LOW,
    mdl::BOUND_HIGH,
];

/// The image's own values of [`F64S`] and [`F32S`]: (address, bytes).
fn image_constants(t: &Twins) -> Vec<(u32, Vec<u8>)> {
    let mut out = Vec::new();
    for (list, n) in [(&F64S[..], 8usize), (&F32S[..], 4)] {
        for &c in list {
            let at = t.rebase(c);
            let mut b = vec![0; n];
            t.a.read_bytes(at, &mut b);
            out.push((at, b));
        }
    }
    out
}

/// A position word: small, never a signalling NaN's high half when read
/// as an `f32` (-128..-65).
fn coord(rng: &mut Rng) -> u16 {
    loop {
        let v = if rng.below(10) == 0 {
            rng.next() as u16
        } else {
            (rng.below(1601) as i32 - 800) as u16
        };
        let hi = v & 0xffc0;
        if hi != 0xff80 && hi != 0x7f80 {
            return v;
        }
    }
}

/// Floats in `0..2` (colours around the 0.5 opaque limit), or random bits.
fn colour_floats(t: &mut Twins, rng: &mut Rng, at: u32, n: u32) {
    for k in 0..n {
        let v = if rng.below(8) == 0 {
            rng.float()
        } else {
            (rng.below(400) as f32 / 200.0).to_bits()
        };
        t.w32(at + 4 * k, v);
    }
}

/// Bytes of a record of each kind.
const SIZES: [u32; 8] = [0x14, 0x1c, 0x20, 0x28, 0x18, 0x20, 0x28, 0x30];

/// Texture page codes the original tests for.
const CODES: [u32; 21] = [
    0x37c0, 0x3780, 0x3800, 0x3840, 0x3880, 0x3900, 0x39c0, 0x3940, 0x3980, 0x3400, 0x35c0, 0x3680,
    0x3760, 0x37a0, 0x3740, 0x3580, 0x3581, 0x3a00, 0x39ff, 0x1234, 0x3880,
];

/// The stubs' tags.
const TAGS: [u32; 4] = [
    mdl::LEVEL_HASH,
    mdl::MAGIC_ID,
    mdl::BUFFER_BY_INDEX,
    mdl::USE_POINT_LIGHT,
];

/// The original's callees that stay stubs.
fn mdl_stubs(t: &mut Twins) {
    for (sym, n) in [
        (mdl::LEVEL_HASH, 0),
        (mdl::MAGIC_ID, 0),
        (mdl::BUFFER_BY_INDEX, 1),
        (mdl::MEMSET, 3),
        (mdl::USE_POINT_LIGHT, 0),
        (mdl::POINT_LIGHT_COLOURS, 5),
        (mdl::UPDATE_PARAMETER, 4),
        (mdl::USE_OPTIMIZED, 2),
    ] {
        t.stub(sym, n, false);
    }
}

/// What a random part is built from.
struct Shape {
    /// The groups' kinds (8 and up: skipped).
    kinds: Vec<u8>,
    /// Primitives per group.
    counts: Vec<u16>,
    mode: i8,
    buffered: bool,
    /// The part's `+0x104` (0x33e0: kind 6's tiles; other nonzero values
    /// leave kind 6's records unread, so only a last group has any).
    flag104: u32,
    /// Identity matrices (depths are the positions' z).
    exact: bool,
}

/// The identity matrix at `at`.
fn identity(t: &mut Twins, at: u32) {
    t.write(at, &[0; 0x40]);
    for k in 0..4 {
        t.w32(at + 0x14 * k, 1f32.to_bits());
    }
}

/// A random part of `s`; a description.
fn mdl_part(t: &mut Twins, rng: &mut Rng, s: &Shape) -> (u32, String) {
    let points = u32::from(rng.pick(&[1u16, 3, 8, 20, 64]));
    let normal_count = rng.below(6) + 1;
    let maxn = u32::from(*s.counts.iter().max().unwrap_or(&0));
    let groups_n = s.kinds.len() as u32;
    // positions: i16 triples, also read as f32 vectors (flags 0x400)
    let positions = t.zeros((16 * points) as usize);
    for k in 0..8 * points {
        let v = coord(rng);
        t.w16(positions + 2 * k, v);
    }
    if s.exact {
        // z in 0..4: band edges and means on integers
        for p in 0..points {
            let z = rng.below(5) as u16;
            t.w16(positions + 6 * p + 4, z);
        }
    }
    let normals = t.zeros((6 * normal_count) as usize);
    for k in 0..3 * normal_count {
        let v = rng.next() as u16;
        t.w16(normals + 2 * k, v);
    }
    // the groups
    let mut bytes = 0;
    for (g, &kind) in s.kinds.iter().enumerate() {
        bytes += 0x10;
        if kind < 8 {
            bytes += SIZES[kind as usize] * u32::from(s.counts[g]);
        }
    }
    let model = t.zeros((0x20 + bytes + 0x10) as usize);
    let total: i32 = s.counts.iter().map(|&n| i32::from(n)).sum();
    t.w16(model + 0x10, total as u16);
    t.w16(model + 0x12, points as u16);
    t.w32(model + 4, 0x20);
    let mut at = model + 0x20;
    for (g, &kind) in s.kinds.iter().enumerate() {
        let n = u32::from(s.counts[g]);
        // kind 6's shifted page more often
        let code = if kind == 6 && rng.below(4) == 0 {
            0x3400
        } else {
            rng.pick(&CODES)
        };
        let lo = (rng.next() & !0x3fff) | code;
        let hi = rng.next();
        t.w8(at, rng.next() as u8);
        t.w8(
            at + 1,
            if kind < 8 {
                kind
            } else {
                rng.pick(&[8, 0xff, 0x80])
            },
        );
        t.w16(at + 2, n as u16);
        t.w32(at + 8, lo);
        t.w32(at + 0xc, hi);
        at += 0x10;
        if kind >= 8 {
            continue;
        }
        let size = SIZES[kind as usize];
        let corners = if kind < 4 { 3 } else { 4 };
        let col = if kind < 4 { 12 } else { 16 };
        for r in 0..n {
            let rec = at + size * r;
            for j in 0..col {
                let b = if rng.below(2) == 0 {
                    rng.below(0x40) as u8
                } else {
                    rng.next() as u8
                };
                t.w8(rec + j, b);
            }
            for j in 0..corners {
                let p = rng.below(points) as u16;
                t.w16(rec + col + 2 * j, p);
            }
            for j in (col + 2 * corners..size).step_by(2) {
                // texture coordinates (then normals, below)
                let v = if rng.below(3) == 0 {
                    rng.next() as u16
                } else {
                    rng.below(0x2000) as u16
                };
                t.w16(rec + j, v);
            }
            let nat = match kind {
                1 => Some(0x14),
                3 => Some(0x20),
                5 => Some(0x18),
                7 => Some(0x28),
                _ => None,
            };
            if let Some(o) = nat {
                for j in 0..corners {
                    let p = rng.below(normal_count) as u16;
                    t.w16(rec + o + 2 * j, p);
                }
            }
        }
        at += size * n;
    }
    // the render data
    let room = maxn + groups_n + 3;
    let mut rds = [0u32; 4];
    for rd in &mut rds {
        *rd = random_block(t, rng, 0xd0);
        let geo = t.zeros((0x6c * groups_n.max(1)) as usize);
        for g in 0..groups_n.max(1) {
            let seg = geo + 0x6c * g;
            // counts that disagree too (the memsets are stubs)
            let filled = rng.below(3);
            t.w32(seg + 4, filled * 4 + rng.pick(&[0, 0, 3]));
            t.w32(seg + 8, filled * 6 + rng.pick(&[0, 0, 6]));
            for (off, size) in [
                (0xc, 0x30),
                (0x10, 0x30),
                (0x14, 0x40),
                (0x18, 0x20),
                (0x1c, 0xc),
            ] {
                let b = random_block(t, rng, size * (room + 2 * filled + 2));
                t.w32(seg + off, b);
            }
        }
        t.w32(*rd + 0x94, geo);
    }
    t.w8(rds[0] + 0xc0, s.mode as u8);
    t.w16(rds[0] + 0xc2, if s.buffered { 7 } else { 0xffff });
    t.w16(rds[0] + 0xc4, rng.pick(&[0u16, 1, 2, 4, 9]));
    // the model buffer
    let mb = if s.buffered && rng.below(5) != 0 {
        let mb = random_block(t, rng, 0x140);
        let v = t.zeros((16 * points) as usize);
        small_floats(t, rng, v, 4 * points, 7000);
        t.w32(mb + 0x14, v);
        for g in 0..groups_n {
            let uv = if rng.below(2) == 0 {
                0
            } else {
                let b = t.zeros((0x20 * (maxn + 1)) as usize);
                floats(t, rng, b, 8 * (maxn + 1));
                b
            };
            t.w32(mb + 0x24 + 4 * g, uv);
            let col = if rng.below(2) == 0 {
                0
            } else {
                let b = t.zeros((0x40 * (maxn + 1)) as usize);
                colour_floats(t, rng, b, 16 * (maxn + 1));
                b
            };
            t.w32(mb + 0x9c + 4 * g, col);
            let verts = if rng.below(3) == 0 {
                let b = t.zeros((0x30 * room) as usize);
                small_floats(t, rng, b, 12 * room, 7000);
                b
            } else {
                0
            };
            t.w32(mb + 0x114 + 4 * g, verts);
        }
        mb
    } else {
        0
    };
    t.ret(mdl::BUFFER_BY_INDEX, mb);
    // the part
    let part = random_block(t, rng, 0x214);
    t.w8(part + 1, rng.pick(&[0, 0, 1]));
    let flags = rng.next() & !0x450 | rng.pick(&[0, 0x10, 0x40, 0x400, 0x410, 0x450, 0x50]);
    t.w32(part + 0x10, flags);
    t.w32(part + 0x14, positions);
    t.w32(part + 0x18, model);
    t.w8(part + 0x1e, rng.pick(&[0x44, 0, 0x41]));
    for m in [0x20, 0x60, 0x1d4] {
        matrix(t, rng, part + m);
        if s.exact {
            identity(t, part + m);
        }
    }
    let scale = rng.pick(&[1.0f32 / 255.0, 1.0 / 128.0, 1.0, 0.004]);
    for k in 0..4 {
        let v = if rng.below(6) == 0 {
            rng.float()
        } else {
            scale.to_bits()
        };
        t.w32(part + 0xb0 + 4 * k, v);
    }
    for (k, &rd) in (0u32..).zip(&rds) {
        t.w32(part + 0xc0 + 4 * k, rd);
    }
    t.w32(part + 0x100, normals);
    t.w32(part + 0x104, s.flag104);
    let desc = format!(
        "kinds {:?} counts {:?} mode {} mb {mb:#x} flags {flags:#x} points {points}",
        s.kinds, s.counts, s.mode
    );
    (part, desc)
}

/// Runs `drawMdl` on `part` in both oracles: the frames zeroed as the
/// shim's, the port's calls below its frame; compares the result, every
/// written byte and the frame blocks callees see or the port keeps there.
fn mdl_check(t: &mut Twins, what: &str, part: u32) {
    let ebp = aligned_ebp(t);
    let frame = ebp - 0x1_4a20;
    t.write(frame - 0x10, &vec![0; (mdl::FRAME + 0x14) as usize]);
    t.a.clear_dirty();
    t.b.clear_dirty();
    t.b.set_call_top(Some(frame - 0x100));
    t.check(what, DRAW_MDL, &[part], |g| {
        u(mdl::draw_mdl(g, frame, part))
    });
    t.b.set_call_top(None);
    for (off, len) in [
        (mdl::FAKE_PART, 0x20usize),
        (mdl::JOB, 0x50),
        (mdl::POSITIONS, (mdl::POSITIONS - 4) as usize),
    ] {
        let at = mdl::slot(frame, off);
        let mut x = vec![0; len];
        let mut y = vec![0; len];
        t.a.read_bytes(at, &mut x);
        t.b.read_bytes(at, &mut y);
        if let Some(k) = (0..len).find(|&k| x[k] != y[k]) {
            let end = (k + 16).min(len);
            panic!(
                "{what}: frame ebp-{:#x} differs: original {:x?}, port {:x?}",
                off as usize - k,
                &x[k..end],
                &y[k..end]
            );
        }
    }
}

/// `drawMdl` on random parts whose groups are of `kinds` (one picked per
/// group, sometimes a skipped kind): every mode, with and without the
/// model buffer (its texture coordinates, colours and vertices), the
/// flags' 0x10/0x40/0x400, the magic and level special cases, random or
/// real constants.
fn mdl_random(kinds: &[u8], seed: u32, rounds: u32) {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(seed);
        mdl_stubs(&mut t);
        let image = image_constants(&t);
        let mut modes = [0u32; 3];
        for i in 0..rounds {
            t.fresh(&mut rng, &TAGS);
            if rng.below(4) == 0 {
                t.constants(&mut rng, &F32S, &F64S);
            } else {
                for (at, b) in &image {
                    t.write(*at, b);
                }
            }
            let exact = rng.below(4) == 0;
            let screen = t.rebase(mdl::SCREEN_MATRIX);
            matrix(&mut t, &mut rng, screen);
            if exact {
                // depths = z (integers), the mean's rounding across a band
                // edge (3 x third just above 1), the w test's edge
                identity(&mut t, screen);
                let third =
                    rng.pick(&[0.333_333_333_333_334f64, 1.0 / 3.0, 0.333_330_005_407_333_4]);
                let at = t.rebase(mdl::THIRD);
                t.write(at, &third.to_le_bytes());
                let w = rng.pick(&[1.0f64, 0.5, 1e-5]);
                let at = t.rebase(mdl::MIN_W);
                t.write(at, &w.to_le_bytes());
            }
            t.ret(mdl::LEVEL_HASH, rng.pick(&[0x1f7, 0x52b, 0, 7]));
            t.ret(
                mdl::MAGIC_ID,
                if kinds == [6] {
                    rng.pick(&[0x241, 0x66, 0x67, 0x1ba, 0x10])
                } else {
                    rng.pick(&[
                        0x9a, 0x241, 0x66, 0x67, 0x1ba, 0x207, 0x133, 0x264, 0xbc, 0x15f, 0x10,
                    ])
                },
            );
            t.ret(mdl::USE_POINT_LIGHT, rng.pick(&[0, 0, 1]));
            let groups = if rng.below(8) == 0 {
                10
            } else {
                rng.below(3) + 1
            };
            let s = Shape {
                kinds: (0..groups)
                    .map(|_| {
                        if rng.below(12) == 0 {
                            8
                        } else {
                            rng.pick(kinds)
                        }
                    })
                    .collect(),
                counts: (0..groups).map(|_| rng.pick(&[0u16, 1, 2, 3, 5])).collect(),
                mode: rng.pick(&[0i8, 0, 1, 1, 2, 3, -1]),
                buffered: rng.below(2) == 0,
                flag104: rng.pick(&[0, 0, 0, 0x33e0, 0x33e0, 5]),
                exact: false,
            };
            let mut s = s;
            s.exact = exact;
            if s.flag104 == 5 {
                let last = s.kinds.len() - 1;
                for (g, (n, &k)) in s.counts.iter_mut().zip(&s.kinds).enumerate() {
                    if k == 6 && g != last {
                        *n = 0;
                    }
                }
            }
            let mode = s.mode;
            let (part, desc) = mdl_part(&mut t, &mut rng, &s);
            mdl_check(&mut t, &format!("{name} {i}: {desc}"), part);
            modes[match mode {
                0 => 0,
                1 => 1,
                _ => 2,
            }] += 1;
        }
        eprintln!("{name}: modes {modes:?}");
    }
}

#[test]
fn mdl_kind0_flat_triangles() {
    mdl_random(&[0], 0x5eed_f320, 400);
}

#[test]
fn mdl_kind1_lit_triangles() {
    mdl_random(&[1], 0x5eed_f321, 400);
}

#[test]
fn mdl_kind2_textured_triangles() {
    mdl_random(&[2], 0x5eed_f322, 400);
}

#[test]
fn mdl_kind3_lit_textured_triangles() {
    mdl_random(&[3], 0x5eed_f323, 400);
}

#[test]
fn mdl_kind4_flat_quads() {
    mdl_random(&[4], 0x5eed_f324, 400);
}

#[test]
fn mdl_kind5_lit_quads() {
    mdl_random(&[5], 0x5eed_f325, 400);
}

#[test]
fn mdl_kind6_textured_quads() {
    mdl_random(&[6], 0x5eed_f326, 600);
}

#[test]
fn mdl_kind7_lit_textured_quads() {
    mdl_random(&[7], 0x5eed_f327, 400);
}

#[test]
fn mdl_mixed_kinds() {
    mdl_random(&[0, 1, 2, 3, 4, 5, 6, 7], 0x5eed_f328, 600);
}
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

/// Small ordinary floats over `n` words.
fn small_floats(t: &mut Twins, rng: &mut Rng, at: u32, n: u32, range: u32) {
    for k in 0..n {
        let v = (rng.below(2 * range + 1) as f32 - range as f32) / 7.0;
        t.w32(at + 4 * k, v.to_bits());
    }
}

/// `drawMdl` on every live part of kind 0 in the six fill buffers of a
/// snapshot (the stubs' results random).
#[test]
fn mdl_live_parts() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f329);
        mdl_stubs(&mut t);
        let mut live = 0;
        for b in 0..6 {
            let parts: u32 = Mem::read(&t.a, t.rebase(0x00ce_c1a8) + 4 * b);
            let count: u32 = Mem::read(&t.a, t.rebase(0x00ce_c1c0) + 4 * b);
            if parts == 0 {
                continue;
            }
            for k in 0..count.min(0x200) {
                let part = parts + 0x214 * k;
                if Mem::read::<u8>(&t.a, part) != 0 {
                    continue;
                }
                t.fresh(&mut rng, &TAGS);
                t.ret(mdl::BUFFER_BY_INDEX, 0);
                mdl_check(&mut t, &format!("{name} buffer {b} part {k}"), part);
                live += 1;
            }
        }
        eprintln!("{name}: {live} live parts");
    }
}

/// Mode 2 past 2000 triangles: a kind-0 group of 2003 then a kind-3
/// group (which fills the render data the split left current).
#[test]
fn mdl_mode2_split() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_f32a);
        mdl_stubs(&mut t);
        for i in 0..2 {
            t.fresh(&mut rng, &TAGS);
            let s = Shape {
                kinds: vec![0, 3],
                counts: vec![2003, 2],
                mode: 2,
                buffered: i == 1,
                flag104: 0,
                exact: false,
            };
            let (part, desc) = mdl_part(&mut t, &mut rng, &s);
            mdl_check(&mut t, &format!("{name} {i}: {desc}"), part);
        }
    }
}
