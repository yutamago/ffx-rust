//! Rust sides of the C runtime DLLs' functions (`MSVCR110.dll`,
//! `MSVCP110.dll`; docs/host.md, "The C runtime DLLs"), with the DLL's
//! observable results, for the imports `FFX.exe` and the ports call most.
//! Each was derived from the player's DLL's machine code; where a case is
//! not covered (an exception the DLL would throw, a stream with input), the
//! Rust side calls the DLL's own export, which runs in Unicorn as before.
//! `tests/crt_dll.rs` checks each against the DLL.

use ffx_bridge::Mem;

use crate::exec::{Exec, Stop};
use crate::imports::ImportFn;
use crate::table::{Args, Conv, Ret};

const CPP: &str = "MSVCP110.dll";

/// `std::basic_istream<char>::getline(char *, streamsize)`.
pub const GETLINE: &str = "?getline@?$basic_istream@DU?$char_traits@D@std@@@std@@QAEAAV12@PAD_J@Z";

/// `ios_base::_Mystate`, `_Except` and `basic_ios::_Mystrbuf`: offsets in
/// the `basic_ios` sub-object (MSVCP110's `setstate` `0x1000fc35`, `clear`
/// `0x1000ee89`).
const MYSTATE: u32 = 0xc;
const EXCEPT: u32 = 0x10;
const MYSTRBUF: u32 = 0x38;
/// `istream::_Chcount` (a 64-bit count after the vbptr and its pad).
const CHCOUNT: u32 = 8;
const FAILBIT: u32 = 2;
const BADBIT: u32 = 4;
/// `clear`'s mask (`_Statmask`).
const STATMASK: u32 = 0x17;

/// The imports with a Rust side here (`MSVCP110` ones keep their DLL's
/// export for the cases the Rust side hands on).
pub const TABLE: &[(&str, &str, Conv, usize, ImportFn)] = &[
    (CPP, GETLINE, Conv::Thiscall, 3, getline),
    (CRT, "_CIsin", Conv::Cdecl, 0, intrinsic_without_operand),
    (CRT, "_CIcos", Conv::Cdecl, 0, intrinsic_without_operand),
    (CRT, "_CIsqrt", Conv::Cdecl, 0, intrinsic_without_operand),
    (CRT, "_stricmp", Conv::Cdecl, 2, stricmp),
    (CRT, "strstr", Conv::Cdecl, 2, strstr),
    (CRT, "strchr", Conv::Cdecl, 2, strchr),
    (CRT, "strncpy", Conv::Cdecl, 3, strncpy),
    (CRT, "memchr", Conv::Cdecl, 3, memchr),
    (CRT, "atoi", Conv::Cdecl, 1, atol),
    (CRT, "atol", Conv::Cdecl, 1, atol),
];

fn a(args: &Args, i: usize) -> u32 {
    args.stack.get(i).copied().unwrap_or(0)
}

/// The NUL-terminated string at `p` (without the NUL).
fn c_str(mem: &impl Mem, p: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let mut at = p;
    loop {
        let b: u8 = mem.read(at);
        if b == 0 {
            return out;
        }
        out.push(b);
        at = at.wrapping_add(1);
    }
}

/// `__locale_changed`: 0 while every thread is in the "C" locale, when
/// the string functions take their ASCII paths (`_stricmp` `0x10026c5c`).
const LOCALE_CHANGED: u32 = 0x100c_87d0;

/// Whether the DLL's "C"-locale paths apply.
pub(crate) fn c_locale(e: &Exec) -> bool {
    e.shared()
        .dll(CRT)
        .is_some_and(|d| e.mem().read::<u32>(d.base + (LOCALE_CHANGED - DLL_BASE)) == 0)
}

/// `_stricmp(a, b)` in the "C" locale (`__ascii_stricmp` `0x10096b4f`):
/// bytes compared with `A`-`Z` lowered, until a difference or the end of
/// `a`; the difference of the two lowered bytes. A NULL argument (errno,
/// the invalid-parameter handler) or another locale: the DLL.
fn stricmp(e: &mut Exec, args: &Args) -> Ret {
    let (left, right) = (a(args, 0), a(args, 1));
    if left == 0 || right == 0 || !c_locale(e) {
        return dll_export(e, CRT, "_stricmp", Conv::Cdecl, args);
    }
    let mem = e.mem();
    let lower = |b: u8| u32::from(if b.is_ascii_uppercase() { b + 0x20 } else { b });
    let mut i = 0u32;
    loop {
        let l = lower(mem.read(left.wrapping_add(i)));
        let r = lower(mem.read(right.wrapping_add(i)));
        if l == 0 || l != r {
            return Ret::eax(l.wrapping_sub(r));
        }
        i += 1;
    }
}

/// `strstr(haystack, needle)`: the first occurrence or NULL; an empty
/// needle finds the haystack's start.
fn strstr(e: &mut Exec, args: &Args) -> Ret {
    let (h, n) = (a(args, 0), a(args, 1));
    let mem = e.mem();
    let needle = c_str(&mem, n);
    let hay = c_str(&mem, h);
    if needle.is_empty() {
        return Ret::eax(h);
    }
    let at = hay
        .windows(needle.len())
        .position(|w| w == needle.as_slice());
    Ret::eax(at.map_or(0, |i| h + u32::try_from(i).unwrap_or(0)))
}

/// `strchr(s, c)`: the first byte equal to `(char) c`, the terminating NUL
/// included (`c` 0 finds it); NULL when none.
fn strchr(e: &mut Exec, args: &Args) -> Ret {
    let (s, c) = (a(args, 0), a(args, 1));
    #[allow(clippy::cast_possible_truncation)] // (char) c
    let c = c as u8;
    let mem = e.mem();
    let mut at = s;
    loop {
        let b: u8 = mem.read(at);
        if b == c {
            return Ret::eax(at);
        }
        if b == 0 {
            return Ret::eax(0);
        }
        at = at.wrapping_add(1);
    }
}

/// `strncpy(dst, src, n)`: at most `n` bytes of `src`, the rest of the `n`
/// zeroed when `src` is shorter (no NUL added when it is not); `dst`.
fn strncpy(e: &mut Exec, args: &Args) -> Ret {
    let (dst, src, n) = (a(args, 0), a(args, 1), a(args, 2));
    let mut mem = e.mem();
    let mut out = Vec::with_capacity(n.min(1 << 20) as usize);
    let mut ended = false;
    for i in 0..n {
        let b: u8 = if ended {
            0
        } else {
            mem.read(src.wrapping_add(i))
        };
        ended |= b == 0;
        out.push(b);
    }
    mem.write_bytes(dst, &out);
    Ret::eax(dst)
}

/// `memchr(p, c, n)`: the first of the `n` bytes equal to `(unsigned
/// char) c`, or NULL.
fn memchr(e: &mut Exec, args: &Args) -> Ret {
    let (p, c, n) = (a(args, 0), a(args, 1), a(args, 2));
    #[allow(clippy::cast_possible_truncation)] // (unsigned char) c
    let c = c as u8;
    let mem = e.mem();
    for i in 0..n {
        if mem.read::<u8>(p.wrapping_add(i)) == c {
            return Ret::eax(p.wrapping_add(i));
        }
    }
    Ret::eax(0)
}

/// `MSVCR110`'s exports whose entries are traps to [`dll_copy`]: the
/// DLL's internal callers reach them directly (gate run 1: 72 k calls,
/// 13 M emulated blocks of `memcpy` and its vector loops, filed under the
/// exports after it: `wcsnlen`, `wcscpy_s`).
pub const DLL_COPIES: [&str; 2] = ["memcpy", "memmove"];

/// `memcpy(dst, src, n)` / `memmove(dst, src, n)` as the DLL's (one body:
/// overlap allowed, `dst` returned), for the traps on their entries.
pub fn dll_copy(e: &mut Exec, _: u32, args: &Args) -> Ret {
    const CHUNK: u32 = 0x1_0000;
    let (dst, src, n) = (a(args, 0), a(args, 1), a(args, 2));
    let mut mem = e.mem();
    let mut buf = vec![0u8; n.min(CHUNK) as usize];
    // in chunks, from the end when the destination overlaps the source's
    // tail, so each chunk is read before a write reaches it
    let backward = dst > src && dst < src.wrapping_add(n);
    let mut done = 0;
    while done < n {
        let len = (n - done).min(CHUNK);
        let off = if backward { n - done - len } else { done };
        let b = &mut buf[..len as usize];
        mem.read_bytes(src.wrapping_add(off), b);
        mem.write_bytes(dst.wrapping_add(off), b);
        done += len;
    }
    Ret::eax(dst)
}

/// `atoi(s)` / `atol(s)` (`strtol(s, NULL, 10)` in VS2012's `atox.c`) in
/// the "C" locale: spaces (`\t`..`\r`, ` `) skipped, a sign, decimal
/// digits; none: 0. A NULL argument and a value past `long`'s range (errno
/// `ERANGE`, the clamp) go to the DLL, as does another locale.
#[allow(clippy::many_single_char_names)] // the C names
fn atol(e: &mut Exec, args: &Args) -> Ret {
    let s = a(args, 0);
    // atoi is (int) atol: one export for both cases handed on
    let name = "atol";
    if s == 0 || !c_locale(e) {
        return dll_export(e, CRT, name, Conv::Cdecl, args);
    }
    let mem = e.mem();
    let mut at = s;
    let mut b: u8 = mem.read(at);
    while b == b' ' || (0x09..=0x0d).contains(&b) {
        at = at.wrapping_add(1);
        b = mem.read(at);
    }
    let negative = b == b'-';
    if b == b'-' || b == b'+' {
        at = at.wrapping_add(1);
        b = mem.read(at);
    }
    let mut n: u64 = 0;
    while b.is_ascii_digit() {
        n = n * 10 + u64::from(b - b'0');
        if n > 0x8000_0000 {
            return dll_export(e, CRT, name, Conv::Cdecl, args);
        }
        at = at.wrapping_add(1);
        b = mem.read(at);
    }
    if n == 0x8000_0000 && !negative {
        return dll_export(e, CRT, name, Conv::Cdecl, args);
    }
    #[allow(clippy::cast_possible_truncation)] // within long's range
    let v = n as u32;
    Ret::eax(if negative { v.wrapping_neg() } else { v })
}

/// Whether `name` (an import of `FFX.exe`) has a Rust side here even when
/// the player's DLLs are loaded.
#[must_use]
pub fn keeps(name: &str) -> bool {
    TABLE
        .iter()
        .chain(crate::crt_format::TABLE)
        .any(|(_, n, ..)| *n == name)
}

/// Calls the DLL's own export `name` with the import's arguments (the case
/// the Rust side doesn't cover); ends the run when there is no DLL.
fn dll_export(e: &mut Exec, dll: &str, name: &str, conv: Conv, args: &Args) -> Ret {
    let at = e
        .shared()
        .dll(dll)
        .and_then(|d| d.exports.get(name).copied());
    let Some(at) = at else {
        e.end(Stop::Failed(
            format!("unstubbed import {dll}!{name} (no DLL for the case Rust hands on)"),
            0,
        ));
        return Ret::VOID;
    };
    let a = Args {
        slots: 0,
        ..args.clone()
    };
    e.call(at, conv, &a).unwrap_or(Ret::VOID)
}

/// [`dll_export`] (cdecl) with `words` as the stack words: a varargs
/// function's declared words and the ones after them.
pub(crate) fn dll_export_words(
    e: &mut Exec,
    dll: &str,
    name: &str,
    args: &Args,
    words: Vec<u32>,
) -> Ret {
    let a = Args {
        stack: words,
        ..args.clone()
    };
    dll_export(e, dll, name, Conv::Cdecl, &a)
}

/// `istream::getline(str, count)` (thiscall, `ret 0xc`; `getline(str,
/// count, widen('\n'))`, MSVCP110 `0x10028412`/`0x1002843f`). The auto-test
/// console thread polls `std::cin` with it every 5 ms (1.4 M calls in gate
/// run 1, about 80 emulated blocks each through the sentry, the locale and
/// the stream's lock). On a stream whose state is not good the DLL's
/// sentry (`_Ipfx`) fails at once: `_Chcount` 0, `*str` 0 (unconditional
/// in this build), the state `(state | failbit | badbit when there is no
/// stream buffer) & 0x17` set twice (`_Ipfx`, then `getline` for the 0
/// characters), the lock taken and released, the locale reference counted
/// up and down; `this` back. That case runs here. A good stream (input to
/// read) and a state the stream's exception mask would throw for go to the
/// DLL's export.
fn getline(e: &mut Exec, args: &Args) -> Ret {
    let this = args.ecx;
    let str_at = args.stack.first().copied().unwrap_or(0);
    let mut mem = e.mem();
    let vbtable: u32 = mem.read(this);
    let ios = this.wrapping_add(mem.read::<u32>(vbtable.wrapping_add(4)));
    let state: u32 = mem.read(ios + MYSTATE);
    let except: u32 = mem.read(ios + EXCEPT);
    let strbuf: u32 = mem.read(ios + MYSTRBUF);
    let failed = (state | FAILBIT | if strbuf == 0 { BADBIT } else { 0 }) & STATMASK;
    if state == 0 || except & failed != 0 {
        return dll_export(e, CPP, GETLINE, Conv::Thiscall, args);
    }
    mem.write(this + CHCOUNT, 0u64);
    mem.write(str_at, 0u8);
    mem.write(ios + MYSTATE, failed);
    Ret::eax(this)
}

/// `MSVCR110.dll`'s preferred base: the addresses below are the DLL's
/// own, found at `base + (va - DLL_BASE)` in the arena.
const DLL_BASE: u32 = 0x1000_0000;
const CRT: &str = "MSVCR110.dll";
/// `__use_sse2_mathfcns`: the `_CI*` intrinsics take their SSE2 paths
/// when set (`_CIcos` `0x10032727`, `_CIsin` `0x10032e60`).
const USE_SSE2: u32 = 0x100c_9df4;

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
    cos: false,
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
    cos: true,
};

/// `_CIsin`'s tiny-argument factor (`0x1004c450`, just under 1) and
/// `_CIcos`'s 1 (`0x10045510`).
const SIN_TINY: u32 = 0x1004_c450;
const COS_ONE: u32 = 0x1004_5510;

/// The DLL's data at its own addresses.
struct Data<'a> {
    mem: &'a crate::arena::ArenaMem,
    base: u32,
}

impl Data<'_> {
    fn f(&self, va: u32) -> f64 {
        self.mem.read(self.base + (va - DLL_BASE))
    }
    fn pair(&self, va: u32) -> [f64; 2] {
        [self.f(va), self.f(va + 8)]
    }
    fn word(&self, va: u32) -> u32 {
        self.mem.read(self.base + (va - DLL_BASE))
    }
}

/// Whether `_CIsin`/`_CIcos` take their SSE2 path now: the DLL's flag,
/// `MXCSR`'s masks and rounding at their defaults (`& 0x7f80 == 0x1f80`)
/// and every x87 exception masked (`cw & 0x7f == 0x7f`).
fn sse2_path(e: &Exec, d: &Data<'_>) -> bool {
    d.word(USE_SSE2) != 0 && e.mxcsr() & 0x7f80 == 0x1f80 && e.fpu_control() & 0x7f == 0x7f
}

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
fn sse2_sin_cos(d: &Data<'_>, c: &SinCos, x: f64) -> Option<f64> {
    let top = u16::try_from(x.to_bits() >> 48).unwrap_or(0) & 0x7fff;
    let band = top.wrapping_sub(0x3030);
    if band > 0x10c5 {
        if band.cast_signed() > 0x10c5 {
            return None; // large, infinite or NaN: the x87 path
        }
        // tiny |x|
        return Some(if c.cos {
            d.f(COS_ONE) - x.abs()
        } else if band >> 4 == 0xcfd {
            // a zero or denormal exponent
            x * d.f(SIN_TINY)
        } else {
            x
        });
    }
    let step_mid = d.pair(c.step_mid);
    let (p_a, p_b) = (d.pair(c.p_a), d.pair(c.p_b));
    let (q_a, q_b) = (d.pair(c.q_a), d.pair(c.q_b));
    // the multiple of π/32 nearest to x (cvtsd2si rounds to nearest even)
    let k_real = d.f(c.inv_step) * x;
    #[allow(clippy::cast_possible_truncation)] // |k| < 2^31 in this band
    let k_int = k_real.round_ties_even() as i32;
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
    let k_mid = [step_mid[0] * k, step_mid[1] * k];
    let k_lo = k * d.f(c.step_lo);
    let r = r1 - k_mid[0];
    let rr = [r1 - k_mid[0], r1 - k_mid[1]];
    let lost = (r1 - r) - k_mid[0];
    let correction = k_lo - lost;
    // the two series in r, lane by lane
    let r2 = [rr[0] * rr[0], rr[1] * rr[1]];
    let r4 = [r2[0] * r2[0], r2[1] * r2[1]];
    let p_lo = [p_a[0] * r1 * rr[0], p_a[1] * r1 * rr[1]];
    let p = [(p_lo[0] + p_b[0]) * r4[0], (p_lo[1] + p_b[1]) * r4[1]];
    let q = [q_a[0] * r2[0] + q_b[0], q_a[1] * r2[1] + q_b[1]];
    let cos_a = cos_m1 + one;
    let weights = [cos_a * r * r2[0], sin_a * r2[1]];
    let series = [(q[0] + p[0]) * weights[0], (q[1] + p[1]) * weights[1]];
    // the large terms, sin a + 1·r + (cos a − 1)·r, with their tails
    let one_r = one * r;
    let cos_m1_r = r * cos_m1;
    let head = one_r + sin_a;
    let sum = cos_m1_r + head;
    let head_tail = (sin_a - head) + one_r;
    let sum_tail = (head - sum) + cos_m1_r;
    let fixed = correction * (sin_a * r - cos_a) + sin_lo;
    let small = fixed + head_tail + sum_tail + series[0] + series[1];
    Some(sum + small)
}

/// `_CIsin`/`_CIcos` of `x` as the DLL computes them now, or `None` when
/// the DLL would take its x87 path (no SSE2, other `MXCSR`/x87 control
/// settings, a large or non-finite `x`): then the DLL's export runs.
fn sin_cos(e: &Exec, c: &SinCos, x: f64) -> Option<f64> {
    let base = e.shared().dll(CRT)?.base;
    let mem = e.mem();
    let d = Data { mem: &mem, base };
    if !sse2_path(e, &d) {
        return None;
    }
    sse2_sin_cos(&d, c, x)
}

/// `_CIsqrt` (`0x10032f48`, x87 only: `fsqrt` under the control word):
/// for a positive or zero finite `x` at 53-bit precision and round to
/// nearest, the correctly rounded square root, as the processor's `fsqrt`
/// gives it; `None` (the DLL's export and its error handling) otherwise.
fn sqrt(e: &Exec, x: f64) -> Option<f64> {
    let cw = e.fpu_control();
    if cw & 0xf00 != 0x200 || !(x >= 0.0 && x.is_finite()) {
        return None;
    }
    Some(x.sqrt())
}

/// An x87 intrinsic's Rust side: its argument (`ST0`) to its result, or
/// `None` to run the DLL's export.
pub type X87Fn = fn(&Exec, f64) -> Option<f64>;

/// The `_CI*` intrinsics with a Rust side ([`intrinsic`]).
pub const INTRINSICS: [&str; 3] = ["_CIsin", "_CIcos", "_CIsqrt"];

/// The Rust side of the x87 intrinsic `name` (`MSVCR110.dll!_CIcos`, ...).
#[must_use]
pub fn intrinsic(name: &str) -> Option<X87Fn> {
    match name.strip_prefix("MSVCR110.dll!")? {
        "_CIsin" => Some(|e, x| sin_cos(e, &SIN, x)),
        "_CIcos" => Some(|e, x| sin_cos(e, &COS, x)),
        "_CIsqrt" => Some(sqrt),
        _ => None,
    }
}

/// The plain import side of the intrinsics (`imports::Bound::run`): the
/// executor computes them before it gets here (`Exec`'s x87 imports), so
/// this only runs for a call that has no operand on the x87 stack.
fn intrinsic_without_operand(e: &mut Exec, _: &Args) -> Ret {
    e.end(Stop::Failed(
        "an x87 intrinsic called without its operand".into(),
        0,
    ));
    Ret::VOID
}
