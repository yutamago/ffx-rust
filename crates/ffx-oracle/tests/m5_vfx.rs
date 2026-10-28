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
