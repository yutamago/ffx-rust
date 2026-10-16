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

/// `sceVu0VectorLengthSquared(v)`: `x² + y² + z²`.
#[must_use]
pub fn length_squared(v: [f32; 3]) -> f32 {
    dot3(v, v).store()
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Mem;

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

    /// `sceVu0VectorLengthSquared(v)` (`0x006ed9b0`).
    pub fn length_squared(mem: &impl Mem, v: u32) -> f32 {
        super::length_squared(mem.read(v))
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
