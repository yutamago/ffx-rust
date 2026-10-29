//! M5 stage 1, list `phyre-anim-left` (`ffx_panim::m5p_anim_left*`): the
//! `PAnimation` functions the first anim round left (the larger ones).
//! Tests: `crates/ffx-oracle/tests/m5p_anim_left.rs`.
//!
//! Threads: animation batches run on the worker threads, the rest where
//! clusters load and bind. No shim keeps state; each works on the game's
//! memory through [`mem`].

use super::{mem, ported};
use ffx_panim::m5p_anim_left_a::bridge as la;
use ffx_panim::m5p_anim_left_b::bridge as lb;
use ffx_panim::m5p_anim_left_c::bridge as lc;
use ffx_panim::m5p_anim_left_d::bridge as ld;
use ffx_panim::m5p_anim_left_e::bridge as le;
use ffx_panim::m5p_anim_left_f::bridge as lf;
use ffx_panim::m5p_anim_left_g::bridge as lg;
use ffx_panim::m5p_anim_left_h::bridge as lh;
use ffx_panim::m5p_anim_left_i::bridge as li;
use ffx_panim::m5p_anim_left_j::bridge as lj;
use ffx_panim::m5p_anim_left_k::bridge as lk;
use ffx_panim::m5p_anim_left_l::bridge as ll;

ported! {
    /// `Phyre_PUtilityGeometry_findNameIndex_00493f50`: the index of `name` among the names at `[this + 0x24]`, -1 when absent.
    Phyre_PUtilityGeometry_findNameIndex_00493f50 @ 0x0049_3f50 => extern "thiscall" fn la_00493f50(this: u32, name: u32) -> u32 {
        la::find_name(&mem(), this, name)
    }
    /// `Phyre_PAnimationChannelTargetBindDataMember_setTarget_0050c850`: the member target set (name copied, state 2); 0, 1 or 0xd.
    Phyre_PAnimationChannelTargetBindDataMember_setTarget_0050c850 @ 0x0050_c850 => extern "thiscall" fn la_0050c850(this: u32, descriptor: u32, obj: u32, name: u32, index: u32) -> u32 {
        la::set_target(&mut mem(), this, descriptor, obj, name, index)
    }
    /// `Phyre_PAnimationChannelTargetBindDataMember_bindCurrentPose_00432e90`: rebinds a member target to the owner's `m_currentPose`.
    Phyre_PAnimationChannelTargetBindDataMember_bindCurrentPose_00432e90 @ 0x0043_2e90 => extern "C" fn la_00432e90(bind: u32, owner: u32) -> u32 {
        la::bind_current_pose(&mut mem(), bind, owner)
    }
    /// `Phyre_PUtilityAnimation_resetStaticArray_0053a070`: empties the static array and releases its backing store; 0.
    Phyre_PUtilityAnimation_resetStaticArray_0053a070 @ 0x0053_a070 => extern "C" fn la_0053a070() -> u32 {
        la::reset_static(&mut mem())
    }
    /// `Phyre_PUtilityAnimation_shutdown_00433250`: the free list, the static array, the channel-target type records unlinked.
    Phyre_PUtilityAnimation_shutdown_00433250 @ 0x0043_3250 => extern "C" fn la_00433250() -> u32 {
        la::shutdown(&mut mem())
    }
    /// `Phyre_PArray_PMeshSegment_copyCtor_00486ff0`: copy constructor of a `PArray` of `0x14`-byte elements with one in place; `this`.
    Phyre_PArray_PMeshSegment_copyCtor_00486ff0 @ 0x0048_6ff0 => extern "thiscall" fn la_00486ff0(this: u32, src: u32) -> u32 {
        la::small_array_copy(&mut mem(), this, src)
    }
    /// `Phyre_PAnimationChannel_compareBatch_00508ba0`: channel batch order, highest first (-1, 0, 1).
    Phyre_PAnimationChannel_compareBatch_00508ba0 @ 0x0050_8ba0 => extern "C" fn la_00508ba0(a: u32, b: u32) -> u32 {
        la::compare_batch(&mem(), a, b)
    }
    /// `Phyre_PArray_float_copyConstruct_00508f00`: placement copy of `n` floats; the destination's end.
    Phyre_PArray_float_copyConstruct_00508f00 @ 0x0050_8f00 => extern "C" fn la_00508f00(dst: u32, src: u32, n: u32) -> u32 {
        la::copy_floats(&mut mem(), dst, src, n)
    }
    /// `Phyre_PAnimationChannelTargetBind_assignNameRange_0050c7e0`: the name at `this` set to `begin..end`; 0, 1, 5 or 0xd.
    Phyre_PAnimationChannelTargetBind_assignNameRange_0050c7e0 @ 0x0050_c7e0 => extern "thiscall" fn la_0050c7e0(this: u32, begin: u32, end: u32) -> u32 {
        la::assign_range(&mut mem(), this, begin, end)
    }
    /// `Phyre_PAnimation_lerp4_0050eb70`: `out = (b - a) * t + a` per component; `out`.
    Phyre_PAnimation_lerp4_0050eb70 @ 0x0050_eb70 => extern "C" fn la_0050eb70(out: u32, t: u32, a: u32, b: u32) -> u32 {
        la::lerp4(&mut mem(), out, t, a, b)
    }
    /// `Phyre_PAnimation_lerp4_0050ebf0`: a copy of `0x0050eb70`.
    Phyre_PAnimation_lerp4_0050ebf0 @ 0x0050_ebf0 => extern "C" fn la_0050ebf0(out: u32, t: u32, a: u32, b: u32) -> u32 {
        la::lerp4(&mut mem(), out, t, a, b)
    }
    /// `Phyre_PAnimation_slerp_0050ed70`: quaternion slerp (linear past the threshold); `out`.
    Phyre_PAnimation_slerp_0050ed70 @ 0x0050_ed70 => extern "C" fn la_0050ed70(out: u32, t: u32, a: u32, b: u32, threshold: u32) -> u32 {
        la::slerp(&mut mem(), out, t, a, b, threshold)
    }
    /// `Phyre_PAnimationSet_countBatchKeys_00516190`: how many distinct batch sort keys the channels have.
    Phyre_PAnimationSet_countBatchKeys_00516190 @ 0x0051_6190 => extern "thiscall" fn la_00516190(this: u32) -> u32 {
        la::distinct_keys(&mut mem(), this)
    }
    /// `Phyre_PAnimationSetTargetLookup_compareKeys_005cccd0`: the target-key order (-1, 0, 1).
    Phyre_PAnimationSetTargetLookup_compareKeys_005cccd0 @ 0x005c_ccd0 => extern "C" fn la_005cccd0(a: u32, b: u32) -> u32 {
        la::compare_keys(&mem(), a, b)
    }
    /// `Phyre_PAnimationSetTargetLookup_treeInsert_005cc9f0`: the lookup tree's insert; the node with an equal key, or 0.
    Phyre_PAnimationSetTargetLookup_treeInsert_005cc9f0 @ 0x005c_c9f0 => extern "thiscall" fn la_005cc9f0(this: u32, node: u32) -> u32 {
        la::tree_insert(&mut mem(), this, node)
    }
    /// `Phyre_PAnimation_PAnimationSetTargetLookup_PAnimationSetTargetLookup`: the constructor (a tree of the set's targets); `this`.
    Phyre_PAnimation_PAnimationSetTargetLookup_PAnimationSetTargetLookup @ 0x005c_ca80 => extern "thiscall" fn la_005cca80(this: u32, set: u32) -> u32 {
        la::lookup_ctor(&mut mem(), this, set)
    }
    /// `Phyre_PAnimationSetTargetLookup_find_005cc870`: the node with `key`, or 0.
    Phyre_PAnimationSetTargetLookup_find_005cc870 @ 0x005c_c870 => extern "thiscall" fn la_005cc870(this: u32, key: u32) -> u32 {
        la::tree_find(&mem(), this, key)
    }
    /// `Phyre_PAnimationConstantChannel_copyConstructN_00511b40`: placement copy of `n` `0x34`-byte channels.
    Phyre_PAnimationConstantChannel_copyConstructN_00511b40 @ 0x0051_1b40 => extern "C" fn la_00511b40(dst: u32, src: u32, n: u32) {
        la::copy_channels(&mut mem(), dst, src, n);
    }
    /// `Phyre_PAnimationConstantChannel_constructN_00515880`: placement construction of `n` `0x34`-byte channels.
    Phyre_PAnimationConstantChannel_constructN_00515880 @ 0x0051_5880 => extern "C" fn la_00515880(dst: u32, n: u32) {
        la::construct_channels(&mut mem(), dst, n);
    }
    /// `Phyre_PAnimationSet_rebuild_00516240`: the set's data object recreated in `cluster`; the object.
    Phyre_PAnimationSet_rebuild_00516240 @ 0x0051_6240 => extern "thiscall" fn la_00516240(this: u32, cluster: u32, arg: u32) -> u32 {
        la::rebuild(&mut mem(), this, cluster, arg)
    }
    /// `Phyre_PAnimationSet_resizeWords_00516fe0`: the word array at `+4` resized; 0 or 0xd.
    Phyre_PAnimationSet_resizeWords_00516fe0 @ 0x0051_6fe0 => extern "thiscall" fn la_00516fe0(this: u32, n: u32) -> u32 {
        la::resize_words(&mut mem(), this, n)
    }
    /// `Phyre_PAnimationSet_resizeConstantChannels_005170c0`: the channel array at `+0xc` resized; 0 or 0xd.
    Phyre_PAnimationSet_resizeConstantChannels_005170c0 @ 0x0051_70c0 => extern "thiscall" fn la_005170c0(this: u32, n: u32) -> u32 {
        la::resize_set_channels(&mut mem(), this, n)
    }
    /// `Phyre_PArray_PAnimationConstantChannel_resize_00516f20`: the channel array resized; 0 or 0xd.
    Phyre_PArray_PAnimationConstantChannel_resize_00516f20 @ 0x0051_6f20 => extern "thiscall" fn la_00516f20(this: u32, n: u32) -> u32 {
        la::resize_channels(&mut mem(), this, n)
    }
    /// `Phyre_PAnimationSetTarget_copyConstructN_00517800`: placement copy of `n` `0x1c`-byte named targets.
    Phyre_PAnimationSetTarget_copyConstructN_00517800 @ 0x0051_7800 => extern "C" fn la_00517800(dst: u32, src: u32, n: u32) {
        la::copy_targets(&mut mem(), dst, src, n);
    }
    // batch 2
    /// `Phyre_PAnimationEventController_dispatchEvents_005207c0`: the events between `from` and `to` (wrapping) handed to the callback; 1 without one, else 0.
    Phyre_PAnimationEventController_dispatchEvents_005207c0 @ 0x0052_07c0 => extern "thiscall" fn lb_005207c0(this: u32, from: u32, to: u32, callback: u32, ctx: u32) -> u32 {
        lb::dispatch_events(&mut mem(), this, from, to, callback, ctx)
    }
    /// `Phyre_PTimeIntervalController_getTime_00522cd0`: the source's time wrapped into the interval (`_CIfmod`), in `ST0`.
    Phyre_PTimeIntervalController_getTime_00522cd0 @ 0x0052_2cd0 => extern "thiscall" fn lb_00522cd0(this: u32) -> f32 {
        lb::interval_time(&mut mem(), this)
    }
    /// `Phyre_PAnimation_findKeyIndex_005253c0`: the last key time not after `value` (binary search).
    Phyre_PAnimation_findKeyIndex_005253c0 @ 0x0052_53c0 => extern "C" fn lb_005253c0(keys: u32, n: u32, value: u32) -> u32 {
        lb::key_search(&mem(), keys, n, value)
    }
    /// `Phyre_PAnimationController_getMaxBatchSize_005262c0`: the largest batch size of the clips.
    Phyre_PAnimationController_getMaxBatchSize_005262c0 @ 0x0052_62c0 => extern "thiscall" fn lb_005262c0(this: u32) -> u32 {
        lb::max_batch_size(&mem(), this)
    }
    /// `Phyre_PAnimationController_writeBatchHeader_00526480`: the controller's batch header; 0x30.
    Phyre_PAnimationController_writeBatchHeader_00526480 @ 0x0052_6480 => extern "thiscall" fn lb_00526480(this: u32, a: u32, out: u32, b: u32) -> u32 {
        let _ = (a, b);
        lb::controller_header(&mut mem(), this, out)
    }
    /// `Phyre_PAnimationBlenderController_scalar_deleting_dtor_0052bb60`: sources freed, the data-source destructor, `PhyreFree` on `flags & 1`; `this`.
    Phyre_PAnimationBlenderController_scalar_deleting_dtor_0052bb60 @ 0x0052_bb60 => extern "thiscall" fn lb_0052bb60(this: u32, flags: u32) -> u32 {
        lb::blender_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PAnimationBlenderController_runBatches_0052c8f0`: each batch's sources handed to the blend kernel (the list in this frame); 0.
    Phyre_PAnimationBlenderController_runBatches_0052c8f0 @ 0x0052_c8f0 => extern "C" fn lb_0052c8f0(out: u32, sources: u32, n: u32, a: u32, b: u32, batches: u32, kernel: u32) -> u32 {
        let mut f = super::StackFrame::<{ ffx_panim::m5p_anim_left_b::LIST_WORDS as usize }>::new();
        lb::blend_batches(&mut mem(), out, sources, n, a, b, batches, kernel, f.at())
    }
    /// `Phyre_PAnimationBlenderController_resizeSources_0052cf10`: the sources array resized; 0 or 0xd.
    Phyre_PAnimationBlenderController_resizeSources_0052cf10 @ 0x0052_cf10 => extern "thiscall" fn lb_0052cf10(this: u32, n: u32) -> u32 {
        lb::resize_sources(&mut mem(), this, n)
    }
    /// `Phyre_PAnimationBlenderController_visit_0052d180`: the visitor over the blender and its sources; `al`.
    Phyre_PAnimationBlenderController_visit_0052d180 @ 0x0052_d180 => extern "thiscall" fn lb_0052d180(this: u32, visitor: u32, ctx: u32) -> u32 {
        lb::blender_visit(&mut mem(), this, visitor, ctx)
    }
    /// `Phyre_PAnimationWeightedBlenderController_markUsed_0052e2f0`: slots with a non-zero weight marked.
    Phyre_PAnimationWeightedBlenderController_markUsed_0052e2f0 @ 0x0052_e2f0 => extern "thiscall" fn lb_0052e2f0(this: u32, flags: u32, ids: u32, n: u32) {
        lb::mark_weighted(&mut mem(), this, flags, ids, n);
    }
    /// `Phyre_PAnimationAdditiveBlenderController_markUsed_0052f8a0`: the base slot and the weighted ones marked.
    Phyre_PAnimationAdditiveBlenderController_markUsed_0052f8a0 @ 0x0052_f8a0 => extern "thiscall" fn lb_0052f8a0(this: u32, flags: u32, ids: u32, n: u32) {
        lb::mark_additive(&mut mem(), this, flags, ids, n);
    }
    /// `Phyre_PAnimationWeightedBlenderController_writeBatchData_0052e3f0`: header and weights; the size.
    Phyre_PAnimationWeightedBlenderController_writeBatchData_0052e3f0 @ 0x0052_e3f0 => extern "thiscall" fn lb_0052e3f0(this: u32, a: u32, out: u32, b: u32) -> u32 {
        let _ = (a, b);
        lb::weight_batch_data(&mut mem(), this, out)
    }
    /// `Phyre_PAnimationAdditiveBlenderController_writeBatchData_0052fa90`: a copy of `0x0052e3f0`.
    Phyre_PAnimationAdditiveBlenderController_writeBatchData_0052fa90 @ 0x0052_fa90 => extern "thiscall" fn lb_0052fa90(this: u32, a: u32, out: u32, b: u32) -> u32 {
        let _ = (a, b);
        lb::weight_batch_data(&mut mem(), this, out)
    }
    /// `Phyre_PAnimationWeightedBlenderController_resize_0052e550`: sources and weights resized; 0 or 0xd.
    Phyre_PAnimationWeightedBlenderController_resize_0052e550 @ 0x0052_e550 => extern "thiscall" fn lb_0052e550(this: u32, n: u32) -> u32 {
        lb::resize_weights(&mut mem(), this, n)
    }
    /// `Phyre_PAnimationAdditiveBlenderController_resize_0052fbf0`: a copy of `0x0052e550`.
    Phyre_PAnimationAdditiveBlenderController_resize_0052fbf0 @ 0x0052_fbf0 => extern "thiscall" fn lb_0052fbf0(this: u32, n: u32) -> u32 {
        lb::resize_weights(&mut mem(), this, n)
    }
    /// `Phyre_Vectormath_Quat_mul_0052ecb0`: `out = this * q`; `out`.
    Phyre_Vectormath_Quat_mul_0052ecb0 @ 0x0052_ecb0 => extern "thiscall" fn lb_0052ecb0(this: u32, out: u32, q: u32) -> u32 {
        lb::quat_mul(&mut mem(), this, out, q)
    }
    /// `Phyre_Vectormath_Quat_normalize_0052fa10`: `out = q / |q|`; `out`.
    Phyre_Vectormath_Quat_normalize_0052fa10 @ 0x0052_fa10 => extern "C" fn lb_0052fa10(out: u32, q: u32) -> u32 {
        lb::quat_normalize(&mut mem(), out, q)
    }
    /// `Phyre_PAnimationSpuTargetBlenderController_writeBatchHeader_00530750`: the header with the rotation/scale/translation types; the size.
    Phyre_PAnimationSpuTargetBlenderController_writeBatchHeader_00530750 @ 0x0053_0750 => extern "thiscall" fn lb_00530750(this: u32, a: u32, out: u32, b: u32) -> u32 {
        let _ = (a, b);
        lb::target_header(&mut mem(), this, out)
    }
    /// `Phyre_PAnimationTargetBlenderController_log_00530b20`: `(obj, fmt, ...)` handed to the logger at `+0x14` with a `va_list` (the variable words at `slots + 8`).
    Phyre_PAnimationTargetBlenderController_log_00530b20 @ 0x0053_0b20 => extern "C" fn lb_00530b20(obj: u32, fmt: u32; slots: u32) -> u32 {
        let mut f = super::StackFrame::<1>::new();
        lb::log(&mut mem(), obj, fmt, slots + 8, f.at())
    }
    /// `Phyre_Vectormath_Matrix3_fromQuat_00530c30`: the rotation matrix of `q` at `this`; `this`.
    Phyre_Vectormath_Matrix3_fromQuat_00530c30 @ 0x0053_0c30 => extern "thiscall" fn lb_00530c30(this: u32, q: u32) -> u32 {
        lb::quat_matrix(&mut mem(), this, q)
    }
    // batch 3
    /// `Phyre_PAnimatableComponent_resizeTargets_0053bde0`: fresh `0x18`-byte elements (`0x00539550`); 0 or 0xd.
    Phyre_PAnimatableComponent_resizeTargets_0053bde0 @ 0x0053_bde0 => extern "thiscall" fn lc_0053bde0(this: u32, n: u32) -> u32 {
        lc::fresh_resize(&mut mem(), const { ffx_panim::m5p_anim_left_c::fresh_of(0x0053_bde0) }, this, n)
    }
    /// `Phyre_PAnimatableComponent_resizeVectors_0053bf50`: fresh 16-byte aligned elements (`0x00539600`); 0 or 0xd.
    Phyre_PAnimatableComponent_resizeVectors_0053bf50 @ 0x0053_bf50 => extern "thiscall" fn lc_0053bf50(this: u32, n: u32) -> u32 {
        lc::fresh_resize(&mut mem(), const { ffx_panim::m5p_anim_left_c::fresh_of(0x0053_bf50) }, this, n)
    }
    /// `Phyre_PAnimatableComponent_constructRecords_00539480`: `n` `0x94`-byte records zeroed.
    Phyre_PAnimatableComponent_constructRecords_00539480 @ 0x0053_9480 => extern "C" fn lc_00539480(dst: u32, n: u32) {
        lc::construct_records(&mut mem(), dst, n);
    }
    /// `Phyre_PAnimatableComponent_resizeRecords_0053bfd0`: the `0x94`-byte record array resized; 0 or 0xd.
    Phyre_PAnimatableComponent_resizeRecords_0053bfd0 @ 0x0053_bfd0 => extern "thiscall" fn lc_0053bfd0(this: u32, n: u32) -> u32 {
        lc::resize_records(&mut mem(), this, n)
    }
    /// `Phyre_PArray_appendWord_00616b00`: one word appended into the caller's storage; 0 or 0xd.
    Phyre_PArray_appendWord_00616b00 @ 0x0061_6b00 => extern "thiscall" fn lc_00616b00(this: u32, new: u32, value: u32) -> u32 {
        lc::append(&mut mem(), 1, this, new, value)
    }
    /// `Phyre_PArray_appendWord_0053b250`: a copy of `0x00616b00`.
    Phyre_PArray_appendWord_0053b250 @ 0x0053_b250 => extern "thiscall" fn lc_0053b250(this: u32, new: u32, value: u32) -> u32 {
        lc::append(&mut mem(), 1, this, new, value)
    }
    /// `Phyre_PArray_appendPair_00616b90`: one pair appended; 0 or 0xd.
    Phyre_PArray_appendPair_00616b90 @ 0x0061_6b90 => extern "thiscall" fn lc_00616b90(this: u32, new: u32, value: u32) -> u32 {
        lc::append(&mut mem(), 2, this, new, value)
    }
    /// `Phyre_PArray_appendTriple_00616c30`: one triple appended (the old ones by `0x00616430`); 0 or 0xd.
    Phyre_PArray_appendTriple_00616c30 @ 0x0061_6c30 => extern "thiscall" fn lc_00616c30(this: u32, new: u32, value: u32) -> u32 {
        lc::append_triple(&mut mem(), this, new, value)
    }
    /// `Phyre_PAnimatableComponent_appendObject_0053b370`: one 8-byte object (vtable `0x00b2a340`) appended; 0 or 0xd.
    Phyre_PAnimatableComponent_appendObject_0053b370 @ 0x0053_b370 => extern "thiscall" fn lc_0053b370(this: u32, new: u32, value: u32) -> u32 {
        lc::append_object(&mut mem(), const { ffx_panim::m5p_anim_left_c::object_append_of(0x0053_b370) }, this, new, value)
    }
    /// `Phyre_PAnimatableComponent_appendObject_0053b4a0`: as `0x0053b370` with vtable `0x00b2a358`.
    Phyre_PAnimatableComponent_appendObject_0053b4a0 @ 0x0053_b4a0 => extern "thiscall" fn lc_0053b4a0(this: u32, new: u32, value: u32) -> u32 {
        lc::append_object(&mut mem(), const { ffx_panim::m5p_anim_left_c::object_append_of(0x0053_b4a0) }, this, new, value)
    }
    /// `Phyre_PAnimatableComponent_appendObject_0053b540`: as `0x0053b370` with vtable `0x00b2a364`.
    Phyre_PAnimatableComponent_appendObject_0053b540 @ 0x0053_b540 => extern "thiscall" fn lc_0053b540(this: u32, new: u32, value: u32) -> u32 {
        lc::append_object(&mut mem(), const { ffx_panim::m5p_anim_left_c::object_append_of(0x0053_b540) }, this, new, value)
    }
    /// `Phyre_PAnimatableComponent_appendObject_0053b5e0`: as `0x0053b370` with vtable `0x00b2a334`.
    Phyre_PAnimatableComponent_appendObject_0053b5e0 @ 0x0053_b5e0 => extern "thiscall" fn lc_0053b5e0(this: u32, new: u32, value: u32) -> u32 {
        lc::append_object(&mut mem(), const { ffx_panim::m5p_anim_left_c::object_append_of(0x0053_b5e0) }, this, new, value)
    }
    /// `Phyre_PAnimationNetworkOptimiser_linksResolved_006168a0`: whether every owner listing `(this, index)` is resolved; `al`.
    Phyre_PAnimationNetworkOptimiser_linksResolved_006168a0 @ 0x0061_68a0 => extern "thiscall" fn lc_006168a0(this: u32, index: u32) -> u32 {
        lc::links_resolved(&mem(), this, index)
    }
    /// `Phyre_PAnimationNetworkOptimiser_releaseResolved_00616930`: resolved entries released (state 2); `al` 1.
    Phyre_PAnimationNetworkOptimiser_releaseResolved_00616930 @ 0x0061_6930 => extern "thiscall" fn lc_00616930(this: u32, arg: u32) -> u32 {
        lc::release_resolved(&mut mem(), this, arg)
    }
    /// `Phyre_PAnimationNetworkOptimiser_resolveEntries_006167a0`: each entry resolved through a context in this frame (state 1); `al` 1.
    Phyre_PAnimationNetworkOptimiser_resolveEntries_006167a0 @ 0x0061_67a0 => extern "thiscall" fn lc_006167a0(this: u32, arg: u32) -> u32 {
        let mut f = super::StackFrame::<3>::new();
        lc::resolve_entries(&mut mem(), this, arg, f.at())
    }
    /// `Phyre_PAnimationNetworkOptimiser_linkNode_005cd920`: node `i` set up, linked and given outputs; `al`.
    Phyre_PAnimationNetworkOptimiser_linkNode_005cd920 @ 0x005c_d920 => extern "thiscall" fn lc_005cd920(this: u32, i: u32, a: u32, n: u32, pairs: u32, m: u32, words: u32) -> u32 {
        lc::link_node(&mut mem(), this, i, a, n, pairs, m, words)
    }
    /// `Phyre_PAnimationNetworkOptimiser_makeNodes_005cd9e0`: the node table remade with `n` nodes; 1 or 0.
    Phyre_PAnimationNetworkOptimiser_makeNodes_005cd9e0 @ 0x005c_d9e0 => extern "thiscall" fn lc_005cd9e0(this: u32, n: u32) -> u32 {
        lc::make_nodes(&mut mem(), this, n)
    }
    /// `Phyre_PAnimatableComponent_bindSourceRecord_00543330`: a source record bound to `clip` (the clip's range in this frame); the failing call's `eax` or the last one's.
    Phyre_PAnimatableComponent_bindSourceRecord_00543330 @ 0x0054_3330 => extern "thiscall" fn lc_00543330(rec: u32, clip: u32) -> u32 {
        let mut f = super::StackFrame::<2>::new();
        lc::bind_record(&mut mem(), rec, clip, f.at())
    }
    /// `Phyre_PAnimatableComponent_setSource_00543480`: source `i` set to `clip`; 0, 1, 3 or the bind's error.
    Phyre_PAnimatableComponent_setSource_00543480 @ 0x0054_3480 => extern "thiscall" fn lc_00543480(this: u32, i: u32, clip: u32) -> u32 {
        let mut f = super::StackFrame::<2>::new();
        lc::set_source(&mut mem(), this, i, clip, f.at())
    }
    /// `Phyre_PAnimatableComponent_setTimes_00544040`: every source record's time set to `t`.
    Phyre_PAnimatableComponent_setTimes_00544040 @ 0x0054_4040 => extern "thiscall" fn lc_00544040(this: u32, t: u32) {
        lc::set_times(&mut mem(), this, t);
    }
    // batch 4
    /// `Phyre_PAnimationSetTarget_constructN_0051bb80`: `n` `0x1c`-byte named targets with the default name.
    Phyre_PAnimationSetTarget_constructN_0051bb80 @ 0x0051_bb80 => extern "C" fn ld_0051bb80(dst: u32, n: u32) {
        ld::construct_named(&mut mem(), dst, n);
    }
    /// `Phyre_PSharray_resize_0051d720`: a word array with one element in place resized; 0 or 0xd.
    Phyre_PSharray_resize_0051d720 @ 0x0051_d720 => extern "thiscall" fn ld_0051d720(this: u32, n: u32) -> u32 {
        ld::small_resize(&mut mem(), this, n)
    }
    /// `Phyre_PSharray_setToExternalBuffer_0051dac0`: the same array set to an external buffer; 0 or 5.
    Phyre_PSharray_setToExternalBuffer_0051dac0 @ 0x0051_dac0 => extern "thiscall" fn ld_0051dac0(this: u32, src: u32, n: u32) -> u32 {
        ld::small_set_external(&mut mem(), this, src, n)
    }
    /// `Phyre_PArray_PAnimationSlotListIndex_resize_0051d670`: 16-byte entries resized (`0x00517900`, `0x0051bc30`); 0 or 0xd.
    Phyre_PArray_PAnimationSlotListIndex_resize_0051d670 @ 0x0051_d670 => extern "thiscall" fn ld_0051d670(this: u32, n: u32) -> u32 {
        ld::copy_resize(&mut mem(), const { ffx_panim::m5p_anim_left_d::copy_resize_of(0x0051_d670) }, this, n)
    }
    /// `Phyre_PAnimatableComponent_resizeSourceRecords_00543bb0`: `0x4c`-byte source records resized (`0x0053d210`, `0x00541780`, `0x00540b80`); 0 or 0xd.
    Phyre_PAnimatableComponent_resizeSourceRecords_00543bb0 @ 0x0054_3bb0 => extern "thiscall" fn ld_00543bb0(this: u32, n: u32) -> u32 {
        ld::copy_resize(&mut mem(), const { ffx_panim::m5p_anim_left_d::copy_resize_of(0x0054_3bb0) }, this, n)
    }
    /// `Phyre_PAnimationSet_findClip_0051ca40`: the clip named `name` (exact, then contained); 0 for none.
    Phyre_PAnimationSet_findClip_0051ca40 @ 0x0051_ca40 => extern "thiscall" fn ld_0051ca40(this: u32, name: u32) -> u32 {
        ld::find_clip(&mut mem(), this, name)
    }
    /// `Phyre_Vectormath_Matrix3_scaleRows_00531310`: `out` rows = `m` rows times `s`; `out`.
    Phyre_Vectormath_Matrix3_scaleRows_00531310 @ 0x0053_1310 => extern "C" fn ld_00531310(out: u32, m: u32, s: u32) -> u32 {
        ld::scale_rows(&mut mem(), out, m, s)
    }
    /// `Phyre_PAnimatableComponent_clearData_0053d100`: the component data's arrays released; 0.
    Phyre_PAnimatableComponent_clearData_0053d100 @ 0x0053_d100 => extern "thiscall" fn ld_0053d100(this: u32) -> u32 {
        ld::clear_data(&mut mem(), this)
    }
    /// `Phyre_PAnimatableComponent_copySourceRecords_0053d210`: placement copy of `n` `0x4c`-byte source records.
    Phyre_PAnimatableComponent_copySourceRecords_0053d210 @ 0x0053_d210 => extern "C" fn ld_0053d210(dst: u32, src: u32, n: u32) {
        ld::copy_records(&mut mem(), dst, src, n);
    }
    /// `Phyre_PAnimationNetworkOptimiser_moveToEnd_005cdad0`: `key` moved to (or added at) the end of the records; `al` 1.
    Phyre_PAnimationNetworkOptimiser_moveToEnd_005cdad0 @ 0x005c_dad0 => extern "thiscall" fn ld_005cdad0(this: u32, key: u32, unused: u32) -> u32 {
        let _ = unused;
        ld::move_to_end(&mut mem(), this, key)
    }
    /// `Phyre_PAnimationChannelTargetBindShaderParameter_bindTargets_005cde50`: shader-parameter targets bound; 0 or 1.
    Phyre_PAnimationChannelTargetBindShaderParameter_bindTargets_005cde50 @ 0x005c_de50 => extern "thiscall" fn ld_005cde50(this: u32, out: u32, targets: u32, n: u32) -> u32 {
        ld::bind_shader_params(&mut mem(), this, out, targets, n)
    }
    /// `Phyre_PAnimationChannelTargetBindScene_compareKeys_00617260`: the scene bind's key order (-1, 0, 1).
    Phyre_PAnimationChannelTargetBindScene_compareKeys_00617260 @ 0x0061_7260 => extern "C" fn ld_00617260(a: u32, b: u32) -> u32 {
        ld::compare_scene_keys(&mem(), a, b)
    }
    /// `Phyre_PAnimationChannelTargetBindScene_treeInsert_006170e0`: the scene bind tree's insert; the node with an equal key, or 0.
    Phyre_PAnimationChannelTargetBindScene_treeInsert_006170e0 @ 0x0061_70e0 => extern "thiscall" fn ld_006170e0(this: u32, node: u32) -> u32 {
        ld::scene_tree_insert(&mut mem(), this, node)
    }
    /// `FFXApplication_inverseBlend_00a2f560`: `b`, or the lerp from `a` to `b` by `f(a) / (f(a) - f(b))`; `out`.
    FFXApplication_inverseBlend_00a2f560 @ 0x00a2_f560 => extern "C" fn ld_00a2f560(out: u32, b: u32, a: u32, f: u32) -> u32 {
        ld::inverse_blend(&mut mem(), out, b, a, f)
    }
    /// `Phyre_PAnimatableComponent_setClipSet_00543c60`: the component's clip set changed and its sources re-bound; 0, 7 or an error.
    Phyre_PAnimatableComponent_setClipSet_00543c60 @ 0x0054_3c60 => extern "thiscall" fn ld_00543c60(this: u32, set: u32) -> u32 {
        ld::set_clip_set(&mut mem(), this, set)
    }
    /// `Phyre_PAnimatableComponent_setSourceCount_00543e50`: the component's source count set; 0 or an error.
    Phyre_PAnimatableComponent_setSourceCount_00543e50 @ 0x0054_3e50 => extern "thiscall" fn ld_00543e50(this: u32, n: u32) -> u32 {
        ld::set_source_count(&mut mem(), this, n)
    }
    // batch 5
    /// `Phyre_PAnimation_PAnimationChannelTargetBindFunction_bindTargets`: bound-function targets bound (state 3); 0 or 1.
    Phyre_PAnimation_PAnimationChannelTargetBindFunction_bindTargets @ 0x005c_df50 => extern "thiscall" fn le_005cdf50(this: u32, out: u32, targets: u32, n: u32) -> u32 {
        le::bind_callables(&mut mem(), const { ffx_panim::m5p_anim_left_e::callable_bind_of(0x005c_df50) }, this, out, targets, n)
    }
    /// `Phyre_PAnimation_PAnimationChannelTargetBindMethod_bindTargets`: bound-method targets bound (state 2, with the object); 0 or 1.
    Phyre_PAnimation_PAnimationChannelTargetBindMethod_bindTargets @ 0x005c_e050 => extern "thiscall" fn le_005ce050(this: u32, out: u32, targets: u32, n: u32) -> u32 {
        le::bind_callables(&mut mem(), const { ffx_panim::m5p_anim_left_e::callable_bind_of(0x005c_e050) }, this, out, targets, n)
    }
    /// `Phyre_PAnimationChannelTargetBindScene_find_00616fd0`: the scene bind tree's node with `key`, or 0.
    Phyre_PAnimationChannelTargetBindScene_find_00616fd0 @ 0x0061_6fd0 => extern "thiscall" fn le_00616fd0(this: u32, key: u32) -> u32 {
        le::scene_tree_find(&mem(), this, key)
    }
    /// `Phyre_PAnimationChannelTargetBindScene_bindNode_006174b0`: a named scene node put in its entry's slot; 0.
    Phyre_PAnimationChannelTargetBindScene_bindNode_006174b0 @ 0x0061_74b0 => extern "thiscall" fn le_006174b0(this: u32, node: u32) -> u32 {
        le::bind_scene_node(&mut mem(), this, node)
    }
    /// `Phyre_PAnimationChannelTargetBindScene_bindTree_00616ee0`: every node of a scene tree bound; 0.
    Phyre_PAnimationChannelTargetBindScene_bindTree_00616ee0 @ 0x0061_6ee0 => extern "C" fn le_00616ee0(this: u32, root: u32) -> u32 {
        le::bind_scene_tree(&mut mem(), this, root)
    }
    /// `Phyre_PAnimationNetworkOptimiser_run_005cd7a0`: the nodes placed in dependency order (the context in this frame); `al` 1.
    Phyre_PAnimationNetworkOptimiser_run_005cd7a0 @ 0x005c_d7a0 => extern "thiscall" fn le_005cd7a0(this: u32, release_first: u32) -> u32 {
        let mut f = super::StackFrame::<{ ffx_panim::m5p_anim_left_e::CONTEXT_WORDS as usize }>::new();
        le::run(&mut mem(), this, release_first, f.at())
    }
    /// `Phyre_PAnimatableComponent_useFirstSource_00543570`: the component plays its first source alone (one made when none); 0, 1, 0x11 or an error.
    Phyre_PAnimatableComponent_useFirstSource_00543570 @ 0x0054_3570 => extern "thiscall" fn le_00543570(this: u32, arg: u32) -> u32 {
        le::use_first_source(&mut mem(), this, arg)
    }
    // batch 6
    /// `Phyre_PAnimationWeightedBlenderController_blendKernel_0052dae0`: weighted blend of four-float sources (linear, quaternion, largest weight); `al`.
    Phyre_PAnimationWeightedBlenderController_blendKernel_0052dae0 @ 0x0052_dae0 => extern "C" fn lf_0052dae0(out: u32, mode: u32, sources: u32, n: u32, weights: u32) -> u32 {
        lf::weighted(&mut mem(), out, mode, sources, n, weights)
    }
    /// `Phyre_PAnimationAdditiveBlenderController_blendKernel_0052ef70`: additive blend of four-float sources onto the first; `al`.
    Phyre_PAnimationAdditiveBlenderController_blendKernel_0052ef70 @ 0x0052_ef70 => extern "C" fn lf_0052ef70(out: u32, mode: u32, sources: u32, n: u32, weights: u32) -> u32 {
        lf::additive(&mut mem(), out, mode, sources, n, weights)
    }
    /// `Phyre_PAnimationTargetBlenderController_applyTargets_00531660`: blended values applied to their bound targets (matrices, copies, logs; locals in this frame); 0 or 1.
    Phyre_PAnimationTargetBlenderController_applyTargets_00531660 @ 0x0053_1660 => extern "thiscall" fn lf_00531660(this: u32, binds: u32, values: u32) -> u32 {
        let mut f = super::StackFrame::<{ ffx_panim::m5p_anim_left_f::APPLY_FRAME_WORDS as usize }>::new();
        lf::apply_targets(&mut mem(), this, binds, values, f.at())
    }
    // batch 7
    /// `Phyre_PAnimationSet_buildData_005114b0`: the set's packed data (header, key counts, sorted channel records, slots; the lookups in this frame); 0.
    Phyre_PAnimationSet_buildData_005114b0 @ 0x0051_14b0 => extern "thiscall" fn lg_005114b0(this: u32, targets: u32, set: u32) -> u32 {
        let mut f = super::StackFrame::<{ ffx_panim::m5p_anim_left_g::FRAME_WORDS as usize }>::new();
        lg::build_set_data(&mut mem(), this, targets, set, f.at())
    }
    /// `Phyre_PAnimationSet_buildSlots_0051de90`: the set's slots per (class, target), sorted by batch, indexed per target (the lookup in this frame); 0 or an error.
    Phyre_PAnimationSet_buildSlots_0051de90 @ 0x0051_de90 => extern "thiscall" fn lh_0051de90(this: u32) -> u32 {
        let mut f = super::StackFrame::<{ ffx_panim::m5p_anim_left_h::FRAME_WORDS as usize }>::new();
        lh::build_slots(&mut mem(), this, f.at())
    }
    /// `Phyre_PAnimationSet_rebuildTargets_0051c490`: the set's distinct targets gathered from its clips, the slots and every clip's data rebuilt; 0 or an error.
    Phyre_PAnimationSet_rebuildTargets_0051c490 @ 0x0051_c490 => extern "thiscall" fn li_0051c490(this: u32, cluster: u32) -> u32 {
        li::rebuild_targets(&mut mem(), this, cluster)
    }
    /// `Phyre_PAnimatableComponent_orderRecords_0053cb90`: the records ordered by a network optimiser (in this frame), the level array sized; 0 or 0xd.
    Phyre_PAnimatableComponent_orderRecords_0053cb90 @ 0x0053_cb90 => extern "thiscall" fn lj_0053cb90(this: u32) -> u32 {
        let mut f = super::StackFrame::<{ ffx_panim::m5p_anim_left_j::FRAME_WORDS as usize }>::new();
        lj::order_records(&mut mem(), this, f.at())
    }
    /// `Phyre_PAnimatableComponent_setNetwork_0053a2e0`: the network's nodes collected and laid out, targets bound, the batch block built (visitors in this frame); 0, 0xd or an error.
    Phyre_PAnimatableComponent_setNetwork_0053a2e0 @ 0x0053_a2e0 => extern "thiscall" fn lk_0053a2e0(this: u32, network: u32, binders: u32, n: u32) -> u32 {
        let mut f = super::StackFrame::<{ ffx_panim::m5p_anim_left_k::FRAME_WORDS as usize }>::new();
        lk::set_network(&mut mem(), this, network, binders, n, f.at())
    }
    // batch 12
    /// `Phyre_PAnimationChannelTargetBindScene_bindNames_00617350`: the target names keyed into the bind's tree and the scene walked (the node array in this frame).
    Phyre_PAnimationChannelTargetBindScene_bindNames_00617350 @ 0x0061_7350 => extern "thiscall" fn ll_00617350(this: u32, names: u32, results: u32, n: u32) {
        let mut f = super::StackFrame::<{ ffx_panim::m5p_anim_left_l::NAMES_FRAME_WORDS as usize }>::new();
        ll::bind_names(&mut mem(), this, names, results, n, f.at());
    }
    /// `Phyre_PAnimationChannelTargetBindScene_bindTargets_005ce1a0`: scene-node targets bound to the nodes found by name (temporaries in this frame, or a heap block past 64 targets); 0 or 1.
    Phyre_PAnimationChannelTargetBindScene_bindTargets_005ce1a0 @ 0x005c_e1a0 => extern "thiscall" fn ll_005ce1a0(this: u32, out: u32, targets: u32, n: u32) -> u32 {
        let mut f = super::StackFrame::<{ ffx_panim::m5p_anim_left_l::TARGETS_FRAME_WORDS as usize }>::new();
        ll::bind_targets_in(&mut mem(), this, out, targets, n, f.at())
    }
}
