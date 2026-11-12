//! `MSVCR110`'s maths in Rust (docs/host.md, "The C runtime DLLs"): the
//! x87 intrinsics (`_CIsin`, `_CItan`, `_CIatan2`, `_CIfmod`, ...) and the
//! SSE2 functions (`_libm_sse2_*_precise`), bit for bit as the player's DLL
//! (11.0.51106.1) computes them on the path `FFX.exe`'s process takes.
//!
//! **Which path.** Each intrinsic tests `__use_sse2_mathfcns`, `MXCSR`
//! (`& 0x7f80 == 0x1f80`: every exception masked, round to nearest) and
//! the x87 control word (`& 0x7f == 0x7f`: every exception masked). The
//! DLL's start-up sets the flag on a processor with SSE2 and `FFX.exe`
//! never clears it (it imports no `_set_SSE2_enable`), so the game takes
//! the SSE2 paths under its control words (`0x027f`, `MXCSR` `0x1f80`);
//! without the DLL the flag counts as set. The SSE2 code hands large or
//! non-finite arguments to the DLL's x87 code; those run here too, with
//! `ffx_fpu` computing the transcendental instructions as the processor
//! does. A case neither covers (another `MXCSR`, an unmasked exception, a
//! result the DLL's error handler would rewrite) returns `None`: the DLL's
//! export with Unicorn, a hard stop without it.
//!
//! **The DLL's constants.** The SSE2 code reads coefficient tables from
//! the DLL's `.text`. They are read from the player's `MSVCR110.dll` in the
//! game folder as data ([`CrtData`]), with or without the DLL loaded; no
//! code of it runs, and nothing of it is in the repository.
//!
//! **Results.** The x87 paths leave an 80-bit `ST0` (the transcendental
//! instructions ignore the precision control); a Rust side returns it as
//! an [`Ext`] and the caller's store rounds once (`call_x87`'s `Float`).

#![allow(clippy::many_single_char_names)] // the formulas' x, y, k, r

use std::path::Path;

use ffx_fpu::x87::{Ext, Op, execute};

use crate::exec::Exec;
use crate::imports::ImportFn;
use crate::table::{Args, Conv, Ret};

/// The DLL the constants come from, and the build they were read from.
const CRT_FILE: &str = "MSVCR110.dll";
const CRT_STAMP: u32 = 0x5098_858e;
const CRT_CHECK_SUM: u32 = 0x000e_396c;
/// Its preferred base: the addresses in this module are the DLL's own.
const DLL_BASE: u32 = 0x1000_0000;
/// `__use_sse2_mathfcns` in the DLL's `.data`.
const USE_SSE2: u32 = 0x100c_9df4;

/// The player's `MSVCR110.dll`'s sections at their addresses, read from
/// the file (no relocations: the tables are numbers).
#[derive(Debug)]
pub struct CrtData {
    image: Vec<u8>,
}

impl CrtData {
    /// The file beside `FFX.exe` in `dir`, if it is the known build.
    #[must_use]
    pub fn load(dir: &Path) -> Option<Self> {
        let lower = CRT_FILE.to_ascii_lowercase();
        let bytes = [CRT_FILE, lower.as_str()]
            .iter()
            .find_map(|n| std::fs::read(dir.join(n)).ok())?;
        Self::from_bytes(&bytes)
    }

    /// The image of the known build's `bytes`.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let pe = goblin::pe::PE::parse(bytes).ok()?;
        let opt = pe.header.optional_header?;
        if pe.header.coff_header.time_date_stamp != CRT_STAMP
            || opt.windows_fields.check_sum != CRT_CHECK_SUM
        {
            return None;
        }
        let mut image = vec![0u8; opt.windows_fields.size_of_image as usize];
        for s in &pe.sections {
            let raw = s.pointer_to_raw_data as usize;
            let va = s.virtual_address as usize;
            let len = (s.size_of_raw_data as usize)
                .min(bytes.len().saturating_sub(raw))
                .min(image.len().saturating_sub(va));
            image[va..va + len].copy_from_slice(&bytes[raw..raw + len]);
        }
        Some(Self { image })
    }

    fn bytes<const N: usize>(&self, va: u32) -> [u8; N] {
        let at = va.wrapping_sub(DLL_BASE) as usize;
        self.image
            .get(at..at + N)
            .and_then(|b| b.try_into().ok())
            .unwrap_or([0; N])
    }

    /// The double at `va`.
    #[must_use]
    pub fn f(&self, va: u32) -> f64 {
        f64::from_le_bytes(self.bytes(va))
    }

    /// The 32-bit word at `va` (0 outside the image).
    #[must_use]
    pub fn word(&self, va: u32) -> u32 {
        u32::from_le_bytes(self.bytes(va))
    }

    /// The NUL-terminated string at `va`, if the image holds it.
    #[must_use]
    pub fn c_string(&self, va: u32) -> Option<Vec<u8>> {
        let at = va.wrapping_sub(DLL_BASE) as usize;
        let rest = self.image.get(at..)?;
        let end = rest.iter().position(|&b| b == 0)?;
        Some(rest[..end].to_vec())
    }

    /// The 16-bit word at `va`, if the image holds it.
    #[must_use]
    pub fn word16(&self, va: u32) -> Option<u16> {
        let at = va.wrapping_sub(DLL_BASE) as usize;
        let b = self.image.get(at..at + 2)?;
        Some(u16::from_le_bytes([b[0], b[1]]))
    }

    /// The double's bits at `va`.
    fn q(&self, va: u32) -> u64 {
        u64::from_le_bytes(self.bytes(va))
    }

    /// The pair of doubles at `va` (an `XMMWORD`: lane 0 first).
    fn p(&self, va: u32) -> P2 {
        P2(self.f(va), self.f(va + 8))
    }

    /// The 80-bit value at `va` (a `TBYTE`).
    fn t(&self, va: u32) -> Ext {
        Ext::from_bytes(&self.bytes(va))
    }
}

// ------------------------------------------------------------ SSE2 lanes

/// An `XMM` register as two doubles (lane 0, lane 1): `addpd`, `subpd`,
/// `mulpd` work lane by lane, one IEEE rounding each.
#[derive(Clone, Copy, Debug)]
struct P2(f64, f64);

impl P2 {
    fn dup(x: f64) -> Self {
        Self(x, x)
    }
}

impl std::ops::Add for P2 {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self(self.0 + o.0, self.1 + o.1)
    }
}

impl std::ops::Sub for P2 {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self(self.0 - o.0, self.1 - o.1)
    }
}

impl std::ops::Mul for P2 {
    type Output = Self;
    fn mul(self, o: Self) -> Self {
        Self(self.0 * o.0, self.1 * o.1)
    }
}

/// `andpd` on one lane.
fn and(x: f64, mask: u64) -> f64 {
    f64::from_bits(x.to_bits() & mask)
}

/// `cvtsd2si` under the default `MXCSR`: to nearest, ties to even (the
/// callers' ranges keep it inside `i32`).
#[allow(clippy::cast_possible_truncation)]
fn cvtsd2si(x: f64) -> i32 {
    x.round_ties_even() as i32
}

/// `pextrw r, xmm, 3`: the top 16 bits of the low lane.
fn top16(x: f64) -> u16 {
    u16::try_from(x.to_bits() >> 48).unwrap_or(0)
}

// ------------------------------------------------------- 80-bit values

/// `x` as an 80-bit value (exact: `fld qword`).
#[must_use]
pub fn ext_of(x: f64) -> Ext {
    let bits = x.to_bits();
    let neg = bits >> 63 != 0;
    let e = u16::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
    let m = bits & ((1 << 52) - 1);
    match e {
        0x7ff => Ext {
            neg,
            exp: 0x7fff,
            mant: (1 << 63) | (m << 11),
        },
        0 if m == 0 => Ext {
            neg,
            exp: 0,
            mant: 0,
        },
        0 => {
            // a denormal: value m * 2^-1074, normalized
            let s = m.leading_zeros();
            Ext {
                neg,
                exp: u16::try_from(0x3fff + 63 - 1074 - i32::try_from(s).unwrap_or(0)).unwrap_or(0),
                mant: m << s,
            }
        }
        _ => Ext {
            neg,
            exp: e - 1023 + 0x3fff,
            mant: (1 << 63) | (m << 11),
        },
    }
}

/// `x` rounded to nearest (ties to even) into an IEEE format with `p`
/// significand bits (the hidden one included) and `ebits` exponent bits:
/// what `fst` stores under the default control word (a NaN keeps its top
/// payload bits, quieted).
fn ext_to_bits(x: Ext, p: u32, ebits: u32) -> u64 {
    let frac_bits = p - 1;
    let sign = u64::from(x.neg) << (frac_bits + ebits);
    let emax = (1u64 << ebits) - 1;
    let bias = i64::try_from((1u64 << (ebits - 1)) - 1).unwrap_or(0);
    if x.exp == 0x7fff {
        if x.mant << 1 == 0 {
            return sign | (emax << frac_bits);
        }
        let payload = (x.mant << 1) >> (64 - frac_bits);
        return sign | (emax << frac_bits) | payload | (1 << (frac_bits - 1));
    }
    if x.mant == 0 {
        return sign;
    }
    let lz = x.mant.leading_zeros();
    let m = x.mant << lz;
    // value = m * 2^(e - 63)
    let e = if x.exp == 0 { 1 } else { i64::from(x.exp) } - 0x3fff - i64::from(lz);
    let mut biased = e + bias;
    let mut shift = i64::from(64 - p);
    if biased < 1 {
        shift += 1 - biased;
        biased = 0;
    }
    let shift = u32::try_from(shift.min(66)).unwrap_or(66);
    // m with two guard words: round at bit `shift`
    let wide = u128::from(m) << 64;
    let s = shift + 64;
    let mut q = u64::try_from(wide >> s).unwrap_or(0);
    let rem = wide & ((1u128 << s) - 1);
    let half = 1u128 << (s - 1);
    if rem > half || (rem == half && q & 1 == 1) {
        q += 1;
    }
    if biased == 0 {
        // a denormal (or rounded up into the smallest normal: same bits)
        return sign | q;
    }
    let mut biased = u64::try_from(biased).unwrap_or(0);
    if q >> p != 0 {
        q >>= 1;
        biased += 1;
    }
    if biased >= emax {
        return sign | (emax << frac_bits);
    }
    sign | (biased << frac_bits) | (q & ((1 << frac_bits) - 1))
}

/// `x` stored as a double (`fst qword`).
#[must_use]
pub fn ext_to_f64(x: Ext) -> f64 {
    f64::from_bits(ext_to_bits(x, 53, 11))
}

/// `x` stored as a float (`fst dword`: one rounding from the 80 bits).
#[must_use]
pub fn ext_to_f32(x: Ext) -> f32 {
    f32::from_bits(u32::try_from(ext_to_bits(x, 24, 8)).unwrap_or(0))
}

/// Whether `x` is exactly a double (so a double carries it unchanged).
#[must_use]
pub fn is_double(x: Ext) -> bool {
    let d = ext_to_f64(x);
    if d.is_nan() {
        return x.exp == 0x7fff && x.mant << 1 != 0 && x.mant << 54 == 0;
    }
    ext_of(d) == x
}

fn neg(x: Ext) -> Ext {
    Ext { neg: !x.neg, ..x }
}

fn abs(x: Ext) -> Ext {
    Ext { neg: false, ..x }
}

/// `a - b` for `a > b > 0` (finite, normal), rounded to the 64-bit
/// significand to nearest (`fsub` at precision control 64 bits).
fn sub_positive(a: Ext, b: Ext) -> Ext {
    let ea = i64::from(a.exp);
    let eb = i64::from(b.exp);
    // a's significand in the top of 128 bits, b's aligned below, with a
    // sticky bit for what falls off
    let wa = u128::from(a.mant) << 62;
    let d = u32::try_from(ea - eb).unwrap_or(u32::MAX);
    let wb0 = u128::from(b.mant) << 62;
    let wb = if d >= 127 {
        u128::from(wb0 != 0)
    } else {
        let lost = wb0 & ((1u128 << d) - 1);
        (wb0 >> d) | u128::from(lost != 0)
    };
    let mut r = wa - wb;
    if r == 0 {
        return Ext {
            neg: false,
            exp: 0,
            mant: 0,
        };
    }
    let lz = r.leading_zeros();
    r <<= lz;
    let exp = ea + 2 - i64::from(lz);
    // 64 bits kept, 64 below for the rounding
    let mut q = u64::try_from(r >> 64).unwrap_or(0);
    let rem = r & u128::from(u64::MAX);
    let half = 1u128 << 63;
    let mut exp = exp;
    if rem > half || (rem == half && q & 1 == 1) {
        let (n, carry) = q.overflowing_add(1);
        q = if carry {
            exp += 1;
            1 << 63
        } else {
            n
        };
    }
    Ext {
        neg: false,
        exp: u16::try_from(exp).unwrap_or(0),
        mant: q,
    }
}

/// `fldpi` under round to nearest.
const PI: Ext = Ext {
    neg: false,
    exp: 0x4000,
    mant: 0xc90f_daa2_2168_c235,
};

/// The x87 default NaN (`INDEFINITE`), as the DLL loads it for a domain
/// error (`0x10060320`).
const INDEFINITE: Ext = Ext {
    neg: true,
    exp: 0x7fff,
    mant: 0xc000_0000_0000_0000,
};

/// A NaN quieted (`fadd` of a signalling one).
fn quiet(x: Ext) -> Ext {
    Ext {
        mant: x.mant | (1 << 62),
        ..x
    }
}

/// The NaN an x87 operation on two NaNs gives: the larger significand,
/// quieted; of two equal ones the positive (as the DLL's results show).
fn nan_of(a: Ext, b: Ext) -> Ext {
    match (is_nan(a), is_nan(b)) {
        (true, true) => {
            let (ma, mb) = (quiet(a).mant, quiet(b).mant);
            if ma > mb || (ma == mb && !a.neg) {
                quiet(a)
            } else {
                quiet(b)
            }
        }
        (false, true) => quiet(b),
        _ => quiet(a),
    }
}

fn is_nan(x: Ext) -> bool {
    x.exp == 0x7fff && x.mant << 1 != 0
}

// ---------------------------------------------------- the path's state

/// The SSE2 paths' condition: the DLL's flag (set without the DLL), the
/// default `MXCSR` masks and rounding, every x87 exception masked.
fn sse2_path(e: &Exec) -> bool {
    let flag = e
        .shared()
        .dll(CRT_FILE)
        .is_none_or(|d| e.mem().read::<u32>(d.base + (USE_SSE2 - DLL_BASE)).ne(&0));
    flag && e.mxcsr() & 0x7f80 == 0x1f80 && e.fpu_control() & 0x7f == 0x7f
}

/// The DLL's constants, when the player's file is the known build.
fn data(e: &Exec) -> Option<&CrtData> {
    e.shared().crt_data.as_ref()
}

use ffx_bridge::Mem as _;

// ------------------------------------------------------------- sin, cos

/// Where `_CIsin`'s and `_CIcos`'s SSE2 paths (`0x1004ba00`,
/// `0x10044aef`) keep their constants: 32/π, the rounding shifter
/// 1.5 × 2^52, π/32 in three parts (the middle one as a pair, one per
/// polynomial lane), the two polynomials' coefficient pairs, the 64-entry
/// table of (cos − 1, sin, sin's low part, 1) at the multiples of π/32,
/// and the index offset (`_CIcos`'s is 16 entries, a quarter turn, on).
/// The two functions' constants and tables hold the same values; each
/// reads its own.
struct SinCos {
    inv_step: u32,
    shifter: u32,
    step_mid: u32,
    step_hi: u32,
    step_lo: u32,
    p_a: u32,
    p_b: u32,
    q_a: u32,
    q_b: u32,
    table: u32,
    index_offset: u32,
    /// `_CIsin`'s tiny-argument factor (just under 1), `_CIcos`'s 1.
    tiny: u32,
    cos: bool,
}

const SIN: SinCos = SinCos {
    inv_step: 0x1004_c410,
    shifter: 0x1004_c418,
    step_mid: 0x1004_c420,
    step_hi: 0x1004_c430,
    step_lo: 0x1004_c438,
    p_a: 0x1004_c400,
    p_b: 0x1004_c3f0,
    q_a: 0x1004_c3e0,
    q_b: 0x1004_c3d0,
    table: 0x1004_bbd0,
    index_offset: 0x001c_7600,
    tiny: 0x1004_c450,
    cos: false,
};

/// `_libm_sse2_sin_precise`'s copy (`0x1003dd21`: the same code and
/// values, its own constants).
const SIN_PRECISE: SinCos = SinCos {
    inv_step: 0x1003_e700,
    shifter: 0x1003_e708,
    step_mid: 0x1003_e710,
    step_hi: 0x1003_e720,
    step_lo: 0x1003_e728,
    p_a: 0x1003_e6f0,
    p_b: 0x1003_e6e0,
    q_a: 0x1003_e6d0,
    q_b: 0x1003_e6c0,
    table: 0x1003_dec0,
    index_offset: 0x001c_7600,
    tiny: 0x1003_e740,
    cos: false,
};

/// `_libm_sse2_cos_precise`'s copy (`0x10036fb1`).
const COS_PRECISE: SinCos = SinCos {
    inv_step: 0x1003_7980,
    shifter: 0x1003_7988,
    step_mid: 0x1003_7990,
    step_hi: 0x1003_79a0,
    step_lo: 0x1003_79a8,
    p_a: 0x1003_7970,
    p_b: 0x1003_7960,
    q_a: 0x1003_7950,
    q_b: 0x1003_7940,
    table: 0x1003_7140,
    index_offset: 0x001c_7610,
    tiny: 0x1003_79b0,
    cos: true,
};

const COS: SinCos = SinCos {
    inv_step: 0x1004_54e0,
    shifter: 0x1004_54e8,
    step_mid: 0x1004_54f0,
    step_hi: 0x1004_5500,
    step_lo: 0x1004_5508,
    p_a: 0x1004_54d0,
    p_b: 0x1004_54c0,
    q_a: 0x1004_54b0,
    q_b: 0x1004_54a0,
    table: 0x1004_4ca0,
    index_offset: 0x001c_7610,
    tiny: 0x1004_5510,
    cos: true,
};

/// The SSE2 `sin`/`cos` of `x` (`c` says which), in the DLL's operation
/// order (every step one IEEE double operation, as `mulsd`/`addsd`/
/// `subsd` and each lane of `mulpd`/`addpd`/`subpd` do; additions and
/// products are written in either operand order, which IEEE arithmetic
/// doesn't tell apart), or `None` for the arguments the DLL hands to its
/// x87 path (large |x|, infinities, NaNs).
///
/// `x = k·π/32 + r` with `k` the nearest integer to `x·32/π` and `r`
/// reduced in three parts of π/32; with the table's entry
/// `j = k mod 64` (plus a quarter turn for cos),
/// `sin(a + r) = sin a + sin a·(cos r − 1) + cos a·sin r`, the two series
/// in `r` evaluated side by side (lane 0 and lane 1) and the small terms
/// summed before the large ones.
#[allow(clippy::many_single_char_names)] // the formula's x, k, j, r
fn sse2_sin_cos(d: &CrtData, c: &SinCos, x: f64) -> Option<f64> {
    let band = (top16(x) & 0x7fff).wrapping_sub(0x3030);
    if band > 0x10c5 {
        if band.cast_signed() > 0x10c5 {
            return None; // large, infinite or NaN: the x87 path
        }
        // tiny |x|
        return Some(if c.cos {
            d.f(c.tiny) - x.abs()
        } else if band >> 4 == 0xcfd {
            // a zero or denormal exponent
            x * d.f(c.tiny)
        } else {
            x
        });
    }
    let step_mid = d.p(c.step_mid);
    let (p_a, p_b) = (d.p(c.p_a), d.p(c.p_b));
    let (q_a, q_b) = (d.p(c.q_a), d.p(c.q_b));
    // the multiple of π/32 nearest to x (cvtsd2si rounds to nearest even)
    let k_real = d.f(c.inv_step) * x;
    let k_int = cvtsd2si(k_real);
    let shifter = d.f(c.shifter);
    let k = (k_real + shifter) - shifter;
    let j = k_int.cast_unsigned().wrapping_add(c.index_offset) & 0x3f;
    let entry = c.table + 32 * j;
    let cos_m1 = d.f(entry);
    let sin_a = d.f(entry + 8);
    let sin_lo = d.f(entry + 0x10);
    let one = d.f(entry + 0x18);
    // r = x - k·π/32 in three parts, and what the reduction lost
    let r1 = x - d.f(c.step_hi) * k;
    let k_mid = P2::dup(k) * step_mid;
    let k_lo = k * d.f(c.step_lo);
    let r = r1 - k_mid.0;
    let rr = P2::dup(r1) - k_mid;
    let lost = (r1 - r) - k_mid.0;
    let correction = k_lo - lost;
    // the two series in r, lane by lane
    let r2 = rr * rr;
    let r4 = r2 * r2;
    let p_lo = p_a * P2::dup(r1) * rr;
    let p = (p_lo + p_b) * r4;
    let q = q_a * r2 + q_b;
    let cos_a = cos_m1 + one;
    let weights = P2(cos_a * r * r2.0, sin_a * r2.1);
    let series = (q + p) * weights;
    // the large terms, sin a + 1·r + (cos a − 1)·r, with their tails
    let one_r = one * r;
    let cos_m1_r = r * cos_m1;
    let head = one_r + sin_a;
    let sum = cos_m1_r + head;
    let head_tail = (sin_a - head) + one_r;
    let sum_tail = (head - sum) + cos_m1_r;
    let fixed = correction * (sin_a * r - cos_a) + sin_lo;
    let small = fixed + head_tail + sum_tail + series.0 + series.1;
    Some(sum + small)
}

/// The x87 path of `_CIsin`/`_CIcos`/`_CItan` for a finite `x` the SSE2
/// code handed on (`0x100327c0`, `0x10032ef9`, `0x100330ae`): the
/// instruction at the caller's control word (`0x027f`; another one the
/// DLL resets to it), and for |x| ≥ 2^63, which the instruction refuses
/// (C2), `fprem1` by π·2^62 (`0x10060346`) until it is in range.
fn x87_trig(d: &CrtData, op: Op, x: f64, cw: u32) -> Option<Ext> {
    if cw != 0x027f {
        return None;
    }
    let cw = 0x027f;
    let mut v = ext_of(x);
    for _ in 0..4 {
        let r = execute(op, v, v, cw);
        if r.c[2] != Some(true) {
            // fptan pushes 1 over tan: the DLL pops it
            return if op == Op::Fptan { r.st1 } else { r.st0 };
        }
        let modulus = d.t(0x1006_0346);
        loop {
            let p = execute(Op::Fprem1, v, modulus, cw);
            v = p.st0?;
            if p.c[2] != Some(true) {
                break;
            }
        }
    }
    None
}

/// `_CIsin`/`_CIcos` of `x`: the SSE2 path, its x87 fallback for large
/// finite `x`; `None` (the DLL's export) for infinities and NaNs (errno,
/// the matherr hook) and other control settings.
fn sin_cos(e: &Exec, c: &SinCos, st: &[f64]) -> Option<Ext> {
    let &[x] = st else { return None };
    let d = data(e)?;
    if !sse2_path(e) {
        return None;
    }
    if let Some(v) = sse2_sin_cos(d, c, x) {
        return Some(ext_of(v));
    }
    if !x.is_finite() {
        return None;
    }
    x87_trig(
        d,
        if c.cos { Op::Fcos } else { Op::Fsin },
        x,
        e.fpu_control(),
    )
}

/// `_CIsqrt` (`0x10032f48`, x87 only: `fsqrt` under the control word):
/// for a positive or zero finite `x` at 53-bit precision and round to
/// nearest, the correctly rounded square root, as the processor's `fsqrt`
/// gives it; `None` (the DLL's export and its error handling) otherwise.
fn sqrt(e: &Exec, st: &[f64]) -> Option<Ext> {
    let &[x] = st else { return None };
    let cw = e.fpu_control();
    if cw & 0xf00 != 0x200 || !(x >= 0.0 && x.is_finite()) {
        return None;
    }
    Some(ext_of(x.sqrt()))
}

// ------------------------------------------------------------------ tan

/// `__libm_sse2_tan` (`0x1004c476`): the constants. 32/π (both lanes),
/// the shifters (lane 0 keeps multiples of 16, lane 1 integers), π/32 in
/// three parts as pairs and the last one alone, the 32 table entries of
/// 0xb0 bytes, 1.0, the mask that cuts a double to 34 significant bits,
/// and the tiny-argument scale pair.
/// The constants block's start (`32/π` pair; the rest at fixed offsets)
/// and the table: `_CItan`'s and `_libm_sse2_tan_precise`'s copies
/// (`0x1003e877`, the same code).
#[derive(Clone, Copy)]
struct Tan {
    k: u32,
    table: u32,
}

const TAN: Tan = Tan {
    k: 0x1004_dcb0,
    table: 0x1004_c6b0,
};

const TAN_PRECISE: Tan = Tan {
    k: 0x1004_00b0,
    table: 0x1003_eab0,
};

impl Tan {
    const fn at(self, off: u32) -> u32 {
        self.k + off
    }
}

const TAN_INV_STEP: u32 = 0;
const TAN_SHIFTER: u32 = 0x10;
const TAN_STEP_HI: u32 = 0x20;
const TAN_STEP_MID: u32 = 0x30;
const TAN_STEP_LO: u32 = 0x40;
const TAN_STEP_MID1: u32 = 0x50;
const TAN_ONE: u32 = 0x58;
const TAN_CUT: u32 = 0x60;
const TAN_TINY_UP: u32 = 0x70;
const TAN_TINY_DOWN: u32 = 0x78;

/// The SSE2 `tan` of `x`, operation for operation as the DLL's
/// (`0x1004c476`), or `None` for the arguments it hands to the x87 path
/// (|x| from about 2^13, infinities, NaNs).
///
/// `x = k·π/32 + r` (`k` mod 32 picks one of the table's 32 intervals of
/// a half turn), `r` reduced in three parts. Each entry holds a
/// polynomial in `r` (lane 1 the high coefficients, lane 0 the low ones)
/// and the interval's `tan` split into parts; the result adds the
/// polynomial to the entry's base value and corrects with the reciprocal
/// of the reduced argument, cut to 34 bits so its products are exact.
#[allow(clippy::many_single_char_names, clippy::similar_names)]
fn sse2_tan(d: &CrtData, c: Tan, x: f64) -> Option<f64> {
    let band = (top16(x) & 0x7fff).wrapping_sub(0x3820);
    if band > 0x8a8 {
        if band.cast_signed() > 0x8a8 {
            return None; // large, infinite or NaN: the x87 path
        }
        // tiny |x| (zeros and denormals included): x + x·2^54·2^-54
        // rounded as the DLL does
        return Some((d.f(c.at(TAN_TINY_UP)) * x + x) * d.f(c.at(TAN_TINY_DOWN)));
    }
    let xx = P2::dup(x);
    let k_real = d.p(c.at(TAN_INV_STEP)) * xx;
    let k_int = cvtsd2si(k_real.0);
    let shifter = d.p(c.at(TAN_SHIFTER));
    let k = (k_real + shifter) - shifter;
    let index = k_int.cast_unsigned().wrapping_add(0x72900) & 0x1f;
    let entry = c.table + 0xb0 * index;
    let t = |off: u32| d.p(entry + off);
    let t1 = |off: u32| d.f(entry + off);
    // the reduction: r1 = x - k·hi, r2 = r1 - k·mid, and what each lost
    let k_hi = d.p(c.at(TAN_STEP_HI)) * k;
    let k_mid = d.p(c.at(TAN_STEP_MID)) * k;
    let r1 = xx - k_hi;
    let k_lo = k * d.p(c.at(TAN_STEP_LO));
    // lane 0's reduced argument, cut for the reciprocal
    let r_sum = d.f(c.at(TAN_STEP_MID1)) * k.0 + r1.0;
    let r_cut = and(r_sum, d.q(c.at(TAN_CUT)));
    let r2 = r1 - k_mid;
    let lost = ((r1 - r2) - k_mid) - k_lo;
    let r = r2.1;
    let rr = P2::dup(r);
    let recip = d.f(c.at(TAN_ONE)) / r_cut;
    let cut_tail = r2.0 - r_cut;
    let lost0 = lost.0 + cut_tail;
    // the entry's polynomial in r, both lanes
    let r_2 = rr * rr;
    let a = t(0x10) * rr + t(0);
    let b = (t(0x30) * rr + t(0x20)) * r_2;
    let cc = t(0x60) * rr + t(0x50);
    let a = a + b;
    let cc = cc + t(0x70) * r_2;
    let r_4 = r_2 * r_2;
    let a = a + t(0x40) * r_4;
    let r_5 = rr * r_4;
    let r_8 = r_4.0 * r_4.0;
    let lin = rr * t(0x90);
    let a = a + cc * r_5;
    let poly = r_8 * a.0 + a.1;
    // the linear term's two lanes summed, with the sum's tail
    let lin_sum = lin.0 + lin.1;
    let lin_tail = lin.1 + (lin.0 - lin_sum);
    let slope = (t1(0x90) + t1(0x98)) * lost.1 + t1(0x88) + lin_tail;
    let poly = poly + slope;
    // the correction through the reciprocal
    let lost_q = lost0 * recip;
    let cut = and(t1(0xa8), recip.to_bits());
    let num = (d.f(c.at(TAN_ONE)) - r_cut * cut) - lost_q;
    let corr = num * (recip * t1(0xa0));
    let base = cut - t1(0x80);
    let head = lin_sum - base;
    let base_tail = lin_sum - (base + head);
    Some(((poly + base_tail) - corr) + head)
}

/// `_CItan`: the SSE2 path, the x87 one for the large finite arguments;
/// `None` for infinities and NaNs (errno through the DLL's handler) and
/// other control settings.
fn tan(e: &Exec, st: &[f64]) -> Option<Ext> {
    let &[x] = st else { return None };
    let d = data(e)?;
    if !sse2_path(e) {
        return None;
    }
    if let Some(v) = sse2_tan(d, TAN, x) {
        return Some(ext_of(v));
    }
    if !x.is_finite() {
        return None;
    }
    x87_trig(d, Op::Fptan, x, e.fpu_control())
}

// ------------------------------------------- the x87 two-operand ones

/// The class `fxam` gives an operand, as the DLL's dispatcher
/// (`0x10042327`, table `0x10060334`) sorts it. An 80-bit denormal
/// counts as zero; the operands here are doubles, whose denormals load
/// as normal 80-bit values.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Normal,
    Zero,
    Nan,
    Inf,
}

fn class(x: Ext) -> Class {
    if x.exp == 0x7fff {
        if x.mant << 1 == 0 {
            Class::Inf
        } else {
            Class::Nan
        }
    } else if x.exp == 0 {
        Class::Zero
    } else {
        Class::Normal
    }
}

/// The dispatcher's NaN cases (`0x100423f8`/`0x100423fa`/`0x10042422`):
/// the NaN operand quieted, or for two the x87 choice.
fn nan_case(a: Ext, b: Ext) -> Ext {
    nan_of(a, b)
}

/// Whether the dispatcher's exit (`0x10044906`) leaves a finite result
/// alone: its double (`fst qword`) has a normal, non-zero exponent field
/// (else the DLL's underflow/overflow handler rewrites it).
fn in_double_range(x: Ext) -> bool {
    let e = (ext_to_f64(x).to_bits() >> 52) & 0x7ff;
    e != 0 && e != 0x7ff
}

/// `_CIatan2` (`0x100326dd`, x87 only: the dispatcher with table
/// `0x10060350`, precision 64 bits): `st` is `[y, x]` (`x` in `ST0`).
/// Both normal: `fpatan` of the magnitudes, π minus it for a negative
/// `x`, the sign of `y`; `y` zero (or denormal) or `x` infinite: 0 or π
/// with `y`'s sign; `x` zero (or denormal) or `y` infinite: ±π/2; both
/// infinite: the indefinite NaN (a domain error); NaNs propagate. A
/// result the handler would rewrite (an underflowing quotient) and an
/// unusual control word: `None`.
fn atan2(e: &Exec, st: &[f64]) -> Option<Ext> {
    let &[y, x] = st else { return None };
    if e.fpu_control() != 0x027f {
        return None;
    }
    let d = data(e)?;
    let (ye, xe) = (ext_of(y), ext_of(x));
    let half_pi = d.t(0x1006_032a);
    let with_sign = |v: Ext, n: bool| if n { neg(v) } else { v };
    Some(match (class(ye), class(xe)) {
        (Class::Normal, Class::Normal) => {
            let r = execute(Op::Fpatan, abs(xe), abs(ye), 0x133f).st1?;
            let r = if xe.neg { sub_positive(PI, r) } else { r };
            let r = with_sign(r, ye.neg);
            if !in_double_range(r) {
                return None;
            }
            r
        }
        (Class::Normal | Class::Zero, Class::Inf) | (Class::Zero, Class::Normal | Class::Zero) => {
            let v = if xe.neg {
                PI
            } else {
                Ext {
                    neg: false,
                    exp: 0,
                    mant: 0,
                }
            };
            with_sign(v, ye.neg)
        }
        (Class::Normal | Class::Inf, Class::Zero) | (Class::Inf, Class::Normal) => {
            with_sign(half_pi, ye.neg)
        }
        (Class::Inf, Class::Inf) => INDEFINITE,
        (Class::Nan, _) | (_, Class::Nan) => through_handler(nan_case(ye, xe)),
    })
}

/// A result that passes the dispatcher's error handler (`0x100449eb`:
/// stored as a double, the handler called, loaded back).
fn through_handler(x: Ext) -> Ext {
    ext_of(ext_to_f64(x))
}

/// `_CIfmod` (`0x100328e1`, x87 only: the dispatcher with table
/// `0x10060290`): `st` is `[x, y]` (`y` in `ST0`). Both normal: `fprem`
/// until complete (the exact remainder; an exact zero is +0); `x`
/// zero and `y` normal: +0 (`fldz`); `y` zero or
/// infinite, `x` infinite: the indefinite NaN (a domain error, through
/// the handler); NaNs propagate.
fn fmod(e: &Exec, st: &[f64]) -> Option<Ext> {
    let &[x, y] = st else { return None };
    if e.fpu_control() != 0x027f {
        return None;
    }
    let (xe, ye) = (ext_of(x), ext_of(y));
    Some(match (class(xe), class(ye)) {
        (Class::Normal, Class::Normal) => {
            // fprem until C2 clears (0x100328eb)
            let mut v = xe;
            loop {
                let r = execute(Op::Fprem, v, ye, 0x133f);
                v = r.st0?;
                if r.c[2] != Some(true) {
                    break v;
                }
            }
        }
        (Class::Zero, Class::Normal) => Ext {
            neg: false,
            exp: 0,
            mant: 0,
        },
        (Class::Nan, _) | (_, Class::Nan) => through_handler(nan_case(xe, ye)),
        _ => INDEFINITE,
    })
}

// ----------------------------------------------------------------- atan

/// `atan`'s constant block (`_libm_sse2_atan_precise` `0x10036e30`,
/// `_CIatan`'s copy `0x100446a0`: the same values at the same offsets)
/// and the shared table of (atan(c) high, low, c) records for c = k/32
/// (`0x100604a8`; the record at word `0x300` holds π/2 for |x| ≥ 8).
const ATAN_PRECISE: u32 = 0x1003_6e30;
const ATAN_CI: u32 = 0x1004_46a0;
const ATAN_TABLE: u32 = 0x1006_04a8;

/// Offsets in the block: ±π/2, the two polynomials' coefficient pairs
/// (Q from `+0x30`, P from `+0xb0`, the highest first at the end), the
/// index bias, -1 and 1, the shifter 8, the band limits (8, 3/8, 1/32,
/// 2^-27, about 1.6e16) and the tiny value added to ±π/2.
const ATAN_HALF_PI: u32 = 0x20;
const ATAN_Q: [u32; 8] = [0xa0, 0x90, 0x80, 0x70, 0x60, 0x50, 0x40, 0x30];
const ATAN_P: [u32; 4] = [0xe0, 0xd0, 0xc0, 0xb0];
const ATAN_BIAS: u32 = 0xf0;
const ATAN_MINUS_ONE: u32 = 0x100;
const ATAN_ONE: u32 = 0x108;
const ATAN_SHIFTER: u32 = 0x110;
const ATAN_EIGHT: u32 = 0x118;
const ATAN_TABLE_FROM: u32 = 0x120;
const ATAN_SERIES_FROM: u32 = 0x128;
const ATAN_TINY_BELOW: u32 = 0x130;
const ATAN_HUGE_FROM: u32 = 0x138;
const ATAN_NUDGE: u32 = 0x148;

/// The SSE2 `atan` of `x` (`0x10036b5e`, `_CIatan`'s `0x100443c6`), or
/// `None` for a NaN (the DLL's handler).
///
/// |x| below 2^-27: `x`. Below 1/32: `x - x·P(x²)`; below 3/8: the same
/// with the longer polynomial Q (each a pair of polynomials in x⁴, lane 0
/// times x² plus lane 1). Up to 8: the table's c = k/32 just below |x|,
/// `u = (|x| - c) / (|x|·c + 1)` and `atan c + atan u` as
/// `hi - ((u·P(u²) - lo) - u)` with x's sign; from 8 the record of π/2
/// with u = -1/|x|. From about 1.6e16 (infinities too): ±π/2 plus a tiny
/// value, as the DLL rounds it.
fn sse2_atan(d: &CrtData, base: u32, x: f64) -> Option<f64> {
    let k = |off: u32| d.f(base + off);
    let a = x.abs();
    if a.is_nan() {
        return None;
    }
    let series = |offs: &[u32], v: f64| {
        let v2 = P2::dup(v * v);
        let v4 = v2 * v2;
        let mut p = d.p(base + offs[0]);
        for &o in &offs[1..] {
            p = p * v4 + d.p(base + o);
        }
        p.0 * v2.0 + p.1
    };
    if a >= k(ATAN_HUGE_FROM) {
        let half_pi = k(ATAN_HALF_PI + if x.is_sign_negative() { 8 } else { 0 });
        return Some(k(ATAN_NUDGE) + half_pi);
    }
    if a < k(ATAN_TINY_BELOW) {
        return Some(x);
    }
    if a < k(ATAN_SERIES_FROM) {
        return Some(x - series(&ATAN_P, a) * x);
    }
    if a < k(ATAN_TABLE_FROM) {
        return Some(x - series(&ATAN_Q, a) * x);
    }
    let (u, record) = if a >= k(ATAN_EIGHT) {
        (k(ATAN_MINUS_ONE) / a, 0x300)
    } else {
        // the record from |x| + 8's top bits (its exponent and 8 bits of
        // mantissa: k = floor of 32·|x| after the rounding), three words each
        let top = u32::try_from((a + k(ATAN_SHIFTER)).to_bits() >> 44).unwrap_or(0);
        let bias = u32::try_from(d.q(base + ATAN_BIAS) & 0xffff_ffff).unwrap_or(0);
        let i = top.wrapping_sub(bias).wrapping_mul(3);
        let c = d.f(ATAN_TABLE.wrapping_add(i.wrapping_mul(8)) + 0x10);
        ((a - c) / (a * c + k(ATAN_ONE)), i)
    };
    let at = ATAN_TABLE.wrapping_add(record * 8);
    let (hi, lo) = (d.f(at), d.f(at + 8));
    let tail = (series(&ATAN_P, u) * u - lo) - u;
    let r = hi - tail;
    Some(f64::from_bits(r.to_bits() | (x.to_bits() & (1 << 63))))
}

/// `_libm_sse2_atan_precise` (`0x10036b28`).
fn atan_precise(e: &Exec, x: f64) -> Option<f64> {
    let d = data(e)?;
    if !sse_controls(e) {
        return None;
    }
    match sse2_atan(d, ATAN_PRECISE, x) {
        Some(v) => Some(v),
        None => non_finite(e, x),
    }
}

/// `_CIatan` (`0x100325ff`): the SSE2 code (`0x100443a8`) on the SSE2
/// path; its NaNs and the x87 path: `None`.
fn atan(e: &Exec, st: &[f64]) -> Option<Ext> {
    let &[x] = st else { return None };
    let d = data(e)?;
    if !sse2_path(e) {
        return None;
    }
    sse2_atan(d, ATAN_CI, x).map(ext_of)
}

// ----------------------------------------------------------------- acos

/// `acos`'s constants (`_libm_sse2_acos_precise`'s block at `0x10034d80`;
/// `_CIacos`'s copy: [`ACOS_CI`]) and its two tables: per interval c of
/// |x| a word `√(1 - c²)` (`T1`) and a pair (`T2`: acos c's low and high
/// parts).
#[derive(Clone, Copy)]
struct AcosK {
    k: u32,
    t1: u32,
    t2: u32,
}

const ACOS_PRECISE: AcosK = AcosK {
    k: 0x1003_4d80,
    t1: 0x1003_4580,
    t2: 0x1003_3680,
};

const ACOS_CI: AcosK = AcosK {
    k: 0x1004_2200,
    t1: 0x1004_1a00,
    t2: 0x1004_0b00,
};

/// Offsets in the block.
const AC_PI_2: u32 = 0x00; // (6.1e-17, π/2)
const AC_MINUS_PI: u32 = 0x10; // (-1.2e-16, -π)
const AC_PI: u32 = 0x20; // (1.2e-16, π)
const AC_CUT27: u32 = 0x30;
const AC_C3: u32 = 0x48; // -1/6
const AC_C5: u32 = 0x50; // -3/40
const AC_C7: u32 = 0x58; // -5/112
const AC_SMALL_A: u32 = 0x60; // pairs: (x⁹ terms, x³ terms)
const AC_SMALL_B: u32 = 0x70;
const AC_SMALL_C: u32 = 0x80;
const AC_ABS: u32 = 0xa0;
const AC_CUT18: u32 = 0xa8;
const AC_ONE: u32 = 0xb0;
const AC_HALF: u32 = 0xb8;

/// The bit that makes a cut value its interval's midpoint.
const AC_MID: u64 = 0x0000_2000_0000_0000;
const SIGN: u64 = 1 << 63;

fn with_sign_of(v: f64, x: f64) -> f64 {
    f64::from_bits(v.to_bits() ^ (x.to_bits() & SIGN))
}

/// The odd series `C3·w³ + (C7·w² + C5)·w⁵` of the bands with a table,
/// in the DLL's order: (the w⁵ part, the w³ part).
fn ac_series(d: &CrtData, c: AcosK, w: f64) -> (f64, f64) {
    let k = |o: u32| d.f(c.k + o);
    let w2 = w * w;
    let w3 = w * w2;
    let w5 = w3 * w2;
    ((k(AC_C7) * w2 + k(AC_C5)) * w5, k(AC_C3) * w3)
}

/// The SSE2 `acos` of `x` (`0x1003318b`, `_CIacos`'s `0x100405be`), or
/// `None` for the arguments the DLL's error handler takes (|x| > 1, NaN).
///
/// The bands by |x|'s exponent and top mantissa bits:
/// - 1/16 to about 0.865: c = |x| cut to 6 mantissa bits plus half an
///   interval (x's sign), w = x·√(1-c²) - c·√(1-x²) = sin(asin x - asin c),
///   the table's acos c (sign-adjusted) minus asin w by its series, with
///   the division `(x+c)(x-c) / (x·T1 + c·√(1-x²))` correcting w;
/// - about 0.865 to 0.992: the same with the roles of x and √(1-x²)
///   swapped (s = √(1-x²) cut, its table entry), x's sign at the end;
/// - 2^-60 to 1/16: `π/2 - x` minus `x³·Q(x²)` in two parts;
/// - 0.992 to 1: `2·√((1-|x|)/2)` times a series in y = (1-|x|)/2, with
///   π - ... for negative x;
/// - below 2^-60: π/2; ±1: 0 or π.
#[allow(clippy::too_many_lines, clippy::similar_names)]
fn sse2_acos(d: &CrtData, c: AcosK, x: f64) -> Option<f64> {
    let k = |o: u32| d.f(c.k + o);
    let kp = |o: u32| d.p(c.k + o);
    let bits = x.to_bits();
    let top = u32::try_from(bits >> 44).unwrap_or(0);
    let e0 = top & 0x7ffff;
    let one = k(AC_ONE);
    if e0.wrapping_sub(0x3fb00) < 0x3bb {
        // 1/16 ≤ |x| < 0.865
        let s = (one - x * x).sqrt();
        let cut = f64::from_bits((bits & d.q(c.k + AC_CUT18)) | AC_MID);
        let i = ((top & 0xffff) & !3).wrapping_sub(0xfb00);
        let t1 = d.f(c.t1.wrapping_add(i.wrapping_mul(2)));
        let t2 = d.p(c.t2.wrapping_add(i.wrapping_mul(4)));
        let num = (x + cut) * (x - cut);
        let x_t1 = x * t1;
        let s_c = s * cut;
        let corr = num / (x_t1 + s_c);
        let w = x_t1 - s_c;
        let sign = bits & SIGN;
        let t2 = P2(
            f64::from_bits(t2.0.to_bits() ^ sign),
            f64::from_bits(t2.1.to_bits() ^ sign),
        ) - kp(AC_PI_2);
        let (w5, w3) = ac_series(d, c, w);
        return Some(((w5 + (w3 - t2.0)) - corr) - t2.1);
    }
    if e0.wrapping_sub(0x3fb00 + 0x3bb) < 0x41 {
        // 0.865 ≤ |x| < 0.992
        let xt = f64::from_bits(bits >> 38 << 38);
        let x_lo = x - xt;
        let one_m = one - xt * xt;
        let tail = (x + xt) * x_lo;
        let s = (one_m - tail).sqrt();
        let st = f64::from_bits((s.to_bits() & d.q(c.k + AC_CUT18)) | AC_MID | (bits & SIGN));
        let i = u32::try_from(((s.to_bits() << 2) >> 48) & 0xffff)
            .unwrap_or(0)
            .wrapping_sub(0xfec0)
            .wrapping_mul(2);
        let t1 = d.f(c.t1.wrapping_add(i.wrapping_mul(4)));
        let negative = bits & SIGN != 0;
        let base = if negative {
            kp(AC_MINUS_PI)
        } else {
            P2(0.0, 0.0)
        };
        let base = base + d.p(c.t2.wrapping_add(i.wrapping_mul(8)));
        let s_t1 = s * t1;
        let w = (xt * st - s_t1) + x_lo * st;
        let den = (s_t1 + s_t1) + w;
        let corr = ((one_m - st * st) - tail) / den;
        let (w5, w3) = ac_series(d, c, w);
        let head = corr + base.1;
        let head_tail = corr + (base.1 - head);
        let r = (w5 + (w3 + base.0)) + head_tail + head;
        return Some(if negative { -r } else { r });
    }
    if e0.wrapping_sub(0x3c300) < 0x3800 {
        // 2^-60 ≤ |x| < 1/16
        let (pa, pb, pc) = (kp(AC_SMALL_A), kp(AC_SMALL_B), kp(AC_SMALL_C));
        let xx = P2::dup(x);
        let x2 = xx * xx;
        let x3 = xx * x2;
        let x4 = x2 * x2;
        let x9 = (x3.0 * x3.0) * x3.0;
        let q = (pa * x2 + pb) + pc * x4;
        let parts = P2(x9, x3.1) * q;
        let pi_2 = kp(AC_PI_2);
        let head = pi_2.1 - x;
        let lost = x - (pi_2.1 - head);
        let low = ((pi_2.0 - parts.0) - parts.1) - lost;
        return Some(head + low);
    }
    if e0.wrapping_sub(0x3fefc) < 4 {
        // 0.992 ≤ |x| < 1
        let a = x.abs();
        let y = k(AC_HALF) - a * k(AC_HALF);
        let r = y.sqrt();
        let yy = P2::dup(y);
        let y2 = yy * yy;
        let q = (kp(AC_SMALL_B) + kp(AC_SMALL_A) * yy) + kp(AC_SMALL_C) * y2;
        let y3 = y2.0 * y;
        let q = P2(q.0 * y3, q.1) * yy;
        let rt = f64::from_bits(r.to_bits() & d.q(c.k + AC_CUT27));
        let r_lo = r - rt;
        let corr = ((y - rt * rt) - r_lo * ((r + r) - r_lo)) / r;
        let q = q * P2::dup(r + r);
        let negative = bits & SIGN != 0;
        let base = if negative {
            kp(AC_MINUS_PI)
        } else {
            P2(0.0, 0.0)
        };
        let sum = (((q.0 + base.0) + q.1) + corr) + (r + r);
        let v = base.1 + sum;
        return Some(if negative { -v } else { v });
    }
    if e0 < 0x3ff00 {
        // below 2^-60
        let pi_2 = kp(AC_PI_2);
        return Some(pi_2.0 + pi_2.1);
    }
    if x.abs() == 1.0 {
        let pi = kp(AC_PI);
        return Some(if x < 0.0 { pi.0 + pi.1 } else { 0.0 + 0.0 });
    }
    let _ = k(AC_ABS);
    None
}

/// `_libm_sse2_acos_precise` (`0x10033155`): the SSE2 code; |x| > 1
/// (the indefinite NaN, a domain error) and NaNs through the DLL's
/// handler.
fn acos_precise(e: &Exec, x: f64) -> Option<f64> {
    let d = data(e)?;
    if !sse_controls(e) {
        return None;
    }
    if let Some(v) = sse2_acos(d, ACOS_PRECISE, x) {
        return Some(v);
    }
    non_finite(e, if x.is_nan() { x } else { f64::INFINITY })
}

/// `_CIacos` (`0x10032375`): the SSE2 code (`0x100405be`) on the SSE2
/// path; |x| > 1, NaNs and the x87 path: `None`.
fn acos(e: &Exec, st: &[f64]) -> Option<Ext> {
    let &[x] = st else { return None };
    let d = data(e)?;
    if !sse2_path(e) {
        return None;
    }
    sse2_acos(d, ACOS_CI, x).map(ext_of)
}

// ----------------------------------------------------------------- asin

/// `asin`'s constants and tables (`_libm_sse2_asin_precise`'s block at
/// `0x10036a90`, `_CIasin`'s at `0x10044310`), laid out as `acos`'s
/// ([`AcosK`]: per interval √(1 - c²) and asin c's two parts).
const ASIN_PRECISE: AcosK = AcosK {
    k: 0x1003_6a90,
    t1: 0x1003_6290,
    t2: 0x1003_5390,
};

const ASIN_CI: AcosK = AcosK {
    k: 0x1004_4310,
    t1: 0x1004_3b10,
    t2: 0x1004_2c10,
};

/// Offsets in the block.
const AS_PI_2: u32 = 0x00; // (6.1e-17, π/2)
const AS_CUT27: u32 = 0x10;
const AS_ABS_CUT18: u32 = 0x20;
const AS_C3: u32 = 0x28; // 1/6
const AS_C5: u32 = 0x30; // 3/40
const AS_C7: u32 = 0x38; // 5/112
const AS_Q: [u32; 4] = [0x40, 0x50, 0x60, 0x70]; // pairs
const AS_CUT18: u32 = 0x88;
const AS_ONE: u32 = 0x90;

/// The odd series of the table bands, in the DLL's order: (the w⁵ part,
/// the w³ part).
fn as_series(d: &CrtData, c: AcosK, w: f64) -> (f64, f64) {
    let k = |o: u32| d.f(c.k + o);
    let w2 = w * w;
    let w3 = w * w2;
    let w5 = w3 * w2;
    ((k(AS_C5) + k(AS_C7) * w2) * w5, k(AS_C3) * w3)
}

/// The SSE2 `asin` of `x` (`0x10034e76`, `_CIasin`'s `0x100426aa`), or
/// `None` for |x| > 1 and NaNs (the DLL's error handler).
///
/// The bands as `acos`'s ([`sse2_acos`]): from 1/16 to about 0.865 the
/// table entry c near x, `asin c + asin w` with w and its correction
/// from √(1 - x²); to about 0.996 the same around √(1 - x²) (π/2 minus
/// the table's asin, x's sign at the end); below 1/16 `x + x³·Q(x²)`;
/// from about 0.996 `π/2 - asin √(1 - x²)` by the long series in
/// s = √(1 - x²) with a correction for s's rounding; below 2^-60: x; ±1:
/// ±π/2.
#[allow(clippy::too_many_lines, clippy::similar_names)]
fn sse2_asin(d: &CrtData, c: AcosK, x: f64) -> Option<f64> {
    let k = |o: u32| d.f(c.k + o);
    let kp = |o: u32| d.p(c.k + o);
    let bits = x.to_bits();
    let top = u32::try_from(bits >> 44).unwrap_or(0);
    let e0 = top & 0x7ffff;
    let one = k(AS_ONE);
    if e0.wrapping_sub(0x3fb00) < 0x3bb {
        // 1/16 ≤ |x| < 0.865
        let s = (one - x * x).sqrt();
        let cut = f64::from_bits((bits & d.q(c.k + AS_CUT18)) | AC_MID);
        let i = ((top & 0xffff) & !3).wrapping_sub(0xfb00);
        let t1 = d.f(c.t1.wrapping_add(i.wrapping_mul(2)));
        let t2 = d.p(c.t2.wrapping_add(i.wrapping_mul(4)));
        let num = (x - cut) * (x + cut);
        let x_t1 = x * t1;
        let s_c = s * cut;
        let corr = num / (x_t1 + s_c);
        let w = x_t1 - s_c;
        let sign = bits & SIGN;
        let t2 = P2(
            f64::from_bits(t2.0.to_bits() ^ sign),
            f64::from_bits(t2.1.to_bits() ^ sign),
        );
        let (w5, w3) = as_series(d, c, w);
        return Some((corr + (w5 + (w3 + t2.0))) + t2.1);
    }
    if e0.wrapping_sub(0x3fb00 + 0x3bb) < 0x43 {
        // 0.865 ≤ |x| < 0.996
        let s = (one - x * x).sqrt();
        let a = x.abs();
        let at = f64::from_bits(bits & d.q(c.k + AS_ABS_CUT18));
        let a_lo = a - at;
        let one_m = one - at * at;
        let tail = (a + at) * a_lo;
        let st = f64::from_bits((s.to_bits() & d.q(c.k + AS_CUT18)) | AC_MID);
        let i = u32::try_from(((s.to_bits() << 2) >> 48) & 0xffff)
            .unwrap_or(0)
            .wrapping_sub(0xfec0)
            .wrapping_mul(2);
        let s_t1 = s * d.f(c.t1.wrapping_add(i.wrapping_mul(4)));
        let w = (at * st - s_t1) + a_lo * st;
        let corr = ((one_m - st * st) - tail) / ((s_t1 + s_t1) + w);
        let base = kp(AS_PI_2) - d.p(c.t2.wrapping_add(i.wrapping_mul(8)));
        let (w5, w3) = as_series(d, c, w);
        let r = (((w5 + (w3 + base.0)) - corr) + base.1).to_bits();
        return Some(f64::from_bits(r | (bits & SIGN)));
    }
    if e0.wrapping_sub(0x3c300) < 0x3800 {
        // 2^-60 ≤ |x| < 1/16
        let xx = P2::dup(x);
        let x2 = xx * xx;
        let x3 = xx * x2;
        let x4 = x2 * x2;
        let x9 = (x3.0 * x3.0) * x3.0;
        let q = (kp(AS_Q[0]) * x2 + kp(AS_Q[1])) + kp(AS_Q[2]) * x4;
        let parts = P2(x9, x3.1) * q;
        return Some(x + (parts.0 + parts.1));
    }
    if e0.wrapping_sub(0x3fefe) < 2 {
        // 0.996 ≤ |x| < 1
        let s = (one - x * x).sqrt();
        let xt = f64::from_bits(bits & d.q(c.k + AS_CUT27));
        let st = f64::from_bits(s.to_bits() & d.q(c.k + AS_CUT27));
        let x_lo = x - xt;
        let r = (((one - xt * xt) - (xt + xt) * x_lo) - st * st) - x_lo * x_lo;
        let st_m = st - s;
        let corr = (r + ((s + s) + st_m) * st_m) / (s + s);
        let ss = P2::dup(s);
        let s2 = ss * ss;
        let s3 = ss * s2;
        let s4 = s2 * s2;
        let s6 = s3.0 * s3.0;
        let s9 = s3.0 * s6;
        let s15 = s6 * s9;
        let last = kp(AS_Q[3]);
        let q = kp(AS_Q[2]) * s4 + (kp(AS_Q[1]) + kp(AS_Q[0]) * s2);
        let q = q * P2(s9, s3.1);
        let long = (last.0 * s2.0 + last.1) * s15;
        let pi_2 = kp(AS_PI_2);
        let head = s - pi_2.1;
        let back = pi_2.1 + head;
        let rest = (q.0 + q.1) + corr;
        let v = (((pi_2.0 + long) - rest) - (s - back)) - head;
        return Some(f64::from_bits(v.to_bits() | (bits & SIGN)));
    }
    if e0 < 0x3ff00 {
        // below 2^-60: x
        return Some(x);
    }
    if x.abs() == 1.0 {
        let pi_2 = kp(AS_PI_2);
        let v = pi_2.1 + pi_2.0;
        return Some(f64::from_bits(v.to_bits() | (bits & SIGN)));
    }
    None
}

/// `_libm_sse2_asin_precise` (`0x10034e40`).
fn asin_precise(e: &Exec, x: f64) -> Option<f64> {
    let d = data(e)?;
    if !sse_controls(e) {
        return None;
    }
    if let Some(v) = sse2_asin(d, ASIN_PRECISE, x) {
        return Some(v);
    }
    non_finite(e, if x.is_nan() { x } else { f64::INFINITY })
}

/// `_CIasin` (`0x100324ba`): the SSE2 code (`0x100426aa`) on the SSE2
/// path; |x| > 1, NaNs and the x87 path: `None`.
fn asin(e: &Exec, st: &[f64]) -> Option<Ext> {
    let &[x] = st else { return None };
    let d = data(e)?;
    if !sse2_path(e) {
        return None;
    }
    sse2_asin(d, ASIN_CI, x).map(ext_of)
}

// ------------------------------------------------------------------ log

/// `_libm_sse2_log_precise`'s constants (`0x10038f60`) and tables (128
/// pairs each: the reciprocal r of each mantissa interval, and -log r in
/// two parts).
const LOG_K: u32 = 0x1003_8f60;
const LOG_RECIP: u32 = 0x1003_9020;
const LOG_NEG_LOG: u32 = 0x1003_9430;

const LOG_MANT: u32 = 0x00;
const LOG_ROUND: u32 = 0x10;
const LOG_CUT: u32 = 0x20;
const LOG_LN2: u32 = 0x30; // (ln 2 high, low)
const LOG_MASKS: u32 = 0x40; // (0, -1) and (-1, 0)
const LOG_ONE: u32 = 0x60;
const LOG_TWO52: u32 = 0x70;
const LOG_P: [u32; 3] = [0x90, 0xa0, 0xb0];

fn and2(v: P2, m: P2) -> P2 {
    P2(
        f64::from_bits(v.0.to_bits() & m.0.to_bits()),
        f64::from_bits(v.1.to_bits() & m.1.to_bits()),
    )
}

fn or2(v: P2, m: P2) -> P2 {
    P2(
        f64::from_bits(v.0.to_bits() | m.0.to_bits()),
        f64::from_bits(v.1.to_bits() | m.1.to_bits()),
    )
}

/// The SSE2 `log` of `x` (`0x10038d01`), or `None` for what the DLL's
/// error handler takes (zero, negative values, NaNs).
///
/// `x = 2^e·m` with m in [1, 2); r the table's reciprocal for m's top 7
/// bits, `u = m·r - 1` (with m split so `m_hi·r` is exact),
/// `log x = e·ln 2 - log r + log(1 + u)`, the series in u in two lanes;
/// a denormal x is scaled by 2^52 first. +∞: +∞.
fn sse2_log(d: &CrtData, x: f64) -> Option<f64> {
    let kp = |o: u32| d.p(LOG_K + o);
    let mut x = x;
    let mut bias = 0i32;
    loop {
        let biased = u32::try_from((x.to_bits() >> 52) & 0xfff).unwrap_or(0);
        let ecx = biased.wrapping_sub(1);
        let m = or2(and2(P2::dup(x), kp(LOG_MANT)), kp(LOG_ONE));
        if ecx > 0x7fd {
            if x == 0.0 {
                return None; // -∞, a pole error
            }
            if ecx == u32::MAX {
                // a positive denormal: scaled, the exponent corrected
                x *= d.f(LOG_K + LOG_TWO52);
                bias = -52;
                continue;
            }
            if ecx == 0x7fe && m.0 == d.f(LOG_K + LOG_ONE) {
                return Some(f64::INFINITY);
            }
            return None; // a NaN or a negative value
        }
        let index = u32::try_from((m + kp(LOG_ROUND)).0.to_bits() & 0x7f0).unwrap_or(0);
        let r = d.p(LOG_RECIP + index);
        let neg_log = d.p(LOG_NEG_LOG + index);
        let m_hi = and2(m, kp(LOG_CUT));
        let m_lo = m - m_hi;
        let hi_r = m_hi * r - kp(LOG_ONE);
        let mut acc = P2(neg_log.0 + hi_r.0, neg_log.1);
        let lo_r = m_lo * r;
        let u = lo_r + hi_r;
        let e = i32::try_from(biased).unwrap_or(0) - 0x3ff + bias;
        let exact = i32::try_from(index).unwrap_or(1).wrapping_add(e << 10) == 0;
        let mask = d.p(LOG_K + LOG_MASKS + if exact { 0x10 } else { 0 });
        let p = kp(LOG_P[0]) * u + kp(LOG_P[1]);
        let u2 = u * u;
        let u4 = u2.0 * u2.0;
        acc = acc + P2::dup(f64::from(e)) * kp(LOG_LN2);
        acc = acc + and2(lo_r, mask);
        let p = p * u + kp(LOG_P[2]);
        let u5 = u4 * u.0;
        let p = p * P2(u5, u2.1);
        return Some(((p.0 + p.1) + acc.1) + acc.0);
    }
}

/// `_libm_sse2_log_precise` (`0x10038cd0`): the SSE2 code; zero (-∞, a
/// pole error), negative values and NaNs (a domain error, the default
/// NaN; a NaN quieted) through the DLL's handler.
fn log_precise(e: &Exec, x: f64) -> Option<f64> {
    let d = data(e)?;
    if !sse_controls(e) {
        return None;
    }
    if let Some(v) = sse2_log(d, x) {
        return Some(v);
    }
    if e.shared().dll(CRT_FILE).is_some() {
        return None;
    }
    Some(if x == 0.0 {
        f64::NEG_INFINITY
    } else if x.is_nan() {
        f64::from_bits(x.to_bits() | (1 << 51))
    } else {
        f64::from_bits(0xfff8_0000_0000_0000)
    })
}

// --------------------------------------------- the SSE2 functions

/// Whether the SSE2 functions take their SSE2 code (`MXCSR` and the x87
/// control word as the intrinsics test them; no flag of their own).
fn sse_controls(e: &Exec) -> bool {
    e.mxcsr() & 0x7f80 == 0x1f80 && e.fpu_control() & 0x7f == 0x7f
}

/// What the DLL's x87 fallback of an SSE2 function gives back for a
/// non-finite argument, without the DLL: a NaN quieted, the indefinite
/// NaN for an infinity (a domain error). With the DLL: `None` (its
/// handler, errno).
fn non_finite(e: &Exec, x: f64) -> Option<f64> {
    if e.shared().dll(CRT_FILE).is_some() {
        return None;
    }
    Some(if x.is_nan() {
        f64::from_bits(x.to_bits() | (1 << 51))
    } else {
        f64::from_bits(0xfff8_0000_0000_0000)
    })
}

/// `_libm_sse2_sin_precise`/`_cos_precise` (`0x1003dcf0`, `0x10036f80`):
/// `_CIsin`'s and `_CIcos`'s SSE2 code with their own constants, the
/// result in `xmm0`; the large arguments through the x87 code, stored as
/// a double.
fn sin_cos_precise(e: &Exec, c: &SinCos, x: f64) -> Option<f64> {
    let d = data(e)?;
    if !sse_controls(e) {
        return None;
    }
    if let Some(v) = sse2_sin_cos(d, c, x) {
        return Some(v);
    }
    if !x.is_finite() {
        return non_finite(e, x);
    }
    x87_trig(
        d,
        if c.cos { Op::Fcos } else { Op::Fsin },
        x,
        e.fpu_control(),
    )
    .map(ext_to_f64)
}

/// `_libm_sse2_tan_precise` (`0x1003e846`): `_CItan`'s code.
fn tan_precise(e: &Exec, x: f64) -> Option<f64> {
    let d = data(e)?;
    if !sse_controls(e) {
        return None;
    }
    if let Some(v) = sse2_tan(d, TAN_PRECISE, x) {
        return Some(v);
    }
    if !x.is_finite() {
        return non_finite(e, x);
    }
    x87_trig(d, Op::Fptan, x, e.fpu_control()).map(ext_to_f64)
}

/// `_libm_sse2_sqrt_precise` (`0x1003e748`): `sqrtsd` for a positive
/// value, +0, -0 and +∞; a negative value (the indefinite NaN, a domain
/// error) and a NaN go through the DLL's handler: the DLL, or without it
/// those values.
fn sqrt_precise(e: &Exec, x: f64) -> Option<f64> {
    if !sse_controls(e) {
        return None;
    }
    if x >= 0.0 || x == 0.0 {
        return Some(x.sqrt());
    }
    non_finite(e, if x.is_nan() { x } else { f64::INFINITY })
}

/// An SSE2 function's Rust side: `xmm0` in, `xmm0` out, or `None` to run
/// the DLL's export.
pub type SseFn = fn(&Exec, f64) -> Option<f64>;

/// The SSE2 functions with a Rust side (`Exec::call_sse`), by import.
#[must_use]
pub fn sse_function(name: &str) -> Option<SseFn> {
    match name.strip_prefix("MSVCR110.dll!")? {
        "_libm_sse2_sin_precise" => Some(|e, x| sin_cos_precise(e, &SIN_PRECISE, x)),
        "_libm_sse2_cos_precise" => Some(|e, x| sin_cos_precise(e, &COS_PRECISE, x)),
        "_libm_sse2_tan_precise" => Some(tan_precise),
        "_libm_sse2_sqrt_precise" => Some(sqrt_precise),
        "_libm_sse2_atan_precise" => Some(atan_precise),
        "_libm_sse2_acos_precise" => Some(acos_precise),
        "_libm_sse2_asin_precise" => Some(asin_precise),
        "_libm_sse2_log_precise" => Some(log_precise),
        _ => None,
    }
}

/// The rows of [`NO_DLL`].
macro_rules! sse_rows {
    ($($name:literal),* $(,)?) => {
        &[$((CRT_FILE, $name, Conv::Cdecl, 0, sse_without_operand as ImportFn)),*]
    };
}

/// The SSE2 functions' import entries without the DLLs: the ports call
/// them through `Exec::call_sse` (`xmm0`); a call through the import's
/// stub has no operand and stops.
pub const NO_DLL: &[(&str, &str, Conv, usize, ImportFn)] = sse_rows![
    "_libm_sse2_sin_precise",
    "_libm_sse2_cos_precise",
    "_libm_sse2_tan_precise",
    "_libm_sse2_sqrt_precise",
    "_libm_sse2_atan_precise",
    "_libm_sse2_acos_precise",
    "_libm_sse2_asin_precise",
    "_libm_sse2_log_precise",
];

fn sse_without_operand(e: &mut Exec, _: &Args) -> Ret {
    e.end(crate::exec::Stop::Failed(
        "an SSE2 maths function called without its xmm0 operand".into(),
        0,
    ));
    Ret::VOID
}

/// An x87 intrinsic's Rust side: its operands (the last in `ST0`) to its
/// result, or `None` to run the DLL's export.
pub type X87Fn = fn(&Exec, &[f64]) -> Option<Ext>;

/// The `_CI*` intrinsics with a Rust side ([`intrinsic`]), with the
/// operands each takes from the x87 stack.
pub const INTRINSICS: [(&str, usize); 9] = [
    ("_CIasin", 1),
    ("_CIacos", 1),
    ("_CIatan", 1),
    ("_CIsin", 1),
    ("_CIcos", 1),
    ("_CIsqrt", 1),
    ("_CItan", 1),
    ("_CIatan2", 2),
    ("_CIfmod", 2),
];

/// How many x87 operands the intrinsic `name` (`MSVCR110.dll!_CIcos`)
/// takes.
#[must_use]
pub fn operands(name: &str) -> usize {
    let n = name.strip_prefix("MSVCR110.dll!").unwrap_or(name);
    INTRINSICS
        .iter()
        .find(|(i, _)| *i == n)
        .map_or(1, |(_, k)| *k)
}

/// The Rust side of the x87 intrinsic `name` (`MSVCR110.dll!_CIcos`, ...).
#[must_use]
pub fn intrinsic(name: &str) -> Option<X87Fn> {
    match name.strip_prefix("MSVCR110.dll!")? {
        "_CIsin" => Some(|e, st| sin_cos(e, &SIN, st)),
        "_CIcos" => Some(|e, st| sin_cos(e, &COS, st)),
        "_CIsqrt" => Some(sqrt),
        "_CItan" => Some(tan),
        "_CIatan" => Some(atan),
        "_CIacos" => Some(acos),
        "_CIasin" => Some(asin),
        "_CIatan2" => Some(atan2),
        "_CIfmod" => Some(fmod),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext_round_trips_doubles() {
        for x in [
            0.0,
            -0.0,
            1.0,
            -2.5,
            5e-324,
            -1e-310,
            f64::MAX,
            f64::MIN_POSITIVE,
            f64::INFINITY,
            std::f64::consts::PI,
        ] {
            assert_eq!(ext_to_f64(ext_of(x)).to_bits(), x.to_bits(), "{x:e}");
            assert!(is_double(ext_of(x)));
        }
        // π to 64 bits stores as the double π
        assert_eq!(ext_to_f64(PI), std::f64::consts::PI);
        assert!(!is_double(PI));
        #[allow(clippy::cast_possible_truncation)]
        let f = std::f64::consts::PI as f32;
        assert_eq!(ext_to_f32(PI), f);
    }

    #[test]
    fn sub_positive_rounds_to_64_bits() {
        let one = ext_of(1.0);
        let half = ext_of(0.5);
        assert_eq!(ext_to_f64(sub_positive(one, half)), 0.5);
        // π - 1 at 64 bits stores as the double π - 1 (no double rounding here)
        let r = sub_positive(PI, one);
        assert!((ext_to_f64(r) - (std::f64::consts::PI - 1.0)).abs() < 1e-15);
    }
}
