//! Scalar SSE arithmetic with the CPU's NaN results (`addss`, `subss`,
//! `mulss`, `divss`): a NaN in the destination operand (`a`) wins, quieted;
//! else one in the source (`b`), quieted; an invalid operation on numbers
//! (`inf - inf`, `0 * inf`, `0 / 0`, `inf / inf`) gives the "indefinite"
//! NaN `0xffc00000`. Rust's operators leave which NaN comes out to the
//! compiler and the target; the ports of straight-line SSE code use these
//! so the bits match the original's everywhere.

/// The quiet bit of an `f32` NaN.
const QUIET: u32 = 0x0040_0000;
/// The x86 "indefinite" NaN.
const INDEFINITE: u32 = 0xffc0_0000;

fn sse(a: f32, b: f32, r: f32) -> f32 {
    if a.is_nan() {
        f32::from_bits(a.to_bits() | QUIET)
    } else if b.is_nan() {
        f32::from_bits(b.to_bits() | QUIET)
    } else if r.is_nan() {
        f32::from_bits(INDEFINITE)
    } else {
        r
    }
}

/// `addss a, b`.
#[must_use]
pub fn add(a: f32, b: f32) -> f32 {
    sse(a, b, a + b)
}

/// `subss a, b`.
#[must_use]
pub fn sub(a: f32, b: f32) -> f32 {
    sse(a, b, a - b)
}

/// `mulss a, b`.
#[must_use]
pub fn mul(a: f32, b: f32) -> f32 {
    sse(a, b, a * b)
}

/// `divss a, b`.
#[must_use]
pub fn div(a: f32, b: f32) -> f32 {
    sse(a, b, a / b)
}

/// `cvtdq2ps`/`cvtsi2ss` of a word: the signed integer rounded to the
/// nearest `f32` (ties to even, the default rounding).
#[must_use]
#[allow(clippy::cast_precision_loss)] // the rounding is the instruction's
pub fn cvt(x: u32) -> f32 {
    x.cast_signed() as f32
}

/// `cvttss2si`: toward 0, `0x8000_0000` for a NaN or a value out of
/// range.
#[must_use]
pub fn cvtt(x: f32) -> i32 {
    if x.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&x) {
        i32::MIN
    } else {
        #[allow(clippy::cast_possible_truncation)] // in range: toward 0
        let n = x as i32;
        n
    }
}

#[cfg(test)]
mod tests {
    use super::{add, div, mul, sub};

    fn bits(x: f32) -> u32 {
        x.to_bits()
    }

    #[test]
    fn nan_rules() {
        let qa = f32::from_bits(0x7fc0_0001);
        let sb = f32::from_bits(0xff80_0002);
        assert_eq!(bits(add(qa, sb)), 0x7fc0_0001);
        assert_eq!(bits(add(sb, qa)), 0xffc0_0002);
        assert_eq!(bits(mul(1.0, sb)), 0xffc0_0002);
        assert_eq!(bits(mul(0.0, f32::INFINITY)), 0xffc0_0000);
        assert_eq!(bits(sub(f32::INFINITY, f32::INFINITY)), 0xffc0_0000);
        assert_eq!(bits(div(0.0, 0.0)), 0xffc0_0000);
        assert_eq!(bits(add(-0.0, -0.0)), 0x8000_0000);
        assert_eq!(bits(mul(-0.0, 5.0)), 0x8000_0000);
    }
}
