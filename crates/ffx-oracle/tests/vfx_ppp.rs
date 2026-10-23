//! `ffx_vfx`'s ppp draws below `0x007e0000` that the vfx-ppp rounds port
//! (`tail`, ...) against the original, on two oracles started alike: one
//! runs `FFX.exe`'s code, the other the Rust bridge on the same memory;
//! results and every written byte must agree. Each test runs on the image
//! alone (synthetic state, needs `FFX_GAME_DIR`) and, with `FFX_SNAPSHOT`,
//! on the snapshot's state too. The helpers are `tests/vfx.rs`'s.

#![allow(dead_code, clippy::many_single_char_names)]

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use ffx_bridge::{Global, Mem, Value};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_vfx::{tail, x87};

fn exe() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    Some(dir?.join("FFX.exe"))
}

fn pages() -> Option<Arc<Pages>> {
    static PAGES: OnceLock<Option<Arc<Pages>>> = OnceLock::new();
    PAGES
        .get_or_init(|| {
            let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
                eprintln!("FFX_SNAPSHOT is not set; snapshot cases skipped");
                return None;
            };
            Some(Pages::read(PathBuf::from(snap)).unwrap())
        })
        .clone()
}

struct Xorshift(u32);

impl Xorshift {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
    fn chance(&mut self, percent: u32) -> bool {
        self.below(100) < percent
    }
    fn pick<T: Copy>(&mut self, of: &[T]) -> T {
        of[self.below(u32::try_from(of.len()).unwrap()) as usize]
    }
    /// Float bits: specials (quiet NaNs, infinities, zeros, denormals) a
    /// fifth of the time, else ordinary values. No signalling NaNs: the
    /// oracle's x87 keeps them signalling where the hardware quiets them
    /// (`x87_nans_match_the_oracle`).
    fn float(&mut self) -> u32 {
        if self.chance(20) {
            let f = self.pick(&SPECIALS);
            return if x87::is_snan(f) { f | 0x0040_0000 } else { f };
        }
        let mantissa = self.next() & 0x007f_ffff;
        let exp = 100 + self.below(56); // about 2^-27 .. 2^28
        (self.next() & 0x8000_0000) | (exp << 23) | mantissa
    }
}

const SPECIALS: [u32; 14] = [
    0x0000_0000,
    0x8000_0000,
    0x7f80_0000,
    0xff80_0000,
    0x7fc0_0000,
    0xffc0_0000,
    0x7fc1_2345,
    0xffe0_0001,
    0x7f80_0001, // signalling
    0xff9f_0000, // signalling
    0x0000_0001, // denormal
    0x807f_ffff, // denormal
    0x7f7f_ffff,
    0x3f80_0000,
];

/// The two oracles.
struct Pair {
    name: &'static str,
    a: Oracle,
    b: Oracle,
}

/// Free code space in the image (the end of `.text`), for the float thunk.
const THUNK: u32 = 0x00b0_bf00;

impl Pair {
    /// On the image alone, and on the snapshot when there is one.
    fn all() -> Vec<Pair> {
        let Some(exe) = exe() else { return Vec::new() };
        // Oracle::load sets the game thread's control word (0x027f)
        let image = Pair {
            name: "image",
            a: Oracle::load(&exe).unwrap(),
            b: Oracle::load(&exe).unwrap(),
        };
        let mut v = vec![image];
        if let Some(p) = pages() {
            v.push(Pair {
                name: "snapshot",
                a: Oracle::from_snapshot(&exe, &p).unwrap(),
                b: Oracle::from_snapshot(&exe, &p).unwrap(),
            });
        }
        v
    }

    fn snapshot(&self) -> bool {
        self.name == "snapshot"
    }

    fn sym(&self, sym: u32) -> u32 {
        self.a.rebase(sym)
    }

    fn set<T: Value>(&mut self, addr: u32, v: T) {
        Mem::write(&mut self.a, addr, v);
        Mem::write(&mut self.b, addr, v);
    }

    fn put<T: Value>(&mut self, g: Global<T>, v: T) {
        let at = self.sym(g.sym);
        self.set(at, v);
    }

    fn get<T: Value>(&self, addr: u32) -> T {
        Mem::read(&self.a, addr)
    }

    fn alloc(&mut self, len: usize) -> u32 {
        let x = self.a.alloc(len).unwrap();
        assert_eq!(self.b.alloc(len).unwrap(), x);
        x
    }

    fn fill(&mut self, at: u32, len: u32, rng: &mut Xorshift) {
        for i in (0..len).step_by(4) {
            let v = rng.next();
            self.set(at + i, v);
        }
    }

    /// Runs the original (cdecl) in `a` and `rust` in `b`; checks the result
    /// (when `rust` gives one) and the memory; returns the original's.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        args: &[u32],
        rust: impl FnOnce(&mut Oracle) -> Option<u32>,
    ) -> u32 {
        let at = self.sym(sym);
        let r = self
            .a
            .call(at, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{}: {what}: {e}", self.name));
        // a float the original leaves in ST0 is a result: the port must
        // return it (check_f32 and float_call), not only eax
        assert!(
            r.st0.is_none(),
            "{}: {what}: the original returns {:?} in ST0",
            self.name,
            r.st0
        );
        let mine = rust(&mut self.b);
        if let Some(m) = mine {
            assert_eq!(r.eax, m, "{}: {what}: result", self.name);
        }
        self.same(what);
        r.eax
    }

    fn same(&mut self, what: &str) {
        let d = self.a.diff(&mut self.b).unwrap();
        assert!(
            d.is_empty(),
            "{}: {what}: memory differs (address, original, rust): {:x?}",
            self.name,
            &d[..d.len().min(16)]
        );
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// A thunk that calls `sym(*arg)` and stores its `f32` result (`ST0`)
    /// at `out`; returns the thunk, in both oracles alike.
    fn float_thunk(&mut self, sym: u32, arg: u32, out: u32) -> u32 {
        let at = self.sym(THUNK);
        let target = self.sym(sym);
        let mut c = vec![0xff, 0x35]; // push dword [arg]
        c.extend(arg.to_le_bytes());
        c.push(0xe8); // call target
        c.extend(target.wrapping_sub(at + 11).to_le_bytes());
        c.extend([0x83, 0xc4, 0x04]); // add esp, 4
        c.extend([0xd9, 0x1d]); // fstp dword [out]
        c.extend(out.to_le_bytes());
        c.push(0xc3);
        self.a.write(at, &c).unwrap();
        self.b.write(at, &c).unwrap();
        at
    }

    /// Calls a float thunk in `a` and stores `rust`'s result at `out` in
    /// `b`; checks both.
    fn check_f32(
        &mut self,
        what: &str,
        thunk: u32,
        out: u32,
        rust: impl FnOnce(&mut Oracle) -> f32,
    ) {
        self.a
            .call(thunk, Conv::Cdecl, &[])
            .unwrap_or_else(|e| panic!("{}: {what}: {e}", self.name));
        let v = rust(&mut self.b);
        Mem::write(&mut self.b, out, v.to_bits());
        let o: u32 = self.get(out);
        assert_eq!(o, v.to_bits(), "{}: {what}: result", self.name);
        self.same(what);
    }
}

/// A float that is a number (no NaNs or infinities: the rotations' x87
/// sums of products would pick between two NaNs by register order).
fn finite(rng: &mut Xorshift) -> u32 {
    loop {
        let v = rng.float();
        if (v & 0x7f80_0000) != 0x7f80_0000 {
            return v;
        }
    }
}

/// A float in `[lo, hi)` with 16 fraction bits, as bits.
fn ranged(rng: &mut Xorshift, lo: f32, hi: f32) -> u32 {
    #[allow(clippy::cast_precision_loss)]
    let t = rng.below(0x1_0000) as f32 / 65536.0;
    (lo + (hi - lo) * t).to_bits()
}

fn call_original(m: &mut Oracle, f: u32, args: &[u32]) -> u32 {
    m.call(f, Conv::Cdecl, args).unwrap().eax
}

/// [`deref_recorder_with`] in the oracles' code area with a jump to it at
/// `sym`, for functions too close together to hold a recorder each.
fn far_recorder(p: &mut Pair, sym: u32, words: &[u8], ret: Option<u32>) -> u32 {
    let (c, log) = deref_recorder_code(p, words, ret);
    let to = p.a.alloc_code(&c).unwrap();
    assert_eq!(p.b.alloc_code(&c).unwrap(), to);
    let at = p.sym(sym);
    let mut j = vec![0xe9]; // jmp to
    j.extend(to.wrapping_sub(at + 5).to_le_bytes());
    p.a.write(at, &j).unwrap();
    p.b.write(at, &j).unwrap();
    log
}

/// The code of a [`deref_recorder_with`] and its log. A word count of
/// [`SKIP`] leaves that argument out (a pointer into the caller's frame).
fn deref_recorder_code(p: &mut Pair, words: &[u8], ret: Option<u32>) -> (Vec<u8>, u32) {
    let log = p.alloc(4 + 0x100 * 64);
    let mut c = vec![0x8b, 0x0d]; // mov ecx, [count]
    c.extend(log.to_le_bytes());
    c.extend([0x81, 0xe1, 0xff, 0, 0, 0]); // and ecx, 0xff
    c.extend([0xc1, 0xe1, 0x06]); // shl ecx, 6
    let mut pos = 0u32;
    for (k, &n) in (0u8..).zip(words) {
        if n == SKIP {
            continue;
        }
        c.extend([0x8b, 0x44, 0x24, 4 + 4 * k]); // mov eax, [esp + 4 + 4k]
        if n == 0 {
            c.extend([0x89, 0x81]); // mov [ecx + slot], eax
            c.extend((log + 4 + 4 * pos).to_le_bytes());
            pos += 1;
        } else {
            for j in 0..n {
                c.extend([0x8b, 0x50, 4 * j]); // mov edx, [eax + 4j]
                c.extend([0x89, 0x91]); // mov [ecx + slot], edx
                c.extend((log + 4 + 4 * pos).to_le_bytes());
                pos += 1;
            }
        }
    }
    assert!(pos <= 15);
    c.extend([0xff, 0x05]); // inc dword [count]
    c.extend(log.to_le_bytes());
    c.push(0xa1); // mov eax, [count] or [ret]
    c.extend(ret.unwrap_or(log).to_le_bytes());
    c.push(0xc3);
    (c, log)
}

/// A recorder word count that leaves the argument out.
const SKIP: u8 = 0xff;

/// An `updateVFXRenderDataShapeField(lo, hi, data, req)` stand-in at
/// `sym` that logs (0x100 bytes per call, after the count) its four
/// arguments, the request's eight words and the two matrices it points at
/// (`+0x10` and, when not null, `+0x1c`: they live in the caller's frame).
fn shape_field_update_recorder(p: &mut Pair, sym: u32) -> u32 {
    let log = p.alloc(4 + 0x100 * 0x100);
    let slot = log + 4;
    let mut c = vec![0x56]; // push esi
    c.extend([0x8b, 0x0d]); // mov ecx, [count]
    c.extend(log.to_le_bytes());
    c.extend([0x81, 0xe1, 0xff, 0, 0, 0]); // and ecx, 0xff
    c.extend([0xc1, 0xe1, 0x08]); // shl ecx, 8
    for k in 0..4u8 {
        c.extend([0x8b, 0x44, 0x24, 8 + 4 * k]); // mov eax, [esp + 8 + 4k]
        c.extend([0x89, 0x81]); // mov [ecx + slot + 4k], eax
        c.extend((slot + 4 * u32::from(k)).to_le_bytes());
    }
    c.extend([0x8b, 0x74, 0x24, 0x14]); // mov esi, [esp + 0x14] (req)
    for k in 0..8u8 {
        c.extend([0x8b, 0x46, 4 * k]); // mov eax, [esi + 4k]
        c.extend([0x89, 0x81]); // mov [ecx + slot + 0x10 + 4k], eax
        c.extend((slot + 0x10 + 4 * u32::from(k)).to_le_bytes());
    }
    for (m, off) in [(0x10u8, 0x30u32), (0x1c, 0x70)] {
        c.extend([0x8b, 0x56, m]); // mov edx, [esi + m]
        c.extend([0x85, 0xd2]); // test edx, edx
        c.extend([0x0f, 0x84]); // jz past the copy
        c.extend((16u32 * 9).to_le_bytes());
        for k in 0..16u8 {
            c.extend([0x8b, 0x42, 4 * k]); // mov eax, [edx + 4k]
            c.extend([0x89, 0x81]); // mov [ecx + slot + off + 4k], eax
            c.extend((slot + off + 4 * u32::from(k)).to_le_bytes());
        }
    }
    c.extend([0xff, 0x05]); // inc dword [count]
    c.extend(log.to_le_bytes());
    c.push(0x5e); // pop esi
    c.push(0xc3);
    let to = p.a.alloc_code(&c).unwrap();
    assert_eq!(p.b.alloc_code(&c).unwrap(), to);
    let at = p.sym(sym);
    let mut j = vec![0xe9]; // jmp to
    j.extend(to.wrapping_sub(at + 5).to_le_bytes());
    p.a.write(at, &j).unwrap();
    p.b.write(at, &j).unwrap();
    log
}

/// [`check_deep`] for any number of arguments and an original that aligns
/// its frame (`push ebx; mov ebx, esp; sub esp, 8; and esp, -16; add esp,
/// 4; push ebp; ...; mov ebp, esp`): `rust` gets that `ebp`. Only memory is
/// compared (no caller reads the original's `eax`).
fn check_deep_aligned(
    p: &mut Pair,
    what: &str,
    sym: u32,
    args: &[u32],
    rust: impl FnOnce(&mut Oracle, u32),
) {
    let n = u8::try_from(args.len()).unwrap();
    let at = p.sym(THUNK);
    let mut c = vec![0x81, 0xec, 0, 2, 0, 0]; // sub esp, 0x200
    for _ in 0..n {
        c.extend([0xff, 0xb4, 0x24, 4 * n, 0x02, 0, 0]); // push [esp + 0x200 + 4n]
    }
    c.push(0xe8); // call sym
    let call_end = at + u32::try_from(c.len()).unwrap() + 4;
    c.extend(p.sym(sym).wrapping_sub(call_end).to_le_bytes());
    c.extend([0x81, 0xc4, 4 * n, 0x02, 0, 0]); // add esp, 0x200 + 4n
    c.push(0xc3);
    p.a.write(at, &c).unwrap();
    p.b.write(at, &c).unwrap();
    let words = 4 * u32::from(n);
    // the original's entry esp, then ebp = (esp - 12) & -16
    let entry = p.b.stack_range().end - 0x100 - words - 4 - 0x200 - words - 4;
    let frame = (entry - 12) & !0xf;
    let r =
        p.a.call(at, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{}: {what}: {e}", p.name));
    assert!(
        r.st0.is_none(),
        "{}: {what}: the original returns {:?} in ST0",
        p.name,
        r.st0
    );
    rust(&mut p.b, frame);
    p.same(what);
}

// ---- tail draws (pppKeShpTail*Draw) ------------------------------------

/// The renderer stand-ins the tail draws call, and the cells their
/// results come from.
struct TailStubs {
    /// What `graphicVFXGetRenderDataByInstance` returns.
    rd: u32,
    /// What `graphicDynGeoMemManagerGetBufferForSegment` returns.
    dyn_geo: u32,
    /// What `calculateVFXSP_AlphaFunction` returns.
    alpha: u32,
    /// The recorders' logs.
    logs: Vec<u32>,
}

fn tail_stubs(p: &mut Pair) -> TailStubs {
    let cells = p.alloc(0x10);
    let (rd, dyn_geo, alpha) = (cells, cells + 4, cells + 8);
    let logs = vec![
        far_recorder(p, 0x0064_53d0, &[0, 0], Some(rd)),
        far_recorder(p, 0x0071_3290, &[0, 0], None),
        far_recorder(p, 0x0071_2bd0, &[0, 0, 0, 0], None),
        far_recorder(p, 0x0063_fdb0, &[0, 0, 0, 0], Some(dyn_geo)),
        shape_field_update_recorder(p, 0x0071_a4e0),
        far_recorder(p, 0x0064_58a0, &[0, 0], None),
        far_recorder(p, 0x0064_4b10, &[0, 0], None),
        far_recorder(p, 0x0071_13d0, &[0; 5], None),
        far_recorder(p, 0x0071_2c30, &[0], Some(alpha)),
    ];
    TailStubs {
        rd,
        dyn_geo,
        alpha,
        logs,
    }
}

/// The memory a tail draw reads.
struct TailWorld {
    obj: u32,
    key: u32,
    ctrl: u32,
    offs: u32,
    env: u32,
    env2: u32,
    slots: u32,
    set: u32,
    part: u32,
    rdata: u32,
    parts: u32,
}

impl TailWorld {
    fn new(p: &mut Pair) -> Self {
        TailWorld {
            obj: p.alloc(0x500),
            key: p.alloc(0x40),
            ctrl: p.alloc(0x10),
            offs: p.alloc(0x10),
            env: p.alloc(4),
            env2: p.alloc(0x40),
            slots: p.alloc(0x40),
            // data offsets reach 0x80 below the set
            set: p.alloc(0x280) + 0x80,
            part: p.alloc(0x80),
            rdata: p.alloc(0xa0),
            parts: p.alloc(0x10),
        }
    }
}

/// The ring, colours, key, shape set and part of one random tail draw
/// that ends (no sizing or sprite loop without a bound).
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn tail_case(
    p: &mut Pair,
    rng: &mut Xorshift,
    w: &TailWorld,
    s: &TailStubs,
    v: &tail::bridge::Tail,
) {
    let float = |rng: &mut Xorshift, lo: f32, hi: f32| {
        if rng.chance(3) {
            rng.float()
        } else {
            ranged(rng, lo, hi)
        }
    };
    let TailWorld {
        obj,
        key,
        ctrl,
        offs,
        env,
        env2,
        slots,
        set,
        part,
        rdata,
        parts,
    } = *w;
    p.fill(obj, 0x500, rng);
    p.fill(key, 0x40, rng);
    p.fill(set, 0x200, rng);
    p.fill(part, 0x80, rng);
    p.set(ctrl + 0xc, offs);
    let off0 = rng.pick(&[0u32, 0x10]);
    p.set(offs, off0);
    p.set(offs + 4, rng.pick(&[0x300u32, 0x308]));
    p.set(offs + 8, rng.next());
    p.set(offs + 0xc, rng.pick(&[0x340u32, 0x380]));
    // the object's matrix
    for k in (0x10..0x50).step_by(4) {
        let f = float(rng, -4.0, 4.0);
        p.set(obj + k, f);
    }
    // the ring: count, head, time, frame, positions
    let area = obj + 0xa0 + off0;
    let count = rng.pick(&[1u32, 2, 3, 4, 5, 8, 16, 32]);
    p.set(area, count as u8);
    p.set(area + 1, rng.below(count) as u8);
    p.set(area + 2, rng.next() as u16);
    p.set(area + 4, rng.below(9) as u16);
    let dup = rng.chance(10);
    for i in 0..count {
        for k in 0..4 {
            let f = float(rng, -30.0, 30.0);
            p.set(area + 0x10 + 16 * i + 4 * k, f);
        }
        if dup && rng.chance(40) && i > 0 {
            for k in 0..4 {
                let v: u32 = p.get(area + 16 * i + 4 * k);
                p.set(area + 0x10 + 16 * i + 4 * k, v);
            }
        }
    }
    if rng.chance(15) {
        // on a grid: segments and spacing meet exactly
        let d = rng.pick(&[1.0f32, 2.0, 4.0]);
        for i in 0..count {
            #[allow(clippy::cast_precision_loss)]
            let x = d * i as f32;
            p.set(area + 0x10 + 16 * i, x.to_bits());
            p.set(area + 0x14 + 16 * i, 0u32);
            p.set(area + 0x18 + 16 * i, 0u32);
        }
    }
    // the key: shape, time step, colour, end scale, spacing, count, X bytes
    let shape = if rng.chance(5) { 0xffff } else { rng.below(8) };
    p.set(key + 4, shape);
    let mut n = rng.pick(&[1u32, 2, 2, 3, 4, 5, 8, 12, 16]);
    let mut k18 = float(rng, -1.0, 2.0);
    let mut l = match rng.below(20) {
        0 => rng.pick(&[0u32, 0x8000_0000]),
        1 => float(rng, -20.0, 0.0),
        _ => float(rng, 0.0, 150.0),
    };
    if rng.chance(15) {
        l = rng.pick(&[1.0f32, 2.0, 3.0, 4.0, 8.0]).to_bits();
        k18 = rng.pick(&[0.0f32, 0.5, 1.0]).to_bits();
    }
    if rng.chance(5) {
        // no count: the lengths alone end the loops
        n = 0;
        k18 = ranged(rng, 1.5, 3.0);
        l = ranged(rng, 1.0, 100.0);
    }
    if f32::from_bits(l) == 0.0 {
        // a zero length leaves no count: the steps must be numbers (only
        // the guarded draw makes 0 of a single sprite's step)
        if !v.guarded || rng.chance(50) {
            n = n.max(2);
        }
        k18 = ranged(rng, -1.0, 2.0);
    }
    if v.two {
        // the scale from +0xc to +0x10 (k18's part), the colour bytes from
        // the fill
        let s0 = if n == 0 {
            1.0f32.to_bits()
        } else if f32::from_bits(l) == 0.0 {
            ranged(rng, 0.5, 2.0)
        } else if rng.chance(5) {
            rng.pick(&[0u32, 0x8000_0000])
        } else {
            float(rng, -2.0, 3.0)
        };
        p.set(key + 0xc, s0);
        p.set(key + 0x10, k18);
    } else {
        p.set(key + 0x18, k18);
    }
    p.set(key + 0x1c, l);
    p.set(key + 0x20, n as u16);
    for k in 0x22..0x26 {
        p.set(key + k, rng.pick(&[0u8, 0, 1, 0x80]));
    }
    // the shape set: frames, per frame a data offset, a length and flags
    p.set(env, env2);
    p.set(env2 + 0x24, slots);
    for i in 0..8u32 {
        p.set(slots + 4 * i, set);
    }
    p.set(set + 6, rng.pick(&[0i16, 1, 3, 5, 8, 9, -1]));
    for f in 0..10u32 {
        let e = set + 8 * f;
        let to = if rng.chance(20) { -0x80 } else { 0x100 };
        p.set(e + 0x10, to + 8 * rng.below(8) as i16);
        let len = if rng.chance(10) {
            rng.next() as i16
        } else {
            rng.below(0x40) as i16
        };
        p.set(e + 0x12, len);
        p.set(e + 0x14, rng.pick(&[0u8, 0x80, 0xff, 0x7f]));
    }
    for k in (0x100..0x180).step_by(4) {
        p.set(set + k, rng.below(0x40));
        p.set(set + k - 0x180, rng.below(0x40));
    }
    if rng.chance(20) {
        // the time reaches the frame's length exactly
        let f = u32::from(p.get::<u16>(area + 4));
        let len: u16 = p.get(set + 8 * f + 0x12);
        let time: u16 = p.get(area + 2);
        p.set(key + 8, len.wrapping_sub(time));
    }
    // the part's scales
    for k in [0x60u32, 0x64] {
        let f = float(rng, -3.0, 3.0);
        p.set(part + k, f);
    }
    p.set(p.sym(0x0230_ffe4), part);
    p.set(p.sym(0x00c3_a4cc), env); // g_pppEnvSlot
    if !p.snapshot() || rng.chance(50) {
        let view = p.sym(0x0230_ff20);
        for k in (0..0x40).step_by(4) {
            let f = float(rng, -2.0, 2.0);
            p.set(view + k, f);
        }
    }
    p.set(p.sym(0x0230_fd2c), rng.next());
    let any = rng.below(0x10);
    p.set(p.sym(0x0230_fd20), rng.pick(&[0u32, 8, 0x8000_0001, any]));
    p.set(p.sym(0x0230_fd34), u32::from(rng.chance(15)));
    p.set(s.rd, if rng.chance(30) { 0 } else { rdata });
    p.set(rdata + 0x94, parts);
    p.fill(parts, 0x10, rng);
    p.set(s.dyn_geo, u32::from(!rng.chance(10)));
    p.set(s.alpha, rng.next());
    for &log in &s.logs {
        p.set(log, 0u32);
    }
}

#[test]
#[allow(clippy::cast_possible_truncation)]
fn vfx_ppp_r1a_shp_tail_draws_match_the_original() {
    use tail::bridge as t;
    let draws = [
        ("pppKeShpTailDraw", 0x0074_c570u32, t::TAIL),
        ("pppKeShpTailXDraw", 0x0074_d570, t::TAIL_X),
        ("pppKeShpTail2Draw", 0x0074_e570, t::TAIL2),
        ("pppKeShpTail2XDraw", 0x0074_f5b0, t::TAIL2_X),
    ];
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0074_c570);
        let s = tail_stubs(&mut p);
        let w = TailWorld::new(&mut p);
        let mut seen = [0u32; 6];
        for round in 0..4000u32 {
            let (name, sym, v) = draws[rng.below(4) as usize];
            tail_case(&mut p, &mut rng, &w, &s, &v);
            // the request's words neither side writes: alike in both
            let words = 3;
            let entry = p.b.stack_range().end - 0x100 - 4 * words - 4 - 0x200 - 4 * words - 4;
            let frame = (entry - 12) & !0xf;
            let req = frame - 0x120;
            let (s1, s2) = (rng.next(), rng.next());
            p.set(req, s1);
            p.set(req + 0x18, s2);
            let n: u16 = p.get(w.key + 0x20);
            seen[usize::from(v.x.is_some())] += 1;
            seen[2 + usize::from(v.two)] += 1;
            seen[4 + usize::from(n > 3)] += 1;
            let (obj, key, ctrl) = (w.obj, w.key, w.ctrl);
            check_deep_aligned(
                &mut p,
                &format!("{name} round {round}"),
                sym,
                &[obj, key, ctrl],
                |o, frame| {
                    t::ppp_ke_shp_tail_draw(
                        o,
                        &v,
                        obj,
                        key,
                        ctrl,
                        frame,
                        &mut call_original,
                        &mut |o, f, a| o.call_f64(f, a).unwrap(),
                    );
                },
            );
        }
        assert!(seen.iter().all(|&k| k > 0), "{}: {seen:?}", p.name);
    }
}

// ---- camera model draws (pppDrawMdlCamera*Draw) ------------------------

/// A renderer entry point's stand-in at `sym` (through a jump) that logs,
/// 0x200 bytes per call after the count, its `nargs` arguments, the
/// request's first `words` words (the request is argument 3) and the 16
/// words of each matrix the request points at (`mats`, offsets in the
/// request; a null pointer logs nothing). Returns 0.
fn request_recorder(p: &mut Pair, sym: u32, nargs: u8, words: u8, mats: &[u8]) -> u32 {
    let log = p.alloc(4 + 0x200 * 0x100);
    let slot = log + 4;
    let mut c = vec![0x56]; // push esi
    c.extend([0x8b, 0x0d]); // mov ecx, [count]
    c.extend(log.to_le_bytes());
    c.extend([0x81, 0xe1, 0xff, 0, 0, 0]); // and ecx, 0xff
    c.extend([0xc1, 0xe1, 0x09]); // shl ecx, 9
    for k in 0..nargs {
        c.extend([0x8b, 0x44, 0x24, 8 + 4 * k]); // mov eax, [esp + 8 + 4k]
        c.extend([0x89, 0x81]); // mov [ecx + slot + 4k], eax
        c.extend((slot + 4 * u32::from(k)).to_le_bytes());
    }
    c.extend([0x8b, 0x74, 0x24, 0x14]); // mov esi, [esp + 0x14] (req)
    let base = slot + 4 * u32::from(nargs);
    for k in 0..words {
        c.extend([0x8b, 0x46, 4 * k]); // mov eax, [esi + 4k]
        c.extend([0x89, 0x81]); // mov [ecx + base + 4k], eax
        c.extend((base + 4 * u32::from(k)).to_le_bytes());
    }
    let mut off = base + 4 * u32::from(words);
    for &m in mats {
        c.extend([0x8b, 0x56, m]); // mov edx, [esi + m]
        c.extend([0x85, 0xd2]); // test edx, edx
        c.extend([0x0f, 0x84]); // jz past the copy
        c.extend((16u32 * 9).to_le_bytes());
        for k in 0..16u8 {
            c.extend([0x8b, 0x42, 4 * k]); // mov eax, [edx + 4k]
            c.extend([0x89, 0x81]); // mov [ecx + off + 4k], eax
            c.extend((off + 4 * u32::from(k)).to_le_bytes());
        }
        off += 0x40;
    }
    assert!(off - slot <= 0x200);
    c.extend([0xff, 0x05]); // inc dword [count]
    c.extend(log.to_le_bytes());
    c.extend([0x31, 0xc0, 0x5e, 0xc3]); // xor eax, eax; pop esi; ret
    let to = p.a.alloc_code(&c).unwrap();
    assert_eq!(p.b.alloc_code(&c).unwrap(), to);
    let at = p.sym(sym);
    let mut j = vec![0xe9]; // jmp to
    j.extend(to.wrapping_sub(at + 5).to_le_bytes());
    p.a.write(at, &j).unwrap();
    p.b.write(at, &j).unwrap();
    log
}

#[test]
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::similar_names
)]
fn vfx_ppp_r1c_mdl_camera_draws_match_the_original() {
    use ffx_vfx::mdl_cam::bridge as mc;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0073_d930);
        let cells = p.alloc(0x20);
        let (rd, alpha, zmat, tex, cnt) = (cells, cells + 4, cells + 8, cells + 0xc, cells + 0x10);
        let logs = [
            far_recorder(&mut p, 0x0064_53d0, &[0, 0], Some(rd)),
            far_recorder(&mut p, 0x0071_2c30, &[0], Some(alpha)),
            far_recorder(&mut p, 0x0071_2d00, &[0], Some(zmat)),
            request_recorder(&mut p, 0x0071_8610, 4, 18, &[0x20, 0x2c, 0x30]),
            request_recorder(&mut p, 0x0071_1180, 5, 18, &[0x20, 0x2c, 0x30]),
            request_recorder(&mut p, 0x0071_0fd0, 5, 18, &[0x20, 0x2c, 0x30]),
            far_recorder(&mut p, 0x0064_5600, &[0, 0], None),
            far_recorder(&mut p, 0x0064_58a0, &[0, 0], None),
            far_recorder(&mut p, 0x0091_9ba0, &[0], Some(tex)),
            far_recorder(&mut p, 0x0091_9be0, &[0, 0, 0], None),
            far_recorder(&mut p, 0x0091_9bc0, &[0], Some(cnt)),
            far_recorder(&mut p, 0x0091_9bd0, &[0, 0], None),
        ];
        let (obj, key, prog, table) = (p.alloc(0x300), p.alloc(0x40), p.alloc(0x10), p.alloc(0x10));
        let (env, env2, recs, models, part) = (
            p.alloc(4),
            p.alloc(0x40),
            p.alloc(0x400),
            p.alloc(0x100),
            p.alloc(0x80),
        );
        let slot = p.sym(0x00c3_a4cc); // g_pppEnvSlot
        let matrices = [0x0230_ff20u32, 0x0230_ff60, 0x0230_ffa0].map(|a| p.sym(a));
        let (cur, cpart, shift, stop, r0) = (
            p.sym(0x0230_fd20),
            p.sym(0x0230_ffe4),
            p.sym(0x00c8_b500),
            p.sym(0x0230_fd34),
            p.sym(0x00c8_f508),
        );
        let mut seen = [0u32; 2];
        for round in 0..2000u32 {
            for log in logs {
                p.set(log, 0u32);
            }
            let looped = rng.chance(50);
            p.fill(obj, 0x300, &mut rng);
            p.fill(key, 0x40, &mut rng);
            p.fill(recs, 0x400, &mut rng);
            p.fill(models, 0x100, &mut rng);
            for o in (0..0x100).step_by(4) {
                // small words: flags, blends and the camera textures' offsets
                p.set(models + o, (4 * rng.below(0x10)) | rng.below(4));
            }
            if !p.snapshot() || rng.chance(50) {
                for m in matrices {
                    for o in (0..0x40).step_by(4) {
                        let x = ranged(&mut rng, -2.0, 2.0);
                        p.set(m + o, x);
                    }
                }
            }
            for o in (0x10..0x90).step_by(4) {
                let x = ranged(&mut rng, -4.0, 4.0);
                p.set(obj + o, x);
            }
            p.fill(part, 0x80, &mut rng);
            p.fill(r0, 0x10, &mut rng);
            p.set(prog + 0xc, table);
            p.set(table, 4 * rng.below(0x10));
            p.set(table + 4, rng.next());
            let off2 = rng.pick(&[0u32, 0x40, 0x80]);
            p.set(table + 8, off2);
            let w = obj + 0xa0 + off2;
            // the work area: floats where the draws step and read them
            for o in (0..0x90).step_by(4) {
                let x = if rng.chance(3) {
                    rng.float()
                } else if rng.chance(20) {
                    ranged(&mut rng, -70000.0, 70000.0)
                } else {
                    ranged(&mut rng, -300.0, 300.0)
                };
                p.set(w + o, x);
            }
            for b in [0x1cu32, 0x1d, 0x1e, 0x1f, 0x8c, 0x8d, 0x8e, 0x8f] {
                p.set(w + b, rng.pick(&[0u8, 0, 1, 2, 3, 0xff]));
            }
            if rng.chance(30) {
                p.set(w, 0u32);
            }
            if rng.chance(30) {
                p.set(w + 0x88, 0u32);
            }
            for o in (8..0x20).step_by(4) {
                let x = ranged(&mut rng, -50.0, 50.0);
                p.set(key + o, x);
            }
            if rng.chance(50) {
                let f: u32 = p.get(obj + 0xc);
                p.set(key, f);
            }
            for o in 0x20..0x2a {
                p.set(key + o, rng.pick(&[0u8, 0, 1, 2, 3, 4, 0xff]));
            }
            p.set(stop, u32::from(rng.chance(15)));
            p.set(shift, if rng.chance(15) { 4 * rng.below(4) } else { 0 });
            p.set(cpart, part);
            p.set(slot, env);
            p.set(env, env2);
            p.set(env2 + 0x20, recs);
            let model = if rng.chance(5) { 0xffff } else { rng.below(8) };
            p.set(key + 4, model);
            for k in 0..0x20u32 {
                let at = if rng.chance(5) {
                    0
                } else {
                    models + 4 * rng.below(0x30)
                };
                p.set(recs + 32 * k + 0x1c, at);
            }
            p.set(obj + 0x90, models + rng.below(0x20));
            p.set(obj + 0x94, models + rng.below(0x20));
            p.set(rd, u32::from(rng.chance(50)));
            for c in [alpha, zmat] {
                let x = if rng.chance(50) { 0 } else { rng.next() };
                p.set(c, x);
            }
            p.set(
                tex,
                if rng.chance(30) {
                    0
                } else {
                    0x100 + rng.below(0x100)
                },
            );
            p.set(cnt, rng.next());
            let r9 = rng.next();
            p.set(cur, rng.pick(&[0u32, 3, 0x8000_0001, r9]));
            // the request's untouched words: alike on both sides
            let words = 3;
            let entry = p.b.stack_range().end - 0x100 - 4 * words - 4 - 0x200 - 4 * words - 4;
            let frame = (entry - 12) & !0xf;
            let req = frame - if looped { 0x1c0 } else { 0x1a0 };
            for o in (0..0x48).step_by(4) {
                let v = rng.next();
                p.set(req + o, v);
            }
            seen[usize::from(looped)] += 1;
            let (obj_, key_, prog_) = (obj, key, prog);
            if looped {
                check_deep_aligned(
                    &mut p,
                    &format!("pppDrawMdlCameraLoopDraw round {round}"),
                    0x0073_fa50,
                    &[obj, key, prog],
                    |o, frame| {
                        mc::ppp_draw_mdl_camera_loop_draw(
                            o,
                            obj_,
                            key_,
                            prog_,
                            frame,
                            &mut call_original,
                        );
                    },
                );
            } else {
                check_deep_aligned(
                    &mut p,
                    &format!("pppDrawMdlCameraDraw round {round}"),
                    0x0073_d930,
                    &[obj, key, prog],
                    |o, frame| {
                        mc::ppp_draw_mdl_camera_draw(
                            o,
                            obj_,
                            key_,
                            prog_,
                            frame,
                            &mut call_original,
                            &mut |o, f, a| o.call_f64(f, a).unwrap(),
                        );
                    },
                );
            }
        }
        assert!(seen.iter().all(|&k| k > 0), "{}: {seen:?}", p.name);
    }
}

#[test]
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::similar_names
)]
fn vfx_ppp_r1d_mdl_sea_draw_matches_the_original() {
    use ffx_vfx::mdl_cam::bridge as mc;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0073_abe0);
        let cells = p.alloc(0x20);
        let (rd, alpha, zmat, tex, level, setup, loaded) = (
            cells,
            cells + 4,
            cells + 8,
            cells + 0xc,
            cells + 0x10,
            cells + 0x14,
            cells + 0x18,
        );
        let logs = [
            far_recorder(&mut p, 0x0064_53d0, &[0, 0], Some(rd)),
            far_recorder(&mut p, 0x0071_2c30, &[0], Some(alpha)),
            far_recorder(&mut p, 0x0071_2d00, &[0], Some(zmat)),
            request_recorder(&mut p, 0x0071_8610, 4, 18, &[0x20, 0x2c, 0x30]),
            request_recorder(&mut p, 0x0071_1180, 5, 18, &[0x20, 0x2c, 0x30]),
            request_recorder(&mut p, 0x0071_0fd0, 5, 18, &[0x20, 0x2c, 0x30]),
            far_recorder(&mut p, 0x0064_5600, &[0, 0], None),
            far_recorder(&mut p, 0x0064_58a0, &[0, 0], None),
            far_recorder(&mut p, 0x0064_0060, &[0, 0], None),
            far_recorder(&mut p, 0x0091_9ba0, &[0], Some(tex)),
            far_recorder(&mut p, 0x0091_9bd0, &[0, 0], None),
            far_recorder(&mut p, 0x0064_1da0, &[], Some(level)),
            far_recorder(&mut p, 0x0073_b5c0, &[0, 0], Some(setup)),
            // the texture's name (8 words of it) and where its record goes
            far_recorder(&mut p, 0x0064_5400, &[8, 0, 0, 0, 0, 0], Some(loaded)),
            far_recorder(&mut p, 0x0064_5aa0, &[0, 0, 0], None),
        ];
        let (obj, key, prog, table) = (p.alloc(0x300), p.alloc(0x60), p.alloc(0x10), p.alloc(0x10));
        let (env, env2, recs, models, part, rest, flags) = (
            p.alloc(4),
            p.alloc(0x40),
            p.alloc(0x400),
            p.alloc(0x200),
            p.alloc(0x80),
            p.alloc(0x100),
            p.alloc(0x40),
        );
        let slot = p.sym(0x00c3_a4cc); // g_pppEnvSlot
        let view = p.sym(0x0230_ff20);
        let (cur, cpart, shift, stop, r0) = (
            p.sym(0x0230_fd20),
            p.sym(0x0230_ffe4),
            p.sym(0x00c8_b500),
            p.sym(0x0230_fd34),
            p.sym(0x00c8_f508),
        );
        let levels = mc::SEA_LEVELS;
        let mut seen = [0u32; 3];
        for round in 0..2000u32 {
            for log in logs {
                p.set(log, 0u32);
            }
            p.fill(obj, 0x300, &mut rng);
            p.fill(key, 0x60, &mut rng);
            p.fill(recs, 0x400, &mut rng);
            p.fill(models, 0x200, &mut rng);
            p.fill(rest, 0x100, &mut rng);
            p.fill(flags, 0x40, &mut rng);
            // two models: vertices at +0x20, a count, a blend
            for m in [models, models + 0x100] {
                p.set(m, rng.below(4));
                p.set(m + 8, 0x20u32);
                let count = rng.pick(&[0i16, 1, 3, 8, 0x20, -1]);
                p.set(m + 0x12, count);
            }
            for i in 0..0x40u32 {
                p.set(flags + i, rng.pick(&[0u8, 1, 0x80]));
            }
            if !p.snapshot() || rng.chance(50) {
                for o in (0..0x40).step_by(4) {
                    let x = ranged(&mut rng, -2.0, 2.0);
                    p.set(view + o, x);
                }
            }
            for o in (0x10..0x90).step_by(4) {
                let x = ranged(&mut rng, -4.0, 4.0);
                p.set(obj + o, x);
            }
            p.fill(part, 0x80, &mut rng);
            p.fill(r0, 0x10, &mut rng);
            p.set(prog + 0xc, table);
            p.set(table, 4 * rng.below(0x10));
            p.set(table + 4, rng.next());
            let off2 = rng.pick(&[0u32, 0x40, 0x80]);
            p.set(table + 8, off2);
            let w = obj + 0xa0 + off2;
            for o in (0x40..0x68).step_by(4) {
                let x = if rng.chance(3) {
                    rng.float()
                } else if rng.chance(20) {
                    ranged(&mut rng, -70000.0, 70000.0)
                } else {
                    ranged(&mut rng, -300.0, 300.0)
                };
                p.set(w + o, x);
            }
            p.set(w + 0x68, rest);
            p.set(w + 0x6c, flags);
            p.set(w + 0x70, if rng.chance(30) { 0 } else { rest });
            for b in [0x74u32, 0x75, 0x76, 0x77] {
                p.set(w + b, rng.pick(&[0u8, 0, 1, 2, 3, 0xff]));
            }
            for o in (8..0x20).step_by(4) {
                let x = ranged(&mut rng, -50.0, 50.0);
                p.set(key + o, x);
            }
            // the wave: amplitudes, phase scales and steps
            for o in (0x20..0x4c).step_by(4) {
                let x = if rng.chance(3) {
                    rng.float()
                } else {
                    ranged(&mut rng, -40.0, 40.0)
                };
                p.set(key + o, x);
            }
            if rng.chance(50) {
                let f: u32 = p.get(obj + 0xc);
                p.set(key, f);
            }
            for o in 0x50..0x58 {
                p.set(key + o, rng.pick(&[0u8, 0, 1, 2, 3, 4, 0xff]));
            }
            p.set(stop, u32::from(rng.chance(15)));
            p.set(shift, if rng.chance(15) { 4 * rng.below(4) } else { 0 });
            p.set(cpart, part);
            p.set(slot, env);
            p.set(env, env2);
            p.set(env2 + 0x20, recs);
            let model = if rng.chance(5) { 0xffff } else { rng.below(8) };
            p.set(key + 4, model);
            for k in 0..8u32 {
                p.set(recs + 32 * k + 0x1c, models + 0x100 * rng.below(2));
            }
            p.set(obj + 0x90, models + rng.below(0x20));
            p.set(obj + 0x94, models + rng.below(0x20));
            let other = rng.next();
            let lv = if rng.chance(85) {
                rng.pick(&levels)
            } else {
                other
            };
            p.set(level, lv);
            p.set(setup, u32::from(rng.chance(70)));
            p.set(loaded, u32::from(rng.chance(50)));
            p.set(rd, u32::from(rng.chance(50)));
            for c in [alpha, zmat] {
                let x = if rng.chance(50) { 0 } else { rng.next() };
                p.set(c, x);
            }
            p.set(tex, if rng.chance(30) { 0 } else { models + 0x100 });
            let r9 = rng.next();
            p.set(cur, rng.pick(&[0u32, 3, 0x8000_0001, r9]));
            let words = 3;
            let entry = p.b.stack_range().end - 0x100 - 4 * words - 4 - 0x200 - 4 * words - 4;
            let frame = (entry - 12) & !0xf;
            let req = frame - 0x2e0;
            for o in (0..0x48).step_by(4) {
                let v = rng.next();
                p.set(req + o, v);
            }
            seen[0] += 1;
            seen[1] += u32::from(levels.contains(&lv) && model != 0xffff);
            seen[2] += u32::from(p.get::<u8>(w + 0x77) != 0);
            let (obj_, key_, prog_) = (obj, key, prog);
            check_deep_aligned(
                &mut p,
                &format!("pppDrawMdlSeaDraw round {round}"),
                0x0073_abe0,
                &[obj, key, prog],
                |o, frame| {
                    mc::ppp_draw_mdl_sea_draw(o, obj_, key_, prog_, frame, &mut call_original);
                },
            );
        }
        assert!(seen.iter().all(|&k| k > 0), "{}: {seen:?}", p.name);
    }
}

// ---- Tail3 draws (pppKeShpTail3Draw, pppKeShpTail3XDraw) -----------------

/// The memory a `Tail3` draw reads besides the globals.
struct Tail3World {
    obj: u32,
    key: u32,
    ctrl: u32,
    offs: u32,
    env: u32,
    env2: u32,
    slots: u32,
    set: u32,
    part: u32,
    rdata: u32,
    parts: u32,
    spr: u32,
}

/// One random `Tail3` draw that ends (no sizing or sprite loop without a
/// bound: a count of 0 or a zero spacing get finite, shrinking steps).
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn tail3_case(p: &mut Pair, rng: &mut Xorshift, w: &Tail3World, cells: u32, x: bool) {
    let float = |rng: &mut Xorshift, lo: f32, hi: f32| {
        if rng.chance(3) {
            rng.float()
        } else {
            ranged(rng, lo, hi)
        }
    };
    let Tail3World {
        obj,
        key,
        ctrl,
        offs,
        env,
        env2,
        slots,
        set,
        part,
        rdata,
        parts,
        spr,
    } = *w;
    p.fill(obj, 0x500, rng);
    p.fill(key, 0x60, rng);
    p.fill(set, 0x200, rng);
    p.fill(part, 0x80, rng);
    p.fill(spr, 0x200, rng);
    p.set(ctrl + 0xc, offs);
    let off0 = rng.pick(&[0u32, 0x10]);
    p.set(offs, off0);
    p.set(offs + 4, rng.pick(&[0x300u32, 0x308]));
    p.set(offs + 8, rng.next());
    p.set(offs + 0xc, rng.pick(&[0x340u32, 0x380]));
    for k in (0x10..0x50).step_by(4) {
        let f = float(rng, -4.0, 4.0);
        p.set(obj + k, f);
    }
    let area = obj + 0xa0 + off0;
    // the ring: 28 points, the head
    for i in 0..28u32 {
        for k in 0..4 {
            let f = float(rng, -30.0, 30.0);
            p.set(area + 0x30 + 16 * i + 4 * k, f);
        }
    }
    if rng.chance(15) {
        let d = rng.pick(&[1.0f32, 2.0, 4.0]);
        for i in 0..28u32 {
            #[allow(clippy::cast_precision_loss)]
            let v = d * i as f32;
            p.set(area + 0x30 + 16 * i, v.to_bits());
            p.set(area + 0x34 + 16 * i, 0u32);
            p.set(area + 0x38 + 16 * i, 0u32);
        }
    }
    let head = if rng.chance(25) { 27 } else { rng.below(28) };
    p.set(area + 0x1f8, head as u8);
    // the key: count, size, spacing
    let shape = if x && rng.chance(5) {
        0xffff
    } else {
        rng.below(8)
    };
    p.set(key + 4, shape);
    let mut count = rng.pick(&[1u32, 2, 2, 3, 4, 5, 8, 12, 16, 40]);
    let mut size = float(rng, -2.0, 3.0);
    let mut to = float(rng, -1.0, 2.0);
    let mut l = match rng.below(20) {
        0 => rng.pick(&[0u32, 0x8000_0000]),
        1 => float(rng, -20.0, 0.0),
        _ => float(rng, 0.0, 150.0),
    };
    if rng.chance(15) {
        l = rng.pick(&[1.0f32, 2.0, 3.0, 4.0, 8.0]).to_bits();
        size = 1.0f32.to_bits();
        to = rng.pick(&[0.0f32, 0.5, 1.0]).to_bits();
    }
    if rng.chance(5) {
        // no count: the spacing alone ends the loops
        count = 0;
        size = ranged(rng, 0.5, 2.0);
        to = (f32::from_bits(size) + f32::from_bits(ranged(rng, 0.5, 2.0))).to_bits();
        l = ranged(rng, 1.0, 100.0);
    }
    if f32::from_bits(l) == 0.0 {
        // a zero spacing leaves no count: the steps must be numbers
        count = count.max(2);
        size = ranged(rng, 0.5, 2.0);
        to = ranged(rng, -1.0, 2.0);
    }
    p.set(key + 0xc, size);
    p.set(key + 0x10, to);
    let k14 = float(rng, -1.0, 2.0);
    p.set(key + 0x14, k14);
    p.set(key + 0x18, l);
    p.set(key + 0x1c, count as u8);
    p.set(key + 0x1d, rng.pick(&[0u8, 0, 1, 0x80]));
    p.set(key + 0x1e, rng.pick(&[0u16, 0, 1, 0x8000]));
    for k in 0x50..0x53 {
        p.set(key + k, rng.pick(&[0u8, 0, 1, 0x80]));
    }
    // the shape set: frames, per frame a data offset and length
    p.set(env, env2);
    p.set(env2 + 0x24, slots);
    for i in 0..8u32 {
        p.set(slots + 4 * i, set);
    }
    p.set(set + 6, rng.pick(&[1i16, 2, 3, 5, 8, 10]));
    let len = match rng.below(10) {
        0 => rng.pick(&[-1i16, -0x100, i16::MIN]),
        1 => 1,
        _ => 1 + rng.below(0x40) as i16,
    };
    p.set(set + 0x12, len);
    for f in 0..10u32 {
        let e = set + 8 * f;
        let to = if rng.chance(20) { -0x80 } else { 0x100 };
        p.set(e + 0x10, to + 8 * rng.below(8) as i16);
    }
    for k in (0x100..0x180).step_by(4) {
        p.set(set + k, rng.below(0x40));
        p.set(set + k - 0x180, rng.below(0x40));
    }
    // the part's scales, the view matrix, the emulated VU0 registers
    for k in (0x60..0x70).step_by(4) {
        let f = float(rng, -3.0, 3.0);
        p.set(part + k, f);
    }
    p.set(p.sym(0x0230_ffe4), part);
    p.set(p.sym(0x00c3_a4cc), env); // g_pppEnvSlot
    let view = p.sym(0x0230_ff20);
    for k in (0..0x40).step_by(4) {
        let f = float(rng, -2.0, 2.0);
        p.set(view + k, f);
    }
    let vu = p.sym(0x00c8_f780);
    for k in (0..0x220).step_by(4) {
        let f = float(rng, -2.0, 2.0);
        p.set(vu + k, f);
    }
    let vf00 = p.sym(0x00c0_a004);
    p.set(vf00, rng.pick(&[0u32, 0x3f80_0000]));
    let w1 = ranged(rng, -2.0, 2.0);
    p.set(vf00 + 0xc, rng.pick(&[0x3f80_0000u32, w1]));
    if !p.snapshot() {
        // the sine table: random values (the image's is filled at start)
        let sin = p.sym(0x00c4_4bf0);
        for k in (0..0x4000).step_by(4) {
            let f = ranged(rng, -1.0, 1.0);
            p.set(sin + k, f);
        }
    }
    p.set(op_top(p), 0i8);
    p.set(p.sym(0x0119_fe10), spr);
    p.set(p.sym(0x0230_fd2c), rng.next());
    let any = rng.below(0x10);
    p.set(p.sym(0x0230_fd20), rng.pick(&[0u32, 8, 0x8000_0001, any]));
    let stop = if rng.chance(10) {
        rng.pick(&[1u32, 2, 0x100, 0x8000_0000])
    } else {
        0
    };
    p.set(p.sym(0x0230_fd34), stop);
    let skip = if rng.chance(8) {
        rng.pick(&[1u32, 0x40, 0x8000_0000])
    } else {
        0
    };
    p.set(p.sym(0x012f_b7d0), skip);
    // the renderer's answers
    let (rd, dyn_geo, alpha, shp_alpha) = (cells, cells + 4, cells + 8, cells + 0xc);
    p.set(rd, if rng.chance(30) { 0 } else { rdata });
    p.set(rdata + 0x90, rng.pick(&[0i32, 1, 1, 2, 3, -1]));
    p.set(rdata + 0x94, parts);
    p.fill(parts, 0x6c * 4, rng);
    p.set(dyn_geo, u32::from(!rng.chance(10)));
    p.set(alpha, rng.next());
    p.set(shp_alpha, rng.next());
}

/// Puts a stub in front of the stand-in `sym` jumps to that adds 1 to
/// each word at `words` (globals) first.
fn clobber_before(p: &mut Pair, sym: u32, words: &[u32]) {
    let at = p.sym(sym);
    let rel: u32 = p.get(at + 1);
    let target = at.wrapping_add(5).wrapping_add(rel);
    let mut c = Vec::new();
    for &w in words {
        c.extend([0xff, 0x05]); // inc dword [w]
        c.extend(p.sym(w).to_le_bytes());
    }
    let code = p.a.alloc_code(&[0; 64]).unwrap();
    assert_eq!(p.b.alloc_code(&[0; 64]).unwrap(), code);
    c.push(0xe9); // jmp target
    let end = code + u32::try_from(c.len()).unwrap() + 4;
    c.extend(target.wrapping_sub(end).to_le_bytes());
    p.a.write(code, &c).unwrap();
    p.b.write(code, &c).unwrap();
    let mut j = vec![0xe9]; // jmp code
    j.extend(code.wrapping_sub(at + 5).to_le_bytes());
    p.a.write(at, &j).unwrap();
    p.b.write(at, &j).unwrap();
}

/// `g_OpSprTop` (the `op_spr_alloc` stack's top index).
fn op_top(p: &Pair) -> u32 {
    p.sym(0x0233_2ea1)
}

#[test]
#[allow(clippy::cast_possible_truncation)]
fn vfx_ppp_r2a_shp_tail3_draws_match_the_original() {
    use tail::bridge as t;
    let draws = [
        ("pppKeShpTail3Draw", 0x0075_0620u32, t::TAIL3),
        ("pppKeShpTail3XDraw", 0x0075_1fc0, t::TAIL3_X),
    ];
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0075_0620);
        let cells = p.alloc(0x10);
        let logs = [
            far_recorder(&mut p, 0x0064_53d0, &[0, 0], Some(cells)),
            far_recorder(&mut p, 0x0071_3290, &[0, 0], None),
            far_recorder(&mut p, 0x0071_2bd0, &[0, 0, 0, 0], None),
            far_recorder(&mut p, 0x0063_fdb0, &[0, 0, 0, 0], Some(cells + 4)),
            shape_field_update_recorder(&mut p, 0x0071_a4e0),
            far_recorder(&mut p, 0x0064_58a0, &[0, 0], None),
            far_recorder(&mut p, 0x0064_4b10, &[0, 0], None),
            far_recorder(&mut p, 0x0071_13d0, &[0; 5], None),
            far_recorder(&mut p, 0x0071_2c30, &[0], Some(cells + 8)),
            far_recorder(&mut p, 0x0071_6550, &[0], Some(cells + 0xc)),
        ];
        // the renderer's update uses the emulated VU0 registers too: its
        // stand-in changes some the `X` draw reloads after each sprite
        clobber_before(
            &mut p,
            0x0071_a4e0,
            &[
                0x00c8_f834,
                0x00c8_f7f4,
                0x00c8_f938,
                0x00c8_f944,
                0x00c8_f958,
            ],
        );
        let w = Tail3World {
            obj: p.alloc(0x500),
            key: p.alloc(0x60),
            ctrl: p.alloc(0x10),
            offs: p.alloc(0x10),
            env: p.alloc(4),
            env2: p.alloc(0x40),
            slots: p.alloc(0x40),
            set: p.alloc(0x280) + 0x80,
            part: p.alloc(0x80),
            rdata: p.alloc(0xa0),
            parts: p.alloc(0x6c * 4),
            spr: p.alloc(0x200),
        };
        let mut seen = [0u32; 4];
        for round in 0..3000u32 {
            let (name, sym, v) = draws[rng.below(2) as usize];
            tail3_case(&mut p, &mut rng, &w, cells, v.x);
            let words = 3;
            let entry = p.b.stack_range().end - 0x100 - 4 * words - 4 - 0x200 - 4 * words - 4;
            let frame = (entry - 12) & !0xf;
            // the request's bytes neither side writes, and the stack
            // words the draw reads unwritten: alike in both
            let req = frame - v.req;
            let (s1, s2) = (rng.next(), rng.next());
            p.set(req, s1);
            p.set(req + 0x18, s2);
            for o in v.stale {
                let s = if rng.chance(50) {
                    rng.float()
                } else {
                    rng.next()
                };
                p.set(frame - o, s);
            }
            for &log in &logs {
                p.set(log, 0u32);
            }
            seen[usize::from(v.x)] += 1;
            let (obj, key, ctrl) = (w.obj, w.key, w.ctrl);
            check_deep_aligned(
                &mut p,
                &format!("{name} round {round}"),
                sym,
                &[obj, key, ctrl],
                |o, frame| {
                    t::ppp_ke_shp_tail3_draw(
                        o,
                        &v,
                        obj,
                        key,
                        ctrl,
                        frame,
                        &mut call_original,
                        &mut |o, f, a| o.call_f64(f, a).unwrap(),
                    );
                },
            );
            let updates: u32 = p.get(logs[4]);
            seen[2 + usize::from(updates > 2)] += 1;
        }
        assert!(seen.iter().all(|&k| k > 0), "{}: {seen:?}", p.name);
    }
}

// ---- pppKeShpTailPhtDraw -------------------------------------------------

/// A stand-in at `sym` (through a jump) that logs the `words` words its
/// first argument points at (0x80 bytes per call, after the count) and
/// returns the word at `ret` (or the count).
fn block_recorder(p: &mut Pair, sym: u32, words: u8, ret: Option<u32>) -> u32 {
    assert!(words <= 32);
    let log = p.alloc(4 + 0x80 * 0x100);
    let slot = log + 4;
    let mut c = vec![0x8b, 0x0d]; // mov ecx, [count]
    c.extend(log.to_le_bytes());
    c.extend([0x81, 0xe1, 0xff, 0, 0, 0]); // and ecx, 0xff
    c.extend([0xc1, 0xe1, 0x07]); // shl ecx, 7
    c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp + 4]
    for k in 0..words {
        c.extend([0x8b, 0x42, 4 * k]); // mov eax, [edx + 4k]
        c.extend([0x89, 0x81]); // mov [ecx + slot + 4k], eax
        c.extend((slot + 4 * u32::from(k)).to_le_bytes());
    }
    c.extend([0xff, 0x05]); // inc dword [count]
    c.extend(log.to_le_bytes());
    c.push(0xa1); // mov eax, [ret] or [count]
    c.extend(ret.unwrap_or(log).to_le_bytes());
    c.push(0xc3);
    let to = p.a.alloc_code(&c).unwrap();
    assert_eq!(p.b.alloc_code(&c).unwrap(), to);
    let at = p.sym(sym);
    let mut j = vec![0xe9]; // jmp to
    j.extend(to.wrapping_sub(at + 5).to_le_bytes());
    p.a.write(at, &j).unwrap();
    p.b.write(at, &j).unwrap();
    log
}

/// An `Sg_GetDBuffDC(out)` stand-in: `*out` = the word at `half`,
/// returns the word at `dc`.
fn dbuff_stub(p: &mut Pair, half: u32, dc: u32) {
    let mut c = vec![0x8b, 0x54, 0x24, 0x04]; // mov edx, [esp + 4]
    c.push(0xa1); // mov eax, [half]
    c.extend(half.to_le_bytes());
    c.extend([0x89, 0x02]); // mov [edx], eax
    c.push(0xa1); // mov eax, [dc]
    c.extend(dc.to_le_bytes());
    c.push(0xc3);
    let to = p.a.alloc_code(&c).unwrap();
    assert_eq!(p.b.alloc_code(&c).unwrap(), to);
    let at = p.sym(0x0082_0530);
    let mut j = vec![0xe9];
    j.extend(to.wrapping_sub(at + 5).to_le_bytes());
    p.a.write(at, &j).unwrap();
    p.b.write(at, &j).unwrap();
}

#[test]
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::similar_names
)]
fn vfx_ppp_r2b_shp_tail_pht_draw_matches_the_original() {
    use tail::bridge as t;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0075_4300);
        let cells = p.alloc(0x20);
        let (rd, dyn_geo, alpha, half, dc, gif) = (
            cells,
            cells + 4,
            cells + 8,
            cells + 0x10,
            cells + 0x14,
            cells + 0x18,
        );
        dbuff_stub(&mut p, half, dc);
        let logs = [
            far_recorder(&mut p, 0x0064_53d0, &[0, 0], Some(rd)),
            far_recorder(&mut p, 0x0071_3290, &[0, 0], None),
            far_recorder(&mut p, 0x0071_2bd0, &[0, 0, 0, 0], None),
            far_recorder(&mut p, 0x0063_fdb0, &[0, 0, 0, 0], Some(dyn_geo)),
            shape_field_update_recorder(&mut p, 0x0071_a4e0),
            far_recorder(&mut p, 0x0064_58a0, &[0, 0], None),
            far_recorder(&mut p, 0x0064_4b10, &[0, 0], None),
            far_recorder(&mut p, 0x0071_13d0, &[0; 5], None),
            far_recorder(&mut p, 0x0071_2c30, &[0], Some(alpha)),
            block_recorder(&mut p, 0x0076_8d30, 24, None),
            block_recorder(&mut p, 0x0076_7790, 24, Some(gif)),
            far_recorder(&mut p, 0x007e_c420, &[0, 0, 0, 0], None),
        ];
        let (obj, key, ctrl, offs) = (p.alloc(0x500), p.alloc(0x70), p.alloc(0x10), p.alloc(0x10));
        let (env, env2, slots) = (p.alloc(4), p.alloc(0x40), p.alloc(0x40));
        let set = p.alloc(0x280) + 0x80;
        let (part, rdata, parts, dcb) =
            (p.alloc(0x80), p.alloc(0xa0), p.alloc(0x10), p.alloc(0x300));
        let float = |rng: &mut Xorshift, lo: f32, hi: f32| {
            if rng.chance(3) {
                rng.float()
            } else {
                ranged(rng, lo, hi)
            }
        };
        let mut seen = [0u32; 4];
        for round in 0..2500u32 {
            p.fill(obj, 0x500, &mut rng);
            p.fill(key, 0x70, &mut rng);
            p.fill(set, 0x200, &mut rng);
            p.fill(part, 0x80, &mut rng);
            p.fill(dcb, 0x300, &mut rng);
            p.set(ctrl + 0xc, offs);
            let off0 = rng.pick(&[0u32, 0x10]);
            p.set(offs, off0);
            p.set(offs + 4, rng.pick(&[0x300u32, 0x308]));
            p.set(offs + 8, rng.pick(&[0x340u32, 0x380]));
            for k in (0x10..0x50).step_by(4) {
                let f = float(&mut rng, -4.0, 4.0);
                p.set(obj + k, f);
            }
            let area = obj + 0xa0 + off0;
            for i in 0..27u32 {
                for k in 0..4 {
                    let f = float(&mut rng, -30.0, 30.0);
                    p.set(area + 0x30 + 16 * i + 4 * k, f);
                }
            }
            if rng.chance(15) {
                let d = rng.pick(&[1.0f32, 2.0, 4.0]);
                for i in 0..27u32 {
                    #[allow(clippy::cast_precision_loss)]
                    let v = d * i as f32;
                    p.set(area + 0x30 + 16 * i, v.to_bits());
                    p.set(area + 0x34 + 16 * i, 0u32);
                    p.set(area + 0x38 + 16 * i, 0u32);
                }
            }
            let head = if rng.chance(25) { 26 } else { rng.below(27) };
            p.set(area + 0x1f4, head as u8);
            let up = float(&mut rng, -10.0, 10.0);
            p.set(area + 0x1e0, up);
            for k in (0x340..0x3c0).step_by(4) {
                let f = float(&mut rng, -2.0, 2.0);
                p.set(obj + 0xa0 + k, f);
            }
            // the key
            let shape = if rng.chance(5) { 0xffff } else { rng.below(8) };
            p.set(key + 4, shape);
            let mut count = rng.pick(&[1u32, 2, 2, 3, 4, 5, 8, 12, 16, 40]);
            let mut size = float(&mut rng, -2.0, 3.0);
            let mut to = float(&mut rng, -1.0, 2.0);
            let mut l = match rng.below(20) {
                0 => rng.pick(&[0u32, 0x8000_0000]),
                1 => float(&mut rng, -20.0, 0.0),
                _ => float(&mut rng, 0.0, 150.0),
            };
            if rng.chance(15) {
                l = rng.pick(&[1.0f32, 2.0, 3.0, 4.0, 8.0]).to_bits();
                size = 1.0f32.to_bits();
                to = rng.pick(&[0.0f32, 0.5, 1.0]).to_bits();
            }
            if rng.chance(5) {
                count = 0;
                size = ranged(&mut rng, 0.5, 2.0);
                to = (f32::from_bits(size) + f32::from_bits(ranged(&mut rng, 0.5, 2.0))).to_bits();
                l = ranged(&mut rng, 1.0, 100.0);
            }
            if f32::from_bits(l) == 0.0 {
                count = count.max(2);
                size = ranged(&mut rng, 0.5, 2.0);
                to = ranged(&mut rng, -1.0, 2.0);
            }
            p.set(key + 0xc, size);
            p.set(key + 0x10, to);
            let k14 = float(&mut rng, -1.0, 2.0);
            p.set(key + 0x14, k14);
            p.set(key + 0x18, l);
            for k in [0x28u32, 0x2c] {
                let f = float(&mut rng, -3.0, 3.0);
                p.set(key + k, f);
            }
            p.set(key + 0x60, count as u8);
            for k in 0x61..0x65 {
                p.set(key + k, rng.pick(&[0u8, 0, 1, 0x80, 0xff]));
            }
            // the shape set
            p.set(env, env2);
            p.set(env2 + 0x24, slots);
            for i in 0..8u32 {
                p.set(slots + 4 * i, set);
            }
            p.set(set + 6, rng.pick(&[1i16, 2, 3, 5, 8, 10]));
            let len = match rng.below(10) {
                0 => rng.pick(&[-1i16, -0x100, i16::MIN]),
                1 => 1,
                _ => 1 + rng.below(0x40) as i16,
            };
            p.set(set + 0x12, len);
            for f in 0..10u32 {
                let to = if rng.chance(20) { -0x80 } else { 0x100 };
                p.set(set + 8 * f + 0x10, to + 8 * rng.below(8) as i16);
            }
            for k in (0x100..0x180).step_by(4) {
                p.set(set + k, rng.below(0x40));
                p.set(set + k - 0x180, rng.below(0x40));
            }
            for k in (0x60..0x70).step_by(4) {
                let f = float(&mut rng, -3.0, 3.0);
                p.set(part + k, f);
            }
            p.set(p.sym(0x0230_ffe4), part);
            p.set(p.sym(0x00c3_a4cc), env);
            let view = p.sym(0x0230_ff20);
            for k in (0..0x40).step_by(4) {
                let f = float(&mut rng, -2.0, 2.0);
                p.set(view + k, f);
            }
            if !p.snapshot() {
                let sin = p.sym(0x00c4_4bf0);
                for k in (0..0x4000).step_by(4) {
                    let f = ranged(&mut rng, -1.0, 1.0);
                    p.set(sin + k, f);
                }
            }
            p.set(p.sym(0x0230_fd2c), rng.next() & !0xf);
            let any = rng.below(0x10);
            p.set(p.sym(0x0230_fd20), rng.pick(&[0u32, 8, 0x8000_0001, any]));
            p.set(rd, if rng.chance(30) { 0 } else { rdata });
            p.set(rdata + 0x94, parts);
            p.fill(parts, 0x10, &mut rng);
            p.set(dyn_geo, u32::from(!rng.chance(10)));
            p.set(alpha, rng.next());
            let hv = rng.next();
            p.set(half, hv);
            p.set(dc, dcb);
            p.set(gif, if rng.chance(50) { 0 } else { rng.next() });
            for &log in &logs {
                p.set(log, 0u32);
            }
            // the frame starts alike in both: the words the draw reads
            // unwritten (the record's +0x2c)
            let words = 3;
            let entry = p.b.stack_range().end - 0x100 - 4 * words - 4 - 0x200 - 4 * words - 4;
            let frame = (entry - 12) & !0xf;
            for o in (4..0x250).step_by(4) {
                let v = if rng.chance(50) {
                    rng.next()
                } else {
                    ranged(&mut rng, -1e5, 1e5)
                };
                p.set(frame - o, v);
            }
            seen[usize::from(shape == 0xffff)] += 1;
            check_deep_aligned(
                &mut p,
                &format!("pppKeShpTailPhtDraw round {round}"),
                0x0075_4300,
                &[obj, key, ctrl],
                |o, frame| {
                    t::ppp_ke_shp_tail_pht_draw(
                        o,
                        obj,
                        key,
                        ctrl,
                        frame,
                        &mut call_original,
                        &mut |o, f, a| o.call_f64(f, a).unwrap(),
                    );
                },
            );
            let ot: u32 = p.get(logs[11]);
            seen[2 + usize::from(ot > 0)] += 1;
        }
        assert!(seen.iter().all(|&k| k > 0), "{}: {seen:?}", p.name);
    }
}

// ---- pppKeMdlBmpDraw -----------------------------------------------------

/// Puts `code` (ending in a fall-through) in front of the stand-in `sym`
/// jumps to.
fn prefix_before(p: &mut Pair, sym: u32, code: &[u8]) {
    let at = p.sym(sym);
    let rel: u32 = p.get(at + 1);
    let target = at.wrapping_add(5).wrapping_add(rel);
    let to = p.a.alloc_code(&[0; 64]).unwrap();
    assert_eq!(p.b.alloc_code(&[0; 64]).unwrap(), to);
    let mut c = code.to_vec();
    c.push(0xe9); // jmp target
    let end = to + u32::try_from(c.len()).unwrap() + 4;
    c.extend(target.wrapping_sub(end).to_le_bytes());
    p.a.write(to, &c).unwrap();
    p.b.write(to, &c).unwrap();
    let mut j = vec![0xe9];
    j.extend(to.wrapping_sub(at + 5).to_le_bytes());
    p.a.write(at, &j).unwrap();
    p.b.write(at, &j).unwrap();
}

/// A `pppKeMdlBmpDraw_Body_00748b00(block)` stand-in that logs (0x100
/// bytes per call) the block's ten words and what its pointers hold: the
/// matrix (`+0`), the colours (`+0x18`, `+0x1c`) and the direction
/// (`+0x20`), all in the caller's frame.
fn bmp_body_recorder(p: &mut Pair) -> u32 {
    let log = p.alloc(4 + 0x100 * 0x100);
    let slot = log + 4;
    let mut c = vec![0x56]; // push esi
    c.extend([0x8b, 0x0d]); // mov ecx, [count]
    c.extend(log.to_le_bytes());
    c.extend([0x81, 0xe1, 0xff, 0, 0, 0]); // and ecx, 0xff
    c.extend([0xc1, 0xe1, 0x08]); // shl ecx, 8
    c.extend([0x8b, 0x74, 0x24, 0x08]); // mov esi, [esp + 8]
    let mut pos = 0u32;
    let mut copy = |c: &mut Vec<u8>, n: u8| {
        for k in 0..n {
            c.extend([0x8b, 0x42, 4 * k]); // mov eax, [edx + 4k]
            c.extend([0x89, 0x81]); // mov [ecx + slot + pos], eax
            c.extend((slot + 4 * pos).to_le_bytes());
            pos += 1;
        }
    };
    c.extend([0x89, 0xf2]); // mov edx, esi
    copy(&mut c, 10);
    for (at, n) in [(0u8, 16u8), (0x18, 4), (0x1c, 4), (0x20, 4)] {
        c.extend([0x8b, 0x56, at]); // mov edx, [esi + at]
        copy(&mut c, n);
    }
    c.extend([0xff, 0x05]); // inc dword [count]
    c.extend(log.to_le_bytes());
    c.push(0x5e); // pop esi
    c.push(0xc3);
    let to = p.a.alloc_code(&c).unwrap();
    assert_eq!(p.b.alloc_code(&c).unwrap(), to);
    let at = p.sym(0x0074_8b00);
    let mut j = vec![0xe9];
    j.extend(to.wrapping_sub(at + 5).to_le_bytes());
    p.a.write(at, &j).unwrap();
    p.b.write(at, &j).unwrap();
    log
}

/// A `dbgPrintf` stand-in that logs (32 bytes per call) the format and,
/// for the format `long` (three doubles), the six words after it.
#[allow(clippy::similar_names)]
fn dbg_recorder(p: &mut Pair, long: u32) -> u32 {
    let log = p.alloc(4 + 0x20 * 0x100);
    let slot = log + 4;
    let mut c = vec![0x8b, 0x0d]; // mov ecx, [count]
    c.extend(log.to_le_bytes());
    c.extend([0x81, 0xe1, 0xff, 0, 0, 0]); // and ecx, 0xff
    c.extend([0xc1, 0xe1, 0x05]); // shl ecx, 5
    c.extend([0x8b, 0x44, 0x24, 0x04]); // mov eax, [esp + 4]
    c.extend([0x89, 0x81]); // mov [ecx + slot], eax
    c.extend(slot.to_le_bytes());
    c.push(0x3d); // cmp eax, long
    c.extend(long.to_le_bytes());
    let mut tail = Vec::new();
    for k in 1..7u8 {
        tail.extend([0x8b, 0x44, 0x24, 4 + 4 * k]); // mov eax, [esp + 4 + 4k]
        tail.extend([0x89, 0x81]); // mov [ecx + slot + 4k], eax
        tail.extend((slot + 4 * u32::from(k)).to_le_bytes());
    }
    c.extend([0x75, u8::try_from(tail.len()).unwrap()]); // jne past
    c.extend(tail);
    c.extend([0xff, 0x05]); // inc dword [count]
    c.extend(log.to_le_bytes());
    c.push(0xc3);
    let to = p.a.alloc_code(&c).unwrap();
    assert_eq!(p.b.alloc_code(&c).unwrap(), to);
    let at = p.sym(0x0062_f500);
    let mut j = vec![0xe9];
    j.extend(to.wrapping_sub(at + 5).to_le_bytes());
    p.a.write(at, &j).unwrap();
    p.b.write(at, &j).unwrap();
    log
}

#[test]
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::similar_names
)]
fn vfx_ppp_r2c_ke_mdl_bmp_draw_matches_the_original() {
    use ffx_vfx::mdl_cam::bridge as mc;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0074_90d0);
        let long = p.sym(0x00b5_1630);
        let logs = [bmp_body_recorder(&mut p), dbg_recorder(&mut p, long)];
        // the body moves the block's packet position
        prefix_before(
            &mut p,
            0x0074_8b00,
            &[0x8b, 0x54, 0x24, 0x04, 0x81, 0x42, 0x08, 0, 2, 0, 0], // mov edx, [esp + 4]; add dword [edx + 8], 0x200
        );
        let (obj, key, ctrl, offs) = (p.alloc(0x200), p.alloc(0x40), p.alloc(0x10), p.alloc(0x10));
        let (env, sets, verts, tris) =
            (p.alloc(4), p.alloc(0x100), p.alloc(0x100), p.alloc(0x2400));
        let (part, recs, other, spr) =
            (p.alloc(0x40), p.alloc(0x40), p.alloc(0x100), p.alloc(0x80));
        let float = |rng: &mut Xorshift, lo: f32, hi: f32| {
            if rng.chance(3) {
                finite(rng)
            } else {
                ranged(rng, lo, hi)
            }
        };
        let mut seen = [0u32; 4];
        for round in 0..2500u32 {
            p.fill(obj, 0x200, &mut rng);
            p.fill(key, 0x40, &mut rng);
            p.fill(other, 0x100, &mut rng);
            p.fill(spr, 0x80, &mut rng);
            let prep = rng.chance(70);
            p.set(obj + 0xc, if prep { 0 } else { rng.pick(&[0u32, 1]) });
            p.set(key, if prep { 0 } else { rng.pick(&[0u32, 1]) });
            p.set(p.sym(0x0230_fd34), u32::from(rng.chance(10)));
            for k in (0x10..0x50).step_by(4) {
                let f = float(&mut rng, -3.0, 3.0);
                p.set(obj + k, f);
            }
            p.set(ctrl + 0xc, offs);
            p.set(offs + 4, rng.pick(&[0u32, 0x20, 0x40]));
            let pick = |rng: &mut Xorshift| if rng.chance(5) { 0xffff } else { rng.below(4) };
            let (k24, k28) = (pick(&mut rng), pick(&mut rng));
            p.set(key + 0x24, k24);
            p.set(key + 0x28, k28);
            p.set(
                key + 0x2c,
                if rng.chance(10) {
                    u32::MAX
                } else {
                    rng.below(4)
                },
            );
            p.set(key + 0x3c, rng.below(6).wrapping_sub(1));
            // the shape sets: vertices and triangle records
            p.set(p.sym(0x00c3_a4cc), env);
            let env2 = p.alloc(0x40);
            p.set(env, env2);
            p.set(env2 + 0x20, sets);
            for k in 0..4u32 {
                p.set(sets + 32 * k + 0x1c, if k % 2 == 0 { verts } else { tris });
            }
            p.set(verts + 8, 0x10u32);
            for k in (0x10..0x100).step_by(2) {
                let v = rng.next() as i16;
                p.set(verts + k, v);
            }
            p.set(tris + 4, 0x10u32);
            let mut at = tris + 0x10;
            for r in 0..rng.below(4) {
                let kind = if r > 0 && rng.chance(5) {
                    rng.pick(&[0u8, 1, 3])
                } else {
                    2
                };
                p.set(at + 1, kind);
                let n = if r == 0 && rng.chance(3) {
                    0x100
                } else {
                    rng.pick(&[0i16, 1, 2, 3, 4])
                };
                p.set(at + 2, n);
                let n = u32::try_from(n.max(0)).unwrap();
                for t in 0..n {
                    let e = at + 0x10 + 32 * t;
                    p.fill(e, 0x20, &mut rng);
                    for o in [0xcu32, 0xe, 0x10] {
                        p.set(e + o, rng.below(16) as u16);
                    }
                    let small = rng.chance(40);
                    for o in (0x14u32..0x20).step_by(2) {
                        let v = if small {
                            rng.below(4) as u16
                        } else {
                            rng.next() as u16
                        };
                        p.set(e + o, v);
                    }
                }
                at += 0x10 + 32 * n;
            }
            p.set(at + 1, 0xffu8);
            // the target
            p.set(p.sym(0x0230_ffe4), part);
            p.set(part + 0x3c, recs);
            let k30 = rng.pick(&[0u32, 0x10, 0x20]);
            p.set(key + 0x30, k30);
            for k in 0..4u32 {
                let t = if rng.chance(15) {
                    p.sym(0x00d5_2398).wrapping_sub(0xa0 + k30)
                } else {
                    other
                };
                p.set(recs + 16 * k + 4, t);
            }
            for k in (0xa0..0x100).step_by(4) {
                let f = float(&mut rng, -10.0, 10.0);
                p.set(other + k, f);
            }
            let world = p.sym(0x0230_ffa0);
            for k in (0..0x40).step_by(4) {
                let f = float(&mut rng, -2.0, 2.0);
                p.set(world + k, f);
            }
            p.set(p.sym(0x0233_2ea1), 0i8);
            p.set(p.sym(0x0119_fe10), spr);
            p.set(p.sym(0x0230_fd2c), rng.next());
            for &log in &logs {
                p.set(log, 0u32);
            }
            let words = 3;
            let entry = p.b.stack_range().end - 0x100 - 4 * words - 4 - 0x200 - 4 * words - 4;
            let frame = (entry - 12) & !0xf;
            for o in (4..0x110).step_by(4) {
                let v = rng.next();
                p.set(frame - o, v);
            }
            seen[usize::from(prep)] += 1;
            check_deep_aligned(
                &mut p,
                &format!("pppKeMdlBmpDraw round {round}"),
                0x0074_90d0,
                &[obj, key, ctrl],
                |o, frame| {
                    mc::ppp_ke_mdl_bmp_draw(o, obj, key, ctrl, frame, &mut call_original);
                },
            );
            let body: u32 = p.get(logs[0]);
            let dbg: u32 = p.get(logs[1]);
            seen[2] += u32::from(body > 0);
            seen[3] += u32::from(dbg > 0);
        }
        assert!(seen.iter().all(|&k| k > 0), "{}: {seen:?}", p.name);
    }
}
