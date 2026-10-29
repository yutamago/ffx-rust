//! M5 stage 1, list `scene`, batch 8 (`ffx_prender::m5p_scene_h`): the
//! render pass array's copy construction, the rendering utility's
//! resource teardown and the animation's time range. Parity tests:
//! `crates/ffx-oracle/tests/m5p_scene.rs` (`m5p_scene_h_*`).

use ffx_prender::m5p_scene_h::bridge as b;

use super::{StackFrame, mem, ported};

ported! {
    /// `Phyre_PSceneRenderPass_copyConstructItems_004e1ae0`: render passes copy-constructed.
    Phyre_PSceneRenderPass_copyConstructItems_004e1ae0 @ 0x004e_1ae0 => extern "C" fn sh_004e1ae0(dst: u32, src: u32, n: u32) {
        b::render_passes_copy(&mut mem(), dst, src, n);
    }
    /// `Phyre_PRendering_PUtilityRendering_releaseResources_004fdca0`: the utility's buffers released.
    Phyre_PRendering_PUtilityRendering_releaseResources_004fdca0 @ 0x004f_dca0 => extern "C" fn sh_004fdca0() -> u32 {
        b::render_resources_release(&mut mem())
    }
    /// `Phyre_PAnimation_PAnimationClip_timeRange_00516a20`: the union of the clips' ranges and its own; 0, 7 or a clip's error.
    Phyre_PAnimation_PAnimationClip_timeRange_00516a20 @ 0x0051_6a20 => extern "thiscall" fn sh_00516a20(this: u32, start: u32, end: u32) -> u32 {
        let mut frame = StackFrame::<2>::new();
        b::time_range(&mut mem(), this, start, end, frame.at())
    }
}
