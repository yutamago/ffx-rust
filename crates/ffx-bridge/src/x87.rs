//! The x87 conversions the original makes that Rust's casts do not.
//!
//! The game thread runs the x87 at 53-bit precision (control word
//! `0x027f`), so its arithmetic is `f64` arithmetic: each operation rounds
//! once to a 53-bit mantissa, as an `f64` operation does. The exponent
//! range is wider (15 bits), which only matters past `f64`'s range; values
//! from `f32` and `i32` operands stay well inside it. Code that keeps
//! intermediates on the x87 stack is reproduced by computing in `f64` and
//! rounding to `f32` exactly where the original stores a float
//! (`fstp dword`).
//!
//! Float to integer goes through the C runtime's `_ftol2_sse`, which picks
//! one of two conversions by the CPU: [`ftol`].

use crate::{Global, Mem};

/// The C runtime's `__sse2_available`: set at start-up on a CPU with SSE2
/// (every CPU the game runs on), 0 in an image that has not run its start-up
/// code (the oracle on the image alone).
pub const SSE2_AVAILABLE: Global<u32> = Global::new(0x0193_ed20);

/// `ftol2_sse` (`_ftol2_sse`, `0x009497e0`): `x` (ST0) to an integer, rounding toward
/// zero, as EAX; [`cvttsd2si`] when [`SSE2_AVAILABLE`] is set, else
/// [`ftol2`].
pub fn ftol(mem: &impl Mem, x: f64) -> u32 {
    if SSE2_AVAILABLE.get(mem) != 0 {
        cvttsd2si(x)
    } else {
        ftol2(x)
    }
}

/// SSE2's `cvttsd2si`: toward zero; NaN and anything outside `i32` give
/// `0x80000000`. (Rust's `as i32` saturates instead.)
#[must_use]
pub fn cvttsd2si(x: f64) -> u32 {
    if x.is_nan() || x >= 2_147_483_648.0 || x <= -2_147_483_649.0 {
        0x8000_0000
    } else {
        #[allow(clippy::cast_possible_truncation)] // in range, checked
        let v = x.trunc() as i32;
        v.cast_unsigned()
    }
}

/// `_ftol2`'s x87 path: `fistp qword` and a correction toward zero, so the
/// low 32 bits of the 64-bit truncation; NaN and anything outside `i64` give
/// the x87's integer indefinite, whose low half is 0.
#[must_use]
pub fn ftol2(x: f64) -> u32 {
    if x.is_nan() || x >= 9_223_372_036_854_775_808.0 || x < -9_223_372_036_854_775_808.0 {
        0
    } else {
        #[allow(clippy::cast_possible_truncation)] // in range
        let wide = x.trunc() as i64;
        let [b0, b1, b2, b3, ..] = wide.to_le_bytes();
        u32::from_le_bytes([b0, b1, b2, b3])
    }
}

/// A float as `fld m32` loads it: exact, but a signalling NaN comes out
/// quiet.
#[must_use]
pub fn load(bits: u32) -> f64 {
    let quiet = if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
        bits | 0x0040_0000
    } else {
        bits
    };
    f64::from(f32::from_bits(quiet))
}

/// `fstp m32`: rounded to `f32` (NaNs keep their sign and top payload
/// bits).
#[must_use]
pub fn store(x: f64) -> u32 {
    #[allow(clippy::cast_possible_truncation)] // the store rounds
    let f = x as f32;
    f.to_bits()
}

/// The NaN an x87 operation on `a` and `b` gives when one of them is NaN:
/// of two, the larger significand, then the positive one (SSE, and so
/// Rust, take the first operand instead). Operands come from [`load`], so
/// they are quiet.
#[must_use]
pub fn nan(a: f64, b: f64) -> Option<f64> {
    const SIG: u64 = 0x000f_ffff_ffff_ffff;
    match (a.is_nan(), b.is_nan()) {
        (true, true) => {
            let (x, y) = (a.to_bits(), b.to_bits());
            Some(if (y & SIG, x >> 63) > (x & SIG, y >> 63) {
                b
            } else {
                a
            })
        }
        (true, false) => Some(a),
        (false, true) => Some(b),
        (false, false) => None,
    }
}

/// `a + b` on the x87 at 53 bits.
#[must_use]
pub fn add(a: f64, b: f64) -> f64 {
    nan(a, b).unwrap_or(a + b)
}

/// `a - b` on the x87 at 53 bits.
#[must_use]
pub fn sub(a: f64, b: f64) -> f64 {
    nan(a, b).unwrap_or(a - b)
}

/// `a * b` on the x87 at 53 bits.
#[must_use]
pub fn mul(a: f64, b: f64) -> f64 {
    nan(a, b).unwrap_or(a * b)
}

/// `a / b` on the x87 at 53 bits.
#[must_use]
pub fn div(a: f64, b: f64) -> f64 {
    nan(a, b).unwrap_or(a / b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_truncate_and_flag_overflow() {
        assert_eq!(cvttsd2si(-1.9), (-1i32).cast_unsigned());
        assert_eq!(cvttsd2si(3e9), 0x8000_0000);
        assert_eq!(cvttsd2si(f64::NAN), 0x8000_0000);
        assert_eq!(ftol2(-1.9), (-1i32).cast_unsigned());
        assert_eq!(ftol2(4_294_967_297.5), 1);
        assert_eq!(ftol2(1e30), 0);
    }
}
