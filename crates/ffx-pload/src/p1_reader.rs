//! List `p1-load`, continuation round, batch 13: the `PBinary` cluster
//! reader's instance-list headers.
//!
//! `PClusterReaderBinary` (the reader), as these functions use it:
//! - `+0`: the cluster being loaded;
//! - `+4`: the namespace mapping: `+0xc` its class count (bit 31 not
//!   owned), `+0x10` its 32-byte class entries (`+4` the packed class,
//!   `+8` the class descriptor or 0, `+0x1c`), `+0x1c` flags;
//! - `+8`: the cluster header (`+0x10` the instance-list count);
//! - `+0xc`: whether the file's words need swapping;
//! - `+0x14`: the instance-list headers, nine words each (`+0` the class
//!   index from 1, `+4` the count, `+8` the alignment, `+0xc` the size,
//!   `+0x10` the extra bytes, `+0x14` the pointer count);
//! - `+0x18`: the instance lists made (a pointer array with one inline
//!   slot);
//! - `+0x24`: the class entries of each header.

use ffx_graphic::host::{Conv, Host};

/// The ports over game memory.
pub mod bridge {
    use super::{Conv, Host};

    /// The class entry of header class index `index` (from 1), or 0
    /// past the mapping's count.
    fn class_entry(h: &impl Host, reader: u32, index: u32) -> Option<u32> {
        let map: u32 = h.read(reader + 4);
        let count: u32 = h.read(map + 0xc);
        let i = index.wrapping_sub(1);
        if i < count & 0x7fff_ffff {
            let table: u32 = h.read(map + 0x10);
            Some((i << 5).wrapping_add(table))
        } else {
            None
        }
    }

    /// `PClusterReaderBinary::initializeInstanceLists(reader)`
    /// (`0x0057b3a0`, thiscall): for each instance-list header, its
    /// class's list in the cluster. A class with no descriptor gets
    /// arena room only (`0x00440cf0(size, 1 << (class +4 >> 28))`, when
    /// the mapping's flags have none of `0x80000005`), and is an error
    /// (6) when its packed class has flags bit 2. Else the list
    /// (`allocateInstanceListForSize`, `0x00440ba0`, for the simple
    /// mapping; `allocateInstanceList`, `0x00440b70`, with the size of
    /// its data, pointers and over-allocation, otherwise), 0xd when none,
    /// marked not in its cluster (`+0x49`) and stored. 0.
    pub fn init_instance_lists(h: &mut impl Host, reader: u32) -> u32 {
        let made: u32 = h.read(reader + 0x18);
        let mut slot = match made & 0x7fff_ffff {
            0 => 0,
            1 => reader + 0x1c,
            _ => h.read(reader + 0x1c),
        };
        let header: u32 = h.read(reader + 8);
        let n: u32 = h.read(header + 0x10);
        let end = slot.wrapping_add(n.wrapping_mul(4));
        let map: u32 = h.read(reader + 4);
        let simple = h.read::<u32>(map + 0x1c) & 0x8000_0005 == 0;
        let mut hdr: u32 = h.read(reader + 0x14);
        let cluster: u32 = h.read(reader);
        while slot < end {
            let entry = class_entry(h, reader, h.read(hdr)).unwrap_or(0);
            let desc: u32 = h.read(entry + 8);
            if desc == 0 {
                let packed: u32 = h.read(entry + 4);
                if simple {
                    let shift: u32 = h.read(packed + 4);
                    let size: u32 = h.read(hdr + 8);
                    h.call_sym(
                        0x0044_0cf0,
                        Conv::Thiscall(cluster),
                        &[size, 1u32 << (shift >> 28)],
                    );
                }
                let flags: u8 = h.read(packed + 0x1c);
                if flags & 4 != 0 {
                    return 6;
                }
            } else {
                let [count, _, size, extra, pointers]: [u32; 5] = h.read(hdr + 4);
                let flags: u8 = h.read(desc + 0x90);
                let over = if flags & 4 == 0 {
                    0
                } else {
                    let elem: u32 = h.read(desc + 0x1c);
                    size.wrapping_sub(elem.wrapping_mul(count))
                };
                let list = if simple {
                    h.call_sym(
                        0x0044_0ba0,
                        Conv::Thiscall(cluster),
                        &[
                            size.wrapping_sub(over),
                            count,
                            desc,
                            extra.wrapping_add(over),
                        ],
                    )
                } else {
                    let tail: u32 = h.read(entry + 0x1c);
                    let data = if tail == 0 {
                        extra
                    } else {
                        extra.wrapping_add(0x10)
                    };
                    let mut total = (data.wrapping_add(3) & !3)
                        .wrapping_add(over)
                        .wrapping_add(pointers.wrapping_mul(4));
                    let [per, align]: [u32; 2] = h.read(desc + 0x74);
                    if per != 0 {
                        if align != 0 {
                            total =
                                total.wrapping_sub(1).wrapping_add(align) & !align.wrapping_sub(1);
                        }
                        total = total.wrapping_add(count.wrapping_mul(per));
                    }
                    h.call_sym(0x0044_0b70, Conv::Thiscall(cluster), &[count, desc, total])
                };
                if list == 0 {
                    return 0xd;
                }
                h.write(list + 0x49, 0u8);
                h.write(slot, list);
            }
            slot = slot.wrapping_add(4);
            hdr = hdr.wrapping_add(0x24);
        }
        0
    }

    /// `0x0057c4a0` (thiscall `(stream, &align, &size, &max_align)` on the
    /// reader): reads the instance-list headers (`readExact`,
    /// `0x0057dd30`, 0x24 bytes each), swaps each through the swapper
    /// object `0x00c97338` (slot 0, `(p, 0)`; stride `*0x00c97354`) when
    /// the file needs it; the read's error, else for each header its
    /// class entry (6 when there is none) into `+0x24`'s array, the
    /// largest alignment so far, and for classes with over-allocation
    /// (`+0x74` per instance, `+0x78` alignment) the size grown. 0.
    pub fn read_list_headers(
        h: &mut impl Host,
        reader: u32,
        stream: u32,
        align: u32,
        size: u32,
        max_align: u32,
    ) -> u32 {
        let header: u32 = h.read(reader + 8);
        let n: u32 = h.read(header + 0x10);
        let first: u32 = h.read(reader + 0x14);
        let mut out: u32 = h.read(reader + 0x24);
        let bytes = n.wrapping_mul(0x24);
        let end = first.wrapping_add(bytes);
        let swap: u8 = h.read(reader + 0xc);
        let r = if n != 0 && first != 0 {
            h.call_sym(0x0057_dd30, Conv::Thiscall(stream), &[first, bytes])
        } else {
            0
        };
        if first != 0 && swap != 0 && n != 0 {
            let swapper = h.rebase(0x00c9_7338);
            let stride: u32 = h.read(h.rebase(0x00c9_7354));
            let mut p = first;
            for _ in 0..n {
                h.call_virtual(swapper, 0, &[p, 0]);
                p = p.wrapping_add(stride);
            }
        }
        if r != 0 {
            return r;
        }
        let mut hdr = first;
        while hdr < end {
            let index: u32 = h.read(hdr);
            let entry = match class_entry(h, reader, index) {
                Some(e) if e != 0 => e,
                _ => return 6,
            };
            h.write(out, entry);
            let want: u32 = h.read(hdr + 8);
            let a: u32 = h.read(align);
            h.write(align, a.max(want));
            let desc: u32 = h.read(entry + 8);
            if desc != 0 {
                let [per, al]: [u32; 2] = h.read(desc + 0x74);
                if per != 0 {
                    if al != 0 {
                        let s: u32 = h.read(size);
                        h.write(
                            size,
                            s.wrapping_sub(1).wrapping_add(al) & !al.wrapping_sub(1),
                        );
                        let m: u32 = h.read(max_align);
                        h.write(max_align, m.max(al));
                    }
                    let count: u32 = h.read(hdr + 4);
                    let s: u32 = h.read(size);
                    h.write(size, s.wrapping_add(count.wrapping_mul(per)));
                }
            }
            hdr = hdr.wrapping_add(0x24);
            out = out.wrapping_add(4);
        }
        0
    }
}
