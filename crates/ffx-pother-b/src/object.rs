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
/// `0x0043cbb0` (thiscall `(desc)`): a class descriptor's default object
/// (building its default buffer first when needed), or 0.
pub const DESCRIPTOR_DEFAULT_OBJECT: u32 = 0x0043_cbb0;
/// `PDynamicMesh`'s constructor `0x004968c0` (thiscall `(this)`).
pub const DYNAMIC_MESH_CTOR: u32 = 0x0049_68c0;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ADD_TO_NAMESPACE, Conv, DESCRIPTOR_DEFAULT_OBJECT, DYNAMIC_MESH_CTOR,
        DYNAMIC_MESH_INSTANCE_CTOR, Host, PHYRE_FREE, REGISTER_TYPE,
    };

    /// A dynamic mesh's `constructInstance(obj)` (`Concrete` descriptor):
    /// for a non-null `obj`, `PDynamicMesh(obj)` and `*obj = vtable`.
    pub fn construct_mesh(h: &mut impl Host, obj: u32, vtable: u32) -> u8 {
        if obj != 0 {
            let f = h.rebase(DYNAMIC_MESH_CTOR);
            h.call(f, Conv::Thiscall(obj), &[]);
            let vt = h.rebase(vtable);
            h.write(obj, vt);
        }
        1
    }

    /// A mesh instance's `constructInstance(obj)` (`WithoutDefaultConstructor`
    /// descriptor): the mesh is the default object of the mesh class's
    /// descriptor `mesh_desc` (an `ffx.sym` address) less 4; without one,
    /// 0. Else, for a non-null `obj`, `PDynamicMeshInstance(obj, mesh)` and
    /// `*obj = vtable`, and 1.
    pub fn construct_mesh_instance_default(
        h: &mut impl Host,
        obj: u32,
        mesh_desc: u32,
        vtable: u32,
    ) -> u8 {
        let desc = h.rebase(mesh_desc);
        let f = h.rebase(DESCRIPTOR_DEFAULT_OBJECT);
        let object = h.call(f, Conv::Thiscall(desc), &[]);
        if object == 0 || object == 4 {
            return 0;
        }
        let mesh = object - 4;
        if obj != 0 {
            let f = h.rebase(DYNAMIC_MESH_INSTANCE_CTOR);
            h.call(f, Conv::Thiscall(obj), &[mesh]);
            let vt = h.rebase(vtable);
            h.write(obj, vt);
        }
        1
    }

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
