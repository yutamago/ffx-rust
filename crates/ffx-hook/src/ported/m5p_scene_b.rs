//! M5 stage 1, list `scene`, batch 2 (`ffx_prender::m5p_scene_b`): the
//! small methods of the scene, gameplay and LOD classes. Parity tests:
//! `crates/ffx-oracle/tests/m5p_scene.rs` (`m5p_scene_b_*`).

use ffx_prender::m5p_scene_b as t;
use ffx_prender::m5p_scene_b::bridge as b;

use super::{mem, ported};

ported! {
    /// `Phyre_PRendering_PUtilityRendering_terminate`: the rendering utility's six clean-ups in order; 0.
    Phyre_PRendering_PUtilityRendering_terminate @ 0x0043_2ac0 => extern "C" fn sb_00432ac0() -> u32 {
        b::rendering_terminate(&mut mem())
    }
    /// `Phyre_PGameplay_PUtilityGameplay_terminate`: the gameplay utility's list clean-up; 0.
    Phyre_PGameplay_PUtilityGameplay_terminate @ 0x0043_3650 => extern "C" fn sb_00433650() -> u32 {
        b::gameplay_terminate(&mut mem())
    }
    /// `Phyre_PWorld_getInstanceCountForScript`: the named class's instance count in the world, or 0.
    Phyre_PWorld_getInstanceCountForScript @ 0x0044_36a0 => extern "thiscall" fn sb_004436a0(this: u32, name: u32) -> u32 {
        b::world_instance_count_for_script(&mut mem(), this, name)
    }
    /// `PNodeOwner_listsNode_00443ae0`: whether the node's owner lists it (al).
    PNodeOwner_listsNode_00443ae0 @ 0x0044_3ae0 => extern "thiscall" fn sb_00443ae0(this: u32) -> u8 {
        b::owner_lists(&mem(), this)
    }
    /// `Phyre_PArray_clearAt8_00443f80`: the array at `+8`/`+0xc` freed and cleared; 0.
    Phyre_PArray_clearAt8_00443f80 @ 0x0044_3f80 => extern "thiscall" fn sb_00443f80(this: u32) -> u32 {
        b::array_clear_at8(&mut mem(), this)
    }
    /// `Phyre_PEntity_classTreeFind_0044e8c0`: the node of a one-word key, or 0.
    Phyre_PEntity_classTreeFind_0044e8c0 @ 0x0044_e8c0 => extern "thiscall" fn sb_0044e8c0(tree: u32, key: u32) -> u32 {
        b::tree_find1(&mem(), tree, key)
    }
    /// `Phyre_PEntity_classLookup_0044f750`: `+0x10` of the key's node in the class tree, or 0.
    Phyre_PEntity_classLookup_0044f750 @ 0x0044_f750 => extern "C" fn sb_0044f750(key: u32) -> u32 {
        b::entity_class_lookup(&mem(), key)
    }
    /// `Phyre_PEntity_findHolder_004501d0`: the list entry whose chain holds `x`, into `out`; `out`.
    Phyre_PEntity_findHolder_004501d0 @ 0x0045_01d0 => extern "thiscall" fn sb_004501d0(this: u32, out: u32, x: u32) -> u32 {
        b::find_holder(&mut mem(), this, out, x)
    }
    /// `Phyre_PCameraProjection_dtor_004553e0`: the projection's destructor (unwind).
    Phyre_PCameraProjection_dtor_004553e0 @ 0x0045_53e0 => extern "thiscall" fn sb_004553e0(this: u32) {
        b::camera_projection_dtor(&mut mem(), this);
    }
    /// `Phyre_PCameraProjection_scalar_deleting_dtor`: the projection's deleting destructor; `this`.
    Phyre_PCameraProjection_scalar_deleting_dtor @ 0x0045_55a0 => extern "thiscall" fn sb_004555a0(this: u32, flags: u32) -> u32 {
        b::camera_projection_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PCameraOrthographic_setSize_00455f40`: size and size / size, then virtual slot 1; 0 for a null pointer.
    Phyre_PCameraOrthographic_setSize_00455f40 @ 0x0045_5f40 => extern "thiscall" fn sb_00455f40(this: u32, p: u32) -> u32 {
        b::ortho_set_size(&mut mem(), this, p)
    }
    /// `Phyre_PCameraProjection_dtor_00456410`: the projection's destructor (unwind).
    Phyre_PCameraProjection_dtor_00456410 @ 0x0045_6410 => extern "thiscall" fn sb_00456410(this: u32) {
        b::camera_projection_dtor(&mut mem(), this);
    }
    /// `Phyre_PCameraOrthographic_copy_00456430`: the base copy and the two floats; `this`.
    Phyre_PCameraOrthographic_copy_00456430 @ 0x0045_6430 => extern "thiscall" fn sb_00456430(this: u32, src: u32) -> u32 {
        b::ortho_copy(&mut mem(), this, src)
    }
    /// `Phyre_PInstancesComponent_copyItems8_00456da0`: 8-byte items copied, null slots skipped.
    Phyre_PInstancesComponent_copyItems8_00456da0 @ 0x0045_6da0 => extern "C" fn sb_00456da0(dst: u32, src: u32, n: u32) {
        b::copy_items8(&mut mem(), dst, src, n);
    }
    /// `Phyre_PInstancesComponent_element8_00459980`: element `i` of the 8-byte array at `+0x10`.
    Phyre_PInstancesComponent_element8_00459980 @ 0x0045_9980 => extern "thiscall" fn sb_00459980(this: u32, i: u32) -> u32 {
        b::element8(&mem(), this, i)
    }
    /// `Phyre_PArray_PTypedObject_8u_setToExternalBuffer_0045a110`: the external buffer set; 0.
    Phyre_PArray_PTypedObject_8u_setToExternalBuffer_0045a110 @ 0x0045_a110 => extern "thiscall" fn sb_0045a110(this: u32, buffer: u32, count: u32) -> u32 {
        b::array_set_external(&mut mem(), this, buffer, count)
    }
    /// `Phyre_PRendering_PLightType_scalar_deleting_dtor`: unregistered, deleted for `flags & 1`; `this`.
    Phyre_PRendering_PLightType_scalar_deleting_dtor @ 0x0049_d880 => extern "thiscall" fn sb_0049d880(this: u32, flags: u32) -> u32 {
        b::type_delete(&mut mem(), this, flags, &t::LIGHT_TYPE_DTOR)
    }
    /// `Phyre_PArray_clear_004c97e0`: an owned, non-null buffer freed, the header cleared.
    Phyre_PArray_clear_004c97e0 @ 0x004c_97e0 => extern "thiscall" fn sb_004c97e0(this: u32) -> u32 {
        b::array_clear_checked(&mut mem(), this)
    }
    /// `Phyre_PRendering_PShadowCasterType_scalar_deleting_dtor`: unregistered, deleted for `flags & 1`; `this`.
    Phyre_PRendering_PShadowCasterType_scalar_deleting_dtor @ 0x004c_aa80 => extern "thiscall" fn sb_004caa80(this: u32, flags: u32) -> u32 {
        b::type_delete(&mut mem(), this, flags, &t::SHADOW_CASTER_TYPE_DTOR)
    }
    /// `Phyre_PRendering_PContextSwitch_packState_004d4070`: the flag byte appended to the cursor; 0.
    Phyre_PRendering_PContextSwitch_packState_004d4070 @ 0x004d_4070 => extern "thiscall" fn sb_004d4070(this: u32, _a0: u32, _a1: u32, _a2: u32, cursor: u32, _a4: u32, _a5: u32, _a6: u32) -> u32 {
        b::pack_flag(&mut mem(), this, cursor)
    }
    /// `Phyre_PRendering_PContextSwitchInstancing_packState_004d4090`: whether instancing is on, appended to the cursor; 0.
    Phyre_PRendering_PContextSwitchInstancing_packState_004d4090 @ 0x004d_4090 => extern "thiscall" fn sb_004d4090(_this: u32, _a0: u32, _a1: u32, _a2: u32, cursor: u32, _a4: u32, object: u32, _a6: u32) -> u32 {
        b::pack_instancing(&mut mem(), cursor, object)
    }
    /// `Phyre_PRendering_PRenderList_ctor_004db1e0`: vtable, eight words and a byte zeroed; `this`.
    Phyre_PRendering_PRenderList_ctor_004db1e0 @ 0x004d_b1e0 => extern "thiscall" fn sb_004db1e0(this: u32) -> u32 {
        b::render_list_ctor(&mut mem(), this)
    }
    /// `Phyre_PContextVariantFoldingTable_zeroItems_004df180`: five words of each 20-byte item zeroed.
    Phyre_PContextVariantFoldingTable_zeroItems_004df180 @ 0x004d_f180 => extern "C" fn sb_004df180(items: u32, n: u32) {
        b::zero_items20(&mut mem(), items, n);
    }
    /// `Phyre_PArray_clear_004e37c0`: an owned buffer freed, the header cleared.
    Phyre_PArray_clear_004e37c0 @ 0x004e_37c0 => extern "thiscall" fn sb_004e37c0(this: u32) -> u32 {
        b::array_clear(&mut mem(), this)
    }
    /// `Phyre_PArray_clearPairs_004e37f0`: an owned buffer's pairs released, the header cleared.
    Phyre_PArray_clearPairs_004e37f0 @ 0x004e_37f0 => extern "thiscall" fn sb_004e37f0(this: u32) -> u32 {
        b::array_clear_release(&mut mem(), this, t::PAIRS_RELEASE)
    }
    /// `Phyre_PRendering_PUtilityRendering_resetGlobals_004e68d0`: the word, the overallocation and the byte reset.
    Phyre_PRendering_PUtilityRendering_resetGlobals_004e68d0 @ 0x004e_68d0 => extern "C" fn sb_004e68d0() -> u32 {
        b::render_reset(&mut mem())
    }
    /// `Phyre_PArray_clear_004f29c0`: an owned buffer freed, the header cleared.
    Phyre_PArray_clear_004f29c0 @ 0x004f_29c0 => extern "thiscall" fn sb_004f29c0(this: u32) -> u32 {
        b::array_clear(&mut mem(), this)
    }
    /// `Phyre_PArray_clearSegmentContexts_004f2a20`: an owned buffer's segment contexts released, the header cleared.
    Phyre_PArray_clearSegmentContexts_004f2a20 @ 0x004f_2a20 => extern "thiscall" fn sb_004f2a20(this: u32) -> u32 {
        b::array_clear_release(&mut mem(), this, t::SEGMENT_CONTEXTS_RELEASE)
    }
    /// `Phyre_PRendering_PMeshInstance_element64_004f8070`: element `i` of the 64-byte array at `+0xc`.
    Phyre_PRendering_PMeshInstance_element64_004f8070 @ 0x004f_8070 => extern "thiscall" fn sb_004f8070(this: u32, i: u32) -> u32 {
        b::element64(&mem(), this, i)
    }
    /// `Phyre_PRendering_PMeshInstance_set1c_004f8ad0`: the word at `+0x1c`; the value.
    Phyre_PRendering_PMeshInstance_set1c_004f8ad0 @ 0x004f_8ad0 => extern "thiscall" fn sb_004f8ad0(this: u32, v: u32) -> u32 {
        b::set_word(&mut mem(), this, 0x1c, v)
    }
    /// `Phyre_PScene_PNode_setLocalMatrix_005073f0`: `PMatrix4_Copy` into `+0x10`.
    Phyre_PScene_PNode_setLocalMatrix_005073f0 @ 0x0050_73f0 => extern "thiscall" fn sb_005073f0(this: u32, m: u32) -> u32 {
        b::forward_sub(&mut mem(), this, 0x10, t::MATRIX_COPY, m)
    }
    /// `Phyre_PScene_PNode_set0c_00507400`: the word at `+0xc`; the value.
    Phyre_PScene_PNode_set0c_00507400 @ 0x0050_7400 => extern "thiscall" fn sb_00507400(this: u32, v: u32) -> u32 {
        b::set_word(&mut mem(), this, 0xc, v)
    }
    /// `Phyre_PAnimation_PAnimationClip_deleteHeld_00516cf0`: the held object deleted from the cluster, the word cleared.
    Phyre_PAnimation_PAnimationClip_deleteHeld_00516cf0 @ 0x0051_6cf0 => extern "thiscall" fn sb_00516cf0(this: u32, cluster: u32) -> u32 {
        b::delete_held(&mut mem(), this, cluster)
    }
    /// `Phyre_PGameplay_PLocatorShapeType_scalar_deleting_dtor`: base destructor, deleted for `flags & 1`; `this`.
    Phyre_PGameplay_PLocatorShapeType_scalar_deleting_dtor @ 0x0054_5450 => extern "thiscall" fn sb_00545450(this: u32, flags: u32) -> u32 {
        b::deleting_dtor(&mut mem(), this, flags, &t::LOCATOR_SHAPE_TYPE_DTOR)
    }
    /// `Phyre_PGameplay_LocatorShapeType_fromName_00545e60`: the shape type of a name into `out`; 0 or 0x13.
    Phyre_PGameplay_LocatorShapeType_fromName_00545e60 @ 0x0054_5e60 => extern "C" fn sb_00545e60(out: u32, name: u32) -> u32 {
        b::find_into(&mut mem(), t::FIND_LOCATOR_SHAPE, out, name)
    }
    /// `Phyre_PArray_clear_00547440`: an owned, non-null buffer freed, the header cleared.
    Phyre_PArray_clear_00547440 @ 0x0054_7440 => extern "thiscall" fn sb_00547440(this: u32) -> u32 {
        b::array_clear_checked(&mut mem(), this)
    }
    /// `Phyre_PArray_clear_00547470`: an owned, non-null buffer freed, the header cleared.
    Phyre_PArray_clear_00547470 @ 0x0054_7470 => extern "thiscall" fn sb_00547470(this: u32) -> u32 {
        b::array_clear_checked(&mut mem(), this)
    }
    /// `Phyre_PGameplay_PSpline_length_005498c0`: the sum of the segment lengths (ST0).
    Phyre_PGameplay_PSpline_length_005498c0 @ 0x0054_98c0 => extern "thiscall" fn sb_005498c0(this: u32) -> f32 {
        b::spline_length(&mut mem(), this)
    }
    /// `Phyre_PArray_Vector3_setToExternalBuffer_0054cac0`: the external buffer set; 0.
    Phyre_PArray_Vector3_setToExternalBuffer_0054cac0 @ 0x0054_cac0 => extern "thiscall" fn sb_0054cac0(this: u32, buffer: u32, count: u32) -> u32 {
        b::array_set_external(&mut mem(), this, buffer, count)
    }
    /// `Phyre_PGameplay_TriggerType_ctor_0054d040`: `(name, 0, 1)`, listed when new; `this`.
    Phyre_PGameplay_TriggerType_ctor_0054d040 @ 0x0054_d040 => extern "thiscall" fn sb_0054d040(this: u32, _unused: u32, name: u32) -> u32 {
        b::named_entry_ctor(&mut mem(), this, name)
    }
    /// `Phyre_PGameplay_TriggerType_find_0054d190`: the listed entry of a name (`_stricmp`, any case), or 0.
    Phyre_PGameplay_TriggerType_find_0054d190 @ 0x0054_d190 => extern "C" fn sb_0054d190(name: u32) -> u32 {
        b::find_named(&mut mem(), name)
    }
    /// `Phyre_PGameplay_TriggerTypeSemantic_nameAndSize_0054d2b0`: the name and its size with the terminator; 0.
    Phyre_PGameplay_TriggerTypeSemantic_nameAndSize_0054d2b0 @ 0x0054_d2b0 => extern "C" fn sb_0054d2b0(pname: u32, out: u32, len: u32) -> u32 {
        b::name_and_size(&mut mem(), pname, out, len)
    }
    /// `Phyre_PGameplay_TriggerType_fromName_0054d320`: the trigger type of a name into `out`; 0 or 0x13.
    Phyre_PGameplay_TriggerType_fromName_0054d320 @ 0x0054_d320 => extern "C" fn sb_0054d320(out: u32, name: u32) -> u32 {
        b::find_into(&mut mem(), t::FIND_GAMEPLAY_TYPE, out, name)
    }
    /// `Phyre_PGameplay_PTrigger_set40_005505f0`: the byte at `+0x40`.
    Phyre_PGameplay_PTrigger_set40_005505f0 @ 0x0055_05f0 => extern "thiscall" fn sb_005505f0(this: u32, v: u32) {
        b::set_byte40(&mut mem(), this, v);
    }
    /// `Phyre_FreeUnlessBit0_00551670`: the block freed unless bit 0 of the word is set (unwind).
    Phyre_FreeUnlessBit0_00551670 @ 0x0055_1670 => extern "thiscall" fn sb_00551670(this: u32) {
        b::free_unless_bit0(&mut mem(), this);
    }
    /// `Phyre_PGameplay_PTimerComponent_initialize`: the countdown from the period, then `PScriptableComponent::initialize`.
    Phyre_PGameplay_PTimerComponent_initialize @ 0x0055_5d90 => extern "thiscall" fn sb_00555d90(this: u32, a: u32, b2: u32, c: u32) -> u32 {
        b::timer_initialize(&mut mem(), this, [a, b2, c])
    }
    /// `Phyre_PGameplay_PTimerComponent_uninitialize`: a held, owned timer released and cleared.
    Phyre_PGameplay_PTimerComponent_uninitialize @ 0x0055_5ef0 => extern "thiscall" fn sb_00555ef0(this: u32) -> u32 {
        b::timer_uninitialize(&mut mem(), this)
    }
    /// `Phyre_PGameplay_PCameraControllerComponent_flySet_005588e0`: the fly controller's `0x005d6f00`.
    Phyre_PGameplay_PCameraControllerComponent_flySet_005588e0 @ 0x0055_88e0 => extern "thiscall" fn sb_005588e0(this: u32, x: u32) -> u32 {
        b::forward_sub(&mut mem(), this, 0x34, t::FLY_SET[0], x)
    }
    /// `Phyre_PGameplay_PCameraControllerComponent_flySlot7_005588f0`: the fly controller's virtual slot 7.
    Phyre_PGameplay_PCameraControllerComponent_flySlot7_005588f0 @ 0x0055_88f0 => extern "thiscall" fn sb_005588f0(this: u32, f: f32) -> u32 {
        b::sub_virtual1(&mut mem(), this, f)
    }
    /// `Phyre_PGameplay_PCameraControllerComponent_flySlot8_00558910`: the fly controller's virtual slot 8.
    Phyre_PGameplay_PCameraControllerComponent_flySlot8_00558910 @ 0x0055_8910 => extern "thiscall" fn sb_00558910(this: u32, a: f32, b2: f32) -> u32 {
        b::sub_virtual2(&mut mem(), this, 0x20, a, b2)
    }
    /// `Phyre_PGameplay_PCameraControllerComponent_flySlot9_005589b0`: the fly controller's virtual slot 9.
    Phyre_PGameplay_PCameraControllerComponent_flySlot9_005589b0 @ 0x0055_89b0 => extern "thiscall" fn sb_005589b0(this: u32, a: f32, b2: f32) -> u32 {
        b::sub_virtual2(&mut mem(), this, 0x24, a, b2)
    }
    /// `Phyre_PGameplay_PCameraControllerComponent_setB4_00558a30`: the word at `+0xb4`; the value.
    Phyre_PGameplay_PCameraControllerComponent_setB4_00558a30 @ 0x0055_8a30 => extern "thiscall" fn sb_00558a30(this: u32, v: u32) -> u32 {
        b::set_word(&mut mem(), this, 0xb4, v)
    }
    /// `Phyre_PGameplay_PCameraControllerComponent_flySet_00558a50`: the fly controller's `0x005d77a0`.
    Phyre_PGameplay_PCameraControllerComponent_flySet_00558a50 @ 0x0055_8a50 => extern "thiscall" fn sb_00558a50(this: u32, x: u32) -> u32 {
        b::forward_sub(&mut mem(), this, 0x34, t::FLY_SET[1], x)
    }
    /// `Phyre_PSharray_clear_00577f20`: an owned buffer of more than one element freed, the header cleared.
    Phyre_PSharray_clear_00577f20 @ 0x0057_7f20 => extern "thiscall" fn sb_00577f20(this: u32) -> u32 {
        b::sharray_clear(&mut mem(), this)
    }
    /// `Phyre_PSharray_clear_005a2880`: an owned buffer of more than one element freed, the header cleared.
    Phyre_PSharray_clear_005a2880 @ 0x005a_2880 => extern "thiscall" fn sb_005a2880(this: u32) -> u32 {
        b::sharray_clear(&mut mem(), this)
    }
    /// `Phyre_PSharray_clear_005a28c0`: an owned, non-null buffer of more than one element freed, the header cleared.
    Phyre_PSharray_clear_005a28c0 @ 0x005a_28c0 => extern "thiscall" fn sb_005a28c0(this: u32) -> u32 {
        b::sharray_clear_checked(&mut mem(), this)
    }
    /// `Phyre_Unwind_nothing_005a2a90`: a jump to a bare `ret` (unwind handlers); nothing.
    Phyre_Unwind_nothing_005a2a90 @ 0x005a_2a90 => extern "C" fn sb_005a2a90() {}
    /// `Phyre_PArray_PLODLevel_setToExternalBuffer_005a8230`: `0x005a7d90` with the count marked external; 0.
    Phyre_PArray_PLODLevel_setToExternalBuffer_005a8230 @ 0x005a_8230 => extern "thiscall" fn sb_005a8230(this: u32, buffer: u32, count: u32) -> u32 {
        b::lod_set_external(&mut mem(), this, buffer, count)
    }
    /// `PLight_isRangeInfinite_005b8d80`: whether the range equals the infinite constant (al).
    PLight_isRangeInfinite_005b8d80 @ 0x005b_8d80 => extern "thiscall" fn sb_005b8d80(this: u32) -> u8 {
        b::range_is_infinite(&mem(), this)
    }
    /// `Phyre_PArray_word_005b8c30`: word `i` of the array at `+0x70`, or 0.
    Phyre_PArray_word_005b8c30 @ 0x005b_8c30 => extern "thiscall" fn sb_005b8c30(this: u32, i: u32) -> u32 {
        b::array_word(&mem(), this, i)
    }
    /// `Phyre_PScripting_PUtilityScripting_terminate_005d6c80`: the global array freed and cleared; 0.
    Phyre_PScripting_PUtilityScripting_terminate_005d6c80 @ 0x005d_6c80 => extern "C" fn sb_005d6c80() -> u32 {
        b::script_array_clear(&mut mem())
    }
    /// `Phyre_PWorldMatrixFlyController_scalar_deleting_dtor`: base vtable, freed for `flags & 1`; `this`.
    Phyre_PWorldMatrixFlyController_scalar_deleting_dtor @ 0x005d_6d20 => extern "thiscall" fn sb_005d6d20(this: u32, flags: u32) -> u32 {
        b::deleting_dtor(&mut mem(), this, flags, &t::FLY_CONTROLLER_DTOR)
    }
    /// `Phyre_PWorldMatrixFlyController_resetMatrix_005d7400`: the matrix at `+0x34` set to the identity.
    Phyre_PWorldMatrixFlyController_resetMatrix_005d7400 @ 0x005d_7400 => extern "thiscall" fn sb_005d7400(this: u32) {
        b::identity_at34(&mut mem(), this);
    }
    /// `Phyre_PWorldMatrixFlyController_getWorldMatrix_005d7790`: `PMatrix4_toWorldMatrix3x4` of `+4`.
    Phyre_PWorldMatrixFlyController_getWorldMatrix_005d7790 @ 0x005d_7790 => extern "thiscall" fn sb_005d7790(this: u32, x: u32) -> u32 {
        b::forward_sub(&mut mem(), this, 4, t::TO_WORLD_MATRIX, x)
    }
    /// `Phyre_PRendering_PInternal_PCheckVisibleBoundsJob_scalar_deleting_dtor`: base vtable, deleted for `flags & 1`; `this`.
    Phyre_PRendering_PInternal_PCheckVisibleBoundsJob_scalar_deleting_dtor @ 0x009f_2860 => extern "thiscall" fn sb_009f2860(this: u32, flags: u32) -> u32 {
        b::deleting_dtor(&mut mem(), this, flags, &t::VISIBLE_BOUNDS_JOB_DTOR)
    }
    /// `Phyre_PRendering_PVisibilityCheck_ringCount_009f3830`: the ring's entry count (`edx`: remainder or start).
    Phyre_PRendering_PVisibilityCheck_ringCount_009f3830 @ 0x009f_3830 => extern "thiscall" fn sb_009f3830(this: u32) -> u64 {
        b::ring_count(&mem(), this)
    }
    /// `PNodeOwner_listsNode_009f3a10`: whether the node's owner lists it (al; a copy of 0x00443ae0).
    PNodeOwner_listsNode_009f3a10 @ 0x009f_3a10 => extern "thiscall" fn sb_009f3a10(this: u32) -> u8 {
        b::owner_lists(&mem(), this)
    }
}
