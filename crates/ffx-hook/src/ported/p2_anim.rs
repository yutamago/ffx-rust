//! `PhyreEngine`'s `PAnimation` code (`ffx_panim`, M4 port round 1, list
//! `p2-anim`).
//!
//! The class-descriptor callbacks of the animation classes (the
//! `PClassDescriptor*` vtables' construct, destruct and fix-after-loading
//! slots, `ffx_panim::descriptor`). They run on the loading threads as
//! well as the main thread; none keeps state.

use super::{mem, ported};
use ffx_panim::descriptor;

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
}
