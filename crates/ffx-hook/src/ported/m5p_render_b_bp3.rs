//! M5 stage 1, list `phyre-render-b`, round big-phyre3
//! (`ffx_prender::m5p_render_b_bp3`). Parity tests:
//! `crates/ffx-oracle/tests/m5p_render_b.rs` (`bp3_*`). Render-thread
//! code: the frames are per call.

use ffx_prender::m5p_render_b_bp3 as rb;

use super::{StackFrame, mem, ported};

ported! {
    /// `Phyre_PPostProcessing_PGlow_renderPasses_00a1d2b0(effect; renderer, out, texture, bloom, flag)`:
    /// the bindings, sizes, scales and surfaces in a frame.
    Phyre_PPostProcessing_PGlow_renderPasses_00a1d2b0 @ 0x00a1_d2b0 => extern "thiscall" fn rbp3_00a1d2b0(this: u32, renderer: u32, out: u32, texture: u32, bloom: u32, flag: u32) -> u32 {
        let mut f = StackFrame::<{ rb::GLOW_PASSES_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * rb::GLOW_PASSES_FRAME_WORDS;
        rb::glow::glow_render_passes(&mut mem(), ebp, this, renderer, out, texture, bloom, flag)
    }
    /// `Phyre_PPostProcessing_PDeferredLighting_uploadLights_00a127c0(effect; renderer, view)`:
    /// the matrices, vectors, ring cursors and bindings in a frame.
    Phyre_PPostProcessing_PDeferredLighting_uploadLights_00a127c0 @ 0x00a1_27c0 => extern "thiscall" fn rbp3_00a127c0(this: u32, renderer: u32, view: u32) -> u32 {
        let mut f = StackFrame::<{ rb::LIGHTS_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * rb::LIGHTS_FRAME_WORDS;
        rb::lights::upload_lights(&mut mem(), ebp, this, renderer, view)
    }
    /// `Phyre_PPostProcessing_PDeferredLighting_render_00a1f220(effect; renderer, view, out)`:
    /// the matrices, bindings, values and surfaces in a frame.
    Phyre_PPostProcessing_PDeferredLighting_render_00a1f220 @ 0x00a1_f220 => extern "thiscall" fn rbp3_00a1f220(this: u32, renderer: u32, view: u32, out: u32) -> u32 {
        let mut f = StackFrame::<{ rb::LIGHTING_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * rb::LIGHTING_FRAME_WORDS;
        rb::lighting::render(&mut mem(), ebp, this, renderer, view, out)
    }
    /// `Phyre_PPostProcessing_PDeferredLighting_drawLight_00a206d0(effect; renderer, light)`:
    /// the parameter values and matrices in a frame.
    Phyre_PPostProcessing_PDeferredLighting_drawLight_00a206d0 @ 0x00a2_06d0 => extern "thiscall" fn rbp3_00a206d0(this: u32, renderer: u32, light: u32) -> u32 {
        let mut f = StackFrame::<{ rb::LIGHT_DRAW_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * rb::LIGHT_DRAW_FRAME_WORDS;
        rb::light_draw::draw_light(&mut mem(), ebp, this, renderer, light)
    }
    /// `Phyre_PPostProcessing_PDeferredLighting_createCone_00a04b20(cluster, n)`:
    /// the two map slots in a frame.
    Phyre_PPostProcessing_PDeferredLighting_createCone_00a04b20 @ 0x00a0_4b20 => extern "C" fn rbp3_00a04b20(cluster: u32, n: u32) -> u32 {
        let mut f = StackFrame::<{ rb::CONE_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * rb::CONE_FRAME_WORDS;
        rb::cone::create_cone(&mut mem(), ebp, cluster, n)
    }
    /// `Phyre_PPostProcessing_PDeferredLighting_createSphere_00a06ab0(cluster, n)`:
    /// the two map slots in a frame.
    Phyre_PPostProcessing_PDeferredLighting_createSphere_00a06ab0 @ 0x00a0_6ab0 => extern "C" fn rbp3_00a06ab0(cluster: u32, n: u32) -> u32 {
        let mut f = StackFrame::<{ rb::SPHERE_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * rb::SPHERE_FRAME_WORDS;
        rb::cone::create_sphere(&mut mem(), ebp, cluster, n)
    }
}
