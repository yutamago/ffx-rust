//! M5 stage 1, list `scene`, batch 4 (`ffx_prender::m5p_scene_d`): the
//! entity class tree, array constructors, `PSharray` copies, the timer and
//! scriptable components, the LOD blend packer. Parity tests:
//! `crates/ffx-oracle/tests/m5p_scene.rs` (`m5p_scene_d_*`).

use ffx_prender::m5p_scene_d as t;
use ffx_prender::m5p_scene_d::bridge as b;

use super::{mem, ported};

ported! {
    /// `Phyre_PClassDescriptor_GetInstanceMemberForType_Phyre_PEntity`: -1 for a name that is no world class, else the entities of that class pushed, their count.
    Phyre_PClassDescriptor_GetInstanceMemberForType_Phyre_PEntity @ 0x0044_e520 => extern "C" fn sd_0044e520(list: u32, state: u32, name: u32) -> u32 {
        b::entity_instance_member(&mut mem(), list, state, name)
    }
    /// `Phyre_PEntity_classTreeAttach_0044e960`: the one-word-key tree's attach; the node found or 0.
    Phyre_PEntity_classTreeAttach_0044e960 @ 0x0044_e960 => extern "thiscall" fn sd_0044e960(tree: u32, node: u32) -> u32 {
        b::tree_attach1(&mut mem(), tree, node)
    }
    /// `Phyre_PEntity_lookup_0044f6d0`: the class tree's value for an entity or its member; 0 for none.
    Phyre_PEntity_lookup_0044f6d0 @ 0x0044_f6d0 => extern "C" fn sd_0044f6d0(object: u32, class: u32) -> u32 {
        b::entity_lookup(&mut mem(), object, class)
    }
    /// `Phyre_PEntity_FixInstancesAfterLoading`: the loaded entities' class nodes back in the class tree; 0.
    Phyre_PEntity_FixInstancesAfterLoading @ 0x0044_f780 => extern "C" fn sd_0044f780(list: u32) -> u32 {
        b::fix_instances(&mut mem(), list)
    }
    /// `PMeshInstance_findDataBlock_00489150`: the segment into `out` and the data block of a key, or 0.
    PMeshInstance_findDataBlock_00489150 @ 0x0048_9150 => extern "thiscall" fn sd_00489150(this: u32, out: u32, key: u32) -> u32 {
        b::find_data_block(&mut mem(), this, out, key)
    }
    /// `Phyre_PRendering_PContextSwitchLODBlend_packState_004d40c0`: whether the LOD blend is on, appended to the cursor; 0.
    Phyre_PRendering_PContextSwitchLODBlend_packState_004d40c0 @ 0x004d_40c0 => extern "thiscall" fn sd_004d40c0(_this: u32, _a0: u32, node: u32, context: u32, cursor: u32, _a4: u32, _a5: u32, _a6: u32) -> u32 {
        b::pack_lod_blend(&mut mem(), node, context, cursor)
    }
    /// `Phyre_PShader_constructItems_004df1d0`: each 32-byte `PShader` constructed.
    Phyre_PShader_constructItems_004df1d0 @ 0x004d_f1d0 => extern "C" fn sd_004df1d0(items: u32, n: u32) {
        b::construct_items(&mut mem(), items, n, 0x20, t::SHADER_CTOR);
    }
    /// `Phyre_PMaterialSwitch_constructItems_004e61d0`: each 8-byte `PMaterialSwitch` constructed.
    Phyre_PMaterialSwitch_constructItems_004e61d0 @ 0x004e_61d0 => extern "C" fn sd_004e61d0(items: u32, n: u32) {
        b::construct_items(&mut mem(), items, n, 8, t::MATERIAL_SWITCH_CTOR);
    }
    /// `Phyre_PRendering_PRenderList_constructItems_004e6250`: each 0x28-byte render list constructed.
    Phyre_PRendering_PRenderList_constructItems_004e6250 @ 0x004e_6250 => extern "C" fn sd_004e6250(items: u32, n: u32) {
        b::construct_items(&mut mem(), items, n, 0x28, t::RENDER_LIST_CTOR);
    }
    /// `Phyre_PEffect_copyConstructItems_004e7d70`: 8-byte items copy-constructed.
    Phyre_PEffect_copyConstructItems_004e7d70 @ 0x004e_7d70 => extern "C" fn sd_004e7d70(dst: u32, src: u32, n: u32) {
        b::copy_construct_items(&mut mem(), dst, src, n, t::SHARRAY_COPY_CTOR);
    }
    /// `Phyre_PSharray_copyCtor_004e9ee0`: a `PSharray` copied from another; `this`.
    Phyre_PSharray_copyCtor_004e9ee0 @ 0x004e_9ee0 => extern "thiscall" fn sd_004e9ee0(this: u32, src: u32) -> u32 {
        b::sharray_copy_ctor(&mut mem(), this, src, t::SHARRAY_ALLOC, false)
    }
    /// `Phyre_PSharray_PTriggerReceiverComponentPtr_set_005506c0`: set to `n` elements at a buffer; 5 for a negative `n`, else 0.
    Phyre_PSharray_PTriggerReceiverComponentPtr_set_005506c0 @ 0x0055_06c0 => extern "thiscall" fn sd_005506c0(this: u32, buffer: u32, n: u32) -> u32 {
        b::sharray_set(&mut mem(), this, buffer, n)
    }
    /// `Phyre_PGameplay_PScriptableComponent_scalar_deleting_dtor`: name freed, reference released, freed for `flags & 1`; `this`.
    Phyre_PGameplay_PScriptableComponent_scalar_deleting_dtor @ 0x0055_2d30 => extern "thiscall" fn sd_00552d30(this: u32, flags: u32) -> u32 {
        b::scriptable_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PGameplay_PTimerComponent_assignFrom_00555270`: the timer component copied; `this`.
    Phyre_PGameplay_PTimerComponent_assignFrom_00555270 @ 0x0055_5270 => extern "thiscall" fn sd_00555270(this: u32, src: u32) -> u32 {
        b::timer_copy(&mut mem(), this, src)
    }
    /// `Phyre_PRedBlackTreeWithFreeList_dtor_00577e90`: orphaned, free list reset and destroyed, cleared twice.
    Phyre_PRedBlackTreeWithFreeList_dtor_00577e90 @ 0x0057_7e90 => extern "thiscall" fn sd_00577e90(this: u32) -> u32 {
        b::tree_owner_dtor(&mut mem(), this)
    }
    /// `Phyre_PFreeList_freeNamedNode_0064b8c0`: the node's two strings freed, the node deallocated.
    Phyre_PFreeList_freeNamedNode_0064b8c0 @ 0x0064_b8c0 => extern "thiscall" fn sd_0064b8c0(this: u32, node: u32) -> u32 {
        b::free_named_node(&mut mem(), this, node)
    }
}
