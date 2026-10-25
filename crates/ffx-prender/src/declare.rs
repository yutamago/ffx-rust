//! `PhyreEngine`'s class descriptors of the render types' arrays, as they
//! are built: `PClassDescriptorAbstract<T>`'s constructor and the static
//! `PArray<T>` descriptor's first-use registration (h2-render-b round 2).
//!
//! Each template instance is its own copy in FFX.exe; the bridges take
//! what differs (the descriptor's address, its vtables, the name and
//! buffer globals, the `atexit` destructors) as a [`Declare`] or
//! [`Abstract`] value, and the shims pass each copy's own.
//!
//! The originals' SEH frames only unwind the half-built statics, and
//! nothing on these paths throws: dropped.

use ffx_graphic::host::{Conv, Host};

/// `Phyre::PBase::GetClassDescriptor()` (cdecl): the base class.
pub const PBASE_DESCRIPTOR: u32 = 0x0043_5740;
/// `Phyre::PNamespace::GetGlobalNamespace()` (cdecl).
pub const GLOBAL_NAMESPACE: u32 = 0x0043_e2f0;
/// `Phyre::PClassDescriptor::PClassDescriptor(this; ns, name, size,
/// align, base, flags)` (thiscall, six words).
pub const DESCRIPTOR_CTOR: u32 = 0x0043_b0a0;
/// `setDefaultBuffer(this; p)`.
pub const SET_DEFAULT_BUFFER: u32 = 0x0043_d510;
/// `setValidationBuffer(this; p)`.
pub const SET_VALIDATION_BUFFER: u32 = 0x0043_d5b0;
/// `setWriteMaskBuffer(this; p)`.
pub const SET_WRITE_MASK_BUFFER: u32 = 0x0043_d5c0;
/// The CRT's `atexit(f)` (cdecl).
pub const ATEXIT: u32 = 0x0094_9207;
/// `Phyre::PTypeDispenser::GetType<unsigned int>()` (cdecl).
pub const TYPE_UINT: u32 = 0x0043_7fc0;
/// `Phyre::PClassDataMember::PClassDataMember(this; owner, type, name,
/// a, b, c)` (thiscall, six words).
pub const DATA_MEMBER_CTOR: u32 = 0x0057_52a0;
/// `Phyre::PAnnotation::PAnnotation(this; value, type, c)` (thiscall,
/// three words).
pub const ANNOTATION_CTOR: u32 = 0x0057_5840;
/// `Phyre::PAnnotatable::addAnnotation(this; annotation)`.
pub const ADD_ANNOTATION: u32 = 0x0057_5c10;
/// `Phyre::PClassDataMemberArray::PClassDataMemberArray(this; owner,
/// type, name, size, flags, a, b)` (thiscall, seven words).
pub const MEMBER_ARRAY_CTOR: u32 = 0x0057_5ef0;
/// `Phyre::PTypeDispenser::RegisterType(desc)` (cdecl).
pub const REGISTER_TYPE: u32 = 0x0043_95b0;
/// `Phyre::PClassDescriptor::addToNamespace(this)`.
pub const ADD_TO_NAMESPACE: u32 = 0x0043_c140;

/// The string `"m_count"` (the count member's name).
pub const M_COUNT: u32 = 0x00b0_fd34;
/// The string `"m_els"` (the elements member's name).
pub const M_ELS: u32 = 0x00b0_fd78;
/// The annotation's value word (`0x00c907f4`, the same in every copy).
pub const ANNOTATION_VALUE: u32 = 0x00c9_07f4;
/// `PAnnotation`'s vtable as the copies store it.
pub const ANNOTATION_VTABLE: u32 = 0x00b0_fd40;
/// The element member's first handler (`0x004320c0`, every copy).
pub const MEMBER_HANDLER: u32 = 0x0043_20c0;

/// Where a class name comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Name {
    /// The word at this global (set at first use).
    Global(u32),
    /// This image string.
    Image(u32),
}

/// One copy of `PClassDescriptorAbstract<T>::PClassDescriptorAbstract`.
#[derive(Debug, Clone, Copy)]
pub struct Abstract {
    pub name: Name,
    /// The instance size the descriptor records.
    pub size: u32,
    pub vtable: u32,
    pub default_buffer: u32,
    pub validation_buffer: u32,
    pub write_mask_buffer: u32,
}

/// The element type of a `PArray<T>` descriptor's elements member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Element {
    /// This descriptor (image address).
    Descriptor(u32),
    /// What this getter (cdecl, no arguments) returns.
    Getter(u32),
}

/// One copy of the `PArray<T>` descriptor's declaration.
#[derive(Debug, Clone, Copy)]
pub struct Declare {
    /// The static descriptor (its first-use guard word is at `+0x94`).
    pub desc: u32,
    /// The descriptor's constructor (an [`Abstract`] copy, thiscall).
    pub ctor: u32,
    /// `PClassDescriptorConcrete<PArray<T>>`'s vtable.
    pub vtable: u32,
    /// The class name's global.
    pub name: u32,
    pub element: Element,
    /// The elements member's flags word.
    pub flags: u32,
    /// `PClassDataMemberArrayDynamic<PArray<T>>`'s vtable.
    pub member_vtable: u32,
    /// The member's storage setter (`+0x3c`).
    pub set_storage: u32,
    /// The `atexit` destructors: descriptor, count member, annotation,
    /// elements member.
    pub atexit: [u32; 4],
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ADD_ANNOTATION, ADD_TO_NAMESPACE, ANNOTATION_CTOR, ANNOTATION_VALUE, ANNOTATION_VTABLE,
        ATEXIT, Abstract, Conv, DATA_MEMBER_CTOR, DESCRIPTOR_CTOR, Declare, Element,
        GLOBAL_NAMESPACE, Host, M_COUNT, M_ELS, MEMBER_ARRAY_CTOR, MEMBER_HANDLER, Name,
        PBASE_DESCRIPTOR, REGISTER_TYPE, SET_DEFAULT_BUFFER, SET_VALIDATION_BUFFER,
        SET_WRITE_MASK_BUFFER, TYPE_UINT,
    };

    /// `PClassDescriptorAbstract<T>::PClassDescriptorAbstract(this)`:
    /// the base descriptor built on `PBase`'s (`size`, alignment 4, no
    /// flags) in the global namespace, flag 2 of `+0x90` set, the vtable,
    /// the three buffers; returns `this`.
    pub fn abstract_ctor(h: &mut impl Host, this: u32, c: &Abstract) -> u32 {
        let base = h.call_sym(PBASE_DESCRIPTOR, Conv::Cdecl, &[]);
        let name = match c.name {
            Name::Global(g) => h.read(h.rebase(g)),
            Name::Image(s) => h.rebase(s),
        };
        let ns = h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
        h.call_sym(
            DESCRIPTOR_CTOR,
            Conv::Thiscall(this),
            &[ns, name, c.size, 4, base, 0],
        );
        let flags: u32 = h.read(this + 0x90);
        h.write(this + 0x90, flags | 2);
        let vt = h.rebase(c.vtable);
        h.write(this, vt);
        for (f, buffer) in [
            (SET_DEFAULT_BUFFER, c.default_buffer),
            (SET_VALIDATION_BUFFER, c.validation_buffer),
            (SET_WRITE_MASK_BUFFER, c.write_mask_buffer),
        ] {
            let b = h.rebase(buffer);
            h.call_sym(f, Conv::Thiscall(this), &[b]);
        }
        this
    }

    /// `atexit(f)` of the image function `f`.
    fn at_exit(h: &mut impl Host, f: u32) {
        let f = h.rebase(f);
        h.call_sym(ATEXIT, Conv::Cdecl, &[f]);
    }

    /// The `PArray<T>` descriptor's `DeclareClassDescriptor(name)`: on
    /// first use the static descriptor (guard bit 0); then, for a name
    /// when the descriptor has none (`+0x18`), the name (the class's
    /// global keeps the first one), the count member (bit 1), its
    /// annotation (bit 2, added once), the elements member (bit 3), the
    /// registration and the namespace; returns the descriptor.
    pub fn declare(h: &mut impl Host, name: u32, c: &Declare) -> u32 {
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
                &[d, elem, m, 4, c.flags, 2, 4],
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
        h.write(d + 0x10c, 4u32);
        h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[d]);
        h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(d), &[]);
        d
    }
}
