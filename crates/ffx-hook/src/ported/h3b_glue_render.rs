//! `ffx_prender::h3b_*`: the render-side glue at `0x00690000` and up
//! (M4, list `h3-glue-render`, second agent h3b).
//!
//! Threads: the main thread (the scene's field-map set-up and render); the
//! shims keep no state.

use super::{StackFrame, mem, ported};
use ffx_bridge::Mem;
use ffx_prender::h3b_glue1::bridge as b1;
use ffx_prender::h3b_glue2::bridge as b2;
use ffx_prender::h3b_glue3::bridge as b3;
use ffx_prender::h3b_glue4::bridge as b4;
use ffx_prender::h3b_glue5::bridge as b5;
use ffx_prender::h3b_glue6::bridge as b6;
use ffx_prender::h3b_glue7::bridge as b7;
use ffx_prender::h3b_glue8::bridge as b8;
use ffx_prender::h3b_glue9::bridge as b9;

ported! {
    // ---- batch 1 (ffx_prender::h3b_glue1) ----
    /// `ShadowManager_selectShadowSolution(this; arg0, battle, map, hash,
    /// scenario, renderer)`: the shadow solution 0-4.
    ShadowManager_selectShadowSolution @ 0x006c_cd10 => extern "thiscall" fn select_shadow_solution(this: u32, _arg0: u32, battle: u32, map: u32, hash: u32, scenario: u32, renderer: u32) -> u32 {
        b1::select_shadow_solution(&mut mem(), this, battle as u8, map, hash, scenario, renderer)
    }
    /// `PostProcessManager_renderUSM(this; renderer)`: the unsharp-mask
    /// passes; the renderer's slot takes the material's shader. The first
    /// error, or 0 (3: the sampler record doesn't fit).
    PostProcessManager_renderUSM @ 0x0069_7670 => extern "thiscall" fn render_usm(this: u32, _renderer: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<0x2e>::new();
        b2::render_usm(&mut mem(), f.at() + 0xb8, slots, this)
    }

    // ---- batch 2 (ffx_prender::h3b_glue3) ----
    /// `PostProcessManager_onAllShaderBinFilesLoaded(this)`: the manager's
    /// targets, meshes, effects and presets; 0, 0x13, 0xd or a callee's
    /// error.
    PostProcessManager_onAllShaderBinFilesLoaded @ 0x0069_3d70 => extern "thiscall" fn on_all_shader_bin_files_loaded(this: u32) -> u32 {
        let mut f = StackFrame::<0x11>::new();
        b3::on_all_shader_bin_files_loaded(&mut mem(), f.at() + 0x44, this)
    }

    // ---- batch 3 (ffx_prender::h3b_glue4) ----
    /// `PostProcessManager_renderHDAOPass(this; renderer, unused, target,
    /// normals)`: the HDAO pass; the renderer's and `normals`' slots are
    /// written. 0, 1, 3 or the first error.
    PostProcessManager_renderHDAOPass @ 0x0069_5ba0 => extern "thiscall" fn render_hdao_pass(this: u32, _renderer: u32, _unused: u32, _target: u32, _normals: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<0x5b>::new();
        b4::render_hdao_pass(&mut mem(), f.at() + 0x16c, slots, this)
    }

    // ---- batch 4 (ffx_prender::h3b_glue5) ----
    /// `ShadowManager_createResources_006d1b40(this)`: the shadow
    /// manager's meshes, material, samplers and state; 0, or 0x15.
    ShadowManager_createResources_006d1b40 @ 0x006d_1b40 => extern "thiscall" fn shadow_create_resources(this: u32) -> u32 {
        let mut f = StackFrame::<0xa>::new();
        b5::create_resources(&mut mem(), f.at() + 0x28, this)
    }

    // ---- round 2, batch 1 (ffx_prender::h3b_glue6) ----
    /// `TextureAnimationManager_addAnimatedMaterials(this; scene, clusters,
    /// first, last, name, unused, table)`: the clusters' textures and
    /// animation objects, the scene's materials and the shadow table into
    /// the manager. The frame keeps 0x40 bytes above `ebp`, where an
    /// over-long number in the table text would spill.
    TextureAnimationManager_addAnimatedMaterials @ 0x006b_05d0 => extern "thiscall" fn add_animated_materials(this: u32, scene: u32, clusters: u32, first: u32, last: u32, name: u32, _unused: u32, table: u32) {
        let mut f = StackFrame::<0x60>::new();
        b6::add_animated_materials(&mut mem(), f.at() + 0x140, this, scene, clusters, first, last, name, table);
    }

    // ---- round 2, batch 2 (ffx_prender::h3b_glue7) ----
    /// `ShadowManager_renderCharacterTiles_006caee0(this; renderer,
    /// solution, keep)`: the per-character shadow tiles, blurred; the
    /// first renderer error, 3, or the final scissor's result.
    ShadowManager_renderCharacterTiles_006caee0 @ 0x006c_aee0 => extern "thiscall" fn render_character_tiles(this: u32, renderer: u32, solution: u32, keep: u32) -> u32 {
        let mut f = StackFrame::<0x62>::new();
        b7::render_character_tiles(&mut mem(), f.at() + 0x188, this, renderer, solution, keep as u8 != 0)
    }

    // ---- round 2, batch 4 (ffx_prender::h3b_glue9) ----
    /// `ShadowManager_fitBattleCascade_006d00d0(this; light, focus)`: the
    /// battle shadow's light camera, box and bias.
    ShadowManager_fitBattleCascade_006d00d0 @ 0x006d_00d0 => extern "thiscall" fn fit_battle_cascade(this: u32, light: u32, focus: u32) {
        let mut f = StackFrame::<0x88>::new();
        b9::fit_battle_cascade(&mut mem(), f.at() + 0x21c, this, light, focus as u8);
    }

    // ---- round 2, batch 3 (ffx_prender::h3b_glue8) ----
    /// `ShadowManager_renderCascades_006c9e50(this; renderer, solution,
    /// keep)`: the cascaded shadow maps; the first renderer error, 3, or
    /// the final scissor's result. The original writes its `renderer`
    /// slot; its callers don't read it after.
    ShadowManager_renderCascades_006c9e50 @ 0x006c_9e50 => extern "thiscall" fn render_cascades(this: u32, renderer: u32, solution: u32, keep: u32) -> u32 {
        let mut f = StackFrame::<0xa8>::new();
        let ebp = f.at() + 0x290;
        b8::render_cascades(&mut mem(), ebp, this, renderer, solution, keep as u8, &mut |_, this, w| caster_crosses_edge(this, w))
    }
}

/// `ShadowManager_casterCrossesEdge_006c8f50(this; view 4x4 by value, flag,
/// ratio, scale_x, scale_y)` (`ret 0x50`): twenty stack words, more than
/// `call_thiscall` takes; `eax` (its `al` is the answer).
fn caster_crosses_edge(this: u32, w: &[u32; 20]) -> u32 {
    let f = mem().rebase(ffx_prender::h3b_glue8::CROSSES_EDGE);
    // the function of this build at that address is a thiscall taking
    // exactly twenty stack words, which it pops (`ret 0x50`)
    ffx_bridge::route::thiscall(f, this, w)
}
