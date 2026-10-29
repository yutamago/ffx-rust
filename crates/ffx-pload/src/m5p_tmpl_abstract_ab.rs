//! M5 stage 1, lists `tmpl-abstract-a` and `tmpl-abstract-b`: the
//! `PClassDescriptorAbstract<T>` vtable slots of each serialisable class.
//!
//! Every class `T` has a `PClassDescriptorAbstract<T>` vtable (and a
//! `...Concrete<T>` or `...WithoutDefaultConstructor<T>` one sharing most
//! slots). The slots here, named after Fahrenheit's Switch symbols for
//! `PClassDescriptorAbstract<T>`, are all thiscall on the descriptor:
//!
//! - slot 7 (`+0x1c`), the scalar deleting destructor
//!   (`~PClassDescriptorAbstract`), `(flags)`, `ret 4`: stores the vtable of
//!   the base descriptor class (the copy's constant, [`DTORS`]), runs
//!   `PClassDescriptor::Destruct` ([`DESTRUCT`], thiscall, no stack words)
//!   and frees the descriptor through `PhyreFree` ([`FREE`]) when `flags`
//!   bit 0 is set; `eax = this`. Only the `atexit` teardown of the
//!   descriptor globals runs it.
//! - slot 12 (`+0x30`), `bind()`, no stack words: a tail jump to the class's
//!   `Bind()` (cdecl, no arguments; [`BIND_JUMP`]), whose `eax` it returns,
//!   or `Bind()` inline: `PTypeDispenser::RegisterType(d)` then a tail jump
//!   to `PClassDescriptor::addToNamespace` on `d` ([`BIND_INLINE`]).
//! - slot 13 (`+0x34`), `getInstanceMember(object, state, name)`,
//!   `ret 0xc`: -1 ([`MEMBER_NONE`]), or the three words forwarded to a
//!   cdecl function ([`MEMBER_FORWARD`]).
//! - slot 14 (`+0x38`), `fixObjectsAfterLoading(list)`, `ret 4`: 0
//!   ([`FIX_NONE`]), the list forwarded to a cdecl function
//!   ([`FIX_FORWARD`]), or one class's own loop
//!   ([`bridge::fix_set_descriptor`]).
//!
//! `PClassDescriptorAbstract<PClassDescriptor>` adds its script slots 3 and
//! 4 ([`bridge::script_get_unsupported`], [`bridge::script_put_descriptor`]).
//!
//! The linker folded identical bodies, so one address can serve several
//! vtables (always of one class here). The constant of each copy is in the
//! generated tables below (`m5p_tmpl_abstract_ab_table.rs.in`), by the
//! copy's address; the shims and the oracle tests read them from here.
//! Image addresses in the tables are rebased before use.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// `PClassDescriptor::Destruct` (thiscall, no stack words).
pub const DESTRUCT: u32 = 0x0043_b3e0;
/// `PhyreFree(block)` (cdecl).
pub const FREE: u32 = 0x0042_fb20;
/// `PTypeDispenser::RegisterType(descriptor)` (cdecl).
pub const REGISTER_TYPE: u32 = 0x0043_95b0;
/// `PClassDescriptor::addToNamespace` (thiscall, no stack words).
pub const ADD_TO_NAMESPACE: u32 = 0x0043_c140;
/// `PhyrePrintf(level, format, ...)` (cdecl, varargs).
pub const PRINTF: u32 = 0x0043_52f0;
/// The message `PClassDescriptorAbstract<PClassDescriptor>`'s slot 3
/// prints.
pub const SCRIPT_GET_MESSAGE: u32 = 0x00b0_e998;
/// The cdecl function slot 4 of `PClassDescriptorAbstract<PClassDescriptor>`
/// forwards to (`(state, object, descriptor)`).
pub const SCRIPT_PUT: u32 = 0x0043_6e80;
/// `PClassDescriptor`'s own class descriptor (the global slot 4 passes).
pub const CLASS_DESCRIPTOR_DESCRIPTOR: u32 = 0x00c9_0760;

include!("m5p_tmpl_abstract_ab_table.rs.in");

/// The constant of copy `f` in `table` (compile time: a missing copy is a
/// build error).
#[must_use]
pub const fn word_of(table: &[(u32, u32)], f: u32) -> u32 {
    let mut i = 0;
    while i < table.len() {
        if table[i].0 == f {
            return table[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// Whether `f` is in `table` (compile time, as [`word_of`]).
#[must_use]
pub const fn listed(table: &[u32], f: u32) -> bool {
    let mut i = 0;
    while i < table.len() {
        if table[i] == f {
            return true;
        }
        i += 1;
    }
    panic!("not in the table");
}

pub mod bridge {
    //! The generic ports over game memory.

    use super::{
        ADD_TO_NAMESPACE, CLASS_DESCRIPTOR_DESCRIPTOR, Conv, DESTRUCT, FREE, Host, Mem, PRINTF,
        REGISTER_TYPE, SCRIPT_GET_MESSAGE, SCRIPT_PUT,
    };

    /// Slot 7: the scalar deleting destructor of the descriptor `this`.
    /// `vtable` is the copy's constant (an image address).
    pub fn scalar_deleting_dtor(h: &mut impl Host, this: u32, flags: u32, vtable: u32) -> u32 {
        let vt = h.rebase(vtable);
        h.write(this, vt);
        h.call_sym(DESTRUCT, Conv::Thiscall(this), &[]);
        // `test byte [flags], 1`
        if flags & 1 != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// Slot 13 of a class without script members: -1.
    #[must_use]
    pub const fn no_instance_member() -> u32 {
        u32::MAX
    }

    /// Slot 13 forwarding its three words to the cdecl function `f`; its
    /// `eax`.
    pub fn forward3(h: &mut impl Host, f: u32, object: u32, state: u32, name: u32) -> u32 {
        h.call_sym(f, Conv::Cdecl, &[object, state, name])
    }

    /// Slot 14 of a class with nothing to fix: 0.
    #[must_use]
    pub const fn no_objects_to_fix() -> u32 {
        0
    }

    /// Slot 14 forwarding the list to the cdecl function `f`; its `eax`.
    pub fn forward1(h: &mut impl Host, f: u32, list: u32) -> u32 {
        h.call_sym(f, Conv::Cdecl, &[list])
    }

    /// Slot 14 of `0x0044e070`: every object of the instance list gets the
    /// list's descriptor at `+8`. The list holds the descriptor at `+0x2c`,
    /// the first object at `+0x30`, the count at `+0x3c`; the stride is the
    /// descriptor's instance size (`+0x1c`, read before the count is
    /// checked). 0.
    pub fn fix_set_descriptor(m: &mut impl Mem, list: u32) -> u32 {
        let count: u32 = m.read(list.wrapping_add(0x3c));
        let mut object: u32 = m.read(list.wrapping_add(0x30));
        let descriptor: u32 = m.read(list.wrapping_add(0x2c));
        let stride: u32 = m.read(descriptor.wrapping_add(0x1c));
        for _ in 0..count {
            m.write(object.wrapping_add(8), descriptor);
            object = object.wrapping_add(stride);
        }
        0
    }

    /// Slot 12 as a tail jump to the class's `Bind()` at `target` (cdecl,
    /// no arguments); its `eax`.
    pub fn bind_jump(h: &mut impl Host, target: u32) -> u32 {
        h.call_sym(target, Conv::Cdecl, &[])
    }

    /// Slot 12 with `Bind()` inline: registers the class descriptor `d`
    /// (an image global) and adds it to its namespace; `addToNamespace`'s
    /// `eax`.
    pub fn bind_inline(h: &mut impl Host, d: u32) -> u32 {
        let d = h.rebase(d);
        h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[d]);
        h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(d), &[])
    }

    /// Slot 3 of `PClassDescriptorAbstract<PClassDescriptor>`
    /// (`getValueFromScriptContext`): prints its message at level 2;
    /// `PhyrePrintf`'s `eax`.
    pub fn script_get_unsupported(h: &mut impl Host) -> u32 {
        let message = h.rebase(SCRIPT_GET_MESSAGE);
        h.call_sym(PRINTF, Conv::Cdecl, &[2, message])
    }

    /// Slot 4 of `PClassDescriptorAbstract<PClassDescriptor>`
    /// (`putValueToScriptContext(state, object)`): `0x00436e80(state,
    /// object + 0x18 or 0 for none, PClassDescriptor's descriptor)`; its
    /// `eax`.
    pub fn script_put_descriptor(h: &mut impl Host, state: u32, object: u32) -> u32 {
        let base = if object == 0 {
            0
        } else {
            object.wrapping_add(0x18)
        };
        let d = h.rebase(CLASS_DESCRIPTOR_DESCRIPTOR);
        h.call_sym(SCRIPT_PUT, Conv::Cdecl, &[state, base, d])
    }
}
