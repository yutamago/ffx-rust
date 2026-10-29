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

/// The bridge's view of an oracle.
struct G<'a>(&'a mut Oracle);

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
        };
        t.log = t.zeros((4 + ENTRY * ENTRIES) as usize);
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
        let rb = port(&mut G(&mut self.b));
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
        self.fresh_log();
        let f = self.rebase(sym);
        let ra = self
            .a
            .call(f, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{} {what}: original: {e}", self.label));
        let rb = port(&mut G(&mut self.b));
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
