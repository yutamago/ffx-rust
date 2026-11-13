//! M5 `system-b`: the standard-library templates of this list (MSVC's
//! `std::_Tree` insert and erase, a `std::string` move, a word vector's
//! growth and `push_back`) and the `AssetListReader` symbol map built on
//! them (`std::map<std::string, ...>`: its equal range, key test, lookup
//! and erase).
//!
//! Tree nodes: `+0` left, `+4` parent, `+8` right, `+0xc` the colour
//! (1 black), `+0xd` the nil flag, the key from `+0x10`. The tree object:
//! `+0` the head (its `+4` the root, `+0` the leftmost, `+8` the
//! rightmost), `+4` the size. A `std::string`: 16 bytes of buffer (or the
//! heap pointer from capacity 16 on), `+0x10` the size, `+0x14` the
//! capacity.

#![allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]

/// `std::_Tree` helpers (thiscall on the tree, `ret 4`):
/// `_Lrotate`/`_Rrotate`; `_Min`/`_Max` (cdecl, the node).
pub const ROTATE_LEFT: u32 = 0x0069_a3f0;
pub const ROTATE_RIGHT: u32 = 0x0069_a4f0;
pub const TREE_MIN: u32 = 0x0069_a470;
pub const TREE_MAX: u32 = 0x0069_a450;

/// The callees of a `std::_Tree::erase(iterator)` copy: its rotations
/// (thiscall `(this; node)`), `_Min`/`_Max` (cdecl `(node)`), and the
/// offset of a `std::string` key the copy frees before the node (wp5e:
/// the string maps' copies `0x00636e20` and its siblings).
#[derive(Clone, Copy)]
pub struct EraseCalls {
    pub rotate_left: u32,
    pub rotate_right: u32,
    pub min: u32,
    pub max: u32,
    pub string_key: Option<u32>,
}

/// `ClusterMap_erase_0069b010`'s callees (no key to free).
pub const CLUSTER_ERASE: EraseCalls = EraseCalls {
    rotate_left: ROTATE_LEFT,
    rotate_right: ROTATE_RIGHT,
    min: TREE_MIN,
    max: TREE_MAX,
    string_key: None,
};
/// `operator_new` / `operator_delete` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `MSVCP110` `std::_Xlength_error`, `std::_Xout_of_range`,
/// `std::_Xbad_alloc` (cdecl; they throw); `MSVCR110` `memmove`.
pub const XLENGTH_IAT: u32 = 0x00b0_c29c;
pub const XOUT_OF_RANGE_IAT: u32 = 0x00b0_c25c;
pub const XBAD_ALLOC_IAT: u32 = 0x00b0_c2a0;
pub const MEMMOVE_IAT: u32 = 0x00b0_c34c;
/// `"map/set<T> too long"`, `"invalid map/set<T> iterator"`,
/// `"vector<T> too long"`.
pub const MAP_TOO_LONG: u32 = 0x00b4_12ac;
pub const BAD_ITERATOR: u32 = 0x00b4_1188;
pub const VECTOR_TOO_LONG: u32 = 0x00b3_4498;
/// `DJB_hash(s)` (cdecl).
pub const DJB_HASH: u32 = 0x0067_b7d0;
/// `KERNEL32` `WaitForSingleObject`, `ReleaseMutex` (stdcall).
pub const WAIT_IAT: u32 = 0x00b0_c0f8;
pub const RELEASE_MUTEX_IAT: u32 = 0x00b0_c1c0;
/// The `AssetListReader` map's callees: `0x00635c40(this; first, last)`
/// (a vector's range destroy, thiscall `ret 8`), `0x00635c80(this;
/// node)` (the tree's subtree erase, `ret 4`), `0x006f3520(first, last,
/// &count, 0)` (cdecl: the range's length), `0x00636d60(this; &out,
/// first, last)` (the range erase, `ret 0xc`), the string assign
/// `0x00404270(this; s, n)` (`ret 8`), `lower_bound` `0x006e9920(this;
/// &key)` (`ret 4`), the string compare `0x006590d0(this; pos, n, s,
/// len)` (`ret 0x10`) and `operator[]` `0x006f3ae0(this; &key)` (`ret
/// 4`, the value's address).
pub const VECTOR_DESTROY: u32 = 0x0063_5c40;
pub const TREE_ERASE_SUBTREE: u32 = 0x0063_5c80;
pub const RANGE_COUNT: u32 = 0x006f_3520;
pub const TREE_ERASE_RANGE: u32 = 0x0063_6d60;
pub const STRING_ASSIGN: u32 = 0x0040_4270;
pub const LOWER_BOUND: u32 = 0x006e_9920;
pub const STRING_COMPARE: u32 = 0x0065_90d0;
pub const MAP_INDEX: u32 = 0x006f_3ae0;

/// `has_key`'s frame: the key string at `ebp - 0x28`, the result at
/// `ebp - 0x2c`.
pub const HAS_KEY_FRAME: u32 = 0x2c;
/// `get_symbol_id`'s frame: the key string at `ebp - 0x28`; `has_key`'s
/// `ebp` is 0x40 below its own.
pub const SYMBOL_FRAME: u32 = 0x28;
pub const SYMBOL_HAS_KEY_EBP: u32 = 0x40;

/// The originals over game memory.
pub mod bridge {
    use super::{
        BAD_ITERATOR, DJB_HASH, LOWER_BOUND, MAP_INDEX, MAP_TOO_LONG, MEMMOVE_IAT, OPERATOR_DELETE,
        OPERATOR_NEW, RANGE_COUNT, RELEASE_MUTEX_IAT, STRING_ASSIGN, STRING_COMPARE,
        SYMBOL_HAS_KEY_EBP, TREE_ERASE_RANGE, TREE_ERASE_SUBTREE, VECTOR_DESTROY, VECTOR_TOO_LONG,
        WAIT_IAT, XBAD_ALLOC_IAT, XLENGTH_IAT, XOUT_OF_RANGE_IAT,
    };
    use crate::stl::bridge::{memcmp, tree_iterator_inc, tree_rotate_left, tree_rotate_right};
    use ffx_graphic::host::{Conv, Host};

    /// Calls the import in `slot` (its `ffx.sym` address).
    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    fn nil(h: &impl Host, node: u32) -> bool {
        h.read::<u8>(node.wrapping_add(0xd)) != 0
    }

    fn red(h: &impl Host, node: u32) -> bool {
        h.read::<u8>(node.wrapping_add(0xc)) == 0
    }

    fn colour(h: &mut impl Host, node: u32, black: u8) {
        h.write(node.wrapping_add(0xc), black);
    }

    fn parent(h: &impl Host, node: u32) -> u32 {
        h.read(node.wrapping_add(4))
    }

    /// `std::_Tree::_Insert_at(this; &out, addLeft, where, value,
    /// node)` (`0x006993d0`, thiscall, `ret 0x14`; the value is not read):
    /// `_Xlength_error` (after freeing the node) past 0x0aaaaaa8 entries;
    /// the size counted, `node` hung left or right of `where` (or as the
    /// root of an empty tree) with the head's extremes kept, then the
    /// red-black fix-up with the rotations inlined; the root black;
    /// `out` = `node`. Returns `out`.
    pub fn tree_insert_at(
        h: &mut impl Host,
        this: u32,
        out: u32,
        add_left: u32,
        at: u32,
        node: u32,
    ) -> u32 {
        tree_insert_at_limited(h, this, out, add_left, at, node, 0x0aaa_aaa9)
    }

    /// [`tree_insert_at`] for a copy whose `max_size()` check is `size >=
    /// limit` (`0x0069c8e0`, the VFX render data table's: 0x07fffffe;
    /// m5-wp5e).
    pub fn tree_insert_at_limited(
        h: &mut impl Host,
        this: u32,
        out: u32,
        add_left: u32,
        at: u32,
        node: u32,
        limit: u32,
    ) -> u32 {
        let mut size: u32 = h.read(this.wrapping_add(4));
        if size >= limit {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[node]);
            let msg = h.rebase(MAP_TOO_LONG);
            size = import(h, XLENGTH_IAT, Conv::Cdecl, &[msg]);
        }
        h.write(this.wrapping_add(4), size.wrapping_add(1));
        h.write(node.wrapping_add(4), at);
        let head: u32 = h.read(this);
        if at == head {
            h.write(head.wrapping_add(4), node);
            let head: u32 = h.read(this);
            h.write(head, node);
            let head: u32 = h.read(this);
            h.write(head.wrapping_add(8), node);
        } else if add_left & 0xff != 0 {
            h.write(at, node);
            let head: u32 = h.read(this);
            if at == h.read::<u32>(head) {
                h.write(head, node);
            }
        } else {
            h.write(at.wrapping_add(8), node);
            let head: u32 = h.read(this);
            if at == h.read::<u32>(head.wrapping_add(8)) {
                h.write(head.wrapping_add(8), node);
            }
        }
        let mut x = node;
        while red(h, parent(h, x)) {
            let p = parent(h, x);
            let g = parent(h, p);
            if p == h.read::<u32>(g) {
                let uncle: u32 = h.read(g.wrapping_add(8));
                if red(h, uncle) {
                    colour(h, p, 1);
                    colour(h, uncle, 1);
                    colour(h, parent(h, parent(h, x)), 0);
                    x = parent(h, parent(h, x));
                    continue;
                }
                if x == h.read::<u32>(p.wrapping_add(8)) {
                    x = p;
                    tree_rotate_left(h, this, x);
                }
                colour(h, parent(h, x), 1);
                colour(h, parent(h, parent(h, x)), 0);
                let g = parent(h, parent(h, x));
                tree_rotate_right(h, this, g);
            } else {
                let uncle: u32 = h.read(g);
                if red(h, uncle) {
                    colour(h, p, 1);
                    colour(h, uncle, 1);
                    colour(h, parent(h, parent(h, x)), 0);
                    x = parent(h, parent(h, x));
                    continue;
                }
                if x == h.read::<u32>(p) {
                    x = p;
                    tree_rotate_right(h, this, x);
                }
                colour(h, parent(h, x), 1);
                colour(h, parent(h, parent(h, x)), 0);
                let g = parent(h, parent(h, x));
                tree_rotate_left(h, this, g);
            }
        }
        let head: u32 = h.read(this);
        let root: u32 = h.read(head.wrapping_add(4));
        colour(h, root, 1);
        h.write(out, node);
        out
    }

    /// `ClusterMap_erase_0069b010(this; &out, where)` (thiscall, `ret 8`;
    /// `slot` the caller's `where` word, which the original advances in
    /// place): `std::_Tree::erase(iterator)`: `_Xout_of_range` for the
    /// head; the successor taken (`++where`); the node unlinked (its
    /// successor moved into its place when it has two children, colours
    /// swapped), the head's extremes fixed (`_Min`/`_Max`), the
    /// red-black fix-up through the original rotations; `operator delete`,
    /// the size counted down (not below 0); `out` = the successor.
    /// Returns `out`.
    pub fn tree_erase(h: &mut impl Host, this: u32, out: u32, slot: u32) -> u32 {
        tree_erase_with(h, this, out, slot, &super::CLUSTER_ERASE)
    }

    /// [`tree_erase`] for a copy with other callees ([`super::EraseCalls`]):
    /// with a string key, after the fix-up the key's heap block freed (a
    /// capacity of 16 or more) and the key emptied, then the node deleted.
    #[allow(clippy::too_many_lines)] // one MSVC routine
    pub fn tree_erase_with(
        h: &mut impl Host,
        this: u32,
        out: u32,
        slot: u32,
        calls: &super::EraseCalls,
    ) -> u32 {
        let erased: u32 = h.read(slot);
        if nil(h, erased) {
            let msg = h.rebase(BAD_ITERATOR);
            import(h, XOUT_OF_RANGE_IAT, Conv::Cdecl, &[msg]);
        }
        tree_iterator_inc(h, slot);
        let left: u32 = h.read(erased);
        let mut pnode = erased;
        let fix: u32 = if nil(h, left) {
            h.read(erased.wrapping_add(8))
        } else {
            let right: u32 = h.read(erased.wrapping_add(8));
            if nil(h, right) {
                left
            } else {
                pnode = h.read(slot);
                h.read(pnode.wrapping_add(8))
            }
        };
        let fix_parent;
        if pnode == erased {
            fix_parent = parent(h, erased);
            if !nil(h, fix) {
                h.write(fix.wrapping_add(4), fix_parent);
            }
            let head: u32 = h.read(this);
            if h.read::<u32>(head.wrapping_add(4)) == erased {
                h.write(head.wrapping_add(4), fix);
            } else if h.read::<u32>(fix_parent) == erased {
                h.write(fix_parent, fix);
            } else {
                h.write(fix_parent.wrapping_add(8), fix);
            }
            let head: u32 = h.read(this);
            if h.read::<u32>(head) == erased {
                let v = if nil(h, fix) {
                    fix_parent
                } else {
                    h.call_sym(calls.min, Conv::Cdecl, &[fix])
                };
                let head: u32 = h.read(this);
                h.write(head, v);
            }
            let head: u32 = h.read(this);
            if h.read::<u32>(head.wrapping_add(8)) == erased {
                let v = if nil(h, fix) {
                    fix_parent
                } else {
                    h.call_sym(calls.max, Conv::Cdecl, &[fix])
                };
                let head: u32 = h.read(this);
                h.write(head.wrapping_add(8), v);
            }
        } else {
            h.write(left.wrapping_add(4), pnode);
            h.write(pnode, left);
            if pnode == h.read::<u32>(erased.wrapping_add(8)) {
                fix_parent = pnode;
            } else {
                fix_parent = parent(h, pnode);
                if !nil(h, fix) {
                    h.write(fix.wrapping_add(4), fix_parent);
                }
                h.write(fix_parent, fix);
                let right: u32 = h.read(erased.wrapping_add(8));
                h.write(pnode.wrapping_add(8), right);
                let right: u32 = h.read(erased.wrapping_add(8));
                h.write(right.wrapping_add(4), pnode);
            }
            let head: u32 = h.read(this);
            if h.read::<u32>(head.wrapping_add(4)) == erased {
                h.write(head.wrapping_add(4), pnode);
            } else {
                let p = parent(h, erased);
                if h.read::<u32>(p) == erased {
                    h.write(p, pnode);
                } else {
                    h.write(p.wrapping_add(8), pnode);
                }
            }
            let p = parent(h, erased);
            h.write(pnode.wrapping_add(4), p);
            let a: u8 = h.read(erased.wrapping_add(0xc));
            let b: u8 = h.read(pnode.wrapping_add(0xc));
            h.write(pnode.wrapping_add(0xc), a);
            h.write(erased.wrapping_add(0xc), b);
        }
        if h.read::<u8>(erased.wrapping_add(0xc)) == 1 {
            erase_fixup(h, this, fix, fix_parent, calls);
        }
        if let Some(k) = calls.string_key {
            free_string(h, erased.wrapping_add(k));
        }
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[erased]);
        let size: u32 = h.read(this.wrapping_add(4));
        if size != 0 {
            h.write(this.wrapping_add(4), size - 1);
        }
        let next: u32 = h.read(slot);
        h.write(out, next);
        out
    }

    /// A `std::string`'s heap block freed (a capacity of 16 or more) and
    /// the string emptied (capacity 15, size 0, the first byte 0).
    fn free_string(h: &mut impl Host, s: u32) {
        if h.read::<u32>(s.wrapping_add(0x14)) >= 0x10 {
            let p: u32 = h.read(s);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
        }
        h.write(s.wrapping_add(0x14), 0xfu32);
        h.write(s.wrapping_add(0x10), 0u32);
        h.write(s, 0u8);
    }

    fn black(h: &impl Host, node: u32) -> bool {
        h.read::<u8>(node.wrapping_add(0xc)) == 1
    }

    fn root(h: &impl Host, this: u32) -> u32 {
        let head: u32 = h.read(this);
        h.read(head.wrapping_add(4))
    }

    /// The erase's red-black fix-up from `fix` (under `fix_parent`).
    fn erase_fixup(
        h: &mut impl Host,
        this: u32,
        mut fix: u32,
        mut fix_parent: u32,
        calls: &super::EraseCalls,
    ) {
        if fix != root(h, this) {
            while black(h, fix) {
                let left: u32 = h.read(fix_parent);
                if fix == left {
                    let mut p: u32 = h.read(fix_parent.wrapping_add(8));
                    if red(h, p) {
                        colour(h, p, 1);
                        colour(h, fix_parent, 0);
                        h.call_sym(calls.rotate_left, Conv::Thiscall(this), &[fix_parent]);
                        p = h.read(fix_parent.wrapping_add(8));
                    }
                    if nil(h, p) {
                        // nothing to recolour: move up
                    } else if black(h, h.read(p)) && black(h, h.read(p.wrapping_add(8))) {
                        colour(h, p, 0);
                    } else {
                        if black(h, h.read(p.wrapping_add(8))) {
                            let l: u32 = h.read(p);
                            colour(h, l, 1);
                            colour(h, p, 0);
                            h.call_sym(calls.rotate_right, Conv::Thiscall(this), &[p]);
                            p = h.read(fix_parent.wrapping_add(8));
                        }
                        let c: u8 = h.read(fix_parent.wrapping_add(0xc));
                        colour(h, p, c);
                        colour(h, fix_parent, 1);
                        let r: u32 = h.read(p.wrapping_add(8));
                        colour(h, r, 1);
                        h.call_sym(calls.rotate_left, Conv::Thiscall(this), &[fix_parent]);
                        break;
                    }
                } else {
                    let mut p = left;
                    if red(h, p) {
                        colour(h, p, 1);
                        colour(h, fix_parent, 0);
                        h.call_sym(calls.rotate_right, Conv::Thiscall(this), &[fix_parent]);
                        p = h.read(fix_parent);
                    }
                    if nil(h, p) {
                        // nothing to recolour: move up
                    } else if black(h, h.read(p.wrapping_add(8))) && black(h, h.read(p)) {
                        colour(h, p, 0);
                    } else {
                        if black(h, h.read(p)) {
                            let r: u32 = h.read(p.wrapping_add(8));
                            colour(h, r, 1);
                            colour(h, p, 0);
                            h.call_sym(calls.rotate_left, Conv::Thiscall(this), &[p]);
                            p = h.read(fix_parent);
                        }
                        let c: u8 = h.read(fix_parent.wrapping_add(0xc));
                        colour(h, p, c);
                        colour(h, fix_parent, 1);
                        let l: u32 = h.read(p);
                        colour(h, l, 1);
                        h.call_sym(calls.rotate_right, Conv::Thiscall(this), &[fix_parent]);
                        break;
                    }
                }
                fix = fix_parent;
                fix_parent = parent(h, fix_parent);
                if fix == root(h, this) {
                    break;
                }
            }
        }
        colour(h, fix, 1);
    }

    /// `0x0069b7c0(this; name)` (thiscall, `ret 4`) -> 0/1: under the
    /// mutex at `**(this + 0x18)`, whether the set at `this + 0x10` (node
    /// keys `+0x10`, words) holds `DJB_hash(name)`.
    pub fn hash_set_contains(h: &mut impl Host, this: u32, name: u32) -> u32 {
        let key = h.call_sym(DJB_HASH, Conv::Cdecl, &[name]);
        let lock: u32 = h.read(this.wrapping_add(0x18));
        let mutex: u32 = h.read(lock);
        import(h, WAIT_IAT, Conv::Stdcall, &[mutex, 0xffff_ffff]);
        let head: u32 = h.read(this.wrapping_add(0x10));
        let mut best = head;
        let mut n: u32 = h.read(head.wrapping_add(4));
        while !nil(h, n) {
            if h.read::<u32>(n.wrapping_add(0x10)) < key {
                n = h.read(n.wrapping_add(8));
            } else {
                best = n;
                n = h.read(n);
            }
        }
        let found = if best == head || key < h.read::<u32>(best.wrapping_add(0x10)) {
            head
        } else {
            best
        };
        let lock: u32 = h.read(this.wrapping_add(0x18));
        let mutex: u32 = h.read(lock);
        import(h, RELEASE_MUTEX_IAT, Conv::Stdcall, &[mutex]);
        u32::from(found != head)
    }

    /// `0x0069c200(this; src, _)` (thiscall, `ret 8`; the second word is
    /// not read): a `std::string` move-construct from `src` (heap
    /// pointer taken, or `memmove` of `size + 1` bytes), `src` left empty,
    /// then the word after the strings (`+0x18`) copied. Returns `this`.
    pub fn string_move_with_word(h: &mut impl Host, this: u32, src: u32) -> u32 {
        h.write(this.wrapping_add(0x14), 0xfu32);
        h.write(this.wrapping_add(0x10), 0u32);
        h.write(this, 0u8);
        if h.read::<u32>(src.wrapping_add(0x14)) < 0x10 {
            let n = h.read::<u32>(src.wrapping_add(0x10)).wrapping_add(1);
            if n != 0 {
                import(h, MEMMOVE_IAT, Conv::Cdecl, &[this, src, n]);
            }
        } else {
            let p: u32 = h.read(src);
            h.write(this, p);
            h.write(src, 0u32);
        }
        let size: u32 = h.read(src.wrapping_add(0x10));
        h.write(this.wrapping_add(0x10), size);
        let cap: u32 = h.read(src.wrapping_add(0x14));
        h.write(this.wrapping_add(0x14), cap);
        h.write(src.wrapping_add(0x14), 0xfu32);
        h.write(src.wrapping_add(0x10), 0u32);
        h.write(src, 0u8);
        let w: u32 = h.read(src.wrapping_add(0x18));
        h.write(this.wrapping_add(0x18), w);
        this
    }

    /// A `std::string`'s characters.
    fn chars(h: &impl Host, s: u32) -> u32 {
        if h.read::<u32>(s.wrapping_add(0x14)) >= 0x10 {
            h.read(s)
        } else {
            s
        }
    }

    /// `basic_string::compare` as the map inlines it: `memcmp` of the
    /// shorter length (by unsigned bytes), then the lengths; -1, 0 or 1.
    fn compare(h: &impl Host, a: u32, b: u32) -> i32 {
        let la: u32 = h.read(a.wrapping_add(0x10));
        let lb: u32 = h.read(b.wrapping_add(0x10));
        let r = memcmp(h, chars(h, a), chars(h, b), la.min(lb));
        if r != 0 {
            return r.signum();
        }
        if la < lb { -1 } else { i32::from(la != lb) }
    }

    /// `0x006f3c60(this; &out, key)` (thiscall, `ret 8`):
    /// `std::map<std::string, ...>::equal_range(key)`: the lower bound,
    /// noting on the way the first node greater than the key (`memcmp_00401020`
    /// for the characters); the upper bound searched from that node's
    /// left (or the root); `out` = (lower, upper). Returns `out`.
    pub fn string_map_equal_range(h: &mut impl Host, this: u32, out: u32, key: u32) -> u32 {
        let head: u32 = h.read(this);
        let root: u32 = h.read(head.wrapping_add(4));
        let mut lower = head;
        let mut upper = head;
        let mut n = root;
        while !nil(h, n) {
            if compare(h, n.wrapping_add(0x10), key) < 0 {
                n = h.read(n.wrapping_add(8));
                continue;
            }
            if nil(h, upper) && compare(h, key, n.wrapping_add(0x10)) < 0 {
                upper = n;
            }
            lower = n;
            n = h.read(n);
        }
        let mut n = if nil(h, upper) { root } else { h.read(upper) };
        while !nil(h, n) {
            if compare(h, key, n.wrapping_add(0x10)) < 0 {
                upper = n;
                n = h.read(n);
            } else {
                n = h.read(n.wrapping_add(8));
            }
        }
        h.write(out, lower);
        h.write(out.wrapping_add(4), upper);
        out
    }

    /// `std::vector<word>::_Reallocate(this; capacity)` (`0x006f3ee0`,
    /// thiscall, `ret 4`): a new block (`operator new`, `_Xbad_alloc` past
    /// 0x3fffffff words or when it fails), the words moved (`memmove`),
    /// the old block deleted; begin, end and capacity set. Returns the new
    /// end.
    pub fn vector_reallocate(h: &mut impl Host, this: u32, capacity: u32) -> u32 {
        let mut p = 0;
        if capacity != 0 {
            if capacity <= 0x3fff_ffff {
                p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[capacity.wrapping_mul(4)]);
            }
            if p == 0 {
                import(h, XBAD_ALLOC_IAT, Conv::Cdecl, &[]);
            }
        }
        let begin: u32 = h.read(this);
        let end: u32 = h.read(this.wrapping_add(4));
        import(
            h,
            MEMMOVE_IAT,
            Conv::Cdecl,
            &[p, begin, end.wrapping_sub(begin) & !3],
        );
        let begin: u32 = h.read(this);
        let end: u32 = h.read(this.wrapping_add(4));
        let count = (end.wrapping_sub(begin) as i32 >> 2) as u32;
        if begin != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[begin]);
        }
        h.write(
            this.wrapping_add(8),
            p.wrapping_add(capacity.wrapping_mul(4)),
        );
        h.write(this, p);
        let end = p.wrapping_add(count.wrapping_mul(4));
        h.write(this.wrapping_add(4), end);
        end
    }

    /// `std::vector<word>::_Reserve(this; more)` (`0x006f3f60`, thiscall,
    /// `ret 4`): nothing while `more` words fit (returns the room left);
    /// `_Xlength_error` past 0x3fffffff; else the capacity grows by half
    /// (not past the limit, at least to the needed size) and
    /// [`vector_reallocate`]'s result is returned.
    pub fn vector_reserve(h: &mut impl Host, this: u32, more: u32) -> u32 {
        let cap_end: u32 = h.read(this.wrapping_add(8));
        let end: u32 = h.read(this.wrapping_add(4));
        let room = (cap_end.wrapping_sub(end) as i32 >> 2) as u32;
        if room >= more {
            return room;
        }
        let begin: u32 = h.read(this);
        let size = (end.wrapping_sub(begin) as i32 >> 2) as u32;
        if 0x3fff_ffff - size < more {
            let msg = h.rebase(VECTOR_TOO_LONG);
            import(h, XLENGTH_IAT, Conv::Cdecl, &[msg]);
        }
        let need = size.wrapping_add(more);
        let begin: u32 = h.read(this);
        let cap = (cap_end.wrapping_sub(begin) as i32 >> 2) as u32;
        let half = cap >> 1;
        let mut new_cap = if 0x3fff_ffff - half < cap {
            0
        } else {
            cap.wrapping_add(half)
        };
        if new_cap < need {
            new_cap = need;
        }
        vector_reallocate(h, this, new_cap)
    }

    /// `std::vector<word>::push_back(this; &value)` (`0x006f4890`,
    /// thiscall, `ret 4`): a value inside the vector is re-read by index
    /// after [`vector_reserve`]`(1)` (when full); the slot written when
    /// not null; end advanced.
    pub fn vector_push_back(h: &mut impl Host, this: u32, value: u32) {
        let end: u32 = h.read(this.wrapping_add(4));
        let begin: u32 = h.read(this);
        let inside = value < end && begin <= value;
        let index = (value.wrapping_sub(begin) as i32 >> 2) as u32;
        if end == h.read::<u32>(this.wrapping_add(8)) {
            vector_reserve(h, this, 1);
        }
        let slot: u32 = h.read(this.wrapping_add(4));
        if slot != 0 {
            let v: u32 = if inside {
                let begin: u32 = h.read(this);
                h.read(begin.wrapping_add(index.wrapping_mul(4)))
            } else {
                h.read(value)
            };
            h.write(slot, v);
        }
        let end: u32 = h.read(this.wrapping_add(4));
        h.write(this.wrapping_add(4), end.wrapping_add(4));
    }

    /// `AssetListReader_clear(this)` (`0x006f4030`, thiscall): the two
    /// vectors at `+0x20` and `+0x14` destroyed and emptied, `+0xc` =
    /// `+8`, the map's nodes erased and its head reset, size 0. Returns
    /// the head.
    pub fn asset_list_clear(h: &mut impl Host, this: u32) -> u32 {
        for off in [0x20u32, 0x14] {
            let v = this.wrapping_add(off);
            let first: u32 = h.read(v);
            let last: u32 = h.read(v.wrapping_add(4));
            h.call_sym(VECTOR_DESTROY, Conv::Thiscall(v), &[first, last]);
            let first: u32 = h.read(v);
            h.write(v.wrapping_add(4), first);
        }
        let w: u32 = h.read(this.wrapping_add(8));
        h.write(this.wrapping_add(0xc), w);
        let head: u32 = h.read(this);
        let root: u32 = h.read(head.wrapping_add(4));
        h.call_sym(TREE_ERASE_SUBTREE, Conv::Thiscall(this), &[root]);
        let head: u32 = h.read(this);
        h.write(head.wrapping_add(4), head);
        let head: u32 = h.read(this);
        h.write(head, head);
        let head: u32 = h.read(this);
        h.write(head.wrapping_add(8), head);
        h.write(this.wrapping_add(4), 0u32);
        head
    }

    /// `0x006f40e0(this; key)` (thiscall, `ret 4`): `erase(key)`: the
    /// equal range into `range` (two words), its length counted into
    /// `count` (the original's own argument slot) by `0x006f3520`, the
    /// range erased (`0x00636d60`, its result iterator at `result`).
    /// Returns the count.
    pub fn string_map_erase_key(
        h: &mut impl Host,
        range: u32,
        result: u32,
        count: u32,
        this: u32,
        key: u32,
    ) -> u32 {
        let r = string_map_equal_range(h, this, range, key);
        let first: u32 = h.read(r);
        let last: u32 = h.read(r.wrapping_add(4));
        h.write(count, 0u32);
        h.call_sym(RANGE_COUNT, Conv::Cdecl, &[first, last, count, 0]);
        h.call_sym(
            TREE_ERASE_RANGE,
            Conv::Thiscall(this),
            &[result, first, last],
        );
        h.read(count)
    }

    /// Builds the frame string at `s` from the C string `name`
    /// (capacity 15, then the assign with `strlen`).
    fn frame_string(h: &mut impl Host, s: u32, name: u32) {
        h.write(s.wrapping_add(0x14), 0xfu32);
        h.write(s.wrapping_add(0x10), 0u32);
        h.write(s, 0u8);
        let mut n = 0u32;
        if h.read::<u8>(name) != 0 {
            while h.read::<u8>(name.wrapping_add(n)) != 0 {
                n = n.wrapping_add(1);
            }
        }
        h.call_sym(STRING_ASSIGN, Conv::Thiscall(s), &[name, n]);
    }

    /// Frees the frame string at `s` (heap buffer from capacity 16).
    fn drop_frame_string(h: &mut impl Host, s: u32) {
        if h.read::<u32>(s.wrapping_add(0x14)) >= 0x10 {
            let p: u32 = h.read(s);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
        }
    }

    /// `0x006f4410(this; name)` (thiscall, `ret 4`; `ebp` the original's
    /// frame pointer, the key string at `ebp - 0x28`, the found node at
    /// `ebp - 0x2c`; the SEH frame and the cookie are left out) -> `al`:
    /// whether the map holds `name` (`lower_bound`, then the string
    /// compare of the key with the node's key not negative).
    pub fn string_map_has_key(h: &mut impl Host, ebp: u32, this: u32, name: u32) -> u8 {
        let s = ebp.wrapping_sub(0x28);
        frame_string(h, s, name);
        let head: u32 = h.read(this);
        let it = h.call_sym(LOWER_BOUND, Conv::Thiscall(this), &[s]);
        let found_at = ebp.wrapping_sub(0x2c);
        let mut found = h.read::<u32>(this);
        if it != found {
            let klen: u32 = h.read(it.wrapping_add(0x20));
            let key = chars(h, it.wrapping_add(0x10));
            let slen: u32 = h.read(s.wrapping_add(0x10));
            let r = h.call_sym(STRING_COMPARE, Conv::Thiscall(s), &[0, slen, key, klen]);
            found = if (r as i32) < 0 { h.read(this) } else { it };
        }
        h.write(found_at, found);
        let yes = found != head;
        drop_frame_string(h, s);
        u8::from(yes)
    }

    /// `AssetListReader_getSymbolId(this; name)` (`0x006f4350`, thiscall,
    /// `ret 4`; `ebp` the original's frame pointer, the key string at
    /// `ebp - 0x28`; [`string_map_has_key`]'s frame 0x40 below): -1 when
    /// the map lacks `name`, else the word `operator[]` finds.
    pub fn get_symbol_id(h: &mut impl Host, ebp: u32, this: u32, name: u32) -> u32 {
        if string_map_has_key(h, ebp.wrapping_sub(SYMBOL_HAS_KEY_EBP), this, name) == 0 {
            return 0xffff_ffff;
        }
        let s = ebp.wrapping_sub(0x28);
        frame_string(h, s, name);
        let p = h.call_sym(MAP_INDEX, Conv::Thiscall(this), &[s]);
        let id: u32 = h.read(p);
        drop_frame_string(h, s);
        id
    }
}
