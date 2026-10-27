//! `ffx_prender::h3b_*`: the render-side glue at `0x00690000` and up
//! (M4, list `h3-glue-render`, second agent h3b).
//!
//! Threads: the main thread (the scene's field-map set-up and render); the
//! shims keep no state.

use super::{StackFrame, mem, ported};
use ffx_prender::h3b_glue1::bridge as b1;
use ffx_prender::h3b_glue2::bridge as b2;

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
}
