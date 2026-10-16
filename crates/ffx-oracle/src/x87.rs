//! The x87 instructions the emulator computes through host `double`
//! functions, redone as an Intel FPU does them: `fsin`, `fcos`, `fsincos`,
//! `fptan`, `fpatan`, `f2xm1`, `fyl2x`, `fyl2xp1`, `fprem`, `fprem1`.
//!
//! Unicorn (QEMU) converts the 80-bit operands to `double`, calls the host
//! `sin`/`pow`/`fmod`/..., and converts back: 11 bits of the 64-bit
//! mantissa are lost, the remainder loses its sign, and `MSVCR110.dll`'s
//! math (which runs on these instructions; its SSE2 paths are off unless a
//! program asks) came out wrong in the last bits for common arguments
//! (`pow(3, 4)` was `81.00000000000003`).
//!
//! The model:
//! - `fprem`/`fprem1` are exact, as specified (partial remainders for
//!   exponent differences of 64 or more, reducing by a multiple of 32).
//! - The transcendental results are the exact function values rounded to
//!   the 64-bit mantissa in the control word's rounding mode, computed with
//!   320-bit arithmetic (precision control does not apply to these
//!   instructions).
//! - `fsin`, `fcos`, `fsincos` and `fptan` reduce their argument with the
//!   FPU's 66-bit pi (`0xc90fdaa2_2168c234_c` x 2^-66), exactly, which is
//!   why `sin(1e22)` on an x87 is 0.98 and not -0.85; arguments of 2^63 or
//!   more set C2 and stay unchanged.
//!
//! Checked against the game's `MSVCR110.dll` on an Intel CPU
//! (`tests/data/msvcr110-math.tsv`).

// x, y, a, b, e, k, q, r: the names of the formulas and the manuals
#![allow(clippy::many_single_char_names)]

use std::cmp::Ordering;
use std::sync::OnceLock;

/// Working precision in bits.
const PREC: u64 = 320;

// ---------------------------------------------------------------- integers

/// An unsigned integer, little-endian 64-bit limbs, no high zero limbs.
type Big = Vec<u64>;

fn trim(mut a: Big) -> Big {
    while a.last() == Some(&0) {
        a.pop();
    }
    a
}

fn bit_len(a: &[u64]) -> u64 {
    a.last().map_or(0, |&h| {
        64 * (a.len() as u64 - 1) + u64::from(64 - h.leading_zeros())
    })
}

fn cmp_big(a: &[u64], b: &[u64]) -> Ordering {
    a.len()
        .cmp(&b.len())
        .then_with(|| a.iter().rev().cmp(b.iter().rev()))
}

fn add_big(a: &[u64], b: &[u64]) -> Big {
    let (a, b) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let mut out = Vec::with_capacity(a.len() + 1);
    let mut carry = 0u128;
    for (i, &x) in a.iter().enumerate() {
        let s = u128::from(x) + u128::from(b.get(i).copied().unwrap_or(0)) + carry;
        #[allow(clippy::cast_possible_truncation)] // the low limb
        out.push(s as u64);
        carry = s >> 64;
    }
    if carry != 0 {
        out.push(1);
    }
    out
}

/// `a - b` for `a >= b`.
fn sub_big(a: &[u64], b: &[u64]) -> Big {
    let mut out = Vec::with_capacity(a.len());
    let mut borrow = 0u64;
    for (i, &x) in a.iter().enumerate() {
        let y = b.get(i).copied().unwrap_or(0);
        let (d, o1) = x.overflowing_sub(y);
        let (d, o2) = d.overflowing_sub(borrow);
        out.push(d);
        borrow = u64::from(o1 || o2);
    }
    trim(out)
}

fn mul_big(a: &[u64], b: &[u64]) -> Big {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0u64; a.len() + b.len()];
    for (i, &x) in a.iter().enumerate() {
        let mut carry = 0u128;
        for (j, &y) in b.iter().enumerate() {
            let t = u128::from(x) * u128::from(y) + u128::from(out[i + j]) + carry;
            #[allow(clippy::cast_possible_truncation)] // the low limb
            {
                out[i + j] = t as u64;
            }
            carry = t >> 64;
        }
        #[allow(clippy::cast_possible_truncation)] // below 2^64
        {
            out[i + b.len()] = carry as u64;
        }
    }
    trim(out)
}

fn shl_big(a: &[u64], n: u64) -> Big {
    if a.is_empty() {
        return Vec::new();
    }
    let limbs = usize::try_from(n / 64).expect("shift");
    let bits = n % 64;
    let mut out = vec![0u64; limbs];
    let mut carry = 0u64;
    for &x in a {
        out.push((x << bits) | carry);
        carry = if bits == 0 { 0 } else { x >> (64 - bits) };
    }
    out.push(carry);
    trim(out)
}

fn shr_big(a: &[u64], n: u64) -> Big {
    let limbs = usize::try_from(n / 64).unwrap_or(usize::MAX);
    if limbs >= a.len() {
        return Vec::new();
    }
    let bits = n % 64;
    let src = &a[limbs..];
    let mut out = Vec::with_capacity(src.len());
    for (i, &x) in src.iter().enumerate() {
        let hi = src.get(i + 1).copied().unwrap_or(0);
        out.push(if bits == 0 {
            x
        } else {
            (x >> bits) | (hi << (64 - bits))
        });
    }
    trim(out)
}

/// Whether any of the low `n` bits is set.
fn low_bits_set(a: &[u64], n: u64) -> bool {
    let full = usize::try_from(n / 64).unwrap_or(usize::MAX);
    if a.iter().take(full).any(|&x| x != 0) {
        return true;
    }
    let rest = n % 64;
    rest != 0 && a.get(full).is_some_and(|&x| x & ((1u64 << rest) - 1) != 0)
}

fn bit(a: &[u64], n: u64) -> bool {
    let i = usize::try_from(n / 64).unwrap_or(usize::MAX);
    a.get(i).is_some_and(|&x| (x >> (n % 64)) & 1 == 1)
}

/// `a / d` (truncated).
fn div_small_big(a: &[u64], d: u64) -> Big {
    let mut out = vec![0u64; a.len()];
    let mut rem = 0u128;
    for i in (0..a.len()).rev() {
        let cur = (rem << 64) | u128::from(a[i]);
        #[allow(clippy::cast_possible_truncation)] // the quotient limb
        {
            out[i] = (cur / u128::from(d)) as u64;
        }
        rem = cur % u128::from(d);
    }
    trim(out)
}

// ------------------------------------------------------------------ floats

/// `(-1)^neg * mant * 2^exp`, exact up to [`PREC`] bits.
#[derive(Clone, Debug)]
struct F {
    neg: bool,
    mant: Big,
    exp: i64,
}

impl F {
    fn zero() -> Self {
        Self {
            neg: false,
            mant: Vec::new(),
            exp: 0,
        }
    }

    fn int(v: u64) -> Self {
        Self {
            neg: false,
            mant: trim(vec![v]),
            exp: 0,
        }
        .norm()
    }

    fn is_zero(&self) -> bool {
        self.mant.is_empty()
    }

    /// The exponent of the leading bit (the value is in [2^e, 2^(e+1))).
    fn top(&self) -> i64 {
        self.exp + i64::try_from(bit_len(&self.mant)).expect("small") - 1
    }

    /// Cuts the mantissa to `p` bits (truncating).
    fn cut(mut self, p: u64) -> Self {
        let n = bit_len(&self.mant);
        if n > p {
            self.mant = shr_big(&self.mant, n - p);
            self.exp += i64::try_from(n - p).expect("small");
        }
        if self.mant.is_empty() {
            self.neg = false;
            self.exp = 0;
        }
        self
    }

    fn norm(self) -> Self {
        self.cut(PREC)
    }

    fn neg(mut self) -> Self {
        if !self.is_zero() {
            self.neg = !self.neg;
        }
        self
    }

    fn abs(mut self) -> Self {
        self.neg = false;
        self
    }

    /// Exact sum (cut to `p` bits).
    fn add_p(&self, other: &Self, p: u64) -> Self {
        if self.is_zero() {
            return other.clone().cut(p);
        }
        if other.is_zero() {
            return self.clone().cut(p);
        }
        // far apart: the smaller is below the precision
        let gap = (self.top() - other.top()).unsigned_abs();
        if gap > p + 64 {
            return if self.top() > other.top() {
                self.clone().cut(p)
            } else {
                other.clone().cut(p)
            };
        }
        let e = self.exp.min(other.exp);
        let a = shl_big(&self.mant, u64::try_from(self.exp - e).expect("aligned"));
        let b = shl_big(&other.mant, u64::try_from(other.exp - e).expect("aligned"));
        let (neg, mant) = if self.neg == other.neg {
            (self.neg, add_big(&a, &b))
        } else {
            match cmp_big(&a, &b) {
                Ordering::Less => (other.neg, sub_big(&b, &a)),
                _ => (self.neg, sub_big(&a, &b)),
            }
        };
        Self { neg, mant, exp: e }.cut(p)
    }

    fn add(&self, other: &Self) -> Self {
        self.add_p(other, PREC)
    }

    fn sub(&self, other: &Self) -> Self {
        self.add(&other.clone().neg())
    }

    fn mul_p(&self, other: &Self, p: u64) -> Self {
        Self {
            neg: self.neg != other.neg,
            mant: mul_big(&self.mant, &other.mant),
            exp: self.exp + other.exp,
        }
        .cut(p)
    }

    fn mul(&self, other: &Self) -> Self {
        self.mul_p(other, PREC)
    }

    fn div_small(&self, d: u64) -> Self {
        let m = shl_big(&self.mant, 128);
        Self {
            neg: self.neg,
            mant: div_small_big(&m, d),
            exp: self.exp - 128,
        }
        .norm()
    }

    fn scale(mut self, k: i64) -> Self {
        if !self.is_zero() {
            self.exp += k;
        }
        self
    }

    /// An approximation for seeding Newton's method.
    fn to_f64(&self) -> f64 {
        if self.is_zero() {
            return 0.0;
        }
        let n = bit_len(&self.mant);
        let top = shr_big(&self.mant, n.saturating_sub(64));
        #[allow(clippy::cast_precision_loss)] // an approximation
        let m = top.first().copied().unwrap_or(0) as f64;
        let e = self.exp + i64::try_from(n.saturating_sub(64)).expect("small");
        let v = m * 2f64.powi(i32::try_from(e.clamp(-1000, 1000)).expect("clamped"));
        if self.neg { -v } else { v }
    }

    fn from_f64(x: f64) -> Self {
        if x == 0.0 {
            return Self::zero();
        }
        let b = x.to_bits();
        let e = i64::try_from((b >> 52) & 0x7ff).expect("11 bits");
        let m = (b & ((1 << 52) - 1)) | (1 << 52);
        Self {
            neg: x < 0.0,
            mant: vec![m],
            exp: e - 1075,
        }
    }

    /// `1 / self`.
    fn recip(&self) -> Self {
        // the seed on a scaled copy, so huge exponents do not overflow f64
        let t = self.top();
        let s = self.clone().scale(-t);
        let mut x = Self::from_f64(1.0 / s.to_f64());
        let two = Self::int(2);
        for _ in 0..4 {
            x = x.mul(&two.sub(&s.mul(&x)));
        }
        x.scale(-t)
    }

    fn div(&self, other: &Self) -> Self {
        self.mul(&other.recip())
    }

    fn sqrt(&self) -> Self {
        if self.is_zero() {
            return Self::zero();
        }
        // even exponent shift so the seed stays in range
        let t = self.top() & !1;
        let s = self.clone().scale(-t);
        let mut y = Self::from_f64(1.0 / s.to_f64().sqrt());
        let three = Self::int(3);
        for _ in 0..4 {
            y = y.mul(&three.sub(&s.mul(&y.mul(&y)))).scale(-1);
        }
        s.mul(&y).scale(t / 2)
    }

    fn cmp_abs(&self, other: &Self) -> Ordering {
        if self.is_zero() || other.is_zero() {
            return (!self.is_zero()).cmp(&!other.is_zero());
        }
        self.top().cmp(&other.top()).then_with(|| {
            let e = self.exp.min(other.exp);
            let a = shl_big(&self.mant, u64::try_from(self.exp - e).expect("aligned"));
            let b = shl_big(&other.mant, u64::try_from(other.exp - e).expect("aligned"));
            cmp_big(&a, &b)
        })
    }

    /// The integer nearest to the value (ties to even), and its sign.
    fn round_int(&self) -> (bool, Big) {
        if self.exp >= 0 {
            return (
                self.neg,
                shl_big(&self.mant, u64::try_from(self.exp).expect("pos")),
            );
        }
        let s = u64::try_from(-self.exp).expect("pos");
        let q = shr_big(&self.mant, s);
        let half = bit(&self.mant, s - 1);
        let rest = low_bits_set(&self.mant, s - 1);
        let up = half && (rest || bit(&q, 0));
        (self.neg, if up { add_big(&q, &[1]) } else { q })
    }

    /// The integer part, truncated toward zero.
    fn trunc_int(&self) -> Big {
        if self.exp >= 0 {
            shl_big(&self.mant, u64::try_from(self.exp).expect("pos"))
        } else {
            shr_big(&self.mant, u64::try_from(-self.exp).expect("pos"))
        }
    }

    fn from_big(neg: bool, m: Big) -> Self {
        Self {
            neg: neg && !m.is_empty(),
            mant: m,
            exp: 0,
        }
    }
}

// --------------------------------------------------------------- functions

/// Whether a series term no longer matters next to the sum.
fn negligible(term: &F, sum: &F) -> bool {
    term.is_zero()
        || (!sum.is_zero() && term.top() < sum.top() - i64::try_from(PREC).expect("small") - 8)
}

/// `sum (-1)^k x^(2k+1) / (2k+1)` (`atan`) or without signs (`atanh`).
fn odd_series(x: &F, alternate: bool) -> F {
    let x2 = x.mul(x);
    let mut pow = x.clone();
    let mut sum = x.clone();
    for k in 1u64.. {
        pow = pow.mul(&x2);
        let mut t = pow.div_small(2 * k + 1);
        if alternate && k % 2 == 1 {
            t = t.neg();
        }
        if negligible(&t, &sum) {
            break;
        }
        sum = sum.add(&t);
    }
    sum
}

fn pi() -> &'static F {
    static PI: OnceLock<F> = OnceLock::new();
    PI.get_or_init(|| {
        // Machin: 16 atan(1/5) - 4 atan(1/239)
        let a = odd_series(&F::int(1).div_small(5), true).scale(4);
        let b = odd_series(&F::int(1).div_small(239), true).scale(2);
        a.sub(&b)
    })
}

fn ln2() -> &'static F {
    static LN2: OnceLock<F> = OnceLock::new();
    LN2.get_or_init(|| odd_series(&F::int(1).div_small(3), false).scale(1))
}

/// The FPU's pi for argument reduction: pi cut to 68 bits (66 significant).
fn pi66() -> &'static F {
    static P: OnceLock<F> = OnceLock::new();
    P.get_or_init(|| pi().clone().cut(68))
}

/// `sin(r)` and `cos(r)` for a reduced `|r| <= ~pi/4`.
fn sin_cos_small(r: &F) -> (F, F) {
    let r2 = r.mul(r);
    let mut s = r.clone();
    let mut t = r.clone();
    for n in 1u64.. {
        t = t.mul(&r2).div_small((2 * n) * (2 * n + 1)).neg();
        if negligible(&t, &s) {
            break;
        }
        s = s.add(&t);
    }
    let mut c = F::int(1);
    let mut t = F::int(1);
    for n in 1u64.. {
        t = t.mul(&r2).div_small((2 * n - 1) * (2 * n)).neg();
        if negligible(&t, &c) {
            break;
        }
        c = c.add(&t);
    }
    (s, c)
}

/// `sin(x)` and `cos(x)` with the reduction by [`pi66`].
fn sin_cos(x: &F) -> (F, F) {
    let half = pi66().clone().scale(-1);
    let (kneg, k) = x.div(&half).round_int();
    if k.is_empty() {
        return sin_cos_small(x);
    }
    // r = x - k * pi66/2, exactly
    let kp = F::from_big(kneg, k.clone()).mul_p(&half, 4 * PREC);
    let r = x.add_p(&kp.neg(), 4 * PREC).norm();
    let (s, c) = sin_cos_small(&r);
    let q = k.first().copied().unwrap_or(0) & 3;
    let q = if kneg { (4 - q) & 3 } else { q };
    match q {
        0 => (s, c),
        1 => (c, s.neg()),
        2 => (s.neg(), c.neg()),
        _ => (c.neg(), s),
    }
}

/// `2^x - 1`.
fn exp2m1(x: &F) -> F {
    let y = x.mul(ln2());
    let mut sum = y.clone();
    let mut t = y.clone();
    for n in 2u64.. {
        t = t.mul(&y).div_small(n);
        if negligible(&t, &sum) {
            break;
        }
        sum = sum.add(&t);
    }
    sum
}

/// `ln(m)` for `m` in about [0.7, 1.5].
fn ln_near_one(m: &F) -> F {
    let one = F::int(1);
    let z = m.sub(&one).div(&m.add(&one));
    odd_series(&z, false).scale(1)
}

/// `log2(x)` for `x > 0`.
fn log2(x: &F) -> F {
    let mut e = x.top();
    let mut m = x.clone().scale(-e); // [1, 2)
    if m.cmp_abs(&F::from_f64(std::f64::consts::SQRT_2)) == Ordering::Greater {
        m = m.scale(-1);
        e += 1;
    }
    let l = ln_near_one(&m).div(ln2());
    let ef = F::from_big(e < 0, trim(vec![e.unsigned_abs()]));
    ef.add(&l)
}

/// `log2(1 + x)` for `|x| < 1`.
fn log2_1p(x: &F) -> F {
    let z = x.div(&F::int(2).add(x));
    odd_series(&z, false).scale(1).div(ln2())
}

/// `atan(t)` for `0 <= t <= 1`.
fn atan_unit(t: &F) -> F {
    // two halvings: atan(t) = 2 atan(t / (1 + sqrt(1 + t^2)))
    let one = F::int(1);
    let mut u = t.clone();
    for _ in 0..2 {
        u = u.div(&one.add(&one.add(&u.mul(&u)).sqrt()));
    }
    odd_series(&u, true).scale(2)
}

// ---------------------------------------------------------------- extended

/// An 80-bit extended value: sign, biased exponent, mantissa (explicit
/// integer bit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Ext {
    pub neg: bool,
    pub exp: u16,
    pub mant: u64,
}

const INDEFINITE: Ext = Ext {
    neg: true,
    exp: 0x7fff,
    mant: 0xc000_0000_0000_0000,
};

impl Ext {
    pub(crate) fn from_bytes(b: &[u8; 10]) -> Self {
        let mant = u64::from_le_bytes(b[..8].try_into().expect("8 bytes"));
        let se = u16::from_le_bytes([b[8], b[9]]);
        Self {
            neg: se & 0x8000 != 0,
            exp: se & 0x7fff,
            mant,
        }
    }

    pub(crate) fn to_bytes(self) -> [u8; 10] {
        let mut b = [0u8; 10];
        b[..8].copy_from_slice(&self.mant.to_le_bytes());
        let se = self.exp | if self.neg { 0x8000 } else { 0 };
        b[8..].copy_from_slice(&se.to_le_bytes());
        b
    }

    fn zero(neg: bool) -> Self {
        Self {
            neg,
            exp: 0,
            mant: 0,
        }
    }

    fn inf(neg: bool) -> Self {
        Self {
            neg,
            exp: 0x7fff,
            mant: 1 << 63,
        }
    }

    fn one() -> Self {
        Self {
            neg: false,
            exp: 0x3fff,
            mant: 1 << 63,
        }
    }

    fn is_nan(self) -> bool {
        self.exp == 0x7fff && self.mant << 1 != 0
    }

    fn is_inf(self) -> bool {
        self.exp == 0x7fff && self.mant << 1 == 0
    }

    fn is_zero(self) -> bool {
        self.mant == 0 && self.exp == 0
    }

    /// Exponent set but the integer bit clear: unsupported, invalid.
    fn unsupported(self) -> bool {
        self.exp != 0 && self.mant >> 63 == 0
    }

    fn quiet(self) -> Self {
        Self {
            mant: self.mant | (1 << 62),
            ..self
        }
    }

    fn signaling(self) -> bool {
        self.is_nan() && self.mant & (1 << 62) == 0
    }

    fn value(self) -> F {
        let e = if self.exp == 0 {
            -16445
        } else {
            i64::from(self.exp) - 16446
        };
        F {
            neg: self.neg,
            mant: trim(vec![self.mant]),
            exp: e,
        }
        .norm()
    }
}

/// The x87 rounding control.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Rc {
    Nearest,
    Down,
    Up,
    Zero,
}

/// Rounds to an extended value: the value, whether it is inexact, and
/// whether it was rounded up in magnitude (C1).
fn round(x: &F, rc: Rc) -> (Ext, bool, bool) {
    if x.is_zero() {
        return (Ext::zero(x.neg), false, false);
    }
    let biased = x.top() + 16383;
    // integer mantissa q = x * 2^shift, 64 bits (fewer when subnormal)
    let shift = if biased >= 1 { 63 - x.top() } else { 16445 };
    let (q, inexact, half, rest) = if x.exp + shift >= 0 {
        let s = u64::try_from(x.exp + shift).expect("pos");
        (shl_big(&x.mant, s), false, false, false)
    } else {
        let s = u64::try_from(-(x.exp + shift)).expect("pos");
        let q = shr_big(&x.mant, s);
        let half = bit(&x.mant, s - 1);
        let rest = low_bits_set(&x.mant, s - 1);
        (q, half || rest, half, rest)
    };
    let odd = bit(&q, 0);
    let up = inexact
        && match rc {
            Rc::Nearest => half && (rest || odd),
            Rc::Zero => false,
            Rc::Up => !x.neg,
            Rc::Down => x.neg,
        };
    let mut q = if up { add_big(&q, &[1]) } else { q };
    let mut biased = biased.max(0);
    let mut exp_field = if biased >= 1 { biased } else { 0 };
    if bit_len(&q) > 64 {
        q = shr_big(&q, 1);
        biased += 1;
        exp_field = biased;
    } else if exp_field == 0 && bit(&q, 63) {
        // a subnormal rounded up to the smallest normal
        exp_field = 1;
    }
    if exp_field >= 0x7fff {
        let to_inf = match rc {
            Rc::Nearest => true,
            Rc::Zero => false,
            Rc::Up => !x.neg,
            Rc::Down => x.neg,
        };
        let r = if to_inf {
            Ext::inf(x.neg)
        } else {
            Ext {
                neg: x.neg,
                exp: 0x7ffe,
                mant: u64::MAX,
            }
        };
        return (r, true, to_inf);
    }
    let r = Ext {
        neg: x.neg,
        exp: u16::try_from(exp_field).expect("in range"),
        mant: q.first().copied().unwrap_or(0),
    };
    (r, inexact, up)
}

// ------------------------------------------------------------ instructions

/// One of the instructions this module computes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Op {
    Fsin,
    Fcos,
    Fsincos,
    Fptan,
    Fpatan,
    F2xm1,
    Fyl2x,
    Fyl2xp1,
    Fprem,
    Fprem1,
}

impl Op {
    /// The instruction for the second byte after `0xd9`.
    pub(crate) fn decode(second: u8) -> Option<Self> {
        Some(match second {
            0xfe => Self::Fsin,
            0xff => Self::Fcos,
            0xfb => Self::Fsincos,
            0xf2 => Self::Fptan,
            0xf3 => Self::Fpatan,
            0xf0 => Self::F2xm1,
            0xf1 => Self::Fyl2x,
            0xf9 => Self::Fyl2xp1,
            0xf8 => Self::Fprem,
            0xf5 => Self::Fprem1,
            _ => return None,
        })
    }
}

/// What an instruction does to the stack and the status word.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Effect {
    /// New ST0 (and ST1 for the two-operand ones); for a push, `st0` is
    /// the pushed value and `st1` the replaced old ST0.
    pub st0: Option<Ext>,
    pub st1: Option<Ext>,
    pub push: bool,
    pub pop: bool,
    /// Condition codes C0, C1, C2, C3 (`None`: unchanged).
    pub c: [Option<bool>; 4],
    /// Exception flags to set (IE, DE, ZE, OE, UE, PE bits).
    pub flags: u16,
}

const IE: u16 = 0x01;
const DE: u16 = 0x02;
const ZE: u16 = 0x04;
const OE: u16 = 0x08;
const UE: u16 = 0x10;
const PE: u16 = 0x20;

fn rc_of(cw: u16) -> Rc {
    match (cw >> 10) & 3 {
        0 => Rc::Nearest,
        1 => Rc::Down,
        2 => Rc::Up,
        _ => Rc::Zero,
    }
}

/// Rounds and records the flags for one result.
fn finish(x: &F, rc: Rc, flags: &mut u16) -> (Ext, bool) {
    let (r, inexact, up) = round(x, rc);
    if inexact {
        *flags |= PE;
    }
    if r.is_inf() {
        *flags |= OE;
    } else if r.exp == 0 && inexact {
        *flags |= UE;
    }
    (r, up)
}

/// The NaN two NaN operands give: the larger significand, quieted.
fn nan_of(a: Ext, b: Ext) -> Ext {
    match (a.is_nan(), b.is_nan()) {
        (true, true) => {
            if (a.mant << 1) >= (b.mant << 1) {
                a.quiet()
            } else {
                b.quiet()
            }
        }
        (true, false) => a.quiet(),
        _ => b.quiet(),
    }
}

fn denormal_flag(xs: &[Ext]) -> u16 {
    if xs.iter().any(|x| x.exp == 0 && x.mant != 0) {
        DE
    } else {
        0
    }
}

/// Computes one instruction from ST0, ST1 and the control word.
#[allow(clippy::too_many_lines)] // the special cases, instruction by instruction
pub(crate) fn execute(op: Op, st0: Ext, st1: Ext, cw: u16) -> Effect {
    let rc = rc_of(cw);
    let mut e = Effect {
        st0: None,
        st1: None,
        push: false,
        pop: false,
        c: [None, Some(false), None, None],
        flags: 0,
    };
    let invalid = |e: &mut Effect| e.flags |= IE;
    match op {
        Op::Fsin | Op::Fcos | Op::Fsincos | Op::Fptan => {
            let push = matches!(op, Op::Fsincos | Op::Fptan);
            e.c[2] = Some(false);
            let special = if st0.is_nan() {
                if st0.signaling() {
                    invalid(&mut e);
                }
                Some((st0.quiet(), st0.quiet()))
            } else if st0.is_inf() || st0.unsupported() {
                invalid(&mut e);
                Some((INDEFINITE, INDEFINITE))
            } else if st0.is_zero() {
                Some(match op {
                    Op::Fsin => (st0, st0),
                    Op::Fcos => (Ext::one(), Ext::one()),
                    // fsincos: ST0 = cos, ST1 = sin; fptan: ST0 = 1, ST1 = tan
                    _ => (Ext::one(), st0),
                })
            } else if i32::from(st0.exp) - 16383 >= 63 {
                // out of range: C2 set, nothing else changes
                e.c[2] = Some(true);
                return e;
            } else {
                None
            };
            e.flags |= denormal_flag(&[st0]);
            let (first, second) = special.unwrap_or_else(|| {
                let (s, c) = sin_cos(&st0.value());
                match op {
                    Op::Fsin => {
                        let (r, up) = finish(&s, rc, &mut e.flags);
                        e.c[1] = Some(up);
                        (r, r)
                    }
                    Op::Fcos => {
                        let (r, up) = finish(&c, rc, &mut e.flags);
                        e.c[1] = Some(up);
                        (r, r)
                    }
                    Op::Fsincos => {
                        let (rs, _) = finish(&s, rc, &mut e.flags);
                        let (rc_, up) = finish(&c, rc, &mut e.flags);
                        e.c[1] = Some(up);
                        (rc_, rs)
                    }
                    _ => {
                        let (rt, up) = finish(&s.div(&c), rc, &mut e.flags);
                        e.c[1] = Some(up);
                        (Ext::one(), rt)
                    }
                }
            });
            if push {
                e.push = true;
                e.st0 = Some(first);
                e.st1 = Some(second);
            } else {
                e.st0 = Some(first);
            }
        }
        Op::F2xm1 => {
            let r = if st0.is_nan() {
                if st0.signaling() {
                    invalid(&mut e);
                }
                st0.quiet()
            } else if st0.unsupported() {
                invalid(&mut e);
                INDEFINITE
            } else if st0.is_inf() {
                if st0.neg {
                    Ext {
                        neg: true,
                        ..Ext::one()
                    }
                } else {
                    st0
                }
            } else if st0.is_zero() {
                st0
            } else {
                e.flags |= denormal_flag(&[st0]);
                let (r, up) = finish(&exp2m1(&st0.value()), rc, &mut e.flags);
                e.c[1] = Some(up);
                r
            };
            e.st0 = Some(r);
        }
        Op::Fpatan => {
            e.pop = true;
            let (y, x) = (st1, st0);
            let r = if x.is_nan() || y.is_nan() {
                if x.signaling() || y.signaling() {
                    invalid(&mut e);
                }
                nan_of(y, x)
            } else if x.unsupported() || y.unsupported() {
                invalid(&mut e);
                INDEFINITE
            } else {
                e.flags |= denormal_flag(&[x, y]);
                let p = pi();
                // the angle magnitude, then its sign from y
                let a = if y.is_zero() {
                    if x.neg { p.clone() } else { F::zero() }
                } else if x.is_zero() {
                    p.clone().scale(-1)
                } else if y.is_inf() && x.is_inf() {
                    if x.neg {
                        p.mul(&F::int(3)).scale(-2)
                    } else {
                        p.clone().scale(-2)
                    }
                } else if y.is_inf() {
                    p.clone().scale(-1)
                } else if x.is_inf() {
                    if x.neg { p.clone() } else { F::zero() }
                } else {
                    let (yv, xv) = (y.value().abs(), x.value().abs());
                    let base = if yv.cmp_abs(&xv) == Ordering::Greater {
                        p.clone().scale(-1).sub(&atan_unit(&xv.div(&yv)))
                    } else {
                        atan_unit(&yv.div(&xv))
                    };
                    if x.neg { p.sub(&base) } else { base }
                };
                if a.is_zero() {
                    Ext::zero(y.neg)
                } else {
                    let a = if y.neg { a.neg() } else { a };
                    let (r, up) = finish(&a, rc, &mut e.flags);
                    e.c[1] = Some(up);
                    r
                }
            };
            e.st1 = Some(r);
        }
        Op::Fyl2x | Op::Fyl2xp1 => {
            e.pop = true;
            let (y, x) = (st1, st0);
            let plus1 = op == Op::Fyl2xp1;
            let r = if x.is_nan() || y.is_nan() {
                if x.signaling() || y.signaling() {
                    invalid(&mut e);
                }
                nan_of(x, y)
            } else if x.unsupported() || y.unsupported() {
                invalid(&mut e);
                INDEFINITE
            } else if plus1 {
                e.flags |= denormal_flag(&[x, y]);
                let xv = x.value();
                let below = !x.is_inf() && xv.add(&F::int(1)).neg && !xv.add(&F::int(1)).is_zero();
                if x.is_zero() {
                    if y.is_inf() {
                        invalid(&mut e);
                        INDEFINITE
                    } else {
                        Ext::zero(x.neg != y.neg)
                    }
                } else if x.is_inf() || below {
                    // outside the instruction's range
                    invalid(&mut e);
                    INDEFINITE
                } else if y.is_inf() {
                    Ext::inf(y.neg != x.neg)
                } else if y.is_zero() {
                    Ext::zero(y.neg != x.neg)
                } else {
                    let v = log2_1p(&xv).mul(&y.value());
                    let (r, up) = finish(&v, rc, &mut e.flags);
                    e.c[1] = Some(up);
                    r
                }
            } else {
                e.flags |= denormal_flag(&[x, y]);
                let one = x == Ext::one();
                if x.neg && !x.is_zero() {
                    invalid(&mut e);
                    INDEFINITE
                } else if x.is_zero() {
                    if y.is_zero() {
                        invalid(&mut e);
                        INDEFINITE
                    } else {
                        if !y.is_inf() {
                            e.flags |= ZE;
                        }
                        Ext::inf(!y.neg)
                    }
                } else if x.is_inf() {
                    if y.is_zero() {
                        invalid(&mut e);
                        INDEFINITE
                    } else {
                        Ext::inf(y.neg)
                    }
                } else if one {
                    if y.is_inf() {
                        invalid(&mut e);
                        INDEFINITE
                    } else {
                        Ext::zero(y.neg)
                    }
                } else {
                    let l = log2(&x.value());
                    if y.is_inf() {
                        Ext::inf(y.neg != l.neg)
                    } else if y.is_zero() {
                        Ext::zero(y.neg != l.neg)
                    } else {
                        let (r, up) = finish(&l.mul(&y.value()), rc, &mut e.flags);
                        e.c[1] = Some(up);
                        r
                    }
                }
            };
            e.st1 = Some(r);
        }
        Op::Fprem | Op::Fprem1 => {
            let (a, b) = (st0, st1);
            e.c[2] = Some(false);
            let r = if a.is_nan() || b.is_nan() {
                if a.signaling() || b.signaling() {
                    invalid(&mut e);
                }
                nan_of(a, b)
            } else if a.is_inf() || b.is_zero() || a.unsupported() || b.unsupported() {
                invalid(&mut e);
                INDEFINITE
            } else if a.is_zero() || b.is_inf() {
                e.c = [Some(false), Some(false), Some(false), Some(false)];
                a
            } else {
                e.flags |= denormal_flag(&[a, b]);
                let (av, bv) = (a.value().abs(), b.value().abs());
                let d = av.top() - bv.top();
                if d >= 64 {
                    // partial: reduce the exponent by a multiple of 32
                    let n = 32 + d % 32;
                    let bs = bv.clone().scale(d - n);
                    let q = av.div(&bs).trunc_int();
                    let mut qb = F::from_big(false, q).mul_p(&bs, 4 * PREC);
                    // the division may round up past the true quotient
                    if qb.cmp_abs(&av) == Ordering::Greater {
                        qb = qb.sub(&bs);
                    }
                    let rem = av.add_p(&qb.neg(), 4 * PREC);
                    e.c[2] = Some(true);
                    let rem = if a.neg { rem.neg() } else { rem };
                    round(&rem, rc).0
                } else {
                    let exact = |q: &Big| {
                        av.add_p(&F::from_big(true, q.clone()).mul_p(&bv, 4 * PREC), 4 * PREC)
                    };
                    let mut q = av.div(&bv).trunc_int();
                    let mut rem = exact(&q);
                    // fix a quotient the rounded division left off by one
                    while rem.neg && !rem.is_zero() {
                        q = sub_big(&q, &[1]);
                        rem = exact(&q);
                    }
                    while rem.cmp_abs(&bv) != Ordering::Less {
                        q = add_big(&q, &[1]);
                        rem = exact(&q);
                    }
                    if op == Op::Fprem1 {
                        // nearest quotient, ties to even
                        let twice = rem.clone().scale(1);
                        let c = twice.cmp_abs(&bv);
                        if c == Ordering::Greater || (c == Ordering::Equal && bit(&q, 0)) {
                            q = add_big(&q, &[1]);
                            rem = exact(&q);
                        }
                    }
                    e.c[0] = Some(bit(&q, 2));
                    e.c[3] = Some(bit(&q, 1));
                    e.c[1] = Some(bit(&q, 0));
                    let neg = a.neg != (rem.neg && !rem.is_zero());
                    if rem.is_zero() {
                        Ext::zero(a.neg)
                    } else {
                        round(&rem.abs(), rc).0.with_sign(neg)
                    }
                }
            };
            e.st0 = Some(r);
        }
    }
    e
}

impl Ext {
    fn with_sign(self, neg: bool) -> Self {
        Self { neg, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ext(x: f64) -> Ext {
        if x == 0.0 {
            return Ext::zero(x.is_sign_negative());
        }
        round(&F::from_f64(x), Rc::Nearest).0
    }

    fn dbl(r: Ext) -> f64 {
        // round-to-nearest to 53 bits, as fstp qword does for normal values
        let v = r.value();
        let n = bit_len(&v.mant);
        let shift = n.saturating_sub(53);
        let q = F {
            neg: v.neg,
            mant: {
                let q = shr_big(&v.mant, shift);
                let up = shift > 0
                    && bit(&v.mant, shift - 1)
                    && (low_bits_set(&v.mant, shift - 1) || bit(&q, 0));
                if up { add_big(&q, &[1]) } else { q }
            },
            exp: v.exp + i64::try_from(shift).unwrap(),
        };
        q.to_f64()
    }

    const CW: u16 = 0x027f;

    #[test]
    fn constants() {
        assert_eq!(pi().to_f64(), std::f64::consts::PI);
        assert_eq!(ln2().to_f64(), std::f64::consts::LN_2);
        // the FPU's pi, 68 bits
        let p = pi66();
        assert_eq!(bit_len(&p.mant), 68);
        assert_eq!(p.mant, vec![0x90fd_aa22_168c_234c, 0xc]);
    }

    #[test]
    fn simple_values() {
        let one = |op, x: f64| dbl(execute(op, ext(x), Ext::one(), CW).st0.unwrap());
        assert_eq!(one(Op::Fsin, 0.5), 0.5f64.sin());
        assert_eq!(one(Op::Fcos, 0.5), 0.5f64.cos());
        assert_eq!(one(Op::F2xm1, 0.5), 0.414_213_562_373_095_03);
        let e = execute(Op::Fyl2x, ext(8.0), ext(3.0), CW);
        assert_eq!(dbl(e.st1.unwrap()), 9.0);
        assert!(e.pop);
        let e = execute(Op::Fpatan, ext(-1.0), ext(1.0), CW);
        assert_eq!(dbl(e.st1.unwrap()), 3.0 * std::f64::consts::FRAC_PI_4);
        // the remainder keeps the dividend's sign, even when zero
        let e = execute(Op::Fprem, ext(-1.0), ext(1.0), CW);
        assert_eq!(e.st0.unwrap(), Ext::zero(true));
        let e = execute(Op::Fprem, ext(7.5), ext(2.0), CW);
        assert_eq!(dbl(e.st0.unwrap()), 1.5);
        assert_eq!(e.c, [Some(false), Some(true), Some(false), Some(true)]); // q = 3
        let e = execute(Op::Fprem1, ext(7.5), ext(2.0), CW);
        assert_eq!(dbl(e.st0.unwrap()), -0.5);
    }

    #[test]
    fn out_of_range_arguments() {
        // 2^63 and up: C2 set, ST0 unchanged (the runtime then reduces
        // with fprem itself)
        let e = execute(Op::Fsin, ext(1e22), Ext::one(), CW);
        assert_eq!(e.c[2], Some(true));
        assert!(e.st0.is_none());
        let e = execute(Op::Fcos, ext(1e18), Ext::one(), CW);
        assert_eq!(e.c[2], Some(false));
        assert!(e.st0.is_some());
    }
}
