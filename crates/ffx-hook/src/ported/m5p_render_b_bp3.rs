//! M5 stage 1, list `phyre-render-b`, round big-phyre3
//! (`ffx_prender::m5p_render_b_bp3`). Parity tests:
//! `crates/ffx-oracle/tests/m5p_render_b.rs` (`bp3_*`). Render-thread
//! code: the frames are per call.

use ffx_prender::m5p_render_b_bp3 as rb;

use super::{StackFrame, mem, ported, with_frame};

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
    /// `Phyre_CharacterShaderManager_makeVariants_006dfad0(key, kind)`:
    /// the static object in a frame; the key's argument slot is handed to
    /// the trees and the second slot written, so the shim takes `slots`.
    Phyre_CharacterShaderManager_makeVariants_006dfad0 @ 0x006d_fad0 => extern "C" fn rbp3_006dfad0(key: u32, kind: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<{ rb::VARIANT_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * rb::VARIANT_FRAME_WORDS;
        rb::variants::make_variants(&mut mem(), ebp, slots, key, kind)
    }
    /// `Phyre_PRendering_PRendererBase_renderBatch_005b9ee0(renderer; batch, cursor, frame)`:
    /// the draw context and matrices in a frame; the joint matrices in a
    /// `with_frame` block where the original carves them with
    /// `_alloca_probe_16`.
    Phyre_PRendering_PRendererBase_renderBatch_005b9ee0 @ 0x005b_9ee0 => extern "thiscall" fn rbp3_005b9ee0(this: u32, batch: u32, cursor: u32, frame: u32) -> u32 {
        let mut f = StackFrame::<{ rb::BATCH_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * rb::BATCH_FRAME_WORDS;
        let args = (batch, cursor, frame);
        match rb::batch::joint_bytes(&mem(), this, batch) {
            Some(bytes) => with_frame(bytes, |at| rb::batch::render_batch(&mut mem(), ebp, this, args, at)),
            None => rb::batch::render_batch(&mut mem(), ebp, this, args, 0),
        }
    }
}
