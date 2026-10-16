//! What the original's x87 code does to `f32` values, for bit-exact ports.
//!
//! The game thread runs the x87 unit at 53-bit precision (control word
//! `0x027f`). Under it:
//! - `fld m32` / `fstp m32` copies a float unchanged, except a signalling
//!   NaN, which comes out quiet (bit 22 set): [`copy`].
//! - One `+ - * /` or `fsqrt` on `f32` operands, stored with `fstp m32`,
//!   rounds twice (to 53 bits, then to 24); for these operations that gives
//!   the correctly rounded `f32` result (53 >= 2 * 24 + 2), so Rust's `f32`
//!   arithmetic matches.
//! - Chains kept on the x87 stack (`a * b - c * d`, sums of squares) round
//!   each step to 53 bits and only the stored result to 24: computed in
//!   `f64` and converted once, they match as long as nothing leaves the
//!   `f64` exponent range (inputs from `f32` cannot).
//!
//! The quieting in [`copy`] is the CPU's (the masked response to the
//! invalid-operation exception of `fld m32`); `ffx-oracle`'s emulator
//! (QEMU 5 softfloat) keeps the signalling bit, so it cannot check those
//! cases and they stay out of the parity tests.
//!
//! NaN operands are left out: the x87 unit picks which NaN propagates by
//! its own rules (the larger significand), SSE by operand order, and the
//! game's data holds none.

/// The bits of `f32` after `fld m32` / `fstp m32`: a signalling NaN is
/// quieted, everything else (denormals included) is kept.
#[must_use]
pub const fn copy(bits: u32) -> u32 {
    let nan = bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0;
    if nan { bits | 0x0040_0000 } else { bits }
}

/// An `f64` result (computed at the x87's 53-bit precision) stored with
/// `fstp m32`: rounded to nearest, as `f32` bits.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // the store rounds
pub fn store(x: f64) -> u32 {
    (x as f32).to_bits()
}

/// An `f32` loaded with `fld m32`, at the x87's precision.
#[must_use]
pub fn load(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_quiet_signalling_nans_only() {
        assert_eq!(copy(0x7f80_0001), 0x7fc0_0001);
        assert_eq!(copy(0xff80_0001), 0xffc0_0001);
        assert_eq!(copy(0x7fc0_0000), 0x7fc0_0000);
        assert_eq!(copy(0x7f80_0000), 0x7f80_0000); // infinity
        assert_eq!(copy(0x0000_0001), 0x0000_0001); // denormal
    }

    #[test]
    fn stores_round_to_nearest() {
        assert_eq!(store(1.0 + f64::EPSILON), 1.0f32.to_bits());
        assert_eq!(store(1e300), f32::INFINITY.to_bits());
    }
}
