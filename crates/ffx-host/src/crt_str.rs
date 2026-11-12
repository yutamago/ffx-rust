//! `MSVCR110`'s string, character-class and small number functions in
//! Rust (docs/host.md, "The C runtime DLLs", m5-wp5c): the results of the
//! player's DLL (11.0.51106.1) in the "C" locale, derived from its
//! machine code. A case a Rust side doesn't cover (an invalid argument,
//! which goes to the invalid-parameter handler; a changed locale; a value
//! that sets `errno` while the DLL keeps its own) calls the DLL's export
//! when the DLLs are loaded; without them the Rust side finishes it as
//! the DLL would (`errno` in [`errno_cell`]) or stops the thread where
//! the DLL would end the process. `tests/crt_dll.rs` compares each with
//! the DLL.

#![allow(clippy::many_single_char_names)] // the C names

use ffx_bridge::Mem;

use crate::crt_dll::{c_locale, dll_export};
use crate::exec::{Exec, Stop};
use crate::imports::ImportFn;
use crate::table::{Args, Conv, Ret};

const CRT: &str = "MSVCR110.dll";

/// `errno` values the functions here set.
pub const EINVAL: u32 = 22;
pub const ERANGE: u32 = 34;
/// `strncpy_s`'s `STRUNCATE`.
const STRUNCATE: u32 = 80;
/// `_TRUNCATE`.
const TRUNCATE: u32 = u32::MAX;

/// The functions, kept as Rust sides with the DLLs loaded too.
pub const TABLE: &[(&str, &str, Conv, usize, ImportFn)] = &[
    (CRT, "strrchr", Conv::Cdecl, 2, strrchr),
    (CRT, "strncat", Conv::Cdecl, 3, strncat),
    (CRT, "strncmp", Conv::Cdecl, 3, strncmp),
    (CRT, "_strnicmp", Conv::Cdecl, 3, strnicmp),
    (CRT, "strspn", Conv::Cdecl, 2, strspn),
    (CRT, "strpbrk", Conv::Cdecl, 2, strpbrk),
    (CRT, "strcoll", Conv::Cdecl, 2, strcoll),
    (CRT, "_strlwr", Conv::Cdecl, 1, strlwr),
    (CRT, "strtok", Conv::Cdecl, 2, strtok),
    (CRT, "strcpy_s", Conv::Cdecl, 3, strcpy_s),
    (CRT, "strcat_s", Conv::Cdecl, 3, strcat_s),
    (CRT, "strncpy_s", Conv::Cdecl, 4, strncpy_s),
    (CRT, "memcpy_s", Conv::Cdecl, 4, memcpy_s),
    (CRT, "memmove_s", Conv::Cdecl, 4, memcpy_s),
    (CRT, "wmemcpy_s", Conv::Cdecl, 4, wmemcpy_s),
    (CRT, "wcscat_s", Conv::Cdecl, 3, wcscat_s),
    (CRT, "wcsstr", Conv::Cdecl, 2, wcsstr),
    (CRT, "toupper", Conv::Cdecl, 1, toupper),
    (CRT, "tolower", Conv::Cdecl, 1, tolower),
    (CRT, "isalpha", Conv::Cdecl, 1, |e, a| {
        ctype(e, a, "isalpha", 0x103)
    }),
    (CRT, "isupper", Conv::Cdecl, 1, |e, a| {
        ctype(e, a, "isupper", 0x1)
    }),
    (CRT, "islower", Conv::Cdecl, 1, |e, a| {
        ctype(e, a, "islower", 0x2)
    }),
    (CRT, "isdigit", Conv::Cdecl, 1, |e, a| {
        ctype(e, a, "isdigit", 0x4)
    }),
    (CRT, "isxdigit", Conv::Cdecl, 1, |e, a| {
        ctype(e, a, "isxdigit", 0x80)
    }),
    (CRT, "isspace", Conv::Cdecl, 1, |e, a| {
        ctype(e, a, "isspace", 0x8)
    }),
    (CRT, "ispunct", Conv::Cdecl, 1, |e, a| {
        ctype(e, a, "ispunct", 0x10)
    }),
    (CRT, "isalnum", Conv::Cdecl, 1, |e, a| {
        ctype(e, a, "isalnum", 0x107)
    }),
    (CRT, "isgraph", Conv::Cdecl, 1, |e, a| {
        ctype(e, a, "isgraph", 0x117)
    }),
    (CRT, "iscntrl", Conv::Cdecl, 1, |e, a| {
        ctype(e, a, "iscntrl", 0x20)
    }),
    (CRT, "abs", Conv::Cdecl, 1, abs),
    (CRT, "ldiv", Conv::Cdecl, 2, ldiv),
    (CRT, "strtol", Conv::Cdecl, 3, strtol),
    (CRT, "strtoul", Conv::Cdecl, 3, strtoul),
    (CRT, "realloc", Conv::Cdecl, 2, realloc),
    (CRT, "_finite", Conv::Cdecl, 2, finite),
    (CRT, "_isnan", Conv::Cdecl, 2, isnan),
    (CRT, "floor", Conv::Cdecl, 2, floor),
    (CRT, "ceil", Conv::Cdecl, 2, ceil),
    (CRT, "modf", Conv::Cdecl, 3, modf),
    (CRT, "frexp", Conv::Cdecl, 3, frexp),
    (CRT, "ldexp", Conv::Cdecl, 3, ldexp),
];

/// The ones with state the DLL keeps for itself (`errno`): Rust sides
/// only without the DLLs.
pub const NO_DLL: &[(&str, &str, Conv, usize, ImportFn)] =
    &[(CRT, "_errno", Conv::Cdecl, 0, errno)];

fn a(args: &Args, i: usize) -> u32 {
    args.stack.get(i).copied().unwrap_or(0)
}

/// The DLL's export for a case handed on, or (no DLL) `none`'s result.
fn dll_or(e: &mut Exec, name: &str, args: &Args, none: impl FnOnce(&mut Exec) -> Ret) -> Ret {
    if e.shared().dll(CRT).is_some() {
        return dll_export(e, CRT, name, Conv::Cdecl, args);
    }
    none(e)
}

/// The DLL's invalid-parameter path without the DLL: the release CRT's
/// handler ends the process (`_invoke_watson`); this thread stops.
fn invalid_parameter(e: &mut Exec, what: &str) -> Ret {
    e.end(Stop::Failed(
        format!("{CRT}: invalid parameter in {what} (the CRT ends the process here)"),
        0,
    ));
    Ret::VOID
}

/// This thread's `errno` cell (allocated on first use; `_errno()`).
pub fn errno_cell(e: &mut Exec) -> u32 {
    if e.crt.errno_at == 0 {
        let at = e.alloc(4);
        e.mem().write(at, 0u32);
        e.crt.errno_at = at;
    }
    e.crt.errno_at
}

/// Sets this thread's `errno` (without the DLLs; with them the DLL's
/// export ran and set its own).
pub fn set_errno(e: &mut Exec, v: u32) {
    let at = errno_cell(e);
    e.mem().write(at, v);
}

fn errno(e: &mut Exec, _: &Args) -> Ret {
    Ret::eax(errno_cell(e))
}

/// The bytes of the NUL-terminated string at `p` (without the NUL).
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

/// The wide string at `p` (without the NUL).
fn w_str(mem: &impl Mem, p: u32) -> Vec<u16> {
    let mut out = Vec::new();
    let mut at = p;
    loop {
        let c: u16 = mem.read(at);
        if c == 0 {
            return out;
        }
        out.push(c);
        at = at.wrapping_add(2);
    }
}

fn len32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[allow(clippy::cast_possible_truncation)] // (char) c
fn byte(c: u32) -> u8 {
    c as u8
}

/// `strrchr(s, c)`: the last byte equal to `(char) c` (`c` 0 finds the
/// NUL), or NULL.
fn strrchr(e: &mut Exec, args: &Args) -> Ret {
    let (s, c) = (a(args, 0), byte(a(args, 1)));
    let mem = e.mem();
    let text = c_str(&mem, s);
    if c == 0 {
        return Ret::eax(s.wrapping_add(len32(text.len())));
    }
    Ret::eax(
        text.iter()
            .rposition(|&b| b == c)
            .map_or(0, |i| s.wrapping_add(len32(i))),
    )
}

/// `strncat(dst, src, n)`: at most `n` bytes of `src` after `dst`'s,
/// and a NUL; `dst`.
fn strncat(e: &mut Exec, args: &Args) -> Ret {
    let (dst, src, n) = (a(args, 0), a(args, 1), a(args, 2));
    let mut mem = e.mem();
    let end = dst.wrapping_add(len32(c_str(&mem, dst).len()));
    let mut add = Vec::new();
    for i in 0..n {
        let b: u8 = mem.read(src.wrapping_add(i));
        if b == 0 {
            break;
        }
        add.push(b);
    }
    add.push(0);
    mem.write_bytes(end, &add);
    Ret::eax(dst)
}

/// `strncmp(a, b, n)`: at the first difference within `n` bytes -1 or 1
/// (the bytes compared unsigned: `sbb`/`or 1`), 0 when they agree up to a
/// NUL or the count.
fn strncmp(e: &mut Exec, args: &Args) -> Ret {
    let (l, r, n) = (a(args, 0), a(args, 1), a(args, 2));
    let mem = e.mem();
    for i in 0..n {
        let x: u8 = mem.read(l.wrapping_add(i));
        let y: u8 = mem.read(r.wrapping_add(i));
        if x != y {
            return Ret::eax(if x < y { u32::MAX } else { 1 });
        }
        if x == 0 {
            break;
        }
    }
    Ret::eax(0)
}

/// `_strnicmp(a, b, n)` in the "C" locale (`__ascii_strnicmp`
/// `0x100316de`): byte by byte within `n`, a NUL on either side ends it
/// with the raw bytes compared, else the bytes with `A`-`Z` lowered; -1,
/// 0 or 1 (unsigned). NULL arguments, a count past `INT_MAX` and another
/// locale: the DLL.
fn strnicmp(e: &mut Exec, args: &Args) -> Ret {
    let (l, r, n) = (a(args, 0), a(args, 1), a(args, 2));
    if n == 0 {
        return Ret::eax(0);
    }
    if l == 0 || r == 0 || n > 0x7fff_ffff || !c_locale(e) {
        return dll_or(e, "_strnicmp", args, |e| invalid_parameter(e, "_strnicmp"));
    }
    let mem = e.mem();
    let order = |x: u8, y: u8| Ret::eax((x.cmp(&y) as i32).cast_unsigned());
    for i in 0..n {
        let x: u8 = mem.read(l.wrapping_add(i));
        let y: u8 = mem.read(r.wrapping_add(i));
        if x == 0 || y == 0 {
            return order(x, y);
        }
        let (x, y) = (x.to_ascii_lowercase(), y.to_ascii_lowercase());
        if x != y {
            return order(x, y);
        }
    }
    Ret::eax(0)
}

/// `strspn(s, set)`: the length of `s`'s start made of `set`'s bytes.
fn strspn(e: &mut Exec, args: &Args) -> Ret {
    let mem = e.mem();
    let s = c_str(&mem, a(args, 0));
    let set = c_str(&mem, a(args, 1));
    Ret::eax(len32(s.iter().take_while(|b| set.contains(b)).count()))
}

/// `strpbrk(s, set)`: the first byte of `s` in `set`, or NULL.
fn strpbrk(e: &mut Exec, args: &Args) -> Ret {
    let p = a(args, 0);
    let mem = e.mem();
    let s = c_str(&mem, p);
    let set = c_str(&mem, a(args, 1));
    Ret::eax(
        s.iter()
            .position(|b| set.contains(b))
            .map_or(0, |i| p.wrapping_add(len32(i))),
    )
}

/// `strcoll(a, b)` in the "C" locale: `strcmp`'s -1, 0, 1. NULL
/// arguments and another locale: the DLL.
fn strcoll(e: &mut Exec, args: &Args) -> Ret {
    let (l, r) = (a(args, 0), a(args, 1));
    if l == 0 || r == 0 || !c_locale(e) {
        return dll_or(e, "strcoll", args, |e| invalid_parameter(e, "strcoll"));
    }
    let mem = e.mem();
    let ord = c_str(&mem, l).cmp(&c_str(&mem, r));
    Ret::eax((ord as i32).cast_unsigned())
}

/// `_strlwr(s)` in the "C" locale: `A`-`Z` lowered in place; `s`. NULL
/// and another locale: the DLL.
fn strlwr(e: &mut Exec, args: &Args) -> Ret {
    let s = a(args, 0);
    if s == 0 || !c_locale(e) {
        return dll_or(e, "_strlwr", args, |e| invalid_parameter(e, "_strlwr"));
    }
    let mut mem = e.mem();
    let mut text = c_str(&mem, s);
    text.make_ascii_lowercase();
    mem.write_bytes(s, &text);
    Ret::eax(s)
}

/// `strtok(s, delims)`: the next token of `s` (or, for NULL, of the
/// string the last call left in this thread's context), its end
/// overwritten with a NUL and the context after it; NULL when only
/// delimiters are left.
fn strtok(e: &mut Exec, args: &Args) -> Ret {
    let (s, d) = (a(args, 0), a(args, 1));
    let start = if s == 0 { e.crt.strtok } else { s };
    let mut mem = e.mem();
    let delims = c_str(&mem, d);
    let mut p = start;
    // skip leading delimiters
    loop {
        let b: u8 = mem.read(p);
        if b == 0 || !delims.contains(&b) {
            break;
        }
        p = p.wrapping_add(1);
    }
    let token = p;
    loop {
        let b: u8 = mem.read(p);
        if b == 0 {
            break;
        }
        if delims.contains(&b) {
            mem.write(p, 0u8);
            p = p.wrapping_add(1);
            break;
        }
        p = p.wrapping_add(1);
    }
    e.crt.strtok = p;
    Ret::eax(if token == p { 0 } else { token })
}

/// `strcpy_s(dst, size, src)`: `src` and its NUL into `dst` when they
/// fit; 0. The invalid cases (NULL, size 0, too long: `dst[0]` cleared,
/// `EINVAL`/`ERANGE` and the handler): the DLL.
fn strcpy_s(e: &mut Exec, args: &Args) -> Ret {
    let (dst, size, src) = (a(args, 0), a(args, 1), a(args, 2));
    if dst == 0 || size == 0 || src == 0 {
        return dll_or(e, "strcpy_s", args, |e| invalid_parameter(e, "strcpy_s"));
    }
    let mut mem = e.mem();
    let mut text = c_str(&mem, src);
    if len32(text.len()) >= size {
        return dll_or(e, "strcpy_s", args, |e| invalid_parameter(e, "strcpy_s"));
    }
    text.push(0);
    mem.write_bytes(dst, &text);
    Ret::eax(0)
}

/// `strcat_s(dst, size, src)`: `src` after `dst`'s string when both and
/// the NUL fit in `size`; 0. The invalid cases: the DLL.
fn strcat_s(e: &mut Exec, args: &Args) -> Ret {
    let (dst, size, src) = (a(args, 0), a(args, 1), a(args, 2));
    if dst == 0 || size == 0 || src == 0 {
        return dll_or(e, "strcat_s", args, |e| invalid_parameter(e, "strcat_s"));
    }
    let mut mem = e.mem();
    // the DLL looks for dst's NUL within size only
    let mut have = 0u32;
    while have < size && mem.read::<u8>(dst.wrapping_add(have)) != 0 {
        have += 1;
    }
    let mut text = c_str(&mem, src);
    if have == size || have + len32(text.len()) >= size {
        return dll_or(e, "strcat_s", args, |e| invalid_parameter(e, "strcat_s"));
    }
    text.push(0);
    mem.write_bytes(dst.wrapping_add(have), &text);
    Ret::eax(0)
}

/// `strncpy_s(dst, size, src, count)`: at most `count` bytes of `src`
/// and a NUL into `dst`; 0. `_TRUNCATE`: as much as fits, `STRUNCATE`
/// when cut. `count` 0 with a NULL `dst` and size 0: 0. The invalid
/// cases (NULL, too long without `_TRUNCATE`): the DLL.
fn strncpy_s(e: &mut Exec, args: &Args) -> Ret {
    let (dst, size, src, count) = (a(args, 0), a(args, 1), a(args, 2), a(args, 3));
    if count == 0 && dst == 0 && size == 0 {
        return Ret::eax(0);
    }
    if dst == 0 || size == 0 {
        return dll_or(e, "strncpy_s", args, |e| invalid_parameter(e, "strncpy_s"));
    }
    if count == 0 {
        e.mem().write(dst, 0u8);
        return Ret::eax(0);
    }
    if src == 0 {
        return dll_or(e, "strncpy_s", args, |e| invalid_parameter(e, "strncpy_s"));
    }
    let mut mem = e.mem();
    let text = c_str(&mem, src);
    let n = len32(text.len()).min(count);
    if count == TRUNCATE && n >= size {
        let keep = (size - 1) as usize;
        let mut out = text[..keep].to_vec();
        out.push(0);
        mem.write_bytes(dst, &out);
        return Ret::eax(STRUNCATE);
    }
    if n >= size {
        return dll_or(e, "strncpy_s", args, |e| invalid_parameter(e, "strncpy_s"));
    }
    let mut out = text[..n as usize].to_vec();
    out.push(0);
    mem.write_bytes(dst, &out);
    Ret::eax(0)
}

/// `memcpy_s`/`memmove_s(dst, size, src, count)`: the `count` bytes
/// (overlap allowed, as the DLL's copies) when they fit; 0. `count` 0:
/// 0. The invalid cases (NULL, too small: `dst` cleared and the
/// handler): the DLL.
fn memcpy_s(e: &mut Exec, args: &Args) -> Ret {
    let (dst, size, src, count) = (a(args, 0), a(args, 1), a(args, 2), a(args, 3));
    if count == 0 {
        return Ret::eax(0);
    }
    if dst == 0 || src == 0 || size < count {
        let name = "memcpy_s";
        return dll_or(e, name, args, |e| invalid_parameter(e, name));
    }
    let mut mem = e.mem();
    let mut buf = vec![0u8; count as usize];
    mem.read_bytes(src, &mut buf);
    mem.write_bytes(dst, &buf);
    Ret::eax(0)
}

/// `wmemcpy_s(dst, size, src, count)`: `memcpy_s` in wide characters.
fn wmemcpy_s(e: &mut Exec, args: &Args) -> Ret {
    let (dst, size, src, count) = (a(args, 0), a(args, 1), a(args, 2), a(args, 3));
    if count == 0 {
        return Ret::eax(0);
    }
    if dst == 0 || src == 0 || size < count || count > 0x7fff_ffff {
        return dll_or(e, "wmemcpy_s", args, |e| invalid_parameter(e, "wmemcpy_s"));
    }
    let mut mem = e.mem();
    let mut buf = vec![0u8; 2 * count as usize];
    mem.read_bytes(src, &mut buf);
    mem.write_bytes(dst, &buf);
    Ret::eax(0)
}

/// `wcscat_s(dst, size, src)`: `strcat_s` in wide characters.
fn wcscat_s(e: &mut Exec, args: &Args) -> Ret {
    let (dst, size, src) = (a(args, 0), a(args, 1), a(args, 2));
    if dst == 0 || size == 0 || src == 0 {
        return dll_or(e, "wcscat_s", args, |e| invalid_parameter(e, "wcscat_s"));
    }
    let mut mem = e.mem();
    let mut have = 0u32;
    while have < size && mem.read::<u16>(dst.wrapping_add(2 * have)) != 0 {
        have += 1;
    }
    let text = w_str(&mem, src);
    if have == size || have + len32(text.len()) >= size {
        return dll_or(e, "wcscat_s", args, |e| invalid_parameter(e, "wcscat_s"));
    }
    let mut out: Vec<u8> = text.iter().flat_map(|c| c.to_le_bytes()).collect();
    out.extend_from_slice(&[0, 0]);
    mem.write_bytes(dst.wrapping_add(2 * have), &out);
    Ret::eax(0)
}

/// `wcsstr(hay, needle)`: the first occurrence, or NULL; an empty needle
/// finds the start.
fn wcsstr(e: &mut Exec, args: &Args) -> Ret {
    let (h, n) = (a(args, 0), a(args, 1));
    let mem = e.mem();
    let needle = w_str(&mem, n);
    if needle.is_empty() {
        return Ret::eax(h);
    }
    let hay = w_str(&mem, h);
    let at = hay
        .windows(needle.len())
        .position(|w| w == needle.as_slice());
    Ret::eax(at.map_or(0, |i| h.wrapping_add(2 * len32(i))))
}

/// `toupper(c)` in the "C" locale: `a`-`z` raised, anything else as it
/// is (any `int`). Another locale: the DLL.
fn toupper(e: &mut Exec, args: &Args) -> Ret {
    let c = a(args, 0);
    if !c_locale(e) {
        return dll_or(e, "toupper", args, |_| Ret::eax(c));
    }
    Ret::eax(if c.wrapping_sub(0x61) <= 0x19 {
        c - 0x20
    } else {
        c
    })
}

/// `tolower(c)` in the "C" locale: `A`-`Z` lowered.
fn tolower(e: &mut Exec, args: &Args) -> Ret {
    let c = a(args, 0);
    if !c_locale(e) {
        return dll_or(e, "tolower", args, |_| Ret::eax(c));
    }
    Ret::eax(if c.wrapping_sub(0x41) <= 0x19 {
        c + 0x20
    } else {
        c
    })
}

/// `_pctype` in the "C" locale: the DLL's class table (`0x10001158`,
/// index -1 for `EOF`), in its `.text`.
const PCTYPE: u32 = 0x1000_1158;

/// `isalpha` and the rest in the "C" locale: `_pctype[c] & mask`, the
/// DLL's table read where the DLL reads it (any `c` that lands in the
/// image, as the release CRT doesn't check it). Another locale, or a `c`
/// outside the image (the DLL would fault): the DLL.
fn ctype(e: &mut Exec, args: &Args, name: &str, mask: u32) -> Ret {
    let c = a(args, 0);
    let at = PCTYPE.wrapping_add(c.wrapping_mul(2));
    let value = if c_locale(e) {
        e.shared().crt_data.as_ref().and_then(|d| d.word16(at))
    } else {
        None
    };
    match value {
        Some(v) => Ret::eax(u32::from(v) & mask),
        None => dll_or(e, name, args, |e| invalid_parameter(e, name)),
    }
}

/// `abs(n)`.
fn abs(_: &mut Exec, args: &Args) -> Ret {
    Ret::eax(a(args, 0).cast_signed().wrapping_abs().cast_unsigned())
}

/// `ldiv(n, d)`: `ldiv_t` in `eax` (quotient) and `edx` (remainder),
/// truncating; a zero divisor faults in the DLL (`idiv`): the DLL.
fn ldiv(e: &mut Exec, args: &Args) -> Ret {
    let (n, d) = (a(args, 0).cast_signed(), a(args, 1).cast_signed());
    if d == 0 || (n == i32::MIN && d == -1) {
        return dll_or(e, "ldiv", args, |e| {
            invalid_parameter(e, "ldiv (a zero divisor)")
        });
    }
    Ret {
        edx: Some((n % d).cast_unsigned()),
        ..Ret::eax((n / d).cast_unsigned())
    }
}

/// `strtol`/`strtoul`'s scan in the "C" locale (`strtoxl`): spaces, a
/// sign, `0x`/`0` prefixes for base 0 and 16, digits of the base; the
/// value (wrapped to 32 bits), whether it overflowed, and where the scan
/// stopped (the start when nothing was read).
fn scan_long(mem: &impl Mem, s: u32, base: u32) -> (u32, bool, bool, u32) {
    let mut p = s;
    let mut c: u8 = mem.read(p);
    while c == b' ' || (0x09..=0x0d).contains(&c) {
        p = p.wrapping_add(1);
        c = mem.read(p);
    }
    let negative = c == b'-';
    if c == b'-' || c == b'+' {
        p = p.wrapping_add(1);
        c = mem.read(p);
    }
    let mut base = base;
    if (base == 0 || base == 16) && c == b'0' {
        let next: u8 = mem.read(p.wrapping_add(1));
        if next == b'x' || next == b'X' {
            p = p.wrapping_add(2);
            c = mem.read(p);
            base = 16;
        }
    }
    if base == 0 {
        base = if c == b'0' { 8 } else { 10 };
    }
    let digit = |c: u8| -> Option<u32> {
        let v = match c {
            b'0'..=b'9' => u32::from(c - b'0'),
            b'a'..=b'z' => u32::from(c - b'a') + 10,
            b'A'..=b'Z' => u32::from(c - b'A') + 10,
            _ => return None,
        };
        (v < base).then_some(v)
    };
    let mut value: u32 = 0;
    let mut overflow = false;
    let mut any = false;
    while let Some(d) = digit(c) {
        any = true;
        match value.checked_mul(base).and_then(|v| v.checked_add(d)) {
            Some(v) => value = v,
            None => overflow = true,
        }
        p = p.wrapping_add(1);
        c = mem.read(p);
    }
    (value, overflow, negative, if any { p } else { s })
}

/// `strtol(s, end, base)` / `strtoul` in the "C" locale: the value,
/// `*end` where the scan stopped. An overflow (`ERANGE`, the clamp),
/// a base outside 0 and 2..36, a NULL `s` and another locale: the DLL,
/// or (no DLL) the clamp and `errno` set here.
fn strto(e: &mut Exec, args: &Args, unsigned: bool) -> Ret {
    let name = if unsigned { "strtoul" } else { "strtol" };
    let (s, end, base) = (a(args, 0), a(args, 1), a(args, 2));
    if s == 0 || base == 1 || base > 36 || !c_locale(e) {
        return dll_or(e, name, args, |e| invalid_parameter(e, name));
    }
    let (value, overflow, negative, stop) = scan_long(&e.mem(), s, base);
    let range = if unsigned {
        overflow
    } else {
        overflow || (!negative && value > 0x7fff_ffff) || (negative && value > 0x8000_0000)
    };
    if range {
        if e.shared().dll(CRT).is_some() {
            return dll_export(e, CRT, name, Conv::Cdecl, args);
        }
        set_errno(e, ERANGE);
    }
    if end != 0 {
        e.mem().write(end, stop);
    }
    let v = match (unsigned, range) {
        (true, true) => u32::MAX,
        (false, true) if negative => 0x8000_0000,
        (false, true) => 0x7fff_ffff,
        _ if negative => value.wrapping_neg(),
        _ => value,
    };
    Ret::eax(v)
}

fn strtol(e: &mut Exec, args: &Args) -> Ret {
    strto(e, args, false)
}

fn strtoul(e: &mut Exec, args: &Args) -> Ret {
    strto(e, args, true)
}

/// `realloc(p, size)` on the arena heap the Rust `malloc` and the DLL's
/// share: NULL `p` allocates, size 0 frees (NULL back), else the block
/// moves with its bytes.
fn realloc(e: &mut Exec, args: &Args) -> Ret {
    let (p, size) = (a(args, 0), a(args, 1));
    if p == 0 {
        return Ret::eax(e.alloc(size));
    }
    if size == 0 {
        e.release(p);
        return Ret::eax(0);
    }
    Ret::eax(crate::imports::resize(e, p, size, false))
}

/// The double in the stack words `i`, `i + 1`.
fn arg_f64(args: &Args, i: usize) -> f64 {
    f64::from_bits(u64::from(a(args, i)) | (u64::from(a(args, i + 1)) << 32))
}

/// `_finite(x)`: 1 for a finite `x`, else 0.
fn finite(_: &mut Exec, args: &Args) -> Ret {
    Ret::eax(u32::from(arg_f64(args, 0).is_finite()))
}

/// `_isnan(x)`: 1 for a NaN, else 0.
fn isnan(_: &mut Exec, args: &Args) -> Ret {
    Ret::eax(u32::from(arg_f64(args, 0).is_nan()))
}

/// A double result in `ST0`.
fn st0(v: f64) -> Ret {
    Ret {
        st0: Some(v),
        ..Ret::VOID
    }
}

/// `floor(x)` / `ceil(x)` (exact for every finite and infinite `x`). A
/// NaN (the DLL's `_handle_qnan1`/invalid path) or other control
/// settings: the DLL, or (no DLL) the NaN quieted.
fn round_to(e: &mut Exec, args: &Args, name: &str, f: fn(f64) -> f64) -> Ret {
    let x = arg_f64(args, 0);
    if x.is_nan() || !plain_controls(e) {
        return dll_or(e, name, args, |_| {
            st0(f64::from_bits(x.to_bits() | (1 << 51)))
        });
    }
    st0(f(x))
}

/// The default control settings the SSE2 paths need (`crt_math`'s test).
fn plain_controls(e: &Exec) -> bool {
    e.mxcsr() & 0x7f80 == 0x1f80 && e.fpu_control() & 0x7f == 0x7f
}

fn floor(e: &mut Exec, args: &Args) -> Ret {
    round_to(e, args, "floor", f64::floor)
}

fn ceil(e: &mut Exec, args: &Args) -> Ret {
    round_to(e, args, "ceil", f64::ceil)
}

/// `modf(x, ip)`: `*ip` the integer part (towards zero), the fraction
/// with `x`'s sign in `ST0`; an infinity: `*ip` it, fraction ±0. NaNs:
/// the DLL.
fn modf(e: &mut Exec, args: &Args) -> Ret {
    let x = arg_f64(args, 0);
    let ip = a(args, 2);
    if x.is_nan() || !plain_controls(e) || ip == 0 {
        return dll_or(e, "modf", args, |e| invalid_parameter(e, "modf"));
    }
    let int = x.trunc();
    e.mem().write(ip, int);
    let frac = if x.is_infinite() { 0.0 } else { x - int };
    st0(frac.copysign(x))
}

/// `frexp(x, e)`: `x = m·2^e` with 0.5 ≤ |m| < 1 (`*e` 0 and `x` back
/// for zeros, which come back as +0). Infinities, NaNs and a NULL `e`: the DLL.
fn frexp(e: &mut Exec, args: &Args) -> Ret {
    let x = arg_f64(args, 0);
    let out = a(args, 2);
    if !x.is_finite() || out == 0 {
        return dll_or(e, "frexp", args, |e| invalid_parameter(e, "frexp"));
    }
    if x == 0.0 {
        e.mem().write(out, 0u32);
        return st0(0.0);
    }
    let bits = x.to_bits();
    let (m, exp) = if x.is_subnormal() {
        // a denormal: scale it up first
        let y = x * f64::from_bits(0x4350_0000_0000_0000); // 2^54
        let b = y.to_bits();
        (b, i32::try_from((b >> 52) & 0x7ff).unwrap_or(0) - 1022 - 54)
    } else {
        (
            bits,
            i32::try_from((bits >> 52) & 0x7ff).unwrap_or(0) - 1022,
        )
    };
    e.mem().write(out, exp.cast_unsigned());
    st0(f64::from_bits((m & !(0x7ff << 52)) | (1022 << 52)))
}

/// `ldexp(x, n)`: `x·2^n` (exact for a normal result). Results that
/// overflow or underflow into the denormals (`ERANGE`: the DLL's error
/// handler rounds them), infinities and NaNs: the DLL; without it the
/// once-rounded value (which can differ from the handler's in the last
/// bit of a denormal).
fn ldexp(e: &mut Exec, args: &Args) -> Ret {
    let x = arg_f64(args, 0);
    let n = a(args, 2).cast_signed();
    let r = scale(x, n);
    if !x.is_finite() || !r.is_finite() || (r.abs() < f64::MIN_POSITIVE && x != 0.0) {
        return dll_or(e, "ldexp", args, |_| st0(r));
    }
    st0(r)
}

/// `x·2^n` with one rounding: exact in 80 bits, then stored.
fn scale(x: f64, n: i32) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let v = crate::crt_math::ext_of(x);
    let exp = i32::from(v.exp) + n.clamp(-40_000, 40_000);
    if exp >= 0x7fff {
        return f64::INFINITY.copysign(x);
    }
    // far below the doubles' denormals: zero
    let Ok(exp) = u16::try_from(exp) else {
        return 0.0f64.copysign(x);
    };
    if exp == 0 {
        return 0.0f64.copysign(x);
    }
    crate::crt_math::ext_to_f64(ffx_fpu::x87::Ext { exp, ..v })
}
