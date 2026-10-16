//! The `cross` assertion helpers of the PC port.

/// `crossAssertIsValidFloat(x)` (`0x006ef2d0`): 1 unless `x` is a NaN
/// or an infinity. The original widens `x` to `f64` and asks the C
/// runtime (`_isnan`, then `_finite`); widening keeps both properties.
#[must_use]
pub fn is_valid_float(x: f32) -> u32 {
    u32::from(x.is_finite())
}
