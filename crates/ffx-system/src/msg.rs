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
    use ffx_bridge::Mem;

    /// `TOSkipCode(p)` (`0x008dfd80`): the address after the code at `p`.
    pub fn skip_code(mem: &impl Mem, p: u32) -> u32 {
        p.wrapping_add(super::skip_code(|i| mem.read(p.wrapping_add(i))))
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
