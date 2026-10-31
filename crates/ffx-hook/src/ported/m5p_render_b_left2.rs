//! M5 stage 1, the leftovers of list `phyre-render-b` (round left2)
//! (`ffx_prender::m5p_render_b_left2`). The parity tests are in
//! `crates/ffx-oracle/tests/m5p_render_b.rs` (`left2_*`).
//!
//! Threads: shader initialisation runs on the render setup path (main
//! thread); no state is kept.

use ffx_prender::m5p_render_b_left2::bridge as lb;

use super::{mem, ported};

ported! {
    /// `PDeferredLightingNVN::initializeShaders(a, effect)`.
    Phyre_PPostProcessing_PDeferredLightingNVN_initializeShaders @ 0x00a1_a4f0 => extern "thiscall" fn init_nvn_shaders(this: u32, a: u32, effect: u32) -> u32 {
        lb::init_nvn_shaders(&mut mem(), this, a, effect)
    }
}
