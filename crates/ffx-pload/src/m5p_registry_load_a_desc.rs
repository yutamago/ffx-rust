//! Class descriptors, data members and `PClassDescriptorDynamic` (M5
//! stage 1, list `phyre-registry-load-a`, batch 4): small virtual methods
//! and helpers that forward to their base class, walk members, or release
//! what a descriptor owns.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// `PhyreFree` (cdecl).
pub const FREE: u32 = 0x0042_fb20;
/// `operator delete` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `PClassDescriptor::Destruct` (thiscall).
pub const DESCRIPTOR_DESTRUCT: u32 = 0x0043_b3e0;
/// `PClassDescriptorForType<PClassDescriptor>`'s vtable.
pub const DESCRIPTOR_FOR_DESCRIPTOR_VTABLE: u32 = 0x00b0_eae4;
/// `PAnnotation::~PAnnotation` (thiscall).
pub const ANNOTATION_DTOR: u32 = 0x0057_5920;
/// `PClassDescriptor::calculateDefaultBuffer` (thiscall).
pub const CALCULATE_DEFAULT: u32 = 0x0043_c420;
/// `PClassDescriptor::calculateValidationBuffer` (thiscall).
pub const CALCULATE_VALIDATION: u32 = 0x0043_c630;
/// `PCluster::deleteObject(obj)` (thiscall, one word).
pub const CLUSTER_DELETE_OBJECT: u32 = 0x0044_1240;
/// `0x0043b9d0` `cdecl(base, x)`: the base descriptor's check.
pub const BASE_CHECK: u32 = 0x0043_b9d0;
/// The script's object accessor `Get(x)` (cdecl).
pub const OBJECT_GET: u32 = 0x0043_5650;
/// `0x00436e80` `cdecl(L, object, descriptor)`: an object pushed to Lua.
pub const PUSH_OBJECT: u32 = 0x0043_6e80;
/// `lua_checkstack(L, n)` (cdecl).
pub const LUA_CHECKSTACK: u32 = 0x0094_ac00;
/// `lua_pushnil(L)` (cdecl).
pub const LUA_PUSHNIL: u32 = 0x0094_b730;
/// The class descriptor `0x0045da20` looks for in a chain.
pub const ASSET_CLASS: u32 = 0x00c9_3f98;
/// `GetFixupForPTypePointer`-style refusal of a class descriptor (flags
/// bit 0): 0x14.
pub const CLASS_REFUSED: u32 = 0x14;

/// The ports over game memory.
pub mod bridge {
    use super::{
        ANNOTATION_DTOR, ASSET_CLASS, BASE_CHECK, CALCULATE_DEFAULT, CALCULATE_VALIDATION,
        CLASS_REFUSED, CLUSTER_DELETE_OBJECT, Conv, DESCRIPTOR_DESTRUCT,
        DESCRIPTOR_FOR_DESCRIPTOR_VTABLE, FREE, Host, LUA_CHECKSTACK, LUA_PUSHNIL, Mem, OBJECT_GET,
        OPERATOR_DELETE, PUSH_OBJECT,
    };

    /// `PArray<T>`'s release (`0x0043b360`, `0x004477d0`, thiscall): the
    /// block (`+4`) freed unless the header's bit 31 marks it external or
    /// it is 0; both words zeroed. `eax`: `0xfe` for an external block, 0
    /// for none, else `PhyreFree`'s.
    pub fn array_release(h: &mut impl Host, this: u32) -> u32 {
        let head: u32 = h.read(this);
        let eax = if head & 0x8000_0000 != 0 {
            0xfe
        } else {
            let data: u32 = h.read(this + 4);
            if data == 0 {
                0
            } else {
                h.call_sym(FREE, Conv::Cdecl, &[data])
            }
        };
        h.write(this + 4, 0u32);
        h.write(this, 0u32);
        eax
    }

    /// `0x0043b3c0` (thiscall on a list node `{next, prev, .., data +0xc,
    /// flags +0x10}`): unlinked and linked to itself; its data freed when
    /// bit 0 of the flags is set. `eax`: `PhyreFree`'s, else the old
    /// `prev`.
    pub fn node_release(h: &mut impl Host, this: u32) -> u32 {
        let next: u32 = h.read(this);
        let prev: u32 = h.read(this + 4);
        h.write(next + 4, prev);
        h.write(prev, next);
        let flags: u8 = h.read(this + 0x10);
        h.write(this, this);
        h.write(this + 4, this);
        if flags & 1 != 0 {
            let data: u32 = h.read(this + 0xc);
            return h.call_sym(FREE, Conv::Cdecl, &[data]);
        }
        prev
    }

    /// A class descriptor's scalar deleting destructor (thiscall
    /// `(flags)`): `PClassDescriptorForType<PClassDescriptor>`'s vtable
    /// first when `with_vtable` (`0x0043b840`; `0x0043b810` stores none),
    /// `PClassDescriptor::Destruct`, `PhyreFree(this)` when bit 0 of the
    /// flags is set; returns this.
    pub fn delete_descriptor(h: &mut impl Host, this: u32, flags: u32, with_vtable: bool) -> u32 {
        if with_vtable {
            let vt = h.rebase(DESCRIPTOR_FOR_DESCRIPTOR_VTABLE);
            h.write(this, vt);
        }
        h.call_sym(DESCRIPTOR_DESTRUCT, Conv::Thiscall(this), &[]);
        if flags & 1 != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `PAnnotationWithValue<PUInt32>`'s scalar deleting destructor
    /// (`0x00447ab0`, thiscall `(flags)`): `PAnnotation::~PAnnotation`,
    /// `operator delete(this)` when bit 0 of the flags is set; returns
    /// this.
    pub fn delete_annotation(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        h.call_sym(ANNOTATION_DTOR, Conv::Thiscall(this), &[]);
        if flags & 1 != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `0x0043bc10` `cdecl(desc, &name, &len)`: `0x14` when the
    /// descriptor's flags bit 0 is set; else its name (`+0x18`) and the
    /// name's length with the zero, 0.
    pub fn class_name_fixup(m: &mut impl Mem, desc: u32, name_out: u32, len_out: u32) -> u32 {
        let flags: u8 = m.read(desc + 0x90);
        if flags & 1 != 0 {
            return CLASS_REFUSED;
        }
        let name: u32 = m.read(desc + 0x18);
        let mut n = 0u32;
        while m.read::<u8>(name.wrapping_add(n)) != 0 {
            n += 1;
        }
        m.write(len_out, n + 1);
        m.write(name_out, name);
        0
    }

    /// `PClassDescriptor::getValidationBuffer` (`0x0043cd80`, thiscall):
    /// computes the buffers unless flags bit 31 says they are (slot 12
    /// first when bit 30, in the namespace, is clear), returns `+0x68`.
    pub fn validation_buffer(h: &mut impl Host, desc: u32) -> u32 {
        let flags: u32 = h.read(desc + 0x90);
        if flags & 0x8000_0000 == 0 {
            if flags & 0x4000_0000 == 0 {
                h.call_virtual(desc, 12, &[]);
            }
            h.call_sym(CALCULATE_DEFAULT, Conv::Thiscall(desc), &[]);
            h.call_sym(CALCULATE_VALIDATION, Conv::Thiscall(desc), &[]);
        }
        h.read(desc + 0x68)
    }

    /// A data member's stored size (`0x0043cd30`, thiscall): 4 (a
    /// pointer) when its flags (`+0x14`) have bit 1, or when bit 6 is
    /// clear and its class descriptor (slot 1) is of kind 2 (`+0x30`);
    /// else its type's size (`[+0x18] + 0x1c`).
    pub fn member_size(h: &mut impl Host, member: u32) -> u32 {
        let ty: u32 = h.read(member + 0x18);
        let size: u32 = h.read(ty + 0x1c);
        let desc = h.call_virtual(member, 1, &[]);
        let flags: u32 = h.read(member + 0x14);
        let kind2 = desc != 0 && h.read::<u32>(desc + 0x30) == 2;
        let pointer = (flags >> 1) & 1 != 0 || ((flags >> 6) & 1 == 0 && kind2);
        if pointer { 4 } else { size }
    }

    /// `0x0043d650` (thiscall `(p, n, x)`): slot 0 (`(p_i, x)`) on each of
    /// `n` elements from `p`, stepping by `+0x1c`; `al` = 1 or-ed with
    /// each result's `al`. `eax`: the last call's upper bytes, or the
    /// stride's when `n` is 0.
    pub fn for_each_element(h: &mut impl Host, this: u32, p: u32, n: u32, x: u32) -> u32 {
        let stride: u32 = h.read(this + 0x1c);
        let mut acc = 1u32;
        let mut eax = stride;
        let mut at = p;
        for _ in 0..n {
            eax = h.call_virtual(this, 0, &[at, x]);
            acc |= eax & 0xff;
            at = at.wrapping_add(stride);
        }
        (eax & !0xff) | (acc & 0xff)
    }

    /// `0x00437440` (thiscall `(x)`): the bound object (`+0x14`) called
    /// with `(x, [+0x18])` through its slot 6 when `+0x10` has bit 1, else
    /// slot 4; its `eax`.
    pub fn forward_bound(h: &mut impl Host, this: u32, x: u32) -> u32 {
        let obj: u32 = h.read(this + 0x14);
        let y: u32 = h.read(this + 0x18);
        let flags: u8 = h.read(this + 0x10);
        let slot = if flags & 2 != 0 { 6 } else { 4 };
        h.call_virtual(obj, slot, &[x, y])
    }

    /// `PCluster::deleteObjectForScript(obj)` (`0x00441380`, thiscall):
    /// `PCluster::deleteObject(obj)` (a tail jump), or 1 for a null
    /// object.
    pub fn delete_object_for_script(h: &mut impl Host, this: u32, obj: u32) -> u32 {
        if obj == 0 {
            return 1;
        }
        h.call_sym(CLUSTER_DELETE_OBJECT, Conv::Thiscall(this), &[obj])
    }

    /// `0x0045da20` `cdecl(obj)`: when `obj`'s class (`+0x2c`) is or
    /// derives from the class at `0x00c93f98` (parents at `+0x40`), its
    /// `n` (`+0x3c`) instances from `+0x30` (stride the class's `+0x1c`)
    /// get `+0x24` = `[obj+0x28]`. `eax`: 0 when it does not, else where
    /// the walk ends.
    pub fn set_asset_owner(m: &mut impl Mem, obj: u32) -> u32 {
        let class: u32 = m.read(obj + 0x2c);
        let target = m.rebase(ASSET_CLASS);
        let mut c = class;
        loop {
            if c == 0 {
                return 0;
            }
            if c == target {
                break;
            }
            c = m.read(c + 0x40);
        }
        let n: u32 = m.read(obj + 0x3c);
        let mut p: u32 = m.read(obj + 0x30);
        let stride: u32 = m.read(class + 0x1c);
        let owner: u32 = m.read(obj + 0x28);
        for _ in 0..n {
            m.write(p + 0x24, owner);
            p = p.wrapping_add(stride);
        }
        p
    }

    /// `PClassDescriptorDynamic` `0x00449480` (thiscall `(x)`): the base
    /// descriptor's check (`0x0043b9d0(base, x)`), or 0 without a base.
    pub fn base_check(h: &mut impl Host, this: u32, x: u32) -> u32 {
        let base: u32 = h.read(this + 0x40);
        if base == 0 {
            return 0;
        }
        h.call_sym(BASE_CHECK, Conv::Cdecl, &[base, x])
    }

    /// `PClassDescriptorDynamic` slot 11 `destroyInstance(p)`
    /// (`0x004495b0`, thiscall): for each member (list at `+0x44`, nodes
    /// at member `+4`) not flagged external (`+0x14` bit 1) whose class
    /// descriptor (its type `+0x18`, slot 1) exists, that descriptor's slot
    /// 11 on the member's place (`p + [+0x24]` adjusted by the two
    /// descriptors' `+0x88`); then the base's slot 11 (`p`), whose `eax` it
    /// returns, or this with `al` = 0 without a base.
    pub fn destroy_dynamic(h: &mut impl Host, this: u32, p: u32) -> u32 {
        let head = this + 0x44;
        let mut link: u32 = h.read(head);
        while link != head && link != 0 && link != 4 {
            let member = link - 4;
            let flags: u8 = h.read(member + 0x14);
            if flags & 2 == 0 {
                let ty: u32 = h.read(member + 0x18);
                let desc = h.call_virtual(ty, 1, &[]);
                if desc != 0 {
                    let owner: u32 = h.read(member + 0xc);
                    let a: u32 = h.read(desc + 0x88);
                    let b: u32 = h.read(owner + 0x88);
                    let off: u32 = h.read(member + 0x24);
                    let at = a.wrapping_sub(b).wrapping_add(off).wrapping_add(p);
                    h.call_virtual(desc, 11, &[at]);
                }
            }
            link = h.read(member + 4);
        }
        let base: u32 = h.read(this + 0x40);
        if base == 0 {
            return this & !0xff;
        }
        h.call_virtual(base, 11, &[p])
    }

    /// `PClassDescriptorDynamic` slots forwarding to the base descriptor
    /// (`+0x40`, a tail jump through its vtable): `0x00449670` slot 14
    /// (one word), `0x004497c0` slot 13 (three words); 0 without a base.
    pub fn forward_to_base(h: &mut impl Host, this: u32, slot: u32, args: &[u32]) -> u32 {
        let base: u32 = h.read(this + 0x40);
        if base == 0 {
            return 0;
        }
        h.call_virtual(base, slot, args)
    }

    /// `PClassDescriptorDynamic` `0x00449840` (thiscall `(x, out)`): the
    /// script object `x` (`0x00435650`) as an offset from this
    /// descriptor's `+0x88`, 0 for none, into `out`; returns it.
    pub fn object_offset(h: &mut impl Host, this: u32, x: u32, out: u32) -> u32 {
        let r = h.call_sym(OBJECT_GET, Conv::Cdecl, &[x]);
        let base: u32 = h.read(this + 0x88);
        let v = if r == 0 { 0 } else { r.wrapping_sub(base) };
        h.write(out, v);
        v
    }

    /// `PClassDescriptorDynamic` `0x00449ac0` / `0x00449b50` (thiscall
    /// `(L, v)`, `v` read through the pointer for `0x00449ac0`): the
    /// object at `v + [+0x88]` pushed to Lua (`0x00436e80(L, object,
    /// this)`), or `nil` (`lua_checkstack(L, 1)`, `lua_pushnil`) when `v`
    /// or the object is 0; the push's `eax`.
    pub fn push_object(h: &mut impl Host, this: u32, l: u32, v: u32) -> u32 {
        if v != 0 {
            let base: u32 = h.read(this + 0x88);
            let obj = base.wrapping_add(v);
            if obj != 0 {
                return h.call_sym(PUSH_OBJECT, Conv::Cdecl, &[l, obj, this]);
            }
        }
        h.call_sym(LUA_CHECKSTACK, Conv::Cdecl, &[l, 1]);
        h.call_sym(LUA_PUSHNIL, Conv::Cdecl, &[l])
    }

    /// `0x00449ac0` (thiscall `(L, &v)`): [`push_object`] of the word at
    /// `pv`.
    pub fn push_object_through(h: &mut impl Host, this: u32, l: u32, pv: u32) -> u32 {
        let v: u32 = h.read(pv);
        push_object(h, this, l, v)
    }
}
