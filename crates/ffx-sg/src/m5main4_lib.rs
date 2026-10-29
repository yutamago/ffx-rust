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

    /// `FUN_006eb950(m, nb)` (cdecl): dlmalloc's `tmalloc_small` for the
    /// heap `m` (`+0` the small map, `+4` the tree map, `+8` the
    /// designated victim's size, `+0x10` the least address, `+0x14` the
    /// victim, small bins from `+0x28`, tree bins from `+0x130`): the
    /// best fit for `nb` in the smallest tree bin, unlinked from its tree,
    /// split (a remainder of 16 bytes or more becomes the new victim, the
    /// old one going to its small bin) or used whole. Checks against the
    /// least address skip the links that would fail them, as the original
    /// does. Returns the chunk's memory, or 0 for a chunk below the least
    /// address. The tree map must not be 0 (the original's `bsf` leaves
    /// an undefined index then; its caller never calls it so).
    pub fn tmalloc_small<H: Host>(h: &mut H, m: u32, nb: u32) -> u32 {
        let map: u32 = h.read(at(m, 4));
        let i = (map & map.wrapping_neg()).trailing_zeros();
        let mut t: u32 = h.read(at(m, 0x130 + 4 * i));
        let size = |h: &mut H, c: u32| -> u32 { h.read::<u32>(at(c, 4)) & !7 };
        let mut rsize = size(h, t).wrapping_sub(nb);
        let mut v = t;
        loop {
            let left: u32 = h.read(at(t, 0x10));
            t = if left != 0 { left } else { h.read(at(t, 0x14)) };
            if t == 0 {
                break;
            }
            let trem = size(h, t).wrapping_sub(nb);
            if trem < rsize {
                rsize = trem;
                v = t;
            }
        }
        let least: u32 = h.read(at(m, 0x10));
        let r = v.wrapping_add(nb);
        if v < least || v >= r {
            return 0;
        }
        unlink_large_chunk(h, m, v);
        if rsize < 0x10 {
            h.write(at(v, 4), rsize.wrapping_add(nb) | 3);
            let next_head = v.wrapping_add(rsize).wrapping_add(nb).wrapping_add(4);
            let head: u32 = h.read(next_head);
            h.write(next_head, head | 1);
            return at(v, 8);
        }
        h.write(at(v, 4), nb | 3);
        h.write(at(r, 4), rsize | 1);
        h.write(at(r, rsize), rsize);
        let dvs: u32 = h.read(at(m, 8));
        if dvs != 0 {
            let dv: u32 = h.read(at(m, 0x14));
            let idx = dvs >> 3;
            let bin = at(m, (idx.wrapping_add(5)) << 3);
            let bit = 1u32 << (idx & 31);
            let small: u32 = h.read(m);
            let mut f = bin;
            if small & bit == 0 {
                h.write(m, small | bit);
            } else {
                let fd: u32 = h.read(at(bin, 8));
                if fd >= h.read::<u32>(at(m, 0x10)) {
                    f = fd;
                }
            }
            h.write(at(bin, 8), dv);
            h.write(at(f, 0xc), dv);
            h.write(at(dv, 8), f);
            h.write(at(dv, 0xc), bin);
        }
        h.write(at(m, 8), rsize);
        h.write(at(m, 0x14), r);
        at(v, 8)
    }

    /// dlmalloc's `unlink_large_chunk(m, v)` as `tmalloc_small` inlines it
    /// (tree chunk: `+8/+0xc` the list links, `+0x10/+0x14` the children,
    /// `+0x18` the parent, `+0x1c` the bin index).
    fn unlink_large_chunk(h: &mut impl Host, m: u32, v: u32) {
        let parent: u32 = h.read(at(v, 0x18));
        let mut r: u32 = h.read(at(v, 0xc));
        if r == v {
            let mut rp = at(v, 0x14);
            r = h.read(rp);
            if r == 0 {
                rp = at(v, 0x10);
                r = h.read(rp);
            }
            if r != 0 {
                loop {
                    let mut cp = at(r, 0x14);
                    let mut c: u32 = h.read(cp);
                    if c == 0 {
                        cp = at(r, 0x10);
                        c = h.read(cp);
                        if c == 0 {
                            break;
                        }
                    }
                    rp = cp;
                    r = c;
                }
                if rp >= h.read::<u32>(at(m, 0x10)) {
                    h.write(rp, 0u32);
                }
            }
        } else {
            let f: u32 = h.read(at(v, 8));
            if f >= h.read::<u32>(at(m, 0x10)) {
                h.write(at(f, 0xc), r);
                h.write(at(r, 8), f);
            }
        }
        if parent == 0 {
            return;
        }
        let index: u32 = h.read(at(v, 0x1c));
        let bin_at = at(m, 0x130u32.wrapping_add(index.wrapping_mul(4)));
        if v == h.read::<u32>(bin_at) {
            h.write(bin_at, r);
            if r == 0 {
                let map: u32 = h.read(at(m, 4));
                h.write(at(m, 4), map & !(1u32 << (index & 31)));
                return;
            }
        } else {
            if parent >= h.read::<u32>(at(m, 0x10)) {
                let child = if h.read::<u32>(at(parent, 0x10)) == v {
                    0x10
                } else {
                    0x14
                };
                h.write(at(parent, child), r);
            }
            if r == 0 {
                return;
            }
        }
        if r < h.read::<u32>(at(m, 0x10)) {
            return;
        }
        h.write(at(r, 0x18), parent);
        for o in [0x10, 0x14] {
            let c: u32 = h.read(at(v, o));
            if c != 0 && c >= h.read::<u32>(at(m, 0x10)) {
                h.write(at(r, o), c);
                h.write(at(c, 0x18), r);
            }
        }
    }
}
