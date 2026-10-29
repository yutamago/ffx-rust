//! M5 stage 1, list `main`, round 3: the `Gu_` packet builders and the
//! debug `printf` (`ffx_sg::m5main3_gu`), the CD library's read queue
//! (`ffx_sg::m5main3_cd`), the debug menu's windows
//! (`ffx_sg::m5main3_menu`) and leftovers (`ffx_sg::m5main3_misc`).
//!
//! Threads: the main thread (the debug GUI runs from `Sg_MainDebug` and
//! `Gu_VSyncMain`; `Cd_QueueRead` from the game's file requests). The
//! shims keep no state; frames the originals hand their callees are
//! `StackFrame`s.

use super::{StackFrame, mem, ported};
use ffx_sg::m5main3_cd::bridge as cd;
use ffx_sg::m5main3_gu::bridge as gu;
use ffx_sg::m5main3_menu::bridge as menu;
use ffx_sg::m5main3_misc::bridge as misc;

ported! {
    // ---- batch 1 ----
    /// `Cd_QueueRead(file, buffer, callback, opts)` -> the request id.
    Cd_QueueRead @ 0x0076_b970 => extern "C" fn cd_queue_read(file: u32, buffer: u32, callback: u32, opts: u32) -> u32 {
        cd::queue_read(&mut mem(), file, buffer, callback, opts)
    }
    /// `(x, y, w, h, u, v, du, dv, skew)` -> the last y.
    Gu_DrawSprite_00846840 @ 0x0084_6840 => extern "C" fn gu_draw_sprite(x: u32, y: u32, w: u32, hh: u32, u: u32, v: u32, du: u32, dv: u32, skew: u32) -> u32 {
        gu::sprite(&mut mem(), x, y, w, hh, u, v, du, dv, skew)
    }
    /// `(control, dx, dy)` -> the GIF tag's address.
    Gu_DrawFrame_00846b10 @ 0x0084_6b10 => extern "C" fn gu_draw_frame(control: u32, dx: u32, dy: u32) -> u32 {
        gu::frame(&mut mem(), control, dx, dy)
    }
    /// `(event, n, req)`: DECI2's callback.
    Sg_Deci2Callback_0084ed10 @ 0x0084_ed10 => extern "C" fn deci2_callback(event: u32, n: u32, req: u32) -> u32 {
        misc::deci2_callback(&mut mem(), event, n, req)
    }
    /// `(window, msg, id; slots)`: the light stock bar; "%d" goes into
    /// the caller's `msg` word.
    SgGu_LightStockProc_0084e590 @ 0x0084_e590 => extern "C" fn light_stock_proc(window: u32, msg: u32, id: u32; slots: u32) -> u32 {
        menu::save_proc(&mut mem(), window, msg, id, slots)
    }
    /// `SgGu_BgColor()`: the colour through a 16-byte frame buffer.
    SgGu_BgColor @ 0x0084_e7c0 => extern "C" fn sggu_bg_color() {
        let mut f = StackFrame::<4>::new();
        menu::bg_color(&mut mem(), f.at());
    }
    /// `SgGu_LightEdit()`.
    SgGu_LightEdit @ 0x0084_e8c0 => extern "C" fn sggu_light_edit() {
        menu::light_edit(&mut mem());
    }
    /// `()`: the debug light table from the game's lights.
    SgGu_LightTableInit_0084ea00 @ 0x0084_ea00 => extern "C" fn light_table_init() -> u32 {
        let mut f = StackFrame::<4>::new();
        misc::light_table_init(&mut mem(), f.at())
    }
    /// `(light)` -> `Sg_LightSetDir`'s `eax`.
    SgGu_LightApply_0084ebd0 @ 0x0084_ebd0 => extern "C" fn light_apply(light: u32) -> u32 {
        let mut f = StackFrame::<4>::new();
        misc::light_apply(&mut mem(), light, f.at())
    }
    /// `(window)`: the main menu's page selector.
    Sg_MainMenuPagePopup_0084efd0 @ 0x0084_efd0 => extern "C" fn page_popup(window: u32) -> u32 {
        menu::page_popup(&mut mem(), window)
    }
    /// `(window, msg)` -> 0.
    Sg_MainMenuPagePicked_0084f230 @ 0x0084_f230 => extern "C" fn page_picked(window: u32, msg: u32) -> u32 {
        menu::page_picked(&mut mem(), window, msg)
    }
    /// `Sg_SelectMainMenu(window, page)`.
    Sg_SelectMainMenu @ 0x0084_f270 => extern "C" fn select_main_menu(window: u32, page: u32) -> u32 {
        menu::select_main_menu(&mut mem(), window, page)
    }
    /// `WFukaProc(window, msg)` -> 0.
    WFukaProc @ 0x0084_f490 => extern "C" fn wfuka_proc(window: u32, msg: u32) -> u32 {
        menu::fuka_proc(&mut mem(), window, msg)
    }
    /// `(list, sel)`: the current map's files.
    Sg_MapFileList_008506a0 @ 0x0085_06a0 => extern "C" fn map_file_list(list: u32, sel: u32) -> u32 {
        let mut f = StackFrame::<{ menu::COMMAND as usize / 4 }>::new();
        menu::file_list(&mut mem(), list, sel, f.at(), true)
    }
    /// `(list, sel)`: the map directory.
    Sg_MapDirList_00850770 @ 0x0085_0770 => extern "C" fn map_dir_list(list: u32, sel: u32) -> u32 {
        let mut f = StackFrame::<{ menu::COMMAND as usize / 4 }>::new();
        menu::file_list(&mut mem(), list, sel, f.at(), false)
    }
    /// `sg_mainmenu_mapname(window)`.
    sg_mainmenu_mapname @ 0x0085_0940 => extern "C" fn mainmenu_mapname(window: u32) -> u32 {
        menu::mainmenu_mapname(&mut mem(), window)
    }
    /// `sg_vfprintf(unused, fmt, args)` -> characters printed.
    sg_vfprintf @ 0x0085_11d0 => extern "C" fn sg_vfprintf(_unused: u32, fmt: u32, args: u32) -> u32 {
        gu::vfprintf(&mut mem(), fmt, args)
    }
    /// `Sg_MouseMain()` -> 0, or 1 on a read error.
    Sg_MouseMain @ 0x0085_1920 => extern "C" fn sg_mouse_main() -> u32 {
        misc::mouse_main(&mut mem())
    }
    /// `(camera, width, height, view)`: the movie viewer's projection.
    MovieViewer_Camera_00a43e60 @ 0x00a4_3e60 => extern "C" fn movie_camera(camera: u32, width: u32, height: u32, view: u32) {
        misc::movie_camera(&mut mem(), camera, width, height, view);
    }
}
