//! `ffx_prender::h3e_*`: `PhyreScene_render` (M4, list `h3-glue-render`,
//! agent h3e).
//!
//! Threads: the main thread (`graphicRender`); the shim keeps no state.

use super::{StackFrame, mem, ported};
use ffx_prender::h3e_render::bridge as render;

ported! {
    // ---- batch 1 (ffx_prender::h3e_render, h3e_scene, h3e_util) ----
    /// `PhyreScene_render(scene)` -> 0 or the first failing renderer
    /// call's code: the scene's frame (frame `ebp - 0x194`: the
    /// surfaces and camera matrices callees see).
    PhyreScene_render @ 0x0066_8930 => extern "thiscall" fn scene_render(this: u32) -> u32 {
        let mut f = StackFrame::<0x65>::new();
        render::render(&mut mem(), f.at() + 0x194, this)
    }
}
