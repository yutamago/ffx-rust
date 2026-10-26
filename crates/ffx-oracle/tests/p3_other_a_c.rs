//! `ffx_pother_a`'s second round (`p3_misc`, `p3_tree`, `p3_bind`) against
//! the original: on the image alone with synthetic data, and on a snapshot
//! (`FFX_SNAPSHOT`) with the same data over the snapshot's memory. Results
//! (`eax`, or `ST0` as `f32`) and every byte written must agree. Needs
//! `FFX_GAME_DIR`.
//!
//! Callees outside the list are replaced in both oracles alike by
//! recording stubs where their effect does not matter (each call appends
//! its tag, `ecx` for methods and six stack words to a log in memory, so
//! the diff checks both made the same calls, in the same order, with the
//! same `this`); the rest run as original code in both.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_lines
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pother_a::{p3_misc as misc, p3_tree as tree};

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
    /// A float's bits, no signalling NaN (the emulator keeps them).
    fn float(&mut self) -> u32 {
        match self.below(8) {
            0 => [
                0,
                0x8000_0000,
                0x3f80_0000,
                0x7f80_0000,
                0xff80_0000,
                0x7fc0_0001,
            ][self.below(6) as usize],
            1 => self.next() & 0x807f_ffff, // denormals
            _ => {
                let exp = 100 + self.below(56);
                (self.next() & 0x807f_ffff) | (exp << 23)
            }
        }
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

impl Host for G<'_> {
    fn call(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> u32 {
        ffx_oracle::check_width("Host::call", addr, args.len(), host::max_words(conv, false));
        let conv = match conv {
            host::Conv::Cdecl => Conv::Cdecl,
            host::Conv::Stdcall => Conv::Stdcall,
            host::Conv::Thiscall(t) => Conv::Thiscall(t),
            host::Conv::Fastcall => Conv::Fastcall,
        };
        self.0
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .eax
    }
    fn scratch(&mut self) -> u32 {
        self.0.stack_range().start + 0x100
    }
    fn call_st0_f64(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> f64 {
        let conv = match conv {
            host::Conv::Cdecl => Conv::Cdecl,
            host::Conv::Stdcall => Conv::Stdcall,
            host::Conv::Thiscall(t) => Conv::Thiscall(t),
            host::Conv::Fastcall => Conv::Fastcall,
        };
        let r = self
            .0
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"));
        let v = r.st0_f64().expect("the callee's ST0");
        // pop it, as the caller's `fstp` does
        self.0.call(self.1 + 16, Conv::Cdecl, &[]).unwrap();
        v
    }
    fn call_intrinsic(&mut self, addr: u32, x: f64) -> f32 {
        // the trampoline: `fld qword [esp+4]; call [esp+0xc]; ret`, then
        // at +16 `fstp st(0); ret`
        let b = x.to_bits();
        let r = self
            .0
            .call(self.1, Conv::Cdecl, &[b as u32, (b >> 32) as u32, addr])
            .unwrap();
        let v = r.st0_f32().expect("the intrinsic's ST0");
        self.0.call(self.1 + 16, Conv::Cdecl, &[]).unwrap();
        v
    }
}

/// The intrinsic trampoline (see `G::call_intrinsic`).
const TRAMPOLINE: [u8; 19] = [
    0xdd, 0x44, 0x24, 0x04, 0xff, 0x54, 0x24, 0x0c, 0xc3, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
    0xdd, 0xd8, 0xc3,
];

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
}

struct Twins {
    label: &'static str,
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 8 words (tag, `ecx`, six
    /// stack words).
    log: u32,
    /// The intrinsic trampoline.
    tramp: u32,
    /// Original bytes of each stubbed address.
    saved: Vec<(u32, Vec<u8>)>,
}

/// How a stub returns.
#[derive(Clone, Copy)]
struct Stub {
    /// Stack words it logs.
    n: u8,
    /// Stack words it pops.
    pops: u32,
    /// `eax`.
    ret: u32,
    /// Whether it logs `ecx` (a method).
    this: bool,
}

const fn cdecl(n: u8, ret: u32) -> Stub {
    Stub {
        n,
        pops: 0,
        ret,
        this: false,
    }
}

const fn method(n: u8, ret: u32) -> Stub {
    Stub {
        n,
        pops: n as u32,
        ret,
        this: true,
    }
}

const fn stdcall(n: u8, ret: u32) -> Stub {
    Stub {
        n,
        pops: n as u32,
        ret,
        this: false,
    }
}

impl Twins {
    fn new(label: &'static str, mut a: Oracle, mut b: Oracle) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        a.set_max_instructions(5_000_000);
        b.set_max_instructions(5_000_000);
        let mut t = Self {
            label,
            a,
            b,
            log: 0,
            tramp: 0,
            saved: Vec::new(),
        };
        t.log = t.zeros(0x8000);
        t.tramp = t.code(&TRAMPOLINE);
        t
    }

    /// The image, then the snapshot when `FFX_SNAPSHOT` is set.
    fn all() -> Vec<Self> {
        let Some(dir) = game_dir() else {
            return vec![];
        };
        let exe = dir.join("FFX.exe");
        let mut out = vec![Self::new(
            "image",
            Oracle::load(&exe).unwrap(),
            Oracle::load(&exe).unwrap(),
        )];
        if let Some(snap) = std::env::var_os("FFX_SNAPSHOT") {
            let pages = Pages::read(PathBuf::from(snap)).unwrap();
            out.push(Self::new(
                "snapshot",
                Oracle::from_snapshot(&exe, &pages).unwrap(),
                Oracle::from_snapshot(&exe, &pages).unwrap(),
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

    fn random(&mut self, rng: &mut Rng, n: usize) -> u32 {
        let v: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
        self.alloc(&v)
    }

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    fn r32(&mut self, at: u32) -> u32 {
        Mem::read(&self.a, at)
    }

    fn code(&mut self, c: &[u8]) -> u32 {
        let x = self.a.alloc_code(c).unwrap();
        assert_eq!(self.b.alloc_code(c).unwrap(), x);
        x
    }

    /// A recording stub tagged `tag`; returns its address.
    fn stub_code(&mut self, tag: u32, s: Stub) -> u32 {
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        if s.this {
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        }
        for i in 0..s.n {
            c.extend([0x8b, 0x54, 0x24, 4 + 4 * i]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, 8 + 4 * i]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xb8);
        c.extend(s.ret.to_le_bytes()); // mov eax, ret
        if s.pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((s.pops * 4) as u16).to_le_bytes());
        }
        self.code(&c)
    }

    /// Puts a `jmp` to `target` at the original `sym`.
    fn jump(&mut self, sym: u32, target: u32) {
        let at = self.rebase(sym);
        if !self.saved.iter().any(|(a, _)| *a == at) {
            let old = Oracle::read(&mut self.a, at, 5).unwrap();
            self.saved.push((at, old));
        }
        let mut j = vec![0xe9];
        j.extend(target.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
    }

    /// Replaces the original at `sym` by a recording stub.
    fn stub(&mut self, sym: u32, s: Stub) {
        let c = self.stub_code(sym, s);
        self.jump(sym, c);
    }

    /// Puts back every stubbed original.
    fn unstub_all(&mut self) {
        for (at, bytes) in std::mem::take(&mut self.saved) {
            self.write(at, &bytes);
        }
    }

    /// An object whose vtable's slots `0..n` are recording stubs (tag
    /// `base + slot`, methods taking `words[slot]` words).
    fn object(&mut self, base: u32, words: &[u8]) -> u32 {
        let slots: Vec<u32> = (0u32..)
            .zip(words)
            .map(|(i, &n)| self.stub_code(base + i, method(n, base + i)))
            .collect();
        let vt: Vec<u8> = slots.iter().flat_map(|s| s.to_le_bytes()).collect();
        let vt = self.alloc(&vt);
        let o = self.zeros(0x40);
        self.w32(o, vt);
        o
    }

    fn g(&mut self) -> G<'_> {
        G(&mut self.b, self.tramp)
    }

    /// Runs the original at `sym` on `a` and `port` on `b`; compares
    /// `eax & mask` and every byte written.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        mask: u32,
        port: impl FnOnce(&mut G) -> u32,
    ) {
        let f = self.rebase(sym);
        let ra = self
            .a
            .call(f, conv, args)
            .unwrap_or_else(|e| panic!("{} {what}: original: {e}", self.label));
        let rb = port(&mut self.g());
        assert_eq!(
            ra.eax & mask,
            rb & mask,
            "{} {what}: eax {:#x} vs {:#x}",
            self.label,
            ra.eax,
            rb
        );
        self.same(what);
    }

    /// As [`Twins::check`] for a function returning a float in `ST0`
    /// (compared as `fstp m32` stores it).
    fn check_f32(&mut self, what: &str, sym: u32, args: &[u32], port: impl FnOnce(&mut G) -> f32) {
        let f = self.rebase(sym);
        let ra = self
            .a
            .call(f, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{} {what}: original: {e}", self.label));
        let x = ra.st0_f32().expect("ST0");
        let y = port(&mut self.g());
        assert_eq!(
            x.to_bits(),
            y.to_bits(),
            "{} {what} {args:x?}: {x} vs {y}",
            self.label
        );
        // the original leaves its result on the x87 stack
        self.a.call(self.tramp + 16, Conv::Cdecl, &[]).unwrap();
        self.same(what);
    }

    fn same(&mut self, what: &str) {
        let d = self.a.diff(&mut self.b).unwrap();
        assert!(
            d.is_empty(),
            "{} {what}: memory differs at {} bytes, first {:x?}",
            self.label,
            d.len(),
            &d[..d.len().min(8)]
        );
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    fn calls(&mut self) -> u32 {
        let log = self.log;
        self.r32(log)
    }
}

const ALL: u32 = u32::MAX;
const AL: u32 = 0xff;
const FREE: u32 = ffx_pother_a::PHYRE_FREE;

/// A random binary tree of `n` nodes (0x10 bytes each) under `tree`'s
/// header, with random colour bits; the nodes in key order.
fn build_tree(t: &mut Twins, rng: &mut Rng, tree: u32, n: usize) -> Vec<u32> {
    let header = tree + 0xc;
    let nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x10)).collect();
    // insert in random order by key = index
    let mut order: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        let j = rng.below(i as u32 + 1) as usize;
        order.swap(i, j);
    }
    let mut left = vec![tree; n];
    let mut right = vec![tree; n];
    let mut parent = vec![header; n];
    let mut root: Option<usize> = None;
    for &k in &order {
        match root {
            None => root = Some(k),
            Some(r) => {
                let mut c = r;
                loop {
                    let side = if k < c { &mut left } else { &mut right };
                    if side[c] == tree {
                        side[c] = nodes[k];
                        parent[k] = nodes[c];
                        break;
                    }
                    c = nodes.iter().position(|&x| x == side[c]).unwrap();
                }
            }
        }
    }
    for i in 0..n {
        t.w32(nodes[i], left[i]);
        t.w32(nodes[i] + 4, right[i]);
        let colour = rng.next() & 1;
        t.w32(nodes[i] + 8, parent[i] | colour);
    }
    // the header: its left child is the root; its right is not a node
    t.w32(header, root.map_or(tree, |r| nodes[r]));
    t.w32(header + 4, 0x7777_0000);
    t.w32(header + 8, tree);
    nodes
}

#[test]
fn red_black_tree_matches_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_0001);
        t.stub(tree::POST_REMOVE_FIXUP, method(1, 0xf1f1));
        for round in 0..24 {
            let n = 1 + rng.below(14) as usize;
            let tr = t.zeros(0x20);
            let nodes = build_tree(&mut t, &mut rng, tr, n);
            for &x in &nodes {
                t.check(
                    "successor",
                    0x0057_5160,
                    Conv::Thiscall(tr),
                    &[x],
                    ALL,
                    |g| tree::bridge::successor(g, tr, x),
                );
            }
            if round % 3 == 0 {
                let n0 = t.calls();
                t.check("clear", 0x0057_4760, Conv::Thiscall(tr), &[], 0, |g| {
                    tree::bridge::clear(g, tr);
                    0
                });
                assert!(t.calls() >= n0);
                continue;
            }
            let mut order = nodes.clone();
            for i in (1..order.len()).rev() {
                let j = rng.below(i as u32 + 1) as usize;
                order.swap(i, j);
            }
            for &x in &order {
                t.check("remove", 0x0057_5000, Conv::Thiscall(tr), &[x], ALL, |g| {
                    tree::bridge::remove(g, tr, x)
                });
                // again: now in no tree
                t.check(
                    "remove again",
                    0x0057_5000,
                    Conv::Thiscall(tr),
                    &[x],
                    ALL,
                    |g| tree::bridge::remove(g, tr, x),
                );
            }
        }
        // a null root ends clear at once
        let tr = t.zeros(0x20);
        t.check("clear null", 0x0057_4760, Conv::Thiscall(tr), &[], 0, |g| {
            tree::bridge::clear(g, tr);
            0
        });
        t.unstub_all();
    }
}

#[test]
fn small_functions_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x00c0_ffee);
        t.stub(misc::CONTROLLER_DTOR, method(0, 0x1001));
        t.stub(misc::CONTROLLER_CTOR, method(0, 0x1002));
        t.stub(FREE, cdecl(1, 0x1003));
        t.stub(misc::ATEXIT, cdecl(1, 0x1004));
        t.stub(misc::ELEMENT_DTOR, method(0, 0x1005));
        t.stub(misc::ASYNC_DETACH, method(1, 0x1006));
        t.stub(misc::CRITICAL_SECTION_INIT, method(0, 0x1007));
        t.stub(misc::CRITICAL_SECTION_RELEASE, method(0, 0x1008));
        t.stub(misc::SLEEP, cdecl(1, 0x1009));
        t.stub(misc::REF_RELEASE, method(0, 0x100a));
        t.stub(misc::LIST_CLEAR, method(0, 0x100b));
        t.stub(misc::REGISTER_TYPE, cdecl(1, 0x100c));
        for (i, &(f, _)) in (0u32..).zip(&misc::GAMEPLAY_INIT) {
            t.stub(f, cdecl(0, 0x2000 + i));
        }
        // the IAT slots: printf, IsDebuggerPresent (0 then 1), DebugBreak
        let printf = t.stub_code(0x3001, cdecl(2, 0x3001));
        let no_debugger = t.stub_code(0x3002, stdcall(0, 0));
        let debugger = t.stub_code(0x3003, stdcall(0, 0x3003));
        let debug_break = t.stub_code(0x3004, stdcall(0, 0x3004));
        let iat = [
            misc::IAT_PRINTF,
            misc::IAT_IS_DEBUGGER_PRESENT,
            misc::IAT_DEBUG_BREAK,
        ]
        .map(|s| t.rebase(s));
        t.w32(iat[0], printf);
        t.w32(iat[2], debug_break);

        // fly controller
        for _ in 0..2 {
            let p = t.random(&mut rng, 0x80);
            t.check("fly dtor", 0x005d_6d10, Conv::Thiscall(p), &[], ALL, |g| {
                misc::bridge::fly_controller_dtor(g, p)
            });
            let p = t.random(&mut rng, 0x80);
            t.check("fly ctor", 0x005d_6cc0, Conv::Thiscall(p), &[], ALL, |g| {
                misc::bridge::fly_controller_ctor(g, p)
            });
        }
        // destroy callbacks
        for w in [0, 1, 0x1234_5678, 0x1234_5679] {
            let p = t.random(&mut rng, 0x60);
            t.w32(p, w);
            t.check("free word", 0x005d_3dd0, Conv::Stdcall, &[p], AL, |g| {
                u32::from(misc::bridge::destroy_free_word(g, p))
            });
            let p = t.random(&mut rng, 0x60);
            t.w32(p + 0x54, w);
            t.check("ref free54", 0x0055_af50, Conv::Stdcall, &[p], AL, |g| {
                u32::from(misc::bridge::destroy_ref_free54(g, p))
            });
            let p = t.random(&mut rng, 0x60);
            t.w32(p + 0x10, w);
            t.check(
                "scriptable dtor",
                0x0055_2c60,
                Conv::Thiscall(p),
                &[],
                ALL,
                |g| misc::bridge::scriptable_dtor(g, p),
            );
        }
        let o = t.object(0x4000, &[0, 0, 1]);
        t.check("virtual2", 0x005d_0640, Conv::Stdcall, &[o + 4], AL, |g| {
            u32::from(misc::bridge::destroy_virtual2(g, o + 4))
        });
        // TTY: print at each level with and without a debugger
        let msg = t.random(&mut rng, 8);
        for (level, dbg) in [
            (0, no_debugger),
            (3, debugger),
            (4, no_debugger),
            (4, debugger),
        ] {
            t.w32(iat[1], dbg);
            t.check(
                "tty print",
                0x0043_54f0,
                Conv::Stdcall,
                &[level, msg],
                ALL,
                |g| misc::bridge::tty_print(g, level, msg),
            );
        }
        // dispatch: both flags clear, the object's flag set, both set (a
        // fake object behind the pointer)
        let fake = t.object(0x4100, &[0, 2]);
        for (once, made) in [(0, 0), (0, 1), (2, 3), (1, 0)] {
            let at = t.rebase(misc::TTY_ONCE);
            t.w32(at, once);
            let at = t.rebase(misc::TTY_OBJECT_ONCE);
            t.w32(at, made);
            let at = t.rebase(misc::TTY_POINTER);
            t.w32(at, fake);
            t.check(
                "tty dispatch",
                0x0043_5390,
                Conv::Cdecl,
                &[4, msg],
                ALL,
                |g| misc::bridge::tty_dispatch(g, 4, msg),
            );
        }
        // comparator
        for _ in 0..40 {
            let a = t.random(&mut rng, 0x10);
            let b = t.random(&mut rng, 0x10);
            if rng.below(2) == 0 {
                let w: u32 = t.r32(a);
                let v = t.r32(b);
                t.w32(b, v & 0xffff | w & 0xffff_0000);
            }
            if rng.below(2) == 0 {
                let w = t.r32(a + 8);
                t.w32(b + 8, w);
            }
            let (pa, pb) = (t.zeros(4), t.zeros(4));
            t.w32(pa, a);
            t.w32(pb, b);
            t.check("compare", 0x005a_0820, Conv::Cdecl, &[pa, pb], ALL, |g| {
                misc::bridge::compare(g, pa, pb)
            });
        }
        // drain: a chain of nodes with stamps around the global's
        for round in 0..64u32 {
            let now = rng.next();
            let at = t.rebase(misc::DRAIN_NOW);
            t.w32(at, now);
            let n = 1 + rng.below(5) as usize;
            let nodes: Vec<u32> = (0..n).map(|_| t.zeros(8)).collect();
            for i in 0..n {
                let next = if i + 1 < n { nodes[i + 1] } else { 0 };
                t.w32(nodes[i], next);
                let stamp = match rng.below(4) {
                    0 => 0,
                    1 => now.wrapping_sub(1 + rng.below(100)),
                    2 => now.wrapping_add(rng.below(100)),
                    _ => rng.next(),
                };
                t.w32(nodes[i] + 4, stamp);
            }
            let end = nodes[rng.below(n as u32) as usize];
            let cursor = t.zeros(4);
            t.w32(cursor, nodes[0]);
            if round % 2 == 0 {
                t.check(
                    "drain",
                    0x005c_e630,
                    Conv::Cdecl,
                    &[end, cursor],
                    ALL,
                    |g| misc::bridge::drain(g, end, cursor),
                );
            } else {
                let this = t.random(&mut rng, 0x30);
                t.w32(this + 0x24, end);
                t.w32(this + 0x28, nodes[0]);
                t.check(
                    "drain and clear",
                    0x005d_1880,
                    Conv::Thiscall(this),
                    &[],
                    ALL,
                    |g| misc::bridge::drain_and_clear(g, this),
                );
            }
        }
        // Class_FindMemberByOffset: the finder stub changes the object word
        // (so before and now differ) and returns m or 0
        for (k, m_found) in [(0u32, true), (1, false), (2, true)] {
            let c = t.random(&mut rng, 0x20);
            let a = t.zeros(8);
            t.w32(a + 4, c);
            let b = t.random(&mut rng, 0x10);
            let m = t.zeros(8);
            t.w32(m, a);
            t.w32(m + 4, b);
            let ret = if m_found { m } else { 0 };
            let mut code = vec![0x89, 0x0d];
            code.extend((t.log + 0x7ff0).to_le_bytes()); // mov [log+..], ecx
            code.extend([0x8b, 0x44, 0x24, 0x08]); // mov eax, [esp+8]
            code.extend([0xc7, 0x00]);
            code.extend((0x0100_0000 * k + 0x55).to_le_bytes()); // mov [eax], k
            code.push(0xb8);
            code.extend(ret.to_le_bytes());
            code.extend([0xc2, 0x10, 0x00]);
            let finder = t.code(&code);
            t.jump(misc::FIND_MEMBER_AT, finder);
            let object = t.random(&mut rng, 8);
            let element = t.zeros(4);
            let (this, class, off) = (rng.next(), rng.next(), rng.next());
            t.check(
                "find member by offset",
                0x0047_d5f0,
                Conv::Thiscall(this),
                &[class, object, off, element],
                ALL,
                |g| misc::bridge::find_member_by_offset(g, this, class, object, off, element),
            );
        }
        // constructors
        for _ in 0..2 {
            let p = t.random(&mut rng, 0x60);
            let d = rng.next();
            t.check(
                "scripted ctor",
                0x0055_3980,
                Conv::Thiscall(p),
                &[d],
                ALL,
                |g| misc::bridge::scripted_ctor(g, p, d),
            );
            let p = t.random(&mut rng, 0x60);
            t.check(
                "base ref ctor",
                0x005d_3860,
                Conv::Thiscall(p),
                &[],
                ALL,
                |g| misc::bridge::base_ref_ctor(g, p),
            );
            let p = t.random(&mut rng, 0x60);
            t.check(
                "async ctor",
                0x005c_e4b0,
                Conv::Thiscall(p),
                &[],
                ALL,
                |g| misc::bridge::async_buffer_ctor(g, p),
            );
            let p = t.random(&mut rng, 0x60);
            t.check(
                "locator ctor",
                0x0054_e0f0,
                Conv::Thiscall(p),
                &[],
                ALL,
                |g| {
                    let f = g.scratch();
                    misc::bridge::locator_ctor(g, p, f)
                },
            );
            let p = t.random(&mut rng, 0x60);
            t.check(
                "receiver ctor",
                0x0055_2200,
                Conv::Thiscall(p),
                &[],
                ALL,
                |g| {
                    let f = g.scratch();
                    misc::bridge::receiver_ctor(g, p, f)
                },
            );
            let p = t.random(&mut rng, 0x40);
            let first = t.zeros(8);
            let head = t.rebase(misc::NAMED_SEMANTIC_LIST);
            t.w32(head, first);
            let args = [rng.next(), rng.next(), rng.next(), rng.next(), rng.next()];
            t.check(
                "named semantic",
                0x005a_0200,
                Conv::Thiscall(p),
                &args,
                ALL,
                |g| misc::bridge::named_semantic_ctor(g, p, args),
            );
        }
        // create callbacks: null and a fresh random block each
        for block in [false, true] {
            let mut p = if block { t.random(&mut rng, 0x60) } else { 0 };
            t.check("zeroed", 0x005d_5000, Conv::Stdcall, &[p], ALL, |g| {
                misc::bridge::create_zeroed(g, p)
            });
            if block {
                p = t.random(&mut rng, 0x60);
            }
            t.check("base ref", 0x005d_3d60, Conv::Stdcall, &[p], AL, |g| {
                u32::from(misc::bridge::create_base_ref(g, p))
            });
            if block {
                p = t.random(&mut rng, 0x60);
            }
            t.check("timer", 0x0055_6910, Conv::Stdcall, &[p], AL, |g| {
                u32::from(misc::bridge::create_timer(g, p))
            });
        }
        t.check("gameplay init", 0x0043_35b0, Conv::Cdecl, &[], ALL, |g| {
            misc::bridge::gameplay_initialize(g)
        });
        // the 0x24-byte array: owned with 0..3 elements, not owned, null
        for (count, data) in [
            (0, true),
            (3, true),
            (1, false),
            (0x8000_0002, true),
            (0x4000_0001, false),
            (2, true),
        ] {
            let p = t.zeros(8);
            t.w32(p, count);
            if data {
                let d = t.random(&mut rng, 0x24 * 3);
                t.w32(p + 4, d);
            }
            t.check(
                "array release",
                0x005a_2700,
                Conv::Thiscall(p),
                &[],
                ALL,
                |g| misc::bridge::array_release(g, p),
            );
        }
        // async buffer destructor: nodes owned by it or not, with a batch
        // owner or none
        for round in 0..6u32 {
            let this = t.random(&mut rng, 0x48);
            let other = t.zeros(4);
            let n = rng.below(5);
            let nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x14)).collect();
            for (i, &x) in nodes.iter().enumerate() {
                let next = nodes.get(i + 1).copied().unwrap_or(0);
                t.w32(x + 0xc, next);
                let owner = if rng.below(3) == 0 { other } else { this };
                t.w32(x + 0x10, owner);
            }
            t.w32(this + 0x44, nodes.first().copied().unwrap_or(0));
            t.w32(this + 0x10, if round % 2 == 0 { 0 } else { rng.next() });
            t.check(
                "async dtor",
                0x005c_e550,
                Conv::Thiscall(this),
                &[],
                0,
                |g| {
                    misc::bridge::async_buffer_dtor(g, this);
                    0
                },
            );
        }
        // data member destructor: empty list or not, owned buffer or not
        for (empty, owned) in [(true, false), (false, true), (true, true), (false, false)] {
            let p = t.random(&mut rng, 0x30);
            let owner = t.object(0x4200, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
            t.w32(p + 0xc, owner);
            if empty {
                t.w32(p + 4, p + 4);
            }
            let w = t.r32(p + 0x14);
            t.w32(p + 0x14, if owned { w | 1 } else { w & !1 });
            t.check(
                "data member dtor",
                0x0057_5470,
                Conv::Thiscall(p),
                &[],
                ALL,
                |g| misc::bridge::data_member_dtor(g, p),
            );
        }
        t.unstub_all();
    }
}

#[test]
fn float_functions_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x0f10_a7ed);
        for _ in 0..60 {
            let x = rng.float();
            t.check_f32("cosf", 0x004c_eac0, &[x], |g| misc::bridge::cosf(g, x));
            t.check_f32("sinf", 0x004c_eae0, &[x], |g| misc::bridge::sinf(g, x));
        }
        for i in 0..80u32 {
            let seed = if i < 4 {
                [0, 0x00bc_602f, u32::MAX, 0x8000_0000][i as usize]
            } else {
                rng.next()
            };
            let (lo, hi) = (rng.float(), rng.float());
            t.check_f32("hashed float", 0x005f_0ce0, &[seed, lo, hi], |g| {
                misc::bridge::hashed_float(g, seed, lo, hi)
            });
        }
        for i in 0..400 {
            let v = t.zeros(0x40);
            for k in 0..4 {
                let x = match i {
                    0 => 0,
                    _ if i % 2 == 0 => rng.float(),
                    // full significands near 1, where the f32 roundings show
                    _ => (rng.next() & 0x807f_ffff) | ((120 + rng.below(16)) << 23),
                };
                t.w32(v + 4 * k, x);
            }
            let out = match i % 5 {
                0 => v,
                1 => v + 4,
                2 => v + 8,
                _ => t.random(&mut rng, 0x40),
            };
            t.check("normalize", 0x0057_44a0, Conv::Cdecl, &[out, v], ALL, |g| {
                misc::bridge::normalize4(g, out, v)
            });
            let out = if i % 4 == 0 {
                v
            } else {
                t.random(&mut rng, 0x40)
            };
            t.check(
                "translation",
                0x0057_46b0,
                Conv::Cdecl,
                &[out, v],
                ALL,
                |g| misc::bridge::translation(g, out, v),
            );
        }
    }
}

#[test]
fn arrays_and_components_match_the_original() {
    use ffx_pother_a::p3_misc2 as m2;
    for mut t in Twins::all() {
        let mut rng = Rng(0x0b47_c402);
        t.stub(FREE, cdecl(1, 0x5001));
        t.stub(m2::REF_RELEASE, method(0, 0x5002));
        t.stub(m2::NAMED_SEMANTIC_CTOR, method(5, 0x5003));
        t.stub(m2::ATEXIT, cdecl(1, 0x5004));
        t.stub(m2::ARRAY24_RELEASE, method(0, 0x5005));
        t.stub(m2::SCRIPTABLE_DTOR, method(0, 0x5006));
        t.stub(m2::RELEASE_HANDLE, cdecl(1, 0x5007));
        t.stub(m2::GLOBAL_NAMESPACE, cdecl(0, 0x5008));
        t.stub(m2::CLASS_DESCRIPTOR_INIT6, method(6, 0x5009));
        t.stub(m2::SET_DEFAULT_BUFFER, method(1, 0x500a));
        t.stub(m2::SET_VALIDATION_BUFFER, method(1, 0x500b));
        t.stub(m2::SET_WRITE_MASK_BUFFER, method(1, 0x500c));
        t.stub(m2::FREE_LIST_INIT, method(4, 0x500d));
        t.stub(m2::FREE_LIST_DTOR, method(0, 0x500e));
        t.stub(m2::ASYNC_BUFFER_CTOR, method(0, 0x500f));
        t.stub(m2::ASYNC_BUFFER_DTOR, method(0, 0x5010));
        t.stub(m2::SCHEDULER_SHUTDOWN, method(0, 0x5011));
        t.stub(m2::MOVE_STRINGS, cdecl(3, 0x5012));
        t.stub(m2::STRING_COPY, method(1, 0x5013));
        t.stub(m2::RELEASE_TAGGED_WORDS, cdecl(2, 0x5014));

        // constructors with float defaults
        for _ in 0..2 {
            let p = t.random(&mut rng, 0x80);
            t.check(
                "light defaults",
                0x005a_2520,
                Conv::Thiscall(p),
                &[],
                ALL,
                |g| m2::bridge::light_defaults(g, p),
            );
            let p = t.random(&mut rng, 0x80);
            t.check(
                "lod level defaults",
                0x005a_2630,
                Conv::Thiscall(p),
                &[],
                ALL,
                |g| m2::bridge::lod_level_defaults(g, p),
            );
            let p = t.random(&mut rng, 0x80);
            t.check(
                "attachable ctor",
                0x0055_9530,
                Conv::Thiscall(p),
                &[],
                ALL,
                |g| m2::bridge::attachable_ctor(g, p),
            );
            let p = t.random(&mut rng, 0xa0);
            t.check(
                "scheduler descriptor",
                0x005c_efd0,
                Conv::Thiscall(p),
                &[],
                ALL,
                |g| m2::bridge::scheduler_descriptor_init(g, p),
            );
            let p = t.random(&mut rng, 0x290);
            t.check(
                "scheduler ctor",
                0x005c_f1e0,
                Conv::Thiscall(p),
                &[],
                ALL,
                |g| m2::bridge::scheduler_ctor(g, p),
            );
        }
        // plane bullet scalar deleting destructors: with and without the
        // Bullet object, both flags
        for (obj, flags) in [(true, 0), (true, 1), (false, 1), (false, 0)] {
            for sym in [0x005e_6d50, 0x005e_6e00] {
                let p = t.random(&mut rng, 0x70);
                let o = if obj { t.object(0x5100, &[1]) } else { 0 };
                t.w32(p + 0x6c, o);
                t.check("plane delete", sym, Conv::Thiscall(p), &[flags], ALL, |g| {
                    m2::bridge::plane_bullet_delete(g, p, flags)
                });
            }
        }
        // callbacks: null and blocks, tagged words or not
        let blocks = [
            0,
            t.random(&mut rng, 0x40),
            t.random(&mut rng, 0x40),
            t.random(&mut rng, 0x40),
        ];
        for (k, p) in blocks.into_iter().enumerate() {
            t.check(
                "create callback data",
                0x0055_1da0,
                Conv::Stdcall,
                &[p],
                AL,
                |g| u32::from(m2::bridge::create_receiver_data(g, p)),
            );
            let p = if p == 0 { 0 } else { t.random(&mut rng, 0x40) };
            t.check(
                "create scripted",
                0x0055_4600,
                Conv::Stdcall,
                &[p],
                AL,
                |g| u32::from(m2::bridge::create_scripted(g, p)),
            );
            if p != 0 {
                let q = t.random(&mut rng, 0x40);
                let w = t.r32(q + 0x20);
                t.w32(q + 0x20, if k == 1 { w | 1 } else { w & !1 });
                let w = t.r32(q + 0x14);
                t.w32(q + 0x14, if k == 2 { w | 1 } else { w & !1 });
                t.check(
                    "destroy callback data",
                    0x0055_1e20,
                    Conv::Stdcall,
                    &[q],
                    AL,
                    |g| u32::from(m2::bridge::destroy_receiver_data(g, q)),
                );
            }
        }
        // the semantic getters: first call and later ones
        for sym in [0x0054_4d70, 0x0054_4e00, 0x0054_ce90] {
            let s = if sym == 0x0054_ce90 {
                m2::TRIGGER_SEMANTIC
            } else {
                m2::LOCATOR_SHAPE_SEMANTIC
            };
            for flags in [0, 0x10, 1, 3] {
                let at = t.rebase(s.once);
                t.w32(at, flags);
                t.check("semantic getter", sym, Conv::Cdecl, &[], ALL, |g| {
                    m2::bridge::semantic_getter(g, &s)
                });
            }
        }
        // push into a new buffer: 4- and 8-byte elements, and strings
        for round in 0..24u32 {
            let words = 1 + round % 2;
            let sym = if words == 1 { 0x0054_bc40 } else { 0x005d_6a60 };
            let data = if rng.below(4) == 0 {
                0
            } else {
                t.random(&mut rng, 0x40)
            };
            // a null block only with no elements to copy
            let n = if data == 0 { 0 } else { rng.below(5) };
            let flag = if rng.below(3) == 0 { 0x8000_0000 } else { 0 };
            let buffer = match round % 6 {
                0 => 0,
                1 => data,
                _ => t.random(&mut rng, 0x40),
            };
            let arr = t.zeros(8);
            t.w32(arr, n | flag);
            t.w32(arr + 4, data);
            let value = t.random(&mut rng, 8);
            t.check(
                "push into",
                sym,
                Conv::Thiscall(arr),
                &[buffer, value],
                ALL,
                |g| m2::bridge::push_into(g, arr, buffer, value, words),
            );
            let arr = t.zeros(8);
            t.w32(arr, n | flag);
            t.w32(arr + 4, data);
            let s = t.random(&mut rng, 8);
            t.check(
                "push string",
                0x005d_6b00,
                Conv::Thiscall(arr),
                &[buffer, s],
                ALL,
                |g| m2::bridge::push_string(g, arr, buffer, s),
            );
        }
        // resize with inline storage: same word, shrink/grow, failed malloc
        for round in 0..30u32 {
            let arr = t.random(&mut rng, 0x10);
            let c = rng.below(4);
            let flag = if rng.below(3) == 0 { 0x8000_0000 } else { 0 };
            t.w32(arr, c | flag);
            let n = match round % 5 {
                0 => c | flag,
                1 => 0,
                2 => 1,
                _ => rng.below(5),
            };
            let block = if round % 7 == 3 {
                0
            } else {
                t.random(&mut rng, 0x20)
            };
            let m = t.stub_code(m2::MALLOC, cdecl(2, block));
            t.jump(m2::MALLOC, m);
            t.check(
                "resize inline",
                0x0048_c3b0,
                Conv::Thiscall(arr),
                &[n],
                ALL,
                |g| m2::bridge::resize_inline(g, arr, n),
            );
        }
        // array releases
        let arrays = |t: &mut Twins, rng: &mut Rng, at: u32| {
            let count = match rng.below(6) {
                0 => 0,
                1 => 1,
                2 => 0x8000_0003,
                3 => 0x4000_0002,
                _ => 2 + rng.below(3),
            };
            t.w32(at, count);
            let data = if rng.below(4) == 0 { 0 } else { rng.next() };
            t.w32(at + 4, data);
        };
        for _ in 0..12 {
            let p = t.random(&mut rng, 0x60);
            arrays(&mut t, &mut rng, p + 0x20);
            t.check(
                "destroy lod group",
                0x005a_6e30,
                Conv::Stdcall,
                &[p],
                AL,
                |g| u32::from(m2::bridge::destroy_lod_group(g, p)),
            );
            let p = t.random(&mut rng, 0x24);
            arrays(&mut t, &mut rng, p + 0x14);
            arrays(&mut t, &mut rng, p + 4);
            t.check(
                "lod element dtor",
                0x005a_29d0,
                Conv::Thiscall(p),
                &[],
                0,
                |g| {
                    m2::bridge::lod_element_dtor(g, p);
                    0
                },
            );
            let p = t.random(&mut rng, 0x30);
            for off in [0x24, 0xc, 4] {
                arrays(&mut t, &mut rng, p + off);
            }
            t.check("spline dtor", 0x0054_7580, Conv::Thiscall(p), &[], 0, |g| {
                m2::bridge::spline_dtor(g, p);
                0
            });
            let n = rng.below(4);
            let data = if rng.below(4) == 0 {
                0
            } else {
                t.random(&mut rng, 0x14 * 4)
            };
            if data != 0 {
                for i in 0..n {
                    arrays(&mut t, &mut rng, data + 0x14 * i);
                    arrays(&mut t, &mut rng, data + 0x14 * i + 8);
                }
            }
            t.check(
                "free pair array",
                0x005c_fa60,
                Conv::Cdecl,
                &[data, n],
                0,
                |g| {
                    m2::bridge::free_pair_array(g, data, n);
                    0
                },
            );
            let p = t.random(&mut rng, 0x40);
            arrays(&mut t, &mut rng, p + 0x34);
            let pairs = t.random(&mut rng, 0x14 * 3);
            for i in 0..3 {
                arrays(&mut t, &mut rng, pairs + 0x14 * i);
                arrays(&mut t, &mut rng, pairs + 0x14 * i + 8);
            }
            let w = rng.below(4) | if rng.below(3) == 0 { 0x8000_0000 } else { 0 };
            t.w32(p + 0x28, w);
            t.w32(p + 0x2c, pairs);
            t.check(
                "scheduler dtor",
                0x005c_f420,
                Conv::Thiscall(p),
                &[],
                ALL,
                |g| m2::bridge::scheduler_dtor(g, p),
            );
        }
        for (b, handle) in [(0u8, true), (1, true), (0x80, false), (0, false)] {
            let p = t.random(&mut rng, 0x30);
            t.write(p + 0x2d, &[b]);
            if !handle {
                t.w32(p + 0x1c, 0);
            }
            t.check(
                "scripted dtor",
                0x0055_3a70,
                Conv::Thiscall(p),
                &[],
                ALL,
                |g| m2::bridge::scripted_dtor(g, p),
            );
        }
        // resize: one inline element grown into a block, owned and not
        for flag in [0, 0x8000_0000] {
            for n in [2, 3] {
                let arr = t.random(&mut rng, 0x10);
                t.w32(arr, 1 | flag);
                let block = t.random(&mut rng, 0x20);
                let m = t.stub_code(m2::MALLOC, cdecl(2, block));
                t.jump(m2::MALLOC, m);
                t.check(
                    "resize inline 1",
                    0x0048_c3b0,
                    Conv::Thiscall(arr),
                    &[n],
                    ALL,
                    |g| m2::bridge::resize_inline(g, arr, n),
                );
            }
        }
        t.unstub_all();
    }
}

#[test]
fn binds_match_the_original() {
    use ffx_pother_a::p3_bind::{self as b, Arg, Step, Ty};
    let tested =
        |f: u32| b::BINDS.iter().any(|(a, _)| *a == f) || b::GETTERS.iter().any(|(a, _)| *a == f);
    let all: Vec<(u32, &[Step], bool)> = b::BINDS
        .iter()
        .map(|&(a, s)| (a, s, false))
        .chain(b::GETTERS.iter().map(|&(a, s)| (a, s, true)))
        .collect();
    for mut t in Twins::all() {
        let mut rng = Rng(0x0b1d_0003);
        t.stub(b::REGISTER_TYPE, cdecl(1, 0x6001));
        t.stub(b::ADD_TO_NAMESPACE, method(0, 0x6002));
        t.stub(b::ATEXIT, cdecl(1, 0x6003));
        t.stub(b::DATA, method(6, 0x6004));
        t.stub(b::DATA_FIXED, method(6, 0x6005));
        t.stub(b::DATA_ARRAY, method(7, 0x6006));
        t.stub(b::CLASS_MEMBER, method(3, 0x6007));
        t.stub(b::DATA_WRAPPED, method(6, 0x6009));
        t.stub(b::GLOBAL_NAMESPACE, cdecl(0, 0x600a));
        t.stub(b::CLASS_DESCRIPTOR_INIT, method(4, 0x600b));
        // every getter and callee the tables call, each its own tag; the
        // tables' own originals (a Bind calling an array getter, a wrapper
        // getter) are tested themselves and run as original code in both
        let mut callees: Vec<(u32, Stub)> = Vec::new();
        for (_, steps, _) in &all {
            for s in *steps {
                match *s {
                    Step::Member { ty: Ty::Get(f), .. } => {
                        callees.push((f, cdecl(0, f ^ 0x00ff_0000)));
                    }
                    Step::Call { f, this, args } => {
                        let n = args.len() as u8;
                        callees.push((
                            f,
                            Stub {
                                n: n.min(6),
                                pops: if this.is_some() { u32::from(n) } else { 0 },
                                ret: f ^ 0x00ee_0000,
                                this: this.is_some(),
                            },
                        ));
                        for a in args {
                            match *a {
                                Arg::Get(g) => callees.push((g, cdecl(0, g ^ 0x00ff_0000))),
                                Arg::Get1(g, _) => callees.push((g, cdecl(1, g ^ 0x00dd_0000))),
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        callees.sort_unstable_by_key(|c| c.0);
        callees.dedup_by_key(|c| c.0);
        for (f, s) in callees {
            if !tested(f) && f != b::DATA && f != b::DATA_ARRAY {
                t.stub(f, s);
            }
        }
        for (k, &(addr, steps, getter)) in all.iter().enumerate() {
            for round in 0..6u32 {
                // the finders: a namesake found or not
                let found = u32::from(round % 2 == 1);
                for f in [b::FIND_FUNCTION.finder, b::FIND_METHOD.finder] {
                    t.stub(f, method(1, found * 0x6008));
                }
                // the guard: all clear, all set, mixed
                for s in steps {
                    if let Step::Guard(g) = s {
                        let w = match round {
                            0 | 1 | 4 => 0,
                            2 => u32::MAX,
                            _ => rng.next(),
                        };
                        let g = t.rebase(*g);
                        t.w32(g, w);
                    }
                }
                // what the stores overwrite and the words read: random
                for s in steps {
                    match *s {
                        Step::Zero(a)
                        | Step::Or(a, _)
                        | Step::And(a, _)
                        | Step::Store(a, _)
                        | Step::StoreImm(a, _)
                        | Step::StoreByte(a, _) => {
                            let a = t.rebase(a);
                            let v = rng.next();
                            t.w32(a, v);
                        }
                        Step::Call { args, .. } => {
                            for a in args {
                                if let Arg::Mem(m) = *a {
                                    let m = t.rebase(m);
                                    let v = rng.next();
                                    t.w32(m, v);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                // the wrappers' done bytes: clear but in round 2
                for s in steps {
                    if let Step::IfZeroByte(a) = s {
                        let a = t.rebase(*a);
                        t.write(a, &[u8::from(round == 2)]);
                    }
                }
                // the getters' owner words: free or taken, a cached owner
                // or none; the annotation lists: empty (pointing at
                // themselves) or not
                for s in steps {
                    match *s {
                        Step::Owner { slot, cache } => {
                            let (slot, cache) = (t.rebase(slot), t.rebase(cache));
                            let taken = if round == 3 { rng.next() | 1 } else { 0 };
                            let cached = if round % 2 == 0 { 0 } else { rng.next() | 1 };
                            t.w32(slot, taken);
                            t.w32(cache, cached);
                        }
                        Step::IfSelf(a) => {
                            let a = t.rebase(a);
                            let v = if round == 5 { rng.next() } else { a };
                            t.w32(a, v);
                        }
                        _ => {}
                    }
                }
                // method members: an owner with an empty list, a name or none
                for s in steps {
                    if let Step::Method { member, find, .. } = s {
                        let m = t.rebase(*member);
                        let owner = t.zeros(0x60);
                        let sink = t.zeros(8);
                        t.w32(owner + find.list, sink);
                        t.w32(owner + find.list + 4, sink);
                        t.w32(m + 8, owner);
                        let name = if (round + k as u32) % 3 == 2 {
                            0
                        } else {
                            rng.next() | 1
                        };
                        t.w32(m + 0xc, name);
                    }
                }
                // a fresh call log (it holds 1024 calls)
                let log = t.log;
                t.w32(log, 0);
                if getter {
                    // the owner: none (round 4) or a word
                    let owner = if round == 4 { 0 } else { rng.next() | 1 };
                    t.check(
                        &format!("getter {addr:#x} round {round}"),
                        addr,
                        Conv::Cdecl,
                        &[owner],
                        ALL,
                        |g| b::bridge::getter(g, steps, owner),
                    );
                } else {
                    t.check(
                        &format!("bind {addr:#x} round {round}"),
                        addr,
                        Conv::Cdecl,
                        &[],
                        ALL,
                        |g| b::bridge::bind(g, steps),
                    );
                }
            }
        }
        t.unstub_all();
    }
}

impl Twins {
    /// A cdecl `(x)` stub tagged `tag` that logs `x` and returns 0x12 when
    /// `x == fail`, else 0.
    fn fail_at(&mut self, tag: u32, fail: u32) -> u32 {
        let log = self.log;
        let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
        c.extend([0x8b, 0x0d]);
        c.extend(log.to_le_bytes()); // mov ecx, [log]
        c.extend([0xc1, 0xe1, 0x05]); // shl ecx, 5
        c.extend([0x81, 0xc1]);
        c.extend((log + 4).to_le_bytes()); // add ecx, log + 4
        c.extend([0xc7, 0x01]);
        c.extend(tag.to_le_bytes()); // mov [ecx], tag
        c.extend([0x89, 0x41, 0x08]); // mov [ecx+8], eax
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0x3d);
        c.extend(fail.to_le_bytes()); // cmp eax, fail
        c.extend([0x75, 0x06]); // jne zero
        c.extend([0xb8, 0x12, 0, 0, 0, 0xc3]); // mov eax, 0x12; ret
        c.extend([0x31, 0xc0, 0xc3]); // zero: xor eax, eax; ret
        self.code(&c)
    }
}

#[test]
fn init_functions_match_the_original() {
    use ffx_pother_a::p3_init::{self as i, Init};
    for mut t in Twins::all() {
        let mut rng = Rng(0x0001_d0d3);
        // the enum bind descriptors: guard clear or set, named or not, a
        // namesake found or not
        t.stub(i::ENUM_INIT, method(1, 0x7001));
        t.stub(i::ENUM_SET_TOKENS, method(2, 0x7002));
        for (addr, e) in [
            (0x005f_09a0, i::COLLISION_GROUP),
            (0x005f_0a40, i::RIGID_BODY_TYPE),
        ] {
            for round in 0..8u32 {
                let guard = t.rebase(e.guard);
                let d = t.rebase(e.descriptor);
                let list = t.rebase(i::ENUM_LIST);
                let g = if round & 1 == 0 {
                    rng.next() & !1
                } else {
                    rng.next() | 1
                };
                t.w32(guard, g);
                let name = if round & 2 == 0 { 0 } else { rng.next() | 1 };
                t.w32(d, name);
                let next = rng.next();
                t.w32(d + 4, next);
                let head = rng.next();
                t.w32(list, head);
                let found = if round & 4 == 0 { 0 } else { 0x7003 };
                t.stub(i::FIND_NAMED, cdecl(1, found));
                let log = t.log;
                t.w32(log, 0);
                t.check(
                    &format!("enum {addr:#x} round {round}"),
                    addr,
                    Conv::Cdecl,
                    &[],
                    ALL,
                    |g| i::bridge::enum_bind(g, &e),
                );
            }
        }
        // the bind lists: every entry registered, or the k-th failing
        for (addr, l) in [
            (0x0043_3020, i::ANIMATION_BINDS),
            (0x0050_3ee0, i::DYNAMIC_GEOMETRY_BINDS),
        ] {
            for c in l.calls {
                match *c {
                    Init::Call(f) | Init::RegisterGot(f) => {
                        t.stub(f, cdecl(0, f ^ 0x00ee_0000));
                    }
                }
            }
            t.stub(i::REGISTER_TYPE, cdecl(1, 0x7004));
            if let Some(f) = l.tail {
                t.stub(f, cdecl(0, 0x7005));
            }
            let n = l.entries.len();
            for k in [0, 1, n / 2, n - 1, n] {
                let fail = l.entries.get(k).map_or(0, |&e| t.rebase(e));
                let s = t.fail_at(l.register, fail);
                t.jump(l.register, s);
                let log = t.log;
                t.w32(log, 0);
                t.check(
                    &format!("bind list {addr:#x} failing at {k}"),
                    addr,
                    Conv::Cdecl,
                    &[],
                    ALL,
                    |g| i::bridge::bind_list(g, &l),
                );
            }
        }
        // the scripting initialisation: the Lua array owned or not, with
        // 0..3 elements; PhyreMalloc's block always the same
        for f in i::SCRIPTING_BINDS {
            t.stub(f, cdecl(0, f ^ 0x00ee_0000));
        }
        t.stub(FREE, cdecl(1, 0x7006));
        for round in 0..6u32 {
            let block = t.random(&mut rng, 0x80);
            t.stub(i::PHYRE_MALLOC, cdecl(2, block));
            let count = round % 4;
            let data = t.random(&mut rng, 0x20);
            let flag = if round >= 4 { 0x8000_0000 } else { 0 };
            let libs = t.rebase(i::LUA_LIBS);
            t.w32(libs, count | flag);
            t.w32(libs + 4, data);
            let log = t.log;
            t.w32(log, 0);
            t.check(
                &format!("scripting round {round}"),
                0x005d_6960,
                Conv::Cdecl,
                &[],
                ALL,
                |g| {
                    let f = g.scratch();
                    i::bridge::scripting_initialize(g, f)
                },
            );
        }
        t.unstub_all();
    }
}

impl Twins {
    /// An object (0x60 zero bytes) whose vtable's slots are recording
    /// method stubs `(words, eax)`, tagged `tag + slot`.
    fn vobj(&mut self, tag: u32, slots: &[(u8, u32)]) -> u32 {
        let code: Vec<u32> = (0u32..)
            .zip(slots)
            .map(|(i, &(n, ret))| self.stub_code(tag + i, method(n, ret)))
            .collect();
        let vt: Vec<u8> = code.iter().flat_map(|s| s.to_le_bytes()).collect();
        let vt = self.alloc(&vt);
        let o = self.zeros(0x60);
        self.w32(o, vt);
        o
    }

    /// A stdcall `(p)` stub tagged `tag` that logs `p`, stores the qword
    /// `v` at `p` and returns `ret`.
    fn qword_stub(&mut self, tag: u32, v: u64, ret: u32) -> u32 {
        let log = self.log;
        let mut c = vec![0x8b, 0x0d];
        c.extend(log.to_le_bytes()); // mov ecx, [log]
        c.extend([0xc1, 0xe1, 0x05]); // shl ecx, 5
        c.extend([0x81, 0xc1]);
        c.extend((log + 4).to_le_bytes()); // add ecx, log + 4
        c.extend([0xc7, 0x01]);
        c.extend(tag.to_le_bytes()); // mov [ecx], tag
        c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
        c.extend([0x89, 0x51, 0x08]); // mov [ecx+8], edx
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.extend([0xc7, 0x02]);
        c.extend((v as u32).to_le_bytes()); // mov [edx], lo
        c.extend([0xc7, 0x42, 0x04]);
        c.extend(((v >> 32) as u32).to_le_bytes()); // mov [edx+4], hi
        c.push(0xb8);
        c.extend(ret.to_le_bytes()); // mov eax, ret
        c.extend([0xc2, 0x04, 0x00]); // ret 4
        self.code(&c)
    }

    /// Points the IAT slot `iat` at `f` in both oracles.
    fn import(&mut self, iat: u32, f: u32) {
        let at = self.rebase(iat);
        self.w32(at, f);
    }
}

#[test]
fn startup_matches_the_original() {
    use ffx_pother_a::p3_misc3::{self as m};
    for mut t in Twins::all() {
        let mut rng = Rng(0x7173_0002);
        // the Win32 timer: the APIs working or not, counters near the
        // 32-bit millisecond wrap and the unsigned fix-ups
        for round in 0..16u32 {
            let period = if round & 1 == 0 { 0 } else { 0x61 };
            let s = t.stub_code(0x7101, stdcall(1, period));
            t.import(m::IAT_TIME_BEGIN_PERIOD, s);
            let s = t.stub_code(0x7102, stdcall(1, 0));
            t.import(m::IAT_SLEEP, s);
            let ticks = if round & 4 == 0 {
                rng.next() >> 1
            } else {
                rng.next() | 0x8000_0000
            };
            let s = t.stub_code(0x7103, stdcall(0, ticks));
            t.import(m::IAT_TIME_GET_TIME, s);
            let freq = [10_000_000u64, 3_579_545, 1 + u64::from(rng.next()), 1][round as usize % 4];
            let ok = [1, 0, 2][round as usize % 3];
            // a counter that fails without writing (the frequency stays 1)
            let s = if round % 5 == 4 {
                t.stub_code(0x7104, stdcall(1, 0))
            } else {
                t.qword_stub(0x7104, freq, ok)
            };
            t.import(m::IAT_QUERY_PERFORMANCE_FREQUENCY, s);
            // counters: small, past the 32-bit milliseconds, negative
            let counter = match round % 4 {
                0 => u64::from(rng.next()),
                1 => (u64::from(rng.next()) << 30) | u64::from(rng.next()),
                2 => (u64::from(rng.next() | 0x8000_0000) << 32) | u64::from(rng.next()),
                _ => (u64::from(rng.next() & 0x3fff_ffff) << 32) | u64::from(rng.next()),
            };
            let s = if round % 7 == 6 {
                t.stub_code(0x7105, stdcall(1, 0))
            } else {
                t.qword_stub(0x7105, counter, 1)
            };
            t.import(m::IAT_QUERY_PERFORMANCE_COUNTER, s);
            // what the timer overwrites: random words
            for a in [
                m::TIMER_START_MS,
                m::TIMER_PRECISE_TIME,
                m::TIMER_START_COMBINED,
                m::TIMER_START_SECONDS,
                m::TIMER_ZERO_A,
                m::TIMER_COUNTER,
                m::TIMER_FREQUENCY,
                m::TIMER_PERIOD,
                m::TIMER_ZERO_B,
                m::TIMER_ZERO_C,
            ] {
                let a = t.rebase(a);
                let (x, y) = (rng.next(), rng.next());
                t.w32(a, x);
                t.w32(a + 4, y);
            }
            let s = t.stub_code(0x7106, stdcall(1, 0x7777_0000));
            t.import(m::IAT_GET_MODULE_HANDLE_A, s);
            let s = t.stub_code(0x7107, stdcall(2, rng.next()));
            t.import(m::IAT_GET_PROC_ADDRESS, s);
            let log = t.log;
            t.w32(log, 0);
            t.check(
                &format!("timer init round {round}"),
                m::TIMER_INITIALIZE,
                Conv::Cdecl,
                &[],
                ALL,
                |g| m::bridge::timer_initialize(g),
            );
        }
        // PhyreInit: done, or failing at each step, or all through
        for f in m::INIT_BINDS {
            t.stub(f, cdecl(0, f ^ 0x00ee_0000));
        }
        for f in m::INIT_TYPES {
            t.stub(f, cdecl(0, f ^ 0x00dd_0000));
        }
        for f in [
            m::REGISTER_TYPE,
            m::THREAD_POOL_SET_STACK_SIZE,
            m::STORE_GLOBAL_C0B958,
            m::STORE_GLOBAL_C0B95C,
            m::ATEXIT,
        ] {
            t.stub(f, cdecl(1, f ^ 0x00cc_0000));
        }
        for f in [
            m::REGISTER_DEFAULT_TYPES,
            m::PBASE_GET_CLASS_DESCRIPTOR,
            m::NAMESPACE_INIT_GLOBAL,
        ] {
            t.stub(f, cdecl(0, f ^ 0x00bb_0000));
        }
        // the start time: `fld qword [x]; ret`
        let x = t.alloc(&1_234.567_891_234_5_f64.to_bits().to_le_bytes());
        let mut c = vec![0xdd, 0x05];
        c.extend(x.to_le_bytes());
        c.push(0xc3);
        let get_time = t.code(&c);
        t.jump(m::TIMER_GET_TIME, get_time);
        for fail in 0..9u32 {
            let r = |k: u32| if fail == k { 0x7300 + k } else { 0 };
            let done = t.rebase(m::PHYRE_INITIALIZED);
            t.write(done, &[u8::from(fail == 7)]);
            t.stub(m::MEMORY_ALLOCATOR_INIT, cdecl(0, r(0)));
            let second = t.vobj(0x7310, &[(0, 0), (0, r(2))]);
            let first = t.vobj(0x7320, &[(0, 0), (0, r(1))]);
            t.w32(first + 4, second);
            let list = t.rebase(m::INIT_LIST);
            t.w32(list, first);
            t.stub(m::SHADER_PASS_INIT_ALL, cdecl(0, r(3)));
            t.stub(m::TIMER_INITIALIZE, cdecl(0, r(4)));
            let pool = t.vobj(0x7330, &[(0, 0), (0, 0), (0, r(5))]);
            t.stub(m::THREAD_POOL_GET_SINGLETON, cdecl(0, pool));
            t.stub(m::SORT_CLASS_DESCRIPTORS, cdecl(0, r(6)));
            let log = t.log;
            t.w32(log, 0);
            t.check(
                &format!("PhyreInit failing at {fail}"),
                0x0043_0dd0,
                Conv::Cdecl,
                &[],
                ALL,
                |g| m::bridge::phyre_init(g),
            );
        }
        t.unstub_all();
    }
}

#[test]
fn printf_matches_the_original() {
    use ffx_pother_a::p3_misc::{TTY_OBJECT, TTY_OBJECT_ONCE, TTY_ONCE, TTY_POINTER, TTY_VFTABLE};
    use ffx_pother_a::p3_misc3::{self as m};
    for mut t in Twins::all() {
        let mut rng = Rng(0x0070_0f01);
        let log = t.log;
        // `_vsnprintf_s(buf, size, count, format, va)`: writes the
        // format's first word, the first two variable arguments, size
        // and count into the buffer; returns 0x55
        let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
        c.extend([0x8b, 0x4c, 0x24, 0x10]); // mov ecx, [esp+0x10]
        c.extend([0x8b, 0x11, 0x89, 0x10]); // mov edx, [ecx]; mov [eax], edx
        c.extend([0x8b, 0x4c, 0x24, 0x14]); // mov ecx, [esp+0x14]
        c.extend([0x8b, 0x11, 0x89, 0x50, 0x04]); // mov edx, [ecx]; mov [eax+4], edx
        c.extend([0x8b, 0x51, 0x04, 0x89, 0x50, 0x08]); // mov edx, [ecx+4]; mov [eax+8], edx
        c.extend([0x8b, 0x54, 0x24, 0x08, 0x89, 0x50, 0x0c]); // size -> [eax+0xc]
        c.extend([0x8b, 0x54, 0x24, 0x0c, 0x89, 0x50, 0x10]); // count -> [eax+0x10]
        c.extend([0xb8, 0x55, 0, 0, 0, 0xc3]); // mov eax, 0x55; ret
        let vsn = t.code(&c);
        t.import(m::IAT_VSNPRINTF_S, vsn);
        // the TTY callback's slot 1 (thiscall `(level, message)`, `ret
        // 8`): logs `ecx`, the level and the message's five words
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(0x7401u32.to_le_bytes()); // mov [eax], tag
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        c.extend([0x8b, 0x54, 0x24, 0x04, 0x89, 0x50, 0x08]); // level -> [eax+8]
        c.extend([0x8b, 0x4c, 0x24, 0x08]); // mov ecx, [esp+8]
        for k in 0..5u8 {
            c.extend([0x8b, 0x51, 4 * k, 0x89, 0x50, 0x0c + 4 * k]);
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.extend([0xb8, 0x66, 0, 0, 0, 0xc2, 0x08, 0x00]); // mov eax, 0x66; ret 8
        let tty = t.code(&c);
        t.jump(0x0043_54f0, tty);
        t.stub(m::ATEXIT, cdecl(1, 0x7402));
        for round in 0..6u32 {
            // the callback made already, made but not pointed at, not made
            let (once, made) = match round % 3 {
                0 => (1, 1),
                1 => (0, 1),
                _ => (0, 0),
            };
            let vt = t.rebase(TTY_VFTABLE);
            let obj = t.rebase(TTY_OBJECT);
            t.w32(obj, vt);
            let p = t.rebase(TTY_POINTER);
            t.w32(p, obj);
            let o = t.rebase(TTY_ONCE);
            let w = rng.next() & !1;
            t.w32(o, w | once);
            let o = t.rebase(TTY_OBJECT_ONCE);
            let w = rng.next() & !1;
            t.w32(o, w | made);
            let format = t.random(&mut rng, 8);
            let (level, v1, v2) = (rng.below(5), rng.next(), rng.next());
            let args = [level, format, v1, v2];
            let block: Vec<u8> = args.iter().flat_map(|w| w.to_le_bytes()).collect();
            let block = t.alloc(&block);
            t.w32(log, 0);
            t.check(
                &format!("PhyrePrintf round {round}"),
                0x0043_52f0,
                Conv::Cdecl,
                &args,
                ALL,
                |g| {
                    let buf = g.scratch();
                    m::bridge::phyre_printf(g, level, format, block + 8, buf)
                },
            );
        }
        t.unstub_all();
    }
}

#[test]
fn destructors_match_the_original() {
    use ffx_pother_a::p3_misc3::{self as m};
    for mut t in Twins::all() {
        let mut rng = Rng(0x0d70_5003);
        t.stub(FREE, cdecl(1, 0x7501));
        t.stub(m::SCRIPT_RELEASE_5D5450, cdecl(1, 0x7502));
        t.stub(m::SCRIPTABLE_DTOR, method(0, 0x7503));
        t.stub(m::SCRIPTABLE_CTOR, method(1, 0x7504));
        t.stub(m::RELEASE_TAGGED_WORDS, cdecl(2, 0x7505));
        t.stub(m::FREE_LIST_DEALLOCATE, method(1, 0x7506));
        t.stub(m::FREE_LIST_RESET, method(0, 0x7507));
        t.stub(m::DRAIN_AND_CLEAR, method(0, 0x7508));
        t.stub(m::FREE_PAIR_ARRAY, cdecl(2, 0x7509));
        for round in 0..12u32 {
            // the timer component: a handle owned or not, deleting or not
            let p = t.random(&mut rng, 0x40);
            let handle = if round & 1 == 0 { 0 } else { rng.next() };
            t.w32(p + 0x1c, handle);
            t.write(
                p + 0x28,
                &[if round & 2 == 0 {
                    0
                } else {
                    1 + rng.below(255) as u8
                }],
            );
            let flags = (rng.next() & !1) | ((round >> 2) & 1);
            t.check(
                &format!("timer delete round {round}"),
                0x0055_53e0,
                Conv::Thiscall(p),
                &[flags],
                ALL,
                |g| m::bridge::timer_component_delete(g, p, flags),
            );
            // the timer component's create callback
            let p = if round & 1 == 0 {
                0
            } else {
                t.random(&mut rng, 0x40)
            };
            t.check(
                &format!("timer create round {round}"),
                0x0055_5bc0,
                Conv::Stdcall,
                &[p],
                AL,
                |g| u32::from(m::bridge::timer_component_create(g, p)),
            );
            // the spline's destructor
            let p = t.random(&mut rng, 0x40);
            t.check(
                &format!("spline dtor round {round}"),
                0x0054_72c0,
                Conv::Thiscall(p),
                &[],
                ALL,
                |g| m::bridge::spline_dtor(g, p),
            );
            // a script's arrays: tagged words, owned or not
            let p = t.random(&mut rng, 0x40);
            if round & 1 == 0 {
                let w = rng.next() & 0x7fff_ffff;
                t.w32(p, w);
            }
            if round & 2 == 0 {
                let w = rng.next() & 0x7fff_ffff;
                t.w32(p + 8, w);
            }
            if round & 4 == 0 {
                t.w32(p + 4, 0);
            }
            t.check(
                &format!("script release round {round}"),
                0x005d_4770,
                Conv::Thiscall(p),
                &[],
                ALL,
                |g| m::bridge::script_release(g, p),
            );
            // the scheduler: two lists of 0..3 nodes, arrays owned or not
            let s = t.random(&mut rng, 0x60);
            for head in [0x24, 0x1c] {
                let mut next = 0;
                for _ in 0..rng.below(4) {
                    let n = t.random(&mut rng, 0x20);
                    t.w32(n, next);
                    next = n;
                }
                t.w32(s + head, next);
            }
            for (k, (count, data)) in [(0x34, 0x38), (0x28, 0x2c)].into_iter().enumerate() {
                let state = (round + 3 * k as u32) % 6;
                let n = match state % 3 {
                    0 => 0,
                    1 => rng.next() | 0x8000_0000,
                    _ => rng.next() & 0x7fff_ffff,
                };
                t.w32(s + count, n);
                if state >= 3 {
                    t.w32(s + data, 0);
                }
            }
            let log = t.log;
            t.w32(log, 0);
            t.check(
                &format!("scheduler clear round {round}"),
                0x005d_18f0,
                Conv::Thiscall(s),
                &[],
                0,
                |g| {
                    m::bridge::scheduler_clear(g, s);
                    0
                },
            );
        }
        t.unstub_all();
    }
}

#[test]
fn member_step_matches_the_original() {
    use ffx_pother_a::p3_misc3::{self as m};
    for mut t in Twins::all() {
        let mut rng = Rng(0x0047_ed80);
        for round in 0..56u32 {
            // rounds 48..: no nested view on a member not skipped
            let late = round >= 48;
            let skip_bit = if late { 0 } else { (round & 1) << 1 };
            let flags = (rng.next() & !0x42) | skip_bit | (((round >> 1) & 1) << 6);
            let kind = if late {
                3
            } else if (round >> 2) & 1 == 0 {
                2
            } else {
                rng.below(4)
            };
            let answer = if (round >> 3) & 1 == 0 {
                1
            } else {
                rng.below(3)
            };
            let obj = if round % 7 == 6 {
                0
            } else {
                let o = t.vobj(0x7600, &[(0, 0), (0, 0), (0, 0), (0, 0), (1, answer)]);
                t.w32(o + 0x30, kind);
                o
            };
            let view = t.vobj(0x7610, &[(0, 0), (0, obj)]);
            let extra = rng.next();
            t.w32(view + 0x24, extra);
            let dm = t.random(&mut rng, 0x20);
            let a = t.random(&mut rng, 0x90);
            let d = t.random(&mut rng, 0x10);
            t.w32(d + 4, dm);
            t.w32(d + 8, a);
            let member = t.random(&mut rng, 0x20);
            t.w32(member + 0x10, flags);
            let c = t.random(&mut rng, 0x20);
            let q = if round % 5 == 4 {
                0
            } else {
                t.random(&mut rng, 0x90)
            };
            let nested = if late {
                0
            } else {
                let n = t.random(&mut rng, 0x10);
                t.w32(n + 4, c);
                t.w32(n + 8, q);
                n
            };
            let this = t.random(&mut rng, 0x20);
            t.w32(this, d);
            t.w32(this + 4, member);
            t.w32(this + 8, view);
            t.w32(this + 0x14, nested);
            let offset = t.random(&mut rng, 4);
            let base = rng.next();
            let log = t.log;
            t.w32(log, 0);
            t.check(
                &format!("member step round {round}"),
                0x0047_ed80,
                Conv::Thiscall(this),
                &[base, offset],
                ALL,
                |g| m::bridge::member_step(g, this, base, offset),
            );
        }
    }
}

#[test]
fn red_black_insert_fixup_matches_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x0057_4980);
        for case in 0..160u32 {
            let tree = t.zeros(0x20);
            let n = 1 + (case % 24) as usize;
            let nodes = build_tree(&mut t, &mut rng, tree, n);
            // the root black, as in every tree the game builds (its
            // parent's other child is no node)
            let root = t.r32(tree + 0xc);
            let w = t.r32(root + 8);
            t.w32(root + 8, w & !1);
            // the new node: red, anywhere (a leaf in most cases)
            let k = rng.below(n as u32) as usize;
            let x = nodes[k];
            let w = t.r32(x + 8);
            t.w32(x + 8, w | 1);
            t.check(
                &format!("insert fixup {case}"),
                0x0057_4980,
                Conv::Thiscall(tree),
                &[x],
                ALL,
                |g| tree::bridge::post_insert_finalize(g, tree, x),
            );
        }
    }
}
