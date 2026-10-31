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
    clippy::format_push_string, // the generated test files
    dead_code // the harness is shared by every batch; not each uses every helper
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_prender::m5m_a::{self as ma, bridge as a};
use ffx_prender::m5m_b::{self as mb, bridge as b};
use ffx_prender::m5m_c::{self as mc, bridge as c};
use ffx_prender::m5m_d::{self as md, bridge as d};
use ffx_prender::m5m_e::{self as me, bridge as e};
use ffx_prender::m5m_f::{self as mf, bridge as f};
use ffx_prender::m5m_g::{self as mg, bridge as g};
use ffx_prender::m5m_h::{self as mh, bridge as h};
use ffx_prender::m5m_i::{self as mi, bridge as gi};

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
    fn call_st0(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> f32 {
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        let r = self.0.call(addr, conv(c), args).unwrap();
        r.st0_f32().unwrap_or_else(|| panic!("{addr:#x}: no ST0"))
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

// ---- batch 3: the TIM2 loader and bounds, the rc packets, wrappers ----

/// A TIM2 picture record (0x4c bytes) whose stored pointers
/// (`get_ptr32_func` is stubbed to the identity) lead to a palette and an
/// 8-bit image of `w` x `h`.
fn tm2_picture(t: &mut Twins, rng: &mut Rng, w: u16, h: u16) -> u32 {
    let pic = t.zeros(0x4c);
    let clut: Vec<u32> = (0..256)
        .map(|_| match rng.below(6) {
            0 => 0,
            1 => 0xff00_0000,
            2 => rng.next() & 0x00ff_ffff,
            // the masks' edges: the alpha's top bit alone, a colour only
            // in its third byte
            3 => 0x8000_0000,
            4 => 0x00ab_0000,
            _ => rng.next(),
        })
        .collect();
    let clut = t.words(&clut);
    let n = usize::from(w) * usize::from(h);
    let sparse = rng.chance(2);
    let img: Vec<u8> = (0..n.max(1))
        .map(|_| {
            if sparse && !rng.chance(8) {
                0
            } else {
                rng.next() as u8
            }
        })
        .collect();
    let img = t.alloc(&img);
    t.w32(pic + 0x40, img);
    t.w32(pic + 0x44, clut);
    t.write(pic + 0x24, &w.to_le_bytes());
    t.write(pic + 0x26, &h.to_le_bytes());
    pic
}

/// `get_ptr32_func` as the identity (the first argument back), popping
/// nothing.
fn identity_ptr32(t: &mut Twins) {
    let code = [0x8b, 0x44, 0x24, 0x04, 0xc3]; // mov eax, [esp+4]; ret
    let x = t.a.alloc_code(&code).unwrap();
    assert_eq!(t.b.alloc_code(&code).unwrap(), x);
    t.jump(mc::GET_PTR32, x);
}

#[test]
fn tm2_bounds_match_the_original() {
    // both kinds; pictures with no counted pixel, odd and even corners,
    // empty sizes; the palette rules (alpha, black key).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a301);
        for case in 0..60 {
            identity_ptr32(t);
            let (w, h) = match case % 6 {
                0 => (0, 4),
                1 => (5, 0),
                _ => (1 + rng.below(40) as u16, 1 + rng.below(40) as u16),
            };
            let pic = tm2_picture(t, &mut rng, w, h);
            if case % 5 == 4 {
                // nothing counted
                let clut = t.r32(pic + 0x44);
                let fill = if case % 2 == 0 {
                    0x0012_3456_u32
                } else {
                    0x0100_0000
                };
                for k in 0..256 {
                    t.w32(clut + 4 * k, fill);
                }
            }
            let out = t.zeros(0x10);
            let o = [out, out + 4, out + 8, out + 12];
            let (sym, kind, what) = if case % 2 == 0 {
                (
                    0x0092_34d0,
                    mc::Opaque::Alpha,
                    "rcdtTm2OpaqueBounds_009234d0",
                )
            } else {
                (0x0092_3660, mc::Opaque::Black, "rcdtTm2KeyBounds_00923660")
            };
            let ebp = frame_ebp(&t.a, 5);
            t.b_top = Some(ebp - 0x440);
            t.check(
                what,
                sym,
                Conv::Cdecl,
                &[pic, o[0], o[1], o[2], o[3]],
                |g| Out::U32(c::tm2_bounds(g, kind, pic, o)),
            );
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn tm2_load_matches_the_original() {
    // no file (with and without a chunk), image types 4, 5 and others,
    // the PS2 alpha conversion or not, parse and chunk freed.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a302);
        for case in 0..40 {
            let pic = t.zeros(0x4c);
            let chunk = if case % 7 == 0 { 0 } else { rng.next() | 1 };
            t.stub_sym(mc::CHUNK_NEW, 1, false, 0, 0, Ret::Val(chunk));
            let loaded = u32::from(case % 5 != 1);
            t.stub_sym(mc::CHUNK_LOAD_FROM_FILE, 2, false, 2, 0, Ret::Val(loaded));
            t.stub_sym(mc::CHUNK_DELETE, 3, false, 1, 0, Ret::Val(rng.next()));
            // rcPrint is varargs: the second call passes one word (the
            // words after it are the first call's, left on the stack)
            t.stub_sym(mc::RC_PRINT, 4, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::TM2_FINI, 5, false, 1, 0, Ret::Val(rng.next()));
            let parse = if case % 9 == 3 { 0 } else { rng.next() | 1 };
            t.stub_sym(mc::PARSE_NEW, 6, false, 0, 0, Ret::Val(parse));
            t.stub_sym(mc::PARSE_SET, 7, false, 2, 0, Ret::Val(rng.next()));
            // the header reads fill the picture: the first two calls write
            // 0x10 and 0x30 bytes at their destination
            let kind = [4_u8, 5, 3, 0][rng.below(4) as usize];
            let mut head = vec![0_u8; 0x40];
            for b in &mut head {
                *b = rng.next() as u8;
            }
            head[0x10 + 0x13] = kind;
            t.write(pic, &head);
            t.stub_sym(mc::PARSE_GET_BIN, 8, false, 3, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::PARSE_SET_POS, 9, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::TM2_GET_PSM, 10, false, 1, 0, Ret::Val(rng.next()));
            let allocs = t.seq(&[rng.next(), rng.next()]);
            t.stub_sym(mc::RC_ALLOC, 11, false, 3, 0, allocs);
            t.stub_sym(mc::SET_PTR32, 12, false, 1, 0, Ret::Val(rng.next()));
            // the third call passes one word (stale ones follow it)
            t.stub_sym(mc::GET_PTR32, 13, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(
                mc::TM2_CLUT_ALPHA_TO_PS2,
                14,
                false,
                1,
                0,
                Ret::Val(rng.next()),
            );
            t.stub_sym(mc::RC_FREE_THUNK, 15, false, 1, 0, Ret::Val(rng.next()));
            let path = rng.next();
            let alpha = rng.below(2);
            t.check(
                "rcdtTm2LoadEx_009232a0",
                0x0092_32a0,
                Conv::Cdecl,
                &[pic, path, alpha],
                |g| Out::U32(c::tm2_load(g, pic, path, alpha)),
            );
            t.reset();
            t.stub_sym(mc::TM2_LOAD, 16, false, 3, 0, Ret::Val(rng.next()));
            t.check("rcdtTm2Load", 0x0092_3800, Conv::Cdecl, &[pic, path], |g| {
                Out::U32(c::tm2_load_ps2(g, pic, path))
            });
            t.reset();
        }
    });
}

#[test]
fn load_mat_and_id_load_match_the_original() {
    // no extension, "mat", "MA2" (the real _stricmp), others; the ID file
    // (no file, read).
    let paths: [&[u8]; 6] = [
        b"model\0",
        b"a/b.mat\0",
        b"a.b/c.MA2\0",
        b"x.ma3\0",
        b"x.\0",
        b"x.mat.ma2\0",
    ];
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a303);
        for case in 0..24 {
            let path = t.alloc(paths[case % 6]);
            let model = rng.next();
            t.stub_sym(mc::LOAD_MAT_RSD, 1, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::LOAD_MAT_RSD2, 2, false, 2, 0, Ret::Val(rng.next()));
            t.check(
                "rcdtRsdLoad_LoadMat_00923b60",
                0x0092_3b60,
                Conv::Cdecl,
                &[model, path],
                |g| Out::U32(c::load_mat(g, model, path)),
            );
            t.reset();

            let work = t.zeros(0x200);
            t.write(work + 0x174, b"c:/ffx\0");
            t.global(mc::RCUI_WORK.sym, work);
            let f = if case % 3 == 0 { 0 } else { rng.next() | 1 };
            t.copy_sym(mc::SCE_OPEN, 3, 6, 0, f);
            let sizes = t.seq(&[rng.below(0x1000), rng.next()]);
            t.stub_sym(mc::SCE_LSEEK, 4, false, 3, 0, sizes);
            t.stub_sym(mc::USER_MALLOC, 5, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::SCE_READ, 6, false, 3, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::SCE_CLOSE, 7, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::ID_FIXUP, 8, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::GET_PLAYER, 9, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::EN_MAP_ID, 10, false, 2, 0, Ret::Val(rng.next()));
            let ebp = frame_ebp(&t.a, 0);
            t.b_top = Some(ebp - 0x120);
            t.check("rcbgIDLoad", 0x0092_5730, Conv::Cdecl, &[], |g| {
                Out::U32(c::id_load(g, ebp))
            });
            t.b_top = None;
            t.reset();
        }
    });
}

/// `rcVectorToFixed_00927ac0` as a stub: logs both pointers and the four
/// input floats, writes four words derived from them to `out`.
fn to_fixed_stub(t: &mut Twins) {
    let log = t.log;
    let mut c = vec![0x56, 0x57]; // push esi; push edi
    c.extend([0x8b, 0x74, 0x24, 0x10]); // mov esi, [esp+0x10] (in)
    c.extend([0x8b, 0x7c, 0x24, 0x0c]); // mov edi, [esp+0xc] (out)
    c.push(0xa1);
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00, 0x77, 0, 0, 0]); // mov [eax], 0x77
    c.extend([0x89, 0x78, 0x04]); // mov [eax+4], edi
    c.extend([0x89, 0x70, 0x08]); // mov [eax+8], esi
    for k in 0..4_u8 {
        c.extend([0x8b, 0x4e, 4 * k]); // mov ecx, [esi+4k]
        c.extend([0x89, 0x48, 0x0c + 4 * k]); // mov [eax+0xc+4k], ecx
        c.extend([0x81, 0xf1, 0x5a, 0x5a, 0x00, 0x00]); // xor ecx, 0x5a5a
        c.extend([0x89, 0x4f, 4 * k]); // mov [edi+4k], ecx
    }
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    c.extend([0x5f, 0x5e, 0xc3]); // pop edi; pop esi; ret
    let x = t.a.alloc_code(&c).unwrap();
    assert_eq!(t.b.alloc_code(&c).unwrap(), x);
    t.jump(mc::TO_FIXED, x);
}

#[test]
fn rc_packets_match_the_original() {
    // the rect, circle and line packets: colours (fractions, negatives,
    // NaN, out of range), positions and sizes; the frame's never-written
    // words are 0 in the original's frame too.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a304);
        for case in 0..36 {
            to_fixed_stub(t);
            let buf = t.zeros(0x200);
            let pkt = t.words(&[0, buf]);
            let rgba: Vec<u32> = (0..4)
                .map(|_| match rng.below(4) {
                    0 => (rng.below(512) as f32 / 3.0).to_bits(),
                    1 => 3.0e9_f32.to_bits(),
                    _ => float_word(&mut rng),
                })
                .collect();
            let rgba = t.words(&rgba);
            let f: Vec<u32> = (0..4).map(|_| float_word(&mut rng)).collect();
            let ebp = frame_ebp(&t.a, if case % 3 == 1 { 5 } else { 6 });
            t.write(ebp - 0x20, &[0; 0x20]);
            t.b_top = Some(ebp - 0x40);
            match case % 3 {
                0 => t.check(
                    "rcPacketRect_00927630",
                    0x0092_7630,
                    Conv::Cdecl,
                    &[pkt, f[0], f[1], f[2], f[3], rgba],
                    |g| Out::U32(c::packet_rect(g, ebp, pkt, [f[0], f[1], f[2], f[3], rgba])),
                ),
                1 => t.check(
                    "rcPacketCircle_00927720",
                    0x0092_7720,
                    Conv::Cdecl,
                    &[pkt, f[0], f[1], f[2], rgba],
                    |g| Out::U32(c::packet_circle(g, ebp, pkt, [f[0], f[1], f[2], rgba])),
                ),
                _ => t.check(
                    "rcPacketLine_00927810",
                    0x0092_7810,
                    Conv::Cdecl,
                    &[pkt, f[0], f[1], f[2], f[3], rgba],
                    |g| Out::U32(c::packet_line(g, ebp, pkt, [f[0], f[1], f[2], f[3], rgba])),
                ),
            }
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn map_wrappers_match_the_original() {
    // the arguments passed on, the jump map present or not, the render
    // status bit.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a305);
        for case in 0..16 {
            let (x, y) = (rng.next(), rng.next());
            t.stub_sym(mc::FUN_0092C600, 1, false, 3, 0, Ret::Val(rng.next()));
            t.check(
                "Wrap_0092c600_0092c5e0",
                0x0092_c5e0,
                Conv::Cdecl,
                &[x, y],
                |g| Out::U32(c::wrap_0092c600(g, x, y)),
            );
            t.stub_sym(mc::STAGE_NAME2, 2, false, 3, 0, Ret::Val(rng.next()));
            t.check("rcuiGetStageName", 0x0093_7940, Conv::Cdecl, &[x, y], |g| {
                Out::U32(c::stage_name(g, x, y))
            });
            let map = if case % 3 == 0 { 0 } else { rng.next() };
            t.stub_sym(mc::JUMP_MAP_NO, 3, false, 0, 0, Ret::Val(map));
            t.stub_sym(mc::FP_SAVE_SUB, 4, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::GUIDE_MAP_CONV, 5, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::RENDER_STATUS, 6, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::FP_LOAD_TEXT_SUB, 7, false, 2, 0, Ret::Val(rng.next()));
            t.check(
                "rcbgFpSaveCurrent_00938340",
                0x0093_8340,
                Conv::Cdecl,
                &[],
                |g| Out::U32(c::fp_save_current(g)),
            );
            t.check(
                "rcbgGuideMapConvCurrent_00938570",
                0x0093_8570,
                Conv::Cdecl,
                &[],
                |g| Out::U32(c::guide_map_conv_current(g)),
            );
            t.check(
                "rcbgFpLoadTextCurrent_0093a7e0",
                0x0093_a7e0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(c::fp_load_text_current(g)),
            );
            t.reset();
        }
    });
}

#[test]
fn load_mat_rsd_matches_the_original() {
    // no file (with and without a chunk); comment lines; the count equal
    // to the model's or not (the assertion message), 0 and negative;
    // every polygon type and an unknown one (the real text parse over a
    // generated file: its chunk and parse code run in both oracles).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a306);
        for case in 0..30 {
            let n = [0_i32, 1, 3, 5, -1][case % 5];
            let mut text = String::new();
            for _ in 0..rng.below(3) {
                text.push_str(if rng.chance(2) {
                    "@ header\n"
                } else {
                    "# note\n"
                });
            }
            text.push_str(&format!("{n}\n"));
            for _ in 0..n.max(0) {
                let kind = ['C', 'D', 'G', 'H', 'T', 'X'][rng.below(6) as usize];
                text.push_str(&format!("1 2 3 {kind}"));
                for _ in 0..16 {
                    text.push_str(&format!(" {}", rng.below(300) as i32 - 20));
                }
                text.push('\n');
            }
            let mut bytes = text.into_bytes();
            bytes.push(0);
            let size = bytes.len() as u32;
            let data = t.alloc(&bytes);
            let polys = t.alloc(&vec![0x5a; 0x58 * 6]);
            let model = t.zeros(0x20);
            let declared = if case % 4 == 3 {
                n as u32 + 1
            } else {
                n as u32
            };
            t.w32(model + 0x10, declared);
            t.w32(model + 0x14, polys);
            let loads = case % 6 != 5;
            // the chunk holds the generated file
            let chunk = if case % 7 == 6 && !loads {
                0
            } else {
                t.words(&[data, size])
            };
            t.stub_sym(mc::CHUNK_NEW, 1, false, 0, 0, Ret::Val(chunk));
            t.stub_sym(
                mc::CHUNK_LOAD_FROM_FILE,
                2,
                false,
                2,
                0,
                Ret::Val(u32::from(loads)),
            );
            t.stub_sym(mc::CHUNK_DELETE, 3, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::RC_PRINT, 4, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::RC_FREE_THUNK, 5, false, 1, 0, Ret::Val(rng.next()));
            // rcdtParseNew without the heap: a zeroed block through the
            // real rcdtParseInit
            let block = t.zeros(0x8c);
            let init = t.rebase(0x0092_bf50);
            let mut code = vec![0x68];
            code.extend(block.to_le_bytes()); // push block
            code.push(0xb8);
            code.extend(init.to_le_bytes()); // mov eax, rcdtParseInit
            code.extend([0xff, 0xd0, 0x83, 0xc4, 0x04]); // call eax; add esp, 4
            code.push(0xb8);
            code.extend(block.to_le_bytes()); // mov eax, block
            code.push(0xc3);
            let x = t.a.alloc_code(&code).unwrap();
            assert_eq!(t.b.alloc_code(&code).unwrap(), x);
            t.jump(mc::PARSE_NEW, x);
            let path = rng.next();
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - 0x40);
            t.check(
                "rcdtRsdLoad_LoadMatRsd",
                0x0092_3bc0,
                Conv::Cdecl,
                &[model, path],
                |g| Out::U32(c::load_mat_rsd(g, ebp, model, path)),
            );
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn load_mat_rsd2_matches_the_original() {
    // no file (with and without a chunk); comment lines; the count equal
    // to the model's or not (the assertion message), 0 and negative;
    // every polygon type and an unknown one (the real text parse over a
    // generated file: its chunk and parse code run in both oracles).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a307);
        for case in 0..30 {
            let n = [0_i32, 1, 3, 5, -1][case % 5];
            let mut text = String::new();
            for _ in 0..rng.below(3) {
                text.push_str(if rng.chance(2) {
                    "@ header\n"
                } else {
                    "# note\n"
                });
            }
            text.push_str(&format!("{n}\n"));
            for _ in 0..n.max(0) {
                let kind = ['H', 'X', 'C'][rng.below(3) as usize];
                text.push_str(&format!("1 2 3 4 5 6 7 8 {kind}"));
                for _ in 0..25 {
                    text.push_str(&format!(" {}", rng.below(300) as i32 - 20));
                }
                text.push('\n');
            }
            let mut bytes = text.into_bytes();
            bytes.push(0);
            let size = bytes.len() as u32;
            let data = t.alloc(&bytes);
            let polys = t.alloc(&vec![0x5a; 0x58 * 6]);
            let model = t.zeros(0x20);
            let declared = if case % 4 == 3 {
                n as u32 + 1
            } else {
                n as u32
            };
            t.w32(model + 0x10, declared);
            t.w32(model + 0x14, polys);
            let loads = case % 6 != 5;
            // the chunk holds the generated file
            let chunk = if case % 7 == 6 && !loads {
                0
            } else {
                t.words(&[data, size])
            };
            t.stub_sym(mc::CHUNK_NEW, 1, false, 0, 0, Ret::Val(chunk));
            t.stub_sym(
                mc::CHUNK_LOAD_FROM_FILE,
                2,
                false,
                2,
                0,
                Ret::Val(u32::from(loads)),
            );
            t.stub_sym(mc::CHUNK_DELETE, 3, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::RC_PRINT, 4, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::RC_FREE_THUNK, 5, false, 1, 0, Ret::Val(rng.next()));
            // rcdtParseNew without the heap: a zeroed block through the
            // real rcdtParseInit
            let block = t.zeros(0x8c);
            let init = t.rebase(0x0092_bf50);
            let mut code = vec![0x68];
            code.extend(block.to_le_bytes()); // push block
            code.push(0xb8);
            code.extend(init.to_le_bytes()); // mov eax, rcdtParseInit
            code.extend([0xff, 0xd0, 0x83, 0xc4, 0x04]); // call eax; add esp, 4
            code.push(0xb8);
            code.extend(block.to_le_bytes()); // mov eax, block
            code.push(0xc3);
            let x = t.a.alloc_code(&code).unwrap();
            assert_eq!(t.b.alloc_code(&code).unwrap(), x);
            t.jump(mc::PARSE_NEW, x);
            let path = rng.next();
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - 0x40);
            t.check(
                "rcdtRsdLoad_LoadMatRsd2",
                0x0092_3ed0,
                Conv::Cdecl,
                &[model, path],
                |g| Out::U32(c::load_mat_rsd2(g, ebp, model, path)),
            );
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn load_ply_matches_the_original() {
    // no file; comment lines; zero, one and several vertices, normals and
    // polygons (negative counts too); float text the real atof reads
    // (fractions, exponents, signs); polygon flags and indices past 16
    // bits.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a308);
        for case in 0..30 {
            let counts = [[0_i32, 0, 0], [1, 1, 1], [3, 2, 4], [-1, 2, 0], [2, -3, 1]][case % 5];
            let mut text = String::new();
            for _ in 0..rng.below(3) {
                text.push_str(if rng.chance(2) { "@ ply\n" } else { "# note\n" });
            }
            text.push_str(&format!("{} {} {}\n", counts[0], counts[1], counts[2]));
            // each block follows a label line (rcdtParseAdvanceLine skips
            // the line the cursor is on)
            for &n in &counts[..2] {
                text.push_str("vectors\n");
                for _ in 0..n.max(0) {
                    for _ in 0..3 {
                        let x = match rng.below(4) {
                            0 => format!("{}", rng.below(1000) as i32 - 500),
                            1 => format!("{}.{}", rng.below(100) as i32 - 50, rng.below(1000)),
                            2 => format!("{}e{}", rng.below(9) + 1, rng.below(20) as i32 - 10),
                            _ => format!("-0.{}", rng.below(100_000)),
                        };
                        text.push_str(&x);
                        text.push(' ');
                    }
                }
                text.push('\n');
            }
            text.push_str("polygons\n");
            for _ in 0..counts[2].max(0) {
                text.push_str(&format!("{}", rng.below(2)));
                for _ in 0..8 {
                    text.push_str(&format!(" {}", rng.below(0x2_0000)));
                }
                text.push(' ');
            }
            text.push('\n');
            let mut bytes = text.into_bytes();
            bytes.push(0);
            let size = bytes.len() as u32;
            let data = t.alloc(&bytes);
            let model = t.zeros(0x20);
            let loads = case % 6 != 5;
            let chunk = if case % 7 == 6 && !loads {
                0
            } else {
                t.words(&[data, size])
            };
            t.stub_sym(mc::CHUNK_NEW, 1, false, 0, 0, Ret::Val(chunk));
            t.stub_sym(
                mc::CHUNK_LOAD_FROM_FILE,
                2,
                false,
                2,
                0,
                Ret::Val(u32::from(loads)),
            );
            t.stub_sym(mc::CHUNK_DELETE, 3, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::RC_PRINT, 4, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::RC_FREE_THUNK, 5, false, 1, 0, Ret::Val(rng.next()));
            let blocks: Vec<u32> = (0..3).map(|_| t.alloc(&[0x5a; 0x58 * 5])).collect();
            let allocs = t.seq(&blocks);
            t.stub_sym(mc::RC_ALLOC, 6, false, 3, 0, allocs);
            let block = t.zeros(0x8c);
            let init = t.rebase(0x0092_bf50);
            let mut code = vec![0x68];
            code.extend(block.to_le_bytes()); // push block
            code.push(0xb8);
            code.extend(init.to_le_bytes()); // mov eax, rcdtParseInit
            code.extend([0xff, 0xd0, 0x83, 0xc4, 0x04]); // call eax; add esp, 4
            code.push(0xb8);
            code.extend(block.to_le_bytes()); // mov eax, block
            code.push(0xc3);
            let x = t.a.alloc_code(&code).unwrap();
            assert_eq!(t.b.alloc_code(&code).unwrap(), x);
            t.jump(mc::PARSE_NEW, x);
            let path = rng.next();
            t.check(
                "rcdtRsdLoad_LoadPly",
                0x0092_4090,
                Conv::Cdecl,
                &[model, path],
                |g| Out::U32(c::load_ply(g, model, path)),
            );
            t.reset();
        }
    });
}

#[test]
fn rsd_load_matches_the_original() {
    // no file; a path with and without a directory; every keyword
    // (mixed case, the real keyword match), comment lines, unknown
    // words; the file names the loaders get (their stubs log the text).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a309);
        for case in 0..30 {
            let mut text = String::new();
            let mut ntex = 0;
            for _ in 0..=rng.below(8) {
                match rng.below(8) {
                    0 => text.push_str("# comment line\n"),
                    1 => text.push_str(&format!("PLY = Model{}.PLY\n", rng.below(9))),
                    2 => text.push_str(&format!("mat=[Mat{}.Mat]\n", rng.below(9))),
                    3 => text.push_str("GRP = GroupName\n"),
                    4 => {
                        ntex = rng.below(3);
                        text.push_str(&format!("NTEX = {ntex}\n"));
                    }
                    5 if ntex > 0 => {
                        text.push_str(&format!(
                            "TEX[{}] = tex{}.tm2\n",
                            rng.below(ntex),
                            rng.below(99)
                        ));
                    }
                    // a VGR value or a comment that reads as a keyword
                    6 => text.push_str("VGR = MAT x.mat\n"),
                    _ => text.push_str("@ PLY header.ply\n"),
                }
            }
            let mut bytes = text.into_bytes();
            bytes.push(0);
            let size = bytes.len() as u32;
            let data = t.alloc(&bytes);
            let model = t.zeros(0x20);
            let path: &[u8] =
                [&b"host:/ffx/map/test.rsd\0"[..], b"plain.rsd\0", b"a/b/\0"][case % 3];
            let path = t.alloc(path);
            let loads = case % 6 != 5;
            let chunk = if case % 7 == 6 && !loads {
                0
            } else {
                t.words(&[data, size])
            };
            t.stub_sym(mc::CHUNK_NEW, 1, false, 0, 0, Ret::Val(chunk));
            t.stub_sym(
                mc::CHUNK_LOAD_FROM_FILE,
                2,
                false,
                2,
                0,
                Ret::Val(u32::from(loads)),
            );
            t.stub_sym(mc::CHUNK_DELETE, 3, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::RC_PRINT, 4, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mc::RC_FREE_THUNK, 5, false, 1, 0, Ret::Val(rng.next()));
            let blocks: Vec<u32> = (0..12).map(|_| t.alloc(&[0x5a; 0x40])).collect();
            let allocs = t.seq(&blocks);
            t.stub_sym(mc::RC_ALLOC, 6, false, 3, 0, allocs);
            let s = t.copy_stub_arg(7, 1, 12, 0, rng.next());
            t.jump(mc::LOAD_PLY, s);
            let s = t.copy_stub_arg(8, 1, 12, 0, rng.next());
            t.jump(mc::LOAD_MAT, s);
            let block = t.zeros(0x8c);
            let init = t.rebase(0x0092_bf50);
            let mut code = vec![0x68];
            code.extend(block.to_le_bytes()); // push block
            code.push(0xb8);
            code.extend(init.to_le_bytes()); // mov eax, rcdtParseInit
            code.extend([0xff, 0xd0, 0x83, 0xc4, 0x04]); // call eax; add esp, 4
            code.push(0xb8);
            code.extend(block.to_le_bytes()); // mov eax, block
            code.push(0xc3);
            let x = t.a.alloc_code(&code).unwrap();
            assert_eq!(t.b.alloc_code(&code).unwrap(), x);
            t.jump(mc::PARSE_NEW, x);
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - 0x340);
            t.check(
                "rcdtRsdLoad_009243c0",
                0x0092_43c0,
                Conv::Cdecl,
                &[model, path],
                |g| Out::U32(c::rsd_load(g, ebp, model, path)),
            );
            t.b_top = None;
            t.reset();
        }
    });
}

impl Twins {
    /// As [`Twins::copy_stub`] for a cdecl callee (`ecx` not logged),
    /// copying the string argument `arg` (0 the first) instead of the
    /// first.
    fn copy_stub_arg(&mut self, tag: u32, arg: u8, words: u32, pops: u32, ret: u32) -> u32 {
        let s = self.copy_stub(tag, words, pops, ret);
        // the stub's `mov esi, [esp+0x10]` is at a fixed place: patch its
        // displacement
        let disp_at = s + 5 + 3 + 5 + 6 + 3 + 3 + 3;
        assert_eq!(Mem::read::<u8>(&self.a, disp_at), 0x10);
        self.write(disp_at, &[0x10 + 4 * arg]);
        // `mov [eax+4], ecx` -> nops
        let ecx_at = s + 5 + 3 + 5 + 6;
        assert_eq!(Mem::read::<u8>(&self.a, ecx_at), 0x89);
        self.write(ecx_at, &[0x90; 3]);
        s
    }
}

// ---- batch 4: the rcui item lists, number windows, percent readout ----

#[test]
fn rcui_items_numbers_and_percent_match_the_original() {
    // item lists (empty, buttons, statics, other kinds); the percentage
    // below and above 50, NaN and out of range; number windows of every
    // value kind and of floats whose text has trailing zeros, a bare
    // point, more than ten characters (the real sprintf).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a401);
        for case in 0..40 {
            let n = case % 4;
            let mut words = Vec::new();
            for _ in 0..n {
                words.push([1, 2, 3, 7][rng.below(4) as usize]);
                words.extend((0..6).map(|_| rng.next()));
            }
            words.extend([0; 7]);
            let items = t.words(&words);
            let window = rng.next();
            t.stub_sym(md::ADD_BUTTON, 1, false, 7, 0, Ret::Val(rng.next()));
            t.stub_sym(md::ADD_STATIC, 2, false, 7, 0, Ret::Val(rng.next()));
            t.check(
                "rcuiAddItems",
                0x0093_7780,
                Conv::Cdecl,
                &[window, items],
                |g| {
                    let r = d::add_items(g, window, items);
                    // with no items the original leaves the caller's eax
                    if n == 0 { Out::None } else { Out::U32(r) }
                },
            );
            t.reset();

            let x = match case % 5 {
                0 => 49.9_f32.to_bits(),
                1 => 50.0_f32.to_bits(),
                2 => 0x7fc0_0001,
                3 => 3.0e9_f32.to_bits(),
                _ => float_word(&mut rng),
            };
            t.stub_sym(md::PRINT_SET_COLOR, 3, false, 4, 0, Ret::Val(rng.next()));
            t.stub_sym(md::PRINTF, 4, false, 2, 0, Ret::Val(rng.next()));
            t.check(
                "rcuiPercentPrint_00930ba0",
                0x0093_0ba0,
                Conv::Cdecl,
                &[x],
                |g| Out::U32(d::percent_print(g, x)),
            );
            t.reset();

            let num = t.alloc(&[0xa5; 0x40]);
            let kind = [0xb, 0xb, 0xb, 1, 2, 5, 9][case % 7];
            let fv = [1.5_f32, 100.0, 0.0, -2.25, 123_456_789.0, 1e-7, 7.0][rng.below(7) as usize];
            let value = t.words(&[if kind == 0xb {
                fv.to_bits()
            } else {
                rng.next()
            }]);
            t.stub_sym(md::RCUI_WINDOW_CREATE, 5, false, 9, 0, Ret::Val(rng.next()));
            t.stub_sym(md::WINDOW_SET_ARG, 6, false, 2, 0, Ret::Val(rng.next()));
            let (title, parent) = (rng.next(), rng.next());
            t.check(
                "rcuiNumberCreate",
                0x0093_c2e0,
                Conv::Cdecl,
                &[num, value, kind, title, parent],
                |g| {
                    d::number_create(g, num, value, kind, title, parent);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

#[test]
fn guide_packet_drivers_match_the_original() {
    // the object list (none, several), the selected one with either
    // highlight; the selected point among none or several, its x/y/w
    // (written by the stubbed vector callees) as floats.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a402);
        for case in 0..30 {
            let w = t.zeros(0x900);
            let n = [0_u32, 1, 3, 0xffff_ffff][case % 4];
            t.w32(w + 0x824, n);
            for k in 0..4 {
                let o = rng.next();
                t.w32(w + 0x870 + 4 * k, o);
            }
            let sel = rng.below(4);
            t.w32(w + 0x8b0, sel);
            let hl = rng.below(2);
            t.w32(w + 0x8b4, hl);
            let np = [0_u32, 1, 4, 0xffff_fffe][(case / 4) % 4];
            t.w32(w + 0x8cc, np);
            let sp = rng.below(5);
            t.w32(w + 0x8d0, sp);
            t.global(md::RCUI_WORK.sym, w);
            for round in 0..2 {
                for (k, f) in [
                    md::SET_REG,
                    md::RESTORE_REG,
                    md::PACKET_INIT,
                    md::PACKET_OPEN,
                    md::PACKET_CLOSE,
                    md::PACKET_DRAW,
                    md::GUIDE_OBJECT,
                    md::GUIDE_SELECTED_A,
                    md::GUIDE_SELECTED_B,
                    md::POINT_GET,
                    md::POINT_FIX,
                    md::POINT_MATRIX,
                    md::PACKET_RECT,
                    md::WRAP_92C600,
                ]
                .into_iter()
                .enumerate()
                {
                    let words = match f {
                        md::RESTORE_REG => 0,
                        md::POINT_GET => 4,
                        md::PACKET_RECT => 6,
                        md::SET_REG
                        | md::PACKET_INIT
                        | md::PACKET_CLOSE
                        | md::PACKET_DRAW
                        | md::POINT_FIX => 1,
                        _ => 2,
                    };
                    // the second round runs FUN_0092c4c0 itself
                    if round == 1 && f == md::GUIDE_SELECTED_B {
                        continue;
                    }
                    t.stub_sym(
                        f,
                        k as u32 + 1,
                        false,
                        words,
                        0,
                        Ret::Val(k as u32 * 0x1111),
                    );
                }
                // the vector callees write the point's x, y, z, w
                let vals: Vec<u32> = (0..4).map(|_| float_word(&mut rng)).collect();
                let mut code = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
                for (k, v) in vals.iter().enumerate() {
                    code.extend([0xc7, 0x40, 4 * k as u8]);
                    code.extend(v.to_le_bytes()); // mov dword [eax+4k], v
                }
                code.push(0xc3);
                let x = t.a.alloc_code(&code).unwrap();
                assert_eq!(t.b.alloc_code(&code).unwrap(), x);
                for f in [md::VEC_ORIGIN, md::APPLY_MATRIX] {
                    t.jump(f, x);
                }
                // the w divide logs its float; the register block is copied
                t.stub_sym(md::DIVIDE_W, 30, false, 3, 0, Ret::Val(0));
                let s = t.copy_stub_arg(31, 0, 12, 0, 0x77);
                t.jump(md::SET_REG, s);
                if round == 0 {
                    let ebp = frame_ebp(&t.a, 0);
                    t.write(ebp - 0x60, &[0; 0x60]);
                    t.b_top = Some(ebp - 0x60);
                    t.check(
                        "rcbgDrawGuidePackets_0092c130",
                        0x0092_c130,
                        Conv::Cdecl,
                        &[],
                        |g| Out::U32(d::draw_guide_packets(g, ebp)),
                    );
                    t.b_top = None;
                    t.reset();
                    t.global(md::RCUI_WORK.sym, w);
                    continue;
                }
                let (pkt, obj) = (rng.next(), rng.next());
                let ebp = frame_ebp(&t.a, 2);
                t.write(ebp - 0xb0, &[0; 0xb0]);
                t.b_top = Some(ebp - 0xb0);
                t.check(
                    "rcbgDrawGuidePoints_0092c4c0",
                    0x0092_c4c0,
                    Conv::Cdecl,
                    &[pkt, obj],
                    |g| Out::U32(d::draw_guide_points(g, ebp, pkt, obj)),
                );
                t.b_top = None;
            }
            t.reset();
        }
    });
}

#[test]
fn stage_name2_matches_the_original() {
    // the file missing; header lines before the first number; the number
    // found on the first, a later or no line (the real parse, isdigit and
    // strncpy); field and battle lists.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a403);
        for case in 0..30 {
            let mut text = String::new();
            // header lines (atoi reads them as 0: number 0 must not
            // match them)
            for _ in 0..=rng.below(2) {
                text.push_str("id,header\n");
            }
            let ids: Vec<u32> = (0..=rng.below(5)).map(|_| rng.below(20)).collect();
            for &id in &ids {
                text.push_str(&format!("{id},bsil{:04},x\n", rng.below(10_000)));
            }
            let mut bytes = text.into_bytes();
            bytes.push(0);
            let size = bytes.len() as u32;
            let data = t.alloc(&bytes);
            let loads = case % 7 != 3;
            // the load fills the chunk (begin, size) and returns 1, or 0
            let mut code = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
            code.extend([0xc7, 0x00]);
            code.extend(data.to_le_bytes()); // mov dword [eax], data
            code.extend([0xc7, 0x40, 0x04]);
            code.extend(size.to_le_bytes()); // mov dword [eax+4], size
            code.push(0xb8);
            code.extend(u32::from(loads).to_le_bytes()); // mov eax, loads
            code.push(0xc3);
            let x = t.a.alloc_code(&code).unwrap();
            assert_eq!(t.b.alloc_code(&code).unwrap(), x);
            t.jump(md::CHUNK_LOAD_FROM_FILE, x);
            // the releases free through the game heap (not set up on the
            // image): logged instead
            t.stub_sym(md::CHUNK_FINI, 1, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(md::PARSE_FINI, 2, false, 1, 0, Ret::Val(rng.next()));
            let out = t.alloc(&[0xa5; 0x10]);
            let no = match case % 4 {
                0 => 99,
                1 => 0,
                _ => ids[rng.below(ids.len() as u32) as usize],
            };
            let battle = rng.below(2);
            let ebp = frame_ebp(&t.a, 3);
            t.write(ebp - 0xa0, &[0; 0xa0]);
            t.b_top = Some(ebp - 0xb0);
            t.check(
                "rcuiGetStageName2",
                0x0093_7960,
                Conv::Cdecl,
                &[no, out, battle],
                |g| Out::U32(d::stage_name2(g, ebp, no, out, battle)),
            );
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn mouse_move_get_matches_the_original() {
    // either pointer missing; dx inside and outside the dead zone (-1..1),
    // dy anything.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a404);
        for case in 0..30 {
            let w = t.zeros(0xc00);
            for off in [0xbe8_u32, 0xbf4, 0xbf8] {
                let v = rng.below(400);
                t.w32(w + off, v);
            }
            t.global(md::RCUI_WORK.sym, w);
            let ox = t.r32(w + 0xbf4);
            let x = ox.wrapping_add([0, 1, 2, 0xffff_ffff, 0xffff_fffe, rng.next()][case % 6]);
            t.stub_sym(md::MOUSE_X, 1, false, 1, 0, Ret::Val(x));
            t.stub_sym(md::MOUSE_Y, 2, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(md::WINDOW_CREATE, 3, false, 6, 0, Ret::Val(rng.next()));
            t.stub_sym(md::WINDOW_UPDATE, 4, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(md::WINDOW_CLOSE_TREE, 5, false, 1, 0, Ret::Val(rng.next()));
            let dx = if case % 4 == 3 {
                0
            } else {
                t.alloc(&[0xa5; 4])
            };
            let dy = if case % 5 == 4 {
                0
            } else {
                t.alloc(&[0xa5; 4])
            };
            t.check(
                "rcuiMouseMoveGet",
                0x0093_c060,
                Conv::Cdecl,
                &[dx, dy],
                |g| Out::U32(d::mouse_move_get(g, dx, dy)),
            );
            t.reset();
        }
    });
}

// ---- round 5 batch 1 (ffx_prender::m5m_e) ----

/// Zeroes `n` bytes of both stacks below the call top.
fn r5_zero_stack(t: &mut Twins, n: u32) {
    let top = call_top(&t.a);
    let z = vec![0; n as usize];
    t.a.write(top - n, &z).unwrap();
    t.b.write(top - n, &z).unwrap();
}

/// Asserts both oracles hold the same bytes at `at..at + n`.
fn r5_same(t: &Twins, what: &str, at: u32, n: usize) {
    let mut x = vec![0; n];
    let mut y = vec![0; n];
    Mem::read_bytes(&t.a, at, &mut x);
    Mem::read_bytes(&t.b, at, &mut y);
    assert_eq!(x, y, "{what}: the stack at {at:#x}");
}

/// A recording stub over `sym` that then stores `v` through its
/// `arg`-th argument (a callee filling an out word).
fn r5_out_stub(t: &mut Twins, sym: u32, tag: u32, n: u32, ret: Ret, arg: u8, v: u32) {
    let s = t.stub_code(tag, false, n, 0, ret, false);
    // the stub ends `ret`: put the store in front of it
    let mut code = vec![0x51]; // push ecx
    code.extend([0x8b, 0x4c, 0x24, 4 + 4 + 4 * arg]); // mov ecx, [esp+8+4arg]
    code.extend([0xc7, 0x01]);
    code.extend(v.to_le_bytes()); // mov dword [ecx], v
    code.push(0x59); // pop ecx
    code.push(0xe9);
    let at = t.a.alloc_code(&[0; 16]).unwrap();
    assert_eq!(t.b.alloc_code(&[0; 16]).unwrap(), at);
    code.extend(s.wrapping_sub(at + code.len() as u32 + 4).to_le_bytes()); // jmp s
    t.write(at, &code);
    t.jump(sym, at);
}

#[test]
fn r5_small_rcui_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_e501);
        for case in 0..24_u32 {
            let w = t.zeros(0xc00);
            for off in [0x830_u32, 0x834, 0x838, 0x83c, 0x8c4, 0x8d0, 0x914, 0x918] {
                t.w32(w + off, rng.next());
            }
            t.w32(w + 0x8b0, rng.below(4));
            t.w32(w + 0x8b4, u32::from(rng.chance(2)));
            t.w32(w + 0x920, if rng.chance(3) { 0 } else { rng.below(40) });
            t.global(me::RCUI_WORK.sym, w);

            // the pre-memory window
            t.stub_sym(me::RCUI_WINDOW_CREATE, 1, false, 9, 0, Ret::Val(rng.next()));
            t.check("rcuiPreMemWinOpen", 0x0093_64a0, Conv::Cdecl, &[], |o| {
                Out::U32(e::pre_mem_win_open(o))
            });

            // its procedure: no system, no map, close, draw, others
            let sys = if case % 5 == 0 { 0 } else { t.zeros(8) };
            if sys != 0 && case % 5 != 1 {
                let map = t.zeros(0x20);
                let used = match case % 3 {
                    0 => rng.below(0x1_0000),
                    1 => 0x8000_0000 | rng.next(),
                    _ => rng.next(),
                };
                t.w32(map + 0x10, used);
                t.w32(sys, map);
            }
            t.global(me::YN_MAP_SYSTEM.sym, sys);
            for (k, sym) in [
                me::PRINT_SET_X,
                me::PRINT_SET_Y,
                me::GU_PRINTF,
                me::PERCENT_PRINT,
            ]
            .into_iter()
            .enumerate()
            {
                let n = if sym == me::GU_PRINTF { 2 } else { 1 };
                t.stub_sym(sym, 2 + k as u32, false, n, 0, Ret::Val(rng.next()));
            }
            let msg = match case % 4 {
                0 => 0x8003,
                1 | 2 => 0x8011,
                _ => rng.next(),
            };
            let args = [rng.next(), msg, rng.next()];
            t.check("rcuiPreMemProc", 0x0093_63f0, Conv::Cdecl, &args, |o| {
                Out::U32(e::pre_mem_proc(o, msg))
            });

            // the particle kick
            // (an absolute address: no relocation)
            let main = if case % 4 == 0 { 0 } else { rng.next() };
            t.patch(me::RCEF_DEBUG_MAIN, &main.to_le_bytes());
            t.stub_sym(
                me::RCEF_GET_CHR_HANDLE,
                6,
                false,
                1,
                0,
                Ret::Val(rng.next()),
            );
            t.stub_sym(
                me::CH_OPTPOS_GET_ELEMENT,
                7,
                false,
                3,
                0,
                Ret::Val(rng.next()),
            );
            t.stub_sym(
                me::RCEF_MAIN_KICK_PAR,
                8,
                false,
                10,
                0,
                Ret::Val(rng.next()),
            );
            let force = rng.next();
            t.check("rcuiParProcKick", 0x0093_6340, Conv::Cdecl, &[force], |o| {
                e::par_proc_kick(o, force);
                Out::None
            });

            // the full path (the real sprintf)
            for (off, s) in [
                (0x74_u32, &b"host:/map\0"[..]),
                (0x274, b"bs\0"),
                (0x27c, b"bsil00\0"),
            ] {
                t.write(w + off, s);
            }
            t.stub_sym(me::RCUI_GET_MAP_NO, 9, false, 1, 0, Ret::Val(rng.next()));
            t.check("rcuiMakeFullPath", 0x0093_7b20, Conv::Cdecl, &[], |o| {
                Out::U32(e::make_full_path(o))
            });

            // the window opener and the thunk
            t.stub_sym(me::GU_WINDOW_CREATE, 10, false, 6, 0, Ret::Val(rng.next()));
            t.stub_sym(me::GU_MOUSE_SET_WINDOW, 11, false, 1, 0, Ret::Val(11));
            t.stub_sym(
                me::GU_WINDOW_CLOSE_TREE,
                12,
                false,
                1,
                0,
                Ret::Val(rng.next()),
            );
            let (x, y) = (rng.next(), rng.next());
            t.check(
                "rcuiTreeWindowOpen_0093c1d0",
                0x0093_c1d0,
                Conv::Cdecl,
                &[x, y],
                |o| Out::U32(e::tree_window_open(o, x, y)),
            );
            t.stub_sym(me::ABMAP_CHENG_PANEL, 13, false, 2, 0, Ret::Val(rng.next()));
            t.check(
                "abmap_cheng_panel_thunk_00a45860",
                0x00a4_5860,
                Conv::Cdecl,
                &[x, y],
                |o| Out::U32(e::abmap_cheng_panel_thunk(o, x, y)),
            );
            t.reset();
        }
    });
}

#[test]
fn r5_rcbg_loads_and_saves_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_e502);
        for case in 0..24_u32 {
            // the scene loads
            t.stub_sym(me::SCENE_LOAD_TEXTURE, 1, false, 1, 0, Ret::Val(1));
            t.stub_sym(me::SCENE_LOAD_TEXT_SUB, 2, false, 1, 0, Ret::Val(2));
            t.stub_sym(me::SCENE_CHK_LAYERS, 3, false, 1, 0, Ret::Val(rng.next()));
            let scene = rng.next();
            t.check(
                "rcbgSceneLoadAll_0093aec0",
                0x0093_aec0,
                Conv::Cdecl,
                &[scene],
                |o| Out::U32(e::scene_load_all(o, scene)),
            );
            let map = t.zeros(0x10);
            let sys = t.words(&[map]);
            t.global(me::YN_MAP_SYSTEM.sym, sys);
            t.stub_sym(me::PRE_INIT, 4, false, 0, 0, Ret::Val(4));
            t.stub_sym(me::RC_ALLOC, 5, false, 3, 0, Ret::Val(rng.next()));
            t.stub_sym(me::SCENE_INIT, 6, false, 1, 0, Ret::Val(6));
            t.check("rcbgPreLoadText", 0x0093_ae60, Conv::Cdecl, &[], |o| {
                Out::U32(e::pre_load_text(o))
            });
            t.reset();

            // the FP texture loads: counts -1..3 (the real sprintf)
            let n = rng.below(5).wrapping_sub(1);
            t.stub_sym(me::DEL_OBJECTS, 1, false, 1, 0, Ret::Val(1));
            t.stub_sym(me::TRX_COUNT, 2, false, 2, 0, Ret::Val(n));
            t.stub_sym(me::TRX_NEW, 3, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(me::TRX_LOAD, 4, false, 4, 0, Ret::Val(4));
            t.stub_sym(me::PRE_ADD_OBJECT, 5, false, 1, 0, Ret::Val(rng.next()));
            let path = t.alloc(b"host:/fp/path\0");
            r5_zero_stack(t, 0x400);
            let ebp = frame_ebp(&t.a, 1);
            t.b_top = Some(ebp - 0x80);
            t.check(
                "rcbgFpTrxTexLoadAll_0093a890",
                0x0093_a890,
                Conv::Cdecl,
                &[path],
                |o| Out::U32(e::fp_trx_load_all(o, ebp - 0x14, path)),
            );
            r5_same(t, "rcbgFpTrxTexLoadAll_0093a890", ebp - 0x14, 0x10);

            // the stage-name path: no id, unknown, the two formats
            let id = if case % 5 == 0 { 0 } else { rng.next() };
            let flag = u32::from(rng.chance(2));
            let known = u32::from(case % 4 != 1);
            let name = u32::from_le_bytes([b'b', 0xe9, b's', b'l']);
            r5_out_stub(t, me::RCUI_GET_STAGE_NAME2, 6, 3, Ret::Val(known), 1, name);
            t.stub_sym(me::FP_TRX_LOAD_ALL, 7, false, 1, 0, Ret::Val(rng.next()));
            r5_zero_stack(t, 0x400);
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - 0x180);
            t.check(
                "rcbgFpLoadTextSub",
                0x0093_a800,
                Conv::Cdecl,
                &[id, flag],
                |o| Out::U32(e::fp_load_text_sub(o, ebp - 0xc, ebp - 0x10c, id, flag)),
            );
            r5_same(t, "rcbgFpLoadTextSub", ebp - 0x10c, 0x108);
            t.reset();

            // the binary save: packs or none
            let packs = rng.below(3).wrapping_sub(1);
            r5_out_stub(
                t,
                me::SAVE_CHK_PACK_LIST,
                1,
                2,
                Ret::Val(packs),
                1,
                rng.next(),
            );
            t.stub_sym(me::GUIDE_MAP_SAVE_SUB, 2, false, 3, 0, Ret::Val(rng.next()));
            t.stub_sym(me::SAVE_BIN_NORMAL, 3, false, 0, 0, Ret::Val(rng.next()));
            r5_zero_stack(t, 0x400);
            let ebp = frame_ebp(&t.a, 0);
            t.b_top = Some(ebp - 0x180);
            t.check("rcbgSaveBin_00938fd0", 0x0093_8fd0, Conv::Cdecl, &[], |o| {
                Out::U32(e::save_bin(o, ebp - 0x108))
            });
            r5_same(t, "rcbgSaveBin_00938fd0", ebp - 0x108, 4);
            t.reset();

            // the writes
            let seq: Vec<u32> = (0..12).map(|_| rng.next()).collect();
            let seq = t.seq(&seq);
            t.stub_sym(me::SCE_WRITE, 1, false, 3, 0, seq);
            let tex = t.zeros(0x40);
            t.w32(tex + 0x30, rng.below(64));
            t.w32(tex + 0x34, rng.below(64));
            t.w32(tex + 0x38, rng.next());
            let fd = rng.next();
            t.check("rcbgTrxSave", 0x0093_9950, Conv::Cdecl, &[tex, fd], |o| {
                Out::U32(e::trx_save(o, tex, fd))
            });
            t.stub_sym(me::RC_PAD16, 2, false, 1, 0, Ret::Val(rng.below(17)));
            let tag = t.alloc(b"YNPR");
            t.stub_sym(me::CHUNK_TAG, 3, false, 1, 0, Ret::Val(tag));
            let size = rng.next();
            r5_zero_stack(t, 0x400);
            let ebp = frame_ebp(&t.a, 2);
            let junk: Vec<u8> = (0..0x10).map(|_| rng.next() as u8).collect();
            t.a.write(ebp - 0x14, &junk).unwrap();
            t.b.write(ebp - 0x14, &junk).unwrap();
            t.b_top = Some(ebp - 0x80);
            let slots = first_slot(&t.a, 2);
            t.b.write(slots, &size.to_le_bytes()).unwrap();
            t.check(
                "rcbgSaveChunkHeader_00938f40",
                0x0093_8f40,
                Conv::Cdecl,
                &[size, fd],
                |o| Out::U32(e::save_chunk_header(o, slots, ebp - 0x14, fd)),
            );
            r5_same(t, "rcbgSaveChunkHeader_00938f40 frame", ebp - 0x14, 0x10);
            r5_same(t, "rcbgSaveChunkHeader_00938f40 slots", slots, 4);
            let obj = t.zeros(0x140);
            t.w32(obj + 0x14, rng.next());
            t.stub_sym(me::MESH_PAST_INDICES, 5, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(me::GET_PTR32, 6, false, 1, 0, Ret::Val(rng.next()));
            let slots = first_slot(&t.a, 2);
            t.b.write(slots, &obj.to_le_bytes()).unwrap();
            t.check(
                "rcbgSaveMesh_00938e70",
                0x0093_8e70,
                Conv::Cdecl,
                &[obj, fd],
                |o| Out::U32(e::save_mesh(o, slots, obj, fd)),
            );
            r5_same(t, "rcbgSaveMesh_00938e70 slots", slots, 4);
            t.b_top = None;
            t.reset();

            // the model by extension: none, each known one, another
            let ext: &[u8] = match case % 4 {
                0 => b"model",
                1 => b"model.a",
                2 => b"model.b",
                _ => b"x.y.c",
            };
            let mut s = ext.to_vec();
            s.push(0);
            let name = t.alloc(&s);
            let dot = s
                .iter()
                .rposition(|&c| c == b'.')
                .map_or(0, |i| name + i as u32);
            t.import_sym(me::STRRCHR_SLOT, 1, 2, 0, Ret::Val(dot));
            let hit = match case % 4 {
                1 => vec![0, 1],
                2 => vec![1, 0],
                _ => vec![1, 1],
            };
            let seq = t.seq(&hit);
            t.stub_sym(me::STRICMP, 2, false, 2, 0, seq);
            t.stub_sym(
                me::MODEL_CREATE_FROM_FILE,
                3,
                false,
                2,
                0,
                Ret::Val(rng.next()),
            );
            let a = rng.next();
            t.check(
                "rcbgModelLoadByExt_0093ad60",
                0x0093_ad60,
                Conv::Cdecl,
                &[a, name],
                |o| Out::U32(e::model_load_by_ext(o, a, name)),
            );
            t.reset();
        }
    });
}

#[test]
fn r5_file_count_and_gauge_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_e503);
        for case in 0..24_u32 {
            // the numbered files: names with and without the prefix and
            // digits (high bytes too)
            let n = rng.below(6);
            let names: Vec<u32> = (0..n)
                .map(|_| {
                    let mut s = b"ab".to_vec();
                    for _ in 0..rng.below(4) {
                        s.push(match rng.below(4) {
                            0 => b'x',
                            1 => 0xb5,
                            _ => b'0' + rng.below(10) as u8,
                        });
                    }
                    s.push(0);
                    t.alloc(&s)
                })
                .collect();
            let table = if names.is_empty() {
                t.zeros(4)
            } else {
                t.words(&names)
            };
            let count = if case % 7 == 0 { 0xffff_ffff } else { n };
            let mut code = vec![0x51, 0x8b, 0x4c, 0x24, 0x08]; // push ecx; mov ecx, [esp+8]
            code.extend([0xc7, 0x01]);
            code.extend(count.to_le_bytes()); // mov dword [ecx], count
            code.extend([0xc7, 0x41, 0x08]);
            code.extend(table.to_le_bytes()); // mov dword [ecx+8], names
            code.push(0x59); // pop ecx
            let s = t.stub_code(1, false, 2, 0, Ret::Val(1), false);
            let at = t.a.alloc_code(&[0; 32]).unwrap();
            assert_eq!(t.b.alloc_code(&[0; 32]).unwrap(), at);
            code.push(0xe9);
            code.extend(s.wrapping_sub(at + code.len() as u32 + 4).to_le_bytes());
            t.write(at, &code);
            t.jump(me::MAKE_FILE_LIST, at);
            let hits: Vec<u32> = (0..8).map(|_| u32::from(rng.chance(3))).collect();
            let seq = t.seq(&hits);
            t.import_sym(me::STRNCMP_SLOT, 2, 3, 0, seq);
            let prefix = t.alloc(b"ab\0");
            let digits = match case % 4 {
                0 => 0,
                1 => 0xffff_ffff,
                _ => rng.below(4),
            };
            let pattern = rng.next();
            r5_zero_stack(t, 0x400);
            let ebp = frame_ebp(&t.a, 3);
            t.b_top = Some(ebp - 0x80);
            t.check(
                "rcbgCountNumberedFiles_0093acb0",
                0x0093_acb0,
                Conv::Cdecl,
                &[pattern, prefix, digits],
                |o| {
                    Out::U32(e::count_numbered_files(
                        o,
                        ebp - 0x10,
                        pattern,
                        prefix,
                        digits,
                    ))
                },
            );
            r5_same(t, "rcbgCountNumberedFiles_0093acb0", ebp - 0x10, 12);
            t.b_top = None;
            t.reset();

            // the gauge: values below, at and past 100
            for (k, sym) in [
                me::PACKET_BEGIN_TAG,
                me::SET_PRIM,
                me::SET_RGBAQ,
                me::SET_XYZF2,
            ]
            .into_iter()
            .enumerate()
            {
                let n = match sym {
                    me::SET_RGBAQ => 4,
                    me::SET_XYZF2 => 2,
                    _ => 1,
                };
                t.stub_sym(sym, 1 + k as u32, false, n, 0, Ret::Val(k as u32));
            }
            t.stub_sym(me::PACKET_END_TAG, 5, false, 0, 0, Ret::Val(rng.next()));
            let value = match case % 3 {
                0 => rng.below(101),
                1 => 100 + rng.below(3),
                _ => rng.next(),
            };
            let args = [rng.next(), rng.below(600), rng.below(400), value];
            t.check(
                "rcuiGaugeDraw_0093c730",
                0x0093_c730,
                Conv::Cdecl,
                &args,
                |o| Out::U32(e::gauge_draw(o, args[0], args[1], args[2], args[3])),
            );
            t.reset();
        }
    });
}

// ---- round 5 batch 2 (ffx_prender::m5m_f) ----

#[test]
fn r5_rcbg_texts_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_f501);
        for case in 0..16_u32 {
            // the FP texture count: names base0_00.tm2.. present in a row
            // or with gaps, no reply, an empty list
            let reply = t.zeros(0x1_0000);
            t.global(mf::PRIM_2D_PACKET.sym, reply);
            // (the name buffer is 16 bytes: a short base, as the game's "tex")
            let base = t.alloc(b"tx\0");
            let path = t.alloc(b"host:/fp\0");
            let present: Vec<u32> = (0..rng.below(6)).filter(|_| !rng.chance(4)).collect();
            let names: Vec<u32> = present
                .iter()
                .map(|k| {
                    let s = format!("tx{k}_00.tm2\0");
                    t.alloc(s.as_bytes())
                })
                .collect();
            let table = if names.is_empty() {
                t.zeros(4)
            } else {
                t.words(&names)
            };
            let n = names.len() as u32;
            let mut code = vec![0x51, 0x8b, 0x4c, 0x24, 0x08]; // push ecx; mov ecx, [esp+8]
            code.extend([0xc7, 0x01]);
            code.extend(n.to_le_bytes()); // mov dword [ecx], n
            code.extend([0xc7, 0x41, 0x08]);
            code.extend(table.to_le_bytes()); // mov dword [ecx+8], names
            code.push(0x59); // pop ecx
            let s = t.stub_code(1, false, 2, 0, Ret::Val(1), false);
            let at = t.a.alloc_code(&[0; 32]).unwrap();
            assert_eq!(t.b.alloc_code(&[0; 32]).unwrap(), at);
            code.push(0xe9);
            code.extend(s.wrapping_sub(at + code.len() as u32 + 4).to_le_bytes());
            t.write(at, &code);
            t.jump(mf::MAKE_FILE_LIST, at);
            let answer = if case % 4 == 0 {
                0xffff_ffff
            } else {
                rng.below(3)
            };
            t.stub_sym(mf::SYSTEM_ON_HOST, 2, false, 2, 0, Ret::Val(answer));
            r5_zero_stack(t, 0x400);
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - 0x180);
            t.check(
                "rcbgFpTrxTexLoadTextGetTrxCnt",
                0x0093_a910,
                Conv::Cdecl,
                &[path, base],
                |o| Out::U32(f::trx_count(o, ebp - 0x128, path, base)),
            );
            r5_same(t, "rcbgFpTrxTexLoadTextGetTrxCnt", ebp - 0x128, 0x124);
            t.b_top = None;
            t.reset();

            // the guide-map model: no id, no stage, the load failing, no
            // map, loaded (vertex words cleared)
            let id = if case % 6 == 0 { 0 } else { rng.next() };
            let known = u32::from(case % 6 != 1);
            let stage = u32::from_le_bytes([b'b', b's', 0xe9, b'l']);
            r5_out_stub(t, mf::GET_STAGE_NAME, 1, 2, Ret::Val(known), 1, stage);
            let model = if case % 5 == 2 { 0 } else { t.zeros(0xb0) };
            if model != 0 {
                for k in (0x80..0xb0).step_by(4) {
                    t.w32(model + k, rng.next());
                }
                let mesh = t.zeros(0x80);
                t.w32(mesh + 8, 0x20);
                t.write(mesh + 0x12, &(rng.below(8) as i16 - 1).to_le_bytes());
                for k in (0x20..0x80).step_by(4) {
                    t.w32(mesh + k, rng.next());
                }
                t.w32(model + 0x14, mesh);
                t.w32(model + 0x10, rng.next());
            }
            t.stub_sym(mf::MODEL_NEW, 2, false, 0, 0, Ret::Val(model));
            let loaded = if case % 3 == 0 || model == 0 {
                0
            } else {
                rng.next()
            };
            t.stub_sym(mf::MODEL_LOAD_BY_EXT, 3, false, 2, 0, Ret::Val(loaded));
            t.stub_sym(mf::MODEL_DELETE, 4, false, 1, 0, Ret::Val(4));
            let guide = t.zeros(0x20);
            let map = t.words(&[0, 0, guide]);
            let sys = match case % 4 {
                0 => 0,
                1 => t.words(&[0]),
                _ => t.words(&[map]),
            };
            t.global(mf::YN_MAP_SYSTEM.sym, sys);
            r5_zero_stack(t, 0x400);
            let ebp = frame_ebp(&t.a, 1);
            t.b_top = Some(ebp - 0x180);
            t.check(
                "rcbgGuideMapLoadTextSub",
                0x0093_aa60,
                Conv::Cdecl,
                &[id],
                |o| Out::U32(f::guide_map_load_text_sub(o, ebp - 0xc, ebp - 0x10c, id)),
            );
            r5_same(t, "rcbgGuideMapLoadTextSub", ebp - 0x10c, 0x108);
            t.b_top = None;
            t.reset();

            // the image save
            let seq: Vec<u32> = (0..16).map(|_| rng.next()).collect();
            let seq = t.seq(&seq);
            t.stub_sym(mf::SCE_WRITE, 1, false, 3, 0, seq);
            let tex = t.zeros(0xa0);
            for k in (0..0xa0).step_by(4) {
                t.w32(tex + k, rng.next());
            }
            for off in [0x50, 0x54, 0x90, 0x94] {
                t.w32(tex + off, rng.below(64));
            }
            let fd = rng.next();
            r5_zero_stack(t, 0x400);
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - 0x80);
            t.check(
                "rcbgTrxSave2_009399f0",
                0x0093_99f0,
                Conv::Cdecl,
                &[tex, fd],
                |o| Out::U32(f::trx_save2(o, ebp - 0x14, tex, fd)),
            );
            r5_same(t, "rcbgTrxSave2_009399f0", ebp - 0x14, 0x10);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 3 (ffx_prender::m5m_g) ----

/// A recording stub over `sym` that returns the float at `cell` in ST0.
fn r5_float_stub(t: &mut Twins, sym: u32, tag: u32, n: u32, cell: u32) {
    let s = t.stub_code(tag, false, n, 0, Ret::Val(0), false);
    let mut code = vec![0xd9, 0x05];
    code.extend(cell.to_le_bytes()); // fld dword [cell]
    code.push(0xe9);
    let at = t.a.alloc_code(&[0; 16]).unwrap();
    assert_eq!(t.b.alloc_code(&[0; 16]).unwrap(), at);
    code.extend(s.wrapping_sub(at + code.len() as u32 + 4).to_le_bytes()); // jmp s
    t.write(at, &code);
    t.jump(sym, at);
}

#[test]
fn r5_scene_config_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_f601);
        for case in 0..24_u32 {
            // the object keys: every keyword, unknown ones, the end
            let obj = t.zeros(0xc0);
            let n = rng.below(6);
            let mut keys: Vec<u32> = (0..n)
                .map(|_| match rng.below(12) {
                    10 => 0xffff_ffff,
                    11 => 9,
                    k => k,
                })
                .collect();
            if case % 4 == 0 {
                keys.insert(0, 6);
            }
            keys.push(9);
            let mut ends: Vec<u32> = (0..n).map(|_| 0).collect();
            ends.insert(0, u32::from(case % 7 == 0));
            ends.push(1);
            let seq = t.seq(&keys);
            t.stub_sym(mg::GET_KEYWORD, 1, false, 2, 0, seq);
            let seq = t.seq(&ends);
            t.stub_sym(mg::IS_END, 2, false, 1, 0, seq);
            let ints: Vec<u32> = (0..8).map(|_| 0x8000_0000 ^ rng.word()).collect();
            let seq = t.seq(&ints);
            t.stub_sym(mg::GET_INT, 3, false, 1, 0, seq);
            let cell = t.words(&[float_word(&mut rng)]);
            r5_float_stub(t, mg::GET_FLOAT, 4, 1, cell);
            t.stub_sym(mg::NEXT_TOKEN, 5, false, 1, 0, Ret::Val(5));
            t.stub_sym(mg::ANIME_LOAD_CONFIG, 6, false, 2, 0, Ret::Val(6));
            t.stub_sym(mg::PAL_LOAD_CONFIG, 7, false, 2, 0, Ret::Val(7));
            let p = rng.next();
            t.check(
                "rcbgSceneLoadTextObjConfig_0093ab80",
                0x0093_ab80,
                Conv::Cdecl,
                &[obj, p],
                |o| Out::U32(g::obj_config(o, obj, p)),
            );
            t.reset();

            // the file: not loaded, sections 0, 1 (up to the object count)
            // and others
            let objs = t.zeros(0xc0 * 3);
            let scene = t.zeros(0x10);
            t.w32(scene + 4, if case % 3 == 0 { 3 } else { rng.below(4) });
            t.w32(scene + 8, objs);
            let n = rng.below(6);
            let keys: Vec<u32> = (0..n)
                .map(|_| if case % 3 == 0 { 1 } else { rng.below(3) })
                .collect();
            let mut ends: Vec<u32> = vec![0; n as usize];
            ends.push(1);
            let seq = t.seq(&keys);
            t.stub_sym(mg::GET_KEYWORD, 1, false, 2, 0, seq);
            let seq = t.seq(&ends);
            t.stub_sym(mg::IS_END, 2, false, 1, 0, seq);
            for (k, sym) in [
                mg::CHUNK_INIT,
                mg::PARSE_INIT,
                mg::PARSE_SET_MODE,
                mg::PARSE_SET,
                mg::CHUNK_FINI,
                mg::PARSE_FINI,
                mg::SCENE_CONFIG_KEYS,
                mg::OBJ_CONFIG,
            ]
            .into_iter()
            .enumerate()
            {
                let words = match sym {
                    mg::CHUNK_INIT | mg::PARSE_INIT | mg::CHUNK_FINI | mg::PARSE_FINI => 1,
                    _ => 2,
                };
                t.stub_sym(sym, 3 + k as u32, false, words, 0, Ret::Val(k as u32));
            }
            let loaded = u32::from(case % 4 != 0);
            t.stub_sym(mg::CHUNK_LOAD, 11, false, 2, 0, Ret::Val(loaded));
            let name = rng.next();
            r5_zero_stack(t, 0x400);
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - 0x100);
            t.check(
                "rcbgSceneLoadTextSubConfig",
                0x0093_b590,
                Conv::Cdecl,
                &[scene, name],
                |o| Out::U32(g::load_text_sub_config(o, ebp - 0x98, scene, name)),
            );
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 4 (ffx_prender::m5m_h) ----

#[test]
fn r5_vram_write_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_f701);
        for case in 0..36_u32 {
            let w = t.zeros(0xc00);
            // the format index: one of the 13 the frame table has (a larger
            // one reads past it, the original's stack)
            let format = rng.below(13);
            // a negative row and height (a positive size; a negative size
            // is a fault in the original's memset, and the one class that
            // halves, format 20, is past the frame table)
            let (row, height) = if case % 3 == 0 {
                (
                    0_u32.wrapping_sub(1 + rng.below(6)),
                    0_u32.wrapping_sub(1 + rng.below(6)),
                )
            } else {
                (rng.below(24), rng.below(24))
            };
            t.w32(w + 0xafc, row);
            t.w32(w + 0xb00, height);
            t.w32(w + 0xaec, format);
            t.w32(w + 0xae8, rng.word());
            t.w32(w + 0xae4, rng.word());
            t.global(mh::RCUI_WORK.sym, w);
            let packet = t.zeros(0x400 + 0x900);
            t.global(mh::PRIM_2D_PACKET.sym, packet);
            t.import_sym(mh::RAND_SLOT, 1, 0, 0, Ret::Val(rng.next() & 0x7fff));
            t.stub_sym(mh::DMA_GET_CHAN, 2, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mh::FLUSH_CACHE, 3, false, 1, 0, Ret::Val(3));
            t.stub_sym(mh::DMA_SEND, 4, false, 2, 0, Ret::Val(4));
            t.stub_sym(mh::DMA_SYNC, 5, false, 3, 0, Ret::Val(rng.next()));
            r5_zero_stack(t, 0x400);
            let ebp = frame_ebp(&t.a, 0);
            t.b_top = Some(ebp - 0x80);
            t.check("rcuiVramWrite", 0x0093_7190, Conv::Cdecl, &[], |o| {
                Out::U32(h::vram_write(o, ebp - 0x38))
            });
            r5_same(t, "rcuiVramWrite", ebp - 0x38, 0x34);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 5 (ffx_prender::m5m_i) ----

#[test]
fn r5_guide_map_conv_sub_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_f801);
        for case in 0..24_u32 {
            let id = if case % 7 == 0 { 0 } else { rng.next() };
            let known = u32::from(case % 7 != 1);
            let stage = u32::from_le_bytes([b'b', b's', 0xe9, b'l']);
            r5_out_stub(t, mi::GET_STAGE_NAME, 1, 2, Ret::Val(known), 1, stage);
            let fd = if case % 5 == 2 { 0 } else { rng.next() | 1 };
            t.stub_sym(mi::SCE_OPEN, 2, false, 2, 0, Ret::Val(fd));
            t.stub_sym(mi::SCE_CLOSE, 3, false, 1, 0, Ret::Val(3));
            let model = if case % 6 == 3 { 0 } else { t.zeros(0xb0) };
            if model != 0 {
                let mesh = t.zeros(0x80);
                t.w32(mesh + 8, 0x20);
                t.write(mesh + 0x12, &(rng.below(8) as i16 - 1).to_le_bytes());
                for k in (0x20..0x80).step_by(4) {
                    t.w32(mesh + k, rng.next());
                }
                for k in (0x80..0xb0).step_by(4) {
                    t.w32(model + k, rng.next());
                }
                t.w32(model + 0x14, mesh);
                t.w32(model + 0x10, rng.next());
            }
            t.stub_sym(mi::MODEL_NEW, 4, false, 0, 0, Ret::Val(model));
            let loaded = if case % 3 == 0 || model == 0 {
                0
            } else {
                rng.next()
            };
            t.stub_sym(mi::MODEL_LOAD_BY_EXT, 5, false, 2, 0, Ret::Val(loaded));
            t.stub_sym(mi::MODEL_DELETE, 6, false, 1, 0, Ret::Val(6));
            let map = t.words(&[0, 0, 0]);
            let sys = match case % 4 {
                0 => 0,
                1 => t.words(&[0]),
                _ => t.words(&[map]),
            };
            t.global(mi::YN_MAP_SYSTEM.sym, sys);
            let sub = if case % 2 == 0 { 0 } else { t.alloc(b"sub\0") };
            r5_zero_stack(t, 0x400);
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - 0x180);
            t.check(
                "rcbgGuideMapConvSub",
                0x0093_8080,
                Conv::Cdecl,
                &[id, sub],
                |o| Out::U32(gi::guide_map_conv_sub(o, ebp - 0xc, ebp - 0x10c, id, sub)),
            );
            r5_same(t, "rcbgGuideMapConvSub", ebp - 0x10c, 0x108);
            t.b_top = None;
            t.reset();
        }
    });
}
