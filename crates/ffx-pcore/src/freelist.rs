//! `Phyre::PFreeList`: fixed-size items handed out from blocks.
//!
//! Layout: `+0` the free list's head (each free item's first word links
//! the next), `+4` the item size (rounded up to 4), `+8` a count, `+0xc`
//! the alignment (rounded up to 4), `+0x10` a word, `+0x14` the blocks
//! (each block: `+0` the next block, `+4` its first item, `+8` its item
//! count; the items start at `(align + 0xb) & !(align - 1)`).

pub mod bridge {
    //! The originals over game memory.

    use ffx_graphic::host::Host;

    use crate::{phyre_free, phyre_malloc};

    /// `PFreeList::PFreeList(this, size, count, align, d)`; `this`.
    pub fn new(h: &mut impl Host, this: u32, size: u32, count: u32, align: u32, d: u32) -> u32 {
        h.write(this + 4, size.wrapping_add(3) & !3);
        h.write(this + 8, count);
        h.write(this + 0xc, align.wrapping_add(3) & !3);
        h.write(this + 0x10, d);
        h.write(this + 0x14, 0u32);
        h.write(this, 0u32);
        this
    }

    /// `PFreeList::~PFreeList(this)` and `PFreeList::reset(this)`: frees
    /// every block (`PhyreFree`), empties the free list.
    pub fn free_blocks(h: &mut impl Host, this: u32) {
        loop {
            let block: u32 = h.read(this + 0x14);
            if block == 0 {
                break;
            }
            let next: u32 = h.read(block);
            h.write(this + 0x14, next);
            phyre_free(h, block);
        }
        h.write(this, 0u32);
    }

    /// `PFreeList::addItemToFreeList(item, head)`: `item` in front of the
    /// list at `*head`; returns the old head.
    pub fn add_item(h: &mut impl Host, item: u32, head: u32) -> u32 {
        let old: u32 = h.read(head);
        h.write(item, old);
        h.write(head, item);
        old
    }

    /// `PFreeList::deallocate(this, item)`: `item` back in front of the
    /// free list; returns the old head.
    pub fn deallocate(h: &mut impl Host, this: u32, item: u32) -> u32 {
        let old: u32 = h.read(this);
        h.write(item, old);
        h.write(this, item);
        old
    }

    /// `0x0049c7c0(this, first, n)`: links the `n` items from `first`
    /// (`this +4` apart) in front of the free list. 0, or 1 when `first`
    /// is null and `n` is not 0.
    pub fn link_items(h: &mut impl Host, this: u32, first: u32, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        if first == 0 {
            return 1;
        }
        let mut cur = first;
        for _ in 1..n {
            let next = h.read::<u32>(this + 4).wrapping_add(cur);
            h.write(cur, next);
            cur = next;
        }
        let head: u32 = h.read(this);
        h.write(cur, head);
        h.write(this, first);
        0
    }

    /// `PFreeList::allocateNewBlock(this, n)`: a block of `n` items
    /// (`PhyreMalloc`), linked in front of the blocks, its items onto the
    /// free list. `al`: 1, or 0 for no items or no memory.
    pub fn allocate_block(h: &mut impl Host, this: u32, n: u32) -> u8 {
        if n == 0 {
            return 0;
        }
        let align: u32 = h.read(this + 0xc);
        let header = !align.wrapping_sub(1) & align.wrapping_add(0xb);
        let size = h.read::<u32>(this + 4).wrapping_mul(n).wrapping_add(header);
        let block = phyre_malloc(h, size, align);
        if block == 0 {
            return 0;
        }
        let first = block.wrapping_add(header);
        h.write(block + 4, first);
        h.write(block + 8, n);
        let blocks: u32 = h.read(this + 0x14);
        h.write(block, blocks);
        h.write(this + 0x14, block);
        link_items(h, this, first, n);
        1
    }
}
