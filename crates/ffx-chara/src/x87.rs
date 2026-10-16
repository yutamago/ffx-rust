//! What the game's x87 code does to the floats it moves and computes
//! (the game thread runs at 53-bit precision, control word `0x027f`, every
//! exception masked).
//!
//! - A float the original loads with `fld m32` and stores with `fst m32`
//!   comes out the same, except that a signalling NaN comes out quiet
//!   ([`load`]). Copies made with integer moves keep every bit; the bridges
//!   copy those as `u32`.
//! - Arithmetic on the stack is [`Reg`]: an `f64`, since at 53-bit
//!   precision every x87 result is rounded to `double`'s significand (and
//!   these functions never leave `double`'s exponent range), then once more
//!   to `f32` when stored. NaNs follow the x87 rules, not SSE's: a quiet
//!   NaN wins over a signalling one, of two quiet NaNs the one with the
//!   larger significand, then the positive one; an invalid operation gives
//!   the negative "indefinite" NaN.
//! - The comparisons follow the original's flag tests: unordered (NaN)
//!   operands set every flag, so a test reads as `<=`, `>=` or `<` only for
//!   ordered values; each caller says which way NaN goes.
//!
//! The oracle's emulator (Unicorn, QEMU 5 softfloat) does not quiet
//! signalling NaNs on `fld m32`; the CPU does. These follow the CPU, and
//! the tests below check them against the host's own x87 unit.

const QUIET: u32 = 0x0040_0000;
const QUIET64: u64 = 0x0008_0000_0000_0000;
const INDEFINITE: u64 = 0xfff8_0000_0000_0000;

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

/// A value on the x87 stack (at 53-bit precision).
#[derive(Debug, Clone, Copy)]
pub struct Reg(f64);

/// An operand straight from memory (`fmul m32`, `fadd m32`): it may be a
/// signalling NaN, which a value already on the stack never is.
#[derive(Debug, Clone, Copy)]
struct Operand {
    v: f64,
    signalling: bool,
}

/// An `f32` widened bit by bit: NaN payloads stay in the top of the
/// significand, quiet bit as given.
fn widen(v: f32) -> f64 {
    if v.is_nan() {
        let b = v.to_bits();
        let sign = u64::from(b >> 31) << 63;
        let frac = u64::from(b & 0x007f_ffff) << 29;
        f64::from_bits(sign | 0x7ff0_0000_0000_0000 | frac)
    } else {
        f64::from(v)
    }
}

/// The NaN an x87 operation on `a` and `b` gives, if either is one.
fn pick_nan(a: Operand, b: Operand) -> Option<f64> {
    let quiet = |x: f64| f64::from_bits(x.to_bits() | QUIET64);
    match (a.v.is_nan(), b.v.is_nan()) {
        (false, false) => None,
        (true, false) => Some(quiet(a.v)),
        (false, true) => Some(quiet(b.v)),
        (true, true) if a.signalling != b.signalling => {
            Some(quiet(if a.signalling { b.v } else { a.v }))
        }
        (true, true) => {
            let (x, y) = (a.v.to_bits(), b.v.to_bits());
            let sig = |n: u64| n & 0x000f_ffff_ffff_ffff;
            let pick = if (sig(y), x >> 63) > (sig(x), y >> 63) {
                b.v
            } else {
                a.v
            };
            Some(quiet(pick))
        }
    }
}

impl Reg {
    /// `fld m32`.
    #[must_use]
    pub fn load(v: f32) -> Self {
        Self(widen(load(v)))
    }

    /// `fld m64` of a constant that is not NaN.
    #[must_use]
    pub const fn constant(c: f64) -> Self {
        Self(c)
    }

    /// `fstp m32`: rounded to `f32`; NaNs keep the top of their
    /// significand.
    #[must_use]
    pub fn store(self) -> f32 {
        if self.0.is_nan() {
            let b = self.0.to_bits();
            #[allow(clippy::cast_possible_truncation)] // the top bits
            let bits =
                ((b >> 63) as u32) << 31 | 0x7f80_0000 | QUIET | ((b >> 29) as u32 & 0x007f_ffff);
            return f32::from_bits(bits);
        }
        #[allow(clippy::cast_possible_truncation)] // the store rounds to f32
        let r = self.0 as f32;
        r
    }

    fn op(self, b: Operand, f: impl Fn(f64, f64) -> f64) -> Self {
        let a = Operand {
            v: self.0,
            signalling: false,
        };
        if let Some(n) = pick_nan(a, b) {
            return Self(n);
        }
        let r = f(self.0, b.v);
        Self(if r.is_nan() {
            f64::from_bits(INDEFINITE)
        } else {
            r
        })
    }

    fn reg(r: Self) -> Operand {
        Operand {
            v: r.0,
            signalling: false,
        }
    }

    fn mem(m: f32) -> Operand {
        Operand {
            v: widen(m),
            signalling: is_signalling(m),
        }
    }

    /// `fadd` with another register.
    #[must_use]
    pub fn add_st(self, b: Self) -> Self {
        self.op(Self::reg(b), |x, y| x + y)
    }

    /// `fadd m32`.
    #[must_use]
    pub fn add_m32(self, m: f32) -> Self {
        self.op(Self::mem(m), |x, y| x + y)
    }

    /// `fsub m32`: `self - m`.
    #[must_use]
    pub fn sub_m32(self, m: f32) -> Self {
        self.op(Self::mem(m), |x, y| x - y)
    }

    /// `fmul` with another register.
    #[must_use]
    pub fn mul_st(self, b: Self) -> Self {
        self.op(Self::reg(b), |x, y| x * y)
    }

    /// `fmul m32`.
    #[must_use]
    pub fn mul_m32(self, m: f32) -> Self {
        self.op(Self::mem(m), |x, y| x * y)
    }

    /// `fsub` with another register: `self - b`.
    #[must_use]
    pub fn sub_st(self, b: Self) -> Self {
        self.op(Self::reg(b), |x, y| x - y)
    }

    /// `fdiv m32`: `self / m`.
    #[must_use]
    pub fn div_m32(self, m: f32) -> Self {
        self.op(Self::mem(m), |x, y| x / y)
    }

    /// `fdiv` with another register: `self / b`.
    #[must_use]
    pub fn div_st(self, b: Self) -> Self {
        self.op(Self::reg(b), |x, y| x / y)
    }

    /// `fabs`.
    #[must_use]
    pub fn abs(self) -> Self {
        Self(f64::from_bits(self.0.to_bits() & !(1 << 63)))
    }

    /// The value, for comparisons.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// `fld m32 a; fadd m32 b; fstp m32`.
#[must_use]
pub fn add(a: f32, b: f32) -> f32 {
    Reg::load(a).add_m32(b).store()
}

/// `fld m32 a; fmul m64 c; fstp m32` with a constant `c` that is not NaN.
#[must_use]
pub fn mul_f64(a: f32, c: f64) -> f32 {
    Reg::load(a).mul_st(Reg::constant(c)).store()
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
        assert_eq!(add(f32::INFINITY, f32::NEG_INFINITY).to_bits(), 0xffc0_0000);
    }

    /// The host's x87 unit, where there is one: the rules above against
    /// the real instructions, at the game's control word.
    #[cfg(target_arch = "x86_64")]
    mod hardware {
        use super::{BITS, Reg, add, load, mul_f64};
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

        /// `fld a; fsub m32 b; fmul m32 c; fstp`.
        fn fsub_fmul(a: u32, b: u32, c: u32) -> u32 {
            let mut out = 0u32;
            // SAFETY: as above.
            unsafe {
                asm!(
                    "fldcw word ptr [{cw}]",
                    "fld dword ptr [{a}]",
                    "fsub dword ptr [{b}]",
                    "fmul dword ptr [{c}]",
                    "fstp dword ptr [{o}]",
                    cw = in(reg) &raw const CW,
                    a = in(reg) &raw const a,
                    b = in(reg) &raw const b,
                    c = in(reg) &raw const c,
                    o = in(reg) &raw mut out,
                );
            }
            out
        }

        /// `fld a; fld b; fmulp; fstp` (two registers).
        fn fmul_regs(a: u32, b: u32) -> u32 {
            let mut out = 0u32;
            // SAFETY: as above.
            unsafe {
                asm!(
                    "fldcw word ptr [{cw}]",
                    "fld dword ptr [{a}]",
                    "fld dword ptr [{b}]",
                    "fmulp st(1), st",
                    "fstp dword ptr [{o}]",
                    cw = in(reg) &raw const CW,
                    a = in(reg) &raw const a,
                    b = in(reg) &raw const b,
                    o = in(reg) &raw mut out,
                );
            }
            out
        }

        /// `fld a; fdivr m64 c; fdiv m32 b; fstp`: `c / a / b`.
        fn fdivr_fdiv(a: u32, c: f64, b: u32) -> u32 {
            let mut out = 0u32;
            // SAFETY: as above.
            unsafe {
                asm!(
                    "fldcw word ptr [{cw}]",
                    "fld dword ptr [{a}]",
                    "fdivr qword ptr [{c}]",
                    "fdiv dword ptr [{b}]",
                    "fstp dword ptr [{o}]",
                    cw = in(reg) &raw const CW,
                    a = in(reg) &raw const a,
                    c = in(reg) &raw const c,
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

        fn values() -> Vec<u32> {
            let mut x = 0x2545_f491u32;
            let mut v = BITS.to_vec();
            for _ in 0..600 {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                v.push(x);
            }
            v
        }

        #[test]
        fn loads_match_the_cpu() {
            for b in values() {
                assert_eq!(load(f32::from_bits(b)).to_bits(), fld_fstp(b), "{b:#x}");
            }
        }

        #[test]
        fn arithmetic_matches_the_cpu() {
            let f = f32::from_bits;
            let v = values();
            for &a in &v {
                for &b in &v[..80] {
                    assert_eq!(add(f(a), f(b)).to_bits(), fadd(a, b), "{a:#x} + {b:#x}");
                    let r = Reg::load(f(a)).mul_st(Reg::load(f(b))).store();
                    assert_eq!(r.to_bits(), fmul_regs(a, b), "{a:#x} * {b:#x}");
                    for &c in &v[..24] {
                        let r = Reg::load(f(a)).sub_m32(f(b)).mul_m32(f(c)).store();
                        assert_eq!(
                            r.to_bits(),
                            fsub_fmul(a, b, c),
                            "({a:#x} - {b:#x}) * {c:#x}"
                        );
                    }
                }
                let c = f64::from_bits(0x3fa9_9999_a000_0000);
                assert_eq!(mul_f64(f(a), c).to_bits(), fmul64(a, c), "{a:#x} * 0.05");
                for &b in &v[..40] {
                    let r = Reg::constant(1.0e4)
                        .div_st(Reg::load(f(a)))
                        .div_m32(f(b))
                        .store();
                    assert_eq!(
                        r.to_bits(),
                        fdivr_fdiv(a, 1.0e4, b),
                        "1e4 / {a:#x} / {b:#x}"
                    );
                }
            }
        }
    }
}
