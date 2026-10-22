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

/// A `g_TkMenuPacket` wrapper: `build` at `g_TkMenuPacket + 0x10`, whose
/// end becomes the menu packet and the result.
fn menu_packet(build: impl FnOnce(&mut super::RawMem, u32) -> u32) -> u32 {
    let mut m = mem();
    let p = tk::MENU_PACKET.get(&m).wrapping_add(0x10);
    let end = build(&mut m, p);
    tk::MENU_PACKET.set(&mut m, end);
    end
}

/// An MSVCR110 x87 intrinsic through the game's import slot `slot`:
/// `a` in `ST0` (with `b` set, `a` in `ST1` and `b` in `ST0`), the result
/// read back from `ST0` as a double.
fn crt_x87(slot: u32, a: f64, b: Option<f64>) -> f64 {
    // SAFETY: the slot holds the address of a CRT intrinsic taking its
    // operands in ST0/ST1 and returning in ST0 (eax, ecx, edx, x87 and SSE
    // clobbered). Every input operand is used before the call.
    #[cfg(target_arch = "x86")]
    unsafe {
        let f = (crate::rebase(slot) as *const usize).read();
        let ops = [a, b.unwrap_or(0.0)];
        let two = u32::from(b.is_some());
        let (lo, hi): (u32, u32);
        std::arch::asm!(
            "fld qword ptr [{ops}]",
            "test {two}, {two}",
            "jz 2f",
            "fld qword ptr [{ops} + 8]",
            "2:",
            "call {f}",
            "sub esp, 8",
            "fstp qword ptr [esp]",
            "pop eax",
            "pop edx",
            ops = in(reg) ops.as_ptr(),
            two = in(reg) two,
            f = in(reg) f,
            lateout("eax") lo,
            lateout("edx") hi,
            clobber_abi("C"),
        );
        f64::from_bits(u64::from(lo) | (u64::from(hi) << 32))
    }
    #[cfg(not(target_arch = "x86"))]
    {
        let _ = (slot, a, b);
        unreachable!("the CRT's intrinsics: this runs in the 32-bit game")
    }
}

/// `user_free(p)` (`0x006fb8b0`), through the game's entry (the system
/// module's port when hooked).
fn user_free(p: u32) -> u32 {
    call_ptr(
        u32::try_from(crate::rebase(0x006f_b8b0)).expect("32-bit"),
        &[p],
    )
}

/// `user_malloc(size)` (`0x00687020`), through the game's entry.
fn user_malloc(size: u32) -> u32 {
    call_ptr(
        u32::try_from(crate::rebase(0x0068_7020)).expect("32-bit"),
        &[size],
    )
}

/// `graphicGetAbmap()` (`0x00641060`), the renderer's.
fn graphic_get_abmap() -> u32 {
    call_ptr(
        u32::try_from(crate::rebase(0x0064_1060)).expect("32-bit"),
        &[],
    )
}

/// `graphicDrawUIElement(element, 0, 1, 0, 0)` (`0x0063eee0`), the
/// renderer's, on an element built from `fields` (the rest zero).
fn graphic_draw_ui_element(fields: &[(u32, u32)]) -> u32 {
    let mut element = [0u32; 0x26];
    for &(o, v) in fields {
        element[(o / 4) as usize] = v;
    }
    let p = u32::try_from(element.as_mut_ptr() as usize).expect("32-bit");
    call_ptr(
        u32::try_from(crate::rebase(0x0063_eee0)).expect("32-bit"),
        &[p, 0, 1, 0, 0],
    )
}

/// Calls a game function pointer (a menu callback, cdecl).
fn call_ptr(f: u32, args: &[u32]) -> u32 {
    super::call_cdecl(f, args)
}

/// Calls a game function returning its result in `ST0` (cdecl, 0 to 8
/// word arguments), read back as a double.
#[allow(clippy::many_single_char_names)] // the arguments, in order
fn fcall_ptr(f: u32, a: &[u32]) -> f64 {
    type F8 = unsafe extern "C" fn(u32, u32, u32, u32, u32, u32, u32, u32) -> f64;
    let mut w = [0u32; 8];
    let n = a.len().min(8);
    w[..n].copy_from_slice(&a[..n]);
    // SAFETY: a cdecl function of at most 8 word arguments returning in
    // ST0; the extra zero words it doesn't read sit on the caller's side.
    unsafe {
        std::mem::transmute::<usize, F8>(f as usize)(w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7])
    }
}

/// `graphicUiRemapX2`/`graphicUiRemapY2(v)` (`0x006447e0`/`0x00644820`)
/// through the game's entries: the result in `ST0`, read back as a double.
fn ui_remap(y: bool, v: f32) -> f64 {
    let f = crate::rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
    // SAFETY: a cdecl function of one float word returning in ST0.
    unsafe { std::mem::transmute::<usize, unsafe extern "C" fn(u32) -> f64>(f)(v.to_bits()) }
}

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
    /// `Gu_PacketEndTag()`.
    Gu_PacketEndTag @ 0x0084_8460 => extern "C" fn gu_packet_end_tag() {
        gu::gu_packet_end_tag(&mut mem());
    }
    /// `Gu_SetPrim(prim)`.
    Gu_SetPrim @ 0x0084_8480 => extern "C" fn gu_set_prim(prim: i32) {
        gu::gu_set_prim(&mut mem(), prim);
    }
    /// `Gu_SetRgbaq(r, g, b, a)`.
    Gu_SetRgbaq @ 0x0084_84c0 => extern "C" fn gu_set_rgbaq(r: i32, g: i32, b: i32, a: i32) {
        gu::gu_set_rgbaq(&mut mem(), [r, g, b, a]);
    }
    /// `Gu_PacketBeginTag(window)`.
    Gu_PacketBeginTag @ 0x0084_8530 => extern "C" fn gu_packet_begin_tag(window: u32) {
        gu::gu_packet_begin_tag(&mut mem(), window);
    }
    /// `Gu_SetTest(ate, atst, aref, afail, date, datm, zte, ztst)`.
    Gu_SetTest @ 0x0084_85d0 => extern "C" fn gu_set_test(f0: i32, f1: i32, f2: i32, f3: i32, f4: i32, f5: i32, f6: i32, f7: i32) {
        gu::gu_set_test(&mut mem(), [f0, f1, f2, f3, f4, f5, f6, f7]);
    }
    /// `Gu_SetXyzf2(x, y)`.
    Gu_SetXyzf2 @ 0x0084_8680 => extern "C" fn gu_set_xyzf2(x: i32, y: i32) {
        gu::gu_set_xyzf2(&mut mem(), x, y);
    }
    /// `Gu_PrintSetBold(on)`.
    Gu_PrintSetBold @ 0x0084_8720 => extern "C" fn gu_print_set_bold(on: u32) {
        gu::PRINT_BOLD.set(&mut mem(), on);
    }
    /// `Gu_PrintSetItalic(on)`.
    Gu_PrintSetItalic @ 0x0084_8760 => extern "C" fn gu_print_set_italic(on: u32) {
        gu::PRINT_ITALIC.set(&mut mem(), on);
    }
    /// `Gu_PrintSetSize(size)`.
    Gu_PrintSetSize @ 0x0084_8770 => extern "C" fn gu_print_set_size(size: i32) {
        gu::gu_print_set_size(&mut mem(), size);
    }
    /// `Gu_PrintSetSizeOnly(size)`: the first size word alone.
    Gu_PrintSetSizeOnly @ 0x0084_87d0 => extern "C" fn gu_print_set_size_only(size: u32) {
        gu::PRINT_SIZE.set(&mut mem(), size);
    }
    /// `Gu_PrintSetXY(x, y)`.
    Gu_PrintSetXY @ 0x0084_8840 => extern "C" fn gu_print_set_xy(x: i32, y: i32) {
        gu::gu_print_set_xy(&mut mem(), x, y);
    }
    /// `Gu_WindowSetHokanEnable(on)`.
    Gu_WindowSetHokanEnable @ 0x0084_8f00 => extern "C" fn gu_window_set_hokan_enable(on: u32) {
        gu::WINDOW_HOKAN.set(&mut mem(), on);
    }
    /// `Gu_MouseGetX()`.
    Gu_MouseGetX @ 0x0084_9ae0 => extern "C" fn gu_mouse_get_x() -> i32 {
        gu::MOUSE_X.get(&mem())
    }
    /// `Gu_MouseGetY()`.
    Gu_MouseGetY @ 0x0084_9af0 => extern "C" fn gu_mouse_get_y() -> i32 {
        gu::MOUSE_Y.get(&mem())
    }
    /// `Gu_SelectorGetItemValue(selector)`.
    Gu_SelectorGetItemValue @ 0x0084_9d10 => extern "C" fn gu_selector_get_item_value(selector: u32) -> u32 {
        gu::gu_selector_get_item_value(&mem(), selector)
    }
    /// `Gu_WindowStartMove(window)`.
    Gu_WindowStartMove @ 0x0084_a530 => extern "C" fn gu_window_start_move(window: u32) {
        gu::gu_window_start_move(&mut mem(), window);
    }
    /// `Gu_WindowGetClientH(window)`.
    Gu_WindowGetClientH @ 0x0084_aed0 => extern "C" fn gu_window_get_client_h(window: u32) -> i32 {
        gu::gu_window_field(&mem(), window, 0x86)
    }
    /// `Gu_WindowGetClientW(window)`.
    Gu_WindowGetClientW @ 0x0084_aee0 => extern "C" fn gu_window_get_client_w(window: u32) -> i32 {
        gu::gu_window_field(&mem(), window, 0x84)
    }
    /// `Gu_WindowGetClientTop(window)`.
    Gu_WindowGetClientTop @ 0x0084_af00 => extern "C" fn gu_window_get_client_top(window: u32) -> i32 {
        gu::gu_window_field(&mem(), window, 0x82)
    }
    /// `Gu_WindowGetSizeH(window)`.
    Gu_WindowGetSizeH @ 0x0084_af10 => extern "C" fn gu_window_get_size_h(window: u32) -> i32 {
        gu::gu_window_field(&mem(), window, 0x12)
    }
    /// `Gu_WindowGetHeight(window)`.
    Gu_WindowGetHeight @ 0x0084_af30 => extern "C" fn gu_window_get_height(window: u32) -> i32 {
        gu::gu_window_field(&mem(), window, 0x1a)
    }
    /// `Gu_WindowGetSizeW(window)`.
    Gu_WindowGetSizeW @ 0x0084_af90 => extern "C" fn gu_window_get_size_w(window: u32) -> i32 {
        gu::gu_window_field(&mem(), window, 0x10)
    }
    /// `Gu_WindowGetLeft(window)`: x (`+0x0c`).
    Gu_WindowGetLeft @ 0x0084_afb0 => extern "C" fn gu_window_get_left(window: u32) -> i32 {
        gu::gu_window_field(&mem(), window, 0xc)
    }
    /// `Gu_WindowSetPos(window, x, y)`.
    Gu_WindowSetPos @ 0x0084_b220 => extern "C" fn gu_window_set_pos(window: u32, x: u32, y: u32) {
        #[allow(clippy::cast_possible_truncation)] // 16-bit stores
        gu::gu_window_set_pos(&mut mem(), window, x as u16, y as u16);
    }
    /// `Gu_WindowSetFlag2(window, on)`.
    Gu_WindowSetFlag2 @ 0x0084_b600 => extern "C" fn gu_window_set_flag2(window: u32, on: u32) {
        gu::gu_window_set_flag2(&mut mem(), window, on);
    }
    /// `Gu_WindowSetSize(window, w, h)`.
    Gu_WindowSetSize @ 0x0084_b720 => extern "C" fn gu_window_set_size(window: u32, w: u32, h: u32) {
        #[allow(clippy::cast_possible_truncation)] // 16-bit stores
        gu::gu_window_set_size(&mut mem(), window, w as u16, h as u16);
    }
    /// `Gu_WindowToFront(window)`.
    Gu_WindowToFront @ 0x0084_b770 => extern "C" fn gu_window_to_front(window: u32) {
        gu::gu_window_to_front(&mut mem(), window);
    }

    /// `TkChSetBattleChrNum(work)`.
    TkChSetBattleChrNum @ 0x0087_2160 => extern "C" fn tk_ch_set_battle_chr_num(work: u32) {
        tk::tk_ch_set_battle_chr_num(&mut mem(), work);
    }
    /// `TkCheckPlayVoice()`.
    TkCheckPlayVoice @ 0x0087_2340 => extern "C" fn tk_check_play_voice() -> u32 {
        tk::tk_check_play_voice(&mem())
    }
    /// `TkGetVibrateOn(pad)`.
    TkGetVibrateOn @ 0x0087_5de0 => extern "C" fn tk_get_vibrate_on(pad: i32) -> u32 {
        u32::from(tk::tk_get_vibrate_on(&mem(), pad))
    }
    /// `TkGetVibrateCurve(kind, i, curve)`.
    TkGetVibrateCurve @ 0x0087_63e0 => extern "C" fn tk_get_vibrate_curve(kind: u32, i: i32, curve: u32) -> u32 {
        u32::from(tk::tk_get_vibrate_curve(&mem(), kind, i, curve))
    }
    /// `TkSndGetKickFlag()`.
    TkSndGetKickFlag @ 0x0088_7ac0 => extern "C" fn tk_snd_get_kick_flag() -> u32 {
        u32::from(tk::SND_KICK_FLAG.get(&mem()))
    }
    /// `TkSndIsReadVagVoice()`.
    TkSndIsReadVagVoice @ 0x0088_7af0 => extern "C" fn tk_snd_is_read_vag_voice() -> u32 {
        tk::tk_snd_flag(&mem(), 1)
    }
    /// `TkSndIsReadShoutVoice()`.
    TkSndIsReadShoutVoice @ 0x0088_7b00 => extern "C" fn tk_snd_is_read_shout_voice() -> u32 {
        tk::tk_snd_flag(&mem(), 2)
    }
    /// `TkSndIsReadVoice()`.
    TkSndIsReadVoice @ 0x0088_7b10 => extern "C" fn tk_snd_is_read_voice() -> u32 {
        tk::tk_snd_flag(&mem(), 0)
    }
    /// `TkSndIsStandbyVoice()`.
    TkSndIsStandbyVoice @ 0x0088_7b20 => extern "C" fn tk_snd_is_standby_voice() -> u32 {
        tk::tk_snd_flag(&mem(), 3)
    }
    /// `TkGetGsRegisterCSR()`.
    TkGetGsRegisterCSR @ 0x0088_7db0 => extern "C" fn tk_get_gs_register_csr() -> u32 {
        tk::GS_CSR.get(&mem())
    }
    /// `TkGetSyncVCounter()`.
    TkGetSyncVCounter @ 0x0088_7e00 => extern "C" fn tk_get_sync_v_counter() -> u32 {
        tk::SYNC_V_COUNTER.get(&mem())
    }
    /// `TkGetControlerWork2(a, b)`.
    TkGetControlerWork2 @ 0x0088_8e10 => extern "C" fn tk_get_controler_work2(a: i32, b: i32) -> u32 {
        tk::tk_get_controler_work2(&mem(), a, b)
    }
    /// `TkGetControlerWork(a, b)`.
    TkGetControlerWork @ 0x0088_8ec0 => extern "C" fn tk_get_controler_work(a: i32, b: i32) -> u32 {
        tk::tk_get_controler_work(&mem(), a, b)
    }
    /// `Math_MinAbs(a, b)`: in `st0`.
    Math_MinAbs @ 0x0088_b400 => extern "C" fn math_min_abs(a: f32, b: f32) -> f32 {
        ffx_menu::math::min_abs(a, b)
    }
    /// `Math_LengthSq3(x, y, z)`: in `st0`.
    Math_LengthSq3 @ 0x0088_c3c0 => extern "C" fn math_length_sq3(x: f32, y: f32, z: f32) -> f32 {
        ffx_menu::math::length_sq3(x, y, z)
    }
    /// `Math_SinTable(angle)`.
    Math_SinTable @ 0x0088_c690 => extern "C" fn math_sin_table(angle: u32) -> i32 {
        ffx_menu::math::bridge::math_sin_table(&mem(), angle)
    }
    /// `Math_RoundUp(n, step)`.
    Math_RoundUp @ 0x0088_d120 => extern "C" fn math_round_up(n: u32, step: u32) -> u32 {
        ffx_menu::math::round_up(n, step)
    }
    /// `TkGetNowEventJumpId()`.
    TkGetNowEventJumpId @ 0x0088_d660 => extern "C" fn tk_get_now_event_jump_id() -> u32 {
        u32::from(tk::tk_get_now_event_jump_id(&mem()))
    }
    /// `TkGetNowEventJumpIndex()`.
    TkGetNowEventJumpIndex @ 0x0088_d670 => extern "C" fn tk_get_now_event_jump_index() -> u32 {
        u32::from(tk::tk_get_now_event_jump_index(&mem()))
    }
    /// `TkGetNowEventJumpMapIdNo()`.
    TkGetNowEventJumpMapIdNo @ 0x0088_d680 => extern "C" fn tk_get_now_event_jump_map_id_no() -> u32 {
        u32::from(tk::tk_get_now_event_jump_map_id_no(&mem()))
    }
    /// `TkGetNowEventJumpMapNo()`.
    TkGetNowEventJumpMapNo @ 0x0088_d690 => extern "C" fn tk_get_now_event_jump_map_no() -> u32 {
        u32::from(tk::tk_get_now_event_jump_map_no(&mem()))
    }
    /// `TkGetOutputEncount()`.
    TkGetOutputEncount @ 0x0088_d6a0 => extern "C" fn tk_get_output_encount() -> u32 {
        u32::from(tk::OUTPUT_ENCOUNT.get(&mem()))
    }
    /// `TkSetEventJumpMapNo(map, id)`.
    TkSetEventJumpMapNo @ 0x0088_eaf0 => extern "C" fn tk_set_event_jump_map_no(map: u32, id: u32) {
        #[allow(clippy::cast_possible_truncation)] // 16-bit stores
        tk::tk_set_event_jump_map_no(&mut mem(), map as u16, id as u16);
    }
    /// `TkSetOutputEncount(on)`: the low byte.
    TkSetOutputEncount @ 0x0088_ebe0 => extern "C" fn tk_set_output_encount(on: u32) {
        #[allow(clippy::cast_possible_truncation)]
        tk::OUTPUT_ENCOUNT.set(&mut mem(), on as u8);
    }
    /// `TkSetTGS(v)`: the low byte.
    TkSetTGS @ 0x0088_ec10 => extern "C" fn tk_set_tgs(v: u32) {
        #[allow(clippy::cast_possible_truncation)]
        tk::TGS.set(&mut mem(), v as u8);
    }
    /// `TkGetStrWidth(proportional, s)`.
    TkGetStrWidth @ 0x0088_ee70 => extern "C" fn tk_get_str_width(proportional: i32, s: u32) -> i32 {
        tk::tk_get_str_width(&mem(), proportional, s)
    }
    /// `TkMenuGetPlayerStep()`.
    TkMenuGetPlayerStep @ 0x008a_9840 => extern "C" fn tk_menu_get_player_step() -> i32 {
        tk::tk_menu_get_player_step(&mem())
    }
    /// `TkMenuGetCurrentSummonPos()`.
    TkMenuGetCurrentSummonPos @ 0x008a_9890 => extern "C" fn tk_menu_get_current_summon_pos() -> u32 {
        tk::MENU_CURRENT_SUMMON_POS.get(&mem())
    }
    /// `TkMenuGetHP(chr)`.
    TkMenuGetHP @ 0x008a_98c0 => extern "C" fn tk_menu_get_hp(chr: u32) -> u32 {
        tk::tk_menu_get_ply(&mem(), chr, 0x1c)
    }
    /// `TkMenuGetItemHelp(id)`.
    TkMenuGetItemHelp @ 0x008a_98f0 => extern "C" fn tk_menu_get_item_help(id: u32) -> u32 {
        tk::tk_get_item_help(&mut mem(), id)
    }
    /// `TkMenuGetMP(chr)`.
    TkMenuGetMP @ 0x008a_9970 => extern "C" fn tk_menu_get_mp(chr: u32) -> u32 {
        tk::tk_menu_get_ply(&mem(), chr, 0x20)
    }
    /// `TkMenuGetMaxHP(chr)`.
    TkMenuGetMaxHP @ 0x008a_9990 => extern "C" fn tk_menu_get_max_hp(chr: u32) -> u32 {
        tk::tk_menu_get_ply(&mem(), chr, 0x24)
    }
    /// `TkMenuGetMaxMP(chr)`.
    TkMenuGetMaxMP @ 0x008a_99b0 => extern "C" fn tk_menu_get_max_mp(chr: u32) -> u32 {
        tk::tk_menu_get_ply(&mem(), chr, 0x28)
    }
    /// `TkMenuGetPlayerPartyMask()`.
    TkMenuGetPlayerPartyMask @ 0x008a_9b70 => extern "C" fn tk_menu_get_player_party_mask() -> u32 {
        tk::MENU_PLAYER_PARTY_MASK.get(&mem())
    }
    /// `TkMenuGetSummonStep()`.
    TkMenuGetSummonStep @ 0x008a_9bb0 => extern "C" fn tk_menu_get_summon_step() -> i32 {
        tk::tk_menu_get_summon_step(&mem())
    }
    /// `TkMenuGetSummonFromIndex(i)`.
    TkMenuGetSummonFromIndex @ 0x008a_9bc0 => extern "C" fn tk_menu_get_summon_from_index(i: i32) -> u32 {
        u32::from(tk::tk_menu_get_summon_from_index(&mem(), i))
    }
    /// `TkMenuGetTimer()`.
    TkMenuGetTimer @ 0x008a_9c60 => extern "C" fn tk_menu_get_timer() -> u32 {
        u32::from(tk::MENU_TIMER.get(&mem()))
    }
    /// `TkMenuSetNextPlayer()`.
    TkMenuSetNextPlayer @ 0x008a_af10 => extern "C" fn tk_menu_set_next_player() {
        tk::tk_menu_set_next_player(&mut mem());
    }
    /// `TkMenuSetNextSummon()`.
    TkMenuSetNextSummon @ 0x008a_af70 => extern "C" fn tk_menu_set_next_summon() {
        tk::tk_menu_set_next_summon(&mut mem());
    }
    /// `TkMenuSetPrevPlayer()`.
    TkMenuSetPrevPlayer @ 0x008a_afa0 => extern "C" fn tk_menu_set_prev_player() {
        tk::tk_menu_set_prev_player(&mut mem());
    }
    /// `TkMenuSetPrevSummon()`.
    TkMenuSetPrevSummon @ 0x008a_afe0 => extern "C" fn tk_menu_set_prev_summon() {
        tk::tk_menu_set_prev_summon(&mut mem());
    }
    /// `TkMenuGetHelpRatio2()`.
    TkMenuGetHelpRatio2 @ 0x008a_b220 => extern "C" fn tk_menu_get_help_ratio2() -> u32 {
        tk::MENU_HELP_RATIO2.get(&mem())
    }
    /// `TkMsGetRomItem(id, &name)`.
    TkMsGetRomItem @ 0x008a_b280 => extern "C" fn tk_ms_get_rom_item(id: u32, out: u32) -> u32 {
        tk::tk_ms_get_rom_item(&mut mem(), id, out)
    }
    /// `TkMsImportantGet(id)`.
    TkMsImportantGet @ 0x008a_b2c0 => extern "C" fn tk_ms_important_get(id: u32) -> u32 {
        tk::tk_ms_important_get(&mem(), id)
    }
    /// `TkMn2GetCommandHelp(id)`.
    TkMn2GetCommandHelp @ 0x008c_19e0 => extern "C" fn tk_mn2_get_command_help(id: u32) -> u32 {
        tk::tk_get_command_help(&mut mem(), id)
    }
    /// `TkMn2GetCommandName(id)`.
    TkMn2GetCommandName @ 0x008c_1a20 => extern "C" fn tk_mn2_get_command_name(id: u32) -> u32 {
        tk::tk_mn2_get_command_name(&mut mem(), id)
    }
    /// `TkMn2GetItemHelp(id)`.
    TkMn2GetItemHelp @ 0x008c_1b30 => extern "C" fn tk_mn2_get_item_help(id: u32) -> u32 {
        tk::tk_get_item_help(&mut mem(), id)
    }
    /// `TkMn2GetSummonGrowMax()`.
    TkMn2GetSummonGrowMax @ 0x008c_1c40 => extern "C" fn tk_mn2_get_summon_grow_max() -> u32 {
        tk::tk_mn2_get_summon_grow_max(&mut mem())
    }
    /// `TkMn2RestoreFontShade()`.
    TkMn2RestoreFontShade @ 0x008c_2300 => extern "C" fn tk_mn2_restore_font_shade() {
        tk::tk_mn2_restore_font_shade(&mut mem());
    }
    /// `TkMn2SaveFontShade()`.
    TkMn2SaveFontShade @ 0x008c_2330 => extern "C" fn tk_mn2_save_font_shade() {
        tk::tk_mn2_save_font_shade(&mut mem());
    }
    /// `TkCalcImportantListMax()`.
    TkCalcImportantListMax @ 0x008c_6790 => extern "C" fn tk_calc_important_list_max() -> u32 {
        tk::tk_calc_important_list_max(&mut mem())
    }
    /// `TkMenuSetDummyControlerPending(buttons)`.
    TkMenuSetDummyControlerPending @ 0x008c_a560 => extern "C" fn tk_menu_set_dummy_controler_pending(buttons: u32) {
        tk::tk_menu_set_dummy_controler_pending(&mut mem(), buttons);
    }
    /// `Math_Clamp1To99(n)`.
    Math_Clamp1To99 @ 0x008c_ed10 => extern "C" fn math_clamp_1_to_99(n: i32) -> i32 {
        ffx_menu::math::clamp_1_to_99(n)
    }
    /// `TMn2SetStatusBGDiff(step)`.
    TMn2SetStatusBGDiff @ 0x008d_2660 => extern "C" fn tmn2_set_status_bg_diff(step: u32) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // a 16-bit store
        tk::tmn2_set_status_bg_diff(&mut mem(), step as i16);
    }
    /// `TkMn2StepStatusBG()`.
    TkMn2StepStatusBG @ 0x008d_3280 => extern "C" fn tk_mn2_step_status_bg() {
        tk::tk_mn2_step_status_bg(&mut mem());
    }
    /// `TkMn2GetStatusBG()`.
    TkMn2GetStatusBG @ 0x008d_4910 => extern "C" fn tk_mn2_get_status_bg() -> i32 {
        tk::tk_mn2_get_status_bg(&mem())
    }
    /// `TkMn2InitStatusBG()`.
    TkMn2InitStatusBG @ 0x008d_4ed0 => extern "C" fn tk_mn2_init_status_bg() {
        tk::tk_mn2_init_status_bg(&mut mem());
    }
    /// `TkMenuGetTableEntry(i, table)`.
    TkMenuGetTableEntry @ 0x008d_ce10 => extern "C" fn tk_menu_get_table_entry(i: u32, table: u32) -> u32 {
        tk::tk_menu_get_table_entry(&mem(), i, table)
    }
    /// `TkMenuGetCommandHelp(id)`.
    TkMenuGetCommandHelp @ 0x008d_de10 => extern "C" fn tk_menu_get_command_help(id: u32) -> u32 {
        tk::tk_get_command_help(&mut mem(), id)
    }
    /// `TkMn2GetListMax2()`.
    TkMn2GetListMax2 @ 0x008e_3390 => extern "C" fn tk_mn2_get_list_max2() -> u32 {
        tk::MN2_LIST_MAX2.get(&mem())
    }
    /// `BeginScissor_XY(&x0, &y0, &x1, &y1)`.
    BeginScissor_XY @ 0x008e_58e0 => extern "C" fn begin_scissor_xy(x0: u32, y0: u32, x1: u32, y1: u32) -> u32 {
        u32::from(scissor::begin_scissor_xy_at(&mut mem(), [x0, y0, x1, y1]))
    }
    /// `TkMenuSaveLoadCloseWindow2()`: the window it closed (left in `eax`).
    TkMenuSaveLoadCloseWindow2 @ 0x008b_3d90 => extern "C" fn tk_menu_save_load_close_window2() -> u32 {
        tk::close_window(&mut mem(), tk::SAVE_LOAD_WINDOW2, false, true)
    }
    /// `TkMenuSaveLoadCloseWindow()`: the window it closed (left in `eax`).
    TkMenuSaveLoadCloseWindow @ 0x008b_3e40 => extern "C" fn tk_menu_save_load_close_window() -> u32 {
        tk::close_window(&mut mem(), tk::SAVE_LOAD_WINDOW, false, true)
    }
    /// `TkMenuItemCloseWindow3()`: the window it closed (left in `eax`).
    TkMenuItemCloseWindow3 @ 0x008c_6b40 => extern "C" fn tk_menu_item_close_window3() -> u32 {
        tk::close_window(&mut mem(), tk::ITEM_WINDOW3, true, true)
    }
    /// `TkMenuItemCloseWindow()`: the window it closed (left in `eax`).
    TkMenuItemCloseWindow @ 0x008c_6b60 => extern "C" fn tk_menu_item_close_window() -> u32 {
        tk::close_window(&mut mem(), tk::ITEM_WINDOW, true, true)
    }
    /// `TkMenuItemCloseWindow2()`: the window it closed (left in `eax`).
    TkMenuItemCloseWindow2 @ 0x008c_6b80 => extern "C" fn tk_menu_item_close_window2() -> u32 {
        tk::close_window(&mut mem(), tk::ITEM_WINDOW2, false, true)
    }
    /// `TkMenuItemCloseWindow4()`: the window it closed (left in `eax`).
    TkMenuItemCloseWindow4 @ 0x008c_6ba0 => extern "C" fn tk_menu_item_close_window4() -> u32 {
        tk::close_window(&mut mem(), tk::ITEM_WINDOW4, true, true)
    }
    /// `TkMenuSummonCloseWindow()`: the window it closed (left in `eax`).
    TkMenuSummonCloseWindow @ 0x008c_cb40 => extern "C" fn tk_menu_summon_close_window() -> u32 {
        tk::close_window(&mut mem(), tk::SUMMON_WINDOW, false, true)
    }
    /// `TkMn2CloseSummonGrowWindow()`: the window it closed (left in `eax`).
    TkMn2CloseSummonGrowWindow @ 0x008c_d430 => extern "C" fn tk_mn2_close_summon_grow_window() -> u32 {
        tk::close_window(&mut mem(), tk::SUMMON_GROW_WINDOW, false, false)
    }
    /// `TkMenuSummonCloseWindow2()`: the window it closed (left in `eax`).
    TkMenuSummonCloseWindow2 @ 0x008c_d440 => extern "C" fn tk_menu_summon_close_window2() -> u32 {
        tk::close_window(&mut mem(), tk::SUMMON_WINDOW2, false, true)
    }
    /// `TkMenuEquipCloseWindow()`: the window it closed (left in `eax`).
    TkMenuEquipCloseWindow @ 0x008c_f970 => extern "C" fn tk_menu_equip_close_window() -> u32 {
        tk::close_window(&mut mem(), tk::EQUIP_WINDOW, false, true)
    }
    /// `TkMenuLimitCloseWindow()`: the window it closed (left in `eax`).
    TkMenuLimitCloseWindow @ 0x008d_0c90 => extern "C" fn tk_menu_limit_close_window() -> u32 {
        tk::close_window(&mut mem(), tk::LIMIT_WINDOW, false, true)
    }
    /// `TkMenuLimitCloseWindow2()`: the window it closed (left in `eax`).
    TkMenuLimitCloseWindow2 @ 0x008d_0cb0 => extern "C" fn tk_menu_limit_close_window2() -> u32 {
        tk::close_window(&mut mem(), tk::LIMIT_WINDOW2, false, true)
    }
    /// `TkMenuKaizouCloseWindow2()`: the window it closed (left in `eax`).
    TkMenuKaizouCloseWindow2 @ 0x008d_57d0 => extern "C" fn tk_menu_kaizou_close_window2() -> u32 {
        tk::close_window(&mut mem(), tk::KAIZOU_WINDOW2, false, true)
    }
    /// `TkMenuKaizouCloseWindow()`: the window it closed (left in `eax`).
    TkMenuKaizouCloseWindow @ 0x008d_57f0 => extern "C" fn tk_menu_kaizou_close_window() -> u32 {
        tk::close_window(&mut mem(), tk::KAIZOU_WINDOW, false, true)
    }
    /// `TkMenuWShopCloseWindow()`: the window it closed (left in `eax`).
    TkMenuWShopCloseWindow @ 0x008d_9b10 => extern "C" fn tk_menu_w_shop_close_window() -> u32 {
        tk::close_window(&mut mem(), tk::WSHOP_WINDOW, true, true)
    }
    /// `TkMenuWShopCloseWindow3()`: the window it closed (left in `eax`).
    TkMenuWShopCloseWindow3 @ 0x008d_9b90 => extern "C" fn tk_menu_w_shop_close_window3() -> u32 {
        tk::close_window(&mut mem(), tk::WSHOP_WINDOW3, true, true)
    }
    /// `TkMenuWShopCloseWindow2()`: the window it closed (left in `eax`).
    TkMenuWShopCloseWindow2 @ 0x008d_9bb0 => extern "C" fn tk_menu_w_shop_close_window2() -> u32 {
        tk::close_window(&mut mem(), tk::WSHOP_WINDOW2, true, true)
    }
    /// `TkMenuCloseSubCmdWindow1()`: the window it closed (left in `eax`).
    TkMenuCloseSubCmdWindow1 @ 0x008e_3710 => extern "C" fn tk_menu_close_sub_cmd_window1() -> u32 {
        tk::close_window(&mut mem(), tk::MENU_SUB_CMD_WINDOW1, false, true)
    }
    /// `TkMenuItemClearWindow()`: the window (left in `eax`).
    TkMenuItemClearWindow @ 0x008c_93a0 => extern "C" fn tk_menu_item_clear_window() -> u32 {
        tk::clear_window(&mut mem(), tk::ITEM_WINDOW)
    }
    /// `TkMenuItemClearWindow2()`: the window (left in `eax`).
    TkMenuItemClearWindow2 @ 0x008c_93c0 => extern "C" fn tk_menu_item_clear_window2() -> u32 {
        tk::clear_window(&mut mem(), tk::ITEM_WINDOW2)
    }
    /// `TkMenuEquipClearWindow()`: the window (left in `eax`).
    TkMenuEquipClearWindow @ 0x008c_fd40 => extern "C" fn tk_menu_equip_clear_window() -> u32 {
        tk::clear_window(&mut mem(), tk::EQUIP_WINDOW)
    }
    /// `TkMenuLimitClearWindow()`: the window (left in `eax`).
    TkMenuLimitClearWindow @ 0x008d_1b50 => extern "C" fn tk_menu_limit_clear_window() -> u32 {
        tk::clear_window(&mut mem(), tk::LIMIT_WINDOW)
    }
    /// `TkMenuLimitClearWindow2()`: the window (left in `eax`).
    TkMenuLimitClearWindow2 @ 0x008d_1b70 => extern "C" fn tk_menu_limit_clear_window2() -> u32 {
        tk::clear_window(&mut mem(), tk::LIMIT_WINDOW2)
    }
    /// `TkMn2SeqGetPos(name, &pos)`.
    TkMn2SeqGetPos @ 0x008c_2b50 => extern "C" fn tk_mn2_seq_get_pos(name: u32, out: u32) {
        tk::tk_mn2_seq_get_pos(&mut mem(), name, out);
    }
    /// `TkMn2SeqGetRect(name, &rect)`.
    TkMn2SeqGetRect @ 0x008c_2b90 => extern "C" fn tk_mn2_seq_get_rect(name: u32, out: u32) {
        tk::tk_mn2_seq_get_rect(&mut mem(), name, out);
    }
    /// `TkMenuItemRefresh(mode)`.
    TkMenuItemRefresh @ 0x008c_8e60 => extern "C" fn tk_menu_item_refresh(mode: i32) {
        tk::tk_menu_item_refresh(&mut mem(), mode);
    }
    /// `TkSetControlerActuatorMode(a, b, mode)`: the pad work (left in `eax`).
    TkSetControlerActuatorMode @ 0x0088_94d0 => extern "C" fn tk_set_controler_actuator_mode(a: i32, b: i32, mode: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)]
        tk::tk_set_controler_actuator_mode(&mut mem(), a, b, mode as u8)
    }
    /// `Gu_DrawRectStrip(rect)`.
    Gu_DrawRectStrip @ 0x0085_4f60 => extern "C" fn gu_draw_rect_strip(rect: u32) {
        gu::gu_draw_rect_strip(&mut mem(), rect);
    }
    /// `Gu_DrawStart(buffer)`.
    Gu_DrawStart @ 0x0084_7b00 => extern "C" fn gu_draw_start(buffer: u32) {
        gu::gu_draw_start(&mut mem(), buffer);
    }
    /// `Cmd_HealAmount(command, chr)`.
    Cmd_HealAmount @ 0x008c_6920 => extern "C" fn cmd_heal_amount(command: u32, chr: u32) -> i32 {
        tk::cmd_heal_amount(&mut mem(), command, chr)
    }
    /// `TkMenuItemImportantRefresh()`.
    TkMenuItemImportantRefresh @ 0x008c_8ba0 => extern "C" fn tk_menu_item_important_refresh() {
        tk::tk_menu_item_important_refresh(&mut mem());
    }
    /// `TkGetAbilityMapIdFromItemId(id)`.
    TkGetAbilityMapIdFromItemId @ 0x008c_67e0 => extern "C" fn tk_get_ability_map_id_from_item_id(id: u32) -> u32 {
        u32::from(tk::tk_get_ability_map_id_from_item_id(&mut mem(), id))
    }
    /// `TkMenuRestartWindow(window)`: the window (left in `eax`).
    TkMenuRestartWindow @ 0x008d_1b90 => extern "C" fn tk_menu_restart_window(window: u32) -> u32 {
        tk::tk_menu_restart_window(&mut mem(), window)
    }
    /// `TkMenuRestartSubCmdWindow1()`: the window (left in `eax`).
    TkMenuRestartSubCmdWindow1 @ 0x008e_49f0 => extern "C" fn tk_menu_restart_sub_cmd_window1() -> u32 {
        tk::tk_menu_restart_sub_cmd_window1(&mut mem())
    }
    /// `TkMenuSetSubCmdWindowCursor(i)`: the window (left in `eax`).
    TkMenuSetSubCmdWindowCursor @ 0x008e_3b80 => extern "C" fn tk_menu_set_sub_cmd_window_cursor(i: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)]
        tk::tk_menu_set_sub_cmd_window_cursor(&mut mem(), i as u16)
    }
    /// `TkMenuWindowStepFade(window)`: `0x1000` (left in `eax`).
    TkMenuWindowStepFade @ 0x008a_b1f0 => extern "C" fn tk_menu_window_step_fade(window: u32) -> u32 {
        tk::tk_menu_window_step_fade(&mut mem(), window)
    }
    /// `TkMn2WindowStepFade(window)`: the window (left in `eax`).
    TkMn2WindowStepFade @ 0x008d_26d0 => extern "C" fn tk_mn2_window_step_fade(window: u32) -> u32 {
        tk::tk_mn2_window_step_fade(&mut mem(), window)
    }
    /// `TkMenuWindowFadedOut(window)`.
    TkMenuWindowFadedOut @ 0x008d_3200 => extern "C" fn tk_menu_window_faded_out(window: u32) -> u32 {
        u32::from(tk::tk_menu_window_faded_out(&mem(), window))
    }
    /// `Gu_WindowSetFlag1(window, on)`.
    Gu_WindowSetFlag1 @ 0x0084_b680 => extern "C" fn gu_window_set_flag1(window: u32, on: u32) {
        gu::gu_window_set_flag1(&mut mem(), window, on);
    }
    /// `Gu_WindowToggleFlag1(window)`.
    Gu_WindowToggleFlag1 @ 0x0084_b7b0 => extern "C" fn gu_window_toggle_flag1(window: u32) -> i32 {
        gu::gu_window_toggle_flag1(&mut mem(), window)
    }
    /// `Gu_WindowGetFlag2(window)`.
    Gu_WindowGetFlag2 @ 0x0084_af40 => extern "C" fn gu_window_get_flag2(window: u32) -> i32 {
        gu::gu_window_get_flag2(&mem(), window)
    }
    /// `Gu_WindowSetX(window, x)`.
    Gu_WindowSetX @ 0x0084_b6e0 => extern "C" fn gu_window_set_x(window: u32, x: u32) {
        #[allow(clippy::cast_possible_truncation)]
        gu::gu_window_set_x(&mut mem(), window, x as u16);
    }
    /// `Gu_WindowSetY(window, y)`.
    Gu_WindowSetY @ 0x0084_b700 => extern "C" fn gu_window_set_y(window: u32, y: u32) {
        #[allow(clippy::cast_possible_truncation)]
        gu::gu_window_set_y(&mut mem(), window, y as u16);
    }
    /// `Gu_WindowSetW(window, w)`.
    Gu_WindowSetW @ 0x0084_b6b0 => extern "C" fn gu_window_set_w(window: u32, w: u32) {
        #[allow(clippy::cast_possible_truncation)]
        gu::gu_window_set_w(&mut mem(), window, w as u16);
    }
    /// `Gu_WindowSetH(window, h)`.
    Gu_WindowSetH @ 0x0084_b5d0 => extern "C" fn gu_window_set_h(window: u32, h: u32) {
        #[allow(clippy::cast_possible_truncation)]
        gu::gu_window_set_h(&mut mem(), window, h as u16);
    }
    /// `Gu_WindowGetClientLeft(window)`.
    Gu_WindowGetClientLeft @ 0x0084_aef0 => extern "C" fn gu_window_get_client_left(window: u32) -> i32 {
        gu::gu_window_field(&mem(), window, 0x80)
    }
    /// `Gu_SelectorGetSelect(selector)`.
    Gu_SelectorGetSelect @ 0x0084_9d50 => extern "C" fn gu_selector_get_select(selector: u32) -> i32 {
        gu::gu_selector_get_select(&mem(), selector)
    }
    /// `Gu_MouseInRect(rect, dx, dy)`.
    Gu_MouseInRect @ 0x0084_9440 => extern "C" fn gu_mouse_in_rect(rect: u32, dx: i32, dy: i32) -> u32 {
        u32::from(gu::gu_mouse_in_rect(&mem(), rect, dx, dy))
    }
    /// `Gu_MouseInClientRect(window, dx, dy, w, h)`.
    Gu_MouseInClientRect @ 0x0084_9b00 => extern "C" fn gu_mouse_in_client_rect(window: u32, dx: i32, dy: i32, w: i32, h: i32) -> u32 {
        u32::from(gu::gu_mouse_in_client_rect(&mem(), window, dx, dy, w, h))
    }
    /// `Gu_RectSetX(rect, x)`.
    Gu_RectSetX @ 0x0084_92f0 => extern "C" fn gu_rect_set_x(rect: u32, v: u32) {
        #[allow(clippy::cast_possible_truncation)]
        gu::gu_rect_set(&mut mem(), rect, 0, v as u16);
    }
    /// `Gu_RectSetY(rect, y)`.
    Gu_RectSetY @ 0x0084_9310 => extern "C" fn gu_rect_set_y(rect: u32, v: u32) {
        #[allow(clippy::cast_possible_truncation)]
        gu::gu_rect_set(&mut mem(), rect, 1, v as u16);
    }
    /// `Gu_RectSetW(rect, w)`.
    Gu_RectSetW @ 0x0084_92d0 => extern "C" fn gu_rect_set_w(rect: u32, v: u32) {
        #[allow(clippy::cast_possible_truncation)]
        gu::gu_rect_set(&mut mem(), rect, 2, v as u16);
    }
    /// `Gu_RectSetH(rect, h)`.
    Gu_RectSetH @ 0x0084_92b0 => extern "C" fn gu_rect_set_h(rect: u32, v: u32) {
        #[allow(clippy::cast_possible_truncation)]
        gu::gu_rect_set(&mut mem(), rect, 3, v as u16);
    }
    /// `TkMenuGetSaveFileInfo(i)`.
    TkMenuGetSaveFileInfo @ 0x008b_4e40 => extern "C" fn tk_menu_get_save_file_info(i: i32) -> u32 {
        tk::tk_menu_get_save_file_info(&mem(), i)
    }
    /// `TkGetVibratePad(pad)`.
    TkGetVibratePad @ 0x0087_5e50 => extern "C" fn tk_get_vibrate_pad(pad: i32) -> u32 {
        tk::tk_get_vibrate_pad(&mem(), pad)
    }
    /// `TkGetVibrateDefaultCurve(kind)`.
    TkGetVibrateDefaultCurve @ 0x0087_6390 => extern "C" fn tk_get_vibrate_default_curve(kind: u32) -> u32 {
        tk::tk_get_vibrate_default_curve(&mem(), kind)
    }
    /// `TkGetSyncVCallbackPtr(i)`.
    TkGetSyncVCallbackPtr @ 0x0088_8650 => extern "C" fn tk_get_sync_v_callback_ptr(i: i32) -> u32 {
        tk::tk_get_sync_v_callback_ptr(&mem(), i)
    }
    /// `TkSetPadButtonMap(a, b, i, to)`.
    TkSetPadButtonMap @ 0x0088_94f0 => extern "C" fn tk_set_pad_button_map(a: i32, b: i32, i: i32, to: u32) {
        #[allow(clippy::cast_possible_truncation)]
        tk::tk_set_pad_button_map(&mut mem(), a, b, i, to as u8);
    }
    /// `TkFindPadButton(a, b, k)`.
    TkFindPadButton @ 0x0088_9ac0 => extern "C" fn tk_find_pad_button(a: i32, b: i32, k: i32) -> i32 {
        tk::tk_find_pad_button(&mem(), a, b, k)
    }
    /// `TkUpdateControlerActuator(work)`.
    TkUpdateControlerActuator @ 0x0088_9410 => extern "C" fn tk_update_controler_actuator(work: u32) {
        tk::tk_update_controler_actuator(&mut mem(), work);
    }
    /// `TkGetControlerFrame(work, i)`.
    TkGetControlerFrame @ 0x0088_8b60 => extern "C" fn tk_get_controler_frame(work: u32, i: i32) -> u32 {
        tk::tk_get_controler_frame(&mem(), work, i)
    }
    /// `TkSetDebugFontLocate2(x, y)`.
    TkSetDebugFontLocate2 @ 0x0089_04e0 => extern "C" fn tk_set_debug_font_locate2(x: i32, y: i32) {
        tk::tk_set_debug_font_locate2(&mut mem(), x, y);
    }
    /// `TkMenuClearDummyControler()`.
    TkMenuClearDummyControler @ 0x008c_a540 => extern "C" fn tk_menu_clear_dummy_controler() {
        tk::tk_menu_clear_dummy_controler(&mut mem());
    }
    /// `TkSetDebugDrawFlag(on)`: the low byte.
    TkSetDebugDrawFlag @ 0x0088_ea20 => extern "C" fn tk_set_debug_draw_flag(on: u32) {
        #[allow(clippy::cast_possible_truncation)]
        tk::DEBUG_DRAW_FLAG.set(&mut mem(), on as u8);
    }
    /// `TkIsDeci2Send()`, sign-extended.
    TkIsDeci2Send @ 0x0088_5420 => extern "C" fn tk_is_deci2_send() -> i32 {
        i32::from(tk::DECI2_SEND.get(&mem()))
    }
    /// `TkParseLast2Digits(s)`.
    TkParseLast2Digits @ 0x0088_8280 => extern "C" fn tk_parse_last_2_digits(s: u32) -> i32 {
        tk::tk_parse_last_2_digits(&mem(), s)
    }
    /// `TkBackupEventJump(event)`.
    TkBackupEventJump @ 0x0088_e800 => extern "C" fn tk_backup_event_jump(event: u32) {
        tk::tk_backup_event_jump(&mut mem(), event);
    }
    /// `TkRestoreEventJumpPrev(event)`.
    TkRestoreEventJumpPrev @ 0x0088_e7c0 => extern "C" fn tk_restore_event_jump_prev(event: u32) {
        tk::tk_restore_event_jump_prev(&mut mem(), event);
    }
    /// `TkRestoreEventJumpMapIndex(event)`.
    TkRestoreEventJumpMapIndex @ 0x0088_e7e0 => extern "C" fn tk_restore_event_jump_map_index(event: u32) {
        tk::tk_restore_event_jump_map_index(&mut mem(), event);
    }
    /// `TkGetCharWidth(proportional, c)`.
    TkGetCharWidth @ 0x0089_0a00 => extern "C" fn tk_get_char_width(proportional: i32, c: u32) -> i32 {
        tk::tk_get_char_width(&mem(), proportional, c)
    }
    /// `Math_UnitMatrix(m)`.
    Math_UnitMatrix @ 0x0088_c510 => extern "C" fn math_unit_matrix(m: u32) {
        ffx_menu::math::bridge::math_unit_matrix(&mut mem(), m);
    }
    /// `Math_Det3Points(x1, y1, x2, y2, x3, y3)`: in `st0`.
    Math_Det3Points @ 0x0088_bc70 => extern "C" fn math_det3_points(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32) -> f32 {
        ffx_menu::math::det3_points(x1, y1, x2, y2, x3, y3)
    }
    /// `TkScreenToMenuX(x)`: in `st0`.
    TkScreenToMenuX @ 0x008e_2690 => extern "C" fn tk_screen_to_menu_x(x: i32) -> f32 {
        ffx_menu::math::screen_to_menu(x, 512.0, 1920.0)
    }
    /// `TkScreenToMenuY(y)`: in `st0`.
    TkScreenToMenuY @ 0x008e_26b0 => extern "C" fn tk_screen_to_menu_y(y: i32) -> f32 {
        ffx_menu::math::screen_to_menu(y, 416.0, 1080.0)
    }
    /// `Msg_TableFlags(table, i)`.
    Msg_TableFlags @ 0x0086_bf00 => extern "C" fn msg_table_flags(table: u32, i: i32) -> u32 {
        u32::from(msg::msg_table_flags(&mem(), table, i))
    }
    /// `TkMenuGetSceneDataAdrs(i)`.
    TkMenuGetSceneDataAdrs @ 0x00a7_b910 => extern "C" fn tk_menu_get_scene_data_adrs(i: i32) -> u32 {
        scene::tk_menu_get_scene_data_adrs(&mem(), i)
    }
    /// `TkMenuGetSceneRecord(i, handle)`.
    TkMenuGetSceneRecord @ 0x00a7_b8f0 => extern "C" fn tk_menu_get_scene_record(i: i32, handle: u32) -> u32 {
        scene::tk_menu_get_scene_record(&mem(), i, handle)
    }
    /// `TkMenuGetSceneOffsetX(i, handle)`.
    TkMenuGetSceneOffsetX @ 0x00a7_b9d0 => extern "C" fn tk_menu_get_scene_x(i: i32, handle: u32) -> i32 {
        scene::tk_menu_get_scene_field(&mem(), i, handle, 8)
    }
    /// `TkMenuGetSceneOffsetY(i, handle)`.
    TkMenuGetSceneOffsetY @ 0x00a7_ba00 => extern "C" fn tk_menu_get_scene_y(i: i32, handle: u32) -> i32 {
        scene::tk_menu_get_scene_field(&mem(), i, handle, 0xa)
    }
    /// `TkMenuSetSceneOffset(i, handle, x, y)`.
    TkMenuSetSceneOffset @ 0x00a7_c810 => extern "C" fn tk_menu_set_scene_pos(i: i32, handle: u32, x: u32, y: u32) {
        #[allow(clippy::cast_possible_truncation)]
        scene::tk_menu_set_scene_pos(&mut mem(), i, handle, x as u16, y as u16);
    }
    /// `TkMenuSceneHash(name)`.
    TkMenuSceneHash @ 0x008e_5cf0 => extern "C" fn tk_menu_scene_hash(name: u32) -> u32 {
        scene::tk_menu_scene_hash(&mem(), name)
    }
    /// `TkMenuGetParamOffset2(i, offset, n)`: a copy of `TkMenuGetParamOffset`.
    TkMenuGetParamOffset2 @ 0x00a7_b890 => extern "C" fn tk_menu_get_param_offset2(i: i32, offset: i32, n: i32) -> u32 {
        scene::tk_menu_get_param_offset(&mem(), i, offset, n)
    }
    /// `TkSndGetLastKickId()`.
    TkSndGetLastKickId @ 0x0088_7ad0 => extern "C" fn tk_snd_get_last_kick_id() -> u32 {
        tk::tk_snd_get_last_kick_id(&mem())
    }
    /// `TkSndSetFootKickStay(v)`: `v` (left in `eax`).
    TkSndSetFootKickStay @ 0x0088_7b40 => extern "C" fn tk_snd_set_foot_kick_stay(v: u32) -> u32 {
        tk::tk_snd_set_foot_kick_stay(&mut mem(), v);
        v
    }
    /// `TkClearSoundSyncVFlag3()`.
    TkClearSoundSyncVFlag3 @ 0x0088_7d50 => extern "C" fn tk_clear_sound_sync_v_flag3() {
        tk::tk_clear_sound_sync_v_flag3(&mut mem());
    }
    /// `TkGetSoundSyncVFlag3()`.
    TkGetSoundSyncVFlag3 @ 0x0088_7df0 => extern "C" fn tk_get_sound_sync_v_flag3() -> u32 {
        tk::tk_get_sound_sync_v_flag3(&mem())
    }
    /// `TkMcGetLoadFilePos(slot)`.
    TkMcGetLoadFilePos @ 0x0088_8300 => extern "C" fn tk_mc_get_load_file_pos(slot: u32) -> u32 {
        tk::tk_mc_get_load_file_pos(&mem(), slot)
    }
    /// `TkMcSetDebugLock(v)`: `v` (left in `eax`).
    TkMcSetDebugLock @ 0x0088_8420 => extern "C" fn tk_mc_set_debug_lock(v: u32) -> u32 {
        tk::tk_mc_set_debug_lock(&mut mem(), v);
        v
    }
    /// `TkMcSetLoadFilePos(slot, pos)`: `slot` (left in `eax`).
    TkMcSetLoadFilePos @ 0x0088_8480 => extern "C" fn tk_mc_set_load_file_pos(slot: u32, pos: u32) -> u32 {
        tk::tk_mc_set_load_file_pos(&mut mem(), slot, pos);
        slot
    }
    /// `TkGetCommonVibrate()`.
    TkGetCommonVibrate @ 0x0088_d590 => extern "C" fn tk_get_common_vibrate() -> u32 {
        tk::tk_get_common_vibrate(&mem())
    }
    /// `TkGetDebugMenuStatus()`.
    TkGetDebugMenuStatus @ 0x0088_d5b0 => extern "C" fn tk_get_debug_menu_status() -> u32 {
        tk::tk_get_debug_menu_status(&mem())
    }
    /// `TkGetEventNewGame()`.
    TkGetEventNewGame @ 0x0088_d5d0 => extern "C" fn tk_get_event_new_game() -> u32 {
        u32::from(tk::tk_get_event_new_game(&mem()))
    }
    /// `TkGetHddErrorFlag()`.
    TkGetHddErrorFlag @ 0x0088_d5f0 => extern "C" fn tk_get_hdd_error_flag() -> u32 {
        tk::tk_get_hdd_error_flag(&mem())
    }
    /// `TkGetLastEventJumpId()`.
    TkGetLastEventJumpId @ 0x0088_d600 => extern "C" fn tk_get_last_event_jump_id() -> u32 {
        u32::from(tk::tk_get_last_event_jump_id(&mem()))
    }
    /// `TkGetLastEventJumpIndex()`.
    TkGetLastEventJumpIndex @ 0x0088_d610 => extern "C" fn tk_get_last_event_jump_index() -> u32 {
        u32::from(tk::tk_get_last_event_jump_index(&mem()))
    }
    /// `TkGetLastEventJumpMapIdNo()`.
    TkGetLastEventJumpMapIdNo @ 0x0088_d620 => extern "C" fn tk_get_last_event_jump_map_id_no() -> u32 {
        u32::from(tk::tk_get_last_event_jump_map_id_no(&mem()))
    }
    /// `TkGetLastEventJumpMapNo()`.
    TkGetLastEventJumpMapNo @ 0x0088_d630 => extern "C" fn tk_get_last_event_jump_map_no() -> u32 {
        u32::from(tk::tk_get_last_event_jump_map_no(&mem()))
    }
    /// `TkGetLocalStartMapEventId()`.
    TkGetLocalStartMapEventId @ 0x0088_d640 => extern "C" fn tk_get_local_start_map_event_id() -> u32 {
        tk::LOCAL_START_MAP_EVENT.get(&mem())[0]
    }
    /// `TkGetLocalStartMapEventIndex()`.
    TkGetLocalStartMapEventIndex @ 0x0088_d650 => extern "C" fn tk_get_local_start_map_event_index() -> u32 {
        tk::LOCAL_START_MAP_EVENT.get(&mem())[1]
    }
    /// `TkGetStartMapEventId()`.
    TkGetStartMapEventId @ 0x0088_d6b0 => extern "C" fn tk_get_start_map_event_id() -> u32 {
        tk::START_MAP_EVENT.get(&mem())[0]
    }
    /// `TkGetStartMapEventIndex()`.
    TkGetStartMapEventIndex @ 0x0088_d6c0 => extern "C" fn tk_get_start_map_event_index() -> u32 {
        tk::START_MAP_EVENT.get(&mem())[1]
    }
    /// `TkSetEventNewGame(v)`: the save block (left in `eax`).
    TkSetEventNewGame @ 0x0088_eb40 => extern "C" fn tk_set_event_new_game(v: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // a byte store
        tk::tk_set_event_new_game(&mut mem(), v as u8)
    }
    /// `TkSetHddErrorFlag(v)`.
    TkSetHddErrorFlag @ 0x0088_eb70 => extern "C" fn tk_set_hdd_error_flag(v: u32) {
        #[allow(clippy::cast_possible_truncation)] // a byte store
        tk::HDD_ERROR_FLAG.set(&mut mem(), v as u8);
    }
    /// `TkSetLocalStartMapIndex(id, index)`: `index` (left in `eax`).
    TkSetLocalStartMapIndex @ 0x0088_eb80 => extern "C" fn tk_set_local_start_map_index(id: u32, index: u32) -> u32 {
        tk::LOCAL_START_MAP_EVENT.set(&mut mem(), [id, index]);
        index
    }
    /// `TkSetMapGroupDrawStatus(group, on)`.
    TkSetMapGroupDrawStatus @ 0x0088_eba0 => extern "C" fn tk_set_map_group_draw_status(group: u32, on: u32) {
        tk::tk_set_map_group_draw_status(&mut mem(), group, on);
    }
    /// `TkSetStartMapIndex(id, index)`: `index` (left in `eax`).
    TkSetStartMapIndex @ 0x0088_ebf0 => extern "C" fn tk_set_start_map_index(id: u32, index: u32) -> u32 {
        tk::START_MAP_EVENT.set(&mut mem(), [id, index]);
        index
    }
    /// `TkGetControlerActuatorMode(pad, port)`.
    TkGetControlerActuatorMode @ 0x0088_8ba0 => extern "C" fn pad_get_work_byte_9d(pad: i32, port: i32) -> u32 {
        u32::from(tk::pad_get_work_byte_9d(&mem(), pad, port))
    }
    /// `TkGetControlerAnalog(pad, port, i)`.
    TkGetControlerAnalog @ 0x0088_8c00 => extern "C" fn tk_get_controler_analog(pad: i32, port: i32, i: u32) -> u32 {
        tk::tk_get_controler_analog(&mem(), pad, port, i)
    }
    /// `TkGetControlerId(pad, port)`.
    TkGetControlerId @ 0x0088_8d90 => extern "C" fn tk_get_controler_id(pad: i32, port: i32) -> u32 {
        tk::tk_get_controler_id(&mem(), pad, port)
    }
    /// `TkGetControlerPress(pad, port)`.
    TkGetControlerPress @ 0x0088_8dd0 => extern "C" fn tk_get_controler_press(pad: i32, port: i32) -> u32 {
        u32::from(tk::tk_get_controler_press(&mem(), pad, port))
    }
    /// `Pad_SetActuatorFF(pad, port, v)`: the work (left in `eax`).
    Pad_SetActuatorFF @ 0x0088_9490 => extern "C" fn pad_set_actuator_ff(pad: i32, port: i32, v: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // a byte store
        tk::pad_set_work_byte(&mut mem(), pad, port, 0xff, v as u8)
    }
    /// `Pad_SetActuatorFE(pad, port, v)`: the work (left in `eax`).
    Pad_SetActuatorFE @ 0x0088_94b0 => extern "C" fn pad_set_actuator_fe(pad: i32, port: i32, v: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // a byte store
        tk::pad_set_work_byte(&mut mem(), pad, port, 0xfe, v as u8)
    }
    /// `Pad_ResetAnalog(work)`: `work` (left in `eax`).
    Pad_ResetAnalog @ 0x0088_9000 => extern "C" fn pad_reset_analog(work: u32) -> u32 {
        tk::pad_reset_analog(&mut mem(), work, 0x84)
    }
    /// `Pad_ResetAnalogAt0C(work)`: `work` (left in `eax`).
    Pad_ResetAnalogAt0C @ 0x0088_9030 => extern "C" fn pad_reset_analog_at0c(work: u32) -> u32 {
        tk::pad_reset_analog(&mut mem(), work, 0xc)
    }
    /// `Pad_SetActuatorPair(work, a, b)`.
    Pad_SetActuatorPair @ 0x0088_9460 => extern "C" fn pad_set_actuator_pair(work: u32, a: u32, b: u32) {
        #[allow(clippy::cast_possible_truncation)] // byte stores
        tk::pad_set_actuator_pair(&mut mem(), work, a as u8, b as u8);
    }
    /// `Pad_ResetMode(work)`: `work` (left in `eax`).
    Pad_ResetMode @ 0x0088_8ee0 => extern "C" fn pad_reset_mode(work: u32) -> u32 {
        tk::pad_reset_mode(&mut mem(), work)
    }
    /// `TkMenuGetArmor(chr)`.
    TkMenuGetArmor @ 0x008a_9820 => extern "C" fn tk_menu_get_armor(chr: u32) -> u32 {
        u32::from(tk::tk_menu_get_ply_byte(&mem(), chr, 0x2e))
    }
    /// `TkMenuGetLimit(chr)`.
    TkMenuGetLimit @ 0x008a_9930 => extern "C" fn tk_menu_get_limit(chr: u32) -> u32 {
        u32::from(tk::tk_menu_get_ply_byte(&mem(), chr, 0x39))
    }
    /// `TkMenuGetLimitMax(chr)`.
    TkMenuGetLimitMax @ 0x008a_9950 => extern "C" fn tk_menu_get_limit_max(chr: u32) -> u32 {
        u32::from(tk::tk_menu_get_ply_byte(&mem(), chr, 0x3a))
    }
    /// `TkMenuGetSP(chr)`: `MsGetSaveSP`.
    TkMenuGetSP @ 0x008a_9b80 => extern "C" fn tk_menu_get_sp(chr: u32) -> u32 {
        ffx_battle::save::bridge::ms_get_save_sp(&mem(), chr)
    }
    /// `BtlEnd_GetSP(chr)`: `MsGetSaveSP`.
    BtlEnd_GetSP @ 0x008b_da50 => extern "C" fn btl_end_get_sp(chr: u32) -> u32 {
        ffx_battle::save::bridge::ms_get_save_sp(&mem(), chr)
    }
    /// `TkMenuGetSummonTotalFromIndex(i)`.
    TkMenuGetSummonTotalFromIndex @ 0x008a_9c00 => extern "C" fn tk_menu_get_summon_total_from_index(i: i32) -> u32 {
        u32::from(tk::tk_menu_get_summon_total_from_index(&mem(), i))
    }
    /// `TkMenuGetTime()`.
    TkMenuGetTime @ 0x008a_9c50 => extern "C" fn tk_menu_get_time() -> u32 {
        tk::tk_menu_get_time(&mem())
    }
    /// `TkMenuGetWeapon(chr)`.
    TkMenuGetWeapon @ 0x008a_9c70 => extern "C" fn tk_menu_get_weapon(chr: u32) -> u32 {
        u32::from(tk::tk_menu_get_ply_byte(&mem(), chr, 0x2d))
    }
    /// `TkMenuSetHP(chr, hp)`: the record (left in `eax`).
    TkMenuSetHP @ 0x008a_ae90 => extern "C" fn tk_menu_set_hp(chr: u32, hp: u32) -> u32 {
        tk::tk_menu_set_hp(&mut mem(), chr, hp)
    }
    /// `TkMenuSetMP(chr, mp)`: the record (left in `eax`).
    TkMenuSetMP @ 0x008a_aef0 => extern "C" fn tk_menu_set_mp(chr: u32, mp: u32) -> u32 {
        tk::tk_menu_set_ply(&mut mem(), chr, 0x20, mp)
    }
    /// `TkMenuSetSP(chr, sp)`: the record (left in `eax`).
    TkMenuSetSP @ 0x008a_b000 => extern "C" fn tk_menu_set_sp(chr: u32, sp: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // a byte store
        tk::tk_menu_set_ply_byte(&mut mem(), chr, 0x3b, sp as u8)
    }
    /// `TkMenuSetHelpRatioDiff2(v)`: `v` (left in `eax`).
    TkMenuSetHelpRatioDiff2 @ 0x008a_aee0 => extern "C" fn tk_menu_set_help_ratio_diff2(v: u32) -> u32 {
        tk::tk_menu_set_help_ratio_diff2(&mut mem(), v);
        v
    }
    /// `MenuHelp_SetRatio2()`.
    MenuHelp_SetRatio2 @ 0x008a_b270 => extern "C" fn menu_help_set_ratio2() {
        tk::tk_menu_set_help_ratio2(&mut mem());
    }
    /// `TkMenuMainRegistMenu(slot, window)`: the init callback's result.
    TkMenuMainRegistMenu @ 0x008a_aab0 => extern "C" fn tk_menu_main_regist_menu(slot: i32, window: u32) -> u32 {
        tk::tk_menu_main_regist_menu(&mut mem(), slot, window, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuMainRegistWindow(window)`: the callback's result, or the
    /// window.
    TkMenuMainRegistWindow @ 0x008a_ab00 => extern "C" fn tk_menu_main_regist_window(window: u32) -> u32 {
        tk::tk_menu_main_regist_window(&mut mem(), window, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuMainSleepMenu(slot)`.
    TkMenuMainSleepMenu @ 0x008a_ad70 => extern "C" fn tk_menu_main_sleep_menu(slot: i32) {
        tk::tk_menu_main_sleep_menu(&mut mem(), slot, &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMenuMainActiveMenu(slot, arg)`.
    TkMenuMainActiveMenu @ 0x008a_a100 => extern "C" fn tk_menu_main_active_menu(slot: i32, arg: u32) {
        tk::tk_menu_main_active_menu(&mut mem(), slot, arg, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuPlayer_DamageClass(chr)`.
    MenuPlayer_DamageClass @ 0x008a_99f0 => extern "C" fn menu_player_damage_class(chr: u32) -> u32 {
        tk::menu_player_damage_class(&mem(), chr, true)
    }
    /// `MenuPlayer_DamageClass2(chr)`.
    MenuPlayer_DamageClass2 @ 0x008a_9a70 => extern "C" fn menu_player_damage_class2(chr: u32) -> u32 {
        tk::menu_player_damage_class(&mem(), chr, false)
    }
    /// `TkMn2GetListMax()`.
    TkMn2GetListMax @ 0x008c_1bb0 => extern "C" fn tk_mn2_get_list_max() -> u32 {
        tk::tk_mn2_get_list_max(&mem())
    }
    /// `TkMn2RefreshList(kind)`.
    TkMn2RefreshList @ 0x008c_2360 => extern "C" fn tk_mn2_refresh_list(kind: u32) {
        tk::tk_mn2_refresh_list(&mut mem(), kind, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuWeapon_BuildKindList(_, kind)`: the count.
    MenuWeapon_BuildKindList @ 0x008c_fd20 => extern "C" fn menu_weapon_build_kind_list(_unused: u32, kind: u32) -> u32 {
        tk::menu_weapon_build_kind_list(&mut mem(), kind, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuKaizou_BuildList(k)`.
    MenuKaizou_BuildList @ 0x008d_6dd0 => extern "C" fn menu_kaizou_build_list(k: u32) {
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::menu_kaizou_build_list(&mut mem(), k, local, &mut |_, f, a| call_ptr(f, a));
    }
    /// `Pad_InitWorks()`.
    Pad_InitWorks @ 0x0088_9340 => extern "C" fn pad_init_works() {
        tk::pad_init_works(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuConfig_DrawRows()`: the last call's `eax`.
    MenuConfig_DrawRows @ 0x008c_bab0 => extern "C" fn menu_config_draw_rows() -> u32 {
        tk::menu_config_draw_rows(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `BtlEnd_AddAP_008bdaf0(chr, ap, mark)`: `0x008ba390`'s `eax`.
    BtlEnd_AddAP_008bdaf0 @ 0x008b_daf0 => extern "C" fn btl_end_add_ap(chr: u32, ap: u32, mark: u32) -> u32 {
        tk::btl_end_add_ap(&mut mem(), chr, ap, mark, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkBtlEndCheckLegendWeapon()`: the count.
    TkBtlEndCheckLegendWeapon @ 0x008b_a5c0 => extern "C" fn tk_btl_end_check_legend_weapon() -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::tk_btl_end_check_legend_weapon(&mut mem(), local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2RefreshWeaponShopSell()`.
    TkMn2RefreshWeaponShopSell @ 0x008d_9550 => extern "C" fn tk_mn2_refresh_weapon_shop_sell() {
        tk::tk_mn2_refresh_weapon_shop_sell(&mut mem(), 0x13, &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMn2RefreshWeaponShopSell2()`.
    TkMn2RefreshWeaponShopSell2 @ 0x008d_9560 => extern "C" fn tk_mn2_refresh_weapon_shop_sell2() {
        tk::tk_mn2_refresh_weapon_shop_sell(&mut mem(), 0x15, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuWShop_OpenSellWindow()`: the window.
    MenuWShop_OpenSellWindow @ 0x008d_a950 => extern "C" fn menu_wshop_open_sell_window() -> u32 {
        tk::menu_wshop_open_sell_window(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuKaizou_OpenListWindow(k)`: the registration's `eax`.
    MenuKaizou_OpenListWindow @ 0x008d_6ce0 => extern "C" fn menu_kaizou_open_list_window(k: u32) -> u32 {
        tk::menu_kaizou_open_list_window(&mut mem(), k, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuEquip_OpenWeaponWindow(a, kind)`: the registration's `eax`.
    MenuEquip_OpenWeaponWindow @ 0x008c_fc90 => extern "C" fn menu_equip_open_weapon_window(a: u32, kind: u32) -> u32 {
        tk::menu_equip_open_weapon_window(&mut mem(), a, kind, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuLimit_OpenModeWindow(chr)`: the registration's `eax`.
    MenuLimit_OpenModeWindow @ 0x008d_0dd0 => extern "C" fn menu_limit_open_mode_window(chr: u32) -> u32 {
        tk::menu_limit_open_mode_window(&mut mem(), chr, &mut |_, f, a| call_ptr(f, a))
    }
    /// `BtlEnd_CountApStep_008ba4e0(chr, mark)`: 1 while AP are left.
    BtlEnd_CountApStep_008ba4e0 @ 0x008b_a4e0 => extern "C" fn btl_end_count_ap_step(chr: u32, mark: u32) -> u32 {
        tk::btl_end_count_ap_step(&mut mem(), chr, mark, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_OpenGearWindow()`: the registration's `eax`.
    MenuItem_OpenGearWindow @ 0x008c_9330 => extern "C" fn menu_item_open_gear_window() -> u32 {
        tk::menu_item_open_gear_window(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_PackGear()`: `TkMn2RefreshWeaponShopSell2`'s `eax`.
    MenuItem_PackGear @ 0x008c_a120 => extern "C" fn menu_item_pack_gear() -> u32 {
        tk::menu_item_pack_gear(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_DrawKeyCursor(_, x, y)`: the cursor call's `eax`, or 1.
    MenuItem_DrawKeyCursor @ 0x008c_4b60 => extern "C" fn menu_item_draw_key_cursor(_a: u32, x: i32, y: i32) -> u32 {
        tk::menu_item_draw_key_cursor(&mut mem(), [x, y], &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `BtlEnd_CountApSteps()`: 1 while any AP are left.
    BtlEnd_CountApSteps @ 0x008b_a550 => extern "C" fn btl_end_count_ap_steps() -> u32 {
        tk::btl_end_count_ap_steps(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuTuto_ChoiceIs5()`.
    MenuTuto_ChoiceIs5 @ 0x008d_b860 => extern "C" fn menu_tuto_choice_is5() -> u32 {
        tk::menu_tuto_choice_is5(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuKaizou_ListWindowCtrl(w)`: the help draw's `eax`.
    MenuKaizou_ListWindowCtrl @ 0x008d_5810 => extern "C" fn menu_kaizou_list_window_ctrl(w: u32) -> u32 {
        tk::menu_kaizou_list_window_ctrl(&mut mem(), w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_SellWindowCtrl(w)`.
    MenuWShop_SellWindowCtrl @ 0x008d_9df0 => extern "C" fn menu_wshop_sell_window_ctrl(w: u32) -> u32 {
        tk::menu_list_window_ctrl_by_row(&mut mem(), 0, w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_GearWindowCtrl(w)`.
    MenuItem_GearWindowCtrl @ 0x008c_7630 => extern "C" fn menu_item_gear_window_ctrl(w: u32) -> u32 {
        tk::menu_list_window_ctrl_by_row(&mut mem(), 1, w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuLimit_ModeWindowCtrl(w)`.
    MenuLimit_ModeWindowCtrl @ 0x008d_0e90 => extern "C" fn menu_limit_mode_window_ctrl(w: u32) -> u32 {
        tk::menu_list_window_ctrl_by_row(&mut mem(), 2, w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSubCmd_SetBrush_008e4a60(w)`: 1.
    MenuSubCmd_SetBrush_008e4a60 @ 0x008e_4a60 => extern "C" fn menu_sub_cmd_set_brush(w: u32) -> u32 {
        tk::menu_sub_cmd_set_brush(&mut mem(), w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuKaizou_IsSelectable(_, k)`.
    MenuKaizou_IsSelectable @ 0x008d_57a0 => extern "C" fn menu_kaizou_is_selectable(_a: u32, k: u32) -> u32 {
        tk::menu_kaizou_is_selectable(&mut mem(), k, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuConfig_SetBgmType(opt)`.
    MenuConfig_SetBgmType @ 0x008c_b0b0 => extern "C" fn menu_config_set_bgm_type(opt: u32) -> u32 {
        tk::menu_config_option(&mut mem(), 0, opt, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuConfig_SetVibration(opt)`.
    MenuConfig_SetVibration @ 0x008c_afa0 => extern "C" fn menu_config_set_vibration(opt: u32) -> u32 {
        tk::menu_config_option(&mut mem(), 1, opt, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuConfig_GetSoundMode(opt)`.
    MenuConfig_GetSoundMode @ 0x008c_b430 => extern "C" fn menu_config_get_sound_mode(opt: u32) -> u32 {
        tk::menu_config_option(&mut mem(), 2, opt, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSummon_GrowWindowInit(w)`: the rows.
    MenuSummon_GrowWindowInit @ 0x008c_d2e0 => extern "C" fn menu_summon_grow_window_init(w: u32) -> u32 {
        tk::menu_summon_grow_window_init(&mut mem(), w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuConfig_Draw(w)`: `TkMn2DrawKickSyncPacket`'s `eax`.
    MenuConfig_Draw @ 0x008c_b320 => extern "C" fn menu_config_draw(w: u32) -> u32 {
        tk::menu_config_draw(&mut mem(), w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuMessageCb_008e02c0(m)`: 0.
    TkMenuMessageCb_008e02c0 @ 0x008e_02c0 => extern "C" fn tk_menu_message_cb(m: u32) -> u32 {
        tk::tk_menu_message_cb(&mut mem(), m, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuMessageWidth_008de210(m)`: 0.
    TkMenuMessageWidth_008de210 @ 0x008d_e210 => extern "C" fn tk_menu_message_width(m: u32) -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::tk_menu_message_width(&mut mem(), m, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_BuyWindowCtrl(w)`.
    MenuWShop_BuyWindowCtrl @ 0x008d_9da0 => extern "C" fn menu_wshop_buy_window_ctrl(w: u32) -> u32 {
        tk::menu_wshop_buy_window_ctrl(&mut mem(), w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuConfig_SetHdd(opt)`.
    MenuConfig_SetHdd @ 0x008c_b060 => extern "C" fn menu_config_set_hdd(opt: u32) -> u32 {
        tk::menu_config_option2(&mut mem(), 0, opt, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuConfig_SetSoundMode(opt)`.
    MenuConfig_SetSoundMode @ 0x008c_b460 => extern "C" fn menu_config_set_sound_mode(opt: u32) -> u32 {
        tk::menu_config_option2(&mut mem(), 1, opt, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuConfig_GetVibration(opt)`.
    MenuConfig_GetVibration @ 0x008c_af60 => extern "C" fn menu_config_get_vibration(opt: u32) -> u32 {
        tk::menu_config_option2(&mut mem(), 2, opt, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuConfig_SetOption1(opt)`: row 1's record.
    MenuConfig_SetOption1 @ 0x008c_b4c0 => extern "C" fn menu_config_set_option1(opt: u32) -> u32 {
        tk::menu_config_option2(&mut mem(), 3, opt, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuConfig_GetHdd(opt)`.
    MenuConfig_GetHdd @ 0x008c_b020 => extern "C" fn menu_config_get_hdd(opt: u32) -> u32 {
        tk::menu_config_option2(&mut mem(), 4, opt, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuStatus_Start_008d1d80(w, a)`: `TkMn2InitStatusBG`'s `eax`.
    MenuStatus_Start_008d1d80 @ 0x008d_1d80 => extern "C" fn menu_status_start(w: u32, a: u32) -> u32 {
        tk::menu_status_start(&mut mem(), w, a, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSummon_WindowCtrl_008cd6d0(w)`.
    MenuSummon_WindowCtrl_008cd6d0 @ 0x008c_d6d0 => extern "C" fn menu_summon_window_ctrl(w: u32) -> u32 {
        tk::menu_summon_window_ctrl(&mut mem(), w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_DrawBuyPanel(w)`.
    MenuWShop_DrawBuyPanel @ 0x008d_7a40 => extern "C" fn menu_wshop_draw_buy_panel(w: u32) -> u32 {
        tk::menu_shop_draw_panel(&mut mem(), 0, w, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_DrawSellPanel(w)`.
    MenuWShop_DrawSellPanel @ 0x008d_7cc0 => extern "C" fn menu_wshop_draw_sell_panel(w: u32) -> u32 {
        tk::menu_shop_draw_panel(&mut mem(), 1, w, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuShop_DrawBuySellPanel(w)`.
    MenuShop_DrawBuySellPanel @ 0x008d_9fe0 => extern "C" fn menu_shop_draw_buy_sell_panel(w: u32) -> u32 {
        tk::menu_shop_draw_panel(&mut mem(), 2, w, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_StockWindowCtrl(w)`.
    MenuWShop_StockWindowCtrl @ 0x008d_77f0 => extern "C" fn menu_wshop_stock_window_ctrl(w: u32) -> u32 {
        tk::menu_wshop_stock_window_ctrl(&mut mem(), w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuShop_DrawSellPanel(w)`.
    MenuShop_DrawSellPanel @ 0x008d_a730 => extern "C" fn menu_shop_draw_sell_panel(w: u32) -> u32 {
        tk::menu_shop_draw_panel(&mut mem(), 3, w, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_SubWindow1Ctrl(w)`.
    MenuWShop_SubWindow1Ctrl @ 0x008d_79c0 => extern "C" fn menu_wshop_sub_window1_ctrl(w: u32) -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::menu_wshop_sub_window1_ctrl(&mut mem(), w, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuCtrlKaizou(m)`.
    TkMenuCtrlKaizou @ 0x008d_5540 => extern "C" fn tk_menu_ctrl_kaizou(m: u32) {
        tk::tk_menu_ctrl_kaizou(&mut mem(), m, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuStatus_DrawMainWindow(w)`.
    MenuStatus_DrawMainWindow @ 0x008d_2710 => extern "C" fn menu_status_draw_main_window(w: u32) -> u32 {
        tk::menu_status_draw_main_window(&mut mem(), w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TOTextScrollInitLang_008de8c0(w)`: 0.
    TOTextScrollInitLang_008de8c0 @ 0x008d_e8c0 => extern "C" fn to_text_scroll_init_lang(w: u32) -> u32 {
        tk::to_text_scroll_init_lang(&mut mem(), w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TOTextScrollStepLang_008de970(w)`: 0.
    TOTextScrollStepLang_008de970 @ 0x008d_e970 => extern "C" fn to_text_scroll_step_lang(w: u32) -> u32 {
        tk::to_text_scroll_step_lang(&mut mem(), w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuCtrlHelp(m)`.
    TkMenuCtrlHelp @ 0x008b_ef50 => extern "C" fn tk_menu_ctrl_help(m: u32) -> u32 {
        tk::tk_menu_ctrl_help(&mut mem(), m, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkCdReadWithMalloc2(file, size_of, min)`: the buffer.
    TkCdReadWithMalloc2 @ 0x0088_d240 => extern "C" fn tk_cd_read_with_malloc2(file: u32, size_of: u32, min: i32) -> u32 {
        tk::tk_cd_read_with_malloc2(&mut mem(), file, size_of, min, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkSqsLoadInfo_0088e3a0(load)`.
    TkSqsLoadInfo_0088e3a0 @ 0x0088_e3a0 => extern "C" fn tk_sqs_load_info_e3a0(load: u32) {
        tk::tk_sqs_load_info(&mut mem(), false, load, &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkSqsLoadInfo_0088e440(load)`.
    TkSqsLoadInfo_0088e440 @ 0x0088_e440 => extern "C" fn tk_sqs_load_info_e440(load: u32) {
        tk::tk_sqs_load_info(&mut mem(), true, load, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuEquip_Open_008cf830(m)`.
    MenuEquip_Open_008cf830 @ 0x008c_f830 => extern "C" fn menu_equip_open(m: u32) -> u32 {
        tk::menu_equip_open(&mut mem(), m, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuMessageSize_008de250(m)`: 0.
    TkMenuMessageSize_008de250 @ 0x008d_e250 => extern "C" fn tk_menu_message_size(m: u32) -> u32 {
        // the original's w (f32) and h words
        let mut frame = super::StackFrame::<2>::new();
        let local = frame.at();
        tk::tk_menu_message_size(&mut mem(), m, local, &mut |_, f, a| call_ptr(f, a), &mut |_, f, a| fcall_ptr(f, a))
    }
    /// `MenuSaveLoad_DrawSlotList(w)`.
    MenuSaveLoad_DrawSlotList @ 0x008b_4a50 => extern "C" fn menu_save_load_draw_slot_list(w: u32) -> u32 {
        tk::menu_save_load_draw_slot_list(&mut mem(), w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuCtrlWorldMap(m)`.
    TkMenuCtrlWorldMap @ 0x008e_2960 => extern "C" fn tk_menu_ctrl_world_map(m: u32) -> u32 {
        tk::tk_menu_ctrl_world_map(&mut mem(), m, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_Open_008c5f20(m)`.
    MenuItem_Open_008c5f20 @ 0x008c_5f20 => extern "C" fn menu_item_open(m: u32) -> u32 {
        // the original's eight text and help words
        let mut frame = super::StackFrame::<8>::new();
        let local = frame.at();
        tk::menu_item_open(&mut mem(), m, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuMain_DrawCmdWindow(w)`.
    MenuMain_DrawCmdWindow @ 0x008e_20f0 => extern "C" fn menu_main_draw_cmd_window(_w: u32) -> u32 {
        tk::menu_main_draw_cmd_window(&mut mem(), &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_DrawTargetFace(w)`.
    MenuItem_DrawTargetFace @ 0x008c_3ec0 => extern "C" fn menu_item_draw_target_face(_w: u32) -> u32 {
        tk::menu_item_draw_target_face(&mut mem(), &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_SortGearByKind_008ca1b0()`: 1, or 0 when a search fails.
    MenuItem_SortGearByKind_008ca1b0 @ 0x008c_a1b0 => extern "C" fn menu_item_sort_gear_by_kind() -> u32 {
        // the original's 0x400-byte flag buffer and MsGetSaveWeapon's out word
        let mut frame = super::StackFrame::<257>::new();
        let flags = frame.at();
        tk::menu_item_sort_gear_by_kind(&mut mem(), flags, flags.wrapping_add(0x400), &mut |_, f, a| call_ptr(f, a))
    }
    /// `BtlEnd_DrawGilPage_008bbb10(dx)`.
    BtlEnd_DrawGilPage_008bbb10 @ 0x008b_bb10 => extern "C" fn btl_end_draw_gil_page(dx: i32) -> u32 {
        tk::btl_end_draw_gil_page(&mut mem(), dx, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `BtlEnd_DrawWeaponPage_008bbed0(dx)`.
    BtlEnd_DrawWeaponPage_008bbed0 @ 0x008b_bed0 => extern "C" fn btl_end_draw_weapon_page(dx: i32) {
        // MsGetSaveWeapon's out word
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::btl_end_draw_weapon_page(&mut mem(), dx, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuSummon_CmdWindowCtrl_008cced0(w)`.
    MenuSummon_CmdWindowCtrl_008cced0 @ 0x008c_ced0 => extern "C" fn menu_summon_cmd_window_ctrl(w: u32) -> u32 {
        tk::menu_summon_cmd_window_ctrl(&mut mem(), w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_TargetWindowCtrl_008c7430(w)`.
    MenuItem_TargetWindowCtrl_008c7430 @ 0x008c_7430 => extern "C" fn menu_item_target_window_ctrl(w: u32) -> u32 {
        tk::menu_item_target_window_ctrl(&mut mem(), w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2GetWShopPos()`.
    TkMn2GetWShopPos @ 0x008d_9190 => extern "C" fn tk_mn2_get_wshop_pos() -> u32 {
        tk::tk_mn2_get_wshop_pos(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_DrawBuyList(w)`.
    MenuWShop_DrawBuyList @ 0x008d_7aa0 => extern "C" fn menu_wshop_draw_buy_list(w: u32) -> u32 {
        tk::menu_wshop_draw_list(&mut mem(), false, w, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_DrawSellList(w)`.
    MenuWShop_DrawSellList @ 0x008d_7d20 => extern "C" fn menu_wshop_draw_sell_list(w: u32) -> u32 {
        tk::menu_wshop_draw_list(&mut mem(), true, w, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_DrawGearList(w)`.
    MenuItem_DrawGearList @ 0x008c_87e0 => extern "C" fn menu_item_draw_gear_list(w: u32) -> u32 {
        tk::menu_item_draw_gear_list(&mut mem(), w, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `BtlEnd_DrawItemPage_008bbca0(dx)`.
    BtlEnd_DrawItemPage_008bbca0 @ 0x008b_bca0 => extern "C" fn btl_end_draw_item_page(dx: i32) {
        // the digit width word
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::btl_end_draw_item_page(&mut mem(), dx, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuWeapon_DrawList_008cf9f0(w)`.
    MenuWeapon_DrawList_008cf9f0 @ 0x008c_f9f0 => extern "C" fn menu_weapon_draw_list(w: u32) -> u32 {
        tk::menu_weapon_draw_list(&mut mem(), w, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_DrawTargetNames_008c4030(w)`.
    MenuItem_DrawTargetNames_008c4030 @ 0x008c_4030 => extern "C" fn menu_item_draw_target_names(w: u32) -> u32 {
        // the name width word
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::menu_item_draw_target_names(&mut mem(), w, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_DrawSellWindow_008da470(w)`.
    MenuWShop_DrawSellWindow_008da470 @ 0x008d_a470 => extern "C" fn menu_wshop_draw_sell_window(w: u32) -> u32 {
        tk::menu_wshop_draw_sell_window(&mut mem(), w, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSummon_GrowWindowCtrl_008cd460(w)`.
    MenuSummon_GrowWindowCtrl_008cd460 @ 0x008c_d460 => extern "C" fn menu_summon_grow_window_ctrl(w: u32) -> u32 {
        tk::menu_summon_grow_window_ctrl(&mut mem(), w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuLimit_Open_008d0a20(m, a)`.
    MenuLimit_Open_008d0a20 @ 0x008d_0a20 => extern "C" fn menu_limit_open(m: u32, a: u32) -> u32 {
        // the original's two help and two text words
        let mut frame = super::StackFrame::<4>::new();
        let local = frame.at();
        tk::menu_limit_open(&mut mem(), m, a, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuMessageLayout_008de370(m)`: 0.
    TkMenuMessageLayout_008de370 @ 0x008d_e370 => extern "C" fn tk_menu_message_layout(m: u32) -> u32 {
        // the width word
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::tk_menu_message_layout(&mut mem(), m, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkBtlEndInit(_, kind)`.
    TkBtlEndInit @ 0x008b_db70 => extern "C" fn tk_btl_end_init(_a: u32, kind: u32) -> u32 {
        // the original's record out word and amount word
        let mut frame = super::StackFrame::<2>::new();
        let local = frame.at();
        tk::tk_btl_end_init(&mut mem(), kind, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSubCmd_FlipWindowCtrl_008e3880(w)`.
    MenuSubCmd_FlipWindowCtrl_008e3880 @ 0x008e_3880 => extern "C" fn menu_sub_cmd_flip_window_ctrl(w: u32) -> u32 {
        tk::menu_sub_cmd_flip_window_ctrl(&mut mem(), w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuCopyLoadedSave(dst, buffer)`: `setDefaultPlayerName`'s `eax`.
    TkMenuCopyLoadedSave @ 0x008b_54a0 => extern "C" fn tk_menu_copy_loaded_save(dst: u32, buffer: u32) -> u32 {
        tk::tk_menu_copy_loaded_save(&mut mem(), dst, buffer, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkInitSaveWork()`: the last call's `eax` (0 without Asia mode).
    TkInitSaveWork @ 0x0088_df40 => extern "C" fn tk_init_save_work() -> u32 {
        tk::tk_init_save_work(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSelList_DrawList_008d1400(w)`: the last call's `eax`.
    MenuSelList_DrawList_008d1400 @ 0x008d_1400 => extern "C" fn menu_sel_list_draw_list(w: u32) -> u32 {
        tk::menu_sel_list_draw_list(&mut mem(), w, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `Pad_ScanPort_00889050(port, slot, pad)`: `pad` (low byte the
    /// mode nibble on a fresh read).
    Pad_ScanPort_00889050 @ 0x0088_9050 => extern "C" fn pad_scan_port(port: u32, slot: u32, pad: u32) -> u32 {
        tk::pad_scan_port(&mut mem(), port, slot, pad, &mut |_, f, a| call_ptr(f, a))
    }
    /// `BtlEnd_DrawExchangePage_008bd670(dx)`.
    BtlEnd_DrawExchangePage_008bd670 @ 0x008b_d670 => extern "C" fn btl_end_draw_exchange_page(dx: i32) {
        // MsGetSaveWeapon's out word
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::btl_end_draw_exchange_page(&mut mem(), dx, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMenuMessageLayout2_008de5d0(m)`: 0.
    TkMenuMessageLayout2_008de5d0 @ 0x008d_e5d0 => extern "C" fn tk_menu_message_layout2(m: u32) -> u32 {
        // the original's lead, width and w words
        let mut frame = super::StackFrame::<3>::new();
        let local = frame.at();
        tk::tk_menu_message_layout2(&mut mem(), m, local, &mut |_, f, a| call_ptr(f, a), &mut |_, f, a| fcall_ptr(f, a))
    }
    /// `TkMn2GetIShopPos()`: the last plate's `eax`.
    TkMn2GetIShopPos @ 0x008d_b080 => extern "C" fn tk_mn2_get_ishop_pos() -> u32 {
        tk::tk_mn2_get_ishop_pos(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSummon_StatusWindowCtrl_008ccb60(w)`.
    MenuSummon_StatusWindowCtrl_008ccb60 @ 0x008c_cb60 => extern "C" fn menu_summon_status_window_ctrl(w: u32) -> u32 {
        tk::menu_summon_status_window_ctrl(&mut mem(), w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `BtlEnd_DrawCaption_008b9f30()`: the last scissor's `eax`.
    BtlEnd_DrawCaption_008b9f30 @ 0x008b_9f30 => extern "C" fn btl_end_draw_caption() -> u32 {
        // the two text width words
        let mut frame = super::StackFrame::<2>::new();
        let local = frame.at();
        tk::btl_end_draw_caption(&mut mem(), local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuKaizou_Draw_008d6860(w)`: the character plate's `eax`.
    MenuKaizou_Draw_008d6860 @ 0x008d_6860 => extern "C" fn menu_kaizou_draw(w: u32) -> u32 {
        tk::menu_kaizou_draw(&mut mem(), w, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_ListWindowDraw_008c7750(w)`: the last call's `eax`.
    MenuItem_ListWindowDraw_008c7750 @ 0x008c_7750 => extern "C" fn menu_item_list_window_draw(w: u32) -> u32 {
        tk::menu_item_list_window_draw(&mut mem(), w, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuCtrlAbility(m)`.
    TkMenuCtrlAbility @ 0x008c_60b0 => extern "C" fn tk_menu_ctrl_ability(m: u32) -> u32 {
        // MsGetRomPlyCommand's out word
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::tk_menu_ctrl_ability(&mut mem(), m, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSummon_Window2Draw_008ce750(w)`: `TkMn2DrawKickSyncPacket`'s `eax`.
    MenuSummon_Window2Draw_008ce750 @ 0x008c_e750 => extern "C" fn menu_summon_window2_draw(w: u32) -> u32 {
        // the cost and gain words
        let mut frame = super::StackFrame::<2>::new();
        let local = frame.at();
        tk::menu_summon_window2_draw(&mut mem(), w, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuStatus_WindowDraw_008d2790(w)`: `TkMn2DrawKickSyncPacket2`'s `eax`.
    MenuStatus_WindowDraw_008d2790 @ 0x008d_2790 => extern "C" fn menu_status_window_draw(w: u32) -> u32 {
        // the original's 22-word command list
        let mut frame = super::StackFrame::<22>::new();
        let list = frame.at();
        tk::menu_status_window_draw(&mut mem(), w, list, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `DrawGearCustomizationMenu(w)`: `MenuKaizou_DrawWeaponTitle`'s `eax`.
    DrawGearCustomizationMenu @ 0x008d_5f60 => extern "C" fn draw_gear_customization_menu(w: u32) -> u32 {
        tk::draw_gear_customization_menu(&mut mem(), w, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `DrawLimitPageOverDrive(chr)`: the last call's `eax`.
    DrawLimitPageOverDrive @ 0x008d_2270 => extern "C" fn draw_limit_page_over_drive(chr: u32) -> u32 {
        // the text width word (the original uses its argument's slot)
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::draw_limit_page_over_drive(&mut mem(), chr, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `BtlEnd_DrawSpoilListPage_008bd280(dx)`.
    BtlEnd_DrawSpoilListPage_008bd280 @ 0x008b_d280 => extern "C" fn btl_end_draw_spoil_list_page(dx: i32) {
        tk::btl_end_draw_spoil_list_page(&mut mem(), dx, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMenuPreSave()`: `SaveDataUpdateIcon`'s `eax`.
    TkMenuPreSave @ 0x008b_50b0 => extern "C" fn tk_menu_pre_save() -> u32 {
        // the original's 0x16cc-byte frame (the save screen's thread)
        let mut frame = super::StackFrame::<0x5b3>::new();
        let local = frame.at();
        tk::tk_menu_pre_save(&mut mem(), local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2BuildItemList_008c2390(kind, arg)`.
    TkMn2BuildItemList_008c2390 @ 0x008c_2390 => extern "C" fn tk_mn2_build_item_list(kind: u32, arg: u32) {
        let mut frame = super::StackFrame::<3>::new();
        let local = frame.at();
        tk::tk_mn2_build_item_list(&mut mem(), kind, arg, local, &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMn2GetNameModeMax()`.
    TkMn2GetNameModeMax @ 0x008c_1bc0 => extern "C" fn tk_mn2_get_name_mode_max() -> u32 {
        tk::tk_mn2_get_name_mode_max(&mem())
    }
    /// `TkMn2GetOverDriveId(chr)`.
    TkMn2GetOverDriveId @ 0x008c_1bd0 => extern "C" fn tk_mn2_get_over_drive_id(chr: u32) -> u32 {
        u32::from(tk::tk_menu_get_ply_byte(&mem(), chr, 0x38))
    }
    /// `TkMn2GetOverDriveType(chr)`.
    TkMn2GetOverDriveType @ 0x008c_1bf0 => extern "C" fn tk_mn2_get_over_drive_type(chr: u32) -> u32 {
        tk::tk_menu_get_ply(&mem(), chr, 0x88)
    }
    /// `TkMn2SetOverDriveId(chr, v)`: the record (left in `eax`).
    TkMn2SetOverDriveId @ 0x008c_2c90 => extern "C" fn tk_mn2_set_over_drive_id(chr: u32, v: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // a byte store
        tk::tk_menu_set_ply_byte(&mut mem(), chr, 0x38, v as u8)
    }
    /// `MenuSummon_GrowEntry(i)`.
    MenuSummon_GrowEntry @ 0x008c_ecf0 => extern "C" fn menu_summon_grow_entry(i: u32) -> u32 {
        tk::menu_summon_grow_entry(&mut mem(), i)
    }
    /// `MenuWorldMap_CloseWindow()`: the window (left in `eax`).
    MenuWorldMap_CloseWindow @ 0x008e_2b10 => extern "C" fn menu_world_map_close_window() -> u32 {
        tk::close_window(&mut mem(), tk::WORLD_MAP_WINDOW, false, true)
    }
    /// `MenuWShop_CloseSubWindow0()`: the window (left in `eax`).
    MenuWShop_CloseSubWindow0 @ 0x008d_7610 => extern "C" fn menu_wshop_close_sub_window0() -> u32 {
        tk::close_window(&mut mem(), tk::WSHOP_WINDOWS[0], true, true)
    }
    /// `MenuWShop_CloseSubWindow1()`: the window (left in `eax`).
    MenuWShop_CloseSubWindow1 @ 0x008d_7650 => extern "C" fn menu_wshop_close_sub_window1() -> u32 {
        tk::close_window(&mut mem(), tk::WSHOP_WINDOWS[1], true, true)
    }
    /// `MenuWShop_CloseSubWindow2()`: the window (left in `eax`).
    MenuWShop_CloseSubWindow2 @ 0x008d_75f0 => extern "C" fn menu_wshop_close_sub_window2() -> u32 {
        tk::close_window(&mut mem(), tk::WSHOP_WINDOWS[2], true, true)
    }
    /// `MenuWShop_CloseSubWindow3()`: the window (left in `eax`).
    MenuWShop_CloseSubWindow3 @ 0x008d_7630 => extern "C" fn menu_wshop_close_sub_window3() -> u32 {
        tk::close_window(&mut mem(), tk::WSHOP_WINDOWS[3], true, true)
    }
    /// `MenuWShop_CloseSubWindow4()`: the window (left in `eax`).
    MenuWShop_CloseSubWindow4 @ 0x008d_7670 => extern "C" fn menu_wshop_close_sub_window4() -> u32 {
        tk::close_window(&mut mem(), tk::WSHOP_WINDOWS[4], true, true)
    }
    /// `MenuSummon_CloseStatusWindow()`.
    MenuSummon_CloseStatusWindow @ 0x008c_cb10 => extern "C" fn menu_summon_close_status_window() {
        tk::menu_summon_close_status_window(&mut mem());
    }
    /// `MenuSummon_RestartGrowWindow()`: the window (left in `eax`).
    MenuSummon_RestartGrowWindow @ 0x008c_ee30 => extern "C" fn menu_summon_restart_grow_window() -> u32 {
        tk::restart_global_window(&mut mem(), tk::SUMMON_GROW_WINDOW)
    }
    /// `MenuSummon_RestartWindow2()`: the window (left in `eax`).
    MenuSummon_RestartWindow2 @ 0x008c_ee50 => extern "C" fn menu_summon_restart_window2() -> u32 {
        tk::restart_global_window(&mut mem(), tk::SUMMON_WINDOW2)
    }
    /// `MenuKaizou_RestartWindow2()`: the window (left in `eax`).
    MenuKaizou_RestartWindow2 @ 0x008d_6e00 => extern "C" fn menu_kaizou_restart_window2() -> u32 {
        tk::restart_global_window(&mut mem(), tk::KAIZOU_WINDOW2)
    }
    /// `MenuTuto_RestartWindow()`: the window (left in `eax`).
    MenuTuto_RestartWindow @ 0x008d_aaa0 => extern "C" fn menu_tuto_restart_window() -> u32 {
        tk::restart_global_window(&mut mem(), tk::TUTO_WINDOWS[0])
    }
    /// `MenuTuto_RestartWindow2()`: the window (left in `eax`).
    MenuTuto_RestartWindow2 @ 0x008d_aac0 => extern "C" fn menu_tuto_restart_window2() -> u32 {
        tk::menu_tuto_restart_window2(&mut mem())
    }
    /// `MenuWShop_RestartListWindow()`: the window (left in `eax`).
    MenuWShop_RestartListWindow @ 0x008d_8420 => extern "C" fn menu_wshop_restart_list_window() -> u32 {
        tk::menu_wshop_restart_list_window(&mut mem())
    }
    /// `MenuStatus_CloseWindow(i)`: the window (left in `eax`).
    MenuStatus_CloseWindow @ 0x008d_2680 => extern "C" fn menu_status_close_window(i: u32) -> u32 {
        tk::menu_status_close_window(&mut mem(), i)
    }
    /// `MenuSummon_RestartWindow()`.
    MenuSummon_RestartWindow @ 0x008c_d1f0 => extern "C" fn menu_summon_restart_window() {
        tk::menu_summon_restart_window(&mut mem());
    }
    /// `TkMenuRefreshSubCmdHelp()`.
    TkMenuRefreshSubCmdHelp @ 0x008e_49b0 => extern "C" fn menu_sub_cmd_refresh_help() {
        tk::menu_sub_cmd_refresh_help(&mut mem());
    }
    /// `MenuSubCmd_SetValue(v)`.
    MenuSubCmd_SetValue @ 0x008e_4a10 => extern "C" fn menu_sub_cmd_set_value(v: u32) {
        #[allow(clippy::cast_possible_truncation)] // a 16-bit store
        tk::menu_sub_cmd_set_value(&mut mem(), tk::MENU_SUB_CMD_WINDOW, v as u16);
    }
    /// `MenuSubCmd_SetValue2(v)`.
    MenuSubCmd_SetValue2 @ 0x008e_4a30 => extern "C" fn menu_sub_cmd_set_value2(v: u32) {
        #[allow(clippy::cast_possible_truncation)] // a 16-bit store
        tk::menu_sub_cmd_set_value(&mut mem(), tk::MENU_SUB_CMD_WINDOW2, v as u16);
    }
    /// `MenuItem_IsUsable(window, list)`.
    MenuItem_IsUsable @ 0x008c_9120 => extern "C" fn menu_item_is_usable(_window: u32, list: i32) -> u32 {
        tk::menu_item_is_usable(&mem(), list)
    }
    /// `MenuList_IsEmptySlot(entry)`.
    MenuList_IsEmptySlot @ 0x008b_f720 => extern "C" fn menu_list_is_empty_slot(entry: u32) -> u32 {
        tk::menu_list_is_empty_slot(&mem(), entry)
    }
    /// `MenuMain_OptionY(i)`.
    MenuMain_OptionY @ 0x008e_2260 => extern "C" fn menu_main_option_y(i: u32) -> i32 {
        tk::menu_main_option_y(&mem(), i)
    }
    /// `MenuTuto_GetChoice()`.
    MenuTuto_GetChoice @ 0x008d_b840 => extern "C" fn menu_tuto_get_choice() -> i32 {
        tk::menu_tuto_get_choice(&mem())
    }
    /// `MenuTuto_SetChoice(window, i)`: `i` (left in `eax`).
    MenuTuto_SetChoice @ 0x008d_b7a0 => extern "C" fn menu_tuto_set_choice(_window: u32, i: i32) -> i32 {
        tk::menu_tuto_set_choice(&mut mem(), i)
    }
    /// `MenuItem_SetCell(x, y)`.
    MenuItem_SetCell @ 0x008c_5260 => extern "C" fn menu_item_set_cell(x: u32, y: u32) {
        tk::menu_item_set_cell(&mut mem(), x, y);
    }
    /// `MenuItem_SetTarget(chr)`: `chr` (left in `eax`).
    MenuItem_SetTarget @ 0x008c_5440 => extern "C" fn menu_item_set_target(chr: u32) -> u32 {
        tk::MENU_ITEM_TARGET.set(&mut mem(), chr);
        chr
    }
    /// `MenuItem_GetRows()`.
    MenuItem_GetRows @ 0x008c_8a00 => extern "C" fn menu_item_get_rows() -> u32 {
        tk::MENU_ITEM_ROWS.get(&mem())
    }
    /// `MenuEquip_IsAltMode()`.
    MenuEquip_IsAltMode @ 0x008c_fc30 => extern "C" fn menu_equip_is_alt_mode() -> u32 {
        u32::from(tk::MENU_EQUIP_ALT_MODE.get(&mem()) != 0)
    }
    /// `MenuSaveLoad_ResetToggles()`.
    MenuSaveLoad_ResetToggles @ 0x008b_4eb0 => extern "C" fn menu_save_load_reset_toggles() {
        tk::MENU_SAVE_LOAD_TOGGLES.set(&mut mem(), [0, 0]);
    }
    /// `BtlEnd_SetFlag()`.
    BtlEnd_SetFlag @ 0x008b_db60 => extern "C" fn btl_end_set_flag() {
        tk::BTL_END_FLAG.set(&mut mem(), 1);
    }
    /// `BtlEnd_GetResult(i)`.
    BtlEnd_GetResult @ 0x008b_da10 => extern "C" fn btl_end_get_result(i: u32) -> u32 {
        tk::btl_end_get_result(&mem(), i)
    }
    /// `BtlEnd_GetGilWon()`: result 19.
    BtlEnd_GetGilWon @ 0x008b_da20 => extern "C" fn btl_end_get_gil_won() -> u32 {
        tk::btl_end_get_result(&mem(), 19)
    }
    /// `Pause_Request()`.
    Pause_Request @ 0x008a_ae00 => extern "C" fn pause_request() {
        tk::PAUSE_REQUEST.set(&mut mem(), 1);
    }
    /// `MenuMn2_StepDrawY()`.
    MenuMn2_StepDrawY @ 0x008c_0a10 => extern "C" fn menu_mn2_step_draw_y() {
        let mut m = mem();
        let v = tk::MN2_DRAW_Y.get(&m).wrapping_add(0x10);
        tk::MN2_DRAW_Y.set(&mut m, v);
    }
    /// `TkMenuGetDummyControler()`.
    TkMenuGetDummyControler @ 0x008c_a500 => extern "C" fn tk_menu_get_dummy_controler() -> u32 {
        tk::MENU_DUMMY_CONTROLER.get(&mem())
    }
    /// `TkMenuGetDummyControlerR()`.
    TkMenuGetDummyControlerR @ 0x008c_a520 => extern "C" fn tk_menu_get_dummy_controler_r() -> u32 {
        tk::MENU_DUMMY_CONTROLER.get(&mem())
    }
    /// `TkMenuGetDummyControlerT()`.
    TkMenuGetDummyControlerT @ 0x008c_a530 => extern "C" fn tk_menu_get_dummy_controler_t() -> u32 {
        tk::MENU_DUMMY_CONTROLER.get(&mem())
    }
    /// `MenuMn2_Plus80(v)`.
    MenuMn2_Plus80 @ 0x008c_22f0 => extern "C" fn menu_mn2_plus80(v: u32) -> u32 {
        ffx_menu::tk::plus_80(v)
    }
    /// `MenuItem_Less(a, b)`.
    MenuItem_Less @ 0x008c_6a60 => extern "C" fn menu_item_less(a: i32, b: i32) -> u32 {
        u32::from(ffx_menu::tk::less(a, b))
    }
    /// `BtlEnd_RowX(i, n)`.
    BtlEnd_RowX @ 0x008b_a370 => extern "C" fn btl_end_row_x(i: i32, n: i32) -> i32 {
        ffx_menu::tk::btl_end_row_x(i, n)
    }
    /// `MenuHelp_Ratio(a, b)`.
    MenuHelp_Ratio @ 0x008b_f120 => extern "C" fn menu_help_ratio(a: i32, b: i32) -> i32 {
        ffx_menu::tk::menu_help_ratio(a, b)
    }
    /// `MenuItem_UsableByte(list, i)`.
    MenuItem_UsableByte @ 0x008c_6a30 => extern "C" fn menu_item_usable_byte(_list: u32, i: u32) -> u32 {
        u32::from(tk::menu_item_usable_byte(&mem(), i))
    }
    /// `MenuItem_UsableByte2(list, i)`.
    MenuItem_UsableByte2 @ 0x008c_6a40 => extern "C" fn menu_item_usable_byte2(_list: u32, i: u32) -> u32 {
        u32::from(tk::menu_item_usable_byte(&mem(), i))
    }
    /// `MenuWShop_UsableByte(list, i)`.
    MenuWShop_UsableByte @ 0x008d_9b00 => extern "C" fn menu_wshop_usable_byte(_list: u32, i: u32) -> u32 {
        u32::from(tk::menu_item_usable_byte(&mem(), i))
    }
    /// `MenuKaizou_ItemUsable(list, i)`.
    MenuKaizou_ItemUsable @ 0x008d_5730 => extern "C" fn menu_kaizou_item_usable(_list: u32, i: u32) -> u32 {
        u32::from(tk::menu_kaizou_item_usable(&mem(), i))
    }
    /// `MenuWShop_ItemPresent(list, i)`.
    MenuWShop_ItemPresent @ 0x008d_75d0 => extern "C" fn menu_wshop_item_present(_list: u32, i: u32) -> u32 {
        u32::from(tk::menu_wshop_item_present(&mem(), i))
    }
    /// `MenuEquip_WindowIdle(window)`: the window (left in `eax`).
    MenuEquip_WindowIdle @ 0x008c_f950 => extern "C" fn menu_equip_window_idle(window: u32) -> u32 {
        tk::window_idle(&mut mem(), window)
    }
    /// `MenuLimit_WindowIdle(window)`: the window (left in `eax`).
    MenuLimit_WindowIdle @ 0x008d_0e80 => extern "C" fn menu_limit_window_idle(window: u32) -> u32 {
        tk::window_idle(&mut mem(), window)
    }
    /// `MenuKaizou_WindowIdle(window)`: the window (left in `eax`).
    MenuKaizou_WindowIdle @ 0x008d_56d0 => extern "C" fn menu_kaizou_window_idle(window: u32) -> u32 {
        tk::window_idle(&mut mem(), window)
    }
    /// `MenuSubCmd_CursorAtTop(window)`.
    MenuSubCmd_CursorAtTop @ 0x008e_4a90 => extern "C" fn menu_sub_cmd_cursor_at_top(window: u32) -> u32 {
        u32::from(tk::menu_sub_cmd_cursor_at_top(&mem(), window))
    }
    /// `MenuSubCmd_CenterX(window)`: the x (left in `eax`).
    MenuSubCmd_CenterX @ 0x008e_4ad0 => extern "C" fn menu_sub_cmd_center_x(window: u32) -> i32 {
        tk::menu_sub_cmd_center_x(&mut mem(), window)
    }
    /// `Gu_FontSetX(x)`: `x` (left in `eax`).
    Gu_FontSetX @ 0x0084_82f0 => extern "C" fn gu_font_set_x(x: i32) -> i32 {
        gu::PRINT_X.set(&mut mem(), x);
        x
    }
    /// `Gu_FontSetY(y)`: `y` (left in `eax`).
    Gu_FontSetY @ 0x0084_8300 => extern "C" fn gu_font_set_y(y: i32) -> i32 {
        gu::PRINT_Y.set(&mut mem(), y);
        y
    }
    /// `Gu_PrintSetCR(v)`: `v` (left in `eax`).
    Gu_PrintSetCR @ 0x0084_8730 => extern "C" fn gu_print_set_cr(v: u32) -> u32 {
        gu::PRINT_CR.set(&mut mem(), v);
        v
    }
    /// `Gu_MouseSetEnable(on)`: `on` (left in `eax`).
    Gu_MouseSetEnable @ 0x0084_9b80 => extern "C" fn gu_mouse_set_enable(on: u32) -> u32 {
        gu::MOUSE_ENABLE.set(&mut mem(), on);
        on
    }
    /// `Gu_WindowGetHScale(window)`.
    Gu_WindowGetHScale @ 0x0084_af20 => extern "C" fn gu_window_get_h_scale(window: u32) -> i32 {
        gu::gu_window_get_h_scale(&mem(), window)
    }
    /// `Gu_WindowSetHScale(window, scale)`: the window (left in `eax`).
    Gu_WindowSetHScale @ 0x0084_b5b0 => extern "C" fn gu_window_set_h_scale(window: u32, scale: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // a 16-bit store
        gu::gu_window_set_h_scale(&mut mem(), window, scale as u16)
    }
    /// `Gu_WindowSetWord30(window, v)`: the window (left in `eax`).
    Gu_WindowSetWord30 @ 0x0084_b7e0 => extern "C" fn gu_window_set_word30(window: u32, v: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // a 16-bit store
        gu::gu_window_set_word30(&mut mem(), window, v as u16)
    }
    /// `TkMn2GetAbilityMes(id)`: an auto-ability's name.
    TkMn2GetAbilityMes @ 0x008c_19a0 => extern "C" fn tk_mn2_get_ability_mes(id: u32) -> u32 {
        tk::tk_mn2_get_ability_mes(&mut mem(), id)
    }
    /// `TkMn2GetAbilityHelp(id)`: an auto-ability's help text.
    TkMn2GetAbilityHelp @ 0x008c_1960 => extern "C" fn tk_mn2_get_ability_help(id: u32) -> u32 {
        tk::tk_mn2_get_ability_help(&mut mem(), id)
    }
    /// `TkMcChdir(dir)`: 0 (`sceMcChdir` stub).
    TkMcChdir @ 0x0088_7fd0 => extern "C" fn tk_mc_chdir(_dir: u32) -> u32 {
        tk::tk_mc_call(&mut mem(), true)
    }
    /// `TkMcClose()`: 0 (stub).
    TkMcClose @ 0x0088_8010 => extern "C" fn tk_mc_close() -> u32 {
        tk::tk_mc_call(&mut mem(), false)
    }
    /// `TkMcDelete(name)`: 0 (stub).
    TkMcDelete @ 0x0088_8170 => extern "C" fn tk_mc_delete(_name: u32) -> u32 {
        tk::tk_mc_call(&mut mem(), true)
    }
    /// `TkMcFlush()`: 0 (stub).
    TkMcFlush @ 0x0088_81a0 => extern "C" fn tk_mc_flush() -> u32 {
        tk::tk_mc_call(&mut mem(), false)
    }
    /// `TkMcFormat()`: 0 (stub).
    TkMcFormat @ 0x0088_81c0 => extern "C" fn tk_mc_format() -> u32 {
        tk::tk_mc_call(&mut mem(), true)
    }
    /// `TkMcGetDir(name, mode, max, table)`: 0 (stub).
    TkMcGetDir @ 0x0088_8200 => extern "C" fn tk_mc_get_dir(_a: u32, _b: u32, _c: u32, _d: u32) -> u32 {
        tk::tk_mc_call(&mut mem(), true)
    }
    /// `TkMcGetEntSpace(path)`: 0 (stub).
    TkMcGetEntSpace @ 0x0088_8240 => extern "C" fn tk_mc_get_ent_space(_path: u32) -> u32 {
        tk::tk_mc_call(&mut mem(), true)
    }
    /// `TkMcGetInfo(port, type, free, format)`: 0 (stub).
    TkMcGetInfo @ 0x0088_82c0 => extern "C" fn tk_mc_get_info(port: u32, _a: u32, _b: u32, _c: u32) -> u32 {
        tk::tk_mc_get_info(&mut mem(), port)
    }
    /// `TkMcMkdir(name)`: 0 (stub).
    TkMcMkdir @ 0x0088_8330 => extern "C" fn tk_mc_mkdir(_name: u32) -> u32 {
        tk::tk_mc_call(&mut mem(), true)
    }
    /// `TkMcOpen(name, mode)`: 0 (stub).
    TkMcOpen @ 0x0088_8360 => extern "C" fn tk_mc_open(_name: u32, _mode: u32) -> u32 {
        tk::tk_mc_call(&mut mem(), true)
    }
    /// `TkMcRead()`: 0 (stub).
    TkMcRead @ 0x0088_83a0 => extern "C" fn tk_mc_read() -> u32 {
        tk::tk_mc_call(&mut mem(), false)
    }
    /// `TkMcRename(old, new)`: 0 (stub).
    TkMcRename @ 0x0088_83c0 => extern "C" fn tk_mc_rename(_old: u32, _new: u32) -> u32 {
        tk::tk_mc_call(&mut mem(), true)
    }
    /// `TkMcSeek()`: 0 (stub).
    TkMcSeek @ 0x0088_8400 => extern "C" fn tk_mc_seek() -> u32 {
        tk::tk_mc_call(&mut mem(), false)
    }
    /// `TkMcSetFileInfo(name, info, flags)`: 0 (stub).
    TkMcSetFileInfo @ 0x0088_8440 => extern "C" fn tk_mc_set_file_info(_a: u32, _b: u32, _c: u32) -> u32 {
        tk::tk_mc_call(&mut mem(), true)
    }
    /// `TkVU1SyncPath()`: 0 (`sceGsSyncPath` stub).
    TkVU1SyncPath @ 0x0088_ec60 => extern "C" fn tk_vu1_sync_path() -> u32 {
        0
    }
    /// `TkGetDebugFontBuffer()`.
    TkGetDebugFontBuffer @ 0x0088_edc0 => extern "C" fn tk_get_debug_font_buffer() -> u32 {
        tk::tk_get_debug_font_buffer(&mem())
    }
    /// `Math_CosTable(angle)`: `Math_SinTable(angle + 0x2000)`.
    Math_CosTable @ 0x0088_c120 => extern "C" fn math_cos_table(angle: u32) -> i32 {
        ffx_menu::math::bridge::math_sin_table(&mem(), angle.wrapping_add(0x2000))
    }
    /// `Math_SinTable2(angle)`: `Math_SinTable(angle)`.
    Math_SinTable2 @ 0x0088_c380 => extern "C" fn math_sin_table2(angle: u32) -> i32 {
        ffx_menu::math::bridge::math_sin_table(&mem(), angle)
    }
    /// `Math_SinLookup(x)`.
    Math_SinLookup @ 0x0088_e730 => extern "C" fn math_sin_lookup(x: i32) -> u32 {
        tk::math_sin_lookup(&mem(), x)
    }
    /// `Math_CosLookup(x)`.
    Math_CosLookup @ 0x0088_e750 => extern "C" fn math_cos_lookup(x: i32) -> u32 {
        tk::math_cos_lookup(&mem(), x)
    }
    /// `TkMn2GetExcelData(id, table)`: the record.
    TkMn2GetExcelData @ 0x008c_1af0 => extern "C" fn tk_mn2_get_excel_data(id: i32, table: u32) -> u32 {
        tk::tk_mn2_get_excel_data(&mem(), id, table)
    }
    /// `TkMn2GetKaizou()`: the first customize record.
    TkMn2GetKaizou @ 0x008c_1b70 => extern "C" fn tk_mn2_get_kaizou() -> u32 {
        tk::tk_mn2_get_kaizou(&mut mem())
    }
    /// `TkMn2GetKaizouMax()`.
    TkMn2GetKaizouMax @ 0x008c_1b90 => extern "C" fn tk_mn2_get_kaizou_max() -> i32 {
        tk::tk_mn2_get_kaizou_max(&mem())
    }
    /// `TkMn2InitWork()`.
    TkMn2InitWork @ 0x008c_1c50 => extern "C" fn tk_mn2_init_work() {
        tk::tk_mn2_init_work(&mut mem());
    }
    /// `MenuMn2_StepDrawY80(...)`: the new y (left in `eax`).
    MenuMn2_StepDrawY80 @ 0x008c_0dd0 => extern "C" fn menu_mn2_step_draw_y80() -> u32 {
        tk::menu_mn2_step_draw_y80(&mut mem())
    }
    /// `MenuItem_AddMP(chr, n)`: the record (left in `eax`).
    MenuItem_AddMP @ 0x008c_8e20 => extern "C" fn menu_item_add_mp(chr: u32, n: u32) -> u32 {
        tk::menu_item_add_mp(&mut mem(), chr, n)
    }
    /// `MenuParty_RefreshMask()`.
    MenuParty_RefreshMask @ 0x008a_ae30 => extern "C" fn menu_party_refresh_mask() {
        tk::menu_party_refresh_mask(&mut mem());
    }
    /// `TkRefreshActuator()`: the pad work (left in `eax`).
    TkRefreshActuator @ 0x0087_2480 => extern "C" fn pad_refresh_actuator_mode() -> u32 {
        tk::pad_refresh_actuator_mode(&mut mem())
    }
    /// `Pad_GetFrame(pad, port, i)`.
    Pad_GetFrame @ 0x0088_8b80 => extern "C" fn pad_get_frame(pad: i32, port: i32, i: i32) -> u32 {
        tk::pad_get_frame(&mem(), pad, port, i)
    }
    /// `BtlEnd_GetNextAP(chr)`: `MsGetNextAP`.
    BtlEnd_GetNextAP @ 0x008b_da40 => extern "C" fn btl_end_get_next_ap(chr: i32) -> i32 {
        ffx_battle::menu::bridge::ms_get_next_ap(&mut mem(), chr)
    }
    /// `TkMenuGetPlayerDamegedColor(chr)`: `MenuPlayer_DamageClass`.
    TkMenuGetPlayerDamegedColor @ 0x008b_9eb0 => extern "C" fn tk_menu_get_player_dameged_color(chr: u32) -> u32 {
        tk::menu_player_damage_class(&mem(), chr, true)
    }
    /// `MenuConfig_SetBit9(option)`: the setting's address (left in `eax`).
    MenuConfig_SetBit9 @ 0x008c_adb0 => extern "C" fn menu_config_set_bit9(option: u32) -> u32 {
        tk::menu_config_set_bit9(&mut mem(), option)
    }
    /// `MenuConfig_GetBit5(option)`: 0 or -1 (left in `eax`).
    MenuConfig_GetBit5 @ 0x008c_ade0 => extern "C" fn menu_config_get_bit5(option: u32) -> u32 {
        tk::menu_config_get_bit5(&mut mem(), option)
    }
    /// `MenuConfig_SetBit5(option)`: the setting's address.
    MenuConfig_SetBit5 @ 0x008c_ae20 => extern "C" fn menu_config_set_bit5(option: u32) -> u32 {
        tk::menu_config_set_bits(&mut mem(), option, 0x20, true)
    }
    /// `MenuConfig_GetBit1(option)`: the option.
    MenuConfig_GetBit1 @ 0x008c_ae40 => extern "C" fn menu_config_get_bit1(option: u32) -> u32 {
        tk::menu_config_get_bit(&mut mem(), option, 1, false)
    }
    /// `MenuConfig_SetBit1(option)`: the setting's address.
    MenuConfig_SetBit1 @ 0x008c_ae60 => extern "C" fn menu_config_set_bit1(option: u32) -> u32 {
        tk::menu_config_set_bits(&mut mem(), option, 2, false)
    }
    /// `MenuConfig_GetBit10(option)`: the option.
    MenuConfig_GetBit10 @ 0x008c_ae80 => extern "C" fn menu_config_get_bit10(option: u32) -> u32 {
        tk::menu_config_get_bit(&mut mem(), option, 10, true)
    }
    /// `MenuConfig_SetBit10(option)`: the setting's address.
    MenuConfig_SetBit10 @ 0x008c_aea0 => extern "C" fn menu_config_set_bit10(option: u32) -> u32 {
        tk::menu_config_set_bits(&mut mem(), option, 0x400, true)
    }
    /// `MenuConfig_GetBit11(option)`: the option.
    MenuConfig_GetBit11 @ 0x008c_aed0 => extern "C" fn menu_config_get_bit11(option: u32) -> u32 {
        tk::menu_config_get_bit(&mut mem(), option, 11, true)
    }
    /// `MenuConfig_SetBit11(option)`: the setting's address.
    MenuConfig_SetBit11 @ 0x008c_aef0 => extern "C" fn menu_config_set_bit11(option: u32) -> u32 {
        tk::menu_config_set_bits(&mut mem(), option, 0x800, true)
    }
    /// `MenuConfig_GetBit6(option)`: the option.
    MenuConfig_GetBit6 @ 0x008c_af20 => extern "C" fn menu_config_get_bit6(option: u32) -> u32 {
        tk::menu_config_get_bit(&mut mem(), option, 6, true)
    }
    /// `MenuConfig_SetBit6(option)`: the setting's address.
    MenuConfig_SetBit6 @ 0x008c_af40 => extern "C" fn menu_config_set_bit6(option: u32) -> u32 {
        tk::menu_config_set_bits(&mut mem(), option, 0x40, true)
    }
    /// `MenuConfig_GetEnglish(option)`: 1 (left in `eax`).
    MenuConfig_GetEnglish @ 0x008c_afd0 => extern "C" fn menu_config_get_english(option: u32) -> u32 {
        tk::menu_config_get_english(&mut mem(), option)
    }
    /// `MenuConfig_SetEnglish(option)`.
    MenuConfig_SetEnglish @ 0x008c_aff0 => extern "C" fn menu_config_set_english(option: u32) {
        tk::menu_config_set_english(&mut mem(), option);
    }
    /// `MenuConfig_GetSaveFlag(option)`: the option.
    MenuConfig_GetSaveFlag @ 0x008c_b090 => extern "C" fn menu_config_get_save_flag(option: u32) -> u32 {
        tk::menu_config_get_save_flag(&mut mem(), option)
    }
    /// `MenuConfig_GetBit8(option)`: the value (left in `eax`).
    MenuConfig_GetBit8 @ 0x008c_b490 => extern "C" fn menu_config_get_bit8(option: u32) -> u32 {
        tk::menu_config_get_bit8(&mut mem(), option)
    }
    /// `MenuConfig_GetBit9(option)`: the value (left in `eax`).
    MenuConfig_GetBit9 @ 0x008c_b500 => extern "C" fn menu_config_get_bit9(option: u32) -> u32 {
        tk::menu_config_get_bit9(&mut mem(), option)
    }
    /// `Vibrate_ResetStream(stream)`: 0.
    Vibrate_ResetStream @ 0x0087_5f30 => extern "C" fn vibrate_reset_stream(stream: u32) -> u32 {
        tk::vibrate_reset_stream(&mut mem(), stream)
    }
    /// `Vibrate_ResetPad(pad)`: 0.
    Vibrate_ResetPad @ 0x0087_5ef0 => extern "C" fn vibrate_reset_pad(pad: u32) -> u32 {
        tk::vibrate_reset_pad(&mut mem(), pad)
    }
    /// `Vibrate_HoldStream(stream)`.
    Vibrate_HoldStream @ 0x0087_6160 => extern "C" fn vibrate_hold_stream(stream: u32) -> u32 {
        tk::vibrate_hold_stream(&mut mem(), stream)
    }
    /// `Vibrate_StartStream(stream, curve)`: the data pointer.
    Vibrate_StartStream @ 0x0087_6350 => extern "C" fn vibrate_start_stream(stream: u32, curve: u32) -> u32 {
        tk::vibrate_start_stream(&mut mem(), stream, curve)
    }
    /// `Vibrate_CheckDone(pad)`: `pad`.
    Vibrate_CheckDone @ 0x0087_5d30 => extern "C" fn vibrate_check_done(pad: u32) -> u32 {
        tk::vibrate_check_done(&mut mem(), pad)
    }
    /// `MenuItem_IsUsable2(window, list)`.
    MenuItem_IsUsable2 @ 0x008c_3550 => extern "C" fn menu_item_is_usable2(_window: u32, list: i32) -> u32 {
        tk::menu_item_is_usable2(&mem(), list)
    }
    /// `MenuEquip_PickItem()`: the id read.
    MenuEquip_PickItem @ 0x008d_0430 => extern "C" fn menu_equip_pick_item() -> u32 {
        tk::menu_equip_pick_item(&mut mem())
    }
    /// `MenuSaveLoad_Reset(v)`: the second window.
    MenuSaveLoad_Reset @ 0x008b_4e70 => extern "C" fn menu_save_load_reset(v: u32) -> u32 {
        tk::menu_save_load_reset(&mut mem(), v)
    }
    /// `Gu_Approach(cur, target, step)`.
    Gu_Approach @ 0x0084_9270 => extern "C" fn gu_approach(cur: i32, target: i32, step: i32) -> i32 {
        ffx_menu::gu::approach(cur, target, step)
    }
    /// `Gu_PrintSetMetrics(size)`: the ×4 metric.
    Gu_PrintSetMetrics @ 0x0084_87e0 => extern "C" fn gu_print_set_metrics(size: i32) -> i32 {
        gu::gu_print_set_metrics(&mut mem(), size)
    }
    /// `Gu_WindowGetWord8C(window)`.
    Gu_WindowGetWord8C @ 0x0084_af60 => extern "C" fn gu_window_get_word8c(window: u32) -> i32 {
        gu::gu_window_get_word8c(&mem(), window)
    }
    /// `Table_PairWord(table, i, second)`.
    Table_PairWord @ 0x0087_25c0 => extern "C" fn table_pair_word(table: u32, i: u32, second: u32) -> u32 {
        tk::table_pair_word(&mem(), table, i, second != 0)
    }
    /// `Ref_Deref(ref)`.
    Ref_Deref @ 0x0087_25a0 => extern "C" fn ref_deref(r: u32) -> u32 {
        tk::ref_deref(&mem(), r)
    }
    /// `MenuStatus_ResetEntry(entry)`: 0.
    MenuStatus_ResetEntry @ 0x008d_ea20 => extern "C" fn menu_status_reset_entry(entry: u32) -> u32 {
        tk::menu_status_reset_entry(&mut mem(), entry)
    }
    /// `MenuHdd_WindowReset(window)`: the window.
    MenuHdd_WindowReset @ 0x008e_28e0 => extern "C" fn menu_hdd_window_reset(window: u32) -> u32 {
        tk::window_clear_1c(&mut mem(), window)
    }
    /// `MenuMain_GetTask()`.
    MenuMain_GetTask @ 0x008a_9b90 => extern "C" fn menu_main_get_task() -> u32 {
        tk::MENU_MAIN_TASK.get(&mem())
    }
    /// `TkGetControlerHist(pad, port, i)`: buttons of frame `i`.
    TkGetControlerHist @ 0x0088_8d70 => extern "C" fn tk_get_controler_hist(pad: i32, port: i32, i: i32) -> u32 {
        tk::tk_get_controler_hist(&mem(), pad, port, i, 4)
    }
    /// `TkGetControlerTrigHist(pad, port, i)`: triggered buttons.
    TkGetControlerTrigHist @ 0x0088_8e80 => extern "C" fn tk_get_controler_trig_hist(pad: i32, port: i32, i: i32) -> u32 {
        tk::tk_get_controler_hist(&mem(), pad, port, i, 6)
    }
    /// `TkGetControlerTrigRHist(pad, port, i)`: released buttons.
    TkGetControlerTrigRHist @ 0x0088_8ea0 => extern "C" fn tk_get_controler_trig_r_hist(pad: i32, port: i32, i: i32) -> u32 {
        tk::tk_get_controler_hist(&mem(), pad, port, i, 8)
    }
    /// `TkGetControlerReptHist(pad, port, i)`: repeating buttons.
    TkGetControlerReptHist @ 0x0088_8e50 => extern "C" fn tk_get_controler_rept_hist(pad: i32, port: i32, i: i32) -> u32 {
        tk::tk_get_controler_hist(&mem(), pad, port, i, 0xa)
    }
    /// `TkGetControlerRept2Hist(pad, port, i)`: the second repeat set.
    TkGetControlerRept2Hist @ 0x0088_8e30 => extern "C" fn tk_get_controler_rept2_hist(pad: i32, port: i32, i: i32) -> u32 {
        tk::tk_get_controler_hist(&mem(), pad, port, i, 0x1c)
    }
    /// `TkGetControlerAnalogHist(pad, port, k, i)`.
    TkGetControlerAnalogHist @ 0x0088_8c20 => extern "C" fn tk_get_controler_analog_hist(pad: i32, port: i32, k: u32, i: i32) -> u32 {
        tk::tk_get_controler_analog_hist(&mem(), pad, port, k, i)
    }
    /// `TkGetControlerActuatorNumber(pad, port)`.
    TkGetControlerActuatorNumber @ 0x0088_8bc0 => extern "C" fn tk_get_controler_actuator_number(pad: i32, port: i32) -> u32 {
        tk::tk_get_controler_actuator_number(&mem(), pad, port)
    }
    /// `MenuAbility_RefreshList(chr)`: the rows.
    MenuAbility_RefreshList @ 0x008c_6800 => extern "C" fn menu_ability_refresh_list(chr: u32) -> i32 {
        tk::menu_ability_refresh_list(&mut mem(), chr)
    }
    /// `MenuMain_ResetWindow(window)`.
    MenuMain_ResetWindow @ 0x008a_a4b0 => extern "C" fn menu_main_reset_window(window: u32) {
        tk::menu_main_reset_window(&mut mem(), window);
    }
    /// `MenuItem_CompareEntries(list, a, b)`.
    MenuItem_CompareEntries @ 0x008c_9410 => extern "C" fn menu_item_compare_entries(list: u32, a: u32, b: u32) -> i32 {
        tk::menu_item_compare_entries(&mem(), list, a, b)
    }
    /// `MenuItem_SwapEntries(list, a, b)`.
    MenuItem_SwapEntries @ 0x008c_9460 => extern "C" fn menu_item_swap_entries(list: u32, a: u32, b: u32) {
        tk::menu_item_swap_entries(&mut mem(), list, a, b);
    }
    /// `MenuWShop_InitCursor(window)`.
    MenuWShop_InitCursor @ 0x008d_9580 => extern "C" fn menu_wshop_init_cursor(window: u32) {
        tk::menu_wshop_init_cursor(&mut mem(), window);
    }
    /// `MenuSaveLoad_MarkSlots(free, need)`.
    MenuSaveLoad_MarkSlots @ 0x008b_54c0 => extern "C" fn menu_save_load_mark_slots(free: i32, need: i32) {
        tk::menu_save_load_mark_slots(&mut mem(), free, need);
    }
    /// `MenuWShop_CountSlots()`.
    MenuWShop_CountSlots @ 0x008d_b440 => extern "C" fn menu_wshop_count_slots() {
        tk::menu_wshop_count_slots(&mut mem());
    }
    /// `BtlEnd_InitGilStep()`.
    BtlEnd_InitGilStep @ 0x008b_df10 => extern "C" fn btl_end_init_gil_step() {
        tk::btl_end_init_gil_step(&mut mem());
    }
    /// `BtlEnd_ApStep(chr)`.
    BtlEnd_ApStep @ 0x008b_a3c0 => extern "C" fn btl_end_ap_step(chr: i32) -> u32 {
        tk::btl_end_ap_step_for(&mut mem(), chr)
    }
    /// `BtlEnd_CompactList()`.
    BtlEnd_CompactList @ 0x008b_a400 => extern "C" fn btl_end_compact_list() {
        tk::btl_end_compact_list(&mut mem());
    }
    /// `MenuMn2_Ease(t, dir)`.
    MenuMn2_Ease @ 0x008d_30c0 => extern "C" fn menu_mn2_ease(t: i32, dir: i32) -> u32 {
        tk::menu_mn2_ease(&mem(), t, dir)
    }
    /// `MenuStatus_OffsetRect(dx, dy, out, rect, sized)`.
    MenuStatus_OffsetRect @ 0x008d_3220 => extern "C" fn menu_status_offset_rect(dx: u32, dy: u32, out: u32, rect: u32, sized: u32) {
        #[allow(clippy::cast_possible_truncation)] // 16-bit adds
        tk::menu_status_offset_rect(&mut mem(), dx as u16, dy as u16, out, rect, sized != 0);
    }
    /// `MenuStatus_GetPos(out, alt)`.
    MenuStatus_GetPos @ 0x008d_4e30 => extern "C" fn menu_status_get_pos(out: u32, alt: u32) {
        tk::menu_status_get_pos(&mut mem(), out, alt != 0, 0x0186_a900, 0x0186_a948);
    }
    /// `MenuStatus_GetPos2(out, alt)`.
    MenuStatus_GetPos2 @ 0x008d_4e80 => extern "C" fn menu_status_get_pos2(out: u32, alt: u32) {
        tk::menu_status_get_pos(&mut mem(), out, alt != 0, 0x0186_a918, 0x0186_a960);
    }
    /// `MenuHdd_Close()`.
    MenuHdd_Close @ 0x008e_2930 => extern "C" fn menu_hdd_close() {
        tk::menu_hdd_close(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuSaveLoad_BuildCrcTable(table)`.
    MenuSaveLoad_BuildCrcTable @ 0x008b_1370 => extern "C" fn menu_save_load_build_crc_table(table: u32) {
        tk::menu_save_load_build_crc_table(&mut mem(), table);
    }
    /// `MenuDraw_GifTag(packet, a, b, c)`: the end of the tag.
    MenuDraw_GifTag @ 0x008d_e0a0 => extern "C" fn menu_draw_gif_tag(p: u32, a: u32, b: u32, c: u32) -> u32 {
        tk::menu_draw_gif_tag(&mut mem(), p, a, b, c)
    }
    /// `TkMenuDrawInitString()`.
    TkMenuDrawInitString @ 0x008d_ce30 => extern "C" fn tk_menu_draw_init_string() -> u32 {
        tk::tk_menu_draw_init_string(&mut mem())
    }
    /// `TkMn2DrawMenuWindowScissor(y, h, mode, ratio)`: the bottom.
    TkMn2DrawMenuWindowScissor @ 0x008c_0f70 => extern "C" fn tk_mn2_draw_menu_window_scissor(y: i32, h: i32, mode: i32, ratio: i32) -> u32 {
        tk::tk_mn2_draw_menu_window_scissor(&mut mem(), y, h, mode, ratio)
    }
    /// `TkMn2DrawScissorClear()`: the full-screen scissor.
    TkMn2DrawScissorClear @ 0x008c_1370 => extern "C" fn tk_mn2_draw_scissor_clear() -> u32 {
        ffx_system::pkt::bridge::draw_scissor_xywh(&mut mem(), 0, 0, 0x200, 0x1a0)
    }
    /// `TkMenuDrawScissorClear()`: the full-screen scissor.
    TkMenuDrawScissorClear @ 0x008d_d440 => extern "C" fn tk_menu_draw_scissor_clear() -> u32 {
        ffx_system::pkt::bridge::draw_scissor_xywh(&mut mem(), 0, 0, 0x200, 0x1a0)
    }
    /// `MenuWShop_GetRecord(id)`.
    MenuWShop_GetRecord @ 0x008d_9170 => extern "C" fn menu_wshop_get_record(id: u32) -> u32 {
        tk::menu_wshop_get_record(&mem(), id)
    }
    /// `Msg_CloseWindow(window)`.
    Msg_CloseWindow @ 0x0086_40f0 => extern "C" fn msg_close_window(window: i32) {
        msg::msg_close_window(&mut mem(), window, |m, i| {
            ffx_system::window::bridge::close_message_window(m, i);
        });
    }
    /// `TkMenuMainFreeWindow(window)`.
    TkMenuMainFreeWindow @ 0x008a_a3f0 => extern "C" fn tk_menu_main_free_window(window: u32) {
        tk::tk_menu_main_free_window(&mut mem(), window);
    }
    /// `MenuMain_ResetWindowRet(window)`: the window (left in `eax`).
    MenuMain_ResetWindowRet @ 0x008a_a180 => extern "C" fn menu_main_reset_window_ret(window: u32) -> u32 {
        tk::menu_main_reset_window(&mut mem(), window);
        window
    }
    /// `MenuMain_ResetAllWindows()`.
    MenuMain_ResetAllWindows @ 0x008a_a470 => extern "C" fn menu_main_reset_all_windows() {
        tk::menu_main_reset_all_windows(&mut mem());
    }
    /// `TkMenuDrawKick()`: the packet pointer (left in `eax`).
    TkMenuDrawKick @ 0x008d_ce50 => extern "C" fn tk_menu_draw_kick() -> u32 {
        tk::tk_menu_draw_kick(&mut mem())
    }
    /// `Msg_CancelChoice(worker, slot)`.
    Msg_CancelChoice @ 0x0086_4150 => extern "C" fn msg_cancel_choice(worker: i32, slot: i32) {
        tk::msg_cancel_choice(&mut mem(), worker, slot);
    }
    /// `TkSetDebugFontBuffer(p)`: the buffer end (left in `eax`).
    TkSetDebugFontBuffer @ 0x0088_fab0 => extern "C" fn tk_set_debug_font_buffer(p: u32) -> u32 {
        tk::tk_set_debug_font_buffer(&mut mem(), p)
    }
    /// `Obj_GetFloat28(p)`.
    Obj_GetFloat28 @ 0x0087_2590 => extern "C" fn obj_get_float28(p: u32) -> f32 {
        tk::obj_get_float28(&mem(), p)
    }
    /// `Tk_GetScale(which)`.
    Tk_GetScale @ 0x0087_2360 => extern "C" fn tk_get_scale(which: u32) -> f32 {
        tk::tk_get_scale(&mem(), which)
    }
    /// `Tk_SetScale(which, v)`.
    Tk_SetScale @ 0x0087_2530 => extern "C" fn tk_set_scale(which: u32, v: f32) {
        tk::tk_set_scale(&mut mem(), which, v);
    }
    /// `Math_AngleFrom12Bits(a)`.
    Math_AngleFrom12Bits @ 0x0088_c0a0 => extern "C" fn math_angle_from_12_bits(a: u32) -> f32 {
        tk::math_angle_from_bits(&mem(), a, 0xfff, 0x00b5_15c8)
    }
    /// `Math_AngleFrom15Bits(a)`.
    Math_AngleFrom15Bits @ 0x0088_c0e0 => extern "C" fn math_angle_from_15_bits(a: u32) -> f32 {
        tk::math_angle_from_bits(&mem(), a, 0x7fff, 0x00b5_1588)
    }
    /// `Gs_Pack4(a, b, c, d)`.
    Gs_Pack4 @ 0x008d_e1b0 => extern "C" fn gs_pack4(a: u32, b: u32, c: u32, d: u32) -> u64 {
        ffx_menu::tk::gs_pack4(a, b, c, d)
    }
    /// `Gs_Pack4b(a, b, c, d)`: a copy.
    Gs_Pack4b @ 0x008c_32e0 => extern "C" fn gs_pack4b(a: u32, b: u32, c: u32, d: u32) -> u64 {
        ffx_menu::tk::gs_pack4(a, b, c, d)
    }
    /// `Gs_Pack5(a, b, c, d, e)`.
    Gs_Pack5 @ 0x008d_e1d0 => extern "C" fn gs_pack5(a: u32, b: u32, c: u32, d: u32, e: u32) -> u64 {
        ffx_menu::tk::gs_pack5(a, b, c, d, e)
    }
    /// `Gs_Pack5b(a, b, c, d, e)`: a copy.
    Gs_Pack5b @ 0x008c_3300 => extern "C" fn gs_pack5b(a: u32, b: u32, c: u32, d: u32, e: u32) -> u64 {
        ffx_menu::tk::gs_pack5(a, b, c, d, e)
    }
    /// `Gs_Pack5c(a, b, c, d, e)`: a copy.
    Gs_Pack5c @ 0x0089_0a30 => extern "C" fn gs_pack5c(a: u32, b: u32, c: u32, d: u32, e: u32) -> u64 {
        ffx_menu::tk::gs_pack5(a, b, c, d, e)
    }
    /// `Gs_Pack5d(a, b, c, d, e)`: a copy.
    Gs_Pack5d @ 0x008c_ad80 => extern "C" fn gs_pack5d(a: u32, b: u32, c: u32, d: u32, e: u32) -> u64 {
        ffx_menu::tk::gs_pack5(a, b, c, d, e)
    }
    /// `MenuStatus_Plus10(p)`: `p + 0x10`.
    MenuStatus_Plus10 @ 0x008d_e1a0 => extern "C" fn menu_status_plus10(p: u32) -> u32 {
        p.wrapping_add(0x10)
    }
    /// `MenuItem_Plus10(p)`: `p + 0x10`.
    MenuItem_Plus10 @ 0x008c_32d0 => extern "C" fn menu_item_plus10(p: u32) -> u32 {
        p.wrapping_add(0x10)
    }
    /// `MenuHelp_Plus40(p)`: `p + 0x40`.
    MenuHelp_Plus40 @ 0x008b_f140 => extern "C" fn menu_help_plus40(p: u32) -> u32 {
        p.wrapping_add(0x40)
    }
    /// `MenuStatus_Second(a, b)`: `b`.
    MenuStatus_Second @ 0x008d_e200 => extern "C" fn menu_status_second(_a: u32, b: u32) -> u32 {
        b
    }
    /// `MenuItem_Second(a, b)`: `b`.
    MenuItem_Second @ 0x008c_3330 => extern "C" fn menu_item_second(_a: u32, b: u32) -> u32 {
        b
    }
    /// `Gu_WindowStartDrag(window)`.
    Gu_WindowStartDrag @ 0x0084_a580 => extern "C" fn gu_window_start_drag(window: u32) {
        gu::gu_window_start_drag(&mut mem(), window);
    }
    /// `Gu_WindowCloseTree(window)`.
    Gu_WindowCloseTree @ 0x0084_ab80 => extern "C" fn gu_window_close_tree(window: u32) {
        gu::gu_window_close_tree(&mut mem(), window, &mut |_, f, a| call_ptr(f, a));
    }
    /// `Gu_ListCopyNext(node)`.
    Gu_ListCopyNext @ 0x0084_a600 => extern "C" fn gu_list_copy_next(node: u32) {
        gu::gu_list_copy_next(&mut mem(), node);
    }
    /// `Gu_WindowMouseIn(window)`.
    Gu_WindowMouseIn @ 0x0084_9490 => extern "C" fn gu_window_mouse_in(window: u32) -> u32 {
        gu::gu_window_mouse_in(&mut mem(), window)
    }
    /// `Gu_WindowControlSetLeftText(window, id, on)`.
    Gu_WindowControlSetLeftText @ 0x0084_a6c0 => extern "C" fn gu_window_control_set_left_text(window: u32, id: u32, on: u32) {
        #[allow(clippy::cast_possible_truncation)] // a 16-bit compare
        gu::gu_window_control_set_left_text(&mut mem(), window, id as u16, on);
    }
    /// `Gu_WindowControlSetRect(window, id, x, y, w, h)`.
    Gu_WindowControlSetRect @ 0x0084_96d0 => extern "C" fn gu_window_control_set_rect(window: u32, id: u32, x: i32, y: i32, w: i32, h: i32) {
        gu::gu_window_control_set_rect(&mut mem(), window, id, [x, y, w, h]);
    }
    /// `Gu_MouseSetWindow(window)`.
    Gu_MouseSetWindow @ 0x0084_9b90 => extern "C" fn gu_mouse_set_window(window: u32) -> i32 {
        gu::gu_mouse_set_window(&mut mem(), window)
    }
    /// `MenuDraw_Sp3RBody(p, _, x, y, entry, ox, oy, alpha)`: the packet end.
    MenuDraw_Sp3RBody @ 0x008d_c1b0 => extern "C" fn menu_draw_sp3_r_body(p: u32, z: u32, x: u32, y: u32, entry: u32, ox: u32, oy: u32, alpha: u32) -> u32 {
        tk::menu_draw_sprites(&mut mem(), [p, z, x, y, entry, ox, oy, alpha], false)
    }
    /// `MenuDraw_Sp3UBody(p, _, x, y, entry, ox, oy, alpha)`: the packet end.
    MenuDraw_Sp3UBody @ 0x008d_c710 => extern "C" fn menu_draw_sp3_u_body(p: u32, z: u32, x: u32, y: u32, entry: u32, ox: u32, oy: u32, alpha: u32) -> u32 {
        tk::menu_draw_sprites(&mut mem(), [p, z, x, y, entry, ox, oy, alpha], true)
    }
    /// `MenuDraw_Bar(p, n, cell, x, y)`: the packet end.
    MenuDraw_Bar @ 0x008d_c360 => extern "C" fn menu_draw_bar(p: u32, n: i32, c: u32, x: i32, y: i32) -> u32 {
        tk::menu_draw_bar(&mut mem(), 0, p, n, 0, c, x, y)
    }
    /// `MenuDraw_BarV(p, n, cell, x, y)`: the packet end.
    MenuDraw_BarV @ 0x008d_c480 => extern "C" fn menu_draw_bar_v(p: u32, n: i32, c: u32, x: i32, y: i32) -> u32 {
        tk::menu_draw_bar(&mut mem(), 1, p, n, 0, c, x, y)
    }
    /// `MenuDraw_Bar2(p, m, n, cell, x, y)`: the packet end.
    MenuDraw_Bar2 @ 0x008d_c5a0 => extern "C" fn menu_draw_bar2(p: u32, m: i32, n: i32, c: u32, x: i32, y: i32) -> u32 {
        tk::menu_draw_bar(&mut mem(), 2, p, n, m, c, x, y)
    }
    /// `MenuDraw_Line(p, x, y, w, a, b)`: the packet end.
    MenuDraw_Line @ 0x008c_a580 => extern "C" fn menu_draw_line(p: u32, x: i32, y: i32, w: i32, a: i32, b: i32) -> u32 {
        tk::menu_draw_line(&mut mem(), p, [x, y, w, a, b])
    }
    /// `MenuDraw_Strip(p, x, y, xs, n, dx, tag)`: the packet end.
    MenuDraw_Strip @ 0x008c_a610 => extern "C" fn menu_draw_strip(p: u32, x: u32, y: u32, xs: u32, n: u32, dx: u32, tag: u32) -> u32 {
        tk::menu_draw_strip(&mut mem(), p, [x, y, xs, n, dx, tag], false)
    }
    /// `MenuDraw_StripR(p, x, y, xs, n, dx, tag)`: the packet end.
    MenuDraw_StripR @ 0x008c_a6f0 => extern "C" fn menu_draw_strip_r(p: u32, x: u32, y: u32, xs: u32, n: u32, dx: u32, tag: u32) -> u32 {
        tk::menu_draw_strip(&mut mem(), p, [x, y, xs, n, dx, tag], true)
    }
    /// `TkMenuMainOpenOption(i)`: the option's menu.
    TkMenuMainOpenOption @ 0x008e_24c0 => extern "C" fn tk_menu_main_open_option(i: i32) -> u32 {
        tk::tk_menu_main_open_option(&mut mem(), i, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuDraw_Sprite(p, x, y, entry)`: the packet end.
    MenuDraw_Sprite @ 0x008d_c0f0 => extern "C" fn menu_draw_sprite(p: u32, x: u32, y: u32, entry: u32) -> u32 {
        tk::menu_draw_sprites(&mut mem(), [p, 0, x, y, entry, 0, 0, 0x80], false)
    }
    /// `MenuDraw_SpriteA(p, x, y, entry, alpha)`: the packet end.
    MenuDraw_SpriteA @ 0x008d_c120 => extern "C" fn menu_draw_sprite_a(p: u32, x: u32, y: u32, entry: u32, alpha: u32) -> u32 {
        tk::menu_draw_sprites(&mut mem(), [p, 0, x, y, entry, 0, 0, alpha], false)
    }
    /// `MenuDraw_Sprite1(p, x, y, entry)`: the packet end.
    MenuDraw_Sprite1 @ 0x008d_c150 => extern "C" fn menu_draw_sprite1(p: u32, x: u32, y: u32, entry: u32) -> u32 {
        tk::menu_draw_sprites(&mut mem(), [p, 1, x, y, entry, 0, 0, 0x80], false)
    }
    /// `MenuDraw_Sprite2(p, x, y, entry)`: the packet end.
    MenuDraw_Sprite2 @ 0x008d_c180 => extern "C" fn menu_draw_sprite2(p: u32, x: u32, y: u32, entry: u32) -> u32 {
        tk::menu_draw_sprites(&mut mem(), [p, 2, x, y, entry, 0, 0, 0x80], false)
    }
    /// `MenuDraw_SpriteV(p, x, y, entry)`: the packet end.
    MenuDraw_SpriteV @ 0x008d_c6b0 => extern "C" fn menu_draw_sprite_v(p: u32, x: u32, y: u32, entry: u32) -> u32 {
        tk::menu_draw_sprites(&mut mem(), [p, 0, x, y, entry, 0, 0, 0x80], true)
    }
    /// `MenuDraw_SpriteV8(p, x, y, entry)`: the packet end.
    MenuDraw_SpriteV8 @ 0x008d_c6e0 => extern "C" fn menu_draw_sprite_v8(p: u32, x: u32, y: u32, entry: u32) -> u32 {
        tk::menu_draw_sprites(&mut mem(), [p, 0, x, y, entry, 8, 0, 0x80], true)
    }
    /// `MenuDraw_SpriteVA(p, x, y, entry, alpha)`: the packet end.
    MenuDraw_SpriteVA @ 0x008d_bdd0 => extern "C" fn menu_draw_sprite_v_a(p: u32, x: u32, y: u32, entry: u32, alpha: u32) -> u32 {
        tk::menu_draw_sprites(&mut mem(), [p, 0, x, y, entry, 0, 0, alpha], true)
    }
    /// `MenuDraw_SpriteV8A(p, x, y, entry, alpha)`: the packet end.
    MenuDraw_SpriteV8A @ 0x008d_be00 => extern "C" fn menu_draw_sprite_v8_a(p: u32, x: u32, y: u32, entry: u32, alpha: u32) -> u32 {
        tk::menu_draw_sprites(&mut mem(), [p, 0, x, y, entry, 8, 0, alpha], true)
    }
    /// `TkMenuDrawSp3R(x, y, entry)`: a new packet at `g_TkMenuPacket + 0x10`; the menu packet and the result are its end.
    TkMenuDrawSp3R @ 0x008d_d7d0 => extern "C" fn tk_menu_draw_sp3_r(x: u32, y: u32, entry: u32) -> u32 {
        let mut m = mem();
        let p = tk::MENU_PACKET.get(&m).wrapping_add(0x10);
        let end = tk::menu_draw_sprites(&mut m, [p, 0, x, y, entry, 0, 0, 0x80], false);
        tk::MENU_PACKET.set(&mut m, end);
        end
    }
    /// `TkMenuDrawSp3RA(x, y, entry, alpha)`: a new packet at `g_TkMenuPacket + 0x10`; the menu packet and the result are its end.
    TkMenuDrawSp3RA @ 0x008d_d800 => extern "C" fn tk_menu_draw_sp3_ra(x: u32, y: u32, entry: u32, alpha: u32) -> u32 {
        let mut m = mem();
        let p = tk::MENU_PACKET.get(&m).wrapping_add(0x10);
        let end = tk::menu_draw_sprites(&mut m, [p, 0, x, y, entry, 0, 0, alpha], false);
        tk::MENU_PACKET.set(&mut m, end);
        end
    }
    /// `TkMenuDrawSp3StretchD(cell, x, y, n)`: a new packet at `g_TkMenuPacket + 0x10`; the menu packet and the result are its end.
    TkMenuDrawSp3StretchD @ 0x008d_d830 => extern "C" fn tk_menu_draw_sp3_stretch_d(c: u32, x: i32, y: i32, n: i32) -> u32 {
        let mut m = mem();
        let p = tk::MENU_PACKET.get(&m).wrapping_add(0x10);
        let end = tk::menu_draw_bar(&mut m, 0, p, n, 0, c, x, y);
        tk::MENU_PACKET.set(&mut m, end);
        end
    }
    /// `TkMenuDrawSp3StretchR(cell, x, y, n)`: a new packet at `g_TkMenuPacket + 0x10`; the menu packet and the result are its end.
    TkMenuDrawSp3StretchR @ 0x008d_d860 => extern "C" fn tk_menu_draw_sp3_stretch_r(c: u32, x: i32, y: i32, n: i32) -> u32 {
        let mut m = mem();
        let p = tk::MENU_PACKET.get(&m).wrapping_add(0x10);
        let end = tk::menu_draw_bar(&mut m, 1, p, n, 0, c, x, y);
        tk::MENU_PACKET.set(&mut m, end);
        end
    }
    /// `TkMenuDrawSp3StretchRD(cell, x, y, m, n)`: a new packet at `g_TkMenuPacket + 0x10`; the menu packet and the result are its end.
    TkMenuDrawSp3StretchRD @ 0x008d_d890 => extern "C" fn tk_menu_draw_sp3_stretch_rd(c: u32, x: i32, y: i32, m2: i32, n: i32) -> u32 {
        let mut m = mem();
        let p = tk::MENU_PACKET.get(&m).wrapping_add(0x10);
        let end = tk::menu_draw_bar(&mut m, 2, p, n, m2, c, x, y);
        tk::MENU_PACKET.set(&mut m, end);
        end
    }
    /// `TkMenuDrawSp3Tex0(a, b, c)`: a new packet at `g_TkMenuPacket + 0x10`; the menu packet and the result are its end.
    TkMenuDrawSp3Tex0 @ 0x008d_d8c0 => extern "C" fn tk_menu_draw_sp3_tex0(a: u32, b: u32, c: u32) -> u32 {
        let mut m = mem();
        let p = tk::MENU_PACKET.get(&m).wrapping_add(0x10);
        let end = tk::menu_draw_gif_tag(&mut m, p, a, b, c);
        tk::MENU_PACKET.set(&mut m, end);
        end
    }
    /// `TkMenuDrawSp3UR(x, y, entry)`: a new packet at `g_TkMenuPacket + 0x10`; the menu packet and the result are its end.
    TkMenuDrawSp3UR @ 0x008d_d8f0 => extern "C" fn tk_menu_draw_sp3_ur(x: u32, y: u32, entry: u32) -> u32 {
        let mut m = mem();
        let p = tk::MENU_PACKET.get(&m).wrapping_add(0x10);
        let end = tk::menu_draw_sprites(&mut m, [p, 0, x, y, entry, 0, 0, 0x80], true);
        tk::MENU_PACKET.set(&mut m, end);
        end
    }
    /// `TkMenuDrawSp3URHalf(x, y, entry)`: a new packet at `g_TkMenuPacket + 0x10`; the menu packet and the result are its end.
    TkMenuDrawSp3URHalf @ 0x008d_d920 => extern "C" fn tk_menu_draw_sp3_ur_half(x: u32, y: u32, entry: u32) -> u32 {
        let mut m = mem();
        let p = tk::MENU_PACKET.get(&m).wrapping_add(0x10);
        let end = tk::menu_draw_sprites(&mut m, [p, 0, x, y, entry, 8, 0, 0x80], true);
        tk::MENU_PACKET.set(&mut m, end);
        end
    }
    /// `Gu_SetPopupWindow(w)`: `w` (left in `eax`).
    Gu_SetPopupWindow @ 0x0084_b670 => extern "C" fn gu_set_popup_window(w: u32) -> u32 {
        tk::set_word(&mut mem(), tk::GU_POPUP_WINDOW, w)
    }
    /// `Atel_SetScriptByteGate(v)`: `v` (left in `eax`).
    Atel_SetScriptByteGate @ 0x0087_20d0 => extern "C" fn atel_set_script_byte_gate(v: u32) -> u32 {
        tk::set_word(&mut mem(), tk::ATEL_SCRIPT_BYTE_GATE, v)
    }
    /// `TkSetVibrateOff(v)`: the low byte; `v` (`al` over it) in `eax`.
    TkSetVibrateOff @ 0x0087_5f50 => extern "C" fn tk_set_vibrate_off(v: u32) -> u32 {
        tk::set_byte(&mut mem(), tk::VIBRATE_OFF, v)
    }
    /// `AtelSetDebugDraw(v)`: the low byte; `v` in `eax`.
    AtelSetDebugDraw @ 0x0088_5110 => extern "C" fn atel_set_debug_draw(v: u32) -> u32 {
        tk::set_byte(&mut mem(), tk::ATEL_DEBUG_DRAW, v)
    }
    /// `TkConfig_SetByteC(v)`: the low byte; `v` in `eax`.
    TkConfig_SetByteC @ 0x0088_ec20 => extern "C" fn tk_config_set_byte_c(v: u32) -> u32 {
        tk::tk_config_set_byte_c(&mut mem(), v)
    }
    /// `TkMenuSetDrawShadeRatioStep(v)`: `v` (left in `eax`).
    TkMenuSetDrawShadeRatioStep @ 0x008a_9680 => extern "C" fn tk_menu_set_draw_shade_ratio_step(v: u32) -> u32 {
        tk::set_word(&mut mem(), tk::MENU_SHADE_RATIO_STEP, v)
    }
    /// `TkMenuSetScreenX2Flag(v)`: the low byte; `v` in `eax`.
    TkMenuSetScreenX2Flag @ 0x008a_b020 => extern "C" fn tk_menu_set_screen_x2_flag(v: u32) -> u32 {
        tk::set_byte(&mut mem(), tk::MENU_SCREEN_X2, v)
    }
    /// `Tk_ByteHighBit(b)`: `b & 0x80`.
    Tk_ByteHighBit @ 0x0087_2580 => extern "C" fn tk_byte_high_bit(b: u32) -> u32 {
        b & 0x80
    }
    /// `TkDebugPrint(fmt)`: `dbgPrintf` is an empty stub on PC; returns 1.
    TkDebugPrint @ 0x0088_ecf0 => extern "C" fn tk_debug_print(_fmt: u32) -> u32 {
        1
    }
    /// `TkGetDebugFontStringWidth(proportional, s)`.
    TkGetDebugFontStringWidth @ 0x0088_edf0 => extern "C" fn tk_get_debug_font_string_width(proportional: i32, s: u32) -> i32 {
        tk::tk_get_debug_font_string_width(&mem(), proportional, s)
    }
    /// `TkGetDebugControler1Hist(i)`: pad 0, zero-extended.
    TkGetDebugControler1Hist @ 0x0088_5150 => extern "C" fn tk_get_debug_controler1_hist(i: i32) -> u32 {
        tk::tk_get_debug_controler_hist(&mem(), 0, i, 4)
    }
    /// `TkGetDebugControler1TrigHist(i)`: pad 0, zero-extended.
    TkGetDebugControler1TrigHist @ 0x0088_5170 => extern "C" fn tk_get_debug_controler1_trig_hist(i: i32) -> u32 {
        tk::tk_get_debug_controler_hist(&mem(), 0, i, 6)
    }
    /// `TkGetDebugControlerHist(i)`: pad 1, zero-extended.
    TkGetDebugControlerHist @ 0x0088_51b0 => extern "C" fn tk_get_debug_controler_hist(i: i32) -> u32 {
        tk::tk_get_debug_controler_hist(&mem(), 1, i, 4)
    }
    /// `TkGetDebugControlerReptHist(i)`: pad 1, zero-extended.
    TkGetDebugControlerReptHist @ 0x0088_51d0 => extern "C" fn tk_get_debug_controler_rept_hist(i: i32) -> u32 {
        tk::tk_get_debug_controler_hist(&mem(), 1, i, 0xa)
    }
    /// `TkGetDebugControlerTrigHist(i)`: pad 1, zero-extended.
    TkGetDebugControlerTrigHist @ 0x0088_51f0 => extern "C" fn tk_get_debug_controler_trig_hist(i: i32) -> u32 {
        tk::tk_get_debug_controler_hist(&mem(), 1, i, 6)
    }
    /// `TkGetControlerIdHist(pad, port, i)`: `al` over the frame pointer.
    TkGetControlerIdHist @ 0x0088_8db0 => extern "C" fn tk_get_controler_id_hist(pad: i32, port: i32, i: i32) -> u32 {
        tk::tk_get_controler_id_hist(&mem(), pad, port, i)
    }
    /// `TkGetControlerPressHist(pad, port, i)`.
    TkGetControlerPressHist @ 0x0088_8df0 => extern "C" fn tk_get_controler_press_hist(pad: i32, port: i32, i: i32) -> u32 {
        tk::tk_get_controler_press_hist(&mem(), pad, port, i)
    }
    /// `TkDebugFont_SetWord4(p)`: `p` in `eax`.
    TkDebugFont_SetWord4 @ 0x0088_fb10 => extern "C" fn tk_debug_font_set_word4(p: u32) -> u32 {
        tk::tk_debug_font_set_word4(&mut mem(), p)
    }
    /// `TkMenuMainFreeWindowGroup(group)`.
    TkMenuMainFreeWindowGroup @ 0x008a_a410 => extern "C" fn tk_menu_main_free_window_group(group: u32) {
        tk::tk_menu_main_free_window_group(&mut mem(), group);
    }
    /// `Dma_CloseCnt(p, end, flags)`: `end`.
    Dma_CloseCnt @ 0x0089_0a60 => extern "C" fn dma_close_cnt(p: u32, end: u32, flags: u32) -> u32 {
        tk::dma_close_cnt(&mut mem(), p, end, flags)
    }
    /// `Dma_CloseCnt2(p, end, a, b)`: `end`.
    Dma_CloseCnt2 @ 0x0089_0aa0 => extern "C" fn dma_close_cnt2(p: u32, end: u32, a: u32, b: u32) -> u32 {
        tk::dma_close_cnt2(&mut mem(), p, end, a, b)
    }
    /// `Dma_CloseNext(p, end, addr, a, b)`: `end`.
    Dma_CloseNext @ 0x0089_0ae0 => extern "C" fn dma_close_next(p: u32, end: u32, addr: u32, a: u32, b: u32) -> u32 {
        tk::dma_close_next(&mut mem(), p, end, addr, a, b)
    }
    /// `Dma_PutCall(p, addr)`: `p + 0x20`.
    Dma_PutCall @ 0x0089_08c0 => extern "C" fn dma_put_call(p: u32, addr: u32) -> u32 {
        tk::dma_put_tag(&mut mem(), p, 0x50, addr)
    }
    /// `Dma_PutEnd(p)`: `p + 0x20`.
    Dma_PutEnd @ 0x0089_0910 => extern "C" fn dma_put_end(p: u32) -> u32 {
        tk::dma_put_tag(&mut mem(), p, 0x70, 0)
    }
    /// `Dma_PutCnt(p)`: `p + 0x20`.
    Dma_PutCnt @ 0x0089_0960 => extern "C" fn dma_put_cnt(p: u32) -> u32 {
        tk::dma_put_tag(&mut mem(), p, 0x10, 0)
    }
    /// `Dma_PutRet(p)`: `p + 0x20`.
    Dma_PutRet @ 0x0089_09b0 => extern "C" fn dma_put_ret(p: u32) -> u32 {
        tk::dma_put_tag(&mut mem(), p, 0x60, 0)
    }
    /// `Dma_PutCall2(p, addr)`: a copy of `Dma_PutCall`; `p + 0x20`.
    Dma_PutCall2 @ 0x0089_0d90 => extern "C" fn dma_put_call2(p: u32, addr: u32) -> u32 {
        tk::dma_put_tag(&mut mem(), p, 0x50, addr)
    }
    /// `MenuWin_StateSlide(w)`: 0.
    MenuWin_StateSlide @ 0x008e_0200 => extern "C" fn menu_win_state_slide(w: u32) -> u32 {
        tk::menu_win_state_slide(&mut mem(), w)
    }
    /// `MenuWin_StateOpen(w)`: 0.
    MenuWin_StateOpen @ 0x008e_0240 => extern "C" fn menu_win_state_open(w: u32) -> u32 {
        tk::menu_win_state_open(&mut mem(), w)
    }
    /// `MenuWin_StateWaitText(w)`: 0.
    MenuWin_StateWaitText @ 0x008e_0280 => extern "C" fn menu_win_state_wait_text(w: u32) -> u32 {
        tk::menu_win_state_wait_text(&mut mem(), w)
    }
    /// `Yi_PutRef(p, a, b)`: `a` (left in `eax`).
    Yi_PutRef @ 0x0091_6ad0 => extern "C" fn yi_put_ref(p: u32, a: u32, b: u32) -> u32 {
        tk::yi_put_ref(&mut mem(), p, a, b)
    }
    /// `Math_ClampPi(a)`: in `st0`.
    Math_ClampPi @ 0x0088_b800 => extern "C" fn math_clamp_pi(a: f32) -> f32 {
        ffx_menu::math::clamp_pi(a)
    }
    /// `TkMenuDrawSp3R0(x, y)`: `TkMenuDrawSp3R(x, y, 0)` (the original
    /// writes the 0 into the caller's third argument slot and jumps).
    TkMenuDrawSp3R0 @ 0x008d_ca30 => extern "C" fn tk_menu_draw_sp3_r0(x: u32, y: u32) -> u32 {
        let mut m = mem();
        let p = tk::MENU_PACKET.get(&m).wrapping_add(0x10);
        let end = tk::menu_draw_sprites(&mut m, [p, 0, x, y, 0, 0, 0, 0x80], false);
        tk::MENU_PACKET.set(&mut m, end);
        end
    }
    /// `TkMn2SeqGetColor(name)`: scene 1's colour word.
    TkMn2SeqGetColor @ 0x008c_2ad0 => extern "C" fn tk_mn2_seq_get_color(name: u32) -> u32 {
        tk::tk_mn2_seq_get_color(&mem(), 1, name)
    }
    /// `TkMn2SeqGetColorCustom(i, name)`.
    TkMn2SeqGetColorCustom @ 0x008c_2af0 => extern "C" fn tk_mn2_seq_get_color_custom(i: i32, name: u32) -> u32 {
        tk::tk_mn2_seq_get_color(&mem(), i, name)
    }
    /// `TkMn2SeqGetNum(name)`: sign-extended.
    TkMn2SeqGetNum @ 0x008c_2b10 => extern "C" fn tk_mn2_seq_get_num(name: u32) -> i32 {
        tk::tk_mn2_seq_get_num(&mem(), 1, name)
    }
    /// `TkMn2SeqGetNumCustom(i, name)`: sign-extended.
    TkMn2SeqGetNumCustom @ 0x008c_2b30 => extern "C" fn tk_mn2_seq_get_num_custom(i: i32, name: u32) -> i32 {
        tk::tk_mn2_seq_get_num(&mem(), i, name)
    }
    /// `TkMn2SeqGetPosCustom(i, name, &pos)`.
    TkMn2SeqGetPosCustom @ 0x008c_2b70 => extern "C" fn tk_mn2_seq_get_pos_custom(i: i32, name: u32, out: u32) {
        tk::tk_mn2_seq_get_pos_custom(&mut mem(), i, name, out);
    }
    /// `TkMn2SeqGetRectCustom(i, name, &rect)`.
    TkMn2SeqGetRectCustom @ 0x008c_2bc0 => extern "C" fn tk_mn2_seq_get_rect_custom(i: i32, name: u32, out: u32) {
        tk::tk_mn2_seq_get_rect_custom(&mut mem(), i, name, out);
    }
    /// `Gu_ControlFreeText(control)`: through the game's `user_free` (itself
    /// ported).
    Gu_ControlFreeText @ 0x0084_94f0 => extern "C" fn gu_control_free_text(control: u32) -> u32 {
        gu::gu_control_free_text(&mut mem(), control, &mut |_, p| user_free(p))
    }
    /// `Gu_SelectorDispose(selector)`: through the game's `user_free`.
    Gu_SelectorDispose @ 0x0084_9bd0 => extern "C" fn gu_selector_dispose(selector: u32) -> u32 {
        gu::gu_selector_dispose(&mut mem(), selector, &mut |_, p| user_free(p))
    }
    /// `MenuDraw_Sp3Body(p, _, x, y, entry, ox, oy, alpha)`.
    MenuDraw_Sp3Body @ 0x008d_bf20 => extern "C" fn menu_draw_sp3_body(p: u32, z: u32, x: u32, y: u32, entry: u32, ox: u32, oy: u32, alpha: u32) -> u32 {
        tk::menu_draw_sprites_plain(&mut mem(), [p, z, x, y, entry, ox, oy, alpha])
    }
    /// `MenuDraw_SpriteP(p, x, y, entry)`.
    MenuDraw_SpriteP @ 0x008d_bd40 => extern "C" fn menu_draw_sprite_p(p: u32, x: u32, y: u32, entry: u32) -> u32 {
        tk::menu_draw_sprites_plain(&mut mem(), [p, 0, x, y, entry, 0, 0, 0x80])
    }
    /// `MenuDraw_SpritePA(p, x, y, entry, alpha)`.
    MenuDraw_SpritePA @ 0x008d_bd70 => extern "C" fn menu_draw_sprite_pa(p: u32, x: u32, y: u32, entry: u32, alpha: u32) -> u32 {
        tk::menu_draw_sprites_plain(&mut mem(), [p, 0, x, y, entry, 0, 0, alpha])
    }
    /// `MenuDraw_SpriteP8A(p, x, y, entry, alpha)`.
    MenuDraw_SpriteP8A @ 0x008d_bda0 => extern "C" fn menu_draw_sprite_p8_a(p: u32, x: u32, y: u32, entry: u32, alpha: u32) -> u32 {
        tk::menu_draw_sprites_plain(&mut mem(), [p, 0, x, y, entry, 8, 0, alpha])
    }
    /// `MenuDraw_SpriteP1(p, x, y, entry)`.
    MenuDraw_SpriteP1 @ 0x008d_be30 => extern "C" fn menu_draw_sprite_p1(p: u32, x: u32, y: u32, entry: u32) -> u32 {
        tk::menu_draw_sprites_plain(&mut mem(), [p, 1, x, y, entry, 0, 0, 0x80])
    }
    /// `MenuDraw_SpriteP1A(p, x, y, entry, alpha)`.
    MenuDraw_SpriteP1A @ 0x008d_be60 => extern "C" fn menu_draw_sprite_p1_a(p: u32, x: u32, y: u32, entry: u32, alpha: u32) -> u32 {
        tk::menu_draw_sprites_plain(&mut mem(), [p, 1, x, y, entry, 0, 0, alpha])
    }
    /// `MenuDraw_SpriteP2(p, x, y, entry)`.
    MenuDraw_SpriteP2 @ 0x008d_be90 => extern "C" fn menu_draw_sprite_p2(p: u32, x: u32, y: u32, entry: u32) -> u32 {
        tk::menu_draw_sprites_plain(&mut mem(), [p, 2, x, y, entry, 0, 0, 0x80])
    }
    /// `MenuDraw_SpriteP2A(p, x, y, entry, alpha)`.
    MenuDraw_SpriteP2A @ 0x008d_bec0 => extern "C" fn menu_draw_sprite_p2_a(p: u32, x: u32, y: u32, entry: u32, alpha: u32) -> u32 {
        tk::menu_draw_sprites_plain(&mut mem(), [p, 2, x, y, entry, 0, 0, alpha])
    }
    /// `MenuDraw_SpriteP8(p, x, y, entry)`.
    MenuDraw_SpriteP8 @ 0x008d_bef0 => extern "C" fn menu_draw_sprite_p8(p: u32, x: u32, y: u32, entry: u32) -> u32 {
        tk::menu_draw_sprites_plain(&mut mem(), [p, 0, x, y, entry, 8, 0, 0x80])
    }
    /// `TkMenuDrawSp3(x, y, entry)`.
    TkMenuDrawSp3 @ 0x008d_d520 => extern "C" fn tk_menu_draw_sp3(x: u32, y: u32, entry: u32) -> u32 {
        menu_packet(|m, p| tk::menu_draw_sprites_plain(m, [p, 0, x, y, entry, 0, 0, 0x80]))
    }
    /// `TkMenuDrawSp3A(x, y, entry, alpha)`.
    TkMenuDrawSp3A @ 0x008d_d550 => extern "C" fn tk_menu_draw_sp3_a(x: u32, y: u32, entry: u32, alpha: u32) -> u32 {
        menu_packet(|m, p| tk::menu_draw_sprites_plain(m, [p, 0, x, y, entry, 0, 0, alpha]))
    }
    /// `TkMenuDrawSp3AHalf(x, y, entry, alpha)`.
    TkMenuDrawSp3AHalf @ 0x008d_d580 => extern "C" fn tk_menu_draw_sp3_ahalf(x: u32, y: u32, entry: u32, alpha: u32) -> u32 {
        menu_packet(|m, p| tk::menu_draw_sprites_plain(m, [p, 0, x, y, entry, 8, 0, alpha]))
    }
    /// `TkMenuDrawSp3AUR(x, y, entry, alpha)`.
    TkMenuDrawSp3AUR @ 0x008d_d5b0 => extern "C" fn tk_menu_draw_sp3_aur(x: u32, y: u32, entry: u32, alpha: u32) -> u32 {
        menu_packet(|m, p| tk::menu_draw_sprites(m, [p, 0, x, y, entry, 0, 0, alpha], true))
    }
    /// `TkMenuDrawSp3AURHalf(x, y, entry, alpha)`.
    TkMenuDrawSp3AURHalf @ 0x008d_d5e0 => extern "C" fn tk_menu_draw_sp3_aurhalf(x: u32, y: u32, entry: u32, alpha: u32) -> u32 {
        menu_packet(|m, p| tk::menu_draw_sprites(m, [p, 0, x, y, entry, 8, 0, alpha], true))
    }
    /// `TkMenuDrawSp3FaceB(x, y, entry)`.
    TkMenuDrawSp3FaceB @ 0x008d_d740 => extern "C" fn tk_menu_draw_sp3_face_b(x: u32, y: u32, entry: u32) -> u32 {
        menu_packet(|m, p| tk::menu_draw_sprites_plain(m, [p, 2, x, y, entry, 0, 0, 0x80]))
    }
    /// `TkMenuDrawSp3FaceBA(x, y, entry, alpha)`.
    TkMenuDrawSp3FaceBA @ 0x008d_d770 => extern "C" fn tk_menu_draw_sp3_face_ba(x: u32, y: u32, entry: u32, alpha: u32) -> u32 {
        menu_packet(|m, p| tk::menu_draw_sprites_plain(m, [p, 2, x, y, entry, 0, 0, alpha]))
    }
    /// `TkMenuDrawSp3Half(x, y, entry)`.
    TkMenuDrawSp3Half @ 0x008d_d7a0 => extern "C" fn tk_menu_draw_sp3_half(x: u32, y: u32, entry: u32) -> u32 {
        menu_packet(|m, p| tk::menu_draw_sprites_plain(m, [p, 0, x, y, entry, 8, 0, 0x80]))
    }
    /// `TkMenuDrawSp30(x, y)`.
    TkMenuDrawSp30 @ 0x008d_ca20 => extern "C" fn tk_menu_draw_sp30(x: u32, y: u32) -> u32 {
        menu_packet(|m, p| tk::menu_draw_sprites_plain(m, [p, 0, x, y, 0, 0, 0, 0x80]))
    }
    /// `TkMn2SeqStartScene(name)`.
    TkMn2SeqStartScene @ 0x008c_2bf0 => extern "C" fn tk_mn2_seq_start_scene(name: u32) -> u32 {
        tk::tk_mn2_seq_start_scene(&mut mem(), name)
    }
    /// `TkMn2SeqStopScene(name)`.
    TkMn2SeqStopScene @ 0x008c_2c10 => extern "C" fn tk_mn2_seq_stop_scene(name: u32) -> u32 {
        tk::tk_mn2_seq_stop_scene(&mut mem(), name)
    }
    /// `TkMn2SetMacroParam(n, b, v)`: `v`.
    TkMn2SetMacroParam @ 0x008c_2c60 => extern "C" fn tk_mn2_set_macro_param(n: i32, b: u32, v: u32) -> u32 {
        tk::tk_mn2_set_macro_param(&mut mem(), n, b, v)
    }
    /// `TkGetDebugControlerAnalog(k)`.
    TkGetDebugControlerAnalog @ 0x0088_5190 => extern "C" fn tk_get_debug_controler_analog(k: u32) -> u32 {
        tk::tk_get_debug_controler_analog(&mem(), k)
    }
    /// `TkMenuMainAllocWindow()`: the record, or 0.
    TkMenuMainAllocWindow @ 0x008a_a1a0 => extern "C" fn tk_menu_main_alloc_window() -> u32 {
        tk::tk_menu_main_alloc_window(&mut mem())
    }
    /// `TkMalloc(size)`: through the game's `user_malloc`.
    TkMalloc @ 0x0088_e650 => extern "C" fn tk_malloc(size: u32) -> u32 {
        tk::tk_malloc(&mut mem(), size, &mut |_, n| user_malloc(n))
    }
    /// `TkMn2DumpPlate(plate)`: the halfword `+6` (left in `eax`).
    TkMn2DumpPlate @ 0x008c_18a0 => extern "C" fn tk_mn2_dump_plate(plate: u32) -> u32 {
        tk::tk_mn2_dump_plate(&mem(), plate)
    }
    /// `TkMn2GetMainPos()`: the last colour.
    TkMn2GetMainPos @ 0x008e_2630 => extern "C" fn tk_mn2_get_main_pos() -> u32 {
        tk::tk_mn2_get_main_pos(&mut mem())
    }
    /// `TkMn2GetCommonPlate(plate, rect, c1, c2, a, b)`: `b`.
    TkMn2GetCommonPlate @ 0x008c_1a60 => extern "C" fn tk_mn2_get_common_plate(plate: u32, rect: u32, c1: u32, c2: u32, a: u32, b: u32) -> u32 {
        tk::tk_mn2_get_common_plate(&mut mem(), 1, plate, [rect, c1, c2], a, b)
    }
    /// `TkMn2GetCommonPlateCustom(i, plate, rect, c1, c2, a, b)`: `b`.
    TkMn2GetCommonPlateCustom @ 0x008c_1aa0 => extern "C" fn tk_mn2_get_common_plate_custom(i: i32, plate: u32, rect: u32, c1: u32, c2: u32, a: u32, b: u32) -> u32 {
        tk::tk_mn2_get_common_plate(&mut mem(), i, plate, [rect, c1, c2], a, b)
    }
    /// `Script_PcOffset(ctx, worker)`.
    Script_PcOffset @ 0x0087_25e0 => extern "C" fn script_pc_offset(ctx: u32, worker: u32) -> u32 {
        tk::script_pc_offset(&mem(), ctx, worker)
    }
    /// `Tk_NoteEternalCalm(scene)`: the check's result.
    Tk_NoteEternalCalm @ 0x0088_ec90 => extern "C" fn tk_note_eternal_calm(scene: i32) -> u32 {
        tk::tk_note_eternal_calm(&mut mem(), scene)
    }
    /// `Gu_SelectorResetNbItem(selector, n)`: through the game's
    /// `user_free`/`user_malloc`; the data array.
    Gu_SelectorResetNbItem @ 0x0084_9f40 => extern "C" fn gu_selector_reset_nb_item(selector: u32, n: u32) -> u32 {
        gu::gu_selector_reset_nb_item(&mut mem(), selector, n, &mut |_, p| user_free(p), &mut |_, n| user_malloc(n))
    }
    /// `Gu_InitBox(box, x, y, w, h)`: 0.
    Gu_InitBox @ 0x0084_97f0 => extern "C" fn gu_init_box(b: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
        gu::gu_init_box(&mut mem(), b, [x, y, w, h])
    }
    /// `MenuTuto_CloseWindow()`: the stop's result.
    MenuTuto_CloseWindow @ 0x008d_9b30 => extern "C" fn menu_tuto_close_window() -> u32 {
        tk::menu_tuto_close_window(&mut mem(), 0)
    }
    /// `MenuTuto_CloseWindow2()`: the stop's result, or 0.
    MenuTuto_CloseWindow2 @ 0x008d_9b60 => extern "C" fn menu_tuto_close_window2() -> u32 {
        tk::menu_tuto_close_window(&mut mem(), 1)
    }
    /// `MenuSave_OpenWindow2(arg)`: the window.
    MenuSave_OpenWindow2 @ 0x008b_4fb0 => extern "C" fn menu_save_open_window2(arg: u32) -> u32 {
        tk::menu_save_open_window2(&mut mem(), arg, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWorldMap_OpenWindow()`: the window.
    MenuWorldMap_OpenWindow @ 0x008e_2d60 => extern "C" fn menu_world_map_open_window() -> u32 {
        tk::menu_world_map_open_window(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2OpenSummonGrowWindow()`: the window.
    TkMn2OpenSummonGrowWindow @ 0x008c_ed30 => extern "C" fn tk_mn2_open_summon_grow_window() -> u32 {
        tk::tk_mn2_open_summon_grow_window(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuLimit_OpenWindow(arg)`: the registration's result.
    MenuLimit_OpenWindow @ 0x008d_0d50 => extern "C" fn menu_limit_open_window(arg: u32) -> u32 {
        tk::menu_limit_open_window(&mut mem(), arg, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuOpenSubCmdWindowMain(x, items, c, d)`: the window.
    TkMenuOpenSubCmdWindowMain @ 0x008e_33d0 => extern "C" fn tk_menu_open_sub_cmd_window_main(x: u32, items: u32, c: u32, d: u32) -> u32 {
        tk::menu_sub_cmd_open(&mut mem(), [x, items, c, d], true, [1, 1], &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuOpenSubCmdWindowMain_008e4790(id, texts, values, n, width, b44)`: the window.
    TkMenuOpenSubCmdWindowMain_008e4790 @ 0x008e_4790 => extern "C" fn tk_menu_open_sub_cmd_window_row(id: u32, texts: u32, values: u32, n: u32, width: u32, b44: u32) -> u32 {
        tk::tk_menu_open_sub_cmd_window_row(&mut mem(), [id, texts, values, n, width, b44], &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSubCmd_OpenMainA(x, items, c, d)`: the window.
    MenuSubCmd_OpenMainA @ 0x008e_3470 => extern "C" fn menu_sub_cmd_open_main_a(x: u32, items: u32, c: u32, d: u32) -> u32 {
        tk::menu_sub_cmd_open(&mut mem(), [x, items, c, d], false, [0, 1], &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSubCmd_OpenMainB(x, items, c, d)`: the window.
    MenuSubCmd_OpenMainB @ 0x008e_3510 => extern "C" fn menu_sub_cmd_open_main_b(x: u32, items: u32, c: u32, d: u32) -> u32 {
        tk::menu_sub_cmd_open(&mut mem(), [x, items, c, d], true, [1, 2], &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSubCmd_OpenMainC(x, items, c, d)`: the window.
    MenuSubCmd_OpenMainC @ 0x008e_35b0 => extern "C" fn menu_sub_cmd_open_main_c(x: u32, items: u32, c: u32, d: u32) -> u32 {
        tk::menu_sub_cmd_open(&mut mem(), [x, items, c, d], true, [0, 2], &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuOpenSubCmdWindow(x, items, c)`: `d` 0.
    TkMenuOpenSubCmdWindow @ 0x008e_33a0 => extern "C" fn tk_menu_open_sub_cmd_window(x: u32, items: u32, c: u32) -> u32 {
        tk::menu_sub_cmd_open(&mut mem(), [x, items, c, 0], true, [1, 1], &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuOpenSubCmdWindow2(x, items, c, d)`: a jump to the main opener.
    TkMenuOpenSubCmdWindow2 @ 0x008e_33c0 => extern "C" fn tk_menu_open_sub_cmd_window2(x: u32, items: u32, c: u32, d: u32) -> u32 {
        tk::menu_sub_cmd_open(&mut mem(), [x, items, c, d], true, [1, 1], &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSubCmd_OpenA(x, items, c)`: `d` 0.
    MenuSubCmd_OpenA @ 0x008e_3650 => extern "C" fn menu_sub_cmd_open_a(x: u32, items: u32, c: u32) -> u32 {
        tk::menu_sub_cmd_open(&mut mem(), [x, items, c, 0], false, [0, 1], &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSubCmd_OpenB(x, items, c)`: `d` 0.
    MenuSubCmd_OpenB @ 0x008e_3670 => extern "C" fn menu_sub_cmd_open_b(x: u32, items: u32, c: u32) -> u32 {
        tk::menu_sub_cmd_open(&mut mem(), [x, items, c, 0], true, [1, 2], &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSubCmd_OpenC(x, items, c)`: `d` 0.
    MenuSubCmd_OpenC @ 0x008e_3690 => extern "C" fn menu_sub_cmd_open_c(x: u32, items: u32, c: u32) -> u32 {
        tk::menu_sub_cmd_open(&mut mem(), [x, items, c, 0], true, [0, 2], &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuCtrlHdd(w)`.
    TkMenuCtrlHdd @ 0x008e_2870 => extern "C" fn tk_menu_ctrl_hdd(w: u32) {
        tk::tk_menu_ctrl_hdd(&mut mem(), w, &mut |_, f, a| call_ptr(f, a));
    }
    /// `Vibrate_BumpStream(k)`: the stream record.
    Vibrate_BumpStream @ 0x0087_5af0 => extern "C" fn vibrate_bump_stream(k: i32) -> u32 {
        tk::vibrate_bump_stream(&mut mem(), k)
    }
    /// `TkDebugFont_SwapBuffer()`: the buffer's start.
    TkDebugFont_SwapBuffer @ 0x0089_0780 => extern "C" fn tk_debug_font_swap_buffer() -> u32 {
        tk::tk_debug_font_swap_buffer(&mut mem())
    }
    /// `Dma_PutTagEx(p, qwc, pce, id, irq, addr)`: the end.
    Dma_PutTagEx @ 0x0089_0860 => extern "C" fn dma_put_tag_ex(p: u32, qwc: u32, pce: u32, id: u32, irq: u32, addr: u32) -> u32 {
        tk::dma_put_tag_ex(&mut mem(), p, qwc, [pce, id, irq], addr)
    }
    /// `Dma_PutTagEx2(p, qwc, pce, id, irq, addr)`: a copy; the end.
    Dma_PutTagEx2 @ 0x0089_0d30 => extern "C" fn dma_put_tag_ex2(p: u32, qwc: u32, pce: u32, id: u32, irq: u32, addr: u32) -> u32 {
        tk::dma_put_tag_ex(&mut mem(), p, qwc, [pce, id, irq], addr)
    }
    /// `Vibrate_SetStream(entry, src)`: the data pointer.
    Vibrate_SetStream @ 0x0087_6230 => extern "C" fn vibrate_set_stream(entry: u32, src: u32) -> u32 {
        tk::vibrate_set_stream(&mut mem(), entry, src)
    }
    /// `Vibrate_ResetAll()`: `0xff`.
    Vibrate_ResetAll @ 0x0087_5e80 => extern "C" fn vibrate_reset_all() -> u32 {
        tk::vibrate_reset_all(&mut mem())
    }
    /// `TkEucToSjis(src, out)`: the bytes read.
    TkEucToSjis @ 0x0089_07e0 => extern "C" fn tk_euc_to_sjis(src: u32, out: u32) -> u32 {
        tk::tk_euc_to_sjis(&mut mem(), src, out)
    }
    /// `Gu_SetAlpha(a, b, c, d, fix)`.
    Gu_SetAlpha @ 0x0084_83e0 => extern "C" fn gu_set_alpha(a: i32, b: i32, c: i32, d: i32, fix: i32) {
        gu::gu_set_alpha(&mut mem(), [a, b, c, d, fix]);
    }
    /// `Gu_FontSetupWindow(window)`.
    Gu_FontSetupWindow @ 0x0084_8310 => extern "C" fn gu_font_setup_window(window: u32) {
        gu::gu_font_setup_window(&mut mem(), window);
    }
    /// `TkNormalFVector(v)`.
    TkNormalFVector @ 0x0088_c5a0 => extern "C" fn tk_normal_f_vector(v: u32) {
        tk::tk_normal_f_vector(&mut mem(), v);
    }
    /// `hardCodeToFixTheWrongData(_, kind)`: see the bridge for `eax`.
    hardCodeToFixTheWrongData @ 0x008e_02f0 => extern "C" fn hard_code_to_fix_the_wrong_data(_a: u32, kind: i32) -> u32 {
        tk::hard_code_to_fix_the_wrong_data(&mut mem(), kind)
    }
    /// `TkMenuDrawRect(x, y, w, h, c0, c1)`: through the renderer.
    TkMenuDrawRect @ 0x008d_d340 => extern "C" fn tk_menu_draw_rect(x: i32, y: i32, w: i32, h: i32, c0: u32, c1: u32) -> u32 {
        tk::tk_menu_draw_rect(&mut mem(), [x, y, w, h], [c0, c1], &mut |_| graphic_get_abmap(), &mut |_, f| graphic_draw_ui_element(f))
    }
    /// `TkMenuDrawSp3FaceA(x, y, entry)`: the last rectangle's result.
    TkMenuDrawSp3FaceA @ 0x008d_d610 => extern "C" fn tk_menu_draw_sp3_face_a(x: u32, y: u32, entry: u32) -> u32 {
        tk::tk_menu_draw_sp3_face(&mut mem(), [x, y, entry, 0x80], &mut |_| graphic_get_abmap(), &mut |_, f| graphic_draw_ui_element(f))
    }
    /// `TkMenuDrawSp3FaceAA(x, y, entry, alpha)`: the last rectangle's result.
    TkMenuDrawSp3FaceAA @ 0x008d_d6a0 => extern "C" fn tk_menu_draw_sp3_face_aa(x: u32, y: u32, entry: u32, alpha: u32) -> u32 {
        tk::tk_menu_draw_sp3_face(&mut mem(), [x, y, entry, alpha], &mut |_| graphic_get_abmap(), &mut |_, f| graphic_draw_ui_element(f))
    }
    /// `TkMenuGetCommandName(id)`.
    TkMenuGetCommandName @ 0x008d_de50 => extern "C" fn tk_menu_get_command_name(id: u32) -> u32 {
        tk::tk_menu_get_command_name(&mem(), id)
    }
    /// `Math_StoreMax(dst, src, v)`.
    Math_StoreMax @ 0x0088_9b00 => extern "C" fn math_store_max(dst: u32, src: u32, v: f32) {
        tk::math_store_max(&mut mem(), dst, src, v);
    }
    /// `Math_StoreMin(dst, src, _, _, _, v)`.
    Math_StoreMin @ 0x0088_9b40 => extern "C" fn math_store_min(dst: u32, src: u32, _a: u32, _b: u32, _c: u32, v: f32) {
        tk::math_store_min(&mut mem(), dst, src, v);
    }
    /// `TkDeci2Send(a, n)`: 1, or -1 when busy.
    TkDeci2Send @ 0x0088_5470 => extern "C" fn tk_deci2_send(a: u32, n: i32) -> i32 {
        tk::tk_deci2_send(&mut mem(), a, n)
    }
    /// `TkCh_SetPos(chr, x, y, z)`.
    TkCh_SetPos @ 0x0087_2310 => extern "C" fn tk_ch_set_pos(chr: u32, x: f32, y: f32, z: f32) {
        tk::tk_ch_set_pos(&mut mem(), chr, [x, y, z]);
    }
    /// `CIsqrt(x)`: in `st0`.
    CIsqrt @ 0x0088_c400 => extern "C" fn ci_sqrt(x: f32) -> f32 {
        ffx_menu::math::ci_sqrt(x)
    }
    /// `Math_WrapAngleFloor(a)`: in `st0`.
    Math_WrapAngleFloor @ 0x0088_c630 => extern "C" fn math_wrap_angle_floor(a: f32) -> f32 {
        ffx_menu::math::wrap_angle_floor(a)
    }
    /// `Gu_SelectorAllocate()`: through the game's `user_malloc`.
    Gu_SelectorAllocate @ 0x0084_9bb0 => extern "C" fn gu_selector_allocate() -> u32 {
        gu::gu_selector_allocate(&mut mem(), &mut |_, n| user_malloc(n))
    }
    /// `Gu_ControlInit(control)`: 0.
    Gu_ControlInit @ 0x0084_95e0 => extern "C" fn gu_control_init(control: u32) -> u32 {
        gu::gu_control_init(&mut mem(), control)
    }
    /// `Yi_StrLower(s)`.
    Yi_StrLower @ 0x0091_6290 => extern "C" fn yi_str_lower(s: u32) {
        gu::yi_str_case(&mut mem(), s, false);
    }
    /// `Yi_StrUpper(s)`.
    Yi_StrUpper @ 0x0091_62c0 => extern "C" fn yi_str_upper(s: u32) {
        gu::yi_str_case(&mut mem(), s, true);
    }
    /// `Math_AngleTo12Bits(a)`.
    Math_AngleTo12Bits @ 0x0088_bfe0 => extern "C" fn math_angle_to12_bits(a: f32) -> u32 {
        tk::math_angle_to_bits(&mem(), a, 0x00b5_15c8, 0xfff)
    }
    /// `Math_AngleTo15Bits(a)`.
    Math_AngleTo15Bits @ 0x0088_c040 => extern "C" fn math_angle_to15_bits(a: f32) -> u32 {
        tk::math_angle_to_bits(&mem(), a, 0x00b5_1588, 0x7fff)
    }
    /// `Math_AngleToDegrees(a)`: in `st0`.
    Math_AngleToDegrees @ 0x0088_c5c0 => extern "C" fn math_angle_to_degrees(a: f32) -> f32 {
        tk::math_angle_to_degrees(a)
    }
    /// `Math_AngleStepTo(a, target, step)`: in `st0`.
    Math_AngleStepTo @ 0x0088_b6b0 => extern "C" fn math_angle_step_to(a: f32, target: f32, step: f32) -> f32 {
        tk::math_angle_step_to(a, target, step)
    }
    /// `Vibrate_SetStreams(rec, a, b)`: the second data pointer.
    Vibrate_SetStreams @ 0x0087_6180 => extern "C" fn vibrate_set_streams(rec: u32, a: u32, b: u32) -> u32 {
        tk::vibrate_set_streams(&mut mem(), rec, a, b)
    }
    /// `Vibrate_StartLevel(k, e, from, to, len)`: the length.
    Vibrate_StartLevel @ 0x0087_6050 => extern "C" fn vibrate_start_level(k: i32, e: i32, from: i32, to: i32, len: i32) -> i32 {
        tk::vibrate_start_level(&mut mem(), k, e, from, to, len)
    }
    /// `Vibrate_CurveData(k, n, table)`.
    Vibrate_CurveData @ 0x0087_5d70 => extern "C" fn vibrate_curve_data(k: u32, n: i32, table: u32) -> u32 {
        tk::vibrate_curve_data(&mem(), k, n, table)
    }
    /// `yiClutAlloc_init(mode)`: 0.
    yiClutAlloc_init @ 0x0091_6e60 => extern "C" fn yi_clut_alloc_init(mode: i32) -> u32 {
        tk::yi_clut_alloc_init(&mut mem(), mode)
    }
    /// `yiClutAlloc_get256idx()`.
    yiClutAlloc_get256idx @ 0x0091_6e50 => extern "C" fn yi_clut_alloc_get256idx() -> u32 {
        tk::yi_clut_alloc_get256idx(&mem())
    }
    /// `yiClutAlloc_copy256(clut, src, k)`: 0.
    yiClutAlloc_copy256 @ 0x0091_6d40 => extern "C" fn yi_clut_alloc_copy256(clut: u32, src: u32, k: i32) -> u32 {
        tk::yi_clut_alloc_copy256(&mut mem(), clut, src, k)
    }
    /// `yiClutAlloc_copy16(clut, src, k, i)`: 0.
    yiClutAlloc_copy16 @ 0x0091_6c80 => extern "C" fn yi_clut_alloc_copy16(clut: u32, src: u32, k: i32, i: i32) -> u32 {
        tk::yi_clut_alloc_copy16(&mut mem(), clut, src, k, i)
    }
    /// `Gu_ControlSetText(control, s)`: through the game's heap entries.
    Gu_ControlSetText @ 0x0084_9610 => extern "C" fn gu_control_set_text(control: u32, s: u32) {
        gu::gu_control_set_text(&mut mem(), control, s, &mut |_, p| user_free(p), &mut |_, n| user_malloc(n));
    }
    /// `Gu_WindowControlSetText(window, id, s)`.
    Gu_WindowControlSetText @ 0x0084_a710 => extern "C" fn gu_window_control_set_text(window: u32, id: u32, s: u32) {
        gu::gu_window_control_set_text(&mut mem(), window, id, s, &mut |_, p| user_free(p), &mut |_, n| user_malloc(n));
    }
    /// `TkMn2GetStatusPos()`.
    TkMn2GetStatusPos @ 0x008d_4920 => extern "C" fn tk_mn2_get_status_pos() {
        tk::tk_mn2_get_status_pos(&mut mem());
    }
    /// `Gu_WindowSetTitle(window, s)`: the title control (`+0x34`).
    Gu_WindowSetTitle @ 0x0084_b6a0 => extern "C" fn gu_window_set_title(window: u32, s: u32) {
        gu::gu_control_set_text(&mut mem(), window.wrapping_add(0x34), s, &mut |_, p| user_free(p), &mut |_, n| user_malloc(n));
    }
    /// `Vibrate_SetStreamsMode(rec, mode, a, b)`.
    Vibrate_SetStreamsMode @ 0x0087_6130 => extern "C" fn vibrate_set_streams_mode(rec: u32, mode: i32, a: u32, b: u32) -> u32 {
        tk::vibrate_set_streams_mode(&mut mem(), rec, mode, a, b)
    }
    /// `TkSetVibrateStream(k, curve, table)`.
    TkSetVibrateStream @ 0x0087_5fb0 => extern "C" fn tk_set_vibrate_stream(k: i32, curve: u32, table: u32) -> u32 {
        tk::tk_set_vibrate_stream(&mut mem(), k, curve, table)
    }
    /// `TkGetSavedLang()`.
    TkGetSavedLang @ 0x0088_c850 => extern "C" fn tk_get_saved_lang() -> u32 {
        tk::get_byte(&mem(), 0x0133_0874)
    }
    /// `Ch_RequestCalcNormal()`.
    Ch_RequestCalcNormal @ 0x0083_2540 => extern "C" fn ch_request_calc_normal() {
        tk::ch_request_calc_normal(&mut mem());
    }
    /// `TkGetControlerBindAnalogIndex(pad, port, code)`.
    TkGetControlerBindAnalogIndex @ 0x0088_8cf0 => extern "C" fn tk_get_controler_bind_analog_index(pad: i32, port: i32, code: i32) -> i32 {
        tk::tk_get_controler_bind_analog_index(&mem(), pad, port, code)
    }
    /// `Atel_FindWorkerByOwner(owner)`: `u16 +0x2e`, or -1.
    Atel_FindWorkerByOwner @ 0x0087_6440 => extern "C" fn atel_find_worker_by_owner(owner: u32) -> i32 {
        tk::atel_find_worker_by_owner(&mut mem(), owner)
    }
    /// `TkInitSceneFromSave()`.
    TkInitSceneFromSave @ 0x0088_dd60 => extern "C" fn tk_init_scene_from_save() {
        tk::tk_init_scene_from_save(&mut mem());
    }
    /// `TkMenuDrawMenuWindowScissor(window, mode, t)`.
    TkMenuDrawMenuWindowScissor @ 0x008d_d220 => extern "C" fn tk_menu_draw_menu_window_scissor(window: u32, mode: i32, t: i32) -> u32 {
        tk::tk_menu_draw_menu_window_scissor(&mut mem(), window, mode, t)
    }
    /// `Vibrate_StepLevel(rec, entry, len)`: the level.
    Vibrate_StepLevel @ 0x0087_5c90 => extern "C" fn vibrate_step_level(rec: u32, entry: u32, len: i32) -> u32 {
        tk::vibrate_step_level(&mut mem(), rec, entry, len)
    }
    /// `Gu_WindowFreeClosed()`: through the game's `user_free`.
    Gu_WindowFreeClosed @ 0x0084_abd0 => extern "C" fn gu_window_free_closed() {
        gu::gu_window_free_closed(&mut mem(), &mut |_, p| user_free(p));
    }
    /// `Gu_SelectorFitWindow(window, selector)`.
    Gu_SelectorFitWindow @ 0x0084_a000 => extern "C" fn gu_selector_fit_window(window: u32, selector: u32) -> i32 {
        gu::gu_selector_fit_window(&mut mem(), window, selector)
    }
    /// `TkEucToSjisString(src, dst, n)`.
    TkEucToSjisString @ 0x0088_ed10 => extern "C" fn tk_euc_to_sjis_string(src: u32, dst: u32, n: i32) -> u32 {
        tk::tk_euc_to_sjis_string(&mut mem(), src, dst, n)
    }
    /// `TkMenuDrawMenuWindowScissorWithLabel(window, mode, t)`.
    TkMenuDrawMenuWindowScissorWithLabel @ 0x008d_d2d0 => extern "C" fn tk_menu_draw_menu_window_scissor_with_label(window: u32, mode: i32, t: i32) -> u32 {
        tk::tk_menu_draw_menu_window_scissor_with_label(&mut mem(), window, mode, t)
    }
    /// `Vibrate_EntryLevel(rec, e)`.
    Vibrate_EntryLevel @ 0x0087_5c20 => extern "C" fn vibrate_entry_level(rec: u32, e: u32) -> u32 {
        tk::vibrate_entry_level(&mut mem(), rec, e)
    }
    /// `CIcos(x)`: `_CIcos`, in `st0`.
    CIcos @ 0x0088_b3e0 => extern "C" fn ci_cos(x: f32) -> f32 {
        ffx_menu::math::ci_unary(x, |v| crt_x87(0x00b0_c3b4, v, None))
    }
    /// `CIsin(x)`: `_CIsin`, in `st0`.
    CIsin @ 0x0088_c3a0 => extern "C" fn ci_sin(x: f32) -> f32 {
        ffx_menu::math::ci_unary(x, |v| crt_x87(0x00b0_c3b8, v, None))
    }
    /// `CItan(x)`: `_CItan`, in `st0`.
    CItan @ 0x0088_c420 => extern "C" fn ci_tan(x: f32) -> f32 {
        ffx_menu::math::ci_unary(x, |v| crt_x87(0x00b0_c3a0, v, None))
    }
    /// `CIatan2(y, x)`: `_CIatan2`, in `st0`.
    CIatan2 @ 0x0088_9e70 => extern "C" fn ci_atan2(y: f32, x: f32) -> f32 {
        ffx_menu::math::ci_atan2(y, x, |a, b| crt_x87(0x00b0_c3b0, a, Some(b)))
    }
    /// `MenuSave_PathDir(out, n)`: `/S`.
    MenuSave_PathDir @ 0x008b_3ff0 => extern "C" fn menu_save_path_dir(out: u32, n: u32) {
        tk::menu_save_path(&mut mem(), out, n, 0);
    }
    /// `MenuSave_PathData(out, n)`: `/S/S`.
    MenuSave_PathData @ 0x008b_4060 => extern "C" fn menu_save_path_data(out: u32, n: u32) {
        tk::menu_save_path(&mut mem(), out, n, 1);
    }
    /// `MenuSave_PathIcon(out, n)`: `/S/010906nn.ico`.
    MenuSave_PathIcon @ 0x008b_4110 => extern "C" fn menu_save_path_icon(out: u32, n: u32) {
        tk::menu_save_path(&mut mem(), out, n, 2);
    }
    /// `MenuSave_PathIconSys(out, n)`: `/S/icon.sys`.
    MenuSave_PathIconSys @ 0x008b_4230 => extern "C" fn menu_save_path_icon_sys(out: u32, n: u32) {
        tk::menu_save_path(&mut mem(), out, n, 3);
    }
    /// `MenuSave_IconName(out, n)`.
    MenuSave_IconName @ 0x008b_41e0 => extern "C" fn menu_save_icon_name(out: u32, n: u32) {
        tk::menu_save_icon_name(&mut mem(), out, n);
    }
    /// `Gu_WindowControlSetTextf(window, id, fmt, ...)`: up to 12 argument
    /// words read; a format beyond the bridge's formatter goes through the
    /// game's own `vsprintf` (import slot `0x00b0c430`).
    Gu_WindowControlSetTextf @ 0x0084_a810 => extern "C" fn gu_window_control_set_textf(window: u32, id: u32, fmt: u32, a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32) {
        let args = [a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11];
        let mut m = mem();
        let done = gu::gu_window_control_set_textf(&mut m, window, id, fmt, &args, &mut |_, p| user_free(p), &mut |_, n| user_malloc(n));
        if !done {
            let mut buf = [0u8; 0x400];
            let at = u32::try_from(buf.as_mut_ptr() as usize).unwrap_or(0);
            let list = u32::try_from(args.as_ptr() as usize).unwrap_or(0);
            // SAFETY: an import slot of this build, filled by the loader.
            let vsprintf = unsafe { (crate::rebase(0x00b0_c430) as *const u32).read() };
            call_ptr(vsprintf, &[at, fmt, list]);
            gu::gu_window_control_set_text(&mut m, window, id, at, &mut |_, p| user_free(p), &mut |_, n| user_malloc(n));
        }
    }
    /// `MenuKaizou_InitList(w)`.
    MenuKaizou_InitList @ 0x008d_55e0 => extern "C" fn menu_kaizou_init_list(w: u32) {
        tk::menu_kaizou_init_list(&mut mem(), w);
    }
    /// `MenuSummon_GetStat(base, desc, max)`.
    MenuSummon_GetStat @ 0x008c_eca0 => extern "C" fn menu_summon_get_stat(base: u32, desc: u32, max: u32) -> u32 {
        tk::menu_summon_get_stat(&mem(), base, desc, max)
    }
    /// `MenuSummon_SetStat(base, desc, max, v)`.
    MenuSummon_SetStat @ 0x008c_ee70 => extern "C" fn menu_summon_set_stat(base: u32, desc: u32, max: u32, v: u32) {
        tk::menu_summon_set_stat(&mut mem(), base, desc, max, v);
    }
    /// `Menu_FindByte(bytes, v, from, to)`.
    Menu_FindByte @ 0x008c_9bc0 => extern "C" fn menu_find_byte(bytes: u32, v: u32, from: i32, to: i32) -> i32 {
        tk::menu_find_byte(&mem(), bytes, v, from, to)
    }
    /// `Menu_FindFlagged(entries, from, to)`.
    Menu_FindFlagged @ 0x008c_9b90 => extern "C" fn menu_find_flagged(entries: u32, from: i32, to: i32) -> i32 {
        tk::menu_find_flagged(&mem(), entries, from, to)
    }
    /// `Menu_FadeStep(obj)`.
    Menu_FadeStep @ 0x008e_2b30 => extern "C" fn menu_fade_step(obj: u32) -> u32 {
        tk::menu_fade_step(&mut mem(), obj)
    }
    /// `Menu_ParseItemKeys(s)`.
    Menu_ParseItemKeys @ 0x008c_5280 => extern "C" fn menu_parse_item_keys(s: u32) {
        tk::menu_parse_item_keys(&mut mem(), s);
    }
    /// `MenuScene_ResetGroup(scene, off, on)`.
    MenuScene_ResetGroup @ 0x00a7_ba70 => extern "C" fn menu_scene_reset_group(scene: u32, off: u32, on: u32) -> u32 {
        tk::menu_scene_reset_group(&mut mem(), scene, off, on)
    }
    /// `MenuParty_CursorPrev()`.
    MenuParty_CursorPrev @ 0x008c_ba60 => extern "C" fn menu_party_cursor_prev() {
        tk::menu_party_cursor_prev(&mut mem());
    }
    /// `MenuParty_CursorNext()`.
    MenuParty_CursorNext @ 0x008c_ba20 => extern "C" fn menu_party_cursor_next() {
        tk::menu_party_cursor_next(&mut mem());
    }
    /// `Menu_Scale32(a, b)`.
    Menu_Scale32 @ 0x008b_f100 => extern "C" fn menu_scale32(a: i32, b: i32) -> i32 {
        tk::menu_scale32(a, b)
    }
    /// `Menu_EaseCounter(-, counter, a, b)`.
    Menu_EaseCounter @ 0x008b_f7e0 => extern "C" fn menu_ease_counter(_x: u32, counter: u32, a: u32, b: u32) {
        tk::menu_ease_counter(&mut mem(), counter, a, b);
    }
    /// `TkSetRgba_0133c8f4(i, r, g, b, a)`.
    TkSetRgba_0133c8f4 @ 0x0088_fca0 => extern "C" fn tk_set_rgba_c8f4(i: i32, r: i32, g: i32, b: i32, a: i32) -> u32 {
        tk::tk_set_rgba(&mut mem(), 0x0133_c8f4, 3, i, [r, g, b, a])
    }
    /// `TkSetRgba_0133c8e4(i, r, g, b, a)`.
    TkSetRgba_0133c8e4 @ 0x0089_0320 => extern "C" fn tk_set_rgba_c8e4(i: i32, r: i32, g: i32, b: i32, a: i32) -> u32 {
        tk::tk_set_rgba(&mut mem(), 0x0133_c8e4, 3, i, [r, g, b, a])
    }
    /// `TkSetRgba_0133c904(i, r, g, b, a)`.
    TkSetRgba_0133c904 @ 0x0089_06d0 => extern "C" fn tk_set_rgba_c904(i: i32, r: i32, g: i32, b: i32, a: i32) -> u32 {
        tk::tk_set_rgba(&mut mem(), 0x0133_c904, 2, i, [r, g, b, a])
    }
    /// `Menu_AddName(s)`.
    Menu_AddName @ 0x008e_5200 => extern "C" fn menu_add_name(s: u32) -> u32 {
        tk::menu_add_name(&mut mem(), s)
    }
    /// `Math_CrossHeightGap(p1, p2, p3, p4)`: in `st0`.
    Math_CrossHeightGap @ 0x0088_9e90 => extern "C" fn math_cross_height_gap(p1: u32, p2: u32, p3: u32, p4: u32) -> f32 {
        let m = mem();
        let v = |p: u32| -> [f32; 3] { [0u32, 4, 8].map(|o| ffx_bridge::x87::round_trip(ffx_bridge::Mem::read(&m, p.wrapping_add(o)))) };
        ffx_menu::math::cross_height_gap(v(p1), v(p2), v(p3), v(p4))
    }
    /// `TkSyncV()`: 0.
    TkSyncV @ 0x0088_7f00 => extern "C" fn tk_sync_v() -> u32 {
        tk::tk_sync_v(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2DrawInitPacket()`: the half packet size.
    TkMn2DrawInitPacket @ 0x008c_0be0 => extern "C" fn tk_mn2_draw_init_packet() -> u32 {
        tk::tk_mn2_draw_init_packet(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2DrawKickPacketBody(clear)`.
    TkMn2DrawKickPacketBody @ 0x008c_0c50 => extern "C" fn tk_mn2_draw_kick_packet_body(clear: u32) -> u32 {
        tk::tk_mn2_draw_kick_packet_body(&mut mem(), clear, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2DrawKickSyncPacket()`.
    TkMn2DrawKickSyncPacket @ 0x008c_0cc0 => extern "C" fn tk_mn2_draw_kick_sync_packet() -> u32 {
        tk::tk_mn2_draw_kick_sync_packet(&mut mem(), 1, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2DrawKickSyncPacket2()`.
    TkMn2DrawKickSyncPacket2 @ 0x008c_0cd0 => extern "C" fn tk_mn2_draw_kick_sync_packet2() -> u32 {
        tk::tk_mn2_draw_kick_sync_packet(&mut mem(), 0, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuMn2_EmitPacket(a, b, kind)`: the new cursor.
    MenuMn2_EmitPacket @ 0x008c_11b0 => extern "C" fn menu_mn2_emit_packet(a: u32, b: u32, kind: u32) -> u32 {
        tk::menu_mn2_emit_packet(&mut mem(), [a, b, kind], &mut |_, f, x| call_ptr(f, x))
    }
    /// `TkSndIsRead()`: 1 or 0.
    TkSndIsRead @ 0x0088_6600 => extern "C" fn tk_snd_is_read() -> u32 {
        tk::tk_snd_is_read(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkSetSyncVCallback(id, f)`.
    TkSetSyncVCallback @ 0x0088_7ed0 => extern "C" fn tk_set_sync_v_callback(id: u32, f: u32) -> u32 {
        tk::tk_set_sync_v_callback(&mut mem(), id, f, &mut |_, g, a| call_ptr(g, a))
    }
    /// `TkSyncVEI()`.
    TkSyncVEI @ 0x0088_7f40 => extern "C" fn tk_sync_v_ei() -> u32 {
        tk::tk_sync_v_ei(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuShop_SellPrice(id)`.
    MenuShop_SellPrice @ 0x008d_b400 => extern "C" fn menu_shop_sell_price(id: u32) -> u32 {
        tk::menu_shop_sell_price(&mut mem(), id, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkFree(p)`: the last memory check's result.
    TkFree @ 0x0088_d520 => extern "C" fn tk_free(p: u32) -> u32 {
        tk::tk_free(&mut mem(), p, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkSndUpdateMain()`.
    TkSndUpdateMain @ 0x0088_7bd0 => extern "C" fn tk_snd_update_main() -> u32 {
        tk::tk_snd_update_main(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `Math_YawPitchTo(yaw, pitch, fx, fy, fz, tx, ty, tz)`.
    Math_YawPitchTo @ 0x0088_b4d0 => extern "C" fn math_yaw_pitch_to(yaw: u32, pitch: u32, fx: f32, fy: f32, fz: f32, tx: f32, ty: f32, tz: f32) -> u32 {
        tk::math_yaw_pitch_to(&mut mem(), yaw, pitch, [fx, fy, fz], [tx, ty, tz], &mut |_, a, b| crt_x87(0x00b0_c3b0, a, Some(b)))
    }
    /// `Math_FacingWithin(heading, tol, x0, z0, x1, z1)`: 1 or 0.
    Math_FacingWithin @ 0x0088_b780 => extern "C" fn math_facing_within(heading: f32, tol: f32, x0: f32, z0: f32, x1: f32, z1: f32) -> u32 {
        tk::math_facing_within(&mut mem(), heading, tol, [x0, z0], [x1, z1], &mut |_, a, b| crt_x87(0x00b0_c3b0, a, Some(b)))
    }
    /// `Math_PolarToVector(out, r, yaw, pitch)`.
    Math_PolarToVector @ 0x0088_a3a0 => extern "C" fn math_polar_to_vector(out: u32, r: f32, yaw: f32, pitch: f32) {
        tk::math_polar_to_vector(&mut mem(), out, [r, yaw, pitch], &mut |_, cos, v| crt_x87(if cos { 0x00b0_c3b4 } else { 0x00b0_c3b8 }, v, None));
    }
    /// `Math_SegmentsCross(a, b, c, d)`: 1, -1 or 0.
    Math_SegmentsCross @ 0x0088_9bb0 => extern "C" fn math_segments_cross(a: u32, b: u32, c: u32, d: u32) -> i32 {
        tk::math_segments_cross(&mem(), [a, b, c, d])
    }
    /// `TkSndUpdate()`.
    TkSndUpdate @ 0x0088_7b60 => extern "C" fn tk_snd_update() -> u32 {
        tk::tk_snd_update(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkSndUpdate2()`.
    TkSndUpdate2 @ 0x0088_7b80 => extern "C" fn tk_snd_update2() -> u32 {
        tk::tk_snd_update2(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkSndUpdate3(last)`: the sync counter.
    TkSndUpdate3 @ 0x0088_7ba0 => extern "C" fn tk_snd_update3(last: u32) -> u32 {
        tk::tk_snd_update3(&mut mem(), last, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMsImportantSet(n)`.
    TkMsImportantSet @ 0x0088_e790 => extern "C" fn tk_ms_important_set(n: u32) -> u32 {
        tk::tk_ms_important_set(&mut mem(), n, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkRefreshSoundMode()`.
    TkRefreshSoundMode @ 0x0087_24f0 => extern "C" fn tk_refresh_sound_mode() -> u32 {
        tk::tk_refresh_sound_mode(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSubCmd_CenterOpen()`.
    MenuSubCmd_CenterOpen @ 0x008e_4ab0 => extern "C" fn menu_sub_cmd_center_open() -> u32 {
        tk::menu_sub_cmd_center_open(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `Math_Atan2XZ(x0, z0, x1, z1)`: in `st0`.
    Math_Atan2XZ @ 0x0088_b430 => extern "C" fn math_atan2_xz(x0: f32, z0: f32, x1: f32, z1: f32) -> f32 {
        ffx_menu::math::atan2_xz(x0, z0, x1, z1, |a, b| crt_x87(0x00b0_c3b0, a, Some(b)))
    }
    /// `Menu_CursorEase(summon, counter, a, b, last)`.
    Menu_CursorEase @ 0x008b_f750 => extern "C" fn menu_cursor_ease(summon: u32, counter: u32, a: u32, b: u32, last: u32) {
        tk::menu_cursor_ease(&mut mem(), summon == 1, [counter, a, b, last], &mut |_, f, x| call_ptr(f, x));
    }
    /// `TkSetEventJump(map, entrance)`.
    TkSetEventJump @ 0x0088_ea60 => extern "C" fn tk_set_event_jump(map: i32, entrance: u32) {
        tk::tk_set_event_jump(&mut mem(), map, entrance, &mut |_, f, a| call_ptr(f, a));
    }
    /// `Math_AngleStep(cur, target, step)`: in `st0`.
    Math_AngleStep @ 0x0088_b570 => extern "C" fn math_angle_step(cur: f32, target: f32, step: f32) -> f32 {
        ffx_menu::math::angle_step(cur, target, step)
    }
    /// `MenuItem_CursorTable()`.
    MenuItem_CursorTable @ 0x008c_4ff0 => extern "C" fn menu_item_cursor_table() -> u32 {
        tk::menu_item_cursor_table(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_CursorCount()`.
    MenuItem_CursorCount @ 0x008c_5030 => extern "C" fn menu_item_cursor_count() -> u32 {
        tk::menu_item_cursor_count(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_CellTable()`.
    MenuItem_CellTable @ 0x008c_4f80 => extern "C" fn menu_item_cell_table() -> u32 {
        tk::menu_item_cell_table(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuShop_Price(id)`.
    MenuShop_Price @ 0x008d_b3b0 => extern "C" fn menu_shop_price(id: u32) -> i32 {
        tk::menu_shop_price(&mut mem(), id, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuMainRemoveMenu(slot)`.
    TkMenuMainRemoveMenu @ 0x008a_ab20 => extern "C" fn tk_menu_main_remove_menu(slot: i32) -> u32 {
        tk::tk_menu_main_remove_menu(&mut mem(), slot, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkCh_BindToElem(kind, a, chr, b, keep)`.
    TkCh_BindToElem @ 0x0087_22b0 => extern "C" fn tk_ch_bind_to_elem(kind: u32, a: u32, chr: u32, b: u32, keep: u32) -> u32 {
        tk::tk_ch_bind_to_elem(&mut mem(), [kind, a, chr, b, keep], &mut |_, f, x| call_ptr(f, x))
    }
    /// `MenuNameEntry_RegistWindow()`: the window.
    MenuNameEntry_RegistWindow @ 0x008c_4450 => extern "C" fn menu_name_entry_regist_window() -> u32 {
        tk::menu_name_entry_regist_window(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuStatus_OpenWindow(a, index, b, c)`: the window.
    MenuStatus_OpenWindow @ 0x008d_3140 => extern "C" fn menu_status_open_window(a: u32, index: u32, b: u32, c: u32) -> u32 {
        tk::menu_status_open_window(&mut mem(), [a, index, b, c], &mut |_, f, x| call_ptr(f, x))
    }
    /// `MenuItem_KeysMatch(t)`: 1 or 0.
    MenuItem_KeysMatch @ 0x008c_5390 => extern "C" fn menu_item_keys_match(t: i32) -> u32 {
        tk::menu_item_keys_match(&mut mem(), t, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_OpenListWindow(kind)`: the window.
    MenuItem_OpenListWindow_008c9140 @ 0x008c_9140 => extern "C" fn menu_item_open_list_window(kind: u32) -> u32 {
        tk::menu_item_open_list_window(&mut mem(), kind, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_CanAfford(e)`: 1 or 0.
    MenuWShop_CanAfford @ 0x008d_85d0 => extern "C" fn menu_wshop_can_afford(e: u32) -> u32 {
        tk::menu_wshop_can_afford(&mut mem(), e, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuShop_CanBuy(id)`: 1 or 0.
    MenuShop_CanBuy @ 0x008d_ab30 => extern "C" fn menu_shop_can_buy(id: u32) -> u32 {
        tk::menu_shop_can_buy(&mut mem(), id, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuNameEntry_OpenCharWindow(chr)`: the window.
    MenuNameEntry_OpenCharWindow @ 0x008c_43f0 => extern "C" fn menu_name_entry_open_char_window(chr: u32) -> u32 {
        tk::menu_name_entry_open_char_window(&mut mem(), chr, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_TargetNeedsRecovery(kind, all, chr)`: 1 or 0.
    MenuItem_TargetNeedsRecovery_008c6a70 @ 0x008c_6a70 => extern "C" fn menu_item_target_needs_recovery(kind: u32, _all: u32, chr: u32) -> u32 {
        tk::menu_item_target_needs_recovery(&mem(), kind, chr)
    }
    /// `Msg_PollClose(i)`: 1 or 0.
    Msg_PollClose @ 0x0086_0aa0 => extern "C" fn msg_poll_close(i: u32) -> u32 {
        tk::msg_poll_close(&mut mem(), i, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSummon_GrowInfo(stat, chr, cost, gain)`: 1 or 0.
    MenuSummon_GrowInfo @ 0x008c_d320 => extern "C" fn menu_summon_grow_info(stat: i32, chr: u32, cost: u32, gain: u32) -> u32 {
        tk::menu_summon_grow_info(&mut mem(), stat, chr, [cost, gain], &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuNameEntry_LoadLayout()`: the last lookup's result.
    MenuNameEntry_LoadLayout @ 0x008c_5050 => extern "C" fn menu_name_entry_load_layout() -> u32 {
        tk::menu_name_entry_load_layout(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSummon_DrawFaceSlide(x, y, w, h, t, cur, next)`.
    MenuSummon_DrawFaceSlide @ 0x008d_34a0 => extern "C" fn menu_summon_draw_face_slide(x: u32, y: u32, w: u32, h: u32, t: i32, cur: u32, next: u32) -> u32 {
        tk::menu_summon_draw_face_slide(&mut mem(), [x, y, w, h], t, [cur, next], &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSummon_BuildCommandList(chr)`.
    MenuSummon_BuildCommandList @ 0x008d_2b40 => extern "C" fn menu_summon_build_command_list(chr: u32) -> u32 {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return 0;
        };
        tk::menu_summon_build_command_list(&mut m, chr, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuStopScene(scene, group)`: the entry.
    TkMenuStopScene @ 0x00a7_c8a0 => extern "C" fn tk_menu_stop_scene(scene: u32, group: u32) -> u32 {
        tk::tk_menu_stop_scene(&mut mem(), scene, group, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuKaizou_WeaponByte4(w)`.
    MenuKaizou_WeaponByte4 @ 0x008d_56f0 => extern "C" fn menu_kaizou_weapon_byte4(w: u32) -> u32 {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return 0;
        };
        tk::menu_kaizou_weapon_byte4(&mut m, w, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_OpenSubWindow2(parent)`: the weapon shop's buy-amount window.
    MenuWShop_OpenSubWindow2 @ 0x008d_81d0 => extern "C" fn menu_wshop_open_sub_window2(parent: u32) -> u32 {
        tk::menu_wshop_open_sub_window2(&mut mem(), parent, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_OpenSubWindow3(parent)`: the weapon shop's sell window.
    MenuWShop_OpenSubWindow3 @ 0x008d_82d0 => extern "C" fn menu_wshop_open_sub_window3(parent: u32) -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::menu_wshop_open_sub_window3(&mut mem(), parent, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuShop_OpenBuyWindow(parent)`: the item shop's buy-amount window.
    MenuShop_OpenBuyWindow @ 0x008d_a860 => extern "C" fn menu_shop_open_buy_window(parent: u32) -> u32 {
        tk::menu_shop_open_buy_window(&mut mem(), parent, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuShop_OpenSellWindow(parent)`: the item shop's sell window.
    MenuShop_OpenSellWindow @ 0x008d_aa00 => extern "C" fn menu_shop_open_sell_window(parent: u32) -> u32 {
        tk::menu_shop_open_sell_window(&mut mem(), parent, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSummon_OpenWindow2()`: the aeon menu's second window.
    MenuSummon_OpenWindow2 @ 0x008c_eda0 => extern "C" fn menu_summon_open_window2() -> u32 {
        tk::menu_summon_open_window2(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_CountGearByOwner()`: the item menu's equipment counts.
    MenuItem_CountGearByOwner @ 0x008c_94e0 => extern "C" fn menu_item_count_gear_by_owner() -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::menu_item_count_gear_by_owner(&mut mem(), local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuOpenSubCmdWindowRect(id, texts, values, n, b44, x, y, width, h)`.
    TkMenuOpenSubCmdWindowRect @ 0x008e_4930 => extern "C" fn tk_menu_open_sub_cmd_window_rect(id: u32, texts: u32, values: u32, n: u32, b44: u32, x: f32, y: f32, width: f32, h: f32) -> u32 {
        tk::tk_menu_open_sub_cmd_window_rect(&mut mem(), [id, texts, values, n, b44], [x, y, width, h], &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuOpenSubCmdWindowBeside(id, texts, values, n, b44)`.
    TkMenuOpenSubCmdWindowBeside @ 0x008e_46b0 => extern "C" fn tk_menu_open_sub_cmd_window_beside(id: u32, texts: u32, values: u32, n: u32, b44: u32) -> u32 {
        tk::tk_menu_open_sub_cmd_window_beside(&mut mem(), [id, texts, values, n, b44], &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMsSetSaveParam(chr)`.
    TkMsSetSaveParam @ 0x008c_3070 => extern "C" fn tk_ms_set_save_param(chr: u32) -> u32 {
        tk::tk_ms_set_save_param(&mut mem(), chr, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuKaizou_AddAbility(weapon, ability)`.
    MenuKaizou_AddAbility @ 0x008d_5680 => extern "C" fn menu_kaizou_add_ability(weapon: u32, ability: u32) -> u32 {
        tk::menu_kaizou_add_ability(&mut mem(), weapon, ability, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMsEventEffectRead(id)`.
    TkMsEventEffectRead @ 0x0087_2470 => extern "C" fn tk_ms_event_effect_read(id: u32) -> u32 {
        tk::tk_ms_event_effect_read(&mut mem(), id, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkResetClip()`.
    TkResetClip @ 0x0088_c710 => extern "C" fn tk_reset_clip() -> u32 {
        tk::tk_reset_clip(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuMain_Reset()`.
    MenuMain_Reset @ 0x008a_a570 => extern "C" fn menu_main_reset() -> u32 {
        tk::menu_main_reset(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_DrawRows(window, _, first, x, y)`: nine item rows.
    MenuItem_DrawRows @ 0x008c_99f0 => extern "C" fn menu_item_draw_rows(window: u32, _unused: u32, first: u32, x: f32, y: f32) {
        tk::menu_item_draw_rows(&mut mem(), window, first, [x, y], &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuNameEntry_Commit()`: the typed name written.
    MenuNameEntry_Commit @ 0x008c_4e60 => extern "C" fn menu_name_entry_commit() -> u32 {
        tk::menu_name_entry_commit(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkHideAttachedChrs(actor, hide)`: returns `actor`.
    TkHideAttachedChrs @ 0x0087_2380 => extern "C" fn tk_hide_attached_chrs(actor: u32, hide: u32) -> u32 {
        tk::tk_hide_attached_chrs(&mut mem(), actor, hide, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuMain_BusyMask()`.
    MenuMain_BusyMask @ 0x008a_a610 => extern "C" fn menu_main_busy_mask() -> u32 {
        tk::menu_main_busy_mask(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_OpenSubWindow4()`: the window.
    MenuWShop_OpenSubWindow4 @ 0x008d_83e0 => extern "C" fn menu_wshop_open_sub_window4() -> u32 {
        tk::menu_wshop_open_sub_window4(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuShop_WeaponPrice(e)`.
    MenuShop_WeaponPrice @ 0x008d_9370 => extern "C" fn menu_shop_weapon_price(e: u32) -> i32 {
        tk::menu_shop_weapon_price(&mut mem(), e, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuShop_WeaponSellPrice(e)`.
    MenuShop_WeaponSellPrice @ 0x008d_9410 => extern "C" fn menu_shop_weapon_sell_price(e: u32) -> u32 {
        tk::menu_shop_weapon_sell_price(&mut mem(), e, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuEquip_IsSelectable(id, any)`.
    MenuEquip_IsSelectable @ 0x008d_5750 => extern "C" fn menu_equip_is_selectable(id: u32, any: u32) -> u32 {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return 0;
        };
        tk::menu_equip_is_selectable(&mut m, id, any, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuItemHealAmount(item, player, mp)`.
    TkMenuItemHealAmount_008c69a0 @ 0x008c_69a0 => extern "C" fn tk_menu_item_heal_amount(item: u32, player: u32, mp: u32) -> i32 {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return 0;
        };
        tk::tk_menu_item_heal_amount(&mut m, [item, player, mp], local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `DrawCrossMenuSummonPlateNoHPMPClut(chr, x, y, colour)`: the font call's eax.
    DrawCrossMenuSummonPlateNoHPMPClut @ 0x008e_7110 => extern "C" fn draw_cross_menu_summon_plate_no_hpmp(chr: u32, x: u32, y: u32, colour: u32) -> u32 {
        tk::draw_cross_menu_summon_plate_no_hpmp(&mut mem(), chr, [f32::from_bits(x), f32::from_bits(y)], colour, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuNameEntry_DeleteKey()`.
    MenuNameEntry_DeleteKey @ 0x008c_44d0 => extern "C" fn menu_name_entry_delete_key() {
        tk::menu_name_entry_delete_key(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkFindLegendWeapon_008c30e0(chr)`: the weapon record, or 0.
    TkFindLegendWeapon_008c30e0 @ 0x008c_30e0 => extern "C" fn tk_find_legend_weapon(chr: u32) -> u32 {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return 0;
        };
        tk::tk_find_legend_weapon(&mut m, chr, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `DrawCrossMenuIconItemName(item, x, y, colour)`.
    DrawCrossMenuIconItemName @ 0x008e_6700 => extern "C" fn draw_cross_menu_icon_item_name(item: u32, x: u32, y: u32, colour: u32) -> u32 {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return 0;
        };
        tk::draw_cross_menu_icon_item_name(&mut m, item, [f32::from_bits(x), f32::from_bits(y)], colour, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `DrawCrossMenuIconWeaponName(id, x, y, colour)`.
    DrawCrossMenuIconWeaponName @ 0x008e_67f0 => extern "C" fn draw_cross_menu_icon_weapon_name(id: u32, x: u32, y: u32, colour: u32) {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return;
        };
        tk::draw_cross_menu_icon_weapon_name(&mut m, id, [x, y, colour], local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `DrawCrossMenuIconWeaponName2(e, x, y, colour)`.
    DrawCrossMenuIconWeaponName2 @ 0x008e_6970 => extern "C" fn draw_cross_menu_icon_weapon_name2(e: u32, x: u32, y: u32, colour: u32) {
        tk::draw_cross_menu_icon_weapon_name2(&mut mem(), e, [x, y, colour], &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMn2DrawGillPlate(x, y, w, h, gil)`.
    TkMn2DrawGillPlate @ 0x008c_0a20 => extern "C" fn tk_mn2_draw_gill_plate(x: u32, y: u32, w: u32, h: u32, gil: u32) -> u32 {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return 0;
        };
        let xywh = [x, y, w, h].map(f32::from_bits);
        tk::tk_mn2_draw_gill_plate(&mut m, xywh, gil, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuListWindowCtrl(w)`.
    TkMenuListWindowCtrl_008b44b0 @ 0x008b_44b0 => extern "C" fn tk_menu_list_window_ctrl(w: u32) {
        tk::tk_menu_list_window_ctrl(&mut mem(), w, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuMain_RestoreVram()`.
    MenuMain_RestoreVram @ 0x008a_a690 => extern "C" fn menu_main_restore_vram() -> u32 {
        tk::menu_main_restore_vram(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuMain_SaveVram()`.
    MenuMain_SaveVram @ 0x008a_ab90 => extern "C" fn menu_main_save_vram() -> u32 {
        tk::menu_main_save_vram(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuAbility_ApplyHeal(target, user)`.
    MenuAbility_ApplyHeal_008c6d00 @ 0x008c_6d00 => extern "C" fn menu_ability_apply_heal(target: u32, user: u32) -> u32 {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return 0;
        };
        tk::menu_ability_apply_heal(&mut m, target, user, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2DrawCommonPlateL(plate)`.
    TkMn2DrawCommonPlateL @ 0x008b_f9b0 => extern "C" fn tk_mn2_draw_common_plate_l(plate: u32) -> u32 {
        tk::tk_mn2_draw_common_plate_l(&mut mem(), plate, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_OpenWindow3()`: the window.
    MenuWShop_OpenWindow3 @ 0x008d_a9c0 => extern "C" fn menu_wshop_open_window3() -> u32 {
        tk::menu_wshop_open_window3(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuStatus_OpenMainWindow()`: the window.
    MenuStatus_OpenMainWindow @ 0x008d_3100 => extern "C" fn menu_status_open_main_window() -> u32 {
        tk::menu_status_open_main_window(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSummon_OpenWindow()`.
    MenuSummon_OpenWindow @ 0x008c_d1b0 => extern "C" fn menu_summon_open_window() -> u32 {
        tk::menu_summon_open_window(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkRefreshHDDMode()`.
    TkRefreshHDDMode @ 0x0088_e930 => extern "C" fn tk_refresh_hdd_mode() -> u32 {
        tk::tk_refresh_hdd_mode(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `SsSetProjectBase(path)`.
    SsSetProjectBase @ 0x0088_7a90 => extern "C" fn ss_set_project_base(path: u32) -> u32 {
        tk::ss_set_project_base(&mut mem(), path, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2StringWidth(text)`.
    TkMn2StringWidth @ 0x008c_2cb0 => extern "C" fn tk_mn2_string_width(text: u32) -> u32 {
        tk::tk_mn2_string_width(&mut mem(), text, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_LoadTargetKeys(target)`: void (the parser's `eax` is unread).
    MenuItem_LoadTargetKeys @ 0x008c_5320 => extern "C" fn menu_item_load_target_keys(target: i32) {
        tk::menu_item_load_target_keys(&mut mem(), target, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuSummon_RefreshGrowMasks(chr)`.
    MenuSummon_RefreshGrowMasks @ 0x008c_d3b0 => extern "C" fn menu_summon_refresh_grow_masks(chr: u32) -> u32 {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return 0;
        };
        tk::menu_summon_refresh_grow_masks(&mut m, chr, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `BtlEnd_GetSaveNextAP(chr)`.
    BtlEnd_GetSaveNextAP @ 0x008b_da00 => extern "C" fn btl_end_get_save_next_ap(chr: u32) -> u32 {
        tk::btl_end_get_save_next_ap(&mut mem(), chr, &mut |_, f, a| call_ptr(f, a))
    }
    /// `BtlEnd_AddGil(gil)`.
    BtlEnd_AddGil @ 0x008b_a3a0 => extern "C" fn btl_end_add_gil(gil: u32) -> u32 {
        tk::btl_end_add_gil(&mut mem(), gil, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2DrawKickPacket()`.
    TkMn2DrawKickPacket @ 0x008c_0c40 => extern "C" fn tk_mn2_draw_kick_packet() -> u32 {
        tk::tk_mn2_draw_kick_packet(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkSyncVDI()`.
    TkSyncVDI @ 0x0088_7f20 => extern "C" fn tk_sync_vdi() -> u32 {
        tk::tk_sync_vdi(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuRefreshPartyList()`.
    TkMenuRefreshPartyList @ 0x008a_ae10 => extern "C" fn tk_menu_refresh_party_list() -> u32 {
        tk::tk_menu_refresh_party_list(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuMain_RemoveAllMenus()`.
    MenuMain_RemoveAllMenus @ 0x008a_add0 => extern "C" fn menu_main_remove_all_menus() -> u32 {
        tk::menu_main_remove_all_menus(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkChPatchM339(chr)`.
    TkChPatchM339 @ 0x0082_ba00 => extern "C" fn tk_ch_patch_m339(chr: u32) -> u32 {
        tk::tk_ch_patch_m339(&mut mem(), chr, &mut |_, f, a| call_ptr(f, a))
    }
    /// `Pad_StickToByte(axis)`: the byte in `al`, all of `eax` as the original.
    Pad_StickToByte @ 0x0088_9aa0 => extern "C" fn pad_stick_to_byte(axis: u32) -> u32 {
        tk::pad_stick_to_byte(&mut mem(), axis)
    }
    /// `TkMenuStartScene(scene, group)`.
    TkMenuStartScene @ 0x00a7_c880 => extern "C" fn tk_menu_start_scene(scene: u32, group: u32) -> u32 {
        tk::tk_menu_start_scene(&mut mem(), scene, group, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSave_McWrite(fd, buf, size)`.
    MenuSave_McWrite @ 0x0088_8570 => extern "C" fn menu_save_mc_write(fd: u32, buf: u32, size: u32) -> u32 {
        tk::menu_save_mc_write(&mut mem(), [fd, buf, size], &mut |_, f, a| call_ptr(f, a))
    }
    /// `HelpMem_Free(size)`.
    HelpMem_Free @ 0x0077_4aa0 => extern "C" fn help_mem_free(size: i32) -> u32 {
        tk::help_mem_free(&mut mem(), size, &mut |_, f, a| call_ptr(f, a))
    }
    /// `InitSysModuleEncoding()`.
    InitSysModuleEncoding @ 0x0064_6910 => extern "C" fn init_sys_module_encoding() -> u32 {
        tk::init_sys_module_encoding(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkInitSyncV()`.
    TkInitSyncV @ 0x0088_7e30 => extern "C" fn tk_init_sync_v() -> u32 {
        tk::tk_init_sync_v(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuTuto_RestoreNames()`.
    MenuTuto_RestoreNames @ 0x008d_b7e0 => extern "C" fn menu_tuto_restore_names() -> u32 {
        tk::menu_tuto_restore_names(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_OpenSelWindow()`: the window.
    MenuWShop_OpenSelWindow @ 0x008d_a8f0 => extern "C" fn menu_wshop_open_sel_window() -> u32 {
        tk::menu_wshop_open_sel_window(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `Pad_CheckPressMode(port, slot, ret, status)`.
    Pad_CheckPressMode @ 0x0088_9510 => extern "C" fn pad_check_press_mode(port: u32, slot: u32, ret: u32, status: u32) -> u32 {
        tk::pad_check_press_mode(&mut mem(), [port, slot, ret, status], &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuMain_SaveFrameBuffer()`.
    MenuMain_SaveFrameBuffer @ 0x008a_ac80 => extern "C" fn menu_main_save_frame_buffer() -> u32 {
        tk::menu_main_save_frame_buffer(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkRefreshHdd()`.
    TkRefreshHdd @ 0x0087_24a0 => extern "C" fn tk_refresh_hdd() -> u32 {
        tk::tk_refresh_hdd(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `BtlEnd_StepGil()`: 1 while gil remains.
    BtlEnd_StepGil @ 0x008b_a590 => extern "C" fn btl_end_step_gil() -> u32 {
        tk::btl_end_step_gil(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkInitWork()`.
    TkInitWork @ 0x0088_e570 => extern "C" fn tk_init_work() -> u32 {
        tk::tk_init_work(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_OpenListWindow()`: the window.
    MenuWShop_OpenListWindow @ 0x008d_8260 => extern "C" fn menu_wshop_open_list_window() -> u32 {
        tk::menu_wshop_open_list_window(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_OpenSubWindow1()`.
    MenuWShop_OpenSubWindow1 @ 0x008d_8370 => extern "C" fn menu_wshop_open_sub_window1() -> u32 {
        tk::menu_wshop_open_sub_window1(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSummon_OpenStatusWindow()`.
    MenuSummon_OpenStatusWindow_008cd140 @ 0x008c_d140 => extern "C" fn menu_summon_open_status_window() -> u32 {
        tk::menu_summon_open_status_window(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `BtlEnd_SelectedWeapon()`: the record.
    BtlEnd_SelectedWeapon @ 0x008b_da80 => extern "C" fn btl_end_selected_weapon() -> u32 {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return 0;
        };
        tk::btl_end_selected_weapon(&mut m, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuKaizou_OpenWindow()`.
    MenuKaizou_OpenWindow @ 0x008d_6d60 => extern "C" fn menu_kaizou_open_window() -> u32 {
        tk::menu_kaizou_open_window(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `BtlEnd_InParty(chr)`.
    BtlEnd_InParty @ 0x008b_9ec0 => extern "C" fn btl_end_in_party(chr: u32) -> u32 {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return 0;
        };
        tk::btl_end_in_party(&mut m, chr, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_ClampSubWindow1()`: void (its `eax` is unread; with no
    /// window the original leaves it alone).
    MenuWShop_ClampSubWindow1 @ 0x008d_8450 => extern "C" fn menu_wshop_clamp_sub_window1() {
        tk::menu_wshop_clamp_sub_window1(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuSave_PackSave(out, save, slot)`: a save file's header and data.
    MenuSave_PackSave @ 0x008b_3e60 => extern "C" fn menu_save_pack_save(out: u32, save: u32, slot: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the byte argument
        tk::menu_save_pack_save(&mut mem(), out, save, slot as u8, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2WeaponBuySellWindow(x, y, weapon, amount, owned)`: the last call's eax.
    TkMn2WeaponBuySellWindow @ 0x008c_2d00 => extern "C" fn tk_mn2_weapon_buy_sell_window(x: u32, y: u32, weapon: u32, amount: i32, owned: i32) -> u32 {
        tk::tk_mn2_weapon_buy_sell_window(&mut mem(), [f32::from_bits(x), f32::from_bits(y)], weapon, amount, owned, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2BuySellWindow(x, y, item, price, owned_count, amount, gil)`: the last call's eax.
    TkMn2BuySellWindow @ 0x008b_f170 => extern "C" fn tk_mn2_buy_sell_window(x: u32, y: u32, item: u32, price: u32, owned_count: u32, amount: u32, gil: u32) -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        let width = frame.at();
        tk::tk_mn2_buy_sell_window(&mut mem(), [f32::from_bits(x), f32::from_bits(y)], [item, price, owned_count, amount, gil], width, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkSetLegendAbility(chr, level)`: 1 when the abilities were set.
    TkSetLegendAbility @ 0x008c_3170 => extern "C" fn tk_set_legend_ability(chr: u32, level: u32) -> u32 {
        tk::tk_set_legend_ability(&mut mem(), chr, level, &mut |_, f, a| call_ptr(f, a))
    }
    /// `AtelOpenChoiceWindow(worker, args, stack, mode)`: `Native_MsgSelect`'s eax.
    AtelOpenChoiceWindow @ 0x0086_0b00 => extern "C" fn atel_open_choice_window(worker: u32, args: u32, stack: u32, mode: u32) -> u32 {
        msg::atel_open_choice_window(&mut mem(), [worker, args, stack], mode, &mut |_, f, a| call_ptr(f, a))
    }
    /// `Math_SegmentNearestT(a, b, c)`: the parameter in `ST0`.
    Math_SegmentNearestT @ 0x0088_a040 => extern "C" fn math_segment_nearest_t(a: u32, b: u32, c: u32) -> f32 {
        ffx_menu::math::bridge::math_segment_nearest_t(&mut mem(), a, b, c)
    }
    /// `Math_AngleStepMode(a, target, step, mode)`: in `ST0`.
    Math_AngleStepMode @ 0x0088_b610 => extern "C" fn math_angle_step_mode(a: f32, target: f32, step: f32, mode: i32) -> f32 {
        tk::math_angle_step_mode(a, target, step, mode)
    }
    /// `toGetStatusWeaponMesPos()`: the caption end, an integer.
    toGetStatusWeaponMesPos @ 0x008d_54a0 => extern "C" fn to_get_status_weapon_mes_pos() -> u32 {
        let mut frame = super::StackFrame::<2>::new();
        let base = frame.at();
        let widths = [base, base.wrapping_add(4)];
        tk::to_get_status_weapon_mes_pos(&mut mem(), widths, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `Pad_ReadCurrDevice(a, b, work, pad)`: `work`.
    Pad_ReadCurrDevice @ 0x0088_98a0 => extern "C" fn pad_read_curr_device(_a: u32, _b: u32, work: u32, pad: u32) -> u32 {
        tk::pad_read_curr_device(&mut mem(), work, pad, &mut |_, f, a| call_ptr(f, a))
    }
    /// `Pad_ResetWork(work)`: `Pad_ResetAnalogAt0C`'s eax.
    Pad_ResetWork @ 0x0088_8970 => extern "C" fn pad_reset_work(work: u32) -> u32 {
        tk::pad_reset_work(&mut mem(), work, &mut |_, f, a| call_ptr(f, a))
    }
    /// `Pad_ClearAnalogWork(a, b)`: the work + 0xd0.
    Pad_ClearAnalogWork @ 0x0088_89f0 => extern "C" fn pad_clear_analog_work(a: u32, b: u32) -> u32 {
        tk::pad_clear_analog_work(&mut mem(), a, b, &mut |_, f, a| call_ptr(f, a))
    }
    /// `AtelMenuSaveMes_00860780(id)`: menu text 3's name.
    AtelMenuSaveMes_00860780 @ 0x0086_0780 => extern "C" fn atel_menu_save_mes_00860780(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 3, false, id)
    }
    /// `TOMenuGetArmsHelp(id)`: menu text 5's help.
    TOMenuGetArmsHelp @ 0x008b_ea50 => extern "C" fn to_menu_get_arms_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 5, true, id)
    }
    /// `TOMenuGetArmsMes(id)`: menu text 5's name.
    TOMenuGetArmsMes @ 0x008b_ea70 => extern "C" fn to_menu_get_arms_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 5, false, id)
    }
    /// `TOMenuGetBuildHelp(id)`: menu text 6's help.
    TOMenuGetBuildHelp @ 0x008b_ea90 => extern "C" fn to_menu_get_build_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 6, true, id)
    }
    /// `TOMenuGetBuildMes(id)`: menu text 6's name.
    TOMenuGetBuildMes @ 0x008b_eab0 => extern "C" fn to_menu_get_build_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 6, false, id)
    }
    /// `TOMenuGetConfigHelp(id)`: menu text 7's help.
    TOMenuGetConfigHelp @ 0x008b_ead0 => extern "C" fn to_menu_get_config_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 7, true, id)
    }
    /// `TOMenuGetConfigMes(id)`: menu text 7's name.
    TOMenuGetConfigMes @ 0x008b_eaf0 => extern "C" fn to_menu_get_config_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 7, false, id)
    }
    /// `TOMenuGetItemHelp(id)`: menu text 8's help.
    TOMenuGetItemHelp @ 0x008b_ec90 => extern "C" fn to_menu_get_item_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 8, true, id)
    }
    /// `TOMenuGetItemMes(id)`: menu text 8's name.
    TOMenuGetItemMes @ 0x008b_ecb0 => extern "C" fn to_menu_get_item_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 8, false, id)
    }
    /// `TOMenuGetMainHelp(id)`: menu text 9's help.
    TOMenuGetMainHelp @ 0x008b_ecd0 => extern "C" fn to_menu_get_main_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 9, true, id)
    }
    /// `TOMenuGetMainMes(id)`: menu text 9's name.
    TOMenuGetMainMes @ 0x008b_ecf0 => extern "C" fn to_menu_get_main_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 9, false, id)
    }
    /// `TOMenuGetMenu1Help(id)`: menu text 1's help.
    TOMenuGetMenu1Help @ 0x008b_ed10 => extern "C" fn to_menu_get_menu1_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 1, true, id)
    }
    /// `TOMenuGetMenu1Mes(id)`: menu text 1's name.
    TOMenuGetMenu1Mes @ 0x008b_ed30 => extern "C" fn to_menu_get_menu1_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 1, false, id)
    }
    /// `TOMenuGetMenu2Help(id)`: menu text 2's help.
    TOMenuGetMenu2Help @ 0x008b_ed50 => extern "C" fn to_menu_get_menu2_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 2, true, id)
    }
    /// `TOMenuGetMenu2Mes(id)`: menu text 2's name.
    TOMenuGetMenu2Mes @ 0x008b_ed70 => extern "C" fn to_menu_get_menu2_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 2, false, id)
    }
    /// `TOMenuGetNameHelp(id)`: menu text 10's help.
    TOMenuGetNameHelp @ 0x008b_edb0 => extern "C" fn to_menu_get_name_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 10, true, id)
    }
    /// `TOMenuGetNameMes(id)`: menu text 10's name.
    TOMenuGetNameMes @ 0x008b_edd0 => extern "C" fn to_menu_get_name_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 10, false, id)
    }
    /// `TOMenuGetSaveHelp(id)`: menu text 3's help.
    TOMenuGetSaveHelp @ 0x008b_ee10 => extern "C" fn to_menu_get_save_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 3, true, id)
    }
    // TOMenuGetSaveMes: hooked by ported/system.rs (system round 37, the same body).
    // TOMenuGetStatusHelp: hooked by ported/system.rs (system round 37, the same body).
    /// `TOMenuGetStatusMes(id)`: menu text 11's name.
    TOMenuGetStatusMes @ 0x008b_ee70 => extern "C" fn to_menu_get_status_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 11, false, id)
    }
    // TOMenuGetSummonHelp: hooked by ported/system.rs (system round 37, the same body).
    /// `TOMenuGetSummonMes(id)`: menu text 12's name.
    TOMenuGetSummonMes @ 0x008b_eeb0 => extern "C" fn to_menu_get_summon_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 12, false, id)
    }
    /// `TkMn2ArmsMes(id)`: menu text 5's name.
    TkMn2ArmsMes @ 0x008b_f150 => extern "C" fn tk_mn2_arms_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 5, false, id)
    }
    /// `TkMenuArmsHelp(id)`: menu text 5's help.
    TkMenuArmsHelp @ 0x008d_c8f0 => extern "C" fn tk_menu_arms_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 5, true, id)
    }
    /// `TkMenuArmsMes(id)`: menu text 5's name.
    TkMenuArmsMes @ 0x008d_c910 => extern "C" fn tk_menu_arms_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 5, false, id)
    }
    /// `TkMenuBuildHelp(id)`: menu text 6's help.
    TkMenuBuildHelp @ 0x008d_c930 => extern "C" fn tk_menu_build_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 6, true, id)
    }
    /// `TkMenuBuildMes(id)`: menu text 6's name.
    TkMenuBuildMes @ 0x008d_c950 => extern "C" fn tk_menu_build_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 6, false, id)
    }
    /// `TkMenuConfigMes(id)`: menu text 7's name.
    TkMenuConfigMes @ 0x008d_c9b0 => extern "C" fn tk_menu_config_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 7, false, id)
    }
    /// `TkMenuItemHelp(id)`: menu text 8's help.
    TkMenuItemHelp @ 0x008d_df20 => extern "C" fn tk_menu_item_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 8, true, id)
    }
    /// `TkMenuItemMes(id)`: menu text 8's name.
    TkMenuItemMes @ 0x008d_df40 => extern "C" fn tk_menu_item_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 8, false, id)
    }
    /// `TkMenuMainHelp(id)`: menu text 9's help.
    TkMenuMainHelp @ 0x008d_df60 => extern "C" fn tk_menu_main_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 9, true, id)
    }
    /// `TkMenuMainMes(id)`: menu text 9's name.
    TkMenuMainMes @ 0x008d_df80 => extern "C" fn tk_menu_main_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 9, false, id)
    }
    /// `TkMenuMenu1Help(id)`: menu text 1's help.
    TkMenuMenu1Help @ 0x008d_dfa0 => extern "C" fn tk_menu_menu1_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 1, true, id)
    }
    /// `TkMenuMenu1Mes(id)`: menu text 1's name.
    TkMenuMenu1Mes @ 0x008d_dfc0 => extern "C" fn tk_menu_menu1_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 1, false, id)
    }
    /// `TkMenuMenu2Help(id)`: menu text 2's help.
    TkMenuMenu2Help @ 0x008d_dfe0 => extern "C" fn tk_menu_menu2_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 2, true, id)
    }
    /// `TkMenuMenu2Mes(id)`: menu text 2's name.
    TkMenuMenu2Mes @ 0x008d_e000 => extern "C" fn tk_menu_menu2_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 2, false, id)
    }
    /// `TkMenuNameHelp(id)`: menu text 10's help.
    TkMenuNameHelp @ 0x008d_e020 => extern "C" fn tk_menu_name_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 10, true, id)
    }
    /// `TkMenuNameMes(id)`: menu text 10's name.
    TkMenuNameMes @ 0x008d_e040 => extern "C" fn tk_menu_name_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 10, false, id)
    }
    /// `TkMenuSaveHelp(id)`: menu text 3's help.
    TkMenuSaveHelp @ 0x008d_e060 => extern "C" fn tk_menu_save_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 3, true, id)
    }
    /// `TkMenuSaveMes(id)`: menu text 3's name.
    TkMenuSaveMes @ 0x008d_e080 => extern "C" fn tk_menu_save_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 3, false, id)
    }
    /// `TkMenuStatusHelp(id)`: menu text 11's help.
    TkMenuStatusHelp @ 0x008d_e120 => extern "C" fn tk_menu_status_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 11, true, id)
    }
    /// `TkMenuStatusMes(id)`: menu text 11's name.
    TkMenuStatusMes @ 0x008d_e140 => extern "C" fn tk_menu_status_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 11, false, id)
    }
    /// `TkMenuSummonHelp(id)`: menu text 12's help.
    TkMenuSummonHelp @ 0x008d_e160 => extern "C" fn tk_menu_summon_help(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 12, true, id)
    }
    /// `TkMenuSummonMes(id)`: menu text 12's name.
    TkMenuSummonMes @ 0x008d_e180 => extern "C" fn tk_menu_summon_mes(id: u32) -> u32 {
        tk::menu_text_of(&mem(), 12, false, id)
    }
    /// `TkMenuDraw1612Width(text)`: the text's width.
    TkMenuDraw1612Width @ 0x008d_c9e0 => extern "C" fn tk_menu_draw1612_width(text: u32) -> u32 {
        tk::tk_menu_draw1612_width(&mut mem(), text, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2TransCharFace(summon)`: the upload's eax.
    TkMn2TransCharFace @ 0x008c_2ce0 => extern "C" fn tk_mn2_trans_char_face(summon: u32) -> u32 {
        tk::tk_mn2_trans_char_face(&mut mem(), summon, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuNameEntry_CommitDefault0b()`: `MenuNameEntry_Commit`'s eax.
    MenuNameEntry_CommitDefault0b @ 0x008c_5370 => extern "C" fn menu_name_entry_commit_default0b() -> u32 {
        tk::menu_name_entry_commit_default0b(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2DrawMesL(x, y, text, colour)`: the new cursor.
    TkMn2DrawMesL @ 0x008c_1060 => extern "C" fn tk_mn2_draw_mes_l(x: u32, y: u32, text: u32, colour: u32) -> u32 {
        tk::tk_mn2_draw_mes_l(&mut mem(), [x, y, text, colour], &mut |_, f, a| call_ptr(f, a))
    }
    /// `debugSetRikkuName()`: the name minus the text (the original's eax).
    debugSetRikkuName @ 0x0086_0a60 => extern "C" fn debug_set_rikku_name() -> u32 {
        tk::debug_set_rikku_name(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_RefreshHelp_008c7650(window)`: `TkMenuSetHelpMessage`'s eax
    /// (0 when it is not called; no caller reads it then).
    MenuItem_RefreshHelp_008c7650 @ 0x008c_7650 => extern "C" fn menu_item_refresh_help(window: u32) -> u32 {
        tk::menu_item_refresh_help(&mut mem(), window, &mut |_, f, a| call_ptr(f, a)).unwrap_or(0)
    }
    /// `MenuSave_McSync(mode, cmd, result)`: the sync's result.
    MenuSave_McSync @ 0x0088_84a0 => extern "C" fn menu_save_mc_sync(mode: u32, cmd: u32, result: u32) -> u32 {
        tk::menu_save_mc_sync(&mut mem(), [mode, cmd, result], &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuShop_SellWindowCtrl(window)`: void (the dispatcher never reads eax).
    MenuShop_SellWindowCtrl @ 0x008d_9e20 => extern "C" fn menu_shop_sell_window_ctrl(window: u32) {
        tk::menu_shop_sell_window_ctrl(&mut mem(), window, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuShop_BuyWindowCtrl(window)`: void (the dispatcher never reads eax).
    MenuShop_BuyWindowCtrl @ 0x008d_9bd0 => extern "C" fn menu_shop_buy_window_ctrl(window: u32) {
        tk::menu_shop_buy_window_ctrl(&mut mem(), window, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuWShop_BuyConfirmCtrl(window)`: void (the dispatcher never reads eax).
    MenuWShop_BuyConfirmCtrl @ 0x008d_7690 => extern "C" fn menu_wshop_buy_confirm_ctrl(window: u32) {
        tk::menu_wshop_buy_confirm_ctrl(&mut mem(), window, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuWShop_SellConfirmCtrl(window)`: void (the dispatcher never reads eax).
    MenuWShop_SellConfirmCtrl @ 0x008d_7860 => extern "C" fn menu_wshop_sell_confirm_ctrl(window: u32) {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return;
        };
        tk::menu_wshop_sell_confirm_ctrl(&mut m, window, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMn2DrawSelItemsHelp(window)`: ends in `TkMenuSetHelpMessage`'s tail jump (void).
    TkMn2DrawSelItemsHelp @ 0x008d_1a20 => extern "C" fn tk_mn2_draw_sel_items_help(window: u32) {
        tk::tk_mn2_draw_sel_items_help(&mut mem(), window, &mut |_, f, a| call_ptr(f, a));
    }
    /// `HelpMem_Get(reset, size)`: the block.
    HelpMem_Get @ 0x0077_4c00 => extern "C" fn help_mem_get(reset: u32, size: i32) -> u32 {
        tk::help_mem_get(&mut mem(), reset, size, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuExchangePlayerPos(a, b)`: void (ends in `MenuParty_RefreshMask`).
    TkMenuExchangePlayerPos @ 0x008a_9720 => extern "C" fn tk_menu_exchange_player_pos(a: i32, b: i32) {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return;
        };
        tk::tk_menu_exchange_player_pos(&mut m, a, b, local, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuItem_OpenTargetWindow(aeons, a, b)`: `TkMenuMainRegistWindow`'s result.
    MenuItem_OpenTargetWindow_008c9240 @ 0x008c_9240 => extern "C" fn menu_item_open_target_window(aeons: u32, a: u32, b: u32) -> u32 {
        tk::menu_item_open_target_window(&mut mem(), aeons, a, b, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2ItemRequireWindow(x, y, item, need)`: the eax the original leaves.
    TkMn2ItemRequireWindow @ 0x008c_1c90 => extern "C" fn tk_mn2_item_require_window(x: i32, y: i32, item: u32, need: i32) -> u32 {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return 0;
        };
        tk::tk_mn2_item_require_window(&mut m, [x, y], item, need, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSave_KindText(kind, base)`: the text.
    MenuSave_KindText @ 0x008b_5560 => extern "C" fn menu_save_kind_text(kind: u32, base: u32) -> u32 {
        tk::menu_save_kind_text(&mut mem(), kind, base, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_EquipBought(id)`: `TkMsSetSaveParam`'s result.
    MenuWShop_EquipBought @ 0x008d_9120 => extern "C" fn menu_wshop_equip_bought(id: u32) -> u32 {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return 0;
        };
        tk::menu_wshop_equip_bought(&mut m, id, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuDrawChrNameOrCommand(chr, x, y, colour)`: the callee's result.
    MenuDrawChrNameOrCommand @ 0x008c_0ec0 => extern "C" fn menu_draw_chr_name_or_command(chr: u32, x: u32, y: u32, colour: u32) -> u32 {
        tk::menu_draw_chr_name_or_command(&mut mem(), [chr, x, y, colour], &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuDrawLayer(layer)`: void.
    TkMenuDrawLayer @ 0x008a_9690 => extern "C" fn tk_menu_draw_layer(layer: i32) {
        tk::tk_menu_draw_layer(&mut mem(), layer, &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMenuCloseAll()`: `TOSeqCtrl`'s result.
    TkMenuCloseAll @ 0x008a_a200 => extern "C" fn tk_menu_close_all() -> u32 {
        tk::tk_menu_close_all(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSaveLoad_SyncToggle()`: 1 when done or failed, 0 while waiting.
    MenuSaveLoad_SyncToggle @ 0x008b_4da0 => extern "C" fn menu_save_load_sync_toggle() -> u32 {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return 0;
        };
        tk::menu_save_load_sync_toggle(&mut m, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSummon_GrowStat(slot, chr)`: `TkMsSetSaveParam`'s result.
    MenuSummon_GrowStat @ 0x008c_d240 => extern "C" fn menu_summon_grow_stat(slot: i32, chr: u32) -> u32 {
        tk::menu_summon_grow_stat(&mut mem(), slot, chr, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkInitStartMap()`: void.
    TkInitStartMap @ 0x0088_e880 => extern "C" fn tk_init_start_map() {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return;
        };
        tk::tk_init_start_map(&mut m, local, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuSaveLoad_OpenWindow(mode)`: the window.
    MenuSaveLoad_OpenWindow @ 0x008b_5000 => extern "C" fn menu_save_load_open_window(mode: u32) -> u32 {
        tk::menu_save_load_open_window(&mut mem(), mode, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_EquipLoss(id)`: bit 0 HP, bit 1 MP lost.
    MenuWShop_EquipLoss @ 0x008d_8520 => extern "C" fn menu_wshop_equip_loss(id: u32) -> u32 {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return 0;
        };
        tk::menu_wshop_equip_loss(&mut m, id, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_OpenCommandWindow()`: `MenuWShop_OpenSubWindow4`'s result.
    MenuWShop_OpenCommandWindow @ 0x008d_8110 => extern "C" fn menu_wshop_open_command_window() -> u32 {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return 0;
        };
        tk::menu_wshop_open_command_window(&mut m, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_Start()`: the command window's result.
    MenuWShop_Start @ 0x008d_a7a0 => extern "C" fn menu_wshop_start() -> u32 {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return 0;
        };
        tk::menu_wshop_start(&mut m, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkCdReadWithMallocWithMessage(dir, file)`: the buffer.
    TkCdReadWithMallocWithMessage @ 0x0088_d2c0 => extern "C" fn tk_cd_read_with_malloc_with_message(dir: u32, file: i32) -> u32 {
        tk::tk_cd_read_with_malloc_with_message(&mut mem(), dir, file, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSave_DicText(out, _, _, index)`: void.
    MenuSave_DicText @ 0x008b_43e0 => extern "C" fn menu_save_dic_text(out: u32, _b: u32, _c: u32, index: i32) {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return;
        };
        tk::menu_save_dic_text(&mut m, out, index, local, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuItem_DrawCellCursor(_, x, y)`: void.
    MenuItem_DrawCellCursor @ 0x008c_4a90 => extern "C" fn menu_item_draw_cell_cursor(_a: u32, x: i32, y: i32) {
        tk::menu_item_draw_cell_cursor(&mut mem(), [x, y], &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuItem_UseOnTargets(target)`: void.
    MenuItem_UseOnTargets @ 0x008c_8d50 => extern "C" fn menu_item_use_on_targets(target: u32) {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return;
        };
        tk::menu_item_use_on_targets(&mut m, target, local, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuWeapon_DrawRow(window, x, y, row, fixed)`: void.
    MenuWeapon_DrawRow @ 0x008d_0170 => extern "C" fn menu_weapon_draw_row(window: u32, x: u32, y: u32, row: i32, fixed: i32) {
        let mut m = mem();
        let Some(local) = ffx_bridge::Host::callee_scratch(&mut m) else {
            return;
        };
        tk::menu_weapon_draw_row(&mut m, window, [f32::from_bits(x), f32::from_bits(y)], [row, fixed], local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `Pad_ActuatorIds(port, slot)`: `(large << 8) | small`.
    Pad_ActuatorIds @ 0x0088_8a70 => extern "C" fn pad_actuator_ids(port: u32, slot: u32) -> u32 {
        tk::pad_actuator_ids(&mut mem(), port, slot, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_DrawChrSmallPlate(x, y, chr)`: the plate's result.
    MenuItem_DrawChrSmallPlate @ 0x008c_1260 => extern "C" fn menu_item_draw_chr_small_plate(x: u32, y: u32, chr: u32) -> u32 {
        tk::menu_item_draw_chr_small_plate(&mut mem(), [f32::from_bits(x), f32::from_bits(y)], chr, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuConfigHelp(id)`: the help line.
    TkMenuConfigHelp @ 0x008d_c970 => extern "C" fn tk_menu_config_help(id: u32) -> u32 {
        tk::tk_menu_config_help(&mut mem(), id, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWeapon_DrawRows(window, fixed, first, _, y)`: void.
    MenuWeapon_DrawRows @ 0x008d_0250 => extern "C" fn menu_weapon_draw_rows(window: u32, fixed: i32, first: i32, _d: u32, y: i32) {
        tk::menu_weapon_draw_rows(&mut mem(), window, [fixed, first, y], &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuSave_BuildIconSys(n, b, index)`: void.
    MenuSave_BuildIconSys @ 0x008b_42e0 => extern "C" fn menu_save_build_icon_sys(n: u32, b: u32, index: u32) {
        let mut frame = super::StackFrame::<0x40>::new();
        let local = frame.at();
        tk::menu_save_build_icon_sys(&mut mem(), [n, b, index], local, &mut |_, f, a| call_ptr(f, a));
    }
    /// `BtlEnd_StepEntry(i)`: 1 in state 2 or 5.
    BtlEnd_StepEntry @ 0x008b_b690 => extern "C" fn btl_end_step_entry(i: u32) -> u32 {
        tk::btl_end_step_entry(&mut mem(), i, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuNameEntry_DrawCharFaceSlide(x, y, w, h, t, cur, next)`: the
    /// last call's result.
    MenuNameEntry_DrawCharFaceSlide @ 0x008d_33b0 => extern "C" fn menu_name_entry_draw_char_face_slide(x: u32, y: u32, w: u32, h: u32, t: i32, cur: u32, next: u32) -> u32 {
        tk::menu_name_entry_draw_face_slide(&mut mem(), [x, y, w, h], t, [cur, next], &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuNameEntry_DrawSummonFaceSlide(x, y, w, h, t, cur, next)`: the
    /// same body.
    MenuNameEntry_DrawSummonFaceSlide @ 0x008d_35a0 => extern "C" fn menu_name_entry_draw_summon_face_slide(x: u32, y: u32, w: u32, h: u32, t: i32, cur: u32, next: u32) -> u32 {
        tk::menu_name_entry_draw_face_slide(&mut mem(), [x, y, w, h], t, [cur, next], &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuPlayer_DrawFaceSlide(x, y, w, h, t, cur, next)`: the last
    /// call's result.
    MenuPlayer_DrawFaceSlide @ 0x008d_32b0 => extern "C" fn menu_player_draw_face_slide(x: u32, y: u32, w: u32, h: u32, t: i32, cur: u32, next: u32) -> u32 {
        tk::menu_player_draw_face_slide(&mut mem(), [x, y, w, h], t, [cur, next], &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuScene_KeyEase(key, frame, frames)`: in `ST0`.
    MenuScene_KeyEase @ 0x00a7_c6f0 => extern "C" fn menu_scene_key_ease(key: u32, frame: i32, frames: i32) -> f32 {
        tk::menu_scene_key_ease(&mut mem(), key, frame, frames, &mut |_, v| crt_x87(0x00b0_c3b4, v, None))
    }
    /// `MenuKaizou_DrawWeaponTitle(x, y, i)`: void.
    MenuKaizou_DrawWeaponTitle @ 0x008d_6660 => extern "C" fn menu_kaizou_draw_weapon_title(x: i32, y: i32, i: i32) {
        let mut frame = super::StackFrame::<2>::new();
        let local = frame.at();
        tk::menu_kaizou_draw_weapon_title(&mut mem(), [x, y, i], local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuKaizou_DrawRow(window, x, y, row, fixed)`: void.
    MenuKaizou_DrawRow @ 0x008d_6760 => extern "C" fn menu_kaizou_draw_row(window: u32, x: u32, y: u32, row: i32, fixed: i32) {
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::menu_kaizou_draw_row(&mut mem(), window, [f32::from_bits(x), f32::from_bits(y)], [row, fixed], local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `Pad_RepeatButton(work, _, i, now, prev, slot)`: the bits in `ax`.
    Pad_RepeatButton @ 0x0088_9980 => extern "C" fn pad_repeat_button(work: u32, _b: u32, i: u32, now: u32, prev: u32, slot: u32) -> u32 {
        tk::pad_repeat_button(&mut mem(), work, [i, now, prev, slot], &mut |_| input_get_cur_time())
    }
    /// `Pad_RepeatButtons(work, b, now, prev, slot)`: the bits in `ax`.
    Pad_RepeatButtons @ 0x0088_9940 => extern "C" fn pad_repeat_buttons(work: u32, b: u32, now: u32, prev: u32, slot: u32) -> u32 {
        tk::pad_repeat_buttons(&mut mem(), work, [b, now, prev, slot], &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuKaizou_DrawRows(window, fixed, first, _, y)`: void.
    MenuKaizou_DrawRows @ 0x008d_6560 => extern "C" fn menu_kaizou_draw_rows(window: u32, fixed: i32, first: i32, _d: u32, y: i32) {
        tk::menu_kaizou_draw_rows(&mut mem(), window, [fixed, first, y], &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `Math_SemicircleTable(out, n, r)`: `out`.
    Math_SemicircleTable @ 0x008c_ac70 => extern "C" fn math_semicircle_table(out: u32, n: i32, r: i32) -> u32 {
        // _CIsin and _CIacos through the C runtime's import slots
        tk::math_semicircle_table(&mut mem(), out, [n, r], &mut |_, sin, v| crt_x87(if sin { 0x00b0_c3b8 } else { 0x00b0_c3c4 }, v, None))
    }
    /// `MenuAbility_CommandTargetMask(cmd)`: 1 with aeons only, else 0.
    MenuAbility_CommandTargetMask_008c6bc0 @ 0x008c_6bc0 => extern "C" fn menu_ability_command_target_mask(cmd: u32) -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::menu_ability_command_target_mask(&mut mem(), cmd, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_SortInventory()`: void.
    MenuItem_SortInventory @ 0x008c_8fe0 => extern "C" fn menu_item_sort_inventory() {
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::menu_item_sort_inventory(&mut mem(), local, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuItem_ItemTargetMask(item)`: 1 with aeons only, else 0.
    MenuItem_ItemTargetMask_008c8a30 @ 0x008c_8a30 => extern "C" fn menu_item_item_target_mask(item: u32) -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::menu_item_item_target_mask(&mut mem(), item, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_DrawRow(window, x, y, row)`: void.
    MenuItem_DrawRow @ 0x008d_af50 => extern "C" fn menu_item_draw_row(window: u32, x: u32, y: u32, row: i32) {
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::menu_item_draw_row(&mut mem(), window, [f32::from_bits(x), f32::from_bits(y)], row, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `NameInputChrChange(chr)`: `Menu_ParseItemKeys`'s result.
    NameInputChrChange @ 0x008c_33f0 => extern "C" fn name_input_chr_change(chr: u32) -> u32 {
        tk::name_input_chr_change(&mut mem(), chr, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_DrawRowPairs(window, _, pair, x, y)`: ten row pairs.
    MenuItem_DrawRowPairs @ 0x008d_aea0 => extern "C" fn menu_item_draw_row_pairs(window: u32, _unused: u32, pair: u32, x: f32, y: f32) {
        tk::menu_item_draw_row_pairs(&mut mem(), window, pair, [x, y], &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `Pad_UpdateFrame(a, b)`: `Pad_ResetWork`'s or the last value's `eax`.
    Pad_UpdateFrame @ 0x0088_9790 => extern "C" fn pad_update_frame(a: u32, b: u32) -> u32 {
        tk::pad_update_frame(&mut mem(), a, b, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuDrawAll()`: `TkVU1SyncPath`'s result.
    TkMenuDrawAll @ 0x008a_a290 => extern "C" fn tk_menu_draw_all() -> u32 {
        tk::tk_menu_draw_all(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSelList_CursorX(window)`: in `ST0`.
    MenuSelList_CursorX @ 0x008d_1bc0 => extern "C" fn menu_sel_list_cursor_x(window: u32) -> f32 {
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::menu_sel_list_cursor_x(&mut mem(), window, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_SortGearByOwner()`: 1, or 0 when a search fails.
    MenuItem_SortGearByOwner @ 0x008c_9fb0 => extern "C" fn menu_item_sort_gear_by_owner() -> u32 {
        // the original's 0x400-byte owner buffer and MsGetSaveWeapon's out word
        let mut frame = super::StackFrame::<257>::new();
        let owners = frame.at();
        tk::menu_item_sort_gear_by_owner(&mut mem(), owners, owners.wrapping_add(0x400), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_SortGear_008c9c40(list, start, n)`.
    MenuItem_SortGear_008c9c40 @ 0x008c_9c40 => extern "C" fn menu_item_sort_gear(list: u32, start: u32, n: i32) {
        // the original's 0x1900-byte ability table and MsGetSaveWeapon's out word
        let mut frame = super::StackFrame::<0x641>::new();
        let table = frame.at();
        tk::menu_item_sort_gear(&mut mem(), list, start, n, table, table.wrapping_add(0x1900), &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMtCalcSpline3Segment_0088a500(out, index, t, points, position)`.
    TkMtCalcSpline3Segment_0088a500 @ 0x0088_a500 => extern "C" fn tk_mt_calc_spline3_segment(out: u32, index: u32, t: f32, points: u32, position: u32) {
        // the coefficients' frame copy
        let mut frame = super::StackFrame::<4>::new();
        let local = frame.at();
        tk::tk_mt_calc_spline3_segment(&mut mem(), out, index, t, points, position, local, &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMtCalcSplineSegment_0088abd0(out, index, t, points, position)`.
    TkMtCalcSplineSegment_0088abd0 @ 0x0088_abd0 => extern "C" fn tk_mt_calc_spline_segment(out: u32, index: u32, t: f32, points: u32, position: u32) {
        // the coefficients' frame copy (its fourth word is never written)
        let mut frame = super::StackFrame::<4>::new();
        let local = frame.at();
        tk::tk_mt_calc_spline_segment(&mut mem(), out, index, t, points, position, local, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuConfig_DrawOptionRow(_, _, row)` (the first two words are unused).
    MenuConfig_DrawOptionRow @ 0x008c_bae0 => extern "C" fn menu_config_draw_option_row(_a: u32, _b: u32, row: u32) {
        // the original's two 256-byte text buffers and its width word
        let mut frame = super::StackFrame::<0x81>::new();
        let title = frame.at();
        tk::menu_config_draw_option_row(&mut mem(), row, [title, title.wrapping_add(0x100), title.wrapping_add(0x200)], &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMn2CtrlSelItemsWindow(w)`.
    TkMn2CtrlSelItemsWindow @ 0x008d_0eb0 => extern "C" fn tk_mn2_ctrl_sel_items_window(w: u32) {
        tk::tk_mn2_ctrl_sel_items_window(&mut mem(), w, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuItem_DrawListRow(window, x, y, row, mode)`.
    MenuItem_DrawListRow @ 0x008c_7c30 => extern "C" fn menu_item_draw_list_row(window: u32, x: f32, y: f32, row: u32, mode: u32) {
        // the digit width (the original's argument slot) and the command out word
        let mut frame = super::StackFrame::<2>::new();
        let width = frame.at();
        tk::menu_item_draw_list_row(&mut mem(), window, [x, y], row, mode, [width, width.wrapping_add(4)], &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuItem_DrawGearInfo(window)`: `TkMn2DrawKickSyncPacket2`'s `eax`.
    MenuItem_DrawGearInfo @ 0x008c_9570 => extern "C" fn menu_item_draw_gear_info(window: u32) -> u32 {
        // MsGetSaveWeapon's out word and the text width
        let mut frame = super::StackFrame::<2>::new();
        let local = frame.at();
        tk::menu_item_draw_gear_info(&mut mem(), window, [local, local.wrapping_add(4)], &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuAppearMainCmdWindow(slide, mode)`: `TkMenuMainRegistWindow`'s `eax`.
    TkMenuAppearMainCmdWindow @ 0x008e_1c60 => extern "C" fn tk_menu_appear_main_cmd_window(slide: u32, mode: u32) -> u32 {
        tk::tk_menu_appear_main_cmd_window(&mut mem(), slide, mode, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuShop_DrawRow(window, x, y, row, ids)`: the name call's `eax`.
    MenuShop_DrawRow @ 0x008d_ac70 => extern "C" fn menu_shop_draw_row(window: u32, x: f32, y: f32, row: i32, ids: u32) -> u32 {
        tk::menu_shop_draw_row(&mut mem(), window, [x, y], row, ids, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_DrawBuyRow(window, x, y, row, ids)`: the name call's `eax`.
    MenuWShop_DrawBuyRow @ 0x008d_8cb0 => extern "C" fn menu_wshop_draw_buy_row(window: u32, x: f32, y: f32, row: i32, ids: u32) -> u32 {
        tk::menu_wshop_draw_buy_row(&mut mem(), window, [x, y], row, ids, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_DrawSellRow(window, x, y, row)`: the name call's `eax`.
    MenuWShop_DrawSellRow @ 0x008d_8f60 => extern "C" fn menu_wshop_draw_sell_row(window: u32, x: f32, y: f32, row: i32) -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::menu_wshop_draw_sell_row(&mut mem(), window, [x, y], row, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSummon_DrawGrowRow(window, x, y, row, fixed)`: the digits' `eax`.
    MenuSummon_DrawGrowRow @ 0x008c_e5c0 => extern "C" fn menu_summon_draw_grow_row(window: u32, x: f32, y: f32, row: i32, fixed: u32) -> u32 {
        tk::menu_summon_draw_grow_row(&mut mem(), window, [x, y], row, fixed, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkScanControler()`: the last `Pad_UpdateFrame`'s `eax`.
    TkScanControler @ 0x0088_93e0 => extern "C" fn tk_scan_controler() -> u32 {
        tk::tk_scan_controler(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_DrawBuyRows(window, _, first, x, y, ids)`: nine rows.
    MenuWShop_DrawBuyRows @ 0x008d_8c30 => extern "C" fn menu_wshop_draw_buy_rows(window: u32, _unused: u32, first: u32, x: f32, y: f32, ids: u32) {
        tk::menu_wshop_draw_buy_rows(&mut mem(), window, first, [x, y], ids, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuWShop_DrawSellRows(window, _, first, x, y, ids)`: nine rows.
    MenuWShop_DrawSellRows @ 0x008d_8ee0 => extern "C" fn menu_wshop_draw_sell_rows(window: u32, _unused: u32, first: u32, x: f32, y: f32, ids: u32) {
        tk::menu_wshop_draw_sell_rows(&mut mem(), window, first, [x, y], ids, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuShop_DrawRows(window, _, first, x, y, ids)`: ten rows.
    MenuShop_DrawRows @ 0x008d_abf0 => extern "C" fn menu_shop_draw_rows(window: u32, _unused: u32, first: u32, x: f32, y: f32, ids: u32) {
        tk::menu_shop_draw_rows(&mut mem(), window, first, [x, y], ids, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuSummon_DrawGrowRows(window, fixed, first, x, y)`: eleven rows.
    MenuSummon_DrawGrowRows @ 0x008c_e520 => extern "C" fn menu_summon_draw_grow_rows(window: u32, fixed: u32, first: u32, x: i32, y: i32) {
        tk::menu_summon_draw_grow_rows(&mut mem(), window, fixed, first, [x, y], &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuSelList_DrawCursor(window)`: `TkMn2DrawCrossCursor`'s `eax`.
    MenuSelList_DrawCursor @ 0x008d_15a0 => extern "C" fn menu_sel_list_draw_cursor(window: u32) -> u32 {
        tk::menu_sel_list_draw_cursor(&mut mem(), window, &mut |_, y, v| ui_remap(y, v), &mut |_, w| sel_list_cursor_x(w), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2DrawCrossSummonBattleCountPlate(x, y, chr)`: the last font call's `eax`.
    TkMn2DrawCrossSummonBattleCountPlate @ 0x008c_d720 => extern "C" fn tk_mn2_draw_cross_summon_battle_count_plate(x: f32, y: f32, chr: u32) -> u32 {
        tk::tk_mn2_draw_cross_summon_battle_count_plate(&mut mem(), [x, y], chr, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2DrawOverDrive(x, y, chr)`: the last call's `eax`.
    TkMn2DrawOverDrive @ 0x008d_3d70 => extern "C" fn tk_mn2_draw_over_drive(x: i32, y: i32, chr: i32) -> u32 {
        // the original's 24-byte plate rectangle
        let mut frame = super::StackFrame::<6>::new();
        let plate = frame.at();
        tk::tk_mn2_draw_over_drive(&mut mem(), [x, y], chr, plate, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuStatus_DrawLayoutRow(x, y, chr)`: the new draw cursor.
    MenuStatus_DrawLayoutRow @ 0x008c_11e0 => extern "C" fn menu_status_draw_layout_row(x: i32, y: i32, chr: u32) -> u32 {
        tk::menu_status_draw_layout_row(&mut mem(), [x, y], chr, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuStatus_DrawEquipPlate(chr, x)`: the last call's `eax`.
    MenuStatus_DrawEquipPlate @ 0x008d_4540 => extern "C" fn menu_status_draw_equip_plate(chr: u32, x: i32) -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        let width = frame.at();
        tk::menu_status_draw_equip_plate(&mut mem(), chr, x, width, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSummon_DrawCommandRows(_, x)`: its only caller calls on.
    MenuSummon_DrawCommandRows @ 0x008d_4280 => extern "C" fn menu_summon_draw_command_rows(_unused: u32, x: i32) {
        tk::menu_summon_draw_command_rows(&mut mem(), x, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuSummon_DrawCommandRow(x, y, i)`: the name's font `eax`.
    MenuSummon_DrawCommandRow @ 0x008d_4350 => extern "C" fn menu_summon_draw_command_row(x: i32, y: i32, i: i32) -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        let width = frame.at();
        tk::menu_summon_draw_command_row(&mut mem(), [x, y], i, width, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuStatus_DrawPage(x, y, chr, page, a, b, c, _, t)`: `TkMn2DrawKickPacket`'s `eax`.
    MenuStatus_DrawPage @ 0x008d_4ee0 => extern "C" fn menu_status_draw_page(x: i32, y: i32, chr: u32, page: u32, a: u32, b: u32, c: u32, _unused: u32, t: i32) -> u32 {
        tk::menu_status_draw_page(&mut mem(), [x, y], chr, page, [a, b, c], t, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSelList_DrawRow(window, x, y, row, fixed)`: its only caller ignores `eax`.
    MenuSelList_DrawRow @ 0x008d_1820 => extern "C" fn menu_sel_list_draw_row(window: u32, x: f32, y: f32, row: i32, fixed: u32) {
        let mut frame = super::StackFrame::<1>::new();
        let width = frame.at();
        tk::menu_sel_list_draw_row(&mut mem(), window, [x, y], row, fixed != 0, width, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMn2DrawStatusPlate(x, y, _, _, chr, mes, field)`: the digits' `eax`.
    TkMn2DrawStatusPlate @ 0x008c_1490 => extern "C" fn tk_mn2_draw_status_plate(x: i32, y: i32, _a: u32, _b: u32, chr: u32, mes: u32, field: u32) -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        let width = frame.at();
        tk::tk_mn2_draw_status_plate(&mut mem(), [x, y], chr, mes, field, width, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuMain_DrawOptionRow(i, x, _, _, fade)`: the last call's `eax`.
    MenuMain_DrawOptionRow @ 0x008e_22a0 => extern "C" fn menu_main_draw_option_row(index: i32, x: f32, _a: u32, _b: u32, fade: i32) -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        let width = frame.at();
        tk::menu_main_draw_option_row(&mut mem(), index, x, fade, width, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `Pad_UpdateAnalog(work, raw, buttons)`: its only caller ignores `eax`.
    Pad_UpdateAnalog @ 0x0088_9570 => extern "C" fn pad_update_analog(work: u32, raw: u32, buttons: u32) {
        tk::pad_update_analog(&mut mem(), work, raw, buttons, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuSaveLoad_DrawSlotRow(window, x, y, row, mode)`: its only caller ignores `eax`.
    MenuSaveLoad_DrawSlotRow @ 0x008b_4b70 => extern "C" fn menu_save_load_draw_slot_row(window: u32, x: i32, y: i32, row: i32, mode: u32) {
        // the original's 8-byte sprintf buffer
        let mut frame = super::StackFrame::<2>::new();
        let digits = frame.at();
        tk::menu_save_load_draw_slot_row(&mut mem(), window, [x, y], row, mode, digits, &mut |_, f, a| call_ptr(f, a));
    }
    /// `DrawCrossMenuSummonPlateClut(chr, x, y, alpha)`: the MP text's `eax`.
    DrawCrossMenuSummonPlateClut @ 0x008e_6e10 => extern "C" fn draw_cross_menu_summon_plate_clut(chr: u32, x: f32, y: f32, alpha: u32) -> u32 {
        // the original's 32-byte sprintf buffer
        let mut frame = super::StackFrame::<8>::new();
        let text = frame.at();
        tk::draw_cross_menu_summon_plate_clut(&mut mem(), chr, [x, y], alpha, text, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMn2DrawSummonPlateCheckHP(x, y, chr)`: the last call's `eax`.
    TkMn2DrawSummonPlateCheckHP @ 0x008c_eb00 => extern "C" fn tk_mn2_draw_summon_plate_check_hp(x: i32, y: i32, chr: i32) -> u32 {
        tk::tk_mn2_draw_summon_plate_check_hp(&mut mem(), [x, y], chr, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `Pad_ReadAndUpdate(port, slot, pad)`: its only caller ignores `eax`.
    Pad_ReadAndUpdate @ 0x0088_8f90 => extern "C" fn pad_read_and_update(port: u32, slot: u32, pad: u32) {
        tk::pad_read_and_update(&mut mem(), port, slot, pad, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuSelList_DrawRows(window, fixed, first, _, y)`: its only caller ignores `eax`.
    MenuSelList_DrawRows @ 0x008d_1620 => extern "C" fn menu_sel_list_draw_rows(window: u32, fixed: u32, first: i32, _a: u32, y: i32) {
        tk::menu_sel_list_draw_rows(&mut mem(), window, fixed, first, y, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkBtlEndGetText(id)`: the text pointer.
    TkBtlEndGetText @ 0x008b_da60 => extern "C" fn tk_btl_end_get_text(id: u32) -> u32 {
        tk::tk_btl_end_get_text(&mut mem(), id, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TKMallocAndReadSPS2(index, page)`: the buffer, or 0.
    TKMallocAndReadSPS2 @ 0x0088_ce80 => extern "C" fn tk_malloc_and_read_sps2(index: u32, page: u32) -> u32 {
        // the original's two 256-byte path buffers
        let mut frame = super::StackFrame::<128>::new();
        let path = frame.at();
        tk::tk_malloc_and_read_sps2(&mut mem(), index, page, [path, path + 0x100], &mut |_, f, a| call_ptr(f, a))
    }
    /// `DrawMesFontLClutTypeRGBA_ForHelp(key, text, x, y, flags, r, g, b, a)`: its only caller ignores `eax`.
    DrawMesFontLClutTypeRGBA_ForHelp @ 0x008e_75c0 => extern "C" fn draw_mes_font_lclut_type_rgba_for_help(key: u32, text: u32, x: i32, y: i32, flags: u32, r: u32, g: u32, b: u32, a: u32) {
        // graphicFontGetScreenWH's two unread outputs
        let mut frame = super::StackFrame::<2>::new();
        let screen = frame.at();
        tk::draw_mes_font_lclut_type_rgba_for_help(&mut mem(), key, text, [x, y], flags, [r, g, b, a], screen, &mut |_, f, a| call_ptr(f, a), &mut |_, f, a| fcall_ptr(f, a));
    }
    /// `MenuItem_DrawKeyUseCells()`: its only caller ignores `eax`.
    MenuItem_DrawKeyUseCells @ 0x008c_4770 => extern "C" fn menu_item_draw_key_use_cells() {
        // the original's frame word for the text width
        let mut frame = super::StackFrame::<1>::new();
        let width = frame.at();
        tk::menu_item_draw_key_use_cells(&mut mem(), width, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a), &mut |_, f, a| fcall_ptr(f, a));
    }
    /// `TkMn2MainStatusPlayerStatus(_, y, chr, t)`: the kick's `eax`.
    TkMn2MainStatusPlayerStatus @ 0x008d_5180 => extern "C" fn tk_mn2_main_status_player_status(_a: u32, y: i32, chr: i32, t: u32) -> u32 {
        // the original's frame word for the text width
        let mut frame = super::StackFrame::<1>::new();
        let width = frame.at();
        tk::tk_mn2_main_status_player_status(&mut mem(), y, chr, t, width, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `Pad_InitWork(port, slot)`: its only caller ignores `eax`.
    Pad_InitWork @ 0x0088_9360 => extern "C" fn pad_init_work(port: u32, slot: u32) {
        tk::pad_init_work(&mut mem(), port, slot, &mut |_, f, a| call_ptr(f, a));
    }
    /// `AtelEvent_FreeMovie()`: the `eax` the original leaves.
    AtelEvent_FreeMovie @ 0x0087_4130 => extern "C" fn atel_event_free_movie() -> u32 {
        tk::atel_event_free_movie(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_InitList()`: the `eax` the original leaves.
    MenuWShop_InitList @ 0x008d_94a0 => extern "C" fn menu_wshop_init_list() -> u32 {
        // the original's count and name frame words
        let mut frame = super::StackFrame::<2>::new();
        let count = frame.at();
        tk::menu_wshop_init_list(&mut mem(), [count, count + 4], &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_KeyRowCount()`: rows of three.
    MenuItem_KeyRowCount @ 0x008c_5180 => extern "C" fn menu_item_key_row_count() -> i32 {
        tk::menu_item_key_row_count(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_TypeKey()`: its only caller ignores `eax`.
    MenuItem_TypeKey @ 0x008c_4c90 => extern "C" fn menu_item_type_key() {
        tk::menu_item_type_key(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMenuGetDispCommand(kind, sub, a, chr, out)`: the count.
    TkMenuGetDispCommand @ 0x008d_2e10 => extern "C" fn tk_menu_get_disp_command(kind: u32, sub: u32, a: u32, chr: i32, out: u32) -> i32 {
        // the original's two frame words (MsMenuStatusNum's and MsGetSaveWeapon's outs)
        let mut frame = super::StackFrame::<2>::new();
        let scratch = frame.at();
        tk::tk_menu_get_disp_command(&mut mem(), kind, sub, a, chr, out, [scratch, scratch + 4], &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuMes_AddPktFont1612(cursor, key, text, x, y, flags, edge, r, g, b, a)`: the packet cursor.
    MenuMes_AddPktFont1612 @ 0x008e_4af0 => extern "C" fn menu_mes_add_pkt_font1612(cursor: u32, key: u32, text: u32, x: i32, y: i32, flags: u32, edge: u32, r: u32, g: u32, b: u32, a: u32) -> u32 {
        tk::menu_mes_add_pkt_font1612(&mut mem(), cursor, key, text, [x, y], [flags, edge], [r, g, b, a], &mut |_, f, a| call_ptr(f, a), &mut |_, f, a| fcall_ptr(f, a))
    }
    /// `MenuMes_AddPktFont1612Icons(cursor, key, text, x, y, flags, edge, r, g, b, a)`: the packet cursor.
    MenuMes_AddPktFont1612Icons @ 0x008e_4e60 => extern "C" fn menu_mes_add_pkt_font1612_icons(cursor: u32, key: u32, text: u32, x: i32, y: i32, flags: u32, edge: u32, r: u32, g: u32, b: u32, a: u32) -> u32 {
        tk::menu_mes_add_pkt_font1612_icons(&mut mem(), cursor, key, text, [x, y], [flags, edge], [r, g, b, a], &mut |_, f, a| call_ptr(f, a), &mut |_, f, a| fcall_ptr(f, a))
    }
    /// `TkUpdateScanControler()`: the last `Pad_ScanPort_00889050`'s `eax`.
    TkUpdateScanControler @ 0x0088_9a80 => extern "C" fn tk_update_scan_controler() -> u32 {
        tk::tk_update_scan_controler(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMtCalcSplineAt_0088afd0(out, points, n, t, position)`.
    TkMtCalcSplineAt_0088afd0 @ 0x0088_afd0 => extern "C" fn tk_mt_calc_spline_at(out: u32, points: u32, n: i32, t: f32, position: u32) {
        tk::tk_mt_calc_spline_at(&mut mem(), [out, points], n, t, position, &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMtCalcSpline(out, points, n, t)`.
    TkMtCalcSpline @ 0x0088_a4a0 => extern "C" fn tk_mt_calc_spline(out: u32, points: u32, n: i32, t: f32) {
        tk::tk_mt_calc_spline(&mut mem(), [out, points], n, t, &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMtCalcSplineTangent(out, points, n, t)`.
    TkMtCalcSplineTangent @ 0x0088_b3b0 => extern "C" fn tk_mt_calc_spline_tangent(out: u32, points: u32, n: i32, t: f32) {
        tk::tk_mt_calc_spline_tangent(&mut mem(), [out, points], n, t, &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMtCalcSpline3At_0088aa40(out, points, n, t, position)`.
    TkMtCalcSpline3At_0088aa40 @ 0x0088_aa40 => extern "C" fn tk_mt_calc_spline3_at(out: u32, points: u32, n: i32, t: f32, position: u32) {
        tk::tk_mt_calc_spline3_at(&mut mem(), [out, points], n, t, position, &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMtCalcSpline3(out, points, n, t)`.
    TkMtCalcSpline3 @ 0x0088_a4d0 => extern "C" fn tk_mt_calc_spline3(out: u32, points: u32, n: i32, t: f32) {
        tk::tk_mt_calc_spline3(&mut mem(), [out, points], n, t, &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMtCalcSpline3Tangent_0088aba0(out, points, n, t)`.
    TkMtCalcSpline3Tangent_0088aba0 @ 0x0088_aba0 => extern "C" fn tk_mt_calc_spline3_tangent(out: u32, points: u32, n: i32, t: f32) {
        tk::tk_mt_calc_spline3_tangent(&mut mem(), [out, points], n, t, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuSummon_DrawList_008ce130(dx, _, cursor)`: the last row's `Y`.
    MenuSummon_DrawList_008ce130 @ 0x008c_e130 => extern "C" fn menu_summon_draw_list(dx: i32, _unused: u32, cursor: i32) -> u32 {
        tk::menu_summon_draw_list(&mut mem(), dx, cursor, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuCtrlStatus(m)`: the original's `eax`.
    TkMenuCtrlStatus @ 0x008d_1e00 => extern "C" fn tk_menu_ctrl_status(m: u32) -> u32 {
        tk::tk_menu_ctrl_status(&mut mem(), m, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuCtrlWShop(m)`: the original's `eax`.
    TkMenuCtrlWShop @ 0x008d_9600 => extern "C" fn tk_menu_ctrl_w_shop(m: u32) -> u32 {
        tk::tk_menu_ctrl_w_shop(&mut mem(), m, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSubCmd_DrawItems_008e4140(m)`: the original's `eax`.
    MenuSubCmd_DrawItems_008e4140 @ 0x008e_4140 => extern "C" fn menu_sub_cmd_draw_items(m: u32) -> u32 {
        // the text width word
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::menu_sub_cmd_draw_items(&mut mem(), m, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSubCmd_DrawRow_008e3cf0(m)`: the original's `eax`.
    MenuSubCmd_DrawRow_008e3cf0 @ 0x008e_3cf0 => extern "C" fn menu_sub_cmd_draw_row(m: u32) -> u32 {
        // the text width word
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::menu_sub_cmd_draw_row(&mut mem(), m, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_BuyListDraw_008da050(w)`: `DrawCrossMenuScrollParts`'s `eax`.
    MenuWShop_BuyListDraw_008da050 @ 0x008d_a050 => extern "C" fn menu_wshop_buy_list_draw(w: u32) -> u32 {
        // the digit width word (the original's argument slot)
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::menu_wshop_buy_list_draw(&mut mem(), w, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_DrawCompare_008d8620(w, shop)` (its caller drops `eax`).
    MenuWShop_DrawCompare_008d8620 @ 0x008d_8620 => extern "C" fn menu_wshop_draw_compare(w: u32, shop: u32) {
        // MsGetSaveWeapon's two out words and the text width
        let mut frame = super::StackFrame::<3>::new();
        let local = frame.at();
        tk::menu_wshop_draw_compare(&mut mem(), w, shop, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuLimit_WindowDraw_008d16f0(w)`: `TkMn2DrawKickSyncPacket`'s `eax`.
    MenuLimit_WindowDraw_008d16f0 @ 0x008d_16f0 => extern "C" fn menu_limit_window_draw(w: u32) -> u32 {
        // MsMenuStatusNum's out word and the text width (the original's argument slot)
        let mut frame = super::StackFrame::<2>::new();
        let local = frame.at();
        tk::menu_limit_window_draw(&mut mem(), w, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuMainOpenMode_008aa780(mode, arg)`: the last call's `eax`.
    TkMenuMainOpenMode_008aa780 @ 0x008a_a780 => extern "C" fn tk_menu_main_open_mode(mode: u32, arg: u32) -> u32 {
        tk::tk_menu_main_open_mode(&mut mem(), mode, arg, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSummon_WindowDraw_008cebe0(w)`: `MenuSummon_DrawList_008ce130`'s `eax`.
    MenuSummon_WindowDraw_008cebe0 @ 0x008c_ebe0 => extern "C" fn menu_summon_window_draw(w: u32) -> u32 {
        tk::menu_summon_window_draw(&mut mem(), w, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_TargetWindowDraw_008c80b0(w)` (a window draw: `TkMenuDrawLayer` drops its `eax`).
    MenuItem_TargetWindowDraw_008c80b0 @ 0x008c_80b0 => extern "C" fn menu_item_target_window_draw(w: u32) {
        tk::menu_item_target_window_draw(&mut mem(), w, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkMenuCtrlMain(m)`: the original's `eax`.
    TkMenuCtrlMain @ 0x008e_0350 => extern "C" fn tk_menu_ctrl_main(m: u32) -> u32 {
        tk::tk_menu_ctrl_main(&mut mem(), m, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkBtlEndDraw(i)`: `TOMenuDrawKickTmp`'s `eax` (the kind for kinds 0 and 5).
    TkBtlEndDraw @ 0x008b_c340 => extern "C" fn tk_btl_end_draw(i: i32) -> u32 {
        // ToGetBtlEasyDigitWidth2's width word
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::tk_btl_end_draw(&mut mem(), i, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkBtlEndCtrl()` (menu slot 0x11's control: `TkMenuCloseAll` drops its `eax`).
    TkBtlEndCtrl @ 0x008b_a660 => extern "C" fn tk_btl_end_ctrl() {
        // MsGetSaveWeapon's name word
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::tk_btl_end_ctrl(&mut mem(), local, &mut |_, f, a| call_ptr(f, a));
    }
    /// `TkCh_Dispose(chr)`: `Ch_Dispose`'s `eax`, or `Ch_CheckHandle`'s when it refuses.
    TkCh_Dispose @ 0x0087_22f0 => extern "C" fn tk_ch_dispose(chr: u32) -> u32 {
        tk::tk_ch_dispose(&mut mem(), chr, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuConfig_Open_008cb3c0(m)`: the last row callback's `eax`, else `m`.
    MenuConfig_Open_008cb3c0 @ 0x008c_b3c0 => extern "C" fn menu_config_open(m: u32) -> u32 {
        tk::menu_config_open(&mut mem(), m, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuCloseSaveLoadScreen()`: the last call's `eax`.
    TkMenuCloseSaveLoadScreen @ 0x008b_3db0 => extern "C" fn tk_menu_close_save_load_screen() -> u32 {
        tk::tk_menu_close_save_load_screen(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuJumpToLoadedScene()`: `TkMenuMainSleepMenu`'s `eax`.
    TkMenuJumpToLoadedScene @ 0x008b_4ec0 => extern "C" fn tk_menu_jump_to_loaded_scene() -> u32 {
        tk::tk_menu_jump_to_loaded_scene(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuOpenNameEntryWindow(aeon, target)`: the window.
    TkMenuOpenNameEntryWindow @ 0x008c_4250 => extern "C" fn tk_menu_open_name_entry_window(aeon: u32, target: i32) -> u32 {
        tk::tk_menu_open_name_entry_window(&mut mem(), aeon, target, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuNameEntry_DrawKeys_008c4550()`: 0 or the password count.
    MenuNameEntry_DrawKeys_008c4550 @ 0x008c_4550 => extern "C" fn menu_name_entry_draw_keys() -> u32 {
        // the character buffer ToMakeBtlEasyFont draws
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::menu_name_entry_draw_keys(&mut mem(), local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuSave_DrawMesBox_008b4920(w)`: `TkVU1SyncPath`'s `eax` (0 with nothing to draw).
    MenuSave_DrawMesBox_008b4920 @ 0x008b_4920 => extern "C" fn menu_save_draw_mes_box(w: u32) -> u32 {
        // TOGetMessageWH's width and height
        let mut frame = super::StackFrame::<2>::new();
        let local = frame.at();
        tk::menu_save_draw_mes_box(&mut mem(), w, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuCtrlConfig(m)` (a menu slot's control: `TkMenuCloseAll` drops its `eax`).
    TkMenuCtrlConfig @ 0x008c_b0e0 => extern "C" fn tk_menu_ctrl_config(m: u32) {
        tk::tk_menu_ctrl_config(&mut mem(), m, &mut |_, f, a| call_ptr(f, a));
    }
    /// `UpdateGearCustomizationMenuState(m)`: the original's `eax`.
    UpdateGearCustomizationMenuState @ 0x008d_5830 => extern "C" fn update_gear_customization_menu_state(m: u32) -> u32 {
        // MsGetSaveWeapon's name word
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::update_gear_customization_menu_state(&mut mem(), m, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuCtrlLimit(m)`: the original's `eax`.
    TkMenuCtrlLimit @ 0x008d_0460 => extern "C" fn tk_menu_ctrl_limit(m: u32) -> u32 {
        tk::tk_menu_ctrl_limit(&mut mem(), m, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuCtrlEquip(m)`: the original's `eax`.
    TkMenuCtrlEquip @ 0x008c_f020 => extern "C" fn tk_menu_ctrl_equip(m: u32) -> u32 {
        tk::tk_menu_ctrl_equip(&mut mem(), m, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuItem_ListWindowCtrl_008c6dd0(w)`: the original's `eax`.
    MenuItem_ListWindowCtrl_008c6dd0 @ 0x008c_6dd0 => extern "C" fn menu_item_list_window_ctrl(w: u32) -> u32 {
        tk::menu_item_list_window_ctrl(&mut mem(), w, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuWShop_Ctrl_008d6e40(m)`: the original's `eax`.
    MenuWShop_Ctrl_008d6e40 @ 0x008d_6e40 => extern "C" fn menu_wshop_ctrl(m: u32) -> u32 {
        // MsGetSaveWeapon's name word (the original's argument slot)
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::menu_wshop_ctrl(&mut mem(), m, local, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TkMenuCtrlSummon(m)`: the original's `eax`.
    TkMenuCtrlSummon @ 0x008c_c330 => extern "C" fn tk_menu_ctrl_summon(m: u32) -> u32 {
        tk::tk_menu_ctrl_summon(&mut mem(), m, &mut |_, f, a| call_ptr(f, a))
    }
    /// `MenuNameEntry_WindowCtrl_008c3580(w)` (a window control: its `eax` is dropped).
    MenuNameEntry_WindowCtrl_008c3580 @ 0x008c_3580 => extern "C" fn menu_name_entry_window_ctrl(w: u32) {
        tk::menu_name_entry_window_ctrl(&mut mem(), w, &mut |_, f, a| call_ptr(f, a));
    }
    /// `MenuSummon_GrowWindowDraw_008cdba0(w)`: `TkMn2DrawKickSyncPacket`'s `eax`.
    MenuSummon_GrowWindowDraw_008cdba0 @ 0x008c_dba0 => extern "C" fn menu_summon_grow_window_draw(w: u32) -> u32 {
        // the caption width word
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        tk::menu_summon_grow_window_draw(&mut mem(), w, local, &mut |_, y, v| ui_remap(y, v), &mut |_, f, a| call_ptr(f, a))
    }
}

/// `MenuSelList_CursorX(window)` (`0x008d1bc0`) through the game's entry
/// (the hook when it is on): the result in `ST0`, read back as a double.
fn sel_list_cursor_x(window: u32) -> f64 {
    let f = crate::rebase(0x008d_1bc0);
    // SAFETY: a cdecl function of one word returning in ST0.
    unsafe { std::mem::transmute::<usize, unsafe extern "C" fn(u32) -> f64>(f)(window) }
}

/// `inputGetCurTime()` (`0x00630c60`) through the game's entry: the result
/// in `ST0`, read back as a double.
fn input_get_cur_time() -> f64 {
    let f = crate::rebase(0x0063_0c60);
    // SAFETY: a cdecl function of no arguments returning in ST0.
    unsafe { std::mem::transmute::<usize, unsafe extern "C" fn() -> f64>(f)() }
}
