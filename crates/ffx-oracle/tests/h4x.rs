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
use ffx_glue_other::l11_btlpos::bridge as l11b;
use ffx_glue_other::l11_shp::bridge as l11s;
use ffx_glue_other::{h4_j2 as j2, h4_j4 as j4, h4x_shape as xs, h4x_video as xv};
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
    /// The cave's length ([`CAVE_LEN`], or more after [`Twins::big_cave`]).
    cave_len: u32,
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
            cave_len: CAVE_LEN,
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

    /// Moves the cave to 64 KiB of the oracles' own code memory (the
    /// same address in both), for tests with many stubs.
    fn big_cave(&mut self) {
        let fill = vec![0xcc; 0x1_0000];
        let at = self.a.alloc_code(&fill).unwrap();
        assert_eq!(self.b.alloc_code(&fill).unwrap(), at);
        self.cave = at;
        self.cave_next = at;
        self.cave_len = 0x1_0000;
    }

    /// Machine code in the cave; its address.
    fn code(&mut self, c: &[u8]) -> u32 {
        let addr = self.cave_next;
        assert!(
            addr + c.len() as u32 <= self.cave + self.cave_len,
            "cave full"
        );
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
        if !d.is_empty() && std::env::var_os("H4X_LOG").is_some() {
            let n: u32 = Mem::read(&self.a, self.log);
            for k in 0..n.min(200) {
                let at = self.log + 4 + 64 * k;
                let x: Vec<u32> = (0..10).map(|j| Mem::read(&self.a, at + 4 * j)).collect();
                let y: Vec<u32> = (0..10).map(|j| Mem::read(&self.b, at + 4 * j)).collect();
                eprintln!(
                    "{k} {at:#x} {x:x?}{}",
                    if x == y {
                        String::new()
                    } else {
                        format!("\n   port {y:x?}")
                    }
                );
            }
        }
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

// ---- batch 2 (ffx_glue_other::h4x_shape)

/// A recording stub at `sym` (`n` words) standing in for a transform: it
/// copies `words` words from its stack argument `src` into a ring of
/// 0x40-byte records (what it was handed), then `words` words from
/// `results` through its first argument (what it computes) and flips the
/// sign of `results`' first word. Returns the ring.
fn transform_stub(t: &mut Twins, sym: u32, n: u32, src: u8, words: u8, results: u32) -> u32 {
    let rest = t.stub_code(sym, false, n, 0, false, &[]);
    let ring = t.zeros(4 + 16 * 0x40);
    let mut c = vec![0x51, 0xa1]; // push ecx; mov eax, [ring]
    c.extend(ring.to_le_bytes());
    c.extend([0x83, 0xe0, 0x0f, 0xc1, 0xe0, 0x06, 0x05]); // and eax, 15; shl eax, 6; add eax,
    c.extend((ring + 4).to_le_bytes()); // ring + 4
    c.extend([0x8b, 0x54, 0x24, 8 + 4 * src]); // mov edx, [esp+8+4*src]
    for k in 0..words {
        c.extend([0x8b, 0x4a, 4 * k, 0x89, 0x48, 4 * k]); // mov ecx, [edx+4k]; mov [eax+4k], ecx
    }
    c.extend([0xff, 0x05]);
    c.extend(ring.to_le_bytes()); // inc dword [ring]
    c.extend([0x8b, 0x44, 0x24, 0x08]); // mov eax, [esp+8] (the destination)
    for k in 0..words {
        c.extend([0x8b, 0x0d]);
        c.extend((results + 4 * u32::from(k)).to_le_bytes()); // mov ecx, [results+4k]
        c.extend([0x89, 0x48, 4 * k]); // mov [eax+4k], ecx
    }
    c.extend([0x81, 0x35]);
    c.extend(results.to_le_bytes());
    c.extend(0x8000_0000u32.to_le_bytes()); // xor dword [results], sign (no signalling NaN)
    c.push(0x59); // pop ecx
    jmp_to(t, &mut c, rest);
    let f = t.code(&c);
    t.jump(sym, f);
    ring
}

/// `updateVFXRenderDataShapeField`: both layouts, empty and used buffers
/// (the first quad's depths, junk-read words 0), drawn and clipped quads,
/// tints and alphas of 0, the fade (below 0, between, above 128, NaN), the
/// uv remap of level 0x672's record 0x32c0, the depth correction of level
/// 0x1bbd's record 0x3980, the YIQ map, skipped frames and missing render
/// data; random and real constants.
#[test]
fn x2_update_shape_field() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_4a02);
        let (mul_out, apply_out, clip_out) = (t.zeros(64), t.zeros(64), t.zeros(64));
        transform_stub(&mut t, xs::MUL_MATRIX, 3, 2, 16, mul_out);
        transform_stub(&mut t, xs::APPLY_MATRIX, 3, 2, 4, apply_out);
        transform_stub(&mut t, xs::ZCLIP, 6, 0, 16, clip_out);
        let stubs = [
            (xs::LEVEL_HASH, 0),
            (xs::RENDER_DATA, 2),
            (xs::CURRENT_BUFFER, 0),
            (xs::ENABLE_YIQ, 2),
            (xs::SET_Z_CORRECTION, 3),
            (xs::SET_VISIBLE, 3),
        ];
        for (sym, n) in stubs {
            t.stub_at(sym, false, n, 0, &[]);
        }
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.extend([xs::MUL_MATRIX, xs::APPLY_MATRIX, xs::ZCLIP]);
        let f32s = [
            xs::ALPHA_SCALE,
            xs::F128,
            xs::F_HALF,
            xs::F_EIGHTH,
            xs::F_THREE_EIGHTHS,
            xs::F_FIVE_EIGHTHS,
            xs::F_QUARTER,
            xs::F_THREE_QUARTERS,
            xs::F_MINUS_ONE,
            xs::F_LOW,
            xs::F_HIGH,
        ];
        let f64s = [
            xs::INV_16,
            xs::INV_256,
            xs::INV_128,
            xs::EIGHT,
            xs::THIRTY_TWO,
            xs::INV_4096,
            xs::SEVEN_EIGHTHS,
            xs::EIGHTH,
            xs::QUARTER,
            xs::TWO,
        ];
        let mut seen = [0u32; 6];
        for i in 0..1500 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            let real = rng.below(2) == 0;
            t.constants(&mut rng, &f32s, &f64s);
            if real {
                for (c, v) in [
                    (xs::ALPHA_SCALE, 128.0f32),
                    (xs::F128, 128.0),
                    (xs::F_HALF, 0.5),
                    (xs::F_EIGHTH, 0.125),
                    (xs::F_THREE_EIGHTHS, 0.375),
                    (xs::F_FIVE_EIGHTHS, 0.625),
                    (xs::F_QUARTER, 0.25),
                    (xs::F_THREE_QUARTERS, 0.75),
                    (xs::F_MINUS_ONE, -1.0),
                    (xs::F_LOW, -1e6),
                    (xs::F_HIGH, 1e6),
                ] {
                    // each real now and then only: mixed constants too
                    if rng.below(4) != 0 {
                        t.g32(c, v.to_bits());
                    }
                }
                for (c, v) in [
                    (xs::INV_16, 0.0625f64),
                    (xs::INV_256, 1.0 / 256.0),
                    (xs::INV_128, 1.0 / 128.0),
                    (xs::EIGHT, 8.0),
                    (xs::THIRTY_TWO, 32.0),
                    (xs::INV_4096, 1.0 / 4096.0),
                    (xs::SEVEN_EIGHTHS, 0.875),
                    (xs::EIGHTH, 0.125),
                    (xs::QUARTER, 0.25),
                    (xs::TWO, 2.0),
                ] {
                    if rng.below(4) != 0 {
                        let at = t.rebase(c);
                        t.write(at, &v.to_le_bytes());
                    }
                }
            }
            if rng.below(6) == 0 {
                // texture coordinates whose 1 - v is not an f32
                let at = t.rebase(xs::INV_4096);
                t.write(at, &(1.0f64 / 3001.0).to_le_bytes());
            }
            let remap = real && rng.below(4) == 0;
            if remap {
                // the remap's f64 constants off their f32 twins (on its
                // level and record)
                for c in [xs::SEVEN_EIGHTHS, xs::EIGHTH, xs::QUARTER] {
                    let v = rng.double();
                    let at = t.rebase(c);
                    t.write(at, &v.to_le_bytes());
                }
            }
            // the transforms' results: w of 0 now and then, the fade's w
            // around its limits
            for out in [mul_out, clip_out] {
                for k in 0..16 {
                    let v = if k % 4 == 3 && rng.below(4) == 0 {
                        rng.pick(&[0, 0x8000_0000])
                    } else {
                        rng.float()
                    };
                    t.w32(out + 4 * k, v);
                }
            }
            for k in 0..4 {
                let v = rng.float();
                t.w32(apply_out + 4 * k, v);
            }
            let fade = rng.pick(&[3.0f32, 4.0, 4.5, 10.0, 20.0, 20.5, 100.0, f32::NAN]);
            if rng.below(4) != 0 {
                t.w32(apply_out + 12, fade.to_bits());
            }
            if rng.below(16) == 0 {
                // w * 8 - 32 = -0: below zero for none of the tests
                t.w32(apply_out + 12, 0x8000_0000);
                let at = t.rebase(xs::THIRTY_TWO);
                t.write(at, &0f64.to_le_bytes());
            }
            let level = if remap {
                0x672
            } else {
                rng.pick(&[0x672, 0x1bbd, 0x1234])
            };
            t.ret(xs::LEVEL_HASH, level);
            t.ret(xs::CURRENT_BUFFER, rng.below(3));
            t.ret(xs::ZCLIP, rng.pick(&[0, 0, 0, 1, 2]));
            t.g32(xs::SKIP_FRAME, u32::from(rng.below(16) == 0));
            t.g32(xs::YIQ_MODE, rng.pick(&[u32::MAX, 0, 2]));
            t.g32(xs::MAP_NO, rng.pick(&[0x188, 0x10]));
            t.g32(xs::OPTIMIZED_SHADER, 1);
            let fill = t.rebase(xs::FILL_MATRIX);
            for k in 0..16 {
                let v = rng.float();
                t.w32(fill + 4 * k, v);
            }
            // the object
            let obj = random_block(&mut t, &mut rng, 0x20);
            let m = random_block(&mut t, &mut rng, 0x40);
            t.w32(obj + 0x10, m);
            let other = random_block(&mut t, &mut rng, 0x40);
            t.w32(obj + 0x1c, rng.pick(&[0, m, other]));
            t.w16(obj + 2, rng.pick(&[0, 0x400, 0x4ff, 0xfbff]));
            if rng.below(6) == 0 {
                t.w8(obj + 7, 0);
            }
            // the record
            let wide = rng.below(2) == 0;
            let stride = if wide { 0x18 } else { 0x10 };
            let start = rng.pick(&[0u32, 0, 1, 3]);
            let count = rng.pick(&[0i16, 1, 2, 3, 4, -1]);
            let used = if count < 0 { 6 * start + 12 } else { 6 * start };
            let used = used + rng.pick(&[0, 0, 5]);
            let room = u32::from(count.max(0).cast_unsigned());
            let shape = t.zeros((0x20 + stride * room) as usize);
            t.w8(shape, rng.pick(&[0u8, 8, 0x88, 0x07]));
            t.w8(shape + 1, rng.below(256) as u8);
            t.w16(shape + 2, count.cast_unsigned());
            for k in 0..stride * room / 2 {
                let v = if k % (stride / 2) >= stride / 2 - 4 && real && rng.below(2) == 0 {
                    // uvs on the remap's values
                    rng.pick(&[512u32, 1536, 2560, 3584, 1024, 3072, 100])
                } else {
                    rng.small()
                };
                t.w16(shape + 0x20 + 2 * k, v as u16);
            }
            let colours = random_block(&mut t, &mut rng, 8 * room + 8);
            for k in 0..room {
                if rng.below(6) == 0 {
                    t.w8(colours + 8 * k + 3, 0);
                }
                if rng.below(3) == 0 {
                    // colours at most one half
                    for c in 0..4 {
                        let v = rng.below(0x40) as u8;
                        t.w8(colours + 8 * k + c, v);
                    }
                }
            }
            t.w32(shape + 4, colours.wrapping_sub(shape));
            let id = rng.next() & !0x3fff
                | if remap {
                    0x32c0
                } else {
                    rng.pick(&[0x32c0, 0x3980, 0x1111])
                };
            t.w32(shape + 0x10, id);
            // the render data and its buffers
            let rd = random_block(&mut t, &mut rng, 0xb8);
            let seg = random_block(&mut t, &mut rng, 0x80);
            let quads = start + room + 2;
            for (off, size) in [(0xc, 0x30), (0x14, 0x40), (0x18, 0x20), (0x1c, 12)] {
                let b = random_block(&mut t, &mut rng, size * quads);
                t.w32(seg + off, b);
            }
            t.w32(seg + 8, used);
            t.w32(rd + 0x94, seg);
            let missing = rng.below(12) == 0;
            t.ret(xs::RENDER_DATA, if missing { 0 } else { rd });
            let (lo, hi) = (rng.next(), rng.pick(&[0, 3]));
            let ebp = t.ebp_for(4);
            let frame = ebp - xs::SHAPE_FRAME;
            // the depth words the original may leave unwritten (junk read: 0)
            for off in [0x130u32, 0x12c, 0x128, 0x124] {
                t.w32(ebp - off, 0);
            }
            let r = t.check_framed(
                &format!("{w} wide {wide} start {start} count {count}"),
                0x0071_a4e0,
                &[lo, hi, shape, obj],
                frame,
                &[(0, 0xd0)],
                |g| u(xs::update_shape_field(g, frame, lo, hi, shape, obj)),
            );
            let cleared: u32 = Mem::read(&t.b, t.rebase(xs::OPTIMIZED_SHADER));
            seen[0] += u32::from(r == 0);
            seen[1] += u32::from(start == 0 && room > 0 && r == 1);
            seen[2] += u32::from(start > 0 && room > 0 && r == 1);
            seen[3] += u32::from(cleared == 0);
            seen[4] += u32::from(wide && room > 0);
            seen[5] += u32::from(!wide && room > 0);
        }
        eprintln!("{name}: {seen:?}");
        assert!(seen.iter().all(|&k| k > 10), "{name}: {seen:?}");
    }
}

// ---- M5 round 11 (ffx_glue_other::l11_shp)

/// `updateVFXRenderDataShp_OShapeVu1`: kind 3 with both layouts (counts
/// 0..3 and negative), drawn and clipped quads (the second matrix 0, the
/// first or another), tints and alphas, the segment buffer refused; other
/// kinds with and without a valid fill buffer (the request's fields, the
/// second matrix copied or not); missing render data; random and real
/// constants. Every branch is reached by synthetic data (the snapshots
/// hold no shape the test could pick).
#[test]
fn l11_update_shp_oshape_vu1() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_1b11);
        let (mul_out, clip_out) = (t.zeros(64), t.zeros(64));
        transform_stub(&mut t, l11s::MUL_MATRIX, 3, 2, 16, mul_out);
        transform_stub(&mut t, l11s::ZCLIP, 6, 0, 16, clip_out);
        let stubs = [
            (l11s::GEOMETRY, 2),
            (l11s::RENDER_DATA, 1),
            (l11s::FILL_VALID, 0),
            (l11s::FILL_REQUEST, 1),
            (l11s::SEGMENT_BUFFER, 4),
            (l11s::SET_VISIBLE, 3),
        ];
        for (sym, n) in stubs {
            t.stub_at(sym, false, n, 0, &[]);
        }
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.extend([l11s::MUL_MATRIX, l11s::ZCLIP]);
        let f32s = [l11s::ALPHA_SCALE, l11s::F_LOW, l11s::F_HIGH];
        let f64s = [
            l11s::FIFTEEN,
            l11s::MINUS_FIFTEEN,
            l11s::INV_256,
            l11s::INV_128,
            l11s::INV_4096,
        ];
        let mut seen = [0u32; 8];
        for i in 0..1200 {
            let w = format!("{name} {i}");
            t.fresh(&mut rng, &tags);
            t.constants(&mut rng, &f32s, &f64s);
            if rng.below(2) == 0 {
                for (c, v) in [
                    (l11s::ALPHA_SCALE, 128.0f32),
                    (l11s::F_LOW, -100_000.0),
                    (l11s::F_HIGH, 100_000.0),
                ] {
                    if rng.below(4) != 0 {
                        t.g32(c, v.to_bits());
                    }
                }
                for (c, v) in [
                    (l11s::FIFTEEN, 15.0f64),
                    (l11s::MINUS_FIFTEEN, -15.0),
                    (l11s::INV_256, 1.0 / 256.0),
                    (l11s::INV_128, 1.0 / 128.0),
                    (l11s::INV_4096, 1.0 / 4096.0),
                ] {
                    if rng.below(4) != 0 {
                        let at = t.rebase(c);
                        t.write(at, &v.to_le_bytes());
                    }
                }
            }
            if rng.below(6) == 0 {
                // texture coordinates whose 1 - v is not an f32
                let at = t.rebase(l11s::INV_4096);
                t.write(at, &(1.0f64 / 3001.0).to_le_bytes());
            }
            for out in [mul_out, clip_out] {
                for k in 0..16 {
                    let v = rng.float();
                    t.w32(out + 4 * k, v);
                }
            }
            t.ret(l11s::ZCLIP, rng.pick(&[0, 0, 0, 1, 2]));
            t.ret(l11s::FILL_VALID, rng.below(2));
            for sym in [l11s::FILL_MATRIX, l11s::FILL_MATRIX2, l11s::FILL_MATRIX3] {
                let at = t.rebase(sym);
                for k in 0..16 {
                    let v = rng.float();
                    t.w32(at + 4 * k, v);
                }
            }
            // the object
            let obj = random_block(&mut t, &mut rng, 0x20);
            let m = random_block(&mut t, &mut rng, 0x40);
            t.w32(obj + 0x10, m);
            let other = random_block(&mut t, &mut rng, 0x40);
            t.w32(obj + 0x1c, rng.pick(&[0, m, other]));
            // the record
            let wide = rng.below(2) == 0;
            let stride = if wide { 0x28 } else { 0x20 };
            let count = rng.pick(&[0i16, 1, 2, 3, 3, -1]);
            let room = u32::from(count.max(0).cast_unsigned());
            let shape = t.zeros((0x18 + stride * room + 8) as usize);
            let flags: u16 = rng.pick(&[0, 8, 0x88, 0x07, 0xff08, 0x8000]);
            let flags = if wide { flags | 8 } else { flags & !8 };
            t.w16(shape, flags);
            t.w16(shape + 2, count.cast_unsigned());
            let kind = rng.pick(&[3u8, 3, 3, 0, 1, 0x83]);
            t.w8(shape + 0xf, kind);
            for k in 0..stride * room / 2 {
                let v = rng.small();
                t.w16(shape + 0x18 + 2 * k, v as u16);
            }
            for q in 0..room {
                for c in 0..4 {
                    let v = if rng.below(3) == 0 {
                        rng.pick(&[0u8, 0xff, 0x80])
                    } else {
                        rng.below(256) as u8
                    };
                    t.w8(shape + 0x18 + stride * q + c, v);
                }
            }
            // the render data and its buffers
            let rd = random_block(&mut t, &mut rng, 0xc4);
            let seg = random_block(&mut t, &mut rng, 0x80);
            let quads = room + 2;
            for (off, size) in [(0xc, 0x30), (0x14, 0x40), (0x18, 0x20), (0x1c, 12)] {
                let b = random_block(&mut t, &mut rng, size * quads);
                t.w32(seg + off, b);
            }
            t.w32(rd + 0x94, seg);
            let req = random_block(&mut t, &mut rng, 0x190);
            t.ret(l11s::FILL_REQUEST, req);
            let missing = rng.below(12) == 0;
            t.ret(l11s::RENDER_DATA, if missing { 0 } else { rd });
            let refused = rng.below(10) == 0;
            t.ret(
                l11s::SEGMENT_BUFFER,
                if refused { 0 } else { rng.next() | 1 },
            );
            let (lo, hi) = (rng.next(), rng.pick(&[0, 3]));
            let ebp = t.ebp_for(4);
            let frame = ebp - 0x84;
            let valid = t.cell(l11s::FILL_VALID) != 0;
            let r = t.check_framed(
                &format!("{w} wide {wide} kind {kind} count {count}"),
                0x0071_c0b0,
                &[lo, hi, shape, obj],
                frame,
                &[(0, 0x80)],
                |g| u(l11s::update_shp_oshape_vu1(g, frame, lo, hi, shape, obj)),
            );
            let second: u32 = Mem::read(&t.b, obj + 0x1c);
            seen[0] += u32::from(missing);
            seen[1] += u32::from(r == 0 && !missing);
            seen[2] += u32::from(r == 1 && kind == 3 && wide && room > 0);
            seen[3] += u32::from(r == 1 && kind == 3 && !wide && room > 0);
            seen[4] += u32::from(r == 1 && kind == 3 && room > 0 && second != 0 && second != m);
            seen[5] += u32::from(!missing && kind != 3 && valid);
            seen[6] += u32::from(!missing && kind != 3 && !valid);
            seen[7] += u32::from(r == 1 && kind == 3 && room == 0);
        }
        eprintln!("{name}: {seen:?}");
        assert!(seen.iter().all(|&k| k > 10), "{name}: {seen:?}");
    }
}

/// `MsBtlPosWriteBin`: both table kinds, 0..4 records with 0..3 points in
/// each array (and a negative count), 0..3 sub-records with 0..3 points,
/// 0..3 extra words; into a buffer (the game's only call) and to a file
/// (`sceWrite` copies what it is handed into a log, so the records it sees
/// are compared), the file failing to open (with and without the parser's
/// file and line), the focus message, both arguments 0. Random and
/// non-signalling special floats.
#[test]
#[allow(clippy::similar_names)] // subs, sub records
fn l11_btl_pos_write_bin() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_ca58);
        // sceWrite: the bytes it is handed appended to a log, then recorded
        let sink = t.zeros(0x4_0000);
        t.w32(sink, sink + 4);
        let rest = t.stub_code(l11b::SCE_WRITE, false, 3, 0, false, &[]);
        let mut c = vec![0x56, 0x57, 0x8b, 0x3d]; // push esi; push edi; mov edi, [sink]
        c.extend(sink.to_le_bytes());
        c.extend([0x8b, 0x74, 0x24, 0x10]); // mov esi, [esp+0x10]
        c.extend([0x8b, 0x4c, 0x24, 0x14]); // mov ecx, [esp+0x14]
        c.extend([0xf3, 0xa4]); // rep movsb
        c.extend([0x89, 0x3d]);
        c.extend(sink.to_le_bytes()); // mov [sink], edi
        c.extend([0x5f, 0x5e]); // pop edi; pop esi
        jmp_to(&t, &mut c, rest);
        let f = t.code(&c);
        t.jump(l11b::SCE_WRITE, f);
        let stubs = [
            (l11b::SCE_OPEN, 2),
            (l11b::SCE_CLOSE, 1),
            (l11b::DBG_PRINTF, 2),
        ];
        for (sym, n) in stubs {
            t.stub_at(sym, false, n, 0, &[]);
        }
        stub_import(&mut t, l11b::MALLOC_IAT, 1);
        stub_import(&mut t, l11b::FREE_IAT, 1);
        let tags = [
            l11b::SCE_OPEN,
            l11b::SCE_CLOSE,
            l11b::DBG_PRINTF,
            l11b::SCE_WRITE,
            l11b::MALLOC_IAT,
            l11b::FREE_IAT,
        ];
        let file_name = string(&mut t, b"btl.bin");
        let mut seen = [0u32; 5];
        for i in 0..400 {
            t.fresh(&mut rng, &tags);
            t.w32(sink, sink + 4);
            let full = rng.below(3) != 0;
            let n = rng.pick(&[0i32, 1, 2, 3, 4, -1]);
            let room = n.max(0).cast_unsigned();
            let s = t.zeros(0x40);
            for k in 1..7 {
                let v = rng.float();
                t.w32(s + 4 * k, v);
            }
            t.w32(s, if full { 0 } else { rng.pick(&[1, 0x100, 0xff]) });
            t.w32(s + 0x1c, n.cast_unsigned());
            let records = t.zeros((0x74 * room.max(1)) as usize);
            t.w32(s + 0x30, records);
            let extras = rng.pick(&[0i32, 0, 1, 3, -2]);
            let words = random_block(&mut t, &mut rng, 16);
            t.w32(s + 0x24, extras.cast_unsigned());
            t.w32(s + 0x2c, words);
            for r in 0..room {
                let e = records + 0x74 * r;
                for k in 0..0x74 / 4 {
                    let v = rng.float();
                    t.w32(e + 4 * k, v);
                }
                for (cnt, arrays) in [
                    (0xc, [0x28, 0x2c]),
                    (0x30, [0x4c, 0x50]),
                    (0x54, [0x60, 0x64]),
                ] {
                    let m = rng.pick(&[0i32, 1, 2, 3, -1]);
                    t.w32(e + cnt, m.cast_unsigned());
                    for a in arrays {
                        let p = t.zeros(36);
                        for k in 0..9 {
                            let v = rng.float();
                            t.w32(p + 4 * k, v);
                        }
                        t.w32(e + a, p);
                    }
                }
                let subs = rng.pick(&[0i32, 1, 2, 3, -1]);
                t.w32(e + 0x68, subs.cast_unsigned());
                let list = t.zeros(0x14 * 3);
                for k in 0..subs.max(0).cast_unsigned() {
                    let sr = list + 0x14 * k;
                    let (x, y) = (rng.next(), rng.next());
                    t.w32(sr, x);
                    t.w32(sr + 4, y);
                    let pts = rng.pick(&[0i32, 1, 2, 3, -1]);
                    t.w32(sr + 8, pts.cast_unsigned());
                    let p = t.zeros(36);
                    for j in 0..9 {
                        let v = rng.float();
                        t.w32(p + 4 * j, v);
                    }
                    t.w32(sr + 0x10, p);
                }
                t.w32(e + 0x70, list);
            }
            t.g32(l11b::TABLE, s);
            let focus = rng.pick(&[0, 0, 0xffff_ffff, 1]);
            t.g32(l11b::FOCUS, focus);
            let file = rng.pick(&[0, file_name]);
            t.g32(l11b::FILE, file);
            let line = rng.pick(&[0, 7]);
            t.g32(l11b::LINE, line);
            let mode = rng.below(10);
            let (arg_name, out) = match mode {
                0..=5 => (0, t.zeros(0x2000)),
                6..=8 => (file_name, 0),
                _ => (0, 0),
            };
            if arg_name != 0 {
                let fd = rng.pick(&[3, 0, 0xffff_ffff, 5]);
                t.ret(l11b::SCE_OPEN, fd);
                let block = rng.pick(&[0, 0x1234_5678]);
                t.ret(l11b::MALLOC_IAT, block);
            }
            let ebp = t.ebp_for(2);
            let frame = ebp - l11b::FRAME;
            t.check_framed(
                &format!("{name} {i} full {full} n {n} mode {mode}"),
                0x007c_a580,
                &[arg_name, out],
                frame,
                &[(0, 0xa4)],
                |g| {
                    if arg_name == 0 && out == 0 {
                        Out::None
                    } else {
                        u(l11b::write_bin(g, frame, arg_name, out))
                    }
                },
            );
            seen[0] += u32::from(full && room > 0 && out != 0);
            seen[1] += u32::from(!full && room > 0 && out != 0);
            seen[2] += u32::from(arg_name != 0);
            seen[3] += u32::from(full && extras > 0 && out != 0);
            seen[4] += u32::from(arg_name == 0 && out == 0);
        }
        eprintln!("{name}: {seen:?}");
        assert!(seen.iter().all(|&k| k > 5), "{name}: {seen:?}");
    }
}

// ---- M5 round 11, batch 5 (ffx_menu::l11_refpos)

/// `Ms_Gui_ReferencePos`: every message; the step messages with each
/// button and the auto-repeat saying go or not, every slider mode; the
/// refresh with the object's lists of each kind (or none), the camera
/// picked (the pad in every direction, slow and fast, up and down, every
/// polar step) under camera modes 2, 4 and others, the actor picked, the
/// printers of every mode with the shorts at and past -1; the selector's
/// pick found or not; kinds 5 and others. Callees are recording stubs
/// (`MsVectorRect2Polar` writes its three outputs, the float callees
/// return random doubles); `_CIcos` / `_CIsin` are the CRT's.
#[test]
fn l11_reference_pos() {
    use ffx_menu::l11_refpos::bridge as rp;
    use ffx_menu::m5m_n::bridge as mn;
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_9c20);
        t.big_cave();
        // MsDebugGetCameraMode: arg 0 and 1 answered by two stubs
        let mode0 = t.stub_code(mn::MS_DEBUG_GET_CAMERA_MODE, false, 1, 0, false, &[]);
        let mode1 = t.stub_code(mn::MS_DEBUG_GET_CAMERA_MODE + 1, false, 1, 0, false, &[]);
        let mut c = vec![0x83, 0x7c, 0x24, 0x04, 0x00]; // cmp dword [esp+4], 0
        c.extend([0x0f, 0x84]); // je mode0
        let here = t.cave_next + c.len() as u32 + 4;
        c.extend(mode0.wrapping_sub(here).to_le_bytes());
        jmp_to(&t, &mut c, mode1);
        let f = t.code(&c);
        t.jump(mn::MS_DEBUG_GET_CAMERA_MODE, f);
        // MsVectorRect2Polar writes yaw, pitch and distance
        let polar = [t.zeros(4), t.zeros(4), t.zeros(4)];
        let writes = [wr(2, polar[0]), wr(3, polar[1]), wr(4, polar[2])];
        t.stub_at(mn::MS_VECTOR_RECT2_POLAR, false, 5, 0, &writes);
        let plain = [
            (rp::ATEL_CHANGE_CURRENT_CTRL, 1),
            (mn::MS_CAMERA_GET_ID, 1),
            (mn::MS_CAMERA_GET_NUM, 1),
            (mn::MS_DEBUG_CAMERA_KEEP, 1),
            (mn::ATEL_GET_NOW_CTRL_TOP_ID, 0),
            (rp::MS_SAVE_UC_ID, 1),
            (rp::ATEL_GET_TOTAL_ACTOR_NUMBER2, 0),
            (mn::CP_WINDOW_GET_ARG, 1),
            (mn::CP_USER_MALLOC, 1),
            (mn::CP_MEMSET, 3),
            (mn::CP_WINDOW_SET_ARG, 2),
            (mn::GU_SELECTOR_ALLOCATE, 4),
            (mn::GU_WINDOW_SET_TITLE, 2),
            (mn::GU_WINDOW_ADD_BUTTON, 7),
            (mn::MS_AUTO_REPEAT_INIT, 3),
            (mn::MS_SET_TABLE_ENTRY, 2),
            (mn::GU_SELECTOR_GET_ITEM_NAME, 1),
            (mn::MS_DEBUG_FIND_NAME, 2),
            (rp::REF_MODE_NEXT, 7),
            (mn::MS_AUTO_REPEAT_CHECK, 2),
            (rp::SLIDER_XYZ, 3),
            (rp::SLIDER_ACTORS, 4),
            (rp::SLIDER_REFERENCE, 4),
            (mn::MS_DEBUG_OUT_CAMERA, 0),
            (rp::REF_FROM_CAMERA, 1),
            (rp::REF_DRAW, 1),
            (rp::REF_ROUND, 0),
            (mn::GU_SELECTOR_RESET_NB_ITEM, 2),
            (mn::GU_SELECTOR_SET_ITEM, 4),
            (mn::GU_SELECTOR_OPEN, 3),
            (mn::MS_BATTLE_GET_STATUS, 0),
            (rp::ATEL_SET_CTRL_CHAR, 1),
            (mn::GU_WINDOW_SET_BUTTON_TEXT, 3),
            (mn::MS_DEBUG_SET_CAMERA_MODE, 2),
            (rp::SG_SET_PLAYER, 1),
            (mn::TK_GET_CONTROLER_HIST, 3),
            (rp::MS_VECTOR_POLAR2RECT, 5),
            (rp::PRINT_XYZ, 3),
            (rp::PRINT_ACTORS, 3),
            (rp::PRINT_BTL, 5),
            (mn::MS_DEBUG_SPLINE_PRINT, 4),
            (rp::PRINT_DASHES, 1),
            (mn::GU_PRINT_SET_X, 1),
            (mn::GU_PRINT_SET_Y, 1),
            (mn::GU_PRINTF, 2),
            (mn::MS_DEBUG_CAMERA_APPLY, 1),
        ];
        for (sym, n) in plain {
            t.stub_at(sym, false, n, 0, &[]);
        }
        for (sym, n) in [
            (rp::REF_ADD, 1),
            (rp::REF_SET, 7),
            (rp::MS_CAMERA_GET_YAW, 0),
        ] {
            t.stub(sym, n, true);
        }
        // sprintf: the words each format takes, by format
        let wide = t.stub_code(mn::SPRINTF + 1, false, 4, 0, false, &[]);
        let flst = t.stub_code(mn::SPRINTF + 2, false, 9, 0, false, &[]);
        let short = t.stub_code(mn::SPRINTF, false, 2, 0, false, &[]);
        let mut c = Vec::new();
        for (fmt, to) in [(rp::S_ACTOR, wide), (rp::S_FLOAT, wide), (rp::S_FLST, flst)] {
            c.extend([0x81, 0x7c, 0x24, 0x08]); // cmp dword [esp+8], fmt
            c.extend(t.rebase(fmt).to_le_bytes());
            c.extend([0x0f, 0x84]); // je to
            let here = t.cave_next + c.len() as u32 + 4;
            c.extend(to.wrapping_sub(here).to_le_bytes());
        }
        jmp_to(&t, &mut c, short);
        let f = t.code(&c);
        t.g32(mn::SPRINTF, f);
        let mut tags: Vec<u32> = plain.iter().map(|p| p.0).collect();
        tags.extend([
            mn::MS_DEBUG_GET_CAMERA_MODE,
            mn::MS_DEBUG_GET_CAMERA_MODE + 1,
            mn::MS_VECTOR_RECT2_POLAR,
            rp::REF_ADD,
            rp::REF_SET,
            rp::MS_CAMERA_GET_YAW,
            mn::SPRINTF,
            mn::SPRINTF + 1,
            mn::SPRINTF + 2,
        ]);
        let f64s = [
            rp::D_MINUS_MILLI,
            rp::D_MILLI,
            rp::D_QUARTER_PI,
            rp::D_QUARTER,
            rp::D_YAW_STEP,
            rp::D_PITCH_STEP,
            rp::D_TWO,
        ];
        let f32s = [
            rp::F_UP_RIGHT,
            rp::F_UP_LEFT,
            rp::F_DOWN_RIGHT,
            rp::F_DOWN_LEFT,
            rp::F_DOWN,
            rp::F_RIGHT,
            rp::F_LEFT,
        ];
        // the camera blocks: random floats
        let cams = t.rebase(rp::CAMERAS);
        for k in 0..3 * 0xda0 / 4 {
            let v = rng.float();
            t.w32(cams + 4 * k, v);
        }
        let mut seen = [0u32; 6];
        for i in 0..1500 {
            t.fresh(&mut rng, &tags);
            if rng.below(3) == 0 {
                t.constants(&mut rng, &f32s, &f64s);
            }
            let screen = rng.below(2);
            t.g32(rp::DEBUG_CAMERA_INDEX, screen);
            let cam = rng.below(3);
            let at = t.rebase(rp::DEBUG_CAMERA) + 0x1c0 * screen;
            t.w8(at, cam as u8);
            let kind = rng.pick(&[5u32, 5, 0, 3, 0xf, 0xd]);
            t.g32(rp::OPEN, 1);
            let id = rng.next() & 0xfff0_ffff | kind << 16;
            t.ret(mn::MS_CAMERA_GET_ID, id);
            let num = rng.below(4);
            t.ret(mn::MS_CAMERA_GET_NUM, num);
            t.ret(mn::MS_DEBUG_CAMERA_KEEP, rng.below(3));
            let mode = rng.pick(&[1u32, 1, 3, 3, 0xb, 0xc, 0xd, 0xe, 2, 7, 0x20]);
            t.ret(mn::MS_DEBUG_GET_CAMERA_MODE, mode);
            let mode1 = rng.pick(&[2u32, 4, 0, 1]);
            t.ret(mn::MS_DEBUG_GET_CAMERA_MODE + 1, mode1);
            let actor = rng.pick(&[-3i32, -2, -1, -1, 0, 1, 5, 40]);
            t.g32(rp::ACTOR, actor.cast_unsigned());
            t.ret(
                mn::ATEL_GET_NOW_CTRL_TOP_ID,
                rng.pick(&[0, 3, 0x40, u32::MAX]),
            );
            t.ret(
                rp::ATEL_GET_TOTAL_ACTOR_NUMBER2,
                rng.pick(&[0, 1, 4, 0xffff_fffe]),
            );
            let arg = random_block(&mut t, &mut rng, 0x94);
            t.ret(mn::CP_WINDOW_GET_ARG, arg);
            let block = t.zeros(0x94);
            t.ret(mn::CP_USER_MALLOC, block);
            t.ret(mn::MS_AUTO_REPEAT_CHECK, u32::from(rng.below(5) != 0));
            t.ret(mn::MS_BATTLE_GET_STATUS, rng.below(2));
            t.ret(
                mn::MS_DEBUG_FIND_NAME,
                rng.pick(&[1, 3, 0xb, 0xc, 0xd, 2, 9]),
            );
            let pad = match rng.below(4) {
                0 => rng.next(),
                1 => 0,
                _ => {
                    rng.pick(&[
                        0x1000u32, 0x9000, 0x3000, 0x4000, 0xc000, 0x6000, 0x8000, 0x2000,
                    ]) | rng.below(0x100)
                }
            };
            t.ret(mn::TK_GET_CONTROLER_HIST, pad);
            for p in polar {
                let v = rng.float();
                t.w32(p, v);
            }
            for sym in [rp::REF_ADD, rp::REF_SET, rp::MS_CAMERA_GET_YAW] {
                let v = rng.double();
                t.ret_f64(sym, v);
            }
            // the reference object's flags, shorts and lists
            let obj = cams + 0xda0 * cam + 0xc0 + 0x130 * num;
            t.w8(obj + 4, rng.pick(&[0u8, 1, 0x80]));
            for k in 0..4 {
                let v = rng.pick(&[0xffffu16, 0xfffe, 0, 5, 0x8000]);
                t.w16(obj + 0xc + 2 * k, v);
            }
            for k in 0..5 {
                let ty = rng.pick(&[0u32, 0, 1, 2, 3, 4]);
                t.w32(obj + 0x90 + 0x20 * k, ty);
            }
            let (msg, param) = match rng.below(12) {
                0..=3 => (
                    rng.pick(&[0x8005, 0x8009, 0x800d]),
                    rng.pick(&[2u32, 5, 6, 8, 9, 0xa, 0xd, 0xe, 0x33, 0x36, 0x27, 7]),
                ),
                4..=7 => (0x8011, rng.next()),
                8 => (0x8012, rng.pick(&[0x36, 0x36, 2])),
                _ => (
                    rng.pick(&[
                        0x8001, 0x8003, 0x8006, 0x800a, 0x800e, 0x8002, 0x8013, 0x8000,
                    ]),
                    rng.next(),
                ),
            };
            let window = rng.next();
            let ebp = t.ebp_for(3);
            let frame = ebp - rp::FRAME;
            t.check_framed(
                &format!("{name} {i} msg {msg:#x} param {param:#x} mode {mode} mode1 {mode1}"),
                0x007d_9c20,
                &[window, msg, param],
                frame,
                &[(0, 4), (8, 4), (0x10, 4), (0x20, 0x100)],
                |g| u(rp::reference_pos(g, ebp, window, msg, param)),
            );
            let now: i32 = Mem::read(&t.b, t.rebase(rp::ACTOR));
            seen[0] += u32::from(msg == 0x8011 && now < 0);
            seen[1] += u32::from(msg == 0x8011 && now >= 0);
            seen[2] += u32::from(msg & 0xfff3 == 0x8001 && msg != 0x8001);
            seen[3] += u32::from(msg == 0x8012);
            seen[4] += u32::from(msg == 0x8001);
            seen[5] += u32::from(msg == 0x8011 && now < 0 && pad & 0x50 != 0);
        }
        eprintln!("{name}: {seen:?}");
        assert!(seen.iter().all(|&k| k > 10), "{name}: {seen:?}");
    }
}

// ---- M5 round 11, batch 6 (ffx_menu::l11_btlproc)

/// `Ms_Gui_BattleProc`: every message; the step messages with each button
/// (the auto-repeat saying go or not), in battle and out, every kind
/// (0, 3, 4, 5, 0xb and others), the field index and distance at their
/// bounds, the file found or not and loaded or not; the refresh with the
/// pad in every direction (slow and fast), every pad edge calling the
/// window again, the group record's count and the kind's count at and
/// past 0. `MsBtlPosAccess` writes the position when it reads (`dir` <=
/// 0); `MsDebugIsTextMode` stays the original (its callers keep `ecx` and
/// `dl` across it); `_CIcos` / `_CIsin` are the CRT's.
#[test]
fn l11_battle_proc() {
    use ffx_menu::l11_btlproc::bridge as bp;
    use ffx_menu::m5m_n::bridge as mn;
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_80a0);
        t.big_cave();
        // MsBtlPosAccess: copies three words from `src` to its argument 4
        // when it reads, and appends the three words there to a log when it
        // writes (what it would store into the table)
        let src = t.zeros(12);
        let written = t.zeros(0x1_0000);
        t.w32(written, written + 4);
        let rest = t.stub_code(bp::POS_ACCESS, false, 5, 0, false, &[]);
        let mut c = vec![0x83, 0x7c, 0x24, 0x04, 0x00]; // cmp dword [esp+4], 0
        c.extend([0x7f, 0x24]); // jg the write path (31 + 5 bytes on)
        c.extend([0x8b, 0x44, 0x24, 0x14]); // mov eax, [esp+0x14]
        for k in 0..3u8 {
            c.push(0x8b);
            c.push(0x0d);
            c.extend((src + 4 * u32::from(k)).to_le_bytes()); // mov ecx, [src+4k]
            c.extend([0x89, 0x48, 4 * k]); // mov [eax+4k], ecx
        }
        jmp_to(&t, &mut c, rest);
        c.extend([0x56, 0x57, 0x8b, 0x74, 0x24, 0x1c]); // push esi; push edi; mov esi, [esp+0x1c]
        c.extend([0x8b, 0x3d]);
        c.extend(written.to_le_bytes()); // mov edi, [written]
        c.extend([0xa5, 0xa5, 0xa5]); // movsd x3
        c.extend([0x89, 0x3d]);
        c.extend(written.to_le_bytes()); // mov [written], edi
        c.extend([0x5f, 0x5e]); // pop edi; pop esi
        jmp_to(&t, &mut c, rest);
        let f = t.code(&c);
        t.jump(bp::POS_ACCESS, f);
        let plain = [
            (mn::MS_AUTO_REPEAT_INIT, 3),
            (mn::GU_WINDOW_ADD_BUTTON, 7),
            (mn::MS_AUTO_REPEAT_CHECK, 2),
            (bp::POS_GROUP, 0),
            (bp::NEXT_KIND, 1),
            (bp::SET_DIR_AT, 4),
            (bp::POS_DISTANCES, 2),
            (bp::WRITE_ALL, 1),
            (bp::COPY_POINTS, 3),
            (bp::PLACE_ALL, 0),
            (bp::WINDOWS_ALL, 1),
            (bp::EDIT_FREE, 0),
            (bp::WRITE_SCR, 1),
            (bp::POS_LOAD, 1),
            (bp::BATTLE_POS_INIT, 3),
            (bp::CHECK_FILE, 1),
            (bp::STR_COPY, 3),
            (bp::PATH_SET_EXT, 4),
            (bp::SCRIPT_COMPILE, 2),
            (bp::GET_BATTLE_NAME, 2),
            (bp::DBG_PRINTF, 2),
            (mn::TK_GET_CONTROLER_HIST, 3),
            (mn::GU_WINDOW_SET_BUTTON_TEXT, 3),
            (mn::GU_WINDOW_SET_TITLE, 2),
            (mn::GU_PRINT_SET_X, 1),
            (mn::GU_PRINT_SET_Y, 1),
            (mn::GU_PRINTF, 1),
        ];
        for (sym, n) in plain {
            t.stub_at(sym, false, n, 0, &[]);
        }
        for (sym, n) in [(bp::DIR_AT, 3), (bp::CAMERA_GET_YAW, 0)] {
            t.stub(sym, n, true);
        }
        // sprintf: the words each format takes
        let three = t.stub_code(mn::SPRINTF + 1, false, 3, 0, false, &[]);
        let four = t.stub_code(mn::SPRINTF + 2, false, 4, 0, false, &[]);
        let two = t.stub_code(mn::SPRINTF, false, 2, 0, false, &[]);
        let mut c = Vec::new();
        for (fmt, to) in [
            (bp::S_TITLE, three),
            (bp::S_3D, three),
            (bp::S_PAIR, four),
            (bp::S_FLOAT, four),
        ] {
            c.extend([0x81, 0x7c, 0x24, 0x08]); // cmp dword [esp+8], fmt
            c.extend(t.rebase(fmt).to_le_bytes());
            c.extend([0x0f, 0x84]); // je to
            let here = t.cave_next + c.len() as u32 + 4;
            c.extend(to.wrapping_sub(here).to_le_bytes());
        }
        jmp_to(&t, &mut c, two);
        let f = t.code(&c);
        t.g32(mn::SPRINTF, f);
        let mut tags: Vec<u32> = plain.iter().map(|p| p.0).collect();
        tags.extend([
            bp::POS_ACCESS,
            bp::DIR_AT,
            bp::CAMERA_GET_YAW,
            mn::SPRINTF,
            mn::SPRINTF + 1,
            mn::SPRINTF + 2,
        ]);
        let f32s = [bp::F_FINE, bp::F_COARSE];
        let f64s = [bp::D_QUARTER_PI, bp::D_QUARTER];
        let mut seen = [0u32; 6];
        for i in 0..1500 {
            t.fresh(&mut rng, &tags);
            if rng.below(3) == 0 {
                t.constants(&mut rng, &f32s, &f64s);
                for &d in &bp::F_DIRECTIONS {
                    let v = rng.float();
                    t.g32(d, v);
                }
            }
            for k in 0..3 {
                let v = rng.float();
                t.w32(src + 4 * k, v);
            }
            let kind = rng.pick(&[0u32, 3, 4, 5, 0xb, 1, 2, 6, 0xa, 0xb, 3]);
            t.g32(bp::KIND, kind);
            for k in 0..12 {
                let v = rng.pick(&[0u32, 1, 2, 5, u32::MAX]);
                t.g32(bp::PICKS + 4 * k, v);
            }
            let group = rng.pick(&[0u8, 1, 3, 0xff, 0x7f, 0x80]);
            t.w8(t.rebase(bp::GROUP), group);
            let dist = rng.pick(&[0u8, 1, 2, 4, 0xff, 0x7f]);
            t.w8(t.rebase(bp::DIST), dist);
            let status = rng.pick(&[0u8, 1, 1, 0x80, 2]);
            t.w8(t.rebase(bp::BTL_STATUS), status);
            let field = rng.pick(&[0u16, 1, 1, 0xffff]);
            t.w16(t.rebase(bp::FIELD_INDEX), field);
            let points = t.zeros(8);
            t.w8(points + 1, rng.pick(&[0u8, 1, 3, 0xff]));
            t.g32(bp::FIELD_POINTS, points);
            let count = rng.pick(&[0u32, 1, 3, u32::MAX]);
            t.g32(bp::COUNT, count);
            for sym in [bp::POS, bp::POS + 4, bp::POS + 8, bp::DIR] {
                let v = rng.float();
                t.g32(sym, v);
            }
            for sym in [
                bp::BLINK,
                bp::PAD_BEFORE,
                bp::BUFFER,
                bp::SELECTED,
                bp::OPEN,
            ] {
                let v = rng.next();
                t.g32(sym, v);
            }
            for state in [bp::REPEAT1, bp::REPEAT10, bp::REPEAT100] {
                let v = rng.next();
                t.g32(state, v);
            }
            for k in 0..0x60 {
                let w = if rng.below(8) == 0 {
                    random_block(&mut t, &mut rng, 0x24)
                } else {
                    0
                };
                t.g32(bp::WINDOWS + 4 * k, w);
            }
            let grp = t.zeros(8);
            t.w8(grp + 1, rng.pick(&[0u8, 1, 4, 0x80]));
            t.ret(bp::POS_GROUP, grp);
            t.ret(mn::MS_AUTO_REPEAT_CHECK, u32::from(rng.below(5) != 0));
            t.ret(bp::CHECK_FILE, rng.pick(&[0, 1, u32::MAX]));
            t.ret(bp::POS_LOAD, rng.below(2));
            let pad = match rng.below(4) {
                0 => rng.next(),
                1 => rng.below(0x100),
                _ => {
                    rng.pick(&[
                        0x1000u32, 0x9000, 0x3000, 0x4000, 0xc000, 0x6000, 0x8000, 0x2000,
                    ]) | rng.below(0x100)
                }
            };
            t.ret(mn::TK_GET_CONTROLER_HIST, pad);
            for sym in [bp::DIR_AT, bp::CAMERA_GET_YAW] {
                let v = rng.double();
                t.ret_f64(sym, v);
            }
            let (msg, param) = match rng.below(12) {
                0..=4 => (
                    rng.pick(&[0x8005, 0x8009, 0x800d]),
                    rng.pick(&[
                        0x15u32, 0x16, 0x1e, 0x1f, 0x3d, 0x3e, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45,
                        0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x52, 0x8d, 0x17, 0x100,
                    ]),
                ),
                5..=8 => (0x8011, rng.next()),
                _ => (
                    rng.pick(&[
                        0x8001, 0x8003, 0x8006, 0x800a, 0x800e, 0x8002, 0x8012, 0x8000,
                    ]),
                    rng.next(),
                ),
            };
            let window = rng.next();
            let ebp = t.ebp_for(3);
            let frame = ebp - bp::FRAME;
            t.check_framed(
                &format!("{name} {i} msg {msg:#x} param {param:#x} kind {kind}"),
                0x007d_80a0,
                &[window, msg, param],
                frame,
                &[(0, 0x300)],
                |g| u(bp::battle_proc(g, ebp, window, msg, param)),
            );
            seen[0] += u32::from(msg == 0x8011 && pad & 0xf000 != 0);
            seen[1] += u32::from(msg == 0x8011 && status != 0);
            seen[2] += u32::from(msg & 0xfff3 == 0x8001 && msg != 0x8001 && status != 0);
            seen[3] += u32::from(msg == 0x8003);
            seen[4] += u32::from(msg == 0x8001);
            seen[5] += u32::from(msg == 0x8011 && pad & 0xf != 0);
        }
        eprintln!("{name}: {seen:?}");
        assert!(seen.iter().all(|&k| k > 10), "{name}: {seen:?}");
    }
}
