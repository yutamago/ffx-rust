//! The game thread's x87 arithmetic on `f32` values, bit for bit.
//!
//! The game thread runs the x87 FPU at 53-bit precision with every
//! exception masked (control word `0x027f`). For one operation on two
//! `f32` values whose result is stored back as `f32`, that is the IEEE
//! single result: rounding the exact result to 53 bits first cannot change
//! its rounding to 24 bits. Operations here are therefore done in `f64` and
//! rounded once to `f32`, as the FPU does. What Rust arithmetic does not
//! give is the x87's NaNs:
//! - loading a signalling NaN (`fld m32`) quiets it, so even a plain copy
//!   through the FPU (`fld` / `fstp`) sets the top fraction bit;
//! - an invalid operation (`inf - inf`, `0 * inf`) gives the "real
//!   indefinite", `0xffc00000`, on every host;
//! - with two NaN operands the result is the one with the larger
//!   significand (the positive one on a tie), but a signalling NaN memory
//!   operand loses to the quiet NaN in the register (Intel SDM vol. 1,
//!   table 4-7).

/// The x87 "real indefinite" (default NaN) as `f32` bits.
pub const INDEFINITE: u32 = 0xffc0_0000;
const QUIET: u32 = 0x0040_0000;

/// Whether `f32` bits are a NaN.
#[must_use]
pub const fn is_nan(bits: u32) -> bool {
    bits & 0x7fff_ffff > 0x7f80_0000
}

/// Whether `f32` bits are a signalling NaN.
#[must_use]
pub const fn is_snan(bits: u32) -> bool {
    is_nan(bits) && bits & QUIET == 0
}

/// An `f32` after a trip through the FPU (`fld m32`, `fst m32`): the same
/// bits, except that a signalling NaN comes back quiet.
#[must_use]
pub const fn load(bits: u32) -> u32 {
    if is_nan(bits) { bits | QUIET } else { bits }
}

/// [`load`] on an `f32`.
#[must_use]
pub fn load_f32(x: f32) -> f32 {
    f32::from_bits(load(x.to_bits()))
}

/// The NaN an operation on `st` (loaded into the register) and `mem` (the
/// `m32` operand) gives, if either is a NaN.
fn propagate(st: u32, mem: u32) -> Option<u32> {
    let st = load(st);
    match (is_nan(st), is_nan(mem)) {
        (false, false) => None,
        (true, false) => Some(st),
        (false, true) => Some(load(mem)),
        (true, true) => {
            if is_snan(mem) {
                return Some(st);
            }
            let (fs, fm) = (st & 0x007f_ffff, mem & 0x007f_ffff);
            Some(if fm > fs || (fm == fs && st & 0x8000_0000 != 0) {
                mem
            } else {
                st
            })
        }
    }
}

fn finish(x: f64) -> f32 {
    #[allow(clippy::cast_possible_truncation)] // the fstp m32 rounding
    let r = x as f32;
    if r.is_nan() {
        f32::from_bits(INDEFINITE)
    } else {
        r
    }
}

/// `fld st; fadd m32; fstp m32`: `st + mem`.
#[must_use]
pub fn add(st: f32, mem: f32) -> f32 {
    match propagate(st.to_bits(), mem.to_bits()) {
        Some(n) => f32::from_bits(n),
        None => finish(f64::from(st) + f64::from(mem)),
    }
}

/// `fld st; fsub m32; fstp m32`: `st - mem`.
#[must_use]
pub fn sub(st: f32, mem: f32) -> f32 {
    match propagate(st.to_bits(), mem.to_bits()) {
        Some(n) => f32::from_bits(n),
        None => finish(f64::from(st) - f64::from(mem)),
    }
}

/// `fld st; fmul m32; fstp m32`: `st * mem`.
#[must_use]
pub fn mul(st: f32, mem: f32) -> f32 {
    match propagate(st.to_bits(), mem.to_bits()) {
        Some(n) => f32::from_bits(n),
        None => finish(f64::from(st) * f64::from(mem)),
    }
}

/// `fld st; fsub m64; fstp m32` with a constant that is not a NaN:
/// `st - c`.
#[must_use]
pub fn sub_f64(st: f32, c: f64) -> f32 {
    if st.is_nan() {
        return load_f32(st);
    }
    finish(f64::from(st) - c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nans() {
        // a copy quiets a signalling NaN only
        assert_eq!(load(0x7f80_0001), 0x7fc0_0001);
        assert_eq!(load(0x7fc0_0001), 0x7fc0_0001);
        assert_eq!(load(0x7f80_0000), 0x7f80_0000);
        // inf - inf: the indefinite
        assert_eq!(add(f32::INFINITY, f32::NEG_INFINITY).to_bits(), INDEFINITE);
        // two quiet NaNs: the larger significand
        let (a, b) = (f32::from_bits(0x7fc0_0002), f32::from_bits(0xffc0_0005));
        assert_eq!(add(a, b).to_bits(), 0xffc0_0005);
        assert_eq!(add(b, a).to_bits(), 0xffc0_0005);
        // a tie goes to the positive one
        let c = f32::from_bits(0xffc0_0002);
        assert_eq!(add(c, a).to_bits(), 0x7fc0_0002);
        assert_eq!(add(a, c).to_bits(), 0x7fc0_0002);
        // a signalling memory operand loses to the register's NaN
        assert_eq!(add(a, f32::from_bits(0x7fbf_ffff)).to_bits(), 0x7fc0_0002);
        assert_eq!(mul(1.5, 2.0), 3.0);
    }
}
