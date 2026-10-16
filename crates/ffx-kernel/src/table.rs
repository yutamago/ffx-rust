//! The table container.
//!
//! Recovered from `MsGetExcelData` (`0x007ab870`):
//!
//! ```text
//! 0x00  u16        segment count S
//! 0x02  [u8; 6]    zero in every shipped table
//! 0x08  Segment*S  12 bytes each:
//!         u16  first record ID
//!         u16  last record ID (inclusive)
//!         u16  record size
//!         u16  data length = size × count
//!         u32  data offset
//! ....  records    each segment's records, back to back
//! ....  strings    NUL-terminated strings, to the end of the file
//! ```
//!
//! A record's strings are referenced by 4-byte pairs at its start: a `u16`
//! offset into the string area and a `u16` the game does not read. Strings
//! share storage (one may be a suffix of another), so the area is kept as
//! is and edited strings are appended.

use crate::{Error, Result};

const HEADER_LEN: usize = 8;
const SEGMENT_LEN: usize = 12;

/// A run of consecutive record IDs with one record size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub first: u16,
    pub record_size: u16,
    data: Vec<u8>,
}

impl Segment {
    /// Number of records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len() / usize::from(self.record_size)
    }

    /// Whether the segment has no records.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// ID of the last record.
    #[must_use]
    pub fn last(&self) -> u16 {
        self.first + u16::try_from(self.len()).expect("checked at parse") - 1
    }
}

/// A reference from a record to a string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringRef {
    /// Offset into the string area.
    pub offset: u16,
    /// Second half of the pair; not read by the game's string lookups.
    pub aux: u16,
}

/// A parsed kernel table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    reserved: [u8; 6],
    segments: Vec<Segment>,
    strings: Vec<u8>,
}

impl Table {
    /// Parses a table file.
    ///
    /// # Errors
    /// Fails if the file is truncated or its segments are not laid out back
    /// to back after the header, as in every shipped table.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let u16_at = |o: usize| -> Result<u16> {
            bytes
                .get(o..o + 2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .ok_or(Error::Malformed("truncated header"))
        };
        let count = usize::from(u16_at(0)?);
        if count == 0 {
            return Err(Error::Malformed("no segments"));
        }
        let reserved = bytes
            .get(2..HEADER_LEN)
            .ok_or(Error::Malformed("truncated header"))?
            .try_into()
            .expect("6 bytes");
        let mut expected = HEADER_LEN + count * SEGMENT_LEN;
        let mut segments = Vec::with_capacity(count);
        for i in 0..count {
            let at = HEADER_LEN + i * SEGMENT_LEN;
            let (first, last, size, len) = (
                u16_at(at)?,
                u16_at(at + 2)?,
                u16_at(at + 4)?,
                u16_at(at + 6)?,
            );
            let offset = bytes
                .get(at + 8..at + 12)
                .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")) as usize)
                .ok_or(Error::Malformed("truncated header"))?;
            if size == 0
                || last < first
                || usize::from(size) * usize::from(last - first + 1) != usize::from(len)
            {
                return Err(Error::Malformed(
                    "segment length does not match its records",
                ));
            }
            if offset != expected {
                return Err(Error::Malformed("segments are not back to back"));
            }
            let data = bytes
                .get(offset..offset + usize::from(len))
                .ok_or(Error::Malformed("segment extends past the end"))?
                .to_vec();
            expected += data.len();
            segments.push(Segment {
                first,
                record_size: size,
                data,
            });
        }
        Ok(Self {
            reserved,
            segments,
            strings: bytes[expected..].to_vec(),
        })
    }

    /// Serializes the table.
    ///
    /// # Errors
    /// Fails if the table has grown past the format's 16-bit limits.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        out.extend(
            u16::try_from(self.segments.len())
                .map_err(|_| Error::TooLarge)?
                .to_le_bytes(),
        );
        out.extend(self.reserved);
        let mut offset = HEADER_LEN + self.segments.len() * SEGMENT_LEN;
        for s in &self.segments {
            out.extend(s.first.to_le_bytes());
            out.extend(s.last().to_le_bytes());
            out.extend(s.record_size.to_le_bytes());
            out.extend(
                u16::try_from(s.data.len())
                    .map_err(|_| Error::TooLarge)?
                    .to_le_bytes(),
            );
            out.extend(
                u32::try_from(offset)
                    .map_err(|_| Error::TooLarge)?
                    .to_le_bytes(),
            );
            offset += s.data.len();
        }
        for s in &self.segments {
            out.extend(&s.data);
        }
        out.extend(&self.strings);
        Ok(out)
    }

    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// All record IDs, in file order.
    pub fn ids(&self) -> impl Iterator<Item = u16> + '_ {
        self.segments.iter().flat_map(|s| s.first..=s.last())
    }

    fn locate(&self, id: u16) -> Option<(usize, usize, usize)> {
        self.segments.iter().enumerate().find_map(|(i, s)| {
            (s.first..=s.last()).contains(&id).then(|| {
                let size = usize::from(s.record_size);
                (i, usize::from(id - s.first) * size, size)
            })
        })
    }

    /// The record with this ID.
    #[must_use]
    pub fn record(&self, id: u16) -> Option<&[u8]> {
        let (s, at, size) = self.locate(id)?;
        Some(&self.segments[s].data[at..at + size])
    }

    /// The record with this ID, mutably.
    pub fn record_mut(&mut self, id: u16) -> Option<&mut [u8]> {
        let (s, at, size) = self.locate(id)?;
        Some(&mut self.segments[s].data[at..at + size])
    }

    /// The `field`th string reference of a record.
    #[must_use]
    pub fn string_ref(&self, id: u16, field: usize) -> Option<StringRef> {
        let r = self.record(id)?.get(field * 4..field * 4 + 4)?;
        Some(StringRef {
            offset: u16::from_le_bytes([r[0], r[1]]),
            aux: u16::from_le_bytes([r[2], r[3]]),
        })
    }

    /// The bytes of a referenced string, without its terminator.
    #[must_use]
    pub fn string(&self, id: u16, field: usize) -> Option<&[u8]> {
        let start = usize::from(self.string_ref(id, field)?.offset);
        let rest = self.strings.get(start..)?;
        Some(&rest[..rest.iter().position(|&b| b == 0).unwrap_or(rest.len())])
    }

    /// Points a record's `field`th string at `text` (encoded, without
    /// terminator), reusing an identical string already in the area or
    /// appending it. The pair's second half is kept.
    ///
    /// # Errors
    /// Fails if the record or field does not exist, or the string area
    /// would outgrow 16-bit offsets.
    pub fn set_string(&mut self, id: u16, field: usize, text: &[u8]) -> Result<()> {
        if self.string(id, field) == Some(text) {
            return Ok(());
        }
        let mut wanted = text.to_vec();
        wanted.push(0);
        let offset = match self.strings.windows(wanted.len()).position(|w| w == wanted) {
            Some(at) => at,
            None => {
                let at = self.strings.len();
                self.strings.extend(&wanted);
                at
            }
        };
        let offset = u16::try_from(offset).map_err(|_| Error::TooLarge)?;
        let record = self.record_mut(id).ok_or(Error::NoRecord(id))?;
        let pair = record
            .get_mut(field * 4..field * 4 + 2)
            .ok_or(Error::NoField(field))?;
        pair.copy_from_slice(&offset.to_le_bytes());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two records of 8 bytes: one string pair and 4 data bytes each.
    fn sample() -> Vec<u8> {
        let mut t = vec![1, 0, 0, 0, 0, 0, 0, 0];
        t.extend([5, 0, 6, 0, 8, 0, 16, 0, 0x14, 0, 0, 0]);
        t.extend([0, 0, 7, 0, 1, 2, 3, 4]);
        t.extend([3, 0, 9, 0, 5, 6, 7, 8]);
        t.extend(b"abc\0de\0");
        t
    }

    #[test]
    fn round_trips() {
        let t = Table::parse(&sample()).unwrap();
        assert_eq!(t.to_bytes().unwrap(), sample());
        assert_eq!(t.ids().collect::<Vec<_>>(), [5, 6]);
        assert_eq!(t.string(5, 0), Some(&b"abc"[..]));
        assert_eq!(t.string(6, 0), Some(&b""[..]));
        assert_eq!(t.string_ref(6, 0), Some(StringRef { offset: 3, aux: 9 }));
        assert_eq!(t.record(6).unwrap()[4..], [5, 6, 7, 8]);
        assert!(t.record(7).is_none());
    }

    #[test]
    fn edits_strings_without_moving_others() {
        let mut t = Table::parse(&sample()).unwrap();
        t.set_string(6, 0, b"de").unwrap(); // already present: reused
        assert_eq!(t.string_ref(6, 0).unwrap().offset, 4);
        t.set_string(5, 0, b"xyz").unwrap(); // new: appended
        assert_eq!(t.string(5, 0), Some(&b"xyz"[..]));
        assert_eq!(t.string_ref(5, 0).unwrap().aux, 7);
        let bytes = t.to_bytes().unwrap();
        assert!(bytes.ends_with(b"abc\0de\0xyz\0"));
        assert_eq!(Table::parse(&bytes).unwrap(), t);
    }

    #[test]
    fn rejects_bad_layouts() {
        let mut bad = sample();
        bad[16] = 0x18; // data offset no longer follows the header
        assert!(Table::parse(&bad).is_err());
        assert!(Table::parse(&[0, 0]).is_err());
    }
}
