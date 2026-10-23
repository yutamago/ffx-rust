//! `op_shape_draw` (`0x007ed0c0`, `ffx_vfx::shape_draw`) against the
//! original, piece by piece: each vertex loop entered at its first
//! instruction through a thunk that builds the original's frame and
//! registers as the head leaves them (the loop ends in the original tail,
//! whose `ret` comes back to the thunk's caller), and the whole function on
//! the selections whose loops are ported. Results, every written byte
//! outside the stacks and the frame words the pieces share (`ebp - 0x1bbc`
//! .. `- 0x1bb0`, the frame packet buffer) must agree. The frame is filled
//! with random words first, so a read of a frame word the original never
//! wrote would show as a difference.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use ffx_bridge::Mem;
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_vfx::shape_draw::bridge::{self as shape, FRAME_SIZE, Frame};

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
    /// tenth of the time, else ordinary values. No signalling NaNs: the
    /// oracle's x87 keeps them signalling where the hardware quiets them.
    fn float(&mut self) -> u32 {
        self.float_specials(10)
    }
    /// As [`Xorshift::float`] with specials `percent` of the time.
    fn float_specials(&mut self, percent: u32) -> u32 {
        if self.chance(percent) {
            return self.pick(&SPECIALS);
        }
        let mantissa = self.next() & 0x007f_ffff;
        let exp = 110 + self.below(36); // about 2^-17 .. 2^18
        (self.next() & 0x8000_0000) | (exp << 23) | mantissa
    }
}

const SPECIALS: [u32; 10] = [
    0x0000_0000,
    0x8000_0000,
    0x7f80_0000,
    0xff80_0000,
    0x7fc0_0000,
    0xffc0_0000,
    0x7fc1_2345,
    0xffe0_0001,
    0x0000_0001,
    0x3f80_0000,
];

/// Free code space in the image (the end of `.text`), for the thunks.
const THUNK: u32 = 0x00b0_bf00;
/// `__security_cookie`.
const COOKIE: u32 = 0x00c6_13d8;
/// `g_Ee_Gpr` and the VU0 block after it, filled with floats.
const EE: u32 = 0x00c8_f508;
const EE_END: u32 = 0x00c8_f990;
const SHIFT: u32 = 0x00c8_b500;
const COUNT: u32 = 0x00c8_f5f8;
const S2: u32 = 0x00c8_f628;
const VF00: u32 = 0x00c0_a004;
const ORIGIN_SRC: u32 = 0x012e_95d8;
const SSE2: u32 = 0x0193_ed20;
/// `g_OpFog +0x10`, the four fog vectors S4 and S1 pick from.
const OP_FOG: u32 = 0x011a_0020;

/// The loop entries past their `mov ebx, [r0]` (S4 and S1 load it before
/// the count store, the others after it).
const ENTRIES_AFTER_EBX: [u32; 2] = [0x007f_1a8b, 0x007e_d1d2];

struct Pair {
    name: &'static str,
    a: Oracle,
    b: Oracle,
}

impl Pair {
    fn all() -> Vec<Pair> {
        let Some(exe) = exe() else { return Vec::new() };
        let mut v = vec![Pair {
            name: "image",
            a: Oracle::load(&exe).unwrap(),
            b: Oracle::load(&exe).unwrap(),
        }];
        if let Some(p) = pages() {
            v.push(Pair {
                name: "snapshot",
                a: Oracle::from_snapshot(&exe, &p).unwrap(),
                b: Oracle::from_snapshot(&exe, &p).unwrap(),
            });
        }
        v
    }

    fn sym(&self, sym: u32) -> u32 {
        self.a.rebase(sym)
    }

    fn set<T: ffx_bridge::Value>(&mut self, addr: u32, v: T) {
        Mem::write(&mut self.a, addr, v);
        Mem::write(&mut self.b, addr, v);
    }

    fn alloc(&mut self, len: usize) -> u32 {
        let x = self.a.alloc(len).unwrap();
        assert_eq!(self.b.alloc(len).unwrap(), x);
        x
    }

    /// `n` random float words at `at` (specials `percent` of the time), in
    /// both oracles, in one write.
    fn floats(&mut self, at: u32, n: usize, rng: &mut Xorshift, percent: u32) {
        let bytes: Vec<u8> = (0..n)
            .flat_map(|_| rng.float_specials(percent).to_le_bytes())
            .collect();
        self.code(at, &bytes);
    }

    fn code(&mut self, at: u32, c: &[u8]) {
        self.a.write(at, c).unwrap();
        self.b.write(at, c).unwrap();
    }

    /// `ebp` of a cdecl function the oracle calls with `n` arguments.
    fn ebp(&self, n: u32) -> u32 {
        self.b.stack_range().end - 0x100 - 4 * n - 4 - 4
    }

    /// Random words in the frame below `ebp`, in both oracles.
    fn fill_frame(&mut self, ebp: u32, rng: &mut Xorshift) {
        let words = (FRAME_SIZE / 4) as usize;
        self.floats(ebp - FRAME_SIZE, words, rng, 10);
    }

    /// Results, the memory outside the stacks and the shared frame words.
    fn same(&mut self, what: &str, ebp: u32) {
        let d = self.a.diff(&mut self.b).unwrap();
        assert!(
            d.is_empty(),
            "{}: {what}: memory differs (address, original, rust): {:x?}",
            self.name,
            &d[..d.len().min(16)]
        );
        for off in [0x1bbc, 0x1bb8, 0x1bb0] {
            let (x, y): (u32, u32) = (Mem::read(&self.a, ebp - off), Mem::read(&self.b, ebp - off));
            assert_eq!(x, y, "{}: {what}: frame word ebp-{off:#x}", self.name);
        }
        // up to ebp - 8: `_alloca_probe` keeps the caller's `ecx` there,
        // CRT stack junk below the cookie (ebp - 4), not the function's
        let len = 0x17fc;
        let (x, y) = (
            Oracle::read(&mut self.a, ebp - 0x1804, len).unwrap(),
            Oracle::read(&mut self.b, ebp - 0x1804, len).unwrap(),
        );
        if let Some(i) = (0..len).find(|&i| x[i] != y[i]) {
            let j = (0..len).rfind(|&i| x[i] != y[i]).unwrap();
            panic!(
                "{}: {what}: the frame packet buffer differs at ebp-{:#x}..ebp-{:#x}",
                self.name,
                0x1804 - i,
                0x1804 - j
            );
        }
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// A thunk `(shape, cmd)` that builds the original's frame (cookie,
    /// saved `ebx`/`esi`/`edi`), sets `ecx` = shape, `esi` = cmd, `edi` =
    /// shape + 0x20 (and `ebx` = `r0`'s low word for the entries in
    /// [`ENTRIES_AFTER_EBX`]) and jumps to `entry`; the tail returns from
    /// it.
    fn loop_thunk(&mut self, entry: u32) -> u32 {
        let at = self.sym(THUNK);
        let mut c = vec![0x55, 0x8b, 0xec]; // push ebp; mov ebp, esp
        c.extend([0x81, 0xec]); // sub esp, FRAME_SIZE
        c.extend(FRAME_SIZE.to_le_bytes());
        c.push(0xa1); // mov eax, [cookie]
        c.extend(self.sym(COOKIE).to_le_bytes());
        c.extend([0x33, 0xc5, 0x89, 0x45, 0xfc]); // xor eax, ebp; mov [ebp-4], eax
        c.extend([0x53, 0x56, 0x57]); // push ebx; push esi; push edi
        c.extend([0x8b, 0x4d, 0x08]); // mov ecx, [ebp+8]
        c.extend([0x8b, 0x75, 0x0c]); // mov esi, [ebp+0xc]
        c.extend([0x8d, 0x79, 0x20]); // lea edi, [ecx+0x20]
        if ENTRIES_AFTER_EBX.contains(&entry) {
            c.extend([0x8b, 0x1d]); // mov ebx, [r0]: loaded before the count store
            c.extend(self.sym(EE).to_le_bytes());
        }
        c.push(0xe9); // jmp entry
        let end = at + u32::try_from(c.len()).unwrap() + 4;
        c.extend(self.sym(entry).wrapping_sub(end).to_le_bytes());
        self.code(at, &c);
        at
    }
}

/// A shape (room for `MAX_RECORDS` vertex records), its command and two
/// matrices for S5 (`cmd +0x10` and the texture `cmd +0x1c`), the second
/// one in a page above 2 GB too (a negative texture pointer: S5 adds its
/// low half sign-extended to a 64-bit register).
#[derive(Clone, Copy)]
struct Case {
    shape: u32,
    cmd: u32,
    matrices: [u32; 3],
}

const MAX_RECORDS: u32 = 5;
/// The page above 2 GB for [`Case::matrices`].
const HIGH: u32 = 0xa123_0000;

impl Case {
    fn new(p: &mut Pair) -> Case {
        p.a.map_zeroed(HIGH, 0x1000).unwrap();
        p.b.map_zeroed(HIGH, 0x1000).unwrap();
        Case {
            shape: p.alloc((0x40 + 0x18 * MAX_RECORDS) as usize),
            cmd: p.alloc(0x20),
            matrices: [p.alloc(0x40), p.alloc(0x40), HIGH + 0x7c0],
        }
    }
}

/// Random state for `n` vertex records: EE/VU0 registers (floats, `r0`
/// zero most of the time), the matrix, `vf00`, the origin block, the records' halves
/// (`+0xc` zero most of the time, to reach the second point), the draw
/// flags `s1`/`s2` (often clear, to reach the fixed-point and texture
/// steps), `cmd` (flags without `0xc00`, no texture) and the CPU's SSE2
/// flag (both `_ftol2_sse` paths).
fn plain_case(p: &mut Pair, rng: &mut Xorshift, c: Case, n: u32, vu0: bool) {
    plain_case_with(p, rng, c, n, vu0, false);
}

/// [`plain_case`] for S5: `shape` bit 3 clear, the matrices at `cmd +0x10`
/// and `cmd +0x1c` (random floats; the second the first, the high one or
/// equal to `r0`'s low word now and then, which picks the first).
fn textured_case(p: &mut Pair, rng: &mut Xorshift, c: Case, n: u32) {
    textured_case_with(p, rng, c, n, false);
}

/// [`textured_case`] with `shape` bit 3 set, for S2.
fn vu0_textured_case(p: &mut Pair, rng: &mut Xorshift, c: Case, n: u32) {
    textured_case_with(p, rng, c, n, true);
}

fn textured_case_with(p: &mut Pair, rng: &mut Xorshift, c: Case, n: u32, vu0: bool) {
    plain_case_with(p, rng, c, n, vu0, true);
    for &m in &c.matrices {
        let specials = if rng.chance(80) { 0 } else { 5 };
        p.floats(m, 16, rng, specials);
    }
    let m1 = c.matrices[0];
    let m2 = match rng.below(10) {
        0 => m1,
        1..=2 => c.matrices[2],
        _ => c.matrices[1],
    };
    p.set(c.cmd + 0x10, m1);
    p.set(c.cmd + 0x1c, m2);
    if rng.chance(8) {
        p.set(p.sym(EE), m2);
    }
}

/// [`textured_case`] for S4 (`vu0` clear) or S1 (`vu0` set): `cmd +2`
/// with `0x400`, `0x800` or both (or, now and then, the bit `r0` holds,
/// so the vector choice compares a set bit with it), and the four fog
/// vectors at `g_OpFog +0x10` (most of the time: on the snapshots the
/// game's own are kept otherwise): depth scales of about `-1/16`..`1/16`
/// and limits around 0..255, so the fog value lands below, inside and
/// above them, or zeros of both signs (ties).
fn fog_case_with(p: &mut Pair, rng: &mut Xorshift, c: Case, n: u32, vu0: bool) {
    textured_case_with(p, rng, c, n, vu0);
    let r0: u32 = Mem::read(&p.a, p.sym(EE));
    let bits = match rng.below(10) {
        0 => u16::try_from(r0 & 0xc00).unwrap() | 0x400,
        1..=3 => 0x400,
        4..=6 => 0x800,
        _ => 0xc00,
    };
    let flags: u16 = Mem::read(&p.a, c.cmd + 2);
    p.set(c.cmd + 2, flags & !0xc00 | bits);
    if rng.chance(8) {
        // zeros of both signs: ties between the fog value and its limits
        // (the low limit -1 now and then, so a tie with the high one shows)
        for k in 0..16 {
            let v = if k % 4 == 1 {
                rng.pick(&[0u32, 0x8000_0000, 0xbf80_0000])
            } else {
                rng.pick(&[0u32, 0x8000_0000])
            };
            p.set(p.sym(OP_FOG) + 4 * k, v);
        }
    } else if rng.chance(80) {
        for k in 0..4 {
            let at = p.sym(OP_FOG) + 0x10 * k;
            let small =
                |rng: &mut Xorshift, n: u32| f32::from(u16::try_from(rng.below(n)).unwrap());
            let scale = small(rng, 4096) / 32768.0 - 0.0625;
            let low = small(rng, 256);
            let high = low + small(rng, 256);
            let vector = [scale.to_bits(), low.to_bits(), rng.float(), high.to_bits()];
            for (i, word) in (0u32..).zip(vector) {
                let word = if rng.chance(3) {
                    rng.pick(&SPECIALS)
                } else {
                    word
                };
                p.set(at + 4 * i, word);
            }
        }
    }
}

/// [`fog_case_with`] for S4.
fn fog_case(p: &mut Pair, rng: &mut Xorshift, c: Case, n: u32) {
    fog_case_with(p, rng, c, n, false);
}

fn plain_case_with(p: &mut Pair, rng: &mut Xorshift, c: Case, n: u32, vu0: bool, tex: bool) {
    // the registers and the matrix: few specials most of the time (one in
    // a matrix element makes the whole projected quad NaN)
    let specials = if rng.chance(80) { 1 } else { 10 };
    p.floats(p.sym(EE), ((EE_END - EE) / 4) as usize, rng, specials);
    // r0: zero, a few bits of the first flag mask (so a record's +0xc
    // can equal it and the flag tests compare against it) or garbage
    let r0 = p.sym(EE);
    let any = rng.next();
    let r0_lo = match rng.below(20) {
        0..=14 => 0,
        15..=17 => rng.pick(&[2u32, 8, 0x40, 0x4a, 0x1000, 0x4002]),
        _ => rng.pick(&[1, 0x8000, any]),
    };
    p.set(r0, r0_lo);
    for k in 1..4 {
        let any = rng.next();
        let v = if rng.chance(90) {
            0
        } else {
            rng.pick(&[1, any])
        };
        p.set(r0 + 4 * k, v);
    }
    for k in 0..4 {
        let v = rng.float();
        p.set(p.sym(VF00) + 4 * k, v);
    }
    for k in 0..12 {
        let v = rng.float();
        p.set(p.sym(ORIGIN_SRC) + 4 * k, v);
    }
    // s2: random, or passing the first flag test (s2 & 0x28514a == r0's
    // low word) with random 0x810 bits, or passing both when r0 is 0
    let s2 = match rng.below(4) {
        0 => rng.next(),
        1 => rng.next() & !0x0028_514a | r0_lo,
        _ => rng.next() & !0x0028_595a | r0_lo,
    };
    p.set(p.sym(S2), s2);
    // s1: its 0x11 bits equal to r0's most of the time (S3's last test)
    let s1 = rng.next();
    let s1 = if rng.chance(60) {
        s1 & !0x11 | r0_lo & 0x11
    } else {
        s1
    };
    p.set(p.sym(0x00c8_f618), s1);
    let sse2 = u32::from(rng.chance(70));
    p.set(p.sym(SSE2), sse2);
    let Case { shape, cmd, .. } = c;
    for k in 0..0x10 + 0xc * n {
        let h = if rng.chance(20) {
            rng.pick(&[0u16, 1, 0x7fff, 0x8000, 0xffff])
        } else {
            u16::try_from(rng.below(0x1_0000)).unwrap()
        };
        p.set(shape + 2 * k, h);
    }
    // the flag halves before each further point: r0's low half most of
    // the time (S6: +0xc; S3: +4, +8, +0xc)
    let flags: &[u32] = if vu0 { &[4, 8, 0xc] } else { &[0xc] };
    for k in 0..n {
        for &off in flags {
            if rng.chance(80) {
                let h = u16::try_from(r0_lo & 0xffff).unwrap();
                p.set(shape + 0x20 + 0x18 * k + off, h);
            }
        }
    }
    let b0: u8 = Mem::read(&p.a, shape);
    p.set(shape, if vu0 { b0 | 8 } else { b0 & !8 });
    for k in 0..8 {
        let v = rng.next();
        p.set(cmd + 4 * k, v);
    }
    let flags = u16::try_from(rng.below(0x1_0000)).unwrap() & !0xc00;
    p.set(cmd + 2, flags);
    if !tex {
        p.set(cmd + 0x1c, 0u32);
    }
}

/// A vertex count (`i16`) and the records the loop then reads: one pass
/// for 0 and negative counts (the loop decrements first). Not 0x8000: it
/// wraps to 0x7fff and runs 32768 passes.
fn count(rng: &mut Xorshift) -> (u16, u32) {
    match rng.below(10) {
        0 => (0, 1),
        1 => (rng.pick(&[0x8001, 0xffff, 0xfff0]), 1),
        _ => {
            let n = 1 + rng.below(MAX_RECORDS);
            (u16::try_from(n).unwrap(), n)
        }
    }
}

/// A loop entered at `entry` through [`Pair::loop_thunk`] with the frame
/// words as the head leaves them, against `port`.
type Setup = fn(&mut Pair, &mut Xorshift, Case, u32);

fn plain_setup(p: &mut Pair, rng: &mut Xorshift, c: Case, n: u32) {
    plain_case(p, rng, c, n, false);
}

fn vu0_setup(p: &mut Pair, rng: &mut Xorshift, c: Case, n: u32) {
    plain_case(p, rng, c, n, true);
}

fn loop_test(
    entry: u32,
    setup: Setup,
    seed: u32,
    name: &str,
    port: fn(&mut Oracle, Frame, u32, u32),
) {
    for mut p in Pair::all() {
        let mut rng = Xorshift(seed);
        let thunk = p.loop_thunk(entry);
        let c = Case::new(&mut p);
        let ebp = p.ebp(2);
        for case in 0..600 {
            let (n, records) = count(&mut rng);
            setup(&mut p, &mut rng, c, records);
            p.set(p.sym(COUNT), n);
            p.set(p.sym(SHIFT), rng.pick(&[0u32, 0, 2, 0x10]));
            p.fill_frame(ebp, &mut rng);
            let cursor = rng.next();
            p.set(ebp - 0x1bbc, cursor);
            p.set(ebp - 0x1bb8, c.cmd);
            p.set(ebp - 0x1bb4, c.shape);
            p.set(ebp - 0x1bb0, c.shape + 0x20);
            let what = format!("{name} case {case}");
            let r =
                p.a.call(thunk, Conv::Cdecl, &[c.shape, c.cmd])
                    .unwrap_or_else(|e| panic!("{}: {what}: {e}", p.name));
            assert!(r.st0.is_none(), "{}: {what}: ST0 left set", p.name);
            let frame = Frame { ebp };
            port(&mut p.b, frame, c.cmd, c.shape + 0x20);
            let mine = shape::tail(&p.b, frame);
            assert_eq!(r.eax, mine, "{}: {what}: result", p.name);
            p.same(&what, ebp);
        }
    }
}

/// The whole function on the selection `setup` makes.
fn whole_test(setup: Setup, seed: u32, name: &str) {
    for mut p in Pair::all() {
        let mut rng = Xorshift(seed);
        let ebp = p.ebp(2);
        let draw = p.sym(0x007e_d0c0);
        let c = Case::new(&mut p);
        for case in 0..400 {
            let (n, records) = count(&mut rng);
            setup(&mut p, &mut rng, c, records);
            let shift = rng.pick(&[0u32, 0, 4, 0x10]);
            p.set(p.sym(SHIFT), shift);
            p.set(c.shape + shift + 2, n);
            p.fill_frame(ebp, &mut rng);
            let what = format!("{name} case {case}");
            let r =
                p.a.call(draw, Conv::Cdecl, &[c.shape, c.cmd])
                    .unwrap_or_else(|e| panic!("{}: {what}: {e}", p.name));
            assert!(r.st0.is_none(), "{}: {what}: ST0 left set", p.name);
            let mine = shape::op_shape_draw(&mut p.b, c.shape, c.cmd, ebp);
            assert_eq!(Some(r.eax), mine, "{}: {what}: result", p.name);
            p.same(&what, ebp);
        }
    }
}

#[test]
fn shape_s6_matches_the_original() {
    loop_test(0x007f_3f03, plain_setup, 0x007f_3f03, "S6", shape::plain);
}

#[test]
fn shape_whole_s6_matches_the_original() {
    whole_test(plain_setup, 0x007e_d0c0, "whole S6");
}

#[test]
fn shape_s3_matches_the_original() {
    loop_test(
        0x007f_0a18,
        vu0_setup,
        0x007f_0a18,
        "S3",
        |m, _, cmd, first| {
            shape::vu0_plain(m, cmd, first);
        },
    );
}

#[test]
fn shape_whole_s3_matches_the_original() {
    whole_test(vu0_setup, 0x007e_d0c1, "whole S3");
}

#[test]
fn shape_s5_matches_the_original() {
    loop_test(
        0x007f_2dae,
        textured_case,
        0x007f_2dae,
        "S5",
        shape::textured,
    );
}

#[test]
fn shape_whole_s5_matches_the_original() {
    whole_test(textured_case, 0x007e_d0c2, "whole S5");
}

#[test]
fn shape_s2_matches_the_original() {
    loop_test(
        0x007e_ed53,
        vu0_textured_case,
        0x007e_ed53,
        "S2",
        shape::vu0_textured,
    );
}

#[test]
fn shape_whole_s2_matches_the_original() {
    whole_test(vu0_textured_case, 0x007e_d0c3, "whole S2");
}

#[test]
fn shape_s4_matches_the_original() {
    loop_test(0x007f_1a8b, fog_case, 0x007f_1a8b, "S4", shape::fog);
}

#[test]
fn shape_whole_s4_matches_the_original() {
    whole_test(fog_case, 0x007e_d0c4, "whole S4");
}

/// Each loop's first instruction after the head's count store, in
/// [`shape::Variant`] order (S1..S6).
const LOOPS: [(u32, shape::Variant); 6] = [
    (0x007e_d1d2, shape::Variant::Vu0Clip),
    (0x007e_ed53, shape::Variant::Vu0Textured),
    (0x007f_0a18, shape::Variant::Vu0Plain),
    (0x007f_1a8b, shape::Variant::Clip),
    (0x007f_2dae, shape::Variant::Textured),
    (0x007f_3f03, shape::Variant::Plain),
];

/// The head on every selection: each loop's start patched (in both
/// oracles, run in the original's) to store its number in a cell and jump
/// to the tail. The Rust head's selection, frame words, origin block, count
/// and shift and the tail's result must agree.
#[test]
fn shape_head_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x007e_d19e);
        let cell = p.alloc(4);
        let tail = p.sym(0x007f_499c);
        for (k, (at, _)) in (0u32..).zip(LOOPS) {
            let at = p.sym(at);
            let mut c = vec![0xc7, 0x05]; // mov dword [cell], k
            c.extend(cell.to_le_bytes());
            c.extend(k.to_le_bytes());
            c.push(0xe9); // jmp tail
            c.extend(tail.wrapping_sub(at + 15).to_le_bytes());
            p.code(at, &c);
        }
        let ebp = p.ebp(2);
        let draw = p.sym(0x007e_d0c0);
        let c = Case::new(&mut p);
        let mut seen = [0u32; 6];
        for case in 0..600 {
            for k in 0..8 {
                let v = rng.next();
                p.set(c.cmd + 4 * k, v);
                let v = rng.next();
                p.set(c.shape + 4 * k, v);
            }
            if rng.chance(40) {
                p.set(c.cmd + 0x1c, 0u32);
            }
            if rng.chance(40) {
                let flags = u16::try_from(rng.below(0x1_0000)).unwrap() & !0xc00;
                p.set(c.cmd + 2, flags);
            }
            if rng.chance(30) {
                p.set(c.cmd + 0x14, rng.pick(&[0u32, 0xffff_fff8, 0x10]));
            }
            for k in 0..12 {
                let v = rng.float();
                p.set(p.sym(ORIGIN_SRC) + 4 * k, v);
            }
            let shift = rng.pick(&[0u32, 2, 4, 0x10]);
            p.set(p.sym(SHIFT), shift);
            let t1 = rng.next();
            let t1 = rng.pick(&[0u32, 1, 0x1_0000, t1]);
            p.set(p.sym(0x00c8_f598), t1);
            p.fill_frame(ebp, &mut rng);
            let what = format!("head case {case}");
            let r =
                p.a.call(draw, Conv::Cdecl, &[c.shape, c.cmd])
                    .unwrap_or_else(|e| panic!("{}: {what}: {e}", p.name));
            let frame = Frame { ebp };
            let v = shape::head(&mut p.b, c.shape, c.cmd, frame);
            let k = (0u32..).zip(LOOPS).find(|(_, (_, l))| *l == v).unwrap().0;
            seen[k as usize] += 1;
            Mem::write(&mut p.b, cell, k);
            assert_eq!(
                r.eax,
                shape::tail(&p.b, frame),
                "{}: {what}: result",
                p.name
            );
            let shape_word: u32 = Mem::read(&p.b, ebp - 0x1bb4);
            assert_eq!(
                shape_word,
                Mem::read::<u32>(&p.a, ebp - 0x1bb4),
                "{}: {what}: shape word",
                p.name
            );
            p.same(&what, ebp);
        }
        assert!(
            seen.iter().all(|&n| n > 20),
            "{}: selections {seen:?}",
            p.name
        );
    }
}
