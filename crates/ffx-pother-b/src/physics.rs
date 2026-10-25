//! The `PPhysics` class descriptors' instance callbacks
//! (`0x005f83b0`..`0x005f8e60`): slot `+0x28` (`constructInstance`) and
//! slot `+0x2c` (`destructInstance`, name ours: the slot after
//! `constructInstance`) of `PClassDescriptorForType<T>` and its
//! `Abstract`/`Concrete`/`WithoutDefaultConstructor` copies, which
//! `Phyre_PClassDescriptor_calculateDefaultBuffer` (`0x0043c420`) and
//! `Phyre_PClassDescriptor_initializeInstance` (`0x0043ce00`) call on an
//! instance in place. Thiscall on the descriptor, which none of them reads,
//! with the instance as the one stack word (`ret 4`); each returns `al = 1`
//! (callers drop it: `calculateDefaultBuffer`, `initializeInstance`, and
//! `0x004495b0`, which passes it on to the same callers).
//!
//! The instance destructors are the type's own: a virtual destructor
//! (`vtable[0]` or `vtable[3]`, flag 0: destroy, don't free) on the object
//! the instance pointer belongs to (`obj - 4`, its primary base; a null
//! instance makes `this` null, and the original then reads address 0), a
//! non-virtual one called with `this = obj`, or inline field releases.
//!
//! The SEH frames some of them set up (state words around a call that
//! never throws) are not reproduced.

use ffx_graphic::host::{Conv, Host};

use crate::PHYRE_FREE;

/// `Phyre_PBase_GetClassDescriptor` (`() -> 0x00c902e8`).
pub const PBASE_CLASS_DESCRIPTOR: u32 = 0x0043_5740;
/// The character-controller release `0x006016c0` (thiscall `(ctrl,
/// owner)`, `ret 4`) that `PPhysicsCharacterController`'s destructor calls
/// on its `+0xcc` controller.
pub const CONTROLLER_RELEASE: u32 = 0x0060_16c0;
/// The getter `0x009c33e0` (thiscall: `[this + 8]`).
pub const FIELD_8: u32 = 0x009c_33e0;
/// `PPhysicsRigidBodyBase`'s destructor `0x005e2660` (thiscall, no stack
/// words).
pub const RIGID_BODY_BASE_DTOR: u32 = 0x005e_2660;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        CONTROLLER_RELEASE, Conv, FIELD_8, Host, PBASE_CLASS_DESCRIPTOR, PHYRE_FREE,
        RIGID_BODY_BASE_DTOR,
    };

    /// `vtable[slot](this, flag)`, thiscall.
    fn virtual_call(h: &mut impl Host, this: u32, slot: u32, flag: u32) {
        h.call_virtual(this, slot, &[flag]);
    }

    /// `destructInstance(obj)` of a type with a virtual destructor at
    /// `slot` (0 or 3): `vtable[slot](obj - 4, 0)`; a null `obj` gives a
    /// null `this` (the original reads its vtable at 0, as this does).
    pub fn destruct_virtual(h: &mut impl Host, obj: u32, slot: u32) -> u8 {
        let this = if obj == 0 { 0 } else { obj.wrapping_sub(4) };
        virtual_call(h, this, slot, 0);
        1
    }

    /// `destructInstance(obj)` of a type with nothing to destroy.
    #[must_use]
    pub fn destruct_nothing() -> u8 {
        1
    }

    /// `destructInstance(obj)` of a type whose destructor `dtor` (thiscall,
    /// no stack words) is called with `this = obj`.
    pub fn destruct_with(h: &mut impl Host, obj: u32, dtor: u32) -> u8 {
        let f = h.rebase(dtor);
        h.call(f, Conv::Thiscall(obj), &[]);
        1
    }

    /// Frees the buffer `[at]` unless its low bit (the "not owned" flag) is
    /// set, as the inline destructors of a `+0x100` buffer do.
    fn free_unless_flagged(h: &mut impl Host, at: u32) {
        let word: u32 = h.read(at);
        if word & 1 == 0 {
            let f = h.rebase(PHYRE_FREE);
            h.call(f, Conv::Cdecl, &[word]);
        }
    }

    /// `PPhysicsCharacterControllerBase`'s `destructInstance(obj)`
    /// (`0x005f8830`): frees `obj + 0x100` unless flagged.
    pub fn destruct_controller_base(h: &mut impl Host, obj: u32) -> u8 {
        free_unless_flagged(h, obj + 0x100);
        1
    }

    /// `PSharray<PPhysicsShape>`'s `destructInstance(arr)` (`0x005f8600`):
    /// the header word `[arr]` holds the count in its low 31 bits and a
    /// "storage not owned" flag in bit 31; an owned array of more than one
    /// element frees its storage `[arr + 4]`. Both words are then cleared.
    pub fn destruct_shape_array(h: &mut impl Host, arr: u32) -> u8 {
        let header: u32 = h.read(arr);
        if header >> 31 == 0 && header & 0x7fff_ffff > 1 {
            let data: u32 = h.read(arr + 4);
            let f = h.rebase(PHYRE_FREE);
            h.call(f, Conv::Cdecl, &[data]);
        }
        h.write(arr + 4, 0u32);
        h.write(arr, 0u32);
        1
    }

    /// `PPhysicsInterfaceBullet`'s `destructInstance(obj)` (`0x005f89c0`):
    /// on `p = obj - 1` (0 for a null `obj`), an array at `p + 0x1c`
    /// (header) / `p + 0x20` (storage): the storage is freed when owned
    /// (bit 31 clear) and not null, then both words are cleared.
    pub fn destruct_interface_bullet(h: &mut impl Host, obj: u32) -> u8 {
        let p = if obj == 0 { 0 } else { obj.wrapping_sub(1) };
        let header: u32 = h.read(p.wrapping_add(0x1c));
        if header >> 31 == 0 {
            let data: u32 = h.read(p.wrapping_add(0x20));
            if data != 0 {
                let f = h.rebase(PHYRE_FREE);
                h.call(f, Conv::Cdecl, &[data]);
            }
        }
        h.write(p.wrapping_add(0x20), 0u32);
        h.write(p.wrapping_add(0x1c), 0u32);
        1
    }

    /// `PPhysicsCharacterControllerBullet`'s release `0x005f8e60`
    /// (thiscall `(obj)`): when it has both a world (`+0x10c`) and a
    /// controller (`+0x110`), the world's `vtable[15](controller)` and
    /// `vtable[8](controller's [+8])`; then the virtual destructors
    /// (flag 1: free) of `+0x118` (slot 0), `+0x110` (slot 0) and `+0x114`
    /// (slot 1) when set, and the three words cleared.
    pub fn controller_bullet_release(h: &mut impl Host, obj: u32) {
        let controller: u32 = h.read(obj + 0x110);
        if controller != 0 {
            let world: u32 = h.read(obj + 0x10c);
            if world != 0 {
                h.call_virtual(world, 15, &[controller]);
                // the original reads the vtable before the getter, the
                // object after it
                let world: u32 = h.read(obj + 0x10c);
                let vtable: u32 = h.read(world);
                let controller: u32 = h.read(obj + 0x110);
                let f = h.rebase(FIELD_8);
                let inner = h.call(f, Conv::Thiscall(controller), &[]);
                let world: u32 = h.read(obj + 0x10c);
                let m: u32 = h.read(vtable + 4 * 8);
                h.call(m, Conv::Thiscall(world), &[inner]);
            }
        }
        for (at, slot) in [(0x118, 0), (0x110, 0), (0x114, 1)] {
            let p: u32 = h.read(obj + at);
            if p != 0 {
                virtual_call(h, p, slot, 1);
            }
        }
        h.write(obj + 0x118, 0u32);
        h.write(obj + 0x110, 0u32);
        h.write(obj + 0x114, 0u32);
    }

    /// `PPhysicsCharacterController`'s `destructInstance(obj)`
    /// (`0x005f87a0`): its controller `+0xcc`'s release
    /// (`0x006016c0(ctrl, obj)`) when set, then
    /// [`controller_bullet_release`] and the `+0x100` buffer.
    pub fn destruct_controller(h: &mut impl Host, obj: u32) -> u8 {
        let ctrl: u32 = h.read(obj + 0xcc);
        if ctrl != 0 {
            let f = h.rebase(CONTROLLER_RELEASE);
            h.call(f, Conv::Thiscall(ctrl), &[obj]);
        }
        controller_bullet_release(h, obj);
        free_unless_flagged(h, obj + 0x100);
        1
    }

    /// `PPhysicsCharacterControllerBullet`'s `destructInstance(obj)`
    /// (`0x005f8860`): [`controller_bullet_release`] and the `+0x100`
    /// buffer.
    pub fn destruct_controller_bullet(h: &mut impl Host, obj: u32) -> u8 {
        controller_bullet_release(h, obj);
        free_unless_flagged(h, obj + 0x100);
        1
    }

    /// `PPhysicsRigidBodyBullet`'s `destructInstance(obj)` (`0x005f8ba0`):
    /// the virtual destructors (flag 1) of `+0xdc` (slot 1) and `+0xe0`
    /// (slot 0) when set, `+0xe0`, `+0xdc` and `+0xb0` cleared, then
    /// `PPhysicsRigidBodyBase`'s destructor.
    pub fn destruct_rigid_body_bullet(h: &mut impl Host, obj: u32) -> u8 {
        for (at, slot) in [(0xdc, 1), (0xe0, 0)] {
            let p: u32 = h.read(obj + at);
            if p != 0 {
                virtual_call(h, p, slot, 1);
            }
        }
        h.write(obj + 0xe0, 0u32);
        h.write(obj + 0xdc, 0u32);
        h.write(obj + 0xb0, 0u32);
        let f = h.rebase(RIGID_BODY_BASE_DTOR);
        h.call(f, Conv::Thiscall(obj), &[]);
        1
    }

    /// `PPhysicsCallbackData`'s `constructInstance(obj)` (`0x005f83b0`):
    /// for a non-null `obj`, `[+0] = 0`, `[+4] = PBase`'s class descriptor,
    /// `[+8] = 0`, `[+0xc]` the descriptor again, `[+0x10] = 0.0`.
    pub fn construct_callback_data(h: &mut impl Host, obj: u32) -> u8 {
        if obj != 0 {
            h.write(obj, 0u32);
            let f = h.rebase(PBASE_CLASS_DESCRIPTOR);
            let desc = h.call(f, Conv::Cdecl, &[]);
            h.write(obj + 4, desc);
            h.write(obj + 8, 0u32);
            let desc = h.call(f, Conv::Cdecl, &[]);
            h.write(obj + 0x10, 0f32);
            h.write(obj + 0xc, desc);
        }
        1
    }
}
