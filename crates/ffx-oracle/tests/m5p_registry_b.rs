//! M5 stage 1, list phyre-registry-load-b (`ffx_pload::m5p_registry_b`):
//! every port runs against its own original on the image alone and on a
//! snapshot (`FFX_SNAPSHOT`). Results and every byte written outside the
//! stacks must agree. Needs `FFX_GAME_DIR`.
//!
//! Callees that stay original are replaced in both oracles alike by
//! recording stubs (in the oracles' code area): each call appends its tag,
//! `ecx` and stack arguments to a log in memory, so the diff checks that
//! both made the same calls in the same order, with the same `this`.
//!
//! Branch reach (almost none of this ever ran in the game, so every branch
//! is synthetic):
//! - the conversion kernels: both directions, counts 0..9 (the unrolled
//!   copies' groups of four and their remainders), positive, odd and
//!   negative strides, both `_ftol2_sse` settings (`g_Sse2Available` 0 and
//!   1), the float edge values (0, -0, halves, the integer types' limits
//!   and one past, 2^63 and 2^64, infinities, quiet NaNs; signalling NaNs
//!   are left out, as the emulator keeps them signalling).

#![allow(
    dead_code, // the shared harness's unused helpers
    clippy::cast_precision_loss,
    clippy::manual_is_multiple_of,
    clippy::precedence,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::similar_names,
    clippy::too_many_lines
)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pload::m5p_registry_b::{
    self as rb, bridge as b, contacts as ct, geometry as gm, helpers as h, mapping as mp,
    more as m, rest as r, skin as sk, swap as s,
};
use ffx_pload::m5p_tmpl_fortype as ft;

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
struct G<'a>(&'a mut Oracle, u32);

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
    fn call_st0_f64(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> f64 {
        let conv = match conv {
            host::Conv::Cdecl => Conv::Cdecl,
            host::Conv::Stdcall => Conv::Stdcall,
            host::Conv::Thiscall(t) => Conv::Thiscall(t),
            host::Conv::Fastcall => Conv::Fastcall,
        };
        let r = self
            .0
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"));
        let v = r.st0_f64().expect("the callee's ST0");
        // the caller's fstp
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        v
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
    /// `eax`.
    U32(u32),
    /// `al`.
    U8(u8),
    /// Nothing: the callers drop `eax` (said where used).
    Void,
}

/// How a recording stub logs.
#[derive(Debug, Clone, Copy)]
enum Log {
    /// Its stack words.
    Words,
    /// The four words of the structure its first stack word points at
    /// (the last as a byte): the `swapEndiannessOfObject` visitors, whose
    /// argument is in the caller's frame.
    Visitor,
    /// [`Log::Visitor`], then the second and third stack words.
    VisitorArgs,
}

/// Bytes per log entry: tag, `ecx`, six words.
const ENTRY: u32 = 32;

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of [`ENTRY`] bytes.
    log: u32,
    /// Stubs written, by original address.
    stubs: HashMap<u32, u32>,
    /// `fstp st(0); ret` (the port's pop of a callee's `ST0`).
    pop: u32,
    /// Regions the port writes in its own memory where the original
    /// writes its stack frame: copied to the original's side before the
    /// next diff.
    sync: Vec<(u32, usize)>,
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let mut t = Self {
            a,
            b,
            log: 0,
            stubs: HashMap::new(),
            pop: 0,
            sync: Vec::new(),
        };
        t.pop = t.a.alloc_code(&[0xdd, 0xd8, 0xc3]).unwrap();
        assert_eq!(t.b.alloc_code(&[0xdd, 0xd8, 0xc3]).unwrap(), t.pop);
        t.log = t.zeros(4 + 64 * ENTRY as usize);
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

    /// `n` random bytes.
    fn object(&mut self, rng: &mut Rng, n: usize) -> u32 {
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

    fn entries(&self) -> u32 {
        Mem::read(&self.b, self.log)
    }

    /// The log entry `i`: tag, `ecx`, six words.
    fn entry(&self, i: u32) -> [u32; 8] {
        let mut e = [0; 8];
        for (k, w) in e.iter_mut().enumerate() {
            *w = Mem::read(&self.b, self.log + 4 + i * ENTRY + 4 * k as u32);
        }
        e
    }

    /// The null page mapped in both, with a fake object at 0 whose
    /// vtable's slots all point at `f` (a stub).
    fn null_object(&mut self, f: u32) {
        self.a.map_zeroed(0, 0x1000).unwrap();
        self.b.map_zeroed(0, 0x1000).unwrap();
        self.w32(0, 0x100);
        for i in 0..16 {
            self.w32(0x100 + 4 * i, f);
        }
    }

    /// Recording stub code (in the code area of both oracles): logs `tag`,
    /// `ecx` when `this` (a cdecl callee's `ecx` is the caller's leftover)
    /// and `n` stack words (or the visitor), pops `pops` words, returns
    /// `ret`. Returns its address.
    fn stub_code(&mut self, tag: u32, this: bool, n: u32, pops: u32, ret: u32, log: Log) -> u32 {
        assert!(n <= 6);
        let l = self.log;
        let mut c = vec![0xa1];
        c.extend(l.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((l + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        if this {
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        }
        match log {
            Log::Words => {
                for i in 0..n {
                    c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
                    c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
                }
            }
            Log::Visitor | Log::VisitorArgs => {
                c.extend([0x8b, 0x4c, 0x24, 0x04]); // mov ecx, [esp+4]
                for i in 0..3u8 {
                    c.extend([0x8b, 0x51, 4 * i]); // mov edx, [ecx+4i]
                    c.extend([0x89, 0x50, 8 + 4 * i]); // mov [eax+8+4i], edx
                }
                c.extend([0x0f, 0xb6, 0x51, 0x0c]); // movzx edx, byte [ecx+0xc]
                c.extend([0x89, 0x50, 0x14]); // mov [eax+0x14], edx
                if matches!(log, Log::VisitorArgs) {
                    c.extend([0x8b, 0x54, 0x24, 0x08]); // mov edx, [esp+8]
                    c.extend([0x89, 0x50, 0x18]); // mov [eax+0x18], edx
                    c.extend([0x8b, 0x54, 0x24, 0x0c]); // mov edx, [esp+0xc]
                    c.extend([0x89, 0x50, 0x1c]); // mov [eax+0x1c], edx
                }
            }
        }
        c.extend([0xff, 0x05]);
        c.extend(l.to_le_bytes()); // inc dword [log]
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

    /// A recording stub over the original at `sym` (a `jmp` there), tagged
    /// with `sym`. Once per address.
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
        self.stub_log(sym, this, n, pops, ret, Log::Words);
    }

    fn stub_log(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32, log: Log) {
        if self.stubs.contains_key(&sym) {
            return;
        }
        let code = self.stub_code(sym, this, n, pops, ret, log);
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
        self.stubs.insert(sym, code);
    }

    fn diff(&mut self, what: &str) {
        for (at, n) in std::mem::take(&mut self.sync) {
            let mut v = vec![0u8; n];
            self.b.read_bytes(at, &mut v);
            self.a.write(at, &v).unwrap();
        }
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

    /// Runs the original at `sym` in `a` and `port` on `b` (the call log
    /// emptied first); compares the result as `port` says and every byte
    /// written. Returns the number of logged calls.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        port: impl FnOnce(&mut G) -> Out,
    ) -> u32 {
        let log = self.log;
        self.w32(log, 0);
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let out = port(&mut G(&mut self.b, self.pop));
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        match out {
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::U8(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
            Out::Void => {}
        }
        self.diff(what);
        self.entries()
    }
}

/// Doubles at the conversions' edges (no signalling NaN).
const F64_EDGES: [f64; 30] = [
    0.0,
    -0.0,
    0.5,
    -0.5,
    1.5,
    -1.5,
    127.9,
    128.0,
    255.9,
    256.0,
    -128.9,
    -129.0,
    32767.5,
    65535.9,
    65536.0,
    -32769.0,
    2_147_483_647.9,
    2_147_483_648.0,
    -2_147_483_648.9,
    -2_147_483_649.0,
    4_294_967_295.5,
    4_294_967_296.0,
    9_223_372_036_854_774_784.0,
    9_223_372_036_854_775_808.0,
    18_446_744_073_709_549_568.0,
    18_446_744_073_709_551_616.0,
    -9_223_372_036_854_775_808.0,
    f64::INFINITY,
    f64::NEG_INFINITY,
    f64::NAN,
];

/// The source element `i`: edge values first, then random ones.
fn element_bytes(from: rb::From, i: usize, rng: &mut Rng) -> Vec<u8> {
    let r64 = |rng: &mut Rng| u64::from(rng.next()) << 32 | u64::from(rng.next());
    match from {
        rb::From::F64 => {
            let x = if i < F64_EDGES.len() && rng.next() % 4 != 0 {
                F64_EDGES[i]
            } else {
                let mut bits = r64(rng);
                if rng.next() % 2 == 0 {
                    // a magnitude the integers can hold
                    bits =
                        (bits & 0x800f_ffff_ffff_ffff) | (u64::from(0x3ff + rng.next() % 70) << 52);
                }
                let x = f64::from_bits(bits);
                if x.is_nan() {
                    f64::from_bits(bits | 0x0008_0000_0000_0000)
                } else {
                    x
                }
            };
            let x = if rng.next() % 7 == 0 { -x } else { x };
            x.to_bits().to_le_bytes().to_vec()
        }
        rb::From::F32 => {
            let x = if i < F64_EDGES.len() && rng.next() % 4 != 0 {
                F64_EDGES[i] as f32
            } else {
                let mut bits = rng.next();
                if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
                    bits |= 0x0040_0000;
                }
                f32::from_bits(bits)
            };
            x.to_bits().to_le_bytes().to_vec()
        }
        rb::From::Q | rb::From::I64 | rb::From::U64 => {
            // 0x8000_0080_0000_0001 rounds differently through f64 (PUInt64 to
            // float rounds twice) than straight to f32.
            const EDGES: [u64; 9] = [
                0x8000_0080_0000_0001,
                0,
                1,
                u64::MAX,
                1 << 63,
                (1 << 53) + 1,
                (1 << 24) + 1,
                0x8000_0000,
                0xffff_ffff,
            ];
            let mut v = if i < EDGES.len() { EDGES[i] } else { r64(rng) };
            if i >= EDGES.len() && rng.next() % 3 == 0 {
                v >>= rng.next() % 64;
            }
            let mut out = v.to_le_bytes().to_vec();
            if from == rb::From::Q {
                out.extend(r64(rng).to_le_bytes());
            }
            out
        }
        rb::From::Bool | rb::From::Char => {
            const EDGES: [u8; 5] = [0, 1, 2, 0x80, 0xff];
            vec![if i < EDGES.len() {
                EDGES[i]
            } else {
                rng.next() as u8
            }]
        }
    }
}

/// The kernels' `eax` is dropped: the driver (`0x0047c610`) calls them
/// with `call eax; add esp, 0x10` and overwrites `eax` on every path
/// after it.
#[test]
fn conversion_kernels_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x1357_9bdf);
        for &(f, from, to) in rb::KERNELS {
            assert_eq!(rb::kernel_of(f), (from, to));
            let size: u32 = match from {
                rb::From::F64 | rb::From::I64 | rb::From::U64 => 8,
                rb::From::Q => 16,
                rb::From::F32 => 4,
                rb::From::Bool | rb::From::Char => 1,
            };
            for case in 0..14u32 {
                let count = [0, 1, 2, 3, 4, 5, 7, 8, 9, 30, 30, 30, 4, 6][case as usize];
                // Direction byte clear / set; case 12 sets only a higher byte.
                let forward = if case == 12 {
                    0x100
                } else {
                    u32::from(case % 2 == 1)
                };
                let (src_stride, dst_stride) = match case {
                    10 => (size + 3, 17),
                    11 => (0u32.wrapping_sub(size), 0u32.wrapping_sub(16)),
                    13 => (0, 0),
                    _ => (size, 16 + 4 * (case % 3)),
                };
                let span = 0x200 + 32 * 30;
                let src_base = t.object(&mut rng, span as usize);
                let dst_base = t.object(&mut rng, span as usize);
                let (a, bo) = (0x10 + rng.next() % 8, 0x18 + rng.next() % 8);
                // Negative strides walk down from the end of the buffers.
                let mid = if case == 11 { span - 0x40 } else { 0 };
                let (src, dst) = (src_base + mid, dst_base + mid);
                let s0 = if forward & 0xff != 0 {
                    src + a
                } else {
                    src + bo
                };
                let mut s = s0;
                for i in 0..count as usize {
                    let bytes = element_bytes(from, i, &mut rng);
                    t.write(s, &bytes);
                    s = s.wrapping_add(src_stride);
                }
                let counts = t.zeros(16);
                t.w32(counts, count);
                t.w32(counts + 4, src_stride);
                t.w32(counts + 8, dst_stride);
                t.w32(counts + 0xc, forward | (rng.next() & 0xffff_0000));
                let (stored, runtime) = (t.object(&mut rng, 0x30), t.object(&mut rng, 0x30));
                t.w32(stored + 8, a);
                t.w32(runtime + 0x24, bo);
                let entry = t.object(&mut rng, 0x20);
                t.w32(entry + 4, stored);
                t.w32(entry + 8, runtime);
                for sse2 in [0, 1] {
                    let at = t.rebase(0x0193_ed20);
                    t.w32(at, sse2);
                    // The destination afresh for each setting.
                    let fill: Vec<u8> = (0..span).map(|_| rng.next() as u8).collect();
                    t.write(dst_base, &fill);
                    t.check(
                        &format!("{name} {f:#x} {from:?}->{to:?} case {case} sse2 {sse2}"),
                        f,
                        Conv::Cdecl,
                        &[src, dst, counts, entry],
                        |g| {
                            b::kernel(g, (from, to), src, dst, counts, entry);
                            Out::Void
                        },
                    );
                }
            }
        }
    }
}

/// A callee's `eax`: every byte set.
const RET: u32 = 0x8765_43fe;

#[test]
fn lazy_descriptors_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x3456_789a);
        t.stub(0x0043_d6a0, true, 2, 2, RET);
        t.stub(0x0094_9207, false, 1, 0, RET ^ 2);
        for &(f, lazy) in h::LAZY {
            assert_eq!(h::of(h::LAZY, f), lazy);
            t.stub(lazy.ctor, true, 0, 0, RET ^ 3);
            for guard in [0x1234_5671, 0x1234_5670, 0, 0xffff_fffe] {
                let (g, fl) = (t.rebase(lazy.guard), t.rebase(lazy.flags));
                t.w32(g, guard);
                t.w32(fl, rng.next());
                let p = t.object(&mut rng, 0x20);
                let calls = t.check(
                    &format!("{name} {f:#x}({p:#x}) guard {guard:#x}"),
                    f,
                    Conv::Cdecl,
                    &[p],
                    |g| Out::U32(ft::bridge::swap_lazy(g, &lazy, p)),
                );
                assert_eq!(calls, if guard & 1 == 0 { 3 } else { 1 }, "{f:#x}");
                let last = t.entry(calls - 1);
                assert_eq!(last[..4], [0x0043_d6a0, t.rebase(lazy.desc), p, lazy.flag]);
            }
        }
    }
}

#[test]
fn destructors_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x2468_ace0);
        t.stub(h::DESCRIPTOR_DESTRUCT, true, 0, 0, RET);
        for &(f, vtable) in h::DESTRUCT {
            let this = t.object(&mut rng, 0x40);
            let calls = t.check(
                &format!("{name} {f:#x}"),
                f,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(h::bridge::destruct(g, this, vtable)),
            );
            assert_eq!(calls, 1);
            assert_eq!(t.entry(0)[..2], [h::DESCRIPTOR_DESTRUCT, this]);
        }
        t.stub(h::DATA_MEMBER_DTOR, true, 0, 0, RET ^ 1);
        t.stub(h::PHYRE_FREE, false, 1, 0, RET ^ 2);
        t.stub(h::OPERATOR_DELETE, false, 1, 0, RET ^ 3);
        for &(f, d) in h::DELETE {
            if let h::Body::Arg(g) = d.body {
                t.stub(g, false, 1, 0, RET ^ 4);
            }
            for flags in [0, 1, 0xfe, 0xff, 0x100] {
                let this = t.object(&mut rng, 0x40);
                let calls = t.check(
                    &format!("{name} {f:#x}({flags:#x})"),
                    f,
                    Conv::Thiscall(this),
                    &[flags],
                    |g| Out::U32(h::bridge::delete(g, &d, this, flags)),
                );
                let body = u32::from(d.body != h::Body::None);
                assert_eq!(calls, body + (flags & 1), "{f:#x}");
            }
        }
        // 0x005706a0: the 0x004355e0 shape (Destruct, PhyreFree).
        let vt = ft::word_of(ft::DELETE, 0x0043_55e0);
        for flags in [0, 1, 0xff] {
            let this = t.object(&mut rng, 0x40);
            t.check(
                &format!("{name} 0x5706a0({flags:#x})"),
                0x0057_06a0,
                Conv::Thiscall(this),
                &[flags],
                |g| Out::U32(ft::bridge::delete(g, this, flags, vt)),
            );
        }
        for flags in [0, 1, 0xff] {
            let this = t.object(&mut rng, 0x40);
            let owner = t.object(&mut rng, 0x40);
            t.w32(this + 4, owner);
            let calls = t.check(
                &format!("{name} 0x575630({flags:#x})"),
                0x0057_5630,
                Conv::Thiscall(this),
                &[flags],
                |g| Out::U32(h::bridge::dynamic_type_delete(g, this, flags, 0x00b3_09cc)),
            );
            assert_eq!(calls, flags & 1);
        }
    }
}

#[test]
fn enum_types_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x1122_3344);
        for &(f, head) in h::UNLINK {
            // lists of 0..4 nodes; the type at each place, or absent
            for len in 0..4u32 {
                for at in 0..=len {
                    let nodes: Vec<u32> = (0..len).map(|_| t.object(&mut rng, 0x10)).collect();
                    for (i, &n) in nodes.iter().enumerate() {
                        let next = nodes.get(i + 1).copied().unwrap_or(0);
                        t.w32(n + 8, next);
                    }
                    let first = nodes.first().copied().unwrap_or(0);
                    let hd = t.rebase(head);
                    t.w32(hd, first);
                    let ty = if at < len {
                        nodes[at as usize]
                    } else {
                        t.object(&mut rng, 0x10)
                    };
                    t.check(
                        &format!("{name} {f:#x} len {len} at {at}"),
                        f,
                        Conv::Cdecl,
                        &[ty],
                        |g| Out::U32(h::bridge::unlink(g, head, ty)),
                    );
                }
            }
        }
        for &(f, offset) in h::NAME {
            for len in [0usize, 1, 7, 40] {
                let mut s: Vec<u8> = (0..len).map(|_| 1 + (rng.next() % 255) as u8).collect();
                s.push(0);
                let text = t.alloc(&s);
                let object = t.object(&mut rng, 0x10);
                t.w32(object + offset, text);
                let (n, sz) = (t.object(&mut rng, 4), t.object(&mut rng, 4));
                for (no, so) in [(n, sz), (n, n)] {
                    t.check(
                        &format!("{name} {f:#x} len {len}"),
                        f,
                        Conv::Cdecl,
                        &[object, no, so],
                        |g| Out::U32(h::bridge::name(g, offset, object, no, so)),
                    );
                }
            }
        }
        for &(f, find) in h::CREATE {
            for found in [0, 0x1234_5678, 0x8000_0000] {
                // a stub per value: a fresh code block, jumped to from `find`
                let code = t.stub_code(find, false, 1, 0, found, Log::Words);
                let at = t.rebase(find);
                let mut j = vec![0xe9];
                j.extend(code.wrapping_sub(at + 5).to_le_bytes());
                t.write(at, &j);
                let out = t.object(&mut rng, 4);
                let nm = t.object(&mut rng, 8);
                let calls = t.check(
                    &format!("{name} {f:#x} found {found:#x}"),
                    f,
                    Conv::Cdecl,
                    &[out, nm],
                    |g| Out::U32(ffx_prender::basic::bridge::create_result(g, find, out, nm)),
                );
                assert_eq!(calls, 1);
                assert_eq!(t.entry(0)[2], nm);
            }
        }
    }
}

#[test]
fn array_resets_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5566_7788);
        t.stub(h::PHYRE_FREE, false, 1, 0, RET);
        for &(f, null_check) in h::RESET {
            for count in [0, 3, 0x8000_0003, 0x7fff_ffff] {
                for data in [0, 0x0012_3450] {
                    let this = t.object(&mut rng, 8);
                    t.w32(this, count);
                    t.w32(this + 4, data);
                    let calls = t.check(
                        &format!("{name} {f:#x} {count:#x} {data:#x}"),
                        f,
                        Conv::Thiscall(this),
                        &[],
                        |g| {
                            ffx_prender::m5r_g::bridge::owned_reset(g, this, null_check);
                            Out::Void
                        },
                    );
                    let freed = count >> 31 == 0 && !(null_check && data == 0);
                    assert_eq!(calls, u32::from(freed), "{f:#x}");
                }
            }
        }
        for &(f, release) in h::RELEASE {
            t.stub(release, false, 2, 0, RET);
            for count in [0, 3, 0x8000_0003, 0x7fff_ffff] {
                let this = t.object(&mut rng, 8);
                t.w32(this, count);
                let calls = t.check(
                    &format!("{name} {f:#x} {count:#x}"),
                    f,
                    Conv::Thiscall(this),
                    &[],
                    |g| {
                        h::bridge::release(g, release, this);
                        Out::Void
                    },
                );
                assert_eq!(calls, u32::from(count >> 31 == 0), "{f:#x}");
            }
        }
        for (f, at) in [(0x0047_a650u32, 0u32), (0x004a_1770, 4), (0x0057_5460, 0xc)] {
            for p in [0, 0x0012_3450, 0x0012_3451] {
                for owned in [0u32, 1, 0xfe, 0xff] {
                    let this = t.object(&mut rng, 0x20);
                    t.w32(this + at, p);
                    t.w32(this + 0x10, owned);
                    let (conv, args) = if f == 0x0047_a650 {
                        (Conv::Cdecl, vec![p])
                    } else {
                        (Conv::Thiscall(this), vec![])
                    };
                    t.check(
                        &format!("{name} {f:#x} {p:#x} {owned:#x}"),
                        f,
                        conv,
                        &args,
                        |g| match f {
                            0x0047_a650 => Out::U32(h::bridge::free_some(g, p)),
                            0x004a_1770 => {
                                h::bridge::free_untagged(g, this);
                                Out::Void
                            }
                            _ => {
                                h::bridge::free_owned(g, this);
                                Out::Void
                            }
                        },
                    );
                }
            }
        }
    }
}

#[test]
fn small_methods_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x99aa_bbcc);
        for &(f, k) in h::CONSTANT {
            let n = match f {
                0x0057_6110 => 2,
                0x0057_6120 => 3,
                _ => 1,
            };
            let this = t.object(&mut rng, 8);
            let args: Vec<u32> = (0..n).map(|_| rng.next()).collect();
            t.check(
                &format!("{name} {f:#x}"),
                f,
                Conv::Thiscall(this),
                &args,
                |_| Out::U32(k),
            );
        }
        let this = t.object(&mut rng, 0x40);
        t.check(
            &format!("{name} 0x479910"),
            0x0047_9910,
            Conv::Thiscall(this),
            &[],
            |g| {
                h::bridge::set_vtable(g, this, 0x00b1_3484);
                Out::Void
            },
        );
        for _ in 0..4 {
            let this = t.object(&mut rng, 0x40);
            let next = t.object(&mut rng, 8);
            let prev = t.object(&mut rng, 8);
            t.w32(this + 0x2c, next);
            t.w32(this + 0x30, prev);
            t.check(
                &format!("{name} 0x5a0270"),
                0x005a_0270,
                Conv::Thiscall(this),
                &[],
                |g| {
                    Out::U32(h::bridge::named_semantic_dtor(
                        g,
                        this,
                        [0x00b3_44b0, 0x00b0_e568],
                    ))
                },
            );
        }
        // the node alone in its list (next = prev = itself)
        let this = t.object(&mut rng, 0x40);
        t.w32(this + 0x2c, this + 0x2c);
        t.w32(this + 0x30, this + 0x2c);
        t.check(
            &format!("{name} 0x5a0270 alone"),
            0x005a_0270,
            Conv::Thiscall(this),
            &[],
            |g| {
                Out::U32(h::bridge::named_semantic_dtor(
                    g,
                    this,
                    [0x00b3_44b0, 0x00b0_e568],
                ))
            },
        );
        for f in [0x0057_6000u32, 0x0057_6020] {
            for _ in 0..4 {
                let this = t.object(&mut rng, 0x40);
                let owner = t.object(&mut rng, 0x90);
                t.w32(this + 0xc, owner);
                let object = rng.next();
                t.check(
                    &format!("{name} {f:#x}"),
                    f,
                    Conv::Thiscall(this),
                    &[object],
                    |g| Out::U32(h::bridge::member_address(g, this, object)),
                );
            }
        }
        let this = t.object(&mut rng, 0x40);
        t.check(
            &format!("{name} 0x576040"),
            0x0057_6040,
            Conv::Thiscall(this),
            &[rng.next()],
            |g| Out::U32(h::bridge::word_at(g, this, 0x34)),
        );
        let virt = t.stub_code(0x7e, true, 2, 2, RET, Log::Words);
        for (f, slots) in [(0x0057_56b0u32, [3u32, 5]), (0x0057_56f0, [4, 6])] {
            for flags in [0u32, 2, 0xfd, 0xff, 0x200] {
                let this = t.object(&mut rng, 0x40);
                let owner = t.object(&mut rng, 0x90);
                let ty = t.object(&mut rng, 8);
                let vtable = t.zeros(0x40);
                t.w32(ty, vtable);
                t.w32(vtable + 4 * slots[0], virt);
                t.w32(vtable + 4 * slots[1], virt);
                t.w32(vtable + 4 * slots[0], virt);
                t.w32(this + 0xc, owner);
                t.w32(this + 0x18, ty);
                t.w32(this + 0x14, flags);
                // a different stub per slot, so the log shows which ran
                let other = t.stub_code(0x7f, true, 2, 2, RET ^ 1, Log::Words);
                t.w32(vtable + 4 * slots[1], other);
                let (a, object) = (rng.next(), rng.next());
                let calls = t.check(
                    &format!("{name} {f:#x} {flags:#x}"),
                    f,
                    Conv::Thiscall(this),
                    &[a, object],
                    |g| Out::U32(h::bridge::member_virtual(g, slots, this, a, object)),
                );
                assert_eq!(calls, 1);
                assert_eq!(t.entry(0)[0], if flags & 2 != 0 { 0x7f } else { 0x7e });
            }
        }
        t.stub(h::NEW_POOL_MESH_INSTANCE, true, 2, 2, RET);
        let scene = t.object(&mut rng, 0x10);
        let at = t.rebase(h::PHYRE_SCENE);
        t.w32(at, scene);
        for existing in [0, 0x0055_6670] {
            let this = t.object(&mut rng, 0x10);
            let mesh = rng.next();
            let calls = t.check(
                &format!("{name} 0x672b30 {existing:#x}"),
                0x0067_2b30,
                Conv::Thiscall(this),
                &[mesh, existing],
                |g| Out::U32(h::bridge::pool_instance(g, this, mesh, existing)),
            );
            assert_eq!(calls, u32::from(existing == 0));
            if existing == 0 {
                assert_eq!(
                    t.entry(0)[..4],
                    [h::NEW_POOL_MESH_INSTANCE, scene, this, mesh]
                );
            }
        }
        t.stub(h::PHYRE_MALLOC, false, 2, 0, 0);
        let malloc_ret = |t: &mut Twins, v: u32| {
            let code = t.stub_code(h::PHYRE_MALLOC, false, 2, 0, v, Log::Words);
            let at = t.rebase(h::PHYRE_MALLOC);
            let mut j = vec![0xe9];
            j.extend(code.wrapping_sub(at + 5).to_le_bytes());
            t.write(at, &j);
        };
        for n in [0u32, 1, 2, 5] {
            for fail in [false, true] {
                let heap = t.object(&mut rng, 0x40);
                malloc_ret(&mut t, if fail { 0 } else { heap });
                let this = t.object(&mut rng, 0x20);
                let src = t.object(&mut rng, 0x40);
                t.check(
                    &format!("{name} 0x4d0470 {n} {fail}"),
                    0x004d_0470,
                    Conv::Thiscall(this),
                    &[n, src],
                    |g| Out::U32(h::bridge::alloc_copy(g, this, n, src, n)),
                );
                for count in [0u32, 1, 3] {
                    let this = t.object(&mut rng, 0x20);
                    t.check(
                        &format!("{name} 0x4d04d0 {n} {count} {fail}"),
                        0x004d_04d0,
                        Conv::Thiscall(this),
                        &[n, src, count],
                        |g| Out::U32(h::bridge::alloc_copy(g, this, n, src, count)),
                    );
                }
            }
        }
        let vstore = t.stub_code(0x7d, true, 4, 4, RET, Log::Words);
        for f in [0x0067_2f20u32, 0x0067_6590, 0x0067_7bf0] {
            let this = t.object(&mut rng, 0x10);
            let vtable = t.zeros(0x40);
            t.w32(this, vtable);
            t.w32(vtable + 0x14, vstore);
            let dst = t.object(&mut rng, 4);
            let out = t.object(&mut rng, 8);
            t.w32(out, dst);
            let a: Vec<u32> = (0..4).map(|_| rng.next()).collect();
            let mut args = a.clone();
            args.push(out);
            let calls = t.check(
                &format!("{name} {f:#x}"),
                f,
                Conv::Thiscall(this),
                &args,
                |g| {
                    Out::U32(ffx_prender::m5r_f::bridge::virtual_store(
                        g,
                        this,
                        [a[0], a[1], a[2], a[3]],
                        out,
                    ))
                },
            );
            assert_eq!(calls, 1);
        }
    }
}

/// A fake object at a fresh address whose vtable's 16 slots all point at
/// `f`; returns the object.
fn fake_object(t: &mut Twins, f: u32) -> u32 {
    let vtable = t.zeros(0x40);
    for i in 0..16 {
        t.w32(vtable + 4 * i, f);
    }
    let obj = t.zeros(0x20);
    t.w32(obj, vtable);
    obj
}

/// Points the jump at `sym` to a new recording stub returning `ret`.
fn restub(t: &mut Twins, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
    let code = t.stub_code(sym, this, n, pops, ret, Log::Words);
    let at = t.rebase(sym);
    let mut j = vec![0xe9];
    j.extend(code.wrapping_sub(at + 5).to_le_bytes());
    t.write(at, &j);
}

#[test]
fn shared_arrays_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0bad_cafe);
        t.stub(m::PHYRE_FREE, false, 1, 0, RET);
        for count in [0u32, 1, 2, 5, 0x8000_0000, 0xffff_ffff] {
            for word in [0u32, 1, 3, 0x8000_0001, 0x8000_0004] {
                for same in [false, true] {
                    let this = t.object(&mut rng, 0x10);
                    let buf = t.object(&mut rng, 0x20);
                    t.w32(this, word);
                    t.w32(
                        this + 4,
                        if word & 0x7fff_ffff > 1 {
                            buf
                        } else {
                            rng.next()
                        },
                    );
                    let data = if same { buf } else { t.object(&mut rng, 0x20) };
                    t.check(
                        &format!("{name} 0x4d0b00 {count:#x} {word:#x} {same}"),
                        m::SHARRAY_SET,
                        Conv::Thiscall(this),
                        &[data, count],
                        |g| Out::U32(m::bridge::sharray_set(g, this, data, count)),
                    );
                }
            }
        }
        // the old buffer 0 (count word > 1 with a null pointer)
        let this = t.object(&mut rng, 0x10);
        t.w32(this, 3);
        t.w32(this + 4, 0);
        let data = t.object(&mut rng, 0x20);
        t.check(
            &format!("{name} 0x4d0b00 null"),
            m::SHARRAY_SET,
            Conv::Thiscall(this),
            &[data, 4],
            |g| Out::U32(m::bridge::sharray_set(g, this, data, 4)),
        );
        // resize: the storage call stubbed (it is ported and tested on its own)
        for ret in [0u32, 1] {
            for n in [0u32, 1, 2, 3, 7] {
                for word in [0u32, 1, 3, 7, 0x8000_0003] {
                    let this = t.object(&mut rng, 0x10);
                    let old = t.object(&mut rng, 0x40);
                    t.w32(this, word);
                    t.w32(
                        this + 4,
                        if word & 0x7fff_ffff > 1 {
                            old
                        } else {
                            rng.next()
                        },
                    );
                    let storage = match ret {
                        0 => 0,
                        _ if n <= 1 => this + 4,
                        _ => t.object(&mut rng, 0x40),
                    };
                    restub(&mut t, m::SHARRAY_ALLOC_COPY, true, 3, 3, storage);
                    t.check(
                        &format!("{name} 0x4d0970 {n} {word:#x} {ret}"),
                        0x004d_0970,
                        Conv::Thiscall(this),
                        &[n],
                        |g| Out::U32(m::bridge::sharray_resize(g, this, n)),
                    );
                }
            }
        }
        let this = t.object(&mut rng, 0x10);
        t.w32(this, 5);
        t.check(
            &format!("{name} 0x4d0970 same"),
            0x004d_0970,
            Conv::Thiscall(this),
            &[5],
            |g| Out::U32(m::bridge::sharray_resize(g, this, 5)),
        );
    }
}

#[test]
fn sharray_member_set_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x7777_1111);
        t.stub(m::SHARRAY_SET, true, 2, 2, RET);
        t.stub(m::APPLY_DEFAULT_BUFFER, true, 2, 2, RET ^ 1);
        for desc in [0u32, 0x0033_4450] {
            let slot = t.stub_code(0x51, true, 0, 0, desc, Log::Words);
            let ty = fake_object(&mut t, slot);
            restub(&mut t, m::GET_TYPE_U32, false, 0, 0, ty);
            for n in [0u32, 1, 2] {
                for inline in [false, true] {
                    let this = t.object(&mut rng, 0x10);
                    let data = if inline {
                        this + 4
                    } else {
                        t.object(&mut rng, 8)
                    };
                    t.check(
                        &format!("{name} 0x4d0ba0 {desc:#x} {n} {inline}"),
                        0x004d_0ba0,
                        Conv::Thiscall(this),
                        &[data, n],
                        |g| Out::U32(m::bridge::sharray_member_set(g, this, data, n)),
                    );
                }
            }
        }
    }
}

#[test]
fn lookups_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x2222_9999);
        // indexOf: inline (one element) and pointed-to storage
        for n in [0u32, 1, 4, 0x8000_0004] {
            for start in [0u32, 1, 3, 9] {
                for hit in [None, Some(0u32), Some(2), Some(3)] {
                    let this = t.object(&mut rng, 0x10);
                    let data = t.object(&mut rng, 0x20);
                    t.w32(this, n);
                    if n & 0x7fff_ffff > 1 {
                        t.w32(this + 4, data);
                    }
                    let v = rng.next();
                    if let Some(i) = hit {
                        let at = if n & 0x7fff_ffff > 1 {
                            data + 4 * i
                        } else {
                            this + 4 + 4 * i
                        };
                        t.w32(at, v);
                    }
                    let value = t.object(&mut rng, 4);
                    t.w32(value, v);
                    t.check(
                        &format!("{name} 0x518a60 {n:#x} {start} {hit:?}"),
                        0x0051_8a60,
                        Conv::Thiscall(this),
                        &[value, start],
                        |g| Out::U32(m::bridge::index_of(g, this, value, start)),
                    );
                }
            }
        }
        // the string index: same pointer, equal text, tagged pointers, nulls
        let words = [
            b"alpha\0".to_vec(),
            b"beta\0".to_vec(),
            b"\0".to_vec(),
            b"alphb\0".to_vec(),
        ];
        for start in [0u32, 1, 2, 5] {
            for case in 0..6u32 {
                let strings: Vec<u32> = words.iter().map(|w| t.alloc(w)).collect();
                let data = t.zeros(0x20);
                let entries = [strings[0] | 1, 0, strings[1], strings[3]];
                for (i, &e) in entries.iter().enumerate() {
                    t.w32(data + 4 * i as u32, e);
                }
                let this = t.object(&mut rng, 8);
                t.w32(this, 4);
                t.w32(this + 4, data);
                let want = match case {
                    0 => strings[1],
                    1 => t.alloc(b"beta\0"),
                    2 => t.alloc(b"alpha\0"),
                    3 => 0,
                    4 => t.alloc(b"gamma\0"),
                    _ => t.alloc(b"\0"),
                };
                let nm = t.object(&mut rng, 4);
                t.w32(nm, want);
                t.check(
                    &format!("{name} 0x54dcd0 {start} {case}"),
                    0x0054_dcd0,
                    Conv::Thiscall(this),
                    &[nm, start],
                    |g| Out::U32(m::bridge::name_index(g, this, nm, start)),
                );
            }
        }
        // trees: insert keys in a random order, then find present and absent keys
        for round in 0..4 {
            let tree = t.object(&mut rng, 0x20);
            t.w32(tree + 0xc, tree);
            let mut keys = Vec::new();
            for _ in 0..(3 + 3 * round) {
                let node = t.object(&mut rng, 0x10);
                let key = rng.next() % 20;
                keys.push(key);
                t.w32(node + 0xc, key);
                t.check(
                    &format!("{name} 0x5cc5e0 {key}"),
                    0x005c_c5e0,
                    Conv::Thiscall(tree),
                    &[node],
                    |g| Out::U32(m::bridge::tree_insert(g, tree, node)),
                );
            }
            for key in 0..21u32 {
                let k = t.object(&mut rng, 4);
                t.w32(k, key);
                t.check(
                    &format!("{name} 0x5cc560 {key}"),
                    0x005c_c560,
                    Conv::Thiscall(tree),
                    &[k],
                    |g| Out::U32(m::bridge::tree_find(g, tree, k)),
                );
            }
        }
    }
}

#[test]
fn visitors_and_filters_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x4444_aaaa);
        t.stub(m::PHYRE_FREE, false, 1, 0, RET);
        t.stub(m::MEMBER_LIST_DTOR, true, 0, 0, RET ^ 1);
        let unlink = t.stub_code(0x99, true, 1, 1, RET ^ 2, Log::Words);
        for f in [0x0057_5510u32, 0x0057_55a0] {
            for flags in [0u32, 1, 0xff] {
                for (empty, owned) in [(true, 0u32), (false, 1), (false, 0xfe), (true, 0xff)] {
                    let this = t.object(&mut rng, 0x40);
                    // only slot 9 is the unlink
                    let owner = fake_object(&mut t, 0);
                    let vt: u32 = Mem::read(&t.b, owner);
                    t.w32(vt + 4 * 9, unlink);
                    t.w32(this + 0xc, owner);
                    t.w32(this + 4, if empty { this + 4 } else { rng.next() });
                    t.w32(this + 0x14, owned);
                    t.check(
                        &format!("{name} {f:#x} {flags} {empty} {owned:#x}"),
                        f,
                        Conv::Thiscall(this),
                        &[flags],
                        |g| Out::U32(m::bridge::member_delete(g, this, flags)),
                    );
                }
            }
        }
        for flags in [0u32, 1, 0xff] {
            let this = t.object(&mut rng, 0x40);
            let next = t.object(&mut rng, 8);
            let prev = t.object(&mut rng, 8);
            t.w32(this + 0x2c, next);
            t.w32(this + 0x30, prev);
            t.check(
                &format!("{name} 0x5a02a0 {flags}"),
                0x005a_02a0,
                Conv::Thiscall(this),
                &[flags],
                |g| Out::U32(m::bridge::named_semantic_delete(g, this, flags)),
            );
        }
        // the delete-children visitor
        let destroy = t.stub_code(0x9a, true, 1, 1, RET ^ 3, Log::Words);
        for has_desc in [false, true] {
            for flags in [0u32, 1, 2, 3, 0xfd] {
                for count in [0u32, 1, 3] {
                    let desc = if has_desc {
                        fake_object(&mut t, destroy)
                    } else {
                        0
                    };
                    if has_desc {
                        t.w32(desc + 0x1c, 0x24);
                    }
                    let getter = t.stub_code(0x9b, true, 0, 0, desc, Log::Words);
                    let member = fake_object(&mut t, getter);
                    let this = t.object(&mut rng, 8);
                    let objects = rng.next();
                    let mut desc_big = desc;
                    if has_desc {
                        // a descriptor large enough for +0x8c
                        desc_big = t.zeros(0x100);
                        let vt: u32 = Mem::read(&t.b, desc);
                        t.w32(desc_big, vt);
                        t.w32(desc_big + 0x1c, 0x24);
                        t.w32(desc_big + 0x8c, 0x10);
                        let getter = t.stub_code(0x9b, true, 0, 0, desc_big, Log::Words);
                        let vtm: u32 = Mem::read(&t.b, member);
                        t.w32(vtm + 4, getter);
                    }
                    let _ = desc_big;
                    t.check(
                        &format!("{name} 0x4d7510 {has_desc} {flags} {count}"),
                        0x004d_7510,
                        Conv::Thiscall(this),
                        &[objects, count, member, flags],
                        |g| Out::U32(m::bridge::delete_children(g, objects, count, member, flags)),
                    );
                }
            }
        }
        // the slot filter: each of the three calls true or false, with and without a child
        for mask in 0..8u32 {
            for child in [false, true] {
                let r1 = if mask & 1 != 0 {
                    0x1234_5601
                } else {
                    0x1234_5600
                };
                let r3 = if mask & 2 != 0 {
                    0x2234_5603
                } else {
                    0x2234_5600
                };
                let rc = if mask & 4 != 0 {
                    0x3234_5601
                } else {
                    0x3234_5600
                };
                let s1 = t.stub_code(0xa1, true, 2, 2, r1, Log::Words);
                let s3 = t.stub_code(0xa3, true, 2, 2, r3, Log::Words);
                let sc = t.stub_code(0xac, true, 2, 2, rc, Log::Words);
                let object = fake_object(&mut t, s1);
                let vt: u32 = Mem::read(&t.b, object);
                t.w32(vt + 12, s3);
                let this = t.object(&mut rng, 0x20);
                let c = if child { fake_object(&mut t, sc) } else { 0 };
                t.w32(this + 0x14, c);
                let x = rng.next();
                t.check(
                    &format!("{name} 0x52ae40 {mask} {child}"),
                    0x0052_ae40,
                    Conv::Thiscall(this),
                    &[object, x],
                    |g| Out::U32(m::bridge::slot_filter(g, this, object, x)),
                );
            }
        }
    }
}

#[test]
fn copies_and_floats_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x6666_dddd);
        // the slot filter's key copy
        for (count, base, list) in [
            (0u32, 0u32, vec![]),
            (5, 3, vec![1, 3, 5, 6, 9]),
            (6, 0, vec![0, 5]),
            (4, 10, vec![2, 4]),
        ] {
            let src = t.object(&mut rng, 0x20 * 8);
            let dst_buf = t.object(&mut rng, 0x20 * 8);
            let dst = t.object(&mut rng, 4);
            t.w32(dst, dst_buf);
            let srcp = t.object(&mut rng, 4);
            t.w32(srcp, src);
            let pp = t.object(&mut rng, 4);
            t.w32(pp, srcp);
            let mut l = vec![list.len() as u32];
            l.extend(list.iter().copied());
            let bytes: Vec<u8> = l.iter().flat_map(|w| w.to_le_bytes()).collect();
            let lst = t.alloc(&bytes);
            let indices = t.object(&mut rng, 4);
            t.w32(indices, lst.wrapping_sub(indices));
            let range = t.object(&mut rng, 12);
            t.w32(range + 4, count);
            t.w32(range + 8, base);
            t.check(
                &format!("{name} 0x529e60 {count} {base}"),
                0x0052_9e60,
                Conv::Cdecl,
                &[dst, pp, 0, indices, 0, range],
                |g| Out::U32(m::bridge::slot_copy(g, dst, pp, indices, range)),
            );
        }
        for n in [0u32, 1, 5] {
            let p = t.object(&mut rng, 0x20);
            t.check(
                &format!("{name} 0x47bf50 {n}"),
                0x0047_bf50,
                Conv::Cdecl,
                &[n, p],
                |g| {
                    m::bridge::swap_words(g, n, p);
                    Out::Void
                },
            );
        }
        for &(f, words) in m::COPY {
            for n in [0u32, 1, 3] {
                let src = t.object(&mut rng, 0x30 * 4);
                let dst = t.object(&mut rng, 0x30 * 4);
                t.check(
                    &format!("{name} {f:#x} {n}"),
                    f,
                    Conv::Cdecl,
                    &[dst, src, n],
                    |g| {
                        m::bridge::copy_construct(g, words, dst, src, n);
                        Out::Void
                    },
                );
            }
            // dst 0: the first slot is skipped (null page mapped)
            t.a.map_zeroed(0, 0x1000).ok();
            t.b.map_zeroed(0, 0x1000).ok();
            let src = t.object(&mut rng, 0x30 * 4);
            t.check(
                &format!("{name} {f:#x} null"),
                f,
                Conv::Cdecl,
                &[0, src, 2],
                |g| {
                    m::bridge::copy_construct(g, words, 0, src, 2);
                    Out::Void
                },
            );
        }
        t.stub(m::DESCRIPTOR_PAIR, true, 2, 2, RET);
        for &(f, desc) in m::PAIR {
            for (a, b) in [(0u32, 0u32), (0x100, 0), (0, 0x200), (0x300, 0x400)] {
                let calls = t.check(
                    &format!("{name} {f:#x} {a:#x} {b:#x}"),
                    f,
                    Conv::Cdecl,
                    &[a, b],
                    |g| Out::U32(m::bridge::pair(g, desc, a, b)),
                );
                assert_eq!(calls, 1);
                assert_eq!(
                    t.entry(0)[1..4],
                    [
                        t.rebase(desc),
                        if a == 0 { 0 } else { a + 4 },
                        if b == 0 { 0 } else { b + 4 }
                    ]
                );
            }
        }
        let floats = [
            0.0f32,
            -0.0,
            1.0,
            -1.0,
            0.5,
            1e30,
            -1e-30,
            f32::INFINITY,
            f32::NAN,
            3.3,
            -7.25,
            16_777_217.0,
        ];
        for (f, off, neg) in [
            (0x00a2_f740u32, 0u32, true),
            (0x00a2_f760, 4, true),
            (0x00a2_f780, 0, false),
            (0x00a2_f7a0, 4, false),
        ] {
            for &x in &floats {
                let xy = t.object(&mut rng, 8);
                t.w32(xy + off, x.to_bits());
                let addr = t.rebase(f);
                let ra = t.a.call(addr, Conv::Cdecl, &[xy]).unwrap();
                let port = m::bridge::axis(&mut G(&mut t.b, t.pop), xy, off, neg);
                assert_eq!(
                    ra.st0_f32().map(f32::to_bits),
                    Some(port.to_bits()),
                    "{name} {f:#x} {x}"
                );
                t.diff(&format!("{name} {f:#x}"));
            }
        }
        // the LOD blend: a sum whose order shows (2^53 + 1 - 2^53)
        {
            let weights = t.alloc(
                &[1.0f32, 1.0, 1.0, 0.0]
                    .iter()
                    .flat_map(|w| w.to_bits().to_le_bytes())
                    .collect::<Vec<u8>>(),
            );
            let arrays = t.zeros(16);
            for (i, v) in [
                9_007_199_254_740_992.0f32,
                1.0,
                -9_007_199_254_740_992.0,
                5.0,
            ]
            .iter()
            .enumerate()
            {
                let arr = t.alloc(&v.to_bits().to_le_bytes());
                t.w32(arrays + 4 * i as u32, arr);
            }
            let out_buf = t.object(&mut rng, 4);
            let out = t.object(&mut rng, 4);
            t.w32(out, out_buf);
            t.check(
                &format!("{name} 0x5cb940 order"),
                0x005c_b940,
                Conv::Cdecl,
                &[weights, 1, arrays, out],
                |g| {
                    m::bridge::blend4(g, weights, 1, arrays, out);
                    Out::Void
                },
            );
        }
        // the LOD blend
        for n in [1u32, 2, 5] {
            let weights = t.zeros(16);
            for i in 0..4 {
                let w = floats[(rng.next() % floats.len() as u32) as usize];
                let w = if rng.next() % 3 == 0 {
                    w
                } else {
                    (rng.next() % 1000) as f32 / 999.0
                };
                t.w32(weights + 4 * i, w.to_bits());
            }
            let arrays = t.zeros(16);
            for i in 0..4 {
                let arr = t.zeros(4 * 8);
                for k in 0..8 {
                    let v = if rng.next() % 5 == 0 {
                        floats[(rng.next() % floats.len() as u32) as usize]
                    } else {
                        (rng.next() as i32 as f32) / 65536.0
                    };
                    t.w32(arr + 4 * k, v.to_bits());
                }
                t.w32(arrays + 4 * i, arr);
            }
            let out_buf = t.object(&mut rng, 4 * 8);
            let out = t.object(&mut rng, 4);
            t.w32(out, out_buf);
            t.check(
                &format!("{name} 0x5cb940 {n}"),
                0x005c_b940,
                Conv::Cdecl,
                &[weights, n, arrays, out],
                |g| {
                    m::bridge::blend4(g, weights, n, arrays, out);
                    Out::Void
                },
            );
        }
    }
}

#[test]
fn batch4_destructors_and_factories_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x1357_2468);
        for sym in [0x0057_4760u32, 0x0057_4970, 0x0049_cc10, 0x0049_c5b0] {
            t.stub(sym, true, 0, 0, RET ^ sym);
        }
        for &f in r::TREE_PAIR_DTOR {
            let this = t.object(&mut rng, 0x40);
            let calls = t.check(
                &format!("{name} {f:#x}"),
                f,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(ffx_prender::m5r_b::bridge::tree_pair_dtor(g, this)),
            );
            assert_eq!(calls, 2);
        }
        let this = t.object(&mut rng, 0x40);
        let calls = t.check(
            &format!("{name} 0x4794e0"),
            0x0047_94e0,
            Conv::Thiscall(this),
            &[],
            |g| Out::U32(ffx_prender::m5r_b::bridge::tree_free_dtor(g, this)),
        );
        assert_eq!(calls, 5);
        for &(f, pair) in r::NEW_INSTANCE {
            t.stub(pair[1], true, 1, 1, RET);
            for (existing, list, inst) in [
                (0x0012_3450u32, 0u32, 0u32),
                (0, 0, 0),
                (0, 1, 0),
                (0, 1, 1),
            ] {
                let l = if list == 0 {
                    0
                } else {
                    t.object(&mut rng, 0x40)
                };
                if l != 0 {
                    let i = if inst == 0 {
                        0
                    } else {
                        t.object(&mut rng, 0x20)
                    };
                    t.w32(l + 0x34, i);
                }
                restub(&mut t, r::ALLOCATE_INSTANCE_LIST, true, 3, 3, l);
                let this = t.object(&mut rng, 0x20);
                let cluster = t.object(&mut rng, 0x20);
                // the port's argument slots: a copy in b, compared with
                // what the original left in its own stack slots
                let slots = t.object(&mut rng, 8);
                let addr = t.rebase(f);
                let ra =
                    t.a.call(addr, Conv::Thiscall(this), &[cluster, existing])
                        .unwrap();
                let port = r::bridge::new_instance(
                    &mut G(&mut t.b, t.pop),
                    pair,
                    slots,
                    this,
                    cluster,
                    existing,
                );
                assert_eq!(ra.eax, port, "{name} {f:#x}");
                // the slots the port wrote, against a fresh copy of what
                // the original would write (instance in both, or one)
                let s0: u32 = Mem::read(&t.b, slots);
                let s1: u32 = Mem::read(&t.b, slots + 4);
                // the slots get the instance (both, or the second alone
                // when it is 0); then the same bytes on the original's side
                if existing == 0 && l != 0 {
                    let i: u32 = Mem::read(&t.b, l + 0x34);
                    assert_eq!(s1, i, "{f:#x}");
                }
                t.a.write(slots, &s0.to_le_bytes()).unwrap();
                t.a.write(slots + 4, &s1.to_le_bytes()).unwrap();
                t.diff(&format!("{name} {f:#x}"));
            }
        }
    }
}

#[test]
fn batch4_copy_kernel_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x2468_1357);
        t.stub(r::MEMCPY, false, 3, 0, RET);
        t.stub(r::MEMBER_COUNT, true, 0, 0, 3);
        for kind in [0u32, 1, 2] {
            for forward in [0u32, 1] {
                for count in [0u32, 1, 3] {
                    for stored_n in [1u32, 5] {
                        let desc = t.zeros(0x40);
                        t.w32(desc + 0x30, if kind == 2 { 1 } else { 2 });
                        let getter = t.stub_code(
                            0xd1,
                            true,
                            0,
                            0,
                            if kind == 0 { 0 } else { desc },
                            Log::Words,
                        );
                        let runtime = fake_object(&mut t, getter);
                        t.w32(runtime + 0x24, 0x18);
                        let stored = t.object(&mut rng, 0x20);
                        t.w32(stored + 8, 0x10);
                        t.w32(stored + 0xc, 0x0c);
                        t.w32(stored + 0x14, stored_n);
                        let entry = t.object(&mut rng, 0x10);
                        t.w32(entry + 4, stored);
                        t.w32(entry + 8, runtime);
                        let counts = t.zeros(16);
                        t.w32(counts, count);
                        t.w32(counts + 4, 0x20);
                        t.w32(counts + 8, 0x30);
                        t.w32(counts + 0xc, forward);
                        let (src, dst) = (rng.next(), rng.next());
                        t.check(
                            &format!("{name} 0x47a510 {kind} {forward} {count} {stored_n}"),
                            0x0047_a510,
                            Conv::Cdecl,
                            &[src, dst, counts, entry],
                            |g| {
                                r::bridge::copy_kernel(g, src, dst, counts, entry);
                                Out::Void
                            },
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn batch4_methods_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x3579_bdf1);
        let floats = [
            0.0f32,
            -0.0,
            1.0,
            -2.5,
            1e30,
            3e-39,
            f32::INFINITY,
            f32::NAN,
            0.1,
        ];
        for s in [1.5f32, -0.0, 1e20, f32::NAN, 3.3] {
            let this = t.zeros(0x40);
            for i in 0..16 {
                let v = if i < floats.len() {
                    floats[i]
                } else {
                    rng.next() as f32 / 7.0
                };
                t.w32(this + 4 * i as u32, v.to_bits());
            }
            for out in [t.zeros(0x40), this] {
                t.check(
                    &format!("{name} 0x570c00 {s}"),
                    0x0057_0c00,
                    Conv::Thiscall(this),
                    &[out, s.to_bits()],
                    |g| Out::U32(r::bridge::scale_matrix(g, this, out, s.to_bits())),
                );
            }
        }
        // the Lua user data test: each call's result
        for (p, meta, eq) in [
            (0u32, 1u32, 1u32),
            (0x55_0000, 0, 1),
            (0x55_0000, 1, 0),
            (0x55_0000, 1, 1),
        ] {
            restub(&mut t, r::LUA[0], false, 2, 0, p);
            restub(&mut t, r::LUA[1], false, 2, 0, meta);
            restub(&mut t, r::LUA[2], false, 3, 0, RET);
            restub(&mut t, r::LUA[3], false, 3, 0, eq);
            restub(&mut t, r::LUA[4], false, 2, 0, RET ^ 1);
            let (l, idx) = (rng.next(), rng.next());
            t.check(
                &format!("{name} 0x570d50 {p} {meta} {eq}"),
                0x0057_0d50,
                Conv::Cdecl,
                &[l, idx],
                |g| Out::U32(r::bridge::test_user_data(g, l, idx)),
            );
        }
        t.stub(r::PHYRE_FREE, false, 1, 0, RET);
        for n in [0u32, 1, 3, 0x8000_0002] {
            for ret_inline in [false, true] {
                let src = t.object(&mut rng, 0x10);
                t.w32(src, n);
                let this = t.object(&mut rng, 0x10);
                let buf = if ret_inline {
                    this + 4
                } else {
                    t.object(&mut rng, 0x10)
                };
                restub(&mut t, r::SHARRAY_COPY, true, 2, 2, buf);
                t.check(
                    &format!("{name} 0x5a2380 {n:#x} {ret_inline}"),
                    0x005a_2380,
                    Conv::Thiscall(this),
                    &[src],
                    |g| Out::U32(r::bridge::sharray_copy(g, this, src)),
                );
            }
        }
        // the slot map constructor; the tree insert stubbed by result
        for (n, fail, existing) in [
            (0u32, false, 0u32),
            (3, false, 0),
            (3, false, 1),
            (2, true, 0),
        ] {
            let heap = t.object(&mut rng, 16 * 4);
            restub(
                &mut t,
                r::PHYRE_MALLOC,
                false,
                2,
                0,
                if fail { 0 } else { heap },
            );
            restub(&mut t, r::TREE_INSERT[0], true, 1, 1, existing);
            restub(&mut t, r::TREE_INSERT[1], true, 1, 1, RET);
            let entries = t.object(&mut rng, 16 * 4);
            for i in 0..4 {
                let obj = if i == 1 { 0 } else { t.object(&mut rng, 0x10) };
                t.w32(entries + 16 * i, obj);
            }
            let src = t.object(&mut rng, 0x18);
            t.w32(src + 0x10, n | 0x8000_0000 * u32::from(n == 3));
            t.w32(src + 0x14, entries);
            let this = t.object(&mut rng, 0x20);
            t.check(
                &format!("{name} 0x5cc670 {n} {fail} {existing}"),
                0x005c_c670,
                Conv::Thiscall(this),
                &[src],
                |g| Out::U32(r::bridge::slot_map(g, this, src)),
            );
        }
        for (i, &g0) in r::RADIAL_GETTERS.iter().enumerate() {
            restub(&mut t, g0, true, 0, 0, 0x100 + i as u32);
        }
        let inner = t.object(&mut rng, 0x10);
        let mesh = t.object(&mut rng, 0x20);
        t.w32(mesh + 0x1c, inner);
        let this = t.object(&mut rng, 8);
        t.w32(this + 4, mesh);
        let out = t.object(&mut rng, 0x30);
        t.check(
            &format!("{name} 0x675a90"),
            0x0067_5a90,
            Conv::Thiscall(this),
            &[out, 7],
            |g| Out::U32(r::bridge::radial_preprocess(g, this, out)),
        );
        // copyBuffers (stdcall)
        t.stub(r::MEMCPY, false, 3, 0, RET);
        for n in [0u32, 2] {
            for nulls in [false, true] {
                let src = t.object(&mut rng, 0x30);
                t.w32(src + 0x20, n);
                t.w32(src + 0x24, n + 1);
                let idx = t.object(&mut rng, 0x20);
                t.w32(src + 0x1c, idx);
                let outs = t.object(&mut rng, 12);
                for k in 0..3 {
                    let o = if nulls && k == 1 {
                        0
                    } else {
                        t.object(&mut rng, 0x40)
                    };
                    t.w32(outs + 4 * k, o);
                }
                let mesh = t.object(&mut rng, 0x40);
                let dstidx = t.object(&mut rng, 0x20);
                t.w32(mesh + 0x38, dstidx);
                t.check(
                    &format!("{name} 0x6764b0 {n} {nulls}"),
                    0x0067_64b0,
                    Conv::Stdcall,
                    &[0, src, outs, mesh],
                    |g| {
                        Out::U32(ffx_pother_b::p4_cloth::bridge::copy_buffers(
                            g, src, outs, mesh,
                        ))
                    },
                );
            }
        }
        for (n, null) in [(0u32, false), (3, false), (2, true)] {
            let p = if null {
                0
            } else {
                t.object(&mut rng, 0x30 * 3)
            };
            if null {
                t.a.map_zeroed(0, 0x1000).ok();
                t.b.map_zeroed(0, 0x1000).ok();
                // a filled slot at 0, which the original leaves
                t.write(0, &[0x5a; 0x30]);
            }
            t.check(
                &format!("{name} 0xa2d1a0 {n} {null}"),
                0x00a2_d1a0,
                Conv::Cdecl,
                &[p, n],
                |g| {
                    r::bridge::font_chars(g, p, n);
                    Out::Void
                },
            );
        }
    }
}

#[test]
fn batch4_application_helpers_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x4680_2468);
        // the cluster releases: each id set or not, active or not
        let manager = t.object(&mut rng, 0x10);
        restub(&mut t, r::CLUSTERS[0], false, 0, 0, manager);
        restub(&mut t, r::CLUSTERS[2], true, 1, 1, RET);
        for mask in 0..8u32 {
            for active in [0x1234_5600u32, 0x1234_5601] {
                restub(&mut t, r::CLUSTERS[1], true, 1, 1, active);
                for (i, &g0) in r::CLUSTER_IDS.iter().enumerate() {
                    let at = t.rebase(g0);
                    t.w32(
                        at,
                        if mask >> i & 1 != 0 {
                            0x77 + i as u32
                        } else {
                            0
                        },
                    );
                }
                t.check(
                    &format!("{name} 0x6dfdd0 {mask} {active:#x}"),
                    0x006d_fdd0,
                    Conv::Cdecl,
                    &[],
                    |g| {
                        r::bridge::release_clusters(g);
                        Out::Void
                    },
                );
            }
        }
        restub(&mut t, r::IGGY_MENU_DTOR, true, 0, 0, RET);
        restub(&mut t, r::OPERATOR_DELETE, false, 1, 0, RET ^ 1);
        restub(&mut t, r::SET_MUTE_SOUND, false, 1, 0, RET ^ 2);
        for menu in [0u32, 0x0066_0000] {
            let at = t.rebase(r::MENU);
            t.w32(at, menu);
            t.check(
                &format!("{name} 0x6f72a0 {menu:#x}"),
                0x006f_72a0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(r::bridge::close_menu(g)),
            );
        }
        for table in [false, true] {
            let tb = if table {
                let tb = t.zeros(0x1c);
                for i in 0..7 {
                    t.w32(tb + 4 * i, if i % 3 == 1 { 0 } else { 0x0066_0000 + i });
                }
                tb
            } else {
                0
            };
            let at = t.rebase(r::MENU_TABLE);
            t.w32(at, tb);
            t.check(
                &format!("{name} 0x6f7820 {table}"),
                0x006f_7820,
                Conv::Cdecl,
                &[],
                |g| {
                    r::bridge::close_menus(g);
                    Out::Void
                },
            );
        }
        restub(&mut t, r::SUBTITLE[1], false, 1, 0, RET ^ 3);
        for battle in [0u32, 7] {
            for word in [0u32, 1] {
                for muted in [0u8, 1, 2] {
                    restub(&mut t, r::SUBTITLE[0], false, 0, 0, battle);
                    let w = t.rebase(r::MENU_WORDS[1]);
                    t.w32(w, word);
                    let m0 = t.rebase(r::SUBTITLE_MUTED);
                    t.write(m0, &[muted]);
                    t.check(
                        &format!("{name} 0x6f7230 {battle} {word} {muted}"),
                        0x006f_7230,
                        Conv::Cdecl,
                        &[],
                        |g| Out::U32(r::bridge::subtitle_mute(g)),
                    );
                }
            }
        }
    }
}

#[test]
fn batch5_swaps_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5a5a_1234);
        let p = t.object(&mut rng, 0x40);
        t.check(
            &format!("{name} 0x47f880"),
            s::HEADER,
            Conv::Thiscall(p),
            &[],
            |g| {
                s::bridge::swap_words(g, 8, p);
                Out::Void
            },
        );
        for (f, words) in [(s::CLASSES, 9u32), (s::MEMBERS, 6)] {
            for n in [0u32, 1, 3] {
                let p = t.object(&mut rng, 0x100);
                t.check(
                    &format!("{name} {f:#x} {n}"),
                    f,
                    Conv::Cdecl,
                    &[n, p],
                    |g| {
                        s::bridge::swap_records(g, words, n, p);
                        Out::Void
                    },
                );
            }
        }
        // the block: the swaps stubbed (each has its own test)
        t.stub(s::HEADER, true, 0, 0, RET);
        t.stub(s::CLASSES, false, 2, 0, RET);
        t.stub(s::MEMBERS, false, 2, 0, RET);
        t.stub(s::WORDS, false, 2, 0, RET);
        for _ in 0..3 {
            let block = t.object(&mut rng, 0x40);
            for k in 2..5 {
                t.w32(block + 4 * k, rng.next() % 50);
            }
            let calls = t.check(
                &format!("{name} 0x47d680"),
                s::SWAP_BLOCK,
                Conv::Thiscall(block),
                &[],
                |g| {
                    s::bridge::swap_block(g, block);
                    Out::Void
                },
            );
            assert_eq!(calls, 4);
        }
    }
}

#[test]
fn batch5_entries_and_reads_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x6b6b_4321);
        // a tree of entries: children lists and member sub-entries
        for round in 0..6u32 {
            let mk = |t: &mut Twins, rng: &mut Rng, cached: bool| {
                let e = t.object(rng, 0x20);
                t.w32(e, 0);
                t.w32(e + 4, 0);
                t.w32(e + 0x14, 1 << (rng.next() % 8));
                t.w32(e + 0x1c, if cached { rng.next() & 0xff } else { u32::MAX });
                e
            };
            let root = mk(&mut t, &mut rng, false);
            let c1 = mk(&mut t, &mut rng, round % 2 == 0);
            let c2 = mk(&mut t, &mut rng, false);
            t.w32(root, c1);
            t.w32(c1, c2);
            if round >= 2 {
                let class = t.object(&mut rng, 0x10);
                t.w32(class + 0xc, 4);
                t.w32(root + 4, class);
                let members = t.zeros(0x80);
                t.w32(root + 0x10, members);
                for i in 0..4u32 {
                    let m = t.object(&mut rng, 0x18);
                    t.w32(m + 0x14, [0u32, 2, 0xfd, 0][i as usize]);
                    let sub = if i == 3 && round == 3 {
                        0
                    } else {
                        mk(&mut t, &mut rng, false)
                    };
                    t.w32(
                        members + 0x20 * i + 8,
                        if i == 2 && round == 4 { 0 } else { m },
                    );
                    t.w32(members + 0x20 * i + 0x14, sub);
                }
            }
            t.check(
                &format!("{name} 0x47dc20 {round}"),
                0x0047_dc20,
                Conv::Thiscall(root),
                &[],
                |g| Out::U32(s::bridge::needs_conversion(g, root)),
            );
        }
        t.stub(s::READ_EXACT, true, 2, 2, RET);
        t.stub(s::SWAP_BLOCK, true, 0, 0, RET ^ 1);
        for (block, size) in [(0u32, 0u32), (0x100, 0), (0, 0x10), (0x100, 0x10)] {
            for swap in [0u32, 1, 0x100] {
                let reader = rng.next();
                let calls = t.check(
                    &format!("{name} 0x579010 {block:#x} {size:#x} {swap:#x}"),
                    0x0057_9010,
                    Conv::Cdecl,
                    &[reader, swap, block, size],
                    |g| Out::U32(s::bridge::read_block(g, reader, swap, block, size)),
                );
                assert_eq!(
                    calls,
                    u32::from(block != 0 && size != 0) + u32::from(block != 0 && swap & 0xff != 0)
                );
            }
        }
    }
}

#[test]
fn batch5_bind_and_bounds_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x7c7c_5678);
        let method = t.stub_code(0x110, false, 5, 5, RET, Log::Words);
        for (has_ctx, count, index, buffer) in [
            (false, 1u32, 0u32, 1u32),
            (true, 0, 0, 1),
            (true, 0, 1, 1),
            (true, 3, 2, 1),
            (true, 3, 3, 1),
            (true, 0x8000_0002, 1, 0),
            (true, 2, 1, 1),
            (true, 0x8000_0000, 1, 1),
        ] {
            restub(
                &mut t,
                s::BUFFER_AT,
                true,
                1,
                1,
                if buffer == 0 { 0 } else { 0x0044_5560 },
            );
            let ctx = if has_ctx {
                let vt = t.zeros(0x120);
                t.w32(vt + 0x110, method);
                let c = t.object(&mut rng, 8);
                t.w32(c, vt);
                c
            } else {
                0
            };
            let this = t.object(&mut rng, 0x80);
            t.w32(this + 0x70, ctx);
            let array = t.object(&mut rng, 0x20);
            t.w32(array + 0x14, count);
            let slot = rng.next();
            // the original's argument slots are on its stack; the port's
            // are a block at the same place in both oracles' logs (the log
            // records the pointers, so they are compared through the
            // values they point at instead)
            let slots = t.zeros(12);
            t.w32(slots, array);
            t.w32(slots + 4, slot);
            t.w32(slots + 8, index);
            let addr = t.rebase(0x0059_8d00);
            let log = t.log;
            t.w32(log, 0);
            let ra =
                t.a.call(addr, Conv::Thiscall(this), &[array, slot, index])
                    .unwrap();
            let na: u32 = Mem::read(&t.a, log);
            let ea: Vec<[u32; 8]> = (0..na)
                .map(|i| core::array::from_fn(|k| Mem::read(&t.a, log + 4 + i * 32 + 4 * k as u32)))
                .collect();
            t.b.write(log, &0u32.to_le_bytes()).unwrap();
            let port = s::bridge::bind_buffer(&mut G(&mut t.b, t.pop), slots, this, array, index);
            assert_eq!(ra.eax, port, "{name} 0x598d00 {count:#x} {index}");
            let nb: u32 = Mem::read(&t.b, log);
            assert_eq!(na, nb, "{name} calls");
            for (i, e) in ea.iter().enumerate() {
                let mut eb = [0u32; 8];
                for (k, w) in eb.iter_mut().enumerate() {
                    *w = Mem::read(&t.b, log + 4 + i as u32 * 32 + 4 * k as u32);
                }
                // tag, (no ecx), ctx, 0, 1: the same; the two pointers differ
                assert_eq!(e[..5], eb[..5], "{name} call {i}");
            }
            if na == 4 {
                // the first call's buffer slot holds the buffer
                let p: u32 = Mem::read(&t.b, slots);
                assert_eq!(p, 0x0044_5560);
                // and the index slot, handed to the second call, cleared
                let q: u32 = Mem::read(&t.b, slots + 8);
                assert_eq!(q, 0);
            }
            // the two oracles' logs differ only in the stack pointers
            let la: Vec<u8> = {
                let mut v = vec![0u8; 4 + 32 * na as usize];
                t.b.read_bytes(log, &mut v);
                v
            };
            t.a.write(log, &la).unwrap();
            let sb: Vec<u8> = {
                let mut v = vec![0u8; 12];
                t.b.read_bytes(slots, &mut v);
                v
            };
            t.a.write(slots, &sb).unwrap();
            t.diff(&format!("{name} 0x598d00"));
        }
        let floats = [0.0f32, -0.0, 1.0, -1.0, 2.5, f32::NAN, f32::INFINITY, -3.0];
        for (f, max) in [(0x00a1_b560u32, true), (0x00a1_b620, false)] {
            for i in 0..floats.len() {
                let a = t.zeros(16);
                let b = t.zeros(16);
                for k in 0..4 {
                    t.w32(a + 4 * k, floats[(i + k as usize) % floats.len()].to_bits());
                    t.w32(
                        b + 4 * k,
                        floats[(i * 3 + 2 * k as usize + 1) % floats.len()].to_bits(),
                    );
                }
                for out in [t.zeros(16), a] {
                    t.check(
                        &format!("{name} {f:#x} {i}"),
                        f,
                        Conv::Cdecl,
                        &[out, a, b],
                        |g| Out::U32(s::bridge::min_max(g, max, out, a, b)),
                    );
                }
            }
        }
    }
}

/// An object whose virtual slot 1 returns `ret` (a recording stub tagged
/// `tag`), with `extra` zeroed bytes after the vtable word.
fn slot1_object(t: &mut Twins, tag: u32, ret: u32, extra: usize) -> u32 {
    let f = t.stub_code(tag, true, 0, 0, ret, Log::Words);
    let vtable = t.zeros(0x10);
    t.w32(vtable + 4, f);
    let obj = t.zeros(4 + extra);
    t.w32(obj, vtable);
    obj
}

/// A runtime member: slot 1 returns a class descriptor of `kind` (none for
/// `None`), flags, its type object (slot 1 returns `ty`, size at `+0x1c`),
/// its offset.
fn runtime_member(
    t: &mut Twins,
    kind: Option<u32>,
    flags: u32,
    ty: u32,
    size: u32,
    offset: u32,
) -> u32 {
    let desc = match kind {
        None => 0,
        Some(k) => {
            let d = t.zeros(0x40);
            t.w32(d + 0x30, k);
            d
        }
    };
    let rt = slot1_object(t, 0xb1, desc, 0x40);
    let tyo = slot1_object(t, 0xb2, ty, 0x20);
    t.w32(tyo + 0x1c, size);
    t.w32(rt + 0x14, flags);
    t.w32(rt + 0x18, tyo);
    t.w32(rt + 0x24, offset);
    t.w32(rt + 0x34, 3);
    rt
}

#[test]
fn batch6_mapping_flags_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0606_0606);
        for same_gen in [false, true] {
            for n in [0u32, 1, 3] {
                let entries = t.zeros(32 * 4);
                for i in 0..n {
                    let e = entries + 32 * i;
                    t.w32(e + 0x14, 1 << i);
                    t.w32(e + 0x1c, rng.next());
                    // entry 1's child list: entry 2
                    if i == 1 && n == 3 {
                        t.w32(e, entries + 64);
                    }
                }
                let mapping = t.object(&mut rng, 0x20);
                t.w32(mapping + 0xc, n | 0x8000_0000);
                t.w32(mapping + 0x10, entries);
                let g = t.rebase(mp::GENERATION);
                let generation = rng.next();
                t.w32(g, generation);
                t.w32(
                    mapping + 0x1c,
                    if same_gen { generation } else { generation ^ 1 },
                );
                t.check(
                    &format!("{name} 0x47c080 {same_gen} {n}"),
                    0x0047_c080,
                    Conv::Thiscall(mapping),
                    &[],
                    |g| {
                        mp::bridge::update_flags(g, mapping);
                        Out::Void
                    },
                );
            }
        }
    }
}

/// A member record case: runtime member present, its type (0x222: the
/// record's), descriptor kind, flags, runtime size, stored size, stored
/// count.
type Case = (bool, u32, Option<u32>, u32, u32, u32, u32);

#[test]
fn batch6_kernel_assignment_matches_the_original() {
    for (name, mut t) in Twins::all() {
        t.stub(mp::MEMBER_COUNT, true, 0, 0, 3);
        for found in [0u32, 0x0047_4210] {
            restub(&mut t, mp::FIND_KERNEL, false, 2, 0, found);
            for dir in [0u32, 1, 0x100] {
                // records: no runtime member; other type; same type with a
                // size mismatch, pointer flags, by-reference arrays, fixed
                // arrays with equal and different counts, one-byte types
                let cases: [Case; 11] = [
                    // (runtime, type, kind, flags, rsize, stored size, stored count)
                    (false, 0, None, 0, 4, 4, 0),
                    (true, 0x111, Some(0), 0, 4, 4, 0),
                    (true, 0x222, Some(0), 0, 8, 4, 0),
                    (true, 0x222, None, 2, 8, 4, 0),
                    (true, 0x222, Some(2), 0, 8, 4, 0),
                    (true, 0x222, Some(2), 0x40, 8, 8, 0),
                    (true, 0x222, Some(1), 0, 4, 4, 3),
                    (true, 0x222, Some(1), 1, 4, 4, 5),
                    (true, 0x222, None, 0, 1, 1, 0),
                    (true, 0x222, Some(0), 0, 1, 1, 0),
                    (true, 0x222, Some(1), 0, 4, 4, 5),
                ];
                let records = t.zeros(32 * cases.len());
                for (i, &(has_rt, ty, kind, flags, rsize, ssize, scount)) in
                    cases.iter().enumerate()
                {
                    let e = records + 32 * i as u32;
                    let st = t.zeros(0x20);
                    t.w32(st + 8, 0x10);
                    t.w32(st + 0xc, ssize);
                    t.w32(st + 0x10, flags & !1);
                    t.w32(st + 0x14, scount);
                    if i == 10 {
                        // offsets differ: the count check is skipped
                        t.w32(st + 8, 0x14);
                    }
                    t.w32(e + 4, st);
                    t.w32(e + 0x10, 0x222);
                    if has_rt {
                        let rt = runtime_member(&mut t, kind, flags, ty, rsize, 0x10);
                        if ty == 0x222 {
                            // same type: the record's type is the runtime's
                            let tyo: u32 = Mem::read(&t.b, rt + 0x18);
                            t.w32(e + 0x10, tyo);
                        }
                        t.w32(e + 8, rt);
                    }
                }
                let class = t.zeros(0x10);
                t.w32(class + 0xc, cases.len() as u32);
                let entry = t.zeros(0x20);
                t.w32(entry + 4, class);
                t.w32(entry + 0x10, records);
                // a chained entry: one record without a runtime member
                let next = t.zeros(0x20);
                let nclass = t.zeros(0x10);
                t.w32(nclass + 0xc, 1);
                t.w32(next + 4, nclass);
                let nrec = t.zeros(0x20);
                t.w32(next + 0x10, nrec);
                t.w32(entry, next);
                t.check(
                    &format!("{name} 0x47e180 {found:#x} {dir:#x}"),
                    0x0047_e180,
                    Conv::Thiscall(entry),
                    &[dir],
                    |g| {
                        mp::bridge::assign_kernels(g, entry, dir);
                        Out::Void
                    },
                );
            }
        }
    }
}

#[test]
fn batch6_driver_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0808_0808);
        t.stub(mp::MEMBER_COUNT, true, 0, 0, 2);
        t.stub(mp::COPY_KERNEL, false, 4, 0, RET);
        t.stub(mp::BYTE_KERNEL, false, 4, 0, RET ^ 1);
        let own = t.stub_code(0xc0, false, 4, 0, RET ^ 2, Log::Words);
        for dir in [0u32, 1] {
            for variant in 0..4u32 {
                // the child mapping: one plain member with its own kernel
                let child = t.zeros(0x20);
                let c4 = t.zeros(0x20);
                t.w32(c4 + 0xc, 1);
                t.w32(c4 + 0x10, 0x30);
                let c8 = t.zeros(0x90);
                t.w32(c8 + 0x84, 0x40);
                t.w32(child + 4, c4);
                t.w32(child + 8, c8);
                let crec = t.zeros(0x20);
                let cty = 0x0033_0000;
                let crt = runtime_member(&mut t, Some(0), 0, cty, 4, 0x8);
                let cst = t.zeros(0x20);
                t.w32(cst + 8, 0xc);
                let csub = slot1_object(&mut t, 0xb3, cty, 0);
                t.w32(crec + 4, cst);
                t.w32(crec + 8, crt);
                t.w32(crec + 0x10, csub);
                t.w32(crec + 0x1c, own);
                t.w32(child + 0x10, crec);
                // the mapping: records of each kind
                let ty = 0x0044_0000 + variant;
                let kinds: [(Option<u32>, u32, u32, bool); 8] = [
                    // (desc kind, flags, size, with a child mapping)
                    (Some(0), 0, 4, false),
                    (None, 0, 1, false),
                    (Some(1), 0, 8, true),
                    (Some(2), 0, 8, true),
                    (Some(2), 0x40, 8, true),
                    (Some(0), 2, 8, true),
                    (Some(0), 0, 4, true),
                    (Some(0), 0, 1, false),
                ];
                let recs = t.zeros(32 * (kinds.len() + 3));
                for (i, &(kind, flags, size, with_child)) in kinds.iter().enumerate() {
                    let e = recs + 32 * i as u32;
                    let rt = runtime_member(&mut t, kind, flags, ty, size, 0x10 + 4 * i as u32);
                    let st = t.zeros(0x20);
                    t.w32(st + 8, 0x20 + 4 * i as u32);
                    let sub = slot1_object(&mut t, 0xb4, ty, 0);
                    t.w32(e + 4, st);
                    t.w32(e + 8, rt);
                    t.w32(e + 0x10, sub);
                    if with_child {
                        t.w32(e + 0x14, child);
                    }
                    if i == 0 && variant % 2 == 1 {
                        t.w32(e + 0x1c, own);
                    }
                }
                // a record with a different sub-type, one without a stored
                // member, one without a sub-type
                let base = recs + 32 * kinds.len() as u32;
                let rt = runtime_member(&mut t, Some(0), 0, ty, 4, 0);
                let st = t.zeros(0x20);
                let other = slot1_object(&mut t, 0xb5, ty ^ 0x100, 0);
                t.w32(base + 4, st);
                t.w32(base + 8, rt);
                t.w32(base + 0x10, other);
                t.w32(base + 32 + 8, rt);
                t.w32(base + 64 + 4, st);
                t.w32(base + 64 + 8, rt);
                let m4 = t.zeros(0x20);
                t.w32(m4 + 0xc, kinds.len() as u32 + 3);
                t.w32(m4 + 0x10, 0x100);
                let m8 = t.zeros(0x90);
                t.w32(m8 + 0x84, 0x200);
                t.w32(m8 + 0x40, 0x5555);
                let mapping = t.zeros(0x20);
                t.w32(mapping + 4, m4);
                t.w32(mapping + 8, m8);
                t.w32(mapping + 0x10, recs);
                // a base mapping: kept (its +8 the stored class's +0x40)
                // or skipped (variant 2, 3), with no entries itself
                if variant >= 1 {
                    let basem = t.zeros(0x20);
                    let b4 = t.zeros(0x20);
                    t.w32(b4 + 0x10, 0x8);
                    t.w32(b4 + 0xc, 2);
                    let b8 = t.zeros(0x90);
                    t.w32(b8 + 0x84, 0x4);
                    t.w32(basem + 4, if variant == 3 { 0 } else { b4 });
                    t.w32(basem + 8, b8);
                    t.w32(basem + 0x10, recs);
                    t.w32(mapping, basem);
                    // kept when the base's +8 is the stored class's +0x40
                    t.w32(m8 + 0x40, if variant == 2 { 0x5555 } else { b8 });
                }
                let counts = t.zeros(16);
                t.w32(counts, 2);
                t.w32(counts + 4, 0x40);
                t.w32(counts + 8, 0x50);
                t.w32(counts + 0xc, dir);
                let (src, dst) = (
                    0x0100_0000 + rng.next() % 0x1000,
                    0x0200_0000 + rng.next() % 0x1000,
                );
                t.check(
                    &format!("{name} 0x47c610 {dir} {variant}"),
                    mp::DRIVER,
                    Conv::Thiscall(mapping),
                    &[counts, src, dst],
                    |g| Out::U32(mp::bridge::driver(g, mapping, counts, src, dst)),
                );
            }
        }
    }
}

#[test]
fn batch7_outcodes_match_the_original() {
    for (name, mut t) in Twins::all() {
        let vals = [-2.0f32, -1.0, 0.0, 0.5, 1.0, 3.0, f32::NAN, -0.0];
        let mut rng = Rng(0x0909_0909);
        for _ in 0..60 {
            let mk = |t: &mut Twins, rng: &mut Rng| {
                let p = t.zeros(12);
                for k in 0..3 {
                    let v = vals[(rng.next() % vals.len() as u32) as usize];
                    t.w32(p + 4 * k, v.to_bits());
                }
                p
            };
            let (min, max, p) = (
                mk(&mut t, &mut rng),
                mk(&mut t, &mut rng),
                mk(&mut t, &mut rng),
            );
            t.check(
                &format!("{name} 0x5a4180"),
                0x005a_4180,
                Conv::Cdecl,
                &[min, max, p],
                |g| Out::U32(gm::bridge::outcode(g, min, max, p)),
            );
        }
    }
}

#[test]
fn batch7_bone_palettes_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0a0a_0a0a);
        for n in [0u32, 1, 3] {
            for with_out in [false, true] {
                let mats = t.zeros(64 * 6);
                let bones = t.zeros(64 * 4);
                for k in 0..(16 * 6) {
                    t.w32(
                        mats + 4 * k,
                        ((rng.next() % 2000) as f32 / 500.0 - 2.0).to_bits(),
                    );
                }
                for k in 0..(16 * 4) {
                    t.w32(
                        bones + 4 * k,
                        ((rng.next() % 2000) as f32 / 500.0 - 2.0).to_bits(),
                    );
                }
                let pairs = t.zeros(16);
                for i in 0..3 {
                    t.write(pairs + 4 * i, &((1 + rng.next() % 5) as u16).to_le_bytes());
                    t.write(pairs + 4 * i + 2, &((rng.next() % 4) as u16).to_le_bytes());
                }
                let d = t.zeros(0x10);
                t.w32(d + 8, n | 0x8000_0000);
                t.w32(d + 0xc, pairs);
                let b1 = t.zeros(0x10);
                t.w32(b1 + 0xc, bones);
                let b0 = t.zeros(4);
                t.w32(b0, b1);
                let s = t.zeros(0x20);
                t.w32(s + 0x10, b0);
                t.w32(s + 0x14, mats + 0x40);
                t.w32(s + 0x1c, d);
                let skin = t.zeros(4);
                t.w32(skin, s);
                let out = if with_out { t.zeros(64 * 3) } else { 0 };
                for (f, rel) in [(0x005c_8830u32, false), (0x005c_8700, true)] {
                    let frame = t.zeros(0xc8);
                    t.sync.push((frame, 0xc8));
                    t.check(
                        &format!("{name} {f:#x} {n} {with_out}"),
                        f,
                        Conv::Cdecl,
                        &[out, skin],
                        |g| {
                            Out::U32(if rel {
                                gm::bridge::palette_relative(g, frame, out, skin)
                            } else {
                                gm::bridge::palette(g, frame, out, skin)
                            })
                        },
                    );
                }
            }
        }
    }
}

#[test]
fn batch7_clipper_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0b0b_0b0b);
        // the distance callback: FFXApplication_axisX_00a2f780 (x - 1)
        let distance = t.rebase(0x00a2_f780);
        let xs = [0.0f32, 2.0, 1.0, 3.0, -1.0, 0.5, 2.5, f32::NAN, 1.5];
        for n in [0u32, 1, 3, 4, 6] {
            for round in 0..4 {
                let input = t.zeros(16 * 8);
                for i in 0..n {
                    let x = xs[((i + round * 3) as usize + rng.next() as usize % 2) % xs.len()];
                    t.w32(input + 16 * i, x.to_bits());
                    for k in 1..4 {
                        t.w32(
                            input + 16 * i + 4 * k,
                            ((rng.next() % 100) as f32 / 10.0).to_bits(),
                        );
                    }
                }
                let out = t.object(&mut rng, 16 * 16);
                let count = t.object(&mut rng, 4);
                let frame = t.zeros(16);
                t.sync.push((frame, 16));
                t.check(
                    &format!("{name} 0xa2f5e0 {n} {round}"),
                    0x00a2_f5e0,
                    Conv::Cdecl,
                    &[input, n, out, count, distance],
                    |g| Out::U32(gm::bridge::clip(g, frame, input, n, out, count, distance)),
                );
            }
        }
    }
}

#[test]
fn batch8_point_in_box_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let vals = [-2.0f32, -1.0, 0.0, 0.5, 1.0, 3.0, f32::NAN, -0.0];
        let mut rng = Rng(0x0c0c_0c0c);
        for _ in 0..80 {
            let mk = |t: &mut Twins, rng: &mut Rng| {
                let p = t.zeros(12);
                for k in 0..3 {
                    let v = vals[(rng.next() % vals.len() as u32) as usize];
                    t.w32(p + 4 * k, v.to_bits());
                }
                p
            };
            let (p, min, max) = (
                mk(&mut t, &mut rng),
                mk(&mut t, &mut rng),
                mk(&mut t, &mut rng),
            );
            t.check(
                &format!("{name} 0xa2fc90"),
                0x00a2_fc90,
                Conv::Cdecl,
                &[p, min, max],
                |g| Out::U32(ct::bridge::inside(g, p, min, max)),
            );
        }
    }
}

#[test]
fn batch8_convert_elements_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0d0d_0d0d);
        t.stub(ct::MEMCPY, false, 3, 0, RET);
        // the driver logs its counts (the struct its first stack word points at)
        t.stub_log(ct::DRIVER, true, 0, 3, RET ^ 1, Log::VisitorArgs);
        for (buf, n) in [(0u32, 3u32), (0x0012_3400, 0), (0x0012_3400, 3)] {
            restub(&mut t, ct::DEFAULT_BUFFER, true, 0, 0, buf);
            let rt = t.object(&mut rng, 0x90);
            t.w32(rt + 0x1c, 0x10);
            let st = t.object(&mut rng, 0x20);
            let entry = t.object(&mut rng, 0x10);
            t.w32(entry + 4, st);
            t.w32(entry + 8, rt);
            let (src, dst) = (rng.next() & !3, rng.next() & !3);
            let frame = t.zeros(16);
            t.sync.push((frame, 16));
            t.check(
                &format!("{name} 0x5788f0 {buf:#x} {n}"),
                0x0057_88f0,
                Conv::Cdecl,
                &[entry, src, dst, n],
                |g| Out::U32(ct::bridge::convert_elements(g, frame, entry, src, dst, n)),
            );
        }
    }
}

#[test]
fn batch8_contact_arrays_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0e0e_0e0e);
        t.stub(ct::PHYRE_FREE, false, 1, 0, RET);
        for &(f, words, copy, ctor) in ct::RESIZES {
            assert_eq!(ct::resize_of(f), (f, words, copy, ctor));
            t.stub(copy, false, 3, 0, RET ^ 1);
            if ctor != 0 {
                t.stub(ctor, false, 2, 0, RET ^ 2);
            }
            for fail in [false, true] {
                for (n, word, old) in [
                    (0u32, 2u32, 1u32),
                    (2, 2, 1),
                    (3, 2, 1),
                    (1, 5, 1),
                    (4, 0x8000_0002, 1),
                    (2, 1, 0),
                    (0, 0, 0),
                ] {
                    let heap = t.object(&mut rng, 0x200);
                    restub(
                        &mut t,
                        ct::PHYRE_MALLOC,
                        false,
                        2,
                        0,
                        if fail { 0 } else { heap },
                    );
                    let this = t.object(&mut rng, 8);
                    t.w32(this, word);
                    let o = if old == 0 {
                        0
                    } else {
                        t.object(&mut rng, 0x40)
                    };
                    t.w32(this + 4, o);
                    t.check(
                        &format!("{name} {f:#x} {n} {word:#x} {fail}"),
                        f,
                        Conv::Thiscall(this),
                        &[n],
                        |g| Out::U32(ct::bridge::resize(g, words, copy, ctor, this, n)),
                    );
                }
            }
        }
        // the append (resizes stubbed), then the wrapper on the real append
        restub(&mut t, ct::RESIZE[0], true, 1, 1, 0);
        restub(&mut t, ct::RESIZE[1], true, 1, 1, 0);
        for (count, cap) in [(0u32, 4u32), (3, 4), (4, 4), (0, 0)] {
            let this = t.object(&mut rng, 0x100);
            t.w32(this + 0x3c, count);
            t.w32(this + 0x90, cap);
            let data = t.object(&mut rng, 36 * 8);
            t.w32(this + 0x84, data);
            let a = t.object(&mut rng, 12);
            let b = t.object(&mut rng, 12);
            let index = t.object(&mut rng, 4);
            let c = rng.next();
            t.check(
                &format!("{name} 0x5f36d0 {count} {cap}"),
                ct::APPEND,
                Conv::Thiscall(this),
                &[a, b, index, c],
                |g| Out::U32(ct::bridge::append(g, this, a, b, index, c)),
            );
            let this2 = t.object(&mut rng, 0x100);
            t.w32(this2 + 0x3c, count);
            t.w32(this2 + 0x90, cap);
            t.w32(this2 + 0x84, data);
            let frame = t.zeros(4);
            t.sync.push((frame, 4));
            t.check(
                &format!("{name} 0x5f31e0 {count} {cap}"),
                0x005f_31e0,
                Conv::Thiscall(this2),
                &[a, b, c],
                |g| Out::U32(ct::bridge::add_contact(g, frame, this2, a, b, c)),
            );
        }
    }
}

#[test]
fn batch9_skin_kernel_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0f0f_0f0f);
        // Two value sets that never let two different NaNs meet: the
        // emulator picks between two NaN operands of SSE code as the x87
        // does (larger significand, then positive), the CPU takes the
        // destination's, so tests keep to one NaN at a time: overflow and
        // invalid operations (the indefinite NaN), or quiet NaN inputs.
        let invalid = [
            0.0f32,
            -0.0,
            1.0,
            f32::INFINITY,
            f32::from_bits(0x0001_0000),
            -3.0e38,
        ];
        let quiet = [0.0f32, -0.0, 1.0, f32::NAN, f32::from_bits(0x0001_0000)];
        for n in [0u32, 1, 2, 3, 4, 5, 8] {
            for special in [0u32, 1, 2] {
                let specials: &[f32] = if special == 1 { &invalid } else { &quiet };
                // 16-byte aligned bones (the SSE loop's movaps)
                let raw = t.zeros(64 * 6 + 16);
                let bones = (raw + 15) & !15;
                for k in 0..(16 * 6) {
                    let v = if special != 0 && rng.next() % 9 == 0 {
                        specials[(rng.next() % specials.len() as u32) as usize]
                    } else {
                        (rng.next() % 4000) as f32 / 1000.0 - 2.0
                    };
                    t.w32(bones + 4 * k, v.to_bits());
                }
                let rnd = |t: &mut Twins, rng: &mut Rng, words: u32| {
                    let at = t.zeros(4 * words as usize + 16);
                    for k in 0..words {
                        let v = (rng.next() % 2000) as f32 / 700.0 - 1.0;
                        t.w32(at + 4 * k, v.to_bits());
                    }
                    at
                };
                let pos = rnd(&mut t, &mut rng, 3 * n);
                let nrm = rnd(&mut t, &mut rng, 3 * n);
                let w = rnd(&mut t, &mut rng, 4 * n);
                let idx = t.zeros(4 * n as usize + 4);
                for k in 0..(4 * n) {
                    t.write(idx + k, &[(rng.next() % 6) as u8]);
                }
                let src = t.zeros(16);
                t.w32(src, pos);
                t.w32(src + 4, nrm);
                t.w32(src + 8, w);
                t.w32(src + 0xc, idx);
                let pout = t.object(&mut rng, 12 * n as usize + 16);
                let nout = t.object(&mut rng, 12 * n as usize + 16);
                let dst = t.zeros(8);
                t.w32(dst, pout);
                t.w32(dst + 4, nout);
                t.check(
                    &format!("{name} 0x5c8930 {n} {special} pout {pout:#x} nout {nout:#x}"),
                    0x005c_8930,
                    Conv::Cdecl,
                    &[bones, n, src, dst],
                    |g| {
                        sk::bridge::skin4(g, bones, n, src, dst);
                        Out::Void
                    },
                );
            }
        }
        // positions and normals in one buffer: the reads after the normal store
        let raw = t.zeros(64 * 6 + 16);
        let bones = (raw + 15) & !15;
        for k in 0..(16 * 6) {
            t.w32(
                bones + 4 * k,
                ((rng.next() % 400) as f32 / 100.0 - 2.0).to_bits(),
            );
        }
        let shared = t.object(&mut rng, 12 * 6 + 16);
        let w = t.object(&mut rng, 16 * 5);
        let idx = t.zeros(24);
        let src = t.zeros(16);
        t.w32(src, shared);
        t.w32(src + 4, shared);
        t.w32(src + 8, w);
        t.w32(src + 0xc, idx);
        let dst = t.zeros(8);
        t.w32(dst, shared + 4);
        t.w32(dst + 4, shared);
        t.check(
            &format!("{name} 0x5c8930 aliased"),
            0x005c_8930,
            Conv::Cdecl,
            &[bones, 5, src, dst],
            |g| {
                sk::bridge::skin4(g, bones, 5, src, dst);
                Out::Void
            },
        );
    }
}

#[test]
fn batch10_skin_positions_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x1010_1010);
        // one NaN source at a time (see batch9_skin_kernel_matches_the_original)
        let invalid = [
            0.0f32,
            -0.0,
            1.0,
            f32::INFINITY,
            f32::from_bits(0x0001_0000),
            -3.0e38,
        ];
        let quiet = [0.0f32, -0.0, 1.0, f32::NAN, f32::from_bits(0x0001_0000)];
        for n in [0u32, 1, 2, 3, 4, 5, 8] {
            for special in [0u32, 1, 2, 3] {
                let specials: &[f32] = if special == 1 { &invalid } else { &quiet };
                let raw = t.zeros(64 * 6 + 16);
                let bones = (raw + 15) & !15;
                for k in 0..(16 * 6) {
                    let v = if special != 0 && rng.next() % 9 == 0 {
                        specials[(rng.next() % specials.len() as u32) as usize]
                    } else {
                        (rng.next() % 4000) as f32 / 1000.0 - if special == 3 { 0.0 } else { 2.0 }
                    };
                    t.w32(bones + 4 * k, v.to_bits());
                }
                let pos = t.zeros(12 * n as usize + 16);
                for k in 0..(3 * n) {
                    t.w32(
                        pos + 4 * k,
                        ((rng.next() % 2000) as f32 / 700.0 - if special == 3 { 0.0 } else { 1.0 })
                            .to_bits(),
                    );
                }
                let w = t.zeros(16 * n as usize + 16);
                for k in 0..(4 * n) {
                    // -0 weights on positive values: every product -0, which a sum
                    // started from +0 would lose
                    let wv = if special == 3 {
                        -0.0
                    } else if rng.next() % 3 == 0 {
                        0.0
                    } else {
                        (rng.next() % 1000) as f32 / 999.0
                    };
                    t.w32(w + 4 * k, wv.to_bits());
                }
                let idx = t.zeros(4 * n as usize + 4);
                for k in 0..(4 * n) {
                    t.write(idx + k, &[(rng.next() % 6) as u8]);
                }
                let src = t.zeros(12);
                t.w32(src, pos);
                t.w32(src + 4, w);
                t.w32(src + 8, idx);
                let pout = t.object(&mut rng, 12 * n as usize + 16);
                let dst = t.zeros(4);
                t.w32(dst, pout);
                t.check(
                    &format!("{name} 0x5c94f0 {n} {special}"),
                    0x005c_94f0,
                    Conv::Cdecl,
                    &[bones, n, src, dst],
                    |g| {
                        sk::bridge::skin4_positions(g, bones, n, src, dst);
                        Out::Void
                    },
                );
            }
        }
    }
}

#[test]
fn batch11_skin_tangents_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x1111_2222);
        // one NaN source at a time (see batch9_skin_kernel_matches_the_original)
        let invalid = [
            0.0f32,
            -0.0,
            1.0,
            f32::INFINITY,
            f32::from_bits(0x0001_0000),
            -3.0e38,
        ];
        let quiet = [0.0f32, -0.0, 1.0, f32::NAN, f32::from_bits(0x0001_0000)];
        for n in [0u32, 1, 2, 3, 4, 5, 8] {
            for special in [0u32, 1, 2] {
                let specials: &[f32] = if special == 1 { &invalid } else { &quiet };
                let raw = t.zeros(64 * 6 + 16);
                let bones = (raw + 15) & !15;
                for k in 0..(16 * 6) {
                    let v = if special != 0 && rng.next() % 9 == 0 {
                        specials[(rng.next() % specials.len() as u32) as usize]
                    } else {
                        (rng.next() % 4000) as f32 / 1000.0 - 2.0
                    };
                    t.w32(bones + 4 * k, v.to_bits());
                }
                let mut vecs = [0u32; 3];
                for v in &mut vecs {
                    *v = t.zeros(12 * n as usize + 16);
                    for k in 0..(3 * n) {
                        t.w32(
                            *v + 4 * k,
                            ((rng.next() % 2000) as f32 / 700.0 - 1.0).to_bits(),
                        );
                    }
                }
                let w = t.zeros(16 * n as usize + 16);
                for k in 0..(4 * n) {
                    t.w32(w + 4 * k, ((rng.next() % 1000) as f32 / 999.0).to_bits());
                }
                let idx = t.zeros(4 * n as usize + 4);
                for k in 0..(4 * n) {
                    t.write(idx + k, &[(rng.next() % 6) as u8]);
                }
                let src = t.zeros(20);
                for (i, v) in vecs.iter().enumerate() {
                    t.w32(src + 4 * i as u32, *v);
                }
                t.w32(src + 0xc, w);
                t.w32(src + 0x10, idx);
                let dst = t.zeros(12);
                for i in 0..3 {
                    let o = t.object(&mut rng, 12 * n as usize + 16);
                    t.w32(dst + 4 * i, o);
                }
                t.check(
                    &format!("{name} 0x5c8e90 {n} {special}"),
                    0x005c_8e90,
                    Conv::Cdecl,
                    &[bones, n, src, dst],
                    |g| {
                        sk::bridge::skin4_tangents(g, bones, n, src, dst);
                        Out::Void
                    },
                );
            }
        }
    }
}

#[test]
fn batch12_utility_delete_matches_the_original() {
    use ffx_pcui::m5u_j::bridge as jj;
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x1212_3434);
        t.stub(0x0063_0390, false, 1, 0, RET);
        for n in 0..4u32 {
            for at in 0..=n {
                for flags in [0u32, 1, 0xff] {
                    let nodes: Vec<u32> = (0..n).map(|_| t.zeros(8)).collect();
                    for (k, &node) in nodes.iter().enumerate() {
                        let next = nodes.get(k + 1).copied().unwrap_or(0);
                        t.w32(node + 4, next);
                    }
                    let this = if at < n {
                        nodes[at as usize]
                    } else {
                        t.zeros(8)
                    };
                    t.w32(this, rng.next());
                    let head = t.rebase(0x00c9_01a8);
                    t.w32(head, nodes.first().copied().unwrap_or(0));
                    t.check(
                        &format!("{name} 0x9f4c20 {n} {at} {flags}"),
                        0x009f_4c20,
                        Conv::Thiscall(this),
                        &[flags],
                        |g| Out::U32(jj::utility_delete(g, this, flags)),
                    );
                }
            }
        }
    }
}
