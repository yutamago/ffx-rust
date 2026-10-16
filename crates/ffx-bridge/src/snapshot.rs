//! Memory snapshots of the running game (`snapshot` harness command), for
//! running original and Rust code on the same state in `ffx-oracle`.
//!
//! File layout, little-endian:
//!
//! | Offset | Size | |
//! |---|---|---|
//! | 0 | 8 | magic `FFXSNAP1` |
//! | 8 | 4 | where `FFX.exe` was loaded |
//! | 12 | 4 | x87 control word of the game thread |
//! | 16 | 4 | number of regions |
//! | 20 | 4 | reserved, 0 |
//!
//! then per region: base, size, protection (`PAGE_*`), type (`MEM_*`) and
//! the length of what follows (`u32` each), then the region's bytes in
//! [`BLOCK`]-sized pieces, each a `u32` length and an LZ4 block (`lz4_flex`
//! with the size prepended). Pieces keep blocks small: `lz4_flex` sizes its
//! output buffer wrongly on 32-bit targets for inputs over about 39 MB.

use std::io::{self, Read, Write};

/// The magic.
pub const MAGIC: &[u8; 8] = b"FFXSNAP1";

/// Bytes per compressed piece.
pub const BLOCK: usize = 0x10_0000;

/// A region of the snapshot.
#[derive(Debug, Clone)]
pub struct Region {
    /// First address.
    pub base: u32,
    /// Length in bytes.
    pub size: u32,
    /// `PAGE_*` protection when it was taken.
    pub protect: u32,
    /// `MEM_PRIVATE`, `MEM_IMAGE` or `MEM_MAPPED`.
    pub kind: u32,
    /// The contents in compressed [`BLOCK`]-sized pieces.
    pub blocks: Vec<Vec<u8>>,
}

impl Region {
    /// The contents.
    ///
    /// # Errors
    /// When a block is corrupt.
    pub fn bytes(&self) -> io::Result<Vec<u8>> {
        let mut out = Vec::with_capacity(self.size as usize);
        for b in &self.blocks {
            out.extend(unpack(b)?);
        }
        Ok(out)
    }
}

/// One compressed block's bytes.
///
/// # Errors
/// When the block is corrupt.
pub fn unpack(block: &[u8]) -> io::Result<Vec<u8>> {
    lz4_flex::decompress_size_prepended(block)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// A whole snapshot.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    /// Where `FFX.exe` was loaded.
    pub image_base: u32,
    /// The game thread's x87 control word.
    pub fpu_control: u32,
    /// Committed, readable regions, ascending.
    pub regions: Vec<Region>,
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

impl Snapshot {
    /// Reads a snapshot file.
    ///
    /// # Errors
    /// On I/O errors or a malformed file.
    pub fn read(mut r: impl Read) -> io::Result<Self> {
        let mut head = [0u8; 24];
        r.read_exact(&mut head)?;
        if &head[..8] != MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "not an FFX snapshot",
            ));
        }
        let mut snap = Self {
            image_base: u32_at(&head, 8),
            fpu_control: u32_at(&head, 12),
            regions: Vec::new(),
        };
        for _ in 0..u32_at(&head, 16) {
            let mut h = [0u8; 20];
            r.read_exact(&mut h)?;
            let mut packed = vec![0u8; u32_at(&h, 16) as usize];
            r.read_exact(&mut packed)?;
            let mut blocks = Vec::new();
            let mut at = 0;
            while at + 4 <= packed.len() {
                let n = u32_at(&packed, at) as usize;
                let block = packed
                    .get(at + 4..at + 4 + n)
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "truncated block"))?;
                blocks.push(block.to_vec());
                at += 4 + n;
            }
            snap.regions.push(Region {
                base: u32_at(&h, 0),
                size: u32_at(&h, 4),
                protect: u32_at(&h, 8),
                kind: u32_at(&h, 12),
                blocks,
            });
        }
        Ok(snap)
    }

    /// Writes the header; regions follow with [`write_region`].
    ///
    /// # Errors
    /// On I/O errors.
    pub fn write_header(
        mut w: impl Write,
        image_base: u32,
        fpu_control: u32,
        regions: u32,
    ) -> io::Result<()> {
        w.write_all(MAGIC)?;
        for v in [image_base, fpu_control, regions, 0] {
            w.write_all(&v.to_le_bytes())?;
        }
        Ok(())
    }

    /// The region containing `addr`.
    #[must_use]
    pub fn region_at(&self, addr: u32) -> Option<&Region> {
        let i = self.regions.partition_point(|r| r.base <= addr);
        let r = self.regions.get(i.checked_sub(1)?)?;
        (u64::from(addr) < u64::from(r.base) + u64::from(r.size)).then_some(r)
    }
}

/// Writes one region (compressing it).
///
/// # Errors
/// On I/O errors.
pub fn write_region(
    mut w: impl Write,
    base: u32,
    protect: u32,
    kind: u32,
    bytes: &[u8],
) -> io::Result<()> {
    let mut packed = Vec::new();
    for piece in bytes.chunks(BLOCK) {
        let block = lz4_flex::compress_prepend_size(piece);
        let n = u32::try_from(block.len()).map_err(|_| io::Error::other("block over 4 GiB"))?;
        packed.extend_from_slice(&n.to_le_bytes());
        packed.extend_from_slice(&block);
    }
    let size = u32::try_from(bytes.len()).map_err(|_| io::Error::other("region over 4 GiB"))?;
    let len = u32::try_from(packed.len()).map_err(|_| io::Error::other("region over 4 GiB"))?;
    for v in [base, size, protect, kind, len] {
        w.write_all(&v.to_le_bytes())?;
    }
    w.write_all(&packed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut file = Vec::new();
        Snapshot::write_header(&mut file, 0x0040_0000, 0x027f, 3).unwrap();
        write_region(&mut file, 0x1000, 4, 0x2_0000, &[7; 0x1000]).unwrap();
        write_region(&mut file, 0x8000, 4, 0x2_0000, &[1, 2, 3]).unwrap();
        let big: Vec<u8> = (0..BLOCK * 2 + 5)
            .map(|i| u8::try_from(i % 251).unwrap())
            .collect();
        write_region(&mut file, 0x40_0000, 4, 0x2_0000, &big).unwrap();
        let s = Snapshot::read(file.as_slice()).unwrap();
        assert_eq!(s.fpu_control, 0x027f);
        assert_eq!(s.region_at(0x1fff).unwrap().base, 0x1000);
        assert!(s.region_at(0x2000).is_none());
        assert_eq!(s.region_at(0x8002).unwrap().bytes().unwrap(), [1, 2, 3]);
        assert_eq!(s.region_at(0x40_0000).unwrap().bytes().unwrap(), big);
    }
}
