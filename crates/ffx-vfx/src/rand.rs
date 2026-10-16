//! The effects' random numbers: VU0's `R` register as the PC port
//! emulates it (`g_Vu0_R`, `0x00c0a000`).
//!
//! `R` holds the bits of a float in `[1, 2)`. A step (the PS2's `vrnext`)
//! shifts the low 22 fraction bits up by one and feeds bit 4 of
//! `R ^ (R >> 18)` into the bottom. Until something seeds it
//! (`g_Vu0_RSeeded` is 0), a draw uses 1.0 instead and marks it seeded.
//!
//! `op_random(x)` and `op_random_p(x)` scale a draw to `[-x, x)` and
//! `[0, x)`, and leave `R` at the draw minus 1.8872414 (the next seed),
//! with the x87 arithmetic of the original ([`crate::x87`]).

use crate::x87;

/// The constant the draws subtract for the next seed (`f64` at
/// `0x00b58250`; it has 21 significant bits, so `draw - C` is exact).
const NEXT: f64 = 1.887_241_363_525_390_6;

/// One step of `R` (bits in, bits out).
#[must_use]
pub const fn step(r: u32) -> u32 {
    ((((r >> 18) ^ r) >> 4) & 1) | (((r & 0x003f_ffff) | 0x1fc0_0000) << 1)
}

/// A draw in `[1, 2)`: `R` stepped, or 1.0 while unseeded.
#[must_use]
pub fn draw(r: u32, seeded: bool) -> f32 {
    if seeded { f32::from_bits(step(r)) } else { 1.0 }
}

/// `op_random(x)`: from the draw `d`, (`2d - 3) * x` and the next `R`.
#[must_use]
pub fn signed(d: f32, x: f32) -> (f32, f32) {
    let two = d + d; // exact
    (x87::mul(x87::sub_f64(two, 3.0), x), x87::sub_f64(two, NEXT))
}

/// `op_random_p(x)`: from the draw `d`, `(d - 1) * x` and the next `R`.
#[must_use]
pub fn positive(d: f32, x: f32) -> (f32, f32) {
    let unit = x87::sub_f64(d, 1.0);
    (x87::mul(unit, x), x87::sub_f64(unit, NEXT))
}

/// The generator where the game keeps it.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// `g_Vu0_R`: VU0's `R`, as `f32` bits.
    pub const R: Global<u32> = Global::new(0x00c0_a000);
    /// `g_Vu0_RSeeded`: nonzero once `R` has been seeded.
    pub const SEEDED: Global<u32> = Global::new(0x00c8_f728);

    fn run(mem: &mut impl Mem, x: f32, f: fn(f32, f32) -> (f32, f32)) -> f32 {
        let d = super::draw(R.get(mem), SEEDED.get(mem) != 0);
        SEEDED.set(mem, 1);
        let (v, next) = f(d, x);
        R.set(mem, next.to_bits());
        v
    }

    /// `op_random(x)` (`0x007e6700`): a random value in `[-x, x)`.
    pub fn op_random(mem: &mut impl Mem, x: f32) -> f32 {
        run(mem, x, super::signed)
    }

    /// `op_random_p(x)` (`0x007e6780`): a random value in `[0, x)`.
    pub fn op_random_p(mem: &mut impl Mem, x: f32) -> f32 {
        run(mem, x, super::positive)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_stay_in_one_to_two() {
        let mut r = 0x3f80_0000;
        for _ in 0..1000 {
            r = step(r);
            let f = f32::from_bits(r);
            assert!((1.0..2.0).contains(&f), "{f}");
        }
        assert_eq!(NEXT.to_bits(), 0x3ffe_3224_0000_0000);
    }
}
