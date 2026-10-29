//! M5 stage 1, list `main`: `FFXApplication`'s own methods and helpers
//! (`ffx_sg::m5main_app`), the main loop's debug side, the achievement
//! forwarder and the movie frame's draw environment
//! (`ffx_sg::m5main_dbg`).
//!
//! Threads: the main thread (the application, the main loop and its
//! debug side) and whichever thread unwinds into the `PhyreFree` thunk;
//! the shims keep no state, frames are per call.

use super::{StackFrame, mem, ported};
use ffx_sg::m5main_app::bridge as app;
use ffx_sg::m5main_dbg::bridge as dbg;
use ffx_sg::m5main_dbg2::bridge as dbg2;
use ffx_sg::m5main_init::bridge as init;
use ffx_sg::m5main_loop::{self as lp, bridge as main_loop};
use ffx_sg::m5main_mem::bridge as sgmem;
use ffx_sg::m5main_task::bridge as task;

ported! {
    // ---- FFXApplication ----
    /// `FFXApplication::FFXApplication(this)` -> `this`.
    FFXApplication_FFXApplication @ 0x0042_ef40 => extern "thiscall" fn ffx_application_ctor(this: u32) -> u32 {
        app::ctor(&mut mem(), this)
    }
    /// `jmp Phyre::PhyreFree` (the unwinding code's thunk).
    Phyre_PhyreFree_thunk_0042f140 @ 0x0042_f140 => extern "C" fn phyre_free_thunk(p: u32) -> u32 {
        app::phyre_free_thunk(&mut mem(), p)
    }
    /// `FFXApplication`'s scalar deleting destructor `(this; flags)`.
    FFXApplication_scalar_deleting_dtor @ 0x0042_f150 => extern "thiscall" fn ffx_application_delete(this: u32, flags: u32) -> u32 {
        app::deleting_dtor(&mut mem(), this, flags)
    }
    /// `Phyre_MemDup_0042f220(src, size)`.
    Phyre_MemDup_0042f220 @ 0x0042_f220 => extern "C" fn phyre_mem_dup(src: u32, size: u32) -> u32 {
        app::mem_dup(&mut mem(), src, size)
    }
    /// `ResetToLauncher()`.
    ResetToLauncher @ 0x0042_f4d0 => extern "C" fn reset_to_launcher() -> u32 {
        app::reset_to_launcher(&mut mem())
    }
    /// `FFXApplication::afterGraphicSync(this)`.
    FFXApplication_afterGraphicSync @ 0x0042_f4f0 => extern "thiscall" fn after_graphic_sync(this: u32) -> u32 {
        app::after_graphic_sync(&mut mem(), this)
    }
    /// `FFXApplication::updateGlobalTime(this)`.
    FFXApplication_updateGlobalTime_0042fa00 @ 0x0042_fa00 => extern "thiscall" fn update_global_time(this: u32) {
        app::update_global_time(&mut mem(), this);
    }
    /// `SaveData_PollRepeatInput_006ef320()`.
    SaveData_PollRepeatInput_006ef320 @ 0x006e_f320 => extern "C" fn poll_repeat_input() -> u32 {
        app::poll_repeat_input(&mut mem())
    }

    // ---- the main loop's debug side ----
    /// `SystemOnHost(command, out)`.
    SystemOnHost @ 0x0084_ede0 => extern "C" fn system_on_host(command: u32, out: u32) -> i32 {
        dbg::system_on_host(&mut mem(), command, out)
    }
    /// `Sg_Capture()`: the name buffer in this frame.
    Sg_Capture @ 0x0083_a270 => extern "C" fn sg_capture() -> u32 {
        let mut f = StackFrame::<{ dbg::CAPTURE_BUF as usize / 4 }>::new();
        dbg::capture(&mut mem(), f.at())
    }
    /// `Sg_SaveResourceCheck()`.
    Sg_SaveResourceCheck @ 0x0083_fd20 => extern "C" fn sg_save_resource_check() -> u32 {
        dbg::save_resource_check(&mut mem())
    }
    /// `Sg_GsXYZF2L(x, y, z)`.
    Sg_GsXYZF2L @ 0x0084_21b0 => extern "C" fn sg_gs_xyzf2l(x: u32, y: u32, z: i32) {
        dbg::gs_xyzf2l(&mut mem(), x, y, z);
    }
    /// `Sg_DbDraw()`: the matrix, vector and point in this frame.
    Sg_DbDraw @ 0x0084_12c0 => extern "C" fn sg_db_draw() {
        let mut f = StackFrame::<{ dbg::DB_FRAME as usize / 4 }>::new();
        let ebp = f.at() + dbg::DB_FRAME;
        dbg::db_draw(&mut mem(), ebp);
    }
    /// `Sg_IdProject_00845030(out, v, m)`: the projected point in this
    /// frame.
    Sg_IdProject_00845030 @ 0x0084_5030 => extern "C" fn sg_id_project(out: u32, v: u32, m: u32) -> u32 {
        let mut f = StackFrame::<{ dbg::PROJECT_FRAME as usize / 4 }>::new();
        let ebp = f.at() + dbg::PROJECT_FRAME;
        dbg::id_project(&mut mem(), ebp, out, v, m)
    }
    /// `Load1TIM(tim)`.
    Load1TIM @ 0x0083_f800 => extern "C" fn load_1tim(tim: u32) -> u32 {
        dbg::load_1tim(&mut mem(), tim)
    }
    /// `Send_DebugFont()`.
    Send_DebugFont @ 0x0085_10b0 => extern "C" fn send_debug_font() -> u32 {
        dbg::send_debug_font(&mut mem())
    }
    /// `Sg_MainDebug()`.
    Sg_MainDebug @ 0x0082_07f0 => extern "C" fn sg_main_debug() {
        dbg::main_debug(&mut mem());
    }
    /// `achievementUnlockAchievement(id)`.
    achievementUnlockAchievement @ 0x0082_2380 => extern "C" fn achievement_unlock(id: u32) -> u32 {
        dbg::unlock_achievement(&mut mem(), id)
    }
    /// `Sg_DrawSetEnvironmentInitMovie()`.
    Sg_DrawSetEnvironmentInitMovie @ 0x0082_caa0 => extern "C" fn draw_set_environment_init_movie() -> u32 {
        dbg::draw_set_environment_init_movie(&mut mem())
    }

    // ---- the frame ----
    /// `Sg_MainLoop(dt)`: the frame's locals in this frame; the scaled
    /// `dt` back in the argument slot, as the original leaves it.
    Sg_MainLoop @ 0x0082_0ae0 => extern "C" fn sg_main_loop(dt: u32; slots: u32) {
        let mut f = StackFrame::<{ lp::MAIN_LOOP_FRAME as usize / 4 }>::new();
        let ebp = f.at() + lp::MAIN_LOOP_FRAME;
        main_loop::main_loop(&mut mem(), ebp, dt, slots);
    }
    /// `updateFFX(dt)`: 0 in the argument slot, as the original leaves it.
    updateFFX @ 0x0082_2840 => extern "C" fn update_ffx(dt: u32; slots: u32) -> u32 {
        main_loop::update_ffx(&mut mem(), dt, slots)
    }
    /// `jmp TkClearVsyncHandler` (`FFXApplication::exitApplication`'s).
    TkClearVsyncHandler_thunk_00822750 @ 0x0082_2750 => extern "C" fn clear_vsync_thunk() -> u32 {
        main_loop::clear_vsync_thunk(&mut mem())
    }

    // ---- FFXApplication's other methods and start-up (round 2) ----
    /// `FFXApplication::exitApplication(this)`.
    FFXApplication_exitApplication @ 0x0042_f600 => extern "thiscall" fn exit_application(this: u32) -> u32 {
        init::exit_application(&mut mem(), this)
    }
    /// `FFXApplication::exitScene(this)`.
    FFXApplication_exitScene @ 0x0042_f660 => extern "thiscall" fn exit_scene(this: u32) -> u32 {
        init::exit_scene(&mut mem(), this)
    }
    /// `FFXApplication::handleInputs(this)`.
    FFXApplication_handleInputs @ 0x0042_f770 => extern "thiscall" fn handle_inputs(this: u32) -> u32 {
        init::handle_inputs(&mut mem(), this)
    }
    /// `FFXApplication::pollInputAsync(this)`.
    FFXApplication_pollInputAsync @ 0x0042_f790 => extern "thiscall" fn poll_input_async(this: u32) -> u32 {
        init::poll_input_async(&mut mem(), this)
    }
    /// `FFXApplication::pollInputSync(this)`.
    FFXApplication_pollInputSync @ 0x0042_f7c0 => extern "thiscall" fn poll_input_sync(this: u32) -> u32 {
        init::poll_input_sync(&mut mem(), this)
    }
    /// `Game_InitSystems_00647fe0()`.
    Game_InitSystems_00647fe0 @ 0x0064_7fe0 => extern "C" fn init_systems() -> u32 {
        init::init_systems(&mut mem())
    }
    /// `Game_InitFileSystem()`: the FIOS parameters and the message
    /// string in this frame.
    Game_InitFileSystem @ 0x0067_95b0 => extern "C" fn init_file_system() -> u32 {
        let mut f = StackFrame::<{ init::INIT_FS_FRAME as usize / 4 }>::new();
        let ebp = f.at() + init::INIT_FS_FRAME;
        init::init_file_system(&mut mem(), ebp)
    }
    /// `FFXApplication::initApplication(this; argv, argc)`.
    FFXApplication_initApplication @ 0x0042_f7f0 => extern "thiscall" fn init_application(this: u32, argv: u32, argc: u32) -> u32 {
        init::init_application(&mut mem(), this, argv, argc)
    }
    /// `FFXApplication::initScene(this)`.
    FFXApplication_initScene @ 0x0042_f8e0 => extern "thiscall" fn init_scene(this: u32) -> u32 {
        init::init_scene(&mut mem(), this)
    }
    /// `FFXApplication::prePhyreInit(this)`.
    FFXApplication_prePhyreInit @ 0x0042_f8f0 => extern "thiscall" fn pre_phyre_init(this: u32) -> u32 {
        init::pre_phyre_init(&mut mem(), this)
    }
    /// `FFXApplication::render(this)`.
    FFXApplication_render @ 0x0042_f930 => extern "thiscall" fn render(this: u32) -> u32 {
        init::render(&mut mem(), this)
    }
    /// `FFXApplication_ApplySettings()`.
    FFXApplication_ApplySettings @ 0x0042_f9b0 => extern "C" fn apply_settings() -> u32 {
        init::apply_settings(&mut mem())
    }
    /// `PostReset()`.
    PostReset @ 0x0042_fa70 => extern "C" fn post_reset() -> u32 {
        init::post_reset(&mut mem())
    }
    /// `Sg_PlayerSetAnimatedMaterials_00829c00(a0..a9)`.
    Sg_PlayerSetAnimatedMaterials_00829c00 @ 0x0082_9c00 => extern "C" fn player_animated_materials(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32) -> u32 {
        init::player_animated_materials(&mut mem(), [a0, a1, a2, a3, a4, a5, a6, a7, a8, a9])
    }

    // ---- the task layer, the CD callbacks, the debug printfs (round 2) ----
    /// `sa_printf_ring_0076b860(fmt, a[16])`: the argument copies in this
    /// frame.
    sa_printf_ring_0076b860 @ 0x0076_b860 => extern "C" fn printf_ring(fmt: u32, a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32, a12: u32, a13: u32, a14: u32, a15: u32) -> u32 {
        let mut f = StackFrame::<{ task::RING_FRAME as usize / 4 }>::new();
        let ebp = f.at() + task::RING_FRAME;
        let a = [a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15];
        task::ring_printf(&mut mem(), ebp, fmt, a)
    }
    /// `task_wait()`.
    task_wait @ 0x0077_a4e0 => extern "C" fn task_wait() -> u32 {
        task::warn(&mut mem(), task::MSG_TASK_WAIT)
    }
    /// `task_init_asm()`.
    task_init_asm @ 0x0077_a4c0 => extern "C" fn task_init_asm() -> u32 {
        task::warn(&mut mem(), task::MSG_TASK_INIT_ASM)
    }
    /// `sa_task_init2(id, record, flag)`.
    sa_task_init2 @ 0x0077_a1d0 => extern "C" fn task_init2(id: u32, record: u32, flag: u32) -> u32 {
        task::task_init2(&mut mem(), id, record, flag)
    }
    /// `sa_task_init(id)`.
    sa_task_init @ 0x0077_a1a0 => extern "C" fn task_init(id: u32) -> u32 {
        task::task_init(&mut mem(), id)
    }
    /// `read_chgname_init_00a43130()`.
    read_chgname_init_00a43130 @ 0x00a4_3130 => extern "C" fn chgname_init() {
        task::chgname_init(&mut mem());
    }
    /// `sa_printf_0076de70(fmt, ...)`: printf2 gets the address of the
    /// caller's words after `fmt`.
    sa_printf_0076de70 @ 0x0076_de70 => extern "C" fn sa_printf(fmt: u32; slots: u32) -> u32 {
        task::printf(&mut mem(), fmt, slots)
    }
    /// `printf_cut()`.
    printf_cut @ 0x0076_de90 => extern "C" fn printf_cut() -> u32 {
        task::printf_cut(&mut mem())
    }
    /// `read_thread_init()`.
    read_thread_init @ 0x0076_e630 => extern "C" fn read_thread_init() -> u32 {
        task::read_thread_init(&mut mem())
    }
    /// `sa_read_callback(req)`.
    sa_read_callback @ 0x0076_e640 => extern "C" fn read_callback(req: u32) -> u32 {
        task::read_callback(&mut mem(), req)
    }
    /// `sa_camera_set()`.
    sa_camera_set @ 0x0076_f0f0 => extern "C" fn camera_set() -> u32 {
        task::warn(&mut mem(), task::MSG_CAMERA_SET)
    }
    /// `sa_cd_callback_init(flag)`: the closing line in the argument slot,
    /// as the original leaves it.
    sa_cd_callback_init @ 0x0076_f100 => extern "C" fn cd_callback_init(flag: u32; slots: u32) -> u32 {
        task::cd_callback_init(&mut mem(), flag, slots)
    }
    /// `sa_warn_0076f170()`.
    sa_warn_0076f170 @ 0x0076_f170 => extern "C" fn warn_0076f170() -> u32 {
        task::warn(&mut mem(), task::MSG_0076F170)
    }
    /// `movie_reset_0076f3b0()`.
    movie_reset_0076f3b0 @ 0x0076_f3b0 => extern "C" fn movie_reset() -> u32 {
        task::movie_reset(&mut mem())
    }
    /// `movie_end_00770340()`.
    movie_end_00770340 @ 0x0077_0340 => extern "C" fn movie_end() -> u32 {
        task::movie_end(&mut mem())
    }
    /// `Ms_BattlesSinceEvent_007722a0()`.
    Ms_BattlesSinceEvent_007722a0 @ 0x0077_22a0 => extern "C" fn battles_since_event() -> u32 {
        task::battles_since_event(&mut mem())
    }
    /// `jmp MsGetSaveBattleCount`.
    MsGetSaveBattleCount_thunk_00772310 @ 0x0077_2310 => extern "C" fn battle_count_thunk() -> u32 {
        task::battle_count_thunk(&mut mem())
    }
    /// `jmp MsGetGIL`.
    MsGetGIL_thunk_00772550 @ 0x0077_2550 => extern "C" fn gil_thunk() -> u32 {
        task::gil_thunk(&mut mem())
    }
    /// `GetWord_012fbbf4_00820520()`.
    GetWord_012fbbf4_00820520 @ 0x0082_0520 => extern "C" fn get_012fbbf4() -> u32 {
        task::get_012fbbf4(&mut mem())
    }
    /// `task_create_asm()`.
    task_create_asm @ 0x0077_a4b0 => extern "C" fn task_create_asm() -> u32 {
        task::task_create_asm(&mut mem())
    }
    /// `jmp task_create_asm` (after a frame).
    task_create_asm_thunk_0077a320 @ 0x0077_a320 => extern "C" fn task_create_asm_thunk() -> u32 {
        task::task_create_asm_thunk(&mut mem())
    }
    /// `task_create_in_group(group, a, b, c)`.
    task_create_in_group @ 0x0077_a390 => extern "C" fn task_create_in_group(group: u32, a: u32, b: u32, c: u32) -> u32 {
        task::task_create_in_group(&mut mem(), group, a, b, c)
    }
    /// `sa_task_run_in_group_0077a420(group)`.
    sa_task_run_in_group_0077a420 @ 0x0077_a420 => extern "C" fn task_run_in_group(group: u32) -> u32 {
        task::task_run_in_group(&mut mem(), group)
    }
    /// `sa_task_delete_in_group_0077a3f0(group)`.
    sa_task_delete_in_group_0077a3f0 @ 0x0077_a3f0 => extern "C" fn task_delete_in_group(group: u32) -> u32 {
        task::task_delete_in_group(&mut mem(), group)
    }
    /// `sa_decode(out, src)`: the decoder advances the caller's `src`
    /// slot.
    sa_decode @ 0x00a4_3de0 => extern "C" fn sa_decode(out: u32, src: u32; slots: u32) -> u32 {
        task::decode(&mut mem(), out, src, slots)
    }
    /// Group 0's create entry `(a, b, c)`.
    sa_task_create_g0_00779f80 @ 0x0077_9f80 => extern "C" fn task_create_g0(a: u32, b: u32, c: u32) -> u32 {
        task::task_create_group(&mut mem(), 0, a, b, c)
    }
    /// Group 1's create entry `(a, b, c)`.
    sa_task_create_g1_00779fa0 @ 0x0077_9fa0 => extern "C" fn task_create_g1(a: u32, b: u32, c: u32) -> u32 {
        task::task_create_group(&mut mem(), 1, a, b, c)
    }
    /// Group 2's create entry `(a, b, c)`.
    sa_task_create_g2_00779fc0 @ 0x0077_9fc0 => extern "C" fn task_create_g2(a: u32, b: u32, c: u32) -> u32 {
        task::task_create_group(&mut mem(), 2, a, b, c)
    }
    /// Group 3's create entry `(a, b, c)`.
    sa_task_create_g3_00779fe0 @ 0x0077_9fe0 => extern "C" fn task_create_g3(a: u32, b: u32, c: u32) -> u32 {
        task::task_create_group(&mut mem(), 3, a, b, c)
    }
    /// Group 4's create entry `(a, b, c)`.
    sa_task_create_g4_0077a000 @ 0x0077_a000 => extern "C" fn task_create_g4(a: u32, b: u32, c: u32) -> u32 {
        task::task_create_group(&mut mem(), 4, a, b, c)
    }
    /// Group 5's create entry `(a, b, c)`.
    sa_task_create_g5_0077a020 @ 0x0077_a020 => extern "C" fn task_create_g5(a: u32, b: u32, c: u32) -> u32 {
        task::task_create_group(&mut mem(), 5, a, b, c)
    }
    /// Group 6's create entry `(a, b, c)`.
    sa_task_create_g6_0077a040 @ 0x0077_a040 => extern "C" fn task_create_g6(a: u32, b: u32, c: u32) -> u32 {
        task::task_create_group(&mut mem(), 6, a, b, c)
    }
    /// Group 0's run entry.
    sa_task_run_g0_0077a080 @ 0x0077_a080 => extern "C" fn task_run_g0() -> u32 {
        task::task_run_group(&mut mem(), 0)
    }
    /// Group 1's run entry.
    sa_task_run_g1_0077a090 @ 0x0077_a090 => extern "C" fn task_run_g1() -> u32 {
        task::task_run_group(&mut mem(), 1)
    }
    /// Group 2's run entry.
    sa_task_run_g2_0077a0a0 @ 0x0077_a0a0 => extern "C" fn task_run_g2() -> u32 {
        task::task_run_group(&mut mem(), 2)
    }
    /// Group 3's run entry.
    sa_task_run_g3_0077a0b0 @ 0x0077_a0b0 => extern "C" fn task_run_g3() -> u32 {
        task::task_run_group(&mut mem(), 3)
    }
    /// Group 4's run entry.
    sa_task_run_g4_0077a0c0 @ 0x0077_a0c0 => extern "C" fn task_run_g4() -> u32 {
        task::task_run_group(&mut mem(), 4)
    }
    /// Group 5's run entry.
    sa_task_run_g5_0077a0d0 @ 0x0077_a0d0 => extern "C" fn task_run_g5() -> u32 {
        task::task_run_group(&mut mem(), 5)
    }
    /// Group 6's run entry.
    sa_task_run_g6_0077a0e0 @ 0x0077_a0e0 => extern "C" fn task_run_g6() -> u32 {
        task::task_run_group(&mut mem(), 6)
    }
    /// Group 7's run entry.
    sa_task_run_g7_0077a0f0 @ 0x0077_a0f0 => extern "C" fn task_run_g7() -> u32 {
        task::task_run_group(&mut mem(), 7)
    }
    /// Group 0's delete entry.
    sa_task_delete_g0_0077a100 @ 0x0077_a100 => extern "C" fn task_delete_g0() -> u32 {
        task::task_delete_group(&mut mem(), 0)
    }
    /// Group 1's delete entry.
    sa_task_delete_g1_0077a110 @ 0x0077_a110 => extern "C" fn task_delete_g1() -> u32 {
        task::task_delete_group(&mut mem(), 1)
    }
    /// Group 2's delete entry.
    sa_task_delete_g2_0077a120 @ 0x0077_a120 => extern "C" fn task_delete_g2() -> u32 {
        task::task_delete_group(&mut mem(), 2)
    }
    /// Group 3's delete entry.
    sa_task_delete_g3_0077a130 @ 0x0077_a130 => extern "C" fn task_delete_g3() -> u32 {
        task::task_delete_group(&mut mem(), 3)
    }
    /// Group 4's delete entry.
    sa_task_delete_g4_0077a140 @ 0x0077_a140 => extern "C" fn task_delete_g4() -> u32 {
        task::task_delete_group(&mut mem(), 4)
    }
    /// Group 5's delete entry.
    sa_task_delete_g5_0077a150 @ 0x0077_a150 => extern "C" fn task_delete_g5() -> u32 {
        task::task_delete_group(&mut mem(), 5)
    }
    /// Group 6's delete entry.
    sa_task_delete_g6_0077a160 @ 0x0077_a160 => extern "C" fn task_delete_g6() -> u32 {
        task::task_delete_group(&mut mem(), 6)
    }
    /// Group 7's delete entry.
    sa_task_delete_g7_0077a170 @ 0x0077_a170 => extern "C" fn task_delete_g7() -> u32 {
        task::task_delete_group(&mut mem(), 7)
    }

    // ---- the debug captures and the ID draw (round 2) ----
    /// `Sg_CaptureToFile(name)`: the name buffer in this frame.
    Sg_CaptureToFile @ 0x0083_a690 => extern "C" fn capture_to_file(name: u32) -> u32 {
        let mut f = StackFrame::<{ dbg2::CAPTURE_FILE_FRAME as usize / 4 }>::new();
        let ebp = f.at() + dbg2::CAPTURE_FILE_FRAME;
        dbg2::capture_to_file(&mut mem(), ebp, name)
    }
    /// `Sg_IdDrawTriangles_008447a0(verts, model, m, packet)`: the three
    /// points in this frame.
    Sg_IdDrawTriangles_008447a0 @ 0x0084_47a0 => extern "C" fn id_triangles(verts: u32, model: u32, m: u32, packet: u32) -> u32 {
        let mut f = StackFrame::<{ dbg2::ID_TRIANGLES_FRAME as usize / 4 }>::new();
        let ebp = f.at() + dbg2::ID_TRIANGLES_FRAME;
        dbg2::id_triangles(&mut mem(), ebp, verts, model, m, packet)
    }
    /// `Sg_IdDrawMain()`: the two matrices in this frame.
    Sg_IdDrawMain @ 0x0084_4a80 => extern "C" fn id_draw_main() {
        let mut f = StackFrame::<{ dbg2::ID_DRAW_FRAME as usize / 4 }>::new();
        let ebp = f.at() + dbg2::ID_DRAW_FRAME;
        dbg2::id_draw_main(&mut mem(), ebp);
    }

    // ---- the Sg_Mem heaps, the allocation wrappers, small getters (round 2) ----
    /// `Sg_MemRealloc(heap, kind, p, size)`.
    Sg_MemRealloc @ 0x0083_c480 => extern "C" fn mem_realloc(_heap: u32, _kind: u32, p: u32, size: u32) -> u32 {
        sgmem::mem_realloc(&mut mem(), p, size)
    }
    /// `SgReallocD(p, size)`.
    SgReallocD @ 0x0083_f030 => extern "C" fn sg_realloc_d(p: u32, size: u32) -> u32 {
        sgmem::sg_realloc(&mut mem(), 1, p, size)
    }
    /// `SgRealloc(p, size)`.
    SgRealloc @ 0x0083_f010 => extern "C" fn sg_realloc(p: u32, size: u32) -> u32 {
        sgmem::sg_realloc(&mut mem(), 0, p, size)
    }
    /// `Sg_ReadWholeFile_0084f1c0(name, out_size)`.
    Sg_ReadWholeFile_0084f1c0 @ 0x0084_f1c0 => extern "C" fn read_whole_file(name: u32, out_size: u32) -> u32 {
        sgmem::read_whole_file(&mut mem(), name, out_size)
    }
    /// `jmp user_malloc` (after a frame).
    user_malloc_thunk_0083f100 @ 0x0083_f100 => extern "C" fn malloc_thunk(size: u32) -> u32 {
        sgmem::malloc_thunk(&mut mem(), size)
    }
    /// `SgStrdupD(s)`.
    SgStrdupD @ 0x0083_f050 => extern "C" fn strdup(s: u32) -> u32 {
        sgmem::strdup(&mut mem(), s)
    }
    /// `sg_split_lines_00850590(text, out, filter)`: the line buffer in
    /// this frame.
    sg_split_lines_00850590 @ 0x0085_0590 => extern "C" fn split_lines(text: u32, out: u32, filter: u32) -> u32 {
        let mut f = StackFrame::<{ sgmem::SPLIT_FRAME as usize / 4 }>::new();
        let ebp = f.at() + sgmem::SPLIT_FRAME;
        sgmem::split_lines(&mut mem(), ebp, text, out, filter)
    }
    /// `sg_make_filelist(list, path, filter)`: the size and the probe
    /// name in this frame.
    sg_make_filelist @ 0x0085_09e0 => extern "C" fn make_filelist(list: u32, path: u32, filter: u32) -> u32 {
        let mut f = StackFrame::<{ sgmem::FILELIST_FRAME as usize / 4 }>::new();
        let ebp = f.at() + sgmem::FILELIST_FRAME;
        sgmem::make_filelist(&mut mem(), ebp, list, path, filter)
    }
    /// `SqsIopCommand3b_0081fc30(a, b)`.
    SqsIopCommand3b_0081fc30 @ 0x0081_fc30 => extern "C" fn sqs_command_3b(a: u32, b: u32) -> u32 {
        sgmem::sqs_command_3b(&mut mem(), a, b)
    }
    /// `GetWord_012fbc3c_00821740()`.
    GetWord_012fbc3c_00821740 @ 0x0082_1740 => extern "C" fn get_012fbc3c() -> u32 {
        sgmem::get_012fbc3c(&mut mem())
    }
    /// `Sg_SetResource9_0083ff00(on)`.
    Sg_SetResource9_0083ff00 @ 0x0083_ff00 => extern "C" fn set_resource9(on: u32) -> u32 {
        sgmem::set_resource9(&mut mem(), on)
    }
    /// `Sg_ToggleResource9_00821e40()`.
    Sg_ToggleResource9_00821e40 @ 0x0082_1e40 => extern "C" fn toggle_resource9() -> u32 {
        sgmem::toggle_resource9(&mut mem())
    }
    /// `Sg_MemInit(id, start, end)`.
    Sg_MemInit @ 0x0083_c260 => extern "C" fn mem_init(id: u32, start: u32, end: u32) -> u32 {
        sgmem::mem_init(&mut mem(), id, start, end)
    }
    /// `Sg_MemSetup_0083c200()`.
    Sg_MemSetup_0083c200 @ 0x0083_c200 => extern "C" fn mem_setup() -> u32 {
        sgmem::mem_setup(&mut mem())
    }
    /// `trophyCheckDefeatMonster(monster)`.
    trophyCheckDefeatMonster @ 0x0082_2760 => extern "C" fn trophy_check_defeat_monster(monster: u32) {
        sgmem::trophy_check_defeat_monster(&mut mem(), monster);
    }
    /// `Sg_MemMergeFree_0083c5b0(block)`.
    Sg_MemMergeFree_0083c5b0 @ 0x0083_c5b0 => extern "C" fn merge_free(block: u32) -> u32 {
        sgmem::merge_free(&mut mem(), block)
    }
    /// `Sg_MemCreateFree(heap, block, prev, next)`.
    Sg_MemCreateFree @ 0x0083_baf0 => extern "C" fn create_free(heap: u32, block: u32, prev: u32, next: u32) -> u32 {
        sgmem::create_free(&mut mem(), heap, block, prev, next)
    }
    /// `Sg_MemMoveWork(pp, moved)`.
    Sg_MemMoveWork @ 0x0083_c350 => extern "C" fn move_work(pp: u32, moved: u32) -> u32 {
        sgmem::move_work(&mut mem(), pp, moved)
    }
    /// `ClearHalf_0130074c_0082cc00()`.
    ClearHalf_0130074c_0082cc00 @ 0x0082_cc00 => extern "C" fn clear_half() -> u32 {
        sgmem::clear_half_0130074c(&mut mem())
    }
    /// `GetHalf_0130074c_0082cc10()`.
    GetHalf_0130074c_0082cc10 @ 0x0082_cc10 => extern "C" fn get_half() -> u32 {
        sgmem::get_half_0130074c(&mut mem())
    }
    /// `Sg_GetCameraFollow_0082d850()`.
    Sg_GetCameraFollow_0082d850 @ 0x0082_d850 => extern "C" fn get_camera_follow() -> u32 {
        sgmem::get_camera_follow(&mut mem())
    }
    /// `jmp MsCameraGetYaw` (`ST0`).
    Sg_CameraGetYaw @ 0x0083_f2d0 => extern "C" fn camera_get_yaw() -> f64 {
        sgmem::camera_get_yaw(&mut mem())
    }
    /// `Sg_CameraYawStore_0082d930()`.
    Sg_CameraYawStore_0082d930 @ 0x0082_d930 => extern "C" fn store_camera_yaw() {
        sgmem::store_camera_yaw(&mut mem());
    }
    /// `jmp Sg_PLightDispose`.
    Sg_PLightDispose_thunk @ 0x0083_1a30 => extern "C" fn plight_dispose_thunk() -> u32 {
        sgmem::plight_dispose_thunk(&mut mem())
    }
    /// `Sg_CaptureClear_0083a570()`.
    Sg_CaptureClear_0083a570 @ 0x0083_a570 => extern "C" fn capture_clear() {
        sgmem::capture_clear(&mut mem());
    }
    /// `Sg_CaptureSetFilter(on)`.
    Sg_CaptureSetFilter @ 0x0083_a650 => extern "C" fn capture_set_filter(on: u32) {
        sgmem::capture_set(&mut mem(), sgmem::CAPTURE_BLEND, on);
    }
    /// `Sg_CaptureSetMode(on)`.
    Sg_CaptureSetMode @ 0x0083_a670 => extern "C" fn capture_set_mode(on: u32) {
        sgmem::capture_set(&mut mem(), sgmem::CAPTURE_INTERLACED, on);
    }
}

/// The name `FFX_HOOK_DISABLE` and `FFX_HOOK_ONLY` know the
/// `FFXApplication::animate` port by: the harness's frame-boundary
/// wrapper (`harness::animate`) owns the address and calls the port
/// instead of its trampoline (two hooks can't share an address).
pub(crate) const ANIMATE_NAME: &str = "FFXApplication_animate_port";

/// Whether the wrapper runs the `FFXApplication::animate` port: not with
/// `FFX_HOOK_DISABLE` naming it or `replacements`, nor when
/// `FFX_HOOK_ONLY` leaves it out.
pub(crate) fn animate_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| {
        let off = std::env::var("FFX_HOOK_DISABLE").unwrap_or_default();
        let only = crate::hook_only().is_none_or(|names| names.iter().any(|n| n == ANIMATE_NAME));
        only && !off
            .split(',')
            .map(str::trim)
            .any(|n| n == "replacements" || n == ANIMATE_NAME)
    })
}

/// `FFXApplication::animate(app)` (`0x0042f520`, thiscall): the port,
/// for the harness's wrapper.
pub(crate) fn animate(app: u32) -> u32 {
    main_loop::animate(&mut mem(), app)
}
