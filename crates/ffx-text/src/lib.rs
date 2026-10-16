//! FFX's in-game text encoding.
//!
//! Recovered from `Text_ToUtf8` (`0x00645f50`) and its two character
//! decoders (`0x006460a0` for Japanese, Korean and Chinese, `0x006461f0` for
//! the western languages). A string is a sequence of
//!
//! | Bytes | Meaning |
//! |---|---|
//! | `00` | end of string |
//! | `01`, `02`, `05` | control code without argument |
//! | `03` | line break |
//! | `04` + character | Japanese second bank: character index + `0x410` |
//! | `06`–`2b` + 1 byte | control code with one argument byte |
//! | `2c`–`2f` + `30`–`ff` | character `(lead − 0x2b) × 0xd0 + (trail − 0x30)` |
//! | `30`–`ff` | character `byte − 0x30` |
//!
//! Character index *n* is the *n*th character of the language's charset
//! table, `ffx_ps2/ffx/master/jppc/ffx_encoding/ffxsjistbl_*.bin` in
//! `FFX_Data.vbf`, which is UTF-8 text listing the characters in code order.
//! The table is game data, so it is read from the player's own copy rather
//! than shipped here.
//!
//! # Readable form
//!
//! [`Charset::decode`] turns bytes into a string that [`Charset::encode`]
//! turns back into the same bytes:
//!
//! - characters as themselves, `{{` and `}}` for literal braces;
//! - `\n` for a line break;
//! - `{01}` for a control code, `{0a:41}` for one with an argument;
//! - `{c:1a2}` for a character index that has no unique character (a
//!   duplicate in the table, or beyond its end);
//! - `{raw:2c}` for a byte that does not start a valid sequence.

use std::collections::HashMap;
use std::fmt::Write;

pub mod messages;

/// A language the game ships text for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Japanese,
    English,
    French,
    Spanish,
    German,
    Italian,
    Korean,
    Chinese,
}

impl Language {
    pub const ALL: [Self; 8] = [
        Self::Japanese,
        Self::English,
        Self::French,
        Self::Spanish,
        Self::German,
        Self::Italian,
        Self::Korean,
        Self::Chinese,
    ];

    /// Two-letter code the game uses in folder names (`new_uspc`, …).
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Japanese => "jp",
            Self::English => "us",
            Self::French => "fr",
            Self::Spanish => "sp",
            Self::German => "de",
            Self::Italian => "it",
            Self::Korean => "kr",
            Self::Chinese => "ch",
        }
    }

    /// Parses [`Language::code`] or the English name, case-insensitively.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        let code = code.to_ascii_lowercase();
        Self::ALL.into_iter().find(|l| {
            l.code() == code
                || format!("{l:?}").to_ascii_lowercase() == code
                || (code == "en" && *l == Self::English)
        })
    }

    /// Archive path of the charset table (`MallocAndReadGamecode`, `0x00646920`).
    /// Western languages share the `us` table.
    #[must_use]
    pub fn charset_path(self) -> String {
        let table = match self {
            Self::Japanese => "jp",
            Self::Korean => "kr",
            Self::Chinese => "ch",
            _ => "us",
        };
        format!("ffx_ps2/ffx/master/jppc/ffx_encoding/ffxsjistbl_{table}.bin")
    }

    /// Archive folder of the language's battle kernel tables
    /// (`getExcelBinSize`, `0x00782df0`).
    #[must_use]
    pub fn kernel_dir(self) -> String {
        format!("ffx_ps2/ffx/master/new_{}pc/battle/kernel/", self.code())
    }

    /// Archive path of event `event`'s dialogue (`cdsp0000`): the folder is
    /// named after its first two letters (`TKMallocAndReadEventMessage`, `0x0088cac0`).
    /// The `obj_psv` copies beside these are the Vita's.
    #[must_use]
    pub fn event_messages_path(self, event: &str) -> String {
        let area = event.get(..2).unwrap_or(event);
        format!(
            "ffx_ps2/ffx/master/new_{}pc/event/obj_ps3/{area}/{event}/{event}.bin",
            self.code()
        )
    }

    /// Archive path of battle `name`'s texts (`cdsp00_00`).
    #[must_use]
    pub fn battle_messages_path(self, name: &str) -> String {
        format!(
            "ffx_ps2/ffx/master/new_{}pc/battle/btl/{name}/{name}.bin",
            self.code()
        )
    }
}

/// The table is not valid UTF-8.
#[derive(Debug, thiserror::Error)]
#[error("charset table is not UTF-8")]
pub struct BadTable;

/// Text that cannot be encoded.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EncodeError {
    #[error("character {0:?} is not in this language's charset")]
    UnknownChar(char),
    #[error("malformed escape {0:?}")]
    BadEscape(String),
    #[error("character index {0:#x} cannot be encoded")]
    IndexOutOfRange(u32),
}

/// Largest index reachable without the Japanese bank prefix.
const BANK: u32 = 0x410;
const PAGE: u32 = 0xd0;

/// One language's character set.
#[derive(Debug, Clone)]
pub struct Charset {
    chars: Vec<char>,
    /// Display substitutions the game makes for particular indices.
    aliases: HashMap<u32, char>,
    /// Canonical index of each character: its first occurrence.
    index: HashMap<char, u32>,
}

impl Charset {
    /// Builds a charset from the bytes of `ffxsjistbl_*.bin`.
    ///
    /// # Errors
    /// Fails if the table is not UTF-8.
    pub fn from_table(table: &[u8], language: Language) -> Result<Self, BadTable> {
        let text = std::str::from_utf8(table).map_err(|_| BadTable)?;
        let chars: Vec<char> = text.chars().collect();
        let mut aliases = HashMap::new();
        // `0x006460a0`: in Japanese, codes 0x5c–0x5e show the character at
        // table index 10; in Korean, index 10 shows an ASCII space.
        match language {
            Language::Japanese => {
                if let Some(&c) = chars.get(10) {
                    for i in 0x2c..=0x2e {
                        aliases.insert(i, c);
                    }
                }
            }
            Language::Korean => {
                aliases.insert(10, ' ');
            }
            _ => {}
        }
        let mut index = HashMap::new();
        for (i, &c) in chars.iter().enumerate() {
            let i = u32::try_from(i).expect("table is small");
            if !aliases.contains_key(&i) {
                index.entry(c).or_insert(i);
            }
        }
        Ok(Self {
            chars,
            aliases,
            index,
        })
    }

    /// Number of characters in the table.
    #[must_use]
    pub fn len(&self) -> usize {
        self.chars.len()
    }

    /// Whether the table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    fn char_at(&self, index: u32) -> Option<char> {
        self.aliases
            .get(&index)
            .copied()
            .or_else(|| self.chars.get(index as usize).copied())
    }

    /// Decodes one string (up to its terminating `00`, or the end of
    /// `bytes`) into its readable form.
    #[must_use]
    pub fn decode(&self, bytes: &[u8]) -> String {
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        let bytes = &bytes[..end];
        let mut out = String::new();
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            match b {
                0x03 => {
                    out.push('\n');
                    i += 1;
                }
                0x01 | 0x02 | 0x05 => {
                    let _ = write!(out, "{{{b:02x}}}");
                    i += 1;
                }
                0x04 => match read_char(&bytes[i + 1..]) {
                    Some((index, len)) => {
                        self.push_char(&mut out, index + BANK, true);
                        i += 1 + len;
                    }
                    None => {
                        let _ = write!(out, "{{raw:{b:02x}}}");
                        i += 1;
                    }
                },
                0x06..=0x2b => match bytes.get(i + 1) {
                    Some(&arg) => {
                        let _ = write!(out, "{{{b:02x}:{arg:02x}}}");
                        i += 2;
                    }
                    None => {
                        let _ = write!(out, "{{raw:{b:02x}}}");
                        i += 1;
                    }
                },
                _ => match read_char(&bytes[i..]) {
                    Some((index, len)) => {
                        self.push_char(&mut out, index, false);
                        i += len;
                    }
                    None => {
                        let _ = write!(out, "{{raw:{b:02x}}}");
                        i += 1;
                    }
                },
            }
        }
        out
    }

    fn push_char(&self, out: &mut String, index: u32, banked: bool) {
        match self.char_at(index) {
            // `\n` stands for code 03, so a table newline needs the escape.
            Some(c)
                if c != '\n' && self.index.get(&c) == Some(&index) && (index >= BANK) == banked =>
            {
                match c {
                    '{' => out.push_str("{{"),
                    '}' => out.push_str("}}"),
                    c => out.push(c),
                }
            }
            _ => {
                let _ = write!(out, "{{c:{index:x}}}");
            }
        }
    }

    /// Encodes a readable string (see the crate docs) into game bytes,
    /// without the terminating `00`.
    ///
    /// # Errors
    /// Fails on characters missing from the charset and malformed escapes.
    pub fn encode(&self, text: &str) -> Result<Vec<u8>, EncodeError> {
        let mut out = Vec::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\n' => out.push(0x03),
                '}' if chars.peek() == Some(&'}') => {
                    chars.next();
                    self.push_index(&mut out, self.lookup('}')?)?;
                }
                '{' if chars.peek() == Some(&'{') => {
                    chars.next();
                    self.push_index(&mut out, self.lookup('{')?)?;
                }
                '{' => {
                    let escape: String = chars.by_ref().take_while(|&c| c != '}').collect();
                    self.push_escape(&mut out, &escape)?;
                }
                c => self.push_index(&mut out, self.lookup(c)?)?,
            }
        }
        Ok(out)
    }

    fn lookup(&self, c: char) -> Result<u32, EncodeError> {
        self.index
            .get(&c)
            .copied()
            .ok_or(EncodeError::UnknownChar(c))
    }

    fn push_escape(&self, out: &mut Vec<u8>, escape: &str) -> Result<(), EncodeError> {
        let bad = || EncodeError::BadEscape(format!("{{{escape}}}"));
        let hex_byte = |s: &str| u8::from_str_radix(s, 16).ok().filter(|_| s.len() == 2);
        if let Some(raw) = escape.strip_prefix("raw:") {
            out.push(hex_byte(raw).ok_or_else(bad)?);
        } else if let Some(index) = escape.strip_prefix("c:") {
            let index = u32::from_str_radix(index, 16).map_err(|_| bad())?;
            self.push_index(out, index)?;
        } else if let Some((code, arg)) = escape.split_once(':') {
            let code = hex_byte(code)
                .filter(|c| (0x06..=0x2b).contains(c))
                .ok_or_else(bad)?;
            out.extend([code, hex_byte(arg).ok_or_else(bad)?]);
        } else {
            let code = hex_byte(escape)
                .filter(|c| matches!(c, 0x01 | 0x02 | 0x05))
                .ok_or_else(bad)?;
            out.push(code);
        }
        Ok(())
    }

    #[allow(clippy::unused_self, clippy::cast_possible_truncation)] // ranges checked below
    fn push_index(&self, out: &mut Vec<u8>, index: u32) -> Result<(), EncodeError> {
        let (prefix, index) = if index >= BANK {
            (Some(0x04), index - BANK)
        } else {
            (None, index)
        };
        if index >= BANK {
            return Err(EncodeError::IndexOutOfRange(index + BANK));
        }
        out.extend(prefix);
        if index < PAGE {
            out.push(0x30 + index as u8);
        } else {
            out.extend([0x2b + (index / PAGE) as u8, 0x30 + (index % PAGE) as u8]);
        }
        Ok(())
    }
}

/// Reads one character code at the start of `bytes`: (index, byte length).
fn read_char(bytes: &[u8]) -> Option<(u32, usize)> {
    match *bytes {
        [lead @ 0x2c..=0x2f, trail @ 0x30..=0xff, ..] => {
            Some((u32::from(lead - 0x2b) * PAGE + u32::from(trail - 0x30), 2))
        }
        [b @ 0x30..=0xff, ..] => Some((u32::from(b - 0x30), 1)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in table with the same shape as the real one: digits, space,
    /// punctuation, letters, a duplicate, and braces.
    fn charset() -> Charset {
        let mut table =
            String::from("0123456789 !-.{}ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-");
        while table.chars().count() < 0x200 {
            table.push('·');
        }
        table.push('★');
        Charset::from_table(table.as_bytes(), Language::English).unwrap()
    }

    #[test]
    fn round_trips_every_construct() {
        let cs = charset();
        for bytes in [
            &b"\x30\x3a\x3b"[..],
            b"\x03\x01\x0a\x41\x05",
            b"\x2d\x30", // a two-byte character
            b"\x5e",     // the duplicate '-' at its second index
            b"\x2c\x30", // a filler character with no unique form
            b"\x2c",     // lead byte without a trail
            b"\x3e\x3f", // braces
        ] {
            let text = cs.decode(bytes);
            assert_eq!(cs.encode(&text).unwrap(), bytes, "{text:?}");
        }
    }

    #[test]
    fn decodes_readably() {
        let cs = charset();
        assert_eq!(
            cs.decode(b"\x30\x3a\x03\x0a\x41\x3e\x00\x31"),
            "0 \n{0a:41}{{"
        );
        assert_eq!(
            cs.encode("Hi!").unwrap(),
            cs.encode("H")
                .unwrap()
                .into_iter()
                .chain(cs.encode("i!").unwrap())
                .collect::<Vec<_>>()
        );
        assert_eq!(cs.encode("é"), Err(EncodeError::UnknownChar('é')));
        assert!(matches!(cs.encode("{zz}"), Err(EncodeError::BadEscape(_))));
    }

    #[test]
    fn language_codes() {
        assert_eq!(Language::from_code("en"), Some(Language::English));
        assert_eq!(Language::from_code("JP"), Some(Language::Japanese));
        assert_eq!(
            Language::German.kernel_dir(),
            "ffx_ps2/ffx/master/new_depc/battle/kernel/"
        );
        assert_eq!(
            Language::French.charset_path(),
            "ffx_ps2/ffx/master/jppc/ffx_encoding/ffxsjistbl_us.bin"
        );
    }
}
