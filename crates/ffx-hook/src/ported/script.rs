//! `ffx-script`: the Atel VM's ctrl works, workers, stacks and variables.

use ffx_bridge::{Mem, RawMem};
use ffx_script::vm::bridge as atel;
use ffx_script::vm::bridge::globals as g;

use super::{mem, ported};

/// The game as the VM bridge's host: its memory, and plain calls for the
/// callbacks a ctrl work holds.
struct Game(RawMem);

impl Mem for Game {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.0.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        self.0.write_bytes(addr, data);
    }
    fn image_base(&self) -> u32 {
        self.0.image_base()
    }
}

impl atel::Host for Game {
    fn call(&mut self, addr: u32, args: &[u32]) -> u32 {
        // SAFETY: the address is a cdecl function the game stored for this
        // call, taking these arguments.
        unsafe {
            match *args {
                [] => {
                    let f: unsafe extern "C" fn() -> u32 = std::mem::transmute(addr as usize);
                    f()
                }
                [x] => {
                    let f: unsafe extern "C" fn(u32) -> u32 = std::mem::transmute(addr as usize);
                    f(x)
                }
                [x, y] => {
                    let f: unsafe extern "C" fn(u32, u32) -> u32 =
                        std::mem::transmute(addr as usize);
                    f(x, y)
                }
                _ => unreachable!("the VM's callbacks take at most two arguments"),
            }
        }
    }
}

fn game() -> Game {
    Game(mem())
}

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
    /// `AtelFetchStackInteger(worker, pop, stack)`.
    AtelFetchStackInteger @ 0x0086_df40 => extern "C" fn atel_fetch_stack_integer(_w: u32, pop: u32, s: u32) -> u32 {
        atel::atel_fetch_stack_integer(&mut mem(), pop, s)
    }
    /// `AtelPopStackFloat(worker, stack)`.
    AtelPopStackFloat @ 0x0086_de50 => extern "C" fn atel_pop_stack_float(_w: u32, s: u32) -> f32 {
        f32::from_bits(atel::atel_pop_stack_float(&mut mem(), s))
    }
    /// `AtelGetRealVarAdrs(worker, word)`.
    AtelGetRealVarAdrs @ 0x0086_c2e0 => extern "C" fn atel_get_real_var_adrs(w: u32, word: u32) -> u32 {
        atel::atel_get_real_var_adrs(&mut game(), w, word)
    }
    /// `AtelGetVarAdrs(worker, v)`.
    AtelGetVarAdrs @ 0x0086_c7d0 => extern "C" fn atel_get_var_adrs(w: u32, v: u32) -> u32 {
        atel::atel_get_var_adrs(&mut game(), w, v)
    }
    /// `AtelGetValueAdrs(worker, v, index)`.
    AtelGetValueAdrs @ 0x0086_c570 => extern "C" fn atel_get_value_adrs(w: u32, v: u32, index: i32) -> u32 {
        atel::atel_get_value_adrs(&mut game(), w, v, index)
    }
    /// `AtelGetValueInteger(worker, v, index)`.
    AtelGetValueInteger @ 0x0086_c730 => extern "C" fn atel_get_value_integer(w: u32, v: u32, index: i32) -> u32 {
        atel::atel_get_value_integer(&mut game(), w, v, index)
    }
    /// `AtelGetValueFloat(worker, v, index)`.
    AtelGetValueFloat @ 0x0086_c610 => extern "C" fn atel_get_value_float(w: u32, v: u32, index: i32) -> f32 {
        f32::from_bits(atel::atel_get_value_float(&mut game(), w, v, index))
    }
    /// `AtelGetValueIncValue(type)`.
    AtelGetValueIncValue @ 0x0086_c6e0 => extern "C" fn atel_get_value_inc_value(kind: u32) -> u32 {
        atel::atel_get_value_inc_value(kind)
    }
    /// `AtelSetValueInteger(worker, v, index, value, clamp)`.
    AtelSetValueInteger @ 0x0087_1400 => extern "C" fn atel_set_value_integer(w: u32, v: u32, index: i32, value: i32, clamp: u32) {
        atel::atel_set_value_integer(&mut game(), w, v, index, value, clamp);
    }
    /// `AtelSetValueFloat(worker, v, index, value, clamp)`, the float as its bits.
    AtelSetValueFloat @ 0x0087_12e0 => extern "C" fn atel_set_value_float(w: u32, v: u32, index: i32, bits: u32, clamp: u32) {
        atel::atel_set_value_float(&mut game(), w, v, index, bits, clamp);
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
        atel::script_push_var(&mut game(), w, s, v, index);
    }
    /// `Script_PushVarElem(worker, stack, v)`.
    Script_PushVarElem @ 0x0086_e100 => extern "C" fn script_push_var_elem(w: u32, s: u32, v: u32) {
        atel::script_push_var_elem(&mut game(), w, s, v);
    }
    /// `Script_StoreVar(worker, stack, v, mode)`.
    Script_StoreVar @ 0x0086_dfb0 => extern "C" fn script_store_var(w: u32, s: u32, v: u32, mode: u32) -> u32 {
        atel::script_store_var(&mut game(), w, s, v, mode)
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
}
