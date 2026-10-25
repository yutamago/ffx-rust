//! `PhyreEngine`'s `PAnimation` code (`ffx_panim`, M4 port round 1, list
//! `p2-anim`) against the original: on the image alone with synthetic
//! data, and on a snapshot (`FFX_SNAPSHOT`) with the same data over the
//! snapshot's memory. Results (all of `eax` where the original defines
//! it) and every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! Callees that stay original (`Phyre::PhyreFree`, the type registry,
//! other classes' methods) are replaced in both oracles alike by
//! recording stubs: each call appends its tag, `ecx` and arguments to a
//! log in memory and returns a known `eax`, so the diff checks that both
//! made the same calls in the same order with the same `this`.

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
use ffx_panim::array::{self, Old, bridge as arr};
use ffx_panim::controller::{self, bridge as ctl};
use ffx_panim::descriptor::{self, bridge as desc};

/// `graphicFontDraw`'s body: no test here calls it, so the recording
/// stubs are written over it.
const CAVE: u32 = 0x0064_0660;
const CAVE_LEN: u32 = 1768;

/// What the `PhyreFree` stub returns (its low byte is overwritten by the
/// callers' `mov al, 1`).
const FREE_EAX: u32 = 0x5a5a_5a5a;

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
    /// Nothing the callers read (or `eax` as the caller left it).
    None,
    /// `eax`.
    U32(u32),
    /// `al`.
    U8(u8),
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 8 words (tag, `this`, six
    /// arguments).
    log: u32,
    cave: u32,
    cave_next: u32,
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let cave = a.rebase(CAVE);
        let mut t = Self {
            a,
            b,
            log: 0,
            cave,
            cave_next: cave,
        };
        t.log = t.zeros(0x8000);
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
            Self::new(Oracle::load(&exe).unwrap(), Oracle::load(&exe).unwrap()),
        )];
        if let Some(snap) = std::env::var_os("FFX_SNAPSHOT") {
            let pages = Pages::read(PathBuf::from(snap)).unwrap();
            out.push((
                "snapshot",
                Self::new(
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
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

    /// `n` bytes of a fixed pattern (so a missing write shows).
    fn junk(&mut self, rng: &mut Rng, n: usize) -> u32 {
        let bytes: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
        self.alloc(&bytes)
    }

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    /// Writes a recording stub over the original at `sym` (a `jmp`
    /// there): logs `sym`, `ecx` when `this` (callees that read it) and
    /// `n` stack arguments, pops `pops` words, returns `ret`.
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) -> u32 {
        let at = self.rebase(sym);
        let addr = self.stub_code(sym, this, n, pops, ret);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
        addr
    }

    /// The stub of [`Twins::stub`] in the cave only (for pointers), `tag`
    /// its log tag. Returns its address.
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
        let addr = self.cave_next;
        assert!(addr + c.len() as u32 <= self.cave + CAVE_LEN, "cave full");
        self.write(addr, &c);
        self.cave_next = addr + c.len() as u32;
        addr
    }

    /// Code in the cave (heap memory does not execute).
    fn code(&mut self, c: &[u8]) -> u32 {
        let addr = self.cave_next;
        assert!(addr + c.len() as u32 <= self.cave + CAVE_LEN, "cave full");
        self.write(addr, c);
        self.cave_next = addr + c.len() as u32;
        addr
    }

    fn log_entries(&self) -> u32 {
        Mem::read(&self.b, self.log)
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
}

// ---- batch 1: the class-descriptor callbacks ----

/// `mov al, 1; ret 4`.
const TRUE: [u32; 10] = [
    0x0050_8c40,
    0x0050_8c80,
    0x0050_8c90,
    0x0051_10b0,
    0x0051_10c0,
    0x0051_1110,
    0x0051_1120,
    0x0052_0740,
    0x0053_a890,
    0x0053_a8d0,
];
/// `xor eax, eax; ret 4`.
const ZERO: [u32; 7] = [
    0x0050_8cb0,
    0x0050_ab80,
    0x0050_c5b0,
    0x0050_e720,
    0x0050_f730,
    0x0051_1190,
    0x0051_cc40,
];
/// Two zero words.
const INIT2: [u32; 18] = [
    0x0050_a950,
    0x0051_6100,
    0x0051_6120,
    0x0051_c8a0,
    0x0051_c8c0,
    0x0051_c8e0,
    0x0052_06a0,
    0x0052_06e0,
    0x0052_a020,
    0x0052_a040,
    0x0052_a0e0,
    0x0052_ca20,
    0x0053_a680,
    0x0053_a6a0,
    0x0053_a6c0,
    0x0053_a6e0,
    0x0051_10d0,
    0x0052_06c0,
];
/// `PArray` destruct with the null check.
const FREE: [u32; 9] = [
    0x0050_a9a0,
    0x0051_c980,
    0x0052_0700,
    0x0052_0750,
    0x0052_a100,
    0x0053_a790,
    0x0053_a7d0,
    0x0053_a810,
    0x0053_a850,
];
const PHYRE_FREE: u32 = descriptor::PHYRE_FREE;

#[test]
fn constant_callbacks_match_the_original() {
    for (what, mut t) in Twins::all() {
        let mut rng = Rng(0x1234_5678);
        for &f in &TRUE {
            for _ in 0..4 {
                let p = rng.next();
                t.check(&format!("{what} {f:#x}"), f, Conv::Stdcall, &[p], |_| {
                    Out::U8(1)
                });
            }
        }
        for &f in &ZERO {
            for _ in 0..4 {
                let p = rng.next();
                t.check(&format!("{what} {f:#x}"), f, Conv::Stdcall, &[p], |_| {
                    Out::U32(0)
                });
            }
        }
    }
}

#[test]
fn construct_callbacks_match_the_original() {
    let more: [(u32, &[u32]); 5] = [
        (0x0050_a970, &[0; 3]),
        (0x0050_8c50, &[0, 3, 0, 0]),
        (0x0051_c900, &[0; 6]),
        (0x0053_a740, &[0; 6]),
        (0x0051_6140, &[]),
    ];
    for (what, mut t) in Twins::all() {
        let mut rng = Rng(0x0bad_cafe);
        let cases: Vec<(u32, Option<&[u32]>)> = INIT2
            .iter()
            .map(|&f| (f, Some(&[0u32; 2][..])))
            .chain(
                more.iter()
                    .map(|&(f, w)| (f, (f != 0x0051_6140).then_some(w))),
            )
            .collect();
        for (f, words) in cases {
            for i in 0..4 {
                let p = if i == 0 { 0 } else { t.junk(&mut rng, 0x40) };
                t.check(
                    &format!("{what} {f:#x} p={p:#x}"),
                    f,
                    Conv::Stdcall,
                    &[p],
                    |g| {
                        Out::U32(match words {
                            Some(w) => desc::init_words(g, p, w),
                            None => desc::clip_init(g, p),
                        })
                    },
                );
            }
        }
    }
}

/// A `PArray` header at a fresh address: `count`, `els`.
fn array(t: &mut Twins, rng: &mut Rng, count: u32, els: u32) -> u32 {
    let p = t.junk(rng, 0x20);
    t.w32(p, count);
    t.w32(p + 4, els);
    p
}

/// A count: owned or not, 0, 1, 2 or larger.
fn count(rng: &mut Rng) -> u32 {
    let n = [0, 1, 2, 3, rng.below(1000)][rng.below(5) as usize];
    if rng.chance(2) { n | 0x8000_0000 } else { n }
}

#[test]
fn destruct_callbacks_match_the_original() {
    for (what, mut t) in Twins::all() {
        // PhyreFree writes ecx before it reads it
        t.stub(PHYRE_FREE, false, 1, 0, FREE_EAX);
        let mut rng = Rng(0xfeed_f00d);
        // (address, offset of the array, null check, shared)
        let mut cases: Vec<(u32, u32, bool, bool)> =
            FREE.iter().map(|&f| (f, 0, true, false)).collect();
        cases.extend([
            (0x0050_a9e0, 4, true, false),
            (0x0051_62d0, 0, false, false),
            (0x0052_ca40, 0, false, false),
            (0x0051_c9c0, 0, false, true),
        ]);
        let mut freed = 0;
        for (f, off, null_check, shared) in cases {
            for i in 0..12 {
                let els = if i % 3 == 0 {
                    0
                } else {
                    0x1000_0000 + rng.below(0x1000) * 4
                };
                let n = count(&mut rng);
                let base = array(&mut t, &mut rng, 0, 0);
                t.w32(base + off, n);
                t.w32(base + off + 4, els);
                let before = t.log_entries();
                t.check(
                    &format!("{what} {f:#x} n={n:#x} els={els:#x}"),
                    f,
                    Conv::Stdcall,
                    &[base],
                    |g| {
                        let at = base + off;
                        Out::U32(if shared {
                            desc::free_owned_shared(g, at)
                        } else {
                            desc::free_owned(g, at, null_check)
                        })
                    },
                );
                freed += t.log_entries() - before;
            }
        }
        assert!(freed > 20, "{what}: PhyreFree reached {freed} times");
    }
}

/// An array of `n` elements of `stride` bytes with flagged string words
/// at `off`.
fn elements(t: &mut Twins, rng: &mut Rng, n: u32, stride: u32, off: u32) -> u32 {
    let els = t.junk(rng, (n * stride + 4) as usize);
    for i in 0..n {
        let s = 0x2000_0000 + rng.below(0x100) * 2 + u32::from(rng.chance(2));
        t.w32(els + i * stride + off, s);
    }
    els
}

#[test]
fn element_arrays_are_freed_as_the_original() {
    for (what, mut t) in Twins::all() {
        // PhyreFree writes ecx before it reads it
        t.stub(PHYRE_FREE, false, 1, 0, FREE_EAX);
        let mut rng = Rng(0x5eed_0001);
        for (f, owner, stride, off) in [
            (0x0051_a890, 0x0051_c940, 0x1c, 0x14),
            (0x0052_8a30, 0x0052_a140, 8, 4),
        ] {
            for i in 0..10 {
                let n = rng.below(5);
                let els = if i == 0 {
                    0
                } else {
                    elements(&mut t, &mut rng, n, stride, off)
                };
                let before = t.log_entries();
                t.check(
                    &format!("{what} {f:#x} n={n}"),
                    f,
                    Conv::Cdecl,
                    &[els, n],
                    |g| match desc::free_elements(g, els, n, stride, off) {
                        Some(eax) => Out::U32(eax),
                        None => Out::None,
                    },
                );
                if els != 0 {
                    assert!(t.log_entries() > before, "{what} {f:#x}: nothing freed");
                }
                let c = if rng.chance(3) { n | 0x8000_0000 } else { n };
                let els2 = if i == 1 {
                    0
                } else {
                    elements(&mut t, &mut rng, n, stride, off)
                };
                let p = array(&mut t, &mut rng, c, els2);
                t.check(
                    &format!("{what} {owner:#x} n={c:#x}"),
                    owner,
                    Conv::Stdcall,
                    &[p],
                    |g| Out::U32(desc::free_owned_elements(g, p, stride, off)),
                );
            }
        }
    }
}

#[test]
fn flagged_strings_are_freed_as_the_original() {
    for (what, mut t) in Twins::all() {
        // PhyreFree writes ecx before it reads it
        t.stub(PHYRE_FREE, false, 1, 0, FREE_EAX);
        let mut rng = Rng(0x0f1a_9000);
        for (f, off, clear) in [
            (0x0050_c590, 0x14, None),
            (0x0052_a1b0, 4, None),
            (0x0050_d740, 0x14, Some(0x1c)),
        ] {
            for _ in 0..6 {
                let p = t.junk(&mut rng, 0x40);
                let s = 0x3000_0000 + rng.below(0x100) * 2 + u32::from(rng.chance(2));
                t.w32(p + off, s);
                t.check(
                    &format!("{what} {f:#x} s={s:#x}"),
                    f,
                    Conv::Stdcall,
                    &[p],
                    |g| Out::U8(u8::from(desc::free_flagged(g, p, off, clear))),
                );
            }
        }
    }
}

// ---- batch 2: virtual destructs, method callbacks, registrations ----

/// Destruct callbacks through the object's virtual destructor.
const VIRTUAL: [u32; 11] = [
    0x0052_20a0,
    0x0052_2c60,
    0x0052_3810,
    0x0052_4c70,
    0x0052_61a0,
    0x0052_a180,
    0x0052_e1f0,
    0x0052_f6e0,
    0x0053_0620,
    0x0053_1480,
    0x0053_a8a0,
];

#[test]
fn virtual_destructs_match_the_original() {
    for (what, mut t) in Twins::all() {
        let mut rng = Rng(0xdead_0001);
        for &f in &VIRTUAL {
            for _ in 0..3 {
                // the deleting destructor: thiscall(this, flags), pops 1
                let ret = rng.next();
                let dtor = t.stub_code(0xd700_0000 | f, true, 1, 1, ret);
                let vtable = t.zeros(8);
                t.w32(vtable, dtor);
                let obj = t.junk(&mut rng, 0x20);
                t.w32(obj, vtable);
                let p = obj + 4;
                let before = t.log_entries();
                t.check(&format!("{what} {f:#x}"), f, Conv::Stdcall, &[p], |g| {
                    Out::U32(desc::destroy_virtual(g, p))
                });
                assert_eq!(t.log_entries(), before + 1, "{what} {f:#x}");
            }
            t.cave_next = t.cave;
        }
    }
}

/// (callback, method it runs, null check)
const METHOD_CALLS: [(u32, u32, bool); 6] = [
    (0x0050_f710, 0x0050_d1d0, false),
    (0x0051_6310, 0x0051_3240, false),
    (0x0051_6330, 0x0051_33f0, false),
    (0x0051_ca00, 0x0051_9380, false),
    (0x0053_a700, 0x0053_4360, true),
    (0x0053_a720, 0x0053_4450, true),
];

#[test]
fn method_callbacks_match_the_original() {
    for (what, mut t) in Twins::all() {
        let mut rng = Rng(0x00c0_ffee);
        for (f, method, null_check) in METHOD_CALLS {
            t.stub(method, true, 0, 0, 0x7700_0000 | (f & 0xffff));
            for i in 0..4 {
                let p = if null_check && i == 0 {
                    0
                } else {
                    t.junk(&mut rng, 0x20)
                };
                let before = t.log_entries();
                t.check(
                    &format!("{what} {f:#x} p={p:#x}"),
                    f,
                    Conv::Stdcall,
                    &[p],
                    |g| {
                        if null_check {
                            Out::U8(u8::from(desc::call_on_nonnull(g, p, method)))
                        } else {
                            Out::U32(desc::call_on(g, p, method))
                        }
                    },
                );
                assert_eq!(t.log_entries(), before + u32::from(p != 0), "{what} {f:#x}");
            }
        }
    }
}

#[test]
fn registrations_match_the_original() {
    for (what, mut t) in Twins::all() {
        t.stub(descriptor::REGISTER_TYPE, false, 1, 0, 0x1111_1111);
        t.stub(descriptor::ADD_TO_NAMESPACE, true, 0, 0, 0x2222_2222);
        for (f, d) in [
            (0x0050_8450, 0x00ca_9810),
            (0x0052_1300, 0x00ca_b858),
            (0x0053_6210, 0x00ca_cdb0),
        ] {
            t.check(&format!("{what} {f:#x}"), f, Conv::Cdecl, &[], |g| {
                Out::U32(desc::register(g, d))
            });
            assert!(t.log_entries() >= 2);
        }
    }
}

// ---- batch 2: PArray setters, dynamic member callbacks ----

#[test]
fn array_setters_match_the_original() {
    let setters = [
        (0x0050_aff0, Old::Free),
        (0x0051_dba0, Old::Free),
        (0x0052_aae0, Old::Free),
        (0x0051_7380, Old::FreeAlways),
        (0x0051_db50, Old::Targets),
    ];
    for (what, mut t) in Twins::all() {
        t.stub(PHYRE_FREE, false, 1, 0, FREE_EAX);
        t.stub(array::CHANNEL_DTOR, true, 0, 0, 0x0d70_0000);
        let mut rng = Rng(0x5e77_e400);
        let mut freed = 0;
        for (f, old) in setters {
            for i in 0..16 {
                let n = rng.below(4);
                let prev = match i % 4 {
                    0 => 0,
                    _ => elements(&mut t, &mut rng, n, 0x1c, 0x14),
                };
                let flags = if rng.chance(3) { n | 0x8000_0000 } else { n };
                let a = array(&mut t, &mut rng, flags, prev);
                let els = if i % 5 == 1 {
                    prev
                } else {
                    0x4000_0000 + rng.below(0x100) * 4
                };
                let count = if rng.chance(4) {
                    0x8000_0000 | rng.below(10)
                } else {
                    rng.below(10)
                };
                let before = t.log_entries();
                t.check(
                    &format!("{what} {f:#x} {i}"),
                    f,
                    Conv::Thiscall(a),
                    &[els, count],
                    |g| Out::U32(arr::set_borrowed(g, a, els, count, old)),
                );
                freed += t.log_entries() - before;
            }
        }
        // PArray<PAnimationChannel>: element destructors, then the array
        for i in 0..24 {
            let n = rng.below(4);
            let prev = if i % 4 == 0 {
                0
            } else {
                t.junk(&mut rng, (n * 0x34 + 4) as usize)
            };
            let flags = if rng.chance(3) { n | 0x8000_0000 } else { n };
            let a = array(&mut t, &mut rng, flags, prev);
            let els = if i % 5 == 1 {
                prev
            } else {
                0x4000_0000 + rng.below(0x100) * 4
            };
            let count = if rng.chance(3) {
                0x8000_0000 | rng.below(10)
            } else {
                rng.below(10)
            };
            let before = t.log_entries();
            if i % 2 == 0 {
                t.check(
                    &format!("{what} assign {i}"),
                    0x0051_7270,
                    Conv::Thiscall(a),
                    &[els, count],
                    |g| Out::U32(arr::assign_channels(g, a, els, count)),
                );
            } else {
                t.check(
                    &format!("{what} set channels {i}"),
                    0x0051_73d0,
                    Conv::Thiscall(a),
                    &[els, count],
                    |g| Out::U32(arr::set_channels(g, a, els, count)),
                );
            }
            freed += t.log_entries() - before;
        }
        assert!(freed > 30, "{what}: {freed} calls");
    }
}

#[test]
fn dynamic_member_callbacks_match_the_original() {
    for (what, mut t) in Twins::all() {
        let mut rng = Rng(0x00d1_ca11);
        for f in [
            0x0050_b040,
            0x0051_73f0,
            0x0051_7420,
            0x0051_dc40,
            0x0051_dc70,
            0x0052_ab80,
        ] {
            for i in 0..3 {
                let desc = t.junk(&mut rng, 0x60);
                let target = if i == 0 {
                    0
                } else {
                    t.stub_code(0xf3c0_0000 | i, true, 2, 2, rng.next())
                };
                t.w32(desc + 0x3c, target);
                let (obj, a, b) = (rng.next(), rng.next(), rng.next());
                t.check(
                    &format!("{what} {f:#x} {i}"),
                    f,
                    Conv::Thiscall(desc),
                    &[obj, a, b],
                    |g| Out::U32(arr::forward_3c(g, desc, obj, a, b)),
                );
            }
        }
        for f in [0x0051_66f0, 0x0051_cf10] {
            for i in 0..4 {
                let desc = t.junk(&mut rng, 0x60);
                // the fallback reads [desc+0x24] - [class+0x88] + i
                let class = t.junk(&mut rng, 0x100);
                let table = t.junk(&mut rng, 0x40);
                t.w32(desc + 0xc, class);
                t.w32(class + 0x88, 0x100);
                t.w32(desc + 0x24, table + 0x100);
                let idx = rng.below(8) * 4;
                // a callee that logs ecx (i) and edx (desc)
                let target = if i < 2 {
                    0
                } else {
                    stub_edx(&mut t, rng.next())
                };
                t.w32(desc + 0x38, target);
                t.check(
                    &format!("{what} {f:#x} {i}"),
                    f,
                    Conv::Thiscall(desc),
                    &[idx],
                    |g| Out::U32(arr::forward_38(g, desc, idx)),
                );
            }
        }
        for i in 0..3 {
            let desc = t.junk(&mut rng, 0x60);
            let target = if i == 0 {
                0
            } else {
                t.stub_code(0xf340_0000 | i, true, 0, 0, rng.next())
            };
            t.w32(desc + 0x34, target);
            let obj = rng.next();
            t.check(
                &format!("{what} 0x51d0b0 {i}"),
                0x0051_d0b0,
                Conv::Thiscall(desc),
                &[obj],
                |g| Out::U32(arr::forward_34(g, desc, obj)),
            );
        }
        t.cave_next = t.cave;
    }
}

/// A stub that logs `ecx` and `edx` (as its first argument word) and
/// returns `ret` (no stack arguments).
fn stub_edx(t: &mut Twins, ret: u32) -> u32 {
    let log = t.log;
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(0xed00_0000u32.to_le_bytes()); // mov [eax], tag
    c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
    c.extend([0x89, 0x50, 0x08]); // mov [eax+8], edx
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    c.push(0xb8);
    c.extend(ret.to_le_bytes()); // mov eax, ret
    c.push(0xc3);
    let addr = t.cave_next;
    t.write(addr, &c);
    t.cave_next = addr + c.len() as u32;
    addr
}

// ---- batch 2: controller constructors and destructors ----

#[test]
fn controller_constructors_match_the_original() {
    type Ctor = fn(&mut G, u32) -> u32;
    let ctors: [(u32, Ctor); 9] = [
        (0x0052_11f0, |g, p| ctl::hierarchy_node_ctor(g, p)),
        (0x0052_1a50, |g, p| ctl::time_controller_ctor(g, p)),
        (0x0052_25a0, |g, p| ctl::time_interval_ctor(g, p)),
        (0x0052_31e0, |g, p| ctl::time_scale_offset_ctor(g, p)),
        (0x0052_3d20, |g, p| ctl::data_source_ctor(g, p)),
        (0x0052_5240, |g, p| ctl::controller_ctor(g, p)),
        (0x0052_b820, |g, p| ctl::blender_ctor(g, p)),
        (0x0052_d660, |g, p| ctl::weighted_blender_ctor(g, p)),
        (0x0053_0dd0, |g, p| ctl::target_blender_ctor(g, p)),
    ];
    for (what, mut t) in Twins::all() {
        let mut rng = Rng(0x00c7_0000);
        for (f, ctor) in ctors {
            for _ in 0..2 {
                let p = t.junk(&mut rng, 0x40);
                t.check(&format!("{what} {f:#x}"), f, Conv::Thiscall(p), &[], |g| {
                    Out::U32(ctor(g, p))
                });
            }
        }
    }
}

#[test]
fn controller_destructors_match_the_original() {
    use controller::vtable as vt;
    use ctl::Dtor;
    type Dt = fn(&mut G, u32);
    let dtors: [(u32, Dt); 6] = [
        (0x0052_1220, |g, p| ctl::hierarchy_node_dtor(g, p)),
        (0x0052_2600, |g, p| {
            ctl::dtor_over_node(g, p, vt::TIME_INTERVAL);
        }),
        (0x0052_3240, |g, p| {
            ctl::dtor_over_node(g, p, vt::TIME_SCALE_OFFSET);
        }),
        (0x0052_3d60, |g, p| ctl::data_source_dtor(g, p)),
        (0x0052_52c0, |g, p| {
            ctl::dtor_over_data_source(g, p, vt::CONTROLLER);
        }),
        (0x0053_0e40, |g, p| {
            ctl::dtor_over_data_source(g, p, vt::TARGET_BLENDER);
        }),
    ];
    let deleting = [
        (0x0052_1b80, Dtor::Node),
        (0x0052_26d0, Dtor::OverNode(vt::TIME_INTERVAL)),
        (0x0052_3310, Dtor::OverNode(vt::TIME_SCALE_OFFSET)),
        (0x0052_4740, Dtor::OverDataSource(vt::EVENT_CONTROLLER)),
        (0x0052_5390, Dtor::OverDataSource(vt::CONTROLLER)),
        (0x0052_8180, Dtor::Original(controller::dtor::SLOT_FILTER)),
        (0x0053_0180, Dtor::OverDataSource(vt::SPU_TARGET_BLENDER)),
        (0x0053_0f20, Dtor::OverDataSource(vt::TARGET_BLENDER)),
        (
            0x0053_57d0,
            Dtor::Original(controller::dtor::NETWORK_INSTANCE),
        ),
        (
            0x0053_f3d0,
            Dtor::Original(controller::dtor::ANIMATABLE_COMPONENT),
        ),
    ];
    for (what, mut t) in Twins::all() {
        t.stub(PHYRE_FREE, false, 1, 0, FREE_EAX);
        for f in [
            controller::dtor::SLOT_FILTER,
            controller::dtor::NETWORK_INSTANCE,
            controller::dtor::ANIMATABLE_COMPONENT,
        ] {
            t.stub(f, true, 0, 0, 0x0d7d_0000);
        }
        let mut rng = Rng(0x00d7_0000);
        for (f, dtor) in dtors {
            let p = t.junk(&mut rng, 0x40);
            t.check(&format!("{what} {f:#x}"), f, Conv::Thiscall(p), &[], |g| {
                dtor(g, p);
                Out::None
            });
        }
        for (f, dtor) in deleting {
            for flags in [0, 1, 2, 3, 0xffff_fffe, 0xffff_ffff] {
                let p = t.junk(&mut rng, 0x40);
                let before = t.log_entries();
                t.check(
                    &format!("{what} {f:#x} flags={flags:#x}"),
                    f,
                    Conv::Thiscall(p),
                    &[flags],
                    |g| Out::U32(ctl::deleting_dtor(g, p, flags, dtor)),
                );
                let extra = u32::from(matches!(dtor, Dtor::Original(_)));
                assert_eq!(
                    t.log_entries(),
                    before + (flags & 1) + extra,
                    "{what} {f:#x}"
                );
            }
        }
    }
}

// ---- batch 3: controller construct callbacks, channels, blender destructors ----

#[test]
fn controller_construct_callbacks_match_the_original() {
    type Ctor = fn(&mut G, u32) -> u32;
    let cbs: [(u32, Ctor); 10] = [
        (0x0052_2040, |g, p| ctl::time_controller_ctor(g, p)),
        (0x0052_2bf0, |g, p| ctl::time_interval_ctor(g, p)),
        (0x0052_37a0, |g, p| ctl::time_scale_offset_ctor(g, p)),
        (0x0052_4c00, |g, p| ctl::event_controller_ctor(g, p)),
        (0x0052_6130, |g, p| ctl::controller_ctor(g, p)),
        (0x0052_e180, |g, p| ctl::weighted_blender_ctor(g, p)),
        (0x0052_f670, |g, p| ctl::additive_blender_ctor(g, p)),
        (0x0053_05c0, |g, p| ctl::spu_target_blender_ctor(g, p)),
        (0x0053_1420, |g, p| ctl::target_blender_ctor(g, p)),
        (0x0052_a060, |g, p| ctl::slot_filter_ctor(g, p)),
    ];
    for (what, mut t) in Twins::all() {
        let mut rng = Rng(0x00cb_0000);
        for (f, ctor) in cbs {
            for i in 0..3 {
                let p = if i == 0 { 0 } else { t.junk(&mut rng, 0x40) };
                t.check(
                    &format!("{what} {f:#x} p={p:#x}"),
                    f,
                    Conv::Stdcall,
                    &[p],
                    |g| Out::U8(u8::from(ctl::construct(g, p, ctor))),
                );
            }
        }
    }
}

#[test]
fn channel_callbacks_match_the_original() {
    for (what, mut t) in Twins::all() {
        t.stub(PHYRE_FREE, false, 1, 0, FREE_EAX);
        t.stub(descriptor::CHANNEL_BASE_CTOR, true, 0, 0, 0x0c70_0000);
        t.stub(descriptor::CHANNEL_BASE_DTOR, true, 0, 0, 0x0d70_0000);
        let mut rng = Rng(0x00c4_a000);
        for i in 0..4 {
            let p = if i == 0 { 0 } else { t.junk(&mut rng, 0x40) };
            for (f, words) in [(0x0050_c510, 6), (0x0050_d6b0, 8)] {
                let p2 = if p == 0 { 0 } else { t.junk(&mut rng, 0x40) };
                t.check(
                    &format!("{what} {f:#x} {i}"),
                    f,
                    Conv::Stdcall,
                    &[p2],
                    |g| Out::U8(u8::from(desc::construct_described(g, p2, words))),
                );
            }
            for f in [0x0050_e5e0, 0x0050_f6b0] {
                let p2 = if p == 0 { 0 } else { t.junk(&mut rng, 0x40) };
                t.check(
                    &format!("{what} {f:#x} {i}"),
                    f,
                    Conv::Stdcall,
                    &[p2],
                    |g| Out::U8(u8::from(desc::construct_channel(g, p2))),
                );
            }
        }
        for i in 0..8 {
            let p = t.junk(&mut rng, 0x40);
            let n = count(&mut rng);
            t.w32(p + 0x28, n);
            t.w32(p + 0x2c, if i % 3 == 0 { 0 } else { 0x1000_0000 + i });
            t.check(
                &format!("{what} 0x50e650 {i}"),
                0x0050_e650,
                Conv::Stdcall,
                &[p],
                |g| Out::U32(desc::destroy_channel(g, p)),
            );
        }
    }
}

#[test]
fn list_entry_and_word_copies_match_the_original() {
    for (what, mut t) in Twins::all() {
        let mut rng = Rng(0x0ce0_0000);
        for _ in 0..3 {
            let p = t.junk(&mut rng, 0xa0);
            t.check(
                &format!("{what} 0x534360"),
                0x0053_4360,
                Conv::Thiscall(p),
                &[],
                |g| Out::U32(ffx_panim::network::bridge::list_entry_ctor(g, p)),
            );
        }
        // PhyreMalloc: a stub handing out fresh zeroed blocks in turn
        let pool = t.zeros(0x400);
        let next = t.zeros(4);
        t.w32(next, pool);
        let mut c = vec![0xa1];
        c.extend(next.to_le_bytes()); // mov eax, [next]
        c.extend([0x8b, 0x4c, 0x24, 0x04]); // mov ecx, [esp+4]
        c.extend([0x01, 0x0d]);
        c.extend(next.to_le_bytes()); // add [next], ecx
        c.push(0xc3);
        let malloc = t.code(&c);
        let at = t.rebase(array::PHYRE_MALLOC);
        let mut j = vec![0xe9];
        j.extend(malloc.wrapping_sub(at + 5).to_le_bytes());
        t.write(at, &j);
        for n in [0, 1, 2, 5, 9] {
            let src = t.junk(&mut rng, 0x40);
            t.check(
                &format!("{what} 0x5281e0 n={n}"),
                0x0052_81e0,
                Conv::Cdecl,
                &[n, src],
                |g| Out::U32(arr::copy_words(g, n, src)),
            );
        }
        // a failed allocation
        let zero = t.code(&[0x31, 0xc0, 0xc3]); // xor eax, eax; ret
        let mut j = vec![0xe9];
        j.extend(zero.wrapping_sub(at + 5).to_le_bytes());
        t.write(at, &j);
        let src = t.junk(&mut rng, 0x10);
        t.check(
            &format!("{what} 0x5281e0 no memory"),
            0x0052_81e0,
            Conv::Cdecl,
            &[3, src],
            |g| Out::U32(arr::copy_words(g, 3, src)),
        );
    }
}

#[test]
fn blender_and_set_destructors_match_the_original() {
    use controller::vtable as vt;
    for (what, mut t) in Twins::all() {
        t.stub(PHYRE_FREE, false, 1, 0, FREE_EAX);
        let mut rng = Rng(0x00b1_e4d0);
        let mut calls = 0;
        for i in 0..10 {
            // an object with owned and borrowed arrays at +0..+0x1c
            let p = t.junk(&mut rng, 0x40);
            let arrays: &[u32] = match i % 7 {
                0 => &[0xc],
                2 => &[0xc, 0x18],
                3 => &[0, 8, 0x10],
                _ => &[0xc, 0x14],
            };
            for &off in arrays {
                let n = rng.below(3);
                let flags = if rng.chance(3) { n | 0x8000_0000 } else { n };
                let els = if i % 4 == 0 {
                    0
                } else {
                    elements(&mut t, &mut rng, n, 0x1c, 0x14)
                };
                t.w32(p + off, flags);
                t.w32(p + off + 4, els);
            }
            let before = t.log_entries();
            match i % 7 {
                0 => t.check(
                    &format!("{what} blender {i}"),
                    0x0052_b8e0,
                    Conv::Thiscall(p),
                    &[],
                    |g| {
                        ctl::blender_dtor(g, p);
                        Out::None
                    },
                ),
                1 => t.check(
                    &format!("{what} weighted {i}"),
                    0x0052_d6b0,
                    Conv::Thiscall(p),
                    &[],
                    |g| {
                        ctl::blender_subclass_dtor(g, p, vt::WEIGHTED_BLENDER);
                        Out::None
                    },
                ),
                2 => t.check(
                    &format!("{what} slot filter {i}"),
                    0x0052_7ba0,
                    Conv::Thiscall(p),
                    &[],
                    |g| {
                        ctl::slot_filter_dtor(g, p);
                        Out::None
                    },
                ),
                3 => t.check(
                    &format!("{what} set {i}"),
                    0x0051_9380,
                    Conv::Thiscall(p),
                    &[],
                    |g| {
                        ctl::animation_set_dtor(g, p);
                        Out::None
                    },
                ),
                k => {
                    let (f, v) = if k % 2 == 0 {
                        (0x0052_d940, vt::WEIGHTED_BLENDER)
                    } else {
                        (0x0052_edd0, vt::ADDITIVE_BLENDER)
                    };
                    let flags = rng.next();
                    t.check(
                        &format!("{what} {f:#x} {i}"),
                        f,
                        Conv::Thiscall(p),
                        &[flags],
                        |g| Out::U32(ctl::blender_subclass_deleting_dtor(g, p, flags, v)),
                    );
                }
            }
            calls += t.log_entries() - before;
        }
        assert!(calls > 10, "{what}: {calls} frees");
    }
}
