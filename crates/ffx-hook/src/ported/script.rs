//! `ffx-script`: the Atel VM's ctrl works, workers, stacks and variables.

use ffx_script::vm::bridge as atel;
use ffx_script::vm::bridge::globals as g;
use ffx_script::vm::bridge::native;
use ffx_script::vm::bridge::run;
use ffx_script::vm::bridge::signal as sig;

use super::{mem, ported};

ported! {
    /// `AtelGetBasicWork(index)`: a worker of the current ctrl work.
    AtelGetBasicWork @ 0x0086_a830 => extern "C" fn atel_get_basic_work(index: i32) -> u32 {
        atel::atel_get_basic_work(&mut mem(), index)
    }
    /// `AtelGetActorType(index)`.
    AtelGetActorType @ 0x0086_a7b0 => extern "C" fn atel_get_actor_type(index: i32) -> u32 {
        atel::atel_get_actor_type(&mut mem(), index)
    }
    /// `AtelGetActorId(worker)`.
    AtelGetActorId @ 0x0086_a5d0 => extern "C" fn atel_get_actor_id(w: u32) -> u32 {
        atel::atel_get_actor_id(&mem(), w)
    }
    /// `AtelGetCameraWorkAdrs(worker)`.
    AtelGetCameraWorkAdrs @ 0x0086_ae50 => extern "C" fn atel_get_camera_work_adrs(w: u32) -> u32 {
        atel::atel_get_camera_work_adrs(w)
    }
    /// `AtelGetBattleIdAdrs2(worker)`.
    AtelGetBattleIdAdrs2 @ 0x0086_a990 => extern "C" fn atel_get_battle_id_adrs2(w: u32) -> u32 {
        atel::atel_get_battle_id_adrs2(w)
    }
    /// `AtelGetCharHandle(worker)`.
    AtelGetCharHandle @ 0x0086_ae80 => extern "C" fn atel_get_char_handle(w: u32) -> u32 {
        atel::atel_get_char_handle(&mem(), w)
    }
    /// `AtelGetPosWorkAdrs(worker)`.
    AtelGetPosWorkAdrs @ 0x0086_c2b0 => extern "C" fn atel_get_pos_work_adrs(w: u32) -> u32 {
        atel::atel_get_pos_work_adrs(&mem(), w)
    }
    /// `Script_WorkerLevel(worker, level)`.
    Script_WorkerLevel @ 0x0087_2650 => extern "C" fn script_worker_level(w: u32, level: i32) -> u32 {
        atel::script_worker_level(&mem(), w, level)
    }
    /// `AtelGetActorNumber(block)`.
    AtelGetActorNumber @ 0x0086_a650 => extern "C" fn atel_get_actor_number(b: u32) -> u32 {
        atel::atel_get_actor_number(&mem(), b)
    }
    /// `AtelGetNamespaceNumber(block)`.
    AtelGetNamespaceNumber @ 0x0086_c190 => extern "C" fn atel_get_namespace_number(b: u32) -> u32 {
        atel::atel_get_namespace_number(&mem(), b)
    }
    /// `AtelGetIndexWork(block, i)`.
    AtelGetIndexWork @ 0x0086_bb80 => extern "C" fn atel_get_index_work(b: u32, i: u32) -> u32 {
        atel::atel_get_index_work(&mem(), b, i)
    }
    /// `AtelGetBinHdrExpandAddChr(block)`.
    AtelGetBinHdrExpandAddChr @ 0x0086_aa20 => extern "C" fn atel_get_bin_hdr_expand_add_chr(b: u32) -> u32 {
        atel::atel_get_bin_hdr_expand_add_chr(&mem(), b)
    }
    /// `AtelGetActorBufferSize(block)`.
    AtelGetActorBufferSize @ 0x0086_a290 => extern "C" fn atel_get_actor_buffer_size(b: u32) -> u32 {
        atel::atel_get_actor_buffer_size(&mem(), b)
    }
    /// `AtelGetCtrlWork(i)`.
    AtelGetCtrlWork @ 0x0086_afa0 => extern "C" fn atel_get_ctrl_work(i: u32) -> u32 {
        atel::atel_get_ctrl_work(&mem(), i)
    }
    /// `AtelGetCurCtrlWork()`.
    AtelGetCurCtrlWork @ 0x0086_aff0 => extern "C" fn atel_get_cur_ctrl_work() -> u32 {
        atel::atel_get_cur_ctrl_work(&mem())
    }
    /// `AtelChangeCurrentCtrl(i)`.
    AtelChangeCurrentCtrl @ 0x0086_2d30 => extern "C" fn atel_change_current_ctrl(i: u32) -> u32 {
        atel::atel_change_current_ctrl(&mut mem(), i)
    }
    /// `AtelSetCurrentCtrl(ctrl)`.
    AtelSetCurrentCtrl @ 0x0086_f730 => extern "C" fn atel_set_current_ctrl(c: u32) -> u32 {
        atel::atel_set_current_ctrl(&mut mem(), c)
    }
    /// `AtelGetCtrlworkActorTotal(ctrl)`.
    AtelGetCtrlworkActorTotal @ 0x0086_afe0 => extern "C" fn atel_get_ctrlwork_actor_total(c: u32) -> u32 {
        atel::atel_get_ctrlwork_actor_total(&mem(), c)
    }
    /// `AtelGetNowCtrlTopId()`.
    AtelGetNowCtrlTopId @ 0x0086_c1a0 => extern "C" fn atel_get_now_ctrl_top_id() -> u32 {
        atel::atel_get_now_ctrl_top_id(&mem())
    }
    /// `AtelGetTotalActorNumber2()`.
    AtelGetTotalActorNumber2 @ 0x0086_c510 => extern "C" fn atel_get_total_actor_number2() -> u32 {
        atel::atel_get_total_actor_number2(&mem())
    }
    /// `AtelGetEventActorTotal()`.
    AtelGetEventActorTotal @ 0x0086_b300 => extern "C" fn atel_get_event_actor_total() -> u32 {
        atel::atel_get_event_actor_total(&mem())
    }
    /// `AtelGetEventMapCameraId()`.
    AtelGetEventMapCameraId @ 0x0086_b5d0 => extern "C" fn atel_get_event_map_camera_id() -> u32 {
        atel::atel_get_event_map_camera_id(&mem())
    }
    /// `AtelRemoveCtrlWork(i)`.
    AtelRemoveCtrlWork @ 0x0086_ed40 => extern "C" fn atel_remove_ctrl_work(i: u32) {
        atel::atel_remove_ctrl_work(&mut mem(), i);
    }
    /// `AtelGetMesWinWork(i)`.
    AtelGetMesWinWork @ 0x0086_be90 => extern "C" fn atel_get_mes_win_work(i: i32) -> u32 {
        atel::atel_get_mes_win_work(&mem(), i)
    }
    /// `AtelPushStackInteger(worker, stack, value)`.
    AtelPushStackInteger @ 0x0086_e3f0 => extern "C" fn atel_push_stack_integer(_w: u32, s: u32, value: u32) {
        atel::atel_push_stack_integer(&mut mem(), s, value);
    }
    /// `AtelPushStackFloat(worker, stack, value)`, the float as its bits.
    AtelPushStackFloat @ 0x0086_e3c0 => extern "C" fn atel_push_stack_float(_w: u32, s: u32, bits: u32) {
        atel::atel_push_stack_float(&mut mem(), s, bits);
    }
    /// `AtelPopStackInteger(worker, stack)`.
    AtelPopStackInteger @ 0x0086_df00 => extern "C" fn atel_pop_stack_integer(_w: u32, s: u32) -> u32 {
        atel::atel_pop_stack_integer(&mut mem(), s)
    }
    /// `Script_TopInt(worker, pop, stack)`.
    Script_TopInt @ 0x0086_df40 => extern "C" fn script_top_int(_w: u32, pop: u32, s: u32) -> u32 {
        atel::script_top_int(&mut mem(), pop, s)
    }
    /// `AtelPopStackFloat(worker, stack)`.
    AtelPopStackFloat @ 0x0086_de50 => extern "C" fn atel_pop_stack_float(_w: u32, s: u32) -> f32 {
        f32::from_bits(atel::atel_pop_stack_float(&mut mem(), s))
    }
    /// `AtelGetRealVarAdrs(worker, word)`.
    AtelGetRealVarAdrs @ 0x0086_c2e0 => extern "C" fn atel_get_real_var_adrs(w: u32, word: u32) -> u32 {
        atel::atel_get_real_var_adrs(&mut mem(), w, word)
    }
    /// `AtelGetVarAdrs(worker, v)`.
    AtelGetVarAdrs @ 0x0086_c7d0 => extern "C" fn atel_get_var_adrs(w: u32, v: u32) -> u32 {
        atel::atel_get_var_adrs(&mut mem(), w, v)
    }
    /// `AtelGetValueAdrs(worker, v, index)`.
    AtelGetValueAdrs @ 0x0086_c570 => extern "C" fn atel_get_value_adrs(w: u32, v: u32, index: i32) -> u32 {
        atel::atel_get_value_adrs(&mut mem(), w, v, index)
    }
    /// `AtelGetValueInteger(worker, v, index)`.
    AtelGetValueInteger @ 0x0086_c730 => extern "C" fn atel_get_value_integer(w: u32, v: u32, index: i32) -> u32 {
        atel::atel_get_value_integer(&mut mem(), w, v, index)
    }
    /// `AtelGetValueFloat(worker, v, index)`.
    AtelGetValueFloat @ 0x0086_c610 => extern "C" fn atel_get_value_float(w: u32, v: u32, index: i32) -> f32 {
        f32::from_bits(atel::atel_get_value_float(&mut mem(), w, v, index))
    }
    /// `AtelGetValueIncValue(type)`.
    AtelGetValueIncValue @ 0x0086_c6e0 => extern "C" fn atel_get_value_inc_value(kind: u32) -> u32 {
        atel::atel_get_value_inc_value(kind)
    }
    /// `AtelSetValueInteger(worker, v, index, value, clamp)`.
    AtelSetValueInteger @ 0x0087_1400 => extern "C" fn atel_set_value_integer(w: u32, v: u32, index: i32, value: i32, clamp: u32) {
        atel::atel_set_value_integer(&mut mem(), w, v, index, value, clamp);
    }
    /// `AtelSetValueFloat(worker, v, index, value, clamp)`, the float as its bits.
    AtelSetValueFloat @ 0x0087_12e0 => extern "C" fn atel_set_value_float(w: u32, v: u32, index: i32, bits: u32, clamp: u32) {
        atel::atel_set_value_float(&mut mem(), w, v, index, bits, clamp);
    }
    /// `AtelGetVarInfoAdrs(worker, v)`.
    AtelGetVarInfoAdrs @ 0x0086_c850 => extern "C" fn atel_get_var_info_adrs(w: u32, v: u32) -> u32 {
        atel::atel_get_var_info_adrs(&mem(), w, v)
    }
    /// `AtelGetVarType(worker, v)`.
    AtelGetVarType @ 0x0086_c870 => extern "C" fn atel_get_var_type(w: u32, v: u32) -> u32 {
        atel::atel_get_var_type(&mem(), w, v)
    }
    /// `AtelGetVarArrayMax(worker, v)`.
    AtelGetVarArrayMax @ 0x0086_c7f0 => extern "C" fn atel_get_var_array_max(w: u32, v: u32) -> u32 {
        atel::atel_get_var_array_max(&mem(), w, v)
    }
    /// `AtelGetVarArrayNumber(worker, v)` (the same code).
    AtelGetVarArrayNumber @ 0x0086_c810 => extern "C" fn atel_get_var_array_number(w: u32, v: u32) -> u32 {
        atel::atel_get_var_array_max(&mem(), w, v)
    }
    /// `AtelGetVarArrayRange(worker, v)`.
    AtelGetVarArrayRange @ 0x0086_c830 => extern "C" fn atel_get_var_array_range(w: u32, v: u32) -> u32 {
        atel::atel_get_var_array_range(&mem(), w, v)
    }
    /// `Script_PushVar(worker, stack, v, index)`.
    Script_PushVar @ 0x0086_e440 => extern "C" fn script_push_var(w: u32, s: u32, v: u32, index: i32) {
        atel::script_push_var(&mut mem(), w, s, v, index);
    }
    /// `Script_PushVarElem(worker, stack, v)`.
    Script_PushVarElem @ 0x0086_e100 => extern "C" fn script_push_var_elem(w: u32, s: u32, v: u32) {
        atel::script_push_var_elem(&mut mem(), w, s, v);
    }
    /// `Script_StoreVar(worker, stack, v, mode)`.
    Script_StoreVar @ 0x0086_dfb0 => extern "C" fn script_store_var(w: u32, s: u32, v: u32, mode: u32) -> u32 {
        atel::script_store_var(&mut mem(), w, s, v, mode)
    }
    /// `Script_PushVarRef(worker, stack, v)`.
    Script_PushVarRef @ 0x0086_e160 => extern "C" fn script_push_var_ref(w: u32, s: u32, v: u32) {
        atel::script_push_var_ref(&mut mem(), w, s, v);
    }

    /// `AtelGetMusicNumber()`.
    AtelGetMusicNumber @ 0x0087_3b00 => extern "C" fn atel_get_music_number() -> u32 {
        g::atel_get_music_number(&mem())
    }
    /// `AtelRegistMusicNumber(n)`.
    AtelRegistMusicNumber @ 0x0087_3d00 => extern "C" fn atel_regist_music_number(n: u32) {
        g::atel_regist_music_number(&mut mem(), n);
    }
    /// `AtelGetEffectFlag(bit)`.
    AtelGetEffectFlag @ 0x0086_b030 => extern "C" fn atel_get_effect_flag(bit: u32) -> u32 {
        g::atel_get_effect_flag(&mem(), bit)
    }
    /// `AtelSetEffectFlag(bit)`.
    AtelSetEffectFlag @ 0x0086_fa40 => extern "C" fn atel_set_effect_flag(bit: u32) {
        g::atel_set_effect_flag(&mut mem(), bit);
    }
    /// `AtelAllClearEffectFlag()`.
    AtelAllClearEffectFlag @ 0x0086_0fd0 => extern "C" fn atel_all_clear_effect_flag() {
        g::atel_all_clear_effect_flag(&mut mem());
    }
    /// `AtelGetEffectParam(i)`.
    AtelGetEffectParam @ 0x0086_b050 => extern "C" fn atel_get_effect_param(i: u32) -> u32 {
        g::atel_get_effect_param(&mem(), i)
    }
    /// `AtelClearEffectParam()`.
    AtelClearEffectParam @ 0x0086_3880 => extern "C" fn atel_clear_effect_param() {
        g::atel_clear_effect_param(&mut mem());
    }
    /// `AtelSetSnd3dVolumeDiffRatio(ratio)`, the float as its bits.
    AtelSetSnd3dVolumeDiffRatio @ 0x0087_7450 => extern "C" fn atel_set_snd3d_volume_diff_ratio(bits: u32) {
        g::atel_set_snd3d_volume_diff_ratio(&mut mem(), bits);
    }
    /// `AtelSetSnd3dVolumeTotalRatio(ratio)`, the float as its bits.
    AtelSetSnd3dVolumeTotalRatio @ 0x0087_7460 => extern "C" fn atel_set_snd3d_volume_total_ratio(bits: u32) {
        g::atel_set_snd3d_volume_total_ratio(&mut mem(), bits);
    }
    /// `AtelClearSnd3dWorkAll()`.
    AtelClearSnd3dWorkAll @ 0x0087_7330 => extern "C" fn atel_clear_snd3d_work_all() {
        g::atel_clear_snd3d_work_all(&mut mem());
    }
    /// `AtelStrcat(dst, src)`.
    AtelStrcat @ 0x0087_1950 => extern "C" fn atel_strcat(dst: u32, src: u32) -> u32 {
        g::atel_strcat(&mut mem(), dst, src)
    }
    /// `AtelStrcpy(dst, src)`.
    AtelStrcpy @ 0x0087_1980 => extern "C" fn atel_strcpy(dst: u32, src: u32) -> u32 {
        g::atel_strcpy(&mut mem(), dst, src)
    }
    /// `AtelRegistAlbhedDic(a, b)`.
    AtelRegistAlbhedDic @ 0x0086_e820 => extern "C" fn atel_regist_albhed_dic(a: u32, b: u32) {
        g::atel_regist_albhed_dic(&mut mem(), a, b);
    }
    /// `AtelSetForceBattleMapId(id)`.
    AtelSetForceBattleMapId @ 0x0087_0130 => extern "C" fn atel_set_force_battle_map_id(id: u32) {
        g::atel_set_force_battle_map_id(&mut mem(), id);
    }
    /// `AtelSetVibrateStreamExt(v)`.
    AtelSetVibrateStreamExt @ 0x0087_15e0 => extern "C" fn atel_set_vibrate_stream_ext(v: u32) {
        g::atel_set_vibrate_stream_ext(&mut mem(), v);
    }
    /// `AtelCheckCallbackQue(i)`.
    AtelCheckCallbackQue @ 0x0087_5000 => extern "C" fn atel_check_callback_que(i: u32) -> u32 {
        g::atel_check_callback_que(&mem(), i)
    }
    /// `AtelIsCallbackQue()`.
    AtelIsCallbackQue @ 0x0087_5180 => extern "C" fn atel_is_callback_que() -> u32 {
        g::atel_is_callback_que(&mem())
    }
    /// `AtelSetEncountBattleEndStatus(a, b, c)`.
    AtelSetEncountBattleEndStatus @ 0x0086_fc20 => extern "C" fn atel_set_encount_battle_end_status(a: u32, b: u32, c: u32) {
        g::atel_set_encount_battle_end_status(&mut mem(), a, b, c);
    }
    /// `AtelInitBattleEndStatus()`.
    AtelInitBattleEndStatus @ 0x0086_cff0 => extern "C" fn atel_init_battle_end_status() {
        g::atel_init_battle_end_status(&mut mem());
    }
    /// `AtelSetBattleEndKeepRecoverType(v)`.
    AtelSetBattleEndKeepRecoverType @ 0x0086_f3a0 => extern "C" fn atel_set_battle_end_keep_recover_type(v: u32) {
        g::atel_set_battle_end_keep_recover_type(&mut mem(), v);
    }
    /// `AtelGetStatusWinFadeOut()`.
    AtelGetStatusWinFadeOut @ 0x0086_c4d0 => extern "C" fn atel_get_status_win_fade_out() -> u32 {
        g::atel_get_status_win_fade_out(&mem())
    }
    /// `AtelSetMCFPInWater(a, b, c, d)`.
    AtelSetMCFPInWater @ 0x0087_0280 => extern "C" fn atel_set_mcfp_in_water(a: u32, b: u32, c: u32, d: u32) {
        g::atel_set_mcfp_in_water(&mut mem(), a, b, c, d);
    }
    /// `AtelSetMenuExecStatus(bank)` (the low byte).
    AtelSetMenuExecStatus @ 0x0087_02e0 => extern "C" fn atel_set_menu_exec_status(bank: u32) {
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        g::atel_set_menu_exec_status(&mut mem(), bank as u8);
    }
    /// `AtelSetShopRatio(r)`.
    AtelSetShopRatio @ 0x0087_0f60 => extern "C" fn atel_set_shop_ratio(r: u32) {
        g::atel_set_shop_ratio(&mut mem(), r);
    }
    /// `AtelEventGetMotionIndex(a, b)`.
    AtelEventGetMotionIndex @ 0x0086_5890 => extern "C" fn atel_event_get_motion_index(a: u32, b: u32) -> u32 {
        g::atel_event_get_motion_index(a, b)
    }
    /// `AtelSetMapIdPage(page)` (the low byte).
    AtelSetMapIdPage @ 0x0087_3e60 => extern "C" fn atel_set_map_id_page(page: u32) {
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        g::atel_set_map_id_page(&mut mem(), page as u8);
    }
    /// `AtelClearPreBgPage()`.
    AtelClearPreBgPage @ 0x0087_2b90 => extern "C" fn atel_clear_pre_bg_page() {
        g::atel_clear_pre_bg_page(&mut mem());
    }
    /// `AtelGetEventMode()`.
    AtelGetEventMode @ 0x0087_3aa0 => extern "C" fn atel_get_event_mode() -> u32 {
        g::atel_get_event_mode(&mem())
    }
    /// `AtelClearPauseFlag()`.
    AtelClearPauseFlag @ 0x0087_2b70 => extern "C" fn atel_clear_pause_flag() {
        g::atel_clear_pause_flag(&mut mem());
    }
    /// `AtelRegistBattleEndMovieFadeout(a, b, c)`.
    AtelRegistBattleEndMovieFadeout @ 0x0086_e840 => extern "C" fn atel_regist_battle_end_movie_fadeout(a: u32, b: u32, c: u32) {
        g::atel_regist_battle_end_movie_fadeout(&mut mem(), a, b, c);
    }
    /// `AtelInitMotionRequest()`.
    AtelInitMotionRequest @ 0x0086_d4c0 => extern "C" fn atel_init_motion_request() {
        g::atel_init_motion_request(&mut mem());
    }
    /// `getMegasAdding()`.
    getMegasAdding @ 0x0086_0a90 => extern "C" fn get_megas_adding() -> u32 {
        g::get_megas_adding(&mem())
    }
    /// `AtelCheckMenuOtakaraFlag()`.
    AtelCheckMenuOtakaraFlag @ 0x0087_2b50 => extern "C" fn atel_check_menu_otakara_flag() -> u32 {
        g::atel_check_menu_otakara_flag(&mem())
    }
    /// `AtelGetMenuMask()`.
    AtelGetMenuMask @ 0x0086_be50 => extern "C" fn atel_get_menu_mask() -> u32 {
        g::atel_get_menu_mask(&mem())
    }
    /// `AtelClearAllMenuMask()`.
    AtelClearAllMenuMask @ 0x0086_3620 => extern "C" fn atel_clear_all_menu_mask() {
        g::atel_clear_all_menu_mask(&mut mem());
    }
    /// `AtelGetExpandFTCAdrs()`.
    AtelGetExpandFTCAdrs @ 0x0087_3ac0 => extern "C" fn atel_get_expand_ftc_adrs() -> u32 {
        g::atel_get_expand_ftc_adrs(&mem())
    }
    /// `AtelClearNotAuronPatchFlag()`.
    AtelClearNotAuronPatchFlag @ 0x0086_3f90 => extern "C" fn atel_clear_not_auron_patch_flag() {
        g::atel_clear_not_auron_patch_flag(&mut mem());
    }
    /// `AtelClearLoveParamDisable()`.
    AtelClearLoveParamDisable @ 0x0086_3ae0 => extern "C" fn atel_clear_love_param_disable() {
        g::atel_clear_love_param_disable(&mut mem());
    }
    /// `AtelSetOverlayStatus(s)` (the low byte).
    AtelSetOverlayStatus @ 0x0087_0900 => extern "C" fn atel_set_overlay_status(s: u32) {
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        g::atel_set_overlay_status(&mut mem(), s as u8);
    }
    /// `AtelEventGetNowCtrlTopId()`.
    AtelEventGetNowCtrlTopId @ 0x0086_58b0 => extern "C" fn atel_event_get_now_ctrl_top_id() -> u32 {
        g::atel_event_get_now_ctrl_top_id(&mem())
    }
    /// `AtelGetCurrentActorTotal()`.
    AtelGetCurrentActorTotal @ 0x0086_b000 => extern "C" fn atel_get_current_actor_total() -> u32 {
        g::atel_get_current_actor_total(&mem())
    }
    /// `AtelGetCtrlWorkStatus(i)`.
    AtelGetCtrlWorkStatus @ 0x0086_afc0 => extern "C" fn atel_get_ctrl_work_status(i: u32) -> u32 {
        g::atel_get_ctrl_work_status(&mem(), i)
    }
    /// `AtelGetActorExecspaceID(worker)`.
    AtelGetActorExecspaceID @ 0x0086_a420 => extern "C" fn atel_get_actor_execspace_id(w: u32) -> u32 {
        g::atel_get_actor_execspace_id(&mem(), w)
    }
    /// `AtelGetGideMapActorNumber()`.
    AtelGetGideMapActorNumber @ 0x0086_b710 => extern "C" fn atel_get_gide_map_actor_number() -> u32 {
        g::atel_get_gide_map_actor_number(&mem())
    }

    /// `Script_Run(worker, mode, callbacks)`: the interpreter; natives and
    /// the request families stay original.
    Script_Run @ 0x0086_41e0 => extern "C" fn script_run(w: u32, mode: u32, callbacks: u32) -> u32 {
        run::script_run(&mut mem(), &mut run::OriginalNatives, w, mode, callbacks)
    }
    /// `AtelFetchCommand(worker, level)`.
    AtelFetchCommand @ 0x0086_9d60 => extern "C" fn atel_fetch_command(_w: u32, l: u32) -> u32 {
        run::atel_fetch_command(&mem(), l)
    }
    /// `AtelFetchCommandMain(long, pc)`.
    AtelFetchCommandMain @ 0x0086_9da0 => extern "C" fn atel_fetch_command_main(long: u32, pc: u32) -> u32 {
        run::atel_fetch_command_main(&mem(), long, pc)
    }
    /// `AtelFetchStackInteger(worker, stack)`.
    AtelFetchStackInteger @ 0x0086_a000 => extern "C" fn atel_fetch_stack_integer(_w: u32, s: u32) -> u32 {
        run::atel_fetch_stack_integer(&mut mem(), s)
    }
    /// `Script_TopTwoInts(worker, stack)`.
    Script_TopTwoInts @ 0x0086_d9e0 => extern "C" fn script_top_two_ints(_w: u32, s: u32) -> u32 {
        run::script_top_two_ints(&mem(), s)
    }
    /// `Script_Jump(worker, level, label)`.
    Script_Jump @ 0x0087_26f0 => extern "C" fn script_jump(w: u32, l: u32, label: u32) {
        run::script_jump(&mut mem(), w, l, label);
    }

    /// `AtelGetControlerT()`.
    AtelGetControlerT @ 0x0086_af30 => extern "C" fn atel_get_controler_t() -> u16 {
        g::atel_get_controler_t(&mem())
    }
    /// `AtelGetControlerRept()`.
    AtelGetControlerRept @ 0x0086_af50 => extern "C" fn atel_get_controler_rept() -> u16 {
        g::atel_get_controler_rept(&mem())
    }
    /// `AtelGetControlerHist1c()`.
    AtelGetControlerHist1c @ 0x0086_af60 => extern "C" fn atel_get_controler_hist1c() -> u16 {
        g::atel_get_controler_hist1c(&mem())
    }
    /// `AtelGetControler()`.
    AtelGetControler @ 0x0086_af70 => extern "C" fn atel_get_controler() -> u16 {
        g::atel_get_controler(&mem())
    }
    /// `AtelGetControlerHist08()`.
    AtelGetControlerHist08 @ 0x0086_af80 => extern "C" fn atel_get_controler_hist08() -> u16 {
        g::atel_get_controler_hist08(&mem())
    }
    /// `AtelGetHalfTurnAngle()`.
    AtelGetHalfTurnAngle @ 0x0087_3ad0 => extern "C" fn atel_get_half_turn_angle() -> f32 {
        f32::from_bits(g::atel_get_half_turn_angle(&mem()))
    }
    /// `AtelGetOverlayStatus()`.
    AtelGetOverlayStatus @ 0x0086_c200 => extern "C" fn atel_get_overlay_status() -> u32 {
        g::atel_get_overlay_status(&mem())
    }
    /// `AtelGetVoicePan()`.
    AtelGetVoicePan @ 0x0087_3b80 => extern "C" fn atel_get_voice_pan() -> u32 {
        g::atel_get_voice_pan(&mem())
    }
    /// `AtelGetVagStreamVolume()`.
    AtelGetVagStreamVolume @ 0x0087_3b90 => extern "C" fn atel_get_vag_stream_volume() -> u32 {
        g::atel_get_vag_stream_volume(&mem())
    }
    /// `AtelGetBattleStatus()`.
    AtelGetBattleStatus @ 0x0086_a9b0 => extern "C" fn atel_get_battle_status() -> u32 {
        g::atel_get_battle_status(&mem())
    }
    /// `AtelGetKeepPlayStatusWhiteInOut()`.
    AtelGetKeepPlayStatusWhiteInOut @ 0x0086_bc30 => extern "C" fn atel_get_keep_play_status_white_in_out() -> u32 {
        g::atel_get_keep_play_status_white_in_out(&mem())
    }
    /// `AtelGetKeepPlayStatusNoStopFanfare()`.
    AtelGetKeepPlayStatusNoStopFanfare @ 0x0086_bc60 => extern "C" fn atel_get_keep_play_status_no_stop_fanfare() -> u32 {
        g::atel_get_keep_play_status_no_stop_fanfare(&mem())
    }
    /// `AtelIsKeepPlayStatusClear12()`.
    AtelIsKeepPlayStatusClear12 @ 0x0086_bc10 => extern "C" fn atel_is_keep_play_status_clear12() -> u32 {
        g::atel_is_keep_play_status_clear12(&mem())
    }
    /// `AtelIsKeepPlayStatusClear8()`.
    AtelIsKeepPlayStatusClear8 @ 0x0086_bc20 => extern "C" fn atel_is_keep_play_status_clear8() -> u32 {
        g::atel_is_keep_play_status_clear8(&mem())
    }
    /// `AtelIsKeepPlayStatusClear11()`.
    AtelIsKeepPlayStatusClear11 @ 0x0086_bc50 => extern "C" fn atel_is_keep_play_status_clear11() -> u32 {
        g::atel_is_keep_play_status_clear11(&mem())
    }
    /// `AtelIsBattleEndStatusClear8()`.
    AtelIsBattleEndStatusClear8 @ 0x0086_c280 => extern "C" fn atel_is_battle_end_status_clear8() -> u32 {
        g::atel_is_battle_end_status_clear8(&mem())
    }
    /// `AtelIsBattleEndStatusClear9()`.
    AtelIsBattleEndStatusClear9 @ 0x0086_c290 => extern "C" fn atel_is_battle_end_status_clear9() -> u32 {
        g::atel_is_battle_end_status_clear9(&mem())
    }
    /// `AtelGetBattleSceneGroup()`.
    AtelGetBattleSceneGroup @ 0x0086_a9a0 => extern "C" fn atel_get_battle_scene_group() -> u32 {
        g::atel_get_battle_scene_group(&mem())
    }
    /// `AtelGetBattleStatusDummyEnc()`.
    AtelGetBattleStatusDummyEnc @ 0x0086_a9c0 => extern "C" fn atel_get_battle_status_dummy_enc() -> u32 {
        g::atel_get_battle_status_dummy_enc(&mem())
    }
    /// `AtelGetBattleStatusSys()`.
    AtelGetBattleStatusSys @ 0x0086_a9d0 => extern "C" fn atel_get_battle_status_sys() -> u32 {
        g::atel_get_battle_status_sys(&mem())
    }
    /// `AtelGetSave20()`.
    AtelGetSave20 @ 0x0086_d9d0 => extern "C" fn atel_get_save20() -> u32 {
        g::atel_get_save20(&mem())
    }
    /// `AtelGetOnMemoryMovieFileNo()`.
    AtelGetOnMemoryMovieFileNo @ 0x0087_3b10 => extern "C" fn atel_get_on_memory_movie_file_no() -> u32 {
        g::atel_get_on_memory_movie_file_no(&mem())
    }
    /// `AtelGetSaveD8()`.
    AtelGetSaveD8 @ 0x0087_3b20 => extern "C" fn atel_get_save_d8() -> u32 {
        g::atel_get_save_d8(&mem())
    }
    /// `AtelGetSaveDC()`.
    AtelGetSaveDC @ 0x0087_3b30 => extern "C" fn atel_get_save_dc() -> u32 {
        g::atel_get_save_dc(&mem())
    }
    /// `AtelGetRandomEncountStatus()`.
    AtelGetRandomEncountStatus @ 0x0087_3b40 => extern "C" fn atel_get_random_encount_status() -> u32 {
        g::atel_get_random_encount_status(&mem())
    }
    /// `AtelGetScenarioFlag()`.
    AtelGetScenarioFlag @ 0x0086_c470 => extern "C" fn atel_get_scenario_flag() -> u32 {
        g::atel_get_scenario_flag(&mem())
    }
    /// `AtelGetAlbhedRikku()`.
    AtelGetAlbhedRikku @ 0x0086_a7e0 => extern "C" fn atel_get_albhed_rikku() -> u32 {
        g::atel_get_albhed_rikku(&mem())
    }
    /// `AtelGetSphereMonitor()`.
    AtelGetSphereMonitor @ 0x0086_c4b0 => extern "C" fn atel_get_sphere_monitor() -> u32 {
        g::atel_get_sphere_monitor(&mem())
    }
    /// `AtelGetWaterBattleEffect()`.
    AtelGetWaterBattleEffect @ 0x0086_c8a0 => extern "C" fn atel_get_water_battle_effect() -> u32 {
        g::atel_get_water_battle_effect(&mem())
    }
    /// `AtelGetBattleEndTagAlways()`.
    AtelGetBattleEndTagAlways @ 0x0087_3a90 => extern "C" fn atel_get_battle_end_tag_always() -> u32 {
        g::atel_get_battle_end_tag_always(&mem())
    }
    /// `AtelGetMapMarkerX()`.
    AtelGetMapMarkerX @ 0x0087_3ae0 => extern "C" fn atel_get_map_marker_x() -> u32 {
        g::atel_get_map_marker_x(&mem())
    }
    /// `AtelGetMapMarkerY()`.
    AtelGetMapMarkerY @ 0x0087_3af0 => extern "C" fn atel_get_map_marker_y() -> u32 {
        g::atel_get_map_marker_y(&mem())
    }
    /// `AtelClearCameraMatrixFlag()`.
    AtelClearCameraMatrixFlag @ 0x0087_72e0 => extern "C" fn atel_clear_camera_matrix_flag() {
        g::atel_clear_camera_matrix_flag(&mut mem());
    }
    /// `AtelClearForcePlaceId()`.
    AtelClearForcePlaceId @ 0x0086_38b0 => extern "C" fn atel_clear_force_place_id() {
        g::atel_clear_force_place_id(&mut mem());
    }
    /// `AtelSetMenuOtakaraFlag()`.
    AtelSetMenuOtakaraFlag @ 0x0087_3e80 => extern "C" fn atel_set_menu_otakara_flag() -> u32 {
        g::atel_set_menu_otakara_flag(&mut mem())
    }
    /// `AtelClearMenuOtakaraFlag()`.
    AtelClearMenuOtakaraFlag @ 0x0087_2b60 => extern "C" fn atel_clear_menu_otakara_flag() -> u32 {
        g::atel_clear_menu_otakara_flag(&mut mem())
    }
    /// `AtelInitActor()`.
    AtelInitActor @ 0x0086_cae0 => extern "C" fn atel_init_actor() -> u32 {
        g::atel_init_actor(&mut mem())
    }
    /// `AtelSetHalfTurnAngle(deg)`.
    AtelSetHalfTurnAngle @ 0x0087_3e50 => extern "C" fn atel_set_half_turn_angle(deg: f32) {
        g::atel_set_half_turn_angle(&mut mem(), deg.to_bits());
    }
    /// `AtelSetVoicePan(pan)`.
    AtelSetVoicePan @ 0x0087_3d10 => extern "C" fn atel_set_voice_pan(pan: u32) {
        g::atel_set_voice_pan(&mut mem(), pan.to_le_bytes()[0]);
    }
    /// `AtelSetVagStreamVolume(v)`.
    AtelSetVagStreamVolume @ 0x0087_3d20 => extern "C" fn atel_set_vag_stream_volume(v: u32) {
        g::atel_set_vag_stream_volume(&mut mem(), v.to_le_bytes()[0]);
    }
    /// `AtelRegistExpandFTCAdrs(adrs)`.
    AtelRegistExpandFTCAdrs @ 0x0087_3cf0 => extern "C" fn atel_regist_expand_ftc_adrs(adrs: u32) {
        g::atel_regist_expand_ftc_adrs(&mut mem(), adrs);
    }
    /// `AtelSetEventMode(mode)`.
    AtelSetEventMode @ 0x0087_3e40 => extern "C" fn atel_set_event_mode(mode: u32) {
        g::atel_set_event_mode(&mut mem(), mode.to_le_bytes()[0]);
    }
    /// `setMegasAdding(v)`.
    setMegasAdding @ 0x0086_0f20 => extern "C" fn set_megas_adding(v: u32) {
        g::set_megas_adding(&mut mem(), v);
    }
    /// `AtelSetEffectParam(i, v)`.
    AtelSetEffectParam @ 0x0086_fa60 => extern "C" fn atel_set_effect_param(i: u32, v: u32) {
        g::atel_set_effect_param(&mut mem(), i, v);
    }
    /// `AtelSetEffectState(v)`.
    AtelSetEffectState @ 0x0086_fa80 => extern "C" fn atel_set_effect_state(v: u32) {
        g::atel_set_effect_state(&mut mem(), v);
    }
    /// `AtelSetBattleSceneGroup(group)`.
    AtelSetBattleSceneGroup @ 0x0086_f3c0 => extern "C" fn atel_set_battle_scene_group(grp: u32) {
        g::atel_set_battle_scene_group(&mut mem(), grp.to_le_bytes()[0]);
    }
    /// `AtelSetMapFadeType(type)`.
    AtelSetMapFadeType @ 0x0087_00c0 => extern "C" fn atel_set_map_fade_type(t: u32) {
        g::atel_set_map_fade_type(&mut mem(), t.to_le_bytes()[0]);
    }
    /// `AtelSetMapFadeFrames(frames)`.
    AtelSetMapFadeFrames @ 0x0087_00d0 => extern "C" fn atel_set_map_fade_frames(n: u32) {
        g::atel_set_map_fade_frames(&mut mem(), n.to_le_bytes()[0]);
    }
    /// `AtelSetOnMemoryMovie(n)`.
    AtelSetOnMemoryMovie @ 0x0087_3e90 => extern "C" fn atel_set_on_memory_movie(n: u32) {
        g::atel_set_on_memory_movie(&mut mem(), n);
    }
    /// `AtelGetBindPCFromID(index)`.
    AtelGetBindPCFromID @ 0x0086_ae00 => extern "C" fn atel_get_bind_pc_from_id(index: i32) -> u32 {
        g::atel_get_bind_pc_from_id(&mut mem(), index)
    }
    /// `Script_NativeEntry(id)`.
    Script_NativeEntry @ 0x0087_7770 => extern "C" fn script_native_entry(id: u32) -> u32 {
        run::script_native_entry(&mem(), id)
    }
    /// `Script_CallNative(id, worker, args, stack)`.
    Script_CallNative @ 0x0087_77a0 => extern "C" fn script_call_native(id: u32, w: u32, args: u32, s: u32) -> u32 {
        run::script_call_native(&mut mem(), id, w, args, s)
    }
    /// `Script_PollNative(id, worker, args)`.
    Script_PollNative @ 0x0087_7730 => extern "C" fn script_poll_native(id: u32, w: u32, args: u32) -> u32 {
        run::script_poll_native(&mut mem(), id, w, args)
    }
    /// `Script_FinishNative(id, worker, args, stack)`.
    Script_FinishNative @ 0x0087_77f0 => extern "C" fn script_finish_native(id: u32, w: u32, args: u32, s: u32) {
        run::script_finish_native(&mut mem(), id, w, args, s);
    }

    // signals (ffx_script::vm::bridge::signal)
    /// `AtelLinkSigWorkNext(at, node)`.
    AtelLinkSigWorkNext @ 0x0086_dc10 => extern "C" fn atel_link_sig_work_next(at: u32, n: u32) {
        sig::atel_link_sig_work_next(&mut mem(), at, n);
    }
    /// `AtelLinkSigWorkPrev(at, node)`.
    AtelLinkSigWorkPrev @ 0x0086_dc30 => extern "C" fn atel_link_sig_work_prev(at: u32, n: u32) {
        sig::atel_link_sig_work_prev(&mut mem(), at, n);
    }
    /// `AtelLinkSigWorkButtom(head, node)`.
    AtelLinkSigWorkButtom @ 0x0086_dc50 => extern "C" fn atel_link_sig_work_buttom(head: u32, n: u32) {
        sig::atel_link_sig_work_buttom(&mut mem(), head, n);
    }
    /// `AtelRegistSignalMain(request, -1)`.
    AtelRegistSignalMain @ 0x0086_ea00 => extern "C" fn atel_regist_signal_main(r: u32) -> u32 {
        let m = mem();
        let req = sig::Request::read(&m, r);
        sig::atel_regist_signal_main(&mut mem(), req)
    }
    /// `AtelRegistSignal(from, to, kind, level, entry)`.
    AtelRegistSignal @ 0x0086_e990 => extern "C" fn atel_regist_signal(from: u32, to: u32, kind: u32, lvl: u32, entry: u32) -> u32 {
        sig::atel_regist_signal(&mut mem(), sig::Request::new(from, to, kind, lvl, entry))
    }
    /// `AtelRegistSignal2(request)`.
    AtelRegistSignal2 @ 0x0086_e9e0 => extern "C" fn atel_regist_signal2(r: u32) -> u32 {
        sig::atel_regist_signal2(&mut mem(), r)
    }
    /// `AtelRegistSignalWithoutSameLevel2(request)`.
    AtelRegistSignalWithoutSameLevel2 @ 0x0086_ebe0 => extern "C" fn atel_regist_signal_without_same_level2(r: u32) -> u32 {
        let m = mem();
        let req = sig::Request::read(&m, r);
        sig::atel_regist_signal_without_same_level(&mut mem(), req)
    }
    /// `AtelRegistSignalWithoutSameLevel(from, to, kind, level, entry)`.
    AtelRegistSignalWithoutSameLevel @ 0x0086_eba0 => extern "C" fn atel_regist_signal_without_same_level(from: u32, to: u32, kind: u32, lvl: u32, entry: u32) -> u32 {
        sig::atel_regist_signal_without_same_level(&mut mem(), sig::Request::new(from, to, kind, lvl, entry))
    }
    /// `AtelRegistSignalWithoutSameTag2(request)`.
    AtelRegistSignalWithoutSameTag2 @ 0x0086_ec80 => extern "C" fn atel_regist_signal_without_same_tag2(r: u32) -> u32 {
        let m = mem();
        let req = sig::Request::read(&m, r);
        sig::atel_regist_signal_without_same_tag(&mut mem(), req)
    }
    /// `AtelRegistSignalWithoutSameTag(from, to, kind, level, entry)`.
    AtelRegistSignalWithoutSameTag @ 0x0086_ec40 => extern "C" fn atel_regist_signal_without_same_tag(from: u32, to: u32, kind: u32, lvl: u32, entry: u32) -> u32 {
        sig::atel_regist_signal_without_same_tag(&mut mem(), sig::Request::new(from, to, kind, lvl, entry))
    }
    /// `AtelCancelSignals(worker, level, mode)`.
    AtelCancelSignals @ 0x0087_2860 => extern "C" fn atel_cancel_signals(w: u32, lvl: i32, mode: u32) {
        sig::atel_cancel_signals(&mut mem(), w, lvl, mode);
    }
    /// `AtelSignalReply(worker, node)`.
    AtelSignalReply @ 0x0087_1650 => extern "C" fn atel_signal_reply(w: u32, n: u32) {
        sig::atel_signal_reply(&mut mem(), w, n);
    }
    /// `AtelSearchReplySignal(worker, level)`.
    AtelSearchReplySignal @ 0x0086_35c0 => extern "C" fn atel_search_reply_signal(w: u32, l: u32) -> u32 {
        sig::atel_search_reply_signal(&mem(), w, l)
    }
    /// `AtelFreeSignal(worker index, node)`.
    AtelFreeSignal @ 0x0086_54c0 => extern "C" fn atel_free_signal(id: u32, n: u32) {
        sig::atel_free_signal(&mut mem(), id, n);
    }
    /// `AtelIsSignalQueued(worker index, entry)`.
    AtelIsSignalQueued @ 0x0086_3570 => extern "C" fn atel_is_signal_queued(id: u32, entry: u32) -> u32 {
        sig::atel_is_signal_queued(&mut mem(), id, entry)
    }
    /// `AtelChangeBinaryHeader(worker index, block, b, header)`.
    AtelChangeBinaryHeader @ 0x0086_2cc0 => extern "C" fn atel_change_binary_header(id: u32, block: u32, b: u32, hdr: u32) {
        sig::atel_change_binary_header(&mut mem(), id, block, b, hdr);
    }
    /// `AtelChangeLevelHeader(worker, level)`.
    AtelChangeLevelHeader @ 0x0086_2c60 => extern "C" fn atel_change_level_header(w: u32, l: u32) {
        sig::atel_change_level_header(&mut mem(), w, l);
    }
    /// `AtelInitLevelMove(worker, level, n)`.
    AtelInitLevelMove @ 0x0086_3750 => extern "C" fn atel_init_level_move(w: u32, l: u32, n: u32) {
        sig::atel_init_level_move(&mut mem(), w, l, n);
    }
    /// `AtelResetLevel(worker, n)`.
    AtelResetLevel @ 0x0086_39e0 => extern "C" fn atel_reset_level(w: u32, n: u32) {
        sig::atel_reset_level(&mut mem(), w, n);
    }
    /// `AtelStartSignal(worker, node)`.
    AtelStartSignal @ 0x0086_90a0 => extern "C" fn atel_start_signal(w: u32, n: u32) {
        sig::atel_start_signal(&mut mem(), w, n);
    }
    /// `AtelDecodeSignal(worker index, level)`.
    AtelDecodeSignal @ 0x0086_53f0 => extern "C" fn atel_decode_signal(id: u32, lvl: i32) {
        sig::atel_decode_signal(&mut mem(), id, lvl);
    }
    /// `AtelExecReturn(worker index, level, reset)`.
    AtelExecReturn @ 0x0086_91d0 => extern "C" fn atel_exec_return(id: u32, lvl: u32, reset: u32) {
        sig::atel_exec_return(&mut mem(), id, lvl, reset);
    }
    /// `AtelFindActorByChar(chr)`.
    AtelFindActorByChar @ 0x0086_a470 => extern "C" fn atel_find_actor_by_char(chr: u32) -> u32 {
        sig::atel_find_actor_by_char(&mut mem(), chr)
    }
    /// `AtelGetPartyWorker(slot)`.
    AtelGetPartyWorker @ 0x0086_a5e0 => extern "C" fn atel_get_party_worker(slot: u32) -> u32 {
        sig::atel_get_party_worker(&mut mem(), slot)
    }
    /// `Script_OpReq(op, level, worker, stack)`.
    Script_OpReq @ 0x0086_7230 => extern "C" fn script_op_req(op: u32, _l: u32, w: u32, s: u32) {
        sig::script_op_req(&mut mem(), op.to_le_bytes()[0], w, s);
    }
    /// `Script_OpReqSw(op, worker, level, stack)`.
    Script_OpReqSw @ 0x0086_7570 => extern "C" fn script_op_req_sw(op: u32, w: u32, l: u32, s: u32) -> u32 {
        sig::script_op_req_wait(&mut mem(), op.to_le_bytes()[0], w, l, s)
    }
    /// `Script_OpReqEw(op, worker, level, stack)`.
    Script_OpReqEw @ 0x0086_73d0 => extern "C" fn script_op_req_ew(op: u32, w: u32, l: u32, s: u32) -> u32 {
        sig::script_op_req_wait(&mut mem(), op.to_le_bytes()[0], w, l, s)
    }
    /// `Script_Op79(worker, var, index, value)`.
    Script_Op79 @ 0x0087_0d30 => extern "C" fn script_op79(w: u32, var: u32, index: u32, value: u32) {
        sig::script_op79(&mut mem(), w, var, index, value);
    }
    /// `Script_Op7a(worker, key, value)`.
    Script_Op7a @ 0x0086_fab0 => extern "C" fn script_op7a(w: u32, key: u32, value: u32) -> u32 {
        sig::script_op7a(&mut mem(), w, key, value)
    }

    // the native functions (ffx_script::vm::bridge::native): (worker, args, stack);
    // the nine under five bytes (Native_WaitForever, Native_ControlEnable, ...)
    // run in Rust only through Script_Run's dispatch
    /// `Native_MsgSetStyle(worker, args, stack)`.
    Native_MsgSetStyle @ 0x0085_8360 => extern "C" fn n_858360(w: u32, a: u32, s: u32) -> u32 {
        native::native_msg_set_style(&mut mem(), w, a, s)
    }
    /// `Native_MsgSetFlags(worker, args, stack)`.
    Native_MsgSetFlags @ 0x0085_84d0 => extern "C" fn n_8584d0(w: u32, a: u32, s: u32) -> u32 {
        native::native_msg_set_flags(&mut mem(), w, a, s)
    }
    /// `Native_MsgGetState(worker, args, stack)`.
    Native_MsgGetState @ 0x0085_fa80 => extern "C" fn n_85fa80(w: u32, a: u32, s: u32) -> u32 {
        native::native_msg_get_state(&mut mem(), w, a, s)
    }
    /// `Native_MsgIsReady(worker, args, stack)`.
    Native_MsgIsReady @ 0x0085_de10 => extern "C" fn n_85de10(w: u32, a: u32, s: u32) -> u32 {
        native::native_msg_is_ready(&mut mem(), w, a, s)
    }
    /// `Native_ChrSetMotionBlend(worker, args, stack)`.
    Native_ChrSetMotionBlend @ 0x00a7_9f40 => extern "C" fn n_a79f40(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_set_motion_blend(&mut mem(), w, a, s)
    }
    /// `Native_ChrSetNextMotionBlend(worker, args, stack)`.
    Native_ChrSetNextMotionBlend @ 0x00a7_8d70 => extern "C" fn n_a78d70(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_set_next_motion_blend(&mut mem(), w, a, s)
    }
    /// `Native_ChrSetBodyHit(worker, args, stack)`.
    Native_ChrSetBodyHit @ 0x00a7_8da0 => extern "C" fn n_a78da0(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_set_body_hit(&mut mem(), w, a, s)
    }
    /// `Native_ChrSetGroundFollow(worker, args, stack)`.
    Native_ChrSetGroundFollow @ 0x00a7_8cb0 => extern "C" fn n_a78cb0(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_set_ground_follow(&mut mem(), w, a, s)
    }
    /// `Native_ChrSetGroundMode(worker, args, stack)`.
    Native_ChrSetGroundMode @ 0x00a7_9130 => extern "C" fn n_a79130(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_set_ground_mode(&mut mem(), w, a, s)
    }
    /// `Native_ChrSetMotionSpeed(worker, args, stack)`.
    Native_ChrSetMotionSpeed @ 0x00a7_9510 => extern "C" fn n_a79510(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_set_motion_speed(&mut mem(), w, a, s)
    }
    /// `Native_ChrSetDistanceCull(worker, args, stack)`.
    Native_ChrSetDistanceCull @ 0x00a7_9af0 => extern "C" fn n_a79af0(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_set_distance_cull(&mut mem(), w, a, s)
    }
    /// `Native_ChrSetShadowMode(worker, args, stack)`.
    Native_ChrSetShadowMode @ 0x00a7_9a40 => extern "C" fn n_a79a40(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_set_shadow_mode(&mut mem(), w, a, s)
    }
    /// `Native_ChrSetAutoMotion(worker, args, stack)`.
    Native_ChrSetAutoMotion @ 0x00a7_9670 => extern "C" fn n_a79670(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_set_auto_motion(&mut mem(), w, a, s)
    }
    /// `Native_ChrReleaseMotion(worker, args, stack)`.
    Native_ChrReleaseMotion @ 0x00a7_8940 => extern "C" fn n_a78940(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_release_motion(&mut mem(), w, a, s)
    }
    /// `Native_ChrReleaseMotion2(worker, args, stack)`.
    Native_ChrReleaseMotion2 @ 0x00a7_94b0 => extern "C" fn n_a794b0(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_release_motion2(&mut mem(), w, a, s)
    }
    /// `Native_ChrGetMotionFrame(worker, args, stack)`.
    Native_ChrGetMotionFrame @ 0x00a7_8750 => extern "C" fn n_a78750(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_get_motion_frame(&mut mem(), w, a, s)
    }
    /// `Native_ChrGetMotion(worker, args, stack)`.
    Native_ChrGetMotion @ 0x00a7_8dd0 => extern "C" fn n_a78dd0(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_get_motion(&mut mem(), w, a, s)
    }
    /// `Native_ChrGetMotionFrameOf(worker, args, stack)`.
    Native_ChrGetMotionFrameOf @ 0x00a7_9dd0 => extern "C" fn n_a79dd0(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_get_motion_frame_of(&mut mem(), w, a, s)
    }
    /// `CT_507E_ret(worker, args, stack)`.
    CT_507E_ret @ 0x00a7_a210 => extern "C" fn n_a7a210(w: u32, a: u32, s: u32) -> u32 {
        native::ct_507e_ret(&mut mem(), w, a, s)
    }
    /// `Native_ChrSetBoneUpdate(worker, args, stack)`.
    Native_ChrSetBoneUpdate @ 0x00a7_8af0 => extern "C" fn n_a78af0(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_set_bone_update(&mut mem(), w, a, s)
    }
    /// `Native_ChrFadeAlpha(worker, args, stack)`.
    Native_ChrFadeAlpha @ 0x00a7_9ea0 => extern "C" fn n_a79ea0(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_fade_alpha(&mut mem(), w, a, s)
    }
    /// `Native_ChrSetOffset(worker, args, stack)`.
    Native_ChrSetOffset @ 0x00a7_90b0 => extern "C" fn n_a790b0(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_set_offset(&mut mem(), w, a, s)
    }
    /// `Native_ChrSetPushWeight(worker, args, stack)`.
    Native_ChrSetPushWeight @ 0x00a7_8b30 => extern "C" fn n_a78b30(w: u32, a: u32, s: u32) -> f64 {
        native::native_chr_set_push_weight(&mut mem(), w, a, s)
    }
    /// `Native_ChrWaitMotion_5003(worker, args, stack)`.
    Native_ChrWaitMotion_5003 @ 0x00a7_87d0 => extern "C" fn n_a787d0(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_wait_motion_5003(&mut mem(), w, a, s)
    }
    /// `Native_ChrWaitMotion_Poll_5003(worker, args, stack)`.
    Native_ChrWaitMotion_Poll_5003 @ 0x00a7_88a0 => extern "C" fn n_a788a0(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_wait_motion_poll_5003(&mut mem(), w, a, s)
    }
    /// `Native_ChrWaitMotion_Poll_501e(worker, args, stack)`.
    Native_ChrWaitMotion_Poll_501e @ 0x00a7_93c0 => extern "C" fn n_a793c0(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_wait_motion_poll_501e(&mut mem(), w, a, s)
    }
    /// `Native_GetWorkerId(worker, args, stack)`.
    Native_GetWorkerId @ 0x0085_ba50 => extern "C" fn n_85ba50(w: u32, a: u32, s: u32) -> u32 {
        native::native_get_worker_id(&mut mem(), w, a, s)
    }
    /// `Native_GetWorkerX(worker, args, stack)`.
    Native_GetWorkerX @ 0x0085_bb80 => extern "C" fn n_85bb80(w: u32, a: u32, s: u32) -> f64 {
        native::native_get_worker_x(&mut mem(), w, a, s)
    }
    /// `Native_GetWorkerY(worker, args, stack)`.
    Native_GetWorkerY @ 0x0085_bd40 => extern "C" fn n_85bd40(w: u32, a: u32, s: u32) -> f64 {
        native::native_get_worker_y(&mut mem(), w, a, s)
    }
    /// `Native_GetWorkerZ(worker, args, stack)`.
    Native_GetWorkerZ @ 0x0085_bf10 => extern "C" fn n_85bf10(w: u32, a: u32, s: u32) -> f64 {
        native::native_get_worker_z(&mut mem(), w, a, s)
    }
    /// `Native_TurnSetYaw(worker, args, stack)`.
    Native_TurnSetYaw @ 0x0085_9640 => extern "C" fn n_859640(w: u32, a: u32, s: u32) -> f64 {
        native::native_turn_set_yaw(&mut mem(), w, a, s)
    }
    /// `Native_TurnSetPitch(worker, args, stack)`.
    Native_TurnSetPitch @ 0x0085_9880 => extern "C" fn n_859880(w: u32, a: u32, s: u32) -> f64 {
        native::native_turn_set_pitch(&mut mem(), w, a, s)
    }
    /// `Native_TurnSetFrames(worker, args, stack)`.
    Native_TurnSetFrames @ 0x0085_60d0 => extern "C" fn n_8560d0(w: u32, a: u32, s: u32) -> f64 {
        native::native_turn_set_frames(&mut mem(), w, a, s)
    }
    /// `Native_MoveSetFrames(worker, args, stack)`.
    Native_MoveSetFrames @ 0x0085_5f10 => extern "C" fn n_855f10(w: u32, a: u32, s: u32) -> f64 {
        native::native_move_set_frames(&mut mem(), w, a, s)
    }
    /// `Native_TurnGetField38(worker, args, stack)`.
    Native_TurnGetField38 @ 0x0085_c590 => extern "C" fn n_85c590(w: u32, a: u32, s: u32) -> f64 {
        native::native_turn_get_field38(&mut mem(), w, a, s)
    }
    /// `Native_MoveWait(worker, args, stack)`.
    Native_MoveWait @ 0x0085_79a0 => extern "C" fn n_8579a0(w: u32, a: u32, s: u32) -> u32 {
        native::native_move_wait(&mut mem(), w, a, s)
    }
    /// `Native_TurnWait(worker, args, stack)`.
    Native_TurnWait @ 0x0085_7b30 => extern "C" fn n_857b30(w: u32, a: u32, s: u32) -> u32 {
        native::native_turn_wait(&mut mem(), w, a, s)
    }
    /// `Native_GetTurnKind(worker, args, stack)`.
    Native_GetTurnKind @ 0x0085_a670 => extern "C" fn n_85a670(w: u32, a: u32, s: u32) -> u32 {
        native::native_get_turn_kind(&mut mem(), w, a, s)
    }
    /// `Native_GetTurnMotion(worker, args, stack)`.
    Native_GetTurnMotion @ 0x0085_abb0 => extern "C" fn n_85abb0(w: u32, a: u32, s: u32) -> u32 {
        native::native_get_turn_motion(&mut mem(), w, a, s)
    }
    /// `Native_SetTurnMotions(worker, args, stack)`.
    Native_SetTurnMotions @ 0x0085_a4e0 => extern "C" fn n_85a4e0(w: u32, a: u32, s: u32) -> u32 {
        native::native_set_turn_motions(&mut mem(), w, a, s)
    }
    /// `Native_EnableTrigger(worker, args, stack)`.
    Native_EnableTrigger @ 0x0085_c060 => extern "C" fn n_85c060(w: u32, a: u32, s: u32) -> u32 {
        native::native_enable_trigger(&mut mem(), w, a, s)
    }
    /// `Native_DisableTrigger(worker, args, stack)`.
    Native_DisableTrigger @ 0x0085_c280 => extern "C" fn n_85c280(w: u32, a: u32, s: u32) -> u32 {
        native::native_disable_trigger(&mut mem(), w, a, s)
    }
    /// `Native_TriggerSetActive(worker, args, stack)`.
    Native_TriggerSetActive @ 0x0085_fbf0 => extern "C" fn n_85fbf0(w: u32, a: u32, s: u32) -> u32 {
        native::native_trigger_set_active(&mut mem(), w, a, s)
    }
    /// `Native_TriggerSetActive2(worker, args, stack)`.
    Native_TriggerSetActive2 @ 0x0086_00c0 => extern "C" fn n_8600c0(w: u32, a: u32, s: u32) -> u32 {
        native::native_trigger_set_active(&mut mem(), w, a, s)
    }
    /// `Native_WorkerSetFlag0200(worker, args, stack)`.
    Native_WorkerSetFlag0200 @ 0x0085_b9b0 => extern "C" fn n_85b9b0(w: u32, a: u32, s: u32) -> u32 {
        native::native_worker_set_flag0200(&mut mem(), w, a, s)
    }
    /// `Native_MinimapSetMarker(worker, args, stack)`.
    Native_MinimapSetMarker @ 0x0085_83b0 => extern "C" fn n_8583b0(w: u32, a: u32, s: u32) -> u32 {
        native::native_minimap_set_marker(&mut mem(), w, a, s)
    }
    /// `Native_Random(worker, args, stack)`.
    Native_Random @ 0x0085_7400 => extern "C" fn n_857400(w: u32, a: u32, s: u32) -> u32 {
        native::native_random(&mut mem(), w, a, s)
    }
    /// `Native_WaitFrames_01d1(worker, args, stack)`.
    Native_WaitFrames_01d1 @ 0x0085_fd20 => extern "C" fn n_85fd20(w: u32, a: u32, s: u32) -> u32 {
        native::native_wait_frames_01d1(&mut mem(), w, a, s)
    }
    /// `Native_WaitFrames_Poll_01d1(worker, args, stack)`.
    Native_WaitFrames_Poll_01d1 @ 0x0085_ff70 => extern "C" fn n_85ff70(w: u32, a: u32, s: u32) -> u32 {
        native::native_wait_frames_poll_01d1(&mut mem(), w, a, s)
    }
    /// `Native_SetMapFadeFrames(worker, args, stack)`.
    Native_SetMapFadeFrames @ 0x0085_96b0 => extern "C" fn n_8596b0(w: u32, a: u32, s: u32) -> u32 {
        native::native_set_map_fade_frames(&mut mem(), w, a, s)
    }
    /// `Native_SetMapFadeType(worker, args, stack)`.
    Native_SetMapFadeType @ 0x0085_9590 => extern "C" fn n_859590(w: u32, a: u32, s: u32) -> u32 {
        native::native_set_map_fade_type(&mut mem(), w, a, s)
    }
    /// `Native_CharRestoreHp(worker, args, stack)`.
    Native_CharRestoreHp @ 0x0085_c680 => extern "C" fn n_85c680(w: u32, a: u32, s: u32) -> u32 {
        native::native_char_restore_hp(&mut mem(), w, a, s)
    }
    /// `Native_CharRestoreMp(worker, args, stack)`.
    Native_CharRestoreMp @ 0x0085_c860 => extern "C" fn n_85c860(w: u32, a: u32, s: u32) -> u32 {
        native::native_char_restore_mp(&mut mem(), w, a, s)
    }
    /// `CT_0241_ret(worker, args, stack)`.
    CT_0241_ret @ 0x0085_6ad0 => extern "C" fn n_856ad0(w: u32, a: u32, s: u32) -> u32 {
        native::ct_0241_ret(&mut mem(), w, a, s)
    }
    /// `Native_FpRun(worker, args, stack)`.
    Native_FpRun @ 0x0091_c670 => extern "C" fn n_91c670(w: u32, a: u32, s: u32) -> u32 {
        native::native_fp_run(&mut mem(), w, a, s)
    }
    /// `Native_FpShow(worker, args, stack)`.
    Native_FpShow @ 0x0091_c740 => extern "C" fn n_91c740(w: u32, a: u32, s: u32) -> u32 {
        native::native_fp_show(&mut mem(), w, a, s)
    }
    /// `Native_FpRunMode2(worker, args, stack)`.
    Native_FpRunMode2 @ 0x0091_c7a0 => extern "C" fn n_91c7a0(w: u32, a: u32, s: u32) -> u32 {
        native::native_fp_run_mode2(&mut mem(), w, a, s)
    }
    /// `Native_FieldSetMaskBit(worker, args, stack)`.
    Native_FieldSetMaskBit @ 0x0091_c530 => extern "C" fn n_91c530(w: u32, a: u32, s: u32) -> u32 {
        native::native_field_set_mask_bit(&mut mem(), w, a, s)
    }
}
