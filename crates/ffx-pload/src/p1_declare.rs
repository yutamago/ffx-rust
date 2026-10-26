//! List `p1-load`, continuation round 3, batch 2: two `PArray<T>`
//! descriptors' `DeclareClassDescriptor(name)` copies.
//!
//! `PArray<PTypedObject, 8>` (`0x00458460`) is the shape h2's
//! [`ffx_prender::declare`] bridge runs; `PArray<u8, 1>` (`0x00448450`)
//! differs in its elements' alignment (1, where every other copy has 4:
//! the elements member's last constructor word and `+0x10c`), so it runs
//! [`bridge::declare_aligned`], the same steps with the alignment as an
//! input.

use ffx_graphic::host::{Conv, Host};
use ffx_prender::declare::{
    ADD_ANNOTATION, ADD_TO_NAMESPACE, ANNOTATION_CTOR, ANNOTATION_VALUE, ANNOTATION_VTABLE, ATEXIT,
    DATA_MEMBER_CTOR, Declare, Element, M_COUNT, M_ELS, MEMBER_ARRAY_CTOR, MEMBER_HANDLER,
    REGISTER_TYPE, TYPE_UINT,
};

/// `0x00458460` `PArray<PTypedObject, 8>`'s constants.
pub const PTYPED_OBJECT_ARRAY: Declare = Declare {
    desc: 0x00c9_3c58,
    ctor: 0x0045_77e0,
    vtable: 0x00b1_1604,
    name: 0x00c9_3dd4,
    element: Element::Descriptor(0x00c9_0f30),
    flags: 0,
    member_vtable: 0x00b1_1648,
    set_storage: 0x0045_a110,
    atexit: [0x00af_ad60, 0x00af_ae80, 0x00af_ad50, 0x00af_ae90],
};

/// `0x00448450` `PArray<u8, 1>`'s constants (its elements' type from
/// `0x00437d30`, `GetType<u8>`).
pub const U8_ARRAY: Declare = Declare {
    desc: 0x00c9_15c8,
    ctor: 0x0044_73f0,
    vtable: 0x00b0_fcf4,
    name: 0x00c9_1928,
    element: Element::Getter(0x0043_7d30),
    flags: 0,
    member_vtable: 0x00b0_fd4c,
    set_storage: 0x0044_a2d0,
    atexit: [0x00af_a420, 0x00af_a450, 0x00af_a410, 0x00af_a480],
};

/// `U8_ARRAY`'s elements' alignment.
pub const U8_ALIGNMENT: u32 = 1;

/// The ports over game memory.
pub mod bridge {
    use super::{
        ADD_ANNOTATION, ADD_TO_NAMESPACE, ANNOTATION_CTOR, ANNOTATION_VALUE, ANNOTATION_VTABLE,
        ATEXIT, Conv, DATA_MEMBER_CTOR, Declare, Element, Host, M_COUNT, M_ELS, MEMBER_ARRAY_CTOR,
        MEMBER_HANDLER, REGISTER_TYPE, TYPE_UINT,
    };

    fn at_exit(h: &mut impl Host, f: u32) {
        let f = h.rebase(f);
        h.call_sym(ATEXIT, Conv::Cdecl, &[f]);
    }

    /// The `PArray<T>` descriptor's `DeclareClassDescriptor(name)` with
    /// the elements' alignment `align`: on first use the static
    /// descriptor (guard bit 0 of `+0x94`; its flags' bit 1 cleared, the
    /// vtable); then, for a name when the descriptor has none (`+0x18`;
    /// the class's global keeps the first), the count member (bit 1),
    /// its annotation (bit 2, added once), the elements member (bit 3:
    /// `PClassDataMemberArray(desc, type, "m_els", 4, flags, 2, align)`,
    /// the dynamic array's vtable and handlers), `+0x10c` = `align`, the
    /// registration and the namespace. Returns the descriptor.
    pub fn declare_aligned(h: &mut impl Host, name: u32, c: &Declare, align: u32) -> u32 {
        let d = h.rebase(c.desc);
        let guard = d + 0x94;
        let mut g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            h.call_sym(c.ctor, Conv::Thiscall(d), &[]);
            let flags: u32 = h.read(d + 0x90);
            h.write(d + 0x90, flags & !2);
            let vt = h.rebase(c.vtable);
            h.write(d, vt);
            at_exit(h, c.atexit[0]);
            g = h.read(guard);
        }
        if name == 0 || h.read::<u32>(d + 0x18) != 0 {
            return d;
        }
        let ng = h.rebase(c.name);
        let mut n: u32 = h.read(ng);
        if n == 0 {
            n = name;
            h.write(ng, n);
        }
        h.write(d + 0x18, n);
        if g & 2 == 0 {
            h.write(guard, g | 2);
            let t = h.call_sym(TYPE_UINT, Conv::Cdecl, &[]);
            let m = h.rebase(M_COUNT);
            h.call_sym(
                DATA_MEMBER_CTOR,
                Conv::Thiscall(d + 0x98),
                &[d, t, m, 0, 0, 0],
            );
            at_exit(h, c.atexit[1]);
            g = h.read(guard);
        }
        if g & 4 == 0 {
            h.write(guard, g | 4);
            let t = h.call_sym(TYPE_UINT, Conv::Cdecl, &[]);
            let v = h.rebase(ANNOTATION_VALUE);
            h.call_sym(ANNOTATION_CTOR, Conv::Thiscall(d + 0xc4), &[v, t, 0]);
            let vt = h.rebase(ANNOTATION_VTABLE);
            h.write(d + 0xc4, vt);
            h.write(d + 0xdc, 0x7fff_ffffu32);
            at_exit(h, c.atexit[2]);
            g = h.read(guard);
        }
        if h.read::<u32>(d + 0xc8) == d + 0xc8 {
            h.call_sym(ADD_ANNOTATION, Conv::Thiscall(d + 0xb4), &[d + 0xc4]);
            g = h.read(guard);
        }
        if g & 8 == 0 {
            h.write(guard, g | 8);
            let elem = match c.element {
                Element::Descriptor(e) => h.rebase(e),
                Element::Getter(f) => h.call_sym(f, Conv::Cdecl, &[]),
            };
            let m = h.rebase(M_ELS);
            h.call_sym(
                MEMBER_ARRAY_CTOR,
                Conv::Thiscall(d + 0xe0),
                &[d, elem, m, 4, c.flags, 2, align],
            );
            let vt = h.rebase(c.member_vtable);
            h.write(d + 0xe0, vt);
            let f = h.rebase(MEMBER_HANDLER);
            h.write(d + 0x114, f);
            h.write(d + 0x118, 0u32);
            let f = h.rebase(c.set_storage);
            h.write(d + 0x11c, f);
            at_exit(h, c.atexit[3]);
        }
        h.write(d + 0x10c, align);
        h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[d]);
        h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(d), &[]);
        d
    }
}
