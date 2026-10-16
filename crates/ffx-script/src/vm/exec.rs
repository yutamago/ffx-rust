//! What the interpreter `Script_Run` (`0x008641e0`) computes, on plain
//! values: instruction decoding, the program counter step, and the
//! arithmetic and comparison opcodes with the x87's float rules. The loop
//! itself, over game memory, is [`super::bridge::run`].

/// The operand-less form of an instruction byte (high bit clear) or the
/// opcode of a three-byte one.
#[must_use]
pub const fn opcode(byte: u8) -> u8 {
    byte & 0x7f
}

/// Bytes an instruction takes, from its first byte: 1, or 3 with the high
/// bit (`((byte | 0x40) >> 6)`, as `Script_Run` steps the PC).
#[must_use]
pub const fn length(byte: u8) -> u32 {
    ((byte | 0x40) >> 6) as u32
}

/// `AtelFetchCommand` (`0x00869d60`) on the bytes at the PC: `opcode << 24 |
/// operand` (the `u16` after a byte with the high bit set, else 0).
#[must_use]
pub fn fetch(b0: u8, b1: u8, b2: u8) -> u32 {
    fetch_main(false, b0, b1, b2)
}

/// `AtelFetchCommandMain` (`0x00869da0`): [`fetch`], reading the operand
/// also when `long` is set.
#[must_use]
pub fn fetch_main(long: bool, b0: u8, b1: u8, b2: u8) -> u32 {
    if b0 & 0x80 == 0 && !long {
        return u32::from(b0) << 24;
    }
    u32::from(opcode(b0)) << 24 | u32::from(u16::from_le_bytes([b1, b2]))
}

/// Opcode with the largest number `Script_Run` dispatches; past it the
/// instruction is invalid.
pub const LAST: u8 = 0x7a;

/// An integer binary opcode (`a` pushed first, `b` on top): `03` or, `04`
/// xor, `05` and, `12` shift left, `13` arithmetic shift right (by `b` mod
/// 32), `14`-`18` add, sub, mul, div, mod (wrapping; `None` for the others).
///
/// # Panics
/// On `17`/`18` with `b` = 0, or `a` = `i32::MIN` and `b` = -1: the
/// original's `idiv` faults there.
#[must_use]
pub fn int_binary(op: u8, a: u32, b: u32) -> Option<u32> {
    let (sa, sb) = (a.cast_signed(), b.cast_signed());
    let fault = || panic!("script integer division {sa} by {sb}: the original's idiv faults");
    Some(match op {
        0x03 => a | b,
        0x04 => a ^ b,
        0x05 => a & b,
        0x12 => a << (b & 31),
        0x13 => (sa >> (b & 31)).cast_unsigned(),
        0x14 => a.wrapping_add(b),
        0x15 => a.wrapping_sub(b),
        0x16 => a.wrapping_mul(b),
        0x17 => sa.checked_div(sb).unwrap_or_else(fault).cast_unsigned(),
        0x18 => sa.checked_rem(sb).unwrap_or_else(fault).cast_unsigned(),
        _ => return None,
    })
}

/// An integer comparison (`06`-`0f`, docs/formats/script.md), `a` pushed
/// first.
#[must_use]
pub fn int_compare(op: u8, a: u32, b: u32) -> Option<bool> {
    let (sa, sb) = (a.cast_signed(), b.cast_signed());
    Some(match op {
        0x06 => a == b,
        0x07 => a != b,
        0x08 => a > b,
        0x0d => a <= b,
        0x0c => a >= b,
        0x09 => a < b,
        0x0a => sa > sb,
        0x0f => sa <= sb,
        0x0e => sa >= sb,
        0x0b => sa < sb,
        _ => return None,
    })
}

/// A float comparison: the same opcodes as [`int_compare`] (signed and
/// unsigned alike). The original tests `==`, `>` and `>=` and inverts for
/// the others, so `!=`, `<=` and `<` are true when either is NaN.
#[must_use]
#[allow(clippy::float_cmp, clippy::neg_cmp_op_on_partial_ord)] // exact, and NaN is the point
pub fn float_compare(op: u8, a: f32, b: f32) -> Option<bool> {
    Some(match op {
        0x06 => a == b,
        0x07 => a != b,
        0x08 | 0x0a => a > b,
        0x0d | 0x0f => !(a > b),
        0x0c | 0x0e => a >= b,
        0x09 | 0x0b => !(a >= b),
        _ => return None,
    })
}

/// The bit tests: `10` (bit `b` of `a` set), `11` (clear).
#[must_use]
pub fn bit_test(op: u8, a: u32, b: u32) -> Option<bool> {
    let set = a & (1 << (b & 31)) != 0;
    match op {
        0x10 => Some(set),
        0x11 => Some(!set),
        _ => None,
    }
}

/// The x87's default NaN (an invalid operation's result), as `f32` bits.
pub const DEFAULT_NAN: u32 = 0xffc0_0000;

/// The NaN an x87 operation on `a` and `b` gives when either is NaN
/// (quieted): the one NaN, or of two the larger significand, the positive
/// one on a tie. `None` when neither is NaN.
#[must_use]
pub fn x87_nan(a: u32, b: u32) -> Option<u32> {
    let nan = |x: u32| x & 0x7f80_0000 == 0x7f80_0000 && x & 0x007f_ffff != 0;
    let quiet = |x: u32| x | 0x0040_0000;
    match (nan(a), nan(b)) {
        (false, false) => None,
        (true, false) => Some(quiet(a)),
        (false, true) => Some(quiet(b)),
        (true, true) => {
            let (ma, mb) = (quiet(a) & 0x007f_ffff, quiet(b) & 0x007f_ffff);
            Some(quiet(if ma > mb || (ma == mb && a >> 31 == 0) {
                a
            } else {
                b
            }))
        }
    }
}

/// An x87 binary operation on two `f32` (bits), rounded to `f32` as the
/// original stores it: NaN operands propagate per [`x87_nan`]; an invalid
/// operation gives [`DEFAULT_NAN`]; anything else is the correctly rounded
/// `f32` result (the x87 at 53 bits, then `fstp`, rounds the same for
/// `+ - * /` of `f32` values).
#[must_use]
pub fn x87_binary(a: u32, b: u32, f: impl Fn(f32, f32) -> f32) -> u32 {
    if let Some(n) = x87_nan(a, b) {
        return n;
    }
    let r = f(f32::from_bits(a), f32::from_bits(b));
    if r.is_nan() { DEFAULT_NAN } else { r.to_bits() }
}

/// A float binary opcode (`14`-`17`), `a` pushed first, as `f32` bits.
#[must_use]
pub fn float_binary(op: u8, a: u32, b: u32) -> Option<u32> {
    Some(match op {
        0x14 => x87_binary(a, b, |x, y| x + y),
        0x15 => x87_binary(a, b, |x, y| x - y),
        0x16 => x87_binary(a, b, |x, y| x * y),
        0x17 => x87_binary(a, b, |x, y| x / y),
        _ => return None,
    })
}

/// Opcode `1a`: times -1.0 (a NaN keeps its sign).
#[must_use]
pub fn float_negate(a: u32) -> u32 {
    x87_binary(a, (-1.0f32).to_bits(), |x, y| x * y)
}

/// Whether an x87 comparison found `x >= 1.0` (ordered).
#[must_use]
pub fn at_least_one(x: u32) -> bool {
    f32::from_bits(x) >= 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoding() {
        assert_eq!(fetch(0x2d, 0x34, 0x12), 0x2d00_0000);
        assert_eq!(fetch(0xae, 0x34, 0x12), 0x2e00_1234);
        assert_eq!(fetch_main(true, 0x40, 0x34, 0x12), 0x4000_1234);
        assert_eq!(length(0x2d), 1);
        assert_eq!(length(0xae), 3);
    }

    #[test]
    fn operations() {
        assert_eq!(int_binary(0x13, 0x8000_0000, 33), Some(0xc000_0000));
        assert_eq!(
            int_binary(0x18, (-7i32).cast_unsigned(), 2),
            Some((-1i32).cast_unsigned())
        );
        assert_eq!(int_compare(0x09, 1, u32::MAX), Some(true));
        assert_eq!(int_compare(0x0b, 1, u32::MAX), Some(false));
        assert_eq!(float_compare(0x0d, f32::NAN, 1.0), Some(true));
        assert_eq!(float_compare(0x06, f32::NAN, f32::NAN), Some(false));
        assert_eq!(float_negate(0x7fc0_0001), 0x7fc0_0001);
        assert_eq!(float_negate(0), 0x8000_0000);
        // inf - inf: the default NaN
        assert_eq!(
            float_binary(0x15, 0x7f80_0000, 0x7f80_0000),
            Some(DEFAULT_NAN)
        );
        // two NaNs: the larger significand, then the positive one
        assert_eq!(x87_nan(0xffc0_0002, 0x7fc0_0001), Some(0xffc0_0002));
        assert_eq!(x87_nan(0xffc0_0001, 0x7fc0_0001), Some(0x7fc0_0001));
    }
}
