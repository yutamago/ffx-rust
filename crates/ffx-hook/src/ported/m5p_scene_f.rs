//! M5 stage 1, list `scene`, batch 6 (`ffx_prender::m5p_scene_f`): the
//! gameplay maths, the scene node search by name, the attachable
//! component's target, the fly controller's matrix set and the animation
//! utility's time span. Parity tests: `crates/ffx-oracle/tests/m5p_scene.rs`
//! (`m5p_scene_f_*`).

use ffx_prender::m5p_scene_f::bridge as b;

use super::{StackFrame, mem, ported};

ported! {
    /// `Phyre_PScene_PUtilityScene_findNode_00504a00`: the last node of the queried name under the root into the query; 0.
    Phyre_PScene_PUtilityScene_findNode_00504a00 @ 0x0050_4a00 => extern "C" fn sf_00504a00(query: u32, root: u32) -> u32 {
        b::find_node(&mut mem(), query, root)
    }
    /// `Phyre_PScene_PNode_findNamed_00506b80`: the node of a name under this one, or 0.
    Phyre_PScene_PNode_findNamed_00506b80 @ 0x0050_6b80 => extern "thiscall" fn sf_00506b80(this: u32, name: u32) -> u32 {
        b::find_node_named(&mem(), this, name)
    }
    /// `Phyre_PAnimation_PAnimationClip_timeSpan_005168b0`: end - start of the time range, or 0 (ST0).
    Phyre_PAnimation_PAnimationClip_timeSpan_005168b0 @ 0x0051_68b0 => extern "thiscall" fn sf_005168b0(this: u32) -> f32 {
        let mut frame = StackFrame::<2>::new();
        b::time_span(&mut mem(), this, frame.at())
    }
    /// `Phyre_PGameplay_rotationProduct_00547870`: the 3x3 product into `out`; `out`.
    Phyre_PGameplay_rotationProduct_00547870 @ 0x0054_7870 => extern "thiscall" fn sf_00547870(this: u32, out: u32, b2: u32) -> u32 {
        b::rotation_product(&mut mem(), this, out, b2)
    }
    /// `Phyre_PGameplay_PAttachableComponent_setTarget_0055ab30`: the target node and the name copy set; 0, 0x13 or 0xd.
    Phyre_PGameplay_PAttachableComponent_setTarget_0055ab30 @ 0x0055_ab30 => extern "thiscall" fn sf_0055ab30(this: u32, node: u32, name: u32) -> u32 {
        b::attach_target(&mut mem(), this, node, name)
    }
    /// `Phyre_PGameplay_polygonArea_005a3af0`: twice the polygon's signed area (ST0).
    Phyre_PGameplay_polygonArea_005a3af0 @ 0x005a_3af0 => extern "C" fn sf_005a3af0(points: u32, n: u32) -> f32 {
        b::polygon_area(&mem(), points, n)
    }
    /// `Phyre_PWorldMatrixFlyController_setMatrix_005d77a0`: the 4x4 and the vector set, the 3x4 rebuilt.
    Phyre_PWorldMatrixFlyController_setMatrix_005d77a0 @ 0x005d_77a0 => extern "thiscall" fn sf_005d77a0(this: u32, v: u32) -> u32 {
        let mut frame = StackFrame::<16>::new();
        b::fly_set_matrix(&mut mem(), this, v, frame.at())
    }
}
