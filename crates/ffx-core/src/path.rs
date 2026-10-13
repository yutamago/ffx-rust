//! Path helpers used by the file system.

/// Lowercases ASCII `A`–`Z` in place, leaving every other byte alone.
///
/// Replaces `Path_ToLowerAscii` (`0x00607d50`). The original compares bytes
/// as signed `char`, so bytes `0x80..=0xff` never match the range.
pub fn to_lower_ascii(bytes: &mut [u8]) {
    for b in bytes {
        if b.is_ascii_uppercase() {
            *b += 0x20;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercases_only_ascii_letters() {
        let mut s = *b"/FFX_Data/GameData\\Z@[\xc4\xff";
        to_lower_ascii(&mut s);
        assert_eq!(&s, b"/ffx_data/gamedata\\z@[\xc4\xff");
    }
}
