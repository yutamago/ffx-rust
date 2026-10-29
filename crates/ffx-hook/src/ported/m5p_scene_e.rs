//! M5 stage 1, list `scene`, batch 5 (`ffx_prender::m5p_scene_e`): array
//! and `PSharray` resizes, the LOD group's external buffer, the gameplay
//! utility's clean-up and type registry, the tree destructors, the split
//! distances and the world cursor's advance. Parity tests:
//! `crates/ffx-oracle/tests/m5p_scene.rs` (`m5p_scene_e_*`).

use ffx_prender::m5p_scene_e as t;
use ffx_prender::m5p_scene_e::bridge as b;

use super::{mem, ported};

ported! {
    /// `Phyre_PGameplay_PUtilityGameplay_cleanup_004334e0`: registered types unlinked, both free lists reset.
    Phyre_PGameplay_PUtilityGameplay_cleanup_004334e0 @ 0x0043_34e0 => extern "C" fn se_004334e0() -> u32 {
        b::gameplay_cleanup(&mut mem())
    }
    /// `Phyre_PArray_PSceneRenderPass_resize_004e75e0`: resized with kept and new elements; 0 or 0xd.
    Phyre_PArray_PSceneRenderPass_resize_004e75e0 @ 0x004e_75e0 => extern "thiscall" fn se_004e75e0(this: u32, n: u32) -> u32 {
        b::render_pass_array_resize(&mut mem(), this, n)
    }
    /// `Phyre_PArray_PMaterialSwitch_resize_004e7530`: resized with kept and new elements; 0 or 0xd.
    Phyre_PArray_PMaterialSwitch_resize_004e7530 @ 0x004e_7530 => extern "thiscall" fn se_004e7530(this: u32, n: u32) -> u32 {
        b::material_array_resize(&mut mem(), this, n)
    }
    /// `Phyre_PSharray_PMeshInstanceSegmentContext_copyCtor_004f2740`: a `PSharray` copied from another; `this`.
    Phyre_PSharray_PMeshInstanceSegmentContext_copyCtor_004f2740 @ 0x004f_2740 => extern "thiscall" fn se_004f2740(this: u32, src: u32) -> u32 {
        ffx_prender::m5p_scene_d::bridge::sharray_copy_ctor(&mut mem(), this, src, t::SEGMENT_CONTEXTS_COPY, true)
    }
    /// `Phyre_PSharray_PMeshInstanceSegmentStreamBindingPtr_resize_004f8a40`: resized through its fill; 0 or 0xd.
    Phyre_PSharray_PMeshInstanceSegmentStreamBindingPtr_resize_004f8a40 @ 0x004f_8a40 => extern "thiscall" fn se_004f8a40(this: u32, n: u32) -> u32 {
        b::sharray_resize(&mut mem(), this, n, t::STREAM_BINDINGS_FILL)
    }
    /// `Phyre_PGameplay_TriggerType_get_0054d1e0`: the listed or a new gameplay type of a name.
    Phyre_PGameplay_TriggerType_get_0054d1e0 @ 0x0054_d1e0 => extern "C" fn se_0054d1e0(name: u32) -> u32 {
        b::gameplay_type(&mut mem(), name)
    }
    /// `Phyre_PSharray_PTriggerReceiverComponentPtr_resize_00550530`: resized through its fill; 0 or 0xd.
    Phyre_PSharray_PTriggerReceiverComponentPtr_resize_00550530 @ 0x0055_0530 => extern "thiscall" fn se_00550530(this: u32, n: u32) -> u32 {
        b::sharray_resize(&mut mem(), this, n, t::RECEIVERS_FILL)
    }
    /// `Phyre_PRedBlackTreeWithFreeList_dtorRemoving_00577da0`: the root removed until empty, then torn down.
    Phyre_PRedBlackTreeWithFreeList_dtorRemoving_00577da0 @ 0x0057_7da0 => extern "thiscall" fn se_00577da0(this: u32) -> u32 {
        b::tree_dtor_removing(&mut mem(), this)
    }
    /// `Phyre_PThreeTrees_dtor_00578030`: three trees with free lists destroyed.
    Phyre_PThreeTrees_dtor_00578030 @ 0x0057_8030 => extern "thiscall" fn se_00578030(this: u32) -> u32 {
        b::three_trees_dtor(&mut mem(), this)
    }
    /// `Phyre_PArray_PLODLevel_setToExternalBuffer_005a7d90`: owned levels destroyed and freed, the external buffer set; `n`.
    Phyre_PArray_PLODLevel_setToExternalBuffer_005a7d90 @ 0x005a_7d90 => extern "thiscall" fn se_005a7d90(this: u32, buffer: u32, n: u32) -> u32 {
        b::lod_levels_set_external(&mut mem(), this, buffer, n)
    }
    /// `PShadowCaster_splitDistances_005b8c70`: the four split distances into `out`; `out`.
    PShadowCaster_splitDistances_005b8c70 @ 0x005b_8c70 => extern "thiscall" fn se_005b8c70(this: u32, out: u32) -> u32 {
        b::split_distances(&mut mem(), this, out)
    }
    /// `PWorldCursor_advance_009f3e50`: the cursor moved on by `n` entries, empty blocks skipped.
    PWorldCursor_advance_009f3e50 @ 0x009f_3e50 => extern "thiscall" fn se_009f3e50(this: u32, n: u32) -> u32 {
        b::cursor_advance(&mut mem(), this, n)
    }
}
