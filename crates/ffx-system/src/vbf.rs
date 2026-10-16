//! The archive index's digest order and lookup (`Md5Digest_Less`,
//! `Vbf_MapLookup`; `docs/formats/vbf.md`).
//!
//! The game orders MD5 digests not as bytes but as two little-endian
//! signed 64-bit numbers, the first half first.

use std::cmp::Ordering;

/// A digest as the game compares it: four `u32`s (the digest's bytes, little
/// endian).
pub type Digest = [u32; 4];

/// The order of two digests: the 64-bit halves compared as signed numbers.
#[must_use]
pub fn cmp(a: &Digest, b: &Digest) -> Ordering {
    let half = |d: &Digest, i: usize| (u64::from(d[i + 1]) << 32 | u64::from(d[i])).cast_signed();
    half(a, 0)
        .cmp(&half(b, 0))
        .then(half(a, 2).cmp(&half(b, 2)))
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use std::cmp::Ordering;

    use ffx_bridge::Mem;

    /// `Md5Digest_Less(b)` (`0x0061d6e0`, `thiscall(a, b)`): `a < b`.
    pub fn less(mem: &impl Mem, a: u32, b: u32) -> bool {
        super::cmp(&mem.read(a), &mem.read(b)) == Ordering::Less
    }

    /// `Vbf_MapLookup(digest)` (`0x0061d140`, `thiscall(map, digest)`): the
    /// node holding `digest` in the archive's search tree, or 0. The map is
    /// its own end marker; its root is at `+0xc`; a node has its left child
    /// at `+0`, right at `+4`, key at `+0x10`.
    pub fn map_lookup(mem: &impl Mem, map: u32, digest: u32) -> u32 {
        let d: super::Digest = mem.read(digest);
        let mut node: u32 = mem.read(map.wrapping_add(0xc));
        while node != map {
            let key: super::Digest = mem.read(node.wrapping_add(0x10));
            node = match super::cmp(&key, &d) {
                Ordering::Less => mem.read(node.wrapping_add(4)),
                Ordering::Greater => mem.read(node),
                Ordering::Equal => return node,
            };
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halves_are_signed() {
        let neg = [0, 0x8000_0000, 0, 0];
        let pos = [0, 0x7fff_ffff, 0, 0];
        assert_eq!(cmp(&neg, &pos), Ordering::Less);
        assert_eq!(cmp(&[1, 0, 5, 0], &[1, 0, 4, 0]), Ordering::Greater);
        assert_eq!(cmp(&[2, 0, 0, 0], &[1, 0, 9, 0]), Ordering::Greater);
    }
}
