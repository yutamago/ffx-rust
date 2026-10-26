//! The fog effect manager's parameter blocks (`FogEffectManager`): reset,
//! initialisation, and the distance between two parameter sets that
//! tells whether the fog is still moving.
//!
//! A parameter set: colour `+0..8`, a second colour `+0x10..0x18`, near /
//! far `+0x20`/`+0x24`, two factors `+0x28`/`+0x2c`. The manager holds
//! the current set at `+0` and the target at `+0x30`; its source block
//! is `+0xa0`.

use ffx_bridge::x87;
use ffx_graphic::host::Host;

/// The words `FogEffectManager::reset` (`0x006e34f0`) stores, by offset.
pub const RESET: [(u32, u32); 23] = [
    (0x00, 0x3dcc_cccd),
    (0x04, 0x3e05_1eb8),
    (0x08, 0x3ecc_cccd),
    (0x10, 0x3eb3_3333),
    (0x14, 0x3eb3_3333),
    (0x18, 0x3eb3_3333),
    (0x20, 0x4780_0000),
    (0x24, 0x4780_0080),
    (0x28, 0x3f80_0000),
    (0x2c, 0x3f80_0000),
    (0x30, 0x3dcc_cccd),
    (0x34, 0x3e05_1eb8),
    (0x38, 0x3ecc_cccd),
    (0x40, 0x3eb3_3333),
    (0x44, 0x3eb3_3333),
    (0x48, 0x3eb3_3333),
    (0x50, 0x4780_0000),
    (0x54, 0x4780_0080),
    (0x58, 0x3f80_0000),
    (0x5c, 0x3f80_0000),
    (0x90, 0x3f80_0000),
    (0x94, 0x3f80_0000),
    (0x98, 0x3f80_0000),
];

/// The words `0x006e3400` stores after the reset, in its order.
pub const INIT: [(u32, u32); 10] = [
    (0x60, 0x3d07_2b02),
    (0x64, 0x3d07_2b02),
    (0x68, 0x3d07_2b02),
    (0x70, 0x3d07_2b02),
    (0x74, 0x3d07_2b02),
    (0x78, 0x3d07_2b02),
    (0x8c, 0x3fa9_374c),
    (0x84, 0xc53b_8000),
    (0x80, 0xc561_0000),
    (0x88, 0x3f2f_afb0),
];

/// `a - b` of two `f32` words as the x87 stores it.
fn diff(a: u32, b: u32) -> f64 {
    x87::load(x87::store(x87::sub(x87::load(a), x87::load(b))))
}

/// `f32(acc + d * d)` (the product exact, one rounding to 53 bits).
fn add_square(acc: f64, d: f64) -> f64 {
    x87::load(x87::store(x87::add(acc, x87::mul(d, d))))
}

/// `FogEffectManager::distance(a, b)` (`0x006e3320`) of the words `+0x10,
/// 0x14, 0x18, 0x20, 0x24, 0x28, 0x2c` of both sets: the squared
/// differences of the four `+0x20..0x2c` summed in `f32` steps, those of
/// the three `+0x10..0x18` the same, then the two sums.
#[must_use]
pub fn distance(a: [u32; 7], b: [u32; 7]) -> f32 {
    let d: Vec<f64> = (0..7).map(|i| diff(a[i], b[i])).collect();
    // d[0..3]: +0x10..0x18; d[3..7]: +0x20..0x2c
    let first = x87::load(x87::store(x87::mul(d[3], d[3])));
    let far = [d[4], d[5], d[6]].into_iter().fold(first, add_square);
    let first = x87::load(x87::store(x87::mul(d[0], d[0])));
    let near = [d[1], d[2]].into_iter().fold(first, add_square);
    f32::from_bits(x87::store(x87::add(far, near)))
}

/// The offsets [`distance`] reads, in its array order.
pub const DISTANCE_FIELDS: [u32; 7] = [0x10, 0x14, 0x18, 0x20, 0x24, 0x28, 0x2c];

pub mod bridge {
    //! The originals over game memory.

    use super::{DISTANCE_FIELDS, Host, INIT, RESET, distance};
    use crate::methods::quiet;

    /// `FogEffectManager::reset(fog)` (`0x006e34f0`, fastcall): the
    /// default current and target sets and `+0x90..0x98` 1.0.
    pub fn reset(h: &mut impl Host, fog: u32) {
        for (at, v) in RESET {
            h.write(fog + at, v);
        }
    }

    /// `FogEffectManager::initialize(fog, src, a, b)` (`0x006e3400`, `ret
    /// 0xc`): the source block and two words at `+0xa0..0xa8`, [`reset`],
    /// then the fade speeds and limits of [`INIT`].
    pub fn initialize(h: &mut impl Host, fog: u32, src: u32, a: u32, b: u32) {
        h.write(fog + 0xa0, src);
        h.write(fog + 0xa4, a);
        h.write(fog + 0xa8, b);
        reset(h, fog);
        for (at, v) in INIT {
            h.write(fog + at, v);
        }
    }

    /// `FogEffectManager::distance(fog, other)` (`0x006e3320`, `ret 4`) ->
    /// `ST0`.
    pub fn distance_to(h: &mut impl Host, fog: u32, other: u32) -> f32 {
        let a = DISTANCE_FIELDS.map(|at| h.read::<u32>(fog + at));
        let b = DISTANCE_FIELDS.map(|at| h.read::<u32>(other + at));
        distance(a, b)
    }

    /// `FogEffectManager::isMoving(fog)` (`0x006e3480`, fastcall) -> `al`:
    /// the source's (`+0xa0`) set, rearranged as the original copies it
    /// (target colour `+0x30..0x38` as the colour, the factors swapped),
    /// is more than 0.1 from the manager's own.
    pub fn is_moving(h: &mut impl Host, fog: u32) -> u8 {
        let src: u32 = h.read(fog + 0xa0);
        // the copy's fields `+0x10..0x18, 0x20, 0x24, 0x28, 0x2c` from the
        // source's `+0x10..0x18, 0x20, 0x24, 0x2c, 0x28`
        let b = [0x10, 0x14, 0x18, 0x20, 0x24, 0x2c, 0x28].map(|at| quiet(h.read(src + at)));
        let a = DISTANCE_FIELDS.map(|at| h.read::<u32>(fog + at));
        u8::from(f64::from(distance(a, b)) > f64::from(0.1f32))
    }
}
