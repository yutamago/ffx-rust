//! `FreeCamera`: the free camera (the PC build's debug fly camera), a
//! singleton at [`INSTANCE`] built on Phyre engine cameras. The engine's
//! constructors and destructors stay the original's; the ports call them
//! through a callback (`call(mem, function, argument, thiscall)`: the
//! argument in `ecx` when `thiscall`, else on the stack).

use ffx_bridge::Global;

/// The free camera (`0x344` bytes), created on first use by its callers
/// and destroyed by `FreeCamera_destroyInstance`.
pub const INSTANCE: Global<u32> = Global::new(0x00cc_c830);
/// `Phyre::PCameraPerspective`'s constructor (thiscall).
pub const PERSPECTIVE_CTOR: u32 = 0x0045_6370;
/// `Phyre::PWorldMatrixFlyController`'s constructor (thiscall).
pub const FLY_CTOR: u32 = 0x005d_6cc0;
/// `Phyre::PWorldMatrixFlyController`'s destructor (thiscall).
pub const FLY_DTOR: u32 = 0x005d_6d10;
/// `Phyre::PhyreFree` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `operator delete` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `Phyre::PCameraProjection`'s vtable, which the destructor restores.
pub const PROJECTION_VTABLE: u32 = 0x00b1_0f5c;
/// 60.0 (`f32`, `.rdata`): the camera's `+8` (its speed).
pub const DEFAULT_SPEED: u32 = 0x00b3_eb9c;

/// Offsets in the free camera.
pub mod layout {
    /// `+0xc`: the first `PCameraPerspective` (its buffer at `+0xdc`).
    pub const CAMERA_A: u32 = 0xc;
    /// `+0x164`: the second (its buffer at `+0x234`).
    pub const CAMERA_B: u32 = 0x164;
    /// `+0x2bc`: the fly controller.
    pub const FLY: u32 = 0x2bc;
}

/// The free camera where the game keeps it.
pub mod bridge {
    use ffx_bridge::Mem;

    use super::layout;
    use crate::x87;

    /// `FreeCamera::FreeCamera` (`0x0067fe70`, thiscall): the two cameras
    /// and the fly controller constructed, `+8` 60.0, `+0`/`+4` 0, the
    /// flags `+0x338`, `+0x339`, `+0x33a`, `+0x33c` cleared. Returns
    /// `this`. (The original links an exception frame around the
    /// constructors.)
    pub fn free_camera_ctor<M: Mem>(
        mem: &mut M,
        this: u32,
        mut call: impl FnMut(&mut M, u32, u32, bool),
    ) -> u32 {
        let ctor = mem.rebase(super::PERSPECTIVE_CTOR);
        call(mem, ctor, this.wrapping_add(layout::CAMERA_A), true);
        let ctor = mem.rebase(super::PERSPECTIVE_CTOR);
        call(mem, ctor, this.wrapping_add(layout::CAMERA_B), true);
        let fly = mem.rebase(super::FLY_CTOR);
        call(mem, fly, this.wrapping_add(layout::FLY), true);
        let at = mem.rebase(super::DEFAULT_SPEED);
        let speed = x87::copy(mem.read::<u32>(at));
        mem.write(this.wrapping_add(8), speed);
        mem.write(this, 0u32);
        mem.write(this.wrapping_add(4), 0u32);
        mem.write(this.wrapping_add(0x339), 0u16);
        mem.write(this.wrapping_add(0x338), 0u8);
        mem.write(this.wrapping_add(0x33c), 0u8);
        this
    }

    /// `FreeCamera::~FreeCamera` (`0x0067ff10`, thiscall): the fly
    /// controller destroyed, then each camera back to
    /// `PCameraProjection`, its buffer freed (`PhyreFree`) unless bit 0 of
    /// the pointer marks it borrowed: the second camera first.
    pub fn free_camera_dtor<M: Mem>(
        mem: &mut M,
        this: u32,
        mut call: impl FnMut(&mut M, u32, u32, bool),
    ) {
        let fly = mem.rebase(super::FLY_DTOR);
        call(mem, fly, this.wrapping_add(layout::FLY), true);
        for (camera, buffer) in [(layout::CAMERA_B, 0x234u32), (layout::CAMERA_A, 0xdc)] {
            let vt = mem.rebase(super::PROJECTION_VTABLE);
            mem.write(this.wrapping_add(camera), vt);
            let p = mem.read::<u32>(this.wrapping_add(buffer));
            if !p & 1 != 0 {
                let free = mem.rebase(super::PHYRE_FREE);
                call(mem, free, p, false);
            }
        }
    }

    /// The scalar deleting destructor (`0x0067ffb0`, thiscall with the
    /// flags): [`free_camera_dtor`], then `operator delete` with flag
    /// bit 0. Returns `this`.
    pub fn free_camera_delete<M: Mem>(
        mem: &mut M,
        this: u32,
        flags: u32,
        mut call: impl FnMut(&mut M, u32, u32, bool),
    ) -> u32 {
        free_camera_dtor(mem, this, &mut call);
        if flags & 1 != 0 {
            let delete = mem.rebase(super::OPERATOR_DELETE);
            call(mem, delete, this, false);
        }
        this
    }

    /// `FreeCamera_destroyInstance()` (`0x00680000`): the free camera,
    /// when there is one, destroyed and deleted, the instance cleared.
    pub fn free_camera_destroy_instance<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, u32, bool),
    ) {
        let cam = super::INSTANCE.get(mem);
        if cam != 0 {
            free_camera_dtor(mem, cam, &mut call);
            let delete = mem.rebase(super::OPERATOR_DELETE);
            call(mem, delete, cam, false);
            super::INSTANCE.set(mem, 0);
        }
    }
}
