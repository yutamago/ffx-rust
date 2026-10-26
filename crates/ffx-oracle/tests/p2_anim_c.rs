//! `ffx_panim`'s continuation round (list `p2-anim`: `p2_bind`,
//! `p2_declare`, `p2_more`) against the original: on the image alone with
//! synthetic data, and on a snapshot (`FFX_SNAPSHOT`) with the same data
//! over the snapshot's memory. Results (all of `eax`) and every byte
//! written must agree. Needs `FFX_GAME_DIR`.
//!
//! Callees outside the list are replaced in both oracles alike by
//! recording stubs: each call appends its tag, `ecx` and up to 14 stack
//! words to a log in memory (cleared before every case), so the diff
//! checks both made the same calls in the same order with the same `this`
//! and arguments.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_panim::p2_more::{self as more, bridge as mb};
use ffx_panim::{p2_bind, p2_declare};
use ffx_pother_a::p3_bind;
use ffx_prender::{declare as dc, h1_bind as h1b};

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
    /// The word in this cell at the time of the call.
    Cell(u32),
}

/// Log entries: 16 words (tag, `ecx`, 14 stack words).
const ENTRY: u32 = 64;
const ENTRIES: u32 = 0x400;

struct Twins {
    label: &'static str,
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then [`ENTRY`]-byte entries.
    log: u32,
    /// Original bytes of each stubbed address.
    saved: Vec<(u32, Vec<u8>)>,
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
            saved: Vec::new(),
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

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
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
        if !self.saved.iter().any(|(a, _)| *a == at) {
            let old = Oracle::read(&mut self.a, at, 5).unwrap();
            self.saved.push((at, old));
        }
        let mut j = vec![0xe9];
        j.extend(c.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
    }

    /// An object whose vtable slots `0..words.len()` are recording stubs
    /// (methods tagged `base + 4 * slot`, taking `words[slot]` words and
    /// returning `rets[slot]`), `size` bytes of random words after its
    /// vtable pointer.
    fn object(&mut self, rng: &mut Rng, base: u32, words: &[u32], rets: &[Ret], size: u32) -> u32 {
        let slots: Vec<u32> = (0u32..)
            .zip(words.iter().zip(rets))
            .map(|(i, (&n, &r))| self.stub_code(base + 4 * i, true, n, n, r))
            .collect();
        let vt: Vec<u8> = slots.iter().flat_map(|s| s.to_le_bytes()).collect();
        let vt = self.alloc(&vt);
        let o = self.random(rng, size as usize);
        self.w32(o, vt);
        o
    }

    fn random(&mut self, rng: &mut Rng, n: usize) -> u32 {
        let v: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
        self.alloc(&v)
    }

    /// Puts back every stubbed original.
    fn unstub_all(&mut self) {
        for (at, bytes) in std::mem::take(&mut self.saved) {
            self.write(at, &bytes);
        }
    }

    /// Clears the call log (every case starts with an empty one).
    fn fresh_log(&mut self) {
        let log = self.log;
        self.w32(log, 0);
    }

    /// Runs the original at `sym` on `a` and `port` on `b`; compares all
    /// of `eax` and every byte written.
    fn check(&mut self, what: &str, sym: u32, args: &[u32], port: impl FnOnce(&mut G) -> u32) {
        self.check_conv(what, sym, Conv::Cdecl, args, port);
    }

    /// [`Twins::check`] in another calling convention.
    fn check_conv(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        port: impl FnOnce(&mut G) -> u32,
    ) {
        self.fresh_log();
        let f = self.rebase(sym);
        let ra = self
            .a
            .call(f, conv, args)
            .unwrap_or_else(|e| panic!("{} {what}: original: {e}", self.label));
        let rb = port(&mut G(&mut self.b));
        assert_eq!(
            ra.eax, rb,
            "{} {what}: eax {:#x} vs {:#x}",
            self.label, ra.eax, rb
        );
        let calls = self.r32(self.log);
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

/// The parts of an h1-format `Bind` a test sets up: guard words, words
/// it reads and rewrites (flag words, `Call` arguments, store targets),
/// callees with their stack words (0x100: a descriptor constructor,
/// thiscall), class members.
#[derive(Default)]
struct Parts {
    guards: Vec<u32>,
    words: Vec<u32>,
    calls: Vec<(u32, u32)>,
    classes: Vec<u32>,
}

fn h1_parts(steps: &[h1b::Step], p: &mut Parts) {
    use h1b::{Step, Ty};
    let ty = |t: Ty, p: &mut Parts| match t {
        Ty::Addr(_) => {}
        Ty::Call(f) => p.calls.push((f, 0)),
        Ty::CallWith(f, _) => p.calls.push((f, 1)),
    };
    for s in steps {
        match *s {
            Step::Guard(g, _, inner) => {
                if !p.guards.contains(&g) {
                    p.guards.push(g);
                }
                h1_parts(inner, p);
            }
            Step::Member { ty: t, .. } | Step::Array { ty: t, .. } => ty(t, p),
            Step::Class { member, .. } => p.classes.push(member),
            Step::Call(f, arg) => {
                p.calls.push((f, u32::from(arg.is_some())));
                if let Some(g) = arg {
                    p.words.push(g);
                }
            }
            Step::Descriptor { ctor, .. } => p.calls.push((ctor, 0x100)),
            Step::Or(at, _) | Step::And(at, _) | Step::Store(at, _) => p.words.push(at),
            Step::AtExit(_) => {}
        }
    }
}

/// The one-word calls' targets as their call sites push them
/// (`push dword [global]`; from the machine code, not the tables).
const ONE_WORD_CALLS: &[u32] = &[
    0x0050_9da0,
    0x0051_45c0,
    0x0051_47d0,
    0x0051_a230,
    0x0051_a440,
    0x0051_a650,
    0x0051_f650,
    0x0052_8610,
    0x0052_8820,
    0x0052_bdb0,
    0x0053_6dc0,
    0x0053_6fe0,
    0x0053_71f0,
    0x0053_7400,
    0x0054_0970,
];

/// A class member's owner: a list at `+list` (empty, or two nodes) and
/// a name or none at the member's `+0xc`.
fn class_owner(t: &mut Twins, rng: &mut Rng, m: u32, list: u32) {
    let owner = t.zeros(0x60);
    let head = owner + list;
    let first = t.zeros(8);
    let second = t.zeros(8);
    if rng.chance(2) {
        t.w32(head, head);
        t.w32(head + 4, head);
    } else {
        t.w32(head, first);
        t.w32(first, second);
        t.w32(second, head);
        t.w32(head + 4, second);
        t.w32(second + 4, first);
        t.w32(first + 4, head);
    }
    t.w32(m + 8, owner);
    let named = if rng.chance(3) { 0 } else { rng.next() | 1 };
    t.w32(m + 0xc, named);
}

#[test]
fn binds_match_the_original() {
    // every h1-format copy has its shim, looked up by its own address
    let src = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ffx-hook/src/ported/p2_anim_c.rs"),
    )
    .unwrap();
    for &(a, _) in p2_bind::BINDS {
        let hex = format!("{:#06x}_{:04x}", a >> 16, a & 0xffff);
        let at = src
            .find(&format!(" @ {hex} => "))
            .unwrap_or_else(|| panic!("{a:#x}: no shim"));
        assert!(
            src[at..].contains(&format!("p2_bind::bind_of({hex})")),
            "{a:#x}: shim table"
        );
    }
    for mut t in Twins::all() {
        let mut rng = Rng(0x0b1d_0002);
        let found = t.zeros(4);
        t.stub(dc::ATEXIT, false, 1, 0, Ret::Imm(0x6003));
        t.stub(dc::REGISTER_TYPE, false, 1, 0, Ret::Imm(0x6001));
        t.stub(dc::ADD_TO_NAMESPACE, true, 0, 0, Ret::Imm(0x5a5a_1234));
        for ctor in [dc::DATA_MEMBER_CTOR, 0x0057_5f30, 0x0057_5370] {
            t.stub(ctor, true, 6, 6, Ret::Imm(ctor));
        }
        t.stub(h1b::ARRAY_MEMBER_CTOR, true, 7, 7, Ret::Imm(7));
        t.stub(h1b::CLASS_MEMBER_CTOR, true, 3, 3, Ret::Imm(3));
        t.stub(h1b::FIND_MEMBER, true, 1, 1, Ret::Cell(found));
        t.stub(p3_bind::FIND_FUNCTION.finder, true, 1, 1, Ret::Cell(found));
        let mut stubbed = vec![];
        let tables: Vec<(u32, Option<h1b::Bind>)> = p2_bind::BINDS
            .iter()
            .map(|&(f, k)| (f, Some(k)))
            .chain([(0x0052_1bb0, None)])
            .collect();
        for (f, k) in tables {
            let mut p = Parts::default();
            match k {
                Some(k) => h1_parts(k.steps, &mut p),
                None => {
                    for s in p2_bind::TIME_CONTROLLER {
                        match *s {
                            p3_bind::Step::Guard(g) => p.guards.push(g),
                            p3_bind::Step::Store(a, _) => p.words.push(a),
                            _ => {}
                        }
                    }
                }
            }
            for &(c, kind) in &p.calls {
                if stubbed.contains(&c) {
                    continue;
                }
                stubbed.push(c);
                if kind == 0x100 {
                    t.stub(c, true, 0, 0, Ret::Imm(0xd));
                } else {
                    // the stub's words come from the machine code, not
                    // from the table under test
                    let n = u32::from(ONE_WORD_CALLS.contains(&c));
                    assert_eq!(n, kind, "{f:#x}: {c:#x} words");
                    t.stub(c, false, n, 0, Ret::Imm(0x00c9_7000 | (c & 0xfff)));
                }
            }
            for case in 0..12u32 {
                if let Some(k) = k {
                    assert_eq!(p2_bind::bind_of(f), k);
                }
                for &g in &p.guards {
                    let word = match case {
                        0 => 0,
                        1 => u32::MAX,
                        _ => rng.next() & rng.next(),
                    };
                    let at = t.rebase(g);
                    t.w32(at, word);
                }
                for &w in &p.words {
                    let at = t.rebase(w);
                    let v = rng.next();
                    t.w32(at, v);
                }
                for &m in &p.classes {
                    let m = t.rebase(m);
                    class_owner(&mut t, &mut rng, m, 0x4c);
                }
                if k.is_none() {
                    let m = t.rebase(0x00ca_b9b4);
                    class_owner(&mut t, &mut rng, m, 0x54);
                }
                let v = rng.next() & 1;
                t.w32(found, v);
                let what = format!("bind {f:#x} case {case}");
                match k {
                    Some(k) => t.check(&what, f, &[], |g| h1b::bridge::bind(g, k)),
                    None => t.check(&what, f, &[], |g| {
                        p3_bind::bridge::bind(g, p2_bind::TIME_CONTROLLER)
                    }),
                }
            }
        }
        t.unstub_all();
    }
}

/// Which declaration a case runs.
#[derive(Clone, Copy)]
enum Decl {
    Plain(dc::Declare),
    Sized(dc::Declare, u32),
    Sharray(p2_declare::Sharray),
}

#[test]
fn declarations_match_the_original() {
    let mut all: Vec<(u32, Decl)> = p2_declare::DECLARES
        .iter()
        .map(|&(f, c)| (f, Decl::Plain(c)))
        .collect();
    all.push((
        0x0053_7400,
        Decl::Sized(p2_declare::PUINT128, p2_declare::PUINT128_SIZE),
    ));
    all.push((0x0051_a650, Decl::Sharray(p2_declare::CLIP_SHARRAY)));
    for mut t in Twins::all() {
        let mut rng = Rng(0x0dec_1a2e);
        t.stub(dc::ATEXIT, false, 1, 0, Ret::Imm(0x6003));
        t.stub(dc::REGISTER_TYPE, false, 1, 0, Ret::Imm(0x6001));
        t.stub(dc::ADD_TO_NAMESPACE, true, 0, 0, Ret::Imm(0x5a5a_1234));
        t.stub(dc::TYPE_UINT, false, 0, 0, Ret::Imm(0x00c9_0500));
        t.stub(dc::DATA_MEMBER_CTOR, true, 6, 6, Ret::Imm(6));
        t.stub(dc::ANNOTATION_CTOR, true, 3, 3, Ret::Imm(3));
        t.stub(dc::ADD_ANNOTATION, true, 1, 1, Ret::Imm(1));
        t.stub(dc::MEMBER_ARRAY_CTOR, true, 7, 7, Ret::Imm(7));
        let mut stubbed = vec![dc::TYPE_UINT];
        for &(f, k) in &all {
            let (desc, ctor, name_global, getter) = match k {
                Decl::Plain(c) | Decl::Sized(c, _) => (
                    c.desc,
                    c.ctor,
                    c.name,
                    match c.element {
                        dc::Element::Getter(g) => Some(g),
                        dc::Element::Descriptor(_) => None,
                    },
                ),
                Decl::Sharray(c) => (c.desc, c.ctor, c.name, None),
            };
            for c in [Some(ctor), getter].into_iter().flatten() {
                if !stubbed.contains(&c) {
                    stubbed.push(c);
                    let this = c == ctor;
                    t.stub(c, this, 0, 0, Ret::Imm(0x00c9_7000 | (c & 0xfff)));
                }
            }
            let d = t.rebase(desc);
            let ng = t.rebase(name_global);
            // the annotation's list head (+4 of the annotation)
            let annot = match k {
                Decl::Sharray(_) => d + 0x104,
                _ => d + 0xc4,
            };
            for case in 0..16u32 {
                let guard = match case {
                    0 | 4 => 0,
                    1 => u32::MAX,
                    _ => rng.next() & rng.next(),
                };
                t.w32(d + 0x94, guard);
                let flags = rng.next();
                t.w32(d + 0x90, flags);
                // a name, mostly; the descriptor's name mostly unset
                let name = if case == 3 { 0 } else { rng.next() | 1 };
                let own = if case % 5 == 2 { rng.next() | 1 } else { 0 };
                t.w32(d + 0x18, own);
                let first = if rng.chance(2) { 0 } else { rng.next() | 1 };
                t.w32(ng, first);
                // the annotation linked to itself (not yet added) or not
                let link = if rng.chance(2) { annot + 4 } else { rng.next() };
                t.w32(annot + 4, link);
                // what the stores overwrite
                for off in [0, 0x2c, 0x34, 0x38, 0x3c, 0x14] {
                    let v = rng.next();
                    t.w32(d + 0x98 + off, v);
                    let v = rng.next();
                    t.w32(d + 0xe0 + off, v);
                }
                for off in [0x0, 0x18] {
                    let v = rng.next();
                    t.w32(annot + off, v);
                }
                let v = rng.next();
                t.w32(d, v);
                let what = format!("declare {f:#x} case {case}");
                t.check(&what, f, &[name], |g| match k {
                    Decl::Plain(c) => {
                        assert_eq!(p2_declare::declare_of(f).desc, c.desc);
                        dc::bridge::declare(g, name, &c)
                    }
                    Decl::Sized(c, size) => p2_declare::bridge::declare_sized(g, name, &c, size),
                    Decl::Sharray(c) => p2_declare::bridge::declare_sharray(g, name, &c),
                });
            }
        }
        t.unstub_all();
    }
}

/// The `PAnimationClipBinding` header's callbacks: the `+0x94` handler
/// (stream objects with recording vtables), the `+0x3c` forwarder and the
/// destruct callback (its visitor logged by contents: the original's is
/// on its stack).
#[test]
fn clip_binding_header_matches_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x0b1d_1a7a);
        let skip = t.zeros(4);
        let align = t.zeros(4);
        t.stub(more::TYPE_U16, false, 0, 0, Ret::Imm(0x00c9_0016));
        for case in 0..24u32 {
            let stream = t.object(
                &mut rng,
                0x7100,
                &[0, 4, 5, 0],
                &[
                    Ret::Imm(0),
                    Ret::Imm(0x7104),
                    Ret::Imm(0x7108),
                    Ret::Cell(skip),
                ],
                0x10,
            );
            let other = t.object(
                &mut rng,
                0x7200,
                &[0, 0, 0, 1],
                &[Ret::Imm(0), Ret::Imm(0), Ret::Imm(0), Ret::Cell(align)],
                0x10,
            );
            t.w32(stream + 4, other);
            // any alignment: the caches block is rounded up to 4 bytes
            let header = t.random(&mut rng, 0x44) + case % 4;
            for off in [2, 4, 6] {
                let v = if case % 4 == 3 {
                    rng.next()
                } else {
                    rng.below(40)
                };
                t.write(header + off, &(v as u16).to_le_bytes());
            }
            let s = match case % 6 {
                0 => 1,
                1 => 0x100,
                2 => rng.next() & !0xff,
                _ => 0,
            };
            t.w32(skip, s);
            let al = match case % 7 {
                0 => 0,
                1 => 3,
                2 => rng.next(),
                n => 1 << n,
            };
            t.w32(align, al);
            t.check(
                &format!("header visit case {case}"),
                0x0051_0f90,
                &[header, stream],
                |g| mb::header_visit(g, header, stream),
            );
            // the forwarder: the handler at +0x94 (cdecl, two words)
            let f = t.stub_code(0x7300, false, 2, 0, Ret::Imm(rng.next()));
            let this = t.random(&mut rng, 0x100);
            t.w32(this + 0x94, f);
            t.check_conv(
                &format!("header +0x3c case {case}"),
                0x0051_1ac0,
                Conv::Thiscall(this),
                &[header, stream],
                |g| mb::header_slot3c(g, this, header, stream),
            );
        }
        // the destruct callback: vslot +0x3c logs the object and the
        // visitor's two words, `ret 8`
        let log = t.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(0x7400u32.to_le_bytes()); // mov [eax], tag
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
        c.extend([0x89, 0x50, 0x08]); // mov [eax+8], edx
        c.extend([0x8b, 0x54, 0x24, 0x08]); // mov edx, [esp+8]
        c.extend([0x8b, 0x0a]); // mov ecx, [edx]
        c.extend([0x89, 0x48, 0x0c]); // mov [eax+0xc], ecx
        c.extend([0x8b, 0x4a, 0x04]); // mov ecx, [edx+4]
        c.extend([0x89, 0x48, 0x10]); // mov [eax+0x10], ecx
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xb8);
        c.extend(0x1234_5600u32.to_le_bytes()); // mov eax, ...
        c.extend([0xc2, 0x08, 0x00]); // ret 8
        let visit = t.code(&c);
        t.stub(more::DELETE_CHILDREN_ARG, true, 0, 0, Ret::Imm(0x00cb_aa78));
        for case in 0..4u32 {
            let mut vt: Vec<u32> = (0..0x10).map(|_| rng.next()).collect();
            vt[0xf] = visit;
            let vt: Vec<u8> = vt.iter().flat_map(|w| w.to_le_bytes()).collect();
            let vt = t.alloc(&vt);
            let this = t.random(&mut rng, 0x20);
            t.w32(this, vt);
            let object = rng.next();
            t.check_conv(
                &format!("header destruct case {case}"),
                0x0051_1130,
                Conv::Thiscall(this),
                &[object],
                |g| {
                    let frame = g.0.stack_range().start + 0x80;
                    mb::header_destruct(g, this, object, frame)
                },
            );
        }
        t.unstub_all();
    }
}

/// `PAnimationClipBinding`'s fix-up and `PAnimationClip`'s
/// fix-after-loading callback.
#[test]
fn clip_fixups_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x0f12_0b1d);
        for case in 0..32u32 {
            // 2-byte aligned (u16 fields), so the records' 4-byte rounding shows
            let this = t.random(&mut rng, 0x304) + 2 * (case % 2);
            for off in [2, 4, 6] {
                let v = if case == 7 { 0xfffe } else { rng.below(24) };
                t.write(this + off, &(v as u16).to_le_bytes());
            }
            // targets: t+0x24 -> a block whose +8 is read, t+0x2c
            let targets: Vec<u32> = (0..6)
                .map(|_| {
                    let tg = t.random(&mut rng, 0x30);
                    let a = t.random(&mut rng, 0x10);
                    t.w32(tg + 0x24, a);
                    tg
                })
                .collect();
            let table: Vec<u8> = targets.iter().flat_map(|w| w.to_le_bytes()).collect();
            let table = t.alloc(&table);
            let n = rng.below(6);
            let entry = t.random(&mut rng, 0x10);
            let flag = if rng.chance(3) { 0x8000_0000 } else { 0 };
            // (bits 16-30 of the count would mean 65536 records and more)
            t.w32(entry + 4, n | flag);
            t.w32(entry + 8, table);
            // the records' target indices
            let h6 = u32::from(Mem::read::<u16>(&t.a, this + 6));
            let h4 = u32::from(Mem::read::<u16>(&t.a, this + 4));
            let h2 = u32::from(Mem::read::<u16>(&t.a, this + 2));
            let p = this.wrapping_add((((h6 + 3) & !3) + (h4 + h2) * 2 + 4) * 2);
            let rec = p + (p.wrapping_neg() & 3);
            if case == 7 {
                // keys past the block: no records
                t.w32(entry + 4, 0);
            } else {
                for i in 0..n {
                    let k = rng.below(6) as u16;
                    t.write(rec + 0x10 * i + 0xc, &k.to_le_bytes());
                }
            }
            t.check_conv(
                &format!("binding fixup case {case}"),
                0x0051_1980,
                Conv::Thiscall(this),
                &[entry],
                |g| mb::clip_binding_fixup(g, this, entry),
            );
        }
        // the clip: entries with a binding or none; the fix-up's result
        let r = t.zeros(4);
        t.stub(more::CLIP_BINDING_FIXUP, true, 1, 1, Ret::Cell(r));
        for case in 0..16u32 {
            let stride = 4 + 4 * rng.below(4);
            let n = rng.below(6);
            let entries = t.random(&mut rng, (stride * 6) as usize);
            for i in 0..n {
                let b = if rng.chance(3) { 0 } else { rng.next() | 1 };
                t.w32(entries + stride * i, b);
            }
            let info = t.random(&mut rng, 0x20);
            t.w32(info + 0x1c, stride);
            let clip = t.random(&mut rng, 0x40);
            t.w32(clip + 0x2c, info);
            t.w32(clip + 0x30, entries);
            t.w32(clip + 0x3c, n);
            let v = if case % 3 == 0 { rng.next() } else { 0 };
            t.w32(r, v);
            t.check_conv(
                &format!("clip fix after loading case {case}"),
                0x0051_6370,
                Conv::Stdcall,
                &[clip],
                |g| mb::clip_fix_after_loading(g, clip),
            );
        }
        t.unstub_all();
    }
}

/// A float's bits, no signalling NaN (the emulator keeps them).
fn float(rng: &mut Rng) -> u32 {
    match rng.below(8) {
        0 => [
            0,
            0x8000_0000,
            0x3f80_0000,
            0x7f80_0000,
            0xff80_0000,
            0x7fc0_0001,
        ][rng.below(6) as usize],
        1 => rng.next() & 0x807f_ffff,
        _ => (rng.next() & 0x807f_ffff) | ((100 + rng.below(56)) << 23),
    }
}

/// The 3x3 copy (aliasing included) and `PAnimationNetworkInstance`'s
/// constructor, destructor and thread pool.
#[test]
fn network_instance_and_matrix_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x3a3a_0e7e);
        for case in 0..12u32 {
            let words: Vec<u8> = (0..0x40)
                .flat_map(|_| float(&mut rng).to_le_bytes())
                .collect();
            let src = t.alloc(&words);
            let dst = match case % 3 {
                0 => src,
                1 => src + 4,
                _ => t.random(&mut rng, 0x40),
            };
            t.check_conv(
                &format!("upper 3x3 case {case}"),
                0x0053_1a40,
                Conv::Thiscall(dst),
                &[src],
                |g| mb::upper3x3(g, dst, src),
            );
        }
        for case in 0..2u32 {
            let this = t.random(&mut rng, 0x40);
            t.check_conv(
                &format!("network ctor case {case}"),
                0x0053_4450,
                Conv::Thiscall(this),
                &[],
                |g| mb::network_instance_ctor(g, this),
            );
        }
        t.stub(more::PHYRE_FREE, false, 1, 0, Ret::Imm(0x00f4_ee00));
        for case in 0..24u32 {
            let this = t.random(&mut rng, 0x40);
            for at in [4, 0xc, 0x20, 0x2c] {
                let owned = if rng.chance(2) { 0 } else { 0x8000_0000 };
                t.w32(this + at, owned | (rng.next() & 0xff));
                let data = if rng.chance(3) { 0 } else { rng.next() | 1 };
                t.w32(this + at + 4, data);
            }
            t.check_conv(
                &format!("network dtor case {case}"),
                0x0053_4b10,
                Conv::Thiscall(this),
                &[],
                |g| mb::network_instance_dtor(g, this),
            );
        }
        let block = t.zeros(4);
        let list = t.zeros(4);
        t.stub(more::PHYRE_MALLOC, false, 1, 0, Ret::Cell(block));
        t.stub(more::FREE_LIST_CTOR, true, 4, 4, Ret::Cell(list));
        let pool = t.rebase(more::THREAD_POOL);
        for case in 0..8u32 {
            let p = if case & 1 == 0 { 0 } else { rng.next() };
            t.w32(pool, p);
            let b = if case & 2 == 0 { 0 } else { rng.next() };
            t.w32(block, b);
            let l = if case & 4 == 0 { 0 } else { rng.next() };
            t.w32(list, l);
            t.check(&format!("thread pool case {case}"), 0x0053_9640, &[], |g| {
                mb::initialize_thread_pool(g)
            });
        }
        t.unstub_all();
    }
}

/// `PAnimatableComponent`'s constructor and destructor.
#[test]
fn animatable_component_matches_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x0a1a_c0c0);
        t.stub(more::OBJECT_REF_INIT, true, 1, 1, Ret::Imm(0x00e0_0001));
        t.stub(more::OBJECT_REF_RELEASE, true, 0, 0, Ret::Imm(0x00e0_0002));
        for f in [
            more::WEIGHTED_BLENDER_CTOR,
            more::WEIGHTED_BLENDER_DTOR,
            more::TARGET_BLENDER_CTOR,
            more::TARGET_BLENDER_DTOR,
            more::NETWORK_INSTANCE_CTOR,
            more::NETWORK_INSTANCE_DTOR,
        ] {
            t.stub(f, true, 0, 0, Ret::Imm(f));
        }
        t.stub(more::DELETE_ARRAY_4C, false, 2, 0, Ret::Imm(0x00e0_0003));
        for case in 0..2u32 {
            let this = t.random(&mut rng, 0x90);
            t.check_conv(
                &format!("animatable ctor case {case}"),
                0x0053_e720,
                Conv::Thiscall(this),
                &[],
                |g| mb::animatable_ctor(g, this),
            );
        }
        for case in 0..8u32 {
            let this = t.random(&mut rng, 0x90);
            let owned = if case % 2 == 0 { 0 } else { 0x8000_0000 };
            t.w32(this + 0x14, owned | rng.below(9));
            t.check_conv(
                &format!("animatable dtor case {case}"),
                0x0053_ebd0,
                Conv::Thiscall(this),
                &[],
                |g| mb::animatable_dtor(g, this),
            );
        }
        t.unstub_all();
    }
}
