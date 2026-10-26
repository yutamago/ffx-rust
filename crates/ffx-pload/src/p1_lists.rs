//! List `p1-load`, continuation round 3, batch 4: the cluster reader's
//! instance-list data (`0x0057c940`).
//!
//! The reader as [`crate::p1_reader`] describes it: `+0` the cluster
//! (`+0x24` its arena, `+4` the arena's block), `+4` the namespace
//! mapping (`+0xc` its class count, `+0x10` its 32-byte class entries,
//! `+0x1c` flags), `+8` the cluster header (`+0x10` the list count,
//! `+0x38` the data size), `+0x14` the instance-list headers (nine
//! words: `+0` the class index from 1, `+8` the data size), `+0x18` the
//! lists made (a pointer `PSharray`).

use ffx_graphic::host::{Conv, Host};

/// `PStreamReader::readExact(buffer, size)` (thiscall on the stream).
const READ_EXACT: u32 = 0x0057_dd30;
/// The stream's skip `(n)` (thiscall).
const SKIP: u32 = 0x0057_e160;
/// `readListData(list, src, &header)` (thiscall on the reader).
const READ_LIST_DATA: u32 = 0x0057_b270;
/// `placeOverallocation(&cursor, list)` (cdecl).
const PLACE_OVERALLOCATION: u32 = 0x0057_8300;
/// `PhyreMalloc(size, align)` and `PhyreFree(p)` (cdecl).
const MALLOC: u32 = 0x0042_fb80;
const FREE: u32 = 0x0042_fb20;

/// The ports over game memory.
pub mod bridge {
    use super::{Conv, FREE, Host, MALLOC, PLACE_OVERALLOCATION, READ_EXACT, READ_LIST_DATA, SKIP};

    /// `PClusterReaderBinary`'s instance-list data `(stream, cursor,
    /// size)` (`0x0057c940`, thiscall, `ret 0xc`): for each instance list
    /// (with its header), its data read from `stream` and fixed
    /// (`readListData`), then its over-allocation placed at the cursor
    /// (`placeOverallocation(&cursor, list)`; `cursor` is the address of
    /// the original's argument word). With the simple mapping (flags
    /// `+0x1c` 0) the whole data is read first into the cluster arena's
    /// block (`+0x24`'s `+4`, the header's `+0x38` bytes) and each list
    /// fixed from its own buffer (`+0x34`). Otherwise each list's data is
    /// read in turn, into its own buffer, or into a temporary block of
    /// `size` bytes (`PhyreMalloc(size, 16)`, freed at the end unless
    /// `size` has bit 31) when its class entry has `+0x1c`; a missing
    /// list's data is skipped. The first error is returned, else 0.
    pub fn read_instance_lists(
        h: &mut impl Host,
        reader: u32,
        stream: u32,
        cursor: u32,
        size: u32,
    ) -> u32 {
        let map: u32 = h.read(reader + 4);
        let mut hdr: u32 = h.read(reader + 0x14);
        let simple = h.read::<u32>(map + 0x1c) == 0;
        let made: u32 = h.read(reader + 0x18);
        let mut slot = match made & 0x7fff_ffff {
            0 => 0,
            1 => reader + 0x1c,
            _ => h.read(reader + 0x1c),
        };
        let header: u32 = h.read(reader + 8);
        let n: u32 = h.read(header + 0x10);
        let end = slot.wrapping_add(n.wrapping_mul(4));
        if simple {
            let cluster: u32 = h.read(reader);
            let arena: u32 = h.read(cluster + 0x24);
            let block = if arena == 0 { 0 } else { h.read(arena + 4) };
            let total: u32 = h.read(header + 0x38);
            let r = h.call_sym(READ_EXACT, Conv::Thiscall(stream), &[block, total]);
            if r != 0 {
                return r;
            }
            while slot < end {
                let list: u32 = h.read(slot);
                if list != 0 {
                    let buf: u32 = h.read(list + 0x34);
                    let r = h.call_sym(READ_LIST_DATA, Conv::Thiscall(reader), &[list, buf, hdr]);
                    if r != 0 {
                        return r;
                    }
                    h.call_sym(PLACE_OVERALLOCATION, Conv::Cdecl, &[cursor, list]);
                }
                slot = slot.wrapping_add(4);
                hdr = hdr.wrapping_add(0x24);
            }
            return 0;
        }
        let block = if size == 0 {
            0
        } else {
            h.call_sym(MALLOC, Conv::Cdecl, &[size, 0x10])
        };
        // freed unless the block was made and `size` has bit 31
        let keep = block != 0 && size >> 31 != 0;
        let mut result = 0;
        while slot < end {
            let map: u32 = h.read(reader + 4);
            let index = h.read::<u32>(hdr).wrapping_sub(1);
            let count = h.read::<u32>(map + 0xc) & 0x7fff_ffff;
            let entry = if index < count {
                (index << 5).wrapping_add(h.read(map + 0x10))
            } else {
                0
            };
            let list: u32 = h.read(slot);
            let bytes: u32 = h.read(hdr + 8);
            if list == 0 {
                h.call_sym(SKIP, Conv::Thiscall(stream), &[bytes]);
            } else {
                let mut buf: u32 = h.read(list + 0x34);
                if h.read::<u32>(entry.wrapping_add(0x1c)) != 0 {
                    buf = block;
                }
                let mut r = h.call_sym(READ_EXACT, Conv::Thiscall(stream), &[buf, bytes]);
                if r == 0 {
                    r = h.call_sym(READ_LIST_DATA, Conv::Thiscall(reader), &[list, buf, hdr]);
                }
                if r != 0 {
                    result = r;
                    break;
                }
                h.call_sym(PLACE_OVERALLOCATION, Conv::Cdecl, &[cursor, list]);
            }
            slot = slot.wrapping_add(4);
            hdr = hdr.wrapping_add(0x24);
        }
        if !keep && block != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[block]);
        }
        result
    }
}
