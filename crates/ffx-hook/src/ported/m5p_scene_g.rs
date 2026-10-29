//! M5 stage 1, list `scene`, batch 7 (`ffx_prender::m5p_scene_g`): the
//! fly controller's integration steps, the context switch's default state
//! and the attachable component's joint target. Parity tests:
//! `crates/ffx-oracle/tests/m5p_scene.rs` (`m5p_scene_g_*`).

use ffx_prender::m5p_scene_g::bridge as b;

use super::{StackFrame, mem, ported};

ported! {
    /// `Phyre_PRendering_PContextSwitch_defaultState_004d4d50`: the default state (two one-word pairs, 0 and 1); 0 or 0xd.
    Phyre_PRendering_PContextSwitch_defaultState_004d4d50 @ 0x004d_4d50 => extern "thiscall" fn sg_004d4d50(_this: u32, _unused: u32, state: u32) -> u32 {
        b::switch_default_state(&mut mem(), state)
    }
    /// `Phyre_PGameplay_PAttachableComponent_setJoint_0055a980`: the joint (or node) target and the name copy set; 0, 0x13 or 0xd.
    Phyre_PGameplay_PAttachableComponent_setJoint_0055a980 @ 0x0055_a980 => extern "thiscall" fn sg_0055a980(this: u32, skeleton: u32, name: u32) -> u32 {
        b::attach_joint(&mut mem(), this, skeleton, name)
    }
    /// `Phyre_PWorldMatrixFlyController_moveForward_005d7190`: moved back along the forward axis by `dt`.
    Phyre_PWorldMatrixFlyController_moveForward_005d7190 @ 0x005d_7190 => extern "thiscall" fn sg_005d7190(this: u32, dt: f32) -> u32 {
        let mut frame = StackFrame::<16>::new();
        b::fly_forward(&mut mem(), this, dt, frame.at())
    }
    /// `Phyre_PWorldMatrixFlyController_strafe_005d72b0`: moved by `row0 * a + row1 * b`.
    Phyre_PWorldMatrixFlyController_strafe_005d72b0 @ 0x005d_72b0 => extern "thiscall" fn sg_005d72b0(this: u32, a: f32, b2: f32) -> u32 {
        let mut frame = StackFrame::<16>::new();
        b::fly_strafe(&mut mem(), this, a, b2, frame.at())
    }
}
