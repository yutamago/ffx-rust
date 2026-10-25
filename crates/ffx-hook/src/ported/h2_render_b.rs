//! `ffx-prender`, M4 port round 1, list h2-render-b: `PhyreEngine`'s
//! render code (the type system's descriptor and array-member slots of
//! the render classes, small render-interface, material and shader
//! methods). Checked by `crates/ffx-oracle/tests/h2_render_b.rs`.
//!
//! Threads: the descriptor slots run where the serializer loads (the main
//! and loader threads), the render-interface setters on the render
//! thread. No shim keeps state; nothing here has a frame.

use super::{mem, ported};
use ffx_prender::descriptor::{Init, bridge as d};
use ffx_prender::misc::bridge as x;

ported! {
    /// `Phyre_PClassDescriptorAbstract_Phyre_PRendering_PMaterial__fixObjectsAfterLoading`.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PMaterial__fixObjectsAfterLoading @ 0x004d_8760 => extern "thiscall" fn s_004d8760(_a0: u32, _a1: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PRendering_PMaterialSwitch__fixObjectsAfterLoading`.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PMaterialSwitch__fixObjectsAfterLoading @ 0x004d_8770 => extern "thiscall" fn s_004d8770(_a0: u32, _a1: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PRendering_PSceneRenderPass__fixObjectsAfterLoading`.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PSceneRenderPass__fixObjectsAfterLoading @ 0x004e_0150 => extern "thiscall" fn s_004e0150(_a0: u32, _a1: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PRendering_PShaderPassInfo__fixObjectsAfterLoading`.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PShaderPassInfo__fixObjectsAfterLoading @ 0x004e_0160 => extern "thiscall" fn s_004e0160(_a0: u32, _a1: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PRendering_PMeshInstance__fixObjectsAfterLoading`.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PMeshInstance__fixObjectsAfterLoading @ 0x004f_7830 => extern "thiscall" fn s_004f7830(_a0: u32, _a1: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PRendering_PMeshInstanceSegmentContext__fixObjectsAfterLoading`.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PMeshInstanceSegmentContext__fixObjectsAfterLoading @ 0x004f_7840 => extern "thiscall" fn s_004f7840(_a0: u32, _a1: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PRendering_PMeshInstanceSegmentStreamBinding__fixObjectsAfterLoading`.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PMeshInstanceSegmentStreamBinding__fixObjectsAfterLoading @ 0x004f_7850 => extern "thiscall" fn s_004f7850(_a0: u32, _a1: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PRendering_PModifierNetworkDynamicMeshInstance__fixObjectsAfterLoading`.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PModifierNetworkDynamicMeshInstance__fixObjectsAfterLoading @ 0x004f_c740 => extern "thiscall" fn s_004fc740(_a0: u32, _a1: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PRendering_PMeshInstanceBounds__fixObjectsAfterLoading`.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PMeshInstanceBounds__fixObjectsAfterLoading @ 0x0050_0710 => extern "thiscall" fn s_00500710(_a0: u32, _a1: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PSprite_PSpriteAnimationInfo__fixObjectsAfterLoading`.
    Phyre_PClassDescriptorAbstract_Phyre_PSprite_PSpriteAnimationInfo__fixObjectsAfterLoading @ 0x0056_ccd0 => extern "thiscall" fn s_0056ccd0(_a0: u32, _a1: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PSprite_PSpriteAnimationInfoChar__fixObjectsAfterLoading`.
    Phyre_PClassDescriptorAbstract_Phyre_PSprite_PSpriteAnimationInfoChar__fixObjectsAfterLoading @ 0x0056_cce0 => extern "thiscall" fn s_0056cce0(_a0: u32, _a1: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PRendering_PStreamInputDescD3D11__fixObjectsAfterLoading`.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PStreamInputDescD3D11__fixObjectsAfterLoading @ 0x0059_3700 => extern "thiscall" fn s_00593700(_a0: u32, _a1: u32) -> u32 {
        0
    }
    /// `Phyre_PRendering_PDynamicMeshInstance_vslot1`.
    Phyre_PRendering_PDynamicMeshInstance_vslot1 @ 0x004f_a980 => extern "thiscall" fn s_004fa980(_a0: u32, _a1: u32) -> u32 {
        0
    }
    /// `Phyre_PRendering_ReturnZero1_00598a60`.
    Phyre_PRendering_ReturnZero1_00598a60 @ 0x0059_8a60 => extern "thiscall" fn s_00598a60(_a0: u32, _a1: u32) -> u32 {
        0
    }
    /// `Phyre_PRendering_PDynamicMeshInstance_vslot2`.
    Phyre_PRendering_PDynamicMeshInstance_vslot2 @ 0x004f_a9c0 => extern "thiscall" fn s_004fa9c0(_a0: u32, _a1: u32, _a2: u32) -> u32 {
        0
    }
    /// `Phyre_PRendering_ReturnZero2_00598a70`.
    Phyre_PRendering_ReturnZero2_00598a70 @ 0x0059_8a70 => extern "thiscall" fn s_00598a70(_a0: u32, _a1: u32, _a2: u32) -> u32 {
        0
    }
    /// `Phyre_PRendering_ReturnZero3_00598a20`.
    Phyre_PRendering_ReturnZero3_00598a20 @ 0x0059_8a20 => extern "thiscall" fn s_00598a20(_a0: u32, _a1: u32, _a2: u32, _a3: u32) -> u32 {
        0
    }
    /// `Phyre_PRendering_PRenderInterfaceNVN_bindToCurrentThread`.
    Phyre_PRendering_PRenderInterfaceNVN_bindToCurrentThread @ 0x0058_de10 => extern "C" fn s_0058de10() -> u32 {
        0x17
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PContextVariantFoldingTable__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PContextVariantFoldingTable__destructInstance @ 0x004e_00a0 => extern "thiscall" fn s_004e00a0(_a0: u32, _a1: u32) -> u8 {
        1u8
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PSceneWideParameterBuffer__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PSceneWideParameterBuffer__destructInstance @ 0x004f_de50 => extern "thiscall" fn s_004fde50(_a0: u32, _a1: u32) -> u8 {
        1u8
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PMeshInstanceBounds__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PMeshInstanceBounds__destructInstance @ 0x0050_0700 => extern "thiscall" fn s_00500700(_a0: u32, _a1: u32) -> u8 {
        1u8
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PMeshInstanceAttachPoint__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PMeshInstanceAttachPoint__destructInstance @ 0x0050_1550 => extern "thiscall" fn s_00501550(_a0: u32, _a1: u32) -> u8 {
        1u8
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PStructuredBufferBase__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PStructuredBufferBase__destructInstance @ 0x0050_2930 => extern "thiscall" fn s_00502930(_a0: u32, _a1: u32) -> u8 {
        1u8
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PGeometry_PDataBlockBufferD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PDataBlockBufferD3D11__destructInstance @ 0x0056_23b0 => extern "thiscall" fn s_005623b0(_a0: u32, _a1: u32) -> u8 {
        1u8
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PGeometry_PIndexDataBlockBufferD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PIndexDataBlockBufferD3D11__destructInstance @ 0x0056_23e0 => extern "thiscall" fn s_005623e0(_a0: u32, _a1: u32) -> u8 {
        1u8
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSprite_PSpriteAttributes__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PSprite_PSpriteAttributes__destructInstance @ 0x0056_cb60 => extern "thiscall" fn s_0056cb60(_a0: u32, _a1: u32) -> u8 {
        1u8
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSprite_PSpriteAnimationInfoInstance__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PSprite_PSpriteAnimationInfoInstance__destructInstance @ 0x0057_01a0 => extern "thiscall" fn s_005701a0(_a0: u32, _a1: u32) -> u8 {
        1u8
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderPassStateD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderPassStateD3D11__destructInstance @ 0x0059_29d0 => extern "thiscall" fn s_005929d0(_a0: u32, _a1: u32) -> u8 {
        1u8
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PContextVariantFoldingTable_4__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PContextVariantFoldingTable_4__constructInstance @ 0x004d_feb0 => extern "thiscall" fn s_004dfeb0(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PShader_4__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PShader_4__constructInstance @ 0x004d_fed0 => extern "thiscall" fn s_004dfed0(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PShaderPassInfo_4__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PShaderPassInfo_4__constructInstance @ 0x004d_fef0 => extern "thiscall" fn s_004dfef0(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PSceneRenderPass_4__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PSceneRenderPass_4__constructInstance @ 0x004e_69c0 => extern "thiscall" fn s_004e69c0(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PMaterialSwitch_4__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PMaterialSwitch_4__constructInstance @ 0x004e_69e0 => extern "thiscall" fn s_004e69e0(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PSceneRenderPass_4__constructInstance_004e6a00`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PSceneRenderPass_4__constructInstance_004e6a00 @ 0x004e_6a00 => extern "thiscall" fn s_004e6a00(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PMeshInstanceSegmentContext_4__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PMeshInstanceSegmentContext_4__constructInstance @ 0x004f_75f0 => extern "thiscall" fn s_004f75f0(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0)])
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PSharray_Phyre_PRendering_PMeshInstanceSegmentStreamBinding_const__slot70`.
    Phyre_PClassDescriptorAbstract_Phyre_PSharray_Phyre_PRendering_PMeshInstanceSegmentStreamBinding_const__slot70 @ 0x004f_7610 => extern "thiscall" fn s_004f7610(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkInstance_4__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkInstance_4__constructInstance @ 0x004f_c610 => extern "thiscall" fn s_004fc610(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PSceneWideParameterBuffer__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PSceneWideParameterBuffer__constructInstance @ 0x004f_de30 => extern "thiscall" fn s_004fde30(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PBlendableAnimationSource_4__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PBlendableAnimationSource_4__constructInstance @ 0x0054_22c0 => extern "thiscall" fn s_005422c0(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PGeometry_PDataBlockD3D11_4__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PGeometry_PDataBlockD3D11_4__constructInstance @ 0x0056_1cc0 => extern "thiscall" fn s_00561cc0(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PSharray_Phyre_PGeometry_PDataBlockBufferD3D11__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PSharray_Phyre_PGeometry_PDataBlockBufferD3D11__constructInstance @ 0x0056_1ce0 => extern "thiscall" fn s_00561ce0(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PSharray_Phyre_PGeometry_PIndexDataBlockBufferD3D11__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PSharray_Phyre_PGeometry_PIndexDataBlockBufferD3D11__constructInstance @ 0x0056_1d00 => extern "thiscall" fn s_00561d00(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PSprite_PSubTextureInfo_4__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PSprite_PSubTextureInfo_4__constructInstance @ 0x0056_c8c0 => extern "thiscall" fn s_0056c8c0(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PStreamInputDescD3D11_4__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PStreamInputDescD3D11_4__constructInstance @ 0x0058_e5e0 => extern "thiscall" fn s_0058e5e0(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PContextVariantFoldingTable__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PContextVariantFoldingTable__constructInstance @ 0x004d_ff10 => extern "thiscall" fn s_004dff10(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0), Init::Word(0x8, 0x0), Init::Word(0xc, 0x0), Init::Word(0x10, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PDataBlockBufferD3D11__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PDataBlockBufferD3D11__constructInstance @ 0x0056_1d20 => extern "thiscall" fn s_00561d20(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0), Init::Word(0x8, 0x0), Init::Word(0xc, 0x0), Init::Word(0x10, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PIndexDataBlockBufferD3D11__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PIndexDataBlockBufferD3D11__constructInstance @ 0x0056_1db0 => extern "thiscall" fn s_00561db0(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0), Init::Word(0x8, 0x0), Init::Word(0xc, 0x0), Init::Word(0x10, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PStreamInputDescD3D11__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PStreamInputDescD3D11__constructInstance @ 0x0058_e790 => extern "thiscall" fn s_0058e790(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0), Init::Word(0x8, 0x0), Init::Word(0xc, 0x0), Init::Word(0x10, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PMeshInstanceSegmentStreamBinding__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PMeshInstanceSegmentStreamBinding__constructInstance @ 0x004f_7670 => extern "thiscall" fn s_004f7670(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0), Init::Word(0x8, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PSprite_PTextureAtlasInfo__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PSprite_PTextureAtlasInfo__constructInstance @ 0x0056_ca40 => extern "thiscall" fn s_0056ca40(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0), Init::Word(0x8, 0x0)])
    }
    /// `Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PStreamInputLayoutD3D11__constructInstance`.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PStreamInputLayoutD3D11__constructInstance @ 0x0058_ea90 => extern "thiscall" fn s_0058ea90(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0), Init::Word(0x8, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PMeshInstanceSegmentContext__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PMeshInstanceSegmentContext__constructInstance @ 0x004f_7630 => extern "thiscall" fn s_004f7630(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x10, 0x0), Init::Word(0x0, 0x0), Init::Word(0x4, 0x0), Init::Word(0x8, 0x0), Init::Word(0xc, 0x0), Init::Word(0x14, 0xffff_ffff), Init::Word(0x18, 0x0), Init::Word(0x1c, 0x0), Init::Word(0x20, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PMeshInstanceBounds__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PMeshInstanceBounds__constructInstance @ 0x0050_0660 => extern "thiscall" fn s_00500660(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0xc, 0x0), Init::Word(0x10, 0x0), Init::Word(0x1c, 0x0), Init::Word(0x0, 0x0), Init::Word(0x14, 0x0), Init::Word(0x4, 0x0), Init::Word(0x18, 0x0), Init::Word(0x8, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PStructuredBufferBase__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PStructuredBufferBase__constructInstance @ 0x0050_2890 => extern "thiscall" fn s_00502890(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0), Init::Word(0x8, 0x0), Init::Word(0xc, 0x0), Init::Byte(0x10, 0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PSprite_PSpriteAnimationInfo__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PSprite_PSpriteAnimationInfo__constructInstance @ 0x0056_c8e0 => extern "thiscall" fn s_0056c8e0(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Float(0x8, 0x00b3_04f4), Init::Word(0x4, 0x0), Init::Word(0xc, 0x0), Init::Word(0x10, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PSprite_PSubTextureInfo__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PSprite_PSubTextureInfo__constructInstance @ 0x0056_ca10 => extern "thiscall" fn s_0056ca10(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0), Init::Word(0x8, 0x0), Init::Word(0xc, 0x3f80_0000), Init::Word(0x10, 0x3f80_0000)])
    }
    /// `Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PEffectVariant__constructInstance`.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PEffectVariant__constructInstance @ 0x004e_6a20 => extern "thiscall" fn s_004e6a20(_a0: u32, a1: u32) -> u32 {
        d::construct_by(&mut mem(), a1, 0x004e_4740)
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PRendering_PEffectVariant__fixObjectsAfterLoading_004e6d70`.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PEffectVariant__fixObjectsAfterLoading_004e6d70 @ 0x004e_6d70 => extern "thiscall" fn s_004e6d70(_a0: u32, a1: u32) -> u32 {
        d::construct_by(&mut mem(), a1, 0x004e_4ec0)
    }
    /// `Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PDynamicMeshInstance__constructInstance`.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PDynamicMeshInstance__constructInstance @ 0x004f_a8c0 => extern "thiscall" fn s_004fa8c0(_a0: u32, a1: u32) -> u8 {
        d::construct_owned(&mut mem(), a1, 0x00cb_b300, 0x00b2_3700)
    }
    /// `Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderComputeProgramD3D11__constructInstance`.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderComputeProgramD3D11__constructInstance @ 0x0058_e950 => extern "thiscall" fn s_0058e950(_a0: u32, a1: u32) -> u8 {
        d::construct_registered(&mut mem(), a1, 0x00c9_1048, 0x0058_3f20)
    }
    /// `Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderFragmentProgramD3D11__constructInstance`.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderFragmentProgramD3D11__constructInstance @ 0x0058_e990 => extern "thiscall" fn s_0058e990(_a0: u32, a1: u32) -> u8 {
        d::construct_registered(&mut mem(), a1, 0x00c9_1048, 0x0058_3f20)
    }
    /// `Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderGeometryProgramD3D11__constructInstance`.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderGeometryProgramD3D11__constructInstance @ 0x0058_e9d0 => extern "thiscall" fn s_0058e9d0(_a0: u32, a1: u32) -> u8 {
        d::construct_registered(&mut mem(), a1, 0x00c9_1048, 0x0058_3f20)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderPassStateD3D11__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderPassStateD3D11__constructInstance @ 0x0058_e770 => extern "thiscall" fn s_0058e770(_a0: u32, a1: u32) -> u8 {
        d::destruct_with_if(&mut mem(), a1, 0x0058_3e20)
    }
    /// `Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderProgramD3D11__constructInstance`.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderProgramD3D11__constructInstance @ 0x0058_ea10 => extern "thiscall" fn s_0058ea10(_a0: u32, a1: u32) -> u8 {
        d::destruct_with_if(&mut mem(), a1, 0x0058_3f20)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShader_4__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShader_4__destructInstance @ 0x004e_0040 => extern "thiscall" fn s_004e0040(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x004d_b710)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PSceneRenderPass__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PSceneRenderPass__destructInstance @ 0x004e_00b0 => extern "thiscall" fn s_004e00b0(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x004d_b9d0)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderPassInfo__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderPassInfo__destructInstance @ 0x004e_00d0 => extern "thiscall" fn s_004e00d0(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x004d_baf0)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PSceneRenderPass_4__destructInstance_004e6d00`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PSceneRenderPass_4__destructInstance_004e6d00 @ 0x004e_6d00 => extern "thiscall" fn s_004e6d00(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x004e_3830)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PEffectVariant__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PEffectVariant__destructInstance @ 0x004e_6d20 => extern "thiscall" fn s_004e6d20(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x004e_39b0)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PMeshInstance__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PMeshInstance__destructInstance @ 0x004f_7790 => extern "thiscall" fn s_004f7790(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x004f_2c50)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PGeometry_PDataBlockD3D11_4__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PGeometry_PDataBlockD3D11_4__destructInstance @ 0x0056_22f0 => extern "thiscall" fn s_005622f0(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x0043_3900)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PGeometry_PDataBlockD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PDataBlockD3D11__destructInstance @ 0x0056_23c0 => extern "thiscall" fn s_005623c0(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x0055_dfa0)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PGeometry_PIndexDataBlockD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PIndexDataBlockD3D11__destructInstance @ 0x0056_23f0 => extern "thiscall" fn s_005623f0(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x0055_e060)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSprite_PSpriteAnimationInfoChar__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PSprite_PSpriteAnimationInfoChar__destructInstance @ 0x0056_cb40 => extern "thiscall" fn s_0056cb40(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x0056_7620)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSprite_PSpriteCollection__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PSprite_PSpriteCollection__destructInstance @ 0x0056_cb70 => extern "thiscall" fn s_0056cb70(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x0056_76c0)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PConstantBufferD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PConstantBufferD3D11__destructInstance @ 0x0059_2760 => extern "thiscall" fn s_00592760(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x0058_4dc0)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderPassD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderPassD3D11__destructInstance @ 0x0059_29b0 => extern "thiscall" fn s_005929b0(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x0058_53c0)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderProgramD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderProgramD3D11__destructInstance @ 0x0059_29e0 => extern "thiscall" fn s_005929e0(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x0058_5610)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderVertexProgramD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderVertexProgramD3D11__destructInstance @ 0x0059_2a00 => extern "thiscall" fn s_00592a00(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x0058_56b0)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PStructuredBufferD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PStructuredBufferD3D11__destructInstance @ 0x0059_2a80 => extern "thiscall" fn s_00592a80(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x0058_57b0)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PTexture2DD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PTexture2DD3D11__destructInstance @ 0x0059_2aa0 => extern "thiscall" fn s_00592aa0(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x0058_5840)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PTexture3DD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PTexture3DD3D11__destructInstance @ 0x0059_2ac0 => extern "thiscall" fn s_00592ac0(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x0058_58d0)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PTextureCubeMapD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PTextureCubeMapD3D11__destructInstance @ 0x0059_2ae0 => extern "thiscall" fn s_00592ae0(_a0: u32, a1: u32) -> u8 {
        d::destruct_with(&mut mem(), a1, 0x0058_5960)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PDynamicMeshInstance__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PDynamicMeshInstance__destructInstance @ 0x004f_a900 => extern "thiscall" fn s_004fa900(_a0: u32, a1: u32) -> u8 {
        d::destruct_virtual(&mut mem(), a1)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PModifierNetworkDynamicMeshInstance__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PModifierNetworkDynamicMeshInstance__destructInstance @ 0x004f_c700 => extern "thiscall" fn s_004fc700(_a0: u32, a1: u32) -> u8 {
        d::destruct_virtual(&mut mem(), a1)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimatableComponent__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimatableComponent__destructInstance @ 0x0054_23f0 => extern "thiscall" fn s_005423f0(_a0: u32, a1: u32) -> u8 {
        d::destruct_virtual(&mut mem(), a1)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PGameplay_PCameraControllerComponent__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PGameplay_PCameraControllerComponent__destructInstance @ 0x0055_8500 => extern "thiscall" fn s_00558500(_a0: u32, a1: u32) -> u8 {
        d::destruct_virtual(&mut mem(), a1)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PContextVariantFoldingTable_4__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PContextVariantFoldingTable_4__destructInstance @ 0x004e_0000 => extern "thiscall" fn s_004e0000(_a0: u32, a1: u32) -> u8 {
        d::free_array(&mut mem(), a1, 0, None, true)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PSceneRenderPass_4__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PSceneRenderPass_4__destructInstance @ 0x004e_6c80 => extern "thiscall" fn s_004e6c80(_a0: u32, a1: u32) -> u8 {
        d::free_array(&mut mem(), a1, 0, None, false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkInstance_4__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkInstance_4__destructInstance @ 0x004f_c6c0 => extern "thiscall" fn s_004fc6c0(_a0: u32, a1: u32) -> u8 {
        d::free_array(&mut mem(), a1, 0, None, false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShaderPassInfo_4__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShaderPassInfo_4__destructInstance @ 0x004e_0060 => extern "thiscall" fn s_004e0060(_a0: u32, a1: u32) -> u8 {
        d::free_array(&mut mem(), a1, 0, Some(0x004d_d770), false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PMaterialSwitch_4__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PMaterialSwitch_4__destructInstance @ 0x004e_6cc0 => extern "thiscall" fn s_004e6cc0(_a0: u32, a1: u32) -> u8 {
        d::free_array(&mut mem(), a1, 0, Some(0x004e_4df0), false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PMeshInstanceSegmentContext_4__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PMeshInstanceSegmentContext_4__destructInstance @ 0x004f_7710 => extern "thiscall" fn s_004f7710(_a0: u32, a1: u32) -> u8 {
        d::free_array(&mut mem(), a1, 0, Some(0x004f_4fc0), false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PBlendableAnimationSource_4__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PBlendableAnimationSource_4__destructInstance @ 0x0054_23b0 => extern "thiscall" fn s_005423b0(_a0: u32, a1: u32) -> u8 {
        d::free_array(&mut mem(), a1, 0, Some(0x0054_0b80), false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PSprite_PSubTextureInfo_4__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PSprite_PSubTextureInfo_4__destructInstance @ 0x0056_ca70 => extern "thiscall" fn s_0056ca70(_a0: u32, a1: u32) -> u8 {
        d::free_array(&mut mem(), a1, 0, Some(0x0056_a700), false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PStreamInputDescD3D11_4__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PStreamInputDescD3D11_4__destructInstance @ 0x0059_2720 => extern "thiscall" fn s_00592720(_a0: u32, a1: u32) -> u8 {
        d::free_array(&mut mem(), a1, 0, Some(0x0058_8190), false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PStreamInputLayoutD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PStreamInputLayoutD3D11__destructInstance @ 0x0059_2a40 => extern "thiscall" fn s_00592a40(_a0: u32, a1: u32) -> u8 {
        d::free_array(&mut mem(), a1, 0, Some(0x0058_8190), false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSprite_PTextureAtlasInfo__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PSprite_PTextureAtlasInfo__destructInstance @ 0x0056_cbb0 => extern "thiscall" fn s_0056cbb0(_a0: u32, a1: u32) -> u8 {
        d::free_array(&mut mem(), a1, 4, Some(0x0056_a700), false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PMeshInstanceSegmentStreamBinding__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PMeshInstanceSegmentStreamBinding__destructInstance @ 0x004f_77f0 => extern "thiscall" fn s_004f77f0(_a0: u32, a1: u32) -> u8 {
        d::free_if_owned(&mut mem(), a1, 4)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSprite_PSubTextureInfo__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PSprite_PSubTextureInfo__destructInstance @ 0x0056_cb90 => extern "thiscall" fn s_0056cb90(_a0: u32, a1: u32) -> u8 {
        d::free_if_owned(&mut mem(), a1, 0)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PConstantBufferD3D11__slot8c`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PConstantBufferD3D11__slot8c @ 0x0059_2a20 => extern "thiscall" fn s_00592a20(_a0: u32, a1: u32) -> u8 {
        d::free_if_owned(&mut mem(), a1, 0)
    }
    /// `Phyre_FreeOwnedBuffers20_0056a700`.
    Phyre_FreeOwnedBuffers20_0056a700 @ 0x0056_a700 => extern "C" fn s_0056a700(a0: u32, a1: u32) {
        d::free_owned_buffers(&mut mem(), a0, a1);
    }
    /// `Phyre_FreeOwnedBuffers20_00588190`.
    Phyre_FreeOwnedBuffers20_00588190 @ 0x0058_8190 => extern "C" fn s_00588190(a0: u32, a1: u32) {
        d::free_owned_buffers(&mut mem(), a0, a1);
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PShader_4__slot20`.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PShader_4__slot20 @ 0x004e_1690 => extern "thiscall" fn s_004e1690(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        d::member_forward(&mut mem(), a0, a1, a2, a3)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PShaderPassInfo_4__slot20`.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PShaderPassInfo_4__slot20 @ 0x004e_16c0 => extern "thiscall" fn s_004e16c0(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        d::member_forward(&mut mem(), a0, a1, a2, a3)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PMaterialSwitch_4__slot20`.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PMaterialSwitch_4__slot20 @ 0x004e_79f0 => extern "thiscall" fn s_004e79f0(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        d::member_forward(&mut mem(), a0, a1, a2, a3)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PSceneRenderPass_4__slot20`.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PSceneRenderPass_4__slot20 @ 0x004e_7a20 => extern "thiscall" fn s_004e7a20(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        d::member_forward(&mut mem(), a0, a1, a2, a3)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PEffectVariant_4__slot20`.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PEffectVariant_4__slot20 @ 0x004f_0470 => extern "thiscall" fn s_004f0470(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        d::member_forward(&mut mem(), a0, a1, a2, a3)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PContextSwitch_const_4__slot20`.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PContextSwitch_const_4__slot20 @ 0x004f_04a0 => extern "thiscall" fn s_004f04a0(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        d::member_forward(&mut mem(), a0, a1, a2, a3)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PLightType_const_4__slot20`.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PLightType_const_4__slot20 @ 0x004f_04d0 => extern "thiscall" fn s_004f04d0(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        d::member_forward(&mut mem(), a0, a1, a2, a3)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PShadowCasterType_const_4__slot20`.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PShadowCasterType_const_4__slot20 @ 0x004f_0500 => extern "thiscall" fn s_004f0500(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        d::member_forward(&mut mem(), a0, a1, a2, a3)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PNodeContext_4__slot20`.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PNodeContext_4__slot20 @ 0x004f_0530 => extern "thiscall" fn s_004f0530(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        d::member_forward(&mut mem(), a0, a1, a2, a3)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkInstance_4__slot20`.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkInstance_4__slot20 @ 0x004f_cc50 => extern "thiscall" fn s_004fcc50(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        d::member_forward(&mut mem(), a0, a1, a2, a3)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PGeometry_PDataBlockD3D11_4__slot20`.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PGeometry_PDataBlockD3D11_4__slot20 @ 0x0056_4380 => extern "thiscall" fn s_00564380(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        d::member_forward(&mut mem(), a0, a1, a2, a3)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PStreamInputDescD3D11_4__slot20`.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PStreamInputDescD3D11_4__slot20 @ 0x0059_ba20 => extern "thiscall" fn s_0059ba20(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        d::member_forward(&mut mem(), a0, a1, a2, a3)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PLightType_const_4__slot18`.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PLightType_const_4__slot18 @ 0x004e_ef20 => extern "thiscall" fn s_004eef20(a0: u32, a1: u32) -> u32 {
        d::member_element(&mut mem(), a0, a1)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PShadowCasterType_const_4__slot18`.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PShadowCasterType_const_4__slot18 @ 0x004e_ef80 => extern "thiscall" fn s_004eef80(a0: u32, a1: u32) -> u32 {
        d::member_element(&mut mem(), a0, a1)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PSharray_Phyre_PRendering_PMeshInstanceSegmentStreamBinding_const__slot18`.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PSharray_Phyre_PRendering_PMeshInstanceSegmentStreamBinding_const__slot18 @ 0x004f_7c10 => extern "thiscall" fn s_004f7c10(a0: u32, a1: u32) -> u32 {
        d::member_element(&mut mem(), a0, a1)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkInstance_4__slot18`.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkInstance_4__slot18 @ 0x004f_c780 => extern "thiscall" fn s_004fc780(a0: u32, a1: u32) -> u32 {
        d::member_element(&mut mem(), a0, a1)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PSharray_Phyre_PRendering_PMeshInstanceSegmentStreamBinding_const__slot10`.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PSharray_Phyre_PRendering_PMeshInstanceSegmentStreamBinding_const__slot10 @ 0x004f_7e20 => extern "thiscall" fn s_004f7e20(a0: u32, a1: u32) -> u32 {
        d::member_handler34(&mut mem(), a0, a1)
    }
    /// `Phyre_PArray_PShader_setBorrowed_004e15f0`.
    Phyre_PArray_PShader_setBorrowed_004e15f0 @ 0x004e_15f0 => extern "thiscall" fn s_004e15f0(a0: u32, a1: u32, a2: u32) -> u32 {
        d::set_borrowed(&mut mem(), a0, a1, a2, 0x004e_13a0)
    }
    /// `Phyre_PArray_PMaterialSwitch_setBorrowed_004e79a0`.
    Phyre_PArray_PMaterialSwitch_setBorrowed_004e79a0 @ 0x004e_79a0 => extern "thiscall" fn s_004e79a0(a0: u32, a1: u32, a2: u32) -> u32 {
        d::set_borrowed(&mut mem(), a0, a1, a2, 0x004e_7780)
    }
    /// `Phyre_PArray_PDataBlockD3D11_setBorrowed_005642c0`.
    Phyre_PArray_PDataBlockD3D11_setBorrowed_005642c0 @ 0x0056_42c0 => extern "thiscall" fn s_005642c0(a0: u32, a1: u32, a2: u32) -> u32 {
        d::set_borrowed(&mut mem(), a0, a1, a2, 0x0043_4e40)
    }
    /// `Phyre_RegisterTypeAndAddToNamespace_004fd620`.
    Phyre_RegisterTypeAndAddToNamespace_004fd620 @ 0x004f_d620 => extern "C" fn s_004fd620() -> u32 {
        d::register_type(&mut mem(), 0x00ca_8298)
    }
    /// `Phyre_RegisterTypeAndAddToNamespace_005020e0`.
    Phyre_RegisterTypeAndAddToNamespace_005020e0 @ 0x0050_20e0 => extern "C" fn s_005020e0() -> u32 {
        d::register_type(&mut mem(), 0x00ca_8dc8)
    }
    /// `Phyre_RegisterTypeAndAddToNamespace_00503490`.
    Phyre_RegisterTypeAndAddToNamespace_00503490 @ 0x0050_3490 => extern "C" fn s_00503490() -> u32 {
        d::register_type(&mut mem(), 0x00ca_90d0)
    }
    /// `Phyre_RegisterTypeAndAddToNamespace_005034b0`.
    Phyre_RegisterTypeAndAddToNamespace_005034b0 @ 0x0050_34b0 => extern "C" fn s_005034b0() -> u32 {
        d::register_type(&mut mem(), 0x00ca_9038)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PGeometry_PDataBlockBufferD3D11__slot78`.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PDataBlockBufferD3D11__slot78 @ 0x0056_19b0 => extern "C" fn s_005619b0() -> u32 {
        d::register_type(&mut mem(), 0x00cb_11b8)
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PGeometry_PIndexDataBlockBufferD3D11__slot30`.
    Phyre_PClassDescriptorAbstract_Phyre_PGeometry_PIndexDataBlockBufferD3D11__slot30 @ 0x0056_19e0 => extern "C" fn s_005619e0() -> u32 {
        d::register_type(&mut mem(), 0x00cb_1088)
    }
    /// `Phyre_PRendering_PMaterial_setEffectVariant`.
    Phyre_PRendering_PMaterial_setEffectVariant @ 0x004d_8940 => extern "thiscall" fn s_004d8940(a0: u32, a1: u32, a2: u32) -> u32 {
        x::set_pair(&mut mem(), a0, 0, a1, a2)
    }
    /// `Phyre_PRendering_PMaterial_setSceneRenderPassRemapping`.
    Phyre_PRendering_PMaterial_setSceneRenderPassRemapping @ 0x004d_8960 => extern "thiscall" fn s_004d8960(a0: u32, a1: u32, a2: u32) -> u32 {
        x::set_pair(&mut mem(), a0, 8, a1, a2)
    }
    /// `Phyre_PRendering_PSceneRenderPassType_InvalidateLookupIndices`.
    Phyre_PRendering_PSceneRenderPassType_InvalidateLookupIndices @ 0x004d_f2d0 => extern "C" fn s_004df2d0() -> u32 {
        x::invalidate_lookup_indices(&mut mem())
    }
    /// `Phyre_PRendering_LookupInto_004df9c0`.
    Phyre_PRendering_LookupInto_004df9c0 @ 0x004d_f9c0 => extern "C" fn s_004df9c0(a0: u32, a1: u32) -> u32 {
        x::lookup_into(&mut mem(), a0, a1, 0x004d_d910)
    }
    /// `Vectormath_Vector4_scaleTo_004f30d0`.
    Vectormath_Vector4_scaleTo_004f30d0 @ 0x004f_30d0 => extern "thiscall" fn s_004f30d0(a0: u32, a1: u32, a2: u32) -> u32 {
        x::scale4(&mut mem(), a0, a1, a2)
    }
    /// `Phyre_PRendering_PConstantBufferNVN_getBufferCount`.
    Phyre_PRendering_PConstantBufferNVN_getBufferCount @ 0x004f_7d50 => extern "thiscall" fn s_004f7d50(a0: u32) -> u32 {
        x::count_or_one(&mut mem(), a0, 0xc)
    }
    /// `Phyre_PRendering_CountOrOne14_00593ba0`.
    Phyre_PRendering_CountOrOne14_00593ba0 @ 0x0059_3ba0 => extern "thiscall" fn s_00593ba0(a0: u32) -> u32 {
        x::count_or_one(&mut mem(), a0, 0x14)
    }
    /// `Phyre_PRendering_CountOrOne24_00593bb0`.
    Phyre_PRendering_CountOrOne24_00593bb0 @ 0x0059_3bb0 => extern "thiscall" fn s_00593bb0(a0: u32) -> u32 {
        x::count_or_one(&mut mem(), a0, 0x24)
    }
    /// `Phyre_PRendering_CountOrOne28_00593bc0`.
    Phyre_PRendering_CountOrOne28_00593bc0 @ 0x0059_3bc0 => extern "thiscall" fn s_00593bc0(a0: u32) -> u32 {
        x::count_or_one(&mut mem(), a0, 0x28)
    }
    /// `Phyre_PRendering_PDynamicMeshInstance_PDynamicMeshInstance`.
    Phyre_PRendering_PDynamicMeshInstance_PDynamicMeshInstance @ 0x004f_a420 => extern "thiscall" fn s_004fa420(a0: u32, a1: u32) -> u32 {
        x::dynamic_mesh_instance(&mut mem(), a0, a1)
    }
    /// `Phyre_PRendering_PDynamicMeshInstance_dtor`.
    Phyre_PRendering_PDynamicMeshInstance_dtor @ 0x004f_a470 => extern "thiscall" fn s_004fa470(a0: u32) {
        x::dynamic_mesh_instance_dtor(&mut mem(), a0);
    }
    /// `Phyre_PRendering_PDynamicMeshInstance_scalarDeletingDtor`.
    Phyre_PRendering_PDynamicMeshInstance_scalarDeletingDtor @ 0x004f_a510 => extern "thiscall" fn s_004fa510(a0: u32, a1: u32) -> u32 {
        x::scalar_deleting_dtor(&mut mem(), a0, a1, None)
    }
    /// `Phyre_PGameplay_PCameraControllerComponent_scalarDeletingDtor`.
    Phyre_PGameplay_PCameraControllerComponent_scalarDeletingDtor @ 0x0055_7670 => extern "thiscall" fn s_00557670(a0: u32, a1: u32) -> u32 {
        x::scalar_deleting_dtor(&mut mem(), a0, a1, Some(0x0055_7410))
    }
    /// `Phyre_PRendering_PRenderInterfaceBase_PDynamicProcessBufferAllocation_PDynamicProcessBufferAllocation`.
    Phyre_PRendering_PRenderInterfaceBase_PDynamicProcessBufferAllocation_PDynamicProcessBufferAllocation @ 0x004f_ee50 => extern "thiscall" fn s_004fee50(a0: u32) -> u32 {
        x::zero_init(&mut mem(), a0, 4, None)
    }
    /// `Phyre_ZeroInit16b_00501c40`.
    Phyre_ZeroInit16b_00501c40 @ 0x0050_1c40 => extern "thiscall" fn s_00501c40(a0: u32) -> u32 {
        x::zero_init(&mut mem(), a0, 4, Some(0x10))
    }
    /// `Phyre_ZeroInit8_005031e0`.
    Phyre_ZeroInit8_005031e0 @ 0x0050_31e0 => extern "thiscall" fn s_005031e0(a0: u32) -> u32 {
        x::zero_init(&mut mem(), a0, 2, None)
    }
    /// `Phyre_PRendering_Construct583580_004fee70`.
    Phyre_PRendering_Construct583580_004fee70 @ 0x004f_ee70 => extern "thiscall" fn s_004fee70(a0: u32) -> u32 {
        x::call_return_this(&mut mem(), a0, 0x0058_3580)
    }
    /// `Phyre_PRendering_PMeshInstanceBounds_setMinAndSize`.
    Phyre_PRendering_PMeshInstanceBounds_setMinAndSize @ 0x0050_0c20 => extern "thiscall" fn s_00500c20(a0: u32, a1: u32, a2: u32) -> u32 {
        x::set_min_and_size(&mut mem(), a0, a1, a2)
    }
    /// `Phyre_PGeometry_PDataBlockNVN_isBound`.
    Phyre_PGeometry_PDataBlockNVN_isBound @ 0x0056_2da0 => extern "thiscall" fn s_00562da0(a0: u32) -> u32 {
        x::field_bit0(&mut mem(), a0, 0x34)
    }
    /// `Phyre_PGeometry_PIndexDataBlockNVN_isBound`.
    Phyre_PGeometry_PIndexDataBlockNVN_isBound @ 0x0056_2db0 => extern "thiscall" fn s_00562db0(a0: u32) -> u32 {
        x::field_bit0(&mut mem(), a0, 0x30)
    }
    /// `Phyre_PRenderInterface_getTextureQuality`.
    Phyre_PRenderInterface_getTextureQuality @ 0x0059_47d0 => extern "thiscall" fn s_005947d0(a0: u32) -> u32 {
        x::field(&mut mem(), a0, 0x1340)
    }
    /// `Phyre_PRendering_PRenderInterfaceNVN_getViewportSize`.
    Phyre_PRendering_PRenderInterfaceNVN_getViewportSize @ 0x0059_4f40 => extern "thiscall" fn s_00594f40(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        x::viewport_size(&mut mem(), a0, a1, a2, a3)
    }
    /// `Phyre_PRendering_PConstantBufferNVN_isBound`.
    Phyre_PRendering_PConstantBufferNVN_isBound @ 0x0059_5f70 => extern "thiscall" fn s_00595f70(a0: u32) -> u32 {
        x::field_nonzero(&mut mem(), a0, 0x38)
    }
    /// `Phyre_PRendering_PStructuredBufferNVN_isBound`.
    Phyre_PRendering_PStructuredBufferNVN_isBound @ 0x0059_5f80 => extern "thiscall" fn s_00595f80(a0: u32) -> u32 {
        x::field_nonzero(&mut mem(), a0, 0x44)
    }
    /// `Phyre_PRendering_PTexture2DD3D11_isBound`.
    Phyre_PRendering_PTexture2DD3D11_isBound @ 0x0059_5f90 => extern "thiscall" fn s_00595f90(a0: u32) -> u32 {
        x::field_nonzero(&mut mem(), a0, 0x70)
    }
    /// `Phyre_PRendering_SetTarget_005980c0`.
    Phyre_PRendering_SetTarget_005980c0 @ 0x0059_80c0 => extern "thiscall" fn s_005980c0(a0: u32, a1: u32) -> u32 {
        x::set_target(&mut mem(), a0, a1)
    }
    /// `Phyre_PRendering_PShaderPassNVN_releaseShaderPass`.
    Phyre_PRendering_PShaderPassNVN_releaseShaderPass @ 0x0059_8a30 => extern "thiscall" fn s_00598a30(_a0: u32, a1: u32) -> u32 {
        x::release_shader_pass(&mut mem(), a1)
    }
    /// `Phyre_PRendering_PRenderInterface_setClearColor_00599d50`.
    Phyre_PRendering_PRenderInterface_setClearColor_00599d50 @ 0x0059_9d50 => extern "thiscall" fn s_00599d50(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32) {
        x::set_clear_color(&mut mem(), a0, [a1, a2, a3, a4]);
    }
    /// `Phyre_PRendering_PRenderInterfaceNVN_setClearDepth`.
    Phyre_PRendering_PRenderInterfaceNVN_setClearDepth @ 0x0059_9eb0 => extern "thiscall" fn s_00599eb0(a0: u32, a1: u32) {
        x::set_clear_depth(&mut mem(), a0, a1);
    }
    /// `Phyre_PRendering_PRenderInterface_setClearStencil_00599ec0`.
    Phyre_PRendering_PRenderInterface_setClearStencil_00599ec0 @ 0x0059_9ec0 => extern "thiscall" fn s_00599ec0(a0: u32, a1: u32) -> u32 {
        x::set_field(&mut mem(), a0, 0x390, a1)
    }
    /// `Renderer_SetVSync`.
    Renderer_SetVSync @ 0x0058_e170 => extern "thiscall" fn s_0058e170(a0: u32, a1: u32) -> u32 {
        x::set_byte_ret0(&mut mem(), a0, 0x131c, a1)
    }
    /// `Phyre_PRendering_MapMode_0058ab10`.
    Phyre_PRendering_MapMode_0058ab10 @ 0x0058_ab10 => extern "C" fn s_0058ab10(a0: u32) -> u32 {
        x::map_mode(a0)
    }
    /// `Phyre_ZeroArray16_0058af90`.
    Phyre_ZeroArray16_0058af90 @ 0x0058_af90 => extern "C" fn s_0058af90(a0: u32, a1: u32) {
        x::zero_array16(&mut mem(), a0, a1);
    }
    /// `Phyre_PRendering_Forward593930_0058e5b0`.
    Phyre_PRendering_Forward593930_0058e5b0 @ 0x0058_e5b0 => extern "thiscall" fn s_0058e5b0(a0: u32, a1: u32) -> u32 {
        x::forward_593930(&mut mem(), a0, a1, 0x0059_3930)
    }
    /// `Phyre_PRendering_ElementField8_00593db0`.
    Phyre_PRendering_ElementField8_00593db0 @ 0x0059_3db0 => extern "thiscall" fn s_00593db0(a0: u32, a1: u32) -> u32 {
        x::element_field(&mut mem(), a0, a1, 0x0058_5c90, 8)
    }
    /// `Phyre_PRendering_ElementField4_00594060`.
    Phyre_PRendering_ElementField4_00594060 @ 0x0059_4060 => extern "thiscall" fn s_00594060(a0: u32, a1: u32) -> u32 {
        x::element_field(&mut mem(), a0, a1, 0x0058_5c90, 4)
    }
    /// `Phyre_PRendering_SetMode_0059a120`.
    Phyre_PRendering_SetMode_0059a120 @ 0x0059_a120 => extern "thiscall" fn s_0059a120(a0: u32, a1: u32, a2: u32) -> u32 {
        x::set_mode(&mut mem(), a0, a1, a2, 0x0058_a9a0)
    }
    /// `Phyre_PRendering_ForwardIfBound_0059aff0`.
    Phyre_PRendering_ForwardIfBound_0059aff0 @ 0x0059_aff0 => extern "stdcall" fn s_0059aff0(a0: u32, a1: u32) -> u32 {
        x::forward_if_bound(&mut mem(), a0, a1, 0x0059_b140)
    }
    /// `Phyre_PRendering_SetViewport_0059bbf0`.
    Phyre_PRendering_SetViewport_0059bbf0 @ 0x0059_bbf0 => extern "thiscall" fn s_0059bbf0(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32) -> u32 {
        x::set_viewport(&mut mem(), a0, [a1, a2, a3, a4], 0x0058_c790)
    }
    /// `Phyre_ZeroInit21_00583420`.
    Phyre_ZeroInit21_00583420 @ 0x0058_3420 => extern "thiscall" fn s_00583420(a0: u32) -> u32 {
        x::zero_init21(&mut mem(), a0)
    }
    /// `PSamplerStateD3D11_ctor`.
    PSamplerStateD3D11_ctor @ 0x0058_3b20 => extern "thiscall" fn s_00583b20(a0: u32) -> u32 {
        x::ctor_then_zero(&mut mem(), a0, 0x004b_e870, &[0x20])
    }
    /// `Phyre_PShaderProgramD3D11_derivedCtor_00583b40`.
    Phyre_PShaderProgramD3D11_derivedCtor_00583b40 @ 0x0058_3b40 => extern "thiscall" fn s_00583b40(a0: u32, _a1: u32) -> u32 {
        x::ctor_then_zero(&mut mem(), a0, 0x0058_3f20, &[0x4e0])
    }
    /// `Phyre_PShaderProgramD3D11_derivedCtor_00583bc0`.
    Phyre_PShaderProgramD3D11_derivedCtor_00583bc0 @ 0x0058_3bc0 => extern "thiscall" fn s_00583bc0(a0: u32, _a1: u32) -> u32 {
        x::ctor_then_zero(&mut mem(), a0, 0x0058_3f20, &[0x4e0])
    }
    /// `Phyre_PShaderProgramD3D11_derivedCtor_00583be0`.
    Phyre_PShaderProgramD3D11_derivedCtor_00583be0 @ 0x0058_3be0 => extern "thiscall" fn s_00583be0(a0: u32, _a1: u32) -> u32 {
        x::ctor_then_zero(&mut mem(), a0, 0x0058_3f20, &[0x4e0])
    }
    /// `Phyre_PShaderProgramD3D11_derivedCtor_00584060`.
    Phyre_PShaderProgramD3D11_derivedCtor_00584060 @ 0x0058_4060 => extern "thiscall" fn s_00584060(a0: u32, _a1: u32) -> u32 {
        x::ctor_then_zero(&mut mem(), a0, 0x0058_3f20, &[0x4e0, 0x4e4, 0x4e8, 0x4ec])
    }
    /// `Phyre_PRendering_Ctor8_005844a0`.
    Phyre_PRendering_Ctor8_005844a0 @ 0x0058_44a0 => extern "thiscall" fn s_005844a0(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32) -> u32 {
        x::ctor8(&mut mem(), a0, [a1, a2, a3, a4, a5, a6, a7, a8], 0x004c_0c00)
    }
}
