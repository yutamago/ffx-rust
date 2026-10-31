//! M5 stage 1, list `small`, round 4: library code the game links, from
//! the machine code: libwebm's `Block::GetTime` and `Block::GetFrame`
//! (the video player's demuxer) and the cloth heap's segment release
//! (dlmalloc's `mspace`).

/// The originals over game memory.
#[allow(clippy::many_single_char_names, clippy::verbose_bit_mask)] // registers; the original's tests
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

    /// `GetCurrentThreadId()`, `SleepEx(ms, alertable)` and
    /// `InterlockedCompareExchange(p, new, old)`'s import slots (stdcall).
    pub const GET_CURRENT_THREAD_ID: u32 = 0x00b0_c108;
    pub const SLEEP_EX: u32 = 0x00b0_c0a8;
    pub const INTERLOCKED_COMPARE_EXCHANGE: u32 = 0x00b0_c12c;
    /// dlmalloc's `tmalloc_small` (`0x006eb950`), `tmalloc_large`
    /// (`0x006eb540`), `sys_alloc` (`0x006eb1d0`) (cdecl, `(m, nb)`) and
    /// `memset`'s thunk.
    pub const TMALLOC_SMALL: u32 = 0x006e_b950;
    pub const TMALLOC_LARGE: u32 = 0x006e_b540;
    pub const SYS_ALLOC: u32 = 0x006e_b1d0;
    pub const MEMSET: u32 = 0x0094_964c;
    pub const MSPACE_MALLOC_SYM: u32 = 0x006e_c5c0;

    fn stdcall_import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    /// dlmalloc's `PREACTION` on the heap `m` (flag 2 of `+0x1b8`: the
    /// recursive spin lock at `+0x1bc`: word, count `+4`, owner thread
    /// `+8`). The original takes the word with `xchg`; the port with
    /// `InterlockedCompareExchange(lock, 1, 0)`, the same atomic step (a
    /// held lock stays 1 either way). Every 64th failed try sleeps
    /// (`SleepEx(0, 0)`). `spins` is the original's counter word (its own
    /// argument slot).
    fn lock(h: &mut impl Host, m: u32, spins: u32) {
        if h.read::<u8>(at(m, 0x1b8)) & 2 == 0 {
            return;
        }
        let l = at(m, 0x1bc);
        h.write(spins, 0u32);
        loop {
            if h.read::<u32>(l) == 0 {
                if stdcall_import(h, INTERLOCKED_COMPARE_EXCHANGE, &[l, 1, 0]) == 0 {
                    let id = stdcall_import(h, GET_CURRENT_THREAD_ID, &[]);
                    h.write(at(l, 8), id);
                    h.write(at(l, 4), 1u32);
                    return;
                }
            } else if h.read::<u32>(at(l, 8)) == stdcall_import(h, GET_CURRENT_THREAD_ID, &[]) {
                let n: u32 = h.read(at(l, 4));
                h.write(at(l, 4), n.wrapping_add(1));
                return;
            }
            let n = h.read::<u32>(spins).wrapping_add(1);
            h.write(spins, n);
            if n & 0x3f == 0 {
                stdcall_import(h, SLEEP_EX, &[0, 0]);
            }
        }
    }

    /// dlmalloc's `POSTACTION`: the count down, the lock released at 0.
    fn unlock(h: &mut impl Host, m: u32) {
        if h.read::<u8>(at(m, 0x1b8)) & 2 == 0 {
            return;
        }
        let l = at(m, 0x1bc);
        let n = h.read::<u32>(at(l, 4)).wrapping_sub(1);
        h.write(at(l, 4), n);
        if n == 0 {
            h.write(at(l, 8), 0u32);
            h.write(l, 0u32);
        }
    }

    /// Small bin `i`'s first chunk taken off its list (the bin's map bit
    /// cleared when it empties; a forward link below the least address
    /// is left alone). Returns the chunk.
    fn unlink_first_small(h: &mut impl Host, m: u32, i: u32) -> u32 {
        let b = at(m, 0x28 + i.wrapping_mul(8));
        let p: u32 = h.read(at(b, 8));
        let f: u32 = h.read(at(p, 8));
        if b == f {
            let map: u32 = h.read(m);
            h.write(m, map & !(1u32 << (i & 31)));
        } else if f >= h.read::<u32>(at(m, 0x10)) {
            h.write(at(b, 8), f);
            h.write(at(f, 0xc), b);
        }
        p
    }

    /// The designated victim `+0x14` (size `+8`) put in its small bin
    /// before `r` (size `rsize`) replaces it.
    fn replace_dv(h: &mut impl Host, m: u32, r: u32, rsize: u32) {
        let dvs: u32 = h.read(at(m, 8));
        if dvs != 0 {
            let dv: u32 = h.read(at(m, 0x14));
            let i = dvs >> 3;
            let b = at(m, 0x28 + i.wrapping_mul(8));
            let bit = 1u32 << (i & 31);
            let map: u32 = h.read(m);
            let mut f = b;
            if map & bit == 0 {
                h.write(m, map | bit);
            } else {
                let fd: u32 = h.read(at(b, 8));
                if fd >= h.read::<u32>(at(m, 0x10)) {
                    f = fd;
                }
            }
            h.write(at(b, 8), dv);
            h.write(at(f, 0xc), dv);
            h.write(at(dv, 8), f);
            h.write(at(dv, 0xc), b);
        }
        h.write(at(m, 8), rsize);
        h.write(at(m, 0x14), r);
    }

    /// `FUN_006ec5c0(m, bytes)` (cdecl): dlmalloc's `mspace_malloc` (the
    /// small bins, the designated victim, the tree bins through
    /// `tmalloc_small`/`tmalloc_large`, the top chunk, `sys_alloc`) under
    /// the heap's lock; the block's usable bytes (the chunk less 4, or 8
    /// for a mapped chunk) filled with 0xcd. `slot` is the caller's `m`
    /// slot, which the original uses as its spin counter and then for
    /// the result. Returns the block.
    pub fn mspace_malloc(h: &mut impl Host, m: u32, bytes: u32, slot: u32) -> u32 {
        lock(h, m, slot);
        let mem = malloc_body(h, m, bytes);
        h.write(slot, mem);
        unlock(h, m);
        let head: u32 = h.read(mem.wrapping_sub(4));
        let overhead = if head & 3 == 0 { 8 } else { 4 };
        let size = head & !7;
        if size > overhead {
            h.call_sym(MEMSET, Conv::Cdecl, &[mem, 0xcd, size - overhead]);
        }
        mem
    }

    fn malloc_body(h: &mut impl Host, m: u32, bytes: u32) -> u32 {
        let nb;
        if bytes <= 0xf4 {
            nb = if bytes < 0xb {
                0x10
            } else {
                bytes.wrapping_add(0xb) & !7
            };
            let idx = nb >> 3;
            let smallbits = h.read::<u32>(m) >> (idx & 31);
            if smallbits & 3 != 0 {
                let i = idx + (!smallbits & 1);
                let p = unlink_first_small(h, m, i);
                h.write(at(p, 4), (i << 3) | 3);
                let next = at(p, (i << 3).wrapping_add(4));
                let head: u32 = h.read(next);
                h.write(next, head | 1);
                return at(p, 8);
            }
            if nb > h.read::<u32>(at(m, 8)) {
                if smallbits != 0 {
                    let left = (smallbits << (idx & 31)) & (u32::MAX << (idx & 31)).wrapping_mul(2);
                    let i = (left & left.wrapping_neg()).trailing_zeros();
                    let p = unlink_first_small(h, m, i);
                    let rsize = (i << 3).wrapping_sub(nb);
                    h.write(at(p, 4), nb | 3);
                    let r = at(p, nb);
                    h.write(at(r, 4), rsize | 1);
                    h.write(at(r, rsize), rsize);
                    replace_dv(h, m, r, rsize);
                    return at(p, 8);
                }
                if h.read::<u32>(at(m, 4)) != 0 {
                    let mem = h.call_sym(TMALLOC_SMALL, Conv::Cdecl, &[m, nb]);
                    if mem != 0 {
                        return mem;
                    }
                }
            }
        } else if bytes >= 0xffff_ffc0 {
            nb = u32::MAX;
        } else {
            nb = bytes.wrapping_add(0xb) & !7;
            if h.read::<u32>(at(m, 4)) != 0 {
                let mem = h.call_sym(TMALLOC_LARGE, Conv::Cdecl, &[m, nb]);
                if mem != 0 {
                    return mem;
                }
            }
        }
        let dvs: u32 = h.read(at(m, 8));
        if nb <= dvs {
            let p: u32 = h.read(at(m, 0x14));
            let rsize = dvs - nb;
            if rsize >= 0x10 {
                let r = at(p, nb);
                h.write(at(m, 8), rsize);
                h.write(at(m, 0x14), r);
                h.write(at(r, 4), rsize | 1);
                h.write(at(r, rsize), rsize);
                h.write(at(p, 4), nb | 3);
            } else {
                h.write(at(m, 8), 0u32);
                h.write(at(m, 0x14), 0u32);
                h.write(at(p, 4), dvs | 3);
                let next = at(p, dvs.wrapping_add(4));
                let head: u32 = h.read(next);
                h.write(next, head | 1);
            }
            return at(p, 8);
        }
        let top_size: u32 = h.read(at(m, 0xc));
        if nb < top_size {
            let top: u32 = h.read(at(m, 0x18));
            let rsize = top_size - nb;
            h.write(at(m, 0xc), rsize);
            let r = at(top, nb);
            h.write(at(m, 0x18), r);
            h.write(at(r, 4), rsize | 1);
            h.write(at(top, 4), nb | 3);
            return at(top, 8);
        }
        h.call_sym(SYS_ALLOC, Conv::Cdecl, &[m, nb])
    }

    /// dlmalloc's `sys_trim(m, pad)` (`0x006eb3e0`) and
    /// `release_unused_segments(m)` (`0x006eae70`) (cdecl).
    pub const SYS_TRIM: u32 = 0x006e_b3e0;
    pub const RELEASE_UNUSED: u32 = 0x006e_ae70;
    pub const MSPACE_FREE_SYM: u32 = 0x006e_bee0;

    /// Small bin chunk `p` (size index `i`) off its list; links below the
    /// least address (and not the bin) leave it as it is.
    fn unlink_small(h: &mut impl Host, m: u32, p: u32, i: u32) {
        let f: u32 = h.read(at(p, 8));
        let b: u32 = h.read(at(p, 0xc));
        if f == b {
            let map: u32 = h.read(m);
            h.write(m, map & !(1u32 << (i & 31)));
            return;
        }
        let bin = at(m, 0x28 + i.wrapping_mul(8));
        let least: u32 = h.read(at(m, 0x10));
        if (f == bin || f >= least) && (b == bin || b >= least) {
            h.write(at(f, 0xc), b);
            h.write(at(b, 8), f);
        }
    }

    /// dlmalloc's `unlink_chunk`.
    fn unlink_chunk(h: &mut impl Host, m: u32, p: u32, size: u32) {
        if size >> 3 < 0x20 {
            unlink_small(h, m, p, size >> 3);
        } else {
            unlink_large_chunk(h, m, p);
        }
    }

    /// dlmalloc's `insert_chunk` of `p` (size `size`); `true` for a large
    /// chunk (the caller then counts a release check).
    fn insert_chunk(h: &mut impl Host, m: u32, p: u32, size: u32) -> bool {
        if size >> 3 < 0x20 {
            let i = size >> 3;
            let b = at(m, 0x28 + i * 8);
            let bit = 1u32 << i;
            let map: u32 = h.read(m);
            let mut f = b;
            if map & bit == 0 {
                h.write(m, map | bit);
            } else {
                let fd: u32 = h.read(at(b, 8));
                if fd >= h.read::<u32>(at(m, 0x10)) {
                    f = fd;
                }
            }
            h.write(at(b, 8), p);
            h.write(at(f, 0xc), p);
            h.write(at(p, 8), f);
            h.write(at(p, 0xc), b);
            return false;
        }
        let x = size >> 8;
        let i = if x == 0 {
            0
        } else if x > 0xffff {
            31
        } else {
            let k = x.ilog2();
            ((size >> (k + 7)) & 1) + k * 2
        };
        let head = at(m, 0x130 + 4 * i);
        h.write(at(p, 0x1c), i);
        h.write(at(p, 0x14), 0u32);
        h.write(at(p, 0x10), 0u32);
        let tree: u32 = h.read(at(m, 4));
        if tree & (1 << i) == 0 {
            h.write(at(m, 4), tree | (1 << i));
            h.write(head, p);
            h.write(at(p, 0x18), head);
            h.write(at(p, 0xc), p);
            h.write(at(p, 8), p);
            return true;
        }
        let mut t: u32 = h.read(head);
        let mut k = size << (if i == 31 { 0 } else { 0x19 - (i >> 1) });
        loop {
            if h.read::<u32>(at(t, 4)) & !7 == size {
                let f: u32 = h.read(at(t, 8));
                let l: u32 = h.read(at(m, 0x10));
                if t >= l && f >= l {
                    h.write(at(f, 0xc), p);
                    h.write(at(t, 8), p);
                    h.write(at(p, 8), f);
                    h.write(at(p, 0xc), t);
                    h.write(at(p, 0x18), 0u32);
                }
                return true;
            }
            let c = at(t, 0x10 + (k >> 31) * 4);
            k <<= 1;
            let child: u32 = h.read(c);
            if child == 0 {
                if c >= h.read::<u32>(at(m, 0x10)) {
                    h.write(c, p);
                    h.write(at(p, 0x18), t);
                    h.write(at(p, 0xc), p);
                    h.write(at(p, 8), p);
                }
                return true;
            }
            t = child;
        }
    }

    /// `mspace_free(m, mem)` (`0x006ebee0`, cdecl): dlmalloc's free. The
    /// block's usable bytes are filled with 0xfe first; then under the
    /// heap's lock (`spins`, the caller's `mem` slot, counts its tries)
    /// the chunk is merged with free neighbours (a mapped chunk is handed
    /// back region by region, `info` the original's 28-byte
    /// `MEMORY_BASIC_INFORMATION` at `ebp - 0x28`, as in
    /// [`release_segments`]), joins the top (trimmed past the threshold
    /// `+0x1c`) or the victim, or goes into its bin (a large one counts
    /// down the release checks `+0x20`). Chunks below the least address
    /// or with bad in-use bits are left alone.
    pub fn mspace_free(h: &mut impl Host, m: u32, mem: u32, spins: u32, info: u32) {
        if mem == 0 {
            return;
        }
        let head: u32 = h.read(mem.wrapping_sub(4));
        let overhead = if head & 3 == 0 { 8 } else { 4 };
        let size = head & !7;
        if size > overhead {
            h.call_sym(MEMSET, Conv::Cdecl, &[mem, 0xfe, size - overhead]);
        }
        lock(h, m, spins);
        free_body(h, m, mem.wrapping_sub(8), info);
        unlock(h, m);
    }

    #[allow(clippy::too_many_lines)] // the original's paths
    fn free_body(h: &mut impl Host, m: u32, p: u32, info: u32) {
        let least: u32 = h.read(at(m, 0x10));
        if p < least {
            return;
        }
        let head: u32 = h.read(at(p, 4));
        let inuse = head & 3;
        if inuse == 1 {
            return;
        }
        let mut psize = head & !7;
        let next = at(p, psize);
        let mut p = p;
        if head & 1 == 0 {
            let prevsize: u32 = h.read(p);
            p = p.wrapping_sub(prevsize);
            if inuse == 0 {
                let total = prevsize.wrapping_add(0x10).wrapping_add(psize);
                let mut base = p;
                let mut left = total;
                let query: u32 = h.read(h.rebase(VIRTUAL_QUERY));
                let free: u32 = h.read(h.rebase(VIRTUAL_FREE));
                while left != 0 {
                    if h.call(query, Conv::Stdcall, &[base, info, 0x1c]) == 0
                        || h.read::<u32>(info) != base
                        || h.read::<u32>(at(info, 4)) != base
                        || h.read::<u32>(at(info, 0x10)) != 0x1000
                        || h.read::<u32>(at(info, 0xc)) > left
                        || h.call(free, Conv::Stdcall, &[base, 0, 0x8000]) == 0
                    {
                        return;
                    }
                    let region: u32 = h.read(at(info, 0xc));
                    base = base.wrapping_add(region);
                    left = left.wrapping_sub(region);
                }
                let n: u32 = h.read(at(m, 0x1b0));
                h.write(at(m, 0x1b0), n.wrapping_sub(total));
                return;
            }
            psize = psize.wrapping_add(prevsize);
            if p < h.read::<u32>(at(m, 0x10)) {
                return;
            }
            if p == h.read::<u32>(at(m, 0x14)) {
                let nhead: u32 = h.read(at(next, 4));
                if nhead & 3 == 3 {
                    h.write(at(m, 8), psize);
                    h.write(at(next, 4), nhead & !1);
                    h.write(at(p, 4), psize | 1);
                    h.write(at(p, psize), psize);
                    return;
                }
            } else {
                unlink_chunk(h, m, p, prevsize);
            }
        }
        if p >= next {
            return;
        }
        let nhead: u32 = h.read(at(next, 4));
        if nhead & 1 == 0 {
            return;
        }
        if nhead & 2 == 0 {
            if next == h.read::<u32>(at(m, 0x18)) {
                let tsize = h.read::<u32>(at(m, 0xc)).wrapping_add(psize);
                h.write(at(m, 0xc), tsize);
                h.write(at(m, 0x18), p);
                h.write(at(p, 4), tsize | 1);
                if p == h.read::<u32>(at(m, 0x14)) {
                    h.write(at(m, 0x14), 0u32);
                    h.write(at(m, 8), 0u32);
                }
                if tsize > h.read::<u32>(at(m, 0x1c)) {
                    h.call_sym(SYS_TRIM, Conv::Cdecl, &[m, 0]);
                }
                return;
            }
            if next == h.read::<u32>(at(m, 0x14)) {
                let dsize = h.read::<u32>(at(m, 8)).wrapping_add(psize);
                h.write(at(m, 8), dsize);
                h.write(at(m, 0x14), p);
                h.write(at(p, 4), dsize | 1);
                h.write(at(p, dsize), dsize);
                return;
            }
            let nsize = nhead & !7;
            psize = psize.wrapping_add(nsize);
            unlink_chunk(h, m, next, nsize);
            h.write(at(p, 4), psize | 1);
            h.write(at(p, psize), psize);
            if p == h.read::<u32>(at(m, 0x14)) {
                h.write(at(m, 8), psize);
                return;
            }
        } else {
            h.write(at(next, 4), nhead & !1);
            h.write(at(p, 4), psize | 1);
            h.write(at(p, psize), psize);
        }
        if insert_chunk(h, m, p, psize) {
            let n = h.read::<u32>(at(m, 0x20)).wrapping_sub(1);
            h.write(at(m, 0x20), n);
            if n == 0 {
                h.call_sym(RELEASE_UNUSED, Conv::Cdecl, &[m]);
            }
        }
    }
}
