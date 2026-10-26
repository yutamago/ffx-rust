//! M4 port, list p6-render-c, continuation round 3 (batch 15): the
//! shims. Ports in `ffx_prender::p6_targets` and `p6_late`.

use super::{StackFrame, mem, ported};
use ffx_prender::{p6_late as lt, p6_targets as tg};

ported! {
    /// `PRenderer::setColorTarget(this; target, slot)`.
    Phyre_PRendering_PRenderer_setColorTarget @ 0x005b_b130 => extern "thiscall" fn x_005bb130(this: u32, target: u32, slot: u32) -> u32 {
        tg::bridge::set_color_target(&mut mem(), this, target, slot)
    }
    /// `PRenderer::setDepthStencilTarget(this; target)`.
    Phyre_PRendering_PRenderer_setDepthStencilTarget @ 0x005b_b5b0 => extern "thiscall" fn x_005bb5b0(this: u32, target: u32) -> u32 {
        let mut frame = StackFrame::<2>::new();
        tg::bridge::set_depth_stencil_target(&mut mem(), this, frame.at(), target)
    }
    /// A light's attenuation factors (&a, &b, &c, &d, light); the light's
    /// argument slot as the original leaves it.
    Phyre_PRendering_PRendererBase_lightAttenuation_005b5ef0 @ 0x005b_5ef0 => extern "C" fn x_005b5ef0(a: u32, b: u32, c: u32, d: u32, light: u32; slots: u32) -> u32 {
        let mut m = mem();
        let (eax, slot) = tg::bridge::light_factors(&mut m, [a, b, c, d], light);
        ffx_bridge::Mem::write(&mut m, slots.wrapping_add(0x10), slot);
        eax
    }
    /// `PLegacyGlowGPUBase::initialize(this; cluster, w, h)`.
    Phyre_PPostProcessing_PLegacyGlowGPUBase_initialize @ 0x00a1_7bd0 => extern "thiscall" fn x_00a17bd0(this: u32, cluster: u32, w: u32, h: u32) -> u32 {
        tg::bridge::glow_gpu_initialize(&mut mem(), this, cluster, w, h)
    }
    /// `PLegacyGlowBase::initializeShaders(this; cluster, effect)`.
    Phyre_PPostProcessing_PLegacyGlowBase_initializeShaders @ 0x00a1_ab00 => extern "thiscall" fn x_00a1ab00(this: u32, cluster: u32, effect: u32) -> u32 {
        let mut frame = StackFrame::<4>::new();
        tg::bridge::glow_initialize_shaders(&mut mem(), this, frame.at(), cluster, effect)
    }
    /// A dynamic-geometry packet's layout (`this` the network; out, ctx).
    Phyre_PDynamicGeometry_layoutPacket_005c6c30 @ 0x005c_6c30 => extern "thiscall" fn x_005c6c30(this: u32, out: u32, ctx: u32) -> u32 {
        tg::bridge::packet_layout(&mut mem(), this, out, ctx)
    }
    /// The post-processing module's `Bind` (`PUtilityPostProcessing`'s
    /// vtable slot 1).
    Phyre_PPostProcessing_PUtilityPostProcessing_bind_00a18020 @ 0x00a1_8020 => extern "C" fn x_00a18020() -> u32 {
        lt::bridge::post_bind(&mut mem())
    }
    /// `PWindowWin32Base::setFocus(this; focus)`.
    Phyre_PFramework_PWindowWin32Base_setFocus @ 0x0062_a460 => extern "thiscall" fn x_0062a460(_this: u32, focus: u32) -> u32 {
        let mut frame = StackFrame::<0x2c>::new();
        lt::bridge::set_focus(&mut mem(), frame.at(), focus)
    }
    /// `PGlowGPUBase`'s destructor (`this`).
    Phyre_PPostProcessing_PGlowGPUBase_dtor_009fe0d0 @ 0x009f_e0d0 => extern "thiscall" fn x_009fe0d0(this: u32) -> u32 {
        lt::bridge::glow_gpu_dtor(&mut mem(), this)
    }
    // batch 16
    /// `PFXAABase::initializeCommonShaders(this; cluster, effect)`.
    Phyre_PPostProcessing_PFXAABase_initializeCommonShaders @ 0x00a1_8870 => extern "thiscall" fn x_00a18870(this: u32, cluster: u32, effect: u32) -> u32 {
        let mut frame = StackFrame::<8>::new();
        tg::bridge::common_shaders(&mut mem(), this, frame.at(), cluster, effect, &tg::FXAA_COMMON).0
    }
    /// `PDepthOfFieldBase::initializeCommonShaders(this; cluster, effect)`.
    Phyre_PPostProcessing_PDepthOfFieldBase_initializeCommonShaders @ 0x00a1_85a0 => extern "thiscall" fn x_00a185a0(this: u32, cluster: u32, effect: u32) -> u32 {
        let mut frame = StackFrame::<8>::new();
        tg::bridge::common_shaders(&mut mem(), this, frame.at(), cluster, effect, &tg::DOF_COMMON).0
    }
    /// `PMotionBlurBase`'s constructor (`this`) -> `this`.
    Phyre_PPostProcessing_PMotionBlurBase_PMotionBlurBase_009fcbb0 @ 0x009f_cbb0 => extern "thiscall" fn x_009fcbb0(this: u32) -> u32 {
        ffx_prender::p6_post::bridge::post_ctor(&mut mem(), this, &const { ffx_prender::p6_post::post_ctor(0x009f_cbb0) })
    }
    /// A mesh instance's draw commands (`this` the renderer; 8 words).
    Phyre_PRendering_PRendererBase_queueMeshCommands_005ba380 @ 0x005b_a380 => extern "thiscall" fn x_005ba380(this: u32, context: u32, instances: u32, extra: u32, material: u32, tag: u32, refs: u32, out: u32, callback: u32) -> u32 {
        let a = tg::MeshCommands { context, instances, extra, material, tag, refs, out, callback };
        tg::bridge::mesh_commands(&mut mem(), this, &a).0
    }
    // batch 17
    /// `PRendererBase::populateContextParameterCaptureBufferWithStreams(buf,
    /// ctx, &out, mask)`; the argument slots as the original leaves them.
    Phyre_PRendering_PRendererBase_populateContextParameterCaptureBufferWithStreams @ 0x005b_94b0 => extern "stdcall" fn x_005b94b0(buf: u32, ctx: u32, out: u32, mask: u32; slots: u32) -> u32 {
        tg::bridge::populate_with_streams(&mut mem(), slots, buf, ctx, out, mask)
    }
    /// `PRendererBase::populateContextParameterCaptureBufferWithDynamicStreams(buf,
    /// ctx, &flags, mesh, &base, &record)` -> the bound mask: the block
    /// pointer array (the original's `_alloca`) in this frame up to 64
    /// blocks, else on the heap.
    Phyre_PRendering_PRendererBase_populateContextParameterCaptureBufferWithDynamicStreams @ 0x005b_9290 => extern "stdcall" fn x_005b9290(buf: u32, ctx: u32, flags: u32, mesh: u32, base: u32, record: u32) -> u32 {
        let mut frame = StackFrame::<64>::new();
        let mut heap: Vec<u32> = Vec::new();
        let small = frame.at();
        let r = tg::bridge::populate_with_dynamic_streams(&mut mem(), [buf, ctx, flags, mesh, base, record], |n| {
            if n <= 64 {
                small
            } else {
                heap = vec![0u32; n as usize];
                u32::try_from(heap.as_mut_ptr().expose_provenance()).unwrap_or(0)
            }
        });
        std::hint::black_box(&mut heap);
        r
    }
    /// `PApplication::resize(this)` (vtable slot 9; `FFXApplication_resize`
    /// jumps here) -> 0 or the first error.
    Phyre_PFramework_PApplication_resize_0062c260 @ 0x0062_c260 => extern "thiscall" fn x_0062c260(this: u32) -> u32 {
        let mut frame = StackFrame::<8>::new();
        lt::bridge::resize(&mut mem(), this, frame.at())
    }
    /// The render thread's dynamic-geometry jobs (`this` the job state; a,
    /// b, first, end) -> the command it stopped at.
    Phyre_PRendering_PRenderThread_issueDynamicJobs_006148d0 @ 0x0061_48d0 => extern "thiscall" fn x_006148d0(this: u32, a: u32, b: u32, first: u32, end: u32) -> u32 {
        let mut frame = StackFrame::<9>::new();
        tg::bridge::issue_dynamic_jobs(&mut mem(), this, frame.at(), [a, b, first, end]).0
    }
}
