//! M5 stage 1, list `phyre-anim` (`ffx_panim::m5p_anim*`): `PhyreEngine`'s
//! `PAnimation` code outside M4's coverage. Template copies take their
//! constants from the crate's tables (`m5p_anim_table.rs.in`), so each copy
//! runs its own original in the tests (`crates/ffx-oracle/tests/m5p_anim.rs`).
//!
//! Threads: animation batches run on the worker threads, the descriptors'
//! slots where clusters load, the destructors at exit or in unwind funclets.
//! No shim keeps state; each works on the game's memory through [`mem`].

use super::{mem, ported};
use ffx_panim::array;
use ffx_panim::m5p_anim_small as sm;

ported! {
    /// `Phyre_PAnimation_getElement8_00432f50`: word `i` of the array at `[this + 8]`.
    Phyre_PAnimation_getElement8_00432f50 @ 0x0043_2f50 => extern "thiscall" fn vt_00432f50(this: u32, i: u32) -> u32 {
        sm::bridge::element_8(&mem(), this, i)
    }
    /// `Phyre_PRendering_getElement34_00432f70`: the address of element `i` (`0x34` bytes) of the array at `[this + 0x10]`.
    Phyre_PRendering_getElement34_00432f70 @ 0x0043_2f70 => extern "thiscall" fn vt_00432f70(this: u32, i: u32) -> u32 {
        sm::bridge::element_34(&mem(), this, i)
    }
    /// `Phyre_PAssetReference_getPair1c`: `out[0..2] = [this + 0x1c], [this + 0x20]`; `out`.
    Phyre_PAssetReference_getPair1c @ 0x0045_de80 => extern "thiscall" fn vt_0045de80(this: u32, out: u32) -> u32 {
        sm::bridge::copy_pair(&mut mem(), this, out)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PGeometry_PSkinBoneRemap_2__swapEndiannessStatic0`: builds the static descriptor `0x00c9_9590` on first use, then `swapEndiannessOfObject(p, 0)`; its `eax`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PGeometry_PSkinBoneRemap_2__swapEndiannessStatic0 @ 0x0048_68d0 => extern "C" fn sl_004868d0(p: u32) -> u32 {
        sm::bridge::swap_lazy(&mut mem(), &const { sm::lazy_of(0x0048_68d0) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PGeometry_PMeshSegment__dtor`: its vtable `0x00b14ca4`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PMeshSegment__dtor @ 0x0048_7280 => extern "thiscall" fn td_00487280(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0048_7280) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PGeometry_PMeshSegmentBase__dtor`: its vtable `0x00b14bbc`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PMeshSegmentBase__dtor @ 0x0048_7290 => extern "thiscall" fn td_00487290(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0048_7290) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PGeometry_PSkinBoneRemap__dtor`: its vtable `0x00b14ad4`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PSkinBoneRemap__dtor @ 0x0048_72a0 => extern "thiscall" fn td_004872a0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0048_72a0) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PSamplerStateBase__dtor`: its vtable `0x00b1d59c`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PSamplerStateBase__dtor @ 0x004b_e900 => extern "thiscall" fn td_004be900(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x004b_e900) }, this)
    }
    /// `Phyre_PScene_PNode_assignName50`: `PString::Assign(text)` on `this + 0x50` (tail jump); its `eax`.
    Phyre_PScene_PNode_assignName50 @ 0x0050_74c0 => extern "thiscall" fn vt_005074c0(this: u32, text: u32) -> u32 {
        sm::bridge::assign_string(&mut mem(), this, 0x50, text)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSlotArray__dtor`: its vtable `0x00b25244`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSlotArray__dtor @ 0x0050_82d0 => extern "thiscall" fn td_005082d0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0050_82d0) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSlotListIndex__dtor`: its vtable `0x00b25154`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSlotListIndex__dtor @ 0x0050_82e0 => extern "thiscall" fn td_005082e0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0050_82e0) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationChannelTarget__dtor`: its vtable `0x00b2591c`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationChannelTarget__dtor @ 0x0050_b6c0 => extern "thiscall" fn td_0050b6c0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0050_b6c0) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationChannel__dtor`: its vtable `0x00b25eb4`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationChannel__dtor @ 0x0050_dda0 => extern "thiscall" fn td_0050dda0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0050_dda0) }, this)
    }
    /// `Phyre_PAnimation_PAnimationChannel_getBatchSortKey`: `[this + 0x20]`, or'ed with `[[this + 0x1c] + 0xc] << 2` when `+0x1c` is set.
    Phyre_PAnimation_PAnimationChannel_getBatchSortKey @ 0x0050_e730 => extern "thiscall" fn vt_0050e730(this: u32) -> u32 {
        sm::bridge::batch_sort_key(&mem(), this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationConstantChannel__dtor`: its vtable `0x00b2608c`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationConstantChannel__dtor @ 0x0050_f200 => extern "thiscall" fn td_0050f200(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0050_f200) }, this)
    }
    /// `Phyre_PArray_dtor_00510150`: `PArray<T>::~PArray`: `PhyreFree` of owned, non-null elements, then both words zero.
    Phyre_PArray_dtor_00510150 @ 0x0051_0150 => extern "thiscall" fn pr_00510150(this: u32) -> u32 {
        sm::bridge::release(&mut mem(), this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationClipBinding__dtor`: its vtable `0x00b2625c`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationClipBinding__dtor @ 0x0051_01d0 => extern "thiscall" fn td_005101d0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0051_01d0) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationClipBinding_PAnimationClipBindingChannelMap__dtor`: its vtable `0x00b2637c`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationClipBinding_PAnimationClipBindingChannelMap__dtor @ 0x0051_01e0 => extern "thiscall" fn td_005101e0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0051_01e0) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationClipBinding_PAnimationClipBindingDataBlockCache__dtor`: its vtable `0x00b26454`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationClipBinding_PAnimationClipBindingDataBlockCache__dtor @ 0x0051_01f0 => extern "thiscall" fn td_005101f0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0051_01f0) }, this)
    }
    /// `Phyre_PAnimation_compareKey14_00510f60`: orders two records by their word at `+0x14`: -1, 0, 1.
    Phyre_PAnimation_compareKey14_00510f60 @ 0x0051_0f60 => extern "C" fn vt_00510f60(a: u32, b: u32) -> u32 {
        sm::bridge::compare_modules(&mem(), a, b)
    }
    /// `Phyre_PAnimation_PAnimationClip_getElement8`: word `i` of the array at `[this + 8]`.
    Phyre_PAnimation_PAnimationClip_getElement8 @ 0x0051_11c0 => extern "thiscall" fn vt_005111c0(this: u32, i: u32) -> u32 {
        sm::bridge::element_8(&mem(), this, i)
    }
    /// `Phyre_PAnimation_PAnimationClip_assignName1c`: `PString::Assign(text)` on `this + 0x1c` (tail jump); its `eax`.
    Phyre_PAnimation_PAnimationClip_assignName1c @ 0x0051_7220 => extern "thiscall" fn vt_00517220(this: u32, text: u32) -> u32 {
        sm::bridge::assign_string(&mut mem(), this, 0x1c, text)
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget_dtor`: `PArray<PAnimationChannelTarget>::~PArray`: owned elements to `Phyre_PAnimation_PAnimationChannelTarget_freeArray`, both words zero.
    Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget_dtor @ 0x0051_9180 => extern "thiscall" fn pw_00519180(this: u32) -> u32 {
        sm::bridge::release_with(&mut mem(), const { sm::word_of(sm::RELEASES_WITH, 0x0051_9180) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget_4__dtor`: its vtable `0x00b27554`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget_4__dtor @ 0x0051_92d0 => extern "thiscall" fn td_005192d0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0051_92d0) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationSlotListIndex_4__dtor`: its vtable `0x00b2744c`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationSlotListIndex_4__dtor @ 0x0051_92e0 => extern "thiscall" fn td_005192e0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0051_92e0) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PAnimation_PAnimationClip__dtor`: its vtable `0x00b2765c`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PAnimation_PAnimationClip__dtor @ 0x0051_92f0 => extern "thiscall" fn td_005192f0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0051_92f0) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSet__dtor`: its vtable `0x00b2714c`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSet__dtor @ 0x0051_9300 => extern "thiscall" fn td_00519300(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0051_9300) }, this)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationEvent_4__swapEndiannessStatic1`: builds the static descriptor `0x00ca_b6a8` on first use, then `swapEndiannessOfObject(p, 1)`; its `eax`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationEvent_4__swapEndiannessStatic1 @ 0x0051_e900 => extern "C" fn sl_0051e900(p: u32) -> u32 {
        sm::bridge::swap_lazy(&mut mem(), &const { sm::lazy_of(0x0051_e900) }, p)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationEvent_4__swapEndiannessStatic0`: builds the static descriptor `0x00ca_b6a8` on first use, then `swapEndiannessOfObject(p, 0)`; its `eax`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationEvent_4__swapEndiannessStatic0 @ 0x0051_e9d0 => extern "C" fn sl_0051e9d0(p: u32) -> u32 {
        sm::bridge::swap_lazy(&mut mem(), &const { sm::lazy_of(0x0051_e9d0) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationEvent_4__dtor`: its vtable `0x00b27ea4`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationEvent_4__dtor @ 0x0051_f000 => extern "thiscall" fn td_0051f000(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0051_f000) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationEvent__dtor`: its vtable `0x00b27acc`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationEvent__dtor @ 0x0051_f010 => extern "thiscall" fn td_0051f010(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0051_f010) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationE6entList__dtor`: its vtable `0x00b27bb4`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationE6entList__dtor @ 0x0051_f020 => extern "thiscall" fn td_0051f020(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0051_f020) }, this)
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationEvent_setBorrowed`: `PArray<T>::setBorrowed(els, count)` (`Phyre_PArray_float_setBorrowed`'s shape); 0.
    Phyre_PArray_Phyre_PAnimation_PAnimationEvent_setBorrowed @ 0x0052_0e00 => extern "thiscall" fn sb_00520e00(this: u32, els: u32, count: u32) -> u32 {
        const { sm::listed(sm::SET_BORROWED, 0x0052_0e00) };
        array::bridge::set_borrowed(&mut mem(), this, els, count, array::Old::Free)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationHierarchyNode__dtor`: its vtable `0x00b280c4`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationHierarchyNode__dtor @ 0x0052_1210 => extern "thiscall" fn td_00521210(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0052_1210) }, this)
    }
    /// `Phyre_PAnimation_PAnimationHierarchyVisitContext_dtor`: `PAnimationHierarchyVisitContext::~PAnimationHierarchyVisitContext`: its vtable; `eax` stays the caller's.
    Phyre_PAnimation_PAnimationHierarchyVisitContext_dtor @ 0x0052_1230 => extern "thiscall" fn sv_00521230(this: u32) {
        sm::bridge::set_vtable(&mut mem(), this, sm::VISIT_CONTEXT_VTABLE);
    }
    /// `Phyre_PAnimation_PAnimationHierarchyNode_scalarDeletingDtor`: scalar deleting destructor: the base destructor, the free when `flags & 1`; `this`.
    Phyre_PAnimation_PAnimationHierarchyNode_scalarDeletingDtor @ 0x0052_12a0 => extern "thiscall" fn dd_005212a0(this: u32, flags: u32) -> u32 {
        sm::bridge::delete(&mut mem(), const { sm::delete_of(0x0052_12a0) }, this, flags)
    }
    /// `Phyre_PAnimation_PAnimationHierarchyVisitContext_scalarDeletingDtor`: scalar deleting destructor: the base destructor, the free when `flags & 1`; `this`.
    Phyre_PAnimation_PAnimationHierarchyVisitContext_scalarDeletingDtor @ 0x0052_12d0 => extern "thiscall" fn dd_005212d0(this: u32, flags: u32) -> u32 {
        sm::bridge::delete(&mut mem(), const { sm::delete_of(0x0052_12d0) }, this, flags)
    }
    /// `Phyre_PAnimation_PAnimationHierarchyVisitContext_visitBegin`: a visit callback: `al = 1`.
    Phyre_PAnimation_PAnimationHierarchyVisitContext_visitBegin @ 0x0052_1730 => extern "thiscall" fn vt_00521730(_this: u32, _a: u32, _b: u32) -> u8 {
        1
    }
    /// `Phyre_PAnimation_PAnimationHierarchyVisitContext_visitEnd`: a visit callback: `al = 1`.
    Phyre_PAnimation_PAnimationHierarchyVisitContext_visitEnd @ 0x0052_1740 => extern "thiscall" fn vt_00521740(_this: u32, _a: u32, _b: u32) -> u8 {
        1
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PTimeController__dtor`: its vtable `0x00b281ec`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PTimeController__dtor @ 0x0052_1a90 => extern "thiscall" fn td_00521a90(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0052_1a90) }, this)
    }
    /// `Phyre_PAnimation_PTimeController_getTime`: the constant time at `0x00cab918` in `ST0`.
    Phyre_PAnimation_PTimeController_getTime @ 0x0052_2110 => extern "thiscall" fn vt_00522110(_this: u32) -> f32 {
        sm::bridge::time_constant(&mem())
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PTimeIntervalController__dtor`: its vtable `0x00b283cc`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PTimeIntervalController__dtor @ 0x0052_25f0 => extern "thiscall" fn td_005225f0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0052_25f0) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PTimeScaleOffsetController__dtor`: its vtable `0x00b285e4`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PTimeScaleOffsetController__dtor @ 0x0052_3230 => extern "thiscall" fn td_00523230(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0052_3230) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationDataSource__dtor`: its vtable `0x00b287e4`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationDataSource__dtor @ 0x0052_3d50 => extern "thiscall" fn td_00523d50(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0052_3d50) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationEventController__dtor`: its vtable `0x00b2893c`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationEventController__dtor @ 0x0052_4640 => extern "thiscall" fn td_00524640(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0052_4640) }, this)
    }
    /// `Phyre_PAnimation_PAnimationEventController_pushEvent`: the script push of `event + 4` (null for a null event; tail jump); its `eax`.
    Phyre_PAnimation_PAnimationEventController_pushEvent @ 0x0052_4bb0 => extern "C" fn vt_00524bb0(l: u32, event: u32, class: u32) -> u32 {
        sm::bridge::push_event(&mut mem(), l, event, class)
    }
    /// `Phyre_PAnimation_PAnimationEventController_vslot7`: a constant virtual method: 1.
    Phyre_PAnimation_PAnimationEventController_vslot7 @ 0x0052_4cb0 => extern "thiscall" fn kc_00524cb0(_this: u32) -> u32 {
        sm::bridge::constant(&mem(), const { sm::constant_of(0x0052_4cb0) })
    }
    /// `Phyre_PAnimation_PAnimationEventController_vslot5`: a constant virtual method: 0xc.
    Phyre_PAnimation_PAnimationEventController_vslot5 @ 0x0052_4cd0 => extern "thiscall" fn kc_00524cd0(_this: u32) -> u32 {
        sm::bridge::constant(&mem(), const { sm::constant_of(0x0052_4cd0) })
    }
    /// `Phyre_PAnimation_PAnimationEventController_getBatchFunction`: returns the batch function's address (rebased).
    Phyre_PAnimation_PAnimationEventController_getBatchFunction @ 0x0052_4d10 => extern "thiscall" fn kc_00524d10(_this: u32) -> u32 {
        sm::bridge::constant(&mem(), const { sm::constant_of(0x0052_4d10) })
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationController__dtor`: its vtable `0x00b28b74`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationController__dtor @ 0x0052_52b0 => extern "thiscall" fn td_005252b0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0052_52b0) }, this)
    }
    /// `Phyre_PAnimation_PAnimationController_getOutputSize`: `([[this + 8] + 0x10] & 0x7fffffff) * 4`, 0 without `[this + 8]`.
    Phyre_PAnimation_PAnimationController_getOutputSize @ 0x0052_6250 => extern "thiscall" fn vt_00526250(this: u32) -> u32 {
        sm::bridge::source_bytes(&mem(), this)
    }
    /// `Phyre_PAnimation_PAnimationController_getBatchFunction`: returns the batch function's address (rebased).
    Phyre_PAnimation_PAnimationController_getBatchFunction @ 0x0052_6390 => extern "thiscall" fn kc_00526390(_this: u32) -> u32 {
        sm::bridge::constant(&mem(), const { sm::constant_of(0x0052_6390) })
    }
    /// `Phyre_PAnimation_PAnimationController_setTimeController`: `[this + 0x10] = tc`; 0.
    Phyre_PAnimation_PAnimationController_setTimeController @ 0x0052_65f0 => extern "thiscall" fn vt_005265f0(this: u32, tc: u32) -> u32 {
        sm::bridge::set_time_controller(&mut mem(), this, tc)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_unsigned_int_4__dtor`: its vtable `0x00b2934c`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PArray_unsigned_int_4__dtor @ 0x0052_7b60 => extern "thiscall" fn td_00527b60(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0052_7b60) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationSlotFilterDeferredLoad_4__dtor`: its vtable `0x00b29244`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationSlotFilterDeferredLoad_4__dtor @ 0x0052_7b70 => extern "thiscall" fn td_00527b70(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0052_7b70) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSlotFilter__dtor`: its vtable `0x00b28e7c`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSlotFilter__dtor @ 0x0052_7b80 => extern "thiscall" fn td_00527b80(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0052_7b80) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSlotFilterDeferredLoad__dtor`: its vtable `0x00b28d84`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSlotFilterDeferredLoad__dtor @ 0x0052_7b90 => extern "thiscall" fn td_00527b90(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0052_7b90) }, this)
    }
    /// `Phyre_PAnimation_PAnimationSlotFilter_getScratchSize`: per-batch scratch bytes: `((n * 4 + 0x13) & !0xf) + 0x10`, `n` the source's count.
    Phyre_PAnimation_PAnimationSlotFilter_getScratchSize @ 0x0052_a4d0 => extern "thiscall" fn vt_0052a4d0(this: u32) -> u32 {
        sm::bridge::size(&mem(), const { sm::size_of(0x0052_a4d0) }, this)
    }
    /// `Phyre_PAnimation_PAnimationSlotFilter_getBatchFunction`: returns the batch function's address (rebased).
    Phyre_PAnimation_PAnimationSlotFilter_getBatchFunction @ 0x0052_a4f0 => extern "thiscall" fn kc_0052a4f0(_this: u32) -> u32 {
        sm::bridge::constant(&mem(), const { sm::constant_of(0x0052_a4f0) })
    }
    /// `Phyre_PAnimation_PAnimationSlotFilter_vslot2`: `[this + 0x14]`.
    Phyre_PAnimation_PAnimationSlotFilter_vslot2 @ 0x0052_a500 => extern "thiscall" fn vt_0052a500(this: u32, _a: u32) -> u32 {
        sm::bridge::word_at(&mem(), this, 0x14)
    }
    /// `Phyre_PAnimation_PAnimationSlotFilter_vslot1`: a constant virtual method: 1.
    Phyre_PAnimation_PAnimationSlotFilter_vslot1 @ 0x0052_a510 => extern "thiscall" fn kc_0052a510(_this: u32) -> u32 {
        sm::bridge::constant(&mem(), const { sm::constant_of(0x0052_a510) })
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_4__swapEndiannessStatic1`: builds the static descriptor `0x00ca_c748` on first use, then `swapEndiannessOfObject(p, 1)`; its `eax`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_4__swapEndiannessStatic1 @ 0x0052_b380 => extern "C" fn sl_0052b380(p: u32) -> u32 {
        sm::bridge::swap_lazy(&mut mem(), &const { sm::lazy_of(0x0052_b380) }, p)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_4__swapEndiannessStatic0`: builds the static descriptor `0x00ca_c748` on first use, then `swapEndiannessOfObject(p, 0)`; its `eax`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_4__swapEndiannessStatic0 @ 0x0052_b440 => extern "C" fn sl_0052b440(p: u32) -> u32 {
        sm::bridge::swap_lazy(&mut mem(), &const { sm::lazy_of(0x0052_b440) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_4__dtor`: its vtable `0x00b297f4`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_4__dtor @ 0x0052_b8c0 => extern "thiscall" fn td_0052b8c0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0052_b8c0) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationBlenderController__dtor`: its vtable `0x00b29644`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationBlenderController__dtor @ 0x0052_b8d0 => extern "thiscall" fn td_0052b8d0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0052_b8d0) }, this)
    }
    /// `Phyre_PAnimation_PAnimationBlenderController_getSource`: source `i`, or 0 past the count.
    Phyre_PAnimation_PAnimationBlenderController_getSource @ 0x0052_cc50 => extern "thiscall" fn vt_0052cc50(this: u32, i: u32) -> u32 {
        sm::bridge::source_at(&mem(), this, i)
    }
    /// `Phyre_PAnimation_PAnimationBlenderController_getSourceCount`: the source count `[this + 0xc] & 0x7fffffff`.
    Phyre_PAnimation_PAnimationBlenderController_getSourceCount @ 0x0052_cc80 => extern "thiscall" fn vt_0052cc80(this: u32) -> u32 {
        sm::bridge::count_at(&mem(), this, 0xc)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationWeightedBlenderController__dtor`: its vtable `0x00b29a0c`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationWeightedBlenderController__dtor @ 0x0052_d6a0 => extern "thiscall" fn td_0052d6a0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0052_d6a0) }, this)
    }
    /// `Phyre_PAnimation_PAnimationWeightedBlenderController_getScratchSize`: per-batch scratch bytes: `((n * 4 + 0xf) & !0xf) + 0x10`, `n` the weight count at `+0x14`.
    Phyre_PAnimation_PAnimationWeightedBlenderController_getScratchSize @ 0x0052_e260 => extern "thiscall" fn vt_0052e260(this: u32) -> u32 {
        sm::bridge::size(&mem(), const { sm::size_of(0x0052_e260) }, this)
    }
    /// `Phyre_PAnimation_PAnimationWeightedBlenderController_getBatchFunction`: returns the batch function's address (rebased).
    Phyre_PAnimation_PAnimationWeightedBlenderController_getBatchFunction @ 0x0052_e280 => extern "thiscall" fn kc_0052e280(_this: u32) -> u32 {
        sm::bridge::constant(&mem(), const { sm::constant_of(0x0052_e280) })
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationAdditiveBlenderController__dtor`: its vtable `0x00b29c4c`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationAdditiveBlenderController__dtor @ 0x0052_ebb0 => extern "thiscall" fn td_0052ebb0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0052_ebb0) }, this)
    }
    /// `Phyre_PAnimation_PAnimationAdditiveBlenderController_getBatchFunction`: returns the batch function's address (rebased).
    Phyre_PAnimation_PAnimationAdditiveBlenderController_getBatchFunction @ 0x0052_f770 => extern "thiscall" fn kc_0052f770(_this: u32) -> u32 {
        sm::bridge::constant(&mem(), const { sm::constant_of(0x0052_f770) })
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSpuTargetBlenderController__dtor`: its vtable `0x00b29e7c`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationSpuTargetBlenderController__dtor @ 0x0053_00b0 => extern "thiscall" fn td_005300b0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0053_00b0) }, this)
    }
    /// `Phyre_PAnimation_PAnimationSpuTargetBlenderController_getScratchSize`: per-batch scratch bytes: `((n * 4 + 0x1f) & !0xf) + 0x10`, `n` the source's count.
    Phyre_PAnimation_PAnimationSpuTargetBlenderController_getScratchSize @ 0x0053_0690 => extern "thiscall" fn vt_00530690(this: u32) -> u32 {
        sm::bridge::size(&mem(), const { sm::size_of(0x0053_0690) }, this)
    }
    /// `Phyre_PAnimation_PAnimationSpuTargetBlenderController_getBatchFunction`: returns the batch function's address (rebased).
    Phyre_PAnimation_PAnimationSpuTargetBlenderController_getBatchFunction @ 0x0053_06b0 => extern "thiscall" fn kc_005306b0(_this: u32) -> u32 {
        sm::bridge::constant(&mem(), const { sm::constant_of(0x0053_06b0) })
    }
    /// `Phyre_PAnimation_PAnimationSpuTargetBlenderController_getSource`: `[this + 0xc]` for source 0, else 0.
    Phyre_PAnimation_PAnimationSpuTargetBlenderController_getSource @ 0x0053_06c0 => extern "thiscall" fn vt_005306c0(this: u32, i: u32) -> u32 {
        sm::bridge::first_source(&mem(), this, i)
    }
    /// `Phyre_PAnimation_PAnimationSpuTargetBlenderController_vslot1`: a constant virtual method: 1.
    Phyre_PAnimation_PAnimationSpuTargetBlenderController_vslot1 @ 0x0053_06e0 => extern "thiscall" fn kc_005306e0(_this: u32) -> u32 {
        sm::bridge::constant(&mem(), const { sm::constant_of(0x0053_06e0) })
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationTargetBlenderController__dtor`: its vtable `0x00b2a0ac`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationTargetBlenderController__dtor @ 0x0053_0e30 => extern "thiscall" fn td_00530e30(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0053_0e30) }, this)
    }
    /// `Phyre_PAnimation_PAnimationTargetBlenderController_vslot1`: a constant virtual method: 1.
    Phyre_PAnimation_PAnimationTargetBlenderController_vslot1 @ 0x0053_15e0 => extern "thiscall" fn kc_005315e0(_this: u32) -> u32 {
        sm::bridge::constant(&mem(), const { sm::constant_of(0x0053_15e0) })
    }
    /// `Phyre_PAnimation_PAnimationTargetBlenderController_vslot10`: a virtual method that returns 0.
    Phyre_PAnimation_PAnimationTargetBlenderController_vslot10 @ 0x0053_1650 => extern "thiscall" fn vt_00531650(_this: u32, _a: u32, _b: u32, _c: u32) -> u32 {
        0
    }
    /// `Phyre_PArray_dtorVirtual_00534550`: `PArray<T>::~PArray` of objects (0xc bytes): each owned element's virtual slot 0 with 0, `PhyreFree`, both words zero.
    Phyre_PArray_dtorVirtual_00534550 @ 0x0053_4550 => extern "thiscall" fn pv_00534550(this: u32) -> u32 {
        sm::bridge::release_virtual(&mut mem(), const { sm::word_of(sm::RELEASES_VIRTUAL, 0x0053_4550) }, this)
    }
    /// `Phyre_PArray_dtorVirtual_005345b0`: `PArray<T>::~PArray` of objects (0x8 bytes): each owned element's virtual slot 0 with 0, `PhyreFree`, both words zero.
    Phyre_PArray_dtorVirtual_005345b0 @ 0x0053_45b0 => extern "thiscall" fn pv_005345b0(this: u32) -> u32 {
        sm::bridge::release_virtual(&mut mem(), const { sm::word_of(sm::RELEASES_VIRTUAL, 0x0053_45b0) }, this)
    }
    /// `Phyre_PArray_dtorVirtual_00534610`: `PArray<T>::~PArray` of objects (0xc bytes): each owned element's virtual slot 0 with 0, `PhyreFree`, both words zero.
    Phyre_PArray_dtorVirtual_00534610 @ 0x0053_4610 => extern "thiscall" fn pv_00534610(this: u32) -> u32 {
        sm::bridge::release_virtual(&mut mem(), const { sm::word_of(sm::RELEASES_VIRTUAL, 0x0053_4610) }, this)
    }
    /// `Phyre_PArray_dtorVirtual_00534670`: `PArray<T>::~PArray` of objects (0x8 bytes): each owned element's virtual slot 0 with 0, `PhyreFree`, both words zero.
    Phyre_PArray_dtorVirtual_00534670 @ 0x0053_4670 => extern "thiscall" fn pv_00534670(this: u32) -> u32 {
        sm::bridge::release_virtual(&mut mem(), const { sm::word_of(sm::RELEASES_VIRTUAL, 0x0053_4670) }, this)
    }
    /// `Phyre_PArray_dtorVirtual_005346d0`: `PArray<T>::~PArray` of objects (0x8 bytes): each owned element's virtual slot 0 with 0, `PhyreFree`, both words zero.
    Phyre_PArray_dtorVirtual_005346d0 @ 0x0053_46d0 => extern "thiscall" fn pv_005346d0(this: u32) -> u32 {
        sm::bridge::release_virtual(&mut mem(), const { sm::word_of(sm::RELEASES_VIRTUAL, 0x0053_46d0) }, this)
    }
    /// `Phyre_PArray_dtorVirtual_00534730`: `PArray<T>::~PArray` of objects (0x8 bytes): each owned element's virtual slot 0 with 0, `PhyreFree`, both words zero.
    Phyre_PArray_dtorVirtual_00534730 @ 0x0053_4730 => extern "thiscall" fn pv_00534730(this: u32) -> u32 {
        sm::bridge::release_virtual(&mut mem(), const { sm::word_of(sm::RELEASES_VIRTUAL, 0x0053_4730) }, this)
    }
    /// `Phyre_PArray_dtor_00534790`: `PArray<T>::~PArray`: `PhyreFree` of owned, non-null elements, then both words zero.
    Phyre_PArray_dtor_00534790 @ 0x0053_4790 => extern "thiscall" fn pr_00534790(this: u32) -> u32 {
        sm::bridge::release(&mut mem(), this)
    }
    /// `Phyre_PArray_dtor_005347c0`: `PArray<T>::~PArray`: `PhyreFree` of owned, non-null elements, then both words zero.
    Phyre_PArray_dtor_005347c0 @ 0x0053_47c0 => extern "thiscall" fn pr_005347c0(this: u32) -> u32 {
        sm::bridge::release(&mut mem(), this)
    }
    /// `Phyre_PArray_dtor_005347f0`: `PArray<T>::~PArray`: `PhyreFree` of owned, non-null elements, then both words zero.
    Phyre_PArray_dtor_005347f0 @ 0x0053_47f0 => extern "thiscall" fn pr_005347f0(this: u32) -> u32 {
        sm::bridge::release(&mut mem(), this)
    }
    /// `Phyre_PArray_dtor_00534850`: `PArray<T>::~PArray`: `PhyreFree` of owned, non-null elements, then both words zero.
    Phyre_PArray_dtor_00534850 @ 0x0053_4850 => extern "thiscall" fn pr_00534850(this: u32) -> u32 {
        sm::bridge::release(&mut mem(), this)
    }
    /// `Phyre_PArray_dtor_00534880`: `PArray<T>::~PArray`: `PhyreFree` of owned, non-null elements, then both words zero.
    Phyre_PArray_dtor_00534880 @ 0x0053_4880 => extern "thiscall" fn pr_00534880(this: u32) -> u32 {
        sm::bridge::release(&mut mem(), this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationDataSourceListEntry_4__dtor`: its vtable `0x00b2aebc`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationDataSourceListEntry_4__dtor @ 0x0053_4a10 => extern "thiscall" fn td_00534a10(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0053_4a10) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationNetworkInstanceTarget_4__dtor`: its vtable `0x00b2adb4`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationNetworkInstanceTarget_4__dtor @ 0x0053_4a20 => extern "thiscall" fn td_00534a20(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0053_4a20) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationSlotArray_4__dtor`: its vtable `0x00b2aba4`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationSlotArray_4__dtor @ 0x0053_4a30 => extern "thiscall" fn td_00534a30(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0053_4a30) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PQWord_16__dtor`: its vtable `0x00b2acac`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PQWord_16__dtor @ 0x0053_4a40 => extern "thiscall" fn td_00534a40(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0053_4a40) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationDataSourceListEntry__dtor`: its vtable `0x00b2a374`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationDataSourceListEntry__dtor @ 0x0053_4a50 => extern "thiscall" fn td_00534a50(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0053_4a50) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationNetworkInstance__dtor`: its vtable `0x00b2a56c`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationNetworkInstance__dtor @ 0x0053_4a60 => extern "thiscall" fn td_00534a60(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0053_4a60) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationNetworkInstanceTarget__dtor`: its vtable `0x00b2a46c`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationNetworkInstanceTarget__dtor @ 0x0053_4a70 => extern "thiscall" fn td_00534a70(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0053_4a70) }, this)
    }
    /// `Phyre_PAnimation_PAnimationHierarchyVisitContext_dtor_00534af0`: `jmp Phyre_PAnimation_PAnimationHierarchyVisitContext_dtor` (an unwind target).
    Phyre_PAnimation_PAnimationHierarchyVisitContext_dtor_00534af0 @ 0x0053_4af0 => extern "thiscall" fn sv_00534af0(this: u32) {
        sm::bridge::set_vtable(&mut mem(), this, sm::VISIT_CONTEXT_VTABLE);
    }
    /// `Phyre_PAnimation_PAnimationHierarchyVisitContext_dtor_00534b00`: `jmp Phyre_PAnimation_PAnimationHierarchyVisitContext_dtor` (an unwind target).
    Phyre_PAnimation_PAnimationHierarchyVisitContext_dtor_00534b00 @ 0x0053_4b00 => extern "thiscall" fn sv_00534b00(this: u32) {
        sm::bridge::set_vtable(&mut mem(), this, sm::VISIT_CONTEXT_VTABLE);
    }
    /// `Phyre_PAnimation_PAnimationChannelTargetBind_scalarDeletingDtor`: scalar deleting destructor: the base destructor, the free when `flags & 1`; `this`.
    Phyre_PAnimation_PAnimationChannelTargetBind_scalarDeletingDtor @ 0x0053_5620 => extern "thiscall" fn dd_00535620(this: u32, flags: u32) -> u32 {
        sm::bridge::delete(&mut mem(), const { sm::delete_of(0x0053_5620) }, this, flags)
    }
    /// `Phyre_PAnimation_PAnimationChannelTargetBindDataMember_scalarDeletingDtor`: scalar deleting destructor: the base destructor, the free when `flags & 1`; `this`.
    Phyre_PAnimation_PAnimationChannelTargetBindDataMember_scalarDeletingDtor @ 0x0053_5650 => extern "thiscall" fn dd_00535650(this: u32, flags: u32) -> u32 {
        sm::bridge::delete(&mut mem(), const { sm::delete_of(0x0053_5650) }, this, flags)
    }
    /// `Phyre_PAnimation_PAnimationChannelTargetBindFunction_scalarDeletingDtor`: scalar deleting destructor: the base destructor, the free when `flags & 1`; `this`.
    Phyre_PAnimation_PAnimationChannelTargetBindFunction_scalarDeletingDtor @ 0x0053_5680 => extern "thiscall" fn dd_00535680(this: u32, flags: u32) -> u32 {
        sm::bridge::delete(&mut mem(), const { sm::delete_of(0x0053_5680) }, this, flags)
    }
    /// `Phyre_PAnimation_PAnimationChannelTargetBindMethod_scalarDeletingDtor`: scalar deleting destructor: the base destructor, the free when `flags & 1`; `this`.
    Phyre_PAnimation_PAnimationChannelTargetBindMethod_scalarDeletingDtor @ 0x0053_56b0 => extern "thiscall" fn dd_005356b0(this: u32, flags: u32) -> u32 {
        sm::bridge::delete(&mut mem(), const { sm::delete_of(0x0053_56b0) }, this, flags)
    }
    /// `Phyre_PAnimation_PAnimationChannelTargetBindScene_scalarDeletingDtor`: scalar deleting destructor: the base destructor, the free when `flags & 1`; `this`.
    Phyre_PAnimation_PAnimationChannelTargetBindScene_scalarDeletingDtor @ 0x0053_56e0 => extern "thiscall" fn dd_005356e0(this: u32, flags: u32) -> u32 {
        sm::bridge::delete(&mut mem(), const { sm::delete_of(0x0053_56e0) }, this, flags)
    }
    /// `Phyre_PAnimation_PAnimationChannelTargetBindSceneToMeshInstance_scalarDeletingDtor`: scalar deleting destructor: the base destructor, the free when `flags & 1`; `this`.
    Phyre_PAnimation_PAnimationChannelTargetBindSceneToMeshInstance_scalarDeletingDtor @ 0x0053_5710 => extern "thiscall" fn dd_00535710(this: u32, flags: u32) -> u32 {
        sm::bridge::delete(&mut mem(), const { sm::delete_of(0x0053_5710) }, this, flags)
    }
    /// `Phyre_PAnimation_PAnimationChannelTargetBindShaderParameter_scalarDeletingDtor`: scalar deleting destructor: the base destructor, the free when `flags & 1`; `this`.
    Phyre_PAnimation_PAnimationChannelTargetBindShaderParameter_scalarDeletingDtor @ 0x0053_5740 => extern "thiscall" fn dd_00535740(this: u32, flags: u32) -> u32 {
        sm::bridge::delete(&mut mem(), const { sm::delete_of(0x0053_5740) }, this, flags)
    }
    /// `Phyre_PAnimation_PAnimationHierarchyCountNodes_scalarDeletingDtor`: scalar deleting destructor: the base destructor, the free when `flags & 1`; `this`.
    Phyre_PAnimation_PAnimationHierarchyCountNodes_scalarDeletingDtor @ 0x0053_5770 => extern "thiscall" fn dd_00535770(this: u32, flags: u32) -> u32 {
        sm::bridge::delete(&mut mem(), const { sm::delete_of(0x0053_5770) }, this, flags)
    }
    /// `Phyre_PAnimation_PAnimationHierarchyPopulateArrays_scalarDeletingDtor`: scalar deleting destructor: the base destructor, the free when `flags & 1`; `this`.
    Phyre_PAnimation_PAnimationHierarchyPopulateArrays_scalarDeletingDtor @ 0x0053_57a0 => extern "thiscall" fn dd_005357a0(this: u32, flags: u32) -> u32 {
        sm::bridge::delete(&mut mem(), const { sm::delete_of(0x0053_57a0) }, this, flags)
    }
    /// `Phyre_PFreelistAllocatedThreadPoolJob_scalarDeletingDtor`: scalar deleting destructor: the base destructor, the free when `flags & 1`; `this`.
    Phyre_PFreelistAllocatedThreadPoolJob_scalarDeletingDtor @ 0x0053_5830 => extern "thiscall" fn dd_00535830(this: u32, flags: u32) -> u32 {
        sm::bridge::delete(&mut mem(), const { sm::delete_of(0x0053_5830) }, this, flags)
    }
    /// `Phyre_PThreadPoolJob_scalarDeletingDtor`: scalar deleting destructor: the base destructor, the free when `flags & 1`; `this`.
    Phyre_PThreadPoolJob_scalarDeletingDtor @ 0x0053_5860 => extern "thiscall" fn dd_00535860(this: u32, flags: u32) -> u32 {
        sm::bridge::delete(&mut mem(), const { sm::delete_of(0x0053_5860) }, this, flags)
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationDataSourceListEntry_setBorrowed`: `PArray<T>::setBorrowed(els, count)` (`Phyre_PArray_float_setBorrowed`'s shape); 0.
    Phyre_PArray_Phyre_PAnimation_PAnimationDataSourceListEntry_setBorrowed @ 0x0053_c810 => extern "thiscall" fn sb_0053c810(this: u32, els: u32, count: u32) -> u32 {
        const { sm::listed(sm::SET_BORROWED, 0x0053_c810) };
        array::bridge::set_borrowed(&mut mem(), this, els, count, array::Old::Free)
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationNetworkInstanceTarget_setBorrowed`: `PArray<T>::setBorrowed(els, count)` (`Phyre_PArray_float_setBorrowed`'s shape); 0.
    Phyre_PArray_Phyre_PAnimation_PAnimationNetworkInstanceTarget_setBorrowed @ 0x0053_c860 => extern "thiscall" fn sb_0053c860(this: u32, els: u32, count: u32) -> u32 {
        const { sm::listed(sm::SET_BORROWED, 0x0053_c860) };
        array::bridge::set_borrowed(&mut mem(), this, els, count, array::Old::Free)
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationSlotArray_setBorrowed`: `PArray<T>::setBorrowed(els, count)` (`Phyre_PArray_float_setBorrowed`'s shape); 0.
    Phyre_PArray_Phyre_PAnimation_PAnimationSlotArray_setBorrowed @ 0x0053_c8b0 => extern "thiscall" fn sb_0053c8b0(this: u32, els: u32, count: u32) -> u32 {
        const { sm::listed(sm::SET_BORROWED, 0x0053_c8b0) };
        array::bridge::set_borrowed(&mut mem(), this, els, count, array::Old::Free)
    }
    /// `Phyre_PArray_Phyre_PUInt128_setBorrowed`: `PArray<T>::setBorrowed(els, count)` (`Phyre_PArray_float_setBorrowed`'s shape); 0.
    Phyre_PArray_Phyre_PUInt128_setBorrowed @ 0x0053_c900 => extern "thiscall" fn sb_0053c900(this: u32, els: u32, count: u32) -> u32 {
        const { sm::listed(sm::SET_BORROWED, 0x0053_c900) };
        array::bridge::set_borrowed(&mut mem(), this, els, count, array::Old::Free)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PBlendableAnimationSource_4__swapEndiannessStatic1`: builds the static descriptor `0x00ca_e060` on first use, then `swapEndiannessOfObject(p, 1)`; its `eax`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PBlendableAnimationSource_4__swapEndiannessStatic1 @ 0x0053_de20 => extern "C" fn sl_0053de20(p: u32) -> u32 {
        sm::bridge::swap_lazy(&mut mem(), &const { sm::lazy_of(0x0053_de20) }, p)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PBlendableAnimationSource_4__swapEndiannessStatic0`: builds the static descriptor `0x00ca_e060` on first use, then `swapEndiannessOfObject(p, 0)`; its `eax`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PAnimation_PBlendableAnimationSource_4__swapEndiannessStatic0 @ 0x0053_df00 => extern "C" fn sl_0053df00(p: u32) -> u32 {
        sm::bridge::swap_lazy(&mut mem(), &const { sm::lazy_of(0x0053_df00) }, p)
    }
    /// `Phyre_PArray_dtor4c_0053e9e0`: `PArray<T>::~PArray` of `0x4c`-byte elements: owned ones to `Phyre_DeleteArray4c_00540b80`, both words zero.
    Phyre_PArray_dtor4c_0053e9e0 @ 0x0053_e9e0 => extern "thiscall" fn pw_0053e9e0(this: u32) -> u32 {
        sm::bridge::release_with(&mut mem(), const { sm::word_of(sm::RELEASES_WITH, 0x0053_e9e0) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PBlendableAnimationSource_4__dtor`: its vtable `0x00b2ba74`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PBlendableAnimationSource_4__dtor @ 0x0053_eaa0 => extern "thiscall" fn td_0053eaa0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0053_eaa0) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimatableComponent__dtor`: its vtable `0x00b2b4e4`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimatableComponent__dtor @ 0x0053_eab0 => extern "thiscall" fn td_0053eab0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0053_eab0) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PBlendableAnimationSource__dtor`: its vtable `0x00b2b3f4`, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PBlendableAnimationSource__dtor @ 0x0053_eac0 => extern "thiscall" fn td_0053eac0(this: u32) -> u32 {
        sm::bridge::tail_dtor(&mut mem(), const { sm::word_of(sm::TAIL_DTORS, 0x0053_eac0) }, this)
    }
    /// `Phyre_PAnimation_PAnimationHierarchyCountNodes_visitBegin`: counts a visit at `+8`; `al = 1`.
    Phyre_PAnimation_PAnimationHierarchyCountNodes_visitBegin @ 0x005c_dab0 => extern "thiscall" fn vt_005cdab0(this: u32, _a: u32, _b: u32) -> u8 {
        sm::bridge::count_visit(&mut mem(), this, 8)
    }
    /// `Phyre_PAnimation_PAnimationHierarchyCountNodes_visitEnd`: counts a visit at `+4`; `al = 1`.
    Phyre_PAnimation_PAnimationHierarchyCountNodes_visitEnd @ 0x005c_dac0 => extern "thiscall" fn vt_005cdac0(this: u32, _a: u32, _b: u32) -> u8 {
        sm::bridge::count_visit(&mut mem(), this, 4)
    }
    /// `Phyre_PAnimation_PAnimationHierarchyPopulateArrays_visitEnd`: a visit callback: `al = 1`.
    Phyre_PAnimation_PAnimationHierarchyPopulateArrays_visitEnd @ 0x005c_db60 => extern "thiscall" fn vt_005cdb60(_this: u32, _a: u32, _b: u32) -> u8 {
        1
    }
    /// `Phyre_PAnimation_PAnimationChannelTargetBind_dtor`: `PAnimationChannelTargetBind::~PAnimationChannelTargetBind`: its vtable; `eax` stays the caller's.
    Phyre_PAnimation_PAnimationChannelTargetBind_dtor @ 0x005c_db70 => extern "thiscall" fn sv_005cdb70(this: u32) {
        sm::bridge::set_vtable(&mut mem(), this, sm::TARGET_BIND_VTABLE);
    }
    /// `Phyre_PAnimation_PAnimationChannelTargetBindScene_clearTree`: `PRedBlackTree::clear` on `this + 0xc` (tail jump); its `eax`.
    Phyre_PAnimation_PAnimationChannelTargetBindScene_clearTree @ 0x005c_e190 => extern "thiscall" fn vt_005ce190(this: u32) -> u32 {
        sm::bridge::clear_tree(&mut mem(), this)
    }
    /// `Phyre_PScripting_PAsyncProcessBuffer_process`: each header from `[this + 4]` along `+8` processed; the first non-zero result, or 0.
    Phyre_PScripting_PAsyncProcessBuffer_process @ 0x005c_ea70 => extern "thiscall" fn vt_005cea70(this: u32) -> u32 {
        sm::bridge::process_all(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_getCount100`: `[this + 0x100] & 0x7fffffff`.
    Phyre_PAnimation_PAnimatableComponent_getCount100 @ 0x0061_6a20 => extern "thiscall" fn vt_00616a20(this: u32) -> u32 {
        sm::bridge::count_at(&mem(), this, 0x100)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_getWord118`: `[this + 0x118]`.
    Phyre_PAnimation_PAnimatableComponent_getWord118 @ 0x0061_6aa0 => extern "thiscall" fn vt_00616aa0(this: u32) -> u32 {
        sm::bridge::word_at(&mem(), this, 0x118)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_hasWord118`: `[this + 0x118] != -1`.
    Phyre_PAnimation_PAnimatableComponent_hasWord118 @ 0x0061_6d50 => extern "thiscall" fn vt_00616d50(this: u32) -> u32 {
        sm::bridge::has_word_118(&mem(), this)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_setWord118`: `[this + 0x118] = v`; `v`.
    Phyre_PAnimation_PAnimatableComponent_setWord118 @ 0x0061_6eb0 => extern "thiscall" fn vt_00616eb0(this: u32, v: u32) -> u32 {
        sm::bridge::set_word_118(&mut mem(), this, v)
    }
    /// `Phyre_PArray_dtor_006171e0`: `PArray<T>::~PArray`: `PhyreFree` of owned, non-null elements, then both words zero.
    Phyre_PArray_dtor_006171e0 @ 0x0061_71e0 => extern "thiscall" fn pr_006171e0(this: u32) -> u32 {
        sm::bridge::release(&mut mem(), this)
    }
}
