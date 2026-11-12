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

/// `operator new(n)` (cdecl) and `MSVCP110!std::_Xbad_alloc`'s import
/// slot.
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const XBAD_ALLOC_IAT: u32 = 0x00b0_c2a0;
/// The node makers the value nodes call (thiscall on the tree): 0x20 and
/// 0x2c bytes, and the video map's copy of the 0x2c one.
pub const BUY_NODE_20: u32 = 0x006a_0400;
pub const BUY_NODE_2C: u32 = 0x006a_03c0;
pub const BUY_NODE_VIDEO: u32 = 0x006d_7930;
/// `StdString_moveWithWord_0069c200(this; src, unused)` (thiscall, `ret
/// 8`): the prototype's move into a node.
pub const PROTOTYPE_COPY: u32 = 0x0069_c200;

/// The video index map's `_Insert_at(this; &out, addLeft, where, value,
/// node)` (thiscall, `ret 0x14`) and iterator `--` (thiscall on the
/// iterator).
pub const VIDEO_INSERT_AT: u32 = 0x006d_5dc0;
pub const VIDEO_ITERATOR_DEC: u32 = 0x006d_7060;

/// The VFX render data table's clear (`VfxRenderDataTable_clearTree`,
/// thiscall) and erase of one node (`0x006a1a70`, thiscall `(&out,
/// where)`, `ret 8`; the same code as `ClusterMap_erase_0069b010`).
pub const VFX_TABLE_CLEAR: u32 = 0x006a_0fe0;
pub const VFX_TABLE_ERASE: u32 = 0x006a_1a70;

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

    /// The characters of the `std::string` at `s`.
    fn chars(h: &impl Host, s: u32) -> u32 {
        let cap: u32 = h.read(s.wrapping_add(0x14));
        if cap >= 16 { h.read(s) } else { s }
    }

    /// `std::map<std::string, ...>::lower_bound(this; &key)`
    /// (`0x006e9920`, thiscall, `ret 4`): from the root, the first node
    /// whose key (the `std::string` at `+0x10`) is not below `key`
    /// (`memcmp` of the shorter length, then the lengths); the head when
    /// none.
    #[allow(clippy::many_single_char_names)]
    pub fn lower_bound_string(h: &mut impl Host, this: u32, key: u32) -> u32 {
        let head: u32 = h.read(this);
        let mut result = head;
        let mut n: u32 = h.read(head.wrapping_add(4));
        let klen: u32 = h.read(key.wrapping_add(0x10));
        while !nil(h, n) {
            let nk = n.wrapping_add(0x10);
            let nlen: u32 = h.read(nk.wrapping_add(0x10));
            let a = chars(h, nk);
            let b = chars(h, key);
            let mut r = crate::stl::bridge::memcmp(h, a, b, nlen.min(klen));
            if r == 0 {
                r = if nlen < klen {
                    -1
                } else {
                    i32::from(nlen != klen)
                };
            }
            if r >= 0 {
                result = n;
                n = h.read(n);
            } else {
                n = h.read(n.wrapping_add(8));
            }
        }
        result
    }

    /// `operator new(size)` for a node; `_Xbad_alloc` (through the import)
    /// when it fails, as the original's tail jump.
    fn new_node(h: &mut impl Host, size: u32) -> u32 {
        let n = h.call_sym(super::OPERATOR_NEW, Conv::Cdecl, &[size]);
        if n == 0 {
            let f: u32 = h.read(h.rebase(super::XBAD_ALLOC_IAT));
            h.call(f, Conv::Cdecl, &[]);
        }
        n
    }

    /// `std::_Tree::_Buyheadnode` of a 0x20-byte node (`0x006a0390`,
    /// cdecl): its left, parent and right itself, black and nil
    /// (`+0xc` = `0x0101`). The node.
    pub fn buy_head_node(h: &mut impl Host) -> u32 {
        buy_head_node_sized(h, 0x20)
    }

    /// [`buy_head_node`] of a `size`-byte node (`0x00806af0`: 0x60).
    pub fn buy_head_node_sized(h: &mut impl Host, size: u32) -> u32 {
        let n = new_node(h, size);
        if n == 0 {
            return 0;
        }
        for off in [0, 4, 8] {
            h.write(n + off, n);
        }
        h.write(n + 0xc, 0x0101u16);
        n
    }

    /// `std::_Tree::_Buynode0` (`0x006a03c0`: `size` 0x2c, `0x006a0400`:
    /// 0x20; thiscall on the tree): a node of `size` bytes with its left,
    /// parent and right the tree's head. The node.
    pub fn buy_node(h: &mut impl Host, this: u32, size: u32) -> u32 {
        let n = new_node(h, size);
        if n == 0 {
            return 0;
        }
        let head: u32 = h.read(this);
        for off in [0, 4, 8] {
            h.write(n + off, head);
        }
        n
    }

    /// `std::_Tree::_Buynode(value)` of a 0x20-byte node with a 12-byte
    /// value (`0x0069c5f0`, `0x0069c630`; thiscall on the tree, `ret 4`):
    /// [`buy_node`], colour and nil 0 (`+0xc` word), the value's three
    /// words to `+0x10`. The node.
    pub fn buy_node_value12(h: &mut impl Host, this: u32, value: u32) -> u32 {
        let n = h.call_sym(super::BUY_NODE_20, Conv::Thiscall(this), &[]);
        h.write(n.wrapping_add(0xc), 0u16);
        let at = n.wrapping_add(0x10);
        if at != 0 {
            for k in 0..3 {
                let w: u32 = h.read(value.wrapping_add(4 * k));
                h.write(at + 4 * k, w);
            }
        }
        n
    }

    /// The same for the prototype table's 0x2c-byte nodes (`0x0069c5c0`):
    /// [`super::BUY_NODE_2C`], the word at `+0xc` 0, the value copied
    /// into `+0x10` by its copy constructor (`0x0069c200`, thiscall
    /// `(value, 0)`). The node.
    pub fn buy_node_prototype(h: &mut impl Host, this: u32, value: u32) -> u32 {
        let n = h.call_sym(super::BUY_NODE_2C, Conv::Thiscall(this), &[]);
        h.write(n.wrapping_add(0xc), 0u16);
        let at = n.wrapping_add(0x10);
        if at != 0 {
            h.call_sym(super::PROTOTYPE_COPY, Conv::Thiscall(at), &[value, 0]);
        }
        n
    }

    /// `VideoIndexMap_buyNode_006d5ca0(this; &(id, path))` (thiscall, `ret
    /// 4`): a 0x2c-byte node ([`super::BUY_NODE_VIDEO`], a copy of
    /// `0x006a03c0`), the word at `+0xc` 0, the id at `+0x10` and a
    /// `std::string` of the C string `path` at `+0x14` (made empty, then
    /// `assign(path, strlen(path))`). The node.
    pub fn video_buy_node(h: &mut impl Host, this: u32, value: u32) -> u32 {
        let n = h.call_sym(super::BUY_NODE_VIDEO, Conv::Thiscall(this), &[]);
        h.write(n.wrapping_add(0xc), 0u16);
        let v = n.wrapping_add(0x10);
        if v != 0 {
            let id: u32 = h.read(value);
            h.write(v, id);
            let path: u32 = h.read(value.wrapping_add(4));
            let s = v + 4;
            h.write(s + 0x14, 0xfu32);
            h.write(s + 0x10, 0u32);
            h.write(s, 0u8);
            crate::wp5_string::bridge::assign_cstr(h, s, path);
        }
        n
    }

    /// `std::_Tree::_Max(node)` (`0x0069a450`, cdecl): down the right links
    /// to the last node before a nil one.
    pub fn tree_max(h: &mut impl Host, node: u32) -> u32 {
        let mut n = node;
        loop {
            let r: u32 = h.read(n.wrapping_add(8));
            if nil(h, r) {
                return n;
            }
            n = r;
        }
    }

    /// `std::_Tree::_Min(node)` (`0x0069a470`, cdecl): the same down the
    /// left links.
    pub fn tree_min(h: &mut impl Host, node: u32) -> u32 {
        let mut n = node;
        loop {
            let l: u32 = h.read(n);
            if nil(h, l) {
                return n;
            }
            n = l;
        }
    }

    /// `VideoIndexMap_insertNode_006d5ff0(this; &out, leftish, &value,
    /// node)` (thiscall, `ret 0x10`): [`insert_nohint`] over signed `i32`
    /// keys, with the video map's own `_Insert_at` (`0x006d5dc0`, thiscall
    /// `(&out, addLeft, where, value, node)`, `ret 0x14`) and iterator `--`
    /// (`0x006d7060`); an equal key frees the node's path string (`+0x14`:
    /// its heap buffer when the capacity `+0x28` is 16 or more, then made
    /// empty) and the node. Returns `out`.
    #[allow(clippy::cast_possible_wrap, clippy::too_many_arguments)]
    pub fn video_insert_node(
        h: &mut impl Host,
        this: u32,
        out: u32,
        leftish: u32,
        value: u32,
        node: u32,
        frame: u32,
    ) -> u32 {
        let head: u32 = h.read(this);
        let want = h.read::<u32>(value) as i32;
        let mut add_left = true;
        let mut place = head;
        let mut at: u32 = h.read(head.wrapping_add(4));
        while !nil(h, at) {
            place = at;
            let k = key(h, at) as i32;
            add_left = if leftish & 0xff != 0 {
                k >= want
            } else {
                want < k
            };
            at = h.read(at.wrapping_add(if add_left { 0 } else { 8 }));
        }
        h.write(frame, place);
        let hang = |h: &mut _, left: bool| -> u32 {
            ffx_graphic::host::Host::call_sym(
                h,
                super::VIDEO_INSERT_AT,
                Conv::Thiscall(this),
                &[frame, u32::from(left), place, value, node],
            );
            let it: u32 = ffx_bridge::Mem::read(h, frame);
            ffx_bridge::Mem::write(h, out, it);
            ffx_bridge::Mem::write(h, out.wrapping_add(4), 1u8);
            out
        };
        if add_left {
            let leftmost: u32 = h.read(head);
            if place == leftmost {
                return hang(h, true);
            }
            h.call_sym(super::VIDEO_ITERATOR_DEC, Conv::Thiscall(frame), &[]);
        }
        let before: u32 = h.read(frame);
        if (key(h, before) as i32) < h.read::<u32>(value) as i32 {
            return hang(h, add_left);
        }
        let s = node.wrapping_add(0x14);
        if h.read::<u32>(s + 0x14) >= 16 {
            let p: u32 = h.read(s);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
        }
        h.write(s + 0x14, 0xfu32);
        h.write(s + 0x10, 0u32);
        h.write(s, 0u8);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[node]);
        h.write(out, before);
        h.write(out.wrapping_add(4), 0u8);
        out
    }

    /// The in-order successor of `n` (MSVC's inlined `++`): the head stays;
    /// else the leftmost node right of it, or the first ancestor it is left
    /// of.
    #[allow(clippy::many_single_char_names)]
    fn successor(h: &impl Host, n: u32) -> u32 {
        if nil(h, n) {
            return n;
        }
        let r: u32 = h.read(n.wrapping_add(8));
        if !nil(h, r) {
            let mut m = r;
            loop {
                let l: u32 = h.read(m);
                if nil(h, l) {
                    return m;
                }
                m = l;
            }
        }
        let mut cur = n;
        let mut p: u32 = h.read(n.wrapping_add(4));
        while !nil(h, p) && cur == h.read::<u32>(p.wrapping_add(8)) {
            cur = p;
            p = h.read(p.wrapping_add(4));
        }
        p
    }

    /// `std_Tree_erase_range_006a19d0(this; &out, first, last)` (thiscall,
    /// `ret 0xc`): the whole tree (`first` the leftmost, `last` the head)
    /// cleared at once ([`super::VFX_TABLE_CLEAR`]) and `out` = the head's
    /// left link; else node by node ([`super::VFX_TABLE_ERASE`], `frame`
    /// one word for its result), each successor taken first; `out` =
    /// `last`. Returns `out`.
    pub fn vfx_erase_range(
        h: &mut impl Host,
        this: u32,
        out: u32,
        first: u32,
        last: u32,
        frame: u32,
    ) -> u32 {
        let head: u32 = h.read(this);
        let leftmost: u32 = h.read(head);
        if first == leftmost && last == head {
            h.call_sym(super::VFX_TABLE_CLEAR, Conv::Thiscall(this), &[]);
            let head: u32 = h.read(this);
            let l: u32 = h.read(head);
            h.write(out, l);
            return out;
        }
        let mut at = first;
        while at != last {
            let next = successor(h, at);
            h.call_sym(super::VFX_TABLE_ERASE, Conv::Thiscall(this), &[frame, at]);
            at = next;
        }
        h.write(out, at);
        out
    }
}
