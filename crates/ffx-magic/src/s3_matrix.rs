//! Three more of the node family's rotation matrices (`magic_0003`'s
//! code, 116 DLLs): `X * Z * Y`, `X * Y * Z` and `Z * X * Y` of the
//! angle words at `edx`, 16 floats at `ecx`. The rotations, the angle
//! words and the rules the sums follow are in [`crate::sb_matrix`]. The
//! compiler ordered the terms of these three less evenly than those of
//! [`crate::sb_matrix`]'s two: their first products are written out entry
//! by entry, each the sum of its products in the original's order, and
//! `0x4d40`'s second product picks one of two orders per entry.

use crate::sb_matrix::{sines, store, sum, times_y, z};
use crate::sb_sse::{add, mul};
use crate::{Game, Image, Sig};

/// `X * Z * Y`, as `(X * S) * (S * Z) * Y`. Returns `ecx` (`eax`).
///
/// `0x100047c0` (1394 bytes, `ecx` = the matrix, `edx` = the angles; 116 DLLs).
pub fn rotation_47c0<G: Game>(g: &mut G, img: &Image, f: u32, ecx: u32, edx: u32) -> u32 {
    let ([s0, s1, s2], [c0, c1, c2]) = sines(g, img, f, 0x1f, edx);
    // X * S = [0 1 0 0; c0 0 s0 0; -s0 0 c0 0; 0 0 0 1] times
    // S * Z = [-s2 c2 0 0; c2 s2 0 0; 0 0 1 0; 0 0 0 1]
    let m = [
        [
            sum([z(-s2), c2, 0.0, 0.0]),
            sum([z(c2), s2, 0.0, 0.0]),
            0.0,
            0.0,
        ],
        [
            sum([mul(-s2, c0), z(c2), z(s0), 0.0]),
            sum([mul(c2, c0), z(s2), z(s0), 0.0]),
            sum([z(c0), 0.0, s0, 0.0]),
            sum([z(c0), 0.0, z(s0), 0.0]),
        ],
        [
            sum([mul(-s0, -s2), z(c2), z(c0), 0.0]),
            sum([mul(-s0, c2), z(s2), z(c0), 0.0]),
            sum([z(-s0), 0.0, c0, 0.0]),
            sum([z(-s0), 0.0, z(c0), 0.0]),
        ],
        [
            sum([z(c2), z(-s2), 0.0, 0.0]),
            sum([z(s2), z(c2), 0.0, 0.0]),
            0.0,
            1.0,
        ],
    ];
    store(g, ecx, &m.map(|r| times_y(r, s1, c1)));
    ecx
}

/// `X * Y * Z`, as `X * (Y * S) * (S * Z)`. Returns `ecx` (`eax`).
///
/// `0x10004d40` (1364 bytes, `ecx` = the matrix, `edx` = the angles; 116 DLLs).
pub fn rotation_4d40<G: Game>(g: &mut G, img: &Image, f: u32, ecx: u32, edx: u32) -> u32 {
    let ([s0, s1, s2], [c0, c1, c2]) = sines(g, img, f, 0x0c, edx);
    // X = [1 0 0 0; 0 c0 s0 0; 0 -s0 c0 0; 0 0 0 1] times
    // Y * S = [0 c1 -s1 0; 1 0 0 0; 0 s1 c1 0; 0 0 0 1]
    let [m0, m1, m2, m3] = [
        [
            0.0,
            sum([0.0, c1, z(s1), 0.0]),
            sum([0.0, -s1, z(c1), 0.0]),
            0.0,
        ],
        [
            sum([0.0, c0, z(s0), 0.0]),
            sum([z(c0), z(c1), mul(s0, s1), 0.0]),
            sum([z(-s1), z(c0), mul(s0, c1), 0.0]),
            sum([z(c0), 0.0, z(s0), 0.0]),
        ],
        [
            sum([0.0, -s0, z(c0), 0.0]),
            sum([z(-s0), z(c1), mul(c0, s1), 0.0]),
            sum([z(-s0), z(-s1), mul(c0, c1), 0.0]),
            sum([z(-s0), 0.0, z(c0), 0.0]),
        ],
        [
            0.0,
            sum([z(c1), 0.0, z(s1), 0.0]),
            sum([z(-s1), 0.0, z(c1), 0.0]),
            1.0,
        ],
    ];
    // times S * Z = [-s2 c2 0 0; c2 s2 0 0; 0 0 1 0; 0 0 0 1]. The
    // compiler added the last two columns' terms in two orders: with the
    // two 0 terms first, as a pair (`then_pair`), or left to right
    // (`in_order`); which one each row's entry got is the original's
    let col0 = |r: [f32; 4]| sum([mul(r[0], -s2), mul(r[1], c2), z(r[2]), z(r[3])]);
    let col1 = |r: [f32; 4]| sum([mul(r[0], c2), mul(r[1], s2), z(r[2]), z(r[3])]);
    let pair = |r: [f32; 4]| add(z(r[0]), z(r[1]));
    let col2_then_pair = |r: [f32; 4]| sum([r[2], pair(r), z(r[3])]);
    let col2_in_order = |r: [f32; 4]| sum([z(r[0]), z(r[1]), r[2], z(r[3])]);
    let col3_then_pair = |r: [f32; 4]| sum([z(r[2]), pair(r), r[3]]);
    let col3_in_order = |r: [f32; 4]| sum([z(r[0]), z(r[1]), z(r[2]), r[3]]);
    let out = [
        [col0(m0), col1(m0), col2_in_order(m0), col3_in_order(m0)],
        [col0(m1), col1(m1), col2_then_pair(m1), col3_then_pair(m1)],
        [col0(m2), col1(m2), col2_then_pair(m2), col3_in_order(m2)],
        [col0(m3), col1(m3), col2_then_pair(m3), col3_in_order(m3)],
    ];
    store(g, ecx, &out);
    ecx
}

/// `Z * X * Y`, as `(Z * S) * (S * X) * Y`. Returns `ecx` (`eax`).
///
/// `0x10004250` (1382 bytes, `ecx` = the matrix, `edx` = the angles; 116 DLLs).
pub fn rotation_4250<G: Game>(g: &mut G, img: &Image, f: u32, ecx: u32, edx: u32) -> u32 {
    let ([s0, s1, s2], [c0, c1, c2]) = sines(g, img, f, 0x0c, edx);
    // Z * S = [s2 c2 0 0; c2 -s2 0 0; 0 0 1 0; 0 0 0 1] times
    // S * X = [0 c0 s0 0; 1 0 0 0; 0 -s0 c0 0; 0 0 0 1]
    let m = [
        [
            sum([z(s2), c2, 0.0, 0.0]),
            sum([mul(c0, s2), z(c2), z(-s0), 0.0]),
            sum([mul(s0, s2), z(c2), z(c0), 0.0]),
            sum([z(c2), z(s2), 0.0, 0.0]),
        ],
        [
            sum([z(c2), -s2, 0.0, 0.0]),
            sum([mul(c2, c0), z(-s2), z(-s0), 0.0]),
            sum([mul(c2, s0), z(-s2), z(c0), 0.0]),
            sum([z(-s2), z(c2), 0.0, 0.0]),
        ],
        [
            0.0,
            sum([z(c0), 0.0, -s0, 0.0]),
            sum([z(s0), 0.0, c0, 0.0]),
            0.0,
        ],
        [
            0.0,
            sum([z(c0), 0.0, z(-s0), 0.0]),
            sum([z(s0), 0.0, z(c0), 0.0]),
            1.0,
        ],
    ];
    store(g, ecx, &m.map(|r| times_y(r, s1, c1)));
    ecx
}

/// [`rotation_47c0`].
pub const ROTATION_47C0: Sig = Sig {
    len: 1394,
    hash: 0x5ca2_99ff_5c8f_7b22,
    calls: &[],
    imports: &[],
};

/// [`rotation_4d40`].
pub const ROTATION_4D40: Sig = Sig {
    len: 1364,
    hash: 0x8cf9_4ae8_61b8_45b9,
    calls: &[],
    imports: &[],
};

/// [`rotation_4250`].
pub const ROTATION_4250: Sig = Sig {
    len: 1382,
    hash: 0x9eb7_8cdc_34f2_0592,
    calls: &[],
    imports: &[],
};
