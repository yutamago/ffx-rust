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
