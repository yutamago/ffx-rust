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
use ffx_panim::m5p_anim_arrays::{self as ar, bridge as ab};
use ffx_panim::m5p_anim_misc::{self as mi, bridge as mb};
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
    /// `Phyre_PAnimation_PUtilityAnimation_unlink_00507b70`: `jmp Phyre_PAnimation_PUtilityAnimation_unlink`.
    Phyre_PAnimation_PUtilityAnimation_unlink_00507b70 @ 0x0050_7b70 => extern "C" fn vi_00507b70(x: u32) -> u32 {
        mb::unlink_utility(&mut mem(), x)
    }
    /// `Phyre_PAnimation_PUtilityAnimation_unlink`: unlinks `x` from the list at `0x00ca9734`; 0.
    Phyre_PAnimation_PUtilityAnimation_unlink @ 0x0050_7b80 => extern "C" fn vi_00507b80(x: u32) -> u32 {
        mb::unlink_utility(&mut mem(), x)
    }
    /// `Phyre_PAnimation_PAnimationChannelTimes_getRange`: the first and last float of the times; 0, or 7 when empty.
    Phyre_PAnimation_PAnimationChannelTimes_getRange @ 0x0050_acf0 => extern "thiscall" fn vi_0050acf0(this: u32, first: u32, last: u32) -> u32 {
        mb::first_last(&mut mem(), this, first, last)
    }
    /// `Phyre_PAnimation_PAnimationConstantChannel_assign`: `operator=`: the base's, then `+0x1c..+0x34`; `this`.
    Phyre_PAnimation_PAnimationConstantChannel_assign @ 0x0050_f220 => extern "thiscall" fn vi_0050f220(this: u32, other: u32) -> u32 {
        mb::assign_constant_channel(&mut mem(), this, other)
    }
    /// `Phyre_PAnimation_PAnimationClip_getConstantChannel`: the address of element `i` (`0x34` bytes) at `[this + 0x10]`.
    Phyre_PAnimation_PAnimationClip_getConstantChannel @ 0x0051_11d0 => extern "thiscall" fn vi_005111d0(this: u32, i: u32) -> u32 {
        sm::bridge::element_34(&mem(), this, i)
    }
    /// `Phyre_PAnimation_PAnimationClip_removeConstantChannel`: removes constant channel `i` (the last moved over it); 3 out of range.
    Phyre_PAnimation_PAnimationClip_removeConstantChannel @ 0x0051_6e60 => extern "thiscall" fn vi_00516e60(this: u32, i: u32) -> u32 {
        mb::remove_constant_channel(&mut mem(), this, i)
    }
    /// `Phyre_PAnimation_PAnimationSlotListIndex_copyConstruct`: placement construction of `n` elements (16-byte elements copied); the original's `eax`.
    Phyre_PAnimation_PAnimationSlotListIndex_copyConstruct @ 0x0051_7900 => extern "C" fn vi_00517900(dst: u32, src: u32, n: u32) -> u32 {
        mb::construct(&mut mem(), const { mi::construct_of(0x0051_7900) }, dst, src, n)
    }
    /// `Phyre_PArray_dtorFree_00519150`: `PArray<T>::~PArray`: owned elements freed (null or not), both words zero.
    Phyre_PArray_dtorFree_00519150 @ 0x0051_9150 => extern "thiscall" fn vi_00519150(this: u32) -> u32 {
        mb::release_free(&mut mem(), this)
    }
    /// `Phyre_PSharray_dtor_00519340`: `PSharray<T>::~PSharray`: an owned array of more than one element freed, both words zero.
    Phyre_PSharray_dtor_00519340 @ 0x0051_9340 => extern "thiscall" fn vi_00519340(this: u32) -> u32 {
        mb::release_sharray(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PAnimationSlotListIndex_construct`: placement construction of `n` elements (16-byte elements set to 0, 3, 0, 0); the original's `eax`.
    Phyre_PAnimation_PAnimationSlotListIndex_construct @ 0x0051_bc30 => extern "C" fn vi_0051bc30(dst: u32, n: u32) -> u32 {
        mb::construct(&mut mem(), const { mi::construct_of(0x0051_bc30) }, dst, 0, n)
    }
    /// `Phyre_PAnimation_PAnimationSet_appendSlot`: the set's lookup of `count + 1`, else an insert of `value` at `count`; their `eax`.
    Phyre_PAnimation_PAnimationSet_appendSlot @ 0x0051_c2e0 => extern "thiscall" fn vi_0051c2e0(this: u32, value: u32) -> u32 {
        mb::append(&mut mem(), this, value)
    }
    /// `Phyre_PSharray_contains_0051ca20`: `key` when the array holds it, else 0.
    Phyre_PSharray_contains_0051ca20 @ 0x0051_ca20 => extern "thiscall" fn vi_0051ca20(this: u32, key: u32) -> u32 {
        mb::contains(&mut mem(), this, key)
    }
    /// `Phyre_PSharray_get_0051cc50`: `PSharray<T>` element `i` (inline for one element), 0 out of range.
    Phyre_PSharray_get_0051cc50 @ 0x0051_cc50 => extern "thiscall" fn vi_0051cc50(this: u32, i: u32) -> u32 {
        mb::sharray_get(&mem(), this, i)
    }
    /// `Phyre_PAnimation_PTimeController_visit`: the hierarchy visit: the visitor's begin, the child's visit, the end; `al` 1 when all went on.
    Phyre_PAnimation_PTimeController_visit @ 0x0052_2240 => extern "thiscall" fn vi_00522240(this: u32, visitor: u32, ctx: u32) -> u32 {
        mb::visit(&mut mem(), const { mi::visit_of(0x0052_2240) }, this, visitor, ctx)
    }
    /// `Phyre_PAnimation_PTimeIntervalController_visit`: the hierarchy visit: the visitor's begin, the child's visit, the end; `al` 1 when all went on.
    Phyre_PAnimation_PTimeIntervalController_visit @ 0x0052_2ea0 => extern "thiscall" fn vi_00522ea0(this: u32, visitor: u32, ctx: u32) -> u32 {
        mb::visit(&mut mem(), const { mi::visit_of(0x0052_2ea0) }, this, visitor, ctx)
    }
    /// `Phyre_PAnimation_PTimeScaleOffsetController_getTime`: the source's time (or the constant) times `+0xc` plus `+0x10`, in `ST0`.
    Phyre_PAnimation_PTimeScaleOffsetController_getTime @ 0x0052_3890 => extern "thiscall" fn vi_00523890(this: u32) -> f32 {
        mb::scaled_time(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PTimeScaleOffsetController_visit`: the hierarchy visit: the visitor's begin, the child's visit, the end; `al` 1 when all went on.
    Phyre_PAnimation_PTimeScaleOffsetController_visit @ 0x0052_3a60 => extern "thiscall" fn vi_00523a60(this: u32, visitor: u32, ctx: u32) -> u32 {
        mb::visit(&mut mem(), const { mi::visit_of(0x0052_3a60) }, this, visitor, ctx)
    }
    /// `Phyre_PAnimation_PAnimationDataSource_scalarDeletingDtor`: scalar deleting destructor: vtable, `~PAnimationHierarchyNode`, free; `this`.
    Phyre_PAnimation_PAnimationDataSource_scalarDeletingDtor @ 0x0052_3dd0 => extern "thiscall" fn vi_00523dd0(this: u32, flags: u32) -> u32 {
        mb::delete_data_source(&mut mem(), this, flags)
    }
    /// `Phyre_PAnimation_PAnimationDataSource_markOutputs`: `flags[idx[i]] = 1` for `i < n`; `n`.
    Phyre_PAnimation_PAnimationDataSource_markOutputs @ 0x0052_41d0 => extern "thiscall" fn vi_005241d0(_this: u32, flags: u32, idx: u32, n: u32) -> u32 {
        mb::mark(&mut mem(), flags, idx, n)
    }
    /// `Phyre_PAnimation_PAnimationEventController_visit`: the hierarchy visit: the visitor's begin, the child's visit, the end; `al` 1 when all went on.
    Phyre_PAnimation_PAnimationEventController_visit @ 0x0052_4ec0 => extern "thiscall" fn vi_00524ec0(this: u32, visitor: u32, ctx: u32) -> u32 {
        mb::visit(&mut mem(), const { mi::visit_of(0x0052_4ec0) }, this, visitor, ctx)
    }
    /// `Phyre_PAnimation_PAnimationController_getClipBytes`: the clip's output bytes (`0xb0` per set, `0x34` per constant, the header's `u16`), 0 without one.
    Phyre_PAnimation_PAnimationController_getClipBytes @ 0x0052_6290 => extern "thiscall" fn vi_00526290(this: u32) -> u32 {
        mb::clip_bytes(&mem(), this)
    }
    /// `Phyre_PAnimation_PAnimationController_setTarget`: the target at `+0xc` when the slot list at `+8` holds it; 0, 0x11 or 0x13.
    Phyre_PAnimation_PAnimationController_setTarget @ 0x0052_6590 => extern "thiscall" fn vi_00526590(this: u32, target: u32) -> u32 {
        mb::set_target(&mut mem(), this, target)
    }
    /// `Phyre_PAnimation_PAnimationController_visit`: the hierarchy visit: the visitor's begin, the child's visit, the end; `al` 1 when all went on.
    Phyre_PAnimation_PAnimationController_visit @ 0x0052_6690 => extern "thiscall" fn vi_00526690(this: u32, visitor: u32, ctx: u32) -> u32 {
        mb::visit(&mut mem(), const { mi::visit_of(0x0052_6690) }, this, visitor, ctx)
    }
    /// `Phyre_PAnimation_PAnimationBlenderController_setSource`: source `i` when it shares the set; 0, 0x11, 0x13 or 3.
    Phyre_PAnimation_PAnimationBlenderController_setSource @ 0x0052_cec0 => extern "thiscall" fn vi_0052cec0(this: u32, i: u32, source: u32) -> u32 {
        mb::set_source_at(&mut mem(), this, i, source)
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_setBorrowed`: `setBorrowed(els, n)` that frees old owned elements null or not; 0.
    Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_setBorrowed @ 0x0052_d000 => extern "thiscall" fn vi_0052d000(this: u32, els: u32, n: u32) -> u32 {
        array::bridge::set_borrowed(&mut mem(), this, els, n, array::Old::FreeAlways)
    }
    /// `Phyre_PAnimation_PAnimationWeightedBlenderController_batch`: the batch function: `0x0052c8f0` with the kernel `0x0052dae0`; its `eax`.
    Phyre_PAnimation_PAnimationWeightedBlenderController_batch @ 0x0052_e140 => extern "C" fn vi_0052e140(a: u32, b: u32, c: u32, d: u32, e: u32, f: u32) -> u32 {
        mb::blend_batch(&mut mem(), 0x0052_dae0, [a, b, c, d, e, f])
    }
    /// `Phyre_PAnimation_PAnimationWeightedBlenderController_setWeight`: weight `i` = `f`; 0, or 3 out of range.
    Phyre_PAnimation_PAnimationWeightedBlenderController_setWeight @ 0x0052_e660 => extern "thiscall" fn vi_0052e660(this: u32, i: u32, f: u32) -> u32 {
        mb::set_weight(&mut mem(), this, i, f)
    }
    /// `Phyre_PAnimation_PAnimationAdditiveBlenderController_batch`: the batch function: `0x0052c8f0` with the kernel `0x0052ef70`; its `eax`.
    Phyre_PAnimation_PAnimationAdditiveBlenderController_batch @ 0x0052_f600 => extern "C" fn vi_0052f600(a: u32, b: u32, c: u32, d: u32, e: u32, f: u32) -> u32 {
        mb::blend_batch(&mut mem(), 0x0052_ef70, [a, b, c, d, e, f])
    }
    /// `Phyre_PAnimation_PAnimationAdditiveBlenderController_getScratchSize`: per-batch scratch bytes: `((n * 4 + 0xf) & !0xf) + 0x10`.
    Phyre_PAnimation_PAnimationAdditiveBlenderController_getScratchSize @ 0x0052_f750 => extern "thiscall" fn vi_0052f750(this: u32) -> u32 {
        sm::bridge::size(&mem(), (sm::Count::Own(0x14), 0xf), this)
    }
    /// `Phyre_PAnimation_PAnimationSpuTargetBlenderController_batch`: the batch function: copies `[desc + 4] * 32` bytes of the input; 0.
    Phyre_PAnimation_PAnimationSpuTargetBlenderController_batch @ 0x0053_0580 => extern "C" fn vi_00530580(out: u32, input: u32, _c: u32, _d: u32, _e: u32, desc: u32) -> u32 {
        mb::copy_batch(&mut mem(), out, input, desc)
    }
    /// `Phyre_PAnimation_PAnimationSpuTargetBlenderController_visit`: the hierarchy visit: the visitor's begin, the child's visit, the end; `al` 1 when all went on.
    Phyre_PAnimation_PAnimationSpuTargetBlenderController_visit @ 0x0053_08e0 => extern "thiscall" fn vi_005308e0(this: u32, visitor: u32, ctx: u32) -> u32 {
        mb::visit(&mut mem(), const { mi::visit_of(0x0053_08e0) }, this, visitor, ctx)
    }
    /// `Phyre_PAnimation_PAnimationTargetBlenderController_getSource`: `[this + 0xc]` for source 0, else 0.
    Phyre_PAnimation_PAnimationTargetBlenderController_getSource @ 0x0053_15c0 => extern "thiscall" fn vi_005315c0(this: u32, i: u32) -> u32 {
        sm::bridge::first_source(&mem(), this, i)
    }
    /// `Phyre_PAnimation_PAnimationTargetBlenderController_setSource`: the source at `+0xc` when it shares the set; 0, 0x11 or 0x13.
    Phyre_PAnimation_PAnimationTargetBlenderController_setSource @ 0x0053_1a10 => extern "thiscall" fn vi_00531a10(this: u32, source: u32) -> u32 {
        mb::set_source(&mut mem(), this, source)
    }
    /// `Phyre_PAnimation_copyConstruct94_00531e60`: placement construction of `n` elements (0x94-byte elements copied); the original's `eax`.
    Phyre_PAnimation_copyConstruct94_00531e60 @ 0x0053_1e60 => extern "C" fn vi_00531e60(dst: u32, src: u32, n: u32) -> u32 {
        mb::construct(&mut mem(), const { mi::construct_of(0x0053_1e60) }, dst, src, n)
    }
    /// `Phyre_PGameplay_findOwnerPart_005337d0`: `[this] - 4` when `0x00ca91f8` is on the `+0x40` list from `[this + 4]`, else 0.
    Phyre_PGameplay_findOwnerPart_005337d0 @ 0x0053_37d0 => extern "thiscall" fn vi_005337d0(this: u32) -> u32 {
        mb::owner_part(&mem(), this)
    }
    /// `Phyre_PAnimation_construct18_00539550`: placement construction of `n` elements (24-byte elements zeroed); the original's `eax`.
    Phyre_PAnimation_construct18_00539550 @ 0x0053_9550 => extern "C" fn vi_00539550(dst: u32, n: u32) -> u32 {
        mb::construct(&mut mem(), const { mi::construct_of(0x0053_9550) }, dst, 0, n)
    }
    /// `Phyre_PAnimation_construct10_00539600`: placement construction of `n` elements (16-byte elements zeroed); the original's `eax`.
    Phyre_PAnimation_construct10_00539600 @ 0x0053_9600 => extern "C" fn vi_00539600(dst: u32, n: u32) -> u32 {
        mb::construct(&mut mem(), const { mi::construct_of(0x0053_9600) }, dst, 0, n)
    }
    /// `Phyre_PAnimation_deleteFreeList_0053a0e0`: deletes the free list at `0x00cacd10`; 0.
    Phyre_PAnimation_deleteFreeList_0053a0e0 @ 0x0053_a0e0 => extern "C" fn vi_0053a0e0() -> u32 {
        mb::delete_free_list(&mut mem())
    }
    /// `Phyre_PAnimation_getElement28_0053afd0`: the address of 28-byte element `i` at `[this + 0xc]`, or 0.
    Phyre_PAnimation_getElement28_0053afd0 @ 0x0053_afd0 => extern "thiscall" fn vi_0053afd0(this: u32, i: u32) -> u32 {
        mb::element_28(&mem(), this, i)
    }
    /// `Phyre_PArray_truncate_0053cf20`: the count lowered to `n`; an owned array left empty freed. The original's `eax`.
    Phyre_PArray_truncate_0053cf20 @ 0x0053_cf20 => extern "thiscall" fn vi_0053cf20(this: u32, n: u32) -> u32 {
        mb::truncate(&mut mem(), this, n)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_getSource`: source record `i`'s first word, or 0.
    Phyre_PAnimation_PAnimatableComponent_getSource @ 0x0054_2500 => extern "thiscall" fn vi_00542500(this: u32, i: u32) -> u32 {
        mb::word_at(&mem(), this, i)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_getFirstSourceValue`: the value of the first set source record, or 0, in `ST0`.
    Phyre_PAnimation_PAnimatableComponent_getFirstSourceValue @ 0x0054_2560 => extern "thiscall" fn vi_00542560(this: u32) -> f32 {
        mb::first_value(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_getSourceValue`: source record `i`'s value, or 0, in `ST0`.
    Phyre_PAnimation_PAnimatableComponent_getSourceValue @ 0x0054_25c0 => extern "thiscall" fn vi_005425c0(this: u32, i: u32) -> f32 {
        mb::value_at(&mut mem(), this, i)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_getSourceSpeed`: source record `i`'s float at `+8`, or 0, in `ST0`.
    Phyre_PAnimation_PAnimatableComponent_getSourceSpeed @ 0x0054_2e90 => extern "thiscall" fn vi_00542e90(this: u32, i: u32) -> f32 {
        mb::float_at(&mem(), this, i, 8)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_getSourceWeight`: source record `i`'s float at `+4`, or 0, in `ST0`.
    Phyre_PAnimation_PAnimatableComponent_getSourceWeight @ 0x0054_2fb0 => extern "thiscall" fn vi_00542fb0(this: u32, i: u32) -> f32 {
        mb::float_at(&mem(), this, i, 4)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_setBySourceA`: the component's lookup of `source`, else `0x00543280(0, source)`.
    Phyre_PAnimation_PAnimatableComponent_setBySourceA @ 0x0054_3120 => extern "thiscall" fn vi_00543120(this: u32, source: u32) -> u32 {
        mb::by_source(&mut mem(), 0x0054_3280, this, source)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_setByNameA`: the named source, looked up, else `0x00543280(0, source)`; 0x11, 0x13 or the `eax`.
    Phyre_PAnimation_PAnimatableComponent_setByNameA @ 0x0054_31f0 => extern "thiscall" fn vi_005431f0(this: u32, name: u32) -> u32 {
        mb::by_name(&mut mem(), 0x0054_3280, this, name)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_setByNameA2`: the named source handed to `0x00543280(a, source)`; 0x11, 0x13 or its `eax`.
    Phyre_PAnimation_PAnimatableComponent_setByNameA2 @ 0x0054_3240 => extern "thiscall" fn vi_00543240(this: u32, a: u32, name: u32) -> u32 {
        mb::by_name2(&mut mem(), 0x0054_3280, this, a, name)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_setBySourceB`: the component's lookup of `source`, else `0x00543480(0, source)`.
    Phyre_PAnimation_PAnimatableComponent_setBySourceB @ 0x0054_3300 => extern "thiscall" fn vi_00543300(this: u32, source: u32) -> u32 {
        mb::by_source(&mut mem(), 0x0054_3480, this, source)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_setByNameB`: the named source, looked up, else `0x00543480(0, source)`; 0x11, 0x13 or the `eax`.
    Phyre_PAnimation_PAnimatableComponent_setByNameB @ 0x0054_33f0 => extern "thiscall" fn vi_005433f0(this: u32, name: u32) -> u32 {
        mb::by_name(&mut mem(), 0x0054_3480, this, name)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_setByNameB2`: the named source handed to `0x00543480(a, source)`; 0x11, 0x13 or its `eax`.
    Phyre_PAnimation_PAnimatableComponent_setByNameB2 @ 0x0054_3440 => extern "thiscall" fn vi_00543440(this: u32, a: u32, name: u32) -> u32 {
        mb::by_name2(&mut mem(), 0x0054_3480, this, a, name)
    }
    /// `Phyre_PArray_Phyre_PAnimation_PBlendableAnimationSource_setBorrowed`: `setBorrowed(els, n)` whose old owned elements go to `Phyre_DeleteArray4c_00540b80`; 0.
    Phyre_PArray_Phyre_PAnimation_PBlendableAnimationSource_setBorrowed @ 0x0054_41e0 => extern "thiscall" fn vi_005441e0(this: u32, els: u32, n: u32) -> u32 {
        mb::set_borrowed_with(&mut mem(), 0x0054_0b80, this, els, n)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_setSourceWeight`: source record `i`'s weight and the blender's = `f`; 0, or 3.
    Phyre_PAnimation_PAnimatableComponent_setSourceWeight @ 0x0054_42e0 => extern "thiscall" fn vi_005442e0(this: u32, i: u32, f: u32) -> u32 {
        mb::set_source_weight(&mut mem(), this, i, f)
    }
    /// `Phyre_PSpriteAnimationInfo_getFrame_0056d690`: `*out` = frame `[[this + 0xc8] + 4i]`'s address; 0, 9 or 5.
    Phyre_PSpriteAnimationInfo_getFrame_0056d690 @ 0x0056_d690 => extern "thiscall" fn vi_0056d690(this: u32, i: u32, out: u32) -> u32 {
        mb::frame(&mut mem(), this, i, out)
    }
    /// `Phyre_PAnimation_findRecord11c_005cd750`: the index of the first `0x11c`-byte record whose `+0x118` is `key`, or -1.
    Phyre_PAnimation_findRecord11c_005cd750 @ 0x005c_d750 => extern "thiscall" fn vi_005cd750(this: u32, key: u32) -> u32 {
        mb::find_record(&mem(), this, key)
    }
    /// `Phyre_PRingBuffer_wouldOverrun_005ce9d0`: whether `n` bytes from the write position run into the reader; `al` over the read position.
    Phyre_PRingBuffer_wouldOverrun_005ce9d0 @ 0x005c_e9d0 => extern "thiscall" fn vi_005ce9d0(this: u32, n: u32) -> u32 {
        mb::would_overrun(&mem(), this, n)
    }
    /// `Phyre_PScripting_PAsyncProcessBuffer_unlink`: unlinks `node` from the list at `+0x44`; 0, or 0x13.
    Phyre_PScripting_PAsyncProcessBuffer_unlink @ 0x005c_ea90 => extern "thiscall" fn vi_005cea90(this: u32, node: u32) -> u32 {
        mb::unlink_node(&mut mem(), this, node)
    }
    /// `Phyre_PAnimation_copyConstruct0c_00616430`: placement construction of `n` elements (12-byte elements copied); the original's `eax`.
    Phyre_PAnimation_copyConstruct0c_00616430 @ 0x0061_6430 => extern "C" fn vi_00616430(dst: u32, src: u32, n: u32) -> u32 {
        mb::construct(&mut mem(), const { mi::construct_of(0x0061_6430) }, dst, src, n)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_appendWord`: appends `value` to the list at `+0x110` (a new buffer); 1 on success.
    Phyre_PAnimation_PAnimatableComponent_appendWord @ 0x0061_66b0 => extern "thiscall" fn vi_006166b0(this: u32, value: u32) -> u32 {
        mb::append_word(&mut mem(), this, value)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_appendPair`: appends the pair `(a, b)` to the list at `+0x100` (a new buffer); 1 on success.
    Phyre_PAnimation_PAnimatableComponent_appendPair @ 0x0061_66f0 => extern "thiscall" fn vi_006166f0(this: u32, a: u32, b: u32) -> u32 {
        mb::append_pair(&mut mem(), this, a, b)
    }
    /// `Phyre_PAnimation_PInternal_PAvoidOutputOverwritingInput_visit`: `al` 0 when `key`'s entry lies past `[this + 8]`, else 1.
    Phyre_PAnimation_PInternal_PAvoidOutputOverwritingInput_visit @ 0x0061_6870 => extern "thiscall" fn vi_00616870(this: u32, key: u32, _b: u32) -> u32 {
        mb::may_write(&mut mem(), this, key)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_getPairRecordWord`: through pair `i` at `[this + 0x104]`, its object's record word, or -1.
    Phyre_PAnimation_PAnimatableComponent_getPairRecordWord @ 0x0061_69e0 => extern "thiscall" fn vi_006169e0(this: u32, i: u32) -> u32 {
        mb::pair_record_word(&mem(), this, i)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_getRecordWord`: word 2 of record `i` at `[this + 0x10c]`, or -1.
    Phyre_PAnimation_PAnimatableComponent_getRecordWord @ 0x0061_6a40 => extern "thiscall" fn vi_00616a40(this: u32, i: u32) -> u32 {
        mb::record_word(&mem(), this, i)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_allPairsBound`: whether every pair's object has its `+0x118` set; `al`.
    Phyre_PAnimation_PAnimatableComponent_allPairsBound @ 0x0061_6d10 => extern "thiscall" fn vi_00616d10(this: u32) -> u8 {
        mb::all_bound(&mem(), this)
    }
    /// `Phyre_PAnimation_PNetworkOptimizeProcess_setName`: `strncpy(this, name, 0xff)`, `this[0xff] = 0`; `al = 1`.
    Phyre_PAnimation_PNetworkOptimizeProcess_setName @ 0x0061_6dc0 => extern "thiscall" fn vi_00616dc0(this: u32, name: u32) -> u32 {
        mb::set_name(&mut mem(), this, name)
    }
    /// `Phyre_PAnimation_PAnimationChannelBase_assign`: `operator=`: the words, the name (`PString::Assign`), `+0x18`; `this`.
    Phyre_PAnimation_PAnimationChannelBase_assign @ 0x0050_b730 => extern "thiscall" fn ar_0050b730(this: u32, other: u32) -> u32 {
        ab::assign_channel_base(&mut mem(), this, other)
    }
    /// `Phyre_PAnimation_PAnimationChannelBase_equals`: `operator==`: the words, the names (`0x004ea7a0`), `+0x18`; `al`.
    Phyre_PAnimation_PAnimationChannelBase_equals @ 0x0050_b7b0 => extern "thiscall" fn ar_0050b7b0(this: u32, other: u32) -> u8 {
        ab::channel_base_equals(&mut mem(), this, other)
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationChannel_ptr_allocCopy`: a new array of `total` words: `n` from `src`, the rest zero; 0 for none.
    Phyre_PArray_Phyre_PAnimation_PAnimationChannel_ptr_allocCopy @ 0x0051_3b80 => extern "C" fn ar_00513b80(total: u32, src: u32, n: u32) -> u32 {
        ab::alloc_copy(&mut mem(), ar::Rest::Zero, total, src, n)
    }
    /// `Phyre_PAnimation_PAnimationClip_removeWord`: removes word `i` (the last moved over it), then the resize; 3 out of range.
    Phyre_PAnimation_PAnimationClip_removeWord @ 0x0051_6e00 => extern "thiscall" fn ar_00516e00(this: u32, i: u32) -> u32 {
        ab::remove_word(&mut mem(), this, i)
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationChannel_ptr_resize`: `resize(n)`: a new array keeping the old words, the old owned one freed; 0, or 0xd.
    Phyre_PArray_Phyre_PAnimation_PAnimationChannel_ptr_resize @ 0x0051_6eb0 => extern "thiscall" fn ar_00516eb0(this: u32, n: u32) -> u32 {
        ab::resize(&mut mem(), this, n)
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget_allocCopy`: a new array of `total` targets: `n` copied from `src`, the rest constructed.
    Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget_allocCopy @ 0x0051_9d30 => extern "C" fn ar_00519d30(total: u32, src: u32, n: u32) -> u32 {
        ab::alloc_targets(&mut mem(), total, src, n)
    }
    /// `Phyre_PSharray_storage_0051c270`: `PSharray` storage for `total` words (inline for one): `n` from `src`, the rest zero.
    Phyre_PSharray_storage_0051c270 @ 0x0051_c270 => extern "thiscall" fn ar_0051c270(this: u32, total: u32, src: u32, n: u32) -> u32 {
        ab::sharray_storage(&mut mem(), this, total, src, n)
    }
    /// `Phyre_PAnimation_PAnimationSet_findChannel`: the index of the first 16-byte record with `key`'s channel key, or -1.
    Phyre_PAnimation_PAnimationSet_findChannel @ 0x0051_cb40 => extern "thiscall" fn ar_0051cb40(this: u32, key: u32) -> u32 {
        ab::find_channel(&mem(), this, key)
    }
    /// `Phyre_PAnimation_findChannelBase_0051cbb0`: the index of the first 28-byte channel base equal to `channel`, or -1.
    Phyre_PAnimation_findChannelBase_0051cbb0 @ 0x0051_cbb0 => extern "thiscall" fn ar_0051cbb0(this: u32, channel: u32) -> u32 {
        ab::find_channel_base(&mut mem(), this, channel)
    }
    /// `Phyre_PSharray_set_0051d7b0`: element `i` = `v`; 0, 1 for null `v`, 3 out of range.
    Phyre_PSharray_set_0051d7b0 @ 0x0051_d7b0 => extern "thiscall" fn ar_0051d7b0(this: u32, i: u32, v: u32) -> u32 {
        ab::sharray_set(&mut mem(), this, i, v)
    }
    /// `Phyre_PSharray_Phyre_PAnimation_PAnimationClip_ptr_setArray`: `setArray(els, n)`: the inline storage registered with the type system, else `0x0051dac0`; 0.
    Phyre_PSharray_Phyre_PAnimation_PAnimationClip_ptr_setArray @ 0x0051_dbf0 => extern "thiscall" fn ar_0051dbf0(this: u32, els: u32, n: u32) -> u32 {
        ab::clips_set_array(&mut mem(), this, els, n)
    }
    /// `Phyre_PAnimation_PAnimationEventController_update`: with a time controller and events: the events between `ctx[1]` and now dispatched; 0.
    Phyre_PAnimation_PAnimationEventController_update @ 0x0052_4d80 => extern "thiscall" fn ar_00524d80(this: u32, time: u32, _b: u32, ctx: u32) -> u32 {
        ab::event_update(&mut mem(), this, time, ctx)
    }
    /// `Phyre_PArray_unsigned_int_allocCopy`: a new array of `total` words holding `n` from `src`; 0 for none.
    Phyre_PArray_unsigned_int_allocCopy @ 0x0052_8270 => extern "C" fn ar_00528270(total: u32, src: u32, n: u32) -> u32 {
        ab::alloc_copy(&mut mem(), ar::Rest::Keep, total, src, n)
    }
    /// `Phyre_PAnimation_PAnimationSlotFilter_setupBatch`: the batch descriptor `(0x10, 0, 0, 0, n, slots...)`; the bytes used.
    Phyre_PAnimation_PAnimationSlotFilter_setupBatch @ 0x0052_a6b0 => extern "thiscall" fn ar_0052a6b0(this: u32, _a: u32, desc: u32, _c: u32) -> u32 {
        ab::slot_filter_setup(&mut mem(), this, desc)
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_ptr_allocCopy`: a new array of `total` words: `n` from `src`, the rest zero; 0 for none.
    Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_ptr_allocCopy @ 0x0052_bc40 => extern "C" fn ar_0052bc40(total: u32, src: u32, n: u32) -> u32 {
        ab::alloc_copy(&mut mem(), ar::Rest::Zero, total, src, n)
    }
    /// `Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_ptr_resize`: `resize(n)`: a new array keeping the old words, the old owned one freed; 0, or 0xd.
    Phyre_PArray_Phyre_PAnimation_PAnimationDataSource_ptr_resize @ 0x0052_cde0 => extern "thiscall" fn ar_0052cde0(this: u32, n: u32) -> u32 {
        ab::resize(&mut mem(), this, n)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_isFirstPastEnd`: whether source record 0's value is past its clip's end; `al`.
    Phyre_PAnimation_PAnimatableComponent_isFirstPastEnd @ 0x0054_2ff0 => extern "thiscall" fn ar_00542ff0(this: u32) -> u8 {
        ab::past_end(&mut mem(), this, 0, 4, 0)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_isPastEnd`: whether source record `i`'s value is past its clip's end; `al`.
    Phyre_PAnimation_PAnimatableComponent_isPastEnd @ 0x0054_30a0 => extern "thiscall" fn ar_005430a0(this: u32, i: u32) -> u8 {
        ab::past_end(&mut mem(), this, i, 0xc, 0)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_setSourceTime`: source record `i`'s time = `t`, its controller's change added at `+0x34`; 0, or 3.
    Phyre_PAnimation_PAnimatableComponent_setSourceTime @ 0x0054_4110 => extern "thiscall" fn ar_00544110(this: u32, i: u32, t: u32) -> u32 {
        ab::set_source_time(&mut mem(), this, i, t)
    }
    /// `Phyre_PAnimation_PAnimationSetTargetLookup_dtor_005cc770`: the tree at `+8` released, `PhyreFree([this + 4])`, the tree cleared; its `eax`.
    Phyre_PAnimation_PAnimationSetTargetLookup_dtor_005cc770 @ 0x005c_c770 => extern "thiscall" fn ar_005cc770(this: u32) -> u32 {
        ab::lookup_dtor(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PAnimationSetTargetLookup_find`: the hashed lookup of `key`'s channel key (the set's search without a table).
    Phyre_PAnimation_PAnimationSetTargetLookup_find @ 0x005c_c800 => extern "thiscall" fn ar_005cc800(this: u32, key: u32) -> u32 {
        ab::lookup_channel(&mut mem(), this, key)
    }
    /// `Phyre_PAnimation_PAnimationSetTargetLookup_dtor_005ccc50`: the tree at `+8` released, `PhyreFree([this + 4])`, the tree cleared; its `eax`.
    Phyre_PAnimation_PAnimationSetTargetLookup_dtor_005ccc50 @ 0x005c_cc50 => extern "thiscall" fn ar_005ccc50(this: u32) -> u32 {
        ab::lookup_dtor(&mut mem(), this)
    }
    /// `Phyre_PScripting_PAsyncProcessBuffer_allocateBlock`: a block of `bytes + 0x14` from the ring at `+0x14` with its id and header; its data, or 0.
    Phyre_PScripting_PAsyncProcessBuffer_allocateBlock @ 0x005c_e930 => extern "thiscall" fn ar_005ce930(this: u32, bytes: u32) -> u32 {
        ab::allocate_block(&mut mem(), this, bytes)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_appendRecord`: appends the record `(a, 0, -1)` to the list at `+0x108`; 1 on success.
    Phyre_PAnimation_PAnimatableComponent_appendRecord @ 0x0061_6740 => extern "thiscall" fn ar_00616740(this: u32, a: u32) -> u32 {
        ab::append_record(&mut mem(), this, a)
    }
    /// `Phyre_PAnimation_PInternal_PAvoidOutputOverwritingInput_find`: the first bound record whose word 2 is `key`, or 0.
    Phyre_PAnimation_PInternal_PAvoidOutputOverwritingInput_find @ 0x0061_6cb0 => extern "thiscall" fn ar_00616cb0(this: u32, key: u32) -> u32 {
        ab::find_bound(&mem(), this, key)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_resetRecords`: `[this + 0x118] = -1`; every record's word 2 = -1, word 1 = 0.
    Phyre_PAnimation_PAnimatableComponent_resetRecords @ 0x0061_6d60 => extern "thiscall" fn ar_00616d60(this: u32) {
        ab::reset_records(&mut mem(), this);
    }
    /// `Phyre_PAnimation_PAnimationChannelTargetBindScene_treeArray`: `n` empty 24-byte tree headers (`PhyreMalloc`); `this`.
    Phyre_PAnimation_PAnimationChannelTargetBindScene_treeArray @ 0x0061_7170 => extern "thiscall" fn ar_00617170(this: u32, n: u32) -> u32 {
        ab::tree_array(&mut mem(), this, n)
    }
    /// `graphicPostCommand_00642b10`: posts `(object, params[0..16])` to the render command queue.
    graphicPostCommand_00642b10 @ 0x0064_2b10 => extern "C" fn ar_00642b10(object: u32, params: u32) -> u32 {
        ab::post_command(&mut mem(), object, params)
    }
}
