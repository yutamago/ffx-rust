//! What the game's C runtime (MSVCR110) prints for the few `sprintf`
//! formats the character module uses: `%s`, `%d`, `%c%03d`, `%f`.
//!
//! `%f` takes the double's 17 significant digits (correctly rounded),
//! rounds those to the precision with the CRT's own rule (up from `'5'`),
//! and pads with zeros. Infinities and NaNs are the digit strings `1#INF`,
//! `1#QNAN`, `1#SNAN` and `1#IND`, rounded the same way (`1.#INF00`).

/// The 17 digits and the exponent of the first, or the CRT's special
/// strings (exponent 0).
fn digits17(x: f64) -> (Vec<u8>, i32) {
    if x.is_infinite() {
        return (b"1#INF".to_vec(), 0);
    }
    if x.is_nan() {
        let b = x.to_bits();
        let s: &[u8] = if b == 0xfff8_0000_0000_0000 {
            b"1#IND"
        } else if b & (1 << 51) != 0 {
            b"1#QNAN"
        } else {
            b"1#SNAN"
        };
        return (s.to_vec(), 0);
    }
    if x == 0.0 {
        return (vec![b'0'; 17], 0);
    }
    let s = format!("{:.16e}", x.abs());
    let (m, e) = s.split_once('e').unwrap_or((&s, "0"));
    (
        m.bytes().filter(u8::is_ascii_digit).collect(),
        e.parse().unwrap_or(0),
    )
}

/// Rounds to `n` digits (up from `'5'`, carrying through `'9'`), padding
/// with `'0'`; true when the carry made a new first digit.
fn round_digits(d: &mut Vec<u8>, n: usize) -> bool {
    if d.len() <= n {
        d.resize(n, b'0');
        return false;
    }
    let up = d[n] >= b'5';
    d.truncate(n);
    if !up {
        return false;
    }
    for i in (0..n).rev() {
        if d[i] == b'9' {
            d[i] = b'0';
        } else {
            d[i] += 1;
            return false;
        }
    }
    d.insert(0, b'1');
    d.truncate(n.max(1));
    true
}

/// `%.<prec>f`.
#[must_use]
pub fn fmt_f(x: f64, prec: usize) -> String {
    let (mut d, mut e) = digits17(x);
    let special = !x.is_finite();
    let int_digits = if special { 1 } else { e + 1 };
    let keep = int_digits + i32::try_from(prec).unwrap_or(i32::MAX);
    if keep < 0 {
        d = vec![b'0'];
        e = -1 - i32::try_from(prec).unwrap_or(0);
    } else if round_digits(&mut d, keep.cast_unsigned() as usize) {
        e += 1;
    }
    let int_digits = if special { 1 } else { e + 1 };
    let digit = |k: i64| -> u8 {
        usize::try_from(k)
            .ok()
            .and_then(|k| d.get(k).copied())
            .unwrap_or(b'0')
    };
    let mut out = Vec::new();
    if x.is_sign_negative() {
        out.push(b'-');
    }
    if int_digits <= 0 {
        out.push(b'0');
    } else {
        out.extend((0..i64::from(int_digits)).map(digit));
    }
    if prec > 0 {
        out.push(b'.');
        let first = i64::from(int_digits);
        out.extend((0..i64::try_from(prec).unwrap_or(0)).map(|k| digit(first + k)));
    }
    String::from_utf8(out).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_the_crt() {
        assert_eq!(fmt_f(70.0, 6), "70.000000");
        assert_eq!(fmt_f(-62.831_84, 6), "-62.831840");
        assert_eq!(fmt_f(0.000_123, 6), "0.000123");
        assert_eq!(fmt_f(1e20, 6), "100000000000000000000.000000");
        assert_eq!(fmt_f(f64::INFINITY, 6), "1.#INF00");
        assert_eq!(fmt_f(f64::NAN, 6), "1.#QNAN0");
        assert_eq!(fmt_f(f64::from_bits(0xfff8_0000_0000_0000), 6), "-1.#IND00");
    }

    /// The `%f` and `%.Nf` lines of what the game's own `msvcr110.dll`
    /// printed (the oracle's test data).
    #[test]
    fn as_the_recorded_crt() {
        let mut n = 0;
        for line in include_str!("../../ffx-oracle/tests/data/msvcr110-sprintf.tsv").lines() {
            let p: Vec<&str> = line.splitn(3, '\t').collect();
            let prec = match p[0] {
                "%f" => 6,
                f if f.starts_with("%.") && f.ends_with('f') => f[2..f.len() - 1].parse().unwrap(),
                _ => continue,
            };
            let x = f64::from_bits(u64::from_str_radix(p[1], 16).unwrap());
            assert_eq!(fmt_f(x, prec), p[2], "{} of {x}", p[0]);
            n += 1;
        }
        assert!(n > 100);
    }
}
