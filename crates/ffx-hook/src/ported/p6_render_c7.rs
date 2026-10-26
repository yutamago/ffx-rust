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
}
