//! The animation classes' `PArray<T>::DeclareClassDescriptor(name)` copies
//! (cdecl, returns the static descriptor), which their `Bind()`s call with
//! a class-name global's word.
//!
//! - [`DECLARES`]: the twelve copies of the plain shape (4-byte elements),
//!   run by [`ffx_prender::declare::bridge::declare`]. Their constants were
//!   read off each copy's machine code (`~/ffx-work/m4/port/p2c/
//!   gen_decl.py`, after `shape.py` showed every copy is the same code
//!   with other constants).
//! - [`PUINT128`] (`PArray<PUInt128>`, `0x00537400`): the same code with
//!   16-byte elements: the elements member's last word and its stride
//!   (`+0x10c`) are 0x10. [`bridge::declare_sized`].
//! - [`CLIP_SHARRAY`] (`PSharray<PAnimationClip *>`, `0x0051a650`): the
//!   shared array's own layout: an `m_u` array member at `+0x98` first,
//!   its flag and handler words, then the count member (`+0xd8`) and its
//!   annotation (`+0x104`). [`bridge::declare_sharray`].
//!
//! The originals' SEH frames only unwind half-built statics; dropped.

#[allow(clippy::wildcard_imports)]
use ffx_prender::declare::*;

/// The plain copies: `(address, constants)`.
pub const DECLARES: &[(u32, Declare)] = &include!("p2_declare_table.rs.in");

/// The [`DECLARES`] constants of the copy at `f` (a compile error when
/// there is none).
#[must_use]
pub const fn declare_of(f: u32) -> Declare {
    let mut i = 0;
    while i < DECLARES.len() {
        if DECLARES[i].0 == f {
            return DECLARES[i].1;
        }
        i += 1;
    }
    panic!("no such DeclareClassDescriptor copy")
}

/// `0x0053_7400` `PArray<PUInt128>`: its constants (elements of
/// [`PUINT128_SIZE`] bytes).
pub const PUINT128: Declare = Declare {
    desc: 0x00ca_d278,
    ctor: 0x0053_3cc0,
    vtable: 0x00b2_ad3c,
    name: 0x00ca_d8e0,
    element: Element::Getter(0x0043_81b0),
    flags: 0x0,
    member_vtable: 0x00b2_ad80,
    set_storage: 0x0053_c900,
    atexit: [0x00b0_10b0, 0x00b0_1110, 0x00b0_1040, 0x00b0_1170],
};

/// `PUInt128`'s size.
pub const PUINT128_SIZE: u32 = 0x10;

/// The string `"m_u"` (the shared array's member).
pub const M_U: u32 = 0x00b0_edd0;

/// A `PSharray<T>` descriptor's declaration.
#[derive(Debug, Clone, Copy)]
pub struct Sharray {
    /// The static descriptor (guard word at `+0x94`).
    pub desc: u32,
    /// Its constructor (thiscall).
    pub ctor: u32,
    pub vtable: u32,
    /// The class name's global.
    pub name: u32,
    /// The `m_u` member's element type (a descriptor).
    pub element: u32,
    /// The `m_u` member's vtable and its `+0x3c` and `+0x38` handlers.
    pub member_vtable: u32,
    pub member_3c: u32,
    pub member_38: u32,
    /// The `atexit` destructors: descriptor, `m_u`, count member,
    /// annotation.
    pub atexit: [u32; 4],
}

/// `0x0051_a650` `PSharray<PAnimationClip *>`.
pub const CLIP_SHARRAY: Sharray = Sharray {
    desc: 0x00ca_b298,
    ctor: 0x0051_8e10,
    vtable: 0x00b2_76ec,
    name: 0x00ca_b440,
    element: 0x00ca_a7e8,
    member_vtable: 0x00b2_7730,
    member_3c: 0x0051_dbf0,
    member_38: 0x0051_d220,
    atexit: [0x00b0_0930, 0x00b0_0a50, 0x00b0_0a00, 0x00b0_08a0],
};

#[allow(clippy::many_single_char_names)]
pub mod bridge {
    //! The originals over game memory.

    use super::{
        ADD_ANNOTATION, ADD_TO_NAMESPACE, ANNOTATION_CTOR, ANNOTATION_VALUE, ANNOTATION_VTABLE,
        ATEXIT, DATA_MEMBER_CTOR, Declare, Element, M_COUNT, M_ELS, M_U, MEMBER_ARRAY_CTOR,
        MEMBER_HANDLER, REGISTER_TYPE, Sharray, TYPE_UINT,
    };
    use ffx_graphic::host::{Conv, Host};

    fn at_exit(h: &mut impl Host, f: u32) {
        let f = h.rebase(f);
        h.call_sym(ATEXIT, Conv::Cdecl, &[f]);
    }

    /// Sets bit `bit` of the guard word and says whether it was clear.
    fn first_time(h: &mut impl Host, guard: u32, bit: u32) -> bool {
        let g: u32 = h.read(guard);
        if g & bit != 0 {
            return false;
        }
        h.write(guard, g | bit);
        true
    }

    /// The static descriptor's construction on first use (guard bit 0):
    /// its constructor, flag 2 of `+0x90` cleared, its vtable, `atexit`.
    fn build(h: &mut impl Host, d: u32, ctor: u32, vtable: u32, dtor: u32) {
        if first_time(h, d + 0x94, 1) {
            h.call_sym(ctor, Conv::Thiscall(d), &[]);
            let flags: u32 = h.read(d + 0x90);
            h.write(d + 0x90, flags & !2);
            let vt = h.rebase(vtable);
            h.write(d, vt);
            at_exit(h, dtor);
        }
    }

    /// The name, when there is one and the descriptor has none (`+0x18`):
    /// the class's global keeps the first one. False: nothing more to do.
    fn take_name(h: &mut impl Host, d: u32, global: u32, name: u32) -> bool {
        if name == 0 || h.read::<u32>(d + 0x18) != 0 {
            return false;
        }
        let g = h.rebase(global);
        let mut n: u32 = h.read(g);
        if n == 0 {
            n = name;
            h.write(g, n);
        }
        h.write(d + 0x18, n);
        true
    }

    /// The count member `m_count` (`unsigned int`) at `member`.
    fn count_member(h: &mut impl Host, d: u32, member: u32, dtor: u32) {
        let t = h.call_sym(TYPE_UINT, Conv::Cdecl, &[]);
        let m = h.rebase(M_COUNT);
        h.call_sym(
            DATA_MEMBER_CTOR,
            Conv::Thiscall(member),
            &[d, t, m, 0, 0, 0],
        );
        at_exit(h, dtor);
    }

    /// The count's annotation at `annot`.
    fn annotation(h: &mut impl Host, annot: u32, dtor: u32) {
        let t = h.call_sym(TYPE_UINT, Conv::Cdecl, &[]);
        let v = h.rebase(ANNOTATION_VALUE);
        h.call_sym(ANNOTATION_CTOR, Conv::Thiscall(annot), &[v, t, 0]);
        let vt = h.rebase(ANNOTATION_VTABLE);
        h.write(annot, vt);
        h.write(annot + 0x18, 0x7fff_ffffu32);
        at_exit(h, dtor);
    }

    /// `ffx_prender::declare::bridge::declare` with elements of `size`
    /// bytes (the elements member's last word and `+0x10c`).
    pub fn declare_sized(h: &mut impl Host, name: u32, c: &Declare, size: u32) -> u32 {
        let d = h.rebase(c.desc);
        let guard = d + 0x94;
        build(h, d, c.ctor, c.vtable, c.atexit[0]);
        if !take_name(h, d, c.name, name) {
            return d;
        }
        if first_time(h, guard, 2) {
            count_member(h, d, d + 0x98, c.atexit[1]);
        }
        if first_time(h, guard, 4) {
            annotation(h, d + 0xc4, c.atexit[2]);
        }
        if h.read::<u32>(d + 0xc8) == d + 0xc8 {
            h.call_sym(ADD_ANNOTATION, Conv::Thiscall(d + 0xb4), &[d + 0xc4]);
        }
        if first_time(h, guard, 8) {
            let elem = match c.element {
                Element::Descriptor(e) => h.rebase(e),
                Element::Getter(f) => h.call_sym(f, Conv::Cdecl, &[]),
            };
            let m = h.rebase(M_ELS);
            h.call_sym(
                MEMBER_ARRAY_CTOR,
                Conv::Thiscall(d + 0xe0),
                &[d, elem, m, 4, c.flags, 2, size],
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
        h.write(d + 0x10c, size);
        h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[d]);
        h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(d), &[]);
        d
    }

    /// `PSharray<T>::DeclareClassDescriptor(name)`: the static
    /// descriptor; with a name and none yet, the `m_u` member (bit 1; its
    /// flag `0x40` at `+0x14`, `+0x38` handler and size word `+0x2c` set
    /// every time), the count member (bit 2), its annotation (bit 3,
    /// added once), the registration and the namespace; returns the
    /// descriptor.
    pub fn declare_sharray(h: &mut impl Host, name: u32, c: &Sharray) -> u32 {
        let d = h.rebase(c.desc);
        let guard = d + 0x94;
        build(h, d, c.ctor, c.vtable, c.atexit[0]);
        if !take_name(h, d, c.name, name) {
            return d;
        }
        let u = d + 0x98;
        if first_time(h, guard, 2) {
            let elem = h.rebase(c.element);
            let m = h.rebase(M_U);
            h.call_sym(
                MEMBER_ARRAY_CTOR,
                Conv::Thiscall(u),
                &[d, elem, m, 4, 2, 2, 4],
            );
            let vt = h.rebase(c.member_vtable);
            h.write(u, vt);
            let f = h.rebase(MEMBER_HANDLER);
            h.write(u + 0x34, f);
            h.write(u + 0x38, 0u32);
            let f = h.rebase(c.member_3c);
            h.write(u + 0x3c, f);
            at_exit(h, c.atexit[1]);
        }
        let flags: u32 = h.read(u + 0x14);
        h.write(u + 0x14, flags | 0x40);
        let f = h.rebase(c.member_38);
        h.write(u + 0x38, f);
        h.write(u + 0x2c, 4u32);
        if first_time(h, guard, 4) {
            count_member(h, d, d + 0xd8, c.atexit[2]);
        }
        if first_time(h, guard, 8) {
            annotation(h, d + 0x104, c.atexit[3]);
        }
        if h.read::<u32>(d + 0x108) == d + 0x108 {
            h.call_sym(ADD_ANNOTATION, Conv::Thiscall(d + 0xf4), &[d + 0x104]);
        }
        h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[d]);
        h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(d), &[]);
        d
    }
}
