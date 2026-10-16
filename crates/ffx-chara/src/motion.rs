//! Motion interpolation helpers: float and angle lerps with a damped
//! weight, and the 12-bit angle lerp in turns (`AngleTurn_Lerp`). All
//! cdecl on floats, results in `st0`; each step rounds as the x87 does
//! ([`crate::x87::Reg`]).

use crate::fixed::{fixed8_to_float, fixed12_to_float};
use crate::x87::Reg;

/// 3.141592 and 6.283184 as the `double`s the original compares with.
const PI: f64 = f64::from_bits(0x4009_21fb_0000_0000);
const TWO_PI: f64 = f64::from_bits(0x4019_21fb_0000_0000);

/// `Chr_Lerp(a, b, t)` (`0x00839cb0`): `t × b + (1 − t) × a`.
#[must_use]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    let one_t = Reg::constant(1.0).sub_st(Reg::load(t)).mul_m32(a);
    Reg::load(t).mul_st(Reg::load(b)).add_st(one_t).store()
}

/// The weight `max(x − |a − b| × k, lo)`, the difference unrounded.
fn damped_weight(a: f32, b: f32, x: f32, lo: f32, k: f32) -> f32 {
    let d = Reg::load(a).sub_st(Reg::load(b)).abs().mul_m32(k);
    let t = Reg::load(x).sub_st(d).store();
    if f64::from(crate::x87::load(lo)) > f64::from(t) {
        crate::x87::load(lo)
    } else {
        t
    }
}

/// `Chr_LerpDamped(a, b, x, lo, k)` (`0x00839d70`): [`lerp`] with the
/// weight `max(x − |a − b| × k, lo)`.
#[must_use]
pub fn lerp_damped(a: f32, b: f32, x: f32, lo: f32, k: f32) -> f32 {
    let t = damped_weight(a, b, x, lo, k);
    lerp(crate::x87::load(a), crate::x87::load(b), t)
}

/// `Chr_LerpAngleDamped(a, b, x, lo, k)` (`0x00839cd0`): as
/// [`lerp_damped`] on angles: `b` first moved by a turn toward `a` when
/// they are more than π apart.
#[must_use]
pub fn lerp_angle_damped(a: f32, b: f32, x: f32, lo: f32, k: f32) -> f32 {
    let a = crate::x87::load(a);
    let mut b = crate::x87::load(b);
    if PI < Reg::load(a).sub_st(Reg::load(b)).get() {
        b = Reg::load(b).add_st(Reg::constant(TWO_PI)).store();
    }
    if Reg::load(b).sub_st(Reg::load(a)).get() > PI {
        b = Reg::load(b).sub_st(Reg::constant(TWO_PI)).store();
    }
    let t = damped_weight(a, b, x, lo, k);
    lerp(a, b, t)
}

/// `AngleTurn_Lerp(a, b, t)` (`0x00839e80`): 12-bit angles `a` toward
/// `b` the short way, in turns (as floats), `t` in 8.8 fixed.
#[must_use]
pub fn angle_turn_lerp(a: u32, b: u32, t: u32) -> f32 {
    let fa = fixed12_to_float(a & 0xfff);
    let fb = fixed12_to_float(b & 0xfff);
    let ft = fixed8_to_float(t);
    let mut base = fa;
    let mut d = Reg::load(fb).sub_st(Reg::load(fa)).store();
    // not below 0.5, NaN included
    if d.partial_cmp(&0.5) != Some(std::cmp::Ordering::Less) {
        base = Reg::load(fa).add_st(Reg::constant(1.0)).store();
        d = Reg::load(fb).sub_st(Reg::load(base)).store();
    }
    if -0.5 >= d {
        let b1 = Reg::load(fb).add_st(Reg::constant(1.0)).store();
        d = Reg::load(b1).sub_st(Reg::load(base)).store();
    }
    Reg::load(d)
        .mul_st(Reg::load(ft))
        .add_st(Reg::load(base))
        .store()
}

/// The originals on the characters.
pub mod bridge {
    use ffx_bridge::Mem;

    use crate::layout as l;

    /// `Ch_MotionGetLoop(chr)` (`0x00839880`): the loops left, `i16` at
    /// `+0x724`, sign-extended.
    pub fn ch_motion_get_loop(mem: &impl Mem, chr: u32) -> i32 {
        i32::from(mem.read::<i16>(chr + l::MOTION_LOOPS_LEFT))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerps() {
        assert_eq!(lerp(0.0, 2.0, 0.5), 1.0);
        assert_eq!(lerp_damped(0.0, 2.0, 0.5, 0.0, 0.0), 1.0);
        assert!((angle_turn_lerp(0, 0xf00, 0x100) - (-0.0625 + 1.0)).abs() < 1e-6);
    }
}
