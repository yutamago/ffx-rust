//! M5 stage 1, lists `tmpl-abstract-c` and `tmpl-abstract-d`: the
//! `PClassDescriptorAbstract<T>` vtable slots of the gameplay, scene,
//! framework, input and physics classes.
//!
//! The copies have the shapes of lists a and b, so they run through the
//! generic ports of [`crate::m5p_tmpl_abstract_ab`] (re-exported here as
//! [`bridge`]), each with its own constant from the generated tables
//! (`m5p_tmpl_abstract_cd_table.rs.in`): slot 7 the scalar deleting
//! destructor ([`DTORS`]), slot 12 `bind()` ([`BIND_JUMP`],
//! [`BIND_INLINE`]), slot 13 `getInstanceMember` ([`MEMBER_NONE`]), slot 14
//! `fixObjectsAfterLoading` ([`FIX_NONE`], [`FIX_FORWARD`]).
//!
//! One shape is new: slot 4 (`+0x10`), `putValueToScriptContext(state,
//! object)`, `ret 8`, of three gameplay components (`PScriptedComponent`,
//! `PTimerComponent`, `PCameraControllerComponent`): the object pointer,
//! moved to its base by a constant offset unless it is null, passed with
//! the class descriptor to a cdecl callee ([`SCRIPT_PUT`],
//! [`bridge::script_put_forward`]). Lists a and b's slot 4 of
//! `PClassDescriptorAbstract<PClassDescriptor>` is the same shape with
//! other constants.
//!
//! Image addresses in the tables are rebased before use.

pub use crate::m5p_tmpl_abstract_ab::{
    ADD_TO_NAMESPACE, DESTRUCT, FREE, REGISTER_TYPE, listed, word_of,
};

include!("m5p_tmpl_abstract_cd_table.rs.in");

/// The `(callee, offset, descriptor)` of slot 4 copy `f` in [`SCRIPT_PUT`]
/// (compile time: a missing copy is a build error).
#[must_use]
pub const fn script_put_row(f: u32) -> (u32, u32, u32) {
    let mut i = 0;
    while i < SCRIPT_PUT.len() {
        let (a, callee, offset, d) = SCRIPT_PUT[i];
        if a == f {
            return (callee, offset, d);
        }
        i += 1;
    }
    panic!("not in the table");
}

pub mod bridge {
    //! The generic ports over game memory: lists a and b's, and slot 4's
    //! forwarder.

    pub use crate::m5p_tmpl_abstract_ab::bridge::*;
    use ffx_graphic::host::{Conv, Host};

    /// Slot 4 (`putValueToScriptContext(state, object)`): `callee(state,
    /// object + offset, d)`, or `callee(state, 0, d)` for a null object
    /// (cdecl); `d` is the class descriptor (an image global). Its `eax`.
    pub fn script_put_forward(
        h: &mut impl Host,
        state: u32,
        object: u32,
        callee: u32,
        offset: u32,
        d: u32,
    ) -> u32 {
        let base = if object == 0 {
            0
        } else {
            object.wrapping_add(offset)
        };
        let d = h.rebase(d);
        h.call_sym(callee, Conv::Cdecl, &[state, base, d])
    }
}
