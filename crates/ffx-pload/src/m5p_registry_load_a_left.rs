//! The leftovers of list `phyre-registry-load-a` (M5 stage 1, round
//! "left2"): the endianness and default-copy visitors of class
//! descriptors, `PClassDescriptorDynamic`'s layout methods, name lookups,
//! cluster helpers and the lazily built `PArray`/`PSharray` descriptors'
//! `swapEndiannessStatic` copies.
//!
//! Visitors. `PClassDescriptor::swapEndiannessOfObject` (`0x0043d6a0`)
//! and the default copy (`0x0043ca50`) build a small record in their
//! frame and hand its address to a walker over the descriptor's member
//! list (`+0x44`, a member linked at member `+4`, its parent at `+0x40`):
//!
//! - the swap visitor `{0, members, object +8, flag byte +0xc}`, read by
//!   the per-member swap (`0x0043d230`) only;
//! - the copy visitor `{0, step +4, from +8, to +0xc, pointer byte
//!   +0x10}`: the per-member copy (`0x0043d140`) writes the byte, the
//!   walker (`0x0043abb0`) reads it for the nested visitor it builds for
//!   a member of class type, and adds `step` (never written: 0) to the
//!   nested `from` after each element.
//!
//! The walkers and per-member steps are all ported, so the copy walker
//! keeps its nested visitors as Rust values ([`CopyVisitor`]); only the
//! hooked entry points read and write them in game memory.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// `memcpy` (the CRT import thunk, cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
/// `Phyre_PClassDataMember_getStoredSize_0043cd30` (thiscall on a member).
pub const STORED_SIZE: u32 = 0x0043_cd30;
/// `Phyre_PClassDataMember_getDynamicType(base)` (thiscall on a member).
pub const DYNAMIC_TYPE: u32 = 0x0057_5690;
/// `Phyre_PClassDataMember_forEachElement_0043d650(p, n, flag)` (thiscall
/// on a type, `ret 0xc`): the per-element swap.
pub const FOR_EACH_ELEMENT: u32 = 0x0043_d650;
/// The swap visitor's walkers: flag set (`0x0043ab60`, forwards through
/// the member links at `+4`), flag clear (`0x0043ad10`, from the tail
/// back through `+8`).
pub const SWAP_WALKERS: [u32; 2] = [0x0043_ab60, 0x0043_ad10];
/// The per-member swap (`0x0043d230`, thiscall on the visitor).
pub const SWAP_MEMBER: u32 = 0x0043_d230;
/// The copy walker (`0x0043abb0`, thiscall on a descriptor).
pub const COPY_WALKER: u32 = 0x0043_abb0;
/// The per-member copy (`0x0043d140`, thiscall on the visitor).
pub const COPY_MEMBER: u32 = 0x0043_d140;

/// `PhyreFree` (cdecl).
pub const FREE: u32 = 0x0042_fb20;
/// `PhyreMalloc(size, align)` (cdecl).
pub const MALLOC_ALIGNED: u32 = 0x0042_fb80;
/// `PhyreMalloc(size)` (cdecl).
pub const MALLOC: u32 = 0x0042_fb40;
/// The CRT's `atexit` (cdecl).
pub const ATEXIT: u32 = 0x0094_9207;
/// `Phyre_PhyrePrintf(level, format, ...)` (cdecl).
pub const PRINTF: u32 = 0x0043_52f0;

/// The Lua calls of `0x00435940`: `lua_gettop`, `lua_checkstack`,
/// `lua_newuserdata`, `lua_getfield`, `lua_setmetatable`, `lua_settop`.
pub const LUA: [u32; 6] = [
    0x0094_b160,
    0x0094_ac00,
    0x0094_b450,
    0x0094_b030,
    0x0094_bce0,
    0x0094_bc80,
];
/// `LUA_REGISTRYINDEX`.
pub const LUA_REGISTRYINDEX: u32 = 0xfff0_b9d8;
/// `0x00435940`'s metatable name and the descriptor its userdata holds.
pub const SCRIPTED_META: [u32; 2] = [0x00b0_e23c, 0x00c9_0760];

/// `0x0043f090`'s static: the guard, the object, its vtable, its
/// `+0x18` pointer and the destructor handed to `atexit`.
pub const STATIC_LIST: [u32; 5] = [
    0x00c9_0c50,
    0x00c9_0c24,
    0x00b0_e6a0,
    0x00b0_f0b8,
    0x00af_a280,
];

/// `0x0043f3c0`/`0x00441180`'s node release: `PInstanceList::freeBuffer`
/// (on the node), `PFreeList::~PFreeList` (on node `+8`),
/// `PFreeList::deallocate(node)` (on the pool); all thiscall.
pub const NODE_RELEASE: [u32; 3] = [0x0057_64a0, 0x0049_c5b0, 0x0049_c980];
/// `PWorld::removeInstanceList(list)` (thiscall on the world).
pub const REMOVE_INSTANCE_LIST: u32 = 0x0044_3c20;
/// The deleting destructor's callees: `PClusterBase::clear`,
/// `PWorld::removeCluster(this)`, the base destructor.
pub const CLUSTER_DTOR: [u32; 3] = [0x0044_0eb0, 0x0044_3b70, 0x0057_6640];
/// `PCluster::createInternal`'s callees: the instance-list allocation
/// (thiscall on the cluster, `(size, count, desc, 0)`) and
/// `PClassDescriptor::initializeInstance(obj)` (thiscall); the message for
/// an abstract class.
pub const CREATE_INTERNAL: [u32; 3] = [0x0044_0ba0, 0x0043_ce00, 0x00b0_f248];

/// `PClassDescriptor::getWriteMask`-style buffer getter (`0x0043cd80`,
/// thiscall) and `PClassDescriptorDynamic`'s own descriptor.
pub const MASK_GETTER: [u32; 2] = [0x0043_cd80, 0x00c9_1c18];
/// `0x004498f0`'s callees: the script's object accessor `Get(L)` (cdecl).
pub const OBJECT_GET: u32 = 0x0043_5650;
/// `0x00449b90`'s callees: `IsDefaultInstance(member)` (cdecl),
/// `addMember(member, size)` (thiscall), `refreshBuffers` (thiscall).
pub const ADD_DYNAMIC: [u32; 3] = [0x0044_6870, 0x0043_c0a0, 0x0044_9e40];
/// `0x00449c80`'s `updateBaseOffsets` (thiscall).
pub const UPDATE_BASE_OFFSETS: u32 = 0x0043_d6e0;
/// `0x00449d50`'s `PClassDescriptor::unlink(member)` (thiscall).
pub const UNLINK: u32 = 0x0043_d400;
/// `0x00449250`'s callees: `findMember(name)` (thiscall),
/// `inheritsFrom(other)` (thiscall), the layout step `0x00449960`
/// (thiscall), the member's constructor (thiscall, 6 words).
pub const ADD_MEMBER_NAMED: [u32; 4] = [0x0043_5ca0, 0x0043_1420, 0x0044_9960, 0x0044_61e0];
/// `0x0045d280`'s callees: the registry lock and unlock (thiscall on
/// [`REGISTRY_LOCK`]), the registry getter (cdecl), the name hash
/// `(name, seed)` (cdecl), the hash tree's `find(&key)` (thiscall).
pub const REGISTRY_FIND: [u32; 5] = [
    0x0043_1920,
    0x0043_1a80,
    0x0045_d420,
    0x0045_d520,
    0x0045_b8f0,
];
/// The registry's lock.
pub const REGISTRY_LOCK: u32 = 0x00c9_40c4;
/// The hash seed `0x0045d280` passes.
pub const REGISTRY_SEED: u32 = 0x7b5;
/// `0x00442370`'s callees: the global namespace (cdecl), its
/// `getClassDescriptor(name)` (thiscall), `PWorld`'s instance count
/// (thiscall), the cursor's seek and next (thiscall on the cursor), the
/// script's object push `(L, object, class)` (cdecl).
pub const PUSH_INSTANCES: [u32; 6] = [
    0x0043_e2f0,
    0x0043_e950,
    0x0044_3550,
    0x0044_3930,
    0x0044_39a0,
    0x0043_6e80,
];

/// The `swapEndiannessStatic` copies of this list (cdecl `(p)`), each a
/// `m5p_tmpl_fortype` [`crate::m5p_tmpl_fortype::Lazy`] row.
pub const SWAP_STATIC: [(u32, crate::m5p_tmpl_fortype::Lazy); 6] = {
    use crate::m5p_tmpl_fortype::Lazy;
    const fn row(guard: u32, desc: u32, ctor: u32, vtable: u32, at_exit: u32, flag: u32) -> Lazy {
        Lazy {
            guard,
            desc,
            ctor,
            flags: desc + 0x90,
            vtable,
            at_exit,
            flag,
        }
    }
    [
        (
            0x0044_70b0,
            row(
                0x00c9_165c,
                0x00c9_15c8,
                0x0044_73f0,
                0x00b0_fcf4,
                0x00af_a420,
                1,
            ),
        ),
        (
            0x0044_7170,
            row(
                0x00c9_165c,
                0x00c9_15c8,
                0x0044_73f0,
                0x00b0_fcf4,
                0x00af_a420,
                0,
            ),
        ),
        (
            0x0044_a990,
            row(
                0x00c9_1abc,
                0x00c9_1a28,
                0x0044_ac10,
                0x00b1_00fc,
                0x00af_a540,
                1,
            ),
        ),
        (
            0x0044_aa40,
            row(
                0x00c9_1abc,
                0x00c9_1a28,
                0x0044_ac10,
                0x00b1_00fc,
                0x00af_a540,
                0,
            ),
        ),
        (
            0x0045_74f0,
            row(
                0x00c9_3cec,
                0x00c9_3c58,
                0x0045_77e0,
                0x00b1_1604,
                0x00af_ad60,
                1,
            ),
        ),
        (
            0x0045_75a0,
            row(
                0x00c9_3cec,
                0x00c9_3c58,
                0x0045_77e0,
                0x00b1_1604,
                0x00af_ad60,
                0,
            ),
        ),
    ]
};

/// The [`SWAP_STATIC`] row of copy `f` (a compile error when missing).
#[must_use]
pub const fn swap_static_of(f: u32) -> crate::m5p_tmpl_fortype::Lazy {
    let mut i = 0;
    while i < SWAP_STATIC.len() {
        if SWAP_STATIC[i].0 == f {
            return SWAP_STATIC[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The named-entry lists `0x00435b80`, `0x00435c10`, `0x00435d30` search
/// (a descriptor's list head offset; an entry's name at `+0xc`).
pub const NAMED_LISTS: [(u32, u32); 3] = [
    (0x0043_5b80, 0x54),
    (0x0043_5c10, 0x4c),
    (0x0043_5d30, 0x5c),
];

/// The list offset of copy `f` in [`NAMED_LISTS`].
#[must_use]
pub const fn named_list_of(f: u32) -> u32 {
    let mut i = 0;
    while i < NAMED_LISTS.len() {
        if NAMED_LISTS[i].0 == f {
            return NAMED_LISTS[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The copy visitor (`0x0043abb0`/`0x0043d140`), as Rust values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CopyVisitor {
    /// `+0`: always 0.
    pub zero: u32,
    /// `+4`: added to `from` after each element of a class member; no one
    /// writes it (0).
    pub step: u32,
    /// `+8`: the object copied from.
    pub from: u32,
    /// `+0xc`: the object copied to (0: nothing copied).
    pub to: u32,
    /// `+0x10` (a byte): the last member visited was a pointer.
    pub pointer: u8,
    /// Whether the per-member copy wrote [`CopyVisitor::pointer`].
    pub pointer_written: bool,
}

impl CopyVisitor {
    /// The visitor at `at`.
    pub fn read(m: &impl Mem, at: u32) -> Self {
        Self {
            zero: m.read(at),
            step: m.read(at.wrapping_add(4)),
            from: m.read(at.wrapping_add(8)),
            to: m.read(at.wrapping_add(12)),
            pointer: m.read(at.wrapping_add(16)),
            pointer_written: false,
        }
    }
}

/// `0x004c1480`'s data and callees: the registration list's head, the
/// slot table, the first registration, their stride and count,
/// `Phyre_PUtilityRendering_unlink_004c1410` and
/// `Phyre_PUtilityRendering_unregister_004c1460` (cdecl).
pub const UTILITY_TEARDOWN: [u32; 7] = [
    0x00ca_2f04,
    0x00ca_2f08,
    0x00ca_2fcc,
    0x28,
    33,
    0x004c_1410,
    0x004c_1460,
];

/// The ports over game memory.
pub mod bridge {
    use super::UTILITY_TEARDOWN;
    use super::{
        ADD_DYNAMIC, ADD_MEMBER_NAMED, ATEXIT, CLUSTER_DTOR, CREATE_INTERNAL, Conv, CopyVisitor,
        DYNAMIC_TYPE, FOR_EACH_ELEMENT, FREE, Host, LUA, LUA_REGISTRYINDEX, MALLOC, MALLOC_ALIGNED,
        MASK_GETTER, MEMCPY, Mem, NODE_RELEASE, OBJECT_GET, PRINTF, PUSH_INSTANCES, REGISTRY_FIND,
        REGISTRY_LOCK, REGISTRY_SEED, REMOVE_INSTANCE_LIST, SCRIPTED_META, STATIC_LIST,
        STORED_SIZE, UNLINK, UPDATE_BASE_OFFSETS,
    };

    /// `strcmp(a, b) == 0`, as the originals' inline loops compare.
    fn same_c_string(m: &impl Mem, a: u32, b: u32) -> bool {
        let mut i = 0u32;
        loop {
            let x: u8 = m.read(a.wrapping_add(i));
            if x != m.read::<u8>(b.wrapping_add(i)) {
                return false;
            }
            if x == 0 {
                return true;
            }
            i = i.wrapping_add(1);
        }
    }

    /// The member list of descriptor `d` (`+0x44`) from `first` (the head's
    /// word at `+0x44` or `+0x48`): the first member, or 0 at the head.
    fn member_at(link: u32, head: u32) -> u32 {
        if link == head || link == 0 {
            0
        } else {
            link.wrapping_sub(4)
        }
    }

    /// A member's kind: its type (`vt[1]`) and the type's `+0x30` (0 for
    /// no type).
    fn member_type(h: &mut impl Host, member: u32) -> (u32, u32) {
        let ty = h.call_virtual(member, 1, &[]);
        let kind = if ty == 0 { 0 } else { h.read(ty + 0x30) };
        (ty, kind)
    }

    /// A member's offset in its object: `+0x24` less its descriptor's
    /// (`+0xc`) base offset `+0x88`.
    fn member_offset(h: &mut impl Host, member: u32) -> u32 {
        let off: u32 = h.read(member + 0x24);
        let desc: u32 = h.read(member + 0xc);
        let base: u32 = h.read(desc + 0x88);
        off.wrapping_sub(base)
    }

    /// The per-member swap (`0x0043d230`, thiscall on the swap visitor,
    /// `(member)`, `ret 4`): skipped for flag bit 1 (external), or a
    /// kind-2 member unless its type is set and bit 6 asks for it; the
    /// count from the type (`vt[4](object)`) for kind 1 or bit 6, which
    /// must be 1 for bit 6; the elements from the type (`vt[7]`) for kind
    /// 1, else in the object at the member's offset; then the dynamic
    /// type's per-element swap `(elements, count, flag)`. No caller reads
    /// `eax`.
    pub fn swap_member(h: &mut impl Host, object: u32, flag: u8, member: u32) {
        let flags: u32 = h.read(member + 0x14);
        let (ty, kind) = member_type(h, member);
        let external = flags & 2 != 0;
        let by_count = ty != 0 && flags & 0x40 != 0;
        if external || (kind == 2 && !by_count) {
            return;
        }
        let single = kind == 1;
        let count = if single || by_count {
            h.call_virtual(ty, 4, &[object])
        } else {
            1
        };
        if by_count && count != 1 {
            return;
        }
        let elements = if single {
            h.call_virtual(ty, 7, &[object])
        } else {
            member_offset(h, member).wrapping_add(object)
        };
        let dynamic = h.call_sym(DYNAMIC_TYPE, Conv::Thiscall(member), &[object]);
        h.call_sym(
            FOR_EACH_ELEMENT,
            Conv::Thiscall(dynamic),
            &[elements, count, u32::from(flag)],
        );
    }

    /// [`swap_member`] with the visitor at `visitor` (`object` at `+8`,
    /// the flag byte at `+0xc`).
    pub fn swap_member_at(h: &mut impl Host, visitor: u32, member: u32) {
        let object: u32 = h.read(visitor + 8);
        let flag: u8 = h.read(visitor + 0xc);
        swap_member(h, object, flag, member);
    }

    /// The swap walkers (`0x0043ab60` with `backwards` false: from the
    /// head `+0x44` through the links at member `+4`; `0x0043ad10` with it
    /// true: from `+0x48` through `+8`), thiscall on a descriptor with the
    /// visitor, `ret 4`: [`swap_member`] on each member of the descriptor
    /// and each parent (`+0x40`). `eax`: the last link read, else `eax_in`
    /// (the caller's: the visitor's address from `0x0043d6a0`).
    pub fn swap_walk(
        h: &mut impl Host,
        desc: u32,
        visitor: u32,
        backwards: bool,
        eax_in: u32,
    ) -> u32 {
        let (first, link) = if backwards { (0x48, 8) } else { (0x44, 4) };
        let mut eax = eax_in;
        let mut d = desc;
        while d != 0 {
            let head = d + 0x44;
            let mut m = member_at(h.read(d + first), head);
            while m != 0 {
                h.call_sym(super::SWAP_MEMBER, Conv::Thiscall(visitor), &[m]);
                let next: u32 = h.read(m + link);
                eax = next;
                m = member_at(next, head);
            }
            d = h.read(d + 0x40);
        }
        eax
    }

    /// `PClassDescriptor::swapEndiannessOfObject(object, flag)`
    /// (`0x0043d6a0`, thiscall on the descriptor, `ret 8`): the visitor
    /// `{0, [desc + 0x1c], object, flag byte}` in `visitor` (a frame of
    /// the port's; the original's frame, whose 3 bytes after the flag stay
    /// as they were), walked by [`swap_walk`] forwards for a flag, else
    /// backwards, with the visitor's address as the `eax` it starts from
    /// (the original's `lea eax, [ebp-0x10]`); the walker's `eax`.
    pub fn swap_object(h: &mut impl Host, desc: u32, object: u32, flag: u32, visitor: u32) -> u32 {
        let members: u32 = h.read(desc + 0x1c);
        h.write(visitor, 0u32);
        h.write(visitor + 4, members);
        h.write(visitor + 8, object);
        #[allow(clippy::cast_possible_truncation)] // the flag is a byte
        h.write(visitor + 12, flag as u8);
        let forwards = flag & 0xff != 0;
        swap_walk(h, desc, visitor, !forwards, visitor)
    }

    /// The per-member copy (`0x0043d140`, thiscall on the copy visitor,
    /// `(member)`, `ret 4`); nothing without a source. Sets the visitor's
    /// pointer byte (kind 3 or 4). Kind 2 and pointers copy through the
    /// type: `vt[8](to, vt[7](from), vt[4](from))`. Others: skipped when
    /// the member's default holder (`+0x18`, `vt[1]`) has one and flag bit
    /// 1 is clear; else `memcpy(to + off, from + off, stored size x
    /// count)`, the count `vt[4](from)` for kind 1. No caller reads `eax`.
    pub fn copy_member(h: &mut impl Host, v: &mut CopyVisitor, member: u32) {
        if v.to == 0 {
            return;
        }
        let flags: u32 = h.read(member + 0x14);
        let (ty, kind) = member_type(h, member);
        let pointer = kind == 3 || kind == 4;
        v.pointer = u8::from(pointer);
        v.pointer_written = true;
        if kind == 2 || pointer {
            let n = h.call_virtual(ty, 4, &[v.from]);
            let r = h.call_virtual(ty, 7, &[v.from]);
            h.call_virtual(ty, 8, &[v.to, r, n]);
            return;
        }
        let holder: u32 = h.read(member + 0x18);
        let has = h.call_virtual(holder, 1, &[]);
        if has != 0 && flags & 2 == 0 {
            return;
        }
        let count = if kind == 1 {
            h.call_virtual(ty, 4, &[v.from])
        } else {
            1
        };
        let off = member_offset(h, member);
        let size = h.call_sym(STORED_SIZE, Conv::Thiscall(member), &[]);
        h.call_sym(
            MEMCPY,
            Conv::Cdecl,
            &[
                v.to.wrapping_add(off),
                v.from.wrapping_add(off),
                size.wrapping_mul(count),
            ],
        );
    }

    /// [`copy_member`] on the visitor in memory at `visitor`.
    pub fn copy_member_at(h: &mut impl Host, visitor: u32, member: u32) {
        let mut v = CopyVisitor::read(h, visitor);
        copy_member(h, &mut v, member);
        if v.pointer_written {
            h.write(visitor + 0x10, v.pointer);
        }
    }

    /// The copy walker (`0x0043abb0`, thiscall on a descriptor with the
    /// visitor, `ret 4`; `eax` 0): for each member of the descriptor and
    /// its parents, [`copy_member`]; then, for a member whose dynamic type
    /// has a class (`vt[1]`) and flag bit 1 clear (kind 2 only with bit 6
    /// and a count `vt[4]` of 1), the class walked again `count` times
    /// (`vt[4](from)` for kind 1, else 1) with a nested visitor at the
    /// element (`from` the member's place in the class, `to` the same
    /// place in the target unless the member was a pointer: 0).
    pub fn copy_walk(h: &mut impl Host, desc: u32, v: &mut CopyVisitor) {
        let mut d = desc;
        while d != 0 {
            let head = d + 0x44;
            let mut m = member_at(h.read(head), head);
            while m != 0 {
                copy_member(h, v, m);
                let flags: u32 = h.read(m + 0x14);
                let (ty, kind) = member_type(h, m);
                let base = v.from;
                let dynamic = h.call_sym(DYNAMIC_TYPE, Conv::Thiscall(m), &[base]);
                let class = h.call_virtual(dynamic, 1, &[]);
                if class != 0 && flags & 2 == 0 {
                    let wanted =
                        kind != 2 || (flags & 0x40 != 0 && h.call_virtual(ty, 4, &[base]) == 1);
                    if wanted {
                        let count = if kind == 1 {
                            h.call_virtual(ty, 4, &[base])
                        } else {
                            1
                        };
                        let class_base: u32 = h.read(class + 0x88);
                        let from = member_offset(h, m)
                            .wrapping_add(class_base)
                            .wrapping_add(base);
                        let to = if v.pointer != 0 {
                            0
                        } else {
                            v.to.wrapping_sub(v.from).wrapping_add(from)
                        };
                        let mut nested = CopyVisitor {
                            from,
                            to,
                            ..CopyVisitor::default()
                        };
                        for _ in 0..count {
                            copy_walk(h, class, &mut nested);
                            nested.from = nested.from.wrapping_add(nested.step);
                        }
                    }
                }
                m = member_at(h.read(m + 4), head);
            }
            d = h.read(d + 0x40);
        }
    }

    /// [`copy_walk`] on the visitor in memory at `visitor`.
    pub fn copy_walk_at(h: &mut impl Host, desc: u32, visitor: u32) -> u32 {
        if desc != 0 {
            let mut v = CopyVisitor::read(h, visitor);
            copy_walk(h, desc, &mut v);
            if v.pointer_written {
                h.write(visitor + 0x10, v.pointer);
            }
        }
        0
    }

    /// `0x0043ca50` (thiscall on a descriptor, `(a, b)`, `ret 8`): the
    /// copy walker with the visitor `{0, 0, b, a, 0}`; the walker copies
    /// from its `+8` to its `+0xc`, so this copies `b` into `a`. `eax` 0.
    pub fn copy_object(h: &mut impl Host, desc: u32, a: u32, b: u32) -> u32 {
        if desc != 0 {
            let mut v = CopyVisitor {
                from: b,
                to: a,
                ..CopyVisitor::default()
            };
            copy_walk(h, desc, &mut v);
        }
        0
    }

    /// `0x00435940` (cdecl `(L, top, object)`, a script function): pushes a
    /// userdata `{PClassDescriptor's descriptor, object + 0x18 (0 for 0)}`
    /// with the metatable registered under `0x00b0e23c`; 9 when the stack
    /// did not grow (`lua_gettop` before and after), else `lua_settop(L,
    /// top)` and 0.
    pub fn push_scripted(h: &mut impl Host, l: u32, top: u32, object: u32) -> u32 {
        let before = h.call_sym(LUA[0], Conv::Cdecl, &[l]);
        let inner = if object == 0 { 0 } else { object + 0x18 };
        h.call_sym(LUA[1], Conv::Cdecl, &[l, 1]);
        let ud = h.call_sym(LUA[2], Conv::Cdecl, &[l, 8]);
        if ud != 0 {
            let d = h.rebase(SCRIPTED_META[1]);
            h.write(ud, d);
            h.write(ud + 4, inner);
        }
        let name = h.rebase(SCRIPTED_META[0]);
        h.call_sym(LUA[3], Conv::Cdecl, &[l, LUA_REGISTRYINDEX, name]);
        h.call_sym(LUA[4], Conv::Cdecl, &[l, 0xffff_fffe]);
        let after = h.call_sym(LUA[0], Conv::Cdecl, &[l]);
        if before == after {
            return 9;
        }
        h.call_sym(LUA[5], Conv::Cdecl, &[l, top]);
        0
    }

    /// The named-entry lookups (`0x00435b80` +0x54, `0x00435c10` +0x4c,
    /// `0x00435d30` +0x5c; thiscall on a descriptor, `(name)`, `ret 4`):
    /// the entry of the list at `off` (linked through `+0`, name `+0xc`)
    /// of the descriptor or a parent (`+0x40`) whose name is `name`; 0 for
    /// none, no name or no descriptor.
    pub fn find_named(h: &mut impl Host, off: u32, desc: u32, name: u32) -> u32 {
        if name == 0 {
            return 0;
        }
        let mut d = desc;
        while d != 0 {
            let head = d + off;
            let first: u32 = h.read(head);
            let mut e = if first == head { 0 } else { first };
            while e != 0 {
                let s: u32 = h.read(e + 0xc);
                if s != 0 && same_c_string(h, s, name) {
                    return e;
                }
                let next: u32 = h.read(e);
                e = if next == head { 0 } else { next };
            }
            d = h.read(d + 0x40);
        }
        0
    }

    /// `0x005709f0` (thiscall on a `{.., pairs +8, count +0xc}` table,
    /// `(name, out)`, `ret 8`): the first pair `{name, value}` whose name
    /// is `name`: `*out = value`, `al = 1`; else `al = 0` (no name, none).
    pub fn find_pair(h: &mut impl Host, this: u32, name: u32, out: u32) -> u8 {
        if name == 0 {
            return 0;
        }
        let mut n: u32 = h.read(this + 0xc);
        let mut p: u32 = h.read(this + 8);
        while n != 0 {
            let s: u32 = h.read(p);
            n -= 1;
            if same_c_string(h, name, s) {
                let v: u32 = h.read(p + 4);
                h.write(out, v);
                return 1;
            }
            p = p.wrapping_add(8);
        }
        0
    }

    /// `0x0043ea30` (thiscall on a namespace, `(name)`, `ret 4`; a copy of
    /// `PNamespace::getClassDescriptor` that recurses into itself): the
    /// descriptor of the list at `+8` (link `+0x2c`, name `+0x18`) named
    /// `name`, else the first match of a child namespace (`+0x14`, linked
    /// through `+0`); 0 for no name or none.
    pub fn namespace_find(h: &mut impl Host, ns: u32, name: u32) -> u32 {
        if name == 0 {
            return 0;
        }
        let head = ns + 8;
        let mut link: u32 = h.read(head);
        while link != head && link != 0 && link != 0x2c {
            let desc = link - 0x2c;
            let n: u32 = h.read(desc + 0x18);
            if n != 0 && same_c_string(h, n, name) {
                return desc;
            }
            link = h.read(desc + 0x2c);
        }
        let head = ns + 0x14;
        let first: u32 = h.read(head);
        let mut child = if first == head { 0 } else { first };
        while child != 0 {
            let r = h.call_sym(0x0043_ea30, Conv::Thiscall(child), &[name]);
            if r != 0 {
                return r;
            }
            let next: u32 = h.read(child);
            child = if next == head { 0 } else { next };
        }
        0
    }

    /// `0x0043f090` (cdecl): a function-local static object built on
    /// first use (guard bit 0): an empty list at `+4`, `+0x10`/`+0x14` 0,
    /// `+0x18` a pointer to `0x00b0f0b8`, `+0x1c`/`+0x20` 4, `+0x24`/`+0x28`
    /// 0, the vtable, its destructor registered with `atexit`; the
    /// object.
    pub fn static_list(h: &mut impl Host) -> u32 {
        let guard = h.rebase(STATIC_LIST[0]);
        let obj = h.rebase(STATIC_LIST[1]);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            let head = obj + 4;
            h.write(obj + 4, head);
            h.write(obj + 8, head);
            h.write(obj + 0xc, head);
            h.write(obj + 0x10, 0u32);
            h.write(obj + 0x14, 0u32);
            let p = h.rebase(STATIC_LIST[3]);
            h.write(obj + 0x18, p);
            h.write(obj + 0x1c, 4u32);
            h.write(obj + 0x20, 4u32);
            h.write(obj + 0x24, 0u32);
            h.write(obj + 0x28, 0u32);
            let vt = h.rebase(STATIC_LIST[2]);
            h.write(obj, vt);
            let dtor = h.rebase(STATIC_LIST[4]);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        obj
    }

    /// `0x0043f3c0` (thiscall on a free list, `(node)`, `ret 4`): the
    /// node's buffer freed, its own free list destroyed, the node given
    /// back to the pool; the pool's `eax`.
    pub fn release_node(h: &mut impl Host, pool: u32, node: u32) -> u32 {
        h.call_sym(NODE_RELEASE[0], Conv::Thiscall(node), &[]);
        h.call_sym(NODE_RELEASE[1], Conv::Thiscall(node + 8), &[]);
        h.call_sym(NODE_RELEASE[2], Conv::Thiscall(pool), &[node])
    }

    /// `PCluster`'s scalar deleting destructor (`0x0043fed0`, thiscall
    /// `(flags)`, `ret 4`): `clear`, the world's `removeCluster(this)` and
    /// `+0x40 = 0` when it has a world, the base destructor, `PhyreFree`
    /// for `flags & 1`; `this`.
    pub fn delete_cluster(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        h.call_sym(CLUSTER_DTOR[0], Conv::Thiscall(this), &[]);
        let world: u32 = h.read(this + 0x40);
        if world != 0 {
            h.call_sym(CLUSTER_DTOR[1], Conv::Thiscall(world), &[this]);
            h.write(this + 0x40, 0u32);
        }
        h.call_sym(CLUSTER_DTOR[2], Conv::Thiscall(this), &[]);
        if flags & 1 != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `PCluster::createInternal(out, desc)` (`0x00441120`, thiscall on the
    /// cluster, `ret 8`): for an abstract class (`+0x90` bit 3) a message
    /// with its name and `{0, desc}`; else a one-object instance list
    /// (`ecx` the cluster) and, when there is one, its object initialised:
    /// `{object, desc}` (0 when the allocation failed). `out`.
    pub fn create_internal(h: &mut impl Host, cluster: u32, out: u32, desc: u32) -> u32 {
        let flags: u32 = h.read(desc + 0x90);
        let mut object = 0;
        if flags & 8 != 0 {
            let name: u32 = h.read(desc + 0x18);
            let msg = h.rebase(CREATE_INTERNAL[2]);
            h.call_sym(PRINTF, Conv::Cdecl, &[1, msg, name]);
        } else {
            let size: u32 = h.read(desc + 0x1c);
            let list = h.call_sym(
                CREATE_INTERNAL[0],
                Conv::Thiscall(cluster),
                &[size, 1, desc, 0],
            );
            if list != 0 {
                object = h.read(list + 0x30);
                h.call_sym(CREATE_INTERNAL[1], Conv::Thiscall(desc), &[object]);
            }
        }
        h.write(out, object);
        h.write(out + 4, desc);
        out
    }

    /// `PCluster::deleteInstancesOfType(desc)` (`0x00441180`, thiscall,
    /// `ret 4`): every instance list of the cluster (`+0x1c`, linked
    /// through `+0`) for `desc` (`+0x2c`) unlinked, taken off the world
    /// (`+0x40`) when there is one, and released into the pool at `+0x28`.
    pub fn delete_instances_of_type(h: &mut impl Host, cluster: u32, desc: u32) {
        let head = cluster + 0x1c;
        let first: u32 = h.read(head);
        let mut next = if first == head { 0 } else { first };
        while next != 0 {
            let list = next;
            let n: u32 = h.read(list);
            next = if n == head { 0 } else { n };
            let d: u32 = h.read(list + 0x2c);
            if d != desc {
                continue;
            }
            let a: u32 = h.read(list);
            let b: u32 = h.read(list + 4);
            h.write(a + 4, b);
            h.write(b, a);
            h.write(list, list);
            h.write(list + 4, list);
            let world: u32 = h.read(cluster + 0x40);
            if world != 0 {
                h.call_sym(REMOVE_INSTANCE_LIST, Conv::Thiscall(world), &[list]);
            }
            release_node(h, cluster + 0x28, list);
        }
    }

    /// `PArray<PUInt8>::operator=(src)` (`0x004479f0`, thiscall, `ret 4`):
    /// a copy of `src`'s bytes (count `+0` without bit 31, data `+4`) in a
    /// new block (`PhyreMalloc(n, 1)`; none for 0 bytes or a failed
    /// allocation), the old block freed unless it is external (bit 31),
    /// the same or 0; `this`.
    pub fn assign_bytes(h: &mut impl Host, this: u32, src: u32) -> u32 {
        let n = h.read::<u32>(src) & 0x7fff_ffff;
        let data: u32 = h.read(src + 4);
        let mut block = 0;
        if n != 0 {
            block = h.call_sym(MALLOC_ALIGNED, Conv::Cdecl, &[n, 1]);
            if block != 0 {
                let mut buf = [0u8; 0x1000];
                let mut done = 0u32;
                while done < n {
                    let k = (n - done).min(0x1000);
                    let chunk = &mut buf[..k as usize];
                    h.read_bytes(data.wrapping_add(done), chunk);
                    h.write_bytes(block.wrapping_add(done), chunk);
                    done += k;
                }
            }
        }
        let head: u32 = h.read(this);
        let old: u32 = h.read(this + 4);
        if old != block && head & 0x8000_0000 == 0 && old != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[old]);
        }
        h.write(this + 4, block);
        h.write(this, n);
        this
    }

    /// `PClassDescriptorDynamic`'s write-mask merge (`0x004494a0`,
    /// thiscall, `(src)`, `ret 4`): without a source or a buffer (`+0x64`)
    /// nothing; else, when the mask (`0x0043cd80`) exists, each of the
    /// `+0x1c` bytes of the buffer takes `src`'s bits outside the mask;
    /// then, when `PClassDescriptorDynamic`'s descriptor is this one or a
    /// parent (`+0x40`), the buffer's word at `[+0x8c] + 8` points back to
    /// this. `eax` is no caller's.
    pub fn merge_buffer(h: &mut impl Host, this: u32, src: u32) {
        if src == 0 {
            return;
        }
        let buf: u32 = h.read(this + 0x64);
        if buf == 0 {
            return;
        }
        let n: u32 = h.read(this + 0x1c);
        let mask = h.call_sym(MASK_GETTER[0], Conv::Thiscall(this), &[]);
        if mask != 0 {
            for i in 0..n {
                let m: u8 = h.read(mask.wrapping_add(i));
                let s: u8 = h.read(src.wrapping_add(i));
                let dst: u32 = h.read(this + 0x64);
                let d: u8 = h.read(dst.wrapping_add(i));
                h.write(dst.wrapping_add(i), (s & !m) | (d & m));
            }
        }
        let target = h.rebase(MASK_GETTER[1]);
        let mut d = this;
        while d != 0 {
            if d == target {
                let table: u32 = h.read(this + 0x8c);
                let buf: u32 = h.read(this + 0x64);
                h.write(table.wrapping_add(buf).wrapping_add(8), this);
                return;
            }
            d = h.read(d + 0x40);
        }
    }

    /// `0x004498f0` (thiscall on a `PClassDescriptorDynamic`, `(L, out)`,
    /// `ret 8`): with a parent (`+0x40`), the parent reads the script
    /// value into `out`'s base part (`vt[3](L, out + [+0x84])`, 0 for no
    /// `out`), then the object's own bytes are copied from the script
    /// object (`Get(L)` less `+0x88`, 0 for none) into `out`: `+0x98` less
    /// the parent's size. `eax` is no caller's.
    pub fn read_script_value(h: &mut impl Host, this: u32, l: u32, out: u32) {
        let parent: u32 = h.read(this + 0x40);
        if parent == 0 {
            return;
        }
        let base = if out == 0 {
            0
        } else {
            h.read::<u32>(this + 0x84).wrapping_add(out)
        };
        h.call_virtual(parent, 3, &[l, base]);
        let obj = h.call_sym(OBJECT_GET, Conv::Cdecl, &[l]);
        let adjust: u32 = h.read(this + 0x88);
        if out != 0 {
            let size: u32 = h.read(this + 0x98);
            let parent_size: u32 = h.read(parent + 0x1c);
            let src = if obj == 0 {
                0
            } else {
                obj.wrapping_sub(adjust)
            };
            h.call_sym(
                MEMCPY,
                Conv::Cdecl,
                &[out, src, size.wrapping_sub(parent_size)],
            );
        }
    }

    /// Rounds `x` up to `align` (a power of two), as the originals' `(x +
    /// align - 1) & ~(align - 1)`.
    fn round_up(x: u32, align: u32) -> u32 {
        let a = align.wrapping_sub(1);
        x.wrapping_add(a) & !a
    }

    /// One member's place in a `PClassDescriptorDynamic`'s layout: aligned
    /// after `+0x98` (which then grows by the size), `+0x20` the largest
    /// alignment, `+0x1c` the rounded size; the offset from the parent's
    /// size (`+0x40`'s `+0x1c`, 0 for none) unless `parent_size` says
    /// otherwise.
    fn place(h: &mut impl Host, this: u32, size: u32, align: u32) -> u32 {
        let end: u32 = h.read(this + 0x98);
        let off = round_up(end, align);
        let end = off.wrapping_add(size);
        h.write(this + 0x98, end);
        let a: u32 = h.read(this + 0x20);
        if a < align {
            h.write(this + 0x20, align);
        }
        let a: u32 = h.read(this + 0x20);
        h.write(this + 0x1c, round_up(end, a));
        off
    }

    /// The parent's size (`[+0x40] + 0x1c`, 0 for none).
    fn parent_size(h: &mut impl Host, this: u32) -> u32 {
        let p: u32 = h.read(this + 0x40);
        if p == 0 { 0 } else { h.read(p + 0x1c) }
    }

    /// `PClassDescriptorDynamic::addMember(member)` (`0x00449b90`,
    /// thiscall, `ret 4`): 0 for the default instance; else the member
    /// placed (4 bytes for bit 1, else its type's `+0x1c`/`+0x20` after
    /// `vt[1]`), its offset `+0x24`, and `addMember(member, size)` with
    /// the stored size (4 for bit 1, or kind 2 without bit 6); on success
    /// (0) the two counts `+0x94`/`+0x96` go up and the buffers are
    /// refreshed. `addMember`'s result.
    pub fn add_member(h: &mut impl Host, this: u32, member: u32) -> u32 {
        let def = h.call_sym(ADD_DYNAMIC[0], Conv::Cdecl, &[member]);
        if def & 0xff != 0 {
            return 0;
        }
        let flags: u32 = h.read(member + 0x14);
        let ty: u32 = h.read(member + 0x18);
        let (size, align) = if flags & 2 != 0 {
            (4, 4)
        } else {
            h.call_virtual(ty, 1, &[]);
            (h.read(ty + 0x1c), h.read(ty + 0x20))
        };
        let off = place(h, this, size, align);
        let base = parent_size(h, this);
        h.write(member + 0x24, off.wrapping_sub(base));
        let ty: u32 = h.read(member + 0x18);
        let mut stored: u32 = h.read(ty + 0x1c);
        let (_, kind) = member_type(h, member);
        let flags: u32 = h.read(member + 0x14);
        if (flags & 0x40 == 0 && kind == 2) || flags & 2 != 0 {
            stored = 4;
        }
        let r = h.call_sym(ADD_DYNAMIC[1], Conv::Thiscall(this), &[member, stored]);
        if r == 0 {
            for off in [0x94, 0x96] {
                let c: u16 = h.read(this + off);
                h.write(this + off, c.wrapping_add(1));
            }
            h.call_sym(ADD_DYNAMIC[2], Conv::Thiscall(this), &[]);
        }
        r
    }

    /// `PClassDescriptorDynamic`'s reset (`0x00449c80`, thiscall): unless
    /// the two counts are equal and not 0, the six lists emptied, the
    /// counts 0, and with a parent the sizes and alignment taken from it
    /// (`+0x84` its negated size) and the base offsets updated; then the
    /// buffers from their saved copies (`+0x18` from `+0xb4` less its bit
    /// 0, `+0x64`/`+0x68`/`+0x6c` from `+0xa0`/`+0xa8`/`+0xb0`). `eax` the
    /// new `+0x64`.
    pub fn reset(h: &mut impl Host, this: u32) -> u32 {
        let a: u16 = h.read(this + 0x94);
        let b: u16 = h.read(this + 0x96);
        if a != b || a == 0 {
            for off in [0x2c, 0x44, 0x4c, 0x54, 0x5c, 0x34] {
                let l = this.wrapping_add(off);
                if l != 0 {
                    h.write(l, l);
                    h.write(l + 4, l);
                }
            }
            h.write(this + 0x94, 0u32);
            let p: u32 = h.read(this + 0x40);
            if p != 0 {
                let size: u32 = h.read(p + 0x1c);
                h.write(this + 0x98, size);
                h.write(this + 0x1c, size);
                let align: u32 = h.read(p + 0x20);
                h.write(this + 0x20, align);
                let size: u32 = h.read(p + 0x1c);
                h.write(this + 0x84, size.wrapping_neg());
                h.call_sym(UPDATE_BASE_OFFSETS, Conv::Thiscall(this), &[]);
            }
        }
        let saved: u32 = h.read(this + 0xb4);
        h.write(this + 0x18, saved.wrapping_sub(saved & 1));
        for (to, from) in [(0x6c, 0xb0), (0x68, 0xa8), (0x64, 0xa0)] {
            let v: u32 = h.read(this + from);
            h.write(this + to, v);
        }
        h.read(this + 0x64)
    }

    /// `PClassDescriptorDynamic::removeMember(member)` (`0x00449d50`,
    /// thiscall, `ret 4`): unlinked, the counts down by one, and every
    /// remaining member placed again from the parent's size. `eax` is no
    /// caller's.
    pub fn remove_member(h: &mut impl Host, this: u32, member: u32) {
        h.call_sym(UNLINK, Conv::Thiscall(this), &[member]);
        for off in [0x94, 0x96] {
            let c: u16 = h.read(this + off);
            h.write(this + off, c.wrapping_add(0xffff));
        }
        let base = parent_size(h, this);
        h.write(this + 0x98, base);
        h.write(this + 0x1c, base);
        let head = this + 0x44;
        let mut m = member_at(h.read(head), head);
        while m != 0 {
            let flags: u32 = h.read(m + 0x14);
            let ty: u32 = h.read(m + 0x18);
            let (size, align) = if flags & 2 != 0 {
                (4, 4)
            } else {
                h.call_virtual(ty, 1, &[]);
                (h.read(ty + 0x1c), h.read(ty + 0x20))
            };
            let off = place(h, this, size, align);
            let base = parent_size(h, this);
            h.write(m + 0x24, off.wrapping_sub(base));
            m = member_at(h.read(m + 4), head);
        }
    }

    /// `0x00449960` (thiscall on a `PClassDescriptorDynamic`, `(type,
    /// flags, sizer)`, `ret 0xc`): a new member's place. Size and
    /// alignment: for flag bit 1 the sizer's `vt[1]()` for both (4 without
    /// a sizer); else the type's `+0x1c`/`+0x20`, or with a sizer and a
    /// class (`type.vt[1]`) the sizer's `vt[2](class)` and `vt[3](class)`.
    /// The offset from the parent's size (the sizer's `vt[2](parent)` when
    /// there is a sizer).
    pub fn place_new(h: &mut impl Host, this: u32, ty: u32, flags: u32, sizer: u32) -> u32 {
        let (size, align) = if flags & 2 != 0 {
            if sizer == 0 {
                (4, 4)
            } else {
                let r = h.call_virtual(sizer, 1, &[]);
                (r, r)
            }
        } else {
            let class = h.call_virtual(ty, 1, &[]);
            let size: u32 = h.read(ty + 0x1c);
            let align: u32 = h.read(ty + 0x20);
            if sizer != 0 && class != 0 {
                let size = h.call_virtual(sizer, 2, &[class]);
                let align = h.call_virtual(sizer, 3, &[class]);
                (size, align)
            } else {
                (size, align)
            }
        };
        let off = place(h, this, size, align);
        let parent: u32 = h.read(this + 0x40);
        if parent == 0 {
            off
        } else if sizer != 0 {
            off.wrapping_sub(h.call_virtual(sizer, 2, &[parent]))
        } else {
            off.wrapping_sub(h.read(parent + 0x1c))
        }
    }

    /// `PClassDescriptorDynamic::addMember(name, type, flags, sizer)`
    /// (`0x00449250`, thiscall, `ret 0x10`): 0 for no name, a member of
    /// that name already, or (unless flag bit 1) a class that derives
    /// from this one or is abstract (`+0x90` bit 1); else the place
    /// ([`place_new`]) and a new 0x30-byte `PClassDataMemberDynamic(this,
    /// type, name, offset, flags | 1, 0)`; its constructor's `eax` (0 when
    /// the allocation fails).
    pub fn add_named_member(
        h: &mut impl Host,
        this: u32,
        name: u32,
        ty: u32,
        flags: u32,
        sizer: u32,
    ) -> u32 {
        if name == 0 {
            return 0;
        }
        if h.call_sym(ADD_MEMBER_NAMED[0], Conv::Thiscall(this), &[name]) != 0 {
            return 0;
        }
        let class = h.call_virtual(ty, 1, &[]);
        if class != 0 && flags & 2 == 0 {
            let derives = h.call_sym(ADD_MEMBER_NAMED[1], Conv::Thiscall(class), &[this]);
            if derives & 0xff != 0 {
                return 0;
            }
            let cflags: u32 = h.read(class + 0x90);
            if cflags & 2 != 0 {
                return 0;
            }
        }
        let off = h.call_sym(
            ADD_MEMBER_NAMED[2],
            Conv::Thiscall(this),
            &[ty, flags, sizer],
        );
        let p = h.call_sym(MALLOC, Conv::Cdecl, &[0x30]);
        if p == 0 {
            return 0;
        }
        h.call_sym(
            ADD_MEMBER_NAMED[3],
            Conv::Thiscall(p),
            &[this, ty, name, off, flags | 1, 0],
        )
    }

    /// `0x0045d280` (cdecl `(name)`): the asset registry's entry for
    /// `name` under its lock: the registry (`0x0045d420`, handed the key's
    /// name word by address before it is written, which it ignores), the
    /// key `{hash(name, 0x7b5), name}` in `key` (the port's scratch, the
    /// original's frame) and the tree's `find(&key)`.
    pub fn registry_find(h: &mut impl Host, name: u32, key: u32) -> u32 {
        let lock = h.rebase(REGISTRY_LOCK);
        h.call_sym(REGISTRY_FIND[0], Conv::Thiscall(lock), &[]);
        let tree = h.call_sym(REGISTRY_FIND[2], Conv::Cdecl, &[key + 4]);
        h.write(key + 4, name);
        let hash = h.call_sym(REGISTRY_FIND[3], Conv::Cdecl, &[name, REGISTRY_SEED]);
        h.write(key, hash);
        let r = h.call_sym(REGISTRY_FIND[4], Conv::Thiscall(tree), &[key]);
        h.call_sym(REGISTRY_FIND[1], Conv::Thiscall(lock), &[]);
        r
    }

    /// `0x00442370` (cdecl `(world, L, name)`): pushes every instance of
    /// the class named `name` in the world to the script, list by list
    /// (the world's lists linked at `+0x48`, a cursor of 8 words in
    /// `cursor`: the port's scratch, the original's frame); the count, or
    /// -1 for no such class or no instance.
    ///
    /// The inner loop is the original's: after each push the cursor moves
    /// by its stride; when it reaches the block end it takes the next
    /// block's link and keeps stepping without pushing until the count
    /// runs out, then the cursor's `next` refills it.
    pub fn push_instances(h: &mut impl Host, world: u32, l: u32, name: u32, cursor: u32) -> u32 {
        let ns = h.call_sym(PUSH_INSTANCES[0], Conv::Cdecl, &[]);
        let desc = h.call_sym(PUSH_INSTANCES[1], Conv::Thiscall(ns), &[name]);
        if desc == 0 {
            return u32::MAX;
        }
        if h.call_sym(PUSH_INSTANCES[2], Conv::Thiscall(world), &[desc]) == 0 {
            return u32::MAX;
        }
        let mut count = 0u32;
        let mut link: u32 = h.read(world);
        while link != world && link != 0 && link != 0x48 {
            let list = link - 0x48;
            let head = list + 0x1c;
            let first: u32 = h.read(head);
            let first = if first == head { 0 } else { first };
            for (i, v) in [0, 0, 0, 0, 0, head, first, desc].into_iter().enumerate() {
                h.write(cursor + 4 * i as u32, v);
            }
            h.call_sym(PUSH_INSTANCES[3], Conv::Thiscall(cursor), &[]);
            let mut left: u32 = h.read(cursor + 0xc);
            while left != 0 {
                let mut at: u32 = h.read(cursor);
                let mut end: u32 = h.read(cursor + 4);
                'push: loop {
                    let node: u32 = h.read(cursor + 0x18);
                    let class: u32 = h.read(node + 0x2c);
                    let data: u32 = h.read(cursor + 0x10);
                    h.call_sym(
                        PUSH_INSTANCES[5],
                        Conv::Cdecl,
                        &[l, data.wrapping_add(at), class],
                    );
                    count += 1;
                    loop {
                        let stride: u32 = h.read(cursor + 8);
                        at = at.wrapping_add(stride);
                        left -= 1;
                        h.write(cursor, at);
                        h.write(cursor + 0xc, left);
                        if end != at {
                            if left == 0 {
                                break 'push;
                            }
                            continue 'push;
                        }
                        end = h.read(end);
                        h.write(cursor + 4, end);
                        if left == 0 {
                            break 'push;
                        }
                    }
                }
                h.call_sym(PUSH_INSTANCES[4], Conv::Thiscall(cursor), &[]);
                left = h.read(cursor + 0xc);
            }
            link = h.read(list + 0x48);
        }
        count
    }

    /// A utility registration's slot (`0x00ca2f08 + 4 [e + 8]`) cleared.
    fn clear_utility_slot(h: &mut impl Host, e: u32) {
        let slots = h.rebase(UTILITY_TEARDOWN[1]);
        let i: u32 = h.read(e + 8);
        h.write(slots.wrapping_add(i.wrapping_mul(4)), 0u32);
    }

    /// `0x004c1480` (cdecl `()`, from `PUtilityRendering::terminate`):
    /// the 33 utility registrations (`0x00ca2fcc`, every 0x28 bytes) taken
    /// out: each one's slot (`0x00ca2f08 + 4 [e + 8]`) cleared and the
    /// entry unlinked from the list `0x00ca2f04` (next at `+4`, cleared):
    /// the first three inline, the next five through
    /// `Phyre_PUtilityRendering_unlink_004c1410`, the rest through
    /// `Phyre_PUtilityRendering_unregister_004c1460`; a call's nonzero
    /// `eax` stops it (never, in the game: both return 0). The last call's
    /// `eax`.
    pub fn utility_teardown(h: &mut impl Host) -> u32 {
        let [head, _, first, stride, count, unlink, unregister] = UTILITY_TEARDOWN;
        let head = h.rebase(head);
        for k in 0..3 {
            let e = h.rebase(first + stride * k);
            clear_utility_slot(h, e);
            let mut link = head;
            loop {
                let node: u32 = h.read(link);
                if node == 0 {
                    break;
                }
                if node == e {
                    let next: u32 = h.read(node + 4);
                    h.write(link, next);
                    h.write(node + 4, 0u32);
                    break;
                }
                link = node + 4;
            }
        }
        let mut r = 0;
        for k in 3..count {
            let e = h.rebase(first + stride * k);
            r = if k < 8 {
                clear_utility_slot(h, e);
                h.call_sym(unlink, Conv::Cdecl, &[e])
            } else {
                h.call_sym(unregister, Conv::Cdecl, &[e])
            };
            if r != 0 {
                break;
            }
        }
        r
    }
}
