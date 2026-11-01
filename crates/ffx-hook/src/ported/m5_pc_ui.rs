//! `ffx_pcui::m5u_*`: M5 stage 1, list `pc-ui` (the PC user interface
//! code the static call graph reaches and no coverage run ran).
//!
//! Threads: the main thread (the frame's input pass, the menus,
//! `graphicFinalize`) and the input thread (`AsyncControllerTaskManager`,
//! 32 KB of stack: the frames here are a few words); the shims keep no
//! state, frames are per call.

use super::{StackFrame, mem, ported};
use ffx_graphic::host::Conv;
use ffx_pcui::l6_pcui::bridge as l6u;
use ffx_pcui::l6_pcui2::bridge as l6u2;
use ffx_pcui::l6_pcui3::bridge as l6u3;
use ffx_pcui::m5u_a::bridge as a;
use ffx_pcui::m5u_b::bridge as b;
use ffx_pcui::m5u_c::bridge as c;
use ffx_pcui::m5u_d::bridge as d;
use ffx_pcui::m5u_e::bridge as e;
use ffx_pcui::m5u_f::bridge as ff;
use ffx_pcui::m5u_g::bridge as gg;
use ffx_pcui::m5u_h::bridge as hh;
use ffx_pcui::m5u_i::bridge as ii;
use ffx_pcui::m5u_j::bridge as jj;
use ffx_pcui::m5u_k::bridge as kk;
use ffx_pcui::m5u_l::bridge as ll;

ported! {
    // ---- batch 1 (ffx_pcui::m5u_a) ----
    /// `AsyncControllerTaskManager_GetControllerInfo(this; &word, &sticks)`.
    AsyncControllerTaskManager_GetControllerInfo @ 0x0068_ccd0 => extern "thiscall" fn get_controller_info(this: u32, word: u32, sticks: u32) -> u32 {
        let mut f = StackFrame::<4>::new();
        a::get_controller_info(&mut mem(), this, word, sticks, f.at())
    }
    /// `inputSetInputInfoToCurrentFrame()`.
    inputSetInputInfoToCurrentFrame @ 0x0063_0de0 => extern "C" fn set_input_info_to_current_frame() -> u32 {
        a::set_input_info_to_current_frame(&mut mem())
    }
    /// `SaveLoadMenu_changeSelectedBtn_00702a10()`.
    SaveLoadMenu_changeSelectedBtn_00702a10 @ 0x0070_2a10 => extern "C" fn save_menu_change_selected() -> u32 {
        a::save_menu_call_as3(&mut mem())
    }
    /// `Gui_WindowSetGlobalTrans(v)`.
    Gui_WindowSetGlobalTrans @ 0x0084_8f20 => extern "C" fn gui_window_set_global_trans(v: u32) {
        a::gui_window_set_global_trans(&mut mem(), v);
    }
    /// `Gu_ParseHex_00848f60(s, out)`: buffer `ebp - 0x14` (16 bytes).
    Gu_ParseHex_00848f60 @ 0x0084_8f60 => extern "C" fn gu_parse_hex(s: u32, out: u32) -> u32 {
        let mut f = StackFrame::<5>::new();
        a::hex_parse(&mut mem(), s, out, f.at())
    }
    /// `Gu_ParseNumber_00848fe0(s, out)`: buffer `ebp - 0x14`.
    Gu_ParseNumber_00848fe0 @ 0x0084_8fe0 => extern "C" fn gu_parse_number(s: u32, out: u32) -> u32 {
        let mut f = StackFrame::<5>::new();
        a::number_parse(&mut mem(), s, out, f.at())
    }
    /// `PVideoPlaybackWin32_dtor_00a26f30(this)`.
    PVideoPlaybackWin32_dtor_00a26f30 @ 0x00a2_6f30 => extern "thiscall" fn video_playback_dtor(this: u32) -> u32 {
        a::video_playback_dtor(&mut mem(), this)
    }
    /// `TextUtil_terminate(this)`.
    TextUtil_terminate @ 0x006f_6950 => extern "thiscall" fn text_util_terminate(this: u32) -> u32 {
        a::text_util_terminate(&mut mem(), this)
    }
    /// `MenuTextureManager_UnloadBGWaterTexture()`: name buffer `ebp - 0x104`.
    MenuTextureManager_UnloadBGWaterTexture @ 0x0067_ecf0 => extern "C" fn unload_bg_water() {
        let mut f = StackFrame::<0x41>::new();
        a::unload_bg_water(&mut mem(), f.at());
    }
    /// `MenuTextureManager_UnloadGameMenuTexture()`.
    MenuTextureManager_UnloadGameMenuTexture @ 0x0067_ee70 => extern "C" fn unload_game_menu() {
        let mut f = StackFrame::<0x41>::new();
        a::unload_game_menu(&mut mem(), f.at());
    }
    /// `MenuTextureManager_UnloadSaMenuNames_0067de60()`: frame `ebp - 0x234`.
    MenuTextureManager_UnloadSaMenuNames_0067de60 @ 0x0067_de60 => extern "C" fn unload_sa_names() {
        let mut f = StackFrame::<0x8d>::new();
        a::unload_sa_names(&mut mem(), f.at());
    }
    /// `MenuTextureManager_UnloadSaLangTextures_0067f030(this)`.
    MenuTextureManager_UnloadSaLangTextures_0067f030 @ 0x0067_f030 => extern "thiscall" fn unload_sa_lang(this: u32) -> u32 {
        a::unload_sa_lang(&mut mem(), this)
    }
    /// `MenuTextureManager_UnloadSaMenuTexture(this)`.
    MenuTextureManager_UnloadSaMenuTexture @ 0x0067_f230 => extern "thiscall" fn unload_sa_menu(this: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::unload_sa_menu(&mut mem(), this, f.at())
    }
    /// `MenuTextureManager_UnLoadDefaultMenuTexture(this)`.
    MenuTextureManager_UnLoadDefaultMenuTexture @ 0x0067_e9d0 => extern "thiscall" fn unload_default_menu(this: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::unload_default_menu(&mut mem(), this, f.at())
    }
    /// `MenuTextureManager_UnLoadExtraMenuTexture(this)`.
    MenuTextureManager_UnLoadExtraMenuTexture @ 0x0067_ec50 => extern "thiscall" fn unload_extra_menu(this: u32) -> u32 {
        a::unload_extra_menu(&mut mem(), this)
    }
    /// `MenuManager_ReleaseMeshes_0067b680(this)`.
    MenuManager_ReleaseMeshes_0067b680 @ 0x0067_b680 => extern "thiscall" fn menu_meshes_release(this: u32) {
        a::menu_meshes_release(&mut mem(), this);
    }
    /// `MenuManager_Teardown_0067b230(this)`.
    MenuManager_Teardown_0067b230 @ 0x0067_b230 => extern "thiscall" fn menu_manager_teardown(this: u32) {
        a::menu_manager_teardown(&mut mem(), this);
    }
    /// `StdDequeU32_tidy_0068d2d0(this)`.
    StdDequeU32_tidy_0068d2d0 @ 0x0068_d2d0 => extern "thiscall" fn deque_u32_tidy(this: u32) {
        a::deque_tidy(&mut mem(), this);
    }
    /// `StdDequeVec4_tidy_0068d330(this)`.
    StdDequeVec4_tidy_0068d330 @ 0x0068_d330 => extern "thiscall" fn deque_v4_tidy(this: u32) {
        a::deque_tidy(&mut mem(), this);
    }
    /// `AsyncControllerTaskManager_sDestroyInstance_0068db80()`.
    AsyncControllerTaskManager_sDestroyInstance_0068db80 @ 0x0068_db80 => extern "C" fn async_controller_destroy() {
        a::async_controller_destroy(&mut mem());
    }
    /// `RemapButtonIcon_sDestoryInstance()`.
    RemapButtonIcon_sDestoryInstance @ 0x006e_56d0 => extern "C" fn remap_button_icon_destroy() {
        a::delete_singleton(&mut mem(), a::REMAP_BUTTON_ICON);
    }
    /// `Singleton_destroy_006e76f0()`.
    Singleton_destroy_006e76f0 @ 0x006e_76f0 => extern "C" fn singleton_destroy_006e76f0() {
        a::delete_singleton(&mut mem(), a::SINGLETON_CE6DC0);
    }
    /// `GDraw_flushContext_00a3d240()`.
    GDraw_flushContext_00a3d240 @ 0x00a3_d240 => extern "C" fn gdraw_flush() {
        a::gdraw_flush(&mut mem());
    }
    /// `GDraw_releaseContext_00a3d570()` -> 0.
    GDraw_releaseContext_00a3d570 @ 0x00a3_d570 => extern "C" fn gdraw_release() -> u32 {
        a::gdraw_release(&mut mem())
    }
    /// `IggyMenuManager_dtor_00a25eb0(this)`.
    IggyMenuManager_dtor_00a25eb0 @ 0x00a2_5eb0 => extern "thiscall" fn iggy_manager_dtor(this: u32) {
        a::iggy_manager_dtor(&mut mem(), this);
    }
    /// `IggyMenuManager_EndDraw_00a362e0()`.
    IggyMenuManager_EndDraw_00a362e0 @ 0x00a3_62e0 => extern "C" fn end_draw() -> u32 {
        a::end_draw(&mut mem())
    }
    /// `IggyMenuManager_sDestroyInstance_00a26270()`.
    IggyMenuManager_sDestroyInstance_00a26270 @ 0x00a2_6270 => extern "C" fn iggy_manager_destroy() {
        a::iggy_manager_destroy(&mut mem());
    }
    /// `StdDequeU32_growMap_0068cf60(this; count)`.
    StdDequeU32_growMap_0068cf60 @ 0x0068_cf60 => extern "thiscall" fn deque_u32_grow_map(this: u32, count: u32) {
        a::grow_map(&mut mem(), this, count, 2);
    }
    /// `StdDequeU32_pushBack_0068d9f0(this; &v)`.
    StdDequeU32_pushBack_0068d9f0 @ 0x0068_d9f0 => extern "thiscall" fn deque_u32_push(this: u32, v: u32) {
        a::push_u32(&mut mem(), this, v);
    }
    /// `StdDequeVec4_growMap_0068d0e0(this; count)`.
    StdDequeVec4_growMap_0068d0e0 @ 0x0068_d0e0 => extern "thiscall" fn deque_v4_grow_map(this: u32, count: u32) {
        a::grow_map(&mut mem(), this, count, 0);
    }
    /// `StdDequeVec4_pushBack_0068da80(this; &v)`.
    StdDequeVec4_pushBack_0068da80 @ 0x0068_da80 => extern "thiscall" fn deque_v4_push(this: u32, v: u32) {
        a::push_v4(&mut mem(), this, v);
    }
    /// `AsyncControllerTaskManager_PushControllerInfo(this; word, s0..s3)`:
    /// hands on the address of its `word` slot; sticks at `ebp - 0x14`.
    AsyncControllerTaskManager_PushControllerInfo @ 0x0068_cc10 => extern "thiscall" fn push_controller_info(this: u32, _word: u32, s0: u32, s1: u32, s2: u32, s3: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<5>::new();
        a::push_controller_info(&mut mem(), this, slots, [s0, s1, s2, s3], f.at())
    }
    /// `AsyncControllerTaskManager_ClearControllerInfo(this)`.
    AsyncControllerTaskManager_ClearControllerInfo @ 0x0068_cc90 => extern "thiscall" fn clear_controller_info(this: u32) {
        a::clear_controller_info(&mut mem(), this);
    }
    /// `inputSwapState_00630df0(out, a, b, c)` -> `out`.
    inputSwapState_00630df0 @ 0x0063_0df0 => extern "C" fn input_swap_state(out: u32, x: u32, y: u32, z: u32) -> u32 {
        a::input_swap_state(&mut mem(), out, x, y, z)
    }
    /// `inputSetVibration(a, b, c)`.
    inputSetVibration @ 0x0063_0e20 => extern "C" fn input_set_vibration(x: u32, y: u32, z: u32) -> u32 {
        a::input_set_vibration(&mut mem(), x, y, z)
    }
    /// `TouchRecord_sCreateInstance_006897d0()`.
    TouchRecord_sCreateInstance_006897d0 @ 0x0068_97d0 => extern "C" fn touch_record_create() {
        a::touch_record_create(&mut mem());
    }
    /// `PArray16_sCreateInstance_0068ba20()`.
    PArray16_sCreateInstance_0068ba20 @ 0x0068_ba20 => extern "C" fn parray16_create() {
        a::parray16_create(&mut mem());
    }
    // ---- batch 2 (ffx_pcui::m5u_b) ----
    /// `MouseState_createInstance_00a25990()`.
    MouseState_createInstance_00a25990 @ 0x00a2_5990 => extern "C" fn mouse_state_create() {
        b::mouse_state_create(&mut mem());
    }
    /// `ShaderParams_write_00637c80(rec)`.
    ShaderParams_write_00637c80 @ 0x0063_7c80 => extern "C" fn write_shader_params(rec: u32) -> u32 {
        b::write_shader_params(&mut mem(), rec)
    }
    /// `CharacterInstance_setWorldMatrix_00637d40(rec)`: matrix at `ebp - 0x44`.
    CharacterInstance_setWorldMatrix_00637d40 @ 0x0063_7d40 => extern "C" fn set_world_matrix(rec: u32) {
        let mut f = StackFrame::<0x11>::new();
        b::set_world_matrix(&mut mem(), rec, f.at());
    }
    /// `MenuManager_SetMenuUiMaterialUnloaded(this; name)` (tail jump).
    MenuManager_SetMenuUiMaterialUnloaded @ 0x0067_e6a0 => extern "thiscall" fn set_menu_ui_material_unloaded(this: u32, name: u32) -> u32 {
        b::set_menu_ui_material_unloaded(&mut mem(), this, name)
    }
    /// `RemapButtonIcon_resetButtonIconScale(this)`.
    RemapButtonIcon_resetButtonIconScale @ 0x006e_76b0 => extern "thiscall" fn reset_button_icon_scale(this: u32) {
        b::reset_button_icon_scale(&mut mem(), this);
    }
    /// `RemapButtonIcon_setButtonIconScale(this; x, y)`.
    RemapButtonIcon_setButtonIconScale @ 0x006e_7720 => extern "thiscall" fn set_button_icon_scale(this: u32, x: u32, y: u32) {
        b::set_button_icon_scale(&mut mem(), this, x, y);
    }
    /// `MenuManager_GetMenuMeshInstace(this; i)`.
    MenuManager_GetMenuMeshInstace @ 0x0067_b9d0 => extern "thiscall" fn get_menu_mesh_instance(this: u32, i: u32) -> u32 {
        b::get_menu_mesh_instance(&mut mem(), this, i)
    }
    /// `MenuFontNames_ctor_0067b180(this)` -> `this`.
    MenuFontNames_ctor_0067b180 @ 0x0067_b180 => extern "thiscall" fn font_names_init(this: u32) -> u32 {
        b::font_names_init(&mut mem(), this)
    }
    /// `MenuTextureManager_LoadDefaultMenuTexture_0067c7a0(this)`.
    MenuTextureManager_LoadDefaultMenuTexture_0067c7a0 @ 0x0067_c7a0 => extern "thiscall" fn load_default_menu(this: u32) -> u32 {
        b::load_default_menu(&mut mem(), this)
    }
    /// `MenuTextureManager_LoadBGWaterTexture()`.
    MenuTextureManager_LoadBGWaterTexture @ 0x0067_c6f0 => extern "C" fn load_bg_water() -> u32 {
        b::load_bg_water(&mut mem())
    }
    /// `MenuTextureManager_MenuLoadTexture(this; id)`: the cluster at `ebp - 4`.
    MenuTextureManager_MenuLoadTexture @ 0x0067_dbb0 => extern "thiscall" fn menu_load_texture(_this: u32, id: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        b::menu_load_texture(&mut mem(), id, f.at())
    }
    /// `MenuTextureManager_LoadCloudSaveUITexture(screen)` (the caller's `ecx` on).
    MenuTextureManager_LoadCloudSaveUITexture @ 0x0067_c720 => extern "thiscall" fn load_cloud_save_ui(this: u32, screen: u32) -> u32 {
        b::load_cloud_save_ui(&mut mem(), this, screen)
    }
    /// `MenuTextureManager_LoadDreamMapTexture()`.
    MenuTextureManager_LoadDreamMapTexture @ 0x0067_c8c0 => extern "C" fn load_dream_map() -> u32 {
        b::load_dream_map(&mut mem())
    }
    /// `MenuTextureManager_LoadEternalCalmCopyRightTexture()`.
    MenuTextureManager_LoadEternalCalmCopyRightTexture @ 0x0067_c8f0 => extern "C" fn load_copyright() -> u32 {
        b::load_copyright(&mut mem())
    }
    /// `MenuTextureManager_LoadGameMenuTexture(kind)`.
    MenuTextureManager_LoadGameMenuTexture @ 0x0067_cb30 => extern "stdcall" fn load_game_menu(kind: u32) -> u32 {
        b::load_game_menu(&mut mem(), kind)
    }
    /// `MenuTextureManager_LoadSaSubMenuTexture(this; name)`: reuses its
    /// argument slot; the cluster at `ebp - 4` without an old texture.
    MenuTextureManager_LoadSaSubMenuTexture @ 0x0067_d0b0 => extern "thiscall" fn load_sa_sub_menu(this: u32, name: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        b::load_sa_sub_menu(&mut mem(), this, name, slots, f.at())
    }
    /// `MenuTextureManager_LoadSceneEventFontTexture(this; name)`.
    MenuTextureManager_LoadSceneEventFontTexture @ 0x0067_d370 => extern "thiscall" fn load_scene_event_font(this: u32, name: u32) {
        b::load_scene_event_font(&mut mem(), this, name);
    }
    /// `MenuTextureManager_LoadLanguageTexture_0067d650()`.
    MenuTextureManager_LoadLanguageTexture_0067d650 @ 0x0067_d650 => extern "C" fn load_one_0067d650() -> u32 {
        b::load_one(&mut mem(), 0x00c5_9d84)
    }
    /// `MenuTextureManager_LoadGameMenuTexture3_0067d670()`.
    MenuTextureManager_LoadGameMenuTexture3_0067d670 @ 0x0067_d670 => extern "C" fn load_one_0067d670() -> u32 {
        b::load_one(&mut mem(), 0x00c5_9d74)
    }
    /// `MenuTextureManager_LoadMenuTexture6c_0067d690()`.
    MenuTextureManager_LoadMenuTexture6c_0067d690 @ 0x0067_d690 => extern "C" fn load_one_0067d690() -> u32 {
        b::load_one(&mut mem(), 0x00c5_9d6c)
    }
    /// `MenuManager_ResetPauseVertexInfo(this)`.
    MenuManager_ResetPauseVertexInfo @ 0x0067_dfa0 => extern "thiscall" fn reset_pause_vertex_info(this: u32) -> u32 {
        b::reset_pause_vertex_info(&mut mem(), this)
    }
    /// `MenuTextureManager_UnLoadDreamMapTexture()`: name buffer `ebp - 0x104`.
    MenuTextureManager_UnLoadDreamMapTexture @ 0x0067_ebc0 => extern "C" fn unload_dream_map() {
        let mut f = StackFrame::<0x41>::new();
        b::unload_dream_map(&mut mem(), f.at());
    }
    /// `MenuTextureManager_UnloadCloudSaveUITexture(screen)`.
    MenuTextureManager_UnloadCloudSaveUITexture @ 0x0067_edc0 => extern "thiscall" fn unload_cloud_save_ui(this: u32, screen: u32) -> u32 {
        b::unload_cloud_save_ui(&mut mem(), this, screen)
    }
    /// `MenuTextureManager_UnloadEternalCalmCopyRightTexture(this)`.
    MenuTextureManager_UnloadEternalCalmCopyRightTexture @ 0x0067_ee40 => extern "thiscall" fn unload_copyright(this: u32) -> u32 {
        b::unload_copyright(&mut mem(), this)
    }
    /// `MenuTextureManager_UnloadSaSubMenuTexture_0067f3c0(this)`.
    MenuTextureManager_UnloadSaSubMenuTexture_0067f3c0 @ 0x0067_f3c0 => extern "thiscall" fn unload_sa_sub_menu(this: u32) {
        b::unload_sa_sub_menu(&mut mem(), this);
    }
    /// `MenuTextureManager_UnloadLanguageTexture_0067f920()`.
    MenuTextureManager_UnloadLanguageTexture_0067f920 @ 0x0067_f920 => extern "C" fn unload_one_0067f920() {
        let mut f = StackFrame::<0x41>::new();
        b::unload_one(&mut mem(), 0x00c5_9d84, f.at());
    }
    /// `MenuTextureManager_UnloadGameMenuTexture3_0067f9a0()`.
    MenuTextureManager_UnloadGameMenuTexture3_0067f9a0 @ 0x0067_f9a0 => extern "C" fn unload_one_0067f9a0() {
        let mut f = StackFrame::<0x41>::new();
        b::unload_one(&mut mem(), 0x00c5_9d74, f.at());
    }
    /// `MenuTextureManager_UnloadMenuTexture6c_0067fa20()`.
    MenuTextureManager_UnloadMenuTexture6c_0067fa20 @ 0x0067_fa20 => extern "C" fn unload_one_0067fa20() {
        let mut f = StackFrame::<0x41>::new();
        b::unload_one(&mut mem(), 0x00c5_9d6c, f.at());
    }
    /// `StdString_tidy_00681800(this)`.
    StdString_tidy_00681800 @ 0x0068_1800 => extern "thiscall" fn string_tidy(this: u32) {
        b::string_tidy(&mut mem(), this);
    }
    // ---- batch 3 (ffx_pcui::m5u_c) ----
    /// `TouchRecord_sDestroyInstance_00689830()`.
    TouchRecord_sDestroyInstance_00689830 @ 0x0068_9830 => extern "C" fn touch_record_destroy() {
        c::touch_record_destroy(&mut mem());
    }
    /// `PArray_free_0068b830(this)`.
    PArray_free_0068b830 @ 0x0068_b830 => extern "thiscall" fn parray_free(this: u32) {
        c::parray_free(&mut mem(), this);
    }
    /// `PArray16_clear_0068bae0(this)`.
    PArray16_clear_0068bae0 @ 0x0068_bae0 => extern "thiscall" fn parray16_clear(this: u32) {
        c::parray16_clear(&mut mem(), this);
    }
    /// `PArray16_sDestroyInstance_0068bba0()`.
    PArray16_sDestroyInstance_0068bba0 @ 0x0068_bba0 => extern "C" fn parray16_destroy() {
        c::parray16_destroy(&mut mem());
    }
    /// `AsyncControllerTaskManager_AsyncControllerTaskManager(this)` -> `this`:
    /// the thread parameters at `ebp - 0x40`.
    AsyncControllerTaskManager_AsyncControllerTaskManager @ 0x0068_c830 => extern "thiscall" fn controller_ctor(this: u32) -> u32 {
        let mut f = StackFrame::<0x0c>::new();
        c::controller_ctor(&mut mem(), this, f.at())
    }
    /// `StdDequeU32_dtor_0068ca50(this)`.
    StdDequeU32_dtor_0068ca50 @ 0x0068_ca50 => extern "thiscall" fn deque_u32_dtor(this: u32) {
        c::deque_u32_dtor(&mut mem(), this);
    }
    /// `AsyncControllerTaskManager_ThreadMain()` (the input thread's entry).
    AsyncControllerTaskManager_ThreadMain @ 0x0068_d740 => extern "C" fn controller_thread() -> u32 {
        c::controller_thread(&mut mem())
    }
    /// `AsyncControllerTaskManager_sCreateInstance()`.
    AsyncControllerTaskManager_sCreateInstance @ 0x0068_db00 => extern "C" fn controller_create() {
        c::controller_create(&mut mem());
    }
    /// `FFEscMenu_updateSpecialFeaturePage_jmp_00702560(this)`.
    FFEscMenu_updateSpecialFeaturePage_jmp_00702560 @ 0x0070_2560 => extern "thiscall" fn esc_update_special_feature_jump(this: u32) -> u32 {
        c::esc_update_special_feature_jump(&mut mem(), this)
    }
    /// `FFEscMenu_ctor_0068dfd0(this; path, x, y, w, h)` -> `this`.
    FFEscMenu_ctor_0068dfd0 @ 0x0068_dfd0 => extern "thiscall" fn esc_menu_ctor(this: u32, path: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
        c::esc_menu_ctor(&mut mem(), this, path, [x, y, w, h])
    }
    /// `LocText_ConvertLineBreaks_jmp_006e96b0(src, dst, len)`.
    LocText_ConvertLineBreaks_jmp_006e96b0 @ 0x006e_96b0 => extern "C" fn convert_line_breaks_jump(src: u32, dst: u32, len: u32) -> u32 {
        c::convert_line_breaks_jump(&mut mem(), src, dst, len)
    }
    /// `FFEscMenu_onParameterButton_0068f610(menu, args)`: value ref at `ebp - 0x48`.
    FFEscMenu_onParameterButton_0068f610 @ 0x0068_f610 => extern "C" fn on_parameter_button(menu: u32, args: u32) {
        let mut f = StackFrame::<0x0e>::new();
        c::on_parameter_button(&mut mem(), menu, args, f.at());
    }
    /// `FFEscMenu_queueDefaultKey_0068f6b0(yes, menu)`.
    FFEscMenu_queueDefaultKey_0068f6b0 @ 0x0068_f6b0 => extern "C" fn queue_default_key(yes: u32, menu: u32) -> u32 {
        c::queue_default_key(&mut mem(), yes, menu)
    }
    /// `FFEscMenu_callOnDefaultKey_0068f6d0(menu)`.
    FFEscMenu_callOnDefaultKey_0068f6d0 @ 0x0068_f6d0 => extern "C" fn call_on_default_key(menu: u32) -> u32 {
        c::call_on_default_key(&mut mem(), menu)
    }
    /// `FFEscMenu_onSpecialFeatureHelpString(menu)`.
    FFEscMenu_onSpecialFeatureHelpString @ 0x0068_f6f0 => extern "C" fn on_special_feature_help(menu: u32) -> u32 {
        c::on_special_feature_help(&mut mem(), menu)
    }
    /// `FFEscMenu_showDefaultKeyPopUp_0068f710(menu)`: frame `ebp - 0x44c`.
    FFEscMenu_showDefaultKeyPopUp_0068f710 @ 0x0068_f710 => extern "C" fn show_default_key_popup(menu: u32) {
        let mut f = StackFrame::<0x113>::new();
        c::show_default_key_popup(&mut mem(), menu, f.at());
    }
    /// `FFEscMenu_onPopUpButton_0068f8f0(menu, args)`.
    FFEscMenu_onPopUpButton_0068f8f0 @ 0x0068_f8f0 => extern "C" fn on_popup_button(_menu: u32, args: u32) -> u32 {
        c::on_popup_button(&mut mem(), args)
    }
    /// `FFEscMenu_onQuit_0068fb00(menu)`.
    FFEscMenu_onQuit_0068fb00 @ 0x0068_fb00 => extern "C" fn on_quit(menu: u32) -> u32 {
        c::on_quit(&mut mem(), menu)
    }
    /// `FFEscMenu_QuitConfirmed_0068fcf0(yes)`.
    FFEscMenu_QuitConfirmed_0068fcf0 @ 0x0068_fcf0 => extern "C" fn quit_confirmed(yes: u32) -> u32 {
        c::quit_confirmed(&mut mem(), yes)
    }
    // ---- batch 4 (ffx_pcui::m5u_d) ----
    /// `TextUtil_resize(this; w, h)`: frame `ebp - 0xc8`.
    TextUtil_resize @ 0x006f_6670 => extern "thiscall" fn text_resize(this: u32, w: u32, h: u32) {
        let mut f = StackFrame::<0x32>::new();
        d::text_resize(&mut mem(), this, w, h, f.at());
    }
    /// `PtrVector_pushBack_006a2b20(this; &v)`.
    PtrVector_pushBack_006a2b20 @ 0x006a_2b20 => extern "thiscall" fn vec_push_back(this: u32, v: u32) {
        d::vec_push_back(&mut mem(), this, v);
    }
    /// `IggyString_skipDelims_00a29030(s, delims)`.
    IggyString_skipDelims_00a29030 @ 0x00a2_9030 => extern "C" fn skip_delims(s: u32, delims: u32) -> u32 {
        d::skip_delims(&mut mem(), s, delims)
    }
    /// `IggyString_countTokens_00a290d0(s, delims)`.
    IggyString_countTokens_00a290d0 @ 0x00a2_90d0 => extern "C" fn count_tokens(s: u32, delims: u32) -> u32 {
        d::count_tokens(&mut mem(), s, delims)
    }
    /// `IggyString_tokenize_00a29140(s, delims, arr)`.
    IggyString_tokenize_00a29140 @ 0x00a2_9140 => extern "C" fn tokenize(s: u32, delims: u32, arr: u32) -> u32 {
        d::tokenize(&mut mem(), s, delims, arr)
    }
    /// `RemapButtonIcon_ctor_006e77d0(this)` -> `this`.
    RemapButtonIcon_ctor_006e77d0 @ 0x006e_77d0 => extern "thiscall" fn icon_state_init(this: u32) -> u32 {
        d::icon_state_init(&mut mem(), this)
    }
    /// `SkipPrompt_show_006e78a0(this)`.
    SkipPrompt_show_006e78a0 @ 0x006e_78a0 => extern "thiscall" fn skip_prompt_show(this: u32) {
        d::skip_prompt_show(&mut mem(), this);
    }
    /// `SkipPrompt_hide_006e78c0(this)`.
    SkipPrompt_hide_006e78c0 @ 0x006e_78c0 => extern "thiscall" fn skip_prompt_hide(this: u32) {
        d::skip_prompt_hide(&mut mem(), this);
    }
    /// `RemapButtonIcon_setButtons_006e78e0(this; a, b, c)`.
    RemapButtonIcon_setButtons_006e78e0 @ 0x006e_78e0 => extern "thiscall" fn icon_set_buttons(this: u32, x: u32, y: u32, z: u32) {
        d::icon_set_buttons(&mut mem(), this, [x, y, z]);
    }
    /// `StdMapIterator_dec_006e91f0(it)` -> `it`.
    StdMapIterator_dec_006e91f0 @ 0x006e_91f0 => extern "thiscall" fn tree_dec(it: u32) -> u32 {
        d::tree_dec(&mut mem(), it)
    }
    /// `InitIggyHelper()`: the record at `ebp - 8`.
    InitIggyHelper @ 0x006e_96c0 => extern "C" fn init_iggy_helper() -> u32 {
        let mut f = StackFrame::<2>::new();
        d::init_iggy_helper(&mut mem(), f.at())
    }
    /// `UnloadCameraData()`.
    UnloadCameraData @ 0x006f_6f80 => extern "C" fn unload_camera_data() {
        d::unload_camera_data(&mut mem());
    }
    /// `FFEscMenu_onDebugFill_006fc970(yes, &choice)`.
    FFEscMenu_onDebugFill_006fc970 @ 0x006f_c970 => extern "C" fn on_debug_fill(yes: u32, choice: u32) -> u32 {
        d::on_debug_fill(&mut mem(), yes, choice)
    }
    /// `IggyValueRef_free_006fcc60(this)`.
    IggyValueRef_free_006fcc60 @ 0x006f_cc60 => extern "thiscall" fn value_ref_free(this: u32) -> u32 {
        d::value_ref_free(&mut mem(), this)
    }
    /// `FFEscMenu_readS32Array12_006fed60(this; out, args)`: the path at `ebp - 0x34`.
    FFEscMenu_readS32Array12_006fed60 @ 0x006f_ed60 => extern "thiscall" fn read_s32_array12(this: u32, out: u32, args: u32) -> u32 {
        let mut f = StackFrame::<0x0d>::new();
        d::read_s32_array12(&mut mem(), this, out, args, f.at())
    }
    // ---- batch 5 (ffx_pcui::m5u_e) ----
    /// `IggyPlayerOpenScaled_006e9250(path, x, y, w, h)`: path at `ebp - 0x110`.
    IggyPlayerOpenScaled_006e9250 @ 0x006e_9250 => extern "C" fn open_scaled(path: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
        let mut f = StackFrame::<0x40>::new();
        e::open_scaled(&mut mem(), path, [x, y, w, h], f.at())
    }
    /// `IggyPlayerOpen_006e93f0(path, x, y, w, h)`.
    IggyPlayerOpen_006e93f0 @ 0x006e_93f0 => extern "C" fn open_at(path: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
        let mut f = StackFrame::<0x40>::new();
        e::open_at(&mut mem(), path, [x, y, w, h], f.at())
    }
    /// `FFEscMenu_onParameterButtonClicked(arg)`: frame `ebp - 0x364`.
    FFEscMenu_onParameterButtonClicked @ 0x006f_c7b0 => extern "C" fn on_parameter_button_clicked(arg: u32) {
        let mut f = StackFrame::<0xd9>::new();
        e::on_parameter_button_clicked(&mut mem(), arg, f.at());
    }
    /// `FFEscMenu_confirmVideoSettings_006ff060(this; changed)`: frame `ebp - 0x364`.
    FFEscMenu_confirmVideoSettings_006ff060 @ 0x006f_f060 => extern "thiscall" fn confirm_video_settings(this: u32, changed: u32) -> u32 {
        let mut f = StackFrame::<0xd9>::new();
        e::confirm_video_settings(&mut mem(), this, changed, f.at())
    }
    /// `FFEscMenu_onVideoApplied_006ff1b0(yes)`.
    FFEscMenu_onVideoApplied_006ff1b0 @ 0x006f_f1b0 => extern "C" fn on_video_applied(yes: u32) -> u32 {
        e::on_video_applied(&mut mem(), yes)
    }
    /// `FFEscMenu_onVideoSettingsChanged_006ff1e0(menu, args)`.
    FFEscMenu_onVideoSettingsChanged_006ff1e0 @ 0x006f_f1e0 => extern "C" fn on_video_settings_changed(menu: u32, args: u32) -> u32 {
        e::on_video_settings_changed(&mut mem(), menu, args)
    }
    /// `FFEscMenu_onVideoAnswer_006ff490(yes)`.
    FFEscMenu_onVideoAnswer_006ff490 @ 0x006f_f490 => extern "C" fn on_video_answer(yes: u32) -> u32 {
        e::on_video_answer(&mut mem(), yes)
    }
    /// `FFEscMenu_onVideoSettingRefresh(menu, args)`.
    FFEscMenu_onVideoSettingRefresh @ 0x006f_f4b0 => extern "C" fn on_video_setting_refresh(menu: u32, args: u32) -> u32 {
        e::on_video_setting_refresh(&mut mem(), menu, args)
    }
    /// `FFEscMenu_onVoiceVolumeChanged(menu, args)`.
    FFEscMenu_onVoiceVolumeChanged @ 0x006f_f4f0 => extern "C" fn on_voice_volume(menu: u32, args: u32) -> u32 {
        e::on_volume_changed(&mut mem(), menu, args, e::VOLUMES[0])
    }
    /// `FFEscMenu_onMusicVolumeChanged(menu, args)`.
    FFEscMenu_onMusicVolumeChanged @ 0x006f_f530 => extern "C" fn on_music_volume(menu: u32, args: u32) -> u32 {
        e::on_volume_changed(&mut mem(), menu, args, e::VOLUMES[1])
    }
    /// `FFEscMenu_onSfxVolumeChanged(menu, args)`.
    FFEscMenu_onSfxVolumeChanged @ 0x006f_f570 => extern "C" fn on_sfx_volume(menu: u32, args: u32) -> u32 {
        e::on_volume_changed(&mut mem(), menu, args, e::VOLUMES[2])
    }
    /// `FFEscMenu_onMasterVolumeChanged(menu, args)`.
    FFEscMenu_onMasterVolumeChanged @ 0x006f_f5b0 => extern "C" fn on_master_volume(menu: u32, args: u32) -> u32 {
        e::on_volume_changed(&mut mem(), menu, args, e::VOLUMES[3])
    }
    // ---- batch 6 (ffx_pcui::m5u_f) ----
    /// `FFEscMenu_onAudioSettingRefresh(menu, args)`: frame `ebp - 0x5c`.
    FFEscMenu_onAudioSettingRefresh @ 0x006f_f660 => extern "C" fn on_audio_setting_refresh(menu: u32, args: u32) -> u32 {
        let mut f = StackFrame::<0x17>::new();
        ff::on_audio_setting_refresh(&mut mem(), menu, args, f.at())
    }
    /// `FFEscMenu_callMember1_006ffbb0(this, f, arg)`.
    FFEscMenu_callMember1_006ffbb0 @ 0x006f_fbb0 => extern "C" fn call_member1(this: u32, f: u32, arg: u32) -> u32 {
        let mut fr = StackFrame::<2>::new();
        ff::call_member1(&mut mem(), this, f, arg, fr.at())
    }
    /// `FFEscMenu_callBoundMember_006ffc30(bound)`.
    FFEscMenu_callBoundMember_006ffc30 @ 0x006f_fc30 => extern "C" fn call_bound_member(bound: u32) {
        ff::call_bound_member(&mut mem(), bound);
    }
    /// `FFEscMenu_clearKeys_006ffc50()`.
    FFEscMenu_clearKeys_006ffc50 @ 0x006f_fc50 => extern "C" fn clear_keys() -> u32 {
        ff::clear_keys(&mut mem())
    }
    /// `FFEscMenu_resetInputState_00701930(this)`: the state at `ebp - 0xc`.
    FFEscMenu_resetInputState_00701930 @ 0x0070_1930 => extern "thiscall" fn reset_input_state(this: u32) -> u32 {
        let mut f = StackFrame::<3>::new();
        ff::reset_input_state(&mut mem(), this, f.at())
    }
    /// `FFEscMenu_onKeyIncomplete_00701660(menu)`.
    FFEscMenu_onKeyIncomplete_00701660 @ 0x0070_1660 => extern "C" fn on_key_incomplete(menu: u32) -> u32 {
        ff::on_key_incomplete(&mut mem(), menu)
    }
    /// `IggyValue_getStringUTF8_00a28b60(this; i, buf, size)`: the length
    /// through its `i` slot; the path at `ebp - 0x18`.
    IggyValue_getStringUTF8_00a28b60 @ 0x00a2_8b60 => extern "thiscall" fn get_string_utf8(this: u32, i: u32, buf: u32, size: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<6>::new();
        ff::get_string_utf8(&mut mem(), this, i, buf, size, slots, f.at())
    }
    /// `IggyMenu_setEnableCeb248_007029e0(on)`.
    IggyMenu_setEnableCeb248_007029e0 @ 0x0070_29e0 => extern "C" fn save_menu_set_enable(on: u32) -> u32 {
        ff::save_menu_set_enable(&mut mem(), on)
    }
    // ---- batch 7 (ffx_pcui::m5u_f) ----
    /// `FFEscMenu_onPadIncomplete_00701e40(menu)`.
    FFEscMenu_onPadIncomplete_00701e40 @ 0x0070_1e40 => extern "C" fn on_pad_incomplete(menu: u32) -> u32 {
        ff::on_pad_incomplete(&mut mem(), menu)
    }
    /// `FFEscMenu_beginPadCapture_00701be0(menu)`: the old state at `ebp - 0xc`.
    FFEscMenu_beginPadCapture_00701be0 @ 0x0070_1be0 => extern "C" fn begin_pad_capture(menu: u32) -> u32 {
        let mut f = StackFrame::<3>::new();
        ff::begin_pad_capture(&mut mem(), menu, f.at())
    }
    /// `FFEscMenu_beginKeyCapture_007013e0(menu)`: the old state at `ebp - 0xc`.
    FFEscMenu_beginKeyCapture_007013e0 @ 0x0070_13e0 => extern "C" fn begin_keyboard_capture(menu: u32) -> u32 {
        let mut f = StackFrame::<3>::new();
        ff::begin_keyboard_capture(&mut mem(), menu, f.at())
    }
    // ---- batch 8 (ffx_pcui::m5u_g) ----
    /// `Gu_PutByte_00853400(c)`.
    Gu_PutByte_00853400 @ 0x0085_3400 => extern "C" fn put_byte(c: u32) {
        gg::put_byte(&mut mem(), c);
    }
    /// `Gui_SpriteWindowInit_00853780()`.
    Gui_SpriteWindowInit_00853780 @ 0x0085_3780 => extern "C" fn sprite_window_init() -> u32 {
        gg::sprite_window_init(&mut mem())
    }
    /// `Gui_ChrInfo()`.
    Gui_ChrInfo @ 0x0085_37b0 => extern "C" fn chr_info() -> u32 {
        gg::chr_info(&mut mem())
    }
    /// `Gui_ClockInit()`.
    Gui_ClockInit @ 0x0085_3ea0 => extern "C" fn clock_init() -> u32 {
        gg::clock_init(&mut mem())
    }
    /// `Gui_YanagiInit()`.
    Gui_YanagiInit @ 0x0093_0d80 => extern "C" fn yanagi_init() -> u32 {
        gg::yanagi_init(&mut mem())
    }
    /// `Gui_ButtonSetTexture()`.
    Gui_ButtonSetTexture @ 0x0084_c9d0 => extern "C" fn button_set_texture() -> u32 {
        gg::button_set_texture(&mut mem())
    }
    /// `Gu_PacketArg_00853140(i)`.
    Gu_PacketArg_00853140 @ 0x0085_3140 => extern "C" fn packet_arg(i: u32) -> u32 {
        gg::packet_arg(&mut mem(), i)
    }
    /// `Gu_PacketArgInt_00853170(i)`.
    Gu_PacketArgInt_00853170 @ 0x0085_3170 => extern "C" fn packet_arg_int(i: u32) -> u32 {
        gg::packet_arg_int(&mut mem(), i)
    }
    /// `Gui_MemGraph()`.
    Gui_MemGraph @ 0x0085_4fd0 => extern "C" fn mem_graph() -> u32 {
        gg::mem_graph(&mut mem())
    }
    /// `Gu_SelectorFillFiles_0084d3a0(list, sel, dir)`.
    Gu_SelectorFillFiles_0084d3a0 @ 0x0084_d3a0 => extern "C" fn selector_fill_files(list: u32, sel: u32, dir: u32) {
        gg::selector_fill_files(&mut mem(), list, sel, dir);
    }
    // ---- batch 9 (ffx_pcui::m5u_g) ----
    /// `Gu_LinePrintf_008533b0(fmt, ...)`: the arguments after `fmt` by address.
    Gu_LinePrintf_008533b0 @ 0x0085_33b0 => extern "C" fn line_printf(fmt: u32; slots: u32) -> u32 {
        gg::line_printf(&mut mem(), fmt, slots + 4)
    }
    /// `Gu_LogPrintf_00853e40(fmt, ...)`: buffer `ebp - 0x404`.
    Gu_LogPrintf_00853e40 @ 0x0085_3e40 => extern "C" fn log_printf(fmt: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<0x101>::new();
        gg::log_printf(&mut mem(), fmt, slots + 4, f.at())
    }
    /// `Gui_SettingSaveLightFile_00853310()`.
    Gui_SettingSaveLightFile_00853310 @ 0x0085_3310 => extern "C" fn save_light_file() -> u32 {
        gg::save_light_file(&mut mem())
    }
    /// `Gu_ChrSelectorFill_00853dc0()`.
    Gu_ChrSelectorFill_00853dc0 @ 0x0085_3dc0 => extern "C" fn chr_selector_fill() -> u32 {
        gg::chr_selector_fill(&mut mem())
    }
    // ---- batch 10 (ffx_pcui::m5u_g) ----
    /// `Gu_WrapAngle_0084d330(x)` -> `ST0`.
    Gu_WrapAngle_0084d330 @ 0x0084_d330 => extern "C" fn wrap_angle_0084d330(x: u32) -> f32 {
        gg::wrap_angle(&mem(), x)
    }
    /// `Gu_WrapAngle_00853000(x)` -> `ST0`.
    Gu_WrapAngle_00853000 @ 0x0085_3000 => extern "C" fn wrap_angle_00853000(x: u32) -> f32 {
        gg::wrap_angle(&mem(), x)
    }
    /// `Gu_WrapAngle_00854200(x)` -> `ST0`.
    Gu_WrapAngle_00854200 @ 0x0085_4200 => extern "C" fn wrap_angle_00854200(x: u32) -> f32 {
        gg::wrap_angle(&mem(), x)
    }
    /// `Gu_HelpWindow_00855360(text)`.
    Gu_HelpWindow_00855360 @ 0x0085_5360 => extern "C" fn help_window(text: u32) -> u32 {
        gg::help_window(&mut mem(), text)
    }
    // ---- render-pcui batch 3 (ffx_pcui::m5u_h) ----
    /// `Gu_WindowControlSetText_thunk_009411f0(window, id, s)`.
    Gu_WindowControlSetText_thunk_009411f0 @ 0x0094_11f0 => extern "C" fn window_control_set_text_thunk(window: u32, id: u32, s: u32) {
        hh::forward(&mut mem(), hh::WINDOW_CONTROL_SET_TEXT, Conv::Cdecl, &[window, id, s]);
    }
    /// `LeaveCriticalSection_ecx_009453c0(cs)`.
    LeaveCriticalSection_ecx_009453c0 @ 0x0094_53c0 => extern "thiscall" fn leave_ecx(cs: u32) -> u32 {
        hh::leave(&mut mem(), cs)
    }
    /// `PtrArray_isSet_00947da0(this; i)`.
    PtrArray_isSet_00947da0 @ 0x0094_7da0 => extern "thiscall" fn is_set(this: u32, i: u32) -> u32 {
        hh::is_set(&mem(), this, i)
    }
    /// `PageManager_unlock_00947ea0()`.
    PageManager_unlock_00947ea0 @ 0x0094_7ea0 => extern "C" fn page_unlock() -> u32 {
        hh::page_unlock(&mut mem())
    }
    /// `Thread_create_00948c10(arg)`.
    Thread_create_00948c10 @ 0x0094_8c10 => extern "C" fn create_thread(arg: u32) -> u32 {
        hh::create_thread(&mut mem(), arg)
    }
    /// `PageManager_startThread_00947630()`.
    PageManager_startThread_00947630 @ 0x0094_7630 => extern "C" fn page_start_thread() -> u32 {
        hh::page_start_thread(&mut mem())
    }
    /// `List_walkToEnd_00948a80(this)`.
    List_walkToEnd_00948a80 @ 0x0094_8a80 => extern "thiscall" fn walk_to_end(this: u32) -> u32 {
        hh::walk_to_end(&mem(), this)
    }
    /// `Thread_entry_00948c30(f)` (stdcall).
    Thread_entry_00948c30 @ 0x0094_8c30 => extern "stdcall" fn thread_entry(f: u32) -> u32 {
        hh::thread_entry(&mut mem(), f)
    }
    /// `PVideoPlaybackWin32_memcpy_00a26fb0(dst, src, n)`.
    PVideoPlaybackWin32_memcpy_00a26fb0 @ 0x00a2_6fb0 => extern "C" fn video_memcpy(dst: u32, src: u32, n: u32) -> u32 {
        hh::crt3(&mut mem(), hh::MEMCPY_THUNK, dst, src, n)
    }
    /// `PVideoPlaybackWin32_memmove_00a26fc0(dst, src, n)`.
    PVideoPlaybackWin32_memmove_00a26fc0 @ 0x00a2_6fc0 => extern "C" fn video_memmove(dst: u32, src: u32, n: u32) -> u32 {
        hh::crt3(&mut mem(), hh::MEMMOVE_IAT, dst, src, n)
    }
    /// `PVideoPlaybackWin32_memset_00a26fd0(dst, c, n)`.
    PVideoPlaybackWin32_memset_00a26fd0 @ 0x00a2_6fd0 => extern "C" fn video_memset(dst: u32, c: u32, n: u32) -> u32 {
        hh::crt3(&mut mem(), hh::MEMSET_THUNK, dst, c, n)
    }
    /// `IggyPlayer_replay_00a3db30(this)`.
    IggyPlayer_replay_00a3db30 @ 0x00a3_db30 => extern "thiscall" fn iggy_replay(this: u32) -> u32 {
        hh::iggy_replay(&mut mem(), this)
    }
    /// `IggyMenu_replay(menu)`.
    IggyMenu_replay @ 0x00a2_8ee0 => extern "thiscall" fn iggy_menu_replay(this: u32) -> u32 {
        hh::iggy_menu_replay(&mut mem(), this)
    }
    /// `GDraw_releaseHandle_00a362c0(handle)`.
    GDraw_releaseHandle_00a362c0 @ 0x00a3_62c0 => extern "C" fn gdraw_release_handle(handle: u32) -> u32 {
        hh::gdraw_release_handle(&mut mem(), handle)
    }
    /// `GDraw_callbackIf_00a398e0(a, key, x)` (stdcall).
    GDraw_callbackIf_00a398e0 @ 0x00a3_98e0 => extern "stdcall" fn callback_if(a: u32, key: u32, x: u32) -> u32 {
        hh::callback_if(&mut mem(), a, key, x)
    }
    /// `GDraw_releaseThis_00a3d3c0(this)`.
    GDraw_releaseThis_00a3d3c0 @ 0x00a3_d3c0 => extern "thiscall" fn release_this(this: u32) -> u32 {
        hh::release_this(&mut mem(), this)
    }
    /// `GDraw_teardown_thunk_00a3d4d0(this)`.
    GDraw_teardown_thunk_00a3d4d0 @ 0x00a3_d4d0 => extern "thiscall" fn gdraw_teardown_thunk(this: u32) -> u32 {
        hh::forward(&mut mem(), hh::GDRAW_TEARDOWN, Conv::Thiscall(this), &[])
    }
    /// `Callback_invoke_00a3d6d0(p)`.
    Callback_invoke_00a3d6d0 @ 0x00a3_d6d0 => extern "C" fn invoke(p: u32) -> u32 {
        hh::invoke(&mut mem(), p)
    }
    /// `IggyPlayerHolder_destroy_00a3d780(this; x)` (`x` unread).
    IggyPlayerHolder_destroy_00a3d780 @ 0x00a3_d780 => extern "thiscall" fn iggy_destroy(this: u32, _x: u32) -> u32 {
        hh::iggy_destroy(&mut mem(), this)
    }
    /// `AddrOfGlobal01a858cc_00a3d7c0()`.
    AddrOfGlobal01a858cc_00a3d7c0 @ 0x00a3_d7c0 => extern "C" fn gdraw_block() -> u32 {
        hh::gdraw_block(&mem())
    }
    // ---- render-pcui batch 4 (ffx_pcui::m5u_i) ----
    /// `PArray_release_00a25e50(array)`.
    PArray_release_00a25e50 @ 0x00a2_5e50 => extern "thiscall" fn array_release_00a25e50(this: u32) {
        ii::array_release(&mut mem(), this, false);
    }
    /// `PArray_releaseChecked_00a25e80(array)`.
    PArray_releaseChecked_00a25e80 @ 0x00a2_5e80 => extern "thiscall" fn array_release_00a25e80(this: u32) {
        ii::array_release(&mut mem(), this, true);
    }
    /// `PArray_releaseChecked_00a3d3d0(array)`.
    PArray_releaseChecked_00a3d3d0 @ 0x00a3_d3d0 => extern "thiscall" fn array_release_00a3d3d0(this: u32) {
        ii::array_release(&mut mem(), this, true);
    }
    /// `GDraw_rectCall_00a36040(rect, a, b)` (argument slots).
    GDraw_rectCall_00a36040 @ 0x00a3_6040 => extern "C" fn rect_call(rect: u32, a: u32, b: u32; slots: u32) -> u32 {
        ii::rect_call(&mut mem(), slots, rect, a, b)
    }
    /// `IggyPlayerHolder_queueDestroy_00a3d720(this; renderer)` (the block pointer's frame).
    IggyPlayerHolder_queueDestroy_00a3d720 @ 0x00a3_d720 => extern "thiscall" fn queue_destroy(this: u32, renderer: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        ii::queue_destroy(&mut mem(), f.at(), this, renderer)
    }
    /// `IggyPlayer_QueueDraw_00a3d9b0(this; renderer)` (the block pointer's frame).
    IggyPlayer_QueueDraw_00a3d9b0 @ 0x00a3_d9b0 => extern "thiscall" fn queue_draw(this: u32, renderer: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        ii::queue_draw(&mut mem(), f.at(), this, renderer)
    }
    /// `GDraw_releaseIf_00a398a0(node, key, x)` (stdcall).
    GDraw_releaseIf_00a398a0 @ 0x00a3_98a0 => extern "stdcall" fn release_if(node: u32, key: u32, x: u32) -> u32 {
        ii::release_if(&mut mem(), node, key, x)
    }
    /// `PageHeap_fillBuckets_00946100(heap)`.
    PageHeap_fillBuckets_00946100 @ 0x0094_6100 => extern "thiscall" fn fill_buckets(this: u32) {
        ii::fill_buckets(&mut mem(), this);
    }
    /// `PageList_free_009470f0(list)`.
    PageList_free_009470f0 @ 0x0094_70f0 => extern "thiscall" fn free_pages(this: u32) {
        ii::free_pages(&mut mem(), this);
    }
    /// `PageBits_ctor_00946520(this; owner, n)`.
    PageBits_ctor_00946520 @ 0x0094_6520 => extern "thiscall" fn page_bits_ctor(this: u32, owner: u32, n: u32) -> u32 {
        ii::page_bits_ctor(&mut mem(), this, owner, n)
    }
    /// `PVideoPlaybackWin32_CopyPlane_00a26fe0(dst, dpitch, src, spitch, w, rows)`.
    PVideoPlaybackWin32_CopyPlane_00a26fe0 @ 0x00a2_6fe0 => extern "C" fn copy_plane(dst: u32, dpitch: u32, src: u32, spitch: u32, w: u32, rows: u32) {
        ii::copy_plane(&mut mem(), dst, dpitch, src, spitch, w, rows);
    }
    /// `GDraw_freeResources_00a36300(stats)`.
    GDraw_freeResources_00a36300 @ 0x00a3_6300 => extern "C" fn free_resources(stats: u32) {
        ii::free_resources(&mut mem(), stats);
    }
    /// `GDraw_releaseSlots_00a3cfc0()`.
    GDraw_releaseSlots_00a3cfc0 @ 0x00a3_cfc0 => extern "C" fn release_slots() {
        ii::release_slots(&mut mem());
    }
    /// `PageHeap_lockAll_00946c40(heap)`.
    PageHeap_lockAll_00946c40 @ 0x0094_6c40 => extern "thiscall" fn lock_all(this: u32) -> u32 {
        ii::lock_all(&mut mem(), this, ii::ENTER_CRITICAL_SECTION_IAT)
    }
    /// `PageHeap_unlockAll_00946cc0(heap)`.
    PageHeap_unlockAll_00946cc0 @ 0x0094_6cc0 => extern "thiscall" fn unlock_all(this: u32) -> u32 {
        ii::lock_all(&mut mem(), this, ii::LEAVE_CRITICAL_SECTION_IAT)
    }
    /// `PageBucket_init_00948900(this; size, align, heap)`.
    PageBucket_init_00948900 @ 0x0094_8900 => extern "thiscall" fn bucket_init(this: u32, size: u32, align: u32, heap: u32) -> u32 {
        ii::bucket_init(&mut mem(), this, size, align, heap)
    }
    /// `PageHash_find_009455d0(table; key, out)` -> `al`.
    PageHash_find_009455d0 @ 0x0094_55d0 => extern "thiscall" fn hash_find(this: u32, key: u32, out: u32) -> bool {
        ii::hash_find(&mut mem(), this, key, out)
    }
    /// `IggyValue_readClass_00a25860(reader; v, class, out, cursor)`.
    IggyValue_readClass_00a25860 @ 0x00a2_5860 => extern "thiscall" fn value_read_00a25860(this: u32, v: u32, class: u32, out: u32, cursor: u32) -> u32 {
        ii::value_read(&mut mem(), this, v, class, out, cursor, true)
    }
    /// `IggyValue_read_00a258c0(reader; v, class, out, cursor)`.
    IggyValue_read_00a258c0 @ 0x00a2_58c0 => extern "thiscall" fn value_read_00a258c0(this: u32, v: u32, class: u32, out: u32, cursor: u32) -> u32 {
        ii::value_read(&mut mem(), this, v, class, out, cursor, false)
    }
    // ---- render-pcui batch 5 (ffx_pcui::m5u_j) ----
    /// `Gu_ChrSelectorProc_00853d40(window, message)`.
    Gu_ChrSelectorProc_00853d40 @ 0x0085_3d40 => extern "C" fn chr_selector_proc(window: u32, message: u32) -> u32 {
        jj::chr_selector_proc(&mut mem(), window, message)
    }
    /// `PUtility_scalarDeletingDtor_00a3e3e0(this; flags)`.
    PUtility_scalarDeletingDtor_00a3e3e0 @ 0x00a3_e3e0 => extern "thiscall" fn utility_delete(this: u32, flags: u32) -> u32 {
        jj::utility_delete(&mut mem(), this, flags)
    }
    /// `PageBucket_lockedCall_00948820(this)`.
    PageBucket_lockedCall_00948820 @ 0x0094_8820 => extern "thiscall" fn locked_call(this: u32) -> u32 {
        jj::locked_call(&mut mem(), this)
    }
    /// `PageLock_ctor_009488a0(lock)`.
    PageLock_ctor_009488a0 @ 0x0094_88a0 => extern "thiscall" fn lock_ctor(this: u32) -> u32 {
        jj::lock_ctor(&mut mem(), this)
    }
    /// `PageStats_update_00947cc0()`.
    PageStats_update_00947cc0 @ 0x0094_7cc0 => extern "C" fn stats_update() -> u32 {
        jj::stats_update(&mut mem())
    }
    // ---- render-pcui batch 6 (ffx_pcui::m5u_k) ----
    /// `PageHeap_initBuckets_00946060(heap)`.
    PageHeap_initBuckets_00946060 @ 0x0094_6060 => extern "thiscall" fn init_buckets(this: u32) -> u32 {
        kk::init_buckets(&mut mem(), this)
    }
    /// `PageHash_ctor_00945630(this; n, owner)`.
    PageHash_ctor_00945630 @ 0x0094_5630 => extern "thiscall" fn hash_ctor(this: u32, n: u32, owner: u32) -> u32 {
        kk::hash_ctor(&mut mem(), this, n, owner)
    }
    /// `PageHash_release_009474f0(this)`.
    PageHash_release_009474f0 @ 0x0094_74f0 => extern "thiscall" fn hash_release(this: u32) {
        kk::hash_release(&mut mem(), this);
    }

    // ---- round 6 (render-pcui4, ffx_pcui::m5u_l) ----
    /// `CrashDump_readMemory_00689370(process, base, unused, buffer, size,
    /// read)` (the count cell).
    CrashDump_readMemory_00689370 @ 0x0068_9370 => extern "stdcall" fn crash_read_memory(process: u32, base: u32, unused: u32, buffer: u32, size: u32, read: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        ll::crash_read_memory(&mut mem(), f.at(), [process, base, unused, buffer, size, read])
    }
    /// `VideoTexture_release_006e7d30(this)`.
    VideoTexture_release_006e7d30 @ 0x006e_7d30 => extern "thiscall" fn video_texture_release(this: u32) -> u32 {
        ll::video_texture_release(&mut mem(), this)
    }
    /// `FFEscMenu_buildResolutionList(this)` (the render scope lock).
    FFEscMenu_buildResolutionList @ 0x006f_edf0 => extern "thiscall" fn build_resolution_list(this: u32) {
        let mut f = StackFrame::<2>::new();
        ll::build_resolution_list(&mut mem(), f.at(), this);
    }
    /// `FFEscMenu_applyVideoSettings(this; s)`.
    FFEscMenu_applyVideoSettings @ 0x006f_f290 => extern "thiscall" fn apply_video_settings(this: u32, s: u32) -> u32 {
        ll::apply_video_settings(&mut mem(), this, s)
    }
    /// `FFEscMenu_keyBindingInput_00701190(this; done)` (the Iggy value and
    /// the string loader).
    FFEscMenu_keyBindingInput_00701190 @ 0x0070_1190 => extern "thiscall" fn key_binding_input(this: u32, done: u32) {
        let mut f = StackFrame::<0x48>::new();
        ll::key_binding_input(&mut mem(), f.at(), this, done);
    }

    /// `FFEscMenu_loadKeyBindings_00701680(menu, args)` (the value
    /// reference and text buffer).
    FFEscMenu_loadKeyBindings_00701680 @ 0x0070_1680 => extern "C" fn load_key_bindings(menu: u32, args: u32) -> u32 {
        let mut f = StackFrame::<0x24>::new();
        ll::load_key_bindings(&mut mem(), f.at(), menu, args)
    }
    /// `FFEscMenu_padBindingInput_007019b0(this; done)` (the Iggy value and
    /// the string loader).
    FFEscMenu_padBindingInput_007019b0 @ 0x0070_19b0 => extern "thiscall" fn pad_binding_input(this: u32, done: u32) {
        let mut f = StackFrame::<0x48>::new();
        ll::pad_binding_input(&mut mem(), f.at(), this, done);
    }
    /// `FFEscMenu_loadPadBindings_00701e60(menu, args)` (the value
    /// reference and text buffer).
    FFEscMenu_loadPadBindings_00701e60 @ 0x0070_1e60 => extern "C" fn load_pad_bindings(menu: u32, args: u32) -> u32 {
        let mut f = StackFrame::<0x21>::new();
        ll::load_pad_bindings(&mut mem(), f.at(), menu, args)
    }

    /// `syncStrPc(play, state)` (the status cell).
    syncStrPc @ 0x0093_db20 => extern "C" fn sync_str_pc(play: u32, state: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        ll::sync_str_pc(&mut mem(), f.at(), play, state)
    }
    /// `GuPrintChar_00849070(c)`.
    GuPrintChar_00849070 @ 0x0084_9070 => extern "C" fn gu_print_char(c: u32) -> u32 {
        ll::gu_print_char(&mut mem(), c)
    }
    /// `SgClock_resetState_0084ca00()`.
    SgClock_resetState_0084ca00 @ 0x0084_ca00 => extern "C" fn sg_clock_reset_state() -> u32 {
        ll::sg_clock_reset_state(&mut mem())
    }
    /// `DebugScript_readLine_008530d0()`.
    DebugScript_readLine_008530d0 @ 0x0085_30d0 => extern "C" fn debug_read_line() -> u32 {
        ll::debug_read_line(&mut mem())
    }
    /// `DebugScript_tokenizeLine_00853070()`.
    DebugScript_tokenizeLine_00853070 @ 0x0085_3070 => extern "C" fn debug_tokenize_line() -> u32 {
        ll::debug_tokenize_line(&mut mem())
    }

    /// `Gui_MasakiInit(which)`.
    Gui_MasakiInit @ 0x007c_86e0 => extern "C" fn gui_masaki_init(which: u32) -> u32 {
        ll::gui_masaki_init(&mut mem(), which)
    }
    /// `DebugScript_readLights_0084d790(n)`.
    DebugScript_readLights_0084d790 @ 0x0084_d790 => extern "C" fn debug_read_lights(n: u32) {
        ll::debug_read_lights(&mut mem(), n);
    }
    /// `DebugScript_interpret_00853210()`.
    DebugScript_interpret_00853210 @ 0x0085_3210 => extern "C" fn debug_interpret() -> u32 {
        ll::debug_interpret(&mut mem())
    }
    /// `DebugScript_loadLights_008531a0()`.
    DebugScript_loadLights_008531a0 @ 0x0085_31a0 => extern "C" fn debug_load_lights() -> u32 {
        ll::debug_load_lights(&mut mem())
    }

    /// `Gui_ProgAvarage()` (the caller's `eax` kept: callers don't read it).
    Gui_ProgAvarage @ 0x0085_5430 => extern "C" fn gui_prog_average() {
        let _ = ll::gui_prog_average(&mut mem());
    }
    /// `ChrView_updateControls_0084cc30(window, chr)`.
    ChrView_updateControls_0084cc30 @ 0x0084_cc30 => extern "C" fn chrview_update_controls(window: u32, chr: u32) -> u32 {
        ll::chrview_update_controls(&mut mem(), window, chr)
    }
    /// `ChrMenu_updateLightTexts_0084d540(window)`.
    ChrMenu_updateLightTexts_0084d540 @ 0x0084_d540 => extern "C" fn chrmenu_update_light_texts(window: u32) -> u32 {
        ll::chrmenu_update_light_texts(&mut mem(), window)
    }

    /// `ChrMenu_fillSelector_0084ce10(selector)`.
    ChrMenu_fillSelector_0084ce10 @ 0x0084_ce10 => extern "C" fn chrmenu_fill_selector(selector: u32) -> u32 {
        ll::chrmenu_fill_selector(&mut mem(), selector)
    }
    /// `create_chrview(chr)`.
    create_chrview @ 0x0084_cea0 => extern "C" fn create_chrview(chr: u32) -> u32 {
        ll::create_chrview(&mut mem(), chr)
    }
    /// `ChrMenu_listMotions_0084d3f0(sel)` (the command buffer).
    ChrMenu_listMotions_0084d3f0 @ 0x0084_d3f0 => extern "C" fn chrmenu_list_motions(sel: u32) -> u32 {
        let mut f = StackFrame::<0x20>::new();
        ll::chrmenu_list_motions(&mut mem(), f.at(), sel)
    }
    /// `ChrMenu_listCharacters_0084d4a0(sel)` (the command buffer).
    ChrMenu_listCharacters_0084d4a0 @ 0x0084_d4a0 => extern "C" fn chrmenu_list_characters(sel: u32) -> u32 {
        let mut f = StackFrame::<0x20>::new();
        ll::chrmenu_list_characters(&mut mem(), f.at(), sel)
    }
    /// `Gui_SettingSaveLight()`.
    Gui_SettingSaveLight @ 0x0084_d8a0 => extern "C" fn gui_setting_save_light() -> u32 {
        ll::gui_setting_save_light(&mut mem())
    }

    /// `SpecWindowProc_0084ccc0(window, msg, arg)` (the number buffer).
    SpecWindowProc_0084ccc0 @ 0x0084_ccc0 => extern "C" fn spec_window_proc(window: u32, msg: u32, arg: u32) -> u32 {
        let mut f = StackFrame::<5>::new();
        ll::spec_window_proc(&mut mem(), f.at(), window, msg, arg)
    }
    /// `gui_chr_GWM_SELECTOR(window, msg, sel)`.
    gui_chr_GWM_SELECTOR @ 0x0084_d270 => extern "C" fn gui_chr_selector(window: u32, msg: u32, sel: u32) -> u32 {
        ll::gui_chr_selector(&mut mem(), window, msg, sel)
    }
    /// `ChrMenu_loadSelected_0084d5e0(sel)` (the id cell and name buffer).
    ChrMenu_loadSelected_0084d5e0 @ 0x0084_d5e0 => extern "C" fn chrmenu_load_selected(sel: u32) -> u32 {
        let mut f = StackFrame::<0x11>::new();
        ll::chrmenu_load_selected(&mut mem(), f.at(), sel)
    }

    /// `Gui_ChrMenuInit(window, info)`.
    Gui_ChrMenuInit @ 0x0084_ca30 => extern "C" fn gui_chr_menu_init(window: u32, info: u32) -> u32 {
        ll::gui_chr_menu_init(&mut mem(), window, info)
    }
    /// `ChrMenu_editLight_0084d940(values, window, msg, button)` (the
    /// window word is never read).
    ChrMenu_editLight_0084d940 @ 0x0084_d940 => extern "C" fn chrmenu_edit_light(values: u32, _window: u32, msg: u32, button: u32) -> u32 {
        ll::chrmenu_edit_light(&mut mem(), values, msg, button)
    }
    /// `LightWindowProc_0084e100(window, msg, button)` (the black colour
    /// and the number buffer).
    LightWindowProc_0084e100 @ 0x0084_e100 => extern "C" fn light_window_proc(window: u32, msg: u32, button: u32) -> u32 {
        let mut f = StackFrame::<9>::new();
        ll::light_window_proc(&mut mem(), f.at(), window, msg, button)
    }

    /// `FUN_0084dac0(window, msg, button)` (the number buffer).
    DirLightWindowProc_0084dac0 @ 0x0084_dac0 => extern "C" fn dir_light_window_proc(window: u32, msg: u32, button: u32) -> u32 {
        let mut f = StackFrame::<5>::new();
        ll::dir_light_window_proc(&mut mem(), f.at(), window, msg, button)
    }

    /// `FUN_0084e3c0(group, load)` (the colour/direction vector).
    LightGroup_loadSave_0084e3c0 @ 0x0084_e3c0 => extern "C" fn light_group_load_save(group: u32, load: u32) -> u32 {
        let mut f = StackFrame::<4>::new();
        ll::light_group_load_save(&mut mem(), f.at(), group, load)
    }

    /// `FUN_00852a30(window, msg, button)`.
    SpecEditWindowProc_00852a30 @ 0x0085_2a30 => extern "C" fn spec_edit_window_proc(window: u32, msg: u32, button: u32) -> u32 {
        ll::spec_edit_window_proc(&mut mem(), window, msg, button)
    }
    /// `FUN_00853420(window)`.
    CaptureWindow_update_00853420 @ 0x0085_3420 => extern "C" fn capture_window_update(window: u32) -> u32 {
        ll::capture_window_update(&mut mem(), window)
    }

    /// `SpriteFrameRegion_Load(window, msg, button)` (the capture window).
    SpriteFrameRegion_Load @ 0x0085_3520 => extern "C" fn capture_window_proc(window: u32, msg: u32, button: u32) -> u32 {
        ll::capture_window_proc(&mut mem(), window, msg, button)
    }

    /// `FUN_008537d0(window, msg)` (the text buffer).
    ChrInfoWindowProc_008537d0 @ 0x0085_37d0 => extern "C" fn chr_info_window_proc(window: u32, msg: u32) -> u32 {
        let mut f = StackFrame::<64>::new();
        ll::chr_info_window_proc(&mut mem(), f.at(), window, msg)
    }

    /// `FUN_00853ec0(window, msg)` (the text buffer).
    ClockWindowProc_00853ec0 @ 0x0085_3ec0 => extern "C" fn clock_window_proc(window: u32, msg: u32) -> u32 {
        let mut f = StackFrame::<8>::new();
        ll::clock_window_proc(&mut mem(), f.at(), window, msg)
    }

    /// `FUN_00854270(window, msg, button)` (the text buffer).
    AccBufWindowProc_00854270 @ 0x0085_4270 => extern "C" fn acc_buf_window_proc(window: u32, msg: u32, button: u32) -> u32 {
        let mut f = StackFrame::<5>::new();
        ll::acc_buf_window_proc(&mut mem(), f.at(), window, msg, button)
    }
    /// `FUN_00854420(window, msg, button)` (the text buffer).
    BlurWindowProc_00854420 @ 0x0085_4420 => extern "C" fn blur_window_proc(window: u32, msg: u32, button: u32) -> u32 {
        let mut f = StackFrame::<5>::new();
        ll::blur_window_proc(&mut mem(), f.at(), window, msg, button)
    }

    /// `FUN_00854590(window, msg, button)` (the text buffer).
    FlashWindowProc_00854590 @ 0x0085_4590 => extern "C" fn flash_window_proc(window: u32, msg: u32, button: u32) -> u32 {
        let mut f = StackFrame::<5>::new();
        ll::flash_window_proc(&mut mem(), f.at(), window, msg, button)
    }

    /// `FUN_00855290(window, msg)`.
    HelpWindowProc_00855290 @ 0x0085_5290 => extern "C" fn help_window_proc(window: u32, msg: u32) -> u32 {
        ll::help_window_proc(&mut mem(), window, msg)
    }
    /// `FUN_008554b0(window, msg)`.
    TipWindowProc_008554b0 @ 0x0085_54b0 => extern "C" fn tip_window_proc(window: u32, msg: u32) -> u32 {
        ll::tip_window_proc(&mut mem(), window, msg)
    }

    /// `FUN_00855020(window, msg)` (the rectangle, locals, usage words
    /// and help text).
    MemGraphWindowProc_00855020 @ 0x0085_5020 => extern "C" fn mem_graph_window_proc(window: u32, msg: u32) -> u32 {
        let mut f = StackFrame::<31>::new();
        ll::mem_graph_window_proc(&mut mem(), f.at(), window, msg)
    }

    /// `FUN_009401c0()` (the picked indices and the list pointers).
    NameList_split_009401c0 @ 0x0094_01c0 => extern "C" fn name_list_split() -> u32 {
        let mut f = StackFrame::<4>::new();
        ll::name_list_split(&mut mem(), f.at())
    }
    /// `FUN_00940580()` (the sums and the two positions).
    NameList_viewCentre_00940580 @ 0x0094_0580 => extern "C" fn name_list_view_centre() -> u32 {
        let mut f = StackFrame::<12>::new();
        ll::name_list_view_centre(&mut mem(), f.at())
    }
    /// `FUN_0093fd60(v, out)` (the matrix).
    DebugView_project_0093fd60 @ 0x0093_fd60 => extern "C" fn debug_project(v: u32, out: u32) -> u32 {
        let mut f = StackFrame::<16>::new();
        ll::debug_project(&mut mem(), f.at(), v, out)
    }

    /// `FUN_009410e0()`.
    NameList_dropPicked_009410e0 @ 0x0094_10e0 => extern "C" fn name_list_drop_picked() -> u32 {
        ll::name_list_drop_picked(&mut mem())
    }
    /// `FUN_00941460()` (the index, cursor, scratch and count).
    NameList_load_00941460 @ 0x0094_1460 => extern "C" fn name_list_load() {
        let mut f = StackFrame::<4>::new();
        ll::name_list_load(&mut mem(), f.at());
    }

    /// `FUN_00946f50(this)`: the page manager's constructor.
    PageManager_ctor_00946f50 @ 0x0094_6f50 => extern "thiscall" fn page_manager_ctor(this: u32) -> u32 {
        ll::page_manager_ctor(&mut mem(), this)
    }
    /// `FUN_00946b20(this; index, arg)`.
    PageManager_addBucket_00946b20 @ 0x0094_6b20 => extern "thiscall" fn page_bucket_add(this: u32, index: u32, arg: u32) {
        ll::page_bucket_add(&mut mem(), this, index, arg);
    }
    /// `FUN_00948750(this; bits, cs)`.
    Bitmap_init_00948750 @ 0x0094_8750 => extern "thiscall" fn bitmap_init(this: u32, bits: u32, cs: u32) -> u32 {
        ll::bitmap_init(&mut mem(), this, bits, cs)
    }

    /// `FUN_00946840(this)`.
    BucketTable_walk_00946840 @ 0x0094_6840 => extern "thiscall" fn bucket_table_walk(this: u32) -> u32 {
        ll::bucket_table_walk(&mut mem(), this)
    }
    /// `FUN_00945b80(this)`.
    PageManager_walk_00945b80 @ 0x0094_5b80 => extern "thiscall" fn page_manager_walk(this: u32) -> u32 {
        ll::page_manager_walk(&mut mem(), this)
    }
    /// `FUN_00946660(this)`: the bucket table's destructor.
    BucketTable_dtor_00946660 @ 0x0094_6660 => extern "thiscall" fn bucket_table_dtor(this: u32) -> u32 {
        ll::bucket_table_dtor(&mut mem(), this)
    }
    /// `FUN_009453d0(this; key)`.
    HashTable_remove_009453d0 @ 0x0094_53d0 => extern "thiscall" fn hash_remove(this: u32, key: u32) -> u32 {
        ll::hash_remove(&mut mem(), this, key)
    }

    /// `FUN_00a26480(this; buffer, value)`.
    Array_pushInto_00a26480 @ 0x00a2_6480 => extern "thiscall" fn array_push_into(this: u32, buffer: u32, value: u32) -> u32 {
        ll::array_push_into(&mut mem(), this, buffer, value)
    }
    /// `FUN_00a35a70(size, bind)` (the buffer cell and description).
    GDraw_createDynamicBuffer_00a35a70 @ 0x00a3_5a70 => extern "C" fn gdraw_create_dynamic_buffer(size: u32, bind: u32) -> u32 {
        let mut f = StackFrame::<7>::new();
        ll::gdraw_create_dynamic_buffer(&mut mem(), f.at(), size, bind)
    }
    /// `FUN_00a35b40(shader, code)`.
    GDraw_createPixelShader_00a35b40 @ 0x00a3_5b40 => extern "C" fn gdraw_create_pixel_shader(shader: u32, code: u32) -> u32 {
        ll::gdraw_create_pixel_shader(&mut mem(), shader, code)
    }
    /// `VbfMkvReader_Read(this; offset, size, buffer)`.
    VbfMkvReader_Read @ 0x00a3_e2b0 => extern "thiscall" fn mkv_reader_read(this: u32, lo: u32, hi: u32, size: u32, buffer: u32) -> u32 {
        ll::mkv_reader_read(&mut mem(), this, lo, hi, size, buffer)
    }

    /// `FUN_00a3ce00(x, y, w, h)` (the viewport and rectangle).
    GDraw_setViewport_00a3ce00 @ 0x00a3_ce00 => extern "C" fn gdraw_set_viewport(x: u32, y: u32, w: u32, ht: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        ll::gdraw_set_viewport(&mut mem(), f.at(), x, y, w, ht)
    }
    /// `FUN_00a3d310(this)`.
    GDrawRecord_ctor_00a3d310 @ 0x00a3_d310 => extern "thiscall" fn gdraw_record_ctor(this: u32) -> u32 {
        ll::gdraw_record_ctor(&mut mem(), this)
    }
    /// `FUN_00a3d410(this)`.
    GDrawRecord_release_00a3d410 @ 0x00a3_d410 => extern "thiscall" fn gdraw_record_release(this: u32) {
        ll::gdraw_record_release(&mut mem(), this);
    }

    /// `FUN_00a3d5f0(renderer, audio, arg)`.
    GDrawContext_acquire_00a3d5f0 @ 0x00a3_d5f0 => extern "C" fn gdraw_context_acquire(renderer: u32, audio: u32, arg: u32) -> u32 {
        ll::gdraw_context_acquire(&mut mem(), renderer, audio, arg)
    }
    /// `FUN_00a3d7f0(this; list)` (the event result).
    Iggy_dispatchEvents_00a3d7f0 @ 0x00a3_d7f0 => extern "thiscall" fn iggy_dispatch_events(this: u32, list: u32) -> u32 {
        let mut f = StackFrame::<3>::new();
        ll::iggy_dispatch_events(&mut mem(), f.at(), this, list)
    }
    /// `FUN_00a3d900(this; a, b, c, d, data, size, _)`.
    IggyPlayer_create_00a3d900 @ 0x00a3_d900 => extern "thiscall" fn iggy_player_create(this: u32, a: u32, b: u32, c: u32, d: u32, data: u32, size: u32, _unused: u32) -> u32 {
        ll::iggy_player_create(&mut mem(), this, [a, b, c, d, data, size])
    }

    /// `FUN_009402c0()`.
    CameraPath_spline_009402c0 @ 0x0094_02c0 => extern "C" fn camera_path_spline() -> u32 {
        ll::camera_path_spline(&mut mem())
    }
    /// `FUN_00940380()` (the position and the aim).
    CameraPath_follow_00940380 @ 0x0094_0380 => extern "C" fn camera_path_follow() -> u32 {
        let mut f = StackFrame::<8>::new();
        ll::camera_path_follow(&mut mem(), f.at(), false)
    }
    /// `FUN_00940480()` (the position and the aim).
    CameraPath_follow3_00940480 @ 0x0094_0480 => extern "C" fn camera_path_follow3() -> u32 {
        let mut f = StackFrame::<8>::new();
        ll::camera_path_follow(&mut mem(), f.at(), true)
    }

    /// `AbmapFindNextConnectingNode(from, to, link)` (`ax`).
    AbmapFindNextConnectingNode @ 0x00a5_6e00 => extern "C" fn abmap_find_next(from: u32, to: u32, link: u32) -> u32 {
        ll::abmap_find_next(&mut mem(), from, to, link)
    }
    /// `FUN_00a36490(a, texture, filter, d, rect, e)` (the clipped
    /// rectangle and texel sizes).
    GDraw_filterPasses_00a36490 @ 0x00a3_6490 => extern "C" fn gdraw_filter_passes(a: u32, tex: u32, filter: u32, d: u32, rect: u32, e: u32) {
        let mut f = StackFrame::<6>::new();
        ll::gdraw_filter_passes(&mut mem(), f.at(), [a, tex, filter, d, rect, e]);
    }

    /// `FUN_00a3d9f0(this; holder)` (the targets and clear colour).
    IggyPlayer_tickDraw_00a3d9f0 @ 0x00a3_d9f0 => extern "thiscall" fn iggy_player_tick_draw(this: u32, holder: u32) {
        let mut f = StackFrame::<6>::new();
        ll::iggy_player_tick_draw(&mut mem(), f.at(), this, holder);
    }

    /// `FUN_009457b0(this; config)`: the page heap's constructor.
    PageHeap_ctor_009457b0 @ 0x0094_57b0 => extern "thiscall" fn page_heap_ctor(this: u32, config: u32) -> u32 {
        ll::page_heap_ctor(&mut mem(), this, config)
    }

    /// `FUN_00945350(this)`: `EnterCriticalSection(this)`.
    Lock_enter_00945350 @ 0x0094_5350 => extern "thiscall" fn lock_enter(this: u32) -> u32 {
        ll::lock_enter(&mut mem(), this)
    }
    // ---- round 6 (left6) batch 2 (ffx_pcui::l6_pcui) ----
    /// `(window, id, state; slots)`: the character page's buttons (writes
    /// its `id` and `state` slots before the toggle buttons' tail jump).
    ChrMenu_buttons_0084cf60 @ 0x0084_cf60 => extern "C" fn chrmenu_buttons(window: u32, id: u32, state: u32; slots: u32) -> u32 {
        l6u::chr_buttons(&mut mem(), slots, window, id, state)
    }
    /// `(window, msg, param; slots)` -> 0: the performance meter window's
    /// procedure (its rectangle in a frame; the bar's width in its `msg`
    /// slot).
    Sg_PerfMeterProc_00855570 @ 0x0085_5570 => extern "C" fn sg_perf_meter_proc(window: u32, msg: u32, param: u32; slots: u32) -> u32 {
        let mut frame = StackFrame::<2>::new();
        let ebp = frame.at() + 8;
        l6u2::meter_proc(&mut mem(), ebp, slots, window, msg, param)
    }
    // ---- round 6 (left6) batch 3 (ffx_pcui::l6_pcui3) ----
    /// `(api, target, obj, a, b, src, dst, c)`: a filter pass over a
    /// rectangle (its coordinates in a frame the callbacks read).
    GDraw_filterBoxPass_00a37010 @ 0x00a3_7010 => extern "C" fn gdraw_filter_box_pass(api: u32, target: u32, obj: u32, a: u32, b: u32, src: u32, dst: u32, c: u32) -> u32 {
        let mut frame = StackFrame::<{ (l6u3::BOX_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + l6u3::BOX_FRAME;
        l6u3::box_pass(&mut mem(), ebp, [api, target, obj, a, b, src, dst, c])
    }
    /// `(api, sub, obj, a, b, src, vertical, n, w, h, dst, c)`: a one-axis
    /// downsample pass (its rectangle and coordinates in a frame).
    GDraw_filterAxisPass_00a37170 @ 0x00a3_7170 => extern "C" fn gdraw_filter_axis_pass(api: u32, sub: u32, obj: u32, a: u32, b: u32, src: u32, vertical: u32, n: u32, w: u32, h: u32, dst: u32, c: u32) -> u32 {
        let mut frame = StackFrame::<{ (l6u3::AXIS_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + l6u3::AXIS_FRAME;
        l6u3::axis_pass(&mut mem(), ebp, [api, sub, obj, a, b, src, vertical, n, w, h, dst, c])
    }
    /// `(px, py, cam, point, offset)`: the debug camera's mouse pick (its
    /// two points and the ray in a frame).
    CamDebug_ScreenPick_00940f30 @ 0x0094_0f30 => extern "C" fn camdebug_screen_pick(px: u32, py: u32, cam: u32, point: u32, offset: u32) -> u32 {
        let mut frame = StackFrame::<{ (l6u3::PICK_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + l6u3::PICK_FRAME;
        l6u3::screen_pick(&mut mem(), ebp, [px, py, cam, point, offset])
    }
    /// `(src, stats, x0, y0, x1, y1)`: a textured quad (the mapped data,
    /// stride and offset in a frame).
    GDraw_drawQuad_00a36090 @ 0x00a3_6090 => extern "C" fn gdraw_draw_quad(src: u32, stats: u32, x0: u32, y0: u32, x1: u32, y1: u32) -> u32 {
        let mut frame = StackFrame::<{ (l6u3::QUAD_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + l6u3::QUAD_FRAME;
        l6u3::quad(&mut mem(), ebp, [src, stats, x0, y0, x1, y1])
    }
    // ---- round 7 (left7) (ffx_pcui::l7_pcui) ----
    /// `(out, from, to, centre, t)`: the sphere grid token's point on its
    /// arc (`eax` the sine's index, as the original leaves it).
    AbmapUpdateMovingChrPos @ 0x00a5_63b0 => extern "C" fn abmap_update_moving_chr_pos(out: u32, from: u32, to: u32, centre: u32, t: u32) -> u32 {
        ffx_pcui::l7_pcui::bridge::update_moving_chr_pos(&mut mem(), out, from, to, centre, t)
    }
    /// `(window, msg, param)` -> 0: the camera test window's procedure
    /// (the actor position, near point and spline result in a frame).
    Gui_CameraTestProc_0093f900 @ 0x0093_f900 => extern "C" fn gui_camera_test_proc(window: u32, msg: u32, param: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_pcui::l7_pcui::bridge::CAMERA_TEST_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_pcui::l7_pcui::bridge::CAMERA_TEST_FRAME;
        ffx_pcui::l7_pcui::bridge::camera_test_proc(&mut mem(), ebp, window, msg, param)
    }
    // ---- round 8 (left8) (ffx_pcui::l8_pcui) ----
    /// `()`: `GDraw`'s D3D11 objects released at context teardown (its
    /// one caller reads no result).
    GDraw_releaseObjects_00a35c90 @ 0x00a3_5c90 => extern "C" fn gdraw_release_objects() {
        ffx_pcui::l8_pcui::bridge::release_objects(&mut mem());
    }
    /// `(device, context, w, h)` -> the `GDraw` function table or 0:
    /// `GDraw`'s D3D11 context set-up.
    gdraw_createContext_00a35780 @ 0x00a3_5780 => extern "C" fn gdraw_create_context(device: u32, context: u32, w: u32, h: u32) -> u32 {
        ffx_pcui::l8_pcui::bridge::create_context(&mut mem(), device, context, w, h)
    }
    /// `(base, _, info)` (`ret 0xc`) -> the texture handle or 0: an Iggy
    /// raw texture made on the device (its description, data array and
    /// outs in a frame; the second word is not read).
    GDraw_createRawTexture_00a37c20 @ 0x00a3_7c20 => extern "stdcall" fn gdraw_create_raw_texture(base: u32, _unused: u32, info: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_pcui::l8_pcui::bridge::RAW_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_pcui::l8_pcui::bridge::RAW_FRAME;
        ffx_pcui::l8_pcui::bridge::create_raw_texture(&mut mem(), ebp, base, info)
    }
    /// `(player)` (fastcall) -> `al` 1 when the stream plays: the video
    /// player's Vorbis decoder and FMOD stream set-up (the
    /// `FMOD_CREATESOUNDEXINFO` and the codec data's size in a frame;
    /// the rest of `eax` the last call's, as the original leaves it).
    PVideoPlaybackWin32_AudioInit_00a273b0 @ 0x00a2_73b0 => extern "fastcall" fn video_audio_init(player: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_pcui::l8_pcui::bridge::AUDIO_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_pcui::l8_pcui::bridge::AUDIO_FRAME;
        ffx_pcui::l8_pcui::bridge::audio_init(&mut mem(), ebp, player)
    }
}
