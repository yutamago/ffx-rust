//! M5 stage 1, list `phyre-render-b` (`ffx_prender::m5p_render_b*`): the
//! rest of `PRendering`, the post-processing classes and the renderer's
//! display-mode and present code. The parity tests are
//! `crates/ffx-oracle/tests/m5p_render_b.rs`.
//!
//! Threads: the destructors and releases run at exit or while a static
//! initialiser unwinds; the state setters on the render thread. No shim
//! keeps state; each works on the game's memory through [`mem`].

use ffx_prender::m5p_render_b::{self as rb, bridge as b};
use ffx_prender::m5p_render_b_arrays::{self as arr, bridge as ab};
use ffx_prender::m5p_render_b_dtors::bridge as tb;
use ffx_prender::m5p_render_b_post::{self as post, bridge as pb};
use ffx_prender::m5p_render_b_post2::{self as post2, bridge as qb};
use ffx_prender::m5p_render_b_small::{self as small, bridge as sb};

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
    // ---- batch 2: the post-processing effects' small methods (ffx_prender::m5p_render_b_post) ----
    /// `Phyre_PPostProcessing_PPostEffectBase_dtor`: the base vtable.
    Phyre_PPostProcessing_PPostEffectBase_dtor @ 0x009f_e780 => extern "thiscall" fn p_009fe780(this: u32) {
        pb::base_dtor(&mut mem(), this);
    }
    /// `Phyre_PPostProcessing_PPostEffectBase_vslot04_markSetUp`: byte `+0x10` = 1; 0.
    Phyre_PPostProcessing_PPostEffectBase_vslot04_markSetUp @ 0x00a1_9c00 => extern "thiscall" fn p_00a19c00(this: u32, _a: u32, _b: u32) -> u32 {
        pb::mark_set_up(&mut mem(), this)
    }
    /// `Phyre_PPostProcessing_PPostEffectBase_vslot08_useMaterial`: `+0xc` = `+8`; 0.
    Phyre_PPostProcessing_PPostEffectBase_vslot08_useMaterial @ 0x00a1_b810 => extern "thiscall" fn p_00a1b810(this: u32) -> u32 {
        pb::use_material(&mut mem(), this)
    }
    /// `Phyre_PPostProcessing_PDeferredLighting_vslot08_initShaders`.
    Phyre_PPostProcessing_PDeferredLighting_vslot08_initShaders @ 0x00a1_b6e0 => extern "thiscall" fn p_00a1b6e0(this: u32) -> u32 {
        pb::init_shaders(&mut mem(), this, const { post::init_shaders_of(0x00a1_b6e0) })
    }
    /// `Phyre_PPostProcessing_PGlow_vslot08_initShaders`.
    Phyre_PPostProcessing_PGlow_vslot08_initShaders @ 0x00a1_b7b0 => extern "thiscall" fn p_00a1b7b0(this: u32) -> u32 {
        pb::init_shaders(&mut mem(), this, const { post::init_shaders_of(0x00a1_b7b0) })
    }
    /// `Phyre_PPostProcessing_PScreenSpaceReflection_vslot08_initShaders`.
    Phyre_PPostProcessing_PScreenSpaceReflection_vslot08_initShaders @ 0x00a1_b820 => extern "thiscall" fn p_00a1b820(this: u32) -> u32 {
        pb::init_shaders(&mut mem(), this, const { post::init_shaders_of(0x00a1_b820) })
    }
    /// `Phyre_PPostProcessing_PPostEffectBase_vslot0c_render`: 0.
    Phyre_PPostProcessing_PPostEffectBase_vslot0c_render @ 0x00a1_b9a0 => extern "thiscall" fn p_00a1b9a0(_this: u32, _a: u32, _b: u32) -> u32 {
        0
    }
    /// `Phyre_PPostProcessing_PDeferredLighting_vslot0c_render`.
    Phyre_PPostProcessing_PDeferredLighting_vslot0c_render @ 0x00a1_b850 => extern "thiscall" fn p_00a1b850(this: u32, a: u32, b: u32) -> u32 {
        pb::render_if_set_up(&mut mem(), this, const { post::render_of(0x00a1_b850) }, a, b)
    }
    /// `Phyre_PPostProcessing_PGlow_vslot0c_render`.
    Phyre_PPostProcessing_PGlow_vslot0c_render @ 0x00a1_b960 => extern "thiscall" fn p_00a1b960(this: u32, a: u32, b: u32) -> u32 {
        pb::render_if_set_up(&mut mem(), this, const { post::render_of(0x00a1_b960) }, a, b)
    }
    /// `Phyre_PPostProcessing_PScreenSpaceReflection_vslot0c_render`.
    Phyre_PPostProcessing_PScreenSpaceReflection_vslot0c_render @ 0x00a1_b9b0 => extern "thiscall" fn p_00a1b9b0(this: u32, a: u32, b: u32) -> u32 {
        pb::render_if_set_up(&mut mem(), this, const { post::render_of(0x00a1_b9b0) }, a, b)
    }
    /// `Phyre_PPostProcessing_PPostEffectBase_vslot10_canRun`.
    Phyre_PPostProcessing_PPostEffectBase_vslot10_canRun @ 0x00a1_b200 => extern "thiscall" fn p_00a1b200(this: u32) -> u32 {
        pb::can_run(&mut mem(), this)
    }
    /// `Phyre_PPostProcessing_PPostEffectBase_vslot14_runsInPass`.
    Phyre_PPostProcessing_PPostEffectBase_vslot14_runsInPass @ 0x00a1_b190 => extern "thiscall" fn p_00a1b190(this: u32, pass: u32, flag: u32) -> u32 {
        pb::runs_in_pass(&mut mem(), this, pass, flag)
    }
    /// `Phyre_PPostProcessing_PDeferredLighting_vslot14_runsInPass`: `al` = 0.
    Phyre_PPostProcessing_PDeferredLighting_vslot14_runsInPass @ 0x00a1_b180 => extern "thiscall" fn p_00a1b180(_this: u32, _pass: u32, _flag: u32) -> u8 {
        0
    }
    /// `Phyre_PPostProcessing_PScreenSpaceReflection_vslot14_runsInPass`.
    Phyre_PPostProcessing_PScreenSpaceReflection_vslot14_runsInPass @ 0x00a1_b1c0 => extern "thiscall" fn p_00a1b1c0(_this: u32, pass: u32, flag: u32) -> u8 {
        pb::ssr_runs_in_pass(&mut mem(), pass, flag)
    }
    /// `Phyre_PPostProcessing_PPostEffectBase_vslot18_setUpOnce`.
    Phyre_PPostProcessing_PPostEffectBase_vslot18_setUpOnce @ 0x00a2_1cc0 => extern "thiscall" fn p_00a21cc0(this: u32, _a: u32, _b: u32, record: u32, _d: u32, _e: u32) -> u32 {
        pb::set_up_once(&mut mem(), this, record)
    }
    /// `Phyre_PPostProcessing_PUtilityPostProcessing_terminate`.
    Phyre_PPostProcessing_PUtilityPostProcessing_terminate @ 0x00a2_4e70 => extern "thiscall" fn p_00a24e70(_this: u32) -> u32 {
        pb::utility_terminate(&mut mem())
    }
    /// `PostProcessManager_releaseSamplers_00a24d60`.
    PostProcessManager_releaseSamplers_00a24d60 @ 0x00a2_4d60 => extern "thiscall" fn p_00a24d60(this: u32) -> u32 {
        pb::release_samplers(&mut mem(), this)
    }
    /// `Phyre_PFramework_inputWord4Even_009ecc70`.
    Phyre_PFramework_inputWord4Even_009ecc70 @ 0x009e_cc70 => extern "thiscall" fn p_009ecc70(this: u32) -> u32 {
        pb::input_word_even(&mut mem(), this)
    }
    /// `Phyre_PArray_at_009ecc80`.
    Phyre_PArray_at_009ecc80 @ 0x009e_cc80 => extern "thiscall" fn p_009ecc80(this: u32, i: u32) -> u32 {
        pb::array_at(&mut mem(), this, i)
    }
    /// `Phyre_PInputs_PInputAction_entryPlus4_009eca80`.
    Phyre_PInputs_PInputAction_entryPlus4_009eca80 @ 0x009e_ca80 => extern "thiscall" fn p_009eca80(this: u32, i: u32) -> u32 {
        pb::entry_plus4(&mut mem(), this, i)
    }
    /// `Phyre_PInputs_PInputAction_entryWord4_009ecaa0`.
    Phyre_PInputs_PInputAction_entryWord4_009ecaa0 @ 0x009e_caa0 => extern "thiscall" fn p_009ecaa0(this: u32, i: u32) -> u32 {
        pb::entry_word4(&mut mem(), this, i)
    }
    // ---- batch 2: the renderer's small methods (ffx_prender::m5p_render_b_small) ----
    /// `Phyre_PRendering_emptyUnwind_006142f0`: a tail jump to an empty function.
    Phyre_PRendering_emptyUnwind_006142f0 @ 0x0061_42f0 => extern "C" fn s_006142f0() {}
    /// `Phyre_PRendering_PDeferredKick_canKick`.
    Phyre_PRendering_PDeferredKick_canKick @ 0x0061_47e0 => extern "thiscall" fn s_006147e0(this: u32) -> u32 {
        sb::can_kick(&mut mem(), this)
    }
    /// `Phyre_PRendering_clearWord0_00613fb0`.
    Phyre_PRendering_clearWord0_00613fb0 @ 0x0061_3fb0 => extern "thiscall" fn s_00613fb0(this: u32) -> u32 {
        sb::clear_word0(&mut mem(), this)
    }
    /// `Phyre_PArray_initExternal_00613f30`.
    Phyre_PArray_initExternal_00613f30 @ 0x0061_3f30 => extern "thiscall" fn s_00613f30(this: u32, a: u32, b: u32) -> u32 {
        sb::init_external(&mut mem(), this, a, b)
    }
    /// `Phyre_PRendering_clearOwnerWord4_005b4930`.
    Phyre_PRendering_clearOwnerWord4_005b4930 @ 0x005b_4930 => extern "thiscall" fn s_005b4930(this: u32) -> u32 {
        sb::clear_owner_word4(&mut mem(), this)
    }
    /// `Phyre_DynamicMeshDefaultImplmenetation_clearWord4_006e2e20`.
    Phyre_DynamicMeshDefaultImplmenetation_clearWord4_006e2e20 @ 0x006e_2e20 => extern "thiscall" fn s_006e2e20(_this: u32, p: u32) -> u32 {
        sb::clear_word4(&mut mem(), p)
    }
    /// `Phyre_PRendering_ringElementsDtor_006142d0`.
    Phyre_PRendering_ringElementsDtor_006142d0 @ 0x0061_42d0 => extern "thiscall" fn s_006142d0(this: u32) -> u32 {
        sb::ring_elements_dtor(&mut mem(), this)
    }
    /// `Phyre_PRendering_PRenderInterface_setState570_00599810`.
    Phyre_PRendering_PRenderInterface_setState570_00599810 @ 0x0059_9810 => extern "thiscall" fn s_00599810(this: u32, a: u32, b: u32) -> u32 {
        sb::set_state_570(&mut mem(), this, a, b)
    }
    /// `Phyre_PRendering_PRenderInterface_setState5a0_0059a1c0`.
    Phyre_PRendering_PRenderInterface_setState5a0_0059a1c0 @ 0x0059_a1c0 => extern "thiscall" fn s_0059a1c0(this: u32, flag: u32) -> u32 {
        sb::set_5a0(&mut mem(), this, flag)
    }
    /// `Phyre_PRendering_PRenderInterface_setState574_0059a8d0`.
    Phyre_PRendering_PRenderInterface_setState574_0059a8d0 @ 0x0059_a8d0 => extern "thiscall" fn s_0059a8d0(this: u32, mode: u32) -> u32 {
        sb::set_574(&mut mem(), this, mode)
    }
    /// `Phyre_PRendering_PRenderInterface_setColorWriteMask_00599ed0`.
    Phyre_PRendering_PRenderInterface_setColorWriteMask_00599ed0 @ 0x0059_9ed0 => extern "thiscall" fn s_00599ed0(this: u32, r: u32, g: u32, b: u32, a: u32) -> u32 {
        sb::set_write_mask(&mut mem(), this, [r, g, b, a])
    }
    /// `Phyre_PRendering_PRenderInterface_forward3_00615cd0`.
    Phyre_PRendering_PRenderInterface_forward3_00615cd0 @ 0x0061_5cd0 => extern "thiscall" fn s_00615cd0(this: u32, a: u32, x: u32, y: u32) -> u32 {
        sb::forward_3(&mut mem(), small::FWD_3_A, this, a, x, y)
    }
    /// `Phyre_PRendering_PRenderInterface_forward3_00615da0`.
    Phyre_PRendering_PRenderInterface_forward3_00615da0 @ 0x0061_5da0 => extern "thiscall" fn s_00615da0(this: u32, a: u32, x: u32, y: u32) -> u32 {
        sb::forward_3(&mut mem(), small::FWD_3_B, this, a, x, y)
    }
    /// `Phyre_PRendering_PRenderInterface_forward4_00615b50`.
    Phyre_PRendering_PRenderInterface_forward4_00615b50 @ 0x0061_5b50 => extern "thiscall" fn s_00615b50(this: u32, x: u32, y: u32, z: u32, w: u32) -> u32 {
        sb::forward_4(&mut mem(), this, [x, y, z, w])
    }
    /// `Phyre_PRendering_applyPassState_0061c870`.
    Phyre_PRendering_applyPassState_0061c870 @ 0x0061_c870 => extern "thiscall" fn s_0061c870(this: u32) -> u32 {
        sb::apply_pass_state(&mut mem(), this)
    }
    /// `Phyre_PRendering_PRenderInterface_contextCallA4_00592b00`.
    Phyre_PRendering_PRenderInterface_contextCallA4_00592b00 @ 0x0059_2b00 => extern "thiscall" fn s_00592b00(this: u32, a: u32, b: u32, c: u32) -> u32 {
        sb::com_a4(&mut mem(), this, a, b, c)
    }
    /// `Phyre_PRendering_releaseComPairs_0059c330`.
    Phyre_PRendering_releaseComPairs_0059c330 @ 0x0059_c330 => extern "thiscall" fn s_0059c330(this: u32) {
        sb::release_pairs(&mut mem(), this);
    }
    /// `Phyre_PRendering_PRendererBase_closeSection_005bc440`.
    Phyre_PRendering_PRendererBase_closeSection_005bc440 @ 0x005b_c440 => extern "thiscall" fn s_005bc440(this: u32) -> u32 {
        sb::close_section(&mut mem(), this).unwrap_or(0)
    }
    /// `Phyre_PRendering_PRenderer_closeSection_005bc400`.
    Phyre_PRendering_PRenderer_closeSection_005bc400 @ 0x005b_c400 => extern "thiscall" fn s_005bc400(this: u32) -> u32 {
        sb::sync_and_close(&mut mem(), this)
    }
    /// `Phyre_PRendering_PRendererBase_generateSceneWideParameterCaptureBuffer`.
    Phyre_PRendering_PRendererBase_generateSceneWideParameterCaptureBuffer @ 0x005b_86d0 => extern "thiscall" fn s_005b86d0(this: u32) -> u32 {
        sb::scene_wide_capture(&mut mem(), this)
    }
    /// `Phyre_PRendering_sharrayWordC_00593fe0`.
    Phyre_PRendering_sharrayWordC_00593fe0 @ 0x0059_3fe0 => extern "thiscall" fn s_00593fe0(this: u32, i: u32) -> u32 {
        sb::sharray_word_c(&mut mem(), this, i)
    }
    /// `Phyre_PRendering_freeJobs_005b5330`.
    Phyre_PRendering_freeJobs_005b5330 @ 0x005b_5330 => extern "C" fn s_005b5330(jobs: u32, n: u32) {
        sb::free_jobs(&mut mem(), jobs, n);
    }
    /// `Phyre_PRendering_PInternal_PRendererThreadPoolJob_deleting_dtor`.
    Phyre_PRendering_PInternal_PRendererThreadPoolJob_deleting_dtor @ 0x005b_5080 => extern "thiscall" fn s_005b5080(this: u32, flags: u32) -> u32 {
        sb::job_delete(&mut mem(), this, flags, small::RENDERER_JOB_DTOR)
    }
    /// `Phyre_PRingBufferedThreadPoolJob_deleting_dtor`.
    Phyre_PRingBufferedThreadPoolJob_deleting_dtor @ 0x005b_50b0 => extern "thiscall" fn s_005b50b0(this: u32, flags: u32) -> u32 {
        sb::job_delete(&mut mem(), this, flags, small::RING_JOB_DTOR)
    }
    /// `Phyre_PAnimation_PNetworkOptimizeBufferAllocator_PBufferAcceptance_deleting_dtor`.
    Phyre_PAnimation_PNetworkOptimizeBufferAllocator_PBufferAcceptance_deleting_dtor @ 0x0061_6170 => extern "thiscall" fn s_00616170(this: u32, flags: u32) -> u32 {
        sb::acceptance_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PAnimation_PNetworkOptimizeBufferAllocator_deleting_dtor`.
    Phyre_PAnimation_PNetworkOptimizeBufferAllocator_deleting_dtor @ 0x0061_61a0 => extern "thiscall" fn s_006161a0(this: u32, flags: u32) -> u32 {
        sb::allocator_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PAnimation_PInternal_PAvoidOutputOverwritingInput_deleting_dtor`.
    Phyre_PAnimation_PInternal_PAvoidOutputOverwritingInput_deleting_dtor @ 0x0061_65c0 => extern "thiscall" fn s_006165c0(this: u32, flags: u32) -> u32 {
        sb::avoid_overwrite_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PAnimation_PInternal_PAvoidOutputOverwritingInput_ctor`.
    Phyre_PAnimation_PInternal_PAvoidOutputOverwritingInput_ctor @ 0x0061_64a0 => extern "thiscall" fn s_006164a0(this: u32, a: u32, b: u32) -> u32 {
        sb::avoid_overwrite_ctor(&mut mem(), this, a, b)
    }
    /// `Phyre_CharacterLighting_destroyInstance_006a6dd0`.
    Phyre_CharacterLighting_destroyInstance_006a6dd0 @ 0x006a_6dd0 => extern "C" fn s_006a6dd0() {
        sb::character_lighting_finalize(&mut mem());
    }
    /// `Phyre_CharacterLightingTweakUtil_destroyInstance_006a6e00`.
    Phyre_CharacterLightingTweakUtil_destroyInstance_006a6e00 @ 0x006a_6e00 => extern "C" fn s_006a6e00() {
        sb::tweak_util_finalize(&mut mem());
    }
    /// `Phyre_FogEffectManager_deleteInstance_006e32a0`.
    Phyre_FogEffectManager_deleteInstance_006e32a0 @ 0x006e_32a0 => extern "C" fn s_006e32a0() -> u32 {
        sb::fog_manager_delete(&mut mem())
    }
    /// `Phyre_TextUtil_release_006ab5c0`.
    Phyre_TextUtil_release_006ab5c0 @ 0x006a_b5c0 => extern "thiscall" fn s_006ab5c0(this: u32) -> u32 {
        sb::text_util_release(&mut mem(), this)
    }
    /// `Phyre_FogEffectManager_setChrFogFadeType`.
    Phyre_FogEffectManager_setChrFogFadeType @ 0x006e_36c0 => extern "thiscall" fn s_006e36c0(this: u32, kind: u32) -> u32 {
        sb::set_fog_fade_type(&mut mem(), this, kind)
    }
    /// `Phyre_PIggy_PUtilityIggy_terminate`.
    Phyre_PIggy_PUtilityIggy_terminate @ 0x00a3_e720 => extern "thiscall" fn s_00a3e720(_this: u32) -> u32 {
        sb::iggy_terminate(&mut mem())
    }
    /// `Phyre_PIggy_warningCallback_00a3e5e0`.
    Phyre_PIggy_warningCallback_00a3e5e0 @ 0x00a3_e5e0 => extern "stdcall" fn s_00a3e5e0(_a: u32, _b: u32, message: u32, _d: u32) -> u32 {
        sb::iggy_warning(&mut mem(), message)
    }
    /// `Phyre_PText_PBitmapFontGlyph_ctor_00a2bab0`.
    Phyre_PText_PBitmapFontGlyph_ctor_00a2bab0 @ 0x00a2_bab0 => extern "thiscall" fn s_00a2bab0(this: u32) -> u32 {
        sb::glyph_ctor(&mut mem(), this)
    }
    /// `Phyre_PText_PBitmapFontGlyphArray_find_00a29320`.
    Phyre_PText_PBitmapFontGlyphArray_find_00a29320 @ 0x00a2_9320 => extern "thiscall" fn s_00a29320(this: u32, key: u32, first: u32) -> u32 {
        sb::glyph_find(&mut mem(), this, key, first)
    }
    /// `Phyre_PText_PBitmapFontText_renderInstance_00a2a400`.
    Phyre_PText_PBitmapFontText_renderInstance_00a2a400 @ 0x00a2_a400 => extern "thiscall" fn s_00a2a400(this: u32, inst: u32) -> u32 {
        sb::text_render(&mut mem(), this, inst)
    }
    /// `Phyre_PDynamicGeometry_stringWithLength_005c8580`.
    Phyre_PDynamicGeometry_stringWithLength_005c8580 @ 0x005c_8580 => extern "C" fn s_005c8580(pp: u32, out: u32, len: u32) -> u32 {
        sb::string_with_length(&mut mem(), pp, out, len)
    }
    // ---- batch 3: arrays, command hand-over, utilities, fog (ffx_prender::m5p_render_b_arrays) ----
    /// `Phyre_copyConstructElements20_005bc610`: `count` 20-byte elements from `src` to `dst`.
    Phyre_copyConstructElements20_005bc610 @ 0x005b_c610 => extern "C" fn c_005bc610(dst: u32, src: u32, count: u32) {
        b::copy(&mut mem(), dst, src, count, const { rb::copy_size_of(0x005b_c610) });
    }
    /// `Phyre_PSharray_field_00593ca0`.
    Phyre_PSharray_field_00593ca0 @ 0x0059_3ca0 => extern "thiscall" fn a_00593ca0(this: u32, i: u32) -> u32 {
        ab::sh_field(&mut mem(), this, i, const { arr::sh_field_of(0x0059_3ca0) })
    }
    /// `Phyre_PSharray_field_00593d10`.
    Phyre_PSharray_field_00593d10 @ 0x0059_3d10 => extern "thiscall" fn a_00593d10(this: u32, i: u32) -> u32 {
        ab::sh_field(&mut mem(), this, i, const { arr::sh_field_of(0x0059_3d10) })
    }
    /// `Phyre_PSharray_field_00593d60`.
    Phyre_PSharray_field_00593d60 @ 0x0059_3d60 => extern "thiscall" fn a_00593d60(this: u32, i: u32) -> u32 {
        ab::sh_field(&mut mem(), this, i, const { arr::sh_field_of(0x0059_3d60) })
    }
    /// `Phyre_PSharray_field_00593de0`.
    Phyre_PSharray_field_00593de0 @ 0x0059_3de0 => extern "thiscall" fn a_00593de0(this: u32, i: u32) -> u32 {
        ab::sh_field(&mut mem(), this, i, const { arr::sh_field_of(0x0059_3de0) })
    }
    /// `Phyre_PSharray_field_00593eb0`.
    Phyre_PSharray_field_00593eb0 @ 0x0059_3eb0 => extern "thiscall" fn a_00593eb0(this: u32, i: u32) -> u32 {
        ab::sh_field(&mut mem(), this, i, const { arr::sh_field_of(0x0059_3eb0) })
    }
    /// `Phyre_PSharray_field_00593f00`.
    Phyre_PSharray_field_00593f00 @ 0x0059_3f00 => extern "thiscall" fn a_00593f00(this: u32, i: u32) -> u32 {
        ab::sh_field(&mut mem(), this, i, const { arr::sh_field_of(0x0059_3f00) })
    }
    /// `Phyre_PSharray_field_00593f90`.
    Phyre_PSharray_field_00593f90 @ 0x0059_3f90 => extern "thiscall" fn a_00593f90(this: u32, i: u32) -> u32 {
        ab::sh_field(&mut mem(), this, i, const { arr::sh_field_of(0x0059_3f90) })
    }
    /// `Phyre_PSharray_field_00594010`.
    Phyre_PSharray_field_00594010 @ 0x0059_4010 => extern "thiscall" fn a_00594010(this: u32, i: u32) -> u32 {
        ab::sh_field(&mut mem(), this, i, const { arr::sh_field_of(0x0059_4010) })
    }
    /// `Phyre_PSharray_field_00594730`.
    Phyre_PSharray_field_00594730 @ 0x0059_4730 => extern "thiscall" fn a_00594730(this: u32, i: u32) -> u32 {
        ab::sh_field(&mut mem(), this, i, const { arr::sh_field_of(0x0059_4730) })
    }
    /// `Phyre_PSharray_field_00594780`.
    Phyre_PSharray_field_00594780 @ 0x0059_4780 => extern "thiscall" fn a_00594780(this: u32, i: u32) -> u32 {
        ab::sh_field(&mut mem(), this, i, const { arr::sh_field_of(0x0059_4780) })
    }
    /// `Phyre_PSharray_field_00594830`.
    Phyre_PSharray_field_00594830 @ 0x0059_4830 => extern "thiscall" fn a_00594830(this: u32, i: u32) -> u32 {
        ab::sh_field(&mut mem(), this, i, const { arr::sh_field_of(0x0059_4830) })
    }
    /// `Phyre_PRendering_PRendererBase_textureSlot_005b8840`.
    Phyre_PRendering_PRendererBase_textureSlot_005b8840 @ 0x005b_8840 => extern "thiscall" fn a_005b8840(this: u32, i: u32) -> u32 {
        ab::sh_field(&mut mem(), this, i, const { arr::sh_field_of(0x005b_8840) })
    }
    /// `Phyre_PArray_allocCopy_005aafb0`.
    Phyre_PArray_allocCopy_005aafb0 @ 0x005a_afb0 => extern "C" fn a_005aafb0(n: u32, src: u32, m: u32) -> u32 {
        ab::alloc_copy(&mut mem(), n, src, m, const { arr::fill_of(0x005a_afb0) })
    }
    /// `Phyre_PArray_allocCopy_005b1320`.
    Phyre_PArray_allocCopy_005b1320 @ 0x005b_1320 => extern "C" fn a_005b1320(n: u32, src: u32, m: u32) -> u32 {
        ab::alloc_copy(&mut mem(), n, src, m, const { arr::fill_of(0x005b_1320) })
    }
    /// `Phyre_PArray_allocCopy_005ab070`.
    Phyre_PArray_allocCopy_005ab070 @ 0x005a_b070 => extern "C" fn a_005ab070(n: u32, src: u32, m: u32) -> u32 {
        ab::alloc_copy(&mut mem(), n, src, m, const { arr::fill_of(0x005a_b070) })
    }
    /// `Phyre_PArray_allocCopy_005c0720`.
    Phyre_PArray_allocCopy_005c0720 @ 0x005c_0720 => extern "C" fn a_005c0720(n: u32, src: u32, m: u32) -> u32 {
        ab::alloc_copy(&mut mem(), n, src, m, const { arr::fill_of(0x005c_0720) })
    }
    /// `Phyre_PArray_resize_005af000`.
    Phyre_PArray_resize_005af000 @ 0x005a_f000 => extern "thiscall" fn a_005af000(this: u32, n: u32) -> u32 {
        ab::resize_u8p(&mut mem(), this, n)
    }
    /// `Phyre_PSharray_allocCopy28_0058c590`.
    Phyre_PSharray_allocCopy28_0058c590 @ 0x0058_c590 => extern "thiscall" fn a_0058c590(this: u32, n: u32, src: u32, m: u32) -> u32 {
        ab::alloc_copy_28(&mut mem(), this, n, src, m)
    }
    /// `Phyre_PPhysics_copyMatrix_006041e0`.
    Phyre_PPhysics_copyMatrix_006041e0 @ 0x0060_41e0 => extern "thiscall" fn a_006041e0(this: u32, src: u32) -> u32 {
        ab::copy_16_words(&mut mem(), this, src)
    }
    /// `Phyre_PPostProcessing_PMeshParticleSystemBase_zero3040_009fc970`.
    Phyre_PPostProcessing_PMeshParticleSystemBase_zero3040_009fc970 @ 0x009f_c970 => extern "thiscall" fn a_009fc970(this: u32) -> u32 {
        ab::zero_3040(&mut mem(), this)
    }
    /// `Phyre_PRendering_initRecords_005b4440`.
    Phyre_PRendering_initRecords_005b4440 @ 0x005b_4440 => extern "thiscall" fn a_005b4440(this: u32, a: u32, b: u32, c: u32) -> u32 {
        ab::init_records(&mut mem(), this, a, b, c)
    }
    /// `Phyre_PRendering_PRenderThread_handOver_006152a0`.
    Phyre_PRendering_PRenderThread_handOver_006152a0 @ 0x0061_52a0 => extern "thiscall" fn a_006152a0(_this: u32, slot: u32, b: u32, c: u32, d: u32) -> u32 {
        ab::hand_over(&mut mem(), slot, b, c, d)
    }
    /// `Phyre_PRendering_PRendererForJobs_flushCommands_005b6ca0`.
    Phyre_PRendering_PRendererForJobs_flushCommands_005b6ca0 @ 0x005b_6ca0 => extern "thiscall" fn a_005b6ca0(this: u32) -> u32 {
        ab::flush_commands(&mut mem(), this)
    }
    /// `Phyre_PArray_setExternalBuffer_00a23be0`.
    Phyre_PArray_setExternalBuffer_00a23be0 @ 0x00a2_3be0 => extern "thiscall" fn a_00a23be0(this: u32, buffer: u32, n: u32) -> u32 {
        ab::set_external(&mut mem(), this, buffer, n)
    }
    /// `Phyre_PSharray_setExternalBuffer_00603dc0`.
    Phyre_PSharray_setExternalBuffer_00603dc0 @ 0x0060_3dc0 => extern "thiscall" fn a_00603dc0(this: u32, buffer: u32, n: u32) -> u32 {
        ab::sharray_set_external(&mut mem(), this, buffer, n)
    }
    /// `Phyre_PSharray_Phyre_PPhysics_PPhysicsShape__setBuffer_00603e50`.
    Phyre_PSharray_Phyre_PPhysics_PPhysicsShape__setBuffer_00603e50 @ 0x0060_3e50 => extern "thiscall" fn a_00603e50(this: u32, buffer: u32, n: u32) -> u32 {
        ab::shapes_set_buffer(&mut mem(), this, buffer, n)
    }
    /// `Phyre_PPostProcessing_PUtilityPostProcessing_deleting_dtor`.
    Phyre_PPostProcessing_PUtilityPostProcessing_deleting_dtor @ 0x00a0_3140 => extern "thiscall" fn a_00a03140(this: u32, flags: u32) -> u32 {
        ab::utility_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PText_PUtilityText_deleting_dtor`.
    Phyre_PText_PUtilityText_deleting_dtor @ 0x00a3_e800 => extern "thiscall" fn a_00a3e800(this: u32, flags: u32) -> u32 {
        ab::utility_delete(&mut mem(), this, flags)
    }
    /// `PostProcessManager_releaseResources_00a24c90`.
    PostProcessManager_releaseResources_00a24c90 @ 0x00a2_4c90 => extern "thiscall" fn a_00a24c90(this: u32) {
        ab::release_post_resources(&mut mem(), this);
    }
    /// `Phyre_FogEffectManager_setColour_006e36f0`.
    Phyre_FogEffectManager_setColour_006e36f0 @ 0x006e_36f0 => extern "thiscall" fn a_006e36f0(fog: u32, r: u32, g: u32, b: u32) -> u32 {
        ab::fog_set(&mut mem(), fog, 0x10, &[r, g, b])
    }
    /// `Phyre_FogEffectManager_setParam28_006e3760`.
    Phyre_FogEffectManager_setParam28_006e3760 @ 0x006e_3760 => extern "thiscall" fn a_006e3760(fog: u32, v: u32) -> u32 {
        ab::fog_set(&mut mem(), fog, const { arr::fog_field_of(0x006e_3760) }, &[v])
    }
    /// `Phyre_FogEffectManager_setParam24_006e3a30`.
    Phyre_FogEffectManager_setParam24_006e3a30 @ 0x006e_3a30 => extern "thiscall" fn a_006e3a30(fog: u32, v: u32) -> u32 {
        ab::fog_set(&mut mem(), fog, const { arr::fog_field_of(0x006e_3a30) }, &[v])
    }
    /// `Phyre_FogEffectManager_setParam20_006e3b10`.
    Phyre_FogEffectManager_setParam20_006e3b10 @ 0x006e_3b10 => extern "thiscall" fn a_006e3b10(fog: u32, v: u32) -> u32 {
        ab::fog_set(&mut mem(), fog, const { arr::fog_field_of(0x006e_3b10) }, &[v])
    }
    // ---- batch 4: post-processing setups, entries, target resizes, dtors (ffx_prender::m5p_render_b_post2) ----
    /// `Phyre_PPostProcessing_PDepthOfField_vslot04_setup`.
    Phyre_PPostProcessing_PDepthOfField_vslot04_setup @ 0x00a1_98d0 => extern "thiscall" fn q_00a198d0(this: u32, w: u32, h: u32) -> u32 {
        qb::setup(&mut mem(), this, const { post2::setup_of(0x00a1_98d0) }, w, h)
    }
    /// `Phyre_PPostProcessing_PGlow_vslot04_setup`.
    Phyre_PPostProcessing_PGlow_vslot04_setup @ 0x00a1_9ac0 => extern "thiscall" fn q_00a19ac0(this: u32, w: u32, h: u32) -> u32 {
        qb::setup(&mut mem(), this, const { post2::setup_of(0x00a1_9ac0) }, w, h)
    }
    /// `Phyre_PPostProcessing_PMotionBlur_vslot04_setup`.
    Phyre_PPostProcessing_PMotionBlur_vslot04_setup @ 0x00a1_9b60 => extern "thiscall" fn q_00a19b60(this: u32, w: u32, h: u32) -> u32 {
        qb::setup(&mut mem(), this, const { post2::setup_of(0x00a1_9b60) }, w, h)
    }
    /// `Phyre_PPostProcessing_PScreenSpaceReflection_vslot04_setup`.
    Phyre_PPostProcessing_PScreenSpaceReflection_vslot04_setup @ 0x00a1_9c10 => extern "thiscall" fn q_00a19c10(this: u32, w: u32, h: u32) -> u32 {
        qb::setup(&mut mem(), this, const { post2::setup_of(0x00a1_9c10) }, w, h)
    }
    /// `Phyre_PPostProcessing_PDeferredLighting_vslot18_render`.
    Phyre_PPostProcessing_PDeferredLighting_vslot18_render @ 0x00a2_1a20 => extern "thiscall" fn q_00a21a20(this: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        qb::entry(&mut mem(), this, const { post2::entry_of(0x00a2_1a20) }, [a, b, c, d, e])
    }
    /// `Phyre_PPostProcessing_PDepthOfField_vslot18_render`.
    Phyre_PPostProcessing_PDepthOfField_vslot18_render @ 0x00a2_1aa0 => extern "thiscall" fn q_00a21aa0(this: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        qb::entry(&mut mem(), this, const { post2::entry_of(0x00a2_1aa0) }, [a, b, c, d, e])
    }
    /// `Phyre_PPostProcessing_PFXAA_vslot18_render`.
    Phyre_PPostProcessing_PFXAA_vslot18_render @ 0x00a2_1b20 => extern "thiscall" fn q_00a21b20(this: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        qb::entry(&mut mem(), this, const { post2::entry_of(0x00a2_1b20) }, [a, b, c, d, e])
    }
    /// `Phyre_PPostProcessing_PScreenSpaceReflection_vslot18_render`.
    Phyre_PPostProcessing_PScreenSpaceReflection_vslot18_render @ 0x00a2_1cf0 => extern "thiscall" fn q_00a21cf0(this: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        qb::entry(&mut mem(), this, const { post2::entry_of(0x00a2_1cf0) }, [a, b, c, d, e])
    }
    /// `Phyre_PPostProcessing_PGlow_vslot18_render`.
    Phyre_PPostProcessing_PGlow_vslot18_render @ 0x00a2_1ba0 => extern "thiscall" fn q_00a21ba0(this: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        qb::entry(&mut mem(), this, const { post2::entry_of(0x00a2_1ba0) }, [a, b, c, d, e])
    }
    /// `Phyre_PPostProcessing_resizeTarget5c_00a23200`.
    Phyre_PPostProcessing_resizeTarget5c_00a23200 @ 0x00a2_3200 => extern "thiscall" fn q_00a23200(this: u32, w: u32, h: u32) -> u32 {
        qb::resize_5c(&mut mem(), this, w, h)
    }
    /// `Phyre_PPostProcessing_PFXAA_vslot0c_resize`.
    Phyre_PPostProcessing_PFXAA_vslot0c_resize @ 0x00a1_b8f0 => extern "thiscall" fn q_00a1b8f0(this: u32, w: u32, h: u32) -> u32 {
        qb::resize_5c_if_set_up(&mut mem(), this, w, h)
    }
    /// `Phyre_PPostProcessing_PDepthOfField_resizeHalves_00a23080`.
    Phyre_PPostProcessing_PDepthOfField_resizeHalves_00a23080 @ 0x00a2_3080 => extern "thiscall" fn q_00a23080(this: u32, w: u32, h: u32) -> u32 {
        qb::resize_halves(&mut mem(), this, w, h)
    }
    /// `Phyre_PPostProcessing_PDepthOfField_vslot0c_resize`.
    Phyre_PPostProcessing_PDepthOfField_vslot0c_resize @ 0x00a1_b870 => extern "thiscall" fn q_00a1b870(this: u32, w: u32, h: u32) -> u32 {
        qb::resize_depth_of_field(&mut mem(), this, w, h)
    }
    /// `Phyre_PPostProcessing_PDeferredLightingBase_releaseResources_00a24a60`.
    Phyre_PPostProcessing_PDeferredLightingBase_releaseResources_00a24a60 @ 0x00a2_4a60 => extern "thiscall" fn q_00a24a60(this: u32) {
        qb::lighting_release(&mut mem(), this);
    }
    /// `Phyre_PPostProcessing_PDeferredLightingBase_dtor_009fdb80`.
    Phyre_PPostProcessing_PDeferredLightingBase_dtor_009fdb80 @ 0x009f_db80 => extern "thiscall" fn q_009fdb80(this: u32) -> u32 {
        qb::lighting_dtor(&mut mem(), this)
    }
    /// `Phyre_PPostProcessing_PDepthOfField_dtor_009fde10`.
    Phyre_PPostProcessing_PDepthOfField_dtor_009fde10 @ 0x009f_de10 => extern "thiscall" fn q_009fde10(this: u32) -> u32 {
        qb::depth_of_field_dtor(&mut mem(), this)
    }
    /// `Phyre_PPostProcessing_PMeshParticleSystemBase_deleting_dtor`.
    Phyre_PPostProcessing_PMeshParticleSystemBase_deleting_dtor @ 0x00a0_2d90 => extern "thiscall" fn q_00a02d90(this: u32, flags: u32) -> u32 {
        qb::particles_delete(&mut mem(), this, flags)
    }
    // ---- batch 4: renderer dtors, state setters, trees (ffx_prender::m5p_render_b_dtors) ----
    /// `Phyre_PRedBlackTree_dtor_006a6720`.
    Phyre_PRedBlackTree_dtor_006a6720 @ 0x006a_6720 => extern "thiscall" fn t_006a6720(this: u32) -> u32 {
        tb::tree_clear_twice(&mut mem(), this)
    }
    /// `Phyre_PRedBlackTree_dtor_006a6780`.
    Phyre_PRedBlackTree_dtor_006a6780 @ 0x006a_6780 => extern "thiscall" fn t_006a6780(this: u32) -> u32 {
        tb::tree_clear_twice(&mut mem(), this)
    }
    /// `Phyre_PRedBlackTreeMap_dtor_006a67e0`.
    Phyre_PRedBlackTreeMap_dtor_006a67e0 @ 0x006a_67e0 => extern "thiscall" fn t_006a67e0(this: u32) -> u32 {
        tb::tree_map_dtor(&mut mem(), this)
    }
    /// `Phyre_PRedBlackTreeMap_removeAllDtor_006a68f0`.
    Phyre_PRedBlackTreeMap_removeAllDtor_006a68f0 @ 0x006a_68f0 => extern "thiscall" fn t_006a68f0(this: u32) -> u32 {
        tb::tree_map_remove_all(&mut mem(), this)
    }
    /// `Phyre_PRendering_PooledHandle_dtor_00614300`.
    Phyre_PRendering_PooledHandle_dtor_00614300 @ 0x0061_4300 => extern "thiscall" fn t_00614300(this: u32) -> u32 {
        tb::handle_dtor(&mut mem(), this)
    }
    /// `Phyre_PRendering_PRendererBase_dtor_005b49e0`.
    Phyre_PRendering_PRendererBase_dtor_005b49e0 @ 0x005b_49e0 => extern "thiscall" fn t_005b49e0(this: u32) {
        tb::renderer_base_dtor(&mut mem(), this, 0);
    }
    /// `Phyre_PRendering_PRendererBase_deleting_dtor`.
    Phyre_PRendering_PRendererBase_deleting_dtor @ 0x005b_4fa0 => extern "thiscall" fn t_005b4fa0(this: u32, flags: u32) -> u32 {
        tb::renderer_base_dtor(&mut mem(), this, flags)
    }
    /// `Phyre_PRendering_PRendererForJobs_deleting_dtor`.
    Phyre_PRendering_PRendererForJobs_deleting_dtor @ 0x005b_5010 => extern "thiscall" fn t_005b5010(this: u32, flags: u32) -> u32 {
        tb::renderer_base_dtor(&mut mem(), this, flags)
    }
    /// `Phyre_PRendering_PRenderer_dtor_005b4950`.
    Phyre_PRendering_PRenderer_dtor_005b4950 @ 0x005b_4950 => extern "thiscall" fn t_005b4950(this: u32) {
        tb::renderer_dtor(&mut mem(), this, 0);
    }
    /// `Phyre_PRendering_PRenderer_deleting_dtor`.
    Phyre_PRendering_PRenderer_deleting_dtor @ 0x005b_4ef0 => extern "thiscall" fn t_005b4ef0(this: u32, flags: u32) -> u32 {
        tb::renderer_dtor(&mut mem(), this, flags)
    }
    /// `Phyre_PRendering_PInternal_PRendererThreadPoolJob_ctor_005b4750`.
    Phyre_PRendering_PInternal_PRendererThreadPoolJob_ctor_005b4750 @ 0x005b_4750 => extern "thiscall" fn t_005b4750(this: u32) -> u32 {
        tb::job_ctor(&mut mem(), this)
    }
    /// `Phyre_PRendering_PInternal_PRendererThreadPoolJob_new_005b3680`.
    Phyre_PRendering_PInternal_PRendererThreadPoolJob_new_005b3680 @ 0x005b_3680 => extern "C" fn t_005b3680() -> u32 {
        tb::new_job(&mut mem())
    }
    /// `Phyre_PDynamicGeometry_recordsDtor_005c82a0`.
    Phyre_PDynamicGeometry_recordsDtor_005c82a0 @ 0x005c_82a0 => extern "thiscall" fn t_005c82a0(this: u32) {
        tb::records_dtor(&mut mem(), this);
    }
    /// `Phyre_PRendering_PRenderInterface_setModes_00599820`.
    Phyre_PRendering_PRenderInterface_setModes_00599820 @ 0x0059_9820 => extern "thiscall" fn t_00599820(this: u32, a: u32, b: u32) -> u32 {
        tb::set_modes(&mut mem(), this, a, b)
    }
    /// `Phyre_PRendering_PRenderInterface_setState578_0059a070`.
    Phyre_PRendering_PRenderInterface_setState578_0059a070 @ 0x0059_a070 => extern "thiscall" fn t_0059a070(this: u32, mode: u32) -> u32 {
        tb::set_578(&mut mem(), this, mode)
    }
    /// `Phyre_PRendering_PRenderInterface_setBlendState_00599950`.
    Phyre_PRendering_PRenderInterface_setBlendState_00599950 @ 0x0059_9950 => extern "thiscall" fn t_00599950(this: u32, enable: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        tb::set_blend(&mut mem(), this, enable, [a, b, c, d])
    }
    /// `Phyre_PRendering_PRenderInterface_setDepthStencilState_0059bac0`.
    Phyre_PRendering_PRenderInterface_setDepthStencilState_0059bac0 @ 0x0059_bac0 => extern "thiscall" fn t_0059bac0(this: u32, w0: u32, w1: u32, _w2: u32, w3: u32, w4: u32, w5: u32, w6: u32, w7: u32, w8: u32, _w9: u32, _w10: u32, w11: u32, w12: u32, w13: u32, _w14: u32) -> u32 {
        tb::set_depth_stencil(&mut mem(), this, [w0, w1, 0, w3, w4, w5, w6, w7, w8, 0, 0, w11, w12, w13, 0])
    }
    /// `PShaderD3D11_release_0059c190`.
    PShaderD3D11_release_0059c190 @ 0x0059_c190 => extern "thiscall" fn t_0059c190(this: u32) -> u32 {
        tb::shader_release(&mut mem(), this)
    }
    /// `Phyre_PRendering_releaseComEight_0059c370`.
    Phyre_PRendering_releaseComEight_0059c370 @ 0x0059_c370 => extern "thiscall" fn t_0059c370(this: u32) {
        tb::release_eight(&mut mem(), this);
    }
    /// `Phyre_CharacterLightingTweakUtil_setEnabled`.
    Phyre_CharacterLightingTweakUtil_setEnabled @ 0x006a_a5f0 => extern "thiscall" fn t_006aa5f0(this: u32, on: u32) -> u32 {
        tb::tweak_set_enabled(&mut mem(), this, on)
    }
    /// `Phyre_CharacterLightingTweakUtil_release_006ab540`.
    Phyre_CharacterLightingTweakUtil_release_006ab540 @ 0x006a_b540 => extern "thiscall" fn t_006ab540(this: u32) -> u32 {
        tb::tweak_release(&mut mem(), this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PText_PBitmapFontCharInfo_4__swapEndiannessStatic1`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PText_PBitmapFontCharInfo_4__swapEndiannessStatic1 @ 0x00a2_b3b0 => extern "C" fn t_00a2b3b0(p: u32) -> u32 {
        tb::char_info_swap(&mut mem(), p, 1)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PText_PBitmapFontCharInfo_4__swapEndiannessStatic0`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PText_PBitmapFontCharInfo_4__swapEndiannessStatic0 @ 0x00a2_b510 => extern "C" fn t_00a2b510(p: u32) -> u32 {
        tb::char_info_swap(&mut mem(), p, 0)
    }
    /// `Phyre_PArray_resize16_005af070`.
    Phyre_PArray_resize16_005af070 @ 0x005a_f070 => extern "thiscall" fn t_005af070(this: u32, n: u32) -> u32 {
        tb::resize16(&mut mem(), this, n)
    }
}
