//! The game's own text decoders, as `FFX.exe` runs them: `Text_ToUtf8`
//! (`0x00645f50`) and its character decoders `DecodingGamecodeJPCHKR`
//! (`0x006460a0`) and `Text_DecodeCharWestern` (`0x006461f0`).
//!
//! [`Charset`](crate::Charset) decodes by character index; the originals
//! instead copy bytes from the loaded charset table at offsets written into
//! the code, which follow the UTF-8 widths of the shipped tables. These
//! functions reproduce those offsets: a character becomes a [`Glyph`], and
//! the caller (in [`crate::bridge`], the game's memory) copies it from the
//! table it has. The table itself is game data and never in here.
//!
//! Languages are `TOGetFFXLang` values: 0 Japanese, 1 English, 2 French,
//! 3 Spanish, 4 German, 5 Italian, 9 Korean, 10 Chinese.

/// Japanese (`TOGetFFXLang`).
pub const LANG_JAPANESE: i32 = 0;
/// Korean.
pub const LANG_KOREAN: i32 = 9;
/// Chinese.
pub const LANG_CHINESE: i32 = 10;

/// What one character writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    /// `len` bytes of the charset table from `offset` (which can be
    /// negative for codes below the first lead byte, as in the original).
    Table {
        /// Byte offset into the table.
        offset: i32,
        /// Bytes copied (1 to 3).
        len: u32,
    },
    /// One literal byte (Korean shows character 10 as an ASCII space).
    Byte(u8),
    /// Nothing (codes past the table's end).
    Nothing,
}

impl Glyph {
    /// Bytes written.
    #[must_use]
    pub const fn len(self) -> u32 {
        match self {
            Self::Table { len, .. } => len,
            Self::Byte(_) => 1,
            Self::Nothing => 0,
        }
    }

    /// Whether it writes nothing.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// The bytes it writes, from a charset table in host memory; `None`
    /// when the offset falls outside `table`.
    #[must_use]
    pub fn bytes(self, table: &[u8]) -> Option<Vec<u8>> {
        match self {
            Self::Table { offset, len } => {
                let start = usize::try_from(offset).ok()?;
                table.get(start..start + len as usize).map(<[u8]>::to_vec)
            }
            Self::Byte(b) => Some(vec![b]),
            Self::Nothing => Some(Vec::new()),
        }
    }
}

/// The character code at the cursor: `(lead − 0x2b) × 0xd0 + (trail − 0x30)`
/// when the first byte is below `0x30`, else `byte − 0x30`. `read(i)` is the
/// byte `i` places on; the trail is read only after a lead byte.
pub fn code(read: impl Fn(u32) -> u8) -> i32 {
    let b = read(0);
    if b < 0x30 {
        (i32::from(b) - 0x2b) * 0xd0 + i32::from(read(1)) - 0x30
    } else {
        i32::from(b) - 0x30
    }
}

/// `DecodingGamecodeJPCHKR`: the character at the cursor (a `04` prefix
/// selects the second Japanese bank, `+0x410`) for the jp/kr/ch tables,
/// whose characters are three bytes each except the halfwidth ones at codes
/// 17 and 40.
///
/// Quirks kept: in Japanese (`lang` 0) the bytes `5c`–`5e` show character 10;
/// in Korean character 10 is an ASCII space; Chinese has no halfwidth
/// character at 40, so the offsets past it shift by one byte, not two.
pub fn cjk(read: impl Fn(u32) -> u8, lang: i32) -> Glyph {
    let second = read(0) == 4;
    let at = u32::from(second);
    let b = read(at);
    if (0x5c..=0x5e).contains(&b) && lang == LANG_JAPANESE {
        return Glyph::Table {
            offset: 0x1e,
            len: 3,
        };
    }
    let mut c = code(|i| read(at + i));
    if second {
        c += 0x410;
    }
    let o = c.wrapping_mul(3);
    match o {
        0x1e if lang == LANG_KOREAN => Glyph::Byte(b' '),
        _ if o < 0x33 => Glyph::Table { offset: o, len: 3 },
        0x33 => Glyph::Table {
            offset: 0x33,
            len: 2,
        },
        _ if lang == LANG_CHINESE || o < 0x78 => Glyph::Table {
            offset: o - 1,
            len: 3,
        },
        0x78 => Glyph::Table {
            offset: 0x77,
            len: 2,
        },
        _ => Glyph::Table {
            offset: o - 2,
            len: 3,
        },
    }
}

/// `Text_DecodeCharWestern`: the character at the cursor for
/// `ffxsjistbl_us.bin`, codes up to `0xbf` (beyond that nothing is written).
/// Every code has its own offset and width in the original (a jump table);
/// these are the same.
pub fn western(read: impl Fn(u32) -> u8) -> Glyph {
    western_glyph(code(read))
}

/// [`western`] for a character code.
#[must_use]
pub fn western_glyph(c: i32) -> Glyph {
    let t = |offset: i32, len: u32| Glyph::Table { offset, len };
    match c {
        // below 0xc, negative codes included (the original compares signed)
        i32::MIN..=0x0b => t(c, 1),
        0x0c => t(0x0c, 3),
        0x0d..=0x10 => t(c + 2, 1),
        0x11 => t(0x13, 3),
        0x12..=0x3e => t(c + 4, 1),
        0x3f => t(0x43, 3),
        0x40..=0x5d => t(c + 6, 1),
        0x5e => t(c + 6, 2),
        0x5f => t(c + 7, 3),
        0x60 => t(c + 9, 3),
        0x61 => t(c + 0xb, 3),
        0x62 => t(c + 0xd, 3),
        0x63 => t(c + 0xf, 1),
        0x64 => t(c + 0xf, 3),
        0x65 => t(c + 0x11, 3),
        0x66 => t(c + 0x13, 3),
        0x67 => t(c + 0x15, 1),
        0x68 => t(c + 0x15, 2),
        0x69 => t(c + 0x16, 3),
        0x6a => t(c + 0x18, 3),
        0x6b => t(c + 0x1a, 3),
        0x6c => t(c + 0x1c, 3),
        0x6d..=0x6f | 0xad..=0xb7 => t(2 * c - 0x4f, 2),
        0x70 => t(0x91, 1),
        0x71..=0x9f => t(2 * c - 0x50, 2),
        0xa0 => t(c + 0x50, 1),
        0xa1 => t(c + 0x50, 2),
        0xa2 => t(c + 0x51, 3),
        0xa3 => t(c + 0x53, 3),
        0xa4 => t(c + 0x55, 1),
        0xa5 => t(c + 0x55, 3),
        0xa6 => t(c + 0x57, 3),
        0xa7 | 0xa8 => t(c + 0x59, 1),
        0xa9 => t(c + 0x59, 3),
        0xaa => t(c + 0x5b, 1),
        0xab => t(c + 0x5b, 3),
        0xac => t(c + 0x5d, 2),
        0xb8 => t(c + 0x69, 3),
        0xb9 => t(c + 0x6b, 3),
        0xba => t(c + 0x6d, 1),
        0xbb => t(c + 0x6d, 2),
        0xbc => t(c + 0x6e, 3),
        0xbd => t(c + 0x70, 3),
        0xbe => t(c + 0x72, 3),
        0xbf => t(c + 0x74, 3),
        _ => Glyph::Nothing,
    }
}

/// Whether `Text_ToUtf8` decodes characters with the jp/kr/ch decoder for
/// `lang` (0 and 9 to 11, compared signed), else the western one.
#[must_use]
pub const fn uses_cjk(lang: i32) -> bool {
    lang == 0 || (lang > 8 && lang <= 11)
}

/// What `Text_ToUtf8` does at a byte (not `00`, which ends the string).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Writes nothing; the cursor moves this far (control codes, with or
    /// without their argument byte).
    Skip(u32),
    /// Writes `\r\n` (`03`); the cursor moves one byte.
    Newline,
    /// A character through [`cjk`]; the cursor moves this far.
    Cjk(u32),
    /// A character through [`western`]; the cursor moves this far.
    Western(u32),
}

/// `Text_ToUtf8`'s step at the cursor (`read(0)` is not 0). A `04` always
/// goes to the jp/kr/ch decoder, whatever the language; an argument byte is
/// skipped even when it is `00`.
pub fn step(read: impl Fn(u32) -> u8, lang: i32) -> Step {
    let char_step = |advance| {
        if uses_cjk(lang) {
            Step::Cjk(advance)
        } else {
            Step::Western(advance)
        }
    };
    match read(0) {
        3 => Step::Newline,
        4 => Step::Cjk(if read(1) <= 0x2f { 3 } else { 2 }),
        0x06..=0x2b => Step::Skip(2),
        0x2c..=0x2f => char_step(2),
        0x30..=0xff => char_step(1),
        _ => Step::Skip(1),
    }
}

/// `Text_ToUtf8` on host bytes: the UTF-8 the game would write, given the
/// loaded tables (`cjk_table` for codes through the jp/kr/ch decoder,
/// `western_table` for the western one). Stops at the first `00` or at the
/// end of `src`; `None` when a glyph falls outside its table.
#[must_use]
pub fn to_utf8(src: &[u8], lang: i32, cjk_table: &[u8], western_table: &[u8]) -> Option<Vec<u8>> {
    let at = |p: usize| move |i: u32| src.get(p + i as usize).copied().unwrap_or(0);
    let mut out = Vec::new();
    let mut p = 0;
    while at(p)(0) != 0 {
        let advance = match step(at(p), lang) {
            Step::Skip(n) => n,
            Step::Newline => {
                out.extend_from_slice(b"\r\n");
                1
            }
            Step::Cjk(n) => {
                out.extend(cjk(at(p), lang).bytes(cjk_table)?);
                n
            }
            Step::Western(n) => {
                out.extend(western(at(p)).bytes(western_table)?);
                n
            }
        };
        p += advance as usize;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The western offsets walk the first 192 characters of the table at
    /// their shipped widths: each glyph starts where the previous one ends.
    #[test]
    fn western_offsets_follow_the_widths() {
        let mut end = 0;
        for c in 0..0xc0 {
            let Glyph::Table { offset, len } = western_glyph(c) else {
                panic!("code {c:#x}");
            };
            assert_eq!(offset, end, "code {c:#x}");
            end = offset + i32::try_from(len).unwrap();
        }
        assert_eq!(end, 0x136);
        assert_eq!(western_glyph(0xc0), Glyph::Nothing);
        assert_eq!(western_glyph(-5), Glyph::Table { offset: -5, len: 1 });
    }

    #[test]
    fn cjk_quirks() {
        let one = |b: u8| move |i: u32| if i == 0 { b } else { 0 };
        // "5c" in Japanese is character 10
        assert_eq!(
            cjk(one(0x5c), 0),
            Glyph::Table {
                offset: 0x1e,
                len: 3
            }
        );
        assert_eq!(
            cjk(one(0x5c), 9),
            Glyph::Table {
                offset: 0x82,
                len: 3
            }
        );
        // character 10 in Korean is a space
        assert_eq!(cjk(one(0x3a), 9), Glyph::Byte(b' '));
        assert_eq!(
            cjk(one(0x3a), 0),
            Glyph::Table {
                offset: 0x1e,
                len: 3
            }
        );
        // character 17 is two bytes
        assert_eq!(
            cjk(one(0x41), 0),
            Glyph::Table {
                offset: 0x33,
                len: 2
            }
        );
        assert_eq!(
            cjk(one(0x58), 10),
            Glyph::Table {
                offset: 0x77,
                len: 3
            }
        );
        assert_eq!(
            cjk(one(0x58), 0),
            Glyph::Table {
                offset: 0x77,
                len: 2
            }
        );
        // second bank
        let bank = |i: u32| [4u8, 0x30][i as usize];
        assert_eq!(
            cjk(bank, 0),
            Glyph::Table {
                offset: 0x410 * 3 - 2,
                len: 3
            }
        );
    }

    #[test]
    fn steps() {
        let s = |b: &'static [u8]| move |i: u32| b.get(i as usize).copied().unwrap_or(0);
        assert_eq!(step(s(b"\x03"), 1), Step::Newline);
        assert_eq!(step(s(b"\x0a\x00"), 1), Step::Skip(2));
        assert_eq!(step(s(b"\x01"), 1), Step::Skip(1));
        assert_eq!(step(s(b"\x2c\x30"), 1), Step::Western(2));
        assert_eq!(step(s(b"\x2c\x30"), 0), Step::Cjk(2));
        assert_eq!(step(s(b"\x04\x2c\x30"), 1), Step::Cjk(3));
        assert_eq!(step(s(b"\x04\x50"), 1), Step::Cjk(2));
        assert!(uses_cjk(11) && !uses_cjk(8) && !uses_cjk(12) && !uses_cjk(-1));
        // "A" + newline + "a" over a stand-in western table
        let mut table = vec![b'?'; 0x100];
        table[0x20 + 4] = b'A';
        table[0x40 + 6] = b'a';
        assert_eq!(
            to_utf8(b"\x50\x03\x70\x00\x50", 1, &[], &table).unwrap(),
            b"A\r\na"
        );
    }
}
