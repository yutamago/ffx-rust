//! The node family's rotation matrices (`magic_0003`'s code, 116 DLLs):
//! a 4x4 matrix (16 floats, row-major, for row vectors) built from three
//! angle words as the product of the rotations about the three axes, in
//! one of the six orders. This module has the parts the five builders
//! share and two of them; [`crate::s3_matrix`] has the other three.
//!
//! An angle word is fixed point, `0x10000` to the turn: `(angle >> 4) &
//! 0xfff` indexes the game's sine table (4,096 floats behind the DLL's
//! pointer global) and the cosine is the sine a quarter turn (`+0x4000`)
//! on. With `si`, `ci` the sine and cosine of angle `i` (the word at
//! `edx + 4 * i`) the rotations are
//!
//! ```text
//! X = [1 0 0 0; 0 c0 s0 0; 0 -s0 c0 0; 0 0 0 1]
//! Y = [c1 0 -s1 0; 0 1 0 0; s1 0 c1 0; 0 0 0 1]
//! Z = [c2 s2 0 0; -s2 c2 0 0; 0 0 1 0; 0 0 0 1]
//! ```
//!
//! The original is two inlined 4x4 products: first factor times second,
//! then that times the third, each entry the sum of its four products.
//! Two of each builder's three factors carry `S`, the swap of the first
//! two axes (`S * S` = 1), as in `Y * Z * X = Y * (Z * S) * (S * X)`; the
//! docs of each give its factors. The compiler dropped the products with
//! a constant 1 and folded those of two constants, but kept every product
//! of a sine or cosine with 0: `x * 0` is `-0.0` for a negative `x` and
//! NaN for an infinite or NaN one, and the sums those reach change with
//! it. The ports keep those products ([`z`]) and add the four terms in the
//! original's order ([`sum`]), the original's operand first in each
//! `mulss`/`addss`: one `f32` rounding per instruction and the CPU's
//! choice of NaN ([`crate::sb_sse`]). Where every row of a product has its
//! terms in the same order, a function or closure multiplies a row and the
//! left factor goes in as rows of values, its 0s and 1s included: those
//! give what the folded constants give (`x * 1` is `x`, a NaN quieted, as
//! the `addss` it goes into does anyway). Where the order changes from
//! row to row, the product is written out entry by entry
//! ([`crate::s3_matrix`]).

use crate::sb_sse::{add, mul};
use crate::{Game, Image, Sig};

/// A 4x4 matrix, row by row.
pub(crate) type Matrix = [[f32; 4]; 4];

/// The last row of every factor.
pub(crate) const E3: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// The sines and the cosines of the three angle words at `angles`, from
/// the sine table behind the pointer global at operand `table_op` of the
/// function at `f`. The original shifts arithmetically (`sar`); the 12
/// bits the mask keeps are the same.
pub(crate) fn sines<G: Game>(
    g: &G,
    img: &Image,
    f: u32,
    table_op: u32,
    angles: u32,
) -> ([f32; 3], [f32; 3]) {
    let table = g.read::<u32>(img.word(f + table_op));
    let at = |angle: u32| {
        let i = (angle >> 4) & 0xfff;
        f32::from_bits(g.read::<u32>(table.wrapping_add(4 * i)))
    };
    let mut s = [0.0; 3];
    let mut c = [0.0; 3];
    for i in 0..3 {
        let angle = g.read::<u32>(angles.wrapping_add(4 * i));
        s[i as usize] = at(angle);
        c[i as usize] = at(angle.wrapping_add(0x4000));
    }
    (s, c)
}

/// The terms added left to right, `((t0 + t1) + t2) + ...`.
pub(crate) fn sum<const N: usize>(terms: [f32; N]) -> f32 {
    terms[1..].iter().fold(terms[0], |s, &t| add(s, t))
}

/// `x * 0.0`: `0.0` with `x`'s sign, NaN for an infinite or NaN `x`. (With
/// a constant operand the order of `mulss`'s operands doesn't matter.)
pub(crate) fn z(x: f32) -> f32 {
    mul(x, 0.0)
}

/// Writes `m` (16 floats) at `at`.
pub(crate) fn store<G: Game>(g: &mut G, at: u32, m: &Matrix) {
    for (i, x) in (0u32..).zip(m.as_flattened()) {
        g.write::<u32>(at.wrapping_add(4 * i), x.to_bits());
    }
}

/// A row times `S * X` = `[0 c s 0; 1 0 0 0; 0 -s c 0; 0 0 0 1]` (`s`,
/// `c` of angle 0): the last factor of [`rotation_52a0`] and
/// [`rotation_19d0`]. (The last column adds `r1`'s term before `r0`'s,
/// as the original does.)
fn times_sx([r0, r1, r2, r3]: [f32; 4], s: f32, c: f32) -> [f32; 4] {
    [
        sum([z(r0), r1, z(r2), z(r3)]),
        sum([mul(r0, c), z(r1), mul(r2, -s), z(r3)]),
        sum([mul(r0, s), z(r1), mul(r2, c), z(r3)]),
        sum([z(r1), z(r0), z(r2), r3]),
    ]
}

/// A row times `Y` (`s`, `c` of angle 1): the last factor of
/// [`crate::s3_matrix::rotation_47c0`] and
/// [`crate::s3_matrix::rotation_4250`].
pub(crate) fn times_y([r0, r1, r2, r3]: [f32; 4], s: f32, c: f32) -> [f32; 4] {
    [
        sum([mul(r0, c), z(r1), mul(r2, s), z(r3)]),
        sum([z(r0), r1, z(r2), z(r3)]),
        sum([mul(r0, -s), z(r1), mul(r2, c), z(r3)]),
        sum([z(r0), z(r1), z(r2), r3]),
    ]
}

/// `Y * Z * X`, as `Y * (Z * S) * (S * X)`. Returns `ecx` (`eax`).
///
/// `0x100052a0` (1,393 bytes, `ecx` = the matrix, `edx` = the angles; 116
/// DLLs).
pub fn rotation_52a0<G: Game>(g: &mut G, img: &Image, f: u32, ecx: u32, edx: u32) -> u32 {
    let ([s0, s1, s2], [c0, c1, c2]) = sines(g, img, f, 0x1c, edx);
    let y = [
        [c1, 0.0, -s1, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [s1, 0.0, c1, 0.0],
        E3,
    ];
    // a row times Z * S = [s2 c2 0 0; c2 -s2 0 0; 0 0 1 0; 0 0 0 1]
    let times_zs = |[r0, r1, r2, r3]: [f32; 4]| {
        [
            sum([mul(r0, s2), mul(c2, r1), z(r2), z(r3)]),
            sum([mul(r0, c2), mul(-s2, r1), z(r2), z(r3)]),
            sum([z(r0), z(r1), r2, z(r3)]),
            sum([z(r0), z(r1), z(r2), r3]),
        ]
    };
    let m = y.map(times_zs).map(|r| times_sx(r, s0, c0));
    store(g, ecx, &m);
    ecx
}

/// [`rotation_52a0`].
pub const ROTATION_52A0: Sig = Sig {
    len: 1393,
    hash: 0x6ec9_b2ef_c318_68b7,
    calls: &[],
    imports: &[],
};

/// `Z * Y * X`, as `Z * (Y * S) * (S * X)`. Returns `ecx` (`eax`).
///
/// `0x100019d0` (1,413 bytes, `ecx` = the matrix, `edx` = the angles; 116
/// DLLs).
pub fn rotation_19d0<G: Game>(g: &mut G, img: &Image, f: u32, ecx: u32, edx: u32) -> u32 {
    let ([s0, s1, s2], [c0, c1, c2]) = sines(g, img, f, 0x17, edx);
    let zr = [
        [c2, s2, 0.0, 0.0],
        [-s2, c2, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        E3,
    ];
    // a row times Y * S = [0 c1 -s1 0; 1 0 0 0; 0 s1 c1 0; 0 0 0 1]
    let times_ys = |[r0, r1, r2, r3]: [f32; 4]| {
        [
            sum([z(r0), r1, z(r2), z(r3)]),
            sum([mul(r0, c1), z(r1), mul(r2, s1), z(r3)]),
            sum([mul(-s1, r0), z(r1), mul(c1, r2), z(r3)]),
            sum([z(r0), z(r1), z(r2), r3]),
        ]
    };
    let m = zr.map(times_ys).map(|r| times_sx(r, s0, c0));
    store(g, ecx, &m);
    ecx
}

/// [`rotation_19d0`].
pub const ROTATION_19D0: Sig = Sig {
    len: 1413,
    hash: 0x9fa0_2a9b_f520_7245,
    calls: &[],
    imports: &[],
};
