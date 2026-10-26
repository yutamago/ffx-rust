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
}
