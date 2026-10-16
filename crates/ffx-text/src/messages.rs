//! Message tables: an event's dialogue
//! (`new_<lang>pc/event/obj_ps3/<xx>/<event>/<event>.bin`) and a battle's
//! texts (`new_<lang>pc/battle/btl/<name>/<name>.bin`), loaded by
//! `TKMallocAndReadEventMessage` (`0x0088cac0`).
//!
//! ```text
//! n x 8 bytes   entries: u16 text offset, u16 flags,
//!                        u16 alternate text offset, u16 alternate flags
//! ...           the strings, each ending in 00
//! ```
//!
//! Offsets are from the start of the file; the first string follows the
//! entries, so the first offset gives the count (all of a table whose
//! offsets are all 0). `Msg_Text` (`0x0086bf30`)
//! returns entry *i*'s text, `Msg_TextAlt` (`0x0086ba90`) its alternate,
//! and an offset of 0 falls back to the same entry of the event file's own
//! table (section 1 of the `.ebp`, the original Japanese). `Msg_Flags`
//! (`0x0086bf10`) reads the flags, which the message window keeps at
//! `+0x16` (`0x008607a0`).
//!
//! In every shipped table the strings follow each other in entry order
//! and each entry's alternate is the same as its text; [`MessageTable`]
//! keeps that layout and writes alternates that differ after their text.

use toml::{Table, Value};

use crate::{Charset, EncodeError};

const ENTRY_LEN: usize = 8;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("malformed message table: {0}")]
    Malformed(&'static str),
    #[error("TOML: {0}")]
    Toml(String),
    #[error("message {index}: {problem}")]
    Value { index: usize, problem: String },
    #[error("message {index}: {source}")]
    Encode {
        index: usize,
        #[source]
        source: EncodeError,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

/// One message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// The encoded text, without its terminating 00; `None` (offset 0) for
    /// the original table's entry.
    pub text: Option<Vec<u8>>,
    pub flags: u16,
    /// The alternate text and its flags, when they differ from the text's.
    pub alternate: Option<(Option<Vec<u8>>, u16)>,
}

/// A parsed message table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageTable {
    pub messages: Vec<Message>,
}

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}

impl MessageTable {
    /// Parses a message table.
    ///
    /// # Errors
    /// Fails on offsets outside the file and on layouts other than strings
    /// that follow each other in entry order.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < ENTRY_LEN {
            return Err(Error::Malformed("shorter than one entry"));
        }
        // The entries end where the first string starts; a table of
        // fallbacks only (13 battle tables) is all entries.
        let mut first = bytes.len();
        let mut at = 0;
        while at + ENTRY_LEN <= first {
            for o in [at, at + 4] {
                let v = usize::from(u16_at(bytes, o));
                if v != 0 {
                    first = first.min(v);
                }
            }
            at += ENTRY_LEN;
        }
        if !first.is_multiple_of(ENTRY_LEN) {
            return Err(Error::Malformed("entries do not end on an entry"));
        }
        let count = first / ENTRY_LEN;
        let entry = |i: usize| {
            let o = i * ENTRY_LEN;
            (
                usize::from(u16_at(bytes, o)),
                u16_at(bytes, o + 2),
                usize::from(u16_at(bytes, o + 4)),
                u16_at(bytes, o + 6),
            )
        };
        // Every string runs to the next one; the stored offsets must be the
        // texts in order, each followed by its alternate if that differs.
        let mut starts = Vec::with_capacity(count);
        for i in 0..count {
            let (text, _, alt, _) = entry(i);
            starts.extend([text].into_iter().filter(|&o| o != 0));
            if alt != text && alt != 0 {
                starts.push(alt);
            }
        }
        if starts.windows(2).any(|w| w[0] >= w[1]) {
            return Err(Error::Malformed("strings out of order"));
        }
        let string = |at: usize| -> Result<Option<Vec<u8>>> {
            if at == 0 {
                return Ok(None);
            }
            let end = starts
                .iter()
                .copied()
                .find(|&s| s > at)
                .unwrap_or(bytes.len());
            match bytes.get(at..end) {
                Some([text @ .., 0]) => Ok(Some(text.to_vec())),
                _ => Err(Error::Malformed("string without its 00")),
            }
        };
        let messages = (0..count)
            .map(|i| {
                let (text, flags, alt, alt_flags) = entry(i);
                Ok(Message {
                    text: string(text)?,
                    flags,
                    alternate: (alt != text || alt_flags != flags)
                        .then(|| Ok::<_, Error>((string(alt)?, alt_flags)))
                        .transpose()?,
                })
            })
            .collect::<Result<_>>()?;
        Ok(Self { messages })
    }

    /// Writes the table.
    ///
    /// # Errors
    /// Fails if the strings outgrow the 16-bit offsets.
    pub fn write(&self) -> Result<Vec<u8>> {
        let mut head = Vec::with_capacity(self.messages.len() * ENTRY_LEN);
        let mut strings = Vec::new();
        let base = self.messages.len() * ENTRY_LEN;
        let offset = |len: usize| {
            u16::try_from(base + len).map_err(|_| Error::Malformed("more than 64 KiB of text"))
        };
        let mut put = |text: &Option<Vec<u8>>| -> Result<u16> {
            let Some(text) = text else { return Ok(0) };
            let at = offset(strings.len())?;
            strings.extend(text);
            strings.push(0);
            Ok(at)
        };
        for m in &self.messages {
            let text = put(&m.text)?;
            let (alt, alt_flags) = match &m.alternate {
                None => (text, m.flags),
                Some((alt_text, alt_flags)) if alt_text == &m.text => (text, *alt_flags),
                Some((alt_text, alt_flags)) => (put(alt_text)?, *alt_flags),
            };
            for v in [text, m.flags, alt, alt_flags] {
                head.extend(v.to_le_bytes());
            }
        }
        head.extend(strings);
        Ok(head)
    }

    /// Exports the messages as TOML, decoded with `charset`: one
    /// `[[message]]` each with its `index` and `text` (absent for the
    /// original table's entry), `flags` when not 0, and
    /// `alternate`/`alternate_flags` when they differ.
    ///
    /// # Errors
    /// Fails only if TOML serialization fails.
    pub fn export(&self, charset: &Charset) -> Result<String> {
        let list = self
            .messages
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let mut t = Table::new();
                t.insert("index".into(), i64::try_from(i).unwrap_or(i64::MAX).into());
                if let Some(text) = &m.text {
                    t.insert("text".into(), charset.decode(text).into());
                }
                if m.flags != 0 {
                    t.insert("flags".into(), i64::from(m.flags).into());
                }
                if let Some((alt, alt_flags)) = &m.alternate {
                    if let Some(alt) = alt {
                        t.insert("alternate".into(), charset.decode(alt).into());
                    }
                    t.insert("alternate_flags".into(), i64::from(*alt_flags).into());
                }
                Value::Table(t)
            })
            .collect();
        let mut doc = Table::new();
        doc.insert("message".into(), Value::Array(list));
        toml::to_string(&doc).map_err(|e| Error::Toml(e.to_string()))
    }

    /// Applies TOML from [`Self::export`] onto the table: each
    /// `[[message]]` replaces the keys it lists of message `index` (the
    /// next message when `index` is absent). An `index` one past the end
    /// adds a message; the event's script decides which ones it shows.
    ///
    /// # Errors
    /// Fails on malformed TOML, unknown keys, indices that skip ahead, and
    /// text the charset cannot encode.
    pub fn import(&mut self, text: &str, charset: &Charset) -> Result<()> {
        let doc: Table = text
            .parse()
            .map_err(|e: toml::de::Error| Error::Toml(e.to_string()))?;
        if let Some(key) = doc.keys().find(|k| *k != "message") {
            return Err(Error::Toml(format!("unknown table [{key}]")));
        }
        let Some(list) = doc.get("message") else {
            return Ok(());
        };
        let list = list
            .as_array()
            .ok_or_else(|| Error::Toml("expected [[message]] tables".into()))?;
        let mut next = 0;
        for v in list {
            let t = v
                .as_table()
                .ok_or_else(|| Error::Toml("expected [[message]] tables".into()))?;
            let index = match t.get("index") {
                None => next,
                Some(v) => v
                    .as_integer()
                    .and_then(|i| usize::try_from(i).ok())
                    .ok_or_else(|| Error::Toml(format!("bad index {v}")))?,
            };
            let err = |problem: String| Error::Value { index, problem };
            if index > self.messages.len() {
                return Err(err(format!(
                    "there are {} messages; add them in order",
                    self.messages.len()
                )));
            }
            if index == self.messages.len() {
                self.messages.push(Message {
                    text: Some(Vec::new()),
                    flags: 0,
                    alternate: None,
                });
            }
            let encode = |v: &Value| -> Result<Option<Vec<u8>>> {
                let s = v
                    .as_str()
                    .ok_or_else(|| err(format!("expected a string, got {v}")))?;
                charset
                    .encode(s)
                    .map(Some)
                    .map_err(|source| Error::Encode { index, source })
            };
            let flag = |v: &Value| -> Result<u16> {
                v.as_integer()
                    .and_then(|i| u16::try_from(i).ok())
                    .ok_or_else(|| err(format!("flags: expected 0-65535, got {v}")))
            };
            let m = &mut self.messages[index];
            for (key, v) in t {
                match key.as_str() {
                    "index" => {}
                    "text" => m.text = encode(v)?,
                    "flags" => m.flags = flag(v)?,
                    "alternate" => {
                        let flags = m.alternate.as_ref().map_or(m.flags, |a| a.1);
                        m.alternate = Some((encode(v)?, flags));
                    }
                    "alternate_flags" => {
                        let text = m.alternate.take().map_or_else(|| m.text.clone(), |a| a.0);
                        m.alternate = Some((text, flag(v)?));
                    }
                    other => return Err(err(format!("unknown key `{other}`"))),
                }
            }
            if m.alternate.as_ref() == Some(&(m.text.clone(), m.flags)) {
                m.alternate = None;
            }
            next = index + 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Language;

    fn charset() -> Charset {
        // Codes 0x30.. are "0".."9", then a space and letters, enough here.
        let table = "0123456789 ?!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        Charset::from_table(table.as_bytes(), Language::English).unwrap()
    }

    #[test]
    fn parses_writes_and_edits() {
        let cs = charset();
        let hi = cs.encode("HI").unwrap();
        let yes = cs.encode("YES\nNO").unwrap();
        let mut bytes = Vec::new();
        for (o, f) in [(16u16, 0u16), (19, 0x200)] {
            for v in [o, f, o, f] {
                bytes.extend(v.to_le_bytes());
            }
        }
        bytes.extend(&hi);
        bytes.push(0);
        bytes.extend(&yes);
        bytes.push(0);

        let mut table = MessageTable::parse(&bytes).unwrap();
        assert_eq!(table.messages[1].flags, 0x200);
        assert_eq!(table.messages[0].text.as_ref(), Some(&hi));
        assert_eq!(table.write().unwrap(), bytes);

        let text = table.export(&cs).unwrap();
        assert!(text.contains("text = \"HI\""), "{text}");
        let mut same = table.clone();
        same.import(&text, &cs).unwrap();
        assert_eq!(same, table);

        table
            .import("[[message]]\nindex = 0\ntext = \"HELLO THERE\"\n[[message]]\nindex = 2\ntext = \"NEW\"\n", &cs)
            .unwrap();
        let out = MessageTable::parse(&table.write().unwrap()).unwrap();
        assert_eq!(out.messages.len(), 3);
        assert_eq!(
            cs.decode(out.messages[0].text.as_ref().unwrap()),
            "HELLO THERE"
        );
        assert_eq!(out.messages[1].text.as_ref(), Some(&yes));
        assert!(
            table
                .import("[[message]]\nindex = 9\ntext = \"X\"\n", &cs)
                .is_err()
        );
        assert!(
            table.import("[[message]]\ntext = \"x\"\n", &cs).is_err(),
            "lower case is not in the charset"
        );

        table
            .import("[[message]]\nindex = 1\nalternate = \"NO\"\n", &cs)
            .unwrap();
        let out = MessageTable::parse(&table.write().unwrap()).unwrap();
        assert_eq!(
            out.messages[1].alternate,
            Some((Some(cs.encode("NO").unwrap()), 0x200))
        );

        // Offsets of 0: the original table's entries.
        let fallback = MessageTable::parse(&[0; 16]).unwrap();
        assert_eq!(fallback.messages.len(), 2);
        assert_eq!(fallback.messages[0].text, None);
        assert_eq!(fallback.write().unwrap(), [0; 16]);
        assert!(!fallback.export(&cs).unwrap().contains("text"));
    }
}
