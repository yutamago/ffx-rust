//! `ffx-sg`: the main module (`Sg`): switches and accessors of the main
//! loop, point lights, the GS packet writers, small vector helpers,
//! `crossAssertIsValidFloat`.

use ffx_sg::{gs, light, state, vec};

use super::{mem, ported};

ported! {
    /// `crossAssertIsValidFloat(x)`.
    crossAssertIsValidFloat @ 0x006e_f2d0 => extern "C" fn cross_assert_is_valid_float(x: f32) -> u32 {
        ffx_sg::assert::is_valid_float(x)
    }
    /// `get_read_buffer()`.
    get_read_buffer @ 0x0076_dc50 => extern "C" fn get_read_buffer() -> u32 {
        state::bridge::get_read_buffer(&mem())
    }
    /// `read_err_disp_set(v)` (a byte).
    read_err_disp_set @ 0x0076_e5e0 => extern "C" fn read_err_disp_set(v: u32) {
        state::bridge::read_err_disp_set(&mut mem(), v);
    }
    /// `set_hdd_err(v)` (a byte).
    set_hdd_err @ 0x0077_9f70 => extern "C" fn set_hdd_err(v: u32) {
        state::bridge::set_hdd_err(&mut mem(), v);
    }
    /// `sa_task_buff_size_get(n)`.
    sa_task_buff_size_get @ 0x0077_a180 => extern "C" fn sa_task_buff_size_get(n: u32) -> u32 {
        state::bridge::sa_task_buff_size_get(n)
    }
    /// `Sg_EffectRoutineEnable(v)` (a byte).
    Sg_EffectRoutineEnable @ 0x0082_0380 => extern "C" fn sg_effect_routine_enable(v: u32) {
        state::bridge::effect_routine_enable(&mut mem(), v);
    }
    /// `Sg_EnableCalcCam(on)`: the old value.
    Sg_EnableCalcCam @ 0x0082_03a0 => extern "C" fn sg_enable_calc_cam(on: u32) -> i32 {
        state::bridge::enable_calc_cam(&mut mem(), on)
    }
    /// `Sg_EnableDrawChr(on)`: the old value.
    Sg_EnableDrawChr @ 0x0082_03c0 => extern "C" fn sg_enable_draw_chr(on: u32) -> i32 {
        state::bridge::enable_draw_chr(&mut mem(), on)
    }
    /// `Sg_EnableCalcChr(on)`: the old value.
    Sg_EnableCalcChr @ 0x0082_03e0 => extern "C" fn sg_enable_calc_chr(on: u32) -> i32 {
        state::bridge::enable_calc_chr(&mut mem(), on)
    }
    /// `Sg_EnableDrawMap(on)`: the old value.
    Sg_EnableDrawMap @ 0x0082_0400 => extern "C" fn sg_enable_draw_map(on: u32) -> i32 {
        state::bridge::enable_draw_map(&mut mem(), on)
    }
    /// `Sg_EnableDrawGui(on)`: the old value.
    Sg_EnableDrawGui @ 0x0082_0420 => extern "C" fn sg_enable_draw_gui(on: u32) -> i32 {
        state::bridge::enable_draw_gui(&mut mem(), on)
    }
    /// `Sg_EnableCalcMap(on)`: the old value.
    Sg_EnableCalcMap @ 0x0082_0440 => extern "C" fn sg_enable_calc_map(on: u32) -> i32 {
        state::bridge::enable_calc_map(&mut mem(), on)
    }
    /// `Sg_EnableDrawMenu(on)`: the old value.
    Sg_EnableDrawMenu @ 0x0082_0460 => extern "C" fn sg_enable_draw_menu(on: u32) -> i32 {
        state::bridge::enable_draw_menu(&mut mem(), on)
    }
    /// `Sg_EnableEffectMatirx()`.
    Sg_EnableEffectMatirx @ 0x0082_0480 => extern "C" fn sg_enable_effect_matirx() {
        state::bridge::enable_effect_matrix(&mut mem());
    }
    /// `Sg_EnableGetCalcCam()`.
    Sg_EnableGetCalcCam @ 0x0082_0490 => extern "C" fn sg_enable_get_calc_cam() -> i32 {
        state::bridge::get_calc_cam(&mem())
    }
    /// `Sg_EnableTkAnti(on)`: the old value.
    Sg_EnableTkAnti @ 0x0082_0500 => extern "C" fn sg_enable_tk_anti(on: u32) -> i32 {
        state::bridge::enable_tk_anti(&mut mem(), on)
    }
    /// `Sg_GetDBuffDC(dc)`.
    Sg_GetDBuffDC @ 0x0082_0530 => extern "C" fn sg_get_d_buff_dc(dc: u32) -> u32 {
        state::bridge::get_dbuff_dc(&mut mem(), dc)
    }
    /// `Sg_GetDispMode()`.
    Sg_GetDispMode @ 0x0082_0550 => extern "C" fn sg_get_disp_mode() -> i32 {
        state::bridge::get_disp_mode(&mem())
    }
    /// `Sg_EnableGetTkAnti()`.
    Sg_EnableGetTkAnti @ 0x0082_0570 => extern "C" fn sg_enable_get_tk_anti() -> i32 {
        state::bridge::get_tk_anti(&mem())
    }
    /// `Sg_GetKeepFps()`.
    Sg_GetKeepFps @ 0x0082_05a0 => extern "C" fn sg_get_keep_fps() -> i32 {
        state::bridge::get_keep_fps(&mem())
    }
    /// `Sg_GetMapAlphaFast()`.
    Sg_GetMapAlphaFast @ 0x0082_05d0 => extern "C" fn sg_get_map_alpha_fast() -> i32 {
        state::bridge::get_map_alpha_fast(&mem())
    }
    /// `Sg_GetPause()`.
    Sg_GetPause @ 0x0082_05e0 => extern "C" fn sg_get_pause() -> i32 {
        state::bridge::get_pause(&mem())
    }
    /// `Sg_GetUserName()`.
    Sg_GetUserName @ 0x0082_0610 => extern "C" fn sg_get_user_name() -> u32 {
        state::bridge::get_user_name(&mem())
    }
    /// `Sg_GetViewerMode()`.
    Sg_GetViewerMode @ 0x0082_0640 => extern "C" fn sg_get_viewer_mode() -> i32 {
        state::bridge::get_viewer_mode(&mem())
    }
    /// `Sg_GetthDispEnable()`.
    Sg_GetthDispEnable @ 0x0082_0650 => extern "C" fn sg_getth_disp_enable() -> i32 {
        state::bridge::get_th_disp_enable(&mem())
    }
    /// `Sg_GuiGetDrawChr()`.
    Sg_GuiGetDrawChr @ 0x0082_0660 => extern "C" fn sg_gui_get_draw_chr() -> i32 {
        state::bridge::gui_get_draw_chr(&mem())
    }
    /// `Sg_GuiSetDrawChr(v)` (a byte).
    Sg_GuiSetDrawChr @ 0x0082_0670 => extern "C" fn sg_gui_set_draw_chr(v: u32) {
        state::bridge::gui_set_draw_chr(&mut mem(), v);
    }
    /// `Sg_SetKeepFps(v)`: the old value.
    Sg_SetKeepFps @ 0x0082_1ae0 => extern "C" fn sg_set_keep_fps(v: u32) -> i32 {
        state::bridge::set_keep_fps(&mut mem(), v)
    }
    /// `Sg_SetPause(v)` (a byte).
    Sg_SetPause @ 0x0082_1cb0 => extern "C" fn sg_set_pause(v: u32) {
        state::bridge::set_pause(&mut mem(), v);
    }
    /// `Sg_SetScreenWH(w, h)`.
    Sg_SetScreenWH @ 0x0082_1d10 => extern "C" fn sg_set_screen_wh(w: i32, h: i32) -> i32 {
        gs::bridge::set_screen_wh(&mut mem(), w, h)
    }
    /// `Sg_SetthDispEnable(v)` (a byte).
    Sg_SetthDispEnable @ 0x0082_1d50 => extern "C" fn sg_setth_disp_enable(v: u32) {
        state::bridge::set_th_disp_enable(&mut mem(), v);
    }
    /// `Sg_ToggleTkAnti()`.
    Sg_ToggleTkAnti @ 0x0082_1e30 => extern "C" fn sg_toggle_tk_anti() {
        state::bridge::toggle_tk_anti(&mut mem());
    }
    /// `isDebugMode()`: `g_FFXIsDebug` in `AL`.
    isDebugMode @ 0x0082_2550 => extern "C" fn is_debug_mode() -> u8 {
        state::bridge::is_debug_mode(&mem())
    }
    /// `Sg_VecAdd(out, a, b)`.
    Sg_VecAdd @ 0x0082_4ce0 => extern "C" fn sg_vec_add(out: u32, a: u32, b: u32) {
        vec::bridge::add(&mut mem(), out, a, b);
    }
    /// `Sg_VecCopy4(out, v)`.
    Sg_VecCopy4 @ 0x0082_4d10 => extern "C" fn sg_vec_copy4(out: u32, v: u32) {
        vec::bridge::copy4(&mut mem(), out, v);
    }
    /// `Sg_VecDot(a, b, out)`.
    Sg_VecDot @ 0x0082_4d40 => extern "C" fn sg_vec_dot(a: u32, b: u32, out: u32) {
        vec::bridge::dot(&mut mem(), a, b, out);
    }
    /// `Sg_VecScale(out, v, s)`.
    Sg_VecScale @ 0x0082_4d70 => extern "C" fn sg_vec_scale(out: u32, v: u32, s: f32) {
        vec::bridge::scale(&mut mem(), out, v, s);
    }
    /// `Sg_VecSub(out, a, b)`.
    Sg_VecSub @ 0x0082_4da0 => extern "C" fn sg_vec_sub(out: u32, a: u32, b: u32) {
        vec::bridge::sub(&mut mem(), out, a, b);
    }
    /// `Sg_GlobalInitWind()`.
    Sg_GlobalInitWind @ 0x0082_4e10 => extern "C" fn sg_global_init_wind() {
        vec::bridge::global_init_wind(&mut mem());
    }
    /// `Sg_GlobalSetWind(x, y, z)`.
    Sg_GlobalSetWind @ 0x0082_4e30 => extern "C" fn sg_global_set_wind(x: f32, y: f32, z: f32) {
        vec::bridge::global_set_wind(&mut mem(), x, y, z);
    }
    /// `Sg_AccSetDrawMap(v)` (a byte).
    Sg_AccSetDrawMap @ 0x0082_bcb0 => extern "C" fn sg_acc_set_draw_map(v: u32) {
        state::bridge::acc_set_draw_map(&mut mem(), v);
    }
    /// `Sg_AccGetAlpha()`.
    Sg_AccGetAlpha @ 0x0082_bcc0 => extern "C" fn sg_acc_get_alpha() -> i32 {
        state::bridge::acc_get_alpha(&mem())
    }
    /// `Sg_AccSetDrawChr(v)` (a byte).
    Sg_AccSetDrawChr @ 0x0082_be00 => extern "C" fn sg_acc_set_draw_chr(v: u32) {
        state::bridge::acc_set_draw_chr(&mut mem(), v);
    }
    /// `Sg_AccEnableBG(v)` (a byte).
    Sg_AccEnableBG @ 0x0082_be10 => extern "C" fn sg_acc_enable_bg(v: u32) {
        state::bridge::acc_enable_bg(&mut mem(), v);
    }
    /// `Sg_DrawSetClearIgnore(v)` (a byte).
    Sg_DrawSetClearIgnore @ 0x0082_c7a0 => extern "C" fn sg_draw_set_clear_ignore(v: u32) {
        state::bridge::draw_set_clear_ignore(&mut mem(), v);
    }
    /// `Sg_EnFocus(v)`.
    Sg_EnFocus @ 0x0082_cb60 => extern "C" fn sg_en_focus(v: u32) -> u32 {
        state::bridge::en_focus(&mut mem(), v)
    }
    /// `Sg_FilterReset()`.
    Sg_FilterReset @ 0x0082_cc70 => extern "C" fn sg_filter_reset() -> u32 {
        state::bridge::filter_reset(&mut mem())
    }
    /// `Sg_FlashReset()`.
    Sg_FlashReset @ 0x0082_cce0 => extern "C" fn sg_flash_reset() -> u32 {
        state::bridge::flash_reset(&mut mem())
    }
    /// `Sg_GetBlur()`.
    Sg_GetBlur @ 0x0082_cd20 => extern "C" fn sg_get_blur() -> i32 {
        state::bridge::get_blur(&mem())
    }
    /// `Sg_GetFocus(i)`.
    Sg_GetFocus @ 0x0082_cd30 => extern "C" fn sg_get_focus(i: u32) -> u32 {
        state::bridge::get_focus(&mem(), i)
    }
    /// `Sg_GetFocusEnable()`.
    Sg_GetFocusEnable @ 0x0082_cd40 => extern "C" fn sg_get_focus_enable() -> u32 {
        state::bridge::get_focus_enable(&mem())
    }
    /// `Sg_VramGetAddrZ()`.
    Sg_VramGetAddrZ @ 0x0082_ce80 => extern "C" fn sg_vram_get_addr_z() -> u32 {
        state::bridge::vram_get_addr_z()
    }
    /// `Sg_GetExecFrames()`.
    Sg_GetExecFrames @ 0x0082_d810 => extern "C" fn sg_get_exec_frames() -> i32 {
        state::bridge::get_exec_frames(&mem())
    }
    /// `Sg_GetExecStep()`.
    Sg_GetExecStep @ 0x0082_d820 => extern "C" fn sg_get_exec_step() -> i32 {
        state::bridge::get_exec_step(&mem())
    }
    /// `Sg_GetIgnoreID()`.
    Sg_GetIgnoreID @ 0x0082_d830 => extern "C" fn sg_get_ignore_id() -> u32 {
        state::bridge::get_ignore_id(&mem())
    }
    /// `Sg_GetLRButtonMode()`.
    Sg_GetLRButtonMode @ 0x0082_d840 => extern "C" fn sg_get_lr_button_mode() -> i32 {
        state::bridge::get_lr_button_mode(&mem())
    }
    /// `Sg_GetPlayer()`.
    Sg_GetPlayer @ 0x0082_d860 => extern "C" fn sg_get_player() -> u32 {
        state::bridge::get_player(&mem())
    }
    /// `Sg_SetExecFrames(n)`.
    Sg_SetExecFrames @ 0x0082_da60 => extern "C" fn sg_set_exec_frames(n: i32) {
        state::bridge::set_exec_frames(&mut mem(), n);
    }
    /// `Sg_SetExecSpecialSpeed(v)` (a byte).
    Sg_SetExecSpecialSpeed @ 0x0082_da90 => extern "C" fn sg_set_exec_special_speed(v: u32) {
        state::bridge::set_exec_special_speed(&mut mem(), v);
    }
    /// `Sg_SetLRButtonMode(v)` (a byte).
    Sg_SetLRButtonMode @ 0x0082_daa0 => extern "C" fn sg_set_lr_button_mode(v: u32) {
        state::bridge::set_lr_button_mode(&mut mem(), v);
    }
    /// `Sg_SetMovieGomiAvoid(v)` (a byte).
    Sg_SetMovieGomiAvoid @ 0x0083_0150 => extern "C" fn sg_set_movie_gomi_avoid(v: u32) {
        state::bridge::set_movie_gomi_avoid(&mut mem(), v);
    }
    /// `Sg_GuiGetTextureAnim()`.
    Sg_GuiGetTextureAnim @ 0x0083_1930 => extern "C" fn sg_gui_get_texture_anim() -> i32 {
        state::bridge::gui_get_texture_anim(&mem())
    }
    /// `Sg_GuiSetTextureAnim(v)` (a byte).
    Sg_GuiSetTextureAnim @ 0x0083_1940 => extern "C" fn sg_gui_set_texture_anim(v: u32) {
        state::bridge::gui_set_texture_anim(&mut mem(), v);
    }
    /// `Sg_PLightGetAttenuation(i)`.
    Sg_PLightGetAttenuation @ 0x0083_1a40 => extern "C" fn sg_p_light_get_attenuation(i: i32) -> f32 {
        light::bridge::get_attenuation(&mem(), i)
    }
    /// `Sg_PLightGetColor(i, r, g, b)`.
    Sg_PLightGetColor @ 0x0083_1a60 => extern "C" fn sg_p_light_get_color(i: i32, r: u32, g: u32, b: u32) {
        light::bridge::get_color(&mut mem(), i, [r, g, b]);
    }
    /// `Sg_PLightGetEnable(i)`.
    Sg_PLightGetEnable @ 0x0083_1ad0 => extern "C" fn sg_p_light_get_enable(i: i32) -> i32 {
        light::bridge::get_enable(&mem(), i)
    }
    /// `Sg_PLightGetLight(i)`.
    Sg_PLightGetLight @ 0x0083_1b00 => extern "C" fn sg_p_light_get_light(i: i32) -> i32 {
        light::bridge::get_light(&mem(), i)
    }
    /// `Sg_PLightGetPos(i, x, y, z)`.
    Sg_PLightGetPos @ 0x0083_1b30 => extern "C" fn sg_p_light_get_pos(i: i32, x: u32, y: u32, z: u32) {
        light::bridge::get_pos(&mut mem(), i, [x, y, z]);
    }
    /// `Sg_PLightGetPtr()`.
    Sg_PLightGetPtr @ 0x0083_1b70 => extern "C" fn sg_p_light_get_ptr() -> u32 {
        light::bridge::get_ptr(&mem())
    }
    /// `Sg_PLightGetRadius(i)`.
    Sg_PLightGetRadius @ 0x0083_1b80 => extern "C" fn sg_p_light_get_radius(i: i32) -> f32 {
        light::bridge::get_radius(&mem(), i)
    }
    /// `Sg_PLightSetAttenuation(i, a)`.
    Sg_PLightSetAttenuation @ 0x0083_1bf0 => extern "C" fn sg_p_light_set_attenuation(i: i32, a: f32) {
        light::bridge::set_attenuation(&mut mem(), i, a);
    }
    /// `Sg_PLightSetColor(i, r, g, b)`.
    Sg_PLightSetColor @ 0x0083_1c10 => extern "C" fn sg_p_light_set_color(i: i32, r: i32, g: i32, b: i32) {
        light::bridge::set_color(&mut mem(), i, [r, g, b]);
    }
    /// `Sg_PLightSetEnable(i, on)`.
    Sg_PLightSetEnable @ 0x0083_1c60 => extern "C" fn sg_p_light_set_enable(i: i32, on: u32) {
        light::bridge::set_enable(&mut mem(), i, on);
    }
    /// `Sg_PLightSetLight(i, light)`.
    Sg_PLightSetLight @ 0x0083_1c80 => extern "C" fn sg_p_light_set_light(i: i32, light: u32) {
        light::bridge::set_light(&mut mem(), i, light);
    }
    /// `Sg_PLightSetPos(i, mask, x, y, z)`.
    Sg_PLightSetPos @ 0x0083_1ca0 => extern "C" fn sg_p_light_set_pos(i: i32, mask: u32, x: f32, y: f32, z: f32) {
        light::bridge::set_pos(&mut mem(), i, mask, [x, y, z]);
    }
    /// `Sg_PLightSetRadius(i, r)`.
    Sg_PLightSetRadius @ 0x0083_1ce0 => extern "C" fn sg_p_light_set_radius(i: i32, r: f32) {
        light::bridge::set_radius(&mut mem(), i, r);
    }
    /// `Ch_SetShadeByLight(v)` (a byte).
    Ch_SetShadeByLight @ 0x0083_1d00 => extern "C" fn ch_set_shade_by_light(v: u32) {
        state::bridge::chr_set_shade_by_light(&mut mem(), v);
    }
    /// `Ch_GuiGetMouthTh()`.
    Ch_GuiGetMouthTh @ 0x0083_4070 => extern "C" fn ch_gui_get_mouth_th() -> i32 {
        state::bridge::chr_gui_get_mouth_th(&mem())
    }
    /// `Sg_GetSuimen()`.
    Sg_GetSuimen @ 0x0083_60a0 => extern "C" fn sg_get_suimen() -> f32 {
        light::bridge::get_suimen(&mem())
    }
    /// `Sg_LightGetColorMatrixPtr()`.
    Sg_LightGetColorMatrixPtr @ 0x0083_61b0 => extern "C" fn sg_light_get_color_matrix_ptr() -> u32 {
        light::bridge::get_color_matrix_ptr(&mem())
    }
    /// `Sg_LightGetNormalMatrixPtr()`.
    Sg_LightGetNormalMatrixPtr @ 0x0083_6240 => extern "C" fn sg_light_get_normal_matrix_ptr() -> u32 {
        light::bridge::get_normal_matrix_ptr(&mem())
    }
    /// `Sg_SetSuimen(h)`.
    Sg_SetSuimen @ 0x0083_6640 => extern "C" fn sg_set_suimen(h: f32) {
        light::bridge::set_suimen(&mut mem(), h);
    }
    /// `Sg_SetWaterLimit(h)`.
    Sg_SetWaterLimit @ 0x0083_6660 => extern "C" fn sg_set_water_limit(h: f32) {
        light::bridge::set_water_limit(&mut mem(), h);
    }
    /// `Sg_VectorGetLength(v)`.
    Sg_VectorGetLength @ 0x0083_6670 => extern "C" fn sg_vector_get_length(v: u32) -> f32 {
        vec::bridge::vector_get_length(&mem(), v)
    }
    /// `Sg_RadToDeg(r)`.
    Sg_RadToDeg @ 0x0083_66f0 => extern "C" fn sg_rad_to_deg(r: f32) -> f32 {
        vec::bridge::rad_to_deg(&mem(), r)
    }
    /// `enableGameControlTextureAnimation(v)`.
    enableGameControlTextureAnimation @ 0x0083_6710 => extern "C" fn enable_game_control_texture_animation(v: u32) -> u32 {
        state::bridge::enable_game_control_texture_animation(&mut mem(), v)
    }
    /// `ReadU16Unaligned(p)`.
    ReadU16Unaligned @ 0x0083_8220 => extern "C" fn read_u16_unaligned(p: u32) -> i32 {
        state::bridge::read_u16_unaligned(&mem(), p)
    }
    /// `Sg_CaptureGetGuiHide()`.
    Sg_CaptureGetGuiHide @ 0x0083_a620 => extern "C" fn sg_capture_get_gui_hide() -> u32 {
        state::bridge::capture_get_gui_hide(&mem())
    }
    /// `Sg_CaptureGetSelect()`.
    Sg_CaptureGetSelect @ 0x0083_a640 => extern "C" fn sg_capture_get_select() -> i32 {
        state::bridge::capture_get_select(&mem())
    }
    /// `Sg_CaptureSetSelect(v)` (a byte).
    Sg_CaptureSetSelect @ 0x0083_a680 => extern "C" fn sg_capture_set_select(v: u32) {
        state::bridge::capture_set_select(&mut mem(), v);
    }
    /// `Sg_EnableTkAnti_0083add0(on)`: the old value.
    Sg_EnableTkAnti_0083add0 @ 0x0083_add0 => extern "C" fn sg_enable_tk_anti_0083add0(on: u32) -> i32 {
        state::bridge::set_enable_tk_anti(&mut mem(), on)
    }
    /// `Sg_PcReadGetLen()`.
    Sg_PcReadGetLen @ 0x0083_ae70 => extern "C" fn sg_pc_read_get_len() -> u32 {
        state::bridge::pc_read_get_len(&mem())
    }
    /// `Sg_SetTkAntiXYB(x, y, b)`.
    Sg_SetTkAntiXYB @ 0x0083_b110 => extern "C" fn sg_set_tk_anti_xyb(x: u32, y: u32, b: u32) -> u32 {
        state::bridge::set_tk_anti_xyb(&mut mem(), x, y, b)
    }
    /// `Sg_PacketGetPtr()`.
    Sg_PacketGetPtr @ 0x0083_b650 => extern "C" fn sg_packet_get_ptr() -> u32 {
        state::bridge::packet_get_ptr(&mem())
    }
    /// `Sg_MemSetCheck(v)` (a byte).
    Sg_MemSetCheck @ 0x0083_c4d0 => extern "C" fn sg_mem_set_check(v: u32) {
        state::bridge::mem_set_check(&mut mem(), v);
    }
    /// `Sg_MemSetErrorIgnore(v)`.
    Sg_MemSetErrorIgnore @ 0x0083_c510 => extern "C" fn sg_mem_set_error_ignore(v: u32) -> u32 {
        state::bridge::mem_set_error_ignore(&mut mem(), v)
    }
    /// `Sg_MemSetFast(v)` (a byte).
    Sg_MemSetFast @ 0x0083_c520 => extern "C" fn sg_mem_set_fast(v: u32) {
        state::bridge::mem_set_fast(&mut mem(), v);
    }
    /// `Sg_MemSetMoveHeapStartAddr(v)`.
    Sg_MemSetMoveHeapStartAddr @ 0x0083_c530 => extern "C" fn sg_mem_set_move_heap_start_addr(v: u32) -> u32 {
        state::bridge::mem_set_move_heap_start_addr(&mut mem(), v)
    }
    /// `Sg_GsloadSetBase(load)`.
    Sg_GsloadSetBase @ 0x0083_d370 => extern "C" fn sg_gsload_set_base(load: u32) {
        gs::bridge::gsload_set_base(&mut mem(), load);
    }
    /// `Sg_TextureGetSpecular()`.
    Sg_TextureGetSpecular @ 0x0083_d3a0 => extern "C" fn sg_texture_get_specular() -> u32 {
        state::bridge::texture_get_specular(&mem())
    }
    /// `Sg_IdGetPtr()`.
    Sg_IdGetPtr @ 0x0083_e970 => extern "C" fn sg_id_get_ptr() -> u32 {
        state::bridge::id_get_ptr(&mem())
    }
    /// `Sg_IdGetZoom()`.
    Sg_IdGetZoom @ 0x0083_e9a0 => extern "C" fn sg_id_get_zoom() -> f32 {
        state::bridge::id_get_zoom(&mem())
    }
    /// `Sg_IdSetEffectTable(i, v)`.
    Sg_IdSetEffectTable @ 0x0083_e9b0 => extern "C" fn sg_id_set_effect_table(i: u32, v: u32) -> u32 {
        state::bridge::id_set_effect_table(&mut mem(), i, v)
    }
    /// `Sg_IdSetEffectTableIndex(v)` (a byte).
    Sg_IdSetEffectTableIndex @ 0x0083_e9d0 => extern "C" fn sg_id_set_effect_table_index(v: u32) {
        state::bridge::id_set_effect_table_index(&mut mem(), v);
    }
    /// `Sg_InitCamera()`.
    Sg_InitCamera @ 0x0083_f780 => extern "C" fn sg_init_camera() {
        vec::bridge::init_camera(&mut mem());
    }
    /// `Sg_GetResourceSTR(i)`.
    Sg_GetResourceSTR @ 0x0083_fc30 => extern "C" fn sg_get_resource_str(i: u32) -> u32 {
        state::bridge::get_resource_str(&mem(), i)
    }
    /// `Sg_DbLine()`.
    Sg_DbLine @ 0x0084_1470 => extern "C" fn sg_db_line() {
        gs::bridge::db_line(&mut mem());
    }
    /// `Sg_DbLineStrip()`.
    Sg_DbLineStrip @ 0x0084_1490 => extern "C" fn sg_db_line_strip() {
        gs::bridge::db_line_strip(&mut mem());
    }
    /// `Sg_DbVertex(v)`.
    Sg_DbVertex @ 0x0084_1610 => extern "C" fn sg_db_vertex(v: u32) {
        gs::bridge::db_vertex(&mut mem(), v);
    }
    /// `Sg_GsALPHA(a, b, c, d, fix)`.
    Sg_GsALPHA @ 0x0084_1670 => extern "C" fn sg_gs_alpha(a: i32, b: i32, c: i32, d: i32, fix: i32) -> i32 {
        gs::bridge::alpha(&mut mem(), [a, b, c, d], fix)
    }
    /// `Sg_GsDIMX()`.
    Sg_GsDIMX @ 0x0084_16f0 => extern "C" fn sg_gs_dimx() -> u32 {
        gs::bridge::dimx(&mut mem())
    }
    /// `Sg_GsDTHE(on)`.
    Sg_GsDTHE @ 0x0084_1730 => extern "C" fn sg_gs_dthe(on: i32) -> i32 {
        gs::bridge::dthe(&mut mem(), on)
    }
    /// `Sg_GsEnd2()`.
    Sg_GsEnd2 @ 0x0084_1800 => extern "C" fn sg_gs_end2() -> u32 {
        gs::bridge::end2(&mut mem())
    }
    /// `Sg_GsFRAME(fbp, fbmsk)`.
    Sg_GsFRAME @ 0x0084_1870 => extern "C" fn sg_gs_frame(fbp: u32, fbmsk: u32) -> u32 {
        gs::bridge::frame_1(&mut mem(), fbp, fbmsk)
    }
    /// `Sg_GsFRAME2(fbp, fbmsk)`.
    Sg_GsFRAME2 @ 0x0084_18f0 => extern "C" fn sg_gs_frame2(fbp: u32, fbmsk: u32) -> u32 {
        gs::bridge::frame_2(&mut mem(), fbp, fbmsk)
    }
    /// `Sg_GsFRAME2_16(fbp, fbmsk)`.
    Sg_GsFRAME2_16 @ 0x0084_1970 => extern "C" fn sg_gs_frame2_16(fbp: u32, fbmsk: u32) -> u32 {
        gs::bridge::frame_16_2(&mut mem(), fbp, fbmsk)
    }
    /// `Sg_GsFRAME_16(fbp, fbmsk)`.
    Sg_GsFRAME_16 @ 0x0084_19d0 => extern "C" fn sg_gs_frame_16(fbp: u32, fbmsk: u32) -> u32 {
        gs::bridge::frame_16_1(&mut mem(), fbp, fbmsk)
    }
    /// `Sg_GsGet()`.
    Sg_GsGet @ 0x0084_1a30 => extern "C" fn sg_gs_get() -> u32 {
        gs::bridge::get(&mem())
    }
    /// `Sg_GsInc()`.
    Sg_GsInc @ 0x0084_1a50 => extern "C" fn sg_gs_inc() {
        gs::bridge::inc(&mut mem());
    }
    /// `Sg_GsPRIM(prim)`.
    Sg_GsPRIM @ 0x0084_1a60 => extern "C" fn sg_gs_prim(prim: i32) -> i32 {
        gs::bridge::prim(&mut mem(), prim)
    }
    /// `Sg_GsRGBAQ(r, g, b, a)`.
    Sg_GsRGBAQ @ 0x0084_1aa0 => extern "C" fn sg_gs_rgbaq(r: i32, g: i32, b: i32, a: i32) -> i32 {
        gs::bridge::rgbaq(&mut mem(), r, g, b, a)
    }
    /// `Sg_GsSCISSOR(x0, x1, y0, y1)`.
    Sg_GsSCISSOR @ 0x0084_1b90 => extern "C" fn sg_gs_scissor(x0: i32, x1: i32, y0: i32, y1: i32) -> i32 {
        gs::bridge::scissor_1(&mut mem(), x0, x1, y0, y1)
    }
    /// `Sg_GsSCISSOR2(x0, x1, y0, y1)`.
    Sg_GsSCISSOR2 @ 0x0084_1c40 => extern "C" fn sg_gs_scissor2(x0: i32, x1: i32, y0: i32, y1: i32) -> i32 {
        gs::bridge::scissor_2(&mut mem(), x0, x1, y0, y1)
    }
    /// `Sg_GsStart()`.
    Sg_GsStart @ 0x0084_1d10 => extern "C" fn sg_gs_start() -> u32 {
        gs::bridge::start(&mut mem())
    }
    /// `Sg_GsStartVIF()`.
    Sg_GsStartVIF @ 0x0084_1d50 => extern "C" fn sg_gs_start_vif() {
        gs::bridge::start_vif(&mut mem());
    }
    /// `Sg_GsTEST(f0, f1, f2, f3, f4, f5, f6, f7)`.
    Sg_GsTEST @ 0x0084_1d80 => extern "C" fn sg_gs_test(f0: i32, f1: i32, f2: i32, f3: i32, f4: i32, f5: i32, f6: i32, f7: i32) -> i32 {
        gs::bridge::test_1(&mut mem(), [f0, f1, f2, f3, f4, f5, f6, f7])
    }
    /// `Sg_GsTEST2(f0, f1, f2, f3, f4, f5, f6, f7)`.
    Sg_GsTEST2 @ 0x0084_1e30 => extern "C" fn sg_gs_test2(f0: i32, f1: i32, f2: i32, f3: i32, f4: i32, f5: i32, f6: i32, f7: i32) -> i32 {
        gs::bridge::test_2(&mut mem(), [f0, f1, f2, f3, f4, f5, f6, f7])
    }
    /// `Sg_GsTEXFLUSH()`.
    Sg_GsTEXFLUSH @ 0x0084_1ee0 => extern "C" fn sg_gs_texflush() -> u32 {
        gs::bridge::texflush(&mut mem())
    }
    /// `Sg_GsTexHokan(on)`.
    Sg_GsTexHokan @ 0x0084_1f10 => extern "C" fn sg_gs_tex_hokan(on: u32) -> u32 {
        gs::bridge::tex_hokan(&mut mem(), on)
    }
    /// `Sg_GsUV(u, v)`.
    Sg_GsUV @ 0x0084_1f60 => extern "C" fn sg_gs_uv(u: u32, v: u32) -> u32 {
        gs::bridge::uv_1(&mut mem(), u, v)
    }
    /// `Sg_GsUV2(u, v)`.
    Sg_GsUV2 @ 0x0084_1fb0 => extern "C" fn sg_gs_uv2(u: u32, v: u32) -> u32 {
        gs::bridge::uv_2(&mut mem(), u, v)
    }
    /// `Sg_GsXYOFFSET(x, y)`.
    Sg_GsXYOFFSET @ 0x0084_2000 => extern "C" fn sg_gs_xyoffset(x: i32, y: i32) -> i32 {
        gs::bridge::xyoffset_1(&mut mem(), x, y)
    }
    /// `Sg_GsXYOFFSET2(x, y)`.
    Sg_GsXYOFFSET2 @ 0x0084_2070 => extern "C" fn sg_gs_xyoffset2(x: i32, y: i32) -> i32 {
        gs::bridge::xyoffset_2(&mut mem(), x, y)
    }
    /// `Sg_GsXYZF2(x, y)`.
    Sg_GsXYZF2 @ 0x0084_2140 => extern "C" fn sg_gs_xyzf2(x: i32, y: i32) -> i32 {
        gs::bridge::xyzf2(&mut mem(), x, y)
    }
    /// `Sg_GsXYZF2V(x, y, zf)`.
    Sg_GsXYZF2V @ 0x0084_2210 => extern "C" fn sg_gs_xyzf2_v(x: i32, y: i32, zf: u32) -> u32 {
        gs::bridge::xyzf2v(&mut mem(), x, y, zf)
    }
    /// `SendSpadToMFIFO()`.
    SendSpadToMFIFO @ 0x0084_3200 => extern "C" fn send_spad_to_mfifo() -> u32 {
        state::bridge::send_spad_to_mfifo(&mem())
    }
    /// `Sg_GetIsPrerend()`: bit 14 of `yiGetRenderStatus()`.
    Sg_GetIsPrerend @ 0x0084_3210 => extern "C" fn sg_get_is_prerend() -> u32 {
        state::bridge::get_is_prerend(ffx_field::yi::bridge::yi_get_render_status(&mem()))
    }
    /// `Sg_GetSpadPtr()`.
    Sg_GetSpadPtr @ 0x0084_3220 => extern "C" fn sg_get_spad_ptr() -> u32 {
        state::bridge::get_spad_ptr(&mem())
    }
    /// `Sg_IdExistShade()`.
    Sg_IdExistShade @ 0x0084_4bb0 => extern "C" fn sg_id_exist_shade() -> i32 {
        state::bridge::id_exist_shade(&mem())
    }
    /// `Sg_Id_coltest()`.
    Sg_Id_coltest @ 0x0084_4cb0 => extern "C" fn sg_id_coltest() -> u32 {
        state::bridge::id_coltest(&mut mem())
    }
    /// `Sg_MapGetDirSelect()`.
    Sg_MapGetDirSelect @ 0x0084_4cd0 => extern "C" fn sg_map_get_dir_select() -> u32 {
        state::bridge::map_get_dir_select(&mem())
    }
    /// `Sg_MapGetEnableShade()`.
    Sg_MapGetEnableShade @ 0x0084_4ce0 => extern "C" fn sg_map_get_enable_shade() -> i32 {
        state::bridge::map_get_enable_shade(&mem())
    }
    /// `Sg_MapGetName(which)`.
    Sg_MapGetName @ 0x0084_4cf0 => extern "C" fn sg_map_get_name(which: u32) -> u32 {
        state::bridge::map_get_name(&mem(), which)
    }
    /// `Sg_MapSetDirSelect(v)`.
    Sg_MapSetDirSelect @ 0x0084_4f90 => extern "C" fn sg_map_set_dir_select(v: u32) -> u32 {
        state::bridge::map_set_dir_select(&mut mem(), v)
    }
    /// `Sg_MapSetEnableShade(v)` (a byte).
    Sg_MapSetEnableShade @ 0x0084_4fa0 => extern "C" fn sg_map_set_enable_shade(v: u32) {
        state::bridge::map_set_enable_shade(&mut mem(), v);
    }
    /// `Sg_SwitchDrawId()`.
    Sg_SwitchDrawId @ 0x0084_5010 => extern "C" fn sg_switch_draw_id() -> i32 {
        state::bridge::switch_draw_id(&mut mem())
    }
    /// `Sg_GetFuncName()`.
    Sg_GetFuncName @ 0x0084_56c0 => extern "C" fn sg_get_func_name() -> u32 {
        state::bridge::get_func_name(&mem())
    }
    /// `sg_getnorm()`.
    sg_getnorm @ 0x0084_5c50 => extern "C" fn sg_getnorm() -> u32 {
        state::bridge::get_norm(&mem())
    }
    /// `sg_getvtex()`.
    sg_getvtex @ 0x0084_5c60 => extern "C" fn sg_getvtex() -> u32 {
        state::bridge::get_vtex(&mem())
    }
    /// `sg_setvtex_and_normal(vtex, norm)`.
    sg_setvtex_and_normal @ 0x0084_5ca0 => extern "C" fn sg_setvtex_and_normal(vtex: u32, norm: u32) -> u32 {
        state::bridge::set_vtex_and_normal(&mut mem(), vtex, norm)
    }
    /// `sg_GetY()`.
    sg_GetY @ 0x0085_0830 => extern "C" fn sg_get_y() -> u32 {
        state::bridge::get_y(&mem())
    }
    /// `SetPrintfFunc(v)`.
    SetPrintfFunc @ 0x0085_10c0 => extern "C" fn set_printf_func(v: u32) -> u32 {
        state::bridge::set_printf_func(&mut mem(), v)
    }
    /// `Sg_MouseGetMove()`.
    Sg_MouseGetMove @ 0x0085_1910 => extern "C" fn sg_mouse_get_move() -> u32 {
        state::bridge::mouse_get_move(&mem())
    }
}
