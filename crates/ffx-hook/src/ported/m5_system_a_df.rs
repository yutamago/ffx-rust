//! M5 stage 1, list `system-a`, round "design-frames"
//! (`ffx_system::m5a_df`). The parity tests are in
//! `crates/ffx-oracle/tests/m5_design_frames.rs`.
//!
//! `File_Probe` and the two script commands size their buffers at run
//! time ([`super::with_frame`]); the screenshot's locals are a per-call
//! [`StackFrame`]. `Input_KeyHeld` is wrapper-owned: the harness's key
//! wrapper (`harness.rs` `key_held`, always installed) holds its address
//! and calls this port where it called the trampoline.
//!
//! Threads: `File_Probe` runs on the async init job and the main thread,
//! the commands and the key query on the main thread; the frames are per
//! call.

use ffx_system::m5a_df::bridge as ad;

use super::{StackFrame, mem, ported, with_frame};

ported! {
    /// `File_Probe(path)` -> `al`.
    File_Probe @ 0x0060_7e00 => extern "C" fn file_probe(path: u32) -> u8 {
        match ad::probe_sizes(&mem(), path) {
            None => ad::file_probe(&mut mem(), path, None),
            Some((len, wide, narrow)) => with_frame(wide, |w| {
                with_frame(narrow, |n| ad::file_probe(&mut mem(), path, Some((len, w, n))))
            }),
        }
    }
    /// The input-log replay command `0x00624410(app, text)`.
    PApplicationScriptProxy_readInputLogCommand_00624410 @ 0x0062_4410 => extern "C" fn read_input_log_cmd(app: u32, text: u32) -> u32 {
        let bytes = ad::name_size(&mem(), text);
        with_frame(bytes, |buf| ad::read_input_log_cmd(&mut mem(), app, text, buf, bytes))
    }
    /// `PApplication::screenshot(app, text)` (`0x00624da0`).
    Phyre_PFramework_PApplication_screenshot_00624da0 @ 0x0062_4da0 => extern "C" fn screenshot(app: u32, text: u32) -> u32 {
        let mut frame = StackFrame::<{ (ad::SCREENSHOT_FRAME / 4) as usize }>::new();
        let at = frame.at();
        let bytes = ad::name_size(&mem(), text);
        with_frame(bytes, |buf| ad::screenshot(&mut mem(), app, text, buf, bytes, at))
    }
}

/// The name `FFX_HOOK_DISABLE` and `FFX_HOOK_ONLY` know [`key_held`] by:
/// the harness's key wrapper owns `Input_KeyHeld`'s address.
#[cfg(not(ffx_host))]
pub(crate) const KEY_HELD_NAME: &str = "Input_KeyHeld_port";

/// Whether the key wrapper runs [`key_held`].
#[cfg(not(ffx_host))]
pub(crate) fn key_held_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| super::wrapper_port_on(KEY_HELD_NAME))
}

/// `Input_KeyHeld(devices; binding, device)` (`0x00629d50`, thiscall, ret
/// 8) -> `eax` (the answer in `al`), in Rust
/// (`ffx_system::m5a_df::bridge::key_held`).
#[cfg(not(ffx_host))]
pub(crate) fn key_held(devices: u32, binding: u32, device: u32) -> u32 {
    ad::key_held(&mem(), devices, binding, device)
}
