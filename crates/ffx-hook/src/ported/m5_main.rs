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
}
