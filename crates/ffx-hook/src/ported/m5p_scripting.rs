//! M5 stage 1, list `phyre-scripting`: `PhyreEngine`'s scripting support
//! outside the script layer (`ffx_pother_a::m5p_scripting*`). The parity
//! tests are `crates/ffx-oracle/tests/m5p_scripting.rs`.
//!
//! Threads: the script runtime runs on the main thread; the async process
//! headers can be processed from a worker. No shim keeps state; each works
//! on the game's memory through [`mem`].

use ffx_pother_a::m5p_scripting_small::{self as s, bridge as b};

use super::{mem, ported};

ported! {
    // ---- batch 1: the small functions ----
    /// `Phyre_PAnimation_PTimeController_slot2_005215f0`.
    Phyre_PAnimation_PTimeController_slot2_005215f0 @ 0x0052_15f0 => extern "thiscall" fn s_005215f0(_this: u32, _a: u32) -> u32 {
        0
    }
    /// `Phyre_DynamicMeshDefaultImplmenetation_slot1_006e1f40`.
    Phyre_DynamicMeshDefaultImplmenetation_slot1_006e1f40 @ 0x006e_1f40 => extern "thiscall" fn s_006e1f40(_this: u32, _a: u32, _b: u32) -> u32 {
        0
    }
    /// `Phyre_PSerialization_PStreamWriterFile_close_009f08a0`.
    Phyre_PSerialization_PStreamWriterFile_close_009f08a0 @ 0x009f_08a0 => extern "thiscall" fn s_009f08a0(this: u32) -> u32 {
        b::close_script_context(&mut mem(), this)
    }
    /// `Phyre_PSerialization_PStreamWriterFile_isOpen_009f08b0`.
    Phyre_PSerialization_PStreamWriterFile_isOpen_009f08b0 @ 0x009f_08b0 => extern "thiscall" fn s_009f08b0(this: u32) -> u32 {
        b::stream_is_open(&mut mem(), this)
    }
    /// `Phyre_PSerialization_PStreamWriterFile_write_009f0900`.
    Phyre_PSerialization_PStreamWriterFile_write_009f0900 @ 0x009f_0900 => extern "thiscall" fn s_009f0900(this: u32, a: u32, b: u32) -> u32 {
        b::stream_write(&mut mem(), this, a, b)
    }
    /// `Phyre_PPostProcessing_firstOfContext_00a161b0`.
    Phyre_PPostProcessing_firstOfContext_00a161b0 @ 0x00a1_61b0 => extern "thiscall" fn s_00a161b0(this: u32) -> u32 {
        b::first_of(&mem(), this)
    }
    /// `Phyre_PInternal_PCallerImplementation_PCallerImplementation1_PMethodCallerConcrete_Phyre_PAnimation_PAnimatableComponent_Phyre_PResult_Phyre_PAnimation_PAnimationClip__dtor_PMethodCallerConcrete`.
    Phyre_PInternal_PCallerImplementation_PCallerImplementation1_PMethodCallerConcrete_Phyre_PAnimation_PAnimatableComponent_Phyre_PResult_Phyre_PAnimation_PAnimationClip__dtor_PMethodCallerConcrete @ 0x00a8_d8b4 => extern "C" fn s_00a8d8b4(p: u32) -> u32 {
        b::delete_forward(&mut mem(), p)
    }
    /// `Phyre_PGameplay_PScriptableComponent_slot5_set14_005534f0`.
    Phyre_PGameplay_PScriptableComponent_slot5_set14_005534f0 @ 0x0055_34f0 => extern "thiscall" fn s_005534f0(this: u32, v: u32) -> u32 {
        b::set_14(&mut mem(), this, v)
    }
    /// `Phyre_PScripting_vaArgWord_005cfdc0`.
    Phyre_PScripting_vaArgWord_005cfdc0 @ 0x005c_fdc0 => extern "C" fn s_005cfdc0(va: u32) -> u32 {
        b::va_arg_word(&mut mem(), va)
    }
    /// `Phyre_PGameplay_PScriptedComponent_slot5_set14_00554a70`.
    Phyre_PGameplay_PScriptedComponent_slot5_set14_00554a70 @ 0x0055_4a70 => extern "thiscall" fn s_00554a70(this: u32, v: u32) -> u32 {
        b::set_14(&mut mem(), this, v)
    }
    /// `Phyre_PClassDescriptorDynamic_constructThrough_0043b9d0`.
    Phyre_PClassDescriptorDynamic_constructThrough_0043b9d0 @ 0x0043_b9d0 => extern "C" fn s_0043b9d0(obj: u32, x: u32) -> u32 {
        b::construct_through(&mut mem(), obj, x)
    }
    /// `Phyre_PRendering_PContextSwitchLights_lessC_004d3bd0`.
    Phyre_PRendering_PContextSwitchLights_lessC_004d3bd0 @ 0x004d_3bd0 => extern "C" fn s_004d3bd0(a: u32, b: u32) -> u32 {
        b::less_c(&mem(), a, b)
    }
    /// `Phyre_PScripting_forward14_00437320`.
    Phyre_PScripting_forward14_00437320 @ 0x0043_7320 => extern "thiscall" fn s_00437320(this: u32, x: u32) -> u32 {
        b::forward_14(&mut mem(), this, x)
    }
    /// `Phyre_PGameplay_PScriptableComponent_slot2_clear_00553590`.
    Phyre_PGameplay_PScriptableComponent_slot2_clear_00553590 @ 0x0055_3590 => extern "thiscall" fn s_00553590(this: u32) {
        b::clear_script(&mut mem(), this);
    }
    /// `Phyre_PRendering_PContextSwitchLights_tableGet_0049e4e0`.
    Phyre_PRendering_PContextSwitchLights_tableGet_0049e4e0 @ 0x0049_e4e0 => extern "C" fn s_0049e4e0(i: u32) -> u32 {
        b::table_get(&mem(), 0x00c9_af00, i)
    }
    /// `Phyre_PRendering_PContextSwitchLights_tableGet_004cb750`.
    Phyre_PRendering_PContextSwitchLights_tableGet_004cb750 @ 0x004c_b750 => extern "C" fn s_004cb750(i: u32) -> u32 {
        b::table_get(&mem(), 0x00ca_43b0, i)
    }
    /// `Phyre_PScripting_ringCount_004361d0`.
    Phyre_PScripting_ringCount_004361d0 @ 0x0043_61d0 => extern "C" fn s_004361d0(list: u32) -> u32 {
        b::ring_count(&mem(), list)
    }
    /// `PScripting_RegisterPhyreNamespace`.
    PScripting_RegisterPhyreNamespace @ 0x0043_6f70 => extern "C" fn s_00436f70(l: u32) -> u32 {
        b::register_phyre_namespace(&mut mem(), l)
    }
    /// `Phyre_DynamicMeshDefaultImplmenetation_slot6_006e21c0`.
    Phyre_DynamicMeshDefaultImplmenetation_slot6_006e21c0 @ 0x006e_21c0 => extern "thiscall" fn s_006e21c0(_this: u32, _a0: u32, a1: u32, _a2: u32, _a3: u32, a4: u32) -> u32 {
        b::unlink_cell(&mut mem(), a1, a4)
    }
    /// `Phyre_PGameplay_PScriptableComponent_slot3_set18_005534d0`.
    Phyre_PGameplay_PScriptableComponent_slot3_set18_005534d0 @ 0x0055_34d0 => extern "thiscall" fn s_005534d0(this: u32, v: u32) -> u32 {
        b::set_nonnull(&mut mem(), this, 0x18, v)
    }
    /// `Phyre_PGameplay_PScriptableComponent_slot7_set1c_00553500`.
    Phyre_PGameplay_PScriptableComponent_slot7_set1c_00553500 @ 0x0055_3500 => extern "thiscall" fn s_00553500(this: u32, v: u32) -> u32 {
        b::set_nonnull(&mut mem(), this, 0x1c, v)
    }
    /// `Phyre_PScripting_PScheduler_add_005d0260`.
    Phyre_PScripting_PScheduler_add_005d0260 @ 0x005d_0260 => extern "thiscall" fn s_005d0260(this: u32, a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        b::scheduler_add(&mut mem(), this, [a0, a1, a2, a3])
    }
    /// `Phyre_PScripting_printHook_005d6050`.
    Phyre_PScripting_printHook_005d6050 @ 0x005d_6050 => extern "C" fn s_005d6050(x: u32) -> u32 {
        b::print_hook(&mut mem(), x)
    }
    /// `btAlignedObjectArray_init_00a8e620`.
    btAlignedObjectArray_init_00a8e620 @ 0x00a8_e620 => extern "thiscall" fn s_00a8e620(this: u32) -> u32 {
        b::aligned_array_init(&mut mem(), this)
    }
    /// `Phyre_PScripting_PScheduler_insert_005d0370`.
    Phyre_PScripting_PScheduler_insert_005d0370 @ 0x005d_0370 => extern "thiscall" fn s_005d0370(this: u32, a0: u32, a1: u32, a2: u32, a3: u32, a4: u32) -> u32 {
        b::scheduler_insert(&mut mem(), this, [a0, a1, a2, a3, a4])
    }
    /// `Phyre_PScripting_setScriptGlobal_005d5330`.
    Phyre_PScripting_setScriptGlobal_005d5330 @ 0x005d_5330 => extern "C" fn s_005d5330(l: u32, p: u32) -> u32 {
        b::set_script_global(&mut mem(), l, p)
    }
    /// `Phyre_ClothInstancingDynamicMesh_slot6_006e2190`.
    Phyre_ClothInstancingDynamicMesh_slot6_006e2190 @ 0x006e_2190 => extern "thiscall" fn s_006e2190(this: u32, a: u32, b: u32, c: u32) -> u32 {
        b::cloth_stage_and_draw(&mut mem(), this, a, b, c)
    }
    /// `Phyre_PScripting_PScheduler_copyBlock_00618130`.
    Phyre_PScripting_PScheduler_copyBlock_00618130 @ 0x0061_8130 => extern "thiscall" fn s_00618130(this: u32) -> u32 {
        b::copy_block(&mut mem(), this)
    }
    /// `Phyre_PCaller_scalarDeletingDtor_00430ad0`.
    Phyre_PCaller_scalarDeletingDtor_00430ad0 @ 0x0043_0ad0 => extern "thiscall" fn s_00430ad0(this: u32, flags: u32) -> u32 {
        b::deleting_dtor(&mut mem(), &s::DELETING_DTORS[0], this, flags)
    }
    /// `Phyre_PScripting_PClassCallableMethodScript_PCallerScript_scalarDeletingDtor_00619540`.
    Phyre_PScripting_PClassCallableMethodScript_PCallerScript_scalarDeletingDtor_00619540 @ 0x0061_9540 => extern "thiscall" fn s_00619540(this: u32, flags: u32) -> u32 {
        b::deleting_dtor(&mut mem(), &s::DELETING_DTORS[1], this, flags)
    }
    /// `Phyre_PScripting_PAsyncProcessBuffer_scalarDeletingDtor_005ce5f0`.
    Phyre_PScripting_PAsyncProcessBuffer_scalarDeletingDtor_005ce5f0 @ 0x005c_e5f0 => extern "thiscall" fn s_005ce5f0(this: u32, flags: u32) -> u32 {
        b::deleting_dtor(&mut mem(), &s::DELETING_DTORS[2], this, flags)
    }
    /// `btSimulationIslandManager_scalarDeletingDtor_00a8dbb0`.
    btSimulationIslandManager_scalarDeletingDtor_00a8dbb0 @ 0x00a8_dbb0 => extern "thiscall" fn s_00a8dbb0(this: u32, flags: u32) -> u32 {
        b::deleting_dtor(&mut mem(), &s::DELETING_DTORS[3], this, flags)
    }
    /// `ShadowDynamicMeshInstance_scalarDeletingDtor_006f9580`.
    ShadowDynamicMeshInstance_scalarDeletingDtor_006f9580 @ 0x006f_9580 => extern "thiscall" fn s_006f9580(this: u32, flags: u32) -> u32 {
        b::deleting_dtor(&mut mem(), &s::DELETING_DTORS[4], this, flags)
    }
    /// `Phyre_PScripting_PAsyncProcessBatch_scalarDeletingDtor_005ce5c0`.
    Phyre_PScripting_PAsyncProcessBatch_scalarDeletingDtor_005ce5c0 @ 0x005c_e5c0 => extern "thiscall" fn s_005ce5c0(this: u32, flags: u32) -> u32 {
        b::deleting_dtor(&mut mem(), &s::DELETING_DTORS[5], this, flags)
    }
    /// `Phyre_PScripting_PAsyncProcessHeader_process`.
    Phyre_PScripting_PAsyncProcessHeader_process @ 0x0061_7f30 => extern "thiscall" fn s_00617f30(this: u32) -> u32 {
        b::async_process(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PAnimationEventController_forwardEvent_00524850`.
    Phyre_PAnimation_PAnimationEventController_forwardEvent_00524850 @ 0x0052_4850 => extern "C" fn s_00524850(a: u32, p: u32) -> u32 {
        b::event_forward(&mut mem(), a, p)
    }
    /// `PScriptedComponent_initialize`.
    PScriptedComponent_initialize @ 0x0055_4830 => extern "thiscall" fn s_00554830(this: u32, a: u32, b: u32, l: u32) -> u32 {
        b::scripted_initialize(&mut mem(), this, a, b, l)
    }
    /// `Phyre_PInstancesComponent_setOwnerAll_0044db10`.
    Phyre_PInstancesComponent_setOwnerAll_0044db10 @ 0x0044_db10 => extern "C" fn s_0044db10(p: u32) -> u32 {
        b::set_owner_all(&mut mem(), p)
    }
    /// `Phyre_PGameplay_PScriptableComponent_initialize`.
    Phyre_PGameplay_PScriptableComponent_initialize @ 0x0055_31d0 => extern "thiscall" fn s_005531d0(this: u32, a: u32, b: u32, c: u32) -> u32 {
        b::scriptable_initialize(&mut mem(), this, a, b, c)
    }
    /// `Phyre_PScene_PUtilityScene_link_00506620`.
    Phyre_PScene_PUtilityScene_link_00506620 @ 0x0050_6620 => extern "thiscall" fn s_00506620(this: u32, node: u32) -> u32 {
        b::scene_link(&mut mem(), this, node)
    }
    /// `Phyre_PPostProcessing_entryPlus4_00a16980`.
    Phyre_PPostProcessing_entryPlus4_00a16980 @ 0x00a1_6980 => extern "thiscall" fn s_00a16980(this: u32, i: u32) -> u32 {
        b::entry_plus_4(&mem(), this, i)
    }
    /// `Phyre_PPostProcessing_entryWord4_00a169b0`.
    Phyre_PPostProcessing_entryWord4_00a169b0 @ 0x00a1_69b0 => extern "thiscall" fn s_00a169b0(this: u32, i: u32) -> u32 {
        b::entry_word_4(&mem(), this, i)
    }
    /// `Phyre_PRendering_PUtilityRendering_nameGet_0049e550`.
    Phyre_PRendering_PUtilityRendering_nameGet_0049e550 @ 0x0049_e550 => extern "C" fn s_0049e550(p: u32, name: u32, len: u32) -> u32 {
        b::name_get(&mut mem(), p, name, len)
    }
    /// `Phyre_PRendering_PUtilityRendering_nameGet_004cb910`.
    Phyre_PRendering_PUtilityRendering_nameGet_004cb910 @ 0x004c_b910 => extern "C" fn s_004cb910(p: u32, name: u32, len: u32) -> u32 {
        b::name_get(&mut mem(), p, name, len)
    }
    /// `Phyre_PRendering_PUtilityRendering_nameGet_004d3b30`.
    Phyre_PRendering_PUtilityRendering_nameGet_004d3b30 @ 0x004d_3b30 => extern "C" fn s_004d3b30(p: u32, name: u32, len: u32) -> u32 {
        b::name_get(&mut mem(), p, name, len)
    }
    /// `Phyre_PGameplay_PUtilityGameplay_nameGet_00545aa0`.
    Phyre_PGameplay_PUtilityGameplay_nameGet_00545aa0 @ 0x0054_5aa0 => extern "C" fn s_00545aa0(p: u32, name: u32, len: u32) -> u32 {
        b::name_get(&mut mem(), p, name, len)
    }
    /// `Phyre_PScripting_GetPScriptFromContext`.
    Phyre_PScripting_GetPScriptFromContext @ 0x005d_5780 => extern "C" fn s_005d5780(l: u32) -> u32 {
        b::get_pscript(&mut mem(), l)
    }
    /// `Phyre_PScripting_PScheduler_popIfLast_005cf3f0`.
    Phyre_PScripting_PScheduler_popIfLast_005cf3f0 @ 0x005c_f3f0 => extern "thiscall" fn s_005cf3f0(this: u32) {
        b::pop_if_last(&mut mem(), this);
    }
    /// `Phyre_PPostProcessing_PFXAA_initialize_00a1b780`.
    Phyre_PPostProcessing_PFXAA_initialize_00a1b780 @ 0x00a1_b780 => extern "thiscall" fn s_00a1b780(this: u32) -> u32 {
        b::fxaa_init(&mut mem(), this)
    }
    /// `Phyre_PScripting_PScheduler_pairArrayRelease_005cf2a0`.
    Phyre_PScripting_PScheduler_pairArrayRelease_005cf2a0 @ 0x005c_f2a0 => extern "thiscall" fn s_005cf2a0(this: u32) -> u32 {
        b::pair_array_release(&mut mem(), this)
    }
    /// `Phyre_PAnimation_destroyArray_00537650`.
    Phyre_PAnimation_destroyArray_00537650 @ 0x0053_7650 => extern "C" fn s_00537650(array: u32, n: u32) {
        b::destroy_array(&mut mem(), array, n);
    }
    /// `Phyre_PAnimation_destroyArray_005376d0`.
    Phyre_PAnimation_destroyArray_005376d0 @ 0x0053_76d0 => extern "C" fn s_005376d0(array: u32, n: u32) {
        b::destroy_array(&mut mem(), array, n);
    }
    /// `Phyre_PAnimation_destroyArray_00537710`.
    Phyre_PAnimation_destroyArray_00537710 @ 0x0053_7710 => extern "C" fn s_00537710(array: u32, n: u32) {
        b::destroy_array(&mut mem(), array, n);
    }
    /// `Phyre_PAnimation_destroyArray_00537750`.
    Phyre_PAnimation_destroyArray_00537750 @ 0x0053_7750 => extern "C" fn s_00537750(array: u32, n: u32) {
        b::destroy_array(&mut mem(), array, n);
    }
    /// `Phyre_PScripting_pushBase_00435580`.
    Phyre_PScripting_pushBase_00435580 @ 0x0043_5580 => extern "C" fn s_00435580(l: u32, pp: u32) -> u32 {
        b::push_base(&mut mem(), l, pp)
    }
    /// `Phyre_PScripting_countEvery_005d0e90`.
    Phyre_PScripting_countEvery_005d0e90 @ 0x005d_0e90 => extern "thiscall" fn s_005d0e90(this: u32, start: u32, step: u32) -> u32 {
        b::count_every(&mut mem(), this, start, step)
    }
    /// `Phyre_PSerialization_PStreamWriterFile_open_009f08c0`.
    Phyre_PSerialization_PStreamWriterFile_open_009f08c0 @ 0x009f_08c0 => extern "thiscall" fn s_009f08c0(this: u32, path: u32) -> u32 {
        b::stream_writer_open(&mut mem(), this, path)
    }
    /// `Phyre_PSerialization_PStreamWriterFile_flush_009f0910`.
    Phyre_PSerialization_PStreamWriterFile_flush_009f0910 @ 0x009f_0910 => extern "thiscall" fn s_009f0910(this: u32) -> u32 {
        b::stream_writer_flush(&mut mem(), this)
    }
    /// `Phyre_PScripting_newMatrixUserdata_00571030`.
    Phyre_PScripting_newMatrixUserdata_00571030 @ 0x0057_1030 => extern "C" fn s_00571030(l: u32) -> u32 {
        b::new_userdata(&mut mem(), l, 0x40, s::STR_MATRIX)
    }
    /// `Phyre_PScripting_newVectorUserdata_00571070`.
    Phyre_PScripting_newVectorUserdata_00571070 @ 0x0057_1070 => extern "C" fn s_00571070(l: u32) -> u32 {
        b::new_userdata(&mut mem(), l, 0x10, s::STR_VECTOR)
    }
    /// `Phyre_PScripting_pushMatrix_00571f50`.
    Phyre_PScripting_pushMatrix_00571f50 @ 0x0057_1f50 => extern "C" fn s_00571f50(l: u32, m: u32) -> u32 {
        b::push_matrix(&mut mem(), l, m)
    }
    /// `Phyre_PScripting_swizzleIndex_005710b0`.
    Phyre_PScripting_swizzleIndex_005710b0 @ 0x0057_10b0 => extern "C" fn s_005710b0(c: u32) -> u32 {
        b::swizzle_index(c)
    }
    /// `Phyre_PScene_PUtilityScene_unlink_00507360`.
    Phyre_PScene_PUtilityScene_unlink_00507360 @ 0x0050_7360 => extern "thiscall" fn s_00507360(this: u32, node: u32) {
        let _ = b::scene_unlink(&mut mem(), this, node);
    }
    /// `Phyre_PPostProcessing_findEnabled_00a16100`.
    Phyre_PPostProcessing_findEnabled_00a16100 @ 0x00a1_6100 => extern "thiscall" fn s_00a16100(this: u32) -> u32 {
        b::find_enabled(&mut mem(), this)
    }
    /// `Phyre_PScripting_PAsyncProcessHeader_wait_00617e40`.
    Phyre_PScripting_PAsyncProcessHeader_wait_00617e40 @ 0x0061_7e40 => extern "thiscall" fn s_00617e40(this: u32, out: u32) -> u32 {
        b::async_wait(&mut mem(), this, out)
    }
    /// `Phyre_PScripting_pushObject_00436e80`.
    Phyre_PScripting_pushObject_00436e80 @ 0x0043_6e80 => extern "C" fn s_00436e80(l: u32, object: u32, descriptor: u32) -> u32 {
        b::push_object(&mut mem(), l, object, descriptor)
    }
    /// `Phyre_PString_compare_005d6760`.
    Phyre_PString_compare_005d6760 @ 0x005d_6760 => extern "C" fn s_005d6760(a: u32, b: u32) -> u32 {
        b::string_compare(&mem(), a, b)
    }
    /// `Phyre_PEntity_link_0044fca0`.
    Phyre_PEntity_link_0044fca0 @ 0x0044_fca0 => extern "thiscall" fn s_0044fca0(this: u32, node: u32) -> u32 {
        b::entity_link(&mut mem(), this, node)
    }
    /// `Phyre_PScripting_pushSelf_00436ed0`.
    Phyre_PScripting_pushSelf_00436ed0 @ 0x0043_6ed0 => extern "C" fn s_00436ed0(l: u32, obj: u32) -> u32 {
        b::push_self(&mut mem(), l, obj)
    }
    /// `btSimulationIslandManager_ctor_00a8da40`.
    btSimulationIslandManager_ctor_00a8da40 @ 0x00a8_da40 => extern "thiscall" fn s_00a8da40(this: u32) -> u32 {
        b::island_manager_ctor(&mut mem(), this)
    }
    /// `Phyre_PScripting_pushArray_00436f20`.
    Phyre_PScripting_pushArray_00436f20 @ 0x0043_6f20 => extern "C" fn s_00436f20(l: u32, n: u32, array: u32, ty: u32) -> u32 {
        b::push_array(&mut mem(), l, n, array, ty)
    }
    /// `Phyre_PScripting_PCallerScript_pushVararg_PChar_00618630`.
    Phyre_PScripting_PCallerScript_pushVararg_PChar_00618630 @ 0x0061_8630 => extern "C" fn s_00618630(ty: u32, va: u32, l: u32) -> u32 {
        b::va_push(&mut mem(), &s::VA_PUSHERS[0], ty, va, l)
    }
    /// `Phyre_PScripting_PCallerScript_pushVararg_PUInt8_00618670`.
    Phyre_PScripting_PCallerScript_pushVararg_PUInt8_00618670 @ 0x0061_8670 => extern "C" fn s_00618670(ty: u32, va: u32, l: u32) -> u32 {
        b::va_push(&mut mem(), &s::VA_PUSHERS[1], ty, va, l)
    }
    /// `Phyre_PScripting_PCallerScript_pushVararg_PInt16_006186b0`.
    Phyre_PScripting_PCallerScript_pushVararg_PInt16_006186b0 @ 0x0061_86b0 => extern "C" fn s_006186b0(ty: u32, va: u32, l: u32) -> u32 {
        b::va_push(&mut mem(), &s::VA_PUSHERS[2], ty, va, l)
    }
    /// `Phyre_PScripting_PCallerScript_pushVararg_PUInt16_006186f0`.
    Phyre_PScripting_PCallerScript_pushVararg_PUInt16_006186f0 @ 0x0061_86f0 => extern "C" fn s_006186f0(ty: u32, va: u32, l: u32) -> u32 {
        b::va_push(&mut mem(), &s::VA_PUSHERS[3], ty, va, l)
    }
    /// `Phyre_PScripting_PCallerScript_pushVararg_PInt32_00618730`.
    Phyre_PScripting_PCallerScript_pushVararg_PInt32_00618730 @ 0x0061_8730 => extern "C" fn s_00618730(ty: u32, va: u32, l: u32) -> u32 {
        b::va_push(&mut mem(), &s::VA_PUSHERS[4], ty, va, l)
    }
    /// `Phyre_PScripting_PCallerScript_pushVararg_unsigned_int_00618770`.
    Phyre_PScripting_PCallerScript_pushVararg_unsigned_int_00618770 @ 0x0061_8770 => extern "C" fn s_00618770(ty: u32, va: u32, l: u32) -> u32 {
        b::va_push(&mut mem(), &s::VA_PUSHERS[5], ty, va, l)
    }
    /// `Phyre_PScripting_PCallerScript_pushVararg_float_006187b0`.
    Phyre_PScripting_PCallerScript_pushVararg_float_006187b0 @ 0x0061_87b0 => extern "C" fn s_006187b0(ty: u32, va: u32, l: u32) -> u32 {
        b::va_push(&mut mem(), &s::VA_PUSHERS[6], ty, va, l)
    }
    /// `Phyre_PScripting_PCallerScript_pushVararg_double_00618800`.
    Phyre_PScripting_PCallerScript_pushVararg_double_00618800 @ 0x0061_8800 => extern "C" fn s_00618800(ty: u32, va: u32, l: u32) -> u32 {
        b::va_push(&mut mem(), &s::VA_PUSHERS[7], ty, va, l)
    }
    /// `Phyre_PScripting_PCallerScript_pushVararg_PBase_00618850`.
    Phyre_PScripting_PCallerScript_pushVararg_PBase_00618850 @ 0x0061_8850 => extern "C" fn s_00618850(ty: u32, va: u32, l: u32) -> u32 {
        b::va_push(&mut mem(), &s::VA_PUSHERS[8], ty, va, l)
    }
    /// `Phyre_PScripting_PCallerScript_pushVararg_int_004381b0_006188b0`.
    Phyre_PScripting_PCallerScript_pushVararg_int_004381b0_006188b0 @ 0x0061_88b0 => extern "C" fn s_006188b0(ty: u32, va: u32, l: u32) -> u32 {
        b::va_push(&mut mem(), &s::VA_PUSHERS[9], ty, va, l)
    }
    /// `Phyre_PScripting_PCallerScript_pushVararg_PInt64_006188f0`.
    Phyre_PScripting_PCallerScript_pushVararg_PInt64_006188f0 @ 0x0061_88f0 => extern "C" fn s_006188f0(ty: u32, va: u32, l: u32) -> u32 {
        b::va_push(&mut mem(), &s::VA_PUSHERS[10], ty, va, l)
    }
    /// `Phyre_PScripting_PCallerScript_pushVararg_PUInt64_00618940`.
    Phyre_PScripting_PCallerScript_pushVararg_PUInt64_00618940 @ 0x0061_8940 => extern "C" fn s_00618940(ty: u32, va: u32, l: u32) -> u32 {
        b::va_push(&mut mem(), &s::VA_PUSHERS[11], ty, va, l)
    }
    /// `Phyre_PScripting_PCallerScript_popValue_PChar_00618a40`.
    Phyre_PScripting_PCallerScript_popValue_PChar_00618a40 @ 0x0061_8a40 => extern "C" fn s_00618a40(ty: u32, l: u32, out: u32) -> u32 {
        b::pop(&mut mem(), &s::POPPERS[0], ty, l, out)
    }
    /// `Phyre_PScripting_PCallerScript_popValue_PUInt8_00618a90`.
    Phyre_PScripting_PCallerScript_popValue_PUInt8_00618a90 @ 0x0061_8a90 => extern "C" fn s_00618a90(ty: u32, l: u32, out: u32) -> u32 {
        b::pop(&mut mem(), &s::POPPERS[1], ty, l, out)
    }
    /// `Phyre_PScripting_PCallerScript_popValue_PInt16_00618ae0`.
    Phyre_PScripting_PCallerScript_popValue_PInt16_00618ae0 @ 0x0061_8ae0 => extern "C" fn s_00618ae0(ty: u32, l: u32, out: u32) -> u32 {
        b::pop(&mut mem(), &s::POPPERS[2], ty, l, out)
    }
    /// `Phyre_PScripting_PCallerScript_popValue_PUInt16_00618b30`.
    Phyre_PScripting_PCallerScript_popValue_PUInt16_00618b30 @ 0x0061_8b30 => extern "C" fn s_00618b30(ty: u32, l: u32, out: u32) -> u32 {
        b::pop(&mut mem(), &s::POPPERS[3], ty, l, out)
    }
    /// `Phyre_PScripting_PCallerScript_popValue_PInt32_00618b80`.
    Phyre_PScripting_PCallerScript_popValue_PInt32_00618b80 @ 0x0061_8b80 => extern "C" fn s_00618b80(ty: u32, l: u32, out: u32) -> u32 {
        b::pop(&mut mem(), &s::POPPERS[4], ty, l, out)
    }
    /// `Phyre_PScripting_PCallerScript_popValue_unsigned_int_00618bd0`.
    Phyre_PScripting_PCallerScript_popValue_unsigned_int_00618bd0 @ 0x0061_8bd0 => extern "C" fn s_00618bd0(ty: u32, l: u32, out: u32) -> u32 {
        b::pop(&mut mem(), &s::POPPERS[5], ty, l, out)
    }
    /// `Phyre_PScripting_PCallerScript_popValue_float_00618c20`.
    Phyre_PScripting_PCallerScript_popValue_float_00618c20 @ 0x0061_8c20 => extern "C" fn s_00618c20(ty: u32, l: u32, out: u32) -> u32 {
        b::pop(&mut mem(), &s::POPPERS[6], ty, l, out)
    }
    /// `Phyre_PScripting_PCallerScript_popValue_double_00618c70`.
    Phyre_PScripting_PCallerScript_popValue_double_00618c70 @ 0x0061_8c70 => extern "C" fn s_00618c70(ty: u32, l: u32, out: u32) -> u32 {
        b::pop(&mut mem(), &s::POPPERS[7], ty, l, out)
    }
    /// `Phyre_PScripting_PCallerScript_popValue_PInt64_00618d00`.
    Phyre_PScripting_PCallerScript_popValue_PInt64_00618d00 @ 0x0061_8d00 => extern "C" fn s_00618d00(ty: u32, l: u32, out: u32) -> u32 {
        b::pop(&mut mem(), &s::POPPERS[8], ty, l, out)
    }
    /// `Phyre_PScripting_PCallerScript_popValue_PUInt64_00618d50`.
    Phyre_PScripting_PCallerScript_popValue_PUInt64_00618d50 @ 0x0061_8d50 => extern "C" fn s_00618d50(ty: u32, l: u32, out: u32) -> u32 {
        b::pop(&mut mem(), &s::POPPERS[9], ty, l, out)
    }
    /// `Phyre_PScripting_PCallerScript_popValue_bool_00618da0`.
    Phyre_PScripting_PCallerScript_popValue_bool_00618da0 @ 0x0061_8da0 => extern "C" fn s_00618da0(ty: u32, l: u32, out: u32) -> u32 {
        b::pop(&mut mem(), &s::POPPERS[10], ty, l, out)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PNameComponent_dtor_0045a800`.
    Phyre_PClassDescriptorForType_Phyre_PNameComponent_dtor_0045a800 @ 0x0045_a800 => extern "thiscall" fn s_0045a800(this: u32) -> u32 {
        b::tail_dtor(&mut mem(), &s::TAIL_DTORS[0], this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PGameplay_PScriptableComponent_dtor_00552c50`.
    Phyre_PClassDescriptorForType_Phyre_PGameplay_PScriptableComponent_dtor_00552c50 @ 0x0055_2c50 => extern "thiscall" fn s_00552c50(this: u32) -> u32 {
        b::tail_dtor(&mut mem(), &s::TAIL_DTORS[1], this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PGameplay_PScriptedComponent_dtor_00553a40`.
    Phyre_PClassDescriptorForType_Phyre_PGameplay_PScriptedComponent_dtor_00553a40 @ 0x0055_3a40 => extern "thiscall" fn s_00553a40(this: u32) -> u32 {
        b::tail_dtor(&mut mem(), &s::TAIL_DTORS[2], this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PScripting_PScheduler_dtor_005cf2f0`.
    Phyre_PClassDescriptorForType_Phyre_PScripting_PScheduler_dtor_005cf2f0 @ 0x005c_f2f0 => extern "thiscall" fn s_005cf2f0(this: u32) -> u32 {
        b::tail_dtor(&mut mem(), &s::TAIL_DTORS[3], this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PScripting_PScriptCallbackHandler_dtor_005d38e0`.
    Phyre_PClassDescriptorForType_Phyre_PScripting_PScriptCallbackHandler_dtor_005d38e0 @ 0x005d_38e0 => extern "thiscall" fn s_005d38e0(this: u32) -> u32 {
        b::tail_dtor(&mut mem(), &s::TAIL_DTORS[4], this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PScripting_PScript_dtor_005d4760`.
    Phyre_PClassDescriptorForType_Phyre_PScripting_PScript_dtor_005d4760 @ 0x005d_4760 => extern "thiscall" fn s_005d4760(this: u32) -> u32 {
        b::tail_dtor(&mut mem(), &s::TAIL_DTORS[5], this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PScripting_PAsyncProcessHeader_dtor_00617880`.
    Phyre_PClassDescriptorForType_Phyre_PScripting_PAsyncProcessHeader_dtor_00617880 @ 0x0061_7880 => extern "thiscall" fn s_00617880(this: u32) -> u32 {
        b::tail_dtor(&mut mem(), &s::TAIL_DTORS[6], this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PScripting_PClassCallableMethodScript_dtor_006193d0`.
    Phyre_PClassDescriptorForType_Phyre_PScripting_PClassCallableMethodScript_dtor_006193d0 @ 0x0061_93d0 => extern "thiscall" fn s_006193d0(this: u32) -> u32 {
        b::tail_dtor(&mut mem(), &s::TAIL_DTORS[7], this)
    }
    /// `Phyre_PClassDescriptorForType_ClassDynamicMesh_dtor_00672670`.
    Phyre_PClassDescriptorForType_ClassDynamicMesh_dtor_00672670 @ 0x0067_2670 => extern "thiscall" fn s_00672670(this: u32) -> u32 {
        b::tail_dtor(&mut mem(), &s::TAIL_DTORS[8], this)
    }
    /// `Phyre_PClassDescriptorForType_ClassDynamicMeshInstance_dtor_00673620`.
    Phyre_PClassDescriptorForType_ClassDynamicMeshInstance_dtor_00673620 @ 0x0067_3620 => extern "thiscall" fn s_00673620(this: u32) -> u32 {
        b::tail_dtor(&mut mem(), &s::TAIL_DTORS[9], this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_ClothInstancingDynamicMesh_dtor_006e0a60`.
    Phyre_PClassDescriptorForType_Phyre_ClothInstancingDynamicMesh_dtor_006e0a60 @ 0x006e_0a60 => extern "thiscall" fn s_006e0a60(this: u32) -> u32 {
        b::tail_dtor(&mut mem(), &s::TAIL_DTORS[10], this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_ClothInstancingDynamicMeshInstance_dtor_006e0a70`.
    Phyre_PClassDescriptorForType_Phyre_ClothInstancingDynamicMeshInstance_dtor_006e0a70 @ 0x006e_0a70 => extern "thiscall" fn s_006e0a70(this: u32) -> u32 {
        b::tail_dtor(&mut mem(), &s::TAIL_DTORS[11], this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_DynamicMeshDefaultImplmenetation_dtor_006e0a80`.
    Phyre_PClassDescriptorForType_Phyre_DynamicMeshDefaultImplmenetation_dtor_006e0a80 @ 0x006e_0a80 => extern "thiscall" fn s_006e0a80(this: u32) -> u32 {
        b::tail_dtor(&mut mem(), &s::TAIL_DTORS[12], this)
    }
}
