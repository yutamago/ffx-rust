//! The C runtime's `printf` family as the game's format strings use it
//! (MSVCR110's `sprintf`/`vsprintf`): integer, character and string
//! conversions with flags, width, precision and size prefixes.
//!
//! Floating-point conversions (`e`, `f`, `g`, `a`) and `%n` are not
//! supported: [`format`] returns `None` for them, so a caller can fall back
//! to the runtime's own function.

// One parser loop over the format (i, c: the cursor and byte), narrowing
// argument words to their C sizes.
#![allow(
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::cast_possible_truncation
)]

use ffx_bridge::Mem;

/// Formats `fmt` (bytes, no terminator) with argument words from `next`
/// (one per 32-bit argument, two for 64-bit ones, low word first); strings
/// for `%s` are read from `mem`. `None` for a conversion it does not
/// support.
pub fn format(mem: &impl Mem, fmt: &[u8], next: &mut impl FnMut() -> u32) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < fmt.len() {
        let c = fmt[i];
        i += 1;
        if c != b'%' {
            out.push(c);
            continue;
        }
        let (mut left, mut plus, mut space, mut alt, mut zero) =
            (false, false, false, false, false);
        while let Some(&f) = fmt.get(i) {
            match f {
                b'-' => left = true,
                b'+' => plus = true,
                b' ' => space = true,
                b'#' => alt = true,
                b'0' => zero = true,
                _ => break,
            }
            i += 1;
        }
        let mut width = 0usize;
        if fmt.get(i) == Some(&b'*') {
            i += 1;
            let w = next().cast_signed();
            if w < 0 {
                left = true;
            }
            width = w.unsigned_abs() as usize;
        } else {
            while let Some(d) = fmt.get(i).filter(|d| d.is_ascii_digit()) {
                width = width * 10 + usize::from(d - b'0');
                i += 1;
            }
        }
        let mut precision: Option<usize> = None;
        if fmt.get(i) == Some(&b'.') {
            i += 1;
            if fmt.get(i) == Some(&b'*') {
                i += 1;
                let p = next().cast_signed();
                precision = usize::try_from(p).ok();
            } else {
                let mut p = 0usize;
                while let Some(d) = fmt.get(i).filter(|d| d.is_ascii_digit()) {
                    p = p * 10 + usize::from(d - b'0');
                    i += 1;
                }
                precision = Some(p);
            }
        }
        // size prefixes
        let mut bits = 32u32;
        loop {
            match fmt.get(i) {
                Some(b'h') => {
                    // MSVCR110 has no `hh`: a second `h` is still a short
                    bits = 16;
                    i += 1;
                }
                Some(b'l' | b'L' | b'w') => {
                    bits =
                        if bits == 32 && fmt.get(i) == Some(&b'l') && fmt.get(i + 1) == Some(&b'l')
                        {
                            i += 1;
                            64
                        } else {
                            32
                        };
                    i += 1;
                }
                Some(b'I') if fmt[i..].starts_with(b"I64") => {
                    bits = 64;
                    i += 3;
                }
                Some(b'I') if fmt[i..].starts_with(b"I32") => {
                    bits = 32;
                    i += 3;
                }
                _ => break,
            }
        }
        let conv = *fmt.get(i)?;
        i += 1;
        let mut take = |bits: u32| -> u64 {
            let lo = u64::from(next());
            if bits == 64 {
                lo | (u64::from(next()) << 32)
            } else {
                lo
            }
        };
        let (prefix, body): (Vec<u8>, Vec<u8>) = match conv {
            b'%' => {
                out.push(b'%');
                continue;
            }
            b'd' | b'i' => {
                let raw = take(bits);
                let v: i64 = match bits {
                    8 => i64::from((raw as u8).cast_signed()),
                    16 => i64::from((raw as u16).cast_signed()),
                    64 => raw.cast_signed(),
                    _ => i64::from((raw as u32).cast_signed()),
                };
                let sign: &[u8] = if v < 0 {
                    b"-"
                } else if plus {
                    b"+"
                } else if space {
                    b" "
                } else {
                    b""
                };
                (
                    sign.to_vec(),
                    digits(v.unsigned_abs(), 10, false, precision),
                )
            }
            b'u' | b'o' | b'x' | b'X' => {
                let raw = take(bits);
                let v = match bits {
                    8 => raw & 0xff,
                    16 => raw & 0xffff,
                    64 => raw,
                    _ => raw & 0xffff_ffff,
                };
                let (base, upper) = match conv {
                    b'u' => (10, false),
                    b'o' => (8, false),
                    b'x' => (16, false),
                    _ => (16, true),
                };
                let mut d = digits(v, base, upper, precision);
                let mut prefix = Vec::new();
                if alt && v != 0 {
                    match conv {
                        b'x' => prefix.extend(b"0x"),
                        b'X' => prefix.extend(b"0X"),
                        b'o' if d.first() != Some(&b'0') => d.insert(0, b'0'),
                        _ => {}
                    }
                }
                (prefix, d)
            }
            b'p' => {
                let v = take(32);
                (Vec::new(), format!("{v:08X}").into_bytes())
            }
            b'c' => {
                #[allow(clippy::cast_possible_truncation)]
                let ch = take(32) as u8;
                pad(&mut out, &[], &[ch], width, left, false);
                continue;
            }
            b's' => {
                #[allow(clippy::cast_possible_truncation)]
                let p = take(32) as u32;
                let mut s = Vec::new();
                if p == 0 {
                    s.extend(b"(null)");
                    if let Some(n) = precision {
                        s.truncate(n);
                    }
                } else {
                    let mut q = p;
                    while precision.is_none_or(|n| s.len() < n) {
                        let b: u8 = mem.read(q);
                        if b == 0 {
                            break;
                        }
                        s.push(b);
                        q = q.wrapping_add(1);
                    }
                }
                pad(&mut out, &[], &s, width, left, false);
                continue;
            }
            _ => return None,
        };
        pad(
            &mut out,
            &prefix,
            &body,
            width,
            left,
            zero && precision.is_none(),
        );
    }
    Some(out)
}

/// `v` in `base`, at least `precision` digits (none for a zero value with
/// precision 0).
fn digits(v: u64, base: u64, upper: bool, precision: Option<usize>) -> Vec<u8> {
    let table: &[u8; 16] = if upper {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    let mut d = Vec::new();
    let mut x = v;
    while x != 0 {
        d.push(table[(x % base) as usize]);
        x /= base;
    }
    let min = precision.unwrap_or(1);
    while d.len() < min {
        d.push(b'0');
    }
    d.reverse();
    d
}

/// `prefix` and `body` padded to `width`: spaces before (after when `left`),
/// or zeros between the prefix and the body when `zero`.
fn pad(out: &mut Vec<u8>, prefix: &[u8], body: &[u8], width: usize, left: bool, zero: bool) {
    let len = prefix.len() + body.len();
    let fill = width.saturating_sub(len);
    if left {
        out.extend(prefix);
        out.extend(body);
        out.resize(out.len() + fill, b' ');
    } else if zero {
        out.extend(prefix);
        out.resize(out.len() + fill, b'0');
        out.extend(body);
    } else {
        out.resize(out.len() + fill, b' ');
        out.extend(prefix);
        out.extend(body);
    }
}

/// A C string from `mem` (without its terminator).
pub fn c_string(mem: &impl Mem, p: u32) -> Vec<u8> {
    let mut s = Vec::new();
    let mut q = p;
    loop {
        let b: u8 = mem.read(q);
        if b == 0 {
            return s;
        }
        s.push(b);
        q = q.wrapping_add(1);
    }
}

/// `sprintf(buf, fmt, args...)` on game memory: the formatted bytes and a
/// terminator at `buf`; their count. `None` (nothing written) for an
/// unsupported conversion.
pub fn sprintf(mem: &mut impl Mem, buf: u32, fmt: u32, args: &[u32]) -> Option<u32> {
    let f = c_string(mem, fmt);
    let mut k = 0;
    let s = format(mem, &f, &mut || {
        let v = args.get(k).copied().unwrap_or(0);
        k += 1;
        v
    })?;
    mem.write_bytes(buf, &s);
    let n = u32::try_from(s.len()).unwrap_or(u32::MAX);
    mem.write(buf.wrapping_add(n), 0u8);
    Some(n)
}
