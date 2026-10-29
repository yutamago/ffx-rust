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
use ffx_menu::m5m_d::bridge as d;
use ffx_menu::m5m_e::bridge as e;
use ffx_menu::m5m_f::bridge as f;
use ffx_menu::m5m_g::bridge as g;
use ffx_menu::m5m_h::bridge as h;
use ffx_menu::m5m_i::bridge as i;
use ffx_menu::m5m_j::bridge as j;

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
    // ---- batch 4 (ffx_menu::m5m_d) ----
    /// `TkCdRead(file, sector, buf)`.
    TkCdRead @ 0x0088_d140 => extern "C" fn tk_cd_read(file: u32, sector: u32, buf: u32) -> u32 {
        d::cd_read(&mut mem(), file, sector, buf)
    }
    /// `TkCdReadSync(mode)`: the reader's sync entry with the caller's word.
    TkCdReadSync @ 0x0088_d1c0 => extern "C" fn tk_cd_read_sync(mode: u32) -> u32 {
        d::cd_read_sync(&mut mem(), mode)
    }
    /// `TkCdReadNB(file, sector, buf)`.
    TkCdReadNB @ 0x0088_d190 => extern "C" fn tk_cd_read_nb(file: u32, sector: u32, buf: u32) -> u32 {
        d::cd_read_nb(&mut mem(), file, sector, buf)
    }
    /// `TkCdSize(file, sector)`.
    TkCdSize @ 0x0088_d390 => extern "C" fn tk_cd_size(file: u32, sector: u32) -> u32 {
        d::cd_size(&mut mem(), file, sector)
    }
    /// `TkCdReadWithMalloc(file, sector)` -> the buffer.
    TkCdReadWithMalloc @ 0x0088_d1d0 => extern "C" fn tk_cd_read_with_malloc(file: u32, sector: u32) -> u32 {
        d::cd_read_with_malloc(&mut mem(), file, sector)
    }
    /// `TkMemLock()`.
    TkMemLock @ 0x0088_e680 => extern "C" fn tk_mem_lock() -> u32 {
        d::mem_check_pair(&mut mem(), 0x1c5)
    }
    /// `TkMemUnlock()`.
    TkMemUnlock @ 0x0088_e6b0 => extern "C" fn tk_mem_unlock() -> u32 {
        d::mem_check_pair(&mut mem(), 0x1d0)
    }
    /// `TkSoftWareReset()`.
    TkSoftWareReset @ 0x0087_2550 => extern "C" fn tk_software_reset() -> u32 {
        d::software_reset(&mut mem())
    }
    /// `TkInitAllSyncVCallback()`.
    TkInitAllSyncVCallback @ 0x0088_86f0 => extern "C" fn tk_init_all_sync_v_callback() -> u32 {
        d::init_all_sync_v_callback(&mut mem())
    }
    /// `TkPadInfoAct(port, slot, actuator)`.
    TkPadInfoAct @ 0x0088_8a40 => extern "C" fn tk_pad_info_act(port: u32, slot: u32, act: u32) -> u32 {
        d::pad_info_act(&mut mem(), port, slot, act)
    }
    /// `TkMcSync(mode, &cmd, &result)`: frame `ebp - 0xc`.
    TkMcSync @ 0x0088_84e0 => extern "C" fn tk_mc_sync(mode: u32, cmd: u32, result: u32) -> u32 {
        let mut f = StackFrame::<3>::new();
        let ebp = f.at() + 0xc;
        d::mc_sync(&mut mem(), ebp, mode, cmd, result)
    }
    /// `TkMcMakePath(dst, name)`: frame `ebp - 0x400`.
    TkMcMakePath @ 0x0088_8030 => extern "C" fn tk_mc_make_path(dst: u32, name: u32) -> u32 {
        let mut f = StackFrame::<256>::new();
        let ebp = f.at() + 0x400;
        d::mc_make_path(&mut mem(), ebp, dst, name)
    }
    /// `TkInitDeci2()` -> the socket.
    TkInitDeci2 @ 0x0088_5240 => extern "C" fn tk_init_deci2() -> u32 {
        d::init_deci2(&mut mem())
    }
    /// `TkDeci2Send(packet, len)`.
    TkDeci2Send @ 0x0088_54c0 => extern "C" fn tk_deci2_send(packet: u32, len: u32) -> u32 {
        d::deci2_send(&mut mem(), packet, len)
    }
    /// `TkRemove(name)`.
    TkRemove @ 0x0088_c740 => extern "C" fn tk_remove(name: u32) -> u32 {
        d::remove(&mut mem(), name)
    }
    // ---- batch 5 (ffx_menu::m5m_e) ----
    /// `Gu_SelectorOpen(selector, owner, arg)`.
    Gu_SelectorOpen @ 0x0084_9d60 => extern "C" fn gu_selector_open(selector: u32, owner: u32, arg: u32) -> u32 {
        e::selector_open(&mut mem(), selector, owner, arg)
    }
    /// `Gu_SelectorOpenXYWH(selector, owner, arg, x, y, w, h)`.
    Gu_SelectorOpenXYWH @ 0x0084_9dc0 => extern "C" fn gu_selector_open_xywh(selector: u32, owner: u32, arg: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
        e::selector_open_xywh(&mut mem(), selector, owner, arg, x, y, w, h)
    }
    /// `Gu_MessageBox(text)` -> the OK button.
    Gu_MessageBox @ 0x0084_9970 => extern "C" fn gu_message_box(text: u32) -> u32 {
        e::message_box(&mut mem(), text)
    }
    /// `Gu_MessageBoxProc(window, msg, arg)` -> 0.
    Gu_MessageBoxProc @ 0x0084_9a10 => extern "C" fn gu_message_box_proc(window: u32, msg: u32, arg: u32) -> u32 {
        e::message_box_proc(&mut mem(), window, msg, arg)
    }
    /// `Gu_SelectorProc(window, msg, arg)` -> 0.
    Gu_SelectorProc @ 0x0084_9e30 => extern "C" fn gu_selector_proc(window: u32, msg: u32, arg: u32) -> u32 {
        e::selector_proc(&mut mem(), window, msg, arg)
    }
    /// `Gu_SelectorPaint(window, selector)`.
    Gu_SelectorPaint @ 0x0084_9c00 => extern "C" fn gu_selector_paint(window: u32, selector: u32) {
        e::selector_paint(&mut mem(), window, selector);
    }
    /// `Gu_SelectorSortItem(selector)`.
    Gu_SelectorSortItem @ 0x0084_a180 => extern "C" fn gu_selector_sort_item(selector: u32) {
        e::selector_sort_item(&mut mem(), selector);
    }
    /// `Gu_MakeFileList(list, path)`: frame `ebp - 0x204`.
    Gu_MakeFileList @ 0x0084_98e0 => extern "C" fn gu_make_file_list(list: u32, path: u32) -> u32 {
        let mut f = StackFrame::<129>::new();
        let ebp = f.at() + 0x204;
        e::make_file_list(&mut mem(), ebp, list, path)
    }
    // ---- batch 6 (ffx_menu::m5m_f) ----
    /// `TkMenuDrawSp3R(x, y, flags)`.
    TkMenuDrawSp3R @ 0x008d_ca60 => extern "C" fn tk_menu_draw_sp3r(x: u32, y: u32, flags: u32) -> u32 {
        f::draw_sp3_r0(&mut mem(), x, y, flags)
    }
    /// `TkMenuDraw1612WidthHeight(text, flags)`.
    TkMenuDraw1612WidthHeight @ 0x008d_c9f0 => extern "C" fn tk_menu_draw_1612_width_height(text: u32, flags: u32) -> u32 {
        f::draw_1612_width_height(&mut mem(), text, flags)
    }
    /// `TkMenuDrawEasyMes1612(x, y, text)`.
    TkMenuDrawEasyMes1612 @ 0x008d_cb80 => extern "C" fn tk_menu_draw_easy_mes_1612(x: u32, y: u32, text: u32) -> u32 {
        f::draw_easy_mes_1612(&mut mem(), x, y, text)
    }
    /// `TkMenuDrawEasyMes1612WithMacro(x, y, text, macro)`.
    TkMenuDrawEasyMes1612WithMacro @ 0x008d_cc70 => extern "C" fn tk_menu_draw_easy_mes_1612_with_macro(x: u32, y: u32, text: u32, m: u32) -> u32 {
        f::draw_easy_mes_1612_with_macro(&mut mem(), x, y, text, m)
    }
    /// `TkMenuDrawWindow1612(x, y, w, h)` -> whether it clipped.
    TkMenuDrawWindow1612 @ 0x008d_cf40 => extern "C" fn tk_menu_draw_window_1612(x: u32, y: u32, w: u32, h: u32) -> u32 {
        f::draw_window_1612(&mut mem(), x, y, w, h)
    }
    /// `DrawCrossButtonIcon(id, x, y, w, h)` (`f32` words).
    DrawCrossButtonIcon @ 0x008e_6030 => extern "C" fn draw_cross_button_icon(id: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
        f::draw_cross_button_icon(&mut mem(), id, x, y, w, h)
    }
    /// `TkMn2DrawCrossRect(x, y, w, h, c0, c1)` (`f32` words): frame `ebp - 0x9c`.
    TkMn2DrawCrossRect @ 0x008c_0930 => extern "C" fn tk_mn2_draw_cross_rect(x: u32, y: u32, w: u32, h: u32, c0: u32, c1: u32) -> u32 {
        let mut fr = StackFrame::<40>::new();
        let ebp = fr.at() + 0xa0;
        f::draw_cross_rect(&mut mem(), ebp, x, y, w, h, c0, c1)
    }
    /// `TkMn2DrawRect(x, y, w, h, c0, c1)`: frame `ebp - 0xa0`.
    TkMn2DrawRect @ 0x008c_1280 => extern "C" fn tk_mn2_draw_rect(x: u32, y: u32, w: u32, h: u32, c0: u32, c1: u32) -> u32 {
        let mut fr = StackFrame::<40>::new();
        let ebp = fr.at() + 0xa0;
        f::draw_rect(&mut mem(), ebp, x, y, w, h, c0, c1)
    }
    /// `DrawBlackBackGround()`: frame `ebp - 0x9c` (zeroed: the junk reads).
    DrawBlackBackGround @ 0x008d_b870 => extern "C" fn draw_black_background() -> u32 {
        let mut fr = StackFrame::<40>::new();
        let ebp = fr.at() + 0xa0;
        f::draw_black_background(&mut mem(), ebp)
    }
    /// `gui_eve_Init()`.
    gui_eve_Init @ 0x0085_4e60 => extern "C" fn gui_eve_init() -> u32 {
        f::eve_init(&mut mem())
    }
    /// `gui_eve_GWM_SELECTOR(window, msg)`: `eax` as the caller had it for
    /// other messages, so the shim returns nothing.
    gui_eve_GWM_SELECTOR @ 0x0085_4df0 => extern "C" fn gui_eve_gwm_selector(window: u32, msg: u32) {
        f::eve_gwm_selector(&mut mem(), window, msg);
    }
    /// `TkMn2DrawScreenStrips(a, b, y)`.
    TkMn2DrawScreenStrips @ 0x008a_9510 => extern "C" fn tk_mn2_draw_screen_strips(a: u32, b: u32, y: u32) -> u32 {
        f::draw_screen_strips(&mut mem(), a, b, y)
    }
    // ---- batch 7 (ffx_menu::m5m_g) ----
    /// `TkPutDebugFontStringPEuc(color, flags, text)`: frame `ebp - 0x2c`.
    TkPutDebugFontStringPEuc @ 0x0088_f4e0 => extern "C" fn tk_put_debug_font_string_p_euc(color: u32, flags: u32, text: u32) -> u32 {
        let mut fr = StackFrame::<11>::new();
        let ebp = fr.at() + 0x2c;
        g::put_string_p_euc(&mut mem(), ebp, color, flags, text)
    }
    /// `gui_eve_GWM_BUTTONUP(window)`.
    gui_eve_GWM_BUTTONUP @ 0x0085_4e80 => extern "C" fn gui_eve_gwm_buttonup(window: u32) -> u32 {
        g::eve_gwm_buttonup(&mut mem(), window)
    }
    // ---- round 3, batch 1 (ffx_menu::m5m_h) ----
    /// `code_0088d500()`: `jmp AtelDrawSemiTrans`.
    AtelDrawSemiTrans_thunk_0088d500 @ 0x0088_d500 => extern "C" fn atel_draw_semi_trans_thunk() -> u32 {
        h::atel_draw_semi_trans_thunk(&mut mem())
    }
    /// `setNearPlane(near)`: tail jump to the camera's virtual method 1.
    setNearPlane @ 0x0064_5f00 => extern "C" fn set_near_plane(near: u32) -> u32 {
        h::set_near_plane(&mut mem(), near)
    }
    /// `code_00887ae0()`: `jmp TkSndIsRead`.
    TkSndIsRead_thunk_00887ae0 @ 0x0088_7ae0 => extern "C" fn tk_snd_is_read_thunk() -> u32 {
        h::tk_snd_is_read_thunk(&mut mem())
    }
    /// `TkGetLang_thunk()`: `jmp TkGetLang`.
    TkGetLang_thunk @ 0x0088_7c30 => extern "C" fn tk_get_lang_thunk() -> u32 {
        h::tk_get_lang_thunk(&mut mem())
    }
    /// `jmp TkGetNowEventJumpMapNo`.
    TkGetNowEventJumpMapNo_0086c1f0 @ 0x0086_c1f0 => extern "C" fn tk_get_now_event_jump_map_no_thunk() -> u32 {
        h::tk_get_now_event_jump_map_no_thunk(&mut mem())
    }
    /// `code_0086c1d0()`: `jmp TkGetNowEventJumpIndex`.
    TkGetNowEventJumpIndex_thunk_0086c1d0 @ 0x0086_c1d0 => extern "C" fn tk_get_now_event_jump_index_thunk() -> u32 {
        h::tk_get_now_event_jump_index_thunk(&mut mem())
    }
    /// `TkGetSaveEventAddress()`: `jmp MsGetSaveEventAddress`.
    TkGetSaveEventAddress @ 0x0088_c830 => extern "C" fn tk_get_save_event_address() -> u32 {
        h::tk_get_save_event_address(&mut mem())
    }
    /// `code_0088e780(id)`: `jmp MsImportantReset`.
    MsImportantReset_thunk_0088e780 @ 0x0088_e780 => extern "C" fn ms_important_reset_thunk(id: u32) -> u32 {
        h::ms_important_reset_thunk(&mut mem(), id)
    }
    /// `code_0087aa30()`: the image address `0x00b546a0`.
    TkVibrateCurve_get_0087aa30 @ 0x0087_aa30 => extern "C" fn vibrate_curve_0087aa30() -> u32 {
        h::vibrate_curve_0087aa30(&mem())
    }
    /// `code_00888310()`.
    TkMcResetPorts_00888310 @ 0x0088_8310 => extern "C" fn tk_mc_reset_ports() -> u32 {
        h::mc_reset_ports(&mut mem())
    }
    /// `code_00888760()`.
    TkSyncAndClearVsync_00888760 @ 0x0088_8760 => extern "C" fn tk_sync_and_clear_vsync() -> u32 {
        h::sync_and_clear_vsync(&mut mem())
    }
    /// `code_0088de60()`.
    TkInitScene_LoadShoutStream_0088de60 @ 0x0088_de60 => extern "C" fn tk_load_shout_stream() -> u32 {
        h::load_shout_stream(&mut mem())
    }
    /// `code_0088ded0()`.
    TkInitScene_SendShoutStream_0088ded0 @ 0x0088_ded0 => extern "C" fn tk_send_shout_stream() -> u32 {
        h::send_shout_stream(&mut mem())
    }
    /// `code_008c1790()`: `jmp TkVU1SyncPath`.
    TkVU1SyncPath_thunk_008c1790 @ 0x008c_1790 => extern "C" fn tk_vu1_sync_path_thunk() -> u32 {
        h::tk_vu1_sync_path_thunk(&mut mem())
    }
    /// `code_008a9850()`: the image address `0x00b5f46c`.
    TkVibrateCurve_get_008a9850 @ 0x008a_9850 => extern "C" fn vibrate_curve_008a9850() -> u32 {
        h::vibrate_curve_008a9850(&mem())
    }
    /// `jmp MsGetGIL`.
    TkMenuGetGil_008a98b0 @ 0x008a_98b0 => extern "C" fn tk_menu_get_gil() -> u32 {
        h::ms_get_gil_thunk(&mut mem())
    }
    /// `Pause_Leave_008ab2d0()`.
    Pause_Leave_008ab2d0 @ 0x008a_b2d0 => extern "C" fn pause_leave() -> u32 {
        h::pause_leave(&mut mem())
    }
    /// `code_008bc080()`: `eax` is left as the callers don't read it.
    BtlEnd_DrawGilPages_008bc080 @ 0x008b_c080 => extern "C" fn btl_end_draw_gil_pages() {
        h::draw_gil_pages(&mut mem());
    }
    /// `code_008bd940()`.
    BtlEnd_DrawSpoilPages_008bd940 @ 0x008b_d940 => extern "C" fn btl_end_draw_spoil_pages() {
        h::draw_spoil_pages(&mut mem());
    }
    /// `code_008bd9c0()`.
    BtlEnd_TakeSpoils_008bd9c0 @ 0x008b_d9c0 => extern "C" fn btl_end_take_spoils() {
        h::take_spoils(&mut mem());
    }
    /// `code_008bda30()`: `jmp MsGetGIL`.
    MsGetGIL_thunk_008bda30 @ 0x008b_da30 => extern "C" fn ms_get_gil_thunk() -> u32 {
        h::ms_get_gil_thunk(&mut mem())
    }
    /// `code_008beff0()`.
    code_008beff0 @ 0x008b_eff0 => extern "C" fn menu_help_draw() -> u32 {
        h::help_draw(&mut mem())
    }
    /// `TkMn2DrawScissorRect(x, y, w, h)`: `jmp TODrawScissorXYWH`.
    TkMn2DrawScissorRect @ 0x008c_1390 => extern "C" fn tk_mn2_draw_scissor_rect(x: u32, y: u32, w: u32, hh: u32) -> u32 {
        h::draw_scissor_rect_thunk(&mut mem(), x, y, w, hh)
    }
    /// `code_008c1b10()`: `jmp TOMenuGetAbilitymapWorkAdrs`.
    TOMenuGetAbilitymapWorkAdrs_thunk_008c1b10 @ 0x008c_1b10 => extern "C" fn get_abilitymap_work_thunk() -> u32 {
        h::get_abilitymap_work_thunk(&mut mem())
    }
    /// `code_008c2380(list, kind)`: `jmp TkMn2BuildItemList_008c2390`.
    TkMn2BuildItemList_thunk_008c2380 @ 0x008c_2380 => extern "C" fn build_item_list_thunk(list: u32, kind: u32) -> u32 {
        h::build_item_list_thunk(&mut mem(), list, kind)
    }
    /// `TkMsSetSaveParamAll()`: `jmp MsSetSaveParamAll`.
    TkMsSetSaveParamAll @ 0x008c_30d0 => extern "C" fn tk_ms_set_save_param_all() -> u32 {
        h::tk_ms_set_save_param_all(&mut mem())
    }
    /// `code_008c3340()`.
    code_008c3340 @ 0x008c_3340 => extern "C" fn menu_help_close() -> u32 {
        h::help_close(&mut mem())
    }
    /// `code_008c3360()`.
    code_008c3360 @ 0x008c_3360 => extern "C" fn draw_empty_screen_008c3360() -> u32 {
        h::draw_empty_screen(&mut mem())
    }
    /// `code_008c3380()`: `0x01871538` = 1.
    code_008c3380 @ 0x008c_3380 => extern "C" fn help_flag_on() {
        h::set_help_flag(&mut mem(), 1);
    }
    /// `code_008c33e0()`: `0x01871538` = 0.
    code_008c33e0 @ 0x008c_33e0 => extern "C" fn help_flag_off() {
        h::set_help_flag(&mut mem(), 0);
    }
    /// `code_008c5ee0()`.
    code_008c5ee0 @ 0x008c_5ee0 => extern "C" fn item_step_draw_bg() -> u32 {
        h::item_step_draw_bg(&mut mem())
    }
    /// `code_008c5ef0()`.
    code_008c5ef0 @ 0x008c_5ef0 => extern "C" fn item_open() -> u32 {
        h::item_open(&mut mem())
    }
    /// `code_008c6050()`.
    code_008c6050 @ 0x008c_6050 => extern "C" fn item_close() -> u32 {
        h::item_close(&mut mem())
    }
    /// `code_008c60a0()`.
    code_008c60a0 @ 0x008c_60a0 => extern "C" fn screen_x2_off() -> u32 {
        h::screen_x2_off(&mut mem())
    }
    /// `code_008c64a0()`.
    code_008c64a0 @ 0x008c_64a0 => extern "C" fn item_draw_plate() -> u32 {
        h::item_draw_plate(&mut mem())
    }
    /// `code_008c64f0()`.
    code_008c64f0 @ 0x008c_64f0 => extern "C" fn shop_open_rect() -> u32 {
        h::shop_open_rect(&mut mem())
    }
    // ---- round 3, batch 2 (ffx_menu::m5m_i) ----
    /// `jmp TkSndUpdate`.
    TkSndUpdate_thunk_008ca470 @ 0x008c_a470 => extern "C" fn tk_snd_update_thunk() -> u32 {
        i::tk_snd_update_thunk(&mut mem())
    }
    /// `code_008cb350()`: the config screen's open.
    code_008cb350 @ 0x008c_b350 => extern "C" fn config_open() -> u32 {
        i::config_open(&mut mem())
    }
    /// `code_008cc1a0()`: `jmp 0x008ceef0`.
    code_008cc1a0 @ 0x008c_c1a0 => extern "C" fn summon_init_thunk() -> u32 {
        i::summon_init_thunk(&mut mem())
    }
    /// `code_008cc300()`.
    code_008cc300 @ 0x008c_c300 => extern "C" fn summon_close() -> u32 {
        i::summon_close(&mut mem())
    }
    /// `code_008cf820()`: `jmp 0x008cfc40`.
    code_008cf820 @ 0x008c_f820 => extern "C" fn weapon_init_thunk() -> u32 {
        i::weapon_init_thunk(&mut mem())
    }
    /// `code_008cf910()`.
    code_008cf910 @ 0x008c_f910 => extern "C" fn weapon_stop_scenes() -> u32 {
        i::weapon_stop_scenes(&mut mem())
    }
    /// `code_008d09b0()`: the limit screen's draw.
    code_008d09b0 @ 0x008d_09b0 => extern "C" fn limit_draw() -> u32 {
        i::limit_draw(&mut mem())
    }
    /// `code_008d0a10()`: `jmp 0x008d0cd0`.
    code_008d0a10 @ 0x008d_0a10 => extern "C" fn limit_init_thunk() -> u32 {
        i::limit_init_thunk(&mut mem())
    }
    /// `code_008d0c70()`.
    code_008d0c70 @ 0x008d_0c70 => extern "C" fn limit_close() -> u32 {
        i::limit_close(&mut mem())
    }
    /// `code_008d1d30()`.
    code_008d1d30 @ 0x008d_1d30 => extern "C" fn status_open() -> u32 {
        i::status_open(&mut mem())
    }
    /// `code_008d1dd0()`.
    code_008d1dd0 @ 0x008d_1dd0 => extern "C" fn status_close() -> u32 {
        i::status_close(&mut mem())
    }
    /// `code_008d55c0()`.
    code_008d55c0 @ 0x008d_55c0 => extern "C" fn draw_sync_only() -> u32 {
        i::draw_sync_only(&mut mem())
    }
    /// `code_008d55d0()`: `jmp 0x008d6be0`.
    code_008d55d0 @ 0x008d_55d0 => extern "C" fn kaizou_init_thunk() -> u32 {
        i::kaizou_init_thunk(&mut mem())
    }
    /// `code_008d5650()`.
    code_008d5650 @ 0x008d_5650 => extern "C" fn kaizou_close() -> u32 {
        i::kaizou_close(&mut mem())
    }
    /// `code_008d7560()`.
    code_008d7560 @ 0x008d_7560 => extern "C" fn draw_empty_screen_008d7560() -> u32 {
        h::draw_empty_screen(&mut mem())
    }
    /// `code_008d7580()`.
    code_008d7580 @ 0x008d_7580 => extern "C" fn shop_get_pos_008d7580() -> u32 {
        i::shop_get_pos(&mut mem())
    }
    /// `code_008d75b0()`.
    code_008d75b0 @ 0x008d_75b0 => extern "C" fn wshop_close() -> u32 {
        i::wshop_close(&mut mem())
    }
    /// `code_008d9aa0()`.
    code_008d9aa0 @ 0x008d_9aa0 => extern "C" fn draw_empty_screen_008d9aa0() -> u32 {
        h::draw_empty_screen(&mut mem())
    }
    /// `code_008d9ac0()`.
    code_008d9ac0 @ 0x008d_9ac0 => extern "C" fn shop_get_pos_008d9ac0() -> u32 {
        i::shop_get_pos(&mut mem())
    }
    /// `code_008d9ae0()`.
    code_008d9ae0 @ 0x008d_9ae0 => extern "C" fn wshop_close3() -> u32 {
        i::wshop_close3(&mut mem())
    }
    /// `code_008db7d0()`: `g_TkMenuTutoChosen` = 0.
    code_008db7d0 @ 0x008d_b7d0 => extern "C" fn tuto_clear() {
        i::tuto_clear(&mut mem());
    }
    /// `TkMenuDrawScissorRect(x, y, w, h)`: `jmp TODrawScissorXYWH`.
    TkMenuDrawScissorRect @ 0x008d_d460 => extern "C" fn tk_menu_draw_scissor_rect(x: u32, y: u32, w: u32, hh: u32) -> u32 {
        i::menu_draw_scissor_rect(&mut mem(), x, y, w, hh)
    }
    /// `code_008dde90()`: the image address `0x00b5f46c`.
    TkVibrateCurve_get_008dde90 @ 0x008d_de90 => extern "C" fn vibrate_curve_008dde90() -> u32 {
        i::vibrate_curve_008dde90(&mem())
    }
    /// `(w, x, y)`: the gear list's cursor.
    MenuItem_DrawGearCursor_008c9940 @ 0x008c_9940 => extern "C" fn menu_item_draw_gear_cursor(w: u32, x: u32, y: u32) -> u32 {
        i::list_cursor(&mut mem(), w, x, y)
    }
    /// `(w, x, y, row, fixed)`: one summon grow row.
    MenuSummon_DrawGrowRow_008cdab0 @ 0x008c_dab0 => extern "C" fn menu_summon_draw_grow_row(w: u32, x: u32, y: u32, row: u32, fixed: u32) {
        i::row_draw(&mut mem(), i::Rows::Summon, w, x, y, row, fixed);
    }
    /// `(w, a, b, dx, dy)`: the summon grow rows.
    MenuSummon_DrawGrowRows_008cd990 @ 0x008c_d990 => extern "C" fn menu_summon_draw_grow_rows(w: u32, a: u32, b: u32, dx: u32, dy: u32) {
        i::list_draw(&mut mem(), i::Rows::Summon, w, a, b, dx, dy);
    }
    /// `(w)`: the summon grow cursor (the callers' two other words are
    /// only overwritten).
    MenuSummon_DrawGrowCursor_008cda20 @ 0x008c_da20 => extern "C" fn menu_summon_draw_grow_cursor(w: u32) -> u32 {
        i::summon_cursor(&mut mem(), w)
    }
    /// `(w, x, y, row, fixed)`: one customize row.
    MenuKaizou_DrawRow_008d5e60 @ 0x008d_5e60 => extern "C" fn menu_kaizou_draw_row(w: u32, x: u32, y: u32, row: u32, fixed: u32) {
        i::row_draw(&mut mem(), i::Rows::Kaizou, w, x, y, row, fixed);
    }
    /// `(w, a, b, dx, dy)`: the customize rows.
    MenuKaizou_DrawRows_008d5d50 @ 0x008d_5d50 => extern "C" fn menu_kaizou_draw_rows(w: u32, a: u32, b: u32, dx: u32, dy: u32) {
        i::list_draw(&mut mem(), i::Rows::Kaizou, w, a, b, dx, dy);
    }
    /// `(w, x, y)`: the customize list's cursor.
    MenuKaizou_DrawCursor_008d5df0 @ 0x008d_5df0 => extern "C" fn menu_kaizou_draw_cursor_008d5df0(w: u32, x: u32, y: u32) -> u32 {
        i::row_cursor(&mut mem(), w, x, y)
    }
    /// `(w, x, y)`: the customize screen's second cursor.
    MenuKaizou_DrawCursor_008d65f0 @ 0x008d_65f0 => extern "C" fn menu_kaizou_draw_cursor_008d65f0(w: u32, x: u32, y: u32) -> u32 {
        i::row_cursor(&mut mem(), w, x, y)
    }
    // ---- round 3, batch 3 (ffx_menu::m5m_j) ----
    /// `MallocAndReadGamecode(lang)`: the name and descriptor in a frame
    /// at the original's `ebp - 0x88`.
    MallocAndReadGamecode @ 0x0064_6920 => extern "C" fn malloc_and_read_gamecode(lang: u32) -> u32 {
        let mut fr = StackFrame::<34>::new();
        let ebp = fr.at() + 0x88;
        j::malloc_and_read_gamecode(&mut mem(), ebp, lang)
    }
    /// `(c)`: the customize screen's fade band.
    MenuKaizou_DrawFadeBand_008d6340 @ 0x008d_6340 => extern "C" fn menu_kaizou_draw_fade_band(c: u32) -> u32 {
        j::kaizou_fade_band(&mut mem(), c)
    }
}
