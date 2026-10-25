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
