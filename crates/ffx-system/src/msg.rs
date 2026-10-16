//! Control codes in the game's encoded text, as the `TO*` message code
//! steps over them (`TOSkipCode`, `TOGetGroup`).

/// `TOSkipCode(p)` (`0x008dfd80`): how far the code at `p` reaches, given
/// its bytes (`byte(i)` is `p[i]`; only the bytes the original reads are
/// asked for). 0 for the terminator; a run of `G` (0x47) as a whole; code
/// 4 takes one argument byte, two when that one is `0x30` or more; codes
/// 1-3, 5, 0x0c, 0x0f, 0x11 and plain characters are one byte, the other
/// codes below 0x30 two.
pub fn skip_code(mut byte: impl FnMut(u32) -> u8) -> u32 {
    match byte(0) {
        0 => 0,
        4 => {
            if byte(1) > 0x2f {
                2
            } else {
                3
            }
        }
        6..=0x0b | 0x0d | 0x0e | 0x10 | 0x12..=0x2f => 2,
        b'G' => {
            let mut n = 1;
            while byte(n) == b'G' {
                n += 1;
            }
            n
        }
        _ => 1,
    }
}

/// `TOGetGroup(code)` (`0x008df610`): the group of a control code (by its
/// low byte): 1 for 0-3, 2 for 7-0x0b, 0x0d, 0x0e and 0x10, 3 for
/// 0x12-0x23, 4 for anything else.
#[must_use]
pub const fn group(code: u32) -> u32 {
    match code & 0xff {
        0..=3 => 1,
        7..=0x0b | 0x0d | 0x0e | 0x10 => 2,
        0x12..=0x23 => 3,
        _ => 4,
    }
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// `TOSkipCode(p)` (`0x008dfd80`): the address after the code at `p`.
    pub fn skip_code(mem: &impl Mem, p: u32) -> u32 {
        p.wrapping_add(super::skip_code(|i| mem.read(p.wrapping_add(i))))
    }

    /// The glyph table (`0x00c58f00`): four bytes per glyph, its x and y in
    /// the font sheet, flags (bit 3: narrow) and its width.
    pub const GLYPHS: Global<[u8; 4]> = Global::new(0x00c5_8f00);
    /// The glyph cell height (`0x00b92314`, 8.0).
    pub const GLYPH_HEIGHT: Global<f32> = Global::new(0x00b9_2314);
    /// Added to the glyph's y for its bottom (`0x00b922a8`, 8.0 as a double).
    pub const GLYPH_BOTTOM: Global<f64> = Global::new(0x00b9_22a8);

    /// `0x0089e360(s, out)`: the glyph of the character at `s` (one byte
    /// from `0x30`, or `s[1] + 0xa0` after a lead byte below it) looked up
    /// into the sprite `out`: `+2` 0x14, `+8`/`+0xc` x and y, `+0x10`/`+0x14`
    /// right and bottom, `+0x1c` width, `+0x20` height, `+0x26` the flags
    /// and `+4` their bit 3. Returns `s`.
    pub fn glyph_sprite(mem: &mut impl Mem, s: u32, out: u32) -> u32 {
        let lead: u8 = mem.read(s);
        let index = if lead < 0x30 {
            u16::from(mem.read::<u8>(s.wrapping_add(1))) + 0xa0
        } else {
            u16::from(lead) - 0x30
        };
        let glyph: [u8; 4] = mem.read(GLYPHS.ptr(mem).addr.wrapping_add(u32::from(index) * 4));
        let (x, y, width) = (
            f32::from(glyph[0]),
            f32::from(glyph[1]),
            f32::from(glyph[3]),
        );
        mem.write(out.wrapping_add(2), 0x14u16);
        mem.write(out.wrapping_add(0x1c), width);
        mem.write(out.wrapping_add(0x20), GLYPH_HEIGHT.get(mem));
        mem.write(out.wrapping_add(8), x);
        mem.write(out.wrapping_add(0xc), y);
        mem.write(out.wrapping_add(0x10), width + x);
        #[allow(clippy::cast_possible_truncation)] // fstp to a float
        mem.write(
            out.wrapping_add(0x14),
            (f64::from(y) + GLYPH_BOTTOM.get(mem)) as f32,
        );
        mem.write(out.wrapping_add(0x26), u16::from(glyph[2]));
        mem.write(out.wrapping_add(4), u16::from(glyph[2] >> 3 & 1));
        s
    }

    /// `0x00a7b1a0(dst, src)`: `src` copied with each line feed turned into
    /// a terminator (an empty `dst` for a null `src`). Returns the length
    /// plus one (0 for null).
    pub fn copy_cutting_lines(mem: &mut impl Mem, dst: u32, src: u32) -> u32 {
        if src == 0 {
            mem.write(dst, 0u8);
            return 0;
        }
        let mut len = 0u32;
        while mem.read::<u8>(src.wrapping_add(len)) != 0 {
            len += 1;
        }
        let mut k = 0;
        loop {
            let c: u8 = mem.read(src.wrapping_add(k));
            mem.write(dst.wrapping_add(k), if c == b'\n' { 0 } else { c });
            k += 1;
            if c == 0 {
                return len + 1;
            }
        }
    }

    /// `0x0089e600(src, dst)`: a number string for the battle font: digits
    /// kept, a space as `:`, `-`, `.`, `/` as `G`, `H`, `I`; then the
    /// terminator. Any other character makes the original spin forever (it
    /// does not step past it); here it is skipped.
    pub fn number_font_string(mem: &mut impl Mem, src: u32, dst: u32) {
        let mut out = dst;
        let mut p = src;
        loop {
            let c: u8 = mem.read(p);
            if c == 0 {
                break;
            }
            let mapped = match c {
                b'0'..=b'9' => Some(c),
                b' ' => Some(b':'),
                b'-'..=b'/' => Some(b'G' + (c - b'-')),
                _ => None,
            };
            if let Some(m) = mapped {
                mem.write(out, m);
                out = out.wrapping_add(1);
            }
            p = p.wrapping_add(1);
        }
        mem.write(out, 0u8);
    }

    /// `0x008b7f40(p)`: `p` moved past control codes ([`skip_code`]) while
    /// its byte is above 2; where it stops.
    pub fn skip_codes_above_2(mem: &impl Mem, mut p: u32) -> u32 {
        while mem.read::<u8>(p) > 2 {
            p = skip_code(mem, p);
        }
        p
    }

    /// The ids `0x009066f0` shifts (`0x00b66e44`, zero-terminated).
    pub const SHIFTED_IDS: Global<u32> = Global::new(0x00b6_6e44);

    /// `0x009066f0(id)`: an id in `0x40e..=0x46e` that is 0x40e or in
    /// [`SHIFTED_IDS`] plus 100; any other unchanged.
    pub fn shift_listed_id(mem: &impl Mem, id: i32) -> i32 {
        if !(0x40e..=0x46e).contains(&id) {
            return id;
        }
        let table = SHIFTED_IDS.ptr(mem).addr;
        let mut cur = 0x40e;
        let mut k = 0u32;
        loop {
            if id == cur {
                return id + 0x64;
            }
            cur = mem.read::<i32>(table.wrapping_add(4 * k));
            k += 1;
            if cur == 0 {
                return id;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes() {
        let at = |s: &'static [u8]| move |i: u32| s[i as usize];
        assert_eq!(skip_code(at(b"\0")), 0);
        assert_eq!(skip_code(at(b"GGGx")), 3);
        assert_eq!(skip_code(at(b"\x04\x30")), 2);
        assert_eq!(skip_code(at(b"\x04\x2f")), 3);
        assert_eq!(skip_code(at(b"\x0a")), 2);
        assert_eq!(skip_code(at(b"a")), 1);
        assert_eq!(group(0x112), 3);
    }
}
