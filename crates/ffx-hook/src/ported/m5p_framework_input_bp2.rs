//! M5 stage 1, list `framework-input`, round big-phyre2
//! (`ffx_pother_b::m5p_framework_input_bp2`). Parity tests:
//! `crates/ffx-oracle/tests/m5p_framework_input.rs` (`bp2_*`).
//! Render-thread code: the frames are per call.

use ffx_pother_b::m5p_framework_input_bp2::{self as fb, bridge as b};

use super::{StackFrame, mem, ported};

ported! {
    /// `Phyre_PPostProcessing_PMotionBlur_renderPass_00a1eb10(effect; renderer, target, a, velocity, colour)`:
    /// the surface and the texture bindings in a frame.
    Phyre_PPostProcessing_PMotionBlur_renderPass_00a1eb10 @ 0x00a1_eb10 => extern "thiscall" fn fbp2_00a1eb10(this: u32, renderer: u32, target: u32, a: u32, velocity: u32, colour: u32) -> u32 {
        let mut f = StackFrame::<{ fb::BLUR_PASS_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * fb::BLUR_PASS_FRAME_WORDS;
        b::blur_render_pass(&mut mem(), ebp, this, renderer, target, a, velocity, colour)
    }
}
