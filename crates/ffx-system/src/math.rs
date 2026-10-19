//! The `cdc` matrix helpers and the VU0 vector length (`cdc::Mul4x4` and
//! friends, `sceVu0VectorLengthSquared`), with the game's x87 rounding
//! ([`ffx_bridge::x87`]).
//!
//! Matrices are 16 `f32`s, row-major, rows `[x, y, z, w]`: a point is a row
//! vector multiplied from the left, the translation is the fourth row.
//! Every element passes through the x87 stack, so a signalling NaN comes out
//! quiet even where an element is only copied.

use ffx_bridge::x87::{self, Reg};

fn load(x: f32) -> Reg {
    Reg::load(x)
}

/// A 4×4 matrix, row-major.
pub type Matrix = [f32; 16];

/// `((a0 b0 + a1 b1) + a2 b2)`, the first three terms of a dot product in
/// the order the game adds them.
fn dot3(a: [f32; 3], b: [f32; 3]) -> Reg {
    load(a[0]) * load(b[0]) + load(a[1]) * load(b[1]) + load(a[2]) * load(b[2])
}

/// Component `i` of `v × m` (`cdc_Mul4x4(dst, m, v)`): `col` is column `i`
/// of `m`, `v` the vector as the original copied it. The last component
/// takes its last two matrix elements as memory operands.
#[must_use]
pub fn mul4x4_component(i: u32, v: [f32; 4], col: [f32; 4]) -> f32 {
    let r = if i == 3 {
        load(v[0]) * load(col[0])
            + load(v[1]) * load(col[1])
            + load(v[2]).mul_m32(col[2])
            + load(v[3]).mul_m32(col[3])
    } else {
        dot3([v[0], v[1], v[2]], [col[0], col[1], col[2]]) + load(v[3]) * load(col[3])
    };
    r.store()
}

/// Component `i < 3` of `cdc_Mul3x4(dst, m, v)`: the point `v` (w = 1)
/// transformed; `col` is column `i` of `m`, translation last. The sum of
/// the products is stored as an `f32` before the translation (a memory
/// operand) is added; component 2 takes `m[10]` as a memory operand.
#[must_use]
pub fn mul3x4_component(i: u32, v: [f32; 3], col: [f32; 4]) -> f32 {
    let r = if i == 2 {
        load(v[0]) * load(col[0]) + load(v[1]) * load(col[1]) + load(v[2]).mul_m32(col[2])
    } else {
        dot3(v, [col[0], col[1], col[2]])
    };
    load(r.store()).add_m32(col[3]).store()
}

/// For element `k` of `cdc_Mul4x4_007059b0`, bit `t` set when term `t`'s
/// `a` factor is the memory operand (else `b`'s is): how the compiled
/// code happens to load them, which decides which NaN a product of two
/// gives.
const MUL4X4_A_IN_MEMORY: [u8; 16] = [
    0b0000, 0b1000, 0b0000, 0b1000, 0b0011, 0b1011, 0b0000, 0b1011, 0b0011, 0b0011, 0b0001, 0b0011,
    0b0111, 0b1011, 0b0100, 0b1011,
];

/// `cdc_Mul4x4_007059b0(dst, a, b)`: `b × a`. Each element's first three
/// products are summed and stored as an `f32` before the fourth is added.
#[must_use]
pub fn mul4x4(a: &Matrix, b: &Matrix) -> Matrix {
    std::array::from_fn(|k| {
        let (row, col) = (4 * (k / 4), k % 4);
        let term = |t: usize| {
            let (x, y) = (a[4 * t + col], b[row + t]);
            if MUL4X4_A_IN_MEMORY[k] >> t & 1 == 1 {
                load(y).mul_m32(x)
            } else {
                load(x).mul_m32(y)
            }
        };
        let first3 = (term(0) + term(1) + term(2)).store();
        (term(3) + load(first3)).store()
    })
}

/// `cdc_Transpose(dst, m)`.
#[must_use]
pub fn transpose(m: &Matrix) -> Matrix {
    std::array::from_fn(|k| x87::round_trip(m[(k % 4) * 4 + k / 4]))
}

/// `cdc_OrthonormalInverse3x4(dst, m)`: the inverse of a rotation plus
/// translation (the rotation transposed, the translation turned back), last
/// column `(0, 0, 0, 1)`.
#[must_use]
pub fn orthonormal_inverse3x4(m: &Matrix) -> Matrix {
    let t = [m[12], m[13], m[14]];
    let mut out = [0.0; 16];
    for i in 0..3 {
        out[12 + i] = (-dot3([m[4 * i], m[4 * i + 1], m[4 * i + 2]], t)).store();
        for j in 0..3 {
            out[4 * j + i] = x87::round_trip(m[4 * i + j]);
        }
    }
    out[15] = 1.0;
    out
}

/// One row of `cdc::Matrix::RotX` / `RotZ`: `(x, y)` turned by the angle
/// whose sine and cosine are `s` and `c`: `(x c - y s, x s + y c)`.
#[must_use]
pub fn rotate(x: f32, y: f32, s: f32, c: f32) -> (f32, f32) {
    let (x, y) = (x87::round_trip(x), x87::round_trip(y));
    let (s, c) = (load(s), load(c));
    (
        (load(x) * c - load(y) * s).store(),
        (load(x) * s + load(y) * c).store(),
    )
}

/// `cdc_Inverse4x4(dst, m)`: the inverse by cofactors, as compiled: `m`
/// copied (transposed) into locals, the 2×2 products stored as `f32`s,
/// each cofactor's positive half stored as an `f32` before the negative
/// half is subtracted, the determinant stored, `1 / det` stored (0 for a
/// zero determinant) and every cofactor multiplied by it. Generated from
/// the instructions (`0x00704de9`..`0x00705492`), one `let` per `fstp
/// dword`, named in order; the comment is the local it went to.
#[must_use]
#[allow(clippy::too_many_lines, clippy::similar_names)] // the original, store by store
pub fn inverse4x4(m: &Matrix) -> Matrix {
    let ld = load;
    let t1 = ld(m[0]).store(); // ebp-0x44
    let t2 = ld(m[1]).store(); // ebp-0x34
    let t3 = ld(m[2]).store(); // ebp-0x24
    let t4 = ld(m[3]).store(); // ebp-0x14
    let t5 = ld(m[4]).store(); // ebp-0x40
    let t6 = ld(m[5]).store(); // ebp-0x30
    let t7 = ld(m[6]).store(); // ebp-0x20
    let t8 = ld(m[7]).store(); // ebp-0x10
    let t9 = ld(m[8]).store(); // ebp-0x3c
    let t10 = ld(m[9]).store(); // ebp-0x2c
    let t11 = ld(m[10]).store(); // ebp-0x1c
    let t12 = ld(m[11]).store(); // ebp-0xc
    let t13 = ld(m[12]).store(); // ebp-0x38
    let t14 = ld(m[13]).store(); // ebp-0x28
    let t15 = ld(m[14]).store(); // ebp-0x18
    let t16 = ld(m[15]).store(); // ebp-0x8
    let t17 = (ld(t16) * ld(t11)).store(); // ebp-0xb8
    let t18 = (ld(t12) * ld(t15)).store(); // ebp-0xb4
    let t19 = (ld(t7) * ld(t16)).store(); // ebp-0xb0
    let t20 = (ld(t8) * ld(t15)).store(); // ebp-0xac
    let t21 = (ld(t7) * ld(t12)).store(); // ebp-0xa8
    let t22 = (ld(t11) * ld(t8)).store(); // ebp-0xa4
    let t23 = (ld(t16) * ld(t3)).store(); // ebp-0xa0
    let t24 = (ld(t15) * ld(t4)).store(); // ebp-0x9c
    let t25 = (ld(t12) * ld(t3)).store(); // ebp-0x98
    let t26 = ld(t4).mul_m32(t11).store(); // ebp-0x94
    let t27 = (ld(t8) * ld(t3)).store(); // ebp-0x90
    let t28 = (ld(t4) * ld(t7)).store(); // ebp-0x8c
    let t29 = (((ld(t6) * ld(t17)) + ld(t10).mul_m32(t20)) + ld(t14).mul_m32(t21)).store(); // ebp-0x88
    let t30 =
        (ld(t29) - ((ld(t14) * ld(t22)) + ((ld(t10) * ld(t19)) + (ld(t6) * ld(t18))))).store(); // ebp-0x84
    let t31 = (((ld(t10) * ld(t23)) + (ld(t18) * ld(t2))) + ld(t14).mul_m32(t26)).store(); // ebp-0x88
    let t32 = (((ld(t10) * ld(t24)) + (ld(t17) * ld(t2))) + ld(t14).mul_m32(t25))
        .subr_m32(t31)
        .store(); // ebp-0x80
    let t33 = ((ld(t14) * ld(t27)) + ((ld(t19) * ld(t2)) + (ld(t6) * ld(t24)))).store(); // ebp-0x88
    let t34 = (ld(t33) - ((ld(t14) * ld(t28)) + ((ld(t2) * ld(t20)) + (ld(t6) * ld(t23))))).store(); // ebp-0x7c
    let t35 = ((ld(t6).mul_m32(t25) + ld(t2).mul_m32(t22)) + (ld(t10) * ld(t28))).store(); // ebp-0x88
    let t36 =
        (ld(t35) - ((ld(t6).mul_m32(t26) + ld(t2).mul_m32(t21)) + (ld(t10) * ld(t27)))).store(); // ebp-0x78
    let t37 = ((ld(t5).mul_m32(t18) + ld(t9).mul_m32(t19)) + ld(t13).mul_m32(t22)).store(); // ebp-0x88
    let t38 =
        (ld(t37) - ((ld(t5).mul_m32(t17) + ld(t9).mul_m32(t20)) + ld(t13).mul_m32(t21))).store(); // ebp-0x74
    let t39 = ((ld(t1).mul_m32(t17) + (ld(t9) * ld(t24))) + ld(t13).mul_m32(t25)).store(); // ebp-0x88
    let t40 =
        (ld(t39) - ((ld(t1).mul_m32(t18) + (ld(t9) * ld(t23))) + ld(t13).mul_m32(t26))).store(); // ebp-0x70
    let t41 = (((ld(t23) * ld(t5)) + ld(t1).mul_m32(t20)) + (ld(t13) * ld(t28))).store(); // ebp-0x88
    let t42 =
        (ld(t41) - (((ld(t24) * ld(t5)) + ld(t1).mul_m32(t19)) + (ld(t13) * ld(t27)))).store(); // ebp-0x6c
    let t43 = ((ld(t27) * ld(t9)) + (ld(t5).mul_m32(t26) + ld(t1).mul_m32(t21))).store(); // ebp-0x88
    let t44 =
        (ld(t43) - ((ld(t28) * ld(t9)) + (ld(t5).mul_m32(t25) + ld(t1).mul_m32(t22)))).store(); // ebp-0x68
    let t45 = (ld(t9) * ld(t14)).store(); // ebp-0xb8
    let t46 = (ld(t13) * ld(t10)).store(); // ebp-0xb4
    let t47 = (ld(t5) * ld(t14)).store(); // ebp-0xb0
    let t48 = (ld(t13) * ld(t6)).store(); // ebp-0xac
    let t49 = (ld(t5) * ld(t10)).store(); // ebp-0xa8
    let t50 = (ld(t9) * ld(t6)).store(); // ebp-0xa4
    let t51 = (ld(t14) * ld(t1)).store(); // ebp-0xa0
    let t52 = (ld(t13) * ld(t2)).store(); // ebp-0x9c
    let t53 = (ld(t10) * ld(t1)).store(); // ebp-0x98
    let t54 = (ld(t9) * ld(t2)).store(); // ebp-0x94
    let t55 = (ld(t6) * ld(t1)).store(); // ebp-0x90
    let t56 = (ld(t5) * ld(t2)).store(); // ebp-0x8c
    let t57 = ((ld(t49) * ld(t16)) + ((ld(t8) * ld(t45)) + ld(t48).mul_m32(t12))).store(); // ebp-0x88
    let t58 =
        (ld(t57) - ((ld(t16) * ld(t50)) + ((ld(t8) * ld(t46)) + ld(t47).mul_m32(t12)))).store(); // ebp-0x64
    let t59 = ((ld(t46).mul_m32(t4) + ld(t51).mul_m32(t12)) + ld(t54).mul_m32(t16)).store(); // ebp-0x88
    let t60 =
        (ld(t59) - ((ld(t52).mul_m32(t12) + (ld(t4) * ld(t45))) + ld(t53).mul_m32(t16))).store(); // ebp-0x60
    let t61 = (((ld(t47) * ld(t4)) + ld(t52).mul_m32(t8)) + ld(t55).mul_m32(t16)).store(); // ebp-0x88
    let t62 =
        (ld(t61) - (((ld(t4) * ld(t48)) + ld(t51).mul_m32(t8)) + ld(t56).mul_m32(t16))).store(); // ebp-0x5c
    let t63 = ((ld(t4).mul_m32(t50) + ld(t53).mul_m32(t8)) + ld(t56).mul_m32(t12)).store(); // ebp-0x88
    let t64 =
        (ld(t63) - ((ld(t4).mul_m32(t49) + ld(t54).mul_m32(t8)) + ld(t55).mul_m32(t12))).store(); // ebp-0x58
    let t65 = ((ld(t50).mul_m32(t15) + ld(t47).mul_m32(t11)) + ld(t7).mul_m32(t46)).store(); // ebp-0x88
    let t66 =
        (ld(t65) - ((ld(t49).mul_m32(t15) + (ld(t7) * ld(t45))) + ld(t48).mul_m32(t11))).store(); // ebp-0x54
    let t67 = (((ld(t45) * ld(t3)) + ld(t53).mul_m32(t15)) + ld(t52).mul_m32(t11)).store(); // ebp-0x88
    let t68 =
        (ld(t67) - ((ld(t54).mul_m32(t15) + ld(t51).mul_m32(t11)) + ld(t3).mul_m32(t46))).store(); // ebp-0x50
    let t69 = (((ld(t51) * ld(t7)) + ld(t56).mul_m32(t15)) + ld(t3).mul_m32(t48)).store(); // ebp-0x88
    let t70 =
        (ld(t69) - ((ld(t52) * ld(t7)) + (ld(t55).mul_m32(t15) + ld(t3).mul_m32(t47)))).store(); // ebp-0x4c
    let t71 = (((ld(t55) * ld(t11)) + ld(t3).mul_m32(t49)) + (ld(t54) * ld(t7))).store(); // ebp-0x88
    let t72 = (((ld(t7) * ld(t53)) + (ld(t11) * ld(t56))) + ld(t3).mul_m32(t50))
        .subr_m32(t71)
        .store(); // ebp-0x48
    let t73 = (((ld(t5).mul_m32(t32) + ld(t1).mul_m32(t30)) + ld(t9).mul_m32(t34))
        + ld(t13).mul_m32(t36))
    .store(); // ebp-0x88
    let det = t73;
    let inv = if det == 0.0 {
        0.0
    } else {
        (Reg::ONE / ld(det)).store()
    };
    let cof = [
        t30, t32, t34, t36, t38, t40, t42, t44, t58, t60, t62, t64, t66, t68, t70, t72,
    ];
    cof.map(|c| (ld(c) * ld(inv)).store())
}

/// `sceVu0VectorLengthSquared(v)`: `x² + y² + z²`.
#[must_use]
pub fn length_squared(v: [f32; 3]) -> f32 {
    dot3(v, v).store()
}

/// `(x² + y²) + z²` as `cdc_Vector_CanNormalize3` and
/// `cdc_Vector_SafeNormalize3` compute it (each square on the stack),
/// stored as an `f32`.
#[must_use]
pub fn square_sum(v: [f32; 3]) -> f32 {
    let [x, y, z] = v.map(load);
    (y * y + x * x + z * z).store()
}

/// What `cdc_Vector_CanNormalize3` leaves in `eax` for the square sum `s`
/// and the threshold `min` (`FLT_MIN`): `al` 1 when `s` is a number, finite
/// and above `min`. The rest of `eax` is the x87 status word's high byte
/// from comparing `min` with `s` (`fcomp`, `fnstsw ax`): C0 (0x100) set
/// when `min < s`, C3 (0x4000) when equal; the stack top field is 0 as the
/// callers leave the stack empty. A NaN or an infinity returns 0 (`al`
/// cleared over `_isnan`'s 1, or `_finite`'s 0).
#[must_use]
pub fn can_normalize3_eax(s: f32, min: f32) -> u32 {
    if s.is_nan() || s.is_infinite() {
        return 0;
    }
    match load(min).value().partial_cmp(&load(s).value()) {
        Some(std::cmp::Ordering::Less) => 0x101,
        Some(std::cmp::Ordering::Equal) => 0x4000,
        _ => 0,
    }
}

/// `cdc_Vector_SafeNormalize3` on `v` whose square sum `s` passed the
/// check: `1 / sqrt(s)` stored as an `f32` (as is the root), times each
/// component (`x` and `y` from the register, `z` as the memory operand).
#[must_use]
pub fn scale_to_unit(v: [f32; 3], s: f32) -> [f32; 3] {
    let root = load(s).sqrt().store();
    let inv = load((Reg::ONE / load(root)).store());
    [
        (load(v[0]) * inv).store(),
        (load(v[1]) * inv).store(),
        inv.mul_m32(v[2]).store(),
    ]
}

/// `sce::Vectormath::Scalar::Aos::inverse(Matrix4)` (`0x00573d00`): the
/// inverse of a column-major 4x4 matrix by cofactors (the scalar
/// Vectormath form): 2x2 minors of rows 2-3 and 0-1, the first column's
/// cofactors, `1 / det` (the cofactor sum, innermost first), then every
/// cofactor times it. Each product sum is formed at the x87's 53 bits and
/// stored as an `f32` where the original stores one.
#[must_use]
pub fn inverse4(m: &Matrix) -> Matrix {
    let r = |i: usize| load(m[i]);
    let f = |x: f32| load(x);
    // the 2x2 minors `a b - c d`
    let minor = |a: usize, b: usize, c: usize, d: usize| (r(a) * r(b) - r(c) * r(d)).store();
    let t1 = minor(10, 3, 11, 2);
    let t2 = minor(14, 7, 15, 6);
    let t3 = minor(10, 1, 2, 9);
    let t4 = minor(14, 5, 13, 6);
    let t5 = minor(9, 3, 1, 11);
    let t6 = minor(13, 7, 5, 15);
    // the first column's cofactors
    let c0 = (r(9) * f(t2) - r(11) * f(t4) - r(10) * f(t6)).store();
    let c1 = (r(13) * f(t1) - r(15) * f(t3) - r(14) * f(t5)).store();
    let c2 = (f(t4) * r(3) + r(2) * f(t6) - r(1) * f(t2)).store();
    let c3 = (f(t5) * r(6) + r(7) * f(t3) - r(5) * f(t1)).store();
    let det = f(c3) * r(12) + (f(c2) * r(8) + (f(c1) * r(4) + f(c0) * r(0)));
    let inv = (Reg::ONE / det).store();
    // the products the other columns reuse
    let p = |a: usize, t: f32| (r(a) * f(t)).store();
    let (p8_t2, p12_t1, p0_t2, p4_t1) = (p(8, t2), p(12, t1), p(0, t2), p(4, t1));
    let (p8_t4, p12_t3, p0_t4, p4_t3) = (p(8, t4), p(12, t3), p(0, t4), p(4, t3));
    let (p8_t6, p12_t5, p0_t6, p4_t5) = (p(8, t6), p(12, t5), p(0, t6), p(4, t5));
    // the 2x2 minors of rows 0-1
    let u1 = minor(8, 1, 9, 0);
    let u2 = minor(12, 5, 13, 4);
    let u3 = minor(8, 3, 11, 0);
    let u4 = minor(12, 7, 15, 4);
    let u5 = minor(8, 2, 0, 10);
    let u6 = minor(12, 6, 4, 14);
    let col2 = [
        (r(11) * f(u2) - r(9) * f(u4) + f(p8_t6)).store(),
        (r(15) * f(u1) - r(13) * f(u3) + f(p12_t5)).store(),
        (f(u4) * r(1) - r(3) * f(u2) - f(p0_t6)).store(),
        (r(5) * f(u3) - r(7) * f(u1) - f(p4_t5)).store(),
    ];
    let col3 = [
        (r(9) * f(u6) - r(10) * f(u2) + f(p8_t4)).store(),
        (r(13) * f(u5) - r(14) * f(u1) + f(p12_t3)).store(),
        (f(u2) * r(2) - r(1) * f(u6) - f(p0_t4)).store(),
        (f(u1) * r(6) - r(5) * f(u5) - f(p4_t3)).store(),
    ];
    let col1 = [
        (r(10) * f(u4) - r(11) * f(u6) - f(p8_t2)).store(),
        (r(14) * f(u3) - r(15) * f(u5) - f(p12_t1)).store(),
        (f(u6) * r(3) - f(u4) * r(2) + f(p0_t2)).store(),
        (f(u5) * r(7) - f(u3) * r(6) + f(p4_t1)).store(),
    ];
    let col0 = [c0, c1, c2, c3];
    let mut out = [0.0; 16];
    for (k, c) in col0
        .iter()
        .chain(&col1)
        .chain(&col2)
        .chain(&col3)
        .enumerate()
    {
        out[k] = (f(*c) * f(inv)).store();
    }
    out
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// The threshold `cdc_Vector_CanNormalize3` compares the square sum with
    /// (`FLT_MIN`, an `f32` in `.rdata`).
    pub const NORMALIZE_MIN: Global<f32> = Global::new(0x00b4_cf60);

    /// `cdc_Vector_CanNormalize3(v)` (`0x00703bc0`, `this` in ECX): whether
    /// `v.xyz` has a square sum that is a finite number above `FLT_MIN`;
    /// the whole `eax` ([`super::can_normalize3_eax`]).
    pub fn can_normalize3(mem: &impl Mem, v: u32) -> u32 {
        let s = super::square_sum(mem.read(v));
        super::can_normalize3_eax(s, NORMALIZE_MIN.get(mem))
    }

    /// `cdc_Vector_SafeNormalize3(v)` (`0x00704200`, fastcall): `v.xyz`
    /// scaled to length 1 (`w` kept), or all four set to 0 when it cannot
    /// be ([`can_normalize3`]).
    pub fn safe_normalize3(mem: &mut impl Mem, v: u32) {
        let x: [f32; 3] = mem.read(v);
        if can_normalize3(mem, v).to_le_bytes()[0] == 0 {
            for i in (0..4).rev() {
                mem.write(at(v, i), 0.0f32);
            }
            return;
        }
        let r = super::scale_to_unit(x, super::square_sum(x));
        for (i, c) in (0..3).zip(r) {
            mem.write(at(v, i), c);
        }
    }

    use super::Matrix;

    fn read_matrix(mem: &impl Mem, m: u32) -> Matrix {
        mem.read(m)
    }

    fn write_matrix(mem: &mut impl Mem, dst: u32, m: &Matrix) {
        mem.write(dst, *m);
    }

    fn at(p: u32, i: u32) -> u32 {
        p.wrapping_add(4 * i)
    }

    /// `cdc_Mul4x4(dst, m, v)` (`0x007058f0`): the 4-vector `v × m`. `v` is
    /// copied first; each component reads its column of `m` before it is
    /// written, as the original does (so `dst` may overlap either).
    pub fn mul4x4_vector(mem: &mut impl Mem, dst: u32, m: u32, v: u32) {
        let v: [f32; 4] = mem.read(v);
        for i in 0..4 {
            let col = [0, 4, 8, 12].map(|r| mem.read::<f32>(at(m, r + i)));
            let x = super::mul4x4_component(i, v, col);
            mem.write(at(dst, i), x);
        }
    }

    /// `cdc_Mul3x4(dst, m, v)` (`0x00705850`): the point `v` transformed by
    /// `m` (translation included), `w` = 0. Same order of reads and writes
    /// as the original.
    pub fn mul3x4(mem: &mut impl Mem, dst: u32, m: u32, v: u32) {
        let v: [f32; 3] = mem.read(v);
        for i in 0..3 {
            let col = [0, 4, 8, 12].map(|r| mem.read::<f32>(at(m, r + i)));
            let x = super::mul3x4_component(i, v, col);
            mem.write(at(dst, i), x);
        }
        mem.write(at(dst, 3), 0.0f32);
    }

    /// `cdc_Mul4x4_007059b0(dst, a, b)` (`0x007059b0`): `b × a`, both read
    /// before `dst` is written.
    pub fn mul4x4(mem: &mut impl Mem, dst: u32, a: u32, b: u32) {
        let r = super::mul4x4(&read_matrix(mem, a), &read_matrix(mem, b));
        write_matrix(mem, dst, &r);
    }

    /// `cdc_Transpose(dst, m)` (`0x007063f0`).
    pub fn transpose(mem: &mut impl Mem, dst: u32, m: u32) {
        let r = super::transpose(&read_matrix(mem, m));
        write_matrix(mem, dst, &r);
    }

    /// `cdc_OrthonormalInverse3x4(dst, m)` (`0x00705db0`).
    pub fn orthonormal_inverse3x4(mem: &mut impl Mem, dst: u32, m: u32) {
        let r = super::orthonormal_inverse3x4(&read_matrix(mem, m));
        write_matrix(mem, dst, &r);
    }

    /// `cdc::Matrix::RotX` (`0x00705ed0`, `thiscall(m, s, c)`) when `first`
    /// is 1, `RotZ` (`0x00706070`) when it is 0: columns `first` and
    /// `first + 1` of every row turned. Returns `m`.
    pub fn rotate(mem: &mut impl Mem, m: u32, first: u32, sin: f32, cos: f32) -> u32 {
        for row in 0..4 {
            let p = at(m, 4 * row + first);
            let (x, y) = super::rotate(mem.read(p), mem.read(at(p, 1)), sin, cos);
            mem.write(p, x);
            mem.write(at(p, 1), y);
        }
        m
    }

    /// `cdc_Inverse4x4(dst, m)` (`0x00704dd0`): `m` read whole first, so
    /// `dst` may be `m`. Returns `dst + 0x48`, where its copy loop leaves
    /// `eax`.
    pub fn inverse4x4(mem: &mut impl Mem, dst: u32, m: u32) -> u32 {
        let r = super::inverse4x4(&read_matrix(mem, m));
        write_matrix(mem, dst, &r);
        dst.wrapping_add(0x48)
    }

    /// `sceVu0InversMatrix4x4(dst, m)` (`0x006ed460`): the identity when
    /// some row of `m` is all zeros (±0; a NaN counts as non-zero), else
    /// [`inverse4x4`]. Returns `dst` for the identity.
    pub fn invers_matrix4x4(mem: &mut impl Mem, dst: u32, m: u32) -> u32 {
        let x = read_matrix(mem, m);
        if x.chunks(4).any(|row| row.iter().all(|&e| e == 0.0)) {
            let id: Matrix = std::array::from_fn(|k| if k % 5 == 0 { 1.0 } else { 0.0 });
            write_matrix(mem, dst, &id);
            return dst;
        }
        inverse4x4(mem, dst, m)
    }

    /// `cdc::Matrix::RotY` (`0x00705fa0`, `thiscall(m, s, c)`): columns 0
    /// (`x`) and 2 (`z`) of every row turned, `z' = z c - x s`,
    /// `x' = z s + x c` ([`super::rotate`] on `(z, x)`). Returns `m`.
    pub fn rotate_y(mem: &mut impl Mem, m: u32, sin: f32, cos: f32) -> u32 {
        for row in 0..4 {
            let p = at(m, 4 * row);
            let (z, x) = super::rotate(mem.read(at(p, 2)), mem.read(p), sin, cos);
            mem.write(p, x);
            mem.write(at(p, 2), z);
        }
        m
    }

    /// `sceVu0VectorLengthSquared(v)` (`0x006ed9b0`).
    pub fn length_squared(mem: &impl Mem, v: u32) -> f32 {
        super::length_squared(mem.read(v))
    }

    /// `0x008f38c0(a, b)`: `(a / b)²` in single precision: both integers
    /// rounded to `f32`, the quotient stored as `f32`, then squared and
    /// stored again; returned in `st(0)`.
    #[must_use]
    pub fn ratio_squared(a: i32, b: i32) -> f32 {
        #[allow(clippy::cast_precision_loss)] // fild, fstp dword
        let (a, b) = (a as f32, b as f32);
        let q = ffx_bridge::x87::Reg::load(a).div_m32(b).store();
        ffx_bridge::x87::Reg::load(q).mul_m32(q).store()
    }

    /// The first screen (`0x01137000`): `+8` near, `+0xc` far, `+0x10`,
    /// `+0x14` width and height, `+0x1c` the focal length (f32).
    pub const SCREEN0: Global<[u32; 8]> = Global::new(0x0113_7000);

    /// `0x00766ae0(m)`: the projection matrix of the first screen: `m` the
    /// identity (`FFXVu0UnitMatrix`, passed in), then `m[0]` =
    /// `2f / (512 / w)`, `m[5]` = `2f / (416 / h)`, `m[10]` =
    /// `(far + near) / (far - near)`, `m[14]` = `near * far * -2 / (far -
    /// near)`, `m[11]` = 1, `m[15]` = 0, in the x87's 53-bit precision with
    /// the two scales and the inputs stored as `f32` on the way.
    pub fn screen_projection<M: Mem>(mem: &mut M, m: u32, unit_matrix: impl FnOnce(&mut M, u32)) {
        use ffx_bridge::x87::{add, div, load, mul, store, sub};
        let s = SCREEN0.get(mem);
        let sx = store(div(512.0, load(s[4])));
        let sy = store(div(416.0, load(s[5])));
        let focal = store(load(s[7]));
        let near = store(load(s[2]));
        let far = store(load(s[3]));
        unit_matrix(mem, m);
        let d = add(load(focal), load(focal));
        mem.write(m, store(div(d, load(sx))));
        mem.write(m.wrapping_add(0x14), store(div(d, load(sy))));
        let (near, far) = (load(near), load(far));
        let range = sub(far, near);
        mem.write(m.wrapping_add(0x28), store(div(add(far, near), range)));
        mem.write(
            m.wrapping_add(0x38),
            store(div(mul(near, mul(far, -2.0)), range)),
        );
        mem.write(m.wrapping_add(0x2c), 1f32.to_bits());
        mem.write(m.wrapping_add(0x3c), 0u32);
    }

    /// `pppCurrentProgram` (`0x0230fd20`).
    pub const PPP_CURRENT_PROGRAM: Global<u32> = Global::new(0x0230_fd20);

    /// `0x009db250(high, low)`: the 64-bit `(high << 32) + low`, `low`
    /// sign-extended (what the original leaves in `edx:eax`).
    #[must_use]
    pub fn wide_key(high: u32, low: u32) -> u64 {
        (u64::from(high) << 32).wrapping_add(i64::from(low.cast_signed()).cast_unsigned())
    }

    /// `0x009db230(high)`: [`wide_key`] of `high` and the effect program
    /// running ([`PPP_CURRENT_PROGRAM`]).
    pub fn current_program_key(mem: &impl Mem, high: u32) -> u64 {
        wide_key(high, PPP_CURRENT_PROGRAM.get(mem))
    }

    /// `sce::Vectormath::Scalar::Aos::inverse(out, m)` (`0x00573d00`):
    /// [`super::inverse4`] of the matrix at `m` written to `out` once every
    /// element is computed (so they may be the same); returns `out`.
    pub fn inverse4(mem: &mut impl Mem, out: u32, m: u32) -> u32 {
        let inv = super::inverse4(&read_matrix(mem, m));
        write_matrix(mem, out, &inv);
        out
    }

    /// `0x0061a810(this)` (thiscall): the 4x4 at `+0x34` set to the
    /// identity (1.0 on the diagonal, 0 elsewhere).
    pub fn identity_at_34(mem: &mut impl Mem, this: u32) {
        let mut m = [0.0f32; 16];
        for k in 0..4 {
            m[5 * k] = 1.0;
        }
        write_matrix(mem, this.wrapping_add(0x34), &m);
    }

    /// The VU0 emulation's registers `0x009dbd90` works in: the scale at
    /// `0x00c8f820`, the vector at `0x00c8f830`.
    pub const VU0_SCALE: Global<f32> = Global::new(0x00c8_f820);
    pub const VU0_VECTOR: Global<[u32; 4]> = Global::new(0x00c8_f830);

    /// `0x009dbd90(out, s, v)`: `v.xyz × s` with `w` kept (the PS2's
    /// `vmulx`), through the VU0 registers: [`VU0_SCALE`] = `s` (as `fst`
    /// stores it), [`VU0_VECTOR`] = `v`, then its `x`, `y`, `z` scaled
    /// (rounded to `f32`), and the register copied to `out`. Returns `w`'s
    /// bits (the original's `eax`).
    pub fn vu0_scale_xyz(mem: &mut impl Mem, out: u32, s: f32, v: u32) -> u32 {
        use ffx_bridge::x87;
        let scale = x87::load(s.to_bits());
        VU0_SCALE.set(mem, f32::from_bits(x87::store(scale)));
        let mut r: [u32; 4] = mem.read(v);
        VU0_VECTOR.set(mem, r);
        for c in &mut r[..3] {
            *c = x87::store(x87::mul(x87::load(*c), scale));
        }
        VU0_VECTOR.set(mem, r);
        mem.write(out, r);
        r[3]
    }

    /// `MagicSqrt_009dbe10(src, dst)` (`0x009dbe10`, `g_MagicImports` slot
    /// 300): `*dst = sqrt(*src)` through `_CIsqrt`, stored as `f32` (a
    /// negative source gives the indefinite NaN). Returns `dst` (the
    /// original's `eax`).
    pub fn magic_sqrt(mem: &mut impl Mem, src: u32, dst: u32) -> u32 {
        use ffx_bridge::x87;
        let v: u32 = mem.read(src);
        let root = x87::store(x87::sqrt(x87::load(v)));
        mem.write(dst, x87::store(x87::load(root)));
        dst
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_and_translation() {
        let mut m = [0.0; 16];
        for i in 0..4 {
            m[5 * i] = 1.0;
        }
        m[12] = 3.0;
        m[13] = -2.0;
        assert_eq!(
            mul3x4_component(0, [1.0, 2.0, 3.0], [1.0, 0.0, 0.0, 3.0]),
            4.0
        );
        let inv = orthonormal_inverse3x4(&m);
        assert_eq!(&inv[12..16], &[-3.0, 2.0, -0.0, 1.0]);
        assert_eq!(mul4x4(&m, &inv)[12..16], [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(transpose(&m)[3], 3.0);
        assert_eq!(rotate(1.0, 0.0, 1.0, 0.0), (0.0, 1.0));
        assert_eq!(length_squared([1.0, 2.0, 2.0]), 9.0);
    }
}
