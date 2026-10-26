//! `PhyreEngine`'s serialization and cluster loading (`ffx_pload`, M4
//! port round 1, list `p1-load`) against the original: the class
//! descriptors' slots and helpers, the streams, the cluster
//! constructors. On the image alone with synthetic objects, and on a
//! snapshot (`FFX_SNAPSHOT`) over the snapshot's memory, where the real
//! class descriptors of the running game are checked too. Results and
//! every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! Callees outside the list run as they are where they are plain code,
//! or are replaced in both oracles alike by recording stubs: each call
//! appends its tag, `ecx` and its stack words to a log in memory, so the
//! diff checks that both made the same calls in the same order with the
//! same `this`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pload::cluster::{self, bridge as c};
use ffx_pload::descriptor::{self, bridge as d, core};
use ffx_pload::stream::{self, bridge as s};

/// Deterministic xorshift stream.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
}

/// The bridge's view of an oracle: its memory, and its emulator for the
/// original callees.
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

/// What the original returns, to compare.
#[derive(Debug, Clone, Copy)]
enum Out {
    /// Nothing the callers read (the original's `eax` is the caller's).
    None,
    /// `eax`.
    U32(u32),
    /// `al` (the rest of `eax` is the caller's or a callee's).
    U8(u8),
}

struct Twins {
    a: Oracle,
    b: Oracle,
    snapshot: bool,
    /// The call log: a count, then entries of 8 words (tag, `this`, six
    /// arguments).
    log: u32,
    /// The bytes each stub's `jmp` replaced, by address.
    saved: HashMap<u32, Vec<u8>>,
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle, snapshot: bool) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let mut t = Self {
            a,
            b,
            snapshot,
            log: 0,
            saved: HashMap::new(),
        };
        t.log = t.zeros(0x4000);
        t
    }

    /// The image, then the snapshot when `FFX_SNAPSHOT` is set.
    fn all() -> Vec<(&'static str, Self)> {
        let Some(dir) = game_dir() else {
            return vec![];
        };
        let exe = dir.join("FFX.exe");
        let mut out = vec![(
            "image",
            Self::new(
                Oracle::load(&exe).unwrap(),
                Oracle::load(&exe).unwrap(),
                false,
            ),
        )];
        if let Some(snap) = std::env::var_os("FFX_SNAPSHOT") {
            let pages = Pages::read(PathBuf::from(snap)).unwrap();
            out.push((
                "snapshot",
                Self::new(
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
                    true,
                ),
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

    /// `n` bytes of a known non-zero pattern.
    fn junk(&mut self, n: usize, seed: u8) -> u32 {
        let v: Vec<u8> = (0..n)
            .map(|i| (i as u8).wrapping_mul(37) ^ seed ^ 0x5a)
            .collect();
        self.alloc(&v)
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

    fn log_entries(&self) -> u32 {
        Mem::read(&self.b, self.log)
    }

    /// Clears the log and puts back every stubbed original.
    fn reset(&mut self) {
        let log = self.log;
        self.w32(log, 0);
        let saved: Vec<(u32, Vec<u8>)> = self.saved.drain().collect();
        for (at, bytes) in saved {
            self.write(at, &bytes);
        }
        self.diff("reset");
    }

    /// A recording stub in fresh code memory: logs `tag`, `ecx` when
    /// `this`, and `n` stack words; pops `pops` words; returns `ret`.
    fn stub_code(&mut self, tag: u32, this: bool, n: u32, pops: u32, ret: u32) -> u32 {
        assert!(n <= 6);
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
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xb8);
        c.extend(ret.to_le_bytes()); // mov eax, ret
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        let at = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), at);
        at
    }

    /// [`Twins::stub_code`] placed at the original `sym` (a `jmp` there),
    /// logged with `sym` as its tag.
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
        let at = self.rebase(sym);
        let code = self.stub_code(sym, this, n, pops, ret);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }

    /// An object whose vtable holds `slots` recording stubs (`this`
    /// logged, slot `i` tagged `0x7000 + i`, taking and popping `words[i]`
    /// words, returning `0x1000 + i`). Returns the object (8 words, after
    /// its vtable pointer, zero).
    fn object(&mut self, words: &[u32]) -> u32 {
        let mut vt = Vec::new();
        for (i, &n) in words.iter().enumerate() {
            let s = self.stub_code(0x7000 + i as u32, true, n, n, 0x1000 + i as u32);
            vt.extend(s.to_le_bytes());
        }
        let vt = self.alloc(&vt);
        let obj = self.zeros(0x40);
        self.w32(obj, vt);
        obj
    }

    fn diff(&mut self, what: &str) {
        let d = self.a.diff(&mut self.b).unwrap();
        assert!(
            d.is_empty(),
            "{what}: memory differs at {} bytes, first {:x?}",
            d.len(),
            &d[..d.len().min(8)]
        );
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// Runs the original at `sym` in `a` and `port` on `b`; compares the
    /// result as `port` says and every byte written.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        port: impl FnOnce(&mut G) -> Out,
    ) {
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let out = port(&mut G(&mut self.b));
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        match out {
            Out::None => {}
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::U8(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
        }
        self.diff(what);
    }

    /// The class descriptors of the image's data (objects whose first
    /// word is a class-descriptor vtable from `symbols/ffx.sym`), at most
    /// `max`.
    fn descriptors(&mut self, max: usize) -> Vec<u32> {
        let vtables: std::collections::HashSet<u32> = include_str!("../../../symbols/ffx.sym")
            .lines()
            .filter_map(|l| {
                let p: Vec<&str> = l.split_whitespace().collect();
                (p.len() >= 3
                    && p[1] == "label"
                    && p[2].contains("PClassDescriptor")
                    && p[2].ends_with("vftable"))
                .then(|| u32::from_str_radix(p[0].trim_start_matches("0x"), 16).ok())
                .flatten()
            })
            .map(|a| self.rebase(a))
            .collect();
        let lo = self.rebase(0x00c0_a000);
        let bytes = Oracle::read(&mut self.a, lo, 0xf0_000).unwrap();
        let mut found = Vec::new();
        for (i, w) in bytes.as_chunks::<4>().0.iter().enumerate() {
            let v = u32::from_le_bytes(*w);
            if vtables.contains(&v) {
                found.push(lo + 4 * i as u32);
            }
        }
        // live descriptors only: every parent is another one (or 0);
        // the pattern also matches torn-down and unrelated words
        let set: std::collections::HashSet<u32> = found.iter().copied().collect();
        let live = |t: &Self, d: u32| {
            let mut p = d;
            for _ in 0..32 {
                p = t.r32(p + descriptor::PARENT);
                if p == 0 {
                    return true;
                }
                if !set.contains(&p) {
                    return false;
                }
            }
            false
        };
        found
            .iter()
            .copied()
            .filter(|&d| live(self, d))
            .take(max)
            .collect()
    }
}

// ---- descriptor slots that return a constant ----

/// The constant slots and what each returns: `al` false
/// ([`core::CONSTRUCT_REFUSED`]), `al` true ([`core::DESTRUCT_DONE`]),
/// `eax` 0 ([`core::SLOT14`]).
const CONSTANT_SLOTS: [(u32, Out); 15] = [
    (0x0043_a540, Out::U8(core::CONSTRUCT_REFUSED)),
    (0x0043_ca40, Out::U8(core::CONSTRUCT_REFUSED)),
    (0x0045_ede0, Out::U8(core::CONSTRUCT_REFUSED)),
    (0x0044_47c0, Out::U8(core::DESTRUCT_DONE)),
    (0x0046_1dc0, Out::U8(core::DESTRUCT_DONE)),
    (0x0046_3150, Out::U8(core::DESTRUCT_DONE)),
    (0x0046_94d0, Out::U8(core::DESTRUCT_DONE)),
    (0x0046_94e0, Out::U8(core::DESTRUCT_DONE)),
    (0x0046_9ed0, Out::U8(core::DESTRUCT_DONE)),
    (0x00a2_d630, Out::U8(core::DESTRUCT_DONE)),
    (0x0043_cad0, Out::U32(core::SLOT14)),
    (0x0050_6bc0, Out::U32(core::SLOT14)),
    (0x005c_bae0, Out::U32(core::SLOT14)),
    (0x00a2_d650, Out::U32(core::SLOT14)),
    (0x00a2_d660, Out::U32(core::SLOT14)),
];

#[test]
fn constant_slots_match_the_original() {
    for (label, mut t) in Twins::all() {
        for (sym, out) in CONSTANT_SLOTS {
            for p in [0, 0x1234_5678, u32::MAX] {
                let what = format!("{label}: {sym:#x}({p:#x})");
                t.check(&what, sym, Conv::Stdcall, &[p], |_| out);
            }
        }
    }
}

// ---- construct / destroy in place ----

#[test]
fn construct_slots_match_the_original() {
    for (label, mut t) in Twins::all() {
        for sym in [0x0044_9400, 0x0045_95e0, 0x005c_ba80, 0x00a2_d500] {
            let p = t.junk(16, sym as u8);
            for at in [0, p, p + 0x81] {
                let what = format!("{label}: zero pair {sym:#x}({at:#x})");
                t.check(&what, sym, Conv::Stdcall, &[at], |g| {
                    Out::U32(d::construct_zero_pair(g, at))
                });
            }
        }
        // constructors that run original code on the instance
        for (sym, ctor) in [(0x0046_94b0, 0x0046_8670), (0x0050_6b30, 0x0050_4ec0)] {
            let p = t.junk(0x80, 3);
            for at in [0, p] {
                let what = format!("{label}: construct {sym:#x}({at:#x})");
                t.check(&what, sym, Conv::Stdcall, &[at], |g| {
                    Out::U8(d::construct_with(g, ctor, at))
                });
            }
        }
        // destructors, recorded
        for (sym, dtor, adjust) in [(0x0050_6b50, 0x0050_5010, 4), (0x00a2_d610, 0x00a2_bbd0, 0)] {
            t.stub(dtor, true, 0, 0, 0x55);
            for at in [0, 0x0123_4560] {
                let what = format!("{label}: destroy {sym:#x}({at:#x})");
                t.check(&what, sym, Conv::Stdcall, &[at], |g| {
                    Out::U8(d::destroy_with(g, dtor, adjust, at))
                });
            }
            assert_eq!(t.log_entries(), 2);
            t.reset();
        }
        // virtual destructors (p = 0 reads address 0 in both: not run)
        for (sym, slot, adjust) in [(0x0044_69a0, 0, 4), (0x0044_9580, 7, 0x18)] {
            let obj = t.object(&[1; 8]);
            let at = obj + adjust;
            let what = format!("{label}: destroy virtual {sym:#x}");
            t.check(&what, sym, Conv::Stdcall, &[at], |g| {
                Out::U8(d::destroy_virtual(g, slot, adjust, at))
            });
            let tag = t.r32(t.log + 4);
            assert_eq!(tag, 0x7000 + slot, "{what}: slot");
            t.reset();
        }
        // constructInstance of PClassDataMemberDynamic: 0x004465c0(p), recorded
        t.stub(0x0044_65c0, true, 1, 0, 0x77);
        let what = format!("{label}: construct member dynamic");
        t.check(&what, 0x0044_6980, Conv::Stdcall, &[0x0abc_def0], |g| {
            Out::U32(d::construct_member_dynamic(g, 0x0abc_def0))
        });
        assert_eq!(t.log_entries(), 1);
        t.reset();
    }
}

// ---- PType and PClassDescriptor ----

#[test]
fn setters_match_the_original() {
    let mut rng = Rng(0x1234_5678);
    for (label, mut t) in Twins::all() {
        let obj = t.junk(0x100, 9);
        for _ in 0..8 {
            let (a, b) = (rng.next(), rng.next());
            for (sym, off, n) in [
                (0x0043_79b0, 0x24, 2),
                (0x0043_d590, 0x74, 2),
                (0x0043_d510, 0x64, 1),
                (0x0043_d5b0, 0x68, 1),
                (0x0043_d5c0, 0x6c, 1),
            ] {
                let args = &[a, b][..n];
                let what = format!("{label}: setter {sym:#x}");
                t.check(&what, sym, Conv::Thiscall(obj), args, |g| {
                    Out::U32(d::set_fields(g, obj, off, args))
                });
            }
        }
    }
}

#[test]
fn fixup_and_resize_match_the_original() {
    for (label, mut t) in Twins::all() {
        let ty = t.junk(0x40, 1);
        // no fixup function
        t.w32(ty + 0x28, 0);
        let args = [1, 2, 3, 4];
        let what = format!("{label}: resolveFixup, none");
        t.check(&what, 0x0043_7980, Conv::Thiscall(ty), &args, |g| {
            Out::U32(d::resolve_fixup(g, ty, args))
        });
        // cdecl: the fixup functions (0x0045d9b0, 0x0043be30) do not read ecx
        let f = t.stub_code(0xf1, false, 4, 0, 0x99);
        t.w32(ty + 0x28, f);
        let what = format!("{label}: resolveFixup");
        t.check(&what, 0x0043_7980, Conv::Thiscall(ty), &args, |g| {
            Out::U32(d::resolve_fixup(g, ty, args))
        });
        assert_eq!(t.log_entries(), 1);

        for sym in [0x0044_a320, 0x00a2_dbc0] {
            let member = t.junk(0x40, 2);
            t.w32(member + 0x3c, 0);
            let what = format!("{label}: resize {sym:#x}, none");
            t.check(&what, sym, Conv::Thiscall(member), &[0xa1, 5, 6], |g| {
                Out::U32(d::resize_array(g, member, 0xa1, 5, 6))
            });
            let f = t.stub_code(0xf2, true, 2, 2, 0x98);
            t.w32(member + 0x3c, f);
            let what = format!("{label}: resize {sym:#x}");
            t.check(&what, sym, Conv::Thiscall(member), &[0xa1, 5, 6], |g| {
                Out::U32(d::resize_array(g, member, 0xa1, 5, 6))
            });
        }
        assert_eq!(t.log_entries(), 3);
        t.reset();
    }
}

#[test]
fn namespace_flags_match_the_original() {
    let mut rng = Rng(0x0bad_cafe);
    for (label, mut t) in Twins::all() {
        let desc = t.junk(0xa0, 4);
        // isInNamespace on random flags
        for _ in 0..16 {
            let flags = rng.next();
            t.w32(desc + descriptor::FLAGS, flags);
            let what = format!("{label}: isInNamespace {flags:#x}");
            t.check(&what, 0x0043_cfb0, Conv::Thiscall(desc), &[], |g| {
                Out::U32(d::in_namespace(g, desc))
            });
        }
        // addToNamespace: PNamespace::add recorded
        t.stub(descriptor::NAMESPACE_ADD, true, 1, 1, 0x4242);
        for flags in [0, 0x4000_0000, 0x8000_0001] {
            t.w32(desc + descriptor::FLAGS, flags);
            t.w32(desc + descriptor::NAMESPACE, 0x00ab_cd00 + flags);
            let what = format!("{label}: addToNamespace {flags:#x}");
            t.check(&what, 0x0043_c140, Conv::Thiscall(desc), &[], |g| {
                Out::U32(d::add_to_namespace(g, desc))
            });
        }
        assert_eq!(t.log_entries(), 3);
        t.reset();
        // getDefaultBuffer: the buffers' calculations recorded, slot 12 of
        // a fake vtable
        t.stub(descriptor::CALCULATE_DEFAULT, true, 0, 0, 1);
        t.stub(descriptor::CALCULATE_VALIDATION, true, 0, 0, 2);
        let obj = t.object(&[0; 13]);
        t.w32(obj + descriptor::DEFAULT_BUFFER, 0x0d0d_0d0d);
        for flags in [0, 0x4000_0000, 0x8000_0000, 0xc000_0000, 0x3fff_ffff] {
            t.w32(obj + descriptor::FLAGS, flags);
            let what = format!("{label}: getDefaultBuffer {flags:#x}");
            t.check(&what, 0x0043_cb80, Conv::Thiscall(obj), &[], |g| {
                Out::U32(d::default_buffer(g, obj))
            });
        }
        // calls: 0 and 3f.. -> 12, def, val; 4.. -> def, val; 8.., c.. -> none
        assert_eq!(t.log_entries(), 3 + 2 + 3);
        t.reset();
        // registering PClusterHeader: RegisterType and add, recorded
        t.stub(descriptor::REGISTER_TYPE, false, 1, 0, 3);
        t.stub(descriptor::NAMESPACE_ADD, true, 1, 1, 0x4343);
        let what = format!("{label}: register cluster header");
        t.check(&what, 0x0046_8870, Conv::Cdecl, &[], |g| {
            Out::U32(d::register_cluster_header(g))
        });
        assert_eq!(t.log_entries(), 2);
        t.reset();
    }
}

#[test]
fn inheritance_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // a chain c -> b -> a -> 0, and an unrelated x
        let a = t.zeros(0xa0);
        let b = t.zeros(0xa0);
        let c = t.zeros(0xa0);
        let x = t.zeros(0xa0);
        t.w32(b + descriptor::PARENT, a);
        t.w32(c + descriptor::PARENT, b);
        for (this, other) in [
            (c, a),
            (c, b),
            (c, c),
            (a, c),
            (c, x),
            (c, 0),
            (b, 0x1234_56ff),
        ] {
            let what = format!("{label}: inheritsFrom {this:#x} {other:#x}");
            t.check(&what, 0x0043_1420, Conv::Thiscall(this), &[other], |g| {
                Out::U32(d::inherits_from(g, this, other))
            });
        }
        // a null descriptor: al only (the original keeps the caller's eax)
        let what = format!("{label}: inheritsFrom null");
        t.check(&what, 0x0043_1420, Conv::Thiscall(0), &[a], |g| {
            Out::U8(d::inherits_from(g, 0, a) as u8)
        });
        // the game's own descriptors
        if t.snapshot {
            let real = t.descriptors(64);
            assert!(real.len() >= 16, "{label}: {} descriptors", real.len());
            for (i, &this) in real.iter().enumerate() {
                // a few others, its parent and its grandparent (found)
                let parent = t.r32(this + descriptor::PARENT);
                let grand = if parent == 0 {
                    0
                } else {
                    t.r32(parent + descriptor::PARENT)
                };
                let others: Vec<u32> = real[..8].iter().copied().chain([parent, grand]).collect();
                for other in others {
                    let what = format!("{label}: real inheritsFrom {i}");
                    t.check(&what, 0x0043_1420, Conv::Thiscall(this), &[other], |g| {
                        Out::U32(d::inherits_from(g, this, other))
                    });
                }
                let what = format!("{label}: real isInNamespace {i}");
                t.check(&what, 0x0043_cfb0, Conv::Thiscall(this), &[], |g| {
                    Out::U32(d::in_namespace(g, this))
                });
                let what = format!("{label}: real getDefaultBuffer {i}");
                t.check(&what, 0x0043_cb80, Conv::Thiscall(this), &[], |g| {
                    Out::U32(d::default_buffer(g, this))
                });
            }
        }
    }
}

#[test]
fn dynamic_descriptors_match_the_original() {
    for (label, mut t) in Twins::all() {
        let desc = t.zeros(0xa0);
        // no base: the original leaves the caller's eax
        let what = format!("{label}: bindBase, no base");
        t.check(&what, 0x0044_9380, Conv::Thiscall(desc), &[], |g| {
            d::bind_base(g, desc);
            Out::None
        });
        let base = t.object(&[0; 13]);
        t.w32(desc + descriptor::PARENT, base);
        for flags in [0x4000_0000, 0, 0xbfff_ffff, 0x7fff_ffff] {
            t.w32(base + descriptor::FLAGS, flags);
            let what = format!("{label}: bindBase {flags:#x}");
            t.check(&what, 0x0044_9380, Conv::Thiscall(desc), &[], |g| {
                Out::U32(d::bind_base(g, desc))
            });
        }
        assert_eq!(t.log_entries(), 2);
        t.reset();
        // the scalar deleting destructor: dtor and free recorded
        t.stub(0x0044_7880, true, 0, 0, 0x11);
        t.stub(descriptor::FREE, false, 1, 0, 0x22);
        for flags in [0, 1, 2, 3, 0xffff_fffe] {
            let what = format!("{label}: delete dynamic {flags:#x}");
            t.check(&what, 0x0044_7cb0, Conv::Thiscall(desc), &[flags], |g| {
                Out::U32(d::delete_dynamic(g, desc, flags))
            });
        }
        assert_eq!(t.log_entries(), 5 + 2);
        t.reset();
        // getDynamicType: the type, or the resolver's slot 1
        let member = t.junk(0x40, 7);
        t.w32(member + 0x28, 0);
        let what = format!("{label}: getDynamicType, none");
        t.check(&what, 0x0057_5690, Conv::Thiscall(member), &[0x5151], |g| {
            Out::U32(d::dynamic_type(g, member, 0x5151))
        });
        let resolver = t.object(&[1, 1]);
        t.w32(member + 0x28, resolver);
        let what = format!("{label}: getDynamicType");
        t.check(&what, 0x0057_5690, Conv::Thiscall(member), &[0x5151], |g| {
            Out::U32(d::dynamic_type(g, member, 0x5151))
        });
        assert_eq!(t.r32(t.log + 4), 0x7001);
        t.reset();
        // IsDefaultInstance
        let marker = t.r32(t.rebase(descriptor::DEFAULT_INSTANCE.sym));
        for v in [marker, marker ^ 1, 0] {
            t.w32(member + 0x10, v);
            let what = format!("{label}: IsDefaultInstance {v:#x}");
            t.check(&what, 0x0044_6870, Conv::Cdecl, &[member], |g| {
                Out::U32(d::is_default_instance(g, member))
            });
        }
        // the block header
        for (p, x) in [(0, 0), (0, 4), (0x100, 0x100), (0x100, 0x104), (2, 0)] {
            t.w32(member, p);
            let what = format!("{label}: blockHeader {p:#x} {x:#x}");
            t.check(&what, 0x0057_5d70, Conv::Thiscall(member), &[x], |g| {
                Out::U32(d::block_header(g, member, x))
            });
        }
    }
}

#[test]
fn key_data_type_parse_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let out = t.junk(8, 1);
        for found in [0, 7, 0xffff_ffff] {
            t.stub(0x0050_7a30, false, 1, 0, found);
            let what = format!("{label}: parse {found:#x}");
            t.check(&what, 0x0050_7b50, Conv::Cdecl, &[out, 0x0dea_d000], |g| {
                Out::U32(d::parse_key_data_type(g, out, 0x0dea_d000))
            });
            t.reset();
        }
    }
}

// ---- PClassDataMember::listsClass (twelve copies) ----

const LISTS_CLASS: [u32; 12] = [
    0x0046_5830,
    0x0046_5860,
    0x0046_5890,
    0x0046_58c0,
    0x0046_58f0,
    0x0046_5920,
    0x0046_5950,
    0x0046_5980,
    0x0046_59b0,
    0x0046_59e0,
    0x0046_5a10,
    0x0046_5a40,
];

#[test]
fn lists_class_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let member = t.junk(0x40, 5);
        let class = t.zeros(0x60);
        // a chain of three entries linked through +0x40
        let e: Vec<u32> = (0..3).map(|_| t.zeros(0x50)).collect();
        t.w32(e[0] + 0x40, e[1]);
        t.w32(e[1] + 0x40, e[2]);
        let cases: [(u32, u32, u32, u32); 8] = [
            // (class, head, wanted, count)
            (0, 0, 0, 0),
            (class, 0, e[0], 1),
            (class, e[0], e[0], 1),
            (class, e[0], e[2], 7),
            (class, e[0], e[2], 0),
            (class, e[0], 0x1234, 1),
            (class, e[1], e[0], 1),
            (class, e[0], e[1], 0x8000_0000),
        ];
        for sym in LISTS_CLASS {
            for (i, &(c, head, want, count)) in cases.iter().enumerate() {
                t.w32(member + 0x18, c);
                t.w32(member + 0x1c, want);
                t.w32(class + 0x2c, head);
                t.w32(class + 0x40, count);
                let what = format!("{label}: listsClass {sym:#x} case {i}");
                t.check(&what, sym, Conv::Thiscall(member), &[], |g| {
                    Out::U8(d::member_lists_class(g, member))
                });
            }
        }
    }
}

// ---- streams and the start-up ----

#[test]
fn stream_wrappers_match_the_original() {
    for (label, mut t) in Twins::all() {
        // needsEndianSwap
        let ns = t.junk(8, 1);
        for v in [0, 1, 0x8000_0000] {
            t.w32(ns, v);
            let what = format!("{label}: needsEndianSwap {v:#x}");
            t.check(&what, 0x0042_b830, Conv::Thiscall(ns), &[], |g| {
                Out::U32(s::needs_endian_swap(g, ns))
            });
        }
        // the reader file's forwarders, recorded
        let reader = t.junk(0x40, 2);
        t.stub(stream::CLOSE_FILE, true, 0, 0, 0x0101_0101);
        t.stub(stream::IS_OPEN, true, 0, 0, 0x0202_0201);
        t.stub(stream::READ, true, 2, 2, 0x333);
        let what = format!("{label}: reader close");
        t.check(&what, 0x0057_6930, Conv::Thiscall(reader), &[], |g| {
            Out::U32(s::reader_close(g, reader))
        });
        let what = format!("{label}: reader isOpen");
        t.check(&what, 0x0057_6980, Conv::Thiscall(reader), &[], |g| {
            Out::U32(s::reader_is_open(g, reader))
        });
        let what = format!("{label}: reader read");
        t.check(
            &what,
            0x0057_6950,
            Conv::Thiscall(reader),
            &[0xb0f, 0x40],
            |g| Out::U32(s::reader_read(g, reader, 0xb0f, 0x40)),
        );
        assert_eq!(t.log_entries(), 3);
        t.reset();
        // the writer's close, recorded
        t.stub(0x0043_1850, true, 1, 1, 0x444);
        let what = format!("{label}: writer close");
        t.check(&what, 0x0043_1840, Conv::Thiscall(reader), &[], |g| {
            Out::U32(s::mutex_lock(g, reader))
        });
        assert_eq!(t.log_entries(), 1);
        t.reset();
        // the destructors: free recorded
        t.stub(stream::FREE, false, 1, 0, 0x55);
        for (sym, name) in [
            (0x0060_f0c0, 0x00ab_0000),
            (0x0060_f0c0, 0x00ab_0001),
            (0x0057_dc70, 0x00ab_0010),
            (0x0057_dc70, 0x00ab_0011),
        ] {
            let obj = t.junk(0x20, 3);
            t.w32(obj + 8, name);
            let what = format!("{label}: dtor {sym:#x} {name:#x}");
            t.check(&what, sym, Conv::Thiscall(obj), &[], |g| {
                if sym == 0x0060_f0c0 {
                    s::stream_dtor(g, obj);
                } else {
                    s::reader_dtor(g, obj);
                }
                Out::None
            });
        }
        assert_eq!(t.log_entries(), 2);
        t.reset();
        // the reader's constructor with PStream's own (no name: no
        // allocation), which writes PStream's vtable first
        let obj = t.junk(0x20, 5);
        let what = format!("{label}: reader ctor, real PStream");
        t.check(&what, 0x0057_dc40, Conv::Thiscall(obj), &[0], |g| {
            Out::U32(s::reader_ctor(g, obj, 0))
        });
        // the reader's constructor, PStream's recorded
        t.stub(stream::STREAM_CTOR, true, 1, 1, 0x66);
        let obj = t.junk(0x20, 4);
        let what = format!("{label}: reader ctor");
        t.check(
            &what,
            0x0057_dc40,
            Conv::Thiscall(obj),
            &[0x00ab_cdef],
            |g| Out::U32(s::reader_ctor(g, obj, 0x00ab_cdef)),
        );
        assert_eq!(t.log_entries(), 1);
        t.reset();
        // counted reads through slot 5 (returns 0x1005)
        let r = t.object(&[0, 0, 0, 0, 0, 2]);
        t.w32(r + 0xc, 0xffff_fff0);
        for size in [0x1005, 7] {
            let what = format!("{label}: reader read counted {size:#x}");
            t.check(&what, 0x0057_dd10, Conv::Thiscall(r), &[0xb0, size], |g| {
                Out::U32(s::reader_read_counted(g, r, 0xb0, size))
            });
            let what = format!("{label}: reader readExact {size:#x}");
            t.check(&what, 0x0057_dd30, Conv::Thiscall(r), &[0xb0, size], |g| {
                Out::U32(s::reader_read_exact(g, r, 0xb0, size))
            });
        }
        assert_eq!(t.log_entries(), 4);
        t.reset();
    }
}

#[test]
fn start_up_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // SetStreamPrefix and the flag getter
        for v in [0x00ab_c000, 0] {
            let what = format!("{label}: SetStreamPrefix {v:#x}");
            t.check(&what, 0x0060_7d40, Conv::Cdecl, &[v], |g| {
                Out::U32(s::set_stream_prefix(g, v))
            });
        }
        let flag = t.rebase(stream::SERIALIZATION_READY.sym);
        for v in [0u8, 1, 0x80] {
            t.write(flag, &[v]);
            let what = format!("{label}: IsInitialized {v}");
            t.check(&what, 0x0043_2020, Conv::Cdecl, &[], |g| {
                Out::U8(s::serialization_ready(g))
            });
        }
        let what = format!("{label}: GetPlatformId");
        t.check(&what, 0x005f_0490, Conv::Cdecl, &[], |_| {
            Out::U32(stream::PLATFORM_ID)
        });
        // PUtilitySerialization::initialize: the Binds and the
        // registration recorded
        t.write(flag, &[0]);
        t.stub(0x0046_8890, false, 0, 0, 1);
        t.stub(0x0046_9a70, false, 0, 0, 2);
        t.stub(0x0046_a340, false, 0, 0, 3);
        t.stub(descriptor::REGISTER_TYPE, false, 1, 0, 4);
        t.stub(descriptor::NAMESPACE_ADD, true, 1, 1, 5);
        let what = format!("{label}: serialization initialize");
        t.check(&what, 0x0043_20e0, Conv::Cdecl, &[], |g| {
            Out::U32(s::serialization_init(g))
        });
        assert_eq!(t.log_entries(), 5);
        t.reset();
        // the asset tree, set up or not
        let tree = t.rebase(stream::ASSET_TREE);
        t.stub(0x0043_17f0, true, 0, 0, 0x77);
        for v in [1u8, 0] {
            t.write(tree, &[v]);
            let what = format!("{label}: InitializeAssetTree {v}");
            t.check(&what, 0x0045_d600, Conv::Cdecl, &[], |g| {
                Out::U32(s::init_asset_tree(g))
            });
        }
        assert_eq!(t.log_entries(), 1);
        t.reset();
        // PUtilityGame::initialize
        t.stub(0x00a3_3540, false, 0, 0, 0x88);
        let what = format!("{label}: game initialize");
        t.check(&what, 0x009f_4c80, Conv::Cdecl, &[], |g| {
            Out::U32(s::game_init(g))
        });
        assert_eq!(t.log_entries(), 1);
        t.reset();
    }
}

// ---- clusters ----

#[test]
fn cluster_constructors_match_the_original() {
    let mut rng = Rng(0x00c0_ffee);
    for (label, mut t) in Twins::all() {
        let obj = t.junk(0x80, 6);
        let what = format!("{label}: PNamespace");
        t.check(&what, 0x0043_dd80, Conv::Thiscall(obj), &[], |g| {
            Out::U32(c::namespace_ctor(g, obj))
        });
        // the PClusterHeader constructor runs the base's as it is
        let obj = t.junk(0x80, 7);
        let what = format!("{label}: PClusterHeader");
        t.check(&what, 0x0046_9980, Conv::Thiscall(obj), &[], |g| {
            Out::U32(c::header_ctor(g, obj))
        });
        // the cluster constructors: PClusterBase's recorded
        t.stub(cluster::CLUSTER_BASE_CTOR, true, 0, 0, 0x99);
        let obj = t.junk(0x80, 8);
        let what = format!("{label}: PClusterBase init");
        t.check(&what, 0x0057_6630, Conv::Thiscall(obj), &[], |g| {
            Out::U32(c::cluster_base_init(g, obj))
        });
        let what = format!("{label}: PCluster");
        t.check(&what, 0x0043_f800, Conv::Thiscall(obj), &[], |g| {
            Out::U32(c::cluster_ctor(g, obj))
        });
        assert_eq!(t.log_entries(), 2);
        t.reset();
        // forwarders, recorded
        t.stub(0x0047_86a0, true, 4, 4, 0xaa);
        t.stub(0x0046_5ab0, true, 3, 3, 0xbb);
        let what = format!("{label}: mapping init");
        t.check(&what, 0x0047_87e0, Conv::Thiscall(obj), &[1, 2, 3], |g| {
            Out::U32(c::mapping_init(g, obj, 1, 2, 3))
        });
        let what = format!("{label}: loadClusterTextures thunk");
        t.check(&what, 0x0046_5aa0, Conv::Thiscall(obj), &[4, 5, 6], |g| {
            Out::U32(c::load_cluster_textures(g, obj, 4, 5, 6))
        });
        assert_eq!(t.log_entries(), 2);
        t.reset();
        // the two small getters
        for i in 0..12 {
            let v = match i {
                0 => 0,
                1 => 0x8000_0000,
                2 => 1,
                _ => rng.next(),
            };
            t.w32(obj + 0x20, v);
            t.w32(obj + 0x64, v);
            let what = format!("{label}: countAtLeastOne {v:#x}");
            t.check(&what, 0x0059_3bd0, Conv::Thiscall(obj), &[], |g| {
                Out::U32(c::count_at_least_one(g, obj))
            });
            for sym in [0x0059_5fa0, 0x0059_5fb0] {
                let what = format!("{label}: hasWord64 {sym:#x} {v:#x}");
                t.check(&what, sym, Conv::Thiscall(obj), &[], |g| {
                    Out::U32(c::has_word_64(g, obj))
                });
            }
        }
    }
}

// ---- batch 3: cluster arenas, lists, trees, arrays ----

/// A circular intrusive list at `head` (two words) of `keys.len()`
/// entries of 0x20 bytes, linked through `entry + 4`, each key at
/// `entry + 0xc`. Returns the entries.
fn list(t: &mut Twins, head: u32, keys: &[u32]) -> Vec<u32> {
    let entries: Vec<u32> = keys.iter().map(|_| t.zeros(0x20)).collect();
    let links: Vec<u32> = entries.iter().map(|e| e + 4).collect();
    let n = links.len();
    if n == 0 {
        t.w32(head, head);
        t.w32(head + 4, head);
        return entries;
    }
    t.w32(head, links[0]);
    t.w32(head + 4, links[n - 1]);
    for i in 0..n {
        let next = if i + 1 < n { links[i + 1] } else { head };
        let prev = if i == 0 { head } else { links[i - 1] };
        t.w32(links[i], next);
        t.w32(links[i] + 4, prev);
        t.w32(entries[i] + 0xc, keys[i]);
    }
    entries
}

#[test]
fn cluster_helpers_match_the_original() {
    let mut rng = Rng(0x5eed_1234);
    for (label, mut t) in Twins::all() {
        // allocateInstanceList: allocateInstanceListForSize recorded
        t.stub(0x0044_0ba0, true, 4, 4, 0x123);
        let desc = t.junk(0x40, 1);
        let cl = t.junk(0x40, 2);
        for n in [0, 3] {
            let what = format!("{label}: allocateInstanceList {n}");
            t.check(&what, 0x0044_0b70, Conv::Thiscall(cl), &[n, desc, 9], |g| {
                Out::U32(c::allocate_instance_list(g, cl, n, desc, 9))
            });
        }
        assert_eq!(t.log_entries(), 1);
        t.reset();
        // the arena
        t.w32(cl + 0x24, 0);
        let what = format!("{label}: arenaAlloc, no arena");
        t.check(&what, 0x0044_0cf0, Conv::Thiscall(cl), &[4, 4], |g| {
            Out::U32(c::arena_alloc(g, cl, 4, 4))
        });
        let arena = t.zeros(0x10);
        t.w32(cl + 0x24, arena);
        t.w32(arena + 4, 0x0010_0000);
        t.w32(arena + 8, 0x100);
        for i in 0..42 {
            // the first two end exactly at the arena's end and one past it
            let (used, size, align) = match i {
                0 => (0, 0x100, 1),
                1 => (0, 0x101, 1),
                _ => (rng.next() % 0x110, rng.next() % 0x40, 1 << (rng.next() % 6)),
            };
            t.w32(arena + 0xc, used);
            let what = format!("{label}: arenaAlloc {used:#x} {size:#x} {align}");
            t.check(
                &what,
                0x0044_0cf0,
                Conv::Thiscall(cl),
                &[size, align],
                |g| Out::U32(c::arena_alloc(g, cl, size, align)),
            );
        }
        // lists: find by key, then clear
        for keys in [&[][..], &[7][..], &[1, 2, 3][..]] {
            let head = t.zeros(8);
            list(&mut t, head, keys);
            for key in [1, 3, 7, 9] {
                let what = format!("{label}: findByKey {keys:?} {key}");
                t.check(&what, 0x0057_5d40, Conv::Thiscall(head), &[key], |g| {
                    Out::U32(c::find_by_key(g, head, key))
                });
            }
            let what = format!("{label}: clear {keys:?}");
            t.check(&what, 0x0057_58e0, Conv::Thiscall(head), &[], |g| {
                Out::U32(c::clear_list(g, head))
            });
        }
        // the header check
        let header = t.zeros(0x20);
        let w = [0x0102_0304u32, 0, 3, 2, 5, 0x10, 0, 0x40];
        let size = 0x40 + 0x20 + 4 * (3 + 9 * 2 + 6 * 5) + 0x10;
        for (magic, sz) in [(w[0], size), (w[0], size + 1), (0x0403_0201, size)] {
            let mut v = w;
            v[0] = magic;
            v[1] = sz;
            for (i, x) in v.iter().enumerate() {
                t.w32(header + 4 * i as u32, *x);
            }
            let what = format!("{label}: header isValid {magic:#x} {sz:#x}");
            t.check(&what, 0x0047_f9d0, Conv::Thiscall(header), &[], |g| {
                Out::U8(c::header_is_valid(g, header))
            });
        }
        // copyHeaderWords
        let src = t.zeros(0x60);
        for words in [
            [0, 0, 0],
            [1, 0, 0],
            [0, 2, 0],
            [0, 0, 3],
            [4, 5, 6],
            [7, 8, 0],
        ] {
            for (i, x) in words.iter().enumerate() {
                t.w32(src + 0x48 + 4 * i as u32, *x);
            }
            let dst = t.junk(0x20, 3);
            let what = format!("{label}: copyHeaderWords {words:?}");
            t.check(
                &what,
                0x0046_47a0,
                Conv::Thiscall(dst),
                &[1, src, 2, 3],
                |g| Out::U32(c::copy_header_words(g, dst, src)),
            );
        }
        // the red-black tree: root 10, left 5, right 20, 20's right 30
        let tree = t.zeros(0x20);
        let node = |t: &mut Twins, key: u32| {
            let n = t.zeros(0x20);
            t.w32(n, tree);
            t.w32(n + 4, tree);
            t.w32(n + 0xc, key);
            n
        };
        let (r, l, rr, rrr) = (
            node(&mut t, 10),
            node(&mut t, 5),
            node(&mut t, 20),
            node(&mut t, 30),
        );
        t.w32(r, l);
        t.w32(r + 4, rr);
        t.w32(rr + 4, rrr);
        let key = t.zeros(4);
        for root in [tree, r] {
            t.w32(tree + 0xc, root);
            for k in [5, 10, 20, 30, 1, 15, 99] {
                t.w32(key, k);
                for sym in [0x0057_6ef0, 0x0058_1620] {
                    let what = format!("{label}: tree find {sym:#x} {k}");
                    t.check(&what, sym, Conv::Thiscall(tree), &[key], |g| {
                        Out::U32(c::tree_find(g, tree, key))
                    });
                }
            }
        }
    }
}

#[test]
fn arrays_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(descriptor::FREE, false, 1, 0, 0x5a);
        // setData
        for sym in [0x0044_a2d0, 0x00a2_db70] {
            for (word, old, data, count) in [
                (5, 0x00aa_0000, 0x00bb_0000, 3i32),
                (0x8000_0005, 0x00aa_0000, 0x00bb_0000, 3),
                (5, 0, 0x00bb_0000, 3),
                (5, 0x00bb_0000, 0x00bb_0000, 0),
                (5, 0x00aa_0000, 0x00bb_0000, -1),
                (0x7fff_ffff, 0x00aa_0000, 0, 0x7fff_ffff),
            ] {
                let arr = t.zeros(8);
                t.w32(arr, word);
                t.w32(arr + 4, old);
                let what = format!("{label}: setData {sym:#x} {word:#x} {old:#x} {count}");
                t.check(
                    &what,
                    sym,
                    Conv::Thiscall(arr),
                    &[data, count.cast_unsigned()],
                    |g| Out::U32(c::array_set_data(g, arr, data, count)),
                );
            }
        }
        assert_eq!(t.log_entries(), 2 * 2);
        t.reset();
        // destructInstance
        t.stub(descriptor::FREE, false, 1, 0, 0x5a);
        for sym in [0x0044_9540, 0x0045_9670, 0x005c_baa0, 0x00a2_d5d0] {
            for (word, data) in [(5, 0x00aa_0000), (0x8000_0005, 0x00aa_0000), (5, 0)] {
                let arr = t.zeros(8);
                t.w32(arr, word);
                t.w32(arr + 4, data);
                let what = format!("{label}: destroy {sym:#x} {word:#x} {data:#x}");
                t.check(&what, sym, Conv::Stdcall, &[arr], |g| {
                    Out::U8(c::array_destroy(g, arr))
                });
            }
        }
        assert_eq!(t.log_entries(), 4);
        t.reset();
        // the char info constructor
        let p = t.junk(0x40, 9);
        for at in [0, p, p + 0x33] {
            let what = format!("{label}: char info ctor {at:#x}");
            t.check(&what, 0x00a2_d520, Conv::Stdcall, &[at], |g| {
                Out::U32(c::char_info_ctor(g, at))
            });
        }
        // PDepthOfField: its base recorded
        t.stub(0x009f_bd90, true, 0, 0, 0x5b);
        let dof = t.junk(0x1000, 4);
        let what = format!("{label}: PDepthOfField");
        t.check(&what, 0x009f_bd40, Conv::Thiscall(dof), &[], |g| {
            Out::U32(c::depth_of_field_ctor(g, dof))
        });
        assert_eq!(t.log_entries(), 1);
        t.reset();
        // ensureCreated: 0x00590800 recorded
        t.stub(0x0059_0800, true, 4, 4, 0x5c);
        let words = t.zeros(8);
        for (count, first, inline) in [
            (1, 0, true),
            (1, 9, true),
            (0x8000_0001, 0, true),
            (2, 0, false),
            (3, 0, false),
            (3, 9, false),
        ] {
            let obj = t.junk(0x70, 5);
            t.w32(obj + 0x20, count);
            if inline {
                t.w32(obj + 0x24, first);
            } else {
                t.w32(obj + 0x24, words);
                t.w32(words, first);
            }
            let what = format!("{label}: ensureCreated {count:#x} {first}");
            t.check(&what, 0x0058_dba0, Conv::Thiscall(obj), &[], |g| {
                Out::U32(c::ensure_created(g, obj))
            });
        }
        assert_eq!(t.log_entries(), 4);
        t.reset();
    }
}

#[test]
fn descriptor_helpers_match_the_original() {
    for (label, mut t) in Twins::all() {
        // findByName: the dispenser and its lookup recorded
        let out = t.junk(4, 1);
        for found in [0, 0x00de_5c00] {
            t.stub(0x0043_9040, false, 0, 0, 0x0d15);
            t.stub(0x0043_7570, true, 1, 1, found);
            for name in [0, 0x00ab_c000] {
                let what = format!("{label}: findByName {name:#x} {found:#x}");
                t.check(&what, 0x0043_be30, Conv::Cdecl, &[out, name], |g| {
                    Out::U32(d::find_class_descriptor(g, out, name))
                });
            }
            assert_eq!(t.log_entries(), 2);
            t.reset();
        }
        // enum entries
        for dup in [0, 1] {
            t.stub(0x0050_7a30, false, 1, 0, dup);
            for name in [0, 0x00ab_c000] {
                let entry = t.junk(8, 2);
                t.w32(entry, name);
                let what = format!("{label}: registerEntry {name:#x} {dup}");
                t.check(&what, 0x0050_7ad0, Conv::Cdecl, &[entry], |g| {
                    Out::U32(d::register_enum_entry(g, entry))
                });
            }
            t.reset();
        }
        // initEntries
        for n in [0, 1, 5] {
            let at = t.junk(0x80, 3);
            let what = format!("{label}: initEntries {n}");
            t.check(&what, 0x0043_bcf0, Conv::Cdecl, &[at, n], |g| {
                d::init_entries(g, at, n);
                Out::None
            });
        }
        // releaseOwned: descriptors with slot 7 recorded
        let ns = t.zeros(0x20);
        let flags = [1u32, 0, 3, 0x8000_0000, 1];
        let descs: Vec<u32> = flags
            .iter()
            .map(|&f| {
                let o = t.object(&[0, 0, 0, 0, 0, 0, 0, 1]);
                let d = t.zeros(0xa0);
                let vt = t.r32(o);
                t.w32(d, vt);
                t.w32(d + descriptor::FLAGS, f);
                d
            })
            .collect();
        let head = ns + 8;
        for (i, &d) in descs.iter().enumerate() {
            let next = descs.get(i + 1).map_or(head, |n| n + 0x2c);
            t.w32(d + 0x2c, next);
        }
        for first in [head, descs[0] + 0x2c] {
            t.w32(head, first);
            let what = format!("{label}: releaseOwned {first:#x}");
            t.check(&what, 0x0043_dde0, Conv::Thiscall(ns), &[], |g| {
                d::release_owned(g, ns);
                Out::None
            });
        }
        assert_eq!(t.log_entries(), 3);
        t.reset();
        // the member array constructors: PClassDataMember's recorded
        t.stub(0x0057_52a0, true, 6, 6, 0x5d);
        let member = t.junk(0x40, 4);
        let args = [1, 2, 3, 4, 5, 6, 7];
        let what = format!("{label}: PClassDataMemberArray");
        t.check(&what, 0x0057_5ef0, Conv::Thiscall(member), &args, |g| {
            Out::U32(d::member_array_ctor(g, member, args))
        });
        let ty = t.junk(0x40, 5);
        let args = [1, ty, 3, 4, 5, 6];
        let what = format!("{label}: PClassDataMemberArrayFixedSize");
        t.check(&what, 0x0057_5f30, Conv::Thiscall(member), &args, |g| {
            Out::U32(d::member_array_fixed_ctor(g, member, args))
        });
        assert_eq!(t.log_entries(), 2);
        t.reset();
        // instance data
        let class = t.junk(0x90, 6);
        t.w32(class + 0x1c, 0x18);
        t.w32(class + 0x8c, 0x10);
        let il = t.junk(0x40, 7);
        t.w32(il + 0x3c, 4);
        t.w32(il + 0x2c, class);
        let base = t.zeros(0x100);
        t.w32(il + 0x34, base);
        // the free list (linked through the first word) holds instance 2
        t.w32(il + 8, base + 2 * 0x18);
        for i in [0, 1, 2, 3, 4, u32::MAX] {
            let what = format!("{label}: instanceData {i}");
            t.check(&what, 0x0057_65e0, Conv::Thiscall(il), &[i], |g| {
                Out::U32(d::instance_data(g, il, i))
            });
        }
        // ResolveFixup: the cluster lookup recorded
        let table = t.zeros(12 * 0x300);
        let cluster = t.zeros(0x40);
        t.w32(cluster + 0x30, table);
        let id = t.zeros(4);
        for (found, index) in [(0, 1u16), (cluster, 1), (cluster, 0x0102)] {
            t.stub(0x0045_cd00, false, 2, 0, found);
            t.write(id, &index.to_le_bytes());
            let slot = t.junk(4, 8);
            let what = format!("{label}: ResolveFixup {found:#x} {index:#x}");
            t.check(
                &what,
                0x0045_d9b0,
                Conv::Cdecl,
                &[slot, id, 3, 0x00ab_0000],
                |g| Out::U32(d::resolve_asset_fixup(g, slot, id, 3, 0x00ab_0000)),
            );
            t.reset();
        }
    }
}

#[test]
fn stream_files_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the constructor: openFile recorded
        t.stub(stream::OPEN_FILE, true, 5, 5, 0x5e);
        for path in [0, 0x00ab_c000] {
            let file = t.junk(8, 1);
            let args = [path, 2, 3, 4, 5];
            let what = format!("{label}: PStreamFile {path:#x}");
            t.check(&what, 0x0060_7bc0, Conv::Thiscall(file), &args, |g| {
                Out::U32(s::file_ctor(g, file, args))
            });
        }
        assert_eq!(t.log_entries(), 1);
        t.reset();
        // closeFile: the entry's release and the two imports recorded
        t.stub(0x0061_aa50, true, 0, 0, 0x5f);
        let flush = t.stub_code(0xf1, false, 1, 1, 0x1234_5601);
        let close = t.stub_code(0xf2, false, 1, 1, 0x00ab_cd77);
        let (iat_flush, iat_close) = (t.rebase(0x00b0_c144), t.rebase(0x00b0_c0dc));
        t.w32(iat_flush, flush);
        t.w32(iat_close, close);
        for (handle, entry) in [
            (u32::MAX, 0),
            (u32::MAX, 0x0011_0000),
            (0x44, 0),
            (0x48, 0x0011_0000),
        ] {
            let file = t.zeros(8);
            t.w32(file, handle);
            t.w32(file + 4, entry);
            let what = format!("{label}: closeFile {handle:#x} {entry:#x}");
            t.check(&what, 0x0060_7d80, Conv::Thiscall(file), &[], |g| {
                Out::U32(s::close_file(g, file))
            });
        }
        assert_eq!(t.log_entries(), 2 + 2 * 2);
        t.reset();
        // read: from the entry (recorded), or ReadFile (a stub that
        // stores half the size as the count), or nothing
        t.stub(0x0061_aa60, true, 2, 2, 0x60);
        let read_file = [
            0x8b, 0x44, 0x24, 0x10, // mov eax, [esp+0x10]
            0x8b, 0x4c, 0x24, 0x0c, // mov ecx, [esp+0xc]
            0xd1, 0xe9, // shr ecx, 1 (a short read)
            0x89, 0x08, // mov [eax], ecx
            0xb8, 1, 0, 0, 0, // mov eax, 1
            0xc2, 0x14, 0x00, // ret 0x14
        ];
        let rf = t.a.alloc_code(&read_file).unwrap();
        assert_eq!(t.b.alloc_code(&read_file).unwrap(), rf);
        let iat_read = t.rebase(0x00b0_c140);
        t.w32(iat_read, rf);
        for (handle, entry) in [(u32::MAX, 0), (u32::MAX, 0x0011_0000), (0x44, 0)] {
            let file = t.zeros(8);
            t.w32(file, handle);
            t.w32(file + 4, entry);
            let what = format!("{label}: read {handle:#x} {entry:#x}");
            t.check(
                &what,
                0x0060_8090,
                Conv::Thiscall(file),
                &[0x00b0_0000, 0x321],
                |g| {
                    let count = g.scratch();
                    Out::U32(s::file_read(g, count, file, 0x00b0_0000, 0x321))
                },
            );
        }
        assert_eq!(t.log_entries(), 1);
        t.reset();
    }
}

// ---- batch 4: iterators, enum lookups, namespace helpers ----

const SEEKS: [u32; 12] = [
    0x0046_4b40,
    0x0046_4bb0,
    0x0046_4c20,
    0x0046_4c90,
    0x0046_4d00,
    0x0046_4d70,
    0x0046_4de0,
    0x0046_4e50,
    0x0046_4ec0,
    0x0046_4f30,
    0x0046_4fa0,
    0x0046_5010,
];
const NEXTS: [u32; 5] = [
    0x0046_5080,
    0x0046_5140,
    0x0046_5260,
    0x0046_5380,
    0x0046_53e0,
];

/// An owner whose list (head at `+0x1c`, linked through the elements'
/// first word) holds four elements for the wanted class `want`: no
/// match, a match with count 0, two matches. Returns the owner and the
/// elements.
fn iter_owner(t: &mut Twins, want: u32) -> (u32, [u32; 4]) {
    let owner = t.zeros(0x40);
    let end = owner + 0x1c;
    let e: Vec<u32> = (0..4).map(|_| t.zeros(0x50)).collect();
    let other = t.zeros(0x50);
    for i in 0..4 {
        let next = if i < 3 { e[i + 1] } else { end };
        t.w32(e[i], next);
    }
    t.w32(end, e[0]);
    // e0's chain: other only; e1: want, count 0; e2: other then want; e3: want
    t.w32(e[0] + 0x2c, other);
    t.w32(e[1] + 0x2c, want);
    t.w32(e[2] + 0x2c, other);
    t.w32(other + 0x40, 0);
    t.w32(e[3] + 0x2c, want);
    t.w32(e[2] + 0x40, 2);
    t.w32(e[3] + 0x40, 1);
    // e2's chain goes on from `other` to `want` through a second node
    let other2 = t.zeros(0x50);
    t.w32(e[2] + 0x2c, other2);
    t.w32(other2 + 0x40, want);
    (owner, [e[0], e[1], e[2], e[3]])
}

#[test]
fn iterators_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(0x0044_1dc0, true, 1, 1, 0x77);
        let want = t.zeros(0x50);
        let (owner, e) = iter_owner(&mut t, want);
        let end = owner + 0x1c;
        for (k, &sym) in SEEKS.iter().chain(NEXTS.iter()).enumerate() {
            let seek = k < SEEKS.len();
            for cur in [e[0], e[1], e[2], e[3], 0] {
                let it = t.junk(0x20, 1);
                t.w32(it + 0x14, end);
                t.w32(it + 0x18, cur);
                t.w32(it + 0x1c, want);
                let what = format!("{label}: iter {sym:#x} {cur:#x}");
                t.check(&what, sym, Conv::Thiscall(it), &[], |g| {
                    Out::U32(if seek {
                        c::iter_seek(g, it)
                    } else {
                        c::iter_next(g, it)
                    })
                });
            }
        }
        // iterBegin: the wanted class is the global one
        let class = t.rebase(0x00cb_1120);
        let (owner, _) = iter_owner(&mut t, class);
        let it = t.junk(0x20, 2);
        let what = format!("{label}: iterBegin");
        t.check(&what, 0x0046_38f0, Conv::Thiscall(it), &[owner], |g| {
            Out::U32(c::iter_begin(g, it, owner))
        });
        // an empty list whose head looks like a match: the head is never
        // an element
        let empty = t.zeros(0x80);
        t.w32(empty + 0x1c, empty + 0x1c);
        t.w32(empty + 0x1c + 0x2c, class);
        t.w32(empty + 0x1c + 0x40, 1);
        let what = format!("{label}: iterBegin, empty");
        t.check(&what, 0x0046_38f0, Conv::Thiscall(it), &[empty], |g| {
            Out::U32(c::iter_begin(g, it, empty))
        });
        t.reset();
    }
}

#[test]
fn cluster_walks_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the PClusterHeader constructInstance, its constructor run as is
        let p = t.junk(0x60, 1);
        for at in [0, p] {
            let what = format!("{label}: construct header {at:#x}");
            t.check(&what, 0x0046_9460, Conv::Stdcall, &[at], |g| {
                Out::U8(c::construct_header(g, at))
            });
        }
        // FixupClusters, both passes recorded
        let list = t.zeros(16);
        t.w32(list, 0x00c1_0000);
        t.w32(list + 8, 0x00c2_0000);
        for (resolve, fixup) in [(0, 0), (5, 0), (0, 7)] {
            t.stub(0x0044_1f20, true, 0, 0, resolve);
            t.stub(0x0044_1620, true, 0, 0, fixup);
            for n in [0, 4] {
                let what = format!("{label}: FixupClusters {resolve} {fixup} {n}");
                t.check(&what, 0x0062_3580, Conv::Cdecl, &[list, n], |g| {
                    Out::U32(c::fixup_clusters(g, list, n))
                });
            }
            t.reset();
        }
        // findById
        let items: Vec<u32> = (0..3)
            .map(|i| {
                let e = t.zeros(0x30);
                t.w32(e + 0x2c, 10 + i);
                e
            })
            .collect();
        let data = t.zeros(12);
        t.w32(data, items[0]);
        t.w32(data + 8, items[2]);
        let one = t.zeros(8);
        t.w32(one, 0x8000_0001);
        t.w32(one + 4, items[1]);
        let three = t.zeros(8);
        t.w32(three, 3);
        t.w32(three + 4, data);
        let none = t.zeros(4);
        let zero = t.zeros(8);
        for arr in [one, three, zero] {
            let r = t.zeros(4);
            t.w32(r, arr);
            for id in [10, 11, 12, 13] {
                let what = format!("{label}: findById {arr:#x} {id}");
                t.check(&what, 0x0045_cd00, Conv::Cdecl, &[r, id], |g| {
                    Out::U32(c::find_by_id(g, r, id))
                });
            }
        }
        let what = format!("{label}: findById, no array");
        t.check(&what, 0x0045_cd00, Conv::Cdecl, &[none, 10], |g| {
            Out::U32(c::find_by_id(g, none, 10))
        });
        // mapping entries
        let map = t.zeros(0x20);
        t.w32(map + 4, 0x8000_0003);
        t.w32(map + 8, 0x0010_0000);
        t.w32(map + 0x10, 0x0020_0000);
        let empty_map = t.zeros(0x20);
        for (m, i, pre) in [
            (map, 0, 0),
            (map, 2, 0),
            (map, 3, 0),
            (map, 4, 0),
            (map, 9, 5),
            (empty_map, 0, 0),
            (empty_map, 1, 0),
        ] {
            let a = t.zeros(4);
            let b = t.zeros(4);
            t.w32(b, pre);
            let what = format!("{label}: mapping entry {m:#x} {i}");
            t.check(&what, 0x0047_e730, Conv::Thiscall(m), &[i, a, b], |g| {
                Out::U8(c::mapping_entry(g, m, i, a, b))
            });
        }
        // zeroed entries and the record
        for sym in [0x0047_baa0, 0x0047_bb00] {
            for n in [0, 1, 3] {
                let at = t.junk(0x80, 3);
                let what = format!("{label}: zeroEntries {sym:#x} {n}");
                t.check(&what, sym, Conv::Cdecl, &[at, n], |g| {
                    c::zero_entries(g, at, n);
                    Out::None
                });
            }
        }
        let rec = t.junk(0x30, 4);
        let what = format!("{label}: recordInit");
        t.check(
            &what,
            0x0057_7760,
            Conv::Thiscall(rec),
            &[1, 2, 3, 0x1234_56ab],
            |g| Out::U32(c::record_init(g, rec, 1, 2, 3, 0x1234_56ab)),
        );
    }
}

/// A stub for a `thiscall (&key)` lookup that copies the key's two words
/// to `out` (the key's address differs between the original's frame and
/// the port's) and returns `ret`.
fn key_stub(t: &mut Twins, out: u32, ret: u32) -> u32 {
    let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
    c.extend([0x8b, 0x10, 0x89, 0x15]); // mov edx, [eax]; mov [out], edx
    c.extend(out.to_le_bytes());
    c.extend([0x8b, 0x50, 0x04, 0x89, 0x15]); // mov edx, [eax+4]; mov [out+4], edx
    c.extend((out + 4).to_le_bytes());
    c.extend([0x89, 0x0d]); // mov [out+8], ecx
    c.extend((out + 8).to_le_bytes());
    c.push(0xb8);
    c.extend(ret.to_le_bytes()); // mov eax, ret
    c.extend([0xc2, 0x04, 0x00]); // ret 4
    let at = t.a.alloc_code(&c).unwrap();
    assert_eq!(t.b.alloc_code(&c).unwrap(), at);
    at
}

#[test]
fn namespace_helpers_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the dispensers' lookups by name: the key copied by the stub
        for (sym, find) in [(0x0043_7570, 0x0043_75c0), (0x0043_8630, 0x0043_8680)] {
            let out = t.zeros(12);
            let code = key_stub(&mut t, out, 0x4444);
            let at = t.rebase(find);
            let mut j = vec![0xe9];
            j.extend(code.wrapping_sub(at + 5).to_le_bytes());
            let old = Oracle::read(&mut t.a, at, 5).unwrap();
            t.saved.entry(at).or_insert(old);
            t.write(at, &j);
            let name = t.alloc(b"PClassDescriptor\0");
            let empty = t.alloc(b"\0");
            for n in [name, empty, 0] {
                let what = format!("{label}: findByName {sym:#x} {n:#x}");
                t.check(&what, sym, Conv::Thiscall(0x00d1_5000), &[n], |g| {
                    let key = g.scratch();
                    Out::U32(d::find_by_name(g, find, key, 0x00d1_5000, n))
                });
            }
            t.reset();
        }
        // getDefaultInstance on a fake descriptor
        t.stub(descriptor::CALCULATE_DEFAULT, true, 0, 0, 1);
        t.stub(descriptor::CALCULATE_VALIDATION, true, 0, 0, 2);
        let obj = t.object(&[0; 13]);
        let desc = t.zeros(0xa0);
        let vt = t.r32(obj);
        t.w32(desc, vt);
        for (inst, flags, buffer) in [
            (0x55, 0, 0),
            (0, 0x8000_0000, 0),
            (0, 0x8000_0000, 0x900),
            (0, 0, 0x900),
            (0, 0x4000_0000, 0),
        ] {
            t.w32(desc + 0x70, inst);
            t.w32(desc + descriptor::FLAGS, flags);
            t.w32(desc + descriptor::DEFAULT_BUFFER, buffer);
            t.w32(desc + 0x8c, 0x10);
            let what = format!("{label}: getDefaultInstance {inst:#x} {flags:#x} {buffer:#x}");
            t.check(&what, 0x0043_cbb0, Conv::Thiscall(desc), &[], |g| {
                Out::U32(d::default_instance(g, desc))
            });
        }
        t.reset();
        // the real descriptors (snapshots)
        if t.snapshot {
            for (i, real) in t.descriptors(48).into_iter().enumerate() {
                let what = format!("{label}: real getDefaultInstance {i}");
                t.check(&what, 0x0043_cbb0, Conv::Thiscall(real), &[], |g| {
                    Out::U32(d::default_instance(g, real))
                });
            }
        }
        // constructWithDefault: the font descriptor's instance set by hand
        let font = t.rebase(0x00ca_37d0);
        let p = t.junk(0x30, 5);
        for inst in [0x0abc_0000, 0] {
            t.w32(font + 0x70, inst);
            t.w32(font + descriptor::FLAGS, 0x8000_0000);
            t.w32(font + descriptor::DEFAULT_BUFFER, 0);
            for at in [p, 0] {
                let what = format!("{label}: constructWithDefault {inst:#x} {at:#x}");
                t.check(&what, 0x00a2_d570, Conv::Stdcall, &[at], |g| {
                    Out::U8(d::construct_with_default(g, at))
                });
            }
        }
        // constructTyped
        for at in [p, 0] {
            let what = format!("{label}: constructTyped {at:#x}");
            t.check(&what, 0x0044_4760, Conv::Stdcall, &[at], |g| {
                Out::U8(d::construct_typed(g, at))
            });
        }
        // PNamespace::remove: GetGlobalNamespace recorded
        for global_is_ns in [false, true] {
            for keep_one in [true, false] {
                let parent = t.zeros(8);
                let ns = t.zeros(0x20);
                let sibling = t.zeros(8);
                // parent -> ns -> sibling -> parent
                t.w32(parent, ns);
                t.w32(parent + 4, sibling);
                t.w32(ns, sibling);
                t.w32(ns + 4, parent);
                t.w32(sibling, parent);
                t.w32(sibling + 4, ns);
                let head = ns + 8;
                let d1 = t.zeros(0x40);
                let d2 = t.zeros(0x40);
                let (l1, l2) = (d1 + 0x2c, d2 + 0x2c);
                if keep_one {
                    t.w32(head, l1);
                    t.w32(head + 4, l2);
                    t.w32(l1, l2);
                    t.w32(l1 + 4, head);
                    t.w32(l2, head);
                    t.w32(l2 + 4, l1);
                } else {
                    t.w32(head, l2);
                    t.w32(head + 4, l2);
                    t.w32(l2, head);
                    t.w32(l2 + 4, head);
                }
                t.stub(
                    descriptor::GLOBAL_NAMESPACE,
                    false,
                    0,
                    0,
                    if global_is_ns { ns } else { 0x00ee_0000 },
                );
                let what = format!("{label}: namespace remove {global_is_ns} {keep_one}");
                t.check(&what, 0x0043_ec40, Conv::Thiscall(ns), &[d2], |g| {
                    Out::U32(d::namespace_remove(g, ns, d2))
                });
                t.reset();
            }
        }
    }
}

#[test]
fn enum_lookups_match_the_original() {
    for (label, mut t) in Twins::all() {
        let names: Vec<u32> = [&b"Linear\0"[..], b"Lin\0", b"Step\0"]
            .iter()
            .map(|n| t.alloc(n))
            .collect();
        let probes: Vec<u32> = [&b"Linear\0"[..], b"Step\0", b"Li\0", b"Stepx\0", b"\0"]
            .iter()
            .map(|n| t.alloc(n))
            .collect();
        let tables = descriptor::ENUM_TABLES
            .iter()
            .map(|&(f, head)| (f, head, 4, 8))
            .chain([
                (0x0050_7a30, 0x00ca_9734, 0, 4),
                (0x0057_87d0, 0x00cc_969c, 0, 4),
            ]);
        for (f, head, name_at, next_at) in tables {
            // three entries, last first in the list
            let mut first = 0;
            for &n in &names {
                let e = t.zeros(0x10);
                t.w32(e + name_at, n);
                t.w32(e + next_at, first);
                first = e;
            }
            let h = t.rebase(head);
            for start in [first, 0] {
                t.w32(h, start);
                for &probe in probes.iter().chain([0u32].iter()) {
                    let what = format!("{label}: enum {f:#x} {start:#x} {probe:#x}");
                    t.check(&what, f, Conv::Cdecl, &[probe], |g| {
                        Out::U32(d::enum_find(g, head, name_at, next_at, probe))
                    });
                }
            }
        }
    }
}

// ---- batch 5: the GetType<T> statics ----

#[test]
fn type_statics_match_the_original() {
    use ffx_pload::types;
    for (label, mut t) in Twins::all() {
        t.stub(types::ATEXIT, false, 1, 0, 0);
        // each getter the first time (guard cleared), then again
        for (i, row) in types::PRIMITIVES.iter().enumerate() {
            let guard = t.rebase(row.object) + 0x2c;
            let g = t.r32(guard);
            t.w32(guard, g & !1);
            for pass in 0..2 {
                let what = format!("{label}: primitive {i} ({:#x}) pass {pass}", row.getter);
                t.check(&what, row.getter, Conv::Cdecl, &[], |g| {
                    Out::U32(types::bridge::primitive(g, row))
                });
            }
        }
        for (i, row) in types::ENUMS.iter().enumerate() {
            let guard = t.rebase(row.object) + 0x34;
            let g = t.r32(guard);
            t.w32(guard, g & !1);
            for pass in 0..2 {
                let what = format!("{label}: enum {i} ({:#x}) pass {pass}", row.getter);
                t.check(&what, row.getter, Conv::Cdecl, &[], |g| {
                    Out::U32(types::bridge::enum_type(g, row))
                });
            }
        }
        let guard = t.rebase(types::DISPENSER) + 0x18;
        let g = t.r32(guard);
        t.w32(guard, g & !1);
        for pass in 0..2 {
            let what = format!("{label}: dispenser pass {pass}");
            t.check(&what, 0x0043_9040, Conv::Cdecl, &[], |g| {
                Out::U32(types::bridge::dispenser(g))
            });
        }
        // one atexit per first pass
        assert_eq!(t.log_entries(), 13 + 9 + 1);
        t.reset();
    }
}

#[test]
fn batch5_helpers_match_the_original() {
    use ffx_pload::types;
    for (label, mut t) in Twins::all() {
        // the asset registry's static
        t.stub(types::ATEXIT, false, 1, 0, 0);
        let guard = t.rebase(types::ASSET_REGISTRY) + 0x18;
        let g = t.r32(guard);
        t.w32(guard, g & !1);
        for pass in 0..2 {
            let what = format!("{label}: asset registry pass {pass}");
            t.check(&what, 0x0045_d420, Conv::Cdecl, &[], |g| {
                Out::U32(types::bridge::list_static(
                    g,
                    types::ASSET_REGISTRY,
                    types::ASSET_REGISTRY_DTOR,
                ))
            });
        }
        assert_eq!(t.log_entries(), 1);
        t.reset();
        // the reader file's destructor and the string release: frees recorded
        t.stub(descriptor::FREE, false, 1, 0, 0);
        t.stub(0x0043_4360, false, 2, 0, 0);
        for name in [0x00ab_0000, 0x00ab_0001] {
            let r = t.junk(0x20, 1);
            t.w32(r + 8, name);
            let what = format!("{label}: reader file dtor {name:#x}");
            t.check(&what, 0x0057_67c0, Conv::Thiscall(r), &[], |g| {
                s::reader_file_dtor(g, r);
                Out::None
            });
        }
        for (word, text) in [
            (0, 0x00ab_0000),
            (1, 0x00ab_0000),
            (1, 0x00ab_0001),
            (2, 0x00ab_0000),
            (3, 0x00ab_0000),
            (0x4000_0003, 0x00ab_0000),
            (0x8000_0003, 0x00ab_0000),
        ] {
            let st = t.zeros(8);
            t.w32(st, word);
            t.w32(st + 4, text);
            let what = format!("{label}: string release {word:#x} {text:#x}");
            t.check(&what, 0x0044_af00, Conv::Thiscall(st), &[], |g| {
                d::string_release(g, st);
                Out::None
            });
        }
        assert_eq!(t.log_entries(), 1 + 4);
        t.reset();
        // constructInGlobal: the namespace and 0x00447690 recorded
        t.stub(descriptor::GLOBAL_NAMESPACE, false, 0, 0, 0x00ee_0000);
        t.stub(0x0044_7690, true, 3, 3, 0x12);
        for p in [0, 0x00ab_c000] {
            let what = format!("{label}: constructInGlobal {p:#x}");
            t.check(&what, 0x0044_9420, Conv::Stdcall, &[p], |g| {
                Out::U8(d::construct_in_global(g, p))
            });
        }
        assert_eq!(t.log_entries(), 2);
        t.reset();
        // the cluster teardown: clear and 0x0049c5b0 recorded, the
        // namespace's release run (one owned descriptor)
        t.stub(0x0044_0eb0, true, 0, 0, 0);
        t.stub(0x0049_c5b0, true, 0, 0, 0);
        let cl = t.zeros(0x60);
        let o = t.object(&[0, 0, 0, 0, 0, 0, 0, 1]);
        let desc = t.zeros(0xa0);
        let vt = t.r32(o);
        t.w32(desc, vt);
        t.w32(desc + descriptor::FLAGS, 1);
        t.w32(cl + 8, desc + 0x2c);
        t.w32(desc + 0x2c, cl + 8);
        let what = format!("{label}: cluster teardown");
        t.check(&what, 0x0043_faa0, Conv::Thiscall(cl), &[], |g| {
            c::cluster_teardown(g, cl);
            Out::None
        });
        assert_eq!(t.log_entries(), 3);
        t.reset();
        // PClassMember: the allocator recorded (memcpy runs)
        let name = t.alloc(b"m_value\0");
        for (flags, block) in [(0, 0), (2, 0), (1, 0), (0x8000_0001, 1)] {
            let buf = if block == 0 { 0 } else { t.zeros(16) };
            t.stub(0x0042_fb80, false, 2, 0, buf);
            let m = t.junk(0x20, 2);
            let what = format!("{label}: PClassMember {flags:#x} {buf:#x}");
            t.check(
                &what,
                0x0043_a080,
                Conv::Thiscall(m),
                &[0x77, name, flags],
                |g| Out::U32(d::member_ctor(g, m, 0x77, name, flags)),
            );
            t.reset();
        }
    }
}

// ---- batch 6 ----

#[test]
fn batch6_arrays_and_strings_match_the_original() {
    for (label, mut t) in Twins::all() {
        // PArray<u8>::resize: the allocator and free recorded
        for (block, count, old, n) in [
            (0x00b1_0000, 4, 0x00aa_0000, 4),
            (0x00b1_0000, 4, 0x00aa_0000, 0),
            (0x00b1_0000, 4, 0x00aa_0000, 6),
            (0, 4, 0x00aa_0000, 6),
            (0x00b1_0000, 0x8000_0004, 0x00aa_0000, 6),
            (0x00b1_0000, 4, 0x00b1_0000, 6),
            (0x00b1_0000, 4, 0, 6),
        ] {
            t.stub(descriptor::ALLOC, false, 2, 0, block);
            t.stub(descriptor::FREE, false, 1, 0, 0);
            let arr = t.zeros(8);
            t.w32(arr, count);
            t.w32(arr + 4, old);
            let what = format!("{label}: resize {block:#x} {count:#x} {old:#x} {n}");
            t.check(&what, 0x0044_a010, Conv::Thiscall(arr), &[n], |g| {
                Out::U32(d::array_resize(g, arr, n))
            });
            t.reset();
        }
        // PString copies (memcpy runs)
        let text = t.alloc(b"PCluster\0");
        for (block, ptr) in [(0, 0), (1, text), (1, text + 1), (0, text)] {
            let buf = if block == 0 { 0 } else { t.zeros(16) };
            t.stub(descriptor::ALLOC, false, 2, 0, buf);
            let src = t.zeros(4);
            t.w32(src, ptr);
            let dst = t.junk(4, 1);
            let what = format!("{label}: string copy {buf:#x} {ptr:#x}");
            t.check(&what, 0x0044_ae10, Conv::Thiscall(dst), &[src], |g| {
                Out::U32(d::string_copy(g, dst, src))
            });
            t.reset();
        }
        // PStream's constructor
        for (block, name) in [(1, text), (0, text), (1, 0)] {
            let buf = if block == 0 { 0 } else { t.zeros(16) };
            t.stub(descriptor::ALLOC, false, 2, 0, buf);
            let st = t.junk(0x10, 2);
            let what = format!("{label}: PStream {buf:#x} {name:#x}");
            t.check(&what, 0x0060_f050, Conv::Thiscall(st), &[name], |g| {
                Out::U32(s::stream_ctor(g, st, name))
            });
            t.reset();
        }
    }
}

#[test]
fn batch6_members_match_the_original() {
    for (label, mut t) in Twins::all() {
        let marker = t.r32(t.rebase(descriptor::DEFAULT_INSTANCE.sym));
        for dup in [0, 1] {
            t.stub(0x0043_5ca0, true, 1, 1, dup);
            t.stub(0x0044_9c80, true, 0, 0, 0);
            t.stub(0x0044_9e40, true, 0, 0, 0);
            for (size, offset, name, default, words) in [
                (0x100, 0x10, 0x00ab_0000, false, [3u16, 3]),
                (0x100, 0x10, 0x00ab_0000, false, [3, 4]),
                (0x10, 0x10, 0x00ab_0000, false, [3, 3]),
                // need is 0x40 - 0x10 + 0x10 + 8 = 0x48: exactly, one short, room
                (0x48, 0x10, 0x00ab_0000, false, [3, 3]),
                (0x47, 0x10, 0x00ab_0000, false, [3, 3]),
                (0x50, 0x10, 0x00ab_0000, false, [3, 3]),
                (0x100, 0x10, 0, false, [3, 3]),
                (0x100, 0x10, 0x00ab_0000, true, [3, 3]),
                (0xffff_fff0, 0x10, 0x00ab_0000, false, [3, 3]),
            ] {
                let desc = t.zeros(0xa0);
                t.w32(desc + 0x8c, 0x40);
                t.w32(desc + 0x88, 0x10);
                t.w32(desc + 0x1c, size);
                t.write(desc + 0x94, &[words[0] as u8, 0, words[1] as u8, 0]);
                // an empty member list with one entry already
                let other = t.zeros(0x30);
                let head = desc + 0x44;
                t.w32(head, other + 4);
                t.w32(head + 4, other + 4);
                t.w32(other + 4, head);
                t.w32(other + 8, head);
                let member = t.junk(0x30, 3);
                t.w32(member + 0x24, offset);
                t.w32(member + 0x10, if default { marker } else { name });
                for sym in [0x0043_c0a0, 0x0044_90a0] {
                    let what = format!(
                        "{label}: addMember {sym:#x} {dup} {size:#x} {name:#x} {default} {words:?}"
                    );
                    t.check(&what, sym, Conv::Thiscall(desc), &[member, 8], |g| {
                        Out::U32(if sym == 0x0043_c0a0 {
                            d::add_member(g, desc, member, 8)
                        } else {
                            d::add_dynamic_member(g, desc, member, 8)
                        })
                    });
                }
            }
            t.reset();
        }
        // updateBaseOffsets: a chain to 0x00435740()'s root, and one to 0
        let root = t.rebase(0x00c9_02e8);
        // a size on the root, which the walk must not count
        t.w32(root + 0x84, 0x1000);
        for to_root in [true, false] {
            let a = t.zeros(0xa0);
            let b = t.zeros(0xa0);
            let c = t.zeros(0xa0);
            t.w32(a + 0x84, 0x10);
            t.w32(b + 0x84, 0x20);
            t.w32(c + 0x84, 0xffff_fff0);
            t.w32(b + 0x8c, 0x300);
            t.w32(a + descriptor::PARENT, b);
            t.w32(b + descriptor::PARENT, c);
            t.w32(c + descriptor::PARENT, if to_root { root } else { 0 });
            for d in [a, c] {
                let what = format!("{label}: updateBaseOffsets {to_root} {d:#x}");
                t.check(&what, 0x0043_d6e0, Conv::Thiscall(d), &[], |g| {
                    Out::U32(d::update_base_offsets(g, d))
                });
            }
        }
        // deleteAll: slot 0, the assertion import and the free hook recorded
        let assert = t.stub_code(0xa5, false, 3, 0, 0);
        let free = t.stub_code(0xf4, false, 1, 0, 0);
        let (ia, ifr) = (t.rebase(0x00b0_c5c0), t.rebase(0x022f_b514));
        t.w32(ia, assert);
        t.w32(ifr, free);
        let o1 = t.object(&[1]);
        let o2 = t.object(&[1]);
        let data = t.zeros(12);
        t.w32(data, o1);
        t.w32(data + 8, o2);
        for (n, dp) in [(3, data), (0, data), (0xffff_ffff, data), (3, 0)] {
            let table = t.zeros(0x40);
            t.w32(table + 0x30, dp);
            t.w32(table + 0x38, n);
            if dp == 0 {
                continue;
            }
            let what = format!("{label}: deleteAll {n:#x}");
            t.check(&what, 0x0042_b7c0, Conv::Thiscall(table), &[], |g| {
                d::delete_all(g, table);
                Out::None
            });
        }
        assert_eq!(t.log_entries(), 4);
        t.reset();
        // the PArray<PTypedObject> constructInstance
        t.stub(0x0044_d6a0, true, 1, 1, 0);
        let p = t.junk(0x20, 4);
        for at in [0, p] {
            let what = format!("{label}: constructTypedArray {at:#x}");
            t.check(&what, 0x0045_9600, Conv::Stdcall, &[at], |g| {
                Out::U8(d::construct_typed_array(g, at))
            });
        }
        assert_eq!(t.log_entries(), 1);
        t.reset();
    }
}

#[test]
fn batch6_clusters_match_the_original() {
    use ffx_pload::types;
    for (label, mut t) in Twins::all() {
        // the second PClusterHeader constructInstance and the base header
        let p = t.junk(0x60, 1);
        for at in [0, p] {
            let what = format!("{label}: construct header b {at:#x}");
            t.check(&what, 0x0046_9e60, Conv::Stdcall, &[at], |g| {
                Out::U8(c::construct_header(g, at))
            });
        }
        let hb = t.junk(0x60, 9);
        let what = format!("{label}: header base ctor");
        t.check(&what, 0x0046_8670, Conv::Thiscall(hb), &[], |g| {
            Out::U32(c::header_base_ctor(g, hb))
        });
        // setLayout
        t.stub(0x004c_2430, false, 3, 0, 0x99);
        for (cur, flags, a, b, shift) in [
            ([8, 2, 3], 0, 8, 2, 3),
            ([8, 2, 3], 0, 8, 2, 4),
            ([32, 2, 3], 0, 32, 2, 4),
            ([8, 2, 3], 0, 0, 2, 0),
            ([8, 2, 3], 2, 9, 2, 3),
            ([8, 2, 3], 0, 4, 2, 3),
            ([8, 2, 3], 0, 8, 5, 0),
            ([8, 2, 3], 2, 64, 5, 40),
            ([8, 2, 0x99], 2, 8, 7, 3),
        ] {
            let o = t.zeros(0x20);
            t.w32(o + 0x1c, cur[0]);
            t.w32(o, cur[1]);
            t.w32(o + 0xc, cur[2]);
            t.w32(o + 0x10, cur[2]);
            t.w32(o + 0x14, flags);
            let what = format!("{label}: setLayout {cur:?} {flags} {a} {b} {shift}");
            t.check(&what, 0x004c_7260, Conv::Thiscall(o), &[a, b, shift], |g| {
                Out::U32(c::set_layout(g, o, a, b, shift))
            });
        }
        t.reset();
        // PCluster's destructor
        t.stub(0x0044_0eb0, true, 0, 0, 0);
        t.stub(0x0044_3b70, true, 1, 1, 0);
        t.stub(0x0057_6640, true, 0, 0, 0);
        for owner in [0, 0x00ab_0000] {
            let cl = t.junk(0x60, 2);
            t.w32(cl + 0x40, owner);
            let what = format!("{label}: PCluster dtor {owner:#x}");
            t.check(&what, 0x0043_fa30, Conv::Thiscall(cl), &[], |g| {
                c::cluster_dtor(g, cl);
                Out::None
            });
        }
        assert_eq!(t.log_entries(), 5);
        t.reset();
        // PClusterBase's constructor: the pool's recorded
        t.stub(0x0049_c480, true, 4, 4, 0);
        let cl = t.junk(0x60, 3);
        let what = format!("{label}: PClusterBase");
        t.check(&what, 0x0043_f820, Conv::Thiscall(cl), &[], |g| {
            Out::U32(c::cluster_base_ctor(g, cl))
        });
        t.reset();
        // GetGlobalNamespace, built from a cleared guard, then again
        t.stub(types::ATEXIT, false, 1, 0, 0);
        let guard = t.rebase(types::GLOBAL_NAMESPACE) + 0x1c;
        let gv = t.r32(guard);
        t.w32(guard, gv & !1);
        for pass in 0..2 {
            let what = format!("{label}: GetGlobalNamespace {pass}");
            t.check(&what, 0x0043_e2f0, Conv::Cdecl, &[], |g| {
                Out::U32(types::bridge::global_namespace(g))
            });
        }
        assert_eq!(t.log_entries(), 1);
        t.reset();
        // PStreamReaderFile's constructor: open, PStream and size recorded
        t.stub(stream::OPEN_FILE, true, 5, 5, 0);
        t.stub(stream::STREAM_CTOR, true, 1, 1, 0);
        t.stub(0x0060_7dc0, true, 0, 0, 0x1234);
        let r = t.junk(0x20, 4);
        let what = format!("{label}: PStreamReaderFile");
        t.check(&what, 0x0057_66d0, Conv::Thiscall(r), &[0x00ab_c000], |g| {
            Out::U32(s::reader_file_ctor(g, r, 0x00ab_c000))
        });
        assert_eq!(t.log_entries(), 3);
        t.reset();
    }
}

// ---- batch 7 ----

#[test]
fn batch7_clusters_match_the_original() {
    use ffx_pload::types;
    for (label, mut t) in Twins::all() {
        // releaseBlocks: frees recorded
        t.stub(descriptor::FREE, false, 1, 0, 0);
        let rec = t.junk(0x28, 1);
        let what = format!("{label}: releaseBlocks");
        t.check(&what, 0x0059_89b0, Conv::Thiscall(rec), &[], |g| {
            c::release_blocks(g, rec);
            Out::None
        });
        assert_eq!(t.log_entries(), 4);
        t.reset();
        // placeOverallocation
        for (over, align, cur) in [
            (0, 4, 0x100),
            (3, 0, 0x101),
            (3, 8, 0x101),
            (3, 8, 0),
            (3, 0, 0),
            (5, 16, 0x1003),
        ] {
            let class = t.zeros(0x80);
            t.w32(class + 0x74, over);
            t.w32(class + 0x78, align);
            t.w32(class + 0x1c, 0x18);
            let list = t.junk(0x48, 2);
            t.w32(list + 0x2c, class);
            t.w32(list + 0x3c, 4);
            t.w32(list + 0x44, 0x13);
            t.w32(list + 0x34, 0x0010_0001);
            let cursor = t.zeros(4);
            t.w32(cursor, cur);
            let what = format!("{label}: placeOverallocation {over} {align} {cur:#x}");
            t.check(&what, 0x0057_8300, Conv::Cdecl, &[cursor, list], |g| {
                c::place_overallocation(g, cursor, list);
                Out::None
            });
        }
        // the tree insert, then lookups through the ported find
        let tree = t.zeros(0x20);
        t.w32(tree + 0xc, tree);
        for (i, key) in [10u32, 5, 20, 7, 10, 30, 1].into_iter().enumerate() {
            let node = t.junk(0x20, i as u8);
            t.w32(node + 0xc, key);
            let what = format!("{label}: tree insert {key}");
            t.check(&what, 0x0058_1750, Conv::Thiscall(tree), &[node], |g| {
                Out::U32(c::tree_insert(g, tree, node))
            });
        }
        // PCluster::create: the list allocation and initializeInstance recorded
        let list = t.zeros(0x40);
        t.w32(list + 0x30, 0x0020_0000);
        for (flags, n, got) in [(8, 3, list), (0, 0, list), (0, 3, 0), (0, 3, list)] {
            t.stub(0x0043_52f0, false, 3, 0, 0);
            t.stub(0x0044_0ba0, true, 4, 4, got);
            t.stub(0x0043_ce00, true, 1, 1, 0);
            let desc = t.zeros(0xa0);
            t.w32(desc + 0x90, flags);
            t.w32(desc + 0x1c, 0x24);
            t.w32(desc + 0x18, 0x00ab_0000);
            let cl = t.zeros(0x48);
            let what = format!("{label}: create {flags} {n} {got:#x}");
            t.check(&what, 0x0044_1010, Conv::Thiscall(cl), &[n, desc, 7], |g| {
                Out::U32(c::create(g, cl, n, desc, 7))
            });
            t.reset();
        }
        // PClusterBase's destructor: the registry and the teardown's callees recorded
        t.stub(0x004f_ee70, true, 0, 0, 0);
        t.stub(types::ATEXIT, false, 1, 0, 0);
        t.stub(0x0059_8c50, true, 1, 1, 0);
        t.stub(0x0044_0eb0, true, 0, 0, 0);
        t.stub(0x0049_c5b0, true, 0, 0, 0);
        let guard = t.rebase(0x00c9_628c);
        let gv = t.r32(guard);
        t.w32(guard, gv & !1);
        for pass in 0..2 {
            let cl = t.zeros(0x60);
            t.w32(cl + 8, cl + 8);
            let what = format!("{label}: PClusterBase dtor {pass}");
            t.check(&what, 0x0057_6640, Conv::Thiscall(cl), &[], |g| {
                c::cluster_base_dtor(g, cl);
                Out::None
            });
        }
        assert_eq!(t.log_entries(), 2 + 3 * 2);
        t.reset();
    }
}

#[test]
fn batch7_descriptors_match_the_original() {
    use ffx_pload::types;
    for (label, mut t) in Twins::all() {
        // findMember over a descriptor and its parent
        let names: Vec<u32> = [&b"a\0"[..], b"bb\0", b"ccc\0"]
            .iter()
            .map(|n| t.alloc(n))
            .collect();
        let child = t.zeros(0xa0);
        let parent = t.zeros(0xa0);
        t.w32(child + descriptor::PARENT, parent);
        for (d, ns) in [(child, &names[..2]), (parent, &names[2..])] {
            let head = d + 0x44;
            let mut prev = head;
            t.w32(head, head);
            t.w32(head + 4, head);
            for &n in ns {
                let m = t.zeros(0x30);
                t.w32(m + 0x10, n);
                let link = m + 4;
                t.w32(prev, link);
                t.w32(link + 4, prev);
                t.w32(link, head);
                t.w32(head + 4, link);
                prev = link;
            }
            // a member without a name
            let m = t.zeros(0x30);
            let link = m + 4;
            t.w32(prev, link);
            t.w32(link + 4, prev);
            t.w32(link, head);
            t.w32(head + 4, link);
        }
        let probes: Vec<u32> = [&b"a\0"[..], b"ccc\0", b"cc\0", b"\0"]
            .iter()
            .map(|n| t.alloc(n))
            .collect();
        for d in [child, parent, 0] {
            for &p in probes.iter().chain([0u32].iter()) {
                let what = format!("{label}: findMember {d:#x} {p:#x}");
                t.check(&what, 0x0043_5ca0, Conv::Thiscall(d), &[p], |g| {
                    Out::U32(d::find_member(g, d, p))
                });
            }
        }
        // findNamed over a list at +0x54
        let owner = t.zeros(0x60);
        let head = owner + 0x54;
        let e1 = t.zeros(0x10);
        let e2 = t.zeros(0x10);
        let e3 = t.zeros(0x10);
        t.w32(head, e1);
        t.w32(e1, e2);
        t.w32(e2, e3);
        t.w32(e3, head);
        t.w32(e1 + 0xc, names[0]);
        t.w32(e3 + 0xc, names[2]);
        // an empty list whose head looks like a named entry: the head is
        // never an entry
        let empty = t.zeros(0x70);
        t.w32(empty + 0x54, empty + 0x54);
        t.w32(empty + 0x54 + 0xc, names[0]);
        for o in [owner, empty] {
            for &p in probes.iter().chain([0u32].iter()) {
                let what = format!("{label}: findNamed {o:#x} {p:#x}");
                t.check(&what, 0x0043_06b0, Conv::Thiscall(o), &[p], |g| {
                    Out::U32(d::find_named(g, o, p))
                });
            }
        }
        // the dynamic member's scalar deleting destructor
        t.stub(descriptor::FREE, false, 1, 0, 0);
        t.stub(0x0057_5470, true, 0, 0, 0);
        for (name, flags) in [(0x00ab_0000, 0), (0x00ab_0001, 1), (0x00ab_0000, 3)] {
            let m = t.junk(0x30, 3);
            t.w32(m + 0x2c, name);
            let what = format!("{label}: member dynamic delete {name:#x} {flags}");
            t.check(&what, 0x0044_6400, Conv::Thiscall(m), &[flags], |g| {
                Out::U32(d::delete_member_dynamic(g, m, flags))
            });
        }
        assert_eq!(t.log_entries(), 2 + 3 + 2);
        t.reset();
        // PArray<T*>::resize
        for (block, count, old, n) in [
            (1, 4, 0x00aa_0000, 4),
            (1, 4, 0x00aa_0000, 3),
            (0, 4, 0x00aa_0000, 3),
            (1, 4, 0, 3),
            (1, 0x8000_0004, 0x00aa_0000, 3),
            (1, 4, 0x00aa_0000, 0),
        ] {
            let buf = if block == 0 { 0 } else { t.junk(16, 4) };
            t.stub(descriptor::ALLOC, false, 2, 0, buf);
            t.stub(descriptor::FREE, false, 1, 0, 0);
            let arr = t.zeros(8);
            t.w32(arr, count);
            t.w32(arr + 4, old);
            let what = format!("{label}: ptr resize {buf:#x} {count:#x} {old:#x} {n}");
            t.check(&what, 0x0047_efc0, Conv::Thiscall(arr), &[n], |g| {
                Out::U32(d::pointer_array_resize(g, arr, n))
            });
            t.reset();
        }
        // the PArray<PTypedObject> destructInstance
        t.stub(descriptor::FREE, false, 1, 0, 0);
        t.stub(0x0044_d730, true, 0, 0, 0);
        for (word, data) in [(3, 0x00aa_0000), (0x8000_0003, 0x00aa_0000), (3, 0)] {
            let p = t.junk(0x20, 5);
            t.w32(p + 0xc, word);
            t.w32(p + 0x10, data);
            let what = format!("{label}: destroyTypedArray {word:#x} {data:#x}");
            t.check(&what, 0x0045_96b0, Conv::Stdcall, &[p], |g| {
                Out::U8(d::destroy_typed_array(g, p))
            });
        }
        assert_eq!(t.log_entries(), 1 + 3);
        t.reset();
        // InitializeGlobalClassDescriptors: on the image with a built
        // namespace of fake descriptors, on the snapshots over the real one
        if !t.snapshot {
            t.stub(types::ATEXIT, false, 1, 0, 0);
            t.stub(descriptor::CALCULATE_VALIDATION, true, 0, 0, 0);
            let ns = t.rebase(types::GLOBAL_NAMESPACE);
            // calculateDefaultBuffer copies the descriptor's +0x8c (set by
            // the first pass) and the namespace's state to `seen`
            let seen = t.zeros(8);
            let mut code = vec![0x8b, 0x81, 0x8c, 0, 0, 0, 0xa3]; // mov eax, [ecx+0x8c]; mov [seen], eax
            code.extend(seen.to_le_bytes());
            code.push(0xa1); // mov eax, [ns+0x10]
            code.extend((ns + 0x10).to_le_bytes());
            code.push(0xa3); // mov [seen+4], eax
            code.extend((seen + 4).to_le_bytes());
            code.push(0xc3);
            let at = t.a.alloc_code(&code).unwrap();
            assert_eq!(t.b.alloc_code(&code).unwrap(), at);
            let calc = t.rebase(descriptor::CALCULATE_DEFAULT);
            let mut j = vec![0xe9];
            j.extend(at.wrapping_sub(calc + 5).to_le_bytes());
            let old = Oracle::read(&mut t.a, calc, 5).unwrap();
            t.saved.entry(calc).or_insert(old);
            t.write(calc, &j);
            let head = ns + 8;
            let o = t.object(&[0; 13]);
            let vt = t.r32(o);
            let mut prev = head;
            t.w32(ns + 0x1c, 1);
            for (i, flags) in [0u32, 0x8000_0000, 0x4000_0000].into_iter().enumerate() {
                let dsc = t.zeros(0xa0);
                t.w32(dsc, vt);
                t.w32(dsc + descriptor::FLAGS, flags);
                t.w32(dsc + 0x84, 0x10 * (i as u32 + 1));
                let link = dsc + 0x2c;
                t.w32(prev, link);
                t.w32(link + 4, prev);
                t.w32(link, head);
                t.w32(head + 4, link);
                prev = link;
            }
        }
        let what = format!("{label}: InitializeGlobalClassDescriptors");
        t.check(&what, 0x0043_e440, Conv::Cdecl, &[], |g| {
            d::init_global_descriptors(g);
            Out::None
        });
        t.reset();
    }
}

// ---- batch 8 ----

#[test]
fn batch8_dispenser_matches_the_original() {
    use ffx_pload::types;
    for (label, mut t) in Twins::all() {
        // the lookup on a synthetic tree: (hash, name) nodes, nil = tree
        let tree = t.zeros(0x20);
        let names: Vec<u32> = [&b"float\0"[..], b"bool\0", b"PType\0", b"PChar\0"]
            .iter()
            .map(|n| t.alloc(n))
            .collect();
        let node = |t: &mut Twins, hash: u32, name: u32| {
            let n = t.zeros(0x20);
            t.w32(n + 4, tree);
            t.w32(n + 8, tree);
            t.w32(n + 0x10, hash);
            t.w32(n + 0x14, name);
            n
        };
        let root = node(&mut t, 100, names[0]);
        let left = node(&mut t, 50, names[1]);
        let right = node(&mut t, 100, names[2]);
        t.w32(root + 4, left + 4);
        t.w32(root + 8, right + 4);
        let key = t.zeros(8);
        for (hash, name) in [
            (100, names[0]),
            (50, names[1]),
            (100, names[2]),
            (100, names[3]),
            (7, names[0]),
            (200, names[2]),
        ] {
            t.w32(key, hash);
            t.w32(key + 4, name);
            for root_link in [root + 4, tree] {
                t.w32(tree + 0xc, root_link);
                for sym in [0x0043_75c0, 0x0043_8680] {
                    let what =
                        format!("{label}: dispenser find {sym:#x} {hash} {name:#x} {root_link:#x}");
                    t.check(&what, sym, Conv::Thiscall(tree), &[key], |g| {
                        Out::U32(d::dispenser_find(g, tree, key))
                    });
                }
            }
        }
        // RegisterType and RegisterDefaultTypes over the real dispenser
        // (built here on the image; the snapshots' own)
        t.stub(types::ATEXIT, false, 1, 0, 0);
        // the second type registered (0x00c90760) has no name on the image
        // until its static constructor runs: give it one there
        let second = t.rebase(0x00c9_0760);
        if t.r32(second + 0x18) == 0 {
            let n = t.alloc(b"p1load_Second\0");
            t.w32(second + 0x18, n);
        }
        let what = format!("{label}: RegisterDefaultTypes");
        t.check(&what, 0x0043_9510, Conv::Cdecl, &[], |g| {
            let key = g.scratch();
            Out::U8(d::register_default_types(g, key))
        });
        // a new type, then the same again, then one without a name
        let fresh = t.zeros(0x30);
        let fname = t.alloc(b"p1load_FreshType\0");
        t.w32(fresh + 0x18, fname);
        let nameless = t.zeros(0x30);
        for ty in [fresh, fresh, nameless] {
            let what = format!("{label}: RegisterType {ty:#x}");
            t.check(&what, 0x0043_95b0, Conv::Cdecl, &[ty], |g| {
                let key = g.scratch();
                Out::U8(d::register_type(g, key, ty))
            });
        }
        // the real dispenser's lookups by name, through the ported hash
        let dispenser = t.rebase(types::DISPENSER);
        for n in [
            &b"float\0"[..],
            b"PUInt128\0",
            b"p1load_FreshType\0",
            b"nothing\0",
        ] {
            let name = t.alloc(n);
            let what = format!("{label}: real findByName {}", String::from_utf8_lossy(n));
            t.check(
                &what,
                0x0043_8630,
                Conv::Thiscall(dispenser),
                &[name],
                |g| {
                    let key = g.scratch();
                    Out::U32(d::find_by_name(g, 0x0043_8680, key, dispenser, name))
                },
            );
        }
        t.reset();
    }
}

#[test]
fn batch8_clusters_match_the_original() {
    for (label, mut t) in Twins::all() {
        // placeTables: the arena growth recorded, memset runs
        for (grow, extra, align, head) in [
            (0, 0, 0, 0x13),
            (0, 0x20, 16, 0x10),
            (0x55, 0x20, 0, 0x10),
            (0, 0, 8, 0),
        ] {
            t.stub(0x0044_2010, true, 3, 3, grow);
            let base = t.junk(0x200, 1);
            let arena = t.zeros(0x10);
            t.w32(arena + 4, base);
            let cluster = t.zeros(0x30);
            t.w32(cluster + 0x24, arena);
            let r = t.zeros(0x40);
            t.w32(r + 0x38, head);
            t.w32(r + 0x2c, if head == 0 { 0 } else { 5 });
            t.w32(r + 0x10, 0x77);
            let this = t.zeros(0x10);
            t.w32(this, cluster);
            t.w32(this + 8, r);
            let out = t.junk(8, 2);
            let what = format!("{label}: placeTables {grow} {extra} {align} {head}");
            t.check(
                &what,
                0x0057_9090,
                Conv::Thiscall(this),
                &[out, out + 4, extra, align],
                |g| Out::U32(c::place_tables(g, this, out, out + 4, extra, align)),
            );
            t.reset();
        }
        // the registry's forget: the mutex, the tree removal and the pool recorded
        for present in [true, false] {
            t.stub(0x0043_1850, true, 1, 1, 0);
            t.stub(0x0043_19d0, true, 0, 0, 0);
            t.stub(descriptor::FREE, false, 1, 0, 0);
            t.stub(0x0057_5000, true, 1, 1, 0);
            t.stub(0x0049_c980, true, 1, 1, 0);
            let registry = t.zeros(0x1400);
            let tree = registry + 0x1344;
            let cluster = 0x00c1_0000;
            t.w32(tree + 0xc, tree);
            if present {
                let n = t.junk(0x40, 3);
                t.w32(n, tree);
                t.w32(n + 4, tree);
                t.w32(n + 0xc, cluster);
                t.w32(tree + 0xc, n);
            }
            let what = format!("{label}: registry forget {present}");
            t.check(
                &what,
                0x0059_8c50,
                Conv::Thiscall(registry),
                &[cluster],
                |g| Out::U32(c::registry_forget(g, registry, cluster)),
            );
            assert_eq!(t.log_entries(), if present { 2 + 4 + 2 } else { 2 });
            t.reset();
        }
    }
}

// ---- batch 9 ----

#[test]
fn batch9_arenas_and_registry_match_the_original() {
    for (label, mut t) in Twins::all() {
        // createArena: the block allocation and the pool's hand-over recorded
        for (size, used, existing, block) in [
            (0, 0, 0, 1),
            (0x40, 0x41, 0, 1),
            (0x40, 0x40, 0, 1),
            (0x40, 8, 0x00ee_0000, 1),
            (0x41, 8, 0, 0),
            (0x41, 8, 0, 1),
        ] {
            let b = if block == 0 { 0 } else { t.junk(0x200, 1) };
            t.stub(descriptor::ALLOC, false, 2, 0, b);
            t.stub(0x0049_c7c0, true, 2, 2, 0);
            let cl = t.junk(0x40, 2);
            t.w32(cl + 0x24, existing);
            let what = format!("{label}: createArena {size:#x} {used:#x} {existing:#x} {b:#x}");
            t.check(
                &what,
                0x0044_2010,
                Conv::Thiscall(cl),
                &[size, used, 3],
                |g| Out::U32(c::create_arena(g, cl, size, used, 3)),
            );
            t.reset();
        }
        // the registry's add
        for (pooled, dup) in [(false, false), (true, false), (true, true)] {
            t.stub(0x0049_c8d0, true, 1, 1, 0);
            t.stub(0x0049_c980, true, 1, 1, 0);
            t.stub(0x0057_4980, true, 1, 1, 0);
            let reg = t.zeros(0x40);
            t.w32(reg + 0xc, reg);
            t.w32(reg + 0x20, 8);
            if pooled {
                let n1 = t.junk(0x40, 3);
                let n2 = t.junk(0x40, 4);
                t.w32(n1, n2);
                t.w32(n2, 0);
                t.w32(reg + 0x18, n1);
            }
            if dup {
                let old = t.zeros(0x40);
                t.w32(old, reg);
                t.w32(old + 4, reg);
                t.w32(old + 0xc, 0x00c1_0000);
                t.w32(reg + 0xc, old);
            }
            let key = t.zeros(4);
            t.w32(key, 0x00c1_0000);
            let record = t.junk(0x28, 5);
            let what = format!("{label}: registry add {pooled} {dup}");
            t.check(
                &what,
                0x0059_5e50,
                Conv::Thiscall(reg),
                &[key, record],
                |g| Out::U32(c::registry_add(g, reg, key, record)),
            );
            t.reset();
        }
        // the import-compared lookup: the CRT compare recorded, its result set
        for cmp in [0u32, 1, u32::MAX] {
            let stub = t.stub_code(0xc3, false, 2, 0, cmp);
            let iat = t.rebase(0x00b0_c398);
            t.w32(iat, stub);
            let tree = t.zeros(0x20);
            let node = t.zeros(0x20);
            t.w32(node + 4, tree);
            t.w32(node + 8, tree);
            t.w32(node + 0x10, 100);
            t.w32(node + 0x14, 0x00ab_0000);
            t.w32(tree + 0xc, node + 4);
            // children: hash 50 on the left, 150 on the right
            for (side, hash) in [(4, 50), (8, 150)] {
                let child = t.zeros(0x20);
                t.w32(child + 4, tree);
                t.w32(child + 8, tree);
                t.w32(child + 0x10, hash);
                t.w32(child + 0x14, 0x00ab_0010);
                t.w32(node + side, child + 4);
            }
            let key = t.zeros(8);
            for hash in [100, 50, 150, 70] {
                t.w32(key, hash);
                t.w32(key + 4, 0x00cd_0000);
                let what = format!("{label}: import find {cmp:#x} {hash}");
                t.check(&what, 0x0045_b8f0, Conv::Thiscall(tree), &[key], |g| {
                    Out::U32(d::import_find(g, tree, key))
                });
            }
        }
    }
}

// ---- continuation round, batch 10 (ffx_pload::p1_misc) ----

use ffx_pload::p1_misc::bridge as pm;

#[test]
fn p1c_abstract_descriptor_ctors_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(ffx_pload::types::ATEXIT, false, 1, 0, 0);
        for (sym, name, vtable, buffers) in [
            (0x0044_73f0, 0x00c9_1928, 0x00b0_fcac, 0x00c9_1930),
            (0x0044_ac10, 0x00c9_1b78, 0x00b1_00b4, 0x00c9_1b80),
        ] {
            let this = t.junk(0xc0, sym as u8);
            let what = format!("{label}: abstract descriptor ctor {sym:#x}");
            t.check(&what, sym, Conv::Thiscall(this), &[], |g| {
                Out::U32(pm::abstract_descriptor_ctor(g, this, name, vtable, buffers))
            });
        }
        t.reset();
    }
}

#[test]
fn p1c_instance_list_set_buffer_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // (elem size, flags, n, size, buffer, alloc result)
        let cases: [(u32, u8, u32, u32, u32, u32); 10] = [
            (1, 0, 0x101, 0x200, 0, 0x00c5_0000),
            (0x10, 0, 4, 0x3f, 0, 0x00c5_0000),
            (0x10, 0, 4, 0x40, 0, 0x00c5_0000),
            (0x10, 0, 4, 0x48, 0x00c6_0000, 0x00c5_0000),
            (0x10, 4, 2, 0x40, 0, 0x00c5_0000),
            (0x10, 4, 1, 0x40, 0, 0x00c5_0000),
            (0x10, 0, 0, 0, 0, 0x00c5_0000),
            (0x10, 0, 3, 0x40, 0, 0),
            (0, 4, 0x300, 0x10, 0x00c6_0000, 0),
            (0x10, 0xfb, 1, 0x10, 0, 0x00c5_0100),
        ];
        for (i, (elem, flags, n, size, buffer, block)) in cases.into_iter().enumerate() {
            t.stub(0x0057_64a0, true, 0, 0, 0x33);
            t.stub(descriptor::ALLOC, false, 2, 0, block);
            let desc = t.junk(0xa0, 7);
            t.w32(desc + 0x1c, elem);
            t.write(desc + 0x90, &[flags]);
            t.w32(desc + 0x20, 0x10);
            t.w32(desc + 0x8c, 0x0c);
            let list = t.junk(0x50, 8);
            t.w32(list + 0x2c, desc);
            let what = format!("{label}: set buffer case {i}");
            t.check(
                &what,
                0x0057_63b0,
                Conv::Thiscall(list),
                &[n, size, buffer],
                |g| Out::U32(pm::instance_list_set_buffer(g, list, n, size, buffer)),
            );
            t.reset();
        }
    }
}

#[test]
fn p1c_construct_asset_import_matches_the_original() {
    for (label, mut t) in Twins::all() {
        for def in [None, Some(0), Some(0x00c4_4400)] {
            // PBase's descriptor is set up only in a running game
            if def.is_none() && !t.snapshot {
                continue;
            }
            if let Some(d) = def {
                t.stub(0x0043_cbb0, true, 0, 0, d);
            }
            t.stub(0x0045_e1a0, true, 0, 0, 0x0012_3412);
            let p = t.junk(0x40, 9);
            for at in [0, p] {
                let what = format!("{label}: construct import {def:x?} {at:#x}");
                t.check(&what, 0x0045_c9e0, Conv::Cdecl, &[at], |g| {
                    Out::U32(pm::construct_asset_import(g, at))
                });
            }
            t.reset();
        }
    }
}

/// UTF-8 samples, valid and not (bare continuation bytes, 5- and 6-byte
/// leads, a lead before the terminator).
const P1C_NAMES: [&[u8]; 9] = [
    b"PNode\0",
    b"\0",
    b"m_elements\0",
    "\u{e9}t\u{e9} \u{65e5}\u{672c} \u{1f600}\0".as_bytes(),
    b"\x80\xbf\xc0\x80\0",
    b"\xf8\x88\x80\x80\x80x\0",
    b"\xfc\x84\x80\x80\x80\x80y\0",
    b"\xffabcdefg\0",
    b"ab\xe0\0\0\0\0",
];

#[test]
fn p1c_utf8_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let cursor = t.zeros(4);
        for name in P1C_NAMES {
            let s = t.alloc(name);
            for at in 0..name.len().min(4) as u32 {
                t.w32(cursor, s + at);
                let what = format!("{label}: utf8 next {name:x?} +{at}");
                t.check(&what, 0x0045_d640, Conv::Cdecl, &[cursor], |g| {
                    Out::U32(pm::utf8_next(g, cursor))
                });
            }
            for seed in [0, 0x7b5, u32::MAX] {
                let what = format!("{label}: utf8 hash {name:x?} {seed:#x}");
                t.check(&what, 0x0045_d520, Conv::Cdecl, &[s, seed], |g| {
                    Out::U32(pm::utf8_hash(g, s, seed))
                });
            }
        }
        let what = format!("{label}: utf8 hash, no name");
        t.check(&what, 0x0045_d520, Conv::Cdecl, &[0, 77], |g| {
            Out::U32(pm::utf8_hash(g, 0, 77))
        });
    }
}

#[test]
fn p1c_releases_match_the_original() {
    for (label, mut t) in Twins::all() {
        // 0x00605660: objects, then the two arrays in all their states
        let states = [
            (0, 0x00c7_0000),
            (3, 0),
            (3, 0x00c7_0000),
            (0x8000_0003, 0x00c7_0000),
        ];
        for round in 0..4u32 {
            t.stub(descriptor::FREE, false, 1, 0, 0x44);
            let this = t.junk(0x100, round as u8);
            for (k, at) in [0x54u32, 0x58, 0x5c].into_iter().enumerate() {
                let obj = if (round + k as u32).is_multiple_of(2) {
                    t.object(&[1])
                } else {
                    0
                };
                t.w32(this + at, obj);
            }
            let (c1, d1) = states[round as usize];
            let (c2, d2) = states[3 - round as usize];
            t.w32(this + 0x80, c1);
            t.w32(this + 0x84, d1);
            t.w32(this + 0x88, c2);
            t.w32(this + 0x8c, d2);
            let what = format!("{label}: release parts {round}");
            t.check(&what, 0x0060_5660, Conv::Thiscall(this), &[], |g| {
                Out::U32(pm::release_parts(g, this))
            });
            t.reset();
        }
        // 0x00479930 and 0x00577f60
        let states = [
            (0, 0),
            (0, 0x00c7_0000),
            (1, 0x00c7_0000),
            (2, 0x00c7_0000),
            (0x8000_0002, 0x00c7_0000),
            (0x8000_0000, 0),
        ];
        for (i, &(count, data)) in states.iter().enumerate() {
            t.stub(descriptor::FREE, false, 1, 0, 0x44);
            let this = t.junk(0x40, i as u8);
            for at in [4, 0xc, 0x14] {
                t.w32(this + at, count);
                t.w32(this + at + 4, data);
            }
            let what = format!("{label}: release three {i}");
            t.check(&what, 0x0047_9930, Conv::Thiscall(this), &[], |g| {
                pm::release_three(g, this);
                Out::None
            });
            let this = t.junk(0x40, i as u8);
            for (k, at) in [0x10, 0x18, 0x20].into_iter().enumerate() {
                let (c, d) = states[(i + k) % states.len()];
                t.w32(this + at, c);
                t.w32(this + at + 4, d);
            }
            let what = format!("{label}: release three inline {i}");
            t.check(&what, 0x0057_7f60, Conv::Thiscall(this), &[], |g| {
                pm::release_three_inline(g, this);
                Out::None
            });
            t.reset();
        }
    }
}

#[test]
fn p1c_inline_array_resize_matches_the_original() {
    for (label, mut t) in Twins::all() {
        for count in [0u32, 1, 3, 0x8000_0001, 0x8000_0003] {
            for n in [0u32, 1, 2, 3, 5] {
                for block in [0u32, 1] {
                    let target = if block == 0 { 0 } else { t.junk(0x20, 3) };
                    t.stub(descriptor::ALLOC, false, 2, 0, target);
                    t.stub(descriptor::FREE, false, 1, 0, 0x44);
                    let array = t.junk(8, 1);
                    t.w32(array, count);
                    t.w32(array + 4, 0x00c8_0000);
                    let what = format!("{label}: inline resize {count:#x} -> {n} {block}");
                    t.check(&what, 0x0057_ccf0, Conv::Thiscall(array), &[n], |g| {
                        Out::U32(pm::inline_array_resize(g, array, n))
                    });
                    t.reset();
                }
            }
        }
    }
}

#[test]
fn p1c_copy_strings_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let a = t.alloc(b"first\0");
        let b = t.alloc(b"\0");
        let src = t.zeros(16);
        t.w32(src, a);
        t.w32(src + 4, 0);
        t.w32(src + 8, b | 1);
        t.w32(src + 12, a + 1);
        for alloc in [0u32, 1] {
            let block = if alloc == 0 { 0 } else { t.zeros(0x20) };
            t.stub(descriptor::ALLOC, false, 2, 0, block);
            for dst in [0u32, 1] {
                let dst = if dst == 0 { 0 } else { t.junk(16, 4) };
                // a dst of 0 moves on to address 4: one element at most
                for n in if dst == 0 { &[0u32, 1][..] } else { &[0, 1, 4] } {
                    let n = *n;
                    let what = format!("{label}: copy strings {alloc} {dst:#x} {n}");
                    t.check(&what, 0x0044_a470, Conv::Cdecl, &[dst, src, n], |g| {
                        pm::copy_strings(g, dst, src, n);
                        Out::None
                    });
                }
            }
            t.reset();
        }
    }
}

#[test]
fn p1c_data_member_ctors_match_the_original() {
    for (label, mut t) in Twins::all() {
        // slot 1 as the constructors call it: the original (0), or a
        // stand-in returning a type of kind 2 or 3
        let slot1_word = t.rebase(0x00b3_09dc);
        let slot1 = t.r32(slot1_word);
        for kind_of in [None, Some(2u32), Some(3)] {
            let r = if let Some(k) = kind_of {
                let ty = t.zeros(0x40);
                t.w32(ty + 0x30, k);
                let s = t.stub_code(0x51, true, 0, 0, ty);
                t.w32(slot1_word, s);
                ty
            } else {
                t.w32(slot1_word, slot1);
                0
            };
            for flags in [0u32, 2, 0x40, 0x42, 0x80, 0x3c] {
                for kind in [0u32, 2] {
                    let desc = t.object(&[0, 0, 0, 0, 0, 0, 0, 0, 2]);
                    let ty = t.junk(0x40, 5);
                    t.w32(ty + 0x1c, 0x24);
                    let name = t.alloc(b"m_value\0");
                    let args = [desc, ty, name, 0x18, flags, kind];
                    let this = t.junk(0x40, 6);
                    let what = format!("{label}: data member {r:#x} {flags:#x} {kind}");
                    t.check(&what, 0x0057_52a0, Conv::Thiscall(this), &args, |g| {
                        Out::U32(pm::data_member_ctor(g, this, args))
                    });
                    let this = t.junk(0x40, 7);
                    let args = [desc, ty, name, 0x18, flags, 0x00c9_9990 + kind];
                    let what = format!("{label}: enum member {r:#x} {flags:#x} {kind}");
                    t.check(&what, 0x0057_5370, Conv::Thiscall(this), &args, |g| {
                        Out::U32(pm::enum_member_ctor(g, this, args))
                    });
                    t.reset();
                }
            }
        }
        t.w32(slot1_word, slot1);
        // the dynamic member: names copied (allocator recorded) or not
        let marker = t.r32(t.rebase(0x00c0_b238));
        let named = t.alloc(b"m_dynamic\0");
        for name in [named, marker, 0] {
            for flags in [0u32, 1, 3] {
                let block = t.zeros(0x20);
                t.stub(descriptor::ALLOC, false, 2, 0, block | (flags & 1));
                let desc = t.object(&[0, 0, 0, 0, 0, 0, 0, 0, 2]);
                let ty = t.junk(0x40, 5);
                let args = [desc, ty, name, 0x1c, flags, 1];
                let this = t.junk(0x40, 8);
                let what = format!("{label}: dynamic member {name:#x} {flags}");
                t.check(&what, 0x0044_61e0, Conv::Thiscall(this), &args, |g| {
                    Out::U32(pm::dynamic_member_ctor(g, this, args))
                });
                t.reset();
            }
        }
        // its construction in place
        for def in [None, Some(0), Some(0x18), Some(0x00c4_5518)] {
            if def.is_none() && !t.snapshot {
                continue;
            }
            if let Some(d) = def {
                t.stub(0x0043_cbb0, true, 0, 0, d);
            }
            t.stub(0x0044_61e0, true, 6, 6, 0x0055_6601);
            for p in [0, 0x00c4_7700] {
                let what = format!("{label}: construct dynamic member {def:x?} {p:#x}");
                t.check(&what, 0x0044_65c0, Conv::Cdecl, &[p], |g| {
                    Out::U32(pm::construct_dynamic_member(g, p))
                });
            }
            t.reset();
        }
    }
}

/// A namespace for the lookups: its descriptors (by name) and children.
fn p1c_namespace(t: &mut Twins, names: &[&[u8]], children: &[u32]) -> (u32, Vec<u32>) {
    let ns = t.zeros(0x20);
    for head in [ns, ns + 8, ns + 0x14] {
        t.w32(head, head);
        t.w32(head + 4, head);
    }
    let mut descs = Vec::new();
    let mut prev = ns + 8;
    for n in names {
        let d = t.junk(0xa0, 3);
        let name = if n.is_empty() { 0 } else { t.alloc(n) };
        t.w32(d + 0x18, name);
        t.w32(prev, d + 0x2c);
        t.w32(d + 0x30, prev);
        t.w32(d + 0x2c, ns + 8);
        t.w32(ns + 8 + 4, d + 0x2c);
        prev = d + 0x2c;
        descs.push(d);
    }
    let mut prev = ns + 0x14;
    for &c in children {
        t.w32(prev, c);
        t.w32(c + 4, prev);
        t.w32(c, ns + 0x14);
        t.w32(ns + 0x18, c);
        prev = c;
    }
    (ns, descs)
}

#[test]
fn p1c_namespace_lookups_match_the_original() {
    for (label, mut t) in Twins::all() {
        let (leaf, _) = p1c_namespace(&mut t, &[b"Deep\0", b"PNode\0"], &[]);
        let (mid, _) = p1c_namespace(&mut t, &[b"Mid\0"], &[leaf]);
        let (other, _) = p1c_namespace(&mut t, &[], &[]);
        let (ns, _) = p1c_namespace(&mut t, &[b"Alpha\0", b"", b"Beta\0"], &[other, mid]);
        // a list whose last link is 0x2c (a descriptor at address 0)
        let (stub_ns, stub_descs) = p1c_namespace(&mut t, &[b"Alpha\0"], &[]);
        t.w32(stub_descs[0] + 0x2c, 0x2c);
        for q in [
            &b"Alpha\0"[..],
            b"Beta\0",
            b"Mid\0",
            b"Deep\0",
            b"PNode\0",
            b"Gamma\0",
            b"\0",
        ] {
            let name = t.alloc(q);
            for space in [ns, mid, other, stub_ns] {
                let what = format!("{label}: find {} in {space:#x}", String::from_utf8_lossy(q));
                t.check(&what, 0x0043_e950, Conv::Thiscall(space), &[name], |g| {
                    Out::U32(pm::namespace_find(g, space, name))
                });
            }
        }
        let what = format!("{label}: find, no name");
        t.check(&what, 0x0043_e950, Conv::Thiscall(ns), &[0], |g| {
            Out::U32(pm::namespace_find(g, ns, 0))
        });
        if t.snapshot {
            // the running game's global namespace
            let gn = t.rebase(descriptor::GLOBAL_NAMESPACE);
            let global = t.a.call(gn, Conv::Cdecl, &[]).unwrap().eax;
            assert_eq!(t.b.call(gn, Conv::Cdecl, &[]).unwrap().eax, global);
            t.diff("global namespace");
            for q in [
                &b"PNode\0"[..],
                b"PMatrix4\0",
                b"PTexture2D\0",
                b"nothing\0",
            ] {
                let name = t.alloc(q);
                let what = format!("{label}: real find {}", String::from_utf8_lossy(q));
                t.check(&what, 0x0043_e950, Conv::Thiscall(global), &[name], |g| {
                    Out::U32(pm::namespace_find(g, global, name))
                });
            }
        }
        // add: duplicates, states, parents in and out of the namespace,
        // first descriptors (joining the global namespace's children)
        t.stub(ffx_pload::types::ATEXIT, false, 1, 0, 0);
        let gn = t.rebase(descriptor::GLOBAL_NAMESPACE);
        let global = t.a.call(gn, Conv::Cdecl, &[]).unwrap().eax;
        assert_eq!(t.b.call(gn, Conv::Cdecl, &[]).unwrap().eax, global);
        t.diff("global namespace");
        for state in [0u32, 1, 2] {
            for case in 0..7u32 {
                let global_first = t.r32(global + 8);
                if case == 6 && global_first != global + 8 && global_first != 0 {
                    // the snapshots' global namespace has descriptors
                    continue;
                }
                t.stub(ffx_pload::p1_misc::PRINTF, false, 3, 0, 0);
                t.stub(0x0043_d6e0, true, 0, 0, 0);
                let names: &[&[u8]] = if case == 4 {
                    &[]
                } else {
                    &[b"Alpha\0", b"Beta\0"]
                };
                let (mut space, descs) = p1c_namespace(&mut t, names, &[]);
                if case == 6 {
                    space = global;
                    if global_first == 0 {
                        t.w32(global + 8, global + 8);
                        t.w32(global + 0xc, global + 8);
                    }
                }
                t.w32(space + 0x10, state);
                let desc = if case == 0 {
                    descs[1]
                } else {
                    let d = t.junk(0xa0, 4);
                    let n = t.alloc(match case {
                        1 => b"Alpha\0",
                        6 => b"Globe\0",
                        _ => b"Fresh\0",
                    });
                    t.w32(d + 0x18, n);
                    t.w32(d + 0x2c, d + 0x2c);
                    t.w32(d + 0x30, d + 0x2c);
                    let parent = match case {
                        2 | 5 => {
                            t.w32(descs[0] + 0x3c, space);
                            descs[0]
                        }
                        3 => descs[0],
                        _ => 0,
                    };
                    t.w32(d + 0x40, parent);
                    if case == 5 {
                        // the namespace's head reads 0: "empty"
                        t.w32(space + 8, 0);
                    }
                    d
                };
                let what = format!("{label}: add {state} {case}");
                t.check(&what, 0x0043_e660, Conv::Thiscall(space), &[desc], |g| {
                    pm::namespace_add(g, space, desc);
                    Out::None
                });
                t.reset();
            }
        }
    }
}

#[test]
fn p1c_import_insert_matches_the_original() {
    for (label, mut t) in Twins::all() {
        for cmp in [0u32, 1, u32::MAX] {
            let stub = t.stub_code(0xc3, false, 2, 0, cmp);
            let iat = t.rebase(0x00b0_c398);
            t.w32(iat, stub);
            for empty in [true, false] {
                for (hash, color) in [(100u32, 0u32), (50, 1), (150, 0), (70, 1), (170, 0)] {
                    let tree = t.zeros(0x20);
                    t.w32(tree + 0xc, tree);
                    if !empty {
                        let node = t.zeros(0x20);
                        t.w32(node + 4, tree);
                        t.w32(node + 8, tree);
                        t.w32(node + 0xc, tree | 1);
                        t.w32(node + 0x10, 100);
                        t.w32(node + 0x14, 0x00ab_0000);
                        t.w32(tree + 0xc, node + 4);
                        for (side, h) in [(4, 50), (8, 150)] {
                            let child = t.zeros(0x20);
                            t.w32(child + 4, tree);
                            t.w32(child + 8, tree);
                            t.w32(child + 0x10, h);
                            t.w32(child + 0x14, 0x00ab_0010);
                            t.w32(node + side, child + 4);
                        }
                    }
                    let import = t.junk(0x30, 2);
                    t.w32(import + 0xc, color | 0x00c1_2340);
                    t.w32(import + 0x10, hash);
                    t.w32(import + 0x14, 0x00cd_0000);
                    let what = format!("{label}: import insert {cmp:#x} {empty} {hash}");
                    t.check(&what, 0x0045_b9d0, Conv::Thiscall(tree), &[import], |g| {
                        Out::U32(pm::import_insert(g, tree, import))
                    });
                }
            }
        }
    }
}

#[test]
fn p1c_apply_default_buffer_matches_the_original() {
    for (label, mut t) in Twins::all() {
        for flags in [0x1000_0000u32, 0, 0x4000_0000, 0x8000_0000, 0xc000_0002] {
            for (p, n, bufs, masked) in [
                (0u32, 2u32, true, true),
                (1, 0, true, true),
                (1, 3, true, true),
                (1, 2, false, true),
                (1, 2, true, false),
            ] {
                t.stub(0x0043_c420, true, 0, 0, 0x0042_0420);
                t.stub(0x0043_c630, true, 0, 0, 0x0063_0630);
                let obj = t.object(&[0; 13]);
                let desc = t.zeros(0xa0);
                let vt = t.r32(obj);
                t.w32(desc, vt);
                t.w32(desc + 0x90, flags);
                t.w32(desc + 0x1c, 6);
                t.w32(desc + 0x8c, 2);
                let default = t.junk(8, 1);
                let mask = t.junk(8, 2);
                t.w32(desc + 0x64, if bufs { default } else { 0 });
                t.w32(desc + 0x68, if masked { mask } else { 0 });
                let inst = t.junk(0x40, 3);
                let at = if p == 0 { 0 } else { inst + 2 };
                let what = format!("{label}: apply default {flags:#x} {p} {n} {bufs} {masked}");
                t.check(&what, 0x0043_c160, Conv::Thiscall(desc), &[at, n], |g| {
                    Out::U32(pm::apply_default_buffer(g, desc, at, n))
                });
                t.reset();
            }
        }
    }
}

#[test]
fn p1c_blend2_matches_the_original() {
    let mut rng = Rng(0x0b1e_0d02);
    let specials = [
        0.0f32,
        -0.0,
        1.0,
        -2.5,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        1e-40,
        3.4e38,
    ];
    for (label, mut t) in Twins::all() {
        for round in 0..32u32 {
            let mut f = || -> f32 {
                let r = rng.next();
                if r.trailing_zeros() >= 3 {
                    specials[(r >> 3) as usize % specials.len()]
                } else {
                    f32::from_bits((r & 0xbfff_ffff) | 0x3000_0000)
                }
            };
            let n = 1 + round % 9;
            let nans = [
                f32::NAN,
                -f32::NAN,
                f32::from_bits(0x7fc0_0001),
                f32::from_bits(0xffc0_1000),
            ];
            let w = t.zeros(8);
            let (w0, w1) = if round >= 24 { (1.5, -0.5) } else { (f(), f()) };
            t.write(w, &w0.to_le_bytes());
            t.write(w + 4, &w1.to_le_bytes());
            let a = t.zeros(4 * n as usize);
            let b = t.zeros(4 * n as usize);
            for i in 0..n {
                let (mut x, mut y) = (f(), f());
                if round >= 24 {
                    x = nans[(round + i) as usize % 4];
                    y = nans[(round + 2 * i + 1) as usize % 4];
                }
                t.write(a + 4 * i, &x.to_le_bytes());
                t.write(b + 4 * i, &y.to_le_bytes());
            }
            let arrays = t.zeros(8);
            t.w32(arrays, a);
            t.w32(arrays + 4, b);
            let dst = t.junk(4 * n as usize + 4, 1);
            let out = t.zeros(4);
            t.w32(out, dst);
            let what = format!("{label}: blend2 {round}");
            t.check(&what, 0x005c_b870, Conv::Cdecl, &[w, n, arrays, out], |g| {
                pm::blend2(g, w, n, arrays, out);
                Out::None
            });
        }
    }
}

#[test]
fn p1c_release_group_resources_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let mut slots = vec![0u32; 20];
        slots.push(1);
        let renderer = t.object(&slots);
        let stranger = t.zeros(0x60);
        for owner in [0, renderer] {
            let this = t.junk(0x60, 1);
            t.w32(this + 0x54, owner);
            let mut prev_group = 0u32;
            let mut first = 0u32;
            for g in 0..3u32 {
                let group = t.junk(0x10, g as u8);
                t.w32(group, 0);
                if prev_group == 0 {
                    first = group;
                } else {
                    t.w32(prev_group, group);
                }
                prev_group = group;
                let mut head = 0u32;
                for e in 0..(g * 3) {
                    let entry = t.junk(0x100, e as u8);
                    t.w32(entry, head);
                    head = entry;
                    let obj = if e % 4 == 0 {
                        0
                    } else {
                        let o = t.zeros(0x10);
                        let o8 = match e % 4 {
                            1 => 0,
                            2 => stranger,
                            _ => {
                                let x = t.zeros(0x60);
                                t.w32(x + 0x54, renderer);
                                x
                            }
                        };
                        t.w32(o + 8, o8);
                        o
                    };
                    t.w32(entry + 0xb0, obj);
                    t.w32(entry + 0xdc, if e == 3 { 0 } else { 0x00d0_0000 + e });
                }
                t.w32(group + 4, head);
            }
            t.w32(this + 0x48, first);
            let what = format!("{label}: release group resources {owner:#x}");
            t.check(&what, 0x0060_19b0, Conv::Thiscall(this), &[], |g| {
                Out::U32(pm::release_group_resources(g, this))
            });
            t.reset();
        }
    }
}

#[test]
fn p1c_registry_add_locked_matches_the_original() {
    for (label, mut t) in Twins::all() {
        for dup in [false, true] {
            t.stub(0x0043_1840, true, 0, 0, 0x40);
            t.stub(0x0043_19d0, true, 0, 0, 0x9d);
            t.stub(0x0049_c8d0, true, 1, 1, 0);
            t.stub(0x0049_c980, true, 1, 1, 0);
            t.stub(0x0057_4980, true, 1, 1, 0);
            let owner = t.junk(0x1400, 2);
            let reg = owner + 0x1344;
            t.write(reg, &[0; 0x30]);
            t.w32(reg + 0xc, reg);
            t.w32(reg + 0x20, 8);
            let n1 = t.junk(0x40, 3);
            t.w32(n1, 0);
            t.w32(reg + 0x18, n1);
            if dup {
                let old = t.zeros(0x40);
                t.w32(old, reg);
                t.w32(old + 4, reg);
                t.w32(old + 0xc, 0x00c1_0001);
                t.w32(reg + 0xc, old);
            }
            let args: [u32; 10] = std::array::from_fn(|i| 0x00c1_0000 + i as u32);
            let what = format!("{label}: registry add locked {dup}");
            t.check(&what, 0x0058_c010, Conv::Thiscall(owner), &args, |g| {
                let frame = g.scratch();
                Out::U32(pm::registry_add_locked(g, frame, owner, args))
            });
            t.reset();
        }
    }
}

#[test]
fn p1c_dynamic_refresh_matches_the_original() {
    for (label, mut t) in Twins::all() {
        for (size, had, parent) in [
            (0u32, 4u32, 0u8),
            (4, 4, 0),
            (10, 4, 1),
            (10, 0x8000_0004, 2),
            (8, 0, 1),
        ] {
            for default in [0u32, 1] {
                if default == 0 && size > had & 0x7fff_ffff {
                    // the tails' memset would write at address 4
                    continue;
                }
                t.stub(0x0044_a080, true, 1, 1, 0x0a08_0000);
                t.stub(0x0043_cb80, true, 0, 0, 0x0cb8_0000);
                let desc = t.junk(0xc0, 2);
                t.w32(desc + 0x1c, size);
                t.w32(desc + 0x9c, had);
                for at in [0xa0, 0xa8, 0xb0] {
                    let arr = t.junk(0x20, at as u8);
                    t.w32(desc + at, arr);
                }
                if default == 0 {
                    t.w32(desc + 0xa0, 0);
                }
                t.w32(desc + 0x8c, 4);
                t.w32(desc + 0x90, 0x8000_0123);
                let target = t.rebase(0x00c9_1c18);
                let p = match parent {
                    0 => 0,
                    1 => target,
                    _ => {
                        let mid = t.zeros(0x60);
                        t.w32(mid + 0x40, target);
                        mid
                    }
                };
                t.w32(desc + 0x40, p);
                let what = format!("{label}: dynamic refresh {size} {had:#x} {parent} {default}");
                t.check(&what, 0x0044_9e40, Conv::Thiscall(desc), &[], |g| {
                    pm::dynamic_refresh(g, desc);
                    Out::None
                });
                t.reset();
            }
        }
    }
}

// ---- continuation round, batch 11 (ffx_pload::p1_types, p1_loader) ----

use ffx_pload::p1_loader::bridge as pl;
use ffx_pload::p1_types::{self as pty, bridge as pt};

#[test]
fn p1c_descriptor_ctors_match_the_original() {
    for (label, mut t) in Twins::all() {
        for round in 0..3u32 {
            let args = [
                0x00c1_0000 + round,
                0x00c2_0000,
                0x40 + round,
                8,
                0x00c3_0000,
                7,
            ];
            let this = t.junk(0xb0, round as u8);
            let what = format!("{label}: descriptor ctor 6 {round}");
            t.check(&what, 0x0043_b0a0, Conv::Thiscall(this), &args, |g| {
                Out::U32(pt::descriptor_ctor(g, this, args))
            });
            let this = t.junk(0xb0, round as u8 + 1);
            let what = format!("{label}: descriptor ctor 4 {round}");
            t.check(&what, 0x0043_afb0, Conv::Thiscall(this), &args[..4], |g| {
                Out::U32(pt::descriptor_ctor(
                    g,
                    this,
                    [args[0], args[1], args[2], args[3], 0, 0],
                ))
            });
        }
        // the dynamic descriptor: named or not, allocation recorded
        for name in [0u32, 1] {
            let block = t.zeros(0x20);
            t.stub(descriptor::ALLOC, false, 2, 0, block | 1);
            t.stub(0x0043_d6e0, true, 0, 0, 0);
            t.stub(0x0044_9e40, true, 0, 0, 0);
            let base = t.junk(0xa0, 4);
            t.w32(base + 0x1c, 0x24);
            t.w32(base + 0x20, 0x10);
            let name = if name == 0 {
                0
            } else {
                t.alloc(b"PDynamicThing\0")
            };
            let this = t.junk(0xc0, 5);
            let what = format!("{label}: dynamic descriptor ctor {name:#x}");
            t.check(
                &what,
                0x0044_7690,
                Conv::Thiscall(this),
                &[0x00c1_0000, name, base],
                |g| Out::U32(pt::dynamic_ctor(g, this, 0x00c1_0000, name, base)),
            );
            t.reset();
        }
    }
}

#[test]
fn p1c_dispenser_insert_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let names: Vec<u32> = [
            &b"Mid\0"[..],
            b"Alpha\0",
            b"Zulu\0",
            b"Mid\0",
            b"Mie\0",
            b"Mi\0",
        ]
        .iter()
        .map(|n| t.alloc(n))
        .collect();
        for empty in [true, false] {
            for (k, hash) in [
                (0usize, 100u32),
                (3, 100),
                (4, 100),
                (5, 100),
                (1, 50),
                (2, 150),
                (1, 70),
            ] {
                let tree = t.zeros(0x20);
                t.w32(tree + 0xc, tree);
                if !empty {
                    let node = t.zeros(0x20);
                    t.w32(node + 4, tree);
                    t.w32(node + 8, tree);
                    t.w32(node + 0xc, tree | 1);
                    t.w32(node + 0x10, 100);
                    t.w32(node + 0x14, names[0]);
                    t.w32(tree + 0xc, node + 4);
                    for (side, h, n) in [(4, 50, names[1]), (8, 150, names[2])] {
                        let child = t.zeros(0x20);
                        t.w32(child + 4, tree);
                        t.w32(child + 8, tree);
                        t.w32(child + 0x10, h);
                        t.w32(child + 0x14, n);
                        t.w32(node + side, child + 4);
                    }
                }
                let ty = t.junk(0x30, 2);
                t.w32(ty + 0xc, 0x00c1_2341);
                t.w32(ty + 0x10, hash);
                t.w32(ty + 0x14, names[k]);
                let what = format!("{label}: dispenser insert {empty} {k} {hash}");
                t.check(&what, 0x0043_8780, Conv::Thiscall(tree), &[ty], |g| {
                    Out::U32(pm::dispenser_insert(g, tree, ty))
                });
            }
        }
    }
}

#[test]
fn p1c_wrapper_descriptors_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(ffx_pload::types::ATEXIT, false, 1, 0, 0);
        for w in pty::WRAPPERS {
            // the first call (on the image), then a bound one
            for round in 0..2 {
                let what = format!("{label}: wrapper {:#x} {round}", w.getter);
                t.check(&what, w.getter, Conv::Cdecl, &[], |g| {
                    Out::U32(pt::wrapper_descriptor(g, &w))
                });
            }
            // not bound again (guard bits kept): addToNamespace once more
            let desc = t.rebase(w.desc);
            t.write(desc + 0x98, &[0]);
            let what = format!("{label}: wrapper {:#x} rebound", w.getter);
            t.check(&what, w.getter, Conv::Cdecl, &[], |g| {
                Out::U32(pt::wrapper_descriptor(g, &w))
            });
        }
        t.reset();
    }
}

/// A cluster for the instance-list tests: its list head, arenas and
/// pool; `lists` instance lists of `desc` linked in.
fn p1c_cluster(t: &mut Twins, lists: &[(u32, u32, u32)]) -> (u32, Vec<u32>) {
    let cluster = t.junk(0x50, 1);
    let head = cluster + 0x1c;
    t.w32(head, head);
    t.w32(head + 4, head);
    t.w32(cluster + 0x24, 0);
    t.w32(cluster + 0x40, 0);
    let mut out = Vec::new();
    for &(desc, buffer, count) in lists {
        let list = t.junk(0x50, 2);
        t.w32(list + 0x2c, desc);
        t.w32(list + 0x34, buffer);
        t.w32(list + 0x3c, count);
        t.w32(list + 0x40, 0);
        let tail = t.r32(head + 4);
        t.w32(list, head);
        t.w32(list + 4, tail);
        t.w32(tail, list);
        t.w32(head + 4, list);
        out.push(list);
    }
    (cluster, out)
}

#[test]
fn p1c_allocate_instance_list_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // (size, n, flags, pool state, buffer ok, world)
        let cases: [(u32, u32, u8, u32, u32, bool); 9] = [
            (0, 4, 0, 1, 1, false),
            (0x40, 0, 0, 1, 1, false),
            (0x40, 4, 2, 1, 1, false),
            (0x3f, 4, 0, 1, 1, false),
            (0x40, 4, 0, 1, 1, false),
            (0x40, 4, 0, 1, 1, true),
            (0x40, 4, 0, 0, 1, false),
            (0x40, 4, 0, 2, 1, false),
            (0x48, 4, 0, 1, 0, true),
        ];
        for (i, (size, n, flags, pool, ok, world)) in cases.into_iter().enumerate() {
            let list = t.junk(0x50, 5);
            // the reports push 3 and 6 words
            t.stub(
                ffx_pload::p1_misc::PRINTF,
                false,
                if i == 3 { 6 } else { 3 },
                0,
                0,
            );
            t.stub(0x0049_c8d0, true, 1, 1, 0);
            t.stub(0x0057_6270, true, 2, 2, list);
            t.stub(0x0044_0cf0, true, 2, 2, 0x00c6_6000);
            t.stub(0x0057_63b0, true, 3, 3, 0x0012_3400 | ok);
            t.stub(0x0043_f3c0, true, 1, 1, 0);
            t.stub(0x0044_3320, true, 1, 1, 0);
            let desc = t.junk(0xa0, 3);
            t.write(desc + 0x90, &[flags]);
            t.w32(desc + 0x1c, 0x10);
            let (cluster, _) = p1c_cluster(&mut t, &[(desc, 0x00c7_0000, 1)]);
            let block = t.junk(0x10, 4);
            t.w32(cluster + 0x28, if pool == 1 { block } else { 0 });
            t.w32(cluster + 0x30, 0x20);
            if pool == 2 {
                // the pool grows: a stand-in that puts the block in it
                // (mov dword [ecx], block; ret 4)
                let mut code = vec![0xc7, 0x01];
                code.extend(block.to_le_bytes());
                code.extend([0xc2, 0x04, 0x00]);
                let at = t.a.alloc_code(&code).unwrap();
                assert_eq!(t.b.alloc_code(&code).unwrap(), at);
                let sym = t.rebase(0x0049_c8d0);
                let mut j = vec![0xe9];
                j.extend(at.wrapping_sub(sym + 5).to_le_bytes());
                t.write(sym, &j);
            }
            t.w32(cluster + 0x40, if world { 0x00c8_0000 } else { 0 });
            let args = [size, n, desc, 8];
            let what = format!("{label}: allocate instance list case {i}");
            t.check(&what, 0x0044_0ba0, Conv::Thiscall(cluster), &args, |g| {
                Out::U32(pl::allocate_instance_list(g, cluster, size, n, desc, 8))
            });
            t.reset();
        }
    }
}

#[test]
fn p1c_delete_object_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // arena: 0 none, 1 two arenas holding the buffer, 2 one starting
        // at the buffer, 3 one ending at it
        for (obj, err, stays, flag, arena) in [
            (0x00c0_0000u32, 0u32, 0u32, 0u8, 0u8),
            (0x00c7_0010, 0, 0, 0, 0),
            (0x00c7_0000, 0, 0, 0, 0),
            (0x00c7_0010, 9, 0, 0, 0),
            (0x00c7_0010, 0, 1, 0, 0),
            (0x00c7_0010, 0, 0, 0, 1),
            (0x00c7_0010, 0, 0, 1, 1),
            (0x00c9_0020, 0, 0, 0, 0),
            (0x00c7_0040, 0, 0, 0, 0),
            (0x00c7_003f, 0, 0, 0, 1),
            (0x00c7_0010, 0, 0, 0, 2),
            (0x00c7_0010, 0, 0, 0, 3),
        ] {
            t.stub(0x0057_6530, true, 1, 1, err);
            t.stub(0x0044_3c20, true, 1, 1, 0);
            t.stub(0x0057_64a0, true, 0, 0, 0);
            t.stub(0x0049_c5b0, true, 0, 0, 0);
            t.stub(0x0049_c980, true, 1, 1, 0);
            let d1 = t.junk(0xa0, 3);
            t.w32(d1 + 0x1c, 0x10);
            t.write(d1 + 0x90, &[flag]);
            let d2 = t.junk(0xa0, 4);
            t.w32(d2 + 0x1c, 0x8);
            let (cluster, lists) = p1c_cluster(
                &mut t,
                &[
                    (d2, 0x00c6_0000, 2),
                    (d1, 0x00c7_0000, 4),
                    (d2, 0x00c9_0000, 8),
                ],
            );
            t.w32(lists[1] + 0x40, stays);
            t.w32(cluster + 0x40, 0x00c8_0000);
            if arena != 0 {
                let a1 = t.zeros(0x10);
                let a2 = t.zeros(0x10);
                t.w32(a1, a2);
                t.w32(a1 + 4, 0x00c5_0000);
                t.w32(a1 + 0xc, 0x100);
                let (base, size) = match arena {
                    1 => (0x00c6_ff00, 0x200),
                    2 => (0x00c7_0000, 0x10),
                    _ => (0x00c6_fff0, 0x10),
                };
                t.w32(a2 + 4, base);
                t.w32(a2 + 0xc, size);
                t.w32(cluster + 0x24, a1);
            }
            let what = format!("{label}: delete object {obj:#x} {err} {stays} {flag} {arena}");
            t.check(&what, 0x0044_1240, Conv::Thiscall(cluster), &[obj], |g| {
                Out::U32(pl::delete_object(g, cluster, obj))
            });
            t.reset();
        }
    }
}

#[test]
fn p1c_cluster_clear_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let late = [t.rebase(0x00c9_1180), t.rebase(0x00c9_1330)];
        for (round, world) in [(0u32, false), (1, true), (2, true), (3, false)] {
            t.stub(0x0044_3c20, true, 1, 1, 0);
            t.stub(0x0057_64a0, true, 0, 0, 0);
            t.stub(0x0049_c5b0, true, 0, 0, 0);
            t.stub(0x0049_c980, true, 1, 1, 0);
            t.stub(0x0049_cc10, true, 0, 0, 0);
            t.stub(descriptor::FREE, false, 1, 0, 0);
            let d = t.junk(0xa0, 3);
            let specs: Vec<(u32, u32, u32)> = (0..round * 3)
                .map(|i| match i % 3 {
                    0 => (d, i, 1),
                    1 => (late[0], i, 1),
                    _ => (late[(i as usize / 3) % 2], i, 1),
                })
                .collect();
            let (cluster, _) = p1c_cluster(&mut t, &specs);
            t.w32(cluster + 0x40, if world { 0x00c8_0000 } else { 0 });
            let mut prev = 0u32;
            for _ in 0..round {
                let a = t.zeros(0x10);
                t.w32(a, prev);
                prev = a;
            }
            t.w32(cluster + 0x24, prev);
            let what = format!("{label}: cluster clear {round} {world}");
            t.check(&what, 0x0044_0eb0, Conv::Thiscall(cluster), &[], |g| {
                let local = g.scratch();
                pl::clear(g, local, cluster);
                Out::None
            });
            t.reset();
        }
    }
}

#[test]
fn p1c_mapping_ctor_matches_the_original() {
    for (label, mut t) in Twins::all() {
        for (counts, alloc) in [
            ([0u32, 0, 0], 1u32),
            ([3, 2, 1], 1),
            ([3, 2, 1], 0),
            ([0, 5, 0], 1),
        ] {
            let block = if alloc == 0 { 0 } else { t.zeros(0x10) };
            t.stub(descriptor::ALLOC, false, 2, 0, block);
            t.stub(0x0047_bb00, false, 2, 0, 0);
            t.stub(0x0047_baa0, false, 2, 0, 0);
            t.stub(0x0047_c8b0, true, 4, 4, 0);
            let ns = t.junk(0x20, 1);
            t.w32(ns + 8, counts[0]);
            t.w32(ns + 0xc, counts[1]);
            t.w32(ns + 0x10, counts[2]);
            let map = t.junk(0x20, 2);
            let args = [ns, 0x00c1_0001, 0x00c1_0002, 0x00c1_0003];
            let what = format!("{label}: mapping ctor {counts:?} {alloc}");
            t.check(&what, 0x0047_86a0, Conv::Thiscall(map), &args, |g| {
                Out::U32(pl::mapping_ctor(g, map, args))
            });
            t.reset();
        }
    }
}

#[test]
fn p1c_asset_registry_matches_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(ffx_pload::types::ATEXIT, false, 1, 0, 0);
        // registration: no name, a duplicate, a new one
        for (named, dup) in [(false, false), (true, true), (true, false)] {
            t.stub(ffx_pload::types::ATEXIT, false, 1, 0, 0);
            t.stub(ffx_pload::p1_loader::LOCK, true, 0, 0, 0);
            t.stub(ffx_pload::p1_loader::UNLOCK, true, 0, 0, 0);
            t.stub(0x0045_b9d0, true, 1, 1, u32::from(dup) * 0x00c1_0000);
            t.stub(0x0057_4980, true, 1, 1, 0);
            let import = t.junk(0x30, 1);
            let name = if named { t.alloc(b"bsil0000\0") | 1 } else { 1 };
            t.w32(import + 0x18, name);
            let what = format!("{label}: import register {named} {dup}");
            t.check(&what, 0x0045_e1a0, Conv::Thiscall(import), &[], |g| {
                let local = g.scratch();
                Out::U32(pl::import_register(g, local, import))
            });
            t.reset();
        }
        // the destructor: in the tree or not, its name owned or not
        for (key, linked, owned) in [
            (0u32, true, true),
            (1, false, false),
            (1, true, true),
            (1, true, false),
        ] {
            t.stub(ffx_pload::types::ATEXIT, false, 1, 0, 0);
            t.stub(ffx_pload::p1_loader::LOCK, true, 0, 0, 0);
            t.stub(ffx_pload::p1_loader::UNLOCK, true, 0, 0, 0);
            t.stub(0x0057_5000, true, 1, 1, 0);
            t.stub(descriptor::FREE, false, 1, 0, 0);
            let import = t.junk(0x30, 2);
            t.w32(import + 0x14, key);
            t.w32(
                import + 0xc,
                if linked {
                    0x00c1_0001
                } else {
                    (import + 4) | 1
                },
            );
            t.w32(import + 0x18, if owned { 0x00c2_0000 } else { 0x00c2_0001 });
            let what = format!("{label}: import dtor {key} {linked} {owned}");
            t.check(&what, 0x0045_c0f0, Conv::Thiscall(import), &[], |g| {
                let local = g.scratch();
                pl::import_dtor(g, local, import);
                Out::None
            });
            t.reset();
        }
        // resolve: no chain; none found; found of the class, of a parent
        // class, of another class. The lookup's key is in the frame,
        // which the port's oracle gets at the original's address.
        let key = t.a.stack_range().end - 0x100 - 8 - 0x10;
        for case in 0..5u32 {
            t.stub(ffx_pload::types::ATEXIT, false, 1, 0, 0);
            t.stub(ffx_pload::p1_loader::LOCK, true, 0, 0, 0);
            t.stub(ffx_pload::p1_loader::UNLOCK, true, 0, 0, 0);
            // the warnings push 4 and 5 words
            t.stub(
                ffx_pload::p1_misc::PRINTF,
                false,
                if case == 1 { 4 } else { 5 },
                0,
                0,
            );
            t.stub(0x0043_cbb0, true, 0, 0, 0x00d0_0000);
            let class = t.junk(0xa0, 3);
            t.w32(class + 0x88, 0x10);
            let parent = t.junk(0xa0, 4);
            t.w32(parent + 0x40, 0);
            t.w32(class + 0x40, parent);
            let other = t.junk(0xa0, 5);
            t.w32(other + 0x40, 0);
            let found = match case {
                0 | 1 => 0,
                _ => {
                    let f = t.junk(0x30, 6);
                    t.w32(f + 0x1c, 0x00d1_0000);
                    let got = match case {
                        2 => class,
                        3 => {
                            // a class whose parent is the wanted one
                            let sub = t.junk(0xa0, 7);
                            t.w32(sub + 0x40, class);
                            sub
                        }
                        _ => other,
                    };
                    t.w32(f + 0x20, got);
                    f
                }
            };
            t.stub(0x0045_b8f0, true, 1, 1, found);
            let reference = t.junk(0x10, 8);
            t.w32(reference, class);
            let name = t.alloc(b"kami0100\0");
            t.w32(reference + 4, name | 1);
            let chain = if case == 0 {
                0
            } else {
                let w = t.zeros(12);
                t.w32(w, w + 4);
                t.w32(w + 4, w + 8);
                t.w32(w + 8, 0);
                w
            };
            t.w32(reference + 8, chain);
            t.b.set_call_top(Some(key - 0x100));
            let what = format!("{label}: resolve {case}");
            t.check(&what, 0x0045_e280, Conv::Thiscall(reference), &[], |g| {
                Out::U32(pl::resolve(g, key, reference))
            });
            t.b.set_call_top(None);
            t.reset();
        }
    }
}

// ---- continuation round, batch 12 (ffx_pload::p1_bind, p1_loader) ----

use ffx_pload::p1_bind::{self as pbind, bridge as pb};

/// Records the registration callees that walk or relink live statics.
/// The finders say "not there" (the method is linked).
fn p1c_bind_stubs(t: &mut Twins) {
    for (sym, this, n) in [
        (0x0057_52a0u32, true, 6u32),
        (0x0057_5f30, true, 6),
        (0x0043_a080, true, 3),
        (0x0043_59d0, true, 1),
        (0x0043_06b0, true, 1),
        (0x0043_c140, true, 0),
        (0x0043_e660, true, 1),
        (0x0043_79b0, true, 2),
        (0x0043_afb0, true, 4),
        (0x0044_ac10, true, 0),
        (0x0057_5440, true, 1),
    ] {
        t.stub(sym, this, n, n, 0);
    }
    // cdecl
    t.stub(0x0043_95b0, false, 1, 0, 0);
    t.stub(0x0044_b3b0, false, 1, 0, 0);
    // seven words: logged as six
    t.stub(0x0057_5ef0, true, 6, 7, 0);
}

#[test]
fn p1c_binds_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the second registered type has no name on the image until its
        // static constructor runs (as in the RegisterType test)
        let second = t.rebase(0x00c9_0760);
        if t.r32(second + 0x18) == 0 {
            let n = t.alloc(b"p1load_Second\0");
            t.w32(second + 0x18, n);
        }
        for b in pbind::STEPS {
            // the image's static descriptors and namespaces are not built
            // (its namespace lists loop): snapshots only
            if !t.snapshot {
                continue;
            }
            // as they find it (built on the snapshots), then once more,
            // then with the guard cleared (every member built again over
            // the live one, the same in both)
            for round in 0..3 {
                t.stub(ffx_pload::types::ATEXIT, false, 1, 0, 0);
                if round == 2 {
                    // building again over the live statics: the callees
                    // that would walk or relink them are recorded instead
                    p1c_bind_stubs(&mut t);
                    let desc = t.rebase(b.desc);
                    let flags = t.r32(desc + 0x90);
                    t.w32(desc + 0x90, flags & !0x4000_0000);
                    let guard = t.rebase(b.guard);
                    t.w32(guard, 0);
                    if let pbind::End::Getter { bound, .. } = b.end {
                        let bound = t.rebase(bound);
                        t.write(bound, &[0]);
                    }
                    // the dynamic-type handler's words the steps write
                    for st in b.steps {
                        if let pbind::Step::DynamicType { at, member, .. } = *st {
                            let (at, member) = (t.rebase(at), t.rebase(member));
                            t.w32(member + 0x28, 0x5a5a_0028);
                            t.w32(at + 0xc, 0x5a5a_000c);
                        }
                    }
                }
                let what = format!("{label}: bind {:#x} {round}", b.addr);
                t.check(&what, b.addr, Conv::Cdecl, &[], |g| {
                    let r = pb::run(g, &b);
                    Out::U32(r)
                });
                t.reset();
            }
        }
        for round in 0..3 {
            if !t.snapshot {
                break;
            }
            t.stub(ffx_pload::types::ATEXIT, false, 1, 0, 0);
            if round == 2 {
                p1c_bind_stubs(&mut t);
                let member = t.rebase(0x00c9_19ec);
                let flags = t.r32(member + 0x14);
                t.w32(member + 0x14, flags & !8);
                let array = t.rebase(0x00c9_1a28);
                let flags = t.r32(array + 0x90);
                t.w32(array + 0x90, flags | 2);
                let guard = t.rebase(0x00c9_1a20);
                t.w32(guard, 0);
                let inner = t.rebase(0x00c9_1abc);
                t.w32(inner, 0);
            }
            let what = format!("{label}: dependency list bind {round}");
            t.check(&what, 0x0044_b2b0, Conv::Cdecl, &[], |g| {
                Out::U32(pb::dependency_list_bind(g))
            });
            t.reset();
        }
    }
}

#[test]
fn p1c_mutex_lock_inner_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // (owner, compare-exchange result, wait result)
        // wait 2: the wait returns 0 after the owner let go (it writes the
        // thread id into the owner word: the next round finds it)
        for (owner, cas, wait) in [
            (0x77u32, 1u32, 1u32),
            (0, 0, 0),
            (5, 5, 1),
            (5, 0, 1),
            (5, 5, 2),
        ] {
            let add = t.stub_code(0xadd, false, 2, 2, 0x0012_0000);
            let cx = t.stub_code(0xca5, false, 3, 3, cas);
            for (slot, code) in [(0x00b0_c128u32, add), (0x00b0_c12c, cx)] {
                let iat = t.rebase(slot);
                t.w32(iat, code);
            }
            t.stub(0x0042_fd00, false, 0, 0, 0x77);
            t.stub(0x0043_01e0, true, 0, 0, 0);
            let mutex = t.junk(0x20, 1);
            let inner = mutex + 0x10;
            if wait == 2 {
                // mov dword [inner], 0x77; xor eax, eax; ret 4
                let mut code = vec![0xc7, 0x05];
                code.extend(inner.to_le_bytes());
                code.extend(0x77u32.to_le_bytes());
                code.extend([0x31, 0xc0, 0xc2, 0x04, 0x00]);
                let at = t.a.alloc_code(&code).unwrap();
                assert_eq!(t.b.alloc_code(&code).unwrap(), at);
                let sym = t.rebase(0x0043_0280);
                let old = Oracle::read(&mut t.a, sym, 5).unwrap();
                t.saved.entry(sym).or_insert(old);
                let mut j = vec![0xe9];
                j.extend(at.wrapping_sub(sym + 5).to_le_bytes());
                t.write(sym, &j);
            } else {
                t.stub(0x0043_0280, true, 1, 1, wait);
            }
            t.w32(inner, owner);
            let what = format!("{label}: mutex lock inner {owner:#x} {cas} {wait}");
            t.check(&what, 0x0043_1850, Conv::Thiscall(mutex), &[inner], |g| {
                Out::U32(pl::mutex_lock_inner(g, mutex, inner))
            });
            t.reset();
        }
    }
}

#[test]
fn p1c_resolve_asset_references_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let class = t.rebase(0x00c9_4030);
        for (round, done) in [(2u32, true), (0, false), (1, false), (2, false), (3, false)] {
            t.stub(0x0045_e280, true, 0, 0, 0);
            let other = t.junk(0xa0, 1);
            t.w32(other + 0x40, 0);
            let sub = t.junk(0xa0, 2);
            t.w32(sub + 0x40, class);
            let cluster = t.junk(0x50, 3);
            let head = cluster + 0x1c;
            t.w32(head, head);
            t.w32(head + 4, head);
            t.w32(cluster + 0x44, u32::from(done) | 0x100);
            for k in 0..round * 2 {
                let desc = match k % 3 {
                    0 => class,
                    1 => sub,
                    _ => other,
                };
                let list = t.junk(0x50, k as u8);
                t.w32(list + 0x2c, desc);
                let count = if k == 3 { 4 } else { 1 + k % 4 };
                let stride = 0x10;
                let buffer = t.zeros((stride * (count + 2)) as usize);
                t.w32(list + 0x34, buffer);
                t.w32(list + 0x3c, count);
                t.w32(list + 0x40, u32::from(k != 2));
                // the chunk ends: one inside the run when there are 3+
                let end = if count >= 3 {
                    let e = buffer + stride;
                    t.w32(e, buffer + 3 * stride);
                    e
                } else {
                    buffer + 0x1000
                };
                t.w32(list + 8, end);
                let d = t.r32(list + 0x2c);
                if d == class {
                    t.w32(class + 0x1c, stride);
                    t.w32(class + 0x8c, 4);
                }
                let tail = t.r32(head + 4);
                t.w32(list, head);
                t.w32(list + 4, tail);
                t.w32(tail, list);
                t.w32(head + 4, list);
            }
            let what = format!("{label}: resolve asset references {round} {done}");
            t.check(&what, 0x0044_1f20, Conv::Thiscall(cluster), &[], |g| {
                let it = g.scratch();
                Out::U32(pl::resolve_asset_references(g, it, cluster))
            });
            t.reset();
        }
    }
}

#[test]
fn p1c_load_asset_file_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // the original's frame: three arguments, the return address and
        // ebp; the reader at ebp - 0x28. The port's oracle gets it there.
        let reader = t.a.stack_range().end - 0x100 - 20 - 0x28;
        // (path, open, cluster block, load result)
        for (i, (path, open, block, load)) in [
            (0u32, 1u32, 1u32, 0u32),
            (1, 0, 1, 0),
            (1, 1, 0, 0),
            (1, 1, 1, 0),
            (1, 1, 1, 0x13),
        ]
        .into_iter()
        .enumerate()
        {
            let iob = t.stub_code(0x10b, false, 0, 0, 0x00c5_0000);
            let fpr = t.stub_code(0xf9f, false, 3, 0, 0);
            for (slot, code) in [(0x00b0_c350u32, iob), (0x00b0_c38c, fpr)] {
                let iat = t.rebase(slot);
                t.w32(iat, code);
            }
            let cluster = t.junk(0x50, 2);
            t.stub(0x0057_66d0, true, 1, 1, 0);
            t.stub(0x0057_6980, true, 0, 0, 0x0012_3400 | open);
            t.stub(
                0x0042_fb40,
                false,
                1,
                0,
                if block == 0 { 0 } else { cluster },
            );
            t.stub(0x0057_6630, true, 0, 0, cluster);
            t.stub(0x0057_6930, true, 0, 0, 0);
            t.stub(0x0057_bce0, false, 4, 0, load);
            t.stub(0x0043_fed0, true, 1, 1, 0);
            t.stub(0x0057_67c0, true, 0, 0, 0);
            let out = t.junk(4, 3);
            let p = if path == 0 {
                0
            } else {
                t.alloc(b"bsil0000.phyre\0")
            };
            t.b.set_call_top(Some(reader - 0x100));
            let what = format!("{label}: load asset file case {i}");
            t.check(&what, 0x0044_06c0, Conv::Cdecl, &[out, p, 7], |g| {
                Out::U32(pl::load_asset_file(g, reader, out, p, 7))
            });
            t.b.set_call_top(None);
            t.reset();
        }
    }
}

// ---- continuation round, batch 13 (ffx_pload::p1_reader) ----

use ffx_pload::p1_reader::bridge as prd;

/// A cluster reader with `classes` mapping entries (each: a descriptor
/// with its flags, size, over-allocation and alignment, or none; the
/// packed class's flags) and `headers` instance-list headers.
fn p1c_reader(
    t: &mut Twins,
    classes: &[(bool, u8, u32, u32, u32, u8)],
    headers: &[[u32; 6]],
    map_flags: u32,
    made: u32,
) -> u32 {
    let reader = t.junk(0x40, 1);
    t.w32(reader, 0x00c1_c000);
    let map = t.junk(0x20, 2);
    t.w32(reader + 4, map);
    t.w32(map + 0xc, classes.len() as u32 | 0x8000_0000);
    t.w32(map + 0x1c, map_flags);
    let table = t.zeros(32 * classes.len().max(1));
    t.w32(map + 0x10, table);
    for (i, &(has, flags, size, per, align, pflags)) in classes.iter().enumerate() {
        let e = table + 32 * i as u32;
        let packed = t.zeros(0x20);
        t.w32(packed + 4, (i as u32 % 4) << 28);
        t.write(packed + 0x1c, &[pflags]);
        t.w32(e + 4, packed);
        t.w32(e + 0x1c, i as u32 % 2);
        if has {
            let d = t.junk(0xa0, i as u8);
            t.write(d + 0x90, &[flags]);
            t.w32(d + 0x1c, size);
            t.w32(d + 0x74, per);
            t.w32(d + 0x78, align);
            t.w32(e + 8, d);
        }
    }
    let header = t.zeros(0x20);
    t.w32(header + 0x10, headers.len() as u32);
    t.w32(reader + 8, header);
    let hdrs = t.zeros(0x24 * headers.len().max(1));
    for (i, h) in headers.iter().enumerate() {
        for (k, &w) in h.iter().enumerate() {
            t.w32(hdrs + 0x24 * i as u32 + 4 * k as u32, w);
        }
    }
    t.w32(reader + 0x14, hdrs);
    let slots = t.junk(4 * headers.len().max(2), 3);
    t.w32(reader + 0x18, made);
    t.w32(
        reader + 0x1c,
        if made & 0x7fff_ffff > 1 {
            slots
        } else {
            0x5151
        },
    );
    let out = t.junk(4 * headers.len().max(1), 4);
    t.w32(reader + 0x24, out);
    t.write(reader + 0xc, &[0]);
    reader
}

const P1C_CLASSES: [(bool, u8, u32, u32, u32, u8); 5] = [
    (true, 0, 0x10, 0, 0, 0),
    (true, 4, 0x10, 0, 0, 0),
    (false, 0, 0, 0, 0, 0),
    (true, 0, 0x20, 8, 0x10, 0),
    (false, 0, 0, 0, 0, 4),
];

#[test]
fn p1c_init_instance_lists_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let hdrs = [
            [1u32, 3, 8, 0x40, 4, 2],
            [2, 2, 16, 0x30, 0, 1],
            [3, 0, 0x20, 0, 0, 0],
            [4, 5, 4, 0xa0, 6, 3],
        ];
        // (headers, mapping flags, list made, list result)
        for (k, (n, map_flags, made, list)) in [
            (4usize, 0u32, 4u32, 1u32),
            (4, 1, 4, 1),
            (4, 4, 4, 1),
            (4, 0x8000_0000, 0x8000_0004, 1),
            (1, 0, 1, 1),
            (2, 0, 2, 0),
            (0, 0, 0, 1),
        ]
        .into_iter()
        .enumerate()
        {
            for bad in [false, true] {
                let mut h: Vec<[u32; 6]> = hdrs[..n].to_vec();
                if bad {
                    h.push([5, 1, 4, 4, 0, 0]);
                }
                let made = if bad { made + 1 } else { made };
                let result = if list == 0 { 0 } else { t.junk(0x50, 5) };
                t.stub(0x0044_0cf0, true, 2, 2, 0x00c6_0000);
                t.stub(0x0044_0ba0, true, 4, 4, result);
                t.stub(0x0044_0b70, true, 3, 3, result);
                let reader = p1c_reader(&mut t, &P1C_CLASSES, &h, map_flags, made);
                let what = format!("{label}: init instance lists {k} {bad}");
                t.check(&what, 0x0057_b3a0, Conv::Thiscall(reader), &[], |g| {
                    Out::U32(prd::init_instance_lists(g, reader))
                });
                t.reset();
            }
        }
    }
}

#[test]
fn p1c_read_list_headers_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let hdrs = [
            [1u32, 3, 8, 0x40, 4, 2],
            [4, 5, 0x40, 0xa0, 6, 3],
            [2, 2, 16, 0x30, 0, 1],
            [4, 1, 2, 0xa0, 6, 3],
        ];
        for (k, (n, err, swap, bad)) in [
            (4usize, 0u32, false, false),
            (4, 9, false, false),
            (4, 0, true, false),
            (0, 0, true, false),
            (2, 0, false, true),
            (3, 0, false, true),
        ]
        .into_iter()
        .enumerate()
        {
            let mut h: Vec<[u32; 6]> = hdrs[..n].to_vec();
            if bad {
                // one past the mapping's count, or far past it
                h.push([if n == 3 { 6 } else { 9 }, 1, 4, 4, 0, 0]);
            }
            t.stub(0x0057_dd30, true, 2, 2, err);
            // the swapper: an object whose slot 0 records (p, 0)
            let swapper = t.rebase(0x00c9_7338);
            let obj = t.object(&[2]);
            let vt = t.r32(obj);
            let old_vt = t.r32(swapper);
            t.w32(swapper, vt);
            let stride_at = t.rebase(0x00c9_7354);
            let old_stride = t.r32(stride_at);
            t.w32(stride_at, 0x24);
            let reader = p1c_reader(&mut t, &P1C_CLASSES, &h, 0, 0);
            t.write(reader + 0xc, &[u8::from(swap)]);
            let words = t.zeros(12);
            t.w32(words, 4);
            t.w32(words + 4, 0x13);
            t.w32(words + 8, if k == 0 { 0x40 } else { 8 });
            let (a, s, m) = (words, words + 4, words + 8);
            let what = format!("{label}: read list headers {k}");
            t.check(
                &what,
                0x0057_c4a0,
                Conv::Thiscall(reader),
                &[0x00c1_5000, a, s, m],
                |g| Out::U32(prd::read_list_headers(g, reader, 0x00c1_5000, a, s, m)),
            );
            t.w32(swapper, old_vt);
            t.w32(stride_at, old_stride);
            t.reset();
        }
    }
}

// ---- continuation round, batch 14 (p1_reader, p1_misc) ----

/// An object of `size` bytes whose vtable's slot `i` is a recording stub
/// taking `slots[i].0` words and returning `slots[i].1`.
fn p1c_vobj(t: &mut Twins, slots: &[(u32, u32)], size: usize) -> u32 {
    let mut vt = Vec::new();
    for (i, &(n, ret)) in slots.iter().enumerate() {
        let s = t.stub_code(0x7100 + i as u32, true, n, n, ret);
        vt.extend(s.to_le_bytes());
    }
    let vt = t.alloc(&vt);
    let obj = t.junk(size, 0x31);
    t.w32(obj, vt);
    obj
}

#[test]
fn p1c_read_list_data_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // (class index, list data set, in place, packed offset, swap)
        for (k, (index, data, in_place, off, swap)) in [
            (9u32, true, false, 0u32, false),
            (3, true, false, 0, false),
            (1, false, false, 0, false),
            (1, true, true, 0, false),
            (1, true, false, 0, true),
            (2, true, false, 0x10, false),
            (2, true, false, 0x80, true),
            (4, true, false, 0, false),
            (2, true, false, 0x40, false),
            (1, true, false, 0xffff, false),
        ]
        .into_iter()
        .enumerate()
        {
            t.stub(0x0043_c160, true, 2, 2, 0);
            t.stub(0x0057_88f0, false, 4, 0, 0);
            t.stub(0x0043_d650, true, 3, 3, 0);
            let reader = p1c_reader(&mut t, &P1C_CLASSES, &[], 0, 0);
            t.write(reader + 0xc, &[u8::from(swap)]);
            if off == 0xffff {
                // a class with a descriptor and no packed class
                let map = t.r32(reader + 4);
                let table = t.r32(map + 0x10);
                t.w32(table + 4, 0);
            } else if off != 0 {
                let map = t.r32(reader + 4);
                let table = t.r32(map + 0x10);
                let packed = t.r32(table + 32 + 4);
                t.w32(packed + 4, 0x3000_0000 | off);
            }
            let buffer = t.junk(0x100, 5) + 4;
            let src = if in_place { buffer } else { t.junk(0x100, 6) };
            let list = t.junk(0x50, 7);
            t.w32(list + 0x30, if data { buffer + 4 } else { 0 });
            t.w32(list + 0x34, buffer);
            t.w32(list + 0x3c, 3);
            let hdr = t.zeros(0x24);
            t.w32(hdr, index);
            t.w32(hdr + 0xc, 0x40);
            t.w32(hdr + 0x10, 0x14);
            let what = format!("{label}: read list data {k}");
            t.check(
                &what,
                0x0057_b270,
                Conv::Thiscall(reader),
                &[list, src, hdr],
                |g| Out::U32(prd::read_list_data(g, reader, list, src, hdr)),
            );
            t.reset();
        }
    }
}

#[test]
fn p1c_read_cluster_header_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // the original's frame: five arguments, the return address and
        // ebp; its header copy at ebp - 0x54
        let local = t.a.stack_range().end - 0x100 - 28 - 0x54;
        let class = t.rebase(0x00c9_6b40);
        let tag = 0x4458_3131;
        // (has the class, packed, size, swap, tag ok)
        for (k, (has, packed, size, swap, ok)) in [
            (false, true, 0x54u32, 0u8, true),
            (true, false, 0x54, 0, true),
            (true, true, 0x54, 0, true),
            (true, true, 0x54, 1, false),
            (true, true, 0x60, 1, true),
            (true, true, 0x40, 1, true),
        ]
        .into_iter()
        .enumerate()
        {
            t.stub(0x0043_d6a0, true, 2, 2, 0);
            t.stub(0x0057_88f0, false, 4, 0, 0);
            let map = t.zeros(0x20);
            let table = t.zeros(0x80);
            t.w32(map + 0xc, 3);
            t.w32(table + 32 * 3 + 8, class);
            t.w32(map + 0x10, table);
            for i in 0..3 {
                t.w32(table + 32 * i + 8, 0x00c1_0000 + i);
            }
            if has {
                t.w32(table + 32 * 2 + 8, class);
                t.w32(table + 32 * 2 + 0x14, u32::from(packed));
            }
            let header = t.junk(0x60, 2);
            t.w32(header + 0xc, if ok { tag } else { tag + 1 });
            let file = t.zeros(8);
            t.w32(file + 4, 0x00c2_0000);
            t.b.set_call_top(Some(local - 0x100));
            let what = format!("{label}: read cluster header {k}");
            t.check(
                &what,
                0x0057_8830,
                Conv::Cdecl,
                &[header, size, file, map, u32::from(swap)],
                |g| {
                    Out::U32(prd::read_cluster_header(
                        g, local, header, size, file, map, swap,
                    ))
                },
            );
            t.b.set_call_top(None);
            t.reset();
        }
    }
}

#[test]
fn p1c_initialize_instance_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // (default buffer, construct result, member kinds, class flags)
        for (k, (default, built, flags)) in [
            (true, 1u32, 0x8000_0000u32),
            (true, 1, 0),
            (false, 0x0012_3400, 0x8000_0000),
            (false, 0x0012_3401, 0x8000_0000),
            (false, 1, 0x8000_0001),
        ]
        .into_iter()
        .enumerate()
        {
            t.stub(0x0043_c420, true, 0, 0, 0);
            t.stub(0x0043_c630, true, 0, 0, 0);
            // the member type: a class of 8 bytes with its default buffer
            let tdef = t.junk(8, 1);
            let ty = p1c_vobj(&mut t, &[(1, 0); 12], 0xa0);
            t.w32(ty + 0x1c, 8);
            t.w32(ty + 0x8c, 4);
            t.w32(ty + 0x88, 4);
            t.w32(ty + 0x90, 0x8000_0000);
            t.w32(ty + 0x64, tdef);
            let mut slots = [(0u32, 0u32); 13];
            slots[10] = (1, built);
            let desc = p1c_vobj(&mut t, &slots, 0xa0);
            t.w32(desc + 0x90, flags);
            t.w32(desc + 0x8c, 2);
            t.w32(desc + 0x1c, 0x40);
            t.w32(desc + 0x88, 0);
            t.w32(desc + 0x64, 0);
            if default {
                let d = t.junk(0x40, 2);
                t.w32(desc + 0x64, d);
            }
            let parent = p1c_vobj(&mut t, &[(1, 0); 2], 0xa0);
            t.w32(parent + 0x90, 0);
            t.w32(parent + 0x40, 0);
            t.w32(parent + 0x88, 0);
            t.w32(desc + 0x40, if k == 4 { 0 } else { parent });
            // the members: (kind info, type, flags, offset)
            for (owner, specs) in [
                (
                    desc,
                    &[
                        (0u32, true, 0u8, 0x10u32),
                        (1, true, 0, 0x18),
                        (2, true, 0, 0x20),
                        (0, false, 0, 0x28),
                        (0, true, 2, 0x30),
                    ][..],
                ),
                (parent, &[(0, true, 0, 0x8)][..]),
            ] {
                let head = owner + 0x44;
                t.w32(head, head);
                t.w32(head + 4, head);
                for &(kind, typed, mflags, offset) in specs {
                    let info = if kind == 0 {
                        0
                    } else {
                        let i = t.zeros(0x40);
                        t.w32(i + 0x30, kind);
                        t.w32(i + 0x34, 2);
                        i
                    };
                    let tyobj = p1c_vobj(&mut t, &[(0, 0), (0, if typed { ty } else { 0 })], 0x10);
                    let m = p1c_vobj(&mut t, &[(0, 0), (0, info)], 0x30);
                    t.write(m + 0x14, &[mflags]);
                    t.w32(m + 0x18, tyobj);
                    t.w32(m + 0x24, offset);
                    t.w32(m + 0xc, owner);
                    let tail = t.r32(head + 4);
                    t.w32(m + 4, head);
                    t.w32(tail, m + 4);
                    t.w32(head + 4, m + 4);
                }
            }
            let inst = t.junk(0x80, 3);
            let p = inst + 2;
            let what = format!("{label}: initialize instance {k}");
            t.check(&what, 0x0043_ce00, Conv::Thiscall(desc), &[p], |g| {
                Out::U32(pm::initialize_instance(g, desc, p))
            });
            t.reset();
        }
    }
}

// ---- continuation round, batch 15 ----

#[test]
fn p1c_delete_object_in_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // (obj, which list: 0 none, 1-3 the lists, err, stays)
        for (obj, which, err, stays) in [
            (0x00c7_0010u32, 0usize, 0u32, 0u32),
            (0x00c7_0010, 2, 0, 0),
            (0x00c7_0010, 1, 0, 0),
            (0x00c7_0000, 2, 0, 0),
            (0x00c7_0040, 2, 0, 0),
            (0x00c6_ffff, 2, 0, 0),
            (0x00c7_0010, 2, 9, 0),
            (0x00c7_0010, 2, 0, 1),
            (0x00c0_0000, 0, 0, 0),
        ] {
            t.stub(0x0057_6530, true, 1, 1, err);
            t.stub(0x0044_3c20, true, 1, 1, 0);
            t.stub(0x0057_64a0, true, 0, 0, 0);
            t.stub(0x0049_c5b0, true, 0, 0, 0);
            t.stub(0x0049_c980, true, 1, 1, 0);
            let d1 = t.junk(0xa0, 3);
            t.w32(d1 + 0x1c, 0x10);
            t.write(d1 + 0x90, &[0]);
            let d2 = t.junk(0xa0, 4);
            t.w32(d2 + 0x1c, 0x8);
            let (cluster, lists) = p1c_cluster(
                &mut t,
                &[
                    (d2, 0x00c6_0000, 2),
                    (d1, 0x00c7_0000, 4),
                    (d2, 0x00c9_0000, 8),
                ],
            );
            t.w32(lists[1] + 0x40, stays);
            t.w32(cluster + 0x40, 0x00c8_0000);
            let a1 = t.zeros(0x10);
            t.w32(a1 + 4, 0x00c6_ff00);
            t.w32(a1 + 0xc, 0x200);
            t.w32(cluster + 0x24, if obj == 0x00c7_0000 { a1 } else { 0 });
            let list = if which == 0 { 0 } else { lists[which - 1] };
            let what = format!("{label}: delete object in {obj:#x} {which} {err} {stays}");
            t.check(
                &what,
                0x0044_13a0,
                Conv::Thiscall(cluster),
                &[obj, list],
                |g| Out::U32(pl::delete_object_in(g, cluster, obj, list)),
            );
            t.reset();
        }
    }
}

#[test]
fn p1c_dynamic_descriptor_dtor_matches_the_original() {
    for (label, mut t) in Twins::all() {
        for (n, name_owned, owned) in [
            (0usize, true, true),
            (3, false, true),
            (4, true, false),
            (1, true, true),
        ] {
            t.stub(descriptor::FREE, false, 1, 0, 0);
            t.stub(0x0043_b3e0, true, 0, 0, 0);
            let desc = t.junk(0xc0, 1);
            let head = desc + 0x44;
            t.w32(head, head);
            t.w32(head + 4, head);
            for i in 0..n {
                let m = p1c_vobj(&mut t, &[(1, 0)], 0x30);
                t.write(m + 0x14, &[u8::from(i % 2 == 0)]);
                let tail = t.r32(head + 4);
                t.w32(m + 4, head);
                t.w32(tail, m + 4);
                t.w32(head + 4, m + 4);
            }
            t.w32(
                desc + 0xb4,
                if name_owned { 0x00c2_0000 } else { 0x00c2_0001 },
            );
            for (k, at) in [0xacu32, 0xa4, 0x9c].into_iter().enumerate() {
                let count = if owned { 3 } else { 0x8000_0003 };
                t.w32(desc + at, count);
                t.w32(desc + at + 4, if k == 1 { 0 } else { 0x00c3_0000 + at });
            }
            let what = format!("{label}: dynamic descriptor dtor {n} {name_owned} {owned}");
            t.check(&what, 0x0044_7880, Conv::Thiscall(desc), &[], |g| {
                pt::dynamic_dtor(g, desc);
                Out::None
            });
            t.reset();
        }
    }
}

/// Links `n` fresh nodes (0x20 bytes, flags at `+0x10` alternating) into
/// the list head at `head`.
fn p1c_fill_list(t: &mut Twins, head: u32, n: u32, owned_first: bool) {
    t.w32(head, head);
    t.w32(head + 4, head);
    for i in 0..n {
        let node = t.junk(0x20, i as u8);
        t.w32(node + 0x10, u32::from((i % 2 == 0) == owned_first));
        t.w32(node + 0xc, 0x00c4_0000 + i);
        let tail = t.r32(head + 4);
        t.w32(node, head);
        t.w32(node + 4, tail);
        t.w32(tail, node);
        t.w32(head + 4, node);
    }
}

#[test]
fn p1c_descriptor_destruct_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let serial_at = t.rebase(0x00c9_0758);
        for (k, (n, last)) in [(0u32, false), (3, true), (4, false), (1, true)]
            .into_iter()
            .enumerate()
        {
            t.stub(0x0043_ec40, true, 1, 1, 0);
            t.stub(0x0057_58e0, true, 0, 0, 0);
            t.stub(descriptor::FREE, false, 1, 0, 0);
            let desc = t.junk(0xa0, 1);
            for (i, at) in [0x44u32, 0x4c, 0x54, 0x5c].into_iter().enumerate() {
                p1c_fill_list(&mut t, desc + at, n + i as u32 % 2, k % 2 == 0);
            }
            let serial = t.r32(serial_at);
            t.w32(
                desc + 0x7c,
                if last { serial.wrapping_sub(1) } else { serial },
            );
            let what = format!("{label}: descriptor destruct {k}");
            t.check(&what, 0x0043_b3e0, Conv::Thiscall(desc), &[], |g| {
                pt::destruct(g, desc);
                Out::None
            });
            t.reset();
        }
    }
}

#[test]
fn p1c_fill_default_buffer_matches_the_original() {
    for (label, mut t) in Twins::all() {
        for (k, (parent_on, def)) in [(true, true), (false, true), (true, false)]
            .into_iter()
            .enumerate()
        {
            let src = t.junk(0x40, 9);
            t.stub(0x0043_cb80, true, 0, 0, if def { src } else { 0 });
            let ty = p1c_vobj(&mut t, &[(1, 0); 2], 0xa0);
            t.w32(ty + 0x88, 4);
            t.w32(ty + 0x8c, 2);
            let desc = p1c_vobj(&mut t, &[(0, 0); 2], 0xa0);
            t.w32(desc + 0x8c, 4);
            t.w32(desc + 0x88, 0);
            let buf = t.junk(0x100, 1);
            t.w32(desc + 0x64, buf);
            let parent = t.junk(0xa0, 2);
            t.w32(parent + 0x1c, 0x10);
            t.w32(parent + 0x8c, 2);
            t.w32(desc + 0x40, if parent_on { parent } else { 0 });
            let head = desc + 0x44;
            t.w32(head, head);
            t.w32(head + 4, head);
            // (kind, typed, flags, offset, type object size)
            for &(kind, typed, mflags, offset, tsize) in &[
                (0u32, true, 0u32, 0x10u32, 6u32),
                (1, true, 0, 0x18, 3),
                (2, true, 0, 0x30, 5),
                (2, true, 0x40, 0x38, 5),
                (0, false, 0, 0x40, 4),
                (0, true, 2, 0x48, 4),
                (0, true, 0x80, 0x50, 7),
            ] {
                let info = if kind == 0 {
                    0
                } else {
                    let i = t.zeros(0x40);
                    t.w32(i + 0x30, kind);
                    i
                };
                let tyobj = p1c_vobj(&mut t, &[(0, 0), (0, if typed { ty } else { 0 })], 0x20);
                t.w32(tyobj + 0x1c, tsize);
                let m = p1c_vobj(&mut t, &[(0, 0), (0, info)], 0x40);
                t.w32(m + 0x14, mflags);
                t.w32(m + 0x18, tyobj);
                t.w32(m + 0x24, offset);
                t.w32(m + 0xc, desc);
                t.w32(m + 0x34, 2);
                let tail = t.r32(head + 4);
                t.w32(m + 4, head);
                t.w32(tail, m + 4);
                t.w32(head + 4, m + 4);
            }
            let what = format!("{label}: fill default buffer {k}");
            t.check(&what, 0x0043_cfd0, Conv::Thiscall(desc), &[], |g| {
                pm::fill_default_buffer(g, desc);
                Out::None
            });
            t.reset();
        }
    }
}

#[test]
fn p1c_sort_class_descriptors_matches_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(ffx_pload::types::ATEXIT, false, 1, 0, 0);
        let n = t.r32(t.rebase(0x00c9_0758));
        for fail in [false, true] {
            let table = if fail {
                0
            } else {
                t.zeros(20 * (n as usize + 1))
            };
            t.stub(descriptor::ALLOC, false, 2, 0, table);
            t.stub(descriptor::FREE, false, 1, 0, 0);
            let what = format!("{label}: sort class descriptors {n} {fail}");
            t.check(&what, 0x0043_be70, Conv::Cdecl, &[], |g| {
                Out::U32(pt::sort_class_descriptors(g))
            });
        }
        t.reset();
    }
}

// ---- continuation round 3, batch 1 (ffx_pload::p1_tree, p1_d3d) ----

use ffx_pload::p1_d3d::bridge as pd3;
use ffx_pload::p1_tree::bridge as ptr;

impl Twins {
    /// Calls `sym` in both oracles alike (the setup both share).
    fn call_both(&mut self, sym: u32, conv: Conv, args: &[u32]) -> u32 {
        let addr = self.rebase(sym);
        let ra = self.a.call(addr, conv, args).unwrap().eax;
        let rb = self.b.call(addr, conv, args).unwrap().eax;
        assert_eq!(ra, rb, "setup call {sym:#x}");
        ra
    }

    /// The `i`th log entry's `(tag, this, words)`.
    fn log_entry(&self, i: u32) -> (u32, u32, [u32; 6]) {
        let e = self.log + 4 + 32 * i;
        let mut w = [0; 6];
        for (k, x) in (0u32..).zip(w.iter_mut()) {
            *x = self.r32(e + 8 + 4 * k);
        }
        (self.r32(e), self.r32(e + 4), w)
    }

    /// A recording stub (`this` logged, `n` stack words, popping them)
    /// that writes `value` through its stack word `k` before returning
    /// `ret`: a COM creation method.
    fn stub_out_code(&mut self, tag: u32, n: u32, k: u32, value: u32, ret: u32) -> u32 {
        assert!(n <= 6);
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0x8b, 0x54, 0x24, (4 + 4 * k) as u8]); // mov edx, [esp+4+4k]
        c.extend([0xc7, 0x02]);
        c.extend(value.to_le_bytes()); // mov [edx], value
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xb8);
        c.extend(ret.to_le_bytes()); // mov eax, ret
        c.push(0xc2);
        c.extend(((n * 4) as u16).to_le_bytes());
        let at = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), at);
        at
    }

    /// The frame bytes `[lo, lo + len)` of both oracles agree.
    fn same_frame(&mut self, lo: u32, len: u32, what: &str) {
        let fa = Oracle::read(&mut self.a, lo, len as usize).unwrap();
        let fb = Oracle::read(&mut self.b, lo, len as usize).unwrap();
        assert_eq!(fa, fb, "{what}: the frame differs");
    }
}

/// The nodes of the tree whose header is `tree + 0xc`, with their left
/// and right links.
fn p1d_nodes(t: &Twins, tree: u32) -> Vec<(u32, u32, u32)> {
    let mut out = Vec::new();
    let mut stack = vec![t.r32(tree + 0xc)];
    while let Some(n) = stack.pop() {
        if n == tree || out.len() > 4096 {
            continue;
        }
        let (l, r) = (t.r32(n), t.r32(n + 4));
        out.push((n, l, r));
        stack.push(l);
        stack.push(r);
    }
    out
}

#[test]
fn p1d_post_remove_fixup_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0x7ee5_0001);
        let mut fixups = 0;
        let mut reds = 0;
        for round in 0..40 {
            // a red-black tree built by the game's own insert and
            // post-insert fixup
            let tree = t.zeros(0x20);
            t.w32(tree + 0xc, tree);
            let n = 1 + (rng.next() % 40) as usize;
            let mut keys = std::collections::HashSet::new();
            for _ in 0..n {
                let k = if round % 2 == 0 {
                    rng.next() % 64
                } else {
                    rng.next()
                };
                if !keys.insert(k) {
                    continue;
                }
                let node = t.junk(0x10, k as u8);
                t.w32(node + 0xc, k);
                t.call_both(0x0058_1750, Conv::Thiscall(tree), &[node]);
                t.call_both(0x0057_4980, Conv::Thiscall(tree), &[node]);
            }
            t.diff("build");
            // remove nodes with at most one child (the fixup then runs at
            // the end of the original's remove), the fixup recorded
            loop {
                let nodes = p1d_nodes(&t, tree);
                let cands: Vec<u32> = nodes
                    .iter()
                    .filter(|&&(_, l, r)| l == tree || r == tree)
                    .map(|&(n, _, _)| n)
                    .collect();
                if cands.is_empty() {
                    break;
                }
                let z = cands[(rng.next() as usize) % cands.len()];
                t.stub(0x0057_4be0, true, 1, 1, 0);
                t.call_both(0x0057_5000, Conv::Thiscall(tree), &[z]);
                let logged = t.log_entries();
                let x = (logged > 0).then(|| t.log_entry(0).2[0]);
                t.reset();
                if let Some(x) = x {
                    fixups += 1;
                    reds += t.r32(x + 8) & 1;
                    if round % 3 == 2 {
                        // a red root (not a valid tree): the final node
                        // is the root read at entry
                        let root = t.r32(tree + 0xc);
                        let w = t.r32(root + 8);
                        t.w32(root + 8, w | 1);
                    }
                    let what = format!("{label}: round {round} fixup at {x:#x}");
                    t.check(&what, 0x0057_4be0, Conv::Thiscall(tree), &[x], |g| {
                        Out::U32(ptr::post_remove_fixup(g, tree, x))
                    });
                }
            }
        }
        // the root itself, red and black
        for colour in [0u32, 1] {
            let tree = t.zeros(0x20);
            let node = t.zeros(0x10);
            t.w32(tree + 0xc, node);
            t.w32(node, tree);
            t.w32(node + 4, tree);
            t.w32(node + 8, (tree + 0xc) | colour);
            let what = format!("{label}: root colour {colour}");
            t.check(&what, 0x0057_4be0, Conv::Thiscall(tree), &[node], |g| {
                Out::U32(ptr::post_remove_fixup(g, tree, node))
            });
        }
        assert!(
            fixups > 100 && reds > 10,
            "{label}: {fixups} fixups, {reds} red"
        );
    }
}

// The D3D11 creations' callees, stubbed alike in both oracles.
const P1D_ENTER: u32 = 0x0046_3d50;
const P1D_LEAVE: u32 = 0x0046_3e90;
const P1D_HOLDER_CTOR: u32 = 0x004f_ee70;
const P1D_ATEXIT: u32 = 0x0094_9207;
const P1D_CONTEXT: u32 = 0x0046_49d0;
const P1D_CACHE: u32 = 0x004f_d8b0;
const P1D_PRINTF: u32 = 0x0043_52f0;
const P1D_RESIZE: u32 = 0x0044_a010;
const P1D_BUFFER: u32 = ffx_pload::p1_d3d::CONSTANT_BUFFER_CREATE;
const P1D_LAYOUT: u32 = ffx_pload::p1_d3d::INPUT_LAYOUT_CREATE;

/// The device: `CreateX` methods at `slots` (slot, stack words, the out
/// word's index) writing an object (or null when not `made`) through
/// their out word and returning `hr`; every other slot null. Its objects
/// name themselves through slot 5. Also puts it in the holder and stubs
/// the lock, the holder's constructor, `atexit` and `PhyrePrintf`.
fn p1d_device(t: &mut Twins, slots: &[(u32, u32, u32)], made: bool, hr: u32) -> u32 {
    let obj = t.object(&[0, 0, 0, 0, 0, 4]);
    let mut vt = vec![0u8; 4 * 24];
    for &(slot, n, k) in slots {
        let value = if made { obj } else { 0 };
        let s = t.stub_out_code(0x8000 + slot, n, k, value, hr);
        vt[4 * slot as usize..4 * slot as usize + 4].copy_from_slice(&s.to_le_bytes());
    }
    let vt = t.alloc(&vt);
    let dev = t.zeros(0x10);
    t.w32(dev, vt);
    let holder = t.rebase(0x00c9_4f6c);
    t.w32(holder, dev);
    t.stub(P1D_ENTER, true, 0, 0, 0x4d50);
    t.stub(P1D_LEAVE, true, 0, 0, 0x4e90);
    t.stub(P1D_HOLDER_CTOR, true, 0, 0, 0);
    t.stub(P1D_ATEXIT, false, 1, 0, 0);
    t.stub(P1D_PRINTF, false, 2, 0, 0);
    dev
}

/// The holder's guard word with bit 0 as given (other bits random).
fn p1d_guard(t: &mut Twins, rng: &mut Rng, built: bool) {
    let g = t.rebase(0x00c9_628c);
    let v = (rng.next() & !1) | u32::from(built);
    t.w32(g, v);
}

/// The original's frame address `ebp - off` for `words` stack arguments.
fn p1d_frame(t: &Twins, words: u32, off: u32) -> u32 {
    t.a.stack_range().end - 0x100 - 4 * words - 8 - off
}

/// Fills `[lo, lo + len)` of both oracles with junk and moves the port's
/// calls below it.
fn p1d_prep_frame(t: &mut Twins, lo: u32, len: u32) {
    let junk: Vec<u8> = (0..len).map(|i| (i * 0x3d + 0x11) as u8).collect();
    t.write(lo, &junk);
    t.b.set_call_top(Some(lo - 0x100));
}

#[test]
fn p1d_shaders_match_the_original() {
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0xd3d1_0001);
        for case in 0..96u32 {
            let vertex = case % 2 == 1;
            let (sym, out_off, slot, p50, p41) = if vertex {
                (0x0058_d780, 0x4ec, 12, 0x7673_3530, 0x7673_3431)
            } else {
                (0x0058_d1c0, 0x4e0, 15, 0x7073_3530, 0x7073_3431)
            };
            let dev = p1d_device(&mut t, &[(slot, 5, 4)], true, 0);
            if case % 7 == 3 {
                let holder = t.rebase(0x00c9_4f6c);
                t.w32(holder, 0);
            }
            p1d_guard(&mut t, &mut rng, case % 3 == 0);
            let ctx = t.junk(0x100, case as u8);
            t.w32(ctx + 0x6c, dev);
            let level = [0x9300, 0xa0ff, 0xa100, 0xafff, 0xb000, 0xb100][(case / 2 % 6) as usize];
            t.w32(ctx + 0xb4, level);
            t.stub(P1D_CONTEXT, false, 0, 0, ctx);
            let cache = t.junk(0x1400, 7);
            t.stub(P1D_CACHE, false, 0, 0, cache);
            let samplers = t.rebase(0x00c9_6218);
            t.w32(samplers, rng.next());
            let rets = [0u32, 0, 5, 0, 9, 0x13];
            t.stub(P1D_BUFFER, true, 0, 0, rets[(case / 3 % 6) as usize]);
            t.stub(P1D_LAYOUT, true, 1, 1, rets[(case / 5 % 6) as usize]);
            let this = t.junk(0x500, case as u8 ^ 0x40);
            t.w32(this + 0xc, 0x100 | ((case % 3) << 30));
            let made = if case % 4 == 2 { 0x0bad_0000 } else { 0 };
            t.w32(this + out_off, made);
            let profile = [p50, p41, 0x1234_5678, p50 ^ p41][(case / 12 % 4) as usize];
            t.w32(this + 0x4dc, profile);
            let lock = p1d_frame(&t, 0, 0x10);
            p1d_prep_frame(&mut t, lock, 4);
            let what = format!("{label}: shader case {case}");
            t.check(&what, sym, Conv::Thiscall(this), &[], |g| {
                Out::U32(if vertex {
                    pd3::create_vertex_shader(g, this, lock)
                } else {
                    pd3::create_pixel_shader(g, this, lock)
                })
            });
            t.b.set_call_top(None);
            t.reset();
        }
    }
}

#[test]
fn p1d_input_layout_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0xd3d1_0002);
        for case in 0..24u32 {
            let made = case % 5 != 4;
            let hr = if case % 6 == 5 { 0x8007_000e } else { 1 };
            let dev = p1d_device(&mut t, &[(11, 6, 5)], made, hr);
            if case % 7 == 6 {
                // a failure that writes nothing
                let vt = t.r32(dev);
                let f = t.stub_code(0x800b, true, 6, 6, 0x8000_4005);
                t.w32(vt + 4 * 11, f);
            }
            p1d_guard(&mut t, &mut rng, case % 3 != 0);
            let n = [0u32, 1, 2, 5, 17, 32][(case % 6) as usize]
                - u32::from(case >= 12 && case % 6 != 0);
            let elems = t.junk(0x14 * n as usize + 4, case as u8);
            for i in 0..n {
                let e = elems + 0x14 * i;
                let name = match rng.next() % 4 {
                    0 => 0,
                    1 => 1,
                    _ => rng.next(),
                };
                t.w32(e, name);
                let step = if rng.next().is_multiple_of(3) {
                    0
                } else {
                    let s = t.zeros(0x10);
                    t.write(s + 8, &[(rng.next() % 2) as u8]);
                    s
                };
                t.w32(e + 4, step);
            }
            let this = t.junk(0x10, 3);
            t.w32(this, n | (rng.next() & 0x8000_0000));
            t.w32(this + 4, elems);
            t.w32(this + 8, if case % 8 == 7 { 0x0bad_0000 } else { 0 });
            let program = t.junk(0x20, 4);
            let frame = p1d_frame(&t, 1, 0x39c);
            p1d_prep_frame(&mut t, frame, pd3::LAYOUT_FRAME);
            let what = format!("{label}: input layout case {case} ({n} elements)");
            t.check(&what, 0x0058_d900, Conv::Thiscall(this), &[program], |g| {
                Out::U32(pd3::create_input_layout(g, this, program, frame))
            });
            t.b.set_call_top(None);
            // the layout word and the descriptions
            t.same_frame(frame + 4, pd3::LAYOUT_FRAME - 4, &what);
            t.reset();
        }
    }
}

#[test]
fn p1d_constant_buffer_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0xd3d1_0003);
        for case in 0..16u32 {
            let hr = if case % 4 == 3 { 0x8007_000e } else { 0 };
            p1d_device(&mut t, &[(3, 4, 3)], true, hr);
            p1d_guard(&mut t, &mut rng, case % 2 == 0);
            t.stub(P1D_RESIZE, true, 1, 1, 0x0d);
            let this = t.junk(0x20, case as u8);
            t.w32(this, if case % 5 == 1 { 0x0bad_0000 } else { 0 });
            t.w32(
                this + 0x14,
                [0, 0x10, 0x100, 0x8000_0010][(case / 2 % 4) as usize],
            );
            let frame = p1d_frame(&t, 0, 0x2c);
            p1d_prep_frame(&mut t, frame, pd3::BUFFER_FRAME);
            let what = format!("{label}: constant buffer case {case}");
            t.check(&what, 0x0058_f490, Conv::Thiscall(this), &[], |g| {
                Out::U32(pd3::create_constant_buffer(g, this, frame))
            });
            t.b.set_call_top(None);
            t.same_frame(frame + 4, 0x18, &what);
            t.reset();
        }
    }
}

#[test]
fn p1d_pass_state_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0xd3d1_0004);
        for case in 0..40u32 {
            let made = case % 7 != 6;
            let hr = if case % 9 == 8 { 0x8007_000e } else { 0 };
            let dev = p1d_device(&mut t, &[(20, 3, 2), (21, 3, 2), (22, 3, 2)], made, hr);
            p1d_guard(&mut t, &mut rng, case % 3 == 1);
            let ctx = t.junk(0x100, 9);
            t.w32(ctx + 0x6c, dev);
            t.stub(P1D_CONTEXT, false, 0, 0, ctx);
            let this = t.junk(0x180, case as u8);
            for (bit, off) in [0x168u32, 0x16c, 0x174, 0x170].into_iter().enumerate() {
                let v = if (case >> bit) & 1 == 1 && case % 11 != 0 {
                    0x0bad_0000 + off
                } else {
                    0
                };
                t.w32(this + off, v);
            }
            let frame = p1d_frame(&t, 0, 0x40);
            p1d_prep_frame(&mut t, frame, pd3::STATE_FRAME);
            let what = format!("{label}: pass state case {case}");
            t.check(&what, 0x0058_d4e0, Conv::Thiscall(this), &[], |g| {
                Out::U32(pd3::create_pass_state(g, this, frame))
            });
            t.b.set_call_top(None);
            t.same_frame(frame + 8, 0x28, &what);
            t.reset();
        }
    }
}

#[test]
fn p1d_views_match_the_original() {
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0xd3d1_0005);
        for case in 0..24u32 {
            let dev = p1d_device(&mut t, &[], true, 0);
            let vt = t.r32(dev);
            // CreateShaderResourceView: the `fail`th call fails (1..4;
            // past the count none does)
            let fail = 1 + case % 4;
            let ok = t.stub_code(0x8007, true, 4, 4, 0);
            let hr = if case % 3 == 0 { u32::MAX } else { 0x8000_4005 };
            let bad = t.stub_code(0x8107, true, 4, 4, hr);
            let calls = t.zeros(4);
            let at = t.a.alloc_code(&[0x90; 32]).unwrap();
            assert_eq!(t.b.alloc_code(&[0x90; 32]).unwrap(), at);
            let mut c = vec![0xff, 0x05];
            c.extend(calls.to_le_bytes()); // inc dword [calls]
            c.extend([0x83, 0x3d]);
            c.extend(calls.to_le_bytes());
            c.push(fail as u8); // cmp dword [calls], fail
            c.extend([0x75, 0x05]); // jne +5
            c.push(0xe9);
            let jbad = bad.wrapping_sub(at + c.len() as u32 + 4);
            c.extend(jbad.to_le_bytes()); // jmp bad
            c.push(0xe9);
            let jok = ok.wrapping_sub(at + c.len() as u32 + 4);
            c.extend(jok.to_le_bytes()); // jmp ok
            t.write(at, &c);
            t.w32(vt + 4 * 7, at);
            p1d_guard(&mut t, &mut rng, case % 2 == 0);
            let n = [1u32, 2, 3, 5, 0x8000_0002, 0x8000_0001][(case / 4 % 6) as usize];
            let this = t.junk(0x80, case as u8);
            let base = t.junk(0x40, 1);
            t.w32(this, base);
            let flags = if case % 6 == 5 {
                0xf7
            } else {
                0x08 | (rng.next() & 0xf7)
            };
            t.write(this + 0x58, &[flags as u8]);
            t.w32(this + 0x20, n);
            if n & 0x7fff_ffff > 1 {
                let views = t.junk(0x14 * 5, 2);
                t.w32(this + 0x24, views);
            }
            let frame = p1d_frame(&t, 0, 0x30);
            p1d_prep_frame(&mut t, frame, pd3::VIEWS_FRAME);
            let what = format!("{label}: views case {case}");
            t.check(&what, 0x0058_f9f0, Conv::Thiscall(this), &[], |g| {
                Out::U32(pd3::create_views(g, this, frame))
            });
            t.b.set_call_top(None);
            t.same_frame(frame, 0x18, &what);
            t.reset();
        }
    }
}

#[test]
fn p1d_cube_map_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0xd3d1_0006);
        for case in 0..48u32 {
            // the texture objects CreateTexture2D makes: GetDesc at slot 10
            let tex = t.object(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]);
            let hr = [0u32, 0, 0, 1, 0x8007_000e][(case % 5) as usize];
            let dev = p1d_device(&mut t, &[], true, 0);
            let vt = t.r32(dev);
            let f = t.stub_out_code(0x8005, 4, 3, tex, hr);
            t.w32(vt + 4 * 5, f);
            if case % 11 == 10 {
                let holder = t.rebase(0x00c9_4f6c);
                t.w32(holder, 0);
            }
            p1d_guard(&mut t, &mut rng, case % 2 == 0);
            t.stub(
                ffx_pload::p1_d3d::CUBE_VIEWS,
                true,
                0,
                0,
                [0, 9, 0][(case % 3) as usize],
            );
            let base = t.junk(0x40, case as u8);
            t.w32(
                base + 0xc,
                [4u32, 8, 32, 0x8000_0008][(case / 3 % 4) as usize],
            );
            t.w32(base + 0x10, rng.next() & 3);
            let this = t.junk(0x80, case as u8 ^ 9);
            t.w32(this, base);
            let size = [0u32, 1, 6, 64, 0x200][(case / 2 % 5) as usize];
            t.w32(this + 0x1c, size);
            t.w32(this + 0xc, [0u32, 1, 3, 9][(case / 4 % 4) as usize]);
            t.w32(this + 0x10, rng.next() % 2);
            t.w32(this + 0x14, rng.next());
            let owner = match case % 6 {
                0 => 0,
                1 => {
                    let o = t.zeros(8);
                    t.write(o + 5, &[1]);
                    o
                }
                _ => t.zeros(8),
            };
            t.w32(this + 0x18, owner);
            let n = [1u32, 2, 3, 0x8000_0001][(case / 6 % 4) as usize];
            t.w32(this + 0x20, n);
            if n & 0x7fff_ffff > 1 {
                let arr = t.junk(0x14 * 3, 5);
                t.w32(this + 0x24, arr);
            }
            let data = if case % 4 == 1 { 0 } else { 0x0012_0000 };
            let views = u32::from(case % 8 < 5) * (1 + (rng.next() & 0xfe));
            let skip = u32::from(case % 13 == 12) | (rng.next() & 0xff00);
            let frame = p1d_frame(&t, 4, 0x650);
            p1d_prep_frame(&mut t, frame, pd3::CUBE_FRAME);
            let what = format!("{label}: cube map case {case}");
            t.check(
                &what,
                0x0059_0800,
                Conv::Thiscall(this),
                &[data, views, 0x5555, skip],
                |g| {
                    Out::U32(pd3::create_cube_map(
                        g,
                        this,
                        data,
                        views as u8,
                        skip as u8,
                        frame,
                    ))
                },
            );
            t.b.set_call_top(None);
            // the description and the subresources
            t.same_frame(frame + 0x14, pd3::CUBE_FRAME - 0x14, &what);
            t.reset();
        }
    }
}

// ---- continuation round 3, batch 2 (ffx_pload::p1_post, p1_d3d data blocks) ----

use ffx_pload::p1_post::bridge as ppo;

#[test]
fn p1d_post_ctors_match_the_original() {
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0xd3d1_0007);
        for case in 0..24u32 {
            // the sampler constructor stubbed in odd cases: the flags'
            // bits 6 and 7 then stay as written here
            if case % 2 == 1 {
                t.stub(0x0058_3b20, true, 0, 0, 0x3b20);
            }
            let this = t.junk(0x4100, case as u8);
            for s in [0x3ee8u32, 0x3f0c, 0x3f30, 0xf40, 0xf64] {
                let w = (rng.next() & !0xc0) | (((case / 6) & 3) << 6);
                t.w32(this + s + 0x1c, w);
            }
            let (sym, what) = match case % 3 {
                0 => (0x009f_b830, "deferred lighting base"),
                1 => (0x009f_bb30, "deferred lighting D3D11"),
                _ => (0x009f_bd90, "depth of field base"),
            };
            let what = format!("{label}: {what} ctor {case}");
            t.check(&what, sym, Conv::Thiscall(this), &[], |g| {
                Out::U32(match case % 3 {
                    0 => ppo::deferred_lighting_base_ctor(g, this),
                    1 => ppo::deferred_lighting_d3d11_ctor(g, this),
                    _ => ppo::depth_of_field_base_ctor(g, this),
                })
            });
            t.reset();
        }
    }
}

#[test]
fn p1d_data_blocks_match_the_original() {
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0xd3d1_0008);
        for case in 0..48u32 {
            let index = case % 2 == 0;
            let (sym, k) = if index {
                (0x0056_1e90, &pd3::INDEX_BLOCK)
            } else {
                (0x0056_20c0, &pd3::DATA_BLOCK)
            };
            // CreateBuffer and CreateShaderResourceView: the `fail`th
            // call of the two fails (1..6; past the calls none does)
            let buffer = t.object(&[0, 0, 0, 0, 0, 4]);
            let dev = p1d_device(&mut t, &[], true, 0);
            let vt = t.r32(dev);
            let fail = 1 + case / 2 % 6;
            let calls = t.zeros(4);
            let bad_hr = if case % 3 == 0 { u32::MAX } else { 0x8000_4005 };
            for (slot, tag) in [(3u32, 0x8003u32), (7, 0x8007)] {
                let ok = t.stub_out_code(tag, 4, 3, buffer, 0);
                let bad = t.stub_code(tag + 0x100, true, 4, 4, bad_hr);
                let at = t.a.alloc_code(&[0x90; 32]).unwrap();
                assert_eq!(t.b.alloc_code(&[0x90; 32]).unwrap(), at);
                let mut c = vec![0xff, 0x05];
                c.extend(calls.to_le_bytes()); // inc dword [calls]
                c.extend([0x83, 0x3d]);
                c.extend(calls.to_le_bytes());
                c.push(fail as u8); // cmp dword [calls], fail
                c.extend([0x75, 0x05]); // jne +5
                c.push(0xe9);
                let jbad = bad.wrapping_sub(at + c.len() as u32 + 4);
                c.extend(jbad.to_le_bytes()); // jmp bad
                c.push(0xe9);
                let jok = ok.wrapping_sub(at + c.len() as u32 + 4);
                c.extend(jok.to_le_bytes()); // jmp ok
                t.write(at, &c);
                t.w32(vt + 4 * slot, at);
            }
            if case % 9 == 8 {
                let holder = t.rebase(0x00c9_4f6c);
                t.w32(holder, 0);
            }
            p1d_guard(&mut t, &mut rng, case % 4 < 2);
            let this = t.junk(0x40, case as u8);
            let size = [0x40u32, 0, 0x1003, 4][(case / 4 % 4) as usize];
            t.w32(this + k.size, size);
            let base = [0u32, 0x0040_0000][(case / 3 % 2) as usize];
            let offset = if case % 5 == 0 { 0 } else { 0x100 };
            t.w32(this + k.offset, offset);
            let n = [1u32, 2, 3, 0, 0x8000_0002, 0x0001_0000][(case / 8 % 6) as usize];
            t.w32(this + k.array, n);
            if n & 0x7fff_ffff > 1 {
                let arr = t.junk(0x14 * 3, 5);
                t.w32(this + k.array + 4, arr);
            }
            let frame = p1d_frame(&t, 1, 0x64);
            p1d_prep_frame(&mut t, frame, pd3::BLOCK_FRAME);
            let what = format!("{label}: data block case {case} (index {index})");
            t.check(&what, sym, Conv::Thiscall(this), &[base], |g| {
                Out::U32(pd3::create_data_block(g, this, base, k, frame))
            });
            t.b.set_call_top(None);
            // the initial data, the view's and the buffer's descriptions
            t.same_frame(frame, 0x24, &what);
            t.same_frame(frame + 0x3c, 0x18, &what);
            t.reset();
        }
    }
}

impl Twins {
    /// [`Twins::stub_code`] logging `this` and the `n` stack words after
    /// the first `skip`.
    fn stub_code_skip(&mut self, tag: u32, skip: u32, n: u32, pops: u32, ret: u32) -> u32 {
        assert!(n <= 6);
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * (skip + i)) as u8]); // mov edx, [esp+4+4(skip+i)]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xb8);
        c.extend(ret.to_le_bytes()); // mov eax, ret
        c.push(0xc2);
        c.extend(((pops * 4) as u16).to_le_bytes());
        let at = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), at);
        at
    }
}

#[test]
fn p1d_declarations_match_the_original() {
    use ffx_pload::p1_declare::{self as pdc, bridge as pdb};
    use ffx_prender::declare as dc;
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0x0dec_1a2f);
        t.stub(dc::ATEXIT, false, 1, 0, 0x6003);
        t.stub(dc::REGISTER_TYPE, false, 1, 0, 0x6001);
        t.stub(dc::ADD_TO_NAMESPACE, true, 0, 0, 0x5a5a_1234);
        t.stub(dc::TYPE_UINT, false, 0, 0, 0x00c9_0500);
        t.stub(dc::DATA_MEMBER_CTOR, true, 6, 6, 6);
        t.stub(dc::ANNOTATION_CTOR, true, 3, 3, 3);
        t.stub(dc::ADD_ANNOTATION, true, 1, 1, 1);
        for (f, c, aligned) in [
            (0x0045_8460u32, pdc::PTYPED_OBJECT_ARRAY, false),
            (0x0044_8450, pdc::U8_ARRAY, true),
        ] {
            t.stub(c.ctor, true, 0, 0, 0x00c9_7000 | (c.ctor & 0xfff));
            if let dc::Element::Getter(g) = c.element {
                t.stub(g, false, 0, 0, 0x00c9_7000 | (g & 0xfff));
            }
            let d = t.rebase(c.desc);
            let ng = t.rebase(c.name);
            let annot = d + 0xc4;
            for case in 0..24u32 {
                // the elements member's constructor: logs `this` and its
                // last six words (the descriptor word is fixed), pops 7
                let ctor = t.rebase(dc::MEMBER_ARRAY_CTOR);
                let code = t.stub_code_skip(dc::MEMBER_ARRAY_CTOR, 1, 6, 7, 7);
                let mut j = vec![0xe9];
                j.extend(code.wrapping_sub(ctor + 5).to_le_bytes());
                let old = Oracle::read(&mut t.a, ctor, 5).unwrap();
                t.saved.entry(ctor).or_insert(old);
                t.write(ctor, &j);
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
                let first = if rng.next().is_multiple_of(2) {
                    0
                } else {
                    rng.next() | 1
                };
                t.w32(ng, first);
                let link = if rng.next().is_multiple_of(2) {
                    annot + 4
                } else {
                    rng.next()
                };
                t.w32(annot + 4, link);
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
                let what = format!("{label}: declare {f:#x} case {case}");
                t.check(&what, f, Conv::Cdecl, &[name], |g| {
                    Out::U32(if aligned {
                        pdb::declare_aligned(g, name, &c, pdc::U8_ALIGNMENT)
                    } else {
                        dc::bridge::declare(g, name, &c)
                    })
                });
                // put the elements member's constructor back for the
                // next case's fresh stub
                let saved = t.saved.remove(&ctor).unwrap();
                t.write(ctor, &saved);
                let log = t.log;
                t.w32(log, 0);
            }
        }
        t.reset();
    }
}

// ---- continuation round 3, batch 3 (ffx_pload::p1_buffers) ----

use ffx_pload::p1_buffers::bridge as pbu;

/// An object whose vtable holds recording stubs: slot `i` takes and pops
/// `slots[i].0` words and returns `slots[i].1`; the object is `size`
/// junk bytes after its vtable word.
fn p1d_vobj(t: &mut Twins, tag: u32, slots: &[(u32, u32)], size: usize) -> u32 {
    let mut vt = Vec::new();
    for (i, &(n, ret)) in (0u32..).zip(slots) {
        let s = t.stub_code(tag + i, true, n, n, ret);
        vt.extend(s.to_le_bytes());
    }
    let vt = t.alloc(&vt);
    let obj = t.junk(size, tag as u8);
    t.w32(obj, vt);
    obj
}

/// A constructor slot (thiscall `(p)`, `ret 4`) that logs, writes the
/// address it is given at `p` (a position-dependent constructor when
/// `moving`, else a constant) and returns `ret`.
fn p1d_ctor_code(t: &mut Twins, tag: u32, moving: bool, ret: u32) -> u32 {
    let log = t.log;
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(tag.to_le_bytes()); // mov [eax], tag
    c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
    c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
    c.extend([0x89, 0x50, 0x08]); // mov [eax+8], edx
    if moving {
        c.extend([0x89, 0x12]); // mov [edx], edx
    } else {
        c.extend([0xc7, 0x02, 0x11, 0x22, 0x33, 0x44]); // mov dword [edx], 0x44332211
    }
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    c.push(0xb8);
    c.extend(ret.to_le_bytes()); // mov eax, ret
    c.extend([0xc2, 0x04, 0x00]); // ret 4
    let at = t.a.alloc_code(&c).unwrap();
    assert_eq!(t.b.alloc_code(&c).unwrap(), at);
    at
}

/// A descriptor of `size` bytes with stub slots (10 construct, 11
/// destruct, 12 its preparation), buffers at `+0x64/+0x68/+0x6c`, its
/// own part at `+0x8c`, `flags`, an empty member ring.
fn p1d_desc(t: &mut Twins, tag: u32, size: u32, flags: u32, moving: bool, made: u32) -> u32 {
    let mut slots = vec![(0u32, 0u32); 13];
    slots[11] = (1, 0x11);
    slots[12] = (0, 0x12);
    let d = p1d_vobj(t, tag, &slots, 0xa0);
    let vt = t.r32(d);
    let ctor = p1d_ctor_code(t, tag + 10, moving, made);
    t.w32(vt + 40, ctor);
    for off in [0x64u32, 0x68, 0x6c] {
        let b = t.junk(size as usize + 8, off as u8);
        t.w32(d + off, b);
    }
    t.w32(d + 0x1c, size);
    t.w32(d + 0x18, 0x00c1_0000 | tag);
    t.w32(d + 0x8c, 4);
    t.w32(d + 0x88, 0x10);
    t.w32(d + 0x90, flags);
    t.w32(d + 0x40, 0);
    let head = d + 0x44;
    t.w32(head, head);
    t.w32(head + 4, head);
    d
}

/// A member of `desc` at `offset` (relative to the owner's base
/// `+0x88`), of kind `kind` (0 none), flags `mflags`, with a type object
/// of element size `esize` whose class is `ty` (0 none).
fn p1d_member(
    t: &mut Twins,
    desc: u32,
    kind: u32,
    mflags: u32,
    offset: u32,
    esize: u32,
    ty: u32,
) -> u32 {
    let info = if kind == 0 {
        0
    } else {
        let i = t.zeros(0x40);
        t.w32(i + 0x30, kind);
        i
    };
    let tyobj = p1d_vobj(t, 0x7200, &[(0, 0), (0, ty)], 0x20);
    t.w32(tyobj + 0x1c, esize);
    let m = p1d_vobj(t, 0x7300, &[(0, 0), (0, info)], 0x40);
    t.w32(m + 0x14, mflags);
    t.w32(m + 0x18, tyobj);
    t.w32(m + 0x24, offset + t.r32(desc + 0x88));
    t.w32(m + 0xc, desc);
    t.w32(m + 0x10, 0x00c2_0000 | offset);
    let head = desc + 0x44;
    let tail = t.r32(head + 4);
    t.w32(m + 4, head);
    t.w32(tail, m + 4);
    t.w32(head + 4, m + 4);
    m
}

#[test]
fn p1d_default_buffer_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0xbf00_0001);
        for case in 0..40u32 {
            let src = t.junk(0x40, 9);
            t.stub(0x0043_cb80, true, 0, 0, src);
            t.stub(0x0043_cfd0, true, 0, 0, 0xcfd0);
            t.stub(0x0043_52f0, false, 3, 0, 0x0123_4567);
            let flags = match case % 8 {
                0 if case & 8 != 0 => u32::MAX,
                0 => 0x8000_0000,
                1 => 1,
                2 => 2,
                3 => 8,
                _ => rng.next() & 0x7000_0000,
            };
            let made = if case % 7 == 6 { 0x100 } else { 1 };
            let d = p1d_desc(&mut t, 0x7400, 0x30, flags, case % 3 == 0, made);
            if case % 11 == 10 {
                t.w32(d + 0x64, 0);
            }
            if case % 13 == 12 {
                t.w32(d + 0x68, 0);
            }
            let ty = p1d_desc(&mut t, 0x7500, 0x8, 0x8000_0000, false, 1);
            for (kind, mflags, off) in [
                (4u32, 0u32, 0u32),
                (0, 0x40, 8),
                (0, 0, 0x10),
                (4, 2, 0x14),
                (2, 0x40, 0x18),
                (4, 0, 0x20),
            ] {
                let typed = if rng.next().is_multiple_of(4) { 0 } else { ty };
                p1d_member(&mut t, d, kind, mflags, off, 8, typed);
            }
            let what = format!("{label}: default buffer {case}");
            t.check(&what, 0x0043_c420, Conv::Thiscall(d), &[], |g| {
                pbu::calculate_default_buffer(g, d);
                Out::None
            });
            t.reset();
        }
    }
}

#[test]
fn p1d_validation_buffer_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0xbf00_0002);
        for case in 0..52u32 {
            // 48..: one fixed array claiming 0x104 of 0x105 bytes (offset
            // 0): the last scan's index and bit 28
            let big = case >= 48;
            t.stub(0x0043_cb80, true, 0, 0, 0);
            t.stub(0x0043_cfd0, true, 0, 0, 0xcfd0);
            t.stub(0x0043_52f0, false, 4, 0, 0x0123_4567);
            let fixed = if big {
                0x41
            } else {
                [2u32, 3, 0][(case % 3) as usize]
            };
            t.stub(0x0057_6050, true, 0, 0, fixed);
            let value = t.junk(8, 3);
            let ann = p1d_vobj(&mut t, 0x7600, &[(0, 0), (0, value)], 0x10);
            t.stub(0x0057_5d40, true, 1, 1, if case % 2 == 0 { ann } else { 0 });
            let size = if big { 0x105 - (case & 1) } else { 0x80 };
            let d = p1d_desc(&mut t, 0x7400, size, rng.next() & 0x7fff_ffff, false, 1);
            if big {
                t.w32(d + 0x8c, 0);
            }
            if case % 11 == 10 {
                t.w32(d + 0x68, 0);
            }
            if case % 13 == 12 {
                t.w32(d + 0x6c, 0);
            }
            // a parent, done or to be worked out (its buffers through the
            // originals, its slots stubs), with or without buffers
            if case % 4 != 0 && !big {
                let pflags = if case % 4 == 1 {
                    0x8800_0000
                } else {
                    0x0800_0003 | ((case / 4 % 2) << 30) | ((case / 8 % 2) << 28)
                };
                let p = p1d_desc(&mut t, 0x7700, 0x10, pflags, false, 1);
                if case % 5 == 0 {
                    t.w32(p + 0x68, 0);
                }
                if case % 7 == 0 {
                    t.w32(p + 0x6c, 0);
                }
                t.w32(d + 0x40, p);
            }
            let tyf = if case % 3 == 0 {
                0x0000_0003
            } else {
                0x8000_0000
            };
            let ty = p1d_desc(&mut t, 0x7500, 0x8, tyf, false, 1);
            if case % 5 == 3 {
                t.w32(ty + 0x68, 0);
            }
            if case % 6 == 4 {
                t.w32(ty + 0x6c, 0);
            }
            // the members: kinds, flags, offsets (an overlap in some
            // cases), element sizes, typed or not
            let overlap = case % 9 == 8;
            for (i, (kind, mflags, off)) in [
                (4u32, 0u32, 0u32),
                (0, 0x40, 0x10),
                (2, 0, 0x20),
                (2, 0x40, 0x28),
                (0, 2, 0x30),
                (1, 0, 0x38),
                (0, 0, if overlap { 2 } else { 0x60 }),
            ]
            .into_iter()
            .enumerate()
            {
                if big && i != 5 {
                    continue;
                }
                let (kind, off) = if big { (1, 0) } else { (kind, off) };
                let kind = if i == 0 && case % 5 == 0 { 0 } else { kind };
                let typed = if (rng.next() >> i).is_multiple_of(3) || big {
                    0
                } else {
                    ty
                };
                let esize = if big { 4 } else { 8 };
                p1d_member(&mut t, d, kind, mflags, off, esize, typed);
            }
            let what = format!("{label}: validation buffer {case}");
            t.check(&what, 0x0043_c630, Conv::Thiscall(d), &[], |g| {
                Out::U32(pbu::calculate_validation_buffer(g, d))
            });
            t.reset();
        }
    }
}

#[test]
fn p1d_fixup_instances_matches_the_original() {
    use ffx_pload::p1_fixup::bridge as pfx;
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0xf1c5_0001);
        let import = t.rebase(0x00c9_4030);
        let named = t.rebase(0x00c9_0760);
        let dynamic = t.rebase(0x00c9_1330);
        for case in 0..32u32 {
            // the asset the lookup finds: a dynamic class object or not
            let adesc = t.junk(0xa0, 4);
            t.w32(adesc + 0x40, if case % 4 == 3 { 0 } else { dynamic });
            let asset = t.junk(0x40, 5);
            t.w32(asset + 0x20, adesc);
            let mut slots = vec![(0u32, 0u32); 15];
            slots[7] = (1, 0x77);
            slots[14] = (1, 0);
            let target = p1d_vobj(&mut t, 0x7b00, &slots, 0x60);
            t.w32(target + 0x40, 0);
            let found = match case % 5 {
                0 => 0,
                1 => 0x18,
                _ => target + 0x18,
            };
            t.w32(asset + 0x1c, found);
            t.stub(
                0x0045_d280,
                false,
                1,
                0,
                if case % 7 == 6 { 0 } else { asset },
            );
            let cluster = t.junk(0x50, 3);
            let iring = cluster + 8;
            t.w32(iring, iring);
            t.w32(iring + 4, iring);
            let lists = cluster + 0x1c;
            t.w32(lists, lists);
            t.w32(lists + 4, lists);
            t.w32(cluster + 0x44, if case == 0 { 2 } else { rng.next() & !2 });
            // the import nodes: named "tex_a", "tex_b" or unnamed
            let str_a = t.alloc(b"tex_a\0");
            let str_b = t.alloc(b"tex_b\0");
            let mut nodes = Vec::new();
            for k in 0..=(case % 3) {
                let n = p1d_vobj(&mut t, 0x7800, &slots, 0x60);
                t.w32(n + 0x40, 0);
                let nm = match (k + case) % 3 {
                    0 => str_a,
                    1 => str_b,
                    _ => 0,
                };
                t.w32(n + 0x18, nm);
                let link = n + 0x2c;
                let tail = t.r32(iring + 4);
                t.w32(link, iring);
                t.w32(link + 4, tail);
                t.w32(tail, link);
                t.w32(iring + 4, link);
                nodes.push(n);
            }
            if case % 6 == 5 {
                // a node that is the asset object itself
                t.w32(asset + 0x1c, nodes[nodes.len() - 1] + 0x18);
            }
            // the instance lists: the imports' (elements naming "tex_a"
            // or "tex_b", a named class or not), lists whose class is an
            // import node, others with a slot 14 result
            let push_list = |t: &mut Twins, desc: u32| -> u32 {
                let list = t.junk(0x50, 7);
                t.w32(list + 0x2c, desc);
                let tail = t.r32(lists + 4);
                t.w32(list, lists);
                t.w32(list + 4, tail);
                t.w32(tail, list);
                t.w32(lists + 4, list);
                list
            };
            let count = 1 + case % 4;
            let stride = 0x10u32;
            let buffer = t.zeros((stride * (count + 2)) as usize);
            for i in 0..count {
                let e = buffer + stride * i;
                let d = if (i + case) % 4 == 0 {
                    let j = t.junk(0xa0, 8);
                    t.w32(j + 0x40, 0);
                    j
                } else {
                    named
                };
                t.w32(e + 4, d);
                let s = if (i + case) % 2 == 0 { str_a } else { str_b };
                t.w32(e + 8, s | (rng.next() & 1));
            }
            let il = push_list(&mut t, import);
            t.w32(il + 0x34, buffer);
            t.w32(il + 0x3c, count);
            t.w32(il + 0x40, 1);
            let end = if count >= 3 {
                let e = buffer + stride;
                t.w32(e, buffer + 3 * stride);
                e
            } else {
                buffer + 0x1000
            };
            t.w32(il + 8, end);
            t.w32(import + 0x1c, stride);
            t.w32(import + 0x8c, 4);
            for &n in &nodes {
                push_list(&mut t, n);
            }
            let mut cslots = vec![(0u32, 0u32); 15];
            cslots[14] = (1, if case % 8 == 7 { 0x15 } else { 0 });
            let cls = p1d_vobj(&mut t, 0x7a00, &cslots, 0xa0);
            t.w32(cls + 0x40, 0);
            push_list(&mut t, cls);
            push_list(&mut t, dynamic);
            let what = format!("{label}: fixup instances {case}");
            t.check(&what, 0x0044_1620, Conv::Thiscall(cluster), &[], |g| {
                let it = g.scratch();
                Out::U32(pfx::fixup_instances(g, it, cluster))
            });
            t.reset();
        }
    }
}

// ---- continuation round 3, batch 4 (ffx_pload::p1_lists) ----

#[test]
fn p1d_read_instance_lists_matches_the_original() {
    use ffx_pload::p1_lists::bridge as pli;
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0x115_7001);
        for case in 0..48u32 {
            let simple = case % 2 == 0;
            // the stream's read fails at its `fail`th call (1..5; none
            // past the calls), readListData at its `lfail`th
            let fail = 1 + case / 2 % 5;
            let lfail = 1 + case / 10 % 4;
            for (sym, n, at, bad) in [
                (0x0057_dd30u32, 2u32, fail, 9u32),
                (0x0057_b270, 3, lfail, 0x13),
            ] {
                let ok = t.stub_code(sym, true, n, n, 0);
                let no = t.stub_code(sym | 1, true, n, n, bad);
                let calls = t.zeros(4);
                let code = t.a.alloc_code(&[0x90; 32]).unwrap();
                assert_eq!(t.b.alloc_code(&[0x90; 32]).unwrap(), code);
                let mut c = vec![0xff, 0x05];
                c.extend(calls.to_le_bytes()); // inc dword [calls]
                c.extend([0x83, 0x3d]);
                c.extend(calls.to_le_bytes());
                c.push(at as u8); // cmp dword [calls], at
                c.extend([0x75, 0x05]); // jne +5
                c.push(0xe9);
                let j = no.wrapping_sub(code + c.len() as u32 + 4);
                c.extend(j.to_le_bytes()); // jmp no
                c.push(0xe9);
                let j = ok.wrapping_sub(code + c.len() as u32 + 4);
                c.extend(j.to_le_bytes()); // jmp ok
                t.write(code, &c);
                let addr = t.rebase(sym);
                let mut jmp = vec![0xe9];
                jmp.extend(code.wrapping_sub(addr + 5).to_le_bytes());
                let old = Oracle::read(&mut t.a, addr, 5).unwrap();
                t.saved.entry(addr).or_insert(old);
                t.write(addr, &jmp);
            }
            t.stub(0x0057_e160, true, 1, 1, 0);
            t.stub(0x0057_8300, false, 2, 0, 0);
            let block = if case % 7 == 6 { 0 } else { t.zeros(0x40) };
            t.stub(0x0042_fb80, false, 2, 0, block);
            t.stub(0x0042_fb20, false, 1, 0, 0);
            // the mapping's class entries (+0x1c set or not)
            let map = t.junk(0x20, 1);
            t.w32(
                map + 0x1c,
                if simple {
                    0
                } else {
                    [1, 0x8000_0004, 0x10][(case % 3) as usize]
                },
            );
            let entries = t.zeros(32 * 4);
            for i in 0..4 {
                t.w32(entries + 32 * i + 0x1c, (rng.next() & 1) * 7);
            }
            t.w32(map + 0xc, 4 | (rng.next() & 0x8000_0000));
            t.w32(map + 0x10, entries);
            let n = case % 5;
            let header = t.junk(0x40, 2);
            t.w32(header + 0x10, n);
            t.w32(header + 0x38, 0x80);
            let cluster = t.junk(0x30, 3);
            let arena = if case % 9 == 4 { 0 } else { t.junk(8, 4) };
            t.w32(cluster + 0x24, arena);
            let hdrs = t.zeros(0x24 * 5);
            let lists = t.zeros(4 * 5);
            for i in 0..n {
                t.w32(hdrs + 0x24 * i, 1 + rng.next() % 4);
                t.w32(hdrs + 0x24 * i + 8, 0x10 + i);
                let list = if rng.next().is_multiple_of(4) {
                    0
                } else {
                    t.junk(0x40, i as u8)
                };
                t.w32(lists + 4 * i, list);
            }
            let reader = t.junk(0x30, 5);
            t.w32(reader, cluster);
            t.w32(reader + 4, map);
            t.w32(reader + 8, header);
            t.w32(reader + 0x14, hdrs);
            let kind: usize = match n {
                0 if case % 3 == 0 => 0,
                0 => 2,
                1 => 1,
                _ => 0,
            };
            t.w32(reader + 0x18, [n.max(2), 1, 0][kind]);
            match kind {
                0 => t.w32(reader + 0x1c, lists),
                1 => {
                    let one = t.r32(lists);
                    t.w32(reader + 0x1c, one);
                }
                _ => {}
            }
            let stream = t.junk(0x20, 6);
            let size = [0x40u32, 0, 0x8000_0040][(case / 3 % 3) as usize];
            let cursor0 = 0x0012_3450 + case;
            // the original's argument word `cursor`: three words, the
            // return address and ebp above its frame
            let ebp = t.a.stack_range().end - 0x100 - 12 - 8;
            let slot = ebp + 0xc;
            t.b.write(slot, &cursor0.to_le_bytes()).unwrap();
            t.b.set_call_top(Some(ebp - 0x200));
            let what = format!("{label}: read instance lists {case}");
            t.check(
                &what,
                0x0057_c940,
                Conv::Thiscall(reader),
                &[stream, cursor0, size],
                |g| Out::U32(pli::read_instance_lists(g, reader, stream, slot, size)),
            );
            t.b.set_call_top(None);
            t.reset();
        }
    }
}
