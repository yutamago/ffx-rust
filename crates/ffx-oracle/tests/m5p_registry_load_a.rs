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
    clippy::many_single_char_names
)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pload::m5p_registry_load_a_convert as cv;
use ffx_pload::m5p_registry_load_a_desc::{self as dc, bridge as dcb};
use ffx_pload::m5p_registry_load_a_misc::{self as ms, bridge as msb};
use ffx_pload::m5p_registry_load_a_ptype::{self as pt, Kind, bridge as ptb};

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
    fn call_st0(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> f32 {
        let conv = match conv {
            host::Conv::Cdecl => Conv::Cdecl,
            host::Conv::Stdcall => Conv::Stdcall,
            host::Conv::Thiscall(t) => Conv::Thiscall(t),
            host::Conv::Fastcall => Conv::Fastcall,
        };
        self.0
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .st0_f32()
            .unwrap_or_else(|| panic!("callee {addr:#x} left no ST0"))
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
    /// `edx:eax`.
    U64(u64),
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

    /// A recording stub (as [`Twins::stub_code`], cdecl) that leaves the
    /// `float` with `bits` in `ST0`, placed at the original `sym`.
    fn stub_float(&mut self, sym: u32, n: u32, bits: u32) {
        let at = self.rebase(sym);
        let cell = self.alloc(&bits.to_le_bytes());
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], sym
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.extend([0xd9, 0x05]);
        c.extend(cell.to_le_bytes()); // fld dword [cell]
        c.push(0xc3);
        let code = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), code);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
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
            Out::U64(v) => assert_eq!(
                u64::from(ra.edx) << 32 | u64::from(ra.eax),
                v,
                "{what}: edx:eax"
            ),
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

// ---- PType and PTypeDefault<T> (batch 2) ----

#[test]
fn ptype_destructors_match_the_original() {
    for (label, mut t) in Twins::all() {
        let obj = t.junk(0x30, 7);
        // PType::~PType: the vtable only, eax left alone
        let what = format!("{label}: ~PType");
        t.check(&what, 0x0043_7700, Conv::Thiscall(obj), &[], |g| {
            ptb::ptype_dtor(g, obj);
            Out::None
        });
        t.stub(pt::FREE, false, 1, 0, 0xf4ee);
        let dtors = std::iter::once(pt::PTYPE_DTOR).chain(pt::TYPES.iter().map(|ty| ty.dtor));
        for d in dtors {
            for flags in [0, 1, 2, 3, 0xffff_fffe, 0xffff_ffff] {
                let what = format!("{label}: deleting dtor {d:#x}({flags:#x})");
                t.check(&what, d, Conv::Thiscall(obj), &[flags], |g| {
                    Out::U32(ptb::delete_type(g, obj, flags))
                });
            }
        }
        t.reset();
    }
}

#[test]
fn ptype_constructor_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let obj = t.junk(0x40, 9);
        for (a, b, c) in [(0, 0, 0), (0x00b0_0000, 4, 0xffff_ffff), (1, 2, 3)] {
            let what = format!("{label}: PType({a:#x}, {b:#x}, {c:#x})");
            t.check(&what, 0x0043_8cc0, Conv::Thiscall(obj), &[a, b, c], |g| {
                Out::U32(ptb::construct(g, obj, a, b, c))
            });
        }
    }
}

#[test]
fn ptype_fixups_match_the_original() {
    for (label, mut t) in Twins::all() {
        let desc = t.zeros(0x100);
        let name = t.alloc(b"PSomeType\0");
        let empty = t.alloc(b"\0");
        let outs = t.junk(0x10, 3);
        for (has_desc, flags, nm) in [
            (false, 0u8, name),
            (true, 0, name),
            (true, 1, name),
            (true, 0xfe, empty),
            (true, 0x81, empty),
        ] {
            let d = if has_desc { desc } else { 0 };
            t.w8(desc + 0x90, flags);
            let ty = t.object(&[0, 0], &[0, d]);
            t.w32(ty + 0x18, nm);
            let what = format!("{label}: GetFixupForPTypePointer desc={d:#x} flags={flags:#x}");
            t.check(
                &what,
                0x0043_7740,
                Conv::Cdecl,
                &[ty, outs, outs + 4],
                |g| Out::U32(ptb::get_fixup(g, ty, outs, outs + 4)),
            );
        }
        // the dispenser lookup, stubbed: found and not found
        for found in [0u32, 0x00c0_1234] {
            t.stub(pt::DISPENSER_INSTANCE, false, 0, 0, 0x00c9_05f0);
            t.stub(pt::DISPENSER_FIND, true, 1, 1, found);
            for nm in [0, name] {
                let what = format!("{label}: ResolveFixupForPTypePointer({nm:#x}) -> {found:#x}");
                t.check(&what, 0x0043_7830, Conv::Cdecl, &[outs, nm], |g| {
                    Out::U32(ptb::resolve_fixup(g, outs, nm))
                });
            }
            t.reset();
        }
    }
}

#[test]
fn ptype_name_hash_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0x2468_ace1);
        for case in 0..40u32 {
            let len = (rng.next() % 24) as usize;
            let mut bytes: Vec<u8> = (0..len).map(|_| (rng.next() % 255 + 1) as u8).collect();
            bytes.push(0);
            bytes.extend([0x41, 0x42, 0]);
            let s = t.alloc(&bytes);
            let seed = if case % 3 == 0 { 0x7b5 } else { rng.next() };
            for n in [0, len as u32, (len as u32).min(3), len as u32 + 2] {
                for p in [s, 0] {
                    let what = format!("{label}: name hash {case} n={n} p={p:#x}");
                    t.check(&what, 0x0043_77d0, Conv::Cdecl, &[p, seed, n], |g| {
                        Out::U32(ptb::name_hash(g, p, seed, n))
                    });
                }
            }
        }
    }
}

#[test]
fn ptype_swap64_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0x0bad_cafe);
        for i in 0..200 {
            let (lo, hi) = match i {
                0 => (0, 0),
                1 => (u32::MAX, u32::MAX),
                2 => (0x8000_0000, 0x8000_0000),
                3 => (0x0102_0304, 0x8506_0708),
                _ => (rng.next(), rng.next()),
            };
            for f in [0x0043_9230, 0x0043_92c0] {
                let what = format!("{label}: swap64 {f:#x}({lo:#x}, {hi:#x})");
                t.check(&what, f, Conv::Cdecl, &[lo, hi], |_| {
                    Out::U64(ptb::swap64(lo, hi))
                });
            }
        }
    }
}

/// Doubles and floats for the swaps and the Lua conversions (no signalling
/// NaNs: the emulator keeps them signalling).
const SWAP_BYTES: [[u8; 16]; 6] = [
    [0; 16],
    [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
        0x80,
    ],
    [
        0x00, 0x00, 0xc0, 0x7f, 0x00, 0x00, 0xf8, 0x7f, 0x34, 0x12, 0xc0, 0xff, 0, 0, 0, 0,
    ],
    [
        0x00, 0x00, 0x80, 0x3f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf0, 0x3f, 1, 2, 3, 4,
    ],
    [
        0xff, 0xff, 0x7f, 0xff, 0xff, 0xff, 0xef, 0xff, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00,
        0x80,
    ],
    [
        0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x5a, 0x5a, 0x5a,
        0x5a,
    ],
];

#[test]
fn ptype_swap_endian_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let p = t.zeros(0x20);
        for ty in &pt::TYPES {
            for (i, bytes) in SWAP_BYTES.iter().enumerate() {
                t.write(p, bytes);
                for n in [0, 1, 5] {
                    let what = format!(
                        "{label}: {} swapEndian {:#x} case {i} n={n}",
                        ty.name, ty.swap
                    );
                    t.check(&what, ty.swap, Conv::Stdcall, &[p, n], |g| {
                        Out::U32(ptb::swap_endian(g, ty.kind, p))
                    });
                }
            }
        }
    }
}

#[test]
fn ptype_script_values_match_the_original() {
    let ints = [
        0u32,
        1,
        0x7f,
        0x80,
        0xff,
        0x7fff,
        0x8000,
        0xffff,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
        0x1234_5678,
    ];
    let floats = [
        0u32,
        0x8000_0000,
        0x3f80_0000,
        0xc2f6_e979,
        0x7f80_0000,
        0x7fc0_0000,
        0xffc1_2345,
        0x0000_0001,
    ];
    for (label, mut t) in Twins::all() {
        let l = t.junk(0x40, 0x11);
        let out = t.zeros(0x20);
        for ty in &pt::TYPES {
            // getValueFromScriptContext: the integer / number / boolean on
            // top, popped, stored
            let values: &[u32] = match ty.kind {
                Kind::F32 | Kind::F64 => &floats,
                _ => &ints,
            };
            for &v in values {
                match ty.kind {
                    Kind::F32 | Kind::F64 => t.stub_float(pt::LUA_TONUMBERX, 3, v),
                    Kind::Bool => t.stub(pt::LUA_TOBOOLEAN, false, 2, 0, v),
                    _ => t.stub(pt::LUA_TOINTEGERX, false, 3, 0, v),
                }
                t.stub(pt::LUA_SETTOP, false, 2, 0, 0x5e70_0000 ^ v);
                t.write(out, &[0xa5; 0x20]);
                let what = format!("{label}: {} get {:#x} value {v:#x}", ty.name, ty.get);
                t.check(&what, ty.get, Conv::Stdcall, &[l, out], |g| {
                    Out::U32(ptb::get_from_script(g, ty.kind, l, out))
                });
                t.reset();
            }
            // putValueToScriptContext: the value read at its width, pushed
            for bytes in &SWAP_BYTES {
                t.stub(pt::LUA_CHECKSTACK, false, 2, 0, 1);
                t.stub(pt::LUA_PUSHINTEGER, false, 2, 0, 0x0001_1111);
                t.stub(pt::LUA_PUSHNUMBER, false, 2, 0, 0x0002_2222);
                t.stub(pt::LUA_PUSHBOOLEAN, false, 2, 0, 0x0003_3333);
                t.write(out, bytes);
                let what = format!("{label}: {} put {:#x} {bytes:x?}", ty.name, ty.put);
                t.check(&what, ty.put, Conv::Stdcall, &[l, out], |g| {
                    Out::U32(ptb::put_to_script(g, ty.kind, l, out))
                });
                t.reset();
            }
        }
    }
}

// ---- batch 3: tail destructors, setters, iterators, utility lists ----

#[test]
fn misc_tail_destructors_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(ms::DESCRIPTOR_DESTRUCT, true, 0, 0, 0xde57);
        t.stub(ms::MEMBER_DTOR, true, 0, 0, 0xde58);
        let obj = t.junk(0x40, 5);
        for d in &ms::TAIL_DTORS {
            let what = format!("{label}: {} dtor {:#x}", d.class, d.addr);
            t.check(&what, d.addr, Conv::Thiscall(obj), &[], |g| {
                Out::U32(msb::tail_dtor(g, d, obj))
            });
        }
        assert_eq!(t.log_entries(), ms::TAIL_DTORS.len() as u32);
        t.reset();
        // PIClassDataMemberDynamicType's dtor and the unlock jump
        let member = t.junk(0x40, 6);
        let ty = t.junk(0x10, 8);
        t.w32(ty + 4, member);
        let what = format!("{label}: PIClassDataMemberDynamicType dtor");
        t.check(&what, 0x0057_54f0, Conv::Thiscall(ty), &[], |g| {
            Out::U32(msb::dynamic_type_dtor(g, ty))
        });
        t.stub(ms::CS_UNLOCK, true, 0, 0, 0x0c5);
        let holder = t.zeros(8);
        t.w32(holder, 0x00c9_1234);
        let what = format!("{label}: unlock through the holder");
        t.check(&what, 0x0045_c0b0, Conv::Thiscall(holder), &[], |g| {
            Out::U32(msb::unlock_held(g, holder))
        });
        t.reset();
    }
}

#[test]
fn misc_float_setters_match_the_original() {
    for (label, mut t) in Twins::all() {
        let obj = t.junk(0x20, 1);
        for (f, off) in ms::FLOAT_SETTERS {
            for v in FLOATS {
                let what = format!("{label}: setter {f:#x} {v:#x}");
                t.check(&what, f, Conv::Thiscall(obj), &[v], |g| {
                    Out::U32(msb::set_float(g, obj, off, v))
                });
            }
        }
    }
}

/// An owner whose list (head at `+0x1c`, linked through the elements'
/// first word) holds four elements for the wanted class `want`: no
/// match, a match with count 0, a match through a second chain node, a
/// match. Returns the owner and the elements (as `p1_load.rs`'s).
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
    t.w32(e[0] + 0x2c, other);
    t.w32(e[1] + 0x2c, want);
    t.w32(other + 0x40, 0);
    t.w32(e[3] + 0x2c, want);
    t.w32(e[2] + 0x40, 2);
    t.w32(e[3] + 0x40, 1);
    let other2 = t.zeros(0x50);
    t.w32(e[2] + 0x2c, other2);
    t.w32(other2 + 0x40, want);
    (owner, [e[0], e[1], e[2], e[3]])
}

#[test]
fn misc_iterators_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(0x0044_1dc0, true, 1, 1, 0x77);
        let want = t.zeros(0x50);
        let (owner, e) = iter_owner(&mut t, want);
        let end = owner + 0x1c;
        for sym in ms::ITER_NEXTS {
            for cur in [e[0], e[1], e[2], e[3], 0] {
                let it = t.junk(0x20, 1);
                t.w32(it + 0x14, end);
                t.w32(it + 0x18, cur);
                t.w32(it + 0x1c, want);
                let what = format!("{label}: iterNext {sym:#x} {cur:#x}");
                t.check(&what, sym, Conv::Thiscall(it), &[], |g| {
                    Out::U32(ffx_pload::cluster::bridge::iter_next(g, it))
                });
            }
        }
        t.reset();
    }
}

/// A list at `head` (an `ffx.sym` address) of `n` fresh nodes linked
/// through `+link`; returns the nodes.
fn chain(t: &mut Twins, head: u32, n: usize, link: u32) -> Vec<u32> {
    let nodes: Vec<u32> = (0..n).map(|_| t.junk(0x20, 0x33)).collect();
    let at = t.rebase(head);
    t.w32(at, nodes.first().copied().unwrap_or(0));
    for (i, &x) in nodes.iter().enumerate() {
        t.w32(x + link, nodes.get(i + 1).copied().unwrap_or(0));
    }
    nodes
}

#[test]
fn misc_utility_lists_match_the_original() {
    for (label, mut t) in Twins::all() {
        // push_front
        let head = t.junk(4, 1);
        let x = t.junk(8, 2);
        let what = format!("{label}: push front");
        t.check(&what, 0x0043_20a0, Conv::Thiscall(x), &[head], |g| {
            Out::U32(msb::push_front(g, x, head))
        });
        // the unlinks: first, middle, last, absent, empty list
        for (f, head) in [
            (0x0047_fe10, ms::GEOMETRY_UTILITIES),
            (0x0047_fe60, ms::GEOMETRY_UTILITIES),
            (0x004c_1410, ms::RENDERING_UTILITIES),
            (0x004c_1460, ms::RENDERING_UTILITIES),
        ] {
            for (n, pick) in [(3, 0), (3, 1), (3, 2), (2, 9), (0, 9)] {
                let nodes = chain(&mut t, head, n, 4);
                let x = nodes.get(pick).copied().unwrap_or(0x0123_4560);
                if f == 0x004c_1460 {
                    let idx = (pick as u32) * 3 + 1;
                    let at = t.rebase(ms::RENDERING_SLOTS) + 4 * idx;
                    t.w32(at, 0xdead_0000 + idx);
                    if nodes.get(pick).is_none() {
                        // an absent node still has an index word
                        let fake = t.zeros(0x10);
                        t.w32(fake + 8, idx);
                        let what = format!("{label}: unlink {f:#x} absent");
                        t.check(&what, f, Conv::Cdecl, &[fake], |g| {
                            Out::U32(if f == 0x004c_1460 {
                                msb::unlink_rendering(g, fake)
                            } else {
                                msb::unlink(g, head, fake)
                            })
                        });
                        continue;
                    }
                    t.w32(x + 8, idx);
                }
                let what = format!("{label}: unlink {f:#x} n={n} pick={pick}");
                t.check(&what, f, Conv::Cdecl, &[x], |g| {
                    Out::U32(if f == 0x004c_1460 {
                        msb::unlink_rendering(g, x)
                    } else {
                        msb::unlink(g, head, x)
                    })
                });
            }
        }
        // PUtility's deleting destructors
        t.stub(ms::OPERATOR_DELETE, false, 1, 0, 0xde1);
        for f in [0x0043_1e10, 0x0043_1e70] {
            for (n, pick) in [(3, 0), (3, 2), (2, 9), (0, 9)] {
                for flags in [0, 1, 0xfe] {
                    let nodes = chain(&mut t, ms::UTILITIES, n, 4);
                    let x = match nodes.get(pick) {
                        Some(&x) => x,
                        None => t.junk(0x10, 4),
                    };
                    let what =
                        format!("{label}: PUtility dtor {f:#x} n={n} pick={pick} flags={flags:#x}");
                    t.check(&what, f, Conv::Thiscall(x), &[flags], |g| {
                        Out::U32(msb::delete_utility(g, x, flags))
                    });
                }
            }
        }
        t.reset();
    }
}

#[test]
fn misc_shutdown_steps_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(ms::TREE_CLEAR, true, 0, 0, 0x7c1);
        t.stub(ms::SERIALIZATION_OBJECT_DTOR, true, 0, 0, 0x0d7);
        t.stub(ms::FREE, false, 1, 0, 0xf4e);
        let flag = t.rebase(ms::KERNELS_REGISTERED);
        t.w8(flag, 1);
        let what = format!("{label}: clear kernels");
        t.check(&what, 0x0047_a210, Conv::Cdecl, &[], |g| {
            Out::U32(msb::clear_kernels(g))
        });
        for n in [0, 1, 3] {
            chain(&mut t, ms::SERIALIZATION_OBJECTS, n, 0);
            t.w8(flag, 1);
            let ready = t.rebase(ms::SERIALIZATION_READY);
            t.w8(ready, 1);
            let what = format!("{label}: serialization shutdown n={n}");
            t.check(&what, 0x0043_2150, Conv::Cdecl, &[], |g| {
                Out::U32(msb::shutdown_serialization(g))
            });
        }
        t.reset();
        // the namespace: remove one descriptor, then clear a namespace of
        // n descriptors whose remove (stubbed) unlinks the first
        let ns = t.zeros(0x40);
        let remove = [
            0x8b, 0x44, 0x24, 0x04, // mov eax, [esp+4]  (desc)
            0x8b, 0x50, 0x2c, // mov edx, [eax+0x2c]  (its next)
            0x89, 0x51, 0x08, // mov [ecx+8], edx  (the namespace's first)
            0xb8, 0x11, 0x22, 0x00, 0x00, // mov eax, 0x2211
            0xc2, 0x04, 0x00, // ret 4
        ];
        let code = t.a.alloc_code(&remove).unwrap();
        assert_eq!(t.b.alloc_code(&remove).unwrap(), code);
        let at = t.rebase(ms::NAMESPACE_REMOVE);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut t.a, at, 5).unwrap();
        t.saved.entry(at).or_insert(old);
        t.write(at, &j);
        let desc = t.junk(0x100, 9);
        t.w32(desc + 0x3c, ns);
        let what = format!("{label}: remove from namespace");
        t.check(&what, 0x0043_d420, Conv::Thiscall(desc), &[], |g| {
            Out::U32(msb::remove_from_namespace(g, desc))
        });
        t.stub(ms::GLOBAL_NAMESPACE, false, 0, 0, ns);
        for n in [0usize, 1, 3] {
            let descs: Vec<u32> = (0..n).map(|_| t.junk(0x100, 0x21)).collect();
            let head = ns + 8;
            t.w32(head, descs.first().map_or(head, |d| d + 0x2c));
            for (i, &d) in descs.iter().enumerate() {
                t.w32(d + 0x3c, ns);
                t.w32(d + 0x2c, descs.get(i + 1).map_or(head, |e| e + 0x2c));
            }
            t.w32(ns + 0x10, 0x5555);
            let what = format!("{label}: clear global namespace n={n}");
            t.check(&what, 0x0043_e610, Conv::Cdecl, &[], |g| {
                Out::U32(msb::clear_global_namespace(g))
            });
        }
        // a list ending in 0 and one whose element is the descriptor at 0
        for last in [0u32, 0x2c] {
            t.w32(ns + 8, last);
            let what = format!("{label}: clear global namespace, ends in {last:#x}");
            t.check(&what, 0x0043_e610, Conv::Cdecl, &[], |g| {
                Out::U32(msb::clear_global_namespace(g))
            });
        }
        t.reset();
        // the pool lock and the thread count
        t.stub(ms::CS_RELEASE, true, 0, 0, 0x4e1);
        for held in [0u8, 1, 0x80] {
            let at = t.rebase(ms::POOL_LOCK);
            t.w8(at, held);
            let what = format!("{label}: release pool lock {held}");
            t.check(&what, 0x0045_da00, Conv::Cdecl, &[], |g| {
                Out::U32(msb::release_pool_lock(g))
            });
        }
        t.stub(ms::SET_THREAD_COUNT, false, 1, 0, 0x7c);
        for n in [0, 4, u32::MAX] {
            let what = format!("{label}: set thread count {n}");
            t.check(&what, 0x0043_1170, Conv::Cdecl, &[n], |g| {
                Out::U32(msb::set_thread_count(g, n))
            });
        }
        t.reset();
    }
}

// ---- batch 4: descriptors, members, PClassDescriptorDynamic ----

/// Words each slot of a recording descriptor object takes (slot 0: 2, 1:
/// 0, 4 and 6: 2, 11: 1, 12: 0, 13: 3, 14: 1).
const DESC_WORDS: [u32; 15] = [2, 0, 0, 0, 2, 0, 2, 0, 0, 0, 0, 1, 0, 3, 1];

#[test]
fn desc_releases_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(dc::FREE, false, 1, 0, 0xf4ee_0000);
        for f in [0x0043_b360, 0x0044_77d0] {
            for (head, data) in [
                (0u32, 0u32),
                (3, 0x0012_3450),
                (0x8000_0003, 0x0012_3450),
                (0x7fff_ffff, 0),
            ] {
                let arr = t.zeros(8);
                t.w32(arr, head);
                t.w32(arr + 4, data);
                let what = format!("{label}: array release {f:#x} {head:#x} {data:#x}");
                t.check(&what, f, Conv::Thiscall(arr), &[], |g| {
                    Out::U32(dcb::array_release(g, arr))
                });
            }
        }
        for flags in [0u8, 1, 0xfe] {
            let a = t.junk(0x20, 1);
            let b = t.junk(0x20, 2);
            let node = t.junk(0x20, 3);
            t.w32(node, a);
            t.w32(node + 4, b);
            t.w8(node + 0x10, flags);
            let what = format!("{label}: node release {flags:#x}");
            t.check(&what, 0x0043_b3c0, Conv::Thiscall(node), &[], |g| {
                Out::U32(dcb::node_release(g, node))
            });
        }
        t.stub(dc::DESCRIPTOR_DESTRUCT, true, 0, 0, 0xde57);
        t.stub(dc::ANNOTATION_DTOR, true, 0, 0, 0xa22);
        t.stub(dc::OPERATOR_DELETE, false, 1, 0, 0xde1);
        let obj = t.junk(0x40, 4);
        for flags in [0, 1, 2, 0xffff_ffff] {
            for (f, vt) in [(0x0043_b810, false), (0x0043_b840, true)] {
                let what = format!("{label}: descriptor delete {f:#x} {flags:#x}");
                t.check(&what, f, Conv::Thiscall(obj), &[flags], |g| {
                    Out::U32(dcb::delete_descriptor(g, obj, flags, vt))
                });
            }
            let what = format!("{label}: annotation delete {flags:#x}");
            t.check(&what, 0x0044_7ab0, Conv::Thiscall(obj), &[flags], |g| {
                Out::U32(dcb::delete_annotation(g, obj, flags))
            });
        }
        t.reset();
    }
}

#[test]
fn desc_queries_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the class-name fixup
        let name = t.alloc(b"PClassName\0");
        let outs = t.junk(8, 1);
        for flags in [0u8, 1, 2, 0xff] {
            let desc = t.zeros(0x100);
            t.w8(desc + 0x90, flags);
            t.w32(desc + 0x18, name);
            let what = format!("{label}: class name fixup {flags:#x}");
            t.check(
                &what,
                0x0043_bc10,
                Conv::Cdecl,
                &[desc, outs, outs + 4],
                |g| Out::U32(dcb::class_name_fixup(g, desc, outs, outs + 4)),
            );
        }
        // the validation buffer: computed / in the namespace / neither
        t.stub(dc::CALCULATE_DEFAULT, true, 0, 0, 0xca1);
        t.stub(dc::CALCULATE_VALIDATION, true, 0, 0, 0xca2);
        for flags in [0x8000_0000u32, 0x4000_0000, 0, 0xc000_0000] {
            let desc = t.object(&DESC_WORDS, &[]);
            t.w32(desc + 0x90, flags);
            t.w32(desc + 0x68, 0x0bad_0068);
            let what = format!("{label}: validation buffer {flags:#x}");
            t.check(&what, 0x0043_cd80, Conv::Thiscall(desc), &[], |g| {
                Out::U32(dcb::validation_buffer(g, desc))
            });
        }
        // a member's size: pointer flag, kind-2 class with and without
        // bit 6, no class
        let ty = t.zeros(0x40);
        t.w32(ty + 0x1c, 0x24);
        for (flags, class_kind) in [
            (0u32, None),
            (2, None),
            (0, Some(2u32)),
            (0x40, Some(2)),
            (0x42, Some(2)),
            (0, Some(1)),
        ] {
            let class = match class_kind {
                Some(k) => {
                    let c = t.zeros(0x40);
                    t.w32(c + 0x30, k);
                    c
                }
                None => 0,
            };
            let member = t.object(&[0, 0], &[0, class]);
            t.w32(member + 0x18, ty);
            t.w32(member + 0x14, flags);
            let what = format!("{label}: member size {flags:#x} {class_kind:?}");
            t.check(&what, 0x0043_cd30, Conv::Thiscall(member), &[], |g| {
                Out::U32(dcb::member_size(g, member))
            });
        }
        t.reset();
    }
}

#[test]
fn desc_forwards_match_the_original() {
    for (label, mut t) in Twins::all() {
        // slot 0 over n elements, results with al 0 and 1
        for ret in [0x1234_5600u32, 0x1234_5601, 0x0000_0056] {
            let this = t.object(&DESC_WORDS, &[ret]);
            t.w32(this + 0x1c, 0x18);
            for n in [0u32, 1, 3] {
                let what = format!("{label}: for each {n} ret={ret:#x}");
                t.check(
                    &what,
                    0x0043_d650,
                    Conv::Thiscall(this),
                    &[0x0040_0000, n, 7],
                    |g| Out::U32(dcb::for_each_element(g, this, 0x0040_0000, n, 7)),
                );
            }
        }
        // the bound forwarder
        let obj = t.object(&DESC_WORDS, &[]);
        for flags in [0u8, 2, 0xfd] {
            let this = t.zeros(0x20);
            t.w32(this + 0x14, obj);
            t.w32(this + 0x18, 0x5151);
            t.w8(this + 0x10, flags);
            let what = format!("{label}: bound forward {flags:#x}");
            t.check(&what, 0x0043_7440, Conv::Thiscall(this), &[0x99], |g| {
                Out::U32(dcb::forward_bound(g, this, 0x99))
            });
        }
        // deleteObjectForScript
        t.stub(dc::CLUSTER_DELETE_OBJECT, true, 1, 1, 0x13);
        let cluster = t.zeros(0x20);
        for o in [0u32, 0x00c0_0000] {
            let what = format!("{label}: delete object for script {o:#x}");
            t.check(&what, 0x0044_1380, Conv::Thiscall(cluster), &[o], |g| {
                Out::U32(dcb::delete_object_for_script(g, cluster, o))
            });
        }
        // PClassDescriptorDynamic: base or not
        t.stub(dc::BASE_CHECK, false, 2, 0, 0x0b01);
        let base = t.object(&DESC_WORDS, &[]);
        for b in [0u32, base] {
            let this = t.zeros(0x100);
            t.w32(this + 0x40, b);
            t.w32(this + 0x44, this + 0x44);
            let what = format!("{label}: dynamic base check {b:#x}");
            t.check(&what, 0x0044_9480, Conv::Thiscall(this), &[5], |g| {
                Out::U32(dcb::base_check(g, this, 5))
            });
            let what = format!("{label}: dynamic slot 14 {b:#x}");
            t.check(&what, 0x0044_9670, Conv::Thiscall(this), &[5], |g| {
                Out::U32(dcb::forward_to_base(g, this, 14, &[5]))
            });
            let what = format!("{label}: dynamic slot 13 {b:#x}");
            t.check(&what, 0x0044_97c0, Conv::Thiscall(this), &[5, 6, 7], |g| {
                Out::U32(dcb::forward_to_base(g, this, 13, &[5, 6, 7]))
            });
        }
        t.reset();
    }
}

#[test]
fn desc_dynamic_members_match_the_original() {
    for (label, mut t) in Twins::all() {
        let base = t.object(&DESC_WORDS, &[]);
        // a member class descriptor whose slot 11 records the place
        let member_desc = t.object(&DESC_WORDS, &[]);
        t.w32(member_desc + 0x88, 0x30);
        for (b, n) in [(0u32, 0usize), (base, 0), (base, 3), (0, 2)] {
            let this = t.zeros(0x100);
            t.w32(this + 0x40, b);
            t.w32(this + 0x88, 0x10);
            let head = this + 0x44;
            // members: external, no class, a class
            let members: Vec<u32> = (0..n).map(|_| t.zeros(0x40)).collect();
            t.w32(head, members.first().map_or(head, |m| m + 4));
            for (i, &m) in members.iter().enumerate() {
                t.w32(m + 4, members.get(i + 1).map_or(head, |x| x + 4));
                t.w32(m + 0xc, this);
                t.w32(m + 0x24, 0x100 * (i as u32 + 1));
                t.w8(m + 0x14, if i == 0 { 2 } else { 0 });
                let ty = t.object(&[0, 0], &[0, if i == 1 { 0 } else { member_desc }]);
                t.w32(m + 0x18, ty);
            }
            let what = format!("{label}: dynamic destroy base={b:#x} n={n}");
            t.check(
                &what,
                0x0044_95b0,
                Conv::Thiscall(this),
                &[0x0050_0000],
                |g| Out::U32(dcb::destroy_dynamic(g, this, 0x0050_0000)),
            );
        }
        // object offsets and pushes
        t.stub(dc::LUA_CHECKSTACK, false, 2, 0, 1);
        t.stub(dc::LUA_PUSHNIL, false, 1, 0, 0x5e11);
        t.stub(dc::PUSH_OBJECT, false, 3, 0, 0x0b1e);
        let this = t.zeros(0x100);
        t.w32(this + 0x88, 0x20);
        let l = t.junk(0x10, 1);
        let cell = t.zeros(4);
        for v in [0u32, 0x1000, 0xffff_ffe0] {
            t.w32(cell, v);
            let what = format!("{label}: push object by pointer {v:#x}");
            t.check(&what, 0x0044_9ac0, Conv::Thiscall(this), &[l, cell], |g| {
                Out::U32(dcb::push_object_through(g, this, l, cell))
            });
            let what = format!("{label}: push object {v:#x}");
            t.check(&what, 0x0044_9b50, Conv::Thiscall(this), &[l, v], |g| {
                Out::U32(dcb::push_object(g, this, l, v))
            });
        }
        let out = t.junk(4, 9);
        for r in [0u32, 0x0040_0030] {
            t.stub(dc::OBJECT_GET, false, 1, 0, r);
            let what = format!("{label}: object offset {r:#x}");
            t.check(
                &what,
                0x0044_9840,
                Conv::Thiscall(this),
                &[0x77, out],
                |g| Out::U32(dcb::object_offset(g, this, 0x77, out)),
            );
        }
        t.reset();
        // the asset owner walk: no class, an unrelated chain, the class
        // itself, a child, with 0 and 3 instances
        let target = t.rebase(dc::ASSET_CLASS);
        let other = t.zeros(0x50);
        let child = t.zeros(0x50);
        t.w32(child + 0x40, target);
        t.w32(child + 0x1c, 0x30);
        for (class, n) in [
            (0u32, 2u32),
            (other, 2),
            (target, 0),
            (child, 0),
            (child, 3),
        ] {
            let obj = t.zeros(0x50);
            let inst = t.junk(0x100, 7);
            t.w32(obj + 0x2c, class);
            t.w32(obj + 0x3c, n);
            t.w32(obj + 0x30, inst);
            t.w32(obj + 0x28, 0x0a55_e700);
            let what = format!("{label}: asset owner {class:#x} n={n}");
            t.check(&what, 0x0045_da20, Conv::Cdecl, &[obj], |g| {
                Out::U32(dcb::set_asset_owner(g, obj))
            });
        }
    }
}

// ---- batch 5: an iterator seek, block copy, asset-import fixup ----

#[test]
fn desc_late_helpers_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the iterator seek copy at 0x00441b80 (check 0x00441e10)
        t.stub(0x0044_1dc0, true, 1, 1, 0x77);
        let want = t.zeros(0x50);
        let (owner, e) = iter_owner(&mut t, want);
        let end = owner + 0x1c;
        for cur in [e[0], e[1], e[2], e[3], 0] {
            let it = t.junk(0x20, 1);
            t.w32(it + 0x14, end);
            t.w32(it + 0x18, cur);
            t.w32(it + 0x1c, want);
            let what = format!("{label}: iterSeek 0x441b80 {cur:#x}");
            t.check(&what, 0x0044_1b80, Conv::Thiscall(it), &[], |g| {
                Out::U32(ffx_pload::cluster::bridge::iter_seek(g, it))
            });
        }
        t.reset();
        // the block copy: no size, no block, copies of 0, 3 and n bytes
        let src = t.alloc(b"abcdefghijklmnop");
        for (n, block, len) in [
            (0u32, 0x10u32, 3u32),
            (8, 0, 3),
            (8, 1, 0),
            (8, 1, 3),
            (16, 1, 16),
        ] {
            let at = if block == 0 { 0 } else { t.junk(0x20, 0x44) };
            t.stub(dc::MALLOC, false, 2, 0, at);
            let what = format!("{label}: copy to new block n={n} block={at:#x} len={len}");
            t.check(&what, 0x0044_7d50, Conv::Cdecl, &[n, src, len], |g| {
                Out::U32(dcb::copy_to_new_block(g, n, src, len))
            });
        }
        t.reset();
        // the asset-import fixup: no entry, no block, indices either sign
        let entry = t.zeros(0x40);
        t.w32(entry + 0x30, 0x0010_0000);
        let outs = t.junk(8, 3);
        for (found, block, x) in [
            (false, true, 0x0010_0000u32),
            (true, false, 0x0010_0000),
            (true, true, 0x0010_0000),
            (true, true, 0x0010_0000 + 12 * 0x1234),
            (true, true, 0x0010_0000 + 12 * 0x1_2345 + 7),
            (true, true, 0x000f_fff4),
            (true, true, 0x000f_0000 - 5),
        ] {
            t.stub(dc::FIND_BY_ID, false, 2, 0, if found { entry } else { 0 });
            let blk = if block { t.junk(4, 0x55) } else { 0 };
            t.stub(dc::MALLOC, false, 2, 0, blk);
            let what = format!("{label}: asset import fixup found={found} block={blk:#x} x={x:#x}");
            t.check(
                &what,
                0x0045_d300,
                Conv::Cdecl,
                &[x, outs, outs + 4, 0x0044_4440],
                |g| Out::U32(dcb::asset_import_fixup(g, x, outs, outs + 4, 0x0044_4440)),
            );
            t.reset();
        }
    }
}
