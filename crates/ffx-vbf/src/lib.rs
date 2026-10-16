//! Reader for the VBF archives (`FFX_Data.vbf`, `FFX2_Data.vbf`, `metamenu.vbf`)
//! shipped with the FFX/X-2 HD Remaster.
//!
//! The layout was recovered from `BigFileHandle_initialize` (`0x0061dac0`) in `FFX.exe`; see
//! `docs/formats/vbf.md` for the full description and the evidence behind it.
//!
//! An archive is a header followed by file data. Every file is split into
//! 64 KiB blocks, and each block is either stored raw or zlib-compressed.
//! Files are looked up by the MD5 digest of their path, exactly as the game
//! does.

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::Path;

use md5::{Digest, Md5};

mod format;
mod pack;

pub use format::{BLOCK_SIZE, MAGIC};
pub use pack::{PackStats, pack};

/// An MD5 digest, used both for the header checksum and for path lookup.
pub type Md5Digest = [u8; 16];

/// Errors returned while opening or reading an archive.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("not a VBF archive (magic {0:#010x}, expected \"SRYK\")")]
    BadMagic(u32),
    #[error("header digest mismatch: header hashes to {computed:02x?}, trailer says {stored:02x?}")]
    DigestMismatch {
        computed: Md5Digest,
        stored: Md5Digest,
    },
    #[error("malformed header: {0}")]
    Malformed(&'static str),
    #[error("block {block} of {path} failed to decompress: {source}")]
    Decompress {
        path: String,
        block: usize,
        #[source]
        source: io::Error,
    },
    #[error("{path}: decompressed {actual} bytes, expected {expected}")]
    SizeMismatch {
        path: String,
        expected: u64,
        actual: u64,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

/// One file stored in an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// MD5 of [`Entry::path`]; the key the game looks files up by.
    pub path_digest: Md5Digest,
    /// Path inside the archive, e.g. `ffx_data/gamedata/ps3data/...`.
    pub path: String,
    /// Index of the file's first block in the block-size table.
    pub first_block: u32,
    /// Second header word of the entry record; not read by `BigFileHandle_initialize`.
    /// Equal to `first_block` in `metamenu.vbf`. In `FFX_Data.vbf` it is
    /// `0` for the 7 empty files and `3675126` for all others. Its purpose is
    /// still unknown.
    pub unknown_04: u32,
    /// Size of the file once decompressed.
    pub size: u64,
    /// Absolute file offset of the file's first block. Meaningless for empty
    /// files, which carry junk here.
    pub offset: u64,
    /// Offset of the path in the path table.
    pub path_offset: u64,
}

impl Entry {
    /// Number of 64 KiB blocks the file is split into.
    #[must_use]
    pub fn block_count(&self) -> u64 {
        self.size.div_ceil(BLOCK_SIZE as u64)
    }
}

/// An open archive. Reads use positioned I/O, so one archive can serve
/// several threads at once.
#[derive(Debug)]
pub struct Archive {
    file: File,
    entries: Vec<Entry>,
    /// Stored size of every block; `0` means a raw 64 KiB block.
    block_sizes: Vec<u16>,
    by_digest: HashMap<Md5Digest, usize>,
    /// The header word at `0x0c` (zero in every shipped archive).
    reserved: u32,
    /// The raw path table.
    path_table: Vec<u8>,
}

impl Archive {
    /// Opens an archive and verifies the MD5 digest of its header.
    ///
    /// # Errors
    /// Fails on I/O errors, a wrong magic number, a digest mismatch, or a
    /// header whose tables do not fit inside it.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path)?;
        let file_len = file.metadata()?.len();

        let mut prefix = [0u8; 8];
        read_exact_at(&file, &mut prefix, 0)?;
        let magic = u32::from_le_bytes(prefix[0..4].try_into().expect("4 bytes"));
        if magic != MAGIC {
            return Err(Error::BadMagic(magic));
        }
        let header_len = u64::from(u32::from_le_bytes(
            prefix[4..8].try_into().expect("4 bytes"),
        ));
        if header_len > file_len.saturating_sub(16) {
            return Err(Error::Malformed("header length exceeds file size"));
        }

        let mut header = vec![
            0u8;
            usize::try_from(header_len)
                .map_err(|_| Error::Malformed("header too large"))?
        ];
        read_exact_at(&file, &mut header, 0)?;
        let mut stored = [0u8; 16];
        read_exact_at(&file, &mut stored, file_len - 16)?;
        let computed: Md5Digest = Md5::digest(&header).into();
        if computed != stored {
            return Err(Error::DigestMismatch { computed, stored });
        }

        let parsed = format::parse_header(&header)?;
        let by_digest = parsed
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.path_digest, i))
            .collect();
        Ok(Self {
            file,
            entries: parsed.entries,
            block_sizes: parsed.block_sizes,
            by_digest,
            reserved: parsed.reserved,
            path_table: parsed.path_table,
        })
    }

    /// All entries, in the order the header stores them (the order of the file data).
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Looks up a file by its path inside the archive, normalized the way the
    /// game does it (see [`normalize_path`]), so `FFX_Data\GameData\x` finds
    /// `ffx_data/gamedata/x`.
    #[must_use]
    pub fn find(&self, path: &str) -> Option<&Entry> {
        self.find_digest(&Md5::digest(normalize_path(path).as_bytes()).into())
    }

    /// Looks up a file by the MD5 digest of its path.
    #[must_use]
    pub fn find_digest(&self, digest: &Md5Digest) -> Option<&Entry> {
        self.by_digest.get(digest).map(|&i| &self.entries[i])
    }

    /// The stored blocks of a file, in order.
    ///
    /// # Errors
    /// Fails if the entry points outside the block table.
    pub fn blocks(&self, entry: &Entry) -> Result<Vec<Block>> {
        let mut blocks = Vec::new();
        let mut offset = entry.offset;
        for i in 0..entry.block_count() {
            let index = usize::try_from(u64::from(entry.first_block) + i)
                .map_err(|_| Error::Malformed("block index overflow"))?;
            let size = usize::try_from((entry.size - i * BLOCK_SIZE as u64).min(BLOCK_SIZE as u64))
                .expect("at most BLOCK_SIZE");
            let stored = match self.block_sizes.get(index) {
                Some(0) => BLOCK_SIZE,
                Some(&n) => usize::from(n),
                None => return Err(Error::Malformed("block index outside the block table")),
            };
            blocks.push(Block {
                index,
                offset,
                stored,
                size,
            });
            offset += stored as u64;
        }
        Ok(blocks)
    }

    /// Reads and decompresses a whole file.
    ///
    /// # Errors
    /// Fails on I/O errors, corrupt zlib data, or a size mismatch.
    pub fn read(&self, entry: &Entry) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(usize::try_from(entry.size).unwrap_or(0));
        let mut buf = vec![0u8; BLOCK_SIZE];
        for block in self.blocks(entry)? {
            let stored = &mut buf[..block.stored];
            read_exact_at(&self.file, stored, block.offset)?;
            if !block.is_compressed() {
                out.extend_from_slice(stored);
                continue;
            }
            let before = out.len();
            let mut decoder = flate2::read::ZlibDecoder::new(&stored[..]);
            io::copy(&mut decoder, &mut out).map_err(|source| Error::Decompress {
                path: entry.path.clone(),
                block: block.index,
                source,
            })?;
            if out.len() - before != block.size {
                return Err(Error::SizeMismatch {
                    path: entry.path.clone(),
                    expected: block.size as u64,
                    actual: (out.len() - before) as u64,
                });
            }
        }
        Ok(out)
    }
}

/// One 64 KiB block of a file, as stored in the archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    /// Index in the archive's block-size table.
    pub index: usize,
    /// Absolute file offset of the stored bytes.
    pub offset: u64,
    /// Stored length.
    pub stored: usize,
    /// Length once decompressed.
    pub size: usize,
}

impl Block {
    /// Whether the block is a zlib stream; raw blocks have equal sizes.
    #[must_use]
    pub fn is_compressed(&self) -> bool {
        self.stored != self.size
    }
}

/// Normalizes a path before hashing, as `File_Probe` (`0x00607e00`) does:
/// `\` becomes `/` and ASCII `A`–`Z` become lowercase. Other bytes, including
/// non-ASCII ones, are left alone.
#[must_use]
pub fn normalize_path(path: &str) -> String {
    path.chars()
        .map(|c| {
            if c == '\\' {
                '/'
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect()
}

#[cfg(unix)]
fn read_exact_at(file: &File, buf: &mut [u8], offset: u64) -> io::Result<()> {
    std::os::unix::fs::FileExt::read_exact_at(file, buf, offset)
}

#[cfg(windows)]
fn read_exact_at(file: &File, mut buf: &mut [u8], mut offset: u64) -> io::Result<()> {
    use std::os::windows::fs::FileExt;
    while !buf.is_empty() {
        match file.seek_read(buf, offset)? {
            0 => return Err(io::ErrorKind::UnexpectedEof.into()),
            n => {
                buf = &mut buf[n..];
                offset += n as u64;
            }
        }
    }
    Ok(())
}
