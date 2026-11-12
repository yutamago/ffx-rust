//! M5 finish, WP5 part 2: `std::_Tree` code after ported symbols that the
//! ports called in Unicorn: the cluster map's `_Insert_nohint`
//! (`0x006995e0`, 212 k emulated blocks in gate run 1). Nodes as in
//! [`crate::m5b_stl`]: `+0` left, `+4` parent, `+8` right, `+0xd` the nil
//! flag, the `u32` key at `+0x10`; the tree's `+0` is the head.

/// `std::_Tree::_Insert_at(this; &out, addLeft, where, value, node)`
/// (thiscall, `ret 0x14`): called as its port,
/// [`crate::m5b_stl::bridge::tree_insert_at`].
pub const INSERT_AT: u32 = 0x0069_93d0;
/// The iterator's `--` (thiscall on the iterator, no stack words; a copy
/// of `std_TreeIterator_dec_0069eca0`).
pub const ITERATOR_DEC: u32 = 0x0069_9f50;
/// `operator delete(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;

pub mod bridge {
    //! The ports over game memory.

    use ffx_graphic::host::{Conv, Host};

    use super::{ITERATOR_DEC, OPERATOR_DELETE};
    use crate::m5b_stl::bridge::tree_insert_at;

    fn nil(h: &impl Host, node: u32) -> bool {
        h.read::<u8>(node.wrapping_add(0xd)) != 0
    }

    fn key(h: &impl Host, node: u32) -> u32 {
        h.read(node.wrapping_add(0x10))
    }

    /// `_Insert_at` of `node` under `place`, `out` = (it, 1).
    fn hang(
        h: &mut impl Host,
        this: u32,
        out: u32,
        left: bool,
        place: u32,
        node: u32,
        frame: u32,
    ) -> u32 {
        tree_insert_at(h, this, frame, u32::from(left), place, node);
        let it: u32 = h.read(frame);
        h.write(out, it);
        h.write(out.wrapping_add(4), 1u8);
        out
    }

    /// `std::_Tree::_Insert_nohint(this; &out, leftish, &value, node)`
    /// (`0x006995e0`, thiscall, `ret 0x10`; called by `0x006998f0` and
    /// `ClusterManager_loadPCluster`): down from the root, left while the
    /// value's `u32` key is below the node's (at most: `leftish`); then, the
    /// predecessor of the place (the iterator one back, unless the place is
    /// the leftmost and the walk went left) holding a smaller key, `node`
    /// hung there (`_Insert_at`, [`super::INSERT_AT`]) and `out` = (it, 1); else `node` freed and
    /// `out` = (the equal one, 0). `frame`: one word of game memory for the
    /// iterator and `_Insert_at`'s result (the original's own argument slot
    /// `ebp + 0xc`). Returns `out`.
    pub fn insert_nohint(
        h: &mut impl Host,
        this: u32,
        out: u32,
        leftish: u32,
        value: u32,
        node: u32,
        frame: u32,
    ) -> u32 {
        let head: u32 = h.read(this);
        let want: u32 = h.read(value);
        let mut add_left = true;
        let mut place = head;
        let mut at: u32 = h.read(head.wrapping_add(4));
        while !nil(h, at) {
            place = at;
            add_left = if leftish & 0xff != 0 {
                key(h, at) >= want
            } else {
                want < key(h, at)
            };
            at = h.read(at.wrapping_add(if add_left { 0 } else { 8 }));
        }
        let insert = |h: &mut _, left: bool| hang(h, this, out, left, place, node, frame);
        h.write(frame, place);
        if add_left {
            let leftmost: u32 = h.read(head);
            if place == leftmost {
                return insert(h, true);
            }
            h.call_sym(ITERATOR_DEC, Conv::Thiscall(frame), &[]);
        }
        let before: u32 = h.read(frame);
        if key(h, before) < h.read::<u32>(value) {
            return insert(h, add_left);
        }
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[node]);
        h.write(out, before);
        h.write(out.wrapping_add(4), 0u8);
        out
    }
}
