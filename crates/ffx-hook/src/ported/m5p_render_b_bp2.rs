//! M5 stage 1, list `phyre-render-b`, round big-phyre2
//! (`ffx_prender::m5p_render_b_bp2`). Parity tests:
//! `crates/ffx-oracle/tests/m5p_render_b.rs` (`bp2_*`). Render-thread
//! code: the frames are per call.

use ffx_prender::m5p_render_b_bp2::{self as rb, bridge as b};

use super::{StackFrame, mem, ported};

ported! {
    /// `Phyre_PPostProcessing_PGlow_renderMesh_00a139c0(effect; renderer, texture)`:
    /// the surface and the texture binding in a frame.
    Phyre_PPostProcessing_PGlow_renderMesh_00a139c0 @ 0x00a1_39c0 => extern "thiscall" fn rbp2_00a139c0(this: u32, renderer: u32, texture: u32) -> u32 {
        let mut f = StackFrame::<{ rb::GLOW_MESH_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * rb::GLOW_MESH_FRAME_WORDS;
        b::glow_render_mesh(&mut mem(), ebp, this, renderer, texture)
    }
    /// `Phyre_PRendering_PRendererBase_queueSegments_005b55a0(renderer; job, context)`:
    /// the sizes `meshInstanceSizes` writes in a frame.
    Phyre_PRendering_PRendererBase_queueSegments_005b55a0 @ 0x005b_55a0 => extern "thiscall" fn rbp2_005b55a0(this: u32, job: u32, ctx: u32) -> u32 {
        let mut f = StackFrame::<{ rb::QUEUE_SEGMENTS_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * rb::QUEUE_SEGMENTS_FRAME_WORDS;
        b::queue_segments(&mut mem(), ebp, this, job, ctx)
    }
    /// `PRendererD3D11_drawPoints_00591dd0(renderer; primitive, points, colour, indices, n, m)`:
    /// the matrix, the colour and the mapped resource in a frame.
    PRendererD3D11_drawPoints_00591dd0 @ 0x0059_1dd0 => extern "thiscall" fn rbp2_00591dd0(this: u32, primitive: u32, points: u32, colour: u32, indices: u32, n: u32, m: u32) -> u32 {
        let mut f = StackFrame::<{ rb::DRAW_POINTS_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * rb::DRAW_POINTS_FRAME_WORDS;
        rb::points::draw_points(&mut mem(), ebp, this, primitive, points, colour, indices, n, m)
    }
    /// `Phyre_PPostProcessing_PMotionBlur_setupMatrices_00a25410(effect; camera)`:
    /// the matrices and products in a frame; `PMatrix4_Copy`'s `eax`.
    Phyre_PPostProcessing_PMotionBlur_setupMatrices_00a25410 @ 0x00a2_5410 => extern "thiscall" fn rbp2_00a25410(this: u32, camera: u32) -> u32 {
        let mut f = StackFrame::<{ rb::BLUR_MATRICES_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * rb::BLUR_MATRICES_FRAME_WORDS;
        rb::blur_matrices::blur_matrices(&mut mem(), ebp, this, camera)
    }
    /// `Phyre_PPostProcessing_PMotionBlur_cameraHistory_00a13b90(effect; camera)`:
    /// the matrices and products in a frame; the last `PMatrix4_Copy`'s `eax`.
    Phyre_PPostProcessing_PMotionBlur_cameraHistory_00a13b90 @ 0x00a1_3b90 => extern "thiscall" fn rbp2_00a13b90(this: u32, camera: u32) -> u32 {
        let mut f = StackFrame::<{ rb::HISTORY_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * rb::HISTORY_FRAME_WORDS;
        rb::history::camera_history(&mut mem(), ebp, this, camera)
    }
    /// `Phyre_PPostProcessing_PDepthOfField_renderPass_00a1ca10(effect; renderer, a, near, far, out)`:
    /// the surfaces and bindings in a frame; the first error or the last call's `eax`.
    Phyre_PPostProcessing_PDepthOfField_renderPass_00a1ca10 @ 0x00a1_ca10 => extern "thiscall" fn rbp2_00a1ca10(this: u32, renderer: u32, a: u32, near: u32, far: u32, out: u32) -> u32 {
        let mut f = StackFrame::<{ rb::DOF_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * rb::DOF_FRAME_WORDS;
        rb::dof::dof_render_pass(&mut mem(), ebp, this, renderer, a, near, far, out)
    }
}
