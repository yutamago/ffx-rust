//! The game's memory as a `snapshot` harness command left it
//! (`ffx_bridge::snapshot`): reads by address and byte searches.

use std::io;
use std::path::Path;

use ffx_bridge::snapshot::Snapshot;

/// One region, unpacked.
#[derive(Debug, Clone)]
pub struct Region {
    pub base: u32,
    pub bytes: Vec<u8>,
}

/// `FFX.exe`'s preferred base: the addresses of `symbols/ffx.sym`.
pub const IMAGE_BASE: u32 = 0x0040_0000;

/// A snapshot with every region unpacked, ascending by address.
#[derive(Debug, Clone)]
pub struct Mem {
    /// Where `FFX.exe` was loaded (the game can run rebased).
    pub image_base: u32,
    pub regions: Vec<Region>,
}

impl Default for Mem {
    fn default() -> Self {
        Self {
            image_base: IMAGE_BASE,
            regions: Vec::new(),
        }
    }
}

impl Mem {
    /// Reads and unpacks a snapshot file.
    ///
    /// # Errors
    /// When the file cannot be read or is not a snapshot.
    pub fn read(path: &Path) -> io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let snap = Snapshot::read(io::BufReader::new(file))?;
        Self::from_snapshot(&snap)
    }

    /// # Errors
    /// When a block is corrupt.
    pub fn from_snapshot(snap: &Snapshot) -> io::Result<Self> {
        let mut regions = Vec::with_capacity(snap.regions.len());
        for r in &snap.regions {
            regions.push(Region {
                base: r.base,
                bytes: r.bytes()?,
            });
        }
        regions.sort_by_key(|r| r.base);
        Ok(Self {
            image_base: snap.image_base,
            regions,
        })
    }

    /// Memory from `(base, bytes)` pieces (tests).
    #[must_use]
    pub fn from_pieces(pieces: Vec<(u32, Vec<u8>)>) -> Self {
        let mut regions: Vec<Region> = pieces
            .into_iter()
            .map(|(base, bytes)| Region { base, bytes })
            .collect();
        regions.sort_by_key(|r| r.base);
        Self {
            image_base: IMAGE_BASE,
            regions,
        }
    }

    /// An `FFX.exe` address (as in `symbols/ffx.sym`) where the image was
    /// loaded.
    #[must_use]
    pub fn image(&self, addr: u32) -> u32 {
        addr.wrapping_sub(IMAGE_BASE).wrapping_add(self.image_base)
    }

    /// A loaded address back to its `FFX.exe` address (identity outside
    /// the image's first 16 MB).
    #[must_use]
    pub fn unrebase(&self, addr: u32) -> u32 {
        let off = addr.wrapping_sub(self.image_base);
        if off < 0x0100_0000 {
            off + IMAGE_BASE
        } else {
            addr
        }
    }

    fn region(&self, addr: u32) -> Option<&Region> {
        let i = self.regions.partition_point(|r| r.base <= addr);
        let r = self.regions.get(i.checked_sub(1)?)?;
        ((addr - r.base) as usize).lt(&r.bytes.len()).then_some(r)
    }

    /// `len` bytes at `addr`, when all of them are in one region.
    #[must_use]
    pub fn bytes(&self, addr: u32, len: usize) -> Option<&[u8]> {
        let r = self.region(addr)?;
        let at = (addr - r.base) as usize;
        r.bytes.get(at..at.checked_add(len)?)
    }

    #[must_use]
    pub fn u8(&self, addr: u32) -> Option<u8> {
        self.bytes(addr, 1).map(|b| b[0])
    }

    #[must_use]
    pub fn u16(&self, addr: u32) -> Option<u16> {
        self.bytes(addr, 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    }

    #[must_use]
    pub fn u32(&self, addr: u32) -> Option<u32> {
        self.bytes(addr, 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    #[must_use]
    pub fn i32(&self, addr: u32) -> Option<i32> {
        self.u32(addr).map(u32::cast_signed)
    }

    #[must_use]
    pub fn f32(&self, addr: u32) -> Option<f32> {
        self.u32(addr).map(f32::from_bits)
    }

    /// The pointer at `addr`, when it is not null.
    #[must_use]
    pub fn ptr(&self, addr: u32) -> Option<u32> {
        self.u32(addr).filter(|&p| p != 0)
    }

    /// A NUL-terminated string at `addr` (at most `max` bytes).
    #[must_use]
    pub fn cstr(&self, addr: u32, max: usize) -> Option<String> {
        let r = self.region(addr)?;
        let at = (addr - r.base) as usize;
        let tail = &r.bytes[at..r.bytes.len().min(at + max)];
        let end = tail.iter().position(|&b| b == 0)?;
        Some(String::from_utf8_lossy(&tail[..end]).into_owned())
    }

    /// Every address where `needle` starts, in ascending order, at most
    /// `limit` of them. `align` restricts the start (1: any byte).
    #[must_use]
    pub fn find(&self, needle: &[u8], align: u32, limit: usize) -> Vec<u32> {
        let mut out = Vec::new();
        let Some(&first) = needle.first() else {
            return out;
        };
        for r in &self.regions {
            let hay = &r.bytes;
            let mut at = 0;
            while let Some(p) = hay[at..].iter().position(|&b| b == first) {
                let i = at + p;
                at = i + 1;
                let addr = r.base.wrapping_add(u32::try_from(i).unwrap_or(u32::MAX));
                if align > 1 && addr % align != 0 {
                    continue;
                }
                if hay[i..].starts_with(needle) {
                    out.push(addr);
                    if out.len() >= limit {
                        return out;
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_finds_across_regions() {
        let m = Mem::from_pieces(vec![
            (0x2000, vec![9, 8, 7, 6, b'h', b'i', 0]),
            (0x1000, vec![1, 0, 0, 0, 0xaa, 0xbb]),
        ]);
        assert_eq!(m.u32(0x1000), Some(1));
        assert_eq!(m.u16(0x1004), Some(0xbbaa));
        assert_eq!(m.u32(0x1004), None, "past the region's end");
        assert_eq!(m.cstr(0x2004, 16).as_deref(), Some("hi"));
        assert_eq!(m.find(&[0xaa, 0xbb], 1, 8), vec![0x1004]);
        assert_eq!(m.find(&[8, 7], 2, 8), Vec::<u32>::new(), "unaligned");
        assert_eq!(m.find(&[8, 7], 1, 8), vec![0x2001]);
        assert_eq!(m.ptr(0x0fff), None);
    }
}
