//! M5 stage 1, list `phyre-render-b`, round "design-frames"
//! (`ffx_prender::m5p_render_b_df`, and the existing
//! `m5p_render_b_small::bridge::present_flip`). The parity tests are in
//! `crates/ffx-oracle/tests/m5_design_frames.rs` and
//! `crates/ffx-oracle/tests/m5p_render_b.rs` (`present_flip`).
//!
//! `Renderer_SetDisplayMode` sizes its buffer at run time
//! ([`super::with_frame`]). `PRenderer::flip` and `Renderer_PresentFlip`
//! are wrapper-owned: the harness's frame-boundary wrappers
//! (`harness.rs` `flip`, `present_flip`) hold their addresses and call
//! these ports where they called the trampoline; with the harness off,
//! `harness::install` puts the ports in as plain replacements.
//!
//! Threads: the display mode and the flip run on the main thread, the
//! present on the render thread; the frames are per call.

use ffx_prender::m5p_render_b_df::bridge as db;

use super::{StackFrame, mem, ported, with_frame};

ported! {
    /// `Renderer_SetDisplayMode(renderer; width, height)` (thiscall, ret 8).
    Renderer_SetDisplayMode @ 0x0058_df00 => extern "thiscall" fn set_display_mode(this: u32, width: u32, height: u32) -> u32 {
        let mut locals = StackFrame::<3>::new();
        let at = locals.at();
        let count = db::display_mode_count(&mut mem(), this, at);
        with_frame(db::display_modes_bytes(count), |modes| {
            db::set_display_mode(&mut mem(), this, width, height, at, count, modes)
        })
    }
}

/// The name `FFX_HOOK_DISABLE` and `FFX_HOOK_ONLY` know [`flip`] by: the
/// harness's wrapper owns `Phyre_PRendering_PRenderer_flip`'s address.
#[cfg(not(ffx_host))]
pub(crate) const FLIP_NAME: &str = "Phyre_PRendering_PRenderer_flip_port";

/// Whether the wrapper runs [`flip`].
#[cfg(not(ffx_host))]
pub(crate) fn flip_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| super::wrapper_port_on(FLIP_NAME))
}

/// `PRenderer::flip(renderer)` (`0x005b6af0`, thiscall) in Rust
/// (`ffx_prender::m5p_render_b_df::bridge::flip`).
#[cfg(not(ffx_host))]
pub(crate) fn flip(this: u32) -> u32 {
    let mut frame = StackFrame::<2>::new();
    db::flip(&mut mem(), this, frame.at())
}

/// The name [`present_flip`] goes by (see [`FLIP_NAME`]).
#[cfg(not(ffx_host))]
pub(crate) const PRESENT_FLIP_NAME: &str = "Renderer_PresentFlip_port";

/// Whether the wrapper runs [`present_flip`].
#[cfg(not(ffx_host))]
pub(crate) fn present_flip_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| super::wrapper_port_on(PRESENT_FLIP_NAME))
}

/// `Renderer_PresentFlip(renderer)` (`0x00593760`, thiscall) -> `al`, in
/// Rust (`ffx_prender::m5p_render_b_small::bridge::present_flip`).
#[cfg(not(ffx_host))]
pub(crate) fn present_flip(this: u32) -> u8 {
    ffx_prender::m5p_render_b_small::bridge::present_flip(&mut mem(), this)
}
