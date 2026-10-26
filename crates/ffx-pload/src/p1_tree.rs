//! List `p1-load`, continuation round 3, batch 1:
//! `PRedBlackTree::postRemoveFixup`.
//!
//! A node is `{+0 left, +4 right, +8 parent | colour}`: bit 0 of the
//! parent word set is red, clear is black. The tree object itself is the
//! nil child (its `+8` is the nil's colour word, written like any other
//! node's), and the node at `tree + 0xc` is the header: its `+0` is the
//! root. The original subtracts and adds the colour bit (`word -
//! (byte & 1)`, `word + (1 - (byte & 1))`); on the 32-bit words those are
//! the same as clearing and setting bit 0, which the port does.

use ffx_bridge::Mem;

/// The ports over game memory.
pub mod bridge {
    use super::Mem;

    fn red(m: &impl Mem, n: u32) -> bool {
        m.read::<u32>(n.wrapping_add(8)) & 1 != 0
    }

    fn paint_black(m: &mut impl Mem, n: u32) {
        let w: u32 = m.read(n.wrapping_add(8));
        m.write(n.wrapping_add(8), w & !1);
    }

    fn paint_red(m: &mut impl Mem, n: u32) {
        let w: u32 = m.read(n.wrapping_add(8));
        m.write(n.wrapping_add(8), w | 1);
    }

    /// The parent word less its colour bit.
    fn parent(m: &impl Mem, n: u32) -> u32 {
        m.read::<u32>(n.wrapping_add(8)) & !1
    }

    /// `n`'s parent set to `p`, keeping its colour bit.
    fn set_parent(m: &mut impl Mem, n: u32, p: u32) {
        let w: u32 = m.read(n.wrapping_add(8));
        m.write(n.wrapping_add(8), (w & 1).wrapping_add(p));
    }

    /// `n`'s child at the parent `p` replaced by `y`: `p`'s right slot
    /// when it holds `n`, else its left (the header has only the left).
    fn replace_child(m: &mut impl Mem, p: u32, n: u32, y: u32) {
        let side = u32::from(m.read::<u32>(p.wrapping_add(4)) == n);
        m.write(p.wrapping_add(4 * side), y);
    }

    /// `n` rotated left: its right child `y` takes its place.
    fn rotate_left(m: &mut impl Mem, tree: u32, n: u32) {
        let y: u32 = m.read(n.wrapping_add(4));
        let np = parent(m, n);
        let inner: u32 = m.read(y);
        m.write(n.wrapping_add(4), inner);
        if inner != tree {
            set_parent(m, inner, n);
        }
        set_parent(m, y, np);
        replace_child(m, np, n, y);
        m.write(y, n);
        set_parent(m, n, y);
    }

    /// `n` rotated right: its left child `y` takes its place.
    fn rotate_right(m: &mut impl Mem, tree: u32, n: u32) {
        let y: u32 = m.read(n);
        let np = parent(m, n);
        let inner: u32 = m.read(y.wrapping_add(4));
        m.write(n, inner);
        if inner != tree {
            set_parent(m, inner, n);
        }
        set_parent(m, y, np);
        replace_child(m, np, n, y);
        m.write(y.wrapping_add(4), n);
        set_parent(m, n, y);
    }

    /// `n` rotated toward the left side (`left`) or the right.
    fn rotate(m: &mut impl Mem, tree: u32, n: u32, left: bool) {
        if left {
            rotate_left(m, tree, n);
        } else {
            rotate_right(m, tree, n);
        }
    }

    /// `PRedBlackTree::postRemoveFixup(tree; x)` (`0x00574be0`,
    /// thiscall, `ret 4`): the colours rebalanced after a black node was
    /// unlinked, `x` the child that took its place. While `x` is black
    /// and not the root, its sibling `w` is made black (rotating at the
    /// parent when red), then either painted red (both its children
    /// black: `x` moves up), or rotated so its far child is red, which
    /// ends the walk at the root. The root is the one read at entry, not
    /// after the rotations (as the original). Last, the final node is
    /// painted black; `eax` is its colour bit before that.
    pub fn post_remove_fixup(m: &mut impl Mem, tree: u32, x: u32) -> u32 {
        let root: u32 = m.read(tree.wrapping_add(0xc));
        let mut x = x;
        while x != root && !red(m, x) {
            let p = parent(m, x);
            let left = m.read::<u32>(p) == x;
            // the near and far sides seen from x
            let (sib, near, far): (u32, u32, u32) = if left { (4, 0, 4) } else { (0, 4, 0) };
            let mut w: u32 = m.read(p.wrapping_add(sib));
            if red(m, w) {
                paint_black(m, w);
                paint_red(m, p);
                rotate(m, tree, p, left);
                let p = parent(m, x);
                w = m.read(p.wrapping_add(sib));
            }
            let mut w_far: u32 = m.read(w.wrapping_add(far));
            let w_near: u32 = m.read(w.wrapping_add(near));
            let far_byte = m.read::<u8>(w_far.wrapping_add(8));
            let near_byte = m.read::<u8>(w_near.wrapping_add(8));
            if (far_byte | near_byte) & 1 == 0 {
                paint_red(m, w);
                x = p;
                continue;
            }
            if !red(m, w_far) {
                paint_black(m, w_near);
                paint_red(m, w);
                rotate(m, tree, w, !left);
                w = m.read(p.wrapping_add(sib));
                w_far = m.read(w.wrapping_add(far));
            }
            // w takes p's colour; p and w's far child black
            let wc: u32 = m.read(w.wrapping_add(8));
            let pc = m.read::<u32>(p.wrapping_add(8)) & 1;
            m.write(w.wrapping_add(8), (wc & !1) | pc);
            paint_black(m, p);
            paint_black(m, w_far);
            rotate(m, tree, p, left);
            x = root;
            break;
        }
        let bit = m.read::<u32>(x.wrapping_add(8)) & 1;
        paint_black(m, x);
        bit
    }
}
