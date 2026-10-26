//! List `p1-load`, continuation round, batch 11: the class descriptor
//! constructors and the statics of the `Vectormath` wrapper types.
//!
//! The wrapper types (`Matrix3`, `Matrix4`, `PMatrix4`) keep one static
//! block each: the class descriptor (0x94 bytes), its guard word
//! (`+0x94`, bit 0 the descriptor built, bit 1 the member), its "bound"
//! byte (`+0x98`) and its one member, a fixed-size float array
//! (`+0x9c`).

use ffx_graphic::host::{Conv, Host};

use crate::descriptor::GLOBAL_NAMESPACE;
use crate::types::ATEXIT;

/// `PClassDescriptor`'s vtable.
pub const DESCRIPTOR_VTABLE: u32 = 0x00b0_ea9c;
/// The class descriptor serial counter (each constructor takes the
/// next).
pub const DESCRIPTOR_SERIAL: u32 = 0x00c9_0758;
/// `PClassDescriptorDynamic`'s vtable.
pub const DYNAMIC_VTABLE: u32 = 0x00b0_fa8c;

/// A wrapper type's static block and constants.
#[derive(Debug, Clone, Copy)]
pub struct Wrapper {
    /// The getter.
    pub getter: u32,
    /// The class descriptor (the block's start).
    pub desc: u32,
    /// Its name.
    pub name: u32,
    /// Its size.
    pub size: u32,
    /// Its vtable.
    pub vtable: u32,
    /// The descriptor's `atexit` destructor.
    pub desc_dtor: u32,
    /// The member's element count.
    pub count: u32,
    /// The member's `atexit` destructor.
    pub member_dtor: u32,
}

/// `PTypeDispenser::GetClassDescriptor<T>()` of the three wrappers.
pub const WRAPPERS: [Wrapper; 3] = [
    Wrapper {
        getter: 0x0045_f210,
        desc: 0x00c9_4b00,
        name: 0x00b1_1f40,
        size: 0x30,
        vtable: 0x00b1_22a4,
        desc_dtor: 0x00af_b110,
        count: 0xc,
        member_dtor: 0x00af_b210,
    },
    Wrapper {
        getter: 0x0045_f300,
        desc: 0x00c9_4bd8,
        name: 0x00b1_1f48,
        size: 0x40,
        vtable: 0x00b1_2334,
        desc_dtor: 0x00af_b130,
        count: 0x10,
        member_dtor: 0x00af_b220,
    },
    Wrapper {
        getter: 0x0045_f3f0,
        desc: 0x00c9_4cb0,
        name: 0x00b1_1f50,
        size: 0x40,
        vtable: 0x00b1_23c4,
        desc_dtor: 0x00af_b150,
        count: 0x10,
        member_dtor: 0x00af_b230,
    },
];

/// The wrapper's getter.
#[must_use]
pub fn wrapper(getter: u32) -> Option<&'static Wrapper> {
    WRAPPERS.iter().find(|w| w.getter == getter)
}

/// The ports over game memory.
pub mod bridge {
    use super::{
        ATEXIT, Conv, DESCRIPTOR_SERIAL, DESCRIPTOR_VTABLE, DYNAMIC_VTABLE, GLOBAL_NAMESPACE, Host,
        Wrapper,
    };
    use crate::descriptor::bridge::copy_string;

    /// `PClassDescriptor::PClassDescriptor(desc, ns, name, size, align,
    /// base, c)` (`0x0043b0a0`, thiscall; `0x0043afb0` is the same with
    /// no base and `c` 0): its empty lists, the name, size and
    /// alignment, the namespace (`+0x3c`), the parent (`+0x40`), the next
    /// serial (`+0x7c`), `c` (`+0x84`), -1 (`+0x80`), zero words
    /// elsewhere. Returns `desc`.
    pub fn descriptor_ctor(h: &mut impl Host, desc: u32, args: [u32; 6]) -> u32 {
        let [ns, name, size, align, base, c] = args;
        for at in [0x2c, 0x34, 0x44, 0x4c, 0x54, 0x5c] {
            h.write(desc + at, [desc + at, desc + at]);
        }
        let list = desc + 4;
        h.write(list, [list, list, list, 0, 0]);
        h.write(desc + 0x18, [name, size, align, 0, 0]);
        let vt = h.rebase(DESCRIPTOR_VTABLE);
        h.write(desc, vt);
        h.write(desc + 0x3c, [ns, base]);
        h.write(desc + 0x64, [0u32; 6]);
        let serial_at = h.rebase(DESCRIPTOR_SERIAL);
        let serial: u32 = h.read(serial_at);
        h.write(desc + 0x7c, serial);
        h.write(serial_at, serial.wrapping_add(1));
        h.write(desc + 0x80, [u32::MAX, c, 0, 0, 0]);
        desc
    }

    /// `PClassDescriptorDynamic::PClassDescriptorDynamic(desc, ns, name,
    /// base)` (`0x00447690`, thiscall): [`descriptor_ctor`] (through
    /// `0x0043b0a0`) with the base's size and alignment and `-size`, its
    /// vtable, the base size (`+0x98`) and zero words, a copy of `name`
    /// (`+0xb4`, alignment 2; 0 for none) which (less bit 0) is the
    /// descriptor's name, flags bit 0, then `updateBaseOffsets`
    /// (`0x0043d6e0`) and the buffers' refresh (`0x00449e40`). Returns
    /// `desc`.
    pub fn dynamic_ctor(h: &mut impl Host, desc: u32, ns: u32, name: u32, base: u32) -> u32 {
        let size: u32 = h.read(base + 0x1c);
        let align: u32 = h.read(base + 0x20);
        h.call_sym(
            0x0043_b0a0,
            Conv::Thiscall(desc),
            &[ns, name, size, align, base, size.wrapping_neg()],
        );
        let vt = h.rebase(DYNAMIC_VTABLE);
        h.write(desc, vt);
        h.write(desc + 0x94, 0u32);
        let size: u32 = h.read(base + 0x1c);
        h.write(desc + 0x98, size);
        h.write(desc + 0x9c, [0u32; 6]);
        let copy = if name == 0 {
            0
        } else {
            copy_string(h, name, 2)
        };
        h.write(desc + 0xb4, copy);
        let flags: u32 = h.read(desc + 0x90);
        h.write(desc + 0x90, flags | 1);
        h.write(desc + 0x18, copy.wrapping_sub(copy & 1));
        h.call_sym(0x0043_d6e0, Conv::Thiscall(desc), &[]);
        h.call_sym(0x0044_9e40, Conv::Thiscall(desc), &[]);
        desc
    }

    /// A wrapper type's `GetClassDescriptor<T>()` (cdecl): on the first
    /// call (guard bit 0) its descriptor (`0x0043afb0` with the global
    /// namespace, alignment 4), vtable and `atexit`; until bound, its
    /// member (guard bit 1: `PClassDataMemberArrayFixedSize`, `0x00575f30`,
    /// `m_elements` of `GetType<float>` (`0x00438060`), `count` of them;
    /// `atexit`), `addToNamespace` (`0x0043c140`) and the bound byte.
    /// Returns the descriptor.
    pub fn wrapper_descriptor(h: &mut impl Host, w: &Wrapper) -> u32 {
        let desc = h.rebase(w.desc);
        let guard_at = desc + 0x94;
        let mut guard: u32 = h.read(guard_at);
        if guard & 1 == 0 {
            h.write(guard_at, guard | 1);
            let name = h.rebase(w.name);
            let ns = h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
            h.call_sym(0x0043_afb0, Conv::Thiscall(desc), &[ns, name, w.size, 4]);
            let vt = h.rebase(w.vtable);
            h.write(desc, vt);
            let dtor = h.rebase(w.desc_dtor);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
            guard = h.read(guard_at);
        }
        if h.read::<u8>(desc + 0x98) == 0 {
            if guard & 2 == 0 {
                h.write(guard_at, guard | 2);
                let ty = h.call_sym(0x0043_8060, Conv::Cdecl, &[]);
                let name = h.rebase(0x00b1_1fec);
                h.call_sym(
                    0x0057_5f30,
                    Conv::Thiscall(desc + 0x9c),
                    &[desc, ty, name, 0, 0, w.count],
                );
                let dtor = h.rebase(w.member_dtor);
                h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
            }
            h.call_sym(0x0043_c140, Conv::Thiscall(desc), &[]);
            h.write(desc + 0x98, 1u8);
        }
        desc
    }

    /// `PClassDescriptorDynamic::~PClassDescriptorDynamic(desc)`
    /// (`0x00447880`, thiscall): its vtable back; every member it owns
    /// (list `+0x44`, the member at link `- 4`, flags bit 0) deleted (slot
    /// 0 with 1; the next one taken first); its name copy (`+0xb4`) freed
    /// unless bit 0; the byte arrays `+0xac`, `+0xa4`, `+0x9c` freed when
    /// owned and set, and cleared; then `PClassDescriptor::Destruct`
    /// (`0x0043b3e0`).
    pub fn dynamic_dtor(h: &mut impl Host, desc: u32) {
        let vt = h.rebase(DYNAMIC_VTABLE);
        h.write(desc, vt);
        let head = desc + 0x44;
        let member_of = |link: u32| {
            if link == head || link == 0 {
                0
            } else {
                link.wrapping_sub(4)
            }
        };
        let mut member = member_of(h.read(head));
        while member != 0 {
            let next = member_of(h.read(member + 4));
            let flags: u8 = h.read(member + 0x14);
            if flags & 1 != 0 {
                h.call_virtual(member, 0, &[1]);
            }
            member = next;
        }
        let name: u32 = h.read(desc + 0xb4);
        if name & 1 == 0 {
            h.call_sym(crate::descriptor::FREE, Conv::Cdecl, &[name]);
        }
        for at in [0xac, 0xa4, 0x9c] {
            let [count, data]: [u32; 2] = h.read(desc + at);
            if count >> 31 == 0 && data != 0 {
                h.call_sym(crate::descriptor::FREE, Conv::Cdecl, &[data]);
            }
            h.write(desc + at, [0u32, 0]);
        }
        h.call_sym(0x0043_b3e0, Conv::Thiscall(desc), &[]);
    }

    /// Unlinks `node` from its list and leaves it pointing at itself.
    fn unlink(h: &mut impl Host, node: u32) {
        let [next, prev]: [u32; 2] = h.read(node);
        h.write(next + 4, prev);
        h.write(prev, next);
        h.write(node, [node, node]);
    }

    /// `PClassDescriptor::Destruct(desc)` (`0x0043b3e0`, thiscall; the
    /// destructor every class descriptor ends in): its vtable back; the
    /// serial counter (`0x00c90758`) handed back when this one was the
    /// last; out of its namespace (`0x0043ec40` on `+0x3c`); the entries
    /// of the lists `+0x4c` and `+0x54` it owns (flags `+0x10` bit 0)
    /// unlinked and freed, their names (`+0xc`) first; the lists `+0x44`,
    /// `+0x4c`, `+0x54`, `+0x5c` emptied; `0x005758e0` on `+0x34`; then
    /// `PType`'s vtable (`0x00b0e568`).
    pub fn destruct(h: &mut impl Host, desc: u32) {
        let vt = h.rebase(DESCRIPTOR_VTABLE);
        h.write(desc, vt);
        let serial_at = h.rebase(DESCRIPTOR_SERIAL);
        let serial: u32 = h.read(serial_at);
        let own: u32 = h.read(desc + 0x7c);
        if own.wrapping_add(1) == serial {
            h.write(serial_at, serial.wrapping_sub(1));
        }
        let ns: u32 = h.read(desc + 0x3c);
        h.call_sym(0x0043_ec40, Conv::Thiscall(ns), &[desc]);
        for head in [desc + 0x4c, desc + 0x54] {
            let first: u32 = h.read(head);
            let mut e = if first == head { 0 } else { first };
            while e != 0 {
                let next: u32 = h.read(e);
                let after = if next == head { 0 } else { next };
                if h.read::<u8>(e + 0x10) & 1 != 0 {
                    unlink(h, e);
                    if h.read::<u8>(e + 0x10) & 1 != 0 {
                        let name: u32 = h.read(e + 0xc);
                        h.call_sym(crate::descriptor::FREE, Conv::Cdecl, &[name]);
                    }
                    h.call_sym(crate::descriptor::FREE, Conv::Cdecl, &[e]);
                }
                e = after;
            }
        }
        for head in [desc + 0x44, desc + 0x4c, desc + 0x54, desc + 0x5c] {
            loop {
                let first: u32 = h.read(head);
                if first == head || first == 0 {
                    break;
                }
                unlink(h, first);
            }
        }
        h.call_sym(0x0057_58e0, Conv::Thiscall(desc + 0x34), &[]);
        let vt = h.rebase(0x00b0_e568);
        h.write(desc, vt);
    }

    /// `PClassDescriptor::SortClassDescriptors()` (`0x0043be70`, cdecl):
    /// numbers every class descriptor so that a class and all its
    /// descendants are contiguous (`+0x80`). A table of `n + 1` entries
    /// of 20 bytes (`n` the serial counter `0x00c90758`; allocated,
    /// `0x0042fb80(.., 4)`, 0xd when that fails, and initialised by
    /// `0x0043bcf0`), indexed by serial (`+0x7c`): the descriptor, its
    /// descendant count, its depth, its parent's serial (`n` for none),
    /// the next number. The global namespace's descriptors fill it; then
    /// level by level each descriptor takes its parent's next number and
    /// reserves room for its descendants. The descriptor count goes to
    /// `0x00c9075c`; the table is freed. 0.
    pub fn sort_class_descriptors(h: &mut impl Host) -> u32 {
        let n: u32 = h.read(h.rebase(DESCRIPTOR_SERIAL));
        let cap = n.wrapping_add(1);
        let mut table = 0;
        let mut owned = 0u32;
        if cap != 0 {
            table = h.call_sym(
                crate::descriptor::ALLOC,
                Conv::Cdecl,
                &[cap.wrapping_mul(20), 4],
            );
            if table == 0 {
                return 0xd;
            }
            h.call_sym(0x0043_bcf0, Conv::Cdecl, &[table, cap]);
            owned = cap;
        }
        let entry = |i: u32| table.wrapping_add(i.wrapping_mul(20));
        h.write(entry(n) + 0x10, 0u32);
        let mut total = 0u32;
        let ns = h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
        let head = ns + 8;
        let mut link: u32 = h.read(head);
        while link != head && link != 0 && link != 0x2c {
            let desc = link - 0x2c;
            let mut parent: u32 = h.read(desc + 0x40);
            let serial: u32 = h.read(desc + 0x7c);
            let e = entry(serial);
            let up = if parent == 0 {
                n
            } else {
                h.read(parent + 0x7c)
            };
            h.write(e + 0xc, up);
            let mut depth = 0u32;
            while parent != 0 {
                depth += 1;
                let ps: u32 = h.read(parent + 0x7c);
                let at = entry(ps) + 4;
                let c: u32 = h.read(at);
                h.write(at, c.wrapping_add(1));
                parent = h.read(parent + 0x40);
            }
            total = total.wrapping_add(1);
            h.write(e, desc);
            h.write(e + 8, depth);
            link = h.read(desc + 0x2c);
        }
        let mut level = 0u32;
        loop {
            let mut more = false;
            for i in 0..n {
                let e = entry(i);
                let d: u32 = h.read(e + 8);
                if d == level {
                    let up: u32 = h.read(e + 0xc);
                    let pe = entry(up);
                    let next: u32 = h.read(pe + 0x10);
                    let desc: u32 = h.read(e);
                    h.write(desc + 0x80, next);
                    let next = next.wrapping_add(1);
                    h.write(pe + 0x10, next);
                    h.write(e + 0x10, next);
                    let below: u32 = h.read(e + 4);
                    h.write(pe + 0x10, next.wrapping_add(below));
                } else if (d as i32) > (level as i32) {
                    more = true;
                }
            }
            level = level.wrapping_add(1);
            if !more {
                break;
            }
        }
        h.write(h.rebase(0x00c9_075c), total);
        if owned >> 31 == 0 && table != 0 {
            h.call_sym(crate::descriptor::FREE, Conv::Cdecl, &[table]);
        }
        0
    }
}
