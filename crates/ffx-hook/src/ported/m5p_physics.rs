//! M5 stage 1, list `phyre-physics` (`ffx_pother_b::m5p_physics`): the
//! physics code the game runs: `PUtilityPhysics`'s `initialize` and
//! destructor, and the world/model/body unlink helpers the physics
//! destructors call. The rest of the list is dropped
//! (`symbols/m5-dropped.tsv`); slot 2, under five bytes, is in
//! `m5p_physics_stubs`.
//!
//! Threads: `initialize` runs in `Phyre_PhyreInit` on the main thread; the
//! destructors on the thread that loads or frees the objects. No state.

#[allow(unused_imports)]
use super::{call_cdecl, mem, ported};
use ffx_pload::m5p_registry_load_a_misc::bridge as msb;
use ffx_pother_b::m5p_physics::{self as ph, bridge as b};

ported! {
    /// `PUtilityPhysics`'s scalar deleting destructor (`this`, flags) -> `this`.
    Phyre_PPhysics_PUtilityPhysics_deletingDtor @ 0x005e_7250 => extern "thiscall" fn ph_005e7250(this: u32, flags: u32) -> u32 {
        msb::delete_utility(&mut mem(), this, flags)
    }
    /// `PUtilityPhysics::initialize()` -> 0: the start-up objects, then the physics types' binds and declarations.
    Phyre_PPhysics_PUtilityPhysics_initialize @ 0x005f_fe40 => extern "C" fn ph_005ffe40() -> u32 {
        b::utility_initialize(&mut mem(), ph::INIT_STEPS)
    }
    /// `0x006016c0(world, controller)` -> 0: the controller off the world (and its dynamics world).
    Phyre_PPhysics_PPhysicsWorld_removeController_006016c0 @ 0x0060_16c0 => extern "thiscall" fn ph_006016c0(world: u32, ctrl: u32) -> u32 {
        b::world_remove_controller(&mut mem(), world, ctrl)
    }
    /// `0x006018b0(world, model)` -> 0: the model and its bodies off the world.
    Phyre_PPhysics_PPhysicsWorld_removeModel_006018b0 @ 0x0060_18b0 => extern "thiscall" fn ph_006018b0(world: u32, model: u32) -> u32 {
        b::world_remove_model(&mut mem(), world, model)
    }
    /// `0x00601a20(body, &head)` -> body or 0: the body off a world's body list.
    Phyre_PPhysics_PPhysicsRigidBody_unlinkFromWorld_00601a20 @ 0x0060_1a20 => extern "thiscall" fn ph_00601a20(body: u32, head: u32) -> u32 {
        b::unlink_world_body(&mut mem(), body, head)
    }
    /// `0x00601a60(model, body)` -> 0: the body off the model.
    Phyre_PPhysics_PPhysicsModel_removeBody_00601a60 @ 0x0060_1a60 => extern "thiscall" fn ph_00601a60(model: u32, body: u32) -> u32 {
        b::model_remove_body(&mut mem(), model, body)
    }
}
