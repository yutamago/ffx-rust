//! M5 stage 1, the leftovers of list `phyre-framework-input` (round left2)
//! (`ffx_pother_b::m5p_framework_input_left2`). The parity tests are in
//! `crates/ffx-oracle/tests/m5p_framework_input.rs` (`left2_*`).
//!
//! Threads: the screen-mode switch runs on the window's (main) thread, the
//! exception filter on whichever thread crashed; the frames the originals
//! hand to Win32 are per-call [`StackFrame`]s.

use ffx_pother_b::m5p_framework_input_left2::{self as lf, bridge as lb};

use super::{StackFrame, mem, ported};

ported! {
    /// `0x0062a0d0(out, v)`: `v` normalised.
    Phyre_Vector2_normalize_0062a0d0 @ 0x0062_a0d0 => extern "C" fn normalize2(out: u32, v: u32) -> u32 {
        lb::normalize2(&mut mem(), out, v)
    }
    /// `CrashDump_UnhandledExceptionFilter(pointers)`.
    CrashDump_UnhandledExceptionFilter @ 0x0062_68e0 => extern "stdcall" fn crash_filter(pointers: u32) -> u32 {
        let mut f = StackFrame::<3>::new();
        lb::crash_filter(&mut mem(), pointers, f.at())
    }
    /// `PWindowWin32Base::applyScreenMode()`.
    Phyre_PFramework_PWindowWin32Base_applyScreenMode @ 0x0062_bf00 => extern "C" fn apply_screen_mode() {
        let mut f = StackFrame::<{ lf::SCREEN_FRAME_WORDS as usize }>::new();
        lb::apply_screen_mode(&mut mem(), f.at());
    }
    /// The jump to `applyScreenMode` at `0x0062bef0`.
    Phyre_PFramework_PWindowWin32Base_applyScreenMode_jump_0062bef0 @ 0x0062_bef0 => extern "C" fn apply_screen_mode_jump() {
        let mut f = StackFrame::<{ lf::SCREEN_FRAME_WORDS as usize }>::new();
        lb::apply_screen_mode(&mut mem(), f.at());
    }
}
