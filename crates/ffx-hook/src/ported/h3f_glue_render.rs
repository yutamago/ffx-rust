//! `ffx_prender::h3f_*`: the render-side glue (M4, list `h3-glue-render`,
//! agent h3f).
//!
//! Threads: the main thread (the shadow manager's cascade update in the
//! scene's render); the shims keep no state.

use super::{StackFrame, mem, ported};
use ffx_prender::h3f_cascade::bridge as cascade;

ported! {
    // ---- batch 1 (ffx_prender::h3f_cascade) ----
    /// `ShadowManager_fitFieldCascades_006cebd0(this; light, solution)`:
    /// the field shadow's light view, cascade boxes, view and bias
    /// matrices. No result (the caller doesn't read `eax`).
    ShadowManager_fitFieldCascades_006cebd0 @ 0x006c_ebd0 => extern "thiscall" fn fit_field_cascades(this: u32, light: u32, solution: u32) {
        let mut f = StackFrame::<0x26a>::new();
        cascade::fit_field_cascades(&mut mem(), f.at() + 0x9a0, this, light, solution);
    }
}
