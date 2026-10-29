//! M5 stage 1, list `phyre-render-b` (`ffx_prender::m5p_render_b*`): the
//! rest of `PRendering`, the post-processing classes and the renderer's
//! display-mode and present code. The parity tests are
//! `crates/ffx-oracle/tests/m5p_render_b.rs`.
//!
//! Threads: the destructors and releases run at exit or while a static
//! initialiser unwinds; the state setters on the render thread. No shim
//! keeps state; each works on the game's memory through [`mem`].

use ffx_prender::m5p_render_b::{self as rb, bridge as b};

use super::{mem, ported};

ported! {
    // ---- batch 1: the generated template shapes (m5p_render_b_table.rs.in) ----
    /// `PClassDescriptorForType_Phyre_PArray_unsigned_char_const_4_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PArray_unsigned_char_const_4_Destruct @ 0x005a_a460 => extern "thiscall" fn d_005aa460(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005a_a460) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment_4_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment_4_Destruct @ 0x005a_a470 => extern "thiscall" fn d_005aa470(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005a_a470) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PRenderStream_4_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PRenderStream_4_Destruct @ 0x005a_a480 => extern "thiscall" fn d_005aa480(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005a_a480) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkDynamicMesh_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkDynamicMesh_Destruct @ 0x005a_a490 => extern "thiscall" fn d_005aa490(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005a_a490) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment_Destruct @ 0x005a_a4a0 => extern "thiscall" fn d_005aa4a0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005a_a4a0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PArray_Phyre_PBase_const_4_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PArray_Phyre_PBase_const_4_Destruct @ 0x005b_0e00 => extern "thiscall" fn d_005b0e00(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005b_0e00) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInstance_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInstance_Destruct @ 0x005b_0e10 => extern "thiscall" fn d_005b0e10(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005b_0e10) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInstanceInput_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInstanceInput_Destruct @ 0x005b_0e20 => extern "thiscall" fn d_005b0e20(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005b_0e20) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInstancePacketInput_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInstancePacketInput_Destruct @ 0x005b_0e30 => extern "thiscall" fn d_005b0e30(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005b_0e30) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PModifierAndInputs_4_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PModifierAndInputs_4_Destruct @ 0x005b_f6c0 => extern "thiscall" fn d_005bf6c0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005b_f6c0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkBuffer_4_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkBuffer_4_Destruct @ 0x005b_f6d0 => extern "thiscall" fn d_005bf6d0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005b_f6d0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PRenderStreamInput_4_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PRenderStreamInput_4_Destruct @ 0x005b_f6e0 => extern "thiscall" fn d_005bf6e0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005b_f6e0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierAndInputs_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierAndInputs_Destruct @ 0x005b_f6f0 => extern "thiscall" fn d_005bf6f0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005b_f6f0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetwork_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetwork_Destruct @ 0x005b_f700 => extern "thiscall" fn d_005bf700(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005b_f700) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkBuffer_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkBuffer_Destruct @ 0x005b_f710 => extern "thiscall" fn d_005bf710(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005b_f710) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_Destruct @ 0x005b_f720 => extern "thiscall" fn d_005bf720(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005b_f720) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_Buffer_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_Buffer_Destruct @ 0x005b_f730 => extern "thiscall" fn d_005bf730(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005b_f730) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_ModifierCode_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_ModifierCode_Destruct @ 0x005b_f740 => extern "thiscall" fn d_005bf740(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005b_f740) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_ModifierInstance_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_ModifierInstance_Destruct @ 0x005b_f750 => extern "thiscall" fn d_005bf750(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005b_f750) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PDynamicGeometry_PRenderStream_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PDynamicGeometry_PRenderStream_Destruct @ 0x005b_f760 => extern "thiscall" fn d_005bf760(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005b_f760) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PDynamicGeometry_PRenderStreamInput_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PDynamicGeometry_PRenderStreamInput_Destruct @ 0x005b_f770 => extern "thiscall" fn d_005bf770(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005b_f770) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PDynamicGeometry_PMorphModifierWeightsUserDataObject_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PDynamicGeometry_PMorphModifierWeightsUserDataObject_Destruct @ 0x005c_b2c0 => extern "thiscall" fn d_005cb2c0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x005c_b2c0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `Phyre_PAnimation_PInternal_PAvoidOutputOverwritingInput_dtor`: `[this]` = its vtable, then the parent's destructor.
    Phyre_PAnimation_PInternal_PAvoidOutputOverwritingInput_dtor @ 0x0061_6550 => extern "thiscall" fn d_00616550(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x0061_6550) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PArray_Phyre_PPostProcessing_PPostEffectBase_4_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PArray_Phyre_PPostProcessing_PPostEffectBase_4_Destruct @ 0x009f_d920 => extern "thiscall" fn d_009fd920(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_d920) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PDeferredLighting_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PDeferredLighting_Destruct @ 0x009f_d930 => extern "thiscall" fn d_009fd930(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_d930) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PDeferredLightingBase_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PDeferredLightingBase_Destruct @ 0x009f_d940 => extern "thiscall" fn d_009fd940(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_d940) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PDeferredLightingD3D11_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PDeferredLightingD3D11_Destruct @ 0x009f_d950 => extern "thiscall" fn d_009fd950(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_d950) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PDepthOfField_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PDepthOfField_Destruct @ 0x009f_d960 => extern "thiscall" fn d_009fd960(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_d960) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PDepthOfFieldBase_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PDepthOfFieldBase_Destruct @ 0x009f_d970 => extern "thiscall" fn d_009fd970(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_d970) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PDepthOfFieldD3D11_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PDepthOfFieldD3D11_Destruct @ 0x009f_d980 => extern "thiscall" fn d_009fd980(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_d980) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PFXAA_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PFXAA_Destruct @ 0x009f_d990 => extern "thiscall" fn d_009fd990(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_d990) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PFXAABase_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PFXAABase_Destruct @ 0x009f_d9a0 => extern "thiscall" fn d_009fd9a0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_d9a0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PFXAAD3D11_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PFXAAD3D11_Destruct @ 0x009f_d9b0 => extern "thiscall" fn d_009fd9b0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_d9b0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PGlow_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PGlow_Destruct @ 0x009f_d9c0 => extern "thiscall" fn d_009fd9c0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_d9c0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PGlowBase_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PGlowBase_Destruct @ 0x009f_d9d0 => extern "thiscall" fn d_009fd9d0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_d9d0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PGlowD3D11_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PGlowD3D11_Destruct @ 0x009f_d9e0 => extern "thiscall" fn d_009fd9e0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_d9e0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PGlowGPUBase_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PGlowGPUBase_Destruct @ 0x009f_d9f0 => extern "thiscall" fn d_009fd9f0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_d9f0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PLegacyGlow_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PLegacyGlow_Destruct @ 0x009f_da00 => extern "thiscall" fn d_009fda00(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_da00) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PLegacyGlowBase_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PLegacyGlowBase_Destruct @ 0x009f_da10 => extern "thiscall" fn d_009fda10(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_da10) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PLegacyGlowD3D11_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PLegacyGlowD3D11_Destruct @ 0x009f_da20 => extern "thiscall" fn d_009fda20(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_da20) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PLegacyGlowGPUBase_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PLegacyGlowGPUBase_Destruct @ 0x009f_da30 => extern "thiscall" fn d_009fda30(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_da30) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PMLAA_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PMLAA_Destruct @ 0x009f_da40 => extern "thiscall" fn d_009fda40(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_da40) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PMLAABase_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PMLAABase_Destruct @ 0x009f_da50 => extern "thiscall" fn d_009fda50(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_da50) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PMLAAD3D11_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PMLAAD3D11_Destruct @ 0x009f_da60 => extern "thiscall" fn d_009fda60(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_da60) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PMeshParticleSystemBase_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PMeshParticleSystemBase_Destruct @ 0x009f_da70 => extern "thiscall" fn d_009fda70(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_da70) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PMotionBlur_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PMotionBlur_Destruct @ 0x009f_da80 => extern "thiscall" fn d_009fda80(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_da80) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PMotionBlurBase_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PMotionBlurBase_Destruct @ 0x009f_da90 => extern "thiscall" fn d_009fda90(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_da90) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PMotionBlurD3D11_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PMotionBlurD3D11_Destruct @ 0x009f_daa0 => extern "thiscall" fn d_009fdaa0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_daa0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PPostEffectBase_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PPostEffectBase_Destruct @ 0x009f_dab0 => extern "thiscall" fn d_009fdab0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_dab0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PPostEffectManager_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PPostEffectManager_Destruct @ 0x009f_dac0 => extern "thiscall" fn d_009fdac0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_dac0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceAmbientOcclusion_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceAmbientOcclusion_Destruct @ 0x009f_dad0 => extern "thiscall" fn d_009fdad0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_dad0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceAmbientOcclusionBase_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceAmbientOcclusionBase_Destruct @ 0x009f_dae0 => extern "thiscall" fn d_009fdae0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_dae0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceAmbientOcclusionD3D11_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceAmbientOcclusionD3D11_Destruct @ 0x009f_daf0 => extern "thiscall" fn d_009fdaf0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_daf0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceReflection_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceReflection_Destruct @ 0x009f_db00 => extern "thiscall" fn d_009fdb00(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_db00) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceReflectionBase_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceReflectionBase_Destruct @ 0x009f_db10 => extern "thiscall" fn d_009fdb10(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_db10) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceReflectionD3D11_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceReflectionD3D11_Destruct @ 0x009f_db20 => extern "thiscall" fn d_009fdb20(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x009f_db20) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PArray_Phyre_PText_PBitmapFontCharInfo_4_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PArray_Phyre_PText_PBitmapFontCharInfo_4_Destruct @ 0x00a2_bb90 => extern "thiscall" fn d_00a2bb90(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x00a2_bb90) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PText_PBitmapFont_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PText_PBitmapFont_Destruct @ 0x00a2_bba0 => extern "thiscall" fn d_00a2bba0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x00a2_bba0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `PClassDescriptorForType_Phyre_PText_PBitmapFontCharInfo_Destruct`: `[this]` = its vtable, then the parent's destructor.
    PClassDescriptorForType_Phyre_PText_PBitmapFontCharInfo_Destruct @ 0x00a2_bbb0 => extern "thiscall" fn d_00a2bbb0(this: u32) -> u32 {
        let (vt, parent) = const { rb::vt_tail_of(0x00a2_bbb0) };
        b::vt_tail(&mut mem(), this, vt, parent)
    }
    /// `Phyre_PArray_releaseStorage_005aa2d0`: the array's storage release (`Free`).
    Phyre_PArray_releaseStorage_005aa2d0 @ 0x005a_a2d0 => extern "thiscall" fn r_005aa2d0(this: u32) -> u32 {
        b::release(&mut mem(), this, const { rb::release_of(0x005a_a2d0) })
    }
    /// `Phyre_PArray_releaseStorage_005b0d30`: the array's storage release (`Free`).
    Phyre_PArray_releaseStorage_005b0d30 @ 0x005b_0d30 => extern "thiscall" fn r_005b0d30(this: u32) -> u32 {
        b::release(&mut mem(), this, const { rb::release_of(0x005b_0d30) })
    }
    /// `Phyre_PArray_releaseStorage_009fd450`: the array's storage release (`Free`).
    Phyre_PArray_releaseStorage_009fd450 @ 0x009f_d450 => extern "thiscall" fn r_009fd450(this: u32) -> u32 {
        b::release(&mut mem(), this, const { rb::release_of(0x009f_d450) })
    }
    /// `Phyre_PArray_releaseStorage_005a03b0`: the array's storage release (`FreeNonNull`).
    Phyre_PArray_releaseStorage_005a03b0 @ 0x005a_03b0 => extern "thiscall" fn r_005a03b0(this: u32) -> u32 {
        b::release(&mut mem(), this, const { rb::release_of(0x005a_03b0) })
    }
    /// `Phyre_PArray_releaseStorage_005aa330`: the array's storage release (`FreeNonNull`).
    Phyre_PArray_releaseStorage_005aa330 @ 0x005a_a330 => extern "thiscall" fn r_005aa330(this: u32) -> u32 {
        b::release(&mut mem(), this, const { rb::release_of(0x005a_a330) })
    }
    /// `Phyre_PArray_releaseStorage_005bf490`: the array's storage release (`FreeNonNull`).
    Phyre_PArray_releaseStorage_005bf490 @ 0x005b_f490 => extern "thiscall" fn r_005bf490(this: u32) -> u32 {
        b::release(&mut mem(), this, const { rb::release_of(0x005b_f490) })
    }
    /// `Phyre_PArray_releaseStorage_005c8230`: the array's storage release (`FreeNonNull`).
    Phyre_PArray_releaseStorage_005c8230 @ 0x005c_8230 => extern "thiscall" fn r_005c8230(this: u32) -> u32 {
        b::release(&mut mem(), this, const { rb::release_of(0x005c_8230) })
    }
    /// `Phyre_PArray_releaseStorage_006abbc0`: the array's storage release (`FreeNonNull`).
    Phyre_PArray_releaseStorage_006abbc0 @ 0x006a_bbc0 => extern "thiscall" fn r_006abbc0(this: u32) -> u32 {
        b::release(&mut mem(), this, const { rb::release_of(0x006a_bbc0) })
    }
    /// `Phyre_PArray_releaseStorage_009fd3f0`: the array's storage release (`FreeNonNull`).
    Phyre_PArray_releaseStorage_009fd3f0 @ 0x009f_d3f0 => extern "thiscall" fn r_009fd3f0(this: u32) -> u32 {
        b::release(&mut mem(), this, const { rb::release_of(0x009f_d3f0) })
    }
    /// `Phyre_PArray_releaseStorage_009fd420`: the array's storage release (`FreeNonNull`).
    Phyre_PArray_releaseStorage_009fd420 @ 0x009f_d420 => extern "thiscall" fn r_009fd420(this: u32) -> u32 {
        b::release(&mut mem(), this, const { rb::release_of(0x009f_d420) })
    }
    /// `Phyre_PArray_releaseStorage_00a2baf0`: the array's storage release (`FreeNonNull`).
    Phyre_PArray_releaseStorage_00a2baf0 @ 0x00a2_baf0 => extern "thiscall" fn r_00a2baf0(this: u32) -> u32 {
        b::release(&mut mem(), this, const { rb::release_of(0x00a2_baf0) })
    }
    /// `Phyre_PArray_releaseStorage_005bc470`: the array's storage release (`SkipEmpty`).
    Phyre_PArray_releaseStorage_005bc470 @ 0x005b_c470 => extern "thiscall" fn r_005bc470(this: u32) -> u32 {
        b::release(&mut mem(), this, const { rb::release_of(0x005b_c470) })
    }
    /// `Phyre_PArray_releaseStorage_005bf450`: the array's storage release (`RecordsFree`).
    Phyre_PArray_releaseStorage_005bf450 @ 0x005b_f450 => extern "thiscall" fn r_005bf450(this: u32) -> u32 {
        b::release(&mut mem(), this, const { rb::release_of(0x005b_f450) })
    }
    /// `Phyre_PRendering_PRenderInterface_unsupportedState_005997a0`: `0x17` (D3D11 has no such state).
    Phyre_PRendering_PRenderInterface_unsupportedState_005997a0 @ 0x0059_97a0 => extern "thiscall" fn n_005997a0(_this: u32, _a0: u32, _a1: u32, _a2: u32) -> u32 {
        const { assert!(rb::not_supported_words(0x0059_97a0) == 3) };
        rb::E_NOT_SUPPORTED
    }
    /// `Phyre_PRendering_PRenderInterfaceNVN_ValidateRenderTargetConfig`: `0x17` (D3D11 has no such state).
    Phyre_PRendering_PRenderInterfaceNVN_ValidateRenderTargetConfig @ 0x0059_97b0 => extern "thiscall" fn n_005997b0(_this: u32, _a0: u32, _a1: u32) -> u32 {
        const { assert!(rb::not_supported_words(0x0059_97b0) == 2) };
        rb::E_NOT_SUPPORTED
    }
    /// `Phyre_PRendering_PRenderInterface_unsupportedState_00599800`: `0x17` (D3D11 has no such state).
    Phyre_PRendering_PRenderInterface_unsupportedState_00599800 @ 0x0059_9800 => extern "thiscall" fn n_00599800(_this: u32, _a0: u32, _a1: u32, _a2: u32, _a3: u32) -> u32 {
        const { assert!(rb::not_supported_words(0x0059_9800) == 4) };
        rb::E_NOT_SUPPORTED
    }
    /// `Phyre_PRendering_PRenderInterface_unsupportedState_0059a1a0`: `0x17` (D3D11 has no such state).
    Phyre_PRendering_PRenderInterface_unsupportedState_0059a1a0 @ 0x0059_a1a0 => extern "thiscall" fn n_0059a1a0(_this: u32, _a0: u32, _a1: u32, _a2: u32) -> u32 {
        const { assert!(rb::not_supported_words(0x0059_a1a0) == 3) };
        rb::E_NOT_SUPPORTED
    }
    /// `Phyre_PRendering_PRenderInterface_unsupportedState_0059a2b0`: `0x17` (D3D11 has no such state).
    Phyre_PRendering_PRenderInterface_unsupportedState_0059a2b0 @ 0x0059_a2b0 => extern "thiscall" fn n_0059a2b0(_this: u32, _a0: u32) -> u32 {
        const { assert!(rb::not_supported_words(0x0059_a2b0) == 1) };
        rb::E_NOT_SUPPORTED
    }
    /// `Phyre_PRendering_PRenderInterface_unsupportedState_0059a8b0`: `0x17` (D3D11 has no such state).
    Phyre_PRendering_PRenderInterface_unsupportedState_0059a8b0 @ 0x0059_a8b0 => extern "thiscall" fn n_0059a8b0(_this: u32, _a0: u32) -> u32 {
        const { assert!(rb::not_supported_words(0x0059_a8b0) == 1) };
        rb::E_NOT_SUPPORTED
    }
    /// `Phyre_PRendering_PRenderInterface_unsupportedState_0059a8c0`: `0x17` (D3D11 has no such state).
    Phyre_PRendering_PRenderInterface_unsupportedState_0059a8c0 @ 0x0059_a8c0 => extern "thiscall" fn n_0059a8c0(_this: u32, _a0: u32) -> u32 {
        const { assert!(rb::not_supported_words(0x0059_a8c0) == 1) };
        rb::E_NOT_SUPPORTED
    }
    /// `Phyre_PRendering_PRenderInterface_unsupportedState_0059a910`: `0x17` (D3D11 has no such state).
    Phyre_PRendering_PRenderInterface_unsupportedState_0059a910 @ 0x0059_a910 => extern "thiscall" fn n_0059a910(_this: u32, _a0: u32, _a1: u32, _a2: u32) -> u32 {
        const { assert!(rb::not_supported_words(0x0059_a910) == 3) };
        rb::E_NOT_SUPPORTED
    }
    /// `Phyre_PRendering_PRenderInterface_unsupportedState_0059bc30`: `0x17` (D3D11 has no such state).
    Phyre_PRendering_PRenderInterface_unsupportedState_0059bc30 @ 0x0059_bc30 => extern "thiscall" fn n_0059bc30(_this: u32, _a0: u32, _a1: u32, _a2: u32) -> u32 {
        const { assert!(rb::not_supported_words(0x0059_bc30) == 3) };
        rb::E_NOT_SUPPORTED
    }
    /// `Phyre_PRendering_PRenderInterface_unsupportedState_0059bc40`: `0x17` (D3D11 has no such state).
    Phyre_PRendering_PRenderInterface_unsupportedState_0059bc40 @ 0x0059_bc40 => extern "thiscall" fn n_0059bc40(_this: u32, _a0: u32, _a1: u32, _a2: u32) -> u32 {
        const { assert!(rb::not_supported_words(0x0059_bc40) == 3) };
        rb::E_NOT_SUPPORTED
    }
    /// `Phyre_copyConstructElements8_005a8840`: `count` 8-byte elements from `src` to `dst`.
    Phyre_copyConstructElements8_005a8840 @ 0x005a_8840 => extern "C" fn c_005a8840(dst: u32, src: u32, count: u32) {
        b::copy(&mut mem(), dst, src, count, const { rb::copy_size_of(0x005a_8840) });
    }
    /// `Phyre_copyConstructElements8_005bc660`: `count` 8-byte elements from `src` to `dst`.
    Phyre_copyConstructElements8_005bc660 @ 0x005b_c660 => extern "C" fn c_005bc660(dst: u32, src: u32, count: u32) {
        b::copy(&mut mem(), dst, src, count, const { rb::copy_size_of(0x005b_c660) });
    }
    /// `Phyre_copyConstructElements16_005a8800`: `count` 16-byte elements from `src` to `dst`.
    Phyre_copyConstructElements16_005a8800 @ 0x005a_8800 => extern "C" fn c_005a8800(dst: u32, src: u32, count: u32) {
        b::copy(&mut mem(), dst, src, count, const { rb::copy_size_of(0x005a_8800) });
    }
    /// `Phyre_copyConstructElements48_00a2adc0`: `count` 48-byte elements from `src` to `dst`.
    Phyre_copyConstructElements48_00a2adc0 @ 0x00a2_adc0 => extern "C" fn c_00a2adc0(dst: u32, src: u32, count: u32) {
        b::copy(&mut mem(), dst, src, count, const { rb::copy_size_of(0x00a2_adc0) });
    }
    /// `Phyre_zeroConstructElements16_005ad320`: `count` 16-byte elements at `dst` zeroed.
    Phyre_zeroConstructElements16_005ad320 @ 0x005a_d320 => extern "C" fn z_005ad320(dst: u32, count: u32) {
        b::zero16(&mut mem(), dst, count);
    }
}
