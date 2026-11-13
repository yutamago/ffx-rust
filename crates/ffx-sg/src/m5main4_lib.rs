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
        insert_large_chunk(h, m, p, size);
        true
    }

    /// dlmalloc's tree bin index of a large chunk size.
    fn tree_index(size: u32) -> u32 {
        let x = size >> 8;
        if x == 0 {
            0
        } else if x > 0xffff {
            31
        } else {
            let k = x.ilog2();
            ((size >> (k + 7)) & 1) + k * 2
        }
    }

    /// dlmalloc's `insert_large_chunk` of `p` (size `size`) into its tree
    /// bin; links below the least address are left alone.
    fn insert_large_chunk(h: &mut impl Host, m: u32, p: u32, size: u32) {
        let i = tree_index(size);
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
            return;
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
                return;
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
                return;
            }
            t = child;
        }
    }

    /// dlmalloc's `internal_memalign(m, alignment, bytes)` (`0x006ea5f0`,
    /// cdecl; `mspace_memalign`'s callee; m5-wp5e): an alignment up to 8
    /// is plain [`MSPACE_MALLOC_SYM`]; else the alignment at least 16 and
    /// rounded up to a power of two, `bytes` under `0xffffffc0 -
    /// alignment` (else 0), a block of `nb + alignment + 12` (`nb` the
    /// padded request, at least 16), and under the heap's lock: when the
    /// block is not aligned, the aligned chunk inside it (moved on by the
    /// alignment when the lead would be under 16 bytes) takes the rest
    /// (a mapped chunk: its previous foot grows by the lead; else both
    /// chunks marked in use) and the lead is freed after; a tail of more
    /// than 16 bytes past `nb` is split off and freed too
    /// ([`MSPACE_FREE_SYM`], after the lock is released). The aligned
    /// block. `spins` is a word for the lock's spin counter (a register in
    /// the original).
    pub fn internal_memalign(
        h: &mut impl Host,
        m: u32,
        alignment: u32,
        bytes: u32,
        spins: u32,
    ) -> u32 {
        if alignment <= 8 {
            return h.call_sym(MSPACE_MALLOC_SYM, Conv::Cdecl, &[m, bytes]);
        }
        let mut align = alignment.max(0x10);
        if align & (align - 1) != 0 {
            let mut a = 0x10u32;
            while a < align {
                a = a.wrapping_add(a);
            }
            align = a;
        }
        if bytes >= 0xffff_ffc0u32.wrapping_sub(align) {
            return 0;
        }
        let nb = if bytes < 0xb {
            0x10
        } else {
            bytes.wrapping_add(0xb) & !7
        };
        let mem = h.call_sym(
            MSPACE_MALLOC_SYM,
            Conv::Cdecl,
            &[m, nb.wrapping_add(0xc).wrapping_add(align)],
        );
        if mem == 0 {
            return 0;
        }
        let mut leader = 0;
        let mut p = mem.wrapping_sub(8);
        lock(h, m, spins);
        if !mem.is_multiple_of(align) {
            let mut br =
                (mem.wrapping_sub(1).wrapping_add(align) & align.wrapping_neg()).wrapping_sub(8);
            if br.wrapping_sub(p) < 0x10 {
                br = br.wrapping_add(align);
            }
            let head: u32 = h.read(at(p, 4));
            let lead = br.wrapping_sub(p);
            let size = (head & !7).wrapping_sub(lead);
            if head & 3 == 0 {
                // a mapped chunk
                let foot: u32 = h.read(p);
                h.write(br, foot.wrapping_add(lead));
                h.write(at(br, 4), size);
            } else {
                let nh: u32 = h.read(at(br, 4));
                h.write(at(br, 4), (nh & 1) | size | 2);
                let next = at(br, size.wrapping_add(4));
                let w: u32 = h.read(next);
                h.write(next, w | 1);
                let ph: u32 = h.read(at(p, 4));
                h.write(at(p, 4), (ph & 1) | lead | 2);
                let next = at(p, lead.wrapping_add(4));
                let w: u32 = h.read(next);
                h.write(next, w | 1);
                leader = at(p, 8);
            }
            p = br;
        }
        let head: u32 = h.read(at(p, 4));
        let mut trailer = 0;
        if head & 3 != 0 && head & !7 > nb.wrapping_add(0x10) {
            let rest = (head & !7).wrapping_sub(nb);
            let r = at(p, nb);
            h.write(at(p, 4), (head & 1) | nb | 2);
            let w: u32 = h.read(at(r, 4));
            h.write(at(r, 4), w | 1);
            let w: u32 = h.read(at(r, 4));
            h.write(at(r, 4), (w & 1) | rest | 2);
            let next = at(r, rest.wrapping_add(4));
            let w: u32 = h.read(next);
            h.write(next, w | 1);
            trailer = at(r, 8);
        }
        unlock(h, m);
        if leader != 0 {
            h.call_sym(MSPACE_FREE_SYM, Conv::Cdecl, &[m, leader]);
        }
        if trailer != 0 {
            h.call_sym(MSPACE_FREE_SYM, Conv::Cdecl, &[m, trailer]);
        }
        at(p, 8)
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

    // ---- m5-wp5f: heap growth and release ----

    /// dlmalloc's `mparams`: `magic` (0 before `init_mparams`), the
    /// allocation granularity, the mapping and trim thresholds.
    pub const MPARAMS_MAGIC: u32 = 0x00ce_6ec4;
    pub const GRANULARITY: u32 = 0x00ce_6ecc;
    pub const MMAP_THRESHOLD: u32 = 0x00ce_6ed0;
    pub const TRIM_THRESHOLD: u32 = 0x00ce_6ed4;
    /// `init_mparams()` (`0x006ea050`, cdecl).
    pub const INIT_MPARAMS: u32 = 0x006e_a050;
    /// `VirtualAlloc(p, n, type, protect)`'s import slot (stdcall).
    pub const VIRTUAL_ALLOC: u32 = 0x00b0_c09c;
    /// dlmalloc's `add_segment(m, base, size, flags)`, `init_top(m, p,
    /// size)`, `mmap_alloc(m, nb)`, `prepend_alloc(m, new, old, nb)` and
    /// `segment_holding(m, p)` (cdecl).
    pub const ADD_SEGMENT: u32 = 0x006e_9ae0;
    pub const INIT_TOP: u32 = 0x006e_a140;
    pub const MMAP_ALLOC: u32 = 0x006e_aa30;
    pub const PREPEND_ALLOC: u32 = 0x006e_ab10;
    pub const SEGMENT_HOLDING: u32 = 0x006e_b1a0;

    fn global(h: &impl Host, g: u32) -> u32 {
        h.read(h.rebase(g))
    }

    /// dlmalloc's `align_offset`: the bytes up to the next 8-byte boundary.
    fn align_offset(p: u32) -> u32 {
        p.wrapping_neg() & 7
    }

    /// The heap's footprint `+0x1b0` grown by `n`, its maximum `+0x1b4`
    /// following.
    fn footprint_grow(h: &mut impl Host, m: u32, n: u32) {
        let f = h.read::<u32>(at(m, 0x1b0)).wrapping_add(n);
        h.write(at(m, 0x1b0), f);
        if h.read::<u32>(at(m, 0x1b4)) < f {
            h.write(at(m, 0x1b4), f);
        }
    }

    /// `init_top(m, p, psize)` (`0x006ea140`, cdecl): the top chunk `+0x18`
    /// at `p` aligned to 8, its size `+0xc` the rest of `psize`, its
    /// head (pinuse set) and the 0x28 bytes of the top foot after it; the
    /// trim check `+0x1c` from `mparams`' trim threshold.
    pub fn init_top(h: &mut impl Host, m: u32, p: u32, psize: u32) {
        let off = align_offset(p);
        let top = at(p, off);
        let size = psize.wrapping_sub(off);
        h.write(at(m, 0x18), top);
        h.write(at(m, 0xc), size);
        h.write(at(top, 4), size | 1);
        let trim = global(h, TRIM_THRESHOLD);
        h.write(at(top, size.wrapping_add(4)), 0x28u32);
        h.write(at(m, 0x1c), trim);
    }

    /// `segment_holding(m, p)` (`0x006eb1a0`, cdecl): the first segment
    /// record (from `m + 0x1c8`: base, size, next) whose range holds `p`,
    /// or 0.
    pub fn segment_holding(h: &mut impl Host, m: u32, p: u32) -> u32 {
        let mut sp = at(m, 0x1c8);
        loop {
            let base: u32 = h.read(sp);
            if p >= base && p < h.read::<u32>(at(sp, 4)).wrapping_add(base) {
                return sp;
            }
            sp = h.read(at(sp, 8));
            if sp == 0 {
                return 0;
            }
        }
    }

    /// The pages from `base` for `size` bytes handed back, region by region
    /// (`VirtualQuery` into `info`, a 28-byte `MEMORY_BASIC_INFORMATION`):
    /// each region must start at `base` and its own allocation, be
    /// committed and fit in what is left, and `VirtualFree(base, 0,
    /// MEM_RELEASE)` must succeed. `false` at the first that fails.
    fn release_pages(h: &mut impl Host, base: u32, size: u32, info: u32) -> bool {
        let query: u32 = h.read(h.rebase(VIRTUAL_QUERY));
        let free: u32 = h.read(h.rebase(VIRTUAL_FREE));
        let (mut base, mut left) = (base, size);
        while left != 0 {
            if h.call(query, Conv::Stdcall, &[base, info, 0x1c]) == 0
                || h.read::<u32>(info) != base
                || h.read::<u32>(at(info, 4)) != base
                || h.read::<u32>(at(info, 0x10)) != 0x1000
                || h.read::<u32>(at(info, 0xc)) > left
                || h.call(free, Conv::Stdcall, &[base, 0, 0x8000]) == 0
            {
                return false;
            }
            let region: u32 = h.read(at(info, 0xc));
            base = base.wrapping_add(region);
            left = left.wrapping_sub(region);
        }
        true
    }

    /// `mmap_alloc(m, nb)` (`0x006eaa30`, cdecl): a chunk of its own
    /// mapping for a large request: `nb + 0x1f` rounded up to the
    /// granularity (0 when that wraps), `VirtualAlloc(0, size,
    /// MEM_RESERVE | MEM_COMMIT | MEM_TOP_DOWN, PAGE_READWRITE)` (0 when it
    /// gives 0 or -1); the chunk at the mapping's first 8-byte boundary
    /// (its previous foot the offset, its head the size less 16 with no
    /// in-use bits, two fence words after it); the least address and the
    /// footprint follow. The chunk's memory.
    pub fn mmap_alloc(h: &mut impl Host, m: u32, nb: u32) -> u32 {
        let unit = global(h, GRANULARITY);
        let size = nb.wrapping_add(0x1e).wrapping_add(unit) & !unit.wrapping_sub(1);
        if nb >= size {
            return 0;
        }
        let mm = stdcall_import(h, VIRTUAL_ALLOC, &[0, size, 0x10_3000, 4]);
        if mm == 0 || mm == u32::MAX {
            return 0;
        }
        let off = align_offset(mm);
        let left = size.wrapping_sub(off);
        let p = at(mm, off);
        h.write(at(p, 4), left.wrapping_sub(0x10));
        h.write(p, off);
        h.write(at(p, left.wrapping_sub(0xc)), 7u32);
        h.write(at(p, left.wrapping_sub(8)), 0u32);
        let least: u32 = h.read(at(m, 0x10));
        if least == 0 || mm < least {
            h.write(at(m, 0x10), mm);
        }
        footprint_grow(h, m, size);
        at(p, 8)
    }

    /// `prepend_alloc(m, newbase, oldbase, nb)` (`0x006eab10`, cdecl): new
    /// memory just below the segment at `oldbase`: the chunk for `nb` at
    /// `newbase` (aligned), the rest up to the old first chunk joins the
    /// top or the designated victim when that chunk is one of them, else
    /// takes in the old first chunk when it is free and goes into its bin.
    /// The chunk's memory.
    pub fn prepend_alloc(h: &mut impl Host, m: u32, newbase: u32, oldbase: u32, nb: u32) -> u32 {
        let p = at(newbase, align_offset(newbase));
        let mut oldfirst = at(oldbase, align_offset(oldbase));
        let mut qsize = oldfirst.wrapping_sub(p).wrapping_sub(nb);
        let q = at(p, nb);
        h.write(at(p, 4), nb | 3);
        if oldfirst == h.read::<u32>(at(m, 0x18)) {
            let tsize = h.read::<u32>(at(m, 0xc)).wrapping_add(qsize);
            h.write(at(m, 0xc), tsize);
            h.write(at(m, 0x18), q);
            h.write(at(q, 4), tsize | 1);
            return at(p, 8);
        }
        if oldfirst == h.read::<u32>(at(m, 0x14)) {
            let dsize = h.read::<u32>(at(m, 8)).wrapping_add(qsize);
            h.write(at(m, 8), dsize);
            h.write(at(m, 0x14), q);
            h.write(at(q, 4), dsize | 1);
            h.write(at(q, dsize), dsize);
            return at(p, 8);
        }
        let head: u32 = h.read(at(oldfirst, 4));
        if head & 3 == 1 {
            let nsize = head & !7;
            unlink_chunk(h, m, oldfirst, nsize);
            oldfirst = at(oldfirst, nsize);
            qsize = qsize.wrapping_add(nsize);
        }
        let w: u32 = h.read(at(oldfirst, 4));
        h.write(at(oldfirst, 4), w & !1);
        h.write(at(q, 4), qsize | 1);
        h.write(at(q, qsize), qsize);
        insert_chunk(h, m, q, qsize);
        at(p, 8)
    }

    /// `add_segment(m, tbase, tsize, mmapped)` (`0x006e9ae0`, cdecl): a new
    /// segment becomes the top (`init_top` inlined); the old segment
    /// record moves into a chunk (0x18 bytes, in use) at the end of the
    /// old top's segment, followed by fence words up to the segment's end;
    /// the record at `m + 0x1c8` names the new segment and links the old
    /// one; what is left of the old top before the record goes into its
    /// bin. A top no segment holds faults in the original (the record
    /// read from 0).
    pub fn add_segment(h: &mut impl Host, m: u32, tbase: u32, tsize: u32, mmapped: u32) {
        let old_top: u32 = h.read(at(m, 0x18));
        let sp = segment_holding(h, m, old_top);
        let size: u32 = h.read(at(sp, 4));
        let old_end = size.wrapping_add(h.read(sp));
        let raw = old_end.wrapping_sub(0x2f);
        let asp = at(raw, align_offset(raw));
        let csp = if asp < old_top.wrapping_add(0x10) {
            old_top
        } else {
            asp
        };
        let off = align_offset(tbase);
        let top_size = tsize.wrapping_sub(0x28).wrapping_sub(off);
        let top = at(tbase, off);
        h.write(at(m, 0xc), top_size);
        h.write(at(m, 0x18), top);
        h.write(at(top, 4), top_size | 1);
        h.write(at(top, top_size.wrapping_add(4)), 0x28u32);
        let trim = global(h, TRIM_THRESHOLD);
        h.write(at(m, 0x1c), trim);
        h.write(at(csp, 4), 0x1bu32);
        for o in [0, 4, 8, 0xc] {
            let w: u32 = h.read(at(m, 0x1c8 + o));
            h.write(at(csp, 8 + o), w);
        }
        h.write(at(m, 0x1cc), tsize);
        h.write(at(m, 0x1c8), tbase);
        h.write(at(m, 0x1d4), mmapped);
        h.write(at(m, 0x1d0), at(csp, 8));
        let mut fence = at(csp, 0x1c);
        h.write(fence, 7u32);
        fence = at(fence, 4);
        while fence < old_end {
            h.write(fence, 7u32);
            fence = at(fence, 4);
        }
        if csp != old_top {
            let psize = csp.wrapping_sub(old_top);
            let w: u32 = h.read(at(csp, 4));
            h.write(at(csp, 4), w & !1);
            h.write(at(old_top, 4), psize | 1);
            h.write(at(old_top, psize), psize);
            insert_chunk(h, m, old_top, psize);
        }
    }

    /// `sys_alloc(m, nb)` (`0x006eb1d0`, cdecl): memory from the system
    /// for `nb` when the bins and the top have none. `init_mparams` first
    /// when `mparams` has no magic; a request at the mapping threshold
    /// gets its own mapping ([`MMAP_ALLOC`]) when the heap may map (flag 1
    /// of `+0x1b8`) and has a top. Else `nb + 0x2f` rounded up to the
    /// granularity is mapped (`VirtualAlloc(0, size, MEM_RESERVE |
    /// MEM_COMMIT, PAGE_READWRITE)`): the first segment of an empty heap
    /// (the bins made empty, the top after the heap's own chunk), else the
    /// top's segment grown when the new memory follows it ([`INIT_TOP`]),
    /// a segment it precedes extended down ([`PREPEND_ALLOC`], its result
    /// returned), or a new segment ([`ADD_SEGMENT`]). The chunk is cut from
    /// the top when that is now larger than `nb`; else 0.
    pub fn sys_alloc(h: &mut impl Host, m: u32, nb: u32) -> u32 {
        if global(h, MPARAMS_MAGIC) == 0 {
            h.call_sym(INIT_MPARAMS, Conv::Cdecl, &[]);
        }
        if h.read::<u8>(at(m, 0x1b8)) & 1 != 0
            && nb >= global(h, MMAP_THRESHOLD)
            && h.read::<u32>(at(m, 0xc)) != 0
        {
            let mem = h.call_sym(MMAP_ALLOC, Conv::Cdecl, &[m, nb]);
            if mem != 0 {
                return mem;
            }
        }
        let unit = global(h, GRANULARITY);
        let asize = unit.wrapping_add(0x2f).wrapping_add(nb) & !unit.wrapping_sub(1);
        if asize <= nb {
            return 0;
        }
        let tbase = stdcall_import(h, VIRTUAL_ALLOC, &[0, asize, 0x3000, 4]);
        if tbase == 0 || tbase == u32::MAX {
            return 0;
        }
        footprint_grow(h, m, asize);
        if h.read::<u32>(at(m, 0x18)) == 0 {
            let least: u32 = h.read(at(m, 0x10));
            if least == 0 || tbase < least {
                h.write(at(m, 0x10), tbase);
            }
            let magic = global(h, MPARAMS_MAGIC);
            h.write(at(m, 0x1c8), tbase);
            h.write(at(m, 0x1cc), asize);
            h.write(at(m, 0x1d4), 1u32);
            h.write(at(m, 0x24), magic);
            h.write(at(m, 0x20), 0xfffu32);
            for i in 0..32 {
                let b = at(m, 0x28 + 8 * i);
                h.write(at(b, 0xc), b);
                h.write(at(b, 8), b);
            }
            // the top after the heap's own chunk
            let own = h.read::<u32>(m.wrapping_sub(4)) & !7;
            let mn = at(m.wrapping_sub(8), own);
            let off = align_offset(mn);
            let top = at(mn, off);
            let tsize = tbase
                .wrapping_sub(mn)
                .wrapping_sub(0x28)
                .wrapping_add(asize)
                .wrapping_sub(off);
            h.write(at(m, 0x18), top);
            h.write(at(m, 0xc), tsize);
            h.write(at(top, 4), tsize | 1);
            let trim = global(h, TRIM_THRESHOLD);
            h.write(at(top, tsize.wrapping_add(4)), 0x28u32);
            h.write(at(m, 0x1c), trim);
        } else if !grow_top_segment(h, m, tbase, asize) {
            if tbase < h.read::<u32>(at(m, 0x10)) {
                h.write(at(m, 0x10), tbase);
            }
            let end = tbase.wrapping_add(asize);
            let mut sp = at(m, 0x1c8);
            while sp != 0 {
                if h.read::<u32>(sp) == end {
                    let flags: u32 = h.read(at(sp, 0xc));
                    if flags & 8 == 0 && flags & 1 != 0 {
                        let oldbase: u32 = h.read(sp);
                        let n = h.read::<u32>(at(sp, 4)).wrapping_add(asize);
                        h.write(at(sp, 4), n);
                        h.write(sp, tbase);
                        return h.call_sym(PREPEND_ALLOC, Conv::Cdecl, &[m, tbase, oldbase, nb]);
                    }
                    break;
                }
                sp = h.read(at(sp, 8));
            }
            h.call_sym(ADD_SEGMENT, Conv::Cdecl, &[m, tbase, asize, 1]);
        }
        let tsize: u32 = h.read(at(m, 0xc));
        if nb >= tsize {
            return 0;
        }
        let p: u32 = h.read(at(m, 0x18));
        let rsize = tsize - nb;
        h.write(at(m, 0xc), rsize);
        let r = at(p, nb);
        h.write(at(m, 0x18), r);
        h.write(at(r, 4), rsize | 1);
        h.write(at(p, 4), nb | 3);
        at(p, 8)
    }

    /// `sys_alloc`'s first case for a heap with a top: the segment ending
    /// where the new memory `tbase` starts, when it holds the top, may be
    /// unmapped (flag 1) and isn't external (flag 8), grows by `asize` and
    /// the top with it ([`INIT_TOP`]). `false` when no segment does.
    fn grow_top_segment(h: &mut impl Host, m: u32, tbase: u32, asize: u32) -> bool {
        let mut sp = at(m, 0x1c8);
        while sp != 0 {
            let base: u32 = h.read(sp);
            if tbase == h.read::<u32>(at(sp, 4)).wrapping_add(base) {
                let flags: u32 = h.read(at(sp, 0xc));
                let top: u32 = h.read(at(m, 0x18));
                if flags & 8 != 0
                    || flags & 1 == 0
                    || top < base
                    || top >= base.wrapping_add(h.read(at(sp, 4)))
                {
                    return false;
                }
                let n = h.read::<u32>(at(sp, 4)).wrapping_add(asize);
                h.write(at(sp, 4), n);
                let size = h.read::<u32>(at(m, 0xc)).wrapping_add(asize);
                h.call_sym(INIT_TOP, Conv::Cdecl, &[m, top, size]);
                return true;
            }
            sp = h.read(at(sp, 8));
        }
        false
    }

    /// `sys_trim(m, pad)` (`0x006eb3e0`, cdecl) -> 1 when memory went back
    /// to the system, else 0. `init_mparams` first when `mparams` has no
    /// magic; nothing for a pad from `0xffffffc0` or a heap without a top.
    /// When the top is larger than `pad` and its foot, whole granules of
    /// it past one (`extra`) are released from the end of its segment
    /// ([`SEGMENT_HOLDING`]; a mapped, not external segment as large as
    /// `extra` that holds no segment record), the top shrunk with them
    /// ([`INIT_TOP`]). Then [`RELEASE_UNUSED`]; with nothing released at
    /// all and the top still past the trim check `+0x1c`, the check is
    /// set to -1. `info` is the original's 28-byte
    /// `MEMORY_BASIC_INFORMATION` at `ebp - 0x28`. (The original keeps
    /// `extra` in its `pad` slot; no caller reads it.)
    pub fn sys_trim(h: &mut impl Host, m: u32, pad: u32, info: u32) -> u32 {
        if global(h, MPARAMS_MAGIC) == 0 {
            h.call_sym(INIT_MPARAMS, Conv::Cdecl, &[]);
        }
        if pad >= 0xffff_ffc0 {
            return 0;
        }
        let top: u32 = h.read(at(m, 0x18));
        if top == 0 {
            return 0;
        }
        let mut released = 0u32;
        let tsize: u32 = h.read(at(m, 0xc));
        let want = pad.wrapping_add(0x28);
        if tsize > want {
            let unit = global(h, GRANULARITY);
            let granules = unit
                .wrapping_sub(1)
                .wrapping_add(tsize - want)
                .checked_div(unit)
                .unwrap_or(0);
            let extra = granules.wrapping_sub(1).wrapping_mul(unit);
            let sp = h.call_sym(SEGMENT_HOLDING, Conv::Cdecl, &[m, top]);
            let flags: u32 = h.read(at(sp, 0xc));
            if flags & 8 == 0 && flags & 1 != 0 && h.read::<u32>(at(sp, 4)) >= extra {
                let base: u32 = h.read(sp);
                let size: u32 = h.read(at(sp, 4));
                if !holds_segment_record(h, m, base, size)
                    && release_pages(h, base.wrapping_sub(extra).wrapping_add(size), extra, info)
                    && extra != 0
                {
                    released = extra;
                    let n = h.read::<u32>(at(sp, 4)).wrapping_sub(extra);
                    h.write(at(sp, 4), n);
                    let tsize = h.read::<u32>(at(m, 0xc));
                    let f = h.read::<u32>(at(m, 0x1b0)).wrapping_sub(extra);
                    h.write(at(m, 0x1b0), f);
                    let top: u32 = h.read(at(m, 0x18));
                    h.call_sym(INIT_TOP, Conv::Cdecl, &[m, top, tsize.wrapping_sub(extra)]);
                }
            }
        }
        released = released.wrapping_add(h.call_sym(RELEASE_UNUSED, Conv::Cdecl, &[m]));
        if released == 0 && h.read::<u32>(at(m, 0xc)) > h.read::<u32>(at(m, 0x1c)) {
            h.write(at(m, 0x1c), u32::MAX);
        }
        u32::from(released != 0)
    }

    /// dlmalloc's `has_segment_link`: a segment record (from `m + 0x1c8`)
    /// inside `[base, base + size)`.
    fn holds_segment_record(h: &mut impl Host, m: u32, base: u32, size: u32) -> bool {
        let mut s = at(m, 0x1c8);
        loop {
            if s >= base && s < size.wrapping_add(base) {
                return true;
            }
            s = h.read(at(s, 8));
            if s == 0 {
                return false;
            }
        }
    }

    /// `release_unused_segments(m)` (`0x006eae70`, cdecl) -> the bytes
    /// released. Every mapped, not external segment after the first
    /// whose first chunk is free and spans it (to its foot) is taken out
    /// of the free structures (the designated victim cleared, or the
    /// chunk unlinked from its tree) and its pages handed back (`info`,
    /// the original's 28-byte `MEMORY_BASIC_INFORMATION` at `ebp - 0x3c`):
    /// then the segment leaves the list and the footprint; when a release
    /// fails the chunk goes back into its tree bin. The release checks
    /// `+0x20` become the segments counted, at least 0xfff.
    pub fn release_unused_segments(h: &mut impl Host, m: u32, info: u32) -> u32 {
        let mut released = 0u32;
        let mut count = 0u32;
        let mut pred = at(m, 0x1c8);
        let mut sp: u32 = h.read(at(m, 0x1d0));
        while sp != 0 {
            let base: u32 = h.read(sp);
            let size: u32 = h.read(at(sp, 4));
            let next: u32 = h.read(at(sp, 8));
            count = count.wrapping_add(1);
            let flags: u32 = h.read(at(sp, 0xc));
            let mut gone = false;
            if flags & 1 != 0 && flags & 8 == 0 {
                let p = at(base, align_offset(base));
                let head: u32 = h.read(at(p, 4));
                let psize = head & !7;
                if head & 3 == 1
                    && size.wrapping_sub(0x28).wrapping_add(base) <= psize.wrapping_add(p)
                {
                    if p == h.read::<u32>(at(m, 0x14)) {
                        h.write(at(m, 0x14), 0u32);
                        h.write(at(m, 8), 0u32);
                    } else {
                        unlink_large_chunk(h, m, p);
                    }
                    if release_pages(h, base, size, info) {
                        released = released.wrapping_add(size);
                        let f = h.read::<u32>(at(m, 0x1b0)).wrapping_sub(size);
                        h.write(at(m, 0x1b0), f);
                        h.write(at(pred, 8), next);
                        gone = true;
                    } else {
                        insert_large_chunk(h, m, p, psize);
                    }
                }
            }
            if !gone {
                pred = sp;
            }
            sp = next;
        }
        h.write(at(m, 0x20), count.max(0xfff));
        released
    }

    /// `tmalloc_large(m, nb)` (`0x006eb540`, cdecl): dlmalloc's best fit
    /// for a large request from the tree bins: down `nb`'s own tree by its
    /// size bits (an exact fit ends the walk; the deepest right subtree
    /// passed is the next place to look), else the smallest larger bin,
    /// then down the leftmost children. A chunk that would leave less than
    /// the designated victim's remainder (`+8 - nb`), lies below the least
    /// address or wraps gives 0; else it is unlinked and split (a
    /// remainder of 16 bytes or more goes into its bin). The chunk's
    /// memory, or 0. (The original uses its `nb` slot as a scratch word
    /// in the remainder's tree insert; no caller reads it.)
    pub fn tmalloc_large<H: Host>(h: &mut H, m: u32, nb: u32) -> u32 {
        let size = |h: &mut H, c: u32| -> u32 { h.read::<u32>(at(c, 4)) & !7 };
        let mut v = 0u32;
        let mut rsize = nb.wrapping_neg();
        let idx = tree_index(nb);
        let mut t: u32 = h.read(at(m, 0x130 + 4 * idx));
        if t != 0 {
            let mut bits = nb << (if idx == 31 { 0 } else { 0x19 - (idx >> 1) });
            let mut rst = 0u32;
            loop {
                let trem = size(h, t).wrapping_sub(nb);
                if trem < rsize {
                    v = t;
                    rsize = trem;
                    if trem == 0 {
                        break;
                    }
                }
                let rt: u32 = h.read(at(t, 0x14));
                t = h.read(at(t, 0x10 + (bits >> 31) * 4));
                if rt != 0 && rt != t {
                    rst = rt;
                }
                if t == 0 {
                    t = rst;
                    break;
                }
                bits <<= 1;
            }
        }
        if t == 0 && v == 0 {
            let left = ((u32::MAX << idx) << 1) & h.read::<u32>(at(m, 4));
            if left != 0 {
                t = h.read(at(m, 0x130 + 4 * left.trailing_zeros()));
            }
        }
        while t != 0 {
            let trem = size(h, t).wrapping_sub(nb);
            if trem < rsize {
                rsize = trem;
                v = t;
            }
            let l: u32 = h.read(at(t, 0x10));
            t = if l != 0 { l } else { h.read(at(t, 0x14)) };
        }
        if v == 0 || rsize >= h.read::<u32>(at(m, 8)).wrapping_sub(nb) {
            return 0;
        }
        let r = at(v, nb);
        if v < h.read::<u32>(at(m, 0x10)) || r <= v {
            return 0;
        }
        unlink_large_chunk(h, m, v);
        if rsize < 0x10 {
            h.write(at(v, 4), rsize.wrapping_add(nb) | 3);
            let next = at(v, nb.wrapping_add(rsize).wrapping_add(4));
            let w: u32 = h.read(next);
            h.write(next, w | 1);
            return at(v, 8);
        }
        h.write(at(v, 4), nb | 3);
        h.write(at(r, 4), rsize | 1);
        h.write(at(r, rsize), rsize);
        insert_chunk(h, m, r, rsize);
        at(v, 8)
    }
}
