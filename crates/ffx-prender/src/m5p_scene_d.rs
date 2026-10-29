//! M5 stage 1, list `scene`, batch 4: the entity class tree (attach,
//! fix-up after loading, lookups), array constructors of the render
//! classes, `PSharray` copies and resizes, the timer and scriptable
//! components, the LOD blend packer and the mesh data block search.
//! Conventions as in [`crate::m5p_scene_b`].
//!
//! Image addresses are rebased before use. Nothing here keeps state of
//! its own (worker threads: per-call scratch only).

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

pub use crate::m5p_scene_b::PHYRE_FREE;
pub use crate::m5p_scene_c::{CLASS_TREE, TREE_FIXUP, WORLD_CLASS};

/// `0x0044e960`, the one-word-key tree's attach (ported here).
pub const TREE_ATTACH1: u32 = 0x0044_e960;
/// `Phyre_PString_Assign` (thiscall `(this; text)`, `ret 4`).
pub const STRING_ASSIGN: u32 = 0x0044_5cf0;
/// `Phyre_PNamespace_GetGlobalNamespace` (cdecl).
pub const GLOBAL_NAMESPACE: u32 = 0x0043_e2f0;
/// `Phyre_PNamespace_getClassDescriptor` (thiscall `(name)`, `ret 4`).
pub const GET_CLASS_DESCRIPTOR: u32 = 0x0043_e950;
/// `0x00436ed0(state, object, class)` (cdecl): pushes an instance to
/// the script state.
pub const PUSH_INSTANCE: u32 = 0x0043_6ed0;
/// The entity class (`PEntity`'s descriptor, which `0x0044f6d0` looks for
/// on a class's base chain).
pub const ENTITY_CLASS: u32 = 0x00c9_2090;
/// `0x00435c10` (thiscall on a class, `(name)`, `ret 4`): a member of
/// that name.
pub const CLASS_MEMBER: u32 = 0x0043_5c10;
/// The member name `0x0044f6d0` asks for.
pub const ENTITY_MEMBER_NAME: u32 = 0x00b1_071c;
/// `0x0044e9e0(member, object)` (cdecl): the member's value.
pub const MEMBER_VALUE: u32 = 0x0044_e9e0;
/// The element constructors of three arrays (thiscall, no words):
/// `PShader` (`0x004bbbd0`, 32 bytes), `PMaterialSwitch` (`0x004d7b60`, 8),
/// the render list (`0x004db1e0`, 0x28).
pub const SHADER_CTOR: u32 = 0x004b_bbd0;
pub const MATERIAL_SWITCH_CTOR: u32 = 0x004d_7b60;
pub const RENDER_LIST_CTOR: u32 = 0x004d_b1e0;
/// The `PEffect` element copy constructor (`0x004e9ee0`, ported here).
pub const SHARRAY_COPY_CTOR: u32 = 0x004e_9ee0;
/// `0x004d0470` (thiscall `(n, data)`, `ret 8`): the shader parameter
/// sharray's buffer for `n` copies of `data`.
pub const SHARRAY_ALLOC: u32 = 0x004d_0470;
/// The tree-owner destructor's callees: `orphanAllContents` and `clear`
/// of the tree, the free list's `reset` and destructor (at `+0x18`).
pub const TREE_ORPHAN: u32 = 0x0057_4970;
pub const TREE_CLEAR: u32 = 0x0057_4760;
pub const FREE_LIST_RESET: u32 = 0x0049_cc10;
pub const FREE_LIST_DTOR: u32 = 0x0049_c5b0;
/// `Phyre_PFreeList_deallocate` (thiscall `(list; node)`, `ret 4`).
pub const FREE_LIST_DEALLOCATE: u32 = 0x0049_c980;
/// `Phyre_PObjectRef_release_0044d730` (thiscall).
pub const OBJECT_REF_RELEASE: u32 = 0x0044_d730;
/// `PScriptableComponent`'s base vtable (its destructor stores it).
pub const SCRIPTABLE_VTABLE: u32 = 0x00b2_d200;

pub mod bridge {
    //! The ports over game memory.

    use super::{
        CLASS_MEMBER, CLASS_TREE, Conv, ENTITY_CLASS, ENTITY_MEMBER_NAME, FREE_LIST_DEALLOCATE,
        FREE_LIST_DTOR, FREE_LIST_RESET, GET_CLASS_DESCRIPTOR, GLOBAL_NAMESPACE, Host,
        MEMBER_VALUE, Mem, OBJECT_REF_RELEASE, PHYRE_FREE, PUSH_INSTANCE, SCRIPTABLE_VTABLE,
        STRING_ASSIGN, TREE_CLEAR, TREE_FIXUP, TREE_ORPHAN, WORLD_CLASS, x87,
    };

    const EXTERNAL: u32 = 0x8000_0000;

    /// Whether `x` is on the chain from `p` linked through `+0x40`.
    fn on_chain(m: &impl Mem, mut p: u32, x: u32) -> bool {
        while p != 0 {
            if p == x {
                return true;
            }
            p = m.read(p + 0x40);
        }
        false
    }

    /// The data of a `PSharray` header at `h`: none, the header's own word
    /// for one element, else the buffer.
    fn sharray_data(m: &impl Mem, h: u32) -> u32 {
        let count: u32 = m.read(h);
        match count & !EXTERNAL {
            0 => 0,
            1 => h + 4,
            _ => m.read(h + 4),
        }
    }

    /// `0x0044e960` (thiscall on the tree head, `(node)`, `ret 4`): the
    /// one-word-key tree's attach: `node` made a red leaf, a node with
    /// its key (`+0xc`) returned, else `node` hung under the last node
    /// visited (below goes to child 0) and 0.
    pub fn tree_attach1(m: &mut impl Mem, tree: u32, node: u32) -> u32 {
        let lo: u8 = m.read(node + 8);
        let word: u32 = m.read(node + 8);
        m.write(node + 8, word.wrapping_add(1 - u32::from(lo & 1)));
        m.write(node, tree);
        m.write(node + 4, tree);
        let mut parent = tree + 0xc;
        let mut right = 0;
        let mut cur: u32 = m.read(parent);
        if cur != tree {
            let key: u32 = m.read(node + 0xc);
            while cur != tree {
                parent = cur;
                let k: u32 = m.read(cur + 0xc);
                if k == key {
                    return cur;
                }
                right = u32::from(k < key);
                cur = m.read(cur + 4 * right);
            }
        }
        let lo: u8 = m.read(node + 8);
        m.write(node + 8, u32::from(lo & 1).wrapping_add(parent));
        m.write(parent.wrapping_add(4 * right), node);
        0
    }

    /// An entity's class node rebuilt and put in the class tree (its
    /// attach is [`tree_attach1`], the fix-up the original's).
    fn insert_class_node(h: &mut impl Host, entity: u32, class: u32) {
        let tree = h.rebase(CLASS_TREE);
        let node = entity + 8;
        h.write(node, node);
        h.write(node + 4, node);
        h.write(node + 8, node);
        h.write(node + 0xc, class);
        h.write(node + 0x10, 0u32);
        h.write(entity + 0x18, entity);
        if tree_attach1(h, tree, node) == 0 {
            h.call_sym(TREE_FIXUP, Conv::Thiscall(tree), &[node]);
        }
    }

    /// `PEntity::FixInstancesAfterLoading(list)` (`0x0044f780`, cdecl):
    /// each loaded entity with a class (`+4`) gets its class node
    /// (`+8`) back in the class tree. The list: count `+0x3c`, first
    /// `+0x30`, stride `[+0x2c] + 0x1c`. 0.
    pub fn fix_instances(h: &mut impl Host, list: u32) -> u32 {
        let n: u32 = h.read(list + 0x3c);
        let mut e: u32 = h.read(list + 0x30);
        let desc: u32 = h.read(list + 0x2c);
        let stride: u32 = h.read(desc + 0x1c);
        for _ in 0..n {
            let class: u32 = h.read(e + 4);
            if class != 0 {
                insert_class_node(h, e, class);
            }
            e = e.wrapping_add(stride);
        }
        0
    }

    /// `0x0044f6d0` (cdecl `(object, class)`): the value the entity class
    /// tree maps an entity to: the object itself when its class derives
    /// from `PEntity`, else its member [`ENTITY_MEMBER_NAME`]'s value
    /// (none: 0). 0 for a null object or a key not in the tree.
    pub fn entity_lookup(h: &mut impl Host, object: u32, class: u32) -> u32 {
        if object == 0 {
            return 0;
        }
        let entity = h.rebase(ENTITY_CLASS);
        let mut key = 0;
        if on_chain(h, class, entity) {
            key = object;
        }
        if key == 0 {
            let name = h.rebase(ENTITY_MEMBER_NAME);
            let member = h.call_sym(CLASS_MEMBER, Conv::Thiscall(class), &[name]);
            if member == 0 {
                return 0;
            }
            key = h.call_sym(MEMBER_VALUE, Conv::Cdecl, &[member, object]);
            if key == 0 {
                return 0;
            }
        }
        let tree = h.rebase(CLASS_TREE);
        let node = find1_value(h, tree, key);
        // the original tests the field's address too (never 0 for a node)
        if node == 0 || node.wrapping_add(0x10) == 0 {
            return 0;
        }
        h.read(node + 0x10)
    }

    /// The one-word-key tree's find (`0x0044e8c0`) for a key in a
    /// register.
    fn find1_value(m: &impl Mem, tree: u32, key: u32) -> u32 {
        let mut node: u32 = m.read(tree + 0xc);
        while node != tree {
            let k: u32 = m.read(node + 0xc);
            if k == key {
                return node;
            }
            node = m.read(node + if k > key { 0 } else { 4 });
        }
        0
    }

    /// `Phyre_PClassDescriptor_GetInstanceMemberForType<PEntity>`
    /// (`0x0044e520`, cdecl `(&list, state, name)`): -1 unless `name` is a
    /// class derived from the world class; else every entity on the list
    /// (`*list`, linked through `+0`) whose class chain (from `+8`) holds
    /// it is pushed to `state`; their count.
    pub fn entity_instance_member(h: &mut impl Host, list: u32, state: u32, name: u32) -> u32 {
        let ns = h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
        let d = h.call_sym(GET_CLASS_DESCRIPTOR, Conv::Thiscall(ns), &[name]);
        let world = h.rebase(WORLD_CLASS);
        if d == 0 || !on_chain(h, d, world) {
            return u32::MAX;
        }
        let mut n = 0u32;
        let mut e: u32 = h.read(list);
        while e != 0 {
            let c: u32 = h.read(e + 8);
            if on_chain(h, c, d) {
                h.call_sym(PUSH_INSTANCE, Conv::Cdecl, &[state, e, world]);
                n += 1;
            }
            e = h.read(e);
        }
        n
    }

    /// `0x00555270` (thiscall `(src)`, `ret 4`): the timer component's
    /// assignment: the three words after the base's vtable (`src + 4`; a
    /// null `src` reads from 0), the name string (`PString::Assign` of the
    /// source's pointer less its owned bit, unless it is the same
    /// string), three words, two floats and a byte. `this`.
    pub fn timer_copy(h: &mut impl Host, this: u32, src: u32) -> u32 {
        let base = if src == 0 { 0 } else { src + 4 };
        for i in 0..3 {
            let w: u32 = h.read(base + 4 * i);
            h.write(this + 4 + 4 * i, w);
        }
        let from = src.wrapping_add(0x10);
        let to = this + 0x10;
        if to != from {
            let lo: u8 = h.read(from);
            let p: u32 = h.read(from);
            h.call_sym(
                STRING_ASSIGN,
                Conv::Thiscall(to),
                &[p.wrapping_sub(u32::from(lo & 1))],
            );
        }
        for off in [0x14, 0x18, 0x1c] {
            let w: u32 = h.read(src.wrapping_add(off));
            h.write(this + off, w);
        }
        for off in [0x20, 0x24] {
            let f: f32 = h.read(src.wrapping_add(off));
            h.write(this + off, x87::round_trip(f));
        }
        let b: u8 = h.read(src.wrapping_add(0x28));
        h.write(this + 0x28, b);
        this
    }

    /// `PMeshInstance_findDataBlock` (`0x00489150`, thiscall `(out,
    /// key)`, `ret 8`): the first of the mesh's 0x40-byte segments (count
    /// `+0x28`, array `+0x2c`) whose 12-byte data blocks (count `+8`,
    /// array `+0xc`) hold one with `+4 == key`: `*out` the segment, the
    /// block returned; else `*out = 0` and 0.
    pub fn find_data_block(m: &mut impl Mem, this: u32, out: u32, key: u32) -> u32 {
        m.write(out, 0u32);
        let n: u32 = m.read(this + 0x28);
        let mut seg: u32 = m.read(this + 0x2c);
        for _ in 0..n & !EXTERNAL {
            let k: u32 = m.read(seg + 8);
            let mut b: u32 = m.read(seg + 0xc);
            for _ in 0..k & !EXTERNAL {
                let w: u32 = m.read(b + 4);
                if w == key {
                    m.write(out, seg);
                    return b;
                }
                b = b.wrapping_add(0xc);
            }
            seg = seg.wrapping_add(0x40);
        }
        0
    }

    /// `0x004df1d0`, `0x004e61d0`, `0x004e6250` (cdecl `(items, n)`): each
    /// item (`stride` bytes) constructed by `ctor` (thiscall), a null slot
    /// skipped. Nothing returned (the original's `eax` is the last
    /// constructor's, or frame junk for no items).
    pub fn construct_items(h: &mut impl Host, items: u32, n: u32, stride: u32, ctor: u32) {
        for i in 0..n {
            let p = items.wrapping_add(i.wrapping_mul(stride));
            if p != 0 {
                h.call_sym(ctor, Conv::Thiscall(p), &[]);
            }
        }
    }

    /// `0x004e7d70` (cdecl `(dst, src, n)`): 8-byte items copy-constructed
    /// by `ctor` (thiscall `(item; src item)`), a null destination slot
    /// skipped without using up a source item. Nothing returned (as
    /// [`construct_items`]).
    pub fn copy_construct_items(h: &mut impl Host, dst: u32, src: u32, n: u32, ctor: u32) {
        let mut d = dst;
        let mut s = src;
        for _ in 0..n {
            if d != 0 {
                h.call_sym(ctor, Conv::Thiscall(d), &[s]);
                s = s.wrapping_add(8);
            }
            d = d.wrapping_add(8);
        }
    }

    /// `0x004e9ee0` (thiscall `(src)`, `ret 4`): a `PSharray` copy
    /// constructor (`0x004f2740` with another `alloc`): the count cleared,
    /// a buffer for the source's elements (`alloc`, thiscall on the
    /// header, with the count and data: [`SHARRAY_ALLOC`] here; `0x004f2740`
    /// frees a null old buffer too, `free_null`), an owned old buffer
    /// that is neither the new one nor the header's own word freed, the
    /// count stored, the buffer word set unless the data is the header's
    /// own. `this`.
    pub fn sharray_copy_ctor(
        h: &mut impl Host,
        this: u32,
        src: u32,
        alloc: u32,
        free_null: bool,
    ) -> u32 {
        h.write(this, 0u32);
        let n = {
            let c: u32 = h.read(src);
            c & !EXTERNAL
        };
        let data = sharray_data(h, src);
        let new = h.call_sym(alloc, Conv::Thiscall(this), &[n, data]);
        let header: u32 = h.read(this);
        let cur = sharray_data(h, this);
        let own = this + 4;
        if cur != new && cur != own && header & EXTERNAL == 0 && (free_null || cur != 0) {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[cur]);
        }
        h.write(this, n);
        if new != own {
            h.write(own, new);
        }
        this
    }

    /// `0x005506c0` (thiscall `(buffer, n)`, `ret 8`): a `PSharray<T*>`
    /// set to `n` elements at `buffer`: 5 for a negative `n`; one element
    /// is copied into the header's own word (count 1), more keep `buffer`
    /// (count marked external); the old data freed when owned and neither
    /// the new data nor the header's own word. 0.
    pub fn sharray_set(h: &mut impl Host, this: u32, buffer: u32, n: u32) -> u32 {
        if n & EXTERNAL != 0 {
            return 5;
        }
        let cur = sharray_data(h, this);
        let own = this + 4;
        let (data, header) = if n == 1 {
            let w: u32 = h.read(buffer);
            h.write(own, w);
            (own, 1)
        } else {
            (buffer, n | EXTERNAL)
        };
        if cur != data && cur != own {
            let old: u32 = h.read(this);
            if old & EXTERNAL == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[cur]);
            }
        }
        if data != own {
            h.write(own, data);
        }
        h.write(this, header);
        0
    }

    /// `0x00577e90` (thiscall): a tree with a free list at `+0x18`
    /// destroyed: the tree orphaned, the free list reset and destroyed,
    /// the tree cleared twice; the last call's `eax`.
    pub fn tree_owner_dtor(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(TREE_ORPHAN, Conv::Thiscall(this), &[]);
        h.call_sym(FREE_LIST_RESET, Conv::Thiscall(this + 0x18), &[]);
        h.call_sym(FREE_LIST_DTOR, Conv::Thiscall(this + 0x18), &[]);
        h.call_sym(TREE_CLEAR, Conv::Thiscall(this), &[]);
        h.call_sym(TREE_CLEAR, Conv::Thiscall(this), &[])
    }

    /// The block a word points at freed unless its bit 0 is set.
    fn free_unless_bit0(h: &mut impl Host, at: u32) {
        let lo: u8 = h.read(at);
        if lo & 1 == 0 {
            let p: u32 = h.read(at);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
        }
    }

    /// `0x0064b8c0` (thiscall on a free list, `(node)`, `ret 4`): a named
    /// node's two strings (`+0x10`, `+0xc`) freed unless unowned, the node
    /// back to the free list; `deallocate`'s `eax`.
    pub fn free_named_node(h: &mut impl Host, this: u32, node: u32) -> u32 {
        free_unless_bit0(h, node + 0x10);
        free_unless_bit0(h, node + 0xc);
        h.call_sym(FREE_LIST_DEALLOCATE, Conv::Thiscall(this), &[node])
    }

    /// `0x00552d30` (thiscall `(flags)`, `ret 4`): `PScriptableComponent`'s
    /// deleting destructor: its vtable, the name (`+0x10`) freed unless
    /// unowned, the object reference (`+4`) released, `PhyreFree` for
    /// `flags & 1`. `this`.
    pub fn scriptable_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(SCRIPTABLE_VTABLE);
        h.write(this, vt);
        free_unless_bit0(h, this + 0x10);
        h.call_sym(OBJECT_REF_RELEASE, Conv::Thiscall(this + 4), &[]);
        if flags & 1 != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `0x004d40c0` (thiscall, seven words, `ret 0x1c`): appends whether
    /// the LOD blend is on to the cursor `*cursor` (the fourth word): the
    /// node (second word) has LOD data (`+0x20`) and the context (third
    /// word) blends (`+0xbc`), and the level's weight (12-byte records of
    /// the data's `PSharray` at `+0x14`, index `+0xc4`) is neither 0 nor 1
    /// (a NaN counts as on). 0.
    pub fn pack_lod_blend(m: &mut impl Mem, node: u32, context: u32, cursor: u32) -> u32 {
        let mut on = false;
        let lod: u32 = m.read(node + 0x20);
        if lod != 0 {
            let blends: u32 = m.read(context + 0xbc);
            if blends != 0 {
                let levels = sharray_data(m, lod + 0x14);
                let index: u32 = m.read(context + 0xc4);
                let at = levels.wrapping_add(index.wrapping_mul(12));
                let bits: u32 = m.read(at);
                let w = f64::from(x87::round_trip(f32::from_bits(bits)));
                #[allow(clippy::float_cmp)] // the exact compares of `fucomp`
                let edge = w == 0.0 || w == 1.0;
                on = !edge;
            }
        }
        let p: u32 = m.read(cursor);
        m.write(p, u32::from(on));
        m.write(cursor, p.wrapping_add(4));
        0
    }
}
