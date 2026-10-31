//! M5 stage 1, list `phyre-framework-input`, round "design-frames"
//! (`ffx_pother_b::m5p_framework_input_df`). The parity tests are in
//! `crates/ffx-oracle/tests/m5_design_frames.rs`.
//!
//! `Framerate` and `openFile` size their buffers at run time
//! ([`super::with_frame`]). `openFile` and `GetTime` are wrapper-owned:
//! the mod wrapper (`mods.rs` `file_open`) and the harness's clock
//! (`harness.rs` `time_now`, always installed) hold their addresses and
//! call these ports where they called the trampoline.
//!
//! Threads: `GetTime` runs on every thread that times something, file
//! opens on the loader threads too; the frames are per call.

use ffx_pother_b::m5p_framework_input_df::bridge as fd;

use super::{mem, ported, with_frame};

ported! {
    /// `PApplicationScriptProxy::Framerate(app, text)`.
    Phyre_PFramework_PApplicationScriptProxy_Framerate @ 0x0062_35d0 => extern "C" fn framerate(app: u32, text: u32) -> u32 {
        let bytes = fd::framerate_size(&mem(), text);
        with_frame(bytes, |buf| fd::framerate(&mut mem(), app, text, buf, bytes))
    }
}

/// The name `FFX_HOOK_DISABLE` and `FFX_HOOK_ONLY` know [`open_file`] by:
/// the mod wrapper owns `PStreamFileWin32::openFile`'s address.
#[cfg(not(ffx_host))]
pub(crate) const OPEN_FILE_NAME: &str = "Phyre_PSerialization_PStreamFileWin32_openFile_port";

/// Whether the mod wrapper runs [`open_file`].
#[cfg(not(ffx_host))]
pub(crate) fn open_file_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| super::wrapper_port_on(OPEN_FILE_NAME))
}

/// `PStreamFileWin32::openFile(file; path, mode, 3 unused)` (`0x00607f40`,
/// thiscall, ret 0x14) in Rust
/// (`ffx_pother_b::m5p_framework_input_df::bridge::open_file`).
#[cfg(not(ffx_host))]
pub(crate) fn open_file(file: u32, path: u32, mode: u32) -> u32 {
    match fd::open_file_sizes(&mut mem(), path) {
        None => fd::open_file(&mut mem(), file, path, mode, None),
        Some((len, wide, narrow)) => with_frame(wide, |w| {
            with_frame(narrow, |n| {
                fd::open_file(&mut mem(), file, path, mode, Some((len, w, n)))
            })
        }),
    }
}

/// The name [`get_time`] goes by (see [`OPEN_FILE_NAME`]): the harness's
/// clock wrapper owns `PTimerWin32::GetTime`'s address.
#[cfg(not(ffx_host))]
pub(crate) const GET_TIME_NAME: &str = "Phyre_PTimerWin32_GetTime_port";

/// Whether the clock wrapper runs [`get_time`].
#[cfg(not(ffx_host))]
pub(crate) fn get_time_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| super::wrapper_port_on(GET_TIME_NAME))
}

/// `PTimerWin32::GetTime()` (`0x0042fd80`, cdecl) -> seconds, in Rust
/// (`ffx_pother_b::m5p_framework_input_df::bridge::get_time`; the
/// FILETIME in a per-call frame).
#[cfg(not(ffx_host))]
pub(crate) fn get_time() -> f64 {
    let mut ft = super::StackFrame::<2>::new();
    fd::get_time(&mut mem(), ft.at())
}
