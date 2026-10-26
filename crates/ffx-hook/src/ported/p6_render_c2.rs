//! M4 port, list p6-render-c, continuation round (batch 8 on): the
//! shims. Ports in `ffx_prender::p6_more` and the shapes other lists
//! ported (`ffx_prender::declare`, `ffx_prender::h2_more`).

use super::{mem, ported};
use ffx_prender::{
    declare, h1_bind, h2_bind, h2_more, p6_binds as binds, p6_binds1 as binds1, p6_more as more,
};

ported! {
    /// `PClassDescriptorAbstract<PArray<T>>`'s constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_unsigned_char_const_4__ctor @ 0x005a_9db0 => extern "thiscall" fn s_005a9db0(this: u32) -> u32 {
        declare::bridge::abstract_ctor(&mut mem(), this, &const { more::abstract_of(0x005a_9db0) })
    }
    /// `PClassDescriptorAbstract<PArray<T>>`'s constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment_4__ctor @ 0x005a_9e50 => extern "thiscall" fn s_005a9e50(this: u32) -> u32 {
        declare::bridge::abstract_ctor(&mut mem(), this, &const { more::abstract_of(0x005a_9e50) })
    }
    /// `PClassDescriptorAbstract<PArray<T>>`'s constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PDynamicGeometry_PRenderStream_4__ctor @ 0x005a_9ef0 => extern "thiscall" fn s_005a9ef0(this: u32) -> u32 {
        declare::bridge::abstract_ctor(&mut mem(), this, &const { more::abstract_of(0x005a_9ef0) })
    }
    /// `PClassDescriptorAbstract<PArray<T>>`'s constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PBase_const_4__ctor @ 0x005b_0910 => extern "thiscall" fn s_005b0910(this: u32) -> u32 {
        declare::bridge::abstract_ctor(&mut mem(), this, &const { more::abstract_of(0x005b_0910) })
    }
    /// `PClassDescriptorAbstract<PArray<T>>`'s constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PDynamicGeometry_PModifierAndInputs_4__ctor @ 0x005b_e710 => extern "thiscall" fn s_005be710(this: u32) -> u32 {
        declare::bridge::abstract_ctor(&mut mem(), this, &const { more::abstract_of(0x005b_e710) })
    }
    /// `PClassDescriptorAbstract<PArray<T>>`'s constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkBuffer_4__ctor @ 0x005b_e7b0 => extern "thiscall" fn s_005be7b0(this: u32) -> u32 {
        declare::bridge::abstract_ctor(&mut mem(), this, &const { more::abstract_of(0x005b_e7b0) })
    }
    /// `PClassDescriptorAbstract<PArray<T>>`'s constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PDynamicGeometry_PRenderStreamInput_4__ctor @ 0x005b_e850 => extern "thiscall" fn s_005be850(this: u32) -> u32 {
        declare::bridge::abstract_ctor(&mut mem(), this, &const { more::abstract_of(0x005b_e850) })
    }
    /// `PClassDescriptorAbstract<PArray<T>>`'s constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PPostProcessing_PPostEffectBase_4__ctor @ 0x009f_96d0 => extern "thiscall" fn s_009f96d0(this: u32) -> u32 {
        declare::bridge::abstract_ctor(&mut mem(), this, &const { more::abstract_of(0x009f_96d0) })
    }
    /// `PClassDescriptorAbstract<PArray<T>>`'s constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PText_PBitmapFontCharInfo_4__ctor @ 0x00a2_b780 => extern "thiscall" fn s_00a2b780(this: u32) -> u32 {
        declare::bridge::abstract_ctor(&mut mem(), this, &const { more::abstract_of(0x00a2_b780) })
    }
    /// `DeclareClassDescriptor(name)` -> the static descriptor.
    Phyre_PArray_unsigned_char_const_4__DeclareClassDescriptor @ 0x005a_b580 => extern "C" fn s_005ab580(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { more::declare_of(0x005a_b580) })
    }
    /// `DeclareClassDescriptor(name)` -> the static descriptor.
    Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment_4__DeclareClassDescriptor @ 0x005a_b790 => extern "C" fn s_005ab790(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { more::declare_of(0x005a_b790) })
    }
    /// `DeclareClassDescriptor(name)` -> the static descriptor.
    Phyre_PArray_Phyre_PDynamicGeometry_PRenderStream_4__DeclareClassDescriptor @ 0x005a_b9a0 => extern "C" fn s_005ab9a0(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { more::declare_of(0x005a_b9a0) })
    }
    /// `DeclareClassDescriptor(name)` -> the static descriptor.
    Phyre_PArray_Phyre_PBase_const_4__DeclareClassDescriptor @ 0x005b_17c0 => extern "C" fn s_005b17c0(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { more::declare_of(0x005b_17c0) })
    }
    /// `DeclareClassDescriptor(name)` -> the static descriptor.
    Phyre_PArray_Phyre_PDynamicGeometry_PModifierAndInputs_4__DeclareClassDescriptor @ 0x005c_18c0 => extern "C" fn s_005c18c0(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { more::declare_of(0x005c_18c0) })
    }
    /// `DeclareClassDescriptor(name)` -> the static descriptor.
    Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkBuffer_4__DeclareClassDescriptor @ 0x005c_1ad0 => extern "C" fn s_005c1ad0(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { more::declare_of(0x005c_1ad0) })
    }
    /// `DeclareClassDescriptor(name)` -> the static descriptor.
    Phyre_PArray_Phyre_PDynamicGeometry_PRenderStreamInput_4__DeclareClassDescriptor @ 0x005c_1ce0 => extern "C" fn s_005c1ce0(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { more::declare_of(0x005c_1ce0) })
    }
    /// `DeclareClassDescriptor(name)` -> the static descriptor.
    Phyre_PArray_Phyre_PPostProcessing_PPostEffectBase_4__DeclareClassDescriptor @ 0x00a0_74c0 => extern "C" fn s_00a074c0(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { more::declare_of(0x00a0_74c0) })
    }
    /// `DeclareClassDescriptor(name)` -> the static descriptor.
    Phyre_PArray_Phyre_PText_PBitmapFontCharInfo_4__DeclareClassDescriptor @ 0x00a2_c6c0 => extern "C" fn s_00a2c6c0(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { more::declare_of(0x00a2_c6c0) })
    }
    /// The occluder cursor's `skipEmpty` (a copy of `0x005009f0`).
    PWorldCursor_skipEmpty_009f3930 @ 0x009f_3930 => extern "fastcall" fn s_009f3930(cursor: u32) {
        h2_more::bridge::skip_empty(&mut mem(), cursor);
    }
    /// The source's semantic check (`this`) -> the last kind read.
    Phyre_PInputs_PInputSourceJoypadButton_checkSemantics @ 0x009f_0350 => extern "thiscall" fn s_009f0350(this: u32) -> u32 {
        let (_, rule, default) = more::SEMANTIC_CHECKS[0];
        more::bridge::check_semantics(&mut mem(), this, rule, default)
    }
    /// The source's semantic check (`this`) -> the last kind read.
    Phyre_PInputs_PInputSourceKey_checkSemantics @ 0x009f_03c0 => extern "thiscall" fn s_009f03c0(this: u32) -> u32 {
        let (_, rule, default) = more::SEMANTIC_CHECKS[1];
        more::bridge::check_semantics(&mut mem(), this, rule, default)
    }
    /// The mouse-delta source's semantic check (`this`) -> the last kind read.
    Phyre_PInputs_PInputSourceMouseDeltaX_checkSemantics @ 0x009f_0470 => extern "thiscall" fn s_009f0470(this: u32) -> u32 {
        more::bridge::check_mouse_delta(&mut mem(), this)
    }
    /// The mouse-delta source's semantic check (`this`) -> the last kind read.
    Phyre_PInputs_PInputSourceMouseDeltaY_checkSemantics @ 0x009f_04f0 => extern "thiscall" fn s_009f04f0(this: u32) -> u32 {
        more::bridge::check_mouse_delta(&mut mem(), this)
    }
    /// `constructInstance(obj)` -> `al` 1.
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceJoypadButton__constructInstance @ 0x009e_b0d0 => extern "thiscall" fn s_009eb0d0(_desc: u32, obj: u32) -> u8 {
        let (_, fields, check) = more::CONSTRUCT_CHECKED[0];
        more::bridge::construct_checked(&mut mem(), obj, fields, check)
    }
    /// `constructInstance(obj)` -> `al` 1.
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceKey__constructInstance @ 0x009e_b160 => extern "thiscall" fn s_009eb160(_desc: u32, obj: u32) -> u8 {
        let (_, fields, check) = more::CONSTRUCT_CHECKED[1];
        more::bridge::construct_checked(&mut mem(), obj, fields, check)
    }
    /// `constructInstance(obj)` -> `al` 1.
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMouseDeltaX__constructInstance @ 0x009e_b500 => extern "thiscall" fn s_009eb500(_desc: u32, obj: u32) -> u8 {
        let (_, fields, check) = more::CONSTRUCT_CHECKED[2];
        more::bridge::construct_checked(&mut mem(), obj, fields, check)
    }
    /// `constructInstance(obj)` -> `al` 1.
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMouseDeltaY__constructInstance @ 0x009e_b590 => extern "thiscall" fn s_009eb590(_desc: u32, obj: u32) -> u8 {
        let (_, fields, check) = more::CONSTRUCT_CHECKED[3];
        more::bridge::construct_checked(&mut mem(), obj, fields, check)
    }
    /// `constructInstance(obj)` -> `al` 1.
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMouseButton__constructInstance @ 0x009e_b470 => extern "thiscall" fn s_009eb470(_desc: u32, obj: u32) -> u8 {
        more::bridge::construct_mouse_button(&mut mem(), obj, more::CONSTRUCT_MOUSE_BUTTON)
    }
    /// The helper's run of one flag's delegations (`this`) -> `ReleaseMutex`'s result.
    RenderDelegationHelper_runFlag200 @ 0x006a_d2d0 => extern "thiscall" fn s_006ad2d0(this: u32) -> u32 {
        more::bridge::run_delegations(&mut mem(), this, const { more::delegation_flag(0x006a_d2d0) })
    }
    /// The helper's run of one flag's delegations (`this`) -> `ReleaseMutex`'s result.
    RenderDelegationHelper_runFlag800 @ 0x006a_d380 => extern "thiscall" fn s_006ad380(this: u32) -> u32 {
        more::bridge::run_delegations(&mut mem(), this, const { more::delegation_flag(0x006a_d380) })
    }
    /// The helper's run of one flag's delegations (`this`) -> `ReleaseMutex`'s result.
    RenderDelegationHelper_runFlag4000 @ 0x006a_d430 => extern "thiscall" fn s_006ad430(this: u32) -> u32 {
        more::bridge::run_delegations(&mut mem(), this, const { more::delegation_flag(0x006a_d430) })
    }
    /// The helper's run of one flag's delegations (`this`) -> `ReleaseMutex`'s result.
    RenderDelegationHelper_runFlag4 @ 0x006a_d4e0 => extern "thiscall" fn s_006ad4e0(this: u32) -> u32 {
        more::bridge::run_delegations(&mut mem(), this, const { more::delegation_flag(0x006a_d4e0) })
    }
    /// The helper's run of one flag's delegations (`this`) -> `ReleaseMutex`'s result.
    RenderDelegationHelper_runFlag400 @ 0x006a_d600 => extern "thiscall" fn s_006ad600(this: u32) -> u32 {
        more::bridge::run_delegations(&mut mem(), this, const { more::delegation_flag(0x006a_d600) })
    }
    /// The helper's run of one flag's delegations (`this`) -> `ReleaseMutex`'s result.
    RenderDelegationHelper_beforeEffects @ 0x006a_d6b0 => extern "thiscall" fn s_006ad6b0(this: u32) -> u32 {
        more::bridge::run_delegations(&mut mem(), this, const { more::delegation_flag(0x006a_d6b0) })
    }
    /// The helper's run of one flag's delegations (`this`) -> `ReleaseMutex`'s result.
    RenderDelegationHelper_runFlag100 @ 0x006a_d760 => extern "thiscall" fn s_006ad760(this: u32) -> u32 {
        more::bridge::run_delegations(&mut mem(), this, const { more::delegation_flag(0x006a_d760) })
    }
    /// The helper's run of one flag's delegations (`this`) -> `ReleaseMutex`'s result.
    RenderDelegationHelper_runFlag40 @ 0x006a_d810 => extern "thiscall" fn s_006ad810(this: u32) -> u32 {
        more::bridge::run_delegations(&mut mem(), this, const { more::delegation_flag(0x006a_d810) })
    }
    /// The helper's run of one flag's delegations (`this`) -> `ReleaseMutex`'s result.
    RenderDelegationHelper_runFlag10 @ 0x006a_d8c0 => extern "thiscall" fn s_006ad8c0(this: u32) -> u32 {
        more::bridge::run_delegations(&mut mem(), this, const { more::delegation_flag(0x006a_d8c0) })
    }
    /// The helper's run of one flag's delegations (`this`) -> `ReleaseMutex`'s result.
    RenderDelegationHelper_runFlag20 @ 0x006a_d970 => extern "thiscall" fn s_006ad970(this: u32) -> u32 {
        more::bridge::run_delegations(&mut mem(), this, const { more::delegation_flag(0x006a_d970) })
    }
    /// The delegation array's compaction (`this`).
    RenderDelegationHelper_compact @ 0x006a_d590 => extern "thiscall" fn s_006ad590(this: u32) {
        more::bridge::compact_delegations(&mut mem(), this);
    }
    /// The render delegation helper's constructor (`this`) -> `this`.
    RenderDelegationHelper_RenderDelegationHelper @ 0x006a_d0d0 => extern "thiscall" fn s_006ad0d0(this: u32) -> u32 {
        more::bridge::delegation_helper_ctor(&mut mem(), this)
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment_Bind @ 0x005a_b400 => extern "C" fn s_005ab400() -> u32 {
        h2_bind::bridge::bind(&mut mem(), &binds::B_005AB400)
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PDynamicGeometry_PModifierNetworkInstanceInput_Bind @ 0x005b_1600 => extern "C" fn s_005b1600() -> u32 {
        h2_bind::bridge::bind(&mut mem(), &binds::B_005B1600)
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PDynamicGeometry_PModifierNetworkInstancePacketInput_Bind @ 0x005b_16e0 => extern "C" fn s_005b16e0() -> u32 {
        h2_bind::bridge::bind(&mut mem(), &binds::B_005B16E0)
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PDynamicGeometry_PModifierNetworkBuffer_Bind @ 0x005c_0eb0 => extern "C" fn s_005c0eb0() -> u32 {
        h2_bind::bridge::bind(&mut mem(), &binds::B_005C0EB0)
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_Buffer_Bind @ 0x005c_1390 => extern "C" fn s_005c1390() -> u32 {
        h2_bind::bridge::bind(&mut mem(), &binds::B_005C1390)
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_ModifierCode_Bind @ 0x005c_1430 => extern "C" fn s_005c1430() -> u32 {
        h2_bind::bridge::bind(&mut mem(), &binds::B_005C1430)
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_ModifierInstance_Bind @ 0x005c_1560 => extern "C" fn s_005c1560() -> u32 {
        h2_bind::bridge::bind(&mut mem(), &binds::B_005C1560)
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PDynamicGeometry_PRenderStream_Bind @ 0x005c_16e0 => extern "C" fn s_005c16e0() -> u32 {
        h2_bind::bridge::bind(&mut mem(), &binds::B_005C16E0)
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PDynamicGeometry_PRenderStreamInput_Bind @ 0x005c_17c0 => extern "C" fn s_005c17c0() -> u32 {
        h2_bind::bridge::bind(&mut mem(), &binds::B_005C17C0)
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PPostProcessing_PDeferredLightingBase_Bind @ 0x00a0_3500 => extern "C" fn s_00a03500() -> u32 {
        h2_bind::bridge::bind(&mut mem(), &binds::B_00A03500)
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PPostProcessing_PDepthOfFieldBase_Bind @ 0x00a0_37c0 => extern "C" fn s_00a037c0() -> u32 {
        h2_bind::bridge::bind(&mut mem(), &binds::B_00A037C0)
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PPostProcessing_PGlowBase_Bind @ 0x00a0_3990 => extern "C" fn s_00a03990() -> u32 {
        h2_bind::bridge::bind(&mut mem(), &binds::B_00A03990)
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PPostProcessing_PMotionBlurBase_Bind @ 0x00a0_3c20 => extern "C" fn s_00a03c20() -> u32 {
        h2_bind::bridge::bind(&mut mem(), &binds::B_00A03C20)
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PPostProcessing_PPostEffectBase_Bind @ 0x00a0_3ce0 => extern "C" fn s_00a03ce0() -> u32 {
        h2_bind::bridge::bind(&mut mem(), &binds::B_00A03CE0)
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PPostProcessing_PScreenSpaceReflectionBase_Bind @ 0x00a0_41c0 => extern "C" fn s_00a041c0() -> u32 {
        h2_bind::bridge::bind(&mut mem(), &binds::B_00A041C0)
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PText_PBitmapFontCharInfo_Bind @ 0x00a2_c350 => extern "C" fn s_00a2c350() -> u32 {
        h2_bind::bridge::bind(&mut mem(), &binds::B_00A2C350)
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PDynamicGeometry_PModifierNetworkDynamicMesh_Bind @ 0x005a_b0e0 => extern "C" fn s_005ab0e0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { binds1::bind_of(0x005a_b0e0) })
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PDynamicGeometry_PModifierNetworkInstance_Bind @ 0x005b_1400 => extern "C" fn s_005b1400() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { binds1::bind_of(0x005b_1400) })
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PDynamicGeometry_PModifierNetwork_Bind @ 0x005c_0970 => extern "C" fn s_005c0970() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { binds1::bind_of(0x005c_0970) })
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_Bind @ 0x005c_1080 => extern "C" fn s_005c1080() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { binds1::bind_of(0x005c_1080) })
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PDynamicGeometry_PMorphModifierWeightsUserDataObject_Bind @ 0x005c_b420 => extern "C" fn s_005cb420() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { binds1::bind_of(0x005c_b420) })
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PPostProcessing_PPostEffectManager_Bind @ 0x00a0_3dc0 => extern "C" fn s_00a03dc0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { binds1::bind_of(0x00a0_3dc0) })
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PText_PBitmapFont_Bind @ 0x00a2_c020 => extern "C" fn s_00a2c020() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { binds1::bind_of(0x00a2_c020) })
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PFramework_PApplicationViewport_Bind @ 0x0062_27b0 => extern "C" fn s_006227b0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { binds1::bind_of(0x0062_27b0) })
    }
    /// The class's `Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PFramework_PInputMapper_Bind @ 0x0062_2b60 => extern "C" fn s_00622b60() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { binds1::bind_of(0x0062_2b60) })
    }
}
