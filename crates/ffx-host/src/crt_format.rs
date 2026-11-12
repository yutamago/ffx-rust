//! Rust sides of `MSVCR110`'s `sprintf` family (m5-wp5b, 2026-11-12):
//! `sprintf`, `sprintf_s`, `_snprintf`, `vsprintf`, `_vsnprintf`,
//! `_vsnprintf_s`. Gate run 1 counted 137 k `sprintf` calls, each a few
//! hundred emulated blocks of the DLL's `_output_l`/`_output_s_l` engine
//! (the profile files them under nearby exports: `type_info::operator!=`,
//! `_wstat64i32`).
//!
//! The formatter here covers the integer, character and string
//! conversions (`%d %i %u %o %x %X %p %c %s %%`) with the flags `-+ #0`,
//! a width and a precision (digits or `*`) and the sizes `h`, `l`, `ll`,
//! `I`, `I32`, `I64`, following VS2012's `_output_l` (`output.c`): the
//! precision as the minimum of digits (it turns the `0` flag off; 0 with
//! the value 0 prints no digits; at most 512), no `0x` prefix for 0, `#`
//! forcing octal's leading 0, the `0` flag padding strings and characters
//! too, a negative `*` width meaning `-`, a negative `*` precision none,
//! `(null)` for a NULL string. Anything else (floats, wide characters
//! and strings, `%n`, positional `%1$d`, a format that ends inside a
//! conversion or names an unknown one, a locale other than "C", NULL
//! buffers, output that doesn't fit) goes to the DLL's own export with
//! the caller's stack words, as before. Without the DLLs (the no-DLL boot)
//! those cases take `imports`' approximate formatter.

use ffx_bridge::Mem;

use crate::crt_dll::{c_locale, dll_export_words};
use crate::exec::Exec;
use crate::imports::ImportFn;
use crate::table::{Args, Conv, Ret};

const CRT: &str = "MSVCR110.dll";

/// The family's imports (the declared words; the varargs are read through
/// `Args::slots`).
pub const TABLE: &[(&str, &str, Conv, usize, ImportFn)] = &[
    (CRT, "sprintf", Conv::Cdecl, 2, sprintf),
    (CRT, "sprintf_s", Conv::Cdecl, 3, sprintf_s),
    (CRT, "_snprintf", Conv::Cdecl, 3, snprintf),
    (CRT, "vsprintf", Conv::Cdecl, 3, vsprintf),
    (CRT, "_vsnprintf", Conv::Cdecl, 4, vsnprintf),
    (CRT, "_vsnprintf_s", Conv::Cdecl, 5, vsnprintf_s),
];

/// The precision's cap (`MAXPRECISION`, the conversion buffer).
const MAX_PRECISION: i64 = 512;
/// Widths and precisions above this go to the DLL (the formatter's
/// output stays small; the game's are a few characters).
const MAX_FIELD: i64 = 4096;

/// Where the variable arguments come from: words in game memory from
/// `at` on (the caller's stack words, or a `va_list`).
struct Varargs<'m, M: Mem> {
    mem: &'m M,
    at: u32,
}

impl<M: Mem> Varargs<'_, M> {
    fn word(&mut self) -> u32 {
        let w: u32 = self.mem.read(self.at);
        self.at = self.at.wrapping_add(4);
        w
    }
    fn dword(&mut self) -> u64 {
        let lo = self.word();
        u64::from(self.word()) << 32 | u64::from(lo)
    }
}

/// A conversion's flags (`output.c`'s `FL_*`).
#[derive(Clone, Copy, Default)]
#[allow(clippy::struct_excessive_bools)] // the C flags, one each
struct Flags {
    left: bool,
    plus: bool,
    space: bool,
    alt: bool,
    zero: bool,
}

/// The integer sizes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Size {
    Int,
    Short,
    Wide64,
}

/// `fmt` (NUL-terminated at `fmt`) formatted with `args`, or `None` when it
/// needs the DLL.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one conversion state machine
fn format(mem: &impl Mem, fmt: u32, args: &mut Varargs<'_, impl Mem>) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut at = fmt;
    let next = |at: &mut u32| -> u8 {
        let b: u8 = mem.read(*at);
        *at = at.wrapping_add(1);
        b
    };
    loop {
        let c = next(&mut at);
        if c == 0 {
            return Some(out);
        }
        if c != b'%' {
            out.push(c);
            continue;
        }
        let mut ch = next(&mut at);
        if ch == b'%' {
            out.push(b'%');
            continue;
        }
        let mut f = Flags::default();
        loop {
            match ch {
                b'-' => f.left = true,
                b'+' => f.plus = true,
                b' ' => f.space = true,
                b'#' => f.alt = true,
                b'0' => f.zero = true,
                _ => break,
            }
            ch = next(&mut at);
        }
        let mut width: i64 = 0;
        if ch == b'*' {
            width = i64::from(args.word().cast_signed());
            if width < 0 {
                f.left = true;
                width = -width;
            }
            ch = next(&mut at);
        } else {
            while ch.is_ascii_digit() {
                width = width * 10 + i64::from(ch - b'0');
                if width > MAX_FIELD {
                    return None;
                }
                ch = next(&mut at);
            }
            if ch == b'$' {
                return None; // positional (`_output_s_l` only)
            }
        }
        if width > MAX_FIELD {
            return None;
        }
        let mut precision: i64 = -1;
        if ch == b'.' {
            precision = 0;
            ch = next(&mut at);
            if ch == b'*' {
                precision = i64::from(args.word().cast_signed());
                if precision < 0 {
                    precision = -1;
                }
                ch = next(&mut at);
            } else {
                while ch.is_ascii_digit() {
                    precision = precision * 10 + i64::from(ch - b'0');
                    if precision > MAX_FIELD {
                        return None;
                    }
                    ch = next(&mut at);
                }
            }
            if precision > MAX_FIELD {
                return None;
            }
        }
        let mut size = Size::Int;
        let mut sized = false;
        match ch {
            b'h' => {
                size = Size::Short;
                sized = true;
                ch = next(&mut at);
            }
            b'l' => {
                ch = next(&mut at);
                if ch == b'l' {
                    size = Size::Wide64;
                    ch = next(&mut at);
                }
                sized = true;
            }
            b'I' => {
                let (a, b): (u8, u8) = (mem.read(at), mem.read(at.wrapping_add(1)));
                if (a, b) == (b'6', b'4') {
                    size = Size::Wide64;
                    at = at.wrapping_add(2);
                } else if (a, b) == (b'3', b'2') {
                    at = at.wrapping_add(2);
                } else if !b"diouxX".contains(&a) {
                    return None;
                }
                sized = true;
                ch = next(&mut at);
            }
            _ => {}
        }
        let mut prefix: &[u8] = b"";
        let text: Vec<u8> = match ch {
            b'd' | b'i' | b'u' | b'o' | b'x' | b'X' | b'p' => {
                let signed = matches!(ch, b'd' | b'i');
                let (radix, upper) = match ch {
                    b'o' => (8, false),
                    b'x' => (16, false),
                    b'X' | b'p' => (16, true),
                    _ => (10, false),
                };
                if ch == b'p' {
                    if sized {
                        return None;
                    }
                    precision = 8;
                }
                if radix == 16 && f.alt {
                    prefix = if upper { b"0X" } else { b"0x" };
                }
                let l: i64 = match size {
                    Size::Wide64 => args.dword().cast_signed(),
                    Size::Short => {
                        #[allow(clippy::cast_possible_truncation)] // (short)
                        let w = args.word() as u16;
                        if signed {
                            i64::from(w.cast_signed())
                        } else {
                            i64::from(w)
                        }
                    }
                    Size::Int => {
                        let w = args.word();
                        if signed {
                            i64::from(w.cast_signed())
                        } else {
                            i64::from(w)
                        }
                    }
                };
                let negative = signed && l < 0;
                let mut number: u64 = if negative {
                    l.unsigned_abs()
                } else {
                    l.cast_unsigned()
                };
                if size != Size::Wide64 {
                    number &= 0xffff_ffff;
                }
                if precision < 0 {
                    precision = 1;
                } else {
                    f.zero = false;
                    precision = precision.min(MAX_PRECISION);
                }
                if number == 0 {
                    prefix = b"";
                }
                let mut digits = Vec::new();
                while precision > 0 || number != 0 {
                    precision -= 1;
                    #[allow(clippy::cast_possible_truncation)] // a digit
                    let d = (number % radix) as u8;
                    number /= radix;
                    digits.push(match d {
                        0..=9 => b'0' + d,
                        _ if upper => b'A' + d - 10,
                        _ => b'a' + d - 10,
                    });
                }
                if ch == b'o' && f.alt && digits.last() != Some(&b'0') {
                    digits.push(b'0');
                }
                digits.reverse();
                if signed {
                    if negative {
                        prefix = b"-";
                    } else if f.plus {
                        prefix = b"+";
                    } else if f.space {
                        prefix = b" ";
                    }
                }
                digits
            }
            b'c' => {
                if sized && size != Size::Short {
                    return None;
                }
                #[allow(clippy::cast_possible_truncation)] // (char)
                let b = args.word() as u8;
                vec![b]
            }
            b's' => {
                if sized && size != Size::Short {
                    return None;
                }
                let p = args.word();
                let limit = if precision < 0 {
                    u32::MAX
                } else {
                    u32::try_from(precision).unwrap_or(u32::MAX)
                };
                if p == 0 {
                    b"(null)"[..6.min(limit as usize)].to_vec()
                } else {
                    let mut s = Vec::new();
                    let mut q = p;
                    while u32::try_from(s.len()).unwrap_or(u32::MAX) < limit {
                        let b: u8 = mem.read(q);
                        if b == 0 {
                            break;
                        }
                        s.push(b);
                        q = q.wrapping_add(1);
                    }
                    s
                }
            }
            _ => return None,
        };
        let used = i64::try_from(text.len() + prefix.len()).unwrap_or(i64::MAX);
        let padding = usize::try_from((width - used).max(0)).unwrap_or(0);
        if !f.left && !f.zero {
            out.extend(std::iter::repeat_n(b' ', padding));
        }
        out.extend_from_slice(prefix);
        if f.zero && !f.left {
            out.extend(std::iter::repeat_n(b'0', padding));
        }
        out.extend_from_slice(&text);
        if f.left {
            out.extend(std::iter::repeat_n(b' ', padding));
        }
    }
}

fn a(args: &Args, i: usize) -> u32 {
    args.stack.get(i).copied().unwrap_or(0)
}

/// The output of the format at `fmt` with the variable arguments from
/// `va` on, when the Rust side covers it.
pub(crate) fn formatted(e: &Exec, fmt: u32, va: u32) -> Option<Vec<u8>> {
    if fmt == 0 || !c_locale(e) {
        return None;
    }
    let mem = e.mem();
    let mut args = Varargs { mem: &mem, at: va };
    format(&mem, fmt, &mut args)
}

/// Writes `text` and its NUL at `buf`; the count as `eax`.
fn put(e: &Exec, buf: u32, text: &[u8]) -> Ret {
    let mut mem = e.mem();
    mem.write_bytes(buf, text);
    mem.write(
        buf.wrapping_add(u32::try_from(text.len()).unwrap_or(0)),
        0u8,
    );
    Ret::eax(u32::try_from(text.len()).unwrap_or(u32::MAX))
}

/// The varargs' address: the stack word after the declared ones.
fn varargs(args: &Args, declared: u32) -> u32 {
    args.slots.wrapping_add(4 * declared)
}

/// The case the Rust side hands on: the DLL's export with the caller's
/// words (the declared ones and enough after them for the varargs), or,
/// with no DLL, `imports`' approximation.
fn hand_on(e: &mut Exec, name: &str, args: &Args, approx: ImportFn) -> Ret {
    if e.shared().dll(CRT).is_none() {
        return approx(e, args);
    }
    let words: Vec<u32> = if args.slots == 0 {
        args.stack.clone()
    } else {
        let mem = e.mem();
        (0..32u32)
            .map(|i| mem.read(args.slots.wrapping_add(4 * i)))
            .collect()
    };
    dll_export_words(e, CRT, name, args, words)
}

/// `sprintf(buf, fmt, ...)`: the output and its NUL; the count.
fn sprintf(e: &mut Exec, args: &Args) -> Ret {
    let (buf, fmt) = (a(args, 0), a(args, 1));
    match formatted(e, fmt, varargs(args, 2)).filter(|_| buf != 0 && args.slots != 0) {
        Some(text) => put(e, buf, &text),
        None => hand_on(e, "sprintf", args, crate::imports::sprintf),
    }
}

/// `sprintf_s(buf, size, fmt, ...)`: as `sprintf` when the output and its
/// NUL fit in `size` (release build: nothing else written); the rest
/// (`size` 0, too small: the invalid-parameter path) goes to the DLL.
fn sprintf_s(e: &mut Exec, args: &Args) -> Ret {
    let (buf, size, fmt) = (a(args, 0), a(args, 1), a(args, 2));
    match formatted(e, fmt, varargs(args, 3))
        .filter(|t| buf != 0 && args.slots != 0 && t.len() < size as usize)
    {
        Some(text) => put(e, buf, &text),
        None => hand_on(e, "sprintf_s", args, crate::imports::sprintf_s),
    }
}

/// `_snprintf(buf, count, fmt, ...)`: as `sprintf` when the output is
/// shorter than `count` (so its NUL fits); a full or short buffer goes to
/// the DLL.
fn snprintf(e: &mut Exec, args: &Args) -> Ret {
    let (buf, count, fmt) = (a(args, 0), a(args, 1), a(args, 2));
    match formatted(e, fmt, varargs(args, 3))
        .filter(|t| buf != 0 && args.slots != 0 && t.len() < count as usize)
    {
        Some(text) => put(e, buf, &text),
        None => hand_on(e, "_snprintf", args, crate::imports::sprintf_s),
    }
}

/// `vsprintf(buf, fmt, va)`.
fn vsprintf(e: &mut Exec, args: &Args) -> Ret {
    let (buf, fmt, va) = (a(args, 0), a(args, 1), a(args, 2));
    match formatted(e, fmt, va).filter(|_| buf != 0) {
        Some(text) => put(e, buf, &text),
        None => hand_on(e, "vsprintf", args, |e, x| approx_va(e, x, 1, 2, None)),
    }
}

/// `_vsnprintf(buf, count, fmt, va)`: as `_snprintf`.
fn vsnprintf(e: &mut Exec, args: &Args) -> Ret {
    let (buf, count, fmt, va) = (a(args, 0), a(args, 1), a(args, 2), a(args, 3));
    match formatted(e, fmt, va).filter(|t| buf != 0 && t.len() < count as usize) {
        Some(text) => put(e, buf, &text),
        None => hand_on(e, "_vsnprintf", args, |e, x| approx_va(e, x, 2, 3, Some(1))),
    }
}

/// `_vsnprintf_s(buf, size, count, fmt, va)`: the output and its NUL when
/// they fit both the buffer and `count + 1` (`count` `_TRUNCATE` or not
/// below the size: also `buf[size - 1] = 0`, as the DLL's path for a
/// count that doesn't limit); a truncation or an invalid argument goes to
/// the DLL.
fn vsnprintf_s(e: &mut Exec, args: &Args) -> Ret {
    let (buf, size, count, fmt, va) = (a(args, 0), a(args, 1), a(args, 2), a(args, 3), a(args, 4));
    let fits = |t: &Vec<u8>| {
        let n = t.len() as u64;
        buf != 0 && size != 0 && n < u64::from(size) && n <= u64::from(count)
    };
    match formatted(e, fmt, va).filter(fits) {
        Some(text) => {
            let r = put(e, buf, &text);
            if size <= count {
                e.mem().write(buf.wrapping_add(size - 1), 0u8);
            }
            r
        }
        None => hand_on(e, "_vsnprintf_s", args, |e, x| {
            approx_va(e, x, 3, 4, Some(1))
        }),
    }
}

/// No DLL: `imports`' approximate formatter over a `va_list` (the buffer
/// is word 0, the format word `fmt`, the list word `va`, the size word
/// `cap`).
fn approx_va(e: &mut Exec, args: &Args, fmt: usize, va: usize, cap: Option<usize>) -> Ret {
    let mut mem = e.mem();
    let text = crate::imports::read_str(&mem, a(args, fmt), false);
    let list = Args {
        slots: a(args, va),
        ..Args::default()
    };
    let s = crate::imports::format(&mem, &text, &list, 0);
    let limit = cap.map_or(u32::MAX, |i| a(args, i));
    let n = crate::imports::write_str(&mut mem, a(args, 0), limit, &s, false);
    Ret::eax(n)
}

/// The console output of `printf`/`vprintf`/`puts` without the DLLs: the
/// host process's standard output (the game's console, when it has one,
/// shows the same).
pub(crate) fn console(text: &[u8]) {
    use std::io::Write as _;
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(text);
}

/// The approximate formatter's text for the cases the exact one hands on.
pub(crate) fn approx_text(e: &Exec, fmt: u32, va: u32) -> Vec<u8> {
    let mem = e.mem();
    let text = crate::imports::read_str(&mem, fmt, false);
    let list = Args {
        slots: va,
        ..Args::default()
    };
    crate::imports::format(&mem, &text, &list, 0).into_bytes()
}

/// `printf(fmt, ...)` / `vprintf(fmt, va)` without the DLLs: the text on
/// the console; its length. With the DLLs the DLL's own runs (these are
/// `imports` entries only).
fn print(e: &mut Exec, fmt: u32, va: u32) -> Ret {
    let text = formatted(e, fmt, va).unwrap_or_else(|| approx_text(e, fmt, va));
    console(&text);
    Ret::eax(u32::try_from(text.len()).unwrap_or(u32::MAX))
}

fn printf(e: &mut Exec, args: &Args) -> Ret {
    print(e, a(args, 0), varargs(args, 1))
}

fn vprintf(e: &mut Exec, args: &Args) -> Ret {
    print(e, a(args, 0), a(args, 1))
}

/// `puts(s)`: `s` and a newline on the console; 0.
fn puts(e: &mut Exec, args: &Args) -> Ret {
    let mut text = crate::imports::read_str(&e.mem(), a(args, 0), false).into_bytes();
    text.push(b'\n');
    console(&text);
    Ret::eax(0)
}

/// The console functions for the no-DLL build (`imports` entries: with
/// the DLLs their own run).
pub const CONSOLE_TABLE: &[(&str, &str, Conv, usize, ImportFn)] = &[
    (CRT, "printf", Conv::Cdecl, 1, printf),
    (CRT, "vprintf", Conv::Cdecl, 2, vprintf),
    (CRT, "puts", Conv::Cdecl, 1, puts),
];
