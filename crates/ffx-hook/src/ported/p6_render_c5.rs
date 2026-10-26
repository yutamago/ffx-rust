//! M4 port, list p6-render-c, continuation round 2 (batch 13): the
//! shims. Ports in `ffx_prender::p6_post`.

use super::{StackFrame, mem, ported};
use ffx_prender::p6_post as po;

ported! {
    /// `PRendererBase::setCamera(this; camera)`.
    Phyre_PRendering_PRendererBase_setCamera @ 0x005b_abd0 => extern "thiscall" fn v_005babd0(this: u32, camera: u32) -> u32 {
        let mut frame = StackFrame::<0x30>::new();
        po::bridge::set_camera(&mut mem(), this, frame.at(), camera)
    }
    /// `PCamera::updateViewMatrices(this)`.
    Phyre_PCamera_updateViewMatrices @ 0x0067_20c0 => extern "thiscall" fn v_006720c0(this: u32) -> u32 {
        let mut frame = StackFrame::<0x30>::new();
        po::bridge::update_view_matrices(&mut mem(), this, frame.at())
    }
    /// `PRenderThread::PRenderThread(this)` -> `this`.
    Phyre_PRendering_PRenderThread_PRenderThread @ 0x0061_4010 => extern "thiscall" fn v_00614010(this: u32) -> u32 {
        po::bridge::thread_ctor(&mut mem(), this)
    }
    /// `PRenderThread::threadEntry(this)` -> 0: the render thread's loop.
    Phyre_PRendering_PRenderThread_threadEntry @ 0x0061_6010 => extern "thiscall" fn v_00616010(this: u32) -> u32 {
        po::bridge::thread_entry(&mut mem(), this)
    }
    /// `PDeferredLightingBase`'s deleting destructor (`this`; flags) -> `this`.
    Phyre_PPostProcessing_PDeferredLightingBase_deletingDtor_00a026d0 @ 0x00a0_26d0 => extern "thiscall" fn v_00a026d0(this: u32, flags: u32) -> u32 {
        po::bridge::deferred_deleting_dtor(&mut mem(), this, flags)
    }
    /// The GPU glow's target and effect (`this`; cluster, w, h) -> 0 or 0xd.
    Phyre_PPostProcessing_PLegacyGlowGPUBase_setupTarget_00a1a2c0 @ 0x00a1_a2c0 => extern "thiscall" fn v_00a1a2c0(this: u32, cluster: u32, w: u32, h: u32) -> u32 {
        po::bridge::glow_gpu_setup(&mut mem(), this, cluster, w, h)
    }
    /// The glow's set-up (`this`; cluster, w, h) -> 0 or an error.
    Phyre_PPostProcessing_PLegacyGlow_setup_00a176d0 @ 0x00a1_76d0 => extern "thiscall" fn v_00a176d0(this: u32, cluster: u32, w: u32, h: u32) -> u32 {
        po::bridge::glow_setup(&mut mem(), this, cluster, w, h)
    }
    /// The glow's three half-size targets (`this`; cluster, w, h) -> 0 or 0xd.
    Phyre_PPostProcessing_PLegacyGlowBase_setupTargets_00a1a190 @ 0x00a1_a190 => extern "thiscall" fn v_00a1a190(this: u32, cluster: u32, w: u32, h: u32) -> u32 {
        po::bridge::glow_base_setup(&mut mem(), this, cluster, w, h)
    }
    /// An Iggy player object created (7 words) -> it, or 0.
    Iggy_createPlayer_00a3e4f0 @ 0x00a3_e4f0 => extern "C" fn v_00a3e4f0(a: u32, b: u32, c: u32, d: u32, e: u32, f: u32, g: u32) -> u32 {
        po::bridge::player_create(&mut mem(), [a, b, c, d, e, f, g])
    }
    /// `PUtilityText::PUtilityText(this)` -> `this`.
    Phyre_PText_PUtilityText_PUtilityText_00a3e730 @ 0x00a3_e730 => extern "thiscall" fn v_00a3e730(this: u32) -> u32 {
        po::bridge::utility_text_ctor(&mut mem(), this)
    }
    /// A zeroing constructor (`this`) -> `this`.
    Phyre_PFramework_PInputMapper_PInputMapper_00620590 @ 0x0062_0590 => extern "thiscall" fn v_00620590(this: u32) -> u32 {
        po::bridge::stores(&mut mem(), this, po::STORES_620590)
    }
    /// A constructor of defaults (`this`) -> `this`.
    Phyre_PFramework_PApplication_defaultsCtor_00620780 @ 0x0062_0780 => extern "thiscall" fn v_00620780(this: u32) -> u32 {
        po::bridge::stores(&mut mem(), this, po::STORES_620780)
    }
    /// A modifier network's destructor (`this`).
    Phyre_PDynamicGeometry_PModifierNetworkDynamicMesh_dtor_005aa4b0 @ 0x005a_a4b0 => extern "thiscall" fn v_005aa4b0(this: u32) -> u32 {
        po::bridge::network_dtor(&mut mem(), this)
    }
    /// A segment's packet filled and submitted (`this`; i, packet, table, _).
    Phyre_PDynamicGeometry_PModifierNetworkDynamicMesh_fillPacket_005ae0f0 @ 0x005a_e0f0 => extern "thiscall" fn v_005ae0f0(this: u32, i: u32, packet: u32, table: u32, _unused: u32) -> u32 {
        po::bridge::fill_packet(&mut mem(), this, i, packet, table)
    }
    /// A mesh instance's buffer sizes (9 words) -> `table`.
    Phyre_PRendering_PRendererBase_meshInstanceSizes_005b5150 @ 0x005b_5150 => extern "C" fn v_005b5150(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32) -> u32 {
        po::bridge::mesh_sizes(&mut mem(), [a0, a1, a2, a3, a4, a5, a6, a7, a8])
    }
    /// The textures' record addresses (base, params, out) -> the cursor.
    Phyre_PRendering_PRendererBase_texturePointers_005b53f0 @ 0x005b_53f0 => extern "C" fn v_005b53f0(base: u32, params: u32, out: u32) -> u32 {
        po::bridge::texture_pointers(&mut mem(), base, params, out)
    }
    /// A callback command with a data block (`this`; f, size, out) -> 0, 1 or 0xd.
    Phyre_PRendering_PRendererBase_queueDataCallback_005b5940 @ 0x005b_5940 => extern "thiscall" fn v_005b5940(this: u32, f: u32, size: u32, out: u32) -> u32 {
        let mut frame = StackFrame::<2>::new();
        po::bridge::queue_data_callback(&mut mem(), this, frame.at(), f, size, out)
    }
    /// A post effect's constructor (`this`) -> `this`.
    Phyre_PPostProcessing_PFXAABase_PFXAABase_009fc160 @ 0x009f_c160 => extern "thiscall" fn w_009fc160(this: u32) -> u32 {
        po::bridge::post_ctor(&mut mem(), this, &const { po::post_ctor(0x009f_c160) })
    }
    /// A post effect's constructor (`this`) -> `this`.
    Phyre_PPostProcessing_PGlowBase_PGlowBase_009fc310 @ 0x009f_c310 => extern "thiscall" fn w_009fc310(this: u32) -> u32 {
        po::bridge::post_ctor(&mut mem(), this, &const { po::post_ctor(0x009f_c310) })
    }
    /// A post effect's constructor (`this`) -> `this`.
    Phyre_PPostProcessing_PPostEffectSampled_ctor_009fc540 @ 0x009f_c540 => extern "thiscall" fn w_009fc540(this: u32) -> u32 {
        po::bridge::post_ctor(&mut mem(), this, &const { po::post_ctor(0x009f_c540) })
    }
    /// A post effect's constructor (`this`) -> `this`.
    Phyre_PPostProcessing_PMLAABase_PMLAABase_009fc7e0 @ 0x009f_c7e0 => extern "thiscall" fn w_009fc7e0(this: u32) -> u32 {
        po::bridge::post_ctor(&mut mem(), this, &const { po::post_ctor(0x009f_c7e0) })
    }
    /// A post effect's constructor (`this`) -> `this`.
    Phyre_PPostProcessing_PScreenSpaceReflectionBase_PScreenSpaceReflectionBase_009fd080 @ 0x009f_d080 => extern "thiscall" fn w_009fd080(this: u32) -> u32 {
        po::bridge::post_ctor(&mut mem(), this, &const { po::post_ctor(0x009f_d080) })
    }
    /// A post effect's destructor (`this`).
    Phyre_PPostProcessing_PDepthOfFieldBase_dtor_009fdd30 @ 0x009f_dd30 => extern "thiscall" fn w_009fdd30(this: u32) -> u32 {
        po::bridge::post_dtor(&mut mem(), this, &const { po::post_dtor(0x009f_dd30) })
    }
    /// A post effect's destructor (`this`).
    Phyre_PPostProcessing_PFXAABase_dtor_009fdf60 @ 0x009f_df60 => extern "thiscall" fn w_009fdf60(this: u32) -> u32 {
        po::bridge::post_dtor(&mut mem(), this, &const { po::post_dtor(0x009f_df60) })
    }
    /// A post effect's destructor (`this`).
    Phyre_PPostProcessing_PGlowBase_dtor_009fe020 @ 0x009f_e020 => extern "thiscall" fn w_009fe020(this: u32) -> u32 {
        po::bridge::post_dtor(&mut mem(), this, &const { po::post_dtor(0x009f_e020) })
    }
    /// A post effect's destructor (`this`).
    Phyre_PPostProcessing_PMLAABase_dtor_009fe2a0 @ 0x009f_e2a0 => extern "thiscall" fn w_009fe2a0(this: u32) -> u32 {
        po::bridge::post_dtor(&mut mem(), this, &const { po::post_dtor(0x009f_e2a0) })
    }
    /// A post effect's destructor (`this`).
    Phyre_PPostProcessing_PMotionBlurBase_dtor_009fe640 @ 0x009f_e640 => extern "thiscall" fn w_009fe640(this: u32) -> u32 {
        po::bridge::post_dtor(&mut mem(), this, &const { po::post_dtor(0x009f_e640) })
    }
    /// A post effect's destructor (`this`).
    Phyre_PPostProcessing_PScreenSpaceReflectionBase_dtor_009fe890 @ 0x009f_e890 => extern "thiscall" fn w_009fe890(this: u32) -> u32 {
        po::bridge::post_dtor(&mut mem(), this, &const { po::post_dtor(0x009f_e890) })
    }
    /// A camera-like object's defaults (`this`) -> `this`.
    Phyre_PPostProcessing_cameraDefaults_009fc6f0 @ 0x009f_c6f0 => extern "thiscall" fn w_009fc6f0(this: u32) -> u32 {
        let mut frame = StackFrame::<0x10>::new();
        po::bridge::camera_defaults(&mut mem(), this, frame.at())
    }
    /// `PDeferredLighting`'s destructor (`this`).
    Phyre_PPostProcessing_PDeferredLightingD3D11_dtor_009fdc00 @ 0x009f_dc00 => extern "thiscall" fn w_009fdc00(this: u32) -> u32 {
        po::bridge::deferred_dtor(&mut mem(), this)
    }
    /// Two render targets and an array released (`this`).
    Phyre_PPostProcessing_targetsArray_dtor_009fe790 @ 0x009f_e790 => extern "thiscall" fn w_009fe790(this: u32) -> u32 {
        po::bridge::targets_array_dtor(&mut mem(), this)
    }
    /// `PTexture2DD3D11::unbind(this)` -> 0 or an error.
    Phyre_PRendering_PTexture2DD3D11_unbind @ 0x0059_cc40 => extern "thiscall" fn w_0059cc40(this: u32) -> u32 {
        po::bridge::unbind(&mut mem(), this, &const { po::unbind_of(0x0059_cc40) })
    }
    /// The cube texture's `unbind(this)` -> 0 or an error.
    Phyre_PRendering_PTextureCubeD3D11_unbind_0059cf00 @ 0x0059_cf00 => extern "thiscall" fn w_0059cf00(this: u32) -> u32 {
        po::bridge::unbind(&mut mem(), this, &const { po::unbind_of(0x0059_cf00) })
    }
    /// The 3D texture's `unbind(this)` -> 0 or an error.
    Phyre_PRendering_PTexture3DD3D11_unbind_0059d0f0 @ 0x0059_d0f0 => extern "thiscall" fn w_0059d0f0(this: u32) -> u32 {
        po::bridge::unbind(&mut mem(), this, &const { po::unbind_of(0x0059_d0f0) })
    }
    /// `PStructuredBufferD3D11::unbind(this)` -> 0.
    Phyre_PRendering_PStructuredBufferD3D11_unbind_0059ca60 @ 0x0059_ca60 => extern "thiscall" fn w_0059ca60(this: u32) -> u32 {
        po::bridge::unbind(&mut mem(), this, &const { po::unbind_of(0x0059_ca60) })
    }
}
