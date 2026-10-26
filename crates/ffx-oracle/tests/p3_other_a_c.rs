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
