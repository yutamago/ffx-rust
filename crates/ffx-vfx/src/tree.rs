//! Two binary trees the effect code keeps: one keyed by `i32`, with the
//! parent's colour in the low bit of the parent link, and the opus code's
//! iterated trees.
//!
//! A node of the keyed tree: `+0` and `+4` the children (the header, the
//! tree object itself, for none), `+8` the parent link (bit 0 the
//! colour), `+0xc` the key. The tree object's `+0xc` is the root (the
//! object itself while empty). A node of the iterated trees: `+0` left,
//! `+4` parent, `+8` right, byte `+0xd` set on the header ("nil").

/// The trees where the game keeps them.
pub mod bridge {
    use ffx_bridge::Mem;

    fn rd(mem: &impl Mem, at: u32) -> u32 {
        mem.read(at)
    }

    /// The child a search for `key` takes from `cur`, or `None` at `key`.
    fn side(mem: &impl Mem, cur: u32, key: i32) -> Option<u32> {
        let k = mem.read::<i32>(cur.wrapping_add(0xc));
        match k.cmp(&key) {
            std::cmp::Ordering::Greater => Some(0),
            std::cmp::Ordering::Less => Some(1),
            std::cmp::Ordering::Equal => None,
        }
    }

    /// `vfxIntTree_Find(this, key)` (`0x0070fd20`, thiscall): the node
    /// whose key is `*key`, or 0.
    pub fn vfx_int_tree_find(mem: &impl Mem, this: u32, key: u32) -> u32 {
        let want = mem.read::<i32>(key);
        let mut cur = rd(mem, this.wrapping_add(0xc));
        while cur != this {
            match side(mem, cur, want) {
                Some(s) => cur = rd(mem, cur.wrapping_add(4 * s)),
                None => return cur,
            }
        }
        0
    }

    /// `vfxIntTree_Insert(this, node)` (`0x0070fda0`, thiscall): the node
    /// red (bit 0 of its parent link set), its children the header, then
    /// hung under the leaf a search for its key ends at (from the root
    /// field `this + 0xc` when the tree is empty); returns 0, or without
    /// linking it the node that already has its key (the node's own
    /// fields already written).
    pub fn vfx_int_tree_insert(mem: &mut impl Mem, this: u32, node: u32) -> u32 {
        let link = rd(mem, node.wrapping_add(8));
        mem.write(node.wrapping_add(8), link.wrapping_add(1 - (link & 1)));
        mem.write(node, this);
        mem.write(node.wrapping_add(4), this);
        let key = mem.read::<i32>(node.wrapping_add(0xc));
        let mut parent = this.wrapping_add(0xc);
        let mut s = 0;
        let mut cur = rd(mem, parent);
        while cur != this {
            parent = cur;
            match side(mem, cur, key) {
                Some(d) => s = d,
                None => return cur,
            }
            cur = rd(mem, cur.wrapping_add(4 * s));
        }
        let colour = rd(mem, node.wrapping_add(8)) & 1;
        mem.write(node.wrapping_add(8), colour.wrapping_add(parent));
        mem.write(parent.wrapping_add(4 * s), node);
        0
    }

    fn nil(mem: &impl Mem, n: u32) -> bool {
        mem.read::<u8>(n.wrapping_add(0xd)) != 0
    }

    /// `opusTree_Next(it)` (`0x00806a50`, and `0x00806aa0` and
    /// `0x0069ebd0`, the same code; `this` the
    /// iterator): the in-order successor of `*it` (nothing at the header);
    /// returns `it`.
    pub fn opus_tree_next(mem: &mut impl Mem, it: u32) -> u32 {
        let n = rd(mem, it);
        if nil(mem, n) {
            return it;
        }
        let right = rd(mem, n.wrapping_add(8));
        if !nil(mem, right) {
            let mut c = right;
            let mut l = rd(mem, c);
            while !nil(mem, l) {
                c = l;
                l = rd(mem, c);
            }
            mem.write(it, c);
            return it;
        }
        let mut p = rd(mem, n.wrapping_add(4));
        while !nil(mem, p) && rd(mem, it) == rd(mem, p.wrapping_add(8)) {
            mem.write(it, p);
            p = rd(mem, p.wrapping_add(4));
        }
        mem.write(it, p);
        it
    }

    /// `opusTree_Min(node)` (`0x00806db0`): the leftmost node below
    /// `node` (following `+0` until the next is nil).
    pub fn opus_tree_min(mem: &impl Mem, node: u32) -> u32 {
        extreme(mem, node, 0)
    }

    /// `opusTree_Max(node)` (`0x00806d70`): the rightmost (`+8`).
    pub fn opus_tree_max(mem: &impl Mem, node: u32) -> u32 {
        extreme(mem, node, 8)
    }

    fn extreme(mem: &impl Mem, mut node: u32, side: u32) -> u32 {
        let mut next = rd(mem, node.wrapping_add(side));
        while !nil(mem, next) {
            node = next;
            next = rd(mem, node.wrapping_add(side));
        }
        node
    }
}
