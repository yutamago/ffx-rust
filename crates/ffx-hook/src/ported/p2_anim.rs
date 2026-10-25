//! `PhyreEngine`'s `PAnimation` code (`ffx_panim`, M4 port round 1, list
//! `p2-anim`).
//!
//! The class-descriptor callbacks of the animation classes (the
//! `PClassDescriptor*` vtables' construct, destruct and fix-after-loading
//! slots, `ffx_panim::descriptor`), the `PArray` setters and dynamic
//! member callbacks (`ffx_panim::array`) and the controllers'
//! constructors and destructors (`ffx_panim::controller`). They run on the
//! loading threads as well as the main thread; none keeps state.

use super::{mem, ported};
use ffx_panim::{array, controller, descriptor, network};

ported! {
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationSlotArray__constructInstance` (0x00508c40).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationSlotArray__constructInstance @ 0x0050_8c40 => extern "stdcall" fn true_508c40(_p: u32) -> bool {
        true
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSlotArray__destructInstance` (0x00508c80).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSlotArray__destructInstance @ 0x0050_8c80 => extern "stdcall" fn true_508c80(_p: u32) -> bool {
        true
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSlotListIndex__destructInstance` (0x00508c90).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSlotListIndex__destructInstance @ 0x0050_8c90 => extern "stdcall" fn true_508c90(_p: u32) -> bool {
        true
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationClipBinding_PAnimationClipBindingChannelMap__constructInstance` (0x005110b0).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationClipBinding_PAnimationClipBindingChannelMap__constructInstance @ 0x0051_10b0 => extern "stdcall" fn true_5110b0(_p: u32) -> bool {
        true
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationClipBinding_PAnimationClipBindingDataBlockCache__constructInstance` (0x005110c0).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationClipBinding_PAnimationClipBindingDataBlockCache__constructInstance @ 0x0051_10c0 => extern "stdcall" fn true_5110c0(_p: u32) -> bool {
        true
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationClipBinding_PAnimationClipBindingChannelMap__destructInstance` (0x00511110).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationClipBinding_PAnimationClipBindingChannelMap__destructInstance @ 0x0051_1110 => extern "stdcall" fn true_511110(_p: u32) -> bool {
        true
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationClipBinding_PAnimationClipBindingDataBlockCache__destructInstance` (0x00511120).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationClipBinding_PAnimationClipBindingDataBlockCache__destructInstance @ 0x0051_1120 => extern "stdcall" fn true_511120(_p: u32) -> bool {
        true
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationEvent__destructInstance` (0x00520740).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationEvent__destructInstance @ 0x0052_0740 => extern "stdcall" fn true_520740(_p: u32) -> bool {
        true
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationDataSourceListEntry__destructInstance` (0x0053a890).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationDataSourceListEntry__destructInstance @ 0x0053_a890 => extern "stdcall" fn true_53a890(_p: u32) -> bool {
        true
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationNetworkInstanceTarget__destructInstance` (0x0053a8d0).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationNetworkInstanceTarget__destructInstance @ 0x0053_a8d0 => extern "stdcall" fn true_53a8d0(_p: u32) -> bool {
        true
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PAnimation_PAnimationSlotListIndex__fixObjectsAfterLoading` (0x00508cb0).
    Phyre_PClassDescriptorAbstract_Phyre_PAnimation_PAnimationSlotListIndex__fixObjectsAfterLoading @ 0x0050_8cb0 => extern "stdcall" fn zero_508cb0(_p: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PAnimation_PAnimationChannelTimes__fixObjectsAfterLoading` (0x0050ab80).
    Phyre_PClassDescriptorAbstract_Phyre_PAnimation_PAnimationChannelTimes__fixObjectsAfterLoading @ 0x0050_ab80 => extern "stdcall" fn zero_50ab80(_p: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PAnimation_PAnimationChannelTarget__fixObjectsAfterLoading` (0x0050c5b0).
    Phyre_PClassDescriptorAbstract_Phyre_PAnimation_PAnimationChannelTarget__fixObjectsAfterLoading @ 0x0050_c5b0 => extern "stdcall" fn zero_50c5b0(_p: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PAnimation_PAnimationChannel__fixObjectsAfterLoading` (0x0050e720).
    Phyre_PClassDescriptorAbstract_Phyre_PAnimation_PAnimationChannel__fixObjectsAfterLoading @ 0x0050_e720 => extern "stdcall" fn zero_50e720(_p: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PAnimation_PAnimationConstantChannel__fixObjectsAfterLoading` (0x0050f730).
    Phyre_PClassDescriptorAbstract_Phyre_PAnimation_PAnimationConstantChannel__fixObjectsAfterLoading @ 0x0050_f730 => extern "stdcall" fn zero_50f730(_p: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PAnimation_PAnimationClipBinding__fixObjectsAfterLoading` (0x00511190).
    Phyre_PClassDescriptorAbstract_Phyre_PAnimation_PAnimationClipBinding__fixObjectsAfterLoading @ 0x0051_1190 => extern "stdcall" fn zero_511190(_p: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PAnimation_PAnimationSet__fixObjectsAfterLoading` (0x0051cc40).
    Phyre_PClassDescriptorAbstract_Phyre_PAnimation_PAnimationSet__fixObjectsAfterLoading @ 0x0051_cc40 => extern "stdcall" fn zero_51cc40(_p: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_float_4__constructInstance` (0x0050a950).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_float_4__constructInstance @ 0x0050_a950 => extern "stdcall" fn init_50a950(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 2])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationChannel_4__constructInstance` (0x00516100).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationChannel_4__constructInstance @ 0x0051_6100 => extern "stdcall" fn init_516100(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 2])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationConstantChannel_4__constructInstance` (0x00516120).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationConstantChannel_4__constructInstance @ 0x0051_6120 => extern "stdcall" fn init_516120(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 2])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget_4__constructInstance` (0x0051c8a0).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget_4__constructInstance @ 0x0051_c8a0 => extern "stdcall" fn init_51c8a0(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 2])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationSlotListIndex_4__constructInstance` (0x0051c8c0).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationSlotListIndex_4__constructInstance @ 0x0051_c8c0 => extern "stdcall" fn init_51c8c0(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 2])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PSharray_Phyre_PAnimation_PAnimationClip__constructInstance` (0x0051c8e0).
    Phyre_PClassDescriptorConcrete_Phyre_PSharray_Phyre_PAnimation_PAnimationClip__constructInstance @ 0x0051_c8e0 => extern "stdcall" fn init_51c8e0(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 2])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationEvent_4__constructInstance` (0x005206a0).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationEvent_4__constructInstance @ 0x0052_06a0 => extern "stdcall" fn init_5206a0(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 2])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationEventList__constructInstance` (0x005206e0).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationEventList__constructInstance @ 0x0052_06e0 => extern "stdcall" fn init_5206e0(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 2])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_unsigned_int_4__constructInstance` (0x0052a020).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_unsigned_int_4__constructInstance @ 0x0052_a020 => extern "stdcall" fn init_52a020(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 2])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationSlotFilterDeferredLoad_4__constructInstance` (0x0052a040).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationSlotFilterDeferredLoad_4__constructInstance @ 0x0052_a040 => extern "stdcall" fn init_52a040(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 2])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationSlotFilterDeferredLoad__constructInstance` (0x0052a0e0).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationSlotFilterDeferredLoad__constructInstance @ 0x0052_a0e0 => extern "stdcall" fn init_52a0e0(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 2])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_4__constructInstance` (0x0052ca20).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_4__constructInstance @ 0x0052_ca20 => extern "stdcall" fn init_52ca20(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 2])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationDataSourceListEntry_4__constructInstance` (0x0053a680).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationDataSourceListEntry_4__constructInstance @ 0x0053_a680 => extern "stdcall" fn init_53a680(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 2])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationNetworkInstanceTarget_4__constructInstance` (0x0053a6a0).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationNetworkInstanceTarget_4__constructInstance @ 0x0053_a6a0 => extern "stdcall" fn init_53a6a0(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 2])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationSlotArray_4__constructInstance` (0x0053a6c0).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationSlotArray_4__constructInstance @ 0x0053_a6c0 => extern "stdcall" fn init_53a6c0(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 2])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PQWord_16__constructInstance` (0x0053a6e0).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PQWord_16__constructInstance @ 0x0053_a6e0 => extern "stdcall" fn init_53a6e0(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 2])
    }
    /// `Phyre_PClassDescriptorHeader_Phyre_PAnimation_PAnimationClipBinding__constructInstance` (0x005110d0).
    Phyre_PClassDescriptorHeader_Phyre_PAnimation_PAnimationClipBinding__constructInstance @ 0x0051_10d0 => extern "stdcall" fn init_5110d0(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 2])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationEvent__constructInstance` (0x005206c0).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationEvent__constructInstance @ 0x0052_06c0 => extern "stdcall" fn init_5206c0(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 2])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationChannelTimes__constructInstance` (0x0050a970).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationChannelTimes__constructInstance @ 0x0050_a970 => extern "stdcall" fn init_50a970(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 3])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationSlotListIndex__constructInstance` (0x00508c50).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationSlotListIndex__constructInstance @ 0x0050_8c50 => extern "stdcall" fn init_508c50(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0, 3, 0, 0])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationSet__constructInstance` (0x0051c900).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationSet__constructInstance @ 0x0051_c900 => extern "stdcall" fn init_51c900(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 6])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationNetworkInstanceTarget__constructInstance` (0x0053a740).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationNetworkInstanceTarget__constructInstance @ 0x0053_a740 => extern "stdcall" fn init_53a740(p: u32) -> u32 {
        descriptor::bridge::init_words(&mut mem(), p, &[0; 6])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationClip__constructInstance` (0x00516140).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationClip__constructInstance @ 0x0051_6140 => extern "stdcall" fn init_516140(p: u32) -> u32 {
        descriptor::bridge::clip_init(&mut mem(), p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_float_4__destructInstance` (0x0050a9a0).
    Phyre_PClassDescriptorForType_Phyre_PArray_float_4__destructInstance @ 0x0050_a9a0 => extern "stdcall" fn free_50a9a0(p: u32) -> u32 {
        descriptor::bridge::free_owned(&mut mem(), p, true)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationSlotListIndex_4__destructInstance` (0x0051c980).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationSlotListIndex_4__destructInstance @ 0x0051_c980 => extern "stdcall" fn free_51c980(p: u32) -> u32 {
        descriptor::bridge::free_owned(&mut mem(), p, true)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationEvent_4__destructInstance` (0x00520700).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationEvent_4__destructInstance @ 0x0052_0700 => extern "stdcall" fn free_520700(p: u32) -> u32 {
        descriptor::bridge::free_owned(&mut mem(), p, true)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationEventList__destructInstance` (0x00520750).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationEventList__destructInstance @ 0x0052_0750 => extern "stdcall" fn free_520750(p: u32) -> u32 {
        descriptor::bridge::free_owned(&mut mem(), p, true)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_unsigned_int_4__destructInstance` (0x0052a100).
    Phyre_PClassDescriptorForType_Phyre_PArray_unsigned_int_4__destructInstance @ 0x0052_a100 => extern "stdcall" fn free_52a100(p: u32) -> u32 {
        descriptor::bridge::free_owned(&mut mem(), p, true)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationDataSourceListEntry_4__destructInstance` (0x0053a790).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationDataSourceListEntry_4__destructInstance @ 0x0053_a790 => extern "stdcall" fn free_53a790(p: u32) -> u32 {
        descriptor::bridge::free_owned(&mut mem(), p, true)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationNetworkInstanceTarget_4__destructInstance` (0x0053a7d0).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationNetworkInstanceTarget_4__destructInstance @ 0x0053_a7d0 => extern "stdcall" fn free_53a7d0(p: u32) -> u32 {
        descriptor::bridge::free_owned(&mut mem(), p, true)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationSlotArray_4__destructInstance` (0x0053a810).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationSlotArray_4__destructInstance @ 0x0053_a810 => extern "stdcall" fn free_53a810(p: u32) -> u32 {
        descriptor::bridge::free_owned(&mut mem(), p, true)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PQWord_16__destructInstance` (0x0053a850).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PQWord_16__destructInstance @ 0x0053_a850 => extern "stdcall" fn free_53a850(p: u32) -> u32 {
        descriptor::bridge::free_owned(&mut mem(), p, true)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationChannelTimes__destructInstance` (0x0050a9e0).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationChannelTimes__destructInstance @ 0x0050_a9e0 => extern "stdcall" fn free_50a9e0(p: u32) -> u32 {
        descriptor::bridge::free_owned(&mut mem(), p.wrapping_add(4), true)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationChannel_4__destructInstance` (0x005162d0).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationChannel_4__destructInstance @ 0x0051_62d0 => extern "stdcall" fn free_5162d0(p: u32) -> u32 {
        descriptor::bridge::free_owned(&mut mem(), p, false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_4__destructInstance` (0x0052ca40).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_4__destructInstance @ 0x0052_ca40 => extern "stdcall" fn free_52ca40(p: u32) -> u32 {
        descriptor::bridge::free_owned(&mut mem(), p, false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PAnimation_PAnimationClip__destructInstance` (0x0051c9c0).
    Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PAnimation_PAnimationClip__destructInstance @ 0x0051_c9c0 => extern "stdcall" fn free_51c9c0(p: u32) -> u32 {
        descriptor::bridge::free_owned_shared(&mut mem(), p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget_4__destructInstance` (0x0051c940).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget_4__destructInstance @ 0x0051_c940 => extern "stdcall" fn free_51c940(p: u32) -> u32 {
        descriptor::bridge::free_owned_elements(&mut mem(), p, 0x1c, 0x14)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationSlotFilterDeferredLoad_4__destructInstance` (0x0052a140).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationSlotFilterDeferredLoad_4__destructInstance @ 0x0052_a140 => extern "stdcall" fn free_52a140(p: u32) -> u32 {
        descriptor::bridge::free_owned_elements(&mut mem(), p, 8, 4)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationChannelTarget__destructInstance` (0x0050c590).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationChannelTarget__destructInstance @ 0x0050_c590 => extern "stdcall" fn flag_50c590(p: u32) -> bool {
        descriptor::bridge::free_flagged(&mut mem(), p, 0x14, None)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSlotFilterDeferredLoad__destructInstance` (0x0052a1b0).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSlotFilterDeferredLoad__destructInstance @ 0x0052_a1b0 => extern "stdcall" fn flag_52a1b0(p: u32) -> bool {
        descriptor::bridge::free_flagged(&mut mem(), p, 4, None)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationChannelBase__destructInstance` (0x0050d740).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationChannelBase__destructInstance @ 0x0050_d740 => extern "stdcall" fn flag_50d740(p: u32) -> bool {
        descriptor::bridge::free_flagged(&mut mem(), p, 0x14, Some(0x1c))
    }
    /// `Phyre_PAnimation_PAnimationChannelTarget_freeArray(els, n)` (0x0051a890): leaves `eax` (callers drop it).
    Phyre_PAnimation_PAnimationChannelTarget_freeArray @ 0x0051_a890 => extern "C" fn free_elements_51a890(els: u32, n: u32) {
        descriptor::bridge::free_elements(&mut mem(), els, n, 0x1c, 0x14);
    }
    /// `Phyre_PAnimation_PAnimationSlotFilterDeferredLoad_freeArray(els, n)` (0x00528a30): leaves `eax` (callers drop it).
    Phyre_PAnimation_PAnimationSlotFilterDeferredLoad_freeArray @ 0x0052_8a30 => extern "C" fn free_elements_528a30(els: u32, n: u32) {
        descriptor::bridge::free_elements(&mut mem(), els, n, 0x8, 0x4);
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PTimeController__destructInstance` (0x005220a0).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PTimeController__destructInstance @ 0x0052_20a0 => extern "stdcall" fn virtual_5220a0(p: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PTimeIntervalController__destructInstance` (0x00522c60).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PTimeIntervalController__destructInstance @ 0x0052_2c60 => extern "stdcall" fn virtual_522c60(p: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PTimeScaleOffsetController__destructInstance` (0x00523810).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PTimeScaleOffsetController__destructInstance @ 0x0052_3810 => extern "stdcall" fn virtual_523810(p: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationEventController__destructInstance` (0x00524c70).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationEventController__destructInstance @ 0x0052_4c70 => extern "stdcall" fn virtual_524c70(p: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationController__destructInstance` (0x005261a0).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationController__destructInstance @ 0x0052_61a0 => extern "stdcall" fn virtual_5261a0(p: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSlotFilter__destructInstance` (0x0052a180).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSlotFilter__destructInstance @ 0x0052_a180 => extern "stdcall" fn virtual_52a180(p: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationWeightedBlenderController__destructInstance` (0x0052e1f0).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationWeightedBlenderController__destructInstance @ 0x0052_e1f0 => extern "stdcall" fn virtual_52e1f0(p: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationAdditiveBlenderController__destructInstance` (0x0052f6e0).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationAdditiveBlenderController__destructInstance @ 0x0052_f6e0 => extern "stdcall" fn virtual_52f6e0(p: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSpuTargetBlenderController__destructInstance` (0x00530620).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSpuTargetBlenderController__destructInstance @ 0x0053_0620 => extern "stdcall" fn virtual_530620(p: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationTargetBlenderController__destructInstance` (0x00531480).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationTargetBlenderController__destructInstance @ 0x0053_1480 => extern "stdcall" fn virtual_531480(p: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationNetworkInstance__destructInstance` (0x0053a8a0).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationNetworkInstance__destructInstance @ 0x0053_a8a0 => extern "stdcall" fn virtual_53a8a0(p: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationConstantChannel__destructInstance` (0x0050f710).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationConstantChannel__destructInstance @ 0x0050_f710 => extern "stdcall" fn call_50f710(p: u32) -> u32 {
        descriptor::bridge::call_on(&mut mem(), p, 0x0050_d1d0)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationConstantChannel_4__destructInstance` (0x00516310).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationConstantChannel_4__destructInstance @ 0x0051_6310 => extern "stdcall" fn call_516310(p: u32) -> u32 {
        descriptor::bridge::call_on(&mut mem(), p, 0x0051_3240)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationClip__destructInstance` (0x00516330).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationClip__destructInstance @ 0x0051_6330 => extern "stdcall" fn call_516330(p: u32) -> u32 {
        descriptor::bridge::call_on(&mut mem(), p, 0x0051_33f0)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSet__destructInstance` (0x0051ca00).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSet__destructInstance @ 0x0051_ca00 => extern "stdcall" fn call_51ca00(p: u32) -> u32 {
        descriptor::bridge::call_on(&mut mem(), p, 0x0051_9380)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationDataSourceListEntry__constructInstance` (0x0053a700).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationDataSourceListEntry__constructInstance @ 0x0053_a700 => extern "stdcall" fn call_53a700(p: u32) -> bool {
        descriptor::bridge::call_on_nonnull(&mut mem(), p, 0x0053_4360)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationNetworkInstance__constructInstance` (0x0053a720).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationNetworkInstance__constructInstance @ 0x0053_a720 => extern "stdcall" fn call_53a720(p: u32) -> bool {
        descriptor::bridge::call_on_nonnull(&mut mem(), p, 0x0053_4450)
    }
    /// `Phyre_PAnimation_RegisterDescriptor_00508450` (0x00508450).
    Phyre_PAnimation_RegisterDescriptor_00508450 @ 0x0050_8450 => extern "C" fn register_508450() -> u32 {
        descriptor::bridge::register(&mut mem(), 0x00ca_9810)
    }
    /// `Phyre_PAnimation_RegisterDescriptor_00521300` (0x00521300).
    Phyre_PAnimation_RegisterDescriptor_00521300 @ 0x0052_1300 => extern "C" fn register_521300() -> u32 {
        descriptor::bridge::register(&mut mem(), 0x00ca_b858)
    }
    /// `Phyre_PAnimation_RegisterDescriptor_00536210` (0x00536210).
    Phyre_PAnimation_RegisterDescriptor_00536210 @ 0x0053_6210 => extern "C" fn register_536210() -> u32 {
        descriptor::bridge::register(&mut mem(), 0x00ca_cdb0)
    }
    /// `Phyre_PArray_float_setBorrowed` (0x0050aff0).
    Phyre_PArray_float_setBorrowed @ 0x0050_aff0 => extern "thiscall" fn set_50aff0(this: u32, els: u32, count: u32) -> u32 {
        array::bridge::set_borrowed(&mut mem(), this, els, count, array::Old::Free)
    }
    /// `Phyre_PArray_PAnimationSlotListIndex_setBorrowed` (0x0051dba0).
    Phyre_PArray_PAnimationSlotListIndex_setBorrowed @ 0x0051_dba0 => extern "thiscall" fn set_51dba0(this: u32, els: u32, count: u32) -> u32 {
        array::bridge::set_borrowed(&mut mem(), this, els, count, array::Old::Free)
    }
    /// `Phyre_PArray_unsigned_int_setBorrowed` (0x0052aae0).
    Phyre_PArray_unsigned_int_setBorrowed @ 0x0052_aae0 => extern "thiscall" fn set_52aae0(this: u32, els: u32, count: u32) -> u32 {
        array::bridge::set_borrowed(&mut mem(), this, els, count, array::Old::Free)
    }
    /// `Phyre_PArray_PAnimationConstantChannel_setBorrowed` (0x00517380).
    Phyre_PArray_PAnimationConstantChannel_setBorrowed @ 0x0051_7380 => extern "thiscall" fn set_517380(this: u32, els: u32, count: u32) -> u32 {
        array::bridge::set_borrowed(&mut mem(), this, els, count, array::Old::FreeAlways)
    }
    /// `Phyre_PArray_PAnimationChannelTarget_setBorrowed` (0x0051db50).
    Phyre_PArray_PAnimationChannelTarget_setBorrowed @ 0x0051_db50 => extern "thiscall" fn set_51db50(this: u32, els: u32, count: u32) -> u32 {
        array::bridge::set_borrowed(&mut mem(), this, els, count, array::Old::Targets)
    }
    /// `Phyre_PArray_PAnimationChannel_assign` (0x00517270).
    Phyre_PArray_PAnimationChannel_assign @ 0x0051_7270 => extern "thiscall" fn assign_517270(this: u32, els: u32, count: u32) -> u32 {
        array::bridge::assign_channels(&mut mem(), this, els, count)
    }
    /// `Phyre_PArray_PAnimationChannel_setBorrowed` (0x005173d0).
    Phyre_PArray_PAnimationChannel_setBorrowed @ 0x0051_73d0 => extern "thiscall" fn set_5173d0(this: u32, els: u32, count: u32) -> u32 {
        array::bridge::set_channels(&mut mem(), this, els, count)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_float_4__vslot20` (0x0050b040).
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_float_4__vslot20 @ 0x0050_b040 => extern "thiscall" fn fwd3c_50b040(desc: u32, object: u32, a: u32, b: u32) -> u32 {
        array::bridge::forward_3c(&mut mem(), desc, object, a, b)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PAnimation_PAnimationChannel_4__vslot20` (0x005173f0).
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PAnimation_PAnimationChannel_4__vslot20 @ 0x0051_73f0 => extern "thiscall" fn fwd3c_5173f0(desc: u32, object: u32, a: u32, b: u32) -> u32 {
        array::bridge::forward_3c(&mut mem(), desc, object, a, b)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PAnimation_PAnimationConstantChannel_4__vslot20` (0x00517420).
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PAnimation_PAnimationConstantChannel_4__vslot20 @ 0x0051_7420 => extern "thiscall" fn fwd3c_517420(desc: u32, object: u32, a: u32, b: u32) -> u32 {
        array::bridge::forward_3c(&mut mem(), desc, object, a, b)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget_4__vslot20` (0x0051dc40).
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget_4__vslot20 @ 0x0051_dc40 => extern "thiscall" fn fwd3c_51dc40(desc: u32, object: u32, a: u32, b: u32) -> u32 {
        array::bridge::forward_3c(&mut mem(), desc, object, a, b)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PAnimation_PAnimationSlotListIndex_4__vslot20` (0x0051dc70).
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PAnimation_PAnimationSlotListIndex_4__vslot20 @ 0x0051_dc70 => extern "thiscall" fn fwd3c_51dc70(desc: u32, object: u32, a: u32, b: u32) -> u32 {
        array::bridge::forward_3c(&mut mem(), desc, object, a, b)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_unsigned_int_4__vslot20` (0x0052ab80).
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_unsigned_int_4__vslot20 @ 0x0052_ab80 => extern "thiscall" fn fwd3c_52ab80(desc: u32, object: u32, a: u32, b: u32) -> u32 {
        array::bridge::forward_3c(&mut mem(), desc, object, a, b)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PAnimation_PAnimationChannel_4__vslot18` (0x005166f0).
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PAnimation_PAnimationChannel_4__vslot18 @ 0x0051_66f0 => extern "thiscall" fn fwd38_5166f0(desc: u32, i: u32) -> u32 {
        array::bridge::forward_38(&mut mem(), desc, i)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PSharray_Phyre_PAnimation_PAnimationClip__vslot18` (0x0051cf10).
    Phyre_PClassDataMemberArrayDynamic_Phyre_PSharray_Phyre_PAnimation_PAnimationClip__vslot18 @ 0x0051_cf10 => extern "thiscall" fn fwd38_51cf10(desc: u32, i: u32) -> u32 {
        array::bridge::forward_38(&mut mem(), desc, i)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PSharray_Phyre_PAnimation_PAnimationClip__vslot10` (0x0051d0b0).
    Phyre_PClassDataMemberArrayDynamic_Phyre_PSharray_Phyre_PAnimation_PAnimationClip__vslot10 @ 0x0051_d0b0 => extern "thiscall" fn fwd34_51d0b0(desc: u32, object: u32) -> u32 {
        array::bridge::forward_34(&mut mem(), desc, object)
    }
    /// `Phyre_PAnimation_PAnimationHierarchyNode_PAnimationHierarchyNode` (0x005211f0).
    Phyre_PAnimation_PAnimationHierarchyNode_PAnimationHierarchyNode @ 0x0052_11f0 => extern "thiscall" fn hierarchy_node_ctor(this: u32) -> u32 {
        controller::bridge::hierarchy_node_ctor(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PTimeController_PTimeController` (0x00521a50).
    Phyre_PAnimation_PTimeController_PTimeController @ 0x0052_1a50 => extern "thiscall" fn time_controller_ctor(this: u32) -> u32 {
        controller::bridge::time_controller_ctor(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PTimeIntervalController_PTimeIntervalController` (0x005225a0).
    Phyre_PAnimation_PTimeIntervalController_PTimeIntervalController @ 0x0052_25a0 => extern "thiscall" fn time_interval_ctor(this: u32) -> u32 {
        controller::bridge::time_interval_ctor(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PTimeScaleOffsetController_PTimeScaleOffsetController` (0x005231e0).
    Phyre_PAnimation_PTimeScaleOffsetController_PTimeScaleOffsetController @ 0x0052_31e0 => extern "thiscall" fn time_scale_offset_ctor(this: u32) -> u32 {
        controller::bridge::time_scale_offset_ctor(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PAnimationDataSource_PAnimationDataSource` (0x00523d20).
    Phyre_PAnimation_PAnimationDataSource_PAnimationDataSource @ 0x0052_3d20 => extern "thiscall" fn data_source_ctor(this: u32) -> u32 {
        controller::bridge::data_source_ctor(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PAnimationController_PAnimationController` (0x00525240).
    Phyre_PAnimation_PAnimationController_PAnimationController @ 0x0052_5240 => extern "thiscall" fn controller_ctor(this: u32) -> u32 {
        controller::bridge::controller_ctor(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PAnimationBlenderController_PAnimationBlenderController` (0x0052b820).
    Phyre_PAnimation_PAnimationBlenderController_PAnimationBlenderController @ 0x0052_b820 => extern "thiscall" fn blender_ctor(this: u32) -> u32 {
        controller::bridge::blender_ctor(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PAnimationWeightedBlenderController_PAnimationWeightedBlenderController` (0x0052d660).
    Phyre_PAnimation_PAnimationWeightedBlenderController_PAnimationWeightedBlenderController @ 0x0052_d660 => extern "thiscall" fn weighted_blender_ctor(this: u32) -> u32 {
        controller::bridge::weighted_blender_ctor(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PAnimationTargetBlenderController_PAnimationTargetBlenderController` (0x00530dd0).
    Phyre_PAnimation_PAnimationTargetBlenderController_PAnimationTargetBlenderController @ 0x0053_0dd0 => extern "thiscall" fn target_blender_ctor(this: u32) -> u32 {
        controller::bridge::target_blender_ctor(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PAnimationHierarchyNode_dtor_PAnimationHierarchyNode` (0x00521220).
    Phyre_PAnimation_PAnimationHierarchyNode_dtor_PAnimationHierarchyNode @ 0x0052_1220 => extern "thiscall" fn dtor_521220(this: u32) {
        controller::bridge::hierarchy_node_dtor(&mut mem(), this);
    }
    /// `Phyre_PAnimation_PTimeIntervalController_dtor_PTimeIntervalController` (0x00522600).
    Phyre_PAnimation_PTimeIntervalController_dtor_PTimeIntervalController @ 0x0052_2600 => extern "thiscall" fn dtor_522600(this: u32) {
        controller::bridge::dtor_over_node(&mut mem(), this, controller::vtable::TIME_INTERVAL);
    }
    /// `Phyre_PAnimation_PTimeScaleOffsetController_dtor_PTimeScaleOffsetController` (0x00523240).
    Phyre_PAnimation_PTimeScaleOffsetController_dtor_PTimeScaleOffsetController @ 0x0052_3240 => extern "thiscall" fn dtor_523240(this: u32) {
        controller::bridge::dtor_over_node(&mut mem(), this, controller::vtable::TIME_SCALE_OFFSET);
    }
    /// `Phyre_PAnimation_PAnimationDataSource_dtor_PAnimationDataSource` (0x00523d60).
    Phyre_PAnimation_PAnimationDataSource_dtor_PAnimationDataSource @ 0x0052_3d60 => extern "thiscall" fn dtor_523d60(this: u32) {
        controller::bridge::data_source_dtor(&mut mem(), this);
    }
    /// `Phyre_PAnimation_PAnimationController_dtor_PAnimationController` (0x005252c0).
    Phyre_PAnimation_PAnimationController_dtor_PAnimationController @ 0x0052_52c0 => extern "thiscall" fn dtor_5252c0(this: u32) {
        controller::bridge::dtor_over_data_source(&mut mem(), this, controller::vtable::CONTROLLER);
    }
    /// `Phyre_PAnimation_PAnimationTargetBlenderController_dtor_PAnimationTargetBlenderController` (0x00530e40).
    Phyre_PAnimation_PAnimationTargetBlenderController_dtor_PAnimationTargetBlenderController @ 0x0053_0e40 => extern "thiscall" fn dtor_530e40(this: u32) {
        controller::bridge::dtor_over_data_source(&mut mem(), this, controller::vtable::TARGET_BLENDER);
    }
    /// `Phyre_PAnimation_PTimeController_scalar_deleting_dtor` (0x00521b80).
    Phyre_PAnimation_PTimeController_scalar_deleting_dtor @ 0x0052_1b80 => extern "thiscall" fn delete_521b80(this: u32, flags: u32) -> u32 {
        controller::bridge::deleting_dtor(&mut mem(), this, flags, controller::bridge::Dtor::Node)
    }
    /// `Phyre_PAnimation_PTimeIntervalController_scalar_deleting_dtor` (0x005226d0).
    Phyre_PAnimation_PTimeIntervalController_scalar_deleting_dtor @ 0x0052_26d0 => extern "thiscall" fn delete_5226d0(this: u32, flags: u32) -> u32 {
        controller::bridge::deleting_dtor(&mut mem(), this, flags, controller::bridge::Dtor::OverNode(controller::vtable::TIME_INTERVAL))
    }
    /// `Phyre_PAnimation_PTimeScaleOffsetController_scalar_deleting_dtor` (0x00523310).
    Phyre_PAnimation_PTimeScaleOffsetController_scalar_deleting_dtor @ 0x0052_3310 => extern "thiscall" fn delete_523310(this: u32, flags: u32) -> u32 {
        controller::bridge::deleting_dtor(&mut mem(), this, flags, controller::bridge::Dtor::OverNode(controller::vtable::TIME_SCALE_OFFSET))
    }
    /// `Phyre_PAnimation_PAnimationEventController_scalar_deleting_dtor` (0x00524740).
    Phyre_PAnimation_PAnimationEventController_scalar_deleting_dtor @ 0x0052_4740 => extern "thiscall" fn delete_524740(this: u32, flags: u32) -> u32 {
        controller::bridge::deleting_dtor(&mut mem(), this, flags, controller::bridge::Dtor::OverDataSource(controller::vtable::EVENT_CONTROLLER))
    }
    /// `Phyre_PAnimation_PAnimationController_scalar_deleting_dtor` (0x00525390).
    Phyre_PAnimation_PAnimationController_scalar_deleting_dtor @ 0x0052_5390 => extern "thiscall" fn delete_525390(this: u32, flags: u32) -> u32 {
        controller::bridge::deleting_dtor(&mut mem(), this, flags, controller::bridge::Dtor::OverDataSource(controller::vtable::CONTROLLER))
    }
    /// `Phyre_PAnimation_PAnimationSlotFilter_scalar_deleting_dtor` (0x00528180).
    Phyre_PAnimation_PAnimationSlotFilter_scalar_deleting_dtor @ 0x0052_8180 => extern "thiscall" fn delete_528180(this: u32, flags: u32) -> u32 {
        controller::bridge::deleting_dtor(&mut mem(), this, flags, controller::bridge::Dtor::Original(controller::dtor::SLOT_FILTER))
    }
    /// `Phyre_PAnimation_PAnimationSpuTargetBlenderController_scalar_deleting_dtor` (0x00530180).
    Phyre_PAnimation_PAnimationSpuTargetBlenderController_scalar_deleting_dtor @ 0x0053_0180 => extern "thiscall" fn delete_530180(this: u32, flags: u32) -> u32 {
        controller::bridge::deleting_dtor(&mut mem(), this, flags, controller::bridge::Dtor::OverDataSource(controller::vtable::SPU_TARGET_BLENDER))
    }
    /// `Phyre_PAnimation_PAnimationTargetBlenderController_scalar_deleting_dtor` (0x00530f20).
    Phyre_PAnimation_PAnimationTargetBlenderController_scalar_deleting_dtor @ 0x0053_0f20 => extern "thiscall" fn delete_530f20(this: u32, flags: u32) -> u32 {
        controller::bridge::deleting_dtor(&mut mem(), this, flags, controller::bridge::Dtor::OverDataSource(controller::vtable::TARGET_BLENDER))
    }
    /// `Phyre_PAnimation_PAnimationNetworkInstance_scalar_deleting_dtor` (0x005357d0).
    Phyre_PAnimation_PAnimationNetworkInstance_scalar_deleting_dtor @ 0x0053_57d0 => extern "thiscall" fn delete_5357d0(this: u32, flags: u32) -> u32 {
        controller::bridge::deleting_dtor(&mut mem(), this, flags, controller::bridge::Dtor::Original(controller::dtor::NETWORK_INSTANCE))
    }
    /// `Phyre_PAnimation_PAnimatableComponent_scalar_deleting_dtor` (0x0053f3d0).
    Phyre_PAnimation_PAnimatableComponent_scalar_deleting_dtor @ 0x0053_f3d0 => extern "thiscall" fn delete_53f3d0(this: u32, flags: u32) -> u32 {
        controller::bridge::deleting_dtor(&mut mem(), this, flags, controller::bridge::Dtor::Original(controller::dtor::ANIMATABLE_COMPONENT))
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PTimeController__constructInstance` (0x00522040).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PTimeController__constructInstance @ 0x0052_2040 => extern "stdcall" fn construct_522040(p: u32) -> bool {
        controller::bridge::construct(&mut mem(), p, controller::bridge::time_controller_ctor)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PTimeIntervalController__constructInstance` (0x00522bf0).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PTimeIntervalController__constructInstance @ 0x0052_2bf0 => extern "stdcall" fn construct_522bf0(p: u32) -> bool {
        controller::bridge::construct(&mut mem(), p, controller::bridge::time_interval_ctor)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PTimeScaleOffsetController__constructInstance` (0x005237a0).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PTimeScaleOffsetController__constructInstance @ 0x0052_37a0 => extern "stdcall" fn construct_5237a0(p: u32) -> bool {
        controller::bridge::construct(&mut mem(), p, controller::bridge::time_scale_offset_ctor)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationEventController__constructInstance` (0x00524c00).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationEventController__constructInstance @ 0x0052_4c00 => extern "stdcall" fn construct_524c00(p: u32) -> bool {
        controller::bridge::construct(&mut mem(), p, controller::bridge::event_controller_ctor)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationController__constructInstance` (0x00526130).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationController__constructInstance @ 0x0052_6130 => extern "stdcall" fn construct_526130(p: u32) -> bool {
        controller::bridge::construct(&mut mem(), p, controller::bridge::controller_ctor)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationWeightedBlenderController__constructInstance` (0x0052e180).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationWeightedBlenderController__constructInstance @ 0x0052_e180 => extern "stdcall" fn construct_52e180(p: u32) -> bool {
        controller::bridge::construct(&mut mem(), p, controller::bridge::weighted_blender_ctor)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationAdditiveBlenderController__constructInstance` (0x0052f670).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationAdditiveBlenderController__constructInstance @ 0x0052_f670 => extern "stdcall" fn construct_52f670(p: u32) -> bool {
        controller::bridge::construct(&mut mem(), p, controller::bridge::additive_blender_ctor)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationSpuTargetBlenderController__constructInstance` (0x005305c0).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationSpuTargetBlenderController__constructInstance @ 0x0053_05c0 => extern "stdcall" fn construct_5305c0(p: u32) -> bool {
        controller::bridge::construct(&mut mem(), p, controller::bridge::spu_target_blender_ctor)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationTargetBlenderController__constructInstance` (0x00531420).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationTargetBlenderController__constructInstance @ 0x0053_1420 => extern "stdcall" fn construct_531420(p: u32) -> bool {
        controller::bridge::construct(&mut mem(), p, controller::bridge::target_blender_ctor)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationSlotFilter__constructInstance` (0x0052a060).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationSlotFilter__constructInstance @ 0x0052_a060 => extern "stdcall" fn construct_52a060(p: u32) -> bool {
        controller::bridge::construct(&mut mem(), p, controller::bridge::slot_filter_ctor)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationChannelTarget__constructInstance` (0x0050c510).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationChannelTarget__constructInstance @ 0x0050_c510 => extern "stdcall" fn construct_50c510(p: u32) -> bool {
        descriptor::bridge::construct_described(&mut mem(), p, 6)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationChannelBase__constructInstance` (0x0050d6b0).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationChannelBase__constructInstance @ 0x0050_d6b0 => extern "stdcall" fn construct_50d6b0(p: u32) -> bool {
        descriptor::bridge::construct_described(&mut mem(), p, 8)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationChannel__constructInstance` (0x0050e5e0).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationChannel__constructInstance @ 0x0050_e5e0 => extern "stdcall" fn construct_50e5e0(p: u32) -> bool {
        descriptor::bridge::construct_channel(&mut mem(), p)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationConstantChannel__constructInstance` (0x0050f6b0).
    Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PAnimationConstantChannel__constructInstance @ 0x0050_f6b0 => extern "stdcall" fn construct_50f6b0(p: u32) -> bool {
        descriptor::bridge::construct_channel(&mut mem(), p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationChannel__destructInstance` (0x0050e650).
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationChannel__destructInstance @ 0x0050_e650 => extern "stdcall" fn destroy_50e650(p: u32) -> u32 {
        descriptor::bridge::destroy_channel(&mut mem(), p)
    }
    /// `Phyre_PAnimation_PAnimationDataSourceListEntry_PAnimationDataSourceListEntry` (0x00534360).
    Phyre_PAnimation_PAnimationDataSourceListEntry_PAnimationDataSourceListEntry @ 0x0053_4360 => extern "thiscall" fn list_entry_ctor(this: u32) -> u32 {
        network::bridge::list_entry_ctor(&mut mem(), this)
    }
    /// `Phyre_PAnimation_copyWords_005281e0` (0x005281e0).
    Phyre_PAnimation_copyWords_005281e0 @ 0x0052_81e0 => extern "C" fn copy_words(n: u32, src: u32) -> u32 {
        array::bridge::copy_words(&mut mem(), n, src)
    }
    /// `Phyre_PAnimation_PAnimationBlenderController_dtor_PAnimationBlenderController` (0x0052b8e0).
    Phyre_PAnimation_PAnimationBlenderController_dtor_PAnimationBlenderController @ 0x0052_b8e0 => extern "thiscall" fn dtor_52b8e0(this: u32) {
        controller::bridge::blender_dtor(&mut mem(), this);
    }
    /// `Phyre_PAnimation_PAnimationWeightedBlenderController_dtor_PAnimationWeightedBlenderController` (0x0052d6b0).
    Phyre_PAnimation_PAnimationWeightedBlenderController_dtor_PAnimationWeightedBlenderController @ 0x0052_d6b0 => extern "thiscall" fn dtor_52d6b0(this: u32) {
        controller::bridge::blender_subclass_dtor(&mut mem(), this, controller::vtable::WEIGHTED_BLENDER);
    }
    /// `Phyre_PAnimation_PAnimationSlotFilter_dtor_PAnimationSlotFilter` (0x00527ba0).
    Phyre_PAnimation_PAnimationSlotFilter_dtor_PAnimationSlotFilter @ 0x0052_7ba0 => extern "thiscall" fn dtor_527ba0(this: u32) {
        controller::bridge::slot_filter_dtor(&mut mem(), this);
    }
    /// `Phyre_PAnimation_PAnimationSet_dtor_PAnimationSet` (0x00519380).
    Phyre_PAnimation_PAnimationSet_dtor_PAnimationSet @ 0x0051_9380 => extern "thiscall" fn dtor_519380(this: u32) {
        controller::bridge::animation_set_dtor(&mut mem(), this);
    }
    /// `Phyre_PAnimation_PAnimationWeightedBlenderController_scalar_deleting_dtor` (0x0052d940).
    Phyre_PAnimation_PAnimationWeightedBlenderController_scalar_deleting_dtor @ 0x0052_d940 => extern "thiscall" fn delete_52d940(this: u32, flags: u32) -> u32 {
        controller::bridge::blender_subclass_deleting_dtor(&mut mem(), this, flags, controller::vtable::WEIGHTED_BLENDER)
    }
    /// `Phyre_PAnimation_PAnimationAdditiveBlenderController_scalar_deleting_dtor` (0x0052edd0).
    Phyre_PAnimation_PAnimationAdditiveBlenderController_scalar_deleting_dtor @ 0x0052_edd0 => extern "thiscall" fn delete_52edd0(this: u32, flags: u32) -> u32 {
        controller::bridge::blender_subclass_deleting_dtor(&mut mem(), this, flags, controller::vtable::ADDITIVE_BLENDER)
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_float_4__PClassDescriptorAbstract` (0x00509630).
    Phyre_PClassDescriptorAbstract_Phyre_PArray_float_4__PClassDescriptorAbstract @ 0x0050_9630 => extern "thiscall" fn abstract_509630(this: u32) -> u32 {
        descriptor::bridge::abstract_descriptor_ctor(&mut mem(), this, &descriptor::AbstractDescriptor { name: descriptor::Name::Global(0x00ca_9bfc), size: 0x8, vtable: 0x00b2_579c, buffers: [0x00ca_9c00, 0x00ca_9c10, 0x00ca_9c20] })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget_4__PClassDescriptorAbstract` (0x00518cd0).
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget_4__PClassDescriptorAbstract @ 0x0051_8cd0 => extern "thiscall" fn abstract_518cd0(this: u32) -> u32 {
        descriptor::bridge::abstract_descriptor_ctor(&mut mem(), this, &descriptor::AbstractDescriptor { name: descriptor::Name::Global(0x00ca_b43c), size: 0x8, vtable: 0x00b2_759c, buffers: [0x00ca_b490, 0x00ca_b4a0, 0x00ca_b4b0] })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PAnimation_PAnimationSlotListIndex_4__PClassDescriptorAbstract` (0x00518d70).
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PAnimation_PAnimationSlotListIndex_4__PClassDescriptorAbstract @ 0x0051_8d70 => extern "thiscall" fn abstract_518d70(this: u32) -> u32 {
        descriptor::bridge::abstract_descriptor_ctor(&mut mem(), this, &descriptor::AbstractDescriptor { name: descriptor::Name::Global(0x00ca_b438), size: 0x8, vtable: 0x00b2_7494, buffers: [0x00ca_b460, 0x00ca_b470, 0x00ca_b480] })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PSharray_Phyre_PAnimation_PAnimationClip__PClassDescriptorAbstract` (0x00518e10).
    Phyre_PClassDescriptorAbstract_Phyre_PSharray_Phyre_PAnimation_PAnimationClip__PClassDescriptorAbstract @ 0x0051_8e10 => extern "thiscall" fn abstract_518e10(this: u32) -> u32 {
        descriptor::bridge::abstract_descriptor_ctor(&mut mem(), this, &descriptor::AbstractDescriptor { name: descriptor::Name::Global(0x00ca_b440), size: 0x8, vtable: 0x00b2_76a4, buffers: [0x00ca_b4c0, 0x00ca_b4d0, 0x00ca_b4e0] })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PAnimation_PAnimationEvent_4__PClassDescriptorAbstract` (0x0051ec40).
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PAnimation_PAnimationEvent_4__PClassDescriptorAbstract @ 0x0051_ec40 => extern "thiscall" fn abstract_51ec40(this: u32) -> u32 {
        descriptor::bridge::abstract_descriptor_ctor(&mut mem(), this, &descriptor::AbstractDescriptor { name: descriptor::Name::Global(0x00ca_b828), size: 0x8, vtable: 0x00b2_7eec, buffers: [0x00ca_b830, 0x00ca_b840, 0x00ca_b850] })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_unsigned_int_4__PClassDescriptorAbstract` (0x005275f0).
    Phyre_PClassDescriptorAbstract_Phyre_PArray_unsigned_int_4__PClassDescriptorAbstract @ 0x0052_75f0 => extern "thiscall" fn abstract_5275f0(this: u32) -> u32 {
        descriptor::bridge::abstract_descriptor_ctor(&mut mem(), this, &descriptor::AbstractDescriptor { name: descriptor::Name::Global(0x00ca_c5b4), size: 0x8, vtable: 0x00b2_9394, buffers: [0x00ca_c5f0, 0x00ca_c600, 0x00ca_c610] })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PAnimation_PAnimationSlotFilterDeferredLoad_4__PClassDescriptorAbstract` (0x00527690).
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PAnimation_PAnimationSlotFilterDeferredLoad_4__PClassDescriptorAbstract @ 0x0052_7690 => extern "thiscall" fn abstract_527690(this: u32) -> u32 {
        descriptor::bridge::abstract_descriptor_ctor(&mut mem(), this, &descriptor::AbstractDescriptor { name: descriptor::Name::Global(0x00ca_c5b0), size: 0x8, vtable: 0x00b2_928c, buffers: [0x00ca_c5c0, 0x00ca_c5d0, 0x00ca_c5e0] })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_4__PClassDescriptorAbstract` (0x0052b670).
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_4__PClassDescriptorAbstract @ 0x0052_b670 => extern "thiscall" fn abstract_52b670(this: u32) -> u32 {
        descriptor::bridge::abstract_descriptor_ctor(&mut mem(), this, &descriptor::AbstractDescriptor { name: descriptor::Name::Global(0x00ca_c868), size: 0x8, vtable: 0x00b2_983c, buffers: [0x00ca_c870, 0x00ca_c880, 0x00ca_c890] })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PAnimation_PAnimationDataSourceListEntry_4__PClassDescriptorAbstract` (0x00533ae0).
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PAnimation_PAnimationDataSourceListEntry_4__PClassDescriptorAbstract @ 0x0053_3ae0 => extern "thiscall" fn abstract_533ae0(this: u32) -> u32 {
        descriptor::bridge::abstract_descriptor_ctor(&mut mem(), this, &descriptor::AbstractDescriptor { name: descriptor::Name::Global(0x00ca_d8e8), size: 0x8, vtable: 0x00b2_af04, buffers: [0x00ca_d980, 0x00ca_d990, 0x00ca_d9a0] })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PAnimation_PAnimationNetworkInstanceTarget_4__PClassDescriptorAbstract` (0x00533b80).
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PAnimation_PAnimationNetworkInstanceTarget_4__PClassDescriptorAbstract @ 0x0053_3b80 => extern "thiscall" fn abstract_533b80(this: u32) -> u32 {
        descriptor::bridge::abstract_descriptor_ctor(&mut mem(), this, &descriptor::AbstractDescriptor { name: descriptor::Name::Global(0x00ca_d8e4), size: 0x8, vtable: 0x00b2_adfc, buffers: [0x00ca_d950, 0x00ca_d960, 0x00ca_d970] })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PAnimation_PAnimationSlotArray_4__PClassDescriptorAbstract` (0x00533c20).
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PAnimation_PAnimationSlotArray_4__PClassDescriptorAbstract @ 0x0053_3c20 => extern "thiscall" fn abstract_533c20(this: u32) -> u32 {
        descriptor::bridge::abstract_descriptor_ctor(&mut mem(), this, &descriptor::AbstractDescriptor { name: descriptor::Name::Global(0x00ca_d8dc), size: 0x8, vtable: 0x00b2_abec, buffers: [0x00ca_d8f0, 0x00ca_d900, 0x00ca_d910] })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PQWord_16__PClassDescriptorAbstract` (0x00533cc0).
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PQWord_16__PClassDescriptorAbstract @ 0x0053_3cc0 => extern "thiscall" fn abstract_533cc0(this: u32) -> u32 {
        descriptor::bridge::abstract_descriptor_ctor(&mut mem(), this, &descriptor::AbstractDescriptor { name: descriptor::Name::Global(0x00ca_d8e0), size: 0x8, vtable: 0x00b2_acf4, buffers: [0x00ca_d920, 0x00ca_d930, 0x00ca_d940] })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PAnimation_PAnimationDataSourceListEntry__PClassDescriptorAbstract` (0x00533d60).
    Phyre_PClassDescriptorAbstract_Phyre_PAnimation_PAnimationDataSourceListEntry__PClassDescriptorAbstract @ 0x0053_3d60 => extern "thiscall" fn abstract_533d60(this: u32) -> u32 {
        descriptor::bridge::abstract_descriptor_ctor(&mut mem(), this, &descriptor::AbstractDescriptor { name: descriptor::Name::Literal(0x00b2_a444), size: 0x94, vtable: 0x00b2_a3bc, buffers: [0x00ca_d5e0, 0x00ca_d680, 0x00ca_d720] })
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PAnimation_PBlendableAnimationSource_4__PClassDescriptorAbstract` (0x0053e220).
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PAnimation_PBlendableAnimationSource_4__PClassDescriptorAbstract @ 0x0053_e220 => extern "thiscall" fn abstract_53e220(this: u32) -> u32 {
        descriptor::bridge::abstract_descriptor_ctor(&mut mem(), this, &descriptor::AbstractDescriptor { name: descriptor::Name::Global(0x00ca_e4b4), size: 0x8, vtable: 0x00b2_babc, buffers: [0x00ca_e520, 0x00ca_e530, 0x00ca_e540] })
    }
}
