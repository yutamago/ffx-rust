//! The game's float arithmetic and conversions, bit for bit
//! (`docs/rewrite-plan.md`, "Float model").
//!
//! The game thread runs the x87 at 53-bit precision (control word
//! `0x027f`), so its arithmetic is `f64` arithmetic: each operation rounds
//! once to a 53-bit mantissa, as an `f64` operation does. The exponent
//! range is wider (15 bits), which only matters past `f64`'s range; values
//! from `f32` and `i32` operands stay well inside it (a product of two
//! `f32`s needs 48 bits). Code that keeps intermediates on the x87 stack is
//! reproduced by computing in `f64` in the original's order and rounding to
//! `f32` exactly where the original stores a float (`fstp dword`). Plain
//! `f32` arithmetic, rounding every step to 24 bits, does not give the
//! game's results.
//!
//! Two interfaces to the same model:
//! - [`Reg`], an x87 register: [`Reg::load`] (`fld dword`), the operators
//!   (`fadd`/`fsub`/`fmul`/`fdiv` between registers, `fchs`), the memory
//!   forms ([`Reg::add_m32`] ...), [`Reg::store`] (`fstp dword`);
//! - free functions on raw `f32` bits and `f64` values: [`load`], [`add`],
//!   [`sub`], [`mul`], [`div`], [`store`].
//!
//! NaNs follow the x87 rules, not SSE's (which Rust's arithmetic and its
//! unspecified NaN results would give): loading a signalling NaN quiets it;
//! with two NaN operands the one with the larger significand wins (the
//! positive one when they tie), not the first; an invalid operation on
//! numbers gives the "indefinite" NaN (negative, quiet, no payload). A
//! memory operand is not loaded first: a signalling NaN there loses to a
//! quiet NaN in the register, whatever their significands. Every NaN is
//! built from its bits here, so the results do not depend on what the
//! compiler does with NaNs. These rules are checked against this machine's
//! own x87 (`the_model_is_the_hardware`), not against the oracle: its
//! emulated x87 leaves a loaded signalling NaN signalling (so `fld`/`fstp`
//! copies it unchanged and it loses to any quiet NaN), which the CPU does
//! not.
//!
//! The model assumes what the game thread has: the x87 at 53 bits, round to
//! nearest, and SSE (the Rust side) without flush-to-zero or
//! denormals-are-zero (`FFX.exe` never loads MXCSR).
//!
//! Float to integer goes through the C runtime's `_ftol2_sse`, which picks
//! one of two conversions by the CPU: [`ftol`].

use crate::{Global, Mem};

/// The quiet bit of an `f64` NaN.
const QUIET64: u64 = 1 << 51;
/// Significand bits of an `f64`.
const FRACTION: u64 = (1 << 52) - 1;
/// The x87 "indefinite" NaN (`0xffc00000` once stored as `f32`).
const INDEFINITE: u64 = 0xfff8_0000_0000_0000;

/// Whether the `f32` bits are a signalling NaN.
#[must_use]
pub const fn is_snan(bits: u32) -> bool {
    bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 && bits & 0x0040_0000 == 0
}

/// A float as `fld m32` loads it: exact, but a signalling NaN comes out
/// quiet.
#[must_use]
pub fn load(bits: u32) -> f64 {
    if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
        let sign = u64::from(bits >> 31) << 63;
        let frac = u64::from(bits & 0x007f_ffff) << 29;
        f64::from_bits(sign | 0x7ff0_0000_0000_0000 | QUIET64 | frac)
    } else {
        f64::from(f32::from_bits(bits))
    }
}

/// `fstp m32`: rounded to `f32` (to nearest; overflow gives infinity; a NaN
/// keeps its sign and top significand bits).
#[must_use]
pub fn store(x: f64) -> u32 {
    if x.is_nan() {
        let b = x.to_bits();
        #[allow(clippy::cast_possible_truncation)] // the top 23 bits
        let frac = ((b & FRACTION) >> 29) as u32;
        #[allow(clippy::cast_possible_truncation)]
        let sign = ((b >> 63) as u32) << 31;
        sign | 0x7fc0_0000 | frac
    } else {
        #[allow(clippy::cast_possible_truncation)] // the store rounds
        let f = x as f32;
        f.to_bits()
    }
}

/// The NaN an x87 operation on `a` and `b` gives when one of them is NaN:
/// of two, the larger significand, then the positive one (SSE, and so
/// Rust, take the first operand instead). Operands come from [`load`], so
/// they are quiet.
#[must_use]
pub fn nan(a: f64, b: f64) -> Option<f64> {
    match (a.is_nan(), b.is_nan()) {
        (true, true) => {
            let (x, y) = (a.to_bits(), b.to_bits());
            let pick = match (x & FRACTION).cmp(&(y & FRACTION)) {
                std::cmp::Ordering::Greater => x,
                std::cmp::Ordering::Less => y,
                std::cmp::Ordering::Equal => x.min(y),
            };
            Some(f64::from_bits(pick))
        }
        (true, false) => Some(a),
        (false, true) => Some(b),
        (false, false) => None,
    }
}

/// `op` on two register values: the x87's NaN for NaN operands, the
/// indefinite NaN for an invalid operation on numbers.
fn arith(a: f64, b: f64, op: fn(f64, f64) -> f64) -> f64 {
    if let Some(n) = nan(a, b) {
        return n;
    }
    let r = op(a, b);
    if r.is_nan() {
        f64::from_bits(INDEFINITE)
    } else {
        r
    }
}

/// `a + b` on the x87 at 53 bits.
#[must_use]
pub fn add(a: f64, b: f64) -> f64 {
    arith(a, b, |x, y| x + y)
}

/// `a - b` on the x87 at 53 bits.
#[must_use]
pub fn sub(a: f64, b: f64) -> f64 {
    arith(a, b, |x, y| x - y)
}

/// `a * b` on the x87 at 53 bits.
#[must_use]
pub fn mul(a: f64, b: f64) -> f64 {
    arith(a, b, |x, y| x * y)
}

/// `a / b` on the x87 at 53 bits.
#[must_use]
pub fn div(a: f64, b: f64) -> f64 {
    arith(a, b, |x, y| x / y)
}

/// `op` with a memory operand `m` (`f32` bits).
fn arith_m32(a: f64, m: u32, op: fn(f64, f64) -> f64) -> f64 {
    if is_snan(m) && a.is_nan() {
        return a;
    }
    arith(a, load(m), op)
}

/// An x87 register at 53-bit precision.
#[derive(Debug, Clone, Copy)]
pub struct Reg(f64);

impl Reg {
    /// `fldz`.
    pub const ZERO: Self = Self(0.0);
    /// `fld1`.
    pub const ONE: Self = Self(1.0);

    /// `fld dword`: an `f32` onto the stack (exact; a signalling NaN becomes
    /// quiet).
    #[must_use]
    pub fn load(x: f32) -> Self {
        Self(load(x.to_bits()))
    }

    /// A constant the original loads as a double (`fld qword`) or computes
    /// exactly.
    #[must_use]
    pub const fn constant(c: f64) -> Self {
        Self(c)
    }

    /// The value held.
    #[must_use]
    pub const fn value(self) -> f64 {
        self.0
    }

    /// `fadd dword [m]`.
    #[must_use]
    pub fn add_m32(self, m: f32) -> Self {
        Self(arith_m32(self.0, m.to_bits(), |x, y| x + y))
    }

    /// `fsub dword [m]`: `self - m`.
    #[must_use]
    pub fn sub_m32(self, m: f32) -> Self {
        Self(arith_m32(self.0, m.to_bits(), |x, y| x - y))
    }

    /// `fsubr dword [m]`: `m - self`.
    #[must_use]
    pub fn subr_m32(self, m: f32) -> Self {
        Self(arith_m32(self.0, m.to_bits(), |x, y| y - x))
    }

    /// `fmul dword [m]`.
    #[must_use]
    pub fn mul_m32(self, m: f32) -> Self {
        Self(arith_m32(self.0, m.to_bits(), |x, y| x * y))
    }

    /// `fdiv dword [m]`: `self / m`.
    #[must_use]
    pub fn div_m32(self, m: f32) -> Self {
        Self(arith_m32(self.0, m.to_bits(), |x, y| x / y))
    }

    /// `fdivr dword [m]`: `m / self`.
    #[must_use]
    pub fn divr_m32(self, m: f32) -> Self {
        Self(arith_m32(self.0, m.to_bits(), |x, y| y / x))
    }

    /// `fstp dword`: rounded to `f32`.
    #[must_use]
    pub fn store(self) -> f32 {
        f32::from_bits(store(self.0))
    }
}

impl std::ops::Add for Reg {
    type Output = Self;
    /// `fadd`.
    fn add(self, b: Self) -> Self {
        Self(add(self.0, b.0))
    }
}

impl std::ops::Sub for Reg {
    type Output = Self;
    /// `fsub`: `self - b`.
    fn sub(self, b: Self) -> Self {
        Self(sub(self.0, b.0))
    }
}

impl std::ops::Mul for Reg {
    type Output = Self;
    /// `fmul`.
    fn mul(self, b: Self) -> Self {
        Self(mul(self.0, b.0))
    }
}

impl std::ops::Div for Reg {
    type Output = Self;
    /// `fdiv`: `self / b`.
    fn div(self, b: Self) -> Self {
        Self(div(self.0, b.0))
    }
}

impl std::ops::Neg for Reg {
    type Output = Self;
    /// `fchs`: the sign flipped (NaNs too).
    fn neg(self) -> Self {
        Self(f64::from_bits(self.0.to_bits() ^ (1 << 63)))
    }
}

/// `fld dword` then `fstp dword`: the value copied through the stack (only
/// a signalling NaN changes: it becomes quiet).
#[must_use]
pub fn round_trip(x: f32) -> f32 {
    Reg::load(x).store()
}

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

    #[test]
    fn the_two_interfaces_agree() {
        let bits = [
            0x7f80_0001u32,
            0xffc0_0002,
            0x3f80_0000,
            0x0000_0001,
            0x7f7f_ffff,
        ];
        for &x in &bits {
            for &y in &bits {
                let (a, b) = (f32::from_bits(x), f32::from_bits(y));
                assert_eq!(
                    (Reg::load(a) * Reg::load(b) - Reg::load(a))
                        .store()
                        .to_bits(),
                    store(sub(mul(load(x), load(y)), load(x)))
                );
            }
        }
    }

    #[test]
    fn sums_round_once_per_operation_at_53_bits() {
        // 1 + 2^-24 + 2^-60: at 53 bits the last term is lost and the tie
        // rounds to even (1.0); at 64 bits it would round up.
        let a = Reg::load(1.0) + Reg::load(2f32.powi(-24)) + Reg::load(2f32.powi(-60));
        assert_eq!(a.store(), 1.0);
        // x² - y for x = 1 + 2^-23, y = 1 + 2^-22: 2^-46, where f32 steps
        // (x² rounded first) give 0
        let x = 1.0 + 2f32.powi(-23);
        let y = 1.0 + 2f32.powi(-22);
        assert_eq!(
            (Reg::load(x) * Reg::load(x) - Reg::load(y)).store(),
            2f32.powi(-46)
        );
        assert_eq!(x * x - y, 0.0);
    }

    #[test]
    fn nans_follow_the_x87() {
        let snan = f32::from_bits(0x7f80_0001);
        assert_eq!(round_trip(snan).to_bits(), 0x7fc0_0001);
        let a = Reg::load(f32::from_bits(0x7fc0_0001));
        let b = Reg::load(f32::from_bits(0xffc0_0002));
        assert_eq!((a + b).store().to_bits(), 0xffc0_0002);
        assert_eq!((b + a).store().to_bits(), 0xffc0_0002);
        let c = Reg::load(f32::from_bits(0xffc0_0001));
        assert_eq!((c - a).store().to_bits(), 0x7fc0_0001);
        let inf = Reg::load(f32::INFINITY);
        assert_eq!((inf - inf).store().to_bits(), 0xffc0_0000);
        assert_eq!((inf * Reg::ZERO).store().to_bits(), 0xffc0_0000);
        assert_eq!((-a).store().to_bits(), 0xffc0_0001);
        // a signalling NaN in memory loses to the register's quiet NaN
        let q = Reg::load(f32::from_bits(0x7fc0_0000));
        assert_eq!(
            q.add_m32(f32::from_bits(0x7f80_0001)).store().to_bits(),
            0x7fc0_0000
        );
        assert_eq!(
            (q + Reg::load(f32::from_bits(0x7f80_0001)))
                .store()
                .to_bits(),
            0x7fc0_0001
        );
        assert_eq!(
            Reg::ONE
                .mul_m32(f32::from_bits(0x7f80_0001))
                .store()
                .to_bits(),
            0x7fc0_0001
        );
    }

    /// The model against this machine's own x87 at the game's control word
    /// (the oracle's emulated x87 keeps a loaded signalling NaN signalling,
    /// so it cannot settle NaN cases; the hardware can).
    #[cfg(target_arch = "x86_64")]
    #[test]
    fn the_model_is_the_hardware() {
        /// `(((a0 b0 + a1 b1) + a2 b2) + a3 [b3]) - a4`, the last product
        /// with a memory operand, on the x87 at control word 0x027f.
        fn hardware(a: &[f32; 5], b: &[f32; 4]) -> f32 {
            let cw: u16 = 0x027f;
            let mut old: u16 = 0;
            let mut out = 0f32;
            // SAFETY: balanced x87 pushes and pops; the control word is
            // restored; the pointers are to locals.
            unsafe {
                std::arch::asm!(
                    "fnstcw word ptr [{old}]",
                    "fldcw word ptr [{cw}]",
                    "fld dword ptr [{a}]",
                    "fld dword ptr [{b}]",
                    "fmulp st(1), st(0)",
                    "fld dword ptr [{a} + 4]",
                    "fld dword ptr [{b} + 4]",
                    "fmulp st(1), st(0)",
                    "faddp st(1), st(0)",
                    "fld dword ptr [{a} + 8]",
                    "fld dword ptr [{b} + 8]",
                    "fmulp st(1), st(0)",
                    "faddp st(1), st(0)",
                    "fld dword ptr [{a} + 12]",
                    "fmul dword ptr [{b} + 12]",
                    "faddp st(1), st(0)",
                    "fld dword ptr [{a} + 16]",
                    "fsubp st(1), st(0)",
                    "fstp dword ptr [{out}]",
                    "fldcw word ptr [{old}]",
                    old = in(reg) &raw mut old,
                    cw = in(reg) &raw const cw,
                    a = in(reg) a.as_ptr(),
                    b = in(reg) b.as_ptr(),
                    out = in(reg) &raw mut out,
                    out("st(0)") _, out("st(1)") _, out("st(2)") _, out("st(3)") _,
                    out("st(4)") _, out("st(5)") _, out("st(6)") _, out("st(7)") _,
                );
            }
            out
        }
        fn model(a: &[f32; 5], b: &[f32; 4]) -> f32 {
            (Reg::load(a[0]) * Reg::load(b[0])
                + Reg::load(a[1]) * Reg::load(b[1])
                + Reg::load(a[2]) * Reg::load(b[2])
                + Reg::load(a[3]).mul_m32(b[3])
                - Reg::load(a[4]))
            .store()
        }
        let mut seed = 0x1234_5678u32;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        let mut float = || {
            let r = next();
            f32::from_bits(match r % 8 {
                // NaNs of every kind
                0 => next() | 0x7f80_0000,
                1 => next() & 0x807f_ffff | 0x7f80_0000,
                2 => {
                    [0, 0x8000_0000, 0x7f80_0000, 0xff80_0000, 1, 0x7f7f_ffff][next() as usize % 6]
                }
                // any bits
                3 => next(),
                // ordinary magnitudes
                _ => next() & 0x807f_ffff | (next() % 40 + 107) << 23,
            })
        };
        for n in 0..200_000 {
            let a = [float(), float(), float(), float(), float()];
            let b = [float(), float(), float(), float()];
            let (h, m) = (hardware(&a, &b), model(&a, &b));
            assert_eq!(h.to_bits(), m.to_bits(), "case {n}: {a:?} {b:?}");
        }
        // NaNs that differ only in sign, in registers and from memory
        let (p, q) = (f32::from_bits(0x7fc0_0005), f32::from_bits(0xffc0_0005));
        let (sp, sq) = (f32::from_bits(0x7f80_0005), f32::from_bits(0xff80_0005));
        for (x, y) in [(p, q), (q, p), (sp, q), (q, sp), (sq, p), (p, sq)] {
            for (a, b) in [
                ([x, 0.0, 0.0, 0.0, y], [1.0, 0.0, 0.0, 0.0]),
                ([x, 0.0, 0.0, 1.0, 0.0], [1.0, 0.0, 0.0, y]),
                ([x, y, 0.0, 0.0, 0.0], [1.0, 1.0, 0.0, 0.0]),
            ] {
                assert_eq!(
                    hardware(&a, &b).to_bits(),
                    model(&a, &b).to_bits(),
                    "{a:?} {b:?}"
                );
            }
        }
        // the double-rounding case: 1 + 2^-24 + 2^-60 rounds to 1.0 at 53 bits
        let a = [1.0, 1.0, 1.0, 0.0, 0.0];
        let b = [1.0, 2f32.powi(-24), 2f32.powi(-60), 0.0];
        assert_eq!(hardware(&a, &b), 1.0);
        assert_eq!(model(&a, &b), 1.0);
    }
}
