//! `ffx_glue_other::m5v_*` (M5 stage 1, list `vfx`) against the original:
//! on the image alone with synthetic objects, and on the snapshots
//! (`FFX_SNAPSHOT`) where the game's own objects are worth running on.
//! Results and every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! None of these functions ran in a coverage run, so every branch is
//! reached with synthetic data; a test says which ones the snapshots
//! reach too. Callees that stay original are replaced in both oracles
//! alike by recording stubs: each call appends its tag, `ecx` (for
//! methods) and its arguments to a log in memory, so the diff checks that
//! both made the same calls in the same order. A stub pops what the
//! original pops. The harness is `m5_render.rs`'s.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::items_after_statements,
    dead_code
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_glue_other::m5v_a::{self as ma, bridge as a};
use ffx_glue_other::m5v_b::{self as mb, bridge as b};
use ffx_glue_other::m5v_c::{self as mc, bridge as c};
use ffx_glue_other::m5v_d::{self as md, bridge as d};
use ffx_glue_other::m5v_e::{self as me, bridge as e};
use ffx_glue_other::m5v_f::{self as mf, bridge as f};
use ffx_glue_other::m5v_g::{self as mg, bridge as vg};
use ffx_glue_other::m5v_h::{self as mh, bridge as gh};
use ffx_glue_other::m5v_i::{self as mi, bridge as gi};
use ffx_glue_other::m5v_j::{self as mj, bridge as gj};
use ffx_glue_other::m5v_k::{self as mk, bridge as gk};
use ffx_glue_other::m5v_l::{self as ml, bridge as gl};
use ffx_glue_other::m5v_m::{self as mm, bridge as gm2};
use ffx_glue_other::m5v_n::{self as mn, bridge as gn};
use ffx_glue_other::m5v_o::{self as mo, bridge as go};
use ffx_glue_other::m5v_p::{self as mp, bridge as gp};
use ffx_glue_other::m5v_q::{self as mq, bridge as gq};
use ffx_glue_other::m5v_r::{self as mr, bridge as gr};
use ffx_glue_other::m5v_s::{self as ms, bridge as gs};
use ffx_glue_other::m5v_t::{self as mt, bridge as gt};
use ffx_glue_other::m5v_u::{self as mu, bridge as gu};
use ffx_glue_other::m5v_v::{self as mv, bridge as gv};
use ffx_glue_other::m5v_w::{self as mw, bridge as gw};
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};

/// The body of `FUN_006cbf20` (unreachable: `tools/reach.py --why`): the
/// recording stubs are written over it.
const CAVE: u32 = 0x006c_bf20;
const CAVE_LEN: u32 = 1800;
/// After the stubs: `mov eax, POISON; ret`, called before each original
/// callee so a callee leaving `eax` alone shows the poison.
const POISON_AT: u32 = CAVE + CAVE_LEN;
const POISON: u32 = 0xdead_beef;
/// After the poison (`+8`): `fld qword [esp+4]; call [esp+0xc]; ret`, for
/// [`Host::call_intrinsic`] (the operand in `ST0`).
const TRAMPOLINE: [u8; 9] = [0xdd, 0x44, 0x24, 0x04, 0xff, 0x54, 0x24, 0x0c, 0xc3];

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
    fn call_intrinsic(&mut self, addr: u32, x: f64) -> f32 {
        let b = x.to_bits();
        let r = self
            .0
            .call(self.1 + 8, Conv::Cdecl, &[b as u32, (b >> 32) as u32, addr])
            .unwrap_or_else(|e| panic!("intrinsic {addr:#x}: {e}"));
        r.st0_f32().expect("the intrinsic's ST0")
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
    /// Code the next stub runs after logging, before it returns.
    extra: Vec<u8>,
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
            extra: Vec::new(),
        };
        t.log = t.zeros(0x1_0000);
        let at = t.rebase(POISON_AT);
        let mut code = vec![0xb8];
        code.extend(POISON.to_le_bytes());
        code.push(0xc3);
        t.write(at, &code);
        t.write(at + 8, &TRAMPOLINE);
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
        c.extend(std::mem::take(&mut self.extra));
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
                let from: u32 = std::env::var("M5_LOG_FROM")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                for k in from..n.min(from + 64) {
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

impl Twins {
    /// [`Twins::stub_sym`] whose stub runs `extra` (machine code) after
    /// logging and setting `eax`.
    #[allow(clippy::too_many_arguments)]
    fn stub_sym_x(
        &mut self,
        sym: u32,
        tag: u32,
        this: bool,
        n: u32,
        pops: u32,
        ret: Ret,
        extra: &[u8],
    ) -> u32 {
        self.extra = extra.to_vec();
        self.stub_sym(sym, tag, this, n, pops, ret)
    }

    /// A recording stub for a virtual method (`ecx` logged, `n` words
    /// logged and popped).
    fn method_stub(&mut self, tag: u32, n: u32, ret: Ret) -> u32 {
        self.stub_code(tag, true, n, n, ret, false)
    }

    /// Zeroes both oracles' stacks below the call top (frame words the
    /// originals never write read as 0, as in the shims' zeroed frames).
    fn zero_frames(&mut self) {
        let top = call_top(&self.a);
        let z = vec![0; 0x400];
        self.a.write(top - 0x400, &z).unwrap();
        self.b.write(top - 0x400, &z).unwrap();
    }

    /// The argument slots of a call with `args` from the call top: `b`
    /// gets the words the original's call puts there (the bridge makes no
    /// call), and its own calls go below them.
    fn arg_slots(&mut self, args: &[u32]) -> u32 {
        let slots = first_slot(&self.a, args.len() as u32);
        let bytes: Vec<u8> = args.iter().flat_map(|v| v.to_le_bytes()).collect();
        self.b.write(slots, &bytes).unwrap();
        self.b_top = Some(slots - 0x40);
        slots
    }

    /// Asserts both oracles hold the same bytes at `at..at + n` (frames and
    /// argument slots, which `diff` leaves out with the stacks).
    fn same_stack(&self, what: &str, at: u32, n: usize) {
        let mut x = vec![0; n];
        let mut y = vec![0; n];
        Mem::read_bytes(&self.a, at, &mut x);
        Mem::read_bytes(&self.b, at, &mut y);
        assert_eq!(x, y, "{what}: the stack at {at:#x}");
    }
}

/// The original's `ebp` for a function that starts `push ebp; mov ebp,
/// esp`, called from the oracle's call top with `words` stack words.
fn frame_ebp(o: &Oracle, words: u32) -> u32 {
    call_top(o) - 4 * words - 8
}

/// The argument slot of a call's first word from the oracle's call top.
fn first_slot(o: &Oracle, words: u32) -> u32 {
    call_top(o) - 4 * words
}

/// A float word: anything but a signalling NaN.
fn float_word(rng: &mut Rng) -> u32 {
    loop {
        let x = match rng.below(4) {
            0 => 0x7fc0_0000 | rng.below(16),
            1 => (rng.below(20) as f32 - 10.0).to_bits(),
            2 => (-1.0_f32).to_bits(),
            _ => rng.next(),
        };
        let snan = x & 0x7f80_0000 == 0x7f80_0000 && x & 0x007f_ffff != 0 && x & 0x0040_0000 == 0;
        if !snan {
            return x;
        }
    }
}

// ---- batch 1 (ffx_glue_other::m5v_a) ----

#[test]
fn op_loop_end_ffx_matches_the_original() {
    // both branches: the debug byte clear, and set (a jump to a bare `ret`)
    Twins::each(|t| {
        for mode in [0_u8, 1, 0x80] {
            let at = t.rebase(ma::OP_DEBUG_MODE.sym);
            t.patch(at, &[mode]);
            t.check("op_loop_end_ffx", 0x007f_f670, Conv::Cdecl, &[], |g| {
                a::op_loop_end_ffx(g);
                Out::None
            });
            t.reset();
        }
    });
}

#[test]
fn singleton_destroys_match_the_original() {
    // each destroy with its global 0 and set (synthetic; on the snapshots
    // the game's own objects every second round, under stubs)
    Twins::each(|t| {
        for case in 0..6_u32 {
            let real = t.snapshot && case % 2 == 1;
            let set = |t: &mut Twins, g: ffx_bridge::Global<u32>| {
                if !real {
                    let v = if case % 3 == 0 { 0 } else { t.zeros(0x40) };
                    t.global(g.sym, v);
                }
            };
            set(t, ma::GRAPHIC_UI);
            t.stub_sym(ma::GRAPHIC_UI_DTOR, 1, true, 0, 0, Ret::Val(0x11));
            t.stub_sym(ma::OPERATOR_DELETE, 2, false, 1, 0, Ret::Val(0x22));
            t.check(
                "GraphicUi_Destroy_0067b820",
                0x0067_b820,
                Conv::Cdecl,
                &[],
                |g| {
                    a::graphic_ui_destroy(g);
                    Out::None
                },
            );
            t.reset();

            set(t, ma::VFX_RENDER_DATA_TABLE);
            t.stub_sym(ma::VFX_RENDER_DATA_TABLE_DTOR, 1, true, 0, 0, Ret::Val(1));
            t.stub_sym(ma::OPERATOR_DELETE, 2, false, 1, 0, Ret::Val(2));
            t.check(
                "VFXRenderDataTable_Destroy_0069f5c0",
                0x0069_f5c0,
                Conv::Cdecl,
                &[],
                |g| {
                    a::vfx_render_data_table_destroy(g);
                    Out::None
                },
            );
            t.reset();

            set(t, ma::VFX_SHADER_MANAGER);
            t.stub_sym(ma::VFX_SHADER_MANAGER_DTOR, 1, true, 0, 0, Ret::Val(1));
            t.stub_sym(ma::OPERATOR_DELETE, 2, false, 1, 0, Ret::Val(2));
            t.check(
                "VfxShaderManager_Destroy_006c14a0",
                0x006c_14a0,
                Conv::Cdecl,
                &[],
                |g| {
                    a::vfx_shader_manager_destroy(g);
                    Out::None
                },
            );
            t.reset();

            set(t, ma::BROKEN_SCREEN_EFFECT);
            t.stub_sym(ma::BROKEN_SCREEN_RELEASE, 1, true, 0, 0, Ret::Val(1));
            t.stub_sym(ma::OPERATOR_DELETE, 2, false, 1, 0, Ret::Val(2));
            t.check(
                "BrokenScreenEffect_Destroy_006d4db0",
                0x006d_4db0,
                Conv::Cdecl,
                &[],
                |g| {
                    a::broken_screen_destroy(g);
                    Out::None
                },
            );
            t.reset();

            // the character shader renderer: with and without the +8 object
            if real {
                // the game's +8 object: its destructor as a stub
                let r: u32 = ma::CHR_SHADER_RENDER.get(&t.b);
                if r != 0 && t.r32(r + 8) != 0 {
                    let m = t.method_stub(3, 1, Ret::Val(3));
                    let obj = t.object(&[m], 0x10);
                    t.patch(r + 8, &obj.to_le_bytes());
                }
            } else {
                let r = if case % 3 == 0 { 0 } else { t.zeros(0x40) };
                if r != 0 && case % 2 == 0 {
                    let m = t.method_stub(3, 1, Ret::Val(3));
                    let obj = t.object(&[m], 0x10);
                    t.w32(r + 8, obj);
                }
                t.global(ma::CHR_SHADER_RENDER.sym, r);
            }
            t.stub_sym(ma::CHR_SHADER_FREE_BUFFERS, 1, true, 0, 0, Ret::Val(1));
            t.stub_sym(ma::OPERATOR_DELETE, 2, false, 1, 0, Ret::Val(2));
            t.check(
                "ChrShaderRender_Destroy_0069f540",
                0x0069_f540,
                Conv::Cdecl,
                &[],
                |g| {
                    a::chr_shader_destroy(g);
                    Out::None
                },
            );
            t.reset();

            // the VFX render map: the erase's out pointer is in the frame
            if !real {
                let map = if case % 3 == 0 {
                    0
                } else {
                    let head = t.words(&[0x1234, 0, 0]);
                    t.words(&[head, 0])
                };
                t.global(ma::VFX_RENDER_MAP.sym, map);
            }
            t.stub_sym(ma::TREE_ERASE_RANGE_1660, 1, true, 3, 3, Ret::Val(1));
            t.stub_sym(ma::OPERATOR_DELETE, 2, false, 1, 0, Ret::Val(2));
            let ebp = frame_ebp(&t.a, 0);
            t.b_top = Some(ebp - 0x40);
            t.check(
                "VFXRenderMap_Destroy_0069f580",
                0x0069_f580,
                Conv::Cdecl,
                &[],
                |g| {
                    a::vfx_render_map_destroy(g, ebp);
                    Out::None
                },
            );
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn create_instances_match_the_original() {
    // the global set already; the allocation failing; the object built
    Twins::each(|t| {
        for case in 0..3_u32 {
            for (sym, global, ctor) in [
                (0x0067_b6c0_u32, ma::GRAPHIC_UI, ma::GRAPHIC_UI_CTOR),
                (
                    0x0067_b740,
                    ma::MENU_TEXTURE_MANAGER,
                    ma::MENU_TEXTURE_MANAGER_CTOR,
                ),
            ] {
                t.global(global.sym, if case == 0 { 0x5555 } else { 0 });
                let mem = if case == 1 { 0 } else { t.zeros(0x20) };
                t.stub_sym(ma::OPERATOR_NEW, 1, false, 1, 0, Ret::Val(mem));
                t.stub_sym(ctor, 2, true, 0, 0, Ret::Val(mem + 0x10));
                t.check("CreateInstance", sym, Conv::Cdecl, &[], |g| {
                    if sym == 0x0067_b6c0 {
                        a::graphic_ui_create_instance(g);
                    } else {
                        a::menu_texture_manager_create_instance(g);
                    }
                    Out::None
                });
                t.reset();
            }
        }
    });
}

#[test]
fn graphic_ui_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0101);
        // the buffers' allocation and clear
        for _ in 0..3 {
            let ui = t.zeros(0x5ba0);
            let r = t.seq(&[0x1000, 0x2000]);
            t.stub_sym(ma::OPERATOR_NEW_ARRAY, 1, false, 1, 0, r);
            let r = t.seq(&[0x3000, 0x4000]);
            t.stub_sym(ma::MEMSET, 2, false, 3, 0, r);
            t.check(
                "GraphicUi_AllocBuffers_0067b770",
                0x0067_b770,
                Conv::Thiscall(ui),
                &[],
                |g| Out::U32(a::graphic_ui_alloc_buffers(g, ui)),
            );
            t.reset();
        }
        // the slot objects: empty slots, slots with an object, and the
        // second word set with an object (only the second word set reads
        // a vtable at 0 in the original: left out)
        for _ in 0..4 {
            let ui = t.zeros(0x5840);
            let m = t.method_stub(1, 1, Ret::Val(7));
            for (field, bytes) in [
                (0x2bd4_u32, 0x2580_u32),
                (0x2bdc, 0x3200),
                (0x582c, 0x2580),
                (0x5834, 0x3200),
            ] {
                let slots = t.zeros(bytes as usize);
                for _ in 0..rng.below(6) {
                    let i = rng.below(bytes / 0x10);
                    let obj = t.object(&[m], 0x10);
                    t.w32(slots + 0x10 * i, obj);
                    if rng.chance(2) {
                        let v = rng.next() | 1;
                        t.w32(slots + 0x10 * i + 4, v);
                    }
                }
                t.w32(ui + field, slots);
            }
            t.check(
                "GraphicUi_ReleaseSlotObjects_0067b850",
                0x0067_b850,
                Conv::Thiscall(ui),
                &[],
                |g| {
                    a::graphic_ui_release_slot_objects(g, ui);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

#[test]
fn chr_shader_free_buffers_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0102);
        for _ in 0..4 {
            let mut bufs = vec![];
            for _ in 0..2 {
                let rec = t.zeros(0x20);
                for off in [0xc_u32, 0x10, 0x14, 0x18, 0x1c] {
                    let v = rng.word();
                    t.w32(rec + off, v);
                }
                let b = t.zeros(0x98);
                t.w32(b + 0x94, rec);
                bufs.push(b);
            }
            let r = t.words(&bufs);
            t.stub_sym(ma::OPERATOR_DELETE_ARRAY, 1, false, 1, 0, Ret::Val(1));
            t.stub_sym(ma::OPERATOR_DELETE, 2, false, 1, 0, Ret::Val(2));
            t.check(
                "ChrShaderRender_FreeBuffers_006a0070",
                0x006a_0070,
                Conv::Thiscall(r),
                &[],
                |g| {
                    a::chr_shader_free_buffers(g, r);
                    Out::None
                },
            );
            assert_eq!(t.calls(), 14);
            t.reset();
        }
    });
}

#[test]
fn vfx_render_data_table_dtor_matches_the_original() {
    // each array owned or not (top bit), with and without data
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0103);
        for _ in 0..24 {
            let table = t.zeros(0x30);
            let head = t.words(&[0x4321, 0, 0]);
            t.w32(table, head);
            for off in [8_u32, 0x10, 0x18] {
                let n = if rng.chance(2) {
                    [0x8000_0000, 0x4000_0000, 0xc000_0000][rng.below(3) as usize] | rng.below(4)
                } else {
                    rng.below(4)
                };
                t.w32(table + off, n);
                let p = if rng.chance(3) { 0 } else { rng.next() };
                t.w32(table + off + 4, p);
            }
            t.stub_sym(ma::PHYRE_FREE, 1, false, 1, 0, Ret::Val(1));
            t.stub_sym(ma::TREE_ERASE_RANGE_19D0, 2, true, 3, 3, Ret::Val(2));
            t.stub_sym(ma::OPERATOR_DELETE, 3, false, 1, 0, Ret::Val(3));
            let ebp = frame_ebp(&t.a, 0);
            t.b_top = Some(ebp - 0x40);
            t.check(
                "VFXRenderDataTable_dtor_0069e900",
                0x0069_e900,
                Conv::Thiscall(table),
                &[],
                |g| {
                    a::vfx_render_data_table_dtor(g, ebp, table);
                    Out::None
                },
            );
            t.same_stack("VFXRenderDataTable_dtor_0069e900", ebp - 0x10, 4);
            t.b_top = None;
            t.reset();

            // the plain map destructor
            let map = t.words(&[head]);
            t.stub_sym(ma::TREE_ERASE_RANGE_19D0, 2, true, 3, 3, Ret::Val(2));
            t.stub_sym(ma::OPERATOR_DELETE, 3, false, 1, 0, Ret::Val(3));
            t.b_top = Some(ebp - 0x40);
            t.check(
                "std_Map_dtor_0069e770",
                0x0069_e770,
                Conv::Thiscall(map),
                &[],
                |g| {
                    a::std_map_dtor(g, ebp, map);
                    Out::None
                },
            );
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn shader_manager_teardown_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0104);
        // the tree + free list clear: chains of 0..4 nodes, each with its
        // +0xc word's low bit set or clear, ended by the tree or 0; the
        // remove stub unlinks the node (tree +0xc = node +0x10)
        for _ in 0..20 {
            let tree = t.zeros(0x30);
            let n = rng.below(5);
            let mut next = if rng.chance(2) { tree } else { 0 };
            for _ in 0..n {
                let node = t.zeros(0x20);
                let w = rng.next();
                t.w32(node + 0xc, w);
                t.w32(node + 0x10, next);
                next = node;
            }
            t.w32(tree + 0xc, next);
            // mov edx, [esp+4]; mov edx, [edx+0x10]; mov [ecx+0xc], edx
            t.stub_sym_x(
                ma::TREE_REMOVE,
                1,
                true,
                1,
                1,
                Ret::Val(1),
                &[0x8b, 0x54, 0x24, 0x04, 0x8b, 0x52, 0x10, 0x89, 0x51, 0x0c],
            );
            t.stub_sym(ma::PHYRE_FREE, 2, false, 1, 0, Ret::Val(2));
            t.stub_sym(ma::FREE_LIST_RESET, 3, true, 0, 0, Ret::Val(0x33));
            t.check(
                "PTreeFreeList_clear_006c0520",
                0x006c_0520,
                Conv::Thiscall(tree),
                &[],
                |g| Out::U32(a::tree_free_list_clear(g, tree)),
            );
            t.reset();
        }
        // the destructor: every callee a stub
        let m = t.zeros(0xa0);
        for (k, sym) in [
            ma::VFX_SHADER_RELEASE_CLUSTERS,
            ma::TREE_FREE_LIST_CLEAR,
            ma::FREE_LIST_DTOR,
            ma::TREE_CLEAR,
            ma::TREE_ORPHAN_ALL,
            ma::FREE_LIST_RESET,
        ]
        .into_iter()
        .enumerate()
        {
            t.stub_sym(sym, k as u32 + 1, true, 0, 0, Ret::Val(k as u32));
        }
        t.check(
            "VfxShaderManager_dtor_006bfaa0",
            0x006b_faa0,
            Conv::Thiscall(m),
            &[],
            |g| {
                a::vfx_shader_manager_dtor(g, m);
                Out::None
            },
        );
        assert_eq!(t.calls(), 14);
        t.reset();
        // the plain tree destructor
        t.stub_sym(ma::TREE_CLEAR, 1, true, 0, 0, Ret::Val(1));
        t.check(
            "PRedBlackTree_dtor_0069e500",
            0x0069_e500,
            Conv::Thiscall(m),
            &[],
            |g| {
                a::tree_dtor(g, m);
                Out::None
            },
        );
        t.reset();
    });
}

#[test]
fn vfx_events_match_the_original() {
    // the frame skipped or not; no character; the flag's byte
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0105);
        for case in 0..12_u32 {
            let scene = t.zeros(0x10);
            t.global(ma::SCENE.sym, scene);
            let skip = if case % 4 == 0 { rng.next() | 1 } else { 0 };
            t.global(ma::SKIP_FRAME.sym, skip);
            let (x, y) = (rng.word(), rng.word());
            for (sym, clear, rgb) in [
                (0x0064_4be0_u32, 0, 0),
                (0x0064_4c40, 1, 1),
                (0x0064_4c70, 0, 1),
            ] {
                t.stub_sym(ma::SCENE_ADD_COPY_BUFFER_EVENT, 1, true, 4, 4, Ret::Val(1));
                t.check(
                    "graphicVFXAddCopyBufferEvent*",
                    sym,
                    Conv::Cdecl,
                    &[x, y],
                    |g| {
                        a::add_copy_buffer_event(g, x, y, clear, rgb);
                        Out::None
                    },
                );
            }
            let chr = if case % 3 == 0 { 0 } else { rng.next() };
            let flag = rng.next();
            // the event the callee gets is in the frame: compared after
            t.zero_frames();
            t.stub_sym(ma::SCENE_ADD_VFX_DRAW_EVENT, 2, true, 1, 1, Ret::Val(2));
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - 0x40);
            t.check(
                "graphicVFXAddDrawCharacterEvent",
                0x0064_4ca0,
                Conv::Cdecl,
                &[chr, flag],
                |g| {
                    a::add_draw_event(g, ebp, 5, chr, Some(flag as u8));
                    Out::None
                },
            );
            t.same_stack("graphicVFXAddDrawCharacterEvent", ebp - 0x14, 0x10);
            t.zero_frames();
            let ebp = frame_ebp(&t.a, 1);
            t.b_top = Some(ebp - 0x40);
            t.check(
                "graphicVFXAddDrawZPrePassEvent",
                0x0064_4cf0,
                Conv::Cdecl,
                &[chr],
                |g| {
                    a::add_draw_event(g, ebp, 6, chr, None);
                    Out::None
                },
            );
            t.same_stack("graphicVFXAddDrawZPrePassEvent", ebp - 0x14, 0x10);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn parray_push_back_matches_the_original() {
    // under and over capacity (signed), the slot address 0 (no data, one
    // element), negative counts
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0106);
        for case in 0..62 {
            let array = t.zeros(0x10);
            let n = match rng.below(4) {
                0 => rng.below(40),
                1 => 0,
                2 => (rng.below(4) as i32 - 2) as u32,
                _ => rng.below(200),
            };
            let cap = match rng.below(3) {
                0 => n.wrapping_add(rng.below(3)),
                1 => (rng.below(4) as i32 - 2) as u32,
                _ => rng.below(300),
            };
            let mut data = if case % 7 == 0 && n == 0 {
                0
            } else {
                t.zeros(0x400) + 0x200
            };
            // a count of -2 over a capacity of -2 (the signed growth), and
            // a slot address wrapping to 0 (data -4, one element after)
            let (n, cap) = match case {
                60 => (0xffff_fffe, 0xffff_fffe),
                61 => {
                    data = 0xffff_fffc;
                    (1, 8)
                }
                _ => (n, cap),
            };
            t.w32(array, n);
            t.w32(array + 4, cap);
            t.w32(array + 8, data);
            let v = rng.next();
            let value = t.words(&[v]);
            t.stub_sym(ma::ARRAY_RESERVE, 1, true, 1, 1, Ret::Val(9));
            t.check(
                "PArray_PushBack_006560c0",
                0x0065_60c0,
                Conv::Thiscall(array),
                &[value],
                |g| Out::U32(a::parray_push_back(g, array, value)),
            );
            t.reset();
        }
    });
}

#[test]
fn compare_depth_matches_the_original() {
    // null geometry either side, NaNs, equal depths; the argument slots
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0107);
        for _ in 0..200 {
            let mut geo = |t: &mut Twins| {
                if rng.chance(4) {
                    0
                } else {
                    let d = float_word(&mut rng);
                    let g = t.zeros(0xe0);
                    t.w32(g + 0xdc, d);
                    g
                }
            };
            let (ga, gb) = (geo(t), geo(t));
            if ga != 0 && gb != 0 && rng.chance(4) {
                let d = t.r32(ga + 0xdc);
                t.w32(gb + 0xdc, d);
            }
            let (pa, pb) = (t.words(&[ga]), t.words(&[gb]));
            let slots = t.arg_slots(&[pa, pb]);
            t.check(
                "VFXGeometry_CompareDepth",
                0x0065_9140,
                Conv::Cdecl,
                &[pa, pb],
                |g| Out::U32(a::compare_depth(g, slots, pa, pb) as u32),
            );
            t.same_stack("VFXGeometry_CompareDepth", slots, 8);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn small_releases_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0108);
        for _ in 0..40 {
            // the Phyre arrays
            for (sym, nonnull) in [(0x0069_e4a0_u32, false), (0x0069_e4d0, true)] {
                let n = if rng.chance(2) {
                    [0x8000_0000, 0x4000_0000, 0xc000_0000][rng.below(3) as usize] | rng.below(4)
                } else {
                    rng.below(4)
                };
                let p = if rng.chance(3) { 0 } else { rng.next() };
                let arr = t.words(&[n, p]);
                t.stub_sym(ma::PHYRE_FREE, 1, false, 1, 0, Ret::Val(1));
                t.check("PArray_release*", sym, Conv::Thiscall(arr), &[], |g| {
                    a::parray_release(g, arr, nonnull);
                    Out::None
                });
                t.reset();
            }
            // the string
            let cap = if rng.chance(4) { 0x10 } else { rng.below(0x20) };
            let words = [
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                cap,
            ];
            let s = t.words(&words);
            t.stub_sym(ma::OPERATOR_DELETE, 1, false, 1, 0, Ret::Val(1));
            t.check(
                "std_String_dtor_0069e7d0",
                0x0069_e7d0,
                Conv::Thiscall(s),
                &[],
                |g| {
                    a::std_string_dtor(g, s);
                    Out::None
                },
            );
            t.reset();
            // the character: 0..4 entries (some 0), negative counts, no
            // list (with no entries), no buffer
            let count = match rng.below(4) {
                0 => 0,
                1 => (-(rng.below(3) as i32) - 1) as u32,
                _ => 1 + rng.below(4),
            };
            let entries: Vec<u32> = (0..8)
                .map(|_| if rng.chance(3) { 0 } else { rng.next() })
                .collect();
            let list = if count == 0 && rng.chance(2) {
                0
            } else {
                t.words(&entries)
            };
            let buffer = if rng.chance(3) { 0 } else { rng.next() };
            let words = [0, 0, buffer, rng.next(), rng.next(), list, count];
            let c = t.words(&words);
            t.stub_sym(ma::OPERATOR_DELETE_ARRAY, 1, false, 1, 0, Ret::Val(1));
            t.stub_sym(ma::OPERATOR_DELETE, 2, false, 1, 0, Ret::Val(2));
            t.check(
                "ClassCharacter_dtor_0069e830",
                0x0069_e830,
                Conv::Thiscall(c),
                &[],
                |g| {
                    a::class_character_dtor(g, c);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

#[test]
fn deleting_destructors_match_the_original() {
    Twins::each(|t| {
        for flags in [0_u32, 1, 2, 3, 0xfffc, 0xffff_ffff] {
            let block = t.zeros(0x200);
            t.w32(block, 3);
            let g = block + 4;
            t.stub_sym(ma::EH_VECTOR_DTOR_ITERATOR, 1, false, 4, 4, Ret::Val(1));
            t.stub_sym(ma::OPERATOR_DELETE_ARRAY, 2, false, 1, 0, Ret::Val(2));
            t.stub_sym(ma::OPERATOR_DELETE, 3, false, 1, 0, Ret::Val(3));
            t.stub_sym(ma::DYNAMIC_GEOMETRY_CLEAR, 4, true, 0, 0, Ret::Val(4));
            t.check(
                "ClassDynamicGeometry_vecDelDtor_0069edd0",
                0x0069_edd0,
                Conv::Thiscall(g),
                &[flags],
                |o| Out::U32(a::dynamic_geometry_vec_del_dtor(o, g, flags)),
            );
            for sym in [0x0069_ef00_u32, 0x0069_efa0, 0x0069_efd0] {
                t.check(
                    "ClassDynamicGeometry_scalarDelDtor",
                    sym,
                    Conv::Thiscall(g),
                    &[flags],
                    |o| Out::U32(a::dynamic_geometry_scalar_del_dtor(o, g, flags)),
                );
            }
            t.stub_sym(ma::CLASS_CHARACTER_DTOR, 5, true, 0, 0, Ret::Val(5));
            t.check(
                "ClassCharacter_scalarDelDtor_0069ee90",
                0x0069_ee90,
                Conv::Thiscall(g),
                &[flags],
                |o| Out::U32(a::class_character_scalar_del_dtor(o, g, flags)),
            );
            t.reset();
        }
    });
}

/// A random MSVC `_Tree` of `n` nodes (left `+0`, parent `+4`, right `+8`,
/// nil byte `+0xd`): its head (nil; left the minimum, parent the root,
/// right the maximum) and the nodes.
fn build_tree(t: &mut Twins, rng: &mut Rng, n: usize) -> (u32, Vec<u32>) {
    let head = t.zeros(0x10);
    let nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x10)).collect();
    let mut keys: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        keys.swap(i, rng.below(i as u32 + 1) as usize);
    }
    // left, parent, right as indices (n = head)
    let mut link = vec![[n, n, n]; n];
    let mut root = n;
    for &k in &keys {
        if root == n {
            root = k;
            continue;
        }
        let mut at = root;
        loop {
            let side = if k < at { 0 } else { 2 };
            if link[at][side] == n {
                link[at][side] = k;
                link[k][1] = at;
                break;
            }
            at = link[at][side];
        }
    }
    let addr = |i: usize| if i == n { head } else { nodes[i] };
    for i in 0..n {
        for (w, &l) in link[i].iter().enumerate() {
            t.w32(nodes[i] + 4 * w as u32, addr(l));
        }
        let colour = rng.below(2) as u8;
        t.w8(nodes[i] + 0xc, colour);
    }
    let (min, max) = if n == 0 { (n, n) } else { (0, n - 1) };
    t.w32(head, addr(min));
    t.w32(head + 4, addr(root));
    t.w32(head + 8, addr(max));
    t.w8(head + 0xd, 1);
    (head, nodes)
}

#[test]
fn tree_iterator_dec_matches_the_original() {
    // every node of random trees (the head, the minimum climbing to the
    // head, left subtrees, parent climbs), both copies
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0109);
        for _ in 0..12 {
            let n = rng.below(9) as usize;
            let (head, nodes) = build_tree(t, &mut rng, n);
            for &start in std::iter::once(&head).chain(nodes.iter()) {
                for sym in [0x0069_eca0_u32, 0x0069_ed00] {
                    let it = t.words(&[start]);
                    t.check("std_TreeIterator_dec", sym, Conv::Thiscall(it), &[], |g| {
                        Out::U32(a::tree_iterator_dec(g, it))
                    });
                }
            }
            t.reset();
        }
    });
}

#[test]
fn key64_less_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_010a);
        for _ in 0..200 {
            let (al, ah) = (rng.word(), rng.word());
            let (bl, bh) = match rng.below(3) {
                0 => (rng.word(), ah),
                1 => (al, ah),
                _ => (rng.word(), rng.word()),
            };
            let (pa, pb) = (t.words(&[al, ah]), t.words(&[bl, bh]));
            t.check(
                "Key64_less_0069eda0",
                0x0069_eda0,
                Conv::Stdcall,
                &[pa, pb],
                |g| Out::U32(a::key64_less(g, pa, pb)),
            );
        }
    });
}

// ---- batch 2 (ffx_glue_other::m5v_b) ----

#[test]
fn chr_shader_create_instance_matches_the_original() {
    // the global set; the renderer's allocation failing; the +8 object's
    // failing; both made
    Twins::each(|t| {
        for case in 0..4_u32 {
            t.global(
                mb::CHR_SHADER_RENDER.sym,
                if case == 0 { 0x7777 } else { 0 },
            );
            let r = t.zeros(0x20);
            let p = t.zeros(0x40);
            let news = match case {
                1 => vec![0],
                2 => vec![r, 0],
                _ => vec![r, p],
            };
            let seq = t.seq(&news);
            t.stub_sym(mb::OPERATOR_NEW, 1, false, 1, 0, seq);
            t.stub_sym(mb::VERTEX_INSTANCE_SETUP, 2, true, 0, 0, Ret::Val(2));
            t.stub_sym(mb::SET_FIELDS, 3, true, 0, 0, Ret::Val(p + 4));
            t.stub_sym(mb::CREATE_MESH, 4, true, 0, 0, Ret::Val(4));
            t.check(
                "ChrShaderRender_CreateInstance_0069f3a0",
                0x0069_f3a0,
                Conv::Cdecl,
                &[],
                |g| {
                    b::chr_shader_create_instance(g);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

#[test]
fn chr_shadow_end_matches_the_original() {
    // no object; no mesh (the warning); the remake by both flags; the
    // hide with and without the object's mesh
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0201);
        for case in 0..40_u32 {
            let scene = t.zeros(0xfaa0);
            if rng.chance(2) {
                t.w8(scene + 0xfa9b, 1);
            }
            t.global(mb::SCENE.sym, scene);
            let r = t.zeros(0x20);
            if case % 8 != 0 {
                let m = t.method_stub(1, 1, Ret::Val(1));
                let obj = t.object(&[0, 0, m], 0x30);
                if rng.chance(2) {
                    let mesh = rng.next() | 1;
                    t.w32(obj + 0xc, mesh);
                }
                t.w32(r + 8, obj);
            }
            let index = rng.below(3);
            t.w32(r + 0x10, index);
            // r[2] is the +8 object itself
            for k in 0..2 {
                let mesh = if rng.chance(4) { 0 } else { rng.next() | 1 };
                t.w32(r + 4 * k, mesh);
            }
            if rng.chance(2) {
                t.w8(r + 0xc, 1);
            }
            t.stub_sym(mb::PRINTF, 2, false, 2, 0, Ret::Val(2));
            t.stub_sym(mb::CREATE_MESH, 3, true, 0, 0, Ret::Val(3));
            t.stub_sym(mb::SET_MESH_VISIBLE, 4, true, 2, 2, Ret::Val(4));
            t.check(
                "ChrShaderRender_ShadowEnd_0069fdc0",
                0x0069_fdc0,
                Conv::Thiscall(r),
                &[],
                |g| {
                    b::chr_shadow_end(g, r);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

/// A dynamic geometry with a geometry list of `n` entries (none: no
/// list), each with a `+4` buffer; the list's top bit random.
fn dyn_geometry(t: &mut Twins, rng: &mut Rng, n: Option<u32>) -> u32 {
    let g = t.zeros(0x100);
    let shader = t.zeros(0x10);
    t.w32(g + 0x28, shader);
    if let Some(n) = n {
        let geos: Vec<u32> = (0..n)
            .map(|_| {
                let geo = t.zeros(0x10);
                let buffer = t.zeros(0x10);
                t.w32(geo + 4, buffer);
                geo
            })
            .collect();
        let top = if rng.chance(2) { 0x8000_0000 } else { 0 };
        let list = if n > 1 {
            let data = t.words(&geos);
            t.words(&[n | top, data])
        } else {
            let mut w = vec![n | top];
            w.extend(&geos);
            w.push(0);
            t.words(&w)
        };
        t.w32(g + 0x14, list);
    }
    g
}

#[test]
fn dynamic_geometry_setters_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0202);
        for case in 0..24_u32 {
            let n = match case % 4 {
                0 => None,
                1 => Some(0),
                2 => Some(1),
                _ => Some(2 + rng.below(2)),
            };
            let finds: Vec<u32> = (0..12)
                .map(|_| if rng.chance(3) { 0 } else { rng.next() | 1 })
                .collect();
            // the point light
            let g = dyn_geometry(t, &mut rng, n);
            let m0: Vec<u32> = (0..16).map(|_| rng.next()).collect();
            let m1: Vec<u32> = (0..16).map(|_| rng.next()).collect();
            let (m0, m1) = (t.words(&m0), t.words(&m1));
            let f = float_word(&mut rng);
            let seq = t.seq(&finds);
            t.stub_sym(mb::FIND_PARAM, 1, true, 1, 1, seq);
            t.stub_sym(mb::SET_VALUE2, 2, true, 2, 2, Ret::Val(2));
            let slots = t.arg_slots(&[m0, m1, f]);
            t.check(
                "VFXDynamicGeometry_setPointLightParameters",
                0x006a_2fd0,
                Conv::Thiscall(g),
                &[m0, m1, f],
                |o| {
                    b::set_point_light_parameters(o, slots, g, m0, m1, f);
                    Out::None
                },
            );
            t.same_stack("VFXDynamicGeometry_setPointLightParameters", slots, 4);
            t.b_top = None;
            t.reset();
            // YIQ on and off
            let g = dyn_geometry(t, &mut rng, n);
            let on = if case % 3 == 0 {
                0
            } else {
                rng.next() as u8 | 1
            };
            let seq = t.seq(&finds);
            t.stub_sym(mb::FIND_PARAM, 1, true, 1, 1, seq);
            t.stub_sym(mb::SET_VALUE2, 2, true, 2, 2, Ret::Val(2));
            t.stub_sym(mb::GET_YIQ_PARAM, 3, true, 6, 6, Ret::Val(3));
            let scene = t.zeros(0x10);
            t.global(mb::SCENE.sym, scene);
            t.zero_frames();
            let ebp = frame_ebp(&t.a, 1);
            t.b_top = Some(ebp - 0x80);
            let arg = u32::from(on) | (rng.next() & 0xffff_ff00);
            t.check(
                "VFXDynamicGeometry_setYIQEnable",
                0x006a_3a20,
                Conv::Thiscall(g),
                &[arg],
                |o| {
                    b::set_yiq_enable(o, ebp, g, on);
                    Out::None
                },
            );
            t.same_stack("VFXDynamicGeometry_setYIQEnable", ebp - 0x28, 0x24);
            t.b_top = None;
            t.reset();
            // RGBA and UV
            let v: Vec<u32> = (0..4).map(|_| float_word(&mut rng)).collect();
            let src = t.words(&v);
            let g = t.zeros(0x100);
            t.check(
                "VFXDynamicGeometry_setRGBAPower",
                0x006a_30d0,
                Conv::Thiscall(g),
                &[src],
                |o| {
                    b::set_rgba_power(o, g, src);
                    Out::None
                },
            );
            t.check(
                "VFXDynamicGeometry_setUVOffset",
                0x006a_3a00,
                Conv::Thiscall(g),
                &[src],
                |o| {
                    b::set_uv_offset(o, g, src);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

#[test]
fn shader_manager_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0203);
        for case in 0..30_u32 {
            // the string node copy: no name, a flag bit alone, names of 0..9
            // characters with and without the flag; the allocation failing
            let node = t.zeros(0x10);
            let src = t.zeros(0x10);
            let tagged = match case % 5 {
                0 => 0,
                1 => 1,
                _ => {
                    let len = rng.below(10) as usize;
                    let mut s: Vec<u8> = (0..len).map(|_| b'a' + rng.below(26) as u8).collect();
                    s.push(0);
                    let at = t.alloc(&[s, vec![0; 4]].concat());
                    // the allocation is 4-aligned: the flag bit is free
                    at | u32::from(rng.chance(2))
                }
            };
            t.w32(src + 0xc, tagged);
            let block = if case % 7 == 3 { 0 } else { t.zeros(0x20) };
            t.stub_sym(mb::PHYRE_MALLOC, 1, false, 2, 0, Ret::Val(block));
            t.stub_sym(mb::MEMCPY, 2, false, 3, 0, Ret::Val(block));
            let slots = t.arg_slots(&[src]);
            t.check(
                "VfxShaderNode_copy_006bf170",
                0x006b_f170,
                Conv::Thiscall(node),
                &[src],
                |o| Out::U32(b::shader_node_copy(o, slots, node, src)),
            );
            t.same_stack("VfxShaderNode_copy_006bf170", slots, 4);
            t.b_top = None;
            t.reset();

            // the tree iterator: the first node the tree, 0, or a chain
            let tree = t.zeros(0x20);
            let first = match case % 3 {
                0 => tree,
                1 => 0,
                _ => {
                    let mut next = tree;
                    for _ in 0..=rng.below(4) {
                        let n = t.zeros(0x10);
                        t.w32(n, next);
                        next = n;
                    }
                    next
                }
            };
            t.w32(tree + 0xc, first);
            let it = t.zeros(0xc);
            t.stub_sym(mb::TREE_SUCCESSOR, 1, true, 1, 1, Ret::Val(0x5151));
            t.check(
                "PTreeIterator_begin_006bf2a0",
                0x006b_f2a0,
                Conv::Thiscall(it),
                &[tree],
                |o| Out::U32(b::tree_iterator_begin(o, it, tree)),
            );
            t.reset();

            // the texture parameter
            let buffer = t.zeros(0x10);
            let param = t.zeros(0x10);
            t.w8(
                param + 2,
                if case % 4 == 0 {
                    rng.next() as u8
                } else {
                    0x44
                },
            );
            let value = if case % 5 == 0 {
                0
            } else {
                let v = t.zeros(0x10);
                for off in [8_u32, 0xc] {
                    let w = if rng.chance(4) { 0 } else { rng.next() | 1 };
                    t.w32(v + off, w);
                }
                v
            };
            let check = if rng.chance(2) { 0 } else { 0x77 };
            t.stub_sym(mb::SAMPLER_CHECK, 1, true, 1, 1, Ret::Val(check));
            t.stub_sym(mb::WRITE_PARAMETER, 2, true, 2, 2, Ret::Val(0x88));
            t.check(
                "VfxShader_writeTextureParameter_006c14d0",
                0x006c_14d0,
                Conv::Thiscall(buffer),
                &[param, value],
                |o| Out::U32(b::write_texture_parameter(o, buffer, param, value)),
            );
            t.reset();
        }
    });
}

#[test]
fn shader_tree_destructors_match_the_original() {
    Twins::each(|t| {
        let tr = t.zeros(0x40);
        for (k, sym) in [
            mb::TREE_CLEAR,
            mb::TREE_ORPHAN_ALL,
            mb::FREE_LIST_RESET,
            mb::FREE_LIST_DTOR,
            mb::TREE_FREE_LIST_CLEAR_04E0,
            mb::TREE_FREE_LIST_CLEAR_0520,
        ]
        .into_iter()
        .enumerate()
        {
            t.stub_sym(sym, k as u32 + 1, true, 0, 0, Ret::Val(k as u32));
        }
        for sym in [0x006b_f590_u32, 0x006b_f5f0, 0x006b_f650] {
            t.check("PRedBlackTree_dtor", sym, Conv::Thiscall(tr), &[], |o| {
                b::tree_dtor(o, tr);
                Out::None
            });
        }
        t.check(
            "PTreeFreeList_dtor_006bf6b0",
            0x006b_f6b0,
            Conv::Thiscall(tr),
            &[],
            |o| {
                b::tree_free_list_dtor_orphan(o, tr);
                Out::None
            },
        );
        t.check(
            "PTreeFreeList_dtor_006bf730",
            0x006b_f730,
            Conv::Thiscall(tr),
            &[],
            |o| {
                b::tree_free_list_dtor(o, tr, false);
                Out::None
            },
        );
        t.check(
            "PTreeFreeList_dtor_006bf7a0",
            0x006b_f7a0,
            Conv::Thiscall(tr),
            &[],
            |o| {
                b::tree_free_list_dtor(o, tr, true);
                Out::None
            },
        );
        assert_eq!(t.calls(), 6 + 5 + 4 + 4);
        t.reset();
        // the copy of the tree + free list clear
        let mut rng = Rng(0x5eed_0204);
        for _ in 0..12 {
            let tree = t.zeros(0x30);
            let mut next = if rng.chance(2) { tree } else { 0 };
            for _ in 0..rng.below(4) {
                let node = t.zeros(0x20);
                let w = rng.next();
                t.w32(node + 0xc, w);
                t.w32(node + 0x10, next);
                next = node;
            }
            t.w32(tree + 0xc, next);
            t.stub_sym_x(
                ma::TREE_REMOVE,
                1,
                true,
                1,
                1,
                Ret::Val(1),
                &[0x8b, 0x54, 0x24, 0x04, 0x8b, 0x52, 0x10, 0x89, 0x51, 0x0c],
            );
            t.stub_sym(ma::PHYRE_FREE, 2, false, 1, 0, Ret::Val(2));
            t.stub_sym(ma::FREE_LIST_RESET, 3, true, 0, 0, Ret::Val(0x33));
            t.check(
                "PTreeFreeList_clear_006c04e0",
                0x006c_04e0,
                Conv::Thiscall(tree),
                &[],
                |o| Out::U32(a::tree_free_list_clear(o, tree)),
            );
            t.reset();
        }
    });
}

#[test]
fn broken_screen_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0205);
        for _ in 0..30 {
            let e = t.zeros(0x60);
            for off in (0x20_u32..0x54).step_by(4) {
                let v = if rng.chance(2) { 0 } else { rng.next() };
                t.w32(e + off, v);
            }
            let cluster = t.zeros(0x10);
            t.w32(e + 0x14, cluster);
            t.stub_sym(mb::DELETE_OBJECT, 1, true, 1, 1, Ret::Val(1));
            t.check(
                "BrokenScreenEffect_releaseObjects_006d3e00",
                0x006d_3e00,
                Conv::Thiscall(e),
                &[],
                |o| {
                    b::broken_screen_release_objects(o, e);
                    Out::None
                },
            );
            let kind = match rng.below(4) {
                0 => 6,
                1 => 8,
                2 => rng.below(10),
                _ => rng.next(),
            };
            t.w32(e + 8, kind);
            t.check(
                "BrokenScreen_vertexStride_006d3f50",
                0x006d_3f50,
                Conv::Thiscall(e),
                &[],
                |o| Out::U32(b::vertex_stride(o, e)),
            );
            t.reset();
        }
    });
}

#[test]
fn cluster_textures_match_the_original() {
    // the cluster's list empty (its head) or not; the cube form's
    // parameter kind, sampler and texture 0 or not, the check's answer
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0206);
        for case in 0..32_u32 {
            let cube = case % 2 == 1;
            let cluster = t.zeros(0x40);
            let head = if rng.chance(2) {
                cluster + 0x1c
            } else {
                rng.next()
            };
            t.w32(cluster + 0x1c, head);
            let buffer = t.zeros(0x10);
            let holder = t.words(&[0, buffer]);
            let param = t.zeros(0x10);
            t.w8(param + 2, if rng.chance(3) { 0x12 } else { 0x44 });
            let kind = rng.below(4);
            let args = t.words(&[kind, holder, param]);
            let scene = t.zeros(0x10);
            t.global(mb::SCENE.sym, scene);
            let (cursor, base) = if rng.chance(4) {
                (0, 0)
            } else {
                (rng.next(), rng.next())
            };
            // mov dword [ecx], cursor; mov dword [ecx+0x10], base
            let mut seek = vec![0xc7, 0x01];
            seek.extend(cursor.to_le_bytes());
            seek.extend([0xc7, 0x41, 0x10]);
            seek.extend(base.to_le_bytes());
            let sym = if cube {
                mb::ITER_SEEK_CUBE
            } else {
                mb::ITER_SEEK_2D
            };
            t.stub_sym(mb::ASYNC_LOAD_MANAGER, 1, false, 0, 0, Ret::Val(1));
            t.stub_sym_x(sym, 2, true, 0, 0, Ret::Val(2), &seek);
            let sampler = if rng.chance(3) { 0 } else { 0x5a5a };
            t.stub_sym(mb::LINEAR_SAMPLER, 3, true, 1, 1, Ret::Val(sampler));
            t.stub_sym(mb::WRITE_VECTOR_PARAMETER, 4, true, 2, 2, Ret::Val(4));
            let check = if rng.chance(2) { 0 } else { 0x99 };
            t.stub_sym(mb::SAMPLER_CHECK, 5, true, 1, 1, Ret::Val(check));
            t.stub_sym(mb::WRITE_PARAMETER, 6, true, 2, 2, Ret::Val(6));
            t.zero_frames();
            let ebp = frame_ebp(&t.a, 2);
            let slots = t.arg_slots(&[cluster, args]);
            t.b_top = Some(ebp - 0x80);
            let (what, at) = if cube {
                ("VfxShader_setClusterCubeMap_006c1320", 0x006c_1320)
            } else {
                ("VfxShader_setClusterTexture_006c1280", 0x006c_1280)
            };
            t.check(what, at, Conv::Cdecl, &[cluster, args], |o| {
                Out::U32(b::set_cluster_texture(o, ebp, slots, cluster, args, cube))
            });
            t.same_stack(what, ebp - 0x30, 0x30);
            t.same_stack(what, slots, 8);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- batch 3 (ffx_glue_other::m5v_c) ----

#[test]
fn small_ppp_helpers_match_the_original() {
    // the destroy's 64-bit instance (signs and carries), the semaphore
    // wait, the packet wait (already done; 1..4 waits)
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0301);
        for case in 0..40_u32 {
            let id = rng.word();
            let prog = rng.word();
            t.global(mc::CURRENT_PROGRAM.sym, prog);
            t.stub_sym(mc::VFX_DESTROY, 1, false, 2, 0, Ret::Val(case));
            t.check("pppDrawFilterDes", 0x0075_7250, Conv::Cdecl, &[id], |o| {
                Out::U32(c::draw_filter_des(o, id))
            });
            t.reset();
        }
        t.stub_sym(mc::WAIT_SEMA, 1, false, 1, 0, Ret::Val(0x42));
        t.check(
            "TkWaitSemaBGThread_00888880",
            0x0088_8880,
            Conv::Cdecl,
            &[],
            |o| Out::U32(c::wait_sema_bg_thread(o)),
        );
        t.reset();
        for waits in 0..5_u32 {
            t.global(mc::WAIT_DONE.sym, u32::from(waits == 0));
            let counter = t.words(&[waits]);
            let done = t.rebase(mc::WAIT_DONE.sym);
            // dec dword [counter]; jnz +10; mov dword [done], 1
            let mut x = vec![0xff, 0x0d];
            x.extend(counter.to_le_bytes());
            x.extend([0x75, 0x0a, 0xc7, 0x05]);
            x.extend(done.to_le_bytes());
            x.extend(1_u32.to_le_bytes());
            t.stub_sym(mc::GS_SYNC_V, 1, false, 1, 0, Ret::Val(1));
            t.stub_sym_x(mc::SCE_PRINTF, 2, false, 2, 0, Ret::Val(2), &x);
            t.check("pppSendWait", 0x0076_b4a0, Conv::Cdecl, &[], |o| {
                c::send_wait(o);
                Out::None
            });
            t.reset();
        }
    });
}

#[test]
fn init_vfx_render_data_matches_the_original() {
    // every failure point (buffer, render data, setup) and the success,
    // segments 0..3, a negative count
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0302);
        for case in 0..48_u32 {
            let (lo, hi) = (rng.word(), rng.word());
            let p = t.zeros(0x10);
            let n = rng.next();
            t.w32(p + 8, n);
            let arg = rng.word();
            let rd = if rng.chance(4) {
                0
            } else {
                let rd = t.zeros(0xc0);
                let k = match rng.below(4) {
                    0 => 0,
                    1 => 0xffff_fffe,
                    _ => 1 + rng.below(3),
                };
                t.w32(rd + 0x90, k);
                let segs: Vec<u32> = (0..3 * 27).map(|_| rng.next()).collect();
                let segs = t.words(&segs);
                t.w32(rd + 0x94, segs);
                rd
            };
            let created = u32::from(case % 5 != 0);
            let setup = u32::from(case % 7 != 3);
            for (k, sym) in [mc::CREATE_RECTS, mc::CREATE_SHP_VU1]
                .into_iter()
                .enumerate()
            {
                t.stub_sym(sym, 1 + k as u32, false, 4, 0, Ret::Val(created));
            }
            t.stub_sym(mc::GET_RENDER_DATA, 3, false, 2, 0, Ret::Val(rd));
            t.stub_sym(mc::VFX_2D_SETUP, 4, false, 2, 0, Ret::Val(setup));
            t.stub_sym(mc::VFX_SETUP, 5, false, 2, 0, Ret::Val(setup));
            t.stub_sym(mc::UPDATE_RECTS, 6, false, 3, 0, Ret::Val(6));
            t.stub_sym(mc::UPDATE_SHP_VU1, 7, false, 4, 0, Ret::Val(7));
            for (k, sym) in [
                mc::VFX_2D_UPDATE,
                mc::VFX_PATH3_UPDATE,
                mc::VFX_UPDATE,
                mc::VFX_DESTROY,
            ]
            .into_iter()
            .enumerate()
            {
                t.stub_sym(sym, 8 + k as u32, false, 2, 0, Ret::Val(8));
            }
            if !t.snapshot && case % 2 == 1 {
                let d = 1 + rng.below(7);
                t.global(mc::SEGMENT_DIVISOR.sym, d);
            }
            t.check(
                "InitVFXRenderDataRects",
                0x0071_1350,
                Conv::Cdecl,
                &[lo, hi, p, arg],
                |o| {
                    c::init_rects(o, lo, hi, p, arg);
                    Out::None
                },
            );
            let (x, y, z) = (rng.word(), rng.word(), rng.word());
            t.check(
                "InitVFXRenderDataShp_OShapeVu1",
                0x0071_10d0,
                Conv::Cdecl,
                &[lo, hi, x, y, z],
                |o| {
                    c::init_shp_vu1(o, lo, hi, x, y, z, true);
                    Out::None
                },
            );
            t.check(
                "InitVFXRenderData_KaGra001",
                0x0071_1500,
                Conv::Cdecl,
                &[lo, hi, x, y, z],
                |o| {
                    c::init_shp_vu1(o, lo, hi, x, y, z, false);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

#[test]
fn combine_texture_name_matches_the_original() {
    // names with no '/', one, several, a leading and a trailing one, empty
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0303);
        let names: [&[u8]; 7] = [
            b"tex.dds",
            b"a/b.dds",
            b"x/y/z/w.dds",
            b"/lead",
            b"trail/",
            b"",
            b"some/longer/path/name_000.dds",
        ];
        for (case, name) in names.iter().enumerate() {
            let mut s = name.to_vec();
            s.push(0);
            let name = t.alloc(&s);
            let out = t.zeros(0x200);
            let block = t.zeros(0x40) + 0x10;
            t.stub_sym(mc::OPERATOR_NEW_ARRAY, 1, false, 1, 0, Ret::Val(block));
            t.stub_sym(mc::OPERATOR_DELETE_ARRAY, 2, false, 1, 0, Ret::Val(2));
            // the unified folder: mov eax, [esp+8]; mov dword [eax], "ab\0\0"
            t.stub_sym_x(
                mc::UNIFY_FILENAME,
                3,
                false,
                3,
                0,
                Ret::Val(3),
                &[0x8b, 0x44, 0x24, 0x08, 0xc7, 0x00, 0x61, 0x62, 0x00, 0x00],
            );
            let level = t.alloc(b"lvl\0");
            t.stub_sym(mc::LEVEL_FOLDER, 4, false, 0, 0, Ret::Val(level));
            t.import_sym(mc::SPRINTF_SLOT, 5, 6, 0, Ret::Val(rng.below(9)));
            t.zero_frames();
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - 0x180);
            t.check(
                "VfxShaderManager_combineTextureNameInCurrentLevel",
                0x006c_0560,
                Conv::Stdcall,
                &[name, out],
                |o| {
                    c::combine_texture_name(o, ebp, name, out);
                    Out::None
                },
            );
            t.same_stack("combineTextureNameInCurrentLevel", ebp - 0x108, 0x104);
            t.b_top = None;
            let _ = case;
            t.reset();
        }
    });
}

#[test]
fn shader_tree_insert_matches_the_original() {
    // random keys into trees built by the original itself (both oracles
    // alike), duplicates, null keys, flag bits on keys and nodes
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0304);
        for _ in 0..10 {
            let tree = t.zeros(0x20);
            t.w32(tree + 0xc, tree);
            for _ in 0..12 {
                let node = t.zeros(0x10);
                let key = if rng.chance(8) {
                    u32::from(rng.chance(2))
                } else {
                    let len = rng.below(3) as usize;
                    let mut s: Vec<u8> = (0..len)
                        .map(|_| [b'a', b'b', 0xe1][rng.below(3) as usize])
                        .collect();
                    s.push(0);
                    s.extend([0; 3]);
                    t.alloc(&s) | u32::from(rng.chance(3))
                };
                t.w32(node + 0xc, key);
                let w = rng.next();
                t.w32(node + 8, w);
                t.check(
                    "VfxShaderTree_insert_006be9e0",
                    0x006b_e9e0,
                    Conv::Thiscall(tree),
                    &[node],
                    |o| Out::U32(c::tree_insert(o, tree, node)),
                );
            }
            t.reset();
        }
    });
}

// ---- batch 4 (ffx_glue_other::m5v_d) ----

#[test]
fn opd_operand_printers_match_the_original() {
    // random opcodes: negative halfwords, bits 12-15 and 15 of the
    // opcode word, op[2] 6 or not; op_num keeps eax (a bare `ret` in the
    // game), so each printer's eax is compared
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0401);
        for _ in 0..60 {
            let mut w: Vec<u16> = (0..4).map(|_| rng.next() as u16).collect();
            if rng.chance(3) {
                w[0] &= 0x0fff;
            }
            if rng.chance(3) {
                w[1] = 6;
            }
            let bytes: Vec<u8> = w.iter().flat_map(|v| v.to_le_bytes()).collect();
            let op = t.alloc(&bytes);
            for (at, f) in [
                (0x00a6_4c00_u32, 0x00b7_bf74_u32),
                (0x00a6_4c20, 0x00b7_bf88),
                (0x00a6_4e00, 0x00b7_c02c),
                (0x00a6_4e70, 0x00b7_c058),
                (0x00a6_4e90, 0x00b7_c070),
                (0x00a6_4eb0, 0x00b7_c088),
                (0x00a6_4ef0, 0x00b7_c0ac),
            ] {
                t.stub_sym(md::OP_NUM, 1, false, 3, 0, Ret::Keep);
                t.check("opd_print (half)", at, Conv::Cdecl, &[op], |o| {
                    Out::U32(d::print_half(o, f, op))
                });
            }
            t.stub_sym(md::OP_NUM, 1, false, 4, 0, Ret::Keep);
            t.check(
                "opd_print_u_unit_00a64d60",
                0x00a6_4d60,
                Conv::Cdecl,
                &[op],
                |o| Out::U32(d::print_unit(o, 0x00b7_bfd8, op)),
            );
            t.stub_sym(md::OP_NUM, 1, false, 4, 0, Ret::Keep);
            t.check(
                "opd_print_u_task_00a64d90",
                0x00a6_4d90,
                Conv::Cdecl,
                &[op],
                |o| Out::U32(d::print_task(o, 0x00b7_bfec, 0x00b7_c004, op)),
            );
            t.stub_sym(md::OP_NUM, 1, false, 3, 0, Ret::Keep);
            t.check(
                "opd_print_u_task_off_00a64de0",
                0x00a6_4de0,
                Conv::Cdecl,
                &[op],
                |o| Out::U32(d::print_task_off(o, 0x00b7_c018, op)),
            );
            for (at, f) in [
                (0x00a6_4e20_u32, 0x00b7_c03c_u32),
                (0x00a6_4ed0, 0x00b7_c09c),
            ] {
                t.stub_sym(md::OP_NUM, 1, false, 2, 0, Ret::Keep);
                t.check("opd_print (plain)", at, Conv::Cdecl, &[op], |o| {
                    d::print_plain(o, f, op);
                    Out::None
                });
            }
            t.stub_sym(md::OP_NUM, 1, false, 3, 0, Ret::Keep);
            t.check(
                "opd_print_e_anmm_00a64e40",
                0x00a6_4e40,
                Conv::Cdecl,
                &[op],
                |o| Out::U32(d::print_anmm(o, 0x00b7_c048, op)),
            );
            t.stub_sym(md::OP_NUM, 1, false, 3, 0, Ret::Keep);
            t.check(
                "opd_print_e_gline_00a64f10",
                0x00a6_4f10,
                Conv::Cdecl,
                &[op],
                |o| match d::print_gline(o, 0x00b7_c0bc, op) {
                    Some(v) => Out::U32(v),
                    None => Out::None,
                },
            );
            t.stub_sym(md::OP_NUM, 1, false, 4, 0, Ret::Keep);
            t.check(
                "opd_print_u_part_init_00a64f40",
                0x00a6_4f40,
                Conv::Cdecl,
                &[op],
                |o| Out::U32(d::print_part_init(o, 0x00b7_c0d0, op)),
            );
            // the tail jump reads only the slot it wrote
            t.stub_sym(md::OP_NUM, 2, false, 1, 0, Ret::Keep);
            let slots = t.arg_slots(&[op]);
            t.check(
                "opd_print_e_weapon_shadow_00a64f60",
                0x00a6_4f60,
                Conv::Cdecl,
                &[op],
                |o| Out::U32(d::print_weapon_shadow(o, slots, 0x00b7_c0ec, op)),
            );
            t.same_stack("opd_print_e_weapon_shadow_00a64f60", slots, 4);
            t.b_top = None;
            // the sprintf printers
            t.stub_sym(md::OP_NUM, 3, false, 1, 0, Ret::Keep);
            let n = rng.below(12);
            t.import_sym(md::SPRINTF_SLOT, 4, 3, 0, Ret::Val(n));
            t.stub_sym(md::FLAGS_STR, 5, false, 2, 0, Ret::Val(5));
            for (at, f) in [
                (0x00a6_4c40_u32, 0x00b7_bfb4_u32),
                (0x00a6_4ca0, 0x00b7_bfc0),
                (0x00a6_4d00, 0x00b7_bfcc),
            ] {
                let ebp = frame_ebp(&t.a, 1);
                t.b_top = Some(ebp - 0x140);
                t.check("opd_print (flags)", at, Conv::Cdecl, &[op], |o| {
                    d::print_flags(o, ebp, f, op);
                    Out::None
                });
                t.b_top = None;
            }
            t.reset();
        }
    });
}

// ---- batch 5 (ffx_glue_other::m5v_e) ----

#[test]
fn bind_flags_and_thunks_match_the_original() {
    // the flag setters on random flags (and the game's own on the
    // snapshots), the debug byte, every thunk with logged words
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0501);
        for case in 0..20 {
            if !(t.snapshot && case == 0) {
                let v = rng.next() as u16;
                t.write(t.rebase(me::BIND_DEFAULT.sym), &v.to_le_bytes());
            }
            for (at, mask, set) in [
                (0x007f_b510_u32, 0x1000_u16, true),
                (0x007f_b520, 0xefff, false),
                (0x007f_b680, 0x0800, true),
                (0x007f_b720, 0x2000, true),
                (0x007f_b730, 0xdfff, false),
            ] {
                t.check("EtBindDefault", at, Conv::Cdecl, &[], |o| {
                    Out::U32(e::bind_flags(o, mask, set))
                });
            }
            let b = rng.next() as u8;
            t.patch(t.rebase(me::OP_DEBUG_BYTE.sym), &[b]);
            t.check(
                "op_debug_reset_008005f0",
                0x0080_05f0,
                Conv::Cdecl,
                &[],
                |o| Out::U32(e::op_debug_reset(o)),
            );
            for (at, target, n) in me::THUNKS {
                let args: Vec<u32> = (0..n).map(|_| rng.word()).collect();
                let r = rng.next();
                t.stub_sym(target, 1, false, n as u32, 0, Ret::Val(r));
                t.check("jmp thunk", at, Conv::Cdecl, &args, |o| {
                    Out::U32(e::jmp_thunk(o, target, &args))
                });
            }
            t.reset();
        }
    });
}

// ---- round 5 batch 1 (ffx_glue_other::m5v_f) ----

/// A `memset(p, v, n)` stub's work after the log: `rep stosb`.
fn memset_code() -> Vec<u8> {
    vec![
        0x57, // push edi
        0x8b, 0x7c, 0x24, 0x08, // mov edi, [esp+8]
        0x8b, 0x44, 0x24, 0x0c, // mov eax, [esp+0xc]
        0x8b, 0x4c, 0x24, 0x10, // mov ecx, [esp+0x10]
        0xf3, 0xaa, // rep stosb
        0x5f, // pop edi
    ]
}

/// A synthetic FP table of `n` entries with ids from a small set (0 for
/// none), in both oracles.
fn fp_table(t: &mut Twins, rng: &mut Rng, n: u32) -> u32 {
    let data = t.zeros((0x64 + 0x50 * n + 0x10) as usize);
    t.write(data + 4, &(n as u16).to_le_bytes());
    for i in 0..n {
        let id = if rng.chance(3) {
            0
        } else {
            1 + rng.below(3) as u16
        };
        t.write(data + 0x64 + 0x50 * i, &id.to_le_bytes());
    }
    data
}

#[test]
fn r5v_fp_natives_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0501);
        for case in 0..16_u32 {
            // no table, an empty one, and tables of 1..12 entries; the
            // snapshot's own table first on a snapshot
            if !(t.snapshot && case == 0) {
                let data = match case % 8 {
                    1 => 0,
                    2 => fp_table(t, &mut rng, 0),
                    _ => {
                        let n = 1 + rng.below(12);
                        fp_table(t, &mut rng, n)
                    }
                };
                t.global(mf::FP_DATA.sym, data);
            }
            let runs = t.rebase(mf::FP_RUN);
            let junk: Vec<u8> = (0..0x80 * 12).map(|_| rng.next() as u8).collect();
            t.patch(runs, &junk);
            let env = t.zeros(4);
            t.global(mf::PPP_ENV_SLOT.sym, env);
            t.stub_sym(
                mf::PPP_ALL_FREE_POBJECT,
                1,
                false,
                1,
                0,
                Ret::Val(rng.next()),
            );
            t.check(
                "pppFpFreeAll_007296a0",
                0x0072_96a0,
                Conv::Cdecl,
                &[],
                |o| {
                    f::fp_free_all(o);
                    Out::None
                },
            );
            t.patch(runs, &junk);
            t.check(
                "pppFpResetSameIds_00729740",
                0x0072_9740,
                Conv::Cdecl,
                &[],
                |o| {
                    f::fp_reset_same_ids(o);
                    Out::None
                },
            );
            t.patch(runs, &junk);
            t.check(
                "pppFpClearFrames_007297e0",
                0x0072_97e0,
                Conv::Cdecl,
                &[],
                |o| {
                    f::fp_clear_frames(o);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

#[test]
fn r5v_opus_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0502);
        for case in 0..12_u32 {
            // the effect characters' field modes
            let chr = t.rebase(mf::ET_CHR);
            let mut recs = vec![0_u8; 0x20 * 0xd0];
            Mem::read_bytes(&t.b, chr, &mut recs);
            if !(t.snapshot && case == 0) {
                for r in recs.chunks_mut(0x20) {
                    let h = if rng.chance(2) { 0 } else { rng.next() };
                    r[..4].copy_from_slice(&h.to_le_bytes());
                    r[0x16] = rng.next() as u8;
                }
            }
            t.patch(chr, &recs);
            let flags = rng.next() as u16;
            t.patch(t.rebase(mf::BIND_DEFAULT.sym), &flags.to_le_bytes());
            let modes: Vec<u32> = (0..0xd0).map(|_| rng.next()).collect();
            let seq = t.seq(&modes);
            t.stub_sym(mf::CH_GET_FIELD_MODE, 1, false, 1, 0, seq);
            t.check(
                "op_et_chr_field_mode_update_007fb690",
                0x007f_b690,
                Conv::Cdecl,
                &[],
                |o| {
                    f::et_chr_field_mode_update(o);
                    Out::None
                },
            );
            t.reset();

            // the opus objects' frees
            let slots = t.rebase(mf::OPUS_SLOTS);
            let w: Vec<u8> = (0..0x100)
                .flat_map(|_| {
                    let v = if rng.chance(3) { rng.next() | 1 } else { 0 };
                    v.to_le_bytes()
                })
                .collect();
            t.patch(slots, &w);
            let env = t.zeros(4);
            t.global(mf::PPP_ENV_SLOT.sym, env);
            t.stub_sym(
                mf::PPP_ALL_FREE_POBJECT,
                1,
                false,
                1,
                0,
                Ret::Val(rng.next()),
            );
            t.check(
                "opu_free_all_pobjects_00800970",
                0x0080_0970,
                Conv::Cdecl,
                &[],
                |o| Out::U32(f::opu_free_all_pobjects(o)),
            );
            t.reset();

            // the battle host run: +0x58 set or not, and set by the run
            let o = t.zeros(0x60);
            let before = if case % 2 == 0 { 0 } else { rng.next() | 1 };
            t.w32(o + 0x58, before);
            t.w32(o + 0x54, rng.next());
            t.global(mf::OPUS_BATTLE.sym, o);
            for g in [mf::OP_LATCH0, mf::OP_LATCH_SRC, mf::OP_EFFECT_RUN_LIMIT] {
                t.global(g.sym, rng.next());
            }
            let after = if case % 4 < 2 { 0 } else { rng.next() };
            let mut code = vec![0xc7, 0x05];
            code.extend((o + 0x58).to_le_bytes());
            code.extend(after.to_le_bytes()); // mov dword [o+0x58], after
            // the limit the run sees, into o + 0x5c
            code.push(0xa1);
            code.extend(t.rebase(mf::OP_EFFECT_RUN_LIMIT.sym).to_le_bytes()); // mov eax, [limit]
            code.push(0xa3);
            code.extend((o + 0x5c).to_le_bytes()); // mov [o+0x5c], eax
            t.stub_sym_x(
                mf::OPU_MAIN_HOST,
                1,
                false,
                2,
                0,
                Ret::Val(rng.next()),
                &code,
            );
            t.check(
                "opu_run_battle_host_00800a30",
                0x0080_0a30,
                Conv::Cdecl,
                &[],
                |g| Out::U32(f::opu_run_battle_host(g)),
            );
            t.reset();

            // the back colour: kept or not
            let kept = u8::from(case % 3 == 0);
            t.patch(t.rebase(mf::BACK_COLOR_KEPT.sym), &[kept]);
            t.stub_sym(mf::YI_SET_BACK_COLOR, 1, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mf::YI_GET_BACK_COLOR, 2, false, 1, 0, Ret::Val(rng.next()));
            t.check(
                "opus_map_back_color_restore_0080a4b0",
                0x0080_a4b0,
                Conv::Cdecl,
                &[],
                |o| {
                    f::back_color_restore(o);
                    Out::None
                },
            );
            t.patch(t.rebase(mf::BACK_COLOR_KEPT.sym), &[kept]);
            let v: Vec<u32> = (0..4)
                .map(|_| match rng.below(3) {
                    0 => (rng.below(600) as f32 - 300.0 + 0.5).to_bits(),
                    1 => float_word(&mut rng),
                    _ => (rng.next() as f32 * 7.0).to_bits(),
                })
                .collect();
            let rgba = t.words(&v);
            t.zero_frames();
            // the frame: ebx = top - 12, esp aligned down to 16 below
            // top - 20, ebp there
            let ebp = (call_top(&t.a) - 20) & !0xf;
            let buf = ebp - 0x20;
            t.b_top = Some(buf - 0x40);
            t.check(
                "opus_map_back_color_set_0080a4d0",
                0x0080_a4d0,
                Conv::Cdecl,
                &[rgba],
                |o| Out::U32(f::back_color_set(o, buf, rgba)),
            );
            t.same_stack("opus_map_back_color_set_0080a4d0", buf, 16);
            t.b_top = None;
            t.reset();

            // the pad ports: negative ports close 0 and 1 first
            let work = t.rebase(mf::CONTROLER_WORK);
            let w: Vec<u8> = (0..4).flat_map(|_| rng.next().to_le_bytes()).collect();
            t.patch(work - 8, &w);
            let port = match case % 4 {
                0 => 0xffff_ffff,
                1 => 0xffff_fffe,
                _ => rng.below(2),
            };
            let slot = if case % 5 == 0 {
                32 + rng.below(8)
            } else {
                rng.below(32)
            };
            t.stub_sym(mf::PAD_PORT_CLOSE, 1, false, 2, 0, Ret::Val(rng.next()));
            t.check(
                "Pad_PortClose_00888990",
                0x0088_8990,
                Conv::Cdecl,
                &[port, slot],
                |o| {
                    f::pad_port_close(o, port, slot);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

/// A random tree of debug-file nodes (`+0`, `+4` blocks, `+8` subtree,
/// `+0x20` next), `depth` levels deep.
fn dbg_tree(t: &mut Twins, rng: &mut Rng, depth: u32) -> u32 {
    if depth == 0 || rng.chance(3) {
        return 0;
    }
    let mut head = 0;
    for _ in 0..=rng.below(3) {
        let n = t.zeros(0x24);
        let a = if rng.chance(3) { 0 } else { rng.next() | 1 };
        let b = if rng.chance(3) { 0 } else { rng.next() | 1 };
        let sub = dbg_tree(t, rng, depth - 1);
        t.w32(n, a);
        t.w32(n + 4, b);
        t.w32(n + 8, sub);
        t.w32(n + 0x20, head);
        head = n;
    }
    head
}

#[test]
fn r5v_debug_file_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0503);
        for case in 0..16_u32 {
            // the tree free
            let tree = dbg_tree(t, &mut rng, 4);
            t.stub_sym(mf::MS_FREE, 1, false, 1, 0, Ret::Val(rng.next()));
            t.check(
                "MsDbgFile_freeTree_007e1b70",
                0x007e_1b70,
                Conv::Cdecl,
                &[tree],
                |o| {
                    f::dbg_file_free_tree(o, tree);
                    Out::None
                },
            );
            t.reset();

            // the name hashes: upper and lower case, high bytes, empty
            let n = rng.below(5);
            let table = t.zeros((0x24 * (n + 1)) as usize);
            for i in 0..=n {
                let len = if i == n {
                    0
                } else {
                    rng.below(12) as usize + 1
                };
                let mut s: Vec<u8> = (0..len)
                    .map(|_| match rng.below(4) {
                        0 => b'A' + rng.below(26) as u8,
                        1 => b'a' + rng.below(26) as u8,
                        2 => 0x80 | rng.next() as u8,
                        _ => 1 + rng.below(0x7f) as u8,
                    })
                    .collect();
                s.push(0);
                let name = t.alloc(&s);
                t.w32(table + 0x24 * i, name);
            }
            t.check(
                "MsDbgFile_hashNames_007e2d90",
                0x007e_2d90,
                Conv::Cdecl,
                &[table],
                |o| Out::U32(f::dbg_file_hash_names(o, table)),
            );
            t.reset();

            // the file load: switches off, open failing, short and full reads
            let name = t.alloc(b"name\0");
            let buf = t.zeros(0x10);
            t.patch(t.rebase(mf::OP_DEBUG_MODE.sym), &[u8::from(case % 5 != 1)]);
            t.patch(t.rebase(mf::OP_DEBUG_FILES.sym), &[u8::from(case % 5 != 2)]);
            let fd = if case % 4 == 3 { 0 } else { rng.next() | 1 };
            let size = rng.below(0x100);
            let read = if case % 2 == 0 {
                size
            } else {
                rng.below(0x100)
            };
            t.stub_sym(mf::SCE_OPEN, 1, false, 2, 0, Ret::Val(fd));
            let seq = t.seq(&[size, 0]);
            t.stub_sym(mf::SCE_LSEEK, 2, false, 3, 0, seq);
            t.stub_sym(mf::SCE_READ, 3, false, 3, 0, Ret::Val(read));
            t.stub_sym(mf::SCE_CLOSE, 4, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mf::DBG_PRINTF, 5, false, 2, 0, Ret::Val(5));
            t.stub_sym(mf::DEBUG_EXIT_TRACE, 6, false, 1, 0, Ret::Val(6));
            t.stub_sym(mf::FLUSH_CACHE, 7, false, 1, 0, Ret::Val(7));
            t.check(
                "op_debug_file_load_007e5c10",
                0x007e_5c10,
                Conv::Cdecl,
                &[name, buf],
                |o| Out::U32(f::debug_file_load(o, name, buf)),
            );
            t.reset();

            // the file read: open failing (negative), sizes around 16
            let rec = t.zeros(0x14);
            let name = t.alloc(b"file\0");
            let fd = match case % 4 {
                1 => 0x8000_0000 | rng.next(),
                3 => 0,
                _ => rng.below(64),
            };
            let size = match case % 3 {
                0 => rng.below(0x60),
                1 => [0xffff_fff0, 0xffff_ffd8, 0xffff_fff0 | rng.next()][(case % 9 / 3) as usize],
                _ => 16 * rng.below(4),
            };
            let block = t.alloc(&[0xcc; 0x80]);
            t.stub_sym(mf::SCE_OPEN, 1, false, 2, 0, Ret::Val(fd));
            let seq = t.seq(&[size, 0]);
            t.stub_sym(mf::SCE_LSEEK, 2, false, 3, 0, seq);
            t.import_sym(mf::MALLOC_SLOT, 3, 1, 0, Ret::Val(block));
            t.stub_sym(mf::SCE_READ, 4, false, 3, 0, Ret::Val(rng.next()));
            t.stub_sym(mf::SCE_CLOSE, 5, false, 1, 0, Ret::Val(rng.next()));
            t.check(
                "op_file_read",
                0x007e_2ed0,
                Conv::Cdecl,
                &[rec, name],
                |o| Out::U32(f::op_file_read(o, rec, name)),
            );
            t.reset();
        }
    });
}

#[test]
fn r5v_gdraw_callbacks_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0504);
        for case in 0..24_u32 {
            let state = t.zeros(0x800);
            for k in (0x6c0..0x6d0).step_by(4) {
                t.w32(state + k, rng.next());
            }
            t.global(mf::GDRAW.sym, state);

            // the unique id
            let handle = t.zeros(0x40);
            let id = rng.below(3);
            t.w32(handle + 0x14, id);
            let (old, new) = (rng.below(3), rng.next());
            t.check(
                "gdraw_SetTextureUniqueID_00a3abc0",
                0x00a3_abc0,
                Conv::Stdcall,
                &[handle, old, new],
                |o| Out::U32(f::set_texture_unique_id(o, handle, old, new)),
            );

            // the 3D transform: none or 12 words
            let m = if case % 3 == 0 {
                0
            } else {
                let w: Vec<u32> = (0..12).map(|_| rng.next()).collect();
                t.words(&w)
            };
            t.check(
                "gdraw_Set3DTransform_00a3a9b0",
                0x00a3_a9b0,
                Conv::Stdcall,
                &[m],
                |o| Out::U32(f::set_3d_transform(o, m)),
            );

            // the three lock checks: states 0..6, ids matching or not
            for at in [0x00a3_b010_u32, 0x00a3_b050, 0x00a3_b0c0] {
                let handle = t.zeros(0x40);
                t.w32(
                    handle + 0x30,
                    (rng.below(7) << 28) | (rng.next() & 0x0fff_ffff),
                );
                let id = rng.below(2);
                t.w32(handle + 0x14, rng.below(2));
                t.stub_sym(mf::GDRAW_RING_MOVE, 1, false, 2, 0, Ret::Val(rng.next()));
                let stats = rng.next();
                t.check(
                    "gdraw_TryLock",
                    at,
                    Conv::Stdcall,
                    &[handle, id, stats],
                    |o| Out::U32(f::try_lock(o, handle, id)),
                );
            }

            // the unlock: a handle in a ring of three, moved to its pool's
            let nodes: Vec<u32> = (0..4).map(|_| t.zeros(0x40)).collect();
            let pool = t.zeros(0x80);
            let head = pool + 0x48;
            let (h0, p, n) = (nodes[0], nodes[1], nodes[2]);
            t.w32(h0 + 0x30, (rng.below(4) << 28) | (rng.next() & 0x0fff_ffff));
            t.w32(h0 + 0x18, pool);
            t.w32(h0 + 0x1c, p);
            t.w32(h0 + 0x20, n);
            t.w32(p + 0x20, h0);
            t.w32(n + 0x1c, h0);
            t.w32(head + 0x20, nodes[3]);
            t.w32(nodes[3] + 0x1c, head);
            let (a, b) = (rng.next(), rng.next());
            t.check(
                "gdraw_Unlock_00a3b100",
                0x00a3_b100,
                Conv::Stdcall,
                &[h0, a, b],
                |o| Out::U32(f::unlock(o, h0)),
            );

            // the texture rectangle update: UpdateSubresource's 7 words
            let method = t.stub_code(2, false, 7, 7, Ret::Val(rng.next()), false);
            let mut slots = vec![0; 49];
            slots[48] = method;
            let ctx = t.object(&slots, 0x10);
            t.w32(state + 4, ctx);
            let handle = t.words(&[rng.next()]);
            let args: Vec<u32> = std::iter::once(handle)
                .chain((0..8).map(|_| rng.next()))
                .collect();
            t.zero_frames();
            let ebp = frame_ebp(&t.a, 9);
            t.b_top = Some(ebp - 0x80);
            t.check(
                "gdraw_UpdateTextureRect_00a3b160",
                0x00a3_b160,
                Conv::Stdcall,
                &args,
                |o| {
                    Out::U32(f::update_texture_rect(
                        o,
                        ebp - 0x1c,
                        args[0],
                        args[2],
                        args[3],
                        args[4],
                        args[5],
                        args[6],
                        args[7],
                    ))
                },
            );
            t.same_stack("gdraw_UpdateTextureRect_00a3b160", ebp - 0x1c, 0x18);
            t.b_top = None;

            // the D3D error report
            let hr = if case % 2 == 0 {
                mf::E_OUTOFMEMORY
            } else {
                rng.next()
            };
            let (what, where_) = (rng.next(), rng.next());
            // the out-of-memory report passes 4 words, the other 5
            let words = if hr == mf::E_OUTOFMEMORY { 4 } else { 5 };
            t.import_sym(mf::SEND_WARNING_SLOT, 3, words, 0, Ret::Val(rng.next()));
            t.check(
                "gdraw_reportD3DError_00a3c170",
                0x00a3_c170,
                Conv::Cdecl,
                &[hr, what, where_],
                |o| Out::U32(f::report_d3d_error(o, hr, what, where_)),
            );
            t.reset();

            // the frees: no state; a render target with 0..2 objects
            t.global(mf::GDRAW.sym, if case % 4 == 0 { 0 } else { state });
            let code = memset_code();
            t.stub_sym_x(mf::MEMSET, 1, false, 3, 0, Ret::Val(1), &code);
            t.stub_sym(mf::GDRAW_FREE_RESOURCES, 2, false, 1, 0, Ret::Val(2));
            t.zero_frames();
            let ebp = frame_ebp(&t.a, 0);
            t.b_top = Some(ebp - 0x80);
            t.check(
                "gdraw_FreeAll_00a380b0",
                0x00a3_80b0,
                Conv::Cdecl,
                &[],
                |o| {
                    f::free_all(o, ebp - 0x30);
                    Out::None
                },
            );
            t.same_stack("gdraw_FreeAll_00a380b0", ebp - 0x30, 0x30);
            let release = t.stub_code(3, false, 1, 1, Ret::Val(3), false);
            let rt = t.zeros(8);
            for k in 0..2 {
                if rng.chance(2) {
                    let obj = t.object(&[0, 0, release], 8);
                    t.w32(rt + 4 * k, obj);
                }
            }
            t.stub_sym(mf::GDRAW_FREE_HANDLE, 4, false, 2, 0, Ret::Val(rng.next()));
            t.zero_frames();
            let ebp = frame_ebp(&t.a, 1);
            t.b_top = Some(ebp - 0x80);
            t.check(
                "gdraw_FreeRenderTarget_00a37ac0",
                0x00a3_7ac0,
                Conv::Stdcall,
                &[rt],
                |o| Out::U32(f::free_render_target(o, ebp - 0x30, rt)),
            );
            t.same_stack("gdraw_FreeRenderTarget_00a37ac0", ebp - 0x30, 0x30);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 2 (ffx_glue_other::m5v_g) ----

/// Asserts both oracles hold the same bytes at `at..at + n`, naming the
/// first difference.
fn r5v_same(t: &Twins, what: &str, at: u32, n: usize) {
    let mut x = vec![0; n];
    let mut y = vec![0; n];
    Mem::read_bytes(&t.a, at, &mut x);
    Mem::read_bytes(&t.b, at, &mut y);
    if let Some(k) = (0..n).find(|&k| x[k] != y[k]) {
        let e = (k + 16).min(n);
        panic!(
            "{what}: the stack at {:#x} (+{k:#x}): {:x?} vs {:x?}",
            at + k as u32,
            &x[k..e],
            &y[k..e]
        );
    }
}

/// A `memcpy(dst, src, n)` stub's work after the log: `rep movsb`.
fn memcpy_code() -> Vec<u8> {
    vec![
        0x56, // push esi
        0x57, // push edi
        0x8b, 0x7c, 0x24, 0x0c, // mov edi, [esp+0xc]
        0x8b, 0x74, 0x24, 0x10, // mov esi, [esp+0x10]
        0x8b, 0x4c, 0x24, 0x14, // mov ecx, [esp+0x14]
        0xf3, 0xa4, // rep movsb
        0x5f, // pop edi
        0x5e, // pop esi
    ]
}

/// `mov dword [at], v`.
fn store_code(at: u32, v: u32) -> Vec<u8> {
    let mut c = vec![0xc7, 0x05];
    c.extend(at.to_le_bytes());
    c.extend(v.to_le_bytes());
    c
}

/// Zeroes `n` bytes of both stacks below the call top (large frames).
fn zero_deep(t: &mut Twins, n: u32) {
    let top = call_top(&t.a);
    let z = vec![0; n as usize];
    t.a.write(top - n, &z).unwrap();
    t.b.write(top - n, &z).unwrap();
}

/// A `PArray` of `items` (inline for one).
fn parray(t: &mut Twins, items: &[u32], flag: u32) -> u32 {
    let n = items.len() as u32 | flag;
    match items.len() {
        0 => t.words(&[n, 0]),
        1 => t.words(&[n, items[0]]),
        _ => {
            let data = t.words(items);
            t.words(&[n, data])
        }
    }
}

#[test]
fn r5v_create_mesh_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0601);
        for case in 0..24_u32 {
            let scene = t.zeros(0x1_0000);
            let cluster = if case % 6 == 0 { 0 } else { rng.next() | 1 };
            t.w32(scene + 0xfa7c, cluster);
            t.global(mg::SCENE.sym, scene);
            let m = t.words(&[0, rng.next(), rng.next(), 0]);
            let mesh0 = t.zeros(0x98);
            t.w32(mesh0 + 0x94, m);
            let first = t.words(&[rng.next(), rng.next()]);
            let others: Vec<u32> = (0..rng.below(2)).map(|_| rng.next()).collect();
            let items: Vec<u32> = std::iter::once(first).chain(others).collect();
            let materials = parray(t, &items, 0);
            let gmesh = t.zeros(0x20);
            t.w32(gmesh + 0x10, materials);
            let slot1 = t.method_stub(1, 2, Ret::Val(rng.next()));
            let slot2 = t.method_stub(2, 1, Ret::Val(rng.next()));
            let geo = t.object(&[0, slot1, slot2], 0x30);
            t.w32(geo + 0xc, gmesh);
            let r = t.zeros(0x14);
            t.w32(r, mesh0);
            t.w32(r + 8, geo);
            t.w32(r + 0x10, rng.next());
            let finds: Vec<u32> = (0..2)
                .map(|_| if rng.chance(3) { 0 } else { rng.next() | 1 })
                .collect();
            let seq = t.seq(&finds);
            t.stub_sym(mg::FIND_PARAM, 3, true, 1, 1, seq);
            t.stub_sym(mg::SHADOW_MANAGER, 4, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(mg::SHADOW_SAMPLER, 5, true, 0, 0, Ret::Val(rng.next()));
            let map = t.words(&[rng.next(), u32::from(rng.chance(2))]);
            t.stub_sym(mg::SHADOW_MAP, 6, true, 0, 0, Ret::Val(map));
            t.stub_sym(mg::SET_VALUE_DESC, 7, true, 2, 2, Ret::Val(7));
            // the mesh gone by the end (the last check)
            let code = if case % 4 == 1 {
                store_code(geo + 0xc, 0)
            } else {
                vec![]
            };
            t.stub_sym_x(
                mg::WRITE_VECTOR_PARAMETER,
                8,
                true,
                2,
                2,
                Ret::Val(8),
                &code,
            );
            t.stub_sym(mg::SET_MESH_VISIBLE, 9, true, 2, 2, Ret::Val(9));
            t.zero_frames();
            let ebp = frame_ebp(&t.a, 0);
            t.b_top = Some(ebp - 0x80);
            t.check(
                "ChrY0Shadow_createMesh",
                0x0069_f9d0,
                Conv::Thiscall(r),
                &[],
                |o| Out::U32(vg::create_mesh(o, ebp - 0x1c, r)),
            );
            r5v_same(t, "ChrY0Shadow_createMesh", ebp - 0x1c, 0x10);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn r5v_set_tk_runtime_clut_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0602);
        for case in 0..32_u32 {
            let g = t.zeros(0x60);
            let n = if case % 5 == 0 {
                None
            } else {
                Some(rng.below(4))
            };
            let mut finds = vec![];
            if let Some(n) = n {
                let elems: Vec<u32> = (0..n)
                    .map(|_| {
                        let buf = t.zeros(0x50);
                        let size = match rng.below(3) {
                            0 => 0x14,
                            1 => 0x24,
                            _ => rng.below(0x50),
                        };
                        t.w32(buf, size);
                        t.words(&[rng.next(), buf])
                    })
                    .collect();
                let list = parray(t, &elems, rng.next() & 0x8000_0000);
                t.w32(g + 0x14, list);
                for _ in 0..3 * n {
                    finds.push(if rng.chance(4) {
                        0
                    } else {
                        let p = t.zeros(0x10);
                        let off = match rng.below(4) {
                            0 => 4,
                            1 => 3,
                            2 => 0x14,
                            _ => rng.below(0x40) as u16,
                        };
                        // the float4s copy at most 16 bytes, the float 4: more
                        // copies the original's saved registers
                        let most = if finds.len() % 3 == 2 { 5 } else { 0x11 };
                        let size = if rng.chance(3) {
                            most - 1
                        } else {
                            rng.below(most)
                        };
                        let size = size as u16 | (rng.next() as u16 & 0xe000);
                        t.write(p + 8, &off.to_le_bytes());
                        t.write(p + 0xa, &size.to_le_bytes());
                        p
                    });
                }
            }
            t.w32(g + 0x28, rng.next());
            finds.push(0);
            let seq = t.seq(&finds);
            t.stub_sym(mg::FIND_PARAM, 1, true, 1, 1, seq);
            let code = memcpy_code();
            t.stub_sym_x(mg::MEMCPY, 2, false, 3, 0, Ret::Val(2), &code);
            let f = match case % 6 {
                0 => (300.0 + rng.below(100) as f32).to_bits(),
                1 => (-300.0 - rng.below(100) as f32).to_bits(),
                2 => 255.0_f32.to_bits(),
                3 => (-256.0_f32).to_bits(),
                _ => float_word(&mut rng),
            };
            let args = [rng.next(), rng.next(), f];
            t.zero_frames();
            let ebp = frame_ebp(&t.a, 3);
            let slots = t.arg_slots(&args);
            t.check(
                "VFXDynamicGeometry_setTkRuntimeClutParameters",
                0x006a_37f0,
                Conv::Thiscall(g),
                &args,
                |o| {
                    vg::set_tk_runtime_clut(o, slots, ebp - 0x24, g, args[0], args[1], args[2]);
                    Out::None
                },
            );
            r5v_same(t, "setTkRuntimeClutParameters slots", slots, 12);
            r5v_same(t, "setTkRuntimeClutParameters frame", ebp - 0x24, 0x24);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn r5v_resize_segments_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0603);
        for case in 0..32_u32 {
            let geo = t.zeros(0xc0);
            t.w32(geo + 0xb8, rng.next());
            let old_n = rng.below(4);
            let old = t.zeros(0x6c * 4);
            for i in 0..4 {
                let e = old + 0x6c * i;
                let count = if rng.chance(2) {
                    4
                } else {
                    rng.below(3) | (rng.next() & 0x8000_0000)
                };
                t.w32(e + 0x28, count);
                let bufs = if rng.chance(4) { 0 } else { t.zeros(0x100) };
                t.w32(e + 0x2c, bufs);
                t.w32(e + 0x64, if rng.chance(2) { 0 } else { rng.next() });
            }
            let n = if case % 3 == 0 { old_n } else { rng.below(4) };
            let flag = if rng.chance(4) { 0x8000_0000 } else { 0 };
            let arr = t.words(&[old_n | flag, old]);
            let g = t.zeros(0x30);
            t.w32(g + 4, arr);
            t.w32(g + 0x24, geo);
            let mut blocks = vec![];
            for k in 0..8 {
                blocks.push(if rng.chance(5) {
                    0
                } else if k == 0 {
                    // fresh entries with stale words (`+0x28`, `+0x64`)
                    let w: Vec<u32> = (0..0x6c).map(|_| rng.below(6)).collect();
                    t.words(&w)
                } else {
                    t.zeros(0x100)
                });
            }
            let seq = t.seq(&blocks);
            t.stub_sym(mg::PHYRE_MALLOC, 1, false, 2, 0, seq);
            t.stub_sym(mg::SEGMENTS_CONSTRUCT, 2, false, 2, 0, Ret::Val(2));
            t.stub_sym(mg::SEGMENTS_DESTROY, 3, false, 2, 0, Ret::Val(3));
            t.stub_sym(mg::BUFFER_CTOR, 4, true, 0, 0, Ret::Val(4));
            t.stub_sym(mg::BUFFER_WAIT_AND_CLEAR, 5, true, 0, 0, Ret::Val(5));
            t.stub_sym(mg::PHYRE_FREE, 6, false, 1, 0, Ret::Val(6));
            t.stub_sym(mg::FORMAT_SIZE, 7, false, 1, 0, Ret::Val(7));
            let bound: Vec<u32> = (0..8)
                .map(|_| [0, 1, 0x100, 0x201][rng.below(4) as usize])
                .collect();
            let seq = t.seq(&bound);
            t.stub_sym(mg::INDEX_IS_BOUND, 8, true, 0, 0, seq);
            t.stub_sym(mg::INDEX_RELEASE, 9, true, 0, 0, Ret::Val(9));
            let args = [n, rng.next(), rng.next()];
            t.check(
                "ClassDynamicGeometry_resizeSegments_006a44d0",
                0x006a_44d0,
                Conv::Thiscall(g),
                &args,
                |o| Out::U32(vg::resize_segments(o, g, n)),
            );
            t.reset();
        }
    });
}

#[test]
fn r5v_copy_shader_map_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0604);
        for case in 0..32_u32 {
            let src = t.zeros(0x30);
            // the leftmost walk: a chain down `+0` to the head, or none
            let k = rng.below(4);
            let mut nodes = vec![];
            let mut prev = src;
            for _ in 0..k {
                let nd = t.zeros(0x14);
                t.w32(nd, prev);
                t.w32(nd + 0x10, rng.next());
                nodes.push(nd);
                prev = nd;
            }
            // (an empty `+0xc` faults in the original: not a tree)
            let start = if case % 7 == 1 { src } else { prev };
            t.w32(src + 0xc, start);
            // the in-order nodes the successor stub hands out, then the head
            let order: Vec<u32> = (0..rng.below(4))
                .map(|_| {
                    let nd = t.zeros(0x14);
                    t.w32(nd + 0x10, rng.next());
                    nd
                })
                .chain(std::iter::once(src))
                .collect();
            let seq = t.seq(&order);
            t.stub_sym(mg::TREE_NEXT, 1, true, 1, 1, seq);
            let count = if case % 5 == 0 { 0 } else { 1 + rng.below(5) };
            t.stub_sym(mg::TREE_COUNT_NODES, 2, true, 0, 0, Ret::Val(count));
            let map = t.zeros(0x30);
            // the free list's items, when the list has any
            let mut head = 0;
            for _ in 0..rng.below(5) {
                let it = t.zeros(0x14);
                t.w32(it, head);
                t.w32(it + 0xc, rng.next());
                head = it;
            }
            t.w32(map + 0x18, head);
            t.w32(map + 0x20, rng.next());
            t.stub_sym(mg::FREE_LIST_CTOR, 3, true, 4, 4, Ret::Val(3));
            t.stub_sym(mg::FREE_LIST_ALLOCATE_BLOCK, 4, true, 1, 1, Ret::Val(4));
            t.stub_sym(mg::SHADER_NODE_COPY, 5, true, 1, 1, Ret::Val(5));
            let ins: Vec<u32> = (0..8).map(|_| u32::from(rng.chance(3))).collect();
            let seq = t.seq(&ins);
            t.stub_sym(mg::SHADER_TREE_INSERT, 6, true, 1, 1, seq);
            t.stub_sym(mg::TREE_POST_INSERT, 7, true, 1, 1, Ret::Val(7));
            t.stub_sym(mg::PHYRE_FREE, 8, false, 1, 0, Ret::Val(8));
            t.stub_sym(mg::FREE_LIST_DEALLOCATE, 9, true, 1, 1, Ret::Val(9));
            t.check(
                "VfxShaderManager_copyShaderMap_006bee80",
                0x006b_ee80,
                Conv::Thiscall(map),
                &[src],
                |o| Out::U32(vg::copy_shader_map(o, map, src)),
            );
            t.reset();
        }
    });
}

/// The render-data creates' common stubs.
fn create_stubs(t: &mut Twins, rng: &mut Rng, sprintf_words: u32) {
    let code = memset_code();
    t.stub_sym_x(mg::MEMSET, 1, false, 3, 0, Ret::Val(1), &code);
    t.import_sym(mg::SPRINTF_SLOT, 2, sprintf_words, 0, Ret::Val(2));
    t.stub_sym(mg::VFX_CREATE, 3, false, 3, 0, Ret::Val(rng.next()));
    t.stub_sym(
        mg::VFX_CREATE_DATA_BUFFER,
        4,
        false,
        7,
        0,
        Ret::Val(rng.next()),
    );
    t.stub_sym(mg::DBG_PRINTF, 5, false, 2, 0, Ret::Val(5));
}

#[test]
fn r5v_render_data_creates_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0605);
        for case in 0..24_u32 {
            // the rectangles
            create_stubs(t, &mut rng, 3);
            let args = [rng.next(), rng.next(), rng.below(0x100), rng.next()];
            // (the frame's top 16 bytes: the cookie and `_alloca_probe`'s
            // pushes, not compared)
            zero_deep(t, 0x7000);
            let ebp = frame_ebp(&t.a, 4);
            t.b_top = Some(ebp - mg::RECTS_FRAME - 0x40);
            t.check(
                "createVFXRenderDataBufferRects",
                0x0071_4c50,
                Conv::Cdecl,
                &args,
                |o| Out::U32(vg::create_rects(o, ebp, args[0], args[1], args[2], args[3])),
            );
            r5v_same(
                t,
                "rects",
                ebp - mg::RECTS_FRAME,
                (mg::RECTS_FRAME - 0x10) as usize,
            );
            t.reset();

            // the guide map: groups of 0..n, one flagged, too many
            create_stubs(t, &mut rng, 2);
            let groups = match case % 6 {
                0 => 0,
                1 => 31,
                _ => 1 + rng.below(6),
            };
            let mut body = vec![];
            let mut total = 0_i32;
            for i in 0..groups {
                let k = if rng.chance(5) {
                    0
                } else {
                    1 + rng.below(3) as i16
                };
                let mut gp = vec![0_u8; 0x10 + 20 * usize::try_from(k.max(0)).unwrap()];
                gp[1] = u8::from(case % 6 == 2 && i == groups - 1);
                gp[2..4].copy_from_slice(&k.to_le_bytes());
                total += i32::from(k);
                body.extend(gp);
            }
            let total = if case % 6 == 3 { total + 2 } else { total };
            let mut model = vec![0_u8; 0x20];
            model[4..8].copy_from_slice(&0x20_u32.to_le_bytes());
            model[0x10..0x12]
                .copy_from_slice(&(total.max(i32::from(groups > 0)) as i16).to_le_bytes());
            model.extend(body);
            model.extend([0_u8; 0x40]);
            let model = t.alloc(&model);
            let info = t.zeros(0x48);
            t.w8(info, rng.next() as u8);
            t.w8(info + 8, rng.next() as u8);
            t.w32(info + 0x44, if rng.chance(2) { 0 } else { rng.next() });
            t.stub_sym(mg::ALPHA_FUNCTION, 6, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mg::GUILDMAP_CREATE, 7, false, 3, 0, Ret::Val(7));
            let args = [rng.next(), rng.next(), model, info];
            zero_deep(t, 0x2400);
            let ebp = frame_ebp(&t.a, 4);
            t.b_top = Some(ebp - mg::GUILD_MAP_FRAME - 0x40);
            t.check(
                "createGuildMapRenderDataBuffer",
                0x0071_33e0,
                Conv::Cdecl,
                &args,
                |o| Out::U32(vg::create_guild_map(o, ebp, args[0], args[1], model, info)),
            );
            r5v_same(
                t,
                "guild map",
                ebp - mg::GUILD_MAP_FRAME,
                (mg::GUILD_MAP_FRAME - 0x10) as usize,
            );
            t.reset();

            // Shp_OShapeVu1: texture missing, empty counts, both flags
            create_stubs(t, &mut rng, 3);
            let shape = t.zeros(0x20);
            let k = if case % 5 == 0 {
                0
            } else {
                rng.below(40) as i16 - 4
            };
            t.write(shape + 2, &k.to_le_bytes());
            t.w32(shape + 0x10, rng.next());
            t.w32(shape + 0x14, rng.next());
            t.global(mg::SHAPE_ENLARGE.sym, 1 + rng.below(3));
            t.global(mg::SEGMENT_DIVISOR.sym, 1 + rng.below(3));
            let loaded = u32::from(case % 4 != 1);
            t.stub_sym(
                mg::IS_TEXTURE_WITH_BP_LOADED,
                6,
                false,
                7,
                0,
                Ret::Val(loaded),
            );
            t.import_sym(mg::STRSTR_SLOT, 7, 2, 0, Ret::Val(rng.below(2)));
            // the empty-count report passes one word, the bp one two: log the
            // format and, for the bp report, its word
            let words = if loaded == 0 { 2 } else { 1 };
            t.stub_sym(mg::DBG_PRINTF, 8, false, words, 0, Ret::Val(8));
            let args = [rng.next(), rng.next(), shape, rng.next()];
            zero_deep(t, 0x7000);
            let ebp = frame_ebp(&t.a, 4);
            t.b_top = Some(ebp - mg::SHP_VU1_FRAME - 0x40);
            t.check(
                "createVFXRenderDataBufferShp_OShapeVu1",
                0x0071_5810,
                Conv::Cdecl,
                &args,
                |o| Out::U32(vg::create_shp_vu1(o, ebp, args[0], args[1], shape, args[3])),
            );
            r5v_same(
                t,
                "shp vu1",
                ebp - mg::SHP_VU1_FRAME,
                (mg::SHP_VU1_FRAME - 0x10) as usize,
            );
            t.reset();

            // the U16 mesh break: each first letter and another
            create_stubs(t, &mut rng, 3);
            let letter = b"mnfcswx"[(case % 7) as usize];
            let name = t.alloc(&[letter, b'0', b'1', 0]);
            t.stub_sym(mg::CH_GET_NAME, 6, false, 1, 0, Ret::Val(name));
            t.stub_sym(mg::UNIFY_FILENAME, 7, false, 3, 0, Ret::Val(7));
            let model = t.words(&[0, rng.below(0x1000)]);
            let args = [rng.next(), rng.next(), model, rng.next(), rng.next()];
            zero_deep(t, 0x800);
            let ebp = frame_ebp(&t.a, 5);
            t.b_top = Some(ebp - mg::U16_BREAK_FRAME - 0x40);
            t.check(
                "createU16MeshBreakRenderDataBuffer",
                0x0080_2fb0,
                Conv::Cdecl,
                &args,
                |o| {
                    Out::U32(vg::create_u16_mesh_break(
                        o, ebp, args[0], args[1], model, args[3], args[4],
                    ))
                },
            );
            r5v_same(
                t,
                "u16 break",
                ebp - mg::U16_BREAK_FRAME,
                (mg::U16_BREAK_FRAME - 4) as usize,
            );
            t.reset();

            // the GT3 mesh break: texture loaded or not
            create_stubs(t, &mut rng, 3);
            t.stub_sym(
                mg::IS_TEXTURE_LOADED,
                6,
                false,
                6,
                0,
                Ret::Val(u32::from(case % 3 != 0)),
            );
            t.import_sym(mg::PRINTF_SLOT, 7, 2, 0, Ret::Val(7));
            let model = t.words(&[0, rng.below(0x1000), rng.next(), rng.next()]);
            let args = [rng.next(), rng.next(), model, rng.next()];
            zero_deep(t, 0x800);
            let ebp = frame_ebp(&t.a, 4);
            t.b_top = Some(ebp - mg::GT3_BREAK_FRAME - 0x40);
            t.check(
                "createGT3MeshBreakRenderDataBuffer",
                0x00a7_3c20,
                Conv::Cdecl,
                &args,
                |o| {
                    Out::U32(vg::create_gt3_mesh_break(
                        o, ebp, args[0], args[1], model, args[3],
                    ))
                },
            );
            r5v_same(
                t,
                "gt3 break",
                ebp - mg::GT3_BREAK_FRAME,
                (mg::GT3_BREAK_FRAME - 4) as usize,
            );
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 3 (ffx_glue_other::m5v_h) ----

/// A fake immediate context: recording stdcall stubs (`this` and the
/// arguments logged and popped) at the methods `m5v_h` calls.
fn gdraw_context(t: &mut Twins, rng: &mut Rng) -> u32 {
    let mut slots = vec![0; 0x1c0 / 4];
    for (k, (off, words)) in [
        (mh::vt::PS_SET_SHADER_RESOURCES, 4),
        (mh::vt::PS_SET_SAMPLERS, 4),
        (mh::vt::IA_SET_VERTEX_BUFFERS, 6),
        (mh::vt::IA_SET_INDEX_BUFFER, 4),
        (mh::vt::IA_SET_PRIMITIVE_TOPOLOGY, 2),
        (mh::vt::OM_SET_RENDER_TARGETS, 4),
        (mh::vt::RS_SET_STATE, 2),
        (mh::vt::CLEAR_STATE, 1),
    ]
    .into_iter()
    .enumerate()
    {
        slots[(off / 4) as usize] = t.stub_code(
            0x20 + k as u32,
            false,
            words,
            words,
            Ret::Val(rng.next()),
            false,
        );
    }
    t.object(&slots, 0x10)
}

/// A `GDraw` state with a context and random words where these read.
fn gdraw_state(t: &mut Twins, rng: &mut Rng) -> u32 {
    let s = t.zeros(0xb00);
    let ctx = gdraw_context(t, rng);
    t.w32(s + 4, ctx);
    for off in (0x6c8..0x6fc).step_by(4).chain((0x780..0x7c0).step_by(4)) {
        t.w32(s + off, rng.next());
    }
    t.w32(s + 0x790, rng.below(4));
    t.w32(s + 0x6f8, u32::from(rng.chance(3)));
    t.global(mh::GDRAW.sym, s);
    s
}

#[test]
fn r5v_gdraw_targets_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0701);
        for case in 0..24_u32 {
            gdraw_state(t, &mut rng);

            // the handle free: a handle in a ring, its pool's free ring
            let pool = t.zeros(0x80);
            t.w32(pool, rng.next());
            let nodes: Vec<u32> = (0..3).map(|_| t.zeros(0x40)).collect();
            let hd = nodes[0];
            t.w32(hd + 0x30, rng.next());
            t.w32(hd + 0x14, rng.next());
            t.w32(hd + 0x18, pool);
            t.w32(hd + 0x1c, nodes[1]);
            t.w32(hd + 0x20, nodes[2]);
            let first = t.zeros(0x40);
            t.w32(pool + 0x10 + 0x20, first);
            let stats = t.words(&(0..12).map(|_| rng.next()).collect::<Vec<_>>());
            t.stub_sym(mh::RELEASE_TEXTURE_VIEWS, 1, false, 1, 0, Ret::Val(1));
            t.check(
                "gdraw_freeHandle_00a3b480",
                0x00a3_b480,
                Conv::Cdecl,
                &[hd, stats],
                |o| Out::U32(gh::free_handle(o, hd, stats)),
            );
            t.reset();

            // the reclaim: empty, a vertex or texture pool, frames on both
            // sides, the warning given already
            let pool = t.zeros(0x1b0);
            let r = t.zeros(0x40);
            let used = match case % 4 {
                0 => pool + 0x48,
                1 => 0,
                _ => r,
            };
            t.w32(pool + 0x64, used);
            t.w32(pool + 0xc, rng.below(4));
            t.w32(r + 0x14, if rng.chance(3) { 0 } else { rng.next() });
            let (lo, hi) = (rng.below(3), rng.below(2));
            t.w32(pool + 0x1a0, lo);
            t.w32(pool + 0x1a4, hi);
            t.w32(r + 0x28, rng.below(3));
            t.w32(r + 0x2c, rng.below(2));
            t.import_sym(mh::DISCARD_SLOT, 2, 2, 2, Ret::Val(2));
            t.import_sym(mh::SEND_WARNING_SLOT, 3, 2, 0, Ret::Val(3));
            t.stub_sym(mh::FREE_HANDLE, 4, false, 2, 0, Ret::Val(rng.next()));
            let stats = rng.next();
            t.check(
                "GDraw_pool_reclaim_00a3b500",
                0x00a3_b500,
                Conv::Cdecl,
                &[pool, stats],
                |o| Out::U32(gh::pool_reclaim(o, pool, stats)),
            );
            t.reset();

            // the unbind: a stack entry or none
            let s = gdraw_state(t, &mut rng);
            let entry = t.words(&[0, 0, rng.next()]);
            let top = t.words(&[if case % 3 == 0 { 0 } else { entry }]);
            t.w32(s + 0xaac, top);
            t.zero_frames();
            let ebp = frame_ebp(&t.a, 0);
            t.b_top = Some(ebp - 0x40);
            t.check(
                "gdraw_unbindTarget_00a3b590",
                0x00a3_b590,
                Conv::Cdecl,
                &[],
                |o| Out::U32(gh::unbind_target(o, ebp - 8)),
            );
            t.same_stack("gdraw_unbindTarget_00a3b590", ebp - 8, 8);
            t.b_top = None;

            // the viewport: the main target, an entry with or without a size
            let entry = t.words(&[0, 0, 0, rng.next(), rng.next(), 0, u32::from(rng.chance(2))]);
            t.w32(s + 0xaac, if case % 2 == 0 { s + 0x9cc } else { entry });
            t.stub_sym(mh::SET_VIEWPORT, 5, false, 4, 0, Ret::Val(rng.next()));
            t.check(
                "gdraw_setViewportForTarget_00a3cd30",
                0x00a3_cd30,
                Conv::Cdecl,
                &[],
                |o| Out::U32(gh::set_viewport_for_target(o)),
            );

            // the texture unit: none or a texture
            let texture = if case % 3 == 0 { 0 } else { t.zeros(8) };
            let args = [rng.below(8), texture, rng.below(4), rng.below(4)];
            t.zero_frames();
            let slots = t.arg_slots(&args);
            t.check(
                "gdraw_setTextureUnit_00a3ccb0",
                0x00a3_ccb0,
                Conv::Cdecl,
                &args,
                |o| {
                    Out::U32(gh::set_texture_unit(
                        o,
                        slots + 4,
                        args[0],
                        args[1],
                        args[2],
                        args[3],
                    ))
                },
            );
            t.same_stack("gdraw_setTextureUnit_00a3ccb0", slots, 16);
            t.b_top = None;

            // the state reset
            t.check(
                "gdraw_resetState_00a3c280",
                0x00a3_c280,
                Conv::Cdecl,
                &[],
                |o| Out::U32(gh::reset_state(o)),
            );

            // the projection: sizes of 0 (infinities, 0/0) and of all signs
            let pick = |rng: &mut Rng| match rng.below(4) {
                0 => 0,
                1 => rng.below(2000),
                2 => rng.below(2000).wrapping_neg(),
                _ => rng.next(),
            };
            let x0 = pick(&mut rng);
            let y0 = pick(&mut rng);
            let x1 = if case % 5 == 0 { x0 } else { pick(&mut rng) };
            let y1 = if case % 7 == 0 { y0 } else { pick(&mut rng) };
            let args = [x0, x1, y0, y1];
            let code = memset_code();
            t.stub_sym_x(mh::MEMSET, 6, false, 3, 0, Ret::Val(6), &code);
            t.zero_frames();
            let slots = t.arg_slots(&args);
            t.check(
                "gdraw_setProjection_00a3c450",
                0x00a3_c450,
                Conv::Cdecl,
                &args,
                |o| Out::U32(gh::set_projection(o, slots, x0, x1, y0, y1)),
            );
            t.same_stack("gdraw_setProjection_00a3c450", slots, 16);
            t.b_top = None;

            // the output binding: the main target or a stack entry with or
            // without a depth buffer
            let main: u32 = t.r32(s + 0x780);
            let target = if case % 2 == 0 { main } else { rng.next() };
            t.stub_sym(mh::UNBIND, 7, false, 0, 0, Ret::Val(target));
            let entry = t.zeros(0x20);
            t.w8(entry + 0x14, rng.next() as u8);
            t.w32(s + 0xaac, entry);
            t.stub_sym(
                mh::CREATE_DEPTH_BUFFER,
                8,
                false,
                1,
                0,
                Ret::Val(rng.next()),
            );
            let stats = t.zeros(0x30);
            t.write(stats, &(rng.next() as u16).to_le_bytes());
            t.zero_frames();
            let ebp = frame_ebp(&t.a, 1);
            t.b_top = Some(ebp - 0x40);
            t.check(
                "gdraw_bindOutput_00a3c570",
                0x00a3_c570,
                Conv::Cdecl,
                &[stats],
                |o| Out::U32(gh::bind_output(o, ebp - 4, stats)),
            );
            t.same_stack("gdraw_bindOutput_00a3c570", ebp - 4, 4);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 4 (ffx_glue_other::m5v_i) ----

#[test]
fn r5v_ppp_packets_and_rpc_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0801);
        for case in 0..24_u32 {
            // the crosshair: either packet, path 1 busy a few times
            t.patch(
                t.rebase(mi::CROSSHAIR_FLIP.sym),
                &(rng.next() as u16).to_le_bytes(),
            );
            let words: Vec<u8> = (0..4).flat_map(|_| rng.next().to_le_bytes()).collect();
            t.patch(t.rebase(mi::CROSSHAIR_WORDS), &words);
            let mut busy: Vec<u32> = (0..rng.below(3)).map(|_| 0x10 | rng.next()).collect();
            busy.push(rng.next() & !0x10);
            let seq = t.seq(&busy);
            t.stub_sym(mi::GS_SYNC_PATH, 1, false, 2, 0, seq);
            t.stub_sym(mi::DMA_SYNC, 2, false, 3, 0, Ret::Val(2));
            t.stub_sym(mi::DMA_SEND, 3, false, 2, 0, Ret::Val(rng.next()));
            let args: Vec<u32> = (0..6).map(|_| rng.next()).collect();
            t.check(
                "pppCrosshairPacket_00760ce0",
                0x0076_0ce0,
                Conv::Cdecl,
                &args,
                |o| {
                    Out::U32(gi::crosshair_packet(
                        o, args[0], args[1], args[2], args[3], args[4], args[5],
                    ))
                },
            );
            t.reset();

            // the CLUT blocks (16-aligned frame: ebx = top - 16, ebp the
            // aligned esp)
            t.stub_sym(mi::GS_SYNC_PATH, 1, false, 2, 0, Ret::Val(rng.next()));
            t.stub_sym(mi::SET_DEF_LOAD_IMAGE, 2, false, 8, 0, Ret::Val(2));
            t.stub_sym(mi::FLUSH_CACHE, 3, false, 1, 0, Ret::Val(3));
            t.stub_sym(mi::EXEC_LOAD_IMAGE, 4, false, 2, 0, Ret::Val(4));
            let (src, dbp) = (rng.next(), rng.next());
            t.zero_frames();
            let ebp = (call_top(&t.a) - 24) & !0xf;
            t.b_top = Some(ebp - 0x100);
            t.check(
                "pppLoadClutBlocks_00762810",
                0x0076_2810,
                Conv::Cdecl,
                &[src, dbp],
                |o| Out::U32(gi::load_clut_blocks(o, ebp - 0x60, src, dbp)),
            );
            t.b_top = None;
            t.reset();

            // the RPC command
            let rec = t.zeros(0x48);
            t.w32(rec + 4, rng.next());
            let len = rng.below(20) as usize;
            let mut text: Vec<u8> = (0..len).map(|_| 1 + rng.below(0xff) as u8).collect();
            text.push(0);
            let text = t.alloc(&text);
            t.stub_sym(mi::SIF_CALL_RPC, 1, false, 9, 0, Ret::Val(1));
            t.check(
                "rpc_send_command_0076b030",
                0x0076_b030,
                Conv::Cdecl,
                &[rec, text],
                |o| Out::U32(gi::rpc_send_command(o, rec, text)),
            );
            t.reset();

            // the RPC read: empty, error, end, direct, chunked (to a buffer
            // or none), checksum right or wrong
            let rec = t.zeros(0x48);
            let e = t.zeros(0x1000);
            let word = match case % 6 {
                0 => 0,
                _ => 0x108 + 8 * rng.below(0x40),
            };
            // a size the first chunk uses up exactly at times
            let size = if case % 4 == 3 {
                word - 8
            } else {
                4 * rng.below(0x180) + rng.below(4)
            };
            t.write(e, &(word as u16).to_le_bytes());
            let kind = match case % 6 {
                1 => 0xffff_ffff,
                2 => 0x80,
                _ => rng.next(),
            };
            t.w32(e + 8, kind);
            t.w32(e + 0xc, rng.next());
            t.w32(e + 0x10, size);
            for k in (0x18..0x1000).step_by(4) {
                t.w32(e + k, rng.below(16));
            }
            let sum = if rng.chance(2) {
                0
            } else {
                (0..size / 4).fold(0_u32, |s, k| s.wrapping_add(t.r32(e + 0x18 + 4 * k)))
            };
            t.w32(e + 0x14, sum);
            t.w32(rec + 0x10, rng.next());
            t.w32(rec + 0x14, e);
            let dst = if case % 4 == 0 { 0 } else { t.zeros(0x1000) };
            t.w32(rec + 0x18, dst);
            let old = t.zeros(0x1000);
            t.w32(rec + 0x1c, old);
            let start = t.stub_code(2, false, 1, 0, Ret::Val(u32::from(case % 3 != 0)), false);
            let done = t.stub_code(3, false, 1, 0, Ret::Val(3), false);
            let failed = t.stub_code(4, false, 1, 0, Ret::Val(4), false);
            t.w32(rec + 0x3c, start);
            t.w32(rec + 0x40, done);
            t.w32(rec + 0x44, failed);
            t.stub_sym(mi::SIF_CALL_RPC, 1, false, 9, 0, Ret::Val(1));
            t.stub_sym(mi::SIF_INIT_RPC, 5, false, 1, 0, Ret::Val(5));
            t.stub_sym(mi::DEBUG_EXIT_TRACE, 6, false, 1, 0, Ret::Val(6));
            t.stub_sym(mi::SCE_PRINTF, 7, false, 1, 0, Ret::Val(7));
            t.stub_sym(mi::RPC_SEND_COMMAND, 8, false, 2, 0, Ret::Val(8));
            t.check("rpc_read", 0x0076_b4f0, Conv::Cdecl, &[rec], |o| {
                Out::U32(gi::rpc_read(o, rec))
            });
            t.reset();
        }
    });
}

// ---- round 5 batch 5 (ffx_glue_other::m5v_j) ----

#[test]
fn r5v_effect_debug_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0901);
        for case in 0..32_u32 {
            // the opus debug command: kinds 0..5 and others
            let unit = t.zeros(0xa0);
            for k in 0..3 {
                t.w32(unit + 0x90 + 4 * k, float_word(&mut rng));
            }
            let host = t.zeros(0x3c0);
            t.w32(host + 0x220, unit);
            t.write(host + 0x212, &(rng.below(2) as u16).to_le_bytes());
            let cmd = t.zeros(8);
            let kind = if case % 8 == 7 {
                rng.next() as i16
            } else {
                (case % 8) as i16 - 1
            };
            t.write(cmd + 2, &kind.to_le_bytes());
            t.write(cmd + 4, &(rng.next() as u16).to_le_bytes());
            t.check("opu_dbg", 0x0081_5e80, Conv::Cdecl, &[host, cmd], |o| {
                Out::U32(gj::opu_dbg(o, host, cmd))
            });
            t.reset();

            // the bind keys: records with handles (some repeated), bound or
            // not, characters and monsters or not
            let chr = t.rebase(mj::ET_CHR);
            let mut recs = vec![0_u8; 0x20 * 0xf0];
            Mem::read_bytes(&t.b, chr, &mut recs);
            if !(t.snapshot && case < 2) {
                let handles: Vec<u32> = (0..6).map(|_| rng.next() | 1).collect();
                for r in recs.chunks_mut(0x20) {
                    let hd = if rng.chance(3) {
                        0
                    } else {
                        handles[rng.below(6) as usize]
                    };
                    r[..4].copy_from_slice(&hd.to_le_bytes());
                    let who = if rng.chance(4) { 0 } else { rng.next() };
                    r[8..12].copy_from_slice(&who.to_le_bytes());
                    r[0xc] = rng.next() as u8;
                    r[0xd] = rng.next() as u8;
                }
            }
            t.patch(chr, &recs);
            t.patch(t.rebase(mj::OP_ET_BIND_OFF.sym), &[u8::from(case % 3 != 0)]);
            t.stub_sym(
                mj::OPU_UNIT_FREE_BY_ID,
                1,
                false,
                2,
                0,
                Ret::Val(rng.next()),
            );
            t.check(
                "op_et_bind_all_off_007fbc20",
                0x007f_bc20,
                Conv::Cdecl,
                &[],
                |o| {
                    gj::bind_all_off(o);
                    Out::None
                },
            );
            t.patch(t.rebase(mj::OP_ET_BIND_OFF.sym), &[u8::from(case % 3 != 0)]);
            // the bind bits as they were (the off key cleared them)
            t.patch(chr, &recs);
            let monsters: Vec<u32> = (0..0x100)
                .map(|_| {
                    if rng.chance(3) {
                        0xffff_ffff
                    } else {
                        rng.below(8)
                    }
                })
                .collect();
            let seq = t.seq(&monsters);
            t.stub_sym(mj::OPUS_GET_MONSTER_INDEX, 2, false, 1, 0, seq);
            t.stub_sym(mj::OP_ET_EFF, 3, false, 4, 0, Ret::Val(1));
            t.check(
                "op_et_bind_all_on_007fb940",
                0x007f_b940,
                Conv::Cdecl,
                &[],
                |o| {
                    gj::bind_all_on(o);
                    Out::None
                },
            );
            t.reset();

            // the number printer's tags
            let p = t.zeros(0x60);
            let end = p.wrapping_add(rng.below(0x200)).wrapping_sub(0x20);
            t.check(
                "opd_num_end_tag_00a5f2d0",
                0x00a5_f2d0,
                Conv::Cdecl,
                &[p, end],
                |o| Out::U32(gj::num_end_tag(o, p, end)),
            );
            let pal_byte = if case % 3 == 0 { 0 } else { rng.next() as u8 };
            t.patch(t.rebase(mj::OPD_NUM_PAL.sym), &[pal_byte]);
            t.patch(t.rebase(mj::OPD_NUM_FRAMES.sym), &rng.next().to_le_bytes());
            t.patch(
                t.rebase(mj::OPD_NUM_TBP.sym),
                &(rng.next() as u16).to_le_bytes(),
            );
            t.patch(
                t.rebase(mj::OPD_NUM_CBP.sym),
                &(rng.next() as u16).to_le_bytes(),
            );
            let pal = if case % 2 == 0 {
                0xffff_ffff
            } else {
                rng.next()
            };
            let tbw = rng.word();
            t.check(
                "opd_num_tex_tag_00a5e7b0",
                0x00a5_e7b0,
                Conv::Cdecl,
                &[p, tbw, pal],
                |o| Out::U32(gj::num_tex_tag(o, p, tbw, pal)),
            );
            t.reset();

            // the character classes: plain and high characters
            let block = t.zeros(0x200);
            let table = block + 0x100;
            let strings: Vec<u32> = (0..7)
                .map(|_| {
                    let mut s: Vec<u8> = (0..rng.below(6))
                        .map(|_| 1 + rng.below(0xff) as u8)
                        .collect();
                    s.push(0);
                    t.alloc(&s)
                })
                .collect();
            let args: Vec<u32> = std::iter::once(table)
                .chain(strings.iter().copied())
                .collect();
            t.check(
                "MsDbgFile_charClasses_007e12d0",
                0x007e_12d0,
                Conv::Cdecl,
                &args,
                |o| {
                    gj::char_classes(
                        o,
                        table,
                        [
                            strings[0], strings[1], strings[2], strings[3], strings[4], strings[5],
                            strings[6],
                        ],
                    );
                    Out::None
                },
            );
            t.reset();
        }
    });
}

// ---- round 5 batch 6 (ffx_glue_other::m5v_k) ----

#[test]
fn r5v_gdraw_context_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0a01);
        for case in 0..16_u32 {
            // the wrap: a handle from the pool, moved to the wrapped ring
            let s = t.zeros(0xb00);
            let pool = t.zeros(0x180);
            t.w32(s + 0xab0, pool);
            t.global(mk::GDRAW.sym, s);
            let r = t.zeros(0x40);
            for k in (0..0x40).step_by(4) {
                t.w32(r + k, rng.next());
            }
            let (prev, next, first) = (t.zeros(0x40), t.zeros(0x40), t.zeros(0x40));
            t.w32(r + 0x18, pool);
            t.w32(r + 0x1c, prev);
            t.w32(r + 0x20, next);
            t.w32(pool + 0x128 + 0x20, first);
            let code = memset_code();
            t.stub_sym_x(mk::MEMSET, 1, false, 3, 0, Ret::Val(1), &code);
            t.stub_sym(mk::POOL_ALLOC, 2, false, 3, 0, Ret::Val(r));
            let texture = rng.next();
            t.zero_frames();
            let ebp = frame_ebp(&t.a, 1);
            t.b_top = Some(ebp - 0x80);
            t.check(
                "gdraw_wrapTexture_00a383a0",
                0x00a3_83a0,
                Conv::Cdecl,
                &[texture],
                |o| Out::U32(gk::wrap_texture(o, ebp - 0x30, texture)),
            );
            r5v_same(t, "gdraw_wrapTexture_00a383a0", ebp - 0x30, 0x30);
            t.b_top = None;
            t.reset();

            // the teardown: no state, a state down, up with objects and pools
            let s = if case % 5 == 0 { 0 } else { t.zeros(0xb00) };
            if s != 0 {
                t.w32(s, u32::from(case % 5 != 1));
                let clear = t.stub_code(1, false, 1, 1, Ret::Val(1), false);
                let ctx = t.object(&[clear; 0x70], 0x10);
                t.w32(s + 4, ctx);
                let release = t.stub_code(2, false, 1, 1, Ret::Val(2), false);
                for off in [0x798_u32, 0x79c, 0x998, 0x9a8] {
                    if rng.chance(2) {
                        let obj = t.object(&[0, 0, release], 8);
                        t.w32(s + off, obj);
                    }
                }
                for off in [0xab0_u32, 0xab4] {
                    t.w32(s + off, if rng.chance(3) { 0 } else { rng.next() });
                }
            }
            t.global(mk::GDRAW.sym, s);
            let code = memset_code();
            t.stub_sym_x(mk::MEMSET, 3, false, 3, 0, Ret::Val(3), &code);
            t.stub_sym(mk::RELEASE_SLOTS, 4, false, 0, 0, Ret::Val(4));
            t.stub_sym(mk::RELEASE_SHADERS, 5, false, 0, 0, Ret::Val(5));
            t.stub_sym(mk::FREE_RESOURCES, 6, false, 1, 0, Ret::Val(6));
            t.stub_sym(mk::POOL_FREE_ALL, 7, false, 2, 0, Ret::Val(7));
            t.import_sym(mk::FREE_SLOT, 8, 1, 1, Ret::Val(8));
            t.zero_frames();
            let ebp = frame_ebp(&t.a, 0);
            t.b_top = Some(ebp - 0x80);
            t.check(
                "gdraw_destroyContext_00a37960",
                0x00a3_7960,
                Conv::Cdecl,
                &[],
                |o| {
                    gk::destroy_context(o, ebp - 0x30);
                    Out::None
                },
            );
            r5v_same(t, "gdraw_destroyContext_00a37960", ebp - 0x30, 0x30);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 7 (ffx_glue_other::m5v_l) ----

#[test]
fn r5v_opd_num_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0b01);
        for case in 0..16_u32 {
            // the queue: counts under and over the limit, heads anywhere
            let queue = t.rebase(ml::QUEUE);
            let words: Vec<u8> = (0..0x10).flat_map(|_| rng.next().to_le_bytes()).collect();
            for k in 0..32 {
                for w in 0..16 {
                    let at = queue + (k << 13) + 0x200 * (w + 1);
                    t.patch(at, &words[(4 * (w as usize % 4))..][..4]);
                }
            }
            for (g, v) in [
                (ml::PRINT_X.sym, rng.next() as u16),
                (ml::PRINT_Y.sym, rng.next() as u16),
                (ml::PRINT_COLUMN.sym, rng.next() as u16),
                (ml::QUEUE_X.sym, rng.next() as u16),
                (ml::QUEUE_Y.sym, rng.next() as u16),
                (ml::QUEUE_COUNT.sym, rng.below(40) as u16),
            ] {
                t.patch(t.rebase(g), &v.to_le_bytes());
            }
            t.global(ml::QUEUE_HEAD.sym, rng.next());
            t.global(
                ml::QUEUE_LIMIT.sym,
                if case % 3 == 0 { rng.below(8) } else { 32 },
            );
            t.stub_sym(ml::PREPARE, 1, false, 0, 0, Ret::Val(1));
            let seen = t.zeros(4);
            let mut code = vec![0x66, 0xa1];
            code.extend(t.rebase(ml::PRINT_COLUMN.sym).to_le_bytes()); // mov ax, [column]
            code.extend([0x66, 0xa3]);
            code.extend(seen.to_le_bytes()); // mov [seen], ax
            t.stub_sym_x(ml::OPD_NUM, 2, false, 14, 0, Ret::Val(2), &code);
            t.check(
                "opd_num_draw_queue_00a5e2e0",
                0x00a5_e2e0,
                Conv::Cdecl,
                &[],
                |o| Out::U32(gl::draw_queue(o)),
            );
            t.reset();

            // the hex digits: none, a few, negative values
            let packet = t.zeros(0x40 * 12);
            let ctx = t.zeros(0x14);
            t.w32(ctx + 8, packet);
            t.write(ctx + 0x10, &(rng.next() as u16).to_le_bytes());
            t.write(ctx + 0x12, &(rng.next() as u16).to_le_bytes());
            let font = t.zeros(0x1c);
            t.w32(font + 4, rng.next());
            for off in [0x16_u32, 0x18] {
                t.write(font + off, &(rng.next() as u16).to_le_bytes());
            }
            t.w8(font + 0x1a, rng.next() as u8);
            t.w8(font + 0x1b, rng.next() as u8);
            let n = match case % 4 {
                0 => 0,
                1 => 0xffff_ffff,
                _ => 1 + rng.below(8),
            };
            let value = rng.next();
            t.stub_sym(ml::TEX_TAG, 3, false, 3, 0, Ret::Val(0x40));
            let args = [value, n, ctx, font];
            t.zero_frames();
            let slots = t.arg_slots(&args);
            t.check(
                "opd_num_put_hex_00a5eea0",
                0x00a5_eea0,
                Conv::Cdecl,
                &args,
                |o| Out::U32(gl::put_hex(o, slots, value, n, ctx, font)),
            );
            t.same_stack("opd_num_put_hex_00a5eea0", slots, 16);
            t.b_top = None;
            t.reset();

            // the motion command: each nibble
            let unit = t.zeros(0x20);
            t.w8(unit + 0x18, rng.below(240) as u8);
            let host = t.zeros(0x400);
            t.w32(host + 0x220, unit);
            let nibble = [0_u16, 0x1000, 0x2000, 0x3000, 0x4000, 0xf000][(case % 6) as usize];
            t.write(
                host + 0x21a,
                &(nibble | (rng.next() as u16 & 0xfff)).to_le_bytes(),
            );
            let cmd = t.zeros(4);
            t.write(cmd + 2, &(rng.next() as u16).to_le_bytes());
            for (k, (sym, n)) in [
                (ml::COPY_1K_SAVE, 2),
                (ml::COPY_1K_RESTORE, 2),
                (ml::DRAW_MAIN_EFFECT, 1),
                (ml::DRAW_SET_SITUATION, 2),
                (ml::EN_CLIP, 2),
            ]
            .into_iter()
            .enumerate()
            {
                t.stub_sym(sym, 4 + k as u32, false, n, 0, Ret::Val(rng.next()));
            }
            t.zero_frames();
            let ebp = (call_top(&t.a) - 24) & !0xf;
            t.b_top = Some(ebp - 0x500);
            t.check(
                "opu_motion_etc_00804b70",
                0x0080_4b70,
                Conv::Cdecl,
                &[host, cmd],
                |o| Out::U32(gl::motion_etc(o, ebp - 0x410, host, cmd)),
            );
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 8 (ffx_glue_other::m5v_m) ----

#[test]
fn r5v_opu_file_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0c01);
        for case in 0..24_u32 {
            let cmd = t.zeros(10);
            t.write(cmd + 2, &u16::from(case % 6 == 0).to_le_bytes());
            for off in [4_u32, 6, 8] {
                t.write(cmd + off, &(rng.below(4) as u16).to_le_bytes());
            }
            let chr = t.rebase(mm::ET_CHR);
            let index = if case % 5 == 1 {
                0xffff_ffff
            } else {
                rng.below(0xd0)
            };
            // (what index -1 would read: a handle)
            t.patch(chr - 0x20, &rng.next().to_le_bytes());
            if index != 0xffff_ffff {
                let handle = if case % 7 == 2 { 0 } else { rng.next() };
                t.patch(chr + (index << 5), &handle.to_le_bytes());
            }
            t.stub_sym(mm::CHR_INDEX_BY_NO, 1, false, 1, 0, Ret::Val(index));
            t.stub_sym(mm::DBG_PRINTF, 2, false, 2, 0, Ret::Val(2));
            t.stub_sym(mm::TEX_NB_IMAGE, 3, false, 1, 0, Ret::Val(rng.below(4)));
            t.stub_sym(mm::TEX_NB_CLUT, 4, false, 1, 0, Ret::Val(rng.below(4)));
            // the image size and palette count through their pointers
            let mut code = vec![0x8b, 0x4c, 0x24, 0x0c, 0xc7, 0x01];
            code.extend(rng.below(64).to_le_bytes());
            code.extend([0x8b, 0x4c, 0x24, 0x10, 0xc7, 0x01]);
            code.extend(rng.below(64).to_le_bytes());
            t.stub_sym_x(mm::TEX_IMAGE, 5, false, 4, 0, Ret::Val(rng.next()), &code);
            let mut code = vec![0x8b, 0x4c, 0x24, 0x0c, 0xc7, 0x01];
            code.extend(rng.below(64).to_le_bytes());
            t.stub_sym_x(mm::TEX_CLUT, 6, false, 3, 0, Ret::Val(rng.next()), &code);
            t.import_sym(mm::SPRINTF_SLOT, 7, 4, 0, Ret::Val(7));
            let fds: Vec<u32> = (0..8)
                .map(|_| if rng.chance(3) { 0 } else { rng.next() })
                .collect();
            let seq = t.seq(&fds);
            t.stub_sym(mm::SCE_OPEN, 8, false, 2, 0, seq);
            t.stub_sym(mm::SCE_WRITE, 9, false, 3, 0, Ret::Val(9));
            t.stub_sym(mm::SCE_CLOSE, 10, false, 1, 0, Ret::Val(10));
            let host = rng.next();
            t.zero_frames();
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - 0x280);
            t.check("opu_file", 0x0081_4a00, Conv::Cdecl, &[host, cmd], |o| {
                Out::U32(gm2::opu_file(o, ebp - 0x224, cmd))
            });
            // what the callees see: the size words and the path (the rest
            // are the original's spills)
            r5v_same(t, "opu_file sizes", ebp - 0x220, 8);
            r5v_same(t, "opu_file count", ebp - 0x214, 4);
            r5v_same(t, "opu_file path", ebp - 0x204, 0x200);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 9 (ffx_glue_other::m5v_n) ----

#[test]
fn r5v_vfx_setup_material_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0d01);
        for case in 0..24_u32 {
            let g = t.zeros(0x60);
            t.w32(g + 0x3c, if case % 6 == 0 { 0 } else { 0xffff_ffff });
            let owner = t.zeros(0x30);
            let list = owner + 0x1c;
            t.w32(list, if case % 2 == 0 { list } else { rng.next() });
            t.w32(g + 0x20, owner);
            t.w32(g + 0x2c, rng.next());
            let segs = t.zeros(0x6c * 8);
            for k in 0..8 {
                t.w32(segs + 0x6c * k + 0x24, rng.next());
            }
            let mesh = t.zeros(0x98);
            t.w32(mesh + 0x94, segs);
            t.w32(g + 0x24, mesh);
            let n = match case % 5 {
                0 => 0,
                1 => 1,
                _ => 2 + rng.below(4),
            };
            let have = if case % 7 == 3 {
                n.saturating_sub(1)
            } else {
                n
            };
            let items: Vec<u32> = (0..have).map(|_| rng.next()).collect();
            let array = parray(t, &items, 0);
            t.w32(g + 0x14, array);
            // the names: some with "GL/" and a shader name after it
            let mut hits = vec![];
            let names: Vec<u32> = (0..n)
                .map(|k| {
                    let text: Vec<u8> = if rng.chance(3) {
                        b"plain".to_vec()
                    } else {
                        format!("x/GL/shader{k}").into_bytes()
                    };
                    let mut z = text.clone();
                    z.push(0);
                    let at = t.alloc(&z);
                    hits.push(
                        text.windows(3)
                            .position(|w| w == b"GL/")
                            .map_or(0, |p| at + p as u32),
                    );
                    at
                })
                .collect();
            let names_at = if names.is_empty() {
                t.zeros(4)
            } else {
                t.words(&names)
            };
            t.stub_sym(mn::SETUP_MATERIAL, 1, true, 3, 3, Ret::Val(rng.next()));
            let base = t.words(&[0, rng.next()]);
            let mut code = vec![0xc7, 0x41, 0x0c];
            code.extend(u32::from(case % 8 != 5).to_le_bytes()); // mov [ecx+0xc], found
            code.extend([0xc7, 0x41, 0x10]);
            code.extend(base.to_le_bytes()); // mov [ecx+0x10], base
            code.extend([0xc7, 0x01]);
            code.extend(4_u32.to_le_bytes()); // mov [ecx], 4
            t.stub_sym_x(mn::CURSOR_SEEK4, 2, true, 0, 0, Ret::Val(2), &code);
            let resized = if case % 9 == 4 { 0xd } else { 0 };
            t.stub_sym(mn::RESIZE_INLINE, 3, true, 1, 1, Ret::Val(resized));
            let blocks: Vec<u32> = (0..2).map(|_| t.zeros(0x40)).collect();
            let seq = t.seq(&blocks);
            t.stub_sym(mn::USER_MALLOC, 4, false, 1, 0, seq);
            let code = memset_code();
            t.stub_sym_x(mn::MEMSET, 5, false, 3, 0, Ret::Val(5), &code);
            let seq = t.seq(&hits);
            t.import_sym(mn::STRSTR_SLOT, 6, 2, 0, seq);
            let code = memcpy_code();
            t.extra = code;
            let s = t.stub_code(7, false, 3, 0, Ret::Val(7), false);
            let slot = t.rebase(mn::STRNCPY_SLOT);
            t.patch(slot, &s.to_le_bytes());
            let mats: Vec<u32> = (0..8).map(|_| rng.next()).collect();
            let seq = t.seq(&mats);
            t.stub_sym(mn::CREATE_MATERIAL, 8, true, 4, 4, seq);
            t.stub_sym(mn::SHADER_MANAGER, 9, false, 0, 0, Ret::Val(rng.next()));
            t.stub_sym(mn::SETUP_SPECIAL, 10, true, 4, 4, Ret::Val(10));
            let scene = rng.next();
            t.global(mn::SCENE.sym, scene);
            let args = [n, names_at, rng.next()];
            t.zero_frames();
            let ebp = frame_ebp(&t.a, 3);
            t.a.write(ebp - 0x84, &[0xaa; 0x80]).unwrap();
            t.b.write(ebp - 0x84, &[0xaa; 0x80]).unwrap();
            t.b_top = Some(ebp - 0x100);
            t.check(
                "VFXDynamicGeometry_setupMaterial_006a4240",
                0x006a_4240,
                Conv::Thiscall(g),
                &args,
                |o| {
                    Out::U32(gn::setup_material(
                        o,
                        ebp - mn::FRAME,
                        g,
                        args[0],
                        args[1],
                        args[2],
                    ))
                },
            );
            // the cursor and the name (the rest are the original's spills)
            r5v_same(t, "setupMaterial cursor", ebp - mn::FRAME, 0x20);
            r5v_same(t, "setupMaterial name", ebp - 0x84, 0x80);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 10 (ffx_glue_other::m5v_o) ----

/// `mov dword [ecx+off], v` for each pair.
fn ecx_stores(pairs: &[(u8, u32)]) -> Vec<u8> {
    let mut c = vec![];
    for &(off, v) in pairs {
        c.extend([0xc7, 0x41, off]);
        c.extend(v.to_le_bytes());
    }
    c
}

#[test]
fn r5v_setup_special_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0e01);
        for case in 0..32_u32 {
            let ebp = frame_ebp(&t.a, 4);
            let map = ebp - 0x1a4;
            let manager = t.zeros(0x80);
            let material = t.zeros(8);
            t.w32(material + 4, rng.next());
            let name = if case % 9 == 0 {
                0
            } else {
                t.alloc(b"vfx_shader\0")
            };
            // the map entries the iterator walks
            let count = rng.below(5);
            let nodes: Vec<u32> = (0..count)
                .map(|_| {
                    let nd = t.zeros(0x14);
                    let key = t.alloc(b"param\0") | u32::from(rng.chance(3));
                    t.w32(nd + 0xc, key);
                    let kind = t.alloc(b"type\0") | u32::from(rng.chance(3));
                    let value = t.alloc(b"0.5\0") | u32::from(rng.chance(3));
                    let v = t.words(&[kind, value]);
                    t.w32(nd + 0x10, v);
                    nd
                })
                .collect();
            let head = t.zeros(0x10);
            let first = nodes.first().copied().unwrap_or(head);
            let next = nodes.get(1).copied().unwrap_or(head);
            let rest: Vec<u32> = nodes.iter().skip(2).copied().chain([head, head]).collect();
            // the lookups: found in the first tree, the second, or neither
            let entry_src = t.zeros(8);
            let found = t.zeros(0x14);
            t.w32(found + 0x10, entry_src);
            let finds = match case % 4 {
                1 => vec![0, found],
                2 => vec![0, 0],
                _ => vec![found],
            };
            let seq = t.seq(&finds);
            t.stub_sym(mo::TREE_FIND, 1, true, 1, 1, seq);
            let key_block = if case % 5 == 4 { 0 } else { t.zeros(0x20) };
            t.stub_sym(mo::PHYRE_MALLOC, 2, false, 2, 0, Ret::Val(key_block));
            let code = memcpy_code();
            t.stub_sym_x(mo::MEMCPY, 3, false, 3, 0, Ret::Val(3), &code);
            t.stub_sym(mo::PHYRE_FREE, 4, false, 1, 0, Ret::Val(4));
            // the ctor's result (not its `this`) is what the lookup gets
            let pstring = t.zeros(8);
            t.stub_sym(mo::PSTRING_CTOR, 5, true, 1, 1, Ret::Val(pstring));
            // the copied map holds one node to tear down at times
            let leftover = t.zeros(0x10);
            t.w32(leftover + 0xc, rng.next());
            let code = ecx_stores(&[(0xc, if case % 3 == 0 { leftover } else { 0 })]);
            t.stub_sym_x(mo::COPY_SHADER_MAP, 6, true, 1, 1, Ret::Val(6), &code);
            let code = ecx_stores(&[(0, head), (4, first), (8, next)]);
            t.stub_sym_x(mo::ITERATOR_BEGIN, 7, true, 1, 1, Ret::Val(7), &code);
            let seq = t.seq(&rest);
            t.stub_sym(mo::TREE_SUCCESSOR, 8, true, 1, 1, seq);
            let params: Vec<u32> = (0..8)
                .map(|_| if rng.chance(4) { 0 } else { rng.next() | 1 })
                .collect();
            let seq = t.seq(&params);
            t.stub_sym(mo::FIND_PARAM, 9, true, 1, 1, seq);
            let hits: Vec<u32> = (0..40).map(|_| u32::from(rng.chance(3))).collect();
            let seq = t.seq(&hits);
            t.import_sym(mo::STRSTR_SLOT, 10, 2, 0, seq);
            t.stub_sym(mo::COMBINE_TEXTURE_NAME, 11, true, 2, 2, Ret::Val(11));
            t.stub_sym(mo::CLUSTER_MANAGER, 12, false, 0, 0, Ret::Val(rng.next()));
            let clusters: Vec<u32> = (0..8)
                .map(|_| if rng.chance(2) { 0 } else { rng.next() })
                .collect();
            let seq = t.seq(&clusters);
            t.stub_sym(mo::CLUSTER_BY_NAME, 13, true, 1, 1, seq);
            t.stub_sym(
                mo::ASYNC_LOAD_MANAGER,
                14,
                false,
                0,
                0,
                Ret::Val(rng.next()),
            );
            t.stub_sym(mo::LOAD_CLUSTER, 15, false, 11, 0, Ret::Val(15));
            let code = ecx_stores(&[(0, rng.next()), (0x10, rng.next())]);
            t.stub_sym_x(mo::TEXTURE_ITERATOR, 16, true, 1, 1, Ret::Val(16), &code);
            let code = ecx_stores(&[(0, rng.next()), (0x10, rng.next())]);
            t.stub_sym_x(mo::CUBE_ITERATOR, 17, true, 1, 1, Ret::Val(17), &code);
            t.stub_sym(mo::LINEAR_SAMPLER, 18, true, 1, 1, Ret::Val(rng.next()));
            t.stub_sym(mo::WRITE_VECTOR_PARAMETER, 19, true, 2, 2, Ret::Val(19));
            t.stub_sym(mo::WRITE_TEXTURE_PARAMETER, 20, true, 2, 2, Ret::Val(20));
            let cell = t.words(&[float_word(&mut rng)]);
            let s = t.stub_code(21, false, 1, 0, Ret::Val(0), false);
            let mut code = vec![0xd9, 0x05];
            code.extend(cell.to_le_bytes()); // fld dword [cell]
            code.push(0xe9);
            let at = t.a.alloc_code(&[0; 16]).unwrap();
            assert_eq!(t.b.alloc_code(&[0; 16]).unwrap(), at);
            code.extend(s.wrapping_sub(at + code.len() as u32 + 4).to_le_bytes());
            t.write(at, &code);
            t.patch(t.rebase(mo::ATOF_SLOT), &at.to_le_bytes());
            t.stub_sym(mo::SET_VALUE2, 22, true, 2, 2, Ret::Val(22));
            let mut code = vec![0xc7, 0x05];
            code.extend((map + 0xc).to_le_bytes());
            code.extend(map.to_le_bytes()); // mov dword [map+0xc], map
            t.stub_sym_x(mo::TREE_REMOVE, 23, true, 1, 1, Ret::Val(23), &code);
            t.stub_sym(mo::FREE_LIST_RESET, 24, true, 0, 0, Ret::Val(24));
            t.stub_sym(mo::FREE_LIST_DTOR, 25, true, 0, 0, Ret::Val(25));
            t.stub_sym(mo::TREE_CLEAR, 26, true, 0, 0, Ret::Val(26));
            let scene = rng.next();
            t.global(mo::SCENE.sym, scene);
            let material_arg = if case % 11 == 7 { 0 } else { material };
            let params_obj = rng.next();
            let which = rng.word();
            let args = [name, material_arg, params_obj, which];
            zero_deep(t, 0x400);
            // junk under the records' +4 (the setup clears it)
            for at in [ebp - 0x150, ebp - 0x140] {
                t.w32(at, rng.next() | 1);
            }
            t.b_top = Some(map - 0x80);
            t.check(
                "VfxShaderManager_setupSpecialShaderParameters",
                0x006c_1520,
                Conv::Thiscall(manager),
                &args,
                |o| {
                    go::setup_special(o, ebp, manager, name, material_arg, params_obj, which);
                    Out::None
                },
            );
            // what the callees see (the rest are the original's spills)
            r5v_same(t, "setupSpecial map and records", map, 0x7c);
            r5v_same(t, "setupSpecial key and name", ebp - 0x114, 0x104);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 11 (ffx_glue_other::m5v_p) ----

#[test]
fn r5v_op_script_compile_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0f01);
        for case in 0..16_u32 {
            for g in [
                mp::ERRORS,
                mp::WARNINGS,
                mp::STATE_WORD,
                mp::LAST_ERROR,
                mp::TREE_ROOT,
            ] {
                t.global(g.sym, rng.next());
            }
            t.patch(t.rebase(mp::FLAGS), &[0xaa; 0x10]);
            t.stub_sym(mp::CHAR_CLASSES, 1, false, 8, 0, Ret::Val(1));
            // the counters the reports print, set by a callee at times
            let mut code = vec![];
            if case % 3 != 1 {
                code.extend([0xc7, 0x05]);
                code.extend(t.rebase(mp::ERRORS.sym).to_le_bytes());
                code.extend(u32::from(case % 3 == 0).to_le_bytes());
            }
            code.extend([0xc7, 0x05]);
            code.extend(t.rebase(mp::WARNINGS.sym).to_le_bytes());
            code.extend(u32::from(case % 2 == 0).wrapping_mul(7).to_le_bytes());
            t.stub_sym_x(mp::HASH_NAMES, 2, false, 1, 0, Ret::Val(2), &code);
            let fd = match case % 4 {
                0 => 0x8000_0000 | rng.next(),
                1 => 0,
                _ => 1 + rng.below(64),
            };
            t.stub_sym(mp::SCE_OPEN, 3, false, 2, 0, Ret::Val(fd));
            let buf = if case % 5 == 2 { 0 } else { rng.next() };
            t.import_sym(mp::MALLOC_SLOT, 4, 1, 0, Ret::Val(buf));
            // the compiler: errors counted (on the failure path, the stubs
            // before it count), the used size and the write flag set
            let mut code = vec![0x8b, 0x4c, 0x24, 0x08]; // mov ecx, [esp+8]
            code.extend([0xc7, 0x41, 0x0c]);
            code.extend((rng.below(0x100).wrapping_sub(0x10)).to_le_bytes()); // used
            code.extend([0xc7, 0x41, 0x04]);
            code.extend(u32::from(rng.chance(2)).to_le_bytes()); // the write flag
            t.stub_sym_x(mp::COMPILE, 5, false, 2, 0, Ret::Val(5), &code);
            t.stub_sym(mp::DBG_PRINTF, 6, false, 2, 0, Ret::Val(6));
            t.stub_sym(mp::SCE_WRITE, 7, false, 3, 0, Ret::Val(7));
            t.import_sym(mp::FREE_SLOT, 8, 1, 0, Ret::Val(8));
            t.stub_sym(mp::SCE_CLOSE, 9, false, 1, 0, Ret::Val(9));
            // the tree free: the root as the state left it
            t.stub_sym(mp::FREE_TREE, 10, false, 1, 0, Ret::Val(10));
            let (source, path) = (rng.next(), rng.next());
            t.zero_frames();
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - 0x80);
            t.check(
                "op_script_compile_007e1010",
                0x007e_1010,
                Conv::Cdecl,
                &[source, path],
                |o| {
                    gp::compile(o, ebp - 0x14, source, path);
                    Out::None
                },
            );
            r5v_same(t, "op_script_compile_007e1010", ebp - 0x14, 0x14);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn r5v_fp_disp_debug_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0f02);
        for case in 0..12_u32 {
            t.global(mp::PPP_FP_DISP.sym, u32::from(case % 2 == 0));
            let n = if case % 4 == 0 { 0 } else { 1 + rng.below(10) };
            let data = if case % 5 == 1 {
                0
            } else {
                let d = t.zeros((0x64 + 0x50 * n + 0x10) as usize);
                t.write(d + 4, &(n as u16).to_le_bytes());
                for k in 0..n {
                    t.w32(d + 0x64 + 0x50 * k, rng.next());
                }
                d
            };
            t.global(mp::FP_DATA.sym, data);
            let run = t.rebase(mp::FP_RUN);
            let disp = t.rebase(mp::FP_DISP);
            for k in 0..n {
                let state = if rng.chance(3) {
                    0xffff_f000
                } else {
                    rng.next()
                };
                t.patch(run + 0x80 * k, &state.to_le_bytes());
                t.patch(run + 0x80 * k + 0x1d, &[u8::from(rng.chance(2))]);
                let flags = [1_u32, 2, 4, 8, 3, 0, 0x11, 0x18][rng.below(8) as usize];
                let w: Vec<u8> = [flags, rng.next(), rng.next()]
                    .iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect();
                t.patch(disp + 0x4c * k, &w);
            }
            // dbgPrintf logs as many words as each format takes: 10 for an
            // entry, 2 for the switch line, 1 for the others
            let ret = Ret::Val(rng.next());
            let s10 = t.stub_code(1, false, 10, 0, ret, false);
            let s2 = t.stub_code(1, false, 2, 0, ret, false);
            let s1 = t.stub_code(1, false, 1, 0, ret, false);
            let at = t.a.alloc_code(&[0; 40]).unwrap();
            assert_eq!(t.b.alloc_code(&[0; 40]).unwrap(), at);
            let mut c = vec![0x81, 0x7c, 0x24, 0x04];
            c.extend(t.rebase(mp::ENTRY_FORMAT).to_le_bytes()); // cmp [esp+4], entry
            c.extend([0x0f, 0x84]);
            c.extend(s10.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // je s10
            c.extend([0x81, 0x7c, 0x24, 0x04]);
            c.extend(t.rebase(mp::LIST_HEAD[1]).to_le_bytes()); // cmp [esp+4], switch
            c.extend([0x0f, 0x84]);
            c.extend(s2.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // je s2
            c.push(0xe9);
            c.extend(s1.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // jmp s1
            t.write(at, &c);
            t.jump(mp::DBG_PRINTF, at);
            t.check("pppFpDispDebug", 0x0072_9060, Conv::Cdecl, &[], |o| {
                Out::U32(gp::fp_disp_debug(o))
            });
            t.reset();
        }
    });
}

#[test]
fn r5v_fp_inf_print_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0f03);
        for case in 0..16_u32 {
            let player = if case % 3 == 0 { 0 } else { rng.next() };
            t.stub_sym(mp::GET_PLAYER, 1, false, 0, 0, Ret::Val(player));
            // the callees write their outputs
            let (x, y, z) = (
                float_word(&mut rng),
                float_word(&mut rng),
                float_word(&mut rng),
            );
            let mut code = vec![];
            for (k, v) in [(8_u8, x), (0xc, y), (0x10, z)] {
                code.extend([0x8b, 0x4c, 0x24, k]); // mov ecx, [esp+k]
                code.extend([0xc7, 0x01]);
                code.extend(v.to_le_bytes()); // mov dword [ecx], v
            }
            t.stub_sym_x(mp::CH_GET_POS, 2, false, 4, 0, Ret::Val(2), &code);
            let row: Vec<u32> = (0..4).map(|_| float_word(&mut rng)).collect();
            let mut code = vec![0x8b, 0x4c, 0x24, 0x04]; // mov ecx, [esp+4]
            for (k, v) in row.iter().enumerate() {
                code.extend([0xc7, 0x41, (0x30 + 4 * k) as u8]);
                code.extend(v.to_le_bytes()); // mov dword [ecx+0x30+4k], v
            }
            t.stub_sym_x(mp::INVERSE_MATRIX, 3, false, 2, 0, Ret::Val(3), &code);
            let code = memcpy_code_n(16);
            t.stub_sym_x(mp::COPY_VECTOR, 4, false, 2, 0, Ret::Val(4), &code);
            t.stub_sym(mp::SUB_VECTOR, 5, false, 3, 0, Ret::Val(5));
            let cell = t.words(&[float_word(&mut rng)]);
            let s = t.stub_code(6, false, 1, 0, Ret::Val(6), false);
            let mut c = vec![0xd9, 0x05];
            c.extend(cell.to_le_bytes()); // fld dword [cell]
            c.push(0xe9);
            let at = t.a.alloc_code(&[0; 16]).unwrap();
            assert_eq!(t.b.alloc_code(&[0; 16]).unwrap(), at);
            c.extend(s.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
            t.write(at, &c);
            t.jump(mp::VECTOR_LENGTH, at);
            t.stub_sym(mp::OP_NUM_XY_SET, 7, false, 2, 0, Ret::Val(7));
            t.stub_sym(mp::OP_NUM, 8, false, 1, 0, Ret::Val(rng.next()));
            // sprintf logs 3 words for the distance, 5 for the vectors
            let s5 = t.stub_code(9, false, 5, 0, Ret::Val(9), false);
            let s3 = t.stub_code(9, false, 3, 0, Ret::Val(9), false);
            let at = t.a.alloc_code(&[0; 24]).unwrap();
            assert_eq!(t.b.alloc_code(&[0; 24]).unwrap(), at);
            let mut c = vec![0x81, 0x7c, 0x24, 0x08];
            c.extend(t.rebase(mp::DISTANCE_FORMAT).to_le_bytes()); // cmp [esp+8], distance
            c.extend([0x0f, 0x84]);
            c.extend(s3.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // je s3
            c.push(0xe9);
            c.extend(s5.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // jmp s5
            t.write(at, &c);
            t.patch(t.rebase(mp::SPRINTF_SLOT), &at.to_le_bytes());
            t.zero_frames();
            let ebp = frame_ebp(&t.a, 0);
            t.b_top = Some(ebp - 0x200);
            t.check("pppFpInfPrint", 0x0072_9850, Conv::Cdecl, &[], |o| {
                Out::U32(gp::fp_inf_print(o, ebp - 0x178))
            });
            r5v_same(t, "pppFpInfPrint distance", ebp - 0x178, 4);
            r5v_same(t, "pppFpInfPrint vectors", ebp - 0x134, 0x30);
            t.b_top = None;
            t.reset();
        }
    });
}

/// `memcpy(dst, src)` of a fixed `n` bytes (`rep movsb`) after the log.
fn memcpy_code_n(n: u32) -> Vec<u8> {
    let mut c = vec![
        0x56, 0x57, 0x8b, 0x7c, 0x24, 0x0c, 0x8b, 0x74, 0x24, 0x10, 0xb9,
    ];
    c.extend(n.to_le_bytes()); // mov ecx, n
    c.extend([0xf3, 0xa4, 0x5f, 0x5e]); // rep movsb; pop edi; pop esi
    c
}

// ---- round 5 batch 12 (ffx_glue_other::m5v_q) ----

#[test]
fn r5v_gdraw_tile_begin_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_1001);
        for case in 0..32_u32 {
            let s = t.zeros(0xb00);
            let clear = t.stub_code(1, false, 5, 5, Ret::Val(1), false);
            let mut slots = vec![0; 0x36];
            slots[0x35] = clear;
            let ctx = t.object(&slots, 0x10);
            t.w32(s + 4, ctx);
            let (sw, sh) = (640 + rng.below(4), 480 + rng.below(4));
            t.w32(s + 0x6d0, sw);
            t.w32(s + 0x6d4, sh);
            t.w32(s + 0x6c0, rng.below(800));
            t.w32(s + 0x6c4, rng.below(600));
            t.w32(s + 0x784, rng.next());
            t.w32(s + 0x788, if rng.chance(2) { 0 } else { rng.next() });
            t.global(mq::GDRAW.sym, s);
            let (x0, y0, x1, y1) = if case % 5 == 0 {
                (0, 0, sw, if case % 10 == 0 { sh } else { rng.below(sh) })
            } else {
                let x0 = rng.below(700).wrapping_sub(30);
                let y0 = rng.below(500).wrapping_sub(30);
                (x0, y0, x0 + rng.below(300), y0 + rng.below(300))
            };
            let pad = rng.below(40);
            let stats = rng.next();
            for (k, sym) in [
                mq::FREE_RESOURCES,
                mq::RESET_STATE,
                mq::SET_PROJECTION_FOR_TARGET,
                mq::SET_VIEWPORT_FOR_TARGET,
                mq::BIND_OUTPUT,
            ]
            .into_iter()
            .enumerate()
            {
                let n = u32::from(sym == mq::FREE_RESOURCES || sym == mq::BIND_OUTPUT);
                t.stub_sym(sym, 2 + k as u32, false, n, 0, Ret::Val(rng.next()));
            }
            let args = [x0, y0, x1, y1, pad, stats];
            t.check(
                "gdraw_tile_begin_00a3a820",
                0x00a3_a820,
                Conv::Stdcall,
                &args,
                |o| Out::U32(gq::tile_begin(o, x0, y0, x1, y1, pad, stats)),
            );
            t.reset();
        }
    });
}

// ---- round 5 batch 13 (ffx_glue_other::m5v_r) ----

/// The printer's font record with random offsets, cell size and colour.
fn opd_font(t: &mut Twins, rng: &mut Rng) {
    let font = t.rebase(mr::FONT);
    t.patch(font + 4, &rng.next().to_le_bytes());
    for off in [0x16_u32, 0x18] {
        t.patch(font + off, &(rng.next() as u16).to_le_bytes());
    }
    t.patch(font + 0x1a, &[1 + rng.below(16) as u8, rng.next() as u8]);
}

#[test]
fn r5v_opd_num_front_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_1101);
        for case in 0..24_u32 {
            // the decimal emitter: negative, large and small values
            let packet = t.zeros(0x40 * 34);
            let ctx = t.zeros(0x18);
            t.w32(ctx + 8, packet);
            t.write(ctx + 0x10, &(rng.next() as u16).to_le_bytes());
            t.write(ctx + 0x12, &(rng.next() as u16).to_le_bytes());
            let font = t.zeros(0x20);
            t.w32(font + 4, rng.next());
            t.write(font + 0x16, &(rng.next() as u16).to_le_bytes());
            t.write(font + 0x18, &(rng.next() as u16).to_le_bytes());
            t.w8(font + 0x1a, rng.next() as u8);
            t.w8(font + 0x1b, rng.next() as u8);
            for table in [mr::GLYPH_U, mr::GLYPH_V] {
                let w: Vec<u8> = (0..10).flat_map(|_| rng.next().to_le_bytes()).collect();
                t.patch(t.rebase(table), &w);
            }
            for g in [mr::BLANK_U.sym, mr::BLANK_V.sym] {
                t.global(g, rng.next());
            }
            let value = match case % 4 {
                0 if case % 8 == 4 => u32::MAX,
                1 if case % 8 == 5 => 0,
                0 => 0x8000_0000 | rng.next(),
                1 => rng.below(100),
                2 => rng.next() & 0x7fff_ffff,
                _ => rng.below(100_000),
            };
            let n = if case % 7 == 3 { 0 } else { 1 + rng.below(9) };
            t.stub_sym(mr::TEX_TAG, 1, false, 3, 0, Ret::Val(0x40));
            let args = [value, n, ctx, font];
            t.zero_frames();
            let slots = t.arg_slots(&args);
            t.check(
                "opd_num_put_dec_00a5e880",
                0x00a5_e880,
                Conv::Cdecl,
                &args,
                |o| Out::U32(gr::put_dec(o, slots, value, n, ctx, font)),
            );
            t.same_stack("opd_num_put_dec_00a5e880", slots, 8);
            t.b_top = None;
            t.reset();

            // the text emitter
            opd_font(t, &mut rng);
            let texts: [&[u8]; 6] = [
                b"AB-cd:09,(x)_/.",
                b"pos x\ty %d",
                b"line\nnext",
                b"",
                b"$5 left",
                b"a\t\tb z9!",
            ];
            let mut s = texts[(case % 6) as usize].to_vec();
            s.push(0);
            let text = t.alloc(&s);
            let ctx = t.zeros(0x18);
            t.w32(ctx, text);
            t.w32(ctx + 8, packet);
            t.write(ctx + 0x10, &(rng.below(40) as u16).to_le_bytes());
            t.write(ctx + 0x12, &(rng.below(40) as u16).to_le_bytes());
            t.stub_sym(mr::TEX_TAG, 1, false, 3, 0, Ret::Val(0x40));
            let fontp = t.rebase(mr::FONT);
            t.check(
                "opd_num_put_text_00a5f040",
                0x00a5_f040,
                Conv::Cdecl,
                &[ctx, fontp],
                |o| Out::U32(gr::put_text(o, ctx)),
            );
            t.reset();

            // the front end: display off, formats with text, %x, %d, %f
            opd_font(t, &mut rng);
            let fpacket = t.zeros(0x40 * 80);
            t.global(mr::FONT_PACKET.sym, fpacket);
            t.patch(
                t.rebase(mr::DISPLAY.sym),
                &u16::from(case % 5 != 0).to_le_bytes(),
            );
            for g in [mr::PRINT_X.sym, mr::PRINT_Y.sym, mr::PRINT_COLUMN.sym] {
                t.patch(t.rebase(g), &(rng.below(30) as u16).to_le_bytes());
            }
            t.global(mr::DMA_CHANNEL.sym, rng.next());
            let formats: [&[u8]; 5] = [
                b"HP %4d/%4d",
                b"%x %12x %f %d",
                b"",
                b"F %2.3f end",
                b"%8x:%d:%.f",
            ];
            let mut f = formats[(case % 5) as usize].to_vec();
            f.push(0);
            let format = t.alloc(&f);
            let mut args = vec![format];
            for k in 0..8 {
                args.push(if k % 3 == 2 {
                    (f64::from(rng.below(2000)) / 7.0).to_bits() as u32
                } else {
                    rng.word()
                });
            }
            if case % 5 == 1 {
                // the double after two words, then a word (the float
                // printer stubbed: its arguments logged). No signalling NaN:
                // the oracle's `fld` keeps it signalling, the hardware's
                // quiets it as the port does
                args[3] = rng.next();
                args[4] = 0x4000_0000 | rng.below(0x10_0000);
                t.stub_sym(mr::PUT_FLOAT, 7, false, 6, 0, Ret::Val(7));
            }
            for (k, sym) in [
                mr::GS_SYNC_PATH,
                mr::FLUSH_CACHE,
                mr::DMA_SEND,
                mr::GS1_TRANSFER,
            ]
            .into_iter()
            .enumerate()
            {
                let n = [2, 1, 2, 0][k];
                t.stub_sym(sym, 2 + k as u32, false, n, 0, Ret::Val(rng.next()));
            }
            t.zero_frames();
            let slots = t.arg_slots(&args);
            t.check("opd_num", 0x00a5_d3c0, Conv::Cdecl, &args, |o| {
                Out::U32(gr::opd_num(o, slots))
            });
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn r5v_opd_flags_str_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_1102);
        for _ in 0..32_u32 {
            let cmd = t.zeros(12);
            t.write(cmd, &(rng.next() as u16).to_le_bytes());
            for k in 0..4 {
                t.write(cmd + 2 + 2 * k, &(rng.next() as u16).to_le_bytes());
            }
            let buf = t.zeros(0x100);
            // each sprintf "writes" a few characters
            let lengths: Vec<u32> = (0..16).map(|_| rng.below(6)).collect();
            let seq = t.seq(&lengths);
            // 3 words logged for an operand, 2 for the texts
            let s3 = t.stub_code(1, false, 3, 0, seq, false);
            let s2 = t.stub_code(1, false, 2, 0, seq, false);
            let at = t.a.alloc_code(&[0; 24]).unwrap();
            assert_eq!(t.b.alloc_code(&[0; 24]).unwrap(), at);
            let mut c = vec![0x81, 0x7c, 0x24, 0x08];
            c.extend(t.rebase(mr::OPERAND_FORMAT).to_le_bytes()); // cmp [esp+8], operand
            c.extend([0x0f, 0x84]);
            c.extend(s3.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // je s3
            c.push(0xe9);
            c.extend(s2.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // jmp s2
            t.write(at, &c);
            t.patch(t.rebase(mr::SPRINTF_SLOT), &at.to_le_bytes());
            let args = [buf, cmd];
            t.zero_frames();
            let slots = t.arg_slots(&args);
            t.check(
                "opd_debug_flags_str_00a65570",
                0x00a6_5570,
                Conv::Cdecl,
                &args,
                |o| Out::U32(gr::flags_str(o, slots, buf, cmd)),
            );
            t.same_stack("opd_debug_flags_str_00a65570", slots, 8);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn r5v_opd_load_font_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_1103);
        for _ in 0..8_u32 {
            let packet = t.rebase(mr::UPLOAD_PACKET);
            let words: Vec<u8> = (0..0x10).flat_map(|_| rng.next().to_le_bytes()).collect();
            t.patch(packet, &words);
            let ch = t.words(&[rng.next()]);
            t.global(mr::DMA_CHANNEL.sym, ch);
            for (k, (sym, n)) in [
                (mr::SET_DEF_LOAD_IMAGE, 8),
                (mr::GS_SYNC_PATH, 2),
                (mr::FLUSH_CACHE, 1),
                (mr::EXEC_LOAD_IMAGE, 2),
                (mr::DMA_SEND_N, 3),
            ]
            .into_iter()
            .enumerate()
            {
                t.stub_sym(sym, 1 + k as u32, false, n, 0, Ret::Val(rng.next()));
            }
            t.check(
                "opd_num_load_font_00a5e4a0",
                0x00a5_e4a0,
                Conv::Cdecl,
                &[],
                |o| Out::U32(gr::load_font(o)),
            );
            t.reset();
        }
    });
}

// ---- round 5 batch 14 (ffx_glue_other::m5v_s) ----

#[test]
fn r5v_kagra_render_data_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_1201);
        for case in 0..24_u32 {
            let groups = match case % 6 {
                0 => 0,
                1 => 31,
                _ => 1 + rng.below(5),
            };
            let mut body = vec![];
            let mut total = 0_i32;
            for i in 0..groups {
                let k = if rng.chance(5) {
                    0
                } else {
                    1 + rng.below(3) as i16
                };
                let mut g = vec![0_u8; 0x10 + 32 * usize::try_from(k.max(0)).unwrap()];
                g[1] = if case % 6 == 2 && i == groups - 1 {
                    2
                } else {
                    3
                };
                g[2..4].copy_from_slice(&k.to_le_bytes());
                g[8..12].copy_from_slice(&rng.next().to_le_bytes());
                g[12..16].copy_from_slice(&rng.next().to_le_bytes());
                total += i32::from(k);
                body.extend(g);
            }
            let mut model = vec![0_u8; 0x20];
            model[4..8].copy_from_slice(&0x20_u32.to_le_bytes());
            let count = if groups > 0 { total.max(1) } else { 0 };
            model[0x10..0x12].copy_from_slice(&(count as i16).to_le_bytes());
            model.extend(body);
            model.extend([0_u8; 0x400]);
            let model = t.alloc(&model);
            let obj = t.zeros(0x20);
            t.w32(obj + 0x1c, rng.next());
            t.stub_sym(ms::GET_PTR32, 1, false, 1, 0, Ret::Val(model));
            let code = memset_code();
            t.stub_sym_x(ms::MEMSET, 2, false, 3, 0, Ret::Val(2), &code);
            t.import_sym(ms::SPRINTF_SLOT, 3, 6, 0, Ret::Val(3));
            // the loaded texture's name into the second argument
            let loaded = u32::from(case % 7 != 4);
            let mut code = vec![0x8b, 0x4c, 0x24, 0x08]; // mov ecx, [esp+8]
            code.extend([0xc7, 0x01]);
            code.extend(u32::from_le_bytes(*b"tex_").to_le_bytes());
            code.extend([0xc7, 0x41, 0x04]);
            code.extend(u32::from_le_bytes([b'0' + (case % 10) as u8, b'1', 0, 0]).to_le_bytes());
            t.stub_sym_x(
                ms::IS_TEXTURE_LOADED,
                4,
                false,
                6,
                0,
                Ret::Val(loaded),
                &code,
            );
            t.stub_sym(ms::DBG_PRINTF, 5, false, 2, 0, Ret::Val(5));
            t.stub_sym(ms::VFX_CREATE, 6, false, 3, 0, Ret::Val(6));
            t.stub_sym(ms::VFX_CREATE_DATA_BUFFER, 7, false, 7, 0, Ret::Val(7));
            let args = [rng.next(), rng.next(), obj, rng.next(), rng.next()];
            zero_deep(t, 0x2800);
            let ebp = frame_ebp(&t.a, 5);
            t.b_top = Some(ebp - ms::FRAME - 0x40);
            t.check(
                "KaGra001_createRenderData_00715dc0",
                0x0071_5dc0,
                Conv::Cdecl,
                &args,
                |o| {
                    Out::U32(gs::create_render_data(
                        o, ebp, args[0], args[1], obj, args[4],
                    ))
                },
            );
            // what the callees see: the counts, names and the texture name
            for (at, n) in [
                (0x215c_u32, 0x78),
                (0x20e4, 0x78),
                (0x206c, 0x78),
                (0x1ff4, 0x1e00),
            ] {
                r5v_same(t, "KaGra001_createRenderData_00715dc0", ebp - at, n);
            }
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 15 (ffx_glue_other::m5v_t) ----

#[test]
fn r5v_fake_env_render_data_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_1501);
        for case in 0..30_u32 {
            let mode = match case % 5 {
                0 | 4 => 0,
                1 => 1,
                2 => 2 + rng.below(100),
                _ => 0x8000_0000 | rng.next(),
            };
            let groups = if case % 6 == 5 { 0 } else { 1 + rng.below(5) };
            let mut body = vec![];
            let mut total = 0_i32;
            for i in 0..groups {
                let last = i == groups - 1;
                // a group of another kind keeps the walk where it is: only
                // last, with k > 0
                let other = last && case % 3 == 2;
                let k = if !other && rng.chance(5) {
                    0
                } else {
                    1 + rng.below(3) as i16
                };
                let k = if last && total + i32::from(k) == 0 {
                    1
                } else {
                    k
                };
                let mut g = vec![0_u8; 0x10 + 0x28 * usize::try_from(k).unwrap_or(0)];
                g[1] = if other { 2 } else { 3 };
                g[2..4].copy_from_slice(&k.to_le_bytes());
                g[8..12].copy_from_slice(&rng.next().to_le_bytes());
                g[12..16].copy_from_slice(&rng.next().to_le_bytes());
                total += i32::from(k);
                body.extend(g);
            }
            let mut model = vec![0_u8; 0x20];
            model[4..8].copy_from_slice(&0x20_u32.to_le_bytes());
            model[0x10..0x12].copy_from_slice(&(total as i16).to_le_bytes());
            model.extend(body);
            model.extend([0_u8; 0x400]);
            let model = t.alloc(&model);
            let code = memset_code();
            t.stub_sym_x(mt::MEMSET, 2, false, 3, 0, Ret::Val(2), &code);
            t.import_sym(mt::SPRINTF_SLOT, 3, 6, 0, Ret::Val(3));
            // the loaded texture's name into the second argument
            let loaded = u32::from(case % 7 != 4);
            let mut code = vec![0x8b, 0x4c, 0x24, 0x08]; // mov ecx, [esp+8]
            code.extend([0xc7, 0x01]);
            code.extend(u32::from_le_bytes(*b"tex_").to_le_bytes());
            code.extend([0xc7, 0x41, 0x04]);
            code.extend(u32::from_le_bytes([b'0' + (case % 10) as u8, b'1', 0, 0]).to_le_bytes());
            t.stub_sym_x(
                mt::IS_TEXTURE_LOADED,
                4,
                false,
                6,
                0,
                Ret::Val(loaded),
                &code,
            );
            t.stub_sym(mt::DBG_PRINTF, 5, false, 2, 0, Ret::Val(5));
            t.stub_sym(mt::VFX_CREATE, 6, false, 3, 0, Ret::Val(6));
            t.stub_sym(mt::VFX_CREATE_DATA_BUFFER, 7, false, 7, 0, Ret::Val(7));
            let mut args = [0_u32; 9];
            for a in &mut args {
                *a = rng.next();
            }
            args[0] = model;
            args[8] = mode;
            zero_deep(t, 0x2400);
            let ebp = frame_ebp(&t.a, 9);
            // junk under the names (each copy brings its own NUL)
            t.write(ebp - 0x1f04, &[0xaa; 0x1e00]);
            t.b_top = Some(ebp - mt::FRAME - 0x40);
            t.check(
                "FakeEnvModel_createRenderData_00715af0",
                0x0071_5af0,
                Conv::Cdecl,
                &args,
                |o| Out::U32(gt::create_render_data(o, ebp, model, mode)),
            );
            // what the callees see: the counts, names and the texture name
            // (its last 0x10 bytes are where `_alloca_probe` saves registers)
            for (at, n) in [
                (0x206c_u32, 0x78),
                (0x1ff4, 0xf0),
                (0x1f04, 0x1e00),
                (0x104, 0xf0),
            ] {
                r5v_same(t, "FakeEnvModel_createRenderData_00715af0", ebp - at, n);
            }
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 16 (ffx_glue_other::m5v_u) ----

#[test]
fn r5v_packet_start_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_1601);
        for case in 0..24_u32 {
            for g in [
                mu::PACKET_STATE.sym,
                mu::PACKET_COUNT_LO.sym,
                mu::PACKET_COUNT_HI.sym,
                mu::PACKET_POS.sym,
            ] {
                t.global(g, rng.next());
            }
            t.global(
                mu::READ_BACK.sym,
                if case % 4 == 0 { 0 } else { rng.next() | 1 },
            );
            t.global(
                mu::BUFFER_SWITCH.sym,
                if case % 3 == 0 { 0 } else { rng.next() | 1 },
            );
            let xy = |rng: &mut Rng| match rng.below(3) {
                0 => rng.next(),
                1 => rng.below(2000).wrapping_sub(1000),
                _ => rng.below(0x200),
            };
            t.global(mu::CROSSHAIR_X.sym, xy(&mut rng));
            t.global(mu::CROSSHAIR_Y.sym, xy(&mut rng));
            t.global(mu::PIXELS, rng.next());
            for b in [mu::BUFFER_OFF, mu::BUFFER_ON] {
                let at = t.rebase(b);
                let w: Vec<u8> = (0..4).flat_map(|_| rng.next().to_le_bytes()).collect();
                t.patch(at, &w);
            }
            t.stub_sym(mu::GS_SYNC_PATH, 1, false, 2, 0, Ret::Val(1));
            t.stub_sym(mu::SET_DEF_STORE_IMAGE, 2, false, 8, 0, Ret::Val(2));
            t.stub_sym(mu::FLUSH_CACHE, 3, false, 1, 0, Ret::Val(3));
            t.stub_sym(mu::EXEC_STORE_IMAGE, 4, false, 2, 0, Ret::Val(4));
            t.stub_sym(mu::SCREEN_TO_WORLD, 5, false, 2, 0, Ret::Val(5));
            t.stub_sym(mu::PPP_SEND_PACKET, 6, false, 4, 0, Ret::Val(6));
            t.stub_sym(mu::PPP_CLEAR_WAIT, 7, false, 0, 0, Ret::Val(7));
            t.zero_frames();
            // the frame: ebx = top - 8, esp aligned down to 16 below
            // top - 16, ebp there
            let ebp = (call_top(&t.a) - 16) & !0xf;
            t.b_top = Some(ebp - mu::FRAME - 0x40);
            t.check(
                "pppPacketStart_00760f30",
                0x0076_0f30,
                Conv::Cdecl,
                &[],
                |o| {
                    gu::packet_start(o, ebp);
                    Out::None
                },
            );
            // what the callees see: the store record and the screen point
            for (at, n) in [(0x94_u32, 0x10), (0x80, 0x6c), (0x14, 0x10)] {
                r5v_same(t, "pppPacketStart_00760f30", ebp - at, n);
            }
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 17 (ffx_glue_other::m5v_v) ----

#[test]
fn r5v_gdraw_filter_quad_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_1701);
        for case in 0..24_u32 {
            // the device context: the five methods (stdcall, context first)
            let mut slots = vec![0; 0x50 / 4];
            for (k, (off, words)) in [
                (mv::vt::PS_SET_SHADER_RESOURCES, 4),
                (mv::vt::PS_SET_SHADER, 4),
                (mv::vt::PS_SET_SAMPLERS, 4),
                (mv::vt::UNMAP, 3),
                (mv::vt::PS_SET_CONSTANT_BUFFERS, 4),
            ]
            .into_iter()
            .enumerate()
            {
                slots[(off / 4) as usize] = t.stub_code(
                    0x20 + k as u32,
                    false,
                    words,
                    words,
                    Ret::Val(rng.next()),
                    false,
                );
            }
            let ctx = t.object(&slots, 0x10);
            let state = t.zeros(0xb00);
            t.w32(state + 4, ctx);
            for off in (0x204..0x284)
                .step_by(4)
                .chain([0x7b4, 0x994, 0xac0, 0xac4])
            {
                t.w32(state + off, rng.next());
            }
            t.global(mv::GDRAW.sym, state);
            let dst = t.zeros(0x80);
            t.stub_sym(mv::MAP_BUFFER, 1, false, 1, 0, Ret::Val(dst));
            let code = memcpy_code();
            t.stub_sym_x(mv::MEMCPY, 2, false, 3, 0, Ret::Val(2), &code);
            t.stub_sym(mv::DRAW_QUAD, 3, false, 6, 0, Ret::Val(rng.next()));
            let filter = t.zeros(0x40);
            if case % 3 != 0 {
                let input = t.zeros(0x40);
                t.w32(filter + 0x1c, input);
            }
            let n = rng.below(5);
            let w: Vec<u32> = (0..20).map(|_| rng.next()).collect();
            let constants = t.words(&w);
            let head = t.words(&w[..4]);
            let r: Vec<u32> = (0..4)
                .map(|_| match rng.below(3) {
                    0 => rng.next(),
                    1 => rng.below(4000).wrapping_sub(2000),
                    _ => (1 << 24) + rng.below(64),
                })
                .collect();
            let rect = t.words(&r);
            let args = [
                filter,
                n,
                constants,
                rect,
                rng.next(),
                rng.next(),
                head,
                rng.next(),
            ];
            let slots = t.arg_slots(&args);
            t.check(
                "gdraw_filterQuad_00a38aa0",
                0x00a3_8aa0,
                Conv::Cdecl,
                &args,
                |o| {
                    Out::U32(gv::filter_quad(
                        o, slots, filter, n, constants, rect, args[4], head, args[7],
                    ))
                },
            );
            t.same_stack("gdraw_filterQuad_00a38aa0", slots, 0x20);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 5 batch 18 (ffx_glue_other::m5v_w) ----

#[test]
fn r5v_gdraw_draw_indexed_triangles_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_1801);
        for case in 0..36_u32 {
            // the device context: the six methods (stdcall, context first)
            let mut slots = vec![0; 0x50 / 4];
            for (k, (off, words)) in [
                (mw::vt::VS_SET_SHADER, 4),
                (mw::vt::DRAW_INDEXED, 4),
                (mw::vt::UNMAP, 3),
                (mw::vt::IA_SET_INPUT_LAYOUT, 2),
                (mw::vt::IA_SET_VERTEX_BUFFERS, 6),
                (mw::vt::IA_SET_INDEX_BUFFER, 4),
            ]
            .into_iter()
            .enumerate()
            {
                slots[(off / 4) as usize] = t.stub_code(
                    0x20 + k as u32,
                    false,
                    words,
                    words,
                    Ret::Val(rng.next()),
                    false,
                );
            }
            let ctx = t.object(&slots, 0x10);
            let state = t.zeros(0xb00);
            t.w32(state + 4, ctx);
            for off in (0x260..0x290)
                .step_by(4)
                .chain([0x7a0, 0x998, 0x9a0, 0x9a8, 0xac0, 0xac4])
            {
                t.w32(state + off, rng.next());
            }
            let chunk = match case % 4 {
                0 => 0,
                1 => 4,
                _ => 4 + rng.below(12),
            };
            t.w32(state + 0x9c0, chunk);
            t.global(mw::GDRAW.sym, state);
            let proceed = u32::from(case % 9 != 4);
            t.stub_sym(mw::SET_RENDER_STATE, 1, false, 4, 0, Ret::Val(proceed));
            let locks: Vec<u32> = (0..16)
                .map(|_| if rng.chance(5) { 0 } else { t.zeros(0x400) })
                .collect();
            let seq = t.seq(&locks);
            t.stub_sym(mw::LOCK_DYNAMIC, 2, false, 2, 0, seq);
            let code = memcpy_code();
            t.stub_sym_x(mw::MEMCPY, 3, false, 3, 0, Ret::Val(3), &code);
            t.stub_sym(mw::UNLOCK, 4, false, 1, 0, Ret::Val(rng.next()));
            let verts: Vec<u8> = (0..0x100).flat_map(|_| rng.next().to_le_bytes()).collect();
            let verts = t.alloc(&verts);
            let indices = if case % 3 == 0 {
                0
            } else {
                let w: Vec<u32> = (0..0x10).map(|_| rng.next()).collect();
                t.words(&w)
            };
            let nverts = match rng.below(4) {
                0 => 0,
                1 => rng.below(4),
                _ => 4 + rng.below(36),
            };
            let p = t.words(&[verts, indices, nverts, rng.below(0x20), rng.below(3)]);
            let r = t.zeros(0x30);
            t.w8(
                r + 4,
                if rng.chance(2) {
                    0x10
                } else {
                    rng.next() as u8 & !0x10
                },
            );
            for off in [0x1c, 0x20] {
                if rng.chance(2) {
                    let o = t.zeros(0x40);
                    t.w32(r + off, o);
                }
            }
            let buffers = if case % 5 == 1 {
                t.words(&[rng.next(), rng.next(), 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
            } else {
                0
            };
            let stats = t.words(&[
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
            ]);
            let args = [r, p, buffers, stats];
            t.zero_frames();
            let slots = t.arg_slots(&args);
            let ebp = slots - 8;
            t.b_top = Some(ebp - mw::FRAME - 0x40);
            t.check(
                "gdraw_DrawIndexedTriangles_00a387e0",
                0x00a3_87e0,
                Conv::Stdcall,
                &args,
                |o| {
                    Out::U32(gw::draw_indexed_triangles(
                        o, ebp, slots, r, p, buffers, stats,
                    ))
                },
            );
            t.same_stack("gdraw_DrawIndexedTriangles_00a387e0", ebp - mw::FRAME, 0x10);
            t.same_stack("gdraw_DrawIndexedTriangles_00a387e0", slots, 0x10);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 6 (left6) batch 1 (ffx_glue_other::l6_vfx) ----

use ffx_glue_other::l6_vfx::{self as l6, bridge as l6v};

/// The stack top `+0xaac`: the main target (its base `+0x9cc`), an entry
/// with a size, or one without.
fn l6_target_top(t: &mut Twins, rng: &mut Rng, s: u32, case: u32) {
    let top = match case % 3 {
        0 => s + 0x9cc,
        k => {
            let e = t.zeros(0x20);
            for off in (4..0x14).step_by(4) {
                t.w32(e + off, rng.next());
            }
            t.w32(e + 0x18, if k == 1 { rng.next() | 1 } else { 0 });
            e
        }
    };
    t.w32(s + 0xaac, top);
    for off in (0x6d8..0x6f8).step_by(4) {
        t.w32(s + off, rng.next());
    }
}

#[test]
fn l6_gdraw_target_projection_matches_the_original() {
    // drawing whole (no call), the main target, an entry with and
    // without a size
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6c01);
        for case in 0..24_u32 {
            let s = gdraw_state(t, &mut rng);
            l6_target_top(t, &mut rng, s, case);
            t.w32(s + 0x6f8, u32::from(case % 4 == 3) * rng.next().max(1));
            t.stub_sym(l6::SET_PROJECTION, 1, false, 4, 0, Ret::Val(rng.next()));
            t.check(
                "gdraw_targetProjection_00a3c350",
                0x00a3_c350,
                Conv::Cdecl,
                &[],
                |o| l6v::target_projection(o).map_or(Out::None, Out::U32),
            );
            t.reset();
        }
    });
}

#[test]
fn l6_gdraw_rt_bind_matches_the_original() {
    // with and without the second depth view; the projection called or
    // not (then the viewport's eax)
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6c02);
        for case in 0..24_u32 {
            let s = gdraw_state(t, &mut rng);
            l6_target_top(t, &mut rng, s, case / 2);
            t.w32(s + 0x6f8, u32::from(case % 4 < 2));
            t.w32(s + 0x788, if case % 2 == 0 { 0 } else { rng.next() });
            if case % 5 == 0 {
                // no first depth view: still bound
                t.w32(s + 0x784, 0);
            }
            t.stub_sym(
                l6::DEPTH_STENCIL_STATE,
                1,
                false,
                4,
                0,
                Ret::Val(rng.next()),
            );
            t.stub_sym(l6::DRAW_TARGET_QUAD, 2, false, 3, 0, Ret::Val(rng.next()));
            t.stub_sym(l6::BIND_OUTPUT, 3, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(
                l6::SET_VIEWPORT_FOR_TARGET,
                4,
                false,
                0,
                0,
                Ret::Val(rng.next()),
            );
            t.stub_sym(l6::SET_PROJECTION, 5, false, 4, 0, Ret::Val(rng.next()));
            t.zero_frames();
            let mask = rng.next();
            let ebp = frame_ebp(&t.a, 1);
            t.b_top = Some(ebp - l6::RT_BIND_FRAME - 0x40);
            t.check(
                "gdraw_rt_bind_00a37410",
                0x00a3_7410,
                Conv::Stdcall,
                &[mask],
                |o| Out::U32(l6v::rt_bind(o, ebp, mask)),
            );
            t.same_stack("gdraw_rt_bind_00a37410", ebp - 0x30, 0x30);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn l6_ppp_mdl_rev_matrix_matches_the_original() {
    // the rotation and the view product for real; each sign flag on and
    // off; scales and rows with NaNs, infinities and zeros
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_6c11);
        for _ in 0..40_u32 {
            let work = t.zeros(0x200);
            for k in 0..0x80 {
                let v = match rng.below(6) {
                    0 => [0x7fc0_0001, 0xff80_0000, 0x7f80_0000, 0x8000_0000, 0]
                        [rng.below(5) as usize],
                    1 => rng.next() & 0xbfff_ffff,
                    _ => (rng.below(4000) as f32 / 256.0 - 8.0).to_bits(),
                };
                t.w32(work + 4 * k, v);
            }
            // the offsets: position, angles, scale, flags
            let offs = [0x00_u32, 0x10, 0x20, 0x30, 0x40, 0x50];
            let d = t.words(&offs);
            for k in 0..3 {
                t.w8(work + 0xa0 + 0x50 + 1 + k, u8::from(rng.chance(2)));
            }
            let rec = t.zeros(0x20);
            t.w32(rec + 0xc, d);
            let view = t.rebase(l6::VIEW_MATRIX_COPY);
            let mut old = vec![0; 64];
            Mem::read_bytes(&t.b, view, &mut old);
            t.patched.push((view, old));
            for k in 0..16 {
                t.w32(view + 4 * k, (rng.below(400) as f32 / 64.0 - 3.0).to_bits());
            }
            let unused = rng.next();
            t.check(
                "ppp_mdl_rev_matrix_007401d0",
                0x0074_01d0,
                Conv::Cdecl,
                &[work, unused, rec],
                |o| Out::U32(l6v::rev_matrix(o, work, rec)),
            );
            t.reset();
        }
    });
}

// ---- round 7 (left7) (ffx_glue_other::l7_vfx) ----

use ffx_glue_other::l7_vfx::bridge as l7v;

/// A C string in both oracles.
fn l7_cstr(t: &mut Twins, s: &[u8]) -> u32 {
    let mut b = s.to_vec();
    b.push(0);
    t.alloc(&b)
}

#[test]
#[allow(clippy::too_many_lines)]
fn l7_symbol_define_matches_the_original() {
    // a new symbol and a redefined one (with and without the file and
    // line, an old description or none); an empty value and 1..3 tokens;
    // a description or none; names with upper case and high bytes
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_7a13);
        let names: [&[u8]; 5] = [b"Abc_Z", b"x", b"MAP\xe9q", b"", b"loNG_namE9"];
        for case in 0..600_u32 {
            let name = l7_cstr(t, names[rng.below(5) as usize]);
            let value = if rng.chance(4) {
                l7_cstr(t, b"")
            } else {
                l7_cstr(t, b"a + B * 3")
            };
            let desc = if rng.chance(3) {
                0
            } else {
                l7_cstr(t, [&b"desc"[..], b"", b"Some Text"][rng.below(3) as usize])
            };
            let kind = rng.next();
            let old = if case % 2 == 0 {
                0
            } else {
                let w: Vec<u32> = (0..9).map(|_| rng.next()).collect();
                let s = t.words(&w);
                if rng.chance(3) {
                    t.w32(s + 4, 0);
                }
                // its token list's tail: a word to append at
                let tl = t.zeros(4);
                t.w32(s + 0xc, tl);
                s
            };
            let tail = t.zeros(8);
            t.global(l7v::SYMBOL_TAIL, tail);
            t.global(l7v::SCRIPT_ERRORS, [0, rng.next()][rng.below(2) as usize]);
            t.global(l7v::SCRIPT_WARNINGS, [0, rng.next()][rng.below(2) as usize]);
            t.stub_sym(l7v::SYMBOL_FIND, 1, false, 2, 0, Ret::Val(old));
            let bufs: Vec<u32> = (0..12)
                .map(|_| {
                    let w: Vec<u32> = (0..0x10).map(|_| rng.next()).collect();
                    t.words(&w)
                })
                .collect();
            let bufs = t.seq(&bufs);
            t.stub_sym(l7v::MS_MALLOC, 2, false, 1, 0, bufs);
            t.stub_sym(l7v::OP_STRDUP, 3, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(l7v::MS_FREE, 4, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(l7v::OP_STRLEN, 5, false, 1, 0, Ret::Val(rng.below(20)));
            t.stub_sym(l7v::DBG_PRINTF, 6, false, 2, 0, Ret::Val(rng.next()));
            // the tokenizer: 0..3 tokens (each a short string written into
            // the caller's buffer), then 0
            let n = rng.below(4);
            let mut rets: Vec<u32> = (0..n).map(|_| rng.next() | 1).collect();
            rets.push(0);
            let rets = t.seq(&rets);
            let (lo, hi) = (
                [0x4142_6178_u32, 0x0000_3231, 0x3132_5f5a][rng.below(3) as usize],
                [0x0000_0063_u32, 0, 0x0041_e941][rng.below(3) as usize],
            );
            let mut x = vec![0x8b, 0x4c, 0x24, 0x08]; // mov ecx, [esp+8]
            x.extend([0xc7, 0x01]);
            x.extend(lo.to_le_bytes()); // mov dword [ecx], lo
            x.extend([0xc7, 0x41, 0x04]);
            x.extend(hi.to_le_bytes()); // mov dword [ecx+4], hi
            t.stub_sym_x(l7v::TOKEN_EXPR, 7, false, 5, 0, rets, &x);
            // a random frame (a store of 0 over 0 shows nothing)
            let top = call_top(&t.a);
            let fill: Vec<u8> = (0..0x100).flat_map(|_| rng.next().to_le_bytes()).collect();
            t.a.write(top - 0x400, &fill).unwrap();
            t.b.write(top - 0x400, &fill).unwrap();
            let ebp = frame_ebp(&t.a, 4);
            t.b_top = Some(ebp - l7v::DEFINE_FRAME - 0x40);
            t.check(
                "op_symbol_define_007e2770",
                0x007e_2770,
                Conv::Cdecl,
                &[name, value, desc, kind],
                |o| Out::U32(l7v::symbol_define(o, ebp, name, value, desc, kind)),
            );
            t.same_stack("op_symbol_define_007e2770", ebp - 0x210, 0x20c);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
#[allow(clippy::too_many_lines)]
fn l7_macro_expand_matches_the_original() {
    // no macro, no body; no parameters, 1..3 (too few and too many
    // arguments, the arguments not opening with "("); body tokens of
    // every class, identifiers expanded (at depths 0..3; the inner
    // expansions find no macro), saved values or none
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_7a14);
        for case in 0..500_u32 {
            let strs: Vec<u32> = (0..4)
                .map(|k| l7_cstr(t, [&b"ab"[..], b"", b"Zq9", b"x"][k]))
                .collect();
            let np = rng.below(4);
            let mut params = 0;
            for _ in 0..np {
                let w: Vec<u32> = (0..9).map(|_| rng.next()).collect();
                let p = t.words(&w);
                t.w32(p + 4, if rng.chance(3) { 0 } else { strs[0] });
                t.w32(p + 0x20, params);
                params = p;
            }
            let mut w: Vec<u32> = (0..9).map(|_| rng.next()).collect();
            w[1] = if case % 19 == 4 { 0 } else { rng.next() | 1 };
            w[2] = params;
            let sym = t.words(&w);
            let len = [0_u16, 1, 2, 4, 0xffff][rng.below(5) as usize];
            t.write(sym + 0x16, &len.to_le_bytes());
            // the macro, then nothing for the expansions inside it: the
            // original recursing into itself runs in a frame of its own
            // whose buffers the stubs would log at different addresses
            // (so an inner expansion returns -1 here, never 0)
            let finds = if case % 23 == 7 {
                t.seq(&[0])
            } else {
                t.seq(&[sym, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
            };
            t.stub_sym(l7v::SYMBOL_FIND, 1, false, 2, 0, finds);
            let open: u32 = [0x78, 0x79, 0x28, 0x28, 0x28][rng.below(5) as usize];
            let mut x = vec![0x8b, 0x4c, 0x24, 0x0c]; // mov ecx, [esp+0xc]
            x.extend([0xc7, 0x01]);
            x.extend(open.to_le_bytes()); // mov dword [ecx], "(" or "x"
            t.stub_sym_x(l7v::TOKEN_GET, 2, false, 7, 0, Ret::Val(rng.next()), &x);
            let args: Vec<u32> = (0..5)
                .map(|_| [0, 1, 1, 2][rng.below(4) as usize])
                .collect();
            let args = t.seq(&args);
            let mut x = vec![0x8b, 0x4c, 0x24, 0x08]; // mov ecx, [esp+8]
            x.extend([0xc7, 0x01]);
            x.extend(0x0031_6261_u32.to_le_bytes()); // mov dword [ecx], "ab1"
            x.extend([0x8b, 0x4c, 0x24, 0x0c, 0xff, 0x01]); // mov ecx, [esp+0xc]; inc dword [ecx]
            t.stub_sym_x(l7v::TOKEN_EXPR, 3, false, 5, 0, args, &x);
            let copies: Vec<u32> = (0..6).map(|k| strs[k % 4]).collect();
            let copies = t.seq(&copies);
            t.stub_sym(l7v::OP_STRDUP, 4, false, 1, 0, copies);
            // the body's tokens: classes, each moving the position by 1
            let classes: Vec<u32> = (0..24)
                .map(|_| [1, 2, 3, 2][rng.below(4) as usize])
                .collect();
            let classes = t.seq(&classes);
            let x = vec![0x8b, 0x4c, 0x24, 0x0c, 0xff, 0x01]; // mov ecx, [esp+0xc]; inc dword [ecx]
            t.stub_sym_x(l7v::TOKEN, 5, false, 5, 0, classes, &x);
            let saved: Vec<u32> = (0..8)
                .map(|_| {
                    if rng.chance(3) {
                        0
                    } else {
                        let w: Vec<u32> = (0..4).map(|_| rng.next()).collect();
                        t.words(&w)
                    }
                })
                .collect();
            let saved = t.seq(&saved);
            t.stub_sym(l7v::LIST_VALUES, 6, false, 1, 0, saved);
            t.stub_sym(l7v::MS_FREE, 7, false, 1, 0, Ret::Val(rng.next()));
            let lens = t.seq(&(0..32).map(|_| rng.below(4)).collect::<Vec<u32>>());
            t.stub_sym(l7v::OP_STRLEN, 8, false, 1, 0, lens);
            t.stub_sym(l7v::DBG_PRINTF, 9, false, 2, 0, Ret::Val(rng.next()));
            t.global(l7v::SCRIPT_ERRORS, [0, rng.next()][rng.below(2) as usize]);
            t.global(l7v::SCRIPT_WARNINGS, [0, rng.next()][rng.below(2) as usize]);
            t.global(l7v::EXPAND_DEPTH, rng.below(4));
            t.global(l7v::SCRIPT_TREE, rng.next());
            let out = t.zeros(0x40);
            let pos = t.words(&[rng.below(8)]);
            let (tree, src, end, eol) = (rng.next(), rng.next(), rng.below(0x30), rng.next());
            let top = call_top(&t.a);
            let fill: Vec<u8> = (0..0x100).flat_map(|_| rng.next().to_le_bytes()).collect();
            t.a.write(top - 0x400, &fill).unwrap();
            t.b.write(top - 0x400, &fill).unwrap();
            let call = [tree, src, out, pos, end, eol];
            let slots = t.arg_slots(&call);
            let ebp = frame_ebp(&t.a, 6);
            t.b_top = Some(ebp - l7v::EXPAND_FRAME - 0x40);
            t.check(
                "op_macro_expand_007e2a60",
                0x007e_2a60,
                Conv::Cdecl,
                &call,
                |o| Out::U32(l7v::macro_expand(o, ebp, slots, tree, src, out, pos, end)),
            );
            t.same_stack("op_macro_expand_007e2a60", ebp - 0x21c, 0x218);
            t.same_stack("op_macro_expand_007e2a60 slots", slots, 0x18);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
#[allow(clippy::too_many_lines)]
fn l7_preprocess_matches_the_original() {
    // the file missing; read with a dirty buffer or not, a handle or
    // none; lines with every token class, directives (skipping or not,
    // the ones that run while skipping), expansions, directives that take
    // lines; an outer file or none
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_7a15);
        for case in 0..500_u32 {
            let fd = [0xffff_ffff_u32, 0, 3, 7][rng.below(4) as usize];
            let fd = if case % 9 == 4 { 0xffff_ffff } else { fd };
            let (buf, dirty, size) = (
                [0, rng.next() | 1][rng.below(2) as usize],
                rng.below(2),
                [0, 5, 0xffff_ffff][rng.below(3) as usize],
            );
            let mut x = vec![0x8b, 0x4c, 0x24, 0x04]; // mov ecx, [esp+4]
            for (o, v) in [(0_u8, buf), (4, dirty), (8, fd), (0xc, size)] {
                x.extend([0xc7, 0x41, o]);
                x.extend(v.to_le_bytes()); // mov dword [ecx+o], v
            }
            t.stub_sym_x(l7v::FILE_READ, 1, false, 2, 0, Ret::Val(rng.next()), &x);
            let ns: Vec<u32> = (0..8).map(|_| rng.below(4)).collect();
            let ns = t.seq(&ns);
            t.stub_sym(l7v::READ_LINE, 2, false, 3, 0, ns);
            let classes: Vec<u32> = (0..64).map(|_| rng.below(4)).collect();
            let classes = t.seq(&classes);
            let mut x = vec![0x8b, 0x4c, 0x24, 0x0c, 0xff, 0x01]; // mov ecx, [esp+0xc]; inc dword [ecx]
            x.extend([0xc7, 0x05]);
            x.extend(t.rebase(l7v::IN_COMMENT).to_le_bytes());
            x.extend(1_u32.to_le_bytes()); // mov dword [comment], 1 (as the tokenizer's comments do)
            t.stub_sym_x(l7v::TOKEN, 3, false, 5, 0, classes, &x);
            let mut d: Vec<u32> = (0..8).map(|_| rng.next()).collect();
            d[4] = rng.below(8) | (rng.next() << 16);
            d[5] = [0, 1][rng.below(2) as usize];
            let d = t.words(&d);
            let finds: Vec<u32> = (0..16).map(|_| [0, d][rng.below(2) as usize]).collect();
            let finds = t.seq(&finds);
            t.stub_sym(l7v::SYMBOL_FIND, 4, false, 2, 0, finds);
            // a directive that took two more lines
            let mut x = vec![0xc7, 0x05];
            x.extend(t.rebase(l7v::LINE_COUNT).to_le_bytes());
            x.extend(rng.below(3).to_le_bytes()); // mov dword [count], k
            t.stub_sym_x(l7v::DIRECTIVE, 5, false, 7, 0, Ret::Val(rng.next()), &x);
            // the callees see the script's file, line and depth: copied to a
            // probe the diff compares
            let probe = t.zeros(16);
            let mut x = vec![];
            for (k, g) in [l7v::SCRIPT_ERRORS, l7v::SCRIPT_WARNINGS, l7v::EXPAND_DEPTH]
                .into_iter()
                .enumerate()
            {
                x.push(0x8b);
                x.push(0x15);
                x.extend(t.rebase(g).to_le_bytes()); // mov edx, [g]
                x.extend([0x89, 0x15]);
                x.extend((probe + 4 * k as u32).to_le_bytes()); // mov [probe+4k], edx
            }
            t.stub_sym_x(l7v::MACRO_EXPAND, 6, false, 6, 0, Ret::Val(rng.next()), &x);
            t.global(l7v::EXPAND_DEPTH, rng.next() | 1);
            let ends: Vec<u32> = (0..6)
                .map(|k| {
                    if k == 5 || rng.chance(3) {
                        0xffff_ffff
                    } else {
                        0
                    }
                })
                .collect();
            let ends = t.seq(&ends);
            t.stub_sym(l7v::LINE_LENGTH, 7, false, 2, 0, ends);
            for (sym, tag, n) in [
                (l7v::FILE_PUTS, 8, 2),
                (l7v::SCE_WRITE, 9, 3),
                (l7v::SCE_CLOSE, 10, 1),
                (l7v::DBG_PRINTF, 11, 2),
            ] {
                t.stub_sym(sym, tag, false, n, 0, Ret::Val(rng.next()));
            }
            let s = t.stub_code(12, false, 5, 0, Ret::Val(rng.next()), false);
            t.global(l7v::SPRINTF_SLOT, s);
            let f = t.stub_code(13, false, 1, 0, Ret::Val(rng.next()), false);
            t.global(l7v::FREE_SLOT, f);
            t.global(l7v::SCRIPT_ERRORS, [0, rng.next()][rng.below(2) as usize]);
            t.global(l7v::SCRIPT_WARNINGS, rng.next());
            t.global(l7v::SKIP_LEVEL, rng.below(2));
            let flags = [u32::from(rng.chance(3)) | (u32::from(rng.chance(2)) << 8)];
            let at = t.rebase(l7v::SKIP_FLAGS);
            t.patch(at, &flags[0].to_le_bytes());
            let (path, out) = (rng.next(), rng.next());
            let top = call_top(&t.a);
            let fill: Vec<u8> = (0..0x200).flat_map(|_| rng.next().to_le_bytes()).collect();
            t.a.write(top - 0x800, &fill).unwrap();
            t.b.write(top - 0x800, &fill).unwrap();
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - l7v::PRE_FRAME - 0x40);
            t.check(
                "op_preprocess_007e2270",
                0x007e_2270,
                Conv::Cdecl,
                &[path, out],
                |o| Out::U32(l7v::preprocess(o, ebp, path, out)),
            );
            t.same_stack("op_preprocess_007e2270", ebp - 0x424, 0x420);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
#[allow(clippy::too_many_lines)]
fn l7_directive_matches_the_original() {
    // every kind 0..10 (and past them); names empty, quoted, "1"; the
    // operator "==" or not, the symbol found or not, with a body or none;
    // the conditional stack at every level and flag, full, empty
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_7a16);
        let words = [
            0x0000_4241_u32, // "AB"
            0x0000_3d3d,     // "=="
            0x0000_0031,     // "1"
            0,               // ""
            0x0027_7827,     // "'x'"
            0x0022_2222,     // "\"\"\""
            0x0000_3132,     // "21"
        ];
        for _ in 0..1500_u32 {
            // the tokens the three reads return
            let toks: Vec<u32> = (0..4).map(|_| words[rng.below(7) as usize]).collect();
            let toks = t.words(&toks);
            let cell = t.words(&[toks]);
            let mut x = vec![0x8b, 0x4c, 0x24, 0x0c]; // mov ecx, [esp+0xc]
            x.extend([0x8b, 0x15]);
            x.extend(cell.to_le_bytes()); // mov edx, [cell]
            x.extend([0x8b, 0x12, 0x89, 0x11]); // mov edx, [edx]; mov [ecx], edx
            x.extend([0x83, 0x05]);
            x.extend(cell.to_le_bytes());
            x.push(0x04); // add dword [cell], 4
            t.stub_sym_x(l7v::TOKEN_GET, 1, false, 7, 0, Ret::Val(rng.next()), &x);
            let body = t.words(&[words[rng.below(7) as usize], 0]);
            let mut sym: Vec<u32> = (0..9).map(|_| rng.next()).collect();
            sym[0] = t.words(&[rng.next()]);
            sym[1] = [0, body][rng.below(2) as usize];
            let sym = t.words(&sym);
            let found = [0, sym][rng.below(2) as usize];
            t.stub_sym(l7v::SYMBOL_FIND, 2, false, 2, 0, Ret::Val(found));
            for (s, tag, n) in [
                (l7v::TOKEN_PAREN, 3, 6),
                (l7v::TOKEN_LINE, 4, 7),
                (l7v::PREPROCESS, 5, 2),
                (l7v::SYMBOL_DEFINE, 6, 4),
                (l7v::RANGECHECK_FAILURE, 7, 0),
                (l7v::DBG_PRINTF, 8, 2),
            ] {
                t.stub_sym(s, tag, false, n, 0, Ret::Val(rng.next()));
            }
            let flags: Vec<u8> = (0..16).map(|_| rng.below(4) as u8).collect();
            let at = t.rebase(l7v::SKIP_FLAGS);
            t.patch(at, &flags);
            let level = [0_u32, 1, 2, 7, 0xe, 0xf, 0x10, 0xffff_ffff][rng.below(8) as usize];
            let kind = rng.below(12);
            // an #else at a level past the flags whose byte is 1 reaches
            // the CRT's range check failure, which never returns (in the
            // emulator its stub would): not run here
            let level = if kind == 6 && level > 0xf { 2 } else { level };
            t.global(l7v::SKIP_LEVEL, level);
            t.global(l7v::LINE_COUNT, rng.next());
            t.global(l7v::SCRIPT_ERRORS, [0, rng.next()][rng.below(2) as usize]);
            t.global(l7v::SCRIPT_WARNINGS, [0, rng.next()][rng.below(2) as usize]);
            t.global(l7v::SCRIPT_TREE, rng.next());
            let a = [
                rng.next(),
                rng.next(),
                rng.next(),
                kind,
                rng.next(),
                rng.next(),
                rng.next(),
            ];
            let top = call_top(&t.a);
            let fill: Vec<u8> = (0..0x200).flat_map(|_| rng.next().to_le_bytes()).collect();
            t.a.write(top - 0x800, &fill).unwrap();
            t.b.write(top - 0x800, &fill).unwrap();
            let ebp = frame_ebp(&t.a, 7);
            t.b_top = Some(ebp - l7v::DIRECTIVE_FRAME - 0x40);
            t.check("op_directive_007e15f0", 0x007e_15f0, Conv::Cdecl, &a, |o| {
                Out::U32(l7v::directive(
                    o, ebp, a[0], a[1], a[2], a[3], a[4], a[5], a[6],
                ))
            });
            t.same_stack("op_directive_007e15f0", ebp - 0x614, 0x610);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 8 (left8) (ffx_glue_other::l8_vfx) ----

use ffx_glue_other::l8_vfx::bridge as l8v;

#[test]
fn l8_opus_map_del_draw() {
    // paused or not; progress below, at and above 1 and NaN; offsets and
    // positions in and out of ftol's range; the argument slots
    Twins::each(|t| {
        let mut rng = Rng(0x00a7_5170);
        let n = l8v::SPRITES;
        let rec_bytes = 0x20 + 0x58 * n;
        let pkt_bytes = 0x80 + 0x60 * n;
        let (work, obj, recs, offset, packet) = (
            t.zeros(0x220),
            t.zeros(0x100),
            t.zeros(rec_bytes as usize),
            t.zeros(8),
            t.zeros(pkt_bytes as usize),
        );
        for round in 0..6 {
            let paused = u16::from(round % 3 == 2);
            t.write(work + 0x212, &paused.to_le_bytes());
            for o in [0x48, 0x4c] {
                let v = float_word(&mut rng);
                t.w32(obj + o, v);
            }
            for o in [0xc2, 0xc3, 0xcb] {
                t.w8(obj + o, rng.next() as u8);
            }
            for o in [0, 4] {
                let v = if rng.chance(3) {
                    float_word(&mut rng)
                } else {
                    (rng.below(1000) as f32 * 0.37).to_bits()
                };
                t.w32(offset + o, v);
            }
            let mut bytes = Vec::with_capacity(rec_bytes as usize);
            for k in 0..rec_bytes / 4 {
                let o = k * 4;
                let v = if o >= 0x20 && matches!((o - 0x20) % 0x58, 0x48 | 0x50) {
                    // progress: near 1, NaN, anything
                    match rng.below(4) {
                        0 => (0.9 + rng.below(20) as f32 * 0.01).to_bits(),
                        1 => 1.0_f32.to_bits(),
                        _ => float_word(&mut rng),
                    }
                } else if o >= 0x20 && matches!((o - 0x20) % 0x58, 0x4c | 0x54) {
                    match rng.below(3) {
                        0 => (rng.below(10) as f32 * 0.02).to_bits(),
                        _ => float_word(&mut rng),
                    }
                } else {
                    float_word(&mut rng)
                };
                bytes.extend(v.to_le_bytes());
            }
            t.write(recs, &bytes);
            let junk: Vec<u8> = (0..pkt_bytes).map(|_| rng.next() as u8).collect();
            t.write(packet, &junk);
            let gif = rng.next();
            t.global(l8v::DMA_GIF, gif);
            let sync = t.seq(&[0x1111, 0x2222]);
            t.stub_sym(l8v::SYNC_PATH, 1, false, 2, 0, sync);
            t.stub_sym(l8v::FLUSH_CACHE, 2, false, 1, 0, Ret::Val(0x33));
            t.stub_sym(l8v::DMA_SEND, 3, false, 2, 0, Ret::Val(0x44));
            let args = [work, obj, recs, offset, packet];
            let slots = t.arg_slots(&args);
            t.check("opus_map_del_draw", 0x00a7_5170, Conv::Cdecl, &args, |g| {
                Out::U32(l8v::opus_map_del_draw(
                    g, slots, work, obj, recs, offset, packet,
                ))
            });
            t.same_stack("opus_map_del_draw", slots, 20);
            assert_eq!(t.calls(), 4);
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn l8_opus_encount5_draw() {
    // paused or not; progress below, at and above 1; fades below, at and
    // past 0.3, NaN, and steps giving i below and from 16 on both arms;
    // the colour in the frame and the radial line's arguments
    Twins::each(|t| {
        let mut rng = Rng(0x007f_8e80);
        let n = l8v::SPRITES;
        let rec_bytes = 0x80 + 0x60 * n;
        let pkt_bytes = 0x80 + 0x60 * n;
        let (work, obj, recs, offset, packet) = (
            t.zeros(0x220),
            t.zeros(0x100),
            t.zeros(rec_bytes as usize),
            t.zeros(8),
            t.zeros(pkt_bytes as usize),
        );
        let third = 0x3e99_999a_u32;
        for round in 0..6 {
            let paused = if round % 3 == 2 {
                0x8000 | rng.below(0x100) as u16
            } else {
                0
            };
            t.write(work + 0x212, &paused.to_le_bytes());
            for o in [0x48, 0x4c] {
                let v = if rng.chance(3) {
                    float_word(&mut rng)
                } else {
                    (rng.below(40000) as f32 - 20000.0).to_bits()
                };
                t.w32(obj + o, v);
            }
            for o in 0xc2..=0xcb {
                t.w8(obj + o, rng.next() as u8);
            }
            for o in [0, 4] {
                let v = if rng.chance(3) {
                    float_word(&mut rng)
                } else {
                    (rng.below(1000) as f32 * 0.37).to_bits()
                };
                t.w32(offset + o, v);
            }
            let mut bytes = Vec::with_capacity(rec_bytes as usize);
            for k in 0..rec_bytes / 4 {
                let o = k * 4;
                let rel = (o + 0x60 - 0x68 % 0x60) % 0x60;
                let v = match rel {
                    // P+0, the lerp: near 1, anything
                    0 => match rng.below(3) {
                        0 => (0.9 + rng.below(20) as f32 * 0.01).to_bits(),
                        1 => 1.0_f32.to_bits(),
                        _ => float_word(&mut rng),
                    },
                    // P+8, the fade: round 0.3, exactly it, anything
                    8 => match rng.below(4) {
                        0 => third,
                        1 => (0.2 + rng.below(40) as f32 * 0.01).to_bits(),
                        2 => (rng.below(30) as f32 * 0.1).to_bits(),
                        _ => float_word(&mut rng),
                    },
                    // P+0xc, the fade's step: small, anything
                    0xc => match rng.below(3) {
                        0 => (0.01 + rng.below(10) as f32 * 0.02).to_bits(),
                        1 => (-0.05_f32).to_bits(),
                        _ => float_word(&mut rng),
                    },
                    4 => (rng.below(10) as f32 * 0.02).to_bits(),
                    _ => float_word(&mut rng),
                };
                bytes.extend(v.to_le_bytes());
            }
            t.write(recs, &bytes);
            let junk: Vec<u8> = (0..pkt_bytes).map(|_| rng.next() as u8).collect();
            t.write(packet, &junk);
            let gif = rng.next();
            t.global(l8v::DMA_GIF, gif);
            let sync = t.seq(&[0x1111, 0x2222]);
            t.stub_sym(l8v::SYNC_PATH, 1, false, 2, 0, sync);
            t.stub_sym(l8v::FLUSH_CACHE, 2, false, 1, 0, Ret::Val(0x33));
            t.stub_sym(l8v::DMA_SEND, 3, false, 2, 0, Ret::Val(0x44));
            t.stub_sym(l8v::RADIAL_LINE, 4, false, 4, 0, Ret::Val(0x55));
            let args = [work, obj, recs, offset, packet];
            let ebp = frame_ebp(&t.a, 5);
            t.b_top = Some(ebp - 0x100);
            t.check("opus_encount5_draw", 0x007f_8e80, Conv::Cdecl, &args, |g| {
                Out::U32(l8v::opus_encount5_draw(
                    g, ebp, work, obj, recs, offset, packet,
                ))
            });
            t.same_stack("opus_encount5_draw", ebp - l8v::ENCOUNT5_COLOUR, 0x10);
            assert_eq!(t.calls(), 5);
            t.b_top = None;
            t.reset();
        }
    });
}

/// After the log: copies the `n` bytes at `src` to the first argument
/// (`rep movsb`), as a `sprintf` that prints them.
fn copy_to_first_code(src: u32, n: u32) -> Vec<u8> {
    let mut c = vec![0x56, 0x57, 0x8b, 0x7c, 0x24, 0x0c, 0xbe]; // push esi; push edi; mov edi, [esp+0xc]; mov esi,
    c.extend(src.to_le_bytes());
    c.push(0xb9); // mov ecx, n
    c.extend(n.to_le_bytes());
    c.extend([0xf3, 0xa4, 0x5f, 0x5e]); // rep movsb; pop edi; pop esi
    c
}

#[test]
fn l8_opd_num_put_float() {
    // signs, widths below, at and past the integer digits (and 0 and
    // negative), decimals 0..8 read past the string, characters past
    // 0x7f, no padding (the never-written link word, zeroed frames), the
    // print position and the decimals' argument slot
    Twins::each(|t| {
        let mut rng = Rng(0x00a5_ea30);
        let texts: [&[u8]; 8] = [
            b"-123.456789",
            b"0.000000",
            b"12345678.500000",
            b"1.#INF00",
            b"-0.250000",
            b"7.\x80\xff\x41\x7f12",
            b"99999999999.0",
            b"-.5",
        ];
        for case in 0..96 {
            let text = texts[case % texts.len()];
            let mut s = text.to_vec();
            s.push(0);
            let src = t.alloc(&s);
            let buffer = t.zeros(0x1000);
            let junk: Vec<u8> = (0..0x1000).map(|_| rng.next() as u8).collect();
            t.write(buffer, &junk);
            let pr = t.zeros(0x20);
            t.w32(pr + 8, buffer);
            for o in [0x10, 0x12] {
                t.write(pr + o, &(rng.next() as u16).to_le_bytes());
            }
            let font = t.zeros(0x20);
            let fb: Vec<u8> = (0..0x20).map(|_| rng.next() as u8).collect();
            t.write(font, &fb);
            let width = match rng.below(5) {
                0 => 0,
                1 => (rng.below(4) as i32 - 4).cast_unsigned(),
                _ => rng.below(13),
            };
            let decimals = match rng.below(4) {
                0 => (-1_i32).cast_unsigned(),
                _ => rng.below(9),
            };
            t.extra = copy_to_first_code(src, s.len() as u32);
            t.import_sym(l8v::SPRINTF_SLOT, 1, 4, 0, Ret::Val(7));
            t.stub_sym(l8v::NUM_TEX_TAG, 2, false, 3, 0, Ret::Val(0x40));
            // the glyph size bytes (0 until opd_num_init runs)
            let size = [rng.next() as u8, rng.next() as u8];
            t.patch(t.rebase(l8v::GLYPH_W), &size);
            let value = [rng.next(), rng.next()];
            let args = [value[0], value[1], width, decimals, pr, font];
            t.zero_frames();
            let slots = t.arg_slots(&args);
            let ebp = frame_ebp(&t.a, 6);
            t.b_top = Some(ebp - 0x260);
            t.check(
                "opd_num_put_float_00a5ea30",
                0x00a5_ea30,
                Conv::Cdecl,
                &args,
                |g| {
                    Out::U32(l8v::put_float(
                        g, ebp, slots, value, width, decimals, pr, font,
                    ))
                },
            );
            t.same_stack("opd_num_put_float_00a5ea30", slots, 24);
            assert_eq!(t.calls(), 2);
            t.b_top = None;
            t.reset();
        }
    });
}

/// After the log: copies the call's argument words to the buffer whose
/// cursor is at `cursor` (advanced): 18 words when the first is `head`,
/// else 17 (`opd_num`'s two line formats).
fn copy_args_code(cursor: u32, head: u32) -> Vec<u8> {
    let mut c = vec![0x56, 0x57, 0x51, 0x8b, 0x3d]; // push esi; push edi; push ecx; mov edi, [cursor]
    c.extend(cursor.to_le_bytes());
    c.extend([0x8d, 0x74, 0x24, 0x10]); // lea esi, [esp+0x10]
    c.push(0xb9);
    c.extend(0x44_u32.to_le_bytes()); // mov ecx, 17 words
    c.extend([0x81, 0x3e]);
    c.extend(head.to_le_bytes()); // cmp dword [esi], head
    c.extend([0x75, 0x05, 0xb9]);
    c.extend(0x48_u32.to_le_bytes()); // jne +5; mov ecx, 18 words
    c.extend([0xf3, 0xa4, 0x89, 0x3d]); // rep movsb; mov [cursor], edi
    c.extend(cursor.to_le_bytes());
    c.extend([0x59, 0x5f, 0x5e]); // pop ecx; pop edi; pop esi
    c
}

#[test]
fn l8_opd_num_char_256() {
    // bytes past 0x7f widen negative; every line's words recorded
    Twins::each(|t| {
        let mut rng = Rng(0x00a5_d5f0);
        for _ in 0..8 {
            let p = t.zeros(0x100);
            let bytes: Vec<u8> = (0..0x100).map(|_| rng.next() as u8).collect();
            t.write(p, &bytes);
            let out = t.zeros(16 * 0x48);
            let cursor = t.words(&[out]);
            let head = t.rebase(l8v::DUMP_HEAD);
            t.extra = copy_args_code(cursor, head);
            let seq = t.seq(&(0..16).map(|k| 0x100 + k).collect::<Vec<_>>());
            t.stub_sym(l8v::OPD_NUM, 1, false, 2, 0, seq);
            t.check("opd_num_char_256", 0x00a5_d5f0, Conv::Cdecl, &[p], |g| {
                Out::U32(l8v::char_256(g, p))
            });
            assert_eq!(t.calls(), 16);
            assert_eq!(t.r32(cursor), out + 4 * 16 * 17 + 4 * 4);
            t.reset();
        }
    });
}

/// A float word for the light tests: mostly small numbers of either
/// sign, sometimes anything but a signalling NaN.
fn light_word(rng: &mut Rng) -> u32 {
    if rng.chance(12) {
        float_word(rng)
    } else {
        ((rng.below(2001) as f32 - 1000.0) * 0.013).to_bits()
    }
}

#[test]
fn l8_ppp_mdl_point_light_colours() {
    // the flag clear, n 0 or negative, 1..5 vertices; lights near and far,
    // matrices with zero rows (1 / sqrt(0)), negative terms clamped
    // (twice), NaNs passing the clamps; the frame (normalised matrix,
    // light directions, their transpose, the light matrix, the vertex,
    // normal and scale ApplyMatrix sees) and the count's argument slot
    Twins::each(|t| {
        let mut rng = Rng(0x0071_e820);
        for case in 0..40_u32 {
            let seg = rng.below(3);
            let n = match case % 8 {
                0 => 0,
                1 => (-2_i32).cast_unsigned(),
                _ => 1 + rng.below(5),
            };
            let cnt = n.cast_signed().max(0) as usize;
            let arr = |t: &mut Twins, rng: &mut Rng, words: usize, w: Option<u32>| {
                let v: Vec<u32> = (0..words)
                    .map(|k| match w {
                        Some(x) if k % 4 == 3 => x,
                        _ => light_word(rng),
                    })
                    .collect();
                t.words(&v)
            };
            let verts = arr(t, &mut rng, 3 * cnt + 1, None);
            let normals = arr(t, &mut rng, 3 * cnt + 1, None);
            let colours = arr(t, &mut rng, 4 * cnt + 1, None);
            let table = t.zeros(0x200);
            let base = table + 0x60;
            let entry = base + 0x6c * seg;
            t.w32(entry - 0x60, verts);
            t.w32(entry - 0x5c, normals);
            t.w32(entry - 0x58, colours);
            let rd = t.zeros(0x100);
            t.w32(rd + 0x94, base);
            let matrix = arr(t, &mut rng, 16, None);
            if rng.chance(4) {
                let r = rng.below(3);
                for c in 0..3 {
                    t.w32(matrix + 0x10 * r + 4 * c, 0);
                }
            }
            let lights = arr(t, &mut rng, 16, None);
            let lmatrix = arr(t, &mut rng, 16, None);
            let job = t.zeros(0x60);
            let flags = if case % 8 == 7 {
                0x8f
            } else {
                0x40 | rng.below(0x100)
            };
            t.w8(job, flags as u8);
            t.w32(job + 0x24, lmatrix);
            t.w32(job + 0x28, lights);
            t.w32(job + 0x2c, matrix);
            t.w8(job + 0x4d, rng.next() as u8);
            // _CIsqrt (an import the oracle lacks) as the CPU's: fsqrt; ret
            t.patch(t.rebase(l8v::CI_SQRT), &[0xd9, 0xfa, 0xc3]);
            let args = [rd, seg, job, rng.next(), n];
            t.zero_frames();
            let slots = t.arg_slots(&args);
            let ebp = frame_ebp(&t.a, 5);
            t.b_top = Some(ebp - 0x200);
            t.check(
                "pppMdl_pointLightColours_0071e820",
                0x0071_e820,
                Conv::Cdecl,
                &args,
                |g| Out::U32(l8v::point_light_colours(g, ebp, slots, rd, seg, job, n)),
            );
            t.same_stack(
                "pppMdl_pointLightColours_0071e820 frame",
                ebp - 0x144,
                0x140,
            );
            t.same_stack("pppMdl_pointLightColours_0071e820 slots", slots, 20);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 9 (tail9) (ffx_glue_other::l9_vfx) ----

use ffx_glue_other::l9_vfx::bridge as l9x;

#[test]
fn l9_opus_colour() {
    // every nibble (0..0xf000) on random command data (floats around 0..1,
    // negative, past 1, NaN, out of ftol's range), the draw mask and the
    // g_Ee_Gpr / VU0 words; the CLUT green vectors at the original's
    // 16-aligned frame
    Twins::each(|t| {
        let mut rng = Rng(0x0080_9f90);
        for case in 0..64_u32 {
            let host = t.zeros(0x240);
            let data = t.zeros(0x60);
            let nibble = (case % 16) << 12;
            let mode = (nibble | rng.below(0x1000)) as u16;
            t.write(host + 0x21a, &mode.to_le_bytes());
            t.w32(host + 0x220, data);
            for o in (0..0x10).step_by(4) {
                let v = float_word(&mut rng);
                t.w32(host + o, v);
            }
            for o in (0x30..0x50).step_by(4) {
                let v = match rng.below(5) {
                    0 => (rng.below(300) as f32 - 20.0).to_bits(),
                    1 => (rng.below(1000) as f32 / 700.0).to_bits(),
                    // -0.0 stays -0.0 (the clamp stores only below 0)
                    2 => (-0.0_f32).to_bits(),
                    _ => float_word(&mut rng),
                };
                t.w32(data + o, v);
            }
            for i in 0..4 {
                let v = float_word(&mut rng);
                t.global(l9x::EE_GPR + 4 * i, v);
            }
            let mask = rng.next();
            for (sym, tag, n) in [
                (l9x::RENDER_DISABLE, 1, 1),
                (l9x::RENDER_ENABLE, 2, 1),
                (l9x::SET_COLOR_SCALE, 3, 1),
                (l9x::BACK_COLOR_RESTORE, 4, 1),
                (l9x::COLOUR_SCALE2, 5, 4),
                (l9x::RGBA_FREE, 6, 0),
                (l9x::BACK_COLOR_SET, 7, 1),
                (l9x::FIELD_VEC4I, 8, 4),
                (l9x::SET_DRAW_MASK, 10, 1),
                (l9x::SET_RENDER_PARAM, 11, 1),
                (l9x::FIELD_VEC6, 12, 6),
                (l9x::STORE_COMPUTED, 13, 1),
                (l9x::COLOUR_PAIR, 14, 7),
                (l9x::MATERIAL_SCALE2, 15, 4),
                (l9x::CLUT_GREEN, 16, 2),
                (l9x::CLEAR_BIT8, 17, 0),
                (l9x::UPDATE_YIQ, 18, 0),
            ] {
                t.stub_sym(sym, tag, false, n, 0, Ret::Val(0x100 + tag));
            }
            t.stub_sym(l9x::GET_DRAW_MASK, 9, false, 0, 0, Ret::Val(mask));
            let cmd = rng.next();
            // the original's 16-aligned ebp (two argument words, the return
            // address and ebx pushed, esp - 8 rounded down to 16; push
            // ebp) and its vectors
            let ebp = (call_top(&t.a) - 8 - 16) & !0xf;
            let vectors = ebp - 0x30;
            t.zero_frames();
            t.b_top = Some(vectors - 0x40);
            t.check(
                &format!("0x00809f90 #{case} {mode:#x}"),
                0x0080_9f90,
                Conv::Cdecl,
                &[host, cmd],
                |g| Out::U32(l9x::opus_colour(g, vectors, host, cmd)),
            );
            t.same_stack("0x00809f90 vectors", vectors, l9x::VECTORS as usize);
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- round 10 (tail10) (ffx_glue_other::l10_vfx) ----

use ffx_glue_other::l10_vfx::{self as l10v, bridge as l10vb};

#[test]
#[allow(clippy::too_many_lines)] // the state, the stubs and every mode
fn l10_opd_heap_display() {
    // every corner and ramp half (mode & 3, & 0x800), the special modes
    // (0x601, 0x800..0x802, 0x8603) and others, the numbers on and off;
    // size 0 (no level), the level rising, falling, NaN; the lit quads from
    // none to all 48; the state's floats random (no signalling NaNs). The
    // transform stubs write four words through their output; op_num's
    // calls are logged with their words. Synthetic only.
    Twins::each(|t| {
        let mut rng = Rng(0x00a6_3ff0);
        let mut seen = [0u32; 4];
        for case in 0..96_u32 {
            t.reset();
            let st = t.rebase(l10v::STATE);
            for off in (0..0x510).step_by(4) {
                let v = float_word(&mut rng);
                t.w32(st + off, v);
            }
            let q = case % 4;
            let slot = st + 0x410 + 0x40 * q;
            // the level and peak (0..1, NaN now and then)
            for o in [0x10, 0x14] {
                let v = match rng.below(6) {
                    0 => 0x7fc0_0000,
                    _ => (rng.below(1100) as f32 / 1000.0).to_bits(),
                };
                t.w32(slot + o, v);
            }
            let ramps = t.rebase(l10v::RAMPS);
            for o in (0..0x100).step_by(4) {
                let v = rng.next() & 0x0f0f_0f0f;
                t.w32(ramps + o, v);
            }
            let closing = t.rebase(l10v::CLOSING);
            for o in (0..0x20).step_by(4) {
                let v = rng.next();
                t.w32(closing + o, v);
            }
            let wide = t.rebase(l10v::WIDE);
            t.w8(wide, u8::from(rng.chance(2)));
            let show = t.rebase(l10v::SHOW_NUMBERS);
            t.w8(show, u8::from(!rng.chance(4)));
            let mode = match case / 4 % 8 {
                0 => 0x601,
                1 => 0x800,
                2 => 0x801,
                3 => 0x802,
                4 => 0x8603,
                5 => q | 0x800,
                6 => q | (rng.next() & 0x7ff_fffc) | 0x1000,
                _ => q,
            };
            let size = match rng.below(5) {
                0 => 0,
                1 => rng.next(),
                _ => 1 + rng.below(0x10000),
            };
            let mut used = if rng.chance(3) {
                rng.next()
            } else {
                rng.below(size.max(1) * 2)
            };
            let mut size = size;
            if case % 7 == 2 {
                // the level exactly at the ratio (and the peak at it)
                size = 2 * (1 + rng.below(0x1000));
                used = size / 2;
                t.w32(slot + 0x10, 0.5f32.to_bits());
                t.w32(slot + 0x14, 0.5f32.to_bits());
            }
            for (sym, tag, n) in [
                (l10v::CAMERA_MATRIX, 1, 4),
                (l10v::VIEW_SCREEN_MATRIX, 2, 10),
                (l10v::UNIT_M_SCALE, 3, 2),
                (l10v::ROT_M, 4, 3),
                (l10v::APPLY_M_W1, 5, 3),
                (l10v::MUL_M, 6, 3),
                (l10v::GS_SYNC_PATH, 8, 2),
                (l10v::FLUSH_CACHE, 9, 1),
                (l10v::DMA_SEND, 10, 2),
                (l10v::NUM_XY_SET, 11, 2),
            ] {
                t.stub_sym(sym, tag, false, n, 0, Ret::Val(0x100 + tag));
            }
            // op_num logs its format; its value, when the format takes one
            // ("fprim" and "fpar" take none), goes to a list of its own
            let values = t.zeros(0x40);
            let cursor = t.words(&[values]);
            let mut x = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
            for (f, skip) in [(0x00b7_bf60_u32, 0x1a_u8), (0x00b7_bf54, 0x13)] {
                x.push(0x3d);
                x.extend(t.rebase(f).to_le_bytes()); // cmp eax, f
                x.extend([0x74, skip]); // je end
            }
            x.extend([0x8b, 0x44, 0x24, 0x08]); // mov eax, [esp+8]
            x.extend([0x8b, 0x0d]);
            x.extend(cursor.to_le_bytes()); // mov ecx, [cursor]
            x.extend([0x89, 0x01]); // mov [ecx], eax
            x.extend([0x83, 0x05]);
            x.extend(cursor.to_le_bytes());
            x.push(0x04); // add dword [cursor], 4
            t.stub_sym_x(l10v::NUM, 12, false, 1, 0, Ret::Val(0x10c), &x);
            // RotTransPers writes four words through its output
            let mut x = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
            for (i, v) in [0x1111_u32, 0x2222, 0x3333, 0x4444].into_iter().enumerate() {
                x.extend([0xc7, 0x40, (4 * i) as u8]);
                x.extend(v.to_le_bytes()); // mov dword [eax+4i], v
            }
            t.stub_sym_x(l10v::ROT_TRANS_PERS, 7, false, 4, 0, Ret::Val(0x107), &x);
            t.check(
                &format!("opd_heap_display #{case} mode {mode:#x}"),
                0x00a6_3ff0,
                Conv::Cdecl,
                &[used, size, mode],
                |g| Out::U32(l10vb::heap_display(g, used, size, mode)),
            );
            // how many quads were lit
            let lit = (0..48)
                .filter(|i| t.r32(t.rebase(l10v::QUADS) + 0x90 * i) == 0x48)
                .count();
            seen[match lit {
                0 => 0,
                48 => 1,
                _ => 2,
            }] += 1;
            seen[3] += u32::from(size == 0);
        }
        assert!(seen.iter().all(|&n| n > 0), "{seen:?}");
    });
}
