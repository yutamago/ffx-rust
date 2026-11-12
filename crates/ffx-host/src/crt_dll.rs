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
    (CRT, "_CItan", Conv::Cdecl, 0, intrinsic_without_operand),
    (CRT, "_CIatan", Conv::Cdecl, 0, intrinsic_without_operand),
    (CRT, "_CIacos", Conv::Cdecl, 0, intrinsic_without_operand),
    (CRT, "_CIasin", Conv::Cdecl, 0, intrinsic_without_operand),
    (CRT, "_CIatan2", Conv::Cdecl, 0, intrinsic_without_operand),
    (CRT, "_CIfmod", Conv::Cdecl, 0, intrinsic_without_operand),
    (CRT, "_stricmp", Conv::Cdecl, 2, stricmp),
    (CRT, "strstr", Conv::Cdecl, 2, strstr),
    (CRT, "strchr", Conv::Cdecl, 2, strchr),
    (CRT, "strncpy", Conv::Cdecl, 3, strncpy),
    (CRT, "memchr", Conv::Cdecl, 3, memchr),
    (CRT, "atoi", Conv::Cdecl, 1, atol),
    (CRT, "atol", Conv::Cdecl, 1, atol),
    (CRT, "qsort", Conv::Cdecl, 4, qsort),
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
/// Without the DLLs (the no-Unicorn build) nothing changes the locale: the
/// "C" locale always.
pub(crate) fn c_locale(e: &Exec) -> bool {
    e.shared()
        .dll(CRT)
        .is_none_or(|d| e.mem().read::<u32>(d.base + (LOCALE_CHANGED - DLL_BASE)) == 0)
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
        .chain(crate::crt_str::TABLE)
        .chain(crate::crt_cxx::TABLE)
        .chain(crate::crt_cpp::TABLE)
        .any(|(_, n, ..)| *n == name)
}

/// Calls the DLL's own export `name` with the import's arguments (the case
/// the Rust side doesn't cover); ends the run when there is no DLL.
pub(crate) fn dll_export(e: &mut Exec, dll: &str, name: &str, conv: Conv, args: &Args) -> Ret {
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
    if state == 0 && e.shared().dll(CPP).is_none() {
        return crate::crt_cpp::getline_at_end(e, this, ios, str_at);
    }
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

/// `qsort`'s insertion threshold (`CUTOFF`): parts of at most 8 elements
/// are selection-sorted.
const QSORT_CUTOFF: u32 = 8;

/// `comp(a, b)` (cdecl, game code): its `int` result.
fn qsort_cmp(e: &mut Exec, comp: u32, x: u32, y: u32) -> Option<i32> {
    let args = Args {
        stack: vec![x, y],
        ..Args::default()
    };
    e.call(comp, Conv::Cdecl, &args)
        .ok()
        .map(|r| r.eax.cast_signed())
}

/// The `width` bytes at `x` and `y` exchanged (nothing when they are the
/// same element).
fn qsort_swap(e: &Exec, x: u32, y: u32, width: u32) {
    if x == y {
        return;
    }
    let mut mem = e.mem();
    let (mut first, mut second) = (vec![0u8; width as usize], vec![0u8; width as usize]);
    mem.read_bytes(x, &mut first);
    mem.read_bytes(y, &mut second);
    mem.write_bytes(x, &second);
    mem.write_bytes(y, &first);
}

/// `shortsort`: the largest of `[lo, hi]` swapped to the end, repeatedly.
fn qsort_short(e: &mut Exec, lo: u32, mut hi: u32, width: u32, comp: u32) -> Option<()> {
    while hi > lo {
        let mut max = lo;
        let mut p = lo + width;
        while p <= hi {
            if qsort_cmp(e, comp, p, max)? > 0 {
                max = p;
            }
            p += width;
        }
        qsort_swap(e, max, hi, width);
        hi -= width;
    }
    Some(())
}

/// `qsort(base, num, width, comp)` as VS2012's `qsort.c`: the median of
/// the first, middle and last element as the pivot, the two-way partition
/// that skips the pivot's equals, the larger part stacked and the smaller
/// sorted first, `shortsort` below [`QSORT_CUTOFF`] + 1 elements; the
/// comparisons and swaps in the CRT's order. An invalid argument (NULL
/// `base` with elements, zero `width`, NULL `comp`): the DLL (its
/// invalid-parameter path), or nothing without it.
#[allow(clippy::many_single_char_names)]
fn qsort(e: &mut Exec, args: &Args) -> Ret {
    let (base, num, width, comp) = (a(args, 0), a(args, 1), a(args, 2), a(args, 3));
    if (base == 0 && num != 0) || width == 0 || comp == 0 {
        if e.shared().dll(CRT).is_some() {
            return dll_export(e, CRT, "qsort", Conv::Cdecl, args);
        }
        return Ret::VOID;
    }
    if num < 2 {
        return Ret::VOID;
    }
    let _ = qsort_run(e, base, num, width, comp);
    Ret::VOID
}

fn qsort_run(e: &mut Exec, base: u32, num: u32, width: u32, comp: u32) -> Option<()> {
    let mut stack: Vec<(u32, u32)> = Vec::new();
    let mut lo = base;
    let mut hi = base + width * (num - 1);
    loop {
        let size = (hi - lo) / width + 1;
        if size <= QSORT_CUTOFF {
            qsort_short(e, lo, hi, width, comp)?;
        } else {
            let mut mid = lo + (size / 2) * width;
            if qsort_cmp(e, comp, lo, mid)? > 0 {
                qsort_swap(e, lo, mid, width);
            }
            if qsort_cmp(e, comp, lo, hi)? > 0 {
                qsort_swap(e, lo, hi, width);
            }
            if qsort_cmp(e, comp, mid, hi)? > 0 {
                qsort_swap(e, mid, hi, width);
            }
            let mut loguy = lo;
            let mut higuy = hi;
            loop {
                if mid > loguy {
                    loop {
                        loguy += width;
                        if !(loguy < mid && qsort_cmp(e, comp, loguy, mid)? <= 0) {
                            break;
                        }
                    }
                }
                if mid <= loguy {
                    loop {
                        loguy += width;
                        if !(loguy <= hi && qsort_cmp(e, comp, loguy, mid)? <= 0) {
                            break;
                        }
                    }
                }
                loop {
                    higuy -= width;
                    if !(higuy > mid && qsort_cmp(e, comp, higuy, mid)? > 0) {
                        break;
                    }
                }
                if higuy < loguy {
                    break;
                }
                qsort_swap(e, loguy, higuy, width);
                if mid == higuy {
                    mid = loguy;
                }
            }
            higuy += width;
            if mid < higuy {
                loop {
                    higuy -= width;
                    if !(higuy > mid && qsort_cmp(e, comp, higuy, mid)? == 0) {
                        break;
                    }
                }
            }
            if mid >= higuy {
                loop {
                    higuy -= width;
                    if !(higuy > lo && qsort_cmp(e, comp, higuy, mid)? == 0) {
                        break;
                    }
                }
            }
            if higuy.wrapping_sub(lo).cast_signed() >= hi.wrapping_sub(loguy).cast_signed() {
                if lo < higuy {
                    stack.push((lo, higuy));
                }
                if loguy < hi {
                    lo = loguy;
                    continue;
                }
            } else {
                if loguy < hi {
                    stack.push((loguy, hi));
                }
                if lo < higuy {
                    hi = higuy;
                    continue;
                }
            }
        }
        let (l, h) = stack.pop()?;
        lo = l;
        hi = h;
    }
}
