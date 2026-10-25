//! Object boilerplate the engine and the remaster's dynamic meshes share:
//! scalar deleting destructors, constructors over
//! `PDynamicMeshInstance`'s, and class registrations.
//!
//! - A scalar deleting destructor (thiscall `(this, flags)`, `ret 4`,
//!   returns `this`) stores its class's base vtable (or none), runs a base
//!   destructor (thiscall, no stack words; or none), and frees `this` with
//!   `Phyre_PhyreFree` when `flags & 1`.
//! - A dynamic-mesh instance constructor (thiscall `(this, mesh)`, `ret
//!   4`, returns `this`) runs `PDynamicMeshInstance`'s (`0x004fa420`) and
//!   then stores its own vtable.
//! - A class registration (cdecl, no arguments) registers a descriptor
//!   with `Phyre_PTypeDispenser_RegisterType` and tail-calls
//!   `Phyre_PClassDescriptor_addToNamespace` on it, returning its `eax`.

use ffx_graphic::host::{Conv, Host};

use crate::PHYRE_FREE;

/// `PDynamicMeshInstance`'s constructor (thiscall `(this, mesh)`, `ret 4`).
pub const DYNAMIC_MESH_INSTANCE_CTOR: u32 = 0x004f_a420;
/// `Phyre_PTypeDispenser_RegisterType` (cdecl `(desc)`).
pub const REGISTER_TYPE: u32 = 0x0043_95b0;
/// `Phyre_PClassDescriptor_addToNamespace` (thiscall `(desc)`).
pub const ADD_TO_NAMESPACE: u32 = 0x0043_c140;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ADD_TO_NAMESPACE, Conv, DYNAMIC_MESH_INSTANCE_CTOR, Host, PHYRE_FREE, REGISTER_TYPE,
    };

    /// A scalar deleting destructor: `*this = vtable` (an `ffx.sym`
    /// address) when given, the base destructor `base` when given, then
    /// `Phyre_PhyreFree(this)` when `flags & 1`. Returns `this`.
    pub fn scalar_deleting_dtor(
        h: &mut impl Host,
        this: u32,
        flags: u32,
        vtable: Option<u32>,
        base: Option<u32>,
    ) -> u32 {
        if let Some(vt) = vtable {
            let vt = h.rebase(vt);
            h.write(this, vt);
        }
        if let Some(base) = base {
            let f = h.rebase(base);
            h.call(f, Conv::Thiscall(this), &[]);
        }
        if flags & 1 != 0 {
            let f = h.rebase(PHYRE_FREE);
            h.call(f, Conv::Cdecl, &[this]);
        }
        this
    }

    /// A dynamic-mesh instance constructor: `PDynamicMeshInstance(this,
    /// mesh)`, then `*this = vtable`. Returns `this`.
    pub fn construct_mesh_instance(h: &mut impl Host, this: u32, mesh: u32, vtable: u32) -> u32 {
        let f = h.rebase(DYNAMIC_MESH_INSTANCE_CTOR);
        h.call(f, Conv::Thiscall(this), &[mesh]);
        let vt = h.rebase(vtable);
        h.write(this, vt);
        this
    }

    /// A class registration: `RegisterType(desc)`, then
    /// `addToNamespace(desc)`, whose `eax` it returns. `desc` is the
    /// descriptor's `ffx.sym` address.
    pub fn register(h: &mut impl Host, desc: u32) -> u32 {
        let desc = h.rebase(desc);
        let f = h.rebase(REGISTER_TYPE);
        h.call(f, Conv::Cdecl, &[desc]);
        let f = h.rebase(ADD_TO_NAMESPACE);
        h.call(f, Conv::Thiscall(desc), &[])
    }
}
