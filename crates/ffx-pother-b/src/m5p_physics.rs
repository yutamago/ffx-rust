//! M5 stage 1, list `phyre-physics`: the physics code the game runs.
//!
//! Of the 141 functions on the list, these run (or can run) in the game as
//! it is; the rest are reachable only through the script binding or through
//! physics objects nothing outside it creates, and are dropped
//! (`symbols/m5-dropped.tsv`):
//!
//! - `PUtilityPhysics` (vtable `0x00b39384`), a static global on the
//!   utility list `0x00c901a8`: `initialize` (slot 1, which
//!   `Phyre_PhyreInit` calls on every utility), slot 2 (`Phyre_PhyreExit`;
//!   `xor eax, eax; ret`, a host-only table entry) and its scalar deleting
//!   destructor (slot 0, the same code as `PUtility`'s, ported in
//!   `ffx_pload::m5p_registry_load_a_misc`);
//! - the world/model/body unlink helpers that the `PPhysicsWorld`,
//!   `PPhysicsModel`, `PPhysicsRigidBody` and `PPhysicsCharacterController`
//!   destructors call. Those destructors run at start-up on every
//!   descriptor's default buffer (`Phyre_PClassDescriptor_calculateDefaultBuffer`),
//!   where the guarding pointer fields are 0, so the helpers never ran in a
//!   coverage session; they run once a script attaches a body to a world.
//!
//! The lists are singly linked: a world's controllers at `+0x4c` and its
//! models at `+0x48` through each node's `+0`, its rigid bodies at `+0x50`
//! through each body's `+0xb8`; a model's bodies at `+4` through `+0`. A
//! body's `+0xb0` is its model, `+0xdc` its Bullet body; a world's `+0x54`
//! is its Bullet dynamics world; a controller's `+0x10c` the dynamics world
//! it was added to, with its Bullet action at `+0x110` and its ghost object
//! at `+0x114`.

use ffx_graphic::host::{Conv, Host};

/// The `PUtilityPhysics` global (on the utility list).
pub const UTILITY_PHYSICS: u32 = 0x00c2_a0f4;
/// The byte that stops `initialize`'s start-up objects (the rendering
/// utility's `initialize` tests it too).
pub const STARTED: u32 = 0x00c0_a09c;
/// `initialize`'s start-up objects: virtual method 1 of each while
/// [`STARTED`] stays clear (the first is `PUtilityPhysics` itself).
pub const STARTUP_OBJECTS: [u32; 3] = [UTILITY_PHYSICS, 0x00c0_a33c, 0x00c0_a3c0];
/// `Phyre_PTypeDispenser_RegisterType` (cdecl `(type)`).
pub const REGISTER_TYPE: u32 = 0x0043_95b0;
/// `Phyre_PClassDescriptor_addToNamespace` (thiscall, no stack words).
pub const ADD_TO_NAMESPACE: u32 = 0x0043_c140;

/// One step of `PUtilityPhysics::initialize`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// `f()` (cdecl): a class's `Bind`.
    Call(u32),
    /// `RegisterType(d)`, then `d->addToNamespace()`: a class descriptor
    /// global `d`.
    Declare(u32),
}

/// `PUtilityPhysics::initialize`'s steps after the start-up objects, in
/// the original's order (`0x005ffe82`..`0x006000c4`).
pub const INIT_STEPS: &[Step] = &[
    Step::Call(0x005e_7750), // PPhysicsBoxBase_Bind
    Step::Declare(0x00cb_f8b0),
    Step::Declare(0x00cb_e778),
    Step::Call(0x005e_7960), // PPhysicsCapsuleBase_Bind
    Step::Declare(0x00cb_f948),
    Step::Declare(0x00cb_e8a8),
    Step::Call(0x005e_8850), // PPhysicsCylinderBase_Bind
    Step::Declare(0x00cb_f9e0),
    Step::Declare(0x00cb_eba0),
    Step::Call(0x005e_89c0), // PPhysicsMaterial_Bind
    Step::Call(0x005e_8b10), // PPhysicsMeshBase_Bind
    Step::Declare(0x00cb_fa78),
    Step::Declare(0x00cb_ee98),
    Step::Call(0x005e_8bd0), // PPhysicsModel_Bind
    Step::Call(0x005e_8d20), // PPhysicsPlaneBase_Bind
    Step::Declare(0x00cb_fb10),
    Step::Declare(0x00cb_f060),
    Step::Call(0x005e_9f70), // PPhysicsShapeBase_Bind
    Step::Declare(0x00cb_f818),
    Step::Declare(0x00cb_f358),
    Step::Call(0x005e_a210), // PPhysicsSphereBase_Bind
    Step::Declare(0x00cb_fba8),
    Step::Declare(0x00cb_f488),
    Step::Call(0x005e_a2d0), // PPhysicsTaperedCapsule_Bind
    Step::Call(0x005e_a410), // PPhysicsTaperedCylinder_Bind
    Step::Call(0x005e_aab0), // PPhysicsWorldBase_Bind
    Step::Declare(0x00cb_fcd8),
    Step::Call(0x005e_a550), // PPhysicsWorld_Bind
    Step::Call(0x005e_9490), // PPhysicsRigidBodyBase_Bind
    Step::Declare(0x00cb_fc40),
    Step::Call(0x005e_8de0), // PPhysicsRigidBody_Bind
    Step::Declare(0x00cb_ec38),
    Step::Declare(0x00cb_fd70),
    Step::Declare(0x00cb_ecd0),
    Step::Call(0x005e_8020), // PPhysicsCharacterControllerBase_Bind
    Step::Declare(0x00cb_fe08),
    Step::Declare(0x00cb_ea70),
    Step::Call(0x005e_ad30), // PRaycastResult_Bind
    Step::Call(0x005e_7810), // PPhysicsCallbackData_Bind
    Step::Call(0x005f_09a0), // PRigidBodyBase PCollisionGroupBindDescriptor
    Step::Call(0x005f_0a40), // PRigidBodyBase PRigidBodyTypeBindDescriptor
];

/// Where a rigid body links into its world's body list.
const BODY_WORLD_LINK: u32 = 0xb8;
/// A rigid body's model.
const BODY_MODEL: u32 = 0xb0;
/// A rigid body's Bullet body.
const BODY_BULLET: u32 = 0xdc;
/// A model's world.
const MODEL_WORLD: u32 = 8;
/// A model's bodies.
const MODEL_BODIES: u32 = 4;
/// A world's models, rigid bodies, Bullet dynamics world and controllers.
const WORLD_MODELS: u32 = 0x48;
const WORLD_BODIES: u32 = 0x50;
const WORLD_DYNAMICS: u32 = 0x54;
const WORLD_CONTROLLERS: u32 = 0x4c;
/// A controller's dynamics world, action and ghost object.
const CONTROLLER_DYNAMICS: u32 = 0x10c;
const CONTROLLER_ACTION: u32 = 0x110;
const CONTROLLER_GHOST: u32 = 0x114;
/// `btDynamicsWorld` virtual methods (word indices): `removeRigidBody`
/// (`+0x50`), `removeAction` (`+0x3c`), `removeCollisionObject` (`+0x20`).
/// Bullet's names for those slots; thiscall, one stack word each.
const REMOVE_RIGID_BODY: u32 = 0x50 / 4;
const REMOVE_ACTION: u32 = 0x3c / 4;
const REMOVE_COLLISION_OBJECT: u32 = 0x20 / 4;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ADD_TO_NAMESPACE, BODY_BULLET, BODY_MODEL, BODY_WORLD_LINK, CONTROLLER_ACTION,
        CONTROLLER_DYNAMICS, CONTROLLER_GHOST, Conv, Host, MODEL_BODIES, MODEL_WORLD,
        REGISTER_TYPE, REMOVE_ACTION, REMOVE_COLLISION_OBJECT, REMOVE_RIGID_BODY, STARTED,
        STARTUP_OBJECTS, Step, WORLD_BODIES, WORLD_CONTROLLERS, WORLD_DYNAMICS, WORLD_MODELS,
    };

    /// `PUtilityPhysics::initialize` (`0x005ffe40`, slot 1; it never reads
    /// `ecx`) -> 0: virtual method 1 of each start-up object while the
    /// [`STARTED`] byte stays clear, then the `steps`
    /// ([`super::INIT_STEPS`]).
    pub fn utility_initialize(h: &mut impl Host, steps: &[Step]) -> u32 {
        let started = h.rebase(STARTED);
        for obj in STARTUP_OBJECTS {
            if h.read::<u8>(started) != 0 {
                break;
            }
            let obj = h.rebase(obj);
            h.call_virtual(obj, 1, &[]);
        }
        for &step in steps {
            match step {
                Step::Call(f) => {
                    h.call_sym(f, Conv::Cdecl, &[]);
                }
                Step::Declare(d) => {
                    let d = h.rebase(d);
                    h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[d]);
                    h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(d), &[]);
                }
            }
        }
        0
    }

    /// Takes `node` off the list whose first link is the word at `head`,
    /// the nodes linked through their word at `+next`: the link that held
    /// it gets its successor and its own link is cleared. Whether it was
    /// on the list.
    fn unlink(h: &mut impl Host, head: u32, next: u32, node: u32) -> bool {
        let mut link = head;
        loop {
            let cur: u32 = h.read(link);
            if cur == 0 {
                return false;
            }
            if cur == node {
                let after: u32 = h.read(cur + next);
                h.write(link, after);
                h.write(cur + next, 0u32);
                return true;
            }
            link = cur + next;
        }
    }

    /// Whether `node` is on the list at `head` (linked through `+0`).
    fn holds(h: &impl Host, head: u32, node: u32) -> bool {
        let mut cur: u32 = h.read(head);
        while cur != 0 {
            if cur == node {
                return true;
            }
            cur = h.read(cur);
        }
        false
    }

    /// `0x006016c0` (thiscall `(world, controller)`, `ret 4`) -> 0: a
    /// controller on the world's list is taken off it; when it was added
    /// to a dynamics world, its action and its ghost object are removed
    /// from that (`removeAction`, `removeCollisionObject`) and the pointer
    /// cleared. One not on the list is left alone.
    pub fn world_remove_controller(h: &mut impl Host, world: u32, ctrl: u32) -> u32 {
        let head = world + WORLD_CONTROLLERS;
        if !holds(h, head, ctrl) {
            return 0;
        }
        unlink(h, head, 0, ctrl);
        let dynamics: u32 = h.read(ctrl + CONTROLLER_DYNAMICS);
        if dynamics != 0 {
            let action: u32 = h.read(ctrl + CONTROLLER_ACTION);
            h.call_virtual(dynamics, REMOVE_ACTION, &[action]);
            // read again: the original does
            let dynamics: u32 = h.read(ctrl + CONTROLLER_DYNAMICS);
            let ghost: u32 = h.read(ctrl + CONTROLLER_GHOST);
            h.call_virtual(dynamics, REMOVE_COLLISION_OBJECT, &[ghost]);
            h.write(ctrl + CONTROLLER_DYNAMICS, 0u32);
        }
        0
    }

    /// `0x006018b0` (thiscall `(world, model)`, `ret 4`) -> 0: a model in
    /// a world ([`MODEL_WORLD`] set) and on this world's list is taken off
    /// it; each of its bodies is taken off the world's body list, removed
    /// from the Bullet world when the body's model's world shares this
    /// world's dynamics world and it has a Bullet body (`removeRigidBody`),
    /// and loses its model; then the model loses its world. A model not on
    /// the list keeps everything.
    pub fn world_remove_model(h: &mut impl Host, world: u32, model: u32) -> u32 {
        if h.read::<u32>(model + MODEL_WORLD) == 0 {
            return 0;
        }
        if !unlink(h, world + WORLD_MODELS, 0, model) {
            return 0;
        }
        let mut body: u32 = h.read(model + MODEL_BODIES);
        while body != 0 {
            let next: u32 = h.read(body);
            unlink(h, world + WORLD_BODIES, BODY_WORLD_LINK, body);
            let owner: u32 = h.read(body + BODY_MODEL);
            if owner != 0 {
                let owner_world: u32 = h.read(owner + MODEL_WORLD);
                let owner_dynamics: u32 = if owner_world == 0 {
                    0
                } else {
                    h.read(owner_world + WORLD_DYNAMICS)
                };
                let dynamics: u32 = h.read(world + WORLD_DYNAMICS);
                let bullet: u32 = h.read(body + BODY_BULLET);
                if dynamics != 0 && dynamics == owner_dynamics && bullet != 0 {
                    h.call_virtual(dynamics, REMOVE_RIGID_BODY, &[bullet]);
                }
            }
            h.write(body + BODY_MODEL, 0u32);
            body = next;
        }
        h.write(model + MODEL_WORLD, 0u32);
        0
    }

    /// `0x00601a20` (thiscall `(body, head)`, `ret 4`): `body` taken off
    /// the world body list at `head` (linked through [`BODY_WORLD_LINK`]);
    /// `eax` = `body` when it was on it, else 0.
    pub fn unlink_world_body(h: &mut impl Host, body: u32, head: u32) -> u32 {
        if unlink(h, head, BODY_WORLD_LINK, body) {
            body
        } else {
            0
        }
    }

    /// `0x00601a60` (thiscall `(model, body)`, `ret 4`) -> 0: `body`
    /// taken off the model's body list, and then loses its model.
    pub fn model_remove_body(h: &mut impl Host, model: u32, body: u32) -> u32 {
        if unlink(h, model + MODEL_BODIES, 0, body) {
            h.write(body + BODY_MODEL, 0u32);
        }
        0
    }
}
