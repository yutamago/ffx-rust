//! The two LZ decoders behind `sa_decode` (`0x00a43de0`, `decode.c`).
//!
//! `sa_decode(dst, src)` calls entry `src[0]` of the table at
//! `0x00c85ea4` as `f(src, dst, &size)`: 0 is a plain copy, 1
//! [`bridge::sa_meth01dec`], 2 [`bridge::sa_meth02dec`] (Fahrenheit's
//! Switch names; that build has them at the same table slots).
//!
//! A stream starts with the method byte and the decoded size (an
//! unaligned `u32`, `sa_get_decode_size`); the codes start at `+5`. Each
//! code byte `c`:
//!
//! - `0`: the end;
//! - `1..=0x7d`: `c` literal bytes follow;
//! - `0x7e`, `n`, `b`: `n + 4` bytes `b`;
//! - `0x7f`, `n` (`u16`), `b`: `n` bytes `b` (a `dec`/`jne` loop, so 0
//!   means 2^32);
//! - `0x80..=0xff`, `x`: a back reference. Method 1: length
//!   `(c >> 3 & 0xf) + 3`, distance `((c & 7) << 8 | x) + 1`; method 2:
//!   length `(c >> 4 & 7) + 3`, distance `((c & 0xf) << 8 | x) + 1`.
//!
//! A reference that starts near the beginning of the output (method 1:
//! before `dst + 0x7ff`; method 2: up to `dst + 0xfff`) writes a zero for
//! each byte it would read from before `dst` (a signed compare of the
//! output pointer with `dst + distance`). Further on it copies without the
//! check: method 1's longest distance (0x800) from `dst + 0x7ff` reads the
//! byte just before `dst`.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Mem;

    /// Which of the two back-reference layouts a stream uses.
    #[derive(Clone, Copy)]
    enum Method {
        /// `sa_meth01dec`: 4 bits of length, 11 of distance.
        One,
        /// `sa_meth02dec`: 3 bits of length, 12 of distance.
        Two,
    }

    impl Method {
        /// The length and distance of a back reference from its code byte
        /// and the byte after it.
        fn reference(self, code: u32, low: u32) -> (u32, u32) {
            match self {
                Self::One => ((code >> 3 & 0xf) + 3, ((code & 7) << 8 | low) + 1),
                Self::Two => ((code >> 4 & 7) + 3, ((code & 0xf) << 8 | low) + 1),
            }
        }

        /// Whether a reference starting at `out` still checks for bytes
        /// before `dst`.
        fn near_start(self, out: u32, dst: u32) -> bool {
            match self {
                Self::One => out < dst.wrapping_add(0x7ff),
                Self::Two => out <= dst.wrapping_add(0xfff),
            }
        }
    }

    fn byte(mem: &impl Mem, at: u32) -> u32 {
        u32::from(mem.read::<u8>(at))
    }

    /// Writes `value` `count` times from `out` (0 counts as 2^32, as the
    /// original's loop tests after the store); returns the new end.
    fn fill(mem: &mut impl Mem, mut out: u32, value: u8, count: u32) -> u32 {
        let mut left = count;
        loop {
            mem.write(out, value);
            out = out.wrapping_add(1);
            left = left.wrapping_sub(1);
            if left == 0 {
                return out;
            }
        }
    }

    /// Decodes the codes from `src + 5` to `dst`; stores the decoded
    /// length at `size_at` and returns it.
    fn decode(mem: &mut impl Mem, method: Method, src: u32, dst: u32, size_at: u32) -> u32 {
        let mut input = src.wrapping_add(5);
        let mut out = dst;
        loop {
            let code = byte(mem, input);
            input = input.wrapping_add(1);
            match code {
                0 => {
                    let len = out.wrapping_sub(dst);
                    mem.write(size_at, len);
                    return len;
                }
                1..=0x7d => {
                    for _ in 0..code {
                        let b: u8 = mem.read(input);
                        mem.write(out, b);
                        input = input.wrapping_add(1);
                        out = out.wrapping_add(1);
                    }
                }
                0x7e => {
                    let count = byte(mem, input) + 4;
                    let value = mem.read(input.wrapping_add(1));
                    input = input.wrapping_add(2);
                    out = fill(mem, out, value, count);
                }
                0x7f => {
                    let count = u32::from(mem.read::<u16>(input));
                    let value = mem.read(input.wrapping_add(2));
                    input = input.wrapping_add(3);
                    out = fill(mem, out, value, count);
                }
                _ => {
                    let (len, distance) = method.reference(code, byte(mem, input));
                    input = input.wrapping_add(1);
                    let near = method.near_start(out, dst);
                    // an output byte below dst + distance would come from
                    // before dst
                    let zeros_end = dst.wrapping_add(distance).cast_signed();
                    let mut from = out.wrapping_sub(distance);
                    for _ in 0..len {
                        let b = if near && out.cast_signed() < zeros_end {
                            0
                        } else {
                            mem.read::<u8>(from)
                        };
                        mem.write(out, b);
                        out = out.wrapping_add(1);
                        from = from.wrapping_add(1);
                    }
                }
            }
        }
    }

    /// `sa_meth01dec(src, dst, size)` (`0x00a43bc0`): decodes a method-1
    /// stream at `src` to `dst`, stores the decoded length at `size` and
    /// returns it.
    pub fn sa_meth01dec(mem: &mut impl Mem, src: u32, dst: u32, size: u32) -> u32 {
        decode(mem, Method::One, src, dst, size)
    }

    /// `sa_meth02dec(src, dst, size)` (`0x00a43cc0`): as
    /// [`sa_meth01dec`] for a method-2 stream.
    pub fn sa_meth02dec(mem: &mut impl Mem, src: u32, dst: u32, size: u32) -> u32 {
        decode(mem, Method::Two, src, dst, size)
    }
}
