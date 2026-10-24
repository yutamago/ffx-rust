//! `ffx-script`: the Atel VM's ctrl works, workers, stacks and variables.

use ffx_script::vm::bridge as atel;
use ffx_script::vm::bridge::globals as g;
use ffx_script::vm::bridge::helpers as hp;
use ffx_script::vm::bridge::native;
use ffx_script::vm::bridge::native_btl;
use ffx_script::vm::bridge::native_btl2;
use ffx_script::vm::bridge::native_cam;
use ffx_script::vm::bridge::native_chr;
use ffx_script::vm::bridge::native_map;
use ffx_script::vm::bridge::native_mods;
use ffx_script::vm::bridge::native_move;
use ffx_script::vm::bridge::native_msg;
use ffx_script::vm::bridge::native_sys;
use ffx_script::vm::bridge::run;
use ffx_script::vm::bridge::signal as sig;

use super::{mem, ported};

ported! {
    /// `AtelDecodeCommand(i, mode)`.
    AtelDecodeCommand @ 0x0086_41c0 => extern "C" fn h_8641c0(i: u32, mode: u32) -> u32 {
        hp::atel_decode_command(&mut mem(), i, mode)
    }
    /// `AtelSetSaveDicIndex(i)`.
    AtelSetSaveDicIndex @ 0x0087_0ef0 => extern "C" fn h_870ef0(i: u32) -> u32 {
        hp::atel_set_save_dic_index(&mut mem(), i)
    }
    /// `AtelSetEventJump(map, entrance)`.
    AtelSetEventJump @ 0x0086_fec0 => extern "C" fn h_86fec0(map: u32, e: u32) -> u32 {
        hp::atel_set_event_jump(&mut mem(), map, e)
    }
    /// `AtelSetEventJump2(map, entrance, own)`.
    AtelSetEventJump2 @ 0x0086_ff40 => extern "C" fn h_86ff40(map: u32, e: u32, own: u32) -> u32 {
        hp::atel_set_event_jump2(&mut mem(), map, e, own)
    }
    /// `AtelUcOn()`.
    AtelUcOn @ 0x0087_1ab0 => extern "C" fn h_871ab0() {
        hp::atel_uc_on(&mut mem());
    }
    /// `Script_StopMove(worker, level)`.
    Script_StopMove @ 0x0086_3e10 => extern "C" fn h_863e10(w: u32, l: u32) -> u32 {
        hp::script_stop_move(&mut mem(), w, l)
    }
    /// `AtelGetBattleIdAdrs(i)`.
    AtelGetBattleIdAdrs @ 0x0086_a970 => extern "C" fn h_86a970(i: u32) -> u32 {
        hp::atel_get_battle_id_adrs(&mut mem(), i)
    }
    /// `AtelSetMoveHokanMode(worker, mode)`.
    AtelSetMoveHokanMode @ 0x0087_03b0 => extern "C" fn h_8703b0(w: u32, mode: u32) -> u32 {
        hp::atel_set_move_hokan_mode(&mut mem(), w, mode)
    }
    /// `AtelGetEventCharHandleFromID(i)`.
    AtelGetEventCharHandleFromID @ 0x0086_b3a0 => extern "C" fn h_86b3a0(i: u32) -> u32 {
        hp::atel_get_event_char_handle_from_id(&mut mem(), i)
    }
    /// `AtelGetEventActorPos(i, x, y, z)`.
    AtelGetEventActorPos @ 0x0086_b1e0 => extern "C" fn h_86b1e0(i: u32, x: u32, y: u32, z: u32) -> u32 {
        hp::atel_get_event_actor_pos(&mut mem(), i, x, y, z)
    }
    /// `Event_GetEntrance(worker)`.
    Event_GetEntrance @ 0x0086_bfe0 => extern "C" fn h_86bfe0(w: u32) -> u32 {
        hp::event_get_entrance(&mut mem(), w)
    }
    /// `Event_GetEntranceAt(worker, i)`.
    Event_GetEntranceAt @ 0x0086_c020 => extern "C" fn h_86c020(w: u32, i: u32) -> u32 {
        hp::event_get_entrance_at(&mut mem(), w, i)
    }
    /// `AtelGetHelpEventFlag(i)`.
    AtelGetHelpEventFlag @ 0x0086_b9e0 => extern "C" fn h_86b9e0(i: u32) -> u32 {
        hp::atel_get_help_event_flag(&mut mem(), i)
    }
    /// `AtelSetUpCtrlWork(i)`.
    AtelSetUpCtrlWork @ 0x0087_1190 => extern "C" fn h_871190(i: u32) -> u32 {
        hp::atel_set_up_ctrl_work(&mut mem(), i)
    }
    /// `AtelSkipReqLevel2(i, level)`.
    AtelSkipReqLevel2 @ 0x0087_1730 => extern "C" fn h_871730(i: u32, l: u32) {
        hp::atel_skip_req_level2(&mut mem(), i, l);
    }
    /// `AtelGetSummonNameforFinalBattleSelector(i)`.
    AtelGetSummonNameforFinalBattleSelector @ 0x0087_3b50 => extern "C" fn h_873b50(i: u32) -> u32 {
        hp::atel_get_summon_name_for_final_battle_selector(&mut mem(), i)
    }
    /// `AtelMapLoad(map)`.
    AtelMapLoad @ 0x0086_dca0 => extern "C" fn h_86dca0(map: u32) -> u32 {
        hp::atel_map_load(&mut mem(), map)
    }
    /// `AtelGetCharHandleFromBindPC(chr)`.
    AtelGetCharHandleFromBindPC @ 0x0086_ae90 => extern "C" fn h_86ae90(chr: u32) -> u32 {
        hp::atel_get_char_handle_from_bind_pc(&mut mem(), chr)
    }
    /// `AtelBindEff2OffSome_00862680(worker, which)`.
    AtelBindEff2OffSome_00862680 @ 0x0086_2680 => extern "C" fn h_862680(w: u32, k: u32) -> u32 {
        hp::atel_bind_eff2_off_some(&mut mem(), w, k)
    }
    /// `AtelCheckCaptMonster(mode, id, min)`.
    AtelCheckCaptMonster @ 0x0087_2aa0 => extern "C" fn h_872aa0(mode: u32, id: u32, min: u32) -> u32 {
        hp::atel_check_capt_monster(&mut mem(), mode, id, min)
    }
    /// `AtelRequestTrigger_008764f0(from, to, kind)`.
    AtelRequestTrigger_008764f0 @ 0x0087_64f0 => extern "C" fn h_8764f0(from: u32, to: u32, k: u32) -> u32 {
        hp::atel_request_trigger(&mut mem(), from, to, k)
    }
    /// `Atel_CtrlStart(ctrl)`: the default start callback.
    Atel_CtrlStart @ 0x0086_83e0 => extern "C" fn h_8683e0(c: u32) -> u32 {
        hp::atel_ctrl_start(&mut mem(), c)
    }
    /// `AtelClearMesWinWork`.
    AtelClearMesWinWork @ 0x0086_3b10 => extern "C" fn h_863b10() -> u32 {
        hp::atel_clear_mes_win_work(&mut mem())
    }
    /// `Native_PlayMotionCommon(worker, stack, mode, other)`.
    Native_PlayMotionCommon @ 0x00a7_aae0 => extern "C" fn h_a7aae0(w: u32, s: u32, m: u32, o: u32) -> u32 {
        native_chr::native_play_motion_common(&mut mem(), w, s, m, o)
    }
    /// `Script_LineActorSide`.
    Script_LineActorSide @ 0x0086_32b0 => extern "C" fn h_8632b0(w: u32, k: u32) -> u32 {
        hp::script_line_actor_side(&mem(), w, k)
    }
    /// `Atel_ParseHex`.
    Atel_ParseHex @ 0x0087_7660 => extern "C" fn h_877660(s: u32) -> u32 {
        hp::atel_parse_hex(&mem(), s)
    }
    /// `Atel_SetBindMask`.
    Atel_SetBindMask @ 0x0086_f460 => extern "C" fn h_86f460(n: u32) -> u32 {
        hp::atel_set_bind_mask(&mut mem(), n)
    }
    /// `Atel_GetBinHdrFloat`.
    Atel_GetBinHdrFloat @ 0x0086_ab80 => extern "C" fn h_86ab80(b: u32, i: u32) -> f64 {
        hp::atel_get_bin_hdr_float(&mem(), b, i)
    }
    /// `AtelGetMyBufferSize`.
    AtelGetMyBufferSize @ 0x0086_c0c0 => extern "C" fn h_86c0c0(b: u32) -> u32 {
        hp::atel_get_my_buffer_size(&mem(), b)
    }
    /// `AtelRegistCallbackQue2`.
    AtelRegistCallbackQue2 @ 0x0087_51b0 => extern "C" fn h_8751b0(q: u32, f: u32, n: u32) -> u32 {
        hp::atel_regist_callback_que2(&mut mem(), q, f, n)
    }
    /// `AtelExecCallbackQue`.
    AtelExecCallbackQue @ 0x0087_50b0 => extern "C" fn h_8750b0(q: u32) -> u32 {
        hp::atel_exec_callback_que(&mut mem(), q)
    }
    /// `AtelRequireCtrlWork`.
    AtelRequireCtrlWork @ 0x0086_ee10 => extern "C" fn h_86ee10(i: u32, blk: u32, a: u32, b: u32) -> u32 {
        hp::atel_require_ctrl_work(&mut mem(), i, blk, a, b)
    }
    /// `AtelSetUpCallFunc`.
    AtelSetUpCallFunc @ 0x0087_7880 => extern "C" fn h_877880(g: u32, t: u32) -> u32 {
        hp::atel_set_up_call_func(&mut mem(), g, t)
    }
    /// `AtelGetSaveDic`.
    AtelGetSaveDic @ 0x0086_c410 => extern "C" fn h_86c410() -> u32 {
        hp::atel_get_save_dic(&mut mem())
    }
    /// `AtelGetSaveDicName`.
    AtelGetSaveDicName @ 0x0086_c430 => extern "C" fn h_86c430(i: u32, d: u32) -> u32 {
        hp::atel_get_save_dic_name(&mut mem(), i, d)
    }
    /// `Atel_PutDigits`.
    Atel_PutDigits @ 0x0087_7520 => extern "C" fn h_877520(v: u32, dst: u32, b: u32) -> u32 {
        hp::atel_put_digits(&mut mem(), v, dst, b)
    }
    /// `Atel_CopyTurnAngles`.
    Atel_CopyTurnAngles @ 0x0086_db80 => extern "C" fn h_86db80(w: u32, m: u32) -> u32 {
        hp::atel_copy_turn_angles(&mut mem(), w, m)
    }
    /// `AtelUcOff`.
    AtelUcOff @ 0x0087_19e0 => extern "C" fn h_8719e0() -> u32 {
        hp::atel_uc_off(&mut mem())
    }
    /// `Atel_GetBattleCountF`.
    Atel_GetBattleCountF @ 0x007a_3ac0 => extern "C" fn h_7a3ac0() -> f64 {
        hp::atel_get_battle_count_f(&mem())
    }
    /// `Native_8007(worker, args, stack)`.
    Native_8007 @ 0x0091_b050 => extern "C" fn n_91b050(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8007(&mut mem(), w, a, s)
    }
    /// `Native_8008(worker, args, stack)`.
    Native_8008 @ 0x0091_b060 => extern "C" fn n_91b060(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_8008(&mut mem(), w, a, s)
    }
    /// `Native_802c(worker, args, stack)`.
    Native_802c @ 0x0091_b680 => extern "C" fn n_91b680(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_802c(&mut mem(), w, a, s)
    }
    /// `Native_802d(worker, args, stack)`.
    Native_802d @ 0x0091_b690 => extern "C" fn n_91b690(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_802d(&mut mem(), w, a, s)
    }
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
    /// `Native_01a5_TexSwapSetFrame(worker, args, stack)`.
    Native_01a5_TexSwapSetFrame @ 0x0085_74c0 => extern "C" fn n_8574c0(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_805d(worker, args, stack)`.
    Native_805d @ 0x0091_c4a0 => extern "C" fn n_91c4a0(w: u32, a: u32, s: u32) -> u32 {
        native_map::native_805d(&mut mem(), w, a, s)
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
    /// `Native_Cam_6076(worker, args, stack)`.
    Native_Cam_6076 @ 0x007b_8fe0 => extern "C" fn n_7b8fe0(w: u32, a: u32, s: u32) -> u32 {
        native_cam::native_cam_6076(&mut mem(), w, a, s)
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
    /// `Native_003d_CopyTurnAnglesAll(worker, args, stack)`.
    Native_003d_CopyTurnAnglesAll @ 0x0085_c3e0 => extern "C" fn n_85c3e0(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_0036_WorkerStopMoveAll(worker, args, stack)`.
    Native_0036_WorkerStopMoveAll @ 0x0085_b4e0 => extern "C" fn n_85b4e0(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_0267_MsgSelectBgmType_Poll(worker, args, stack)`.
    Native_0267_MsgSelectBgmType_Poll @ 0x0085_a540 => extern "C" fn n_85a540(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_0267_poll(&mut mem(), w, a, s)
    }
    /// `Native_0267_MsgSelectBgmType_Result(worker, args, stack)`.
    Native_0267_MsgSelectBgmType_Result @ 0x0085_a620 => extern "C" fn n_85a620(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_0006_ChrAttachModel(worker, args, stack)`.
    Native_0006_ChrAttachModel @ 0x0085_e620 => extern "C" fn n_85e620(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0006(&mut mem(), w, a, s)
    }
    /// `Native_00f5_PitchToPoint(worker, args, stack)`.
    Native_00f5_PitchToPoint @ 0x0085_8f40 => extern "C" fn n_858f40(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_00f5(&mut mem(), w, a, s)
    }
    /// `Native_00cb_PartyRemove(worker, args, stack)`.
    Native_00cb_PartyRemove @ 0x0085_b820 => extern "C" fn n_85b820(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_5037_ChrGetOptPos0(worker, args, stack)`.
    Native_5037_ChrGetOptPos0 @ 0x00a7_a140 => extern "C" fn n_a7a140(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5037(&mut mem(), w, a, s)
    }
    /// `Native_6077_CamGetRoll_ResultF(worker, args, stack)`.
    Native_6077_CamGetRoll_ResultF @ 0x007b_9190 => extern "C" fn n_7b9190(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_6077(&mut mem(), w, a, s)
    }
    /// `Native_c025(worker, args, stack)`.
    Native_c025 @ 0x0087_8c70 => extern "C" fn n_878c70(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c025(&mut mem(), w, a, s)
    }
    /// `Native_506d_ChrSetRotX(worker, args, stack)`.
    Native_506d_ChrSetRotX @ 0x00a7_9a80 => extern "C" fn n_a79a80(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_506d(&mut mem(), w, a, s)
    }
    /// `Native_507d_ChrSetNeckRot(worker, args, stack)`.
    Native_507d_ChrSetNeckRot @ 0x00a7_a090 => extern "C" fn n_a7a090(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_507d(&mut mem(), w, a, s)
    }
    /// `Native_5066_ChrSyncEventMgrp(worker, args, stack)`.
    Native_5066_ChrSyncEventMgrp @ 0x00a7_9480 => extern "C" fn n_a79480(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5066(&mut mem(), w, a, s)
    }
    /// `Native_5066_ChrSyncEventMgrp_Poll(worker, args, stack)`.
    Native_5066_ChrSyncEventMgrp_Poll @ 0x00a7_94d0 => extern "C" fn n_a794d0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5066_poll(&mut mem(), w, a, s)
    }
    /// `Native_5084_ChrSetNeckSpeed(worker, args, stack)`.
    Native_5084_ChrSetNeckSpeed @ 0x00a7_a5d0 => extern "C" fn n_a7a5d0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5084(&mut mem(), w, a, s)
    }
    /// `Native_5085_ChrSetShadowHeight(worker, args, stack)`.
    Native_5085_ChrSetShadowHeight @ 0x00a7_a640 => extern "C" fn n_a7a640(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5085(&mut mem(), w, a, s)
    }
    /// `Native_5080_ChrSetHiju(worker, args, stack)`.
    Native_5080_ChrSetHiju @ 0x00a7_a410 => extern "C" fn n_a7a410(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5080(&mut mem(), w, a, s)
    }
    /// `Native_5014_ChrSetShade(worker, args, stack)`.
    Native_5014_ChrSetShade @ 0x00a7_8f30 => extern "C" fn n_a78f30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5014(&mut mem(), w, a, s)
    }
    /// `Native_5088_ChrSetShadeId(worker, args, stack)`.
    Native_5088_ChrSetShadeId @ 0x00a7_a6b0 => extern "C" fn n_a7a6b0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5088(&mut mem(), w, a, s)
    }
    /// `Native_503b_ChrDisposeSystemMgrp(worker, args, stack)`.
    Native_503b_ChrDisposeSystemMgrp @ 0x00a7_a450 => extern "C" fn n_a7a450(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_503b(&mut mem(), w, a, s)
    }
    /// `Native_1015_FacingWithin(worker, args, stack)`.
    Native_1015_FacingWithin @ 0x0087_7dc0 => extern "C" fn n_877dc0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_1015(&mut mem(), w, a, s)
    }
    /// `Native_4008_FilterAdd(worker, args, stack)`.
    Native_4008_FilterAdd @ 0x00a7_7e30 => extern "C" fn n_a77e30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4008(&mut mem(), w, a, s)
    }
    /// `Native_4009_FilterSub(worker, args, stack)`.
    Native_4009_FilterSub @ 0x00a7_7e80 => extern "C" fn n_a77e80(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4009(&mut mem(), w, a, s)
    }
    /// `Native_5007_ChrSetScale(worker, args, stack)`.
    Native_5007_ChrSetScale @ 0x00a7_89a0 => extern "C" fn n_a789a0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5007(&mut mem(), w, a, s)
    }
    /// `Native_5049_ChrWaitMotionOf(worker, args, stack)`.
    Native_5049_ChrWaitMotionOf @ 0x00a7_85f0 => extern "C" fn n_a785f0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5049(&mut mem(), w, a, s)
    }
    /// `Native_5049_ChrWaitMotionOf_Poll(worker, args, stack)`.
    Native_5049_ChrWaitMotionOf_Poll @ 0x00a7_8620 => extern "C" fn n_a78620(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5049_poll(&mut mem(), w, a, s)
    }
    /// `Native_501f_ChrMotionActive(worker, args, stack)`.
    Native_501f_ChrMotionActive @ 0x00a7_9450 => extern "C" fn n_a79450(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_501f(&mut mem(), w, a, s)
    }
    /// `Native_504a_ChrMotionActiveOf(worker, args, stack)`.
    Native_504a_ChrMotionActiveOf @ 0x00a7_8640 => extern "C" fn n_a78640(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_504a(&mut mem(), w, a, s)
    }
    /// `Native_504b_ChrResetMotionOf(worker, args, stack)`.
    Native_504b_ChrResetMotionOf @ 0x00a7_8670 => extern "C" fn n_a78670(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_504b(&mut mem(), w, a, s)
    }
    /// `Native_5081_ChrSetChocobo(worker, args, stack)`.
    Native_5081_ChrSetChocobo @ 0x00a7_a4a0 => extern "C" fn n_a7a4a0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5081(&mut mem(), w, a, s)
    }
    /// `Native_5082_ChrSetClothEnable(worker, args, stack)`.
    Native_5082_ChrSetClothEnable @ 0x00a7_a4f0 => extern "C" fn n_a7a4f0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5082(&mut mem(), w, a, s)
    }
    /// `Native_5073_ChrSetOverlapHit(worker, args, stack)`.
    Native_5073_ChrSetOverlapHit @ 0x00a7_9d30 => extern "C" fn n_a79d30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5073(&mut mem(), w, a, s)
    }
    /// `Native_5083_ChrSetClipMode(worker, args, stack)`.
    Native_5083_ChrSetClipMode @ 0x00a7_a540 => extern "C" fn n_a7a540(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5083(&mut mem(), w, a, s)
    }
    /// `Native_5060_ChrEnableShade(worker, args, stack)`.
    Native_5060_ChrEnableShade @ 0x00a7_8f00 => extern "C" fn n_a78f00(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5060(&mut mem(), w, a, s)
    }
    /// `Native_508e_ChrSetClothGravity(worker, args, stack)`.
    Native_508e_ChrSetClothGravity @ 0x00a7_a850 => extern "C" fn n_a7a850(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_508e(&mut mem(), w, a, s)
    }
    /// `Native_5067_ChrSetDrawCull(worker, args, stack)`.
    Native_5067_ChrSetDrawCull @ 0x00a7_9580 => extern "C" fn n_a79580(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5067(&mut mem(), w, a, s)
    }
    /// `Native_5071_ChrLoadTextureImage(worker, args, stack)`.
    Native_5071_ChrLoadTextureImage @ 0x00a7_9cc0 => extern "C" fn n_a79cc0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5071(&mut mem(), w, a, s)
    }
    /// `Native_506a_ChrSetPlightEnable(worker, args, stack)`.
    Native_506a_ChrSetPlightEnable @ 0x00a7_9640 => extern "C" fn n_a79640(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_506a(&mut mem(), w, a, s)
    }
    /// `Native_508d_ChrTexAnimTimer(worker, args, stack)`.
    Native_508d_ChrTexAnimTimer @ 0x00a7_a7e0 => extern "C" fn n_a7a7e0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_508d(&mut mem(), w, a, s)
    }
    /// `Native_5069_ChrSetKeepFps(worker, args, stack)`.
    Native_5069_ChrSetKeepFps @ 0x00a7_95e0 => extern "C" fn n_a795e0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5069(&mut mem(), w, a, s)
    }
    /// `Native_505a_ChrSetClutBase(worker, args, stack)`.
    Native_505a_ChrSetClutBase @ 0x00a7_8c60 => extern "C" fn n_a78c60(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_505a(&mut mem(), w, a, s)
    }
    /// `Native_506e_ChrSetBindScale(worker, args, stack)`.
    Native_506e_ChrSetBindScale @ 0x00a7_9b30 => extern "C" fn n_a79b30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_506e(&mut mem(), w, a, s)
    }
    /// `Native_5086_ChrRequestBoneUpdate(worker, args, stack)`.
    Native_5086_ChrRequestBoneUpdate @ 0x00a7_a680 => extern "C" fn n_a7a680(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5086(&mut mem(), w, a, s)
    }
    /// `Native_507c_ChrNeckTargetCam(worker, args, stack)`.
    Native_507c_ChrNeckTargetCam @ 0x00a7_a070 => extern "C" fn n_a7a070(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_507c(&mut mem(), w, a, s)
    }
    /// `Native_5072_ChrGetPNum(worker, args, stack)`.
    Native_5072_ChrGetPNum @ 0x00a7_9cf0 => extern "C" fn n_a79cf0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5072(&mut mem(), w, a, s)
    }
    /// `Native_5035_ChrGetSeqFrame(worker, args, stack)`.
    Native_5035_ChrGetSeqFrame @ 0x00a7_9fa0 => extern "C" fn n_a79fa0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5035(&mut mem(), w, a, s)
    }
    /// `Native_5090(worker, args, stack)`.
    Native_5090 @ 0x00a7_a8d0 => extern "C" fn n_a7a8d0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5090(&mut mem(), w, a, s)
    }
    /// `Native_503f_ChrSetSequence(worker, args, stack)`.
    Native_503f_ChrSetSequence @ 0x00a7_a5a0 => extern "C" fn n_a7a5a0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_503f(&mut mem(), w, a, s)
    }
    /// `Native_5048_ChrSetSequenceOf(worker, args, stack)`.
    Native_5048_ChrSetSequenceOf @ 0x00a7_a940 => extern "C" fn n_a7a940(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5048(&mut mem(), w, a, s)
    }
    /// `Native_507b_ChrNeckTargetPos(worker, args, stack)`.
    Native_507b_ChrNeckTargetPos @ 0x00a7_a000 => extern "C" fn n_a7a000(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_507b(&mut mem(), w, a, s)
    }
    /// `Native_502c_ChrAttachDirect(worker, args, stack)`.
    Native_502c_ChrAttachDirect @ 0x00a7_9bc0 => extern "C" fn n_a79bc0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_502c(&mut mem(), w, a, s)
    }
    /// `Native_5052_ChrAttachDirectOffset(worker, args, stack)`.
    Native_5052_ChrAttachDirectOffset @ 0x00a7_87e0 => extern "C" fn n_a787e0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5052(&mut mem(), w, a, s)
    }
    /// `Native_5026_ChrGetElemPos(worker, args, stack)`.
    Native_5026_ChrGetElemPos @ 0x00a7_96e0 => extern "C" fn n_a796e0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5026(&mut mem(), w, a, s)
    }
    /// `Native_5036_ChrGetOptPos(worker, args, stack)`.
    Native_5036_ChrGetOptPos @ 0x00a7_a120 => extern "C" fn n_a7a120(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5036(&mut mem(), w, a, s)
    }
    /// `Native_5046_ChrPlayMotionOf(worker, args, stack)`.
    Native_5046_ChrPlayMotionOf @ 0x00a7_a880 => extern "C" fn n_a7a880(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5046(&mut mem(), w, a, s)
    }
    /// `Native_5089_ChrPlayMotionIncl(worker, args, stack)`.
    Native_5089_ChrPlayMotionIncl @ 0x00a7_a700 => extern "C" fn n_a7a700(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5089(&mut mem(), w, a, s)
    }
    /// `Native_508b_ChrPlayMotionIncl(worker, args, stack)`.
    Native_508b_ChrPlayMotionIncl @ 0x00a7_a760 => extern "C" fn n_a7a760(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_508b(&mut mem(), w, a, s)
    }
    /// `Native_5019_ChrPlayMotionMarks(worker, args, stack)`.
    Native_5019_ChrPlayMotionMarks @ 0x00a7_9220 => extern "C" fn n_a79220(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5019(&mut mem(), w, a, s)
    }
    /// `Native_500c(worker, args, stack)`.
    Native_500c @ 0x00a7_8d30 => extern "C" fn n_a78d30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_500c(&mut mem(), w, a, s)
    }
    /// `Native_400e_EnableCalcMap(worker, args, stack)`.
    Native_400e_EnableCalcMap @ 0x00a7_7f00 => extern "C" fn n_a77f00(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_400e(&mut mem(), w, a, s)
    }
    /// `Native_400f_EnableDrawMap(worker, args, stack)`.
    Native_400f_EnableDrawMap @ 0x00a7_7f20 => extern "C" fn n_a77f20(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_400f(&mut mem(), w, a, s)
    }
    /// `Native_4045_PlayerSetDiving(worker, args, stack)`.
    Native_4045_PlayerSetDiving @ 0x00a7_7ce0 => extern "C" fn n_a77ce0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4045(&mut mem(), w, a, s)
    }
    /// `Native_401b_AccSetDrawMap(worker, args, stack)`.
    Native_401b_AccSetDrawMap @ 0x00a7_81d0 => extern "C" fn n_a781d0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_401b(&mut mem(), w, a, s)
    }
    /// `Native_401c_AccEnableBG(worker, args, stack)`.
    Native_401c_AccEnableBG @ 0x00a7_81f0 => extern "C" fn n_a781f0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_401c(&mut mem(), w, a, s)
    }
    /// `Native_4023_PLightAllocate(worker, args, stack)`.
    Native_4023_PLightAllocate @ 0x00a7_8340 => extern "C" fn n_a78340(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4023(&mut mem(), w, a, s)
    }
    /// `Native_403a_AccSetDrawChr(worker, args, stack)`.
    Native_403a_AccSetDrawChr @ 0x00a7_7b10 => extern "C" fn n_a77b10(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_403a(&mut mem(), w, a, s)
    }
    /// `Native_4046_SetShadowSemi(worker, args, stack)`.
    Native_4046_SetShadowSemi @ 0x00a7_7d00 => extern "C" fn n_a77d00(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_4022_SetKeepFps(worker, args, stack)`.
    Native_4022_SetKeepFps @ 0x00a7_8320 => extern "C" fn n_a78320(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4022(&mut mem(), w, a, s)
    }
    /// `Native_4043_SetShadowDir(worker, args, stack)`.
    Native_4043_SetShadowDir @ 0x00a7_7c80 => extern "C" fn n_a77c80(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4043(&mut mem(), w, a, s)
    }
    /// `Native_402a_PLightSetEnable(worker, args, stack)`.
    Native_402a_PLightSetEnable @ 0x00a7_85b0 => extern "C" fn n_a785b0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_402a(&mut mem(), w, a, s)
    }
    /// `Native_4036_GlobalSetWind(worker, args, stack)`.
    Native_4036_GlobalSetWind @ 0x00a7_7a10 => extern "C" fn n_a77a10(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4036(&mut mem(), w, a, s)
    }
    /// `Native_403e_PlayerSetDirFix(worker, args, stack)`.
    Native_403e_PlayerSetDirFix @ 0x00a7_7be0 => extern "C" fn n_a77be0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_403e(&mut mem(), w, a, s)
    }
    /// `Native_403c_SetWaterLimit(worker, args, stack)`.
    Native_403c_SetWaterLimit @ 0x00a7_7b60 => extern "C" fn n_a77b60(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_403c(&mut mem(), w, a, s)
    }
    /// `Native_4025_PLightDefine(worker, args, stack)`.
    Native_4025_PLightDefine @ 0x00a7_8370 => extern "C" fn n_a78370(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_101c_SetVarSlot(worker, args, stack)`.
    Native_101c_SetVarSlot @ 0x0087_80f0 => extern "C" fn n_8780f0(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_601b_CamIsMoving(worker, args, stack)`.
    Native_601b_CamIsMoving @ 0x007b_9ca0 => extern "C" fn n_7b9ca0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_601b(&mut mem(), w, a, s)
    }
    /// `Native_6061_ScreenResetShake(worker, args, stack)`.
    Native_6061_ScreenResetShake @ 0x007b_9960 => extern "C" fn n_7b9960(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6061(&mut mem(), w, a, s)
    }
    /// `Native_604e_CamSetScreenVector(worker, args, stack)`.
    Native_604e_CamSetScreenVector @ 0x007b_8ad0 => extern "C" fn n_7b8ad0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_604e(&mut mem(), w, a, s)
    }
    /// `Native_6050_CamSetScreenVector2(worker, args, stack)`.
    Native_6050_CamSetScreenVector2 @ 0x007b_8ba0 => extern "C" fn n_7b8ba0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6050(&mut mem(), w, a, s)
    }
    /// `Native_6051_ScreenSetLink1(worker, args, stack)`.
    Native_6051_ScreenSetLink1 @ 0x007b_8c30 => extern "C" fn n_7b8c30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6051(&mut mem(), w, a, s)
    }
    /// `Native_6052_ScreenSetLink0(worker, args, stack)`.
    Native_6052_ScreenSetLink0 @ 0x007b_8c00 => extern "C" fn n_7b8c00(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6052(&mut mem(), w, a, s)
    }
    /// `Native_6055_ScreenMoveAcc(worker, args, stack)`.
    Native_6055_ScreenMoveAcc @ 0x007b_8cc0 => extern "C" fn n_7b8cc0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6055(&mut mem(), w, a, s)
    }
    /// `Native_6058_ScreenMoveAccSide1(worker, args, stack)`.
    Native_6058_ScreenMoveAccSide1 @ 0x007b_8d70 => extern "C" fn n_7b8d70(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6058(&mut mem(), w, a, s)
    }
    /// `Native_604f_CamSetScreenCamera(worker, args, stack)`.
    Native_604f_CamSetScreenCamera @ 0x007b_8b80 => extern "C" fn n_7b8b80(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_604f(&mut mem(), w, a, s)
    }
    /// `Native_607b_ScreenMoveWait(worker, args, stack)`.
    Native_607b_ScreenMoveWait @ 0x007b_8e40 => extern "C" fn n_7b8e40(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_607b(&mut mem(), w, a, s)
    }
    /// `Native_607b_ScreenMoveWait_Poll(worker, args, stack)`.
    Native_607b_ScreenMoveWait_Poll @ 0x007b_8e60 => extern "C" fn n_7b8e60(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_607b_poll(&mut mem(), w, a, s)
    }
    /// `Native_6069_CamFocusShakeKind3(worker, args, stack)`.
    Native_6069_CamFocusShakeKind3 @ 0x007b_8ff0 => extern "C" fn n_7b8ff0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6069(&mut mem(), w, a, s)
    }
    /// `Native_6067_CamEyeShakeKind2(worker, args, stack)`.
    Native_6067_CamEyeShakeKind2 @ 0x007b_8fc0 => extern "C" fn n_7b8fc0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6067(&mut mem(), w, a, s)
    }
    /// `Native_6081_CamFocusShakeKind7(worker, args, stack)`.
    Native_6081_CamFocusShakeKind7 @ 0x007b_92c0 => extern "C" fn n_7b92c0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6081(&mut mem(), w, a, s)
    }
    /// `Native_6082_CamEyeShakeKind7(worker, args, stack)`.
    Native_6082_CamEyeShakeKind7 @ 0x007b_9300 => extern "C" fn n_7b9300(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6082(&mut mem(), w, a, s)
    }
    /// `Native_6066_CamFocusShakeKind2(worker, args, stack)`.
    Native_6066_CamFocusShakeKind2 @ 0x007b_8fa0 => extern "C" fn n_7b8fa0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6066(&mut mem(), w, a, s)
    }
    /// `Native_606c_CamFocusShakeKind4(worker, args, stack)`.
    Native_606c_CamFocusShakeKind4 @ 0x007b_9150 => extern "C" fn n_7b9150(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_606c(&mut mem(), w, a, s)
    }
    /// `Native_6033(worker, args, stack)`.
    Native_6033 @ 0x007b_8150 => extern "C" fn n_7b8150(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6033(&mut mem(), w, a, s)
    }
    /// `Native_6074_ScreenShakeKind6(worker, args, stack)`.
    Native_6074_ScreenShakeKind6 @ 0x007b_9610 => extern "C" fn n_7b9610(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_6074(&mut mem(), w, a, s)
    }
    /// `Native_606e_ScreenShakeKind4(worker, args, stack)`.
    Native_606e_ScreenShakeKind4 @ 0x007b_94e0 => extern "C" fn n_7b94e0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_606e(&mut mem(), w, a, s)
    }
    /// `Native_605e_ScreenShakeKind1(worker, args, stack)`.
    Native_605e_ScreenShakeKind1 @ 0x007b_9420 => extern "C" fn n_7b9420(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_5028_ChrSetChgRun(worker, args, stack)`.
    Native_5028_ChrSetChgRun @ 0x00a7_99d0 => extern "C" fn n_a799d0(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_5028(&mut mem(), w, a, s)
    }
    /// `Native_505e_ChrGetSpeed(worker, args, stack)`.
    Native_505e_ChrGetSpeed @ 0x00a7_8e20 => extern "C" fn n_a78e20(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_505e(&mut mem(), w, a, s)
    }
    /// `Native_4034_SetMapAlphaFast(worker, args, stack)`.
    Native_4034_SetMapAlphaFast @ 0x00a7_79e0 => extern "C" fn n_a779e0(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_4034(&mut mem(), w, a, s)
    }
    /// `Native_4039_AccSetClear(worker, args, stack)`.
    Native_4039_AccSetClear @ 0x00a7_7ac0 => extern "C" fn n_a77ac0(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_4039(&mut mem(), w, a, s)
    }
    /// `Native_504d_ChrAllSetThickness(worker, args, stack)`.
    Native_504d_ChrAllSetThickness @ 0x00a7_86e0 => extern "C" fn n_a786e0(w: u32, a: u32, s: u32) -> f64 {
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
    /// `Native_504f_ChrDataReadSync(worker, args, stack)`.
    Native_504f_ChrDataReadSync @ 0x00a7_9340 => extern "C" fn n_a79340(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_504f(&mut mem(), w, a, s)
    }
    /// `Native_504f_ChrDataReadSync_Poll(worker, args, stack)`.
    Native_504f_ChrDataReadSync_Poll @ 0x00a7_9380 => extern "C" fn n_a79380(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_504f_poll(&mut mem(), w, a, s)
    }
    /// `Native_5061_ChrReadMgrp(worker, args, stack)`.
    Native_5061_ChrReadMgrp @ 0x00a7_8f80 => extern "C" fn n_a78f80(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5061(&mut mem(), w, a, s)
    }
    /// `Native_401a_AccSetAlpha(worker, args, stack)`.
    Native_401a_AccSetAlpha @ 0x00a7_81a0 => extern "C" fn n_a781a0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_401a(&mut mem(), w, a, s)
    }
    /// `Native_4017_IdSetMapTable(worker, args, stack)`.
    Native_4017_IdSetMapTable @ 0x00a7_8130 => extern "C" fn n_a78130(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4017(&mut mem(), w, a, s)
    }
    /// `Native_5076_ChrSetAutoTalk(worker, args, stack)`.
    Native_5076_ChrSetAutoTalk @ 0x00a7_9e30 => extern "C" fn n_a79e30(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_5015_ChrSetShadeColor(worker, args, stack)`.
    Native_5015_ChrSetShadeColor @ 0x00a7_8fb0 => extern "C" fn n_a78fb0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5015(&mut mem(), w, a, s)
    }
    /// `Native_503d_ChrDisposeData(worker, args, stack)`.
    Native_503d_ChrDisposeData @ 0x00a7_a520 => extern "C" fn n_a7a520(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_503d(&mut mem(), w, a, s)
    }
    /// `Native_101a_DistanceXZ(worker, args, stack)`.
    Native_101a_DistanceXZ @ 0x0087_7ff0 => extern "C" fn n_877ff0(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_101a(&mut mem(), w, a, s)
    }
    /// `Native_5006_ChrSetScaleAll(worker, args, stack)`.
    Native_5006_ChrSetScaleAll @ 0x00a7_8a40 => extern "C" fn n_a78a40(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5006(&mut mem(), w, a, s)
    }
    /// `Native_4019_AccRequestCurrent(worker, args, stack)`.
    Native_4019_AccRequestCurrent @ 0x00a7_8180 => extern "C" fn n_a78180(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4019(&mut mem(), w, a, s)
    }
    /// `Native_5013_ChrSetFieldMode(worker, args, stack)`.
    Native_5013_ChrSetFieldMode @ 0x00a7_8ec0 => extern "C" fn n_a78ec0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5013(&mut mem(), w, a, s)
    }
    /// `Native_SetControlAngle(worker, args, stack)`.
    Native_SetControlAngle @ 0x00a7_7c30 => extern "C" fn n_a77c30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_set_control_angle(&mut mem(), w, a, s)
    }
    /// `Native_5056_ChrSetBoneUpdate(worker, args, stack)`.
    Native_5056_ChrSetBoneUpdate @ 0x00a7_8a80 => extern "C" fn n_a78a80(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5056(&mut mem(), w, a, s)
    }
    /// `Native_4015_SetFocus(worker, args, stack)`.
    Native_4015_SetFocus @ 0x00a7_80c0 => extern "C" fn n_a780c0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4015(&mut mem(), w, a, s)
    }
    /// `Native_503e_ChrDisposeData(worker, args, stack)`.
    Native_503e_ChrDisposeData @ 0x00a7_a570 => extern "C" fn n_a7a570(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_503e(&mut mem(), w, a, s)
    }
    /// `Native_101b_Distance3D(worker, args, stack)`.
    Native_101b_Distance3D @ 0x0087_8060 => extern "C" fn n_878060(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_101b(&mut mem(), w, a, s)
    }
    /// `Native_5054_ChrSetPartHide(worker, args, stack)`.
    Native_5054_ChrSetPartHide @ 0x00a7_8960 => extern "C" fn n_a78960(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5054(&mut mem(), w, a, s)
    }
    /// `Native_5063_ChrSyncSystemMgrp(worker, args, stack)`.
    Native_5063_ChrSyncSystemMgrp @ 0x00a7_9160 => extern "C" fn n_a79160(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5063(&mut mem(), w, a, s)
    }
    /// `Native_5063_ChrSyncSystemMgrp_Poll(worker, args, stack)`.
    Native_5063_ChrSyncSystemMgrp_Poll @ 0x00a7_91a0 => extern "C" fn n_a791a0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5063_poll(&mut mem(), w, a, s)
    }
    /// `Native_4014_EnFocus(worker, args, stack)`.
    Native_4014_EnFocus @ 0x00a7_80a0 => extern "C" fn n_a780a0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4014(&mut mem(), w, a, s)
    }
    /// `Native_5079_ChrSetNeckEnable(worker, args, stack)`.
    Native_5079_ChrSetNeckEnable @ 0x00a7_9f70 => extern "C" fn n_a79f70(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5079(&mut mem(), w, a, s)
    }
    /// `Native_5047_ChrSetSequenceOf(worker, args, stack)`.
    Native_5047_ChrSetSequenceOf @ 0x00a7_a900 => extern "C" fn n_a7a900(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5047(&mut mem(), w, a, s)
    }
    /// `Native_507a_ChrSetNeckTarget(worker, args, stack)`.
    Native_507a_ChrSetNeckTarget @ 0x00a7_9fc0 => extern "C" fn n_a79fc0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_507a(&mut mem(), w, a, s)
    }
    /// `Native_5078_ChrSetTextureAnim(worker, args, stack)`.
    Native_5078_ChrSetTextureAnim @ 0x00a7_9ef0 => extern "C" fn n_a79ef0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5078(&mut mem(), w, a, s)
    }
    /// `Native_4016_PlayerSetSpeed(worker, args, stack)`.
    Native_4016_PlayerSetSpeed @ 0x00a7_80f0 => extern "C" fn n_a780f0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4016(&mut mem(), w, a, s)
    }
    /// `Native_500b_ChrSetGroundHeight(worker, args, stack)`.
    Native_500b_ChrSetGroundHeight @ 0x00a7_8b90 => extern "C" fn n_a78b90(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_500b(&mut mem(), w, a, s)
    }
    /// `Native_506f_ChrSetHideCalc(worker, args, stack)`.
    Native_506f_ChrSetHideCalc @ 0x00a7_9b90 => extern "C" fn n_a79b90(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_506f(&mut mem(), w, a, s)
    }
    /// `Native_5058_ChrAllBoneUpdate(worker, args, stack)`.
    Native_5058_ChrAllBoneUpdate @ 0x00a7_8b70 => extern "C" fn n_a78b70(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5058(&mut mem(), w, a, s)
    }
    /// `Native_503a_ChrSetClipZ(worker, args, stack)`.
    Native_503a_ChrSetClipZ @ 0x00a7_a3d0 => extern "C" fn n_a7a3d0(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_503a(&mut mem(), w, a, s)
    }
    /// `Native_1013_WrapAngle(worker, args, stack)`.
    Native_1013_WrapAngle @ 0x0087_7d30 => extern "C" fn n_877d30(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_1013(&mut mem(), w, a, s)
    }
    /// `Atel_Sg_Flash(worker, args, stack)`.
    Atel_Sg_Flash @ 0x00a7_7d30 => extern "C" fn n_a77d30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::atel_sg_flash(&mut mem(), w, a, s)
    }
    /// `Native_5044_ChrPlayMotionOf(worker, args, stack)`.
    Native_5044_ChrPlayMotionOf @ 0x00a7_a810 => extern "C" fn n_a7a810(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5044(&mut mem(), w, a, s)
    }
    /// `Native_504c_ChrMotionSpeedOf(worker, args, stack)`.
    Native_504c_ChrMotionSpeedOf @ 0x00a7_86a0 => extern "C" fn n_a786a0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_504c(&mut mem(), w, a, s)
    }
    /// `Native_5018_ChrSetMotionSpeed(worker, args, stack)`.
    Native_5018_ChrSetMotionSpeed @ 0x00a7_91e0 => extern "C" fn n_a791e0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_5018(&mut mem(), w, a, s)
    }
    /// `AtelCall_Math_Cos_RF(worker, args, stack)`.
    AtelCall_Math_Cos_RF @ 0x0087_7aa0 => extern "C" fn n_877aa0(w: u32, a: u32, s: u32) -> f64 {
        native_chr::atel_call_math_cos_rf(&mut mem(), w, a, s)
    }
    /// `Native_4007_FadeOutWhite(worker, args, stack)`.
    Native_4007_FadeOutWhite @ 0x00a7_7de0 => extern "C" fn n_a77de0(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_4006_FadeInWhite(worker, args, stack)`.
    Native_4006_FadeInWhite @ 0x00a7_7dc0 => extern "C" fn n_a77dc0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4006(&mut mem(), w, a, s)
    }
    /// `Native_4001_SetSuimen(worker, args, stack)`.
    Native_4001_SetSuimen @ 0x00a7_7b90 => extern "C" fn n_a77b90(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4001(&mut mem(), w, a, s)
    }
    /// `Native_505f_ChrSetMotionLoop(worker, args, stack)`.
    Native_505f_ChrSetMotionLoop @ 0x00a7_8e90 => extern "C" fn n_a78e90(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_505f(&mut mem(), w, a, s)
    }
    /// `Native_500d_ChrSetHideEvent(worker, args, stack)`.
    Native_500d_ChrSetHideEvent @ 0x00a7_8df0 => extern "C" fn n_a78df0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_500d(&mut mem(), w, a, s)
    }
    /// `Native_503c_ChrDisposeMgrp(worker, args, stack)`.
    Native_503c_ChrDisposeMgrp @ 0x00a7_a4d0 => extern "C" fn n_a7a4d0(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_5065_ChrReadEventMgrp(worker, args, stack)`.
    Native_5065_ChrReadEventMgrp @ 0x00a7_9420 => extern "C" fn n_a79420(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_7076(worker, args, stack)`.
    Native_7076 @ 0x007a_56f0 => extern "C" fn n_7a56f0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7076(&mut mem(), w, a, s)
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
    /// `Native_015e_MinimapSetForceDraw(worker, args, stack)`.
    Native_015e_MinimapSetForceDraw @ 0x0085_7980 => extern "C" fn n_857980(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_015e(&mut mem(), w, a, s)
    }
    /// `Native_0119_ChrGetEquipModel(worker, args, stack)`.
    Native_0119_ChrGetEquipModel @ 0x0085_bd70 => extern "C" fn n_85bd70(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_0091_GetYaw(worker, args, stack)`.
    Native_0091_GetYaw @ 0x0085_e0b0 => extern "C" fn n_85e0b0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0091(&mut mem(), w, a, s)
    }
    /// `Native_0004(worker, args, stack)`.
    Native_0004 @ 0x0085_ecb0 => extern "C" fn n_85ecb0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0004(&mut mem(), w, a, s)
    }
    /// `Native_00e0_SndSepStop(worker, args, stack)`.
    Native_00e0_SndSepStop @ 0x0085_5f60 => extern "C" fn n_855f60(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_0180_CurMusicVolumeTranslate(worker, args, stack)`.
    Native_0180_CurMusicVolumeTranslate @ 0x0085_c000 => extern "C" fn n_85c000(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_00aa_SetBattleStatus(worker, args, stack)`.
    Native_00aa_SetBattleStatus @ 0x0085_7a60 => extern "C" fn n_857a60(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00aa(&mut mem(), w, a, s)
    }
    /// `Native_MusicPlay(worker, args, stack)`.
    Native_MusicPlay @ 0x0085_96d0 => extern "C" fn n_8596d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_music_play(&mut mem(), w, a, s)
    }
    /// `Native_0007_ChrDetachModel(worker, args, stack)`.
    Native_0007_ChrDetachModel @ 0x0085_c8c0 => extern "C" fn n_85c8c0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0007(&mut mem(), w, a, s)
    }
    /// `Native_012a(worker, args, stack)`.
    Native_012a @ 0x0085_c890 => extern "C" fn n_85c890(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_012a(&mut mem(), w, a, s)
    }
    /// `Native_0133_WorkerTriggerSetActive(worker, args, stack)`.
    Native_0133_WorkerTriggerSetActive @ 0x0086_0290 => extern "C" fn n_860290(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0133(&mut mem(), w, a, s)
    }
    /// `Native_016f_BackupEventJump(worker, args, stack)`.
    Native_016f_BackupEventJump @ 0x0085_c430 => extern "C" fn n_85c430(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_016f(&mut mem(), w, a, s)
    }
    /// `Native_0236_MsgGetText(worker, args, stack)`.
    Native_0236_MsgGetText @ 0x0086_0350 => extern "C" fn n_860350(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_01ba_ChrGetEquipModel(worker, args, stack)`.
    Native_01ba_ChrGetEquipModel @ 0x0085_beb0 => extern "C" fn n_85beb0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01ba(&mut mem(), w, a, s)
    }
    /// `Native_015b_GiveEventReward_Poll(worker, args, stack)`.
    Native_015b_GiveEventReward_Poll @ 0x0085_af70 => extern "C" fn n_85af70(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_00f6_WaitWorkerSignal(worker, args, stack)`.
    Native_00f6_WaitWorkerSignal @ 0x0085_7df0 => extern "C" fn n_857df0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00f6(&mut mem(), w, a, s)
    }
    /// `Native_00f6_WaitWorkerSignal_Poll(worker, args, stack)`.
    Native_00f6_WaitWorkerSignal_Poll @ 0x0085_7ff0 => extern "C" fn n_857ff0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00f6_poll(&mut mem(), w, a, s)
    }
    /// `Native_0055(worker, args, stack)`.
    Native_0055 @ 0x0085_fa20 => extern "C" fn n_85fa20(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0055(&mut mem(), w, a, s)
    }
    /// `Native_00a4_WorkerDistance(worker, args, stack)`.
    Native_00a4_WorkerDistance @ 0x0085_66c0 => extern "C" fn n_8566c0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_00a4(&mut mem(), w, a, s)
    }
    /// `Native_00a5_WorkerDistanceXZ(worker, args, stack)`.
    Native_00a5_WorkerDistanceXZ @ 0x0085_6990 => extern "C" fn n_856990(w: u32, a: u32, s: u32) -> f64 {
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
    /// `Native_017a_SetEffectFlag(worker, args, stack)`.
    Native_017a_SetEffectFlag @ 0x0085_d210 => extern "C" fn n_85d210(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_004c_PadIsHeld(worker, args, stack)`.
    Native_004c_PadIsHeld @ 0x0085_e130 => extern "C" fn n_85e130(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_004c(&mut mem(), w, a, s)
    }
    /// `Native_004d_PadIsPressed(worker, args, stack)`.
    Native_004d_PadIsPressed @ 0x0085_e820 => extern "C" fn n_85e820(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_004d(&mut mem(), w, a, s)
    }
    /// `Native_SetMapMarkerPos(worker, args, stack)`.
    Native_SetMapMarkerPos @ 0x0085_c2d0 => extern "C" fn n_85c2d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_set_map_marker_pos(&mut mem(), w, a, s)
    }
    /// `Native_0103_MusicClear(worker, args, stack)`.
    Native_0103_MusicClear @ 0x0085_93a0 => extern "C" fn n_8593a0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0103(&mut mem(), w, a, s)
    }
    /// `Native_016d_MusicStop(worker, args, stack)`.
    Native_016d_MusicStop @ 0x0085_95b0 => extern "C" fn n_8595b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_016d(&mut mem(), w, a, s)
    }
    /// `Native_01e9_MsgCloseWait(worker, args, stack)`.
    Native_01e9_MsgCloseWait @ 0x0085_90f0 => extern "C" fn n_8590f0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01e9(&mut mem(), w, a, s)
    }
    /// `Native_01e9_MsgCloseWait_Poll(worker, args, stack)`.
    Native_01e9_MsgCloseWait_Poll @ 0x0085_92c0 => extern "C" fn n_8592c0(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_014a_SetBattleEndStatus(worker, args, stack)`.
    Native_014a_SetBattleEndStatus @ 0x0085_8740 => extern "C" fn n_858740(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_014a(&mut mem(), w, a, s)
    }
    /// `Native_0215_SetLegendAbility(worker, args, stack)`.
    Native_0215_SetLegendAbility @ 0x0085_d170 => extern "C" fn n_85d170(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0215(&mut mem(), w, a, s)
    }
    /// `Native_0132_WorkerTriggerSetActive(worker, args, stack)`.
    Native_0132_WorkerTriggerSetActive @ 0x0085_fe30 => extern "C" fn n_85fe30(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0132(&mut mem(), w, a, s)
    }
    /// `Native_00f2_MinimapSetDrawFlag(worker, args, stack)`.
    Native_00f2_MinimapSetDrawFlag @ 0x0085_7850 => extern "C" fn n_857850(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00f2(&mut mem(), w, a, s)
    }
    /// `Native_015d_MinimapSetBlizFlag(worker, args, stack)`.
    Native_015d_MinimapSetBlizFlag @ 0x0085_7a80 => extern "C" fn n_857a80(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_015d(&mut mem(), w, a, s)
    }
    /// `Native_0225_SetShopRatio(worker, args, stack)`.
    Native_0225_SetShopRatio @ 0x0085_ea10 => extern "C" fn n_85ea10(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0225(&mut mem(), w, a, s)
    }
    /// `Native_012d_SaveCounterGet(worker, args, stack)`.
    Native_012d_SaveCounterGet @ 0x0085_df80 => extern "C" fn n_85df80(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_012d(&mut mem(), w, a, s)
    }
    /// `Native_00ce(worker, args, stack)`.
    Native_00ce @ 0x0085_c2b0 => extern "C" fn n_85c2b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00ce(&mut mem(), w, a, s)
    }
    /// `Native_017e_MusicCrossStart(worker, args, stack)`.
    Native_017e_MusicCrossStart @ 0x0085_d830 => extern "C" fn n_85d830(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_017e(&mut mem(), w, a, s)
    }
    /// `Native_0025(worker, args, stack)`.
    Native_0025 @ 0x0085_9d80 => extern "C" fn n_859d80(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0025(&mut mem(), w, a, s)
    }
    /// `Native_0197_WorkerAttachDobj(worker, args, stack)`.
    Native_0197_WorkerAttachDobj @ 0x0085_edd0 => extern "C" fn n_85edd0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0197(&mut mem(), w, a, s)
    }
    /// `Native_021b_CheckCaptMonster(worker, args, stack)`.
    Native_021b_CheckCaptMonster @ 0x0085_d8c0 => extern "C" fn n_85d8c0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_021b(&mut mem(), w, a, s)
    }
    /// `Native_0128_WorkerInRadius(worker, args, stack)`.
    Native_0128_WorkerInRadius @ 0x0085_d7b0 => extern "C" fn n_85d7b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0128(&mut mem(), w, a, s)
    }
    /// `Native_0069(worker, args, stack)`.
    Native_0069 @ 0x0085_89d0 => extern "C" fn n_8589d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0069(&mut mem(), w, a, s)
    }
    /// `Native_0139_MsgChoice(worker, args, stack)`.
    Native_0139_MsgChoice @ 0x0085_f730 => extern "C" fn n_85f730(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0139(&mut mem(), w, a, s)
    }
    /// `Native_0139_MsgChoice_Poll(worker, args, stack)`.
    Native_0139_MsgChoice_Poll @ 0x0085_f850 => extern "C" fn n_85f850(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0139_poll(&mut mem(), w, a, s)
    }
    /// `Native_0139_MsgChoice_Result(worker, args, stack)`.
    Native_0139_MsgChoice_Result @ 0x0085_f900 => extern "C" fn n_85f900(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_01da_VibrateStartLevel3(worker, args, stack)`.
    Native_01da_VibrateStartLevel3 @ 0x0085_6a80 => extern "C" fn n_856a80(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_0093_GetWorkerYaw(worker, args, stack)`.
    Native_0093_GetWorkerYaw @ 0x0085_e8f0 => extern "C" fn n_85e8f0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0093(&mut mem(), w, a, s)
    }
    /// `Native_0130_SaveCounterSet(worker, args, stack)`.
    Native_0130_SaveCounterSet @ 0x0085_e410 => extern "C" fn n_85e410(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0130(&mut mem(), w, a, s)
    }
    /// `Native_017f_CurMusicVolumeDirect(worker, args, stack)`.
    Native_017f_CurMusicVolumeDirect @ 0x0085_bd10 => extern "C" fn n_85bd10(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_017f(&mut mem(), w, a, s)
    }
    /// `Native_0136_GiveGil(worker, args, stack)`.
    Native_0136_GiveGil @ 0x0085_ee40 => extern "C" fn n_85ee40(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0136(&mut mem(), w, a, s)
    }
    /// `Native_0137_PayGil(worker, args, stack)`.
    Native_0137_PayGil @ 0x0085_ef80 => extern "C" fn n_85ef80(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0137(&mut mem(), w, a, s)
    }
    /// `Native_0157_SndFootOffSlot(worker, args, stack)`.
    Native_0157_SndFootOffSlot @ 0x0085_9dc0 => extern "C" fn n_859dc0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0157(&mut mem(), w, a, s)
    }
    /// `Native_0155_SndFootOnSlot(worker, args, stack)`.
    Native_0155_SndFootOnSlot @ 0x0085_9d00 => extern "C" fn n_859d00(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0155(&mut mem(), w, a, s)
    }
    /// `Native_0100_SndSepVolumeTranslate(worker, args, stack)`.
    Native_0100_SndSepVolumeTranslate @ 0x0086_0460 => extern "C" fn n_860460(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0100(&mut mem(), w, a, s)
    }
    /// `Native_016b_MusicVolumeTranslate(worker, args, stack)`.
    Native_016b_MusicVolumeTranslate @ 0x0085_be70 => extern "C" fn n_85be70(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_01a4_DobjSetMimeParam1(worker, args, stack)`.
    Native_01a4_DobjSetMimeParam1 @ 0x0085_7360 => extern "C" fn n_857360(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01a4(&mut mem(), w, a, s)
    }
    /// `Native_0121(worker, args, stack)`.
    Native_0121 @ 0x0085_d1d0 => extern "C" fn n_85d1d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0121(&mut mem(), w, a, s)
    }
    /// `Native_0122_PadGetAnalog(worker, args, stack)`.
    Native_0122_PadGetAnalog @ 0x0085_d310 => extern "C" fn n_85d310(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0122(&mut mem(), w, a, s)
    }
    /// `Native_01c9_DisposeAnimDobj(worker, args, stack)`.
    Native_01c9_DisposeAnimDobj @ 0x0086_0730 => extern "C" fn n_860730(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01c9(&mut mem(), w, a, s)
    }
    /// `Native_009a(worker, args, stack)`.
    Native_009a @ 0x0086_0390 => extern "C" fn n_860390(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_009a(&mut mem(), w, a, s)
    }
    /// `Native_00de_EventEffectWaitEnd_Poll(worker, args, stack)`.
    Native_00de_EventEffectWaitEnd_Poll @ 0x0085_f280 => extern "C" fn n_85f280(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00de_poll(&mut mem(), w, a, s)
    }
    /// `Native_0094(worker, args, stack)`.
    Native_0094 @ 0x0085_f0b0 => extern "C" fn n_85f0b0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0094(&mut mem(), w, a, s)
    }
    /// `Native_01eb_VibrateSetCommon(worker, args, stack)`.
    Native_01eb_VibrateSetCommon @ 0x0085_8070 => extern "C" fn n_858070(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01eb(&mut mem(), w, a, s)
    }
    /// `Native_021a_CheckCaptMonster(worker, args, stack)`.
    Native_021a_CheckCaptMonster @ 0x0085_d780 => extern "C" fn n_85d780(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_021a(&mut mem(), w, a, s)
    }
    /// `Native_0207_BindEffBreathOn(worker, args, stack)`.
    Native_0207_BindEffBreathOn @ 0x0085_f010 => extern "C" fn n_85f010(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0207(&mut mem(), w, a, s)
    }
    /// `Native_0090(worker, args, stack)`.
    Native_0090 @ 0x0085_e270 => extern "C" fn n_85e270(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0090(&mut mem(), w, a, s)
    }
    /// `Native_019f_IsCtrlRunning(worker, args, stack)`.
    Native_019f_IsCtrlRunning @ 0x0085_67a0 => extern "C" fn n_8567a0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_019f(&mut mem(), w, a, s)
    }
    /// `Native_0196_FpBindAttached(worker, args, stack)`.
    Native_0196_FpBindAttached @ 0x0085_fce0 => extern "C" fn n_85fce0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0196(&mut mem(), w, a, s)
    }
    /// `Native_0109(worker, args, stack)`.
    Native_0109 @ 0x0085_a870 => extern "C" fn n_85a870(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0109(&mut mem(), w, a, s)
    }
    /// `Native_0108_Snd3dSetPos(worker, args, stack)`.
    Native_0108_Snd3dSetPos @ 0x0085_a370 => extern "C" fn n_85a370(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0108(&mut mem(), w, a, s)
    }
    /// `Native_00db_EventEffectRead(worker, args, stack)`.
    Native_00db_EventEffectRead @ 0x0085_e9e0 => extern "C" fn n_85e9e0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00db(&mut mem(), w, a, s)
    }
    /// `Native_00db_EventEffectRead_Poll(worker, args, stack)`.
    Native_00db_EventEffectRead_Poll @ 0x0085_eaf0 => extern "C" fn n_85eaf0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00db_poll(&mut mem(), w, a, s)
    }
    /// `Native_011d(worker, args, stack)`.
    Native_011d @ 0x0085_c650 => extern "C" fn n_85c650(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_011d(&mut mem(), w, a, s)
    }
    /// `Native_00dc_EventEffectStart_Poll(worker, args, stack)`.
    Native_00dc_EventEffectStart_Poll @ 0x0085_ee70 => extern "C" fn n_85ee70(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00dc_poll(&mut mem(), w, a, s)
    }
    /// `Native_00bd(worker, args, stack)`.
    Native_00bd @ 0x0085_9840 => extern "C" fn n_859840(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00bd(&mut mem(), w, a, s)
    }
    /// `Native_01ad_AddMonsterCapture(worker, args, stack)`.
    Native_01ad_AddMonsterCapture @ 0x0085_8b30 => extern "C" fn n_858b30(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_023c_AllHelpEventFlags(worker, args, stack)`.
    Native_023c_AllHelpEventFlags @ 0x0085_c600 => extern "C" fn n_85c600(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_023c(&mut mem(), w, a, s)
    }
    /// `Native_023b_GetHelpEventFlag(worker, args, stack)`.
    Native_023b_GetHelpEventFlag @ 0x0085_c490 => extern "C" fn n_85c490(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_023b(&mut mem(), w, a, s)
    }
    /// `Native_01d0(worker, args, stack)`.
    Native_01d0 @ 0x0085_fbb0 => extern "C" fn n_85fbb0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01d0(&mut mem(), w, a, s)
    }
    /// `Native_01ce_BindEffDrawOn(worker, args, stack)`.
    Native_01ce_BindEffDrawOn @ 0x0085_e7c0 => extern "C" fn n_85e7c0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01ce(&mut mem(), w, a, s)
    }
    /// `Native_01f9(worker, args, stack)`.
    Native_01f9 @ 0x0085_ac50 => extern "C" fn n_85ac50(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01f9(&mut mem(), w, a, s)
    }
    /// `Native_00bf_GetEntranceAtX(worker, args, stack)`.
    Native_00bf_GetEntranceAtX @ 0x0085_9d30 => extern "C" fn n_859d30(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_00bf(&mut mem(), w, a, s)
    }
    /// `Native_00c0_GetEntranceAtY(worker, args, stack)`.
    Native_00c0_GetEntranceAtY @ 0x0085_9e00 => extern "C" fn n_859e00(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_00c0(&mut mem(), w, a, s)
    }
    /// `Native_00c1_GetEntranceAtZ(worker, args, stack)`.
    Native_00c1_GetEntranceAtZ @ 0x0085_9ef0 => extern "C" fn n_859ef0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_00c1(&mut mem(), w, a, s)
    }
    /// `Native_00c2_GetEntranceAtYaw(worker, args, stack)`.
    Native_00c2_GetEntranceAtYaw @ 0x0085_a070 => extern "C" fn n_85a070(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_00c2(&mut mem(), w, a, s)
    }
    /// `Native_017b_ClearEffectFlag(worker, args, stack)`.
    Native_017b_ClearEffectFlag @ 0x0085_d3d0 => extern "C" fn n_85d3d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_017b(&mut mem(), w, a, s)
    }
    /// `Native_00ec_PartyGetSlot(worker, args, stack)`.
    Native_00ec_PartyGetSlot @ 0x0085_6bc0 => extern "C" fn n_856bc0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00ec(&mut mem(), w, a, s)
    }
    /// `Native_01bd_SetWaterBattleEffect(worker, args, stack)`.
    Native_01bd_SetWaterBattleEffect @ 0x0085_9ff0 => extern "C" fn n_859ff0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01bd(&mut mem(), w, a, s)
    }
    /// `Native_0050_PadIsHeldRaw(worker, args, stack)`.
    Native_0050_PadIsHeldRaw @ 0x0085_ec60 => extern "C" fn n_85ec60(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0050(&mut mem(), w, a, s)
    }
    /// `Native_0221(worker, args, stack)`.
    Native_0221 @ 0x0085_e110 => extern "C" fn n_85e110(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0221(&mut mem(), w, a, s)
    }
    /// `Native_0143_BgSepFadeOut(worker, args, stack)`.
    Native_0143_BgSepFadeOut @ 0x0085_7ad0 => extern "C" fn n_857ad0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0143(&mut mem(), w, a, s)
    }
    /// `Native_0144_BgSepFadeIn(worker, args, stack)`.
    Native_0144_BgSepFadeIn @ 0x0085_7d40 => extern "C" fn n_857d40(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0144(&mut mem(), w, a, s)
    }
    /// `Native_025b_UnlockAchievement(worker, args, stack)`.
    Native_025b_UnlockAchievement @ 0x0085_9270 => extern "C" fn n_859270(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_025b(&mut mem(), w, a, s)
    }
    /// `Native_0198_AttachAnimDobj(worker, args, stack)`.
    Native_0198_AttachAnimDobj @ 0x0085_ffa0 => extern "C" fn n_85ffa0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0198(&mut mem(), w, a, s)
    }
    /// `Native_0199_AttachAnimOwnDobj(worker, args, stack)`.
    Native_0199_AttachAnimOwnDobj @ 0x0086_0200 => extern "C" fn n_860200(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0199(&mut mem(), w, a, s)
    }
    /// `Native_0141_MsgSetTextVar(worker, args, stack)`.
    Native_0141_MsgSetTextVar @ 0x0085_7a00 => extern "C" fn n_857a00(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_0237_BuildChoiceText(worker, args, stack)`.
    Native_0237_BuildChoiceText @ 0x0086_04c0 => extern "C" fn n_8604c0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0237(&mut mem(), w, a, s)
    }
    /// `CT_01FC_ret_teachAbilityToPartyMemberSilently(worker, args, stack)`.
    CT_01FC_ret_teachAbilityToPartyMemberSilently @ 0x0085_b170 => extern "C" fn n_85b170(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01fc(&mut mem(), w, a, s)
    }
    /// `Native_0051_PadIsPressedRaw(worker, args, stack)`.
    Native_0051_PadIsPressedRaw @ 0x0085_f0e0 => extern "C" fn n_85f0e0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0051(&mut mem(), w, a, s)
    }
    /// `Native_00e8_PartyGetMembers(worker, args, stack)`.
    Native_00e8_PartyGetMembers @ 0x0085_6270 => extern "C" fn n_856270(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00e8(&mut mem(), w, a, s)
    }
    /// `Native_0030(worker, args, stack)`.
    Native_0030 @ 0x0085_a7c0 => extern "C" fn n_85a7c0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_0030(&mut mem(), w, a, s)
    }
    /// `Native_01d4_ChrSetClip(worker, args, stack)`.
    Native_01d4_ChrSetClip @ 0x0085_5dd0 => extern "C" fn n_855dd0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01d4(&mut mem(), w, a, s)
    }
    /// `Native_022f_ChrSetClipMode(worker, args, stack)`.
    Native_022f_ChrSetClipMode @ 0x0085_5fb0 => extern "C" fn n_855fb0(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_016a_MusicVolumeDirect(worker, args, stack)`.
    Native_016a_MusicVolumeDirect @ 0x0085_bbb0 => extern "C" fn n_85bbb0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_016a(&mut mem(), w, a, s)
    }
    /// `Native_00fe_SndSepVolumeDirect(worker, args, stack)`.
    Native_00fe_SndSepVolumeDirect @ 0x0086_0090 => extern "C" fn n_860090(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00fe(&mut mem(), w, a, s)
    }
    /// `Native_024d(worker, args, stack)`.
    Native_024d @ 0x0085_6dc0 => extern "C" fn n_856dc0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_024d(&mut mem(), w, a, s)
    }
    /// `Native_0209_BindEffWaterOn(worker, args, stack)`.
    Native_0209_BindEffWaterOn @ 0x0085_f430 => extern "C" fn n_85f430(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_010c_MapChangeDefault(worker, args, stack)`.
    Native_010c_MapChangeDefault @ 0x0085_8610 => extern "C" fn n_858610(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_010c(&mut mem(), w, a, s)
    }
    /// `Native_00ab_MapChange(worker, args, stack)`.
    Native_00ab_MapChange @ 0x0085_8330 => extern "C" fn n_858330(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00ab(&mut mem(), w, a, s)
    }
    /// `Native_011a_LoadEventTexList(worker, args, stack)`.
    Native_011a_LoadEventTexList @ 0x0085_c0a0 => extern "C" fn n_85c0a0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_011a(&mut mem(), w, a, s)
    }
    /// `Native_014e_GetAngleFromCenter(worker, args, stack)`.
    Native_014e_GetAngleFromCenter @ 0x0085_8ea0 => extern "C" fn n_858ea0(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_014e(&mut mem(), w, a, s)
    }
    /// `Native_024b_ChrSetHideEvent(worker, args, stack)`.
    Native_024b_ChrSetHideEvent @ 0x0085_7e30 => extern "C" fn n_857e30(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_01f8_MusicFadeOutTrack(worker, args, stack)`.
    Native_01f8_MusicFadeOutTrack @ 0x0085_e570 => extern "C" fn n_85e570(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01f8(&mut mem(), w, a, s)
    }
    /// `Native_0114(worker, args, stack)`.
    Native_0114 @ 0x0085_b670 => extern "C" fn n_85b670(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0114(&mut mem(), w, a, s)
    }
    /// `Native_0220_DisableLoveParam(worker, args, stack)`.
    Native_0220_DisableLoveParam @ 0x0085_e010 => extern "C" fn n_85e010(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0220(&mut mem(), w, a, s)
    }
    /// `Native_0203_ChrLoadModel(worker, args, stack)`.
    Native_0203_ChrLoadModel @ 0x0085_d460 => extern "C" fn n_85d460(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0203(&mut mem(), w, a, s)
    }
    /// `Native_0203_ChrLoadModel_Poll(worker, args, stack)`.
    Native_0203_ChrLoadModel_Poll @ 0x0085_d550 => extern "C" fn n_85d550(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0203_poll(&mut mem(), w, a, s)
    }
    /// `Native_0203_ChrLoadModel_Result(worker, args, stack)`.
    Native_0203_ChrLoadModel_Result @ 0x0085_dc10 => extern "C" fn n_85dc10(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_012e_SaveCounterAdd(worker, args, stack)`.
    Native_012e_SaveCounterAdd @ 0x0085_e040 => extern "C" fn n_85e040(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_023a_SetDolby(worker, args, stack)`.
    Native_023a_SetDolby @ 0x0085_6250 => extern "C" fn n_856250(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_01cf_BindEffDrawOff(worker, args, stack)`.
    Native_01cf_BindEffDrawOff @ 0x0085_ea40 => extern "C" fn n_85ea40(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01cf(&mut mem(), w, a, s)
    }
    /// `Native_01b0_MovieSetTexture(worker, args, stack)`.
    Native_01b0_MovieSetTexture @ 0x0085_90a0 => extern "C" fn n_8590a0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01b0(&mut mem(), w, a, s)
    }
    /// `Native_020f_SetForceBattleMap(worker, args, stack)`.
    Native_020f_SetForceBattleMap @ 0x0085_c100 => extern "C" fn n_85c100(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_020f(&mut mem(), w, a, s)
    }
    /// `Native_01dc_VibrateStop3(worker, args, stack)`.
    Native_01dc_VibrateStop3 @ 0x0085_71d0 => extern "C" fn n_8571d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01dc(&mut mem(), w, a, s)
    }
    /// `Native_0202_FootSoundOffChr(worker, args, stack)`.
    Native_0202_FootSoundOffChr @ 0x0085_a770 => extern "C" fn n_85a770(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_0189_SetMapIdPage(worker, args, stack)`.
    Native_0189_SetMapIdPage @ 0x0085_e890 => extern "C" fn n_85e890(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_0146_MinimapSetSize(worker, args, stack)`.
    Native_0146_MinimapSetSize @ 0x0085_81a0 => extern "C" fn n_8581a0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0146(&mut mem(), w, a, s)
    }
    /// `Native_0145_MinimapSetPos(worker, args, stack)`.
    Native_0145_MinimapSetPos @ 0x0085_7fc0 => extern "C" fn n_857fc0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0145(&mut mem(), w, a, s)
    }
    /// `Native_0226_FpSetDisp(worker, args, stack)`.
    Native_0226_FpSetDisp @ 0x0085_eb80 => extern "C" fn n_85eb80(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0226(&mut mem(), w, a, s)
    }
    /// `Native_00d2_StopTurnAll(worker, args, stack)`.
    Native_00d2_StopTurnAll @ 0x0085_c6f0 => extern "C" fn n_85c6f0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00d2(&mut mem(), w, a, s)
    }
    /// `Native_0037_WorkerStopTurnAll(worker, args, stack)`.
    Native_0037_WorkerStopTurnAll @ 0x0085_b780 => extern "C" fn n_85b780(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_0260_AlwaysTrue(worker, args, stack)`.
    Native_0260_AlwaysTrue @ 0x0085_9f10 => extern "C" fn n_859f10(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0260(&mut mem(), w, a, s)
    }
    /// `Native_0261_AlwaysTrue(worker, args, stack)`.
    Native_0261_AlwaysTrue @ 0x0085_a090 => extern "C" fn n_85a090(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0261(&mut mem(), w, a, s)
    }
    /// `Native_0251(worker, args, stack)`.
    Native_0251 @ 0x0085_e5e0 => extern "C" fn n_85e5e0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0251(&mut mem(), w, a, s)
    }
    /// `Native_0205_BindEffWalkOn(worker, args, stack)`.
    Native_0205_BindEffWalkOn @ 0x0085_ec00 => extern "C" fn n_85ec00(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0205(&mut mem(), w, a, s)
    }
    /// `Native_0206_BindEffWalkOff(worker, args, stack)`.
    Native_0206_BindEffWalkOff @ 0x0085_ee10 => extern "C" fn n_85ee10(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0206(&mut mem(), w, a, s)
    }
    /// `Native_020a_BindEffWaterOff(worker, args, stack)`.
    Native_020a_BindEffWaterOff @ 0x0085_f660 => extern "C" fn n_85f660(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_020a(&mut mem(), w, a, s)
    }
    /// `Native_010a_Snd3dStop(worker, args, stack)`.
    Native_010a_Snd3dStop @ 0x0085_a630 => extern "C" fn n_85a630(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_010a(&mut mem(), w, a, s)
    }
    /// `Native_00e2_SetBattleStatusSys(worker, args, stack)`.
    Native_00e2_SetBattleStatusSys @ 0x0085_7dd0 => extern "C" fn n_857dd0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00e2(&mut mem(), w, a, s)
    }
    /// `Native_00c4_MoveIsActive(worker, args, stack)`.
    Native_00c4_MoveIsActive @ 0x0085_a340 => extern "C" fn n_85a340(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00c4(&mut mem(), w, a, s)
    }
    /// `Native_0003(worker, args, stack)`.
    Native_0003 @ 0x0085_ebd0 => extern "C" fn n_85ebd0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0003(&mut mem(), w, a, s)
    }
    /// `Native_0201_FootSoundOff(worker, args, stack)`.
    Native_0201_FootSoundOff @ 0x0085_a580 => extern "C" fn n_85a580(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0201(&mut mem(), w, a, s)
    }
    /// `Native_0200_FootSoundOnChr(worker, args, stack)`.
    Native_0200_FootSoundOnChr @ 0x0085_a430 => extern "C" fn n_85a430(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_00fa_SetEventNewGame(worker, args, stack)`.
    Native_00fa_SetEventNewGame @ 0x0085_8920 => extern "C" fn n_858920(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_01ac_GetMonsterCapture(worker, args, stack)`.
    Native_01ac_GetMonsterCapture @ 0x0085_8950 => extern "C" fn n_858950(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01ac(&mut mem(), w, a, s)
    }
    /// `Native_01a7_FieldItemGet(worker, args, stack)`.
    Native_01a7_FieldItemGet @ 0x0085_7b70 => extern "C" fn n_857b70(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01a7(&mut mem(), w, a, s)
    }
    /// `Native_01a7_FieldItemGet_Poll(worker, args, stack)`.
    Native_01a7_FieldItemGet_Poll @ 0x0085_7ef0 => extern "C" fn n_857ef0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01a7_poll(&mut mem(), w, a, s)
    }
    /// `Native_01a7_FieldItemGet_Result(worker, args, stack)`.
    Native_01a7_FieldItemGet_Result @ 0x0085_8230 => extern "C" fn n_858230(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01a7_result(&mut mem(), w, a, s)
    }
    /// `Native_01a0(worker, args, stack)`.
    Native_01a0 @ 0x0085_f940 => extern "C" fn n_85f940(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01a0(&mut mem(), w, a, s)
    }
    /// `Native_019e_IsCtrl0Running(worker, args, stack)`.
    Native_019e_IsCtrl0Running @ 0x0085_6950 => extern "C" fn n_856950(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_019e(&mut mem(), w, a, s)
    }
    /// `Native_00c9(worker, args, stack)`.
    Native_00c9 @ 0x0085_b560 => extern "C" fn n_85b560(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_00c9(&mut mem(), w, a, s)
    }
    /// `Native_01d2_MsgTextMatchesSave(worker, args, stack)`.
    Native_01d2_MsgTextMatchesSave @ 0x0086_0180 => extern "C" fn n_860180(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01d2(&mut mem(), w, a, s)
    }
    /// `Native_01bc_SetForcePlaceId(worker, args, stack)`.
    Native_01bc_SetForcePlaceId @ 0x0085_9ec0 => extern "C" fn n_859ec0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01bc(&mut mem(), w, a, s)
    }
    /// `Native_014f_AngleDiffWithin(worker, args, stack)`.
    Native_014f_AngleDiffWithin @ 0x0085_9120 => extern "C" fn n_859120(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_01ee_VibrateStop1(worker, args, stack)`.
    Native_01ee_VibrateStop1 @ 0x0085_7440 => extern "C" fn n_857440(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01ee(&mut mem(), w, a, s)
    }
    /// `Native_01e7_MusicPhraseStart(worker, args, stack)`.
    Native_01e7_MusicPhraseStart @ 0x0085_9320 => extern "C" fn n_859320(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01e7(&mut mem(), w, a, s)
    }
    /// `Native_01e0_ChrSetEffectLevel(worker, args, stack)`.
    Native_01e0_ChrSetEffectLevel @ 0x0085_8490 => extern "C" fn n_858490(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01e0(&mut mem(), w, a, s)
    }
    /// `Native_018a_SetEffectFlag2(worker, args, stack)`.
    Native_018a_SetEffectFlag2 @ 0x0085_ebb0 => extern "C" fn n_85ebb0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_018a(&mut mem(), w, a, s)
    }
    /// `Native_009b_TriggerSetLineVec(worker, args, stack)`.
    Native_009b_TriggerSetLineVec @ 0x0085_f750 => extern "C" fn n_85f750(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_009b(&mut mem(), w, a, s)
    }
    /// `Native_01de_VibrateWait3(worker, args, stack)`.
    Native_01de_VibrateWait3 @ 0x0085_7920 => extern "C" fn n_857920(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01de(&mut mem(), w, a, s)
    }
    /// `Native_01de_VibrateWait3_Poll(worker, args, stack)`.
    Native_01de_VibrateWait3_Poll @ 0x0085_79d0 => extern "C" fn n_8579d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01de_poll(&mut mem(), w, a, s)
    }
    /// `Native_01af_AddMonsterCaptureN(worker, args, stack)`.
    Native_01af_AddMonsterCaptureN @ 0x0085_8e70 => extern "C" fn n_858e70(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01af(&mut mem(), w, a, s)
    }
    /// `Native_0140_MsgBuildChoiceText(worker, args, stack)`.
    Native_0140_MsgBuildChoiceText @ 0x0085_6f20 => extern "C" fn n_856f20(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0140(&mut mem(), w, a, s)
    }
    /// `Native_00cc_PartyIsJoined(worker, args, stack)`.
    Native_00cc_PartyIsJoined @ 0x0085_b9f0 => extern "C" fn n_85b9f0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00cc(&mut mem(), w, a, s)
    }
    /// `Native_0239_SetEventFlag(worker, args, stack)`.
    Native_0239_SetEventFlag @ 0x0085_e8b0 => extern "C" fn n_85e8b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0239(&mut mem(), w, a, s)
    }
    /// `Native_013d_SndTransSepGroup(worker, args, stack)`.
    Native_013d_SndTransSepGroup @ 0x0085_61b0 => extern "C" fn n_8561b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_013d(&mut mem(), w, a, s)
    }
    /// `Native_013d_SndTransSepGroup_Poll(worker, args, stack)`.
    Native_013d_SndTransSepGroup_Poll @ 0x0085_6300 => extern "C" fn n_856300(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_024a_SetHddConfig(worker, args, stack)`.
    Native_024a_SetHddConfig @ 0x0085_7cb0 => extern "C" fn n_857cb0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_024a(&mut mem(), w, a, s)
    }
    /// `Native_0240_SetWeaponInvisible(worker, args, stack)`.
    Native_0240_SetWeaponInvisible @ 0x0085_68d0 => extern "C" fn n_8568d0(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_01f2_GetSummonJoinMask(worker, args, stack)`.
    Native_01f2_GetSummonJoinMask @ 0x0085_97d0 => extern "C" fn n_8597d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01f2(&mut mem(), w, a, s)
    }
    /// `Native_01f0_SetHalfTurnAngle(worker, args, stack)`.
    Native_01f0_SetHalfTurnAngle @ 0x0085_9550 => extern "C" fn n_859550(w: u32, a: u32, s: u32) -> f64 {
        native_move::native_01f0(&mut mem(), w, a, s)
    }
    /// `Native_01a6_PartyFillSlots(worker, args, stack)`.
    Native_01a6_PartyFillSlots @ 0x0085_76d0 => extern "C" fn n_8576d0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01a6(&mut mem(), w, a, s)
    }
    /// `Native_018b_ClearEffectFlag2(worker, args, stack)`.
    Native_018b_ClearEffectFlag2 @ 0x0085_ec90 => extern "C" fn n_85ec90(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_018b(&mut mem(), w, a, s)
    }
    /// `Native_0184_MusicTempoDirect(worker, args, stack)`.
    Native_0184_MusicTempoDirect @ 0x0085_dac0 => extern "C" fn n_85dac0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0184(&mut mem(), w, a, s)
    }
    /// `Native_0185_MusicTempoTranslate(worker, args, stack)`.
    Native_0185_MusicTempoTranslate @ 0x0085_dbb0 => extern "C" fn n_85dbb0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0185(&mut mem(), w, a, s)
    }
    /// `Native_0166_MusicStart(worker, args, stack)`.
    Native_0166_MusicStart @ 0x0085_98f0 => extern "C" fn n_8598f0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0166(&mut mem(), w, a, s)
    }
    /// `Native_017c_MusicMultiStart(worker, args, stack)`.
    Native_017c_MusicMultiStart @ 0x0085_d480 => extern "C" fn n_85d480(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_017c(&mut mem(), w, a, s)
    }
    /// `Native_0156_SndFootOff(worker, args, stack)`.
    Native_0156_SndFootOff @ 0x0085_9920 => extern "C" fn n_859920(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0156(&mut mem(), w, a, s)
    }
    /// `Native_0154_SndFootOn(worker, args, stack)`.
    Native_0154_SndFootOn @ 0x0085_97a0 => extern "C" fn n_8597a0(w: u32, a: u32, s: u32) -> u32 {
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
    /// `Native_00e6_PartyIsInSlot(worker, args, stack)`.
    Native_00e6_PartyIsInSlot @ 0x0085_bf90 => extern "C" fn n_85bf90(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00e6(&mut mem(), w, a, s)
    }
    /// `Native_00ac_SetSaveDicValue(worker, args, stack)`.
    Native_00ac_SetSaveDicValue @ 0x0085_8050 => extern "C" fn n_858050(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00ac(&mut mem(), w, a, s)
    }
    /// `Native_0264_SetBgmType(worker, args, stack)`.
    Native_0264_SetBgmType @ 0x0085_a230 => extern "C" fn n_85a230(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0264(&mut mem(), w, a, s)
    }
    /// `Native_022e_SetMcfpInWater(worker, args, stack)`.
    Native_022e_SetMcfpInWater @ 0x0085_f860 => extern "C" fn n_85f860(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_022e(&mut mem(), w, a, s)
    }
    /// `Native_01f4(worker, args, stack)`.
    Native_01f4 @ 0x0085_9fa0 => extern "C" fn n_859fa0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01f4(&mut mem(), w, a, s)
    }
    /// `Native_01f3_SummonChoiceText(worker, args, stack)`.
    Native_01f3_SummonChoiceText @ 0x0085_9990 => extern "C" fn n_859990(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01f3(&mut mem(), w, a, s)
    }
    /// `Native_01c0_SetEffectParam(worker, args, stack)`.
    Native_01c0_SetEffectParam @ 0x0085_a250 => extern "C" fn n_85a250(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01c0(&mut mem(), w, a, s)
    }
    /// `Native_01b7_ChangeWeaponWakka(worker, args, stack)`.
    Native_01b7_ChangeWeaponWakka @ 0x0085_9800 => extern "C" fn n_859800(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01b7(&mut mem(), w, a, s)
    }
    /// `Native_01b2_GetItemCount(worker, args, stack)`.
    Native_01b2_GetItemCount @ 0x0085_9360 => extern "C" fn n_859360(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01b2(&mut mem(), w, a, s)
    }
    /// `Native_018f_VoiceVolumeTranslate(worker, args, stack)`.
    Native_018f_VoiceVolumeTranslate @ 0x0085_f240 => extern "C" fn n_85f240(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_018f(&mut mem(), w, a, s)
    }
    /// `Native_0179_WaitEffectFlag(worker, args, stack)`.
    Native_0179_WaitEffectFlag @ 0x0085_cd90 => extern "C" fn n_85cd90(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0179(&mut mem(), w, a, s)
    }
    /// `Native_0179_WaitEffectFlag_Poll(worker, args, stack)`.
    Native_0179_WaitEffectFlag_Poll @ 0x0085_cea0 => extern "C" fn n_85cea0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0179_poll(&mut mem(), w, a, s)
    }
    /// `Native_0138_SetGil(worker, args, stack)`.
    Native_0138_SetGil @ 0x0085_f140 => extern "C" fn n_85f140(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0138(&mut mem(), w, a, s)
    }
    /// `Native_0127_WorkerInTouchRange(worker, args, stack)`.
    Native_0127_WorkerInTouchRange @ 0x0085_db10 => extern "C" fn n_85db10(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0127(&mut mem(), w, a, s)
    }
    /// `Native_0107(worker, args, stack)`.
    Native_0107 @ 0x0085_a1f0 => extern "C" fn n_85a1f0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0107(&mut mem(), w, a, s)
    }
    /// `Native_00b4_SetTkAntiY(worker, args, stack)`.
    Native_00b4_SetTkAntiY @ 0x0085_8a20 => extern "C" fn n_858a20(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00b4(&mut mem(), w, a, s)
    }
    /// `Native_00b3_SetTkAntiX(worker, args, stack)`.
    Native_00b3_SetTkAntiX @ 0x0085_8810 => extern "C" fn n_858810(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00b3(&mut mem(), w, a, s)
    }
    /// `Native_00b2_SgEnableTkAnti(worker, args, stack)`.
    Native_00b2_SgEnableTkAnti @ 0x0085_86b0 => extern "C" fn n_8586b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00b2(&mut mem(), w, a, s)
    }
    /// `Native_0012_SetPosXZ(worker, args, stack)`.
    Native_0012_SetPosXZ @ 0x0085_f800 => extern "C" fn n_85f800(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0012(&mut mem(), w, a, s)
    }
    /// `Native_015b_GiveEventReward(worker, args, stack)`.
    Native_015b_GiveEventReward @ 0x0085_a8a0 => extern "C" fn n_85a8a0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_015b(&mut mem(), w, a, s)
    }
    /// `CT_0216_init_teachAbilityToPartyMemberWithMsg(worker, args, stack)`.
    CT_0216_init_teachAbilityToPartyMemberWithMsg @ 0x0085_b280 => extern "C" fn n_85b280(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0216(&mut mem(), w, a, s)
    }
    /// `Native_7123(worker, args, stack)`.
    Native_7123 @ 0x007a_2980 => extern "C" fn n_7a2980(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7123(&mut mem(), w, a, s)
    }
    /// `Native_70c4(worker, args, stack)`.
    Native_70c4 @ 0x007a_2ba0 => extern "C" fn n_7a2ba0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70c4(&mut mem(), w, a, s)
    }
    /// `Native_710c(worker, args, stack)`.
    Native_710c @ 0x007a_2c10 => extern "C" fn n_7a2c10(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_710c(&mut mem(), w, a, s)
    }
    /// `Native_01b2_copy_007a2c50(worker, args, stack)`.
    Native_01b2_copy_007a2c50 @ 0x007a_2c50 => extern "C" fn n_7a2c50(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01b2(&mut mem(), w, a, s)
    }
    /// `Native_70f1(worker, args, stack)`.
    Native_70f1 @ 0x007a_2d30 => extern "C" fn n_7a2d30(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70f1(&mut mem(), w, a, s)
    }
    /// `Native_705b(worker, args, stack)`.
    Native_705b @ 0x007a_2d80 => extern "C" fn n_7a2d80(w: u32, a: u32, s: u32) -> f64 {
        native_btl2::native_705b(&mut mem(), w, a, s)
    }
    /// `Native_705c(worker, args, stack)`.
    Native_705c @ 0x007a_2df0 => extern "C" fn n_7a2df0(w: u32, a: u32, s: u32) -> f64 {
        native_btl2::native_705c(&mut mem(), w, a, s)
    }
    /// `Native_70f2(worker, args, stack)`.
    Native_70f2 @ 0x007a_2e60 => extern "C" fn n_7a2e60(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70f2(&mut mem(), w, a, s)
    }
    /// `Native_709a(worker, args, stack)`.
    Native_709a @ 0x007a_2eb0 => extern "C" fn n_7a2eb0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_709a(&mut mem(), w, a, s)
    }
    /// `Native_70b9(worker, args, stack)`.
    Native_70b9 @ 0x007a_2ee0 => extern "C" fn n_7a2ee0(w: u32, a: u32, s: u32) -> f64 {
        native_btl2::native_70b9(&mut mem(), w, a, s)
    }
    /// `Native_70f5(worker, args, stack)`.
    Native_70f5 @ 0x007a_2f80 => extern "C" fn n_7a2f80(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70f5(&mut mem(), w, a, s)
    }
    /// `Native_70ba(worker, args, stack)`.
    Native_70ba @ 0x007a_3040 => extern "C" fn n_7a3040(w: u32, a: u32, s: u32) -> f64 {
        native_btl2::native_70ba(&mut mem(), w, a, s)
    }
    /// `Native_709b(worker, args, stack)`.
    Native_709b @ 0x007a_3170 => extern "C" fn n_7a3170(w: u32, a: u32, s: u32) -> f64 {
        native_btl2::native_709b(&mut mem(), w, a, s)
    }
    /// `Native_70c2(worker, args, stack)`.
    Native_70c2 @ 0x007a_31d0 => extern "C" fn n_7a31d0(w: u32, a: u32, s: u32) -> f64 {
        native_btl2::native_70c2(&mut mem(), w, a, s)
    }
    /// `Native_709c(worker, args, stack)`.
    Native_709c @ 0x007a_3330 => extern "C" fn n_7a3330(w: u32, a: u32, s: u32) -> f64 {
        native_btl2::native_709c(&mut mem(), w, a, s)
    }
    /// `Native_70d8(worker, args, stack)`.
    Native_70d8 @ 0x007a_3400 => extern "C" fn n_7a3400(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70d8(&mut mem(), w, a, s)
    }
    /// `Native_70fa(worker, args, stack)`.
    Native_70fa @ 0x007a_3420 => extern "C" fn n_7a3420(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70fa(&mut mem(), w, a, s)
    }
    /// `Native_709f(worker, args, stack)`.
    Native_709f @ 0x007a_3460 => extern "C" fn n_7a3460(w: u32, a: u32, s: u32) -> f64 {
        native_btl2::native_709f(&mut mem(), w, a, s)
    }
    /// `Native_70f7(worker, args, stack)`.
    Native_70f7 @ 0x007a_3560 => extern "C" fn n_7a3560(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70f7(&mut mem(), w, a, s)
    }
    /// `Native_7102(worker, args, stack)`.
    Native_7102 @ 0x007a_35d0 => extern "C" fn n_7a35d0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7102(&mut mem(), w, a, s)
    }
    /// `Native_7103(worker, args, stack)`.
    Native_7103 @ 0x007a_3880 => extern "C" fn n_7a3880(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7103(&mut mem(), w, a, s)
    }
    /// `Native_70fd(worker, args, stack)`.
    Native_70fd @ 0x007a_3a10 => extern "C" fn n_7a3a10(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70fd(&mut mem(), w, a, s)
    }
    /// `Native_710b_Poll(worker, args, stack)`.
    Native_710b_Poll @ 0x007a_3a80 => extern "C" fn n_7a3a80(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_710b_poll(&mut mem(), w, a, s)
    }
    /// `Native_7004(worker, args, stack)`.
    Native_7004 @ 0x007a_3aa0 => extern "C" fn n_7a3aa0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7004(&mut mem(), w, a, s)
    }
    /// `Native_70fe(worker, args, stack)`.
    Native_70fe @ 0x007a_3b80 => extern "C" fn n_7a3b80(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70fe(&mut mem(), w, a, s)
    }
    /// `Native_710d(worker, args, stack)`.
    Native_710d @ 0x007a_3c20 => extern "C" fn n_7a3c20(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_710d(&mut mem(), w, a, s)
    }
    /// `Native_70ff(worker, args, stack)`.
    Native_70ff @ 0x007a_3d00 => extern "C" fn n_7a3d00(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70ff(&mut mem(), w, a, s)
    }
    /// `Native_7110(worker, args, stack)`.
    Native_7110 @ 0x007a_3d30 => extern "C" fn n_7a3d30(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7110(&mut mem(), w, a, s)
    }
    /// `Native_711b(worker, args, stack)`.
    Native_711b @ 0x007a_3dd0 => extern "C" fn n_7a3dd0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_711b(&mut mem(), w, a, s)
    }
    /// `Native_7122(worker, args, stack)`.
    Native_7122 @ 0x007a_3ef0 => extern "C" fn n_7a3ef0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7122(&mut mem(), w, a, s)
    }
    /// `Native_7111(worker, args, stack)`.
    Native_7111 @ 0x007a_3ff0 => extern "C" fn n_7a3ff0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7111(&mut mem(), w, a, s)
    }
    /// `Native_711a(worker, args, stack)`.
    Native_711a @ 0x007a_4160 => extern "C" fn n_7a4160(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_711a(&mut mem(), w, a, s)
    }
    /// `Native_7011(worker, args, stack)`.
    Native_7011 @ 0x007a_4280 => extern "C" fn n_7a4280(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7011(&mut mem(), w, a, s)
    }
    /// `Native_70d0(worker, args, stack)`.
    Native_70d0 @ 0x007a_4380 => extern "C" fn n_7a4380(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70d0(&mut mem(), w, a, s)
    }
    /// `Native_7112(worker, args, stack)`.
    Native_7112 @ 0x007a_43d0 => extern "C" fn n_7a43d0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7112(&mut mem(), w, a, s)
    }
    /// `Native_70c1(worker, args, stack)`.
    Native_70c1 @ 0x007a_4b90 => extern "C" fn n_7a4b90(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70c1(&mut mem(), w, a, s)
    }
    /// `Native_70df(worker, args, stack)`.
    Native_70df @ 0x007a_4ee0 => extern "C" fn n_7a4ee0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70df(&mut mem(), w, a, s)
    }
    /// `Native_705e(worker, args, stack)`.
    Native_705e @ 0x007a_5310 => extern "C" fn n_7a5310(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_705e(&mut mem(), w, a, s)
    }
    /// `Native_7020(worker, args, stack)`.
    Native_7020 @ 0x007a_53b0 => extern "C" fn n_7a53b0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7020(&mut mem(), w, a, s)
    }
    /// `Native_7013(worker, args, stack)`.
    Native_7013 @ 0x007a_5840 => extern "C" fn n_7a5840(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7013(&mut mem(), w, a, s)
    }
    /// `Native_70bc(worker, args, stack)`.
    Native_70bc @ 0x007a_5990 => extern "C" fn n_7a5990(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70bc(&mut mem(), w, a, s)
    }
    /// `Native_70bc_Poll(worker, args, stack)`.
    Native_70bc_Poll @ 0x007a_5b90 => extern "C" fn n_7a5b90(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70bc_poll(&mut mem(), w, a, s)
    }
    /// `Native_7027(worker, args, stack)`.
    Native_7027 @ 0x007a_5e90 => extern "C" fn n_7a5e90(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7027(&mut mem(), w, a, s)
    }
    /// `Native_70bd(worker, args, stack)`.
    Native_70bd @ 0x007a_5ed0 => extern "C" fn n_7a5ed0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70bd(&mut mem(), w, a, s)
    }
    /// `Native_70bd_Poll(worker, args, stack)`.
    Native_70bd_Poll @ 0x007a_6030 => extern "C" fn n_7a6030(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70bd_poll(&mut mem(), w, a, s)
    }
    /// `Native_7092(worker, args, stack)`.
    Native_7092 @ 0x007a_5f10 => extern "C" fn n_7a5f10(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7092(&mut mem(), w, a, s)
    }
    /// `Native_70d2(worker, args, stack)`.
    Native_70d2 @ 0x007a_6130 => extern "C" fn n_7a6130(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70d2(&mut mem(), w, a, s)
    }
    /// `Native_7060(worker, args, stack)`.
    Native_7060 @ 0x007a_62a0 => extern "C" fn n_7a62a0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7060(&mut mem(), w, a, s)
    }
    /// `Native_7061(worker, args, stack)`.
    Native_7061 @ 0x007a_6300 => extern "C" fn n_7a6300(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7061(&mut mem(), w, a, s)
    }
    /// `Native_7064(worker, args, stack)`.
    Native_7064 @ 0x007a_63d0 => extern "C" fn n_7a63d0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7064(&mut mem(), w, a, s)
    }
    /// `Native_7064_Result(worker, args, stack)`.
    Native_7064_Result @ 0x007a_65d0 => extern "C" fn n_7a65d0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7064_result(&mut mem(), w, a, s)
    }
    /// `Native_7105_Poll(worker, args, stack)`.
    Native_7105_Poll @ 0x007a_6730 => extern "C" fn n_7a6730(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7105_poll(&mut mem(), w, a, s)
    }
    /// `Native_7043_Poll(worker, args, stack)`.
    Native_7043_Poll @ 0x007a_6a40 => extern "C" fn n_7a6a40(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7043_poll(&mut mem(), w, a, s)
    }
    /// `Native_7043(worker, args, stack)`.
    Native_7043 @ 0x007a_6a10 => extern "C" fn n_7a6a10(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7043(&mut mem(), w, a, s)
    }
    /// `Native_70a9(worker, args, stack)`.
    Native_70a9 @ 0x007a_6c50 => extern "C" fn n_7a6c50(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70a9(&mut mem(), w, a, s)
    }
    /// `Native_706d_Int(worker, args, stack)`.
    Native_706d_Int @ 0x007a_6ea0 => extern "C" fn n_7a6ea0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_706d_int(&mut mem(), w, a, s)
    }
    /// `Native_704c_Poll(worker, args, stack)`.
    Native_704c_Poll @ 0x007a_7cf0 => extern "C" fn n_7a7cf0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_704c_poll(&mut mem(), w, a, s)
    }
    /// `Native_70bf(worker, args, stack)`.
    Native_70bf @ 0x007a_6ae0 => extern "C" fn n_7a6ae0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70bf(&mut mem(), w, a, s)
    }
    /// `Native_70a9_Poll(worker, args, stack)`.
    Native_70a9_Poll @ 0x007a_6cb0 => extern "C" fn n_7a6cb0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70a9_poll(&mut mem(), w, a, s)
    }
    /// `Native_706d(worker, args, stack)`.
    Native_706d @ 0x007a_6d00 => extern "C" fn n_7a6d00(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_706d(&mut mem(), w, a, s)
    }
    /// `Native_704f(worker, args, stack)`.
    Native_704f @ 0x007a_70c0 => extern "C" fn n_7a70c0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_704f(&mut mem(), w, a, s)
    }
    /// `Native_707c_Poll(worker, args, stack)`.
    Native_707c_Poll @ 0x007a_7260 => extern "C" fn n_7a7260(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_707c_poll(&mut mem(), w, a, s)
    }
    /// `Native_70a0(worker, args, stack)`.
    Native_70a0 @ 0x007a_73d0 => extern "C" fn n_7a73d0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70a0(&mut mem(), w, a, s)
    }
    /// `Native_704e(worker, args, stack)`.
    Native_704e @ 0x007a_7530 => extern "C" fn n_7a7530(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_704e(&mut mem(), w, a, s)
    }
    /// `Native_706a(worker, args, stack)`.
    Native_706a @ 0x007a_7580 => extern "C" fn n_7a7580(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_706a(&mut mem(), w, a, s)
    }
    /// `Native_70a6(worker, args, stack)`.
    Native_70a6 @ 0x007a_77a0 => extern "C" fn n_7a77a0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70a6(&mut mem(), w, a, s)
    }
    /// `Native_70b8(worker, args, stack)`.
    Native_70b8 @ 0x007a_7860 => extern "C" fn n_7a7860(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70b8(&mut mem(), w, a, s)
    }
    /// `Native_7075(worker, args, stack)`.
    Native_7075 @ 0x007a_78f0 => extern "C" fn n_7a78f0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7075(&mut mem(), w, a, s)
    }
    /// `Native_7075_Poll(worker, args, stack)`.
    Native_7075_Poll @ 0x007a_7aa0 => extern "C" fn n_7a7aa0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7075_poll(&mut mem(), w, a, s)
    }
    /// `Native_70ee(worker, args, stack)`.
    Native_70ee @ 0x007a_7b90 => extern "C" fn n_7a7b90(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70ee(&mut mem(), w, a, s)
    }
    /// `Native_704c(worker, args, stack)`.
    Native_704c @ 0x007a_7bf0 => extern "C" fn n_7a7bf0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_704c(&mut mem(), w, a, s)
    }
    /// `Native_70e3(worker, args, stack)`.
    Native_70e3 @ 0x007a_7da0 => extern "C" fn n_7a7da0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70e3(&mut mem(), w, a, s)
    }
    /// `Native_70b3(worker, args, stack)`.
    Native_70b3 @ 0x007a_7df0 => extern "C" fn n_7a7df0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70b3(&mut mem(), w, a, s)
    }
    /// `Native_70dd(worker, args, stack)`.
    Native_70dd @ 0x007a_7f80 => extern "C" fn n_7a7f80(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70dd(&mut mem(), w, a, s)
    }
    /// `Native_70d6(worker, args, stack)`.
    Native_70d6 @ 0x007a_80b0 => extern "C" fn n_7a80b0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70d6(&mut mem(), w, a, s)
    }
    /// `Native_7119(worker, args, stack)`.
    Native_7119 @ 0x007a_8430 => extern "C" fn n_7a8430(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7119(&mut mem(), w, a, s)
    }
    /// `Native_7126(worker, args, stack)`.
    Native_7126 @ 0x007a_8550 => extern "C" fn n_7a8550(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7126(&mut mem(), w, a, s)
    }
    /// `Native_70e1(worker, args, stack)`.
    Native_70e1 @ 0x007a_85a0 => extern "C" fn n_7a85a0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70e1(&mut mem(), w, a, s)
    }
    /// `Native_7073(worker, args, stack)`.
    Native_7073 @ 0x007a_85f0 => extern "C" fn n_7a85f0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7073(&mut mem(), w, a, s)
    }
    /// `Native_7125(worker, args, stack)`.
    Native_7125 @ 0x007a_8650 => extern "C" fn n_7a8650(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7125(&mut mem(), w, a, s)
    }
    /// `Native_70e7(worker, args, stack)`.
    Native_70e7 @ 0x007a_8680 => extern "C" fn n_7a8680(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70e7(&mut mem(), w, a, s)
    }
    /// `Native_c05a(worker, args, stack)`.
    Native_c05a @ 0x0087_82f0 => extern "C" fn n_8782f0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c05a(&mut mem(), w, a, s)
    }
    /// `Native_c008(worker, args, stack)`.
    Native_c008 @ 0x0087_8390 => extern "C" fn n_878390(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c008(&mut mem(), w, a, s)
    }
    /// `Native_c00f(worker, args, stack)`.
    Native_c00f @ 0x0087_8750 => extern "C" fn n_878750(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c00f(&mut mem(), w, a, s)
    }
    /// `Native_c05d(worker, args, stack)`.
    Native_c05d @ 0x0087_8470 => extern "C" fn n_878470(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c05d(&mut mem(), w, a, s)
    }
    /// `Native_c010(worker, args, stack)`.
    Native_c010 @ 0x0087_8770 => extern "C" fn n_878770(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c010(&mut mem(), w, a, s)
    }
    /// `Native_c016(worker, args, stack)`.
    Native_c016 @ 0x0087_8860 => extern "C" fn n_878860(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c016(&mut mem(), w, a, s)
    }
    /// `Native_c017(worker, args, stack)`.
    Native_c017 @ 0x0087_8880 => extern "C" fn n_878880(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c017(&mut mem(), w, a, s)
    }
    /// `Native_c01a(worker, args, stack)`.
    Native_c01a @ 0x0087_88e0 => extern "C" fn n_8788e0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c01a(&mut mem(), w, a, s)
    }
    /// `Native_c01b(worker, args, stack)`.
    Native_c01b @ 0x0087_8920 => extern "C" fn n_878920(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c01b(&mut mem(), w, a, s)
    }
    /// `Native_c01c(worker, args, stack)`.
    Native_c01c @ 0x0087_8960 => extern "C" fn n_878960(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c01c(&mut mem(), w, a, s)
    }
    /// `Native_c01d(worker, args, stack)`.
    Native_c01d @ 0x0087_89a0 => extern "C" fn n_8789a0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c01d(&mut mem(), w, a, s)
    }
    /// `Native_c01e(worker, args, stack)`.
    Native_c01e @ 0x0087_89e0 => extern "C" fn n_8789e0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c01e(&mut mem(), w, a, s)
    }
    /// `Native_c01f(worker, args, stack)`.
    Native_c01f @ 0x0087_8a20 => extern "C" fn n_878a20(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c01f(&mut mem(), w, a, s)
    }
    /// `Native_c02d(worker, args, stack)`.
    Native_c02d @ 0x0087_8a60 => extern "C" fn n_878a60(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c02d(&mut mem(), w, a, s)
    }
    /// `Native_c02e(worker, args, stack)`.
    Native_c02e @ 0x0087_8a90 => extern "C" fn n_878a90(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c02e(&mut mem(), w, a, s)
    }
    /// `Native_c020(worker, args, stack)`.
    Native_c020 @ 0x0087_8ac0 => extern "C" fn n_878ac0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c020(&mut mem(), w, a, s)
    }
    /// `Native_c021(worker, args, stack)`.
    Native_c021 @ 0x0087_8ae0 => extern "C" fn n_878ae0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c020(&mut mem(), w, a, s)
    }
    /// `Native_c026(worker, args, stack)`.
    Native_c026 @ 0x0087_8ce0 => extern "C" fn n_878ce0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c026(&mut mem(), w, a, s)
    }
    /// `Native_c032(worker, args, stack)`.
    Native_c032 @ 0x0087_8ef0 => extern "C" fn n_878ef0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c032(&mut mem(), w, a, s)
    }
    /// `Native_c023(worker, args, stack)`.
    Native_c023 @ 0x0087_8b40 => extern "C" fn n_878b40(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c023(&mut mem(), w, a, s)
    }
    /// `Native_c029(worker, args, stack)`.
    Native_c029 @ 0x0087_8d90 => extern "C" fn n_878d90(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c029(&mut mem(), w, a, s)
    }
    /// `Native_c037(worker, args, stack)`.
    Native_c037 @ 0x0087_8fa0 => extern "C" fn n_878fa0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c037(&mut mem(), w, a, s)
    }
    /// `Native_c038(worker, args, stack)`.
    Native_c038 @ 0x0087_8fe0 => extern "C" fn n_878fe0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c038(&mut mem(), w, a, s)
    }
    /// `Native_c039(worker, args, stack)`.
    Native_c039 @ 0x0087_9040 => extern "C" fn n_879040(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c039(&mut mem(), w, a, s)
    }
    /// `Native_c03a(worker, args, stack)`.
    Native_c03a @ 0x0087_9070 => extern "C" fn n_879070(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c03a(&mut mem(), w, a, s)
    }
    /// `Native_c03d(worker, args, stack)`.
    Native_c03d @ 0x0087_90d0 => extern "C" fn n_8790d0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c03d(&mut mem(), w, a, s)
    }
    /// `Native_c03e(worker, args, stack)`.
    Native_c03e @ 0x0087_9110 => extern "C" fn n_879110(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c03e(&mut mem(), w, a, s)
    }
    /// `Native_c03f(worker, args, stack)`.
    Native_c03f @ 0x0087_9130 => extern "C" fn n_879130(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c03f(&mut mem(), w, a, s)
    }
    /// `Native_c041(worker, args, stack)`.
    Native_c041 @ 0x0087_9160 => extern "C" fn n_879160(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c041(&mut mem(), w, a, s)
    }
    /// `Native_c044(worker, args, stack)`.
    Native_c044 @ 0x0087_91a0 => extern "C" fn n_8791a0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c044(&mut mem(), w, a, s)
    }
    /// `Native_c045(worker, args, stack)`.
    Native_c045 @ 0x0087_91c0 => extern "C" fn n_8791c0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c045(&mut mem(), w, a, s)
    }
    /// `Native_c046(worker, args, stack)`.
    Native_c046 @ 0x0087_91e0 => extern "C" fn n_8791e0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c046(&mut mem(), w, a, s)
    }
    /// `Native_c047(worker, args, stack)`.
    Native_c047 @ 0x0087_9200 => extern "C" fn n_879200(w: u32, a: u32, s: u32) -> f64 {
        native_sys::native_c047(&mut mem(), w, a, s)
    }
    /// `Native_c049(worker, args, stack)`.
    Native_c049 @ 0x0087_9220 => extern "C" fn n_879220(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c049(&mut mem(), w, a, s)
    }
    /// `Native_c04b(worker, args, stack)`.
    Native_c04b @ 0x0087_9240 => extern "C" fn n_879240(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c04b(&mut mem(), w, a, s)
    }
    /// `Native_c04d(worker, args, stack)`.
    Native_c04d @ 0x0087_9260 => extern "C" fn n_879260(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c04d(&mut mem(), w, a, s)
    }
    /// `Native_c048(worker, args, stack)`.
    Native_c048 @ 0x0087_9280 => extern "C" fn n_879280(w: u32, a: u32, s: u32) -> f64 {
        native_sys::native_c048(&mut mem(), w, a, s)
    }
    /// `Native_c04a(worker, args, stack)`.
    Native_c04a @ 0x0087_92c0 => extern "C" fn n_8792c0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c04a(&mut mem(), w, a, s)
    }
    /// `Native_c04c(worker, args, stack)`.
    Native_c04c @ 0x0087_9300 => extern "C" fn n_879300(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c04c(&mut mem(), w, a, s)
    }
    /// `Native_c04e(worker, args, stack)`.
    Native_c04e @ 0x0087_9340 => extern "C" fn n_879340(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_c04e(&mut mem(), w, a, s)
    }
    /// `Native_8012(worker, args, stack)`.
    Native_8012 @ 0x0091_b240 => extern "C" fn n_91b240(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8012(&mut mem(), w, a, s)
    }
    /// `Native_8013(worker, args, stack)`.
    Native_8013 @ 0x0091_b260 => extern "C" fn n_91b260(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8013(&mut mem(), w, a, s)
    }
    /// `Native_8017(worker, args, stack)`.
    Native_8017 @ 0x0091_b320 => extern "C" fn n_91b320(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8017(&mut mem(), w, a, s)
    }
    /// `Native_8018(worker, args, stack)`.
    Native_8018 @ 0x0091_b340 => extern "C" fn n_91b340(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8018(&mut mem(), w, a, s)
    }
    /// `Native_801a(worker, args, stack)`.
    Native_801a @ 0x0091_b3a0 => extern "C" fn n_91b3a0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_801a(&mut mem(), w, a, s)
    }
    /// `Native_801b(worker, args, stack)`.
    Native_801b @ 0x0091_b3c0 => extern "C" fn n_91b3c0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_801b(&mut mem(), w, a, s)
    }
    /// `Native_801c(worker, args, stack)`.
    Native_801c @ 0x0091_b3e0 => extern "C" fn n_91b3e0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_801c(&mut mem(), w, a, s)
    }
    /// `Native_8031(worker, args, stack)`.
    Native_8031 @ 0x0091_b720 => extern "C" fn n_91b720(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8031(&mut mem(), w, a, s)
    }
    /// `Native_8065(worker, args, stack)`.
    Native_8065 @ 0x0091_c780 => extern "C" fn n_91c780(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8065(&mut mem(), w, a, s)
    }
    /// `Native_8015(worker, args, stack)`.
    Native_8015 @ 0x0091_b2c0 => extern "C" fn n_91b2c0(w: u32, a: u32, s: u32) -> f64 {
        native_sys::native_8015(&mut mem(), w, a, s)
    }
    /// `Native_8050(worker, args, stack)`.
    Native_8050 @ 0x0091_c1a0 => extern "C" fn n_91c1a0(w: u32, a: u32, s: u32) -> f64 {
        native_sys::native_8050(&mut mem(), w, a, s)
    }
    /// `Native_8051(worker, args, stack)`.
    Native_8051 @ 0x0091_c1c0 => extern "C" fn n_91c1c0(w: u32, a: u32, s: u32) -> f64 {
        native_sys::native_8051(&mut mem(), w, a, s)
    }
    /// `Native_8056(worker, args, stack)`.
    Native_8056 @ 0x0091_c330 => extern "C" fn n_91c330(w: u32, a: u32, s: u32) -> f64 {
        native_sys::native_8056(&mut mem(), w, a, s)
    }
    /// `Native_8057(worker, args, stack)`.
    Native_8057 @ 0x0091_c350 => extern "C" fn n_91c350(w: u32, a: u32, s: u32) -> f64 {
        native_sys::native_8057(&mut mem(), w, a, s)
    }
    /// `Native_805a(worker, args, stack)`.
    Native_805a @ 0x0091_c3f0 => extern "C" fn n_91c3f0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_805a(&mut mem(), w, a, s)
    }
    /// `Native_8061(worker, args, stack)`.
    Native_8061 @ 0x0091_c580 => extern "C" fn n_91c580(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8061(&mut mem(), w, a, s)
    }
    /// `Native_8016(worker, args, stack)`.
    Native_8016 @ 0x0091_b2e0 => extern "C" fn n_91b2e0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8016(&mut mem(), w, a, s)
    }
    /// `Native_8019(worker, args, stack)`.
    Native_8019 @ 0x0091_b360 => extern "C" fn n_91b360(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8019(&mut mem(), w, a, s)
    }
    /// `Native_8023(worker, args, stack)`.
    Native_8023 @ 0x0091_b4e0 => extern "C" fn n_91b4e0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8023(&mut mem(), w, a, s)
    }
    /// `Native_8028(worker, args, stack)`.
    Native_8028 @ 0x0091_b5d0 => extern "C" fn n_91b5d0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8028(&mut mem(), w, a, s)
    }
    /// `Native_802a(worker, args, stack)`.
    Native_802a @ 0x0091_b610 => extern "C" fn n_91b610(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_802a(&mut mem(), w, a, s)
    }
    /// `Native_8052(worker, args, stack)`.
    Native_8052 @ 0x0091_c1e0 => extern "C" fn n_91c1e0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8052(&mut mem(), w, a, s)
    }
    /// `Native_8055(worker, args, stack)`.
    Native_8055 @ 0x0091_c2d0 => extern "C" fn n_91c2d0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8055(&mut mem(), w, a, s)
    }
    /// `Native_8063(worker, args, stack)`.
    Native_8063 @ 0x0091_c600 => extern "C" fn n_91c600(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8063(&mut mem(), w, a, s)
    }
    /// `Native_8064(worker, args, stack)`.
    Native_8064 @ 0x0091_c6b0 => extern "C" fn n_91c6b0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8064(&mut mem(), w, a, s)
    }
    /// `Native_8046(worker, args, stack)`.
    Native_8046 @ 0x0091_bf10 => extern "C" fn n_91bf10(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8046(&mut mem(), w, a, s)
    }
    /// `Native_8047(worker, args, stack)`.
    Native_8047 @ 0x0091_bf60 => extern "C" fn n_91bf60(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8047(&mut mem(), w, a, s)
    }
    /// `Native_8048(worker, args, stack)`.
    Native_8048 @ 0x0091_bfb0 => extern "C" fn n_91bfb0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8048(&mut mem(), w, a, s)
    }
    /// `Native_8053(worker, args, stack)`.
    Native_8053 @ 0x0091_c230 => extern "C" fn n_91c230(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8053(&mut mem(), w, a, s)
    }
    /// `Native_8054(worker, args, stack)`.
    Native_8054 @ 0x0091_c280 => extern "C" fn n_91c280(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_8054(&mut mem(), w, a, s)
    }
    /// `Native_013c_MsgShowWaitEx(worker, args, stack)`.
    Native_013c_MsgShowWaitEx @ 0x0085_5d70 => extern "C" fn n_855d70(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_013c(&mut mem(), w, a, s)
    }
    /// `Native_0101_SndSepPanTranslate(worker, args, stack)`.
    Native_0101_SndSepPanTranslate @ 0x0085_5d90 => extern "C" fn n_855d90(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_0101(&mut mem(), w, a, s)
    }
    /// `Native_01c8_DisposeAnimOwnDobj(worker, args, stack)`.
    Native_01c8_DisposeAnimOwnDobj @ 0x0085_5e50 => extern "C" fn n_855e50(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_01c8(&mut mem(), w, a, s)
    }
    /// `Native_00a0_MsgSetMacroType(worker, args, stack)`.
    Native_00a0_MsgSetMacroType @ 0x0085_5ed0 => extern "C" fn n_855ed0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_00a0(&mut mem(), w, a, s)
    }
    /// `Native_019a_AttachChrDobj(worker, args, stack)`.
    Native_019a_AttachChrDobj @ 0x0085_6040 => extern "C" fn n_856040(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_019a(&mut mem(), w, a, s)
    }
    /// `Native_009f_MsgSetMacroValue(worker, args, stack)`.
    Native_009f_MsgSetMacroValue @ 0x0085_6090 => extern "C" fn n_856090(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_009f(&mut mem(), w, a, s)
    }
    /// `Native_00e5(worker, args, stack)`.
    Native_00e5 @ 0x0085_6130 => extern "C" fn n_856130(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_00e5(&mut mem(), w, a, s)
    }
    /// `Native_01d5(worker, args, stack)`.
    Native_01d5 @ 0x0085_61e0 => extern "C" fn n_8561e0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_01d5(&mut mem(), w, a, s)
    }
    /// `Native_00a1(worker, args, stack)`.
    Native_00a1 @ 0x0085_6220 => extern "C" fn n_856220(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_00a1(&mut mem(), w, a, s)
    }
    /// `Native_019b_AttachChrOwnDobj(worker, args, stack)`.
    Native_019b_AttachChrOwnDobj @ 0x0085_63e0 => extern "C" fn n_8563e0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_019b(&mut mem(), w, a, s)
    }
    /// `Native_001e(worker, args, stack)`.
    Native_001e @ 0x0085_6490 => extern "C" fn n_856490(w: u32, a: u32, s: u32) -> f64 {
        native_sys::native_001e(&mut mem(), w, a, s)
    }
    /// `Native_SetTouchRange_copy_00856600(worker, args, stack)`.
    Native_SetTouchRange_copy_00856600 @ 0x0085_6600 => extern "C" fn n_856600(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_set_touch_range(&mut mem(), w, a, s)
    }
    /// `Native_023e_SepLock(worker, args, stack)`.
    Native_023e_SepLock @ 0x0085_6640 => extern "C" fn n_856640(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_023e(&mut mem(), w, a, s)
    }
    /// `Native_01d7(worker, args, stack)`.
    Native_01d7 @ 0x0085_6740 => extern "C" fn n_856740(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_01d7(&mut mem(), w, a, s)
    }
    /// `Native_01db_VibrateStop(worker, args, stack)`.
    Native_01db_VibrateStop @ 0x0085_6e00 => extern "C" fn n_856e00(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_01db(&mut mem(), w, a, s)
    }
    /// `Native_01ef_VibrateStop2(worker, args, stack)`.
    Native_01ef_VibrateStop2 @ 0x0085_7320 => extern "C" fn n_857320(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_01ef(&mut mem(), w, a, s)
    }
    /// `Native_01dd(worker, args, stack)`.
    Native_01dd @ 0x0085_7660 => extern "C" fn n_857660(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_01dd(&mut mem(), w, a, s)
    }
    /// `Native_01de_Poll_copy_008577f0(worker, args, stack)`.
    Native_01de_Poll_copy_008577f0 @ 0x0085_77f0 => extern "C" fn n_8577f0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01de_poll(&mut mem(), w, a, s)
    }
    /// `Native_01ea_VibrateSetStreamExt(worker, args, stack)`.
    Native_01ea_VibrateSetStreamExt @ 0x0085_7aa0 => extern "C" fn n_857aa0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_01ea(&mut mem(), w, a, s)
    }
    /// `Native_00af_SetBattleSceneGroup(worker, args, stack)`.
    Native_00af_SetBattleSceneGroup @ 0x0085_8440 => extern "C" fn n_858440(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_00af(&mut mem(), w, a, s)
    }
    /// `Native_01a9(worker, args, stack)`.
    Native_01a9 @ 0x0085_8500 => extern "C" fn n_858500(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_01a9(&mut mem(), w, a, s)
    }
    /// `Native_01aa(worker, args, stack)`.
    Native_01aa @ 0x0085_8690 => extern "C" fn n_858690(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_01aa(&mut mem(), w, a, s)
    }
    /// `Native_01e1_ChrGetEffectLevel(worker, args, stack)`.
    Native_01e1_ChrGetEffectLevel @ 0x0085_86e0 => extern "C" fn n_8586e0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_01e1(&mut mem(), w, a, s)
    }
    /// `Native_01ae_RemoveMonsterCapture(worker, args, stack)`.
    Native_01ae_RemoveMonsterCapture @ 0x0085_8df0 => extern "C" fn n_858df0(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_01ae(&mut mem(), w, a, s)
    }
    /// `Native_00b6_GetTkAntiX(worker, args, stack)`.
    Native_00b6_GetTkAntiX @ 0x0085_8e20 => extern "C" fn n_858e20(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_00b6(&mut mem(), w, a, s)
    }
    /// `Native_00b7_GetTkAntiY(worker, args, stack)`.
    Native_00b7_GetTkAntiY @ 0x0085_9080 => extern "C" fn n_859080(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_00b7(&mut mem(), w, a, s)
    }
    /// `Native_00b8_GetTkAntiB(worker, args, stack)`.
    Native_00b8_GetTkAntiB @ 0x0085_9220 => extern "C" fn n_859220(w: u32, a: u32, s: u32) -> u32 {
        native_sys::native_00b8(&mut mem(), w, a, s)
    }
    /// `Native_100b(worker, args, stack)`.
    Native_100b @ 0x0087_78d0 => extern "C" fn n_8778d0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_100b(&mut mem(), w, a, s)
    }
    /// `AtelCall_Math_Ftoul_RI(worker, args, stack)`.
    AtelCall_Math_Ftoul_RI @ 0x0087_78f0 => extern "C" fn n_8778f0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::atel_call_math_ftoul_ri(&mut mem(), w, a, s)
    }
    /// `Native_100d_AngleTo15Bits(worker, args, stack)`.
    Native_100d_AngleTo15Bits @ 0x0087_7920 => extern "C" fn n_877920(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_100d(&mut mem(), w, a, s)
    }
    /// `Native_100e_AngleTo12Bits(worker, args, stack)`.
    Native_100e_AngleTo12Bits @ 0x0087_7950 => extern "C" fn n_877950(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_100e(&mut mem(), w, a, s)
    }
    /// `Native_100f_AngleFrom15Bits(worker, args, stack)`.
    Native_100f_AngleFrom15Bits @ 0x0087_7980 => extern "C" fn n_877980(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_100f(&mut mem(), w, a, s)
    }
    /// `Native_1010_AngleFrom12Bits(worker, args, stack)`.
    Native_1010_AngleFrom12Bits @ 0x0087_79a0 => extern "C" fn n_8779a0(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_1010(&mut mem(), w, a, s)
    }
    /// `AtelCall_Math_Tan_RF(worker, args, stack)`.
    AtelCall_Math_Tan_RF @ 0x0087_7ad0 => extern "C" fn n_877ad0(w: u32, a: u32, s: u32) -> f64 {
        native_mods::atel_call_math_tan_rf(&mut mem(), w, a, s)
    }
    /// `AtelCall_Math_ATan_RF(worker, args, stack)`.
    AtelCall_Math_ATan_RF @ 0x0087_7b00 => extern "C" fn n_877b00(w: u32, a: u32, s: u32) -> f64 {
        native_mods::atel_call_math_atan_rf(&mut mem(), w, a, s)
    }
    /// `AtelCall_Math_InvSqrt_RF(worker, args, stack)`.
    AtelCall_Math_InvSqrt_RF @ 0x0087_7ba0 => extern "C" fn n_877ba0(w: u32, a: u32, s: u32) -> f64 {
        native_mods::atel_call_math_inv_sqrt_rf(&mut mem(), w, a, s)
    }
    /// `Native_1008_SinTable(worker, args, stack)`.
    Native_1008_SinTable @ 0x0087_7bd0 => extern "C" fn n_877bd0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_1008(&mut mem(), w, a, s)
    }
    /// `Native_1009_CosTable(worker, args, stack)`.
    Native_1009_CosTable @ 0x0087_7bf0 => extern "C" fn n_877bf0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_1009(&mut mem(), w, a, s)
    }
    /// `Native_1011_Atan2XZ(worker, args, stack)`.
    Native_1011_Atan2XZ @ 0x0087_7c10 => extern "C" fn n_877c10(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_1011(&mut mem(), w, a, s)
    }
    /// `Native_1012_YawPitchTo(worker, args, stack)`.
    Native_1012_YawPitchTo @ 0x0087_7c80 => extern "C" fn n_877c80(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_1012(&mut mem(), w, a, s)
    }
    /// `Native_1014_AngleStep(worker, args, stack)`.
    Native_1014_AngleStep @ 0x0087_7d60 => extern "C" fn n_877d60(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_1014(&mut mem(), w, a, s)
    }
    /// `Native_1016_InRadius(worker, args, stack)`.
    Native_1016_InRadius @ 0x0087_7e50 => extern "C" fn n_877e50(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_1016(&mut mem(), w, a, s)
    }
    /// `Native_1017_CalcSpline(worker, args, stack)`.
    Native_1017_CalcSpline @ 0x0087_7ef0 => extern "C" fn n_877ef0(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_1017(&mut mem(), w, a, s)
    }
    /// `Native_1018_PolarToVector(worker, args, stack)`.
    Native_1018_PolarToVector @ 0x0087_7f60 => extern "C" fn n_877f60(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_1018(&mut mem(), w, a, s)
    }
    /// `Native_101d_SetVarSlot(worker, args, stack)`.
    Native_101d_SetVarSlot @ 0x0087_8110 => extern "C" fn n_878110(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_101d(&mut mem(), w, a, s)
    }
    /// `Native_402b_PLightGetEnable(worker, args, stack)`.
    Native_402b_PLightGetEnable @ 0x00a7_7880 => extern "C" fn n_a77880(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_402b(&mut mem(), w, a, s)
    }
    /// `Native_402c_PLightSetRadius(worker, args, stack)`.
    Native_402c_PLightSetRadius @ 0x00a7_78a0 => extern "C" fn n_a778a0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_402c(&mut mem(), w, a, s)
    }
    /// `Native_402d_PLightGetRadius(worker, args, stack)`.
    Native_402d_PLightGetRadius @ 0x00a7_78e0 => extern "C" fn n_a778e0(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_402d(&mut mem(), w, a, s)
    }
    /// `Native_402e_PLightSetLight(worker, args, stack)`.
    Native_402e_PLightSetLight @ 0x00a7_7900 => extern "C" fn n_a77900(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_402e(&mut mem(), w, a, s)
    }
    /// `Native_402f_PLightGetLight(worker, args, stack)`.
    Native_402f_PLightGetLight @ 0x00a7_7930 => extern "C" fn n_a77930(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_402f(&mut mem(), w, a, s)
    }
    /// `Native_4030_SetCameraFollow(worker, args, stack)`.
    Native_4030_SetCameraFollow @ 0x00a7_7950 => extern "C" fn n_a77950(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_4030(&mut mem(), w, a, s)
    }
    /// `Native_4032_PLightSetAttenuation(worker, args, stack)`.
    Native_4032_PLightSetAttenuation @ 0x00a7_7980 => extern "C" fn n_a77980(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_4032(&mut mem(), w, a, s)
    }
    /// `Native_4033_PLightGetAttenuation(worker, args, stack)`.
    Native_4033_PLightGetAttenuation @ 0x00a7_79c0 => extern "C" fn n_a779c0(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_4033(&mut mem(), w, a, s)
    }
    /// `Native_4037_GlobalGetWind(worker, args, stack)`.
    Native_4037_GlobalGetWind @ 0x00a7_7a70 => extern "C" fn n_a77a70(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_4037(&mut mem(), w, a, s)
    }
    /// `Native_403d_MemSetFast(worker, args, stack)`.
    Native_403d_MemSetFast @ 0x00a7_7bc0 => extern "C" fn n_a77bc0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_403d(&mut mem(), w, a, s)
    }
    /// `Native_4010(worker, args, stack)`.
    Native_4010 @ 0x00a7_7f40 => extern "C" fn n_a77f40(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_4010(&mut mem(), w, a, s)
    }
    /// `Native_4011_LightSetDir(worker, args, stack)`.
    Native_4011_LightSetDir @ 0x00a7_7f70 => extern "C" fn n_a77f70(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_4011(&mut mem(), w, a, s)
    }
    /// `Native_4012_LightSetColor(worker, args, stack)`.
    Native_4012_LightSetColor @ 0x00a7_7fd0 => extern "C" fn n_a77fd0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_4012(&mut mem(), w, a, s)
    }
    /// `Native_4018_IdGetMapTable(worker, args, stack)`.
    Native_4018_IdGetMapTable @ 0x00a7_8160 => extern "C" fn n_a78160(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_4018(&mut mem(), w, a, s)
    }
    /// `Native_4021_CacheDisposeChr(worker, args, stack)`.
    Native_4021_CacheDisposeChr @ 0x00a7_8300 => extern "C" fn n_a78300(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_4021(&mut mem(), w, a, s)
    }
    /// `Native_4026_PLightSetPos(worker, args, stack)`.
    Native_4026_PLightSetPos @ 0x00a7_8450 => extern "C" fn n_a78450(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_4026(&mut mem(), w, a, s)
    }
    /// `Native_4027_PLightGetPos(worker, args, stack)`.
    Native_4027_PLightGetPos @ 0x00a7_84c0 => extern "C" fn n_a784c0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_4027(&mut mem(), w, a, s)
    }
    /// `Native_4028_PLightSetColor(worker, args, stack)`.
    Native_4028_PLightSetColor @ 0x00a7_8510 => extern "C" fn n_a78510(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_4028(&mut mem(), w, a, s)
    }
    /// `Native_4029_PLightGetColor(worker, args, stack)`.
    Native_4029_PLightGetColor @ 0x00a7_8560 => extern "C" fn n_a78560(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_4029(&mut mem(), w, a, s)
    }
    /// `Native_5002_ChrSetSequence(worker, args, stack)`.
    Native_5002_ChrSetSequence @ 0x00a7_87a0 => extern "C" fn n_a787a0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5002(&mut mem(), w, a, s)
    }
    /// `Native_501d_ChrSetSequence(worker, args, stack)`.
    Native_501d_ChrSetSequence @ 0x00a7_9310 => extern "C" fn n_a79310(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5002(&mut mem(), w, a, s)
    }
    /// `Native_501f_copy_00a788d0(worker, args, stack)`.
    Native_501f_copy_00a788d0 @ 0x00a7_88d0 => extern "C" fn n_a788d0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5004(&mut mem(), w, a, s)
    }
    /// `Native_5053_ChrGetElemDirect(worker, args, stack)`.
    Native_5053_ChrGetElemDirect @ 0x00a7_8900 => extern "C" fn n_a78900(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5053(&mut mem(), w, a, s)
    }
    /// `Native_5055_ChrGetPartHide(worker, args, stack)`.
    Native_5055_ChrGetPartHide @ 0x00a7_8a10 => extern "C" fn n_a78a10(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5055(&mut mem(), w, a, s)
    }
    /// `Native_5059_ChrSetSysMotion(worker, args, stack)`.
    Native_5059_ChrSetSysMotion @ 0x00a7_8bd0 => extern "C" fn n_a78bd0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5059(&mut mem(), w, a, s)
    }
    /// `Native_505b_ChrSetImageBase(worker, args, stack)`.
    Native_505b_ChrSetImageBase @ 0x00a7_8ce0 => extern "C" fn n_a78ce0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_505b(&mut mem(), w, a, s)
    }
    /// `Native_506c_ChrSetPlightMask(worker, args, stack)`.
    Native_506c_ChrSetPlightMask @ 0x00a7_97b0 => extern "C" fn n_a797b0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_506c(&mut mem(), w, a, s)
    }
    /// `Native_500a_ChrSetHitHeight(worker, args, stack)`.
    Native_500a_ChrSetHitHeight @ 0x00a7_8c20 => extern "C" fn n_a78c20(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_500a(&mut mem(), w, a, s)
    }
    /// `Native_5011_ChrDisposeSystemMgrp(worker, args, stack)`.
    Native_5011_ChrDisposeSystemMgrp @ 0x00a7_8e50 => extern "C" fn n_a78e50(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5011(&mut mem(), w, a, s)
    }
    /// `Native_5062_ChrSyncSystemMgrp(worker, args, stack)`.
    Native_5062_ChrSyncSystemMgrp @ 0x00a7_9030 => extern "C" fn n_a79030(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5062(&mut mem(), w, a, s)
    }
    /// `Native_5062_ChrSyncSystemMgrp_Poll(worker, args, stack)`.
    Native_5062_ChrSyncSystemMgrp_Poll @ 0x00a7_9070 => extern "C" fn n_a79070(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5062_poll(&mut mem(), w, a, s)
    }
    /// `Native_5064(worker, args, stack)`.
    Native_5064 @ 0x00a7_9240 => extern "C" fn n_a79240(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5064(&mut mem(), w, a, s)
    }
    /// `Native_504f_Poll_copy_00a79290(worker, args, stack)`.
    Native_504f_Poll_copy_00a79290 @ 0x00a7_9290 => extern "C" fn n_a79290(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5064_poll(&mut mem(), w, a, s)
    }
    /// `Native_5023_NoOp(worker, args, stack)`.
    Native_5023_NoOp @ 0x00a7_95b0 => extern "C" fn n_a795b0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5023(&mut mem(), w, a, s)
    }
    /// `Native_5024_NoOp(worker, args, stack)`.
    Native_5024_NoOp @ 0x00a7_9620 => extern "C" fn n_a79620(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5023(&mut mem(), w, a, s)
    }
    /// `Native_502b_NoOp(worker, args, stack)`.
    Native_502b_NoOp @ 0x00a7_9b70 => extern "C" fn n_a79b70(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_502b(&mut mem(), w, a, s)
    }
    /// `Native_506b_ChrGetPlightEnable(worker, args, stack)`.
    Native_506b_ChrGetPlightEnable @ 0x00a7_96c0 => extern "C" fn n_a796c0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_506b(&mut mem(), w, a, s)
    }
    /// `Native_5070_ChrGetHideCalc(worker, args, stack)`.
    Native_5070_ChrGetHideCalc @ 0x00a7_9c30 => extern "C" fn n_a79c30(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5070(&mut mem(), w, a, s)
    }
    /// `Native_502e_ChrGetMapId(worker, args, stack)`.
    Native_502e_ChrGetMapId @ 0x00a7_9d10 => extern "C" fn n_a79d10(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_502e(&mut mem(), w, a, s)
    }
    /// `Native_502f_ChrMapIdOf(worker, args, stack)`.
    Native_502f_ChrMapIdOf @ 0x00a7_9d60 => extern "C" fn n_a79d60(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_502f(&mut mem(), w, a, s)
    }
    /// `Native_5074_ChrGetOverlapHit(worker, args, stack)`.
    Native_5074_ChrGetOverlapHit @ 0x00a7_9d90 => extern "C" fn n_a79d90(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5074(&mut mem(), w, a, s)
    }
    /// `Native_5030_ChrGetSoundId(worker, args, stack)`.
    Native_5030_ChrGetSoundId @ 0x00a7_9db0 => extern "C" fn n_a79db0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5030(&mut mem(), w, a, s)
    }
    /// `Native_5031_ChrSoundIdOf(worker, args, stack)`.
    Native_5031_ChrSoundIdOf @ 0x00a7_9e00 => extern "C" fn n_a79e00(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5031(&mut mem(), w, a, s)
    }
    /// `Native_5077_ChrGetAutoTalk(worker, args, stack)`.
    Native_5077_ChrGetAutoTalk @ 0x00a7_9e80 => extern "C" fn n_a79e80(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5077(&mut mem(), w, a, s)
    }
    /// `Native_5027_ChrElemPosOffset(worker, args, stack)`.
    Native_5027_ChrElemPosOffset @ 0x00a7_9860 => extern "C" fn n_a79860(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5027(&mut mem(), w, a, s)
    }
    /// `Native_5038_ChrCamRefPos(worker, args, stack)`.
    Native_5038_ChrCamRefPos @ 0x00a7_a250 => extern "C" fn n_a7a250(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5038(&mut mem(), w, a, s)
    }
    /// `Native_507f_ChrSetModelAlpha(worker, args, stack)`.
    Native_507f_ChrSetModelAlpha @ 0x00a7_a380 => extern "C" fn n_a7a380(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_507f(&mut mem(), w, a, s)
    }
    /// `Native_508a_ChrPlayInclOf(worker, args, stack)`.
    Native_508a_ChrPlayInclOf @ 0x00a7_a720 => extern "C" fn n_a7a720(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_508a(&mut mem(), w, a, s)
    }
    /// `Native_508c_ChrPlayInclOf(worker, args, stack)`.
    Native_508c_ChrPlayInclOf @ 0x00a7_a7a0 => extern "C" fn n_a7a7a0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_508a(&mut mem(), w, a, s)
    }
    /// `Native_5019_copy_00a7a740(worker, args, stack)`.
    Native_5019_copy_00a7a740 @ 0x00a7_a740 => extern "C" fn n_a7a740(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5041(&mut mem(), w, a, s)
    }
    /// `Native_5043_ChrPlayMarksOf(worker, args, stack)`.
    Native_5043_ChrPlayMarksOf @ 0x00a7_a7c0 => extern "C" fn n_a7a7c0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5043(&mut mem(), w, a, s)
    }
    /// `Native_5045_ChrPlayMarksOf(worker, args, stack)`.
    Native_5045_ChrPlayMarksOf @ 0x00a7_a830 => extern "C" fn n_a7a830(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_5043(&mut mem(), w, a, s)
    }
    /// `Native_0009_ChrGetHandle(worker, args, stack)`.
    Native_0009_ChrGetHandle @ 0x0085_f160 => extern "C" fn n_85f160(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0009(&mut mem(), w, a, s)
    }
    /// `Native_000c_SgSetPlayer(worker, args, stack)`.
    Native_000c_SgSetPlayer @ 0x0085_f640 => extern "C" fn n_85f640(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_000c(&mut mem(), w, a, s)
    }
    /// `Native_0014_MoveTargetXZ(worker, args, stack)`.
    Native_0014_MoveTargetXZ @ 0x0086_0120 => extern "C" fn n_860120(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0014(&mut mem(), w, a, s)
    }
    /// `Native_0022(worker, args, stack)`.
    Native_0022 @ 0x0085_9510 => extern "C" fn n_859510(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_0022(&mut mem(), w, a, s)
    }
    /// `Native_0023(worker, args, stack)`.
    Native_0023 @ 0x0085_91a0 => extern "C" fn n_8591a0(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_0023(&mut mem(), w, a, s)
    }
    /// `Native_0024(worker, args, stack)`.
    Native_0024 @ 0x0085_92f0 => extern "C" fn n_8592f0(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_0024(&mut mem(), w, a, s)
    }
    /// `Native_0026(worker, args, stack)`.
    Native_0026 @ 0x0085_9e50 => extern "C" fn n_859e50(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_0026(&mut mem(), w, a, s)
    }
    /// `Native_0027(worker, args, stack)`.
    Native_0027 @ 0x0085_9f30 => extern "C" fn n_859f30(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_0027(&mut mem(), w, a, s)
    }
    /// `Native_0031(worker, args, stack)`.
    Native_0031 @ 0x0085_6ba0 => extern "C" fn n_856ba0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0031(&mut mem(), w, a, s)
    }
    /// `Native_0032(worker, args, stack)`.
    Native_0032 @ 0x0085_6da0 => extern "C" fn n_856da0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0032(&mut mem(), w, a, s)
    }
    /// `Native_003b_MoveSetTargetWorker(worker, args, stack)`.
    Native_003b_MoveSetTargetWorker @ 0x0086_04a0 => extern "C" fn n_8604a0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_003b(&mut mem(), w, a, s)
    }
    /// `Native_003c_TurnToWorker(worker, args, stack)`.
    Native_003c_TurnToWorker @ 0x0085_5d50 => extern "C" fn n_855d50(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_003c(&mut mem(), w, a, s)
    }
    /// `Native_003e_CopyTurnAngles(worker, args, stack)`.
    Native_003e_CopyTurnAngles @ 0x0085_c470 => extern "C" fn n_85c470(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_003e(&mut mem(), w, a, s)
    }
    /// `Native_0040(worker, args, stack)`.
    Native_0040 @ 0x0085_c740 => extern "C" fn n_85c740(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_0040(&mut mem(), w, a, s)
    }
    /// `Native_0041(worker, args, stack)`.
    Native_0041 @ 0x0085_c840 => extern "C" fn n_85c840(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_0041(&mut mem(), w, a, s)
    }
    /// `Native_004e(worker, args, stack)`.
    Native_004e @ 0x0085_ea80 => extern "C" fn n_85ea80(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_004e(&mut mem(), w, a, s)
    }
    /// `Native_004f_PadIsRepeat(worker, args, stack)`.
    Native_004f_PadIsRepeat @ 0x0085_e3c0 => extern "C" fn n_85e3c0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_004f(&mut mem(), w, a, s)
    }
    /// `Native_0052(worker, args, stack)`.
    Native_0052 @ 0x0085_f2a0 => extern "C" fn n_85f2a0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0052(&mut mem(), w, a, s)
    }
    /// `Native_0053_PadIsRepeatRaw(worker, args, stack)`.
    Native_0053_PadIsRepeatRaw @ 0x0085_eda0 => extern "C" fn n_85eda0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0053(&mut mem(), w, a, s)
    }
    /// `Native_0067(worker, args, stack)`.
    Native_0067 @ 0x0085_8850 => extern "C" fn n_858850(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0067(&mut mem(), w, a, s)
    }
    /// `Native_0068(worker, args, stack)`.
    Native_0068 @ 0x0085_8640 => extern "C" fn n_858640(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0068(&mut mem(), w, a, s)
    }
    /// `Native_0072(worker, args, stack)`.
    Native_0072 @ 0x0085_bb50 => extern "C" fn n_85bb50(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0072(&mut mem(), w, a, s)
    }
    /// `Native_0073(worker, args, stack)`.
    Native_0073 @ 0x0085_bc90 => extern "C" fn n_85bc90(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0073(&mut mem(), w, a, s)
    }
    /// `Native_0075(worker, args, stack)`.
    Native_0075 @ 0x0085_bf40 => extern "C" fn n_85bf40(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0075(&mut mem(), w, a, s)
    }
    /// `Native_0079(worker, args, stack)`.
    Native_0079 @ 0x0085_c040 => extern "C" fn n_85c040(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0079(&mut mem(), w, a, s)
    }
    /// `Native_007b(worker, args, stack)`.
    Native_007b @ 0x0085_7ea0 => extern "C" fn n_857ea0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_007b(&mut mem(), w, a, s)
    }
    /// `Native_008e(worker, args, stack)`.
    Native_008e @ 0x0085_db40 => extern "C" fn n_85db40(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_008e(&mut mem(), w, a, s)
    }
    /// `Native_0099_SetPosOnSpline(worker, args, stack)`.
    Native_0099_SetPosOnSpline @ 0x0085_fd80 => extern "C" fn n_85fd80(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0099(&mut mem(), w, a, s)
    }
    /// `Native_009c_SetPosOnSpline3(worker, args, stack)`.
    Native_009c_SetPosOnSpline3 @ 0x0086_0000 => extern "C" fn n_860000(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_009c(&mut mem(), w, a, s)
    }
    /// `Native_00ad_SetSaveDicIndex(worker, args, stack)`.
    Native_00ad_SetSaveDicIndex @ 0x0085_81f0 => extern "C" fn n_8581f0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_00ad(&mut mem(), w, a, s)
    }
    /// `Native_00b5_SetTkAntiB(worker, args, stack)`.
    Native_00b5_SetTkAntiB @ 0x0085_8cf0 => extern "C" fn n_858cf0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_00b5(&mut mem(), w, a, s)
    }
    /// `Native_00c3_TurnIsActive(worker, args, stack)`.
    Native_00c3_TurnIsActive @ 0x0085_a130 => extern "C" fn n_85a130(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_00c3(&mut mem(), w, a, s)
    }
    /// `Native_00cd_WorkerInParty(worker, args, stack)`.
    Native_00cd_WorkerInParty @ 0x0085_bbe0 => extern "C" fn n_85bbe0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_00cd(&mut mem(), w, a, s)
    }
    /// `Native_00d3_StopMove(worker, args, stack)`.
    Native_00d3_StopMove @ 0x0085_c7d0 => extern "C" fn n_85c7d0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_00d3(&mut mem(), w, a, s)
    }
    /// `Native_00d4_StopTurn(worker, args, stack)`.
    Native_00d4_StopTurn @ 0x0085_c910 => extern "C" fn n_85c910(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_00d4(&mut mem(), w, a, s)
    }
    /// `Native_00f0_LineSetTable(worker, args, stack)`.
    Native_00f0_LineSetTable @ 0x0085_76a0 => extern "C" fn n_8576a0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_00f0(&mut mem(), w, a, s)
    }
    /// `Native_00f1_LineGetActorSide(worker, args, stack)`.
    Native_00f1_LineGetActorSide @ 0x0085_d620 => extern "C" fn n_85d620(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_00f1(&mut mem(), w, a, s)
    }
    /// `Native_00f7_WaitPartySignal(worker, args, stack)`.
    Native_00f7_WaitPartySignal @ 0x0085_8290 => extern "C" fn n_858290(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_00f7(&mut mem(), w, a, s)
    }
    /// `Native_00f7_WaitPartySignal_Poll(worker, args, stack)`.
    Native_00f7_WaitPartySignal_Poll @ 0x0085_8400 => extern "C" fn n_858400(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_00f7_poll(&mut mem(), w, a, s)
    }
    /// `Native_00fc_SetSaveEventBit(worker, args, stack)`.
    Native_00fc_SetSaveEventBit @ 0x0085_8dd0 => extern "C" fn n_858dd0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_00fc(&mut mem(), w, a, s)
    }
    /// `Native_00ff_SndSepPanDirect(worker, args, stack)`.
    Native_00ff_SndSepPanDirect @ 0x0086_02c0 => extern "C" fn n_8602c0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_00ff(&mut mem(), w, a, s)
    }
    /// `Native_0118(worker, args, stack)`.
    Native_0118 @ 0x0085_bae0 => extern "C" fn n_85bae0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0118(&mut mem(), w, a, s)
    }
    /// `Native_011e_SpriteSetAlpha(worker, args, stack)`.
    Native_011e_SpriteSetAlpha @ 0x0085_caf0 => extern "C" fn n_85caf0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_011e(&mut mem(), w, a, s)
    }
    /// `Native_0123(worker, args, stack)`.
    Native_0123 @ 0x0085_d440 => extern "C" fn n_85d440(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0079(&mut mem(), w, a, s)
    }
    /// `Native_0124(worker, args, stack)`.
    Native_0124 @ 0x0085_d530 => extern "C" fn n_85d530(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0079(&mut mem(), w, a, s)
    }
    /// `Native_0125_SetPosXZ(worker, args, stack)`.
    Native_0125_SetPosXZ @ 0x0085_fc50 => extern "C" fn n_85fc50(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0125(&mut mem(), w, a, s)
    }
    /// `Native_012c(worker, args, stack)`.
    Native_012c @ 0x0085_ddd0 => extern "C" fn n_85ddd0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_012c(&mut mem(), w, a, s)
    }
    /// `Native_012f_SaveCounterSub(worker, args, stack)`.
    Native_012f_SaveCounterSub @ 0x0085_e170 => extern "C" fn n_85e170(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_012f(&mut mem(), w, a, s)
    }
    /// `Native_0131_SaveBitClear(worker, args, stack)`.
    Native_0131_SaveBitClear @ 0x0085_e760 => extern "C" fn n_85e760(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0131(&mut mem(), w, a, s)
    }
    /// `Native_013a_MsgChoiceEx(worker, args, stack)`.
    Native_013a_MsgChoiceEx @ 0x0085_fb30 => extern "C" fn n_85fb30(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_013a(&mut mem(), w, a, s)
    }
    /// `Native_013a_MsgChoiceEx_Poll(worker, args, stack)`.
    Native_013a_MsgChoiceEx_Poll @ 0x0085_fcd0 => extern "C" fn n_85fcd0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_013a_poll(&mut mem(), w, a, s)
    }
    /// `Native_013a_MsgChoiceEx_Result(worker, args, stack)`.
    Native_013a_MsgChoiceEx_Result @ 0x0085_fd40 => extern "C" fn n_85fd40(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_013a_result(&mut mem(), w, a, s)
    }
    /// `Native_0147_MinimapSetOwnMarker(worker, args, stack)`.
    Native_0147_MinimapSetOwnMarker @ 0x0085_8250 => extern "C" fn n_858250(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0147(&mut mem(), w, a, s)
    }
    /// `Native_014b_SndSetFoot3d(worker, args, stack)`.
    Native_014b_SndSetFoot3d @ 0x0085_88e0 => extern "C" fn n_8588e0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_014b(&mut mem(), w, a, s)
    }
    /// `Native_014c_SndSetFoot3dSlot(worker, args, stack)`.
    Native_014c_SndSetFoot3dSlot @ 0x0085_8af0 => extern "C" fn n_858af0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_014c(&mut mem(), w, a, s)
    }
    /// `Native_0150_SndFoot3dOn(worker, args, stack)`.
    Native_0150_SndFoot3dOn @ 0x0085_9380 => extern "C" fn n_859380(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0150(&mut mem(), w, a, s)
    }
    /// `Native_0152_SndFoot3dOff(worker, args, stack)`.
    Native_0152_SndFoot3dOff @ 0x0085_94f0 => extern "C" fn n_8594f0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0152(&mut mem(), w, a, s)
    }
    /// `Native_0153_SndFoot3dOffSlot(worker, args, stack)`.
    Native_0153_SndFoot3dOffSlot @ 0x0085_9600 => extern "C" fn n_859600(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0153(&mut mem(), w, a, s)
    }
    /// `Native_015a_MsgSetTextRaw(worker, args, stack)`.
    Native_015a_MsgSetTextRaw @ 0x0085_7c60 => extern "C" fn n_857c60(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_015a(&mut mem(), w, a, s)
    }
    /// `Native_015c_SetFootKickStay(worker, args, stack)`.
    Native_015c_SetFootKickStay @ 0x0085_b530 => extern "C" fn n_85b530(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_015c(&mut mem(), w, a, s)
    }
    /// `Native_0162_SndKinuOn(worker, args, stack)`.
    Native_0162_SndKinuOn @ 0x0085_9e90 => extern "C" fn n_859e90(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0162(&mut mem(), w, a, s)
    }
    /// `Native_0163_SndKinuOnSlot(worker, args, stack)`.
    Native_0163_SndKinuOnSlot @ 0x0085_a0f0 => extern "C" fn n_85a0f0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0163(&mut mem(), w, a, s)
    }
    /// `Native_0164_SndKinuOff(worker, args, stack)`.
    Native_0164_SndKinuOff @ 0x0085_9f70 => extern "C" fn n_859f70(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0164(&mut mem(), w, a, s)
    }
    /// `Native_0165_SndKinuOffSlot(worker, args, stack)`.
    Native_0165_SndKinuOffSlot @ 0x0085_a1d0 => extern "C" fn n_85a1d0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0165(&mut mem(), w, a, s)
    }
    /// `Native_0168_MusicPlayFast(worker, args, stack)`.
    Native_0168_MusicPlayFast @ 0x0085_b990 => extern "C" fn n_85b990(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0168(&mut mem(), w, a, s)
    }
    /// `Native_0169_MusicSetMute(worker, args, stack)`.
    Native_0169_MusicSetMute @ 0x0085_bab0 => extern "C" fn n_85bab0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0169(&mut mem(), w, a, s)
    }
    /// `Native_0170_BackupEventJump(worker, args, stack)`.
    Native_0170_BackupEventJump @ 0x0085_c530 => extern "C" fn n_85c530(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0170(&mut mem(), w, a, s)
    }
    /// `Native_0173(worker, args, stack)`.
    Native_0173 @ 0x0085_e7a0 => extern "C" fn n_85e7a0(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_0173(&mut mem(), w, a, s)
    }
    /// `Native_0175(worker, args, stack)`.
    Native_0175 @ 0x0085_eec0 => extern "C" fn n_85eec0(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_0175(&mut mem(), w, a, s)
    }
    /// `Native_0176(worker, args, stack)`.
    Native_0176 @ 0x0085_ec30 => extern "C" fn n_85ec30(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_0176(&mut mem(), w, a, s)
    }
    /// `Native_0178_GetEffectFlag(worker, args, stack)`.
    Native_0178_GetEffectFlag @ 0x0085_cc30 => extern "C" fn n_85cc30(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0178(&mut mem(), w, a, s)
    }
    /// `Native_017d_MusicMultiStart(worker, args, stack)`.
    Native_017d_MusicMultiStart @ 0x0085_d680 => extern "C" fn n_85d680(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_017d(&mut mem(), w, a, s)
    }
    /// `Native_0182(worker, args, stack)`.
    Native_0182 @ 0x0085_ed10 => extern "C" fn n_85ed10(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_0182(&mut mem(), w, a, s)
    }
    /// `Native_0183(worker, args, stack)`.
    Native_0183 @ 0x0085_f690 => extern "C" fn n_85f690(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_0183(&mut mem(), w, a, s)
    }
    /// `Native_0186_MusicPitchDirect(worker, args, stack)`.
    Native_0186_MusicPitchDirect @ 0x0085_dd50 => extern "C" fn n_85dd50(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0186(&mut mem(), w, a, s)
    }
    /// `Native_0187_MusicPitchTranslate(worker, args, stack)`.
    Native_0187_MusicPitchTranslate @ 0x0085_df00 => extern "C" fn n_85df00(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0187(&mut mem(), w, a, s)
    }
    /// `Native_018c_SfxPitchDirect(worker, args, stack)`.
    Native_018c_SfxPitchDirect @ 0x0085_ed60 => extern "C" fn n_85ed60(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_018c(&mut mem(), w, a, s)
    }
    /// `Native_018d_SfxPitchTranslate(worker, args, stack)`.
    Native_018d_SfxPitchTranslate @ 0x0085_ef00 => extern "C" fn n_85ef00(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_018d(&mut mem(), w, a, s)
    }
    /// `Native_018e_VoiceVolumeDirect(worker, args, stack)`.
    Native_018e_VoiceVolumeDirect @ 0x0085_f110 => extern "C" fn n_85f110(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_018e(&mut mem(), w, a, s)
    }
    /// `Native_0190_VoicePanDirect(worker, args, stack)`.
    Native_0190_VoicePanDirect @ 0x0085_f330 => extern "C" fn n_85f330(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0190(&mut mem(), w, a, s)
    }
    /// `Native_0191_VoicePanTranslate(worker, args, stack)`.
    Native_0191_VoicePanTranslate @ 0x0085_f5c0 => extern "C" fn n_85f5c0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0191(&mut mem(), w, a, s)
    }
    /// `Native_01a1_DobjSetMimeParam(worker, args, stack)`.
    Native_01a1_DobjSetMimeParam @ 0x0085_6b10 => extern "C" fn n_856b10(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01a1(&mut mem(), w, a, s)
    }
    /// `Native_01a2_DobjSetMimeParam3(worker, args, stack)`.
    Native_01a2_DobjSetMimeParam3 @ 0x0085_6e90 => extern "C" fn n_856e90(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01a2(&mut mem(), w, a, s)
    }
    /// `Native_01a3_DobjSetMimeParam2(worker, args, stack)`.
    Native_01a3_DobjSetMimeParam2 @ 0x0085_7210 => extern "C" fn n_857210(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01a3(&mut mem(), w, a, s)
    }
    /// `Native_01cb(worker, args, stack)`.
    Native_01cb @ 0x0085_e0e0 => extern "C" fn n_85e0e0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01cb(&mut mem(), w, a, s)
    }
    /// `Native_01d6(worker, args, stack)`.
    Native_01d6 @ 0x0085_64b0 => extern "C" fn n_8564b0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01d6(&mut mem(), w, a, s)
    }
    /// `Native_01d8_VibrateStartLevel(worker, args, stack)`.
    Native_01d8_VibrateStartLevel @ 0x0085_67e0 => extern "C" fn n_8567e0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01d8(&mut mem(), w, a, s)
    }
    /// `Native_01df(worker, args, stack)`.
    Native_01df @ 0x0085_f8d0 => extern "C" fn n_85f8d0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01df(&mut mem(), w, a, s)
    }
    /// `Native_01e8_SepClear(worker, args, stack)`.
    Native_01e8_SepClear @ 0x0085_9470 => extern "C" fn n_859470(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01e8(&mut mem(), w, a, s)
    }
    /// `Native_01ec_VibrateSetStream(worker, args, stack)`.
    Native_01ec_VibrateSetStream @ 0x0085_7cf0 => extern "C" fn n_857cf0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01ec(&mut mem(), w, a, s)
    }
    /// `Native_01f5_SetDummyCtrlFlag(worker, args, stack)`.
    Native_01f5_SetDummyCtrlFlag @ 0x0085_a160 => extern "C" fn n_85a160(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01f5(&mut mem(), w, a, s)
    }
    /// `Native_01f6_SetDummyCtrlPending(worker, args, stack)`.
    Native_01f6_SetDummyCtrlPending @ 0x0085_a2d0 => extern "C" fn n_85a2d0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01f6(&mut mem(), w, a, s)
    }
    /// `Native_01f7_BtlEndMovieFadeout(worker, args, stack)`.
    Native_01f7_BtlEndMovieFadeout @ 0x0085_a800 => extern "C" fn n_85a800(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01f7(&mut mem(), w, a, s)
    }
    /// `Native_01fd_SetSaveCommand(worker, args, stack)`.
    Native_01fd_SetSaveCommand @ 0x0085_ba20 => extern "C" fn n_85ba20(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01fd(&mut mem(), w, a, s)
    }
    /// `Native_01fe_GetSaveCommand(worker, args, stack)`.
    Native_01fe_GetSaveCommand @ 0x0085_bb20 => extern "C" fn n_85bb20(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01fe(&mut mem(), w, a, s)
    }
    /// `Native_01ff_FootSoundOn(worker, args, stack)`.
    Native_01ff_FootSoundOn @ 0x0085_a300 => extern "C" fn n_85a300(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01ff(&mut mem(), w, a, s)
    }
    /// `Native_0208_BindEffBreathOff(worker, args, stack)`.
    Native_0208_BindEffBreathOff @ 0x0085_f210 => extern "C" fn n_85f210(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0208(&mut mem(), w, a, s)
    }
    /// `Native_020c(worker, args, stack)`.
    Native_020c @ 0x0085_bcc0 => extern "C" fn n_85bcc0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_020c(&mut mem(), w, a, s)
    }
    /// `Native_0211_ClearHelpEventFlag(worker, args, stack)`.
    Native_0211_ClearHelpEventFlag @ 0x0085_c810 => extern "C" fn n_85c810(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0211(&mut mem(), w, a, s)
    }
    /// `Native_0214_ChrGetOptposElem(worker, args, stack)`.
    Native_0214_ChrGetOptposElem @ 0x0085_cbe0 => extern "C" fn n_85cbe0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0214(&mut mem(), w, a, s)
    }
    /// `Native_0218_GetMonsterCapture(worker, args, stack)`.
    Native_0218_GetMonsterCapture @ 0x0085_d3f0 => extern "C" fn n_85d3f0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0218(&mut mem(), w, a, s)
    }
    /// `Native_0219_AddMonsterCaptureN(worker, args, stack)`.
    Native_0219_AddMonsterCaptureN @ 0x0085_d500 => extern "C" fn n_85d500(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0219(&mut mem(), w, a, s)
    }
    /// `Native_021c_CheckCaptMonster(worker, args, stack)`.
    Native_021c_CheckCaptMonster @ 0x0085_db90 => extern "C" fn n_85db90(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_021c(&mut mem(), w, a, s)
    }
    /// `Native_021f_EnableLoveParam(worker, args, stack)`.
    Native_021f_EnableLoveParam @ 0x0085_dfc0 => extern "C" fn n_85dfc0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_021f(&mut mem(), w, a, s)
    }
    /// `Native_0222(worker, args, stack)`.
    Native_0222 @ 0x0085_e380 => extern "C" fn n_85e380(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0222(&mut mem(), w, a, s)
    }
    /// `Native_0223_SetVibrateConfig(worker, args, stack)`.
    Native_0223_SetVibrateConfig @ 0x0085_e460 => extern "C" fn n_85e460(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0223(&mut mem(), w, a, s)
    }
    /// `Native_0227(worker, args, stack)`.
    Native_0227 @ 0x0085_ac20 => extern "C" fn n_85ac20(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0227(&mut mem(), w, a, s)
    }
    /// `Native_0228(worker, args, stack)`.
    Native_0228 @ 0x0085_b060 => extern "C" fn n_85b060(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0228(&mut mem(), w, a, s)
    }
    /// `Native_022b_Poll(worker, args, stack)`.
    Native_022b_Poll @ 0x0085_efb0 => extern "C" fn n_85efb0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_022b_poll(&mut mem(), w, a, s)
    }
    /// `Native_022c(worker, args, stack)`.
    Native_022c @ 0x0085_f2e0 => extern "C" fn n_85f2e0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_022c(&mut mem(), w, a, s)
    }
    /// `Native_022d_WaitPadButton(worker, args, stack)`.
    Native_022d_WaitPadButton @ 0x0085_f4c0 => extern "C" fn n_85f4c0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_022d(&mut mem(), w, a, s)
    }
    /// `Native_022d_WaitPadButton_Poll(worker, args, stack)`.
    Native_022d_WaitPadButton_Poll @ 0x0085_f600 => extern "C" fn n_85f600(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_022d_poll(&mut mem(), w, a, s)
    }
    /// `Native_0232_PulseDummyCtrlPending(worker, args, stack)`.
    Native_0232_PulseDummyCtrlPending @ 0x0085_a4b0 => extern "C" fn n_85a4b0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0232(&mut mem(), w, a, s)
    }
    /// `Native_0232_PulseDummyCtrlPending_Poll(worker, args, stack)`.
    Native_0232_PulseDummyCtrlPending_Poll @ 0x0085_a5f0 => extern "C" fn n_85a5f0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0232_poll(&mut mem(), w, a, s)
    }
    /// `Native_0233_SetMainFrameRatio(worker, args, stack)`.
    Native_0233_SetMainFrameRatio @ 0x0085_fe10 => extern "C" fn n_85fe10(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0233(&mut mem(), w, a, s)
    }
    /// `Native_0238_GetEventFlag(worker, args, stack)`.
    Native_0238_GetEventFlag @ 0x0085_eb30 => extern "C" fn n_85eb30(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0238(&mut mem(), w, a, s)
    }
    /// `Native_024e(worker, args, stack)`.
    Native_024e @ 0x0085_d4b0 => extern "C" fn n_85d4b0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_024e(&mut mem(), w, a, s)
    }
    /// `Native_0252(worker, args, stack)`.
    Native_0252 @ 0x0085_ef50 => extern "C" fn n_85ef50(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0252(&mut mem(), w, a, s)
    }
    /// `Native_0258_SetEnglishMode(worker, args, stack)`.
    Native_0258_SetEnglishMode @ 0x0085_8a60 => extern "C" fn n_858a60(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0258(&mut mem(), w, a, s)
    }
    /// `Native_0258_SetEnglishMode_Poll(worker, args, stack)`.
    Native_0258_SetEnglishMode_Poll @ 0x0085_8da0 => extern "C" fn n_858da0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0258_poll(&mut mem(), w, a, s)
    }
    /// `Native_6009_CamSetTarget(worker, args, stack)`.
    Native_6009_CamSetTarget @ 0x007b_93e0 => extern "C" fn n_7b93e0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6009(&mut mem(), w, a, s)
    }
    /// `Native_600b_CamAddLineArea(worker, args, stack)`.
    Native_600b_CamAddLineArea @ 0x007b_9480 => extern "C" fn n_7b9480(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_600b(&mut mem(), w, a, s)
    }
    /// `Native_600c(worker, args, stack)`.
    Native_600c @ 0x007b_94c0 => extern "C" fn n_7b94c0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_600c(&mut mem(), w, a, s)
    }
    /// `Native_600d_CamAddCircleArea(worker, args, stack)`.
    Native_600d_CamAddCircleArea @ 0x007b_9500 => extern "C" fn n_7b9500(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_600d(&mut mem(), w, a, s)
    }
    /// `Native_600e(worker, args, stack)`.
    Native_600e @ 0x007b_95a0 => extern "C" fn n_7b95a0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_600e(&mut mem(), w, a, s)
    }
    /// `Native_6018_CamSetDir(worker, args, stack)`.
    Native_6018_CamSetDir @ 0x007b_9a80 => extern "C" fn n_7b9a80(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6018(&mut mem(), w, a, s)
    }
    /// `Native_6022_CamRefSetPolar(worker, args, stack)`.
    Native_6022_CamRefSetPolar @ 0x007b_9e10 => extern "C" fn n_7b9e10(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6022(&mut mem(), w, a, s)
    }
    /// `Native_6023_CamRefAddPolar(worker, args, stack)`.
    Native_6023_CamRefAddPolar @ 0x007b_9e90 => extern "C" fn n_7b9e90(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6023(&mut mem(), w, a, s)
    }
    /// `Native_6024(worker, args, stack)`.
    Native_6024 @ 0x007b_9f20 => extern "C" fn n_7b9f20(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6024(&mut mem(), w, a, s)
    }
    /// `Native_6025(worker, args, stack)`.
    Native_6025 @ 0x007b_9fa0 => extern "C" fn n_7b9fa0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6025(&mut mem(), w, a, s)
    }
    /// `Native_6026(worker, args, stack)`.
    Native_6026 @ 0x007b_9fc0 => extern "C" fn n_7b9fc0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6026(&mut mem(), w, a, s)
    }
    /// `Native_602c(worker, args, stack)`.
    Native_602c @ 0x007b_a350 => extern "C" fn n_7ba350(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_602c(&mut mem(), w, a, s)
    }
    /// `Native_602f(worker, args, stack)`.
    Native_602f @ 0x007b_7fe0 => extern "C" fn n_7b7fe0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_602f(&mut mem(), w, a, s)
    }
    /// `Native_6031(worker, args, stack)`.
    Native_6031 @ 0x007b_80c0 => extern "C" fn n_7b80c0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6031(&mut mem(), w, a, s)
    }
    /// `Native_6036_CamRefSetDir(worker, args, stack)`.
    Native_6036_CamRefSetDir @ 0x007b_8190 => extern "C" fn n_7b8190(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6036(&mut mem(), w, a, s)
    }
    /// `Native_6037_CamRefResetDir(worker, args, stack)`.
    Native_6037_CamRefResetDir @ 0x007b_81b0 => extern "C" fn n_7b81b0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6037(&mut mem(), w, a, s)
    }
    /// `Native_6039_CamFocusIsMoving(worker, args, stack)`.
    Native_6039_CamFocusIsMoving @ 0x007b_82d0 => extern "C" fn n_7b82d0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6039(&mut mem(), w, a, s)
    }
    /// `Native_603d_CamRefSetYaw(worker, args, stack)`.
    Native_603d_CamRefSetYaw @ 0x007b_a000 => extern "C" fn n_7ba000(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_603d(&mut mem(), w, a, s)
    }
    /// `Native_603e_CamSetBtl(worker, args, stack)`.
    Native_603e_CamSetBtl @ 0x007b_86d0 => extern "C" fn n_7b86d0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_603e(&mut mem(), w, a, s)
    }
    /// `Native_6053_ScreenMoveKind1(worker, args, stack)`.
    Native_6053_ScreenMoveKind1 @ 0x007b_8c60 => extern "C" fn n_7b8c60(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6053(&mut mem(), w, a, s)
    }
    /// `Native_6054_ScreenMoveKind3(worker, args, stack)`.
    Native_6054_ScreenMoveKind3 @ 0x007b_8c90 => extern "C" fn n_7b8c90(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6054(&mut mem(), w, a, s)
    }
    /// `Native_6056_ScreenMoveSide1Kind1(worker, args, stack)`.
    Native_6056_ScreenMoveSide1Kind1 @ 0x007b_8d10 => extern "C" fn n_7b8d10(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6056(&mut mem(), w, a, s)
    }
    /// `Native_6057_ScreenMoveSide1Kind3(worker, args, stack)`.
    Native_6057_ScreenMoveSide1Kind3 @ 0x007b_8d40 => extern "C" fn n_7b8d40(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6057(&mut mem(), w, a, s)
    }
    /// `Native_6065_CamSetPriority(worker, args, stack)`.
    Native_6065_CamSetPriority @ 0x007b_87d0 => extern "C" fn n_7b87d0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6065(&mut mem(), w, a, s)
    }
    /// `Native_6068_ScreenShakeKind2(worker, args, stack)`.
    Native_6068_ScreenShakeKind2 @ 0x007b_9460 => extern "C" fn n_7b9460(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6068(&mut mem(), w, a, s)
    }
    /// `Native_606a_CamEyeShakeKind3(worker, args, stack)`.
    Native_606a_CamEyeShakeKind3 @ 0x007b_9090 => extern "C" fn n_7b9090(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_606a(&mut mem(), w, a, s)
    }
    /// `Native_606d_CamEyeShakeKind4(worker, args, stack)`.
    Native_606d_CamEyeShakeKind4 @ 0x007b_9170 => extern "C" fn n_7b9170(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_606d(&mut mem(), w, a, s)
    }
    /// `Native_606f_CamFocusShakeKind5(worker, args, stack)`.
    Native_606f_CamFocusShakeKind5 @ 0x007b_91d0 => extern "C" fn n_7b91d0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_606f(&mut mem(), w, a, s)
    }
    /// `Native_6070_CamEyeShakeKind5(worker, args, stack)`.
    Native_6070_CamEyeShakeKind5 @ 0x007b_9200 => extern "C" fn n_7b9200(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6070(&mut mem(), w, a, s)
    }
    /// `Native_6072_CamFocusShakeKind6(worker, args, stack)`.
    Native_6072_CamFocusShakeKind6 @ 0x007b_9240 => extern "C" fn n_7b9240(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6072(&mut mem(), w, a, s)
    }
    /// `Native_6073_CamEyeShakeKind6(worker, args, stack)`.
    Native_6073_CamEyeShakeKind6 @ 0x007b_9280 => extern "C" fn n_7b9280(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6073(&mut mem(), w, a, s)
    }
    /// `Native_6075_ScreenSetCamNo(worker, args, stack)`.
    Native_6075_ScreenSetCamNo @ 0x007b_8b50 => extern "C" fn n_7b8b50(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6075(&mut mem(), w, a, s)
    }
    /// `Native_6079_ScreenMoveReset(worker, args, stack)`.
    Native_6079_ScreenMoveReset @ 0x007b_8dc0 => extern "C" fn n_7b8dc0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6079(&mut mem(), w, a, s)
    }
    /// `Native_607a_ScreenMoveResetSide1(worker, args, stack)`.
    Native_607a_ScreenMoveResetSide1 @ 0x007b_8de0 => extern "C" fn n_7b8de0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_607a(&mut mem(), w, a, s)
    }
    /// `Native_607c_ScreenMoveWaitSide1(worker, args, stack)`.
    Native_607c_ScreenMoveWaitSide1 @ 0x007b_8e00 => extern "C" fn n_7b8e00(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_607c(&mut mem(), w, a, s)
    }
    /// `Native_607c_ScreenMoveWaitSide1_Poll(worker, args, stack)`.
    Native_607c_ScreenMoveWaitSide1_Poll @ 0x007b_8e20 => extern "C" fn n_7b8e20(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_607c_poll(&mut mem(), w, a, s)
    }
    /// `Native_6085(worker, args, stack)`.
    Native_6085 @ 0x007b_92a0 => extern "C" fn n_7b92a0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6085(&mut mem(), w, a, s)
    }
    /// `Native_6086(worker, args, stack)`.
    Native_6086 @ 0x007b_9d80 => extern "C" fn n_7b9d80(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6086(&mut mem(), w, a, s)
    }
    /// `Native_6089_ScreenShakeKind9(worker, args, stack)`.
    Native_6089_ScreenShakeKind9 @ 0x007b_9630 => extern "C" fn n_7b9630(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_6089(&mut mem(), w, a, s)
    }
    /// `Native_BtlSetFlagBded(worker, args, stack)`.
    Native_BtlSetFlagBded @ 0x007a_39d0 => extern "C" fn n_7a39d0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7001(&mut mem(), w, a, s)
    }
    /// `Native_700e(worker, args, stack)`.
    Native_700e @ 0x007a_5690 => extern "C" fn n_7a5690(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_700e(&mut mem(), w, a, s)
    }
    /// `Native_7042(worker, args, stack)`.
    Native_7042 @ 0x007a_6060 => extern "C" fn n_7a6060(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7042(&mut mem(), w, a, s)
    }
    /// `Native_7044(worker, args, stack)`.
    Native_7044 @ 0x007a_6e10 => extern "C" fn n_7a6e10(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7044(&mut mem(), w, a, s)
    }
    /// `Native_7045(worker, args, stack)`.
    Native_7045 @ 0x007a_4da0 => extern "C" fn n_7a4da0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7045(&mut mem(), w, a, s)
    }
    /// `Native_7056(worker, args, stack)`.
    Native_7056 @ 0x007a_6e80 => extern "C" fn n_7a6e80(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7056(&mut mem(), w, a, s)
    }
    /// `Native_BtlFindActor(worker, args, stack)`.
    Native_BtlFindActor @ 0x007a_6170 => extern "C" fn n_7a6170(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7069(&mut mem(), w, a, s)
    }
    /// `Native_706c(worker, args, stack)`.
    Native_706c @ 0x007a_6bc0 => extern "C" fn n_7a6bc0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_706c(&mut mem(), w, a, s)
    }
    /// `Native_7072(worker, args, stack)`.
    Native_7072 @ 0x007a_6330 => extern "C" fn n_7a6330(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7072(&mut mem(), w, a, s)
    }
    /// `Native_7074(worker, args, stack)`.
    Native_7074 @ 0x007a_29f0 => extern "C" fn n_7a29f0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7074(&mut mem(), w, a, s)
    }
    /// `Native_7077(worker, args, stack)`.
    Native_7077 @ 0x007a_5450 => extern "C" fn n_7a5450(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7077(&mut mem(), w, a, s)
    }
    /// `Native_7079(worker, args, stack)`.
    Native_7079 @ 0x007a_7200 => extern "C" fn n_7a7200(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7079(&mut mem(), w, a, s)
    }
    /// `Native_7080(worker, args, stack)`.
    Native_7080 @ 0x007a_2c70 => extern "C" fn n_7a2c70(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7080(&mut mem(), w, a, s)
    }
    /// `Native_7089(worker, args, stack)`.
    Native_7089 @ 0x007a_7c40 => extern "C" fn n_7a7c40(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7089(&mut mem(), w, a, s)
    }
    /// `Native_7090(worker, args, stack)`.
    Native_7090 @ 0x007a_49c0 => extern "C" fn n_7a49c0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7090(&mut mem(), w, a, s)
    }
    /// `Native_7094(worker, args, stack)`.
    Native_7094 @ 0x007a_72c0 => extern "C" fn n_7a72c0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7094(&mut mem(), w, a, s)
    }
    /// `Native_7095(worker, args, stack)`.
    Native_7095 @ 0x007a_61d0 => extern "C" fn n_7a61d0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7095(&mut mem(), w, a, s)
    }
    /// `Native_7098(worker, args, stack)`.
    Native_7098 @ 0x007a_2dc0 => extern "C" fn n_7a2dc0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7098(&mut mem(), w, a, s)
    }
    /// `Native_BtlSetPadOrientation(worker, args, stack)`.
    Native_BtlSetPadOrientation @ 0x007a_7340 => extern "C" fn n_7a7340(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7099(&mut mem(), w, a, s)
    }
    /// `Native_70a3(worker, args, stack)`.
    Native_70a3 @ 0x007a_3730 => extern "C" fn n_7a3730(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70a3(&mut mem(), w, a, s)
    }
    /// `Native_70a7(worker, args, stack)`.
    Native_70a7 @ 0x007a_54f0 => extern "C" fn n_7a54f0(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_70a7(&mut mem(), w, a, s)
    }
    /// `Native_70b4(worker, args, stack)`.
    Native_70b4 @ 0x007a_7130 => extern "C" fn n_7a7130(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70b4(&mut mem(), w, a, s)
    }
    /// `Native_70b6(worker, args, stack)`.
    Native_70b6 @ 0x007a_52b0 => extern "C" fn n_7a52b0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70b6(&mut mem(), w, a, s)
    }
    /// `Native_70b7(worker, args, stack)`.
    Native_70b7 @ 0x007a_6640 => extern "C" fn n_7a6640(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70b7(&mut mem(), w, a, s)
    }
    /// `Native_70c0(worker, args, stack)`.
    Native_70c0 @ 0x007a_6c70 => extern "C" fn n_7a6c70(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_70c0(&mut mem(), w, a, s)
    }
    /// `Native_70c6(worker, args, stack)`.
    Native_70c6 @ 0x007a_3950 => extern "C" fn n_7a3950(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70c6(&mut mem(), w, a, s)
    }
    /// `Native_70c7(worker, args, stack)`.
    Native_70c7 @ 0x007a_39f0 => extern "C" fn n_7a39f0(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_70c7(&mut mem(), w, a, s)
    }
    /// `Native_70c8(worker, args, stack)`.
    Native_70c8 @ 0x007a_75e0 => extern "C" fn n_7a75e0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70c8(&mut mem(), w, a, s)
    }
    /// `Native_70ca(worker, args, stack)`.
    Native_70ca @ 0x007a_7ab0 => extern "C" fn n_7a7ab0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70ca(&mut mem(), w, a, s)
    }
    /// `Native_70cb(worker, args, stack)`.
    Native_70cb @ 0x007a_7b40 => extern "C" fn n_7a7b40(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70cb(&mut mem(), w, a, s)
    }
    /// `Native_70d3(worker, args, stack)`.
    Native_70d3 @ 0x007a_4a90 => extern "C" fn n_7a4a90(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70d3(&mut mem(), w, a, s)
    }
    /// `Native_70d4(worker, args, stack)`.
    Native_70d4 @ 0x007a_4f20 => extern "C" fn n_7a4f20(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70d4(&mut mem(), w, a, s)
    }
    /// `Native_70e4(worker, args, stack)`.
    Native_70e4 @ 0x007a_2b50 => extern "C" fn n_7a2b50(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70e4(&mut mem(), w, a, s)
    }
    /// `Native_70e5(worker, args, stack)`.
    Native_70e5 @ 0x007a_4c70 => extern "C" fn n_7a4c70(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70e5(&mut mem(), w, a, s)
    }
    /// `Native_70e6(worker, args, stack)`.
    Native_70e6 @ 0x007a_4e00 => extern "C" fn n_7a4e00(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70e6(&mut mem(), w, a, s)
    }
    /// `Native_70f3(worker, args, stack)`.
    Native_70f3 @ 0x007a_3300 => extern "C" fn n_7a3300(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70f3(&mut mem(), w, a, s)
    }
    /// `Native_70f4(worker, args, stack)`.
    Native_70f4 @ 0x007a_2ad0 => extern "C" fn n_7a2ad0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70f4(&mut mem(), w, a, s)
    }
    /// `Native_70f9(worker, args, stack)`.
    Native_70f9 @ 0x007a_5fc0 => extern "C" fn n_7a5fc0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70f9(&mut mem(), w, a, s)
    }
    /// `Native_70fb(worker, args, stack)`.
    Native_70fb @ 0x007a_37e0 => extern "C" fn n_7a37e0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70fb(&mut mem(), w, a, s)
    }
    /// `Native_7100(worker, args, stack)`.
    Native_7100 @ 0x007a_7d20 => extern "C" fn n_7a7d20(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7100(&mut mem(), w, a, s)
    }
    /// `Native_7116(worker, args, stack)`.
    Native_7116 @ 0x007a_6930 => extern "C" fn n_7a6930(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7116(&mut mem(), w, a, s)
    }
    /// `Native_711f(worker, args, stack)`.
    Native_711f @ 0x007a_6200 => extern "C" fn n_7a6200(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_711f(&mut mem(), w, a, s)
    }
    /// `Native_7120(worker, args, stack)`.
    Native_7120 @ 0x007a_65a0 => extern "C" fn n_7a65a0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7120(&mut mem(), w, a, s)
    }
    /// `Native_7121(worker, args, stack)`.
    Native_7121 @ 0x007a_6790 => extern "C" fn n_7a6790(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7121(&mut mem(), w, a, s)
    }
    /// `Native_7124(worker, args, stack)`.
    Native_7124 @ 0x007a_7ee0 => extern "C" fn n_7a7ee0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7124(&mut mem(), w, a, s)
    }
    /// `Native_7127(worker, args, stack)`.
    Native_7127 @ 0x007a_5240 => extern "C" fn n_7a5240(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7127(&mut mem(), w, a, s)
    }
    /// `Native_c00e(worker, args, stack)`.
    Native_c00e @ 0x0087_8700 => extern "C" fn n_878700(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_c00e(&mut mem(), w, a, s)
    }
    /// `Native_c019(worker, args, stack)`.
    Native_c019 @ 0x0087_8550 => extern "C" fn n_878550(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_c019(&mut mem(), w, a, s)
    }
    /// `Native_c035(worker, args, stack)`.
    Native_c035 @ 0x0087_8f50 => extern "C" fn n_878f50(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_c035(&mut mem(), w, a, s)
    }
    /// `Native_c059(worker, args, stack)`.
    Native_c059 @ 0x0087_82a0 => extern "C" fn n_8782a0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_c059(&mut mem(), w, a, s)
    }
    /// `Native_804c(worker, args, stack)`.
    Native_804c @ 0x0091_c090 => extern "C" fn n_91c090(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_804c(&mut mem(), w, a, s)
    }
    /// `Native_804d(worker, args, stack)`.
    Native_804d @ 0x0091_c0d0 => extern "C" fn n_91c0d0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_804d(&mut mem(), w, a, s)
    }
    /// `Native_804e(worker, args, stack)`.
    Native_804e @ 0x0091_c110 => extern "C" fn n_91c110(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_804e(&mut mem(), w, a, s)
    }
    /// `Native_8058(worker, args, stack)`.
    Native_8058 @ 0x0091_c370 => extern "C" fn n_91c370(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_8058(&mut mem(), w, a, s)
    }
    /// `Native_8069(worker, args, stack)`.
    Native_8069 @ 0x0091_af30 => extern "C" fn n_91af30(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_8069(&mut mem(), w, a, s)
    }
    /// `Native_d000(worker, args, stack)`.
    Native_d000 @ 0x00a4_45f0 => extern "C" fn n_a445f0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_d000(&mut mem(), w, a, s)
    }
    /// `Atel_BtlMoveTargetDist(worker, args, stack, gravity)`.
    Atel_BtlMoveTargetDist @ 0x007a_9150 => extern "C" fn h_7a9150(w: u32, a: u32, s: u32, g: u32) -> u32 {
        native_btl::atel_btl_move_target_dist(&mut mem(), w, a, s, g)
    }
    /// `Angle_Wrap_00a7ae20(x)`: a copy of `Angle_Wrap`.
    Angle_Wrap_00a7ae20 @ 0x00a7_ae20 => extern "C" fn h_a7ae20(x: f32) -> f32 {
        ffx_chara::chr::wrap_angle(x)
    }
    /// `Atel_GetVoiceStreamParams(id, &a, &b)`.
    Atel_GetVoiceStreamParams @ 0x0088_5ba0 => extern "C" fn h_885ba0(id: u32, a: u32, b: u32) -> u32 {
        hp::atel_get_voice_stream_params(&mut mem(), id, a, b)
    }
    /// `Atel_SetSpriteAlpha(i, alpha)`.
    Atel_SetSpriteAlpha @ 0x0087_4ad0 => extern "C" fn h_874ad0(i: u32, alpha: u32) -> u32 {
        hp::atel_set_sprite_alpha(&mut mem(), i, alpha)
    }
    /// `Atel_SetSpriteColor(i, r, g, b)`.
    Atel_SetSpriteColor @ 0x0087_49f0 => extern "C" fn h_8749f0(i: u32, r: u32, g: u32, b: u32) -> u32 {
        hp::atel_set_sprite_color(&mut mem(), i, r, g, b)
    }
    /// `Atel_ClearBindMask(n)`.
    Atel_ClearBindMask @ 0x0086_3650 => extern "C" fn h_863650(n: u32) -> u32 {
        hp::atel_clear_bind_mask(&mut mem(), n)
    }
    /// `Atel_NameToLang1(src, dst)`.
    Atel_NameToLang1 @ 0x0087_5220 => extern "C" fn h_875220(src: u32, dst: u32) -> u32 {
        hp::atel_name_to_lang1(&mut mem(), src, dst)
    }
    /// `Atel_NameFromLang1(src, dst)`.
    Atel_NameFromLang1 @ 0x0087_5370 => extern "C" fn h_875370(src: u32, dst: u32) -> u32 {
        hp::atel_name_from_lang1(&mut mem(), src, dst)
    }
    /// `Atel_ResetLineTable(worker)`.
    Atel_ResetLineTable @ 0x0086_d110 => extern "C" fn h_86d110(w: u32) -> u32 {
        hp::atel_reset_line_table(&mut mem(), w)
    }
    /// `Atel_FreeLineSignals(worker)`.
    Atel_FreeLineSignals @ 0x0086_d1c0 => extern "C" fn h_86d1c0(w: u32) -> u32 {
        hp::atel_free_line_signals(&mut mem(), w)
    }
    /// `Atel_SetLineTable(worker, table)`.
    Atel_SetLineTable @ 0x0086_d190 => extern "C" fn h_86d190(w: u32, t: u32) -> u32 {
        hp::atel_set_line_table(&mut mem(), w, t)
    }
    /// `Atel_AddSepLoad(id, flags)`.
    Atel_AddSepLoad @ 0x0086_e250 => extern "C" fn h_86e250(id: u32, flags: u32) -> u32 {
        hp::atel_add_sep_load(&mut mem(), id, flags)
    }
    /// `Atel_StartMoveBlock(worker, mask, mode, param)`.
    Atel_StartMoveBlock @ 0x0087_03e0 => extern "C" fn h_8703e0(w: u32, mask: u32, mode: u32, param: u32) -> u32 {
        hp::atel_start_move_block(&mut mem(), w, mask, mode, param)
    }
    /// `Atel_ShareMoveBlock(worker, level, mask, mode)`.
    Atel_ShareMoveBlock @ 0x0087_0440 => extern "C" fn h_870440(w: u32, level: u32, mask: u32, mode: u32) -> u32 {
        hp::atel_share_move_block(&mut mem(), w, level, mask, mode)
    }
    /// `Atel_StartTurnBlock(worker, mask, mode, param)`.
    Atel_StartTurnBlock @ 0x0087_1030 => extern "C" fn h_871030(w: u32, mask: u32, mode: u32, param: u32) -> u32 {
        hp::atel_start_turn_block(&mut mem(), w, mask, mode, param)
    }
    /// `Atel_ShareTurnBlock(worker, level, mask, mode)`.
    Atel_ShareTurnBlock @ 0x0087_1090 => extern "C" fn h_871090(w: u32, level: u32, mask: u32, mode: u32) -> u32 {
        hp::atel_share_turn_block(&mut mem(), w, level, mask, mode)
    }
    /// `AtelSetBindObject(worker)`: the rotation and translation are the
    /// original's locals.
    AtelSetBindObject @ 0x0086_f4c0 => extern "C" fn h_86f4c0(w: u32) -> u32 {
        let mut frame = super::StackFrame::<8>::new();
        hp::atel_set_bind_object(&mut mem(), w, frame.at())
    }
    /// `AtelSetPos3d(worker, x, y, z, snap)`: the camera rectangle is the
    /// original's local.
    AtelSetPos3d @ 0x0087_0b20 => extern "C" fn h_870b20(w: u32, x: u32, y: u32, z: u32, snap: u32) -> u32 {
        let mut frame = super::StackFrame::<4>::new();
        hp::atel_set_pos_3d(&mut mem(), w, [x, y, z], snap, frame.at())
    }
    /// `Atel_UcRestore()`.
    Atel_UcRestore @ 0x0086_a0e0 => extern "C" fn h_86a0e0() {
        hp::atel_uc_restore(&mut mem());
    }
    /// `Atel_UcResume()`.
    Atel_UcResume @ 0x0086_a130 => extern "C" fn h_86a130() {
        hp::atel_uc_resume(&mut mem());
    }
    /// `Atel_GetWorkerRotY(i)`: `ST0`.
    Atel_GetWorkerRotY @ 0x0086_a6e0 => extern "C" fn h_86a6e0(i: u32) -> f32 {
        hp::atel_get_worker_rot_y(&mut mem(), i)
    }
    /// `Atel_SyncChrDir(worker)`: `Ch_GetDir` through the game.
    Atel_SyncChrDir @ 0x0086_9df0 => extern "C" fn h_869df0(w: u32) {
        hp::atel_sync_chr_dir(&mut mem(), w, |m, c| {
            super::call_cdecl_f64(ffx_bridge::Mem::rebase(m, 0x0082_ab80), &[c])
        });
    }
    /// `AtelBindEff2On(worker, kind)`.
    AtelBindEff2On @ 0x0086_2700 => extern "C" fn h_862700(w: u32, kind: u32) -> u32 {
        hp::atel_bind_eff2_on(&mut mem(), w, kind)
    }
    /// `Atel_QueueSepLoads(header)`.
    Atel_QueueSepLoads @ 0x0086_38c0 => extern "C" fn h_8638c0(b: u32) -> u32 {
        hp::atel_queue_sep_loads(&mut mem(), b)
    }
    /// `AtelGetEventActorLinkWork(i)`.
    AtelGetEventActorLinkWork @ 0x0086_b1b0 => extern "C" fn h_86b1b0(i: u32) -> u32 {
        hp::atel_get_event_actor_link_work(&mut mem(), i)
    }
    /// `AtelSetEventActorLinkWork(i, v)`.
    AtelSetEventActorLinkWork @ 0x0086_fc40 => extern "C" fn h_86fc40(i: u32, v: u32) -> u32 {
        hp::atel_set_event_actor_link_work(&mut mem(), i, v)
    }
    /// `AtelGetEventBindIdFromID(i)`.
    AtelGetEventBindIdFromID @ 0x0086_b370 => extern "C" fn h_86b370(i: u32) -> u32 {
        hp::atel_get_event_bind_id_from_id(&mut mem(), i)
    }
    /// `AtelGetEventCharIDFromID(i)`.
    AtelGetEventCharIDFromID @ 0x0086_b3d0 => extern "C" fn h_86b3d0(i: u32) -> u32 {
        hp::atel_get_event_char_id_from_id(&mut mem(), i)
    }
    /// `AtelGetEventElemCharIDFromID(i, k)`.
    AtelGetEventElemCharIDFromID @ 0x0086_b400 => extern "C" fn h_86b400(i: u32, k: u32) -> u32 {
        hp::atel_get_event_elem_char_id_from_id(&mut mem(), i, k)
    }
    /// `AtelGetEventElemHandleFromID(i, k)`.
    AtelGetEventElemHandleFromID @ 0x0086_b440 => extern "C" fn h_86b440(i: u32, k: u32) -> u32 {
        hp::atel_get_event_elem_handle_from_id(&mut mem(), i, k)
    }
    /// `AtelGetEventElemPosFromID(i, k)`.
    AtelGetEventElemPosFromID @ 0x0086_b480 => extern "C" fn h_86b480(i: u32, k: u32) -> u32 {
        hp::atel_get_event_elem_pos_from_id(&mut mem(), i, k)
    }
    /// `AtelSetEventElemHandleFromID(i, k, v)`.
    AtelSetEventElemHandleFromID @ 0x0086_fde0 => extern "C" fn h_86fde0(i: u32, k: u32, v: u32) -> u32 {
        hp::atel_set_event_elem_handle_from_id(&mut mem(), i, k, v)
    }
    /// `AtelSetEventElemPosFromID(i, k, v)`.
    AtelSetEventElemPosFromID @ 0x0086_fe20 => extern "C" fn h_86fe20(i: u32, k: u32, v: u32) -> u32 {
        hp::atel_set_event_elem_pos_from_id(&mut mem(), i, k, v)
    }
    /// `AtelSetEventElemCharIDFromID(i, k, v)`.
    AtelSetEventElemCharIDFromID @ 0x0086_fda0 => extern "C" fn h_86fda0(i: u32, k: u32, v: u32) -> u32 {
        hp::atel_set_event_elem_char_id_from_id(&mut mem(), i, k, v)
    }
    /// `AtelSetEventCharHandleFromID(i, model)`.
    AtelSetEventCharHandleFromID @ 0x0086_fd60 => extern "C" fn h_86fd60(i: u32, model: u32) -> u32 {
        hp::atel_set_event_char_handle_from_id(&mut mem(), i, model)
    }
    /// `AtelGetEventElemStatus(i, k)`.
    AtelGetEventElemStatus @ 0x0086_b4c0 => extern "C" fn h_86b4c0(i: u32, k: u32) -> u32 {
        hp::atel_get_event_elem_status(&mut mem(), i, k)
    }
    /// `AtelSetEventElemStatus(i, k, on)`.
    AtelSetEventElemStatus @ 0x0086_fe60 => extern "C" fn h_86fe60(i: u32, k: u32, on: u32) -> u32 {
        hp::atel_set_event_elem_status(&mut mem(), i, k, on)
    }
    /// `AtelGetEventActorType(i)`.
    AtelGetEventActorType @ 0x0086_b320 => extern "C" fn h_86b320(i: u32) -> u32 {
        hp::atel_get_event_actor_type(&mut mem(), i)
    }
    /// `AtelGetHideStatusFromID(i)`.
    AtelGetHideStatusFromID @ 0x0086_ba40 => extern "C" fn h_86ba40(i: u32) -> u32 {
        hp::atel_get_hide_status_from_id(&mut mem(), i)
    }
    /// `AtelGetKeepBattlePartyFromID(i)`.
    AtelGetKeepBattlePartyFromID @ 0x0086_bbf0 => extern "C" fn h_86bbf0(i: u32) -> u32 {
        hp::atel_get_keep_battle_party_from_id(&mut mem(), i)
    }
    /// `AtelGetRequestLevel(i)`.
    AtelGetRequestLevel @ 0x0086_c3f0 => extern "C" fn h_86c3f0(i: u32) -> u32 {
        hp::atel_get_request_level(&mut mem(), i)
    }
    /// `AtelGetMyWorkSize(i)`.
    AtelGetMyWorkSize @ 0x0086_c160 => extern "C" fn h_86c160(i: u32) -> u32 {
        hp::atel_get_my_work_size(&mut mem(), i)
    }
    /// `AtelSetMyWorkAdrs(i, at)`.
    AtelSetMyWorkAdrs @ 0x0087_0870 => extern "C" fn h_870870(i: u32, at: u32) -> u32 {
        hp::atel_set_my_work_adrs(&mut mem(), i, at)
    }
    /// `AtelGetElemId(chr, kind)`.
    AtelGetElemId @ 0x0086_b150 => extern "C" fn h_86b150(chr: u32, kind: u32) -> u32 {
        hp::atel_get_elem_id(&mut mem(), chr, kind)
    }
    /// `AtelSetWaterBattleEffect(on)`.
    AtelSetWaterBattleEffect @ 0x0087_15f0 => extern "C" fn h_8715f0(on: u32) -> u32 {
        hp::atel_set_water_battle_effect(&mut mem(), on)
    }
    /// `Atel_SetSaveEventD1(on)`.
    Atel_SetSaveEventD1 @ 0x0086_f060 => extern "C" fn h_86f060(on: u32) -> u32 {
        hp::atel_set_save_event_d1(&mut mem(), on)
    }
    /// `Atel_SetSaveEvent13Bit1(on)`.
    Atel_SetSaveEvent13Bit1 @ 0x0086_f430 => extern "C" fn h_86f430(on: u32) -> u32 {
        hp::atel_set_save_event_13_bit1(&mut mem(), on)
    }
    /// `AtelSetForcePlaceId(id)`.
    AtelSetForcePlaceId @ 0x0087_0140 => extern "C" fn h_870140(id: u32) -> u32 {
        hp::atel_set_force_place_id(&mut mem(), id)
    }
    /// `Atel_SetSaveDicIndexValue(v)`.
    Atel_SetSaveDicIndexValue @ 0x0087_0ed0 => extern "C" fn h_870ed0(v: u32) -> u32 {
        hp::atel_set_save_event_word(&mut mem(), 0xe, v)
    }
    /// `Atel_SetSaveEventE8(v)`.
    Atel_SetSaveEventE8 @ 0x0087_3dd0 => extern "C" fn h_873dd0(v: u32) -> u32 {
        hp::atel_set_save_event_word(&mut mem(), 0xe8, v)
    }
    /// `Atel_SetSaveEventEA(v)`.
    Atel_SetSaveEventEA @ 0x0087_0fa0 => extern "C" fn h_870fa0(v: u32) -> u32 {
        hp::atel_set_save_event_word(&mut mem(), 0xea, v)
    }
    /// `Atel_SetSaveEventE4(v)`.
    Atel_SetSaveEventE4 @ 0x0087_4030 => extern "C" fn h_874030(v: u32) -> u32 {
        hp::atel_set_save_event_e4(&mut mem(), v)
    }
    /// `AtelSetHelpEventFlag(i, on)`.
    AtelSetHelpEventFlag @ 0x0087_01f0 => extern "C" fn h_8701f0(i: u32, on: u32) -> u32 {
        hp::atel_set_help_event_flag(&mut mem(), i, on)
    }
    /// `AtelInitSaveWork()`.
    AtelInitSaveWork @ 0x0086_d4e0 => extern "C" fn h_86d4e0() -> u32 {
        hp::atel_init_save_work(&mut mem())
    }
    /// `AtelMapLoadNoCache(map)`: `AtelMapLoad`'s code.
    AtelMapLoadNoCache @ 0x0086_dd30 => extern "C" fn h_86dd30(map: u32) -> u32 {
        hp::atel_map_load(&mut mem(), map)
    }
    /// `AtelMapLoadBtl(map)`.
    AtelMapLoadBtl @ 0x0086_dcd0 => extern "C" fn h_86dcd0(map: u32) -> u32 {
        hp::atel_map_load_btl(&mut mem(), map)
    }
    /// `Atel_JumpToEntrance(e)`.
    Atel_JumpToEntrance @ 0x0087_4000 => extern "C" fn h_874000(e: u32) -> u32 {
        hp::atel_jump_to_entrance(&mut mem(), e)
    }
    /// `AtelSetDolby(on)`.
    AtelSetDolby @ 0x0086_f9e0 => extern "C" fn h_86f9e0(on: u32) -> u32 {
        hp::atel_set_dolby(&mut mem(), on)
    }
    /// `AtelGetActorStartPos(i, x, y, z)`.
    AtelGetActorStartPos @ 0x0086_a720 => extern "C" fn h_86a720(i: u32, x: u32, y: u32, z: u32) -> u32 {
        hp::atel_get_actor_start_pos(&mut mem(), i, x, y, z)
    }
    /// `AtelGetActorEndPos(i, x, y, z)`.
    AtelGetActorEndPos @ 0x0086_a390 => extern "C" fn h_86a390(i: u32, x: u32, y: u32, z: u32) -> u32 {
        hp::atel_get_actor_end_pos(&mut mem(), i, x, y, z)
    }
    /// `Script_GetWorkerPos(i, x, y, z)`.
    Script_GetWorkerPos @ 0x0086_a2f0 => extern "C" fn h_86a2f0(i: u32, x: u32, y: u32, z: u32) -> u32 {
        hp::script_get_worker_pos(&mut mem(), i, x, y, z)
    }
    /// `Event_SetPlayer(id)`.
    Event_SetPlayer @ 0x0086_f6a0 => extern "C" fn h_86f6a0(id: u32) -> u32 {
        hp::event_set_player(&mut mem(), id)
    }
    /// `AtelIsBind(i)`.
    AtelIsBind @ 0x0086_d950 => extern "C" fn h_86d950(i: u32) -> u32 {
        hp::atel_is_bind(&mut mem(), i)
    }
    /// `AtelGetCharHandleFromBindPC2(chr)`: `AtelGetCharHandleFromBindPC`'s code.
    AtelGetCharHandleFromBindPC2 @ 0x0086_aee0 => extern "C" fn h_86aee0(chr: u32) -> u32 {
        hp::atel_get_char_handle_from_bind_pc(&mut mem(), chr)
    }
    /// `Atel_HasSignalFromLevel(i, level)`.
    Atel_HasSignalFromLevel @ 0x0086_3520 => extern "C" fn h_863520(i: u32, level: u32) -> u32 {
        hp::atel_has_signal_from_level(&mut mem(), i, level)
    }
    /// `Atel_EffectStartPoll(arg)`.
    Atel_EffectStartPoll @ 0x0086_33e0 => extern "C" fn h_8633e0(arg: u32) -> u32 {
        hp::atel_effect_start_poll(&mut mem(), arg)
    }
    /// `Atel_EffectIdlePoll()`.
    Atel_EffectIdlePoll @ 0x0086_33a0 => extern "C" fn h_8633a0() -> u32 {
        hp::atel_effect_idle_poll(&mut mem())
    }
    /// `Atel_EffectEndPoll()`.
    Atel_EffectEndPoll @ 0x0086_3350 => extern "C" fn h_863350() -> u32 {
        hp::atel_effect_end_poll(&mut mem())
    }
    /// `Atel_SetLevelVectorsXZ(worker, a, c)`.
    Atel_SetLevelVectorsXZ @ 0x0086_f750 => extern "C" fn h_86f750(w: u32, a: u32, c: u32) -> u32 {
        hp::atel_set_level_vectors_xz(&mut mem(), w, a, c)
    }
    /// `Atel_GetWorkerBits10_12(i)`.
    Atel_GetWorkerBits10_12 @ 0x0086_a440 => extern "C" fn h_86a440(i: u32) -> u32 {
        hp::atel_get_worker_bits10_12(&mut mem(), i)
    }
    /// `AtelRecoverPreBgPage()`.
    AtelRecoverPreBgPage @ 0x0087_3cc0 => extern "C" fn h_873cc0() {
        hp::atel_recover_pre_bg_page(&mut mem());
    }
    /// `AtelExecReturn2(i, level, reset)`.
    AtelExecReturn2 @ 0x0086_9350 => extern "C" fn h_869350(i: u32, level: u32, reset: u32) {
        hp::atel_exec_return2(&mut mem(), i, level, reset);
    }
    /// `AtelInitSourceCodeDebugWork(i, src)`.
    AtelInitSourceCodeDebugWork @ 0x0086_d670 => extern "C" fn h_86d670(i: u32, src: u32) {
        hp::atel_init_source_code_debug_work(&mut mem(), i, src);
    }
    /// `AtelEventRemoveGuideMap()`.
    AtelEventRemoveGuideMap @ 0x0086_61a0 => extern "C" fn h_8661a0() {
        hp::atel_event_remove_guide_map(&mut mem());
    }
    /// `AtelEventSetupGuideMap()`.
    AtelEventSetupGuideMap @ 0x0086_6490 => extern "C" fn h_866490() -> u32 {
        hp::atel_event_setup_guide_map(&mut mem())
    }
    /// `AtelEventSetUpFoot3d(hdr)`.
    AtelEventSetUpFoot3d @ 0x0086_6210 => extern "C" fn h_866210(hdr: u32) -> u32 {
        hp::atel_event_set_up_foot_3d(&mut mem(), hdr)
    }
    /// `AtelEventLoadMusic(hdr)`.
    AtelEventLoadMusic @ 0x0086_5a60 => extern "C" fn h_865a60(hdr: u32) -> u32 {
        hp::atel_event_load_music(&mut mem(), hdr)
    }
    /// `AtelInitCalcSound()`.
    AtelInitCalcSound @ 0x0087_7420 => extern "C" fn h_877420() -> u32 {
        hp::atel_init_calc_sound(&mut mem())
    }
    /// `AtelGetGuideMapAngEx(i)`: `ST0`.
    AtelGetGuideMapAngEx @ 0x0086_b9a0 => extern "C" fn h_86b9a0(i: u32) -> f32 {
        hp::atel_get_guide_map_ang_ex(&mem(), i)
    }
    /// `AtelInitInternalActorWork(i)`.
    AtelInitInternalActorWork @ 0x0086_d260 => extern "C" fn h_86d260(i: u32) -> u32 {
        hp::atel_init_internal_actor_work(&mut mem(), i)
    }
    /// `AtelSetOnMemoryMovieFileWait(movie, flags)`.
    AtelSetOnMemoryMovieFileWait @ 0x0087_3f70 => extern "C" fn h_873f70(movie: u32, flags: u32) -> u32 {
        hp::atel_set_on_memory_movie_file_wait(&mut mem(), movie, flags)
    }
    /// `AtelRecoverFromMenuCallback()`.
    AtelRecoverFromMenuCallback @ 0x0086_e700 => extern "C" fn h_86e700() -> u32 {
        hp::atel_recover_from_menu_callback(&mut mem())
    }
    /// `AtelBindFp(worker, fp, bone)`.
    AtelBindFp @ 0x0086_2800 => extern "C" fn h_862800(w: u32, fp: u32, bone: u32) {
        hp::atel_bind_fp(&mut mem(), w, fp, bone);
    }
    /// `AtelBindFpOff(worker, fp)`.
    AtelBindFpOff @ 0x0086_28a0 => extern "C" fn h_8628a0(w: u32, fp: u32) {
        hp::atel_bind_fp_off(&mut mem(), w, fp);
    }
    /// `AtelBindFpPos(worker, fp, x, y, z)`: the vector is the original's
    /// local.
    AtelBindFpPos @ 0x0086_2920 => extern "C" fn h_862920(w: u32, fp: u32, x: u32, y: u32, z: u32) {
        let mut frame = super::StackFrame::<4>::new();
        hp::atel_bind_fp_pos(&mut mem(), w, fp, [x, y, z], frame.at());
    }
    /// `AtelRecoverFPFromBattle(worker)`: the vector is the original's
    /// local.
    AtelRecoverFPFromBattle @ 0x0086_e4c0 => extern "C" fn h_86e4c0(w: u32) {
        let mut frame = super::StackFrame::<4>::new();
        hp::atel_recover_fp_from_battle(&mut mem(), w, frame.at());
    }
    /// `Atel_InRadiusXZ(r, worker, x, y, z)`.
    Atel_InRadiusXZ @ 0x0086_34b0 => extern "C" fn h_8634b0(r: u32, w: u32, x: u32, _y: u32, z: u32) -> u32 {
        hp::atel_in_radius_xz(&mut mem(), r, w, x, z)
    }
    /// `Atel_InRadiusXYZ(r, worker, x, y, z)`.
    Atel_InRadiusXYZ @ 0x0086_3440 => extern "C" fn h_863440(r: u32, w: u32, x: u32, y: u32, z: u32) -> u32 {
        hp::atel_in_radius_xyz(&mut mem(), r, w, [x, y, z])
    }
    /// `AtelGetMapIndex(hdr, i)`.
    AtelGetMapIndex @ 0x0086_bd90 => extern "C" fn h_86bd90(hdr: u32, i: u32) -> u32 {
        hp::atel_get_map_index(&mut mem(), hdr, i)
    }
    /// `AtelEventPlayMusic(hdr)`.
    AtelEventPlayMusic @ 0x0086_5ac0 => extern "C" fn h_865ac0(hdr: u32) -> u32 {
        hp::atel_event_play_music(&mut mem(), hdr)
    }
    /// `AtelRequestMapCamera(set)`.
    AtelRequestMapCamera @ 0x0086_ed80 => extern "C" fn h_86ed80(set: u32) -> u32 {
        hp::atel_request_map_camera(&mut mem(), set)
    }
    /// `getEventLocTexListPath(out)`: `fiosUnifyFilename`'s output is the
    /// original's 256-byte local.
    getEventLocTexListPath @ 0x0087_4c50 => extern "C" fn h_874c50(out: u32) -> u32 {
        let mut frame = super::StackFrame::<64>::new();
        hp::get_event_loc_tex_list_path(&mut mem(), out, frame.at())
    }
    /// `AtelStopSoundAll()`.
    AtelStopSoundAll @ 0x0087_1890 => extern "C" fn h_871890() -> u32 {
        hp::atel_stop_sound_all(&mut mem())
    }
    /// `AtelGetHyoukaMenuEnable()`.
    AtelGetHyoukaMenuEnable @ 0x0086_bae0 => extern "C" fn h_86bae0() -> u32 {
        hp::atel_get_hyouka_menu_enable(&mut mem())
    }
    /// `Atel_SetRunInBattle_008630c0()`.
    Atel_SetRunInBattle_008630c0 @ 0x0086_30c0 => extern "C" fn h_8630c0() -> u32 {
        hp::atel_set_run_in_battle(&mut mem())
    }
    /// `Atel_InitFreeSignals_0086d510(worker)`.
    Atel_InitFreeSignals_0086d510 @ 0x0086_d510 => extern "C" fn h_86d510(w: u32) -> u32 {
        hp::atel_init_free_signals(&mut mem(), w)
    }
    /// `Atel_ResetMsgWindowBank_00863c90(bank)`.
    Atel_ResetMsgWindowBank_00863c90 @ 0x0086_3c90 => extern "C" fn h_863c90(bank: u32) -> u32 {
        hp::atel_reset_msg_window_bank(&mut mem(), bank)
    }
    /// `Atel_LoadEventSeps_00866520(_, mode, group)`: the first word is
    /// never read.
    Atel_LoadEventSeps_00866520 @ 0x0086_6520 => extern "C" fn h_866520(_p0: u32, mode: u32, group: u32) -> u32 {
        hp::atel_load_event_seps(&mut mem(), mode, group)
    }
    /// `destroyMoviePicture()`.
    destroyMoviePicture @ 0x0076_ece0 => extern "C" fn h_76ece0() -> u32 {
        hp::destroy_movie_picture(&mut mem())
    }
    /// `Movie_IsBusy_0076ed10()`.
    Movie_IsBusy_0076ed10 @ 0x0076_ed10 => extern "C" fn h_76ed10() -> u32 {
        hp::movie_is_busy(&mut mem())
    }
    /// `TkMenuSaveLoad_PollCard_008b3d20()`.
    TkMenuSaveLoad_PollCard_008b3d20 @ 0x008b_3d20 => extern "C" fn h_8b3d20() -> u32 {
        hp::tk_menu_save_load_poll_card(&mut mem())
    }
    /// `Atel_SetVarSlot_00878140(worker, args, stack)`: `args` is not read.
    Atel_SetVarSlot_00878140 @ 0x0087_8140 => extern "C" fn h_878140(w: u32, _a: u32, s: u32) -> u32 {
        hp::atel_set_var_slot(&mut mem(), w, s)
    }
    /// `Atel_GetEventActorRot_0086b270(i, x, y, z)`.
    Atel_GetEventActorRot_0086b270 @ 0x0086_b270 => extern "C" fn h_86b270(i: u32, x: u32, y: u32, z: u32) -> u32 {
        hp::atel_get_event_actor_rot(&mut mem(), i, x, y, z)
    }
    /// `Atel_TransSepGroup_00871780(hdr, data, sep_only, group, flag)`.
    Atel_TransSepGroup_00871780 @ 0x0087_1780 => extern "C" fn h_871780(hdr: u32, data: u32, sep_only: u32, group: u32, flag: u32) -> u32 {
        hp::atel_trans_sep_group(&mut mem(), hdr, data, sep_only, group, flag)
    }
    /// `AtelGetGuideMapActorInfo(i, out)`: the start and end y go to the
    /// original's argument slot, here a frame word.
    AtelGetGuideMapActorInfo @ 0x0086_b740 => extern "C" fn h_86b740(i: u32, out: u32) -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        hp::atel_get_guide_map_actor_info(&mut mem(), i, out, frame.at())
    }
    /// `AtelGetGideMapActorInfo(i, out)`: a jump to
    /// `AtelGetGuideMapActorInfo` behind an empty frame.
    AtelGetGideMapActorInfo @ 0x0086_b700 => extern "C" fn h_86b700(i: u32, out: u32) -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        hp::atel_get_guide_map_actor_info(&mut mem(), i, out, frame.at())
    }
    /// `AtelSndTransSepSPKDataGroup(hdr, data, group)`.
    AtelSndTransSepSPKDataGroup @ 0x0087_1750 => extern "C" fn h_871750(hdr: u32, data: u32, group: u32) -> u32 {
        hp::atel_snd_trans_sep_spk_data_group(&mut mem(), hdr, data, group)
    }
    /// `AtelEventRemoveSep(mode)`.
    AtelEventRemoveSep @ 0x0086_61d0 => extern "C" fn h_8661d0(mode: u32) -> u32 {
        hp::atel_event_remove_sep(&mut mem(), mode)
    }
    /// `AtelEnterMenuCallback()`.
    AtelEnterMenuCallback @ 0x0086_55e0 => extern "C" fn h_8655e0() -> u32 {
        hp::atel_enter_menu_callback(&mut mem())
    }
    /// `AtelEventSetUpSep(hdr, data, sep_only)`.
    AtelEventSetUpSep @ 0x0086_6430 => extern "C" fn h_866430(hdr: u32, data: u32, sep_only: u32) -> u32 {
        hp::atel_event_set_up_sep(&mut mem(), hdr, data, sep_only)
    }
    /// `Atel_RequestBattleEndEntries_0086e8d0(tag)`.
    Atel_RequestBattleEndEntries_0086e8d0 @ 0x0086_e8d0 => extern "C" fn h_86e8d0(tag: u32) -> u32 {
        hp::atel_request_battle_end_entries(&mut mem(), tag)
    }
    /// `Atel_SetLevelTarget_0086f940(worker, i)`.
    Atel_SetLevelTarget_0086f940 @ 0x0086_f940 => extern "C" fn h_86f940(w: u32, i: u32) -> u32 {
        hp::atel_set_level_target(&mut mem(), w, i)
    }
    /// `Atel_CamGetVars_007ba3c0(worker, args, stack, side)`: `Cam_GetPos`.
    Atel_CamGetVars_007ba3c0 @ 0x007b_a3c0 => extern "C" fn h_7ba3c0(w: u32, _a: u32, s: u32, side: u32) -> u32 {
        hp::atel_cam_get_vars(&mut mem(), 0x007b_d420, w, s, side)
    }
    /// `Atel_CamGetRefVars_007ba460(worker, args, stack, side)`:
    /// `Cam_GetRefPos_007bd4b0`.
    Atel_CamGetRefVars_007ba460 @ 0x007b_a460 => extern "C" fn h_7ba460(w: u32, _a: u32, s: u32, side: u32) -> u32 {
        hp::atel_cam_get_vars(&mut mem(), 0x007b_d4b0, w, s, side)
    }
    /// `Vibrate_UpdateVolume_00875b40()`.
    Vibrate_UpdateVolume_00875b40 @ 0x0087_5b40 => extern "C" fn h_875b40() -> u32 {
        hp::vibrate_update_volume(&mut mem())
    }
    /// `Atel_RequestOwnerTrigger_008764a0(owner, kind)`.
    Atel_RequestOwnerTrigger_008764a0 @ 0x0087_64a0 => extern "C" fn h_8764a0(owner: u32, kind: u32) -> u32 {
        hp::atel_request_owner_trigger(&mut mem(), owner, kind)
    }
    /// `TkMenuSaveLoad_ClearLoadPos_008b3cd0(slot)`.
    TkMenuSaveLoad_ClearLoadPos_008b3cd0 @ 0x008b_3cd0 => extern "C" fn h_8b3cd0(slot: u32) -> u32 {
        hp::tk_menu_save_load_clear_load_pos(&mut mem(), slot)
    }
    /// `Atel_InitMovieState_00873c00()`.
    Atel_InitMovieState_00873c00 @ 0x0087_3c00 => extern "C" fn h_873c00() -> u32 {
        hp::atel_init_movie_state(&mut mem())
    }
    /// `Atel_QueueBattleEndEffect_00861550(kind, v)`.
    Atel_QueueBattleEndEffect_00861550 @ 0x0086_1550 => extern "C" fn h_861550(kind: u32, v: u32) -> u32 {
        hp::atel_queue_battle_end_effect(&mut mem(), kind, v)
    }
    /// `Atel_FadeOutQueuedSeps_00868460()`.
    Atel_FadeOutQueuedSeps_00868460 @ 0x0086_8460 => extern "C" fn h_868460() -> u32 {
        hp::atel_fade_out_queued_seps(&mut mem())
    }
    /// `TkMenuSaveLoad_OpenWindow_008b3c20()`.
    TkMenuSaveLoad_OpenWindow_008b3c20 @ 0x008b_3c20 => extern "C" fn h_8b3c20() -> u32 {
        hp::tk_menu_save_load_open_window(&mut mem())
    }
    /// `TkMenuSaveLoad_OpenChoice_008b3c70(arg)`.
    TkMenuSaveLoad_OpenChoice_008b3c70 @ 0x008b_3c70 => extern "C" fn h_8b3c70(arg: u32) -> u32 {
        hp::tk_menu_save_load_open_choice(&mut mem(), arg)
    }
    /// `Atel_ShowWaterFps_008682a0(on)`.
    Atel_ShowWaterFps_008682a0 @ 0x0086_82a0 => extern "C" fn h_8682a0(on: u32) -> u32 {
        hp::atel_show_water_fps(&mut mem(), on)
    }
    /// `Atel_SetSaveEventBit_008700e0(n)`.
    Atel_SetSaveEventBit_008700e0 @ 0x0087_00e0 => extern "C" fn h_8700e0(n: u32) -> u32 {
        hp::atel_set_save_event_bit(&mut mem(), n)
    }
    /// `AtelExecSoftWareReset()`.
    AtelExecSoftWareReset @ 0x0087_38d0 => extern "C" fn h_8738d0() -> u32 {
        hp::atel_exec_soft_ware_reset(&mut mem())
    }
    /// `AtelClearShape()`: the texture list path and its unify buffer in
    /// one 512-byte frame.
    AtelClearShape @ 0x0087_42b0 => extern "C" fn h_8742b0() -> u32 {
        let mut frame = super::StackFrame::<128>::new();
        hp::atel_clear_shape(&mut mem(), frame.at())
    }
    /// `Atel_TranslateBGSeps_00861020(volume, frames, all)`.
    Atel_TranslateBGSeps_00861020 @ 0x0086_1020 => extern "C" fn h_861020(volume: u32, frames: u32, all: u32) -> u32 {
        hp::atel_translate_bg_seps(&mut mem(), volume, frames, all)
    }
    /// `AtelEventKickBGSep(header, fade)`.
    AtelEventKickBGSep @ 0x0086_58d0 => extern "C" fn h_8658d0(hdr: u32, fade: u32) {
        hp::atel_event_kick_bg_sep(&mut mem(), hdr, fade);
    }
    /// `Atel_SetWorkerPosXZ_00870970(worker, x f32, z f32, keep)`.
    Atel_SetWorkerPosXZ_00870970 @ 0x0087_0970 => extern "C" fn h_870970(w: u32, x: u32, z: u32, keep: u32) {
        hp::atel_set_worker_pos_xz(&mut mem(), w, x, z, keep);
    }
    /// `Atel_CtrlExec_00867710(ctrl)`: the caller ignores `EAX`.
    Atel_CtrlExec_00867710 @ 0x0086_7710 => extern "C" fn h_867710(c: u32) {
        hp::atel_ctrl_exec(&mut mem(), c);
    }
    /// `AtelExecCtrlEvent(ctrl)`.
    AtelExecCtrlEvent @ 0x0086_7740 => extern "C" fn h_867740(c: u32) -> u32 {
        hp::atel_exec_ctrl_event(&mut mem(), c)
    }
    /// `Atel_CameraCallback_00861800(word)`.
    Atel_CameraCallback_00861800 @ 0x0086_1800 => extern "C" fn h_861800(word: u32) -> u32 {
        hp::atel_camera_callback(&mut mem(), word)
    }
    /// `Atel_EffectDoneCallback_00861f20(word)`.
    Atel_EffectDoneCallback_00861f20 @ 0x0086_1f20 => extern "C" fn h_861f20(word: u32) -> u32 {
        hp::atel_effect_done_callback(&mut mem(), word)
    }
    /// `Atel_ChrRegCallback_00861850(state)`.
    Atel_ChrRegCallback_00861850 @ 0x0086_1850 => extern "C" fn h_861850(state: u32) -> u32 {
        hp::atel_chr_reg_callback(&mut mem(), state)
    }
    /// `Atel_EffectCallback_00861f90(state)`.
    Atel_EffectCallback_00861f90 @ 0x0086_1f90 => extern "C" fn h_861f90(state: u32) -> u32 {
        hp::atel_effect_callback(&mut mem(), state)
    }
    /// `AtelFlushAlbhedDic()`: its caller (Native 0x0234) ignores `EAX`.
    AtelFlushAlbhedDic @ 0x0086_a040 => extern "C" fn h_86a040() {
        hp::atel_flush_albhed_dic(&mut mem());
    }
    /// `AtelExecCtrlEventInit_008677a0(ctrl, battle)`.
    AtelExecCtrlEventInit_008677a0 @ 0x0086_77a0 => extern "C" fn h_8677a0(c: u32, battle: u32) {
        hp::atel_exec_ctrl_event_init(&mut mem(), c, battle);
    }
    /// `AtelEventSetUpForBattleMusic()`.
    AtelEventSetUpForBattleMusic @ 0x0086_6240 => extern "C" fn h_866240() {
        hp::atel_event_set_up_for_battle_music(&mut mem());
    }
    /// `AtelEventRegistFootInfo(header)`.
    AtelEventRegistFootInfo @ 0x0086_5b80 => extern "C" fn h_865b80(hdr: u32) -> u32 {
        hp::atel_event_regist_foot_info(&mut mem(), hdr)
    }
    /// `AtelAllClearCallbackQue()`.
    AtelAllClearCallbackQue @ 0x008b_3cf0 => extern "C" fn h_8b3cf0() -> u32 {
        hp::atel_all_clear_callback_que(&mut mem())
    }
    /// `AtelExecVibrate(update)`: the two volumes in a 2-word frame.
    AtelExecVibrate @ 0x0087_39a0 => extern "C" fn h_8739a0(update: u32) -> u32 {
        let mut frame = super::StackFrame::<2>::new();
        hp::atel_exec_vibrate(&mut mem(), update, frame.at())
    }
    /// `Atel_SetChrAltParts_008720e0(chr, on)`.
    Atel_SetChrAltParts_008720e0 @ 0x0087_20e0 => extern "C" fn h_8720e0(chr: u32, on: u32) {
        hp::atel_set_chr_alt_parts(&mut mem(), chr, on);
    }
    /// `Atel_ApplyChrDir_00870360(worker)`.
    Atel_ApplyChrDir_00870360 @ 0x0087_0360 => extern "C" fn h_870360(w: u32) {
        hp::atel_apply_chr_dir(&mut mem(), w);
    }
    /// `Atel_SetSpriteScale_00874a80(i, x f32, y f32)`.
    Atel_SetSpriteScale_00874a80 @ 0x0087_4a80 => extern "C" fn h_874a80(i: u32, x: u32, y: u32) {
        hp::atel_set_sprite_scale(&mut mem(), i, x, y);
    }
    /// `Atel_SyncChrRot_00869f90(worker)`: `Ch_GetRot` and
    /// `Math_WrapAngle` through the game.
    Atel_SyncChrRot_00869f90 @ 0x0086_9f90 => extern "C" fn h_869f90(w: u32) {
        hp::atel_sync_chr_rot(&mut mem(), w, |m, f, args| {
            super::call_cdecl_f64(ffx_bridge::Mem::rebase(m, f), args)
        });
    }
    /// `Atel_SaveLights_0086db20()`.
    Atel_SaveLights_0086db20 @ 0x0086_db20 => extern "C" fn h_86db20() -> u32 {
        hp::atel_save_lights(&mut mem())
    }
    /// `Atel_RestoreLights_0086e770()`.
    Atel_RestoreLights_0086e770 @ 0x0086_e770 => extern "C" fn h_86e770() -> u32 {
        hp::atel_restore_lights(&mut mem())
    }
    /// `Atel_ResetTextLanguage_0086ece0()`.
    Atel_ResetTextLanguage_0086ece0 @ 0x0086_ece0 => extern "C" fn h_86ece0() {
        hp::atel_reset_text_language(&mut mem());
    }
    /// `Atel_StepSpriteAnim_00874bf0(sprite)`.
    Atel_StepSpriteAnim_00874bf0 @ 0x0087_4bf0 => extern "C" fn h_874bf0(s: u32) {
        hp::atel_step_sprite_anim(&mut mem(), s);
    }
    /// `Atel_LoadEventTexList_00874b70(on, x, y)`: the path and its unify
    /// buffer in one 512-byte frame.
    Atel_LoadEventTexList_00874b70 @ 0x0087_4b70 => extern "C" fn h_874b70(on: u32, x: u32, y: u32) {
        let mut frame = super::StackFrame::<128>::new();
        hp::atel_load_event_tex_list(&mut mem(), on, x, y, frame.at());
    }
    /// `AtelBGSepFade(volume, frames)`.
    AtelBGSepFade @ 0x0086_1000 => extern "C" fn h_861000(volume: u32, frames: u32) -> u32 {
        hp::atel_bg_sep_fade(&mut mem(), volume, frames)
    }
    /// `Atel_StepSprites_008743e0()`.
    Atel_StepSprites_008743e0 @ 0x0087_43e0 => extern "C" fn h_8743e0() {
        hp::atel_step_sprites(&mut mem());
    }
    /// `AtelExecDrawShape()`: the render record and matrix in a frame.
    AtelExecDrawShape @ 0x0087_4420 => extern "C" fn h_874420() {
        let mut frame = super::StackFrame::<24>::new();
        hp::atel_exec_draw_shape(&mut mem(), frame.at());
    }
    /// `AtelDrawNoTrans()`: water status and event sprites, one frame.
    AtelDrawNoTrans @ 0x0087_2c90 => extern "C" fn h_872c90() {
        let mut frame = super::StackFrame::<24>::new();
        hp::atel_draw_no_trans(&mut mem(), frame.at());
    }
    /// `AtelDrawSemiTrans()`.
    AtelDrawSemiTrans @ 0x0087_2ca0 => extern "C" fn h_872ca0() {
        hp::atel_draw_semi_trans(&mut mem());
    }
    /// `Atel_SetPartyMask_008623b0()`: the three members in a 3-word frame.
    Atel_SetPartyMask_008623b0 @ 0x0086_23b0 => extern "C" fn h_8623b0() {
        let mut frame = super::StackFrame::<3>::new();
        hp::atel_set_party_mask(&mut mem(), frame.at());
    }
    /// `Atel_RegistVibrateSignal_00876590(from, i, k)`.
    Atel_RegistVibrateSignal_00876590 @ 0x0087_6590 => extern "C" fn h_876590(from: u32, i: u32, k: u32) -> u32 {
        hp::atel_regist_vibrate_signal(&mut mem(), from, i, k)
    }
    /// `Native_DrawBarSegment_00855a30(pos, seg)`.
    Native_DrawBarSegment_00855a30 @ 0x0085_5a30 => extern "C" fn h_855a30(pos: u32, seg: u32) -> u32 {
        hp::native_draw_bar_segment(&mut mem(), pos, seg)
    }
    /// `Atel_BattleEndMapStart_008615b0(map)`.
    Atel_BattleEndMapStart_008615b0 @ 0x0086_15b0 => extern "C" fn h_8615b0(map: u32) -> u32 {
        hp::atel_battle_end_map_start(&mut mem(), map)
    }
    /// `Atel_SetMovieTexture_00873eb0(id, flags)`.
    Atel_SetMovieTexture_00873eb0 @ 0x0087_3eb0 => extern "C" fn h_873eb0(id: u32, flags: u32) -> u32 {
        hp::atel_set_movie_texture(&mut mem(), id, flags)
    }
    /// `Atel_RegistVibrateSignalWith_008766d0(from, i, k, curve)`.
    Atel_RegistVibrateSignalWith_008766d0 @ 0x0087_66d0 => extern "C" fn h_8766d0(from: u32, i: u32, k: u32, curve: u32) -> u32 {
        hp::atel_regist_vibrate_signal_with(&mut mem(), from, i, k, curve)
    }
    /// `AtelCalcSound(req, play)`: the request's pan in `EAX`.
    AtelCalcSound @ 0x0087_6930 => extern "C" fn h_876930(req: u32, play: u32) -> u32 {
        hp::atel_calc_sound(&mut mem(), req, play)
    }
    /// `AtelExecSound()`: its one caller (`0x00872c6d`) drops `EAX`.
    AtelExecSound @ 0x0087_7380 => extern "C" fn h_877380() {
        hp::atel_exec_sound(&mut mem());
    }
    /// `Atel_EventSoundStep_008682f0()`: its one caller drops `EAX`.
    Atel_EventSoundStep_008682f0 @ 0x0086_82f0 => extern "C" fn h_8682f0() {
        hp::atel_event_sound_step(&mut mem());
    }
    /// `Atel_BattleEndMovieStep_00862000(state)`: a step callback; done
    /// (1) or not (0) in `EAX`.
    Atel_BattleEndMovieStep_00862000 @ 0x0086_2000 => extern "C" fn h_862000(state: u32) -> u32 {
        hp::atel_battle_end_movie_step(&mut mem(), state)
    }
    /// `Atel_BattleEndMusicStep_00862120(state)`: a step callback.
    Atel_BattleEndMusicStep_00862120 @ 0x0086_2120 => extern "C" fn h_862120(state: u32) -> u32 {
        hp::atel_battle_end_music_step(&mut mem(), state)
    }
    /// `Atel_BattleEndSepStep_00862220(state)`: a step callback.
    Atel_BattleEndSepStep_00862220 @ 0x0086_2220 => extern "C" fn h_862220(state: u32) -> u32 {
        hp::atel_battle_end_sep_step(&mut mem(), state)
    }
    /// `Atel_WorkerPlaceChr_00866800(worker, level)`: the ctrl work's
    /// `+0x58` callback (its dispatcher `0x008642f2` drops `EAX`); the
    /// `y - 0.1` the original rounds in its own argument slot goes through
    /// a one-word frame (the caller pops its slots unread).
    Atel_WorkerPlaceChr_00866800 @ 0x0086_6800 => extern "C" fn h_866800(w: u32, level: u32) {
        let mut slot = super::StackFrame::<1>::new();
        hp::atel_worker_place_chr(&mut mem(), w, level, slot.at());
    }
    /// `Atel_BattleMusicStep_00862470(state)`: a step callback; done (1)
    /// or not (0) in `EAX`.
    Atel_BattleMusicStep_00862470 @ 0x0086_2470 => extern "C" fn h_862470(state: u32) -> u32 {
        hp::atel_battle_music_step(&mut mem(), state)
    }
    /// `AtelEventSetUp(map)`: the debug source table (`ebp - 0x68`, 25
    /// words) in a frame of its own; `TkInitScene` drops `EAX`.
    AtelEventSetUp @ 0x0087_2ef0 => extern "C" fn h_872ef0(map: u32) {
        let mut work = super::StackFrame::<{ (hp::EVENT_SET_UP_WORK / 4) as usize }>::new();
        hp::atel_event_set_up(&mut mem(), map, work.at(), |_, f, a| super::call_cdecl(f, a));
    }
    /// `AtelSetSndCameraPos()`: the matrix in a 16-word frame.
    AtelSetSndCameraPos @ 0x0087_7470 => extern "C" fn h_877470() {
        let mut frame = super::StackFrame::<16>::new();
        hp::atel_set_snd_camera_pos(&mut mem(), frame.at());
    }
    /// `Atel_TurnToPoint_0086f7f0(worker, x, y, z)`: yaw and pitch in a
    /// 2-word frame, `Math_YawPitchTo`'s eight words through `call_cdecl`.
    Atel_TurnToPoint_0086f7f0 @ 0x0086_f7f0 => extern "C" fn h_86f7f0(w: u32, x: u32, y: u32, z: u32) {
        let mut frame = super::StackFrame::<2>::new();
        hp::atel_turn_to_point(&mut mem(), w, [x, y, z], frame.at(), |m, args| {
            super::call_cdecl(ffx_bridge::Mem::rebase(m, 0x0088_b4d0), args)
        });
    }
    /// `Atel_ShowSprite_008748a0(i, tex)`: the path and its unify buffer in
    /// one 512-byte frame.
    Atel_ShowSprite_008748a0 @ 0x0087_48a0 => extern "C" fn h_8748a0(i: u32, tex: u32) {
        let mut frame = super::StackFrame::<128>::new();
        hp::atel_show_sprite(&mut mem(), i, tex, frame.at());
    }
    /// `AtelUpdateWaterStatus()`: the camera position in a 4-word frame.
    AtelUpdateWaterStatus @ 0x0087_1e50 => extern "C" fn h_871e50() {
        let mut frame = super::StackFrame::<4>::new();
        hp::atel_update_water_status(&mut mem(), frame.at());
    }
    /// `AtelExec_Internal_871d10()`.
    AtelExec_Internal_871d10 @ 0x0087_1d70 => extern "C" fn h_871d70() {
        hp::atel_exec_internal_871d10(&mut mem());
    }
    /// `Atel_SetEventPathLang_00879430(to)`.
    Atel_SetEventPathLang_00879430 @ 0x0087_9430 => extern "C" fn h_879430(to: u32) {
        hp::atel_set_event_path_lang(&mut mem(), to);
    }
    /// `Atel_AddLoveParam_00865650(i, skip)`.
    Atel_AddLoveParam_00865650 @ 0x0086_5650 => extern "C" fn h_865650(i: u32, skip: u32) {
        hp::atel_add_love_param(&mut mem(), i, skip);
    }
    /// `Atel_FindVarSlot_0087bc90(rec)`.
    Atel_FindVarSlot_0087bc90 @ 0x0087_bc90 => extern "C" fn h_87bc90(rec: u32) {
        hp::atel_find_var_slot(&mut mem(), rec);
    }
    /// `Atel_TurnToWorker_0086f8a0(worker, target)`: yaw and pitch in a
    /// 2-word frame, `Math_YawPitchTo` through `call_cdecl`.
    Atel_TurnToWorker_0086f8a0 @ 0x0086_f8a0 => extern "C" fn h_86f8a0(w: u32, target: u32) {
        let mut frame = super::StackFrame::<2>::new();
        hp::atel_turn_to_worker(&mut mem(), w, target, frame.at(), |m, args| {
            super::call_cdecl(ffx_bridge::Mem::rebase(m, 0x0088_b4d0), args)
        });
    }
    /// `Atel_SyncTurnEnd_00870dc0(worker, level)`.
    Atel_SyncTurnEnd_00870dc0 @ 0x0087_0dc0 => extern "C" fn h_870dc0(w: u32, lvl: u32) {
        hp::atel_sync_turn_end(&mut mem(), w, lvl);
    }
    /// `Atel_StreamSeek_00885bf0(start, count, sector, id, pos)`: the name
    /// in a 4-word frame.
    Atel_StreamSeek_00885bf0 @ 0x0088_5bf0 => extern "C" fn h_885bf0(start: u32, count: u32, sector: u32, id: u32, pos: u32) -> u32 {
        let mut frame = super::StackFrame::<4>::new();
        hp::atel_stream_seek(&mut mem(), start, count, sector, id, pos, frame.at())
    }
    /// `AtelJumpGameOver()`.
    AtelJumpGameOver @ 0x0086_da10 => extern "C" fn h_86da10() {
        hp::atel_jump_game_over(&mut mem());
    }
    /// `Atel_KickBGSepsOnFlag_00865780(header, on)`.
    Atel_KickBGSepsOnFlag_00865780 @ 0x0086_5780 => extern "C" fn h_865780(hdr: u32, on: u32) {
        hp::atel_kick_bg_seps_on_flag(&mut mem(), hdr, on);
    }
    /// `Native_GetChrOptPos_00a7a980(worker, _, stack, mode)`: the position
    /// in a 4-word frame.
    Native_GetChrOptPos_00a7a980 @ 0x00a7_a980 => extern "C" fn h_a7a980(w: u32, _unused: u32, s: u32, mode: u32) -> u32 {
        let mut frame = super::StackFrame::<4>::new();
        hp::native_get_chr_opt_pos(&mut mem(), w, s, mode, frame.at())
    }
    /// `Atel_ApplyPolarMove_008629c0(worker)`: the step and the camera
    /// vector in an 8-word frame.
    Atel_ApplyPolarMove_008629c0 @ 0x0086_29c0 => extern "C" fn h_8629c0(w: u32) {
        let mut frame = super::StackFrame::<8>::new();
        hp::atel_apply_polar_move(&mut mem(), w, frame.at());
    }
    /// `Atel_SetEventWaterFlag_00865740(on)`.
    Atel_SetEventWaterFlag_00865740 @ 0x0086_5740 => extern "C" fn h_865740(on: u32) {
        hp::atel_set_event_water_flag(&mut mem(), on);
    }
    /// `AtelUpdateWindowStatus()`.
    AtelUpdateWindowStatus @ 0x0087_1f30 => extern "C" fn h_871f30() -> u32 {
        hp::atel_update_window_status(&mut mem())
    }
    /// `AtelUpdateBattleStatus()`: `Math_LengthSq3` and `CIsqrt` through
    /// the game.
    AtelUpdateBattleStatus @ 0x0087_1bc0 => extern "C" fn h_871bc0() {
        hp::atel_update_battle_status(&mut mem(), |m, f, args| {
            super::call_cdecl_f64(ffx_bridge::Mem::rebase(m, f), args)
        });
    }
    /// `Atel_InitWorkerLevels_0086d300(worker, a, b, c)`.
    Atel_InitWorkerLevels_0086d300 @ 0x0086_d300 => extern "C" fn h_86d300(w: u32, a: u32, b: u32, c: u32) {
        hp::atel_init_worker_levels(&mut mem(), w, a, b, c);
    }
    /// `Atel_CheckLineCross_008680a0(worker, _, target, rec)`: the four
    /// points in a 16-word frame, `Math_CrossHeightGap` through the game.
    Atel_CheckLineCross_008680a0 @ 0x0086_80a0 => extern "C" fn h_8680a0(w: u32, _unused: u32, target: u32, rec: u32) {
        let mut frame = super::StackFrame::<16>::new();
        hp::atel_check_line_cross(&mut mem(), w, target, rec, frame.at(), |m, f, args| {
            super::call_cdecl_f64(ffx_bridge::Mem::rebase(m, f), args)
        });
    }

    // script round 26
    /// `AtelBattleEndCallback(mode, v, map, end)`.
    AtelBattleEndCallback @ 0x0086_1160 => extern "C" fn h_861160(mode: u32, v: u32, map: u32, end: u32) -> u32 {
        hp::atel_battle_end_callback(&mut mem(), mode, v, map, end)
    }
    /// `Atel_RunCtrlWorkers_00867950(event, input, clear)`.
    Atel_RunCtrlWorkers_00867950 @ 0x0086_7950 => extern "C" fn h_867950(event: u32, input: u32, clear: u32) -> u32 {
        hp::atel_run_ctrl_workers(&mut mem(), event, input, clear)
    }
    /// `AtelUpdateFilePath()`.
    AtelUpdateFilePath @ 0x0087_9f00 => extern "C" fn h_879f00() -> u32 {
        hp::atel_update_file_path(&mut mem())
    }
    /// `Atel_CheckPause_00873620()`.
    Atel_CheckPause_00873620 @ 0x0087_3620 => extern "C" fn h_873620() -> u32 {
        hp::atel_check_pause(&mut mem())
    }
    /// `AtelInitTotal()`.
    AtelInitTotal @ 0x0086_d6d0 => extern "C" fn h_86d6d0() -> u32 {
        hp::atel_init_total(&mut mem())
    }
    /// `Atel_MoveStep_00870540(worker, level)`: `_CIsin`/`_CIcos` through
    /// the game's import thunks.
    Atel_MoveStep_00870540 @ 0x0087_0540 => extern "C" fn h_870540(w: u32, level: u32) {
        hp::atel_move_step(&mut mem(), w, level);
    }
    /// `AtelBindElemOff(worker, i)`.
    AtelBindElemOff @ 0x0086_2780 => extern "C" fn h_862780(w: u32, i: u32) -> u32 {
        hp::atel_bind_elem_off(&mut mem(), w, i)
    }
    /// `Atel_CalcSpline_00862b20(out, curve, t)`.
    Atel_CalcSpline_00862b20 @ 0x0086_2b20 => extern "C" fn h_862b20(out: u32, curve: u32, t: u32) -> u32 {
        hp::atel_calc_spline(&mut mem(), out, curve, t)
    }
    /// `Atel_CalcSpline3_00862b70(out, curve, t)`.
    Atel_CalcSpline3_00862b70 @ 0x0086_2b70 => extern "C" fn h_862b70(out: u32, curve: u32, t: u32) -> u32 {
        hp::atel_calc_spline3(&mut mem(), out, curve, t)
    }
    /// `Atel_ExecCtrlWorks_008688f0(from, to)`.
    Atel_ExecCtrlWorks_008688f0 @ 0x0086_88f0 => extern "C" fn h_8688f0(from: u32, to: u32) -> u32 {
        hp::atel_exec_ctrl_works(&mut mem(), from, to)
    }
    /// `Atel_CheckLineCrosses_00868020(worker, x)`.
    Atel_CheckLineCrosses_00868020 @ 0x0086_8020 => extern "C" fn h_868020(w: u32, x: u32) -> u32 {
        hp::atel_check_line_crosses(&mut mem(), w, x)
    }
    /// `Atel_WorkerFrame_008666e0(w, level)`: ctrl work `+0x5c`; its one
    /// caller (`0x00865208`) drops `EAX`.
    Atel_WorkerFrame_008666e0 @ 0x0086_66e0 => extern "C" fn h_8666e0(w: u32, level: u32) {
        hp::atel_worker_frame(&mut mem(), w, level);
    }
    /// `Atel_BattleEndMapStep_00861650(state)`: 1 when done.
    Atel_BattleEndMapStep_00861650 @ 0x0086_1650 => extern "C" fn h_861650(state: u32) -> u32 {
        hp::atel_battle_end_map_step(&mut mem(), state)
    }
    /// `Atel_PlayTimeUpdate_0086ca20()`: a frame callback.
    Atel_PlayTimeUpdate_0086ca20 @ 0x0086_ca20 => extern "C" fn h_86ca20() {
        hp::atel_play_time_update(&mut mem(), |_, f, args| super::call_cdecl_f64(f, args));
    }
    /// `AtelEventGetFilePath2(dirs, sub, alt, name, ext)`: the three
    /// 256-byte locals in a frame.
    AtelEventGetFilePath2 @ 0x0087_95b0 => extern "C" fn h_8795b0(dirs: u32, sub: u32, alt: u32, name: u32, ext: u32) -> u32 {
        let mut frame = super::StackFrame::<0xc0>::new();
        hp::atel_event_get_file_path2(&mut mem(), [dirs, sub, alt, name, ext], frame.at())
    }
    /// `spline_get_near_point_init(spline, x, y, z, cubic)`: the locals in
    /// a 12-word frame, `spline_get_near_point` through the game.
    spline_get_near_point_init @ 0x00a4_4420 => extern "C" fn h_a44420(spline: u32, x: u32, y: u32, z: u32, cubic: u32) {
        let mut frame = super::StackFrame::<12>::new();
        hp::spline_get_near_point_init(&mut mem(), spline, [x, y, z], cubic, frame.at(), |_, f, args| {
            super::call_cdecl_f64(f, args)
        });
    }
    /// `AtelExec()`.
    AtelExec @ 0x0086_66d0 => extern "C" fn h_8666d0() -> u32 {
        hp::atel_exec(&mut mem())
    }
    /// `AtelMenuExec()`.
    AtelMenuExec @ 0x0086_dd60 => extern "C" fn h_86dd60() -> u32 {
        hp::atel_menu_exec(&mut mem())
    }
    /// `AtelLoadSourceData(slot, arg)`.
    AtelLoadSourceData @ 0x0087_9a70 => extern "C" fn h_879a70(slot: u32, arg: u32) -> u32 {
        hp::atel_load_source_data(&mut mem(), slot, arg)
    }
    /// `AtelInitBasicWork(setup, clear)`.
    AtelInitBasicWork @ 0x0086_cb00 => extern "C" fn h_86cb00(setup: u32, clear: u32) {
        hp::atel_init_basic_work(&mut mem(), setup, clear);
    }
    /// `AtelEventRemove()`: `EAX` the tail-called `Sg_EventjumpDispose`'s.
    AtelEventRemove @ 0x0087_2d20 => extern "C" fn h_872d20() -> u32 {
        hp::atel_event_remove(&mut mem())
    }
    /// `Atel_ReturnActorFromBattle_00862d70(w, check)`.
    Atel_ReturnActorFromBattle_00862d70 @ 0x0086_2d70 => extern "C" fn h_862d70(w: u32, check: u32) {
        hp::atel_return_actor_from_battle(&mut mem(), w, check);
    }
    /// `AtelActorRemove()`.
    AtelActorRemove @ 0x0087_5430 => extern "C" fn h_875430() -> u32 {
        hp::atel_actor_remove(&mut mem())
    }
    /// `Atel_RestoreActorModel_0086e5b0(w, keep_motion, party)`.
    Atel_RestoreActorModel_0086e5b0 @ 0x0086_e5b0 => extern "C" fn h_86e5b0(w: u32, keep_motion: u32, party: u32) {
        hp::atel_restore_actor_model(&mut mem(), w, keep_motion, party);
    }
    /// `AtelEventRemoveForBattle()`.
    AtelEventRemoveForBattle @ 0x0086_5dc0 => extern "C" fn h_865dc0() -> u32 {
        hp::atel_event_remove_for_battle(&mut mem())
    }
    /// `Atel_MoveUpdate_00868990(w, level)`: the turn outputs and the spline
    /// point in a frame, the float maths called through the game.
    Atel_MoveUpdate_00868990 @ 0x0086_8990 => extern "C" fn h_868990(w: u32, level: u32) {
        let mut frame = super::StackFrame::<8>::new();
        hp::atel_move_update(&mut mem(), w, level, frame.at(), |_, f, args| super::call_cdecl_f64(f, args));
    }
    /// `Atel_TurnUpdate_00869510(w, level)`: the aim outputs in a frame, the
    /// float maths called through the game.
    Atel_TurnUpdate_00869510 @ 0x0086_9510 => extern "C" fn h_869510(w: u32, level: u32) {
        let mut frame = super::StackFrame::<4>::new();
        hp::atel_turn_update(&mut mem(), w, level, frame.at(), |_, f, args| super::call_cdecl_f64(f, args));
    }
    /// `Atel_CheckTouchTalk_00866980(worker, _, player)`: `Math_LengthSq3`
    /// through the game.
    Atel_CheckTouchTalk_00866980 @ 0x0086_6980 => extern "C" fn h_866980(w: u32, _unused: u32, player: u32) -> u32 {
        hp::atel_check_touch_talk(&mut mem(), w, player, |_, f, args| super::call_cdecl_f64(f, args))
    }
    /// `AtelExecMenuCtrl()`.
    AtelExecMenuCtrl @ 0x0087_3600 => extern "C" fn h_873600() -> u32 {
        hp::atel_exec_menu_ctrl(&mut mem())
    }
    /// `AtelCommonSetUp(ctrl, a, block, x34, x8, debug)`: the setup record
    /// in an eight-word frame.
    AtelCommonSetUp @ 0x0087_59b0 => extern "C" fn h_8759b0(ctrl: u32, a: u32, block: u32, x34: u32, x8: u32, debug: u32) -> u32 {
        let mut frame = super::StackFrame::<8>::new();
        hp::atel_common_set_up(&mut mem(), [ctrl, a, block, x34, x8, debug], frame.at())
    }
    /// `AtelActorSetUp2(a, block, x34, x38, x8, debug, d)`: the setup
    /// record in an eight-word frame; `EAX` `AtelInitActor`'s.
    AtelActorSetUp2 @ 0x0087_5510 => extern "C" fn h_875510(a: u32, block: u32, x34: u32, x38: u32, x8: u32, debug: u32, d: u32) -> u32 {
        let mut frame = super::StackFrame::<8>::new();
        hp::atel_actor_set_up2(&mut mem(), [a, block, x34, x38, x8, debug, d], frame.at())
    }
    /// `Atel_SyncWorkerPos_00869e40(worker)`: its caller ignores `EAX`.
    Atel_SyncWorkerPos_00869e40 @ 0x0086_9e40 => extern "C" fn h_869e40(w: u32) {
        hp::atel_sync_worker_pos(&mut mem(), w);
    }
    /// `AtelPopMember()`: the old push flag.
    AtelPopMember @ 0x0086_ddb0 => extern "C" fn h_86ddb0() -> u32 {
        hp::atel_pop_member(&mut mem())
    }
    /// `AtelPushMember()`: the party words in a frame; the old push flag.
    AtelPushMember @ 0x0086_e310 => extern "C" fn h_86e310() -> u32 {
        let mut frame = super::StackFrame::<3>::new();
        hp::atel_push_member(&mut mem(), frame.at())
    }
    /// `AtelBattleSetUp(blocks, x8s, n, x30, work, a, ctrl, callback)`: the
    /// setup record in an eight-word frame.
    AtelBattleSetUp @ 0x0087_56d0 => extern "C" fn h_8756d0(blocks: u32, x8s: u32, n: u32, x30: u32, work: u32, a: u32, ctrl: u32, callback: u32) -> u32 {
        let mut frame = super::StackFrame::<8>::new();
        hp::atel_battle_set_up(&mut mem(), [blocks, x8s, n, x30, work, a, ctrl, callback], frame.at())
    }
    /// `Atel_CheckBoxTouch_00867c10(worker, _)`: `Math_LengthSq3` through
    /// the game.
    Atel_CheckBoxTouch_00867c10 @ 0x0086_7c10 => extern "C" fn h_867c10(w: u32, _unused: u32) -> u32 {
        hp::atel_check_box_touch(&mut mem(), w, |_, f, args| super::call_cdecl_f64(f, args))
    }
    /// `Atel_CheckLineTouch_008684b0(worker, _, player)`: the four vectors
    /// in a 16-word frame, the math helpers through the game.
    Atel_CheckLineTouch_008684b0 @ 0x0086_84b0 => extern "C" fn h_8684b0(w: u32, _unused: u32, player: u32) {
        let mut frame = super::StackFrame::<16>::new();
        hp::atel_check_line_touch(&mut mem(), w, player, frame.at(), |_, f, args| super::call_cdecl_f64(f, args));
    }
    /// `Atel_CheckBoxTrigger_00866cc0(worker, _, player)`:
    /// `Math_LengthSq3` through the game.
    Atel_CheckBoxTrigger_00866cc0 @ 0x0086_6cc0 => extern "C" fn h_866cc0(w: u32, _unused: u32, player: u32) {
        hp::atel_check_box_trigger(&mut mem(), w, player, |_, f, args| super::call_cdecl_f64(f, args));
    }
    // ---- pc round 8, group a
    /// `Atel_EventFrame_00872bd0()`: the two vibration volumes in a
    /// 2-word frame; `EAX` the flag or `TkClearSoundSyncVFlag3`'s.
    Atel_EventFrame_00872bd0 @ 0x0087_2bd0 => extern "C" fn h_872bd0() -> u32 {
        let mut frame = super::StackFrame::<2>::new();
        hp::atel_event_frame(&mut mem(), frame.at())
    }
}
