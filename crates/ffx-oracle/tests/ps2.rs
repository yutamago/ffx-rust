//! The PS2 emulation layer (`ffx_ps2`: the `FFXVu0` vector and matrix
//! library, `Matrix::copy`, the emulated registers' setters) against the
//! original, on two oracles started alike: one runs `FFX.exe`'s code, the
//! other the Rust bridge on the same memory; results (the `eax` the
//! "void" originals leave too) and every written byte must agree, floats
//! to the bit. Synthetic cases need `FFX_GAME_DIR`; the snapshot cases also
//! `FFX_SNAPSHOT` (both kept snapshots). NaNs here are quiet: the oracle's
//! x87 keeps loaded signalling NaNs signalling (`docs/porting.md`).

// a, b: the two oracles; m, v, x, y: matrix and vectors
#![allow(clippy::many_single_char_names)]

use std::path::PathBuf;

use ffx_bridge::{Global, Mem, Value};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_ps2::{ee::bridge as ee, vu0::bridge as vu0};

/// The game thread's x87 control word.
const GAME_FPU: u32 = 0x027f;

fn exe() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    Some(dir?.join("FFX.exe"))
}

fn synthetic() -> Option<(Oracle, Oracle)> {
    let exe = exe()?;
    let mut a = Oracle::load(&exe).unwrap();
    let mut b = Oracle::load(&exe).unwrap();
    a.set_fpu_control(GAME_FPU).unwrap();
    b.set_fpu_control(GAME_FPU).unwrap();
    Some((a, b))
}

fn snapshot() -> Option<(Oracle, Oracle)> {
    let exe = exe()?;
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; skipping");
        return None;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    Some((
        Oracle::from_snapshot(&exe, &pages).unwrap(),
        Oracle::from_snapshot(&exe, &pages).unwrap(),
    ))
}

struct Xorshift(u32);

impl Xorshift {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    /// Ordinary floats of mixed magnitudes, special values, NaNs (quiet).
    fn float(&mut self) -> f32 {
        const SPECIAL: [u32; 9] = [
            0,
            0x8000_0000,
            0x7f80_0000,
            0xff80_0000,
            0x7fc0_0000,
            0xffc1_2345,
            1,
            0x7f7f_ffff,
            0x3f80_0000,
        ];
        let bits = match self.next() % 12 {
            0 => self.next(),
            1 => SPECIAL[(self.next() % 9) as usize],
            2 => self.next() | 0x7fc0_0000,
            _ => self.next() & 0x807f_ffff | (self.next() % 40 + 107) << 23,
        };
        let f = f32::from_bits(bits);
        if f.is_nan() {
            f32::from_bits(bits | 0x0040_0000)
        } else {
            f
        }
    }
}

fn eax(o: &mut Oracle, sym: u32, conv: Conv, args: &[u32]) -> u32 {
    let at = o.rebase(sym);
    o.call(at, conv, args)
        .unwrap_or_else(|e| panic!("{sym:#010x}: {e}"))
        .eax
}

fn cdecl(o: &mut Oracle, sym: u32, args: &[u32]) -> u32 {
    eax(o, sym, Conv::Cdecl, args)
}

fn alloc(a: &mut Oracle, b: &mut Oracle, len: usize) -> u32 {
    let x = a.alloc(len).unwrap();
    assert_eq!(b.alloc(len).unwrap(), x);
    x
}

fn put<T: Value>(a: &mut Oracle, b: &mut Oracle, addr: u32, value: T) {
    Mem::write(a, addr, value);
    Mem::write(b, addr, value);
}

fn set<T: Value>(a: &mut Oracle, b: &mut Oracle, g: Global<T>, value: T) {
    g.set(a, value);
    g.set(b, value);
}

fn words(m: &Oracle, addr: u32, n: u32) -> Vec<u32> {
    (0..n).map(|i| Mem::read::<u32>(m, addr + 4 * i)).collect()
}

fn same_memory(what: &str, a: &mut Oracle, b: &mut Oracle) {
    assert_eq!(a.diff(b).unwrap(), [], "{what}: memory");
}

/// Fills `n` floats at `at` in both.
fn fill(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, at: u32, n: u32) {
    for i in 0..n {
        put(a, b, at + 4 * i, rng.float());
    }
}

const COPY_VECTOR: u32 = 0x006e_d370;
const COPY_VECTOR_XYZ: u32 = 0x0062_fca0;
const ADD_VECTOR: u32 = 0x006e_d290;
const SUB_VECTOR: u32 = 0x006e_d940;
const MUL_VECTOR: u32 = 0x006e_d5d0;
const SCALE_VECTOR: u32 = 0x006e_d8f0;
const DIV_VECTOR: u32 = 0x006e_d3a0;
const DIV_VECTOR_XYZ: u32 = 0x006e_d420;
const INNER_PRODUCT: u32 = 0x0062_fce0;
const OUTER_PRODUCT: u32 = 0x006e_d720;
const INTER_VECTOR: u32 = 0x0062_fd10;
const INTER_VECTOR_XYZ: u32 = 0x0062_fd70;

/// A Rust vector function `f(dst, a, b)`.
type Three = fn(&mut Oracle, u32, u32, u32) -> u32;
/// A Rust vector function `f(dst, v, s)`.
type Scalar = fn(&mut Oracle, u32, u32, f32) -> u32;

/// Where the destination goes in a block `[dst 16][a 16][b 16]`: apart,
/// on `a`, on `b`, or a word into `a`.
fn dst_at(block: u32, n: u32) -> u32 {
    block
        + match n % 5 {
            0 | 1 => 0,
            2 => 16,
            3 => 32,
            _ => 20,
        }
}

#[test]
fn vectors_match_the_original_bit_for_bit() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let (a, b) = (&mut a, &mut b);
    let mut rng = Xorshift(0x0ff0_7e00);
    let block = alloc(a, b, 64);
    let three: [(&str, u32, Three); 7] = [
        ("CopyVector", COPY_VECTOR, |m, d, x, _| {
            vu0::copy_vector(m, d, x)
        }),
        ("CopyVectorXYZ", COPY_VECTOR_XYZ, |m, d, x, _| {
            vu0::copy_vector_xyz(m, d, x)
        }),
        ("AddVector", ADD_VECTOR, vu0::add_vector),
        ("SubVector", SUB_VECTOR, vu0::sub_vector),
        ("MulVector", MUL_VECTOR, vu0::mul_vector),
        ("OuterProduct", OUTER_PRODUCT, vu0::outer_product),
        ("InnerProduct", INNER_PRODUCT, |_, _, _, _| 0),
    ];
    for (name, sym, rust) in three {
        for n in 0..4000 {
            fill(a, b, &mut rng, block, 12);
            let (dst, x, y) = (dst_at(block, n), block + 16, block + 32);
            if sym == INNER_PRODUCT {
                let r = a.call(a.rebase(sym), Conv::Cdecl, &[x, y]).unwrap();
                let rb = vu0::inner_product(b, x, y);
                assert_eq!(
                    st0_f32(&r.st0_raw.unwrap()),
                    rb.to_bits(),
                    "{name} case {n}"
                );
                continue;
            }
            let args: &[u32] = if sym == COPY_VECTOR || sym == COPY_VECTOR_XYZ {
                &[dst, x]
            } else {
                &[dst, x, y]
            };
            let ra = cdecl(a, sym, args);
            let rb = rust(b, dst, x, y);
            assert_eq!(ra, rb, "{name} case {n}: eax");
            assert_eq!(words(a, block, 12), words(b, block, 12), "{name} case {n}");
        }
        same_memory(name, a, b);
    }
    let scalar: [(&str, u32, Scalar); 3] = [
        ("ScaleVector", SCALE_VECTOR, vu0::scale_vector),
        ("DivVector", DIV_VECTOR, vu0::div_vector),
        ("DivVectorXYZ", DIV_VECTOR_XYZ, vu0::div_vector_xyz),
    ];
    for (name, sym, rust) in scalar {
        for n in 0..4000 {
            fill(a, b, &mut rng, block, 12);
            let s = match n % 7 {
                0 => 0.0,
                1 => -0.0,
                _ => rng.float(),
            };
            let (dst, v) = (dst_at(block, n), block + 16);
            let ra = cdecl(a, sym, &[dst, v, s.to_bits()]);
            let rb = rust(b, dst, v, s);
            assert_eq!(ra, rb, "{name} case {n}: eax");
            assert_eq!(
                words(a, block, 12),
                words(b, block, 12),
                "{name} case {n} s {s}"
            );
        }
        same_memory(name, a, b);
    }
    for (name, sym) in [
        ("InterVector", INTER_VECTOR),
        ("InterVectorXYZ", INTER_VECTOR_XYZ),
    ] {
        for n in 0..4000 {
            fill(a, b, &mut rng, block, 12);
            let t = if n % 5 == 0 {
                f32::from(u8::try_from(rng.next() % 5).unwrap()) / 4.0
            } else {
                rng.float()
            };
            let (dst, x, y) = (dst_at(block, n), block + 16, block + 32);
            let ra = cdecl(a, sym, &[dst, x, y, t.to_bits()]);
            let rb = if sym == INTER_VECTOR {
                vu0::inter_vector(b, dst, x, y, t)
            } else {
                vu0::inter_vector_xyz(b, dst, x, y, t)
            };
            assert_eq!(ra, rb, "{name} case {n}: eax");
            assert_eq!(
                words(a, block, 12),
                words(b, block, 12),
                "{name} case {n} t {t}"
            );
        }
        same_memory(name, a, b);
    }
}

/// An `f32` result from `ST0`'s 80 bits (exact: it was loaded from one).
fn st0_f32(raw: &[u8; 10]) -> u32 {
    let mant = u64::from_le_bytes(raw[..8].try_into().unwrap());
    let se = u16::from_le_bytes([raw[8], raw[9]]);
    let sign = u32::from(se >> 15) << 31;
    let exp = i32::from(se & 0x7fff);
    if exp == 0x7fff {
        #[allow(clippy::cast_possible_truncation)] // the top 23 fraction bits
        let frac = ((mant >> 40) & 0x7f_ffff) as u32;
        return sign | 0x7f80_0000 | frac;
    }
    if exp == 0 && mant == 0 {
        return sign;
    }
    #[allow(clippy::cast_precision_loss)] // f32 values: exact
    let v = mant as f64 / (1u64 << 63) as f64 * 2f64.powi(exp - 16383);
    #[allow(clippy::cast_possible_truncation)] // exact
    let f = v as f32;
    sign | f.to_bits() & 0x7fff_ffff
}

const MATRIX_IDENTITY: u32 = 0x006e_ce60;
const UNIT_MATRIX: u32 = 0x006e_d9a0;
const MATRIX_COPY: u32 = 0x006e_cc40;
const COPY_MATRIX: u32 = 0x006e_d360;
const APPLY_MATRIX: u32 = 0x006e_d2e0;
const ROT_TRANS_PERS: u32 = 0x006e_d840;
const LIGHT_COLOR_MATRIX: u32 = 0x006e_d4f0;
const NORMAL_LIGHT_MATRIX: u32 = 0x006e_d620;

fn matrices(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    // [dst 64][m 64][v 16][rows 64]
    let block = alloc(a, b, 0xd0);
    let (m, v, rows) = (block + 0x40, block + 0x80, block + 0x90);
    for n in 0..1500u32 {
        fill(a, b, rng, block, 0xd0 / 4);
        let dst = block + if n % 4 == 3 { 0x40 } else { 0 };
        let ra = eax(a, MATRIX_IDENTITY, Conv::Thiscall(dst), &[]);
        assert_eq!(ra, vu0::matrix_identity(b, dst));
        let ra = cdecl(a, UNIT_MATRIX, &[m]);
        assert_eq!(ra, vu0::matrix_identity(b, m));
        fill(a, b, rng, block, 0xd0 / 4);
        let ra = eax(a, MATRIX_COPY, Conv::Thiscall(dst), &[m + 4 * (n % 3)]);
        assert_eq!(ra, vu0::matrix_copy(b, dst, m + 4 * (n % 3)));
        let ra = cdecl(a, COPY_MATRIX, &[dst, rows]);
        assert_eq!(ra, vu0::matrix_copy(b, dst, rows));
        fill(a, b, rng, block, 0xd0 / 4);
        let out = block + if n % 3 == 0 { 0x80 } else { 0 };
        assert_eq!(
            cdecl(a, APPLY_MATRIX, &[out, m, v]),
            vu0::apply_matrix(b, out, m, v),
            "ApplyMatrix {n}"
        );
        let keep = n % 2;
        assert_eq!(
            cdecl(a, ROT_TRANS_PERS, &[out, m, v, keep]),
            vu0::rot_trans_pers(b, out, m, v, keep),
            "RotTransPers {n}"
        );
        let r = [
            rows,
            rows + 16,
            rows + 32,
            if n % 5 == 0 { block } else { rows + 48 },
        ];
        let ra = cdecl(a, LIGHT_COLOR_MATRIX, &[block, r[0], r[1], r[2], r[3]]);
        assert_eq!(ra, vu0::light_color_matrix(b, block, r));
        let ra = cdecl(a, NORMAL_LIGHT_MATRIX, &[block, r[0], r[1], r[3]]);
        assert_eq!(ra, vu0::normal_light_matrix(b, block, [r[0], r[1], r[3]]));
        assert_eq!(
            words(a, block, 0xd0 / 4),
            words(b, block, 0xd0 / 4),
            "matrices {n}"
        );
    }
    same_memory("matrices", a, b);
}

const SET_FUNC_PARAM: [u32; 4] = [0x0042_eec0, 0x0042_eed0, 0x0042_eee0, 0x0042_eef0];
const RESTORE_VF00: u32 = 0x0042_ef00;
const SET_ZS_FLAG: u32 = 0x0042_ef30;
const SET_SS_FLAG: u32 = 0x0042_ef20;

fn registers(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    assert_eq!(cdecl(a, 0x0083_5b90, &[]), vu0::clip_screen(b));
    for (n, &sym) in (0..).zip(&SET_FUNC_PARAM) {
        let v = rng.next();
        assert_eq!(cdecl(a, sym, &[v]), ee::set_func_param(b, n, v));
    }
    set(a, b, ee::VF00, [rng.float(); 4]);
    cdecl(a, RESTORE_VF00, &[]);
    ee::restore_vf00(b);
    for start in [0, 0x40, 0x80, rng.next()] {
        set(a, b, ee::STATUS, start);
        cdecl(a, SET_ZS_FLAG, &[]);
        ee::set_zs_flag(b);
        cdecl(a, SET_SS_FLAG, &[]);
        ee::set_ss_flag(b);
    }
    same_memory("registers", a, b);
}

#[test]
fn matrices_and_registers_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x0ff0_7e01);
    matrices(&mut a, &mut b, &mut rng);
    registers(&mut a, &mut b, &mut rng);
}

#[test]
fn matrices_and_registers_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x0ff0_7e02);
    matrices(&mut a, &mut b, &mut rng);
    registers(&mut a, &mut b, &mut rng);
}

const NORMALIZE: u32 = 0x006e_d6f0;
const SAFE_NORMALIZE3: u32 = 0x0070_4200;
const CAN_NORMALIZE3: u32 = 0x0070_3bc0;

/// Free image bytes after `.text` (mapped, executable) for the C runtime
/// stand-ins below.
const TEST_CODE: u32 = 0x00b0_bb00;

/// Stand-ins for the C runtime imports the oracle lacks: `_isnan` (1 for
/// a NaN) and `_finite` at their import slots, `_CIsqrt`'s thunk as the
/// CPU's square root (`fsqrt; ret`).
fn patch_crt_math(o: &mut Oracle) {
    #[rustfmt::skip]
    let isnan: [u8; 34] = [
        0x8b, 0x44, 0x24, 0x08,             // mov eax, [esp+8]
        0x25, 0xff, 0xff, 0xff, 0x7f,       // and eax, 0x7fffffff
        0x3d, 0x00, 0x00, 0xf0, 0x7f,       // cmp eax, 0x7ff00000
        0x77, 0x0c,                         // ja nan
        0x72, 0x07,                         // jb num
        0x83, 0x7c, 0x24, 0x04, 0x00,       // cmp dword [esp+4], 0
        0x75, 0x03,                         // jne nan
        0x31, 0xc0, 0xc3,                   // num: xor eax, eax; ret
        0xb8, 0x01, 0x00, 0x00, 0x00, 0xc3, // nan: mov eax, 1; ret
    ];
    #[rustfmt::skip]
    let finite: [u8; 25] = [
        0x8b, 0x44, 0x24, 0x08,             // mov eax, [esp+8]
        0x25, 0x00, 0x00, 0xf0, 0x7f,       // and eax, 0x7ff00000
        0x3d, 0x00, 0x00, 0xf0, 0x7f,       // cmp eax, 0x7ff00000
        0x0f, 0x95, 0xc0,                   // setne al
        0x0f, 0xb6, 0xc0,                   // movzx eax, al
        0xc3, 0xcc, 0xcc, 0xcc, 0xcc,
    ];
    let code = o.rebase(TEST_CODE);
    o.write(code, &isnan).unwrap();
    o.write(code + 0x40, &finite).unwrap();
    Mem::write(o, o.rebase(0x00b0_c478), code);
    Mem::write(o, o.rebase(0x00b0_c474), code + 0x40);
    let sqrt = o.rebase(0x0094_97b8);
    o.write(sqrt, &[0xd9, 0xfa, 0xc3]).unwrap();
}

/// A vector whose square sum lands anywhere, near `FLT_MIN` included.
fn short_vector(rng: &mut Xorshift) -> [f32; 4] {
    let e = rng.next() % 3;
    std::array::from_fn(|_| match rng.next() % 8 {
        0 => rng.float(),
        1 => 0.0,
        2 if e == 0 => f32::from_bits(rng.next() & 0x807f_ffff | (rng.next() % 12 + 58) << 23),
        _ => f32::from_bits(rng.next() & 0x807f_ffff | (rng.next() % 30 + 112) << 23),
    })
}

fn normalize(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    patch_crt_math(a);
    patch_crt_math(b);
    // [dst 16][src 16]
    let block = alloc(a, b, 32);
    for n in 0..6000u32 {
        let v = short_vector(rng);
        put(a, b, block + 16, v);
        fill(a, b, rng, block, 4);
        let ra = eax(a, CAN_NORMALIZE3, Conv::Thiscall(block + 16), &[]);
        let rb = ffx_system::math::bridge::can_normalize3(b, block + 16);
        assert_eq!(ra, rb, "CanNormalize3 {n} {v:?}");
        let dst = if n % 4 == 0 { block + 16 } else { block };
        cdecl(a, NORMALIZE, &[dst, block + 16]);
        vu0::normalize(b, dst, block + 16);
        assert_eq!(
            words(a, block, 8),
            words(b, block, 8),
            "Normalize {n} {v:?}"
        );
        put(a, b, block, short_vector(rng));
        eax(a, SAFE_NORMALIZE3, Conv::Fastcall, &[block]);
        ffx_system::math::bridge::safe_normalize3(b, block);
        assert_eq!(words(a, block, 4), words(b, block, 4), "SafeNormalize3 {n}");
    }
    same_memory("normalize", a, b);
}

#[test]
fn normalize_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    normalize(&mut a, &mut b, &mut Xorshift(0x0ff0_7e03));
}

#[test]
fn normalize_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    normalize(&mut a, &mut b, &mut Xorshift(0x0ff0_7e04));
}

const INVERS_MATRIX: u32 = 0x006e_d460;
const INVERSE4X4: u32 = 0x0070_4dd0;

/// A matrix: random elements, an invertible transform, a zero row, two
/// equal rows (determinant 0) or ±0s.
fn any_matrix(rng: &mut Xorshift) -> [f32; 16] {
    let mut m: [f32; 16] = std::array::from_fn(|_| rng.float());
    match rng.next() % 6 {
        0 => {
            let r = (rng.next() % 4) as usize * 4;
            for e in &mut m[r..r + 4] {
                *e = if rng.next().is_multiple_of(2) {
                    0.0
                } else {
                    -0.0
                };
            }
        }
        1 => {
            let (r, s) = ((rng.next() % 4) as usize * 4, (rng.next() % 4) as usize * 4);
            let row: [f32; 4] = m[r..r + 4].try_into().unwrap();
            m[s..s + 4].copy_from_slice(&row);
        }
        2 | 3 => {
            for (k, e) in m.iter_mut().enumerate() {
                let small = f32::from(u8::try_from(rng.next() % 200).unwrap()) / 50.0 - 2.0;
                *e = if k % 5 == 0 { 1.0 + small } else { small };
            }
            m[3] = 0.0;
            m[7] = 0.0;
            m[11] = 0.0;
            m[15] = 1.0;
        }
        _ => {}
    }
    m
}

fn inverses(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    // [dst 64][m 64]
    let block = alloc(a, b, 0x80);
    for n in 0..6000u32 {
        fill(a, b, rng, block, 16);
        put(a, b, block + 0x40, any_matrix(rng));
        let dst = block + if n % 4 == 0 { 0x40 } else { 0 };
        let (sym, rb) = if n % 3 == 0 {
            (
                INVERSE4X4,
                ffx_system::math::bridge::inverse4x4(b, dst, block + 0x40),
            )
        } else {
            (
                INVERS_MATRIX,
                ffx_system::math::bridge::invers_matrix4x4(b, dst, block + 0x40),
            )
        };
        let ra = cdecl(a, sym, &[dst, block + 0x40]);
        assert_eq!(ra, rb, "inverse {n}: eax");
        assert_eq!(words(a, block, 32), words(b, block, 32), "inverse {n}");
    }
    same_memory("inverses", a, b);
}

#[test]
fn inverses_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    inverses(&mut a, &mut b, &mut Xorshift(0x0ff0_7e05));
}

#[test]
fn inverses_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    inverses(&mut a, &mut b, &mut Xorshift(0x0ff0_7e06));
}

const ROT_MATRIX: [(u32, ffx_ps2::vu0::Axis); 3] = [
    (0x006e_d780, ffx_ps2::vu0::Axis::X),
    (0x006e_d7c0, ffx_ps2::vu0::Axis::Y),
    (0x006e_d800, ffx_ps2::vu0::Axis::Z),
];
const CDC_ROT_Y: u32 = 0x0070_5fa0;

/// `_CIcos` and `_CIsin` (imports the oracle lacks) replaced at their
/// thunks by exact stand-ins, `x²` and `2x` (`fmul st, st; ret` and `fadd
/// st, st; ret`): the bridges get the same through their callback, so the
/// angle's path and the matrix arithmetic are checked; the C runtime's own
/// sine and cosine are the game's in both.
fn patch_trig(o: &mut Oracle) {
    let cos = o.rebase(0x0094_98c2);
    o.write(cos, &[0xd8, 0xc8, 0xc3]).unwrap();
    let sin = o.rebase(0x0094_98c8);
    o.write(sin, &[0xd8, 0xc0, 0xc3]).unwrap();
}

fn stand_in(t: ffx_ps2::vu0::Trig, x: f32) -> f32 {
    use ffx_bridge::x87::Reg;
    match t {
        ffx_ps2::vu0::Trig::Cos => (Reg::load(x) * Reg::load(x)).store(),
        ffx_ps2::vu0::Trig::Sin => (Reg::load(x) + Reg::load(x)).store(),
    }
}

fn rotations(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    patch_trig(a);
    patch_trig(b);
    // [dst 64][src 64]
    let block = alloc(a, b, 0x80);
    for n in 0..3000u32 {
        fill(a, b, rng, block, 32);
        let angle = if n % 3 == 0 {
            f32::from(u8::try_from(rng.next() % 64).unwrap()) / 8.0 - 4.0
        } else {
            rng.float()
        };
        let (sym, axis) = ROT_MATRIX[(n % 3) as usize];
        let dst = block + if n % 5 == 0 { 0x40 } else { 0 };
        let ra = cdecl(a, sym, &[dst, block + 0x40, angle.to_bits()]);
        let rb = vu0::rot_matrix(b, axis, dst, block + 0x40, angle, &mut stand_in);
        assert_eq!(ra, rb, "RotMatrix {axis:?} {n}: eax");
        assert_eq!(
            words(a, block, 32),
            words(b, block, 32),
            "RotMatrix {axis:?} {n}"
        );
        let (s, c) = (rng.float(), rng.float());
        let ra = eax(
            a,
            CDC_ROT_Y,
            Conv::Thiscall(block),
            &[s.to_bits(), c.to_bits()],
        );
        assert_eq!(ra, ffx_system::math::bridge::rotate_y(b, block, s, c));
        assert_eq!(words(a, block, 32), words(b, block, 32), "RotY {n}");
    }
    same_memory("rotations", a, b);
}

#[test]
fn rotations_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    rotations(&mut a, &mut b, &mut Xorshift(0x0ff0_7e07));
}

#[test]
fn rotations_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    rotations(&mut a, &mut b, &mut Xorshift(0x0ff0_7e08));
}
