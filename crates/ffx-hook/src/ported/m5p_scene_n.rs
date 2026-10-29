//! M5 stage 1, list `scene`, batch 14 (`ffx_prender::m5p_scene_n`): the
//! visible bounds job's plane test of a point and a joint's world matrix. Parity tests:
//! `crates/ffx-oracle/tests/m5p_scene.rs` (`m5p_scene_n_*`).

use ffx_prender::m5p_scene_n::bridge as b;

use super::{StackFrame, mem, ported};

ported! {
    /// `Phyre_PRendering_PInternal_PCheckVisibleBoundsJob_planesBelow_009f2890`: which of four planes a point is below, one byte each.
    Phyre_PRendering_PInternal_PCheckVisibleBoundsJob_planesBelow_009f2890 @ 0x009f_2890 => extern "C" fn sn_009f2890(planes: u32, point: u32) -> u32 {
        b::planes_below(&mem(), planes, point)
    }
    /// `Phyre_PAnimation_PSkeletonInstance_jointWorld_004f71e0`: a joint's world matrix into `out`; `out`.
    Phyre_PAnimation_PSkeletonInstance_jointWorld_004f71e0 @ 0x004f_71e0 => extern "thiscall" fn sn_004f71e0(this: u32, out: u32, joint: u32) -> u32 {
        let mut frame = StackFrame::<51>::new();
        b::joint_world(&mut mem(), this, out, joint, frame.at())
    }
}
