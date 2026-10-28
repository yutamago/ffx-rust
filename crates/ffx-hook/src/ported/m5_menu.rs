//! M5 stage 1, list `menu` (`ffx_menu::m5m_*`): the `Gu_` debug GUI's
//! frame (`Sg_MainDebug` > `Gu_Main`: mouse, hit tests, window drawing)
//! and the rest of the menu code the static call graph reaches.
//!
//! Threads: the main thread (the frame's update); the shims keep no
//! state, frames are per call.

use super::{StackFrame, mem, ported};
use ffx_menu::m5m_a::bridge as a;
use ffx_menu::m5m_b::bridge as b;
use ffx_menu::m5m_c::bridge as c;

ported! {
    // ---- batch 1 (ffx_menu::m5m_a) ----
    /// `Gu_DrawEnd(channel)` -> `sceGsSyncPath`'s `eax`.
    Gu_DrawEnd @ 0x0084_79d0 => extern "C" fn gu_draw_end(channel: u32) -> u32 {
        a::draw_end(&mut mem(), channel)
    }
    /// `Gu_DrawMouseCursor()` -> the sprite call's `eax`.
    Gu_DrawMouseCursor @ 0x0084_7a40 => extern "C" fn gu_draw_mouse_cursor() -> u32 {
        a::draw_mouse_cursor(&mut mem())
    }
    /// `Gu_ClientMouse(window)`.
    Gu_ClientMouse @ 0x0084_9510 => extern "C" fn gu_client_mouse(window: u32) {
        a::client_mouse(&mut mem(), window);
    }
    /// `Gu_ControlMouseButtonHandle(window, id)` -> the last callback's
    /// `eax` (or the buttons).
    Gu_ControlMouseButtonHandle @ 0x0084_b960 => extern "C" fn gu_control_mouse_button(window: u32, id: u32) -> u32 {
        a::control_mouse_button(&mut mem(), window, id)
    }
    /// `Gu_ControlMouse(window, control)` -> 0.
    Gu_ControlMouse @ 0x0084_b7f0 => extern "C" fn gu_control_mouse(window: u32, control: u32) -> u32 {
        a::control_mouse(&mut mem(), window, control)
    }
    /// `Gu_WindowMouse(window)`.
    Gu_WindowMouse @ 0x0084_afd0 => extern "C" fn gu_window_mouse(window: u32) -> u32 {
        a::window_mouse(&mut mem(), window)
    }
    /// `Gu_WindowListMouse()`.
    Gu_WindowListMouse @ 0x0084_b2e0 => extern "C" fn gu_window_list_mouse() -> u32 {
        a::windows_mouse(&mut mem())
    }
    /// `Gu_VSync()`.
    Gu_VSync @ 0x0084_a260 => extern "C" fn gu_vsync() {
        a::vsync(&mut mem());
    }
    /// `Gu_DrawCursor(cursor, dx, dy)` -> the GIF tag's address.
    Gu_DrawCursor @ 0x0084_7420 => extern "C" fn gu_draw_cursor(cursor: u32, dx: u32, dy: u32) -> u32 {
        a::cursor_draw(&mut mem(), cursor, dx, dy)
    }
    /// `Gu_PrintBegin()` -> the red colour word.
    Gu_PrintBegin @ 0x0084_9170 => extern "C" fn gu_print_begin() -> u32 {
        a::print_begin(&mut mem())
    }
    /// `Gu_PrintGetWidth(text)`: the codes' arguments go to a word of
    /// this frame (the original's own argument word).
    Gu_PrintGetWidth @ 0x0084_8d60 => extern "C" fn gu_print_get_width(text: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        a::print_get_width(&mut mem(), text, f.at())
    }
    /// `Gu_DrawControl(control, dx, dy, window)`.
    Gu_DrawControl @ 0x0084_7720 => extern "C" fn gu_draw_control(control: u32, dx: u32, dy: u32, window: u32) {
        a::draw_control(&mut mem(), control, dx, dy, window);
    }
    /// `Gu_DrawWindowTree(window)`.
    Gu_DrawWindowTree @ 0x0084_ac80 => extern "C" fn gu_draw_window_tree(window: u32) {
        a::draw_window_tree(&mut mem(), window);
    }
    /// `Gu_Main(channel, buffer)`.
    Gu_Main @ 0x0084_9850 => extern "C" fn gu_main(channel: u32, buffer: u32) {
        a::gu_main(&mut mem(), channel, buffer);
    }
    // ---- batch 2 (ffx_menu::m5m_b) ----
    /// `TkSetVsyncHandler(handler)` -> the thread start's `eax`, or -1.
    TkSetVsyncHandler @ 0x0088_8680 => extern "C" fn tk_set_vsync_handler(handler: u32) -> u32 {
        b::set_vsync_handler(&mut mem(), handler)
    }
    /// `TkClearVsyncHandler()`.
    TkClearVsyncHandler @ 0x0088_8830 => extern "C" fn tk_clear_vsync_handler() -> u32 {
        b::clear_vsync_handler(&mut mem())
    }
    /// `escMenuSetAudioByConfig(mode, timeout)`: `sceGsSyncPath`'s tail.
    escMenuSetAudioByConfig @ 0x0088_ec70 => extern "C" fn esc_menu_set_audio_by_config(mode: u32, timeout: u32) -> u32 {
        b::esc_menu_set_audio_by_config(&mut mem(), mode, timeout)
    }
    /// `TkSignalSemaBGThread()`.
    TkSignalSemaBGThread @ 0x0088_8820 => extern "C" fn tk_signal_sema_bg_thread() -> u32 {
        b::signal_sema_bg_thread(&mut mem())
    }
    /// `MenuRecord_InitMulti(record)` -> record (stdcall).
    MenuRecord_InitMulti_0067c030 @ 0x0067_c030 => extern "stdcall" fn menu_record_init_multi(record: u32) -> u32 {
        b::record_init_multi(&mut mem(), record)
    }
    /// `MenuRecord_InitSingle(record)` -> record (stdcall).
    MenuRecord_InitSingle_0067c2d0 @ 0x0067_c2d0 => extern "stdcall" fn menu_record_init_single(record: u32) -> u32 {
        b::record_init_single(&mut mem(), record)
    }
    /// `MenuTextures_ResetBuffers(this)` -> `memset`'s `eax`.
    MenuTextures_ResetBuffers @ 0x0067_c440 => extern "thiscall" fn menu_textures_reset_buffers(this: u32) -> u32 {
        b::reset_buffers(&mut mem(), this)
    }
    /// `Gu_WindowAddControl(window, id, text, x, y, w, h)` -> the control.
    Gu_WindowAddControl @ 0x0084_bae0 => extern "C" fn gu_window_add_control(window: u32, id: u32, text: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
        b::window_add_control(&mut mem(), window, id, text, x, y, w, h)
    }
    /// `Gu_WindowAddButton(window, id, text, x, y, w, h)` -> the control.
    Gu_WindowAddButton @ 0x0084_a470 => extern "C" fn gu_window_add_button(window: u32, id: u32, text: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
        b::window_add_button(&mut mem(), window, id, text, x, y, w, h)
    }
    /// `Gu_WindowAddStatic(window, id, text, x, y, w, h)` -> the control.
    Gu_WindowAddStatic @ 0x0084_a4d0 => extern "C" fn gu_window_add_static(window: u32, id: u32, text: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
        b::window_add_static(&mut mem(), window, id, text, x, y, w, h)
    }
    /// `Gu_WindowAddClient(window, id, x, y, w, h)` -> the control.
    Gu_WindowAddClient @ 0x0084_a4a0 => extern "C" fn gu_window_add_client(window: u32, id: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
        b::window_add_client(&mut mem(), window, id, x, y, w, h)
    }
    /// `Gu_VSyncMain()` -> the clock tick's `eax`.
    Gu_VSyncMain @ 0x0084_a460 => extern "C" fn gu_vsync_main() -> u32 {
        b::vsync_main(&mut mem())
    }
    /// `Gu_WindowSetLocalTrans(window, on, fade)` -> the fade.
    Gu_WindowSetLocalTrans @ 0x0084_b620 => extern "C" fn gu_window_set_local_trans(window: u32, on: u32, fade: u32) -> u32 {
        b::window_set_local_trans(&mut mem(), window, on, fade)
    }
    /// `Gu_WindowCreate(x, y, w, h, flags, callback)` -> the window.
    Gu_WindowCreate @ 0x0084_a920 => extern "C" fn gu_window_create(x: u32, y: u32, w: u32, h: u32, flags: u32, callback: u32) -> u32 {
        b::window_create(&mut mem(), x, y, w, h, flags, callback)
    }
    // ---- batch 3 (ffx_menu::m5m_c) ----
    /// `AtelDebug_PrintPosWork_00(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintPosWork_00 @ 0x0087_a4a0 => extern "C" fn ateldebug_printposwork_00(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Pos(0x0), worker)
    }
    /// `AtelDebug_PrintPosWork_04(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintPosWork_04 @ 0x0087_a4f0 => extern "C" fn ateldebug_printposwork_04(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Pos(0x4), worker)
    }
    /// `AtelDebug_PrintPosWork_08(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintPosWork_08 @ 0x0087_a550 => extern "C" fn ateldebug_printposwork_08(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Pos(0x8), worker)
    }
    /// `AtelDebug_PrintLevelF32_28(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintLevelF32_28 @ 0x0087_a5b0 => extern "C" fn ateldebug_printlevelf32_28(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Level(0x28), worker)
    }
    /// `AtelDebug_PrintLevelF32_2c(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintLevelF32_2c @ 0x0087_a610 => extern "C" fn ateldebug_printlevelf32_2c(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Level(0x2c), worker)
    }
    /// `AtelDebug_PrintLevelF32_30(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintLevelF32_30 @ 0x0087_a670 => extern "C" fn ateldebug_printlevelf32_30(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Level(0x30), worker)
    }
    /// `AtelDebug_PrintPosWork_0c(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintPosWork_0c @ 0x0087_a6d0 => extern "C" fn ateldebug_printposwork_0c(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Pos(0xc), worker)
    }
    /// `AtelDebug_PrintLevelF32_10(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintLevelF32_10 @ 0x0087_a730 => extern "C" fn ateldebug_printlevelf32_10(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Level(0x10), worker)
    }
    /// `AtelDebug_PrintPosWork_1c(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintPosWork_1c @ 0x0087_a790 => extern "C" fn ateldebug_printposwork_1c(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Pos(0x1c), worker)
    }
    /// `AtelDebug_PrintLevelF32_38(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintLevelF32_38 @ 0x0087_a7f0 => extern "C" fn ateldebug_printlevelf32_38(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Level(0x38), worker)
    }
    /// `AtelDebug_PrintLevel44I16_06(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintLevel44I16_06 @ 0x0087_a850 => extern "C" fn ateldebug_printlevel44i16_06(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::LevelI16(0x44, 6), worker)
    }
    /// `AtelDebug_PrintLevel44I16_04(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintLevel44I16_04 @ 0x0087_a8b0 => extern "C" fn ateldebug_printlevel44i16_04(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::LevelI16(0x44, 4), worker)
    }
    /// `AtelDebug_PrintLevel48I16_06(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintLevel48I16_06 @ 0x0087_a910 => extern "C" fn ateldebug_printlevel48i16_06(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::LevelI16(0x48, 6), worker)
    }
    /// `AtelDebug_PrintLevel48I16_04(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintLevel48I16_04 @ 0x0087_a970 => extern "C" fn ateldebug_printlevel48i16_04(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::LevelI16(0x48, 4), worker)
    }
    /// `AtelDebug_PrintPosWork_20(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintPosWork_20 @ 0x0087_a9d0 => extern "C" fn ateldebug_printposwork_20(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Pos(0x20), worker)
    }
    /// `AtelDebug_PrintPosAngle_20(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintPosAngle_20 @ 0x0087_aa40 => extern "C" fn ateldebug_printposangle_20(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::PosAngle(0x20), worker)
    }
    /// `AtelDebug_PrintPosWork_24(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintPosWork_24 @ 0x0087_aaa0 => extern "C" fn ateldebug_printposwork_24(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Pos(0x24), worker)
    }
    /// `AtelDebug_PrintPosWork_34(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintPosWork_34 @ 0x0087_ab00 => extern "C" fn ateldebug_printposwork_34(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Pos(0x34), worker)
    }
    /// `AtelDebug_PrintPosAngle_34(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintPosAngle_34 @ 0x0087_ab60 => extern "C" fn ateldebug_printposangle_34(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::PosAngle(0x34), worker)
    }
    /// `AtelDebug_PrintPosWork_30(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintPosWork_30 @ 0x0087_abc0 => extern "C" fn ateldebug_printposwork_30(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Pos(0x30), worker)
    }
    /// `AtelDebug_PrintPosWork_38(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintPosWork_38 @ 0x0087_ac20 => extern "C" fn ateldebug_printposwork_38(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Pos(0x38), worker)
    }
    /// `AtelDebug_PrintPosWork_40(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintPosWork_40 @ 0x0087_ac80 => extern "C" fn ateldebug_printposwork_40(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Pos(0x40), worker)
    }
    /// `AtelDebug_PrintPosWork_48(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintPosWork_48 @ 0x0087_ace0 => extern "C" fn ateldebug_printposwork_48(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Pos(0x48), worker)
    }
    /// `AtelDebug_PrintPosWork_44(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintPosWork_44 @ 0x0087_ad40 => extern "C" fn ateldebug_printposwork_44(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Pos(0x44), worker)
    }
    /// `AtelDebug_PrintPosWork_3c(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintPosWork_3c @ 0x0087_ada0 => extern "C" fn ateldebug_printposwork_3c(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Pos(0x3c), worker)
    }
    /// `AtelDebug_PrintPosWork_4c(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintPosWork_4c @ 0x0087_ae00 => extern "C" fn ateldebug_printposwork_4c(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_field(&mut mem(), ebp, ffx_menu::m5m_c::Field::Pos(0x4c), worker)
    }
    /// `AtelDebug_PrintKind(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintKind @ 0x0087_ae60 => extern "C" fn print_kind(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_kind(&mut mem(), ebp, worker)
    }
    /// `AtelDebug_PrintModel(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintModel @ 0x0087_af50 => extern "C" fn print_model(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_model(&mut mem(), ebp, worker)
    }
    /// `AtelDebug_PrintEntry(worker)`: frame `ebp - 0x28`.
    AtelDebug_PrintEntry @ 0x0087_afa0 => extern "C" fn print_entry(worker: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        let ebp = f.at() + 0x28;
        c::print_entry(&mut mem(), ebp, worker)
    }
    /// `AtelDebug_PrintMotion(worker)`.
    AtelDebug_PrintMotion @ 0x0087_b040 => extern "C" fn print_motion(worker: u32) -> u32 {
        c::print_motion(&mut mem(), worker)
    }
    /// `TkPutDebugFontString(color, flags, s)`.
    TkPutDebugFontString @ 0x0088_f140 => extern "C" fn tk_put_debug_font_string(color: u32, flags: u32, s: u32) -> u32 {
        c::put_string(&mut mem(), color, flags, s)
    }
    /// `TkPutDebugFontStringP(color, flags, s)`.
    TkPutDebugFontStringP @ 0x0088_f4c0 => extern "C" fn tk_put_debug_font_string_p(color: u32, flags: u32, s: u32) -> u32 {
        c::put_string_p(&mut mem(), color, flags, s)
    }
    /// `TkPutDebugFontStringPR(color, flags, s)`.
    TkPutDebugFontStringPR @ 0x0088_f5c0 => extern "C" fn tk_put_debug_font_string_pr(color: u32, flags: u32, s: u32) -> u32 {
        c::put_string_pr(&mut mem(), color, flags, s)
    }
}
