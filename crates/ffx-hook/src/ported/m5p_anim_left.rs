//! M5 stage 1, list `phyre-anim-left` (`ffx_panim::m5p_anim_left*`): the
//! `PAnimation` functions the first anim round left (the larger ones).
//! Tests: `crates/ffx-oracle/tests/m5p_anim_left.rs`.
//!
//! Threads: animation batches run on the worker threads, the rest where
//! clusters load and bind. No shim keeps state; each works on the game's
//! memory through [`mem`].

use super::{mem, ported};
use ffx_panim::m5p_anim_left_a::bridge as la;

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
}
