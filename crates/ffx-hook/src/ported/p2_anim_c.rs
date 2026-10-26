//! `PhyreEngine`'s `PAnimation` code, list `p2-anim` continued
//! (`ffx_panim::p2_bind`, `ffx_panim::p2_declare`, `ffx_panim::p2_more`).
//!
//! The animation classes' `Bind()` functions, as tables run by h1's
//! interpreter (`ffx_prender::h1_bind`) and, for
//! `PTimeController::Bind`, p3's (`ffx_pother_a::p3_bind`); their
//! `PArray<T>` / `PSharray<T>` declarations, the plain ones run by h2's
//! `ffx_prender::declare`, start-up code on the main thread; then
//! `PAnimationClipBinding`'s and `PAnimationClip`'s loading callbacks,
//! `PAnimationNetworkInstance`'s and `PAnimatableComponent`'s constructors
//! and destructors (loading threads too), a matrix copy. Nothing keeps
//! state of its own; the one local a callee sees is a per-call
//! `StackFrame`.

use super::{StackFrame, mem, ported};
use ffx_panim::{p2_bind, p2_declare, p2_more::bridge as more};
use ffx_pother_a::p3_bind;
use ffx_prender::{declare, h1_bind};

ported! {
    /// `Phyre_PAnimation_PAnimationSlotListIndex_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationSlotListIndex_Bind @ 0x0050_8470 => extern "C" fn bind_00508470() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0050_8470) })
    }
    /// `Phyre_PAnimation_PAnimationChannelTimes_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationChannelTimes_Bind @ 0x0050_9c50 => extern "C" fn bind_00509c50() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0050_9c50) })
    }
    /// `Phyre_PAnimation_PAnimationChannelTarget_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationChannelTarget_Bind @ 0x0050_b9a0 => extern "C" fn bind_0050b9a0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0050_b9a0) })
    }
    /// `Phyre_PAnimation_PAnimationChannel_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationChannel_Bind @ 0x0050_e120 => extern "C" fn bind_0050e120() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0050_e120) })
    }
    /// `Phyre_PAnimation_PAnimationClipBinding_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationClipBinding_Bind @ 0x0051_0450 => extern "C" fn bind_00510450() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0051_0450) })
    }
    /// `Phyre_PAnimation_PAnimationClipBinding_PAnimationClipBindingChannelMap_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationClipBinding_PAnimationClipBindingChannelMap_Bind @ 0x0051_05d0 => extern "C" fn bind_005105d0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0051_05d0) })
    }
    /// `Phyre_PAnimation_PAnimationClipBinding_PAnimationClipBindingDataBlockCache_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationClipBinding_PAnimationClipBindingDataBlockCache_Bind @ 0x0051_06b0 => extern "C" fn bind_005106b0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0051_06b0) })
    }
    /// `Phyre_PAnimation_PAnimationClip_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationClip_Bind @ 0x0051_3c40 => extern "C" fn bind_00513c40() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0051_3c40) })
    }
    /// `Phyre_PAnimation_PAnimationSet_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationSet_Bind @ 0x0051_9de0 => extern "C" fn bind_00519de0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0051_9de0) })
    }
    /// `Phyre_PAnimation_PAnimationEvent_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationEvent_Bind @ 0x0051_f470 => extern "C" fn bind_0051f470() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0051_f470) })
    }
    /// `Phyre_PAnimation_PAnimationEventList_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationEventList_Bind @ 0x0051_f550 => extern "C" fn bind_0051f550() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0051_f550) })
    }
    /// `Phyre_PAnimation_PTimeIntervalController_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PTimeIntervalController_Bind @ 0x0052_2700 => extern "C" fn bind_00522700() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0052_2700) })
    }
    /// `Phyre_PAnimation_PAnimationEventController_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationEventController_Bind @ 0x0052_4770 => extern "C" fn bind_00524770() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0052_4770) })
    }
    /// `Phyre_PAnimation_PAnimationSlotFilter_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationSlotFilter_Bind @ 0x0052_8330 => extern "C" fn bind_00528330() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0052_8330) })
    }
    /// `Phyre_PAnimation_PAnimationSlotFilterDeferredLoad_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationSlotFilterDeferredLoad_Bind @ 0x0052_8530 => extern "C" fn bind_00528530() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0052_8530) })
    }
    /// `Phyre_PAnimation_PAnimationBlenderController_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationBlenderController_Bind @ 0x0052_bcb0 => extern "C" fn bind_0052bcb0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0052_bcb0) })
    }
    /// `Phyre_PAnimation_PAnimationDataSourceListEntry_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationDataSourceListEntry_Bind @ 0x0053_5bf0 => extern "C" fn bind_00535bf0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0053_5bf0) })
    }
    /// `Phyre_PAnimation_PAnimationNetworkInstance_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationNetworkInstance_Bind @ 0x0053_5e10 => extern "C" fn bind_00535e10() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0053_5e10) })
    }
    /// `Phyre_PAnimation_PAnimatableComponent_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimatableComponent_Bind @ 0x0053_f510 => extern "C" fn bind_0053f510() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0053_f510) })
    }
    /// `Phyre_PAnimation_PAnimationChannelBase_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationChannelBase_Bind @ 0x0050_d2f0 => extern "C" fn bind_0050d2f0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0050_d2f0) })
    }
    /// `Phyre_PAnimation_PAnimationConstantChannel_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationConstantChannel_Bind @ 0x0050_f330 => extern "C" fn bind_0050f330() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0050_f330) })
    }
    /// `Phyre_PAnimation_PTimeScaleOffsetController_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PTimeScaleOffsetController_Bind @ 0x0052_3340 => extern "C" fn bind_00523340() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0052_3340) })
    }
    /// `Phyre_PAnimation_PAnimationDataSource_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationDataSource_Bind @ 0x0052_3e00 => extern "C" fn bind_00523e00() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0052_3e00) })
    }
    /// `Phyre_PAnimation_PAnimationController_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationController_Bind @ 0x0052_5440 => extern "C" fn bind_00525440() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0052_5440) })
    }
    /// `Phyre_PAnimation_PAnimationWeightedBlenderController_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationWeightedBlenderController_Bind @ 0x0052_d9e0 => extern "C" fn bind_0052d9e0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0052_d9e0) })
    }
    /// `Phyre_PAnimation_PAnimationAdditiveBlenderController_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationAdditiveBlenderController_Bind @ 0x0052_ee70 => extern "C" fn bind_0052ee70() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0052_ee70) })
    }
    /// `Phyre_PAnimation_PAnimationSpuTargetBlenderController_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationSpuTargetBlenderController_Bind @ 0x0053_01b0 => extern "C" fn bind_005301b0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0053_01b0) })
    }
    /// `Phyre_PAnimation_PAnimationTargetBlenderController_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PAnimationTargetBlenderController_Bind @ 0x0053_0f50 => extern "C" fn bind_00530f50() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { p2_bind::bind_of(0x0053_0f50) })
    }
    /// `Phyre_PAnimation_PTimeController_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PAnimation_PTimeController_Bind @ 0x0052_1bb0 => extern "C" fn bind_00521bb0() -> u32 {
        p3_bind::bridge::bind(&mut mem(), p2_bind::TIME_CONTROLLER)
    }
    /// `Phyre_PArray_float__DeclareClassDescriptor(name)` -> the descriptor.
    Phyre_PArray_float__DeclareClassDescriptor @ 0x0050_9da0 => extern "C" fn declare_00509da0(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { p2_declare::declare_of(0x0050_9da0) })
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationChannel_ptr__DeclareClassDescriptor(name)` -> the descriptor.
    Phyre_PArray_Phyre_PAnimation_PAnimationChannel_ptr__DeclareClassDescriptor @ 0x0051_45c0 => extern "C" fn declare_005145c0(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { p2_declare::declare_of(0x0051_45c0) })
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationConstantChannel__DeclareClassDescriptor(name)` -> the descriptor.
    Phyre_PArray_Phyre_PAnimation_PAnimationConstantChannel__DeclareClassDescriptor @ 0x0051_47d0 => extern "C" fn declare_005147d0(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { p2_declare::declare_of(0x0051_47d0) })
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget__DeclareClassDescriptor(name)` -> the descriptor.
    Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget__DeclareClassDescriptor @ 0x0051_a230 => extern "C" fn declare_0051a230(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { p2_declare::declare_of(0x0051_a230) })
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationSlotListIndex__DeclareClassDescriptor(name)` -> the descriptor.
    Phyre_PArray_Phyre_PAnimation_PAnimationSlotListIndex__DeclareClassDescriptor @ 0x0051_a440 => extern "C" fn declare_0051a440(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { p2_declare::declare_of(0x0051_a440) })
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationEvent_4u__DeclareClassDescriptor(name)` -> the descriptor.
    Phyre_PArray_Phyre_PAnimation_PAnimationEvent_4u__DeclareClassDescriptor @ 0x0051_f650 => extern "C" fn declare_0051f650(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { p2_declare::declare_of(0x0051_f650) })
    }
    /// `Phyre_PArray_Phyre_PUInt32__DeclareClassDescriptor(name)` -> the descriptor.
    Phyre_PArray_Phyre_PUInt32__DeclareClassDescriptor @ 0x0052_8610 => extern "C" fn declare_00528610(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { p2_declare::declare_of(0x0052_8610) })
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationSlotFilterDeferredLoad__DeclareClassDescriptor(name)` -> the descriptor.
    Phyre_PArray_Phyre_PAnimation_PAnimationSlotFilterDeferredLoad__DeclareClassDescriptor @ 0x0052_8820 => extern "C" fn declare_00528820(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { p2_declare::declare_of(0x0052_8820) })
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_8u__DeclareClassDescriptor(name)` -> the descriptor.
    Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_8u__DeclareClassDescriptor @ 0x0052_bdb0 => extern "C" fn declare_0052bdb0(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { p2_declare::declare_of(0x0052_bdb0) })
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationDataSourceListEntry__DeclareClassDescriptor(name)` -> the descriptor.
    Phyre_PArray_Phyre_PAnimation_PAnimationDataSourceListEntry__DeclareClassDescriptor @ 0x0053_6dc0 => extern "C" fn declare_00536dc0(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { p2_declare::declare_of(0x0053_6dc0) })
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationNetworkInstanceTarget__DeclareClassDescriptor(name)` -> the descriptor.
    Phyre_PArray_Phyre_PAnimation_PAnimationNetworkInstanceTarget__DeclareClassDescriptor @ 0x0053_6fe0 => extern "C" fn declare_00536fe0(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { p2_declare::declare_of(0x0053_6fe0) })
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationSlotArray__DeclareClassDescriptor(name)` -> the descriptor.
    Phyre_PArray_Phyre_PAnimation_PAnimationSlotArray__DeclareClassDescriptor @ 0x0053_71f0 => extern "C" fn declare_005371f0(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &const { p2_declare::declare_of(0x0053_71f0) })
    }
    /// `Phyre_PArray_Phyre_PUInt128__DeclareClassDescriptor(name)` -> the descriptor.
    Phyre_PArray_Phyre_PUInt128__DeclareClassDescriptor @ 0x0053_7400 => extern "C" fn declare_00537400(name: u32) -> u32 {
        p2_declare::bridge::declare_sized(&mut mem(), name, &p2_declare::PUINT128, p2_declare::PUINT128_SIZE)
    }
    /// `Phyre_PSharray_Phyre_PAnimation_PAnimationClip_ptr__DeclareClassDescriptor(name)` -> the descriptor.
    Phyre_PSharray_Phyre_PAnimation_PAnimationClip_ptr__DeclareClassDescriptor @ 0x0051_a650 => extern "C" fn declare_0051a650(name: u32) -> u32 {
        p2_declare::bridge::declare_sharray(&mut mem(), name, &p2_declare::CLIP_SHARRAY)
    }
    // batch 2: the hand-written ones (ffx_panim::p2_more)
    /// `Phyre_PAnimation_PAnimationClipBinding_headerVisit_00510f90`.
    Phyre_PAnimation_PAnimationClipBinding_headerVisit_00510f90 @ 0x0051_0f90 => extern "C" fn header_visit_00510f90(header: u32, stream: u32) -> u32 {
        more::header_visit(&mut mem(), header, stream)
    }
    /// `Phyre_PClassDescriptorHeader_Phyre_PAnimation_PAnimationClipBinding__destructInstance`.
    Phyre_PClassDescriptorHeader_Phyre_PAnimation_PAnimationClipBinding__destructInstance @ 0x0051_1130 => extern "thiscall" fn header_destruct_00511130(this: u32, object: u32) -> u32 {
        let mut frame = StackFrame::<2>::new();
        more::header_destruct(&mut mem(), this, object, frame.at())
    }
    /// `Phyre_PAnimation_PAnimationClipBinding_fixup_00511980`.
    Phyre_PAnimation_PAnimationClipBinding_fixup_00511980 @ 0x0051_1980 => extern "thiscall" fn fixup_00511980(this: u32, entry: u32) -> u32 {
        more::clip_binding_fixup(&mut mem(), this, entry)
    }
    /// `Phyre_PClassDescriptorHeader_Phyre_PAnimation_PAnimationClipBinding__slot3c`.
    Phyre_PClassDescriptorHeader_Phyre_PAnimation_PAnimationClipBinding__slot3c @ 0x0051_1ac0 => extern "thiscall" fn header_slot3c_00511ac0(this: u32, header: u32, stream: u32) -> u32 {
        more::header_slot3c(&mut mem(), this, header, stream)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationClip__fixObjectsAfterLoading`.
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationClip__fixObjectsAfterLoading @ 0x0051_6370 => extern "stdcall" fn fix_after_loading_00516370(object: u32) -> u32 {
        more::clip_fix_after_loading(&mut mem(), object)
    }
    /// `Vectormath_Matrix3_fromMatrix4Upper_00531a40`.
    Vectormath_Matrix3_fromMatrix4Upper_00531a40 @ 0x0053_1a40 => extern "thiscall" fn upper3x3_00531a40(this: u32, src: u32) -> u32 {
        more::upper3x3(&mut mem(), this, src)
    }
    /// `Phyre_PAnimation_PAnimationNetworkInstance_PAnimationNetworkInstance`.
    Phyre_PAnimation_PAnimationNetworkInstance_PAnimationNetworkInstance @ 0x0053_4450 => extern "thiscall" fn ctor_00534450(this: u32) -> u32 {
        more::network_instance_ctor(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PAnimationNetworkInstance_dtor_PAnimationNetworkInstance`.
    Phyre_PAnimation_PAnimationNetworkInstance_dtor_PAnimationNetworkInstance @ 0x0053_4b10 => extern "thiscall" fn dtor_00534b10(this: u32) -> u32 {
        more::network_instance_dtor(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PAnimationNetworkInstance_InitializeThreadPool`.
    Phyre_PAnimation_PAnimationNetworkInstance_InitializeThreadPool @ 0x0053_9640 => extern "C" fn thread_pool_00539640() -> u32 {
        more::initialize_thread_pool(&mut mem())
    }
    /// `Phyre_PAnimation_PAnimatableComponent_PAnimatableComponent`.
    Phyre_PAnimation_PAnimatableComponent_PAnimatableComponent @ 0x0053_e720 => extern "thiscall" fn ctor_0053e720(this: u32) -> u32 {
        more::animatable_ctor(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_dtor_PAnimatableComponent`.
    Phyre_PAnimation_PAnimatableComponent_dtor_PAnimatableComponent @ 0x0053_ebd0 => extern "thiscall" fn dtor_0053ebd0(this: u32) -> u32 {
        more::animatable_dtor(&mut mem(), this)
    }
}
