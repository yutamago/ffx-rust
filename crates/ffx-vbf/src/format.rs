//! Header layout. Offsets are relative to the start of the file; all
//! integers are little-endian.
//!
//! ```text
//! 0x00  u32        magic "SRYK"
//! 0x04  u32        header length (file data starts here)
//! 0x08  u32        entry count N
//! 0x0c  u32        zero
//! 0x10  [u8;16]*N  MD5 of each entry's path, in data order (not sorted)
//! ....  Record*N   one 32-byte record per digest, same order
//! ....  u32        L = 4 + length of the path table
//! ....  [u8]       NUL-terminated paths, L - 4 bytes
//! ....  u16*B      stored size of every block; B = sum of block counts
//! ```
//!
//! The header is followed by the file data, and the archive ends with the
//! MD5 of the header bytes.

use crate::{Entry, Error, Result};

/// `"SRYK"` read as a little-endian `u32`.
pub const MAGIC: u32 = 0x4b59_5253;

/// Files are split into blocks of this many bytes before compression.
pub const BLOCK_SIZE: usize = 0x1_0000;

const DIGESTS_OFFSET: usize = 0x10;
const DIGEST_LEN: usize = 16;
const RECORD_LEN: usize = 32;

pub(crate) struct Header {
    pub entries: Vec<Entry>,
    pub block_sizes: Vec<u16>,
}

pub(crate) fn parse_header(header: &[u8]) -> Result<Header> {
    let r = Reader(header);
    let count = r.u32(0x08)? as usize;

    let records_offset = count
        .checked_mul(DIGEST_LEN)
        .and_then(|n| n.checked_add(DIGESTS_OFFSET))
        .ok_or(Error::Malformed("entry count overflow"))?;
    let path_table_len_offset = count
        .checked_mul(RECORD_LEN)
        .and_then(|n| n.checked_add(records_offset))
        .ok_or(Error::Malformed("entry count overflow"))?;
    let path_table_len = r.u32(path_table_len_offset)? as usize;
    let paths_offset = path_table_len_offset + 4;
    let block_table_offset = path_table_len_offset
        .checked_add(path_table_len)
        .ok_or(Error::Malformed("path table length overflow"))?;
    if path_table_len < 4 || block_table_offset > header.len() {
        return Err(Error::Malformed("path table outside the header"));
    }
    let paths = &header[paths_offset..block_table_offset];

    let mut entries = Vec::with_capacity(count);
    let mut block_count: u64 = 0;
    for i in 0..count {
        let digest_at = DIGESTS_OFFSET + i * DIGEST_LEN;
        let rec = records_offset + i * RECORD_LEN;
        let path_offset = usize::try_from(r.u64(rec + 0x18)?)
            .map_err(|_| Error::Malformed("path offset overflow"))?;
        let entry = Entry {
            path_digest: r
                .bytes(digest_at, DIGEST_LEN)?
                .try_into()
                .expect("16 bytes"),
            path: c_string(paths, path_offset)?,
            first_block: r.u32(rec)?,
            unknown_04: r.u32(rec + 0x04)?,
            size: r.u64(rec + 0x08)?,
            offset: r.u64(rec + 0x10)?,
        };
        block_count = block_count.max(u64::from(entry.first_block) + entry.block_count());
        entries.push(entry);
    }

    let block_count =
        usize::try_from(block_count).map_err(|_| Error::Malformed("block count overflow"))?;
    let raw = r.bytes(block_table_offset, block_count * 2)?;
    let block_sizes = raw
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&b| u16::from_le_bytes(b))
        .collect();

    Ok(Header {
        entries,
        block_sizes,
    })
}

fn c_string(table: &[u8], offset: usize) -> Result<String> {
    let rest = table
        .get(offset..)
        .ok_or(Error::Malformed("path offset outside the path table"))?;
    let end = rest
        .iter()
        .position(|&b| b == 0)
        .ok_or(Error::Malformed("unterminated path"))?;
    String::from_utf8(rest[..end].to_vec()).map_err(|_| Error::Malformed("path is not UTF-8"))
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn bytes(&self, offset: usize, len: usize) -> Result<&'a [u8]> {
        offset
            .checked_add(len)
            .and_then(|end| self.0.get(offset..end))
            .ok_or(Error::Malformed("table extends past the header"))
    }

    fn u32(&self, offset: usize) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.bytes(offset, 4)?.try_into().expect("4 bytes"),
        ))
    }

    fn u64(&self, offset: usize) -> Result<u64> {
        Ok(u64::from_le_bytes(
            self.bytes(offset, 8)?.try_into().expect("8 bytes"),
        ))
    }
}
