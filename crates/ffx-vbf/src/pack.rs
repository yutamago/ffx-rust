//! Writing archives.
//!
//! [`pack`] rebuilds an archive from an existing one plus changed and added
//! files. Unchanged files keep their stored (already compressed) blocks,
//! copied verbatim, so packing with no changes reproduces the original byte
//! for byte. Changed and added files are split into 64 KiB blocks and
//! zlib-compressed where that saves space, as in the shipped archives.
//!
//! What `Vbf_Open` checks is kept: the magic, the header layout and the MD5
//! of the header in the last 16 bytes. Fields the game does not read are
//! carried over from the base archive.

use std::collections::{BTreeMap, HashMap};
use std::io::Write;

use md5::{Digest, Md5};

use crate::format::MAGIC;
use crate::{Archive, BLOCK_SIZE, Error, Result, normalize_path};

/// What [`pack`] did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PackStats {
    pub files: usize,
    pub replaced: usize,
    pub added: usize,
    /// Bytes written.
    pub bytes: u64,
}

enum Source {
    /// Copy this many stored bytes from the base archive at this offset.
    Copy { offset: u64, len: u64 },
    /// Freshly stored blocks.
    New(Vec<u8>),
}

struct Planned {
    digest: [u8; 16],
    path_offset: u64,
    unknown_04: Option<u32>,
    size: u64,
    /// Kept for unchanged empty files, whose offset field is junk.
    junk_offset: Option<u64>,
    block_sizes: Vec<u16>,
    source: Source,
}

/// Splits data into stored blocks: zlib where it is smaller, raw otherwise
/// (stored size 0 for a raw full block).
fn store(data: &[u8]) -> Result<(Vec<u16>, Vec<u8>)> {
    let mut sizes = Vec::new();
    let mut out = Vec::new();
    for chunk in data.chunks(BLOCK_SIZE) {
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
        z.write_all(chunk)?;
        let z = z.finish()?;
        if z.len() < chunk.len() {
            sizes.push(u16::try_from(z.len()).expect("smaller than a block"));
            out.extend(&z);
        } else {
            sizes.push(if chunk.len() == BLOCK_SIZE {
                0
            } else {
                u16::try_from(chunk.len()).expect("partial block")
            });
            out.extend(chunk);
        }
    }
    Ok((sizes, out))
}

/// Rebuilds `base` with `changes` (archive path → new contents) applied and
/// writes it to `out`. Paths are normalized like the game's lookups; paths
/// not in `base` are added after its files.
///
/// # Errors
/// Fails on I/O errors and archives that outgrow the format.
pub fn pack(
    base: &Archive,
    changes: &BTreeMap<String, Vec<u8>>,
    out: &mut impl Write,
) -> Result<PackStats> {
    let mut stats = PackStats::default();
    let plan = plan(base, changes, &mut stats)?;
    let (header, data_end) = header(base, &plan)?;
    let digest: [u8; 16] = Md5::digest(&header).into();
    let planned = plan.entries;
    out.write_all(&header)?;
    let mut buf = vec![0u8; 1 << 20];
    for p in &planned {
        match &p.source {
            Source::New(bytes) => out.write_all(bytes)?,
            Source::Copy { offset, len } => {
                let mut done = 0;
                while done < *len {
                    let step =
                        usize::try_from((*len - done).min(buf.len() as u64)).expect("bounded");
                    crate::read_exact_at(&base.file, &mut buf[..step], offset + done)?;
                    out.write_all(&buf[..step])?;
                    done += step as u64;
                }
            }
        }
    }
    out.write_all(&digest)?;
    stats.bytes = data_end + 16;
    Ok(stats)
}

/// The files of the new archive, in order.
struct Plan {
    entries: Vec<Planned>,
    path_table: Vec<u8>,
    mirrors_first_block: bool,
    common_04: u32,
}

fn plan(
    base: &Archive,
    changes: &BTreeMap<String, Vec<u8>>,
    stats: &mut PackStats,
) -> Result<Plan> {
    let changes: BTreeMap<String, &Vec<u8>> = changes
        .iter()
        .map(|(p, d)| (normalize_path(p), d))
        .collect();

    // Field 0x04 of new records: shipped archives use either the first-block
    // index or one constant; follow whichever the base does.
    let non_empty: Vec<_> = base.entries().iter().filter(|e| e.size > 0).collect();
    let mirrors_first_block = non_empty.iter().all(|e| e.unknown_04 == e.first_block);
    let common_04 = {
        let mut counts: HashMap<u32, usize> = HashMap::new();
        for e in &non_empty {
            *counts.entry(e.unknown_04).or_default() += 1;
        }
        counts
            .into_iter()
            .max_by_key(|&(v, n)| (n, v))
            .map_or(0, |(v, _)| v)
    };

    let mut path_table = base.path_table.clone();
    let mut planned = Vec::with_capacity(base.entries().len());
    for entry in base.entries() {
        let replacement = changes.get(&entry.path);
        let (block_sizes, source, size) = match replacement {
            Some(data) => {
                let (sizes, stored) = store(data)?;
                stats.replaced += 1;
                (sizes, Source::New(stored), data.len() as u64)
            }
            None => {
                let blocks = base.blocks(entry)?;
                let sizes = blocks.iter().map(|b| base.block_sizes[b.index]).collect();
                let len = blocks.iter().map(|b| b.stored as u64).sum();
                (
                    sizes,
                    Source::Copy {
                        offset: entry.offset,
                        len,
                    },
                    entry.size,
                )
            }
        };
        planned.push(Planned {
            digest: entry.path_digest,
            path_offset: entry.path_offset,
            unknown_04: Some(entry.unknown_04),
            junk_offset: (replacement.is_none() && entry.size == 0).then_some(entry.offset),
            size,
            block_sizes,
            source,
        });
    }
    for (path, data) in &changes {
        if base.find(path).is_some() {
            continue;
        }
        let path_offset = path_table.len() as u64;
        path_table.extend(path.as_bytes());
        path_table.push(0);
        let (sizes, stored) = store(data)?;
        planned.push(Planned {
            digest: Md5::digest(path.as_bytes()).into(),
            path_offset,
            unknown_04: None,
            junk_offset: None,
            size: data.len() as u64,
            block_sizes: sizes,
            source: Source::New(stored),
        });
        stats.added += 1;
    }
    stats.files = planned.len();
    Ok(Plan {
        entries: planned,
        path_table,
        mirrors_first_block,
        common_04,
    })
}

/// Builds the header; returns it with the end offset of the file data.
fn header(base: &Archive, plan: &Plan) -> Result<(Vec<u8>, u64)> {
    // Header: fixed part, digests, records, path table, block table.
    let n = plan.entries.len();
    let blocks: usize = plan.entries.iter().map(|p| p.block_sizes.len()).sum();
    let header_len = 0x10 + n * 16 + n * 32 + 4 + plan.path_table.len() + blocks * 2;
    let too_large = |_| Error::Malformed("archive outgrows the format");
    let mut header = Vec::with_capacity(header_len);
    header.extend(MAGIC.to_le_bytes());
    header.extend(u32::try_from(header_len).map_err(too_large)?.to_le_bytes());
    header.extend(u32::try_from(n).map_err(too_large)?.to_le_bytes());
    header.extend(base.reserved.to_le_bytes());
    for p in &plan.entries {
        header.extend(p.digest);
    }
    let mut offset = header_len as u64;
    let mut first_block = 0u32;
    for p in &plan.entries {
        let stored: u64 = match &p.source {
            Source::Copy { len, .. } => *len,
            Source::New(bytes) => bytes.len() as u64,
        };
        header.extend(first_block.to_le_bytes());
        let unknown = p.unknown_04.unwrap_or(if plan.mirrors_first_block {
            first_block
        } else {
            plan.common_04
        });
        header.extend(unknown.to_le_bytes());
        header.extend(p.size.to_le_bytes());
        header.extend(p.junk_offset.unwrap_or(offset).to_le_bytes());
        header.extend(p.path_offset.to_le_bytes());
        offset += stored;
        first_block += u32::try_from(p.block_sizes.len()).map_err(too_large)?;
    }
    header.extend(
        u32::try_from(plan.path_table.len() + 4)
            .map_err(too_large)?
            .to_le_bytes(),
    );
    header.extend(&plan.path_table);
    for p in &plan.entries {
        for s in &p.block_sizes {
            header.extend(s.to_le_bytes());
        }
    }
    debug_assert_eq!(header.len(), header_len);
    Ok((header, offset))
}
