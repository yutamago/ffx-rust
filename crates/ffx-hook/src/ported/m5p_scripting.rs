//! M5 stage 1, list `phyre-scripting`: `PhyreEngine`'s scripting support
//! outside the script layer (`ffx_pother_a::m5p_scripting*`). The parity
//! tests are `crates/ffx-oracle/tests/m5p_scripting.rs`.
//!
//! Threads: the script runtime runs on the main thread; the async process
//! headers can be processed from a worker. No shim keeps state; each works
//! on the game's memory through [`mem`].

use ffx_pother_a::m5p_scripting_lua::{self as lb, bridge as lua};
use ffx_pother_a::m5p_scripting_mid::{self as mid, bridge as m};
use ffx_pother_a::m5p_scripting_more::bridge as mo;
use ffx_pother_a::m5p_scripting_run::bridge as ru;
use ffx_pother_a::m5p_scripting_sched::bridge as sc;
use ffx_pother_a::m5p_scripting_small::{self as s, bridge as b};
use ffx_pother_a::m5p_scripting_state::bridge as st;
use ffx_pother_a::m5p_scripting_vec::{self as ve_, bridge as ve};

use super::{StackFrame, mem, ported};

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
    // ---- batch 2: middle-sized Lua glue and fixups ----
    /// `Phyre_PScripting_pushVector_00572150`.
    Phyre_PScripting_pushVector_00572150 @ 0x0057_2150 => extern "C" fn s_00572150(l: u32, v: u32) -> u32 {
        m::push_vector(&mut mem(), l, v)
    }
    /// `Phyre_PNameComponent_linkAll_0045ad00`.
    Phyre_PNameComponent_linkAll_0045ad00 @ 0x0045_ad00 => extern "C" fn s_0045ad00(p: u32) -> u32 {
        m::link_names(&mut mem(), p)
    }
    /// `Phyre_PScripting_readArray_004363b0`.
    Phyre_PScripting_readArray_004363b0 @ 0x0043_63b0 => extern "C" fn s_004363b0(l: u32, n: u32, at: u32, ty: u32) -> u32 {
        m::read_array(&mut mem(), l, n, at, ty)
    }
    /// `Phyre_PScripting_runChunk_005d65f0`.
    Phyre_PScripting_runChunk_005d65f0 @ 0x005d_65f0 => extern "C" fn s_005d65f0(pl: u32) -> u32 {
        m::run_chunk(&mut mem(), pl)
    }
    /// `Phyre_PScripting_PUtilityScripting_scalarDeletingDtor_005d52d0`.
    Phyre_PScripting_PUtilityScripting_scalarDeletingDtor_005d52d0 @ 0x005d_52d0 => extern "thiscall" fn s_005d52d0(this: u32, flags: u32) -> u32 {
        m::registrant_dtor(&mut mem(), this, flags)
    }
    /// `Phyre_PScripting_luaAlloc_005d5ba0`.
    Phyre_PScripting_luaAlloc_005d5ba0 @ 0x005d_5ba0 => extern "C" fn s_005d5ba0(_ud: u32, old: u32, old_size: u32, new_size: u32) -> u32 {
        m::lua_alloc(&mut mem(), old, old_size, new_size)
    }
    /// `Phyre_PScripting_loadScript_00436750`.
    Phyre_PScripting_loadScript_00436750 @ 0x0043_6750 => extern "C" fn s_00436750(l: u32, s: u32) -> u32 {
        m::load_script(&mut mem(), l, s)
    }
    /// `Phyre_PScripting_PScheduler_reapFinished_005d1220`.
    Phyre_PScripting_PScheduler_reapFinished_005d1220 @ 0x005d_1220 => extern "thiscall" fn s_005d1220(this: u32) {
        m::reap_finished(&mut mem(), this);
    }
    /// `Phyre_PScripting_pushValues_00436e20`.
    Phyre_PScripting_pushValues_00436e20 @ 0x0043_6e20 => extern "C" fn s_00436e20(l: u32, n: u32, at: u32, ty: u32) -> u32 {
        m::push_values(&mut mem(), l, n, at, ty)
    }
    /// `Phyre_PScripting_PScheduler_hasEntry_005d0e20`.
    Phyre_PScripting_PScheduler_hasEntry_005d0e20 @ 0x005d_0e20 => extern "thiscall" fn s_005d0e20(this: u32, owner: u32, key: u32, name: u32) -> u8 {
        m::has_entry(&mut mem(), this, owner, key, name)
    }
    /// `Phyre_PRandomGenerator_nextFloat_0045ea50`.
    Phyre_PRandomGenerator_nextFloat_0045ea50 @ 0x0045_ea50 => extern "C" fn s_0045ea50() -> f32 {
        m::random_float(&mut mem())
    }
    /// `Phyre_PRendering_PContextSwitchLights_readBits_004d3ef0`.
    Phyre_PRendering_PContextSwitchLights_readBits_004d3ef0 @ 0x004d_3ef0 => extern "thiscall" fn s_004d3ef0(this: u32, start: u32, n: u32) -> u32 {
        m::read_bits(&mem(), this, start, n)
    }
    /// `Phyre_PScripting_GetLibraryOpener`.
    Phyre_PScripting_GetLibraryOpener @ 0x005d_5710 => extern "C" fn s_005d5710(name: u32) -> u32 {
        m::library_opener(&mem(), name)
    }
    /// `Phyre_PScripting_checkPhyreObject_00436160`.
    Phyre_PScripting_checkPhyreObject_00436160 @ 0x0043_6160 => extern "C" fn s_00436160(l: u32, idx: u32) -> u32 {
        m::check_userdata(&mut mem(), mid::USERDATA_CHECKS[0].1, l, idx)
    }
    /// `Phyre_PScripting_checkVector_00570dc0`.
    Phyre_PScripting_checkVector_00570dc0 @ 0x0057_0dc0 => extern "C" fn s_00570dc0(l: u32, idx: u32) -> u32 {
        m::check_userdata(&mut mem(), mid::USERDATA_CHECKS[1].1, l, idx)
    }
    /// `Phyre_PPostProcessing_PDepthOfField_initialize_00a1b710`.
    Phyre_PPostProcessing_PDepthOfField_initialize_00a1b710 @ 0x00a1_b710 => extern "thiscall" fn s_00a1b710(this: u32) -> u32 {
        m::dof_init(&mut mem(), this)
    }
    /// `Phyre_PScripting_componentIndex_00436240`.
    Phyre_PScripting_componentIndex_00436240 @ 0x0043_6240 => extern "C" fn s_00436240(l: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        m::component_index(&mut mem(), l, f.at())
    }
    /// `Phyre_PSerialization_PStreamWriterFile_scalarDeletingDtor_009f07a0`.
    Phyre_PSerialization_PStreamWriterFile_scalarDeletingDtor_009f07a0 @ 0x009f_07a0 => extern "thiscall" fn s_009f07a0(this: u32, flags: u32) -> u32 {
        m::writer_dtor(&mut mem(), this, flags)
    }
    /// `Phyre_PScripting_RegisterFunctions`.
    Phyre_PScripting_RegisterFunctions @ 0x005d_6070 => extern "C" fn s_005d6070(l: u32) -> u32 {
        m::register_functions(&mut mem(), l)
    }
    /// `Phyre_PClassDataMemberDynamic_fixObjects_00446650`.
    Phyre_PClassDataMemberDynamic_fixObjects_00446650 @ 0x0044_6650 => extern "C" fn s_00446650(p: u32) -> u32 {
        m::fix_dynamic_members(&mut mem(), p)
    }
    /// `Phyre_PPostProcessing_PDeferredLighting_fixObjects_00a07e80`.
    Phyre_PPostProcessing_PDeferredLighting_fixObjects_00a07e80 @ 0x00a0_7e80 => extern "C" fn s_00a07e80(p: u32) -> u32 {
        m::fix_postfx(&mut mem(), mid::POSTFX_FIXUPS[0].1, p)
    }
    /// `Phyre_PPostProcessing_PFXAA_fixObjects_00a07fb0`.
    Phyre_PPostProcessing_PFXAA_fixObjects_00a07fb0 @ 0x00a0_7fb0 => extern "C" fn s_00a07fb0(p: u32) -> u32 {
        m::fix_postfx(&mut mem(), mid::POSTFX_FIXUPS[1].1, p)
    }
    /// `Phyre_PPostProcessing_PGlow_fixObjects_00a08030`.
    Phyre_PPostProcessing_PGlow_fixObjects_00a08030 @ 0x00a0_8030 => extern "C" fn s_00a08030(p: u32) -> u32 {
        m::fix_postfx(&mut mem(), mid::POSTFX_FIXUPS[2].1, p)
    }
    /// `Phyre_PPostProcessing_PMotionBlur_fixObjects_00a080b0`.
    Phyre_PPostProcessing_PMotionBlur_fixObjects_00a080b0 @ 0x00a0_80b0 => extern "C" fn s_00a080b0(p: u32) -> u32 {
        m::fix_postfx(&mut mem(), mid::POSTFX_FIXUPS[3].1, p)
    }
    /// `Phyre_PPostProcessing_PScreenSpaceReflection_fixObjects_00a08130`.
    Phyre_PPostProcessing_PScreenSpaceReflection_fixObjects_00a08130 @ 0x00a0_8130 => extern "C" fn s_00a08130(p: u32) -> u32 {
        m::fix_postfx(&mut mem(), mid::POSTFX_FIXUPS[4].1, p)
    }
    /// `Phyre_PScripting_upvalueCall_00437160`.
    Phyre_PScripting_upvalueCall_00437160 @ 0x0043_7160 => extern "C" fn s_00437160(l: u32) -> u32 {
        m::upvalue_call(&mut mem(), l)
    }
    /// `Phyre_PScripting_upvalueCall_004371e0`.
    Phyre_PScripting_upvalueCall_004371e0 @ 0x0043_71e0 => extern "C" fn s_004371e0(l: u32) -> u32 {
        m::upvalue_call(&mut mem(), l)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_bind_00541c00`.
    Phyre_PAnimation_PAnimatableComponent_bind_00541c00 @ 0x0054_1c00 => extern "thiscall" fn s_00541c00(this: u32) -> u32 {
        m::animatable_bind(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_setSlot_00543280`.
    Phyre_PAnimation_PAnimatableComponent_setSlot_00543280 @ 0x0054_3280 => extern "thiscall" fn s_00543280(this: u32, index: u32, target: u32) -> u32 {
        m::animatable_set_slot(&mut mem(), this, index, target)
    }
    /// `Phyre_PAnimation_PAnimatableComponent_fixObjects_00540c10`.
    Phyre_PAnimation_PAnimatableComponent_fixObjects_00540c10 @ 0x0054_0c10 => extern "C" fn s_00540c10(p: u32) -> u32 {
        m::fix_animatables(&mut mem(), p)
    }
    /// `Phyre_PLODGroup_fixObjects_005a5110`.
    Phyre_PLODGroup_fixObjects_005a5110 @ 0x005a_5110 => extern "C" fn s_005a5110(p: u32) -> u32 {
        m::lod_fixup(&mut mem(), p, false)
    }
    /// `Phyre_PLODLevel_fixObjects_005a5190`.
    Phyre_PLODLevel_fixObjects_005a5190 @ 0x005a_5190 => extern "C" fn s_005a5190(p: u32) -> u32 {
        m::lod_fixup(&mut mem(), p, true)
    }
    /// `Phyre_PScripting_PClassCallableMethodScript_fixObjects_00619980`.
    Phyre_PScripting_PClassCallableMethodScript_fixObjects_00619980 @ 0x0061_9980 => extern "C" fn s_00619980(p: u32) -> u32 {
        m::fix_method_scripts(&mut mem(), p)
    }
    /// `Phyre_PScripting_loadedModule_005d5a90`.
    Phyre_PScripting_loadedModule_005d5a90 @ 0x005d_5a90 => extern "C" fn s_005d5a90(l: u32, name: u32, keep: u32) -> u32 {
        m::loaded_module(&mut mem(), l, name, keep)
    }
    /// `Phyre_PSerialization_PStreamWriterFile_create_009f0810`.
    Phyre_PSerialization_PStreamWriterFile_create_009f0810 @ 0x009f_0810 => extern "C" fn s_009f0810(path: u32) -> u32 {
        m::new_writer(&mut mem(), path)
    }
    /// `Phyre_PScripting_closeState_005d5450`.
    Phyre_PScripting_closeState_005d5450 @ 0x005d_5450 => extern "C" fn s_005d5450(l: u32) -> u32 {
        b::close_state(&mut mem(), l)
    }
    // ---- batch 3: the object accessors, Vector and Matrix functions ----
    /// `Phyre_PScripting_PScriptAccessors_PObjectAccessor_Phyre_PBase__Get`.
    Phyre_PScripting_PScriptAccessors_PObjectAccessor_Phyre_PBase__Get @ 0x0043_5650 => extern "C" fn s_00435650(l: u32) -> u32 {
        lua::accessor_get(&mut mem(), &lb::ACCESSORS[0], l)
    }
    /// `Phyre_PScripting_PScriptAccessors_PObjectAccessor_Phyre_PClusterDependencyList__Get_0044b970`.
    Phyre_PScripting_PScriptAccessors_PObjectAccessor_Phyre_PClusterDependencyList__Get_0044b970 @ 0x0044_b970 => extern "C" fn s_0044b970(l: u32) -> u32 {
        lua::accessor_get(&mut mem(), &lb::ACCESSORS[1], l)
    }
    /// `Phyre_PScripting_PScriptAccessors_PObjectAccessor_Phyre_PRendering_PNodeContext__Get_004cfe80`.
    Phyre_PScripting_PScriptAccessors_PObjectAccessor_Phyre_PRendering_PNodeContext__Get_004cfe80 @ 0x004c_fe80 => extern "C" fn s_004cfe80(l: u32) -> u32 {
        lua::accessor_get(&mut mem(), &lb::ACCESSORS[2], l)
    }
    /// `Phyre_PScripting_PScriptAccessors_PObjectAccessor_Phyre_PGeometry_PMeshSegmentD3D11__Get_00560450`.
    Phyre_PScripting_PScriptAccessors_PObjectAccessor_Phyre_PGeometry_PMeshSegmentD3D11__Get_00560450 @ 0x0056_0450 => extern "C" fn s_00560450(l: u32) -> u32 {
        lua::accessor_get(&mut mem(), &lb::ACCESSORS[3], l)
    }
    /// `Phyre_PScripting_PScriptAccessors_PObjectAccessor_Phyre_PClassDescriptor__Get`.
    Phyre_PScripting_PScriptAccessors_PObjectAccessor_Phyre_PClassDescriptor__Get @ 0x0043_bad0 => extern "C" fn s_0043bad0(l: u32) -> u32 {
        lua::accessor_get(&mut mem(), &lb::ACCESSORS[4], l)
    }
    /// `Phyre_PScripting_PScriptAccessors_PObjectAccessor_Phyre_PAnimation_PTimeController__Get_00521d80`.
    Phyre_PScripting_PScriptAccessors_PObjectAccessor_Phyre_PAnimation_PTimeController__Get_00521d80 @ 0x0052_1d80 => extern "C" fn s_00521d80(l: u32) -> u32 {
        lua::accessor_get(&mut mem(), &lb::ACCESSORS[5], l)
    }
    /// `Phyre_PScripting_PScriptAccessors_PObjectAccessor_Phyre_PGameplay_PTimerComponent__Get_005557f0`.
    Phyre_PScripting_PScriptAccessors_PObjectAccessor_Phyre_PGameplay_PTimerComponent__Get_005557f0 @ 0x0055_57f0 => extern "C" fn s_005557f0(l: u32) -> u32 {
        lua::accessor_get(&mut mem(), &lb::ACCESSORS[6], l)
    }
    /// `Phyre_PScripting_PScriptAccessors_PObjectAccessor_Phyre_PEntity__Get`.
    Phyre_PScripting_PScriptAccessors_PObjectAccessor_Phyre_PEntity__Get @ 0x0044_f7f0 => extern "C" fn s_0044f7f0(l: u32) -> u32 {
        lua::entity_get(&mut mem(), l)
    }
    /// `Phyre_PScripting_Vector_length3_00573160`.
    Phyre_PScripting_Vector_length3_00573160 @ 0x0057_3160 => extern "C" fn s_00573160(l: u32) -> u32 {
        lua::vector_length(&mut mem(), l, 3, true)
    }
    /// `Phyre_PScripting_Vector_length4_00573230`.
    Phyre_PScripting_Vector_length4_00573230 @ 0x0057_3230 => extern "C" fn s_00573230(l: u32) -> u32 {
        lua::vector_length(&mut mem(), l, 4, true)
    }
    /// `Phyre_PScripting_Vector_lengthSquared3_00573300`.
    Phyre_PScripting_Vector_lengthSquared3_00573300 @ 0x0057_3300 => extern "C" fn s_00573300(l: u32) -> u32 {
        lua::vector_length(&mut mem(), l, 3, false)
    }
    /// `Phyre_PScripting_Vector_lengthSquared4_005733c0`.
    Phyre_PScripting_Vector_lengthSquared4_005733c0 @ 0x0057_33c0 => extern "C" fn s_005733c0(l: u32) -> u32 {
        lua::vector_length(&mut mem(), l, 4, false)
    }
    /// `Phyre_PScripting_Matrix_scale_00571480`.
    Phyre_PScripting_Matrix_scale_00571480 @ 0x0057_1480 => extern "C" fn s_00571480(l: u32) -> u32 {
        lua::matrix_scale(&mut mem(), l)
    }
    /// `Phyre_PScripting_Matrix_translation_00571540`.
    Phyre_PScripting_Matrix_translation_00571540 @ 0x0057_1540 => extern "C" fn s_00571540(l: u32) -> u32 {
        lua::matrix_translation(&mut mem(), l)
    }
    /// `Phyre_PScripting_Matrix_inverse_005719a0`.
    Phyre_PScripting_Matrix_inverse_005719a0 @ 0x0057_19a0 => extern "C" fn s_005719a0(l: u32) -> u32 {
        let mut f = StackFrame::<{(lb::FRAME_BYTES / 4) as usize}>::new();
        lua::unary(&mut mem(), &lb::UNARY[0], l, f.at())
    }
    /// `Phyre_PScripting_Matrix_transpose_00571e00`.
    Phyre_PScripting_Matrix_transpose_00571e00 @ 0x0057_1e00 => extern "C" fn s_00571e00(l: u32) -> u32 {
        let mut f = StackFrame::<{(lb::FRAME_BYTES / 4) as usize}>::new();
        lua::unary(&mut mem(), &lb::UNARY[1], l, f.at())
    }
    /// `Phyre_PScripting_Vector_normalize4_00573860`.
    Phyre_PScripting_Vector_normalize4_00573860 @ 0x0057_3860 => extern "C" fn s_00573860(l: u32) -> u32 {
        let mut f = StackFrame::<{(lb::FRAME_BYTES / 4) as usize}>::new();
        lua::unary(&mut mem(), &lb::UNARY[2], l, f.at())
    }
    /// `Phyre_PScripting_Vector_normalize3_00573760`.
    Phyre_PScripting_Vector_normalize3_00573760 @ 0x0057_3760 => extern "C" fn s_00573760(l: u32) -> u32 {
        let mut f = StackFrame::<{(lb::FRAME_BYTES / 4) as usize}>::new();
        lua::unary(&mut mem(), &lb::UNARY[3], l, f.at())
    }
    /// `Phyre_PScripting_Matrix_rotation_005713e0`.
    Phyre_PScripting_Matrix_rotation_005713e0 @ 0x0057_13e0 => extern "C" fn s_005713e0(l: u32) -> u32 {
        let mut f = StackFrame::<{(lb::FRAME_BYTES / 4) as usize}>::new();
        lua::matrix_rotation(&mut mem(), l, f.at())
    }
    // ---- batch 4: the scheduler, scripted components, fixups ----
    /// `Phyre_PScripting_PScheduler_addEntry_005d01d0`.
    Phyre_PScripting_PScheduler_addEntry_005d01d0 @ 0x005d_01d0 => extern "thiscall" fn s_005d01d0(this: u32, a0: u32, a1: u32, a2: u32, a3: u32, a4: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        sc::scheduler_add(&mut mem(), this, [a0, a1, a2, a3, a4], f.at())
    }
    /// `Phyre_PScripting_PScheduler_leastPhase_005d0a80`.
    Phyre_PScripting_PScheduler_leastPhase_005d0a80 @ 0x005d_0a80 => extern "thiscall" fn s_005d0a80(this: u32, n: u32) -> u32 {
        sc::least_phase(&mem(), this, n)
    }
    /// `Phyre_PScripting_PScheduler_insertEntry_005d0280`.
    Phyre_PScripting_PScheduler_insertEntry_005d0280 @ 0x005d_0280 => extern "thiscall" fn s_005d0280(this: u32, a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        sc::scheduler_insert(&mut mem(), this, [a0, a1, a2, a3, a4, a5], f.at())
    }
    /// `Phyre_PGameplay_PScriptedComponent_setFunctionName_00553440`.
    Phyre_PGameplay_PScriptedComponent_setFunctionName_00553440 @ 0x0055_3440 => extern "thiscall" fn s_00553440(this: u32, name: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        sc::set_function_name(&mut mem(), this, name, f.at())
    }
    /// `Phyre_PGameplay_PScriptedComponent_run_00554b60`.
    Phyre_PGameplay_PScriptedComponent_run_00554b60 @ 0x0055_4b60 => extern "thiscall" fn s_00554b60(this: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        sc::scripted_run(&mut mem(), this, f.at())
    }
    /// `Phyre_PGameplay_PScriptedComponent_slot2_00554b50`.
    Phyre_PGameplay_PScriptedComponent_slot2_00554b50 @ 0x0055_4b50 => extern "thiscall" fn s_00554b50(this: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        sc::scripted_run(&mut mem(), this, f.at())
    }
    /// `Phyre_PGameplay_PScriptedComponent_assign_00553b80`.
    Phyre_PGameplay_PScriptedComponent_assign_00553b80 @ 0x0055_3b80 => extern "thiscall" fn s_00553b80(this: u32, src: u32) -> u32 {
        sc::scripted_assign(&mut mem(), this, src)
    }
    /// `Phyre_PNameComponent_find_0045ac30`.
    Phyre_PNameComponent_find_0045ac30 @ 0x0045_ac30 => extern "C" fn s_0045ac30(name: u32) -> u32 {
        sc::find_name_component(&mem(), name)
    }
    /// `Phyre_PLODGroup_setExternalBuffer_005a80f0`.
    Phyre_PLODGroup_setExternalBuffer_005a80f0 @ 0x005a_80f0 => extern "thiscall" fn s_005a80f0(this: u32, buf: u32, n: u32) -> u32 {
        sc::sharray_set_external(&mut mem(), this, buf, n, 1)
    }
    /// `Phyre_PLODLevel_setExternalBuffer_005a8190`.
    Phyre_PLODLevel_setExternalBuffer_005a8190 @ 0x005a_8190 => extern "thiscall" fn s_005a8190(this: u32, buf: u32, n: u32) -> u32 {
        sc::sharray_set_external(&mut mem(), this, buf, n, 3)
    }
    /// `Phyre_PClassDescriptorDynamic_fixObjects_00448680`.
    Phyre_PClassDescriptorDynamic_fixObjects_00448680 @ 0x0044_8680 => extern "C" fn s_00448680(p: u32) -> u32 {
        sc::fix_dynamic_classes(&mut mem(), p)
    }
    /// `Phyre_PScripting_PClassCallableMethodScript_dtor_00619400`.
    Phyre_PScripting_PClassCallableMethodScript_dtor_00619400 @ 0x0061_9400 => extern "thiscall" fn s_00619400(this: u32) -> u32 {
        sc::method_script_dtor(&mut mem(), this)
    }
    /// `Phyre_PAnimation_constructSlots_00541780`.
    Phyre_PAnimation_constructSlots_00541780 @ 0x0054_1780 => extern "C" fn s_00541780(array: u32, n: u32) {
        sc::construct_slots(&mut mem(), array, n);
    }
    /// `Phyre_PPostProcessing_PDepthOfField_fixObjects_00a07f00`.
    Phyre_PPostProcessing_PDepthOfField_fixObjects_00a07f00 @ 0x00a0_7f00 => extern "C" fn s_00a07f00(p: u32) -> u32 {
        sc::fix_dof(&mut mem(), p)
    }
    // ---- batch 5: script states and runs, member setters ----
    /// `PScripting_NewStateForScript`.
    PScripting_NewStateForScript @ 0x005d_53b0 => extern "C" fn s_005d53b0(script: u32, alloc: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        st::new_state(&mut mem(), script, alloc, f.at())
    }
    /// `PScriptedComponent_createThread`.
    PScriptedComponent_createThread @ 0x0055_4860 => extern "thiscall" fn s_00554860(this: u32, l: u32) -> u32 {
        let mut f = StackFrame::<2>::new();
        let cell = f.at();
        st::create_thread(&mut mem(), this, l, cell, cell.wrapping_add(4))
    }
    /// `Phyre_PScripting_VectorNewIndexHandler`.
    Phyre_PScripting_VectorNewIndexHandler @ 0x0057_36c0 => extern "C" fn s_005736c0(l: u32) -> u32 {
        st::vector_newindex(&mut mem(), l)
    }
    /// `Phyre_PScripting_MatrixNewIndexHandler`.
    Phyre_PScripting_MatrixNewIndexHandler @ 0x0057_1d60 => extern "C" fn s_00571d60(l: u32) -> u32 {
        st::matrix_newindex(&mut mem(), l)
    }
    /// `Phyre_PScripting_PScheduledScript_execute`.
    Phyre_PScripting_PScheduledScript_execute @ 0x0061_8330 => extern "thiscall" fn s_00618330(this: u32, l: u32, function: u32) -> u32 {
        st::execute(&mut mem(), this, l, function)
    }
    /// `Phyre_PScripting_PScheduledScript_executePersistent`.
    Phyre_PScripting_PScheduledScript_executePersistent @ 0x0061_8490 => extern "thiscall" fn s_00618490(this: u32, function: u32) -> u32 {
        st::execute_persistent(&mut mem(), this, function)
    }
    // ---- batch 6: scheduler removal, Vector constructor, type queries ----
    /// `Phyre_PScripting_PScheduler_removeEntry_005d1370`.
    Phyre_PScripting_PScheduler_removeEntry_005d1370 @ 0x005d_1370 => extern "thiscall" fn s_005d1370(this: u32, owner: u32, key: u32, name: u32) -> u32 {
        mo::scheduler_remove(&mut mem(), this, owner, key, name)
    }
    /// `Phyre_PScripting_PScheduler_setEntryStep_005d1670`.
    Phyre_PScripting_PScheduler_setEntryStep_005d1670 @ 0x005d_1670 => extern "thiscall" fn s_005d1670(this: u32, owner: u32, key: u32, name: u32, step: u32) -> u32 {
        mo::scheduler_set_step(&mut mem(), this, owner, key, name, step)
    }
    /// `Phyre_PScripting_Vector_new_00570f80`.
    Phyre_PScripting_Vector_new_00570f80 @ 0x0057_0f80 => extern "C" fn s_00570f80(l: u32) -> u32 {
        mo::vector_new(&mut mem(), l)
    }
    /// `Phyre_PScripting_PhyreGetTypeOf`.
    Phyre_PScripting_PhyreGetTypeOf @ 0x0043_6b90 => extern "C" fn s_00436b90(l: u32) -> u32 {
        mo::get_type_of(&mut mem(), l)
    }
    /// `Phyre_PScripting_objectsEqual_00436aa0`.
    Phyre_PScripting_objectsEqual_00436aa0 @ 0x0043_6aa0 => extern "C" fn s_00436aa0(l: u32) -> u32 {
        mo::objects_equal(&mut mem(), l)
    }
    // ---- batch 7: the scheduler runs ----
    /// `Phyre_PScripting_PScheduler_tick_005d0910`.
    Phyre_PScripting_PScheduler_tick_005d0910 @ 0x005d_0910 => extern "thiscall" fn s_005d0910(this: u32, frame: u32) -> u32 {
        ru::tick(&mut mem(), this, frame)
    }
    /// `Phyre_PScripting_PScheduler_runList_005d0670`.
    Phyre_PScripting_PScheduler_runList_005d0670 @ 0x005d_0670 => extern "thiscall" fn s_005d0670(this: u32, list: u32) -> u32 {
        ru::run_list(&mut mem(), this, list)
    }
    /// `Phyre_PScripting_PScheduler_runNow_005d07d0`.
    Phyre_PScripting_PScheduler_runNow_005d07d0 @ 0x005d_07d0 => extern "thiscall" fn s_005d07d0(this: u32, owner: u32, key: u32, name: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        ru::run_now(&mut mem(), this, owner, key, name, f.at())
    }
    // ---- batch 8: Vector dot and cross products ----
    /// `Phyre_PScripting_Vector_dot3_00572a40`.
    Phyre_PScripting_Vector_dot3_00572a40 @ 0x0057_2a40 => extern "C" fn s_00572a40(l: u32) -> u32 {
        ve::dot(&mut mem(), l, 3)
    }
    /// `Phyre_PScripting_Vector_dot4_00572b70`.
    Phyre_PScripting_Vector_dot4_00572b70 @ 0x0057_2b70 => extern "C" fn s_00572b70(l: u32) -> u32 {
        ve::dot(&mut mem(), l, 4)
    }
    /// `Phyre_PScripting_Vector_cross_00572690`.
    Phyre_PScripting_Vector_cross_00572690 @ 0x0057_2690 => extern "C" fn s_00572690(l: u32) -> u32 {
        ve::cross(&mut mem(), l)
    }
    // ---- batch 9: Vector arithmetic ----
    /// `Phyre_PScripting_Vector_add_00572450`.
    Phyre_PScripting_Vector_add_00572450 @ 0x0057_2450 => extern "C" fn s_00572450(l: u32) -> u32 {
        ve::arith(&mut mem(), l, ve_::ARITH[0].1)
    }
    /// `Phyre_PScripting_Vector_sub_00573930`.
    Phyre_PScripting_Vector_sub_00573930 @ 0x0057_3930 => extern "C" fn s_00573930(l: u32) -> u32 {
        ve::arith(&mut mem(), l, ve_::ARITH[1].1)
    }
    /// `Phyre_PScripting_Vector_mul_00573480`.
    Phyre_PScripting_Vector_mul_00573480 @ 0x0057_3480 => extern "C" fn s_00573480(l: u32) -> u32 {
        ve::arith(&mut mem(), l, ve_::ARITH[2].1)
    }
    /// `Phyre_PScripting_Vector_div_00572810`.
    Phyre_PScripting_Vector_div_00572810 @ 0x0057_2810 => extern "C" fn s_00572810(l: u32) -> u32 {
        ve::arith(&mut mem(), l, ve_::ARITH[3].1)
    }
    // ---- batch 10: Vector and Matrix equality ----
    /// `Phyre_PScripting_Vector_equals_00572c80`.
    Phyre_PScripting_Vector_equals_00572c80 @ 0x0057_2c80 => extern "C" fn s_00572c80(l: u32) -> u32 {
        ve::equals(&mut mem(), l, ve_::EQUALS[0].1, ve_::EQUALS[0].2)
    }
    /// `Phyre_PScripting_Matrix_equals_00571600`.
    Phyre_PScripting_Matrix_equals_00571600 @ 0x0057_1600 => extern "C" fn s_00571600(l: u32) -> u32 {
        ve::equals(&mut mem(), l, ve_::EQUALS[1].1, ve_::EQUALS[1].2)
    }
    // ---- batch 11: bit-packed pairs, mesh target binding ----
    /// `Phyre_PRendering_PContextSwitchLights_decodePairs_004d3d30`.
    Phyre_PRendering_PContextSwitchLights_decodePairs_004d3d30 @ 0x004d_3d30 => extern "C" fn s_004d3d30(out_a: u32, out_b: u32, cursor: u32) -> u32 {
        mo::decode_pairs(&mut mem(), out_a, out_b, cursor)
    }
    /// `Phyre_PAnimation_PAnimationChannelTargetBindSceneToMeshInstance_bindTargets_005ce320`.
    Phyre_PAnimation_PAnimationChannelTargetBindSceneToMeshInstance_bindTargets_005ce320 @ 0x005c_e320 => extern "thiscall" fn s_005ce320(this: u32, out: u32, targets: u32, n: u32) -> u32 {
        mo::bind_mesh_targets(&mut mem(), this, out, targets, n)
    }
    // ---- batch 12: the Phyre namespace index ----
    /// `Phyre_PScripting_PhyreIndex_00436c80`.
    Phyre_PScripting_PhyreIndex_00436c80 @ 0x0043_6c80 => extern "C" fn s_00436c80(l: u32) -> u32 {
        mo::phyre_index(&mut mem(), l)
    }
}
