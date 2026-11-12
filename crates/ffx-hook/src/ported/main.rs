//! `ffx-sg`: the main module (`Sg`): switches and accessors of the main
//! loop, point lights, the GS packet writers, small vector helpers,
//! `crossAssertIsValidFloat`.

use ffx_sg::quat::Crt;
use ffx_sg::{
    abmap, cache, camera, cdread, chpacket, cloth, control, decode, envelope, gs, help, light,
    player, quat, savecmd, seq, state, vec,
};

use super::{mem, ported};

/// Calls the original cdecl function at `sym` (an `ffx.sym` address).
fn call_sym<M>(_mem: &mut M, sym: u32, args: &[u32]) -> u32 {
    #[allow(clippy::cast_possible_truncation)] // 32-bit process
    super::call_cdecl(crate::rebase(sym) as u32, args)
}

ported! {
    /// `spline_get_near_point(spline, x, y, z, &ox, &oy, &oz, cubic)`: the
    /// spline through menu's ports (`TkMtCalcSpline`/`3`).
    spline_get_near_point @ 0x00a4_4120 => extern "C" fn spline_get_near_point(spline: u32, x: f32, y: f32, z: f32, ox: u32, oy: u32, oz: u32, cubic: u32) -> f32 {
        // the original's three points
        let mut frame = super::StackFrame::<12>::new();
        let locals = frame.at();
        let args = [spline, x.to_bits(), y.to_bits(), z.to_bits(), ox, oy, oz, cubic];
        ffx_sg::spline::bridge::spline_get_near_point(&mut mem(), args, locals, &mut |m, cubic, w| {
            use ffx_menu::tk::bridge as tk;
            let call = &mut |_: &mut _, f: u32, a: &[u32]| super::call_cdecl(f, a);
            let [out, points, n, t] = w;
            let (n, t) = (n.cast_signed(), f32::from_bits(t));
            if cubic {
                tk::tk_mt_calc_spline3(m, [out, points], n, t, call);
            } else {
                tk::tk_mt_calc_spline(m, [out, points], n, t, call);
            }
        })
    }
    /// `crossAssertIsValidFloat(x)`.
    crossAssertIsValidFloat @ 0x006e_f2d0 => extern "C" fn cross_assert_is_valid_float(x: f32) -> u32 {
        ffx_sg::assert::is_valid_float(x)
    }
    /// `get_read_buffer()`.
    get_read_buffer @ 0x0076_dc50 => extern "C" fn get_read_buffer() -> u32 {
        state::bridge::get_read_buffer(&mem())
    }
    /// `CDfileReadSector_CD(sector, buffer, size, callback, opts)`.
    CDfileReadSector_CD @ 0x0076_bdd0 => extern "C" fn cd_file_read_sector(sector: u32, buffer: u32, size: u32, callback: u32, opts: u32) -> i32 {
        cdread::bridge::cd_file_read_sector(&mut mem(), sector, buffer, size, callback, opts)
    }
    /// `cd_read_q_clear(id)`.
    cd_read_q_clear @ 0x0076_e130 => extern "C" fn cd_read_q_clear(id: i32) {
        cdread::bridge::cd_read_q_clear(&mut mem(), id);
    }
    /// `sa_menu_start(kind)` (`0x00770430`).
    sa_menu_start @ 0x0077_0430 => extern "C" fn sa_menu_start(kind: u32) -> u32 {
        state::bridge::sa_menu_start(&mut mem(), kind)
    }
    /// `menu_list_step()` (`0x0076f470`): the jump word and the step's
    /// answer in this frame.
    menu_list_step @ 0x0076_f470 => extern "C" fn menu_list_step() -> u32 {
        let mut w = super::StackFrame::<{ state::bridge::MENU_STEP_WORK as usize / 4 }>::new();
        state::bridge::menu_list_step(&mut mem(), w.at())
    }
    /// `menu_list_draw()` (`0x0076f850`): the words its callees write
    /// through pointers (and the texture name) in this frame.
    menu_list_draw @ 0x0076_f850 => extern "C" fn menu_list_draw() {
        let mut w = super::StackFrame::<{ help::bridge::DRAW_WORK as usize / 4 }>::new();
        help::bridge::menu_list_draw(&mut mem(), w.at(), &mut |_, f, a| super::call_cdecl(f, a));
    }
    /// `cd_read_thread_sector(cd, request, n)` (`0x0076e4b0`).
    cd_read_thread_sector @ 0x0076_e4b0 => extern "C" fn cd_read_thread_sector(cd: u32, request: u32, n: u32) {
        cdread::bridge::cd_read_thread_sector(&mut mem(), cd, request, n);
    }
    /// `cd_read_thread_file(cd, request, n)` (`0x0076e1c0`).
    cd_read_thread_file @ 0x0076_e1c0 => extern "C" fn cd_read_thread_file(cd: u32, request: u32, n: u32) {
        cdread::bridge::cd_read_thread_file(&mut mem(), cd, request, n);
    }
    /// The CD-mode read thread `0x0076e780` (never returns).
    cd_read_thread @ 0x0076_e780 => extern "C" fn cd_read_thread() {
        cdread::bridge::cd_read_thread(&mut mem());
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
    /// `Sg_MainInit(argc, argv)`: the tail call `0x00a44a30`'s `eax`.
    Sg_MainInit @ 0x0082_0860 => extern "C" fn sg_main_init(argc: i32, argv: u32) -> u32 {
        state::bridge::main_init(&mut mem(), argc, argv)
    }
    /// `Sg_MainCheckMenu()`: 1 when the menu opened this frame.
    Sg_MainCheckMenu @ 0x0082_0750 => extern "C" fn sg_main_check_menu() -> u32 {
        state::bridge::main_check_menu(&mut mem())
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
    // Round 15: the MsGetSaveCommand accessors of the table at 0x00c410c8.
    /// `KernelSaveCommand_3056_0076f180(chr)`: `MsGetSaveCommand(chr, 0x3056)`.
    KernelSaveCommand_3056_0076f180 @ 0x0076_f180 => extern "C" fn kernel_save_command_0076f180(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0076_f180, chr, call_sym)
    }
    /// `KernelSaveCommand_3058_0076f1d0(chr)`: `MsGetSaveCommand(chr, 0x3058)`.
    KernelSaveCommand_3058_0076f1d0 @ 0x0076_f1d0 => extern "C" fn kernel_save_command_0076f1d0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0076_f1d0, chr, call_sym)
    }
    /// `KernelSaveCommand_3059_0076f220(chr)`: `MsGetSaveCommand(chr, 0x3059)`.
    KernelSaveCommand_3059_0076f220 @ 0x0076_f220 => extern "C" fn kernel_save_command_0076f220(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0076_f220, chr, call_sym)
    }
    /// `KernelSaveCommand_305a_0076f240(chr)`: `MsGetSaveCommand(chr, 0x305a)`.
    KernelSaveCommand_305a_0076f240 @ 0x0076_f240 => extern "C" fn kernel_save_command_0076f240(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0076_f240, chr, call_sym)
    }
    /// `KernelSaveCommand_305b_0076f290(chr)`: `MsGetSaveCommand(chr, 0x305b)`.
    KernelSaveCommand_305b_0076f290 @ 0x0076_f290 => extern "C" fn kernel_save_command_0076f290(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0076_f290, chr, call_sym)
    }
    /// `KernelSaveCommand_305c_0076f2e0(chr)`: `MsGetSaveCommand(chr, 0x305c)`.
    KernelSaveCommand_305c_0076f2e0 @ 0x0076_f2e0 => extern "C" fn kernel_save_command_0076f2e0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0076_f2e0, chr, call_sym)
    }
    /// `KernelSaveCommand_305d_0076f330(chr)`: `MsGetSaveCommand(chr, 0x305d)`.
    KernelSaveCommand_305d_0076f330 @ 0x0076_f330 => extern "C" fn kernel_save_command_0076f330(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0076_f330, chr, call_sym)
    }
    /// `KernelSaveCommand_305e_0076f370(chr)`: `MsGetSaveCommand(chr, 0x305e)`.
    KernelSaveCommand_305e_0076f370 @ 0x0076_f370 => extern "C" fn kernel_save_command_0076f370(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0076_f370, chr, call_sym)
    }
    /// `KernelSaveCommand_305f_0076f450(chr)`: `MsGetSaveCommand(chr, 0x305f)`.
    KernelSaveCommand_305f_0076f450 @ 0x0076_f450 => extern "C" fn kernel_save_command_0076f450(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0076_f450, chr, call_sym)
    }
    /// `KernelSaveCommand_3060_00770320(chr)`: `MsGetSaveCommand(chr, 0x3060)`.
    KernelSaveCommand_3060_00770320 @ 0x0077_0320 => extern "C" fn kernel_save_command_00770320(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0320, chr, call_sym)
    }
    /// `KernelSaveCommand_3061_00770950(chr)`: `MsGetSaveCommand(chr, 0x3061)`.
    KernelSaveCommand_3061_00770950 @ 0x0077_0950 => extern "C" fn kernel_save_command_00770950(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0950, chr, call_sym)
    }
    /// `KernelSaveCommand_3062_00770970(chr)`: `MsGetSaveCommand(chr, 0x3062)`.
    KernelSaveCommand_3062_00770970 @ 0x0077_0970 => extern "C" fn kernel_save_command_00770970(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0970, chr, call_sym)
    }
    /// `KernelSaveCommand_3063_007709b0(chr)`: `MsGetSaveCommand(chr, 0x3063)`.
    KernelSaveCommand_3063_007709b0 @ 0x0077_09b0 => extern "C" fn kernel_save_command_007709b0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_09b0, chr, call_sym)
    }
    /// `KernelSaveCommand_3064_007709f0(chr)`: `MsGetSaveCommand(chr, 0x3064)`.
    KernelSaveCommand_3064_007709f0 @ 0x0077_09f0 => extern "C" fn kernel_save_command_007709f0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_09f0, chr, call_sym)
    }
    /// `KernelSaveCommand_3065_00770a40(chr)`: `MsGetSaveCommand(chr, 0x3065)`.
    KernelSaveCommand_3065_00770a40 @ 0x0077_0a40 => extern "C" fn kernel_save_command_00770a40(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0a40, chr, call_sym)
    }
    /// `KernelSaveCommand_3066_00770a90(chr)`: `MsGetSaveCommand(chr, 0x3066)`.
    KernelSaveCommand_3066_00770a90 @ 0x0077_0a90 => extern "C" fn kernel_save_command_00770a90(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0a90, chr, call_sym)
    }
    /// `KernelSaveCommand_3067_00770ae0(chr)`: `MsGetSaveCommand(chr, 0x3067)`.
    KernelSaveCommand_3067_00770ae0 @ 0x0077_0ae0 => extern "C" fn kernel_save_command_00770ae0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0ae0, chr, call_sym)
    }
    /// `KernelSaveCommand_3068_00770b30(chr)`: `MsGetSaveCommand(chr, 0x3068)`.
    KernelSaveCommand_3068_00770b30 @ 0x0077_0b30 => extern "C" fn kernel_save_command_00770b30(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0b30, chr, call_sym)
    }
    /// `KernelSaveCommand_3069_00770b80(chr)`: `MsGetSaveCommand(chr, 0x3069)`.
    KernelSaveCommand_3069_00770b80 @ 0x0077_0b80 => extern "C" fn kernel_save_command_00770b80(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0b80, chr, call_sym)
    }
    /// `KernelSaveCommand_306a_00770ba0(chr)`: `MsGetSaveCommand(chr, 0x306a)`.
    KernelSaveCommand_306a_00770ba0 @ 0x0077_0ba0 => extern "C" fn kernel_save_command_00770ba0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0ba0, chr, call_sym)
    }
    /// `KernelSaveCommand_306b_00770bf0(chr)`: `MsGetSaveCommand(chr, 0x306b)`.
    KernelSaveCommand_306b_00770bf0 @ 0x0077_0bf0 => extern "C" fn kernel_save_command_00770bf0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0bf0, chr, call_sym)
    }
    /// `KernelSaveCommand_306c_00770c40(chr)`: `MsGetSaveCommand(chr, 0x306c)`.
    KernelSaveCommand_306c_00770c40 @ 0x0077_0c40 => extern "C" fn kernel_save_command_00770c40(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0c40, chr, call_sym)
    }
    /// `KernelSaveCommand_306d_00770c90(chr)`: `MsGetSaveCommand(chr, 0x306d)`.
    KernelSaveCommand_306d_00770c90 @ 0x0077_0c90 => extern "C" fn kernel_save_command_00770c90(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0c90, chr, call_sym)
    }
    /// `KernelSaveCommand_306e_00770ce0(chr)`: `MsGetSaveCommand(chr, 0x306e)`.
    KernelSaveCommand_306e_00770ce0 @ 0x0077_0ce0 => extern "C" fn kernel_save_command_00770ce0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0ce0, chr, call_sym)
    }
    /// `KernelSaveCommand_306f_00770d30(chr)`: `MsGetSaveCommand(chr, 0x306f)`.
    KernelSaveCommand_306f_00770d30 @ 0x0077_0d30 => extern "C" fn kernel_save_command_00770d30(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0d30, chr, call_sym)
    }
    /// `KernelSaveCommand_3070_00770d50(chr)`: `MsGetSaveCommand(chr, 0x3070)`.
    KernelSaveCommand_3070_00770d50 @ 0x0077_0d50 => extern "C" fn kernel_save_command_00770d50(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0d50, chr, call_sym)
    }
    /// `KernelSaveCommand_3071_00770da0(chr)`: `MsGetSaveCommand(chr, 0x3071)`.
    KernelSaveCommand_3071_00770da0 @ 0x0077_0da0 => extern "C" fn kernel_save_command_00770da0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0da0, chr, call_sym)
    }
    /// `KernelSaveCommand_3072_00770df0(chr)`: `MsGetSaveCommand(chr, 0x3072)`.
    KernelSaveCommand_3072_00770df0 @ 0x0077_0df0 => extern "C" fn kernel_save_command_00770df0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0df0, chr, call_sym)
    }
    /// `KernelSaveCommand_3073_00770e40(chr)`: `MsGetSaveCommand(chr, 0x3073)`.
    KernelSaveCommand_3073_00770e40 @ 0x0077_0e40 => extern "C" fn kernel_save_command_00770e40(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0e40, chr, call_sym)
    }
    /// `KernelSaveCommand_3074_00770e90(chr)`: `MsGetSaveCommand(chr, 0x3074)`.
    KernelSaveCommand_3074_00770e90 @ 0x0077_0e90 => extern "C" fn kernel_save_command_00770e90(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0e90, chr, call_sym)
    }
    /// `KernelSaveCommand_3075_00770ee0(chr)`: `MsGetSaveCommand(chr, 0x3075)`.
    KernelSaveCommand_3075_00770ee0 @ 0x0077_0ee0 => extern "C" fn kernel_save_command_00770ee0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0ee0, chr, call_sym)
    }
    /// `AbmapDrawNodeHalos()`.
    AbmapDrawNodeHalos @ 0x00a5_0290 => extern "C" fn abmap_draw_node_halos() {
        // laid out and aligned as the original's frame
        let mut work = super::AlignedFrame::<{ abmap::bridge::NODE_WORK as usize }>::new();
        let w = work.at();
        abmap::bridge::abmap_draw_node_halos(&mut mem(), w, &mut |_, f, a| super::call_cdecl(f, a));
    }
    /// `AbmapDrawNodeSpheres()`.
    AbmapDrawNodeSpheres @ 0x00a5_05e0 => extern "C" fn abmap_draw_node_spheres() {
        // laid out and aligned as the original's frame
        let mut work = super::AlignedFrame::<{ abmap::bridge::NODE_WORK as usize }>::new();
        let w = work.at();
        abmap::bridge::abmap_draw_node_spheres(&mut mem(), w, &mut |_, f, a| super::call_cdecl(f, a));
    }
    /// `AbmapDrawLinkStrip(p, points, n)`.
    AbmapDrawLinkStrip @ 0x00a5_21a0 => extern "C" fn abmap_draw_link_strip(p: u32, points: u32, n: u32) {
        let mut w = super::StackFrame::<{ abmap::bridge::STRIP_WORK as usize / 4 }>::new();
        abmap::bridge::abmap_draw_link_strip(&mut mem(), p, points, n, w.at(), &mut |_, f, a| super::call_cdecl(f, a));
    }
    // Round 15: more kernel menu accessors (MsGetSaveCommand, slot names).
    /// `KernelSaveCommand_3076_00770f00(chr)`: `MsGetSaveCommand(chr, 0x3076)`.
    KernelSaveCommand_3076_00770f00 @ 0x0077_0f00 => extern "C" fn kernel_save_command_00770f00(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0f00, chr, call_sym)
    }
    /// `KernelSaveCommand_3077_00770f50(chr)`: `MsGetSaveCommand(chr, 0x3077)`.
    KernelSaveCommand_3077_00770f50 @ 0x0077_0f50 => extern "C" fn kernel_save_command_00770f50(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0f50, chr, call_sym)
    }
    /// `KernelSaveCommand_308b_00770fa0(chr)`: `MsGetSaveCommand(chr, 0x308b)`.
    KernelSaveCommand_308b_00770fa0 @ 0x0077_0fa0 => extern "C" fn kernel_save_command_00770fa0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0fa0, chr, call_sym)
    }
    /// `KernelSaveCommand_308c_00770ff0(chr)`: `MsGetSaveCommand(chr, 0x308c)`.
    KernelSaveCommand_308c_00770ff0 @ 0x0077_0ff0 => extern "C" fn kernel_save_command_00770ff0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_0ff0, chr, call_sym)
    }
    /// `KernelSaveCommand_308d_00771040(chr)`: `MsGetSaveCommand(chr, 0x308d)`.
    KernelSaveCommand_308d_00771040 @ 0x0077_1040 => extern "C" fn kernel_save_command_00771040(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1040, chr, call_sym)
    }
    /// `KernelSaveCommand_308e_00771090(chr)`: `MsGetSaveCommand(chr, 0x308e)`.
    KernelSaveCommand_308e_00771090 @ 0x0077_1090 => extern "C" fn kernel_save_command_00771090(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1090, chr, call_sym)
    }
    /// `KernelSaveCommand_308f_007710b0(chr)`: `MsGetSaveCommand(chr, 0x308f)`.
    KernelSaveCommand_308f_007710b0 @ 0x0077_10b0 => extern "C" fn kernel_save_command_007710b0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_10b0, chr, call_sym)
    }
    /// `KernelSaveCommand_3090_00771100(chr)`: `MsGetSaveCommand(chr, 0x3090)`.
    KernelSaveCommand_3090_00771100 @ 0x0077_1100 => extern "C" fn kernel_save_command_00771100(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1100, chr, call_sym)
    }
    /// `KernelSaveCommand_3091_00771150(chr)`: `MsGetSaveCommand(chr, 0x3091)`.
    KernelSaveCommand_3091_00771150 @ 0x0077_1150 => extern "C" fn kernel_save_command_00771150(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1150, chr, call_sym)
    }
    /// `KernelSaveCommand_3092_007711a0(chr)`: `MsGetSaveCommand(chr, 0x3092)`.
    KernelSaveCommand_3092_007711a0 @ 0x0077_11a0 => extern "C" fn kernel_save_command_007711a0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_11a0, chr, call_sym)
    }
    /// `KernelSaveCommand_3093_007711f0(chr)`: `MsGetSaveCommand(chr, 0x3093)`.
    KernelSaveCommand_3093_007711f0 @ 0x0077_11f0 => extern "C" fn kernel_save_command_007711f0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_11f0, chr, call_sym)
    }
    /// `KernelSaveCommand_3094_00771240(chr)`: `MsGetSaveCommand(chr, 0x3094)`.
    KernelSaveCommand_3094_00771240 @ 0x0077_1240 => extern "C" fn kernel_save_command_00771240(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1240, chr, call_sym)
    }
    /// `KernelSaveCommand_3095_00771260(chr)`: `MsGetSaveCommand(chr, 0x3095)`.
    KernelSaveCommand_3095_00771260 @ 0x0077_1260 => extern "C" fn kernel_save_command_00771260(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1260, chr, call_sym)
    }
    /// `KernelSaveCommand_3096_007712b0(chr)`: `MsGetSaveCommand(chr, 0x3096)`.
    KernelSaveCommand_3096_007712b0 @ 0x0077_12b0 => extern "C" fn kernel_save_command_007712b0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_12b0, chr, call_sym)
    }
    /// `KernelSaveCommand_3097_00771300(chr)`: `MsGetSaveCommand(chr, 0x3097)`.
    KernelSaveCommand_3097_00771300 @ 0x0077_1300 => extern "C" fn kernel_save_command_00771300(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1300, chr, call_sym)
    }
    /// `KernelSaveCommand_3098_00771350(chr)`: `MsGetSaveCommand(chr, 0x3098)`.
    KernelSaveCommand_3098_00771350 @ 0x0077_1350 => extern "C" fn kernel_save_command_00771350(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1350, chr, call_sym)
    }
    /// `KernelSaveCommand_3099_007713a0(chr)`: `MsGetSaveCommand(chr, 0x3099)`.
    KernelSaveCommand_3099_007713a0 @ 0x0077_13a0 => extern "C" fn kernel_save_command_007713a0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_13a0, chr, call_sym)
    }
    /// `KernelSaveCommand_309a_007713f0(chr)`: `MsGetSaveCommand(chr, 0x309a)`.
    KernelSaveCommand_309a_007713f0 @ 0x0077_13f0 => extern "C" fn kernel_save_command_007713f0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_13f0, chr, call_sym)
    }
    /// `KernelSaveCommand_309b_00771410(chr)`: `MsGetSaveCommand(chr, 0x309b)`.
    KernelSaveCommand_309b_00771410 @ 0x0077_1410 => extern "C" fn kernel_save_command_00771410(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1410, chr, call_sym)
    }
    /// `KernelSaveCommand_309c_00771460(chr)`: `MsGetSaveCommand(chr, 0x309c)`.
    KernelSaveCommand_309c_00771460 @ 0x0077_1460 => extern "C" fn kernel_save_command_00771460(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1460, chr, call_sym)
    }
    /// `KernelSaveCommand_309d_007714b0(chr)`: `MsGetSaveCommand(chr, 0x309d)`.
    KernelSaveCommand_309d_007714b0 @ 0x0077_14b0 => extern "C" fn kernel_save_command_007714b0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_14b0, chr, call_sym)
    }
    /// `KernelSaveCommand_309e_00771500(chr)`: `MsGetSaveCommand(chr, 0x309e)`.
    KernelSaveCommand_309e_00771500 @ 0x0077_1500 => extern "C" fn kernel_save_command_00771500(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1500, chr, call_sym)
    }
    /// `KernelSaveCommand_309f_00771550(chr)`: `MsGetSaveCommand(chr, 0x309f)`.
    KernelSaveCommand_309f_00771550 @ 0x0077_1550 => extern "C" fn kernel_save_command_00771550(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1550, chr, call_sym)
    }
    /// `KernelSaveCommand_30a0_007715a0(chr)`: `MsGetSaveCommand(chr, 0x30a0)`.
    KernelSaveCommand_30a0_007715a0 @ 0x0077_15a0 => extern "C" fn kernel_save_command_007715a0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_15a0, chr, call_sym)
    }
    /// `KernelSaveCommand_30a1_007715c0(chr)`: `MsGetSaveCommand(chr, 0x30a1)`.
    KernelSaveCommand_30a1_007715c0 @ 0x0077_15c0 => extern "C" fn kernel_save_command_007715c0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_15c0, chr, call_sym)
    }
    /// `KernelSaveCommand_30a2_00771610(chr)`: `MsGetSaveCommand(chr, 0x30a2)`.
    KernelSaveCommand_30a2_00771610 @ 0x0077_1610 => extern "C" fn kernel_save_command_00771610(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1610, chr, call_sym)
    }
    /// `KernelSaveCommand_30a3_00771660(chr)`: `MsGetSaveCommand(chr, 0x30a3)`.
    KernelSaveCommand_30a3_00771660 @ 0x0077_1660 => extern "C" fn kernel_save_command_00771660(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1660, chr, call_sym)
    }
    /// `KernelSaveCommand_30a4_007716b0(chr)`: `MsGetSaveCommand(chr, 0x30a4)`.
    KernelSaveCommand_30a4_007716b0 @ 0x0077_16b0 => extern "C" fn kernel_save_command_007716b0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_16b0, chr, call_sym)
    }
    /// `KernelSaveCommand_30a5_00771700(chr)`: `MsGetSaveCommand(chr, 0x30a5)`.
    KernelSaveCommand_30a5_00771700 @ 0x0077_1700 => extern "C" fn kernel_save_command_00771700(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1700, chr, call_sym)
    }
    /// `KernelSaveCommand_30a6_00771750(chr)`: `MsGetSaveCommand(chr, 0x30a6)`.
    KernelSaveCommand_30a6_00771750 @ 0x0077_1750 => extern "C" fn kernel_save_command_00771750(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1750, chr, call_sym)
    }
    /// `KernelSaveCommand_30a7_00771770(chr)`: `MsGetSaveCommand(chr, 0x30a7)`.
    KernelSaveCommand_30a7_00771770 @ 0x0077_1770 => extern "C" fn kernel_save_command_00771770(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1770, chr, call_sym)
    }
    /// `KernelSaveCommand_30a8_007717c0(chr)`: `MsGetSaveCommand(chr, 0x30a8)`.
    KernelSaveCommand_30a8_007717c0 @ 0x0077_17c0 => extern "C" fn kernel_save_command_007717c0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_17c0, chr, call_sym)
    }
    /// `KernelSaveCommand_30a9_00771810(chr)`: `MsGetSaveCommand(chr, 0x30a9)`.
    KernelSaveCommand_30a9_00771810 @ 0x0077_1810 => extern "C" fn kernel_save_command_00771810(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1810, chr, call_sym)
    }
    /// `KernelSaveCommand_30aa_00771860(chr)`: `MsGetSaveCommand(chr, 0x30aa)`.
    KernelSaveCommand_30aa_00771860 @ 0x0077_1860 => extern "C" fn kernel_save_command_00771860(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1860, chr, call_sym)
    }
    /// `KernelSaveCommand_30ab_007718b0(chr)`: `MsGetSaveCommand(chr, 0x30ab)`.
    KernelSaveCommand_30ab_007718b0 @ 0x0077_18b0 => extern "C" fn kernel_save_command_007718b0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_18b0, chr, call_sym)
    }
    /// `KernelSaveCommand_30ac_00771900(chr)`: `MsGetSaveCommand(chr, 0x30ac)`.
    KernelSaveCommand_30ac_00771900 @ 0x0077_1900 => extern "C" fn kernel_save_command_00771900(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1900, chr, call_sym)
    }
    /// `KernelSaveCommand_30ad_00771920(chr)`: `MsGetSaveCommand(chr, 0x30ad)`.
    KernelSaveCommand_30ad_00771920 @ 0x0077_1920 => extern "C" fn kernel_save_command_00771920(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1920, chr, call_sym)
    }
    /// `KernelSaveCommand_30ae_00771970(chr)`: `MsGetSaveCommand(chr, 0x30ae)`.
    KernelSaveCommand_30ae_00771970 @ 0x0077_1970 => extern "C" fn kernel_save_command_00771970(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1970, chr, call_sym)
    }
    /// `KernelSaveCommand_30af_007719c0(chr)`: `MsGetSaveCommand(chr, 0x30af)`.
    KernelSaveCommand_30af_007719c0 @ 0x0077_19c0 => extern "C" fn kernel_save_command_007719c0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_19c0, chr, call_sym)
    }
    /// `KernelSaveCommand_30b0_00771a10(chr)`: `MsGetSaveCommand(chr, 0x30b0)`.
    KernelSaveCommand_30b0_00771a10 @ 0x0077_1a10 => extern "C" fn kernel_save_command_00771a10(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1a10, chr, call_sym)
    }
    /// `KernelSaveCommand_30b1_00771a60(chr)`: `MsGetSaveCommand(chr, 0x30b1)`.
    KernelSaveCommand_30b1_00771a60 @ 0x0077_1a60 => extern "C" fn kernel_save_command_00771a60(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1a60, chr, call_sym)
    }
    /// `KernelSaveCommand_30b2_00771ab0(chr)`: `MsGetSaveCommand(chr, 0x30b2)`.
    KernelSaveCommand_30b2_00771ab0 @ 0x0077_1ab0 => extern "C" fn kernel_save_command_00771ab0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1ab0, chr, call_sym)
    }
    /// `KernelSaveCommand_30b3_00771ad0(chr)`: `MsGetSaveCommand(chr, 0x30b3)`.
    KernelSaveCommand_30b3_00771ad0 @ 0x0077_1ad0 => extern "C" fn kernel_save_command_00771ad0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1ad0, chr, call_sym)
    }
    /// `KernelSaveCommand_30b4_00771b20(chr)`: `MsGetSaveCommand(chr, 0x30b4)`.
    KernelSaveCommand_30b4_00771b20 @ 0x0077_1b20 => extern "C" fn kernel_save_command_00771b20(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1b20, chr, call_sym)
    }
    /// `KernelSaveCommand_30b5_00771b70(chr)`: `MsGetSaveCommand(chr, 0x30b5)`.
    KernelSaveCommand_30b5_00771b70 @ 0x0077_1b70 => extern "C" fn kernel_save_command_00771b70(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1b70, chr, call_sym)
    }
    /// `KernelSaveCommand_30b6_00771bc0(chr)`: `MsGetSaveCommand(chr, 0x30b6)`.
    KernelSaveCommand_30b6_00771bc0 @ 0x0077_1bc0 => extern "C" fn kernel_save_command_00771bc0(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1bc0, chr, call_sym)
    }
    /// `KernelSaveCommand_30b7_00771c10(chr)`: `MsGetSaveCommand(chr, 0x30b7)`.
    KernelSaveCommand_30b7_00771c10 @ 0x0077_1c10 => extern "C" fn kernel_save_command_00771c10(chr: u32) -> u32 {
        savecmd::bridge::kernel_save_command(&mut mem(), 0x0077_1c10, chr, call_sym)
    }
    /// `KernelSlotItemName_9a_0076f1a0(i)`: `Kernel_GetItemName` of record `i`'s `u16` at 0x9a.
    KernelSlotItemName_9a_0076f1a0 @ 0x0076_f1a0 => extern "C" fn kernel_slot_name_0076f1a0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0076_f1a0, i, call_sym)
    }
    /// `KernelSlotItemName_9c_0076f1f0(i)`: `Kernel_GetItemName` of record `i`'s `u16` at 0x9c.
    KernelSlotItemName_9c_0076f1f0 @ 0x0076_f1f0 => extern "C" fn kernel_slot_name_0076f1f0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0076_f1f0, i, call_sym)
    }
    /// `KernelSlotItemName_9e_0076f260(i)`: `Kernel_GetItemName` of record `i`'s `u16` at 0x9e.
    KernelSlotItemName_9e_0076f260 @ 0x0076_f260 => extern "C" fn kernel_slot_name_0076f260(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0076_f260, i, call_sym)
    }
    /// `KernelSlotItemName_a4_0076f2b0(i)`: `Kernel_GetItemName` of record `i`'s `u16` at 0xa4.
    KernelSlotItemName_a4_0076f2b0 @ 0x0076_f2b0 => extern "C" fn kernel_slot_name_0076f2b0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0076_f2b0, i, call_sym)
    }
    /// `KernelSlotItemName_a6_0076f300(i)`: `Kernel_GetItemName` of record `i`'s `u16` at 0xa6.
    KernelSlotItemName_a6_0076f300 @ 0x0076_f300 => extern "C" fn kernel_slot_name_0076f300(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0076_f300, i, call_sym)
    }
    /// `KernelSlotAbilityName_aa_00770920(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xaa.
    KernelSlotAbilityName_aa_00770920 @ 0x0077_0920 => extern "C" fn kernel_slot_name_00770920(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0920, i, call_sym)
    }
    /// `KernelSlotAbilityName_b2_00770a10(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xb2.
    KernelSlotAbilityName_b2_00770a10 @ 0x0077_0a10 => extern "C" fn kernel_slot_name_00770a10(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0a10, i, call_sym)
    }
    /// `KernelSlotAbilityName_b4_00770a60(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xb4.
    KernelSlotAbilityName_b4_00770a60 @ 0x0077_0a60 => extern "C" fn kernel_slot_name_00770a60(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0a60, i, call_sym)
    }
    /// `KernelSlotAbilityName_b6_00770ab0(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xb6.
    KernelSlotAbilityName_b6_00770ab0 @ 0x0077_0ab0 => extern "C" fn kernel_slot_name_00770ab0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0ab0, i, call_sym)
    }
    /// `KernelSlotAbilityName_b8_00770b00(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xb8.
    KernelSlotAbilityName_b8_00770b00 @ 0x0077_0b00 => extern "C" fn kernel_slot_name_00770b00(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0b00, i, call_sym)
    }
    /// `KernelSlotAbilityName_ba_00770b50(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xba.
    KernelSlotAbilityName_ba_00770b50 @ 0x0077_0b50 => extern "C" fn kernel_slot_name_00770b50(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0b50, i, call_sym)
    }
    /// `KernelSlotAbilityName_bc_00770bc0(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xbc.
    KernelSlotAbilityName_bc_00770bc0 @ 0x0077_0bc0 => extern "C" fn kernel_slot_name_00770bc0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0bc0, i, call_sym)
    }
    /// `KernelSlotAbilityName_be_00770c10(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xbe.
    KernelSlotAbilityName_be_00770c10 @ 0x0077_0c10 => extern "C" fn kernel_slot_name_00770c10(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0c10, i, call_sym)
    }
    /// `KernelSlotAbilityName_c0_00770c60(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xc0.
    KernelSlotAbilityName_c0_00770c60 @ 0x0077_0c60 => extern "C" fn kernel_slot_name_00770c60(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0c60, i, call_sym)
    }
    /// `KernelSlotAbilityName_c2_00770cb0(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xc2.
    KernelSlotAbilityName_c2_00770cb0 @ 0x0077_0cb0 => extern "C" fn kernel_slot_name_00770cb0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0cb0, i, call_sym)
    }
    /// `KernelSlotAbilityName_c4_00770d00(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xc4.
    KernelSlotAbilityName_c4_00770d00 @ 0x0077_0d00 => extern "C" fn kernel_slot_name_00770d00(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0d00, i, call_sym)
    }
    /// `KernelSlotAbilityName_c6_00770d70(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xc6.
    KernelSlotAbilityName_c6_00770d70 @ 0x0077_0d70 => extern "C" fn kernel_slot_name_00770d70(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0d70, i, call_sym)
    }
    /// `KernelSlotAbilityName_c8_00770dc0(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xc8.
    KernelSlotAbilityName_c8_00770dc0 @ 0x0077_0dc0 => extern "C" fn kernel_slot_name_00770dc0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0dc0, i, call_sym)
    }
    /// `KernelSlotAbilityName_ca_00770e10(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xca.
    KernelSlotAbilityName_ca_00770e10 @ 0x0077_0e10 => extern "C" fn kernel_slot_name_00770e10(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0e10, i, call_sym)
    }
    /// `KernelSlotAbilityName_cc_00770e60(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xcc.
    KernelSlotAbilityName_cc_00770e60 @ 0x0077_0e60 => extern "C" fn kernel_slot_name_00770e60(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0e60, i, call_sym)
    }
    /// `KernelSlotAbilityName_ce_00770eb0(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xce.
    KernelSlotAbilityName_ce_00770eb0 @ 0x0077_0eb0 => extern "C" fn kernel_slot_name_00770eb0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0eb0, i, call_sym)
    }
    /// `KernelSlotAbilityName_d0_00770f20(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xd0.
    KernelSlotAbilityName_d0_00770f20 @ 0x0077_0f20 => extern "C" fn kernel_slot_name_00770f20(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0f20, i, call_sym)
    }
    /// `KernelSlotAbilityName_d2_00770f70(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xd2.
    KernelSlotAbilityName_d2_00770f70 @ 0x0077_0f70 => extern "C" fn kernel_slot_name_00770f70(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0f70, i, call_sym)
    }
    /// `KernelSlotAbilityName_d4_00770fc0(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xd4.
    KernelSlotAbilityName_d4_00770fc0 @ 0x0077_0fc0 => extern "C" fn kernel_slot_name_00770fc0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_0fc0, i, call_sym)
    }
    /// `KernelSlotAbilityName_d6_00771010(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xd6.
    KernelSlotAbilityName_d6_00771010 @ 0x0077_1010 => extern "C" fn kernel_slot_name_00771010(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1010, i, call_sym)
    }
    /// `KernelSlotAbilityName_d8_00771060(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xd8.
    KernelSlotAbilityName_d8_00771060 @ 0x0077_1060 => extern "C" fn kernel_slot_name_00771060(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1060, i, call_sym)
    }
    /// `KernelSlotAbilityName_da_007710d0(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xda.
    KernelSlotAbilityName_da_007710d0 @ 0x0077_10d0 => extern "C" fn kernel_slot_name_007710d0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_10d0, i, call_sym)
    }
    /// `KernelSlotAbilityName_dc_00771120(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xdc.
    KernelSlotAbilityName_dc_00771120 @ 0x0077_1120 => extern "C" fn kernel_slot_name_00771120(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1120, i, call_sym)
    }
    /// `KernelSlotAbilityName_de_00771170(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xde.
    KernelSlotAbilityName_de_00771170 @ 0x0077_1170 => extern "C" fn kernel_slot_name_00771170(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1170, i, call_sym)
    }
    /// `KernelSlotAbilityName_e0_007711c0(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xe0.
    KernelSlotAbilityName_e0_007711c0 @ 0x0077_11c0 => extern "C" fn kernel_slot_name_007711c0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_11c0, i, call_sym)
    }
    /// `KernelSlotAbilityName_e2_00771210(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xe2.
    KernelSlotAbilityName_e2_00771210 @ 0x0077_1210 => extern "C" fn kernel_slot_name_00771210(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1210, i, call_sym)
    }
    /// `KernelSlotAbilityName_e4_00771280(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xe4.
    KernelSlotAbilityName_e4_00771280 @ 0x0077_1280 => extern "C" fn kernel_slot_name_00771280(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1280, i, call_sym)
    }
    /// `KernelSlotAbilityName_e6_007712d0(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xe6.
    KernelSlotAbilityName_e6_007712d0 @ 0x0077_12d0 => extern "C" fn kernel_slot_name_007712d0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_12d0, i, call_sym)
    }
    /// `KernelSlotAbilityName_e8_00771320(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xe8.
    KernelSlotAbilityName_e8_00771320 @ 0x0077_1320 => extern "C" fn kernel_slot_name_00771320(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1320, i, call_sym)
    }
    /// `KernelSlotAbilityName_ea_00771370(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xea.
    KernelSlotAbilityName_ea_00771370 @ 0x0077_1370 => extern "C" fn kernel_slot_name_00771370(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1370, i, call_sym)
    }
    /// `KernelSlotAbilityName_ec_007713c0(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xec.
    KernelSlotAbilityName_ec_007713c0 @ 0x0077_13c0 => extern "C" fn kernel_slot_name_007713c0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_13c0, i, call_sym)
    }
    /// `KernelSlotAbilityName_ee_00771430(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xee.
    KernelSlotAbilityName_ee_00771430 @ 0x0077_1430 => extern "C" fn kernel_slot_name_00771430(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1430, i, call_sym)
    }
    /// `KernelSlotAbilityName_f0_00771480(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xf0.
    KernelSlotAbilityName_f0_00771480 @ 0x0077_1480 => extern "C" fn kernel_slot_name_00771480(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1480, i, call_sym)
    }
    /// `KernelSlotAbilityName_f2_007714d0(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xf2.
    KernelSlotAbilityName_f2_007714d0 @ 0x0077_14d0 => extern "C" fn kernel_slot_name_007714d0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_14d0, i, call_sym)
    }
    /// `KernelSlotAbilityName_f4_00771520(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xf4.
    KernelSlotAbilityName_f4_00771520 @ 0x0077_1520 => extern "C" fn kernel_slot_name_00771520(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1520, i, call_sym)
    }
    /// `KernelSlotAbilityName_f6_00771570(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xf6.
    KernelSlotAbilityName_f6_00771570 @ 0x0077_1570 => extern "C" fn kernel_slot_name_00771570(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1570, i, call_sym)
    }
    /// `KernelSlotAbilityName_f8_007715e0(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xf8.
    KernelSlotAbilityName_f8_007715e0 @ 0x0077_15e0 => extern "C" fn kernel_slot_name_007715e0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_15e0, i, call_sym)
    }
    /// `KernelSlotAbilityName_fa_00771630(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xfa.
    KernelSlotAbilityName_fa_00771630 @ 0x0077_1630 => extern "C" fn kernel_slot_name_00771630(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1630, i, call_sym)
    }
    /// `KernelSlotAbilityName_fc_00771680(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xfc.
    KernelSlotAbilityName_fc_00771680 @ 0x0077_1680 => extern "C" fn kernel_slot_name_00771680(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1680, i, call_sym)
    }
    /// `KernelSlotAbilityName_fe_007716d0(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0xfe.
    KernelSlotAbilityName_fe_007716d0 @ 0x0077_16d0 => extern "C" fn kernel_slot_name_007716d0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_16d0, i, call_sym)
    }
    /// `KernelSlotAbilityName_100_00771720(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0x100.
    KernelSlotAbilityName_100_00771720 @ 0x0077_1720 => extern "C" fn kernel_slot_name_00771720(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1720, i, call_sym)
    }
    /// `KernelSlotAbilityName_102_00771790(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0x102.
    KernelSlotAbilityName_102_00771790 @ 0x0077_1790 => extern "C" fn kernel_slot_name_00771790(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1790, i, call_sym)
    }
    /// `KernelSlotAbilityName_104_007717e0(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0x104.
    KernelSlotAbilityName_104_007717e0 @ 0x0077_17e0 => extern "C" fn kernel_slot_name_007717e0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_17e0, i, call_sym)
    }
    /// `KernelSlotAbilityName_106_00771830(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0x106.
    KernelSlotAbilityName_106_00771830 @ 0x0077_1830 => extern "C" fn kernel_slot_name_00771830(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1830, i, call_sym)
    }
    /// `KernelSlotAbilityName_108_00771880(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0x108.
    KernelSlotAbilityName_108_00771880 @ 0x0077_1880 => extern "C" fn kernel_slot_name_00771880(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1880, i, call_sym)
    }
    /// `KernelSlotAbilityName_10a_007718d0(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0x10a.
    KernelSlotAbilityName_10a_007718d0 @ 0x0077_18d0 => extern "C" fn kernel_slot_name_007718d0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_18d0, i, call_sym)
    }
    /// `KernelSlotAbilityName_10c_00771940(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0x10c.
    KernelSlotAbilityName_10c_00771940 @ 0x0077_1940 => extern "C" fn kernel_slot_name_00771940(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1940, i, call_sym)
    }
    /// `KernelSlotAbilityName_10e_00771990(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0x10e.
    KernelSlotAbilityName_10e_00771990 @ 0x0077_1990 => extern "C" fn kernel_slot_name_00771990(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1990, i, call_sym)
    }
    /// `KernelSlotAbilityName_110_007719e0(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0x110.
    KernelSlotAbilityName_110_007719e0 @ 0x0077_19e0 => extern "C" fn kernel_slot_name_007719e0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_19e0, i, call_sym)
    }
    /// `KernelSlotAbilityName_112_00771a30(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0x112.
    KernelSlotAbilityName_112_00771a30 @ 0x0077_1a30 => extern "C" fn kernel_slot_name_00771a30(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1a30, i, call_sym)
    }
    /// `KernelSlotAbilityName_114_00771a80(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0x114.
    KernelSlotAbilityName_114_00771a80 @ 0x0077_1a80 => extern "C" fn kernel_slot_name_00771a80(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1a80, i, call_sym)
    }
    /// `KernelSlotAbilityName_116_00771af0(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0x116.
    KernelSlotAbilityName_116_00771af0 @ 0x0077_1af0 => extern "C" fn kernel_slot_name_00771af0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1af0, i, call_sym)
    }
    /// `KernelSlotAbilityName_118_00771b40(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0x118.
    KernelSlotAbilityName_118_00771b40 @ 0x0077_1b40 => extern "C" fn kernel_slot_name_00771b40(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1b40, i, call_sym)
    }
    /// `KernelSlotAbilityName_11a_00771b90(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0x11a.
    KernelSlotAbilityName_11a_00771b90 @ 0x0077_1b90 => extern "C" fn kernel_slot_name_00771b90(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1b90, i, call_sym)
    }
    /// `KernelSlotAbilityName_11c_00771be0(i)`: `Kernel_GetAbilityName` of record `i`'s `u16` at 0x11c.
    KernelSlotAbilityName_11c_00771be0 @ 0x0077_1be0 => extern "C" fn kernel_slot_name_00771be0(i: u32) -> u32 {
        savecmd::bridge::kernel_slot_name(&mut mem(), 0x0077_1be0, i, call_sym)
    }
    /// `sa_meth00dec(src, dst, size)`.
    sa_meth00dec @ 0x00a4_3da0 => extern "C" fn sa_meth00dec(src: u32, dst: u32, size: u32) -> u32 {
        decode::bridge::sa_meth00dec(&mut mem(), src, dst, size)
    }
    /// `pc_sizetbl_set_Ex()`.
    pc_sizetbl_set_Ex @ 0x00a4_3100 => extern "C" fn pc_sizetbl_set_ex() -> u32 {
        cdread::bridge::pc_sizetbl_set_ex(&mut mem())
    }
    /// `pc_module_sizetbl_set()`.
    pc_module_sizetbl_set @ 0x00a4_3080 => extern "C" fn pc_module_sizetbl_set() -> u32 {
        let mut w = super::StackFrame::<{ cdread::bridge::COUNTRY_WORK as usize / 4 }>::new();
        cdread::bridge::pc_sizetbl_set(&mut mem(), cdread::bridge::MODULE_SIZE_PATH, w.at())
    }
    /// `pc_sizetbl_set()`.
    pc_sizetbl_set @ 0x00a4_30c0 => extern "C" fn pc_sizetbl_set() -> u32 {
        let mut w = super::StackFrame::<{ cdread::bridge::COUNTRY_WORK as usize / 4 }>::new();
        cdread::bridge::pc_sizetbl_set(&mut mem(), cdread::bridge::SIZETBL_PATH, w.at())
    }
    /// `Sg_CameraPutRects()`.
    Sg_CameraPutRects @ 0x0083_f130 => extern "C" fn sg_camera_put_rects() -> u32 {
        camera::bridge::sg_camera_put_rects(&mut mem())
    }
    /// `Cache_FreeEntry(e)`.
    Cache_FreeEntry @ 0x0084_0ec0 => extern "C" fn cache_free_entry(e: u32) -> u32 {
        cache::bridge::cache_free_entry(&mut mem(), e)
    }
    /// `task_create_group7(entry, stack, prio)`.
    task_create_group7 @ 0x0077_a060 => extern "C" fn task_create_group7(entry: u32, stack: u32, prio: u32) -> u32 {
        cache::bridge::task_create_group7(&mut mem(), entry, stack, prio)
    }
    /// `Cam_SetPolar(id, side, theta, phi, len)`.
    Cam_SetPolar @ 0x007b_f700 => extern "C" fn cam_set_polar(id: u32, side: u32, a: u32, b: u32, c: u32) -> u32 {
        camera::bridge::cam_set_polar(&mut mem(), id, side, [a, b, c])
    }
    /// `Cam_AddPolar(id, side, dtheta, dphi, dlen)`.
    Cam_AddPolar @ 0x007b_f7f0 => extern "C" fn cam_add_polar(id: u32, side: u32, a: u32, b: u32, c: u32) -> u32 {
        camera::bridge::cam_add_polar(&mut mem(), id, side, [a, b, c])
    }
    /// `MsCameraSetTargetPolar_007bf600(id, side, flags, target, d, a, b)`.
    MsCameraSetTargetPolar_007bf600 @ 0x007b_f600 => extern "C" fn cam_set_target_polar(id: u32, side: u32, flags: u32, target: u32, d: u32, a: u32, b: u32) -> u32 {
        camera::bridge::cam_set_target_polar(&mut mem(), id, side, flags, target, [d, a, b])
    }
    /// `Sg_LightSetColor(kind, rgba)`.
    Sg_LightSetColor @ 0x0083_6310 => extern "C" fn sg_light_set_color(kind: u32, rgba: u32) -> u32 {
        light::bridge::sg_light_set_color(&mut mem(), kind, rgba)
    }
    // Round 15: the sphere grid pad entry draw callbacks (no caller reads eax).
    /// `AbmapPadEntryDrawIcon(panel, entry, index, x, y)`.
    AbmapPadEntryDrawIcon @ 0x00a4_f250 => extern "C" fn abmap_pad_entry_draw_icon(_p: u32, entry: u32, _i: u32, x: u32, y: u32) -> u32 {
        abmap::bridge::abmap_pad_entry_draw_icon(&mut mem(), entry, x, y, &mut |_, f, a| super::call_cdecl(f, a)).unwrap_or(0)
    }
    /// `AbmapPadEntryDrawIcon_copy_00a4f9a0(panel, entry, index, x, y)`.
    AbmapPadEntryDrawIcon_copy_00a4f9a0 @ 0x00a4_f9a0 => extern "C" fn abmap_pad_entry_draw_icon_copy(_p: u32, entry: u32, _i: u32, x: u32, y: u32) -> u32 {
        abmap::bridge::abmap_pad_entry_draw_icon(&mut mem(), entry, x, y, &mut |_, f, a| super::call_cdecl(f, a)).unwrap_or(0)
    }
    /// `AbmapPadEntryDrawText(panel, entry, index, x, y)`.
    AbmapPadEntryDrawText @ 0x00a4_f9e0 => extern "C" fn abmap_pad_entry_draw_text(_p: u32, entry: u32, _i: u32, x: u32, y: u32) -> u32 {
        abmap::bridge::abmap_pad_entry_draw_text(&mut mem(), entry, x, y, &mut |_, f, a| super::call_cdecl(f, a))
    }
    /// `AbmapPadEntryDrawHelp_e(panel, entry, index, x, y)`.
    AbmapPadEntryDrawHelp_e @ 0x00a4_f930 => extern "C" fn abmap_pad_entry_draw_help_e(_p: u32, entry: u32, _i: u32, x: u32, y: u32) -> u32 {
        abmap::bridge::abmap_pad_entry_draw_help(&mut mem(), entry, x, y, 0xe, &mut |_, f, a| super::call_cdecl(f, a))
    }
    /// `AbmapPadEntryDrawHelp_5(panel, entry, index, x, y)`.
    AbmapPadEntryDrawHelp_5 @ 0x00a4_fb70 => extern "C" fn abmap_pad_entry_draw_help_5(_p: u32, entry: u32, _i: u32, x: u32, y: u32) -> u32 {
        abmap::bridge::abmap_pad_entry_draw_help(&mut mem(), entry, x, y, 5, &mut |_, f, a| super::call_cdecl(f, a))
    }
    /// `AbmapPadEntryDrawStat(panel, entry, index, x, y)`.
    AbmapPadEntryDrawStat @ 0x00a4_fa30 => extern "C" fn abmap_pad_entry_draw_stat(_p: u32, entry: u32, _i: u32, x: u32, y: u32) -> u32 {
        abmap::bridge::abmap_pad_entry_draw_stat(&mut mem(), entry, x, y, &mut |_, f, a| super::call_cdecl(f, a))
    }
    /// `AbmapPadEntryDrawItem(panel, entry, index, x, y)` (`0x00a4f630`):
    /// the quad and its name buffer in this frame.
    AbmapPadEntryDrawItem @ 0x00a4_f630 => extern "C" fn abmap_pad_entry_draw_item(panel: u32, entry: u32, index: u32, x: u32, y: u32) -> u32 {
        let mut w = super::StackFrame::<{ abmap::bridge::ITEM_WORK as usize / 4 }>::new();
        abmap::bridge::abmap_pad_entry_draw_item(&mut mem(), w.at(), [panel, entry, index, x, y])
    }
    /// `Cam_Apply(camera)`.
    Cam_Apply @ 0x007b_c980 => extern "C" fn cam_apply(cam: u32) {
        let mut w = super::StackFrame::<{ camera::bridge::APPLY_WORK as usize / 4 }>::new();
        camera::bridge::cam_apply(&mut mem(), cam, w.at(), &mut |_, f, a| super::call_cdecl(f, a), |_, x| crt(Crt::Atan, x).to_bits());
    }
    /// `sa_meth01dec(src, dst, size)`.
    sa_meth01dec @ 0x00a4_3bc0 => extern "C" fn sa_meth01dec(src: u32, dst: u32, size: u32) -> u32 {
        decode::bridge::sa_meth01dec(&mut mem(), src, dst, size)
    }
    /// `sa_meth02dec(src, dst, size)`.
    sa_meth02dec @ 0x00a4_3cc0 => extern "C" fn sa_meth02dec(src: u32, dst: u32, size: u32) -> u32 {
        decode::bridge::sa_meth02dec(&mut mem(), src, dst, size)
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
    /// `Sg_PcRead(path)`.
    Sg_PcRead @ 0x0083_ae30 => extern "C" fn sg_pc_read(path: u32) -> u32 {
        control::bridge::pc_read(&mut mem(), path)
    }
    /// `Sg_PcReadD(path)`.
    Sg_PcReadD @ 0x0083_ae50 => extern "C" fn sg_pc_read_d(path: u32) -> u32 {
        control::bridge::pc_read_d(&mut mem(), path)
    }
    /// `Sg_ControlPlayer()`.
    Sg_ControlPlayer @ 0x0082_d180 => extern "C" fn sg_control_player() {
        player::bridge::control_player(&mut mem(), &crt_vector_trig, &crt_atan2);
    }
    /// `Sg_PlayerSetControlAngle(deg)`: `Sg_CameraGetYaw`'s `ST0` through the game.
    Sg_PlayerSetControlAngle @ 0x0082_d950 => extern "C" fn sg_player_set_control_angle(deg: f32) {
        player::bridge::set_control_angle(&mut mem(), deg, &mut |_, f, a| super::call_cdecl_f64(f, a));
    }
    /// `FFXApplication::resize(this)`: the base class's through the game.
    FFXApplication_resize @ 0x0042_f9a0 => extern "thiscall" fn ffx_application_resize(this: u32) -> u32 {
        ffx_sg::app::bridge::resize(
            &mut mem(),
            this,
            &mut |_, f, a| super::call_cdecl(f, a),
            &mut |_, f, t, a| super::call_thiscall(f, t, a),
        )
    }
    /// `Cam_SetScreenVector(screen, x, y, z, w, v)`.
    Cam_SetScreenVector @ 0x007b_c890 => extern "C" fn cam_set_screen_vector(screen: u32, x: i32, y: i32, z: i32, w: i32, v: u32) -> u32 {
        camera::bridge::set_screen_vector(&mut mem(), screen, [x, y, z, w], v)
    }
    /// `Sg_LightSetColorRGB(kind, r, g, b)`.
    Sg_LightSetColorRGB @ 0x0083_6400 => extern "C" fn sg_light_set_color_rgb(kind: u32, r: f32, g: f32, b: f32) -> u32 {
        light::bridge::sg_light_set_color_rgb(&mut mem(), kind, r, g, b)
    }
    /// `Light_Set(light, r, g, b, pitch, yaw)`: the CRT's sine and cosine.
    Light_Set @ 0x0083_6520 => extern "C" fn light_set(light: u32, r: i32, g: i32, b: i32, pitch: i32, yaw: i32) -> u32 {
        light::bridge::light_set(&mut mem(), &crt_vector_trig, light, [r, g, b], pitch, yaw)
    }
    /// `eiAbmSetLang()`.
    eiAbmSetLang @ 0x00a5_4ab0 => extern "C" fn ei_abm_set_lang() -> u32 {
        abmap::bridge::ei_abm_set_lang(&mut mem())
    }
    /// `Sg_FrameSync(dt)` (`0x00821e80`): the clock through
    /// `graphicGetTimeInMicroseconds`' `edx:eax`.
    Sg_FrameSync @ 0x0082_1e80 => extern "C" fn sg_frame_sync(dt: u32) -> u32 {
        let mut m = mem();
        let at = ffx_bridge::Mem::rebase(&m, state::bridge::TIME_US);
        state::bridge::frame_sync(&mut m, dt, &mut |_| super::call_cdecl_u64(at, &[]))
    }
    /// `eiAbmStart()`.
    eiAbmStart @ 0x00a5_4b40 => extern "C" fn ei_abm_start() -> u32 {
        abmap::bridge::ei_abm_start(&mut mem())
    }
    /// `eiAbmDrawFrame_00a54560()`.
    eiAbmDrawFrame_00a54560 @ 0x00a5_4560 => extern "C" fn ei_abm_draw_frame() -> u32 {
        abmap::bridge::ei_abm_draw_frame(&mut mem())
    }
    /// `eiAbmDraw()`.
    eiAbmDraw @ 0x00a5_45a0 => extern "C" fn ei_abm_draw() -> u32 {
        abmap::bridge::ei_abm_draw(&mut mem())
    }
    /// `eiAbmDrawImmediate_00a54660()`.
    eiAbmDrawImmediate_00a54660 @ 0x00a5_4660 => extern "C" fn ei_abm_draw_immediate() -> u32 {
        abmap::bridge::ei_abm_draw_immediate(&mut mem())
    }
    /// `eiAbmLoad()`.
    eiAbmLoad @ 0x00a5_4810 => extern "C" fn ei_abm_load() -> u32 {
        abmap::bridge::ei_abm_load(&mut mem())
    }
    /// `Sg_InitAll()`.
    Sg_InitAll @ 0x0083_b5f0 => extern "C" fn sg_init_all() -> u32 {
        state::bridge::init_all(&mut mem())
    }
    /// `Sg_Test(n)`.
    Sg_Test @ 0x0084_67b0 => extern "C" fn sg_test(n: u32) -> u32 {
        state::bridge::sg_test(&mut mem(), n)
    }
    /// `eiAbmCalc()`: the screen vector in this frame.
    eiAbmCalc @ 0x00a5_3570 => extern "C" fn ei_abm_calc() -> u32 {
        let mut w = super::StackFrame::<{ abmap::bridge::CALC_WORK as usize / 4 }>::new();
        abmap::bridge::ei_abm_calc(&mut mem(), w.at())
    }
    /// `eiAbmParaGet()`: `MsGetExcelData`'s strings word in this frame.
    eiAbmParaGet @ 0x00a5_4860 => extern "C" fn ei_abm_para_get() -> u32 {
        let mut w = super::StackFrame::<1>::new();
        abmap::bridge::ei_abm_para_get(&mut mem(), w.at())
    }
    /// `eiAbmDataInit(save)`: the paths and the table buffer in this frame.
    eiAbmDataInit @ 0x00a5_3de0 => extern "C" fn ei_abm_data_init(save: u32) -> u32 {
        let mut w = super::StackFrame::<{ abmap::bridge::DATA_INIT_WORK as usize / 4 }>::new();
        abmap::bridge::ei_abm_data_init(&mut mem(), save, w.at())
    }
    /// `AbmapDrawChrPanel()` (`0x00a4fbe0`): the message size in this
    /// frame.
    AbmapDrawChrPanel @ 0x00a4_fbe0 => extern "C" fn abmap_draw_chr_panel() -> u32 {
        let mut w = super::StackFrame::<{ abmap::bridge::CHR_PANEL_WORK as usize / 4 }>::new();
        abmap::bridge::abmap_draw_chr_panel(&mut mem(), w.at())
    }
    /// `pc_fat_index_filename_set()`: the country name in this frame.
    pc_fat_index_filename_set @ 0x00a4_2fc0 => extern "C" fn pc_fat_index_filename_set() -> u32 {
        let mut w = super::StackFrame::<{ cdread::bridge::FAT_COUNTRY_WORK as usize / 4 }>::new();
        cdread::bridge::pc_fat_index_filename_set(&mut mem(), w.at())
    }
    /// `Sg_LoadVramResource()`: the store request in this frame.
    Sg_LoadVramResource @ 0x0083_fc50 => extern "C" fn sg_load_vram_resource() -> u32 {
        let mut w = super::StackFrame::<{ gs::bridge::STORE_IMAGE_WORK as usize / 4 }>::new();
        gs::bridge::load_vram_resource(&mut mem(), w.at(), &mut |_, f, a| super::call_cdecl(f, a))
    }
    /// `Sg_CameraSetPlayer()`: the CRT's sine and cosine.
    Sg_CameraSetPlayer @ 0x0083_f2e0 => extern "C" fn sg_camera_set_player() -> u32 {
        state::bridge::camera_set_player(&mut mem(), &crt_vector_trig)
    }
    /// `Sg_EffectReadPoll()`.
    Sg_EffectReadPoll @ 0x0082_2250 => extern "C" fn sg_effect_read_poll() -> u32 {
        state::bridge::effect_read_poll(&mut mem())
    }
    /// `Sg_MapInit()`: the three paths in this frame.
    Sg_MapInit @ 0x0084_4d10 => extern "C" fn sg_map_init() {
        let mut w = super::StackFrame::<{ state::bridge::MAP_INIT_WORK as usize / 4 }>::new();
        state::bridge::map_init(&mut mem(), w.at());
    }
    /// `Sg_CanSpeedUp()`.
    Sg_CanSpeedUp @ 0x0081_fd30 => extern "C" fn sg_can_speed_up() -> u32 {
        state::bridge::can_speed_up(&mut mem())
    }
    /// `Sg_IdChrMove(chr)`: the point in this frame, `Sg_IdGetHeight`'s `ST0`
    /// through the game.
    Sg_IdChrMove @ 0x0083_e5f0 => extern "C" fn sg_id_chr_move(chr: u32) {
        let mut w = super::StackFrame::<4>::new();
        state::bridge::id_chr_move(&mut mem(), chr, w.at(), &mut |_, f, a| super::call_cdecl_f64(f, a));
    }
    /// `SG_UpdateMotion(chr, apply)`.
    SG_UpdateMotion @ 0x0083_8c90 => extern "C" fn sg_update_motion(chr: u32, apply: u32) {
        seq::bridge::update_motion(&mut mem(), chr, apply);
    }
    // ---- chara-main-map round 1
    /// `Sg_CacheDisposeChr(id)`: the last `Cache_Dispose`'s `eax`.
    Sg_CacheDisposeChr @ 0x0084_0e60 => extern "C" fn sg_cache_dispose_chr(id: u32) -> u32 {
        state::bridge::cache_dispose_chr(&mut mem(), id)
    }
    /// `Sg_EventjumpDispose()`: the last `dbgPrintf`'s `eax`.
    Sg_EventjumpDispose @ 0x0083_b420 => extern "C" fn sg_eventjump_dispose() -> u32 {
        state::bridge::eventjump_dispose(&mut mem())
    }
    /// `Sg_EventjumpCacheChrs_0083b670()`: `Ch_ReadSync`'s `eax`.
    Sg_EventjumpCacheChrs_0083b670 @ 0x0083_b670 => extern "C" fn sg_eventjump_cache_chrs() -> u32 {
        state::bridge::eventjump_cache_chrs(&mut mem())
    }
    // ---- chara-main-map round 2
    /// `Sg_ChrMovePushApart_0083d3b0()`: void (`Ch_CalcMain` drops `eax`);
    /// `Sg_VectorGetLengthXZ`'s `ST0` through the game.
    Sg_ChrMovePushApart_0083d3b0 @ 0x0083_d3b0 => extern "C" fn sg_chr_move_push_apart() {
        state::bridge::chr_move_push_apart(&mut mem(), &mut |_, f, a| super::call_cdecl_f64(f, a));
    }
    /// `Sg_EventjumpInit(n)`: the last `dbgPrintf`'s `eax`; the fade colour
    /// in a frame of its own.
    Sg_EventjumpInit @ 0x0083_b460 => extern "C" fn sg_eventjump_init(n: u32) -> u32 {
        let mut w = super::StackFrame::<3>::new();
        state::bridge::eventjump_init(&mut mem(), n, w.at())
    }
    /// `HandleSaveDataScreen()`: void; the debug description in its frame.
    HandleSaveDataScreen @ 0x0081_ff80 => extern "C" fn handle_save_data_screen() {
        let mut w = super::StackFrame::<{ state::bridge::SAVE_SCREEN_SCRATCH as usize / 4 }>::new();
        state::bridge::handle_save_data_screen(&mut mem(), w.at());
    }
    /// `AbmapDrawNodesEamap_00a51340()`: void.
    AbmapDrawNodesEamap_00a51340 @ 0x00a5_1340 => extern "C" fn abmap_draw_nodes_eamap() {
        // laid out and aligned as the original's frame (`ebp - 0x100` on 16)
        let mut work = super::AlignedFrame::<{ abmap::bridge::EAMAP_WORK as usize }>::new();
        let w = work.at();
        abmap::bridge::abmap_draw_nodes_eamap(&mut mem(), w, &mut |_, f, a| super::call_cdecl(f, a));
    }
    /// `MovieCdReadTaskLoop_0076e660()`: void (its caller ignores `eax`).
    MovieCdReadTaskLoop_0076e660 @ 0x0076_e660 => extern "C" fn movie_cd_read_task_loop() {
        cdread::bridge::movie_cd_read_task_loop(&mut mem());
    }
}

/// `Phyre_PTimerWin32_GetTime()` (`0x0042fd80`): the game's clock in
/// seconds (the harness's virtual clock under replay), for
/// `FFXApplication::getAppTime`.
fn app_clock() -> f64 {
    // a cdecl function of this build taking nothing and returning a double
    // in ST0
    #[allow(clippy::cast_possible_truncation)] // a game address
    ffx_bridge::route::cdecl(crate::rebase(0x0042_fd80) as u32, &[])
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
        Crt::Atan => 0x0094_9d5c,
    };
    // the thunk jumps to the CRT's intrinsic, which takes its argument in
    // ST0 (`fld qword`) and returns in ST0 (`fstp dword`)
    #[allow(clippy::cast_possible_truncation)] // a game address
    ffx_bridge::route::x87(crate::rebase(thunk) as u32, &[x])
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
    // the thunk jumps to the CRT's intrinsic, which takes its argument in
    // ST0 (`fld dword`: exactly the f32 widened) and returns in ST0
    // (`fstp dword`)
    #[allow(clippy::cast_possible_truncation)] // a game address
    ffx_bridge::route::x87::<f32>(crate::rebase(thunk) as u32, &[f64::from(f32::from_bits(x))])
        .to_bits()
}

/// MSVCR110's `_CIatan2` (`ST1` = y, `ST0` = x, the result in `ST0`)
/// through the game's import thunk (`0x009498bc`): both loaded with `fld
/// dword`, the result stored with `fstp dword`, as `Sg_ControlPlayer`
/// does; `f32` bits in and out.
fn crt_atan2(y: u32, x: u32) -> u32 {
    #[allow(clippy::cast_possible_truncation)] // a game address
    let f = crate::rebase(0x0094_98bc) as u32;
    // the thunk jumps to the CRT's `_CIatan2`, which takes its two
    // operands on the x87 stack (`fld dword` each: y in ST1, x in ST0) and
    // leaves the result there (`fstp dword`)
    let (y, x) = (f64::from(f32::from_bits(y)), f64::from(f32::from_bits(x)));
    ffx_bridge::route::x87::<f32>(f, &[y, x]).to_bits()
}
