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
    // The wrapped originals WP6 left (m5-wp5, 2026-11-12): ffx-host's clock and
    // harness wrap these addresses (`crate::clock`, `crate::harness`) and
    // called the original's trampoline, so their bodies ran in Unicorn (gate
    // run 1: `Input_KeyHeld` 2.7 M blocks, `openFile` 2.7 M, `flip` 0.4 M, ...).
    // As table entries, `Table::wrap` keeps each as the wrapper's original
    // (`Exec::call_original`). The 32-bit game's hand-written hooks call the
    // same bridges (`harness::key_held`, `mods::file_open`, `flip_port`,
    // `present_flip_port`, `animate_port`, `mods::cd_file_size_pc`,
    // `loader_turn::loader_thread`). The parity tests are the bridges' own:
    // `Input_KeyHeld` in `m5_design_frames.rs`, `openFile` in
    // `m5p_framework_input.rs`, `flip` and `PresentFlip` in `m5p_render_b.rs`,
    // `animate` in `m5_main.rs`, `CDfileSize_PC` and the loader thread in
    // `m5_system_b.rs`.
    /// `Input_KeyHeld(devices; binding, device)` -> `eax` (the answer in
    /// `al`).
    Input_KeyHeld @ 0x0062_9d50 => extern "thiscall" fn key_held(devices: u32, binding: u32, device: u32) -> u32 {
        ffx_system::m5a_df::bridge::key_held(&super::mem(), devices, binding, device)
    }
    /// `PStreamFileWin32::openFile(file; path, mode, 3 unused)` -> `eax`.
    Phyre_PSerialization_PStreamFileWin32_openFile @ 0x0060_7f40 => extern "thiscall" fn open_file(file: u32, path: u32, mode: u32, a3: u32, a4: u32, a5: u32) -> u32 {
        use ffx_pother_b::m5p_framework_input_df::bridge as fd;
        let _ = (a3, a4, a5);
        match fd::open_file_sizes(&mut super::mem(), path) {
            None => fd::open_file(&mut super::mem(), file, path, mode, None),
            Some((len, wide, narrow)) => super::with_frame(wide, |w| {
                super::with_frame(narrow, |n| {
                    fd::open_file(&mut super::mem(), file, path, mode, Some((len, w, n)))
                })
            }),
        }
    }
    /// `PRenderer::flip(renderer)` -> `eax`.
    Phyre_PRendering_PRenderer_flip @ 0x005b_6af0 => extern "thiscall" fn flip(renderer: u32) -> u32 {
        let mut frame = super::StackFrame::<2>::new();
        ffx_prender::m5p_render_b_df::bridge::flip(&mut super::mem(), renderer, frame.at())
    }
    /// `Renderer_PresentFlip(renderer)` -> `al`.
    Renderer_PresentFlip @ 0x0059_3760 => extern "thiscall" fn present_flip(renderer: u32) -> u8 {
        ffx_prender::m5p_render_b_small::bridge::present_flip(&mut super::mem(), renderer)
    }
    /// `FFXApplication::animate(app)` -> `eax`.
    FFXApplication_animate @ 0x0042_f520 => extern "thiscall" fn animate(app: u32) -> u32 {
        super::ffx_application_animate(app)
    }
    /// `CDfileSize_PC(number)` -> the size.
    CDfileSize_PC @ 0x00a4_28a0 => extern "C" fn cd_file_size_pc(number: u32) -> u32 {
        ffx_system::m5b_hdd::bridge::file_size_pc(&mut super::mem(), number)
    }
    /// The `AsyncLoadManager` thread's body (its argument unread) -> `eax`.
    AsyncLoadManager_threadMain_00691360 @ 0x0069_1360 => extern "C" fn loader_thread_main(arg: u32) -> u32 {
        let _ = arg;
        ffx_system::m5b_async::bridge::thread_main(&mut super::mem())
    }
    // `Rng_ClockSeed` (m5-wp5d, 2026-11-12): in the 32-bit game the
    // harness's wrapper (`harness::clock_seed`) holds the address and runs
    // the same port; without this entry a boot without Unicorn stopped at
    // `MsRandMize`'s call (the harness registers its fixed seed there only
    // with `FFX_HARNESS_SEED`). Parity: `m5_battle::clock_seed`'s tests.
    /// `Rng_ClockSeed()` -> `eax`: the XOR of the system time's bytes, plus 1.
    Rng_ClockSeed @ 0x0079_8950 => extern "C" fn rng_clock_seed() -> u32 {
        super::clock_seed_port()
    }
}
