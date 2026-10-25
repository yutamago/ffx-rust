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
            Out::U32(s::writer_close(g, reader))
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
