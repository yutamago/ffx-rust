//! `ffx_pcui::m5u_*`: M5 stage 1, list `pc-ui` (the PC user interface
//! code the static call graph reaches and no coverage run ran).
//!
//! Threads: the main thread (the frame's input pass, the menus,
//! `graphicFinalize`) and the input thread (`AsyncControllerTaskManager`,
//! 32 KB of stack: the frames here are a few words); the shims keep no
//! state, frames are per call.

use super::{StackFrame, mem, ported};
use ffx_pcui::m5u_a::bridge as a;
use ffx_pcui::m5u_b::bridge as b;
use ffx_pcui::m5u_c::bridge as c;
use ffx_pcui::m5u_d::bridge as d;
use ffx_pcui::m5u_e::bridge as e;
use ffx_pcui::m5u_f::bridge as ff;
use ffx_pcui::m5u_g::bridge as gg;

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
}
