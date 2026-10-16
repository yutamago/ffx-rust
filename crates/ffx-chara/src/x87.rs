//! What the game's x87 code does to the floats it moves and compares
//! (the game thread runs at 53-bit precision, control word `0x027f`, every
//! exception masked).
//!
//! - A float the original loads with `fld m32` and stores with `fst m32`
//!   comes out the same, except that a signalling NaN comes out quiet
//!   ([`load`]). Copies made with integer moves keep every bit; the bridges
//!   copy those as `u32`.
//! - `f32` arithmetic with a `double` constant or another `f32` is rounded
//!   once to 53 bits and once more when stored as `f32`: Rust's `f64`
//!   arithmetic then `as f32` rounds the same, for the values these
//!   functions see (no exponent beyond `f64`'s range). NaNs follow the x87
//!   rules instead of SSE's ([`add`]).
//! - The comparisons follow the original's flag tests: unordered (NaN)
//!   operands set every flag, so a test reads as `<=`, `>=` or `<` only for
//!   ordered values; each caller says which way NaN goes.
//!
//! The oracle's emulator (Unicorn, QEMU 5 softfloat) does not quiet
//! signalling NaNs on `fld m32`; the CPU does. These follow the CPU, and
//! the tests below check them against the host's own x87 unit.

const QUIET: u32 = 0x0040_0000;
const SIGNIFICAND: u32 = 0x007f_ffff;

/// A float through `fld m32` and `fst m32`: signalling NaNs come out quiet.
#[must_use]
pub fn load(v: f32) -> f32 {
    if v.is_nan() {
        f32::from_bits(v.to_bits() | QUIET)
    } else {
        v
    }
}

/// Whether `v` is a signalling NaN.
#[must_use]
pub fn is_signalling(v: f32) -> bool {
    v.is_nan() && v.to_bits() & QUIET == 0
}

/// `fld m32 a; fadd m32 b; fstp m32` at 53-bit precision. NaNs: a quiet
/// NaN wins over a signalling one; of two quiet NaNs the one with the
/// larger significand, then the positive one; a signalling NaN alone comes
/// out quiet. `a` went through `fld` first, so it is never signalling.
#[must_use]
pub fn add(a: f32, b: f32) -> f32 {
    let a = load(a);
    match (a.is_nan(), b.is_nan()) {
        (true, true) if is_signalling(b) => a,
        (true, true) => {
            let (x, y) = (a.to_bits(), b.to_bits());
            if (y & SIGNIFICAND, x >> 31) > (x & SIGNIFICAND, y >> 31) {
                b
            } else {
                a
            }
        }
        (true, false) => a,
        (false, true) => load(b),
        (false, false) => {
            #[allow(clippy::cast_possible_truncation)] // the store rounds to f32
            let r = (f64::from(a) + f64::from(b)) as f32;
            r
        }
    }
}

/// `fld m32 a; fmul m64 c; fstp m32` with a constant `c` that is not NaN.
#[must_use]
pub fn mul_f64(a: f32, c: f64) -> f32 {
    let a = load(a);
    if a.is_nan() {
        return a;
    }
    #[allow(clippy::cast_possible_truncation)] // the store rounds to f32
    let r = (f64::from(a) * c) as f32;
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bit patterns: zeros, normals, denormals, infinities, quiet and
    /// signalling NaNs of both signs and several payloads.
    const BITS: [u32; 22] = [
        0x0000_0000,
        0x8000_0000,
        0x3f80_0000,
        0xbf80_0000,
        0x4049_0fd8,
        0x7f7f_ffff,
        0x0000_0001,
        0x8000_0001,
        0x0080_0000,
        0x7f80_0000,
        0xff80_0000,
        0x7fc0_0000,
        0xffc0_0000,
        0x7fc0_0001,
        0xffc0_0005,
        0x7fff_ffff,
        0x7f80_0001,
        0x7f80_0005,
        0xff80_0003,
        0xff80_4321,
        0x7fbf_ffff,
        0x5f00_0000,
    ];

    #[test]
    fn signalling_nans_come_out_quiet() {
        assert_eq!(load(f32::from_bits(0x7f80_0001)).to_bits(), 0x7fc0_0001);
        assert_eq!(load(f32::from_bits(0xff80_4321)).to_bits(), 0xffc0_4321);
        assert_eq!(load(f32::from_bits(0x7fc0_0000)).to_bits(), 0x7fc0_0000);
        assert_eq!(load(f32::from_bits(1)).to_bits(), 1);
        assert_eq!(load(-0.0).to_bits(), 0x8000_0000);
    }

    #[test]
    fn nans_propagate_like_the_x87() {
        let f = f32::from_bits;
        assert_eq!(add(1.0, 2.0), 3.0);
        assert_eq!(add(1.0e30, 1.0), 1.0e30);
        assert_eq!(add(f(0xffc0_0000), f(0x7fc0_0000)).to_bits(), 0x7fc0_0000);
        assert_eq!(add(f(0x7fc0_0001), f(0xffc0_0005)).to_bits(), 0xffc0_0005);
        assert_eq!(add(f(0x7fc0_0000), f(0x7f80_0001)).to_bits(), 0x7fc0_0000);
        assert_eq!(add(f(0x7f80_0001), f(0x7fc0_0000)).to_bits(), 0x7fc0_0001);
        assert_eq!(add(1.0, f(0xff80_0003)).to_bits(), 0xffc0_0003);
    }

    /// The host's x87 unit, where there is one: the rules above against
    /// the real instructions, at the game's control word.
    #[cfg(target_arch = "x86_64")]
    mod hardware {
        use super::{BITS, add, load, mul_f64};
        use std::arch::asm;

        static CW: u16 = 0x027f;

        fn fld_fstp(v: u32) -> u32 {
            let mut out = 0u32;
            // SAFETY: x87 instructions on locals; the stack is left empty.
            unsafe {
                asm!(
                    "fldcw word ptr [{cw}]",
                    "fld dword ptr [{v}]",
                    "fstp dword ptr [{o}]",
                    cw = in(reg) &raw const CW,
                    v = in(reg) &raw const v,
                    o = in(reg) &raw mut out,
                );
            }
            out
        }

        fn fadd(a: u32, b: u32) -> u32 {
            let mut out = 0u32;
            // SAFETY: as above.
            unsafe {
                asm!(
                    "fldcw word ptr [{cw}]",
                    "fld dword ptr [{a}]",
                    "fadd dword ptr [{b}]",
                    "fstp dword ptr [{o}]",
                    cw = in(reg) &raw const CW,
                    a = in(reg) &raw const a,
                    b = in(reg) &raw const b,
                    o = in(reg) &raw mut out,
                );
            }
            out
        }

        fn fmul64(a: u32, c: f64) -> u32 {
            let mut out = 0u32;
            // SAFETY: as above.
            unsafe {
                asm!(
                    "fldcw word ptr [{cw}]",
                    "fld dword ptr [{a}]",
                    "fmul qword ptr [{c}]",
                    "fstp dword ptr [{o}]",
                    cw = in(reg) &raw const CW,
                    a = in(reg) &raw const a,
                    c = in(reg) &raw const c,
                    o = in(reg) &raw mut out,
                );
            }
            out
        }

        #[test]
        fn loads_match_the_cpu() {
            for b in BITS {
                assert_eq!(load(f32::from_bits(b)).to_bits(), fld_fstp(b), "{b:#x}");
            }
        }

        #[test]
        fn sums_match_the_cpu() {
            let mut x = 0x2545_f491u32;
            let mut extra = Vec::new();
            for _ in 0..2000 {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                extra.push(x);
            }
            for &a in BITS.iter().chain(&extra) {
                for &b in BITS.iter().chain(&extra[..50]) {
                    let ours = add(f32::from_bits(a), f32::from_bits(b)).to_bits();
                    assert_eq!(ours, fadd(a, b), "{a:#x} + {b:#x}");
                }
                let c = f64::from_bits(0x3fa9_9999_a000_0000);
                assert_eq!(
                    mul_f64(f32::from_bits(a), c).to_bits(),
                    fmul64(a, c),
                    "{a:#x} * 0.05"
                );
            }
        }
    }
}
