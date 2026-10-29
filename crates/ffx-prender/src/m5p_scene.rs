//! M5 stage 1, list `scene`, batch 1: the data-converter table and the
//! class destructors that end in `PClassDescriptor::Destruct`.
//!
//! **The converter table.** `PhyreEngine` converts a value between two of
//! twelve primitive types (`PChar` .. `bool`, the `PTypeDispenser`
//! getters) through a function picked by the pair. The pairs live in a
//! red-black tree at [`TREE`] (a sentinel head: `+0xc` the root, empty
//! when it points at the head itself), one function-local static node per
//! pair (`+0` `+4` the children, `+8` the parent with the colour in bit 0,
//! `+0xc` `+0x10` the key, `+0x14` the converter). Each of the 144
//! registrations ([`CONVERTERS`], 192 bytes each in `FFX.exe`,
//! `0x0046aad0..0x00471610`) builds its node once (its guard word bit 0),
//! registers the node's destructor with `atexit`, then attaches it with
//! [`bridge::tree_attach`] and runs the tree's insert fix-up
//! (`Phyre_PRedBlackTree_postInsertFinalize`) when it went in.
//! [`bridge::converter_find`] (`0x0047b630`) runs all registrations on its
//! first call (in [`REGISTER_ORDER`]) and looks a pair up.
//!
//! **Destructor tails** ([`VT_TAILS`]): the destructor of a class whose
//! base is `PClassDescriptor`: it stores the class's vtable and tail-jumps
//! to `PClassDescriptor::Destruct` (thiscall), whose `eax` it returns.
//!
//! Image addresses in the tables are rebased before use. Nothing here
//! keeps state of its own (worker threads: per-call scratch only).

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// The converter tree's head (a `PRedBlackTree` global).
pub const TREE: u32 = 0x00c0_e2f0;
/// `Phyre_PRedBlackTree_postInsertFinalize` (thiscall on the tree,
/// `(node)`, `ret 4`).
pub const TREE_FIXUP: u32 = 0x0057_4980;
/// The CRT's `atexit` (cdecl).
pub const ATEXIT: u32 = 0x0094_9207;
/// `PClassDescriptor::Destruct` (thiscall, no stack words).
pub const DESTRUCT: u32 = 0x0043_b3e0;
/// The byte `0x0047b630` sets once the registrations ran.
pub const REGISTERED: u32 = 0x00c9_75ec;
/// The converter tree's attach (`0x00477710`).
pub const ATTACH: u32 = 0x0047_7710;
/// The converter tree's find (`0x00477150`).
pub const FIND: u32 = 0x0047_7150;
/// The pair lookup that registers everything first (`0x0047b630`).
pub const CONVERTER_FIND: u32 = 0x0047_b630;

/// One registration of a converter (see the module doc).
#[derive(Clone, Copy, Debug)]
pub struct Converter {
    /// The copy's address.
    pub addr: u32,
    /// Its guard word (bit 0: the node is built).
    pub guard: u32,
    /// Its static node.
    pub entry: u32,
    /// The type getters, in the order it calls them (cdecl, no
    /// arguments): the first's result goes to `+0x10`, the second's to
    /// `+0xc`.
    pub types: [u32; 2],
    /// The converter stored at `+0x14`.
    pub convert: u32,
    /// The node's destructor, registered with `atexit`.
    pub dtor: u32,
}

include!("m5p_scene_table.rs.in");

/// The row of registration `f` (compile time: a missing copy is a build
/// error).
#[must_use]
pub const fn converter(f: u32) -> &'static Converter {
    let mut i = 0;
    while i < CONVERTERS.len() {
        if CONVERTERS[i].addr == f {
            return &CONVERTERS[i];
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The vtable destructor tail `f` stores (compile time, as [`converter`]).
#[must_use]
pub const fn vt_tail(f: u32) -> u32 {
    let mut i = 0;
    while i < VT_TAILS.len() {
        if VT_TAILS[i].0 == f {
            return VT_TAILS[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

pub mod bridge {
    //! The ports over game memory.

    use super::{
        ATEXIT, CONVERTERS, Conv, Converter, DESTRUCT, Host, Mem, REGISTER_ORDER, REGISTERED, TREE,
        TREE_FIXUP,
    };

    /// Which child of `node` the key `(k0, k1)` goes under: 0 for a key
    /// below the node's (`+0xc`, then `+0x10`, unsigned), 1 above, `None`
    /// for the same key. `k1` is read only when the first words tie, as
    /// the originals do.
    fn side<M: Mem>(m: &M, node: u32, k0: u32, k1: impl Fn(&M) -> u32) -> Option<u32> {
        let n0: u32 = m.read(node + 0xc);
        if k0 < n0 {
            return Some(0);
        }
        if n0 < k0 {
            return Some(1);
        }
        let k1 = k1(m);
        let n1: u32 = m.read(node + 0x10);
        if k1 < n1 {
            Some(0)
        } else if n1 >= k1 {
            None
        } else {
            Some(1)
        }
    }

    /// `0x00477710`, thiscall on the tree head, `(node)`, `ret 4`: makes
    /// `node` a red leaf (children: the head; bit 0 of `+8` set) and walks
    /// down by its key (`+0xc`, `+0x10`). Finds a node with that key:
    /// returns it, nothing else changes. Else hangs `node` under the last
    /// node visited (or as the root: under `head + 0xc`), keeping the
    /// colour bit in the parent word, and returns 0.
    pub fn tree_attach(m: &mut impl Mem, tree: u32, node: u32) -> u32 {
        let lo: u8 = m.read(node + 8);
        let parent_word: u32 = m.read(node + 8);
        m.write(node + 8, parent_word.wrapping_add(1 - u32::from(lo & 1)));
        m.write(node, tree);
        m.write(node + 4, tree);
        let k0: u32 = m.read(node + 0xc);
        // the empty tree: the root slot of the head
        let mut parent = tree + 0xc;
        let mut right = 0;
        let mut cur: u32 = m.read(parent);
        while cur != tree {
            parent = cur;
            match side(m, cur, k0, |m| m.read(node + 0x10)) {
                None => return cur,
                Some(s) => right = s,
            }
            cur = m.read(cur + 4 * right);
        }
        let lo: u8 = m.read(node + 8);
        m.write(node + 8, u32::from(lo & 1).wrapping_add(parent));
        m.write(parent.wrapping_add(4 * right), node);
        0
    }

    /// `0x00477150`, thiscall on the tree head, `(&key)`, `ret 4`: the
    /// node with the key `(key[0], key[1])`, or 0. The key is read only
    /// when the tree is not empty.
    pub fn tree_find(m: &impl Mem, tree: u32, key: u32) -> u32 {
        let mut cur: u32 = m.read(tree + 0xc);
        if cur == tree {
            return 0;
        }
        let k0: u32 = m.read(key);
        while cur != tree {
            match side(m, cur, k0, |m| m.read(key + 4)) {
                None => return cur,
                Some(s) => cur = m.read(cur + 4 * s),
            }
        }
        0
    }

    /// [`tree_find`] for a key held in registers (the caller's frame,
    /// which no callee sees).
    fn tree_find_value(m: &impl Mem, tree: u32, [k0, k1]: [u32; 2]) -> u32 {
        let mut cur: u32 = m.read(tree + 0xc);
        while cur != tree {
            match side(m, cur, k0, |_| k1) {
                None => return cur,
                Some(s) => cur = m.read(cur + 4 * s),
            }
        }
        0
    }

    /// One registration (`r`), cdecl, no arguments: builds the node once,
    /// then attaches it to the converter tree and fixes the tree up when
    /// it went in. Its `eax`: the node already there, or the fix-up's.
    pub fn register(h: &mut impl Host, r: &Converter) -> u32 {
        let guard = h.rebase(r.guard);
        let entry = h.rebase(r.entry);
        let tree = h.rebase(TREE);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            let first = h.call_sym(r.types[0], Conv::Cdecl, &[]);
            let second = h.call_sym(r.types[1], Conv::Cdecl, &[]);
            h.write(entry, entry);
            h.write(entry + 4, entry);
            h.write(entry + 8, entry);
            h.write(entry + 0xc, second);
            h.write(entry + 0x10, first);
            let convert = h.rebase(r.convert);
            h.write(entry + 0x14, convert);
            let dtor = h.rebase(r.dtor);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        let found = tree_attach(h, tree, entry);
        if found != 0 {
            return found;
        }
        h.call_sym(TREE_FIXUP, Conv::Thiscall(tree), &[entry])
    }

    /// `0x0047b630`, cdecl `(k0, k1)`: on the first call every
    /// registration (in [`REGISTER_ORDER`]), then the converter of the pair
    /// `(k0, k1)`, or 0.
    pub fn converter_find(h: &mut impl Host, k0: u32, k1: u32) -> u32 {
        let flag = h.rebase(REGISTERED);
        let done: u8 = h.read(flag);
        if done == 0 {
            for f in REGISTER_ORDER {
                if let Some(r) = CONVERTERS.iter().find(|r| r.addr == f) {
                    register(h, r);
                }
            }
            h.write(flag, 1u8);
        }
        let tree = h.rebase(TREE);
        let node = tree_find_value(h, tree, [k0, k1]);
        // the original tests the field's address too (never 0 for a node)
        if node == 0 || node.wrapping_add(0x14) == 0 {
            return 0;
        }
        h.read(node + 0x14)
    }

    /// A destructor tail: stores `vtable` (an image address) at `this` and
    /// tail-jumps to `PClassDescriptor::Destruct`; its `eax`.
    pub fn vt_tail_destruct(h: &mut impl Host, this: u32, vtable: u32) -> u32 {
        let vt = h.rebase(vtable);
        h.write(this, vt);
        h.call_sym(DESTRUCT, Conv::Thiscall(this), &[])
    }
}
