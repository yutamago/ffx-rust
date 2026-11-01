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
use ffx_prender::m5m_j::{self as mj, bridge as gj};
use ffx_prender::m5m_k::{self as mk, bridge as gk};
use ffx_prender::m5m_l::{self as ml, bridge as gl};
use ffx_prender::m5m_m::{self as mm, bridge as gm};
use ffx_prender::m5m_n::{self as mn, bridge as gn};
use ffx_prender::m5m_o::{self as mo, bridge as go};
use ffx_prender::m5m_p::{self as mp, bridge as gp};
use ffx_prender::m5m_q::{self as mq, bridge as gq};
use ffx_prender::m5m_r::{self as mr, bridge as gr};
use ffx_prender::m5m_s::{self as ms, bridge as gs};
use ffx_prender::m5m_t::{self as mt, bridge as gt};
use ffx_prender::m5m_u::{self as mu, bridge as gu};
use ffx_prender::m5m_v::{self as mv, bridge as gv};

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
    fn call_intrinsic2(&mut self, addr: u32, x: f64, y: f64) -> f32 {
        // the trampoline 8 bytes past the poison
        let at = self.1 + 8;
        let (x, y) = (x.to_bits(), y.to_bits());
        let words = [x as u32, (x >> 32) as u32, y as u32, (y >> 32) as u32, addr];
        let r = self
            .0
            .call(at, Conv::Cdecl, &words)
            .unwrap_or_else(|e| panic!("intrinsic {addr:#x}: {e}"));
        let v = r
            .st0_f32()
            .unwrap_or_else(|| panic!("intrinsic {addr:#x}: no ST0"));
        self.0.call(at + 13, Conv::Cdecl, &[]).unwrap();
        v
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
        // after the poison: `fld qword [esp+4]; fld qword [esp+0xc];
        // call [esp+0x14]; ret` for Host::call_intrinsic2 (operands in ST1
        // and ST0; the cave function is 2438 bytes long), then `fstp st0;
        // ret`
        t.write(
            at + 8,
            &[
                0xdd, 0x44, 0x24, 0x04, 0xdd, 0x44, 0x24, 0x0c, 0xff, 0x54, 0x24, 0x14, 0xc3, 0xdd,
                0xd8, 0xc3,
            ],
        );
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
    // 17 bytes with the jump's offset
    let at = t.a.alloc_code(&[0; 20]).unwrap();
    assert_eq!(t.b.alloc_code(&[0; 20]).unwrap(), at);
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

// ---- round 5 batch 6 (ffx_prender::m5m_j) ----

#[test]
fn r5_num_window_create_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_f901);
        for case in 0..32_u32 {
            let w: Vec<u32> = (0..8).map(|_| rng.next()).collect();
            let win = t.words(&w);
            let v = match case % 4 {
                0 => (rng.below(200_000) as f32 / 100.0).to_bits(),
                1 => (-(rng.below(5000) as f32) / 7.0).to_bits(),
                2 => 0x7fc0_0000 | rng.below(16),
                _ => rng.next() & 0x7f7f_ffff,
            };
            let value = t.words(&[v]);
            t.stub_sym(mj::WINDOW_CREATE, 1, false, 9, 0, Ret::Val(rng.next()));
            t.stub_sym(mj::WINDOW_SET_ARG, 2, false, 2, 0, Ret::Val(rng.next()));
            // `modf`: the whole part through the third argument, the
            // fraction in ST0 (any, so the clamps and conversions all run)
            let fraction: f64 = match rng.below(5) {
                0 => 0.995,
                1 => -0.5,
                2 => 0.0,
                3 => 2.5,
                _ => f64::from(rng.below(2000)) / 1999.0,
            };
            let whole: f64 = match rng.below(6) {
                0 => -f64::from(rng.below(50)),
                1 => 999.0,
                2 => 1000.0,
                3 => 1.0e12,
                4 => f64::NAN,
                _ => f64::from(rng.below(1500)),
            };
            let cell = t.alloc(&fraction.to_le_bytes());
            let s = t.stub_code(3, false, 3, 0, Ret::Val(0), false);
            let wb = whole.to_bits();
            let mut code = vec![0x51, 0x8b, 0x4c, 0x24, 0x10]; // push ecx; mov ecx, [esp+0x10]
            code.extend([0xc7, 0x01]);
            code.extend(((wb & 0xffff_ffff) as u32).to_le_bytes()); // mov dword [ecx], lo
            code.extend([0xc7, 0x41, 0x04]);
            code.extend(((wb >> 32) as u32).to_le_bytes()); // mov dword [ecx+4], hi
            code.push(0x59); // pop ecx
            code.extend([0xdd, 0x05]);
            code.extend(cell.to_le_bytes()); // fld qword [cell]
            code.push(0xe9);
            let at = t.a.alloc_code(&[0; 40]).unwrap();
            assert_eq!(t.b.alloc_code(&[0; 40]).unwrap(), at);
            code.extend(s.wrapping_sub(at + code.len() as u32 + 4).to_le_bytes()); // jmp s
            t.write(at, &code);
            t.patch(t.rebase(mj::MODF_SLOT), &at.to_le_bytes());
            let args = [win, value, rng.next(), rng.next()];
            r5_zero_stack(t, 0x200);
            let ebp = frame_ebp(&t.a, 4);
            t.b_top = Some(ebp - mj::FRAME - 0x40);
            t.check(
                "rcuiNumWindowCreate_0093c670",
                0x0093_c670,
                Conv::Cdecl,
                &args,
                |o| Out::U32(gj::num_window_create(o, ebp, win, value, args[2], args[3])),
            );
            r5_same(t, "rcuiNumWindowCreate_0093c670", ebp - 8, 8);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 7 (ffx_prender::m5m_k) ----

#[test]
fn r5_guide_save_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_fa01);
        for case in 0..24_u32 {
            t.stub_sym(mk::SCE_WRITE, 1, false, 3, 0, Ret::Val(rng.next()));
            t.stub_sym(mk::TRX_SAVE, 2, false, 2, 0, Ret::Val(rng.next()));

            // one map: no transfers, a multiple of 4, or a tail to pad
            let image = t.zeros(0x40);
            t.w32(image + 0x30, rng.below(64));
            t.w32(image + 0x34, rng.below(64));
            t.w32(image + 0x38, rng.next());
            let m = t.zeros(0xc0);
            let n = match case % 4 {
                0 => 0,
                1 => 4 * (1 + rng.below(3)),
                2 => 0x8000_0000 | rng.next(),
                _ => 1 + rng.below(15),
            };
            t.w32(m + 0x98, n);
            t.w32(m + 0x9c, rng.next());
            t.w32(m + 0xa0, rng.next());
            t.w32(m + 0xbc, image);
            let fd = rng.next();
            r5_zero_stack(t, 0x200);
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - mk::MAP_FRAME - 0x40);
            t.check(
                "rcuiGuideSaveMap_00938d20",
                0x0093_8d20,
                Conv::Cdecl,
                &[m, fd],
                |o| Out::U32(gk::save_map(o, ebp, m, fd)),
            );
            r5_same(t, "rcuiGuideSaveMap_00938d20", ebp - 0x14, 0x10);
            t.b_top = None;
            t.reset();

            // the guide: its maps through the map save (stubbed), the
            // padding in its header
            t.stub_sym(mk::SCE_WRITE, 1, false, 3, 0, Ret::Val(rng.next()));
            t.stub_sym(mk::SAVE_MAP, 3, false, 2, 0, Ret::Val(rng.next()));
            let size = match rng.below(3) {
                // negative: the units round toward zero (no pad, so the
                // header keeps them)
                _ if case % 6 == 0 => 0x8000_0003,
                0 => 0x7fff_fff0 | rng.below(16),
                1 => 16 * rng.below(100),
                _ => rng.below(0x10000),
            };
            t.stub_sym(mk::GUIDE_SAVE_SIZE, 4, false, 1, 0, Ret::Val(size));
            let pad = match case % 5 {
                _ if case % 6 == 0 => 0,
                0 => 0,
                _ => 1 + rng.below(16),
            };
            t.stub_sym(mk::PAD16, 5, false, 1, 0, Ret::Val(pad));
            let tag = t.words(&[rng.next()]);
            t.stub_sym(mk::CHUNK_TAG, 6, false, 1, 0, Ret::Val(tag));
            let g = t.zeros(0xb0);
            let maps = match case % 3 {
                0 => 0,
                1 => 0x8000_0000 | rng.next(),
                _ => 1 + rng.below(4),
            };
            t.w32(g + 4, maps);
            t.w32(g + 8, rng.next());
            let fd = rng.next();
            r5_zero_stack(t, 0x200);
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - mk::SAVE_FRAME - 0x40);
            t.check(
                "rcuiGuideSave_009397f0",
                0x0093_97f0,
                Conv::Cdecl,
                &[g, fd],
                |o| Out::U32(gk::save(o, ebp, g, fd)),
            );
            r5_same(t, "rcuiGuideSave_009397f0", ebp - 0x30, 0x2c);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 8 (ffx_prender::m5m_l) ----

/// Machine code at a fresh address in both oracles, jumped to from `sym`.
fn r5_code(t: &mut Twins, sym: u32, code: &[u8]) {
    let at = t.a.alloc_code(&vec![0; code.len()]).unwrap();
    assert_eq!(t.b.alloc_code(&vec![0; code.len()]).unwrap(), at);
    t.write(at, code);
    t.jump(sym, at);
}

#[test]
fn r5_map_id_from_csv_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_fb01);
        for case in 0..24_u32 {
            // the table: 16-byte lines (text at +0, the line's int at +8),
            // header lines first, then ids and names
            // two names that differ in the sixth character only
            let names: [&[u8; 6]; 5] = [b"bsil00", b"lmyt01", b"kino03", b"nagi00", b"bsil01"];
            let headers = rng.below(3);
            let rows = rng.below(5);
            let mut lines = vec![];
            for _ in 0..headers {
                let mut l = *b"map,id\0\0\0\0\0\0\0\0\0\0";
                l[8..12].copy_from_slice(&rng.next().to_le_bytes());
                lines.push(l);
            }
            for _ in 0..rows {
                let mut l = [0_u8; 16];
                l[0] = b'0' + rng.below(10) as u8;
                l[1..6].copy_from_slice(&names[rng.below(5) as usize][1..]);
                if rng.chance(2) {
                    l[..6].copy_from_slice(names[rng.below(5) as usize]);
                    l[0] = b'1';
                }
                l[8..12].copy_from_slice(&rng.next().to_le_bytes());
                lines.push(l);
            }
            if case % 3 == 0 {
                // a near miss (the sixth character) before the match
                for text in [b"1sil01", b"1sil00"] {
                    let mut l = [0_u8; 16];
                    l[..6].copy_from_slice(text);
                    l[8..12].copy_from_slice(&rng.next().to_le_bytes());
                    lines.push(l);
                }
            }
            // a digit line to stop the skip, at the end at least
            let mut l = *b"9xxxxx\0\0\0\0\0\0\0\0\0\0";
            l[8..12].copy_from_slice(&rng.next().to_le_bytes());
            lines.push(l);
            let bytes: Vec<u8> = lines.iter().flatten().copied().collect();
            let table = t.alloc(&bytes);
            let end = table + 16 * lines.len() as u32 - 16 * u32::from(case % 4 == 1);
            let mut name = names[rng.below(5) as usize].to_vec();
            name[0] = if rng.chance(2) { b'1' } else { b'b' };
            if case % 3 == 0 {
                name[..6].copy_from_slice(b"1sil00");
            }
            name.extend(b"_x\0");
            let name = t.alloc(&name);

            t.stub_sym(ml::CHUNK_INIT, 1, false, 1, 0, Ret::Val(1));
            t.stub_sym(
                ml::CHUNK_LOAD,
                2,
                false,
                2,
                0,
                Ret::Val(u32::from(case % 5 != 0)),
            );
            t.stub_sym(ml::CHUNK_FINI, 3, false, 1, 0, Ret::Val(3));
            t.stub_sym(ml::PARSE_INIT, 4, false, 1, 0, Ret::Val(4));
            t.stub_sym(ml::PARSE_SET_MODE, 5, false, 2, 0, Ret::Val(5));
            t.stub_sym(ml::PARSE_ADD_SEPARATORS, 6, false, 2, 0, Ret::Val(6));
            t.stub_sym(ml::PARSE_FINI, 7, false, 1, 0, Ret::Val(7));
            // the set puts the cursor on the table, a line is 16 bytes, the
            // end is `end`, the int the line's +8
            let mut set = vec![0x8b, 0x4c, 0x24, 0x04, 0xc7, 0x41, 0x04]; // mov ecx, [esp+4]; mov dword [ecx+4], table
            set.extend(table.to_le_bytes());
            set.push(0xc3);
            r5_code(t, ml::PARSE_SET, &set);
            r5_code(
                t,
                ml::PARSE_ADVANCE_LINE,
                &[0x8b, 0x4c, 0x24, 0x04, 0x83, 0x41, 0x04, 0x10, 0xc3],
            );
            let mut is_end = vec![0x8b, 0x4c, 0x24, 0x04, 0x31, 0xc0, 0x81, 0x79, 0x04];
            is_end.extend(end.to_le_bytes());
            is_end.extend([0x0f, 0x93, 0xc0, 0xc3]); // setae al; ret
            r5_code(t, ml::PARSE_IS_END, &is_end);
            r5_code(
                t,
                ml::PARSE_GET_INT,
                &[
                    0x8b, 0x4c, 0x24, 0x04, 0x8b, 0x41, 0x04, 0x8b, 0x40, 0x08, 0xc3,
                ],
            );
            r5_zero_stack(t, 0x300);
            let ebp = frame_ebp(&t.a, 1);
            t.b_top = Some(ebp - ml::FRAME - 0x40);
            t.check(
                "rcuiMapIdFromCsv_009377e0",
                0x0093_77e0,
                Conv::Cdecl,
                &[name],
                |o| Out::U32(gl::map_id_from_csv(o, ebp, name)),
            );
            r5_same(t, "rcuiMapIdFromCsv_009377e0", ebp - 0x90, 8);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 9 (ffx_prender::m5m_m) ----

#[test]
fn r5_save_bin_normal_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_fc01);
        for case in 0..24_u32 {
            let work = t.zeros(0x200);
            let dirs: [&[u8]; 3] = [b"/ffx/proj/map/bsil", b"", b"/a/b"];
            t.write(work + 0x174, dirs[(case % 3) as usize]);
            t.global(mm::RCUI_WORK.sym, work);
            // the map system: its first word points at the guides (a count
            // and their base), whose first word the chunk header gets
            let guides = t.zeros(0x10);
            let count = match case % 4 {
                0 => 0,
                1 => 0x8000_0000 | rng.next(),
                _ => 1 + rng.below(3),
            };
            t.w32(guides, count);
            t.w32(guides + 4, rng.next());
            let sys = t.words(&[guides]);
            t.global(mm::YN_MAP_SYSTEM.sym, sys);
            let fd = if case % 5 == 2 {
                u32::MAX
            } else {
                rng.below(100)
            };
            t.stub_sym(mm::SCE_OPEN, 1, false, 2, 0, Ret::Val(fd));
            t.stub_sym(mm::SCE_WRITE, 2, false, 3, 0, Ret::Val(rng.next()));
            t.stub_sym(mm::SCE_CLOSE, 3, false, 1, 0, Ret::Val(rng.next()));
            let tags = [t.words(&[rng.next()]), t.words(&[rng.next()])];
            let seq = t.seq(&tags);
            t.stub_sym(mm::CHUNK_TAG, 4, false, 1, 0, seq);
            t.stub_sym(mm::SAVE_CHUNK_HEADER, 5, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(mm::GUIDE_SAVE, 6, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(mm::SYSTEM_ON_HOST, 7, false, 2, 0, Ret::Val(rng.next()));
            // rcPrint: one word for the failure text, three otherwise
            let s1 = t.stub_code(8, false, 1, 0, Ret::Val(rng.next()), false);
            let s3 = t.stub_code(9, false, 3, 0, Ret::Val(rng.next()), false);
            let at = t.a.alloc_code(&[0; 24]).unwrap();
            assert_eq!(t.b.alloc_code(&[0; 24]).unwrap(), at);
            let mut c = vec![0x81, 0x7c, 0x24, 0x04];
            c.extend(t.rebase(mm::FAILED_TEXT).to_le_bytes()); // cmp [esp+4], failed
            c.extend([0x0f, 0x84]);
            c.extend(s1.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // je s1
            c.push(0xe9);
            c.extend(s3.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // jmp s3
            t.write(at, &c);
            t.jump(mm::RC_PRINT, at);
            r5_zero_stack(t, 0x600);
            let ebp = frame_ebp(&t.a, 0);
            // junk under the chunk header (the save clears it)
            t.write(ebp - 0x314, &[0x5a; 0x10]);
            t.b_top = Some(ebp - mm::FRAME - 0x40);
            t.check("rcbgSaveBinNormal", 0x0093_9040, Conv::Cdecl, &[], |o| {
                Out::U32(gm::save_bin_normal(o, ebp))
            });
            r5_same(t, "rcbgSaveBinNormal", ebp - 0x318, 0x314);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 10 (ffx_prender::m5m_n) ----

#[test]
fn r5_load_texture_normal_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_fd01);
        for case in 0..24_u32 {
            let work = t.zeros(0x300);
            t.write(work + 0x174, b"/ffx/map");
            let maps: [&[u8]; 3] = [b"bsil00", b"", b"kino03"];
            t.write(work + 0x27c, maps[(case % 3) as usize]);
            t.global(mn::RCUI_WORK.sym, work);
            let i = match case % 4 {
                0 => 0x19,
                1 => 0x1a,
                _ => rng.below(40),
            };
            let entries = t.zeros(0xc0 * (i as usize + 1));
            let e = entries + 0xc0 * i;
            for off in [4_u32, 8, 0xc, 0x10] {
                t.w32(e + off, rng.below(300));
            }
            let scene = t.zeros(0x10);
            t.w32(scene + 8, entries);
            let tm2 = t.zeros(0x50);
            t.w32(tm2 + 0x44, rng.next());
            let w: Vec<u32> = (0..0x100).map(|_| rng.next()).collect();
            let clut = t.words(&w);
            let pal = t.zeros(0x400);
            t.stub_sym(mn::TM2_NEW, 1, false, 0, 0, Ret::Val(tm2));
            t.stub_sym(mn::TM2_LOAD, 2, false, 2, 0, Ret::Val(rng.next()));
            // the bounds: the width the original reads back
            r5_out_stub(
                t,
                mn::TM2_OPAQUE_BOUNDS,
                3,
                5,
                Ret::Val(3),
                3,
                rng.below(300),
            );
            r5_out_stub(t, mn::TEX_LOG2_SIZE, 4, 5, Ret::Val(4), 4, rng.below(16));
            t.stub_sym(mn::ANIME_SET_DATA_CNT, 5, false, 2, 0, Ret::Val(5));
            t.stub_sym(mn::TRX_NEW, 6, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(mn::TRX_SET_REGS_ALLOC, 7, false, 9, 0, Ret::Val(7));
            t.stub_sym(mn::TRX_GET_ENTRY_DATA, 8, false, 6, 0, Ret::Val(rng.next()));
            t.stub_sym(mn::TM2_COPY_RECT, 9, false, 6, 0, Ret::Val(9));
            t.stub_sym(mn::PAL_ALLOC, 10, false, 2, 0, Ret::Val(10));
            t.stub_sym(mn::GET_PTR32, 11, false, 1, 0, Ret::Val(clut));
            t.stub_sym(mn::PAL_GET_ENTRY_DATA, 12, false, 2, 0, Ret::Val(pal));
            t.stub_sym(mn::TM2_DELETE, 13, false, 1, 0, Ret::Val(rng.next()));
            r5_zero_stack(t, 0x400);
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - mn::FRAME - 0x40);
            t.check(
                "rcbgSceneLoadTextureNormal",
                0x0093_bd10,
                Conv::Cdecl,
                &[scene, i],
                |o| Out::U32(gn::load_texture_normal(o, ebp, scene, i)),
            );
            r5_same(t, "rcbgSceneLoadTextureNormal", ebp - 0x118, 0x114);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 11 (ffx_prender::m5m_o) ----

#[test]
fn r5_scene_load_text_sub_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_fe01);
        for case in 0..24_u32 {
            let work = t.zeros(0x300);
            t.write(work + 0x174, b"/ffx/proj/map");
            let maps: [&[u8]; 3] = [b"bsil00", b"", b"kino03"];
            t.write(work + 0x27c, maps[(case % 3) as usize]);
            t.global(mo::RCUI_WORK.sym, work);
            // the config: the .2d or the .txt
            let configs = t.seq(&[u32::from(case % 2 == 0), rng.next()]);
            t.stub_sym(mo::LOAD_CONFIG, 1, false, 2, 0, configs);
            // the camera text: the first, second, third or none loads
            let k = case % 4;
            let loads: Vec<u32> = (0..3).map(|j| u32::from(j == k)).collect();
            let loads = t.seq(&loads);
            t.stub_sym(mo::CHUNK_LOAD, 2, false, 2, 0, loads);
            t.stub_sym(mo::LOAD_CAMERA, 3, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(mo::CHUNK_INIT, 4, false, 1, 0, Ret::Val(4));
            t.stub_sym(mo::CHUNK_FINI, 5, false, 1, 0, Ret::Val(5));
            t.stub_sym(mo::PARSE_INIT, 6, false, 1, 0, Ret::Val(6));
            t.stub_sym(mo::PARSE_SET_MODE, 7, false, 2, 0, Ret::Val(7));
            t.stub_sym(mo::PARSE_SET, 8, false, 2, 0, Ret::Val(8));
            t.stub_sym(mo::PARSE_FINI, 9, false, 1, 0, Ret::Val(rng.next()));
            let scene = rng.next();
            r5_zero_stack(t, 0x400);
            let ebp = frame_ebp(&t.a, 1);
            t.b_top = Some(ebp - mo::FRAME - 0x40);
            t.check(
                "rcbgSceneLoadTextSub",
                0x0093_b3e0,
                Conv::Cdecl,
                &[scene],
                |o| Out::U32(go::load_text_sub(o, ebp, scene)),
            );
            r5_same(t, "rcbgSceneLoadTextSub", ebp - 0x198, 0x194);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 12 (ffx_prender::m5m_p) ----

/// A dispatcher over `sym`: calls whose first argument is `first` go to
/// stub `a`, the others to stub `b`.
fn r5_dispatch(t: &mut Twins, sym: u32, first: u32, a: u32, b: u32) {
    let at = t.a.alloc_code(&[0; 24]).unwrap();
    assert_eq!(t.b.alloc_code(&[0; 24]).unwrap(), at);
    let mut c = vec![0x81, 0x7c, 0x24, 0x04];
    c.extend(first.to_le_bytes()); // cmp [esp+4], first
    c.extend([0x0f, 0x84]);
    c.extend(a.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // je a
    c.push(0xe9);
    c.extend(b.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // jmp b
    t.write(at, &c);
    t.jump(sym, at);
}

#[test]
fn r5_trx_tex_load_from_file_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_ff01);
        for case in 0..24_u32 {
            let trx = t.zeros(0x80);
            let dir = t.alloc(b"/ffx/map/tex\0");
            let name = t.alloc(if case % 2 == 0 { b"anm\0" } else { b"w_\0\0" });
            let digits = 1 + rng.below(3);
            let n = match case % 5 {
                0 => 0,
                1 => 0x8000_0000 | rng.next(),
                _ => 1 + rng.below(4),
            };
            t.stub_sym(mp::COUNT_FILES, 1, false, 3, 0, Ret::Val(n));
            t.stub_sym(mp::TM2_INIT, 2, false, 1, 0, Ret::Val(2));
            let loads: Vec<u32> = (0..4)
                .map(|k| u32::from(k > 0 || case % 3 != 0) & u32::from(!rng.chance(5)))
                .collect();
            let loads = t.seq(&loads);
            t.stub_sym(mp::TM2_LOAD, 3, false, 2, 0, loads);
            t.stub_sym(mp::TRX_ALLOC_ENTRIES, 4, false, 5, 0, Ret::Val(4));
            let data = t.seq(&[rng.next(), rng.next(), rng.next(), rng.next()]);
            let s6 = t.stub_code(5, false, 6, 0, data, false);
            let dst = t.zeros(0x400);
            let s2 = t.stub_code(6, false, 2, 0, Ret::Val(dst), false);
            r5_dispatch(t, mp::TRX_GET_ENTRY_DATA, trx + 0x20, s6, s2);
            t.stub_sym(mp::TM2_COPY_RECT, 7, false, 6, 0, Ret::Val(7));
            let w: Vec<u32> = (0..0x100).map(|_| rng.next()).collect();
            let clut = t.words(&w);
            t.stub_sym(mp::GET_PTR32, 8, false, 1, 0, Ret::Val(clut));
            t.stub_sym(mp::TM2_FINI, 9, false, 1, 0, Ret::Val(rng.next()));
            let p1 = t.stub_code(10, false, 1, 0, Ret::Val(10), false);
            let p3 = t.stub_code(11, false, 3, 0, Ret::Val(11), false);
            r5_dispatch(t, mp::RC_PRINT, t.rebase(mp::NOT_FOUND_TEXT), p1, p3);
            let args = [trx, dir, name, digits];
            r5_zero_stack(t, 0x600);
            let ebp = frame_ebp(&t.a, 4);
            // the TIM2 record's size words and CLUT (as a load leaves them)
            t.w32(ebp - 0x230, rng.next());
            t.w32(ebp - 0x22c, rng.next());
            t.w32(ebp - 0x210, rng.next());
            t.b_top = Some(ebp - mp::FRAME - 0x40);
            t.check(
                "rcbgTrxTexLoadFromFile",
                0x0093_beb0,
                Conv::Cdecl,
                &args,
                |o| Out::U32(gp::tex_load_from_file(o, ebp, trx, dir, name, digits)),
            );
            r5_same(t, "rcbgTrxTexLoadFromFile", ebp - 0x25c, 0x258);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 13 (ffx_prender::m5m_q) ----

#[test]
fn r5_load_anime_rect_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_f0a1);
        for case in 0..24_u32 {
            let work = t.zeros(0x300);
            t.write(work + 0x174, b"/ffx/map");
            let maps: [&[u8]; 3] = [b"bsil00", b"", b"kino03"];
            t.write(work + 0x27c, maps[(case % 3) as usize]);
            t.global(mq::RCUI_WORK.sym, work);
            let i = match case % 4 {
                0 => 0x19,
                1 => 0x1a,
                _ => rng.below(40),
            };
            let tm2 = if case % 7 == 3 { 0 } else { rng.next() };
            t.stub_sym(mq::TM2_NEW, 1, false, 0, 0, Ret::Val(tm2));
            t.stub_sym(
                mq::TM2_LOAD,
                2,
                false,
                2,
                0,
                Ret::Val(u32::from(case % 5 != 2)),
            );
            let rect = t.zeros(0x10);
            let (x, y, w, hh) = (rect, rect + 4, rect + 8, rect + 12);
            t.w32(w, rng.below(64));
            // the bounds: x as the callee leaves it, odd at times
            r5_out_stub(
                t,
                mq::TM2_KEY_BOUNDS,
                3,
                5,
                Ret::Val(rng.next()),
                1,
                rng.below(64),
            );
            t.stub_sym(mq::TM2_DELETE, 4, false, 1, 0, Ret::Val(rng.next()));
            let p1 = t.stub_code(5, false, 1, 0, Ret::Val(rng.next()), false);
            let p3 = t.stub_code(6, false, 3, 0, Ret::Val(rng.next()), false);
            r5_dispatch(t, mq::RC_PRINT, t.rebase(mq::NOT_FOUND_TEXT), p1, p3);
            let args = [i, x, y, w, hh];
            r5_zero_stack(t, 0x400);
            let ebp = frame_ebp(&t.a, 5);
            t.b_top = Some(ebp - mq::FRAME - 0x40);
            t.check(
                "rcbgSceneLoadTextureAnimeRect",
                0x0093_bbc0,
                Conv::Cdecl,
                &args,
                |o| Out::U32(gq::load_anime_rect(o, ebp, i, x, y, w, hh)),
            );
            r5_same(t, "rcbgSceneLoadTextureAnimeRect", ebp - 0x11c, 0x118);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 14 (ffx_prender::m5m_r) ----

#[test]
fn r5_load_texture_anime_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_f0b1);
        for case in 0..24_u32 {
            let work = t.zeros(0x300);
            t.write(work + 0x174, b"/ffx/map");
            let maps: [&[u8]; 3] = [b"bsil00", b"", b"kino03"];
            t.write(work + 0x27c, maps[(case % 3) as usize]);
            t.global(mr::RCUI_WORK.sym, work);
            let i = match case % 4 {
                0 => 0x19,
                1 => 0x1a,
                _ => rng.below(40),
            };
            let entries = t.zeros(0xc0 * (i as usize + 1));
            let e = entries + 0xc0 * i;
            let (w, hh) = (rng.below(300), rng.below(300));
            t.w32(e + 4, w);
            t.w32(e + 8, hh);
            t.w32(e + 0xc, rng.below(300));
            t.w32(e + 0x10, rng.below(300));
            let records = t.zeros(0x20);
            t.w32(e + 0x90 + 0xc, records);
            let scene = t.zeros(0x10);
            t.w32(scene + 8, entries);
            // the image: the rectangle's size at times
            let tm2 = if case % 7 == 3 { 0 } else { t.zeros(0x50) };
            if tm2 != 0 {
                let same = case % 2 == 0;
                let iw = if same { w } else { rng.below(300) };
                let ih = if same || rng.chance(2) {
                    hh
                } else {
                    rng.below(300)
                };
                t.write(tm2 + 0x24, &(iw as u16).to_le_bytes());
                t.write(tm2 + 0x26, &(ih as u16).to_le_bytes());
                t.w32(tm2 + 0x44, rng.next());
            }
            let n = match case % 5 {
                0 => 0,
                1 => 0x8000_0000 | rng.next(),
                _ => 1 + rng.below(4),
            };
            t.stub_sym(mr::ANIME_RECT, 1, false, 5, 0, Ret::Val(1));
            t.stub_sym(mr::COUNT_FILES, 2, false, 3, 0, Ret::Val(n));
            r5_out_stub(t, mr::TEX_LOG2_SIZE, 3, 5, Ret::Val(3), 4, rng.below(16));
            t.stub_sym(mr::ANIME_SET_DATA_CNT, 4, false, 2, 0, Ret::Val(4));
            t.stub_sym(mr::TRX_NEW, 5, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(mr::TRX_SET_REGS_ALLOC, 6, false, 9, 0, Ret::Val(6));
            t.stub_sym(mr::PAL_ALLOC, 7, false, 2, 0, Ret::Val(7));
            t.stub_sym(mr::TM2_NEW, 8, false, 0, 0, Ret::Val(tm2));
            // (no image: every load fails, as a null one would)
            let loads: Vec<u32> = (0..4)
                .map(|_| u32::from(tm2 != 0 && !rng.chance(4)))
                .collect();
            let loads = t.seq(&loads);
            t.stub_sym(mr::TM2_LOAD, 9, false, 2, 0, loads);
            let wds: Vec<u32> = (0..0x100).map(|_| rng.next()).collect();
            let clut = t.words(&wds);
            t.stub_sym(mr::GET_PTR32, 10, false, 1, 0, Ret::Val(clut));
            let dst = t.zeros(0x400);
            t.stub_sym(mr::PAL_GET_ENTRY_DATA, 11, false, 2, 0, Ret::Val(dst));
            t.stub_sym(
                mr::TRX_GET_ENTRY_DATA,
                12,
                false,
                6,
                0,
                Ret::Val(rng.next()),
            );
            t.stub_sym(mr::TM2_COPY_RECT, 13, false, 6, 0, Ret::Val(rng.next()));
            t.stub_sym(mr::TM2_DELETE, 14, false, 1, 0, Ret::Val(rng.next()));
            let p1 = t.stub_code(15, false, 1, 0, Ret::Val(rng.next()), false);
            let p3 = t.stub_code(16, false, 3, 0, Ret::Val(rng.next()), false);
            r5_dispatch(t, mr::RC_PRINT, t.rebase(mr::NOT_FOUND_TEXT), p1, p3);
            r5_zero_stack(t, 0x400);
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - mr::FRAME - 0x40);
            t.check(
                "rcbgSceneLoadTextureAnime",
                0x0093_b930,
                Conv::Cdecl,
                &[scene, i],
                |o| Out::U32(gr::load_texture_anime(o, ebp, scene, i)),
            );
            r5_same(t, "rcbgSceneLoadTextureAnime", ebp - 0x138, 0x134);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 15 (ffx_prender::m5m_s) ----

#[test]
fn r5_chk_pack_list_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_f0c1);
        for case in 0..24_u32 {
            let work = t.zeros(0x300);
            t.write(work + 0x174, b"/ffx/proj/map/bsil");
            let map: &[u8; 6] = if case % 2 == 0 { b"bsil01" } else { b"kino03" };
            t.write(work + 0x27c, map);
            t.global(ms::RCUI_WORK.sym, work);
            // the table: 16-byte lines, `[packNN]` sections and map names
            // (two of them share only 3 characters with a map)
            let names: [&[u8; 6]; 7] = [
                b"bsil00", b"bsil01", b"bsil12", b"kino03", b"lmyt05", b"bsix02", b"kinz04",
            ];
            let mut lines: Vec<[u8; 16]> = vec![];
            for _ in 0..=rng.below(3) {
                let mut l = *b"[pack00]\0\0\0\0\0\0\0\0";
                l[5] = b'0' + rng.below(10) as u8;
                l[6] = b'0' + rng.below(10) as u8;
                lines.push(l);
                for _ in 0..rng.below(4) {
                    let mut l = [0_u8; 16];
                    l[..6].copy_from_slice(names[rng.below(7) as usize]);
                    lines.push(l);
                }
            }
            let bytes: Vec<u8> = lines.iter().flatten().copied().collect();
            let table = t.alloc(&bytes);
            let end = table + 16 * lines.len() as u32 - 16 * u32::from(case % 7 == 6);
            let maps = t.zeros(0x20);
            let pack = t.words(&[rng.next()]);
            t.stub_sym(ms::CHUNK_INIT, 1, false, 1, 0, Ret::Val(1));
            t.stub_sym(
                ms::CHUNK_LOAD,
                2,
                false,
                2,
                0,
                Ret::Val(u32::from(case % 5 != 0)),
            );
            t.stub_sym(ms::CHUNK_FINI, 3, false, 1, 0, Ret::Val(3));
            t.stub_sym(ms::PARSE_INIT, 4, false, 1, 0, Ret::Val(4));
            t.stub_sym(ms::PARSE_FINI, 5, false, 1, 0, Ret::Val(5));
            let mut set = vec![0x8b, 0x4c, 0x24, 0x04, 0xc7, 0x41, 0x04]; // mov ecx, [esp+4]; mov dword [ecx+4], table
            set.extend(table.to_le_bytes());
            set.push(0xc3);
            r5_code(t, ms::PARSE_SET, &set);
            r5_code(
                t,
                ms::PARSE_ADVANCE_LINE,
                &[0x8b, 0x4c, 0x24, 0x04, 0x83, 0x41, 0x04, 0x10, 0xc3],
            );
            let mut is_end = vec![0x8b, 0x4c, 0x24, 0x04, 0x31, 0xc0, 0x81, 0x79, 0x04];
            is_end.extend(end.to_le_bytes());
            is_end.extend([0x0f, 0x93, 0xc0, 0xc3]); // setae al; ret
            r5_code(t, ms::PARSE_IS_END, &is_end);
            r5_zero_stack(t, 0x400);
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - ms::FRAME - 0x40);
            t.check(
                "rcbgSaveChkPackList",
                0x0093_9480,
                Conv::Cdecl,
                &[maps, pack],
                |o| Out::U32(gs::chk_pack_list(o, ebp, maps, pack)),
            );
            r5_same(t, "rcbgSaveChkPackList", ebp - 0x1a4, 0x1a0);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 16 (ffx_prender::m5m_t) ----

#[test]
fn r5_scene_load_texture_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_f0d1);
        for case in 0..24_u32 {
            let work = t.zeros(0x300);
            t.write(work + 0x174, b"/ffx/map");
            t.write(
                work + 0x27c,
                if case % 4 == 3 {
                    &b"kino"[..]
                } else {
                    &b"bsil00"[..]
                },
            );
            t.global(mt::RCUI_WORK.sym, work);
            // the directory's files: plain and animated layers, others
            let pool: [&[u8]; 12] = [
                b"bsil00a.tm2\0",
                b"bsil00b\0",
                b"bsil00c.tm2\0",
                b"bsil00_a.tm2\0",
                b"bsil00_c\0",
                b"bsil00\0",
                b"bsil00A.tm2\0",
                b"kino03d.tm2\0",
                b"bsil00_\0",
                // the last letter, a non-letter after '_', a near miss of
                // the plain name
                b"bsil00z.tm2\0",
                b"bsil00_~.tm2\0",
                b"bsil00b.tmp\0",
            ];
            let count = if case % 6 == 5 { 0 } else { 1 + rng.below(6) };
            let ptrs: Vec<u32> = (0..count)
                .map(|_| {
                    let s = pool[rng.below(12) as usize];
                    t.alloc(s)
                })
                .collect();
            let names = t.words(&ptrs);
            let s = t.stub_code(1, false, 2, 0, Ret::Val(1), false);
            let mut c = vec![0x51, 0x8b, 0x4c, 0x24, 0x08]; // push ecx; mov ecx, [esp+8]
            c.extend([0xc7, 0x01]);
            c.extend(count.to_le_bytes()); // mov dword [ecx], count
            c.extend([0xc7, 0x41, 0x08]);
            c.extend(names.to_le_bytes()); // mov dword [ecx+8], names
            c.push(0x59); // pop ecx
            c.push(0xe9);
            let at = t.a.alloc_code(&[0; 32]).unwrap();
            assert_eq!(t.b.alloc_code(&[0; 32]).unwrap(), at);
            c.extend(s.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // jmp s
            t.write(at, &c);
            t.jump(mt::MAKE_FILE_LIST, at);
            let layers = t.zeros(0xc0 * 60);
            t.stub_sym(mt::RC_ALLOC, 2, false, 3, 0, Ret::Val(layers));
            t.stub_sym(mt::LAYER_INIT, 3, false, 1, 0, Ret::Val(3));
            t.stub_sym(mt::LOAD_NORMAL, 4, false, 2, 0, Ret::Val(4));
            t.stub_sym(mt::LOAD_ANIME, 5, false, 2, 0, Ret::Val(5));
            t.stub_sym(mt::LAYER_CALC_CENTER, 6, false, 1, 0, Ret::Val(rng.next()));
            let scene = t.zeros(0x10);
            r5_zero_stack(t, 0x400);
            let ebp = frame_ebp(&t.a, 1);
            t.b_top = Some(ebp - mt::FRAME - 0x40);
            t.check(
                "rcbgSceneLoadTexture",
                0x0093_b6b0,
                Conv::Cdecl,
                &[scene],
                |o| Out::U32(gt::load_texture(o, ebp, scene)),
            );
            r5_same(t, "rcbgSceneLoadTexture", ebp - 0x118, 0x114);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 17 (ffx_prender::m5m_u) ----

#[test]
fn r5_guide_map_save_sub_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_f0e1);
        for case in 0..24_u32 {
            let work = t.zeros(0x300);
            t.write(work + 0x174, b"/ffx/proj/map/bs01");
            t.write(work + 0x27c, b"bsil01");
            t.global(mu::RCUI_WORK.sym, work);
            // the map system: none, an empty one, or its scene
            let sys = match case % 3 {
                0 => 0,
                1 => t.words(&[0]),
                _ => {
                    let first = t.words(&[0, 0, rng.next()]);
                    t.words(&[first])
                }
            };
            t.global(mu::YN_MAP_SYSTEM.sym, sys);
            let n = match case % 4 {
                0 => 0,
                1 => 0x8000_0000 | rng.next(),
                _ => 1 + rng.below(4),
            };
            let bytes: Vec<u8> = (0..4).map(|_| rng.next() as u8).collect();
            let maps = t.alloc(&bytes);
            t.stub_sym(mu::SET_NUMBER, 1, false, 1, 0, Ret::Val(1));
            t.stub_sym(mu::SCE_OPEN, 2, false, 2, 0, Ret::Val(rng.below(100)));
            t.stub_sym(mu::SCE_WRITE, 3, false, 3, 0, Ret::Val(rng.next()));
            t.stub_sym(mu::SCE_CLOSE, 4, false, 1, 0, Ret::Val(rng.next()));
            let tags = [t.words(&[rng.next()]), t.words(&[rng.next()])];
            let seq = t.seq(&tags);
            t.stub_sym(mu::CHUNK_TAG, 5, false, 1, 0, seq);
            t.stub_sym(mu::SAVE_CHUNK_HEADER, 6, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(mu::SCENE_INIT, 7, false, 1, 0, Ret::Val(7));
            t.stub_sym(mu::SCENE_LOAD_ALL, 8, false, 1, 0, Ret::Val(rng.next()));
            // the guide save sees the map name's digits (+4..+8 captured
            // per call)
            let cap = t.zeros(0x40);
            let s = t.stub_code(9, false, 2, 0, Ret::Val(9), false);
            let mut c = vec![0x51, 0x50, 0x8b, 0x0d]; // push ecx; push eax; mov ecx, [cap]
            c.extend(cap.to_le_bytes());
            c.push(0xa1); // mov eax, [name + 4]
            c.extend((work + 0x27c + 4).to_le_bytes());
            c.extend([0x89, 0x04, 0x8d]); // mov [ecx*4 + cap + 4], eax
            c.extend((cap + 4).to_le_bytes());
            c.extend([0xff, 0x05]); // inc dword [cap]
            c.extend(cap.to_le_bytes());
            c.extend([0x58, 0x59, 0xe9]); // pop eax; pop ecx; jmp s
            let at = t.a.alloc_code(&[0; 40]).unwrap();
            assert_eq!(t.b.alloc_code(&[0; 40]).unwrap(), at);
            c.extend(s.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
            t.write(at, &c);
            t.jump(mu::GUIDE_SAVE, at);
            t.stub_sym(mu::SCENE_FINI, 10, false, 1, 0, Ret::Val(10));
            t.stub_sym(mu::SYSTEM_ON_HOST, 11, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(mu::RC_PRINT, 12, false, 3, 0, Ret::Val(rng.next()));
            let args = [n, maps, rng.next()];
            r5_zero_stack(t, 0x800);
            let ebp = frame_ebp(&t.a, 3);
            // junk under the chunk header (the save clears it)
            t.write(ebp - 0x41c, &[0x5a; 0x10]);
            t.b_top = Some(ebp - mu::FRAME - 0x40);
            t.check(
                "rcbgGuideMapSaveSub",
                0x0093_91d0,
                Conv::Cdecl,
                &args,
                |o| Out::U32(gu::guide_map_save_sub(o, ebp, n, maps, args[2])),
            );
            r5_same(t, "rcbgGuideMapSaveSub", ebp - 0x534, 0x530);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 18 (ffx_prender::m5m_v) ----

#[test]
fn r5_guide_convert_all_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_f0f1);
        for case in 0..24_u32 {
            // the table: 16-byte lines (text at +0, the line's int at +8),
            // header lines first, then ids
            let mut lines = vec![];
            for _ in 0..rng.below(3) {
                let mut l = *b"map,id\0\0\0\0\0\0\0\0\0\0";
                l[8..12].copy_from_slice(&rng.next().to_le_bytes());
                lines.push(l);
            }
            for _ in 0..=rng.below(4) {
                let mut l = *b"0xxxxx\0\0\0\0\0\0\0\0\0\0";
                l[0] = b'0' + rng.below(10) as u8;
                l[8..12].copy_from_slice(&rng.next().to_le_bytes());
                lines.push(l);
            }
            let bytes: Vec<u8> = lines.iter().flatten().copied().collect();
            let table = t.alloc(&bytes);
            let end = table + 16 * lines.len() as u32 - 16 * u32::from(case % 4 == 1);
            t.stub_sym(mv::CHUNK_INIT, 1, false, 1, 0, Ret::Val(1));
            t.stub_sym(
                mv::CHUNK_LOAD,
                2,
                false,
                2,
                0,
                Ret::Val(u32::from(case % 5 != 0)),
            );
            t.stub_sym(mv::CHUNK_FINI, 3, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mv::PARSE_INIT, 4, false, 1, 0, Ret::Val(4));
            t.stub_sym(mv::PARSE_SET_MODE, 5, false, 2, 0, Ret::Val(5));
            t.stub_sym(mv::PARSE_ADD_SEPARATORS, 6, false, 2, 0, Ret::Val(6));
            t.stub_sym(mv::PARSE_FINI, 7, false, 1, 0, Ret::Val(7));
            t.stub_sym(mv::GUIDE_MAP_CONV, 8, false, 1, 0, Ret::Val(8));
            t.stub_sym(mv::RC_PRINT, 9, false, 1, 0, Ret::Val(9));
            let mut set = vec![0x8b, 0x4c, 0x24, 0x04, 0xc7, 0x41, 0x04]; // mov ecx, [esp+4]; mov dword [ecx+4], table
            set.extend(table.to_le_bytes());
            set.push(0xc3);
            r5_code(t, mv::PARSE_SET, &set);
            r5_code(
                t,
                mv::PARSE_ADVANCE_LINE,
                &[0x8b, 0x4c, 0x24, 0x04, 0x83, 0x41, 0x04, 0x10, 0xc3],
            );
            let mut is_end = vec![0x8b, 0x4c, 0x24, 0x04, 0x31, 0xc0, 0x81, 0x79, 0x04];
            is_end.extend(end.to_le_bytes());
            is_end.extend([0x0f, 0x93, 0xc0, 0xc3]); // setae al; ret
            r5_code(t, mv::PARSE_IS_END, &is_end);
            r5_code(
                t,
                mv::PARSE_GET_INT,
                &[
                    0x8b, 0x4c, 0x24, 0x04, 0x8b, 0x41, 0x04, 0x8b, 0x40, 0x08, 0xc3,
                ],
            );
            r5_zero_stack(t, 0x300);
            let ebp = frame_ebp(&t.a, 0);
            t.b_top = Some(ebp - mv::FRAME - 0x40);
            t.check(
                "rcuiGuideProcConvertAll",
                0x0093_4250,
                Conv::Cdecl,
                &[],
                |o| Out::U32(gv::convert_all(o, ebp)),
            );
            r5_same(t, "rcuiGuideProcConvertAll", ebp - 0x90, 8);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 6 (left6) batch 1 (ffx_prender::l6_map) ----

use ffx_prender::l6_map::{self as l6, bridge as l6m};

/// `Some(eax)` compares `eax`; `None` (the caller's `eax`) nothing.
fn l6_out(v: Option<u32>) -> Out {
    v.map_or(Out::None, Out::U32)
}

/// A message: the four the procedures handle, the others the table
/// sends to the default, and some outside it.
fn l6_msg(rng: &mut Rng) -> u32 {
    match rng.below(8) {
        0 | 1 => 0x8001,
        2 => 0x8003,
        3 | 4 => 0x8006,
        5 => 0x8011,
        6 => 0x8001 + rng.below(0x11),
        _ => rng.word(),
    }
}

/// `g_YnMapSystem`: 0, a system without a map, or one with `map`.
fn l6_map_system(t: &mut Twins, rng: &mut Rng, map: u32) {
    let sys = match rng.below(8) {
        0 => 0,
        1 => t.words(&[0, map]),
        _ => t.words(&[map, map]),
    };
    t.global(l6::YN_MAP_SYSTEM.sym, sys);
}

/// A [`Ret::Seq`] of `n` random words.
fn l6_seq(t: &mut Twins, rng: &mut Rng, n: usize) -> Ret {
    let v: Vec<u32> = (0..n).map(|_| rng.next()).collect();
    t.seq(&v)
}

#[test]
fn l6_chr_select_reset_matches_the_original() {
    // in and out of a battle; the handles' memset runs for real
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b01);
        for case in 0..8_u32 {
            let w = t.zeros(0x900);
            for k in 0..0x240_u32 {
                t.w32(w + 4 * k, rng.next());
            }
            t.global(l6::RCUI_WORK.sym, w);
            let battle = case % 2 * rng.next().max(1);
            t.stub_sym(l6::RCEF_IS_BATTLE, 1, false, 0, 0, Ret::Val(battle));
            t.stub_sym(l6::COLLECT_BY_GROUP, 2, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(l6::COLLECT, 3, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(l6::CHR_MOTION_SETUP, 4, false, 0, 0, Ret::Val(rng.next()));
            t.check(
                "rcuiChrSelectReset_00932320",
                0x0093_2320,
                Conv::Cdecl,
                &[],
                |o| Out::U32(l6m::chr_select_reset(o)),
            );
            t.reset();
        }
    });
}

#[test]
fn l6_save_proc_matches_the_original() {
    // without a work area or a map; every message; the save ids, 0 and
    // others; a save that moves g_RcuiWork (read again for the close)
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b02);
        for case in 0..160_u32 {
            let w = if case % 13 == 5 { 0 } else { t.zeros(0x100) };
            t.global(l6::RCUI_WORK.sym, w);
            let map = t.zeros(0x20);
            l6_map_system(t, &mut rng, map);
            let w2 = t.zeros(0x100);
            let mut swap = vec![0xc7, 0x05];
            swap.extend(t.rebase(l6::RCUI_WORK.sym).to_le_bytes());
            swap.extend(w2.to_le_bytes()); // mov dword [g_RcuiWork], w2
            swap.push(0xb8);
            swap.extend(rng.next().to_le_bytes()); // mov eax, x
            swap.push(0xc3);
            if case % 3 == 0 {
                r5_code(t, l6::SAVE_BIN, &swap);
            } else {
                t.stub_sym(l6::SAVE_BIN, 5, false, 0, 0, Ret::Val(rng.next()));
            }
            t.stub_sym(l6::ADD_ITEMS, 1, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(l6::CLOSE_WINDOW, 2, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(l6::MAKE_BASE_PATH, 3, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(l6::MAKE_FULL_PATH, 4, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(l6::SAVE_TEXT, 6, false, 0, 0, Ret::Val(rng.next()));
            let msg = l6_msg(&mut rng);
            let param = match rng.below(4) {
                0 => 0,
                1 | 2 => 0xd + rng.below(4),
                _ => [0xc, 0x11, 0x8000_000d, rng.next()][rng.below(4) as usize],
            };
            let window = rng.next();
            t.check(
                "rcuiSaveProc_009364d0",
                0x0093_64d0,
                Conv::Cdecl,
                &[window, msg, param],
                |o| l6_out(l6m::save_proc(o, window, msg, param)),
            );
            t.reset();
        }
    });
}

#[test]
fn l6_psim_proc_matches_the_original() {
    // with and without a simulation; every message (the position vector
    // is compared in the frame after the call)
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b03);
        for case in 0..80_u32 {
            let w = if case % 11 == 3 {
                0
            } else {
                l6_random(t, &mut rng, 0x100)
            };
            t.global(l6::RCUI_WORK.sym, w);
            let n = if rng.chance(2) { 0 } else { rng.next() };
            t.stub_sym(l6::ADD_ITEMS, 1, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(l6::PSIM_GET_CNT, 2, false, 0, 0, Ret::Val(n));
            t.stub_sym(l6::EMPTY_STUB, 3, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(l6::PSIM_GET_HANDLE, 4, false, 1, 0, Ret::Val(rng.next()));
            let pos = l6_seq(t, &mut rng, 2);
            t.stub_sym(l6::PSIM_SET_POS, 5, false, 3, 0, pos);
            l6_fill_stack(t, &mut rng, 0x200);
            let msg = l6_msg(&mut rng);
            let window = rng.next();
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - l6::PSIM_FRAME - 0x40);
            t.check(
                "rcuiPSimProc_00935710",
                0x0093_5710,
                Conv::Cdecl,
                &[window, msg],
                |o| l6_out(l6m::psim_proc(o, ebp, window, msg)),
            );
            r5_same(t, "rcuiPSimProc_00935710", ebp - 0x14, 0xc);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn l6_guide_target_proc_matches_the_original() {
    // without a work area, a system or a guide; the target steps (4 and
    // up wrap to 0, negatives step), the conversion, the text
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b04);
        for case in 0..400_u32 {
            let w = if case % 13 == 7 {
                0
            } else {
                l6_random(t, &mut rng, 0x100)
            };
            t.global(l6::RCUI_WORK.sym, w);
            let g = if rng.chance(8) { 0 } else { t.zeros(0x10) };
            let sys = if rng.chance(8) {
                0
            } else {
                t.words(&[rng.next(), g])
            };
            t.global(l6::YN_MAP_SYSTEM.sym, sys);
            let target = match rng.below(3) {
                0 => [3, 4, 5, rng.below(7)][rng.below(4) as usize],
                1 => (rng.below(3) as i32 - 2).cast_unsigned(),
                _ => rng.word(),
            };
            t.stub_sym(l6::ADD_ITEMS, 1, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(l6::GUIDE_GET_TARGET, 2, false, 1, 0, Ret::Val(target));
            t.stub_sym(l6::GUIDE_SET_TARGET, 3, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(l6::GUIDE_CONV_CURRENT, 4, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(l6::SET_TEXT, 5, false, 3, 0, Ret::Val(rng.next()));
            r5_zero_stack(t, 0x300);
            let msg = l6_msg(&mut rng);
            let param = match rng.below(4) {
                0 => 0x8f,
                1 => 0x91,
                2 => 0x90,
                _ => rng.word(),
            };
            let window = rng.next();
            let ebp = frame_ebp(&t.a, 3);
            t.b_top = Some(ebp - l6::GUIDE_FRAME - 0x40);
            t.check(
                "rcuiGuideTargetProc_00934110",
                0x0093_4110,
                Conv::Cdecl,
                &[window, msg, param],
                |o| l6_out(l6m::guide_proc(o, ebp, window, msg, param)),
            );
            r5_same(t, "rcuiGuideTargetProc_00934110", ebp - 0x14, 0x10);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn l6_palette_proc_matches_the_original() {
    // without a work area or a map, a zero entry; the mode and number at
    // and past their ends
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b05);
        for case in 0..600_u32 {
            let w = if case % 17 == 9 { 0 } else { t.zeros(0x300) };
            let k = rng.below(4);
            if w != 0 {
                t.w32(w + 0x288, k);
            }
            t.global(l6::RCUI_WORK.sym, w);
            let block = t.zeros(0x400);
            let base = if rng.chance(8) {
                0u32.wrapping_sub(0xb0 + 0xc0 * k)
            } else {
                block
            };
            let list = t.words(&[0, 0, base]);
            let map = t.words(&[0, 0, list]);
            l6_map_system(t, &mut rng, map);
            let ent = block + 0xb0 + 0xc0 * k;
            t.w8(ent + 0xa, rng.below(5) as u8);
            let n = [0u8, 1, 0x62, 0x63, 0x64, 0xff][rng.below(6) as usize];
            t.w8(ent + 0xb, n);
            t.stub_sym(l6::ADD_ITEMS, 1, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(l6::SET_TEXT, 2, false, 3, 0, Ret::Val(rng.next()));
            t.stub_sym(l6::PRINT_SET_X, 3, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(l6::PRINT_SET_Y, 4, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(l6::GU_PRINTF, 5, false, 2, 0, Ret::Val(rng.next()));
            let msg = l6_msg(&mut rng);
            let param = match rng.below(5) {
                0 => 0x33,
                1 => 0x34,
                2 => 0x35,
                3 => 0x36,
                _ => rng.word(),
            };
            let window = rng.next();
            t.check(
                "rcuiPaletteProc",
                0x0093_5820,
                Conv::Cdecl,
                &[window, msg, param],
                |o| l6_out(l6m::palette_proc(o, window, msg, param)),
            );
            t.reset();
        }
    });
}

/// A 4x4 matrix of float words at `at`.
fn l6_matrix(t: &mut Twins, rng: &mut Rng, at: u32) {
    for k in 0..16 {
        let v = if rng.chance(6) {
            float_word(rng)
        } else {
            (rng.below(4000) as f32 / 64.0 - 30.0).to_bits()
        };
        t.w32(at + 4 * k, v);
    }
}

#[test]
fn l6_pack_points_matches_the_original() {
    // the matrices and vector math run for real; the clip flags come from
    // a stub (0 and not, so both entry kinds and the carried flag); n of
    // 0 and negative leaves only the colour; ftol both ways (the CRT's
    // SSE2 flag off on the image, set here on odd cases)
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b06);
        for case in 0..40_u32 {
            for m in [l6::CAMERA_MATRIX, l6::SCREEN_MATRIX] {
                let at = t.rebase(m);
                let mut old = vec![0; 64];
                Mem::read_bytes(&t.b, at, &mut old);
                t.patched.push((at, old));
                l6_matrix(t, &mut rng, at);
            }
            let matrix = t.zeros(64);
            l6_matrix(t, &mut rng, matrix);
            let n = match case % 8 {
                0 => 0,
                1 => 0xffff_fffe,
                _ => 1 + rng.below(6),
            };
            let points = t.zeros(0x80);
            for k in 0..0x20 {
                let v = if rng.chance(8) {
                    float_word(&mut rng)
                } else {
                    (rng.below(2000) as f32 / 16.0 - 60.0).to_bits()
                };
                t.w32(points + 4 * k, v);
            }
            let colour = t.zeros(16);
            for k in 0..4 {
                let v = match rng.below(4) {
                    0 => float_word(&mut rng),
                    1 => (rng.below(256) as f32).to_bits(),
                    _ => (rng.below(2000) as f32 / 7.0 - 100.0).to_bits(),
                };
                t.w32(colour + 4 * k, v);
            }
            let out = l6_random(t, &mut rng, 0x100);
            let pkt = t.words(&[rng.next(), out]);
            let clips: Vec<u32> = (0..8)
                .map(|_| {
                    if rng.chance(3) {
                        rng.next()
                    } else {
                        rng.below(2) * 4
                    }
                })
                .collect();
            let seq = t.seq(&clips);
            t.stub_sym(l6::SCREEN_CLIP, 1, false, 1, 0, seq);
            if case % 2 == 1 {
                t.global(ffx_bridge::x87::SSE2_AVAILABLE.sym, 1);
            }
            r5_zero_stack(t, 0x300);
            let ebp = frame_ebp(&t.a, 5);
            t.b_top = Some(ebp - l6::PACK_FRAME - 0x40);
            let args = [pkt, points, n, colour, matrix];
            t.check(
                "rcbgGuidePackPoints_009278d0",
                0x0092_78d0,
                Conv::Cdecl,
                &args,
                |o| Out::U32(l6m::pack_points(o, ebp, pkt, points, n, colour, matrix)),
            );
            r5_same(
                t,
                "rcbgGuidePackPoints_009278d0",
                ebp - l6::PACK_FRAME,
                0x68,
            );
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn l6_axis_marker_matches_the_original() {
    // the colour scaling, the perspective matrix and the vector math run
    // for real; the point lists and the circle are stubs; sizes of 0,
    // negative, large; the circle's radius from negative and positive
    // counters
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b07);
        for case in 0..32_u32 {
            let w = t.zeros(0x100);
            t.w32(w + 0x58, rng.word());
            t.global(l6::RCUI_WORK.sym, w);
            let matrix = t.zeros(64);
            l6_matrix(t, &mut rng, matrix);
            let size = match case % 4 {
                0 => 0,
                1 => (rng.below(40) as i32 - 20).cast_unsigned(),
                _ => rng.word(),
            };
            let pkt = rng.next();
            t.stub_sym(l6::PACK_POINTS, 1, false, 5, 0, Ret::Val(rng.next()));
            t.stub_sym(l6::PACKET_CIRCLE, 2, false, 5, 0, Ret::Val(rng.next()));
            r5_zero_stack(t, 0x300);
            let ebp = frame_ebp(&t.a, 3);
            t.b_top = Some(ebp - l6::AXIS_FRAME - 0x60);
            t.check(
                "rcbgGuideAxisMarker_0092c600",
                0x0092_c600,
                Conv::Cdecl,
                &[pkt, matrix, size],
                // the oracle's edi, esi, ebx at the call (saved where the scale's
                // by-value vector reads them)
                |o| {
                    let junk = [0x3333_3333, 0x2222_2222, 0x1111_1111];
                    Out::U32(l6m::axis_marker(o, ebp, pkt, matrix, size, junk))
                },
            );
            r5_same(t, "rcbgGuideAxisMarker_0092c600", ebp - 0x24, 0x20);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 6 (left6) batch 1, part 2 (ffx_prender::l6_map2) ----

use ffx_prender::l6_map2::{self as l62, bridge as l6m2};

/// The argument slots of a cdecl call with `args` from the call top: `b`
/// gets the words the original's call puts there (the bridge makes no
/// call), and its own calls go below them.
fn l6_arg_slots(t: &mut Twins, args: &[u32]) -> u32 {
    let slots = first_slot(&t.a, args.len() as u32);
    let bytes: Vec<u8> = args.iter().flat_map(|v| v.to_le_bytes()).collect();
    t.b.write(slots, &bytes).unwrap();
    t.b_top = Some(slots - 0x60);
    slots
}

/// A layer list (`[map + 8]`): `n` entries of 0xc0 bytes with random
/// `+4`, `+8`, a random `+0xf8`, and the scroll at `+0x5c..`.
fn l6_layer_list(t: &mut Twins, rng: &mut Rng, n: u32) -> u32 {
    let entries = t.zeros(0xc0 * 8);
    for k in 0..8 {
        let [x, y] = [0, 1].map(|_| match rng.below(3) {
            0 => rng.word(),
            _ => rng.below(0x600),
        });
        t.w32(entries + 0xc0 * k + 4, x);
        t.w32(entries + 0xc0 * k + 8, y);
    }
    let list = t.zeros(0x100);
    t.w32(list + 4, n);
    t.w32(list + 8, entries);
    t.w32(list + 0xf8, u32::from(rng.chance(2)));
    for off in [0x5c, 0x60, 0x64] {
        let v = match rng.below(3) {
            0 => rng.word(),
            1 => (rng.below(40) as i32 - 20).cast_unsigned(),
            _ => rng.below(0x400),
        };
        t.w32(list + off, v);
    }
    list
}

#[test]
fn l6_layer_select_proc_matches_the_original() {
    // without a work area, system, map or list; the steps at both ends
    // (wrapping), other buttons, the text
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b11);
        for case in 0..200_u32 {
            let w = if case % 17 == 4 {
                0
            } else {
                l6_random(t, &mut rng, 0xb00)
            };
            let n = rng.below(5);
            let list = if rng.chance(10) {
                0
            } else {
                l6_layer_list(t, &mut rng, n)
            };
            let msg = match rng.below(6) {
                0 => 0x800a,
                _ => l6_msg(&mut rng),
            };
            if w != 0 {
                // indices inside the 8 entries for the draw (it reads one)
                let k = match rng.below(4) {
                    0 => 0,
                    1 if n > 0 => n - 1,
                    3 if msg != 0x8011 => rng.word(),
                    _ => rng.below(8),
                };
                t.w32(w + 0xae0, k);
            }
            t.global(l62::RCUI_WORK.sym, w);
            let map = t.words(&[0, 0, list]);
            l6_map_system(t, &mut rng, map);
            t.stub_sym(l62::ADD_ITEMS, 1, false, 2, 0, Ret::Val(rng.next()));
            l6_copy(t, l62::SET_TEXT, 2, 3, 2, 2, rng.next());
            t.stub_sym(l62::PRINT_SET_X, 3, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(l62::PRINT_SET_Y, 4, false, 1, 0, Ret::Val(rng.next()));
            let pr = l6_seq(t, &mut rng, 2);
            t.stub_sym(l62::GU_PRINTF, 5, false, 2, 0, pr);
            let param = if rng.chance(4) { rng.word() } else { 0xb7 };
            let window = rng.next();
            r5_zero_stack(t, 0x300);
            let ebp = frame_ebp(&t.a, 3);
            t.b_top = Some(ebp - l62::LAYER_FRAME - 0x40);
            t.check(
                "rcuiLayerSelectProc_00936840",
                0x0093_6840,
                Conv::Cdecl,
                &[window, msg, param],
                |o| l6_out(l6m2::layer_proc(o, ebp, window, msg, param)),
            );
            r5_same(t, "rcuiLayerSelectProc_00936840", ebp - 0x14, 0x10);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn l6_fp_mem_proc_matches_the_original() {
    // sizes of 0 (counted as 1), negative and large counts; the close;
    // other messages
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b12);
        for case in 0..60_u32 {
            for (k, f) in l62::FP_STATS.iter().enumerate() {
                let v = match rng.below(5) {
                    0 => 0,
                    1 => rng.word(),
                    _ => rng.below(0x10_0000) + u32::from(k == 2 || k == 5),
                };
                t.stub_sym(*f, 1 + k as u32, false, 0, 0, Ret::Val(v));
            }
            t.stub_sym(l62::PRINT_SET_X, 7, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(l62::PRINT_SET_Y, 8, false, 1, 0, Ret::Val(rng.next()));
            let pr = l6_seq(t, &mut rng, 8);
            l6_printf(t, l62::GU_PRINTF, 9, &[(l62::FP_KB_FORMAT, 1)], pr);
            t.stub_sym(l62::PERCENT_PRINT, 10, false, 1, 0, Ret::Val(rng.next()));
            let holder = t.rebase(l62::FP_MEM_HOLDER);
            let mut old = [0; 4];
            Mem::read_bytes(&t.b, holder, &mut old);
            t.patched.push((holder, old.to_vec()));
            t.w32(holder, rng.next());
            let msg = match case % 6 {
                0 => 0x8003,
                1 => rng.word(),
                _ => 0x8011,
            };
            let window = rng.next();
            r5_zero_stack(t, 0x300);
            let args = [window, msg];
            let slots = l6_arg_slots(t, &args);
            let ebp = frame_ebp(&t.a, 2);
            t.check(
                "rcuiFpMemProc_00933aa0",
                0x0093_3aa0,
                Conv::Cdecl,
                &args,
                |o| Out::U32(l6m2::fp_mem_proc(o, ebp, slots, msg)),
            );
            r5_same(t, "rcuiFpMemProc_00933aa0", ebp - 0x14, 0x14);
            r5_same(t, "rcuiFpMemProc_00933aa0 slots", slots, 8);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn l6_num_entry_proc_matches_the_original() {
    // every key on texts of 0..12 characters (with and without a sign, a
    // pending point, afresh), the enter for floats (atof for real) and
    // integers, the cancel, the draw with and without an entry
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b13);
        for case in 0..400_u32 {
            let e = t.zeros(0x80) + 0x20;
            let len = match rng.below(4) {
                0 => 0,
                1 => 10 + rng.below(3),
                _ => rng.below(10),
            };
            let mut text = vec![0u8; 16];
            for (k, c) in text.iter_mut().enumerate().take(len as usize) {
                *c = b"0123456789."[rng.below(11) as usize];
                if k == 0 && rng.chance(3) {
                    *c = b'-';
                }
            }
            if case % 16 == 5 {
                text[0] = b'-';
            }
            t.write(e + 0xc, &text);
            t.w32(e, u32::from(!rng.chance(5)));
            let target = t.zeros(8);
            t.w32(e + 4, target);
            t.w32(e + 8, if rng.chance(2) { 0xb } else { rng.below(12) });
            // odd lengths: negative, past the text (16: the copy reaches
            // the length itself)
            let odd = [0xffff_ffff, 0xffff_fffe, 16, 17, 20, rng.below(30)];
            let n = if case % 16 == 5 {
                [16, 17, 20][rng.below(3) as usize]
            } else if rng.chance(10) {
                odd[rng.below(6) as usize]
            } else {
                len
            };
            t.w32(e + 0x1c, n);
            t.w32(e + 0x20, u32::from(rng.chance(4)));
            t.w32(e + 0x24, u32::from(rng.chance(3)));
            let (msg, param) = match if case % 16 == 5 { 0 } else { rng.below(8) } {
                0 if case % 16 == 5 => (0x8007, 0xb),
                0..=3 => (0x8007, rng.below(14)),
                4 => (0x8006, 0xd + rng.below(4)),
                5 => (0x8011, rng.next()),
                6 => (l6_msg(&mut rng), rng.next()),
                _ => (0x8003, rng.next()),
            };
            // no entry only where the original survives it: the draw
            // checks, the item add and the default don't read it
            let safe = msg == 0x8011 || msg == 0x8001 || !(0x8001..=0x8011).contains(&msg);
            let arg = if safe && case % 5 == 1 { 0 } else { e };
            t.stub_sym(l62::WINDOW_GET_ARG, 1, false, 1, 0, Ret::Val(arg));
            t.stub_sym(l62::ADD_ITEMS, 2, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(l62::VALUE_SET, 3, false, 3, 0, Ret::Val(rng.next()));
            t.stub_sym(l62::WINDOW_CLOSE_TREE, 4, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(l62::PACKET_BEGIN_TAG, 5, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(l62::SET_PRIM, 6, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(l62::SET_RGBAQ, 7, false, 4, 0, Ret::Val(rng.next()));
            t.stub_sym(l62::SET_XYZF2, 8, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(l62::PACKET_END_TAG, 9, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(l62::PRINT_SET_X, 10, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(l62::PRINT_SET_Y, 11, false, 1, 0, Ret::Val(rng.next()));
            l6_copy(t, l62::GU_PRINTF, 12, 2, 1, 4, rng.next());
            let window = rng.next();
            r5_zero_stack(t, 0x300);
            let args = [window, msg, param];
            let slots = l6_arg_slots(t, &args);
            t.check(
                "rcuiNumEntryProc_0093c3f0",
                0x0093_c3f0,
                Conv::Cdecl,
                &args,
                |o| Out::U32(l6m2::entry_proc(o, slots, window, msg, param)),
            );
            r5_same(t, "rcuiNumEntryProc_0093c3f0 slots", slots, 12);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn l6_scroll_proc_matches_the_original() {
    // without a work area or a map; no layers, negative extents; every
    // step button and the clamps at both ends; the mouse follow
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b14);
        for case in 0..300_u32 {
            let w = if case % 19 == 6 { 0 } else { t.zeros(0x100) };
            t.global(l62::RCUI_WORK.sym, w);
            let n = match rng.below(4) {
                0 => 0,
                1 => rng.word() | 0x8000_0000,
                _ => 1 + rng.below(8),
            };
            let list = l6_layer_list(t, &mut rng, n);
            let map = t.words(&[0, 0, list]);
            l6_map_system(t, &mut rng, map);
            t.stub_sym(l62::ADD_ITEMS, 1, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(l62::MOUSE_MOVE_RESET, 2, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(l62::MOUSE_MOVE_BEGIN, 3, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(l62::MOUSE_MOVE_END, 4, false, 0, 0, Ret::Val(rng.next()));
            let moving = u32::from(rng.chance(2));
            t.stub_sym(l62::MOUSE_MOVE_IS_ID, 5, false, 1, 0, Ret::Val(moving));
            let (dx, dy) = [0, 1]
                .map(|_| match rng.below(3) {
                    0 => rng.word(),
                    _ => (rng.below(0x200) as i32 - 0x100).cast_unsigned(),
                })
                .into();
            // the delta getter writes both outs
            let mut get = vec![0x8b, 0x44, 0x24, 0x04, 0xc7, 0x00]; // mov eax, [esp+4]; mov dword [eax], dx
            get.extend(dx.to_le_bytes());
            get.extend([0x8b, 0x44, 0x24, 0x08, 0xc7, 0x00]); // mov eax, [esp+8]; mov dword [eax], dy
            get.extend(dy.to_le_bytes());
            get.push(0xc3);
            r5_code(t, l62::MOUSE_MOVE_GET, &get);
            t.stub_sym(l62::PRINT_SET_X, 6, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(l62::PRINT_SET_Y, 7, false, 1, 0, Ret::Val(rng.next()));
            let pr = l6_seq(t, &mut rng, 3);
            t.stub_sym(l62::GU_PRINTF, 8, false, 2, 0, pr);
            let (msg, param) = match rng.below(6) {
                0..=2 => (0x8006, 0x52 + rng.below(9)),
                3 => (0x8007, if rng.chance(2) { 0x59 } else { rng.next() }),
                4 => (0x8011, rng.next()),
                _ => (l6_msg(&mut rng), rng.next()),
            };
            let window = rng.next();
            r5_zero_stack(t, 0x300);
            let ebp = frame_ebp(&t.a, 3);
            t.b_top = Some(ebp - l62::SCROLL_FRAME - 0x40);
            t.check(
                "rcuiScrollProc_009365c0",
                0x0093_65c0,
                Conv::Cdecl,
                &[window, msg, param],
                |o| l6_out(l6m2::scroll_proc(o, ebp, window, msg, param)),
            );
            r5_same(t, "rcuiScrollProc_009365c0", ebp - 0x14, 0x14);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn l6_fp_save_sub_matches_the_original() {
    // the open failing; 0, 1 and several models with pads 0..15; both
    // path formats (the real sprintf builds the path and the command)
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b15);
        for case in 0..32_u32 {
            let status = if rng.chance(2) {
                0x1000
            } else {
                rng.next() & !0x1000
            };
            t.stub_sym(l62::GET_RENDER_STATUS, 1, false, 0, 0, Ret::Val(status));
            let fd = match case % 7 {
                3 => u32::MAX,
                5 => 0xffff_fffe,
                _ => rng.below(100),
            };
            l6_copy(t, l62::SCE_OPEN, 2, 2, 0, 10, fd);
            let n = match case % 5 {
                0 => 0,
                1 => 1,
                4 if case % 2 == 0 => 0xffff_fff0,
                _ => rng.below(4),
            };
            t.stub_sym(l62::SCENE_COUNT_OBJECTS, 3, false, 1, 0, Ret::Val(n));
            let objs: Vec<u32> = (0..4)
                .map(|_| {
                    let o = t.zeros(0xa0);
                    for off in [0x50, 0x54, 0x90, 0x94] {
                        let v = if rng.chance(4) {
                            rng.next()
                        } else {
                            rng.below(300)
                        };
                        t.w32(o + off, v);
                    }
                    o
                })
                .collect();
            let objs = t.seq(&objs);
            t.stub_sym(l62::SCENE_GET_OBJECT, 4, false, 2, 0, objs);
            let pads: Vec<u32> = (0..4).map(|_| rng.below(16)).collect();
            let pads = t.seq(&pads);
            t.stub_sym(l62::PAD16, 5, false, 1, 0, pads);
            let tags = t.zeros(0x40);
            for k in 0..0x10 {
                t.w32(tags + 4 * k, rng.next());
            }
            let tag_seq: Vec<u32> = (0..8).map(|k| tags + 4 * k).collect();
            let tag_seq = t.seq(&tag_seq);
            t.stub_sym(l62::CHUNK_TAG, 6, false, 1, 0, tag_seq);
            l6_copy(t, l62::SCE_WRITE, 7, 3, 1, 4, rng.next());
            t.stub_sym(l62::TRX_SAVE2, 8, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(l62::SCE_CLOSE, 9, false, 1, 0, Ret::Val(rng.next()));
            l6_copy(t, l62::SYSTEM_ON_HOST, 10, 2, 0, 12, rng.next());
            let pr = l6_seq(t, &mut rng, 2);
            let more = [(l62::FP_SAVE_ERROR[0], 2), (l62::FP_SAVE_LOG, 2)];
            l6_printf(t, l62::RC_PRINT, 11, &more, pr);
            let name = t.alloc(b"b042\0");
            r5_zero_stack(t, 0x600);
            let ebp = frame_ebp(&t.a, 1);
            t.b_top = Some(ebp - l62::FP_SAVE_FRAME - 0x60);
            t.check("rcbgFpSaveSub", 0x0093_8350, Conv::Cdecl, &[name], |o| {
                Out::U32(l6m2::fp_save_sub(o, ebp, name))
            });
            r5_same(t, "rcbgFpSaveSub", ebp - 0x340, 0x33c);
            t.b_top = None;
            t.reset();
        }
    });
}

/// A recording stub over `sym` (cdecl) that logs `n` arguments, then
/// `words` words read through argument `k` (a string or buffer the
/// caller built), and returns `ret`. Unlike `copy_sym` it does not log
/// `ecx` (these callees don't read it).
fn l6_copy(t: &mut Twins, sym: u32, tag: u32, n: u32, k: u32, words: u32, ret: u32) {
    assert!(n + words <= 14);
    let log = t.log;
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(tag.to_le_bytes()); // mov [eax], tag
    for i in 0..n {
        c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
        c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
    }
    if words > 0 {
        c.extend([0x56, 0x57, 0x51]); // push esi; push edi; push ecx
        c.extend([0x8b, 0x74, 0x24, (0x10 + 4 * k) as u8]); // mov esi, [esp+0x10+4k]
        c.extend([0x8d, 0x78, (8 + 4 * n) as u8]); // lea edi, [eax+8+4n]
        c.push(0xb9);
        c.extend(words.to_le_bytes()); // mov ecx, words
        c.extend([0xf3, 0xa5]); // rep movsd
        c.extend([0x59, 0x5f, 0x5e]); // pop ecx; pop edi; pop esi
    }
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    c.push(0xb8);
    c.extend(ret.to_le_bytes()); // mov eax, ret
    c.push(0xc3);
    r5_code(t, sym, &c);
}

/// A recording stub for a printf (`Gu_Printf`, `rcPrint`): logs the
/// format, and the next words only for the formats in `more` (with their
/// argument count; the others take none: the words there are the
/// caller's stack), returning `ret`.
fn l6_printf(t: &mut Twins, sym: u32, tag: u32, more: &[(u32, u8)], ret: Ret) {
    let log = t.log;
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(tag.to_le_bytes()); // mov [eax], tag
    c.extend([0x8b, 0x54, 0x24, 0x04, 0x89, 0x50, 0x08]); // mov edx, [esp+4]; mov [eax+8], edx
    for &(f, n) in more {
        c.extend([0x81, 0xfa]);
        c.extend(t.rebase(f).to_le_bytes()); // cmp edx, f
        c.extend([0x75, 7 * n]); // jne past the copies
        for i in 0..n {
            c.extend([0x8b, 0x4c, 0x24, 8 + 4 * i]); // mov ecx, [esp+8+4i]
            c.extend([0x89, 0x48, 0xc + 4 * i]); // mov [eax+0xc+4i], ecx
        }
    }
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    match ret {
        Ret::Val(v) => {
            c.push(0xb8);
            c.extend(v.to_le_bytes()); // mov eax, v
        }
        Ret::Seq(cell) => {
            c.push(0xa1);
            c.extend(cell.to_le_bytes()); // mov eax, [cell]
            c.extend([0x8b, 0x00]); // mov eax, [eax]
            c.extend([0x83, 0x05]);
            c.extend(cell.to_le_bytes());
            c.push(0x04); // add dword [cell], 4
        }
        Ret::Keep => {}
    }
    c.push(0xc3);
    r5_code(t, sym, &c);
}

// ---- round 6 (left6) batch 2 (ffx_prender::l6_map3) ----

use ffx_prender::l6_map3::{self as l63, bridge as l6m3};

#[test]
fn l6_guide_skeleton_matches_the_original() {
    // a picked bone or the ten from +0x8c0 (cut at the last); no bones,
    // a negative count; parents -1, in and out of the range; the bone
    // matrices (one of four by index) and the projection math for real
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b21);
        for case in 0..60_u32 {
            let w = t.zeros(0x900);
            let n = match case % 6 {
                0 => 0,
                1 => 0xffff_fffd,
                _ => 1 + rng.below(12),
            };
            t.w32(w + 0x8b8, n);
            t.w32(w + 0x8bc, u32::from(rng.chance(3)));
            // a negative first bone: the range compares are signed
            t.w32(
                w + 0x8c0,
                if case % 4 == 2 {
                    0xffff_fffe
                } else {
                    rng.below(14)
                },
            );
            t.w32(w + 0x8c4, rng.below(14));
            t.global(l63::RCUI_WORK.sym, w);
            let buf = t.zeros(0x100);
            t.stub_sym(l63::GET_PTR32, 1, false, 1, 0, Ret::Val(buf));
            let mats = t.zeros(0x100);
            l6_matrix(t, &mut rng, mats);
            l6_matrix(t, &mut rng, mats + 0x40);
            l6_matrix(t, &mut rng, mats + 0x80);
            l6_matrix(t, &mut rng, mats + 0xc0);
            // Ch_ElemGetMatrix(chr, k, out): matrix k & 3 into out
            let mut get = vec![0x56, 0x57]; // push esi; push edi
            get.extend([0x8b, 0x44, 0x24, 0x10]); // mov eax, [esp+0x10] (k)
            get.extend([0x83, 0xe0, 0x03, 0xc1, 0xe0, 0x06]); // and eax, 3; shl eax, 6
            get.extend([0x8d, 0xb0]);
            get.extend(mats.to_le_bytes()); // lea esi, [eax+mats]
            get.extend([0x8b, 0x7c, 0x24, 0x14]); // mov edi, [esp+0x14] (out)
            get.extend([0xb9, 0x10, 0, 0, 0, 0xf3, 0xa5]); // mov ecx, 16; rep movsd
            get.extend([0x5f, 0x5e, 0xc3]); // pop edi; pop esi; ret
            r5_code(t, l63::ELEM_GET_MATRIX, &get);
            // a stub: the original hands its by-value scale vector's y, z,
            // w from its caller's frame (saved esi, ebp, the return
            // address) to the callee, which copies them into 0x00c8f974
            t.stub_sym(l63::SCALE_MATRIX100, 6, false, 1, 0, Ret::Val(rng.next()));
            let parents: Vec<u32> = (0..16)
                .map(|_| {
                    if rng.chance(8) {
                        0xffff_fffe
                    } else if rng.chance(3) {
                        u32::MAX
                    } else {
                        rng.below(n.clamp(1, 14))
                    }
                })
                .collect();
            let parents = t.seq(&parents);
            t.stub_sym(l63::ELEM_GET_PARENT, 2, false, 2, 0, parents);
            let lines = l6_seq(t, &mut rng, 16);
            t.stub_sym(l63::PACKET_LINE, 3, false, 6, 0, lines);
            let rects = l6_seq(t, &mut rng, 16);
            t.stub_sym(l63::PACKET_RECT, 4, false, 6, 0, rects);
            t.stub_sym(l63::AXIS_MARKER_WRAP, 5, false, 2, 0, Ret::Val(rng.next()));
            let (pkt, chr) = (rng.next(), rng.next());
            r5_zero_stack(t, 0x400);
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - l63::SKELETON_FRAME - 0x60);
            t.check(
                "rcbgGuideSkeleton_0092c250",
                0x0092_c250,
                Conv::Cdecl,
                &[pkt, chr],
                |o| Out::U32(l6m3::skeleton(o, ebp, pkt, chr)),
            );
            r5_same(t, "rcbgGuideSkeleton_0092c250", ebp - 0x94, 0x90);
            t.b_top = None;
            t.reset();
        }
    });
}

/// A block of `n` random bytes (words).
fn l6_random(t: &mut Twins, rng: &mut Rng, n: u32) -> u32 {
    let w: Vec<u32> = (0..n / 4).map(|_| rng.next()).collect();
    t.words(&w)
}

/// Fills `n` bytes of both stacks below the call top with the same
/// random words (a store of 0 over 0 shows nothing).
fn l6_fill_stack(t: &mut Twins, rng: &mut Rng, n: u32) {
    let top = call_top(&t.a);
    let w: Vec<u8> = (0..n / 4).flat_map(|_| rng.next().to_le_bytes()).collect();
    t.a.write(top - n, &w).unwrap();
    t.b.write(top - n, &w).unwrap();
}

// ---- round 6 (left6) batch 3 (ffx_prender::l6_map4) ----

use ffx_prender::l6_map4::{self as l64, bridge as l6m4};

/// A scroll model: header (`+0` flags, `+4` the sub-meshes' offset, `i16
/// +0x10` the total count) and `n` sub-meshes of `5c + 2` quadwords (`+1`
/// kind, `i16 +2` count c, `+8` a texture descriptor). The first is of
/// kind 3 (another kind steps to a never-written frame word there).
fn l6_scroll_model(t: &mut Twins, rng: &mut Rng, n: u32) -> u32 {
    let model = t.zeros(0x2000);
    // the update walks on from the header itself after the first
    // sub-mesh: its i16 at +2 must count down (0 or less loops for ever
    // in the original)
    t.w32(model, (rng.next() & 0xffff) | (1 + rng.below(4)) << 16);
    t.w32(model + 4, 0x40);
    let mut at = model + 0x40;
    let mut total = 0_u32;
    for k in 0..n {
        let c = 1 + rng.below(3);
        let kind = if k == 0 || rng.chance(3) {
            3
        } else {
            rng.below(5) as u8
        };
        t.w8(at + 1, kind);
        t.write(at + 2, &(c as u16).to_le_bytes());
        t.w32(at + 8, rng.next());
        t.w32(at + 0xc, rng.next());
        total += c;
        at += (5 * c + 2) * 8;
    }
    let total = if rng.chance(6) { 0 } else { total };
    t.write(model + 0x10, &(total as u16).to_le_bytes());
    model
}

#[test]
fn l6_scroll_model_create_matches_the_original() {
    // the three modes; textures failing; no sub-meshes; the frame (counts,
    // slots, flags) compared after
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b31);
        for case in 0..40_u32 {
            let subs = 1 + rng.below(5);
            let model = l6_scroll_model(t, &mut rng, subs);
            let loads: Vec<u32> = (0..8)
                .map(|_| if rng.chance(6) { 0 } else { rng.next() | 1 })
                .collect();
            let loads = t.seq(&loads);
            t.stub_sym(l64::VFX_IS_TEXTURE_LOADED, 1, false, 7, 0, loads);
            t.stub_sym(l64::VFX_CREATE, 2, false, 3, 0, Ret::Val(rng.next()));
            t.stub_sym(
                l64::VFX_CREATE_DATA_BUFFER,
                3,
                false,
                7,
                0,
                Ret::Val(rng.next()),
            );
            t.stub_sym(l64::DBG_PRINTF, 4, false, 3, 0, Ret::Val(rng.next()));
            let mode = [0, 1, 2, rng.next()][(case % 4) as usize];
            let args = [
                rng.next(),
                rng.next(),
                model,
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                mode,
            ];
            r5_zero_stack(t, 0x2200);
            let ebp = frame_ebp(&t.a, 9);
            t.b_top = Some(ebp - l64::CREATE_FRAME - 0x60);
            t.check(
                "createVFXRenderDataBuffer_DrawScrollModel",
                0x00a5_c130,
                Conv::Cdecl,
                &args,
                |o| Out::U32(l6m4::create_buffer(o, ebp, args)),
            );
            // to ebp - 8: there the original's _alloca_probe saved the caller's ecx
            l6_same_words(
                t,
                "createVFXRenderDataBuffer_DrawScrollModel",
                ebp,
                ebp - 0x1f78,
                0x1f70,
            );
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn l6_scroll_model_update_matches_the_original() {
    // no render data, a segment buffer failing, both blend flags; the
    // copies and the matrix product for real; the argument slots after
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b32);
        for case in 0..40_u32 {
            let subs = 1 + rng.below(5);
            let model = l6_scroll_model(t, &mut rng, subs);
            let (m1, m2) = (t.zeros(64), t.zeros(64));
            l6_matrix(t, &mut rng, m1);
            l6_matrix(t, &mut rng, m2);
            let rd = if case % 7 == 3 {
                0
            } else {
                l6_random(t, &mut rng, 0x100)
            };
            if rd != 0 {
                let parts = l6_random(t, &mut rng, 0x6c * 16);
                t.w32(rd + 0x94, parts);
            }
            let req = l6_random(t, &mut rng, 0x100);
            let fill = t.rebase(l64::FILL_MATRIX);
            let mut old = vec![0; 64];
            Mem::read_bytes(&t.b, fill, &mut old);
            t.patched.push((fill, old));
            l6_matrix(t, &mut rng, fill);
            t.stub_sym(l64::LOAD_MATRIX2, 1, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(l64::LOAD_MATRIX1, 2, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(l64::PACKET_GET_PTR, 3, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(l64::GET_RENDER_DATA, 4, false, 2, 0, Ret::Val(rd));
            t.stub_sym(l64::FILL_REQUEST, 5, false, 1, 0, Ret::Val(req));
            let bufs: Vec<u32> = (0..16)
                .map(|_| if rng.chance(10) { 0 } else { rng.next() | 1 })
                .collect();
            let bufs = t.seq(&bufs);
            t.stub_sym(l64::BUFFER_FOR_SEGMENT, 6, false, 4, 0, bufs);
            t.stub_sym(l64::VFX_SET_VISIBLE, 7, false, 3, 0, Ret::Val(rng.next()));
            let mut args = [0_u32; 17];
            for a in &mut args {
                *a = rng.next();
            }
            args[2] = model;
            args[4] = m1;
            args[5] = m2;
            args[10] = float_word(&mut rng);
            args[11] = float_word(&mut rng);
            args[12] = rng.word();
            r5_zero_stack(t, 0x300);
            let slots = l6_arg_slots(t, &args);
            t.check(
                "updateVFXRenderData_DrawModelScroll",
                0x00a5_c370,
                Conv::Cdecl,
                &args,
                |o| Out::U32(l6m4::update(o, slots, args)),
            );
            r5_same(t, "updateVFXRenderData_DrawModelScroll slots", slots, 0x44);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn l6_scroll_model_entry_matches_the_original() {
    // render data or not, a skipped frame, the creation or the set-up
    // failing (the callees are stubs; the update logs its first 14 words)
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b33);
        for case in 0..40_u32 {
            let skip = t.rebase(l64::SKIP_FRAME);
            t.patch(skip, &u32::from(case % 5 == 1).to_le_bytes());
            let rd = if case % 2 == 0 { 0 } else { rng.next() };
            t.stub_sym(l64::PACKET_GET_PTR, 1, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(l64::SET_PTR32, 2, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(l64::GET_RENDER_DATA, 3, false, 2, 0, Ret::Val(rd));
            t.stub_sym(l64::VFX_UPDATE, 4, false, 2, 0, Ret::Val(rng.next()));
            let made = if rng.chance(4) { 0 } else { rng.next() };
            t.stub_sym(l64::CREATE_BUFFER, 5, false, 9, 0, Ret::Val(made));
            let set = if rng.chance(4) { 0 } else { rng.next() };
            t.stub_sym(l64::VFX_SETUP, 6, false, 2, 0, Ret::Val(set));
            t.stub_sym(l64::UPDATE, 7, false, 14, 0, Ret::Val(rng.next()));
            t.stub_sym(l64::VFX_DESTROY, 8, false, 2, 0, Ret::Val(rng.next()));
            let mut args = [0_u32; 17];
            for a in &mut args {
                *a = rng.next();
            }
            args[10] = float_word(&mut rng);
            args[11] = float_word(&mut rng);
            // the original's buffer: its frame is aligned to 16 below the
            // return address and the saved ebx
            let entry = call_top(&t.a) - 4 * 17 - 4;
            let buf = ((entry - 12) & !0xf) - 0x70;
            t.check(
                "updateVFXRenderData_ScrollModelEntry_00a5bf50",
                0x00a5_bf50,
                Conv::Cdecl,
                &args,
                |o| Out::U32(l6m4::scroll_entry(o, args, buf)),
            );
            t.reset();
        }
    });
}

/// Both oracles' words at `at..at + n`; a difference names its `ebp`
/// offsets and values.
fn l6_same_words(t: &Twins, what: &str, ebp: u32, at: u32, n: u32) {
    let bad: Vec<(u32, u32, u32)> = (0..n / 4)
        .map(|k| at + 4 * k)
        .filter_map(|a| {
            let (x, y): (u32, u32) = (Mem::read(&t.a, a), Mem::read(&t.b, a));
            (x != y).then_some((ebp.wrapping_sub(a), x, y))
        })
        .collect();
    assert!(
        bad.is_empty(),
        "{what}: frame words (ebp - off, original, port): {bad:x?}"
    );
}

// ---- round 6 (left6) batch 3 (ffx_prender::l6_map5) ----

use ffx_prender::l6_map5::{self as l65, bridge as l6m5};

#[test]
fn l6_scene_camera_line_matches_the_original() {
    // the end of the text; the target form and the angle form, angles
    // across the wrap (the CRT's _CIfmod for real), negative, huge, NaN
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b41);
        for case in 0..80_u32 {
            let floats: Vec<u32> = (0..12)
                .map(|_| match rng.below(6) {
                    0 => float_word(&mut rng),
                    1 => (rng.below(4000) as f32 - 2000.0).to_bits(),
                    2 => [180.0_f32, -180.0, 540.0, -540.0, 360.0, 0.0][rng.below(6) as usize]
                        .to_bits(),
                    _ => (rng.below(7200) as f32 / 10.0 - 360.0).to_bits(),
                })
                .collect();
            let table = t.words(&floats);
            let cell = t.words(&[table]);
            // rcdtParseGetFloat(p): the next float in ST0
            let mut gf = vec![0xa1];
            gf.extend(cell.to_le_bytes()); // mov eax, [cell]
            gf.extend([0xd9, 0x00, 0x83, 0x05]); // fld dword [eax]; add dword [cell], 4
            gf.extend(cell.to_le_bytes());
            gf.extend([0x04, 0xc3]);
            r5_code(t, l65::PARSE_GET_FLOAT, &gf);
            let end = u32::from(case % 9 == 4) * rng.next().max(1);
            t.stub_sym(l65::PARSE_IS_END, 1, false, 1, 0, Ret::Val(end));
            let kw = if rng.chance(2) { 0 } else { rng.next() };
            t.stub_sym(l65::PARSE_GET_KEYWORD, 2, false, 2, 0, Ret::Val(kw));
            t.stub_sym(l65::PARSE_NEXT_TOKEN, 3, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(l65::SCENE_SET_MATRIX, 4, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(l65::CAMERA_CALC_REF, 5, false, 1, 0, Ret::Val(rng.next()));
            let scene = l6_random(t, &mut rng, 0x100);
            let parse = rng.next();
            t.check(
                "rcbgSceneLoadTextCamera_0093af60",
                0x0093_af60,
                Conv::Cdecl,
                &[scene, parse],
                |o| Out::U32(l6m5::camera_line(o, scene, parse)),
            );
            t.reset();
        }
    });
}

#[test]
fn l6_slider_proc_matches_the_original() {
    // every message and button; parts at and past their bounds; the mouse
    // on either gauge, another id, a zero delta; no entry, a hidden one
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b51);
        for case in 0..300_u32 {
            let e = t.zeros(0x40);
            t.w32(e, u32::from(!rng.chance(5)));
            let target = t.zeros(8);
            t.w32(e + 4, target);
            t.w32(e + 8, float_word(&mut rng));
            for off in [0xc, 0x10] {
                let v = match rng.below(4) {
                    0 => rng.word(),
                    1 => [0, 99, 100, 999, 1000][rng.below(5) as usize],
                    _ => rng.below(1000),
                };
                t.w32(e + off, v);
            }
            let (msg, param) = match rng.below(8) {
                0..=2 => (0x8006, rng.below(14)),
                3 => (0x8007, rng.below(11)),
                4 | 5 => (0x8011, rng.next()),
                6 => (l6_msg(&mut rng), rng.next()),
                _ => (0x8003, rng.next()),
            };
            let safe = msg == 0x8011
                || msg == 0x8001
                || msg == 0x8007
                || !(0x8001..=0x8011).contains(&msg);
            let arg = if safe && case % 6 == 1 { 0 } else { e };
            t.stub_sym(l65::bridge::WINDOW_GET_ARG, 1, false, 1, 0, Ret::Val(arg));
            t.stub_sym(l65::bridge::ADD_ITEMS, 2, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(
                l65::bridge::ADD_CLIENT,
                3,
                false,
                6,
                0,
                Ret::Val(rng.next()),
            );
            t.stub_sym(
                l65::bridge::CLOSE_TREE,
                4,
                false,
                1,
                0,
                Ret::Val(rng.next()),
            );
            t.stub_sym(
                l65::bridge::MOVE_RESET,
                5,
                false,
                0,
                0,
                Ret::Val(rng.next()),
            );
            t.stub_sym(
                l65::bridge::MOVE_BEGIN,
                6,
                false,
                2,
                0,
                Ret::Val(rng.next()),
            );
            t.stub_sym(l65::bridge::MOVE_END, 7, false, 0, 0, Ret::Val(rng.next()));
            let moving = u32::from(rng.chance(2));
            t.stub_sym(l65::bridge::MOVE_IS_ID, 8, false, 1, 0, Ret::Val(moving));
            let id = [8, 9, 7, rng.next()][rng.below(4) as usize];
            t.stub_sym(l65::bridge::MOVE_GET_ID, 9, false, 0, 0, Ret::Val(id));
            let dy = if rng.chance(4) { 0 } else { rng.word() };
            let mut get = vec![0x8b, 0x44, 0x24, 0x04, 0xc7, 0x00]; // mov eax, [esp+4]; mov [eax], dx
            get.extend(rng.next().to_le_bytes());
            get.extend([0x8b, 0x44, 0x24, 0x08, 0xc7, 0x00]); // mov eax, [esp+8]; mov [eax], dy
            get.extend(dy.to_le_bytes());
            get.push(0xc3);
            r5_code(t, l65::bridge::MOVE_GET, &get);
            t.stub_sym(
                l65::bridge::GAUGE_DRAW,
                10,
                false,
                4,
                0,
                Ret::Val(rng.next()),
            );
            t.stub_sym(
                l65::bridge::PRINT_SET_X,
                11,
                false,
                1,
                0,
                Ret::Val(rng.next()),
            );
            t.stub_sym(
                l65::bridge::PRINT_SET_Y,
                12,
                false,
                1,
                0,
                Ret::Val(rng.next()),
            );
            t.stub_sym(
                l65::bridge::PRINT_SET_METRICS,
                13,
                false,
                1,
                0,
                Ret::Val(rng.next()),
            );
            let pr = l6_seq(t, &mut rng, 8);
            let texts = l65::bridge::SLIDER_TEXTS;
            l6_printf(
                t,
                l65::bridge::GU_PRINTF,
                14,
                &[(texts[0], 1), (texts[1], 1)],
                pr,
            );
            let window = rng.next();
            l6_fill_stack(t, &mut rng, 0x300);
            let args = [window, msg, param];
            let slots = l6_arg_slots(t, &args);
            let ebp = frame_ebp(&t.a, 3);
            t.check(
                "rcuiSliderProc_0093c860",
                0x0093_c860,
                Conv::Cdecl,
                &args,
                |o| Out::U32(l6m5::slider_proc(o, ebp, slots, window, msg, param)),
            );
            l6_same_words(t, "rcuiSliderProc_0093c860", ebp, ebp - 0xc, 0xc);
            l6_same_words(t, "rcuiSliderProc_0093c860 slots", ebp, slots, 0xc);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn l6_motion_proc_matches_the_original() {
    // no work area, no character; every button with the motion at its
    // ends; paused, stepping and running; other messages
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b61);
        for case in 0..300_u32 {
            let w = if case % 23 == 7 {
                0
            } else {
                l6_random(t, &mut rng, 0xa00)
            };
            if w != 0 {
                let sel = rng.below(4);
                t.w32(w + 0x8b0, sel);
                let chr = if rng.chance(4) { u32::MAX } else { rng.next() };
                t.w32(w + 0x870 + 4 * sel, chr);
                // a motion inside the table (the draw reads its name)
                let m = [0, 1, 0x68, 0x67, rng.below(0x69)][rng.below(5) as usize];
                t.w32(w + 0x8d8, m);
                for off in [0x8dc, 0x8e4] {
                    t.w32(w + off, u32::from(rng.chance(2)));
                }
                t.w32(w + 0x8d4, [0, 0x100, rng.next()][rng.below(3) as usize]);
            }
            t.global(l65::bridge::RCUI_WORK, w);
            for (sym, tag, n) in [
                (l65::bridge::ADD_ITEMS, 1, 2),
                (l65::bridge::END_MOTION, 2, 2),
                (l65::bridge::CHR_HANDLE_INDEX, 3, 1),
                (l65::bridge::SET_MOTION, 4, 2),
                (l65::bridge::GET_SEQ_FRAME, 5, 1),
                (l65::bridge::MOTION_STEP, 6, 1),
                (l65::bridge::NUMBER_CREATE, 7, 5),
                (l65::bridge::SET_MOTION_SPEED, 8, 2),
                (l65::bridge::MOTION_GET_FRAME, 9, 1),
                (l65::bridge::PRINT_SET_X, 10, 1),
                (l65::bridge::PRINT_SET_Y, 11, 1),
                (l65::bridge::GU_PRINTF, 12, 3),
            ] {
                t.stub_sym(sym, tag, false, n, 0, Ret::Val(rng.next()));
            }
            l6_copy(t, l65::bridge::SET_TEXT, 13, 3, 2, 2, rng.next());
            let (msg, param) = match rng.below(8) {
                0..=2 => (0x8006, 0x98 + rng.below(9)),
                3 => (0x800a, 0x9d + rng.below(4)),
                4 | 5 => (0x8011, rng.next()),
                6 => (l6_msg(&mut rng), rng.next()),
                _ => (0x8003, rng.next()),
            };
            let window = rng.next();
            l6_fill_stack(t, &mut rng, 0x300);
            let ebp = frame_ebp(&t.a, 3);
            t.b_top = Some(ebp - l65::bridge::MOTION_FRAME - 0x40);
            t.check(
                "rcuiMotionProc",
                0x0093_5340,
                Conv::Cdecl,
                &[window, msg, param],
                |o| l6_out(l6m5::motion_proc(o, ebp, window, msg, param)),
            );
            l6_same_words(t, "rcuiMotionProc", ebp, ebp - 0x14, 0x10);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn l6_alpha_proc_matches_the_original() {
    // no work area, system, map or layers; every button with each field
    // value 0..3; the FIX entered or not
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b71);
        for case in 0..300_u32 {
            let w = if case % 29 == 3 {
                0
            } else {
                l6_random(t, &mut rng, 0x500)
            };
            let k = rng.below(4);
            if w != 0 {
                t.w32(w + 0x288, k);
            }
            t.global(l65::bridge::RCUI_WORK, w);
            let layers = l6_random(t, &mut rng, 0xc0 * 4);
            let n = [0, 0xffff_ffff, 4][rng.below(3) as usize];
            let list = t.words(&[0, if case % 5 == 0 { n } else { 4 }, layers]);
            let map = t.words(&[0, 0, list]);
            l6_map_system(t, &mut rng, map);
            t.stub_sym(l65::bridge::ADD_ITEMS, 1, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(l65::bridge::MOVE_END, 2, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(
                l65::bridge::NUMBER_CREATE,
                3,
                false,
                5,
                0,
                Ret::Val(rng.next()),
            );
            let ok = if rng.chance(2) { 0 } else { rng.next() };
            t.stub_sym(l65::bridge::NUMBER_IS_OK, 4, false, 1, 0, Ret::Val(ok));
            // names by pointer (a field of 3 indexes past the name tables;
            // the FIX text is compared in the frame)
            t.stub_sym(l65::bridge::SET_TEXT, 5, false, 3, 0, Ret::Val(rng.next()));
            let (msg, param) = match rng.below(7) {
                0..=2 => (0x8006, 0x3d + rng.below(11)),
                3 | 4 => (0x8011, rng.next()),
                5 => (l6_msg(&mut rng), rng.next()),
                _ => (0x8003, rng.next()),
            };
            let window = rng.next();
            l6_fill_stack(t, &mut rng, 0x300);
            let ebp = frame_ebp(&t.a, 3);
            t.b_top = Some(ebp - l65::bridge::ALPHA_FRAME - 0x40);
            t.check(
                "rcuiAlphaProc",
                0x0093_1050,
                Conv::Cdecl,
                &[window, msg, param],
                |o| l6_out(l6m5::alpha_proc(o, ebp, window, msg, param)),
            );
            l6_same_words(t, "rcuiAlphaProc", ebp, ebp - 0x18, 0x14);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 6 (left6) batch 4 (ffx_prender::l6_map6) ----

use ffx_prender::l6_map6::{self as l66, bridge as l6m6};

#[test]
fn l6_menu_proc_matches_the_original() {
    // no work area or system; every button with and without a scene and
    // layers; the shutdown with the work area cleared under it; the draw
    // with and without a map, both resolutions
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b81);
        for case in 0..300_u32 {
            let w = if case % 31 == 3 {
                0
            } else {
                l6_random(t, &mut rng, 0x100)
            };
            if w != 0 {
                t.w32(w + 0x5c, u32::from(rng.chance(2)));
            }
            t.global(l66::RCUI_WORK, w);
            let n = [0, 0xffff_ffff, 3][rng.below(3) as usize];
            let list = l6_random(t, &mut rng, 0x100);
            t.w32(list + 4, n);
            t.w32(list + 0xf8, u32::from(rng.chance(2)));
            let map = t.words(&[0, 0, list]);
            l6_map_system(t, &mut rng, map);
            for (sym, tag, k) in [
                (l66::ADD_ITEMS, 1, 2),
                (l66::MAKE_BASE_PATH, 2, 0),
                (l66::WINDOW_CREATE, 3, 9),
                (l66::LOAD_SET_TITLE, 4, 0),
                (l66::FP_MEM_WIN_OPEN, 5, 0),
                (l66::PRINT_SET_X, 7, 1),
                (l66::PRINT_SET_Y, 8, 1),
                (l66::PRINT_SET_BOLD, 9, 1),
                (l66::GU_PRINTF, 10, 1),
            ] {
                t.stub_sym(sym, tag, false, k, 0, Ret::Val(rng.next()));
            }
            // the shutdown: clears g_RcuiWork in some cases
            let mut sd = vec![];
            if rng.chance(2) {
                sd.extend([0xc7, 0x05]);
                sd.extend(t.rebase(l66::RCUI_WORK).to_le_bytes());
                sd.extend(0_u32.to_le_bytes()); // mov dword [g_RcuiWork], 0
            }
            sd.push(0xb8);
            sd.extend(rng.next().to_le_bytes()); // mov eax, x
            sd.push(0xc3);
            r5_code(t, l66::SHUTDOWN, &sd);
            let (msg, param) = match rng.below(7) {
                0 => (0x8006, 3),
                1 | 2 => (0x8006, rng.below(14)),
                3 => (0x8011, rng.next()),
                4 => (0x8003, rng.next()),
                5 => (l6_msg(&mut rng), rng.next()),
                _ => (0x8001, rng.next()),
            };
            // the menu's holder is the work area's first word: a closing
            // menu clears it, and the shutdown's own test runs before
            let window = rng.next();
            t.check(
                "rcuiMenuProc_00935000",
                0x0093_5000,
                Conv::Cdecl,
                &[window, msg, param],
                |o| l6_out(l6m6::menu_proc(o, window, msg, param)),
            );
            t.reset();
        }
    });
}

#[test]
fn l6_reset_chr_pos_matches_the_original() {
    // the vector helpers and the length for real; no system or map (a
    // zero list: the image's low page), no model, no vertices, a vertex
    // in a triangle of the first, a later or no group
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6b91);
        for case in 0..60_u32 {
            let small = |rng: &mut Rng| (rng.below(2000) as f32 / 20.0 - 50.0).to_bits();
            let list = t.zeros(0x100);
            for off in (0x68..0x80).step_by(4) {
                t.w32(list + off, small(&mut rng));
            }
            t.w32(
                list + 0xf4,
                [1.0_f32, 1.5, 3.0, 0.7, 5.0][rng.below(5) as usize].to_bits(),
            );
            let map = t.words(&[0, 0, list]);
            let sys = t.words(&[map, 0]);
            t.global(l66::YN_MAP_SYSTEM, sys);
            let n = [0, 1, 6, 12][rng.below(4) as usize];
            let verts: Vec<u32> = (0..2 * 16)
                .map(|_| {
                    let a = (rng.below(400) as i32 - 200) as u16;
                    let b = (rng.below(400) as i32 - 200) as u16;
                    u32::from(a) | u32::from(b) << 16
                })
                .collect();
            let mut verts = verts;
            // two equally near vertices: the first wins
            verts[2] = verts[0];
            verts[3] = verts[1];
            let verts = t.words(&verts);
            let groups_n = rng.below(4);
            let groups = t.zeros(0x10 * 4);
            for g in 0..groups_n {
                let c = rng.below(4);
                let tris = t.zeros(0x10 * 4);
                for k in 0..4 {
                    for q in 0..3 {
                        t.write(
                            tris + 0x10 * k + 2 * q,
                            &(rng.below(n.max(1)) as u16).to_le_bytes(),
                        );
                    }
                }
                t.write(groups + 0x10 * g + 8, &(c as u16).to_le_bytes());
                t.w32(groups + 0x10 * g + 0xc, tris);
            }
            let m = t.zeros(0x40);
            t.write(m + 6, &(groups_n as u16).to_le_bytes());
            t.write(m + 0xa, &(n as u16).to_le_bytes());
            t.w32(
                m + 0xc,
                [1.0_f32, 3.0, 7.0, 0.3, 16.0][rng.below(5) as usize].to_bits(),
            );
            t.w32(m + 0x18, verts);
            t.w32(m + 0x1c, groups);
            let model = if case % 7 == 2 { 0 } else { m };
            t.stub_sym(l66::bridge::ID_GET_PTR, 1, false, 0, 0, Ret::Val(model));
            t.stub_sym(
                l66::bridge::GET_PLAYER,
                2,
                false,
                0,
                0,
                Ret::Val(rng.next()),
            );
            t.stub_sym(l66::bridge::SET_POS3, 3, false, 4, 0, Ret::Val(rng.next()));
            l6_fill_stack(t, &mut rng, 0x300);
            let ebp = frame_ebp(&t.a, 0);
            t.b_top = Some(ebp - l66::bridge::RESET_FRAME - 0x60);
            t.check("rcuiResetChrPos", 0x0093_7b70, Conv::Cdecl, &[], |o| {
                // the oracle's esi, ebx, edi at the call
                Out::U32(l6m6::reset_chr_pos(
                    o,
                    ebp,
                    [0x2222_2222, 0x1111_1111, 0x3333_3333],
                ))
            });
            l6_same_words(t, "rcuiResetChrPos", ebp, ebp - 0x5c, 0x58);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn l6_fp_proc_matches_the_original() {
    // no work area; every button up and down with the number at and past
    // its ends; the stick to a character or an element of it
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6ba1);
        for case in 0..400_u32 {
            let w = if case % 37 == 3 {
                0
            } else {
                l6_random(t, &mut rng, 0xa00)
            };
            if w != 0 {
                let v = [
                    0,
                    1,
                    9,
                    10,
                    0x62,
                    0x63,
                    0x5a,
                    0x64,
                    0xffff_fff0,
                    rng.below(100),
                ][rng.below(10) as usize];
                t.w32(w + 0x5fc, v);
                t.w32(w + 0x8b0, rng.below(4));
                t.w32(w + 0x8b4, u32::from(rng.chance(2)));
            }
            t.global(l66::RCUI_WORK, w);
            let disp = t.rebase(l66::bridge::FP_DISP);
            t.patch(disp, &u32::from(rng.chance(2)).to_le_bytes());
            for (sym, tag, n) in [
                (l66::ADD_ITEMS, 1, 2),
                (l66::bridge::FP_RESTART, 2, 1),
                (l66::bridge::FP_END, 3, 1),
                (l66::bridge::FP_DISP_ON, 4, 1),
                (l66::bridge::FP_DISP_OFF, 5, 1),
                (l66::bridge::FP_RESTART_MODE2, 6, 1),
                (l66::bridge::FP_LOAD_TEXT, 7, 0),
                (l66::bridge::FP_SAVE, 8, 0),
                (l66::WINDOW_CREATE, 9, 9),
                (l66::bridge::FP_DISP_DEBUG, 10, 0),
                (l66::bridge::OPTPOS_GET_ELEMENT, 11, 3),
                (l66::bridge::FP_SET_STICK_CHR, 12, 3),
                (l66::bridge::FP_RESTART_GROUP, 13, 1),
                (l66::bridge::FP_FREE, 14, 1),
                (l66::bridge::FP_END_GROUP, 15, 1),
                (l66::bridge::FP_ID_STATE, 16, 1),
                (l66::bridge::SET_TEXT, 17, 3),
            ] {
                t.stub_sym(sym, tag, false, n, 0, Ret::Val(rng.next()));
            }
            let (msg, param) = match rng.below(9) {
                7 => (0x8006, 0x6a),
                8 => (0x800a, 0x6a),
                0..=2 => (0x8006, 0x69 + rng.below(16)),
                3 => (0x800a, 0x69 + rng.below(16)),
                4 => (0x8011, rng.next()),
                5 => (l6_msg(&mut rng), rng.word()),
                _ => ([0x8001, 0x8003][rng.below(2) as usize], rng.next()),
            };
            let window = rng.next();
            l6_fill_stack(t, &mut rng, 0x300);
            let ebp = frame_ebp(&t.a, 3);
            t.b_top = Some(ebp - l66::bridge::FP_FRAME - 0x40);
            t.check(
                "rcuiFpProc_00933cc0",
                0x0093_3cc0,
                Conv::Cdecl,
                &[window, msg, param],
                |o| l6_out(l6m6::fp_proc(o, ebp, window, msg, param)),
            );
            l6_same_words(t, "rcuiFpProc_00933cc0", ebp, ebp - 0x14, 0x10);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 6 (left6) batch 6 (ffx_prender::l6_map7) ----

use ffx_prender::l6_map7::{self as l67, bridge as l6m7};

#[test]
fn l6_load_proc_matches_the_original() {
    // no work area or system; the mouse in and out of the list, past its
    // end; a directory then a map (text or chunk, chunk made or there,
    // its load failing); the draw cut at the list's end; window sizes
    // giving no columns or rows
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6bb1);
        for case in 0..300_u32 {
            let w = if case % 41 == 3 {
                0
            } else {
                l6_random(t, &mut rng, 0xd00)
            };
            let n = rng.below(12);
            let mut names = vec![];
            for k in 0..12_u32 {
                let s = format!("e{k}_{}\0", rng.below(100));
                names.push(t.alloc(s.as_bytes()));
            }
            let names = t.words(&names);
            if w != 0 {
                t.w32(w + 0x68, n);
                t.w32(w + 0x70, names);
                t.w8(w + 0x274, u8::from(rng.chance(2)) * b'd');
                t.w8(w + 0x74, b'b');
                t.w8(w + 0x75, 0);
                t.w8(w + 0x174, b'm');
                t.w8(w + 0x175, 0);
                t.w32(w + 0x60, u32::from(rng.chance(2)));
            }
            t.global(l67::RCUI_WORK, w);
            let chunk = t.words(&[rng.next(), 0]);
            let sys_v = if case % 13 == 5 {
                0
            } else {
                t.words(&[0, 0, 0, 0, 0, 0, if rng.chance(2) { 0 } else { chunk }])
            };
            t.global(l67::YN_MAP_SYSTEM, sys_v);
            let width = [0, 79, 80, 240, 400][rng.below(5) as usize];
            let height = [0, 10, 20, 45, 100][rng.below(5) as usize];
            t.stub_sym(l67::GET_WIDTH, 1, false, 1, 0, Ret::Val(width));
            t.stub_sym(l67::GET_HEIGHT, 2, false, 1, 0, Ret::Val(height));
            let mx = [0xffff_fff0, 0, 79, 80, 170, 300][rng.below(6) as usize];
            let my = [0xffff_fff0, 5, 10, 25, 99][rng.below(5) as usize];
            t.stub_sym(l67::MOUSE_WINDOW_Y, 3, false, 1, 0, Ret::Val(mx));
            t.stub_sym(l67::MOUSE_WINDOW_X, 4, false, 1, 0, Ret::Val(my));
            for (sym, tag, k) in [
                (l67::MAKE_BASE_PATH, 5, 0),
                (l67::LOAD_SET_TITLE, 6, 0),
                (l67::MAKE_FULL_PATH, 8, 0),
                (l67::RESET, 9, 0),
                (l67::PRE_LOAD_TEXT, 10, 0),
                (l67::PRE_BG_SET_DATA, 13, 1),
                (l67::RC_FREE, 14, 1),
                (l67::ID_LOAD, 16, 0),
                (l67::RESET_CHR_POS, 17, 0),
                (l67::CLOSE_WINDOW, 18, 1),
                (l67::PRINT_SET_X, 19, 1),
                (l67::PRINT_SET_Y, 20, 1),
                (l67::PRINT_SET_BOLD, 21, 1),
            ] {
                t.stub_sym(sym, tag, false, k, 0, Ret::Val(rng.next()));
            }
            l6_copy(t, l67::MAKE_FILE_LIST, 7, 2, 1, 2, rng.next());
            t.stub_sym(l67::CHUNK_NEW, 11, false, 0, 0, Ret::Val(chunk));
            let ok = u32::from(rng.chance(2));
            l6_copy(t, l67::CHUNK_LOAD, 12, 2, 1, 2, ok);
            t.stub_sym(l67::SET_PTR32, 15, false, 1, 0, Ret::Val(rng.next()));
            l6_printf(t, l67::GU_PRINTF, 22, &[], Ret::Val(rng.next()));
            let msg = match rng.below(8) {
                0..=2 => 0x8006,
                3 | 4 => 0x8011,
                5 => [0x8001, 0x800a, 0x800e, 0x8003][rng.below(4) as usize],
                _ => l6_msg(&mut rng),
            };
            let window = rng.next();
            let param = rng.next();
            l6_fill_stack(t, &mut rng, 0x400);
            let ebp = frame_ebp(&t.a, 3);
            t.b_top = Some(ebp - l67::LOAD_FRAME - 0x60);
            t.check(
                "rcuiLoadProc",
                0x0093_4bc0,
                Conv::Cdecl,
                &[window, msg, param],
                |o| Out::U32(l6m7::load_proc(o, ebp, window, msg)),
            );
            l6_same_words(t, "rcuiLoadProc", ebp, ebp - 0x11c, 0x118);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn l6_layer_rect_proc_matches_the_original() {
    // no work area, system, map or layers; every button with and without
    // an entry open; the presets taken by either take and restored; ftol
    // both ways (the CRT's SSE2 flag)
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6bc1);
        for case in 0..300_u32 {
            let w = if case % 29 == 3 {
                0
            } else {
                l6_random(t, &mut rng, 0x500)
            };
            let k = rng.below(4);
            if w != 0 {
                t.w32(w + 0x288, k);
                t.w32(w + 0x398, if rng.chance(3) { rng.next() | 2 } else { 0 });
                t.w32(w + 0x3d8, 0x38 + rng.below(8));
                for off in (0x3b8..0x3d8).step_by(4) {
                    t.w32(w + off, float_word(&mut rng));
                }
            }
            t.global(l67::RCUI_WORK, w);
            let layers = l6_random(t, &mut rng, 0xc0 * 4);
            for j in 0..4 {
                for off in (0x14..0x24).step_by(4) {
                    let v = if rng.chance(5) {
                        float_word(&mut rng)
                    } else {
                        (rng.below(4000) as f32 / 8.0 - 250.0).to_bits()
                    };
                    t.w32(layers + 0xc0 * j + off, v);
                }
            }
            let n = [0, 0xffff_ffff, 4][rng.below(3) as usize];
            let list = t.words(&[0, if case % 5 == 0 { n } else { 4 }, layers]);
            let map = t.words(&[0, 0, list]);
            l6_map_system(t, &mut rng, map);
            if case % 2 == 1 {
                t.global(ffx_bridge::x87::SSE2_AVAILABLE.sym, 1);
            }
            for (sym, tag, n) in [
                (l67::bridge::ADD_ITEMS, 1, 2),
                (l67::bridge::NUM_WINDOW_CREATE, 2, 4),
                (l67::bridge::PACKET_BEGIN_TAG, 6, 1),
                (l67::bridge::SET_PRIM, 7, 1),
                (l67::bridge::SET_RGBAQ, 8, 4),
                (l67::bridge::SET_XYZF2, 9, 2),
                (l67::bridge::PACKET_END_TAG, 10, 0),
            ] {
                t.stub_sym(sym, tag, false, n, 0, Ret::Val(rng.next()));
            }
            for (sym, tag) in [
                (l67::bridge::TAKE_14, 3),
                (l67::bridge::TAKE_1C, 4),
                (l67::bridge::TAKE_18, 5),
            ] {
                let v = if rng.chance(2) { 0 } else { rng.next() };
                t.stub_sym(sym, tag, false, 1, 0, Ret::Val(v));
            }
            l6_copy(t, l67::bridge::SET_TEXT, 11, 3, 2, 3, rng.next());
            let (msg, param) = match rng.below(7) {
                0..=2 => (0x8006, 0x35 + rng.below(11)),
                3 | 4 => (0x8011, rng.next()),
                5 => (l6_msg(&mut rng), rng.next()),
                _ => ([0x8001, 0x8003][rng.below(2) as usize], rng.next()),
            };
            let window = rng.next();
            l6_fill_stack(t, &mut rng, 0x300);
            let ebp = frame_ebp(&t.a, 3);
            t.b_top = Some(ebp - l67::bridge::RECT_FRAME - 0x40);
            t.check(
                "rcuiLayerRectProc_00932a60",
                0x0093_2a60,
                Conv::Cdecl,
                &[window, msg, param],
                |o| l6_out(l6m7::rect_proc(o, ebp, window, msg, param)),
            );
            l6_same_words(t, "rcuiLayerRectProc_00932a60", ebp, ebp - 0x18, 0x14);
            t.b_top = None;
            t.reset();
        }
    });
}
