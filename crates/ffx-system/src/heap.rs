//! The PC port's own heap (`user_malloc` / `user_free`) and the page
//! manager under it, as compiled into `FFX.exe`.
//!
//! **The heap** (`*g_UserHeap`): a `CRITICAL_SECTION` (24 bytes), then the
//! allocator at `+0x18`:
//! - `+0x00` arena size, `+0x04`/`+0x08` the smallest and largest free-block
//!   size the bins cover, `+0x0c` the first arena (arenas chain through
//!   `+0xff0`), `+0x10` the free-node pool ([`node_alloc`]).
//! - An arena starts with 255 bin heads of 16 bytes (circular lists of
//!   free nodes, `+8` next, `+4` previous); its big free block's node is at
//!   `+0x101c`.
//! - A block has a 16-byte header before the user pointer: `+4` size
//!   (header included), `+8` distance back to the previous block, `+0xc`
//!   its free node, or 0..=2 while in use (3: free without a node).
//! - A free node: block, previous, next, size. Bins 0..=253 keep nodes by
//!   size (then address); bin 254 by address.
//!
//! **The page manager** (`*g_PageManager`): 256 lists (28 bytes each) of
//! released reservations kept for reuse, by size in MiB; `+0x1c00` bytes
//! waiting to be decommitted, `+0x1c04` reserved but unused, `+0x1c08`
//! cached; a `CRITICAL_SECTION` at `+0x1c0c`; a hash table of live
//! reservations at `+0x1c24`; node pools at `+0x1c40` (cache nodes, 28
//! bytes) and `+0x1c4c` (queue nodes, 20 bytes); the decommit queue's
//! sentinel at `+0x1c58` (sorted by address; `+0xc` next, `+8` previous);
//! the frame stamp at `+0x1c6c`. Freed pages are queued, not decommitted:
//! [`flush`] releases what is older than the stamp.
//!
//! Every Win32 call goes through [`Os`]: the game's critical sections and
//! `VirtualAlloc`/`VirtualFree`, so a host can run these over its own
//! memory. The functions follow the originals' reads and writes in order.

// t, n, i, s, ...: the table, node and index names of the originals
#![allow(clippy::many_single_char_names)]

use ffx_bridge::{Global, Mem};

/// The Win32 calls the heap makes.
pub trait Os {
    /// `EnterCriticalSection(cs)`.
    fn enter_critical_section(&mut self, cs: u32);
    /// `LeaveCriticalSection(cs)`.
    fn leave_critical_section(&mut self, cs: u32);
    /// `VirtualAlloc(addr, size, alloc_type, protect)`: the address, or 0.
    fn virtual_alloc(&mut self, addr: u32, size: u32, alloc_type: u32, protect: u32) -> u32;
    /// `VirtualFree(addr, size, free_type)`: non-zero on success.
    fn virtual_free(&mut self, addr: u32, size: u32, free_type: u32) -> u32;
    /// After a failed `VirtualFree`: the originals format `GetLastError()`'s
    /// message (`FormatMessageA`, allocating) and pass it to the error hook
    /// at `0x0193cd4c` when one is set.
    fn report_free_failure(&mut self);
    /// `InitializeCriticalSection(cs)`.
    fn initialize_critical_section(&mut self, cs: u32);
    /// The C runtime's `malloc(size)` (import `0x00b0c468`).
    fn crt_malloc(&mut self, size: u32) -> u32;
    /// The C runtime's `free(p)` (import `0x00b0c370`).
    fn crt_free(&mut self, p: u32);
    /// The memory system's first set-up (`0x009428b0`, what `0x009428a0`
    /// calls while [`MEM_SYSTEM`] is null).
    fn mem_system_init(&mut self);
    /// `WaitForSingleObject(handle, ms)`.
    fn wait_for_single_object(&mut self, handle: u32, ms: u32) -> u32;
    /// `ReleaseMutex(handle)`.
    fn release_mutex(&mut self, handle: u32) -> u32;
}

/// The memory system (`0x0193cd2c`: the object at `0x0193c390` once
/// `0x009428b0` has set it up).
pub const MEM_SYSTEM: Global<u32> = Global::new(0x0193_cd2c);
/// The first block `malloc_pool_init` takes (`0x00ce9020`).
pub const FIRST_BLOCK: Global<u32> = Global::new(0x00ce_9020);
/// The arena size `malloc_pool_init` settled on (`0x00ce9024`).
pub const ARENA_SIZE: Global<u32> = Global::new(0x00ce_9024);

/// `0x009428a0()`: the memory system set up unless it already is.
pub fn mem_system_ensure(mem: &impl Mem, os: &mut impl Os) {
    if MEM_SYSTEM.get(mem) == 0 {
        os.mem_system_init();
    }
}

/// `UserHeap_Init(arena, a, b)` (`0x00942a40`, `thiscall`, `ret 0xc`): the
/// arena size rounded up to 4 KiB, the two limits, no arenas yet, the
/// 16-byte minimum, no free list. Returns `heap`.
pub fn heap_init(mem: &mut impl Mem, heap: u32, arena: u32, a: u32, b: u32) -> u32 {
    wr(mem, heap, arena.wrapping_add(0xfff) & 0xffff_f000);
    wr(mem, heap + 4, a);
    wr(mem, heap + 8, b);
    wr(mem, heap + 0xc, 0);
    wr(mem, heap + 0x10, 0x10);
    wr(mem, heap + 0x14, 0);
    heap
}

/// `malloc_pool_init()` (`0x006fb9b0`, behind the `0x00687040` jump): the
/// memory system, then a heap object from the C runtime (0x30 bytes: its
/// critical section, the allocator at `+0x18` with 0x30000000-byte arenas
/// and a 0x2ff00000 limit) and a first 16-byte block; while that block
/// fails, the object is freed and the arena size drops by 16 MiB.
pub fn malloc_pool_init(mem: &mut impl Mem, os: &mut impl Os) {
    mem_system_ensure(mem, os);
    let mut arena = 0x3000_0000u32;
    loop {
        let mut heap = os.crt_malloc(0x30);
        USER_HEAP.set(mem, heap);
        if heap != 0 {
            os.initialize_critical_section(heap);
            heap_init(mem, heap + 0x18, arena, 0, arena.wrapping_sub(0x10_0000));
        } else {
            heap = 0;
        }
        USER_HEAP.set(mem, heap);
        os.enter_critical_section(heap);
        let first = heap_alloc(mem, os, heap + 0x18, 0x10);
        os.leave_critical_section(heap);
        FIRST_BLOCK.set(mem, first);
        if first != 0 {
            ARENA_SIZE.set(mem, arena);
            return;
        }
        os.crt_free(USER_HEAP.get(mem));
        arena = arena.wrapping_sub(0x100_0000);
    }
}

/// `0x006fb9a0()` (behind the `0x00687030` jump): the heap object handed
/// back to the C runtime (the pointer is left as it was).
pub fn user_heap_destroy(mem: &impl Mem, os: &mut impl Os) {
    os.crt_free(USER_HEAP.get(mem));
}

/// The user heap object (`*g_UserHeap`: lock, then the allocator at
/// `+0x18`).
pub const USER_HEAP: Global<u32> = Global::new(0x00ce_901c);
/// The page manager (`*g_PageManager`).
pub const PAGE_MANAGER: Global<u32> = Global::new(0x00c6_13b0);
/// Bytes reserved through the wrappers.
pub const RESERVED_BYTES: Global<u32> = Global::new(0x0193_cd44);
/// Bytes committed through the wrappers.
pub const COMMITTED_BYTES: Global<u32> = Global::new(0x0193_cd48);

const MEM_COMMIT: u32 = 0x1000;
const MEM_RESERVE: u32 = 0x2000;
const MEM_DECOMMIT: u32 = 0x4000;
const MEM_RELEASE: u32 = 0x8000;
const PAGE_NOACCESS: u32 = 1;
const PAGE_READWRITE: u32 = 4;

fn rd(mem: &impl Mem, a: u32) -> u32 {
    mem.read(a)
}

fn wr(mem: &mut impl Mem, a: u32, v: u32) {
    mem.write(a, v);
}

/// `*a += v` (wrapping).
fn add(mem: &mut impl Mem, a: u32, v: u32) {
    let x = rd(mem, a).wrapping_add(v);
    wr(mem, a, x);
}

/// `*a -= v` (wrapping).
fn sub(mem: &mut impl Mem, a: u32, v: u32) {
    let x = rd(mem, a).wrapping_sub(v);
    wr(mem, a, x);
}

/// A `bool` the originals return in `al`.
fn al(eax: u32) -> bool {
    eax.to_le_bytes()[0] != 0
}

fn word(p: u32, i: u32) -> u32 {
    p.wrapping_add(4 * i)
}

// ---- the VirtualAlloc / VirtualFree wrappers ----

/// `VirtualAlloc_Commit_RW(addr, size)` (`0x009438b0`): commits read-write;
/// counts the bytes when it worked. `al` is the result; the rest of `eax`
/// is `VirtualAlloc`'s.
pub fn commit_rw(mem: &mut impl Mem, os: &mut impl Os, addr: u32, size: u32) -> u32 {
    let r = os.virtual_alloc(addr, size, MEM_COMMIT, PAGE_READWRITE);
    if r != 0 {
        let at = COMMITTED_BYTES.ptr(mem).addr;
        add(mem, at, size);
    }
    r & !0xff | u32::from(r != 0)
}

/// `VirtualFree_Decommit(addr, size)` (`0x009438e0`).
pub fn decommit(mem: &mut impl Mem, os: &mut impl Os, addr: u32, size: u32) {
    if os.virtual_free(addr, size, MEM_DECOMMIT) == 0 {
        os.report_free_failure();
    }
    let at = COMMITTED_BYTES.ptr(mem).addr;
    sub(mem, at, size);
}

/// `VirtualFree_Release(addr, size)` (`0x00943940`): releases the whole
/// reservation at `addr`; `size` is only counted. Returns `size` (left in
/// `eax`).
pub fn release(mem: &mut impl Mem, os: &mut impl Os, addr: u32, size: u32) -> u32 {
    if os.virtual_free(addr, 0, MEM_RELEASE) == 0 {
        os.report_free_failure();
    }
    let at = RESERVED_BYTES.ptr(mem).addr;
    sub(mem, at, size);
    size
}

/// `VirtualAlloc_Reserve_NA(size)` (`0x009439a0`): reserves no-access
/// anywhere; the address or 0.
pub fn reserve(mem: &mut impl Mem, os: &mut impl Os, size: u32) -> u32 {
    let r = os.virtual_alloc(0, size, MEM_RESERVE, PAGE_NOACCESS);
    if r != 0 {
        let at = RESERVED_BYTES.ptr(mem).addr;
        add(mem, at, size);
    }
    r
}

// ---- node pools of the page manager ----

/// The page-manager node pools (`0x00947350`, 20-byte nodes;
/// `0x009473c0`, 28-byte nodes; `this` = free head, page list, page
/// count): a node, after taking a fresh zeroed page when the free list is
/// empty.
fn pool_take(mem: &mut impl Mem, os: &mut impl Os, pool: u32, stride: u32) -> u32 {
    if rd(mem, pool) == 0 {
        let page = reserve(mem, os, 0x1000);
        commit_rw(mem, os, page, 0x1000);
        mem.write(page, [0u32; 0x400]);
        let pages = rd(mem, word(pool, 1));
        wr(mem, page, pages);
        add(mem, word(pool, 2), 1);
        let first = page.wrapping_add(stride);
        wr(mem, word(pool, 1), page);
        let end = page.wrapping_add(0xfdc);
        let mut p = first;
        while p != end {
            let n = p.wrapping_add(stride);
            wr(mem, p, n);
            p = n;
        }
        wr(mem, pool, first);
    }
    let node = rd(mem, pool);
    let next = rd(mem, node);
    wr(mem, pool, next);
    node
}

/// `0x00947350` (fastcall, the pool in ECX): a 20-byte decommit-queue node.
pub fn alloc_queue_node(mem: &mut impl Mem, os: &mut impl Os, pool: u32) -> u32 {
    pool_take(mem, os, pool, 0x14)
}

/// `0x009473c0` (fastcall, the pool in ECX): a 28-byte cache node.
pub fn alloc_cache_node(mem: &mut impl Mem, os: &mut impl Os, pool: u32) -> u32 {
    pool_take(mem, os, pool, 0x1c)
}

// ---- the reservation hash table (page manager + 0x1c24) ----
//
// `+0` capacity (a power of two), `+4` slots, `+8` count, `+0x10` node
// pages, `+0x14` free nodes, `+0x18` bytes committed for it. A node:
// address, hash, then four words (size, reserved size, base, alignment).

/// The hash `0x00948130` and `0x00947ec0` key the table with.
#[must_use]
pub const fn address_hash(a: u32) -> u32 {
    let h = ((a ^ 0x003d_0000) >> 16 ^ a).wrapping_mul(9);
    let h = (h >> 4 ^ h).wrapping_mul(0x27d4_eb2d);
    h >> 15 ^ h
}

/// `0x00947430` (fastcall, the table in ECX): a 24-byte node, cleared
/// link; a fresh page of 169 nodes when none is free.
pub fn alloc_hash_node(mem: &mut impl Mem, os: &mut impl Os, t: u32) -> u32 {
    if rd(mem, t + 0x14) == 0 {
        let page = reserve(mem, os, 0x1000);
        commit_rw(mem, os, page, 0x1000);
        add(mem, t + 0x18, 0x1000);
        let pages = rd(mem, t + 0x10);
        wr(mem, page, pages);
        let mut p = page.wrapping_add(0x18);
        wr(mem, t + 0x10, page);
        wr(mem, t + 0x14, p);
        for _ in 0..0xa8 {
            let n = p.wrapping_add(0x18);
            wr(mem, p, n);
            p = n;
        }
        wr(mem, p, 0);
    }
    let node = rd(mem, t + 0x14);
    let next = rd(mem, node);
    wr(mem, t + 0x14, next);
    if node != 0 {
        wr(mem, node, 0);
    }
    node
}

/// `0x009482c0` (`thiscall(table, n)`, `ret 4`): grows the slots to the
/// smallest power of two from 2 up that is at least `n`, reinserting every
/// node; the old slots are decommitted and released.
pub fn hash_rehash(mem: &mut impl Mem, os: &mut impl Os, t: u32, n: i32) {
    let mut cap: i32 = 2;
    if n > 2 {
        while cap < n {
            cap = cap.wrapping_add(cap);
        }
    }
    let old_cap = rd(mem, t).cast_signed();
    let old = rd(mem, t + 4);
    wr(mem, t, cap.cast_unsigned());
    if cap > 0 {
        let bytes = cap.cast_unsigned().wrapping_mul(4);
        let size = bytes.wrapping_add(0xfff) & 0xffff_f000;
        let p = reserve(mem, os, size);
        commit_rw(mem, os, p, size);
        add(mem, t + 0x18, size);
        wr(mem, t + 4, p);
        for i in 0..bytes / 4 {
            wr(mem, word(p, i), 0);
        }
    }
    if old_cap > 0 {
        for k in 0..old_cap.cast_unsigned() {
            let e = rd(mem, word(old, k));
            if e == 0 {
                continue;
            }
            let mask = rd(mem, t).wrapping_sub(1);
            let slots = rd(mem, t + 4);
            let mut i = rd(mem, e + 4) & mask;
            let key = rd(mem, e);
            loop {
                let s = rd(mem, word(slots, i));
                if s == 0 || rd(mem, s) == key {
                    break;
                }
                i = i.wrapping_add(1) & mask;
            }
            wr(mem, word(slots, i), e);
        }
    }
    if old != 0 {
        let size = old_cap.cast_unsigned().wrapping_mul(4).wrapping_add(0xfff) & 0xffff_f000;
        decommit(mem, os, old, size);
        release(mem, os, old, size);
        sub(mem, t + 0x18, size);
    }
}

/// `0x00947270` (`thiscall(table, &key, &data)`, `ret 8`), with the key
/// (address, hash) and the four data words as values: sets `key`'s data,
/// adding a node when it is new; grows the table first when empty (to
/// 1024) or when the count is 0x66/256 of the capacity.
pub fn hash_insert(mem: &mut impl Mem, os: &mut impl Os, t: u32, key: [u32; 2], data: [u32; 4]) {
    let cap = rd(mem, t).cast_signed();
    if cap == 0 {
        hash_rehash(mem, os, t, 0x400);
    } else if rd(mem, t + 8).cast_signed() == cap.wrapping_mul(0x66) / 256 {
        hash_rehash(mem, os, t, cap.wrapping_add(cap));
    }
    let mask = rd(mem, t).wrapping_sub(1);
    let slots = rd(mem, t + 4);
    let mut i = key[1] & mask;
    loop {
        let s = rd(mem, word(slots, i));
        if s == 0 || rd(mem, s) == key[0] {
            break;
        }
        i = i.wrapping_add(1) & mask;
    }
    let found = rd(mem, word(slots, i));
    if found != 0 {
        mem.write(found + 8, data);
        return;
    }
    let node = alloc_hash_node(mem, os, t);
    mem.write(node, key);
    mem.write(node + 8, data);
    let slots = rd(mem, t + 4);
    wr(mem, word(slots, i), node);
    add(mem, t + 8, 1);
}

/// [`hash_insert`] with the key and data in game memory, as the original
/// takes them.
pub fn hash_insert_at(mem: &mut impl Mem, os: &mut impl Os, t: u32, key: u32, data: u32) {
    let (k, d) = (mem.read(key), mem.read(data));
    hash_insert(mem, os, t, k, d);
}

/// `0x00948000` (`thiscall(table, &out, &key)`, `ret 8`), with the key as
/// a value: removes `key` (which must be present) and returns its four data
/// words; the cluster after it is closed up (linear probing's backward
/// shift).
pub fn hash_remove(mem: &mut impl Mem, t: u32, key: [u32; 2]) -> [u32; 4] {
    let cap = rd(mem, t);
    let mask = cap.wrapping_sub(1);
    let slots = rd(mem, t + 4);
    let addr = key[0];
    let mut i = key[1] & mask;
    while rd(mem, word(slots, i)) != 0 {
        let s = rd(mem, word(slots, i));
        if rd(mem, s) == addr {
            break;
        }
        i = i.wrapping_add(1) & mask;
    }
    let node = rd(mem, word(slots, i));
    let d: [u32; 4] = mem.read(node + 8);
    // the first slot of the cluster
    let mut start = i;
    let mut j = if i == 0 { cap } else { i }.wrapping_sub(1);
    while rd(mem, word(slots, j)) != 0 {
        start = j;
        j = if j == 0 { cap } else { j }.wrapping_sub(1);
    }
    let (s, mut i) = (start.cast_signed(), i.cast_signed());
    loop {
        let mask = rd(mem, t).wrapping_sub(1);
        let slots = rd(mem, t + 4);
        let mut k = (i.cast_unsigned().wrapping_add(1) & mask).cast_signed();
        let mut best = -1i32;
        let mut found = false;
        while rd(mem, word(slots, k.cast_unsigned())) != 0 {
            let e = rd(mem, word(slots, k.cast_unsigned()));
            let h = (rd(mem, e + 4) & mask).cast_signed();
            let movable = if s <= i {
                s <= h && h <= i
            } else {
                h >= s || h <= i
            };
            if movable {
                best = k;
                found = true;
            }
            k = (k.cast_unsigned().wrapping_add(1) & mask).cast_signed();
        }
        if !found {
            break;
        }
        let e = rd(mem, word(slots, best.cast_unsigned()));
        wr(mem, word(slots, i.cast_unsigned()), e);
        i = best;
    }
    let slots = rd(mem, t + 4);
    wr(mem, word(slots, i.cast_unsigned()), 0);
    let free = rd(mem, t + 0x14);
    wr(mem, node, free);
    sub(mem, t + 8, 1);
    wr(mem, t + 0x14, node);
    d
}

/// [`hash_remove`] with the key and the result in game memory, as the
/// original: returns `out`.
pub fn hash_remove_at(mem: &mut impl Mem, t: u32, out: u32, key: u32) -> u32 {
    let k = mem.read(key);
    let d = hash_remove(mem, t, k);
    mem.write(out, d);
    out
}

// ---- the page manager ----

/// `mgr + 0x1c0c`.
const fn lock(mgr: u32) -> u32 {
    mgr.wrapping_add(0x1c0c)
}

/// `0x009478e0` (`thiscall(mgr, force)`, `ret 4`): decommits the queued
/// ranges and releases the cached reservations whose stamp the frame stamp
/// has reached, or all of them when `force` (or when the stamp is 0).
pub fn flush(mem: &mut impl Mem, os: &mut impl Os, mgr: u32, force: u8) {
    os.enter_critical_section(lock(mgr));
    let force = rd(mem, mgr + 0x1c6c) == 0 || force != 0;
    let sentinel = mgr + 0x1c58;
    let mut n = rd(mem, mgr + 0x1c64);
    while n != sentinel {
        let next = rd(mem, n + 0xc);
        if rd(mem, mgr + 0x1c6c).cast_signed() >= rd(mem, n + 0x10).cast_signed() || force {
            let size = rd(mem, n + 4);
            let base = rd(mem, n);
            decommit(mem, os, base, size);
            sub(mem, mgr + 0x1c00, size);
            unlink_queue(mem, n);
            let free = rd(mem, mgr + 0x1c4c);
            wr(mem, n, free);
            wr(mem, mgr + 0x1c4c, n);
        }
        n = next;
    }
    for b in 0..0x100 {
        let head = mgr + b * 0x1c;
        let mut e = rd(mem, head + 0x10);
        while e != head {
            let next = rd(mem, e + 0x10);
            if rd(mem, mgr + 0x1c6c).cast_signed() >= rd(mem, e + 0x18).cast_signed() || force {
                let reserved = rd(mem, e + 4);
                let size = rd(mem, e);
                let base = rd(mem, e + 8);
                release(mem, os, base, reserved);
                sub(mem, mgr + 0x1c08, size);
                add(mem, mgr + 0x1c04, size.wrapping_sub(reserved));
                unlink_cache(mem, e);
                let free = rd(mem, mgr + 0x1c40);
                wr(mem, e, free);
                wr(mem, mgr + 0x1c40, e);
            }
            e = next;
        }
    }
    os.leave_critical_section(lock(mgr));
}

/// A queue node out of its list (`+8` previous, `+0xc` next).
fn unlink_queue(mem: &mut impl Mem, n: u32) {
    let (prev, next) = (rd(mem, n + 8), rd(mem, n + 0xc));
    wr(mem, prev + 0xc, next);
    let (next, prev) = (rd(mem, n + 0xc), rd(mem, n + 8));
    wr(mem, next + 8, prev);
}

/// A cache node out of its list (`+0xc` previous, `+0x10` next).
fn unlink_cache(mem: &mut impl Mem, n: u32) {
    let (prev, next) = (rd(mem, n + 0xc), rd(mem, n + 0x10));
    wr(mem, prev + 0x10, next);
    let (next, prev) = (rd(mem, n + 0x10), rd(mem, n + 0xc));
    wr(mem, next + 0xc, prev);
}

/// `0x00948130` (`thiscall(mgr, size, align)`, `ret 8`): `size` bytes of
/// address space aligned to `align`, from a cached reservation of the same
/// size and alignment or a new one (flushing everything and trying once
/// more when that fails); recorded in the hash table. Returns the aligned
/// address, or 0.
pub fn page_alloc(mem: &mut impl Mem, os: &mut impl Os, mgr: u32, size: u32, align: u32) -> u32 {
    os.enter_critical_section(lock(mgr));
    let mut base = 0;
    let mut reserved = size;
    if (size >> 20).cast_signed() < 0x100 {
        let head = mgr.wrapping_add((size >> 20) * 0x1c);
        let mut e = rd(mem, head + 0x10);
        while e != head {
            if rd(mem, e) == size && rd(mem, e + 0x14) == align {
                base = rd(mem, e + 8);
                unlink_cache(mem, e);
                let free = rd(mem, mgr + 0x1c40);
                wr(mem, e, free);
                wr(mem, mgr + 0x1c40, e);
                sub(mem, mgr + 0x1c08, size);
                reserved = rd(mem, e + 4);
                break;
            }
            e = rd(mem, e + 0x10);
        }
    }
    let unaligned = align != 0x1000;
    if base == 0 {
        if unaligned {
            reserved = reserved.wrapping_add(0xffe).wrapping_add(align) & 0xffff_f000;
        }
        base = reserve(mem, os, reserved);
        if base == 0 {
            flush(mem, os, mgr, 1);
            base = reserve(mem, os, reserved);
            if base == 0 {
                os.leave_critical_section(lock(mgr));
                return 0;
            }
        }
        add(mem, mgr + 0x1c04, reserved.wrapping_sub(size));
    }
    let aligned = if unaligned {
        base.wrapping_add(align.wrapping_sub(1)) & !align.wrapping_sub(1)
    } else {
        base
    };
    let key = [aligned, address_hash(aligned)];
    hash_insert(mem, os, mgr + 0x1c24, key, [size, reserved, base, align]);
    os.leave_critical_section(lock(mgr));
    aligned
}

/// `0x00947ec0` (`thiscall(mgr, addr)`, `ret 4`): gives back the
/// reservation `page_alloc` returned as `addr`: kept in its size's list
/// (stamped with the next frame) when under 256 MiB, else released.
pub fn release_pages(mem: &mut impl Mem, os: &mut impl Os, mgr: u32, addr: u32) {
    os.enter_critical_section(lock(mgr));
    let [size, reserved, base, align] = hash_remove(mem, mgr + 0x1c24, [addr, address_hash(addr)]);
    let bucket = reserved >> 20;
    if bucket.cast_signed() < 0x100 {
        let n = alloc_cache_node(mem, os, mgr + 0x1c40);
        wr(mem, n + 8, base);
        wr(mem, n + 4, reserved);
        wr(mem, n + 0x14, align);
        wr(mem, n + 0x10, 0);
        wr(mem, n + 0xc, 0);
        wr(mem, n, size);
        let stamp = rd(mem, mgr + 0x1c6c).wrapping_add(1);
        wr(mem, n + 0x18, stamp);
        let head = mgr.wrapping_add(bucket * 0x1c);
        let first = rd(mem, head + 0x10);
        wr(mem, n + 0xc, head);
        wr(mem, n + 0x10, first);
        wr(mem, head + 0x10, n);
        wr(mem, first + 0xc, n);
        add(mem, mgr + 0x1c08, size);
    } else {
        release(mem, os, base, reserved);
        add(mem, mgr + 0x1c04, size.wrapping_sub(reserved));
    }
    os.leave_critical_section(lock(mgr));
}

/// `0x00947690` (`thiscall(mgr, addr, size)`, `ret 8`): queues `[addr,
/// addr + size)` for decommitting, stamped with the next frame, before the
/// first queued range that starts at or after its end.
pub fn queue_decommit(mem: &mut impl Mem, os: &mut impl Os, mgr: u32, addr: u32, size: u32) {
    os.enter_critical_section(lock(mgr));
    add(mem, mgr + 0x1c00, size);
    let sentinel = mgr + 0x1c58;
    let end = addr.wrapping_add(size);
    let mut pos = rd(mem, mgr + 0x1c64);
    while pos != sentinel && rd(mem, pos) < end {
        pos = rd(mem, pos + 0xc);
    }
    let n = alloc_queue_node(mem, os, mgr + 0x1c4c);
    wr(mem, n, addr);
    wr(mem, n + 4, size);
    let stamp = rd(mem, mgr + 0x1c6c).wrapping_add(1);
    wr(mem, n + 0x10, stamp);
    let prev = rd(mem, pos + 8);
    let at = rd(mem, prev + 0xc);
    wr(mem, n + 8, prev);
    wr(mem, n + 0xc, at);
    wr(mem, prev + 0xc, n);
    wr(mem, at + 8, n);
    os.leave_critical_section(lock(mgr));
}

/// A queue node back to the pool at `mgr + 0x1c4c` after unlinking it.
fn drop_queue_node(mem: &mut impl Mem, mgr: u32, n: u32) {
    unlink_queue(mem, n);
    let free = rd(mem, mgr + 0x1c4c);
    wr(mem, n, free);
    wr(mem, mgr + 0x1c4c, n);
}

/// `0x009483d0` (`thiscall(mgr, &addr, &size)`, `ret 8`): commits `[addr,
/// addr + size)`, taking back from the decommit queue the parts still
/// waiting there (they are still committed) and committing the rest;
/// `addr` and `size` move as it goes. Returns whether every commit worked.
pub fn commit_range(
    mem: &mut impl Mem,
    os: &mut impl Os,
    mgr: u32,
    pa: &mut u32,
    ps: &mut u32,
) -> bool {
    let addr = *pa;
    let size = *ps;
    let end = addr.wrapping_add(size);
    let sentinel = mgr + 0x1c58;
    let mut n = rd(mem, mgr + 0x1c64);
    // the first queued range that overlaps
    loop {
        if n == sentinel {
            return al(commit_rw(mem, os, addr, size));
        }
        let s = rd(mem, n);
        let overlaps = if s <= addr {
            addr < rd(mem, n + 4).wrapping_add(s) || (s >= addr && s < end)
        } else {
            s < end
        };
        if overlaps {
            break;
        }
        n = rd(mem, n + 0xc);
    }
    let s = rd(mem, n);
    if s == addr && rd(mem, n + 4) == *ps {
        drop_queue_node(mem, mgr, n);
        let v = *ps;
        sub(mem, mgr + 0x1c00, v);
        return true;
    }
    if s < addr {
        let qend = rd(mem, n + 4).wrapping_add(s);
        let take = qend.wrapping_sub(addr).min(*ps);
        wr(mem, n + 4, addr.wrapping_sub(s));
        *pa = pa.wrapping_add(take);
        *ps = ps.wrapping_sub(take);
        sub(mem, mgr + 0x1c00, take);
        if *pa < qend {
            let m = alloc_queue_node(mem, os, mgr + 0x1c4c);
            let a = *pa;
            wr(mem, m, a);
            wr(mem, m + 4, qend.wrapping_sub(a));
            let next = rd(mem, n + 0xc);
            wr(mem, m + 8, n);
            wr(mem, m + 0xc, next);
            wr(mem, n + 0xc, m);
            wr(mem, next + 8, m);
        }
        n = rd(mem, n + 0xc);
    }
    if *ps == 0 {
        return true;
    }
    while n != sentinel {
        let next = rd(mem, n + 0xc);
        let s = rd(mem, n);
        let a = *pa;
        if a < s {
            let c = s.wrapping_sub(a).min(*ps);
            if !al(commit_rw(mem, os, a, c)) {
                return false;
            }
            *pa = pa.wrapping_add(c);
            *ps = ps.wrapping_sub(c);
        }
        let want = *ps;
        if want != 0 {
            let have = rd(mem, n + 4);
            let c = have.min(want);
            add(mem, n, c);
            wr(mem, n + 4, have.wrapping_sub(c));
            *pa = pa.wrapping_add(c);
            *ps = ps.wrapping_sub(c);
            sub(mem, mgr + 0x1c00, c);
            if rd(mem, n + 4) == 0 {
                drop_queue_node(mem, mgr, n);
            }
        }
        if *ps == 0 {
            return true;
        }
        n = next;
    }
    let want = *ps;
    if want != 0 {
        let a = *pa;
        return al(commit_rw(mem, os, a, want));
    }
    true
}

/// `0x00947590` (`thiscall(mgr, addr, size)`, `ret 8`): [`commit_range`];
/// on failure [`flush`] everything and try again from where it stopped.
pub fn commit(mem: &mut impl Mem, os: &mut impl Os, mgr: u32, addr: u32, size: u32) -> bool {
    os.enter_critical_section(lock(mgr));
    let (mut a, mut s) = (addr, size);
    let mut ok = commit_range(mem, os, mgr, &mut a, &mut s);
    if !ok {
        flush(mem, os, mgr, 1);
        ok = commit_range(mem, os, mgr, &mut a, &mut s);
    }
    os.leave_critical_section(lock(mgr));
    ok
}

/// [`commit_range`] with the address and size in game memory, as the
/// original takes them (written back at the end).
pub fn commit_range_at(mem: &mut impl Mem, os: &mut impl Os, mgr: u32, pa: u32, ps: u32) -> bool {
    let (mut a, mut s) = (rd(mem, pa), rd(mem, ps));
    let ok = commit_range(mem, os, mgr, &mut a, &mut s);
    wr(mem, pa, a);
    wr(mem, ps, s);
    ok
}

// ---- the user heap ----

/// `0x009485b0` (fastcall, the pool in ECX: node size, page list): a free
/// node, from a new page when none is left; 0 when that fails.
pub fn node_alloc(mem: &mut impl Mem, os: &mut impl Os, pool: u32) -> u32 {
    let mut page = rd(mem, pool + 4);
    if page == 0 {
        page = node_new_page(mem, os, pool);
        wr(mem, pool + 4, page);
        if page == 0 {
            return 0;
        }
    }
    let node = rd(mem, page);
    let next = rd(mem, node);
    sub(mem, page + 8, 1);
    wr(mem, page, next);
    if rd(mem, page + 8) == 0 {
        let p = rd(mem, pool + 4);
        let after = rd(mem, p + 4);
        wr(mem, pool + 4, after);
    }
    node
}

/// `0x009485f0` (fastcall, the pool in ECX): a page of nodes from the page
/// manager (`0x00948110`, `0x00947570`): `+0` free list, `+4` next page,
/// `+8` free count, nodes from `+0xc`.
pub fn node_new_page(mem: &mut impl Mem, os: &mut impl Os, pool: u32) -> u32 {
    let mgr = PAGE_MANAGER.get(mem);
    let page = page_alloc(mem, os, mgr, 0x1000, 0x1000);
    if page == 0 {
        return 0;
    }
    let mgr = PAGE_MANAGER.get(mem);
    if !commit(mem, os, mgr, page, 0x1000) {
        return 0;
    }
    let count = 0xff4 / rd(mem, pool);
    let first = page + 0xc;
    let last = count
        .wrapping_sub(1)
        .wrapping_mul(rd(mem, pool))
        .wrapping_add(first);
    let mut p = first;
    while p != last {
        let n = rd(mem, pool).wrapping_add(p);
        wr(mem, p, n);
        p = n;
    }
    wr(mem, p, 0);
    wr(mem, page, first);
    wr(mem, page + 8, count);
    wr(mem, page + 4, 0);
    page
}

/// `0x009486a0` (`thiscall(pool, node)`, `ret 4`): the node back to its
/// page; a page all free leaves the pool and goes back to the page manager
/// (decommit queued, reservation given back); a page that was full joins
/// the pool's list.
pub fn node_free(mem: &mut impl Mem, os: &mut impl Os, pool: u32, node: u32) {
    let page = node & 0xffff_f000;
    let free = rd(mem, page);
    wr(mem, node, free);
    add(mem, page + 8, 1);
    wr(mem, page, node);
    let per_page = 0xff4 / rd(mem, pool);
    let n = rd(mem, page + 8);
    if n == per_page {
        let mut before = 0;
        let mut p = rd(mem, pool + 4);
        while p != page {
            before = p;
            p = rd(mem, p + 4);
        }
        let after = rd(mem, page + 4);
        if before == 0 {
            wr(mem, pool + 4, after);
        } else {
            wr(mem, before + 4, after);
        }
        let mgr = PAGE_MANAGER.get(mem);
        queue_decommit(mem, os, mgr, page, 0x1000);
        let mgr = PAGE_MANAGER.get(mem);
        release_pages(mem, os, mgr, page);
    } else if n == 1 {
        let first = rd(mem, pool + 4);
        wr(mem, page + 4, first);
        wr(mem, pool + 4, page);
    }
}

/// `0x009432e0` (`thiscall(heap, size)`, `ret 4`): the bin of a free block:
/// `x = min(size - min, max - min)`; above a range of 0x23dc the scale is
/// bent toward small sizes (`x = r - (r - x)³ / r²`, in 64-bit steps
/// truncating toward zero); then `x * 254 / r`.
pub fn bin_index(mem: &impl Mem, heap: u32, size: u32) -> i64 {
    let lo = rd(mem, heap + 4);
    let x = i64::from(size.wrapping_sub(lo).cast_signed());
    let r = i64::from(rd(mem, heap + 8).wrapping_sub(lo).cast_signed());
    let mut x = x.min(r);
    if r > 0x23dc {
        let d = r.wrapping_sub(x);
        let t = d.wrapping_mul(d) / r;
        let t = t.wrapping_mul(d) / r;
        x = r.wrapping_sub(t);
    }
    x.wrapping_mul(0xfe) / r
}

/// `0x009435a0` (`thiscall(heap, node)`, `ret 4`): a free node out of its
/// bin.
pub fn remove_free(mem: &mut impl Mem, node: u32) {
    let prev = rd(mem, node + 4);
    let next = rd(mem, node + 8);
    wr(mem, prev + 8, next);
    wr(mem, next + 4, prev);
    wr(mem, node + 4, 0);
    wr(mem, node + 8, 0);
}

/// `0x009434c0` (`thiscall(heap, node, arena)`, `ret 8`): a free node into
/// `arena`'s bin for its size: after the nodes of a smaller size, or of
/// the same size at a lower address (bin 254: by address only).
pub fn insert_free(mem: &mut impl Mem, heap: u32, node: u32, arena: u32) {
    let blk = rd(mem, node);
    let size = rd(mem, node + 0xc);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the original keeps eax
    let bin = bin_index(mem, heap, size) as u32;
    let bin_head = arena.wrapping_add(bin.wrapping_mul(16));
    let mut pos = bin_head;
    let mut cur = rd(mem, bin_head + 8);
    if bin == 0xfe {
        while cur != bin_head && blk >= rd(mem, cur) {
            pos = cur;
            cur = rd(mem, cur + 8);
        }
    } else {
        while cur != bin_head {
            let s = rd(mem, cur + 0xc).cast_signed();
            if s > size.cast_signed() || (s == size.cast_signed() && blk < rd(mem, cur)) {
                break;
            }
            pos = cur;
            cur = rd(mem, cur + 8);
        }
    }
    let next = rd(mem, pos + 8);
    wr(mem, pos + 8, node);
    wr(mem, node + 4, pos);
    wr(mem, node + 8, next);
    wr(mem, next + 4, node);
}

/// `0x00943050` (`thiscall(heap, arena)`, `ret 4`): an arena that became
/// one free block leaves the arena list; its node goes back to the pool,
/// its first two pages (and the page holding its last header) are queued
/// for decommitting and its reservation is given back.
pub fn release_arena(mem: &mut impl Mem, os: &mut impl Os, heap: u32, arena: u32) {
    let first = rd(mem, heap + 0xc);
    if first == arena {
        let next = rd(mem, arena + 0xff0);
        wr(mem, heap + 0xc, next);
    } else {
        let mut a = first;
        while a != 0 {
            let next = rd(mem, a + 0xff0);
            if next == arena {
                let after = rd(mem, arena + 0xff0);
                wr(mem, a + 0xff0, after);
                break;
            }
            a = next;
        }
    }
    let node = rd(mem, arena + 0x101c);
    remove_free(mem, node);
    node_free(mem, os, heap + 0x10, node);
    let mgr = PAGE_MANAGER.get(mem);
    queue_decommit(mem, os, mgr, arena, 0x2000);
    let last = arena.wrapping_sub(0x10).wrapping_add(rd(mem, heap)) & 0xffff_f000;
    if last >= arena.wrapping_add(0x2000) {
        let mgr = PAGE_MANAGER.get(mem);
        queue_decommit(mem, os, mgr, last, 0x1000);
    }
    let mgr = PAGE_MANAGER.get(mem);
    release_pages(mem, os, mgr, arena);
}

/// `0x00943130` (`thiscall(heap, p)`, `ret 4`): frees the block at `p`
/// (merging it with free neighbours, reusing their node), queues the whole
/// pages inside it for decommitting and releases an arena left empty.
/// Returns whether `p` was in an arena (in `al`).
pub fn heap_free(mem: &mut impl Mem, os: &mut impl Os, heap: u32, p: u32) -> bool {
    let mut arena = rd(mem, heap + 0xc);
    loop {
        if arena == 0 {
            return false;
        }
        if p >= arena && p < rd(mem, heap).wrapping_add(arena) {
            break;
        }
        arena = rd(mem, arena + 0xff0);
    }
    let hdr = p.wrapping_sub(0x10);
    let mut size = rd(mem, hdr + 4);
    let mut end = hdr.wrapping_add(size);
    let prev = hdr.wrapping_sub(rd(mem, hdr + 8));
    let next = hdr.wrapping_add(size);
    let mut blk = hdr;
    let mut node = 0;
    let mut start = p;
    let prev_node = rd(mem, prev + 0xc);
    if prev_node > 2 {
        size = size.wrapping_add(rd(mem, prev + 4));
        blk = prev;
        wr(mem, next + 8, size);
        if prev_node != 3 {
            remove_free(mem, prev_node);
            node = prev_node;
        }
        start = hdr;
        if hdr & 0xffff_f000 >= prev.wrapping_add(0x10) {
            start = hdr & 0xffff_f000;
        }
    }
    let next_node = rd(mem, next + 0xc);
    if next_node > 2 {
        size = size.wrapping_add(rd(mem, next + 4));
        if next_node != 3 {
            remove_free(mem, next_node);
            if node == 0 {
                node = next_node;
                wr(mem, next_node, blk);
            } else {
                node_free(mem, os, heap + 0x10, next_node);
            }
        }
        let after = size.wrapping_add(blk);
        end = end.wrapping_add(0x10);
        let rounded = end.wrapping_add(0xfff) & 0xffff_f000;
        wr(mem, after + 8, size);
        if rounded <= after {
            end = rounded;
        }
    }
    wr(mem, blk + 4, size);
    if node == 0 {
        node = node_alloc(mem, os, heap + 0x10);
        wr(mem, node, blk);
    }
    wr(mem, node + 0xc, size);
    wr(mem, node + 8, 0);
    wr(mem, node + 4, 0);
    wr(mem, blk + 0xc, node);
    insert_free(mem, heap, node, arena);
    let from = start.wrapping_add(0xfff) & 0xffff_f000;
    let to = end & 0xffff_f000;
    if from < to {
        let mgr = PAGE_MANAGER.get(mem);
        queue_decommit(mem, os, mgr, from, to.wrapping_sub(from));
    }
    if size == rd(mem, heap).wrapping_sub(0x1020) {
        release_arena(mem, os, heap, arena);
    }
    true
}

/// `0x00942dc0` (fastcall, the heap in ECX): a new arena from the page
/// manager (its first two pages and its last one committed): 255 empty
/// bins, an in-use 16-byte block at `+0x1000`, one free block from
/// `+0x1010` in bin 254, an in-use 16-byte block at the end; 0 when a page
/// call fails.
pub fn new_arena(mem: &mut impl Mem, os: &mut impl Os, heap: u32) -> u32 {
    let mgr = PAGE_MANAGER.get(mem);
    let size = rd(mem, heap);
    let a = page_alloc(mem, os, mgr, size, 0x1000);
    if a == 0 {
        return 0;
    }
    let mgr = PAGE_MANAGER.get(mem);
    if !commit(mem, os, mgr, a, 0x2000) {
        return 0;
    }
    let last = rd(mem, heap).wrapping_sub(0x1000).wrapping_add(a);
    if last >= a.wrapping_add(0x2000) {
        let mgr = PAGE_MANAGER.get(mem);
        if !commit(mem, os, mgr, last, 0x1000) {
            return 0;
        }
    }
    mem.write(a, [0u32; 0xff0 / 4]);
    for i in 0..0xff {
        let h = a + 16 * i;
        wr(mem, h + 4, h);
        wr(mem, h + 8, h);
    }
    wr(mem, a + 0x1008, 0);
    mem.write(a + 0x1000, BLOCK_MAGIC);
    wr(mem, a + 0x1004, 0x10);
    wr(mem, a + 0x100c, 2);
    let big = rd(mem, heap).wrapping_sub(0x1020);
    let blk = a + 0x1010;
    mem.write(blk, [BLOCK_MAGIC, big, 0x10]);
    let end = blk.wrapping_add(big);
    mem.write(end, [BLOCK_MAGIC, 0x10, big, 2]);
    let node = node_alloc(mem, os, heap + 0x10);
    wr(mem, node, blk);
    wr(mem, node + 0xc, big);
    wr(mem, blk + 0xc, node);
    let bin = a + 0xfe0;
    wr(mem, bin + 8, node);
    wr(mem, bin + 4, node);
    wr(mem, node + 4, bin);
    wr(mem, node + 8, bin);
    wr(mem, a + 0xff0, 0);
    mem.write(a + 0xff4, [BLOCK_MAGIC; 3]);
    a
}

/// What a block header holds at `+0`.
pub const BLOCK_MAGIC: u32 = 0xabcd_ef12;

/// `0x00943380` (`thiscall(heap, size, &arena)`, `ret 8`), with the arena
/// returned: the first free node of at least `size` bytes, out of its bin:
/// in the size's bin (the first big enough), else the first node of the
/// next non-empty bin, in the first arena that has one (arenas added at
/// the end as needed). `(node, arena)`, the node 0 when no arena can be
/// made.
pub fn find_free(mem: &mut impl Mem, os: &mut impl Os, heap: u32, size: u32) -> (u32, u32) {
    let mut arena = rd(mem, heap + 0xc);
    if arena == 0 {
        arena = new_arena(mem, os, heap);
        wr(mem, heap + 0xc, arena);
        if arena == 0 {
            return (0, 0);
        }
    }
    loop {
        #[allow(clippy::cast_possible_truncation)] // the original keeps eax
        let mut bin = bin_index(mem, heap, size) as i32;
        let mut bin_head = bin.cast_unsigned().wrapping_mul(16).wrapping_add(arena);
        let mut n = rd(mem, bin_head + 8);
        let mut found = false;
        while n != bin_head {
            if rd(mem, n + 0xc).cast_signed() >= size.cast_signed() {
                found = true;
                break;
            }
            n = rd(mem, n + 8);
        }
        if !found {
            loop {
                if bin >= 0xff {
                    break;
                }
                n = rd(mem, bin_head + 0x18);
                bin_head = bin_head.wrapping_add(0x10);
                bin += 1;
                if n != bin_head {
                    break;
                }
            }
        }
        if bin < 0xff && n != bin_head {
            remove_free(mem, n);
            return (n, arena);
        }
        if rd(mem, arena + 0xff0) == 0 {
            let next = new_arena(mem, os, heap);
            wr(mem, arena + 0xff0, next);
        }
        arena = rd(mem, arena + 0xff0);
        if arena == 0 {
            return (0, 0);
        }
    }
}

/// [`find_free`] with the arena stored at `arena_out`, as the original.
pub fn find_free_at(
    mem: &mut impl Mem,
    os: &mut impl Os,
    heap: u32,
    size: u32,
    arena_out: u32,
) -> u32 {
    let first = rd(mem, heap + 0xc);
    wr(mem, arena_out, first);
    let (node, arena) = find_free(mem, os, heap, size);
    wr(mem, arena_out, arena);
    node
}

/// `0x00942b60` (`thiscall(heap, size)`, `ret 4`): a block of `size`
/// bytes rounded up to 16, plus its header, carved from [`find_free`]'s
/// node: the pages it now covers committed, the rest left free (its node
/// moves to the remainder, or the remainder is free without a node when
/// under the bins' minimum, or the whole node is used). Returns the user
/// pointer, or 0.
pub fn heap_alloc(mem: &mut impl Mem, os: &mut impl Os, heap: u32, size: u32) -> u32 {
    let mut need = (size.wrapping_add(0xf) & !0xf).wrapping_add(0x10);
    let (node, arena) = find_free(mem, os, heap, need);
    if node == 0 {
        return 0;
    }
    let blk = rd(mem, node);
    let mut rest = rd(mem, node + 0xc).wrapping_sub(need).cast_signed();
    if rest < 0x10 {
        need = need.wrapping_add(rest.cast_unsigned());
        rest = 0;
    }
    let next = rd(mem, blk + 4).wrapping_add(blk);
    let p = blk + 0x10;
    wr(mem, blk + 0xc, 2);
    wr(mem, blk + 4, need);
    let mgr = PAGE_MANAGER.get(mem);
    if rest == 0 {
        node_free(mem, os, heap + 0x10, node);
        wr(mem, next + 8, need);
        let from = p.wrapping_add(0xfff) & 0xffff_f000;
        let to = need.wrapping_sub(0x10).wrapping_add(p) & 0xffff_f000;
        if from < to {
            let mgr = PAGE_MANAGER.get(mem);
            if !commit(mem, os, mgr, from, to.wrapping_sub(from)) {
                return 0;
            }
        }
        return p;
    }
    let mut end = p.wrapping_add(need);
    let up = end.wrapping_add(0xfff) & 0xffff_f000;
    if up <= next {
        end = up;
    }
    let from = p.wrapping_add(0xfff) & 0xffff_f000;
    let to = end & 0xffff_f000;
    if from < to && !commit(mem, os, mgr, from, to.wrapping_sub(from)) {
        return 0;
    }
    let r = blk.wrapping_add(need);
    let rest = rest.cast_unsigned();
    mem.write(r, [BLOCK_MAGIC, rest, need]);
    wr(mem, next + 8, rest);
    if rest.cast_signed() < rd(mem, heap + 4).cast_signed() {
        wr(mem, r + 0xc, 3);
        node_free(mem, os, heap + 0x10, node);
        return p;
    }
    wr(mem, r + 0xc, node);
    wr(mem, node, r);
    wr(mem, node + 0xc, rest);
    insert_free(mem, heap, node, arena);
    p
}

/// `0x00943480` (`thiscall(heap, p)`, `ret 4`): the usable size of the
/// block at `p` (its size less the header), or -1 when `p` is in no arena.
pub fn block_size(mem: &impl Mem, heap: u32, p: u32) -> u32 {
    let mut arena = rd(mem, heap + 0xc);
    while arena != 0 {
        if p >= arena && p < rd(mem, heap).wrapping_add(arena) {
            return rd(mem, p.wrapping_sub(0xc)).wrapping_sub(0x10);
        }
        arena = rd(mem, arena + 0xff0);
    }
    u32::MAX
}

/// The user heap, created on first use ([`malloc_pool_init`]).
fn user_heap(mem: &mut impl Mem, os: &mut impl Os) -> u32 {
    if USER_HEAP.get(mem) == 0 {
        malloc_pool_init(mem, os);
    }
    USER_HEAP.get(mem)
}

/// `memset(p, 0xcd, len)`, the fill `user_malloc` leaves in new blocks.
fn fill_cd(mem: &mut impl Mem, p: u32, len: u32) {
    mem.write_bytes(p, &vec![0xcd; len as usize]);
}

/// `user_malloc(size)` (`0x006fb910`, behind the `0x00687020` jump):
/// [`heap_alloc`] under the heap's lock, the block filled with `0xcd`.
pub fn user_malloc(mem: &mut impl Mem, os: &mut impl Os, size: u32) -> u32 {
    let heap = user_heap(mem, os);
    os.enter_critical_section(heap);
    let p = heap_alloc(mem, os, heap + 0x18, size);
    os.leave_critical_section(heap);
    fill_cd(mem, p, size);
    p
}

/// `user_aligned_malloc(size, align)` (`0x006fb770`, behind `0x00686ff0`):
/// `size + align` bytes, the pointer moved up to the next multiple of
/// `align` (by 1..=`align` bytes; the remainder taken of the pointer
/// sign-extended to 64 bits, as `__aullrem` gets it), the distance stored
/// in the byte before it; the first `size` bytes of the block filled with
/// `0xcd`.
pub fn user_aligned_malloc(mem: &mut impl Mem, os: &mut impl Os, size: u32, align: u32) -> u32 {
    let heap = user_heap(mem, os);
    os.enter_critical_section(heap);
    let p = heap_alloc(mem, os, heap + 0x18, size.wrapping_add(align));
    os.leave_critical_section(heap);
    #[allow(clippy::cast_possible_truncation)] // a remainder below `align`
    let rem = (i64::from(p.cast_signed()).cast_unsigned() % u64::from(align)) as u32;
    let r = p.wrapping_sub(rem).wrapping_add(align);
    fill_cd(mem, p, size);
    mem.write(
        r.wrapping_sub(1),
        r.to_le_bytes()[0].wrapping_sub(p.to_le_bytes()[0]),
    );
    r
}

/// `user_mem_size(p)` (`0x006fba90`, behind `0x00687050`): [`block_size`]
/// under the heap's lock.
pub fn user_mem_size(mem: &mut impl Mem, os: &mut impl Os, p: u32) -> u32 {
    let heap = USER_HEAP.get(mem);
    os.enter_critical_section(heap);
    let n = block_size(mem, heap + 0x18, p);
    os.leave_critical_section(heap);
    n
}

/// `user_realloc(p, size)` (`0x006fbb00`, behind `0x00687060`): a new block
/// of `size` (none for 0), the old contents copied up to the smaller size
/// (`memcpy_s`), the old block freed; returns the new block.
pub fn user_realloc(mem: &mut impl Mem, os: &mut impl Os, p: u32, size: u32) -> u32 {
    let mut heap = user_heap(mem, os);
    let mut q = 0;
    if size != 0 {
        os.enter_critical_section(heap);
        q = heap_alloc(mem, os, heap + 0x18, size);
        os.leave_critical_section(heap);
        heap = USER_HEAP.get(mem);
    }
    if p != 0 {
        os.enter_critical_section(heap);
        let old = block_size(mem, heap + 0x18, p);
        os.leave_critical_section(heap);
        let n = old.min(size);
        if q != 0 {
            let mut bytes = vec![0u8; n as usize];
            mem.read_bytes(p, &mut bytes);
            mem.write_bytes(q, &bytes);
        }
        let heap = USER_HEAP.get(mem);
        os.enter_critical_section(heap);
        heap_free(mem, os, heap + 0x18, p);
        os.leave_critical_section(heap);
    }
    q
}

/// `user_free(p)` (`0x006fb8b0`): [`heap_free`] on the user heap under its
/// lock.
pub fn user_free(mem: &mut impl Mem, os: &mut impl Os, p: u32) {
    let heap = USER_HEAP.get(mem);
    os.enter_critical_section(heap);
    heap_free(mem, os, heap + 0x18, p);
    os.leave_critical_section(heap);
}

/// `user_aligned_free(p)` (`0x006fb6c0`): the block `p` was moved up
/// from (by the byte before `p`) measured under the heap's lock, filled
/// with `0xfe`, then freed under the lock.
pub fn user_aligned_free(mem: &mut impl Mem, os: &mut impl Os, p: u32) {
    let base = p.wrapping_sub(u32::from(mem.read::<u8>(p.wrapping_sub(1))));
    let heap = USER_HEAP.get(mem);
    os.enter_critical_section(heap);
    let size = block_size(mem, heap + 0x18, base);
    os.leave_critical_section(heap);
    mem.write_bytes(base, &vec![0xfe; size as usize]);
    let heap = USER_HEAP.get(mem);
    os.enter_critical_section(heap);
    heap_free(mem, os, heap + 0x18, base);
    os.leave_critical_section(heap);
}

/// `user_calloc(n, size)` (`0x006fb820`, behind the `0x00687000` jump):
/// `n * size` bytes (32-bit product) under the heap's lock, zeroed.
pub fn user_calloc(mem: &mut impl Mem, os: &mut impl Os, n: u32, size: u32) -> u32 {
    let heap = user_heap(mem, os);
    os.enter_critical_section(heap);
    let len = n.wrapping_mul(size);
    let p = heap_alloc(mem, os, heap + 0x18, len);
    os.leave_critical_section(heap);
    mem.write_bytes(p, &vec![0; len as usize]);
    p
}

/// `Phyre::PhyreMalloc(size)` (`0x0042fb40`): `size + 0x14` user bytes, the
/// result 16-aligned past a word that keeps the block (at the word below
/// it, as the original's `(-(r - 7) & 3) + r - 7` gives); 0 when the heap
/// is out.
pub fn phyre_malloc(mem: &mut impl Mem, os: &mut impl Os, size: u32) -> u32 {
    let p = user_malloc(mem, os, size.wrapping_add(0x14));
    if p == 0 {
        return 0;
    }
    let q = p.wrapping_add(4);
    let r = q.wrapping_add(q.wrapping_neg() & 0xf);
    store_block(mem, r, p);
    r
}

/// `Phyre::PhyreMalloc(size, align)` (`0x0042fb80`): as [`phyre_malloc`]
/// with `size + 4 + align` bytes and the result aligned to `align` (a power
/// of two).
pub fn phyre_malloc_aligned(mem: &mut impl Mem, os: &mut impl Os, size: u32, align: u32) -> u32 {
    let p = user_malloc(mem, os, size.wrapping_add(4).wrapping_add(align));
    if p == 0 {
        return 0;
    }
    let q = p.wrapping_add(4);
    let r = q.wrapping_add(q.wrapping_neg() & align.wrapping_sub(1));
    store_block(mem, r, p);
    r
}

/// Where the Phyre allocations keep their block: `r - 7` plus
/// `-(r - 7) & 3` (the word below `r` when `r` is word-aligned).
fn block_word(r: u32) -> u32 {
    let low = r.wrapping_sub(7);
    low.wrapping_add(low.wrapping_neg() & 3)
}

fn store_block(mem: &mut impl Mem, r: u32, p: u32) {
    wr(mem, block_word(r), p);
}

/// `Phyre::PhyreFree(p)` (`0x0042fb20`): the block kept below `p` freed
/// ([`user_free`]); nothing for null.
pub fn phyre_free(mem: &mut impl Mem, os: &mut impl Os, p: u32) {
    if p != 0 {
        let block = rd(mem, block_word(p));
        user_free(mem, os, block);
    }
}

/// `memclear(p, n)` (`0x0062f4e0`): `memset(p, 0, n)`; returns `p`.
pub fn memclear(mem: &mut impl Mem, p: u32, n: u32) -> u32 {
    mem.write_bytes(p, &vec![0; n as usize]);
    p
}

/// The allocation tracker (`*0x00ccb15c`: its mutex at `+0xa8`) and its map
/// from block to record (`0x00c31e9c`, a `Phyre::PRedBlackTree` whose
/// nodes come from the free list at `0x00c31eb4`).
pub const ALLOC_TRACKER: Global<u32> = Global::new(0x00cc_b15c);
/// See [`ALLOC_TRACKER`].
pub const TRACKER_MAP: Global<[u32; 6]> = Global::new(0x00c3_1e9c);
/// See [`ALLOC_TRACKER`].
pub const TRACKER_NODES: Global<u32> = Global::new(0x00c3_1eb4);
/// A block `operator delete` hands back to the C runtime instead
/// (`0x00ccb160`).
pub const CRT_BLOCK: Global<u32> = Global::new(0x00cc_b160);

/// `Phyre::PFreeList::deallocate(node)` (`0x0049c980`, `thiscall`): `node`
/// pushed on the free list.
pub fn free_list_deallocate(mem: &mut impl Mem, list: u32, node: u32) {
    let head = rd(mem, list);
    wr(mem, node, head);
    wr(mem, list, node);
}

/// `0x00630a60(p)`: with the allocation tracker on, under its mutex: the
/// record of `p` (`+0x10` of its map node) freed, then the node removed from
/// the map (`Phyre::PRedBlackTree::remove`, `0x00575000`, passed in) and
/// put back on the node free list.
pub fn tracker_release<M: Mem>(
    mem: &mut M,
    os: &mut impl Os,
    p: u32,
    tree_remove: impl FnOnce(&mut M, u32, u32),
) {
    let tracker = ALLOC_TRACKER.get(mem);
    if tracker == 0 {
        return;
    }
    let mutex = rd(mem, tracker + 0xa8);
    os.wait_for_single_object(mutex, u32::MAX);
    let map = TRACKER_MAP.ptr(mem).addr;
    let find = |mem: &M| crate::stl::bridge::map_find_u32_value(mem, map, p);
    let n = find(mem);
    if n != 0 {
        let record = rd(mem, n + 0x10);
        user_free(mem, os, record);
        let n = find(mem);
        if n != 0 {
            tree_remove(mem, map, n);
            let nodes = TRACKER_NODES.ptr(mem).addr;
            free_list_deallocate(mem, nodes, n);
        }
    }
    os.release_mutex(mutex);
}

/// `operator delete(p)` (`0x00630390`): the C runtime's block
/// ([`CRT_BLOCK`]) goes back to `free` and is forgotten; anything else
/// leaves the tracker ([`tracker_release`]) and is [`user_free`]d.
pub fn operator_delete<M: Mem>(
    mem: &mut M,
    os: &mut impl Os,
    p: u32,
    tree_remove: impl FnOnce(&mut M, u32, u32),
) {
    if CRT_BLOCK.get(mem) == p {
        os.crt_free(p);
        CRT_BLOCK.set(mem, 0);
        return;
    }
    tracker_release(mem, os, p, tree_remove);
    user_free(mem, os, p);
}

/// `operator delete[](p)` (`0x00630540`): [`tracker_release`], then
/// [`user_free`].
pub fn operator_delete_array<M: Mem>(
    mem: &mut M,
    os: &mut impl Os,
    p: u32,
    tree_remove: impl FnOnce(&mut M, u32, u32),
) {
    tracker_release(mem, os, p, tree_remove);
    user_free(mem, os, p);
}

/// The destructors that free a buffer held at `+8` (`0x00633b70` and 12
/// more copies, thiscall; run by exception unwinding): `user_free` of it
/// when set. Returns 0, the `eax` the original leaves either way.
pub fn free_member8(mem: &mut impl Mem, os: &mut impl Os, this: u32) -> u32 {
    let p = rd(mem, this.wrapping_add(8));
    if p != 0 {
        user_free(mem, os, p);
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_mixes() {
        assert_eq!(address_hash(0), {
            let h = (0x3du32).wrapping_mul(9);
            let h = (h >> 4 ^ h).wrapping_mul(0x27d4_eb2d);
            h >> 15 ^ h
        });
    }
}
