//! `ffx_pcore::p5_more` (list `p5-core`, continuation round) against the
//! original: on the image alone with synthetic data, and on a snapshot
//! (`FFX_SNAPSHOT`) with the same data over the snapshot's memory. Results
//! (all of `eax` where the original defines it) and every byte written
//! must agree. Needs `FFX_GAME_DIR`. The harness is p2-anim's
//! (`tests/p2_anim_c.rs`): recording stubs log their tag, `ecx` and up to
//! 14 stack words, the log cleared before every case.

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
use ffx_pcore::p5_more::{self as p5m, bridge as p5b};
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
const ENTRIES: u32 = 0x800;

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
        self.check_masked(what, sym, conv, args, u32::MAX, port);
    }

    /// [`Twins::check_conv`] comparing `eax & mask` only.
    fn check_masked(
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
        assert_eq!(
            ra.eax & mask,
            rb & mask,
            "{} {what}: eax {:#x} vs {:#x}",
            self.label,
            ra.eax,
            rb
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

/// `PString::Bind` (h1's interpreter) and the `PSharray<T>` /
/// `PArray<T *>` declarations (p2's and h2's), on every guard pattern.
/// Which declaration bridge a copy runs on.
#[derive(Clone, Copy)]
enum D {
    Plain(dc::Declare),
    Sharray(ffx_panim::p2_declare::Sharray),
}

#[test]
fn registrations_match_the_original() {
    use ffx_panim::p2_declare as p2d;
    for mut t in Twins::all() {
        let mut rng = Rng(0x5005_b1d0);
        let found = t.zeros(4);
        t.stub(dc::ATEXIT, false, 1, 0, Ret::Imm(0x6003));
        t.stub(dc::REGISTER_TYPE, false, 1, 0, Ret::Imm(0x6001));
        t.stub(dc::ADD_TO_NAMESPACE, true, 0, 0, Ret::Imm(0x5a5a_1234));
        t.stub(h1b::ARRAY_MEMBER_CTOR, true, 7, 7, Ret::Imm(7));
        t.stub(h1b::CLASS_MEMBER_CTOR, true, 3, 3, Ret::Imm(3));
        t.stub(h1b::FIND_MEMBER, true, 1, 1, Ret::Cell(found));
        let k = p5m::P5_PSTRING_BIND;
        let mut p = Parts::default();
        h1_parts(k.steps, &mut p);
        for &(c, kind) in &p.calls {
            assert_eq!(kind, 0, "{c:#x}");
            t.stub(c, false, 0, 0, Ret::Imm(0x00c9_7000 | (c & 0xfff)));
        }
        for case in 0..12u32 {
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
            let v = rng.next() & 1;
            t.w32(found, v);
            t.check(
                &format!("PString::Bind case {case}"),
                0x0044_54a0,
                &[],
                |g| h1b::bridge::bind(g, k),
            );
        }
        // the declarations
        t.stub(dc::TYPE_UINT, false, 0, 0, Ret::Imm(0x00c9_0500));
        t.stub(dc::DATA_MEMBER_CTOR, true, 6, 6, Ret::Imm(6));
        t.stub(dc::ANNOTATION_CTOR, true, 3, 3, Ret::Imm(3));
        t.stub(dc::ADD_ANNOTATION, true, 1, 1, Ret::Imm(1));
        let all = [
            (0x0044_b3b0, D::Sharray(p5m::P5_PSTRING_SHARRAY)),
            (0x0054_ecb0, D::Sharray(p5m::P5_TRIGGER_RECEIVER_SHARRAY)),
            (0x00a3_3680, D::Plain(p5m::P5_INPUT_MAP_ARRAY)),
        ];
        for (f, k) in all {
            let (desc, ctor, name_global, annot) = match k {
                D::Plain(c) => (c.desc, c.ctor, c.name, 0xc4),
                D::Sharray(c) => (c.desc, c.ctor, c.name, 0x104),
            };
            t.stub(ctor, true, 0, 0, Ret::Imm(0x00c9_7000 | (ctor & 0xfff)));
            let d = t.rebase(desc);
            let ng = t.rebase(name_global);
            let annot = d + annot;
            for case in 0..16u32 {
                let guard = match case {
                    0 | 4 => 0,
                    1 => u32::MAX,
                    _ => rng.next() & rng.next(),
                };
                t.w32(d + 0x94, guard);
                let flags = rng.next();
                t.w32(d + 0x90, flags);
                let name = if case == 3 { 0 } else { rng.next() | 1 };
                let own = if case % 5 == 2 { rng.next() | 1 } else { 0 };
                t.w32(d + 0x18, own);
                let first = if rng.chance(2) { 0 } else { rng.next() | 1 };
                t.w32(ng, first);
                let link = if rng.chance(2) { annot + 4 } else { rng.next() };
                t.w32(annot + 4, link);
                for off in [0, 0x2c, 0x34, 0x38, 0x3c, 0x14] {
                    let v = rng.next();
                    t.w32(d + 0x98 + off, v);
                    let v = rng.next();
                    t.w32(d + 0xe0 + off, v);
                }
                let v = rng.next();
                t.w32(d, v);
                t.check(
                    &format!("declare {f:#x} case {case}"),
                    f,
                    &[name],
                    |g| match k {
                        D::Plain(c) => dc::bridge::declare(g, name, &c),
                        D::Sharray(c) => p2d::bridge::declare_sharray(g, name, &c),
                    },
                );
            }
        }
        t.unstub_all();
    }
}

/// The two `PClassDescriptorAbstract<T>` constructors (h2's bridge).
#[test]
fn abstract_descriptors_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x0ab5_7ac7);
        t.stub(dc::PBASE_DESCRIPTOR, false, 0, 0, Ret::Imm(0x00c9_0b45));
        t.stub(dc::GLOBAL_NAMESPACE, false, 0, 0, Ret::Imm(0x00c9_0a45));
        t.stub(dc::DESCRIPTOR_CTOR, true, 6, 6, Ret::Imm(0x6006));
        for f in [
            dc::SET_DEFAULT_BUFFER,
            dc::SET_VALIDATION_BUFFER,
            dc::SET_WRITE_MASK_BUFFER,
        ] {
            t.stub(f, true, 1, 1, Ret::Imm(f));
        }
        for (f, c) in [
            (0x0045_2e80, p5m::P5_PCAMERA_ABSTRACT),
            (0x004c_9160, p5m::P5_PSHADOWSPLIT_ABSTRACT),
        ] {
            for case in 0..3u32 {
                let this = t.random(&mut rng, 0x100);
                t.check_conv(
                    &format!("abstract {f:#x} case {case}"),
                    f,
                    Conv::Thiscall(this),
                    &[],
                    |g| dc::bridge::abstract_ctor(g, this, &c),
                );
            }
        }
        t.unstub_all();
    }
}

/// The singletons, on every state of their pointers and guards.
#[test]
fn singletons_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x5195_1e70);
        t.stub(dc::ATEXIT, false, 1, 0, Ret::Imm(0x6003));
        t.stub(p5m::THREAD_POOL_CTOR, true, 2, 2, Ret::Imm(0x44cb));
        t.stub(p5m::CRITICAL_SECTION_CTOR, true, 0, 0, Ret::Imm(0x4316));
        for case in 0..8u32 {
            let p = t.rebase(p5m::IMMEDIATE);
            let v = if case & 1 == 0 { 0 } else { rng.next() };
            t.w32(p, v);
            let g = t.rebase(p5m::IMMEDIATE_GUARD);
            let v = if case & 2 == 0 {
                rng.next() & !1
            } else {
                rng.next() | 1
            };
            t.w32(g, v);
            let s = t.rebase(p5m::IMMEDIATE_STATIC);
            let v = rng.next();
            t.w32(s, v);
            t.check(
                &format!("immediate pool case {case}"),
                0x0057_cf50,
                &[],
                |g| p5b::immediate_pool(g),
            );
            let g = t.rebase(p5m::RING_GUARD);
            let v = if case & 2 == 0 {
                rng.next() & !1
            } else {
                rng.next() | 1
            };
            t.w32(g, v);
            let r = t.rebase(p5m::RING);
            for i in 0..8 {
                let v = rng.next();
                t.w32(r + 0x14 + 4 * i, v);
            }
            t.check(&format!("ring buffer case {case}"), 0x005c_d010, &[], |g| {
                p5b::ring_buffer(g)
            });
        }
        t.stub(p5m::IMMEDIATE_GET, false, 0, 0, Ret::Imm(0x00cb_3428));
        t.stub(p5m::STEALING_CTOR, true, 2, 2, Ret::Imm(0x57d1));
        for case in 0..16u32 {
            let p = t.rebase(p5m::POOL);
            let v = if case % 4 == 3 { rng.next() } else { 0 };
            t.w32(p, v);
            let w = t.rebase(p5m::WORKERS);
            let n = [0, 1, 2, 7, u32::MAX][(case % 5) as usize];
            t.w32(w, n);
            let wa = t.rebase(p5m::WORKERS_ARG);
            let v = rng.next();
            t.w32(wa, v);
            let g = t.rebase(p5m::STEALING_GUARD);
            let v = if case & 4 == 0 {
                rng.next() & !1
            } else {
                rng.next() | 1
            };
            t.w32(g, v);
            t.check(&format!("thread pool case {case}"), 0x0044_cd30, &[], |g| {
                p5b::thread_pool(g)
            });
        }
        t.unstub_all();
    }
}

/// `PShadowCaster`'s constructor and descriptor callbacks, and
/// `PInputAction`'s destructor.
#[test]
fn objects_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x05ad_0ca5);
        t.stub(p5m::SAMPLER_CTOR, true, 0, 0, Ret::Imm(0x583b));
        t.stub(p5m::SAMPLER_DTOR, true, 0, 0, Ret::Imm(0x5852));
        t.stub(p5m::PHYRE_FREE, false, 1, 0, Ret::Imm(0x00f4_ee00));
        for case in 0..8u32 {
            let this = t.random(&mut rng, 0x80);
            let flags = if case % 2 == 0 {
                rng.next() & !0x80
            } else {
                rng.next() | 0x80
            };
            t.w32(this + 0x60, flags);
            t.check_conv(
                &format!("shadow caster ctor case {case}"),
                0x004c_9500,
                Conv::Thiscall(this),
                &[],
                |g| p5b::shadow_caster_ctor(g, this),
            );
        }
        for case in 0..12u32 {
            let p = t.random(&mut rng, 0x80);
            let owned = if case % 2 == 0 { 0 } else { 0x8000_0000 };
            t.w32(p + 8, owned | rng.below(5));
            let data = if case % 3 == 0 { 0 } else { rng.next() | 1 };
            t.w32(p + 0xc, data);
            t.check_conv(
                &format!("shadow caster destruct case {case}"),
                0x004c_c2e0,
                Conv::Stdcall,
                &[p],
                |g| p5b::shadow_caster_destruct(g, p),
            );
        }
        // the constructor stubbed: its eax under `al`
        t.stub(p5m::SHADOW_CASTER_CTOR, true, 0, 0, Ret::Imm(0x1234_5678));
        for case in 0..2u32 {
            let p = if case == 0 {
                0
            } else {
                t.random(&mut rng, 0x80)
            };
            let mask = if p == 0 { 0xff } else { u32::MAX };
            t.check_masked(
                &format!("shadow caster construct case {case}"),
                0x004c_c1f0,
                Conv::Stdcall,
                &[p],
                mask,
                |g| p5b::shadow_caster_construct(g, p),
            );
        }
        // PInputAction: elements deleted through their vtables
        for case in 0..17u32 {
            // case 16: 0x101 elements (the count's bits past the low byte)
            let n = if case == 16 { 0x101 } else { rng.below(5) };
            let els: Vec<u32> = (0..n)
                .map(|i| {
                    if rng.chance(3) {
                        0
                    } else {
                        t.object(&mut rng, 0x9000 + i, &[1], &[Ret::Imm(0x9e)], 8)
                    }
                })
                .collect();
            let bytes: Vec<u8> = els.iter().flat_map(|w| w.to_le_bytes()).collect();
            let data = if n == 0 && rng.chance(2) {
                0
            } else {
                t.alloc(&[bytes, vec![0; 4]].concat())
            };
            let this = t.random(&mut rng, 0x20);
            let owned = if case % 2 == 0 { 0 } else { 0x8000_0000 };
            t.w32(this + 8, owned | n);
            t.w32(this + 0xc, data);
            let b = if case % 4 < 2 {
                rng.next() & !1
            } else {
                rng.next() | 1
            };
            t.w32(this + 4, b);
            t.check_conv(
                &format!("input action dtor case {case}"),
                0x009e_2330,
                Conv::Thiscall(this),
                &[],
                |g| p5b::input_action_dtor(g, this),
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

/// The 4x4 transpose, in place too.
#[test]
fn transpose_matches_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x7a05_9053);
        for case in 0..8u32 {
            // two spare words: `out` may start one word into `m`
            let words: Vec<u8> = (0..18)
                .flat_map(|_| float(&mut rng).to_le_bytes())
                .collect();
            let m = t.alloc(&words);
            let out = match case % 3 {
                0 => m,
                1 => m + 4,
                _ => t.random(&mut rng, 0x48),
            };
            t.check(
                &format!("transpose case {case}"),
                0x004f_93f0,
                &[out, m],
                |g| p5b::transpose(g, out, m),
            );
        }
    }
}

/// `setLightMatrix`: the parameter lookup stubbed (its results from a
/// table, call by call), `PArray_Data` and `memcpy` original in both.
#[test]
fn set_light_matrix_matches_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x1197_7000);
        // the lookup: logs its `this` and name, returns rets[call index]
        let rets = t.zeros(4 * 64);
        let log = t.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.push(0x52); // push edx
        c.extend([0x8b, 0x14, 0x85]);
        c.extend(rets.to_le_bytes()); // mov edx, [rets + eax*4]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(p5m::FIND_PARAMETER.to_le_bytes()); // mov [eax], tag
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        c.extend([0x8b, 0x4c, 0x24, 0x08]); // mov ecx, [esp+8]
        c.extend([0x89, 0x48, 0x08]); // mov [eax+8], ecx
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.extend([0x89, 0xd0]); // mov eax, edx
        c.push(0x5a); // pop edx
        c.extend([0xc2, 0x04, 0x00]); // ret 4
        let find = t.code(&c);
        let at = t.rebase(p5m::FIND_PARAMETER);
        let mut j = vec![0xe9];
        j.extend(find.wrapping_sub(at + 5).to_le_bytes());
        t.write(at, &j);
        for case in 0..24u32 {
            let this = t.random(&mut rng, 0x200);
            let on = if case % 6 == 5 { 0 } else { 1 + rng.below(255) };
            t.write(this + 0x1cc, &[on as u8]);
            let n = rng.below(4);
            let mut words = vec![n | if rng.chance(4) { 0x8000_0000 } else { 0 }];
            let mut els = vec![];
            for _ in 0..n {
                // 0x40 = a 0x20 entry at 0x20 exactly: the bound is inclusive
                let size = if rng.chance(2) {
                    0x40
                } else {
                    0x40 + rng.below(0x40)
                };
                let buf = t.random(&mut rng, size as usize + 0x40);
                t.w32(buf, size);
                let e = t.random(&mut rng, 8);
                t.w32(e, rng.next());
                t.w32(e + 4, buf);
                els.push(e);
            }
            // PArray: one element inline, more through a pointer
            if n == 1 {
                words.push(els[0]);
            } else {
                let bytes: Vec<u8> = els.iter().flat_map(|w| w.to_le_bytes()).collect();
                let p = t.alloc(&[bytes, vec![0; 4]].concat());
                words.push(p);
            }
            let wb: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
            let arr = t.alloc(&wb);
            let node = t.random(&mut rng, 0x20);
            t.w32(node + 0x10, arr);
            // the lookups' entries: none, or {offset, size} that fit or not
            for k in 0..2 * n {
                let p = if rng.chance(4) {
                    0
                } else {
                    let p = t.random(&mut rng, 0x10);
                    // every third case's first lookups end exactly at a 0x40 buffer
                    let exact = case % 3 == 0 && k < 2;
                    let off = if exact {
                        0x20
                    } else {
                        [0, 2, 4, 8, 0x20, 0x3c][rng.below(6) as usize]
                    };
                    let size = if exact {
                        0x20
                    } else {
                        [0, 4, 0x10, 0x20, 0x2010, 0x80][rng.below(6) as usize]
                    };
                    t.write(p + 8, &(off as u16).to_le_bytes());
                    t.write(p + 0xa, &(size as u16).to_le_bytes());
                    p
                };
                t.w32(rets + 4 * k, p);
            }
            let a = if case % 4 == 1 {
                0
            } else {
                t.random(&mut rng, 0x80)
            };
            let b = if case % 4 == 2 {
                0
            } else {
                t.random(&mut rng, 0x80)
            };
            t.check_masked(
                &format!("setLightMatrix case {case}"),
                0x006a_abf0,
                Conv::Thiscall(this),
                &[node, a, b],
                0,
                |g| {
                    p5b::set_light_matrix(g, this, node, a, b);
                    0
                },
            );
        }
        t.unstub_all();
    }
}

/// A float near `scale` (both signs), or any float now and then.
fn sane(rng: &mut Rng, scale: f32) -> u32 {
    if rng.chance(12) {
        return float(rng);
    }
    #[allow(clippy::cast_precision_loss)]
    let x = (rng.below(20001) as f32 - 10000.0) / 10000.0 * scale;
    x.to_bits()
}

/// `n` words: mostly sane floats, all `float()` for `wild`.
fn floats(rng: &mut Rng, n: usize, wild: bool) -> Vec<u8> {
    (0..n)
        .flat_map(|_| if wild { float(rng) } else { sane(rng, 4.0) }.to_le_bytes())
        .collect()
}

/// The affine inverse, `PMatrix4_Inverse` around it (the originals of
/// `Vectormath_Transform3_inverse` and `PMatrix4_SetAffine` run in both
/// oracles: their locals are on the stack) and `PMatrix4_lookAt` (the
/// original `PMatrix4_FromAxes` in both).
#[test]
fn matrix_math_matches_the_original() {
    use ffx_pcore::p5_math::bridge as math;
    for mut t in Twins::all() {
        let mut rng = Rng(0x0a7f_1e55);
        for case in 0..48u32 {
            let m = t.alloc(&floats(&mut rng, 20, case % 6 == 5));
            let out = match case % 4 {
                0 => m,
                _ => t.random(&mut rng, 0x48),
            };
            t.check(
                &format!("transform3 inverse case {case}"),
                0x004c_c890,
                &[out, m],
                |g| math::transform3_inverse(g, out, m),
            );
        }
        for case in 0..32u32 {
            let m = t.alloc(&floats(&mut rng, 16, case % 6 == 5));
            let out = if case % 4 == 0 {
                m
            } else {
                t.random(&mut rng, 0x40)
            };
            t.check(
                &format!("inverse case {case}"),
                0x004c_c0c0,
                &[out, m],
                |g| {
                    let frame = g.0.stack_range().start + 0x400;
                    math::inverse(g, out, m, frame)
                },
            );
        }
        for case in 0..48u32 {
            let wild = case % 8 == 7;
            let p = t.alloc(&floats(&mut rng, 4, wild));
            let q = if case == 3 {
                p
            } else {
                t.alloc(&floats(&mut rng, 4, wild))
            };
            let u = t.alloc(&floats(&mut rng, 4, wild));
            let this = match case % 5 {
                0 => p,
                _ => t.random(&mut rng, 0x40),
            };
            t.check_conv(
                &format!("lookAt case {case} this {this:#x} p {p:#x} q {q:#x} u {u:#x}"),
                0x0045_1c30,
                Conv::Thiscall(this),
                &[p, q, u],
                |g| {
                    let frame = g.0.stack_range().start + 0x400;
                    math::look_at(g, this, p, q, u, frame)
                },
            );
        }
    }
}

/// A recording stub for methods handed pointers to the caller's locals:
/// logs, per entry of `words`, the stack word at that offset or
/// (`Some(k)`) the word at `k` bytes into what it points to; optionally
/// stores the word of `out.1` (a cell) through the pointer at stack
/// offset `out.0`; pops `pops` words and returns `ret`.
fn deref_stub(
    t: &mut Twins,
    tag: u32,
    words: &[(u8, Option<u32>)],
    out: Option<(u8, u32)>,
    pops: u32,
    ret: Ret,
) -> u32 {
    assert!(words.len() <= 14);
    let log = t.log;
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(tag.to_le_bytes()); // mov [eax], tag
    // no `ecx`: COM methods and the cdecl warning take everything on the
    // stack, and the callers leave `ecx` as it happens to be
    for (i, &(off, deref)) in (0u8..).zip(words) {
        c.extend([0x8b, 0x54, 0x24, off]); // mov edx, [esp+off]
        if let Some(k) = deref {
            c.extend([0x8b, 0x92]);
            c.extend(k.to_le_bytes()); // mov edx, [edx+k]
        }
        c.extend([0x89, 0x50, 8 + 4 * i]); // mov [eax+8+4i], edx
    }
    if let Some((off, cell)) = out {
        c.extend([0x8b, 0x54, 0x24, off]); // mov edx, [esp+off]
        c.extend([0x8b, 0x0d]);
        c.extend(cell.to_le_bytes()); // mov ecx, [cell]
        c.extend([0x89, 0x0a]); // mov [edx], ecx
    }
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    match ret {
        Ret::Imm(v) => {
            c.push(0xb8);
            c.extend(v.to_le_bytes());
        }
        Ret::Cell(at) => {
            c.push(0xa1);
            c.extend(at.to_le_bytes());
        }
    }
    c.push(0xc2);
    c.extend(((pops * 4) as u16).to_le_bytes());
    t.code(&c)
}

/// A COM object: its vtable's slots `slots[i]` (offset, stub address),
/// every other slot 0; `size` random bytes after the vtable word.
fn com_object(t: &mut Twins, rng: &mut Rng, slots: &[(u32, u32)], size: u32) -> u32 {
    let mut vt = [0u32; 0x20];
    for &(off, stub) in slots {
        vt[(off / 4) as usize] = stub;
    }
    let bytes: Vec<u8> = vt.iter().flat_map(|w| w.to_le_bytes()).collect();
    let vt = t.alloc(&bytes);
    let o = t.random(rng, size as usize);
    t.w32(o, vt);
    o
}

/// `GDraw`'s helpers: the immediate context and the texture's views as COM
/// objects with recording methods (the vertex-buffer call's pointers to
/// the caller's locals logged by contents), the blend-state call's
/// description logged by contents and its result written through.
#[test]
fn gdraw_helpers_match_the_original() {
    use ffx_pcore::p5_gdraw::{self as gd, bridge as gdb};
    for mut t in Twins::all() {
        let mut rng = Rng(0x6d4a_3d11);
        // the context: slots 0x20 (4 words), 0x48 (6; three pointers to
        // the caller's locals), 0x4c (4)
        let plain = [(4, None), (8, None), (12, None), (16, None)];
        let s20 = deref_stub(&mut t, 0x20, &plain, None, 4, Ret::Imm(0));
        let s48 = deref_stub(
            &mut t,
            0x48,
            &[
                (4, None),
                (8, None),
                (12, None),
                (16, Some(0)),
                (20, Some(0)),
                (24, Some(0)),
            ],
            None,
            6,
            Ret::Imm(0),
        );
        let s4c = deref_stub(&mut t, 0x4c, &plain, None, 4, Ret::Imm(0));
        let ctx = com_object(
            &mut t,
            &mut rng,
            &[(0x20, s20), (0x48, s48), (0x4c, s4c)],
            0x10,
        );
        let state = t.random(&mut rng, 0x20);
        t.w32(state + 4, ctx);
        let g = t.rebase(gd::STATE);
        t.w32(g, state);
        for case in 0..24u32 {
            let texture = t.random(&mut rng, 0x40);
            for (k, off) in [0u32, 4, 8].into_iter().enumerate() {
                let o = if case == 4 || rng.chance(3) {
                    0
                } else {
                    let r = rng.next();
                    let rel =
                        deref_stub(&mut t, 0x100 + k as u32, &[(4, None)], None, 1, Ret::Imm(r));
                    com_object(&mut t, &mut rng, &[(8, rel)], 8)
                };
                t.w32(texture + off, o);
            }
            let kind = if case % 6 == 5 {
                0x5000_0000
            } else {
                rng.next() & 0xf000_0000
            };
            t.w32(texture + 0x30, kind | (rng.next() & 0x0fff_ffff));
            let info = t.random(&mut rng, 0x10);
            t.write(
                info + 0xc,
                &[(rng.next() as u8 & !1) | u8::from(case % 2 == 1)],
            );
            t.w32(texture + 0x18, info);
            t.check(
                &format!("unbind case {case}"),
                0x00a3_4bc0,
                &[texture],
                |g| {
                    let frame = g.0.stack_range().start + 0x400;
                    // junk where the locals go: the port must set what it uses
                    Mem::write_bytes(g, frame, &[0xa5; 8]);
                    gdb::unbind(g, texture, frame)
                },
            );
        }
        // the blend state: the device's slot 0x50 logs the description's
        // words (and two past the ones set) and writes the result through
        // its third word; the warning (an import slot, cdecl) logs its five
        let hr = t.zeros(4);
        let result = t.zeros(4);
        // the twelve words set and the last two memset clears
        let desc: Vec<(u8, Option<u32>)> = (0..12u32)
            .chain([0x40, 0x41])
            .map(|k| (8, Some(4 * k)))
            .collect();
        let s50 = deref_stub(&mut t, 0x50, &desc, Some((12, result)), 3, Ret::Cell(hr));
        let dev = com_object(&mut t, &mut rng, &[(0x50, s50)], 0x10);
        // the warning takes four words when out of memory, five (the
        // code last) otherwise: a stub for each, so none logs the
        // caller's leftovers
        let words = [(4, None), (8, None), (12, None), (16, None), (20, None)];
        let warn4 = deref_stub(&mut t, 0x874, &words[..4], None, 0, Ret::Imm(0x77));
        let warn5 = deref_stub(&mut t, 0x875, &words, None, 0, Ret::Imm(0x78));
        let slot = t.rebase(gd::WARN_SLOT);
        for case in 0..16u32 {
            let h = match case % 4 {
                0 => 0,
                1 => [1, 0x7fff_ffff][rng.below(2) as usize],
                2 => gd::E_OUTOFMEMORY,
                _ => 0x8000_0000 | rng.next(),
            };
            t.w32(slot, if h == gd::E_OUTOFMEMORY { warn4 } else { warn5 });
            t.w32(hr, h);
            let r = rng.next();
            t.w32(result, r);
            let a = rng.below(20);
            let b = if case % 3 == 0 { 9 } else { rng.below(20) };
            let c = rng.below(20);
            t.check(
                &format!("blend state case {case}"),
                0x00a3_5670,
                &[dev, a, b, c],
                |g| {
                    let frame = g.0.stack_range().start + 0x400;
                    Mem::write_bytes(g, frame, &[0xa5; 0x110]);
                    gdb::create_blend_state(g, dev, a, b, c, frame)
                },
            );
        }
        t.unstub_all();
    }
}

/// The ring buffer's allocator: `fits` stubbed (its answers from a table,
/// call by call), the lock, sleep and printf stubbed; rings whose write
/// position is near the end or not, with chains of busy and free blocks
/// to retire.
#[test]
fn ring_allocate_matches_the_original() {
    use ffx_pcore::p5_ring::{self as ring, bridge as rb};
    for mut t in Twins::all() {
        let mut rng = Rng(0x0819_b10c);
        // fits: logs `this` and n, returns answers[call index]
        let answers = t.zeros(4 * 0x800);
        let log = t.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.push(0x52); // push edx
        c.extend([0x8b, 0x14, 0x85]);
        c.extend(answers.to_le_bytes()); // mov edx, [answers + eax*4]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(ring::FITS.to_le_bytes()); // mov [eax], tag
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        c.extend([0x8b, 0x4c, 0x24, 0x08]); // mov ecx, [esp+8]
        c.extend([0x89, 0x48, 0x08]); // mov [eax+8], ecx
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.extend([0x89, 0xd0]); // mov eax, edx
        c.push(0x5a); // pop edx
        c.extend([0xc2, 0x04, 0x00]); // ret 4
        let fits = t.code(&c);
        let at = t.rebase(ring::FITS);
        let mut j = vec![0xe9];
        j.extend(fits.wrapping_sub(at + 5).to_le_bytes());
        t.write(at, &j);
        t.stub(ring::LOCK, true, 0, 0, Ret::Imm(0x1920));
        t.stub(ring::UNLOCK, true, 0, 0, Ret::Imm(0x1a80));
        // sleep: a bare `ret` (a long wait would fill the log)
        let ret = t.code(&[0xc3]);
        let at = t.rebase(ring::SLEEP);
        let mut j = vec![0xe9];
        j.extend(ret.wrapping_sub(at + 5).to_le_bytes());
        t.write(at, &j);
        t.stub(ring::PRINTF, false, 3, 0, Ret::Imm(0));
        for case in 0..40u32 {
            let size = 0x400;
            let buf = t.random(&mut rng, size as usize + 0x40);
            let r = t.random(&mut rng, 0x40);
            t.w32(r + 0x18, buf);
            t.w32(r + 0x1c, buf + size);
            t.w32(r + 0x20, size);
            // the write position: near the end (a filler block first) or
            // not, 16-aligned; 0 now and then
            let used = match case % 5 {
                0 => buf + size - 0x20,
                1 => buf + size - 0x40,
                4 if case % 10 == 4 => 0,
                _ => buf + 0x10 * rng.below(0x30),
            };
            t.w32(r + 0x24, used);
            // blocks to retire from +0x28: a chain ending at the write
            // position, some busy
            let n = rng.below(4);
            let mut next = used;
            for k in 0..n {
                let b = buf + 0x300 + 0x20 * (3 - k);
                t.w32(b, next);
                t.w32(b + 4, u32::from(rng.chance(3)));
                next = b;
            }
            t.w32(r + 0x28, next);
            // the write position's block not busy, its next the (busy) start:
            // retiring must stop at the write position, not run past it
            if used != 0 {
                t.w32(used, buf + 0x10);
                t.w32(used + 4, 0);
                t.w32(buf + 0x14, 1);
            }
            // fits' answers (index 0 is the lock): the first random, then
            // "busy" for a while, sometimes a wait of a few
            // rounds, once past a multiple of 1000
            // (log entries: the lock, fits and printf calls; sleep is not logged)
            let waits = if case == 7 { 1010 } else { rng.below(5) };
            for k in 0..0x800u32 {
                let v = if k == 1 {
                    u32::from(case == 7 || !rng.chance(3))
                } else {
                    u32::from(k <= waits + 2)
                };
                let v = v | (rng.next() & 0xffff_ff00);
                t.w32(answers + 4 * k, v);
            }
            let want =
                [0, 1, 0x10, 0x11, 0x3e0, 0x3f0, 0x400, rng.below(0x200)][(case % 8) as usize];
            t.check_conv(
                &format!("allocate case {case}"),
                0x005c_d110,
                Conv::Thiscall(r),
                &[want],
                |g| rb::allocate_block(g, r, want),
            );
        }
        t.unstub_all();
    }
}

// ---- round 4: the per-instance light selection (p5_lights) ----

/// A `PArray` of `words` at a fresh block: one element inline, more at a
/// block of their own.
fn p5e_array(t: &mut Twins, words: &[u32], flag: u32) -> u32 {
    let arr = t.zeros(8);
    t.w32(arr, words.len() as u32 | flag);
    if words.len() == 1 {
        t.w32(arr + 4, words[0]);
    } else if !words.is_empty() {
        let mut b = Vec::new();
        for w in words {
            b.extend(w.to_le_bytes());
        }
        let data = t.alloc(&b);
        t.w32(arr + 4, data);
    }
    arr
}

#[test]
fn pack_lights_matches_the_original() {
    use ffx_pcore::p5_lights::bridge as pl;
    for mut t in Twins::all() {
        let mut rng = Rng(0x4d41_4000);
        let skip = t.r32(t.rebase(0x00c1_60ec));
        for case in 0..240u32 {
            let masks = t.zeros(0x40);
            t.w32(masks, [0xffff_ffffu32, 1, 6, 0][(case % 4) as usize]);
            t.w32(masks + 4, [0xffff_ffffu32, 2, 0][(case / 4 % 3) as usize]);
            t.w32(
                masks + 0x38,
                [8u32, 3, 1, 0, 0x100][(case / 3 % 5) as usize],
            );
            if case % 16 == 15 {
                // 40 lights, all kept: more than the sort's insertion
                // threshold (its third word, the depth budget, matters)
                t.w32(masks, u32::MAX);
                t.w32(masks + 0x38, [0x100u32, 20][(case / 16 % 2) as usize]);
            }
            // the lights
            let mut lists = [0u32; 2];
            for l in &mut lists {
                let n = if case % 16 == 15 { 40 } else { rng.below(14) };
                let mut lights = Vec::new();
                for _ in 0..n {
                    let light = t.zeros(0x20);
                    let ty = t.zeros(0x20);
                    t.w32(ty + 0xc, 1 << rng.below(4));
                    t.w32(ty + 0x10, rng.below(32));
                    t.w32(light + 0x18, ty);
                    if !rng.chance(3) {
                        let holder = t.zeros(8);
                        if !rng.chance(3) {
                            let shadow = t.zeros(0x20);
                            t.w32(shadow + 0xc, 1 << rng.below(3));
                            t.w32(shadow + 0x10, rng.below(32));
                            t.w32(holder + 4, shadow);
                        }
                        t.w32(light + 0x14, holder);
                    }
                    lights.push(light);
                }
                let mut b = Vec::new();
                for w in &lights {
                    b.extend(w.to_le_bytes());
                }
                b.extend([0; 4]);
                let data = t.alloc(&b);
                *l = n | if rng.chance(4) { 0x8000_0000 } else { 0 };
                let at = t.zeros(8);
                t.w32(at, *l);
                t.w32(at + 4, data);
                *l = at;
            }
            let instance = t.zeros(0x100);
            for (k, &l) in lists.iter().enumerate() {
                let w0 = t.r32(l);
                let w1 = t.r32(l + 4);
                t.w32(instance + 0x9c + 8 * k as u32, w0);
                t.w32(instance + 0xa0 + 8 * k as u32, w1);
            }
            let fill: Vec<u8> = (0..0x40).map(|_| rng.next() as u8).collect();
            t.write(instance + 0xac, &fill);
            // the switches
            let mut switches = Vec::new();
            for _ in 0..rng.below(4) {
                let sw = t.zeros(0x20);
                let data = t.zeros(0x60);
                t.w32(data, [0x40u32, 0x20, 8][rng.below(3) as usize]);
                t.w32(sw + 4, data);
                if rng.chance(3) {
                    t.w32(sw + 0x14, rng.below(0x40));
                } else {
                    let block = t.zeros(0x10);
                    let off = [0u16, 3, 4, 8, 0x1c][rng.below(5) as usize];
                    let len = [0u16, 4, 0x10, 0x20, 0xe010][rng.below(5) as usize];
                    t.write(block + 8, &off.to_le_bytes());
                    t.write(block + 0xa, &len.to_le_bytes());
                    t.w32(sw + 0x14, block);
                }
                switches.push(sw);
            }
            let segments = t.zeros(0x20);
            let arr = p5e_array(&mut t, &switches, 0);
            t.w32(segments + 0x10, arr);
            let stream = t.random(&mut rng, 0x40);
            let words = t.zeros(4);
            t.w32(words, stream);
            let out = t.random(&mut rng, 0x60);
            let mode = if case % 5 == 2 { skip } else { skip ^ 1 };
            // the pairs and the bit writer's word on the port's stack
            // (as the original's _alloca and frame word): not compared
            let cursor = t.b.stack_range().end - 0x400;
            let pairs = cursor - 0x200;
            t.b.set_call_top(Some(pairs - 0x100));
            let args = [masks, segments, instance, words, out, 0x5a5a, mode];
            t.check_conv(
                &format!("pack lights {case}"),
                0x004d_4140,
                Conv::Stdcall,
                &args,
                |g| pl::pack_lights(g, args, pairs, cursor),
            );
            t.b.set_call_top(None);
        }
    }
}
