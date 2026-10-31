//! M5 stage 1, list `map`, round 5 batch 7: the guide map save
//! (`rcuiGuideSave_009397f0`) and its per-map record
//! (`rcuiGuideSaveMap_00938d20`): a chunk header, the maps (their header,
//! transfers, indices and image) and the guide's own words, padded to 16.
//!
//! The zero padding and headers live in the originals' frames and come in
//! through their `ebp` (the shims pass a `StackFrame`).

/// The frames: `ebp` minus the lowest local.
pub const MAP_FRAME: u32 = 0x14;
pub const SAVE_FRAME: u32 = 0x30;

// ---- callees ----

/// `sceWrite(fd, buf, n)` -> 0, `rcbgTrxSave(t, fd)`,
/// `rcuiGuideSaveSize(g)`, `rcPad16(n)`, `rcbgChunkTag(i)`,
/// `crt_memset_thunk(p, v, n)` (cdecl).
pub const SCE_WRITE: u32 = 0x0062_fdc0;
pub const TRX_SAVE: u32 = 0x0093_9950;
pub const GUIDE_SAVE_SIZE: u32 = 0x0093_96a0;
pub const PAD16: u32 = 0x0092_b2c0;
pub const CHUNK_TAG: u32 = 0x0092_1b90;
pub const MEMSET: u32 = 0x0094_964c;
/// `rcuiGuideSaveMap_00938d20(m, fd)` (cdecl; ported here, called as the
/// original is).
pub const SAVE_MAP: u32 = 0x0093_8d20;

pub mod bridge {
    use super::{CHUNK_TAG, GUIDE_SAVE_SIZE, MEMSET, PAD16, SAVE_MAP, SCE_WRITE, TRX_SAVE};
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn write(h: &mut impl Host, fd: u32, buf: u32, n: u32) -> u32 {
        cdecl(h, SCE_WRITE, &[fd, buf, n])
    }

    /// `rcuiGuideSaveMap_00938d20(m, fd)` -> the last `sceWrite`'s `eax`:
    /// the map's 0x90-byte header, its transfer count (`+0x98`), 12 zero
    /// bytes; with transfers, the `+0x9c` indices padded to 16 and the
    /// `+0xa0` transfers (`rcbgTrxSave`); the two bytes `+0xba`/`+0xbb`, 14
    /// zero bytes; the image (`+0xbc`): three 8-byte fields `+0x18..`, its
    /// width and height and their product's bytes from `+0x38`.
    pub fn save_map(h: &mut impl Host, ebp: u32, m: u32, fd: u32) -> u32 {
        let zero = ebp - 0x14;
        for k in 0..4 {
            h.write(zero + 4 * k, 0_u32);
        }
        write(h, fd, m, 0x90);
        let count_at = m + 0x98;
        write(h, fd, count_at, 4);
        write(h, fd, zero, 0xc);
        let n = h.read::<u32>(count_at);
        if n.cast_signed() > 0 {
            let indices: u32 = h.read(m + 0x9c);
            write(h, fd, indices, n << 2);
            let tail = h.read::<u32>(count_at).wrapping_shl(2) & 0xf;
            if tail != 0 {
                write(h, fd, zero, 0x10 - tail);
            }
            let trx: u32 = h.read(m + 0xa0);
            cdecl(h, TRX_SAVE, &[trx, fd]);
        }
        write(h, fd, m + 0xba, 1);
        write(h, fd, m + 0xbb, 1);
        write(h, fd, zero, 0xe);
        let image: u32 = h.read(m + 0xbc);
        for off in [0x18, 0x20, 0x28] {
            write(h, fd, image + off, 8);
        }
        write(h, fd, image + 0x30, 4);
        write(h, fd, image + 0x34, 4);
        let size = h
            .read::<u32>(image + 0x34)
            .wrapping_mul(h.read(image + 0x30));
        let pixels: u32 = h.read(image + 0x38);
        write(h, fd, pixels, size)
    }

    /// `rcuiGuideSave_009397f0(g, fd)` -> the last `sceWrite`'s `eax`: a
    /// 16-byte chunk header (`rcbgChunkTag(2)`'s tag, the save's size
    /// (`rcuiGuideSaveSize`) padded to 16 in 16-byte units), the map count
    /// (`+4`), 12 zero bytes, each map (`+8`, 0xc0 bytes apart, the count
    /// read each time) through `rcuiGuideSaveMap`, the words `+0x68`, `+0x78`,
    /// `+0x88` (16 bytes each), `+0xa8` and `+0x64`, then the padding as
    /// zero bytes (over the header).
    pub fn save(h: &mut impl Host, ebp: u32, g: u32, fd: u32) -> u32 {
        h.write(ebp - 0x28, g);
        let zero = ebp - 0x24;
        for k in 0..4 {
            h.write(zero + 4 * k, 0_u32);
        }
        let size = cdecl(h, GUIDE_SAVE_SIZE, &[g]);
        let pad = cdecl(h, PAD16, &[size]);
        h.write(ebp - 0x2c, pad);
        let header = ebp - 0x14;
        for k in 0..4 {
            h.write(header + 4 * k, 0_u32);
        }
        let tag_at = cdecl(h, CHUNK_TAG, &[2]);
        let tag: u32 = h.read(tag_at);
        h.write(header, tag);
        let total = pad.wrapping_add(size).cast_signed();
        let units = total.wrapping_add((total >> 31) & 0xf) >> 4;
        h.write(header + 4, units.cast_unsigned());
        write(h, fd, header, 0x10);
        let count_at = g + 4;
        h.write(ebp - 0x30, count_at);
        write(h, fd, count_at, 4);
        write(h, fd, zero, 0xc);
        let mut i = 0_i32;
        let mut at = 0_u32;
        while i < h.read::<i32>(count_at) {
            let maps: u32 = h.read(g + 8);
            cdecl(h, SAVE_MAP, &[maps.wrapping_add(at), fd]);
            i += 1;
            at = at.wrapping_add(0xc0);
        }
        for (off, n) in [
            (0x68, 0x10),
            (0x78, 0x10),
            (0x88, 0x10),
            (0xa8, 4),
            (0x64, 4),
        ] {
            write(h, fd, g + off, n);
        }
        cdecl(h, MEMSET, &[header, 0, pad]);
        write(h, fd, header, pad)
    }
}
