//! `ffx_pcui::m5u_*`: M5 stage 1, list `pc-ui` (the PC user interface
//! code the static call graph reaches and no coverage run ran).
//!
//! Threads: the main thread (the frame's input pass, the menus,
//! `graphicFinalize`) and the input thread (`AsyncControllerTaskManager`,
//! 32 KB of stack: the frames here are a few words); the shims keep no
//! state, frames are per call.

use super::{StackFrame, mem, ported};
use ffx_pcui::m5u_a::bridge as a;

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
}
