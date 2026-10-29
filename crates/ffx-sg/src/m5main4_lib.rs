//! M5 stage 1, list `small`, round 4: library code the game links, from
//! the machine code: libwebm's `Block::GetTime` and `Block::GetFrame`
//! (the video player's demuxer) and the cloth heap's segment release
//! (dlmalloc's `mspace`).

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // registers
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    /// `_wassert(msg, file, line)`'s import slot (cdecl), the source file
    /// name and the messages.
    pub const WASSERT: u32 = 0x00b0_c5c0;
    pub const MKVPARSER_FILE: u32 = 0x00b9_02f0;
    pub const MSG_CLUSTER: u32 = 0x00b9_0694;
    pub const MSG_TC_POSITIVE: u32 = 0x00b9_2010;
    pub const MSG_INFO: u32 = 0x00b9_0ff0;
    pub const MSG_SCALE: u32 = 0x00b9_0ffc;
    pub const MSG_IDX_POSITIVE: u32 = 0x00b9_2024;
    pub const MSG_IDX_COUNT: u32 = 0x00b9_2038;
    pub const MSG_POS: u32 = 0x00b9_2060;
    pub const MSG_LEN: u32 = 0x00b9_2074;

    fn wassert(h: &mut impl Host, msg: u32, line: u32) {
        let f: u32 = h.read(h.rebase(WASSERT));
        let (m, file) = (h.rebase(msg), h.rebase(MKVPARSER_FILE));
        h.call(f, Conv::Cdecl, &[m, file, line]);
    }

    fn read_i64(h: &impl Host, p: u32) -> i64 {
        let lo: u32 = h.read(p);
        let hi: u32 = h.read(at(p, 4));
        (u64::from(hi) << 32 | u64::from(lo)).cast_signed()
    }

    /// `mkvparser::Cluster::Load(cluster; &pos, &len)` (thiscall,
    /// `ret 8`) -> a status (negative on error).
    pub const CLUSTER_LOAD: u32 = 0x0042_9e10;

    /// `libwebm_mkvparser_Block_GetTime(block; cluster)` (`0x0042df70`,
    /// thiscall, `ret 4`) -> `edx:eax`. `pos` is the original's 8-byte
    /// local (`ebp - 0x10`) and `len` its own `cluster` slot (`ebp + 8`),
    /// both handed to `Cluster::Load`.
    ///
    /// The cluster's time code (`+0x28`, 64-bit; `Load`'s status when
    /// that is negative) plus the block's (`+0x18`, `i16`), times the
    /// segment's (`[cluster] + 0x70`) info's time code scale (`+0x28`).
    /// `_wassert` reports a null cluster or info, a negative cluster time
    /// code and a scale below 1, and the code runs on (a null cluster or
    /// info then faults, as in the original).
    pub fn block_get_time(h: &mut impl Host, block: u32, cluster: u32, pos: u32, len: u32) -> u64 {
        let own = i64::from(h.read::<i16>(at(block, 0x18)));
        let tc = if cluster == 0 {
            wassert(h, MSG_CLUSTER, 0x2070);
            own
        } else {
            h.write(len, cluster);
            let status = h.call_sym(CLUSTER_LOAD, Conv::Thiscall(cluster), &[pos, len]);
            let tc0 = if status & 0x8000_0000 != 0 {
                i64::from(status.cast_signed())
            } else {
                read_i64(h, at(cluster, 0x28))
            };
            if tc0 < 0 {
                wassert(h, MSG_TC_POSITIVE, 0x2068);
            }
            own.wrapping_add(tc0)
        };
        let segment: u32 = h.read(cluster);
        let info: u32 = h.read(at(segment, 0x70));
        if info == 0 {
            wassert(h, MSG_INFO, 0x2076);
        }
        let scale = read_i64(h, at(info, 0x28));
        if scale < 1 {
            wassert(h, MSG_SCALE, 0x2079);
        }
        // __allmul: the low 64 bits of the product
        scale.wrapping_mul(tc).cast_unsigned()
    }

    /// `libwebm_mkvparser_Block_GetFrame(block; idx)` (`0x0042e0d0`,
    /// thiscall, `ret 4`) -> `&frames[idx]` (16 bytes each from `[+0x1c]`;
    /// count `+0x20`). `_wassert` reports `idx` out of range, a frame
    /// position not above 0 (64-bit) and a length not above 0.
    pub fn block_get_frame(h: &mut impl Host, block: u32, idx: u32) -> u32 {
        if idx.cast_signed() < 0 {
            wassert(h, MSG_IDX_POSITIVE, 0x2097);
        }
        if idx.cast_signed() >= h.read::<i32>(at(block, 0x20)) {
            wassert(h, MSG_IDX_COUNT, 0x2098);
        }
        let frames: u32 = h.read(at(block, 0x1c));
        let f = at(idx << 4, frames);
        if read_i64(h, f) <= 0 {
            wassert(h, MSG_POS, 0x209b);
        }
        if h.read::<i32>(at(f, 8)) <= 0 {
            wassert(h, MSG_LEN, 0x209c);
        }
        f
    }

    /// `VirtualQuery(p, &info, n)` and `VirtualFree(p, n, type)`'s import
    /// slots (stdcall).
    pub const VIRTUAL_QUERY: u32 = 0x00b0_c094;
    pub const VIRTUAL_FREE: u32 = 0x00b0_c098;

    /// `FUN_006ebda0(m)` (cdecl): every segment of the heap `m` (the list
    /// at `m + 0x1c8`: base, size, next, flags) that was mapped (flag 1)
    /// and isn't external (flag 8) handed back to the system, region by
    /// region (`VirtualQuery` into `info`, the original's 28-byte
    /// `MEMORY_BASIC_INFORMATION` at `ebp - 0x20`; a committed region
    /// starting at its own allocation and no larger than what is left is
    /// released with `VirtualFree(base, 0, MEM_RELEASE)`). Returns the
    /// bytes of the segments released whole.
    pub fn release_segments(h: &mut impl Host, m: u32, info: u32) -> u32 {
        let mut released = 0u32;
        let mut seg = at(m, 0x1c8);
        if seg == 0 {
            return 0;
        }
        let query: u32 = h.read(h.rebase(VIRTUAL_QUERY));
        let free: u32 = h.read(h.rebase(VIRTUAL_FREE));
        loop {
            let flags: u32 = h.read(at(seg, 0xc));
            let size: u32 = h.read(at(seg, 4));
            let mut base: u32 = h.read(seg);
            seg = h.read(at(seg, 8));
            if flags & 1 != 0 && flags & 8 == 0 {
                let mut left = size;
                let mut whole = true;
                while left != 0 {
                    if h.call(query, Conv::Stdcall, &[base, info, 0x1c]) == 0
                        || h.read::<u32>(info) != base
                        || h.read::<u32>(at(info, 4)) != base
                        || h.read::<u32>(at(info, 0x10)) != 0x1000
                        || h.read::<u32>(at(info, 0xc)) > left
                        || h.call(free, Conv::Stdcall, &[base, 0, 0x8000]) == 0
                    {
                        whole = false;
                        break;
                    }
                    let region: u32 = h.read(at(info, 0xc));
                    base = base.wrapping_add(region);
                    left = left.wrapping_sub(region);
                }
                if whole {
                    released = released.wrapping_add(size);
                }
            }
            if seg == 0 {
                return released;
            }
        }
    }
}
