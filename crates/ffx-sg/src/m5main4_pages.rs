//! M5 stage 1, list `main`, round 4: the game's page heap (`g_MemSystem`
//! at `0x0193c390`): the block pool of whole pages (a page list from
//! `[pool]`, `+4` the page size; each page a ring of blocks), the
//! fixed-size allocators built on it (chunks with free bitmaps), the
//! large-allocation hash table, and their teardown (C++ unwind funclets
//! reach most of these).
//!
//! A page (`[pool]`, next page `+0x10`): its free ring head at `+0`
//! (`+0`/`+4` the ring links), then blocks from `+0x24`: `+0`/`+4` free
//! ring links (0 while in use), `+8` the size, `+0xc` the distance back
//! to the previous block; a sentinel block at the page's end.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // registers
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn stdcall_import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    /// `PageAlloc(size, align)`, `PageCommit(addr, size)` -> `al`,
    /// `PageQueueDecommit(addr, size)`, `PageRelease(addr)` (cdecl).
    pub const PAGE_ALLOC: u32 = 0x0094_8110;
    pub const PAGE_COMMIT: u32 = 0x0094_7570;
    pub const PAGE_QUEUE_DECOMMIT: u32 = 0x0094_7670;
    pub const PAGE_RELEASE: u32 = 0x0094_7e80;
    /// The critical section imports (stdcall, one word).
    pub const INITIALIZE_CS: u32 = 0x00b0_c150;
    pub const ENTER_CS: u32 = 0x00b0_c154;
    pub const LEAVE_CS: u32 = 0x00b0_c158;
    pub const DELETE_CS: u32 = 0x00b0_c15c;
    /// This module's originals, for the ones that call each other.
    pub const NEW_PAGE: u32 = 0x0094_3c50;
    pub const POOL_ALLOC: u32 = 0x0094_3a50;
    pub const POOL_FREE: u32 = 0x0094_3e30;
    pub const POOL_FREE_ALL: u32 = 0x0094_39f0;
    pub const HEAP_FREE_ALL: u32 = 0x0094_2a80;
    pub const CHUNK_REMOVE: u32 = 0x0094_4660;
    pub const TABLE_CLEAR: u32 = 0x0094_4f00;
    pub const BIG_FREE: u32 = 0x0094_4f80;

    /// `FUN_00943c50(pool)` (thiscall): a new page of the pool's page
    /// size (`+4`, 4 KB aligned), committed, with one free block over all
    /// of it (size `page - 0x34`) and the end sentinel. Returns it, or 0.
    pub fn new_page(h: &mut impl Host, pool: u32) -> u32 {
        let size: u32 = h.read(at(pool, 4));
        let p = cdecl(h, PAGE_ALLOC, &[size, 0x1000]);
        if p == 0 {
            return 0;
        }
        let size: u32 = h.read(at(pool, 4));
        if cdecl(h, PAGE_COMMIT, &[p, size]).to_le_bytes()[0] == 0 {
            return 0;
        }
        h.write(at(p, 0x10), 0u32);
        for o in [0, 4, 8, 0xc, 0x14, 0x18, 0x20] {
            h.write(at(p, o), 0u32);
        }
        h.write(at(p, 0x1c), 0x10u32);
        let b = at(p, 0x24);
        h.write(b, p);
        h.write(at(b, 4), p);
        let free = h.read::<u32>(at(pool, 4)).wrapping_sub(0x34);
        h.write(at(b, 0xc), 0x10u32);
        h.write(at(b, 8), free);
        h.write(p, b);
        h.write(at(p, 4), b);
        let end = at(b, h.read(at(b, 8)));
        h.write(end, 0u32);
        h.write(at(end, 4), 0u32);
        h.write(at(end, 8), 0x10u32);
        h.write(at(end, 0xc), free);
        p
    }

    /// `FUN_00943a50(pool; n)` (thiscall, `ret 4`): `n` bytes (plus the
    /// 16-byte header) from the first free block that fits, in the pages
    /// in order (a page added when none fits); a remainder under 16 bytes
    /// stays in the block. Returns the memory, or 0.
    pub fn pool_alloc(h: &mut impl Host, pool: u32, n: u32) -> u32 {
        let need = n.wrapping_add(0x10);
        let mut page: u32 = h.read(pool);
        if page == 0 {
            page = h.call_sym(NEW_PAGE, Conv::Thiscall(pool), &[]);
            h.write(pool, page);
            if page == 0 {
                return 0;
            }
        }
        loop {
            let mut b: u32 = h.read(at(page, 4));
            while b != page {
                let rest = h.read::<u32>(at(b, 8)).wrapping_sub(need);
                if rest.cast_signed() >= 0 {
                    return take(h, b, need, rest);
                }
                b = h.read(at(b, 4));
            }
            if h.read::<u32>(at(page, 0x10)) == 0 {
                let fresh = h.call_sym(NEW_PAGE, Conv::Thiscall(pool), &[]);
                h.write(at(page, 0x10), fresh);
            }
            page = h.read(at(page, 0x10));
            if page == 0 {
                return 0;
            }
        }
    }

    /// The split of block `b` into `need` bytes and the free rest.
    fn take(h: &mut impl Host, b: u32, need: u32, rest: u32) -> u32 {
        let (mut need, mut rest) = (need, rest);
        let prev: u32 = h.read(b);
        let next: u32 = h.read(at(b, 4));
        if rest.cast_signed() < 0x10 {
            need = need.wrapping_add(rest);
            rest = 0;
        }
        let after = at(b, h.read(at(b, 8)));
        if rest != 0 {
            let nb = at(b, need);
            h.write(at(nb, 8), rest);
            h.write(at(nb, 0xc), need);
            h.write(at(prev, 4), nb);
            h.write(nb, prev);
            h.write(at(nb, 4), next);
            h.write(next, nb);
            h.write(at(after, 0xc), rest);
        } else {
            h.write(at(prev, 4), next);
            h.write(next, prev);
            h.write(at(after, 0xc), need);
        }
        h.write(at(b, 8), need);
        h.write(b, 0u32);
        h.write(at(b, 4), 0u32);
        at(b, 0x10)
    }

    /// `FUN_00943e30(pool; p)` (thiscall, `ret 4`): the block of `p` freed
    /// and merged with free neighbours, put on its page's free ring; a
    /// page left wholly free is unlinked, decommitted and released.
    /// Returns `PageRelease`'s `eax`, else the page's free size.
    pub fn pool_free(h: &mut impl Host, pool: u32, p: u32) -> u32 {
        let mut page: u32 = h.read(pool);
        while page != 0 && !(p >= page && p < page.wrapping_add(h.read(at(pool, 4)))) {
            page = h.read(at(page, 0x10));
        }
        let mut b = p.wrapping_sub(0x10);
        let mut sz: u32 = h.read(at(b, 8));
        let prev = b.wrapping_sub(h.read(at(b, 0xc)));
        let next = at(b, sz);
        if prev != 0 && h.read::<u32>(prev) != 0 {
            sz = sz.wrapping_add(h.read(at(prev, 8)));
            b = prev;
            h.write(at(prev, 8), sz);
            h.write(at(next, 0xc), sz);
            let (pp, pn): (u32, u32) = (h.read(prev), h.read(at(prev, 4)));
            h.write(at(pp, 4), pn);
            h.write(pn, pp);
        }
        if h.read::<u32>(next) != 0 {
            let next_size: u32 = h.read(at(next, 8));
            sz = sz.wrapping_add(next_size);
            let after = at(next, next_size);
            if after != page.wrapping_add(h.read(at(pool, 4))) {
                h.write(at(after, 0xc), sz);
            }
            h.write(at(b, 8), sz);
            let (np, nn): (u32, u32) = (h.read(next), h.read(at(next, 4)));
            h.write(at(np, 4), nn);
            h.write(nn, np);
        }
        let old: u32 = h.read(at(page, 4));
        h.write(at(page, 4), b);
        h.write(at(b, 4), old);
        h.write(b, page);
        h.write(old, b);
        let whole = h.read::<u32>(at(pool, 4)).wrapping_sub(0x34);
        if sz != whole {
            return whole;
        }
        let mut before = 0u32;
        let mut q: u32 = h.read(pool);
        while q != page {
            before = q;
            q = h.read(at(q, 0x10));
        }
        let after: u32 = h.read(at(page, 0x10));
        if before == 0 {
            h.write(pool, after);
        } else {
            h.write(at(before, 0x10), after);
        }
        let size: u32 = h.read(at(pool, 4));
        cdecl(h, PAGE_QUEUE_DECOMMIT, &[page, size]);
        cdecl(h, PAGE_RELEASE, &[page])
    }

    /// `FUN_009439f0(pool)` (thiscall): every block of every page freed
    /// (the page's used bytes counted, then the first used block freed
    /// until none is left; the last free releases the page). A page with
    /// no used block that stays in the pool spins forever, as in the
    /// original.
    pub fn pool_free_all(h: &mut impl Host, pool: u32) {
        loop {
            let page: u32 = h.read(pool);
            if page == 0 {
                return;
            }
            let end = at(page, h.read::<u32>(at(pool, 4)).wrapping_sub(0x10));
            let first = at(page, 0x24);
            let mut used = 0u32;
            if first != end {
                let mut b = first;
                loop {
                    let n: u32 = h.read(at(b, 8));
                    if h.read::<u32>(b) == 0 {
                        used = used.wrapping_add(n);
                    }
                    b = at(b, n);
                    if b == end {
                        break;
                    }
                }
                while used != 0 {
                    let b = if h.read::<u32>(first) == 0 {
                        first
                    } else {
                        at(first, h.read(at(first, 8)))
                    };
                    used = used.wrapping_sub(h.read(at(b, 8)));
                    h.call_sym(POOL_FREE, Conv::Thiscall(pool), &[at(b, 0x10)]);
                }
            }
        }
    }

    /// `UserHeap_Free(heap; p)` (thiscall, `ret 4`).
    pub const USER_HEAP_FREE: u32 = 0x0094_3130;

    /// The first block with tag 2 (`+0xc`) from `b` on (sizes `+4`), or
    /// 0 when that is `end`.
    fn next_tagged(h: &impl Host, b: u32, end: u32) -> u32 {
        let mut d = b;
        while h.read::<u32>(at(d, 0xc)) != 2 {
            d = at(d, h.read(at(d, 4)));
        }
        if d == end { 0 } else { d }
    }

    /// `FUN_00942a80(heap)` (thiscall): every allocated block (tag 2) of
    /// every arena (`[heap + 0xc]`, blocks from `+0x1010`, the arena size
    /// `[heap]`) freed through `UserHeap_Free` until the heap has no
    /// arena left.
    pub fn heap_free_all(h: &mut impl Host, heap: u32) {
        loop {
            let arena: u32 = h.read(at(heap, 0xc));
            if arena == 0 {
                return;
            }
            let end = at(arena, h.read::<u32>(heap).wrapping_sub(0x10));
            let mut c = next_tagged(h, at(arena, 0x1010), end);
            while c != 0 {
                let end = at(arena, h.read::<u32>(heap).wrapping_sub(0x10));
                let s = next_tagged(h, at(c, h.read(at(c, 4))), end);
                h.call_sym(USER_HEAP_FREE, Conv::Thiscall(heap), &[at(c, 0x10)]);
                c = s;
            }
        }
    }

    /// `FUN_00942630(cs)` (thiscall): the heap at `cs + 0x18` emptied
    /// ([`heap_free_all`]), then `DeleteCriticalSection(cs)`. Returns its
    /// `eax`.
    pub fn heap_destroy(h: &mut impl Host, cs: u32) -> u32 {
        h.call_sym(HEAP_FREE_ALL, Conv::Thiscall(at(cs, 0x18)), &[]);
        stdcall_import(h, DELETE_CS, &[cs])
    }

    /// `FUN_00942680(cs)` (thiscall): the pool at `cs + 0x18` emptied
    /// ([`pool_free_all`]), then `DeleteCriticalSection(cs)`. Returns its
    /// `eax`.
    pub fn pool_destroy(h: &mut impl Host, cs: u32) -> u32 {
        h.call_sym(POOL_FREE_ALL, Conv::Thiscall(at(cs, 0x18)), &[]);
        stdcall_import(h, DELETE_CS, &[cs])
    }

    /// `FUN_00944080(alloc; size, align, pool)` (thiscall, `ret 0xc`): a
    /// fixed-size allocator over `pool` (a critical section first): `+0x18`
    /// the item size, `+0x1c` the chunk size (at least 4 KB), `+0x20` the
    /// aligned span, `+0x24` the chunk's item count (`align / size`),
    /// `+0x28` items per page (`chunk / size`), `+0x2c` no chunks, `+0x30`
    /// a 4-word chunk table (from the pool, the last word -1) at `+0x34`,
    /// `+0x38` no large list, `+0x3c` the pool. Returns `alloc`. (The
    /// original divides by `size`: 0 faults there; the port's quotient
    /// is then 0.)
    pub fn alloc_init(h: &mut impl Host, a: u32, size: u32, align: u32, pool: u32) -> u32 {
        stdcall_import(h, INITIALIZE_CS, &[a]);
        h.write(at(a, 0x18), size);
        let chunk = if size.cast_signed() > 0x1000 {
            size
        } else {
            0x1000
        };
        h.write(at(a, 0x1c), chunk);
        let m = size.wrapping_sub(1);
        h.write(at(a, 0x20), m.wrapping_add(align) & !m);
        let div = |x: u32| {
            x.cast_signed()
                .checked_div(size.cast_signed())
                .unwrap_or(0)
                .cast_unsigned()
        };
        h.write(at(a, 0x2c), 0u32);
        h.write(at(a, 0x30), 4u32);
        h.write(at(a, 0x38), 0u32);
        h.write(at(a, 0x24), div(align));
        h.write(at(a, 0x3c), pool);
        h.write(at(a, 0x28), div(chunk));
        stdcall_import(h, ENTER_CS, &[pool]);
        let table = h.call_sym(POOL_ALLOC, Conv::Thiscall(at(pool, 0x18)), &[0x10]);
        stdcall_import(h, LEAVE_CS, &[pool]);
        h.write(at(a, 0x34), table);
        for o in [0, 4, 8, 0xc] {
            h.write(at(table, o), 0u32);
        }
        let table: u32 = h.read(at(a, 0x34));
        let n: u32 = h.read(at(a, 0x30));
        h.write(at(table, n.wrapping_mul(4).wrapping_sub(4)), u32::MAX);
        a
    }

    /// `pool_free` on the allocator's pool under its critical section.
    fn pool_free_locked(h: &mut impl Host, pool: u32, p: u32) -> u32 {
        stdcall_import(h, ENTER_CS, &[pool]);
        h.call_sym(POOL_FREE, Conv::Thiscall(at(pool, 0x18)), &[p]);
        stdcall_import(h, LEAVE_CS, &[pool])
    }

    /// `FUN_00944190(alloc)` (thiscall): the allocator torn down: its
    /// chunks removed ([`chunk_remove`]), its chunk table and its large
    /// list's nodes (`+0x38`, linked by `+0`) given back to the pool, the
    /// critical section deleted. Returns `DeleteCriticalSection`'s `eax`.
    pub fn alloc_destroy(h: &mut impl Host, a: u32) -> u32 {
        loop {
            let chunk: u32 = h.read(at(a, 0x2c));
            if chunk == 0 {
                break;
            }
            h.call_sym(CHUNK_REMOVE, Conv::Thiscall(a), &[chunk]);
        }
        let pool: u32 = h.read(at(a, 0x3c));
        let table: u32 = h.read(at(a, 0x34));
        pool_free_locked(h, pool, table);
        let mut node: u32 = h.read(at(a, 0x38));
        while node != 0 {
            let pool: u32 = h.read(at(a, 0x3c));
            let next: u32 = h.read(node);
            pool_free_locked(h, pool, node);
            node = next;
        }
        stdcall_import(h, DELETE_CS, &[a])
    }

    /// `PageBucket_lockedCall(this)` (thiscall).
    pub const BUCKET_LOCKED_CALL: u32 = 0x0094_8820;

    /// `FUN_00944660(alloc; chunk)` (thiscall, `ret 4`): `chunk` (`+0` its
    /// memory, `+8` its bitmap, `+0x18` the next chunk) taken out of the
    /// chunk table and the chunk list; each page of its memory with a set
    /// bit in its bitmap range queued for decommit; the memory released,
    /// the bucket's locked call made, the chunk given back to the pool.
    /// Returns `LeaveCriticalSection`'s `eax`.
    pub fn chunk_remove(h: &mut impl Host, a: u32, chunk: u32) -> u32 {
        let base: u32 = h.read(chunk);
        let table: u32 = h.read(at(a, 0x34));
        let mut i = 0u32;
        while h.read::<u32>(at(table, 4 * i)) != base {
            i = i.wrapping_add(1);
        }
        h.write(at(table, 4 * i), 0u32);
        let head: u32 = h.read(at(a, 0x2c));
        let next: u32 = h.read(at(chunk, 0x18));
        if head == chunk {
            h.write(at(a, 0x2c), next);
        } else {
            let mut c = head;
            while h.read::<u32>(at(c, 0x18)) != chunk {
                c = h.read(at(c, 0x18));
            }
            h.write(at(c, 0x18), next);
        }
        let mut addr = base;
        let mut k = 0u32;
        while k.cast_signed() < h.read::<i32>(at(a, 0x24)) {
            let per: u32 = h.read(at(a, 0x28));
            if per.cast_signed() > 0 {
                let bits: u32 = h.read(at(chunk, 8));
                let mut bit = k & !per.wrapping_sub(1);
                for _ in 0..per {
                    let word = (bit.cast_signed() / 32).cast_unsigned();
                    if h.read::<u32>(at(bits, word.wrapping_mul(4))) & (1 << (bit & 31)) != 0 {
                        let span: u32 = h.read(at(a, 0x1c));
                        cdecl(h, PAGE_QUEUE_DECOMMIT, &[addr, span]);
                        break;
                    }
                    bit = bit.wrapping_add(1);
                }
            }
            k = k.wrapping_add(h.read(at(a, 0x28)));
            addr = addr.wrapping_add(h.read(at(a, 0x1c)));
        }
        let base: u32 = h.read(chunk);
        cdecl(h, PAGE_RELEASE, &[base]);
        let pool: u32 = h.read(at(a, 0x3c));
        h.call_sym(BUCKET_LOCKED_CALL, Conv::Thiscall(at(chunk, 4)), &[]);
        pool_free_locked(h, pool, chunk)
    }

    /// `FUN_00944b40(it; table)` (thiscall, `ret 4`): an iterator before
    /// the first slot of `table` (`+4` -1, `+8` the slots less 4), or at
    /// the end for an empty table. Returns `it`.
    pub fn iter_init(h: &mut impl Host, it: u32, table: u32) -> u32 {
        h.write(it, table);
        if h.read::<u32>(table) == 0 {
            h.write(at(it, 4), 0u32);
            h.write(at(it, 8), 0u32);
        } else {
            h.write(at(it, 4), u32::MAX);
            let slots: u32 = h.read(at(table, 4));
            h.write(at(it, 8), slots.wrapping_sub(4));
        }
        it
    }

    /// `VirtualFree_Decommit(addr, size)`, `VirtualFree_Release(addr,
    /// size)` (cdecl).
    pub const VIRTUAL_DECOMMIT: u32 = 0x0094_38e0;
    pub const VIRTUAL_RELEASE: u32 = 0x0094_3940;

    /// `FUN_00944f00(table)` (thiscall): the table's slot array (`+4`,
    /// `[+0]` slots) and its overflow pages (`+0x10`, linked by `+0`)
    /// decommitted and released, the byte count `+0x18` lowered, the
    /// table emptied. Returns the last `VirtualFree_Release`'s `eax`.
    pub fn table_clear(h: &mut impl Host, t: u32) -> u32 {
        let n: u32 = h.read(t);
        let slots: u32 = h.read(at(t, 4));
        let size = n.wrapping_mul(4).wrapping_add(0xfff) & 0xffff_f000;
        cdecl(h, VIRTUAL_DECOMMIT, &[slots, size]);
        let mut r = cdecl(h, VIRTUAL_RELEASE, &[slots, size]);
        let bytes: u32 = h.read(at(t, 0x18));
        h.write(at(t, 0x18), bytes.wrapping_sub(size));
        let mut page: u32 = h.read(at(t, 0x10));
        for o in [0, 4, 8] {
            h.write(at(t, o), 0u32);
        }
        while page != 0 {
            let next: u32 = h.read(page);
            cdecl(h, VIRTUAL_DECOMMIT, &[page, 0x1000]);
            r = cdecl(h, VIRTUAL_RELEASE, &[page, 0x1000]);
            let bytes: u32 = h.read(at(t, 0x18));
            h.write(at(t, 0x18), bytes.wrapping_sub(0x1000));
            page = next;
        }
        h.write(at(t, 0x14), 0u32);
        h.write(at(t, 0x10), 0u32);
        r
    }

    /// `PageHash_find(table; &key, &out)` (thiscall, `ret 8`) -> `al`,
    /// `PageHash_remove(table; &key)` (thiscall, `ret 4`).
    pub const HASH_FIND: u32 = 0x0094_55d0;
    pub const HASH_REMOVE: u32 = 0x0094_53d0;

    /// The large allocation table's hash of `p`.
    #[must_use]
    pub fn hash(p: u32) -> u32 {
        let a = ((p ^ 0x003d_0000) >> 16 ^ p).wrapping_mul(9);
        let b = (a >> 4 ^ a).wrapping_mul(0x27d4_eb2d);
        b >> 15 ^ b
    }

    /// `FUN_00944f80(big; p)` (thiscall, `ret 4`) -> `al`: the large
    /// allocation `p` looked up (under `big`'s critical section) in the
    /// table at `+0x18` by its key (the original's frame `key`, `ebp -
    /// 0x1c`: `p`, its hash, then the original's copy of `big`, then the
    /// size the lookup fills); when found it is queued for decommit (its
    /// size in whole pages), released and removed. Returns whether it
    /// was found.
    pub fn big_free(h: &mut impl Host, big: u32, p: u32, key: u32) -> u32 {
        stdcall_import(h, ENTER_CS, &[big]);
        h.write(at(key, 4), hash(p));
        h.write(at(key, 8), big);
        h.write(at(key, 0xc), 0u32);
        h.write(key, p);
        let table = at(big, 0x18);
        let found = h.call_sym(HASH_FIND, Conv::Thiscall(table), &[key, at(key, 0xc)]) & 0xff != 0;
        if found {
            let size = h.read::<u32>(at(key, 0xc)).wrapping_add(0xfff) & 0xffff_f000;
            cdecl(h, PAGE_QUEUE_DECOMMIT, &[p, size]);
            cdecl(h, PAGE_RELEASE, &[p]);
            h.call_sym(HASH_REMOVE, Conv::Thiscall(table), &[key]);
        }
        stdcall_import(h, LEAVE_CS, &[big]);
        u32::from(found)
    }

    /// `FUN_00944ba0(big)` (thiscall): every large allocation in the
    /// table (`+0x18`: `+0` slots, `+4` the slot array, a used slot
    /// pointing at its key) freed ([`big_free`] on the key's address), the
    /// table cleared, the critical section deleted. The walk keeps the
    /// slot array's first address, as the original does. Returns
    /// `DeleteCriticalSection`'s `eax`.
    pub fn big_destroy(h: &mut impl Host, big: u32) -> u32 {
        let table = at(big, 0x18);
        let (mut i, mut cursor) = if h.read::<u32>(table) == 0 {
            (0u32, 0u32)
        } else {
            (u32::MAX, h.read::<u32>(at(table, 4)).wrapping_sub(4))
        };
        'walk: loop {
            let n: u32 = h.read(table);
            if i == n {
                break;
            }
            i = i.wrapping_add(1);
            cursor = cursor.wrapping_add(4);
            loop {
                if i == n {
                    break 'walk;
                }
                let slots: u32 = h.read(at(table, 4));
                if h.read::<u32>(at(slots, i.wrapping_mul(4))) != 0 {
                    break;
                }
                i = i.wrapping_add(1);
                cursor = cursor.wrapping_add(4);
            }
            let entry: u32 = h.read(cursor);
            let p: u32 = h.read(entry);
            h.call_sym(BIG_FREE, Conv::Thiscall(big), &[p]);
        }
        h.call_sym(TABLE_CLEAR, Conv::Thiscall(table), &[]);
        stdcall_import(h, DELETE_CS, &[big])
    }

    /// `jmp_00947cc0_009436b0()`, `SetFields_00945750(desc)` (thiscall),
    /// `FUN_009457b0(sys; desc)` (thiscall, `ret 4`),
    /// `PageManager_startThread()` (cdecl); `g_MemSystem` and its object.
    pub const MEM_PRE_INIT: u32 = 0x0094_36b0;
    pub const DESC_INIT: u32 = 0x0094_5750;
    pub const SYSTEM_INIT: u32 = 0x0094_57b0;
    pub const START_THREAD: u32 = 0x0094_7630;
    pub const MEM_SYSTEM: u32 = 0x0193_cd2c;
    pub const MEM_SYSTEM_OBJECT: u32 = 0x0193_c390;

    /// `FUN_009428b0()` (cdecl): `MemSystem_Ensure`'s slow path. Without
    /// `g_MemSystem`: the page manager's set-up, a descriptor in the
    /// original's frame (`desc`, `ebp - 0x44`, 0x30 bytes filled by
    /// `SetFields_00945750`, then two words = the object), the object at
    /// `0x0193c390` initialised from it, `g_MemSystem` set, the page
    /// thread started.
    pub fn mem_system_ensure(h: &mut impl Host, desc: u32) {
        let sys = h.rebase(MEM_SYSTEM);
        if h.read::<u32>(sys) != 0 {
            return;
        }
        cdecl(h, MEM_PRE_INIT, &[]);
        h.call_sym(DESC_INIT, Conv::Thiscall(desc), &[]);
        let object = h.rebase(MEM_SYSTEM_OBJECT);
        h.write(at(desc, 0x34), object);
        h.write(at(desc, 0x30), object);
        h.call_sym(SYSTEM_INIT, Conv::Thiscall(object), &[desc]);
        h.write(sys, object);
        cdecl(h, START_THREAD, &[]);
    }
}
