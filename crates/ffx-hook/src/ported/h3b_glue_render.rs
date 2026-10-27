//! `ffx_prender::h3b_*`: the render-side glue at `0x00690000` and up
//! (M4, list `h3-glue-render`, second agent h3b).
//!
//! Threads: the main thread (the scene's field-map set-up and render); the
//! shims keep no state.

use super::{StackFrame, mem, ported};
use ffx_prender::h3b_glue1::bridge as b1;
use ffx_prender::h3b_glue2::bridge as b2;
use ffx_prender::h3b_glue3::bridge as b3;
use ffx_prender::h3b_glue4::bridge as b4;
use ffx_prender::h3b_glue5::bridge as b5;

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
}
