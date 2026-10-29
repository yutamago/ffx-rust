//! `ffx_prender::m5r_*` (M5 stage 1, list `render`) against the original:
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
//! original pops.
//!
//! Throws: ATL's error paths call `_CxxThrowException` (or
//! `ThrowStdException_0064fa50`), which never returns. Their stubs
//! "unwind": they log, restore the callee-saved registers the oracle
//! checks and return straight to the test's call with the stack the call
//! expects, as a `catch` at the top would. The port's call to the thrower
//! unwinds the same way to the bridge, which then returns.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::items_after_statements
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_prender::m5r_a::{self as ma, bridge as a};
use ffx_prender::m5r_b::{self as mb, bridge as b};
use ffx_prender::m5r_c::{self as mc, bridge as c};
use ffx_prender::m5r_d::{self as md, bridge as d};
use ffx_prender::m5r_e::{self as me, bridge as e};
use ffx_prender::m5r_f::{self as mf, bridge as f};
use ffx_prender::m5r_g::{self as mg, bridge as gb};
use ffx_prender::m5r_h::{self as mh, bridge as h};
use ffx_prender::m5r_i::{self as mi, bridge as i};
use ffx_prender::m5r_j::{self as mj, bridge as j};
use ffx_prender::m5r_k::{self as mk, bridge as k};
use ffx_prender::m5r_l::{self as ml, bridge as lb};

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
    /// `ST0`, a float the original loads with `fld dword`.
    F32(f32),
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
        if let Out::F32(v) = out {
            let st0 = ra.st0.unwrap_or_else(|| panic!("{what}: no float in ST0"));
            assert!(
                ffx_oracle::same_float(st0, f64::from(v)),
                "{what}: st0 {st0} vs {v}"
            );
        } else {
            assert!(
                ra.st0_raw.is_none(),
                "{what}: the original leaves ST0 set ({:?})",
                ra.st0
            );
        }
        match out {
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::U8(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
            Out::None | Out::F32(_) => {}
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

// ---- forwarders ----

#[test]
fn forwarders_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0001);
        for case in 0..6 {
            let scene = t.zeros(0x1_0e00);
            for off in [0x10cc_u32, 0x1_0dd0, 0x1_0dd4] {
                let v = rng.word();
                t.w32(scene + off, v);
            }
            if !t.snapshot || case % 2 == 0 {
                t.global(ma::SCENE.sym, scene);
            }
            t.stub_sym(ma::SCENE_ANIMATE, 1, true, 0, 0, Ret::Val(0x1234));
            t.check(
                "graphicSceneAnimate_00639300",
                0x0063_9300,
                Conv::Cdecl,
                &[],
                |g| Out::U32(a::scene_animate(g)),
            );
            t.stub_sym(ma::POST_RESIZE_TARGET, 2, true, 2, 2, Ret::Val(case));
            t.check(
                "PhyreScene_resizePostTarget_0066c670",
                0x0066_c670,
                Conv::Thiscall(scene),
                &[],
                |g| Out::U32(a::scene_resize_post_target(g, scene)),
            );
            t.stub_sym(ma::SCENE_RESIZE_POST_TARGET, 3, true, 0, 0, Ret::Val(3));
            t.check(
                "graphicResizePostTarget_00642ce0",
                0x0064_2ce0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(a::resize_post_target(g)),
            );
            t.stub_sym(ma::SCENE_CLEAR_SPECIAL, 4, true, 0, 0, Ret::Val(7));
            t.check(
                "graphicClearSpecialRenderList",
                0x0064_4dd0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(a::graphic_clear_special_render_list(g)),
            );
            t.stub_sym(
                ma::DELEGATIONS_REMOVE_FLAG_1000,
                5,
                false,
                0,
                0,
                Ret::Val(9),
            );
            t.check(
                "graphicRemoveDelegations_0063dab0",
                0x0063_dab0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(a::remove_delegations(g)),
            );
            let this = t.zeros(0x20);
            t.stub_sym(ma::TEXT_UTIL_TERMINATE, 6, true, 0, 0, Ret::Val(case + 1));
            t.check(
                "PostProcessTweakUtil_terminate",
                0x0069_8f60,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(a::tweak_terminate(g, this)),
            );
            assert_eq!(t.calls(), 6);
            t.reset();
        }
    });
}

#[test]
fn post_resize_target_matches_the_original() {
    // Every branch on synthetic data: no flag, no target, the width or the
    // height differing, both equal. A target of another kind (byte +4 set)
    // reads its size at address 0x1c in the original: left out.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0002);
        for case in 0..200 {
            let this = t.zeros(0x1_2400);
            for i in 0..4 {
                if rng.chance(3) {
                    let v = rng.next() as u8 | 1;
                    t.w8(this + 0x1_23b0 + i, v);
                }
            }
            let (w, h) = (rng.below(3) * 640, rng.below(3) * 360);
            if !rng.chance(5) {
                let tex = t.zeros(0x40);
                t.w32(tex + 0x1c, if rng.chance(2) { w } else { rng.word() });
                t.w32(tex + 0x20, if rng.chance(2) { h } else { rng.word() });
                let target = t.words(&[tex, 0]);
                t.w32(this + 0xac, target);
            }
            t.stub_sym(ma::RENDER_TARGET_RESIZE, 1, false, 4, 0, Ret::Val(case));
            t.check(
                "PostProcessManager_resizeTarget_006980a0",
                0x0069_80a0,
                Conv::Thiscall(this),
                &[w, h],
                |g| Out::U32(a::post_resize_target(g, this, w, h)),
            );
            t.reset();
        }
        if t.snapshot {
            // the game's post-process manager, at its own size and another
            let scene: u32 = ma::SCENE.get(&t.b);
            if scene != 0 {
                let post = t.r32(scene + 0x10cc);
                let (w, h) = (t.r32(scene + 0x1_0dd0), t.r32(scene + 0x1_0dd4));
                for (w, h) in [(w, h), (w + 2, h), (w, h + 2)] {
                    t.stub_sym(ma::RENDER_TARGET_RESIZE, 1, false, 4, 0, Ret::Val(0));
                    t.check(
                        "PostProcessManager_resizeTarget_006980a0 (game)",
                        0x0069_80a0,
                        Conv::Thiscall(post),
                        &[w, h],
                        |g| Out::U32(a::post_resize_target(g, post, w, h)),
                    );
                    t.reset();
                }
            }
        }
    });
}

// ---- delegations ----

/// A delegation helper: `n` delegations (some flagged `0x1000`, some with
/// data), and running tasks in the 64 pairs at `+8` for some of them.
fn build_helper(t: &mut Twins, rng: &mut Rng, n: u32) -> (u32, Vec<u32>) {
    let helper = t.zeros(0x210);
    let mut ds = vec![];
    for _ in 0..n {
        let d = t.zeros(0x10);
        if rng.chance(2) {
            let inner = t.zeros(4);
            t.w32(d + 4, inner);
        }
        let flags = if rng.chance(2) { 0x1000 } else { 0 } | (rng.next() & 0xefff);
        t.w32(d + 0xc, flags);
        let busy = rng.next() | 1;
        t.w32(d + 8, busy);
        ds.push(if rng.chance(8) { 0 } else { d });
    }
    let data = t.words(&ds);
    let top = if rng.chance(4) { 0x8000_0000 } else { 0 };
    t.w32(helper, n | top);
    t.w32(helper + 4, data);
    for (k, &d) in ds.iter().enumerate() {
        if d != 0 && rng.chance(3) {
            let j = rng.below(0x40);
            t.w32(helper + 8 + 8 * j, d);
            t.w32(helper + 0xc + 8 * j, rng.below(3));
        }
        let _ = k;
    }
    (helper, ds)
}

fn stub_mutex(t: &mut Twins) {
    t.import_sym(ma::WAIT_FOR_SINGLE_OBJECT, 0x10, 2, 2, Ret::Val(0));
    t.import_sym(ma::RELEASE_MUTEX, 0x11, 1, 1, Ret::Val(0x4d));
}

#[test]
fn delegations_remove_matches_the_original() {
    // found or not; running (busy word 1 or not: the warning) or not;
    // with and without data.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0003);
        for _ in 0..150 {
            let n = rng.below(6);
            let (helper, ds) = build_helper(t, &mut rng, n);
            let delegation = if !ds.is_empty() && !rng.chance(4) {
                ds[rng.below(ds.len() as u32) as usize]
            } else {
                rng.word()
            };
            stub_mutex(t);
            let cas = rng.below(3);
            t.import_sym(ma::INTERLOCKED_COMPARE_EXCHANGE, 0x12, 3, 3, Ret::Val(cas));
            let iob = t.zeros(0x80);
            t.import_sym(ma::IOB_FUNC, 0x13, 0, 0, Ret::Val(iob));
            t.import_sym(ma::FPRINTF, 0x14, 3, 0, Ret::Val(5));
            t.stub_sym(ma::OPERATOR_DELETE, 0x15, false, 1, 0, Ret::Keep);
            t.stub_sym(ma::DELEGATIONS_COMPACT, 0x16, true, 0, 0, Ret::Val(1));
            t.check(
                "RenderDelegationHelper_remove_006ae240",
                0x006a_e240,
                Conv::Thiscall(helper),
                &[delegation],
                |g| Out::U32(a::delegations_remove(g, helper, delegation)),
            );
            t.reset();
        }
    });
}

#[test]
fn delegations_remove_flag_1000_matches_the_original() {
    // The removal runs as the original (its imports and deletes stubbed),
    // so the array really shrinks under the loop. On the snapshot the
    // game's helper too (its delegations without flag 0x1000 stay).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0004);
        for case in 0..80 {
            if !(t.snapshot && case == 0) {
                let n = rng.below(7);
                let (helper, _) = build_helper(t, &mut rng, n);
                t.global(ma::DELEGATIONS.sym, helper);
            }
            stub_mutex(t);
            t.import_sym(ma::INTERLOCKED_COMPARE_EXCHANGE, 0x12, 3, 3, Ret::Val(1));
            t.stub_sym(ma::OPERATOR_DELETE, 0x15, false, 1, 0, Ret::Keep);
            t.check(
                "RenderDelegationHelper_removeFlag1000_006ae530",
                0x006a_e530,
                Conv::Cdecl,
                &[],
                |g| {
                    let helper = ma::DELEGATIONS.get(g);
                    let r = a::delegations_remove_flag_1000(g);
                    // an empty array leaves the caller's eax
                    if ma::DELEGATIONS.get(g) == helper
                        && g.read::<u32>(helper).trailing_zeros() >= 31
                    {
                        Out::None
                    } else {
                        Out::U32(r)
                    }
                },
            );
            t.reset();
        }
    });
}

// ---- the free camera ----

#[test]
fn free_camera_type_matches_the_original() {
    // made now (allocation or not) or there; bytes +0x338/+0x33c.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0005);
        for case in 0..40 {
            let cam = t.zeros(0x340);
            t.w32(cam, rng.word());
            for off in [0x338, 0x33c] {
                if rng.chance(3) {
                    let v = rng.next() as u8 | 1;
                    t.w8(cam + off, v);
                }
            }
            let made = case % 3;
            t.global(ma::FREE_CAMERA.sym, if made == 0 { cam } else { 0 });
            let block = if made == 1 { cam } else { 0 };
            t.stub_sym(ma::OPERATOR_NEW, 1, false, 1, 0, Ret::Val(block));
            t.stub_sym(ma::FREE_CAMERA_CTOR, 2, true, 0, 0, Ret::Val(cam));
            if made == 2 {
                // operator new fails: the camera is 0, read at 0x338
                let zero = t.zeros(0x1000);
                let _ = zero;
                t.reset();
                continue;
            }
            t.check(
                "graphicGetFreeCameraType",
                0x0064_11e0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(a::free_camera_type(g)),
            );
            t.reset();
        }
    });
}

// ---- arrays ----

#[test]
fn array_reserves_match_the_original() {
    // no data (any n); data with n > 0 (allocation or not), n <= 0.
    let list: [(u32, u32); 8] = [
        (0x0066_7e00, 4),
        (0x0066_7e80, 4),
        (0x0066_7f80, 4),
        (0x0066_8000, 4),
        (0x0066_8080, 4),
        (0x0066_8100, 4),
        (0x0066_8200, 20),
        (0x0066_8280, 16),
    ];
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0006);
        for &(sym, elem) in &list {
            for _ in 0..30 {
                let arr = t.zeros(12);
                t.w32(arr, rng.word());
                t.w32(arr + 4, rng.word());
                if !rng.chance(3) {
                    t.w32(arr + 8, 0x0abc_0000 | rng.below(0x1000));
                }
                let n = rng.word();
                let block = if rng.chance(4) { 0 } else { 0x0def_0000 };
                t.stub_sym(ma::USER_MALLOC, 1, false, 1, 0, Ret::Val(block));
                t.stub_sym(ma::MEMCPY, 2, false, 3, 0, Ret::Keep);
                t.stub_sym(ma::USER_FREE, 3, false, 1, 0, Ret::Val(0x77));
                t.check(
                    &format!("Array_Reserve_{sym:08x}"),
                    sym,
                    Conv::Thiscall(arr),
                    &[n],
                    |g| Out::U32(a::reserve(g, arr, n, elem)),
                );
                t.reset();
            }
        }
    });
}

#[test]
fn clear_special_render_list_matches_the_original() {
    // the reserve when both words are negative, or not; on the snapshot
    // the game's scene.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0007);
        for case in 0..30 {
            let scene = if t.snapshot && case == 0 {
                ma::SCENE.get(&t.b)
            } else {
                let s = t.zeros(0xfa00);
                for off in [0x3878_u32, 0x3854, 0x3860, 0x386c] {
                    let v = if rng.chance(2) {
                        0x8000_0000 | rng.next()
                    } else {
                        rng.word()
                    };
                    t.w32(s + off, v);
                    let v = if rng.chance(2) {
                        0x8000_0000 | rng.next()
                    } else {
                        rng.word()
                    };
                    t.w32(s + off + 4, v);
                }
                t.w32(s + 0x3884, rng.word());
                t.w32(s + 0x3888, rng.word());
                for k in 0..0x30 {
                    let v = rng.next();
                    t.w32(s + 0xf8c0 + 4 * k, v);
                }
                s
            };
            if scene == 0 {
                continue;
            }
            t.stub_sym(ma::MEMSET, 1, false, 3, 0, Ret::Keep);
            t.stub_sym(ma::RESERVE_4, 2, true, 1, 1, Ret::Val(0));
            t.check(
                "PhyreScene_clearSpecialRenderList_00654b70",
                0x0065_4b70,
                Conv::Thiscall(scene),
                &[],
                |g| Out::U32(a::clear_special_render_list(g, scene)),
            );
            t.reset();
        }
    });
}

// ---- ATL ----

#[test]
fn atl_throws_match_the_original() {
    // _CxxThrowException's arguments: the thrower's own argument slot and
    // the throw info; every errno of the jump table (0..0x50) and past it.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0008);
        for _ in 0..10 {
            let hr = rng.word();
            t.throw_sym(ma::CXX_THROW, 1, 2);
            let slot = first_slot(&t.b, 1);
            t.b_top = Some(slot - 0x40);
            t.check(
                "ThrowStdException_0064fa50",
                0x0064_fa50,
                Conv::Cdecl,
                &[hr],
                |g| {
                    a::throw(g, slot, hr);
                    Out::None
                },
            );
            // the exception object: the code stored back into the slot
            let (x, y): (u32, u32) = (Mem::read(&t.a, slot), Mem::read(&t.b, slot));
            assert_eq!(x, y, "ThrowStdException_0064fa50: the exception object");
            t.b_top = None;
            t.reset();
        }
        for errno in (0..0x58).chain([0xffff_ffff, 0x100, 0x8000_0000]) {
            t.throw_sym(ma::THROW, 2, 1);
            t.check(
                "AtlCrtErrorCheck_0064f8d0",
                0x0064_f8d0,
                Conv::Cdecl,
                &[errno],
                |g| {
                    let r = a::crt_error_check(g, errno);
                    if errno == 0 || errno == 80 {
                        Out::U32(r)
                    } else {
                        Out::None
                    }
                },
            );
            t.reset();
        }
        t.throw_sym(ma::THROW, 3, 1);
        t.check(
            "AtlThrowOutOfMemory_00654950",
            0x0065_4950,
            Conv::Cdecl,
            &[],
            |g| {
                a::throw_out_of_memory(g);
                Out::None
            },
        );
        t.reset();
    });
}

#[test]
fn find_string_resource_instance_matches_the_original() {
    // instances until 0; FindResourceExW and the image failing or not.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0009);
        for _ in 0..60 {
            let k = rng.below(4) as usize;
            let inst: Vec<u32> = (0..k).map(|i| 0x0100_0000 + i as u32).chain([0]).collect();
            let found: Vec<u32> = (0..=k)
                .map(|_| if rng.chance(2) { 0x55 } else { 0 })
                .collect();
            let image: Vec<u32> = (0..=k)
                .map(|_| if rng.chance(2) { 0x66 } else { 0 })
                .collect();
            let r = t.seq(&inst);
            t.stub_sym(ma::GET_HINSTANCE_AT, 1, true, 1, 1, r);
            let r = t.seq(&found);
            t.import_sym(ma::FIND_RESOURCE_EX_W, 2, 4, 4, r);
            let r = t.seq(&image);
            t.stub_sym(ma::STRING_RESOURCE_IMAGE, 3, false, 3, 0, r);
            let (id, lang) = (rng.next() & 0x1_ffff, rng.next());
            t.check(
                "AtlFindStringResourceInstance_0064f970",
                0x0064_f970,
                Conv::Cdecl,
                &[id, lang],
                |g| Out::U32(a::find_string_resource_instance(g, id, lang)),
            );
            t.reset();
        }
    });
}

/// A `CStringW`: `[this]` = data, header at `data - 0x10` (manager,
/// length, allocated, references). Returns (this, header).
fn build_string(t: &mut Twins, manager: u32, length: u32, alloc: u32, refs: u32) -> (u32, u32) {
    let header = t.alloc(&[0xff; 0x10 + 0x80]);
    t.w32(header, manager);
    t.w32(header + 4, length);
    t.w32(header + 8, alloc);
    t.w32(header + 0xc, refs);
    let this = t.words(&[header + 0x10]);
    (this, header)
}

#[test]
fn string_fork_and_reallocate_match_the_original() {
    // Fork: allocation or none (the throw), length below/above n, last
    // reference or not. Reallocate: allocated below n or not, n <= 0, the
    // allocation failing.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_000a);
        for _ in 0..60 {
            let new = if rng.chance(4) { 0 } else { t.zeros(0x40) };
            let alloc_stub = t.stub_code(1, true, 2, 2, Ret::Val(new), false);
            let clone = t.object(&[alloc_stub], 4);
            let clone_stub = t.stub_code(2, true, 0, 0, Ret::Val(clone), false);
            let free_stub = t.stub_code(3, true, 1, 1, Ret::Val(0), false);
            let manager = t.object(&[0, free_stub, 0, 0, clone_stub], 4);
            let (length, n) = (rng.below(40), rng.word() % 64);
            let refs = rng.below(3);
            let (this, _) = build_string(t, manager, length, 50, refs);
            t.import_sym(ma::MEMCPY_S, 4, 4, 0, Ret::Val(0));
            t.throw_sym(ma::THROW_OUT_OF_MEMORY, 5, 0);
            t.check(
                "CSimpleString_Fork_00651460",
                0x0065_1460,
                Conv::Thiscall(this),
                &[n],
                |g| {
                    let r = a::string_fork(g, this, n);
                    if new == 0 { Out::None } else { Out::U32(r) }
                },
            );
            t.reset();
        }
        for _ in 0..60 {
            let new = if rng.chance(4) { 0 } else { t.zeros(0x40) };
            let re_stub = t.stub_code(1, true, 3, 3, Ret::Val(new), false);
            let manager = t.object(&[0, 0, re_stub], 4);
            let alloc = if rng.chance(5) {
                rng.word() | 0x8000_0000
            } else {
                rng.word() % 80
            };
            let n = match rng.below(6) {
                0 => rng.word(),
                1 => 0,
                _ => rng.below(100),
            };
            let (this, _) = build_string(t, manager, 3, alloc, 1);
            t.throw_sym(ma::THROW_OUT_OF_MEMORY, 5, 0);
            let throws = !((alloc as i32) < (n as i32) && (n as i32) > 0 && new != 0);
            t.check(
                "CSimpleString_Reallocate_006528d0",
                0x0065_28d0,
                Conv::Thiscall(this),
                &[n],
                |g| {
                    let r = a::string_reallocate(g, this, n);
                    if throws { Out::None } else { Out::U32(r) }
                },
            );
            t.reset();
        }
    });
}

#[test]
fn string_prepare_write2_matches_the_original() {
    // shared (fork), short (reallocate: half again, past 1 G plus 1 M, at
    // least n), long enough (the length in eax); n below the length.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_000b);
        for _ in 0..200 {
            let length = rng.word() % 0x100;
            let alloc = match rng.below(4) {
                0 => 0x4000_0000 + rng.below(3) - 1,
                1 => rng.word(),
                _ => rng.below(0x200),
            };
            let refs = rng.below(4);
            let n = match rng.below(3) {
                0 => rng.word(),
                1 => 0x4000_0000 + rng.below(0x20_0000),
                _ => rng.below(0x300),
            };
            let (this, _) = build_string(t, 0, length, alloc, refs);
            t.stub_sym(ma::STRING_FORK, 1, true, 1, 1, Ret::Val(0x11));
            t.stub_sym(ma::STRING_REALLOCATE, 2, true, 1, 1, Ret::Val(0x22));
            t.check(
                "CSimpleString_PrepareWrite2_00652600",
                0x0065_2600,
                Conv::Thiscall(this),
                &[n],
                |g| Out::U32(a::string_prepare_write2(g, this, n)),
            );
            t.reset();
        }
    });
}

#[test]
fn load_string_matches_the_original() {
    // no resource, no image; shared or short (PrepareWrite2) or not; the
    // copy's errno (checked); the length past the allocation (throws).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_000c);
        for _ in 0..120 {
            let len = rng.below(20);
            let mut img = vec![0_u8; 2 + 2 * 20];
            img[..2].copy_from_slice(&(len as u16).to_le_bytes());
            let image = if rng.chance(5) { 0 } else { t.alloc(&img) };
            let res = if rng.chance(6) { 0 } else { 0x77 };
            let alloc = rng.below(30);
            let refs = rng.below(3);
            let (this, _) = build_string(t, 0, rng.below(10), alloc, refs);
            t.import_sym(ma::FIND_RESOURCE_W, 1, 3, 3, Ret::Val(res));
            t.stub_sym(ma::STRING_RESOURCE_IMAGE, 2, false, 3, 0, Ret::Val(image));
            t.stub_sym(ma::STRING_PREPARE_WRITE2, 3, true, 1, 1, Ret::Val(0));
            let e = if rng.chance(3) { rng.below(40) } else { 0 };
            t.stub_sym(ma::WMEMCPY_S, 4, false, 4, 0, Ret::Val(e));
            t.stub_sym(ma::ATL_CRT_ERROR_CHECK, 5, false, 1, 0, Ret::Val(e));
            t.throw_sym(ma::THROW, 6, 1);
            let (inst, id) = (rng.next(), rng.next() & 0xffff);
            let throws = res != 0 && image != 0 && len > alloc;
            t.check(
                "CStringW_LoadString_00652310",
                0x0065_2310,
                Conv::Thiscall(this),
                &[inst, id],
                |g| {
                    let r = a::load_string(g, this, inst, id);
                    if throws { Out::None } else { Out::U32(r) }
                },
            );
            t.reset();
        }
    });
}

// ---- graphicFinalize's releases ----

#[test]
fn texanim_unregister_all_matches_the_original() {
    // characters and materials (tag bits), the material array owned or
    // borrowed (top bit), without data, empty.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_000d);
        for _ in 0..60 {
            let mgr = t.zeros(0x80);
            let nc = rng.below(4);
            let chars: Vec<u32> = (0..3 * nc).map(|_| rng.next()).collect();
            let cd = t.words(&chars);
            t.w32(mgr + 0x6c, nc | if rng.chance(4) { 0x8000_0000 } else { 0 });
            t.w32(mgr + 0x70, cd);
            let nm = rng.below(4);
            let mats: Vec<u32> = (0..8 * nm).map(|_| rng.next()).collect();
            let md = if nm == 0 && rng.chance(2) {
                0
            } else {
                t.words(&mats)
            };
            let top = if rng.chance(4) { 0x8000_0000 } else { 0 };
            t.w32(mgr + 0x64, nm | top);
            t.w32(mgr + 0x68, md);
            t.stub_sym(ma::TEXANIM_UNREGISTER_CHARACTER, 1, true, 1, 1, Ret::Val(0));
            t.stub_sym(ma::TEXANIM_UNREGISTER_BY_NAME, 2, true, 1, 1, Ret::Val(0));
            t.stub_sym(ma::TEXANIM_MATERIAL_DTOR, 3, true, 0, 0, Ret::Val(0));
            t.stub_sym(ma::PHYRE_FREE, 4, false, 1, 0, Ret::Val(0));
            t.check(
                "TextureAnimationManager_unregisterAll_006b1b60",
                0x006b_1b60,
                Conv::Thiscall(mgr),
                &[],
                |g| {
                    a::texanim_unregister_all(g, mgr);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

#[test]
fn character_lists_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_000e);
        for _ in 0..10 {
            let head = t.zeros(0x10);
            let table = t.words(&[head, rng.word()]);
            t.stub_sym(ma::LOADED_CHARACTERS_ERASE, 1, true, 1, 1, Ret::Val(0));
            t.check(
                "ClassLoadedCharacterTable_clear_006a11d0",
                0x006a_11d0,
                Conv::Thiscall(table),
                &[],
                |g| Out::U32(a::loaded_characters_clear(g, table)),
            );
            t.reset();
            let chr = t.words(&[0, 0, rng.next(), rng.next()]);
            stub_mutex(t);
            t.check(
                "ClassCharacter_clearInstanceList",
                0x006a_1200,
                Conv::Thiscall(chr),
                &[],
                |g| Out::U32(a::character_clear_instances(g, chr)),
            );
            t.reset();
        }
    });
}

/// A `std::map` tree of `n` nodes (value word `+0x28`) in a random shape:
/// the head (nil byte `+0xd` = 1; `+0` leftmost, `+4` root, `+8`
/// rightmost) and the nodes in order.
fn build_tree(t: &mut Twins, rng: &mut Rng, values: &[u32]) -> u32 {
    let head = t.zeros(0x30);
    t.w8(head + 0xd, 1);
    let nodes: Vec<u32> = values.iter().map(|_| t.zeros(0x30)).collect();
    for (&n, &v) in nodes.iter().zip(values) {
        t.w32(n + 0x28, v);
    }
    // a random binary tree over the in-order sequence
    fn place(t: &mut Twins, rng: &mut Rng, nodes: &[u32], parent: u32, head: u32) -> u32 {
        if nodes.is_empty() {
            return head;
        }
        let k = rng.below(nodes.len() as u32) as usize;
        let n = nodes[k];
        t.w32(n + 4, parent);
        let l = place(t, rng, &nodes[..k], n, head);
        let r = place(t, rng, &nodes[k + 1..], n, head);
        t.w32(n, l);
        t.w32(n + 8, r);
        n
    }
    let root = place(t, rng, &nodes, head, head);
    t.w32(head + 4, root);
    t.w32(head, *nodes.first().unwrap_or(&head));
    t.w32(head + 8, *nodes.last().unwrap_or(&head));
    head
}

#[test]
fn character_destroy_all_matches_the_original() {
    // characters with 0..3 instances, trees of 0..6 nodes in any shape
    // (every successor case).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_000f);
        for _ in 0..40 {
            let dtor = t.stub_code(1, true, 1, 1, Ret::Val(0), false);
            let k = rng.below(7) as usize;
            let mut chars = vec![];
            for _ in 0..k {
                let n = rng.below(4);
                let inst: Vec<u32> = (0..n).map(|_| rng.next()).collect();
                let v = if inst.is_empty() {
                    t.zeros(4)
                } else {
                    t.words(&inst)
                };
                let chr = t.object(&[dtor], 0x10);
                t.w32(chr + 8, v);
                t.w32(chr + 0xc, v + 4 * n);
                chars.push(chr);
            }
            let head = build_tree(t, &mut rng, &chars);
            let table = t.zeros(8);
            let head_at = t.words(&[head]);
            t.stub_sym(ma::LOADED_CHARACTERS, 2, false, 0, 0, Ret::Val(table));
            t.stub_sym(ma::LOADED_CHARACTER_TREE, 3, true, 0, 0, Ret::Val(head_at));
            t.stub_sym(ma::CHARACTER_INSTANCE_DESTROY, 4, false, 2, 0, Ret::Val(0));
            t.stub_sym(ma::CHARACTER_CLEAR_INSTANCES, 5, true, 0, 0, Ret::Val(0));
            t.stub_sym(ma::LOADED_CHARACTERS_CLEAR, 6, true, 0, 0, Ret::Val(0));
            t.stub_sym(ma::SCENE_RELEASE_CHARACTERS, 7, true, 0, 0, Ret::Val(0x99));
            t.check(
                "graphicCharacterDestroyAll_006378b0",
                0x0063_78b0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(a::character_destroy_all(g)),
            );
            assert!(t.calls() >= 4);
            t.reset();
        }
    });
}

#[test]
fn post_release_and_text_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0010);
        for _ in 0..4 {
            let this = t.zeros(0x1_2400);
            for k in 0..15 {
                let v = rng.next();
                t.w32(this + 4 * k, v);
            }
            let m = t.zeros(4);
            for (i, s) in [
                ma::POST_RELEASE_A,
                ma::POST_RELEASE_B,
                ma::RENDER_TARGET_RELEASE,
                ma::SAMPLER_RELEASE,
            ]
            .into_iter()
            .enumerate()
            {
                t.stub_sym(s, i as u32 + 1, true, 0, 0, Ret::Val(0));
            }
            t.stub_sym(ma::CLUSTER_MANAGER, 5, false, 0, 0, Ret::Val(m));
            t.stub_sym(ma::CLUSTER_RELEASE, 6, true, 1, 1, Ret::Val(0x31));
            t.check(
                "PostProcessManager_release_00698f10",
                0x0069_8f10,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(a::post_release(g, this)),
            );
            t.reset();
        }
        for _ in 0..80 {
            // TextUtilAsset_release: count 0, 1 (inline), 2+, top bit
            let this = t.zeros(0x20);
            let n = rng.below(4) | if rng.chance(3) { 0x8000_0000 } else { 0 };
            t.w32(this + 4, n);
            t.w32(this + 8, rng.word());
            t.stub_sym(ma::PHYRE_FREE, 1, false, 1, 0, Ret::Val(0));
            t.stub_sym(ma::SAMPLER_DTOR, 2, true, 0, 0, Ret::Val(0));
            t.check(
                "TextUtilAsset_release_006f5c40",
                0x006f_5c40,
                Conv::Thiscall(this),
                &[],
                |g| {
                    a::text_asset_release(g, this);
                    Out::None
                },
            );
            t.reset();
        }
        for _ in 0..40 {
            // TextUtilEntry_dtor: tag bit, owned words
            let this = t.zeros(0x80);
            t.w32(this + 0x7c, rng.word());
            t.w32(this + 0x74, rng.word());
            t.w32(this + 0x78, rng.word());
            t.stub_sym(ma::PHYRE_FREE, 1, false, 1, 0, Ret::Val(0));
            t.stub_sym(ma::RELEASE_TAGGED_WORDS, 2, false, 2, 0, Ret::Val(0));
            t.stub_sym(ma::STRING_BUILDER_DTOR, 3, true, 0, 0, Ret::Val(0x42));
            t.check(
                "TextUtilEntry_dtor_00692950",
                0x0069_2950,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(a::text_entry_dtor(g, this)),
            );
            t.reset();
        }
        for _ in 0..40 {
            // BootText: the buffer flag, the terminate flag and util
            let this = t.zeros(0x110);
            t.w32(this, rng.word());
            let v = rng.word();
            t.w32(this + 4, v);
            t.w32(this + 0x10c, rng.word());
            let v = rng.word() as u8;
            t.w8(this + 1, v);
            t.stub_sym(ma::PHYRE_FREE, 1, false, 1, 0, Ret::Val(0));
            t.check(
                "BootText_releaseBuffer_006ddfa0",
                0x006d_dfa0,
                Conv::Thiscall(this),
                &[],
                |g| {
                    a::boot_text_release_buffer(g, this);
                    Out::None
                },
            );
            t.reset();
            let this = t.zeros(0x10);
            t.w32(this, if rng.chance(4) { 0 } else { rng.next() | 1 });
            t.w32(this + 4, rng.word());
            t.stub_sym(ma::TEXT_UTIL_TERMINATE, 2, true, 0, 0, Ret::Val(0));
            t.stub_sym(ma::TEXT_ENTRY_DTOR, 3, true, 0, 0, Ret::Val(0));
            t.stub_sym(ma::OPERATOR_DELETE, 4, false, 1, 0, Ret::Val(0));
            t.check(
                "BootText_terminate_006ddfd0",
                0x006d_dfd0,
                Conv::Thiscall(this),
                &[],
                |g| {
                    a::boot_text_terminate(g, this);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

#[test]
fn morph_clear_matches_the_original() {
    // arrays with 0..4 elements, data or none (empty), owned or borrowed.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0011);
        for _ in 0..60 {
            let mm = t.zeros(0x20);
            for (off, elem) in [(0_u32, 8_u32), (8, 8), (0x10, 8), (0x18, 16)] {
                let n = rng.below(5);
                let top = if rng.chance(3) { 0x8000_0000 } else { 0 };
                let data = if n == 0 && rng.chance(2) {
                    0
                } else {
                    let w: Vec<u32> = (0..n * elem / 4).map(|_| rng.next()).collect();
                    t.words(&w)
                };
                t.w32(mm + off, n | top);
                t.w32(mm + off + 4, data);
            }
            t.stub_sym(ma::PHYRE_FREE, 1, false, 1, 0, Ret::Val(0));
            t.check(
                "MorphManager_clear_006acbd0",
                0x006a_cbd0,
                Conv::Thiscall(mm),
                &[],
                |g| {
                    a::morph_clear(g, mm);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

#[test]
fn scene_release_clusters_match_the_original() {
    // no cluster; held (any al byte) or not. On the snapshot the game's
    // scene words with the manager stubbed.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0012);
        for &(sym, off) in &ma::SCENE_CLUSTERS {
            for case in 0..8 {
                let scene = if t.snapshot && case == 0 {
                    ma::SCENE.get(&t.b)
                } else {
                    let s = t.zeros(0xfa98);
                    t.w32(s + off, if rng.chance(4) { 0 } else { rng.next() });
                    s
                };
                if scene == 0 {
                    continue;
                }
                let m = t.zeros(4);
                t.stub_sym(ma::CLUSTER_MANAGER, 1, false, 0, 0, Ret::Val(m));
                let held = match rng.below(3) {
                    0 => 0,
                    1 => 0x100,
                    _ => rng.next() | 1,
                };
                t.stub_sym(ma::CLUSTER_HELD, 2, true, 1, 1, Ret::Val(held));
                t.stub_sym(ma::CLUSTER_RELEASE, 3, true, 1, 1, Ret::Val(0));
                t.check(
                    &format!("PhyreScene_releaseCluster{off:x}_{sym:08x}"),
                    sym,
                    Conv::Thiscall(scene),
                    &[],
                    |g| Out::U32(a::scene_release_cluster(g, scene, off)),
                );
                t.reset();
            }
        }
    });
}

// ---- batch 2 (ffx_prender::m5r_b) ----

/// An object of `size` bytes of random words (a quarter of them 0).
fn rand_obj(t: &mut Twins, rng: &mut Rng, size: u32) -> u32 {
    let w: Vec<u32> = (0..size / 4)
        .map(|_| if rng.chance(4) { 0 } else { rng.next() })
        .collect();
    t.words(&w)
}

/// Stubs every callee in `list` (`sym`, logs `ecx`, `n` words, `pops`
/// popped as the original does), tags from `first`.
fn stub_all(t: &mut Twins, first: u32, list: &[(u32, bool, u32, u32)]) {
    stub_all_but(t, first, list, 0);
}

/// [`stub_all`] but for `under_test`.
fn stub_all_but(t: &mut Twins, first: u32, list: &[(u32, bool, u32, u32)], under_test: u32) {
    for (k, &(sym, this, n, pops)) in list.iter().enumerate() {
        if sym == under_test {
            continue;
        }
        t.stub_sym(
            sym,
            first + k as u32,
            this,
            n,
            pops,
            Ret::Val(0x100 + k as u32),
        );
    }
}

#[test]
fn finalize_singletons_match_the_original() {
    // each singleton set or not; the pointer fields set or 0.
    type P = fn(&mut G) -> Out;
    let cases: [(&str, u32, P); 9] = [
        (
            "PostProcessManager_destroyInstance_00692b90",
            0x0069_2b90,
            |g| {
                b::post_manager_destroy(g);
                Out::None
            },
        ),
        (
            "TextureAnimationManager_destroyInstance_006b0120",
            0x006b_0120,
            |g| {
                b::texanim_destroy(g);
                Out::None
            },
        ),
        ("Distortion_destroyInstance_006c2210", 0x006c_2210, |g| {
            b::distortion_destroy(g);
            Out::None
        }),
        ("RadialBlur_destroyInstance_006c3550", 0x006c_3550, |g| {
            b::radial_blur_destroy(g);
            Out::None
        }),
        ("BlurEx_destroyInstance_006c4a30", 0x006c_4a30, |g| {
            b::blur_ex_destroy(g);
            Out::None
        }),
        ("PostEffect_destroyInstance_006c50b0", 0x006c_50b0, |g| {
            b::post_effect_destroy(g);
            Out::None
        }),
        (
            "SpecularMapParamsManager_delete_006c57c0",
            0x006c_57c0,
            |g| Out::U32(b::specular_map_params_delete(g)),
        ),
        (
            "DynGeoMemManager_destroyInstance_006dec20",
            0x006d_ec20,
            |g| {
                b::dyn_geo_destroy(g);
                Out::None
            },
        ),
        ("graphicMenuUnloadExtraTexture", 0x0064_2380, |g| {
            Out::U32(b::menu_unload_extra_texture(g))
        }),
    ];
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0101);
        for case in 0..24 {
            let set = case % 3 != 0;
            stub_all(
                t,
                1,
                &[
                    (mb::POST_MANAGER_DTOR, true, 0, 0),
                    (mb::OPERATOR_DELETE, false, 1, 0),
                    (mb::SAMPLER_RELEASE, true, 0, 0),
                    (mb::SAMPLER_DTOR, true, 0, 0),
                    (mb::DISTORTION_TERMINATE, true, 0, 0),
                    (mb::RADIAL_BLUR_RELEASE, true, 0, 0),
                    (mb::BLUR_EX_RESET, true, 0, 0),
                    (mb::CLUSTER_DTOR, true, 0, 0),
                    (mb::DYN_GEO_RELEASE_POOL, true, 1, 1),
                    (mb::MENU_UNLOAD_EXTRA, true, 0, 0),
                ],
            );
            t.import_sym(mb::CLOSE_HANDLE, 0x20, 1, 1, Ret::Val(1));
            let dtor = t.stub_code(0x30, true, 1, 1, Ret::Val(0), false);
            for (name, sym, port) in cases {
                let obj = rand_obj(t, &mut rng, 0x80);
                let holder = rand_obj(t, &mut rng, 8);
                let tam = t.object(&[dtor], 0x40);
                for g in [
                    mb::POST_PROCESS_MANAGER,
                    mb::DISTORTION,
                    mb::RADIAL_BLUR,
                    mb::POST_EFFECT,
                    mb::SPECULAR_MAP_PARAMS,
                    mb::DYN_GEO_FILL,
                ] {
                    t.global(g.sym, if set { obj } else { 0 });
                }
                t.global(mb::BLUR_EX.sym, if set { holder } else { 0 });
                t.global(mb::TEXTURE_ANIMATION_MANAGER.sym, if set { tam } else { 0 });
                t.check(name, sym, Conv::Cdecl, &[], port);
            }
            t.reset();
        }
    });
}

#[test]
fn finalize_destructors_match_the_original() {
    // objects of random words: every pointer field set or 0, every owned
    // bit both ways.
    type P = fn(&mut G, u32) -> Out;
    let cases: [(&str, u32, u32, P); 13] = [
        (
            "Distortion_releaseBuffers_006c2410",
            0x006c_2410,
            0x80,
            |g, o| {
                b::distortion_release(g, o);
                Out::None
            },
        ),
        ("Distortion_terminate", 0x006c_31c0, 0x80, |g, o| {
            Out::U32(b::distortion_terminate(g, o))
        }),
        ("RadialLine_destroyRadialLine", 0x006f_7b20, 0x80, |g, o| {
            b::radial_line_destroy(g, o);
            Out::None
        }),
        ("RadialBlur_release_006c3cd0", 0x006c_3cd0, 0x40, |g, o| {
            Out::U32(b::radial_blur_release(g, o))
        }),
        (
            "BrokenScreenEffect_release_006d5a00",
            0x006d_5a00,
            0x60,
            |g, o| Out::U32(b::broken_screen_release(g, o)),
        ),
        ("DynGeoPool_dtor_006de420", 0x006d_e420, 0x20, |g, o| {
            b::dyn_geo_pool_dtor(g, o);
            Out::None
        }),
        (
            "PostProcessManager_dtor_00692820",
            0x0069_2820,
            0x1_2400,
            |g, o| Out::U32(b::post_manager_dtor(g, o)),
        ),
        (
            "PRedBlackTreePair_dtor_00633dc0",
            0x0063_3dc0,
            0x20,
            |g, o| Out::U32(b::tree_pair_dtor(g, o)),
        ),
        (
            "PRedBlackTreePair_dtor_00633e20",
            0x0063_3e20,
            0x20,
            |g, o| Out::U32(b::tree_pair_dtor(g, o)),
        ),
        (
            "PRedBlackTreePair_dtor_00633e80",
            0x0063_3e80,
            0x20,
            |g, o| Out::U32(b::tree_pair_dtor(g, o)),
        ),
        (
            "PRedBlackTreeFree_dtor_00633ef0",
            0x0063_3ef0,
            0x40,
            |g, o| Out::U32(b::tree_free_dtor(g, o)),
        ),
        (
            "PhyreScene_exitRelease_0065b820",
            0x0065_b820,
            0x1_0e00,
            |g, o| Out::U32(b::scene_exit_release(g, o)),
        ),
        (
            "RenderDelegationHelper_dtor_006ad180",
            0x006a_d180,
            0x10,
            |g, o| {
                b::delegations_dtor(g, o);
                Out::None
            },
        ),
    ];
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0102);
        for _ in 0..40 {
            for (name, sym, size, port) in cases {
                stub_all_but(
                    t,
                    1,
                    &[
                        (mb::OPERATOR_DELETE, false, 1, 0),
                        (mb::OPERATOR_DELETE_2, false, 1, 0),
                        (mb::PHYRE_FREE, false, 1, 0),
                        (mb::CLUSTER_DTOR, true, 0, 0),
                        (mb::SAMPLER_RELEASE, true, 0, 0),
                        (mb::SAMPLER_DTOR, true, 0, 0),
                        (mb::DISTORTION_RELEASE, true, 0, 0),
                        (mb::RADIAL_BLUR_MEMBER_DTOR, true, 0, 0),
                        (mb::RADIAL_LINE_DESTROY_THUNK, true, 0, 0),
                        (mb::RENDER_TARGET_RELEASE, true, 0, 0),
                        (mb::BROKEN_SCREEN_RESET, true, 0, 0),
                        (mb::TREE_ORPHAN_ALL, true, 0, 0),
                        (mb::TREE_CLEAR, true, 0, 0),
                        (mb::FREE_LIST_RESET, true, 0, 0),
                        (mb::FREE_LIST_DTOR, true, 0, 0),
                        (mb::FXAA_DTOR, true, 0, 0),
                        (mb::POST_MEMBER_DTOR, true, 0, 0),
                        (mb::MLAA_DTOR, true, 0, 0),
                        (mb::IGGY_MENU_DTOR, true, 0, 0),
                        (0x006f_5450, false, 0, 0),
                        (0x006e_8700, false, 0, 0),
                        (0x006f_7820, false, 0, 0),
                        (mb::CLEAR_DELEGATIONS, true, 1, 1),
                    ],
                    sym,
                );
                t.import_sym(mb::CLOSE_HANDLE, 0x40, 1, 1, Ret::Val(1));
                let o = rand_obj(t, &mut rng, size);
                t.check(name, sym, Conv::Thiscall(o), &[], |g| port(g, o));
                t.reset();
            }
            t.import_sym(mb::CLOSE_HANDLE, 0x40, 1, 1, Ret::Val(1));
            t.stub_sym(mb::OPERATOR_DELETE, 1, false, 1, 0, Ret::Val(0));
            // StdString_tidy (stdcall): short and long
            let s = rand_obj(t, &mut rng, 0x18);
            let cap = [0xf, 0x10, 0x11, rng.next()][rng.below(4) as usize];
            t.w32(s + 0x14, cap);
            t.check(
                "StdString_tidy_006335a0",
                0x0063_35a0,
                Conv::Stdcall,
                &[s],
                |g| {
                    b::std_string_tidy(g, s);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

#[test]
fn forwarders_b_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0103);
        for case in 0..8 {
            let scene = rand_obj(t, &mut rng, 0x1_0e00);
            let post = rand_obj(t, &mut rng, 0x100);
            let target = rand_obj(t, &mut rng, 8);
            t.w8(target + 4, (case % 2) as u8);
            t.w32(post + 0x8c, target);
            t.w32(scene + 0x10cc, post);
            if !t.snapshot || case % 2 == 0 {
                t.global(mb::SCENE.sym, scene);
            }
            stub_all(
                t,
                1,
                &[
                    (mb::SCENE_EXIT_RELEASE, true, 0, 0),
                    (mb::IGGY_EXIT, false, 0, 0),
                    (mb::POST_COPY_BUFFER, true, 6, 6),
                    (mb::SCENE_RENDER_CAPTURE, true, 0, 0),
                    (mb::IGGY_RENDER, true, 0, 0),
                ],
            );
            t.check(
                "graphicExitScene_006401c0",
                0x0064_01c0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(b::graphic_exit_scene(g)),
            );
            t.check(
                "PhyreScene_copyPostBuffer_006582a0",
                0x0065_82a0,
                Conv::Thiscall(scene),
                &[],
                |g| Out::U32(b::scene_copy_post_buffer(g, scene)),
            );
            t.stub_sym(mb::SCENE_COPY_POST_BUFFER, 9, true, 0, 0, Ret::Val(0x109));
            t.check(
                "graphicCopyPostBuffer_00639d10",
                0x0063_9d10,
                Conv::Cdecl,
                &[],
                |g| Out::U32(b::copy_post_buffer(g)),
            );
            t.check(
                "graphicRenderCaptureAndIggy_00642be0",
                0x0064_2be0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(b::render_capture_and_iggy(g)),
            );
            t.reset();
        }
    });
}

/// `graphicFinalize`'s callees: (address, `this`, stack words).
const FINALIZE_CALLEES: [(u32, bool, u32); 66] = [
    (0x006d_be80, true, 0),
    (0x006d_b730, true, 0),
    (0x0063_78b0, false, 0),
    (0x0067_1280, true, 0),
    (0x0067_10e0, true, 0),
    (0x0067_1140, true, 0),
    (0x0067_1180, true, 0),
    (0x0067_1200, true, 0),
    (0x0067_0920, true, 0),
    (0x0067_08e0, true, 0),
    (0x0067_11c0, true, 0),
    (0x0067_0ea0, true, 0),
    (0x0067_10a0, true, 0),
    (0x0067_1240, true, 0),
    (0x0065_9d20, true, 0),
    (0x006d_fdd0, false, 0),
    (0x006c_14c0, false, 0),
    (0x006c_1f20, true, 0),
    (0x006c_14a0, false, 0),
    (0x0069_f6c0, false, 0),
    (0x006a_11d0, true, 0),
    (0x0069_f580, false, 0),
    (0x0068_db80, false, 0),
    (0x0069_1b00, false, 0),
    (0x0069_f000, true, 0),
    (0x0069_f5c0, false, 0),
    (0x0069_f540, false, 0),
    (0x0069_2b90, false, 0),
    (0x0069_2bc0, false, 0),
    (0x006b_4990, false, 0),
    (0x006b_cc70, false, 0),
    (0x006a_beb0, false, 0),
    (0x006a_d180, true, 0),
    (0x006a_6dd0, false, 0),
    (0x006a_6e00, false, 0),
    (0x006b_62d0, false, 0),
    (0x006b_6210, false, 0),
    (0x006b_62a0, false, 0),
    (0x006b_6240, false, 0),
    (0x006b_61e0, false, 0),
    (0x006b_60f0, false, 0),
    (0x006b_61b0, false, 0),
    (0x006b_6180, false, 0),
    (0x006b_6270, false, 0),
    (0x006b_0120, false, 0),
    (0x006d_7370, false, 0),
    (0x006c_2210, false, 0),
    (0x006c_50b0, false, 0),
    (0x006c_57c0, false, 0),
    (0x006c_4a30, false, 0),
    (0x006c_3550, false, 0),
    (0x006e_76f0, false, 0),
    (0x006d_4db0, false, 0),
    (0x006e_32a0, false, 0),
    (0x0063_4790, true, 0),
    (0x0068_0000, false, 0),
    (0x0067_b820, false, 0),
    (0x006e_54b0, false, 0),
    (0x006d_ec20, false, 0),
    (0x006e_56d0, false, 0),
    (0x0069_a1a0, false, 0),
    (0x0069_a9c0, true, 0),
    (0x0069_a0e0, false, 0),
    (0x006d_c870, false, 0),
    (0x0063_4850, true, 0),
    (0x0064_8850, false, 2),
];

#[test]
fn graphic_finalize_matches_the_original() {
    // initialised or not; the singletons set or 0. Every callee stubbed
    // (logging ecx for the methods).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0104);
        for case in 0..8 {
            for (k, &(f, this, n)) in FINALIZE_CALLEES.iter().enumerate() {
                t.stub_sym(f, k as u32 + 1, this, n, 0, Ret::Val(0x1000 + k as u32));
            }
            t.stub_sym(mb::OPERATOR_DELETE, 0x99, false, 1, 0, Ret::Val(0));
            for g in [
                mb::CLOTH_JOBS,
                mb::DELEGATIONS,
                mb::LOADING_SCREEN,
                mb::SCENE,
                mb::VFX_RENDER_DATA_TABLE,
            ] {
                let v = if rng.chance(3) { 0 } else { rng.next() | 1 };
                t.global(g.sym, v);
            }
            t.global(mb::GRAPHIC_INITIALISED.sym, u32::from(case % 4 != 0));
            t.check("graphicFinalize", 0x0064_03e0, Conv::Cdecl, &[], |g| {
                Out::U32(b::graphic_finalize(g))
            });
            t.reset();
        }
    });
}

#[test]
fn tree_releases_match_the_original() {
    // the menu texture tree and the VFX shader manager's tree: chains of
    // 0..3 before the first node, 0..4 successors; once flag; buffers,
    // held clusters (any al byte) or not.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0105);
        for case in 0..60 {
            let vfx = case % 2 == 1;
            let head = if vfx {
                t.zeros(0x30)
            } else {
                t.rebase(mb::MENU_TREE)
            };
            let chain: Vec<u32> = (0..rng.below(4)).map(|_| t.zeros(0x20)).collect();
            for w in chain.windows(2) {
                t.w32(w[0], w[1]);
            }
            // (a 0 first word reads its node at 0 in the original: left out)
            let first = match chain.first() {
                Some(&c) if !rng.chance(5) => c,
                _ => head,
            };
            if let Some(&last) = chain.last() {
                t.w32(last, head);
            }
            let mut succ: Vec<u32> = (0..rng.below(5))
                .map(|_| rand_obj(t, &mut rng, 0x20))
                .collect();
            succ.push(head);
            succ.push(head);
            if vfx {
                t.w32(head + 0xc, first);
            } else {
                t.patch(head + 0xc, &first.to_le_bytes());
                let flag = u8::from(rng.chance(5));
                t.global(mb::MENU_TREE_RELEASED.sym, u32::from(flag));
                let scene = t.zeros(4);
                t.global(mb::SCENE.sym, scene);
            }
            let r = t.seq(&succ);
            t.stub_sym(mb::TREE_SUCCESSOR, 1, true, 1, 1, r);
            let held: Vec<u32> = (0..8).map(|_| rng.word()).collect();
            let r = t.seq(&held);
            t.stub_sym(mb::CLUSTER_HELD, 2, true, 1, 1, r);
            let m = t.zeros(4);
            t.stub_sym(mb::CLUSTER_MANAGER, 3, false, 0, 0, Ret::Val(m));
            stub_all(
                t,
                4,
                &[
                    (mb::CLUSTER_RELEASE, true, 1, 1),
                    (mb::TREE_ORPHAN_ALL, true, 0, 0),
                    (mb::FREE_LIST_RESET, true, 0, 0),
                    (mb::OPERATOR_DELETE, false, 1, 0),
                    (mb::SCENE_DELETE_MATERIAL, true, 4, 4),
                ],
            );
            let this = rand_obj(t, &mut rng, 0x40);
            if vfx {
                t.check(
                    "VfxShaderManager_releaseClusters_006c1f20",
                    0x006c_1f20,
                    Conv::Thiscall(head),
                    &[],
                    |g| Out::U32(b::vfx_shader_release_clusters(g, head)),
                );
            } else {
                t.check(
                    "MenuTextureTree_release_006a1230",
                    0x006a_1230,
                    Conv::Thiscall(this),
                    &[],
                    |g| Out::U32(b::menu_tree_release(g, this)),
                );
            }
            t.reset();
        }
    });
}

#[test]
fn dyn_geo_clear_matches_the_original() {
    // pooled objects set or 0; material arrays of 0..3 (inline for one),
    // with per-material words (heap or inline blocks) or without.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0106);
        for _ in 0..80 {
            let virt = t.stub_code(1, true, 0, 0, Ret::Val(0), false);
            let this = t.object(&[0, 0, 0, 0, virt], 0x40);
            for off in [4, 8, 0x10, 0x20, 0x2c, 0x30] {
                let v = if rng.chance(3) { 0 } else { rng.next() };
                t.w32(this + off, v);
            }
            if !rng.chance(4) {
                let n = rng.below(4);
                let mats: Vec<u32> = (0..n)
                    .map(|_| if rng.chance(4) { 0 } else { rng.next() })
                    .collect();
                let data = t.words(&mats);
                let first = mats.first().copied().unwrap_or(0);
                let top = if rng.chance(4) { 0x8000_0000 } else { 0 };
                let arr = t.words(&[n | top, if n == 1 { first } else { data }]);
                t.w32(this + 0x14, arr);
                for (off, inline) in [(0x18, 0x31), (0x1c, 0x35)] {
                    let v = match rng.below(4) {
                        0 => 0,
                        1 => this + inline,
                        _ => {
                            let w: Vec<u32> = (0..4).map(|_| rng.next()).collect();
                            t.words(&w)
                        }
                    };
                    t.w32(this + off, v);
                }
            }
            let scene = t.zeros(4);
            t.global(mb::SCENE.sym, scene);
            stub_all(
                t,
                2,
                &[
                    (mb::SCENE_DELETE_OBJECT, true, 2, 2),
                    (mb::SCENE_DELETE_POOL_28, true, 2, 2),
                    (mb::SCENE_DELETE_MATERIAL, true, 4, 4),
                    (mb::SCENE_RELEASE_POOL_ARRAY, true, 2, 2),
                    (mb::SCENE_DELETE_POOL_56, true, 2, 2),
                    (mb::USER_FREE, false, 1, 0),
                ],
            );
            t.check(
                "ClassDynamicGeometry_clear",
                0x006a_1040,
                Conv::Thiscall(this),
                &[],
                |g| {
                    b::dyn_geo_clear(g, this);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

#[test]
fn clear_delegations_matches_the_original() {
    // screen fades kept (flags 0x1001 or the fade task), the free-list
    // result negative or not, data or none; the 64 pairs' tasks likewise.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0107);
        for _ in 0..60 {
            let fade = t.rebase(mb::SCREEN_FADE_TASK);
            let helper = t.zeros(0x210);
            let task = |t: &mut Twins, rng: &mut Rng| {
                let d = rand_obj(t, rng, 0x14);
                match rng.below(4) {
                    0 => t.w32(d + 0xc, 0x1001),
                    1 => t.w32(d + 0x10, fade),
                    _ => {}
                }
                d
            };
            let n = rng.below(6);
            let ds: Vec<u32> = (0..n)
                .map(|_| if rng.chance(6) { 0 } else { task(t, &mut rng) })
                .collect();
            let data = t.words(&ds);
            t.w32(helper, n | if rng.chance(5) { 0x8000_0000 } else { 0 });
            t.w32(helper + 4, data);
            for j in 0..0x40 {
                let d = task(t, &mut rng);
                t.w32(helper + 8 + 8 * j, d);
                let v = rng.word();
                t.w32(helper + 0xc + 8 * j, v);
            }
            stub_mutex(t);
            let res: Vec<u32> = (0..8).map(|_| rng.word()).collect();
            let r = t.seq(&res);
            t.stub_sym(mb::SET_DATA_TO_FREE_LIST, 0x20, true, 1, 1, r);
            t.stub_sym(mb::OPERATOR_DELETE, 0x21, false, 1, 0, Ret::Val(0));
            let w = rng.word();
            t.check(
                "RenderDelegationHelper_clearDelegations_006adf70",
                0x006a_df70,
                Conv::Thiscall(helper),
                &[w],
                |g| Out::U32(b::clear_delegations(g, helper)),
            );
            t.reset();
        }
    });
}

#[test]
fn dyn_geo_release_pool_matches_the_original() {
    // pool or none, the current pool or not, 0..3 buffers (count signed),
    // buffers set or 0 with data or none.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0108);
        for _ in 0..60 {
            let this = rand_obj(t, &mut rng, 0x20);
            let i = rng.below(4);
            t.w32(this + 4 + 4 * i, 0);
            if !rng.chance(4) {
                let bufs: Vec<u32> = (0..3)
                    .map(|_| {
                        if rng.chance(4) {
                            0
                        } else {
                            rand_obj(t, &mut rng, 0x10)
                        }
                    })
                    .collect();
                let arr = t.words(&bufs);
                let pool = rand_obj(t, &mut rng, 0x18);
                let count = match rng.below(4) {
                    0 => 0xffff_ffff,
                    _ => rng.below(4),
                };
                t.w32(pool + 8, count);
                t.w32(pool + 0x10, arr);
                t.w32(this + 4 + 4 * i, pool);
                if rng.chance(2) {
                    t.w32(this, pool);
                }
            }
            stub_all(
                t,
                1,
                &[
                    (mb::OPERATOR_DELETE_2, false, 1, 0),
                    (mb::OPERATOR_DELETE, false, 1, 0),
                    (mb::DYN_GEO_POOL_DTOR, true, 0, 0),
                ],
            );
            t.import_sym(mb::CLOSE_HANDLE, 4, 1, 1, Ret::Val(1));
            t.check(
                "DynGeoMemManager_releasePool_006de860",
                0x006d_e860,
                Conv::Thiscall(this),
                &[i],
                |g| {
                    b::dyn_geo_release_pool(g, this, i);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

#[test]
fn cluster_move_instance_lists_matches_the_original() {
    // 0..3 instance lists, worlds set or 0, free chains of 0..3.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0109);
        for _ in 0..60 {
            let cluster = |t: &mut Twins, rng: &mut Rng| {
                let c = rand_obj(t, rng, 0x48);
                let ring = c + 0x1c;
                let lists: Vec<u32> = (0..rng.below(4)).map(|_| rand_obj(t, rng, 0x30)).collect();
                let mut prev = ring;
                for &l in &lists {
                    t.w32(prev, l);
                    t.w32(l + 4, prev);
                    prev = l;
                }
                t.w32(prev, ring);
                t.w32(ring + 4, prev);
                let free: Vec<u32> = (0..rng.below(4)).map(|_| rand_obj(t, rng, 4)).collect();
                let mut next = 0;
                for &f in free.iter().rev() {
                    t.w32(f, next);
                    next = f;
                }
                t.w32(c + 0x24, next);
                if rng.chance(3) {
                    t.w32(c + 0x40, 0);
                }
                c
            };
            let this = cluster(t, &mut rng);
            let other = cluster(t, &mut rng);
            stub_all(
                t,
                1,
                &[
                    (mb::MOVE_ASSETS, true, 1, 1),
                    (mb::WORLD_REMOVE_LIST, true, 1, 1),
                    (mb::WORLD_ADD_LIST, true, 1, 1),
                    (mb::LIST_REBIND, false, 1, 0),
                ],
            );
            t.check(
                "PCluster_moveInstanceLists_004409e0",
                0x0044_09e0,
                Conv::Thiscall(this),
                &[other],
                |g| Out::U32(b::cluster_move_instance_lists(g, this, other)),
            );
            t.reset();
        }
    });
}

#[test]
fn small_b_functions_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_010a);
        for _ in 0..40 {
            // PhyreCursor_seekValid: valid at once, after k steps, chains
            // ending at the end node or 0
            let cursor = rand_obj(t, &mut rng, 0x1c);
            let nodes: Vec<u32> = (0..3).map(|_| t.zeros(4)).collect();
            for w in nodes.windows(2) {
                t.w32(w[0], w[1]);
            }
            t.w32(cursor + 0x14, if rng.chance(2) { nodes[2] } else { 0 });
            t.w32(cursor + 0x18, if rng.chance(5) { 0 } else { nodes[0] });
            let valid: Vec<u32> = (0..6)
                .map(|k| {
                    if k == 5 || rng.chance(3) {
                        rng.next() | 1
                    } else {
                        rng.next() & !0xff
                    }
                })
                .collect();
            let r = t.seq(&valid);
            t.stub_sym(mb::CURSOR_IS_VALID, 1, true, 0, 0, r);
            t.stub_sym(mb::CURSOR_SET_NODE, 2, true, 1, 1, Ret::Val(0x22));
            t.check(
                "PhyreCursor_seekValid_00441bf0",
                0x0044_1bf0,
                Conv::Thiscall(cursor),
                &[],
                |g| Out::U32(b::cursor_seek_valid(g, cursor)),
            );
            t.reset();

            // PArray_copyFrom
            let this = rand_obj(t, &mut rng, 8);
            let src = rand_obj(t, &mut rng, 8);
            let p = if rng.chance(3) { 0 } else { rng.next() };
            t.stub_sym(mb::COPY_ARRAY, 3, false, 2, 0, Ret::Val(p));
            t.check(
                "PArray_copyFrom_006337e0",
                0x0063_37e0,
                Conv::Thiscall(this),
                &[src],
                |g| Out::U32(b::array_copy_from(g, this, src)),
            );
            t.reset();

            // PhyreScene_ChangeToHSVPass: on, off with the default pass
            // remapped to the HSV pass or another, or another pass
            let material = rand_obj(t, &mut rng, 0x10);
            let (def, hsv) = (t.rebase(mb::PASS_DEFAULT), t.rebase(mb::PASS_HSV));
            if rng.chance(2) {
                t.w32(material + 8, def);
            }
            if rng.chance(2) {
                t.w32(material + 0xc, hsv);
            }
            let on = match rng.below(3) {
                0 => 0x100,
                1 => 0,
                _ => rng.next(),
            };
            t.stub_sym(mb::SET_PASS_REMAPPING, 4, true, 2, 2, Ret::Val(0x44));
            t.check(
                "PhyreScene_ChangeToHSVPass",
                0x0064_faa0,
                Conv::Cdecl,
                &[material, on],
                |g| Out::U32(b::change_to_hsv_pass(g, material, on)),
            );
            t.reset();

            // graphicResetHSV
            let chr = if rng.chance(5) {
                0
            } else {
                rand_obj(t, &mut rng, 4)
            };
            let flip = match rng.below(3) {
                0 => 0x7fc0_1234,
                1 => 0x3f80_0000,
                _ => rng.next() & 0x7f7f_ffff,
            };
            t.global(mb::CHR_CLOTH_FLIP, flip);
            t.stub_sym(mb::SET_HSV_PARAM, 5, false, 5, 0, Ret::Val(0));
            t.stub_sym(mb::ENABLE_HSV, 6, false, 2, 0, Ret::Val(0));
            t.check(
                "graphicResetHSV_00637c10",
                0x0063_7c10,
                Conv::Cdecl,
                &[chr],
                |g| {
                    b::reset_hsv(g, chr);
                    Out::None
                },
            );
            t.reset();

            // LuaBind_pushObject (stdcall)
            let p = rand_obj(t, &mut rng, 4);
            let l = rng.next();
            stub_all(
                t,
                7,
                &[
                    (mb::LUA_PUSH_CLASS_OBJECT, false, 3, 0),
                    (mb::LUA_0094AC00, false, 2, 0),
                    (mb::LUA_PUSHNIL, false, 1, 0),
                ],
            );
            t.check(
                "LuaBind_pushObject_00441e70",
                0x0044_1e70,
                Conv::Stdcall,
                &[l, p],
                |g| Out::U32(b::lua_push_object(g, l, p)),
            );
            t.reset();
        }
    });
}

#[test]
fn load_queue_push_matches_the_original() {
    // lanes 0..2, ring indexes 0..15 and small negatives.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_010b);
        for _ in 0..40 {
            let this = rand_obj(t, &mut rng, 3 * 0x1e8 + 0x240);
            let lane = rng.below(3);
            let l = this + lane * 0x1e8;
            for k in 0..20 {
                let req = rand_obj(t, &mut rng, 0x28);
                t.w32(l + 0x1b0 + 4 * k, req);
            }
            for off in [0x1f0, 0x224] {
                let v = (rng.below(24) as i32 - 8) as u32;
                t.w32(l + off, v);
            }
            t.import_sym(mb::WAIT_FOR_SINGLE_OBJECT, 1, 2, 2, Ret::Val(0));
            t.import_sym(mb::RELEASE_SEMAPHORE, 2, 3, 3, Ret::Val(1));
            let args = [rng.next(), rng.next(), rng.next(), rng.next(), lane];
            t.check(
                "LoadQueue_push_00633670",
                0x0063_3670,
                Conv::Thiscall(this),
                &args,
                |g| {
                    b::load_queue_push(g, this, args[0], args[1], args[2], args[3], lane);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

#[test]
fn enable_hsv_matches_the_original() {
    // no model; the copy failing; 0..3 meshes of 0..3 materials; on any
    // word; the copy owned or borrowed. The mesh index left in the first
    // argument slot is compared too.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_010c);
        for _ in 0..60 {
            let n = rng.below(4);
            let mut meshes = vec![];
            let mut lists = vec![];
            for _ in 0..n {
                let k = rng.below(4);
                let mats: Vec<u32> = (0..k).map(|_| rng.next()).collect();
                let list = t.words(&mats);
                let arr = t.words(&[k | if rng.chance(4) { 0x8000_0000 } else { 0 }, 0]);
                let mesh = t.zeros(0x14);
                t.w32(mesh + 0x10, arr);
                meshes.push(mesh);
                lists.push(list);
            }
            let copy = if rng.chance(5) { 0 } else { t.words(&meshes) };
            let top = if rng.chance(4) { 0x8000_0000 } else { 0 };
            let model = if rng.chance(6) {
                0
            } else {
                t.words(&[n | top, rng.next()])
            };
            t.stub_sym(mb::COPY_ARRAY, 1, false, 2, 0, Ret::Val(copy));
            // PArray_Data: each mesh's list, as often as it has materials
            let mut seq = vec![];
            for (m, &l) in lists.iter().enumerate() {
                let k = t.r32(t.r32(meshes[m] + 0x10)) & 0x7fff_ffff;
                for _ in 0..k {
                    seq.push(l);
                }
            }
            seq.push(0);
            let r = t.seq(&seq);
            t.stub_sym(mb::ARRAY_DATA, 2, true, 0, 0, r);
            t.stub_sym(mb::CHANGE_TO_HSV_PASS, 3, false, 2, 0, Ret::Val(0));
            t.stub_sym(mb::PHYRE_FREE, 4, false, 1, 0, Ret::Val(0));
            let on = rng.word();
            let slot = first_slot(&t.b, 2);
            t.b_top = Some(slot - 0x40);
            t.check(
                "graphicEnableHSV",
                0x0064_0100,
                Conv::Cdecl,
                &[model, on],
                |g| {
                    b::enable_hsv(g, slot, model, on);
                    Out::None
                },
            );
            t.b_top = None;
            if model != 0 {
                let (x, y): (u32, u32) = (Mem::read(&t.a, slot), Mem::read(&t.b, slot));
                assert_eq!(x, y, "graphicEnableHSV: the argument slot");
            }
            t.reset();
        }
    });
}

// ---- batch 3 (ffx_prender::m5r_c) ----

/// The original's `ebp` for a function that starts `push ebp; mov ebp,
/// esp`, called from the oracle's call top with `words` stack words.
fn frame_ebp(o: &Oracle, words: u32) -> u32 {
    call_top(o) - 4 * words - 8
}

/// A float word for the x87 forwarders: anything but a signalling NaN.
fn float_word(rng: &mut Rng) -> u32 {
    loop {
        let x = match rng.below(4) {
            0 => 0x7fc0_0000 | rng.below(16),
            1 => (rng.below(2000) as f32 - 1000.0).to_bits(),
            _ => rng.next(),
        };
        let snan = x & 0x7f80_0000 == 0x7f80_0000 && x & 0x007f_ffff != 0 && x & 0x0040_0000 == 0;
        if !snan {
            return x;
        }
    }
}

#[test]
fn small_c_functions_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0201);
        for case in 0..30 {
            // getCurrentControlMode
            let im = t.zeros(4);
            t.stub_sym(mc::INPUT_MANAGER, 1, false, 0, 0, Ret::Val(im));
            let v = rng.next();
            t.stub_sym(mc::INPUT_VIBRATION_FLAG, 2, true, 0, 0, Ret::Val(v));
            t.check(
                "getCurrentControlMode",
                0x0063_8e30,
                Conv::Cdecl,
                &[],
                |g| Out::U32(c::current_control_mode(g)),
            );
            t.reset();

            // graphicGetSkipTexturePath: no manager, languages 0..12
            let mgr = t.words(&[0, case % 13]);
            let r = t.seq(&[if case % 7 == 0 { 0 } else { mgr }, mgr]);
            t.stub_sym(mc::LOCALIZATION_MANAGER, 3, false, 0, 0, r);
            let iob = t.zeros(0x80);
            t.import_sym(mc::IOB_FUNC, 4, 0, 0, Ret::Val(iob));
            t.import_sym(mc::FPRINTF, 5, 2, 0, Ret::Val(9));
            t.check(
                "graphicGetSkipTexturePath_00638e90",
                0x0063_8e90,
                Conv::Cdecl,
                &[],
                |g| Out::U32(c::skip_texture_path(g)),
            );
            t.reset();

            // graphicActivateRadialBlur, graphicCharacterShaderRestore,
            // graphicSegmentsAllReady, graphicDetachChrClusterFrom3DWorld
            let blur = t.zeros(4);
            t.stub_sym(mc::BLUR_EFFECT, 6, false, 0, 0, Ret::Val(blur));
            t.stub_sym(mc::BLUR_ACTIVATE, 7, true, 0, 0, Ret::Val(0x77));
            t.check(
                "graphicActivateRadialBlur",
                0x0063_9140,
                Conv::Cdecl,
                &[],
                |g| Out::U32(c::activate_radial_blur(g)),
            );
            t.stub_sym(mc::SHADER_RESET, 8, false, 0, 0, Ret::Val(0x78));
            t.check(
                "graphicCharacterShaderRestore",
                0x0063_d870,
                Conv::Cdecl,
                &[],
                |g| Out::U32(c::shader_restore(g)),
            );
            let scene = rand_obj(t, &mut rng, 0x1_0000);
            t.global(mc::SCENE.sym, scene);
            t.stub_sym(mc::SEGMENTS_ALL_READY, 9, true, 0, 0, Ret::Val(0x79));
            t.check(
                "graphicSegmentsAllReady_006522b0",
                0x0065_22b0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(c::segments_all_ready(g)),
            );
            t.stub_sym(mc::SCENE_DETACH_CHR_CLUSTER, 10, true, 0, 0, Ret::Val(0x7a));
            t.check(
                "graphicDetachChrClusterFrom3DWorld",
                0x0063_df50,
                Conv::Cdecl,
                &[],
                |g| Out::U32(c::graphic_detach_chr_cluster(g)),
            );
            t.stub_sym(mc::SCENE_MONSTER_ET_NAME, 11, true, 2, 2, Ret::Val(0x7b));
            let (et, which) = (rng.next(), rng.next());
            t.check(
                "graphicGetMonsterEtTextureName",
                0x0064_1330,
                Conv::Cdecl,
                &[et, which],
                |g| Out::U32(c::graphic_monster_et_texture_name(g, et, which)),
            );
            t.reset();

            // graphicAddScreenFadeTaskBeforeVFX
            let mut args: Vec<u32> = (0..4).map(|_| float_word(&mut rng)).collect();
            args.push(rng.next());
            t.stub_sym(mc::REGISTER_SCREEN_FADE, 12, false, 6, 0, Ret::Val(0x7c));
            t.check(
                "graphicAddScreenFadeTaskBeforeVFX",
                0x0063_92c0,
                Conv::Cdecl,
                &args,
                |g| {
                    Out::U32(c::add_screen_fade_before_vfx(
                        g,
                        [args[0], args[1], args[2], args[3]],
                        args[4],
                    ))
                },
            );
            t.reset();

            // attach / detach the character cluster
            let scene = rand_obj(t, &mut rng, 0x1_0000);
            t.w8(scene + 0xfb44, (case % 2) as u8);
            t.stub_sym(mc::WORLD_REMOVE_CLUSTER, 13, true, 1, 1, Ret::Val(0));
            t.stub_sym(mc::WORLD_ADD_CLUSTER, 14, true, 1, 1, Ret::Val(0));
            t.check(
                "PhyreScene_attachChrClusterTo3DWorld",
                0x0065_78e0,
                Conv::Thiscall(scene),
                &[],
                |g| {
                    c::attach_chr_cluster(g, scene);
                    Out::None
                },
            );
            t.w8(scene + 0xfb44, (case / 2 % 2) as u8);
            t.check(
                "PhyreScene_detachChrClusterFrom3DWorld_0065a550",
                0x0065_a550,
                Conv::Thiscall(scene),
                &[],
                |g| {
                    c::detach_chr_cluster(g, scene);
                    Out::None
                },
            );
            t.reset();

            // the draw situation callback
            let inst = if case % 3 == 0 {
                0
            } else {
                rand_obj(t, &mut rng, 0x20)
            };
            let cb = rng.next();
            t.check(
                "graphicCharacterInstanceSetDrawSituationCallback",
                0x0063_b7f0,
                Conv::Cdecl,
                &[inst, cb],
                |g| {
                    c::set_draw_situation_callback(g, inst, cb);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

#[test]
fn monster_et_texture_name_matches_the_original() {
    // records used or not, the id matching or not, either kind word.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0202);
        for _ in 0..100 {
            let scene = t.zeros(0x1_1000);
            let id = rng.below(4);
            for e in 0..8 {
                let rec = scene + 0xfcd0 + 0x20c * e;
                t.w32(rec - 4, u32::from(!rng.chance(3)));
                t.w32(rec, rng.below(4));
                let (k0, k1) = (rng.below(4) as u16, rng.below(4) as u16);
                t.write(rec + 4, &k0.to_le_bytes());
                t.write(rec + 6, &k1.to_le_bytes());
            }
            let et = t.zeros(0x20);
            t.w32(et + 0x10, id);
            let which = if rng.chance(6) {
                0x1_0001
            } else {
                rng.below(4)
            };
            t.check(
                "PhyreScene_getMonsterEtTextureName",
                0x0065_e200,
                Conv::Thiscall(scene),
                &[et, which],
                |g| Out::U32(c::monster_et_texture_name(g, scene, et, which)),
            );
            t.reset();
        }
    });
}

#[test]
fn character_lighting_and_shaders_match_the_original() {
    // lists of 0..3 instances, floats (no signalling NaN), no list;
    // characters without an instance.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0203);
        for case in 0..40 {
            let n = rng.below(4);
            let inst: Vec<u32> = (0..n).map(|_| rng.next()).collect();
            let data = t.words(&inst);
            let list = if case % 5 == 0 {
                0
            } else {
                t.words(&[n, data])
            };
            let l = t.zeros(4);
            t.stub_sym(mc::CHARACTER_LIGHTING, 1, false, 0, 0, Ret::Val(l));
            t.stub_sym(mc::LIGHTING_SET_4, 2, true, 5, 5, Ret::Val(0));
            t.stub_sym(mc::LIGHTING_SET_8, 3, true, 9, 9, Ret::Val(0));
            let f: Vec<u32> = (0..8).map(|_| float_word(&mut rng)).collect();
            let mut args = vec![list];
            args.extend(&f[..4]);
            t.check(
                "graphicCharacterLightingSet4_0063c9e0",
                0x0063_c9e0,
                Conv::Cdecl,
                &args,
                |g| {
                    c::lighting_set_4(g, list, [f[0], f[1], f[2], f[3]]);
                    Out::None
                },
            );
            let mut args = vec![list];
            args.extend(&f);
            t.check(
                "graphicCharacterLightingSet8_0063ca50",
                0x0063_ca50,
                Conv::Cdecl,
                &args,
                |g| {
                    let mut a = [0; 8];
                    a.copy_from_slice(&f);
                    c::lighting_set_8(g, list, a);
                    Out::None
                },
            );
            t.reset();

            let ri = if case % 4 == 0 {
                0
            } else {
                t.words(&[n, data])
            };
            t.stub_sym(mc::CHR_RENDER_INSTANCE, 4, false, 1, 0, Ret::Val(ri));
            t.stub_sym(mc::SHADER_SET_TYPE, 5, false, 2, 0, Ret::Val(1));
            t.stub_sym(mc::SHADER_SET_PARAMETER, 6, false, 3, 0, Ret::Val(1));
            t.stub_sym(
                mc::SHADER_SET_PARAMETER_TEXTURE,
                7,
                false,
                3,
                0,
                Ret::Val(1),
            );
            let (chr, x, y) = (rng.next(), rng.next(), rng.next());
            t.check(
                "graphicCharacterSetShaderType",
                0x0063_d810,
                Conv::Cdecl,
                &[chr, x],
                |g| Out::U32(c::set_shader_type(g, chr, x)),
            );
            t.check(
                "graphicCharacterShaderSetParameter",
                0x0063_d880,
                Conv::Cdecl,
                &[chr, x, y],
                |g| Out::U32(c::shader_set_parameter(g, chr, x, y)),
            );
            t.check(
                "graphicCharacterShaderSetParameterTexture",
                0x0063_d8e0,
                Conv::Cdecl,
                &[chr, x, y],
                |g| Out::U32(c::shader_set_parameter_texture(g, chr, x, y)),
            );
            t.reset();

            // graphicSetFreeCameraMode: there, made, allocation failing
            let cam = t.zeros(0x340);
            let made = case % 3;
            t.global(mc::FREE_CAMERA.sym, if made == 0 { cam } else { 0 });
            t.stub_sym(
                mc::OPERATOR_NEW,
                8,
                false,
                1,
                0,
                Ret::Val(if made == 1 { cam } else { 0 }),
            );
            t.stub_sym(mc::FREE_CAMERA_CTOR, 9, true, 0, 0, Ret::Val(cam));
            t.stub_sym(mc::FREE_CAMERA_SET_MODE, 10, true, 1, 1, Ret::Val(0x55));
            let mode = rng.next();
            t.check(
                "graphicSetFreeCameraMode_00640e20",
                0x0064_0e20,
                Conv::Cdecl,
                &[mode],
                |g| Out::U32(c::set_free_camera_mode(g, mode)),
            );
            t.reset();
        }
    });
}

#[test]
fn replace_summon_texture_matches_the_original() {
    // magic ids around 0x276..0x27d, the summons' names, every load
    // result, the cluster found or not, the skip list full or not. The
    // port's frame sits where the original's is.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0204);
        let names: [&[u8]; 5] = [b"m179\0", b"m180\0", b"m17\0", b"m1790\0", b"\0"];
        for _ in 0..120 {
            let ids: Vec<u32> = (0..4)
                .map(|_| match rng.below(4) {
                    0 => 0x27d,
                    1 => 0x275 + rng.below(10),
                    2 => 0x276,
                    _ => rng.next(),
                })
                .collect();
            let r = t.seq(&ids);
            t.stub_sym(mc::MAGIC_ID, 1, false, 0, 0, r);
            let inst = rng.next();
            t.stub_sym(mc::CHR_RENDER_INSTANCE, 2, false, 1, 0, Ret::Val(inst));
            let name = t.alloc(names[rng.below(5) as usize]);
            t.stub_sym(mc::CHR_NAME, 3, false, 1, 0, Ret::Val(name));
            let scene = t.zeros(4);
            t.global(mc::SCENE.sym, scene);
            t.stub_sym(
                mc::SCENE_IS_VFX_TEXTURE_LOADED,
                4,
                true,
                7,
                7,
                Ret::Val(rng.below(4)),
            );
            let m = t.zeros(4);
            t.stub_sym(mc::CLUSTER_MANAGER, 5, false, 0, 0, Ret::Val(m));
            t.stub_sym(mc::CLUSTER_BY_NAME, 6, true, 1, 1, Ret::Val(rng.below(2)));
            t.stub_sym(mc::SUMMON_REPLACE, 7, false, 2, 0, Ret::Val(0));
            t.stub_sym(mc::SUMMON_UPDATE, 8, false, 1, 0, Ret::Val(0));
            let skip = t.rebase(mc::TEX_ANIM_SKIP.sym);
            let count = (rng.below(8) as i32 - 1) as u32;
            let mut w = vec![count];
            w.extend((0..6).map(|_| rng.next()));
            let bytes: Vec<u8> = w.iter().flat_map(|v| v.to_le_bytes()).collect();
            t.patch(skip, &bytes);
            let chr = rng.next();
            let ebp = frame_ebp(&t.a, 1);
            t.b_top = Some(ebp - 0x140);
            t.check(
                "graphicCharacterReplaceSummonTexture",
                0x0063_d400,
                Conv::Cdecl,
                &[chr],
                |g| {
                    c::replace_summon_texture(g, ebp, chr);
                    Out::None
                },
            );
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn unloads_with_frames_match_the_original() {
    // the persistent VFX lists (the first unload succeeding or not); the
    // default menu textures (0..3 names, short and long strings).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0205);
        for case in 0..30 {
            let scene = t.zeros(4);
            t.stub_sym(mc::UNIFY_FILENAME, 1, false, 3, 0, Ret::Val(0));
            let r = t.seq(&[if case % 2 == 0 { 0 } else { 5 }, 6]);
            t.stub_sym(mc::SCENE_UNLOAD_VFX_TEXTURE, 2, true, 2, 2, r);
            let ebp = frame_ebp(&t.a, 0);
            t.b_top = Some(ebp - 0x240);
            t.check(
                "PhyreScene_unloadPersistentVFXTexture",
                0x0067_1280,
                Conv::Thiscall(scene),
                &[],
                |g| Out::U32(c::unload_persistent_vfx_texture(g, ebp, scene)),
            );
            t.b_top = None;
            t.reset();

            let head = t.zeros(0x20);
            let nodes: Vec<u32> = (0..rng.below(4)).map(|_| t.zeros(0x20)).collect();
            let mut prev = head;
            for &n in &nodes {
                t.w32(prev, n);
                let cap = [0xf, 0x10, rng.next()][rng.below(3) as usize];
                t.w32(n + 0x1c, cap);
                let v = rng.next();
                t.w32(n + 8, v);
                prev = n;
            }
            t.w32(prev, head);
            let list = t.rebase(mc::MENU_DEFAULT_TEXTURES);
            t.patch(list, &head.to_le_bytes());
            t.global(mc::SCENE.sym, scene);
            t.stub_sym(mc::UNIFY_FILENAME, 3, false, 3, 0, Ret::Val(0));
            t.stub_sym(mc::SCENE_UNLOAD_MENU_TEXTURE, 4, true, 1, 1, Ret::Val(0));
            t.stub_sym(mc::STRING_LIST_CLEAR, 5, true, 0, 0, Ret::Val(0x33));
            let ebp = frame_ebp(&t.a, 0);
            t.b_top = Some(ebp - 0x140);
            t.check(
                "MenuTextureManager_unloadDefaultTextures_006857c0",
                0x0068_57c0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(c::unload_default_menu_textures(g, ebp)),
            );
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn material_params_match_the_original() {
    // graphicSetHSVParam and graphicCharacterSetClipPlane: no model; the
    // copy failing; 0..3 meshes of 0..3 materials; the parameter missing,
    // fitting, past the buffer, below offset 4; no plane. The argument
    // slot (the mesh index) is compared too.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0206);
        for case in 0..80 {
            let hsv = case % 2 == 0;
            let n = rng.below(4);
            let mut meshes = vec![];
            let mut lists = vec![];
            let mut params = vec![];
            for _ in 0..n {
                let k = rng.below(4);
                let mut mats = vec![];
                for _ in 0..k {
                    let buf = t.words(&[rng.below(0x40), 0, 0, 0]);
                    let m = t.words(&[rng.next(), buf]);
                    mats.push(m);
                    let p = if rng.chance(4) {
                        0
                    } else {
                        let p = t.zeros(0x10);
                        let off = [3, 4, 5, rng.below(0x20)][rng.below(4) as usize] as u16;
                        let size = (rng.below(0x20) as u16) | ((rng.below(8) as u16) << 13);
                        t.write(p + 8, &off.to_le_bytes());
                        t.write(p + 0xa, &size.to_le_bytes());
                        p
                    };
                    params.push(p);
                }
                let list = t.words(&mats);
                let arr = t.words(&[k | if rng.chance(4) { 0x8000_0000 } else { 0 }, 0]);
                let mesh = t.zeros(0x14);
                t.w32(mesh + 0x10, arr);
                meshes.push(mesh);
                lists.push((list, k));
            }
            let copy = if rng.chance(5) { 0 } else { t.words(&meshes) };
            let top = if rng.chance(4) { 0x8000_0000 } else { 0 };
            let model = if rng.chance(6) {
                0
            } else {
                t.words(&[n | top, rng.next()])
            };
            t.stub_sym(mc::COPY_ARRAY, 1, false, 2, 0, Ret::Val(copy));
            let mut seq = vec![];
            for &(l, k) in &lists {
                for _ in 0..k {
                    seq.push(l);
                }
            }
            seq.push(0);
            let r = t.seq(&seq);
            t.stub_sym(mc::ARRAY_DATA, 2, true, 0, 0, r);
            params.push(0);
            let r = t.seq(&params);
            t.stub_sym(mc::FIND_PARAM, 3, true, 1, 1, r);
            t.stub_sym(mc::MEMCPY, 4, false, 3, 0, Ret::Val(0));
            t.stub_sym(mc::PHYRE_FREE, 5, false, 1, 0, Ret::Val(0));
            if hsv {
                let f: Vec<u32> = (0..4).map(|_| float_word(&mut rng)).collect();
                let slot = first_slot(&t.b, 5);
                let ebp = frame_ebp(&t.a, 5);
                t.b_top = Some(ebp - 0x60);
                let mut args = vec![model];
                args.extend(&f);
                t.check("graphicSetHSVParam", 0x0064_3340, Conv::Cdecl, &args, |g| {
                    c::set_hsv_param(g, ebp, slot, model, [f[0], f[1], f[2], f[3]]);
                    Out::None
                });
                if model != 0 {
                    let (x, y): (u32, u32) = (Mem::read(&t.a, slot), Mem::read(&t.b, slot));
                    assert_eq!(x, y, "graphicSetHSVParam: the argument slot");
                    // the floats the memcpy stub saw a pointer to
                    for k in 0..4 {
                        let (x, y): (u32, u32) = (
                            Mem::read(&t.a, ebp - 0x20 + 4 * k),
                            Mem::read(&t.b, ebp - 0x20 + 4 * k),
                        );
                        assert_eq!(x, y, "graphicSetHSVParam: the frame's float {k}");
                    }
                }
            } else {
                let plane = if rng.chance(4) { 0 } else { rng.next() };
                let slot = first_slot(&t.b, 2);
                t.b_top = Some(slot - 0x40);
                t.check(
                    "graphicCharacterSetClipPlane",
                    0x0063_d710,
                    Conv::Cdecl,
                    &[model, plane],
                    |g| {
                        c::set_clip_plane(g, slot, model, plane);
                        Out::None
                    },
                );
                if model != 0 {
                    let (x, y): (u32, u32) = (Mem::read(&t.a, slot), Mem::read(&t.b, slot));
                    assert_eq!(x, y, "graphicCharacterSetClipPlane: the argument slot");
                }
            }
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn release_character_clusters_matches_the_original() {
    // maps of 0..5 entries, each with 0..3 clusters, storage or none;
    // the vector copy into the frame by a stub that copies the three words
    // as 0x0064d620 does.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0207);
        for _ in 0..40 {
            let k = rng.below(6) as usize;
            let mut entries = vec![];
            for _ in 0..k {
                let n = rng.below(4);
                let cl: Vec<u32> = (0..n).map(|_| rng.next()).collect();
                let v = if n == 0 && rng.chance(2) {
                    0
                } else {
                    t.words(&cl)
                };
                let e = t.words(&[rng.next(), v, v + 4 * n, v + 4 * n]);
                entries.push(e);
            }
            let head = build_tree(t, &mut rng, &entries);
            let scene = t.zeros(0x1200);
            t.w32(scene + 0x11dc, head);
            t.w32(scene + 0x11e0, k as u32);
            let tam = t.zeros(4);
            t.stub_sym(mc::TEXANIM_MANAGER, 1, false, 0, 0, Ret::Val(tam));
            t.stub_sym(mc::TEXANIM_UNREGISTER_ALL, 2, true, 0, 0, Ret::Val(0));
            let m = t.zeros(4);
            t.stub_sym(mc::CLUSTER_MANAGER, 3, false, 0, 0, Ret::Val(m));
            t.stub_sym(mc::CLUSTER_RELEASE, 4, true, 1, 1, Ret::Val(0));
            t.stub_sym(mc::OPERATOR_DELETE, 5, false, 1, 0, Ret::Val(0));
            t.stub_sym(mc::TREE_ERASE, 6, true, 1, 1, Ret::Val(0));
            // the vector copy: [ecx..ecx+0xc] = [arg..arg+0xc], then a
            // logging stub that pops the argument
            let log = t.stub_code(7, true, 1, 1, Ret::Val(0), false);
            let mut code = vec![
                0x8b, 0x54, 0x24, 0x04, // mov edx, [esp+4]
                0x8b, 0x02, 0x89, 0x01, // mov eax, [edx]; mov [ecx], eax
                0x8b, 0x42, 0x04, 0x89, 0x41, 0x04, // +4
                0x8b, 0x42, 0x08, 0x89, 0x41, 0x08, // +8
                0x68, // push log
            ];
            code.extend(log.to_le_bytes());
            code.push(0xc3); // ret (to the log stub, which returns to the caller)
            let copy = t.a.alloc_code(&code).unwrap();
            assert_eq!(t.b.alloc_code(&code).unwrap(), copy);
            t.jump(mc::VECTOR_COPY, copy);
            let ebp = frame_ebp(&t.a, 0);
            t.b_top = Some(ebp - 0x60);
            t.check(
                "PhyreScene_releaseCharacterClusters_00670960",
                0x0067_0960,
                Conv::Thiscall(scene),
                &[],
                |g| Out::U32(c::release_character_clusters(g, ebp, scene)),
            );
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- batch 4 (ffx_prender::m5r_d) ----

/// A forwarder case: the original, its stack words, the callees to stub
/// (`sym`, `this`, logged words, `pops`) and the port on the arguments.
type Fwd = (
    &'static str,
    u32,
    usize,
    &'static [(u32, bool, u32, u32)],
    fn(&mut G, &[u32]) -> u32,
);

const GETTERS: [u32; 6] = [
    md::REMAP_ICONS,
    md::SPECULAR_MANAGER,
    md::SHADOW_MANAGER,
    md::FOG_MANAGER,
    md::MORPH_MANAGER,
    md::GET_APPLICATION,
];

#[test]
fn setters_match_the_original() {
    // random words (floats included, no signalling NaN) in every argument;
    // the managers' getters return a synthetic instance.
    let cases: [Fwd; 22] = [
        (
            "graphicMenuSetTextureUnloadedInMaterial",
            0x0064_2360,
            1,
            &[(md::MENU_SET_MATERIAL_UNLOADED, true, 1, 1)],
            |g, a| d::menu_set_texture_unloaded(g, a[0]),
        ),
        (
            "graphicRegisterDelegation_006429f0",
            0x0064_29f0,
            4,
            &[(md::REGISTER_DELEGATION, true, 4, 4)],
            |g, a| d::register_delegation(g, [a[0], a[1], a[2], a[3]]),
        ),
        (
            "graphicResetButtonIconScale",
            0x0064_2c60,
            0,
            &[(md::REMAP_RESET_SCALE, true, 0, 0)],
            |g, _| d::reset_button_icon_scale(g),
        ),
        (
            "graphicResize_00642cd0",
            0x0064_2cd0,
            0,
            &[(md::SCENE_RESIZE, true, 0, 0)],
            |g, _| d::resize(g),
        ),
        (
            "graphicSetAkazaNoLightSpecularPassParam",
            0x0064_2e00,
            9,
            &[(md::SPECULAR_AKAZA_NO_LIGHT, true, 9, 9)],
            |g, a| {
                let mut w = [0; 9];
                w.copy_from_slice(a);
                d::akaza_no_light_specular(g, w)
            },
        ),
        (
            "graphicSetAkazaSpecularPassParam",
            0x0064_2e40,
            12,
            &[(md::SPECULAR_AKAZA, true, 12, 12)],
            |g, a| {
                let mut w = [0; 12];
                w.copy_from_slice(a);
                d::akaza_specular(g, w)
            },
        ),
        (
            "graphicSetNoLightSpecularPassParam",
            0x0064_3770,
            10,
            &[(md::SPECULAR_NO_LIGHT, true, 10, 10)],
            |g, a| {
                let mut w = [0; 10];
                w.copy_from_slice(a);
                d::no_light_specular(g, w)
            },
        ),
        (
            "graphicSetSpecularMapParams",
            0x0064_3980,
            5,
            &[(md::SPECULAR_SHADE, true, 5, 5)],
            |g, a| d::specular_map_params(g, [a[0], a[1], a[2], a[3], a[4]]),
        ),
        (
            "graphicSetButtonIconScale",
            0x0064_2ec0,
            2,
            &[(md::REMAP_SET_SCALE, true, 2, 2)],
            |g, a| d::set_button_icon_scale(g, a[0], a[1]),
        ),
        (
            "graphicSetShadowFloats_00642fb0",
            0x0064_2fb0,
            2,
            &[(md::SHADOW_SET_FLOATS, true, 2, 2)],
            |g, a| d::set_shadow_floats(g, a[0], a[1]),
        ),
        (
            "graphicSetFogColour",
            0x0064_3150,
            3,
            &[(md::FOG_SET_COLOUR, true, 3, 3)],
            |g, a| d::set_fog_colour(g, [a[0], a[1], a[2]]),
        ),
        (
            "graphicSetFogParam_00643180",
            0x0064_3180,
            1,
            &[(md::FOG_SET_3760, true, 1, 1)],
            |g, a| d::set_fog_float(g, md::FOG_SET_3760, a[0]),
        ),
        (
            "graphicSetFogFar",
            0x0064_31a0,
            1,
            &[(md::FOG_SET_FAR, true, 1, 1)],
            |g, a| d::set_fog_float(g, md::FOG_SET_FAR, a[0]),
        ),
        (
            "graphicSetFogNear",
            0x0064_31e0,
            1,
            &[(md::FOG_SET_NEAR, true, 1, 1)],
            |g, a| d::set_fog_float(g, md::FOG_SET_NEAR, a[0]),
        ),
        (
            "graphicSetAppFloatD8_006432c0",
            0x0064_32c0,
            1,
            &[],
            |g, a| d::set_app_float(g, 0xd8, a[0]),
        ),
        (
            "graphicSetAppFloatD4_006432e0",
            0x0064_32e0,
            1,
            &[],
            |g, a| d::set_app_float(g, 0xd4, a[0]),
        ),
        (
            "graphicSetAppFloatDC_00643300",
            0x0064_3300,
            1,
            &[],
            |g, a| d::set_app_float(g, 0xdc, a[0]),
        ),
        (
            "graphicSetAppFloatD0_00643320",
            0x0064_3320,
            1,
            &[],
            |g, a| d::set_app_float(g, 0xd0, a[0]),
        ),
        (
            "graphicSetSceneSix0_00643630",
            0x0064_3630,
            6,
            &[(md::SCENE_SET_SIX, true, 7, 7)],
            |g, a| d::scene_set_six(g, [a[0], a[1], a[2], a[3], a[4], a[5]], 0),
        ),
        (
            "graphicSetSceneSix1_006436a0",
            0x0064_36a0,
            6,
            &[(md::SCENE_SET_SIX, true, 7, 7)],
            |g, a| d::scene_set_six(g, [a[0], a[1], a[2], a[3], a[4], a[5]], 1),
        ),
        (
            "graphicSetMeshVertexColorMorphValue",
            0x0064_3700,
            2,
            &[(md::MORPH_SET_VERTEX_COLOR, true, 2, 2)],
            |g, a| d::set_vertex_color_morph(g, a[0], a[1]),
        ),
        (
            "graphicSetNumberText_006438c0",
            0x0064_38c0,
            1,
            &[(md::SCENE_SET_NUMBER_TEXT, true, 1, 1)],
            |g, a| d::set_number_text(g, a[0]),
        ),
    ];
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0301);
        for _ in 0..12 {
            for &(name, sym, n, stubs, port) in &cases {
                let inst = rand_obj(t, &mut rng, 0x100);
                for (k, &gtr) in GETTERS.iter().enumerate() {
                    t.stub_sym(gtr, 0x40 + k as u32, false, 0, 0, Ret::Val(inst));
                }
                let scene = rand_obj(t, &mut rng, 0x10);
                t.global(md::SCENE.sym, scene);
                t.global(md::GRAPHIC_UI.sym, inst);
                t.global(md::DELEGATIONS.sym, inst);
                for (k, &(s, this, words, pops)) in stubs.iter().enumerate() {
                    t.stub_sym(
                        s,
                        1 + k as u32,
                        this,
                        words,
                        pops,
                        Ret::Val(0x900 + k as u32),
                    );
                }
                let args: Vec<u32> = (0..n).map(|_| float_word(&mut rng)).collect();
                t.check(name, sym, Conv::Cdecl, &args, |g| Out::U32(port(g, &args)));
                t.reset();
            }
        }
    });
}

#[test]
fn loading_push_and_others_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0302);
        for _ in 0..40 {
            // the loading queue: below, at and past its capacity; data 0
            // with count 1 (a null slot)
            let q = t.rebase(md::LOADING_QUEUE);
            let count = rng.below(6);
            let cap = match rng.below(3) {
                0 => count,
                1 => count + 1,
                _ => count + rng.below(4),
            };
            let data = if count == 0 && rng.chance(3) {
                0
            } else {
                t.zeros(0x40)
            };
            let w = [count, cap, data];
            let bytes: Vec<u8> = w.iter().flat_map(|v| v.to_le_bytes()).collect();
            t.patch(q, &bytes);
            stub_mutex(t);
            let mtx = rng.next();
            t.global(md::LOADING_MUTEX.sym, mtx);
            t.stub_sym(md::LOADING_QUEUE_RESERVE, 1, true, 1, 1, Ret::Val(0));
            let x = rng.next();
            t.check(
                "graphicCharacterLoadingPush_00642980",
                0x0064_2980,
                Conv::Cdecl,
                &[x],
                |g| Out::U32(d::loading_push(g, x)),
            );
            t.reset();

            // the HSV reset delegation: its argument slot as the data
            let helper = t.zeros(4);
            t.global(md::DELEGATIONS.sym, helper);
            t.stub_sym(md::REGISTER_DELEGATION, 2, true, 4, 4, Ret::Val(0x31));
            let chr = rng.next();
            let slot = first_slot(&t.b, 1);
            t.check(
                "graphicRegisterResetHSV_00642a10",
                0x0064_2a10,
                Conv::Cdecl,
                &[chr],
                |g| Out::U32(d::register_reset_hsv(g, slot)),
            );
            t.reset();

            // the FMV pause byte
            let m = rand_obj(t, &mut rng, 0x800);
            t.stub_sym(md::FMV_MANAGER, 3, false, 0, 0, Ret::Val(m));
            let on = rng.word();
            t.check(
                "graphicSetPauseVideoAtEndFrame",
                0x0064_37f0,
                Conv::Cdecl,
                &[on],
                |g| Out::U32(d::set_pause_video_at_end(g, on)),
            );
            t.reset();

            // the matrix transpose (frame)
            let scene = rand_obj(t, &mut rng, 0x100);
            if rng.chance(3) {
                t.w32(scene + 0xdc, 0);
            }
            t.global(md::SCENE.sym, scene);
            t.stub_sym(md::MATRIX_INVERSE, 4, false, 2, 0, Ret::Val(0x41));
            t.stub_sym(md::MATRIX_TRANSPOSE, 5, false, 2, 0, Ret::Val(0x42));
            let ebp = frame_ebp(&t.a, 0);
            t.b_top = Some(ebp - 0xc0);
            t.check(
                "graphicSceneMatrixTranspose_00643b80",
                0x0064_3b80,
                Conv::Cdecl,
                &[],
                |g| Out::U32(d::scene_matrix_transpose(g, ebp)),
            );
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- batch 5 (ffx_prender::m5r_e) ----

#[test]
fn forwarders_e_match_the_original() {
    let cases: [Fwd; 6] = [
        (
            "graphicSetMeshesVisible_00643df0",
            0x0064_3df0,
            1,
            &[(me::SCENE_SET_MESHES_VISIBLE, true, 1, 1)],
            |g, a| e::set_meshes_visible(g, a[0]),
        ),
        (
            "graphicTextureVideoPlay",
            0x0064_4280,
            1,
            &[(me::SCENE_TEXTURE_VIDEO_PLAY, true, 1, 1)],
            |g, a| e::texture_video_play(g, a[0]),
        ),
        (
            "graphicSetWaveStrength_00645e40",
            0x0064_5e40,
            0,
            &[(me::SCENE_SET_WAVE_STRENGTH, true, 0, 0)],
            |g, _| e::set_wave_strength(g),
        ),
        (
            "graphicShadowReset_00643e80",
            0x0064_3e80,
            1,
            &[
                (me::SHADOW_MANAGER, false, 0, 0),
                (me::SHADOW_RESET_IF, true, 1, 1),
            ],
            |g, a| e::shadow_reset(g, a[0]),
        ),
        (
            "graphicUpdateRadialLineLogic",
            0x0064_49d0,
            4,
            &[
                (me::BLUR_EFFECT, false, 0, 0),
                (me::BLUR_RADIAL_LINE, true, 4, 4),
            ],
            |g, a| e::update_radial_line_logic(g, [a[0], a[1], a[2], a[3]]),
        ),
        (
            "graphicSetAnimatedMaterialsChar_006440e0",
            0x0064_40e0,
            11,
            &[
                (me::TEXANIM_MANAGER, false, 0, 0),
                (me::TEXANIM_SET_CHAR, true, 9, 9),
            ],
            |g, a| {
                let mut w = [0; 11];
                w.copy_from_slice(a);
                e::set_animated_materials_char(g, w)
            },
        ),
    ];
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0401);
        for _ in 0..16 {
            for &(name, sym, n, stubs, port) in &cases {
                let scene = rand_obj(t, &mut rng, 0x10);
                t.global(me::SCENE.sym, scene);
                for (k, &(s, this, words, pops)) in stubs.iter().enumerate() {
                    t.stub_sym(
                        s,
                        1 + k as u32,
                        this,
                        words,
                        pops,
                        Ret::Val(0x900 + k as u32),
                    );
                }
                let args: Vec<u32> = (0..n)
                    .map(|_| {
                        if rng.chance(4) {
                            rng.below(3)
                        } else {
                            float_word(&mut rng)
                        }
                    })
                    .collect();
                t.check(name, sym, Conv::Cdecl, &args, |g| Out::U32(port(g, &args)));
                t.reset();
            }
        }
    });
}

#[test]
fn small_e_methods_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0402);
        for case in 0..40 {
            // PostProcessTweakUtil_setEnabled: the argument byte and +4
            let this = rand_obj(t, &mut rng, 0x190);
            t.w8(this + 4, (case % 2) as u8);
            t.stub_sym(me::TEXT_SET_ENABLED, 1, true, 1, 1, Ret::Val(0x11));
            t.stub_sym(me::TEXT_RESIZE, 2, true, 2, 2, Ret::Val(0x12));
            let on = [0, 1, 0x100, rng.next()][(case / 2 % 4) as usize];
            t.check(
                "PostProcessTweakUtil_setEnabled_006984f0",
                0x0069_84f0,
                Conv::Thiscall(this),
                &[on],
                |g| Out::U32(e::post_tweak_set_enabled(g, this, on)),
            );
            t.reset();

            // DebugMenuManager_disableTweaks and the soft reset
            for (k, &g) in me::TWEAK_GETTERS.iter().enumerate() {
                t.stub_sym(g, 0x10 + k as u32, false, 0, 0, Ret::Val(0x1000 + k as u32));
            }
            t.stub_sym(me::TWEAK_GETTER_LAST, 0x20, false, 0, 0, Ret::Val(0));
            for (k, &f) in me::TWEAK_SET_ENABLED.iter().enumerate() {
                t.stub_sym(f, 0x30 + k as u32, true, 1, 1, Ret::Val(0x2000 + k as u32));
            }
            t.check(
                "DebugMenuManager_disableTweaks_006b49c0",
                0x006b_49c0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(e::disable_tweaks(g)),
            );
            let scene = rand_obj(t, &mut rng, 0xfa00);
            let dmm = t.zeros(4);
            t.stub_sym(me::DEBUG_MENU_MANAGER, 0x40, false, 0, 0, Ret::Val(dmm));
            t.stub_sym(me::DISABLE_TWEAKS, 0x41, true, 0, 0, Ret::Val(0x42));
            t.global(me::SCENE_3D.sym, 0);
            t.check(
                "PhyreScene_softResetScene",
                0x0066_fad0,
                Conv::Thiscall(scene),
                &[],
                |g| Out::U32(e::soft_reset(g, scene)),
            );
            t.reset();

            // ShadowManager_resetIfSolution: 0x203b, other, negative
            let shadow = t.zeros(0x4060);
            let s = [0x203b, 0x203a, 0xffff_e03b, 0][(case % 4) as usize];
            t.w32(shadow + 0x4050, s);
            t.stub_sym(me::SHADOW_RESET_SOLUTION, 3, true, 1, 1, Ret::Val(0x33));
            let x = rng.next();
            t.check(
                "ShadowManager_resetIfSolution_006ce0c0",
                0x006c_e0c0,
                Conv::Thiscall(shadow),
                &[x],
                |g| Out::U32(e::shadow_reset_if_solution(g, shadow, x)),
            );
            t.reset();

            // BlurEffect_updateRadialLine
            let blur = rand_obj(t, &mut rng, 0x20);
            t.stub_sym(me::RADIAL_LINE_UPDATE, 4, true, 4, 4, Ret::Val(0x44));
            let a: Vec<u32> = (0..4).map(|_| float_word(&mut rng)).collect();
            t.check(
                "BlurEffect_updateRadialLine_006c3fd0",
                0x006c_3fd0,
                Conv::Thiscall(blur),
                &a,
                |g| {
                    Out::U32(e::blur_update_radial_line(
                        g,
                        blur,
                        [a[0], a[1], a[2], a[3]],
                    ))
                },
            );
            t.reset();

            // graphicVideoGetElapsedFrame
            let m = rand_obj(t, &mut rng, 0x700);
            t.stub_sym(me::FMV_MANAGER, 5, false, 0, 0, Ret::Val(m));
            let frame = rng.next();
            t.stub_sym(me::FMV_CURRENT_FRAME, 6, true, 0, 0, Ret::Val(frame));
            t.check(
                "graphicVideoGetElapsedFrame",
                0x0064_5d90,
                Conv::Cdecl,
                &[],
                |g| Out::U32(e::video_elapsed_frame(g)),
            );
            t.reset();

            // the scene cursors' constructors: an empty or a filled list
            let cluster = rand_obj(t, &mut rng, 0x40);
            if case % 2 == 0 {
                t.w32(cluster + 0x1c, cluster + 0x1c);
            }
            for (sym, class, seek) in [
                (0x0064_cda0, me::CURSOR_CLASS2, me::CURSOR_SEEK2),
                (0x0064_ce00, me::CURSOR_CLASS3, me::CURSOR_SEEK3),
                (0x0064_ce60, me::CURSOR_CLASS4, me::CURSOR_SEEK4),
            ] {
                let cursor = rand_obj(t, &mut rng, 0x20);
                t.stub_sym(seek, 7, true, 0, 0, Ret::Val(0x77));
                t.check(
                    &format!("PhyreSceneCursor_ctor_{sym:08x}"),
                    sym,
                    Conv::Thiscall(cursor),
                    &[cluster],
                    |g| Out::U32(e::cursor_ctor(g, cursor, cluster, class, seek)),
                );
                t.reset();
            }

            // the word arrays: n 0..4, count 0..n, the allocation failing
            let n = rng.below(5);
            let count = if n == 0 { 0 } else { rng.below(n + 1) };
            let src = rand_obj(t, &mut rng, 0x20);
            let block = if rng.chance(5) {
                0
            } else {
                rand_obj(t, &mut rng, 0x20)
            };
            t.stub_sym(me::PHYRE_MALLOC_ALIGNED, 8, false, 2, 0, Ret::Val(block));
            for sym in [0x0064_f7f0, 0x0064_f860] {
                t.check(
                    &format!("WordArray_alloc_{sym:08x}"),
                    sym,
                    Conv::Cdecl,
                    &[n, src, count],
                    |g| Out::U32(e::word_array_alloc(g, n, src, count)),
                );
            }
            t.reset();

            // AtlThrowLastError: positive, zero, negative codes
            let code = [5, 0, 0x8000_4005, 0x8_0002][(case % 4) as usize];
            t.import_sym(me::GET_LAST_ERROR, 9, 0, 0, Ret::Val(code));
            t.throw_sym(me::THROW, 10, 1);
            t.check(
                "AtlThrowLastError_0064fa70",
                0x0064_fa70,
                Conv::Cdecl,
                &[],
                |g| {
                    e::throw_last_error(g);
                    Out::None
                },
            );
            t.reset();

            // the vector allocators: 0, small, past the limit, no memory
            let n = [0, 3, 0x4000_0000, 2][(case % 4) as usize];
            let vec = rand_obj(t, &mut rng, 12);
            let p = if case % 8 == 3 {
                0
            } else {
                0x0ab0_0000 + (case << 8)
            };
            t.stub_sym(me::OPERATOR_NEW, 11, false, 1, 0, Ret::Val(p));
            t.throw_import(me::XLENGTH_ERROR, 12, 1);
            t.throw_import(me::XBAD_ALLOC, 13, 0);
            let throws = n > 0x3fff_ffff || (n != 0 && p == 0);
            for sym in [0x0065_4d40, 0x0065_4db0] {
                t.check(
                    &format!("StdVector_allocate_{sym:08x}"),
                    sym,
                    Conv::Thiscall(vec),
                    &[n],
                    |g| {
                        let r = e::vector_allocate(g, vec, n);
                        if throws { Out::None } else { Out::U32(r) }
                    },
                );
            }
            t.reset();

            // the loading cluster: either flag
            let scene = rand_obj(t, &mut rng, 0xfa00);
            t.w8(scene + 0xf9d1, (case % 2) as u8);
            t.w8(scene + 0xf9d0, (case / 2 % 2) as u8);
            t.stub_sym(me::WORLD_REMOVE_CLUSTER, 14, true, 1, 1, Ret::Val(0));
            t.stub_sym(me::WORLD_ADD_CLUSTER, 15, true, 1, 1, Ret::Val(0));
            t.check(
                "PhyreScene_restoreLoadingCluster_0065a590",
                0x0065_a590,
                Conv::Thiscall(scene),
                &[],
                |g| {
                    e::restore_loading_cluster(g, scene);
                    Out::None
                },
            );
            t.reset();

            // the water texture (frame) and the font texture fix-up (slot)
            t.stub_sym(me::UNIFY_FILENAME, 16, false, 3, 0, Ret::Val(0));
            t.stub_sym(
                me::SCENE_LOAD_VFX_TEXTURE_ASYNC,
                17,
                true,
                2,
                2,
                Ret::Val(0x55),
            );
            let ebp = frame_ebp(&t.a, 0);
            t.b_top = Some(ebp - 0x140);
            t.check(
                "PhyreScene_loadWaterTexture_00665fd0",
                0x0066_5fd0,
                Conv::Thiscall(scene),
                &[],
                |g| Out::U32(e::load_water_texture(g, ebp, scene)),
            );
            t.b_top = None;
            t.stub_sym(me::ASYNC_LOAD_MANAGER, 18, false, 0, 0, Ret::Val(1));
            t.stub_sym(me::GET_APPLICATION, 19, false, 0, 0, Ret::Val(2));
            t.stub_sym(me::FIXUP_CLUSTERS, 20, false, 2, 0, Ret::Val(0x66));
            let slot = first_slot(&t.b, 1);
            let cluster = rng.next();
            t.check(
                "PhyreScene_onFontTextureLoaded",
                0x0066_66a0,
                Conv::Cdecl,
                &[cluster],
                |g| Out::U32(e::on_font_texture_loaded(g, slot)),
            );
            t.reset();
        }
    });
}

#[test]
fn array_reserves_and_resize_b5_match_the_original() {
    let list: [(u32, u32); 4] = [
        (0x0066_7f00, 4),
        (0x0066_8180, 8),
        (0x0066_8300, 8),
        (0x0066_8380, 1),
    ];
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0403);
        for &(sym, elem) in &list {
            for _ in 0..30 {
                let arr = t.zeros(12);
                t.w32(arr, rng.word());
                t.w32(arr + 4, rng.word());
                if !rng.chance(3) {
                    t.w32(arr + 8, 0x0abc_0000 | rng.below(0x1000));
                }
                let n = rng.word();
                let block = if rng.chance(4) { 0 } else { 0x0def_0000 };
                t.stub_sym(ma::USER_MALLOC, 1, false, 1, 0, Ret::Val(block));
                t.stub_sym(ma::MEMCPY, 2, false, 3, 0, Ret::Keep);
                t.stub_sym(ma::USER_FREE, 3, false, 1, 0, Ret::Val(0x77));
                t.check(
                    &format!("Array_Reserve_{sym:08x}"),
                    sym,
                    Conv::Thiscall(arr),
                    &[n],
                    |g| Out::U32(a::reserve(g, arr, n, elem)),
                );
                t.reset();
            }
        }
        // PArray_ptr_resize_0066c240 (ffx_pload's port of its twin
        // 0x0047efc0): the same size, a new size (owned or borrowed old
        // block), 0, the allocation failing
        for _ in 0..60 {
            let n = rng.below(6);
            let count = if rng.chance(4) {
                n
            } else {
                rng.below(6) | if rng.chance(4) { 0x8000_0000 } else { 0 }
            };
            let arr = t.words(&[count, rng.next()]);
            let block = if rng.chance(5) { 0 } else { t.zeros(0x20) };
            t.stub_sym(me::PHYRE_MALLOC_ALIGNED, 1, false, 2, 0, Ret::Val(block));
            t.stub_sym(ma::PHYRE_FREE, 2, false, 1, 0, Ret::Val(0));
            t.check(
                "PArray_ptr_resize_0066c240",
                0x0066_c240,
                Conv::Thiscall(arr),
                &[n],
                |g| {
                    Out::U32(ffx_pload::descriptor::bridge::pointer_array_resize(
                        g, arr, n,
                    ))
                },
            );
            t.reset();
        }
    });
}

// ---- batch 6 (ffx_prender::m5r_f) ----

/// The class descriptor and binding template functions: (name, address,
/// family, the class's constant: vtable, accessor or class record).
const TEMPLATES: [(&str, u32, u8, u32); 47] = [
    (
        "PClassDescriptorForType_DistortionGridDynamicMesh_Destruct_00673fc0",
        0x0067_3fc0,
        b'A',
        mf::VT_DISTORTION_GRID,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMesh_scalarDeletingDtor_00674020",
        0x0067_4020,
        b'B',
        mf::VT_DISTORTION_GRID,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMesh_scalarDeletingDtor_00674050",
        0x0067_4050,
        b'B',
        mf::VT_DISTORTION_GRID,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMesh_scalarDeletingDtor_00674080",
        0x0067_4080,
        b'B',
        mf::VT_DISTORTION_GRID,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMesh_virtualStore_00674650",
        0x0067_4650,
        b'I',
        0,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMesh_return0_00674680",
        0x0067_4680,
        b'Z',
        0,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMesh_returnMinus1_00674690",
        0x0067_4690,
        b'M',
        0,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMesh_accessorStore_006746a0",
        0x0067_46a0,
        b'D',
        mf::GET_DISTORTION_GRID,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMesh_notSupported_006746c0",
        0x0067_46c0,
        b'E',
        0,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMesh_pushPointer_006748a0",
        0x0067_48a0,
        b'F',
        mf::CLASS_DISTORTION_GRID,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMesh_pushObject_006748e0",
        0x0067_48e0,
        b'G',
        mf::CLASS_DISTORTION_GRID,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMesh_registerFlagged_00674910",
        0x0067_4910,
        b'H',
        mf::CLASS_DISTORTION_GRID,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMeshInstance_Destruct_00674c20",
        0x0067_4c20,
        b'A',
        mf::VT_DISTORTION_GRID_INSTANCE,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMeshInstance_scalarDeletingDtor_00674c50",
        0x0067_4c50,
        b'B',
        mf::VT_DISTORTION_GRID_INSTANCE,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMeshInstance_scalarDeletingDtor_00674c80",
        0x0067_4c80,
        b'B',
        mf::VT_DISTORTION_GRID_INSTANCE,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMeshInstance_scalarDeletingDtor_00674cb0",
        0x0067_4cb0,
        b'B',
        mf::VT_DISTORTION_GRID_INSTANCE,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMeshInstance_returnMinus1_006750b0",
        0x0067_50b0,
        b'M',
        0,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMeshInstance_accessorStore_006750c0",
        0x0067_50c0,
        b'D',
        mf::GET_DISTORTION_GRID_INSTANCE,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMeshInstance_notSupported_006750f0",
        0x0067_50f0,
        b'E',
        0,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMeshInstance_pushPointer_00675190",
        0x0067_5190,
        b'F',
        mf::CLASS_DISTORTION_GRID_INSTANCE,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMeshInstance_pushObject_006751d0",
        0x0067_51d0,
        b'G',
        mf::CLASS_DISTORTION_GRID_INSTANCE,
    ),
    (
        "PClassDescriptorForType_DistortionGridDynamicMeshInstance_registerFlagged_00675200",
        0x0067_5200,
        b'H',
        mf::CLASS_DISTORTION_GRID_INSTANCE,
    ),
    (
        "PClassDescriptorForType_RadialLineDynamicMeshInstance_Destruct_00675540",
        0x0067_5540,
        b'A',
        mf::VT_RADIAL_LINE_INSTANCE,
    ),
    (
        "PClassDescriptorForType_RadialLineDynamicMeshInstance_scalarDeletingDtor_00675570",
        0x0067_5570,
        b'B',
        mf::VT_RADIAL_LINE_INSTANCE,
    ),
    (
        "PClassDescriptorForType_RadialLineDynamicMeshInstance_scalarDeletingDtor_006755a0",
        0x0067_55a0,
        b'B',
        mf::VT_RADIAL_LINE_INSTANCE,
    ),
    (
        "PClassDescriptorForType_RadialLineDynamicMeshInstance_scalarDeletingDtor_006755d0",
        0x0067_55d0,
        b'B',
        mf::VT_RADIAL_LINE_INSTANCE,
    ),
    (
        "PClassDescriptorForType_RadialLineDynamicMesh_scalarDeletingDtor_00675fa0",
        0x0067_5fa0,
        b'B',
        mf::VT_RADIAL_LINE,
    ),
    (
        "PClassDescriptorForType_RadialLineDynamicMesh_scalarDeletingDtor_00675fd0",
        0x0067_5fd0,
        b'B',
        mf::VT_RADIAL_LINE,
    ),
    (
        "PClassDescriptorForType_RadialLineDynamicMesh_scalarDeletingDtor_00676000",
        0x0067_6000,
        b'B',
        mf::VT_RADIAL_LINE,
    ),
    (
        "PClassDescriptorForType_ShadowDynamicMesh_scalarDeletingDtor_006784a0",
        0x0067_84a0,
        b'B',
        mf::VT_SHADOW,
    ),
    (
        "PClassDescriptorForType_ShadowDynamicMesh_scalarDeletingDtor_006784d0",
        0x0067_84d0,
        b'B',
        mf::VT_SHADOW,
    ),
    (
        "PClassDescriptorForType_ShadowDynamicMesh_scalarDeletingDtor_00678500",
        0x0067_8500,
        b'B',
        mf::VT_SHADOW,
    ),
    (
        "PClassDescriptorForType_ShadowDynamicMesh_virtualStore_00678b10",
        0x0067_8b10,
        b'I',
        0,
    ),
    (
        "PClassDescriptorForType_ShadowDynamicMesh_accessorStore_00678b60",
        0x0067_8b60,
        b'D',
        mf::GET_SHADOW,
    ),
    (
        "PClassDescriptorForType_ShadowDynamicMesh_notSupported_00678b80",
        0x0067_8b80,
        b'E',
        0,
    ),
    (
        "PClassDescriptorForType_ShadowDynamicMesh_pushPointer_00678d70",
        0x0067_8d70,
        b'F',
        mf::CLASS_SHADOW,
    ),
    (
        "PClassDescriptorForType_ShadowDynamicMesh_pushObject_00678db0",
        0x0067_8db0,
        b'G',
        mf::CLASS_SHADOW,
    ),
    (
        "PClassDescriptorForType_ShadowDynamicMesh_registerFlagged_00678de0",
        0x0067_8de0,
        b'H',
        mf::CLASS_SHADOW,
    ),
    (
        "PClassDescriptorForType_ShadowDynamicMeshInstance_Destruct_006f94c0",
        0x006f_94c0,
        b'A',
        mf::VT_SHADOW_INSTANCE,
    ),
    (
        "PClassDescriptorForType_ShadowDynamicMeshInstance_scalarDeletingDtor_006f94f0",
        0x006f_94f0,
        b'B',
        mf::VT_SHADOW_INSTANCE,
    ),
    (
        "PClassDescriptorForType_ShadowDynamicMeshInstance_scalarDeletingDtor_006f9520",
        0x006f_9520,
        b'B',
        mf::VT_SHADOW_INSTANCE,
    ),
    (
        "PClassDescriptorForType_ShadowDynamicMeshInstance_scalarDeletingDtor_006f9550",
        0x006f_9550,
        b'B',
        mf::VT_SHADOW_INSTANCE,
    ),
    (
        "PClassDescriptorForType_ShadowDynamicMeshInstance_accessorStore_006f99a0",
        0x006f_99a0,
        b'D',
        mf::GET_SHADOW_INSTANCE,
    ),
    (
        "PClassDescriptorForType_ShadowDynamicMeshInstance_notSupported_006f99e0",
        0x006f_99e0,
        b'E',
        0,
    ),
    (
        "PClassDescriptorForType_ShadowDynamicMeshInstance_pushPointer_006f9a80",
        0x006f_9a80,
        b'F',
        mf::CLASS_SHADOW_INSTANCE,
    ),
    (
        "PClassDescriptorForType_ShadowDynamicMeshInstance_pushObject_006f9ac0",
        0x006f_9ac0,
        b'G',
        mf::CLASS_SHADOW_INSTANCE,
    ),
    (
        "PClassDescriptorForType_ShadowDynamicMeshInstance_registerFlagged_006f9af0",
        0x006f_9af0,
        b'H',
        mf::CLASS_SHADOW_INSTANCE,
    ),
];

#[test]
fn class_templates_match_the_original() {
    // every member of every template: the flag bit both ways, null and
    // set pointers, random words for what the constant returns ignore.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0501);
        for _ in 0..12 {
            for &(name, sym, fam, k) in &TEMPLATES {
                t.stub_sym(mf::DESCRIPTOR_DESTRUCT, 1, true, 0, 0, Ret::Val(0x11));
                t.stub_sym(mf::PHYRE_FREE, 2, false, 1, 0, Ret::Val(0));
                t.stub_sym(mf::PHYRE_PRINTF, 3, false, 2, 0, Ret::Val(0x33));
                t.stub_sym(mf::LUA_PUSH_CLASS_OBJECT, 4, false, 3, 0, Ret::Val(0x44));
                t.stub_sym(mf::LUA_0094AC00, 5, false, 2, 0, Ret::Val(0x55));
                t.stub_sym(mf::LUA_PUSHNIL, 6, false, 1, 0, Ret::Val(0x66));
                let reg = rng.next();
                t.stub_sym(mf::CLASS_REGISTER, 7, true, 2, 2, Ret::Val(reg));
                let obj = rand_obj(t, &mut rng, 0x20);
                let maybe = |rng: &mut Rng, p: u32| if rng.chance(3) { 0 } else { p };
                match fam {
                    b'A' => t.check(name, sym, Conv::Thiscall(obj), &[], |g| {
                        Out::U32(f::descriptor_destruct(g, obj, k))
                    }),
                    b'B' => {
                        let flags = rng.word();
                        t.check(name, sym, Conv::Thiscall(obj), &[flags], |g| {
                            Out::U32(f::descriptor_delete(g, obj, flags, k))
                        });
                    }
                    b'I' => {
                        let virt = t.stub_code(8, true, 4, 4, Ret::Val(0x88), false);
                        let this = t.object(&[0, 0, 0, 0, 0, virt], 8);
                        let dst = t.zeros(4);
                        let out = t.words(&[dst, rng.next()]);
                        let a: Vec<u32> = (0..4).map(|_| rng.next()).chain([out]).collect();
                        t.check(name, sym, Conv::Thiscall(this), &a, |g| {
                            Out::U32(f::virtual_store(g, this, [a[0], a[1], a[2], a[3]], out))
                        });
                    }
                    b'Z' => {
                        let x = rng.next();
                        t.check(name, sym, Conv::Stdcall, &[x], |_| Out::U32(0));
                    }
                    b'M' => {
                        let a: Vec<u32> = (0..3).map(|_| rng.next()).collect();
                        t.check(name, sym, Conv::Stdcall, &a, |_| Out::U32(0xffff_ffff));
                    }
                    b'D' => {
                        let v = rng.next();
                        t.stub_sym(k, 9, false, 1, 0, Ret::Val(v));
                        let l = rng.next();
                        let out = t.zeros(4);
                        t.check(name, sym, Conv::Stdcall, &[l, out], |g| {
                            Out::U32(f::accessor_store(g, l, out, k))
                        });
                    }
                    b'E' => {
                        let a = [rng.next(), rng.next()];
                        t.check(name, sym, Conv::Stdcall, &a, |g| {
                            Out::U32(f::not_supported(g))
                        });
                    }
                    b'F' => {
                        let p = maybe(&mut rng, obj);
                        let pp = t.words(&[p]);
                        let l = rng.next();
                        t.check(name, sym, Conv::Stdcall, &[l, pp], |g| {
                            Out::U32(f::push_pointer(g, l, pp, k))
                        });
                    }
                    b'G' => {
                        let p = maybe(&mut rng, obj);
                        let l = rng.next();
                        t.check(name, sym, Conv::Stdcall, &[l, p], |g| {
                            Out::U32(f::push_object(g, l, p, k))
                        });
                    }
                    _ => {
                        let p = maybe(&mut rng, obj);
                        let flag = rng.word();
                        t.check(name, sym, Conv::Stdcall, &[p, flag], |g| {
                            Out::U32(f::register_flagged(g, p, flag, k))
                        });
                    }
                }
                t.reset();
            }
        }
    });
}

// ---- batch 7 (ffx_prender::m5r_g) ----

#[test]
fn small_g_functions_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0601);
        for case in 0..40 {
            // the scalar deleting destructors: flag bit both ways
            let flags = rng.word();
            for (name, sym, dtor) in [
                (
                    "PostProcessTweakUtil_scalarDeletingDtor_00692a60",
                    0x0069_2a60,
                    mg::TEXT_ENTRY_DTOR,
                ),
                (
                    "TextureAnimationInfoChar_scalarDeletingDtor_006afe00",
                    0x006a_fe00,
                    mg::TEXANIM_INFO_DTOR,
                ),
                (
                    "TextureAnimationInfo_scalarDeletingDtor_006afe30",
                    0x006a_fe30,
                    mg::TEXANIM_INFO2_DTOR,
                ),
                (
                    "DebugMenuManager_scalarDeletingDtor_006b4920",
                    0x006b_4920,
                    0,
                ),
            ] {
                let this = rand_obj(t, &mut rng, 0x20);
                if dtor != 0 {
                    t.stub_sym(dtor, 1, true, 0, 0, Ret::Val(0x11));
                }
                t.stub_sym(mg::OPERATOR_DELETE, 2, false, 1, 0, Ret::Val(0));
                t.check(name, sym, Conv::Thiscall(this), &[flags], |g| {
                    Out::U32(match sym {
                        0x0069_2a60 => gb::post_tweak_delete(g, this, flags),
                        0x006b_4920 => gb::debug_menu_delete(g, this, flags),
                        _ => gb::delete_with(g, this, flags, dtor),
                    })
                });
                t.reset();
            }

            // DebugMenuManager_CreateInstance: there, made, no memory
            let made = case % 3;
            t.global(
                mg::DEBUG_MENU_MANAGER.sym,
                if made == 0 { rng.next() | 1 } else { 0 },
            );
            let p = if made == 1 { t.zeros(8) } else { 0 };
            t.stub_sym(mg::OPERATOR_NEW, 3, false, 1, 0, Ret::Val(p));
            t.check(
                "DebugMenuManager_CreateInstance",
                0x006b_4950,
                Conv::Cdecl,
                &[],
                |g| {
                    gb::debug_menu_create(g);
                    Out::None
                },
            );
            t.reset();

            // PhyreScene_AddVFXToUI2DList: below, at, past the capacity;
            // data 0 with count 0 (a null slot)
            let scene = t.zeros(0x3870);
            let count = rng.below(6);
            let cap = [count, count + 1, count + rng.below(3)][rng.below(3) as usize];
            let data = if count == 0 && rng.chance(3) {
                0
            } else {
                t.zeros(0x40)
            };
            t.w32(scene + 0x3860, count);
            t.w32(scene + 0x3864, cap);
            t.w32(scene + 0x3868, data);
            t.stub_sym(mg::RESERVE_4, 4, true, 1, 1, Ret::Val(0));
            let vfx = rng.next();
            t.check(
                "PhyreScene_AddVFXToUI2DList",
                0x006a_0d40,
                Conv::Thiscall(scene),
                &[vfx],
                |g| Out::U32(gb::add_vfx_to_ui2d(g, scene, vfx)),
            );
            t.reset();

            // the pair pushes
            let this = rand_obj(t, &mut rng, 0x30);
            if case % 2 == 0 {
                t.w32(this + 0xc, 0);
            }
            let s = t.zeros(4);
            t.global(mg::SCENE.sym, s);
            t.stub_sym(mg::PUSH_PAIR_3890, 5, true, 2, 2, Ret::Val(0x55));
            for sym in [0x006a_0f30, 0x006a_0f50] {
                t.check(
                    &format!("VfxRenderData_pushPair_{sym:08x}"),
                    sym,
                    Conv::Thiscall(this),
                    &[],
                    |g| Out::U32(gb::push_pair(g, this)),
                );
            }
            t.reset();

            // the tree clear: spines of 0..4
            let head = t.zeros(0x10);
            t.w8(head + 0xd, 1);
            let mut node = head;
            for _ in 0..rng.below(5) {
                let n = rand_obj(t, &mut rng, 0x10);
                t.w8(n + 0xd, 0);
                t.w32(n, node);
                node = n;
            }
            let table = t.words(&[head, rng.next()]);
            t.w32(head + 4, node);
            t.stub_sym(mg::ERASE_SUBTREE, 6, true, 1, 1, Ret::Val(0));
            t.stub_sym(mg::OPERATOR_DELETE, 7, false, 1, 0, Ret::Val(0));
            t.check(
                "VfxRenderDataTable_clearTree_006a0fe0",
                0x006a_0fe0,
                Conv::Thiscall(table),
                &[],
                |g| Out::U32(gb::clear_tree(g, table)),
            );
            t.reset();

            // the owned resets
            for (sym, checked) in [
                (0x006a_d130, false),
                (0x006a_f4e0, false),
                (0x006a_f570, true),
                (0x006c_7e60, false),
                (0x006d_e370, false),
            ] {
                let n = rng.below(4) | if rng.chance(3) { 0x8000_0000 } else { 0 };
                let d = if rng.chance(3) { 0 } else { rng.next() };
                let arr = t.words(&[n, d]);
                t.stub_sym(mg::PHYRE_FREE, 8, false, 1, 0, Ret::Val(0));
                t.check(
                    &format!("PArray_Reset_{sym:08x}"),
                    sym,
                    Conv::Thiscall(arr),
                    &[],
                    |g| {
                        gb::owned_reset(g, arr, checked);
                        Out::None
                    },
                );
                t.reset();
            }

            // the specular pass
            let this = t.zeros(4);
            t.stub_sym(mg::RENDER_CORRESPOND_SPECULAR, 9, true, 2, 2, Ret::Val(0));
            t.stub_sym(mg::RESET_SPECULAR_MAPS, 10, true, 0, 0, Ret::Val(0x99));
            let x = rng.next();
            t.check(
                "SpecularMapParamsManager_renderSpecularPass",
                0x006c_5b60,
                Conv::Thiscall(this),
                &[x],
                |g| Out::U32(gb::render_specular_pass(g, this, x)),
            );
            t.reset();

            // the predicates
            let (fa, fb) = (
                float_word(&mut rng),
                if rng.chance(4) {
                    0x7fc0_0000
                } else {
                    float_word(&mut rng)
                },
            );
            let fb = if case % 5 == 0 { fa } else { fb };
            t.check(
                "FloatLess_006c94e0",
                0x006c_94e0,
                Conv::Cdecl,
                &[fa, fb],
                |_| Out::U8(gb::float_less(fa, fb)),
            );
            for (sym, ids, is) in [
                (0x006c_c8e0_u32, &[1_u32, 0x202b][..], true),
                (0x006c_c900, &[0x2144, 0x206e][..], false),
                (0x006c_c920, &[0x201a, 0x2021, 0x215d, 0x5009][..], false),
                (0x006c_c950, &[0x20dd, 0x215d, 0x215a][..], false),
            ] {
                let x = if rng.chance(2) {
                    ids[rng.below(ids.len() as u32) as usize]
                } else {
                    rng.next()
                };
                t.check(
                    &format!("ShadowSolution_{sym:08x}"),
                    sym,
                    Conv::Cdecl,
                    &[x],
                    |_| Out::U32(gb::solution_in(x, ids, is)),
                );
            }

            // the dump line, the slot claim
            let entry = rand_obj(t, &mut rng, 0x110);
            let file = rng.next();
            t.import_sym(mg::FPRINTF, 11, 7, 0, Ret::Val(0x31));
            t.check(
                "TextUtil_dumpEntry_006dd200",
                0x006d_d200,
                Conv::Cdecl,
                &[entry, file],
                |g| Out::U32(gb::dump_entry(g, entry, file)),
            );
            t.reset();
            let slots = t.zeros(0x40);
            let free = rng.below(18);
            let results: Vec<u32> = (0..16)
                .map(|k| if k == free { 0 } else { rng.next() | 1 })
                .collect();
            let r = t.seq(&results);
            t.import_sym(mg::INTERLOCKED_COMPARE_EXCHANGE, 12, 3, 3, r);
            let iob = t.zeros(0x80);
            t.import_sym(mg::IOB_FUNC, 13, 0, 0, Ret::Val(iob));
            t.import_sym(mg::FPRINTF, 14, 3, 0, Ret::Val(0x32));
            let v = rng.next();
            t.check(
                "TextUtil_claimSlot_006f5de0",
                0x006f_5de0,
                Conv::Thiscall(slots),
                &[v],
                |g| Out::U32(gb::claim_slot(g, slots, v)),
            );
            t.reset();

            // the blur's float copies
            let this = rand_obj(t, &mut rng, 0x50);
            let w: Vec<u32> = (0..4).map(|_| float_word(&mut rng)).collect();
            let src = t.words(&w);
            for (sym, off, n) in [(0x006f_8b10, 0x38, 4), (0x006f_8ba0, 0x48, 2)] {
                t.check(
                    &format!("BlurEffect_setFloats_{sym:08x}"),
                    sym,
                    Conv::Thiscall(this),
                    &[src],
                    |g| Out::U32(gb::copy_floats(g, this, src, off, n)),
                );
            }
            t.reset();

            // the shadow meshes' binds, the PRX unload
            t.stub_sym(mg::REGISTER_TYPE, 15, false, 1, 0, Ret::Val(0));
            t.stub_sym(mg::ADD_TO_NAMESPACE, 16, true, 0, 0, Ret::Val(0x77));
            for (sym, class) in [
                (0x0067_88c0, mg::CLASS_SHADOW),
                (0x006f_9870, mg::CLASS_SHADOW_INSTANCE),
            ] {
                t.check(
                    &format!("ShadowBind_{sym:08x}"),
                    sym,
                    Conv::Cdecl,
                    &[],
                    |g| Out::U32(gb::bind_class(g, class)),
                );
            }
            t.import_sym(mg::FREE_LIBRARY, 17, 1, 1, Ret::Val(1));
            t.global(mg::PRX_STATE.sym, 3);
            let m = rng.next();
            t.check("PhyreFIOS_UnloadPrx", 0x006f_9d20, Conv::Cdecl, &[m], |g| {
                Out::U32(gb::unload_prx(g, m))
            });
            t.reset();

            // the PS2 movie helpers
            let stream = t.zeros(0x5_0760);
            t.w8(stream + 0x5_075c, (case % 2) as u8);
            t.stub_sym(mg::STR_PC_A, 18, false, 2, 0, Ret::Val(0xa));
            t.stub_sym(mg::STR_PC_B, 19, false, 2, 0, Ret::Val(0xb));
            let (x, y) = (rng.next(), rng.next());
            t.check(
                "syncStrPcSelect_0077e880",
                0x0077_e880,
                Conv::Cdecl,
                &[x, y, stream],
                |g| Out::U32(gb::sync_str_select(g, y, stream)),
            );
            t.reset();
            let out = t.zeros(8);
            let (a0, a1, a2) = (rng.next(), rng.next(), rng.next());
            t.check(
                "GsPackTag_0077ead0",
                0x0077_ead0,
                Conv::Cdecl,
                &[out, a0, a1, a2],
                |g| Out::U32(gb::gs_pack_tag(g, out, a0, a1, a2)),
            );
            t.reset();
        }
    });
}

// ---- batch 8 (ffx_prender::m5r_h and twins) ----

#[test]
fn batch8_twins_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0701);
        for case in 0..30 {
            // the delegation runs, flags 8 and 2: callbacks stubbed,
            // one-shot entries, free-list results
            for (sym, flag) in [(0x006a_da20_u32, 8_u32), (0x006a_dad0, 2)] {
                let cb = t.stub_code(1, false, 1, 0, Ret::Val(0), false);
                let n = rng.below(5);
                let ds: Vec<u32> = (0..n)
                    .map(|_| {
                        if rng.chance(6) {
                            return 0;
                        }
                        let d = rand_obj(t, &mut rng, 0x14);
                        let f = (rng.next() & !0xff)
                            | if rng.chance(2) { flag } else { 0 }
                            | u32::from(rng.chance(2));
                        t.w32(d + 0xc, f);
                        t.w32(d + 0x10, cb);
                        d
                    })
                    .collect();
                let data = t.words(&ds);
                let helper = t.words(&[n, data]);
                stub_mutex(t);
                let res: Vec<u32> = (0..8).map(|_| rng.word()).collect();
                let r = t.seq(&res);
                t.stub_sym(mb::SET_DATA_TO_FREE_LIST, 2, true, 1, 1, r);
                t.stub_sym(mb::OPERATOR_DELETE, 3, false, 1, 0, Ret::Val(0));
                t.stub_sym(ma::DELEGATIONS_COMPACT, 4, true, 0, 0, Ret::Val(0));
                t.check(
                    &format!("delegations_{sym:08x}"),
                    sym,
                    Conv::Thiscall(helper),
                    &[],
                    |g| {
                        Out::U32(ffx_prender::p6_more::bridge::run_delegations(
                            g, helper, flag,
                        ))
                    },
                );
                t.reset();
            }

            // destructInstance: virtual 0 of p - 4
            let virt = t.stub_code(5, true, 1, 1, Ret::Val(0), false);
            let obj = t.object(&[virt], 8);
            let p = obj + 4;
            t.check(
                "destructInstance_006f9910",
                0x006f_9910,
                Conv::Stdcall,
                &[p],
                |g| Out::U8(ffx_pload::descriptor::bridge::destroy_virtual(g, 0, 4, p)),
            );
            t.reset();

            // the shadow descriptors' constructInstance
            let obj = if case % 4 == 0 {
                0
            } else {
                rand_obj(t, &mut rng, 0x40)
            };
            t.stub_sym(0x0049_68c0, 6, true, 0, 0, Ret::Val(0));
            t.check(
                "constructInstance_006788e0",
                0x0067_88e0,
                Conv::Thiscall(0x1234),
                &[obj],
                |g| {
                    Out::U8(ffx_prender::p6_descriptor::bridge::construct_with(
                        g,
                        obj,
                        0x0049_68c0,
                        Some(0x00b4_4c78),
                        None,
                    ))
                },
            );
            let object = [0, 4, rng.next() | 8][(case % 3) as usize];
            t.stub_sym(0x0043_cbb0, 7, true, 0, 0, Ret::Val(object));
            t.stub_sym(0x004f_a420, 8, true, 1, 1, Ret::Val(0));
            t.check(
                "constructInstance_006f9890",
                0x006f_9890,
                Conv::Thiscall(0x1234),
                &[obj],
                |g| {
                    Out::U8(
                        ffx_pother_b::object::bridge::construct_mesh_instance_default(
                            g,
                            obj,
                            0x00cc_c158,
                            0x00b4_c250,
                        ),
                    )
                },
            );
            t.reset();

            // the radial line's parameter setters (they copy from their
            // own argument words)
            for (sym, guard, param) in [
                (0x006f_8b40_u32, 0x6c_u32, 0x6c_u32),
                (0x006f_8be0, 0x74, 0x70),
            ] {
                let this = rand_obj(t, &mut rng, 0x80);
                if rng.chance(5) {
                    t.w32(this + guard, 0);
                }
                let rec = t.zeros(0x10);
                let off = [3_u16, 4, 8, 0x30][rng.below(4) as usize];
                t.write(rec + 8, &off.to_le_bytes());
                t.write(rec + 0xa, &(0x10_u16 | 0xe000).to_le_bytes());
                t.w32(this + param, rec);
                let buf = t.words(&[[0x18, 0x40][rng.below(2) as usize], 0, 0, 0]);
                let holder = t.words(&[0, buf]);
                t.w32(this + 0x1c, holder);
                t.stub_sym(mc::MEMCPY, 9, false, 3, 0, Ret::Val(0));
                let a: Vec<u32> = (0..4).map(|_| rng.next()).collect();
                let args = first_slot(&t.b, 4);
                t.check(
                    &format!("setParam_{sym:08x}"),
                    sym,
                    Conv::Thiscall(this),
                    &a,
                    |g| {
                        let slot = ffx_glue_other::frames::ParamSlot {
                            guard,
                            param,
                            holder: 0x1c,
                        };
                        // the port reads the caller's words: put them where
                        // the original's slots were
                        for (k, &w) in a.iter().enumerate() {
                            g.write(args + 4 * k as u32, w);
                        }
                        Out::U32(ffx_glue_other::frames::post_set_param(g, slot, this, args))
                    },
                );
                t.reset();
            }
        }
    });
}

#[test]
fn batch8_functions_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0702);
        for case in 0..40 {
            // the loading cluster's move
            let scene = rand_obj(t, &mut rng, 0xfa00);
            t.w8(scene + 0xf9d1, (case % 2) as u8);
            t.w8(scene + 0xf9d0, (case / 2 % 2) as u8);
            t.stub_sym(mh::WORLD_REMOVE_CLUSTER, 1, true, 1, 1, Ret::Val(0));
            t.stub_sym(mh::WORLD_ADD_CLUSTER, 2, true, 1, 1, Ret::Val(0));
            t.check(
                "PhyreScene_moveLoadingCluster_00657920",
                0x0065_7920,
                Conv::Thiscall(scene),
                &[],
                |g| {
                    h::move_loading_cluster(g, scene);
                    Out::None
                },
            );
            t.reset();

            // the mesh matrix push (frame)
            let scene = rand_obj(t, &mut rng, 0x1_0e00);
            let matrix = rand_obj(t, &mut rng, 0x40);
            let mesh = rng.next();
            t.stub_sym(mh::PHYRE_MALLOC_ALIGNED, 3, false, 2, 0, Ret::Val(0x44));
            t.stub_sym(mh::MESH_MATRIX_PUSH, 4, true, 2, 2, Ret::Val(0x45));
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - 0x80);
            t.check(
                "PhyreScene_PushMeshMatrix",
                0x0065_6780,
                Conv::Thiscall(scene),
                &[mesh, matrix],
                |g| Out::U32(h::push_mesh_matrix(g, ebp, scene, mesh, matrix)),
            );
            t.b_top = None;
            for k in 0..0x11 {
                let (x, y): (u32, u32) = (
                    Mem::read(&t.a, ebp - 0x44 + 4 * k),
                    Mem::read(&t.b, ebp - 0x44 + 4 * k),
                );
                assert_eq!(x, y, "PushMeshMatrix: the frame record");
            }
            t.reset();

            // the dyn-geo swap: no buffers, indexes around the count
            let n = [0, 1, 3, 4][(case % 4) as usize];
            let bufs: Vec<u32> = (0..n.max(1)).map(|_| rand_obj(t, &mut rng, 0x10)).collect();
            let arr = t.words(&bufs);
            let this = t.words(&[0, rng.below(n.max(1)), n, 0, arr, rng.next()]);
            t.import_sym(mh::PRINTF, 5, 1, 0, Ret::Val(9));
            stub_mutex(t);
            t.check(
                "DynGeoMemManager_swapBuffer_006dece0",
                0x006d_ece0,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(h::swap_buffer(g, this)),
            );
            t.reset();

            // the water unload (frame)
            let scene = t.zeros(4);
            t.stub_sym(mh::UNIFY_FILENAME, 6, false, 3, 0, Ret::Val(0));
            t.stub_sym(mh::SCENE_UNLOAD_VFX_TEXTURE, 7, true, 2, 2, Ret::Val(0x66));
            let ebp = frame_ebp(&t.a, 0);
            t.b_top = Some(ebp - 0x140);
            t.check(
                "PhyreScene_unloadWaterEffectTexture",
                0x0067_1b90,
                Conv::Thiscall(scene),
                &[],
                |g| Out::U32(h::unload_water_texture(g, ebp, scene)),
            );
            t.b_top = None;
            t.reset();

            // the texture video callback
            let busy = t.zeros(4);
            let flag = t.words(&[u32::from(case % 3 != 0)]);
            let r = t.words(&[rng.next(), rng.next(), rng.next(), rng.next(), flag, busy]);
            let job = t.words(&[0, r]);
            let s = t.zeros(0xe00);
            t.global(mh::SCENE.sym, s);
            let ok = [0x100, 1, rng.next()][rng.below(3) as usize];
            t.stub_sym(mh::VIDEO_UPDATE_TEXTURE, 8, true, 3, 3, Ret::Val(ok));
            t.stub_sym(mh::OPERATOR_DELETE, 9, false, 1, 0, Ret::Val(0x99));
            t.check(
                "PhyreScene_updateTextureVideoCallback",
                0x0067_2070,
                Conv::Cdecl,
                &[job],
                |g| Out::U32(h::update_texture_video_callback(g, job)),
            );
            t.reset();

            // the word array resizes
            for (sym, alloc) in [
                (0x0066_c690_u32, 0x0064_f7f0_u32),
                (0x0066_c700, 0x0064_f860),
            ] {
                let c = rng.below(5) | if rng.chance(4) { 0x8000_0000 } else { 0 };
                let n = if rng.chance(4) { c } else { rng.below(5) };
                let old = rng.next();
                let arr = t.words(&[c, old]);
                let p = match rng.below(4) {
                    0 => 0,
                    1 => old,
                    _ => rng.next(),
                };
                t.stub_sym(alloc, 10, false, 3, 0, Ret::Val(p));
                t.stub_sym(mh::PHYRE_FREE, 11, false, 1, 0, Ret::Val(0));
                t.check(
                    &format!("PArray_resizeWords_{sym:08x}"),
                    sym,
                    Conv::Thiscall(arr),
                    &[n],
                    |g| Out::U32(h::resize_words(g, arr, n, alloc)),
                );
                t.reset();
            }

            // the instance list owners
            let scene = t.zeros(0xfb60);
            let k = rng.below(4);
            let mut lists = vec![];
            for _ in 0..k {
                if rng.chance(5) {
                    lists.push(0);
                    continue;
                }
                let n = rng.below(4);
                let insts: Vec<u32> = (0..n)
                    .map(|_| {
                        if rng.chance(5) {
                            return 0;
                        }
                        let i = rand_obj(t, &mut rng, 0x20);
                        if rng.chance(4) {
                            t.w32(i + 0x1c, 0);
                        } else {
                            let rec = rand_obj(t, &mut rng, 0x20);
                            t.w32(i + 0x1c, rec);
                        }
                        i
                    })
                    .collect();
                let d = t.words(&insts);
                let l = t.words(&[n, d, 0, 0, 0, 0]);
                t.w8(l + 0x14, (rng.next() & 1) as u8);
                lists.push(l);
            }
            let d = t.words(&lists);
            t.w32(scene + 0xfb48, k);
            t.w32(scene + 0xfb50, d);
            t.check(
                "PhyreScene_setInstanceListOwners_0066c770",
                0x0066_c770,
                Conv::Thiscall(scene),
                &[],
                |g| {
                    h::set_instance_list_owners(g, scene);
                    Out::None
                },
            );
            t.reset();

            // the 0x20 arrays
            for (sym, dtor) in [
                (0x006a_f510_u32, Some(mh::ELEMENT20_DTOR)),
                (0x006a_f5d0, None),
            ] {
                let virt = t.stub_code(12, true, 1, 1, Ret::Val(0), false);
                let n = rng.below(4);
                let mut els = vec![];
                for _ in 0..n {
                    let vt = t.words(&[virt]);
                    els.extend([vt, 0, 0, 0, 0, 0, 0, 0]);
                }
                let data = if n == 0 && rng.chance(2) {
                    0
                } else {
                    t.words(&els)
                };
                let arr = t.words(&[n | if rng.chance(4) { 0x8000_0000 } else { 0 }, data]);
                t.stub_sym(mh::ELEMENT20_DTOR, 13, true, 0, 0, Ret::Val(0));
                t.stub_sym(mh::PHYRE_FREE, 14, false, 1, 0, Ret::Val(0));
                t.check(
                    &format!("release20_{sym:08x}"),
                    sym,
                    Conv::Thiscall(arr),
                    &[],
                    |g| {
                        h::release_array20(g, arr, dtor);
                        Out::None
                    },
                );
                t.reset();
            }

            // the heap sort loop (its second argument slot)
            let n = rng.below(6);
            let fl: Vec<u32> = (0..n).map(|_| float_word(&mut rng)).collect();
            let first = t.words(&fl);
            let last = first + 4 * n;
            t.stub_sym(mh::FLOAT_HEAP_ADJUST, 15, false, 5, 0, Ret::Val(0));
            let pred = rng.next();
            let slot = first_slot(&t.b, 3) + 4;
            t.b_top = Some(slot - 0x44);
            t.check(
                "FloatHeap_sortHeap_006c7890",
                0x006c_7890,
                Conv::Cdecl,
                &[first, last, pred],
                |g| Out::U32(h::sort_heap(g, slot, first, last, pred)),
            );
            t.b_top = None;
            t.reset();

            // the character push (its argument slot)
            let chr = t.zeros(0x10);
            stub_mutex(t);
            t.stub_sym(mh::CHARACTER_TABLE_PUSH, 16, true, 1, 1, Ret::Val(0));
            let inst = rng.next();
            let slot = first_slot(&t.b, 1);
            t.check(
                "ClassCharacter_pushInstance_006a0d00",
                0x006a_0d00,
                Conv::Thiscall(chr),
                &[inst],
                |g| Out::U32(h::push_instance(g, chr, slot)),
            );
            t.reset();

            // the tagged free, the shadow copy, the DLL load
            let this = rand_obj(t, &mut rng, 0xd0);
            t.stub_sym(mh::PHYRE_FREE, 17, false, 1, 0, Ret::Val(0));
            t.check(
                "TaggedBuffer_free_006c7ec0",
                0x006c_7ec0,
                Conv::Thiscall(this),
                &[],
                |g| {
                    h::tagged_free(g, this);
                    Out::None
                },
            );
            let copy = rand_obj(t, &mut rng, 0x1f0);
            t.stub_sym(mh::SHADOW_GET, 18, false, 1, 0, Ret::Val(rng.next()));
            t.stub_sym(mh::SHADOW_COPY, 19, true, 1, 1, Ret::Val(copy));
            let (l, out) = (rng.next(), rng.next());
            t.check(
                "ShadowDynamicMesh_copyOut_00678ba0",
                0x0067_8ba0,
                Conv::Stdcall,
                &[l, out],
                |g| Out::U32(h::shadow_copy_out(g, l, out)),
            );
            let cb = t.stub_code(20, false, 1, 0, Ret::Val(0x77), false);
            let job = rand_obj(t, &mut rng, 0x88);
            t.w32(job + 0x84, cb);
            t.import_sym(mh::LOAD_LIBRARY_A, 21, 1, 1, Ret::Val(0x1000_0000));
            t.check("MagicDLLAsyncLoad", 0x006f_9cc0, Conv::Cdecl, &[job], |g| {
                Out::U32(h::magic_dll_async_load(g, job))
            });
            t.reset();

            // the PS2 stream checks (frame word), the GS regs, the cache dump
            let stream = t.zeros(0x5_0760);
            let fd = [0xffff_fffe, 3, 0][(case % 3) as usize];
            t.w32(stream + 0x5_0544, fd);
            // the ioctl writes 0 or 1 through its third argument, or
            // leaves it (-1 from the original)
            let mut code = vec![0x8b, 0x44, 0x24, 0x0c, 0xc7, 0x00];
            code.extend((case % 2_u32).to_le_bytes());
            code.push(0xc3);
            if case % 5 == 4 {
                code = vec![0xc3];
            }
            let io = t.a.alloc_code(&code).unwrap();
            assert_eq!(t.b.alloc_code(&code).unwrap(), io);
            t.jump(mh::SCE_IOCTL, io);
            let ebp = frame_ebp(&t.a, 1);
            t.b_top = Some(ebp - 0x40);
            t.check(
                "syncStrCheckPC_HDD",
                0x0077_e8e0,
                Conv::Cdecl,
                &[stream],
                |g| Out::U32(h::sync_str_check_hdd(g, ebp, stream)),
            );
            t.b_top = None;
            t.reset();
            let stream = t.zeros(0x5_0760);
            t.w8(stream + 0x5_075c, (case % 2) as u8);
            t.stub_sym(mh::STR_A, 22, false, 1, 0, Ret::Val(0xa));
            t.stub_sym(mh::STR_CHECK_HDD, 23, false, 1, 0, Ret::Val(0xb));
            t.check(
                "syncStrCheck_0077e8b0",
                0x0077_e8b0,
                Conv::Cdecl,
                &[stream],
                |g| Out::U32(h::sync_str_check(g, stream)),
            );
            let (a0, a1, a2) = (rng.next(), rng.next(), rng.word());
            t.check(
                "GsSetFrameRegs_0077e830",
                0x0077_e830,
                Conv::Cdecl,
                &[a0, a1, a2],
                |g| Out::U32(h::gs_set_frame_regs(g, a0, a1, a2)),
            );
            t.stub_sym(mh::DBG_PRINTF, 24, false, 1, 0, Ret::Val(0x31)); // the tail call passes one word
            t.stub_sym(mh::CACHE_DUMP_SLOT, 25, false, 1, 0, Ret::Val(0));
            let tag = rng.next();
            let slot = first_slot(&t.b, 1);
            t.b_top = Some(slot - 0x40);
            t.check("Cache_Dump", 0x0084_0510, Conv::Cdecl, &[tag], |g| {
                Out::U32(h::cache_dump(g, slot, tag))
            });
            t.b_top = None;
            t.reset();
        }
    });
}

// ---- batch 9 (ffx_prender::m5r_i and twins) ----

#[test]
fn batch9_functions_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0801);
        for case in 0..40 {
            // the unique add on +0x3878: found, appended, grown (no data,
            // data, allocation failing)
            let scene = t.zeros(0x3890);
            let n = rng.below(5);
            let vals: Vec<u32> = (0..n).map(|_| rng.below(6)).collect();
            let data = if n == 0 && rng.chance(2) {
                0
            } else {
                t.words(&[vals.clone(), vec![0; 4]].concat())
            };
            t.w32(scene + 0x3878, n);
            t.w32(scene + 0x387c, [n, n + 1, n + 2, 0][rng.below(4) as usize]);
            t.w32(scene + 0x3880, data);
            // (a failed allocation writes through the null block in the
            // original: only with old data, where the count stays small)
            let block = if data != 0 && n == 0 && rng.chance(3) {
                0
            } else {
                t.zeros(0x40)
            };
            t.stub_sym(mi::USER_MALLOC, 1, false, 1, 0, Ret::Val(block));
            t.stub_sym(mi::MEMCPY, 2, false, 3, 0, Ret::Val(0));
            t.stub_sym(mi::USER_FREE, 3, false, 1, 0, Ret::Val(0));
            let v = rng.below(6);
            t.check(
                "PhyreScene_AddVFXToList3880",
                0x0065_65e0,
                Conv::Thiscall(scene),
                &[v],
                |g| Out::U32(i::add_unique(g, scene, 0x3878, v)),
            );
            t.reset();

            // the DTT list: empty, found, appended, grown, null slot
            let scene = t.zeros(0x3870);
            let n = rng.below(5);
            let vals: Vec<u32> = (0..n).map(|_| rng.below(6)).collect();
            let data = if n == 0 && rng.chance(3) {
                0
            } else {
                t.words(&[vals, vec![0; 4]].concat())
            };
            t.w32(scene + 0x3854, n);
            t.w32(scene + 0x3858, [n, n + 1, n + 2][rng.below(3) as usize]);
            t.w32(scene + 0x385c, data);
            t.stub_sym(mi::RESERVE_4, 4, true, 1, 1, Ret::Val(0));
            let v = rng.below(6);
            t.check(
                "PhyreScene_AddVFXToDTTList",
                0x0065_6990,
                Conv::Thiscall(scene),
                &[v],
                |g| Out::U32(i::add_vfx_to_dtt(g, scene, v)),
            );
            t.reset();

            // the tree add twin
            let tree = rand_obj(t, &mut rng, 0x30);
            let node = if case % 4 == 0 {
                0
            } else {
                rand_obj(t, &mut rng, 0x20)
            };
            t.w32(tree + 0x18, node);
            let key = rand_obj(t, &mut rng, 4);
            let value = rand_obj(t, &mut rng, 4);
            t.stub_sym(ffx_glue_other::r3::POOL_REFILL, 5, true, 1, 1, Ret::Val(0));
            t.stub_sym(
                mi::TREE_INSERT_691E60,
                6,
                true,
                1,
                1,
                Ret::Val(rng.below(2)),
            );
            t.stub_sym(ffx_glue_other::r3::POOL_GIVE, 7, true, 1, 1, Ret::Val(0));
            t.stub_sym(ffx_glue_other::r3::TREE_FIXUP, 8, true, 1, 1, Ret::Val(0));
            t.check(
                "TaggedTree_add_00693c50",
                0x0069_3c50,
                Conv::Thiscall(tree),
                &[key, value],
                |g| {
                    Out::U32(ffx_glue_other::r3::tree_add(
                        g,
                        mi::TREE_INSERT_691E60,
                        tree,
                        key,
                        value,
                    ))
                },
            );
            t.reset();

            // the string nodes: names (tagged or not), none, no memory
            let mk = |t: &mut Twins, rng: &mut Rng| {
                let s = t.alloc(b"name_of_a_texture\0\0\0");
                let w = match rng.below(4) {
                    0 => 0,
                    1 => 1,
                    2 => s | 1,
                    _ => s,
                };
                t.words(&[w])
            };
            let (k, v) = (mk(t, &mut rng), mk(t, &mut rng));
            let p = if rng.chance(4) { 0 } else { t.zeros(0x20) };
            t.stub_sym(mi::PHYRE_MALLOC_ALIGNED, 9, false, 2, 0, Ret::Val(p));
            t.stub_sym(mi::MEMCPY, 10, false, 3, 0, Ret::Val(0));
            let node = t.zeros(0x14);
            t.check(
                "StringTreeNode_ctor_0064cba0",
                0x0064_cba0,
                Conv::Thiscall(node),
                &[k],
                |g| Out::U32(i::string_node_ctor(g, node, k)),
            );
            t.stub_sym(mi::STRING_NODE_CTOR, 11, true, 1, 1, Ret::Val(node));
            t.check(
                "StringPairTreeNode_ctor_0064cb00",
                0x0064_cb00,
                Conv::Thiscall(node),
                &[k, v],
                |g| Out::U32(i::string_pair_node_ctor(g, node, k, v)),
            );
            t.reset();

            // the texture animation info dtor, the sampler pairs, the
            // tree twins
            let this = rand_obj(t, &mut rng, 0x10);
            t.stub_sym(mi::PHYRE_FREE, 12, false, 1, 0, Ret::Val(0));
            t.check(
                "TextureAnimationInfo_dtor_006af760",
                0x006a_f760,
                Conv::Thiscall(this),
                &[],
                |g| {
                    i::texanim_info_dtor(g, this);
                    Out::None
                },
            );
            t.stub_sym(mi::SAMPLER_DTOR, 13, true, 0, 0, Ret::Val(0x13));
            for sym in [0x0069_2680, 0x0069_27a0] {
                t.check(
                    &format!("SamplerPair_dtor_{sym:08x}"),
                    sym,
                    Conv::Thiscall(this),
                    &[],
                    |g| Out::U32(i::sampler_pair_dtor(g, this)),
                );
            }
            stub_all(
                t,
                20,
                &[
                    (mb::TREE_ORPHAN_ALL, true, 0, 0),
                    (mb::TREE_CLEAR, true, 0, 0),
                    (mb::FREE_LIST_RESET, true, 0, 0),
                    (mb::FREE_LIST_DTOR, true, 0, 0),
                ],
            );
            t.check(
                "PRedBlackTreePair_dtor_006924e0",
                0x0069_24e0,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(b::tree_pair_dtor(g, this)),
            );
            for sym in [0x0069_2540, 0x0069_25c0] {
                t.check(
                    &format!("PRedBlackTreeFree_dtor_{sym:08x}"),
                    sym,
                    Conv::Thiscall(this),
                    &[],
                    |g| Out::U32(b::tree_free_dtor(g, this)),
                );
            }
            t.reset();

            // the specular textures (frame)
            let this = rand_obj(t, &mut rng, 0x80);
            let (ta, tb) = (
                if rng.chance(3) { 0 } else { rng.next() },
                if rng.chance(3) { 0 } else { rng.next() },
            );
            let found: Vec<u32> = (0..2)
                .map(|_| if rng.chance(3) { 0 } else { rng.next() })
                .collect();
            let r = t.seq(&found);
            t.stub_sym(mi::FIND_PARAM, 14, true, 1, 1, r);
            let holder = rand_obj(t, &mut rng, 8);
            t.w32(this + 0x60, holder);
            t.stub_sym(mi::SET_TEXTURE_CHECKED, 15, true, 2, 2, Ret::Val(0));
            let ebp = frame_ebp(&t.a, 2);
            t.b_top = Some(ebp - 0x40);
            t.check(
                "SpecularMapParams_setTextures_006c5570",
                0x006c_5570,
                Conv::Thiscall(this),
                &[ta, tb],
                |g| {
                    i::set_specular_textures(g, ebp, this, ta, tb);
                    Out::None
                },
            );
            t.b_top = None;
            for k in 0..4 {
                let (x, y): (u32, u32) = (
                    Mem::read(&t.a, ebp - 0x10 + 4 * k),
                    Mem::read(&t.b, ebp - 0x10 + 4 * k),
                );
                // compared when a texture went in (the callee read it)
                let set = (0..t.calls()).any(|e| t.r32(t.log + 4 + 64 * e) == 15);
                if set {
                    assert_eq!(x, y, "setTextures: the record");
                }
            }
            t.reset();

            // the median of nine: below and above 40 floats
            let n = [3, 40, 41, 100, 47][(case % 5) as usize];
            let first = t.zeros(4 * n as usize + 4);
            let (mid, last) = (first + 4 * (n / 2), first + 4 * n);
            t.stub_sym(mi::MEDIAN3, 16, false, 4, 0, Ret::Val(0x16));
            let pred = rng.next();
            let slot = first_slot(&t.b, 4);
            t.b_top = Some(slot - 0x40);
            t.check(
                "FloatSort_Median9_006c7560",
                0x006c_7560,
                Conv::Cdecl,
                &[first, mid, last, pred],
                |g| Out::U32(i::median9(g, slot, first, mid, last, pred)),
            );
            t.b_top = None;
            if n > 0x28 {
                let (x, y): (u32, u32) = (Mem::read(&t.a, slot), Mem::read(&t.b, slot));
                assert_eq!(x, y, "Median9: the argument slot");
            }
            t.reset();

            // the dyn-geo manager: there, made, no memory
            let made = case % 3;
            t.global(
                mi::DYN_GEO_FILL.sym,
                if made == 0 { rng.next() | 1 } else { 0 },
            );
            let m = if made == 1 {
                rand_obj(t, &mut rng, 0x10)
            } else {
                0
            };
            t.stub_sym(mi::OPERATOR_NEW, 17, false, 1, 0, Ret::Val(m));
            t.import_sym(mi::CREATE_MUTEX_W, 18, 3, 3, Ret::Val(0x77));
            // the pool's creation writes +4 (mov dword [ecx+4], 0x1234; ret 4)
            let code = [0xc7, 0x41, 0x04, 0x34, 0x12, 0, 0, 0xc2, 0x04, 0x00];
            let pool = t.a.alloc_code(&code).unwrap();
            assert_eq!(t.b.alloc_code(&code).unwrap(), pool);
            t.jump(mi::CREATE_POOL, pool);
            t.check(
                "DynGeoMemManager_createInstance_006deb90",
                0x006d_eb90,
                Conv::Cdecl,
                &[],
                |g| {
                    i::dyn_geo_create(g);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

// ---- batch 10 (ffx_prender::m5r_j) ----

#[test]
fn batch10_functions_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0901);
        for _ in 0..40 {
            for (sym, flag) in [(0x006a_e450_u32, 0x1000_u32), (0x006a_e0c0, 0x8000)] {
                let cb = t.stub_code(1, false, 1, 0, Ret::Val(0), false);
                let n = rng.below(5);
                let ds: Vec<u32> = (0..n)
                    .map(|_| {
                        if rng.chance(6) {
                            return 0;
                        }
                        let d = rand_obj(t, &mut rng, 0x14);
                        let data = t.zeros(0x10);
                        t.w32(d, data);
                        let f = (rng.next() & !0x9001)
                            | if rng.chance(2) { flag } else { 0 }
                            | u32::from(rng.chance(2));
                        t.w32(d + 0xc, f);
                        t.w32(d + 0x10, cb);
                        d
                    })
                    .collect();
                let data = t.words(&ds);
                let helper = t.words(&[n, data]);
                stub_mutex(t);
                let res: Vec<u32> = (0..8).map(|_| rng.word()).collect();
                let r = t.seq(&res);
                t.stub_sym(mj::SET_DATA_TO_FREE_LIST, 2, true, 1, 1, r);
                t.stub_sym(mj::OPERATOR_DELETE, 3, false, 1, 0, Ret::Val(0));
                // the original compaction runs (the loop re-reads the slot)
                t.import_sym(mj::RELEASE_MUTEX, 4, 1, 1, Ret::Val(0x1234_5601));
                let value = rng.next();
                t.check(
                    &format!("runFlag_{sym:08x}"),
                    sym,
                    Conv::Thiscall(helper),
                    &[value],
                    |g| Out::U32(j::run_with_value(g, helper, flag, value)),
                );
                t.reset();
            }

            // the after-post-process VFX pass
            let scene = rand_obj(t, &mut rng, 0x3890);
            let r = rand_obj(t, &mut rng, 0x140);
            t.w32(scene + 0xd0, r);
            if rng.chance(3) {
                t.w32(scene + 0xdc, 0);
            }
            let n = [0, 1, 3, 0xffff_ffff][rng.below(4) as usize];
            let insts: Vec<u32> = (0..n.min(3))
                .map(|_| {
                    if rng.chance(4) {
                        0
                    } else {
                        rand_obj(t, &mut rng, 0x20)
                    }
                })
                .collect();
            let list = t.words(&[insts, vec![0]].concat());
            t.w32(scene + 0x3888, list);
            t.w32(scene + 0x388c, n);
            t.stub_sym(
                mj::BEGIN_SCENE,
                5,
                true,
                1,
                1,
                Ret::Val(if rng.chance(5) { 3 } else { 0 }),
            );
            t.stub_sym(mj::SET_CAMERA, 6, true, 1, 1, Ret::Val(0));
            t.stub_sym(mj::RENDER_MESH_INSTANCE, 7, true, 4, 4, Ret::Val(0));
            t.stub_sym(mj::END_SCENE, 8, true, 1, 1, Ret::Val(0x88));
            t.check(
                "PhyreScene_RenderVFXAfterPostProcess",
                0x0065_38d0,
                Conv::Thiscall(scene),
                &[],
                |g| Out::U32(j::render_vfx_after_post(g, scene)),
            );
            t.reset();
        }
    });
}

// ---- batch 11 (ffx_prender::m5r_k) ----

#[test]
fn batch11_functions_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0a01);
        for case in 0..24 {
            // the PRX load: async (the file there or not), direct (loaded
            // or not)
            let path = t.alloc(b"magic.dll\0\0\0");
            let avail = u32::from(case % 2 == 0);
            t.stub_sym(mk::ASYNC_AVAILABLE, 1, false, 0, 0, Ret::Val(avail));
            t.stub_sym(
                mk::FILE_PROBE,
                2,
                false,
                1,
                0,
                Ret::Val([0x100, 1, rng.next()][rng.below(3) as usize]),
            );
            t.stub_sym(mk::MEMSET, 3, false, 3, 0, Ret::Val(0));
            t.import_sym(mk::SPRINTF_S, 4, 4, 0, Ret::Val(9));
            t.stub_sym(mk::REGIST_ASYNC_TASK, 5, false, 2, 0, Ret::Val(0));
            let m = if case % 4 == 1 { 0 } else { 0x1000_0000 };
            t.import_sym(mk::LOAD_LIBRARY_A, 6, 1, 1, Ret::Val(m));
            let cb = t.stub_code(7, false, 1, 0, Ret::Val(0x77), false);
            let x = rng.next();
            t.check(
                "PhyreFIOS_LoadsPrx",
                0x006f_9bc0,
                Conv::Cdecl,
                &[path, x, cb],
                |g| Out::U32(k::loads_prx(g, path, cb)),
            );
            t.reset();

            // the auto-test manager's deleting destructor
            let this = rand_obj(t, &mut rng, 0x128);
            if case % 3 == 0 {
                t.w32(this + 0x118, 0);
            }
            t.stub_sym(mk::UNINIT_SAVELOAD, 8, false, 1, 0, Ret::Val(0));
            t.stub_sym(mk::STRING_VECTOR_DESTROY, 9, true, 2, 2, Ret::Val(0));
            t.stub_sym(mk::OPERATOR_DELETE, 10, false, 1, 0, Ret::Val(0));
            let flags = rng.word();
            t.check(
                "AutoTestManager_scalarDeletingDtor_006bcbd0",
                0x006b_cbd0,
                Conv::Thiscall(this),
                &[flags],
                |g| Out::U32(k::auto_test_delete(g, this, flags)),
            );
            t.reset();

            // the debug text panel's constructor
            let this = rand_obj(t, &mut rng, 0x120);
            t.stub_sym(mk::TEXT_UTIL_CTOR, 11, true, 0, 0, Ret::Val(0));
            t.stub_sym(mk::MEMSET, 12, false, 3, 0, Ret::Val(0));
            t.check(
                "DebugTextPanel_ctor_006b53b0",
                0x006b_53b0,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(k::debug_text_panel_ctor(g, this)),
            );
            t.reset();
        }
    });
}

// ---- render-pcui batch 1 (ffx_prender::m5r_l) ----

#[test]
fn rp1_thunks_match_the_original() {
    // Every thunk's target is a recording stub: the diff shows the
    // target was called with the thunk's `ecx` and words, and its `eax`
    // comes back.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0b01);
        for case in 0..8 {
            let this = t.zeros(0x40);
            let scene = t.zeros(0x40);
            t.global(ml::SCENE.sym, scene);
            let r = rng.word();
            t.stub_sym(ml::RADIAL_LINE_DESTROY, 1, true, 0, 0, Ret::Val(r));
            t.check(
                "RadialLine_destroyRadialLine_thunk_006f7b10",
                0x006f_7b10,
                Conv::Thiscall(this),
                &[],
                |g| {
                    Out::U32(lb::forward(
                        g,
                        ml::RADIAL_LINE_DESTROY,
                        host::Conv::Thiscall(this),
                        &[],
                    ))
                },
            );
            t.stub_sym(ml::BROKEN_SCREEN_RELEASE, 2, true, 0, 0, Ret::Val(r ^ 1));
            t.check(
                "BrokenScreenEffect_releaseObjects_thunk_006d3600",
                0x006d_3600,
                Conv::Thiscall(this),
                &[],
                |g| {
                    Out::U32(lb::forward(
                        g,
                        ml::BROKEN_SCREEN_RELEASE,
                        host::Conv::Thiscall(this),
                        &[],
                    ))
                },
            );
            for (k, (sym, target)) in [
                (0x0063_8e20, ml::MS_GET_ENCOUNT_TYPE),
                (0x0063_8e60, ml::MS_BATTLE_CHECK),
                (0x0064_20d0, ml::SEGMENTS_ALL_READY),
            ]
            .into_iter()
            .enumerate()
            {
                let k = k as u32;
                t.stub_sym(target, 3 + k, false, 0, 0, Ret::Val(r.wrapping_add(k)));
                t.check("cdecl thunk", sym, Conv::Cdecl, &[], |g| {
                    Out::U32(lb::forward(g, target, host::Conv::Cdecl, &[]))
                });
            }
            let chr = if case % 3 == 0 { 0 } else { this };
            t.stub_sym(ml::CHARACTER_UNLOAD_TASK, 6, false, 1, 0, Ret::Val(r));
            t.check(
                "graphicCharacterUnloadTask_thunk_0063d9a0",
                0x0063_d9a0,
                Conv::Cdecl,
                &[chr],
                |g| {
                    Out::U32(lb::forward(
                        g,
                        ml::CHARACTER_UNLOAD_TASK,
                        host::Conv::Cdecl,
                        &[chr],
                    ))
                },
            );
            for (k, (sym, target)) in [
                (0x0063_9350, ml::ATTACH_CHR_CLUSTER),
                (0x0064_49c0, ml::SCENE_UPDATE_MAP_YIQ),
            ]
            .into_iter()
            .enumerate()
            {
                t.stub_sym(target, 7 + k as u32, true, 0, 0, Ret::Val(r ^ 0x55));
                t.check("scene thunk", sym, Conv::Cdecl, &[], |g| {
                    Out::U32(lb::scene_forward(g, target))
                });
            }
            assert_eq!(t.calls(), 8);
            t.reset();
        }
    });
}

#[test]
fn rp1_video_accessors_match_the_original() {
    // A synthetic FMV manager from a stubbed GetInstance: the playing and
    // state bytes in every combination, getEndTimeMs and
    // isVideoAtEndFrame stubs returning words with upper bytes set. The
    // snapshots run the game's own manager too (GetInstance original).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0b02);
        for case in 0..24 {
            let m = t.zeros(0x800) + 4 * rng.below(64);
            let playing = [0, 1, 0x80, rng.next() as u8][case % 4];
            let state = [0, 1, 0xff, rng.next() as u8][(case / 4) % 4];
            t.w8(m + ml::FMV_PLAYING, playing);
            t.w8(m + ml::FMV_STATE, state);
            let real = t.snapshot && case >= 20;
            if !real {
                t.stub_sym(ml::FMV_MANAGER, 1, false, 0, 0, Ret::Val(m));
            }
            let r = rng.word();
            t.stub_sym(ml::FMV_END_TIME_MS, 2, true, 0, 0, Ret::Val(r));
            t.check(
                "graphicGetVideoEndTime",
                0x0064_1140,
                Conv::Cdecl,
                &[],
                |g| Out::U32(lb::video_end_time(g)),
            );
            t.stub_sym(
                ml::FMV_AT_END_FRAME,
                3,
                true,
                0,
                0,
                Ret::Val(r ^ 0xffff_ff00),
            );
            t.check(
                "graphicisVideoAtEndFrame",
                0x0064_5e50,
                Conv::Cdecl,
                &[],
                |g| Out::U32(lb::video_at_end_frame(g)),
            );
            t.check(
                "graphicVideoState_00645dc0",
                0x0064_5dc0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(lb::video_state(g)),
            );
            t.check(
                "graphicVideoIsPlaying_00660410",
                0x0066_0410,
                Conv::Cdecl,
                &[],
                |g| Out::U32(lb::video_is_playing(g)),
            );
            t.check(
                "graphicVideoClearPlaying_00643e50",
                0x0064_3e50,
                Conv::Cdecl,
                &[],
                |g| Out::U32(lb::video_clear_playing(g)),
            );
            t.reset();
        }
    });
}

#[test]
fn rp1_app_floats_and_flags_match_the_original() {
    // The application's floats from a stubbed GetApplication (any float
    // but a signalling NaN; the snapshots also read the game's own
    // application), the post-process flag on a synthetic manager and on
    // the game's, and the float swap on distinct and equal cells.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_0b03);
        for case in 0..24 {
            let app = t.zeros(0x100);
            for k in 0..0x40 {
                let v = float_word(&mut rng);
                t.w32(app + 4 * k, v);
            }
            let real = t.snapshot && case >= 20;
            if !real {
                t.stub_sym(ml::GET_APPLICATION, 1, false, 0, 0, Ret::Val(app));
            }
            for (sym, off) in [
                (0x0064_11a0, 0x78),
                (0x0064_1270, 0xd8),
                (0x0064_1280, 0xd4),
                (0x0064_1290, 0xdc),
                (0x0064_12a0, 0xd0),
            ] {
                t.check("app float", sym, Conv::Cdecl, &[], |g| {
                    Out::F32(lb::app_float(g, off))
                });
            }
            if !real {
                let p = t.zeros(0x1_2400);
                let v = rng.next() as u8;
                t.w8(p + ml::POST_FLAG, v);
                t.global(ml::POST_PROCESS.sym, p);
            }
            if ml::POST_PROCESS.get(&t.b) != 0 {
                t.check(
                    "graphicPostProcessSetFlag_00699080",
                    0x0069_9080,
                    Conv::Cdecl,
                    &[],
                    |g| Out::U32(lb::post_set_flag(g)),
                );
            }
            let cells = t.zeros(8);
            let (a, b) = if case % 5 == 0 {
                (cells, cells)
            } else {
                (cells, cells + 4)
            };
            let x = float_word(&mut rng);
            let y = float_word(&mut rng);
            t.w32(cells, x);
            t.w32(cells + 4, y);
            let slot = first_slot(&t.a, 2);
            t.check(
                "FloatSwap_006c7c80",
                0x006c_7c80,
                Conv::Cdecl,
                &[a, b],
                |g| Out::U32(lb::float_swap(g, slot, a, b)),
            );
            assert_eq!(
                Mem::read::<u32>(&t.a, slot),
                Mem::read::<u32>(&t.b, slot),
                "FloatSwap_006c7c80: the parked word"
            );
            t.reset();
        }
    });
}
