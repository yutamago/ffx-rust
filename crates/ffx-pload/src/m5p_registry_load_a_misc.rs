//! Small type-registry and utility functions (M5 stage 1, list
//! `phyre-registry-load-a`, batch 3): the class-descriptor destructors
//! that tail-call their parent's, float field setters, the utility lists'
//! unlink helpers, the namespace and serialization shutdown steps.

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

/// `PClassDescriptor::Destruct` (thiscall, no words).
pub const DESCRIPTOR_DESTRUCT: u32 = 0x0043_b3e0;
/// `PClassDataMember::~PClassDataMember` (thiscall, no words).
pub const MEMBER_DTOR: u32 = 0x0057_5470;

/// A destructor that stores its class's vtable at `this`, then tail-jumps
/// to the parent's destructor (`mov [ecx], vtable; jmp parent`).
#[derive(Debug, Clone, Copy)]
pub struct TailDtor {
    /// The function.
    pub addr: u32,
    /// The vtable it stores (an `ffx.sym` address).
    pub vtable: u32,
    /// The parent destructor it jumps to.
    pub parent: u32,
    /// The class (the vtable's name).
    pub class: &'static str,
}

/// The `PClassDescriptorForType<T>` destructors (and one
/// `PClassDataMemberArray`'s).
pub const TAIL_DTORS: [TailDtor; 15] = [
    TailDtor {
        addr: 0x0043_a100,
        vtable: 0x00b0_e8c4,
        parent: DESCRIPTOR_DESTRUCT,
        class: "PClassDescriptorForType<PClassMember>",
    },
    TailDtor {
        addr: 0x0043_ddb0,
        vtable: 0x00b0_ee5c,
        parent: DESCRIPTOR_DESTRUCT,
        class: "PClassDescriptorForType<PNamespace>",
    },
    TailDtor {
        addr: 0x0043_f9e0,
        vtable: 0x00b0_f024,
        parent: DESCRIPTOR_DESTRUCT,
        class: "PClassDescriptorForType<PCluster>",
    },
    TailDtor {
        addr: 0x0044_4330,
        vtable: 0x00b0_f4bc,
        parent: DESCRIPTOR_DESTRUCT,
        class: "PClassDescriptorForType<PTypedObject>",
    },
    TailDtor {
        addr: 0x0057_5f70,
        vtable: 0x00b3_0a60,
        parent: MEMBER_DTOR,
        class: "PClassDataMemberArray",
    },
    TailDtor {
        addr: 0x0044_62d0,
        vtable: 0x00b0_f82c,
        parent: DESCRIPTOR_DESTRUCT,
        class: "PClassDescriptorForType<PClassDataMemberDynamic>",
    },
    TailDtor {
        addr: 0x0044_7850,
        vtable: 0x00b0_fc64,
        parent: DESCRIPTOR_DESTRUCT,
        class: "PClassDescriptorForType<PArray<unsigned char>>",
    },
    TailDtor {
        addr: 0x0044_7860,
        vtable: 0x00b0_f9b4,
        parent: DESCRIPTOR_DESTRUCT,
        class: "PClassDescriptorForType<PClassDescriptorDynamic>",
    },
    TailDtor {
        addr: 0x0045_7b20,
        vtable: 0x00b1_1574,
        parent: DESCRIPTOR_DESTRUCT,
        class: "PClassDescriptorForType<PArray<PTypedObject>>",
    },
    TailDtor {
        addr: 0x0045_7b30,
        vtable: 0x00b1_1344,
        parent: DESCRIPTOR_DESTRUCT,
        class: "PClassDescriptorForType<PInstancesComponent>",
    },
    TailDtor {
        addr: 0x0045_c080,
        vtable: 0x00b1_1994,
        parent: DESCRIPTOR_DESTRUCT,
        class: "PClassDescriptorForType<PAssetReference>",
    },
    TailDtor {
        addr: 0x0045_c090,
        vtable: 0x00b1_1a7c,
        parent: DESCRIPTOR_DESTRUCT,
        class: "PClassDescriptorForType<PAssetReferenceImport>",
    },
    TailDtor {
        addr: 0x0046_8730,
        vtable: 0x00b1_2bfc,
        parent: DESCRIPTOR_DESTRUCT,
        class: "PClassDescriptorForType<PClusterHeader>",
    },
    TailDtor {
        addr: 0x0046_8740,
        vtable: 0x00b1_2b14,
        parent: DESCRIPTOR_DESTRUCT,
        class: "PClassDescriptorForType<PClusterHeaderBase>",
    },
    TailDtor {
        addr: 0x0046_99d0,
        vtable: 0x00b1_302c,
        parent: DESCRIPTOR_DESTRUCT,
        class: "PClassDescriptorForType<PClusterHeaderD3D11>",
    },
];

/// The float field setters (`thiscall(f) ret 4`, `fld`/`fstp` into
/// `this + offset`, `eax` = this): address and offset.
pub const FLOAT_SETTERS: [(u32, u32); 12] = [
    (0x0046_2610, 0xc),
    (0x0046_2620, 0),
    (0x0046_2630, 0),
    (0x0046_2640, 0),
    (0x0046_2650, 0),
    (0x0046_2660, 4),
    (0x0046_2670, 4),
    (0x0046_2680, 4),
    (0x0046_2690, 4),
    (0x0046_26a0, 8),
    (0x0046_26b0, 8),
    (0x0046_26c0, 8),
];

/// The class-member iterators' next (`thiscall`), copies of
/// `0x00465080` (`ffx_pload::cluster::bridge::iter_next`) that differ
/// only in which (identical) `PClassDataMember::listsClass` copy they
/// call.
pub const ITER_NEXTS: [u32; 7] = [
    0x0046_50e0,
    0x0046_51a0,
    0x0046_5200,
    0x0046_52c0,
    0x0046_5320,
    0x0046_5440,
    0x0046_54a0,
];

/// `PCriticalSectionSimple::unlock` (thiscall, no words).
pub const CS_UNLOCK: u32 = 0x0043_1a80;
/// `PIClassDataMemberDynamicType`'s vtable.
pub const DYNAMIC_TYPE_VTABLE: u32 = 0x00b3_09cc;
/// `PUtility`'s vtable.
pub const UTILITY_VTABLE: u32 = 0x00b0_e078;
/// The registered `PUtility` objects (a list linked through `+4`).
pub const UTILITIES: u32 = 0x00c9_01a8;
/// `operator delete` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `PRedBlackTree::clear` (thiscall).
pub const TREE_CLEAR: u32 = 0x0057_4760;
/// The conversion-kernel tree (`0x0047b630`'s).
pub const KERNEL_TREE: u32 = 0x00c0_e2f0;
/// Whether the kernels are registered (byte).
pub const KERNELS_REGISTERED: u32 = 0x00c9_75ec;
/// The serialization's owned objects (a list linked through `+0`).
pub const SERIALIZATION_OBJECTS: u32 = 0x00c9_75e4;
/// Their destructor (thiscall).
pub const SERIALIZATION_OBJECT_DTOR: u32 = 0x0043_1bf0;
/// `PhyreFree` (cdecl).
pub const FREE: u32 = 0x0042_fb20;
/// The serialization's "initialised" byte.
pub const SERIALIZATION_READY: u32 = 0x00c9_0280;
/// The geometry utilities' list (linked through `+4`).
pub const GEOMETRY_UTILITIES: u32 = 0x00c9_85b0;
/// The rendering utilities' list (linked through `+4`).
pub const RENDERING_UTILITIES: u32 = 0x00ca_2f04;
/// The rendering utilities' slots by index (`+8` of each).
pub const RENDERING_SLOTS: u32 = 0x00ca_2f08;
/// `PNamespace::remove(desc)` (thiscall, one word).
pub const NAMESPACE_REMOVE: u32 = 0x0043_ec40;
/// `PNamespace::GetGlobalNamespace()` (cdecl).
pub const GLOBAL_NAMESPACE: u32 = 0x0043_e2f0;
/// The thread pool's critical section (byte 0: held).
pub const POOL_LOCK: u32 = 0x00c9_40c4;
/// `PCriticalSectionBase::release` (thiscall).
pub const CS_RELEASE: u32 = 0x0043_1950;
/// `PThreadPool::SetThreadCount(n)` (cdecl).
pub const SET_THREAD_COUNT: u32 = 0x0044_ce10;

/// The ports over game memory.
pub mod bridge {
    use super::{
        CS_RELEASE, CS_UNLOCK, Conv, DYNAMIC_TYPE_VTABLE, FREE, GLOBAL_NAMESPACE, Host,
        KERNEL_TREE, KERNELS_REGISTERED, Mem, NAMESPACE_REMOVE, OPERATOR_DELETE, POOL_LOCK,
        SERIALIZATION_OBJECT_DTOR, SERIALIZATION_OBJECTS, SERIALIZATION_READY, SET_THREAD_COUNT,
        TREE_CLEAR, TailDtor, UTILITIES, UTILITY_VTABLE, x87,
    };

    /// A [`TailDtor`]: its vtable at `this`, then the parent destructor
    /// (thiscall on `this`), whose `eax` it returns.
    pub fn tail_dtor(h: &mut impl Host, d: &TailDtor, this: u32) -> u32 {
        let vt = h.rebase(d.vtable);
        h.write(this, vt);
        h.call_sym(d.parent, Conv::Thiscall(this), &[])
    }

    /// A float setter: `f` through the x87 into `this + offset`; returns
    /// this.
    pub fn set_float(m: &mut impl Mem, this: u32, offset: u32, f: u32) -> u32 {
        m.write(this.wrapping_add(offset), x87::store(x87::load(f)));
        this
    }

    /// `0x0045c0b0` (thiscall): `PCriticalSectionSimple::unlock` on the
    /// lock `this` points to (a tail jump); its `eax`.
    pub fn unlock_held(h: &mut impl Host, this: u32) -> u32 {
        let cs: u32 = h.read(this);
        h.call_sym(CS_UNLOCK, Conv::Thiscall(cs), &[])
    }

    /// `PIClassDataMemberDynamicType::~PIClassDataMemberDynamicType`
    /// (`0x005754f0`, thiscall): its vtable, and the member it belongs to
    /// (`+4`) forgets it (`+0x28` = 0); returns that member.
    pub fn dynamic_type_dtor(m: &mut impl Mem, this: u32) -> u32 {
        let member: u32 = m.read(this + 4);
        let vt = m.rebase(DYNAMIC_TYPE_VTABLE);
        m.write(this, vt);
        m.write(member + 0x28, 0u32);
        member
    }

    /// `0x004320a0` (thiscall `(head)`): `this` pushed on the list at
    /// `head` (linked through `+0`); returns the old first element.
    pub fn push_front(m: &mut impl Mem, this: u32, head: u32) -> u32 {
        let first: u32 = m.read(head);
        m.write(this, first);
        m.write(head, this);
        first
    }

    /// `0x0047a210` (cdecl): the conversion kernels' tree cleared and
    /// marked unregistered; the clear's `eax`.
    pub fn clear_kernels(h: &mut impl Host) -> u32 {
        let tree = h.rebase(KERNEL_TREE);
        let r = h.call_sym(TREE_CLEAR, Conv::Thiscall(tree), &[]);
        h.write(h.rebase(KERNELS_REGISTERED), 0u8);
        r
    }

    /// `0x0047fe10` / `0x004c1410` (cdecl `(x)`): `x` unlinked from the
    /// list at `head` (each element's next at `+4`); 0.
    pub fn unlink(m: &mut impl Mem, head: u32, x: u32) -> u32 {
        let mut link = m.rebase(head);
        let mut node: u32 = m.read(link);
        while node != 0 {
            if node == x {
                let next: u32 = m.read(node + 4);
                m.write(link, next);
                m.write(node + 4, 0u32);
                return 0;
            }
            link = node + 4;
            node = m.read(link);
        }
        0
    }

    /// `0x004c1460` (cdecl `(x)`): the rendering utilities' slot `x+8`
    /// cleared, then [`unlink`] from their list.
    pub fn unlink_rendering(m: &mut impl Mem, x: u32) -> u32 {
        let i: u32 = m.read(x + 8);
        let slot = m
            .rebase(super::RENDERING_SLOTS)
            .wrapping_add(i.wrapping_mul(4));
        m.write(slot, 0u32);
        unlink(m, super::RENDERING_UTILITIES, x)
    }

    /// `PUtility`'s scalar deleting destructor (`0x00431e10`,
    /// `0x00431e70`, thiscall `(flags)`): its vtable; `this` unlinked from
    /// the registered utilities (linked through `+4`); `operator delete`
    /// when bit 0 of the flags is set; returns this.
    pub fn delete_utility(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(UTILITY_VTABLE);
        h.write(this, vt);
        let mut link = h.rebase(UTILITIES);
        loop {
            let node: u32 = h.read(link);
            if node == 0 {
                break;
            }
            if node == this {
                let next: u32 = h.read(this + 4);
                h.write(link, next);
                h.write(this + 4, 0u32);
                break;
            }
            link = node + 4;
        }
        if flags & 1 != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `0x00432150` (cdecl): every object on the serialization's list
    /// (linked through `+0`) taken off the front, destroyed (`0x00431bf0`)
    /// and freed; then [`clear_kernels`] (its original, `0x0047a210`, is
    /// in this list) and the "ready" byte cleared; 0.
    pub fn shutdown_serialization(h: &mut impl Host) -> u32 {
        let list = h.rebase(SERIALIZATION_OBJECTS);
        loop {
            let first: u32 = h.read(list);
            if first == 0 {
                break;
            }
            let next: u32 = h.read(first);
            h.write(list, next);
            h.write(first, 0u32);
            h.call_sym(SERIALIZATION_OBJECT_DTOR, Conv::Thiscall(first), &[]);
            h.call_sym(FREE, Conv::Cdecl, &[first]);
        }
        clear_kernels(h);
        h.write(h.rebase(SERIALIZATION_READY), 0u8);
        0
    }

    /// `0x0043d420` (thiscall on a class descriptor): removed from its
    /// namespace (`+0x3c`), then flags bit 30 cleared; `remove`'s `eax`.
    pub fn remove_from_namespace(h: &mut impl Host, desc: u32) -> u32 {
        let ns: u32 = h.read(desc + 0x3c);
        let r = h.call_sym(NAMESPACE_REMOVE, Conv::Thiscall(ns), &[desc]);
        let flags: u32 = h.read(desc + 0x90);
        h.write(desc + 0x90, flags & 0xbfff_ffff);
        r
    }

    /// `0x0043e610` (cdecl): every class descriptor in the global
    /// namespace (list at `+8`, descriptors linked at their `+0x2c`)
    /// removed ([`remove_from_namespace`], whose original `0x0043d420` is
    /// in this list), then `+0x10` = 0. `eax`: the last removal's, the
    /// stopping element's descriptor when it is 0, or the namespace.
    pub fn clear_global_namespace(h: &mut impl Host) -> u32 {
        let ns = h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
        let head = ns.wrapping_add(8);
        let mut eax = ns;
        loop {
            let e: u32 = h.read(head);
            if e == head || e == 0 {
                break;
            }
            eax = e.wrapping_sub(0x2c);
            if eax == 0 {
                break;
            }
            eax = remove_from_namespace(h, eax);
        }
        h.write(ns.wrapping_add(0x10), 0u32);
        eax
    }

    /// `0x0045da00` (cdecl): `0x15` when the thread pool's lock byte is
    /// clear; else `PCriticalSectionBase::release` on it, 0.
    pub fn release_pool_lock(h: &mut impl Host) -> u32 {
        let cs = h.rebase(POOL_LOCK);
        let held: u8 = h.read(cs);
        if held == 0 {
            return 0x15;
        }
        h.call_sym(CS_RELEASE, Conv::Thiscall(cs), &[]);
        0
    }

    /// `0x00431170` (a `jmp` to `PThreadPool::SetThreadCount`): its
    /// `eax`.
    pub fn set_thread_count(h: &mut impl Host, n: u32) -> u32 {
        h.call_sym(SET_THREAD_COUNT, Conv::Cdecl, &[n])
    }
}
