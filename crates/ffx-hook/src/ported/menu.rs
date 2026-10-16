//! `ffx-menu`: windows (`Gu_`), message lookups (`Msg_`), the `Tk` menu
//! state and scene data.
//!
//! Results narrower than 32 bits come back widened the way the originals
//! widen them (`movzx`/`movsx`), so callers reading all of `eax` see the
//! same value.

use ffx_menu::{
    gu::bridge as gu, msg::bridge as msg, scene::bridge as scene, scissor::bridge as scissor,
    tk::bridge as tk,
};

use super::{mem, ported};

ported! {
    /// `Gu_PrintSetColor(r, g, b)`.
    Gu_PrintSetColor @ 0x0084_8740 => extern "C" fn gu_print_set_color(r: u32, g: u32, b: u32) {
        gu::gu_print_set_color(&mut mem(), r, g, b);
    }
    /// `Gu_PrintSetX(x)`.
    Gu_PrintSetX @ 0x0084_8820 => extern "C" fn gu_print_set_x(x: i32) {
        gu::gu_print_set_x(&mut mem(), x);
    }
    /// `Gu_PrintSetY(y)`.
    Gu_PrintSetY @ 0x0084_8860 => extern "C" fn gu_print_set_y(y: i32) {
        gu::gu_print_set_y(&mut mem(), y);
    }
    /// `Gu_SetHScale(scale)`.
    Gu_SetHScale @ 0x0084_8b30 => extern "C" fn gu_set_h_scale(scale: i32) {
        gu::gu_set_h_scale(&mut mem(), scale);
    }
    /// `Gu_SetScissor(x0, x1, y0, y1)`: queues a `SCISSOR_1` packet.
    Gu_SetScissor @ 0x0084_8b40 => extern "C" fn gu_set_scissor(x0: i32, x1: i32, y0: i32, y1: i32) {
        gu::gu_set_scissor(&mut mem(), x0, x1, y0, y1);
    }
    /// `Gu_SetTexFlush()`: queues a `TEXFLUSH` packet.
    Gu_SetTexFlush @ 0x0084_8d10 => extern "C" fn gu_set_tex_flush() {
        gu::gu_set_tex_flush(&mut mem());
    }
    /// `Gu_ButtonGetToggleValue(window, id)`.
    Gu_ButtonGetToggleValue @ 0x0084_9330 => extern "C" fn gu_button_get_toggle_value(window: u32, id: u32) -> u32 {
        gu::gu_button_get_toggle_value(&mem(), window, id)
    }
    /// `Gu_ButtonSetToggle(window, id, on)`.
    Gu_ButtonSetToggle @ 0x0084_9380 => extern "C" fn gu_button_set_toggle(window: u32, id: u32, on: u32) {
        gu::gu_button_set_toggle(&mut mem(), window, id, on);
    }
    /// `Gu_ButtonSetToggleValue(window, id, on)`.
    Gu_ButtonSetToggleValue @ 0x0084_93e0 => extern "C" fn gu_button_set_toggle_value(window: u32, id: u32, on: u32) {
        gu::gu_button_set_toggle_value(&mut mem(), window, id, on);
    }
    /// `Gu_Hide(on)`: the old flag.
    Gu_Hide @ 0x0084_9780 => extern "C" fn gu_hide(on: u32) -> u32 {
        gu::gu_hide(&mut mem(), on)
    }
    /// `Gu_MouseGetWindowY(window)`.
    Gu_MouseGetWindowY @ 0x0084_9aa0 => extern "C" fn gu_mouse_get_window_y(window: u32) -> i32 {
        gu::gu_mouse_get_window_y(&mem(), window)
    }
    /// `Gu_MouseGetWindowX(window)`.
    Gu_MouseGetWindowX @ 0x0084_9ac0 => extern "C" fn gu_mouse_get_window_x(window: u32) -> i32 {
        gu::gu_mouse_get_window_x(&mem(), window)
    }
    /// `Gu_MouseSetCapture(a, b)`.
    Gu_MouseSetCapture @ 0x0084_9b60 => extern "C" fn gu_mouse_set_capture(a: u32, b: u32) {
        gu::gu_mouse_set_capture(&mut mem(), a, b);
    }
    /// `Gu_SelectorGetItemName(selector)`.
    Gu_SelectorGetItemName @ 0x0084_9d30 => extern "C" fn gu_selector_get_item_name(selector: u32) -> u32 {
        gu::gu_selector_get_item_name(&mem(), selector)
    }
    /// `Gu_SelectorSetCallback(selector, callback)`.
    Gu_SelectorSetCallback @ 0x0084_9fc0 => extern "C" fn gu_selector_set_callback(selector: u32, callback: u32) {
        gu::gu_selector_set_callback(&mut mem(), selector, callback);
    }
    /// `Gu_SelectorSetItem(selector, i, name, value)`.
    Gu_SelectorSetItem @ 0x0084_9fd0 => extern "C" fn gu_selector_set_item(selector: u32, i: i32, name: u32, value: u32) {
        gu::gu_selector_set_item(&mut mem(), selector, i, name, value);
    }
    /// `Gu_SelectorSetMouseFukki(selector, on)`.
    Gu_SelectorSetMouseFukki @ 0x0084_a0d0 => extern "C" fn gu_selector_set_mouse_fukki(selector: u32, on: u32) {
        gu::gu_selector_set_mouse_fukki(&mut mem(), selector, on);
    }
    /// `Gu_SelectorSetSelect(selector, i)`.
    Gu_SelectorSetSelect @ 0x0084_a0e0 => extern "C" fn gu_selector_set_select(selector: u32, i: i32) {
        gu::gu_selector_set_select(&mut mem(), selector, i);
    }
    /// `Gu_SelectorSetSelectSTR(selector, name)`.
    Gu_SelectorSetSelectSTR @ 0x0084_a110 => extern "C" fn gu_selector_set_select_str(selector: u32, name: u32) {
        gu::gu_selector_set_select_str(&mut mem(), selector, name);
    }
    /// `Gu_WindowControlSetDisable(window, id, on)`.
    Gu_WindowControlSetDisable @ 0x0084_a620 => extern "C" fn gu_window_control_set_disable(window: u32, id: u32, on: u32) {
        gu::gu_window_control_set_disable(&mut mem(), window, id, on);
    }
    /// `Gu_WindowControlSetHide(window, id, on)`.
    Gu_WindowControlSetHide @ 0x0084_a670 => extern "C" fn gu_window_control_set_hide(window: u32, id: u32, on: u32) {
        gu::gu_window_control_set_hide(&mut mem(), window, id, on);
    }
    /// `Gu_WindowGetArg(window)`.
    Gu_WindowGetArg @ 0x0084_aec0 => extern "C" fn gu_window_get_arg(window: u32) -> u32 {
        gu::gu_window_get_arg(&mem(), window)
    }
    /// `Gu_WindowGetWidth(window)`.
    Gu_WindowGetWidth @ 0x0084_afa0 => extern "C" fn gu_window_get_width(window: u32) -> i32 {
        gu::gu_window_get_width(&mem(), window)
    }
    /// `Gu_WindowGetX(window)`.
    Gu_WindowGetX @ 0x0084_afc0 => extern "C" fn gu_window_get_x(window: u32) -> i32 {
        gu::gu_window_get_x(&mem(), window)
    }
    /// `Gu_WindowIsDrag(window)`.
    Gu_WindowIsDrag @ 0x0084_b200 => extern "C" fn gu_window_is_drag(window: u32) -> u32 {
        u32::from(gu::gu_window_is_drag(&mem(), window))
    }
    /// `Gu_WindowOpen(window)`.
    Gu_WindowOpen @ 0x0084_b250 => extern "C" fn gu_window_open(window: u32) {
        gu::gu_window_open(&mut mem(), window);
    }
    /// `Gu_WindowSetArg(window, arg)`.
    Gu_WindowSetArg @ 0x0084_b560 => extern "C" fn gu_window_set_arg(window: u32, arg: u32) {
        gu::gu_window_set_arg(&mut mem(), window, arg);
    }
    /// `Gu_WindowSetClientH(window, h)`.
    Gu_WindowSetClientH @ 0x0084_b580 => extern "C" fn gu_window_set_client_h(window: u32, h: i32) {
        gu::gu_window_set_client_h(&mut mem(), window, h);
    }
    /// `Gu_WindowSetParent(window, parent)`.
    Gu_WindowSetParent @ 0x0084_b660 => extern "C" fn gu_window_set_parent(window: u32, parent: u32) {
        gu::gu_window_set_parent(&mut mem(), window, parent);
    }

    /// `Msg_Text1(i)`: entry `i`'s text in the context's second table.
    Msg_Text1 @ 0x0086_b170 => extern "C" fn msg_text1(i: i32) -> u32 {
        msg::msg_text1(&mut mem(), i)
    }
    /// `Msg_TableTextAlt(table, i)`.
    Msg_TableTextAlt @ 0x0086_ba60 => extern "C" fn msg_table_text_alt(table: u32, i: i32) -> u32 {
        msg::msg_table_text_alt(&mem(), table, i)
    }
    /// `Msg_TextAlt(i)`.
    Msg_TextAlt @ 0x0086_ba90 => extern "C" fn msg_text_alt(i: i32) -> u32 {
        msg::msg_text_alt(&mut mem(), i)
    }
    /// `Msg_TableText(table, i)`.
    Msg_TableText @ 0x0086_bed0 => extern "C" fn msg_table_text(table: u32, i: i32) -> u32 {
        msg::msg_table_text(&mem(), table, i)
    }
    /// `Msg_Flags(i)`.
    Msg_Flags @ 0x0086_bf10 => extern "C" fn msg_flags(i: i32) -> u32 {
        u32::from(msg::msg_flags(&mem(), i))
    }
    /// `Msg_Get(i, alternate, second)`.
    Msg_Get @ 0x0086_bf70 => extern "C" fn msg_get(i: i32, alternate: i32, second: i32) -> u32 {
        msg::msg_get(&mut mem(), i, alternate, second)
    }
    /// `Msg_ClampChoices(window, mode)`.
    Msg_ClampChoices @ 0x0087_1210 => extern "C" fn msg_clamp_choices(window: u32, mode: u32) -> u32 {
        msg::msg_clamp_choices(&mut mem(), window, mode)
    }

    /// `TkGetVibrateVolume(&a, &b)`.
    TkGetVibrateVolume @ 0x0087_5e10 => extern "C" fn tk_get_vibrate_volume(a: u32, b: u32) {
        tk::tk_get_vibrate_volume(&mut mem(), a, b);
    }
    /// `TkSetVibrateMask(pad, on)`.
    TkSetVibrateMask @ 0x0087_5f60 => extern "C" fn tk_set_vibrate_mask(pad: i32, on: i32) {
        tk::tk_set_vibrate_mask(&mut mem(), pad, on);
    }
    /// `TkStopVibrate(pad)`.
    TkStopVibrate @ 0x0087_6290 => extern "C" fn tk_stop_vibrate(pad: i32) {
        tk::tk_stop_vibrate(&mut mem(), pad);
    }
    /// `TkStopVibrateAll()`.
    TkStopVibrateAll @ 0x0087_62f0 => extern "C" fn tk_stop_vibrate_all() {
        tk::tk_stop_vibrate_all(&mut mem());
    }
    /// `TkCreateDECI2Header(out, a, b, c, d)`.
    TkCreateDECI2Header @ 0x0088_5120 => extern "C" fn tk_create_deci2_header(out: u32, a: u32, b: u32, c: u32, d: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original stores the low parts
        tk::tk_create_deci2_header(&mut mem(), out, a as u16, b as u16, c as u8, d as u8);
    }
    /// `TkSndClearKickFlag()`.
    TkSndClearKickFlag @ 0x0088_7ab0 => extern "C" fn tk_snd_clear_kick_flag() {
        tk::tk_snd_clear_kick_flag(&mut mem());
    }
    /// `TkSndReportStatus()`.
    TkSndReportStatus @ 0x0088_7b30 => extern "C" fn tk_snd_report_status() -> u32 {
        u32::from(tk::tk_snd_report_status(&mem()))
    }
    /// `TkSndSetStandbyVoice(v)`.
    TkSndSetStandbyVoice @ 0x0088_7b50 => extern "C" fn tk_snd_set_standby_voice(v: u32) {
        tk::tk_snd_set_standby_voice(&mut mem(), v);
    }
    /// `TkGetLang()`.
    TkGetLang @ 0x0088_7d00 => extern "C" fn tk_get_lang() -> i32 {
        tk::tk_get_lang(&mem())
    }
    /// `FFXSetLang(lang)`: the low byte.
    FFXSetLang @ 0x0088_7d20 => extern "C" fn ffx_set_lang(lang: u32) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        tk::ffx_set_lang(&mut mem(), lang as i8);
    }
    /// `TkClearSoundSyncVFlag()`.
    TkClearSoundSyncVFlag @ 0x0088_7d30 => extern "C" fn tk_clear_sound_sync_v_flag() {
        tk::tk_clear_sound_sync_v_flag(&mut mem());
    }
    /// `TkGetLastSyncVCounter()`: 1.
    TkGetLastSyncVCounter @ 0x0088_7dc0 => extern "C" fn tk_get_last_sync_v_counter() -> u32 {
        tk::tk_get_last_sync_v_counter()
    }
    /// `TkGetSoundSyncVFlag()`: 1.
    TkGetSoundSyncVFlag @ 0x0088_7dd0 => extern "C" fn tk_get_sound_sync_v_flag() -> u32 {
        tk::tk_get_last_sync_v_counter()
    }
    /// `TkGetTotalSyncVCounter()`: 64 bits in `edx:eax`.
    TkGetTotalSyncVCounter @ 0x0088_7e10 => extern "C" fn tk_get_total_sync_v_counter() -> u64 {
        tk::tk_get_total_sync_v_counter(&mem())
    }
    /// `TkGetSyncVCallbackHandle(i)`.
    TkGetSyncVCallbackHandle @ 0x0088_8620 => extern "C" fn tk_get_sync_v_callback_handle(i: i32) -> u32 {
        tk::tk_get_sync_v_callback_handle(&mem(), i)
    }
    /// `TkRegistBGThreadFunc(f)`: the old one.
    TkRegistBGThreadFunc @ 0x0088_8740 => extern "C" fn tk_regist_bg_thread_func(f: u32) -> u32 {
        tk::tk_regist_bg_thread_func(&mut mem(), f)
    }
    /// `TkSetSyncVCallbackPtr(i, f)`.
    TkSetSyncVCallbackPtr @ 0x0088_8790 => extern "C" fn tk_set_sync_v_callback_ptr(i: i32, f: u32) {
        tk::tk_set_sync_v_callback_ptr(&mut mem(), i, f);
    }
    /// `Pad_MapButtons(a, b, buttons)`.
    Pad_MapButtons @ 0x0088_8c40 => extern "C" fn pad_map_buttons(a: i32, b: i32, buttons: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the original reads a u16
        u32::from(tk::pad_map_buttons(&mem(), a, b, buttons as u16))
    }
    /// `TkInitControlerWork()`.
    TkInitControlerWork @ 0x0088_8f00 => extern "C" fn tk_init_controler_work() {
        tk::tk_init_controler_work(&mut mem());
    }
    /// `TkGetSyncH(i)`.
    TkGetSyncH @ 0x0088_c800 => extern "C" fn tk_get_sync_h(i: i32) -> u32 {
        tk::tk_get_sync_h(&mem(), i)
    }
    /// `TkGetBattleMusicSize()`: its address.
    TkGetBattleMusicSize @ 0x0088_d570 => extern "C" fn tk_get_battle_music_size() -> u32 {
        tk::tk_get_battle_music_size(&mem())
    }
    /// `TkGetBattleMusicBuffer()`.
    TkGetBattleMusicBuffer @ 0x0088_d580 => extern "C" fn tk_get_battle_music_buffer() -> u32 {
        tk::tk_get_battle_music_buffer(&mem())
    }
    /// `TkGetEncountCounterEventStart()`.
    TkGetEncountCounterEventStart @ 0x0088_d5c0 => extern "C" fn tk_get_encount_counter_event_start() -> u32 {
        tk::tk_get_encount_counter_event_start(&mem())
    }
    /// `TkGetFFXCofingAdrs()`: the configuration block.
    TkGetFFXCofingAdrs @ 0x0088_d5e0 => extern "C" fn tk_get_ffx_cofing_adrs() -> u32 {
        tk::tk_get_ffx_cofing_adrs(&mem())
    }
    /// `TkGetTGS()`.
    TkGetTGS @ 0x0088_d6d0 => extern "C" fn tk_get_tgs() -> u32 {
        u32::from(tk::tk_get_tgs(&mem()))
    }
    /// `TkDegtoRad(deg)`: in `st0`.
    TkDegtoRad @ 0x0088_9b80 => extern "C" fn tk_degto_rad(deg: f32) -> f32 {
        ffx_menu::math::deg_to_rad(deg)
    }
    /// `TkSetDebugMenuStatus(on)`.
    TkSetDebugMenuStatus @ 0x0088_ea30 => extern "C" fn tk_set_debug_menu_status(on: i32) {
        tk::tk_set_debug_menu_status(&mut mem(), on);
    }
    /// `TkSetEvSaveStatus(p)`.
    TkSetEvSaveStatus @ 0x0088_ea50 => extern "C" fn tk_set_ev_save_status(p: u32) {
        tk::tk_set_ev_save_status(&mut mem(), p);
    }
    /// `TkSetFadeOut(v)`.
    TkSetFadeOut @ 0x0088_eb50 => extern "C" fn tk_set_fade_out(v: u32) {
        tk::tk_set_fade_out(&mut mem(), v);
    }
    /// `Math_InRadius(r, dx, dy, dz)`.
    Math_InRadius @ 0x0088_b730 => extern "C" fn math_in_radius(r: f32, dx: f32, dy: f32, dz: f32) -> u32 {
        u32::from(ffx_menu::math::in_radius(r, dx, dy, dz))
    }
    /// `Math_WrapAngle(a)`: in `st0`.
    Math_WrapAngle @ 0x0088_bc20 => extern "C" fn math_wrap_angle(a: f32) -> f32 {
        ffx_menu::math::wrap_angle(a)
    }
    /// `TkSetDebugFontLocate(x, y)`.
    TkSetDebugFontLocate @ 0x0088_faf0 => extern "C" fn tk_set_debug_font_locate(x: i32, y: i32) {
        tk::tk_set_debug_font_locate(&mut mem(), x, y);
    }
    /// `TkMenuDrawShadeRatioDiff(v)`.
    TkMenuDrawShadeRatioDiff @ 0x008a_9670 => extern "C" fn tk_menu_draw_shade_ratio_diff(v: u32) {
        tk::tk_menu_draw_shade_ratio_diff(&mut mem(), v);
    }
    /// `TkMenuGetCurrentPlayer()`.
    TkMenuGetCurrentPlayer @ 0x008a_9860 => extern "C" fn tk_menu_get_current_player() -> u32 {
        u32::from(tk::tk_menu_get_current_player(&mem()))
    }
    /// `TkMenuGetCurrentPlayerPos()`.
    TkMenuGetCurrentPlayerPos @ 0x008a_9870 => extern "C" fn tk_menu_get_current_player_pos() -> u32 {
        tk::tk_menu_get_current_player_pos(&mem())
    }
    /// `TkMenuGetCurrentSummon()`.
    TkMenuGetCurrentSummon @ 0x008a_9880 => extern "C" fn tk_menu_get_current_summon() -> u32 {
        u32::from(tk::tk_menu_get_current_summon(&mem()))
    }
    /// `TkMenuGetDrawShadeRatio()`.
    TkMenuGetDrawShadeRatio @ 0x008a_98a0 => extern "C" fn tk_menu_get_draw_shade_ratio() -> u32 {
        tk::tk_menu_get_draw_shade_ratio(&mem())
    }
    /// `TkMenuGetPlayerListMax2()`.
    TkMenuGetPlayerListMax2 @ 0x008a_9b50 => extern "C" fn tk_menu_get_player_list_max2() -> u32 {
        tk::tk_menu_get_player_list_max2(&mem())
    }
    /// `TkMenuGetPlayerFromIndex2(i)` (over `TkMenuGetPlayerListMax`).
    TkMenuGetPlayerFromIndex2 @ 0x008a_9ad0 => extern "C" fn tk_menu_get_player_from_index(i: i32) -> u32 {
        u32::from(tk::tk_menu_get_player_from_index(&mem(), i))
    }
    /// The same over `TkMenuGetPlayerListMax2`.
    TkMenuGetPlayerFromIndex2_008a9b00 @ 0x008a_9b00 => extern "C" fn tk_menu_get_player_from_index2(i: i32) -> u32 {
        u32::from(tk::tk_menu_get_player_from_index2(&mem(), i))
    }
    /// `TkMenuGetPlayerListMax()`.
    TkMenuGetPlayerListMax @ 0x008a_9b30 => extern "C" fn tk_menu_get_player_list_max() -> u32 {
        tk::tk_menu_get_player_list_max(&mem())
    }
    /// `TkMenuGetSummonListMax()`.
    TkMenuGetSummonListMax @ 0x008a_9bf0 => extern "C" fn tk_menu_get_summon_list_max() -> u32 {
        tk::tk_menu_get_summon_list_max(&mem())
    }
    /// `TkMenuMainClearSelectWindow(window)`.
    TkMenuMainClearSelectWindow @ 0x008a_a1e0 => extern "C" fn tk_menu_main_clear_select_window(window: u32) {
        tk::tk_menu_main_clear_select_window(&mut mem(), window);
    }
    /// `TkMenuMainGetIgnoreJoinFlag()`.
    TkMenuMainGetIgnoreJoinFlag @ 0x008a_a450 => extern "C" fn tk_menu_main_get_ignore_join_flag() -> u32 {
        tk::tk_menu_main_get_ignore_join_flag(&mem())
    }
    /// `TkMenuMainGetMenuActiveMask()`.
    TkMenuMainGetMenuActiveMask @ 0x008a_a460 => extern "C" fn tk_menu_main_get_menu_active_mask() -> u32 {
        tk::tk_menu_main_get_menu_active_mask(&mem())
    }
    /// `TkMenuMainSetIgnoreJoinFlag(v)`.
    TkMenuMainSetIgnoreJoinFlag @ 0x008a_ad60 => extern "C" fn tk_menu_main_set_ignore_join_flag(v: u32) {
        tk::tk_menu_main_set_ignore_join_flag(&mut mem(), v);
    }
    /// `TkMenuSetCurrentPlayerPos(i)`.
    TkMenuSetCurrentPlayerPos @ 0x008a_ae80 => extern "C" fn tk_menu_set_current_player_pos(i: u32) {
        tk::tk_menu_set_current_player_pos(&mut mem(), i);
    }
    /// `TkMenuSetHelpMessage(id)`.
    TkMenuSetHelpMessage @ 0x008a_aeb0 => extern "C" fn tk_menu_set_help_message(id: i32) {
        tk::tk_menu_set_help_message(&mut mem(), id);
    }
    /// `TkMenuSetNextPlayer2()`.
    TkMenuSetNextPlayer2 @ 0x008a_af40 => extern "C" fn tk_menu_set_next_player2() {
        tk::tk_menu_set_next_player2(&mut mem());
    }
    /// `TkMenuSetPrevPlayer2()`.
    TkMenuSetPrevPlayer2 @ 0x008a_afc0 => extern "C" fn tk_menu_set_prev_player2() {
        tk::tk_menu_set_prev_player2(&mut mem());
    }
    /// `TkMenuSummonEnableMask()`.
    TkMenuSummonEnableMask @ 0x008a_b1e0 => extern "C" fn tk_menu_summon_enable_mask() -> u32 {
        tk::tk_menu_summon_enable_mask(&mem())
    }
    /// `TkMn2GetMainFrameRatio()`.
    TkMn2GetMainFrameRatio @ 0x008a_b230 => extern "C" fn tk_mn2_get_main_frame_ratio() -> u32 {
        tk::tk_mn2_get_main_frame_ratio(&mem())
    }
    /// `TkMn2GetMenuMode()`.
    TkMn2GetMenuMode @ 0x008a_b240 => extern "C" fn tk_mn2_get_menu_mode() -> u32 {
        tk::tk_mn2_get_menu_mode(&mem())
    }
    /// `TkMn2SetMainFrameRatioDiff(v)`.
    TkMn2SetMainFrameRatioDiff @ 0x008a_b260 => extern "C" fn tk_mn2_set_main_frame_ratio_diff(v: u32) {
        tk::tk_mn2_set_main_frame_ratio_diff(&mut mem(), v);
    }
    /// `resetButtonData()`.
    resetButtonData @ 0x008b_1330 => extern "C" fn reset_button_data() {
        tk::reset_button_data(&mut mem());
    }
    /// `setButtonData(a, b)`.
    setButtonData @ 0x008b_1350 => extern "C" fn set_button_data(a: u32, b: u32) {
        tk::set_button_data(&mut mem(), a, b);
    }
    /// `TkCRC(len, data)`.
    TkCRC @ 0x008b_1450 => extern "C" fn tk_crc(len: i32, data: u32) -> u32 {
        u32::from(tk::tk_crc(&mem(), len, data))
    }
    /// `TkMenuRestartSelFileWindow(window)`.
    TkMenuRestartSelFileWindow @ 0x008b_5530 => extern "C" fn tk_menu_restart_sel_file_window(window: u32) {
        tk::tk_menu_restart_sel_file_window(&mut mem(), window);
    }
    /// `TkMenuSaveLoadActive()`.
    TkMenuSaveLoadActive @ 0x008b_55c0 => extern "C" fn tk_menu_save_load_active() {
        tk::tk_menu_save_load_active(&mut mem());
    }
    /// `TkMn2GetSummonGrow()`: the summon growth record.
    TkMn2GetSummonGrow @ 0x008c_1c20 => extern "C" fn tk_mn2_get_summon_grow() -> u32 {
        tk::tk_mn2_get_summon_grow(&mut mem())
    }
    /// `SetSummonGrowMax()`.
    SetSummonGrowMax @ 0x008c_1910 => extern "C" fn set_summon_grow_max() {
        tk::set_summon_grow_max(&mut mem());
    }
    /// `TkMn2SetFontShade(s0, s1, s2, s3)`.
    TkMn2SetFontShade @ 0x008c_2c30 => extern "C" fn tk_mn2_set_font_shade(s0: u32, s1: u32, s2: u32, s3: u32) {
        tk::tk_mn2_set_font_shade(&mut mem(), s0, s1, s2, s3);
    }
    /// `TkMenuGetImportantCounter()`.
    TkMenuGetImportantCounter @ 0x008c_8a10 => extern "C" fn tk_menu_get_important_counter() -> u32 {
        tk::tk_menu_get_important_counter(&mem())
    }
    /// `TkMenuImportantListMax()`.
    TkMenuImportantListMax @ 0x008c_8a20 => extern "C" fn tk_menu_important_list_max() -> u32 {
        tk::tk_menu_important_list_max(&mem())
    }
    /// `TkMenuFlushDummyControler()`.
    TkMenuFlushDummyControler @ 0x008c_a490 => extern "C" fn tk_menu_flush_dummy_controler() {
        tk::tk_menu_flush_dummy_controler(&mut mem());
    }
    /// `TkMenuGetDummyControlerFlag()`.
    TkMenuGetDummyControlerFlag @ 0x008c_a510 => extern "C" fn tk_menu_get_dummy_controler_flag() -> u32 {
        tk::tk_menu_get_dummy_controler_flag(&mem())
    }
    /// `TkMenuSetDummyControlerFlag(v)`: the low byte.
    TkMenuSetDummyControlerFlag @ 0x008c_a570 => extern "C" fn tk_menu_set_dummy_controler_flag(v: u32) {
        #[allow(clippy::cast_possible_truncation)]
        tk::tk_menu_set_dummy_controler_flag(&mut mem(), v as u8);
    }
    /// `TkMenuGetCommandMax(list, enabled, a, b)`.
    TkMenuGetCommandMax @ 0x008d_2d80 => extern "C" fn tk_menu_get_command_max(list: u32, enabled: i32, a: u32, b: u32) -> i32 {
        tk::tk_menu_get_command_max(&mem(), list, enabled, a, b)
    }
    /// `TkMenuGetCommandOffset(list, enabled, a, b)`.
    TkMenuGetCommandOffset @ 0x008d_2dc0 => extern "C" fn tk_menu_get_command_offset(list: u32, enabled: i32, a: u32, b: u32) -> i32 {
        tk::tk_menu_get_command_offset(&mem(), list, enabled, a, b)
    }
    /// `TkMenuDrawEnv()`: queues the menu's drawing environment.
    TkMenuDrawEnv @ 0x008d_cca0 => extern "C" fn tk_menu_draw_env() {
        tk::tk_menu_draw_env(&mut mem());
    }
    /// `TkMenuMainGetOptionMenu(i)`.
    TkMenuMainGetOptionMenu @ 0x008e_2270 => extern "C" fn tk_menu_main_get_option_menu(i: i32) -> u32 {
        tk::tk_menu_main_get_option_menu(&mem(), i)
    }
    /// `TkMenuMainGetMaxOptions()`.
    TkMenuMainGetMaxOptions @ 0x008e_2280 => extern "C" fn tk_menu_main_get_max_options() -> u32 {
        tk::tk_menu_main_get_max_options(&mem())
    }
    /// `TkMenuMainGetOptionEnabled(i)`.
    TkMenuMainGetOptionEnabled @ 0x008e_2290 => extern "C" fn tk_menu_main_get_option_enabled(i: i32) -> i32 {
        tk::tk_menu_main_get_option_enabled(&mem(), i)
    }
    /// `TkMenuCloseSubCmdWindow()`.
    TkMenuCloseSubCmdWindow @ 0x008e_36f0 => extern "C" fn tk_menu_close_sub_cmd_window() {
        tk::tk_menu_close_sub_cmd_window(&mut mem());
    }
    /// `TkMenuRestartSubCmdWindow2()`.
    TkMenuRestartSubCmdWindow2 @ 0x008e_49d0 => extern "C" fn tk_menu_restart_sub_cmd_window2() {
        tk::tk_menu_restart_sub_cmd_window2(&mut mem());
    }
    /// `BeginScissor_XYUV(&x0, &y0, &x1, &y1, &u0, &v0, &u1, &v1)`.
    BeginScissor_XYUV @ 0x008e_5a20 => extern "C" fn begin_scissor_xyuv(x0: u32, y0: u32, x1: u32, y1: u32, u0: u32, v0: u32, u1: u32, v1: u32) -> u32 {
        u32::from(scissor::begin_scissor_xyuv_at(&mut mem(), [x0, y0, x1, y1, u0, v0, u1, v1]))
    }
    /// `CloseScissor()`.
    CloseScissor @ 0x008e_5cd0 => extern "C" fn close_scissor() {
        scissor::close_scissor(&mut mem());
    }
    /// `get_ptr32_func(p)`: `p`.
    get_ptr32_func @ 0x00a4_4650 => extern "C" fn get_ptr32_func(p: u32) -> u32 {
        ffx_menu::get_ptr32_func(p)
    }
    /// `is_ptr32_null(p)`.
    is_ptr32_null @ 0x00a4_46a0 => extern "C" fn is_ptr32_null(p: u32) -> u32 {
        u32::from(ffx_menu::is_ptr32_null(p))
    }
    /// `TkMenuGetParamHandle(i, name)`.
    TkMenuGetParamHandle @ 0x00a7_b7d0 => extern "C" fn tk_menu_get_param_handle(i: i32, name: u32) -> u32 {
        scene::tk_menu_get_param_handle(&mem(), i, name)
    }
    /// `TkMenuGetParamOffset(i, offset, n)`.
    TkMenuGetParamOffset @ 0x00a7_b8c0 => extern "C" fn tk_menu_get_param_offset(i: i32, offset: i32, n: i32) -> u32 {
        scene::tk_menu_get_param_offset(&mem(), i, offset, n)
    }
    /// `TkMenuGetSceneHandle(i, name)`.
    TkMenuGetSceneHandle @ 0x00a7_b920 => extern "C" fn tk_menu_get_scene_handle(i: i32, name: u32) -> u32 {
        scene::tk_menu_get_scene_handle(&mem(), i, name)
    }
    /// `TkMenuInitSeq()`.
    TkMenuInitSeq @ 0x00a7_bb00 => extern "C" fn tk_menu_init_seq() {
        scene::tk_menu_init_seq(&mut mem());
    }
    /// `TkMenuIsTermScene(i, handle)`.
    TkMenuIsTermScene @ 0x00a7_bb90 => extern "C" fn tk_menu_is_term_scene(i: i32, handle: u32) -> u32 {
        scene::tk_menu_is_term_scene(&mem(), i, handle)
    }
    /// `TkMenuSetSceneDataAdrs(i, data)`.
    TkMenuSetSceneDataAdrs @ 0x00a7_c7f0 => extern "C" fn tk_menu_set_scene_data_adrs(i: i32, data: u32) {
        scene::tk_menu_set_scene_data_adrs(&mut mem(), i, data);
    }
}
