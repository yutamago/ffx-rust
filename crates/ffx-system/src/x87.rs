//! The game's float arithmetic, bit for bit (`docs/rewrite-plan.md`,
//! "Float model").
//!
//! The game thread runs x87 with control word `0x027f`: every operation
//! rounds its result to 53 bits, to nearest, in an 80-bit register with the
//! extended exponent range. For the math here (sums and products of a few
//! `f32`s) that is IEEE double arithmetic exactly: a product of two `f32`s
//! needs 48 bits, and no intermediate can leave the `f64` range, so the
//! wider exponent never matters. [`Reg`] is a register holding such a value;
//! [`load`] is `fld dword`, [`Reg::store`] is `fstp dword` (round to `f32`).
//!
//! Plain `f32` arithmetic, rounding every step to 24 bits, does not give the
//! game's results; neither does a sum computed in another order.
//!
//! NaNs follow the x87 rules, not SSE's (which Rust's arithmetic and its
//! unspecified NaN results would give): loading a signalling NaN quiets it;
//! with two NaN operands the one with the larger significand wins (the
//! positive one when they tie), not the first; an invalid operation on
//! numbers gives the "indefinite" NaN (negative, quiet, no payload). A
//! memory operand (`fmul dword [m]`, [`Reg::mul_m32`]) is not loaded first:
//! a signalling NaN there loses to a quiet NaN in the register, whatever
//! their significands. Every NaN is built from its bits here, so the
//! results do not depend on what the compiler does with NaNs.
//!
//! The NaN rules are checked against this machine's own x87
//! (`the_model_is_the_hardware`), not against the oracle: its emulated x87
//! leaves a loaded signalling NaN signalling (so `fld`/`fstp` copies it
//! unchanged and it loses to any quiet NaN), which the hardware does not.
//!
//! The model assumes what the game thread has: the x87 at 53 bits, round to
//! nearest, and SSE (the Rust side) without flush-to-zero or
//! denormals-are-zero (`FFX.exe` never loads MXCSR).

/// The quiet bit of an `f64` NaN.
const QUIET: u64 = 1 << 51;
/// Significand bits of an `f64`.
const FRACTION: u64 = (1 << 52) - 1;
/// The x87 "indefinite" NaN (`0xffc00000` once stored as `f32`).
const INDEFINITE: u64 = 0xfff8_0000_0000_0000;

/// An x87 register at 53-bit precision.
#[derive(Debug, Clone, Copy)]
pub struct Reg(f64);

/// `fld dword`: an `f32` onto the stack (exact; a signalling NaN becomes
/// quiet).
#[must_use]
pub fn load(x: f32) -> Reg {
    if x.is_nan() {
        let b = x.to_bits();
        let sign = u64::from(b >> 31) << 63;
        let frac = u64::from(b & 0x007f_ffff) << 29;
        Reg(f64::from_bits(sign | 0x7ff0_0000_0000_0000 | QUIET | frac))
    } else {
        Reg(f64::from(x))
    }
}

/// `fldz`.
pub const ZERO: Reg = Reg(0.0);
/// `fld1`.
pub const ONE: Reg = Reg(1.0);

/// The NaN an operation on `a` and `b` gives, if any.
fn nan(a: f64, b: f64) -> Option<f64> {
    match (a.is_nan(), b.is_nan()) {
        (false, false) => None,
        (true, false) => Some(a),
        (false, true) => Some(b),
        (true, true) => {
            let (x, y) = (a.to_bits(), b.to_bits());
            // larger significand; on a tie, the positive one
            let pick = match (x & FRACTION).cmp(&(y & FRACTION)) {
                std::cmp::Ordering::Greater => x,
                std::cmp::Ordering::Less => y,
                std::cmp::Ordering::Equal => x.min(y),
            };
            Some(f64::from_bits(pick))
        }
    }
}

/// `op` on numbers, or the x87's NaN.
fn arith(a: Reg, b: Reg, op: fn(f64, f64) -> f64) -> Reg {
    if let Some(n) = nan(a.0, b.0) {
        return Reg(n);
    }
    let r = op(a.0, b.0);
    Reg(if r.is_nan() {
        f64::from_bits(INDEFINITE)
    } else {
        r
    })
}

/// Whether `x` is a signalling NaN.
fn signalling(x: f32) -> bool {
    x.is_nan() && x.to_bits() & 0x0040_0000 == 0
}

/// `op` with a memory operand `m`.
fn arith_m32(a: Reg, m: f32, op: fn(f64, f64) -> f64) -> Reg {
    if signalling(m) && a.0.is_nan() {
        return a;
    }
    arith(a, load(m), op)
}

impl std::ops::Add for Reg {
    type Output = Self;
    /// `fadd`.
    fn add(self, b: Self) -> Self {
        arith(self, b, |x, y| x + y)
    }
}

impl std::ops::Sub for Reg {
    type Output = Self;
    /// `fsub`: `self - b`.
    fn sub(self, b: Self) -> Self {
        arith(self, b, |x, y| x - y)
    }
}

impl std::ops::Mul for Reg {
    type Output = Self;
    /// `fmul`.
    fn mul(self, b: Self) -> Self {
        arith(self, b, |x, y| x * y)
    }
}

impl std::ops::Neg for Reg {
    type Output = Self;
    /// `fchs`: the sign flipped (NaNs too).
    fn neg(self) -> Self {
        Reg(f64::from_bits(self.0.to_bits() ^ (1 << 63)))
    }
}

impl Reg {
    /// `fadd dword [m]`.
    #[must_use]
    pub fn add_m32(self, m: f32) -> Self {
        arith_m32(self, m, |x, y| x + y)
    }

    /// `fmul dword [m]`.
    #[must_use]
    pub fn mul_m32(self, m: f32) -> Self {
        arith_m32(self, m, |x, y| x * y)
    }

    /// `fstp dword`: rounded to `f32` (to nearest; overflow gives infinity;
    /// a NaN keeps its sign and top significand bits).
    #[must_use]
    pub fn store(self) -> f32 {
        if self.0.is_nan() {
            let b = self.0.to_bits();
            #[allow(clippy::cast_possible_truncation)] // the top 23 bits
            let frac = ((b & FRACTION) >> 29) as u32;
            #[allow(clippy::cast_possible_truncation)]
            let sign = ((b >> 63) as u32) << 31;
            f32::from_bits(sign | 0x7fc0_0000 | frac)
        } else {
            #[allow(clippy::cast_possible_truncation)] // the rounding fstp does
            let r = self.0 as f32;
            r
        }
    }
}

/// `fld dword` then `fstp dword`: the value copied through the stack (only
/// a signalling NaN changes: it becomes quiet).
#[must_use]
pub fn round_trip(x: f32) -> f32 {
    load(x).store()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_round_once_per_operation_at_53_bits() {
        // 1 + 2^-24 + 2^-60: at 53 bits the last term is lost and the tie
        // rounds to even (1.0); at 64 bits it would round up.
        let a = load(1.0) + load(2f32.powi(-24)) + load(2f32.powi(-60));
        assert_eq!(a.store(), 1.0);
        // x² - y for x = 1 + 2^-23, y = 1 + 2^-22: 2^-46, where f32 steps
        // (x² rounded first) give 0
        let x = 1.0 + 2f32.powi(-23);
        let y = 1.0 + 2f32.powi(-22);
        assert_eq!((load(x) * load(x) - load(y)).store(), 2f32.powi(-46));
        assert_eq!(x * x - y, 0.0);
    }

    #[test]
    fn nans_follow_the_x87() {
        let snan = f32::from_bits(0x7f80_0001);
        assert_eq!(round_trip(snan).to_bits(), 0x7fc0_0001);
        let a = load(f32::from_bits(0x7fc0_0001));
        let b = load(f32::from_bits(0xffc0_0002));
        assert_eq!((a + b).store().to_bits(), 0xffc0_0002);
        assert_eq!((b + a).store().to_bits(), 0xffc0_0002);
        let c = load(f32::from_bits(0xffc0_0001));
        assert_eq!((c - a).store().to_bits(), 0x7fc0_0001);
        let inf = load(f32::INFINITY);
        assert_eq!((inf - inf).store().to_bits(), 0xffc0_0000);
        assert_eq!((inf * ZERO).store().to_bits(), 0xffc0_0000);
        assert_eq!((-a).store().to_bits(), 0xffc0_0001);
        // a signalling NaN in memory loses to the register's quiet NaN
        let q = load(f32::from_bits(0x7fc0_0000));
        assert_eq!(
            q.add_m32(f32::from_bits(0x7f80_0001)).store().to_bits(),
            0x7fc0_0000
        );
        assert_eq!(
            (q + load(f32::from_bits(0x7f80_0001))).store().to_bits(),
            0x7fc0_0001
        );
        assert_eq!(
            ONE.mul_m32(f32::from_bits(0x7f80_0001)).store().to_bits(),
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
            (load(a[0]) * load(b[0])
                + load(a[1]) * load(b[1])
                + load(a[2]) * load(b[2])
                + load(a[3]).mul_m32(b[3])
                - load(a[4]))
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
