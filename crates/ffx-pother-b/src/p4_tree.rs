//! An intrusive binary tree keyed by C strings (`0x0064c0a0`).
//!
//! Layout: the tree object is its own sentinel (`tree + 0xc` the root
//! link); a node has two children (`+0`, `+4`, the sentinel for none), a
//! parent word with a flag in bit 0 (`+8`) and its key's address with a
//! flag in bit 0 (`+0xc`). Keys compare as `strcmp` on unsigned bytes; a
//! null key compares by address.

use ffx_bridge::Mem;

/// `strcmp(a, b) < 0` as the tree's compare loop computes it, with the
/// null-pointer rule.
pub fn less(m: &impl Mem, a: u32, b: u32) -> bool {
    if a == 0 || b == 0 {
        return a < b;
    }
    let mut i = 0u32;
    loop {
        let x: u8 = m.read(a.wrapping_add(i));
        let y: u8 = m.read(b.wrapping_add(i));
        if x != y {
            return x < y;
        }
        if x == 0 {
            return false;
        }
        i = i.wrapping_add(1);
    }
}

/// A tagged word without its flag bit (`w - (w & 1)`).
fn untag(w: u32) -> u32 {
    w.wrapping_sub(w & 1)
}

pub mod bridge {
    //! The original over game memory.

    use super::{Mem, less, untag};

    /// `StringTree_insert(tree, node)` (`0x0064c0a0`, thiscall, `ret 4`)
    /// -> 0, or the node already holding an equal key (then `node` is not
    /// linked). First `node` gets its flag bit set (`+8`) and both
    /// children the sentinel; then it is linked as the left (key less) or
    /// right child of the last node visited (the root link for an empty
    /// tree), and its parent word becomes that node plus the flag.
    pub fn insert(m: &mut impl Mem, tree: u32, node: u32) -> u32 {
        let w: u32 = m.read(node + 8);
        m.write(node + 8, w.wrapping_add(1 - (w & 1)));
        m.write(node, tree);
        m.write(node + 4, tree);
        let mut parent = tree + 0xc;
        let mut side = 0u32;
        let mut cur: u32 = m.read(tree + 0xc);
        if cur != tree {
            let key = untag(m.read(node + 0xc));
            loop {
                let other = untag(m.read(cur + 0xc));
                parent = cur;
                side = if less(m, key, other) {
                    0
                } else if less(m, other, key) {
                    1
                } else {
                    return cur;
                };
                cur = m.read(cur.wrapping_add(4 * side));
                if cur == tree {
                    break;
                }
            }
        }
        let w: u32 = m.read(node + 8);
        m.write(node + 8, (w & 1).wrapping_add(parent));
        m.write(parent.wrapping_add(4 * side), node);
        0
    }
}
