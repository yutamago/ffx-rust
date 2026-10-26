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
