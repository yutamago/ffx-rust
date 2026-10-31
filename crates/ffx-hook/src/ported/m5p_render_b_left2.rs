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
    /// `PDeferredLightingBase::initializeCommonShaders(cluster, effect)`.
    Phyre_PPostProcessing_PDeferredLightingBase_initializeCommonShaders @ 0x00a1_82f0 => extern "thiscall" fn init_common_shaders(this: u32, cluster: u32, effect: u32) -> u32 {
        lb::init_common_shaders(&mut mem(), this, cluster, effect)
    }
    /// A 4x4 matrix applied to a column `(out, v)`.
    PMatrix4_transformColumn_005b4c00 @ 0x005b_4c00 => extern "thiscall" fn transform_column(m: u32, out: u32, v: u32) -> u32 {
        lb::transform_column(&mut mem(), m, out, v)
    }
    /// The shader resource tables bound `(renderer, unused)`; `al` 1.
    PShaderResourceTables_bind_005930e0 @ 0x0059_30e0 => extern "thiscall" fn bind_resource_tables(this: u32, renderer: u32, _unused: u32) -> u8 {
        let mut f = StackFrame::<{ lf::TABLES_FRAME_WORDS as usize }>::new();
        lb::bind_resource_tables(&mut mem(), this, renderer, f.at())
    }
    /// A constant buffer set to a depth-remapped transposed matrix and a
    /// color `(renderer, m, color)`.
    PRendererD3D11_updateDepthTransform_0059a390 @ 0x0059_a390 => extern "thiscall" fn update_depth_transform(this: u32, renderer: u32, m: u32, color: u32) -> u32 {
        let mut f = StackFrame::<{ lf::DEPTH_FRAME_WORDS as usize }>::new();
        lb::update_depth_transform(&mut mem(), this, renderer, m, color, f.at())
    }
    /// The deferred lighting's texture parameters `(renderer, unused)`.
    Phyre_PPostProcessing_PDeferredLighting_setTextureParameters_00a13400 @ 0x00a1_3400 => extern "thiscall" fn set_texture_params(this: u32, renderer: u32, _unused: u32) -> u32 {
        let mut f = StackFrame::<{ lf::TEXTURE_FRAME_WORDS as usize }>::new();
        lb::set_texture_params(&mut mem(), this, renderer, f.at())
    }
    /// The profiles raised into a height line `(this, tables; values,
    /// floors, out, bias)`.
    HeightProfiles_raise_00a965d0 @ 0x00a9_65d0 => extern "fastcall-caller-pops" fn raise_profiles(this: u32, tables: u32, values: u32, floors: u32, out: u32, bias: u32) {
        lb::raise_profiles(&mut mem(), this, tables, values, floors, out, bias);
    }
    /// The deferred lighting's targets taken `(cluster, targets)`; the
    /// maker's error word (the original's argument slot) in a frame word.
    Phyre_PPostProcessing_PDeferredLighting_setTargets_00a191f0 @ 0x00a1_91f0 => extern "thiscall" fn take_targets(this: u32, cluster: u32, targets: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        lb::take_targets(&mut mem(), this, cluster, targets, f.at())
    }
    /// The deferred lighting's buffers `()`.
    Phyre_PPostProcessing_PDeferredLighting_createBuffers_00a19cf0 @ 0x00a1_9cf0 => extern "thiscall" fn create_buffers(this: u32) -> u32 {
        lb::create_buffers(&mut mem(), this)
    }
    /// The deferred pass's draw commands queued `(material, instance, a,
    /// b, c)`; the sizes' three words in a frame.
    Phyre_PRendering_PRendererBase_queueDeferredPass_005b6620 @ 0x005b_6620 => extern "thiscall" fn queue_pass(this: u32, material: u32, instance: u32, a: u32, b: u32, c: u32) -> u32 {
        let mut f = StackFrame::<{ lf::QUEUE_FRAME_WORDS as usize }>::new();
        lb::queue_pass(&mut mem(), this, material, instance, [a, b, c], f.at())
    }
}
