//! The particle heaps (`pppCreateHeap`, `pppAlloc`, `pppFree`).
//!
//! A heap is four words: `cur` (the block to search from, 0 for none),
//! `first` and `last` (the sentinel blocks at both ends) and `used`
//! (bytes taken, headers included). Blocks are a doubly linked list in
//! address order, each with a 16-byte header: `next` (the following block,
//! so the end of this one), `prev`, `used`, and a pad word; the data
//! follows the header. A new heap is one free block at `first` (the start
//! of the memory) whose `next` is `last`, the 16 bytes at the end of the
//! memory, marked used, with no `next`.
//!
//! `pppAlloc` rounds the size up to 16 and adds the header, then walks
//! back (`prev`) from the block after `cur` for a free block that is large
//! enough, first from `cur` and then once more from `last`; it takes the
//! top of the block and splits off the rest when 16 bytes or more remain.
//! `pppFree` marks the block free, makes it `cur` and merges it with free
//! neighbours.

/// The heaps where the game keeps them.
pub mod bridge {
    use ffx_bridge::{Global, Mem, Ptr};

    /// `g_pppFpHeap`: the field effects' first heap (`pppFpGetHeapSize`).
    pub const FP_HEAP: Global<[u32; 4]> = Global::new(0x00ce_c22c);
    /// `g_pppFpHeap2`: their second heap.
    pub const FP_HEAP2: Global<[u32; 4]> = Global::new(0x00ce_c23c);

    fn w(at: u32, field: u32) -> Ptr<u32> {
        Ptr::new(at.wrapping_add(field))
    }
    const CUR: u32 = 0;
    const FIRST: u32 = 4;
    const LAST: u32 = 8;
    const USED: u32 = 0xc;
    const NEXT: u32 = 0;
    const PREV: u32 = 4;
    const TAKEN: u32 = 8;

    /// `pppCreateHeap(heap, memory, size)` (`0x0072c480`).
    pub fn ppp_create_heap(mem: &mut impl Mem, heap: u32, memory: u32, size: u32) {
        w(heap, FIRST).set(mem, memory);
        let last = size.wrapping_sub(0x10).wrapping_add(memory);
        w(heap, LAST).set(mem, last);
        w(heap, CUR).set(mem, 0);
        w(memory, PREV).set(mem, 0);
        let last = w(heap, LAST).get(mem);
        w(memory, NEXT).set(mem, last);
        w(memory, TAKEN).set(mem, 0);
        let last = w(heap, LAST).get(mem);
        let first = w(heap, FIRST).get(mem);
        w(last, PREV).set(mem, first);
        w(last, NEXT).set(mem, 0);
        w(last, TAKEN).set(mem, 1);
        w(heap, USED).set(mem, 0);
    }

    /// `pppAlloc(heap, size)` (`0x0072c3d0`): the data of a new block, or
    /// 0 when nothing is large enough.
    pub fn ppp_alloc(mem: &mut impl Mem, heap: u32, size: u32) -> u32 {
        let need = (size.wrapping_add(0xf) & !0xf).wrapping_add(0x10);
        let cur = w(heap, CUR).get(mem);
        let mut from = if cur == 0 {
            let first = w(heap, FIRST).get(mem);
            let next = w(first, NEXT).get(mem);
            w(heap, CUR).set(mem, first);
            next
        } else {
            w(cur, NEXT).get(mem)
        };
        loop {
            let mut b = w(from, PREV).get(mem);
            while b != 0 {
                let end = w(b, NEXT).get(mem);
                if w(b, TAKEN).get(mem) == 0
                    && end.wrapping_sub(b).cast_signed() >= need.cast_signed()
                {
                    return take(mem, heap, b, need);
                }
                b = w(b, PREV).get(mem);
            }
            if w(heap, CUR).get(mem) == 0 {
                return 0;
            }
            from = w(heap, LAST).get(mem);
            w(heap, CUR).set(mem, 0);
        }
    }

    fn take(mem: &mut impl Mem, heap: u32, b: u32, need: u32) -> u32 {
        let end = w(b, NEXT).get(mem);
        let used = w(heap, USED).get(mem).wrapping_add(need);
        w(heap, USED).set(mem, used);
        let top = w(b, NEXT).get(mem).wrapping_sub(need);
        if top != b && top.wrapping_sub(b) >= 0x10 {
            w(heap, CUR).set(mem, b);
            w(top, PREV).set(mem, b);
            w(top, NEXT).set(mem, end);
            w(end, PREV).set(mem, top);
            w(b, NEXT).set(mem, top);
            w(top, TAKEN).set(mem, 1);
            top.wrapping_add(0x10)
        } else {
            w(heap, CUR).set(mem, 0);
            w(b, TAKEN).set(mem, 1);
            b.wrapping_add(0x10)
        }
    }

    /// `pppFree(heap, data)` (`0x0072c510`).
    pub fn ppp_free(mem: &mut impl Mem, heap: u32, data: u32) {
        let mut h = data.wrapping_sub(0x10);
        w(h, TAKEN).set(mem, 0);
        w(heap, CUR).set(mem, h);
        let size = h.wrapping_sub(w(h, NEXT).get(mem));
        let used = w(heap, USED).get(mem).wrapping_add(size);
        w(heap, USED).set(mem, used);
        let prev = w(h, PREV).get(mem);
        if prev != 0 && w(prev, TAKEN).get(mem) == 0 {
            let next = w(h, NEXT).get(mem);
            w(prev, NEXT).set(mem, next);
            let next = w(h, NEXT).get(mem);
            w(next, PREV).set(mem, prev);
            h = prev;
        }
        let next = w(h, NEXT).get(mem);
        if w(next, TAKEN).get(mem) == 0 {
            let after = w(next, NEXT).get(mem);
            w(h, NEXT).set(mem, after);
            let after = w(next, NEXT).get(mem);
            w(after, PREV).set(mem, h);
        }
    }

    /// `pppFpGetHeapSize()` (`0x00729290`): the bytes of both field effect
    /// heaps (`last - first + 16` each, for a heap that has both).
    pub fn ppp_fp_get_heap_size(mem: &impl Mem) -> u32 {
        let mut n = 0u32;
        for h in [FP_HEAP, FP_HEAP2] {
            let [_, first, last, _] = h.get(mem);
            if last != 0 && first != 0 {
                n = n.wrapping_add(last.wrapping_sub(first).wrapping_add(0x10));
            }
        }
        n
    }

    /// `g_pppMemAllocFailed`: set by `pppMemAlloc` when neither heap had
    /// room.
    pub const MEM_ALLOC_FAILED: Global<u32> = Global::new(0x0230_fd3c);
    /// `g_pppEnvSlot`: points at where `pppInitEnv` stores the current
    /// environment (`rcefMainSetEnv` sets it).
    pub const ENV_SLOT: Global<u32> = Global::new(0x00c3_a4cc);

    /// `pppMemAlloc(heap, spare, size)` (`0x00716a30`): from `heap`, else
    /// from `spare` when it has memory (`+4`); 0 and the failure flag when
    /// neither has room.
    pub fn ppp_mem_alloc(mem: &mut impl Mem, heap: u32, spare: u32, size: u32) -> u32 {
        let p = ppp_alloc(mem, heap, size);
        MEM_ALLOC_FAILED.set(mem, 0);
        if p != 0 {
            return p;
        }
        if mem.read::<u32>(spare.wrapping_add(4)) != 0 {
            let p = ppp_alloc(mem, spare, size);
            if p != 0 {
                return p;
            }
        }
        MEM_ALLOC_FAILED.set(mem, 1);
        0
    }

    /// `pppInitEnv(env, data, memory, size)` (`0x007169d0`): makes `env`
    /// the current environment, takes `data`'s three tables (`+0x14..0x1c`)
    /// into `+0x20..0x28`, and creates its heap.
    pub fn ppp_init_env(mem: &mut impl Mem, env: u32, data: u32, memory: u32, size: u32) {
        let slot = ENV_SLOT.get(mem);
        mem.write(slot, env);
        if data != 0 {
            for i in 0..3 {
                let t = ffx_system::stub::ptr32(mem.read(data.wrapping_add(0x14 + 4 * i)));
                mem.write(env.wrapping_add(0x20 + 4 * i), t);
            }
        }
        ppp_create_heap(mem, env, memory, size);
        mem.write(env.wrapping_add(0x14), 0u32);
    }
}
