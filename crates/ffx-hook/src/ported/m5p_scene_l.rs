//! M5 stage 1, list `scene`, batch 12 (`ffx_prender::m5p_scene_l`, and two
//! bridges of `m5p_scene_b`): the camera controller's look-at update, the
//! input mapper's device getter and the mesh instance's vector getter.
//! Parity tests: `crates/ffx-oracle/tests/m5p_scene.rs` (`m5p_scene_l_*`).

use ffx_prender::m5p_scene_b::bridge as sb;
use ffx_prender::m5p_scene_l::bridge as b;

use super::{StackFrame, mem, ported};

ported! {
    /// `Phyre_PRendering_PMeshInstance_getVector10_004f8100`: the vector at `+0x10` into `out`; `out`.
    Phyre_PRendering_PMeshInstance_getVector10_004f8100 @ 0x004f_8100 => extern "thiscall" fn sl_004f8100(this: u32, out: u32) -> u32 {
        sb::get_vector3(&mut mem(), this, out)
    }
    /// `Phyre_PGameplay_PCameraControllerComponent_follow_00558b10`: the look-at set from the followed node; the last call's eax (0 for none; off: 0, the original keeps its caller's).
    Phyre_PGameplay_PCameraControllerComponent_follow_00558b10 @ 0x0055_8b10 => extern "thiscall" fn sl_00558b10(this: u32, f: f32) -> u32 {
        let mut frame = StackFrame::<28>::new();
        b::camera_follow(&mut mem(), this, f, frame.at()).unwrap_or(0)
    }
    /// `Phyre_PInputMapper_getDevice4_00628700`: `PInput_GetDevice(4, n)`.
    Phyre_PInputMapper_getDevice4_00628700 @ 0x0062_8700 => extern "stdcall" fn sl_00628700(n: u32) -> u32 {
        sb::input_device4(&mut mem(), n)
    }
}
