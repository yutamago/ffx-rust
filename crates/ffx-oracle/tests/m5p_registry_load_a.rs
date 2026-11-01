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
use ffx_pload::m5p_registry_load_a_left::{self as lf, bridge as lb};
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
    /// `al` (the rest of `eax` is the caller's).
    Al(u8),
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
            Out::Al(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
        }
        self.diff(what);
    }

    /// [`Twins::check`] for a port whose callees see the original's frame
    /// by address: the port gets a frame at the original's address
    /// (`below` bytes under where the call pushes from) and its own calls
    /// run under it; the first `frame_len` bytes of both frames compared.
    #[allow(clippy::too_many_arguments)]
    fn check_frame(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        below: u32,
        frame_len: usize,
        port: impl FnOnce(&mut G, u32) -> Out,
    ) {
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let top = self.b.stack_range().end - 0x100;
        assert_eq!(top, self.a.stack_range().end - 0x100);
        self.b.set_call_top(Some(top - 0x400));
        let frame = top - below;
        let out = port(&mut G(&mut self.b), frame);
        self.b.set_call_top(None);
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        match out {
            Out::None | Out::U64(_) => {}
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::Al(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
        }
        if frame_len != 0 {
            let fa = Oracle::read(&mut self.a, frame, frame_len).unwrap();
            let fb = Oracle::read(&mut self.b, frame, frame_len).unwrap();
            assert_eq!(fa, fb, "{what}: the frame the callees saw");
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

// ---- round left2: the list's leftovers (ffx_pload::m5p_registry_load_a_left) ----

/// An object of `size` bytes (zero after its vtable pointer) whose vtable
/// holds recording stubs, as [`Twins::object`].
fn vobj(t: &mut Twins, words: &[u32], rets: &[u32], size: usize) -> u32 {
    let mut vt = Vec::new();
    for (i, &n) in words.iter().enumerate() {
        let r = rets.get(i).copied().unwrap_or(0x1000 + i as u32);
        let s = t.stub_code(0x7000 + i as u32, true, n, n, r);
        vt.extend(s.to_le_bytes());
    }
    let vt = t.alloc(&vt);
    let obj = t.zeros(size);
    t.w32(obj, vt);
    obj
}

/// A member's type: slots 1 (0 words), 4 (`count`), 7 (`elements`), 8
/// (3 words); `kind` at `+0x30`, `base` at `+0x88`.
fn member_type(t: &mut Twins, kind: u32, count: u32, elements: u32, base: u32) -> u32 {
    let ty = vobj(
        t,
        &[0, 0, 0, 0, 1, 0, 0, 1, 3],
        &[0, 0x1001, 0, 0, count, 0, 0, elements, 0x1008],
        0xa0,
    );
    t.w32(ty + 0x30, kind);
    t.w32(ty + 0x88, base);
    ty
}

/// A member: `vt[1]` returns `ty`; flags `+0x14`, default holder `+0x18`
/// (whose `vt[1]` returns `has`), offset `+0x24`, descriptor `+0xc` with
/// base `+0x88`.
fn member(t: &mut Twins, ty: u32, flags: u32, has: u32, off: u32) -> u32 {
    let m = vobj(t, &[0, 0], &[0, ty], 0x40);
    let holder = vobj(t, &[0, 0], &[0, has], 0x10);
    let desc = t.zeros(0x90);
    t.w32(desc + 0x88, 0x10);
    t.w32(m + 0xc, desc);
    t.w32(m + 0x14, flags);
    t.w32(m + 0x18, holder);
    t.w32(m + 0x24, off);
    m
}

/// Links `members` into `desc`'s list (`+0x44` next, `+0x48` prev; a
/// member's node at `+4`), circular through the head.
fn link_members(t: &mut Twins, desc: u32, members: &[u32]) {
    let head = desc + 0x44;
    let nodes: Vec<u32> = members.iter().map(|m| m + 4).collect();
    let mut all = vec![head];
    all.extend(&nodes);
    for i in 0..all.len() {
        let next = all[(i + 1) % all.len()];
        let prev = all[(i + all.len() - 1) % all.len()];
        t.w32(all[i], next);
        t.w32(all[i] + 4, prev);
    }
}

#[test]
fn left2_swap_visitors_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(lf::DYNAMIC_TYPE, true, 1, 1, 0xd7d7);
        t.stub(lf::FOR_EACH_ELEMENT, true, 3, 3, 1);
        // the per-member step: kinds, flags, counts, no type
        for kind in 0..=4u32 {
            for typed in [true, false] {
                for flags in [0u32, 2, 0x40, 0x42, 0xffff_ffbd] {
                    for count in [1u32, 3] {
                        for flag in [0u8, 1, 0xff] {
                            if !typed && (kind != 0 || count != 1) {
                                continue;
                            }
                            let ty = if typed {
                                member_type(&mut t, kind, count, 0x00e1_e000, 0)
                            } else {
                                0
                            };
                            let m = member(&mut t, ty, flags, 0, 0x30);
                            let v = t.junk(0x10, 7);
                            t.w32(v + 8, 0x0050_0000);
                            t.w8(v + 0xc, flag);
                            let what = format!(
                                "{label}: swap member kind {kind} typed {typed} flags {flags:#x} count {count} flag {flag}"
                            );
                            t.check(&what, lf::SWAP_MEMBER, Conv::Thiscall(v), &[m], |g| {
                                lb::swap_member_at(g, v, m);
                                Out::None
                            });
                        }
                    }
                }
            }
        }
        t.reset();
        // the walkers: a chain with members, an empty parent, a 0 link
        t.stub(lf::SWAP_MEMBER, true, 1, 1, 0x5a5a);
        let d3 = t.zeros(0x100);
        let m3 = t.zeros(0x30);
        link_members(&mut t, d3, &[m3]);
        let d2 = t.zeros(0x100);
        link_members(&mut t, d2, &[]);
        t.w32(d2 + 0x40, d3);
        let d1 = t.zeros(0x100);
        let ms: Vec<u32> = (0..3).map(|_| t.zeros(0x30)).collect();
        link_members(&mut t, d1, &ms);
        t.w32(d1 + 0x40, d2);
        let empty = t.zeros(0x100);
        link_members(&mut t, empty, &[]);
        let cut = t.zeros(0x100);
        let mc: Vec<u32> = (0..3).map(|_| t.zeros(0x30)).collect();
        link_members(&mut t, cut, &mc);
        t.w32(mc[1] + 4, 0);
        t.w32(mc[1] + 8, 0);
        let nolist = t.zeros(0x100);
        let visitor = t.junk(0x10, 9);
        for (desc, visits) in [
            (d1, true),
            (d2, true),
            (cut, true),
            (empty, false),
            (nolist, false),
            (0, false),
        ] {
            for (i, f) in lf::SWAP_WALKERS.into_iter().enumerate() {
                let what = format!("{label}: swap walker {f:#x} desc {desc:#x}");
                t.check(&what, f, Conv::Thiscall(desc), &[visitor], |g| {
                    let eax = lb::swap_walk(g, desc, visitor, i == 1, 0);
                    if visits { Out::U32(eax) } else { Out::None }
                });
            }
        }
        t.reset();
        // swapEndiannessOfObject: the visitor in the frame (logged by the
        // member stub), either walker; without members the eax is the
        // visitor's address
        t.stub(lf::SWAP_MEMBER, true, 1, 1, 0x5a5a);
        t.w32(d1 + 0x1c, 0x1234_5678);
        for d in [empty, d1, cut, nolist] {
            for object in [0u32, 0x0060_0000] {
                for flag in [0u32, 1, 0xff, 0x100, 0xffff_ff00] {
                    let what =
                        format!("{label}: swapEndiannessOfObject {d:#x} {object:#x} {flag:#x}");
                    t.check_frame(
                        &what,
                        0x0043_d6a0,
                        Conv::Thiscall(d),
                        &[object, flag],
                        0x20,
                        13,
                        |g, fr| Out::U32(lb::swap_object(g, d, object, flag, fr)),
                    );
                }
            }
        }
        t.reset();
    }
}

#[test]
fn left2_copy_visitors_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(lf::STORED_SIZE, true, 0, 0, 6);
        t.stub(lf::MEMCPY, false, 3, 0, 0x3c3c);
        // the per-member step
        for kind in 0..=4u32 {
            for typed in [true, false] {
                for flags in [0u32, 2, 0xffff_ffff] {
                    for has in [0u32, 1] {
                        for to in [0u32, 0x0070_0000] {
                            if !typed && kind != 0 {
                                continue;
                            }
                            let ty = if typed {
                                member_type(&mut t, kind, 5, 0x00e2_e000, 0)
                            } else {
                                0
                            };
                            let m = member(&mut t, ty, flags, has, 0x48);
                            let v = t.junk(0x14, 3);
                            t.w32(v + 8, 0x0080_0000);
                            t.w32(v + 0xc, to);
                            let what = format!(
                                "{label}: copy member kind {kind} typed {typed} flags {flags:#x} has {has} to {to:#x}"
                            );
                            t.check(&what, lf::COPY_MEMBER, Conv::Thiscall(v), &[m], |g| {
                                lb::copy_member_at(g, v, m);
                                Out::None
                            });
                        }
                    }
                }
            }
        }
        // the walker: members of each kind, nested classes walked
        // `count` times, a pointer member's nested target 0
        let inner_ty = member_type(&mut t, 0, 1, 0, 0);
        let inner = t.zeros(0x100);
        t.w32(inner + 0x88, 0x20);
        let im: Vec<u32> = (0..2)
            .map(|i| member(&mut t, inner_ty, 2 + 0x40 * i, 1, 8 + 4 * i))
            .collect();
        link_members(&mut t, inner, &im);
        for with_class in [true, false] {
            let dynamic = vobj(
                &mut t,
                &[0, 0],
                &[0, if with_class { inner } else { 0 }],
                0x20,
            );
            t.stub(lf::DYNAMIC_TYPE, true, 1, 1, dynamic);
            let parent = t.zeros(0x100);
            let pm = member(&mut t, inner_ty, 0, 0, 0x10);
            link_members(&mut t, parent, &[pm]);
            let desc = t.zeros(0x100);
            t.w32(desc + 0x40, parent);
            let mut ms = Vec::new();
            for (kind, flags, count) in [
                (0u32, 0u32, 1u32),
                (1, 0, 3),
                (1, 0, 0),
                (2, 0, 1),
                (2, 0x40, 1),
                (2, 0x40, 2),
                (3, 0, 1),
                (4, 0x40, 1),
                (0, 2, 1),
            ] {
                let ty = member_type(&mut t, kind, count, 0x00e3_e000, 0);
                ms.push(member(&mut t, ty, flags, 0, 0x20 + 4 * kind));
            }
            link_members(&mut t, desc, &ms);
            for (from, to, pointer) in [
                (0x0090_0000u32, 0x00a0_0000u32, 0u8),
                (0x0090_0000, 0, 1),
                (0x0090_0000, 0x00a0_0000, 1),
            ] {
                let v = t.zeros(0x14);
                t.w32(v + 8, from);
                t.w32(v + 0xc, to);
                t.w8(v + 0x10, pointer);
                let what = format!(
                    "{label}: copy walker class {with_class} from {from:#x} to {to:#x} pointer {pointer}"
                );
                t.check(&what, lf::COPY_WALKER, Conv::Thiscall(desc), &[v], |g| {
                    Out::U32(lb::copy_walk_at(g, desc, v))
                });
            }
            let v = t.zeros(0x14);
            let what = format!("{label}: copy walker no descriptor");
            t.check(&what, lf::COPY_WALKER, Conv::Thiscall(0), &[v], |g| {
                Out::U32(lb::copy_walk_at(g, 0, v))
            });
            for (a, b) in [(0x00b0_0000u32, 0x00c0_0000u32), (0, 0x00c0_0000)] {
                for d in [desc, 0] {
                    let what =
                        format!("{label}: copy object {d:#x} {a:#x} {b:#x} class {with_class}");
                    t.check(&what, 0x0043_ca50, Conv::Thiscall(d), &[a, b], |g| {
                        Out::U32(lb::copy_object(g, d, a, b))
                    });
                }
            }
        }
        t.reset();
    }
}

#[test]
fn left2_lookups_match_the_original() {
    for (label, mut t) in Twins::all() {
        let strs: [&[u8]; 5] = [b"alpha\0", b"beta\0", b"al\0", b"\0", b"alphabet\0"];
        let names: Vec<u32> = strs.iter().map(|s| t.alloc(s)).collect();
        // the named-entry lists of a descriptor and its parent
        for &(f, off) in &lf::NAMED_LISTS {
            let parent = t.zeros(0x100);
            let desc = t.zeros(0x100);
            t.w32(desc + 0x40, parent);
            for (d, ns) in [(desc, [names[0], 0]), (parent, [names[2], names[1]])] {
                let head = d + off;
                let e1 = t.zeros(0x10);
                let e2 = t.zeros(0x10);
                t.w32(e1 + 0xc, ns[0]);
                t.w32(e2 + 0xc, ns[1]);
                t.w32(head, e1);
                t.w32(e1, e2);
                t.w32(e2, head);
            }
            let other = t.zeros(0x100);
            t.w32(other + off, other + off);
            for d in [desc, parent, other, 0] {
                for name in [names[0], names[1], names[2], names[3], names[4], 0] {
                    let what = format!("{label}: named list {f:#x} desc {d:#x} name {name:#x}");
                    t.check(&what, f, Conv::Thiscall(d), &[name], |g| {
                        Out::U32(lb::find_named(g, off, d, name))
                    });
                }
            }
        }
        // the name/value table
        let pairs = t.zeros(0x20);
        for (i, n) in [names[2], names[0], names[1]].into_iter().enumerate() {
            t.w32(pairs + 8 * i as u32, n);
            t.w32(pairs + 8 * i as u32 + 4, 0x100 + i as u32);
        }
        for count in [0u32, 1, 3] {
            let table = t.zeros(0x10);
            t.w32(table + 8, pairs);
            t.w32(table + 0xc, count);
            for name in [names[0], names[1], names[4], 0] {
                let out = t.junk(4, 5);
                let what = format!("{label}: name/value count {count} name {name:#x}");
                t.check(
                    &what,
                    0x0057_09f0,
                    Conv::Thiscall(table),
                    &[name, out],
                    |g| Out::Al(lb::find_pair(g, table, name, out)),
                );
            }
        }
        // the namespace copy: own list, a child's, a link of 0
        let child = t.zeros(0x40);
        let ns = t.zeros(0x40);
        let d1 = t.zeros(0x40);
        let d2 = t.zeros(0x40);
        let d3 = t.zeros(0x40);
        t.w32(d1 + 0x18, names[0]);
        t.w32(d2 + 0x18, 0);
        t.w32(d3 + 0x18, names[1]);
        t.w32(ns + 8, d1 + 0x2c);
        t.w32(d1 + 0x2c, d2 + 0x2c);
        t.w32(d2 + 0x2c, ns + 8);
        t.w32(child + 8, d3 + 0x2c);
        t.w32(d3 + 0x2c, 0);
        t.w32(child + 0x14, child + 0x14);
        t.w32(ns + 0x14, child);
        t.w32(child, ns + 0x14);
        let odd = t.zeros(0x40);
        t.w32(odd + 8, 0x2c);
        t.w32(odd + 0x14, odd + 0x14);
        for name in [names[0], names[1], names[2], 0] {
            for n in [ns, child, odd] {
                let what = format!("{label}: namespace copy {n:#x} name {name:#x}");
                t.check(&what, 0x0043_ea30, Conv::Thiscall(n), &[name], |g| {
                    Out::U32(lb::namespace_find(g, n, name))
                });
            }
        }
    }
}

/// A code stub (cdecl) that returns `[cell]` and adds `step` to it.
fn counter_stub(t: &mut Twins, sym: u32, cell: u32, step: u8) {
    let at = t.rebase(sym);
    let mut c = vec![0xa1];
    c.extend(cell.to_le_bytes()); // mov eax, [cell]
    c.extend([0x83, 0x05]);
    c.extend(cell.to_le_bytes());
    c.push(step); // add dword [cell], step
    c.push(0xc3);
    let code = t.a.alloc_code(&c).unwrap();
    assert_eq!(t.b.alloc_code(&c).unwrap(), code);
    let mut j = vec![0xe9];
    j.extend(code.wrapping_sub(at + 5).to_le_bytes());
    let old = Oracle::read(&mut t.a, at, 5).unwrap();
    t.saved.entry(at).or_insert(old);
    t.write(at, &j);
}

#[test]
fn left2_small_helpers_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the PhyreObject push: the stack grows or not, a userdata or none
        for step in [0u8, 1] {
            for ud in [true, false] {
                let cell = t.zeros(4);
                t.w32(cell, 5);
                counter_stub(&mut t, lf::LUA[0], cell, step);
                let block = if ud { t.junk(8, 2) } else { 0 };
                t.stub(lf::LUA[1], false, 2, 0, 1);
                t.stub(lf::LUA[2], false, 2, 0, block);
                t.stub(lf::LUA[3], false, 3, 0, 0);
                t.stub(lf::LUA[4], false, 2, 0, 0);
                t.stub(lf::LUA[5], false, 2, 0, 0);
                for object in [0u32, 0x0012_0000] {
                    let what =
                        format!("{label}: push PhyreObject step {step} ud {ud} object {object:#x}");
                    t.check(
                        &what,
                        0x0043_5940,
                        Conv::Cdecl,
                        &[0x00d0_0000, 3, object],
                        |g| Out::U32(lb::push_scripted(g, 0x00d0_0000, 3, object)),
                    );
                }
                t.reset();
            }
        }
        // the static list object: built, then cached
        t.stub(lf::ATEXIT, false, 1, 0, 0);
        let guard = t.rebase(lf::STATIC_LIST[0]);
        for g0 in [0u32, 2, 1, 3] {
            t.w32(guard, g0);
            let what = format!("{label}: static list guard {g0:#x}");
            t.check(&what, 0x0043_f090, Conv::Cdecl, &[], |g| {
                Out::U32(lb::static_list(g))
            });
        }
        t.reset();
        // the node release and the cluster's deleting destructor
        t.stub(lf::NODE_RELEASE[0], true, 0, 0, 0x11);
        t.stub(lf::NODE_RELEASE[1], true, 0, 0, 0x12);
        t.stub(lf::NODE_RELEASE[2], true, 1, 1, 0x13);
        let pool = t.zeros(0x20);
        let node = t.zeros(0x40);
        t.check(
            &format!("{label}: node release"),
            0x0043_f3c0,
            Conv::Thiscall(pool),
            &[node],
            |g| Out::U32(lb::release_node(g, pool, node)),
        );
        for c in lf::CLUSTER_DTOR {
            t.stub(c, true, 1, 1, 0x21);
        }
        t.stub(lf::CLUSTER_DTOR[0], true, 0, 0, 0x20);
        t.stub(lf::CLUSTER_DTOR[2], true, 0, 0, 0x22);
        t.stub(lf::FREE, false, 1, 0, 0x23);
        for world in [0u32, 0x0034_0000] {
            for flags in [0u32, 1, 0xffff_fffe] {
                let this = t.junk(0x60, 4);
                t.w32(this + 0x40, world);
                let what = format!("{label}: cluster delete world {world:#x} flags {flags:#x}");
                t.check(&what, 0x0043_fed0, Conv::Thiscall(this), &[flags], |g| {
                    Out::U32(lb::delete_cluster(g, this, flags))
                });
            }
        }
        t.reset();
        // createInternal: abstract, no list, a list
        t.stub(lf::PRINTF, false, 3, 0, 0);
        t.stub(lf::CREATE_INTERNAL[1], true, 1, 1, 0x31);
        let list = t.zeros(0x40);
        t.w32(list + 0x30, 0x0045_6780);
        for (flags, got) in [
            (8u32, list),
            (0xffff_ffff, list),
            (0, 0),
            (0xffff_fff7, list),
        ] {
            t.stub(lf::CREATE_INTERNAL[0], true, 4, 4, got);
            let desc = t.zeros(0xa0);
            t.w32(desc + 0x90, flags);
            t.w32(desc + 0x18, 0x00b0_f248);
            t.w32(desc + 0x1c, 0x44);
            let out = t.junk(8, 6);
            let cluster = t.zeros(0x10);
            let what = format!("{label}: createInternal flags {flags:#x} list {got:#x}");
            t.check(
                &what,
                0x0044_1120,
                Conv::Thiscall(cluster),
                &[out, desc],
                |g| Out::U32(lb::create_internal(g, cluster, out, desc)),
            );
        }
        t.reset();
        // deleteInstancesOfType: matches at the head, in the middle, none
        t.stub(lf::NODE_RELEASE[0], true, 0, 0, 0x11);
        t.stub(lf::NODE_RELEASE[1], true, 0, 0, 0x12);
        t.stub(lf::NODE_RELEASE[2], true, 1, 1, 0x13);
        t.stub(lf::REMOVE_INSTANCE_LIST, true, 1, 1, 0x14);
        for world in [0u32, 0x0034_0000] {
            for pattern in [[1u32, 2, 1, 1], [2, 2, 2, 2], [1, 1, 1, 1], [2, 1, 2, 1]] {
                let cluster = t.zeros(0x60);
                t.w32(cluster + 0x40, world);
                let head = cluster + 0x1c;
                let lists: Vec<u32> = pattern
                    .iter()
                    .map(|&d| {
                        let l = t.zeros(0x40);
                        t.w32(l + 0x2c, d);
                        l
                    })
                    .collect();
                let mut all = vec![head];
                all.extend(&lists);
                for i in 0..all.len() {
                    t.w32(all[i], all[(i + 1) % all.len()]);
                    t.w32(all[i] + 4, all[(i + all.len() - 1) % all.len()]);
                }
                let what = format!("{label}: deleteInstancesOfType world {world:#x} {pattern:?}");
                t.check(&what, 0x0044_1180, Conv::Thiscall(cluster), &[1], |g| {
                    lb::delete_instances_of_type(g, cluster, 1);
                    Out::None
                });
            }
            let cluster = t.zeros(0x60);
            t.w32(cluster + 0x1c, cluster + 0x1c);
            t.check(
                &format!("{label}: deleteInstancesOfType empty"),
                0x0044_1180,
                Conv::Thiscall(cluster),
                &[1],
                |g| {
                    lb::delete_instances_of_type(g, cluster, 1);
                    Out::None
                },
            );
        }
        t.reset();
        // PArray<PUInt8>::operator=
        let src_bytes = t.alloc(b"0123456789abcdef");
        for n in [0u32, 1, 16, 0x8000_0005] {
            for block in [true, false] {
                for (head, old) in [(3u32, 0x0055_0000u32), (0x8000_0003, 0x0055_0000), (3, 0)] {
                    let b = if block { t.junk(0x20, 8) } else { 0 };
                    t.stub(lf::MALLOC_ALIGNED, false, 2, 0, b);
                    t.stub(lf::FREE, false, 1, 0, 0x23);
                    let src = t.zeros(8);
                    t.w32(src, n);
                    t.w32(src + 4, src_bytes);
                    let this = t.zeros(8);
                    t.w32(this, head);
                    t.w32(this + 4, old);
                    let what = format!(
                        "{label}: PArray<u8> assign n {n:#x} block {block} head {head:#x} old {old:#x}"
                    );
                    t.check(&what, 0x0044_79f0, Conv::Thiscall(this), &[src], |g| {
                        Out::U32(lb::assign_bytes(g, this, src))
                    });
                }
            }
        }
        // same block: old == new
        let b = t.junk(0x20, 8);
        t.stub(lf::MALLOC_ALIGNED, false, 2, 0, b);
        let src = t.zeros(8);
        t.w32(src, 4);
        t.w32(src + 4, src_bytes);
        let this = t.zeros(8);
        t.w32(this, 4);
        t.w32(this + 4, b);
        t.check(
            &format!("{label}: PArray<u8> assign same block"),
            0x0044_79f0,
            Conv::Thiscall(this),
            &[src],
            |g| Out::U32(lb::assign_bytes(g, this, src)),
        );
        t.reset();
    }
}

#[test]
fn left2_swap_static_copies_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(lf::ATEXIT, false, 1, 0, 0);
        t.stub(0x0043_d6a0, true, 2, 2, 0x5151);
        for &(f, l) in &lf::SWAP_STATIC {
            t.stub(l.ctor, true, 0, 0, 0x99);
            let guard = t.rebase(l.guard);
            for g0 in [0u32, 1, 2, 0xffff_fffe] {
                t.w32(guard, g0);
                let flags = t.rebase(l.flags);
                t.w32(flags, 0xffff_ffff);
                for p in [0u32, 0x0042_4240] {
                    let what =
                        format!("{label}: swapEndiannessStatic {f:#x} guard {g0:#x} p {p:#x}");
                    t.check(&what, f, Conv::Cdecl, &[p], |g| {
                        Out::U32(ffx_pload::m5p_tmpl_fortype::bridge::swap_lazy(g, &l, p))
                    });
                }
            }
        }
        t.reset();
    }
}

#[test]
fn left2_dynamic_descriptor_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // the write-mask merge: no source, no buffer, no mask; the back
        // pointer when PClassDescriptorDynamic's descriptor is in the chain
        let dynamic_desc = t.rebase(lf::MASK_GETTER[1]);
        let src = t.junk(0x20, 0x31);
        for (has_src, has_buf, has_mask, chain) in [
            (false, true, true, 0u8),
            (true, false, true, 0),
            (true, true, false, 1),
            (true, true, true, 0),
            (true, true, true, 1),
            (true, true, true, 2),
        ] {
            let mask = if has_mask { t.junk(0x20, 0x5c) } else { 0 };
            t.stub(lf::MASK_GETTER[0], true, 0, 0, mask);
            let this = t.zeros(0xc0);
            let buf = if has_buf { t.junk(0x20, 0x77) } else { 0 };
            let table = t.zeros(0x40);
            t.w32(this + 0x64, buf);
            t.w32(this + 0x1c, 0x13);
            t.w32(this + 0x8c, table.wrapping_sub(buf));
            match chain {
                1 => t.w32(this + 0x40, dynamic_desc),
                2 => {
                    let p = t.zeros(0x50);
                    t.w32(this + 0x40, p);
                    t.w32(p + 0x40, dynamic_desc);
                }
                _ => {}
            }
            let s = if has_src { src } else { 0 };
            let what = format!(
                "{label}: merge buffer src {has_src} buf {has_buf} mask {has_mask} chain {chain}"
            );
            t.check(&what, 0x0044_94a0, Conv::Thiscall(this), &[s], |g| {
                lb::merge_buffer(g, this, s);
                Out::None
            });
        }
        t.reset();
        // the script read: no parent, out or none, an object or none
        t.stub(lf::MEMCPY, false, 3, 0, 0x3c3c);
        for got in [0u32, 0x0066_0000] {
            t.stub(lf::OBJECT_GET, false, 1, 0, got);
            for (parent, out) in [(false, 0x0077_0000u32), (true, 0x0077_0000), (true, 0)] {
                let this = t.zeros(0xa0);
                if parent {
                    let p = vobj(&mut t, &[0, 0, 0, 2], &[], 0x40);
                    t.w32(p + 0x1c, 0x18);
                    t.w32(this + 0x40, p);
                }
                t.w32(this + 0x84, 0x10);
                t.w32(this + 0x88, 0x08);
                t.w32(this + 0x98, 0x30);
                let what =
                    format!("{label}: script read parent {parent} out {out:#x} obj {got:#x}");
                t.check(
                    &what,
                    0x0044_98f0,
                    Conv::Thiscall(this),
                    &[0x00d0_0000, out],
                    |g| {
                        lb::read_script_value(g, this, 0x00d0_0000, out);
                        Out::None
                    },
                );
            }
        }
        t.reset();
        // addMember: default instance, external, kinds; addMember's answer
        for (default, flags, kind, typed, ok, parent) in [
            (1u32, 0u32, 0u32, true, 0u32, false),
            (0, 2, 0, true, 0, false),
            (0, 0, 0, true, 0, true),
            (0, 0, 2, true, 0, false),
            (0, 0x40, 2, true, 0, true),
            (0, 0, 1, false, 0, false),
            (0, 0, 0, true, 5, false),
            (0, 0x42, 3, true, 0x12, true),
        ] {
            t.stub(lf::ADD_DYNAMIC[0], false, 1, 0, 0x100 | default);
            t.stub(lf::ADD_DYNAMIC[1], true, 2, 2, ok);
            t.stub(lf::ADD_DYNAMIC[2], true, 0, 0, 0x2e);
            let this = t.zeros(0xc0);
            t.w32(this + 0x98, 0x13);
            t.w32(this + 0x20, 4);
            t.w32(this + 0x94, 0xffff_0003);
            if parent {
                let p = t.zeros(0x40);
                t.w32(p + 0x1c, 0x0c);
                t.w32(this + 0x40, p);
            }
            let holder = vobj(&mut t, &[0, 0], &[], 0x30);
            t.w32(holder + 0x1c, 0x0a);
            t.w32(holder + 0x20, 8);
            let ty = if typed {
                member_type(&mut t, kind, 1, 0, 0)
            } else {
                0
            };
            let m = member(&mut t, ty, flags, 0, 0);
            t.w32(m + 0x18, holder);
            let what = format!(
                "{label}: dynamic addMember default {default} flags {flags:#x} kind {kind} ok {ok:#x}"
            );
            t.check(&what, 0x0044_9b90, Conv::Thiscall(this), &[m], |g| {
                Out::U32(lb::add_member(g, this, m))
            });
            t.reset();
        }
        // reset: counts equal and not 0, or not; a parent or none
        t.stub(lf::UPDATE_BASE_OFFSETS, true, 0, 0, 0x6e);
        for (a, b, parent) in [
            (3u16, 3u16, true),
            (0, 0, true),
            (2, 3, false),
            (2, 3, true),
        ] {
            let this = t.junk(0xc0, 0x21);
            t.write(this + 0x94, &a.to_le_bytes());
            t.write(this + 0x96, &b.to_le_bytes());
            t.w32(this + 0x40, 0);
            if parent {
                let p = t.junk(0x40, 0x22);
                t.w32(this + 0x40, p);
            }
            let what = format!("{label}: dynamic reset {a} {b} parent {parent}");
            t.check(&what, 0x0044_9c80, Conv::Thiscall(this), &[], |g| {
                Out::U32(lb::reset(g, this))
            });
        }
        t.reset();
        // removeMember: members placed again, external and typed
        t.stub(lf::UNLINK, true, 1, 1, 0x1d);
        for parent in [false, true] {
            let this = t.zeros(0xc0);
            t.w32(this + 0x94, 0x0001_0001);
            t.w32(this + 0x20, 2);
            if parent {
                let p = t.zeros(0x40);
                t.w32(p + 0x1c, 0x0e);
                t.w32(this + 0x40, p);
            }
            let mut ms = Vec::new();
            for (flags, size, align) in [(0u32, 3u32, 1u32), (2, 0, 0), (0, 0x10, 8), (0x40, 1, 2)]
            {
                let holder = vobj(&mut t, &[0, 0], &[], 0x30);
                t.w32(holder + 0x1c, size);
                t.w32(holder + 0x20, align);
                let m = member(&mut t, 0, flags, 0, 0x55);
                t.w32(m + 0x18, holder);
                ms.push(m);
            }
            link_members(&mut t, this, &ms);
            let what = format!("{label}: dynamic removeMember parent {parent}");
            t.check(
                &what,
                0x0044_9d50,
                Conv::Thiscall(this),
                &[0x0012_3400],
                |g| {
                    lb::remove_member(g, this, 0x0012_3400);
                    Out::None
                },
            );
        }
        t.reset();
        // the new member's place and addMember(name, ...)
        for (flags, sizer, class, parent) in [
            (2u32, false, false, false),
            (2, true, false, true),
            (0, false, true, true),
            (0, true, true, true),
            (0, true, false, false),
            (0xffff_fffd, true, true, false),
        ] {
            let this = t.zeros(0xc0);
            t.w32(this + 0x98, 0x0b);
            t.w32(this + 0x20, 4);
            if parent {
                let p = t.zeros(0x40);
                t.w32(p + 0x1c, 0x06);
                t.w32(this + 0x40, p);
            }
            let c = if class { t.zeros(0xa0) } else { 0 };
            let ty = vobj(&mut t, &[0, 0], &[0, c], 0x30);
            t.w32(ty + 0x1c, 0x0a);
            t.w32(ty + 0x20, 8);
            let s = if sizer {
                vobj(&mut t, &[0, 0, 1, 1], &[0, 16, 0x24, 4], 0x10)
            } else {
                0
            };
            let what = format!(
                "{label}: dynamic place flags {flags:#x} sizer {sizer} class {class} parent {parent}"
            );
            t.check(
                &what,
                0x0044_9960,
                Conv::Thiscall(this),
                &[ty, flags, s],
                |g| Out::U32(lb::place_new(g, this, ty, flags, s)),
            );
        }
        let name = t.alloc(b"member\0");
        for (n, exists, class, derives, cflags, flags, block) in [
            (0u32, 0u32, false, 0u32, 0u32, 0u32, true),
            (name, 1, false, 0, 0, 0, true),
            (name, 0, true, 0x101, 0, 0, true),
            (name, 0, true, 0x100, 2, 0, true),
            (name, 0, true, 0x100, 0, 0, true),
            (name, 0, true, 0x101, 2, 2, true),
            (name, 0, true, 0, 0, 0, false),
            (name, 0, false, 0, 0, 0x41, true),
        ] {
            t.stub(lf::ADD_MEMBER_NAMED[0], true, 1, 1, exists);
            t.stub(lf::ADD_MEMBER_NAMED[1], true, 1, 1, derives);
            t.stub(lf::ADD_MEMBER_NAMED[2], true, 3, 3, 0x2c);
            let p = if block { t.zeros(0x30) } else { 0 };
            t.stub(lf::MALLOC, false, 1, 0, p);
            t.stub(lf::ADD_MEMBER_NAMED[3], true, 6, 6, 0xc7);
            let c = if class { t.zeros(0xa0) } else { 0 };
            if class {
                t.w32(c + 0x90, cflags);
            }
            let ty = vobj(&mut t, &[0, 0], &[0, c], 0x30);
            let this = t.zeros(0xc0);
            let what = format!(
                "{label}: dynamic addMember(name) name {n:#x} exists {exists} class {class} derives {derives:#x} cflags {cflags} flags {flags:#x} block {block}"
            );
            t.check(
                &what,
                0x0044_9250,
                Conv::Thiscall(this),
                &[n, ty, flags, 0x5150],
                |g| Out::U32(lb::add_named_member(g, this, n, ty, flags, 0x5150)),
            );
            t.reset();
        }
    }
}

/// A code stub (thiscall, no words) that writes `words` (offset, value)
/// into the object in `ecx`.
fn writer_stub(t: &mut Twins, sym: u32, words: &[(u8, u32)]) {
    let at = t.rebase(sym);
    let mut c = Vec::new();
    for &(off, v) in words {
        c.extend([0xc7, 0x41, off]);
        c.extend(v.to_le_bytes()); // mov dword [ecx+off], v
    }
    c.push(0xc3);
    let code = t.a.alloc_code(&c).unwrap();
    assert_eq!(t.b.alloc_code(&c).unwrap(), code);
    let mut j = vec![0xe9];
    j.extend(code.wrapping_sub(at + 5).to_le_bytes());
    let old = Oracle::read(&mut t.a, at, 5).unwrap();
    t.saved.entry(at).or_insert(old);
    t.write(at, &j);
}

#[test]
fn left2_frame_users_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the registry lookup: its key in the frame
        t.stub(lf::REGISTRY_FIND[0], true, 0, 0, 0x10);
        t.stub(lf::REGISTRY_FIND[1], true, 0, 0, 0x11);
        t.stub(lf::REGISTRY_FIND[2], false, 1, 0, 0x0061_0000);
        t.stub(lf::REGISTRY_FIND[3], false, 2, 0, 0xa5a5_1234);
        t.stub(lf::REGISTRY_FIND[4], true, 1, 1, 0xf00d);
        for name in [0u32, 0x0062_0000] {
            let what = format!("{label}: registry find {name:#x}");
            t.check_frame(
                &what,
                0x0045_d280,
                Conv::Cdecl,
                &[name],
                0x14,
                8,
                |g, fr| Out::U32(lb::registry_find(g, name, fr)),
            );
        }
        t.reset();
        // the instance push: no class, no instance, lists with a cursor
        // whose block ends inside the count
        let blocks = t.zeros(0x40);
        let end2 = t.zeros(8);
        t.w32(blocks + 8, end2);
        // `left`: what the cursor's seek leaves (0: an empty list, the
        // cursor as the original filled it)
        for (has_class, count, lists, left) in [
            (false, 1u32, 1usize, 1u32),
            (true, 0, 1, 1),
            (true, 2, 0, 1),
            (true, 2, 1, 2),
            (true, 2, 1, 0),
            (true, 5, 2, 5),
        ] {
            t.stub(lf::PUSH_INSTANCES[0], false, 0, 0, 0x0063_0000);
            t.stub(
                lf::PUSH_INSTANCES[1],
                true,
                1,
                1,
                if has_class { 0x0064_0000 } else { 0 },
            );
            t.stub(lf::PUSH_INSTANCES[2], true, 1, 1, count);
            t.stub(lf::PUSH_INSTANCES[5], false, 3, 0, 0x77);
            let node = t.zeros(0x40);
            t.w32(node + 0x2c, 0x0065_0000);
            if left == 0 {
                writer_stub(&mut t, lf::PUSH_INSTANCES[3], &[(0xc, 0)]);
            } else {
                writer_stub(
                    &mut t,
                    lf::PUSH_INSTANCES[3],
                    &[
                        (0, blocks),
                        (4, blocks + 8),
                        (8, 4),
                        (0xc, left),
                        (0x10, 0x100),
                        (0x18, node),
                    ],
                );
            }
            writer_stub(&mut t, lf::PUSH_INSTANCES[4], &[(0xc, 0)]);
            let world = t.zeros(0x10);
            let mut all = vec![world];
            for _ in 0..lists {
                let l = t.zeros(0x80);
                t.w32(l + 0x1c, l + 0x1c);
                all.push(l + 0x48);
            }
            for i in 0..all.len() {
                t.w32(all[i], all[(i + 1) % all.len()]);
            }
            let what = format!(
                "{label}: push instances class {has_class} count {count} lists {lists} left {left}"
            );
            t.check_frame(
                &what,
                0x0044_2370,
                Conv::Cdecl,
                &[world, 0x00d0_0000, 0x0066_0000],
                0x40,
                // the cursor is only written for a class with instances
                if has_class && count != 0 && lists != 0 {
                    0x20
                } else {
                    0
                },
                |g, fr| Out::U32(lb::push_instances(g, world, 0x00d0_0000, 0x0066_0000, fr)),
            );
            t.reset();
        }
    }
}

/// `0x004c1480` (the utility registrations taken out) against the
/// original: the memory as it is (the snapshots' live list), a list built
/// in a shuffled order with a foreign node and some registrations
/// missing, and the unlink answering nonzero (the original stops there).
#[test]
fn big_utility_teardown_matches_the_original() {
    use ffx_pload::m5p_registry_load_a_left::{self as l2, bridge as l2b};
    let [head, slots, first, stride, count, unlink, _] = l2::UTILITY_TEARDOWN;
    for case in 0..3u32 {
        for (label, mut t) in Twins::all() {
            if case >= 1 {
                // the slot table and each registration's index
                let slots_at = t.rebase(slots);
                t.write(slots_at, &[0x5a; 0x40]);
                for k in 0..count {
                    let e = t.rebase(first + stride * k);
                    t.w32(e + 8, k % 16);
                    t.w32(e + 4, 0);
                }
                // the list: shuffled, a foreign node in the middle, every
                // fourth registration left out
                let foreign = t.zeros(8);
                // the inline three early, between others
                let mut order: Vec<u32> = vec![4, 0, 5, 1, 6, 2];
                order.extend((8..count).filter(|k| k % 4 != 3).rev());
                let mut nodes: Vec<u32> = order
                    .iter()
                    .map(|&k| t.rebase(first + stride * k))
                    .collect();
                nodes.insert(5, foreign);
                let head_at = t.rebase(head);
                t.w32(head_at, nodes[0]);
                for w in nodes.windows(2) {
                    t.w32(w[0] + 4, w[1]);
                }
            }
            if case == 2 {
                t.stub(unlink, false, 1, 0, 9);
            }
            let what = format!("{label}: utility teardown case {case}");
            t.check(&what, 0x004c_1480, Conv::Cdecl, &[], |g| {
                Out::U32(l2b::utility_teardown(g))
            });
            t.reset();
        }
    }
}

// ---- big-phyre3 ----

impl Twins {
    /// `code` at fresh code memory, jumped to from the original `sym`.
    fn place(&mut self, sym: u32, code: &[u8]) {
        let addr = self.a.alloc_code(code).unwrap();
        assert_eq!(self.b.alloc_code(code).unwrap(), addr);
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }

    /// The logging part of a stub: `tag`, `ecx` when `this`, `n` stack
    /// words.
    fn log_part(&self, tag: u32, this: bool, n: u32) -> Vec<u8> {
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
        c
    }

    /// A recording stub answering `answers` in turn (0 past the end).
    fn seq(&mut self, sym: u32, this: bool, n: u32, pops: u32, answers: &[u32]) {
        let mut table: Vec<u8> = answers.iter().flat_map(|v| v.to_le_bytes()).collect();
        table.extend([0u8; 256]);
        let table = self.alloc(&table);
        let cell = self.alloc(&table.to_le_bytes());
        let mut c = self.log_part(sym, this, n);
        c.push(0xa1);
        c.extend(cell.to_le_bytes()); // mov eax, [cell]
        c.extend([0x8b, 0x10]); // mov edx, [eax]
        c.extend([0x83, 0x05]);
        c.extend(cell.to_le_bytes());
        c.push(0x04); // add dword [cell], 4
        c.extend([0x89, 0xd0]); // mov eax, edx
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        self.place(sym, &c);
    }
}

/// `0x00466da0` (`PBinary::reloadClusterTextures`) with every callee
/// recorded; the cluster iterators' seek stubs hand out a block of
/// textures (one cursor hitting the block's end node, skipped), the next
/// stubs a second block once. Cases: no textures; the render interface
/// made or already; qualities 0, 1, 2 and past the levels; record
/// dimensions given or 0 (the texture's own); block and plain formats;
/// textures bound or not; read, creation and bind errors; the bad
/// format (7). Only synthetic data reaches it.
#[test]
#[allow(clippy::too_many_lines)] // the case data
fn bp3_reload_cluster_textures() {
    use ffx_pload::m5p_registry_load_a_bp3::{self as rl};
    let [malloc, ctor, atexit, quality_of, read_exact, free] = rl::RELOAD_CALLS;
    for (label, mut t) in Twins::all() {
        for k in 0..24u32 {
            let quality = [0u32, 1, 2, 7][k as usize % 4];
            let guard = t.rebase(rl::RENDER_INTERFACE[1]);
            let g: u32 = Mem::read(&t.a, guard);
            t.w32(guard, if k % 3 == 0 { g & !1 } else { g | 1 });
            let header = t.zeros(0x40);
            let list = header + 0x1c;
            t.w32(list, if k % 2 == 0 { list } else { 0x7e00 });
            let info = t.zeros(0x40);
            t.w32(info + 4, header);
            t.w32(info + 0x24, if k == 5 { 0 } else { 0x100 });
            let plain = t.zeros(0x20);
            t.w32(plain + 0xc, 32);
            let block = t.zeros(0x20);
            t.w32(block + 0xc, 4);
            t.w8(block + 0x10, 1);
            for (kind, (records, stride)) in [(0x10u32, 0x74u32), (0x18, 0x68), (0x20, 0x68)]
                .into_iter()
                .enumerate()
            {
                let kd = rl::KINDS[kind];
                // textures 0x40 apart: the first block of 3 (the second
                // cursor is the block's end node), a second block of 2
                let n = 5u32;
                let objs = t.zeros(0x40 * 8);
                let recs = t.zeros((stride * n) as usize);
                for i in 0..6u32 {
                    let tex = objs + 0x40 * i + 8;
                    let fmt = if (i + k) % 3 == 0 { block } else { plain };
                    let bad = k % 11 == 7 && i == 2 && kind == 1;
                    t.w32(tex, if bad { t.rebase(rl::BAD_FORMAT) } else { fmt });
                    for (j, v) in [13u32, 9, 3].into_iter().enumerate() {
                        t.w32(tex + 0x1c + 4 * j as u32, v + i);
                    }
                }
                for i in 0..n {
                    let rec = recs + stride * i;
                    t.w32(rec + 0xc, (i + k) % 4);
                    for j in 0..3u32 {
                        let v = if (i + j + k) % 4 == 0 { 0 } else { 16 >> j };
                        t.w32(rec + 0x1c + 4 * j, v);
                    }
                }
                t.w32(info + records, recs);
                let present = !(k % 5 == 3 && kind == 2);
                // seek: {cursor, node, stride, left, offset}
                let mut c = t.log_part(kd.seek, true, 0);
                for (off, v) in [
                    (0u8, objs),
                    (4, objs + 0x40),
                    (8, 0x40),
                    (0xc, if present { 3 } else { 0 }),
                    (0x10, 8),
                ] {
                    c.extend([0xc7, 0x41, off]);
                    c.extend(v.to_le_bytes()); // mov dword [ecx+off], v
                }
                c.push(0xc3);
                t.place(kd.seek, &c);
                // next: the second block once, then none
                let once = t.alloc(&2u32.to_le_bytes());
                let mut c = t.log_part(kd.next, true, 0);
                c.extend([0xc7, 0x01]);
                c.extend((objs + 0x40 * 3).to_le_bytes()); // mov dword [ecx], second block
                c.push(0xa1);
                c.extend(once.to_le_bytes()); // mov eax, [once]
                c.extend([0x89, 0x41, 0x0c]); // mov [ecx+0xc], eax
                c.extend([0xc7, 0x05]);
                c.extend(once.to_le_bytes());
                c.extend(0u32.to_le_bytes()); // mov dword [once], 0
                c.push(0xc3);
                t.place(kd.next, &c);
                let bound: Vec<u32> = (0..12).map(|j| 0x100 | ((j + k) & 1)).collect();
                t.seq(kd.is_bound, true, 0, 0, &bound);
                t.stub(kd.unbind, true, 0, 0, 0x61);
                let words = [4u32, 5, 3][kind];
                t.stub(kd.dimensions, true, words, words, 0x62);
                let creates: Vec<u32> = (0..8)
                    .map(|j| if k % 13 == 6 && j == 1 { 0x2a } else { 0 })
                    .collect();
                t.seq(kd.create, true, 4, 4, &creates);
                let binds: Vec<u32> = (0..8)
                    .map(|j| if k % 17 == 9 && j == 0 { 0x2b } else { 0 })
                    .collect();
                t.seq(kd.bind, true, 0, 0, &binds);
            }
            let reads: Vec<u32> = (0..200)
                .map(|j| if k % 19 == 4 && j == 9 { 0x2c } else { 0 })
                .collect();
            t.stub(malloc, false, 2, 0, 0xb0f0);
            t.stub(ctor, true, 0, 0, 0x63);
            t.stub(atexit, false, 1, 0, 0);
            t.stub(quality_of, true, 0, 0, quality);
            t.seq(read_exact, true, 2, 2, &reads);
            t.stub(free, false, 1, 0, 0);
            // the frame from ebp - 0x54 (the iterator), ebp = top - 0x10
            t.check_frame(
                &format!("{label}: reload {k}"),
                0x0046_6da0,
                Conv::Stdcall,
                &[0x5eed, info],
                0x64,
                0x20,
                |g, frame| {
                    Out::U32(rl::reload::reload_cluster_textures(
                        g,
                        frame + 0x54,
                        0x5eed,
                        info,
                    ))
                },
            );
            t.reset();
        }
    }
}
