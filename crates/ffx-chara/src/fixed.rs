//! The PS2 fixed-point helpers the character code uses: 4.12 and 8.8
//! values to floats, and 12-bit angles (4096 to the turn).

/// `Fixed12_ToFloat(v)` (`0x00839dd0`): the low 16 bits, signed, / 4096.
/// Exact, like the original's `fild` and multiply by 2^-12.
#[must_use]
pub fn fixed12_to_float(v: u32) -> f32 {
    #[allow(clippy::cast_possible_truncation)] // movsx of the low half
    let i = (v as u16).cast_signed();
    f32::from(i) / 4096.0
}

/// `Fixed8_ToFloat(v)` (`0x00839e00`): the low 16 bits, signed, / 256.
#[must_use]
pub fn fixed8_to_float(v: u32) -> f32 {
    #[allow(clippy::cast_possible_truncation)] // movsx of the low half
    let i = (v as u16).cast_signed();
    f32::from(i) / 256.0
}

/// `Angle12_Lerp(a, b, t)` (`0x00839e30`): from `a` toward `b` the short
/// way round, `t` in 8.8 fixed; not wrapped afterwards.
#[must_use]
pub const fn angle12_lerp(a: i32, b: i32, t: i32) -> i32 {
    let mut a = a & 0xfff;
    let b = b & 0xfff;
    let mut d = b - a;
    if d >= 0x800 {
        a += 0x1000;
        d = b - a;
    }
    if d <= -0x800 {
        d += 0x1000;
    }
    (d.wrapping_mul(t) >> 8).wrapping_add(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed() {
        assert_eq!(fixed12_to_float(0x1000), 1.0);
        assert_eq!(fixed12_to_float(0xffff_8000), -8.0);
        assert_eq!(fixed8_to_float(0x0180), 1.5);
    }

    #[test]
    fn angles_take_the_short_way() {
        assert_eq!(angle12_lerp(0, 0x100, 0x80), 0x80);
        assert_eq!(angle12_lerp(0, 0xf00, 0x100), 0xf00);
        assert_eq!(angle12_lerp(0xf00, 0x100, 0x80), 0xf00 + 0x100);
    }
}
