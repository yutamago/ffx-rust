//! `ffx_prender::m5m_*` (M5 stage 1, list `map`) against the original:
//! on the image alone with synthetic objects, and on the snapshots
//! (`FFX_SNAPSHOT`), where the game's own globals and heap are there too.
//! Results and every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! None of these functions ran in a coverage run, so every branch is
//! reached with synthetic data; the snapshots add the real image state
//! (globals, the CRT's SSE2 flag) under the same synthetic objects.
//! Callees that stay original are replaced in both oracles alike by
//! recording stubs: each call appends its tag, `ecx` (for methods) and its
//! arguments to a log in memory, so the diff checks that both made the
//! same calls in the same order. A stub pops what the original pops. The
//! MSVC `std::string` helpers, `memset` and the CRT's `sprintf_s`/`atoi`
//! run for real in both (the oracle's runtime), so the frame buffers the
//! ports build are compared through what those callees read and write.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::items_after_statements,
    dead_code // the harness is shared by every batch; not each uses every helper
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_prender::m5m_a::{self as ma, bridge as a};
use ffx_prender::m5m_b::{self as mb, bridge as b};

/// The body of `FUN_006cbf20` (unreachable: `tools/reach.py --why`): the
/// recording stubs are written over it.
const CAVE: u32 = 0x006c_bf20;
const CAVE_LEN: u32 = 1800;
/// After the stubs: `mov eax, POISON; ret`, called before each original
/// callee so a callee leaving `eax` alone shows the poison.
const POISON_AT: u32 = CAVE + CAVE_LEN;
const POISON: u32 = 0xdead_beef;

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
    fn chance(&mut self, n: u32) -> bool {
        self.below(n) == 0
    }
    /// A word: small values, 0, 1, -1, the top bit and anything.
    fn word(&mut self) -> u32 {
        match self.below(6) {
            0 => 0,
            1 => 1,
            2 => self.below(8),
            3 => 0xffff_ffff,
            4 => 0x8000_0000 | self.below(4),
            _ => self.next(),
        }
    }
}

/// Where a stub's `eax` comes from.
#[derive(Clone, Copy)]
enum Ret {
    /// Left as it was.
    Keep,
    /// A constant.
    Val(u32),
    /// The next word of a table: the cell holds the table cursor.
    Seq(u32),
}

/// The cells the unwinding stubs read: the `esp` to unwind to and the
/// oracle's return sentinel, at the bottom of the stack (not compared).
fn cells(o: &Oracle) -> (u32, u32) {
    let s = o.stack_range().start;
    (s + 0x10, s + 0x14)
}

/// The oracle's default call top.
fn call_top(o: &Oracle) -> u32 {
    o.stack_range().end - 0x100
}

/// Sets the unwind target for a call of `conv` with `args` from the call
/// top `top`.
fn arm_unwind(o: &mut Oracle, top: u32, conv: Conv, args: &[u32]) {
    let n = match conv {
        Conv::Fastcall => args.len().saturating_sub(2),
        _ => args.len(),
    } as u32;
    let esp = match conv {
        Conv::Cdecl => top - 4 * n,
        _ => top,
    };
    let (cell, _) = cells(o);
    o.write(cell, &esp.to_le_bytes()).unwrap();
}

/// The bridge's view of an oracle: its memory, and its emulator for the
/// original callees.
struct G<'a>(&'a mut Oracle, u32, u32);

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

impl Host for G<'_> {
    fn call(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> u32 {
        ffx_oracle::check_width("Host::call", addr, args.len(), host::max_words(c, false));
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        arm_unwind(self.0, self.2, conv(c), args);
        self.0
            .call(addr, conv(c), args)
            .unwrap_or_else(|e| panic!("{addr:#x}: {e}"))
            .eax
    }
    fn call_st0_f64(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> f64 {
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        let r = self.0.call(addr, conv(c), args).unwrap();
        r.st0.unwrap_or_else(|| panic!("{addr:#x}: no ST0"))
    }
    fn call_wide(&mut self, addr: u32, args: &[u32]) -> u32 {
        ffx_oracle::check_width("Host::call_wide", addr, args.len(), host::WIDE_WORDS);
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        self.0.call(addr, Conv::Cdecl, args).unwrap().eax
    }
    fn scratch(&mut self) -> u32 {
        self.0.stack_range().start + 0x100
    }
}

/// What a function returns, to compare.
#[derive(Debug, Clone, Copy)]
enum Out {
    /// Nothing the callers read.
    None,
    /// `eax`.
    U32(u32),
    /// `al`.
    U8(u8),
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 16 words (tag, `this`, up
    /// to 14 arguments).
    log: u32,
    cave: u32,
    cave_next: u32,
    snapshot: bool,
    /// Bytes the stubs replaced, put back by [`Twins::reset`].
    patched: Vec<(u32, Vec<u8>)>,
    /// A call top for the port's callees below the default one (to keep
    /// the words above it, such as an argument slot the port writes).
    b_top: Option<u32>,
}

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle, snapshot: bool) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        a.set_max_instructions(2_000_000);
        b.set_max_instructions(2_000_000);
        let cave = a.rebase(CAVE);
        let mut t = Self {
            a,
            b,
            log: 0,
            cave,
            cave_next: cave,
            snapshot,
            patched: Vec::new(),
            b_top: None,
        };
        t.log = t.zeros(0x1_0000);
        let at = t.rebase(POISON_AT);
        let mut code = vec![0xb8];
        code.extend(POISON.to_le_bytes());
        code.push(0xc3);
        t.write(at, &code);
        // the return sentinel: what a call leaves at its top - 4
        for o in [&mut t.a, &mut t.b] {
            o.call(at, Conv::Cdecl, &[]).unwrap();
            let top = o.stack_range().end - 0x100;
            let sentinel: u32 = Mem::read(o, top - 4);
            let (_, cell) = cells(o);
            o.write(cell, &sentinel.to_le_bytes()).unwrap();
        }
        t
    }

    fn image() -> Option<Self> {
        let exe = game_dir()?.join("FFX.exe");
        Some(Self::new(
            Oracle::load(&exe).unwrap(),
            Oracle::load(&exe).unwrap(),
            false,
        ))
    }

    fn snapshot() -> Option<Self> {
        let dir = game_dir()?;
        let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
            eprintln!("FFX_SNAPSHOT is not set; skipping");
            return None;
        };
        let exe = dir.join("FFX.exe");
        let pages = Pages::read(PathBuf::from(snap)).unwrap();
        Some(Self::new(
            Oracle::from_snapshot(&exe, &pages).unwrap(),
            Oracle::from_snapshot(&exe, &pages).unwrap(),
            true,
        ))
    }

    /// Both kinds: the image, then the snapshot when one is set.
    fn each(mut f: impl FnMut(&mut Self)) {
        if let Some(mut t) = Self::image() {
            f(&mut t);
        }
        if let Some(mut t) = Self::snapshot() {
            f(&mut t);
        }
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

    fn words(&mut self, w: &[u32]) -> u32 {
        let bytes: Vec<u8> = w.iter().flat_map(|v| v.to_le_bytes()).collect();
        self.alloc(&bytes)
    }

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    fn w8(&mut self, at: u32, v: u8) {
        self.write(at, &[v]);
    }

    fn r32(&self, at: u32) -> u32 {
        Mem::read(&self.b, at)
    }

    /// Writes `bytes` at `at`, to be put back by [`Twins::reset`].
    fn patch(&mut self, at: u32, bytes: &[u8]) {
        let mut old = vec![0; bytes.len()];
        Mem::read_bytes(&self.b, at, &mut old);
        self.patched.push((at, old));
        self.write(at, bytes);
    }

    /// Sets the global at `sym` until [`Twins::reset`].
    fn global(&mut self, sym: u32, v: u32) {
        let at = self.rebase(sym);
        self.patch(at, &v.to_le_bytes());
    }

    fn reset(&mut self) {
        for (at, old) in std::mem::take(&mut self.patched).into_iter().rev() {
            self.write(at, &old);
        }
        self.cave_next = self.cave;
        let log = self.log;
        self.w32(log, 0);
    }

    /// A table of return values for [`Ret::Seq`]: the cursor cell.
    fn seq(&mut self, values: &[u32]) -> Ret {
        let table = self.words(values);
        Ret::Seq(self.words(&[table]))
    }

    /// Writes a recording stub into the cave: logs `tag`, `ecx` when
    /// `this`, and `n` stack arguments; returns `ret` and pops `pops`
    /// words, or with `unwind` returns to the test's call (a throw).
    fn stub_code(
        &mut self,
        tag: u32,
        this: bool,
        n: u32,
        pops: u32,
        ret: Ret,
        unwind: bool,
    ) -> u32 {
        assert!(n <= 14);
        let log = self.log;
        let keep = u8::from(matches!(ret, Ret::Keep)) * 4;
        let mut c = vec![];
        if keep != 0 {
            c.push(0x50); // push eax
        }
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
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8 + keep]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        match ret {
            Ret::Val(v) => {
                c.push(0xb8);
                c.extend(v.to_le_bytes()); // mov eax, v
            }
            Ret::Keep => c.push(0x58), // pop eax
            Ret::Seq(cell) => {
                c.push(0xa1);
                c.extend(cell.to_le_bytes()); // mov eax, [cell]
                c.extend([0x8b, 0x00]); // mov eax, [eax]
                c.extend([0x83, 0x05]);
                c.extend(cell.to_le_bytes());
                c.push(0x04); // add dword [cell], 4
            }
        }
        if unwind {
            let (esp, sentinel) = cells(&self.a);
            for (op, v) in [
                (0xbb, 0x1111_1111_u32),
                (0xbe, 0x2222_2222),
                (0xbf, 0x3333_3333),
                (0xbd, 0x4444_4444),
            ] {
                c.push(op);
                c.extend(v.to_le_bytes()); // the saved registers' check values
            }
            c.extend([0x8b, 0x25]);
            c.extend(esp.to_le_bytes()); // mov esp, [cell]
            c.extend([0xff, 0x25]);
            c.extend(sentinel.to_le_bytes()); // jmp [sentinel]
        } else if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        let addr = self.cave_next;
        if addr + c.len() as u32 > self.cave + CAVE_LEN {
            let x = self.a.alloc_code(&c).unwrap();
            assert_eq!(self.b.alloc_code(&c).unwrap(), x);
            return x;
        }
        self.write(addr, &c);
        self.cave_next = addr + c.len() as u32;
        addr
    }

    /// A stub over the original at `sym` (a `jmp` to it).
    fn stub_sym(&mut self, sym: u32, tag: u32, this: bool, n: u32, pops: u32, ret: Ret) -> u32 {
        let s = self.stub_code(tag, this, n, pops, ret, false);
        self.jump(sym, s);
        s
    }

    /// An unwinding stub over the thrower at `sym`.
    fn throw_sym(&mut self, sym: u32, tag: u32, n: u32) -> u32 {
        let s = self.stub_code(tag, false, n, 0, Ret::Val(0), true);
        self.jump(sym, s);
        s
    }

    /// An unwinding stub for the throwing import whose slot is `sym`.
    fn throw_import(&mut self, sym: u32, tag: u32, n: u32) {
        let s = self.stub_code(tag, false, n, 0, Ret::Val(0), true);
        let at = self.rebase(sym);
        self.patch(at, &s.to_le_bytes());
    }

    fn jump(&mut self, sym: u32, to: u32) {
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(to.wrapping_sub(at + 5).to_le_bytes());
        self.patch(at, &j);
    }

    /// A stub for the import whose slot is `sym`, popping `pops` words.
    fn import_sym(&mut self, sym: u32, tag: u32, n: u32, pops: u32, ret: Ret) {
        let s = self.stub_code(tag, false, n, pops, ret, false);
        let at = self.rebase(sym);
        self.patch(at, &s.to_le_bytes());
    }

    /// An object whose vtable holds `slots` (stub addresses).
    fn object(&mut self, slots: &[u32], size: usize) -> u32 {
        let vt = self.words(slots);
        let obj = self.zeros(size.max(4));
        self.w32(obj, vt);
        obj
    }

    fn diff(&mut self, what: &str) {
        let d = self.a.diff(&mut self.b).unwrap();
        if !d.is_empty() && std::env::var_os("M5_LOG").is_some() {
            for (side, o) in [("a", &self.a), ("b", &self.b)] {
                let n: u32 = Mem::read(o, self.log);
                for k in 0..n.min(64) {
                    let e: Vec<u32> = (0..16)
                        .map(|w| Mem::read::<u32>(o, self.log + 4 + 64 * k + 4 * w))
                        .collect();
                    eprintln!("{side} {k}: {e:x?}");
                }
            }
        }
        assert_eq!(d, [], "{what}: memory");
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// Runs the original at `sym` in `a` and `bridge` on `b`; compares the
    /// result as `bridge` says and every byte written.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        cv: Conv,
        args: &[u32],
        bridge: impl FnOnce(&mut G) -> Out,
    ) {
        let addr = self.rebase(sym);
        let top = call_top(&self.a);
        arm_unwind(&mut self.a, top, cv, args);
        let ra = self
            .a
            .call(addr, cv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let poison = self.rebase(POISON_AT);
        let top = self.b_top.unwrap_or_else(|| call_top(&self.b));
        self.b.set_call_top(self.b_top);
        let out = bridge(&mut G(&mut self.b, poison, top));
        self.b.set_call_top(None);
        assert!(
            ra.st0_raw.is_none(),
            "{what}: the original leaves ST0 set ({:?})",
            ra.st0
        );
        match out {
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::U8(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
            Out::None => {}
        }
        self.diff(what);
    }

    /// The call log's entry count in `b`.
    fn calls(&self) -> u32 {
        self.r32(self.log)
    }
}

/// The argument slot of a cdecl call's first word from the oracle's call
/// top (what the original sees at `[ebp + 8]`).
fn first_slot(o: &Oracle, words: u32) -> u32 {
    o.stack_range().end - 0x100 - 4 * words
}

/// `operator_new` (cdecl).
const OPERATOR_NEW: u32 = 0x0063_0310;

/// The `ebp` of a call with `words` stack words from the oracle's call
/// top (the return address and the saved `ebp` below them).
fn frame_ebp(o: &Oracle, words: u32) -> u32 {
    call_top(o) - 4 * words - 8
}

/// A float word: anything but a signalling NaN.
fn float_word(rng: &mut Rng) -> u32 {
    loop {
        let x = match rng.below(4) {
            0 => 0x7fc0_0000 | rng.below(16),
            1 => (rng.below(2000) as f32 / 8.0 - 100.0).to_bits(),
            _ => rng.next(),
        };
        let snan = x & 0x7f80_0000 == 0x7f80_0000 && x & 0x007f_ffff != 0 && x & 0x0040_0000 == 0;
        if !snan {
            return x;
        }
    }
}

impl Twins {
    /// A recording stub that also copies `words` words of the string its
    /// first argument points to into the log entry (in place of the
    /// arguments): the text a port built in its frame reaches the diff.
    fn copy_stub(&mut self, tag: u32, words: u32, pops: u32, ret: u32) -> u32 {
        assert!(words <= 14);
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        c.extend([0x56, 0x57, 0x51]); // push esi; push edi; push ecx
        c.extend([0x8b, 0x74, 0x24, 0x10]); // mov esi, [esp+0x10]
        c.extend([0x8d, 0x78, 0x08]); // lea edi, [eax+8]
        c.push(0xb9);
        c.extend(words.to_le_bytes()); // mov ecx, words
        c.extend([0xf3, 0xa5]); // rep movsd
        c.extend([0x59, 0x5f, 0x5e]); // pop ecx; pop edi; pop esi
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xb8);
        c.extend(ret.to_le_bytes()); // mov eax, ret
        c.push(0xc2);
        c.extend(((pops * 4) as u16).to_le_bytes());
        let x = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), x);
        x
    }

    fn copy_sym(&mut self, sym: u32, tag: u32, words: u32, pops: u32, ret: u32) {
        let s = self.copy_stub(tag, words, pops, ret);
        self.jump(sym, s);
    }

    /// An MSVC `std::string` holding `text` (inline below 16 bytes).
    fn string(&mut self, text: &[u8]) -> u32 {
        let s = self.zeros(0x18);
        if text.len() < 16 {
            self.write(s, text);
            self.w32(s + 0x14, 15);
        } else {
            let mut t = text.to_vec();
            t.push(0);
            let p = self.alloc(&t);
            self.w32(s, p);
            self.w32(s + 0x14, text.len() as u32);
        }
        self.w32(s + 0x10, text.len() as u32);
        s
    }

    /// `operator_new` as a bump allocator over a zeroed block (the game's
    /// heap starts threads the oracle can't run) and a recording
    /// `operator_delete`.
    fn bump_heap(&mut self, tag: u32) {
        let block = self.zeros(0x4_0000);
        let cursor = self.words(&[block]);
        let mut c = vec![0xa1];
        c.extend(cursor.to_le_bytes()); // mov eax, [cursor]
        c.extend([0x8b, 0x4c, 0x24, 0x04]); // mov ecx, [esp+4]
        c.extend([0x83, 0xc1, 0x1f]); // add ecx, 0x1f
        c.extend([0x83, 0xe1, 0xf0]); // and ecx, -16
        c.extend([0x01, 0x0d]);
        c.extend(cursor.to_le_bytes()); // add [cursor], ecx
        c.push(0xc3);
        let x = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), x);
        self.jump(OPERATOR_NEW, x);
        self.stub_sym(ma::OPERATOR_DELETE, tag, false, 1, 0, Ret::Val(0));
    }

    fn rd(&self, side_a: bool, at: u32) -> u32 {
        Mem::read(if side_a { &self.a } else { &self.b }, at)
    }
}

// ---- batch 1: the tweak panels, cursors, strings, the free camera ----

#[test]
fn section_push_back_matches_the_original() {
    // inside the vector (multiples of 12 and not), outside, full (the real
    // grow reallocates) or not, and an end of 0 (nothing copied).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a101);
        for case in 0..40 {
            let n = rng.below(5);
            let spare = if case % 3 == 0 { 0 } else { 1 + rng.below(3) };
            let bytes: Vec<u8> = (0..12 * (n + spare)).map(|_| rng.next() as u8).collect();
            let begin = t.alloc(&bytes);
            let v = t.words(&[begin, begin + 12 * n, begin + 12 * (n + spare)]);
            let item = match rng.below(4) {
                0 if n > 0 => begin + 12 * rng.below(n),
                1 if n > 0 => begin + rng.below(12 * n),
                2 if spare == 0 => begin + 12 * n,
                _ => t.words(&[rng.next(), rng.next(), rng.next()]),
            };
            t.bump_heap(1);
            if case == 7 {
                t.w32(v, 0);
                t.w32(v + 4, 0);
                t.w32(v + 8, 12);
            }
            t.check(
                "SectionVector_pushBack_006ba110",
                ma::SECTION_VECTOR_PUSH,
                Conv::Thiscall(v),
                &[item],
                |g| {
                    a::section_push_back(g, v, item);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

/// The event-section text: lines with three, two, one or no commas,
/// short and long (heap) fields, a last line without its CRLF.
fn section_text(rng: &mut Rng) -> Vec<u8> {
    let mut s = Vec::new();
    for _ in 0..rng.below(6) {
        let mut line = String::new();
        let fields = rng.below(5);
        for f in 0..fields {
            if f > 0 {
                line.push(',');
            }
            match rng.below(4) {
                0 => line.push_str(&(rng.next() as i32).to_string()),
                1 => line.push_str("  12345678901234567890"),
                2 => {}
                _ => line.push_str(&rng.below(1000).to_string()),
            }
        }
        s.extend(line.as_bytes());
        s.extend(b"\r\n");
    }
    if rng.chance(2) {
        s.extend(b"7,8,9");
    }
    s.push(0);
    s
}

#[test]
fn basic_set_enabled_matches_the_original() {
    // off, on hidden, on shown (resize), no file, the file's lines into
    // the section vector (the real string code and push_back, its grow),
    // line 9 reformatted or not.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a102);
        for case in 0..40 {
            let tw = t.zeros(0x120);
            t.w8(tw + 0x28, u8::from(case % 4 != 0));
            let (w, hh) = (rng.next(), rng.next());
            t.w32(tw + 0x108, w);
            t.w32(tw + 0x10c, hh);
            let on = [0, 1, 0x100, 0xff, rng.next()][rng.below(5) as usize];
            let text = if case % 5 == 1 {
                0
            } else if case % 8 == 4 {
                t.alloc(b"7,8,9\0")
            } else {
                let s = section_text(&mut rng);
                t.alloc(&s)
            };
            t.bump_heap(6);
            t.stub_sym(ma::TEXT_SET_ENABLED, 1, true, 1, 1, Ret::Val(rng.next()));
            t.stub_sym(ma::TEXT_RESIZE, 2, true, 2, 2, Ret::Val(rng.next()));
            t.stub_sym(ma::SG_PC_READ, 3, false, 1, 0, Ret::Val(text));
            t.stub_sym(ma::BASIC_LINE_TEXT, 4, true, 2, 2, Ret::Val(rng.next()));
            t.stub_sym(ma::TEXT_CHANGE_LINE, 5, true, 2, 2, Ret::Val(rng.next()));
            let ebp = frame_ebp(&t.a, 1);
            t.b_top = Some(ebp - 0x1a0);
            t.check(
                "BasicDebugTweakUtil_setEnabled",
                0x006b_af60,
                Conv::Thiscall(tw),
                &[on],
                |g| Out::U32(a::basic_set_enabled(g, ebp, tw, on)),
            );
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn set_enabled_variants_match_the_original() {
    // each flag combination; the texture-animation panel out of debug
    // mode, with no animation, with none of its infos, with several.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a103);
        for case in 0..120 {
            let tw = t.zeros(0x210);
            for off in [0_u32, 0x14, 0x1c] {
                t.w8(tw + off, u8::from(rng.chance(2)));
            }
            for off in [0xc8_u32, 0xcc, 0xd8, 0xdc, 0xe0, 0x1fc, 0x200, 0xc, 4] {
                let v = rng.next();
                t.w32(tw + off, v);
            }
            let on = if case % 8 == 3 {
                1
            } else {
                [0, 1, 0x100, rng.next()][rng.below(4) as usize]
            };
            t.stub_sym(ma::TEXT_SET_ENABLED, 1, true, 1, 1, Ret::Val(rng.next()));
            t.stub_sym(ma::TEXT_RESIZE, 2, true, 2, 2, Ret::Val(rng.next()));
            t.stub_sym(ma::CHAR_ANIM_UPDATE, 3, true, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(
                ma::CHAR_ANIM_UPDATE_TEXT,
                4,
                true,
                0,
                0,
                Ret::Val(rng.next()),
            );
            match case % 4 {
                0 => t.check(
                    "CharacterAnimationTweakUtil_setEnabled",
                    0x006b_b5f0,
                    Conv::Thiscall(tw),
                    &[on],
                    |g| Out::U32(a::char_anim_set_enabled(g, tw, on)),
                ),
                1 => t.check(
                    "CharacterSwitchTweakUtil_setEnabled_006bb670",
                    0x006b_b670,
                    Conv::Thiscall(tw),
                    &[on],
                    |g| Out::U32(a::set_enabled(g, ma::CHAR_SWITCH_ENABLE, tw, on)),
                ),
                2 => t.check(
                    "MapSwitchTweakUtil_setEnabled_006bb880",
                    0x006b_b880,
                    Conv::Thiscall(tw),
                    &[on],
                    |g| Out::U32(a::set_enabled(g, ma::MAP_SWITCH_ENABLE, tw, on)),
                ),
                _ => {
                    let dbg = [0, 1, 0x100, 0x1234_5601][(case / 20) % 4];
                    t.stub_sym(ma::IS_DEBUG_MODE, 5, false, 0, 0, Ret::Val(dbg));
                    t.stub_sym(ma::ENABLE_TEX_ANIM, 6, false, 1, 0, Ret::Val(rng.next()));
                    let mgr = t.zeros(4);
                    t.stub_sym(ma::TEX_ANIM_MANAGER, 7, false, 0, 0, Ret::Val(mgr));
                    let n = [0, 1, 3, 0x8000_0002, 0x8000_0000][(case / 4) % 5];
                    let infos = t.zeros(0x40 * 3);
                    let anim = t.words(&[0, 0, n, infos]);
                    let found = if rng.chance(4) { 0 } else { anim };
                    t.stub_sym(ma::TEX_ANIM_FIND, 8, true, 1, 1, Ret::Val(found));
                    t.stub_sym(ma::TEX_ANIM_ACTIVE, 9, true, 0, 0, Ret::Val(rng.next()));
                    t.stub_sym(ma::TEX_ANIM_DEACTIVE, 10, true, 0, 0, Ret::Val(rng.next()));
                    t.stub_sym(
                        ma::TEX_ANIM_SET_BYTE_1C,
                        11,
                        true,
                        1,
                        1,
                        Ret::Val(rng.next()),
                    );
                    t.stub_sym(
                        ma::CHAR_TEX_UPDATE_TEXT,
                        12,
                        true,
                        0,
                        0,
                        Ret::Val(rng.next()),
                    );
                    let viewer = t.zeros(0x60);
                    if !t.snapshot || rng.chance(2) {
                        t.global(ma::VIEWER_MANAGER.sym, viewer);
                    }
                    t.check(
                        "CharacterTextureAnimationTweakUtil_setEnabled_006bb6d0",
                        0x006b_b6d0,
                        Conv::Thiscall(tw),
                        &[on],
                        |g| Out::U32(a::char_tex_set_enabled(g, tw, on)),
                    );
                }
            }
            t.reset();
        }
    });
}

#[test]
fn panel_initializers_match_the_original() {
    // TextUtil_init failing and not; the lines each panel prints (the real
    // sprintf_s, the text copied into the append stub's log): every value
    // branch, the cursor on each line and none.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a104);
        for case in 0..140 {
            let tw = t.zeros(0x200);
            for k in 0..0x80 {
                let v = if rng.chance(3) {
                    rng.next()
                } else {
                    rng.below(3)
                };
                t.w32(tw + 4 * k, v);
            }
            let name = t.alloc(b"bsil0300\0");
            t.write(tw + 8, b"map_name\0");
            t.w32(tw + 0x1b4, name);
            let fail = u32::from(rng.chance(5));
            t.stub_sym(
                ma::TEXT_INIT,
                1,
                true,
                0,
                0,
                Ret::Val(fail * rng.next().max(1)),
            );
            t.stub_sym(ma::TEXT_SET_ENABLED, 2, true, 1, 1, Ret::Val(rng.next()));
            let which = case % 7;
            let (words, ebp_off) = match which {
                3 => (10, 0x2c),
                4 => (5, 0x18),
                5 => (12, 0x38),
                _ => (14, 0x104),
            };
            t.copy_sym(ma::TEXT_APPEND, 3, words, 1, rng.next());
            for f in [
                ma::BASIC_LINE_TEXT,
                ma::BTL_CHR_PARAM_LINE_TEXT,
                ma::GAME_PARAM_LINE_TEXT,
            ] {
                t.stub_sym(f, 4, true, 2, 2, Ret::Val(rng.next()));
            }
            // the texture animation and its getters
            let anim = if rng.chance(3) { 0 } else { t.zeros(0x20) };
            t.w32(tw + 0x18, anim);
            let pa = t.words(&[name | u32::from(rng.chance(2))]);
            let pb = t.words(&[name + 4]);
            t.stub_sym(ma::TEX_ANIM_NAME_A, 5, true, 0, 0, Ret::Val(pa));
            t.stub_sym(ma::TEX_ANIM_NAME_B, 6, true, 0, 0, Ret::Val(pb));
            t.stub_sym(ma::TEX_ANIM_COUNT, 7, true, 0, 0, Ret::Val(rng.next()));
            let flag = [0, 1, 0x100, 0x2ff][rng.below(4) as usize];
            t.stub_sym(ma::TEX_ANIM_FLAG, 8, true, 0, 0, Ret::Val(flag));
            // the motion mode: an index into the name table
            if which == 3 {
                t.w32(tw + 0xc, rng.below(3));
                t.w8(tw, u8::from(rng.chance(2)));
            }
            let ebp = frame_ebp(&t.a, 0);
            t.write(ebp - 0x110, &[0xa5; 0x110]);
            t.b_top = Some(ebp - ebp_off - 0x10);
            let (sym, what) = match which {
                0 => (0x006b_9360, "BasicDebugTweakUtil_initialize"),
                1 => (0x006b_94a0, "BtlChrParamConfigTweakUitl_initialize"),
                2 => (0x006b_9600, "BtlDebugTweakUtil_initialize"),
                3 => (0x006b_97f0, "CharacterAnimationTweakUtil_initialize"),
                4 => (0x006b_9970, "CharacterSwitchTweakUtil_initialize"),
                5 => (0x006b_9aa0, "CharacterTextureAnimationTweakUtil_initialize"),
                _ => (0x006b_9cd0, "GameParamConfigTweakUitl_initialize"),
            };
            t.check(what, sym, Conv::Thiscall(tw), &[], |g| {
                Out::U32(match which {
                    0 => a::init_lines(g, ma::BASIC_INIT, ebp, tw),
                    1 => a::init_lines(g, ma::BTL_CHR_PARAM_INIT, ebp, tw),
                    2 => a::btl_debug_init(g, ebp, tw),
                    3 => a::char_anim_init(g, ebp, tw),
                    4 => a::char_switch_init(g, ebp, tw),
                    5 => a::char_tex_init(g, ebp, tw),
                    _ => a::init_lines(g, ma::GAME_PARAM_INIT, ebp, tw),
                })
            });
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn panel_renders_match_the_original() {
    // hidden, no context, shown: the title of the fps (the real
    // sprintf_s's "%.2f" of the double, NaNs and infinities included) and
    // the map switcher's map name.
    let renders: [(u32, &str, ma::Render); 8] = [
        (0x006b_a1a0, "BasicDebugTweakUtil_render", ma::BASIC_RENDER),
        (
            0x006b_a240,
            "BtlChrParamConfigTweakUitl_render",
            ma::BTL_CHR_PARAM_RENDER,
        ),
        (
            0x006b_a2e0,
            "BtlDebugTweakUtil_render",
            ma::BTL_DEBUG_RENDER,
        ),
        (
            0x006b_a390,
            "CharacterAnimationTweakUtil_render",
            ma::CHAR_ANIM_RENDER,
        ),
        (
            0x006b_a430,
            "CharacterSwitchTweakUtil_render",
            ma::CHAR_SWITCH_RENDER,
        ),
        (
            0x006b_a4d0,
            "CharacterTextureAnimationTweakUtil_render",
            ma::CHAR_TEX_RENDER,
        ),
        (
            0x006b_a570,
            "GameParamConfigTweakUitl_render",
            ma::GAME_PARAM_RENDER,
        ),
        (
            0x006b_a610,
            "MapSwitchTweakUtil_render",
            ma::MAP_SWITCH_RENDER,
        ),
    ];
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a105);
        for case in 0..64 {
            let (sym, what, r) = renders[case % 8];
            let tw = t.zeros(0x200);
            t.w8(tw + r.flag, u8::from(!rng.chance(4)));
            let ctx = if rng.chance(4) { 0 } else { rng.next() | 1 };
            let fps = float_word(&mut rng);
            let viewer = t.zeros(0x80);
            t.write(viewer + 0x51, b"bsil0300\0");
            if !t.snapshot || rng.chance(2) {
                t.global(ma::VIEWER_MANAGER.sym, viewer);
            }
            let words = (r.size / 4).min(14);
            t.copy_sym(ma::TEXT_SET_TITLE, 1, words, 1, rng.next());
            t.stub_sym(ma::TEXT_RENDER, 2, true, 1, 1, Ret::Val(rng.next()));
            let ebp = frame_ebp(&t.a, 2);
            t.write(ebp - 0x110, &[0xa5; 0x110]);
            t.b_top = Some(ebp - r.buf - 0x10);
            t.check(what, sym, Conv::Thiscall(tw), &[ctx, fps], |g| {
                Out::U32(a::render(g, r, ebp, tw, ctx, fps))
            });
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn unwind_cleanups_match_the_original() {
    // the inline buffer (inline or heap), the ATL string's last reference
    // or not, a std::string with heap or inline storage, a vector with or
    // without storage.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a106);
        // InterlockedExchangeAdd as the original's lock xadd
        let code = [
            0x8b, 0x4c, 0x24, 0x04, 0x8b, 0x44, 0x24, 0x08, 0xf0, 0x0f, 0xc1, 0x01, 0xc2, 0x08,
            0x00,
        ];
        let xadd = t.a.alloc_code(&code).unwrap();
        assert_eq!(t.b.alloc_code(&code).unwrap(), xadd);
        for case in 0..32 {
            let b = t.zeros(0x20);
            let p = if case % 2 == 0 { b + 4 } else { t.zeros(8) };
            t.w32(b, p);
            t.import_sym(ma::FREE, 1, 1, 0, Ret::Val(rng.next()));
            t.check(
                "InlineBuffer_free_0064e9a0",
                0x0064_e9a0,
                Conv::Thiscall(b),
                &[],
                |g| Out::U32(a::inline_buffer_free(g, b)),
            );
            t.reset();

            let release = t.stub_code(2, true, 1, 1, Ret::Val(rng.next()), false);
            let manager = t.object(&[0, release], 4);
            let refs = [1, 2, 0, 0xffff_ffff, 0x8000_0000, rng.next()][rng.below(6) as usize];
            let header = t.words(&[manager, 0, 0, refs, 0, 0]);
            let s = t.words(&[header + 0x10]);
            let slot = t.rebase(ma::INTERLOCKED_ADD);
            t.patch(slot, &xadd.to_le_bytes());
            t.check(
                "AtlCString_release_0064e9e0",
                0x0064_e9e0,
                Conv::Thiscall(s),
                &[],
                |g| Out::U32(a::cstring_release(g, s)),
            );
            t.reset();

            let st = t.zeros(0x18);
            let cap = [0xf, 0x10, rng.next()][rng.below(3) as usize];
            let (w0, w4) = (rng.next(), rng.next());
            t.w32(st, w0);
            t.w32(st + 4, w4);
            t.w32(st + 0x14, cap);
            let size = rng.next();
            t.w32(st + 0x10, size);
            t.stub_sym(ma::OPERATOR_DELETE, 3, false, 1, 0, Ret::Val(rng.next()));
            t.check(
                "std_string_tidy_0064ea00",
                0x0064_ea00,
                Conv::Thiscall(st),
                &[],
                |g| {
                    a::string_tidy(g, st);
                    Out::None
                },
            );
            t.reset();

            let p = if rng.chance(3) { 0 } else { rng.next() | 1 };
            let v = t.words(&[p, rng.next(), rng.next()]);
            t.stub_sym(ma::OPERATOR_DELETE, 4, false, 1, 0, Ret::Val(rng.next()));
            t.check(
                "std_vector_tidy_0064ea30",
                0x0064_ea30,
                Conv::Thiscall(v),
                &[],
                |g| Out::U32(a::vector_tidy(g, v)),
            );
            t.reset();
        }
    });
}

#[test]
fn cursor_and_string_helpers_match_the_original() {
    // the cursor with no count, a step inside the block, a step onto the
    // block end (once and repeated), to count 0; std::string compares of
    // inline and heap strings (equal, prefixes, greater, less); appends
    // of C strings (empty included).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a107);
        for case in 0..40 {
            let stride = 4 * (1 + rng.below(4));
            let pos = t.zeros(0x100);
            let end2 = pos + stride * (2 + rng.below(2));
            let end = pos + stride * (1 + rng.below(2));
            let next = if rng.chance(2) { end2 } else { rng.next() };
            t.w32(end, next);
            t.w32(end2, pos + 0x80);
            let count = [0, 1, 2, 3, rng.below(8)][rng.below(5) as usize];
            let mut c = vec![pos, end, stride, count];
            c.extend((0..4).map(|_| rng.next()));
            let c = t.words(&c);
            let out = t.zeros(0x20);
            t.stub_sym(ma::CURSOR_NEXT4, 1, true, 0, 0, Ret::Val(rng.next()));
            let junk = rng.next();
            t.check(
                "PhyreSceneCursor_postNext4_0064f1a0",
                0x0064_f1a0,
                Conv::Thiscall(c),
                &[out, junk],
                |g| Out::U32(a::cursor_post_next4(g, c, out)),
            );
            t.reset();

            let texts: [&[u8]; 6] = [
                b"",
                b"abc",
                b"abd",
                b"abcdefghijklmnopqrstuvwxyz",
                b"abcdefghijklmnopq",
                b"zz",
            ];
            let x = t.string(texts[rng.below(6) as usize]);
            let y = t.string(texts[case % 6]);
            t.check(
                "std_string_less_0064f550",
                0x0064_f550,
                Conv::Stdcall,
                &[x, y],
                |g| Out::U32(a::string_less(g, x, y)),
            );
            t.reset();

            let s = t.zeros(0x18);
            let p = t.alloc(&[texts[case % 6], b"\0"].concat());
            t.stub_sym(ma::STRING_APPEND, 2, true, 2, 2, Ret::Val(rng.next()));
            t.check(
                "std_string_append_cstr_0064f5e0",
                0x0064_f5e0,
                Conv::Thiscall(s),
                &[p],
                |g| Out::U32(a::string_append_cstr(g, s, p)),
            );
            t.reset();
        }
    });
}

#[test]
fn free_camera_attach_matches_the_original() {
    // attached elsewhere (0x16), to this target, not attached; the
    // target's matrix or none (the identity); the real matrix copies.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a108);
        for case in 0..30 {
            let words: Vec<u32> = (0..0x20).map(|_| float_word(&mut rng)).collect();
            let cam = t.words(&words);
            let target = if case == 5 || case == 6 {
                0
            } else {
                t.zeros(0xc0)
            };
            let m: Vec<u32> = (0..12).map(|_| float_word(&mut rng)).collect();
            let m = t.words(&m);
            if target != 0 && !rng.chance(3) {
                t.w32(target + 0xb4, m);
            }
            let attached = match rng.below(3) {
                0 => 0,
                1 => target.wrapping_add(4),
                _ => rng.next() | 1,
            };
            let attached = match case {
                5 => rng.next() | 1,
                6 => 4,
                _ => attached,
            };
            t.w32(cam + 0x78, attached);
            let ebp = frame_ebp(&t.a, 1);
            t.b_top = Some(ebp - 0x90);
            t.check(
                "FreeCamera_attach_0067fda0",
                0x0067_fda0,
                Conv::Thiscall(cam),
                &[target],
                |g| Out::U32(a::free_camera_attach(g, ebp, cam, target)),
            );
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn guide_map_save_matches_the_original() {
    // the double truncated (fractions, negatives, out of range, NaN) into
    // the argument slot and passed on.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a109);
        for case in 0..30 {
            let x: f64 = match case % 6 {
                0 => f64::from(rng.below(1000)) / 7.0,
                1 => -f64::from(rng.below(1000)) / 3.0,
                2 => 3.0e9,
                3 => f64::NAN,
                4 => -2_147_483_648.5,
                _ => f64::from(rng.next()),
            };
            let loc = t.zeros(0x10);
            t.write(loc + 8, &x.to_le_bytes());
            let first = rng.next();
            t.stub_sym(
                ma::GUIDE_SET_LOCATION_TEXT,
                1,
                false,
                2,
                0,
                Ret::Val(rng.next()),
            );
            let slot = first_slot(&t.a, 2);
            t.b_top = Some(slot - 0x40);
            t.check(
                "rcbgGuideMapSave",
                0x006e_9570,
                Conv::Cdecl,
                &[first, loc],
                |g| Out::U32(a::guide_map_save(g, slot, first, loc)),
            );
            t.b_top = None;
            assert_eq!(t.rd(true, slot + 4), t.rd(false, slot + 4), "the slot");
            t.reset();
        }
    });
}

// ---- batch 2: the Sqs straight stream, the debug GUI windows, thunks ----

/// Every global word the stream functions read or write, from
/// `0x012f4e00` to `0x012f4fc0`, randomised (half masks and halves kept
/// small, so each branch is taken).
fn stream_state(t: &mut Twins, rng: &mut Rng) {
    for k in 0..0x70 {
        let v = rng.next();
        t.global(0x012f_4e00 + 4 * k, v);
    }
    for (g, v) in [
        (mb::STREAM_HALF_MASK.sym, rng.below(4)),
        (mb::STREAM_RUNNING.sym, rng.below(2)),
        (mb::STREAM_HALF.sym, rng.below(3)),
        (mb::STREAM_POS.sym, rng.below(3)),
        (mb::STREAM_PREV_HALF.sym, rng.below(3)),
        (
            mb::STREAM_SIZE.sym,
            [0x800, 0x3000, 0x3800, 0x6000, 0xfff][rng.below(5) as usize],
        ),
        (mb::PLAY_MODE.sym, rng.below(3)),
        (mb::DEBUG_FLAGS.sym, rng.below(4)),
    ] {
        t.global(g, v);
    }
}

#[test]
fn stream_next_half_and_record_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a201);
        for _ in 0..60 {
            stream_state(t, &mut rng);
            t.stub_sym(mb::DBG_PRINTF, 1, false, 3, 0, Ret::Val(rng.next()));
            t.check(
                "SqsStraightStreamingNextHalf_0081f160",
                0x0081_f160,
                Conv::Cdecl,
                &[],
                |g| Out::U32(b::stream_next_half(g)),
            );
            t.check(
                "SqsStraightStreamingRecord_0081f760",
                0x0081_f760,
                Conv::Cdecl,
                &[],
                |g| Out::U32(b::stream_record(g)),
            );
            t.reset();
        }
    });
}

#[test]
fn straight_data_send_matches_the_original() {
    // fresh start or not, play modes 0..2 (mode 2 averages the blocks),
    // data, ADPCM or neither, a start buffer or none, the length rules
    // (size a multiple of 0x3000 or not, arg % 6), the DMA retries and the
    // status polls, the debug print.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a202);
        for case in 0..80 {
            stream_state(t, &mut rng);
            if case % 3 == 0 {
                t.global(mb::STREAM_RUNNING.sym, 0);
                t.global(mb::STREAM_HALF_MASK.sym, 3);
            }
            let size = match case % 9 {
                0 => 0xffff_e801,
                1 => 0x3000,
                2 => 0x6000,
                _ => 0x800 * (1 + rng.below(4)),
            };
            t.global(mb::STREAM_SIZE.sym, size);
            let len = if size > 0x8000 { 0x400 } else { size + 0x400 };
            let bytes: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
            let data = if rng.chance(3) { 0 } else { t.alloc(&bytes) };
            let adpcm = if rng.chance(3) { t.zeros(16) } else { 0 };
            let start = if rng.chance(3) { 0 } else { rng.next() | 1 };
            let arg = [0, 6, 7, 12, rng.next()][rng.below(5) as usize];
            let send = t.zeros(0x40);
            t.global(mb::RPC_SEND.sym, send);
            t.stub_sym(mb::DBG_PRINTF, 1, false, 6, 0, Ret::Val(rng.next()));
            t.stub_sym(mb::FLUSH_CACHE, 2, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mb::DECODE_ADPCM, 3, false, 2, 0, Ret::Val(rng.next()));
            let ids = t.seq(&[0, 0, rng.next() | 1, 5]);
            t.stub_sym(mb::SIF_SET_DMA, 4, false, 2, 0, ids);
            let stats = t.seq(&[1, 0, 0x8000_0000, 0xffff_ffff]);
            t.stub_sym(mb::SIF_DMA_STAT, 5, false, 1, 0, stats);
            t.stub_sym(mb::CALL_RPC, 6, false, 2, 0, Ret::Val(rng.next()));
            let slot = first_slot(&t.a, 4);
            t.b_top = Some(slot - 0x40);
            t.check(
                "SqsStraightStreamingDataSend",
                0x0081_f210,
                Conv::Cdecl,
                &[data, start, adpcm, arg],
                |g| {
                    // the caller's argument words, which the original reuses
                    for (k, v) in [data, start, adpcm, arg].into_iter().enumerate() {
                        g.write(slot + 4 * k as u32, v);
                    }
                    Out::U32(b::straight_data_send(g, slot, data, start, adpcm, arg))
                },
            );
            t.b_top = None;
            for k in 0..2 {
                assert_eq!(
                    t.rd(true, slot + 4 * k),
                    t.rd(false, slot + 4 * k),
                    "slot {k}"
                );
            }
            t.reset();
        }
    });
}

#[test]
fn digital_mode_set_matches_the_original() {
    // room in the queue or not (the queue sent first), the debug message.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a203);
        for case in 0..30 {
            let end = t.rebase(mb::CMD_END);
            let next = if case % 2 == 0 {
                end - 0x20 * (1 + rng.below(3))
            } else {
                end - 0x20 + 4 * rng.below(8)
            };
            t.global(mb::CMD_NEXT.sym, next);
            // the queue send moves g_SqsCmdNext back to a fresh top (as
            // the original's send does) and leaves a mark in eax
            t.global(mb::DEBUG_FLAGS.sym, rng.below(4));
            let top = t.alloc(&[0xa5; 0x40]);
            // each send sets g_SqsCmdNext to the next table word: first
            // `next` (full or not), then the emptied queue's top
            let table = t.words(&[next, top, top]);
            let cell = t.words(&[table]);
            let reset = {
                let mut c = vec![0xa1];
                c.extend(cell.to_le_bytes()); // mov eax, [cell]
                c.extend([0x8b, 0x00]); // mov eax, [eax]
                c.push(0xa3);
                c.extend(t.rebase(mb::CMD_NEXT.sym).to_le_bytes()); // mov [next], eax
                c.extend([0x83, 0x05]);
                c.extend(cell.to_le_bytes());
                c.push(0x04); // add dword [cell], 4
                c.extend([0xb8, 0x11, 0x22, 0x33, 0x44, 0xc3]);
                let x = t.a.alloc_code(&c).unwrap();
                assert_eq!(t.b.alloc_code(&c).unwrap(), x);
                x
            };
            if case % 2 == 0 {
                t.patch(next, &[0xa5; 0x20]);
            }
            t.jump(mb::IOP_COMMAND_SEND, reset);
            t.stub_sym(mb::REPORT_STANDBY, 2, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(mb::DBG_PRINTF, 3, false, 1, 0, Ret::Val(rng.next()));
            let x = rng.next();
            t.check(
                "SqsStraightStreamingDigitalModeSet",
                0x0081_f550,
                Conv::Cdecl,
                &[x],
                |g| Out::U32(b::digital_mode_set(g, x)),
            );
            t.reset();
        }
    });
}

#[test]
fn gui_windows_match_the_original() {
    // rcuiWindowCreate with a window already there, a new one with and
    // without a parent; the windows the init functions open.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a204);
        for case in 0..30 {
            t.stub_sym(mb::WINDOW_CREATE, 1, false, 6, 0, Ret::Val(rng.next() | 1));
            t.stub_sym(mb::WINDOW_OPEN, 2, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mb::WINDOW_SET_TITLE, 3, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(mb::WINDOW_SET_PARENT, 4, false, 2, 0, Ret::Val(rng.next()));
            let holder = t.words(&[if case % 3 == 0 { rng.next() | 1 } else { 0 }]);
            let a: Vec<u32> = (0..6).map(|_| rng.next()).collect();
            let title = rng.next();
            let parent = if rng.chance(2) { 0 } else { rng.next() };
            let mut args = vec![holder];
            args.extend(&a);
            args.extend([title, parent]);
            t.check("rcuiWindowCreate", 0x0093_7ef0, Conv::Cdecl, &args, |g| {
                Out::U32(b::window_create(
                    g,
                    holder,
                    [a[0], a[1], a[2], a[3], a[4], a[5]],
                    title,
                    parent,
                ))
            });
            t.check("yiGui_setfogparam", 0x0090_8f90, Conv::Cdecl, &[], |g| {
                Out::U32(b::set_fog_param(g))
            });
            let holder = t.rebase(mb::FP_MEM_WINDOW);
            let w = if case % 2 == 0 { 0 } else { rng.next() | 1 };
            t.patch(holder, &w.to_le_bytes());
            t.check("Yn_FpMemWinOpen", 0x0093_3c80, Conv::Cdecl, &[], |g| {
                Out::U32(b::fp_mem_win_open(g))
            });
            let work = t.zeros(0xc0c);
            if case % 2 == 1 {
                t.w32(work, rng.next() | 1);
            }
            t.stub_sym(mb::RC_CALLOC, 5, false, 4, 0, Ret::Val(work));
            t.global(mb::RCUI_WORK.sym, rng.next());
            t.check("rcuiInit", 0x0093_4390, Conv::Cdecl, &[], |g| {
                Out::U32(b::rcui_init(g))
            });
            t.reset();
        }
    });
}

#[test]
fn small_field_helpers_match_the_original() {
    // the status bit, the fog init, the curve address, the map chain's
    // bit (each link missing in turn), the GS packet send, the chunk load
    // (no file, no memory, read).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a205);
        for case in 0..40 {
            let v = rng.next();
            t.global(mb::YI_RENDER_STATUS.sym, v);
            t.check(
                "YiRenderStatus_clearBit8_00908440",
                0x0090_8440,
                Conv::Cdecl,
                &[],
                |g| {
                    b::yi_status_clear_bit8(g);
                    Out::None
                },
            );
            t.stub_sym(mb::YI_RENDER_INIT_FOG, 1, false, 1, 0, Ret::Val(rng.next()));
            t.check(
                "YiRender_initFogDefault_0090a420",
                0x0090_a420,
                Conv::Cdecl,
                &[],
                |g| Out::U32(b::yi_init_fog(g)),
            );
            t.check(
                "TkVibrateCurve_get_009090b0",
                0x0090_90b0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(b::vibrate_curve(g)),
            );

            let list = t.words(&[rng.next()]);
            let map = t.zeros(0x10);
            t.w32(map + 8, if case % 4 == 1 { 0 } else { list });
            let sys = t.words(&[if case % 4 == 2 { 0 } else { map }]);
            t.global(mb::YN_MAP_SYSTEM.sym, if case % 4 == 3 { 0 } else { sys });
            t.stub_sym(mb::SCENE_CALL_CLEAR, 2, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(mb::SCENE_CALL_SET, 3, false, 0, 0, Ret::Val(rng.next()));
            t.check(
                "YnMapChain_clearBit0_0091a200",
                0x0091_a200,
                Conv::Cdecl,
                &[],
                |g| Out::U32(b::chain_bit0(g, false)),
            );
            t.check(
                "YnMapChain_setBit0_0091a220",
                0x0091_a220,
                Conv::Cdecl,
                &[],
                |g| Out::U32(b::chain_bit0(g, true)),
            );

            let chan = t.words(&[rng.next()]);
            t.stub_sym(mb::MFIFO_SYNC, 4, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(mb::GS_SYNC_PATH, 5, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(mb::DMA_GET_CHAN, 6, false, 1, 0, Ret::Val(chan));
            t.stub_sym(mb::DMA_SYNC, 7, false, 3, 0, Ret::Val(rng.next()));
            t.stub_sym(mb::FLUSH_CACHE, 8, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mb::DMA_SEND_N, 9, false, 3, 0, Ret::Val(rng.next()));
            let (packet, qwc) = (rng.next(), rng.next());
            let slot = first_slot(&t.a, 2);
            t.b_top = Some(slot - 0x40);
            t.check(
                "yiSendGsPacket",
                0x0091_2690,
                Conv::Cdecl,
                &[packet, qwc],
                |g| Out::U32(b::send_gs_packet(g, slot, packet, qwc)),
            );
            t.b_top = None;
            for k in 0..2 {
                assert_eq!(
                    t.rd(true, slot + 4 * k),
                    t.rd(false, slot + 4 * k),
                    "slot {k}"
                );
            }

            let chunk = rng.next();
            let path = rng.next();
            t.stub_sym(mb::CHUNK_FINI, 10, false, 1, 0, Ret::Val(rng.next()));
            let f = if case % 5 == 0 { 0 } else { rng.next() | 1 };
            t.stub_sym(mb::SCE_OPEN, 11, false, 2, 0, Ret::Val(f));
            let sizes = t.seq(&[rng.below(0x1000), rng.next()]);
            t.stub_sym(mb::SCE_LSEEK, 12, false, 3, 0, sizes);
            let buf = if case % 5 == 1 { 0 } else { rng.next() | 1 };
            t.stub_sym(mb::RC_ALLOC, 13, false, 3, 0, Ret::Val(buf));
            t.stub_sym(mb::SCE_READ, 14, false, 3, 0, Ret::Val(rng.next()));
            t.stub_sym(mb::CHUNK_SET, 15, false, 3, 0, Ret::Val(rng.next()));
            t.stub_sym(mb::SCE_CLOSE, 16, false, 1, 0, Ret::Val(rng.next()));
            t.check(
                "rcdtChunkLoadFromFile",
                0x0092_b880,
                Conv::Cdecl,
                &[chunk, path],
                |g| Out::U32(b::chunk_load_from_file(g, chunk, path)),
            );
            t.reset();
        }
    });
}

#[test]
fn jump_thunks_match_the_original() {
    // each thunk with its target stubbed: the words passed on and the
    // target's eax.
    let thunks = [
        mb::SE_SEPARATE_MASK_CLEAR_THUNK,
        mb::SET_CYCLE_MODE_THUNK,
        mb::SET_REVERB_PLAY_SWITCH_THUNK,
        mb::ALL_VOICE_TERMINATE_THUNK,
        mb::SHOUT_STREAMING_DATA_SEND_THUNK,
        mb::YI_FREE,
        mb::RCBG_INIT_THUNK,
        mb::FP_DISP_DEBUG_THUNK,
        mb::FP_MEM_WIN_OPEN_THUNK,
        mb::PRE_BG_SET_DATA_THUNK_A,
        mb::TRX_PAIR_SET_NO_THUNK,
        mb::PRE_BG_SET_DATA_THUNK_B,
        mb::SCENE_SET_CHADIST_THUNK,
        mb::RC_FREE,
        mb::RC_FREE_THUNK,
        mb::TASK_RET_SEQ_CONT_THUNK,
    ];
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a206);
        for (k, th) in thunks.iter().enumerate() {
            let args: Vec<u32> = (0..8).map(|_| rng.next()).collect();
            // a thunk whose target is itself a thunk: stub the final one
            let target = if th.target == mb::RC_FREE.at {
                mb::RC_FREE.target
            } else {
                th.target
            };
            t.stub_sym(
                target,
                k as u32 + 1,
                false,
                th.words as u32,
                0,
                Ret::Val(rng.next()),
            );
            let words = &args[..th.words];
            t.check("thunk", th.at, Conv::Cdecl, words, |g| {
                Out::U32(b::thunk(g, *th, words))
            });
            t.reset();
        }
    });
}
