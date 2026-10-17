//! `ffx-script`: the Atel VM's ctrl works, workers, stacks and variables.

use ffx_script::vm::bridge as atel;
use ffx_script::vm::bridge::globals as g;
use ffx_script::vm::bridge::helpers as hp;
use ffx_script::vm::bridge::native;
use ffx_script::vm::bridge::native_btl;
use ffx_script::vm::bridge::native_cam;
use ffx_script::vm::bridge::native_chr;
use ffx_script::vm::bridge::native_map;
use ffx_script::vm::bridge::native_move;
use ffx_script::vm::bridge::native_msg;
use ffx_script::vm::bridge::run;
use ffx_script::vm::bridge::signal as sig;

use super::{mem, ported};

ported! {
    /// `AtelClearEffectFlag_00863860`.
    AtelClearEffectFlag_00863860 @ 0x0086_3860 => extern "C" fn h_863860(n: u32) -> u32 {
        hp::atel_clear_effect_flag(&mut mem(), n)
    }
    /// `Atel_SetEffectFlag2`.
    Atel_SetEffectFlag2 @ 0x0087_02f0 => extern "C" fn h_8702f0(n: u32) -> u32 {
        hp::atel_set_effect_flag2(&mut mem(), n)
    }
    /// `Atel_ClearEffectFlag2`.
    Atel_ClearEffectFlag2 @ 0x0086_3af0 => extern "C" fn h_863af0(n: u32) -> u32 {
        hp::atel_clear_effect_flag2(&mut mem(), n)
    }
    /// `AtelSetLoveParamDisable`.
    AtelSetLoveParamDisable @ 0x0087_0250 => extern "C" fn h_870250(i: u32, on: u32) -> u32 {
        hp::atel_set_love_param_disable(&mut mem(), i, on)
    }
    /// `AtelSetGuideMapDrawBlizFlag`.
    AtelSetGuideMapDrawBlizFlag @ 0x0087_0160 => extern "C" fn h_870160(on: u32) -> u32 {
        hp::atel_set_guide_map_draw_bliz_flag(&mut mem(), on)
    }
    /// `AtelSetGuideMapDrawFlag`.
    AtelSetGuideMapDrawFlag @ 0x0087_0190 => extern "C" fn h_870190(on: u32) -> u32 {
        hp::atel_set_guide_map_draw_flag(&mut mem(), on)
    }
    /// `AtelSetGuideMapForceDrawFlag`.
    AtelSetGuideMapForceDrawFlag @ 0x0087_01c0 => extern "C" fn h_8701c0(on: u32) -> u32 {
        hp::atel_set_guide_map_force_draw_flag(&mut mem(), on)
    }
    /// `Atel_SetCtrlFlag1325db0`.
    Atel_SetCtrlFlag1325db0 @ 0x0086_f3d0 => extern "C" fn h_86f3d0(on: u32) -> u32 {
        hp::atel_set_ctrl_flag_1325db0(&mut mem(), on)
    }
    /// `Atel_GetBattleMapId`.
    Atel_GetBattleMapId @ 0x0086_2d50 => extern "C" fn h_862d50(map: u32) -> u32 {
        hp::atel_get_battle_map_id(&mem(), map)
    }
    /// `Atel_SetWorkerFlag36_8`.
    Atel_SetWorkerFlag36_8 @ 0x0087_0fe0 => extern "C" fn h_870fe0(w: u32, on: u32) -> u32 {
        hp::atel_set_worker_flag36_8(&mut mem(), w, on)
    }
    /// `AtelGetBinHdrExpandSpFlags`.
    AtelGetBinHdrExpandSpFlags @ 0x0086_adc0 => extern "C" fn h_86adc0(b: u32) -> u32 {
        hp::atel_get_bin_hdr_expand_sp_flags(&mem(), b)
    }
    /// `Atel_GetBinHdrExpand24`.
    Atel_GetBinHdrExpand24 @ 0x0086_acc0 => extern "C" fn h_86acc0(b: u32) -> u32 {
        hp::atel_get_bin_hdr_expand_24(&mem(), b)
    }
    /// `Atel_GetBinHdrExpandName`.
    Atel_GetBinHdrExpandName @ 0x0086_aa70 => extern "C" fn h_86aa70(b: u32) -> u32 {
        hp::atel_get_bin_hdr_expand_name(&mem(), b)
    }
    /// `AtelGetBindpList`.
    AtelGetBindpList @ 0x0086_ae20 => extern "C" fn h_86ae20() -> u32 {
        hp::atel_get_bindp_list(&mem())
    }
    /// `Atel_GetBinHdrTable18A`.
    Atel_GetBinHdrTable18A @ 0x0086_aaf0 => extern "C" fn h_86aaf0(b: u32, i: u32) -> u32 {
        hp::atel_get_bin_hdr_table18_a(&mem(), b, i)
    }
    /// `Atel_GetBinHdrTable18B`.
    Atel_GetBinHdrTable18B @ 0x0086_aac0 => extern "C" fn h_86aac0(b: u32, i: u32) -> u32 {
        hp::atel_get_bin_hdr_table18_b(&mem(), b, i)
    }
    /// `AtelGetBinHdrExpandSoundEvent`.
    AtelGetBinHdrExpandSoundEvent @ 0x0086_ad20 => extern "C" fn h_86ad20(b: u32) -> u32 {
        hp::atel_get_bin_hdr_expand_sound_event(&mem(), b)
    }
    /// `Atel_GetBlockString`.
    Atel_GetBlockString @ 0x0088_58c0 => extern "C" fn h_8858c0(c: u32, i: u32) -> u32 {
        hp::atel_get_block_string(&mem(), c, i)
    }
    /// `Atel_GetSprite`.
    Atel_GetSprite @ 0x0087_47b0 => extern "C" fn h_8747b0(i: u32) -> u32 {
        hp::atel_get_sprite(&mem(), i)
    }
    /// `Atel_ClearSprite`.
    Atel_ClearSprite @ 0x0087_47f0 => extern "C" fn h_8747f0(i: u32) -> u32 {
        hp::atel_clear_sprite(&mut mem(), i)
    }
    /// `Atel_SetSpriteFlag`.
    Atel_SetSpriteFlag @ 0x0087_4970 => extern "C" fn h_874970(i: u32, on: u32) -> u32 {
        hp::atel_set_sprite_flag(&mut mem(), i, on)
    }
    /// `Atel_SetSprite13`.
    Atel_SetSprite13 @ 0x0087_49b0 => extern "C" fn h_8749b0(i: u32, v: u32) -> u32 {
        hp::atel_set_sprite_13(&mut mem(), i, v)
    }
    /// `Atel_SetSpritePos`.
    Atel_SetSpritePos @ 0x0087_4b30 => extern "C" fn h_874b30(i: u32, x: u32, y: u32) -> u32 {
        hp::atel_set_sprite_pos(&mut mem(), i, x, y)
    }
    /// `AtelClearSnd3dWork`.
    AtelClearSnd3dWork @ 0x0087_72f0 => extern "C" fn h_8772f0(i: u32) -> u32 {
        hp::atel_clear_snd3d_work(&mut mem(), i)
    }
    /// `Atel_ClearRecords1327f50`.
    Atel_ClearRecords1327f50 @ 0x0087_4fb0 => extern "C" fn h_874fb0() -> u32 {
        hp::atel_clear_records_1327f50(&mut mem())
    }
    /// `Atel_Strlen`.
    Atel_Strlen @ 0x0087_19c0 => extern "C" fn h_8719c0(s: u32) -> u32 {
        hp::atel_strlen(&mem(), s)
    }
    /// `Atel_StrTrimLeading`.
    Atel_StrTrimLeading @ 0x0087_7610 => extern "C" fn h_877610(s: u32, n: u32, ch: u32) -> u32 {
        hp::atel_str_trim_leading(&mut mem(), s, n, ch)
    }
    /// `Atel_SetLevelF74`.
    Atel_SetLevelF74 @ 0x0087_0d10 => extern "C" fn h_870d10(w: u32, i: u32, v: u32) -> u32 {
        hp::atel_set_level_f74(&mut mem(), w, i, v)
    }
    /// `Atel_SetCurLevel5ec`.
    Atel_SetCurLevel5ec @ 0x0086_f040 => extern "C" fn h_86f040(w: u32, v: u32) -> u32 {
        hp::atel_set_cur_level_f32(&mut mem(), w, 0x5ec, v)
    }
    /// `Atel_SetCurLevel5c8`.
    Atel_SetCurLevel5c8 @ 0x0087_15c0 => extern "C" fn h_8715c0(w: u32, v: u32) -> u32 {
        hp::atel_set_cur_level_f32(&mut mem(), w, 0x5c8, v)
    }
    /// `Atel_SetCurLevel5c4`.
    Atel_SetCurLevel5c4 @ 0x0087_1610 => extern "C" fn h_871610(w: u32, v: u32) -> u32 {
        hp::atel_set_cur_level_f32(&mut mem(), w, 0x5c4, v)
    }
    /// `Atel_SetCurLevel5c0`.
    Atel_SetCurLevel5c0 @ 0x0087_1630 => extern "C" fn h_871630(w: u32, v: u32) -> u32 {
        hp::atel_set_cur_level_f32(&mut mem(), w, 0x5c0, v)
    }
    /// `Atel_SetLevel5f8`.
    Atel_SetLevel5f8 @ 0x0087_1500 => extern "C" fn h_871500(w: u32, i: u32, v: u32) -> f64 {
        hp::atel_set_level_f32(&mut mem(), w, i, 0x5f8, v)
    }
    /// `Atel_SetLevel5f4`.
    Atel_SetLevel5f4 @ 0x0087_1520 => extern "C" fn h_871520(w: u32, i: u32, v: u32) -> f64 {
        hp::atel_set_level_f32(&mut mem(), w, i, 0x5f4, v)
    }
    /// `Atel_SetTurn880`.
    Atel_SetTurn880 @ 0x0087_1540 => extern "C" fn h_871540(w: u32, i: u32, v: u32) -> f64 {
        hp::atel_set_turn_f32(&mut mem(), w, i, 0x880, v)
    }
    /// `Atel_SetTurn87c`.
    Atel_SetTurn87c @ 0x0087_1560 => extern "C" fn h_871560(w: u32, i: u32, v: u32) -> f64 {
        hp::atel_set_turn_f32(&mut mem(), w, i, 0x87c, v)
    }
    /// `Atel_SetTurn874`.
    Atel_SetTurn874 @ 0x0087_1580 => extern "C" fn h_871580(w: u32, i: u32, v: u32) -> f64 {
        hp::atel_set_turn_f32(&mut mem(), w, i, 0x874, v)
    }
    /// `Atel_SetTurn878`.
    Atel_SetTurn878 @ 0x0087_15a0 => extern "C" fn h_8715a0(w: u32, i: u32, v: u32) -> f64 {
        hp::atel_set_turn_f32(&mut mem(), w, i, 0x878, v)
    }
    /// `Atel_SetAllLevels5f0`.
    Atel_SetAllLevels5f0 @ 0x0086_f0d0 => extern "C" fn h_86f0d0(w: u32, v: u32) -> u32 {
        hp::atel_set_all_f32(&mut mem(), w, 0x5f0, 0x4c, v)
    }
    /// `Atel_SetAllLevels5f8`.
    Atel_SetAllLevels5f8 @ 0x0086_f170 => extern "C" fn h_86f170(w: u32, v: u32) -> u32 {
        hp::atel_set_all_f32(&mut mem(), w, 0x5f8, 0x4c, v)
    }
    /// `Atel_SetAllLevels5f4`.
    Atel_SetAllLevels5f4 @ 0x0086_f1c0 => extern "C" fn h_86f1c0(w: u32, v: u32) -> u32 {
        hp::atel_set_all_f32(&mut mem(), w, 0x5f4, 0x4c, v)
    }
    /// `Atel_SetAllLevels5c8`.
    Atel_SetAllLevels5c8 @ 0x0086_f350 => extern "C" fn h_86f350(w: u32, v: u32) -> u32 {
        hp::atel_set_all_f32(&mut mem(), w, 0x5c8, 0x4c, v)
    }
    /// `Atel_SetAllTurns880`.
    Atel_SetAllTurns880 @ 0x0086_f210 => extern "C" fn h_86f210(w: u32, v: u32) -> u32 {
        hp::atel_set_all_f32(&mut mem(), w, 0x880, 0x44, v)
    }
    /// `Atel_SetAllTurns87c`.
    Atel_SetAllTurns87c @ 0x0086_f260 => extern "C" fn h_86f260(w: u32, v: u32) -> u32 {
        hp::atel_set_all_f32(&mut mem(), w, 0x87c, 0x44, v)
    }
    /// `Atel_SetAllTurns874`.
    Atel_SetAllTurns874 @ 0x0086_f2b0 => extern "C" fn h_86f2b0(w: u32, v: u32) -> u32 {
        hp::atel_set_all_f32(&mut mem(), w, 0x874, 0x44, v)
    }
    /// `Atel_SetAllTurns878`.
    Atel_SetAllTurns878 @ 0x0086_f300 => extern "C" fn h_86f300(w: u32, v: u32) -> f64 {
        {
            hp::atel_set_all_f32(&mut mem(), w, 0x878, 0x44, v);
            ffx_bridge::x87::load(v)
        }
    }
    /// `Script_StopTurn`.
    Script_StopTurn @ 0x0086_3fe0 => extern "C" fn h_863fe0(w: u32, l: u32) -> u32 {
        hp::script_stop_turn(&mut mem(), w, l)
    }
    /// `Atel_SetLevelVectors`.
    Atel_SetLevelVectors @ 0x0086_f780 => extern "C" fn h_86f780(w: u32, a: u32, b: u32, c: u32) -> u32 {
        hp::atel_set_level_vectors(&mut mem(), w, a, b, c)
    }
    /// `Atel_SetWorkerFlag200`.
    Atel_SetWorkerFlag200 @ 0x0087_12a0 => extern "C" fn h_8712a0(w: u32, on: u32) -> u32 {
        hp::atel_set_worker_flag_200(&mut mem(), w, on)
    }
    /// `Atel_BinTableCount`.
    Atel_BinTableCount @ 0x0086_ac60 => extern "C" fn h_86ac60(b: u32) -> u32 {
        hp::atel_bin_table_count(&mem(), b)
    }
    /// `Atel_BinTableEntryA`.
    Atel_BinTableEntryA @ 0x0086_ac30 => extern "C" fn h_86ac30(b: u32, i: u32) -> u32 {
        hp::atel_bin_table_entry_a(&mem(), b, i)
    }
    /// `Atel_BinTableEntryB`.
    Atel_BinTableEntryB @ 0x0086_ac90 => extern "C" fn h_86ac90(b: u32, i: u32) -> u32 {
        hp::atel_bin_table_entry_b(&mem(), b, i)
    }
    /// `AtelGetSnd3dWork`.
    AtelGetSnd3dWork @ 0x0087_73f0 => extern "C" fn h_8773f0(i: u32) -> u32 {
        hp::atel_get_snd3d_work(&mem(), i)
    }
    /// `AtelSoundTableGet_00876860`.
    AtelSoundTableGet_00876860 @ 0x0087_6860 => extern "C" fn h_876860(k: u32, i: u32) -> u32 {
        hp::atel_sound_table_get(&mem(), k, i)
    }
    /// `Atel_GetWindowStatus`.
    Atel_GetWindowStatus @ 0x0086_be60 => extern "C" fn h_86be60(i: u32) -> u32 {
        hp::atel_get_window_status(&mem(), i)
    }
    /// `Atel_WindowsBusy`.
    Atel_WindowsBusy @ 0x0086_c8b0 => extern "C" fn h_86c8b0() -> u32 {
        hp::atel_windows_busy(&mem())
    }
    /// `Atel_CtrlRunning`.
    Atel_CtrlRunning @ 0x0086_c530 => extern "C" fn h_86c530(strict: u32) -> u32 {
        hp::atel_ctrl_running(&mem(), strict)
    }
    /// `AtelAllClearBindMask`.
    AtelAllClearBindMask @ 0x0086_0f50 => extern "C" fn h_860f50() -> u32 {
        hp::atel_all_clear_bind_mask(&mut mem())
    }
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

    /// `Native_SpriteShow(worker, args, stack)`.
    Native_SpriteShow @ 0x0085_c370 => extern "C" fn n_85c370(w: u32, a: u32, s: u32) -> u32 {
        native::native_sprite_show(&mut mem(), w, a, s)
    }
    /// `Native_SpriteHide(worker, args, stack)`.
    Native_SpriteHide @ 0x0085_c4c0 => extern "C" fn n_85c4c0(w: u32, a: u32, s: u32) -> u32 {
        native::native_sprite_hide(&mut mem(), w, a, s)
    }
    /// `Native_SpriteSetColor(worker, args, stack)`.
    Native_SpriteSetColor @ 0x0085_c760 => extern "C" fn n_85c760(w: u32, a: u32, s: u32) -> u32 {
        native::native_sprite_set_color(&mut mem(), w, a, s)
    }
    /// `Native_SpriteSetScale(worker, args, stack)`.
    Native_SpriteSetScale @ 0x0085_cde0 => extern "C" fn n_85cde0(w: u32, a: u32, s: u32) -> u32 {
        native::native_sprite_set_scale(&mut mem(), w, a, s)
    }
    /// `Native_WorkerStopMove(worker, args, stack)`.
    Native_WorkerStopMove @ 0x0085_b620 => extern "C" fn n_85b620(w: u32, a: u32, s: u32) -> u32 {
        native::native_worker_stop_move(&mut mem(), w, a, s)
    }
    /// `Native_WorkerStopTurn(worker, args, stack)`.
    Native_WorkerStopTurn @ 0x0085_b950 => extern "C" fn n_85b950(w: u32, a: u32, s: u32) -> u32 {
        native::native_worker_stop_turn(&mut mem(), w, a, s)
    }
    /// `Native_SfxPlayEx(worker, args, stack)`.
    Native_SfxPlayEx @ 0x0085_ff30 => extern "C" fn n_85ff30(w: u32, a: u32, s: u32) -> u32 {
        native::native_sfx_play_ex(&mut mem(), w, a, s)
    }
    /// `Native_SfxPlay(worker, args, stack)`.
    Native_SfxPlay @ 0x0085_f310 => extern "C" fn n_85f310(w: u32, a: u32, s: u32) -> u32 {
        native::native_sfx_play(&mut mem(), w, a, s)
    }
    /// `CT_0013_setPosition(worker, args, stack)`.
    CT_0013_setPosition @ 0x0085_faa0 => extern "C" fn n_85faa0(w: u32, a: u32, s: u32) -> u32 {
        native::ct_0013_set_position(&mut mem(), w, a, s)
    }
    /// `Native_ChrSetMotion_5000(worker, args, stack)`.
    Native_ChrSetMotion_5000 @ 0x00a7_8770 => extern "C" fn n_a78770(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_set_motion(&mut mem(), w, a, s)
    }
    /// `Native_ChrSetMotion_501b(worker, args, stack)`.
    Native_ChrSetMotion_501b @ 0x00a7_92d0 => extern "C" fn n_a792d0(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_set_motion(&mut mem(), w, a, s)
    }
    /// `Native_ChrHide(worker, args, stack)`.
    Native_ChrHide @ 0x00a7_8ac0 => extern "C" fn n_a78ac0(w: u32, a: u32, s: u32) -> u32 {
        native::native_chr_hide(&mut mem(), w, a, s)
    }

    // the map natives (ffx_script::vm::bridge::native_map); six that the
    // dump has no function start for (0x0091b040, ...) run in Rust only
    // through Script_Run's dispatch
    /// `Native_01a5(worker, args, stack)`.
    Native_01a5 @ 0x0085_74c0 => extern "C" fn n_8574c0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_01a5(&mut mem(), w, a, s)
    }
    /// `Native_01e4(worker, args, stack)`.
    Native_01e4 @ 0x0085_8c90 => extern "C" fn n_858c90(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_01e4(&mut mem(), w, a, s)
    }
    /// `Native_01e6(worker, args, stack)`.
    Native_01e6 @ 0x0085_91d0 => extern "C" fn n_8591d0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_01e6(&mut mem(), w, a, s)
    }
    /// `Native_MapSpriteWait(worker, args, stack)`.
    Native_MapSpriteWait @ 0x0091_af60 => extern "C" fn n_91af60(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8004(&mut mem(), w, a, s)
    }
    /// `Native_MapSpriteWait_Poll(worker, args, stack)`.
    Native_MapSpriteWait_Poll @ 0x0091_afa0 => extern "C" fn n_91afa0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8004_poll(&mut mem(), w, a, s)
    }
    /// `Native_MapSpriteWaitEnd(worker, args, stack)`.
    Native_MapSpriteWaitEnd @ 0x0091_c7e0 => extern "C" fn n_91c7e0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8005(&mut mem(), w, a, s)
    }
    /// `Native_MapSpriteWaitEnd_Poll(worker, args, stack)`.
    Native_MapSpriteWaitEnd_Poll @ 0x0091_c860 => extern "C" fn n_91c860(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8005_poll(&mut mem(), w, a, s)
    }
    /// `Native_806a(worker, args, stack)`.
    Native_806a @ 0x0091_af80 => extern "C" fn n_91af80(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_806a(&mut mem(), w, a, s)
    }
    /// `Native_806b(worker, args, stack)`.
    Native_806b @ 0x0091_afc0 => extern "C" fn n_91afc0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_806b(&mut mem(), w, a, s)
    }
    /// `Native_8006(worker, args, stack)`.
    Native_8006 @ 0x0091_aff0 => extern "C" fn n_91aff0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8006(&mut mem(), w, a, s)
    }
    /// `Native_FpBind(worker, args, stack)`.
    Native_FpBind @ 0x0091_b070 => extern "C" fn n_91b070(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_fp_bind(&mut mem(), w, a, s)
    }
    /// `Native_FpBindPos(worker, args, stack)`.
    Native_FpBindPos @ 0x0091_b0a0 => extern "C" fn n_91b0a0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_fp_bind_pos(&mut mem(), w, a, s)
    }
    /// `Native_FpUnbind(worker, args, stack)`.
    Native_FpUnbind @ 0x0091_b130 => extern "C" fn n_91b130(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_fp_unbind(&mut mem(), w, a, s)
    }
    /// `Native_800c(worker, args, stack)`.
    Native_800c @ 0x0091_b160 => extern "C" fn n_91b160(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_800c(&mut mem(), w, a, s)
    }
    /// `Native_MapPartShow(worker, args, stack)`.
    Native_MapPartShow @ 0x0091_b1a0 => extern "C" fn n_91b1a0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_map_part_show(&mut mem(), w, a, s)
    }
    /// `Native_MapPartHide(worker, args, stack)`.
    Native_MapPartHide @ 0x0091_b1e0 => extern "C" fn n_91b1e0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_map_part_hide(&mut mem(), w, a, s)
    }
    /// `Native_8011(worker, args, stack)`.
    Native_8011 @ 0x0091_b200 => extern "C" fn n_91b200(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8011(&mut mem(), w, a, s)
    }
    /// `Native_8014(worker, args, stack)`.
    Native_8014 @ 0x0091_b280 => extern "C" fn n_91b280(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8014(&mut mem(), w, a, s)
    }
    /// `Native_801d(worker, args, stack)`.
    Native_801d @ 0x0091_b400 => extern "C" fn n_91b400(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_801d(&mut mem(), w, a, s)
    }
    /// `Native_801e(worker, args, stack)`.
    Native_801e @ 0x0091_b430 => extern "C" fn n_91b430(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_801e(&mut mem(), w, a, s)
    }
    /// `Native_801f(worker, args, stack)`.
    Native_801f @ 0x0091_b450 => extern "C" fn n_91b450(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_801f(&mut mem(), w, a, s)
    }
    /// `Native_8020(worker, args, stack)`.
    Native_8020 @ 0x0091_b470 => extern "C" fn n_91b470(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8020(&mut mem(), w, a, s)
    }
    /// `Native_8021(worker, args, stack)`.
    Native_8021 @ 0x0091_b490 => extern "C" fn n_91b490(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8021(&mut mem(), w, a, s)
    }
    /// `Native_8022(worker, args, stack)`.
    Native_8022 @ 0x0091_b4b0 => extern "C" fn n_91b4b0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8022(&mut mem(), w, a, s)
    }
    /// `Native_8026(worker, args, stack)`.
    Native_8026 @ 0x0091_b550 => extern "C" fn n_91b550(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8026(&mut mem(), w, a, s)
    }
    /// `Native_802e(worker, args, stack)`.
    Native_802e @ 0x0091_b6a0 => extern "C" fn n_91b6a0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_802e(&mut mem(), w, a, s)
    }
    /// `Native_802f(worker, args, stack)`.
    Native_802f @ 0x0091_b6d0 => extern "C" fn n_91b6d0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_802f(&mut mem(), w, a, s)
    }
    /// `Native_8030(worker, args, stack)`.
    Native_8030 @ 0x0091_b6f0 => extern "C" fn n_91b6f0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8030(&mut mem(), w, a, s)
    }
    /// `Native_8032(worker, args, stack)`.
    Native_8032 @ 0x0091_b740 => extern "C" fn n_91b740(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8032(&mut mem(), w, a, s)
    }
    /// `Native_8035(worker, args, stack)`.
    Native_8035 @ 0x0091_b790 => extern "C" fn n_91b790(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8035(&mut mem(), w, a, s)
    }
    /// `Native_8036(worker, args, stack)`.
    Native_8036 @ 0x0091_b7d0 => extern "C" fn n_91b7d0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8036(&mut mem(), w, a, s)
    }
    /// `Native_FogSetParams(worker, args, stack)`.
    Native_FogSetParams @ 0x0091_b810 => extern "C" fn n_91b810(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8037(&mut mem(), w, a, s)
    }
    /// `Native_FogSetParam0(worker, args, stack)`.
    Native_FogSetParam0 @ 0x0091_b8b0 => extern "C" fn n_91b8b0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8038(&mut mem(), w, a, s)
    }
    /// `Native_FogGetParam0(worker, args, stack)`.
    Native_FogGetParam0 @ 0x0091_b930 => extern "C" fn n_91b930(w: u32, a: u32, s: u32) -> f64 {
        native_map::native_8039(&mut mem(), w, a, s)
    }
    /// `Native_FogSetParam1(worker, args, stack)`.
    Native_FogSetParam1 @ 0x0091_b960 => extern "C" fn n_91b960(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_803a(&mut mem(), w, a, s)
    }
    /// `Native_FogGetParam1(worker, args, stack)`.
    Native_FogGetParam1 @ 0x0091_b9e0 => extern "C" fn n_91b9e0(w: u32, a: u32, s: u32) -> f64 {
        native_map::native_803b(&mut mem(), w, a, s)
    }
    /// `Native_FogSetParam2(worker, args, stack)`.
    Native_FogSetParam2 @ 0x0091_ba10 => extern "C" fn n_91ba10(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_803c(&mut mem(), w, a, s)
    }
    /// `Native_FogGetParam2(worker, args, stack)`.
    Native_FogGetParam2 @ 0x0091_baa0 => extern "C" fn n_91baa0(w: u32, a: u32, s: u32) -> f64 {
        native_map::native_803d(&mut mem(), w, a, s)
    }
    /// `Native_FogSetColor(worker, args, stack)`.
    Native_FogSetColor @ 0x0091_bad0 => extern "C" fn n_91bad0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_803e(&mut mem(), w, a, s)
    }
    /// `Native_FogSetRed(worker, args, stack)`.
    Native_FogSetRed @ 0x0091_bbb0 => extern "C" fn n_91bbb0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_803f(&mut mem(), w, a, s)
    }
    /// `Native_FogSetGreen(worker, args, stack)`.
    Native_FogSetGreen @ 0x0091_bc80 => extern "C" fn n_91bc80(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8040(&mut mem(), w, a, s)
    }
    /// `Native_FogSetBlue(worker, args, stack)`.
    Native_FogSetBlue @ 0x0091_bd50 => extern "C" fn n_91bd50(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8041(&mut mem(), w, a, s)
    }
    /// `Native_FogGetRed(worker, args, stack)`.
    Native_FogGetRed @ 0x0091_be20 => extern "C" fn n_91be20(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8042(&mut mem(), w, a, s)
    }
    /// `Native_FogGetGreen(worker, args, stack)`.
    Native_FogGetGreen @ 0x0091_be50 => extern "C" fn n_91be50(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8043(&mut mem(), w, a, s)
    }
    /// `Native_FogGetBlue(worker, args, stack)`.
    Native_FogGetBlue @ 0x0091_be80 => extern "C" fn n_91be80(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8044(&mut mem(), w, a, s)
    }
    /// `Native_BackSetColor(worker, args, stack)`.
    Native_BackSetColor @ 0x0091_beb0 => extern "C" fn n_91beb0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8045(&mut mem(), w, a, s)
    }
    /// `Native_BackGetRed(worker, args, stack)`.
    Native_BackGetRed @ 0x0091_c000 => extern "C" fn n_91c000(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8049(&mut mem(), w, a, s)
    }
    /// `Native_BackGetGreen(worker, args, stack)`.
    Native_BackGetGreen @ 0x0091_c030 => extern "C" fn n_91c030(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_804a(&mut mem(), w, a, s)
    }
    /// `Native_BackGetBlue(worker, args, stack)`.
    Native_BackGetBlue @ 0x0091_c060 => extern "C" fn n_91c060(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_804b(&mut mem(), w, a, s)
    }
    /// `Native_804f(worker, args, stack)`.
    Native_804f @ 0x0091_c150 => extern "C" fn n_91c150(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_804f(&mut mem(), w, a, s)
    }
    /// `Native_8059(worker, args, stack)`.
    Native_8059 @ 0x0091_c3c0 => extern "C" fn n_91c3c0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8059(&mut mem(), w, a, s)
    }
    /// `Native_FpSetStickPos(worker, args, stack)`.
    Native_FpSetStickPos @ 0x0091_c410 => extern "C" fn n_91c410(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_805b(&mut mem(), w, a, s)
    }
    /// `Native_805c(worker, args, stack)`.
    Native_805c @ 0x0091_c470 => extern "C" fn n_91c470(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_805c(&mut mem(), w, a, s)
    }
    /// `Native_805e(worker, args, stack)`.
    Native_805e @ 0x0091_c4d0 => extern "C" fn n_91c4d0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_805e(&mut mem(), w, a, s)
    }
    /// `Native_805f(worker, args, stack)`.
    Native_805f @ 0x0091_c4f0 => extern "C" fn n_91c4f0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_805f(&mut mem(), w, a, s)
    }
    /// `Native_8060(worker, args, stack)`.
    Native_8060 @ 0x0091_c510 => extern "C" fn n_91c510(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8060(&mut mem(), w, a, s)
    }
    /// `Native_8001(worker, args, stack)`.
    Native_8001 @ 0x0091_c5b0 => extern "C" fn n_91c5b0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8001(&mut mem(), w, a, s)
    }
    /// `Native_8066(worker, args, stack)`.
    Native_8066 @ 0x0091_c7c0 => extern "C" fn n_91c7c0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8066(&mut mem(), w, a, s)
    }
    /// `Native_BltzSetObjModels(worker, args, stack)`.
    Native_BltzSetObjModels @ 0x0091_c810 => extern "C" fn n_91c810(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_bltz_set_obj_models(&mut mem(), w, a, s)
    }

    // the camera natives (ffx_script::vm::bridge::native_cam);
    // Native_Cam_6076 (0x007b8fe0), which the dump has no function start
    // for, runs in Rust only through Script_Run's dispatch
    /// `Native_CamFocusTimeLinear(worker, args, stack)`.
    Native_CamFocusTimeLinear @ 0x007b_7f70 => extern "C" fn n_7b7f70(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_focus_time_linear(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6030(worker, args, stack)`.
    Native_Cam_6030 @ 0x007b_8050 => extern "C" fn n_7b8050(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6030(&mut mem(), w, a, s)
    }
    /// `Native_CamFocusTimeEase(worker, args, stack)`.
    Native_CamFocusTimeEase @ 0x007b_8130 => extern "C" fn n_7b8130(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_focus_time_ease(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6035(worker, args, stack)`.
    Native_Cam_6035 @ 0x007b_8170 => extern "C" fn n_7b8170(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6035(&mut mem(), w, a, s)
    }
    /// `Native_CamFocusCut(worker, args, stack)`.
    Native_CamFocusCut @ 0x007b_8210 => extern "C" fn n_7b8210(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_focus_cut(&mut mem(), w, a, s)
    }
    /// `Native_CamFocusWait(worker, args, stack)`.
    Native_CamFocusWait @ 0x007b_8270 => extern "C" fn n_7b8270(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_focus_wait(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6043(worker, args, stack)`.
    Native_Cam_6043 @ 0x007b_8660 => extern "C" fn n_7b8660(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6043(&mut mem(), w, a, s)
    }
    /// `Native_Cam_603f(worker, args, stack)`.
    Native_Cam_603f @ 0x007b_86f0 => extern "C" fn n_7b86f0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_603f(&mut mem(), w, a, s)
    }
    /// `Native_CamEyeOrbitScaled(worker, args, stack)`.
    Native_CamEyeOrbitScaled @ 0x007b_8710 => extern "C" fn n_7b8710(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_eye_orbit_scaled(&mut mem(), w, a, s)
    }
    /// `Native_CamFocusOrbitScaled(worker, args, stack)`.
    Native_CamFocusOrbitScaled @ 0x007b_8730 => extern "C" fn n_7b8730(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_focus_orbit_scaled(&mut mem(), w, a, s)
    }
    /// `Native_CamEyeOrbit(worker, args, stack)`.
    Native_CamEyeOrbit @ 0x007b_8750 => extern "C" fn n_7b8750(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_eye_orbit(&mut mem(), w, a, s)
    }
    /// `Native_CamFocusOrbit(worker, args, stack)`.
    Native_CamFocusOrbit @ 0x007b_8770 => extern "C" fn n_7b8770(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_focus_orbit(&mut mem(), w, a, s)
    }
    /// `Native_Cam_604d(worker, args, stack)`.
    Native_Cam_604d @ 0x007b_87b0 => extern "C" fn n_7b87b0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_604d(&mut mem(), w, a, s)
    }
    /// `Native_Cam_607d(worker, args, stack)`.
    Native_Cam_607d @ 0x007b_8840 => extern "C" fn n_7b8840(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_607d(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6087(worker, args, stack)`.
    Native_Cam_6087 @ 0x007b_8a10 => extern "C" fn n_7b8a10(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6087(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6088(worker, args, stack)`.
    Native_Cam_6088 @ 0x007b_8a70 => extern "C" fn n_7b8a70(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6088(&mut mem(), w, a, s)
    }
    /// `Native_Cam_605c(worker, args, stack)`.
    Native_Cam_605c @ 0x007b_8e80 => extern "C" fn n_7b8e80(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_605c(&mut mem(), w, a, s)
    }
    /// `Native_CamSuspend(worker, args, stack)`.
    Native_CamSuspend @ 0x007b_8ea0 => extern "C" fn n_7b8ea0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_suspend(&mut mem(), w, a, s)
    }
    /// `Native_Cam_605d(worker, args, stack)`.
    Native_Cam_605d @ 0x007b_8f10 => extern "C" fn n_7b8f10(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_605d(&mut mem(), w, a, s)
    }
    /// `Native_CamResume(worker, args, stack)`.
    Native_CamResume @ 0x007b_8f30 => extern "C" fn n_7b8f30(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_resume(&mut mem(), w, a, s)
    }
    /// `Native_CamSetRoll(worker, args, stack)`.
    Native_CamSetRoll @ 0x007b_9010 => extern "C" fn n_7b9010(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_set_roll(&mut mem(), w, a, s)
    }
    /// `Native_CamSetScreenDepth(worker, args, stack)`.
    Native_CamSetScreenDepth @ 0x007b_90b0 => extern "C" fn n_7b90b0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_set_screen_depth(&mut mem(), w, a, s)
    }
    /// `Native_CamSetPos(worker, args, stack)`.
    Native_CamSetPos @ 0x007b_9220 => extern "C" fn n_7b9220(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_set_pos(&mut mem(), w, a, s)
    }
    /// `Native_CamGetPos(worker, args, stack)`.
    Native_CamGetPos @ 0x007b_9260 => extern "C" fn n_7b9260(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_get_pos(&mut mem(), w, a, s)
    }
    /// `Native_CamSetPolar(worker, args, stack)`.
    Native_CamSetPolar @ 0x007b_92e0 => extern "C" fn n_7b92e0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_set_polar(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6005(worker, args, stack)`.
    Native_Cam_6005 @ 0x007b_9320 => extern "C" fn n_7b9320(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6005(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6083(worker, args, stack)`.
    Native_Cam_6083 @ 0x007b_9340 => extern "C" fn n_7b9340(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6083(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6006(worker, args, stack)`.
    Native_Cam_6006 @ 0x007b_9360 => extern "C" fn n_7b9360(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6006(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6084(worker, args, stack)`.
    Native_Cam_6084 @ 0x007b_9380 => extern "C" fn n_7b9380(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6084(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6007(worker, args, stack)`.
    Native_Cam_6007 @ 0x007b_93a0 => extern "C" fn n_7b93a0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6007(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6008(worker, args, stack)`.
    Native_Cam_6008 @ 0x007b_93c0 => extern "C" fn n_7b93c0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6008(&mut mem(), w, a, s)
    }
    /// `Native_Cam_603c(worker, args, stack)`.
    Native_Cam_603c @ 0x007b_9400 => extern "C" fn n_7b9400(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_603c(&mut mem(), w, a, s)
    }
    /// `Native_Cam_606b(worker, args, stack)`.
    Native_Cam_606b @ 0x007b_94a0 => extern "C" fn n_7b94a0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_606b(&mut mem(), w, a, s)
    }
    /// `Native_Cam_600f(worker, args, stack)`.
    Native_Cam_600f @ 0x007b_9520 => extern "C" fn n_7b9520(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_600f(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6071(worker, args, stack)`.
    Native_Cam_6071 @ 0x007b_9580 => extern "C" fn n_7b9580(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6071(&mut mem(), w, a, s)
    }
    /// `Native_CamMove(worker, args, stack)`.
    Native_CamMove @ 0x007b_96c0 => extern "C" fn n_7b96c0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_move(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6011(worker, args, stack)`.
    Native_Cam_6011 @ 0x007b_9730 => extern "C" fn n_7b9730(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6011(&mut mem(), w, a, s)
    }
    /// `Native_Cam_605f(worker, args, stack)`.
    Native_Cam_605f @ 0x007b_97a0 => extern "C" fn n_7b97a0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_605f(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6012(worker, args, stack)`.
    Native_Cam_6012 @ 0x007b_9800 => extern "C" fn n_7b9800(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6012(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6060(worker, args, stack)`.
    Native_Cam_6060 @ 0x007b_98e0 => extern "C" fn n_7b98e0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6060(&mut mem(), w, a, s)
    }
    /// `Native_CamMoveEased(worker, args, stack)`.
    Native_CamMoveEased @ 0x007b_9940 => extern "C" fn n_7b9940(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_move_eased(&mut mem(), w, a, s)
    }
    /// `Native_CamMoveEased1(worker, args, stack)`.
    Native_CamMoveEased1 @ 0x007b_9980 => extern "C" fn n_7b9980(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_move_eased1(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6063(worker, args, stack)`.
    Native_Cam_6063 @ 0x007b_99a0 => extern "C" fn n_7b99a0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6063(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6017(worker, args, stack)`.
    Native_Cam_6017 @ 0x007b_9a00 => extern "C" fn n_7b9a00(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6017(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6062(worker, args, stack)`.
    Native_Cam_6062 @ 0x007b_9a20 => extern "C" fn n_7b9a20(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6062(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6064(worker, args, stack)`.
    Native_Cam_6064 @ 0x007b_9aa0 => extern "C" fn n_7b9aa0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6064(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6019(worker, args, stack)`.
    Native_Cam_6019 @ 0x007b_9ac0 => extern "C" fn n_7b9ac0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6019(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6064_Poll(worker, args, stack)`.
    Native_Cam_6064_Poll @ 0x007b_9b20 => extern "C" fn n_7b9b20(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6064_poll(&mut mem(), w, a, s)
    }
    /// `Native_CamRefSetPos(worker, args, stack)`.
    Native_CamRefSetPos @ 0x007b_9d00 => extern "C" fn n_7b9d00(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_ref_set_pos(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6046(worker, args, stack)`.
    Native_Cam_6046 @ 0x007b_9d20 => extern "C" fn n_7b9d20(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6046(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6021(worker, args, stack)`.
    Native_Cam_6021 @ 0x007b_9d40 => extern "C" fn n_7b9d40(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6021(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6047(worker, args, stack)`.
    Native_Cam_6047 @ 0x007b_9d60 => extern "C" fn n_7b9d60(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6047(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6048(worker, args, stack)`.
    Native_Cam_6048 @ 0x007b_9da0 => extern "C" fn n_7b9da0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6048(&mut mem(), w, a, s)
    }
    /// `Native_CamPathAddEye(worker, args, stack)`.
    Native_CamPathAddEye @ 0x007b_9e30 => extern "C" fn n_7b9e30(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_path_add_eye(&mut mem(), w, a, s)
    }
    /// `Native_Cam_604a(worker, args, stack)`.
    Native_Cam_604a @ 0x007b_9eb0 => extern "C" fn n_7b9eb0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_604a(&mut mem(), w, a, s)
    }
    /// `Native_CamPathAddFocus(worker, args, stack)`.
    Native_CamPathAddFocus @ 0x007b_9f40 => extern "C" fn n_7b9f40(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_path_add_focus(&mut mem(), w, a, s)
    }
    /// `Native_CamRefSetTarget(worker, args, stack)`.
    Native_CamRefSetTarget @ 0x007b_9fe0 => extern "C" fn n_7b9fe0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_ref_set_target(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6028(worker, args, stack)`.
    Native_Cam_6028 @ 0x007b_a0e0 => extern "C" fn n_7ba0e0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6028(&mut mem(), w, a, s)
    }
    /// `Native_CamRefAddLineArea(worker, args, stack)`.
    Native_CamRefAddLineArea @ 0x007b_a1d0 => extern "C" fn n_7ba1d0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_ref_add_line_area(&mut mem(), w, a, s)
    }
    /// `Native_CamRefAddCircleArea(worker, args, stack)`.
    Native_CamRefAddCircleArea @ 0x007b_a2d0 => extern "C" fn n_7ba2d0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_ref_add_circle_area(&mut mem(), w, a, s)
    }
    /// `Native_CamRefClearAreas(worker, args, stack)`.
    Native_CamRefClearAreas @ 0x007b_a2f0 => extern "C" fn n_7ba2f0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_ref_clear_areas(&mut mem(), w, a, s)
    }

    // the message and voice natives (ffx_script::vm::bridge::native_msg);
    // the dump gives the two voice waits (0x0085e1d0, 0x0085e4a0) one byte,
    // so they run in Rust only through Script_Run's dispatch
    /// `Native_MsgSetValue(worker, args, stack)`.
    Native_MsgSetValue @ 0x0085_5c70 => extern "C" fn n_855c70(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_set_value(&mut mem(), w, a, s)
    }
    /// `Native_MacroDicText(worker, args, stack)`.
    Native_MacroDicText @ 0x0085_6a00 => extern "C" fn n_856a00(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_macro_dic_text(&mut mem(), w, a, s)
    }
    /// `Native_MsgSetText(worker, args, stack)`.
    Native_MsgSetText @ 0x0085_7870 => extern "C" fn n_857870(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_set_text(&mut mem(), w, a, s)
    }
    /// `Native_BitIndex(worker, args, stack)`.
    Native_BitIndex @ 0x0085_78c0 => extern "C" fn n_8578c0(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_bit_index(&mut mem(), w, a, s)
    }
    /// `Native_MsgSetPos(worker, args, stack)`.
    Native_MsgSetPos @ 0x0085_80c0 => extern "C" fn n_8580c0(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_set_pos(&mut mem(), w, a, s)
    }
    /// `Native_MsgClose(worker, args, stack)`.
    Native_MsgClose @ 0x0085_9060 => extern "C" fn n_859060(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_close(&mut mem(), w, a, s)
    }
    /// `Msg_Text(i)`.
    Msg_Text @ 0x0086_bf30 => extern "C" fn msg_text(i: u32) -> u32 {
        native_msg::msg_text(&mut mem(), i)
    }

    // the move, turn, worker, character-motion and screen natives (ffx_script::vm::bridge::native_move)
    /// `Native_MoveStart(worker, args, stack)`.
    Native_MoveStart @ 0x0085_6c30 => extern "C" fn n_856c30(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_move_start(&mut mem(), w, a, s)
    }
    /// `Native_TurnStart(worker, args, stack)`.
    Native_TurnStart @ 0x0085_7520 => extern "C" fn n_857520(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_turn_start(&mut mem(), w, a, s)
    }
    /// `Native_MoveSetTarget(worker, args, stack)`.
    Native_MoveSetTarget @ 0x0086_02f0 => extern "C" fn n_8602f0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_move_set_target(&mut mem(), w, a, s)
    }
    /// `Native_TurnSetSpeed2(worker, args, stack)`.
    Native_TurnSetSpeed2 @ 0x0085_a5c0 => extern "C" fn n_85a5c0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_turn_set_speed2(&mut mem(), w, a, s)
    }
    /// `Native_TurnSetSpeed(worker, args, stack)`.
    Native_TurnSetSpeed @ 0x0085_a2a0 => extern "C" fn n_85a2a0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_turn_set_speed(&mut mem(), w, a, s)
    }
    /// `Native_SetTurnSpeed(worker, args, stack)`.
    Native_SetTurnSpeed @ 0x0085_a480 => extern "C" fn n_85a480(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_set_turn_speed(&mut mem(), w, a, s)
    }
    /// `Native_SetMoveSpeed(worker, args, stack)`.
    Native_SetMoveSpeed @ 0x0085_7f40 => extern "C" fn n_857f40(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_set_move_speed(&mut mem(), w, a, s)
    }
    /// `Native_MoveSetSpeed(worker, args, stack)`.
    Native_MoveSetSpeed @ 0x0085_7da0 => extern "C" fn n_857da0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_move_set_speed(&mut mem(), w, a, s)
    }
    /// `Native_SetArriveRadiusAll(worker, args, stack)`.
    Native_SetArriveRadiusAll @ 0x0085_8020 => extern "C" fn n_858020(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_set_arrive_radius_all(&mut mem(), w, a, s)
    }
    /// `Native_SetArriveRadius(worker, args, stack)`.
    Native_SetArriveRadius @ 0x0085_f040 => extern "C" fn n_85f040(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_set_arrive_radius(&mut mem(), w, a, s)
    }
    /// `Native_SetYaw(worker, args, stack)`.
    Native_SetYaw @ 0x0085_f170 => extern "C" fn n_85f170(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_set_yaw(&mut mem(), w, a, s)
    }
    /// `Native_GetYawToMoveTarget(worker, args, stack)`.
    Native_GetYawToMoveTarget @ 0x0085_87a0 => extern "C" fn n_8587a0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_get_yaw_to_move_target(&mut mem(), w, a, s)
    }
    /// `Native_AngleToPoint(worker, args, stack)`.
    Native_AngleToPoint @ 0x0085_8970 => extern "C" fn n_858970(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_angle_to_point(&mut mem(), w, a, s)
    }
    /// `Native_SetTalkRange(worker, args, stack)`.
    Native_SetTalkRange @ 0x0085_7180 => extern "C" fn n_857180(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_set_talk_range(&mut mem(), w, a, s)
    }
    /// `Native_GetEntranceZ(worker, args, stack)`.
    Native_GetEntranceZ @ 0x0085_cb20 => extern "C" fn n_85cb20(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_get_entrance_z(&mut mem(), w, a, s)
    }
    /// `Native_GetEntranceYaw(worker, args, stack)`.
    Native_GetEntranceYaw @ 0x0085_cce0 => extern "C" fn n_85cce0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_get_entrance_yaw(&mut mem(), w, a, s)
    }
    /// `AtelCall_Math_Sqrt_RF(worker, args, stack)`.
    AtelCall_Math_Sqrt_RF @ 0x0087_7b70 => extern "C" fn n_877b70(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_math_sqrt(&mut mem(), w, a, s)
    }
    /// `Native_WaitFrames_0000(worker, args, stack)`.
    Native_WaitFrames_0000 @ 0x0085_c4e0 => extern "C" fn n_85c4e0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_wait_frames(&mut mem(), w, a, s)
    }
    /// `Native_WaitFrames_Poll_0000(worker, args, stack)`.
    Native_WaitFrames_Poll_0000 @ 0x0085_c6b0 => extern "C" fn n_85c6b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_wait_frames_poll(&mut mem(), w, a, s)
    }
    /// `Native_MapChange(worker, args, stack)`.
    Native_MapChange @ 0x0085_81c0 => extern "C" fn n_8581c0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_map_change(&mut mem(), w, a, s)
    }
    /// `Native_MapJump(worker, args, stack)`.
    Native_MapJump @ 0x0085_8460 => extern "C" fn n_858460(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_map_jump(&mut mem(), w, a, s)
    }
    /// `Native_WorkerDetach(worker, args, stack)`.
    Native_WorkerDetach @ 0x0085_cdc0 => extern "C" fn n_85cdc0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_worker_detach(&mut mem(), w, a, s)
    }
    /// `Native_ChrLoadAlias(worker, args, stack)`.
    Native_ChrLoadAlias @ 0x0085_d3c0 => extern "C" fn n_85d3c0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_chr_load_alias(&mut mem(), w, a, s)
    }
    /// `Native_BitFlagSet(worker, args, stack)`.
    Native_BitFlagSet @ 0x0085_c3b0 => extern "C" fn n_85c3b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_bit_flag_set(&mut mem(), w, a, s)
    }
    /// `CT_0060_ret(worker, args, stack)`.
    CT_0060_ret @ 0x0085_6920 => extern "C" fn n_856920(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0060(&mut mem(), w, a, s)
    }
    /// `Native_FootstepHookEnable(worker, args, stack)`.
    Native_FootstepHookEnable @ 0x0085_9410 => extern "C" fn n_859410(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_footstep_hook_enable(&mut mem(), w, a, s)
    }
    /// `Native_003d(worker, args, stack)`.
    Native_003d @ 0x0085_c3e0 => extern "C" fn n_85c3e0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_003d(&mut mem(), w, a, s)
    }
    /// `Native_SetMoveMOT(worker, args, stack)`.
    Native_SetMoveMOT @ 0x0085_bdc0 => extern "C" fn n_85bdc0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_set_move_mot(&mut mem(), w, a, s)
    }
    /// `Native_LineGetCrossSide(worker, args, stack)`.
    Native_LineGetCrossSide @ 0x0085_d410 => extern "C" fn n_85d410(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_line_get_cross_side(&mut mem(), w, a, s)
    }
    /// `Native_0036(worker, args, stack)`.
    Native_0036 @ 0x0085_b4e0 => extern "C" fn n_85b4e0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0036(&mut mem(), w, a, s)
    }
    /// `Native_GetWorkerPos(worker, args, stack)`.
    Native_GetWorkerPos @ 0x0085_6e20 => extern "C" fn n_856e20(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_get_worker_pos(&mut mem(), w, a, s)
    }
    /// `Native_PartySwapIn(worker, args, stack)`.
    Native_PartySwapIn @ 0x0085_bdf0 => extern "C" fn n_85bdf0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_party_swap_in(&mut mem(), w, a, s)
    }
    /// `Native_SetActorProperty(worker, args, stack)`.
    Native_SetActorProperty @ 0x007a_5360 => extern "C" fn n_7a5360(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_set_actor_property(&mut mem(), w, a, s)
    }
    /// `Native_ActorPlayMotion(worker, args, stack)`.
    Native_ActorPlayMotion @ 0x007a_35a0 => extern "C" fn n_7a35a0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_actor_play_motion(&mut mem(), w, a, s)
    }
    /// `Native_ChrPlayMotion(worker, args, stack)`.
    Native_ChrPlayMotion @ 0x00a7_9270 => extern "C" fn n_a79270(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_chr_play_motion(&mut mem(), w, a, s)
    }
    /// `Native_ChrPlayMotion2(worker, args, stack)`.
    Native_ChrPlayMotion2 @ 0x00a7_a780 => extern "C" fn n_a7a780(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_chr_play_motion(&mut mem(), w, a, s)
    }
    /// `Native_ChrPlayDefaultMotion(worker, args, stack)`.
    Native_ChrPlayDefaultMotion @ 0x00a7_9e60 => extern "C" fn n_a79e60(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_chr_play_default_motion(&mut mem(), w, a, s)
    }
    /// `Native_ChrSetMotion2(worker, args, stack)`.
    Native_ChrSetMotion2 @ 0x00a7_a610 => extern "C" fn n_a7a610(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_chr_set_motion2(&mut mem(), w, a, s)
    }
    /// `Native_ChrSetMotionSeqLevel(worker, args, stack)`.
    Native_ChrSetMotionSeqLevel @ 0x00a7_9550 => extern "C" fn n_a79550(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_chr_set_motion_seq_level(&mut mem(), w, a, s)
    }
    /// `Native_ChrLoad(worker, args, stack)`.
    Native_ChrLoad @ 0x00a7_9800 => extern "C" fn n_a79800(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_chr_load(&mut mem(), w, a, s)
    }
    /// `Native_FadeIn(worker, args, stack)`.
    Native_FadeIn @ 0x00a7_7d80 => extern "C" fn n_a77d80(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_fade_in(&mut mem(), w, a, s)
    }
    /// `Native_FadeOut(worker, args, stack)`.
    Native_FadeOut @ 0x00a7_7da0 => extern "C" fn n_a77da0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_fade_out(&mut mem(), w, a, s)
    }
    /// `Native_MotionBlur(worker, args, stack)`.
    Native_MotionBlur @ 0x00a7_7e00 => extern "C" fn n_a77e00(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_motion_blur(&mut mem(), w, a, s)
    }

    // the message waits and opens (native_msg)
    /// `Native_MsgOpen(worker, args, stack)`.
    Native_MsgOpen @ 0x0085_8b50 => extern "C" fn n_858b50(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_open(&mut mem(), w, a, s)
    }
    /// `Native_MsgSelect(worker, args, stack)`.
    Native_MsgSelect @ 0x0085_9710 => extern "C" fn n_859710(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_select(&mut mem(), w, a, s)
    }
    /// `Native_MsgSelect_Poll(worker, args, stack)`.
    Native_MsgSelect_Poll @ 0x0085_9c50 => extern "C" fn n_859c50(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_select_poll(&mut mem(), w, a, s)
    }
    /// `Native_MsgShowWait_Poll(worker, args, stack)`.
    Native_MsgShowWait_Poll @ 0x0086_03d0 => extern "C" fn n_8603d0(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_select_poll(&mut mem(), w, a, s)
    }
    /// `Native_MsgSelect_Result(worker, args, stack)`.
    Native_MsgSelect_Result @ 0x0085_a6f0 => extern "C" fn n_85a6f0(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_select_result(&mut mem(), w, a, s)
    }
    /// `Native_MsgShowWait_Result(worker, args, stack)`.
    Native_MsgShowWait_Result @ 0x0086_0760 => extern "C" fn n_860760(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_select_result(&mut mem(), w, a, s)
    }
    /// `Native_MsgWaitEx(worker, args, stack)`.
    Native_MsgWaitEx @ 0x0085_abd0 => extern "C" fn n_85abd0(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_wait_ex(&mut mem(), w, a, s)
    }
    /// `Native_MsgWaitEx_Poll(worker, args, stack)`.
    Native_MsgWaitEx_Poll @ 0x0085_ae90 => extern "C" fn n_85ae90(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_wait_ex_poll(&mut mem(), w, a, s)
    }
    /// `Native_MsgWait_Poll(worker, args, stack)`.
    Native_MsgWait_Poll @ 0x0085_b980 => extern "C" fn n_85b980(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_wait_ex_poll(&mut mem(), w, a, s)
    }
    /// `Native_MsgWaitEx_End(worker, args, stack)`.
    Native_MsgWaitEx_End @ 0x0085_b6d0 => extern "C" fn n_85b6d0(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_wait_ex_end(&mut mem(), w, a, s)
    }
    /// `Native_MsgWait_End(worker, args, stack)`.
    Native_MsgWait_End @ 0x0085_ba80 => extern "C" fn n_85ba80(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_wait_ex_end(&mut mem(), w, a, s)
    }
    /// `Native_MsgWait(worker, args, stack)`.
    Native_MsgWait @ 0x0085_b7c0 => extern "C" fn n_85b7c0(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_wait(&mut mem(), w, a, s)
    }
    /// `Native_MsgShowWait(worker, args, stack)`.
    Native_MsgShowWait @ 0x0086_02a0 => extern "C" fn n_8602a0(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_show_wait(&mut mem(), w, a, s)
    }

    // more camera natives (native_cam)
    /// `Native_Cam_601c(worker, args, stack)`.
    Native_Cam_601c @ 0x007b_8330 => extern "C" fn n_7b8330(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_601c(&mut mem(), w, a, s)
    }
    /// `Native_Cam_601d(worker, args, stack)`.
    Native_Cam_601d @ 0x007b_8470 => extern "C" fn n_7b8470(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_601d(&mut mem(), w, a, s)
    }
    /// `Native_Cam_601f(worker, args, stack)`.
    Native_Cam_601f @ 0x007b_8530 => extern "C" fn n_7b8530(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_601f(&mut mem(), w, a, s)
    }
    /// `Native_BtlRandom(worker, args, stack)`.
    Native_BtlRandom @ 0x007b_89a0 => extern "C" fn n_7b89a0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_btl_random(&mut mem(), w, a, s)
    }
    /// `Native_CamMoveFinish(worker, args, stack)`.
    Native_CamMoveFinish @ 0x007b_9b40 => extern "C" fn n_7b9b40(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_move_finish(&mut mem(), w, a, s)
    }
    /// `Native_CamWait(worker, args, stack)`.
    Native_CamWait @ 0x007b_9be0 => extern "C" fn n_7b9be0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_wait(&mut mem(), w, a, s)
    }
    /// `Native_Cam_6059(worker, args, stack)`.
    Native_Cam_6059 @ 0x007b_a020 => extern "C" fn n_7ba020(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6059(&mut mem(), w, a, s)
    }
    /// `Native_Cam_605a(worker, args, stack)`.
    Native_Cam_605a @ 0x007b_a100 => extern "C" fn n_7ba100(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_605a(&mut mem(), w, a, s)
    }
    /// `Native_Cam_605b(worker, args, stack)`.
    Native_Cam_605b @ 0x007b_a210 => extern "C" fn n_7ba210(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_605b(&mut mem(), w, a, s)
    }
    /// `Msg_ShowSetup(worker, args, stack, mode)`.
    Msg_ShowSetup @ 0x0086_07a0 => extern "C" fn msg_show_setup(w: u32, a: u32, s: u32, mode: u32) -> u32 {
        native_msg::msg_show_setup(&mut mem(), w, a, s, mode)
    }

    // round 8 (native_msg)
    /// `Native_0267_Poll(worker, args, stack)`.
    Native_0267_Poll @ 0x0085_a540 => extern "C" fn n_85a540(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_0267_poll(&mut mem(), w, a, s)
    }
    /// `Native_0267_Result(worker, args, stack)`.
    Native_0267_Result @ 0x0085_a620 => extern "C" fn n_85a620(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_select_result(&mut mem(), w, a, s)
    }
    /// `Native_VoiceStandby(worker, args, stack)`.
    Native_VoiceStandby @ 0x0085_cd00 => extern "C" fn n_85cd00(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_voice_standby(&mut mem(), w, a, s)
    }
    /// `Native_VoiceStandby_End(worker, args, stack)`.
    Native_VoiceStandby_End @ 0x0085_d360 => extern "C" fn n_85d360(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_voice_standby_end(&mut mem(), w, a, s)
    }
    /// `Native_VoiceStart(worker, args, stack)`.
    Native_VoiceStart @ 0x0085_d6b0 => extern "C" fn n_85d6b0(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_voice_start(&mut mem(), w, a, s)
    }
    /// `Native_VoiceStart_Poll(worker, args, stack)`.
    Native_VoiceStart_Poll @ 0x0085_d9d0 => extern "C" fn n_85d9d0(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_voice_start_poll(&mut mem(), w, a, s)
    }

    // round 8 (native_move)
    /// `Native_SpriteSetPos(worker, args, stack)`.
    Native_SpriteSetPos @ 0x0085_cc50 => extern "C" fn n_85cc50(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_sprite_set_pos(&mut mem(), w, a, s)
    }
    /// `Native_LightSet(worker, args, stack)`.
    Native_LightSet @ 0x00a7_8030 => extern "C" fn n_a78030(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_light_set(&mut mem(), w, a, s)
    }
    /// `Native_SetModel(worker, args, stack)`.
    Native_SetModel @ 0x0085_d000 => extern "C" fn n_85d000(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_set_model(&mut mem(), w, a, s)
    }

    // round 8 (native_btl)
    /// `Native_SetActorsProperty(worker, args, stack)`.
    Native_SetActorsProperty @ 0x007a_50c0 => extern "C" fn n_7a50c0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_set_actors_property(&mut mem(), w, a, s)
    }
    /// `Native_SetSelfFloatProperty(worker, args, stack)`.
    Native_SetSelfFloatProperty @ 0x007a_5930 => extern "C" fn n_7a5930(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_set_self_float_property(&mut mem(), w, a, s)
    }
    /// `Native_GetActorProperty(worker, args, stack)`.
    Native_GetActorProperty @ 0x007a_4d50 => extern "C" fn n_7a4d50(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_get_actor_property(&mut mem(), w, a, s)
    }
    /// `Native_GetSelfProperty(worker, args, stack)`.
    Native_GetSelfProperty @ 0x007a_4ea0 => extern "C" fn n_7a4ea0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_get_self_property(&mut mem(), w, a, s)
    }
    /// `Native_BtlUseCommand(worker, args, stack)`.
    Native_BtlUseCommand @ 0x007a_44b0 => extern "C" fn n_7a44b0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_btl_use_command(&mut mem(), w, a, s)
    }
    /// `Native_ActorWaitMotion(worker, args, stack)`.
    Native_ActorWaitMotion @ 0x007a_3980 => extern "C" fn n_7a3980(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_actor_wait_motion(&mut mem(), w, a, s)
    }
    /// `Native_BtlSelectTarget(worker, args, stack)`.
    Native_BtlSelectTarget @ 0x007a_5af0 => extern "C" fn n_7a5af0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_btl_select_target(&mut mem(), w, a, s)
    }
    /// `Native_BtlCountActorsInBoth(worker, args, stack)`.
    Native_BtlCountActorsInBoth @ 0x007a_6870 => extern "C" fn n_7a6870(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_btl_count_actors_in_both(&mut mem(), w, a, s)
    }
    /// `Native_ActorGetSlotRef(worker, args, stack)`.
    Native_ActorGetSlotRef @ 0x007a_6b40 => extern "C" fn n_7a6b40(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_actor_get_slot_ref(&mut mem(), w, a, s)
    }
    /// `Native_7002(worker, args, stack)`.
    Native_7002 @ 0x007a_3530 => extern "C" fn n_7a3530(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7002(&mut mem(), w, a, s)
    }
    /// `Native_BtlRunEvent(worker, args, stack)`.
    Native_BtlRunEvent @ 0x007a_5df0 => extern "C" fn n_7a5df0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_btl_run_event(&mut mem(), w, a, s)
    }
    /// `Native_BtlRunEvent_Poll(worker, args, stack)`.
    Native_BtlRunEvent_Poll @ 0x007a_5ec0 => extern "C" fn n_7a5ec0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_btl_run_event_poll(&mut mem(), w, a, s)
    }
    /// `Native_7097_Poll(worker, args, stack)`.
    Native_7097_Poll @ 0x007a_58c0 => extern "C" fn n_7a58c0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_btl_run_event_poll(&mut mem(), w, a, s)
    }
    /// `Native_ActorQueueMotion(worker, args, stack)`.
    Native_ActorQueueMotion @ 0x007a_2a20 => extern "C" fn n_7a2a20(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_actor_queue_motion(&mut mem(), w, a, s)
    }
    /// `Native_70ac(worker, args, stack)`.
    Native_70ac @ 0x007a_5650 => extern "C" fn n_7a5650(w: u32, a: u32, s: u32) -> f64 {
        native_btl::native_70ac(&mut mem(), w, a, s)
    }
    /// `Native_700c(worker, args, stack)`.
    Native_700c @ 0x007a_4310 => extern "C" fn n_7a4310(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_700c(&mut mem(), w, a, s)
    }
    /// `Native_7085(worker, args, stack)`.
    Native_7085 @ 0x007a_43a0 => extern "C" fn n_7a43a0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7085(&mut mem(), w, a, s)
    }
    /// `Native_7000(worker, args, stack)`.
    Native_7000 @ 0x007a_8620 => extern "C" fn n_7a8620(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7000(&mut mem(), w, a, s)
    }
    /// `Native_7006(worker, args, stack)`.
    Native_7006 @ 0x007a_3f50 => extern "C" fn n_7a3f50(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7006(&mut mem(), w, a, s)
    }
    /// `Native_7003(worker, args, stack)`.
    Native_7003 @ 0x007a_3d90 => extern "C" fn n_7a3d90(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7003(&mut mem(), w, a, s)
    }
    /// `Native_707b(worker, args, stack)`.
    Native_707b @ 0x007a_7470 => extern "C" fn n_7a7470(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_707b(&mut mem(), w, a, s)
    }
    /// `Native_7048(worker, args, stack)`.
    Native_7048 @ 0x007a_53f0 => extern "C" fn n_7a53f0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7048(&mut mem(), w, a, s)
    }
    /// `Native_ActorGetId(worker, args, stack)`.
    Native_ActorGetId @ 0x007a_6a80 => extern "C" fn n_7a6a80(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_actor_get_id(&mut mem(), w, a, s)
    }
    /// `Native_70a2(worker, args, stack)`.
    Native_70a2 @ 0x007a_7440 => extern "C" fn n_7a7440(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70a2(&mut mem(), w, a, s)
    }
    /// `Native_702b(worker, args, stack)`.
    Native_702b @ 0x007a_3c50 => extern "C" fn n_7a3c50(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_702b(&mut mem(), w, a, s)
    }
    /// `Native_ActorSetFlag36(worker, args, stack)`.
    Native_ActorSetFlag36 @ 0x007a_51e0 => extern "C" fn n_7a51e0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_actor_set_flag36(&mut mem(), w, a, s)
    }
    /// `Native_7097(worker, args, stack)`.
    Native_7097 @ 0x007a_5780 => extern "C" fn n_7a5780(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7097(&mut mem(), w, a, s)
    }
    /// `Native_7026(worker, args, stack)`.
    Native_7026 @ 0x007a_5d80 => extern "C" fn n_7a5d80(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7026(&mut mem(), w, a, s)
    }
    /// `Native_702e(worker, args, stack)`.
    Native_702e @ 0x007a_6d70 => extern "C" fn n_7a6d70(w: u32, a: u32, s: u32) -> f64 {
        native_btl::native_702e(&mut mem(), w, a, s)
    }
    /// `Native_702d(worker, args, stack)`.
    Native_702d @ 0x007a_4e60 => extern "C" fn n_7a4e60(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_702d(&mut mem(), w, a, s)
    }
    /// `Native_7019(worker, args, stack)`.
    Native_7019 @ 0x007a_62d0 => extern "C" fn n_7a62d0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7019(&mut mem(), w, a, s)
    }
    /// `Native_704a(worker, args, stack)`.
    Native_704a @ 0x007a_58d0 => extern "C" fn n_7a58d0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_704a(&mut mem(), w, a, s)
    }
    /// `Native_701d(worker, args, stack)`.
    Native_701d @ 0x007a_5bf0 => extern "C" fn n_7a5bf0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_701d(&mut mem(), w, a, s)
    }
    /// `Native_700a(worker, args, stack)`.
    Native_700a @ 0x007a_4100 => extern "C" fn n_7a4100(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_700a(&mut mem(), w, a, s)
    }
    /// `Native_701a(worker, args, stack)`.
    Native_701a @ 0x007a_6390 => extern "C" fn n_7a6390(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_701a(&mut mem(), w, a, s)
    }
    /// `Native_7037(worker, args, stack)`.
    Native_7037 @ 0x007a_6d30 => extern "C" fn n_7a6d30(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7037(&mut mem(), w, a, s)
    }
    /// `Native_706f(worker, args, stack)`.
    Native_706f @ 0x007a_36c0 => extern "C" fn n_7a36c0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_706f(&mut mem(), w, a, s)
    }
    /// `Native_7104(worker, args, stack)`.
    Native_7104 @ 0x007a_6600 => extern "C" fn n_7a6600(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7104(&mut mem(), w, a, s)
    }
    /// `Msg_ChoicePoll(worker, args, handlers)`.
    Msg_ChoicePoll @ 0x0086_0990 => extern "C" fn msg_choice_poll(_w: u32, a: u32, t: u32) -> u32 {
        native_msg::msg_choice_poll(&mut mem(), a, t)
    }

    // round 9
    /// `Native_0006(worker, args, stack)`.
    Native_0006 @ 0x0085_e620 => extern "C" fn n_85e620(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0006(&mut mem(), w, a, s)
    }
    /// `Native_00f5(worker, args, stack)`.
    Native_00f5 @ 0x0085_8f40 => extern "C" fn n_858f40(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_00f5(&mut mem(), w, a, s)
    }
    /// `Native_00cb(worker, args, stack)`.
    Native_00cb @ 0x0085_b820 => extern "C" fn n_85b820(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00cb(&mut mem(), w, a, s)
    }
    /// `CT_401D_init(worker, args, stack)`.
    CT_401D_init @ 0x00a7_8210 => extern "C" fn n_a78210(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_menu_open(&mut mem(), w, a, s)
    }
    /// `Native_703c_Poll(worker, args, stack)`.
    Native_703c_Poll @ 0x007a_5680 => extern "C" fn n_7a5680(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_btl_run_event_poll(&mut mem(), w, a, s)
    }
    /// `Native_7009(worker, args, stack)`.
    Native_7009 @ 0x007a_3b20 => extern "C" fn n_7a3b20(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7009(&mut mem(), w, a, s)
    }
    /// `Native_7058(worker, args, stack)`.
    Native_7058 @ 0x007a_8160 => extern "C" fn n_7a8160(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7058(&mut mem(), w, a, s)
    }
    /// `Native_70a8(worker, args, stack)`.
    Native_70a8 @ 0x007a_57e0 => extern "C" fn n_7a57e0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70a8(&mut mem(), w, a, s)
    }
    /// `Native_706b(worker, args, stack)`.
    Native_706b @ 0x007a_84c0 => extern "C" fn n_7a84c0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_706b(&mut mem(), w, a, s)
    }
    /// `Native_7029(worker, args, stack)`.
    Native_7029 @ 0x007a_3fb0 => extern "C" fn n_7a3fb0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7029(&mut mem(), w, a, s)
    }
    /// `Native_701c(worker, args, stack)`.
    Native_701c @ 0x007a_59d0 => extern "C" fn n_7a59d0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_701c(&mut mem(), w, a, s)
    }
    /// `Native_7033(worker, args, stack)`.
    Native_7033 @ 0x007a_3cc0 => extern "C" fn n_7a3cc0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7033(&mut mem(), w, a, s)
    }
    /// `Native_70e0(worker, args, stack)`.
    Native_70e0 @ 0x007a_8450 => extern "C" fn n_7a8450(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70e0(&mut mem(), w, a, s)
    }
    /// `Native_703c(worker, args, stack)`.
    Native_703c @ 0x007a_5540 => extern "C" fn n_7a5540(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_703c(&mut mem(), w, a, s)
    }
    /// `CT_703B_ret_setCommandDisabled(worker, args, stack)`.
    CT_703B_ret_setCommandDisabled @ 0x007a_6ef0 => extern "C" fn n_7a6ef0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_703b(&mut mem(), w, a, s)
    }
    /// `Native_70c9(worker, args, stack)`.
    Native_70c9 @ 0x007a_7740 => extern "C" fn n_7a7740(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70c9(&mut mem(), w, a, s)
    }
    /// `Native_709e(worker, args, stack)`.
    Native_709e @ 0x007a_2fe0 => extern "C" fn n_7a2fe0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_709e(&mut mem(), w, a, s)
    }
    /// `Native_707a(worker, args, stack)`.
    Native_707a @ 0x007a_2a80 => extern "C" fn n_7a2a80(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_707a(&mut mem(), w, a, s)
    }
    /// `Native_705a(worker, args, stack)`.
    Native_705a @ 0x007a_49f0 => extern "C" fn n_7a49f0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_705a(&mut mem(), w, a, s)
    }
    /// `Native_7017(worker, args, stack)`.
    Native_7017 @ 0x007a_4050 => extern "C" fn n_7a4050(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7017(&mut mem(), w, a, s)
    }
    /// `Native_7059(worker, args, stack)`.
    Native_7059 @ 0x007a_6e40 => extern "C" fn n_7a6e40(w: u32, a: u32, s: u32) -> f64 {
        native_btl::native_7059(&mut mem(), w, a, s)
    }
    /// `Native_7039(worker, args, stack)`.
    Native_7039 @ 0x007a_2b10 => extern "C" fn n_7a2b10(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7039(&mut mem(), w, a, s)
    }
    /// `Native_DebugPrint(worker, args, stack)`.
    Native_DebugPrint @ 0x007a_4910 => extern "C" fn n_7a4910(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_debug_print(&mut mem(), w, a, s)
    }
    /// `Native_7012(worker, args, stack)`.
    Native_7012 @ 0x007a_29a0 => extern "C" fn n_7a29a0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7012(&mut mem(), w, a, s)
    }
    /// `Native_7086(worker, args, stack)`.
    Native_7086 @ 0x007a_2ce0 => extern "C" fn n_7a2ce0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7086(&mut mem(), w, a, s)
    }
    /// `Native_7055(worker, args, stack)`.
    Native_7055 @ 0x007a_3eb0 => extern "C" fn n_7a3eb0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7055(&mut mem(), w, a, s)
    }
    /// `Native_7038(worker, args, stack)`.
    Native_7038 @ 0x007a_6dd0 => extern "C" fn n_7a6dd0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7038(&mut mem(), w, a, s)
    }
    /// `Native_7108(worker, args, stack)`.
    Native_7108 @ 0x007a_40a0 => extern "C" fn n_7a40a0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7108(&mut mem(), w, a, s)
    }
    /// `Native_70e2(worker, args, stack)`.
    Native_70e2 @ 0x007a_32a0 => extern "C" fn n_7a32a0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70e2(&mut mem(), w, a, s)
    }
    /// `Native_7047(worker, args, stack)`.
    Native_7047 @ 0x007a_5270 => extern "C" fn n_7a5270(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7047(&mut mem(), w, a, s)
    }
    /// `Native_7093(worker, args, stack)`.
    Native_7093 @ 0x007a_60f0 => extern "C" fn n_7a60f0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7093(&mut mem(), w, a, s)
    }
    /// `Native_708f(worker, args, stack)`.
    Native_708f @ 0x007a_6700 => extern "C" fn n_7a6700(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_708f(&mut mem(), w, a, s)
    }
    /// `Native_708c(worker, args, stack)`.
    Native_708c @ 0x007a_3920 => extern "C" fn n_7a3920(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_708c(&mut mem(), w, a, s)
    }
    /// `Native_70b5(worker, args, stack)`.
    Native_70b5 @ 0x007a_2f40 => extern "C" fn n_7a2f40(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70b5(&mut mem(), w, a, s)
    }
    /// `Native_7031(worker, args, stack)`.
    Native_7031 @ 0x007a_2d00 => extern "C" fn n_7a2d00(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7031(&mut mem(), w, a, s)
    }
    /// `Native_7030(worker, args, stack)`.
    Native_7030 @ 0x007a_5090 => extern "C" fn n_7a5090(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7030(&mut mem(), w, a, s)
    }
    /// `Native_702a(worker, args, stack)`.
    Native_702a @ 0x007a_5bc0 => extern "C" fn n_7a5bc0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_702a(&mut mem(), w, a, s)
    }
    /// `Native_7041(worker, args, stack)`.
    Native_7041 @ 0x007a_56d0 => extern "C" fn n_7a56d0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7041(&mut mem(), w, a, s)
    }
    /// `Native_707f(worker, args, stack)`.
    Native_707f @ 0x007a_5e50 => extern "C" fn n_7a5e50(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_707f(&mut mem(), w, a, s)
    }
    /// `Native_70d5(worker, args, stack)`.
    Native_70d5 @ 0x007a_7fa0 => extern "C" fn n_7a7fa0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70d5(&mut mem(), w, a, s)
    }
    /// `Native_7084(worker, args, stack)`.
    Native_7084 @ 0x007a_4440 => extern "C" fn n_7a4440(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7084(&mut mem(), w, a, s)
    }
    /// `Native_7034(worker, args, stack)`.
    Native_7034 @ 0x007a_5330 => extern "C" fn n_7a5330(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7034(&mut mem(), w, a, s)
    }
    /// `Native_7051(worker, args, stack)`.
    Native_7051 @ 0x007a_5890 => extern "C" fn n_7a5890(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7051(&mut mem(), w, a, s)
    }
    /// `Native_7062(worker, args, stack)`.
    Native_7062 @ 0x007a_4580 => extern "C" fn n_7a4580(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7062(&mut mem(), w, a, s)
    }
    /// `Native_70cd(worker, args, stack)`.
    Native_70cd @ 0x007a_7d60 => extern "C" fn n_7a7d60(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70cd(&mut mem(), w, a, s)
    }
    /// `Native_70cc(worker, args, stack)`.
    Native_70cc @ 0x007a_7cb0 => extern "C" fn n_7a7cb0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70cc(&mut mem(), w, a, s)
    }
    /// `Native_70ce(worker, args, stack)`.
    Native_70ce @ 0x007a_7f20 => extern "C" fn n_7a7f20(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70ce(&mut mem(), w, a, s)
    }
    /// `Native_70a5(worker, args, stack)`.
    Native_70a5 @ 0x007a_76f0 => extern "C" fn n_7a76f0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70a5(&mut mem(), w, a, s)
    }
    /// `Native_702c(worker, args, stack)`.
    Native_702c @ 0x007a_45c0 => extern "C" fn n_7a45c0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_702c(&mut mem(), w, a, s)
    }
    /// `Native_70b0(worker, args, stack)`.
    Native_70b0 @ 0x007a_4f80 => extern "C" fn n_7a4f80(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70b0(&mut mem(), w, a, s)
    }
    /// `Native_7028(worker, args, stack)`.
    Native_7028 @ 0x007a_6970 => extern "C" fn n_7a6970(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7028(&mut mem(), w, a, s)
    }
    /// `Native_707d(worker, args, stack)`.
    Native_707d @ 0x007a_81f0 => extern "C" fn n_7a81f0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_707d(&mut mem(), w, a, s)
    }
    /// `Native_707e(worker, args, stack)`.
    Native_707e @ 0x007a_27e0 => extern "C" fn n_7a27e0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_707e(&mut mem(), w, a, s)
    }
    /// `Native_700d(worker, args, stack)`.
    Native_700d @ 0x007a_63f0 => extern "C" fn n_7a63f0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_700d(&mut mem(), w, a, s)
    }
    /// `Native_7032(worker, args, stack)`.
    Native_7032 @ 0x007a_6f70 => extern "C" fn n_7a6f70(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7032(&mut mem(), w, a, s)
    }
    /// `Native_7052(worker, args, stack)`.
    Native_7052 @ 0x007a_7e10 => extern "C" fn n_7a7e10(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7052(&mut mem(), w, a, s)
    }
    /// `AtelCall_Math_ATan2_RF(worker, args, stack)`.
    AtelCall_Math_ATan2_RF @ 0x0087_7b30 => extern "C" fn n_877b30(w: u32, a: u32, s: u32) -> f64 {
        native_chr::atel_call_math_atan2_rf(&mut mem(), w, a, s)
    }
    /// `Native_5037(worker, args, stack)`.
    Native_5037 @ 0x00a7_a140 => extern "C" fn n_a7a140(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5037(&mut mem(), w, a, s)
    }
    /// `Native_6077_ResultF(worker, args, stack)`.
    Native_6077_ResultF @ 0x007b_9190 => extern "C" fn n_7b9190(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_6077(&mut mem(), w, a, s)
    }
    /// `Native_c025(worker, args, stack)`.
    Native_c025 @ 0x0087_8c70 => extern "C" fn n_878c70(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c025(&mut mem(), w, a, s)
    }
    /// `Native_506d(worker, args, stack)`.
    Native_506d @ 0x00a7_9a80 => extern "C" fn n_a79a80(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_506d(&mut mem(), w, a, s)
    }
    /// `Native_507d(worker, args, stack)`.
    Native_507d @ 0x00a7_a090 => extern "C" fn n_a7a090(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_507d(&mut mem(), w, a, s)
    }
    /// `Native_5066(worker, args, stack)`.
    Native_5066 @ 0x00a7_9480 => extern "C" fn n_a79480(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5066(&mut mem(), w, a, s)
    }
    /// `Native_5066_Poll(worker, args, stack)`.
    Native_5066_Poll @ 0x00a7_94d0 => extern "C" fn n_a794d0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5066_poll(&mut mem(), w, a, s)
    }
    /// `Native_5084(worker, args, stack)`.
    Native_5084 @ 0x00a7_a5d0 => extern "C" fn n_a7a5d0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5084(&mut mem(), w, a, s)
    }
    /// `Native_5085(worker, args, stack)`.
    Native_5085 @ 0x00a7_a640 => extern "C" fn n_a7a640(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5085(&mut mem(), w, a, s)
    }
    /// `Native_5080(worker, args, stack)`.
    Native_5080 @ 0x00a7_a410 => extern "C" fn n_a7a410(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5080(&mut mem(), w, a, s)
    }
    /// `Native_5014(worker, args, stack)`.
    Native_5014 @ 0x00a7_8f30 => extern "C" fn n_a78f30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5014(&mut mem(), w, a, s)
    }
    /// `Native_5088(worker, args, stack)`.
    Native_5088 @ 0x00a7_a6b0 => extern "C" fn n_a7a6b0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5088(&mut mem(), w, a, s)
    }
    /// `Native_503b(worker, args, stack)`.
    Native_503b @ 0x00a7_a450 => extern "C" fn n_a7a450(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_503b(&mut mem(), w, a, s)
    }
    /// `Native_1015(worker, args, stack)`.
    Native_1015 @ 0x0087_7dc0 => extern "C" fn n_877dc0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_1015(&mut mem(), w, a, s)
    }
    /// `Native_4008(worker, args, stack)`.
    Native_4008 @ 0x00a7_7e30 => extern "C" fn n_a77e30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4008(&mut mem(), w, a, s)
    }
    /// `Native_4009(worker, args, stack)`.
    Native_4009 @ 0x00a7_7e80 => extern "C" fn n_a77e80(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4009(&mut mem(), w, a, s)
    }
    /// `Native_5007(worker, args, stack)`.
    Native_5007 @ 0x00a7_89a0 => extern "C" fn n_a789a0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5007(&mut mem(), w, a, s)
    }
    /// `Native_5049(worker, args, stack)`.
    Native_5049 @ 0x00a7_85f0 => extern "C" fn n_a785f0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5049(&mut mem(), w, a, s)
    }
    /// `Native_5049_Poll(worker, args, stack)`.
    Native_5049_Poll @ 0x00a7_8620 => extern "C" fn n_a78620(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5049_poll(&mut mem(), w, a, s)
    }
    /// `Native_501f(worker, args, stack)`.
    Native_501f @ 0x00a7_9450 => extern "C" fn n_a79450(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_501f(&mut mem(), w, a, s)
    }
    /// `Native_504a(worker, args, stack)`.
    Native_504a @ 0x00a7_8640 => extern "C" fn n_a78640(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_504a(&mut mem(), w, a, s)
    }
    /// `Native_504b(worker, args, stack)`.
    Native_504b @ 0x00a7_8670 => extern "C" fn n_a78670(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_504b(&mut mem(), w, a, s)
    }
    /// `Native_5081(worker, args, stack)`.
    Native_5081 @ 0x00a7_a4a0 => extern "C" fn n_a7a4a0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5081(&mut mem(), w, a, s)
    }
    /// `Native_5082(worker, args, stack)`.
    Native_5082 @ 0x00a7_a4f0 => extern "C" fn n_a7a4f0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5082(&mut mem(), w, a, s)
    }
    /// `Native_5073(worker, args, stack)`.
    Native_5073 @ 0x00a7_9d30 => extern "C" fn n_a79d30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5073(&mut mem(), w, a, s)
    }
    /// `Native_5083(worker, args, stack)`.
    Native_5083 @ 0x00a7_a540 => extern "C" fn n_a7a540(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5083(&mut mem(), w, a, s)
    }
    /// `Native_5060(worker, args, stack)`.
    Native_5060 @ 0x00a7_8f00 => extern "C" fn n_a78f00(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5060(&mut mem(), w, a, s)
    }
    /// `Native_508e(worker, args, stack)`.
    Native_508e @ 0x00a7_a850 => extern "C" fn n_a7a850(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_508e(&mut mem(), w, a, s)
    }
    /// `Native_5067(worker, args, stack)`.
    Native_5067 @ 0x00a7_9580 => extern "C" fn n_a79580(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5067(&mut mem(), w, a, s)
    }
    /// `Native_5071(worker, args, stack)`.
    Native_5071 @ 0x00a7_9cc0 => extern "C" fn n_a79cc0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5071(&mut mem(), w, a, s)
    }
    /// `Native_506a(worker, args, stack)`.
    Native_506a @ 0x00a7_9640 => extern "C" fn n_a79640(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_506a(&mut mem(), w, a, s)
    }
    /// `Native_508d(worker, args, stack)`.
    Native_508d @ 0x00a7_a7e0 => extern "C" fn n_a7a7e0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_508d(&mut mem(), w, a, s)
    }
    /// `Native_5069(worker, args, stack)`.
    Native_5069 @ 0x00a7_95e0 => extern "C" fn n_a795e0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5069(&mut mem(), w, a, s)
    }
    /// `Native_505a(worker, args, stack)`.
    Native_505a @ 0x00a7_8c60 => extern "C" fn n_a78c60(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_505a(&mut mem(), w, a, s)
    }
    /// `Native_506e(worker, args, stack)`.
    Native_506e @ 0x00a7_9b30 => extern "C" fn n_a79b30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_506e(&mut mem(), w, a, s)
    }
    /// `Native_5086(worker, args, stack)`.
    Native_5086 @ 0x00a7_a680 => extern "C" fn n_a7a680(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5086(&mut mem(), w, a, s)
    }
    /// `Native_507c(worker, args, stack)`.
    Native_507c @ 0x00a7_a070 => extern "C" fn n_a7a070(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_507c(&mut mem(), w, a, s)
    }
    /// `Native_5072(worker, args, stack)`.
    Native_5072 @ 0x00a7_9cf0 => extern "C" fn n_a79cf0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5072(&mut mem(), w, a, s)
    }
    /// `Native_5035(worker, args, stack)`.
    Native_5035 @ 0x00a7_9fa0 => extern "C" fn n_a79fa0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5035(&mut mem(), w, a, s)
    }
    /// `Native_5090(worker, args, stack)`.
    Native_5090 @ 0x00a7_a8d0 => extern "C" fn n_a7a8d0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5090(&mut mem(), w, a, s)
    }
    /// `Native_503f(worker, args, stack)`.
    Native_503f @ 0x00a7_a5a0 => extern "C" fn n_a7a5a0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_503f(&mut mem(), w, a, s)
    }
    /// `Native_5048(worker, args, stack)`.
    Native_5048 @ 0x00a7_a940 => extern "C" fn n_a7a940(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5048(&mut mem(), w, a, s)
    }
    /// `Native_507b(worker, args, stack)`.
    Native_507b @ 0x00a7_a000 => extern "C" fn n_a7a000(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_507b(&mut mem(), w, a, s)
    }
    /// `Native_502c(worker, args, stack)`.
    Native_502c @ 0x00a7_9bc0 => extern "C" fn n_a79bc0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_502c(&mut mem(), w, a, s)
    }
    /// `Native_5052(worker, args, stack)`.
    Native_5052 @ 0x00a7_87e0 => extern "C" fn n_a787e0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5052(&mut mem(), w, a, s)
    }
    /// `Native_5026(worker, args, stack)`.
    Native_5026 @ 0x00a7_96e0 => extern "C" fn n_a796e0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5026(&mut mem(), w, a, s)
    }
    /// `Native_5036(worker, args, stack)`.
    Native_5036 @ 0x00a7_a120 => extern "C" fn n_a7a120(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5036(&mut mem(), w, a, s)
    }
    /// `Native_5046(worker, args, stack)`.
    Native_5046 @ 0x00a7_a880 => extern "C" fn n_a7a880(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5046(&mut mem(), w, a, s)
    }
    /// `Native_5089(worker, args, stack)`.
    Native_5089 @ 0x00a7_a700 => extern "C" fn n_a7a700(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5089(&mut mem(), w, a, s)
    }
    /// `Native_508b(worker, args, stack)`.
    Native_508b @ 0x00a7_a760 => extern "C" fn n_a7a760(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_508b(&mut mem(), w, a, s)
    }
    /// `Native_5019(worker, args, stack)`.
    Native_5019 @ 0x00a7_9220 => extern "C" fn n_a79220(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5019(&mut mem(), w, a, s)
    }
    /// `Native_500c(worker, args, stack)`.
    Native_500c @ 0x00a7_8d30 => extern "C" fn n_a78d30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_500c(&mut mem(), w, a, s)
    }
    /// `Native_400e(worker, args, stack)`.
    Native_400e @ 0x00a7_7f00 => extern "C" fn n_a77f00(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_400e(&mut mem(), w, a, s)
    }
    /// `Native_400f(worker, args, stack)`.
    Native_400f @ 0x00a7_7f20 => extern "C" fn n_a77f20(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_400f(&mut mem(), w, a, s)
    }
    /// `Native_4045(worker, args, stack)`.
    Native_4045 @ 0x00a7_7ce0 => extern "C" fn n_a77ce0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4045(&mut mem(), w, a, s)
    }
    /// `Native_401b(worker, args, stack)`.
    Native_401b @ 0x00a7_81d0 => extern "C" fn n_a781d0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_401b(&mut mem(), w, a, s)
    }
    /// `Native_401c(worker, args, stack)`.
    Native_401c @ 0x00a7_81f0 => extern "C" fn n_a781f0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_401c(&mut mem(), w, a, s)
    }
    /// `Native_4023(worker, args, stack)`.
    Native_4023 @ 0x00a7_8340 => extern "C" fn n_a78340(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4023(&mut mem(), w, a, s)
    }
    /// `Native_403a(worker, args, stack)`.
    Native_403a @ 0x00a7_7b10 => extern "C" fn n_a77b10(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_403a(&mut mem(), w, a, s)
    }
    /// `Native_4046(worker, args, stack)`.
    Native_4046 @ 0x00a7_7d00 => extern "C" fn n_a77d00(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4046(&mut mem(), w, a, s)
    }
    /// `Native_c003(worker, args, stack)`.
    Native_c003 @ 0x0087_8510 => extern "C" fn n_878510(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c003(&mut mem(), w, a, s)
    }
    /// `Native_c031(worker, args, stack)`.
    Native_c031 @ 0x0087_8530 => extern "C" fn n_878530(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c031(&mut mem(), w, a, s)
    }
    /// `Native_4022(worker, args, stack)`.
    Native_4022 @ 0x00a7_8320 => extern "C" fn n_a78320(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4022(&mut mem(), w, a, s)
    }
    /// `Native_4043(worker, args, stack)`.
    Native_4043 @ 0x00a7_7c80 => extern "C" fn n_a77c80(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4043(&mut mem(), w, a, s)
    }
    /// `Native_402a(worker, args, stack)`.
    Native_402a @ 0x00a7_85b0 => extern "C" fn n_a785b0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_402a(&mut mem(), w, a, s)
    }
    /// `Native_4036(worker, args, stack)`.
    Native_4036 @ 0x00a7_7a10 => extern "C" fn n_a77a10(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4036(&mut mem(), w, a, s)
    }
    /// `Native_403e(worker, args, stack)`.
    Native_403e @ 0x00a7_7be0 => extern "C" fn n_a77be0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_403e(&mut mem(), w, a, s)
    }
    /// `Native_403c(worker, args, stack)`.
    Native_403c @ 0x00a7_7b60 => extern "C" fn n_a77b60(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_403c(&mut mem(), w, a, s)
    }
    /// `Native_4025(worker, args, stack)`.
    Native_4025 @ 0x00a7_8370 => extern "C" fn n_a78370(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4025(&mut mem(), w, a, s)
    }
    /// `Native_4044(worker, args, stack)`.
    Native_4044 @ 0x00a7_7cb0 => extern "C" fn n_a77cb0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4044(&mut mem(), w, a, s)
    }
    /// `Native_c053(worker, args, stack)`.
    Native_c053 @ 0x0087_93f0 => extern "C" fn n_8793f0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c053(&mut mem(), w, a, s)
    }
    /// `Native_c058(worker, args, stack)`.
    Native_c058 @ 0x0087_8280 => extern "C" fn n_878280(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c058(&mut mem(), w, a, s)
    }
    /// `Native_c02f(worker, args, stack)`.
    Native_c02f @ 0x0087_8e80 => extern "C" fn n_878e80(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c02f(&mut mem(), w, a, s)
    }
    /// `Native_c009(worker, args, stack)`.
    Native_c009 @ 0x0087_8630 => extern "C" fn n_878630(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c009(&mut mem(), w, a, s)
    }
    /// `Native_c007(worker, args, stack)`.
    Native_c007 @ 0x0087_8600 => extern "C" fn n_878600(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c007(&mut mem(), w, a, s)
    }
    /// `Native_c051(worker, args, stack)`.
    Native_c051 @ 0x0087_93b0 => extern "C" fn n_8793b0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c051(&mut mem(), w, a, s)
    }
    /// `Native_c036(worker, args, stack)`.
    Native_c036 @ 0x0087_8f70 => extern "C" fn n_878f70(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c036(&mut mem(), w, a, s)
    }
    /// `Native_c00d(worker, args, stack)`.
    Native_c00d @ 0x0087_86d0 => extern "C" fn n_8786d0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c00d(&mut mem(), w, a, s)
    }
    /// `Native_c002(worker, args, stack)`.
    Native_c002 @ 0x0087_84e0 => extern "C" fn n_8784e0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c002(&mut mem(), w, a, s)
    }
    /// `Native_c054(worker, args, stack)`.
    Native_c054 @ 0x0087_81b0 => extern "C" fn n_8781b0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c054(&mut mem(), w, a, s)
    }
    /// `Native_c055(worker, args, stack)`.
    Native_c055 @ 0x0087_81d0 => extern "C" fn n_8781d0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c055(&mut mem(), w, a, s)
    }
    /// `Native_101c(worker, args, stack)`.
    Native_101c @ 0x0087_80f0 => extern "C" fn n_8780f0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_101c(&mut mem(), w, a, s)
    }
    /// `Native_c024(worker, args, stack)`.
    Native_c024 @ 0x0087_8c10 => extern "C" fn n_878c10(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c024(&mut mem(), w, a, s)
    }
    /// `Native_b00a(worker, args, stack)`.
    Native_b00a @ 0x0076_eb80 => extern "C" fn n_76eb80(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_b00a(&mut mem(), w, a, s)
    }
    /// `Native_b00b_Result(worker, args, stack)`.
    Native_b00b_Result @ 0x0076_ec40 => extern "C" fn n_76ec40(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_b00b_result(&mut mem(), w, a, s)
    }
    /// `Native_601e(worker, args, stack)`.
    Native_601e @ 0x007b_83d0 => extern "C" fn n_7b83d0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_601e(&mut mem(), w, a, s)
    }
    /// `Native_6042(worker, args, stack)`.
    Native_6042 @ 0x007b_85f0 => extern "C" fn n_7b85f0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6042(&mut mem(), w, a, s)
    }
    /// `Native_CamSetDepthMaskMode(worker, args, stack)`.
    Native_CamSetDepthMaskMode @ 0x007b_88b0 => extern "C" fn n_7b88b0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_cam_set_depth_mask_mode(&mut mem(), w, a, s)
    }
    /// `Native_CamSetDepthMask(worker, args, stack)`.
    Native_CamSetDepthMask @ 0x007b_8920 => extern "C" fn n_7b8920(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_cam_set_depth_mask(&mut mem(), w, a, s)
    }
    /// `Native_6013(worker, args, stack)`.
    Native_6013 @ 0x007b_9870 => extern "C" fn n_7b9870(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6013(&mut mem(), w, a, s)
    }
    /// `Native_601b(worker, args, stack)`.
    Native_601b @ 0x007b_9ca0 => extern "C" fn n_7b9ca0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_601b(&mut mem(), w, a, s)
    }
    /// `Native_6061(worker, args, stack)`.
    Native_6061 @ 0x007b_9960 => extern "C" fn n_7b9960(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6061(&mut mem(), w, a, s)
    }
    /// `Native_604e(worker, args, stack)`.
    Native_604e @ 0x007b_8ad0 => extern "C" fn n_7b8ad0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_604e(&mut mem(), w, a, s)
    }
    /// `Native_6050(worker, args, stack)`.
    Native_6050 @ 0x007b_8ba0 => extern "C" fn n_7b8ba0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6050(&mut mem(), w, a, s)
    }
    /// `Native_6051(worker, args, stack)`.
    Native_6051 @ 0x007b_8c30 => extern "C" fn n_7b8c30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6051(&mut mem(), w, a, s)
    }
    /// `Native_6052(worker, args, stack)`.
    Native_6052 @ 0x007b_8c00 => extern "C" fn n_7b8c00(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6052(&mut mem(), w, a, s)
    }
    /// `Native_6055(worker, args, stack)`.
    Native_6055 @ 0x007b_8cc0 => extern "C" fn n_7b8cc0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6055(&mut mem(), w, a, s)
    }
    /// `Native_6058(worker, args, stack)`.
    Native_6058 @ 0x007b_8d70 => extern "C" fn n_7b8d70(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6058(&mut mem(), w, a, s)
    }
    /// `Native_604f(worker, args, stack)`.
    Native_604f @ 0x007b_8b80 => extern "C" fn n_7b8b80(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_604f(&mut mem(), w, a, s)
    }
    /// `Native_607b(worker, args, stack)`.
    Native_607b @ 0x007b_8e40 => extern "C" fn n_7b8e40(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_607b(&mut mem(), w, a, s)
    }
    /// `Native_607b_Poll(worker, args, stack)`.
    Native_607b_Poll @ 0x007b_8e60 => extern "C" fn n_7b8e60(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_607b_poll(&mut mem(), w, a, s)
    }
    /// `Native_6069(worker, args, stack)`.
    Native_6069 @ 0x007b_8ff0 => extern "C" fn n_7b8ff0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6069(&mut mem(), w, a, s)
    }
    /// `Native_6067(worker, args, stack)`.
    Native_6067 @ 0x007b_8fc0 => extern "C" fn n_7b8fc0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6067(&mut mem(), w, a, s)
    }
    /// `Native_6081(worker, args, stack)`.
    Native_6081 @ 0x007b_92c0 => extern "C" fn n_7b92c0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6081(&mut mem(), w, a, s)
    }
    /// `Native_6082(worker, args, stack)`.
    Native_6082 @ 0x007b_9300 => extern "C" fn n_7b9300(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6082(&mut mem(), w, a, s)
    }
    /// `Native_6066(worker, args, stack)`.
    Native_6066 @ 0x007b_8fa0 => extern "C" fn n_7b8fa0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6066(&mut mem(), w, a, s)
    }
    /// `Native_606c(worker, args, stack)`.
    Native_606c @ 0x007b_9150 => extern "C" fn n_7b9150(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_606c(&mut mem(), w, a, s)
    }
    /// `Native_6033(worker, args, stack)`.
    Native_6033 @ 0x007b_8150 => extern "C" fn n_7b8150(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6033(&mut mem(), w, a, s)
    }
    /// `Native_6074(worker, args, stack)`.
    Native_6074 @ 0x007b_9610 => extern "C" fn n_7b9610(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6074(&mut mem(), w, a, s)
    }
    /// `Native_606e(worker, args, stack)`.
    Native_606e @ 0x007b_94e0 => extern "C" fn n_7b94e0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_606e(&mut mem(), w, a, s)
    }
    /// `Native_605e(worker, args, stack)`.
    Native_605e @ 0x007b_9420 => extern "C" fn n_7b9420(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_605e(&mut mem(), w, a, s)
    }
    /// `Native_600a(worker, args, stack)`.
    Native_600a @ 0x007b_9440 => extern "C" fn n_7b9440(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_600a(&mut mem(), w, a, s)
    }
    /// `Native_602a(worker, args, stack)`.
    Native_602a @ 0x007b_a1f0 => extern "C" fn n_7ba1f0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_602a(&mut mem(), w, a, s)
    }
    /// `Native_604c(worker, args, stack)`.
    Native_604c @ 0x007b_8790 => extern "C" fn n_7b8790(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_604c(&mut mem(), w, a, s)
    }
    /// `AtelCall_Math_Abs_RF(worker, args, stack)`.
    AtelCall_Math_Abs_RF @ 0x0087_7fd0 => extern "C" fn n_877fd0(w: u32, a: u32, s: u32) -> f64 {
        native_chr::atel_call_math_abs_rf(&mut mem(), w, a, s)
    }
    /// `Native_5028(worker, args, stack)`.
    Native_5028 @ 0x00a7_99d0 => extern "C" fn n_a799d0(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_5028(&mut mem(), w, a, s)
    }
    /// `Native_505e(worker, args, stack)`.
    Native_505e @ 0x00a7_8e20 => extern "C" fn n_a78e20(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_505e(&mut mem(), w, a, s)
    }
    /// `Native_4034(worker, args, stack)`.
    Native_4034 @ 0x00a7_79e0 => extern "C" fn n_a779e0(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_4034(&mut mem(), w, a, s)
    }
    /// `Native_4039(worker, args, stack)`.
    Native_4039 @ 0x00a7_7ac0 => extern "C" fn n_a77ac0(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_4039(&mut mem(), w, a, s)
    }
    /// `Native_504d(worker, args, stack)`.
    Native_504d @ 0x00a7_86e0 => extern "C" fn n_a786e0(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_504d(&mut mem(), w, a, s)
    }
    /// `Native_100a(worker, args, stack)`.
    Native_100a @ 0x0087_78b0 => extern "C" fn n_8778b0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_pop_one(&mut mem(), w, a, s)
    }
    /// `Native_c02c(worker, args, stack)`.
    Native_c02c @ 0x0087_8e60 => extern "C" fn n_878e60(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_pop_one(&mut mem(), w, a, s)
    }
    /// `Native_ChrLoadSystemMgrp(worker, args, stack)`.
    Native_ChrLoadSystemMgrp @ 0x00a7_9820 => extern "C" fn n_a79820(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_chr_load_system_mgrp(&mut mem(), w, a, s)
    }
    /// `Native_ChrLoadSystemMgrp_Result(worker, args, stack)`.
    Native_ChrLoadSystemMgrp_Result @ 0x00a7_9980 => extern "C" fn n_a79980(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_chr_load_system_mgrp_result(&mut mem(), w, a, s)
    }
    /// `Native_ChrAttach(worker, args, stack)`.
    Native_ChrAttach @ 0x00a7_9c50 => extern "C" fn n_a79c50(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_chr_attach(&mut mem(), w, a, s)
    }
    /// `Native_504f(worker, args, stack)`.
    Native_504f @ 0x00a7_9340 => extern "C" fn n_a79340(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_504f(&mut mem(), w, a, s)
    }
    /// `Native_504f_Poll(worker, args, stack)`.
    Native_504f_Poll @ 0x00a7_9380 => extern "C" fn n_a79380(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_504f_poll(&mut mem(), w, a, s)
    }
    /// `Native_5061(worker, args, stack)`.
    Native_5061 @ 0x00a7_8f80 => extern "C" fn n_a78f80(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5061(&mut mem(), w, a, s)
    }
    /// `Native_401a(worker, args, stack)`.
    Native_401a @ 0x00a7_81a0 => extern "C" fn n_a781a0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_401a(&mut mem(), w, a, s)
    }
    /// `Native_4017(worker, args, stack)`.
    Native_4017 @ 0x00a7_8130 => extern "C" fn n_a78130(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4017(&mut mem(), w, a, s)
    }
    /// `Native_5076(worker, args, stack)`.
    Native_5076 @ 0x00a7_9e30 => extern "C" fn n_a79e30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5076(&mut mem(), w, a, s)
    }
    /// `Native_1000(worker, args, stack)`.
    Native_1000 @ 0x0087_79c0 => extern "C" fn n_8779c0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_1000(&mut mem(), w, a, s)
    }
    /// `Native_ChrLoadMotionGroup(worker, args, stack)`.
    Native_ChrLoadMotionGroup @ 0x00a7_99b0 => extern "C" fn n_a799b0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_chr_load_motion_group(&mut mem(), w, a, s)
    }
    /// `Native_ChrLoadMotionGroup_Result(worker, args, stack)`.
    Native_ChrLoadMotionGroup_Result @ 0x00a7_9a20 => extern "C" fn n_a79a20(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_chr_load_motion_group_result(&mut mem(), w, a, s)
    }
    /// `Native_ChrSetHitRadiusScale(worker, args, stack)`.
    Native_ChrSetHitRadiusScale @ 0x00a7_a340 => extern "C" fn n_a7a340(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_chr_set_hit_radius_scale(&mut mem(), w, a, s)
    }
    /// `Native_5050(worker, args, stack)`.
    Native_5050 @ 0x00a7_8710 => extern "C" fn n_a78710(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5050(&mut mem(), w, a, s)
    }
    /// `Native_5015(worker, args, stack)`.
    Native_5015 @ 0x00a7_8fb0 => extern "C" fn n_a78fb0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5015(&mut mem(), w, a, s)
    }
    /// `Native_503d(worker, args, stack)`.
    Native_503d @ 0x00a7_a520 => extern "C" fn n_a7a520(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_503d(&mut mem(), w, a, s)
    }
    /// `Native_101a(worker, args, stack)`.
    Native_101a @ 0x0087_7ff0 => extern "C" fn n_877ff0(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_101a(&mut mem(), w, a, s)
    }
    /// `Native_5006(worker, args, stack)`.
    Native_5006 @ 0x00a7_8a40 => extern "C" fn n_a78a40(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5006(&mut mem(), w, a, s)
    }
    /// `Native_4019(worker, args, stack)`.
    Native_4019 @ 0x00a7_8180 => extern "C" fn n_a78180(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4019(&mut mem(), w, a, s)
    }
    /// `Native_5013(worker, args, stack)`.
    Native_5013 @ 0x00a7_8ec0 => extern "C" fn n_a78ec0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5013(&mut mem(), w, a, s)
    }
    /// `Native_SetControlAngle(worker, args, stack)`.
    Native_SetControlAngle @ 0x00a7_7c30 => extern "C" fn n_a77c30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_set_control_angle(&mut mem(), w, a, s)
    }
    /// `Native_5056(worker, args, stack)`.
    Native_5056 @ 0x00a7_8a80 => extern "C" fn n_a78a80(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5056(&mut mem(), w, a, s)
    }
    /// `Native_4015(worker, args, stack)`.
    Native_4015 @ 0x00a7_80c0 => extern "C" fn n_a780c0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4015(&mut mem(), w, a, s)
    }
    /// `Native_503e(worker, args, stack)`.
    Native_503e @ 0x00a7_a570 => extern "C" fn n_a7a570(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_503e(&mut mem(), w, a, s)
    }
    /// `Native_101b(worker, args, stack)`.
    Native_101b @ 0x0087_8060 => extern "C" fn n_878060(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_101b(&mut mem(), w, a, s)
    }
    /// `Native_5054(worker, args, stack)`.
    Native_5054 @ 0x00a7_8960 => extern "C" fn n_a78960(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5054(&mut mem(), w, a, s)
    }
    /// `Native_5063(worker, args, stack)`.
    Native_5063 @ 0x00a7_9160 => extern "C" fn n_a79160(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5063(&mut mem(), w, a, s)
    }
    /// `Native_5063_Poll(worker, args, stack)`.
    Native_5063_Poll @ 0x00a7_91a0 => extern "C" fn n_a791a0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5063_poll(&mut mem(), w, a, s)
    }
    /// `Native_4014(worker, args, stack)`.
    Native_4014 @ 0x00a7_80a0 => extern "C" fn n_a780a0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4014(&mut mem(), w, a, s)
    }
    /// `Native_5079(worker, args, stack)`.
    Native_5079 @ 0x00a7_9f70 => extern "C" fn n_a79f70(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5079(&mut mem(), w, a, s)
    }
    /// `Native_5047(worker, args, stack)`.
    Native_5047 @ 0x00a7_a900 => extern "C" fn n_a7a900(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5047(&mut mem(), w, a, s)
    }
    /// `Native_507a(worker, args, stack)`.
    Native_507a @ 0x00a7_9fc0 => extern "C" fn n_a79fc0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_507a(&mut mem(), w, a, s)
    }
    /// `Native_5078(worker, args, stack)`.
    Native_5078 @ 0x00a7_9ef0 => extern "C" fn n_a79ef0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5078(&mut mem(), w, a, s)
    }
    /// `Native_4016(worker, args, stack)`.
    Native_4016 @ 0x00a7_80f0 => extern "C" fn n_a780f0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4016(&mut mem(), w, a, s)
    }
    /// `Native_500b(worker, args, stack)`.
    Native_500b @ 0x00a7_8b90 => extern "C" fn n_a78b90(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_500b(&mut mem(), w, a, s)
    }
    /// `Native_506f(worker, args, stack)`.
    Native_506f @ 0x00a7_9b90 => extern "C" fn n_a79b90(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_506f(&mut mem(), w, a, s)
    }
    /// `Native_5058(worker, args, stack)`.
    Native_5058 @ 0x00a7_8b70 => extern "C" fn n_a78b70(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5058(&mut mem(), w, a, s)
    }
    /// `Native_503a(worker, args, stack)`.
    Native_503a @ 0x00a7_a3d0 => extern "C" fn n_a7a3d0(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_503a(&mut mem(), w, a, s)
    }
    /// `Native_1013(worker, args, stack)`.
    Native_1013 @ 0x0087_7d30 => extern "C" fn n_877d30(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_1013(&mut mem(), w, a, s)
    }
    /// `Atel_Sg_Flash(worker, args, stack)`.
    Atel_Sg_Flash @ 0x00a7_7d30 => extern "C" fn n_a77d30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::atel_sg_flash(&mut mem(), w, a, s)
    }
    /// `Native_5044(worker, args, stack)`.
    Native_5044 @ 0x00a7_a810 => extern "C" fn n_a7a810(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5044(&mut mem(), w, a, s)
    }
    /// `Native_504c(worker, args, stack)`.
    Native_504c @ 0x00a7_86a0 => extern "C" fn n_a786a0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_504c(&mut mem(), w, a, s)
    }
    /// `Native_5018(worker, args, stack)`.
    Native_5018 @ 0x00a7_91e0 => extern "C" fn n_a791e0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5018(&mut mem(), w, a, s)
    }
    /// `AtelCall_Math_Cos_RF(worker, args, stack)`.
    AtelCall_Math_Cos_RF @ 0x0087_7aa0 => extern "C" fn n_877aa0(w: u32, a: u32, s: u32) -> f64 {
        native_chr::atel_call_math_cos_rf(&mut mem(), w, a, s)
    }
    /// `Native_4007(worker, args, stack)`.
    Native_4007 @ 0x00a7_7de0 => extern "C" fn n_a77de0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4007(&mut mem(), w, a, s)
    }
    /// `AtelCall_Math_Sin_RF(worker, args, stack)`.
    AtelCall_Math_Sin_RF @ 0x0087_7a70 => extern "C" fn n_877a70(w: u32, a: u32, s: u32) -> f64 {
        native_chr::atel_call_math_sin_rf(&mut mem(), w, a, s)
    }
    /// `Native_b000(worker, args, stack)`.
    Native_b000 @ 0x0076_e910 => extern "C" fn n_76e910(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_b000(&mut mem(), w, a, s)
    }
    /// `Native_b000_Result(worker, args, stack)`.
    Native_b000_Result @ 0x0076_e990 => extern "C" fn n_76e990(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_b000_result(&mut mem(), w, a, s)
    }
    /// `Native_b001_Result(worker, args, stack)`.
    Native_b001_Result @ 0x0076_e9f0 => extern "C" fn n_76e9f0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_b001_result(&mut mem(), w, a, s)
    }
    /// `Native_4006(worker, args, stack)`.
    Native_4006 @ 0x00a7_7dc0 => extern "C" fn n_a77dc0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4006(&mut mem(), w, a, s)
    }
    /// `Native_4001(worker, args, stack)`.
    Native_4001 @ 0x00a7_7b90 => extern "C" fn n_a77b90(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4001(&mut mem(), w, a, s)
    }
    /// `Native_505f(worker, args, stack)`.
    Native_505f @ 0x00a7_8e90 => extern "C" fn n_a78e90(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_505f(&mut mem(), w, a, s)
    }
    /// `Native_500d(worker, args, stack)`.
    Native_500d @ 0x00a7_8df0 => extern "C" fn n_a78df0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_500d(&mut mem(), w, a, s)
    }
    /// `Native_503c(worker, args, stack)`.
    Native_503c @ 0x00a7_a4d0 => extern "C" fn n_a7a4d0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_503c(&mut mem(), w, a, s)
    }
    /// `Native_508f(worker, args, stack)`.
    Native_508f @ 0x00a7_a8a0 => extern "C" fn n_a7a8a0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_508f(&mut mem(), w, a, s)
    }
    /// `Native_FootSetMaterial(worker, args, stack)`.
    Native_FootSetMaterial @ 0x00a7_7b30 => extern "C" fn n_a77b30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_foot_set_material(&mut mem(), w, a, s)
    }
    /// `Native_5065(worker, args, stack)`.
    Native_5065 @ 0x00a7_9420 => extern "C" fn n_a79420(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5065(&mut mem(), w, a, s)
    }
    /// `Native_706e(worker, args, stack)`.
    Native_706e @ 0x007a_7020 => extern "C" fn n_7a7020(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_706e(&mut mem(), w, a, s)
    }
    /// `Native_706e_Poll(worker, args, stack)`.
    Native_706e_Poll @ 0x007a_71f0 => extern "C" fn n_7a71f0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_btl_run_event_poll(&mut mem(), w, a, s)
    }
    /// `Native_708e(worker, args, stack)`.
    Native_708e @ 0x007a_5c20 => extern "C" fn n_7a5c20(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_708e(&mut mem(), w, a, s)
    }
    /// `Native_7107(worker, args, stack)`.
    Native_7107 @ 0x007a_6800 => extern "C" fn n_7a6800(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7107(&mut mem(), w, a, s)
    }
    /// `Native_7057(worker, args, stack)`.
    Native_7057 @ 0x007a_4220 => extern "C" fn n_7a4220(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7057(&mut mem(), w, a, s)
    }
    /// `Native_7083(worker, args, stack)`.
    Native_7083 @ 0x007a_42a0 => extern "C" fn n_7a42a0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7083(&mut mem(), w, a, s)
    }
    /// `Native_7117(worker, args, stack)`.
    Native_7117 @ 0x007a_4b00 => extern "C" fn n_7a4b00(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7117(&mut mem(), w, a, s)
    }
    /// `Native_7078(worker, args, stack)`.
    Native_7078 @ 0x007a_6540 => extern "C" fn n_7a6540(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7078(&mut mem(), w, a, s)
    }
    /// `Native_7040(worker, args, stack)`.
    Native_7040 @ 0x007a_5610 => extern "C" fn n_7a5610(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7040(&mut mem(), w, a, s)
    }
    /// `Native_7046(worker, args, stack)`.
    Native_7046 @ 0x007a_5220 => extern "C" fn n_7a5220(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7046(&mut mem(), w, a, s)
    }
    /// `Native_7049(worker, args, stack)`.
    Native_7049 @ 0x007a_55a0 => extern "C" fn n_7a55a0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7049(&mut mem(), w, a, s)
    }
    /// `Native_70dc(worker, args, stack)`.
    Native_70dc @ 0x007a_7a10 => extern "C" fn n_7a7a10(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70dc(&mut mem(), w, a, s)
    }
    /// `Native_7091(worker, args, stack)`.
    Native_7091 @ 0x007a_77c0 => extern "C" fn n_7a77c0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7091(&mut mem(), w, a, s)
    }
    /// `CT_7106_ret_doesChrKnowCommand(worker, args, stack)`.
    CT_7106_ret_doesChrKnowCommand @ 0x007a_30a0 => extern "C" fn n_7a30a0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7106(&mut mem(), w, a, s)
    }
    /// `Native_7087(worker, args, stack)`.
    Native_7087 @ 0x007a_7880 => extern "C" fn n_7a7880(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7087(&mut mem(), w, a, s)
    }
    /// `Native_7088(worker, args, stack)`.
    Native_7088 @ 0x007a_79c0 => extern "C" fn n_7a79c0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7088(&mut mem(), w, a, s)
    }
    /// `Native_70ed(worker, args, stack)`.
    Native_70ed @ 0x007a_2ca0 => extern "C" fn n_7a2ca0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70ed(&mut mem(), w, a, s)
    }
    /// `Native_7050(worker, args, stack)`.
    Native_7050 @ 0x007a_7400 => extern "C" fn n_7a7400(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7050(&mut mem(), w, a, s)
    }
    /// `Native_7050_Poll(worker, args, stack)`.
    Native_7050_Poll @ 0x007a_74f0 => extern "C" fn n_7a74f0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_btl_run_event_poll(&mut mem(), w, a, s)
    }
    /// `Native_7109(worker, args, stack)`.
    Native_7109 @ 0x007a_3df0 => extern "C" fn n_7a3df0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7109(&mut mem(), w, a, s)
    }
    /// `Native_70d1(worker, args, stack)`.
    Native_70d1 @ 0x007a_5cb0 => extern "C" fn n_7a5cb0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70d1(&mut mem(), w, a, s)
    }
    /// `Native_701b(worker, args, stack)`.
    Native_701b @ 0x007a_66a0 => extern "C" fn n_7a66a0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_701b(&mut mem(), w, a, s)
    }
    /// `Native_7036(worker, args, stack)`.
    Native_7036 @ 0x007a_7180 => extern "C" fn n_7a7180(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7036(&mut mem(), w, a, s)
    }
    /// `Native_702f(worker, args, stack)`.
    Native_702f @ 0x007a_5140 => extern "C" fn n_7a5140(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_702f(&mut mem(), w, a, s)
    }
    /// `Native_704b(worker, args, stack)`.
    Native_704b @ 0x007a_7640 => extern "C" fn n_7a7640(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_704b(&mut mem(), w, a, s)
    }
    /// `Native_704b_Poll(worker, args, stack)`.
    Native_704b_Poll @ 0x007a_7850 => extern "C" fn n_7a7850(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_btl_run_event_poll(&mut mem(), w, a, s)
    }
    /// `Native_7082(worker, args, stack)`.
    Native_7082 @ 0x007a_41e0 => extern "C" fn n_7a41e0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7082(&mut mem(), w, a, s)
    }
    /// `Native_709d(worker, args, stack)`.
    Native_709d @ 0x007a_3ae0 => extern "C" fn n_7a3ae0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_709d(&mut mem(), w, a, s)
    }
    /// `Native_70d7(worker, args, stack)`.
    Native_70d7 @ 0x007a_30f0 => extern "C" fn n_7a30f0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70d7(&mut mem(), w, a, s)
    }
    /// `Native_701f(worker, args, stack)`.
    Native_701f @ 0x007a_5f80 => extern "C" fn n_7a5f80(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_701f(&mut mem(), w, a, s)
    }
    /// `Native_7022(worker, args, stack)`.
    Native_7022 @ 0x007a_4cd0 => extern "C" fn n_7a4cd0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7022(&mut mem(), w, a, s)
    }
    /// `Native_7023(worker, args, stack)`.
    Native_7023 @ 0x007a_4940 => extern "C" fn n_7a4940(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7023(&mut mem(), w, a, s)
    }
    /// `Native_7054(worker, args, stack)`.
    Native_7054 @ 0x007a_7fe0 => extern "C" fn n_7a7fe0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7054(&mut mem(), w, a, s)
    }
    /// `Native_70be(worker, args, stack)`.
    Native_70be @ 0x007a_3780 => extern "C" fn n_7a3780(w: u32, a: u32, s: u32) -> f64 {
        native_btl::native_70be(&mut mem(), w, a, s)
    }
    /// `Native_70c3(worker, args, stack)`.
    Native_70c3 @ 0x007a_3650 => extern "C" fn n_7a3650(w: u32, a: u32, s: u32) -> f64 {
        native_btl::native_70c3(&mut mem(), w, a, s)
    }
    /// `Native_70ea(worker, args, stack)`.
    Native_70ea @ 0x007a_7b10 => extern "C" fn n_7a7b10(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70ea(&mut mem(), w, a, s)
    }
    /// `Native_7115(worker, args, stack)`.
    Native_7115 @ 0x007a_4560 => extern "C" fn n_7a4560(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7115(&mut mem(), w, a, s)
    }
    /// `Native_711d(worker, args, stack)`.
    Native_711d @ 0x007a_4330 => extern "C" fn n_7a4330(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_711d(&mut mem(), w, a, s)
    }
    /// `Native_7025(worker, args, stack)`.
    Native_7025 @ 0x007a_5cf0 => extern "C" fn n_7a5cf0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7025(&mut mem(), w, a, s)
    }
    /// `Native_7053(worker, args, stack)`.
    Native_7053 @ 0x007a_4470 => extern "C" fn n_7a4470(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7053(&mut mem(), w, a, s)
    }
    /// `Native_703a(worker, args, stack)`.
    Native_703a @ 0x007a_7360 => extern "C" fn n_7a7360(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_703a(&mut mem(), w, a, s)
    }
    /// `Native_7063(worker, args, stack)`.
    Native_7063 @ 0x007a_5dc0 => extern "C" fn n_7a5dc0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7063(&mut mem(), w, a, s)
    }
    /// `Native_708a(worker, args, stack)`.
    Native_708a @ 0x007a_3710 => extern "C" fn n_7a3710(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_708a(&mut mem(), w, a, s)
    }
    /// `Native_708b(worker, args, stack)`.
    Native_708b @ 0x007a_3900 => extern "C" fn n_7a3900(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_708b(&mut mem(), w, a, s)
    }
    /// `Native_70bb(worker, args, stack)`.
    Native_70bb @ 0x007a_4cf0 => extern "C" fn n_7a4cf0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70bb(&mut mem(), w, a, s)
    }
    /// `Native_70ef(worker, args, stack)`.
    Native_70ef @ 0x007a_64f0 => extern "C" fn n_7a64f0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70ef(&mut mem(), w, a, s)
    }
    /// `Native_7118(worker, args, stack)`.
    Native_7118 @ 0x007a_81d0 => extern "C" fn n_7a81d0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7118(&mut mem(), w, a, s)
    }
    /// `Native_711c(worker, args, stack)`.
    Native_711c @ 0x007a_8530 => extern "C" fn n_7a8530(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_711c(&mut mem(), w, a, s)
    }
    /// `Native_GetEntranceX(worker, args, stack)`.
    Native_GetEntranceX @ 0x0085_c7b0 => extern "C" fn n_85c7b0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_get_entrance_x(&mut mem(), w, a, s)
    }
    /// `Native_GetEntranceY(worker, args, stack)`.
    Native_GetEntranceY @ 0x0085_c8f0 => extern "C" fn n_85c8f0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_get_entrance_y(&mut mem(), w, a, s)
    }
    /// `Native_002c(worker, args, stack)`.
    Native_002c @ 0x0085_a1a0 => extern "C" fn n_85a1a0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_002c(&mut mem(), w, a, s)
    }
    /// `Native_002b(worker, args, stack)`.
    Native_002b @ 0x0085_a0b0 => extern "C" fn n_85a0b0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_002b(&mut mem(), w, a, s)
    }
    /// `Native_0021(worker, args, stack)`.
    Native_0021 @ 0x0085_9430 => extern "C" fn n_859430(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0021(&mut mem(), w, a, s)
    }
    /// `Native_015e(worker, args, stack)`.
    Native_015e @ 0x0085_7980 => extern "C" fn n_857980(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_015e(&mut mem(), w, a, s)
    }
    /// `Native_0119(worker, args, stack)`.
    Native_0119 @ 0x0085_bd70 => extern "C" fn n_85bd70(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0119(&mut mem(), w, a, s)
    }
    /// `Native_SetTriggerMode(worker, args, stack)`.
    Native_SetTriggerMode @ 0x0085_c320 => extern "C" fn n_85c320(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_set_trigger_mode(&mut mem(), w, a, s)
    }
    /// `Native_SetPlayer(worker, args, stack)`.
    Native_SetPlayer @ 0x0085_c9a0 => extern "C" fn n_85c9a0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_set_player(&mut mem(), w, a, s)
    }
    /// `Native_0091(worker, args, stack)`.
    Native_0091 @ 0x0085_e0b0 => extern "C" fn n_85e0b0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0091(&mut mem(), w, a, s)
    }
    /// `Native_0004(worker, args, stack)`.
    Native_0004 @ 0x0085_ecb0 => extern "C" fn n_85ecb0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0004(&mut mem(), w, a, s)
    }
    /// `Native_00e0(worker, args, stack)`.
    Native_00e0 @ 0x0085_5f60 => extern "C" fn n_855f60(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00e0(&mut mem(), w, a, s)
    }
    /// `Native_SetTalkAngle(worker, args, stack)`.
    Native_SetTalkAngle @ 0x0085_74f0 => extern "C" fn n_8574f0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_set_talk_angle(&mut mem(), w, a, s)
    }
    /// `Native_SetTouchRange(worker, args, stack)`.
    Native_SetTouchRange @ 0x0085_6460 => extern "C" fn n_856460(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_set_touch_range(&mut mem(), w, a, s)
    }
    /// `Native_SetTYZAngle(worker, args, stack)`.
    Native_SetTYZAngle @ 0x0085_77c0 => extern "C" fn n_8577c0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_set_tyz_angle(&mut mem(), w, a, s)
    }
    /// `Native_SetRadius44(worker, args, stack)`.
    Native_SetRadius44 @ 0x0085_72e0 => extern "C" fn n_8572e0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_set_radius44(&mut mem(), w, a, s)
    }
    /// `Native_00a7(worker, args, stack)`.
    Native_00a7 @ 0x0085_7820 => extern "C" fn n_857820(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00a7(&mut mem(), w, a, s)
    }
    /// `Native_0058(worker, args, stack)`.
    Native_0058 @ 0x0085_5f80 => extern "C" fn n_855f80(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0058(&mut mem(), w, a, s)
    }
    /// `Native_00fb(worker, args, stack)`.
    Native_00fb @ 0x0085_8ac0 => extern "C" fn n_858ac0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00fb(&mut mem(), w, a, s)
    }
    /// `Native_0180(worker, args, stack)`.
    Native_0180 @ 0x0085_c000 => extern "C" fn n_85c000(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0180(&mut mem(), w, a, s)
    }
    /// `Native_SetTurnSpeedAll(worker, args, stack)`.
    Native_SetTurnSpeedAll @ 0x0085_b030 => extern "C" fn n_85b030(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_set_turn_speed_all(&mut mem(), w, a, s)
    }
    /// `Native_SetMoveParam3C(worker, args, stack)`.
    Native_SetMoveParam3C @ 0x0085_ab80 => extern "C" fn n_85ab80(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_set_move_param3c(&mut mem(), w, a, s)
    }
    /// `Native_SetMoveParam40(worker, args, stack)`.
    Native_SetMoveParam40 @ 0x0085_acd0 => extern "C" fn n_85acd0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_set_move_param40(&mut mem(), w, a, s)
    }
    /// `Native_SetTurnParam10(worker, args, stack)`.
    Native_SetTurnParam10 @ 0x0085_af20 => extern "C" fn n_85af20(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_set_turn_param10(&mut mem(), w, a, s)
    }
    /// `Native_SetTurnParam18(worker, args, stack)`.
    Native_SetTurnParam18 @ 0x0085_b230 => extern "C" fn n_85b230(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_set_turn_param18(&mut mem(), w, a, s)
    }
    /// `Native_SetTurnParam1C(worker, args, stack)`.
    Native_SetTurnParam1C @ 0x0085_b480 => extern "C" fn n_85b480(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_set_turn_param1c(&mut mem(), w, a, s)
    }
    /// `Native_GetOwnWorkerType(worker, args, stack)`.
    Native_GetOwnWorkerType @ 0x0085_f5a0 => extern "C" fn n_85f5a0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_get_own_worker_type(&mut mem(), w, a, s)
    }
    /// `Native_00aa(worker, args, stack)`.
    Native_00aa @ 0x0085_7a60 => extern "C" fn n_857a60(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00aa(&mut mem(), w, a, s)
    }
    /// `Native_MusicPlay(worker, args, stack)`.
    Native_MusicPlay @ 0x0085_96d0 => extern "C" fn n_8596d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_music_play(&mut mem(), w, a, s)
    }
    /// `Native_0007(worker, args, stack)`.
    Native_0007 @ 0x0085_c8c0 => extern "C" fn n_85c8c0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0007(&mut mem(), w, a, s)
    }
    /// `Native_012a(worker, args, stack)`.
    Native_012a @ 0x0085_c890 => extern "C" fn n_85c890(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_012a(&mut mem(), w, a, s)
    }
    /// `Native_0133(worker, args, stack)`.
    Native_0133 @ 0x0086_0290 => extern "C" fn n_860290(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0133(&mut mem(), w, a, s)
    }
    /// `Native_016f(worker, args, stack)`.
    Native_016f @ 0x0085_c430 => extern "C" fn n_85c430(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_016f(&mut mem(), w, a, s)
    }
    /// `Native_0236(worker, args, stack)`.
    Native_0236 @ 0x0086_0350 => extern "C" fn n_860350(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0236(&mut mem(), w, a, s)
    }
    /// `Native_SetPos_0126(worker, args, stack)`.
    Native_SetPos_0126 @ 0x0085_fea0 => extern "C" fn n_85fea0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_set_pos_0126(&mut mem(), w, a, s)
    }
    /// `Native_GetTurnFrames(worker, args, stack)`.
    Native_GetTurnFrames @ 0x0085_ad00 => extern "C" fn n_85ad00(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_get_turn_frames(&mut mem(), w, a, s)
    }
    /// `Native_ChrGetAttachBone(worker, args, stack)`.
    Native_ChrGetAttachBone @ 0x0085_ce30 => extern "C" fn n_85ce30(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_chr_get_attach_bone(&mut mem(), w, a, s)
    }
    /// `Native_SetPartyIndex(worker, args, stack)`.
    Native_SetPartyIndex @ 0x0085_cb40 => extern "C" fn n_85cb40(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_set_party_index(&mut mem(), w, a, s)
    }
    /// `Native_0158(worker, args, stack)`.
    Native_0158 @ 0x0085_cfa0 => extern "C" fn n_85cfa0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0158(&mut mem(), w, a, s)
    }
    /// `Native_GetPitchToMoveTarget(worker, args, stack)`.
    Native_GetPitchToMoveTarget @ 0x0085_8d30 => extern "C" fn n_858d30(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_get_pitch_to_move_target(&mut mem(), w, a, s)
    }
    /// `Native_PartyAdd(worker, args, stack)`.
    Native_PartyAdd @ 0x0085_b700 => extern "C" fn n_85b700(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_party_add(&mut mem(), w, a, s)
    }
    /// `Native_TriggerSetLine(worker, args, stack)`.
    Native_TriggerSetLine @ 0x0085_f370 => extern "C" fn n_85f370(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_trigger_set_line(&mut mem(), w, a, s)
    }
    /// `Worker_Detach(worker, args, stack)`.
    Worker_Detach @ 0x0085_ca00 => extern "C" fn n_85ca00(w: u32, a: u32, s: u32) -> u32 {
        native_move::worker_detach(&mut mem(), w, a, s)
    }
    /// `Native_AttachCamera(worker, args, stack)`.
    Native_AttachCamera @ 0x0085_e920 => extern "C" fn n_85e920(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_attach_camera(&mut mem(), w, a, s)
    }
    /// `Native_SetPitch(worker, args, stack)`.
    Native_SetPitch @ 0x0085_f460 => extern "C" fn n_85f460(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_set_pitch(&mut mem(), w, a, s)
    }
    /// `CT_01D1_ret(worker, args, stack)`.
    CT_01D1_ret @ 0x0085_c5b0 => extern "C" fn n_85c5b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00d1(&mut mem(), w, a, s)
    }
    /// `Native_01ba(worker, args, stack)`.
    Native_01ba @ 0x0085_beb0 => extern "C" fn n_85beb0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01ba(&mut mem(), w, a, s)
    }
    /// `Native_015b_Poll(worker, args, stack)`.
    Native_015b_Poll @ 0x0085_af70 => extern "C" fn n_85af70(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_015b_poll(&mut mem(), w, a, s)
    }
    /// `Native_MusicFadeOut(worker, args, stack)`.
    Native_MusicFadeOut @ 0x0085_e290 => extern "C" fn n_85e290(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_music_fade_out(&mut mem(), w, a, s)
    }
    /// `Native_TriggerSetPoint(worker, args, stack)`.
    Native_TriggerSetPoint @ 0x0086_03e0 => extern "C" fn n_8603e0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_trigger_set_point(&mut mem(), w, a, s)
    }
    /// `Native_00f6(worker, args, stack)`.
    Native_00f6 @ 0x0085_7df0 => extern "C" fn n_857df0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00f6(&mut mem(), w, a, s)
    }
    /// `Native_00f6_Poll(worker, args, stack)`.
    Native_00f6_Poll @ 0x0085_7ff0 => extern "C" fn n_857ff0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00f6_poll(&mut mem(), w, a, s)
    }
    /// `Native_0055(worker, args, stack)`.
    Native_0055 @ 0x0085_fa20 => extern "C" fn n_85fa20(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0055(&mut mem(), w, a, s)
    }
    /// `Native_00a4(worker, args, stack)`.
    Native_00a4 @ 0x0085_66c0 => extern "C" fn n_8566c0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_00a4(&mut mem(), w, a, s)
    }
    /// `Native_00a5(worker, args, stack)`.
    Native_00a5 @ 0x0085_6990 => extern "C" fn n_856990(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_00a5(&mut mem(), w, a, s)
    }
    /// `Native_MusicLoad(worker, args, stack)`.
    Native_MusicLoad @ 0x0085_8e50 => extern "C" fn n_858e50(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_music_load(&mut mem(), w, a, s)
    }
    /// `Native_MusicLoad_Result(worker, args, stack)`.
    Native_MusicLoad_Result @ 0x0085_9250 => extern "C" fn n_859250(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_music_load_result(&mut mem(), w, a, s)
    }
    /// `Native_017a(worker, args, stack)`.
    Native_017a @ 0x0085_d210 => extern "C" fn n_85d210(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_017a(&mut mem(), w, a, s)
    }
    /// `Native_002a(worker, args, stack)`.
    Native_002a @ 0x0085_9bb0 => extern "C" fn n_859bb0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_002a(&mut mem(), w, a, s)
    }
    /// `Native_0059(worker, args, stack)`.
    Native_0059 @ 0x0085_5cd0 => extern "C" fn n_855cd0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0059(&mut mem(), w, a, s)
    }
    /// `Native_013e(worker, args, stack)`.
    Native_013e @ 0x0085_6890 => extern "C" fn n_856890(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_013e(&mut mem(), w, a, s)
    }
    /// `Native_004c(worker, args, stack)`.
    Native_004c @ 0x0085_e130 => extern "C" fn n_85e130(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_004c(&mut mem(), w, a, s)
    }
    /// `Native_004d(worker, args, stack)`.
    Native_004d @ 0x0085_e820 => extern "C" fn n_85e820(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_004d(&mut mem(), w, a, s)
    }
    /// `Native_SetMapMarkerPos(worker, args, stack)`.
    Native_SetMapMarkerPos @ 0x0085_c2d0 => extern "C" fn n_85c2d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_set_map_marker_pos(&mut mem(), w, a, s)
    }
    /// `Native_0103(worker, args, stack)`.
    Native_0103 @ 0x0085_93a0 => extern "C" fn n_8593a0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0103(&mut mem(), w, a, s)
    }
    /// `Native_016d(worker, args, stack)`.
    Native_016d @ 0x0085_95b0 => extern "C" fn n_8595b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_016d(&mut mem(), w, a, s)
    }
    /// `Native_01e9(worker, args, stack)`.
    Native_01e9 @ 0x0085_90f0 => extern "C" fn n_8590f0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01e9(&mut mem(), w, a, s)
    }
    /// `Native_01e9_Poll(worker, args, stack)`.
    Native_01e9_Poll @ 0x0085_92c0 => extern "C" fn n_8592c0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01e9_poll(&mut mem(), w, a, s)
    }
    /// `Native_MinimapSetCorner(worker, args, stack)`.
    Native_MinimapSetCorner @ 0x0085_7c40 => extern "C" fn n_857c40(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_minimap_set_corner(&mut mem(), w, a, s)
    }
    /// `Native_00a8(worker, args, stack)`.
    Native_00a8 @ 0x0085_7950 => extern "C" fn n_857950(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00a8(&mut mem(), w, a, s)
    }
    /// `Native_014a(worker, args, stack)`.
    Native_014a @ 0x0085_8740 => extern "C" fn n_858740(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_014a(&mut mem(), w, a, s)
    }
    /// `Native_0215(worker, args, stack)`.
    Native_0215 @ 0x0085_d170 => extern "C" fn n_85d170(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0215(&mut mem(), w, a, s)
    }
    /// `Native_0132(worker, args, stack)`.
    Native_0132 @ 0x0085_fe30 => extern "C" fn n_85fe30(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0132(&mut mem(), w, a, s)
    }
    /// `Native_00f2(worker, args, stack)`.
    Native_00f2 @ 0x0085_7850 => extern "C" fn n_857850(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00f2(&mut mem(), w, a, s)
    }
    /// `Native_015d(worker, args, stack)`.
    Native_015d @ 0x0085_7a80 => extern "C" fn n_857a80(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_015d(&mut mem(), w, a, s)
    }
    /// `Native_0225(worker, args, stack)`.
    Native_0225 @ 0x0085_ea10 => extern "C" fn n_85ea10(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0225(&mut mem(), w, a, s)
    }
    /// `Native_012d(worker, args, stack)`.
    Native_012d @ 0x0085_df80 => extern "C" fn n_85df80(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_012d(&mut mem(), w, a, s)
    }
    /// `Native_00ce(worker, args, stack)`.
    Native_00ce @ 0x0085_c2b0 => extern "C" fn n_85c2b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00ce(&mut mem(), w, a, s)
    }
    /// `Native_017e(worker, args, stack)`.
    Native_017e @ 0x0085_d830 => extern "C" fn n_85d830(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_017e(&mut mem(), w, a, s)
    }
    /// `Native_0025(worker, args, stack)`.
    Native_0025 @ 0x0085_9d80 => extern "C" fn n_859d80(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0025(&mut mem(), w, a, s)
    }
    /// `Native_0197(worker, args, stack)`.
    Native_0197 @ 0x0085_edd0 => extern "C" fn n_85edd0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0197(&mut mem(), w, a, s)
    }
    /// `Native_021b(worker, args, stack)`.
    Native_021b @ 0x0085_d8c0 => extern "C" fn n_85d8c0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_021b(&mut mem(), w, a, s)
    }
    /// `Native_0128(worker, args, stack)`.
    Native_0128 @ 0x0085_d7b0 => extern "C" fn n_85d7b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0128(&mut mem(), w, a, s)
    }
    /// `Native_0069(worker, args, stack)`.
    Native_0069 @ 0x0085_89d0 => extern "C" fn n_8589d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0069(&mut mem(), w, a, s)
    }
    /// `Native_0139(worker, args, stack)`.
    Native_0139 @ 0x0085_f730 => extern "C" fn n_85f730(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0139(&mut mem(), w, a, s)
    }
    /// `Native_0139_Poll(worker, args, stack)`.
    Native_0139_Poll @ 0x0085_f850 => extern "C" fn n_85f850(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0139_poll(&mut mem(), w, a, s)
    }
    /// `Native_0139_Result(worker, args, stack)`.
    Native_0139_Result @ 0x0085_f900 => extern "C" fn n_85f900(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0139_result(&mut mem(), w, a, s)
    }
    /// `Native_00b1(worker, args, stack)`.
    Native_00b1 @ 0x0085_6a50 => extern "C" fn n_856a50(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_00b1(&mut mem(), w, a, s)
    }
    /// `Native_00e9(worker, args, stack)`.
    Native_00e9 @ 0x0085_6660 => extern "C" fn n_856660(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00e9(&mut mem(), w, a, s)
    }
    /// `Native_0117(worker, args, stack)`.
    Native_0117 @ 0x0085_bc50 => extern "C" fn n_85bc50(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0117(&mut mem(), w, a, s)
    }
    /// `Native_01da(worker, args, stack)`.
    Native_01da @ 0x0085_6a80 => extern "C" fn n_856a80(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01da(&mut mem(), w, a, s)
    }
    /// `Native_0181(worker, args, stack)`.
    Native_0181 @ 0x0085_e550 => extern "C" fn n_85e550(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0181(&mut mem(), w, a, s)
    }
    /// `Native_LineExtend(worker, args, stack)`.
    Native_LineExtend @ 0x0085_c130 => extern "C" fn n_85c130(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_line_extend(&mut mem(), w, a, s)
    }
    /// `Native_015f(worker, args, stack)`.
    Native_015f @ 0x0085_b650 => extern "C" fn n_85b650(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_015f(&mut mem(), w, a, s)
    }
    /// `Native_0257(worker, args, stack)`.
    Native_0257 @ 0x0085_88b0 => extern "C" fn n_8588b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0257(&mut mem(), w, a, s)
    }
    /// `Native_0256(worker, args, stack)`.
    Native_0256 @ 0x0085_8710 => extern "C" fn n_858710(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0256(&mut mem(), w, a, s)
    }
    /// `Native_0093(worker, args, stack)`.
    Native_0093 @ 0x0085_e8f0 => extern "C" fn n_85e8f0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0093(&mut mem(), w, a, s)
    }
    /// `Native_0130(worker, args, stack)`.
    Native_0130 @ 0x0085_e410 => extern "C" fn n_85e410(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0130(&mut mem(), w, a, s)
    }
    /// `Native_017f(worker, args, stack)`.
    Native_017f @ 0x0085_bd10 => extern "C" fn n_85bd10(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_017f(&mut mem(), w, a, s)
    }
    /// `Native_0136(worker, args, stack)`.
    Native_0136 @ 0x0085_ee40 => extern "C" fn n_85ee40(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0136(&mut mem(), w, a, s)
    }
    /// `Native_0137(worker, args, stack)`.
    Native_0137 @ 0x0085_ef80 => extern "C" fn n_85ef80(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0137(&mut mem(), w, a, s)
    }
    /// `Native_0157(worker, args, stack)`.
    Native_0157 @ 0x0085_9dc0 => extern "C" fn n_859dc0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0157(&mut mem(), w, a, s)
    }
    /// `Native_0155(worker, args, stack)`.
    Native_0155 @ 0x0085_9d00 => extern "C" fn n_859d00(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0155(&mut mem(), w, a, s)
    }
    /// `Native_0100(worker, args, stack)`.
    Native_0100 @ 0x0086_0460 => extern "C" fn n_860460(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0100(&mut mem(), w, a, s)
    }
    /// `Native_016b(worker, args, stack)`.
    Native_016b @ 0x0085_be70 => extern "C" fn n_85be70(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_016b(&mut mem(), w, a, s)
    }
    /// `Native_005a(worker, args, stack)`.
    Native_005a @ 0x0085_6170 => extern "C" fn n_856170(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_005a(&mut mem(), w, a, s)
    }
    /// `Native_00be(worker, args, stack)`.
    Native_00be @ 0x0085_9950 => extern "C" fn n_859950(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00be(&mut mem(), w, a, s)
    }
    /// `Native_01a4(worker, args, stack)`.
    Native_01a4 @ 0x0085_7360 => extern "C" fn n_857360(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01a4(&mut mem(), w, a, s)
    }
    /// `Native_0121(worker, args, stack)`.
    Native_0121 @ 0x0085_d1d0 => extern "C" fn n_85d1d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0121(&mut mem(), w, a, s)
    }
    /// `Native_0122(worker, args, stack)`.
    Native_0122 @ 0x0085_d310 => extern "C" fn n_85d310(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0122(&mut mem(), w, a, s)
    }
    /// `Native_01c9(worker, args, stack)`.
    Native_01c9 @ 0x0086_0730 => extern "C" fn n_860730(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01c9(&mut mem(), w, a, s)
    }
    /// `Native_009a(worker, args, stack)`.
    Native_009a @ 0x0086_0390 => extern "C" fn n_860390(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_009a(&mut mem(), w, a, s)
    }
    /// `Native_00de_Poll(worker, args, stack)`.
    Native_00de_Poll @ 0x0085_f280 => extern "C" fn n_85f280(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00de_poll(&mut mem(), w, a, s)
    }
    /// `Native_0094(worker, args, stack)`.
    Native_0094 @ 0x0085_f0b0 => extern "C" fn n_85f0b0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0094(&mut mem(), w, a, s)
    }
    /// `Native_01eb(worker, args, stack)`.
    Native_01eb @ 0x0085_8070 => extern "C" fn n_858070(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01eb(&mut mem(), w, a, s)
    }
    /// `Native_021a(worker, args, stack)`.
    Native_021a @ 0x0085_d780 => extern "C" fn n_85d780(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_021a(&mut mem(), w, a, s)
    }
    /// `Native_0207(worker, args, stack)`.
    Native_0207 @ 0x0085_f010 => extern "C" fn n_85f010(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0207(&mut mem(), w, a, s)
    }
    /// `Native_0090(worker, args, stack)`.
    Native_0090 @ 0x0085_e270 => extern "C" fn n_85e270(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0090(&mut mem(), w, a, s)
    }
    /// `Native_019f(worker, args, stack)`.
    Native_019f @ 0x0085_67a0 => extern "C" fn n_8567a0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_019f(&mut mem(), w, a, s)
    }
    /// `Native_0196(worker, args, stack)`.
    Native_0196 @ 0x0085_fce0 => extern "C" fn n_85fce0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0196(&mut mem(), w, a, s)
    }
    /// `Native_0109(worker, args, stack)`.
    Native_0109 @ 0x0085_a870 => extern "C" fn n_85a870(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0109(&mut mem(), w, a, s)
    }
    /// `Native_0108(worker, args, stack)`.
    Native_0108 @ 0x0085_a370 => extern "C" fn n_85a370(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0108(&mut mem(), w, a, s)
    }
    /// `Native_00db(worker, args, stack)`.
    Native_00db @ 0x0085_e9e0 => extern "C" fn n_85e9e0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00db(&mut mem(), w, a, s)
    }
    /// `Native_00db_Poll(worker, args, stack)`.
    Native_00db_Poll @ 0x0085_eaf0 => extern "C" fn n_85eaf0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00db_poll(&mut mem(), w, a, s)
    }
    /// `Native_011d(worker, args, stack)`.
    Native_011d @ 0x0085_c650 => extern "C" fn n_85c650(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_011d(&mut mem(), w, a, s)
    }
    /// `Native_00dc_Poll(worker, args, stack)`.
    Native_00dc_Poll @ 0x0085_ee70 => extern "C" fn n_85ee70(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00dc_poll(&mut mem(), w, a, s)
    }
    /// `Native_00bd(worker, args, stack)`.
    Native_00bd @ 0x0085_9840 => extern "C" fn n_859840(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00bd(&mut mem(), w, a, s)
    }
    /// `Native_01ad(worker, args, stack)`.
    Native_01ad @ 0x0085_8b30 => extern "C" fn n_858b30(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01ad(&mut mem(), w, a, s)
    }
    /// `Native_0106(worker, args, stack)`.
    Native_0106 @ 0x0085_a010 => extern "C" fn n_85a010(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0106(&mut mem(), w, a, s)
    }
    /// `Native_0177(worker, args, stack)`.
    Native_0177 @ 0x0085_c9c0 => extern "C" fn n_85c9c0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0177(&mut mem(), w, a, s)
    }
    /// `CT_0160_ret_hasKeyItem(worker, args, stack)`.
    CT_0160_ret_hasKeyItem @ 0x0085_b900 => extern "C" fn n_85b900(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0160(&mut mem(), w, a, s)
    }
    /// `Native_023c(worker, args, stack)`.
    Native_023c @ 0x0085_c600 => extern "C" fn n_85c600(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_023c(&mut mem(), w, a, s)
    }
    /// `Native_023b(worker, args, stack)`.
    Native_023b @ 0x0085_c490 => extern "C" fn n_85c490(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_023b(&mut mem(), w, a, s)
    }
    /// `Native_01d0(worker, args, stack)`.
    Native_01d0 @ 0x0085_fbb0 => extern "C" fn n_85fbb0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01d0(&mut mem(), w, a, s)
    }
    /// `Native_01ce(worker, args, stack)`.
    Native_01ce @ 0x0085_e7c0 => extern "C" fn n_85e7c0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01ce(&mut mem(), w, a, s)
    }
    /// `Native_01f9(worker, args, stack)`.
    Native_01f9 @ 0x0085_ac50 => extern "C" fn n_85ac50(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01f9(&mut mem(), w, a, s)
    }
    /// `Native_00bf(worker, args, stack)`.
    Native_00bf @ 0x0085_9d30 => extern "C" fn n_859d30(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_00bf(&mut mem(), w, a, s)
    }
    /// `Native_00c0(worker, args, stack)`.
    Native_00c0 @ 0x0085_9e00 => extern "C" fn n_859e00(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_00c0(&mut mem(), w, a, s)
    }
    /// `Native_00c1(worker, args, stack)`.
    Native_00c1 @ 0x0085_9ef0 => extern "C" fn n_859ef0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_00c1(&mut mem(), w, a, s)
    }
    /// `Native_00c2(worker, args, stack)`.
    Native_00c2 @ 0x0085_a070 => extern "C" fn n_85a070(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_00c2(&mut mem(), w, a, s)
    }
    /// `Native_017b(worker, args, stack)`.
    Native_017b @ 0x0085_d3d0 => extern "C" fn n_85d3d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_017b(&mut mem(), w, a, s)
    }
    /// `Native_00ec(worker, args, stack)`.
    Native_00ec @ 0x0085_6bc0 => extern "C" fn n_856bc0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00ec(&mut mem(), w, a, s)
    }
    /// `Native_01bd(worker, args, stack)`.
    Native_01bd @ 0x0085_9ff0 => extern "C" fn n_859ff0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01bd(&mut mem(), w, a, s)
    }
    /// `Native_0050(worker, args, stack)`.
    Native_0050 @ 0x0085_ec60 => extern "C" fn n_85ec60(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0050(&mut mem(), w, a, s)
    }
    /// `Native_0221(worker, args, stack)`.
    Native_0221 @ 0x0085_e110 => extern "C" fn n_85e110(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0221(&mut mem(), w, a, s)
    }
    /// `Native_0143(worker, args, stack)`.
    Native_0143 @ 0x0085_7ad0 => extern "C" fn n_857ad0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0143(&mut mem(), w, a, s)
    }
    /// `Native_0144(worker, args, stack)`.
    Native_0144 @ 0x0085_7d40 => extern "C" fn n_857d40(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0144(&mut mem(), w, a, s)
    }
    /// `Native_025b(worker, args, stack)`.
    Native_025b @ 0x0085_9270 => extern "C" fn n_859270(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_025b(&mut mem(), w, a, s)
    }
    /// `Native_0198(worker, args, stack)`.
    Native_0198 @ 0x0085_ffa0 => extern "C" fn n_85ffa0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0198(&mut mem(), w, a, s)
    }
    /// `Native_0199(worker, args, stack)`.
    Native_0199 @ 0x0086_0200 => extern "C" fn n_860200(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0199(&mut mem(), w, a, s)
    }
    /// `Native_0141(worker, args, stack)`.
    Native_0141 @ 0x0085_7a00 => extern "C" fn n_857a00(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0141(&mut mem(), w, a, s)
    }
    /// `Native_00cf(worker, args, stack)`.
    Native_00cf @ 0x0085_c400 => extern "C" fn n_85c400(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00cf(&mut mem(), w, a, s)
    }
    /// `Native_020b(worker, args, stack)`.
    Native_020b @ 0x0085_be30 => extern "C" fn n_85be30(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_020b(&mut mem(), w, a, s)
    }
    /// `Native_01ca(worker, args, stack)`.
    Native_01ca @ 0x0085_e340 => extern "C" fn n_85e340(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01ca(&mut mem(), w, a, s)
    }
    /// `Native_0237(worker, args, stack)`.
    Native_0237 @ 0x0086_04c0 => extern "C" fn n_8604c0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0237(&mut mem(), w, a, s)
    }
    /// `CT_01FC_ret_teachAbilityToPartyMemberSilently(worker, args, stack)`.
    CT_01FC_ret_teachAbilityToPartyMemberSilently @ 0x0085_b170 => extern "C" fn n_85b170(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01fc(&mut mem(), w, a, s)
    }
    /// `Native_0051(worker, args, stack)`.
    Native_0051 @ 0x0085_f0e0 => extern "C" fn n_85f0e0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0051(&mut mem(), w, a, s)
    }
    /// `Native_00e8(worker, args, stack)`.
    Native_00e8 @ 0x0085_6270 => extern "C" fn n_856270(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00e8(&mut mem(), w, a, s)
    }
    /// `Native_0030(worker, args, stack)`.
    Native_0030 @ 0x0085_a7c0 => extern "C" fn n_85a7c0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0030(&mut mem(), w, a, s)
    }
    /// `Native_01d4(worker, args, stack)`.
    Native_01d4 @ 0x0085_5dd0 => extern "C" fn n_855dd0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01d4(&mut mem(), w, a, s)
    }
    /// `Native_022f(worker, args, stack)`.
    Native_022f @ 0x0085_5fb0 => extern "C" fn n_855fb0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_022f(&mut mem(), w, a, s)
    }
    /// `Native_0174(worker, args, stack)`.
    Native_0174 @ 0x0085_e3a0 => extern "C" fn n_85e3a0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0174(&mut mem(), w, a, s)
    }
    /// `Native_0161(worker, args, stack)`.
    Native_0161 @ 0x0085_b760 => extern "C" fn n_85b760(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0161(&mut mem(), w, a, s)
    }
    /// `Native_016a(worker, args, stack)`.
    Native_016a @ 0x0085_bbb0 => extern "C" fn n_85bbb0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_016a(&mut mem(), w, a, s)
    }
    /// `Native_00fe(worker, args, stack)`.
    Native_00fe @ 0x0086_0090 => extern "C" fn n_860090(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00fe(&mut mem(), w, a, s)
    }
    /// `Native_024d(worker, args, stack)`.
    Native_024d @ 0x0085_6dc0 => extern "C" fn n_856dc0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_024d(&mut mem(), w, a, s)
    }
    /// `Native_0209(worker, args, stack)`.
    Native_0209 @ 0x0085_f430 => extern "C" fn n_85f430(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0209(&mut mem(), w, a, s)
    }
    /// `Native_010d(worker, args, stack)`.
    Native_010d @ 0x0085_ac90 => extern "C" fn n_85ac90(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_010d(&mut mem(), w, a, s)
    }
    /// `Native_0194(worker, args, stack)`.
    Native_0194 @ 0x0085_f8b0 => extern "C" fn n_85f8b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0194(&mut mem(), w, a, s)
    }
    /// `Native_010c(worker, args, stack)`.
    Native_010c @ 0x0085_8610 => extern "C" fn n_858610(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_010c(&mut mem(), w, a, s)
    }
    /// `Native_00ab(worker, args, stack)`.
    Native_00ab @ 0x0085_8330 => extern "C" fn n_858330(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00ab(&mut mem(), w, a, s)
    }
    /// `Native_011a(worker, args, stack)`.
    Native_011a @ 0x0085_c0a0 => extern "C" fn n_85c0a0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_011a(&mut mem(), w, a, s)
    }
    /// `Native_014e(worker, args, stack)`.
    Native_014e @ 0x0085_8ea0 => extern "C" fn n_858ea0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_014e(&mut mem(), w, a, s)
    }
    /// `Native_024b(worker, args, stack)`.
    Native_024b @ 0x0085_7e30 => extern "C" fn n_857e30(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_024b(&mut mem(), w, a, s)
    }
    /// `CT_0216_exec_teachAbilityToPartyMemberWithMsg(worker, args, stack)`.
    CT_0216_exec_teachAbilityToPartyMemberWithMsg @ 0x0085_b5d0 => extern "C" fn n_85b5d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0216_poll(&mut mem(), w, a, s)
    }
    /// `CT_0216_ret_teachAbilityToPartyMemberWithMsg(worker, args, stack)`.
    CT_0216_ret_teachAbilityToPartyMemberWithMsg @ 0x0085_b920 => extern "C" fn n_85b920(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0216_result(&mut mem(), w, a, s)
    }
    /// `Native_01f8(worker, args, stack)`.
    Native_01f8 @ 0x0085_e570 => extern "C" fn n_85e570(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01f8(&mut mem(), w, a, s)
    }
    /// `Native_0114(worker, args, stack)`.
    Native_0114 @ 0x0085_b670 => extern "C" fn n_85b670(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0114(&mut mem(), w, a, s)
    }
    /// `Native_0220(worker, args, stack)`.
    Native_0220 @ 0x0085_e010 => extern "C" fn n_85e010(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0220(&mut mem(), w, a, s)
    }
    /// `Native_0203(worker, args, stack)`.
    Native_0203 @ 0x0085_d460 => extern "C" fn n_85d460(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0203(&mut mem(), w, a, s)
    }
    /// `Native_0203_Poll(worker, args, stack)`.
    Native_0203_Poll @ 0x0085_d550 => extern "C" fn n_85d550(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0203_poll(&mut mem(), w, a, s)
    }
    /// `Native_0203_Result(worker, args, stack)`.
    Native_0203_Result @ 0x0085_dc10 => extern "C" fn n_85dc10(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0203_result(&mut mem(), w, a, s)
    }
    /// `Native_01a8(worker, args, stack)`.
    Native_01a8 @ 0x0085_8390 => extern "C" fn n_858390(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01a8(&mut mem(), w, a, s)
    }
    /// `Native_0192(worker, args, stack)`.
    Native_0192 @ 0x0085_f700 => extern "C" fn n_85f700(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0192(&mut mem(), w, a, s)
    }
    /// `Native_01c4(worker, args, stack)`.
    Native_01c4 @ 0x0085_a840 => extern "C" fn n_85a840(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01c4(&mut mem(), w, a, s)
    }
    /// `Native_012e(worker, args, stack)`.
    Native_012e @ 0x0085_e040 => extern "C" fn n_85e040(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_012e(&mut mem(), w, a, s)
    }
    /// `Native_012b(worker, args, stack)`.
    Native_012b @ 0x0085_dc50 => extern "C" fn n_85dc50(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_012b(&mut mem(), w, a, s)
    }
    /// `Native_0092(worker, args, stack)`.
    Native_0092 @ 0x0085_eac0 => extern "C" fn n_85eac0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0092(&mut mem(), w, a, s)
    }
    /// `Native_023a(worker, args, stack)`.
    Native_023a @ 0x0085_6250 => extern "C" fn n_856250(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_023a(&mut mem(), w, a, s)
    }
    /// `Native_01e5(worker, args, stack)`.
    Native_01e5 @ 0x0085_9000 => extern "C" fn n_859000(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01e5(&mut mem(), w, a, s)
    }
    /// `Native_023f(worker, args, stack)`.
    Native_023f @ 0x0085_6770 => extern "C" fn n_856770(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_023f(&mut mem(), w, a, s)
    }
    /// `Native_01cf(worker, args, stack)`.
    Native_01cf @ 0x0085_ea40 => extern "C" fn n_85ea40(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01cf(&mut mem(), w, a, s)
    }
    /// `Native_01b0(worker, args, stack)`.
    Native_01b0 @ 0x0085_90a0 => extern "C" fn n_8590a0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01b0(&mut mem(), w, a, s)
    }
    /// `Native_020f(worker, args, stack)`.
    Native_020f @ 0x0085_c100 => extern "C" fn n_85c100(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_020f(&mut mem(), w, a, s)
    }
    /// `Native_01dc(worker, args, stack)`.
    Native_01dc @ 0x0085_71d0 => extern "C" fn n_8571d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01dc(&mut mem(), w, a, s)
    }
    /// `Native_0202(worker, args, stack)`.
    Native_0202 @ 0x0085_a770 => extern "C" fn n_85a770(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0202(&mut mem(), w, a, s)
    }
    /// `Native_0098(worker, args, stack)`.
    Native_0098 @ 0x0085_fb50 => extern "C" fn n_85fb50(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0098(&mut mem(), w, a, s)
    }
    /// `Native_0193(worker, args, stack)`.
    Native_0193 @ 0x0085_f4e0 => extern "C" fn n_85f4e0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0193(&mut mem(), w, a, s)
    }
    /// `Native_0189(worker, args, stack)`.
    Native_0189 @ 0x0085_e890 => extern "C" fn n_85e890(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0189(&mut mem(), w, a, s)
    }
    /// `Native_00a2(worker, args, stack)`.
    Native_00a2 @ 0x0085_6570 => extern "C" fn n_856570(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00a2(&mut mem(), w, a, s)
    }
    /// `Native_00ee(worker, args, stack)`.
    Native_00ee @ 0x0085_72b0 => extern "C" fn n_8572b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00ee(&mut mem(), w, a, s)
    }
    /// `Native_020d(worker, args, stack)`.
    Native_020d @ 0x0085_bf70 => extern "C" fn n_85bf70(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_020d(&mut mem(), w, a, s)
    }
    /// `Native_0149(worker, args, stack)`.
    Native_0149 @ 0x0085_8580 => extern "C" fn n_858580(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0149(&mut mem(), w, a, s)
    }
    /// `Native_0146(worker, args, stack)`.
    Native_0146 @ 0x0085_81a0 => extern "C" fn n_8581a0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0146(&mut mem(), w, a, s)
    }
    /// `Native_0145(worker, args, stack)`.
    Native_0145 @ 0x0085_7fc0 => extern "C" fn n_857fc0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0145(&mut mem(), w, a, s)
    }
    /// `Native_0226(worker, args, stack)`.
    Native_0226 @ 0x0085_eb80 => extern "C" fn n_85eb80(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0226(&mut mem(), w, a, s)
    }
    /// `Native_00d2(worker, args, stack)`.
    Native_00d2 @ 0x0085_c6f0 => extern "C" fn n_85c6f0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00d2(&mut mem(), w, a, s)
    }
    /// `Native_0037(worker, args, stack)`.
    Native_0037 @ 0x0085_b780 => extern "C" fn n_85b780(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0037(&mut mem(), w, a, s)
    }
    /// `Native_0159(worker, args, stack)`.
    Native_0159 @ 0x0085_d290 => extern "C" fn n_85d290(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0159(&mut mem(), w, a, s)
    }
    /// `Native_0204(worker, args, stack)`.
    Native_0204 @ 0x0085_dc90 => extern "C" fn n_85dc90(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0204(&mut mem(), w, a, s)
    }
    /// `Native_01c3(worker, args, stack)`.
    Native_01c3 @ 0x0085_a750 => extern "C" fn n_85a750(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01c3(&mut mem(), w, a, s)
    }
    /// `Native_0260(worker, args, stack)`.
    Native_0260 @ 0x0085_9f10 => extern "C" fn n_859f10(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0260(&mut mem(), w, a, s)
    }
    /// `Native_0261(worker, args, stack)`.
    Native_0261 @ 0x0085_a090 => extern "C" fn n_85a090(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0261(&mut mem(), w, a, s)
    }
    /// `Native_0251(worker, args, stack)`.
    Native_0251 @ 0x0085_e5e0 => extern "C" fn n_85e5e0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0251(&mut mem(), w, a, s)
    }
    /// `Native_0205(worker, args, stack)`.
    Native_0205 @ 0x0085_ec00 => extern "C" fn n_85ec00(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0205(&mut mem(), w, a, s)
    }
    /// `Native_0206(worker, args, stack)`.
    Native_0206 @ 0x0085_ee10 => extern "C" fn n_85ee10(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0206(&mut mem(), w, a, s)
    }
    /// `Native_020a(worker, args, stack)`.
    Native_020a @ 0x0085_f660 => extern "C" fn n_85f660(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_020a(&mut mem(), w, a, s)
    }
    /// `Native_010a(worker, args, stack)`.
    Native_010a @ 0x0085_a630 => extern "C" fn n_85a630(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_010a(&mut mem(), w, a, s)
    }
    /// `Native_00e2(worker, args, stack)`.
    Native_00e2 @ 0x0085_7dd0 => extern "C" fn n_857dd0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00e2(&mut mem(), w, a, s)
    }
    /// `Native_00c4(worker, args, stack)`.
    Native_00c4 @ 0x0085_a340 => extern "C" fn n_85a340(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00c4(&mut mem(), w, a, s)
    }
    /// `Native_0003(worker, args, stack)`.
    Native_0003 @ 0x0085_ebd0 => extern "C" fn n_85ebd0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0003(&mut mem(), w, a, s)
    }
    /// `Native_0201(worker, args, stack)`.
    Native_0201 @ 0x0085_a580 => extern "C" fn n_85a580(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0201(&mut mem(), w, a, s)
    }
    /// `Native_0200(worker, args, stack)`.
    Native_0200 @ 0x0085_a430 => extern "C" fn n_85a430(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0200(&mut mem(), w, a, s)
    }
    /// `Native_01c5(worker, args, stack)`.
    Native_01c5 @ 0x0085_b1a0 => extern "C" fn n_85b1a0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01c5(&mut mem(), w, a, s)
    }
    /// `Native_01c5_Poll(worker, args, stack)`.
    Native_01c5_Poll @ 0x0085_b590 => extern "C" fn n_85b590(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01c5_poll(&mut mem(), w, a, s)
    }
    /// `Native_01c5_Result(worker, args, stack)`.
    Native_01c5_Result @ 0x0085_d8f0 => extern "C" fn n_85d8f0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01c5_result(&mut mem(), w, a, s)
    }
    /// `Native_0115(worker, args, stack)`.
    Native_0115 @ 0x0085_b8c0 => extern "C" fn n_85b8c0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0115(&mut mem(), w, a, s)
    }
    /// `Native_00fa(worker, args, stack)`.
    Native_00fa @ 0x0085_8920 => extern "C" fn n_858920(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00fa(&mut mem(), w, a, s)
    }
    /// `Native_0259(worker, args, stack)`.
    Native_0259 @ 0x0085_8fc0 => extern "C" fn n_858fc0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0259(&mut mem(), w, a, s)
    }
    /// `Native_0254(worker, args, stack)`.
    Native_0254 @ 0x0085_85d0 => extern "C" fn n_8585d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0254(&mut mem(), w, a, s)
    }
    /// `Native_01ac(worker, args, stack)`.
    Native_01ac @ 0x0085_8950 => extern "C" fn n_858950(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01ac(&mut mem(), w, a, s)
    }
    /// `Native_01a7(worker, args, stack)`.
    Native_01a7 @ 0x0085_7b70 => extern "C" fn n_857b70(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01a7(&mut mem(), w, a, s)
    }
    /// `Native_01a7_Poll(worker, args, stack)`.
    Native_01a7_Poll @ 0x0085_7ef0 => extern "C" fn n_857ef0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01a7_poll(&mut mem(), w, a, s)
    }
    /// `Native_01a7_Result(worker, args, stack)`.
    Native_01a7_Result @ 0x0085_8230 => extern "C" fn n_858230(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01a7_result(&mut mem(), w, a, s)
    }
    /// `Native_01a0(worker, args, stack)`.
    Native_01a0 @ 0x0085_f940 => extern "C" fn n_85f940(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01a0(&mut mem(), w, a, s)
    }
    /// `Native_019e(worker, args, stack)`.
    Native_019e @ 0x0085_6950 => extern "C" fn n_856950(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_019e(&mut mem(), w, a, s)
    }
    /// `Native_00c9(worker, args, stack)`.
    Native_00c9 @ 0x0085_b560 => extern "C" fn n_85b560(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_00c9(&mut mem(), w, a, s)
    }
    /// `Native_01d2(worker, args, stack)`.
    Native_01d2 @ 0x0086_0180 => extern "C" fn n_860180(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01d2(&mut mem(), w, a, s)
    }
    /// `Native_01bc(worker, args, stack)`.
    Native_01bc @ 0x0085_9ec0 => extern "C" fn n_859ec0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01bc(&mut mem(), w, a, s)
    }
    /// `Native_014f(worker, args, stack)`.
    Native_014f @ 0x0085_9120 => extern "C" fn n_859120(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_014f(&mut mem(), w, a, s)
    }
    /// `Native_025d(worker, args, stack)`.
    Native_025d @ 0x0085_9c20 => extern "C" fn n_859c20(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_025d(&mut mem(), w, a, s)
    }
    /// `Native_0231(worker, args, stack)`.
    Native_0231 @ 0x0085_fca0 => extern "C" fn n_85fca0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0231(&mut mem(), w, a, s)
    }
    /// `Native_01ee(worker, args, stack)`.
    Native_01ee @ 0x0085_7440 => extern "C" fn n_857440(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01ee(&mut mem(), w, a, s)
    }
    /// `Native_01e7(worker, args, stack)`.
    Native_01e7 @ 0x0085_9320 => extern "C" fn n_859320(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01e7(&mut mem(), w, a, s)
    }
    /// `Native_01e0(worker, args, stack)`.
    Native_01e0 @ 0x0085_8490 => extern "C" fn n_858490(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01e0(&mut mem(), w, a, s)
    }
    /// `Native_018a(worker, args, stack)`.
    Native_018a @ 0x0085_ebb0 => extern "C" fn n_85ebb0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_018a(&mut mem(), w, a, s)
    }
    /// `Native_009b(worker, args, stack)`.
    Native_009b @ 0x0085_f750 => extern "C" fn n_85f750(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_009b(&mut mem(), w, a, s)
    }
    /// `Native_01de(worker, args, stack)`.
    Native_01de @ 0x0085_7920 => extern "C" fn n_857920(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01de(&mut mem(), w, a, s)
    }
    /// `Native_01de_Poll(worker, args, stack)`.
    Native_01de_Poll @ 0x0085_79d0 => extern "C" fn n_8579d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01de_poll(&mut mem(), w, a, s)
    }
    /// `Native_01af(worker, args, stack)`.
    Native_01af @ 0x0085_8e70 => extern "C" fn n_858e70(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01af(&mut mem(), w, a, s)
    }
    /// `Native_0140(worker, args, stack)`.
    Native_0140 @ 0x0085_6f20 => extern "C" fn n_856f20(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0140(&mut mem(), w, a, s)
    }
    /// `Native_00cc(worker, args, stack)`.
    Native_00cc @ 0x0085_b9f0 => extern "C" fn n_85b9f0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00cc(&mut mem(), w, a, s)
    }
    /// `Native_0239(worker, args, stack)`.
    Native_0239 @ 0x0085_e8b0 => extern "C" fn n_85e8b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0239(&mut mem(), w, a, s)
    }
    /// `Native_013d(worker, args, stack)`.
    Native_013d @ 0x0085_61b0 => extern "C" fn n_8561b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_013d(&mut mem(), w, a, s)
    }
    /// `Native_013d_Poll(worker, args, stack)`.
    Native_013d_Poll @ 0x0085_6300 => extern "C" fn n_856300(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_013d_poll(&mut mem(), w, a, s)
    }
    /// `Native_025f(worker, args, stack)`.
    Native_025f @ 0x0085_9e20 => extern "C" fn n_859e20(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_025f(&mut mem(), w, a, s)
    }
    /// `Native_025e(worker, args, stack)`.
    Native_025e @ 0x0085_9d50 => extern "C" fn n_859d50(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_025e(&mut mem(), w, a, s)
    }
    /// `Native_024a(worker, args, stack)`.
    Native_024a @ 0x0085_7cb0 => extern "C" fn n_857cb0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_024a(&mut mem(), w, a, s)
    }
    /// `Native_0240(worker, args, stack)`.
    Native_0240 @ 0x0085_68d0 => extern "C" fn n_8568d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0240(&mut mem(), w, a, s)
    }
    /// `Native_021d(worker, args, stack)`.
    Native_021d @ 0x0085_dcd0 => extern "C" fn n_85dcd0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_021d(&mut mem(), w, a, s)
    }
    /// `Native_0212(worker, args, stack)`.
    Native_0212 @ 0x0085_c950 => extern "C" fn n_85c950(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0212(&mut mem(), w, a, s)
    }
    /// `Native_01f2(worker, args, stack)`.
    Native_01f2 @ 0x0085_97d0 => extern "C" fn n_8597d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01f2(&mut mem(), w, a, s)
    }
    /// `Native_01f0(worker, args, stack)`.
    Native_01f0 @ 0x0085_9550 => extern "C" fn n_859550(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_01f0(&mut mem(), w, a, s)
    }
    /// `Native_01a6(worker, args, stack)`.
    Native_01a6 @ 0x0085_76d0 => extern "C" fn n_8576d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01a6(&mut mem(), w, a, s)
    }
    /// `Native_018b(worker, args, stack)`.
    Native_018b @ 0x0085_ec90 => extern "C" fn n_85ec90(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_018b(&mut mem(), w, a, s)
    }
    /// `Native_0184(worker, args, stack)`.
    Native_0184 @ 0x0085_dac0 => extern "C" fn n_85dac0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0184(&mut mem(), w, a, s)
    }
    /// `Native_0185(worker, args, stack)`.
    Native_0185 @ 0x0085_dbb0 => extern "C" fn n_85dbb0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0185(&mut mem(), w, a, s)
    }
    /// `Native_0166(worker, args, stack)`.
    Native_0166 @ 0x0085_98f0 => extern "C" fn n_8598f0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0166(&mut mem(), w, a, s)
    }
    /// `Native_017c(worker, args, stack)`.
    Native_017c @ 0x0085_d480 => extern "C" fn n_85d480(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_017c(&mut mem(), w, a, s)
    }
    /// `Native_0156(worker, args, stack)`.
    Native_0156 @ 0x0085_9920 => extern "C" fn n_859920(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0156(&mut mem(), w, a, s)
    }
    /// `Native_0154(worker, args, stack)`.
    Native_0154 @ 0x0085_97a0 => extern "C" fn n_8597a0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0154(&mut mem(), w, a, s)
    }
    /// `Native_0110(worker, args, stack)`.
    Native_0110 @ 0x0085_b260 => extern "C" fn n_85b260(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0110(&mut mem(), w, a, s)
    }
    /// `Native_010f(worker, args, stack)`.
    Native_010f @ 0x0085_afd0 => extern "C" fn n_85afd0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_010f(&mut mem(), w, a, s)
    }
    /// `Native_00ef(worker, args, stack)`.
    Native_00ef @ 0x0085_7470 => extern "C" fn n_857470(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00ef(&mut mem(), w, a, s)
    }
    /// `Native_00e6(worker, args, stack)`.
    Native_00e6 @ 0x0085_bf90 => extern "C" fn n_85bf90(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00e6(&mut mem(), w, a, s)
    }
    /// `Native_00ac(worker, args, stack)`.
    Native_00ac @ 0x0085_8050 => extern "C" fn n_858050(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00ac(&mut mem(), w, a, s)
    }
    /// `Native_0264(worker, args, stack)`.
    Native_0264 @ 0x0085_a230 => extern "C" fn n_85a230(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0264(&mut mem(), w, a, s)
    }
    /// `Native_022e(worker, args, stack)`.
    Native_022e @ 0x0085_f860 => extern "C" fn n_85f860(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_022e(&mut mem(), w, a, s)
    }
    /// `Native_01f4(worker, args, stack)`.
    Native_01f4 @ 0x0085_9fa0 => extern "C" fn n_859fa0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01f4(&mut mem(), w, a, s)
    }
    /// `Native_01f3(worker, args, stack)`.
    Native_01f3 @ 0x0085_9990 => extern "C" fn n_859990(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01f3(&mut mem(), w, a, s)
    }
    /// `Native_01c0(worker, args, stack)`.
    Native_01c0 @ 0x0085_a250 => extern "C" fn n_85a250(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01c0(&mut mem(), w, a, s)
    }
    /// `Native_01b7(worker, args, stack)`.
    Native_01b7 @ 0x0085_9800 => extern "C" fn n_859800(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01b7(&mut mem(), w, a, s)
    }
    /// `Native_01b2(worker, args, stack)`.
    Native_01b2 @ 0x0085_9360 => extern "C" fn n_859360(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01b2(&mut mem(), w, a, s)
    }
    /// `Native_018f(worker, args, stack)`.
    Native_018f @ 0x0085_f240 => extern "C" fn n_85f240(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_018f(&mut mem(), w, a, s)
    }
    /// `Native_0179(worker, args, stack)`.
    Native_0179 @ 0x0085_cd90 => extern "C" fn n_85cd90(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0179(&mut mem(), w, a, s)
    }
    /// `Native_0179_Poll(worker, args, stack)`.
    Native_0179_Poll @ 0x0085_cea0 => extern "C" fn n_85cea0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0179_poll(&mut mem(), w, a, s)
    }
    /// `Native_0138(worker, args, stack)`.
    Native_0138 @ 0x0085_f140 => extern "C" fn n_85f140(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0138(&mut mem(), w, a, s)
    }
    /// `Native_0127(worker, args, stack)`.
    Native_0127 @ 0x0085_db10 => extern "C" fn n_85db10(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0127(&mut mem(), w, a, s)
    }
    /// `Native_0107(worker, args, stack)`.
    Native_0107 @ 0x0085_a1f0 => extern "C" fn n_85a1f0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0107(&mut mem(), w, a, s)
    }
    /// `Native_00b4(worker, args, stack)`.
    Native_00b4 @ 0x0085_8a20 => extern "C" fn n_858a20(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00b4(&mut mem(), w, a, s)
    }
    /// `Native_00b3(worker, args, stack)`.
    Native_00b3 @ 0x0085_8810 => extern "C" fn n_858810(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00b3(&mut mem(), w, a, s)
    }
    /// `Native_00b2(worker, args, stack)`.
    Native_00b2 @ 0x0085_86b0 => extern "C" fn n_8586b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00b2(&mut mem(), w, a, s)
    }
    /// `Native_0012(worker, args, stack)`.
    Native_0012 @ 0x0085_f800 => extern "C" fn n_85f800(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0012(&mut mem(), w, a, s)
    }
    /// `Native_015b(worker, args, stack)`.
    Native_015b @ 0x0085_a8a0 => extern "C" fn n_85a8a0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_015b(&mut mem(), w, a, s)
    }
    /// `CT_0216_init_teachAbilityToPartyMemberWithMsg(worker, args, stack)`.
    CT_0216_init_teachAbilityToPartyMemberWithMsg @ 0x0085_b280 => extern "C" fn n_85b280(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0216(&mut mem(), w, a, s)
    }
}
