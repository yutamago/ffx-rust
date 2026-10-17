//! `ffx-sg`: the main module (`Sg`): switches and accessors of the main
//! loop, point lights, the GS packet writers, small vector helpers,
//! `crossAssertIsValidFloat`.

use ffx_sg::quat::Crt;
use ffx_sg::{
    camera, chpacket, cloth, control, envelope, gs, light, player, quat, seq, state, vec,
};

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

    // Round 2.
    /// `FFXApplication::getAppTime(this)`, with the time from the game's own clock.
    FFXApplication_getAppTime @ 0x0042_f690 => extern "thiscall" fn ffx_application_get_app_time(this: u32) -> f32 {
        state::bridge::get_app_time(&mem(), this, app_clock())
    }
    /// `sa_task_start()`.
    sa_task_start @ 0x0077_a2c0 => extern "C" fn sa_task_start() {
        state::bridge::sa_task_start(&mut mem());
    }
    /// `Cam_GetPos(cam, side, x, y, z)`.
    Cam_GetPos @ 0x007b_d420 => extern "C" fn cam_get_pos(cam: u32, side: u32, x: u32, y: u32, z: u32) -> u32 {
        camera::bridge::cam_get_pos(&mut mem(), cam, side, [x, y, z])
    }
    /// `Cam_SetChannelSuspended(id, on)`.
    Cam_SetChannelSuspended @ 0x007c_0010 => extern "C" fn cam_set_channel_suspended(id: u32, on: u32) {
        camera::bridge::cam_set_channel_suspended(&mut mem(), id, on);
    }
    /// `Sg_GlobalGetWind(x, y, z)`.
    Sg_GlobalGetWind @ 0x0082_4dd0 => extern "C" fn sg_global_get_wind(x: u32, y: u32, z: u32) {
        vec::bridge::global_get_wind(&mut mem(), x, y, z);
    }
    /// `Sg_MatrixApplyVec3(out, m, v)`.
    Sg_MatrixApplyVec3 @ 0x0082_4e60 => extern "C" fn sg_matrix_apply_vec3(out: u32, m: u32, v: u32) {
        vec::bridge::matrix_apply_vec3(&mut mem(), out, m, v);
    }
    /// `Sg_VecCross(out, a, b)`.
    Sg_VecCross @ 0x0082_95a0 => extern "C" fn sg_vec_cross(out: u32, a: u32, b: u32) {
        vec::bridge::cross(&mut mem(), out, a, b);
    }
    /// `Angle_Wrap_00829800(x)`.
    Angle_Wrap_00829800 @ 0x0082_9800 => extern "C" fn angle_wrap_00829800(x: f32) -> f32 {
        vec::bridge::angle_wrap(x)
    }
    /// `sg_getdbname(kind)`.
    sg_getdbname @ 0x0082_9c90 => extern "C" fn sg_getdbname(kind: u32) -> u32 {
        state::bridge::get_db_name(&mem(), kind)
    }
    /// `sg_getdbnum(c)`.
    sg_getdbnum @ 0x0082_9d80 => extern "C" fn sg_getdbnum(c: i32) -> i32 {
        state::bridge::get_db_num(c)
    }
    /// `sg_getdbtype(kind)`.
    sg_getdbtype @ 0x0082_9e10 => extern "C" fn sg_getdbtype(kind: u32) -> u32 {
        state::bridge::get_db_type(kind)
    }
    /// `Sg_AccSetClear(r, g, b, a)`.
    Sg_AccSetClear @ 0x0082_bdc0 => extern "C" fn sg_acc_set_clear(r: u32, g: u32, b: u32, a: u32) {
        state::bridge::acc_set_clear(&mut mem(), r, g, b, a);
    }
    /// `Sg_FilterAdd(r, g, b, time)`.
    Sg_FilterAdd @ 0x0082_cc20 => extern "C" fn sg_filter_add(r: u32, g: u32, b: u32, time: u32) -> u32 {
        state::bridge::filter_add(&mut mem(), r, g, b, time)
    }
    /// `Sg_Flash(time, r, g, b)`.
    Sg_Flash @ 0x0082_cca0 => extern "C" fn sg_flash(time: u32, r: u32, g: u32, b: u32) {
        state::bridge::flash(&mut mem(), time, r, g, b);
    }
    /// `Sg_Fade_Common(time, out, black)`.
    Sg_Fade_Common @ 0x0082_cdc0 => extern "C" fn sg_fade_common(time: u32, out: u32, black: u32) -> u32 {
        state::bridge::fade_common(&mut mem(), time, out, black)
    }
    /// `Sg_SetFocus(i, v)`.
    Sg_SetFocus @ 0x0082_ce20 => extern "C" fn sg_set_focus(i: u32, v: u32) -> u32 {
        state::bridge::set_focus(&mut mem(), i, v)
    }
    /// `Sg_GetCurExecFrames()`.
    Sg_GetCurExecFrames @ 0x0082_d7e0 => extern "C" fn sg_get_cur_exec_frames() -> i32 {
        state::bridge::get_cur_exec_frames(&mem())
    }
    /// `Sg_GetPlayerPos(v)`.
    Sg_GetPlayerPos @ 0x0082_d880 => extern "C" fn sg_get_player_pos(v: u32) -> u32 {
        state::bridge::get_player_pos(&mut mem(), v)
    }
    /// `Sg_SetPlayer(chr)`.
    Sg_SetPlayer @ 0x0082_dad0 => extern "C" fn sg_set_player(chr: u32) -> u32 {
        state::bridge::set_player(&mut mem(), chr)
    }
    /// `Angle_Wrap(x)`.
    Angle_Wrap @ 0x0082_db50 => extern "C" fn angle_wrap(x: f32) -> f32 {
        vec::bridge::angle_wrap(x)
    }
    /// `Sg_PLightDefine(i, x, y, z, radius, r, g, b, light)`.
    Sg_PLightDefine @ 0x0083_19b0 => extern "C" fn sg_p_light_define(i: i32, x: f32, y: f32, z: f32, radius: f32, r: i32, g: i32, b: i32, light: u32) {
        light::bridge::define(&mut mem(), i, [x, y, z], radius, [r, g, b], light);
    }
    /// `Sg_AnimValF(v)`.
    Sg_AnimValF @ 0x0083_6020 => extern "C" fn sg_anim_val_f(v: u32) -> u32 {
        light::bridge::anim_val_f(&mut mem(), v)
    }
    /// `Sg_GetLightPos(_, out)`.
    Sg_GetLightPos @ 0x0083_6070 => extern "C" fn sg_get_light_pos(_unused: u32, out: u32) -> u32 {
        light::bridge::get_light_pos(&mut mem(), out)
    }
    /// `Sg_InitLight()`.
    Sg_InitLight @ 0x0083_60b0 => extern "C" fn sg_init_light() -> u32 {
        light::bridge::init_light(&mut mem())
    }
    /// `Sg_LightGetColor(i, out)`.
    Sg_LightGetColor @ 0x0083_60f0 => extern "C" fn sg_light_get_color(i: u32, out: u32) {
        light::bridge::get_color_of(&mut mem(), i, out);
    }
    /// `Sg_LightGetDir(i, out)`.
    Sg_LightGetDir @ 0x0083_61c0 => extern "C" fn sg_light_get_dir(i: u32, out: u32) {
        light::bridge::get_dir(&mut mem(), i, out);
    }
    /// `Sg_LightSetChr(chr)`.
    Sg_LightSetChr @ 0x0083_6250 => extern "C" fn sg_light_set_chr(chr: u32) -> u32 {
        light::bridge::set_chr(&mut mem(), chr)
    }
    /// `Angle_Wrap_00836720(x)`.
    Angle_Wrap_00836720 @ 0x0083_6720 => extern "C" fn angle_wrap_00836720(x: f32) -> f32 {
        vec::bridge::angle_wrap(x)
    }
    /// `SG_RelocateSequence(seq, base)`.
    SG_RelocateSequence @ 0x0083_6fc0 => extern "C" fn sg_relocate_sequence(seq: u32, base: u32) -> u32 {
        seq::bridge::relocate_sequence(&mut mem(), seq, base)
    }
    /// `SG_StepChannels(obj, frames)`.
    SG_StepChannels @ 0x0083_94d0 => extern "C" fn sg_step_channels(obj: u32, frames: i32) -> u32 {
        seq::bridge::step_channels(&mut mem(), obj, frames)
    }
    /// `Angle_Wrap_0083a010(x)`.
    Angle_Wrap_0083a010 @ 0x0083_a010 => extern "C" fn angle_wrap_0083a010(x: f32) -> f32 {
        vec::bridge::angle_wrap(x)
    }
    /// `Sg_CaptureEnable()`.
    Sg_CaptureEnable @ 0x0083_a470 => extern "C" fn sg_capture_enable() -> u32 {
        state::bridge::capture_enable(&mut mem())
    }
    /// `Sg_CaptureEnable_Kari(mode, on)`.
    Sg_CaptureEnable_Kari @ 0x0083_a4f0 => extern "C" fn sg_capture_enable_kari(mode: u32, on: u32) {
        state::bridge::capture_enable_kari(&mut mem(), mode, on);
    }
    /// `Sg_CaptureEnable_KariOn(mode)`.
    Sg_CaptureEnable_KariOn @ 0x0083_a580 => extern "C" fn sg_capture_enable_kari_on(mode: u32) -> u32 {
        state::bridge::capture_enable_kari_on(&mut mem(), mode)
    }
    /// `Sg_CaptureSetGui(v)`.
    Sg_CaptureSetGui @ 0x0083_a660 => extern "C" fn sg_capture_set_gui(v: u32) {
        state::bridge::capture_set_gui(&mut mem(), v);
    }
    /// `Sg_GetTkAntiXYB(x, y, b)`.
    Sg_GetTkAntiXYB @ 0x0083_adf0 => extern "C" fn sg_get_tk_anti_xyb(x: u32, y: u32, b: u32) {
        state::bridge::get_tk_anti_xyb(&mut mem(), [x, y, b]);
    }
    /// `Sg_CalcScreenMatrix()`.
    Sg_CalcScreenMatrix @ 0x0083_f160 => extern "C" fn sg_calc_screen_matrix() {
        camera::bridge::calc_screen_matrix(&mut mem());
    }
    /// `Sg_CalcViewClipMatrix(m, wide)`.
    Sg_CalcViewClipMatrix @ 0x0083_f1c0 => extern "C" fn sg_calc_view_clip_matrix(m: u32, wide: u32) -> u32 {
        camera::bridge::calc_view_clip_matrix(&mut mem(), m, wide)
    }
    /// `Sg_CameraGetPos(v)`.
    Sg_CameraGetPos @ 0x0083_f260 => extern "C" fn sg_camera_get_pos(v: u32) -> u32 {
        camera::bridge::camera_get_pos(&mut mem(), v)
    }
    /// `Sg_CameraGetRef(v)`.
    Sg_CameraGetRef @ 0x0083_f2a0 => extern "C" fn sg_camera_get_ref(v: u32) -> u32 {
        camera::bridge::camera_get_ref(&mut mem(), v)
    }
    /// `Sg_GetLookVector(v)`.
    Sg_GetLookVector @ 0x0083_f710 => extern "C" fn sg_get_look_vector(v: u32) -> u32 {
        camera::bridge::get_look_vector(&mut mem(), v)
    }
    /// `Sg_SetScreenCenter(x, y)`.
    Sg_SetScreenCenter @ 0x0083_f7a0 => extern "C" fn sg_set_screen_center(x: i32, y: i32) -> i32 {
        camera::bridge::set_screen_center(&mut mem(), x, y)
    }
    /// `Sg_GetResource(i, default)`.
    Sg_GetResource @ 0x0083_fbe0 => extern "C" fn sg_get_resource(i: u32, default: u32) -> u32 {
        state::bridge::get_resource(&mem(), i, default)
    }
    /// `Sg_SetResource(i, v)`.
    Sg_SetResource @ 0x0083_fe10 => extern "C" fn sg_set_resource(i: u32, v: u32) {
        state::bridge::set_resource(&mut mem(), i, v);
    }
    /// `Sg_SetResourceSTR(i, s)`.
    Sg_SetResourceSTR @ 0x0083_fe60 => extern "C" fn sg_set_resource_str(i: u32, s: u32) {
        state::bridge::set_resource_str(&mut mem(), i, s);
    }
    /// `Sg_Db2DVertex(x, y)`.
    Sg_Db2DVertex @ 0x0084_0fc0 => extern "C" fn sg_db2_d_vertex(x: u32, y: u32) {
        gs::bridge::db_2d_vertex(&mut mem(), x, y);
    }
    /// `Sg_DbCircle(c, r)`.
    Sg_DbCircle @ 0x0084_10a0 => extern "C" fn sg_db_circle(c: u32, r: f32) {
        gs::bridge::db_circle_xz(&mut mem(), c, r);
    }
    /// `Sg_DbCircleXY(c, r)`.
    Sg_DbCircleXY @ 0x0084_1170 => extern "C" fn sg_db_circle_xy(c: u32, r: f32) {
        gs::bridge::db_circle_xy(&mut mem(), c, r);
    }
    /// `Sg_DbCircleYZ(c, r)`.
    Sg_DbCircleYZ @ 0x0084_11d0 => extern "C" fn sg_db_circle_yz(c: u32, r: f32) {
        gs::bridge::db_circle_yz(&mut mem(), c, r);
    }
    /// `Sg_DbColor(r, g, b)`.
    Sg_DbColor @ 0x0084_1230 => extern "C" fn sg_db_color(r: u32, g: u32, b: u32) {
        gs::bridge::db_color(&mut mem(), r, g, b);
    }
    /// `Sg_DbNull(c, s)`.
    Sg_DbNull @ 0x0084_14b0 => extern "C" fn sg_db_null(c: u32, s: f32) {
        gs::bridge::db_null(&mut mem(), c, s);
    }
    /// `Sg_GsSetPacket(p)`.
    Sg_GsSetPacket @ 0x0084_1cf0 => extern "C" fn sg_gs_set_packet(p: u32) -> u32 {
        gs::bridge::set_packet(&mut mem(), p)
    }
    /// `Sg_GsZBUF(zmsk)`.
    Sg_GsZBUF @ 0x0084_22f0 => extern "C" fn sg_gs_zbuf(zmsk: u32) -> u32 {
        gs::bridge::zbuf_1(&mut mem(), zmsk)
    }
    /// `Sg_GsZBUF2(zmsk)`.
    Sg_GsZBUF2 @ 0x0084_2350 => extern "C" fn sg_gs_zbuf2(zmsk: u32) -> u32 {
        gs::bridge::zbuf_2(&mut mem(), zmsk)
    }
    /// `Sg_MapInitAll()`.
    Sg_MapInitAll @ 0x0084_4eb0 => extern "C" fn sg_map_init_all() {
        state::bridge::map_init_all(&mut mem());
    }
    /// `Sg_MapSetName(name, which)`.
    Sg_MapSetName @ 0x0084_4fb0 => extern "C" fn sg_map_set_name(name: u32, which: u32) {
        state::bridge::map_set_name(&mut mem(), name, which);
    }
    /// `Angle_Wrap_00845640(x)`.
    Angle_Wrap_00845640 @ 0x0084_5640 => extern "C" fn angle_wrap_00845640(x: f32) -> f32 {
        vec::bridge::angle_wrap(x)
    }
    /// `Angle_Wrap_0084eb60(x)`.
    Angle_Wrap_0084eb60 @ 0x0084_eb60 => extern "C" fn angle_wrap_0084eb60(x: f32) -> f32 {
        vec::bridge::angle_wrap(x)
    }
    /// `sg_AddY(d)`.
    sg_AddY @ 0x0085_0820 => extern "C" fn sg_add_y(d: u32) -> u32 {
        state::bridge::add_y(&mut mem(), d)
    }
    /// `Sg_SetDebugTEX_0(q)`.
    Sg_SetDebugTEX_0 @ 0x0085_1160 => extern "C" fn sg_set_debug_tex_0(q: u32) -> u32 {
        gs::bridge::set_debug_tex0(&mut mem(), q)
    }
    /// `Sg_SetMouse(x, y)`.
    Sg_SetMouse @ 0x0085_1ae0 => extern "C" fn sg_set_mouse(x: i32, y: i32) -> i32 {
        state::bridge::set_mouse(&mut mem(), x, y)
    }

    // Round 3.
    /// `eidprintfPrint()`.
    eidprintfPrint @ 0x0076_b8d0 => extern "C" fn eidprintf_print() {
        state::bridge::eid_printf_print(&mut mem());
    }
    /// `SPAD_CALL()`: never implemented on PC.
    SPAD_CALL @ 0x0077_a480 => extern "C" fn spad_call() {
        // Only a warning through the empty `dbgPrintf`.
    }
    /// `rgb32_to_24_asm()`: never implemented on PC.
    rgb32_to_24_asm @ 0x0077_a490 => extern "C" fn rgb32_to_24_asm() {
        // Only a warning through the empty `dbgPrintf`.
    }
    /// `sa_printf_set()`: never implemented on PC.
    sa_printf_set @ 0x0077_a4a0 => extern "C" fn sa_printf_set() {
        // Only a warning through the empty `dbgPrintf`.
    }
    /// `Cam_IsMoving(cam, side)`.
    Cam_IsMoving @ 0x007b_dd20 => extern "C" fn cam_is_moving(cam: u32, side: u32) -> u32 {
        camera::bridge::cam_is_moving(&mem(), cam, side)
    }
    /// `Sg_GetUserName2()`.
    Sg_GetUserName2 @ 0x0082_0620 => extern "C" fn sg_get_user_name2() -> u32 {
        state::bridge::get_user_name2(&mem())
    }
    /// `Sg_MainCalcRate()`.
    Sg_MainCalcRate @ 0x0082_06c0 => extern "C" fn sg_main_calc_rate() -> i32 {
        state::bridge::main_calc_rate(&mut mem())
    }
    /// `Sg_MenuSetReserve(request)`.
    Sg_MenuSetReserve @ 0x0082_1750 => extern "C" fn sg_menu_set_reserve(request: u32) {
        state::bridge::menu_set_reserve(&mut mem(), request, |m| {
            ffx_menu::tk::bridge::tk_menu_save_load_active(m);
        });
    }
    /// `getUCStatus()`.
    getUCStatus @ 0x0082_2460 => extern "C" fn get_uc_status() -> u32 {
        state::bridge::get_uc_status(&mem())
    }
    /// `set0434isDarkYojimbo(id)`.
    set0434isDarkYojimbo @ 0x0082_9c60 => extern "C" fn set0434is_dark_yojimbo(id: u32) {
        state::bridge::set_dark_yojimbo(&mut mem(), id);
    }
    /// `Sg_FadeIn(time)`.
    Sg_FadeIn @ 0x0082_cb80 => extern "C" fn sg_fade_in(time: u32) -> u32 {
        state::bridge::fade_in(&mut mem(), time)
    }
    /// `Sg_FadeInW(time)`.
    Sg_FadeInW @ 0x0082_cba0 => extern "C" fn sg_fade_in_w(time: u32) -> u32 {
        state::bridge::fade_in_w(&mut mem(), time)
    }
    /// `Sg_FadeOut(time)`.
    Sg_FadeOut @ 0x0082_cbc0 => extern "C" fn sg_fade_out(time: u32) -> u32 {
        state::bridge::fade_out(&mut mem(), time)
    }
    /// `Sg_FadeOutW(time)`.
    Sg_FadeOutW @ 0x0082_cbe0 => extern "C" fn sg_fade_out_w(time: u32) -> u32 {
        state::bridge::fade_out_w(&mut mem(), time)
    }
    /// `Sg_FilterSub(r, g, b, time)`.
    Sg_FilterSub @ 0x0082_cc80 => extern "C" fn sg_filter_sub(r: u32, g: u32, b: u32, time: u32) -> u32 {
        state::bridge::filter_sub(&mut mem(), r, g, b, time)
    }
    /// `Sg_FocusReset()`.
    Sg_FocusReset @ 0x0082_ccf0 => extern "C" fn sg_focus_reset() -> u32 {
        state::bridge::focus_reset(&mut mem())
    }
    /// `Sg_ToggleFocus()`.
    Sg_ToggleFocus @ 0x0082_cd80 => extern "C" fn sg_toggle_focus() {
        state::bridge::toggle_focus(&mut mem());
    }
    /// `Sg_DrawSetFrame()`.
    Sg_DrawSetFrame @ 0x0082_cf80 => extern "C" fn sg_draw_set_frame() {
        gs::bridge::draw_set_frame(&mut mem());
    }
    /// `Sg_PlayerSetDirFix(on, degrees)`.
    Sg_PlayerSetDirFix @ 0x0082_d990 => extern "C" fn sg_player_set_dir_fix(on: u32, degrees: f32) {
        state::bridge::player_set_dir_fix(&mut mem(), on, degrees);
    }
    /// `Sg_AngleApproach(cur, target, step)`.
    Sg_AngleApproach @ 0x0083_5f80 => extern "C" fn sg_angle_approach(cur: f32, target: f32, step: f32) -> f32 {
        vec::bridge::angle_approach(cur, target, step)
    }
    /// `Sg_VectorGetLength2(a, b)`.
    Sg_VectorGetLength2 @ 0x0083_66a0 => extern "C" fn sg_vector_get_length2(a: u32, b: u32) -> f32 {
        vec::bridge::vector_get_length2(&mem(), a, b)
    }
    /// `SG_RemoveMotionGroup(group)`.
    SG_RemoveMotionGroup @ 0x0083_6da0 => extern "C" fn sg_remove_motion_group(group: u32) {
        seq::bridge::remove_motion_group(&mut mem(), group);
    }
    /// `SG_RemoveSequence(seq)`.
    SG_RemoveSequence @ 0x0083_6de0 => extern "C" fn sg_remove_sequence(seq: u32) {
        seq::bridge::remove_sequence(&mut mem(), seq);
    }
    /// `SG_FindMotionGroup(a, b, key)`.
    SG_FindMotionGroup @ 0x0083_6f10 => extern "C" fn sg_find_motion_group(a: i32, b: i32, key: u32) -> u32 {
        seq::bridge::find_motion_group(&mem(), a, b, key)
    }
    /// `SG_RelocateGroup(group)`.
    SG_RelocateGroup @ 0x0083_6f70 => extern "C" fn sg_relocate_group(group: u32) {
        seq::bridge::relocate_group(&mut mem(), group);
    }
    /// `SG_RawToRad(x)`.
    SG_RawToRad @ 0x0083_8c60 => extern "C" fn sg_raw_to_rad(x: f32) -> f32 {
        vec::bridge::raw_to_rad(&mem(), x)
    }
    /// `SG_RewindChannels(obj)`.
    SG_RewindChannels @ 0x0083_98f0 => extern "C" fn sg_rewind_channels(obj: u32) {
        seq::bridge::rewind_channels(&mut mem(), obj);
    }
    /// `Sg_ToggleEnableTkAnti()`.
    Sg_ToggleEnableTkAnti @ 0x0083_b100 => extern "C" fn sg_toggle_enable_tk_anti() {
        state::bridge::toggle_enable_tk_anti(&mut mem());
    }
    /// `Sg_MemCache(p)`.
    Sg_MemCache @ 0x0083_b700 => extern "C" fn sg_mem_cache(p: u32) -> u32 {
        state::bridge::mem_cache(p)
    }
    /// `Sg_MemCache_SerchFree(p)`.
    Sg_MemCache_SerchFree @ 0x0083_b770 => extern "C" fn sg_mem_cache_serch_free(p: u32) -> u32 {
        state::bridge::mem_cache_serch_free(&mem(), p)
    }
    /// `Sg_MemClearStr(p)`.
    Sg_MemClearStr @ 0x0083_bab0 => extern "C" fn sg_mem_clear_str(p: u32) {
        state::bridge::mem_clear_str(&mut mem(), p);
    }
    /// `Sg_MemGetStr(p)`.
    Sg_MemGetStr @ 0x0083_c070 => extern "C" fn sg_mem_get_str(p: u32) -> u32 {
        state::bridge::mem_get_str(&mem(), p)
    }
    /// `Sg_MemGetSum(header)`.
    Sg_MemGetSum @ 0x0083_c0b0 => extern "C" fn sg_mem_get_sum(header: u32) -> u16 {
        state::bridge::mem_get_sum(&mem(), header)
    }
    /// `Sg_MemIsFree(p)`.
    Sg_MemIsFree @ 0x0083_c320 => extern "C" fn sg_mem_is_free(p: u32) -> u32 {
        state::bridge::mem_is_free(&mem(), p)
    }
    /// `Sg_MemSetAlign(align)`.
    Sg_MemSetAlign @ 0x0083_c4a0 => extern "C" fn sg_mem_set_align(align: u32) -> u32 {
        state::bridge::mem_set_align(&mut mem(), align)
    }
    /// `Sg_MemSetRtAddr(a)`.
    Sg_MemSetRtAddr @ 0x0083_c560 => extern "C" fn sg_mem_set_rt_addr(a: u32) -> u32 {
        state::bridge::mem_set_rt_addr(&mut mem(), a)
    }
    /// `Sg_VectorGetLengthXZ(a, b)`.
    Sg_VectorGetLengthXZ @ 0x0083_e3e0 => extern "C" fn sg_vector_get_length_xz(a: u32, b: u32) -> f32 {
        vec::bridge::vector_get_length_xz(&mem(), a, b)
    }
    /// `Sg_Debug(what, v)`.
    Sg_Debug @ 0x0083_e5c0 => extern "C" fn sg_debug(what: u32, v: u32) {
        state::bridge::debug(&mut mem(), what, v);
    }
    /// `Sg_IdEventInitAll()`.
    Sg_IdEventInitAll @ 0x0083_e8d0 => extern "C" fn sg_id_event_init_all() {
        state::bridge::id_event_init_all(&mut mem());
    }
    /// `Sg_IdGetMapTable(i)`.
    Sg_IdGetMapTable @ 0x0083_e960 => extern "C" fn sg_id_get_map_table(i: u32) -> u32 {
        state::bridge::id_get_map_table(&mem(), i)
    }
    /// `Sg_IdGetShade(c)`.
    Sg_IdGetShade @ 0x0083_e980 => extern "C" fn sg_id_get_shade(c: u32) -> u32 {
        state::bridge::id_get_shade(&mem(), c)
    }
    /// `Sg_IdSetMapTable(i, v)`.
    Sg_IdSetMapTable @ 0x0083_e9e0 => extern "C" fn sg_id_set_map_table(i: u32, v: u32) -> u32 {
        state::bridge::id_set_map_table(&mut mem(), i, v)
    }
    /// `Sg_IdSetPtr(id)`.
    Sg_IdSetPtr @ 0x0083_ea00 => extern "C" fn sg_id_set_ptr(id: u32) {
        state::bridge::id_set_ptr(&mut mem(), id);
    }
    /// `Sg_DbColor8(c)`.
    Sg_DbColor8 @ 0x0084_1270 => extern "C" fn sg_db_color8(c: u32) {
        gs::bridge::db_color8(&mut mem(), c);
    }
    /// `Sg_CreateVectorFromHV(out, h, v)`.
    Sg_CreateVectorFromHV @ 0x0084_50d0 => extern "C" fn sg_create_vector_from_hv(out: u32, h: f32, v: f32) {
        quat::bridge::create_vector_from_hv(&mut mem(), out, h, v, &mut crt);
    }
    /// `Sg_MatrixToQuaternion(m, q)`.
    Sg_MatrixToQuaternion @ 0x0084_5170 => extern "C" fn sg_matrix_to_quaternion(m: u32, q: u32) {
        quat::bridge::matrix_to_quaternion(&mut mem(), m, q);
    }
    /// `Sg_QuaternionSlerp(a, b, t, out)`.
    Sg_QuaternionSlerp @ 0x0084_5310 => extern "C" fn sg_quaternion_slerp(a: u32, b: u32, t: f32, out: u32) -> u32 {
        quat::bridge::quaternion_slerp(&mut mem(), a, b, t, out, &mut crt)
    }
    /// `Sg_QuaternionToMatrix(q, m)`.
    Sg_QuaternionToMatrix @ 0x0084_5450 => extern "C" fn sg_quaternion_to_matrix(q: u32, m: u32) {
        quat::bridge::quaternion_to_matrix(&mut mem(), q, m);
    }
    /// `Sg_VectorToMatrix2(m, v)`.
    Sg_VectorToMatrix2 @ 0x0084_5560 => extern "C" fn sg_vector_to_matrix2(m: u32, v: u32) -> u32 {
        quat::bridge::vector_to_matrix2(&mut mem(), m, v)
    }
    /// `sg_normalize(v, n)`.
    sg_normalize @ 0x0084_6710 => extern "C" fn sg_normalize(v: u32, n: u32) {
        quat::bridge::normalize(&mut mem(), v, n);
    }
    /// `SgGu_LightDump()`: nothing (its output goes to the empty `dbgPrintf`).
    SgGu_LightDump @ 0x0084_e870 => extern "C" fn sg_gu_light_dump() {
        // Only `dbgPrintf` (empty) of the light table.
    }
    /// `Sg_MouseButtonMain()`.
    Sg_MouseButtonMain @ 0x0085_17c0 => extern "C" fn sg_mouse_button_main() -> u32 {
        state::bridge::mouse_button_main(&mut mem())
    }
    /// `sa_meth01dec2_ex()`: never implemented on PC; 0.
    sa_meth01dec2_ex @ 0x00a4_3ca0 => extern "C" fn sa_meth01dec2_ex() -> u32 {
        state::bridge::not_implemented()
    }
    /// `sa_meth01dec_ex()`: never implemented on PC; 0.
    sa_meth01dec_ex @ 0x00a4_3cb0 => extern "C" fn sa_meth01dec_ex() -> u32 {
        state::bridge::not_implemented()
    }
    // Round 3: the character packet setters.
    /// `Ch_PacketSetName(v)`.
    Ch_PacketSetName @ 0x0084_3000 => extern "C" fn ch_packet_set_name(v: u32) -> u32 {
        chpacket::bridge::set_name(&mut mem(), v)
    }
    /// `Ch_PacketSetCull(v)`.
    Ch_PacketSetCull @ 0x0084_3010 => extern "C" fn ch_packet_set_cull(v: u32) {
        chpacket::bridge::set_cull(&mut mem(), v);
    }
    /// `Ch_PacketSetFSpecular(v)`.
    Ch_PacketSetFSpecular @ 0x0084_3020 => extern "C" fn ch_packet_set_fspecular(v: u32) {
        chpacket::bridge::set_fspecular(&mut mem(), v);
    }
    /// `Ch_PacketSetShadow(v)`.
    Ch_PacketSetShadow @ 0x0084_3040 => extern "C" fn ch_packet_set_shadow(v: u32) {
        chpacket::bridge::set_shadow(&mut mem(), v);
    }
    /// `Ch_PacketSetShadowColor(r, g, b)`.
    Ch_PacketSetShadowColor @ 0x0084_3050 => extern "C" fn ch_packet_set_shadow_color(r: u32, g: u32, b: u32) {
        chpacket::bridge::set_shadow_color(&mut mem(), r, g, b);
    }
    /// `Ch_PacketSetShadowSemi(v)`.
    Ch_PacketSetShadowSemi @ 0x0084_3070 => extern "C" fn ch_packet_set_shadow_semi(v: u32) {
        chpacket::bridge::set_shadow_semi(&mut mem(), v);
    }
    /// `Ch_PacketSetShadowTexture(v)`.
    Ch_PacketSetShadowTexture @ 0x0084_3080 => extern "C" fn ch_packet_set_shadow_texture(v: u32) {
        chpacket::bridge::set_shadow_texture(&mut mem(), v);
    }
    /// `Ch_PacketSetSpecCoefficient(x)`.
    Ch_PacketSetSpecCoefficient @ 0x0084_3090 => extern "C" fn ch_packet_set_spec_coefficient(x: f32) {
        chpacket::bridge::set_spec_coefficient(&mut mem(), x);
    }
    /// `Ch_PacketSetSpecColor(r, g, b, a)`.
    Ch_PacketSetSpecColor @ 0x0084_30a0 => extern "C" fn ch_packet_set_spec_color(r: u32, g: u32, b: u32, a: u32) {
        chpacket::bridge::set_spec_color(&mut mem(), r, g, b, a);
    }
    /// `Ch_PacketSetSpecSize(x)`.
    Ch_PacketSetSpecSize @ 0x0084_30d0 => extern "C" fn ch_packet_set_spec_size(x: f32) {
        chpacket::bridge::set_spec_size(&mut mem(), x);
    }
    /// `Ch_PacketSetSpecular(v)`.
    Ch_PacketSetSpecular @ 0x0084_30e0 => extern "C" fn ch_packet_set_specular(v: u32) {
        chpacket::bridge::set_specular(&mut mem(), v);
    }
    /// `Ch_PacketSetTexture(v)`.
    Ch_PacketSetTexture @ 0x0084_30f0 => extern "C" fn ch_packet_set_texture(v: u32) {
        chpacket::bridge::set_texture(&mut mem(), v);
    }
    /// `Sg_SetMapAlphaFast(on)`.
    Sg_SetMapAlphaFast @ 0x0082_1b00 => extern "C" fn sg_set_map_alpha_fast(on: u32) -> u32 {
        control::bridge::set_map_alpha_fast(&mut mem(), on)
    }
    /// `Sg_SetMuteSound(on)`.
    Sg_SetMuteSound @ 0x0082_1c60 => extern "C" fn sg_set_mute_sound(on: u32) {
        control::bridge::set_mute_sound(&mut mem(), on);
    }
    /// `Sg_SetPauseAllSounds(on)`.
    Sg_SetPauseAllSounds @ 0x0082_1cc0 => extern "C" fn sg_set_pause_all_sounds(on: u32) {
        control::bridge::set_pause_all_sounds(&mut mem(), on);
    }
    /// `dreamParamReset()`.
    dreamParamReset @ 0x0082_2410 => extern "C" fn dream_param_reset() -> u32 {
        control::bridge::dream_param_reset(&mut mem())
    }
    /// `Sg_AccSetAlpha(alpha, frames)`.
    Sg_AccSetAlpha @ 0x0082_bd10 => extern "C" fn sg_acc_set_alpha(alpha: u32, frames: u32) -> u32 {
        control::bridge::acc_set_alpha(&mut mem(), alpha, frames)
    }
    /// `Sg_AccRequestCurrent()`.
    Sg_AccRequestCurrent @ 0x0082_bdf0 => extern "C" fn sg_acc_request_current() -> u32 {
        control::bridge::acc_request_current(&mut mem())
    }
    /// `Sg_Blur(weight)`.
    Sg_Blur @ 0x0082_bee0 => extern "C" fn sg_blur(weight: u32) -> u32 {
        control::bridge::blur(&mut mem(), weight)
    }
    /// `Sg_PLightDispose()`.
    Sg_PLightDispose @ 0x0083_1ba0 => extern "C" fn sg_plight_dispose() -> u32 {
        control::bridge::plight_dispose(&mut mem())
    }
    /// `Sg_LightSetDir(i, dir)`.
    Sg_LightSetDir @ 0x0083_6430 => extern "C" fn sg_light_set_dir(i: u32, dir: u32) -> u32 {
        control::bridge::light_set_dir(&mut mem(), i, dir)
    }
    /// `Sg_GsEnd()`.
    Sg_GsEnd @ 0x0084_1770 => extern "C" fn sg_gs_end() -> u32 {
        control::bridge::gs_end(&mut mem())
    }
    /// `Sg_PlayerSetDiving(v)`.
    Sg_PlayerSetDiving @ 0x0082_d9d0 => extern "C" fn sg_player_set_diving(v: u32) -> u8 {
        state::bridge::player_set_diving(&mut mem(), v)
    }
    /// `Sg_PlayerSetSpeed(kind, v)`.
    Sg_PlayerSetSpeed @ 0x0082_d9e0 => extern "C" fn sg_player_set_speed(kind: u32, v: f32) -> u32 {
        state::bridge::player_set_speed(&mut mem(), kind, v)
    }
    /// `Sg_SetCameraFollow(on)`.
    Sg_SetCameraFollow @ 0x0082_dab0 => extern "C" fn sg_set_camera_follow(on: u32) -> u8 {
        state::bridge::set_camera_follow(&mut mem(), on)
    }
    /// `SG_DisposeMotionGroup(a, b, key)`.
    SG_DisposeMotionGroup_00836e20 @ 0x0083_6e20 => extern "C" fn sg_dispose_motion_group(a: i32, b: i32, key: u32) -> u32 {
        seq::bridge::dispose_motion_group(&mut mem(), a, b, key)
    }
    /// `Sg_IdFixupAndSet(map)`.
    Sg_IdFixupAndSet_00844bc0 @ 0x0084_4bc0 => extern "C" fn sg_id_fixup_and_set(map: u32) {
        state::bridge::id_fixup_and_set(&mut mem(), map);
    }
    /// `Sg_IdInitAll()`.
    Sg_IdInitAll @ 0x0084_4ca0 => extern "C" fn sg_id_init_all() {
        state::bridge::id_init_all(&mut mem());
    }
    /// `Sg_PlayerInitAll()`.
    Sg_PlayerInitAll @ 0x0082_d910 => extern "C" fn sg_player_init_all() -> u32 {
        state::bridge::player_init_all(&mut mem())
    }
    /// `Sg_SetUserName(name)`.
    Sg_SetUserName @ 0x0082_1d30 => extern "C" fn sg_set_user_name(name: u32) -> u32 {
        state::bridge::set_user_name(&mut mem(), name)
    }
    /// `Sg_InitFilter()`.
    Sg_InitFilter @ 0x0082_cd50 => extern "C" fn sg_init_filter() -> u32 {
        state::bridge::init_filter(&mut mem())
    }
    /// `Sg_PLightAllocate(n)`.
    Sg_PLightAllocate @ 0x0083_1950 => extern "C" fn sg_plight_allocate(n: u32) -> u32 {
        control::bridge::plight_allocate(&mut mem(), n)
    }
    /// `Sg_InitArgs(argc, argv)`.
    Sg_InitArgs @ 0x0082_0310 => extern "C" fn sg_init_args(count: u32, args: u32) {
        control::bridge::init_args(&mut mem(), count, args);
    }
    /// `setFramSync()`.
    setFramSync @ 0x0082_2720 => extern "C" fn set_fram_sync() -> u32 {
        control::bridge::set_fram_sync(&mut mem())
    }
    /// `Sg_AccInitAll()`.
    Sg_AccInitAll @ 0x0082_bcd0 => extern "C" fn sg_acc_init_all() {
        control::bridge::acc_init_all(&mut mem());
    }
    /// `Sg_PlayerInitDefault()`.
    Sg_PlayerInitDefault @ 0x0082_d8d0 => extern "C" fn sg_player_init_default() -> u32 {
        state::bridge::player_init_default(&mut mem())
    }
    /// `Sg_LightSetDirXYZ(i, x, y, z)`.
    Sg_LightSetDirXYZ @ 0x0083_64f0 => extern "C" fn sg_light_set_dir_xyz(i: u32, x: f32, y: f32, z: f32) -> u32 {
        control::bridge::light_set_dir_xyz(&mut mem(), i, x, y, z)
    }
    /// `Sg_CalcTriangleNormal(v0, v1, v2, out)`.
    Sg_CalcTriangleNormal_0083e420 @ 0x0083_e420 => extern "C" fn sg_calc_triangle_normal(v0: u32, v1: u32, v2: u32, out: u32) {
        vec::bridge::calc_triangle_normal(&mut mem(), v0, v1, v2, out);
    }
    /// `Sg_MatrixFromDir(m, dir, rows)`.
    Sg_MatrixFromDir @ 0x0084_35a0 => extern "C" fn sg_matrix_from_dir(m: u32, dir: u32, rows: u32) -> u32 {
        vec::bridge::matrix_from_dir(&mut mem(), m, dir, rows)
    }
    /// `Sg_IntVecToFloat(out, v)`.
    Sg_IntVecToFloat @ 0x0084_5d90 => extern "C" fn sg_int_vec_to_float(out: u32, v: u32) -> u32 {
        vec::bridge::int_vec_to_float(&mut mem(), out, v)
    }
    /// `SG_ApplyBone(_, chan, bone, scale)`.
    SG_ApplyBone @ 0x0083_8280 => extern "C" fn sg_apply_bone(_a: u32, chan: u32, bone: u32, scale: f32) {
        seq::bridge::apply_bone(&mut mem(), chan, bone, scale, false);
    }
    /// `SG_ApplyBoneDamped(_, chan, bone, scale)`.
    SG_ApplyBoneDamped @ 0x0083_8470 => extern "C" fn sg_apply_bone_damped(_a: u32, chan: u32, bone: u32, scale: f32) {
        seq::bridge::apply_bone(&mut mem(), chan, bone, scale, true);
    }
    /// `SG_AddSequence(seq, v)`.
    SG_AddSequence @ 0x0083_6d30 => extern "C" fn sg_add_sequence(seq: u32, v: u32) {
        seq::bridge::add_sequence(&mut mem(), seq, v);
    }
    /// `Sg_IdMatchChr(chr, i)`.
    Sg_IdMatchChr @ 0x0083_e4e0 => extern "C" fn sg_id_match_chr(chr: u32, i: u32) -> u32 {
        state::bridge::id_match_chr(&mem(), chr, i)
    }
    /// `recallSceneEffect(layer)`.
    recallSceneEffect @ 0x0082_2580 => extern "C" fn recall_scene_effect(layer: u32) -> u32 {
        control::bridge::recall_scene_effect(&mut mem(), layer)
    }
    /// `recallSceneEffect2(a, b)`.
    recallSceneEffect2 @ 0x0082_2620 => extern "C" fn recall_scene_effect2(a: u32, b: u32) -> u32 {
        control::bridge::recall_scene_effect2(&mut mem(), a, b)
    }
    /// `Sg_ClothRodMaxPin(rods, n, points)`.
    Sg_ClothRodMaxPin @ 0x0082_2890 => extern "C" fn sg_cloth_rod_max_pin(rods: u32, n: u32, points: u32) -> u32 {
        vec::bridge::cloth_rods(&mut mem(), 1, rods, n, points)
    }
    /// `Sg_ClothRodMax(rods, n, points)`.
    Sg_ClothRodMax @ 0x0082_2960 => extern "C" fn sg_cloth_rod_max(rods: u32, n: u32, points: u32) -> u32 {
        vec::bridge::cloth_rods(&mut mem(), 2, rods, n, points)
    }
    /// `Sg_ClothRodMinPin(rods, n, points)`.
    Sg_ClothRodMinPin @ 0x0082_2a50 => extern "C" fn sg_cloth_rod_min_pin(rods: u32, n: u32, points: u32) -> u32 {
        vec::bridge::cloth_rods(&mut mem(), 3, rods, n, points)
    }
    /// `Sg_ClothRodMin(rods, n, points)`.
    Sg_ClothRodMin @ 0x0082_2b20 => extern "C" fn sg_cloth_rod_min(rods: u32, n: u32, points: u32) -> u32 {
        vec::bridge::cloth_rods(&mut mem(), 4, rods, n, points)
    }
    /// `Sg_ClothRodFixPin(rods, n, points)`.
    Sg_ClothRodFixPin @ 0x0082_2c10 => extern "C" fn sg_cloth_rod_fix_pin(rods: u32, n: u32, points: u32) -> u32 {
        vec::bridge::cloth_rods(&mut mem(), 5, rods, n, points)
    }
    /// `Sg_ClothRodFix(rods, n, points)`.
    Sg_ClothRodFix @ 0x0082_2cf0 => extern "C" fn sg_cloth_rod_fix(rods: u32, n: u32, points: u32) -> u32 {
        vec::bridge::cloth_rods(&mut mem(), 6, rods, n, points)
    }
    /// `Sg_SyncCutScene()`.
    Sg_SyncCutScene @ 0x0082_1d60 => extern "C" fn sg_sync_cut_scene() -> u32 {
        control::bridge::sync_cut_scene(&mut mem())
    }
    /// `Sg_IdGetNormal(out, i)`.
    Sg_IdGetNormal @ 0x0083_e130 => extern "C" fn sg_id_get_normal(out: u32, i: u32) -> u32 {
        state::bridge::id_get_normal(&mut mem(), out, i)
    }
    /// `SG_ApplyBoneSel(a, chan, bone, scale)`.
    SG_ApplyBoneSel @ 0x0083_8240 => extern "C" fn sg_apply_bone_sel(_a: u32, chan: u32, bone: u32, scale: f32) {
        seq::bridge::apply_bone_sel(&mut mem(), chan, bone, scale);
    }
    /// `drawDreamEffect()`.
    drawDreamEffect @ 0x0082_23a0 => extern "C" fn draw_dream_effect() -> u32 {
        control::bridge::draw_dream_effect(&mut mem())
    }
    /// `Ch_EffectPacketStoreStrip(v)`.
    Ch_EffectPacketStoreStrip @ 0x0084_4370 => extern "C" fn ch_effect_packet_store_strip(v: u32) -> u32 {
        control::bridge::effect_store_strip(&mut mem(), v)
    }
    /// `SG_SampleChannels(obj)`.
    SG_SampleChannels @ 0x0083_93c0 => extern "C" fn sg_sample_channels(obj: u32) {
        seq::bridge::sample_channels(&mut mem(), obj);
    }
    /// `SG_BlendToTarget(obj, n)`.
    SG_BlendToTarget @ 0x0083_95b0 => extern "C" fn sg_blend_to_target(obj: u32, n: i32) {
        seq::bridge::blend_to_target(&mut mem(), obj, n);
    }
    /// `Sg_IdSlideEdge(from, to)`.
    Sg_IdSlideEdge @ 0x0083_e180 => extern "C" fn sg_id_slide_edge(from: u32, to: u32) {
        state::bridge::id_slide_edge(&mut mem(), from, to);
    }
    /// `Sg_ChrPushApart(a, b, reach, dist)`.
    Sg_ChrPushApart @ 0x0083_ecd0 => extern "C" fn sg_chr_push_apart(a: u32, b: u32, reach: f32, dist: f32) -> u32 {
        state::bridge::chr_push_apart(&mut mem(), a, b, reach, dist)
    }
    /// `Sg_ClothPlaceAnchors(chr)`.
    Sg_ClothPlaceAnchors @ 0x0082_dbc0 => extern "C" fn sg_cloth_place_anchors(chr: u32) {
        vec::bridge::cloth_place_anchors(&mut mem(), chr);
    }
    /// `Sg_ClothStep(chr)`.
    Sg_ClothStep @ 0x0082_2de0 => extern "C" fn sg_cloth_step(chr: u32) {
        cloth::bridge::cloth_step(&mut mem(), chr, &mut crt);
    }
    /// `ExportValidateInterface(id)`.
    ExportValidateInterface @ 0x0082_02e0 => extern "C" fn export_validate_interface(id: u32) -> u32 {
        control::bridge::export_validate_interface(id)
    }
    /// `Sg_DrawFilter()`.
    Sg_DrawFilter @ 0x0082_c0c0 => extern "C" fn sg_draw_filter() {
        control::bridge::draw_filter(&mut mem());
    }
    /// `Sg_SetChrPacketState(v)`.
    Sg_SetChrPacketState @ 0x0084_5c30 => extern "C" fn sg_set_chr_packet_state(v: u32) {
        #[allow(clippy::cast_possible_truncation)] // the low half
        envelope::bridge::set_chr_packet_state(&mut mem(), v as u16);
    }
    /// `Sg_SetChrPacketWord(v)`.
    Sg_SetChrPacketWord @ 0x0084_5c40 => extern "C" fn sg_set_chr_packet_word(v: u32) {
        #[allow(clippy::cast_possible_truncation)] // the low half
        envelope::bridge::set_chr_packet_word(&mut mem(), v as u16);
    }
    /// `sg_Daddiu(dst, src, imm)`.
    sg_Daddiu @ 0x0084_5cc0 => extern "C" fn sg_daddiu(dst: u32, src: u32, imm: u32) -> u32 {
        envelope::bridge::daddiu(&mut mem(), dst, src, imm)
    }
    /// `sg_SetFloat(dst, x)`.
    sg_SetFloat @ 0x0084_5cf0 => extern "C" fn sg_set_float(dst: u32, x: f32) -> u32 {
        envelope::bridge::set_float(&mut mem(), dst, x)
    }
    /// `sg_Sll(dst, src, n)`.
    sg_Sll @ 0x0084_5d70 => extern "C" fn sg_sll(dst: u32, src: u32, n: u32) -> u32 {
        envelope::bridge::sll(&mut mem(), dst, src, n)
    }
    /// `sg_VmaddaScalar(-, v, -, s)`.
    sg_VmaddaScalar @ 0x0084_5df0 => extern "C" fn sg_vmadda_scalar(_a: u32, v: u32, _c: u32, s: f32) -> u32 {
        envelope::bridge::vmadda_scalar(&mut mem(), v, s)
    }
    /// `sg_VmaddScalar(dst, v, -, -, s)`.
    sg_VmaddScalar @ 0x0084_5e40 => extern "C" fn sg_vmadd_scalar(dst: u32, v: u32, _c: u32, _d: u32, s: f32) -> u32 {
        envelope::bridge::vmadd_scalar(&mut mem(), dst, v, s)
    }
    /// `sg_VmulScalar(dst, v, s)`.
    sg_VmulScalar @ 0x0084_5e80 => extern "C" fn sg_vmul_scalar(dst: u32, v: u32, s: f32) -> u32 {
        envelope::bridge::vmul_scalar(&mut mem(), dst, v, s)
    }
    /// `Sg_ControlCamera()`.
    Sg_ControlCamera @ 0x0083_f4b0 => extern "C" fn sg_control_camera() -> u32 {
        state::bridge::control_camera(&mut mem())
    }
    /// `SG_ApplyRawClip(obj, clip, t, t2)`.
    SG_ApplyRawClip @ 0x0083_8750 => extern "C" fn sg_apply_raw_clip(obj: u32, clip: u32, t: u32, t2: u32) {
        seq::bridge::apply_raw_clip(&mut mem(), obj, clip, t, t2);
    }
    /// `SG_AddMotionGroup(group, a, b, key)`.
    SG_AddMotionGroup @ 0x0083_6c90 => extern "C" fn sg_add_motion_group(group: u32, a: u32, b: u32, key: u32) {
        seq::bridge::add_motion_group(&mut mem(), group, a, b, key);
    }
    /// `Sg_IdGetHeight(p, i)`: the height in ST0.
    Sg_IdGetHeight @ 0x0083_e040 => extern "C" fn sg_id_get_height(p: u32, i: u32) -> f32 {
        state::bridge::id_get_height(&mut mem(), p, i)
    }
    /// `SG_ApplyBones(chr)`.
    SG_ApplyBones @ 0x0083_8f70 => extern "C" fn sg_apply_bones(chr: u32) {
        seq::bridge::apply_bones(&mut mem(), chr);
    }
    /// `SG_SeqStep(chr)`.
    SG_SeqStep @ 0x0083_7900 => extern "C" fn sg_seq_step(chr: u32) -> u32 {
        seq::bridge::seq_step(&mut mem(), chr)
    }
    /// `Sg_ControlMain()`.
    Sg_ControlMain @ 0x0082_d070 => extern "C" fn sg_control_main() -> u32 {
        state::bridge::control_main(&mut mem())
    }
    /// `SG_InitBones(chr)`.
    SG_InitBones @ 0x0082_77f0 => extern "C" fn sg_init_bones(chr: u32) {
        seq::bridge::init_bones(&mut mem(), chr);
    }
    /// `sg_calcenvelopeW_asm_opt(lists, out, m)`.
    sg_calcenvelopeW_asm_opt @ 0x006e_e270 => extern "C" fn sg_calcenvelope_w_asm_opt(lists: u32, out: u32, m: u32) {
        envelope::bridge::calc_envelope_w_opt(&mut mem(), lists, out, m);
    }
    /// `sg_calcenvelope_asm_opt(lists, out, m)`.
    sg_calcenvelope_asm_opt @ 0x006e_ee40 => extern "C" fn sg_calcenvelope_asm_opt(lists: u32, out: u32, m: u32) {
        envelope::bridge::calc_envelope_opt(&mut mem(), lists, out, m);
    }
    /// `sg_calcenvelopeW_asm(lists, out, mask, m)`.
    sg_calcenvelopeW_asm @ 0x006e_d9e0 => extern "C" fn sg_calcenvelope_w_asm(lists: u32, out: u32, mask: u32, m: u32) -> u32 {
        envelope::bridge::calc_envelope_w(&mut mem(), lists, out, mask, m)
    }
    /// `sg_calcenvelope_asm(lists, out, mask, m)`.
    sg_calcenvelope_asm @ 0x006e_e570 => extern "C" fn sg_calcenvelope_asm(lists: u32, out: u32, mask: u32, m: u32) -> u32 {
        envelope::bridge::calc_envelope(&mut mem(), lists, out, mask, m)
    }
    /// `sg_calcenvelope_normW(lists, out, m)`.
    sg_calcenvelope_normW @ 0x0084_5ec0 => extern "C" fn sg_calcenvelope_norm_w(lists: u32, out: u32, m: u32) {
        envelope::bridge::calc_envelope_norm_w(&mut mem(), lists, out, m);
    }
    /// `Cam_SetScreenCameraOn(screen, on)`.
    Cam_SetScreenCameraOn @ 0x007b_c930 => extern "C" fn cam_set_screen_camera_on(screen: u32, on: u32) {
        camera::bridge::set_screen_camera_on(&mut mem(), screen, ffx_sg::lo8(on));
    }
    /// `Cam_SetScreenCamera(screen, on, camera)`.
    Cam_SetScreenCamera @ 0x007b_c950 => extern "C" fn cam_set_screen_camera(screen: u32, on: u32, camera: u32) {
        camera::bridge::set_screen_camera(&mut mem(), screen, ffx_sg::lo8(on), ffx_sg::lo8(camera));
    }
    /// `passThisTriangle(a, b, c, group, tag)`.
    passThisTriangle @ 0x0082_9870 => extern "C" fn pass_this_triangle(a: u32, b: u32, c: u32, group: u32, tag: u32) -> u32 {
        use ffx_sg::lo16;
        ffx_sg::triangle::pass_this_triangle(lo16(a), lo16(b), lo16(c), lo16(group), lo16(tag))
    }
    /// `Sg_ChrSortByZ(n)`.
    Sg_ChrSortByZ @ 0x0083_eb30 => extern "C" fn sg_chr_sort_by_z(n: i32) {
        state::bridge::chr_sort_by_z(&mut mem(), n);
    }
    /// `SG_SeqFrame(chr)`.
    SG_SeqFrame @ 0x0083_77c0 => extern "C" fn sg_seq_frame(chr: u32) -> u32 {
        seq::bridge::seq_frame(&mut mem(), chr)
    }
    /// `Sg_IdFindTriangle(p)`.
    Sg_IdFindTriangle @ 0x0083_de10 => extern "C" fn sg_id_find_triangle(p: u32) -> i32 {
        state::bridge::id_find_triangle(&mut mem(), p)
    }
    /// `Sg_ControlPlayer()`.
    Sg_ControlPlayer @ 0x0082_d180 => extern "C" fn sg_control_player() {
        player::bridge::control_player(&mut mem(), &crt_vector_trig, &crt_atan2);
    }
}

/// `Phyre_PTimerWin32_GetTime()` (`0x0042fd80`): the game's clock in
/// seconds (the harness's virtual clock under replay), for
/// `FFXApplication::getAppTime`.
fn app_clock() -> f64 {
    // SAFETY: a cdecl function of this build taking nothing and returning a
    // double in ST0.
    unsafe {
        let f: unsafe extern "C" fn() -> f64 = std::mem::transmute(crate::rebase(0x0042_fd80));
        f()
    }
}

/// MSVCR110's `_CIsin`, `_CIcos` or `_CIacos` through the game's import
/// thunks (`0x009498c8`, `0x009498c2`, `0x009498da`): the argument in
/// `ST0` at full precision (`fld qword`), the result stored as `fstp
/// dword` stores it, as the originals do right after the call.
fn crt(c: Crt, x: f64) -> f32 {
    let thunk = match c {
        Crt::Sin => 0x0094_98c8,
        Crt::Cos => 0x0094_98c2,
        Crt::Acos => 0x0094_98da,
    };
    // SAFETY: the thunk jumps to the CRT's intrinsic, which takes its
    // argument in ST0 and returns in ST0 (eax, ecx, edx, the x87 and SSE
    // registers clobbered).
    #[cfg(target_arch = "x86")]
    unsafe {
        let f = crate::rebase(thunk);
        let bits: u32;
        std::arch::asm!(
            "fld qword ptr [{x}]",
            "call {f}",
            "sub esp, 4",
            "fstp dword ptr [esp]",
            "pop eax",
            x = in(reg) &raw const x,
            f = in(reg) f,
            lateout("eax") bits,
            clobber_abi("C"),
        );
        f32::from_bits(bits)
    }
    #[cfg(not(target_arch = "x86"))]
    {
        let _ = (thunk, x);
        unreachable!("the CRT's trigonometry runs in the 32-bit game")
    }
}

/// MSVCR110's `_CIcos`, `_CIsin` or `_CIatan` through the game's import
/// thunks (`0x009498c2`, `0x009498c8`, `0x00949d5c`), as the battle
/// vector code calls them: the argument (`f32` bits) loaded with `fld
/// dword`, the result stored with `fstp dword`.
fn crt_vector_trig(t: ffx_battle::pos::bridge::Trig, x: u32) -> u32 {
    use ffx_battle::pos::bridge::Trig;
    let thunk = match t {
        Trig::Cos => 0x0094_98c2,
        Trig::Sin => 0x0094_98c8,
        Trig::Atan => 0x0094_9d5c,
    };
    // SAFETY: the thunk jumps to the CRT's intrinsic, which takes its
    // argument in ST0 and returns in ST0 (eax, ecx, edx, the x87 and SSE
    // registers clobbered).
    #[cfg(target_arch = "x86")]
    unsafe {
        let f = crate::rebase(thunk);
        let bits: u32;
        std::arch::asm!(
            "fld dword ptr [{x}]",
            "call {f}",
            "sub esp, 4",
            "fstp dword ptr [esp]",
            "pop eax",
            x = in(reg) &raw const x,
            f = in(reg) f,
            lateout("eax") bits,
            clobber_abi("C"),
        );
        bits
    }
    #[cfg(not(target_arch = "x86"))]
    {
        let _ = (thunk, x);
        0
    }
}

/// MSVCR110's `_CIatan2` (`ST1` = y, `ST0` = x, the result in `ST0`)
/// through the game's import thunk (`0x009498bc`): both loaded with `fld
/// dword`, the result stored with `fstp dword`, as `Sg_ControlPlayer`
/// does; `f32` bits in and out.
fn crt_atan2(y: u32, x: u32) -> u32 {
    let f = crate::rebase(0x0094_98bc);
    #[cfg(target_arch = "x86")]
    {
        let pair = [y, x];
        let mut out = 0u32;
        // SAFETY: the thunk jumps to the CRT's `_CIatan2`, which takes its
        // two operands on the x87 stack and leaves the result there; the
        // asm loads them, calls it and stores the result through edi
        // (which the callee keeps), leaving the x87 stack empty.
        unsafe {
            std::arch::asm!(
                "fld dword ptr [{p}]",
                "fld dword ptr [{p} + 4]",
                "call {f}",
                "fstp dword ptr [edi]",
                p = in(reg) &raw const pair,
                in("edi") &raw mut out,
                f = in(reg) f,
                clobber_abi("C"),
            );
        }
        out
    }
    #[cfg(not(target_arch = "x86"))]
    {
        let _ = (f, y, x);
        0
    }
}
