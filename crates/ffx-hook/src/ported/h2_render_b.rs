//! `ffx-prender`, M4 port round 1, list h2-render-b: `PhyreEngine`'s
//! render code (the type system's descriptor and array-member slots of
//! the render classes, small render-interface, material and shader
//! methods). Checked by `crates/ffx-oracle/tests/h2_render_b.rs`.
//!
//! Threads: the descriptor slots run where the serializer loads (the main
//! and loader threads), the render-interface setters on the render
//! thread. No shim keeps state; nothing here has a frame.
//!
//! Round 2 adds the `PArray<T>` class descriptors' constructors and
//! `DeclareClassDescriptor`s (`ffx_prender::declare`), which run where a
//! class is first used (the main and loader threads).
//!
//! Round 3 (`ffx_prender::h2_more`): spin waits on job counters (main
//! thread), tree and free-list inserts, world cursors, constructors and
//! destructors of D3D11 objects, the sampler filter map. Shims whose
//! original hands a local to a callee pass a `StackFrame`.

use super::{mem, ported};
use ffx_prender::array::bridge as a;
use ffx_prender::declare as dc;
use ffx_prender::descriptor::{Init, bridge as d};
use ffx_prender::dtor::bridge as dt;
use ffx_prender::h2_bind::bridge as hb;
use ffx_prender::h2_more::bridge as hm;
use ffx_prender::h2_more2::bridge as h2;
use ffx_prender::h2_more3::bridge as h3;
use ffx_prender::h2_more4::bridge as h4;
use ffx_prender::h2_more5::bridge as h5;
use ffx_prender::misc::bridge as x;
use ffx_prender::small::bridge as sm;

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
        d::construct_registered(&mut mem(), a1, 0x00c9_1048, 0x0058_3f20, &[0x4e0])
    }
    /// `Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderFragmentProgramD3D11__constructInstance`.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderFragmentProgramD3D11__constructInstance @ 0x0058_e990 => extern "thiscall" fn s_0058e990(_a0: u32, a1: u32) -> u8 {
        d::construct_registered(&mut mem(), a1, 0x00c9_1048, 0x0058_3f20, &[0x4e0])
    }
    /// `Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderGeometryProgramD3D11__constructInstance`.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderGeometryProgramD3D11__constructInstance @ 0x0058_e9d0 => extern "thiscall" fn s_0058e9d0(_a0: u32, a1: u32) -> u8 {
        d::construct_registered(&mut mem(), a1, 0x00c9_1048, 0x0058_3f20, &[0x4e0])
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
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PStructure_Buffer__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PStructure_Buffer__constructInstance @ 0x0050_2840 => extern "thiscall" fn s_00502840(_a0: u32, a1: u32) -> u8 {
        d::destruct_with_if(&mut mem(), a1, 0x0058_41a0)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PIndirectArgsBuffer__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PIndirectArgsBuffer__constructInstance @ 0x0050_3a80 => extern "thiscall" fn s_00503a80(_a0: u32, a1: u32) -> u8 {
        d::destruct_with_if(&mut mem(), a1, 0x0058_3420)
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PAnimation_PAnimatableComponent__slot70`.
    Phyre_PClassDescriptorAbstract_Phyre_PAnimation_PAnimatableComponent__slot70 @ 0x0054_22e0 => extern "thiscall" fn s_005422e0(_a0: u32, a1: u32) -> u8 {
        d::destruct_with_if(&mut mem(), a1, 0x0053_e720)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PGameplay_PCameraControllerComponent__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PGameplay_PCameraControllerComponent__constructInstance @ 0x0055_84b0 => extern "thiscall" fn s_005584b0(_a0: u32, a1: u32) -> u8 {
        d::destruct_with_if(&mut mem(), a1, 0x0055_72e0)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PDataBlockD3D11__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PDataBlockD3D11__constructInstance @ 0x0056_1d60 => extern "thiscall" fn s_00561d60(_a0: u32, a1: u32) -> u8 {
        d::destruct_with_if(&mut mem(), a1, 0x0055_dc40)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PIndexDataBlockD3D11__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PIndexDataBlockD3D11__constructInstance @ 0x0056_1df0 => extern "thiscall" fn s_00561df0(_a0: u32, a1: u32) -> u8 {
        d::destruct_with_if(&mut mem(), a1, 0x0055_dd30)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PMeshSegmentD3D11__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PMeshSegmentD3D11__constructInstance @ 0x0056_1e40 => extern "thiscall" fn s_00561e40(_a0: u32, a1: u32) -> u8 {
        d::destruct_with_if(&mut mem(), a1, 0x0043_37d0)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSprite_PSpriteCollection__slotb8`.
    Phyre_PClassDescriptorForType_Phyre_PSprite_PSpriteCollection__slotb8 @ 0x0056_c9c0 => extern "thiscall" fn s_0056c9c0(_a0: u32, a1: u32) -> u8 {
        d::destruct_with_if(&mut mem(), a1, 0x0056_7110)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PConstantBufferD3D11__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PConstantBufferD3D11__constructInstance @ 0x0058_e600 => extern "thiscall" fn s_0058e600(_a0: u32, a1: u32) -> u8 {
        d::destruct_with_if(&mut mem(), a1, 0x0058_3240)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderPassD3D11__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderPassD3D11__constructInstance @ 0x0058_e720 => extern "thiscall" fn s_0058e720(_a0: u32, a1: u32) -> u8 {
        d::destruct_with_if(&mut mem(), a1, 0x0058_3c00)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PStructuredBufferD3D11__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PStructuredBufferD3D11__constructInstance @ 0x0058_e7d0 => extern "thiscall" fn s_0058e7d0(_a0: u32, a1: u32) -> u8 {
        d::destruct_with_if(&mut mem(), a1, 0x0058_41a0)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PTexture3DD3D11__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PTexture3DD3D11__constructInstance @ 0x0058_e8b0 => extern "thiscall" fn s_0058e8b0(_a0: u32, a1: u32) -> u8 {
        d::destruct_with_if(&mut mem(), a1, 0x0058_42c0)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PTextureCubeMapD3D11__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PTextureCubeMapD3D11__constructInstance @ 0x0058_e900 => extern "thiscall" fn s_0058e900(_a0: u32, a1: u32) -> u8 {
        d::destruct_with_if(&mut mem(), a1, 0x0058_4400)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PSamplerStateD3D11__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PSamplerStateD3D11__constructInstance @ 0x0058_e6c0 => extern "thiscall" fn s_0058e6c0(_a0: u32, a1: u32) -> u8 {
        d::construct_with_ctor(&mut mem(), a1, 0x004b_e870, &[0x20])
    }
    /// `Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderVertexProgramD3D11__constructInstance`.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderVertexProgramD3D11__constructInstance @ 0x0058_ea30 => extern "thiscall" fn s_0058ea30(_a0: u32, a1: u32) -> u8 {
        d::construct_registered(&mut mem(), a1, 0x00c9_1048, 0x0058_3f20, &[0x4e0, 0x4e4, 0x4e8, 0x4ec])
    }
    /// `Phyre_PRendering_PEffectVariant_constructInPlace_004e4740`.
    Phyre_PRendering_PEffectVariant_constructInPlace_004e4740 @ 0x004e_4740 => extern "C" fn s_004e4740(a0: u32) -> u8 {
        d::construct_typed(&mut mem(), a0, 0x00ca_66e8, &[Init::Word(0x4, 0x0), Init::Word(0x8, 0x0), Init::Word(0xc, 0x0), Init::Word(0x10, 0x0), Init::Word(0x14, 0x0), Init::Word(0x18, 0x0), Init::Half(0x1c, 0), Init::Word(0x20, 0x0), Init::Word(0x24, 0x0), Init::Word(0x28, 0x0), Init::Word(0x2c, 0x0), Init::Word(0x30, 0x0)])
    }
    /// `Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PMeshInstanceAttachPoint__constructInstance`.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PMeshInstanceAttachPoint__constructInstance @ 0x0050_1500 => extern "thiscall" fn s_00501500(_a0: u32, a1: u32) -> u8 {
        d::construct_two_types(&mut mem(), a1, 0x00c9_2090, 0x00ca_7228)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PSamplerStateD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PSamplerStateD3D11__destructInstance @ 0x0059_2830 => extern "thiscall" fn s_00592830(_a0: u32, a1: u32) -> u8 {
        d::destruct_with2(&mut mem(), a1, 0x0059_c770, 0x004b_e970)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderComputeProgramD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderComputeProgramD3D11__destructInstance @ 0x0059_2890 => extern "thiscall" fn s_00592890(_a0: u32, a1: u32) -> u8 {
        d::destruct_with2(&mut mem(), a1, 0x0059_c830, 0x0058_5610)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderFragmentProgramD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderFragmentProgramD3D11__destructInstance @ 0x0059_28f0 => extern "thiscall" fn s_005928f0(_a0: u32, a1: u32) -> u8 {
        d::destruct_with2(&mut mem(), a1, 0x0059_c890, 0x0058_5610)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderGeometryProgramD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderGeometryProgramD3D11__destructInstance @ 0x0059_2950 => extern "thiscall" fn s_00592950(_a0: u32, a1: u32) -> u8 {
        d::destruct_with2(&mut mem(), a1, 0x0059_c8f0, 0x0058_5610)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PStructuredBuffer__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PStructuredBuffer__destructInstance @ 0x0050_28c0 => extern "thiscall" fn s_005028c0(_a0: u32, a1: u32) -> u8 {
        d::destruct_bound(&mut mem(), a1, 0x0059_5f80, 0x0059_ca60, 0x0058_57b0)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderPassInfo__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderPassInfo__constructInstance @ 0x004d_ffb0 => extern "thiscall" fn s_004dffb0(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0), Init::Word(0x8, 0x0), Init::Word(0xc, 0x0), Init::Word(0x10, 0x0), Init::Word(0x14, 0x0), Init::Word(0x18, 0x0), Init::Word(0x1c, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PSceneRenderPass__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PSceneRenderPass__constructInstance @ 0x004d_ff50 => extern "thiscall" fn s_004dff50(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Address(0, 0x00c1_6bd4), Init::Word(0x4, 0x0), Init::Word(0x8, 0x0), Init::Word(0xc, 0x0), Init::Word(0x10, 0x0), Init::Word(0x14, 0x0), Init::Word(0x18, 0x0), Init::Word(0x1c, 0x0), Init::Word(0x20, 0x0), Init::Byte(0x24, 0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PSprite_PSpriteAnimationInfoChar__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PSprite_PSpriteAnimationInfoChar__constructInstance @ 0x0056_c920 => extern "thiscall" fn s_0056c920(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Float(0x8, 0x00b3_04f4), Init::Word(0x4, 0x0), Init::Word(0xc, 0x0), Init::Word(0x10, 0x0), Init::Word(0x14, 0x0), Init::Word(0x18, 0x0), Init::Word(0x1c, 0x0), Init::Word(0x20, 0x0), Init::Word(0x24, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PSprite_PSpriteAttributes__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PSprite_PSpriteAttributes__constructInstance @ 0x0056_c970 => extern "thiscall" fn s_0056c970(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0), Init::Word(0x8, 0x0), Init::Word(0xc, 0x3f80_0000), Init::Word(0x18, 0x3f80_0000), Init::Word(0x1c, 0x3f80_0000), Init::Word(0x10, 0x0), Init::Word(0x14, 0x0), Init::Float(0x20, 0x00b3_04f8), Init::Float(0x24, 0x00b3_04f8), Init::Float(0x28, 0x00b3_04fc), Init::Word(0x2c, 0x0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PSprite_PSpriteAnimationInfoInstance__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PSprite_PSpriteAnimationInfoInstance__constructInstance @ 0x0057_0150 => extern "thiscall" fn s_00570150(_a0: u32, a1: u32) -> u8 {
        d::construct(&mut mem(), a1, &[Init::Word(0x0, 0x0), Init::Float(0x14, 0x00b3_0710), Init::Word(0x4, 0x0), Init::Word(0x8, 0x0), Init::Word(0x18, 0x0), Init::Word(0x1c, 0x0), Init::Word(0xc, 0xffff_ffff), Init::Word(0x10, 0x0), Init::Half(0x20, 0)])
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PRendering_PMeshInstanceSegmentStreamBinding_const__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PRendering_PMeshInstanceSegmentStreamBinding_const__destructInstance @ 0x004f_7750 => extern "thiscall" fn s_004f7750(_a0: u32, a1: u32) -> u8 {
        a::destruct_sharray(&mut mem(), a1, 0, false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PMeshInstanceSegmentContext__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PMeshInstanceSegmentContext__destructInstance @ 0x004f_77b0 => extern "thiscall" fn s_004f77b0(_a0: u32, a1: u32) -> u8 {
        a::destruct_sharray(&mut mem(), a1, 0x1c, false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PGeometry_PDataBlockBufferD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PGeometry_PDataBlockBufferD3D11__destructInstance @ 0x0056_2310 => extern "thiscall" fn s_00562310(_a0: u32, a1: u32) -> u8 {
        a::destruct_sharray(&mut mem(), a1, 0, true)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PGeometry_PIndexDataBlockBufferD3D11__destructInstance`.
    Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PGeometry_PIndexDataBlockBufferD3D11__destructInstance @ 0x0056_2360 => extern "thiscall" fn s_00562360(_a0: u32, a1: u32) -> u8 {
        a::destruct_sharray(&mut mem(), a1, 0, true)
    }
    /// `Phyre_FreeSharrayItems8_004ec310`.
    Phyre_FreeSharrayItems8_004ec310 @ 0x004e_c310 => extern "C" fn s_004ec310(a0: u32, a1: u32) {
        a::free_items(&mut mem(), a0, a1, 8, 0, true);
    }
    /// `Phyre_FreeSharrayItems24_004f4fc0`.
    Phyre_FreeSharrayItems24_004f4fc0 @ 0x004f_4fc0 => extern "C" fn s_004f4fc0(a0: u32, a1: u32) {
        a::free_items(&mut mem(), a0, a1, 0x24, 0x1c, false);
    }
    /// `Phyre_PRendering_PMeshInstanceSegmentContext_constructArray_004f6380`.
    Phyre_PRendering_PMeshInstanceSegmentContext_constructArray_004f6380 @ 0x004f_6380 => extern "C" fn s_004f6380(a0: u32, a1: u32) {
        a::init_items(&mut mem(), a0, a1, 0x24, &[Init::Word(0x10, 0x0), Init::Word(0x0, 0x0), Init::Word(0x4, 0x0), Init::Word(0x8, 0x0), Init::Word(0xc, 0x0), Init::Word(0x14, 0xffff_ffff), Init::Word(0x18, 0x0), Init::Word(0x1c, 0x0), Init::Word(0x20, 0x0)]);
    }
    /// `Phyre_ZeroArray28_0058b070`.
    Phyre_ZeroArray28_0058b070 @ 0x0058_b070 => extern "C" fn s_0058b070(a0: u32, a1: u32) {
        a::init_items(&mut mem(), a0, a1, 0x1c, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0), Init::Word(0x8, 0x0), Init::Word(0xc, 0x0), Init::Word(0x10, 0x0), Init::Word(0x14, 0x0), Init::Word(0x18, 0x0)]);
    }
    /// `Phyre_PArray_PShaderPassInfo_setStorage_004e1610`.
    Phyre_PArray_PShaderPassInfo_setStorage_004e1610 @ 0x004e_1610 => extern "thiscall" fn s_004e1610(a0: u32, a1: u32, a2: u32) -> u32 {
        a::set_storage(&mut mem(), a0, a1, a2, Some(0x004d_d770))
    }
    /// `Phyre_PArray_PMaterialSwitch_setStorage_004e7950`.
    Phyre_PArray_PMaterialSwitch_setStorage_004e7950 @ 0x004e_7950 => extern "thiscall" fn s_004e7950(a0: u32, a1: u32, a2: u32) -> u32 {
        a::set_storage(&mut mem(), a0, a1, a2, Some(0x004e_4df0))
    }
    /// `Phyre_PArray_PNodeContext_setStorage_004f0420`.
    Phyre_PArray_PNodeContext_setStorage_004f0420 @ 0x004f_0420 => extern "thiscall" fn s_004f0420(a0: u32, a1: u32, a2: u32) -> u32 {
        a::set_storage(&mut mem(), a0, a1, a2, Some(0x004e_c310))
    }
    /// `Phyre_PArray_PStreamInputDescD3D11_setStorage_0059b9d0`.
    Phyre_PArray_PStreamInputDescD3D11_setStorage_0059b9d0 @ 0x0059_b9d0 => extern "thiscall" fn s_0059b9d0(a0: u32, a1: u32, a2: u32) -> u32 {
        a::set_storage(&mut mem(), a0, a1, a2, Some(0x0058_8190))
    }
    /// `Phyre_PArray_setStorage_004f02e0`.
    Phyre_PArray_setStorage_004f02e0 @ 0x004f_02e0 => extern "thiscall" fn s_004f02e0(a0: u32, a1: u32, a2: u32) -> u32 {
        a::set_storage(&mut mem(), a0, a1, a2, None)
    }
    /// `Phyre_PArray_setStorage_004f0330`.
    Phyre_PArray_setStorage_004f0330 @ 0x004f_0330 => extern "thiscall" fn s_004f0330(a0: u32, a1: u32, a2: u32) -> u32 {
        a::set_storage(&mut mem(), a0, a1, a2, None)
    }
    /// `Phyre_PArray_setStorage_004f0380`.
    Phyre_PArray_setStorage_004f0380 @ 0x004f_0380 => extern "thiscall" fn s_004f0380(a0: u32, a1: u32, a2: u32) -> u32 {
        a::set_storage(&mut mem(), a0, a1, a2, None)
    }
    /// `Phyre_PArray_setStorage_004f03d0`.
    Phyre_PArray_setStorage_004f03d0 @ 0x004f_03d0 => extern "thiscall" fn s_004f03d0(a0: u32, a1: u32, a2: u32) -> u32 {
        a::set_storage(&mut mem(), a0, a1, a2, None)
    }
    /// `Phyre_PArray_setStorage_004fcc00`.
    Phyre_PArray_setStorage_004fcc00 @ 0x004f_cc00 => extern "thiscall" fn s_004fcc00(a0: u32, a1: u32, a2: u32) -> u32 {
        a::set_storage(&mut mem(), a0, a1, a2, None)
    }
    /// `Phyre_PArray_PShader_clear_004db710`.
    Phyre_PArray_PShader_clear_004db710 @ 0x004d_b710 => extern "thiscall" fn s_004db710(a0: u32) {
        a::clear_with_dtor(&mut mem(), a0, 0x004b_bf50, 0x20);
    }
    /// `Phyre_PArray_PSceneRenderPass_clear_004e3830`.
    Phyre_PArray_PSceneRenderPass_clear_004e3830 @ 0x004e_3830 => extern "thiscall" fn s_004e3830(a0: u32) {
        a::clear_with_dtor(&mut mem(), a0, 0x004d_b9d0, 0x28);
    }
    /// `Phyre_PSharray_element20_0055e0f0`.
    Phyre_PSharray_element20_0055e0f0 @ 0x0055_e0f0 => extern "thiscall" fn s_0055e0f0(a0: u32, a1: u32) -> u32 {
        a::element(&mut mem(), a0, a1, 0x14)
    }
    /// `Phyre_PSharray_element28_00585c90`.
    Phyre_PSharray_element28_00585c90 @ 0x0058_5c90 => extern "thiscall" fn s_00585c90(a0: u32, a1: u32) -> u32 {
        a::element(&mut mem(), a0, a1, 0x1c)
    }
    /// `Phyre_PRendering_BoundElementWord_00593af0`.
    Phyre_PRendering_BoundElementWord_00593af0 @ 0x0059_3af0 => extern "thiscall" fn s_00593af0(a0: u32, a1: u32) -> u32 {
        a::bound_element_word(&mut mem(), a0, a1)
    }
    /// `Phyre_PRendering_PSceneRenderPassType_find_004dd860`.
    Phyre_PRendering_PSceneRenderPassType_find_004dd860 @ 0x004d_d860 => extern "C" fn s_004dd860(a0: u32) -> u32 {
        x::find_pass_type(&mut mem(), a0)
    }
    /// `Phyre_PRendering_PSceneRenderPassType_findBy_004dd8c0`.
    Phyre_PRendering_PSceneRenderPassType_findBy_004dd8c0 @ 0x004d_d8c0 => extern "C" fn s_004dd8c0(a0: u32) -> u32 {
        x::find_pass_type_by(&mut mem(), a0, 0x00b0_c398)
    }
    /// `Phyre_PRendering_PSceneRenderPassType_PSceneRenderPassType`.
    Phyre_PRendering_PSceneRenderPassType_PSceneRenderPassType @ 0x004d_b230 => extern "thiscall" fn s_004db230(a0: u32, a1: u32, a2: u32) -> u32 {
        x::pass_type_init(&mut mem(), a0, a1, a2, 0x004d_d860)
    }
    /// `Phyre_PRendering_PSceneRenderPassType_PSceneRenderPassType_004db280`.
    Phyre_PRendering_PSceneRenderPassType_PSceneRenderPassType_004db280 @ 0x004d_b280 => extern "thiscall" fn s_004db280(a0: u32, _a1: u32, a2: u32) -> u32 {
        x::pass_type_init(&mut mem(), a0, a2, 1, 0x004d_d860)
    }
    /// `Phyre_PRendering_NameIs_004ea7a0`.
    Phyre_PRendering_NameIs_004ea7a0 @ 0x004e_a7a0 => extern "thiscall" fn s_004ea7a0(a0: u32, a1: u32) -> u8 {
        x::name_is(&mut mem(), a0, a1)
    }
    /// `Phyre_PRendering_PMeshInstanceBounds_setMinAndMax`.
    Phyre_PRendering_PMeshInstanceBounds_setMinAndMax @ 0x0050_0bd0 => extern "thiscall" fn s_00500bd0(a0: u32, a1: u32, a2: u32) -> u32 {
        x::set_min_and_max(&mut mem(), a0, a1, a2)
    }
    /// `Phyre_PRendering_InitIdentity_00504ec0`.
    Phyre_PRendering_InitIdentity_00504ec0 @ 0x0050_4ec0 => extern "thiscall" fn s_00504ec0(a0: u32) -> u32 {
        x::init_fields(&mut mem(), a0, &[Init::Word(0x0, 0x0), Init::Word(0x4, 0x0), Init::Word(0x8, 0x0), Init::Word(0xc, 0x0), Init::Word(0x10, 0x3f80_0000), Init::Word(0x14, 0x0), Init::Word(0x18, 0x0), Init::Word(0x1c, 0x0), Init::Word(0x20, 0x0), Init::Word(0x24, 0x3f80_0000), Init::Word(0x28, 0x0), Init::Word(0x2c, 0x0), Init::Word(0x30, 0x0), Init::Word(0x34, 0x0), Init::Word(0x38, 0x3f80_0000), Init::Word(0x3c, 0x0), Init::Word(0x40, 0x0), Init::Word(0x44, 0x0), Init::Word(0x48, 0x0), Init::Word(0x4c, 0x3f80_0000), Init::Word(0x50, 0x0)])
    }
    /// `Phyre_PRendering_PSamplerState_initDefaults_00583100`.
    Phyre_PRendering_PSamplerState_initDefaults_00583100 @ 0x0058_3100 => extern "thiscall" fn s_00583100(a0: u32, _a1: u32) -> u32 {
        x::init_fields(&mut mem(), a0, &[Init::Word(0x0, 0x15), Init::Word(0x10, 0x0), Init::Word(0x4, 0x3), Init::Word(0x8, 0x3), Init::Word(0xc, 0x3), Init::Word(0x14, 0x1), Init::Word(0x18, 0x1), Init::Word(0x1c, 0x3f80_0000), Init::Word(0x20, 0x3f80_0000), Init::Word(0x24, 0x3f80_0000), Init::Word(0x28, 0x3f80_0000), Init::Float(0x2c, 0x00b1_f2d0), Init::Float(0x30, 0x00b1_7504)])
    }
    /// `Phyre_PRendering_Ctor_00583a60`.
    Phyre_PRendering_Ctor_00583a60 @ 0x0058_3a60 => extern "thiscall" fn s_00583a60(a0: u32, a1: u32) -> u32 {
        x::ctor_arg_then_zero(&mut mem(), a0, a1, 0x004c_78a0, &[0x10, 0x14, 0x18, 0x1c, 0x20, 0x24])
    }
    /// `Phyre_PRendering_Ctor_00583b80`.
    Phyre_PRendering_Ctor_00583b80 @ 0x0058_3b80 => extern "thiscall" fn s_00583b80(a0: u32, a1: u32) -> u32 {
        x::first_then_zero(&mut mem(), a0, a1, 7)
    }
    /// `PParameterCaptureBuffer_setValue`.
    PParameterCaptureBuffer_setValue @ 0x004f_edc0 => extern "thiscall" fn s_004fedc0(a0: u32, a1: u32, a2: u32) -> u32 {
        x::set_value(&mut mem(), a0, a1, a2, 0x0094_9256)
    }
    /// `PEffect_buildNodeContextKey`.
    PEffect_buildNodeContextKey @ 0x004e_f3f0 => extern "thiscall" fn s_004ef3f0(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32) -> u32 {
        x::build_node_context_key(&mut mem(), a0, [a1, a2, a3, a4, a5, a6])
    }
    /// `Phyre_TreeFind_00581670`.
    Phyre_TreeFind_00581670 @ 0x0058_1670 => extern "thiscall" fn s_00581670(a0: u32, a1: u32) -> u32 {
        x::tree_find(&mut mem(), a0, a1)
    }
    /// `Phyre_PRendering_MapMode8_0058a9a0`.
    Phyre_PRendering_MapMode8_0058a9a0 @ 0x0058_a9a0 => extern "C" fn s_0058a9a0(a0: u32) -> u32 {
        x::map_mode8(a0)
    }
    /// `Phyre_PGeometry_FormatSize_0055bc40`.
    Phyre_PGeometry_FormatSize_0055bc40 @ 0x0055_bc40 => extern "C" fn s_0055bc40(a0: u32) -> u32 {
        x::format_size(a0)
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PContextVariantFoldingTable_4__ctor`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PContextVariantFoldingTable_4__ctor @ 0x004d_a8e0 => extern "thiscall" fn s_004da8e0(a0: u32) -> u32 {
        dc::bridge::abstract_ctor(&mut mem(), a0, &dc::Abstract { name: dc::Name::Global(0x00ca_5f48), size: 0x8, vtable: 0x00b2_0bcc, default_buffer: 0x00ca_5f60, validation_buffer: 0x00ca_5f70, write_mask_buffer: 0x00ca_5f80 })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PShader_4__ctor`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PShader_4__ctor @ 0x004d_a980 => extern "thiscall" fn s_004da980(a0: u32) -> u32 {
        dc::bridge::abstract_ctor(&mut mem(), a0, &dc::Abstract { name: dc::Name::Global(0x00ca_5f50), size: 0x8, vtable: 0x00b2_0ddc, default_buffer: 0x00ca_5fc0, validation_buffer: 0x00ca_5fd0, write_mask_buffer: 0x00ca_5fe0 })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PShaderPassInfo_4__ctor`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PShaderPassInfo_4__ctor @ 0x004d_aa20 => extern "thiscall" fn s_004daa20(a0: u32) -> u32 {
        dc::bridge::abstract_ctor(&mut mem(), a0, &dc::Abstract { name: dc::Name::Global(0x00ca_5f4c), size: 0x8, vtable: 0x00b2_0cd4, default_buffer: 0x00ca_5f90, validation_buffer: 0x00ca_5fa0, write_mask_buffer: 0x00ca_5fb0 })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PSceneRenderPass_4__ctor_004e3100`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PSceneRenderPass_4__ctor_004e3100 @ 0x004e_3100 => extern "thiscall" fn s_004e3100(a0: u32) -> u32 {
        dc::bridge::abstract_ctor(&mut mem(), a0, &dc::Abstract { name: dc::Name::Global(0x00ca_6654), size: 0x8, vtable: 0x00b2_14f4, default_buffer: 0x00ca_6660, validation_buffer: 0x00ca_6670, write_mask_buffer: 0x00ca_6680 })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PMaterialSwitch_4__ctor`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PMaterialSwitch_4__ctor @ 0x004e_31a0 => extern "thiscall" fn s_004e31a0(a0: u32) -> u32 {
        dc::bridge::abstract_ctor(&mut mem(), a0, &dc::Abstract { name: dc::Name::Global(0x00ca_665c), size: 0x8, vtable: 0x00b2_1704, default_buffer: 0x00ca_66c0, validation_buffer: 0x00ca_66d0, write_mask_buffer: 0x00ca_66e0 })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PSceneRenderPass_4__ctor_004e3240`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PSceneRenderPass_4__ctor_004e3240 @ 0x004e_3240 => extern "thiscall" fn s_004e3240(a0: u32) -> u32 {
        dc::bridge::abstract_ctor(&mut mem(), a0, &dc::Abstract { name: dc::Name::Global(0x00ca_6658), size: 0x8, vtable: 0x00b2_15fc, default_buffer: 0x00ca_6690, validation_buffer: 0x00ca_66a0, write_mask_buffer: 0x00ca_66b0 })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PEffectVariant_4__ctor`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PEffectVariant_4__ctor @ 0x004e_9940 => extern "thiscall" fn s_004e9940(a0: u32) -> u32 {
        dc::bridge::abstract_ctor(&mut mem(), a0, &dc::Abstract { name: dc::Name::Global(0x00ca_7000), size: 0x8, vtable: 0x00b2_2344, default_buffer: 0x00ca_70d0, validation_buffer: 0x00ca_70e0, write_mask_buffer: 0x00ca_70f0 })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PContextSwitch_const_4__ctor`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PContextSwitch_const_4__ctor @ 0x004e_99e0 => extern "thiscall" fn s_004e99e0(a0: u32) -> u32 {
        dc::bridge::abstract_ctor(&mut mem(), a0, &dc::Abstract { name: dc::Name::Global(0x00ca_6ff4), size: 0x8, vtable: 0x00b2_202c, default_buffer: 0x00ca_7040, validation_buffer: 0x00ca_7050, write_mask_buffer: 0x00ca_7060 })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PLightType_const_4__ctor`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PLightType_const_4__ctor @ 0x004e_9a80 => extern "thiscall" fn s_004e9a80(a0: u32) -> u32 {
        dc::bridge::abstract_ctor(&mut mem(), a0, &dc::Abstract { name: dc::Name::Global(0x00ca_6ffc), size: 0x8, vtable: 0x00b2_223c, default_buffer: 0x00ca_70a0, validation_buffer: 0x00ca_70b0, write_mask_buffer: 0x00ca_70c0 })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PShadowCasterType_const_4__ctor`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PShadowCasterType_const_4__ctor @ 0x004e_9b20 => extern "thiscall" fn s_004e9b20(a0: u32) -> u32 {
        dc::bridge::abstract_ctor(&mut mem(), a0, &dc::Abstract { name: dc::Name::Global(0x00ca_6ff8), size: 0x8, vtable: 0x00b2_2134, default_buffer: 0x00ca_7070, validation_buffer: 0x00ca_7080, write_mask_buffer: 0x00ca_7090 })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PNodeContext_4__ctor`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PNodeContext_4__ctor @ 0x004e_9bc0 => extern "thiscall" fn s_004e9bc0(a0: u32) -> u32 {
        dc::bridge::abstract_ctor(&mut mem(), a0, &dc::Abstract { name: dc::Name::Global(0x00ca_6ff0), size: 0x8, vtable: 0x00b2_1f24, default_buffer: 0x00ca_7010, validation_buffer: 0x00ca_7020, write_mask_buffer: 0x00ca_7030 })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PMeshInstanceSegmentContext_4__ctor`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PMeshInstanceSegmentContext_4__ctor @ 0x004f_20c0 => extern "thiscall" fn s_004f20c0(a0: u32) -> u32 {
        dc::bridge::abstract_ctor(&mut mem(), a0, &dc::Abstract { name: dc::Name::Global(0x00ca_7e88), size: 0x8, vtable: 0x00b2_325c, default_buffer: 0x00ca_7f00, validation_buffer: 0x00ca_7f10, write_mask_buffer: 0x00ca_7f20 })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PSharray_Phyre_PRendering_PMeshInstanceSegmentStreamBinding_const__ctor`.
    Phyre_PClassDescriptorAbstract_Phyre_PSharray_Phyre_PRendering_PMeshInstanceSegmentStreamBinding_const__ctor @ 0x004f_2160 => extern "thiscall" fn s_004f2160(a0: u32) -> u32 {
        dc::bridge::abstract_ctor(&mut mem(), a0, &dc::Abstract { name: dc::Name::Global(0x00ca_7e84), size: 0x8, vtable: 0x00b2_3154, default_buffer: 0x00ca_7ed0, validation_buffer: 0x00ca_7ee0, write_mask_buffer: 0x00ca_7ef0 })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkInstance_4__ctor`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkInstance_4__ctor @ 0x004f_b1e0 => extern "thiscall" fn s_004fb1e0(a0: u32) -> u32 {
        dc::bridge::abstract_ctor(&mut mem(), a0, &dc::Abstract { name: dc::Name::Global(0x00ca_8264), size: 0x8, vtable: 0x00b2_39ec, default_buffer: 0x00ca_8270, validation_buffer: 0x00ca_8280, write_mask_buffer: 0x00ca_8290 })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PGeometry_PDataBlockD3D11_4__ctor`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PGeometry_PDataBlockD3D11_4__ctor @ 0x0055_d460 => extern "thiscall" fn s_0055d460(a0: u32) -> u32 {
        dc::bridge::abstract_ctor(&mut mem(), a0, &dc::Abstract { name: dc::Name::Global(0x00cb_1b5c), size: 0x8, vtable: 0x00b2_ed8c, default_buffer: 0x00cb_1b70, validation_buffer: 0x00cb_1b80, write_mask_buffer: 0x00cb_1b90 })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PSprite_PSubTextureInfo_4__ctor`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PSprite_PSubTextureInfo_4__ctor @ 0x0056_6810 => extern "thiscall" fn s_00566810(a0: u32) -> u32 {
        dc::bridge::abstract_ctor(&mut mem(), a0, &dc::Abstract { name: dc::Name::Global(0x00cb_310c), size: 0x8, vtable: 0x00b3_0294, default_buffer: 0x00cb_3130, validation_buffer: 0x00cb_3140, write_mask_buffer: 0x00cb_3150 })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PRendering_PMeshInstance__ctor`.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PMeshInstance__ctor @ 0x004f_2200 => extern "thiscall" fn s_004f2200(a0: u32) -> u32 {
        dc::bridge::abstract_ctor(&mut mem(), a0, &dc::Abstract { name: dc::Name::Image(0x00b2_2bdc), size: 0x84, vtable: 0x00b2_2b54, default_buffer: 0x00ca_7ce0, validation_buffer: 0x00ca_7d70, write_mask_buffer: 0x00ca_7e00 })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PSprite_PSpriteCollection__ctor`.
    Phyre_PClassDescriptorAbstract_Phyre_PSprite_PSpriteCollection__ctor @ 0x0056_6a80 => extern "thiscall" fn s_00566a80(a0: u32) -> u32 {
        dc::bridge::abstract_ctor(&mut mem(), a0, &dc::Abstract { name: dc::Name::Image(0x00b2_f92c), size: 0xdc, vtable: 0x00b2_f8a4, default_buffer: 0x00cb_2e70, validation_buffer: 0x00cb_2f50, write_mask_buffer: 0x00cb_3030 })
    }
    /// `Phyre_PArray_Phyre_PRendering_PContextVariantFoldingTable_4__DeclareClassDescriptor`.
    Phyre_PArray_Phyre_PRendering_PContextVariantFoldingTable_4__DeclareClassDescriptor @ 0x004d_d0c0 => extern "C" fn s_004dd0c0(a0: u32) -> u32 {
        dc::bridge::declare(&mut mem(), a0, &dc::Declare { desc: 0x00ca_5aa0, ctor: 0x004d_a8e0, vtable: 0x00b2_0c14, name: 0x00ca_5f48, element: dc::Element::Descriptor(0x00ca_5660), flags: 0, member_vtable: 0x00b2_0c58, set_storage: 0x004e_15a0, atexit: [0x00af_ecd0, 0x00af_edb0, 0x00af_eca0, 0x00af_ede0] })
    }
    /// `Phyre_PArray_Phyre_PRendering_PShader_4__DeclareClassDescriptor`.
    Phyre_PArray_Phyre_PRendering_PShader_4__DeclareClassDescriptor @ 0x004d_d2d0 => extern "C" fn s_004dd2d0(a0: u32) -> u32 {
        dc::bridge::declare(&mut mem(), a0, &dc::Declare { desc: 0x00ca_5ce0, ctor: 0x004d_a980, vtable: 0x00b2_0e24, name: 0x00ca_5f50, element: dc::Element::Descriptor(0x00ca_2548), flags: 0, member_vtable: 0x00b2_0e68, set_storage: 0x004e_15f0, atexit: [0x00af_ecf0, 0x00af_edc0, 0x00af_ecb0, 0x00af_edf0] })
    }
    /// `Phyre_PArray_Phyre_PRendering_PShaderPassInfo_4__DeclareClassDescriptor`.
    Phyre_PArray_Phyre_PRendering_PShaderPassInfo_4__DeclareClassDescriptor @ 0x004d_d4e0 => extern "C" fn s_004dd4e0(a0: u32) -> u32 {
        dc::bridge::declare(&mut mem(), a0, &dc::Declare { desc: 0x00ca_5bc0, ctor: 0x004d_aa20, vtable: 0x00b2_0d1c, name: 0x00ca_5f4c, element: dc::Element::Descriptor(0x00ca_55c8), flags: 0, member_vtable: 0x00b2_0d60, set_storage: 0x004e_1610, atexit: [0x00af_ed10, 0x00af_edd0, 0x00af_ecc0, 0x00af_ee00] })
    }
    /// `Phyre_PArray_Phyre_PRendering_PSceneRenderPass_4__DeclareClassDescriptor_004e47b0`.
    Phyre_PArray_Phyre_PRendering_PSceneRenderPass_4__DeclareClassDescriptor_004e47b0 @ 0x004e_47b0 => extern "C" fn s_004e47b0(a0: u32) -> u32 {
        dc::bridge::declare(&mut mem(), a0, &dc::Declare { desc: 0x00ca_6240, ctor: 0x004e_3100, vtable: 0x00b2_153c, name: 0x00ca_6654, element: dc::Element::Descriptor(0x00ca_5530), flags: 2, member_vtable: 0x00b2_1580, set_storage: 0x004e_7900, atexit: [0x00af_ef20, 0x00af_ef80, 0x00af_eef0, 0x00af_efc0] })
    }
    /// `Phyre_PArray_Phyre_PRendering_PMaterialSwitch_4__DeclareClassDescriptor`.
    Phyre_PArray_Phyre_PRendering_PMaterialSwitch_4__DeclareClassDescriptor @ 0x004e_49c0 => extern "C" fn s_004e49c0(a0: u32) -> u32 {
        dc::bridge::declare(&mut mem(), a0, &dc::Declare { desc: 0x00ca_6480, ctor: 0x004e_31a0, vtable: 0x00b2_174c, name: 0x00ca_665c, element: dc::Element::Descriptor(0x00ca_51d0), flags: 0, member_vtable: 0x00b2_1790, set_storage: 0x004e_7950, atexit: [0x00af_ef40, 0x00af_ef90, 0x00af_ef00, 0x00af_efd0] })
    }
    /// `Phyre_PArray_Phyre_PRendering_PSceneRenderPass_4__DeclareClassDescriptor_004e4bd0`.
    Phyre_PArray_Phyre_PRendering_PSceneRenderPass_4__DeclareClassDescriptor_004e4bd0 @ 0x004e_4bd0 => extern "C" fn s_004e4bd0(a0: u32) -> u32 {
        dc::bridge::declare(&mut mem(), a0, &dc::Declare { desc: 0x00ca_6360, ctor: 0x004e_3240, vtable: 0x00b2_1644, name: 0x00ca_6658, element: dc::Element::Descriptor(0x00ca_5530), flags: 0, member_vtable: 0x00b2_1688, set_storage: 0x004e_79a0, atexit: [0x00af_ef60, 0x00af_efa0, 0x00af_ef10, 0x00af_efe0] })
    }
    /// `Phyre_PArray_Phyre_PRendering_PEffectVariant_4__DeclareClassDescriptor`.
    Phyre_PArray_Phyre_PRendering_PEffectVariant_4__DeclareClassDescriptor @ 0x004e_b850 => extern "C" fn s_004eb850(a0: u32) -> u32 {
        dc::bridge::declare(&mut mem(), a0, &dc::Declare { desc: 0x00ca_6e10, ctor: 0x004e_9940, vtable: 0x00b2_238c, name: 0x00ca_7000, element: dc::Element::Descriptor(0x00ca_5ff0), flags: 2, member_vtable: 0x00b2_23d0, set_storage: 0x004f_02e0, atexit: [0x00af_f0e0, 0x00af_f1a0, 0x00af_f090, 0x00af_f220] })
    }
    /// `Phyre_PArray_Phyre_PRendering_PNodeContext_4__DeclareClassDescriptor`.
    Phyre_PArray_Phyre_PRendering_PNodeContext_4__DeclareClassDescriptor @ 0x004e_c090 => extern "C" fn s_004ec090(a0: u32) -> u32 {
        dc::bridge::declare(&mut mem(), a0, &dc::Declare { desc: 0x00ca_6990, ctor: 0x004e_9bc0, vtable: 0x00b2_1f6c, name: 0x00ca_6ff0, element: dc::Element::Descriptor(0x00ca_4bc8), flags: 0, member_vtable: 0x00b2_1fb0, set_storage: 0x004f_0420, atexit: [0x00af_f160, 0x00af_f1e0, 0x00af_f0d0, 0x00af_f260] })
    }
    /// `Phyre_PArray_Phyre_PRendering_PMeshInstanceSegmentContext_4__DeclareClassDescriptor`.
    Phyre_PArray_Phyre_PRendering_PMeshInstanceSegmentContext_4__DeclareClassDescriptor @ 0x004f_4b50 => extern "C" fn s_004f4b50(a0: u32) -> u32 {
        dc::bridge::declare(&mut mem(), a0, &dc::Declare { desc: 0x00ca_7ac8, ctor: 0x004f_20c0, vtable: 0x00b2_32a4, name: 0x00ca_7e88, element: dc::Element::Descriptor(0x00ca_7190), flags: 0, member_vtable: 0x00b2_32e8, set_storage: 0x004f_90b0, atexit: [0x00af_f350, 0x00af_f550, 0x00af_f330, 0x00af_f5a0] })
    }
    /// `Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkInstance_4__DeclareClassDescriptor`.
    Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkInstance_4__DeclareClassDescriptor @ 0x004f_ba50 => extern "C" fn s_004fba50(a0: u32) -> u32 {
        dc::bridge::declare(&mut mem(), a0, &dc::Declare { desc: 0x00ca_80f0, ctor: 0x004f_b1e0, vtable: 0x00b2_3a34, name: 0x00ca_8264, element: dc::Element::Descriptor(0x00cb_bb38), flags: 2, member_vtable: 0x00b2_3a78, set_storage: 0x004f_cc00, atexit: [0x00af_f830, 0x00af_f850, 0x00af_f820, 0x00af_f860] })
    }
    /// `Phyre_PArray_Phyre_PAnimation_PBlendableAnimationSource_8u__DeclareClassDescriptor`.
    Phyre_PArray_Phyre_PAnimation_PBlendableAnimationSource_8u__DeclareClassDescriptor @ 0x0054_0970 => extern "C" fn s_00540970(a0: u32) -> u32 {
        dc::bridge::declare(&mut mem(), a0, &dc::Declare { desc: 0x00ca_e060, ctor: 0x0053_e220, vtable: 0x00b2_bb04, name: 0x00ca_e4b4, element: dc::Element::Descriptor(0x00ca_d9a8), flags: 0, member_vtable: 0x00b2_bb48, set_storage: 0x0054_41e0, atexit: [0x00b0_1270, 0x00b0_1540, 0x00b0_1260, 0x00b0_1550] })
    }
    /// `Phyre_PArray_Phyre_PGeometry_PDataBlockD3D11_4__DeclareClassDescriptor`.
    Phyre_PArray_Phyre_PGeometry_PDataBlockD3D11_4__DeclareClassDescriptor @ 0x0055_edf0 => extern "C" fn s_0055edf0(a0: u32) -> u32 {
        dc::bridge::declare(&mut mem(), a0, &dc::Declare { desc: 0x00cb_1468, ctor: 0x0055_d460, vtable: 0x00b2_edd4, name: 0x00cb_1b5c, element: dc::Element::Descriptor(0x00cb_1250), flags: 0, member_vtable: 0x00b2_ee18, set_storage: 0x0056_42c0, atexit: [0x00b0_2a70, 0x00b0_2af0, 0x00b0_2a40, 0x00b0_2b40] })
    }
    /// `Phyre_PArray_Phyre_PSprite_PSubTextureInfo_4__DeclareClassDescriptor`.
    Phyre_PArray_Phyre_PSprite_PSubTextureInfo_4__DeclareClassDescriptor @ 0x0056_a4d0 => extern "C" fn s_0056a4d0(a0: u32) -> u32 {
        dc::bridge::declare(&mut mem(), a0, &dc::Declare { desc: 0x00cb_2a60, ctor: 0x0056_6810, vtable: 0x00b3_02dc, name: 0x00cb_310c, element: dc::Element::Descriptor(0x00cb_1d88), flags: 0, member_vtable: 0x00b3_0320, set_storage: 0x0056_ebf0, atexit: [0x00b0_2d00, 0x00b0_2e70, 0x00b0_2cf0, 0x00b0_2ea0] })
    }
    /// `Phyre_PArray_Phyre_PRendering_PContextSwitch_const_4__DeclareClassDescriptor`.
    Phyre_PArray_Phyre_PRendering_PContextSwitch_const_4__DeclareClassDescriptor @ 0x004e_ba60 => extern "C" fn s_004eba60(a0: u32) -> u32 {
        dc::bridge::declare(&mut mem(), a0, &dc::Declare { desc: 0x00ca_6ab0, ctor: 0x004e_99e0, vtable: 0x00b2_2074, name: 0x00ca_6ff4, element: dc::Element::Getter(0x004d_0e20), flags: 2, member_vtable: 0x00b2_20b8, set_storage: 0x004f_0330, atexit: [0x00af_f100, 0x00af_f1b0, 0x00af_f0a0, 0x00af_f230] })
    }
    /// `Phyre_PArray_Phyre_PRendering_PLightType_const_4__DeclareClassDescriptor`.
    Phyre_PArray_Phyre_PRendering_PLightType_const_4__DeclareClassDescriptor @ 0x004e_bc70 => extern "C" fn s_004ebc70(a0: u32) -> u32 {
        dc::bridge::declare(&mut mem(), a0, &dc::Declare { desc: 0x00ca_6cf0, ctor: 0x004e_9a80, vtable: 0x00b2_2284, name: 0x00ca_6ffc, element: dc::Element::Getter(0x0049_d050), flags: 2, member_vtable: 0x00b2_22c8, set_storage: 0x004f_0380, atexit: [0x00af_f120, 0x00af_f1c0, 0x00af_f0b0, 0x00af_f240] })
    }
    /// `Phyre_PArray_Phyre_PRendering_PShadowCasterType_const_4__DeclareClassDescriptor`.
    Phyre_PArray_Phyre_PRendering_PShadowCasterType_const_4__DeclareClassDescriptor @ 0x004e_be80 => extern "C" fn s_004ebe80(a0: u32) -> u32 {
        dc::bridge::declare(&mut mem(), a0, &dc::Declare { desc: 0x00ca_6bd0, ctor: 0x004e_9b20, vtable: 0x00b2_217c, name: 0x00ca_6ff8, element: dc::Element::Getter(0x004c_8a70), flags: 2, member_vtable: 0x00b2_21c0, set_storage: 0x004f_03d0, atexit: [0x00af_f140, 0x00af_f1d0, 0x00af_f0c0, 0x00af_f250] })
    }
    /// `Phyre_PRendering_PSamplerStateD3D11_dtor`.
    Phyre_PRendering_PSamplerStateD3D11_dtor @ 0x0058_5230 => extern "thiscall" fn s_00585230(a0: u32) -> u32 {
        dt::chain2(&mut mem(), a0, 0x0059_c770, 0x004b_e970)
    }
    /// `Phyre_PRendering_PShaderComputeProgramD3D11_dtor`.
    Phyre_PRendering_PShaderComputeProgramD3D11_dtor @ 0x0058_5290 => extern "thiscall" fn s_00585290(a0: u32) -> u32 {
        dt::chain2(&mut mem(), a0, 0x0059_c830, 0x0058_5610)
    }
    /// `Phyre_PRendering_PShaderFragmentProgramD3D11_dtor`.
    Phyre_PRendering_PShaderFragmentProgramD3D11_dtor @ 0x0058_5300 => extern "thiscall" fn s_00585300(a0: u32) -> u32 {
        dt::chain2(&mut mem(), a0, 0x0059_c890, 0x0058_5610)
    }
    /// `Phyre_PRendering_PShaderGeometryProgramD3D11_dtor`.
    Phyre_PRendering_PShaderGeometryProgramD3D11_dtor @ 0x0058_5360 => extern "thiscall" fn s_00585360(a0: u32) -> u32 {
        dt::chain2(&mut mem(), a0, 0x0059_c8f0, 0x0058_5610)
    }
    /// `Phyre_PRendering_PShaderComputeProgramD3D11_release`.
    Phyre_PRendering_PShaderComputeProgramD3D11_release @ 0x0059_c830 => extern "thiscall" fn s_0059c830(a0: u32) {
        dt::release_shader(&mut mem(), a0);
    }
    /// `Phyre_PRendering_PShaderFragmentProgramD3D11_release`.
    Phyre_PRendering_PShaderFragmentProgramD3D11_release @ 0x0059_c890 => extern "thiscall" fn s_0059c890(a0: u32) {
        dt::release_shader(&mut mem(), a0);
    }
    /// `Phyre_PRendering_PShaderGeometryProgramD3D11_release`.
    Phyre_PRendering_PShaderGeometryProgramD3D11_release @ 0x0059_c8f0 => extern "thiscall" fn s_0059c8f0(a0: u32) {
        dt::release_shader(&mut mem(), a0);
    }
    /// `Phyre_PRendering_PConstantBufferD3D11_dtor`.
    Phyre_PRendering_PConstantBufferD3D11_dtor @ 0x0058_4dc0 => extern "thiscall" fn s_00584dc0(a0: u32) -> u32 {
        dt::with_sharray(&mut mem(), a0, 0x0059_c540, 0xc, 0x0049_fa70)
    }
    /// `Phyre_PRendering_PStructuredBufferD3D11_dtor`.
    Phyre_PRendering_PStructuredBufferD3D11_dtor @ 0x0058_57b0 => extern "thiscall" fn s_005857b0(a0: u32) {
        dt::with_sharray(&mut mem(), a0, 0x0059_ca60, 0x14, 0x0050_1d30);
    }
    /// `Phyre_PRendering_PTexture2DD3D11_dtor`.
    Phyre_PRendering_PTexture2DD3D11_dtor @ 0x0058_5840 => extern "thiscall" fn s_00585840(a0: u32) -> u32 {
        dt::with_sharray(&mut mem(), a0, 0x0059_cc40, 0x24, 0x004c_3070)
    }
    /// `Phyre_PRendering_PTextureD3D11_dtor_005858d0`.
    Phyre_PRendering_PTextureD3D11_dtor_005858d0 @ 0x0058_58d0 => extern "thiscall" fn s_005858d0(a0: u32) -> u32 {
        dt::with_sharray(&mut mem(), a0, 0x0059_cf00, 0x28, 0x004c_4890)
    }
    /// `Phyre_PRendering_PTextureD3D11_dtor_00585960`.
    Phyre_PRendering_PTextureD3D11_dtor_00585960 @ 0x0058_5960 => extern "thiscall" fn s_00585960(a0: u32) -> u32 {
        dt::with_sharray(&mut mem(), a0, 0x0059_d0f0, 0x20, 0x004c_6150)
    }
    /// `Phyre_PArray_PShader_setStorage_004e13a0`.
    Phyre_PArray_PShader_setStorage_004e13a0 @ 0x004e_13a0 => extern "thiscall" fn s_004e13a0(a0: u32, a1: u32, a2: u32) -> u32 {
        dt::set_storage_dtor(&mut mem(), a0, a1, a2, 0x004b_bf50, 0x20)
    }
    /// `Phyre_PArray_PSceneRenderPass_setStorage_004e7780`.
    Phyre_PArray_PSceneRenderPass_setStorage_004e7780 @ 0x004e_7780 => extern "thiscall" fn s_004e7780(a0: u32, a1: u32, a2: u32) -> u32 {
        dt::set_storage_dtor(&mut mem(), a0, a1, a2, 0x004d_b9d0, 0x28)
    }
    /// `Phyre_PArray_PConstantBuffer_setStorage_004fed50`.
    Phyre_PArray_PConstantBuffer_setStorage_004fed50 @ 0x004f_ed50 => extern "thiscall" fn s_004fed50(a0: u32, a1: u32, a2: u32) -> u32 {
        dt::set_storage_dtor(&mut mem(), a0, a1, a2, 0x0049_fa10, 0x3c)
    }
    /// `Phyre_PRendering_PModifierNetworkDynamicMeshInstance_vslot1`.
    Phyre_PRendering_PModifierNetworkDynamicMeshInstance_vslot1 @ 0x004f_c8e0 => extern "thiscall" fn s_004fc8e0(a0: u32, a1: u32) -> u32 {
        let mut fr = super::StackFrame::<4>::new();
        sm::network_forward1(&mut mem(), a0, a1, fr.at())
    }
    /// `Phyre_PRendering_PModifierNetworkDynamicMeshInstance_vslot2`.
    Phyre_PRendering_PModifierNetworkDynamicMeshInstance_vslot2 @ 0x004f_c9d0 => extern "thiscall" fn s_004fc9d0(a0: u32, a1: u32, a2: u32) -> u32 {
        let mut fr = super::StackFrame::<4>::new();
        sm::network_forward2(&mut mem(), a0, a1, a2, fr.at())
    }
    /// `Phyre_PRendering_PTextureD3D11_bind_0058cf20`.
    Phyre_PRendering_PTextureD3D11_bind_0058cf20 @ 0x0058_cf20 => extern "thiscall" fn s_0058cf20(a0: u32) -> u32 {
        sm::bind_if_unset(&mut mem(), a0)
    }
    /// `Phyre_PRendering_PTexture2DD3D11_bind`.
    Phyre_PRendering_PTexture2DD3D11_bind @ 0x0058_db00 => extern "thiscall" fn s_0058db00(a0: u32) -> u32 {
        sm::texture_bind(&mut mem(), a0)
    }
    /// `PRendererD3D11_draw_00592b60`.
    PRendererD3D11_draw_00592b60 @ 0x0059_2b60 => extern "thiscall" fn s_00592b60(a0: u32, a1: u32, a2: u32, a3: u32) -> u8 {
        sm::draw(&mut mem(), a0, a1, a2, a3)
    }
    /// `Phyre_PRendering_PBounds_forward_005008c0`.
    Phyre_PRendering_PBounds_forward_005008c0 @ 0x0050_08c0 => extern "thiscall" fn s_005008c0(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        let mut fr = super::StackFrame::<8>::new();
        sm::bounds_forward(&mut mem(), a0, a1, a2, a3, fr.at())
    }
    /// `Phyre_PSharray_elementWord1_00593e30`.
    Phyre_PSharray_elementWord1_00593e30 @ 0x0059_3e30 => extern "thiscall" fn s_00593e30(a0: u32, a1: u32) -> u32 {
        sm::element_word1(&mut mem(), a0, a1)
    }
    /// `PRendererD3D11_setScissorRect_0059aaf0`.
    PRendererD3D11_setScissorRect_0059aaf0 @ 0x0059_aaf0 => extern "thiscall" fn s_0059aaf0(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32) -> u32 {
        let mut fr = super::StackFrame::<4>::new();
        sm::set_scissor(&mut mem(), a0, [a1, a2, a3, a4], fr.at())
    }
    /// `PShaderResourceTables_unbindAll_00598a80`.
    PShaderResourceTables_unbindAll_00598a80 @ 0x0059_8a80 => extern "stdcall" fn s_00598a80(a0: u32, a1: u32, a2: u32) -> u32 {
        sm::unbind_all(&mut mem(), a0, a1, a2)
    }
    /// `PShaderResourceTable_unbindSlot_00598b50`.
    PShaderResourceTable_unbindSlot_00598b50 @ 0x0059_8b50 => extern "thiscall" fn s_00598b50(a0: u32, _a1: u32, a2: u32) -> u32 {
        sm::unbind_table(&mut mem(), a0, a2)
    }
    /// `PRendererD3D11_mapPrimitiveTopology`.
    PRendererD3D11_mapPrimitiveTopology @ 0x0058_ab60 => extern "C" fn s_0058ab60(a0: u32) -> u32 {
        sm::topology(&mut mem(), a0)
    }
    /// `PRendererD3D11_setBlendFactors_005998f0`.
    PRendererD3D11_setBlendFactors_005998f0 @ 0x0059_98f0 => extern "thiscall" fn s_005998f0(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        sm::set_blend_pair(&mut mem(), a0, a1, a2, a3)
    }
    /// `PRendererD3D11_mapBlendFactor`.
    PRendererD3D11_mapBlendFactor @ 0x0058_a8f0 => extern "C" fn s_0058a8f0(a0: u32) -> u32 {
        sm::blend_factor(a0)
    }
    /// `PRendererD3D11_getDisplayModeList_005940d0`.
    PRendererD3D11_getDisplayModeList_005940d0 @ 0x0059_40d0 => extern "thiscall" fn s_005940d0(a0: u32, a1: u32, a2: u32) -> u32 {
        let mut fr = super::StackFrame::<2>::new();
        sm::query_and_set(&mut mem(), a0, a1, a2, fr.at())
    }
    /// `Phyre_PSprite_elementOffset_00563900`.
    Phyre_PSprite_elementOffset_00563900 @ 0x0056_3900 => extern "thiscall" fn s_00563900(a0: u32, a1: u32, a2: u32) -> u32 {
        let mut fr = super::StackFrame::<1>::new();
        sm::element_offset(&mut mem(), a0, a1, a2, fr.at())
    }
    /// `Phyre_PRendering_PMeshInstance_computeJointMatrixFromIdentity`.
    Phyre_PRendering_PMeshInstance_computeJointMatrixFromIdentity @ 0x004f_6f70 => extern "thiscall" fn s_004f6f70(a0: u32, a1: u32) -> u32 {
        sm::joint_from_identity(&mut mem(), a0, a1)
    }
    /// `PShaderResourceTables_bindAll_005934c0`.
    PShaderResourceTables_bindAll_005934c0 @ 0x0059_34c0 => extern "stdcall" fn s_005934c0(a0: u32, a1: u32) -> u32 {
        sm::bind_all(&mut mem(), a0, a1)
    }
    /// `PRenderStateCache_ctor_005833b0`.
    PRenderStateCache_ctor_005833b0 @ 0x0058_33b0 => extern "thiscall" fn s_005833b0(a0: u32) -> u32 {
        sm::state_cache_ctor(&mut mem(), a0)
    }
    /// `PShaderD3D11_releaseViews_0059c950`.
    PShaderD3D11_releaseViews_0059c950 @ 0x0059_c950 => extern "thiscall" fn s_0059c950(a0: u32) {
        sm::release_views(&mut mem(), a0);
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PIndirectArgsBufferD3D11__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PIndirectArgsBufferD3D11__constructInstance @ 0x0058_e650 => extern "thiscall" fn s_0058e650(_a0: u32, a1: u32) -> u8 {
        sm::construct_then(&mut mem(), a1, 0x0050_31e0, &[Init::Word(8, 0), Init::Word(0xc, 0), Init::Word(0x10, 0), Init::Byte(0x14, 0)])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PBlendableAnimationSource__constructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PBlendableAnimationSource__constructInstance @ 0x0054_2330 => extern "thiscall" fn s_00542330(_a0: u32, a1: u32) -> u8 {
        sm::construct_members(&mut mem(), a1, &[Init::Word(4, 0x3f80_0000), Init::Word(0, 0), Init::Word(8, 0x3f80_0000)], &[(0xc, 0x0052_25a0), (0x24, 0x0052_31e0), (0x38, 0x0052_5240)])
    }
    /// `Phyre_PRendering_GetRenderStateCache_004fd8b0`.
    Phyre_PRendering_GetRenderStateCache_004fd8b0 @ 0x004f_d8b0 => extern "C" fn s_004fd8b0() -> u32 {
        sm::static_once(&mut mem(), 0x00c9_628c, 0x00c9_4f00, 0x004f_ee70, 0x00af_b710)
    }
    /// `Phyre_SpinWaitCounters18_00564630`.
    Phyre_SpinWaitCounters18_00564630 @ 0x0056_4630 => extern "thiscall" fn s_00564630(a0: u32) -> u32 {
        hm::spin_wait(&mut mem(), a0, 0x18, 0x0043_4870)
    }
    /// `Phyre_SpinWaitCounters14_00564680`.
    Phyre_SpinWaitCounters14_00564680 @ 0x0056_4680 => extern "thiscall" fn s_00564680(a0: u32) -> u32 {
        hm::spin_wait(&mut mem(), a0, 0x14, 0x0043_4910)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PBlendableAnimationSource__destructInstance`.
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PBlendableAnimationSource__destructInstance @ 0x0054_2420 => extern "thiscall" fn s_00542420(_a0: u32, a1: u32) -> u8 {
        hm::destruct_members(&mut mem(), a1, &[(0x38, 0x0052_52c0), (0x24, 0x0052_3240), (0xc, 0x0052_2600)])
    }
    /// `Phyre_PInstanceList_PInstanceList`.
    Phyre_PInstanceList_PInstanceList @ 0x0057_6270 => extern "thiscall" fn s_00576270(a0: u32, a1: u32, a2: u32) -> u32 {
        hm::instance_list_ctor(&mut mem(), a0, a1, a2)
    }
    /// `Phyre_DetachAll_00505010`.
    Phyre_DetachAll_00505010 @ 0x0050_5010 => extern "thiscall" fn s_00505010(a0: u32) {
        hm::detach_all(&mut mem(), a0);
    }
    /// `Phyre_ConstructWithType_004f76a0`.
    Phyre_ConstructWithType_004f76a0 @ 0x004f_76a0 => extern "thiscall" fn s_004f76a0(_a0: u32, a1: u32) -> u8 {
        hm::construct_with_type(&mut mem(), a1, 0x00c9_9c80, 0x004f_27e0)
    }
    /// `Phyre_PParameterBuffer_setValueDesc_0056e600`.
    Phyre_PParameterBuffer_setValueDesc_0056e600 @ 0x0056_e600 => extern "thiscall" fn s_0056e600(a0: u32, a1: u32, a2: u32) -> u32 {
        let mut fr = super::StackFrame::<4>::new();
        hm::set_value_local(&mut mem(), a0, a1, a2, fr.at())
    }
    /// `Phyre_PParameterBuffer_setTextureChecked_0056e670`.
    Phyre_PParameterBuffer_setTextureChecked_0056e670 @ 0x0056_e670 => extern "thiscall" fn s_0056e670(a0: u32, a1: u32, a2: u32) -> u32 {
        hm::set_texture_checked(&mut mem(), a0, a1, a2)
    }
    /// `Phyre_StaticCriticalSection_initialize_004ff020`.
    Phyre_StaticCriticalSection_initialize_004ff020 @ 0x004f_f020 => extern "C" fn s_004ff020() -> u32 {
        hm::static_once_then(&mut mem(), 0x00c9_4efc, 0x00c9_4ee4, 0x0043_15e0, 0x00af_b700, 0x0043_17f0)
    }
    /// `Phyre_PRedBlackTree_addKey_00595ee0`.
    Phyre_PRedBlackTree_addKey_00595ee0 @ 0x0059_5ee0 => extern "thiscall" fn s_00595ee0(a0: u32, a1: u32) -> u32 {
        hm::tree_add(&mut mem(), a0, a1)
    }
    /// `Phyre_ZeroInit16_004e9f70`.
    Phyre_ZeroInit16_004e9f70 @ 0x004e_9f70 => extern "thiscall" fn s_004e9f70(a0: u32) -> u32 {
        x::zero_init(&mut mem(), a0, 16, None)
    }
    /// `Phyre_ConstructArray3c_004fd960`.
    Phyre_ConstructArray3c_004fd960 @ 0x004f_d960 => extern "C" fn s_004fd960(a0: u32, a1: u32) {
        hm::for_each(&mut mem(), a0, a1, 0x3c, 0x0058_3240);
    }
    /// `PWorldCursor_skipEmpty_005009f0`.
    PWorldCursor_skipEmpty_005009f0 @ 0x0050_09f0 => extern "fastcall" fn s_005009f0(a0: u32) {
        hm::skip_empty(&mut mem(), a0);
    }
    /// `PRendererD3D11_mapSamplerFilter_0058aa10`.
    PRendererD3D11_mapSamplerFilter_0058aa10 @ 0x0058_aa10 => extern "C" fn s_0058aa10(a0: u32, a1: u32, a2: u32) -> u32 {
        hm::filter(&mut mem(), a0, a1, a2)
    }
    /// `Phyre_PCluster_allocateMeshInstance_00566680`.
    Phyre_PCluster_allocateMeshInstance_00566680 @ 0x0056_6680 => extern "thiscall" fn s_00566680(a0: u32, a1: u32) -> u32 {
        hm::instance_list_call(&mut mem(), a0, a1)
    }
    /// `Phyre_Ctor_00583320`.
    Phyre_Ctor_00583320 @ 0x0058_3320 => extern "thiscall" fn s_00583320(a0: u32) -> u32 {
        hm::ctor_583320(&mut mem(), a0)
    }
    /// `Phyre_SetColor2a8_0059a200`.
    Phyre_SetColor2a8_0059a200 @ 0x0059_a200 => extern "thiscall" fn s_0059a200(a0: u32, a1: u32) -> u32 {
        let mut fr = super::StackFrame::<4>::new();
        hm::set_color(&mut mem(), a0, a1, fr.at())
    }
    /// `PShaderProgramD3D11_release_0059c9c0`.
    PShaderProgramD3D11_release_0059c9c0 @ 0x0059_c9c0 => extern "thiscall" fn s_0059c9c0(a0: u32) {
        hm::release_program(&mut mem(), a0);
    }
    /// `Phyre_PArrayU32_resize_004f8880`.
    Phyre_PArrayU32_resize_004f8880 @ 0x004f_8880 => extern "thiscall" fn s_004f8880(a0: u32, a1: u32) -> u32 {
        hm::resize_words(&mut mem(), a0, a1)
    }
    /// `PWorldCursor_MeshInstanceBounds_004ff6e0`.
    PWorldCursor_MeshInstanceBounds_004ff6e0 @ 0x004f_f6e0 => extern "thiscall" fn s_004ff6e0(a0: u32, a1: u32) -> u32 {
        hm::bounds_cursor(&mut mem(), a0, a1)
    }
    /// `Phyre_PRendering_PTexture2DD3D11_PTexture2DD3D11`.
    Phyre_PRendering_PTexture2DD3D11_PTexture2DD3D11 @ 0x0058_4240 => extern "thiscall" fn s_00584240(a0: u32) -> u32 {
        hm::texture2d_ctor(&mut mem(), a0)
    }
    /// `Phyre_PRendering_PMeshInstance_computeJointMatrices`.
    Phyre_PRendering_PMeshInstance_computeJointMatrices @ 0x004f_6ef0 => extern "thiscall" fn s_004f6ef0(a0: u32, a1: u32) -> u32 {
        let mut fr = super::StackFrame::<32>::new();
        hm::compute_joint_matrices(&mut mem(), a0, a1, fr.at())
    }
    /// `Phyre_PRedBlackTree_insertNode_005817d0`.
    Phyre_PRedBlackTree_insertNode_005817d0 @ 0x0058_17d0 => extern "thiscall" fn s_005817d0(a0: u32, a1: u32) -> u32 {
        hm::tree_insert(&mut mem(), a0, a1)
    }
    /// `Phyre_Ctor_0055dd30`.
    Phyre_Ctor_0055dd30 @ 0x0055_dd30 => extern "thiscall" fn s_0055dd30(a0: u32) -> u32 {
        hm::ctor_55dd30(&mut mem(), a0)
    }
    /// `Phyre_GetNamedSemanticDescriptor_004d9950`.
    Phyre_GetNamedSemanticDescriptor_004d9950 @ 0x004d_9950 => extern "C" fn s_004d9950() -> u32 {
        hm::static_once_args(&mut mem(), 0x00ca_5728, 0x00ca_56f4, 0x005a_0200, &NAMED_SEMANTIC_ARGS, 0x00b2_062c, 0x00af_ed30)
    }
    /// `Phyre_GetNamedSemanticDescriptor_004d9b80`.
    Phyre_GetNamedSemanticDescriptor_004d9b80 @ 0x004d_9b80 => extern "C" fn s_004d9b80() -> u32 {
        hm::static_once_args(&mut mem(), 0x00ca_5728, 0x00ca_56f4, 0x005a_0200, &NAMED_SEMANTIC_ARGS, 0x00b2_062c, 0x00af_ed30)
    }
    /// `Phyre_WaitAndClear_0055e060`.
    Phyre_WaitAndClear_0055e060 @ 0x0055_e060 => extern "thiscall" fn s_0055e060(a0: u32) {
        h2::wait_and_clear(&mut mem(), a0);
    }
    /// `PBufferD3D11_dtor_00584ea0`.
    PBufferD3D11_dtor_00584ea0 @ 0x0058_4ea0 => extern "thiscall" fn s_00584ea0(a0: u32) -> u32 {
        h2::release_buffer(&mut mem(), a0)
    }
    /// `Phyre_PInstanceList_freeBuffer`.
    Phyre_PInstanceList_freeBuffer @ 0x0057_64a0 => extern "thiscall" fn s_005764a0(a0: u32) {
        let mut fr = super::StackFrame::<7>::new();
        h2::free_buffer(&mut mem(), a0, fr.at());
    }
    /// `Phyre_PDynamicMeshInstanceSub_scalarDtor_004fb770`.
    Phyre_PDynamicMeshInstanceSub_scalarDtor_004fb770 @ 0x004f_b770 => extern "thiscall" fn s_004fb770(a0: u32, a1: u32) -> u32 {
        h2::dynamic_dtor(&mut mem(), a0, a1)
    }
    /// `Phyre_PDynamicMeshInstanceSub_constructInstance_004fc630`.
    Phyre_PDynamicMeshInstanceSub_constructInstance_004fc630 @ 0x004f_c630 => extern "thiscall" fn s_004fc630(_a0: u32, a1: u32) -> u8 {
        h2::construct_dynamic(&mut mem(), a1)
    }
    /// `Phyre_FreePairs_004e4df0`.
    Phyre_FreePairs_004e4df0 @ 0x004e_4df0 => extern "C" fn s_004e4df0(a0: u32, a1: u32) {
        h2::free_pairs(&mut mem(), a0, a1);
    }
    /// `Phyre_Vec3Max_004f84f0`.
    Phyre_Vec3Max_004f84f0 @ 0x004f_84f0 => extern "C" fn s_004f84f0(a0: u32, a1: u32, a2: u32) -> u32 {
        h2::vec3_select(&mut mem(), a0, a1, a2, true)
    }
    /// `Phyre_Vec3Min_004f8580`.
    Phyre_Vec3Min_004f8580 @ 0x004f_8580 => extern "C" fn s_004f8580(a0: u32, a1: u32, a2: u32) -> u32 {
        h2::vec3_select(&mut mem(), a0, a1, a2, false)
    }
    /// `Phyre_Ctor_00583240`.
    Phyre_Ctor_00583240 @ 0x0058_3240 => extern "thiscall" fn s_00583240(a0: u32) -> u32 {
        h2::ctor_block(&mut mem(), a0, 0x0049_f990, 0xc, 0x10, 4, &[0x38, 0x20, 0x24, 0x28, 0x2c, 0x30, 0x34], None)
    }
    /// `PBufferD3D11_destructInstance_00592780`.
    PBufferD3D11_destructInstance_00592780 @ 0x0059_2780 => extern "thiscall" fn s_00592780(_a0: u32, a1: u32) -> u8 {
        h2::destruct_buffer(&mut mem(), a1)
    }
    /// `Phyre_PClassDescriptor_PTexture2DD3D11_constructInstance_0058e820`.
    Phyre_PClassDescriptor_PTexture2DD3D11_constructInstance_0058e820 @ 0x0058_e820 => extern "thiscall" fn s_0058e820(_a0: u32, a1: u32) -> u8 {
        h2::construct_texture2d(&mut mem(), a1)
    }
    /// `Phyre_Ctor_00584400`.
    Phyre_Ctor_00584400 @ 0x0058_4400 => extern "thiscall" fn s_00584400(a0: u32) -> u32 {
        h2::ctor_block(&mut mem(), a0, 0x004c_6070, 0x20, 0x24, 5, &[0x64], Some((0x38, 0x2c)))
    }
    /// `Phyre_Ctor_005841a0`.
    Phyre_Ctor_005841a0 @ 0x0058_41a0 => extern "thiscall" fn s_005841a0(a0: u32) -> u32 {
        h2::ctor_block(&mut mem(), a0, 0x0050_1c40, 0x14, 0x18, 5, &[0x44, 0x2c, 0x30, 0x34, 0x38, 0x3c, 0x40], None)
    }
    /// `Phyre_Ctor_005842c0`.
    Phyre_Ctor_005842c0 @ 0x0058_42c0 => extern "thiscall" fn s_005842c0(a0: u32) -> u32 {
        h2::ctor_block(&mut mem(), a0, 0x004c_47a0, 0x28, 0x2c, 5, &[0x64, 0x40, 0x44, 0x48, 0x4c, 0x50, 0x54, 0x58, 0x5c, 0x60], None)
    }
    /// `PShaderProgramD3D11_dtor_005856b0`.
    PShaderProgramD3D11_dtor_005856b0 @ 0x0058_56b0 => extern "thiscall" fn s_005856b0(a0: u32) -> u32 {
        h2::program_dtor(&mut mem(), a0)
    }
    /// `Phyre_FreeTwoArrays_00585610`.
    Phyre_FreeTwoArrays_00585610 @ 0x0058_5610 => extern "thiscall" fn s_00585610(a0: u32) {
        h2::free_two_arrays(&mut mem(), a0);
    }
    /// `Phyre_PRendering_PDynamicMeshInstance_Bind`.
    Phyre_PRendering_PDynamicMeshInstance_Bind @ 0x004f_a540 => extern "C" fn s_004fa540() -> u32 {
        h2::dynamic_mesh_bind(&mut mem())
    }
    /// `PRendererD3D11_setViewport_0058c790`.
    PRendererD3D11_setViewport_0058c790 @ 0x0058_c790 => extern "thiscall" fn s_0058c790(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32) -> u32 {
        let mut fr = super::StackFrame::<7>::new();
        h2::set_viewport_rect(&mut mem(), a0, [a1, a2, a3, a4], fr.at())
    }
    /// `PRendererD3D11_setTargetClearColor_00599f30`.
    PRendererD3D11_setTargetClearColor_00599f30 @ 0x0059_9f30 => extern "thiscall" fn s_00599f30(a0: u32, a1: u32, a2: u32) -> u32 {
        let mut fr = super::StackFrame::<4>::new();
        h2::set_target_color(&mut mem(), a0, a1, a2, fr.at())
    }
    /// `Phyre_NewConstantBuffer_004f49d0`.
    Phyre_NewConstantBuffer_004f49d0 @ 0x004f_49d0 => extern "C" fn s_004f49d0() -> u32 {
        h2::new_constant_buffer(&mut mem())
    }
    /// `Phyre_FreeTagged4_004dbaf0`.
    Phyre_FreeTagged4_004dbaf0 @ 0x004d_baf0 => extern "thiscall" fn s_004dbaf0(a0: u32) {
        h3::free_tagged4(&mut mem(), a0);
    }
    /// `Phyre_Dtor_00567620`.
    Phyre_Dtor_00567620 @ 0x0056_7620 => extern "thiscall" fn s_00567620(a0: u32) {
        h3::free_567620(&mut mem(), a0);
    }
    /// `PShaderD3D11_dtor_00585180`.
    PShaderD3D11_dtor_00585180 @ 0x0058_5180 => extern "thiscall" fn s_00585180(a0: u32) -> u32 {
        h3::dtor_585180(&mut mem(), a0)
    }
    /// `Phyre_PGameplay_Component_ctor_005572e0`.
    Phyre_PGameplay_Component_ctor_005572e0 @ 0x0055_72e0 => extern "thiscall" fn s_005572e0(a0: u32) -> u32 {
        h3::component_ctor(&mut mem(), a0)
    }
    /// `Phyre_PGameplay_Component_dtor_00557410`.
    Phyre_PGameplay_Component_dtor_00557410 @ 0x0055_7410 => extern "thiscall" fn s_00557410(a0: u32) -> u32 {
        h3::component_dtor(&mut mem(), a0)
    }
    /// `Phyre_Ctor_004fee80`.
    Phyre_Ctor_004fee80 @ 0x004f_ee80 => extern "thiscall" fn s_004fee80(a0: u32) -> u32 {
        h3::ctor_4fee80(&mut mem(), a0)
    }
    /// `PTextureD3D11_register_0058cf80`.
    PTextureD3D11_register_0058cf80 @ 0x0058_cf80 => extern "thiscall" fn s_0058cf80(a0: u32) -> u32 {
        let mut fr = super::StackFrame::<2>::new();
        h3::register_texture(&mut mem(), a0, fr.at())
    }
    /// `PRendererD3D11_drawIndexed_00592c60`.
    PRendererD3D11_drawIndexed_00592c60 @ 0x0059_2c60 => extern "thiscall" fn s_00592c60(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32) -> u8 {
        h3::draw_indexed(&mut mem(), a0, a1, a2, a3, a4)
    }
    /// `Phyre_PInstanceList_freeObject`.
    Phyre_PInstanceList_freeObject @ 0x0057_6530 => extern "thiscall" fn s_00576530(a0: u32, a1: u32) -> u32 {
        h3::free_object(&mut mem(), a0, a1)
    }
    /// `Phyre_Ctor_0055dc40`.
    Phyre_Ctor_0055dc40 @ 0x0055_dc40 => extern "thiscall" fn s_0055dc40(a0: u32) -> u32 {
        h3::ctor_55dc40(&mut mem(), a0)
    }
    /// `Phyre_PRendering_PMeshInstanceSegmentContext_Bind`.
    Phyre_PRendering_PMeshInstanceSegmentContext_Bind @ 0x004f_4700 => extern "C" fn s_004f4700() -> u32 {
        hb::bind(&mut mem(), &ffx_prender::h2_bind::B_004F4700)
    }
    /// `Phyre_PRendering_PModifierNetworkDynamicMeshInstance_Bind`.
    Phyre_PRendering_PModifierNetworkDynamicMeshInstance_Bind @ 0x004f_b8c0 => extern "C" fn s_004fb8c0() -> u32 {
        hb::bind(&mut mem(), &ffx_prender::h2_bind::B_004FB8C0)
    }
    /// `Phyre_PRendering_PMeshInstanceAttachPoint_Bind`.
    Phyre_PRendering_PMeshInstanceAttachPoint_Bind @ 0x0050_1110 => extern "C" fn s_00501110() -> u32 {
        hb::bind(&mut mem(), &ffx_prender::h2_bind::B_00501110)
    }
    /// `Phyre_PSprite_PTextureAtlasInfo_Bind`.
    Phyre_PSprite_PTextureAtlasInfo_Bind @ 0x0056_a390 => extern "C" fn s_0056a390() -> u32 {
        hb::bind(&mut mem(), &ffx_prender::h2_bind::B_0056A390)
    }
    /// `Phyre_PGeometry_PMeshSegmentNVN_Bind`.
    Phyre_PGeometry_PMeshSegmentNVN_Bind @ 0x0055_eca0 => extern "C" fn s_0055eca0() -> u32 {
        hb::bind(&mut mem(), &ffx_prender::h2_bind::B_0055ECA0)
    }
    /// `Phyre_PRendering_PMeshInstanceSegmentStreamBinding_Bind`.
    Phyre_PRendering_PMeshInstanceSegmentStreamBinding_Bind @ 0x004f_4800 => extern "C" fn s_004f4800() -> u32 {
        hb::bind(&mut mem(), &ffx_prender::h2_bind::B_004F4800)
    }
    /// `Phyre_PRendering_PContextVariantFoldingTable_Bind`.
    Phyre_PRendering_PContextVariantFoldingTable_Bind @ 0x004d_c870 => extern "C" fn s_004dc870() -> u32 {
        hb::bind(&mut mem(), &ffx_prender::h2_bind::B_004DC870)
    }
    /// `Phyre_PSprite_PSpriteAnimationInfoInstance_Bind`.
    Phyre_PSprite_PSpriteAnimationInfoInstance_Bind @ 0x0056_f990 => extern "C" fn s_0056f990() -> u32 {
        hb::bind(&mut mem(), &ffx_prender::h2_bind::B_0056F990)
    }
    /// `Phyre_PAnimation_PBlendableAnimationSource_Bind`.
    Phyre_PAnimation_PBlendableAnimationSource_Bind @ 0x0054_0750 => extern "C" fn s_00540750() -> u32 {
        hb::bind(&mut mem(), &ffx_prender::h2_bind::B_00540750)
    }
    /// `Phyre_PRendering_PShaderPassInfo_Bind`.
    Phyre_PRendering_PShaderPassInfo_Bind @ 0x004d_cdd0 => extern "C" fn s_004dcdd0() -> u32 {
        hb::bind(&mut mem(), &ffx_prender::h2_bind::B_004DCDD0)
    }
    /// `Phyre_PSprite_PSpriteAttributes_Bind`.
    Phyre_PSprite_PSpriteAttributes_Bind @ 0x0056_8dc0 => extern "C" fn s_00568dc0() -> u32 {
        hb::bind(&mut mem(), &ffx_prender::h2_bind::B_00568DC0)
    }
    /// `Phyre_PRendering_PSceneRenderPass_Bind`.
    Phyre_PRendering_PSceneRenderPass_Bind @ 0x004d_ca40 => extern "C" fn s_004dca40() -> u32 {
        hb::bind(&mut mem(), &ffx_prender::h2_bind::B_004DCA40)
    }
    /// `Phyre_PRendering_PEffectVariant_Bind`.
    Phyre_PRendering_PEffectVariant_Bind @ 0x004e_4270 => extern "C" fn s_004e4270() -> u32 {
        hb::bind(&mut mem(), &ffx_prender::h2_bind::B_004E4270)
    }
    /// `Phyre_PRendering_PShaderPassNVN_Bind`.
    Phyre_PRendering_PShaderPassNVN_Bind @ 0x0058_7300 => extern "C" fn s_00587300() -> u32 {
        hb::bind(&mut mem(), &ffx_prender::h2_bind::B_00587300)
    }
    /// `Phyre_PRendering_PMeshInstanceBounds_Bind`.
    Phyre_PRendering_PMeshInstanceBounds_Bind @ 0x004f_fa30 => extern "C" fn s_004ffa30() -> u32 {
        hb::bind(&mut mem(), &ffx_prender::h2_bind::B_004FFA30)
    }
    /// `Phyre_WaitAndClear_0055dfa0`.
    Phyre_WaitAndClear_0055dfa0 @ 0x0055_dfa0 => extern "thiscall" fn s_0055dfa0(a0: u32) {
        h4::wait_and_clear2(&mut mem(), a0);
    }
    /// `PRenderStateBlock_ctor_00583e20`.
    PRenderStateBlock_ctor_00583e20 @ 0x0058_3e20 => extern "thiscall" fn s_00583e20(a0: u32) -> u32 {
        h4::state_block_ctor(&mut mem(), a0)
    }
    /// `PConstantBufferD3D11_resizeSlots_00599a80`.
    PConstantBufferD3D11_resizeSlots_00599a80 @ 0x0059_9a80 => extern "thiscall" fn s_00599a80(a0: u32, a1: u32) -> u32 {
        h4::resize_slots(&mut mem(), a0, a1)
    }
    /// `Phyre_DeleteArray4c_00540b80`.
    Phyre_DeleteArray4c_00540b80 @ 0x0054_0b80 => extern "C" fn s_00540b80(a0: u32, a1: u32) {
        h5::delete_array(&mut mem(), a0, a1, 0x4c, &[(0x38, 0x0052_52c0), (0x24, 0x0052_3240), (0xc, 0x0052_2600)]);
    }
    /// `Phyre_DeleteStrings4_004dd770`.
    Phyre_DeleteStrings4_004dd770 @ 0x004d_d770 => extern "C" fn s_004dd770(a0: u32, a1: u32) {
        h5::delete_strings4(&mut mem(), a0, a1);
    }
    /// `Phyre_DestructArrayName_0056cab0`.
    Phyre_DestructArrayName_0056cab0 @ 0x0056_cab0 => extern "thiscall" fn s_0056cab0(_a0: u32, a1: u32) -> u8 {
        h5::destruct_array_name(&mut mem(), a1)
    }
    /// `Phyre_DestructInstance_00562410`.
    Phyre_DestructInstance_00562410 @ 0x0056_2410 => extern "thiscall" fn s_00562410(_a0: u32, a1: u32) -> u8 {
        h5::destruct_562410(&mut mem(), a1)
    }
    /// `PTexture2DD3D11_setTexture_0058fb90`.
    PTexture2DD3D11_setTexture_0058fb90 @ 0x0058_fb90 => extern "thiscall" fn s_0058fb90(a0: u32, a1: u32, a2: u32, a3: u32, _a4: u32) -> u32 {
        h5::set_texture(&mut mem(), a0, a1, a2, a3)
    }
    /// `PMeshInstanceBounds_link_004ff800`.
    PMeshInstanceBounds_link_004ff800 @ 0x004f_f800 => extern "thiscall" fn s_004ff800(a0: u32, a1: u32) -> u32 {
        h5::link_bounds(&mut mem(), a0, a1)
    }
    /// `PConstantBufferRing_next_004f8460`.
    PConstantBufferRing_next_004f8460 @ 0x004f_8460 => extern "thiscall" fn s_004f8460(a0: u32, a1: u32) -> u32 {
        h5::next_ring_slot(&mut mem(), a0, a1)
    }
    /// `PConstantBufferRing_entry_00596020`.
    PConstantBufferRing_entry_00596020 @ 0x0059_6020 => extern "thiscall" fn s_00596020(a0: u32, a1: u32, a2: u32) -> u32 {
        h5::ring_entry(&mut mem(), a0, a1, a2)
    }
    /// `Phyre_PRendering_PEffectVariant_createMaterialWithPreallocatedBuffer`.
    Phyre_PRendering_PEffectVariant_createMaterialWithPreallocatedBuffer @ 0x004e_6b40 => extern "thiscall" fn s_004e6b40(a0: u32, a1: u32, a2: u32) -> u32 {
        h5::create_material_prealloc(&mut mem(), a0, a1, a2)
    }
    /// `Phyre_PRendering_PEffectVariant_createMaterial`.
    Phyre_PRendering_PEffectVariant_createMaterial @ 0x004e_6a40 => extern "thiscall" fn s_004e6a40(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        h5::create_material(&mut mem(), a0, a1, a2, a3)
    }
    /// `PConstantBufferD3D11_upload_00593930`.
    PConstantBufferD3D11_upload_00593930 @ 0x0059_3930 => extern "thiscall" fn s_00593930(a0: u32, a1: u32, a2: u32) -> u32 {
        let mut fr = super::StackFrame::<3>::new();
        h5::upload_constants(&mut mem(), a0, a1, a2, fr.at())
    }
    /// `PRenderPassType_findOrAdd_004dd910`.
    PRenderPassType_findOrAdd_004dd910 @ 0x004d_d910 => extern "C" fn s_004dd910(a0: u32) -> u32 {
        h5::find_or_add_pass_type(&mut mem(), a0)
    }
    /// `PSamplerStateD3D11_release_0059c770`.
    PSamplerStateD3D11_release_0059c770 @ 0x0059_c770 => extern "thiscall" fn s_0059c770(a0: u32) -> u32 {
        let mut fr = super::StackFrame::<2>::new();
        h5::release_sampler(&mut mem(), a0, fr.at())
    }
    /// `ShaderParams_FindByName`.
    ShaderParams_FindByName @ 0x0056_cbf0 => extern "thiscall" fn s_0056cbf0(a0: u32, a1: u32) -> u32 {
        h5::find_param(&mut mem(), a0, a1)
    }
    /// `PRendererD3D11_resolveTargets_00593010`.
    PRendererD3D11_resolveTargets_00593010 @ 0x0059_3010 => extern "thiscall" fn s_00593010(a0: u32) -> u8 {
        h5::resolve_targets(&mut mem(), a0)
    }
    /// `PShaderD3D11_bindPixel_005932b0`.
    PShaderD3D11_bindPixel_005932b0 @ 0x0059_32b0 => extern "thiscall" fn s_005932b0(a0: u32, a1: u32, _a2: u32) -> u8 {
        h5::bind_pixel_shader(&mut mem(), a0, a1)
    }
    /// `PShaderD3D11_bindVertex_00593510`.
    PShaderD3D11_bindVertex_00593510 @ 0x0059_3510 => extern "thiscall" fn s_00593510(a0: u32, a1: u32, _a2: u32) -> u8 {
        let mut fr = super::StackFrame::<8>::new();
        h5::bind_vertex_shader(&mut mem(), a0, a1, fr.at())
    }
    /// `PRenderStateBlock_defaults_00582f50`.
    PRenderStateBlock_defaults_00582f50 @ 0x0058_2f50 => extern "thiscall" fn s_00582f50(a0: u32, _a1: u32) -> u32 {
        let mut fr = super::StackFrame::<8>::new();
        h5::default_blocks(&mut mem(), a0, fr.at())
    }
    /// `Renderer_SetAnisotropy`.
    Renderer_SetAnisotropy @ 0x0058_de30 => extern "C" fn s_0058de30(a0: u32) -> u32 {
        h5::set_anisotropy(&mut mem(), a0)
    }
    /// `PSharray16_resize_00599460`.
    PSharray16_resize_00599460 @ 0x0059_9460 => extern "thiscall" fn s_00599460(a0: u32, a1: u32) -> u32 {
        h5::resize16(&mut mem(), a0, a1)
    }
    /// `Phyre_Destroy_004db9d0`.
    Phyre_Destroy_004db9d0 @ 0x004d_b9d0 => extern "thiscall" fn s_004db9d0(a0: u32) -> u32 {
        h5::destroy_4db9d0(&mut mem(), a0)
    }
    /// `PConstantBufferD3D11_unmapEntry_00565000`.
    PConstantBufferD3D11_unmapEntry_00565000 @ 0x0056_5000 => extern "thiscall" fn s_00565000(a0: u32, a1: u32, _a2: u32) -> u32 {
        let mut fr = super::StackFrame::<1>::new();
        h5::unmap_entry(&mut mem(), a0, a1, fr.at())
    }
    /// `PRenderThread_mapResource_00560e90`.
    PRenderThread_mapResource_00560e90 @ 0x0056_0e90 => extern "C" fn s_00560e90(a0: u32, a1: u32, a2: u32) -> u32 {
        let mut fr = super::StackFrame::<10>::new();
        h5::map_resource(&mut mem(), a0, a1, a2, fr.at())
    }
    /// `PRenderThread_mapResource_00560f90`.
    PRenderThread_mapResource_00560f90 @ 0x0056_0f90 => extern "C" fn s_00560f90(a0: u32, a1: u32, a2: u32) -> u32 {
        let mut fr = super::StackFrame::<10>::new();
        h5::map_resource(&mut mem(), a0, a1, a2, fr.at())
    }
    /// `PShaderProgramD3D11_set_0059ac40`.
    PShaderProgramD3D11_set_0059ac40 @ 0x0059_ac40 => extern "thiscall" fn s_0059ac40(a0: u32, a1: u32) -> u32 {
        h5::set_program(&mut mem(), a0, a1)
    }
    /// `PShaderProgramD3D11_bindTables_0059ab50`.
    PShaderProgramD3D11_bindTables_0059ab50 @ 0x0059_ab50 => extern "stdcall" fn s_0059ab50(a0: u32, a1: u32, a2: u32) -> u32 {
        h5::bind_tables(&mut mem(), a0, a1, a2, 0x0059_a650, false, &H5_TABLES_A)
    }
    /// `PShaderProgramD3D11_bindTextures_0059b010`.
    PShaderProgramD3D11_bindTextures_0059b010 @ 0x0059_b010 => extern "stdcall" fn s_0059b010(a0: u32, a1: u32, a2: u32) -> u32 {
        h5::bind_tables(&mut mem(), a0, a1, a2, 0x0059_b3d0, true, &H5_TABLES_B)
    }
    /// `Phyre_Ctor_00567110`.
    Phyre_Ctor_00567110 @ 0x0056_7110 => extern "thiscall" fn s_00567110(a0: u32) -> u32 {
        h5::ctor_567110(&mut mem(), a0)
    }
    /// `PConstantBufferD3D11_releaseEntries_0059c540`.
    PConstantBufferD3D11_releaseEntries_0059c540 @ 0x0059_c540 => extern "thiscall" fn s_0059c540(a0: u32) -> u32 {
        h5::release_entries(&mut mem(), a0)
    }
    /// `Phyre_Destroy_004e39b0`.
    Phyre_Destroy_004e39b0 @ 0x004e_39b0 => extern "thiscall" fn s_004e39b0(a0: u32) {
        h5::destroy_4e39b0(&mut mem(), a0);
    }
    /// `PRendererD3D11_createBuffers_00595620`.
    PRendererD3D11_createBuffers_00595620 @ 0x0059_5620 => extern "thiscall" fn s_00595620(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32) -> u32 {
        let mut fr = super::StackFrame::<12>::new();
        h5::create_buffers(&mut mem(), a0, a1, a2, a3, a4, a5, a6, fr.at())
    }
    /// `PRenderTargetD3D11_releaseViews_0059c650`.
    PRenderTargetD3D11_releaseViews_0059c650 @ 0x0059_c650 => extern "thiscall" fn s_0059c650(a0: u32) -> u32 {
        h5::release_views2(&mut mem(), a0)
    }
}

/// The `PNamedSemanticDescriptor` constructor's words in
/// `0x004d9950`/`0x004d9b80` (`true`: image addresses).
const NAMED_SEMANTIC_ARGS: [(u32, bool); 5] = [
    (0x00b2_0350, true),
    (0x10, false),
    (4, false),
    (0x004d_ec30, true),
    (0x004d_f9c0, true),
];

/// The constant-table layouts of `0x0059ab50` (first, count, base).
const H5_TABLES_A: [(u32, u32, u32); 4] = [
    (0x194, 0x19c, 0x1a8),
    (0x1ac, 0x1b4, 0x1c0),
    (0x1c4, 0x1cc, 0x1d8),
    (0x1dc, 0x1e4, 0x1f0),
];
/// The same of `0x0059b010`.
const H5_TABLES_B: [(u32, u32, u32); 4] = [
    (0x1f4, 0x1fc, 0x208),
    (0x20c, 0x214, 0x220),
    (0x224, 0x22c, 0x238),
    (0x23c, 0x244, 0x250),
];
