//! Small constructors and destructors, and the class descriptors'
//! create/destroy callbacks.
//!
//! The callbacks (no direct callers: they sit in the class descriptors'
//! tables) are `stdcall (object)` and return `al = 1`; the rest of `eax`
//! is what the original leaves there. Where that is the object pointer
//! (no call in between) the port returns all of `eax`; after a call into
//! original code it returns `al` alone ([`u8`]), as their dispatchers
//! read it.
//!
//! Destroy callbacks of the `delete` kind (`0x00549e00`, `0x00554690`,
//! `0x00555c30`) call the object's scalar deleting destructor (vtable slot
//! 0, `thiscall (object, 0)`) on `object - 4`; with a null object the
//! original calls through address 0's vtable, and so does the port (a
//! fault in both).

use crate::{free, thiscall, with_al};
use ffx_graphic::host::Host;

/// `0x00450450` (thiscall `(list, node)`): unlinks a node from its owner.
pub const UNLINK_OWNER: u32 = 0x0045_0450;
/// `0x0044ede0` (thiscall `(node)`).
pub const NODE_RELEASE: u32 = 0x0044_ede0;
/// `0x0044d6a0` (thiscall `(ref, descriptor)`).
pub const REF_INIT: u32 = 0x0044_d6a0;
/// `0x0044d730` (thiscall `(ref)`).
pub const REF_RELEASE: u32 = 0x0044_d730;
/// `Phyre_PRedBlackTree_remove` (thiscall `(tree, node)`).
pub const TREE_REMOVE: u32 = 0x0057_5000;
/// The red-black tree `0x0044ede0` removes from.
pub const NODE_TREE: u32 = 0x00c0_bb70;
/// `Phyre_PBase_GetClassDescriptor`'s descriptor.
pub const PBASE_DESCRIPTOR: u32 = 0x00c9_02e8;
/// `Phyre_PBase_GetClassDescriptor`.
pub const PBASE_GET_DESCRIPTOR: u32 = 0x0043_5740;
/// The descriptor `0x0045b050` initialises its reference with.
pub const NAME_DESCRIPTOR: u32 = 0x00c9_3e38;
/// The list head `0x0045b050` links its objects into (`next`, `prev`).
pub const NAME_LIST: u32 = 0x00c0_cd50;
/// The descriptor `0x00546040` stores.
pub const LOCATOR_DESCRIPTOR: u32 = 0x00c2_0868;
/// `Phyre_PGameplay_PScriptableComponent_vftable`.
pub const SCRIPTABLE_VFTABLE: u32 = 0x00b2_d200;
/// `Phyre_PAnnotation_vftable`.
pub const ANNOTATION_VFTABLE: u32 = 0x00b3_0a00;
/// `Phyre_PIClassDataMemberDynamicType_vftable`.
pub const DYNAMIC_TYPE_VFTABLE: u32 = 0x00b3_09cc;
/// `Phyre_PPhysics_PPhysicsShapeBase_vftable`.
pub const SHAPE_BASE_VFTABLE: u32 = 0x00b3_9508;
/// `Phyre_PhyreOS_Sleep` (cdecl `(ms)`).
pub const SLEEP: u32 = 0x0043_0030;
/// `0x00584ea0` (thiscall `(object)`).
pub const WAIT_RELEASE: u32 = 0x0058_4ea0;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ANNOTATION_VFTABLE, DYNAMIC_TYPE_VFTABLE, Host, LOCATOR_DESCRIPTOR, NAME_DESCRIPTOR,
        NAME_LIST, NODE_RELEASE, NODE_TREE, PBASE_DESCRIPTOR, PBASE_GET_DESCRIPTOR, REF_INIT,
        REF_RELEASE, SCRIPTABLE_VFTABLE, SHAPE_BASE_VFTABLE, SLEEP, TREE_REMOVE, UNLINK_OWNER,
        WAIT_RELEASE, free, thiscall, with_al,
    };

    /// `0x00435740` `Phyre_PBase_GetClassDescriptor()`.
    pub fn pbase_descriptor(h: &impl Host) -> u32 {
        h.rebase(PBASE_DESCRIPTOR)
    }

    /// `0x0044cdf0` / `0x0044ce00`: `*global = v`; `eax = v`.
    pub fn set_global(h: &mut impl Host, global: u32, v: u32) -> u32 {
        let g = h.rebase(global);
        h.write(g, v);
        v
    }

    /// `0x0044d6a0` (thiscall `(ref, descriptor)`): `{0, 0, descriptor}`;
    /// `this`.
    pub fn ref_init(h: &mut impl Host, this: u32, descriptor: u32) -> u32 {
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
        h.write(this + 8, descriptor);
        this
    }

    /// `0x004677d0` (thiscall `(pair)`): `{0, 0}`; `this`.
    pub fn pair_init(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
        this
    }

    /// `0x00570830` (thiscall `(this, a)`): `{a, 0, 0, 0}`; `this`.
    pub fn quad_init(h: &mut impl Host, this: u32, a: u32) -> u32 {
        h.write(this, a);
        h.write(this + 4, 0u32);
        h.write(this + 8, 0u32);
        h.write(this + 0xc, 0u32);
        this
    }

    /// `0x00575870` (thiscall `(this, a)`): `{a, 0}`; `this`.
    pub fn pair_set(h: &mut impl Host, this: u32, a: u32) -> u32 {
        h.write(this, a);
        h.write(this + 4, 0u32);
        this
    }

    /// `0x00570a60` `PEnumDescriptor::setTokens(this, tokens, count)`:
    /// `eax = count`.
    pub fn enum_set_tokens(h: &mut impl Host, this: u32, tokens: u32, count: u32) -> u32 {
        h.write(this + 8, tokens);
        h.write(this + 0xc, count);
        count
    }

    /// `0x00574970` `PRedBlackTree::orphanAllContents(tree)`: the root and
    /// both ends point at the tree itself.
    pub fn tree_orphan_all(h: &mut impl Host, tree: u32) {
        h.write(tree + 0x14, tree);
        h.write(tree + 0x10, tree);
        h.write(tree + 0xc, tree);
    }

    /// `0x00575440` `PIClassDataMemberDynamicType(this, member)`: flags
    /// the member (`+0x14 |= 4`) and points it back (`+0x28`); `this`.
    pub fn dynamic_type_init(h: &mut impl Host, this: u32, member: u32) -> u32 {
        h.write(this + 4, member);
        let vt = h.rebase(DYNAMIC_TYPE_VFTABLE);
        h.write(this, vt);
        let flags: u32 = h.read(member + 0x14);
        h.write(member + 0x14, flags | 4);
        let m: u32 = h.read(this + 4);
        h.write(m + 0x28, this);
        this
    }

    /// `0x00575840` `PAnnotation(this, a, b, c)`: an empty list at `+4`
    /// and the three words; `this`.
    pub fn annotation_init(h: &mut impl Host, this: u32, a: u32, b: u32, c: u32) -> u32 {
        let list = this + 4;
        h.write(list, list);
        h.write(list + 4, list);
        h.write(this + 0xc, a);
        h.write(this + 0x10, b);
        h.write(this + 0x14, c);
        let vt = h.rebase(ANNOTATION_VFTABLE);
        h.write(this, vt);
        this
    }

    /// `0x00575c10` `PAnnotatable::addAnnotation(list, annotation)`: links
    /// the annotation's node (`+4`) at the list's head; `eax` = the old
    /// first node.
    pub fn add_annotation(h: &mut impl Host, list: u32, annotation: u32) -> u32 {
        let node = annotation.wrapping_add(4);
        let first: u32 = h.read(list);
        h.write(node, first);
        h.write(node.wrapping_add(4), list);
        h.write(list, node);
        h.write(first.wrapping_add(4), node);
        first
    }

    /// `0x00503ad0` / `0x00549d50` / `0x0054fda0` (create): `{0, 0}`.
    pub fn create_pair(h: &mut impl Host, p: u32) -> u32 {
        if p != 0 {
            h.write(p, 0u32);
            h.write(p + 4, 0u32);
        }
        with_al(p, 1)
    }

    /// `0x00546040` (create): `{0, 0, 0x00c20868}`.
    pub fn create_locator_ref(h: &mut impl Host, p: u32) -> u32 {
        if p != 0 {
            h.write(p, 0u32);
            h.write(p + 4, 0u32);
            let d = h.rebase(LOCATOR_DESCRIPTOR);
            h.write(p + 8, d);
        }
        with_al(p, 1)
    }

    /// `0x0044ffe0` (create): `{0, 0}` and five zero words from `+8`.
    pub fn create_ref_tree(h: &mut impl Host, p: u32) -> u32 {
        if p == 0 {
            return 1;
        }
        h.write(p, 0u32);
        h.write(p + 4, 0u32);
        let t = p + 8;
        h.write(t + 0xc, 0u32);
        h.write(t + 0x10, 0u32);
        for i in 0..5 {
            h.write(t + 4 * i, 0u32);
        }
        with_al(t, 1)
    }

    /// `0x0044dff0` (create): `{0, 0, PBase's descriptor}`.
    pub fn create_pbase_ref(h: &mut impl Host, p: u32) -> u8 {
        if p != 0 {
            h.write(p, 0u32);
            h.write(p + 4, 0u32);
            let f = h.rebase(PBASE_GET_DESCRIPTOR);
            let d = h.call(f, ffx_graphic::host::Conv::Cdecl, &[]);
            h.write(p + 8, d);
        }
        1
    }

    /// `0x0044d730` (thiscall `(ref)`): unlinks the reference from its
    /// owner (`+4`) when it has one; `eax` = the owner (0) or the unlink's.
    pub fn ref_release(h: &mut impl Host, r: u32) -> u32 {
        let owner: u32 = h.read(r + 4);
        if owner == 0 {
            return 0;
        }
        thiscall(h, UNLINK_OWNER, owner, &[r])
    }

    /// `0x0044e050` (destroy): `0x00450450(owner, p)` when `p +4` is set.
    pub fn destroy_unlink(h: &mut impl Host, p: u32) -> u8 {
        let owner: u32 = h.read(p + 4);
        if owner != 0 {
            thiscall(h, UNLINK_OWNER, owner, &[p]);
        }
        1
    }

    /// `0x00450020` (destroy): `0x0044ede0(p)`.
    pub fn destroy_node(h: &mut impl Host, p: u32) -> u8 {
        thiscall(h, NODE_RELEASE, p, &[]);
        1
    }

    /// `0x00552850` (destroy): `0x0044d730(p)`.
    pub fn destroy_ref(h: &mut impl Host, p: u32) -> u8 {
        thiscall(h, REF_RELEASE, p, &[]);
        1
    }

    /// `0x00556980` (destroy): `p +0xc = 0`, then `0x0044d730(p)`.
    pub fn destroy_ref_c(h: &mut impl Host, p: u32) -> u8 {
        h.write(p + 0xc, 0u32);
        thiscall(h, REF_RELEASE, p, &[]);
        1
    }

    /// `0x0054fdc0` / `0x00549d70` (destroy): `f(p)` (thiscall) unless
    /// null.
    pub fn destroy_call(h: &mut impl Host, f: u32, p: u32) -> u8 {
        if p != 0 {
            thiscall(h, f, p, &[]);
        }
        1
    }

    /// `0x00546070` (destroy): frees `p +4` unless its low bit is set
    /// (not owned).
    pub fn destroy_owned_ptr(h: &mut impl Host, p: u32) -> u8 {
        let b: u8 = h.read(p + 4);
        if b & 1 == 0 {
            let q: u32 = h.read(p + 4);
            free(h, q);
        }
        1
    }

    /// `0x00549dc0` (destroy): an array `{count | 0x80000000 when not
    /// owned, data}`: frees owned non-null data, clears both words.
    pub fn destroy_array(h: &mut impl Host, p: u32) -> u8 {
        let count: u32 = h.read(p);
        if count >> 31 == 0 {
            let data: u32 = h.read(p + 4);
            if data != 0 {
                free(h, data);
            }
        }
        h.write(p + 4, 0u32);
        h.write(p, 0u32);
        1
    }

    /// `0x0054fdf0` (`at` 0) / `0x0054fe30` (`at` 4) (destroy): an array
    /// `{count | 0x80000000 when not owned, data}` at `p + at` whose data
    /// is freed when owned and longer than one; clears both words.
    pub fn destroy_array_gt1(h: &mut impl Host, p: u32, at: u32) -> u8 {
        let a = p + at;
        let count: u32 = h.read(a);
        if count >> 31 == 0 && count & 0x7fff_ffff > 1 {
            let data: u32 = h.read(a + 4);
            free(h, data);
        }
        h.write(a + 4, 0u32);
        h.write(a, 0u32);
        1
    }

    /// `0x00549e00` / `0x00554690` / `0x00555c30` (destroy): the object
    /// at `p - 4`'s scalar deleting destructor (slot 0) with flags 0.
    pub fn destroy_virtual(h: &mut impl Host, p: u32) -> u8 {
        let obj = if p == 0 { 0 } else { p.wrapping_sub(4) };
        h.call_virtual(obj, 0, &[0]);
        1
    }

    /// `0x00547d20` / `0x00553d20` scalar deleting destructors (thiscall
    /// `(this, flags)`): `dtor(this)`, then frees `this` when `flags & 1`;
    /// `this`.
    pub fn scalar_delete(h: &mut impl Host, dtor: u32, this: u32, flags: u32) -> u32 {
        thiscall(h, dtor, this, &[]);
        if flags & 1 != 0 {
            free(h, this);
        }
        this
    }

    /// `0x00552c00` `PScriptableComponent(this, descriptor)`: its
    /// reference at `+4`, the vtable, four zero words from `+0x10`; `this`.
    pub fn scriptable_init(h: &mut impl Host, this: u32, descriptor: u32) -> u32 {
        thiscall(h, REF_INIT, this + 4, &[descriptor]);
        let vt = h.rebase(SCRIPTABLE_VFTABLE);
        h.write(this, vt);
        for off in [0x10, 0x14, 0x18, 0x1c] {
            h.write(this + off, 0u32);
        }
        this
    }

    /// `0x0045b050` (create): the reference (`0x0044d6a0` with
    /// `0x00c93e38`), then the node at `+0xc` linked first in the list at
    /// `0x00c0cd50`, `+0x14 = 0`.
    pub fn create_listed_ref(h: &mut impl Host, p: u32) -> u8 {
        if p != 0 {
            let d = h.rebase(NAME_DESCRIPTOR);
            thiscall(h, REF_INIT, p, &[d]);
            let node = p + 0xc;
            h.write(node, node);
            h.write(node + 4, node);
            h.write(p + 0x14, 0u32);
            let head = h.rebase(NAME_LIST);
            let first: u32 = h.read(head);
            h.write(node, first);
            h.write(node + 4, head);
            h.write(head, node);
            h.write(first.wrapping_add(4), node);
        }
        1
    }

    /// `0x0045b0d0` (destroy): unlinks the node at `+0xc`, frees `+0x14`
    /// unless its low bit is set, releases the reference.
    pub fn destroy_listed_ref(h: &mut impl Host, p: u32) -> u8 {
        let node = p + 0xc;
        let next: u32 = h.read(node);
        let prev: u32 = h.read(p + 0x10);
        h.write(next.wrapping_add(4), prev);
        h.write(prev, next);
        h.write(node, node);
        h.write(node + 4, node);
        let b: u8 = h.read(p + 0x14);
        if b & 1 == 0 {
            let q: u32 = h.read(p + 0x14);
            free(h, q);
        }
        thiscall(h, REF_RELEASE, p, &[]);
        1
    }

    /// `0x0044ede0` (thiscall `(node)`): detaches every child (a list
    /// from `+0` whose entries point back at `+4`), then removes the node
    /// (`+8`) from the tree at `0x00c0bb70` when it has a parent (`+4`),
    /// else resets it to an empty tree node. `eax` = the removal's, or
    /// `node + 8`.
    pub fn node_release(h: &mut impl Host, n: u32) -> u32 {
        let mut child: u32 = h.read(n);
        // the original searches the list from `n` for the first child,
        // which it finds at once, and pops it; a first child whose parent
        // is not `n` makes it spin, as here
        while child != 0 {
            if h.read::<u32>(child + 4) == n {
                let next: u32 = h.read(child);
                h.write(n, next);
                h.write(child, 0u32);
                h.write(child + 4, 0u32);
            }
            child = h.read(n);
        }
        let t = n + 8;
        if h.read::<u32>(n + 4) != 0 {
            let tree = h.rebase(NODE_TREE);
            return thiscall(h, TREE_REMOVE, tree, &[t]);
        }
        if t != 0 {
            h.write(t, t);
            h.write(t + 4, t);
            h.write(t + 8, t);
            h.write(t + 0xc, 0u32);
            h.write(t + 0x10, 0u32);
        }
        t
    }

    /// `0x00503af0` (destroy): waits until the object's count (`+4`)
    /// drops to 0 or below (`Phyre_PhyreOS_Sleep(1)` per round), then
    /// `0x00584ea0(p)`.
    pub fn destroy_when_idle(h: &mut impl Host, p: u32) -> u8 {
        while h.read::<i32>(p + 4) > 0 {
            crate::cdecl(h, SLEEP, &[1]);
        }
        thiscall(h, WAIT_RELEASE, p, &[]);
        1
    }

    /// `0x005a6dd0` / `0x005a6ec0` / `0x005d5060` (destroy): `f(p)`
    /// (thiscall), null or not.
    pub fn destroy_call_always(h: &mut impl Host, f: u32, p: u32) -> u8 {
        thiscall(h, f, p, &[]);
        1
    }

    /// The physics shapes' scalar deleting destructors (thiscall `(this,
    /// flags)`, `0x005e6740` and its copies): the vtable `vt` (the
    /// destructor chain's last, `PPhysicsShapeBase`'s or another), then
    /// frees `this` when `flags & 1`; `this`.
    pub fn base_delete(h: &mut impl Host, this: u32, flags: u32, vt: u32) -> u32 {
        let v = h.rebase(vt);
        h.write(this, v);
        if flags & 1 != 0 {
            free(h, this);
        }
        this
    }

    /// The `Bullet` shapes' scalar deleting destructors (`0x005e66b0` and
    /// its copies): its own vtable, the Bullet object at `+off` deleted
    /// (its slot 0, flags 1) and cleared, `+0x58` cleared, then the
    /// `PPhysicsShapeBase` vtable and the free when `flags & 1`; `this`.
    pub fn bullet_delete(h: &mut impl Host, this: u32, flags: u32, vt: u32, off: u32) -> u32 {
        let v = h.rebase(vt);
        h.write(this, v);
        let obj: u32 = h.read(this + off);
        if obj != 0 {
            h.call_virtual(obj, 0, &[1]);
        }
        h.write(this + off, 0u32);
        h.write(this + 0x58, 0u32);
        let base = h.rebase(SHAPE_BASE_VFTABLE);
        h.write(this, base);
        if flags & 1 != 0 {
            free(h, this);
        }
        this
    }
}
