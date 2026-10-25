//! `ffx_pother_a` (M4 port round 1, list `p3-other-a`) against the
//! original: the type system's lookups, class registration, small
//! constructors and the class descriptors' create/destroy callbacks. On
//! the image alone with synthetic data, and on a snapshot
//! (`FFX_SNAPSHOT`) with the same data over the snapshot's memory.
//! Results and every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! Callees outside the list (the class descriptor, the type dispenser,
//! the heap, the tree) are replaced in both oracles alike by recording
//! stubs: each call appends its tag, `ecx` and six stack words to a log in
//! memory, so the diff checks that both made the same calls, in the same
//! order, with the same `this`.

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
use ffx_pother_a::{class, object};

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

/// What a stub returns.
#[derive(Clone, Copy)]
enum StubRet {
    /// A constant.
    Value(u32),
    /// `[ecx]`.
    ThisWord,
}

struct Twins {
    label: &'static str,
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 8 words (tag, `ecx`, six
    /// stack words).
    log: u32,
    /// Original bytes of each stubbed address.
    saved: Vec<(u32, Vec<u8>)>,
}

impl Twins {
    fn new(label: &'static str, mut a: Oracle, mut b: Oracle) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        a.set_max_instructions(2_000_000);
        b.set_max_instructions(2_000_000);
        let mut t = Self {
            label,
            a,
            b,
            log: 0,
            saved: Vec::new(),
        };
        t.log = t.zeros(0x4000);
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

    /// A block of `n` random bytes.
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

    /// A C string.
    fn string(&mut self, s: &str) -> u32 {
        let mut v = s.as_bytes().to_vec();
        v.push(0);
        self.alloc(&v)
    }

    fn code(&mut self, c: &[u8]) -> u32 {
        let x = self.a.alloc_code(c).unwrap();
        assert_eq!(self.b.alloc_code(c).unwrap(), x);
        x
    }

    /// A recording stub (logs `tag`, `ecx`, `n` stack words; pops `pops`
    /// words; returns `ret`). Returns its address.
    fn stub_code(&mut self, tag: u32, n: u8, pops: u32, ret: StubRet) -> u32 {
        // cdecl callees: `ecx` is whatever the caller left, not an argument
        let this = !CDECL.contains(&tag) && !class::SHADER_PASS_INIT.iter().any(|&(f, _)| f == tag);
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        if this {
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        }
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, 4 + 4 * i]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, 8 + 4 * i]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        match ret {
            StubRet::Value(v) => {
                c.push(0xb8);
                c.extend(v.to_le_bytes()); // mov eax, v
            }
            StubRet::ThisWord => c.extend([0x8b, 0x01]), // mov eax, [ecx]
        }
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
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
    fn stub(&mut self, sym: u32, n: u8, pops: u32, ret: StubRet) {
        let s = self.stub_code(sym, n, pops, ret);
        self.jump(sym, s);
    }

    /// Puts back the original at `sym`.
    fn unstub(&mut self, sym: u32) {
        let at = self.rebase(sym);
        let i = self
            .saved
            .iter()
            .position(|(a, _)| *a == at)
            .expect("stubbed");
        let (_, bytes) = self.saved.remove(i);
        self.write(at, &bytes);
    }

    /// Puts back every stubbed original.
    fn unstub_all(&mut self) {
        for (at, bytes) in std::mem::take(&mut self.saved) {
            self.write(at, &bytes);
        }
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
        let rb = port(&mut G(&mut self.b));
        assert_eq!(
            ra.eax & mask,
            rb & mask,
            "{} {what}: eax {:#x} vs {:#x}",
            self.label,
            ra.eax,
            rb
        );
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

    /// The number of calls logged so far.
    fn calls(&mut self) -> u32 {
        let log = self.log;
        self.r32(log)
    }
}

/// Stubbed cdecl callees (their stubs do not log `ecx`).
const CDECL: [u32; 8] = [
    ffx_pother_a::PHYRE_FREE,
    object::SLEEP,
    class::REGISTER_TYPE,
    class::GLOBAL_NAMESPACE,
    class::LOD_TYPES[0],
    class::LOD_TYPES[1],
    class::LOD_BINDS[0],
    class::LOD_BINDS[1],
];

const ALL: u32 = u32::MAX;
const AL: u32 = 0xff;

#[test]
fn constructors_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x1234_5678);
        let d = t.rebase(object::PBASE_DESCRIPTOR);
        t.check(
            "GetClassDescriptor",
            0x0043_5740,
            Conv::Cdecl,
            &[],
            ALL,
            |g| object::bridge::pbase_descriptor(g),
        );
        assert_eq!(d, t.rebase(0x00c9_02e8));
        for v in [0, 1, 0xdead_beef] {
            t.check("store c0b95c", 0x0044_cdf0, Conv::Cdecl, &[v], ALL, |g| {
                object::bridge::set_global(g, 0x00c0_b95c, v)
            });
            t.check("store c0b958", 0x0044_ce00, Conv::Cdecl, &[v], ALL, |g| {
                object::bridge::set_global(g, 0x00c0_b958, v)
            });
        }
        for _ in 0..4 {
            let p = t.random(&mut rng, 0x40);
            let v = rng.next();
            let w = rng.next();
            let x = rng.next();
            t.check("ref_init", 0x0044_d6a0, Conv::Thiscall(p), &[v], ALL, |g| {
                object::bridge::ref_init(g, p, v)
            });
            let p = t.random(&mut rng, 0x40);
            t.check("pair_init", 0x0046_77d0, Conv::Thiscall(p), &[], ALL, |g| {
                object::bridge::pair_init(g, p)
            });
            let p = t.random(&mut rng, 0x40);
            t.check(
                "quad_init",
                0x0057_0830,
                Conv::Thiscall(p),
                &[v],
                ALL,
                |g| object::bridge::quad_init(g, p, v),
            );
            let p = t.random(&mut rng, 0x40);
            t.check("pair_set", 0x0057_5870, Conv::Thiscall(p), &[v], ALL, |g| {
                object::bridge::pair_set(g, p, v)
            });
            let p = t.random(&mut rng, 0x40);
            t.check(
                "setTokens",
                0x0057_0a60,
                Conv::Thiscall(p),
                &[v, w],
                ALL,
                |g| object::bridge::enum_set_tokens(g, p, v, w),
            );
            let p = t.random(&mut rng, 0x40);
            t.check("orphanAll", 0x0057_4970, Conv::Thiscall(p), &[], 0, |g| {
                object::bridge::tree_orphan_all(g, p);
                0
            });
            let p = t.random(&mut rng, 0x40);
            let m = t.random(&mut rng, 0x40);
            t.check(
                "dynamic type",
                0x0057_5440,
                Conv::Thiscall(p),
                &[m],
                ALL,
                |g| object::bridge::dynamic_type_init(g, p, m),
            );
            let p = t.random(&mut rng, 0x40);
            t.check(
                "annotation",
                0x0057_5840,
                Conv::Thiscall(p),
                &[v, w, x],
                ALL,
                |g| object::bridge::annotation_init(g, p, v, w, x),
            );
            // a list with one node, then another annotation linked first
            let list = t.zeros(8);
            let first = t.random(&mut rng, 0x20);
            t.w32(list, first);
            let ann = t.random(&mut rng, 0x20);
            t.check(
                "addAnnotation",
                0x0057_5c10,
                Conv::Thiscall(list),
                &[ann],
                ALL,
                |g| object::bridge::add_annotation(g, list, ann),
            );
        }
        // create callbacks: null and a block
        let big = t.random(&mut rng, 0x300);
        let carry = ((big + 0xff) & !0xff) + 0xf8;
        for p in [0, t.random(&mut rng, 0x40), carry] {
            for sym in [0x0050_3ad0, 0x0054_9d50, 0x0054_fda0] {
                t.check("create_pair", sym, Conv::Stdcall, &[p], ALL, |g| {
                    object::bridge::create_pair(g, p)
                });
            }
            t.check(
                "create_locator",
                0x0054_6040,
                Conv::Stdcall,
                &[p],
                ALL,
                |g| object::bridge::create_locator_ref(g, p),
            );
            t.check(
                "create_ref_tree",
                0x0044_ffe0,
                Conv::Stdcall,
                &[p],
                ALL,
                |g| object::bridge::create_ref_tree(g, p),
            );
            t.check("create_pbase", 0x0044_dff0, Conv::Stdcall, &[p], AL, |g| {
                u32::from(object::bridge::create_pbase_ref(g, p))
            });
            t.check("true 503b60", 0x0050_3b60, Conv::Stdcall, &[p], AL, |_| 1);
            t.check("true 5511f0", 0x0055_11f0, Conv::Stdcall, &[p], AL, |_| 1);
            if p != 0 {
                let dsc = rng.next();
                t.check(
                    "scriptable",
                    0x0055_2c00,
                    Conv::Thiscall(p),
                    &[dsc],
                    ALL,
                    |g| object::bridge::scriptable_init(g, p, dsc),
                );
            }
        }
        // the listed reference: a list head with one node (the image's is
        // empty memory)
        let head = t.rebase(object::NAME_LIST);
        if t.r32(head) == 0 {
            let n = t.zeros(8);
            t.w32(head, n);
            t.w32(n + 4, head);
        }
        for p in [0, t.random(&mut rng, 0x40)] {
            t.check("create_listed", 0x0045_b050, Conv::Stdcall, &[p], AL, |g| {
                u32::from(object::bridge::create_listed_ref(g, p))
            });
        }
    }
}

#[test]
fn destroy_callbacks_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x0bad_cafe);
        t.stub(ffx_pother_a::PHYRE_FREE, 1, 0, StubRet::Value(0x1111));
        t.stub(object::UNLINK_OWNER, 1, 1, StubRet::Value(0x2222));
        t.stub(object::NODE_RELEASE, 0, 0, StubRet::Value(0x3333));
        t.stub(object::REF_RELEASE, 0, 0, StubRet::Value(0x4444));
        t.stub(0x0054_72c0, 0, 0, StubRet::Value(0x5555));
        t.stub(0x0054_e0f0, 0, 0, StubRet::Value(0x6666));
        t.stub(0x0054_7580, 0, 0, StubRet::Value(0x7777));
        t.stub(0x0055_3a70, 0, 0, StubRet::Value(0x8888));
        t.stub(object::WAIT_RELEASE, 0, 0, StubRet::Value(0x9999));
        t.stub(object::TREE_REMOVE, 1, 1, StubRet::Value(0xaaaa));
        for round in 0..18u32 {
            let p = t.random(&mut rng, 0x40);
            // owner / pointer words with both low bits and null
            let word = match round % 4 {
                0 => 0,
                1 => rng.next() | 1,
                2 => rng.next() & !1,
                _ => rng.next(),
            };
            t.w32(p + 4, word);
            let count = match round % 6 {
                0 => 0,
                1 => 1,
                2 => 2,
                3 => 0x8000_0005,
                4 => 0x8000_0000,
                5 if round % 4 == 1 => 0x4000_0003,
                _ => rng.next(),
            };
            t.w32(p, count);
            t.w32(p + 8, if round % 3 == 0 { 0 } else { rng.next() });
            let n0 = t.calls();
            t.check("unlink", 0x0044_e050, Conv::Stdcall, &[p], AL, |g| {
                u32::from(object::bridge::destroy_unlink(g, p))
            });
            t.check("node", 0x0045_0020, Conv::Stdcall, &[p], AL, |g| {
                u32::from(object::bridge::destroy_node(g, p))
            });
            t.check("ref", 0x0055_2850, Conv::Stdcall, &[p], AL, |g| {
                u32::from(object::bridge::destroy_ref(g, p))
            });
            t.check("ref_c", 0x0055_6980, Conv::Stdcall, &[p], AL, |g| {
                u32::from(object::bridge::destroy_ref_c(g, p))
            });
            t.check("owned_ptr", 0x0054_6070, Conv::Stdcall, &[p], AL, |g| {
                u32::from(object::bridge::destroy_owned_ptr(g, p))
            });
            for q in [p, 0] {
                t.check("call 549d70", 0x0054_9d70, Conv::Stdcall, &[q], AL, |g| {
                    u32::from(object::bridge::destroy_call(g, 0x0054_72c0, q))
                });
                t.check("call 54fdc0", 0x0054_fdc0, Conv::Stdcall, &[q], AL, |g| {
                    u32::from(object::bridge::destroy_call(g, 0x0054_e0f0, q))
                });
            }
            let flags = rng.below(4);
            t.check(
                "scalar 547d20",
                0x0054_7d20,
                Conv::Thiscall(p),
                &[flags],
                ALL,
                |g| object::bridge::scalar_delete(g, 0x0054_7580, p, flags),
            );
            t.check(
                "scalar 553d20",
                0x0055_3d20,
                Conv::Thiscall(p),
                &[flags],
                ALL,
                |g| object::bridge::scalar_delete(g, 0x0055_3a70, p, flags),
            );
            // arrays: rewritten before each, as each clears its words
            for (sym, at) in [
                (0x0054_9dc0, None),
                (0x0054_fdf0, Some(0)),
                (0x0054_fe30, Some(4)),
            ] {
                t.w32(p, count);
                t.w32(p + 4, word);
                t.w32(p + 8, rng.next());
                t.check("array", sym, Conv::Stdcall, &[p], AL, |g| {
                    u32::from(match at {
                        None => object::bridge::destroy_array(g, p),
                        Some(at) => object::bridge::destroy_array_gt1(g, p, at),
                    })
                });
                if at == Some(4) {
                    t.w32(p + 4, count);
                    t.check("array +4", sym, Conv::Stdcall, &[p], AL, |g| {
                        u32::from(object::bridge::destroy_array_gt1(g, p, 4))
                    });
                }
            }
            // the listed reference: a node at +0xc between two others
            let prev = t.zeros(8);
            let next = t.zeros(8);
            t.w32(p + 0xc, next);
            t.w32(p + 0x10, prev);
            t.w32(p + 0x14, word);
            t.check(
                "destroy_listed",
                0x0045_b0d0,
                Conv::Stdcall,
                &[p],
                AL,
                |g| u32::from(object::bridge::destroy_listed_ref(g, p)),
            );
            assert!(t.calls() > n0);
        }
        // virtual destructors: the object's vtable slot 0 is a stub
        let dtor = t.stub_code(0xd700, 1, 1, StubRet::Value(0xd7));
        let vt = t.zeros(8);
        t.w32(vt, dtor);
        for sym in [0x0054_9e00, 0x0055_4690, 0x0055_5c30] {
            let o = t.random(&mut rng, 0x20);
            t.w32(o, vt);
            t.check("virtual", sym, Conv::Stdcall, &[o + 4], AL, |g| {
                u32::from(object::bridge::destroy_virtual(g, o + 4))
            });
        }
        // the idle wait: Sleep decrements the count it waits on
        let p = t.random(&mut rng, 0x20);
        let mut c = vec![0xff, 0x0d];
        c.extend((p + 4).to_le_bytes()); // dec dword [p+4]
        c.push(0xc3);
        let sleep = t.code(&c);
        let sleep_log = t.stub_code(object::SLEEP, 1, 0, StubRet::Value(0));
        // log the call, then decrement
        let at = t.code(&[0; 32]);
        let mut c2 = vec![0xff, 0x74, 0x24, 0x04, 0xe8]; // push dword [esp+4]
        c2.extend(sleep_log.wrapping_sub(at + 9).to_le_bytes()); // call log
        c2.extend([0x83, 0xc4, 0x04, 0xe9]); // add esp, 4
        c2.extend(sleep.wrapping_sub(at + 17).to_le_bytes()); // jmp dec
        t.write(at, &c2);
        t.jump(object::SLEEP, at);
        for count in [0u32, 3, 0x8000_0000, 1] {
            t.w32(p + 4, count);
            t.check("when_idle", 0x0050_3af0, Conv::Stdcall, &[p], AL, |g| {
                u32::from(object::bridge::destroy_when_idle(g, p))
            });
        }
        // node release (the original, not its stub): children lists,
        // with and without a parent
        t.unstub(object::NODE_RELEASE);
        t.unstub(object::REF_RELEASE);
        for owner in [0, rng.next(), 1] {
            let p = t.random(&mut rng, 0x10);
            t.w32(p + 4, owner);
            t.check(
                "ref_release",
                0x0044_d730,
                Conv::Thiscall(p),
                &[],
                ALL,
                |g| object::bridge::ref_release(g, p),
            );
        }
        for round in 0..6u32 {
            let n = t.random(&mut rng, 0x20);
            t.w32(n, 0);
            t.w32(n + 4, 0);
            let kids = round % 4;
            let mut prev = n;
            for _ in 0..kids {
                let k = t.zeros(8);
                t.w32(prev, k);
                t.w32(k + 4, n);
                prev = k;
            }
            if round % 2 == 1 {
                t.w32(n + 4, rng.next() | 1);
            }
            t.check(
                "node_release",
                0x0044_ede0,
                Conv::Thiscall(n),
                &[],
                ALL,
                |g| object::bridge::node_release(g, n),
            );
        }
        t.unstub_all();
    }
}

#[test]
fn lookups_match_the_original() {
    for mut t in Twins::all() {
        let mut rng = Rng(0x7777_1111);
        let names = ["", "a", "ab", "abc", "abd", "name", "names", "PBase"];
        let strs: Vec<u32> = names.iter().map(|s| t.string(s)).collect();
        let queries: Vec<u32> = ["a", "ab", "abc", "abd", "x", "name", "nam", "PBase", ""]
            .iter()
            .map(|s| t.string(s))
            .collect();
        // a member list at +0x4c (sentinel head), some null names
        let class = t.zeros(0x60);
        let head = class + 0x4c;
        let mut prev = head;
        for (i, &s) in strs.iter().enumerate() {
            let e = t.zeros(0x10);
            t.w32(prev, e);
            t.w32(e + 0xc, if i == 2 { 0 } else { s });
            prev = e;
        }
        t.w32(prev, head);
        let empty = t.zeros(0x60);
        t.w32(empty + 0x4c, empty + 0x4c);
        t.w32(empty + 0x58, queries[0]);
        for &q in queries.iter().chain([0u32].iter()) {
            for c in [class, empty] {
                t.check(
                    "findMember",
                    0x0043_59d0,
                    Conv::Thiscall(c),
                    &[q],
                    ALL,
                    |g| class::bridge::find_member(g, c, q),
                );
            }
        }
        // the named lists: the real ones (snapshot), then synthetic ones
        for (sym, list, name_at, next_at) in [
            (0x0043_6350u32, class::NAMED_LIST, 0u32, 4u32),
            (0x0054_5840, class::NAMED_LIST2, 4, 8),
        ] {
            let g = t.rebase(list);
            let real = t.r32(g);
            let mut qs = queries.clone();
            qs.push(0);
            // the real list's first and last names
            let mut e = real;
            let mut steps = 0;
            while e != 0 && steps < 200 {
                if steps == 0 || t.r32(e + next_at) == 0 {
                    qs.push(t.r32(e + name_at));
                }
                e = t.r32(e + next_at);
                steps += 1;
            }
            for &q in &qs {
                t.check("findNamed real", sym, Conv::Cdecl, &[q], ALL, |g| {
                    if sym == 0x0043_6350 {
                        class::bridge::find_named(g, q)
                    } else {
                        class::bridge::find_named2(g, q)
                    }
                });
            }
            let mut first = 0;
            for &s in strs.iter().rev() {
                let e = t.zeros(0xc);
                t.w32(e + name_at, s);
                t.w32(e + next_at, first);
                first = e;
            }
            t.w32(g, first);
            for &q in &qs {
                t.check("findNamed", sym, Conv::Cdecl, &[q], ALL, |g| {
                    if sym == 0x0043_6350 {
                        class::bridge::find_named(g, q)
                    } else {
                        class::bridge::find_named2(g, q)
                    }
                });
            }
            t.w32(g, real);
        }
        // the 32-byte tables
        for round in 0..8u32 {
            let n = round % 5;
            let data = t.random(&mut rng, (n as usize).max(1) * 0x20);
            let keys: Vec<u32> = (0..n).map(|i| t.r32(data + 0x20 * i + 8)).collect();
            let table = t.random(&mut rng, 0x20);
            t.w32(
                table + 0xc,
                n | if round % 2 == 1 { 0x8000_0000 } else { 0 },
            );
            t.w32(table + 0x10, data);
            let mut ks = keys.clone();
            ks.push(rng.next());
            for k in ks {
                t.check(
                    "findByKey",
                    0x0047_d2f0,
                    Conv::Thiscall(table),
                    &[k],
                    ALL,
                    |g| class::bridge::find_by_key(g, table, k),
                );
            }
            // offsets: members with starts, sizes and counts (0, 1, many)
            let cls = t.zeros(0x20);
            t.w32(cls + 0xc, n);
            t.w32(cls + 0x14, rng.below(64));
            let entries = t.zeros((n as usize).max(1) * 0x20);
            for i in 0..n {
                let m = t.zeros(0x20);
                t.w32(m + 8, (i * 0x10 + rng.below(8)).wrapping_sub(4));
                t.w32(m + 0xc, 1 + rng.below(12));
                // the first member of round 1 has count 0 (counted as 1)
                let count = if round == 1 && i == 0 {
                    0
                } else {
                    rng.below(4)
                };
                t.w32(m + 0x14, count);
                t.w32(entries + 0x20 * i + 4, m);
            }
            let view = t.zeros(0x20);
            t.w32(view + 4, cls);
            t.w32(view + 0x10, entries);
            for o in (0u32..0x60).chain([0x7fff_ffff, 0xffff_fff0]) {
                t.check(
                    "findByOffset",
                    0x0047_e4e0,
                    Conv::Thiscall(view),
                    &[o],
                    ALL,
                    |g| class::bridge::find_by_offset(g, view, o),
                );
            }
        }
    }
}

#[test]
fn registration_matches_the_original() {
    for mut t in Twins::all() {
        t.stub(class::REGISTER_TYPE, 1, 0, StubRet::Value(0x1234));
        t.stub(class::ADD_TO_NAMESPACE, 0, 0, StubRet::Value(0xadd0));
        t.stub(class::SET_DEFAULT_BUFFER, 1, 1, StubRet::Value(1));
        t.stub(class::SET_VALIDATION_BUFFER, 1, 1, StubRet::Value(2));
        t.stub(class::SET_WRITE_MASK_BUFFER, 1, 1, StubRet::Value(3));
        t.check("PBase_Bind", 0x0043_5610, Conv::Cdecl, &[], ALL, |g| {
            class::bridge::pbase_bind(g)
        });
        for (f, d) in class::REGISTERS {
            t.check("register", f, Conv::Cdecl, &[], ALL, |g| {
                class::bridge::register(g, d)
            });
        }
        assert_eq!(t.calls(), 4 + 2 * 11);
        // the shader pass init: every callee a stub returning its tag
        for (f, _) in class::SHADER_PASS_INIT {
            t.stub(f, 0, 0, StubRet::Value(f ^ 0x5a5a));
        }
        t.check(
            "shader pass init",
            0x0046_4400,
            Conv::Cdecl,
            &[],
            ALL,
            |g| class::bridge::shader_pass_init(g),
        );
        // the LOD init, with the flag clear and set
        for f in class::LOD_TYPES.into_iter().chain(class::LOD_BINDS) {
            t.stub(f, 0, 0, StubRet::Value(f ^ 0xa5));
        }
        let pre = t.stub_code(0x1d00, 0, 0, StubRet::Value(0x1d));
        let vt = t.zeros(8);
        t.w32(vt + 4, pre);
        let o = t.rebase(class::LOD_PRE_INIT);
        t.w32(o, vt);
        let flag = t.rebase(class::LOD_INITIALIZED);
        for v in [0u8, 1, 0x80] {
            t.write(flag, &[v]);
            t.check("lod init", 0x0050_4260, Conv::Cdecl, &[], ALL, |g| {
                class::bridge::lod_initialize(g)
            });
        }
        // the base descriptor
        t.stub(class::GLOBAL_NAMESPACE, 0, 0, StubRet::Value(0x0e5e));
        t.stub(class::CLASS_DESCRIPTOR_INIT, 4, 4, StubRet::Value(0));
        let d = t.random(&mut Rng(0x4040_4040), 0x40);
        t.check(
            "descriptor base",
            0x0057_0670,
            Conv::Thiscall(d),
            &[],
            ALL,
            |g| class::bridge::base_descriptor_init(g, d),
        );
        // the callback types: the stub returns the descriptor's first word
        t.stub(class::DESCRIPTOR_TYPE, 0, 0, StubRet::ThisWord);
        let ds = class::CALLBACK_DESCRIPTORS.map(|d| t.rebase(d));
        let saved: Vec<u32> = ds.iter().map(|&d| t.r32(d)).collect();
        let types: Vec<u32> = (0..3).map(|_| t.zeros(0x10)).collect();
        for (i, &ty) in types.iter().enumerate() {
            t.w32(ty + 8, 0x0100_0000 * (i as u32 + 1) + 0x42);
        }
        for mask in 0..8u32 {
            for (i, &d) in ds.iter().enumerate() {
                let v = if mask & (1 << i) != 0 { types[i] } else { 0 };
                t.w32(d, v);
            }
            for out in [0, t.random(&mut Rng(mask + 1), 0x18)] {
                t.check(
                    "callback types",
                    0x0055_0ed0,
                    Conv::Cdecl,
                    &[out],
                    ALL,
                    |g| class::bridge::callback_types(g, out),
                );
                t.check(
                    "callback thunk",
                    0x0055_11d0,
                    Conv::Stdcall,
                    &[out],
                    ALL,
                    |g| class::bridge::callback_types(g, out),
                );
            }
        }
        for (&d, &v) in ds.iter().zip(&saved) {
            t.w32(d, v);
        }
        t.unstub_all();
    }
}

#[test]
fn physics_callbacks_match_the_original() {
    use ffx_pother_a::physics::{self as ph, Create};
    let creates: [(u32, &Create, u32); 27] = [
        (0x005f_7a10, &ph::BOX, AL),
        (0x005f_7a50, &ph::BOX_BASE, AL),
        (0x005f_7a90, &ph::BOX_BULLET, AL),
        (0x005f_7ad0, &ph::CAPSULE, AL),
        (0x005f_7b20, &ph::CAPSULE_BASE, AL),
        (0x005f_7b70, &ph::CAPSULE_BULLET, AL),
        (0x005f_7cd0, &ph::CYLINDER, AL),
        (0x005f_7d20, &ph::CYLINDER_BASE, AL),
        (0x005f_7d60, &ph::CYLINDER_BULLET, AL),
        (0x005f_7e50, &ph::MESH, AL),
        (0x005f_7ea0, &ph::MESH_BASE, AL),
        (0x005f_7ee0, &ph::MESH_BULLET, AL),
        (0x005f_7f60, &ph::PLANE, AL),
        (0x005f_7fb0, &ph::PLANE_BASE, AL),
        (0x005f_7ff0, &ph::PLANE_BULLET, AL),
        (0x005f_8170, &ph::SHAPE, AL),
        (0x005f_81a0, &ph::SHAPE_BULLET, AL),
        (0x005f_81d0, &ph::SPHERE, AL),
        (0x005f_8210, &ph::SPHERE_BASE, AL),
        (0x005f_8250, &ph::SPHERE_BULLET, AL),
        (0x005f_8290, &ph::TAPERED_CAPSULE, AL),
        (0x005f_82e0, &ph::TAPERED_CYLINDER, AL),
        (0x005f_79f0, &ph::PAIR, ALL),
        (0x005f_7db0, &ph::BYTE_HEADER, ALL),
        (0x005f_7e30, &ph::VEC3_ZERO, ALL),
        (0x005f_7f30, &ph::TRIPLE, ALL),
        (0x005f_8390, &ph::TAIL_PAIR, ALL),
    ];
    let calls: [(u32, u32); 9] = [
        (0x005f_7bc0, 0x005d_ff30),
        (0x005f_7be0, 0x005e_01c0),
        (0x005f_7c30, 0x005e_0050),
        (0x005f_7c80, 0x005e_01c0),
        (0x005f_7de0, 0x005e_0330),
        (0x005f_80b0, 0x005e_0590),
        (0x005f_8330, 0x005e_0970),
        (0x005f_8350, 0x005e_0900),
        (0x005f_8370, 0x005e_0970),
    ];
    for mut t in Twins::all() {
        let mut rng = Rng(0x5f7a_1000);
        t.stub(ph::SHAPE_BASE, 0, 0, StubRet::Value(0x00ba_5e00));
        for (_, f) in calls {
            let at = t.rebase(f);
            if !t.saved.iter().any(|(a, _)| *a == at) {
                t.stub(f, 0, 0, StubRet::Value(f));
            }
        }
        let big = t.random(&mut rng, 0x400);
        let carry = ((big + 0xff) & !0xff) + 0xfc;
        for p in [0, t.random(&mut rng, 0x100), carry] {
            for &(sym, c, mask) in &creates {
                let n0 = t.calls();
                t.check("create", sym, Conv::Stdcall, &[p], mask, |g| {
                    ph::bridge::create(g, p, c)
                });
                if p != 0 && c.base.is_some() {
                    assert_eq!(t.calls(), n0 + 1, "{sym:#x}: one base call");
                }
                // fresh garbage for the next one
                if p != 0 {
                    let junk: Vec<u8> = (0..0x70).map(|_| rng.next() as u8).collect();
                    t.write(p, &junk);
                }
            }
            for (sym, f) in calls {
                t.check("destroy", sym, Conv::Stdcall, &[p], AL, |g| {
                    u32::from(object::bridge::destroy_call(g, f, p))
                });
            }
            for sym in [0x005f_8040, 0x005f_8100] {
                if p != 0 {
                    let junk: Vec<u8> = (0..0xe8).map(|_| rng.next() as u8).collect();
                    t.write(p, &junk);
                }
                t.check("body", sym, Conv::Stdcall, &[p], AL, |g| {
                    u32::from(ph::bridge::destroy_body(g, p))
                });
            }
        }
        t.unstub_all();
    }
}
