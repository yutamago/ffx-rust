//! `ffx_pcore::m5p_core` (M5 stage 1, list `phyre-core`) against the
//! original: on the image alone with synthetic data, and on a snapshot
//! (`FFX_SNAPSHOT`) with the same data over the snapshot's memory. No
//! coverage run reached these functions, so every branch gets synthetic
//! cases. Results (all of `eax` where the original defines it, `ST0` for
//! the float result) and every byte written must agree; the x87 stack must
//! be empty after the integer ones. Needs `FFX_GAME_DIR`. The harness is
//! p5-core's (`tests/p5_core_c.rs`): recording stubs log their tag, `ecx`
//! and up to 14 stack words, the log cleared before every case and
//! compared as memory.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pcore::m5p_core::{self as c, bridge as b};
use ffx_pcore::m5p_core_b::{self as cb, bridge as bb};

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
}

/// The bridge's view of an oracle, with the oracle's float trampoline
/// ([`TRAMPOLINE`]).
struct G<'a>(&'a mut Oracle, u32);

/// `fld qword [esp+4]; call [esp+0xc]; ret`, then at +16 `fstp st(0);
/// ret`: a CRT intrinsic called with its operand in `ST0`, and the pop
/// after a float result was read.
const TRAMPOLINE: [u8; 19] = [
    0xdd, 0x44, 0x24, 0x04, 0xff, 0x54, 0x24, 0x0c, 0xc3, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
    0xdd, 0xd8, 0xc3,
];

impl G<'_> {
    /// Pops the float a call left in `ST0`.
    fn pop(&mut self) {
        self.0.call(self.1 + 16, Conv::Cdecl, &[]).unwrap();
    }
}

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
    fn call_st0(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> f32 {
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
        let v = r.st0_f32().expect("the callee's ST0");
        self.pop();
        v
    }
    fn call_intrinsic(&mut self, addr: u32, x: f64) -> f32 {
        let b = x.to_bits();
        let r = self
            .0
            .call(self.1, Conv::Cdecl, &[b as u32, (b >> 32) as u32, addr])
            .unwrap();
        let v = r.st0_f32().expect("the intrinsic's ST0");
        self.pop();
        v
    }
}

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
}

/// A stub's `eax`.
#[derive(Clone, Copy)]
enum Ret {
    Imm(u32),
    /// The word in this cell at the time of the call.
    Cell(u32),
}

/// Log entries: 16 words (tag, `ecx`, 14 stack words).
const ENTRY: u32 = 64;
const ENTRIES: u32 = 0x400;

const ALL: u32 = u32::MAX;

struct Twins {
    label: &'static str,
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then [`ENTRY`]-byte entries.
    log: u32,
    /// [`TRAMPOLINE`] in both oracles.
    tramp: u32,
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
        };
        t.log = t.zeros((4 + ENTRY * ENTRIES) as usize);
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

    fn w8(&mut self, at: u32, v: u8) {
        self.write(at, &[v]);
    }

    fn r32(&self, at: u32) -> u32 {
        Mem::read(&self.a, at)
    }

    fn code(&mut self, c: &[u8]) -> u32 {
        let x = self.a.alloc_code(c).unwrap();
        assert_eq!(self.b.alloc_code(c).unwrap(), x);
        x
    }

    /// A recording stub tagged `tag`: logs `ecx` when `this`, `n` stack
    /// words, pops `pops` words, returns `ret`.
    fn stub_code(&mut self, tag: u32, this: bool, n: u32, pops: u32, ret: Ret) -> u32 {
        assert!(n <= 14);
        let log = self.log;
        let mut c = vec![0xa1];
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
        match ret {
            Ret::Imm(v) => {
                c.push(0xb8);
                c.extend(v.to_le_bytes()); // mov eax, v
            }
            Ret::Cell(at) => {
                c.push(0xa1);
                c.extend(at.to_le_bytes()); // mov eax, [at]
            }
        }
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        self.code(&c)
    }

    /// Replaces the original at `sym` by a recording stub (tag `sym`).
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: Ret) {
        let at = self.rebase(sym);
        let c = self.stub_code(sym, this, n, pops, ret);
        self.jump(at, c);
    }

    fn jump(&mut self, at: u32, to: u32) {
        let mut j = vec![0xe9];
        j.extend(to.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
    }

    /// Points import slot `slot` at a recording stub (stdcall, `n` words,
    /// tag `slot`).
    fn import(&mut self, slot: u32, n: u32, ret: Ret) {
        let c = self.stub_code(slot, false, n, n, ret);
        let at = self.rebase(slot);
        self.w32(at, c);
    }

    /// An object whose vtable slots `0..words.len()` are recording stubs
    /// (thiscall methods tagged `base + 4 * slot`, taking and popping
    /// `words[slot]` words, returning `base | slot`), `size` bytes of
    /// random words after its vtable pointer.
    fn object(&mut self, rng: &mut Rng, base: u32, words: &[u32], size: u32) -> u32 {
        let slots: Vec<u32> = (0u32..)
            .zip(words)
            .map(|(i, &n)| self.stub_code(base + 4 * i, true, n, n, Ret::Imm(base | i)))
            .collect();
        let vt: Vec<u8> = slots.iter().flat_map(|s| s.to_le_bytes()).collect();
        let vt = self.alloc(&vt);
        let o = self.random(rng, size as usize);
        self.w32(o, vt);
        o
    }

    /// Clears the call log (every case starts with an empty one).
    fn fresh_log(&mut self) {
        let log = self.log;
        self.w32(log, 0);
    }

    fn calls(&self) -> u32 {
        self.r32(self.log)
    }

    /// Runs the original at `sym` on `a` and `port` on `b`; compares
    /// `eax & mask` and every byte written; the original must leave the
    /// x87 stack empty.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        mask: u32,
        port: impl FnOnce(&mut G) -> u32,
    ) {
        self.fresh_log();
        let f = self.rebase(sym);
        let ra = self
            .a
            .call(f, conv, args)
            .unwrap_or_else(|e| panic!("{} {what}: original: {e}", self.label));
        let rb = port(&mut G(&mut self.b, self.tramp));
        assert!(
            ra.st0.is_none(),
            "{} {what}: the original leaves ST0 set",
            self.label
        );
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

    /// [`Twins::check`] for a float result in `ST0`.
    fn check_f32(&mut self, what: &str, sym: u32, args: &[u32], port: impl FnOnce(&mut G) -> f32) {
        self.check_f32_conv(what, sym, Conv::Cdecl, args, port);
    }

    /// [`Twins::check_f32`] in another convention.
    fn check_f32_conv(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        port: impl FnOnce(&mut G) -> f32,
    ) {
        self.fresh_log();
        let f = self.rebase(sym);
        let ra = self
            .a
            .call(f, conv, args)
            .unwrap_or_else(|e| panic!("{} {what}: original: {e}", self.label));
        let rb = port(&mut G(&mut self.b, self.tramp));
        assert!(
            ra.st0_is(f64::from(rb)),
            "{} {what}: ST0 {:?} vs {rb:?}",
            self.label,
            ra.st0
        );
        self.same(what);
    }

    fn same(&mut self, what: &str) {
        let calls = self.calls();
        assert!(calls < ENTRIES, "{} {what}: call log full", self.label);
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
}

const FREE: u32 = c::PHYRE_FREE;
const DELETE: u32 = c::OPERATOR_DELETE;

/// A count word: owned or external (bit 31), small.
fn count_word(rng: &mut Rng) -> u32 {
    let n = rng.below(5);
    // bit 30 now and then: only bit 31 is the flag
    let n = if rng.chance(4) { n | 0x4000_0000 } else { n };
    if rng.chance(2) { n | 0x8000_0000 } else { n }
}

/// A floating-point word: ordinary values, zeros, infinities, quiet NaNs
/// and denormals (no signalling NaNs: the emulator keeps them).
fn float_word(rng: &mut Rng) -> u32 {
    let sign = (rng.next() & 1) << 31;
    let w = match rng.below(8) {
        0 => 0,
        1 => 0x7f80_0000,
        2 => 0x7fc0_0000 | (rng.next() & 0x003f_ffff),
        3 => rng.next() & 0x007f_ffff,
        4 => 0x3f80_0000,
        _ => {
            let x = f32::from((rng.next() % 2000) as u16) / 37.0 - 27.0;
            return x.to_bits();
        }
    };
    sign | w
}

#[test]
fn descriptor_destructors_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d001);
        t.stub(
            ffx_prender::m5r_f::DESCRIPTOR_DESTRUCT,
            true,
            0,
            0,
            Ret::Imm(0x00de_57a0),
        );
        for &(f, vt) in c::DESTRUCT {
            assert_eq!(c::vtable_of(f), vt);
            for _ in 0..3 {
                let this = t.random(&mut rng, 0x40);
                t.check("destruct", f, Conv::Thiscall(this), &[], ALL, |g| {
                    ffx_prender::m5r_f::bridge::descriptor_destruct(g, this, vt)
                });
                assert_eq!(t.calls(), 1);
            }
        }
    }
}

#[test]
fn array_resets_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d002);
        t.stub(FREE, false, 1, 0, Ret::Imm(0x0f7e_e000));
        t.stub(c::SEGMENTS_DESTROY, false, 2, 0, Ret::Imm(0x5e65_0000));
        for &f in c::RESET {
            for case in 0..8u32 {
                let this = t.random(&mut rng, 0x10);
                t.w32(this, count_word(&mut rng) | (case & 1) << 31);
                let data = if case & 2 == 0 { 0 } else { t.zeros(8) };
                t.w32(this + 4, data);
                t.check("reset", f, Conv::Thiscall(this), &[], ALL, |g| {
                    b::owned_reset(g, this)
                });
            }
        }
        for case in 0..8u32 {
            let this = t.random(&mut rng, 0x10);
            t.w32(this, rng.next() & 0x7fff_ffff | (case & 1) << 31);
            if case & 2 == 0 {
                t.w32(this + 4, 0);
            }
            t.check(
                "segments reset",
                0x0043_3990,
                Conv::Thiscall(this),
                &[],
                ALL,
                |g| b::segments_reset(g, this),
            );
        }
        // PString's destructor: borrowed (bit 0) or owned text
        for case in 0..6u32 {
            let this = t.random(&mut rng, 8);
            let p = rng.next() & !1 | (case & 1);
            t.w32(this, p);
            t.check(
                "string dtor",
                0x0042_f120,
                Conv::Thiscall(this),
                &[],
                0,
                |g| {
                    b::string_dtor(g, this);
                    0
                },
            );
            assert_eq!(t.calls(), 1 - (case & 1));
        }
    }
}

/// A random binary tree of `n` nodes (0x10 bytes each) under `tree`'s
/// header, with random colour bits; the nodes in key order (p3's).
fn build_tree(t: &mut Twins, rng: &mut Rng, tree: u32, n: usize) -> Vec<u32> {
    let header = tree + 0xc;
    let nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x10)).collect();
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
    t.w32(header, root.map_or(tree, |r| nodes[r]));
    t.w32(header + 4, 0x7777_0000);
    t.w32(header + 8, tree);
    nodes
}

#[test]
fn byte_copies_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d003);
        t.stub(FREE, false, 1, 0, Ret::Imm(0x0f7e_e000));
        let fresh = t.zeros(0x40);
        let malloc = 0x0042_fb80;
        for case in 0..24u32 {
            // resize (PArray<u8>'s code): same size, 0, a failed or a fresh block
            let ret = if case % 3 == 0 { 0 } else { fresh };
            t.stub(malloc, false, 2, 0, Ret::Imm(ret));
            let this = t.random(&mut rng, 0x10);
            let count = count_word(&mut rng);
            t.w32(this, count);
            if case % 4 == 0 {
                t.w32(this + 4, 0);
            }
            let n = match case % 4 {
                0 => count,
                1 => 0,
                _ => rng.below(9),
            };
            t.check(
                "reserve",
                0x0044_4e10,
                Conv::Thiscall(this),
                &[n],
                ALL,
                |g| ffx_pload::descriptor::bridge::array_resize(g, this, n),
            );
        }
        for case in 0..24u32 {
            let this = t.random(&mut rng, 0x10);
            t.w32(this, count_word(&mut rng));
            let old = if case & 1 == 0 { 0 } else { rng.next() };
            t.w32(this + 4, old);
            let data = if case & 2 == 0 { old } else { rng.next() };
            let count = match case % 3 {
                0 => rng.next() | 0x8000_0000,
                1 => 1,
                _ => rng.below(100),
            };
            t.check(
                "setData",
                0x0049_c100,
                Conv::Thiscall(this),
                &[data, count],
                ALL,
                |g| ffx_pload::cluster::bridge::array_set_data(g, this, data, count as i32),
            );
            for f in [0x004b_aac0, 0x004c_d600] {
                t.w32(this, count_word(&mut rng));
                t.w32(this + 4, old);
                t.check(
                    "setToExternalBuffer",
                    f,
                    Conv::Thiscall(this),
                    &[data, count],
                    ALL,
                    |g| {
                        ffx_prender::class_slots::bridge::set_external(
                            g,
                            this,
                            data,
                            count,
                            ffx_prender::class_slots::Release::Free,
                        )
                    },
                );
            }
            t.w32(this, count_word(&mut rng));
            t.w32(this + 4, old);
            t.check(
                "setStorage",
                0x00a3_47b0,
                Conv::Thiscall(this),
                &[data, count],
                ALL,
                |g| ffx_prender::array::bridge::set_storage(g, this, data, count, None),
            );
            // the PSharray: one element lives in the header
            let src = t.random(&mut rng, 8);
            let data = if case & 4 == 0 { src } else { this + 4 };
            let count = match case % 4 {
                0 => 0x8000_0001,
                1 => 1,
                2 => 0,
                _ => rng.below(6),
            };
            t.w32(this, count_word(&mut rng));
            t.w32(this + 4, old);
            t.check(
                "PSharray set",
                0x004f_8f90,
                Conv::Thiscall(this),
                &[data, count],
                ALL,
                |g| ffx_prender::h1_small2::bridge::sharray_set(g, this, data, count),
            );
            t.w32(this, count_word(&mut rng));
            let i = rng.below(40);
            t.check(
                "element28",
                0x0058_5c30,
                Conv::Thiscall(this),
                &[i],
                ALL,
                |g| ffx_prender::array::bridge::element(g, this, i, 0x1c),
            );
        }
        for _ in 0..4 {
            let out = t.random(&mut rng, 0x40);
            t.check("identity", 0x0045_4cf0, Conv::Cdecl, &[out], ALL, |g| {
                ffx_pcore::matrix::bridge::identity(g, out)
            });
            let pair = t.random(&mut rng, 8);
            t.check(
                "pair init",
                0x005c_d3f0,
                Conv::Thiscall(pair),
                &[],
                ALL,
                |g| ffx_pother_a::object::bridge::pair_init(g, pair),
            );
            let x = rng.next();
            t.check("workerThread", 0x0057_d0a0, Conv::Stdcall, &[x], ALL, |_| 0);
        }
        for _ in 0..12 {
            let n = 1 + rng.below(12) as usize;
            let tr = t.zeros(0x20);
            let nodes = build_tree(&mut t, &mut rng, tr, n);
            for &x in &nodes {
                t.check(
                    "successor",
                    0x0057_51d0,
                    Conv::Thiscall(tr),
                    &[x],
                    ALL,
                    |g| ffx_pother_a::p3_tree::bridge::successor(g, tr, x),
                );
            }
        }
    }
}

#[test]
fn thread_members_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d004);
        t.import(c::WAIT_FOR_SINGLE_OBJECT, 2, Ret::Imm(0));
        t.import(c::CLOSE_HANDLE, 1, Ret::Imm(1));
        // GetExitCodeThread: writes its code through the second word
        let rec = t.stub_code(c::GET_EXIT_CODE_THREAD, false, 1, 2, Ret::Imm(1));
        let mut code = vec![0x8b, 0x44, 0x24, 0x08]; // mov eax, [esp+8]
        code.extend([0xc7, 0x00, 0x03, 0x01, 0xc0, 0x0e]); // mov dword [eax], 0x0ec00103
        code.extend([0x90; 5]); // jmp rec, patched below
        let at = t.code(&code);
        t.jump(at + code.len() as u32 - 5, rec);
        let slot = t.rebase(c::GET_EXIT_CODE_THREAD);
        t.w32(slot, at);
        for _ in 0..4 {
            let this = t.random(&mut rng, 8);
            let out = t.random(&mut rng, 8);
            let local = t.b.stack_range().start + 0x80;
            t.check(
                "join",
                0x0043_01b0,
                Conv::Thiscall(this),
                &[out],
                ALL,
                |g| b::thread_join(g, this, out, local),
            );
            assert_eq!(t.r32(out), 0x0ec0_0103);
            for f in [0x0043_0240, 0x0043_0250] {
                t.check("close", f, Conv::Thiscall(this), &[], ALL, |g| {
                    b::close_handle(g, this)
                });
            }
        }
        t.check("thread name", 0x0043_17a0, Conv::Cdecl, &[], ALL, |g| {
            b::default_thread_name(g)
        });
        // the pool's thread entry: slot 0 of the pool, (pool; arg)
        for _ in 0..3 {
            let pool = t.object(&mut rng, 0x7001_0000, &[1], 0x20);
            let arg = t.random(&mut rng, 0x14);
            t.w32(arg, pool);
            t.check("thread entry", 0x0044_ce30, Conv::Cdecl, &[arg], ALL, |g| {
                b::thread_entry(g, arg)
            });
        }
        t.stub(c::ZERO_WORD_CTOR, true, 0, 0, Ret::Imm(0x2e40_0000));
        t.stub(c::THREAD_POOL_DTOR_BODY, true, 0, 0, Ret::Imm(0x0d70_0000));
        t.stub(c::TIMELINE_DTOR, true, 0, 0, Ret::Imm(0x0c70_0000));
        t.stub(c::SEMAPHORE_CTOR, true, 0, 0, Ret::Imm(0x0c90_0000));
        t.stub(c::CRITICAL_SECTION_CTOR, true, 0, 0, Ret::Imm(0x1690_0000));
        t.stub(DELETE, false, 1, 0, Ret::Imm(0x0de1_0000));
        t.stub(FREE, false, 1, 0, Ret::Imm(0x0f7e_e000));
        for case in 0..6u32 {
            let this = t.random(&mut rng, 0x40);
            if case == 3 {
                // the immediate pool's delete calls the pool's destructor
                t.stub(c::THREAD_POOL_DTOR, true, 0, 0, Ret::Imm(0x0c60_0000));
            }
            t.check(
                "state init",
                0x0044_cc20,
                Conv::Thiscall(this),
                &[],
                ALL,
                |g| b::pool_state_init(g, this),
            );
            if case < 3 {
                t.check(
                    "pool dtor",
                    0x0044_cc60,
                    Conv::Thiscall(this),
                    &[],
                    ALL,
                    |g| b::pool_dtor(g, this),
                );
            }
            let flags = rng.next() & !1 | (case & 1);
            t.check(
                "pool delete",
                0x0044_ccd0,
                Conv::Thiscall(this),
                &[flags],
                ALL,
                |g| b::pool_delete(g, this, flags),
            );
            if case >= 3 {
                t.check(
                    "immediate delete",
                    0x0057_cf20,
                    Conv::Thiscall(this),
                    &[flags],
                    ALL,
                    |g| b::immediate_delete(g, this, flags),
                );
            }
            t.check(
                "timeline delete",
                0x0057_d2e0,
                Conv::Thiscall(this),
                &[flags],
                ALL,
                |g| b::timeline_delete(g, this, flags),
            );
            // fresh random words for each member below
            let this = t.random(&mut rng, 0x40);
            t.check(
                "semaphore flag",
                0x0057_d0f0,
                Conv::Thiscall(this),
                &[],
                ALL,
                |g| b::semaphore_flag_init(g, this),
            );
            t.check(
                "locked list",
                0x005c_cfb0,
                Conv::Thiscall(this),
                &[],
                ALL,
                |g| b::locked_list_init(g, this),
            );
            t.check(
                "shutdown",
                0x0057_d050,
                Conv::Thiscall(this),
                &[],
                ALL,
                |g| b::free_buffer(g, this),
            );
            for f in [0x005c_d3b0, 0x005c_d3c0] {
                t.check("job vtable", f, Conv::Thiscall(this), &[], 0, |g| {
                    b::job_vtable(g, this);
                    0
                });
            }
            let job = t.object(&mut rng, 0x7002_0000, &[0, 3], 0x10);
            let this = t.random(&mut rng, 0x40);
            t.check(
                "run job",
                0x0057_d070,
                Conv::Thiscall(this),
                &[job],
                ALL,
                |g| b::run_job(g, this, job),
            );
        }
        // the morph manager's teardown: none, or one
        let g = t.rebase(c::MORPH_MANAGER);
        t.stub(c::MORPH_MANAGER_DTOR, true, 0, 0, Ret::Imm(0x0bc2_0000));
        for case in 0..2u32 {
            let m = if case == 0 {
                0
            } else {
                t.random(&mut rng, 0x10)
            };
            t.w32(g, m);
            t.check("morph manager", 0x006a_beb0, Conv::Cdecl, &[], 0, |g| {
                b::morph_manager_destroy(g);
                0
            });
            assert_eq!(t.calls(), 2 * case);
        }
    }
}

#[test]
fn lists_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d005);
        t.stub(DELETE, false, 1, 0, Ret::Imm(0x0de1_0000));
        // PUtility: on the list (first, middle, last), off it, empty list
        let head = t.rebase(c::UTILITY_LIST);
        let saved = t.r32(head);
        for case in 0..10u32 {
            let n = rng.below(4);
            let nodes: Vec<u32> = (0..n).map(|_| t.random(&mut rng, 0x10)).collect();
            for (i, &x) in nodes.iter().enumerate() {
                let next = nodes.get(i + 1).copied().unwrap_or(0);
                t.w32(x + 4, next);
            }
            t.w32(head, nodes.first().copied().unwrap_or(0));
            let this = if n > 0 && case % 3 != 0 {
                nodes[rng.below(n) as usize]
            } else {
                t.random(&mut rng, 0x10)
            };
            t.check(
                "utility dtor",
                0x0043_1d50,
                Conv::Thiscall(this),
                &[],
                0,
                |g| {
                    b::utility_dtor(g, this);
                    0
                },
            );
        }
        t.w32(head, saved);
        // PAnnotation: alone in its ring, or with others
        for case in 0..6u32 {
            let this = t.random(&mut rng, 0x10);
            let link = this + 4;
            let (next, prev) = if case % 2 == 0 {
                (link, link)
            } else {
                (t.random(&mut rng, 0x10), t.random(&mut rng, 0x10))
            };
            t.w32(link, next);
            t.w32(link + 4, prev);
            t.check(
                "annotation dtor",
                0x0057_5920,
                Conv::Thiscall(this),
                &[],
                ALL,
                |g| b::annotation_dtor(g, this),
            );
            t.w32(link, next);
            t.w32(link + 4, prev);
            let flags = rng.next() & !1 | (case >> 1 & 1);
            t.check(
                "annotation delete",
                0x0057_59f0,
                Conv::Thiscall(this),
                &[flags],
                ALL,
                |g| b::annotation_delete(g, this, flags),
            );
        }
        // the cursor skip: empty nodes, a full one, the end
        for _ in 0..8 {
            let n = 1 + rng.below(5) as usize;
            let end = t.random(&mut rng, 8);
            let nodes: Vec<u32> = (0..n).map(|_| t.random(&mut rng, 8)).collect();
            for (i, &x) in nodes.iter().enumerate() {
                t.w32(x, nodes.get(i + 1).copied().unwrap_or(end));
                let full = if rng.chance(3) { rng.next() | 1 } else { 0 };
                t.w32(x + 4, full);
            }
            let cursor = t.alloc(&nodes[0].to_le_bytes());
            t.check("skip", 0x005c_d0c0, Conv::Cdecl, &[end, cursor], ALL, |g| {
                b::skip_empty(g, end, cursor)
            });
            t.w32(cursor, end);
            t.check(
                "skip at end",
                0x005c_d0c0,
                Conv::Cdecl,
                &[end, cursor],
                ALL,
                |g| b::skip_empty(g, end, cursor),
            );
        }
        // the tree erase: found or not
        t.stub(c::TREE_REMOVE, true, 1, 1, Ret::Imm(0x5000_0000));
        t.stub(c::FREE_LIST_DEALLOCATE, true, 1, 1, Ret::Imm(0x0980_0000));
        for case in 0..2u32 {
            let found = if case == 0 { 0 } else { 0x0bad_0de0 };
            t.stub(c::TREE_FIND, true, 1, 1, Ret::Imm(found));
            let tree = t.random(&mut rng, 0x40);
            let key = t.random(&mut rng, 4);
            t.check(
                "tree erase",
                0x0057_cb70,
                Conv::Thiscall(tree),
                &[key],
                ALL,
                |g| b::tree_erase(g, tree, key),
            );
        }
        // the element array delete: none, or a counted block
        t.stub(c::VECTOR_DTOR, false, 4, 4, Ret::Imm(0x96d2_0000));
        t.stub(FREE, false, 1, 0, Ret::Imm(0x0f7e_e000));
        for case in 0..2u32 {
            let this = t.random(&mut rng, 8);
            let p = if case == 0 {
                0
            } else {
                let block = t.random(&mut rng, 0x20);
                block + 4
            };
            t.w32(this, p);
            t.check(
                "delete elements",
                0x005c_d490,
                Conv::Thiscall(this),
                &[],
                ALL,
                |g| b::element_array_delete(g, this),
            );
        }
        // the uninitialised copy: n = 0..4, a null first slot (skipped
        // without moving the source; page 0 mapped so the later slots
        // land there)
        t.a.map_zeroed(0, 0x1000).unwrap();
        t.b.map_zeroed(0, 0x1000).unwrap();
        for case in 0..10u32 {
            let src = t.random(&mut rng, 64);
            let dst = if case % 3 == 0 {
                0
            } else {
                t.random(&mut rng, 64)
            };
            let n = rng.below(5);
            t.check("copy12", 0x005a_1030, Conv::Cdecl, &[dst, src, n], 0, |g| {
                b::copy12(g, dst, src, n);
                0
            });
        }
    }
}

#[test]
fn small_members_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d006);
        // the class register, (descriptor; base + offset, flag)
        t.stub(c::CLASS_REGISTER, true, 2, 2, Ret::Imm(0x3d6a_0042));
        for _ in 0..4 {
            let this = t.random(&mut rng, 0x90);
            let (offset, flag) = (rng.next(), rng.below(3));
            t.check(
                "register",
                0x0044_a450,
                Conv::Thiscall(this),
                &[offset, flag],
                ALL,
                |g| b::register_member(g, this, offset, flag),
            );
        }
        t.stub(c::TO_WORLD_MATRIX, true, 1, 1, Ret::Imm(0x1190_0000));
        t.stub(c::MATRIX_COPY, true, 1, 1, Ret::Imm(0x3340_0000));
        for _ in 0..8 {
            let this = t.random(&mut rng, 0x100);
            let m = t.random(&mut rng, 0x40);
            t.check(
                "to world",
                0x0045_2730,
                Conv::Thiscall(this),
                &[m],
                ALL,
                |g| b::to_world_matrix(g, this, m),
            );
            let v = float_word(&mut rng);
            for (f, off) in [
                (0x0045_2790, 0xc),
                (0x0045_4e50, 0),
                (0x0045_4e60, 4),
                (0x0045_4e70, 8),
            ] {
                t.check("set float", f, Conv::Thiscall(this), &[v], ALL, |g| {
                    b::set_float(g, this, off, v)
                });
            }
            let proj = t.object(&mut rng, 0x7003_0000, &[0, 0], 0x100);
            for (f, off) in [(0x0045_4e10, 0xbc), (0x0045_4e30, 0xb8)] {
                t.check("set and update", f, Conv::Thiscall(proj), &[v], ALL, |g| {
                    b::set_float_update(g, proj, off, v)
                });
            }
            // a row of four floats
            let row = t.alloc(
                &(0..4)
                    .flat_map(|_| float_word(&mut rng).to_le_bytes())
                    .collect::<Vec<u8>>(),
            );
            let i = rng.below(4);
            t.check(
                "set row",
                0x0057_4680,
                Conv::Thiscall(this),
                &[i, row],
                ALL,
                |g| b::set_row(g, this, i, row),
            );
            // a matrix array: below the count or not
            let arr = t.random(&mut rng, 0x10);
            t.w32(arr + 8, count_word(&mut rng));
            let i = rng.below(6);
            t.check(
                "matrix set",
                0x004f_8e80,
                Conv::Thiscall(arr),
                &[i, m],
                ALL,
                |g| b::matrix_set(g, arr, i, m),
            );
        }
        // Phyre_maxFloat over every pair kind
        for _ in 0..200 {
            let (x, y) = (float_word(&mut rng), float_word(&mut rng));
            t.check_f32("maxFloat", 0x0051_26e0, &[x, y], |_| b::max_float(x, y));
        }
        // the owner store: no value, no target, 0 < v or not
        for case in 0..24u32 {
            let this = t.random(&mut rng, 0x20);
            let target = if case % 5 == 0 {
                0
            } else {
                t.random(&mut rng, 0x20)
            };
            t.w32(this + 0x1c, target);
            let v = if case % 7 == 0 {
                0
            } else {
                t.alloc(&float_word(&mut rng).to_le_bytes())
            };
            t.check(
                "owner if positive",
                0x004f_9260,
                Conv::Thiscall(this),
                &[v],
                0,
                |g| {
                    b::set_owner_if_positive(g, this, v);
                    0
                },
            );
        }
        // the 16-bit index flag: no shader, no format, 12 or not; the
        // cursor pointing at itself once
        for case in 0..10u32 {
            let shader = if case % 5 == 0 {
                0
            } else {
                t.random(&mut rng, 0x20)
            };
            if shader != 0 {
                let fmt = if case % 5 == 1 {
                    0
                } else {
                    let p = t.random(&mut rng, 0x40);
                    t.w8(p + 0x3c, if case & 1 == 0 { 0xc } else { rng.next() as u8 });
                    p
                };
                t.w32(shader + 0x1c, fmt);
            }
            let cursor = t.zeros(4);
            let at = if case == 9 {
                cursor
            } else {
                t.random(&mut rng, 8)
            };
            t.w32(cursor, at);
            let args = [
                rng.next(),
                rng.next(),
                rng.next(),
                cursor,
                rng.next(),
                shader,
            ];
            t.check("16-bit flag", 0x004d_43f0, Conv::Stdcall, &args, ALL, |g| {
                b::index16_flag(g, cursor, shader)
            });
        }
        // the rigid body's forward: the class in the chain or not, no object
        let want = t.rebase(c::RIGID_BODY_CLASS);
        for case in 0..10u32 {
            let this = t.random(&mut rng, 0x10);
            let n = rng.below(4);
            let mut chain = 0;
            for k in 0..n {
                let cls = if k == 0 && case % 3 != 0 {
                    want
                } else {
                    t.random(&mut rng, 0x44)
                };
                if cls != want {
                    t.w32(cls + 0x40, chain);
                }
                chain = cls;
            }
            t.w32(this + 8, chain);
            let obj = t.object(&mut rng, 0x7004_0000, &[0, 0, 0, 0, 0, 0, 0, 0, 0], 0x10);
            let p = match case % 4 {
                0 => 0,
                1 => 4,
                _ => obj + 4,
            };
            t.w32(this + 4, p);
            t.check(
                "rigid body",
                0x005d_3e90,
                Conv::Thiscall(this),
                &[],
                ALL,
                |g| b::rigid_body_slot8(g, this),
            );
        }
    }
}

// ---- batch 2: the containers (ffx_pcore::m5p_core_b) ----

/// An array header: a count word (owned or external) and data (none or a
/// block of `size` random bytes).
fn array_header(t: &mut Twins, rng: &mut Rng, size: usize) -> u32 {
    let this = t.random(rng, 0x10);
    t.w32(this, count_word(rng));
    let data = if rng.chance(4) {
        0
    } else {
        t.random(rng, size)
    };
    t.w32(this + 4, data);
    this
}

/// The new count for a resize of `this`: the same word, 0, or small.
fn new_count(t: &Twins, rng: &mut Rng, this: u32, case: u32) -> u32 {
    match case % 5 {
        0 => t.r32(this),
        1 => 0,
        _ => rng.below(7),
    }
}

#[test]
fn resizes_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d101);
        let cell = t.zeros(4);
        let fresh = t.zeros(0x400);
        t.stub(cb::PHYRE_MALLOC, false, 2, 0, Ret::Cell(cell));
        t.stub(cb::PHYRE_FREE, false, 1, 0, Ret::Imm(0x0f7e_e000));
        for &(f, r) in cb::RESIZE {
            match r.alloc {
                cb::Alloc::Realloc(g) => t.stub(g, false, 3, 0, Ret::Cell(cell)),
                cb::Alloc::Malloc {
                    copy, construct, ..
                } => {
                    if copy != 0 {
                        t.stub(copy, false, 3, 0, Ret::Imm(0x00c0_0000));
                    }
                    if construct != 0 {
                        t.stub(construct, false, 2, 0, Ret::Imm(0x00c1_0000));
                    }
                }
            }
            if let cb::Release::Destroy(g) = r.release {
                t.stub(g, false, 2, 0, Ret::Imm(0x00de_0000));
            }
            for case in 0..30u32 {
                let this = array_header(&mut t, &mut rng, 0x40);
                let n = new_count(&t, &mut rng, this, case);
                // the block: fresh, none, or the old data itself
                let block = match case % 7 {
                    0 => 0,
                    1 => t.r32(this + 4),
                    _ => fresh,
                };
                t.w32(cell, block);
                t.check("resize", f, Conv::Thiscall(this), &[n], ALL, |g| {
                    bb::resize(g, this, n, r)
                });
            }
            assert_eq!(format!("{:?}", cb::resize_of(f)), format!("{r:?}"));
        }
        // the PSharray resize: inline and heap storage
        t.stub(cb::SHARRAY_REALLOC, true, 3, 3, Ret::Cell(cell));
        for case in 0..40u32 {
            let this = array_header(&mut t, &mut rng, 0x40);
            let n = new_count(&t, &mut rng, this, case);
            let block = match case % 6 {
                0 => 0,
                1 => this + 4,
                2 => t.r32(this + 4),
                _ => fresh,
            };
            t.w32(cell, block);
            t.check(
                "sharray resize",
                0x0048_c470,
                Conv::Thiscall(this),
                &[n],
                ALL,
                |g| bb::sharray_resize(g, this, n),
            );
        }
        // the LOD levels: copy, defaults and the storage setter
        t.stub(cb::LOD_LEVELS_COPY, false, 3, 0, Ret::Imm(0x0f80_0000));
        t.stub(cb::LOD_LEVELS_DEFAULT, false, 2, 0, Ret::Imm(0x6280_0000));
        t.stub(cb::LOD_LEVELS_SET, true, 2, 2, Ret::Imm(0x7d90_0000));
        for case in 0..20u32 {
            let this = array_header(&mut t, &mut rng, 0x40);
            let n = new_count(&t, &mut rng, this, case);
            t.w32(cell, if case % 4 == 0 { 0 } else { fresh });
            t.check(
                "lod resize",
                0x005a_7b60,
                Conv::Thiscall(this),
                &[n],
                ALL,
                |g| bb::lod_levels_resize(g, this, n),
            );
        }
    }
}

#[test]
fn elements_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d102);
        for _ in 0..20 {
            let this = t.random(&mut rng, 0x20);
            t.w32(this, count_word(&mut rng) & 0x8000_0003);
            t.w32(this + 0x14, count_word(&mut rng) & 0x8000_0003);
            let i = rng.below(9);
            t.check(
                "element16",
                0x0058_5b50,
                Conv::Thiscall(this),
                &[i],
                ALL,
                |g| ffx_prender::array::bridge::element(g, this, i, 0x10),
            );
            t.check(
                "element12",
                0x005a_7770,
                Conv::Thiscall(this),
                &[i],
                ALL,
                |g| ffx_prender::array::bridge::element(g, this + 0x14, i, 12),
            );
        }
        // the 20-byte zeroing and the 64-byte constructions, a null start
        t.stub(cb::CTOR_55DC40, true, 0, 0, Ret::Imm(0xdc40_0000));
        // page 0 for the slots after a null start
        t.a.map_zeroed(0, 0x1000).unwrap();
        t.b.map_zeroed(0, 0x1000).unwrap();
        for case in 0..8u32 {
            let p = if case % 4 == 0 {
                0
            } else {
                t.random(&mut rng, 0x140)
            };
            let n = rng.below(5);
            if p == 0 {
                // known words in the slots the original writes
                let junk: Vec<u8> = (0..0x80).map(|_| rng.next() as u8).collect();
                t.write(0, &junk);
            }
            t.check("zero20", 0x0058_afd0, Conv::Cdecl, &[p, n], 0, |g| {
                bb::zero20(g, p, n);
                0
            });
            t.check("construct64", 0x0043_43c0, Conv::Cdecl, &[p, n], 0, |g| {
                bb::construct64(g, p, n);
                0
            });
        }
    }
}

#[test]
fn element_copies_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d103);
        // a null destination skips its slot without moving the source;
        // page 0 holds the slots after it
        t.a.map_zeroed(0, 0x1000).unwrap();
        t.b.map_zeroed(0, 0x1000).unwrap();
        let cell = t.zeros(4);
        let fresh = t.zeros(0x400);
        t.stub(cb::PHYRE_MALLOC, false, 2, 0, Ret::Cell(cell));
        let texts: Vec<u32> = ["", "a", "Position", "TEXCOORD0"]
            .iter()
            .map(|s| {
                let mut v = s.as_bytes().to_vec();
                v.push(0);
                t.alloc(&v)
            })
            .collect();
        for case in 0..16u32 {
            let n = rng.below(4);
            let src = t.random(&mut rng, 0x14 * 4);
            for k in 0..4 {
                let s = match rng.below(4) {
                    0 => 0,
                    1 => 1,
                    _ => texts[rng.below(4) as usize] | (rng.next() & 1),
                };
                t.w32(src + 0x14 * k, s);
            }
            let dst = if case % 4 == 0 {
                0
            } else {
                t.random(&mut rng, 0x14 * 4)
            };
            t.w32(cell, if case % 5 == 0 { 0 } else { fresh });
            t.check(
                "stream descs",
                0x0057_e190,
                Conv::Cdecl,
                &[dst, src, n],
                0,
                |g| {
                    bb::copy_stream_descs(g, dst, src, n);
                    0
                },
            );
        }
        t.stub(cb::CONTEXT_TAIL_COPY, true, 1, 1, Ret::Imm(0x2740_0000));
        t.stub(cb::LOD_LEVEL_4_COPY, true, 1, 1, Ret::Imm(0x2380_0000));
        t.stub(cb::LOD_SHARRAY_COPY, true, 1, 1, Ret::Imm(0x2440_0000));
        for case in 0..12u32 {
            let n = rng.below(4);
            let src = t.alloc(
                &(0..0x24)
                    .flat_map(|_| float_word(&mut rng).to_le_bytes())
                    .collect::<Vec<u8>>(),
            );
            let dst = if case % 4 == 0 {
                0
            } else {
                t.random(&mut rng, 0x24 * 4)
            };
            t.check(
                "segment contexts",
                0x004f_0950,
                Conv::Cdecl,
                &[dst, src, n],
                0,
                |g| {
                    bb::copy_segment_contexts(g, dst, src, n);
                    0
                },
            );
            // a fresh destination: the copy above wrote the first words
            let dst = if dst == 0 {
                0
            } else {
                t.random(&mut rng, 0x24 * 4)
            };
            t.check(
                "lod levels",
                0x005a_0f80,
                Conv::Cdecl,
                &[dst, src, n],
                0,
                |g| {
                    bb::copy_lod_levels(g, dst, src, n);
                    0
                },
            );
            t.check(
                "lod defaults",
                0x005a_6280,
                Conv::Cdecl,
                &[dst, n],
                0,
                |g| {
                    bb::default_lod_levels(g, dst, n);
                    0
                },
            );
        }
    }
}

#[test]
fn sharrays_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d104);
        let cell = t.zeros(4);
        let fresh = t.zeros(0x400);
        t.stub(cb::PHYRE_MALLOC, false, 2, 0, Ret::Cell(cell));
        t.stub(cb::PHYRE_FREE, false, 1, 0, Ret::Imm(0x0f7e_e000));
        t.stub(cb::COPY12, false, 3, 0, Ret::Imm(0x1030_0000));
        t.stub(cb::STRING_COPY, true, 1, 1, Ret::Imm(0xae10_0000));
        t.stub(cb::RELEASE_TAGGED_WORDS, false, 2, 0, Ret::Imm(0x4360_0000));
        for case in 0..48u32 {
            let src = array_header(&mut t, &mut rng, 0x40);
            t.w32(src, rng.below(4) | (rng.next() & 0x8000_0000));
            let this = array_header(&mut t, &mut rng, 0x40);
            t.w32(cell, if case % 5 == 0 { 0 } else { fresh });
            t.check(
                "sharray12 copy",
                0x005a_2440,
                Conv::Thiscall(this),
                &[src],
                ALL,
                |g| bb::sharray12_copy(g, this, src),
            );
            // PSharray<PString>: one element (inline, owned or borrowed
            // text), several, or a negative count; one element before in
            // half the cases
            let this = array_header(&mut t, &mut rng, 0x40);
            let count = if case % 2 == 0 {
                1 | (rng.next() & 0x8000_0000)
            } else {
                count_word(&mut rng) & 0xc000_0003
            };
            t.w32(this, count);
            let n = match case % 6 {
                0 | 3 => 1,
                1 => 0x8000_0002,
                5 => u32::MAX,
                _ => rng.below(4),
            };
            let data = if case % 3 == 0 {
                this + 4
            } else {
                t.random(&mut rng, 0x10)
            };
            t.check(
                "string sharray set",
                0x0044_c960,
                Conv::Thiscall(this),
                &[data, n],
                ALL,
                |g| bb::string_sharray_set(g, this, data, n),
            );
        }
        // the assign: the inline word with one element and a descriptor,
        // or anything else
        let desc = t.zeros(4);
        let vt_slot = t.stub_code(0x1048_0004, true, 0, 0, Ret::Cell(desc));
        let vt = t.alloc(&[[0u8; 4], vt_slot.to_le_bytes()].concat());
        let g = t.rebase(cb::STRING_SHARRAY_TYPE);
        t.w32(g, vt);
        t.stub(cb::APPLY_DEFAULT_BUFFER, true, 2, 2, Ret::Imm(0xc160_0000));
        t.stub(cb::STRING_SHARRAY_SET, true, 2, 2, Ret::Imm(0xc960_0000));
        for case in 0..12u32 {
            let this = t.random(&mut rng, 0x10);
            t.w32(desc, if case % 3 == 0 { 0 } else { 0x00d0_0d00 });
            let data = if case % 2 == 0 { this + 4 } else { rng.next() };
            let n = if case % 4 < 2 { 1 } else { rng.below(3) };
            t.check(
                "string sharray assign",
                0x0044_ca80,
                Conv::Thiscall(this),
                &[data, n],
                ALL,
                |g| bb::string_sharray_assign(g, this, data, n),
            );
        }
    }
}

#[test]
fn builders_and_records_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d105);
        let cell = t.zeros(4);
        let fresh = t.zeros(0x400);
        t.stub(cb::PHYRE_MALLOC, false, 2, 0, Ret::Cell(cell));
        t.stub(cb::PHYRE_FREE, false, 1, 0, Ret::Imm(0x0f7e_e000));
        // the builder from a text (the real reserve on the stubbed allocator)
        for case in 0..8u32 {
            let this = t.random(&mut rng, 0x10);
            let s = match case % 4 {
                0 => 0,
                1 => t.alloc(b"\0"),
                _ => t.alloc(b"PhyreEngine builder\0"),
            };
            t.w32(cell, if case % 3 == 0 { 0 } else { fresh });
            t.check(
                "builder",
                0x0044_49e0,
                Conv::Thiscall(this),
                &[s],
                ALL,
                |g| bb::builder_new(g, this, s),
            );
        }
        // += int (the CRT's sprintf_s, the real append) into room enough
        for v in [0u32, 1, 42, 0xffff_ffff, 0x8000_0000, 1_234_567_890] {
            let this = t.zeros(0x10);
            let buf = t.zeros(0x100);
            t.w32(this, 0x100);
            t.w32(this + 4, buf);
            let local = t.b.stack_range().start + 0x200;
            t.check(
                "append int",
                0x0044_4b90,
                Conv::Thiscall(this),
                &[v],
                ALL,
                |g| bb::builder_append_int(g, this, v, local),
            );
        }
        // the record's arrays
        t.stub(cb::RELEASE_THREE, true, 0, 0, Ret::Imm(0x9930_0000));
        for _ in 0..10 {
            let this = t.random(&mut rng, 0x40);
            t.w32(this + 4, count_word(&mut rng));
            if rng.chance(3) {
                t.w32(this + 8, 0);
            }
            t.w32(this + 0x30, count_word(&mut rng));
            t.check(
                "record dtor",
                0x0043_1bf0,
                Conv::Thiscall(this),
                &[],
                ALL,
                |g| bb::record_dtor(g, this),
            );
        }
        // the pool's destructor body: records or none
        t.stub(cb::THREAD_JOIN, true, 0, 1, Ret::Imm(0x01b0_0000));
        t.stub(cb::THREAD_CLOSE, true, 0, 0, Ret::Imm(1));
        t.stub(cb::VECTOR_DTOR, false, 4, 4, Ret::Imm(0x96d2_0000));
        for case in 0..6u32 {
            let this = t.random(&mut rng, 0x20);
            let n = rng.below(4);
            let recs = if case % 3 == 0 {
                0
            } else {
                let block = t.random(&mut rng, 4 + 0x18 * 4);
                t.w32(block, n);
                block + 4
            };
            t.w32(this + 4, recs);
            t.w32(this + 8, n);
            let local = t.b.stack_range().start + 0x80;
            t.check(
                "pool dtor body",
                0x0044_d070,
                Conv::Thiscall(this),
                &[],
                ALL,
                |g| bb::pool_dtor_body(g, this, local),
            );
        }
        // the 3x4 identity through the real PMatrix4_toWorldMatrix3x4
        for _ in 0..2 {
            let m = t.random(&mut rng, 0x30);
            let local = t.b.stack_range().start + 0x200;
            t.check(
                "identity 3x4",
                0x0045_0ee0,
                Conv::Thiscall(m),
                &[],
                ALL,
                |g| bb::identity_3x4(g, m, local),
            );
        }
    }
}

// ---- batch 3 (ffx_pcore::m5p_core_c) ----

use ffx_pcore::m5p_core_c::{self as cc, bridge as bc};

/// A bump allocator in place of `PhyreMalloc(size, align)`: 16-byte
/// steps from the block in `cell`; `left` allocations succeed, then 0
/// (`u32::MAX`: always).
fn bump_malloc(t: &mut Twins, size: usize) -> (u32, u32) {
    // junk in the pool: the callers' own stores must show
    let junk: Vec<u8> = (0..size).map(|i| (i * 37 + 11) as u8).collect();
    let pool = t.alloc(&junk);
    let cell = t.alloc(&pool.to_le_bytes());
    let left = t.alloc(&u32::MAX.to_le_bytes());
    let mut c = vec![0xa1];
    c.extend(left.to_le_bytes()); // mov eax, [left]
    c.extend([0x85, 0xc0, 0x74, 0x1c]); // test eax, eax; je fail
    c.extend([0xff, 0x0d]);
    c.extend(left.to_le_bytes()); // dec dword [left]
    c.push(0xa1);
    c.extend(cell.to_le_bytes()); // mov eax, [cell]
    c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
    c.extend([0x83, 0xc2, 0x0f, 0x83, 0xe2, 0xf0]); // round up to 16
    c.extend([0x01, 0x15]);
    c.extend(cell.to_le_bytes()); // add [cell], edx
    c.push(0xc3);
    c.extend([0x31, 0xc0, 0xc3]); // fail: xor eax, eax; ret
    let code = t.code(&c);
    // logged (size and alignment) before it allocates
    let rec = t.stub_code(cc::PHYRE_MALLOC, false, 2, 0, Ret::Imm(0));
    let mut l = vec![0xff, 0x74, 0x24, 0x08, 0xff, 0x74, 0x24, 0x08]; // push [esp+8] twice: the words
    l.push(0xe8);
    l.extend([0; 4]); // call rec (patched)
    l.extend([0x83, 0xc4, 0x08]); // add esp, 8
    l.extend([0x90; 5]); // jmp code (patched)
    let front = t.code(&l);
    let rel = rec.wrapping_sub(front + 13);
    t.write(front + 9, &rel.to_le_bytes());
    t.jump(front + 16, code);
    let at = t.rebase(cc::PHYRE_MALLOC);
    t.jump(at, front);
    (cell, left)
}

#[test]
fn lookups_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d201);
        // the tree lookup by name over real trees (the successor copy runs)
        let names: Vec<u32> = ["", "alpha", "beta", "gamma", "alphabet"]
            .iter()
            .map(|s| {
                let mut v = s.as_bytes().to_vec();
                v.push(0);
                t.alloc(&v)
            })
            .collect();
        for _ in 0..16 {
            let n = rng.below(7) as usize;
            let tr = t.zeros(0x20);
            if n == 0 {
                t.w32(tr + 0xc, tr);
            } else {
                let nodes = build_tree(&mut t, &mut rng, tr, n);
                for &x in &nodes {
                    let obj = if rng.chance(4) {
                        0
                    } else {
                        let o = t.random(&mut rng, 0x20);
                        t.w32(o + 0x18, names[rng.below(5) as usize]);
                        o
                    };
                    t.w32(x + 0xc, obj);
                }
            }
            for k in 0..3 {
                let name = if k == 0 {
                    0
                } else {
                    names[rng.below(5) as usize]
                };
                t.check(
                    "find by name",
                    0x0047_d330,
                    Conv::Thiscall(tr),
                    &[name],
                    ALL,
                    |g| bc::find_by_name(g, tr, name),
                );
            }
        }
        // 16-bit indices widened through the real resize on a stubbed allocator
        let (cell, left) = bump_malloc(&mut t, 0x4000);
        t.stub(c::PHYRE_FREE, false, 1, 0, Ret::Imm(0x0f7e_e000));
        for case in 0..16u32 {
            let this = t.random(&mut rng, 0x20);
            let n = rng.below(6);
            t.w32(this, n);
            let fmt = match case % 4 {
                0 => 8,
                1 => rng.next() as u8,
                _ => 0xc,
            };
            t.w8(this + 0x19, fmt);
            let src = t.random(&mut rng, 16);
            t.w32(this + 8, src);
            let out = if case % 5 == 0 { 0 } else { t.zeros(8) };
            t.w32(left, if case % 7 == 3 { 0 } else { u32::MAX });
            t.check(
                "indices32",
                0x0049_bd10,
                Conv::Thiscall(this),
                &[out],
                ALL,
                |g| bc::indices32(g, this, out),
            );
        }
        let _ = cell;
        // free-list merges: chains of 0..3 nodes, sizes equal or not
        for case in 0..8u32 {
            let this = t.random(&mut rng, 0x20);
            let other = t.random(&mut rng, 0x20);
            let size = rng.next();
            t.w32(this + 4, size);
            t.w32(other + 4, if case % 4 == 0 { size ^ 1 } else { size });
            for at in [0u32, 0x14] {
                for list in [this, other] {
                    let mut head = 0u32;
                    for _ in 0..rng.below(4) {
                        let node = t.alloc(&head.to_le_bytes());
                        head = node;
                    }
                    t.w32(list + at, head);
                }
            }
            t.check(
                "free list merge",
                0x0049_c730,
                Conv::Thiscall(this),
                &[other],
                ALL,
                |g| bc::free_list_merge(g, this, other),
            );
        }
    }
}

#[test]
fn shader_passes_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d202);
        let cell = t.zeros(4);
        let fresh = t.zeros(0x400);
        t.stub(cc::PHYRE_MALLOC, false, 2, 0, Ret::Cell(cell));
        t.stub(cc::COPY2, false, 3, 0, Ret::Imm(0x2d00_0000));
        t.stub(cc::COPY16, false, 3, 0, Ret::Imm(0x2d70_0000));
        for case in 0..12u32 {
            let src = t.random(&mut rng, 0x18);
            t.w32(src + 0x10, count_word(&mut rng));
            let this = t.random(&mut rng, 0x18);
            t.w32(cell, if case % 4 == 0 { 0 } else { fresh });
            t.check(
                "pass array2",
                0x004b_bc30,
                Conv::Thiscall(this),
                &[src],
                ALL,
                |g| bc::pass_array_copy(g, this, src, 2, 2, cc::COPY2),
            );
            t.check(
                "pass array16",
                0x004b_bdc0,
                Conv::Thiscall(this),
                &[src],
                ALL,
                |g| bc::pass_array_copy(g, this, src, 16, 4, cc::COPY16),
            );
        }
        t.stub(cc::PASS_ARRAY2_COPY, true, 1, 1, Ret::Imm(0xbc30_0000));
        t.stub(cc::PASS_ARRAY16_COPY, true, 1, 1, Ret::Imm(0xbdc0_0000));
        for _ in 0..3 {
            let src = t.random(&mut rng, 0x254);
            let this = t.random(&mut rng, 0x254);
            t.check(
                "shader pass",
                0x004b_bcb0,
                Conv::Thiscall(this),
                &[src],
                ALL,
                |g| bc::shader_pass_copy(g, this, src),
            );
        }
    }
}

#[test]
fn evaluators_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d203);
        // the LOD level: no level, below or at the cap, no levels
        for case in 0..16u32 {
            let shader = t.random(&mut rng, 0x40);
            t.w32(shader + 0x3c, rng.below(4));
            let segment = t.random(&mut rng, 0x24);
            if case % 4 == 0 {
                t.w32(segment + 0x20, 0);
            } else {
                let q = t.random(&mut rng, 0x34);
                t.w32(q + 0x30, rng.below(5));
                let p = t.alloc(&q.to_le_bytes());
                t.w32(segment + 0x20, p);
            }
            let cursor = t.zeros(4);
            let at = if case == 5 {
                cursor
            } else {
                t.random(&mut rng, 8)
            };
            t.w32(cursor, at);
            let args = [
                shader,
                segment,
                rng.next(),
                cursor,
                rng.next(),
                rng.next(),
                rng.next(),
            ];
            t.check("lod level", 0x004d_4430, Conv::Stdcall, &args, ALL, |g| {
                bc::lod_level(g, shader, segment, cursor)
            });
        }
        // the orthographic camera test: no pointer, no camera, the float
        for case in 0..40u32 {
            let camera = match case % 8 {
                0 => 0,
                1 => t.zeros(4),
                _ => {
                    let cam = t.random(&mut rng, 0x70);
                    let w = if case % 3 == 0 {
                        t.r32(t.rebase(cc::ORTHO_THRESHOLD))
                    } else if case % 8 == 7 {
                        // unordered counts as at least
                        0x7fc0_1234 | (rng.next() & 0x8000_0000)
                    } else {
                        float_word(&mut rng)
                    };
                    t.w32(cam + 0x6c, w);
                    t.alloc(&cam.to_le_bytes())
                }
            };
            let cursor = t.zeros(4);
            let at = t.random(&mut rng, 8);
            t.w32(cursor, at);
            let args = [
                rng.next(),
                rng.next(),
                rng.next(),
                cursor,
                rng.next(),
                camera,
            ];
            t.check(
                "ortho camera",
                0x004d_4480,
                Conv::Stdcall,
                &args,
                ALL,
                |g| bc::ortho_camera(g, cursor, camera),
            );
        }
        // the LOD level's per-segment arrays
        let (cell, left) = bump_malloc(&mut t, 0x8000);
        t.stub(cc::PHYRE_FREE, false, 1, 0, Ret::Imm(0x0f7e_e000));
        t.stub(
            cc::SEGMENT_ARRAYS_RELEASE,
            false,
            2,
            0,
            Ret::Imm(0x3720_0000),
        );
        for case in 0..32u32 {
            let shader = t.random(&mut rng, 0x40);
            let n = rng.below(4);
            t.w32(shader + 0x3c, n);
            let arrays = t.zeros(8);
            match case % 4 {
                0 => {}
                1 => {
                    // already n headers, some with one word
                    let block = t.zeros(8 * 4);
                    for k in 0..n {
                        let e = block + 8 * k;
                        if rng.chance(3) {
                            t.w32(e, 1);
                            let w = t.random(&mut rng, 4);
                            t.w32(e + 4, w);
                        } else {
                            // not exactly 1 (one word, read as the index slot)
                            let c = count_word(&mut rng);
                            let c = if c == 1 { 2 } else { c };
                            // external every other one: kept, not freed
                            t.w32(e, if k % 2 == 1 { c | 0x8000_0000 } else { c });
                            let w = if rng.chance(3) {
                                0
                            } else {
                                t.random(&mut rng, 4)
                            };
                            t.w32(e + 4, w);
                        }
                    }
                    t.w32(arrays, n);
                    t.w32(arrays + 4, block);
                }
                _ => {
                    t.w32(arrays, count_word(&mut rng));
                    let d = t.random(&mut rng, 8);
                    t.w32(arrays + 4, d);
                }
            }
            t.w32(
                left,
                if case % 4 == 3 {
                    case / 4 % 3
                } else {
                    u32::MAX
                },
            );
            t.check(
                "segment arrays",
                0x004d_5400,
                Conv::Stdcall,
                &[shader, arrays],
                ALL,
                |g| bc::lod_segment_arrays(g, shader, arrays),
            );
        }
        let _ = cell;
        // the light-type define, built by the real PStringBuilder members
        t.stub(cc::RELEASE_TAGGED_WORDS, false, 2, 0, Ret::Imm(0x4360_0000));
        let names: Vec<u32> = ["LIGHT", "", "SPOT_SHADOW"]
            .iter()
            .map(|s| {
                let mut v = s.as_bytes().to_vec();
                v.push(0);
                t.alloc(&v)
            })
            .collect();
        for case in 0..24u32 {
            let this = t.random(&mut rng, 8);
            t.w32(this + 4, names[case as usize % 3]);
            let values = t.random(&mut rng, 8);
            if case % 4 == 0 {
                t.w32(values, 0);
            }
            let cursor = t.alloc(&values.to_le_bytes());
            let defines = t.zeros(8);
            if case % 3 == 1 {
                t.w32(defines, 1);
                let s = t.zeros(4);
                t.w32(
                    s,
                    if case % 2 == 0 {
                        0x0bad_0001
                    } else {
                        0x0bad_0000
                    },
                );
                t.w32(defines + 4, s);
            } else {
                t.w32(defines, count_word(&mut rng));
                let d = t.random(&mut rng, 8);
                t.w32(defines + 4, d);
            }
            // the defines word or the final copy fails (the builder's own
            // allocation failing would crash the original too)
            let needs = t.r32(defines) != 1;
            let fail = match (case % 4, case / 4 % 2, needs) {
                (2, 0, true) => 0,
                (2, _, false) => 1,
                (2, _, true) => 2,
                _ => u32::MAX,
            };
            t.w32(left, fail);
            let local = t.b.stack_range().start + 0x300;
            let a1 = rng.next();
            t.check(
                "light define",
                0x004d_46a0,
                Conv::Thiscall(this),
                &[cursor, a1, defines],
                ALL,
                |g| bc::light_define(g, this, cursor, defines, local),
            );
        }
    }
}

#[test]
fn registers_and_validations_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d204);
        t.stub(cc::CLASS_REGISTER, true, 2, 2, Ret::Imm(0x3d6a_0000));
        t.stub(cc::ATEXIT, false, 1, 0, Ret::Imm(0));
        let mut ctors: Vec<u32> = Vec::new();
        for &(_, l) in cc::LAZY {
            if !ctors.contains(&l.ctor) {
                ctors.push(l.ctor);
                t.stub(l.ctor, true, 0, 0, Ret::Imm(0x0c70_0000));
            }
        }
        for &(f, l) in cc::LAZY {
            assert_eq!(format!("{:?}", cc::lazy_of(f)), format!("{l:?}"));
            for first in [true, false] {
                let guard = t.rebase(l.guard);
                let g = t.r32(guard);
                t.w32(guard, if first { g & !1 } else { g | 1 });
                // flag bits the build clears (bit 1 only)
                let flags = t.rebase(l.descriptor) + 0x90;
                t.w32(flags, rng.next() | 3);
                let p = rng.next();
                t.check("lazy register", f, Conv::Cdecl, &[p], ALL, |g| {
                    bc::lazy_register(g, p, l)
                });
            }
        }
        // the external buffer with its destroy
        t.stub(cc::SLOT_FILTER_FREE, false, 2, 0, Ret::Imm(0x8a30_0000));
        for case in 0..12u32 {
            let this = array_header(&mut t, &mut rng, 0x20);
            let data = if case % 3 == 0 {
                t.r32(this + 4)
            } else {
                rng.next()
            };
            let n = if case % 4 == 0 {
                0x8000_0001
            } else {
                rng.below(9)
            };
            t.check(
                "set external",
                0x0052_ab30,
                Conv::Thiscall(this),
                &[data, n],
                ALL,
                |g| bc::slot_filters_set_external(g, this, data, n),
            );
        }
        // the two-stage validation: each check yes or no, with or without
        // a target
        let yes = t.zeros(4);
        for case in 0..16u32 {
            let code1 = t.stub_code(0x531b_0004, true, 2, 2, Ret::Cell(yes));
            let code3 = t.stub_code(0x531b_000c, true, 2, 2, Ret::Imm(0x5555_5502));
            let vt = t.alloc(
                &[0, code1, 0, code3]
                    .iter()
                    .flat_map(|w: &u32| w.to_le_bytes())
                    .collect::<Vec<u8>>(),
            );
            let a = t.random(&mut rng, 8);
            t.w32(a, vt);
            let this = t.random(&mut rng, 0x10);
            let target = if case % 2 == 0 {
                0
            } else {
                let tc = t.stub_code(
                    0x531b_100c,
                    true,
                    2,
                    2,
                    Ret::Imm(0x6666_6600 | (case >> 2 & 1)),
                );
                let tvt = t.alloc(
                    &[0, 0, 0, tc]
                        .iter()
                        .flat_map(|w: &u32| w.to_le_bytes())
                        .collect::<Vec<u8>>(),
                );
                let o = t.random(&mut rng, 8);
                t.w32(o, tvt);
                o
            };
            t.w32(this + 0xc, target);
            t.w32(yes, 0x7777_7700 | (case >> 1 & 1));
            let v = rng.next();
            t.check(
                "validate",
                0x0053_1b40,
                Conv::Thiscall(this),
                &[a, v],
                ALL,
                |g| bc::validate_pair(g, this, a, v),
            );
        }
    }
}

#[test]
fn animatable_elements_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d205);
        // the element key: its first word, through a stub that reads it
        let mut key_code = vec![0x8b, 0x01]; // mov eax, [ecx]
        key_code.push(0xc3);
        let key_fn = t.code(&key_code);
        let at = t.rebase(cc::ELEMENT_KEY);
        t.jump(at, key_fn);
        t.stub(cc::ELEMENT_SET, true, 1, 1, Ret::Imm(0x69e0_0000));
        t.stub(cc::ELEMENT_GET, true, 0, 0, Ret::Imm(0x6a20_0000));
        t.stub(cc::ELEMENT_SET_B, true, 1, 1, Ret::Imm(0x6a40_0000));
        for case in 0..12u32 {
            let n = rng.below(4);
            let elems = t.random(&mut rng, (0x11c * 4) as usize);
            for k in 0..n {
                t.w32(elems + 0x11c * k, 100 + k);
            }
            let this = t.zeros(8);
            t.w32(this, elems);
            t.w32(this + 4, n);
            let key = if case % 3 == 0 { 7 } else { 100 + rng.below(4) };
            let v = rng.next();
            t.check(
                "set by key",
                0x005c_d660,
                Conv::Thiscall(this),
                &[key, v],
                ALL,
                |g| bc::element_set(g, this, key, v),
            );
            t.check(
                "get by key",
                0x005c_d6b0,
                Conv::Thiscall(this),
                &[key],
                ALL,
                |g| bc::element_get(g, this, key),
            );
            t.check(
                "set by key b",
                0x005c_d700,
                Conv::Thiscall(this),
                &[key, v],
                ALL,
                |g| bc::element_set_b(g, this, key, v),
            );
        }
        // the element destructor's three arrays
        t.stub(cc::PHYRE_FREE, false, 1, 0, Ret::Imm(0x0f7e_e000));
        for _ in 0..12 {
            let e = t.random(&mut rng, 0x11c);
            for off in [0x100u32, 0x108, 0x110] {
                t.w32(e + off, count_word(&mut rng));
                if rng.chance(3) {
                    t.w32(e + off + 4, 0);
                }
            }
            t.check(
                "element dtor",
                0x005c_d4c0,
                Conv::Thiscall(e),
                &[],
                ALL,
                |g| bc::element_dtor(g, e),
            );
        }
        // the key hash: a name (tagged or not) or none
        let names: Vec<u32> = ["", "Bip01", "root_joint", "\x7f\x01zz"]
            .iter()
            .map(|s| {
                let mut v = s.as_bytes().to_vec();
                v.push(0);
                t.alloc(&v)
            })
            .collect();
        let keys: Vec<u32> = (0..8)
            .map(|k| {
                let key = t.random(&mut rng, 0x1c);
                let s = match k % 4 {
                    0 => 0,
                    1 => 1,
                    _ => names[rng.below(4) as usize] | (rng.next() & 1),
                };
                t.w32(key + 0x14, s);
                key
            })
            .collect();
        for &key in &keys {
            t.check("key hash", 0x005c_cd70, Conv::Cdecl, &[key], ALL, |g| {
                bc::key_hash(g, key)
            });
        }
        assert_eq!(cc::name_hash(b""), 0x7b5);
        // the map lookup: no map (the owner's fallback), found or not; the
        // stub copies the key it gets into `seen`
        let seen = t.zeros(0x20);
        let found = t.zeros(4);
        let rec = t.stub_code(cc::MAP_FIND, true, 0, 1, Ret::Cell(found));
        let mut c = vec![0x51, 0x56, 0x57]; // push ecx; push esi; push edi
        c.extend([0x8b, 0x74, 0x24, 0x10]); // mov esi, [esp+0x10]
        c.push(0xbf);
        c.extend(seen.to_le_bytes()); // mov edi, seen
        c.extend([0xb9, 8, 0, 0, 0, 0xf3, 0xa5]); // mov ecx, 8; rep movsd
        c.extend([0x5f, 0x5e, 0x59]); // pop edi; pop esi; pop ecx
        c.extend([0x90; 5]);
        let code = t.code(&c);
        t.jump(code + c.len() as u32 - 5, rec);
        let at = t.rebase(cc::MAP_FIND);
        t.jump(at, code);
        t.stub(cc::KEY_FALLBACK, true, 1, 1, Ret::Imm(0xcbb0_0000));
        for (i, &key) in keys.iter().enumerate() {
            let this = t.random(&mut rng, 0x10);
            let base = rng.next() & !3;
            t.w32(this + 4, if i % 4 == 0 { 0 } else { base });
            let entry = match i % 3 {
                0 => 0,
                1 => base.wrapping_add(0x2c * rng.below(9)),
                _ => base.wrapping_sub(0x2c * rng.below(9)),
            };
            t.w32(found, entry);
            let local = t.b.stack_range().start + 0x300;
            t.check(
                "key index",
                0x005c_ce50,
                Conv::Thiscall(this),
                &[key],
                ALL,
                |g| bc::key_index(g, this, key, local),
            );
        }
        // the rigid body's other forward
        let other = t.rebase(cc::RIGID_BODY_OTHER_CLASS);
        let want = t.rebase(cc::RIGID_BODY_CLASS);
        for case in 0..16u32 {
            let this = t.random(&mut rng, 0x10);
            let mut chain = 0;
            for k in 0..rng.below(4) {
                let cls = match (k, case % 4) {
                    (0, 1) => other,
                    (0, 2) => want,
                    _ => t.random(&mut rng, 0x44),
                };
                if cls != other && cls != want {
                    t.w32(cls + 0x40, chain);
                }
                chain = cls;
            }
            t.w32(this + 8, chain);
            let obj = t.object(&mut rng, 0x7005_0000, &[0, 0, 0, 0, 0, 0, 0], 0x10);
            let p = match case % 5 {
                0 => 0,
                1 => 4,
                _ => obj + 4,
            };
            t.w32(this + 4, p);
            t.check(
                "rigid body 18",
                0x005d_3e30,
                Conv::Thiscall(this),
                &[],
                ALL,
                |g| bc::rigid_body_slot6(g, this),
            );
        }
    }
}

// ---- batch 4 (ffx_pcore::m5p_core_d) ----

use ffx_pcore::m5p_core_d::{self as cd, bridge as bd};

/// `n` floats from [`float_word`] in a fresh block.
fn floats(t: &mut Twins, rng: &mut Rng, n: usize) -> u32 {
    let v: Vec<u8> = (0..n).flat_map(|_| float_word(rng).to_le_bytes()).collect();
    t.alloc(&v)
}

/// Ordinary floats (no NaN or infinity): angles and matrices the real
/// callees take.
fn plain_floats(t: &mut Twins, rng: &mut Rng, n: usize) -> u32 {
    let v: Vec<u8> = (0..n)
        .flat_map(|_| (f32::from((rng.next() % 4000) as u16) / 500.0 - 4.0).to_le_bytes())
        .collect();
    t.alloc(&v)
}

#[test]
fn matrices_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d301);
        // the look-at through the real 4x4 helpers
        for _ in 0..12 {
            let this = plain_floats(&mut t, &mut rng, 12);
            let m = plain_floats(&mut t, &mut rng, 12);
            let local = t.b.stack_range().start + 0x300;
            t.check(
                "world look at",
                0x0045_2520,
                Conv::Thiscall(this),
                &[m],
                ALL,
                |g| bd::world_look_at(g, this, m, local),
            );
        }
        for _ in 0..8 {
            let this = t.random(&mut rng, 0x30);
            let m = floats(&mut t, &mut rng, 12);
            t.check(
                "copy 3x4",
                0x0045_2740,
                Conv::Thiscall(this),
                &[m],
                ALL,
                |g| bd::copy_3x4(g, this, m),
            );
        }
        // the Euler matrix through the CRT's sine and cosine
        for _ in 0..24 {
            let out = t.random(&mut rng, 0x40);
            let angles = floats(&mut t, &mut rng, 3);
            t.check(
                "euler",
                0x0057_4520,
                Conv::Cdecl,
                &[out, angles],
                ALL,
                |g| bd::euler_matrix(g, out, angles),
            );
        }
        // the lerp: every float kind, the output over the first input once
        for case in 0..60u32 {
            let a = floats(&mut t, &mut rng, 3);
            let b = floats(&mut t, &mut rng, 3);
            let out = if case % 10 == 0 {
                a
            } else {
                t.random(&mut rng, 12)
            };
            let tt = float_word(&mut rng);
            t.check(
                "lerp3",
                0x006a_9b40,
                Conv::Cdecl,
                &[out, tt, a, b],
                ALL,
                |g| bd::lerp3(g, out, tt, a, b),
            );
        }
    }
}

#[test]
fn small_utilities_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d302);
        // the shadow caster types' list: first, middle, last, absent, empty
        let head = t.rebase(cd::SHADOW_CASTER_TYPES);
        let saved = t.r32(head);
        for case in 0..10u32 {
            let n = rng.below(4);
            let nodes: Vec<u32> = (0..n).map(|_| t.random(&mut rng, 0x10)).collect();
            for (i, &x) in nodes.iter().enumerate() {
                t.w32(x + 8, nodes.get(i + 1).copied().unwrap_or(0));
            }
            t.w32(head, nodes.first().copied().unwrap_or(0));
            let ty = if n > 0 && case % 3 != 0 {
                nodes[rng.below(n) as usize]
            } else {
                t.random(&mut rng, 0x10)
            };
            t.check(
                "shadow type unlink",
                0x004c_c050,
                Conv::Cdecl,
                &[ty],
                ALL,
                |g| bd::shadow_type_unlink(g, ty),
            );
        }
        t.w32(head, saved);
        // the node count over real trees (the successor copy runs)
        for _ in 0..12 {
            let n = if rng.chance(4) {
                0
            } else {
                rng.below(9) as usize
            };
            let tr = t.zeros(0x20);
            if n == 0 {
                t.w32(tr + 0xc, tr);
            } else {
                build_tree(&mut t, &mut rng, tr, n);
            }
            t.check(
                "count nodes",
                0x0057_4860,
                Conv::Thiscall(tr),
                &[],
                ALL,
                |g| bd::count_nodes(g, tr),
            );
        }
        // byte swaps
        for _ in 0..6 {
            let p = t.random(&mut rng, 16);
            t.check("swap4", 0x0057_6c70, Conv::Cdecl, &[p], ALL, |g| {
                bd::swap4(g, p)
            });
            t.check("swap3", 0x0057_6d20, Conv::Cdecl, &[p], ALL, |g| {
                bd::swap3(g, p)
            });
        }
        // the name reset: owned or borrowed name
        t.stub(cd::PBASE_DESCRIPTOR, false, 0, 0, Ret::Imm(0x00c0_b000));
        t.stub(cd::PHYRE_FREE, false, 1, 0, Ret::Imm(0x0f7e_e000));
        for case in 0..6u32 {
            let this = t.random(&mut rng, 0x68);
            let name = rng.next() & !1 | (case & 1);
            t.w32(this + 0x54, name);
            t.check(
                "reset names",
                0x0055_afc0,
                Conv::Thiscall(this),
                &[],
                ALL,
                |g| bd::reset_names(g, this),
            );
        }
        // the float pairs set by key
        for case in 0..10u32 {
            let n = rng.below(5);
            let pairs = t.random(&mut rng, 8 * 5);
            let key = rng.next();
            for i in 0..n {
                let obj = if rng.chance(4) {
                    0
                } else {
                    let o = t.random(&mut rng, 0x60);
                    if rng.chance(2) {
                        t.w32(o + 0x58, key);
                    }
                    o
                };
                t.w32(pairs + 8 * i, obj);
            }
            let this = t.random(&mut rng, 0x10);
            t.w32(this + 8, n | (case & 1) << 31);
            t.w32(this + 0xc, pairs);
            let f = float_word(&mut rng);
            t.check(
                "set by key",
                0x006a_c0b0,
                Conv::Thiscall(this),
                &[key, f],
                ALL,
                |g| bd::set_by_key(g, this, key, f),
            );
        }
        // the PSharray resize through 0x0058c590
        let cell = t.zeros(4);
        let fresh = t.zeros(0x100);
        t.stub(cd::SHARRAY_REALLOC, true, 3, 3, Ret::Cell(cell));
        for case in 0..30u32 {
            let this = t.random(&mut rng, 0x10);
            t.w32(
                this,
                if case % 7 == 3 {
                    1
                } else {
                    count_word(&mut rng)
                },
            );
            if case % 4 == 0 {
                t.w32(this + 4, 0);
            }
            let n = match case % 5 {
                0 => t.r32(this),
                1 => 0,
                _ => rng.below(6),
            };
            let block = match case % 6 {
                0 => 0,
                1 => this + 4,
                2 => t.r32(this + 4),
                _ => fresh,
            };
            t.w32(cell, block);
            t.check(
                "sharray resize",
                0x0059_9540,
                Conv::Thiscall(this),
                &[n],
                ALL,
                |g| bd::sharray_resize(g, this, n),
            );
        }
    }
}

#[test]
fn lod_groups_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d303);
        // a group of levels, each with a PSharray of 12-byte elements and
        // one of objects with targets
        let group = |t: &mut Twins, rng: &mut Rng| {
            let n = rng.below(4);
            let levels = t.random(rng, (0x24 * 4) as usize);
            for k in 0..n {
                let lv = levels + 0x24 * k;
                let m = rng.below(4);
                t.w32(lv + 0x14, m | (rng.next() & 0x8000_0000));
                if m > 1 {
                    let s = t.random(rng, (12 * m) as usize);
                    t.w32(lv + 0x18, s);
                }
                let c = rng.below(4);
                t.w32(lv + 4, c | (rng.next() & 0x8000_0000));
                let objs: Vec<u32> = (0..c)
                    .map(|_| {
                        if rng.chance(4) {
                            0
                        } else {
                            let o = t.random(rng, 0x20);
                            let tg = if rng.chance(3) {
                                0
                            } else {
                                t.random(rng, 0x20)
                            };
                            t.w32(o + 0x1c, tg);
                            o
                        }
                    })
                    .collect();
                if c == 1 {
                    t.w32(lv + 8, objs[0]);
                } else if c > 1 {
                    let v: Vec<u8> = objs.iter().flat_map(|o| o.to_le_bytes()).collect();
                    let s = t.alloc(&v);
                    t.w32(lv + 8, s);
                }
            }
            let g = t.random(rng, 0x48);
            t.w32(g + 0x3c, n | (rng.next() & 0x8000_0000));
            t.w32(g + 0x40, levels);
            (g, levels, n)
        };
        for _ in 0..8 {
            let (g, levels, n) = group(&mut t, &mut rng);
            t.check(
                "levels reset",
                0x005a_7990,
                Conv::Thiscall(g),
                &[],
                ALL,
                |gg| bd::lod_levels_reset(gg, g),
            );
            for k in 0..n {
                let lv = levels + 0x24 * k;
                let flag = rng.next() & 0xffff_ff00 | rng.below(2);
                t.check(
                    "level owners",
                    0x005a_83a0,
                    Conv::Thiscall(lv),
                    &[flag],
                    0,
                    |gg| {
                        bd::lod_level_owners(gg, lv, flag);
                        0
                    },
                );
            }
        }
        for case in 0..12u32 {
            let (g, _, _) = group(&mut t, &mut rng);
            let cur = rng.below(2) as u8;
            t.w8(g + 0x44, cur);
            let flag = rng.next() & 0xffff_ff00 | (case & 1);
            t.check(
                "group enable",
                0x005a_7d10,
                Conv::Thiscall(g),
                &[flag],
                0,
                |gg| {
                    bd::lod_group_enable(gg, g, flag);
                    0
                },
            );
        }
    }
}

#[test]
fn character_lighting_matches_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d304);
        let coeff = t.code(&[
            0xd9, 0x05, 0x00, 0x00, 0x00, 0x00, // fld dword [k]
            0xc3,
        ]);
        let k = t.alloc(&0.625f32.to_bits().to_le_bytes());
        t.write(coeff + 2, &k.to_le_bytes());
        let at = t.rebase(cd::SHADE_COEFF);
        t.jump(at, coeff);
        t.stub(cd::SET_SHADE_COEFF, true, 2, 2, Ret::Imm(0xaef0_0000));
        // PArray_Data: the array's +4
        let data = t.code(&[0x8b, 0x41, 0x04, 0xc3]); // mov eax, [ecx+4]; ret
        let at = t.rebase(cd::ARRAY_DATA);
        t.jump(at, data);
        // the parameter lookup: the shader's word for each name, a table
        // [name, param] the stub walks: `mov eax,[ecx]` is the table
        let mut find = vec![0x8b, 0x01]; // mov eax, [ecx]
        find.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
        // loop: cmp [eax], 0; je none; cmp [eax], edx; je hit; add eax, 8; jmp loop
        find.extend([0x83, 0x38, 0x00, 0x74, 0x0f]);
        find.extend([0x39, 0x10, 0x74, 0x05]);
        find.extend([0x83, 0xc0, 0x08, 0xeb, 0xf2]);
        find.extend([0x8b, 0x40, 0x04, 0xc2, 0x04, 0x00]); // hit: mov eax, [eax+4]; ret 4
        find.extend([0x31, 0xc0, 0xc2, 0x04, 0x00]); // none: xor eax, eax; ret 4
        let find = t.code(&find);
        let at = t.rebase(cd::FIND_PARAMETER);
        t.jump(at, find);
        let names = [cd::ADD_LIGHT_COLOUR, cd::SHININESS, cd::MATERIAL_COLOUR].map(|n| t.rebase(n));
        for case in 0..32u32 {
            let n = rng.below(4);
            let insts: Vec<u32> = (0..n)
                .map(|_| {
                    // parameters: offsets and sizes in and out of range
                    let mut table: Vec<u32> = Vec::new();
                    for (j, &name) in names.iter().enumerate() {
                        if rng.chance(4) {
                            continue;
                        }
                        let size = match j {
                            0 => 16,
                            1 => 12,
                            _ => 16 - 4 * rng.below(2),
                        };
                        let off = match rng.below(6) {
                            0 => 0,
                            1 => 0x70,
                            // ending at the buffer's size word exactly
                            2 => 0x40 - size,
                            _ => 4 + 4 * rng.below(8),
                        };
                        let q = t.random(&mut rng, 12);
                        t.write(q + 8, &(off as u16).to_le_bytes());
                        let hi = (rng.next() as u16) & 0xe000;
                        t.write(q + 0xa, &(size as u16 | hi).to_le_bytes());
                        table.extend([name, q]);
                    }
                    table.extend([0, 0]);
                    let tb: Vec<u8> = table.iter().flat_map(|w| w.to_le_bytes()).collect();
                    let tb = t.alloc(&tb);
                    let shader = t.alloc(&tb.to_le_bytes());
                    let buf = t.random(&mut rng, 0x60);
                    t.w32(
                        buf,
                        if rng.chance(2) {
                            0x40
                        } else {
                            0x40 + 8 * rng.below(4)
                        },
                    );
                    t.alloc(&[shader.to_le_bytes(), buf.to_le_bytes()].concat())
                })
                .collect();
            let ptrs: Vec<u8> = insts.iter().flat_map(|w| w.to_le_bytes()).collect();
            let ptrs = t.alloc(&[ptrs, vec![0; 4]].concat());
            let arr = t.zeros(8);
            t.w32(arr, n | (rng.next() & 0x8000_0000));
            t.w32(arr + 4, ptrs);
            let inst = t.random(&mut rng, 0x20);
            t.w32(inst + 0x10, arr);
            let cl = t.random(&mut rng, 0x10);
            let rgb = [
                float_word(&mut rng),
                float_word(&mut rng),
                float_word(&mut rng),
            ];
            let intensity = float_word(&mut rng);
            let local = t.b.stack_range().start + 0x300;
            if case % 2 == 0 {
                let args = [inst, rgb[0], rgb[1], rgb[2], intensity];
                t.check(
                    "add light",
                    0x006a_9bb0,
                    Conv::Thiscall(cl),
                    &args,
                    0,
                    |g| {
                        bd::add_light(g, cl, inst, rgb, intensity, local);
                        0
                    },
                );
            } else {
                let c = [
                    float_word(&mut rng),
                    float_word(&mut rng),
                    float_word(&mut rng),
                    float_word(&mut rng),
                ];
                let args = [
                    inst, c[0], c[1], c[2], c[3], rgb[0], rgb[1], rgb[2], intensity,
                ];
                t.check(
                    "add light material",
                    0x006a_9cd0,
                    Conv::Thiscall(cl),
                    &args,
                    0,
                    |g| {
                        bd::add_light_material(g, cl, inst, c, rgb, intensity, local);
                        0
                    },
                );
            }
        }
    }
}

// ---- batch 5 (ffx_pcore::m5p_core_e) ----

use ffx_pcore::m5p_core_e::{self as ce, bridge as be};

/// A work-stealing pool with `workers` workers: random words, a vtable
/// whose slot `+0x18` is a recording stub, two job buffers of `cap`
/// words, the thread records (+4), range records per parity.
fn ws_pool(t: &mut Twins, rng: &mut Rng, workers: u32, cap: u32) -> u32 {
    let this = t.random(rng, 0x60);
    let slots: Vec<u32> = (0..8u32)
        .map(|i| t.stub_code(0x7d00_0000 + 4 * i, true, 0, 0, Ret::Imm(0x7d00_0000 | i)))
        .collect();
    let vt: Vec<u8> = slots.iter().flat_map(|s| s.to_le_bytes()).collect();
    let vt = t.alloc(&vt);
    t.w32(this, vt);
    t.w32(this + 8, workers);
    let threads = t.random(rng, (0x18 * 4) as usize);
    t.w32(this + 4, threads);
    t.w32(this + 0x14, cap);
    let a = t.random(rng, (4 * cap + 4) as usize);
    let b = t.random(rng, (4 * cap + 4) as usize);
    t.w32(this + 0x18, a);
    t.w32(this + 0x48, b);
    for p in 0..2 {
        let r = t.random(rng, (12 * 4) as usize);
        t.w32(this + 0x58 + 4 * p, r);
    }
    t.w32(this + 0x40, rng.below(9));
    t.w32(this + 0x4c, rng.below(cap + 1));
    // started flags: 0, 1 or another non-zero byte
    t.write(this + 0x54, &[rng.below(3) as u8, rng.below(3) as u8]);
    this
}

#[test]
fn work_stealing_pool_matches_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d401);
        t.stub(ce::SEMAPHORE_WAIT, true, 0, 0, Ret::Imm(0));
        t.stub(ce::SEMAPHORE_SIGNAL, true, 2, 2, Ret::Imm(0));
        for case in 0..80u32 {
            let workers = rng.below(4);
            let cap = 1 + rng.below(4);
            let this = ws_pool(&mut t, &mut rng, workers, cap);
            let job = t.random(&mut rng, 0x10);
            match case % 4 {
                0 => t.check(
                    "submit",
                    0x0057_d3e0,
                    Conv::Thiscall(this),
                    &[job],
                    ALL,
                    |g| be::submit(g, this, job),
                ),
                1 => t.check(
                    "try submit",
                    0x0057_d8d0,
                    Conv::Thiscall(this),
                    &[job],
                    ALL,
                    |g| be::try_submit(g, this, job),
                ),
                2 => {
                    // a job of this generation, one or two back, or older
                    let g = t.r32(this + 0x40);
                    t.w32(job + 8, g.wrapping_sub(rng.below(5)));
                    t.check(
                        "wait",
                        0x0057_d970,
                        Conv::Thiscall(this),
                        &[job],
                        ALL,
                        |gg| be::wait(gg, this, job),
                    );
                }
                _ => {
                    if case % 8 == 3 {
                        t.w32(this + 0x4c, 0);
                    } else {
                        // jobs queued, workers to share them
                        // some not divisible between the workers
                        let (q, w) = if case % 16 == 7 {
                            (2, 3)
                        } else {
                            (1 + rng.below(cap), 1 + rng.below(3))
                        };
                        t.w32(this + 0x4c, q);
                        t.w32(this + 8, w);
                    }

                    // nothing queued: the original returns with the caller's eax
                    let mask = if t.r32(this + 0x4c) == 0 { 0 } else { ALL };
                    t.check("flush", 0x0057_d740, Conv::Thiscall(this), &[], mask, |g| {
                        be::flush(g, this)
                    });
                }
            }
        }
    }
}

#[test]
fn work_stealing_lifetime_matches_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d402);
        t.stub(ce::POOL_CTOR, true, 2, 2, Ret::Imm(0xcbe0_0000));
        t.stub(ce::VECTOR_CTOR, false, 5, 5, Ret::Imm(0x966a_0000));
        t.stub(ce::VECTOR_DTOR, false, 4, 4, Ret::Imm(0x96d2_0000));
        t.stub(ce::SHUTDOWN, true, 0, 0, Ret::Imm(0xd7e0_0000));
        t.stub(ce::POOL_DTOR, true, 0, 0, Ret::Imm(0xcc60_0000));
        t.stub(ce::OPERATOR_DELETE, false, 1, 0, Ret::Imm(0x0390_0000));
        t.stub(
            ce::OPERATOR_DELETE_ARRAY,
            false,
            1,
            0,
            Ret::Imm(0x0540_0000),
        );
        t.stub(ce::TIMELINE_DTOR, true, 0, 0, Ret::Imm(0xec70_0000));
        for case in 0..8u32 {
            let this = t.random(&mut rng, 0x60);
            let (a, v) = (rng.next(), rng.next());
            t.check(
                "ctor",
                0x0057_d100,
                Conv::Thiscall(this),
                &[a, v],
                ALL,
                |g| be::ctor(g, this, a, v),
            );
            t.check("dtor", 0x0057_d1f0, Conv::Thiscall(this), &[], ALL, |g| {
                be::dtor(g, this)
            });
            let flags = rng.next() & !3 | (case & 3);
            t.check(
                "delete",
                0x0057_d310,
                Conv::Thiscall(this),
                &[flags],
                ALL,
                |g| be::delete(g, this, flags),
            );
            let block = t.random(&mut rng, 0x10);
            let tl = block + 4;
            t.check(
                "timeline delete",
                0x0057_d280,
                Conv::Thiscall(tl),
                &[flags],
                ALL,
                |g| be::timeline_vector_delete(g, tl, flags),
            );
        }
    }
}

#[test]
fn work_stealing_shutdown_matches_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d403);
        // the shutdown: timelines or none, records or none
        t.stub(ce::REMOVE_TIMELINE, true, 1, 1, Ret::Imm(0xede0_0000));
        t.stub(
            ce::OPERATOR_DELETE_ARRAY,
            false,
            1,
            0,
            Ret::Imm(0x0540_0000),
        );
        t.stub(ce::SEMAPHORE_SIGNAL, true, 2, 2, Ret::Imm(0));
        t.stub(ce::POOL_DTOR_BODY, true, 0, 0, Ret::Imm(0xd070_0000));
        t.stub(ce::CLOSE, true, 0, 0, Ret::Imm(0));
        t.stub(ce::PHYRE_FREE, false, 1, 0, Ret::Imm(0x0f7e_e000));
        for case in 0..12u32 {
            let workers = rng.below(4);
            let this = ws_pool(&mut t, &mut rng, workers, 2);
            let tls = match case % 3 {
                0 => 0,
                1 => {
                    let block = t.zeros(4 + 4 * 4);
                    block + 4
                }
                _ => {
                    let block = t.zeros(4 + 4 * 4);
                    t.w32(block, workers.max(1));
                    let obj = t.object(&mut rng, 0x7006_0000, &[1], 4);
                    let vt = t.r32(obj);
                    t.w32(block + 4, vt);
                    block + 4
                }
            };
            t.w32(this + 0x1c, tls);
            if case % 2 == 0 {
                t.w32(this + 0x58, 0);
            }
            t.check(
                "shutdown",
                0x0057_d7e0,
                Conv::Thiscall(this),
                &[],
                ALL,
                |g| be::shutdown(g, this),
            );
        }
    }
}

// ---- batch 6 (ffx_pcore::m5p_core_f) ----

use ffx_pcore::m5p_core_f::{self as cf, bridge as bf};

/// A stub that copies `words` words from the pointer in its stack word
/// `arg` into `seen`, then runs `rec` (a recording stub; `ecx` kept).
fn copying_stub(t: &mut Twins, rec: u32, seen: u32, arg: u32, words: u32, next: bool) -> u32 {
    let mut c = vec![0x51, 0x56, 0x57]; // push ecx; push esi; push edi
    c.extend([0x8b, 0x74, 0x24, (0x10 + 4 * arg) as u8]); // mov esi, [esp+0x10+4arg]
    c.push(0xbf);
    c.extend(seen.to_le_bytes()); // mov edi, seen
    c.push(0xb9);
    c.extend(words.to_le_bytes()); // mov ecx, words
    c.extend([0xf3, 0xa5]); // rep movsd
    if next {
        // the next stack word after the pointer (a size, never a frame pointer)
        c.extend([0x8b, 0x74, 0x24, (0x14 + 4 * arg) as u8]); // mov esi, [esp+0x14+4arg]
        c.extend([0x89, 0x37]); // mov [edi], esi
    }
    c.extend([0x5f, 0x5e, 0x59]); // pop edi; pop esi; pop ecx
    c.extend([0x90; 5]);
    let code = t.code(&c);
    t.jump(code + c.len() as u32 - 5, rec);
    code
}

#[test]
fn start_threads_matches_the_original() {
    // the array new's size saturates as the original's seto/setb pair
    assert_eq!(cf::array_bytes(3, 0x18), 0x4c);
    assert_eq!(cf::array_bytes(0x1000_0000, 0x18), u32::MAX);
    assert_eq!(cf::array_bytes(0x3fff_ffff, 4), u32::MAX);
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d501);
        let (_, left) = bump_malloc(&mut t, 0x8000);
        t.stub(c::PHYRE_FREE, false, 1, 0, Ret::Imm(0x0f7e_e000));
        let plain = t.zeros(4);
        t.stub(cf::PHYRE_MALLOC_PLAIN, false, 1, 0, Ret::Cell(plain));
        t.stub(cf::VECTOR_CTOR, false, 5, 5, Ret::Imm(0x966a_0000));
        let seen = t.zeros(0x20);
        let err = t.zeros(4);
        let rec = t.stub_code(cf::THREAD_CREATE, true, 0, 1, Ret::Cell(err));
        let code = copying_stub(&mut t, rec, seen, 0, 6, false);
        let at = t.rebase(cf::THREAD_CREATE);
        t.jump(at, code);
        for case in 0..48u32 {
            let this = t.random(&mut rng, 0x20);
            let n = rng.below(4);
            t.w32(this + 8, n);
            t.w32(this + 0x14, rng.below(4));
            let block = t.random(&mut rng, (4 + 0x18 * 4) as usize);
            t.w32(plain, if case % 7 == 3 { 0 } else { block });
            // an allocation fails: the job pointers or the scratch (the name
            // builder's failing would crash the original too)
            let stack = if case % 2 == 0 { 0 } else { 0x40 };
            let k = u32::from(t.r32(this + 0x14) != 0) + u32::from(stack != 0);
            t.w32(
                left,
                if case % 5 == 2 && k > 0 {
                    case / 5 % k
                } else {
                    u32::MAX
                },
            );
            // the frame starts with junk: every word the original sets shows
            let local = t.b.stack_range().start + 0x300;
            let junk: Vec<u8> = (0..0x40).map(|_| rng.next() as u8).collect();
            t.b.write(local, &junk).unwrap();
            t.w32(err, if case % 6 == 5 { 0x0000_0e11 } else { 0 });
            let priority = rng.below(3);
            t.check(
                "start threads",
                0x0044_ce70,
                Conv::Thiscall(this),
                &[stack, priority],
                ALL,
                |g| bf::start_threads(g, this, stack, priority, local),
            );
        }
    }
}

#[test]
fn work_stealing_initialize_matches_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d502);
        let (_, left) = bump_malloc(&mut t, 0x8000);
        let newa = t.zeros(4);
        t.stub(cf::OPERATOR_NEW_ARRAY, false, 1, 0, Ret::Cell(newa));
        t.stub(cf::VECTOR_CTOR, false, 5, 5, Ret::Imm(0x966a_0000));
        let seen = t.zeros(0x20);
        let init_err = t.zeros(4);
        let rec = t.stub_code(cf::TIMELINE_INIT, true, 0, 2, Ret::Cell(init_err));
        let code = copying_stub(&mut t, rec, seen, 0, 4, true);
        let at = t.rebase(cf::TIMELINE_INIT);
        t.jump(at, code);
        let add_err = t.zeros(4);
        t.stub(cf::ADD_TIMELINE, true, 1, 1, Ret::Cell(add_err));
        t.stub(cf::SEMAPHORE_INIT, true, 2, 2, Ret::Imm(0));
        t.stub(cf::START_THREADS, true, 2, 2, Ret::Imm(0x5a5a_0000));
        for case in 0..48u32 {
            let this = t.random(&mut rng, 0x60);
            let n = if case % 8 == 0 { 0 } else { rng.below(4) };
            t.w32(this + 8, n);
            t.w32(
                this + 0x14,
                if case % 9 == 4 { 0 } else { 1 + rng.below(3) },
            );
            let block = t.random(&mut rng, 4 + 4 * 4);
            t.w32(newa, if case % 7 == 3 { 0 } else { block });
            t.w32(init_err, if case % 11 == 6 { 0x31 } else { 0 });
            t.w32(add_err, if case % 13 == 5 { 0x32 } else { 0 });
            t.w32(
                left,
                if case % 5 == 2 {
                    case / 5 % 2
                } else {
                    u32::MAX
                },
            );
            let local = t.b.stack_range().start + 0x300;
            t.check(
                "initialize",
                0x0057_d4f0,
                Conv::Thiscall(this),
                &[],
                ALL,
                |g| bf::initialize(g, this, local),
            );
        }
    }
}

#[test]
fn worker_thread_matches_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d503);
        // the run semaphore's wait: logs, and sets the record's stop flag
        // (address in `stop`) on the `count`th call
        let count = t.zeros(4);
        let stop = t.zeros(4);
        let rec = t.stub_code(cf::SEMAPHORE_WAIT, true, 0, 0, Ret::Imm(0));
        let mut c = vec![0xff, 0x0d];
        c.extend(count.to_le_bytes()); // dec dword [count]
        c.extend([0x75, 0x08]); // jnz +8
        c.push(0xa1);
        c.extend(stop.to_le_bytes()); // mov eax, [stop]
        c.extend([0xc6, 0x00, 0x01]); // mov byte [eax], 1
        c.extend([0x90; 5]);
        let code = t.code(&c);
        t.jump(code + c.len() as u32 - 5, rec);
        let at = t.rebase(cf::SEMAPHORE_WAIT);
        t.jump(at, code);
        t.stub(cf::SEMAPHORE_SIGNAL, true, 2, 2, Ret::Imm(0));
        // InterlockedCompareExchange(dest, exchange, comparand)
        let cas = t.code(&[
            0x8b, 0x4c, 0x24, 0x04, 0x8b, 0x54, 0x24, 0x08, 0x8b, 0x44, 0x24, 0x0c, 0xf0, 0x0f,
            0xb1, 0x11, 0xc2, 0x0c, 0x00,
        ]);
        let slot = t.rebase(cf::COMPARE_EXCHANGE);
        t.w32(slot, cas);
        for _ in 0..16 {
            let workers = 1 + rng.below(3);
            let index = rng.below(workers);
            let this = t.random(&mut rng, 0x60);
            let recd = t.random(&mut rng, 0x18);
            t.write(recd + 0x10, &(index as u16).to_le_bytes());
            t.write(recd + 0x12, &(workers as u16).to_le_bytes());
            t.w8(recd + 0x14, 0);
            for p in 0..2u32 {
                let records = t.random(&mut rng, (12 * 3) as usize);
                for w in 0..workers {
                    let k = rng.below(4);
                    let jobs: Vec<u32> = (0..k)
                        .map(|_| {
                            if rng.chance(5) {
                                0
                            } else {
                                t.object(&mut rng, 0x7007_0000, &[0, 3], 8)
                            }
                        })
                        .collect();
                    let v: Vec<u8> = jobs.iter().flat_map(|j| j.to_le_bytes()).collect();
                    let list = t.alloc(&[v, vec![0; 4]].concat());
                    let r = records + 12 * w;
                    t.w32(r + 4, list);
                    t.w32(r + 8, list + 4 * k);
                }
                t.w32(this + 0x58 + 4 * p, records);
            }
            t.w32(count, 1 + rng.below(5));
            t.w32(stop, recd + 0x14);
            let local = t.b.stack_range().start + 0x300;
            // the counter and flag cells are shared: each side starts alike
            t.check(
                "worker thread",
                0x0057_da20,
                Conv::Thiscall(this),
                &[recd],
                ALL,
                |g| bf::worker_thread(g, this, recd, local),
            );
        }
    }
}

// ---- batch 7 (ffx_pcore::m5p_core_g) ----

use ffx_pcore::m5p_core_g::{self as cg, bridge as bg};

/// Plain floats in `[-r, r)` (no NaN or infinity).
fn plain3(t: &mut Twins, rng: &mut Rng, n: usize, range: f32) -> u32 {
    let v: Vec<u8> = (0..n)
        .flat_map(|_| {
            ((f32::from((rng.next() % 4000) as u16) / 2000.0 - 1.0) * range).to_le_bytes()
        })
        .collect();
    t.alloc(&v)
}

/// A spline: the control point callee replaced by a table lookup (point
/// `i & 7` of 16-byte points), tension `+0x1c`, length `+0x20`, the
/// segment table `+0x24`/`+0x28`, the world matrix `+0x14`.
fn spline(t: &mut Twins, rng: &mut Rng, segments: u32) -> u32 {
    let this = t.random(rng, 0x30);
    let tension = f32::from((rng.next() % 100) as u16) / 100.0;
    t.w32(this + 0x1c, tension.to_bits());
    let length = 1.0 + f32::from((rng.next() % 900) as u16) / 10.0;
    t.w32(this + 0x20, length.to_bits());
    t.w32(this + 0x24, segments | (rng.next() & 0x8000_0000));
    let table: Vec<u8> = (0..=segments)
        .flat_map(|_| {
            let seg = rng.below(6);
            let tt = f32::from((rng.next() % 1000) as u16) / 1000.0;
            let mut e = seg.to_le_bytes().to_vec();
            e.extend(tt.to_le_bytes());
            e.extend([0; 20]);
            e
        })
        .collect();
    let table = t.alloc(&table);
    t.w32(this + 0x28, table);
    let m = plain3(t, rng, 12, 2.0);
    t.w32(this + 0x14, m);
    this
}

/// `0x0054b080` replaced: `[out]` = point `i & 7` of `table` (16 bytes),
/// `ret 8`.
fn control_points(t: &mut Twins, rng: &mut Rng) {
    let table = plain3(t, rng, 32, 20.0);
    let mut c = vec![0x56, 0x57]; // push esi; push edi
    c.extend([0x8b, 0x44, 0x24, 0x10]); // mov eax, [esp+0x10] (i)
    c.extend([0x83, 0xe0, 0x07, 0xc1, 0xe0, 0x04]); // and eax, 7; shl eax, 4
    c.push(0x05);
    c.extend(table.to_le_bytes()); // add eax, table
    c.extend([0x89, 0xc6, 0x8b, 0x7c, 0x24, 0x0c]); // mov esi, eax; mov edi, [esp+0xc]
    c.extend([0xa5, 0xa5, 0xa5, 0xa5]); // movsd x4
    c.extend([0x5f, 0x5e, 0x8b, 0x44, 0x24, 0x04, 0xc2, 0x08, 0x00]); // pop; pop; mov eax, [esp+4]; ret 8
    let code = t.code(&c);
    let at = t.rebase(cg::CONTROL_POINT);
    t.jump(at, code);
}

#[test]
fn spline_points_match_the_original() {
    assert_eq!(cg::chop(2.75), 2);
    assert_eq!(cg::chop(-1.5), u32::MAX);
    assert_eq!(cg::chop(f64::NAN), 0);
    assert_eq!(cg::chop(1e300), 0);
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d601);
        control_points(&mut t, &mut rng);
        for case in 0..40u32 {
            // coincident points once in a while (a zero tangent)
            let p = if case % 8 == 0 {
                let one = plain3(&mut t, &mut rng, 3, 5.0);
                let v = Oracle::read(&mut t.a, one, 12).unwrap();
                t.alloc(&[v.clone(), v.clone(), v.clone(), v].concat())
            } else {
                plain3(&mut t, &mut rng, 12, 10.0)
            };
            let out = t.random(&mut rng, 12);
            let tension = float_word(&mut rng);
            t.check(
                "tangent",
                0x0054_8980,
                Conv::Cdecl,
                &[out, p, p + 12, p + 24, tension],
                ALL,
                |g| bg::tangent(g, out, p, p + 12, p + 24, tension),
            );
            let tt = if case % 5 == 0 {
                float_word(&mut rng)
            } else {
                (f32::from((rng.next() % 1000) as u16) / 1000.0).to_bits()
            };
            let args = [out, p, p + 12, p + 24, p + 36, tt, tension];
            t.check("hermite", 0x0054_87d0, Conv::Cdecl, &args, ALL, |g| {
                bg::hermite(g, out, p, p + 12, p + 24, p + 36, tt, tension)
            });
            let v = if case % 9 == 0 {
                t.zeros(12)
            } else {
                plain3(&mut t, &mut rng, 3, 50.0)
            };
            t.check("length", 0x0054_bd60, Conv::Cdecl, &[out, v], ALL, |g| {
                bg::length_of(g, out, v)
            });
        }
        for _ in 0..24 {
            let this = spline(&mut t, &mut rng, 4);
            let out = t.random(&mut rng, 12);
            let i = rng.below(10);
            let tt = (f32::from((rng.next() % 1000) as u16) / 1000.0).to_bits();
            let local = t.b.stack_range().start + 0x300;
            t.check(
                "evaluate",
                0x0054_9920,
                Conv::Thiscall(this),
                &[out, i, tt],
                ALL,
                |g| bg::eval(g, this, out, i, tt, local),
            );
            t.check(
                "direction",
                0x0054_9a60,
                Conv::Thiscall(this),
                &[out, i, tt],
                ALL,
                |g| bg::direction(g, this, out, i, tt, local),
            );
        }
    }
}

#[test]
fn spline_distances_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d602);
        control_points(&mut t, &mut rng);
        for case in 0..40u32 {
            let segments = if case % 6 == 0 { 0 } else { 1 + rng.below(5) };
            let this = spline(&mut t, &mut rng, segments);
            let out = t.random(&mut rng, 12);
            let length = f32::from_bits(t.r32(this + 0x20));
            // along the spline, at its ends, a little outside (index 0's
            // neighbour), never past the table
            let d = match case % 5 {
                0 => 0.0,
                1 => length * 0.999,
                // before the start: index -1 (the entry before the table)
                3 if segments > 0 => -length * 1.5 / f32::from(segments as u16),
                _ => length * f32::from((rng.next() % 1000) as u16) / 1000.0,
            };
            let d = d.to_bits();
            let local = t.b.stack_range().start + 0x300;
            let what = if case % 2 == 0 {
                cg::Along::Direction
            } else {
                cg::Along::Position
            };
            let f = if what == cg::Along::Direction {
                0x0054_b710
            } else {
                0x0054_b8f0
            };
            t.check("at", f, Conv::Thiscall(this), &[out, d], ALL, |g| {
                bg::at(g, this, out, d, what, local)
            });
            t.check(
                "world direction",
                0x0054_b860,
                Conv::Thiscall(this),
                &[out, d],
                ALL,
                |g| bg::world_direction(g, this, out, d, local),
            );
            t.check(
                "world position",
                0x0054_ba40,
                Conv::Thiscall(this),
                &[out, d],
                ALL,
                |g| bg::world_position(g, this, out, d, local),
            );
        }
    }
}

#[test]
fn spline_segment_lengths_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d603);
        control_points(&mut t, &mut rng);
        for case in 0..24u32 {
            let this = spline(&mut t, &mut rng, 3);
            t.w32(
                this + 4,
                if case % 6 == 0 {
                    rng.below(2)
                } else {
                    2 + rng.below(8)
                } | (rng.next() & 0x8000_0000),
            );
            let i = rng.below(9);
            let local = t.b.stack_range().start + 0x300;
            t.check_f32_conv(
                "segment length",
                0x0054_bdb0,
                Conv::Thiscall(this),
                &[i],
                |g| bg::segment_length(g, this, i, local),
            );
        }
    }
}

#[test]
fn camera_assign_matches_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d304);
        let names: Vec<u32> = ["", "MainCamera"]
            .iter()
            .map(|s| {
                let mut v = s.as_bytes().to_vec();
                v.push(0);
                t.alloc(&v)
            })
            .collect();
        t.stub(c::PHYRE_FREE, false, 1, 0, Ret::Imm(0x0f7e_e000));
        let (_, _) = bump_malloc(&mut t, 0x1000);
        for case in 0..12u32 {
            let src = floats(&mut t, &mut rng, 0x150 / 4);
            let name = match case % 3 {
                0 => 0,
                _ => names[rng.below(2) as usize] | (rng.next() & 1),
            };
            t.w32(src + 0xcc, name);
            let dst = if case % 4 == 3 {
                src
            } else {
                floats(&mut t, &mut rng, 0x150 / 4)
            };
            if dst != src {
                // borrowed text: the assignment does not free it
                t.w32(dst + 0xcc, names[1] | 1);
            }
            t.check(
                "camera assign",
                0x0045_33b0,
                Conv::Thiscall(dst),
                &[src],
                ALL,
                |g| bd::camera_assign(g, dst, src),
            );
        }
    }
}

#[test]
fn texture_upload_matches_the_original() {
    use ffx_pcore::m5p_core_h::{self as ch, bridge as bh};
    for mut t in Twins::all() {
        let mut rng = Rng(0x5eed_d701);
        t.stub(ch::LOCK_ENTER, false, 0, 0, Ret::Imm(0x3d50_0000));
        t.stub(ch::LOCK_LEAVE, false, 0, 0, Ret::Imm(0x3e90_0000));
        t.stub(ch::RENDERER_CTOR, true, 0, 0, Ret::Imm(0xee70_0000));
        t.stub(ch::ATEXIT, false, 1, 0, Ret::Imm(0));
        t.a.map_zeroed(0, 0x1000).unwrap();
        t.b.map_zeroed(0, 0x1000).unwrap();
        for case in 0..40u32 {
            // the device context: methods +0x3c (3 words) and +0xc0 (7),
            // stdcall with the context first (ecx is junk: not logged)
            let m3c = t.stub_code(0x7008_003c, false, 3, 3, Ret::Imm(0));
            let mc0 = t.stub_code(0x7008_00c0, false, 7, 7, Ret::Imm(0));
            let mut vt = [0u32; 49];
            vt[15] = m3c;
            vt[48] = mc0;
            let vt: Vec<u8> = vt.iter().flat_map(|w| w.to_le_bytes()).collect();
            let vt = t.alloc(&vt);
            let ctx = t.alloc(&vt.to_le_bytes());
            let c = t.rebase(ch::CONTEXT);
            t.w32(c, ctx);
            let guard = t.rebase(ch::RENDERER_GUARD);
            let g = t.r32(guard);
            t.w32(guard, if case % 3 == 0 { g & !1 } else { g | 1 });
            let this = t.random(&mut rng, 0x70);
            let fmt = t.random(&mut rng, 0x14);
            t.w32(fmt + 0xc, [4u32, 8, 16, 32][rng.below(4) as usize]);
            t.w8(fmt + 0x10, rng.below(2) as u8);
            t.w32(this, fmt);
            t.w32(this + 0xc, rng.below(4));
            t.w32(this + 0x14, rng.next() & 2 | (rng.next() & !2));
            t.w32(
                this + 0x1c,
                if case % 7 == 0 { 0 } else { 1 + rng.below(64) },
            );
            t.w32(
                this + 0x20,
                if case % 5 == 0 { 0 } else { 1 + rng.below(64) },
            );
            t.w32(
                this + 0x68,
                if case % 4 == 1 {
                    0x1_0000
                } else {
                    rng.next() & !0x1_0000
                },
            );
            let level = rng.below(5);
            // no surfaces counts as one at a null storage (page 0, mapped
            // with a zero resource word: 0x15)
            let n = rng.below(4);
            let entries = t.random(&mut rng, (0x1c * 4) as usize);
            for k in 0..n.max(1) {
                let res = if rng.chance(5) { 0 } else { rng.next() | 1 };
                t.w32(entries + 0x1c * k, res);
            }
            t.w32(this + 0x24, n | (rng.next() & 0x8000_0000));
            if n == 1 {
                // one surface lives in the header itself: entry at +0x28
                let res = if rng.chance(5) { 0 } else { rng.next() | 1 };
                t.w32(this + 0x28, res);
            } else {
                t.w32(this + 0x28, entries);
            }
            let unused = rng.next();
            let local = t.b.stack_range().start + 0x300;
            t.check(
                "upload",
                0x0059_d8f0,
                Conv::Thiscall(this),
                &[level, unused],
                ALL,
                |g| bh::upload(g, this, level, local),
            );
        }
    }
}
