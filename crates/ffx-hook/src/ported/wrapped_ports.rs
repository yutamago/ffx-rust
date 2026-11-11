//! The ports of the wrapper-owned addresses that aren't `ported!` lines
//! (docs/host.md, "One source of truth"): in the 32-bit game a hand-written
//! hook holds each address and calls the port ([`super::wrapper_owned!`]
//! gives the game no hook from these lines); in `ffx-host` they are the
//! function table's entries. The parity tests are the ports' own:
//! `MsScreenProcess` in `crates/ffx-oracle/tests/battle_target.rs`,
//! `rcbgGuideUpdateEx` in `crates/ffx-oracle/tests/field.rs`,
//! `renderWorldCulledSorted` in `crates/ffx-oracle/tests/phyre_render.rs`.
//!
//! - `MsScreenProcess`: `gfx::live::cam_update` (the live editor's free
//!   camera around the port);
//! - `rcbgGuideUpdateEx`: `minimap::guide_update` (the minimap mod, which
//!   replaces the game's minimap when it is on);
//! - `renderWorldCulledSorted`: `world::render_world` (the draw capture's
//!   scene inputs before the port, `FFX_HOOK_DISABLE`'s
//!   `Phyre_PRendering_PRenderer_renderWorldCulledSorted_port`).

use super::wrapper_owned;

wrapper_owned! {
    /// `MsScreenProcess(camera)` -> `eax`.
    MsScreenProcess @ 0x007b_c110 => extern "C" fn ms_screen_process(camera: i32) -> u32 {
        super::ms_screen_process(camera)
    }
    /// `rcbgGuideUpdateEx(guide, show)` -> `eax` (`rcbgGuideUpdate` hands
    /// it on).
    rcbgGuideUpdateEx @ 0x0091_da80 => extern "C" fn rcbg_guide_update_ex(guide: u32, show: u32) -> u32 {
        super::rcbg_guide_update_ex(guide, show)
    }
    /// `PRenderer::renderWorldCulledSorted(renderer; world, camera, ctx,
    /// pass)` -> 0.
    Phyre_PRendering_PRenderer_renderWorldCulledSorted @ 0x009f_3c30 => extern "thiscall" fn render_world_culled_sorted(renderer: u32, world: u32, camera: u32, ctx: u32, pass: u32) -> u32 {
        super::culled_sorted(renderer, world, camera, ctx, pass)
    }
}
