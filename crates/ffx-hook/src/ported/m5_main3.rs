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
use ffx_sg::m5main3_cam::bridge as cam;
use ffx_sg::m5main3_cd::bridge as cd;
use ffx_sg::m5main3_gu::bridge as gu;
use ffx_sg::m5main3_menu::bridge as menu;
use ffx_sg::m5main3_misc::bridge as misc;
use ffx_sg::m5main3_small::bridge as small;

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

    // ---- batch 2 ----
    /// `(window, msg, id, spin)`: a spin control's arrows.
    Gu_SpinMsg_00851b20 @ 0x0085_1b20 => extern "C" fn spin_msg(window: u32, msg: u32, id: u32, spin: u32) -> u32 {
        menu::spin_msg(&mut mem(), window, msg, id, spin)
    }
    /// `(window, spin, x, y, h, label, w, wv)`: a spin control's row.
    Gu_SpinAdd_00851c70 @ 0x0085_1c70 => extern "C" fn spin_add(window: u32, spin: u32, x: u32, y: u32, hh: u32, label: u32, w: u32, wv: u32) -> u32 {
        menu::spin_add(&mut mem(), window, spin, x, y, hh, label, w, wv)
    }
    /// `(window, msg, id)` -> 0: the mouth window.
    Gui_MouthProc_00851e70 @ 0x0085_1e70 => extern "C" fn mouth_proc(window: u32, msg: u32, id: u32) -> u32 {
        menu::mouth_proc(&mut mem(), window, msg, id)
    }
    /// `(window, msg, id)` -> 0: the physics window.
    Gui_PhysicsProc_00852080 @ 0x0085_2080 => extern "C" fn physics_proc(window: u32, msg: u32, id: u32; slots: u32) -> u32 {
        menu::physics_proc(&mut mem(), window, msg, id, slots)
    }
    /// `(window, msg, id; slots)` -> 0: the shadow window; the colour
    /// goes through the caller's `msg` word and the frame.
    Gui_ShadowProc_008523e0 @ 0x0085_23e0 => extern "C" fn shadow_proc(window: u32, msg: u32, id: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<{ menu::SHADOW_FRAME as usize / 4 }>::new();
        let ebp = f.at() + menu::SHADOW_FRAME;
        menu::shadow_proc(&mut mem(), window, msg, id, slots, ebp)
    }
    /// `()`: the camera onto the selected point.
    CamDebug_RectFromPoint_00941a30 @ 0x0094_1a30 => extern "C" fn cam_rect_from_point() -> u32 {
        let mut f = StackFrame::<9>::new();
        let ebp = f.at() + 0x24;
        cam::rect_from_point(&mut mem(), ebp)
    }
    /// `()`: the current list's points as `.float` lines.
    CamDebug_DumpPoints_00941ae0 @ 0x0094_1ae0 => extern "C" fn cam_dump_points() -> u32 {
        cam::dump_points(&mut mem())
    }
    /// `()`: the camera keeps its position, takes the point's y.
    CamDebug_RectKeepY_00942060 @ 0x0094_2060 => extern "C" fn cam_rect_keep_y() -> u32 {
        let mut f = StackFrame::<9>::new();
        let ebp = f.at() + 0x24;
        cam::rect_keep_y(&mut mem(), ebp)
    }
    /// `()`: the current list written to the path file.
    CamDebug_WritePoints_00942140 @ 0x0094_2140 => extern "C" fn cam_write_points() {
        let mut f = StackFrame::<3>::new();
        let ebp = f.at() + 0xc;
        cam::write_points(&mut mem(), ebp);
    }
    /// `(dst, m, v)` -> `rcuiCopyVector`'s `eax`.
    CamDebug_MatVec_00942260 @ 0x0094_2260 => extern "C" fn cam_mat_vec(dst: u32, m: u32, v: u32) -> u32 {
        let mut f = StackFrame::<4>::new();
        cam::mat_vec(&mut mem(), dst, m, v, f.at())
    }
    /// `set_path_name(window, ms, controls, page)`.
    set_path_name @ 0x0094_2360 => extern "C" fn set_path_name(window: u32, ms: u32, controls: u32, page: u32) {
        cam::set_path_name(&mut mem(), window, ms, controls, page);
    }

    // ---- batch 3 (list small) ----
    /// `(map)` -> the head.
    ClusterMap_clear_0069a970 @ 0x0069_a970 => extern "thiscall" fn cluster_map_clear(map: u32) -> u32 {
        small::cluster_map_clear(&mut mem(), map)
    }
    /// `mspace_memalign(m, align, n)` -> the block.
    mspace_memalign @ 0x006e_c8d0 => extern "C" fn mspace_memalign(m: u32, align: u32, n: u32) -> u32 {
        small::mspace_memalign(&mut mem(), m, align, n)
    }
    /// `()` -> the decoder's `eax`.
    MovieViewer_Audio_0077c360 @ 0x0077_c360 => extern "C" fn viewer_audio() -> u32 {
        small::viewer_audio(&mut mem())
    }
    /// `()` -> the cursor or the viewer.
    MovieViewer_Camera_0077c470 @ 0x0077_c470 => extern "C" fn viewer_camera() -> u32 {
        small::viewer_camera(&mut mem())
    }
    /// `()` -> 0 or 1.
    TexAnim_GsSinkReady_0077cf80 @ 0x0077_cf80 => extern "C" fn gs_sink_ready() -> u32 {
        small::gs_sink_ready(&mut mem())
    }
    /// `(q, b)`: the TRXDIR quadword.
    TexAnim_SetTrxDir_0077f0e0 @ 0x0077_f0e0 => extern "C" fn trxdir(q: u32, b: u32) -> u32 {
        small::trxdir(&mut mem(), q, b)
    }
    /// `(q, x, y, dir)`: the TRXPOS quadword.
    TexAnim_SetTrxPos_0077f100 @ 0x0077_f100 => extern "C" fn trxpos(q: u32, x: u32, y: u32, dir: u32) -> u32 {
        small::trxpos(&mut mem(), q, x, y, dir)
    }
    /// `(q, b, c)`: the TRXREG quadword.
    TexAnim_SetTrxReg_0077f140 @ 0x0077_f140 => extern "C" fn trxreg(q: u32, b: u32, c: u32) -> u32 {
        small::trxreg(&mut mem(), q, b, c)
    }
    /// `Ch_BlkDisposeHandle(handle)`.
    Ch_BlkDisposeHandle @ 0x0082_59f0 => extern "C" fn blk_dispose_handle(handle: u32) -> u32 {
        small::blk_dispose_handle(&mut mem(), handle)
    }
    /// `Ch_ChrMemHeapSet()`.
    Ch_ChrMemHeapSet @ 0x0082_6230 => extern "C" fn chr_mem_heap_set() -> u32 {
        small::chr_mem_heap_set(&mut mem())
    }
    /// `Ch_FunctionHistoryDump()`.
    Ch_FunctionHistoryDump @ 0x0082_69e0 => extern "C" fn function_history_dump() -> u32 {
        small::function_history_dump(&mut mem())
    }
    /// `jmp Ch_MgrpRead`.
    Ch_MgrpRead_thunk @ 0x0083_6a40 => extern "C" fn mgrp_read_thunk(id: u32, n: u32) -> u32 {
        small::thunk(&mut mem(), small::MGRP_READ, &[id, n])
    }
    /// `jmp Sg_SetKeepFps`.
    Sg_SetKeepFps_thunk_00839ca0 @ 0x0083_9ca0 => extern "C" fn set_keep_fps_thunk(v: u32) -> u32 {
        small::thunk(&mut mem(), small::SET_KEEP_FPS, &[v])
    }
    /// `chr_anim_win(chr)`.
    chr_anim_win @ 0x0093_e510 => extern "C" fn chr_anim_win(chr: u32) -> u32 {
        small::chr_anim_win(&mut mem(), chr)
    }
    /// `()`: `GlobalMemoryStatusEx` on a frame buffer.
    Mem_QueryStatus_009437c0 @ 0x0094_37c0 => extern "C" fn memory_status() -> u32 {
        let mut f = StackFrame::<16>::new();
        small::memory_status(&mut mem(), f.at())
    }
    /// `(ok, name)`: stops on a failed read.
    HddRead_Check_00a42f50 @ 0x00a4_2f50 => extern "C" fn read_check(ok: u32, name: u32) {
        small::read_check(&mut mem(), ok, name);
    }
}
