//! `ffx-hook`'s `ported!` lists, compiled for `ffx-host`'s function table
//! (docs/host.md, "One source of truth").
//!
//! [`ported`] is `crates/ffx-hook/src/ported` itself, built with
//! `cfg(ffx_host)`: every `ported!` line becomes a [`Hook`] (name,
//! address, convention, stack words, and an adapter from the caller's
//! words to the shim) instead of a thunk and a patch. The shims call
//! original code through `ffx_bridge::route`, which the executor routes to
//! its table, and read memory through `ffx_bridge::RawMem`, which is the
//! arena there. The crate-root names the shims use (`crate::rebase`,
//! `crate::log`, the harness's hooks) are this file's.

use ffx_bridge::route::{Args, Conv, Ret};

/// One ported function, for the table.
#[derive(Debug, Clone, Copy)]
pub struct Hook {
    /// Name in `symbols/ffx.sym`.
    pub name: &'static str,
    /// Address in `FFX.exe` (its preferred base, where the arena maps it).
    pub address: u32,
    /// The original's convention.
    pub conv: Conv,
    /// Stack words its caller pushes (register arguments not counted).
    pub words: usize,
    /// The shim, on the caller's words.
    pub run: fn(&Args) -> Ret,
}

// The shims as ffx-hook builds them for the 32-bit game, where these
// lints are allowed crate-wide (`cast_possible_truncation`) or never fire.
#[allow(
    dead_code,
    unused_imports,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::fn_to_numeric_cast,
    clippy::many_single_char_names,
    clippy::fn_to_numeric_cast_with_truncation,
    clippy::ptr_as_ptr,
    clippy::unnecessary_cast,
    clippy::too_many_arguments
)]
#[path = "../../ffx-hook/src/ported/mod.rs"]
pub mod ported;

pub mod held;

/// Every port of the `ported!` lists.
#[must_use]
pub fn hooks() -> Vec<Hook> {
    ported::hooks()
}

/// The ports the table can run: [`hooks`] but the [`held::HELD`] ones
/// (they hand the game host addresses), with how many were held.
#[must_use]
pub fn table_hooks() -> (Vec<Hook>, usize) {
    let all = hooks();
    let n = all.len();
    let keep: Vec<Hook> = all
        .into_iter()
        .filter(|h| held::HELD.binary_search(&h.address).is_err())
        .collect();
    let held = n - keep.len();
    (keep, held)
}

/// `FFX.exe` addresses are game addresses in the arena: no rebasing.
#[must_use]
pub fn rebase(address: u32) -> usize {
    address as usize
}

/// The hook's `FFX_HOOK_ONLY`: not used here (every port is in the table).
#[must_use]
pub fn hook_only() -> Option<Vec<String>> {
    None
}

/// The hook's log: stderr here.
pub mod log {
    /// One line.
    pub fn log(line: &str) {
        use std::io::Write;
        let _ = writeln!(std::io::stderr(), "{line}");
    }
}

/// The harness's hooks into the ports: none in `ffx-host` yet (the
/// harness comes with the second process, docs/host.md).
pub mod harness {
    /// The fast-mode sleep of the start-up loop: none (the port sleeps as
    /// the original).
    #[must_use]
    pub fn boot_sleep() -> Option<u32> {
        None
    }

    /// The call log: off.
    pub fn log_call(_tag: &str, _value: u32) {}
}

/// The Iggy texture log: off.
pub mod iggy_log {
    /// A texture.
    pub fn note_texture(_: u32, _: u32, _: u32, _: u32, _: u32, _: u32) {}
    /// A vertex buffer.
    pub fn note_vertex_buffer(_: u32) {}
    /// A row texture.
    pub fn note_row_texture(_: u32, _: u32) {}
    /// A render state.
    pub fn note_render_state(_: u32, _: u32) {}
}

/// The loader's lockstep turn: off (no lockstep without the harness).
pub mod loader_turn {
    /// Before the render lock is taken.
    pub fn before_take(_this: u32, _owner: u32) {}
}
