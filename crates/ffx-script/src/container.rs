//! The files scripts live in.
//!
//! - Monster files (`battle/mon/_mNNN/mNNN.bin`): `u32` count 8, then eight
//!   `u32` section offsets; the script is section 0.
//! - Event files (`event/**/*.ebp`): `"EV01"`, then section offsets from
//!   `0x04` up to a `0xffffffff` terminator, 0 for absent sections; the
//!   script is section 0, at `0x40`. All offsets are multiples of `0x40`.
//!
//! In both, sections are in ascending order and each runs to the next.

use crate::script::Script;
use crate::{Error, Result};

/// Which kind of file holds the script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Monster,
    Event,
}

/// A container with its script.
#[derive(Debug, Clone)]
pub struct Container {
    pub kind: Kind,
    bytes: Vec<u8>,
    /// Byte positions of the section offset words.
    offset_words: Vec<usize>,
    /// The script section: (offset, length up to the next section).
    section: (usize, usize),
    pub script: Script,
}

fn u32_at(b: &[u8], o: usize) -> Option<usize> {
    b.get(o..o + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")) as usize)
}

/// Sections in these files start on this boundary; growing a section shifts
/// later ones by a multiple of it.
const ALIGN: usize = 0x40;

impl Container {
    /// Opens a monster or event file.
    ///
    /// # Errors
    /// Fails on files of neither kind and malformed scripts.
    pub fn parse(bytes: Vec<u8>) -> Result<Self> {
        let (kind, offset_words): (Kind, Vec<usize>) = if bytes.starts_with(b"EV01") {
            let words = (1..16)
                .map(|i| i * 4)
                .take_while(|&o| u32_at(&bytes, o).is_some_and(|v| v != 0xffff_ffff))
                .collect();
            (Kind::Event, words)
        } else if u32_at(&bytes, 0) == Some(8) {
            (Kind::Monster, (1..=8).map(|i| i * 4).collect())
        } else {
            return Err(Error::Malformed("neither a monster nor an event file"));
        };
        let offsets: Vec<usize> = offset_words
            .iter()
            .filter_map(|&o| u32_at(&bytes, o))
            .filter(|&v| v != 0)
            .collect();
        let start = *offsets
            .first()
            .ok_or(Error::Malformed("no script section"))?;
        let end = offsets
            .iter()
            .copied()
            .find(|&o| o > start)
            .unwrap_or(bytes.len());
        if end > bytes.len() || offsets.windows(2).any(|w| w[0] > w[1]) {
            return Err(Error::Malformed("sections out of order"));
        }
        let script = Script::parse(bytes[start..end].to_vec())?;
        Ok(Self {
            kind,
            bytes,
            offset_words,
            section: (start, end - start),
            script,
        })
    }

    /// The file's bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Puts a rebuilt script back. A script that grew past its section
    /// shifts every later section by a multiple of `0x40`.
    ///
    /// # Errors
    /// Fails if the file would outgrow 32-bit offsets.
    pub fn replace_script(&self, script: &Script) -> Result<Self> {
        let (start, room) = self.section;
        let new = script.bytes();
        let grow = new.len().saturating_sub(room).next_multiple_of(ALIGN);
        let mut bytes = Vec::with_capacity(self.bytes.len() + grow);
        bytes.extend(&self.bytes[..start]);
        bytes.extend(new);
        // Keep the original padding up to the next section, shifted.
        bytes.resize(start + room + grow, 0);
        if new.len() < room {
            bytes[start + new.len()..start + room]
                .copy_from_slice(&self.bytes[start + new.len()..start + room]);
        }
        bytes.extend(&self.bytes[start + room..]);
        if grow > 0 {
            for &w in &self.offset_words {
                let v = u32_at(&bytes, w).expect("offset word");
                if v > start {
                    let moved = u32::try_from(v + grow)
                        .map_err(|_| Error::Shape("file too large".into()))?;
                    bytes[w..w + 4].copy_from_slice(&moved.to_le_bytes());
                }
            }
        }
        Self::parse(bytes)
    }
}
