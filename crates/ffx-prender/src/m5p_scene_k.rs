//! M5 stage 1, list `scene`, batch 11: the world's instance count of a
//! class (`0x00443550`). The world keeps one instance list per class
//! (20-byte entries at `+0xc`, count `+8`, indexed by the class's
//! `+0x80`); a class without its own list counts the lists of its
//! derived classes, walked with the world cursor (`PWorldCursor_nextList`
//! on a 3-word cursor in the frame, `ebp-0x10`) over blocks of
//! instances. Conventions as in [`crate::m5p_scene_b`].

use ffx_graphic::host::{Conv, Host};

/// `PWorldCursor_nextList_00443ef0` (thiscall on the cursor).
pub const NEXT_LIST: u32 = 0x0044_3ef0;

pub mod bridge {
    //! The ports over game memory.

    use super::{Conv, Host, NEXT_LIST};

    const EXTERNAL: u32 = 0x8000_0000;

    /// Whether `x` is on the class chain from `p` (through `+0x40`).
    fn derives(h: &impl Host, mut p: u32, x: u32) -> bool {
        while p != 0 {
            if p == x {
                return true;
            }
            p = h.read(p + 0x40);
        }
        false
    }

    /// The instances one list (`node`, of class `class`) holds: its count
    /// (`+0x1c`) of `stride`-byte instances from `+0x14`, laid out over
    /// blocks (from the block word at `node - 0x18`, each block ending at
    /// the next block's address). A run that takes nothing loops as the
    /// original does.
    fn list_count(h: &impl Host, node: u32, class: u32) -> u32 {
        let mut left: u32 = h.read(node + 0x1c);
        let mut p: u32 = h.read(node + 0x14);
        let stride: u32 = h.read(class + 0x1c);
        let mut block: u32 = h.read(node.wrapping_sub(0x18));
        let mut count = 0u32;
        if left == 0 {
            return 0;
        }
        // leading empty blocks
        while block == p {
            block = h.read(block);
            p = p.wrapping_add(stride);
            left -= 1;
            if left == 0 {
                return 0;
            }
        }
        loop {
            let mut take = left;
            if p < block {
                // a zero stride faults in the original (a divide error)
                match (block - p).checked_div(stride) {
                    Some(t) => take = t,
                    None => return count,
                }
            }
            count = count.wrapping_add(take);
            if take == left && block == 0 {
                return count;
            }
            let mut steps = take;
            while steps != 0 {
                steps -= 1;
                loop {
                    p = p.wrapping_add(stride);
                    left = left.wrapping_sub(1);
                    if block != p {
                        break;
                    }
                    block = h.read(block);
                    if left == 0 {
                        break;
                    }
                }
            }
            if left == 0 {
                return count;
            }
        }
    }

    /// `0x00443550` (thiscall `(class)`, `ret 4`): the world's instance
    /// count of `class`: its own list's count (`+8` of its entry) when it
    /// has one; else, from the first class on its chain (`+0x40`) that has
    /// a list, every list the cursor visits whose class derives from
    /// `class`, counted over its blocks. 0 when no class on the chain has
    /// a list.
    pub fn world_instance_count(h: &mut impl Host, this: u32, class: u32, frame: u32) -> u32 {
        let index: u32 = h.read(class + 0x80);
        let count: u32 = h.read(this + 8);
        let n = count & !EXTERNAL;
        let lists: u32 = h.read(this + 0xc);
        if index < n {
            let e = lists.wrapping_add(index.wrapping_mul(20));
            if e != 0 {
                return h.read(e + 8);
            }
        }
        let mut c: u32 = h.read(class + 0x40);
        let first = loop {
            if c == 0 {
                return 0;
            }
            let i: u32 = h.read(c + 0x80);
            if i & EXTERNAL == 0 {
                break i;
            }
            c = h.read(c + 0x40);
        };
        let lists: u32 = h.read(this + 0xc);
        let e = if first < n {
            lists.wrapping_add(first.wrapping_mul(20))
        } else {
            0
        };
        h.write(frame, e);
        h.write(frame + 8, e);
        if e != 0 {
            let key: u32 = h.read(e + 0x10);
            h.write(frame + 4, key);
            h.write(frame, e + 0x14);
        } else {
            h.write(frame + 4, u32::MAX);
        }
        h.call_sym(NEXT_LIST, Conv::Thiscall(frame), &[]);
        let mut total = 0u32;
        let mut node: u32 = h.read(frame + 8);
        while node != 0 {
            let cls: u32 = h.read(node + 0xc);
            if derives(h, cls, class) {
                total = total.wrapping_add(list_count(h, node, cls));
            }
            h.call_sym(NEXT_LIST, Conv::Thiscall(frame), &[]);
            node = h.read(frame + 8);
        }
        total
    }
}
