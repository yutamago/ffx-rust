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
use ffx_glue_other::{h4b_a as ba, h4b_b as bb};
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
