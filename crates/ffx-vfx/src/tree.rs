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

    /// The in-order successor of `n` (as [`opus_tree_next`] moves an
    /// iterator; `n` itself at the header).
    fn successor(mem: &impl Mem, n: u32) -> u32 {
        if nil(mem, n) {
            return n;
        }
        let right = rd(mem, n.wrapping_add(8));
        if !nil(mem, right) {
            return extreme(mem, right, 0);
        }
        let mut cur = n;
        let mut p = rd(mem, n.wrapping_add(4));
        while !nil(mem, p) && cur == rd(mem, p.wrapping_add(8)) {
            cur = p;
            p = rd(mem, p.wrapping_add(4));
        }
        p
    }

    /// `0x008075b0` (`opusTree_Erase`, ours; MSVC's `std::_Tree::erase`
    /// on an opus tree, thiscall `(this, out, at)`, `ret 8`): removes node
    /// `at` from the red-black tree `this` (`+0` the header, `+4` the
    /// size; nodes as above, byte `+0xc` the colour, 1 black) and frees
    /// it (`operator_delete`); `*out` = its successor, `out` returned. The
    /// header itself gets `std::_Xout_of_range("invalid map/set<T>
    /// iterator")` through the import, which throws (the original's code
    /// after the call assumes it does not return; here `out` is returned
    /// untouched if it does). The extreme nodes come from `0x00806dd0` (min) and
    /// `0x00806d90` (max) through `call`, the rotations from
    /// `0x00806d10` (left) and `0x00806f10` (right) through `this_call`;
    /// the size is lowered when not 0.
    #[allow(clippy::too_many_lines)]
    pub fn opus_tree_erase<M: Mem>(
        mem: &mut M,
        this: u32,
        out: u32,
        at: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        this_call: &mut impl FnMut(&mut M, u32, u32, &[u32]) -> u32,
    ) -> u32 {
        // the engine's rotations (thiscall `(this, node)`)
        const LEFT: u32 = 0x0080_6d10;
        const RIGHT: u32 = 0x0080_6f10;
        let sym = |mem: &M, a: u32| ffx_bridge::Global::<u32>::new(a).ptr(mem).addr;
        let left = |mem: &M, n: u32| rd(mem, n);
        let parent = |mem: &M, n: u32| rd(mem, n.wrapping_add(4));
        let right = |mem: &M, n: u32| rd(mem, n.wrapping_add(8));
        let black = |mem: &M, n: u32| mem.read::<u8>(n.wrapping_add(0xc)) == 1;
        let paint = |mem: &mut M, n: u32, c: u8| mem.write(n.wrapping_add(0xc), c);
        let head = |mem: &M| rd(mem, this);
        let mut rotate = |mem: &mut M, f: u32, n: u32| {
            let f = sym(mem, f);
            this_call(mem, f, this, &[n]);
        };
        if nil(mem, at) {
            let x = rd(mem, sym(mem, 0x00b0_c25c));
            let msg = sym(mem, 0x00b4_1188);
            call(mem, x, &[msg]);
            return out;
        }
        let next = successor(mem, at);
        let erased = at;
        // the node that takes the erased one's place, and its parent
        let (mut fix, mut fix_parent);
        let (l, r) = (left(mem, erased), right(mem, erased));
        if nil(mem, l) || nil(mem, r) || next == erased {
            fix = if nil(mem, l) { r } else { l };
            if !nil(mem, l) && !nil(mem, r) {
                fix = right(mem, next);
            }
            fix_parent = parent(mem, erased);
            if !nil(mem, fix) {
                mem.write(fix.wrapping_add(4), fix_parent);
            }
            let h = head(mem);
            if parent(mem, h) == erased {
                mem.write(h.wrapping_add(4), fix);
            } else if left(mem, fix_parent) == erased {
                mem.write(fix_parent, fix);
            } else {
                mem.write(fix_parent.wrapping_add(8), fix);
            }
            let h = head(mem);
            if left(mem, h) == erased {
                let m = if nil(mem, fix) {
                    fix_parent
                } else {
                    let f = sym(mem, 0x0080_6dd0);
                    call(mem, f, &[fix])
                };
                mem.write(head(mem), m);
            }
            let h = head(mem);
            if right(mem, h) == erased {
                let m = if nil(mem, fix) {
                    fix_parent
                } else {
                    let f = sym(mem, 0x0080_6d90);
                    call(mem, f, &[fix])
                };
                mem.write(head(mem).wrapping_add(8), m);
            }
        } else {
            // two children: the successor moves into the erased place
            let p = next;
            fix = right(mem, p);
            mem.write(l.wrapping_add(4), p);
            let l2 = left(mem, erased);
            mem.write(p, l2);
            if p == right(mem, erased) {
                fix_parent = p;
            } else {
                fix_parent = parent(mem, p);
                if !nil(mem, fix) {
                    mem.write(fix.wrapping_add(4), fix_parent);
                }
                mem.write(fix_parent, fix);
                let r2 = right(mem, erased);
                mem.write(p.wrapping_add(8), r2);
                let r2 = right(mem, erased);
                mem.write(r2.wrapping_add(4), p);
            }
            let h = head(mem);
            let ep = parent(mem, erased);
            if parent(mem, h) == erased {
                mem.write(h.wrapping_add(4), p);
            } else if left(mem, ep) == erased {
                mem.write(ep, p);
            } else {
                mem.write(ep.wrapping_add(8), p);
            }
            let ep = parent(mem, erased);
            mem.write(p.wrapping_add(4), ep);
            let (ce, cp): (u8, u8) = (
                mem.read(erased.wrapping_add(0xc)),
                mem.read(p.wrapping_add(0xc)),
            );
            paint(mem, p, ce);
            paint(mem, erased, cp);
        }
        if black(mem, erased) {
            // fix-up: `fix` carries an extra black up the tree
            while fix != parent(mem, head(mem)) && black(mem, fix) {
                if fix == left(mem, fix_parent) {
                    let mut w = right(mem, fix_parent);
                    if mem.read::<u8>(w.wrapping_add(0xc)) == 0 {
                        paint(mem, w, 1);
                        paint(mem, fix_parent, 0);
                        rotate(mem, LEFT, fix_parent);
                        w = right(mem, fix_parent);
                    }
                    if !nil(mem, w) {
                        if black(mem, left(mem, w)) && black(mem, right(mem, w)) {
                            paint(mem, w, 0);
                        } else {
                            if black(mem, right(mem, w)) {
                                paint(mem, left(mem, w), 1);
                                paint(mem, w, 0);
                                rotate(mem, RIGHT, w);
                                w = right(mem, fix_parent);
                            }
                            let c: u8 = mem.read(fix_parent.wrapping_add(0xc));
                            paint(mem, w, c);
                            paint(mem, fix_parent, 1);
                            paint(mem, right(mem, w), 1);
                            rotate(mem, LEFT, fix_parent);
                            break;
                        }
                    }
                } else {
                    let mut w = left(mem, fix_parent);
                    if mem.read::<u8>(w.wrapping_add(0xc)) == 0 {
                        paint(mem, w, 1);
                        paint(mem, fix_parent, 0);
                        rotate(mem, RIGHT, fix_parent);
                        w = left(mem, fix_parent);
                    }
                    if !nil(mem, w) {
                        if black(mem, right(mem, w)) && black(mem, left(mem, w)) {
                            paint(mem, w, 0);
                        } else {
                            if black(mem, left(mem, w)) {
                                paint(mem, right(mem, w), 1);
                                paint(mem, w, 0);
                                rotate(mem, LEFT, w);
                                w = left(mem, fix_parent);
                            }
                            let c: u8 = mem.read(fix_parent.wrapping_add(0xc));
                            paint(mem, w, c);
                            paint(mem, fix_parent, 1);
                            paint(mem, left(mem, w), 1);
                            rotate(mem, RIGHT, fix_parent);
                            break;
                        }
                    }
                }
                fix = fix_parent;
                fix_parent = parent(mem, fix_parent);
            }
            paint(mem, fix, 1);
        }
        let f = sym(mem, 0x0063_0390);
        call(mem, f, &[erased]);
        let n: u32 = mem.read(this.wrapping_add(4));
        if n != 0 {
            mem.write(this.wrapping_add(4), n - 1);
        }
        mem.write(out, next);
        out
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

    /// `0x0070fd70` (`vfxIntTree_InsertFinalize`, ours; thiscall `(this,
    /// node)`, `ret 4`): [`vfx_int_tree_insert`]; when that finds the key
    /// taken, 0x12; else the engine's
    /// `Phyre_PRedBlackTree_postInsertFinalize(node)` on `this` (through
    /// `this_call(f, this, args)`) and 0.
    pub fn vfx_int_tree_insert_finalize<M: Mem>(
        mem: &mut M,
        this: u32,
        node: u32,
        this_call: &mut impl FnMut(&mut M, u32, u32, &[u32]) -> u32,
    ) -> u32 {
        if vfx_int_tree_insert(mem, this, node) != 0 {
            return 0x12;
        }
        let f = ffx_bridge::Global::<u32>::new(0x0057_4980).ptr(mem).addr;
        this_call(mem, f, this, &[node]);
        0
    }
}
