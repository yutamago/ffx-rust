//! `PhyreEngine`'s type registry and loading code, list
//! `phyre-registry-load-a` (`ffx_pload::m5p_registry_load_a*`, M5 stage
//! 1), against the original. Almost none of this code ran in a coverage
//! run, so every case is synthetic memory built for its branch, on the
//! image alone and over a snapshot (`FFX_SNAPSHOT`, run once with each of
//! field.snap and battle.snap). Results and every byte written must agree.
//! Needs `FFX_GAME_DIR`.
//!
//! Callees outside the list run as they are where they are plain code,
//! or are replaced in both oracles alike by recording stubs: each call
//! appends its tag, `ecx` and its stack words to a log in memory, so the
//! diff checks that both made the same calls in the same order.

#![allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    dead_code
)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pload::m5p_registry_load_a_convert as cv;

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
    /// The call log: a count, then entries of 8 words (tag, `this`, six
    /// arguments).
    log: u32,
    /// The bytes each stub's `jmp` replaced, by address.
    saved: HashMap<u32, Vec<u8>>,
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        a.set_max_instructions(20_000_000);
        b.set_max_instructions(20_000_000);
        let mut t = Self {
            a,
            b,
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

    fn w8(&mut self, at: u32, v: u8) {
        self.write(at, &[v]);
    }

    fn r32(&self, at: u32) -> u32 {
        Mem::read(&self.a, at)
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
    /// words, returning `rets[i]`, or `0x1000 + i` past its end). Returns
    /// the object (0x40 bytes, after its vtable pointer, zero).
    fn object(&mut self, words: &[u32], rets: &[u32]) -> u32 {
        let mut vt = Vec::new();
        for (i, &n) in words.iter().enumerate() {
            let r = rets.get(i).copied().unwrap_or(0x1000 + i as u32);
            let s = self.stub_code(0x7000 + i as u32, true, n, n, r);
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
}

// ---- the data-conversion kernels ----

/// Source floats for the `float` kernels: signs, fractions either side of
/// .5, the integer types' limits, 2^31/2^32/2^63/2^64, huge, infinities,
/// quiet NaNs with payloads, a denormal. No signalling NaNs (the emulator
/// keeps them signalling, the CPU quiets them).
const FLOATS: [u32; 28] = [
    0x0000_0000,
    0x8000_0000,
    0x3f80_0000, // 1
    0xbfc0_0000, // -1.5
    0x3f33_3333, // 0.7
    0xbf33_3333, // -0.7
    0x437f_e666, // 255.9
    0x4380_0000, // 256
    0xc301_0000, // -129
    0x477f_ffb3, // 65535.7
    0x4780_0000, // 65536
    0xc700_0100, // -32769
    0x4f00_0000, // 2^31
    0xcf00_0000, // -2^31
    0xcf00_0001, // below -2^31
    0x4f80_0000, // 2^32
    0x4f86_8c20, // 4.5e9
    0x5f00_0000, // 2^63
    0x5eff_ffff, // just below 2^63
    0xdf00_0000, // -2^63
    0xdf00_0001, // below -2^63
    0x5f80_0000, // 2^64
    0x7149_f2ca, // 1e30
    0x7f80_0000, // inf
    0xff80_0000, // -inf
    0x7fc0_0000, // NaN
    0xffc1_2345, // -NaN with a payload
    0x0000_0001, // denormal
];

/// Writes the kernel's `desc` and `ctx` and its source bytes.
struct KernelCase {
    src: u32,
    dst: u32,
    desc: u32,
    ctx: u32,
}

impl KernelCase {
    fn new(t: &mut Twins) -> Self {
        let src = t.zeros(0x400);
        let dst = t.zeros(0x400);
        let desc = t.zeros(0x10);
        let ctx = t.zeros(0x10);
        let m1 = t.zeros(0x40);
        let m2 = t.zeros(0x40);
        t.w32(ctx + 4, m1);
        t.w32(ctx + 8, m2);
        Self {
            src,
            dst,
            desc,
            ctx,
        }
    }

    /// One call: `count` elements, the strides, the direction, the two
    /// offsets; the source from `rng` (floats from [`FLOATS`] for the
    /// `float` kernels), the destination filled with junk.
    #[allow(clippy::too_many_arguments)]
    fn run(
        &self,
        t: &mut Twins,
        label: &str,
        k: u32,
        elem: cv::Elem,
        count: u32,
        strides: (u32, u32),
        forward: u8,
        offsets: (u32, u32),
        rng: &mut Rng,
    ) {
        let float_src = matches!(
            elem,
            cv::Elem::F32ToF32 | cv::Elem::F32ToF64 | cv::Elem::FromF32(_)
        );
        let mut src = vec![0u8; 0x400];
        for (i, w) in src.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let v = if float_src {
                FLOATS[(rng.next() as usize + i) % FLOATS.len()]
            } else {
                rng.next()
            };
            w.copy_from_slice(&v.to_le_bytes());
        }
        t.write(self.src, &src);
        let junk: Vec<u8> = (0..0x400u32).map(|i| (i as u8) ^ 0xa5).collect();
        t.write(self.dst, &junk);
        t.w32(self.desc, count);
        t.w32(self.desc + 4, strides.0);
        t.w32(self.desc + 8, strides.1);
        t.w8(self.desc + 0xc, forward);
        let m1 = t.r32(self.ctx + 4);
        let m2 = t.r32(self.ctx + 8);
        t.w32(m1 + 8, offsets.0);
        t.w32(m2 + 0x24, offsets.1);
        let what = format!(
            "{label}: kernel {k:#x} {elem:?} n={count} strides={strides:?} fwd={forward} off={offsets:?}"
        );
        let (s, d, desc, ctx) = (self.src, self.dst, self.desc, self.ctx);
        t.check(&what, k, Conv::Cdecl, &[s, d, desc, ctx], |g| {
            let (s_end, d_end) = cv::bridge::convert(g, elem, s, d, desc, ctx);
            if cv::EAX_SOURCE_END.contains(&k) {
                Out::U32(s_end)
            } else if cv::EAX_DEST_END.contains(&k) {
                Out::U32(d_end)
            } else {
                Out::None
            }
        });
    }
}

#[test]
fn conversion_kernels_match_the_original() {
    assert_eq!(cv::KERNELS.len(), 83);
    for (label, mut t) in Twins::all() {
        let case = KernelCase::new(&mut t);
        let mut rng = Rng(0x1357_9bdf);
        for &(k, _, _, elem) in cv::KERNELS {
            // nothing, one, the unrolled four plus a rest, both directions
            for (count, strides, fwd, offs) in [
                (0, (4, 4), 1, (0, 0)),
                (1, (4, 16), 0, (8, 0)),
                (4, (16, 16), 1, (4, 0x20)),
                (7, (4, 16), 0, (0x10, 4)),
                (13, (8, 24), 1, (0, 0x18)),
                (28, (4, 16), 1, (0, 0)),
                (28, (4, 16), 0, (0, 0)),
            ] {
                case.run(&mut t, label, k, elem, count, strides, fwd, offs, &mut rng);
            }
            // strides of zero and below the element size
            for (count, strides) in [(5, (0, 0)), (9, (1, 1)), (6, (3, 2))] {
                case.run(&mut t, label, k, elem, count, strides, 1, (0, 0), &mut rng);
            }
        }
    }
}

#[test]
fn conversion_kernels_keep_their_types() {
    // the table names each kernel's (destination, source) pair once
    let mut seen = std::collections::HashSet::new();
    for &(k, dst, src, _) in cv::KERNELS {
        assert!(seen.insert((dst, src)), "{k:#x}: pair twice");
        assert_eq!(cv::kernel(k).map(|_| ()), Some(()));
    }
}
