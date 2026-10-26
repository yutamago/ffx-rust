//! `PRedBlackTree`: successor, remove, clear.
//!
//! A node is `{+0 left, +4 right, +8 parent | colour}`: bit 0 of the
//! parent word is the colour; removal calls `postRemoveFixup` when the
//! unlinked node's bit is clear. The tree itself stands for the null
//! child; the node at
//! `tree + 0xc` is the root's parent (the header). A node that points at
//! itself (`left == node`) is in no tree.

use crate::thiscall;
use ffx_graphic::host::Host;

/// `Phyre_PRedBlackTree_postRemoveFixup` (thiscall `(tree, node)`).
pub const POST_REMOVE_FIXUP: u32 = 0x0057_4be0;

pub mod bridge {
    //! The originals over game memory.

    use super::{Host, POST_REMOVE_FIXUP, thiscall};

    /// A node's parent: the parent word less its colour bit (the
    /// originals subtract `byte & 1`).
    fn parent(h: &impl Host, n: u32) -> u32 {
        let w: u32 = h.read(n + 8);
        w.wrapping_sub(w & 1)
    }

    /// Sets `n`'s parent to `p`, keeping its colour bit.
    fn set_parent(h: &mut impl Host, n: u32, p: u32) {
        let w: u32 = h.read(n + 8);
        h.write(n + 8, (w & 1).wrapping_add(p));
    }

    /// `0x00575160` `PRedBlackTree::successor(tree, node)`: the leftmost
    /// node of `node`'s right subtree; else the first ancestor reached from
    /// a left child, or the tree when that is the header.
    pub fn successor(h: &mut impl Host, tree: u32, node: u32) -> u32 {
        let right: u32 = h.read(node + 4);
        if right != tree {
            let mut n = right;
            loop {
                let left: u32 = h.read(n);
                if left == tree {
                    return n;
                }
                n = left;
            }
        }
        let mut p = parent(h, node);
        if h.read::<u32>(p + 4) == node {
            loop {
                let child = p;
                p = parent(h, p);
                if h.read::<u32>(p + 4) != child {
                    break;
                }
            }
        }
        if p == tree.wrapping_add(0xc) { tree } else { p }
    }

    /// The body `remove` and `clear` share, for a node in the tree
    /// (`left != node`); the `eax` it leaves.
    fn unlink(h: &mut impl Host, tree: u32, z: u32) -> u32 {
        let zl: u32 = h.read(z);
        let y = if zl == tree || h.read::<u32>(z + 4) == tree {
            z
        } else {
            successor(h, tree, z)
        };
        let mut x: u32 = h.read(y);
        if x == tree {
            x = h.read(y + 4);
        }
        let py = parent(h, y);
        set_parent(h, x, py);
        let header = tree.wrapping_add(0xc);
        if py == header || h.read::<u32>(py) == y {
            h.write(py, x);
        } else {
            h.write(py + 4, x);
        }
        let black = h.read::<u32>(y + 8) & 1 != 0;
        let mut eax = header;
        if !black {
            eax = thiscall(h, POST_REMOVE_FIXUP, tree, &[x]);
        }
        if y != z {
            let zp: u32 = h.read(z + 8);
            let l: u32 = h.read(z);
            let r: u32 = h.read(z + 4);
            h.write(y, l);
            h.write(y + 8, zp);
            h.write(y + 4, r);
            let l: u32 = h.read(z);
            set_parent(h, l, y);
            let r: u32 = h.read(z + 4);
            set_parent(h, r, y);
            let p = parent(h, z);
            let side = u32::from(h.read::<u32>(p + 4) == z);
            h.write(p + 4 * side, y);
            eax = side;
        }
        h.write(z + 8, z);
        h.write(z, z);
        h.write(z + 4, z);
        eax
    }

    /// `0x00575000` `PRedBlackTree::remove(tree, node)`: unlinks `node`
    /// (nothing when it is in no tree) and leaves it pointing at itself;
    /// `eax` as the original leaves it.
    pub fn remove(h: &mut impl Host, tree: u32, node: u32) -> u32 {
        let left: u32 = h.read(node);
        if left == node {
            return left;
        }
        unlink(h, tree, node)
    }

    /// `0x00574760` `PRedBlackTree::clear(tree)`: removes the root until the
    /// header's child is the tree or null. A root in no tree
    /// (`left == root`) makes the original spin, and the port with it.
    pub fn clear(h: &mut impl Host, tree: u32) {
        loop {
            let root: u32 = h.read(tree.wrapping_add(0xc));
            if root == tree || root == 0 {
                return;
            }
            if h.read::<u32>(root) == root {
                continue;
            }
            unlink(h, tree, root);
        }
    }
}
