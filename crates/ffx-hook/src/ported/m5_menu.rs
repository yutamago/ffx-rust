//! M5 stage 1, list `menu` (`ffx_menu::m5m_*`): the `Gu_` debug GUI's
//! frame (`Sg_MainDebug` > `Gu_Main`: mouse, hit tests, window drawing)
//! and the rest of the menu code the static call graph reaches.
//!
//! Threads: the main thread (the frame's update); the shims keep no
//! state, frames are per call.

use super::{StackFrame, mem, ported};
use ffx_menu::m5m_a::bridge as a;

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
}
