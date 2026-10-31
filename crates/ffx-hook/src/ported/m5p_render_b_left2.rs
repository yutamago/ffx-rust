//! M5 stage 1, the leftovers of list `phyre-render-b` (round left2)
//! (`ffx_prender::m5p_render_b_left2`). The parity tests are in
//! `crates/ffx-oracle/tests/m5p_render_b.rs` (`left2_*`).
//!
//! Threads: shader initialisation runs on the render setup path (main
//! thread); no state is kept.

use ffx_prender::m5p_render_b_left2::{self as lf, bridge as lb};

use super::{StackFrame, mem, ported};

ported! {
    /// `PDeferredLightingNVN::initializeShaders(a, effect)`.
    Phyre_PPostProcessing_PDeferredLightingNVN_initializeShaders @ 0x00a1_a4f0 => extern "thiscall" fn init_nvn_shaders(this: u32, a: u32, effect: u32) -> u32 {
        lb::init_nvn_shaders(&mut mem(), this, a, effect)
    }
    /// `PScreenSpaceReflectionBase::initializeShaders(cluster, effect)`.
    Phyre_PPostProcessing_PScreenSpaceReflectionBase_initializeShaders @ 0x00a1_ae00 => extern "thiscall" fn init_ssr_shaders(this: u32, cluster: u32, effect: u32) -> u32 {
        let mut f = StackFrame::<{ lf::SAMPLER_FRAME_WORDS as usize }>::new();
        lb::init_sampler_shaders(&mut mem(), &const { lf::sampler_shaders_of(0x00a1_ae00) }, this, cluster, effect, f.at())
    }
    /// `PGlowBase::initializeShaders(cluster, effect)`.
    Phyre_PPostProcessing_PGlowBase_initializeShaders @ 0x00a1_a830 => extern "thiscall" fn init_glow_shaders(this: u32, cluster: u32, effect: u32) -> u32 {
        let mut f = StackFrame::<{ lf::SAMPLER_FRAME_WORDS as usize }>::new();
        lb::init_sampler_shaders(&mut mem(), &const { lf::sampler_shaders_of(0x00a1_a830) }, this, cluster, effect, f.at())
    }
    /// The light setup's cluster load `()` (its name argument is a junk
    /// read: 0).
    Phyre_PS2LightingSetup_loadCluster_006e5be0 @ 0x006e_5be0 => extern "C" fn load_light_cluster() {
        let mut f = StackFrame::<8>::new();
        lb::load_light_cluster(&mut mem(), f.at());
    }
}
