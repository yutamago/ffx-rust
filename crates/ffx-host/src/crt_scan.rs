//! `MSVCR110`'s number parsing in Rust (docs/host.md, "The C runtime
//! DLLs", m5-wp5c): `strtod`, `atof`, and the `sscanf` family (`sscanf`,
//! `sscanf_s`, `swscanf_s`, `fscanf`) in the "C" locale, for the formats
//! the game uses (`%d %i %u %x %X %o %c %s %f %e %g`, widths, `*`, the
//! sizes `h l ll L I64`, literal text and white space).
//!
//! Decimal to binary: VS2012's `__strgtold12` reads up to 21 significant
//! digits into a 96-bit mantissa, scales it by powers of ten at that
//! precision and rounds once to the double or float. For the decimals the
//! game reads (short ones, as data files and `%f` write them) that is the
//! correctly rounded value, which Rust's parser gives (no difference in
//! 16,000 such texts); inputs with more than 15 significant digits,
//! where the DLL's rounding differs (`3.450085329462328e17`), go to the
//! DLL's export when it is loaded, and without it are the correctly
//! rounded value (a known difference in the last bit for some).
//! `tests/crt_dll.rs` compares each with the DLL.

#![allow(clippy::many_single_char_names)] // the C names

use ffx_bridge::Mem;

use crate::crt_dll::{c_locale, dll_export, dll_export_words};
use crate::crt_str::{ERANGE, set_errno};
use crate::exec::Exec;
use crate::imports::ImportFn;
use crate::table::{Args, Conv, Ret};

const CRT: &str = "MSVCR110.dll";

/// Kept with the DLLs.
pub const TABLE: &[(&str, &str, Conv, usize, ImportFn)] = &[
    (CRT, "strtod", Conv::Cdecl, 2, strtod),
    (CRT, "atof", Conv::Cdecl, 1, atof),
    (CRT, "sscanf", Conv::Cdecl, 2, sscanf),
    (CRT, "sscanf_s", Conv::Cdecl, 2, sscanf_s),
    (CRT, "swscanf_s", Conv::Cdecl, 2, swscanf_s),
];

/// Without the DLLs (`fscanf` reads the run's streams, `crt_stdio`).
pub const NO_DLL: &[(&str, &str, Conv, usize, ImportFn)] =
    &[(CRT, "fscanf", Conv::Cdecl, 2, fscanf)];

fn a(args: &Args, i: usize) -> u32 {
    args.stack.get(i).copied().unwrap_or(0)
}

/// A decimal number's text at the start of `s` as `strtod` reads it: a
/// sign, digits with one `.`, an exponent (`e`, `E`, and for `strtod`
/// `d`, `D`: `fortran`) with a sign and digits. Its length and the text
/// for the parser; 0 when no digit.
fn number_text(s: &[u8], width: usize, fortran: bool) -> (usize, String) {
    let mut i = 0;
    let mut out = String::new();
    let take = |i: usize| i < width && i < s.len();
    if take(i) && (s[i] == b'+' || s[i] == b'-') {
        out.push(char::from(s[i]));
        i += 1;
    }
    let mut digits = 0;
    while take(i) && s[i].is_ascii_digit() {
        out.push(char::from(s[i]));
        i += 1;
        digits += 1;
    }
    if take(i) && s[i] == b'.' {
        out.push('.');
        i += 1;
        while take(i) && s[i].is_ascii_digit() {
            out.push(char::from(s[i]));
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return (0, String::new());
    }
    if take(i) && (matches!(s[i], b'e' | b'E') || (fortran && matches!(s[i], b'd' | b'D'))) {
        let mut j = i + 1;
        let mut exp = String::from("e");
        if take(j) && (s[j] == b'+' || s[j] == b'-') {
            exp.push(char::from(s[j]));
            j += 1;
        }
        let start = j;
        while take(j) && s[j].is_ascii_digit() {
            exp.push(char::from(s[j]));
            j += 1;
        }
        if j > start {
            out.push_str(&exp);
            i = j;
        }
    }
    (i, out)
}

/// Whether the number's text has more significant digits than the
/// correctly rounded conversion is known to agree with the DLL's on (16
/// digits beyond 2^53 already differ: `3.450085329462328e17`).
fn long_mantissa(text: &str) -> bool {
    let mant = text.split('e').next().unwrap_or("");
    let digits: String = mant.chars().filter(char::is_ascii_digit).collect();
    digits.trim_start_matches('0').trim_end_matches('0').len() > 15
}

/// The text's value as a double (an exponent past the range: ±∞ or 0).
fn parse_f64(text: &str) -> f64 {
    text.parse::<f64>().unwrap_or(0.0)
}

fn c_bytes(mem: &impl Mem, p: u32, limit: usize) -> Vec<u8> {
    let mut v = Vec::new();
    let mut at = p;
    while v.len() < limit {
        let b: u8 = mem.read(at);
        if b == 0 {
            break;
        }
        v.push(b);
        at = at.wrapping_add(1);
    }
    v
}

fn st0(v: f64) -> Ret {
    Ret {
        st0: Some(v),
        ..Ret::VOID
    }
}

/// `strtod(s, end)` in the "C" locale: the value, `*end` after the
/// number (or `s`); an overflow gives ±`HUGE_VAL` and an underflow 0
/// (`ERANGE`). NULL, another locale and long mantissas: the DLL.
fn strtod(e: &mut Exec, args: &Args) -> Ret {
    let (s, end) = (a(args, 0), a(args, 1));
    if s == 0 || !c_locale(e) {
        return hand_on(e, "strtod", args);
    }
    let text = c_bytes(&e.mem(), s, 4096);
    let skip = text
        .iter()
        .take_while(|&&b| b == b' ' || (0x09..=0x0d).contains(&b))
        .count();
    let (len, number) = number_text(&text[skip..], usize::MAX, true);
    if long_mantissa(&number) && e.shared().dll(CRT).is_some() {
        return hand_on(e, "strtod", args);
    }
    let v = parse_f64(&number);
    let out_of_range = len > 0 && (v.is_infinite() || (v == 0.0 && has_nonzero_digit(&number)));
    if out_of_range {
        if e.shared().dll(CRT).is_some() {
            return hand_on(e, "strtod", args);
        }
        set_errno(e, ERANGE);
    }
    if end != 0 {
        let stop = if len == 0 {
            s
        } else {
            s + u32::try_from(skip + len).unwrap_or(0)
        };
        e.mem().write(end, stop);
    }
    st0(v)
}

fn has_nonzero_digit(text: &str) -> bool {
    text.split('e')
        .next()
        .unwrap_or("")
        .chars()
        .any(|c| c.is_ascii_digit() && c != '0')
}

/// `atof(s)`: `strtod(s, NULL)`'s value (no `errno` the game reads).
fn atof(e: &mut Exec, args: &Args) -> Ret {
    let s = a(args, 0);
    if s == 0 || !c_locale(e) {
        return hand_on(e, "atof", args);
    }
    let text = c_bytes(&e.mem(), s, 4096);
    let skip = text
        .iter()
        .take_while(|&&b| b == b' ' || (0x09..=0x0d).contains(&b))
        .count();
    let (_, number) = number_text(&text[skip..], usize::MAX, true);
    if long_mantissa(&number) && e.shared().dll(CRT).is_some() {
        return hand_on(e, "atof", args);
    }
    st0(parse_f64(&number))
}

fn hand_on(e: &mut Exec, name: &str, args: &Args) -> Ret {
    if e.shared().dll(CRT).is_some() {
        return dll_export(e, CRT, name, Conv::Cdecl, args);
    }
    e.end(crate::exec::Stop::Failed(
        format!("{CRT}!{name}: a case without a Rust side (NULL, another locale)"),
        0,
    ));
    Ret::VOID
}

/// Where the scan's input comes from: a string (narrow or wide), or a
/// stream (`fscanf`).
enum Input {
    Bytes(Vec<u16>),
}

/// One `scanf` run: the format against the input, the results stored
/// through the argument words from `first` on (`secure`: `%s %c %[` take
/// a size word after the pointer); the count assigned, or -1 for an
/// input failure before the first conversion. `None`: a format this
/// doesn't cover (the DLL's).
#[allow(clippy::too_many_lines)]
fn scan(
    e: &mut Exec,
    input: &Input,
    fmt: &[u16],
    args: &Args,
    first: u32,
    secure: bool,
    wide: bool,
) -> Option<u32> {
    let Input::Bytes(text) = input;
    let mut next_word = args.slots.wrapping_add(4 * first);
    let mut word = |e: &Exec| {
        let v: u32 = e.mem().read(next_word);
        next_word = next_word.wrapping_add(4);
        v
    };
    let is_space = |c: u16| c == 0x20 || (0x09..=0x0d).contains(&c);
    let mut i = 0usize; // input position
    let mut f = 0usize; // format position
    let mut assigned = 0u32;
    let mut converted_any = false;
    while f < fmt.len() {
        let c = fmt[f];
        if is_space(c) {
            while f < fmt.len() && is_space(fmt[f]) {
                f += 1;
            }
            while i < text.len() && is_space(text[i]) {
                i += 1;
            }
            continue;
        }
        if c != u16::from(b'%') || fmt.get(f + 1) == Some(&u16::from(b'%')) {
            let lit = if c == u16::from(b'%') {
                f += 1;
                c
            } else {
                c
            };
            if i >= text.len() {
                return Some(if converted_any || assigned > 0 {
                    assigned
                } else {
                    u32::MAX
                });
            }
            if text[i] != lit {
                return Some(assigned);
            }
            i += 1;
            f += 1;
            continue;
        }
        f += 1;
        let suppress = fmt.get(f) == Some(&u16::from(b'*'));
        if suppress {
            f += 1;
        }
        let mut width = 0usize;
        while let Some(&d) = fmt.get(f) {
            if !(u16::from(b'0')..=u16::from(b'9')).contains(&d) {
                break;
            }
            width = width * 10 + usize::from(d - u16::from(b'0'));
            f += 1;
        }
        let mut size = 0u8; // 0 int, b'h', b'l', b'L' (64-bit / long double)
        loop {
            match fmt.get(f).map(|&x| u8::try_from(x).unwrap_or(0)) {
                Some(b'h') => {
                    size = b'h';
                    f += 1;
                }
                Some(b'l') => {
                    size = if size == b'l' { b'L' } else { b'l' };
                    f += 1;
                }
                Some(b'L') => {
                    size = b'L';
                    f += 1;
                }
                Some(b'I') => {
                    if fmt.get(f + 1..f + 3) == Some(&[u16::from(b'6'), u16::from(b'4')]) {
                        size = b'L';
                        f += 3;
                    } else {
                        return None;
                    }
                }
                _ => break,
            }
        }
        let conv = u8::try_from(*fmt.get(f)?).ok()?;
        f += 1;
        let width = if width == 0 { usize::MAX } else { width };
        if conv != b'c' {
            while i < text.len() && is_space(text[i]) {
                i += 1;
            }
        }
        if i >= text.len() && conv != b'n' {
            return Some(if converted_any || assigned > 0 {
                assigned
            } else {
                u32::MAX
            });
        }
        match conv {
            b'd' | b'i' | b'u' | b'x' | b'X' | b'o' => {
                let base_given = match conv {
                    b'x' | b'X' => 16,
                    b'o' => 8,
                    b'i' => 0,
                    _ => 10,
                };
                let start = i;
                let mut j = i;
                let take = |j: usize| j < text.len() && j - start < width;
                let mut neg = false;
                if take(j) && (text[j] == u16::from(b'-') || text[j] == u16::from(b'+')) {
                    neg = text[j] == u16::from(b'-');
                    j += 1;
                }
                let mut base = base_given;
                if (base == 16 || base == 0) && take(j) && text[j] == u16::from(b'0') {
                    let n = text.get(j + 1).copied().unwrap_or(0);
                    if take(j + 1) && (n == u16::from(b'x') || n == u16::from(b'X')) {
                        j += 2;
                        base = 16;
                    } else if base == 0 {
                        base = 8;
                    }
                }
                if base == 0 {
                    base = 10;
                }
                let mut value: u64 = 0;
                let mut digits = 0;
                while take(j) {
                    let ch = text[j];
                    let d = match u8::try_from(ch).unwrap_or(0) {
                        b @ b'0'..=b'9' => u64::from(b - b'0'),
                        b @ b'a'..=b'f' => u64::from(b - b'a') + 10,
                        b @ b'A'..=b'F' => u64::from(b - b'A') + 10,
                        _ => break,
                    };
                    if d >= base {
                        break;
                    }
                    value = value.wrapping_mul(base).wrapping_add(d);
                    digits += 1;
                    j += 1;
                }
                if digits == 0 {
                    // the input ended inside the field: an input failure
                    let ended = j >= text.len() && !converted_any;
                    return Some(if ended { u32::MAX } else { assigned });
                }
                i = j;
                converted_any = true;
                if !suppress {
                    let v = if neg { value.wrapping_neg() } else { value };
                    let p = word(e);
                    let mut mem = e.mem();
                    #[allow(clippy::cast_possible_truncation)] // the C stores
                    match size {
                        b'h' => mem.write(p, v as u16),
                        b'L' => mem.write(p, v),
                        _ => mem.write(p, v as u32),
                    }
                    assigned += 1;
                }
            }
            b'e' | b'E' | b'f' | b'g' | b'G' => {
                let rest: Vec<u8> = text[i..]
                    .iter()
                    .map(|&x| u8::try_from(x).unwrap_or(0xff))
                    .collect();
                let (len, number) = number_text(&rest, width, false);
                if len == 0 {
                    // what the DLL took before it found no digit (a sign, a
                    // point): to the end of the input is an input failure
                    let mut k = usize::from(matches!(rest.first(), Some(b'+' | b'-')));
                    if rest.get(k) == Some(&b'.') {
                        k += 1;
                    }
                    let ended = k.min(width) >= rest.len() && !converted_any;
                    return Some(if ended { u32::MAX } else { assigned });
                }
                if long_mantissa(&number) {
                    return None;
                }
                i += len;
                converted_any = true;
                if !suppress {
                    let p = word(e);
                    let mut mem = e.mem();
                    if size == b'l' || size == b'L' {
                        mem.write(p, parse_f64(&number));
                    } else {
                        mem.write(p, number.parse::<f32>().unwrap_or(0.0));
                    }
                    assigned += 1;
                }
            }
            b's' | b'c' => {
                let start = i;
                let n = if conv == b'c' && width == usize::MAX {
                    1
                } else {
                    width
                };
                let mut j = i;
                while j < text.len() && j - start < n && (conv == b'c' || !is_space(text[j])) {
                    j += 1;
                }
                if j == start {
                    return Some(assigned);
                }
                i = j;
                converted_any = true;
                if !suppress {
                    let p = word(e);
                    let cap = if secure { word(e) } else { u32::MAX };
                    let chars = &text[start..j];
                    let need =
                        u32::try_from(chars.len()).unwrap_or(u32::MAX) + u32::from(conv == b's');
                    if need > cap {
                        return None; // the buffer is too small: the DLL's path
                    }
                    let wide_out = wide ^ (size == b'h') || (!wide && size == b'l');
                    let mut mem = e.mem();
                    let mut bytes: Vec<u8> = if wide_out {
                        chars.iter().flat_map(|c| c.to_le_bytes()).collect()
                    } else {
                        chars
                            .iter()
                            .map(|&c| u8::try_from(c).unwrap_or(b'?'))
                            .collect()
                    };
                    if conv == b's' {
                        bytes.extend(if wide_out { &[0u8, 0][..] } else { &[0u8][..] });
                    }
                    mem.write_bytes(p, &bytes);
                    assigned += 1;
                }
            }
            _ => return None,
        }
    }
    Some(assigned)
}

/// A string's code units (narrow bytes or wide units), up to its NUL.
fn units(mem: &impl Mem, p: u32, wide: bool) -> Vec<u16> {
    let mut v = Vec::new();
    let mut at = p;
    loop {
        let c: u16 = if wide {
            mem.read(at)
        } else {
            u16::from(mem.read::<u8>(at))
        };
        if c == 0 || v.len() > 1 << 20 {
            return v;
        }
        v.push(c);
        at = at.wrapping_add(if wide { 2 } else { 1 });
    }
}

fn string_scan(e: &mut Exec, args: &Args, name: &str, secure: bool, wide: bool) -> Ret {
    let (s, fmt) = (a(args, 0), a(args, 1));
    if s == 0 || fmt == 0 || !c_locale(e) {
        return hand_on_words(e, name, args);
    }
    let (text, format) = {
        let mem = e.mem();
        (units(&mem, s, wide), units(&mem, fmt, wide))
    };
    match scan(e, &Input::Bytes(text), &format, args, 2, secure, wide) {
        Some(n) => Ret::eax(n),
        None => hand_on_words(e, name, args),
    }
}

/// The DLL's export with the caller's words (a varargs call: as many as
/// the format can use; 32 is more than any of the game's).
fn hand_on_words(e: &mut Exec, name: &str, args: &Args) -> Ret {
    if e.shared().dll(CRT).is_none() {
        e.end(crate::exec::Stop::Failed(
            format!("{CRT}!{name}: a format or case without a Rust side"),
            0,
        ));
        return Ret::VOID;
    }
    let words: Vec<u32> = {
        let mem = e.mem();
        (0..32)
            .map(|i| mem.read(args.slots.wrapping_add(4 * i)))
            .collect()
    };
    dll_export_words(e, CRT, name, args, words)
}

fn sscanf(e: &mut Exec, args: &Args) -> Ret {
    string_scan(e, args, "sscanf", false, false)
}

fn sscanf_s(e: &mut Exec, args: &Args) -> Ret {
    string_scan(e, args, "sscanf_s", true, false)
}

fn swscanf_s(e: &mut Exec, args: &Args) -> Ret {
    string_scan(e, args, "swscanf_s", true, true)
}

/// `fscanf(f, fmt, ...)` on the run's streams, for the game's only caller
/// (Lua's `io.read("n")`, one number): the rest of the stream's line is
/// read and scanned (the DLL reads only the number's characters; the rest
/// of that line is consumed here too).
fn fscanf(e: &mut Exec, args: &Args) -> Ret {
    let (f, fmt) = (a(args, 0), a(args, 1));
    if f == 0 || fmt == 0 {
        return Ret::eax(u32::MAX);
    }
    let format = units(&e.mem(), fmt, false);
    let text = crate::crt_stdio::read_line(e, f, 512);
    let mut probe = args.clone();
    probe.stack = vec![f, fmt];
    let input = Input::Bytes(text.iter().map(|&b| u16::from(b)).collect());
    let n = scan(e, &input, &format, &probe, 2, false, false).unwrap_or(0);
    Ret::eax(n)
}
