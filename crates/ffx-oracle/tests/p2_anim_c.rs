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
        self.fresh_log();
        let f = self.rebase(sym);
        let ra = self
            .a
            .call(f, Conv::Cdecl, args)
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
