//! Continuation round 6 of list `h4-glue-other`, batch 8: the movie
//! skip prompt's and the loading screen's draws.
//!
//! The prompt (`[g_PhyreScene + 0x10de0]`) has a visibility byte `+0x14`
//! (`SkipPrompt_show_006e78a0` sets it), its alpha `+0x10`, the variants
//! `+0x20`/`+0x3c` and the mesh instances `+0x30`, `+0x4c`, `+0x68`.

use ffx_bridge::x87::load;
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::pass;
use crate::h4_h4::FIND_PARAMETER;
use crate::h4_h5::SET_VALUE;

/// `Phyre_PRendering_PRenderer_beginScene(renderer; clear)` (thiscall,
/// ret 4).
pub const BEGIN_SCENE: u32 = 0x005b_5e30;
/// `Phyre_PRendering_PRenderer_endScene(renderer; flag)` (thiscall, ret 4).
pub const END_SCENE: u32 = 0x005b_6950;
/// `PArray_Data_004d45a0(array)` (thiscall): a material list's entries.
pub const ARRAY_DATA: u32 = 0x004d_45a0;
/// `renderMeshInstanceForPass_005b9a70(renderer; instance)` (thiscall,
/// ret 4).
pub const RENDER_FOR_PASS: u32 = 0x005b_9a70;
/// `Phyre_PRendering_PRendererBase_renderMeshInstance(renderer; instance,
/// materials, 0, pass)` (thiscall, ret 0x10).
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
/// `"ColorScale"` (`0x00b47a48`).
pub const COLOR_SCALE: u32 = 0x00b4_7a48;
/// The pass the prompt sets on the renderer's `+0x48` (`0x00ce6e94`).
pub const PROMPT_PASS: u32 = 0x00ce_6e94;
/// Bytes of [`skip_prompt_render`]'s frame: the colour (`ebp - 0x10`).
pub const PROMPT_FRAME: u32 = 0x10;

/// The colour at `frame` set as `ColorScale` (a parameter of `variant`)
/// on the first material of the instance `instance`.
fn set_color(h: &mut impl Host, frame: u32, instance: u32, variant: u32) {
    let materials: u32 = h.read(instance.wrapping_add(0x10));
    let entries = h.call_sym(ARRAY_DATA, Conv::Thiscall(materials), &[]);
    let name = h.rebase(COLOR_SCALE);
    let p = h.call_sym(FIND_PARAMETER, Conv::Thiscall(variant), &[name]);
    let material: u32 = h.read(entries);
    let buffer: u32 = h.read(material.wrapping_add(4));
    h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[p, frame]);
}

/// `SkipPrompt_render_006e7f00(this; renderer, mode)` (thiscall, ret 8;
/// from `PhyreScene_render`; `frame` its colour at `ebp - 0x10`): when
/// shown (`+0x14`) with a renderer and its three instances, a scene
/// begun, the renderer's pass `+0x48` set, the colour (1, 1, 1, alpha
/// `+0x10`) put on the button instance (`mode` 0: `+0x30` with variant
/// `+0x20`; 1: `+0x4c` with `+0x3c`) which is drawn, then on `+0x68`,
/// drawn with the renderer's pass `+0x13c`, and the scene ended. The
/// original's `eax` is whatever its last call left (`PhyreScene_render`
/// does not read it).
pub fn skip_prompt_render(h: &mut impl Host, frame: u32, this: u32, renderer: u32, mode: u32) {
    if h.read::<u8>(this.wrapping_add(0x14)) == 0
        || renderer == 0
        || h.read::<u32>(this.wrapping_add(0x30)) == 0
        || h.read::<u32>(this.wrapping_add(0x4c)) == 0
        || h.read::<u32>(this.wrapping_add(0x68)) == 0
    {
        return;
    }
    h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[0]);
    let one = 1.0f32.to_bits();
    h.write(frame, one);
    h.write(frame.wrapping_add(4), one);
    let pass_at = h.rebase(PROMPT_PASS);
    h.write(renderer.wrapping_add(0x48), pass_at);
    h.write(frame.wrapping_add(8), one);
    let alpha = pass(h.read(this.wrapping_add(0x10)));
    h.write(frame.wrapping_add(0xc), alpha);
    let button = match mode {
        0 => Some((0x30u32, 0x20u32)),
        1 => Some((0x4c, 0x3c)),
        _ => None,
    };
    if let Some((instance_at, variant_at)) = button {
        let instance: u32 = h.read(this.wrapping_add(instance_at));
        let variant: u32 = h.read(this.wrapping_add(variant_at));
        set_color(h, frame, instance, variant);
        let instance: u32 = h.read(this.wrapping_add(instance_at));
        h.call_sym(RENDER_FOR_PASS, Conv::Thiscall(renderer), &[instance]);
    }
    let instance: u32 = h.read(this.wrapping_add(0x68));
    let variant: u32 = h.read(this.wrapping_add(0x20));
    set_color(h, frame, instance, variant);
    let instance: u32 = h.read(this.wrapping_add(0x68));
    let materials: u32 = h.read(instance.wrapping_add(0x10));
    let render_pass: u32 = h.read(renderer.wrapping_add(0x13c));
    h.call_sym(
        RENDER_MESH_INSTANCE,
        Conv::Thiscall(renderer),
        &[instance, materials, 0, render_pass],
    );
    h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1]);
}

// ---- the loading screen's draw

/// `Phyre_PRendering_PRenderer_setColorTarget(renderer; target, i)`
/// (thiscall, ret 8).
pub const SET_COLOR_TARGET: u32 = 0x005b_b130;
/// `Phyre_PRendering_PRenderer_setDepthStencilTarget(renderer; target)`
/// (thiscall, ret 4).
pub const SET_DEPTH_TARGET: u32 = 0x005b_b5b0;
/// `PostProcessManager_GetInstance()` (cdecl).
pub const POST_PROCESS_MANAGER: u32 = crate::h4_h2::POST_PROCESS_MANAGER;
/// `PostProcessManager_setNon16By9Viewport(manager; renderer, 0)`
/// (thiscall, ret 8).
pub const SET_VIEWPORT: u32 = 0x0069_8750;
/// `Phyre_PRendering_PRendererBase_setClearColor(renderer; r, g, b, a)`
/// (thiscall, ret 0x10).
pub const SET_CLEAR_COLOR: u32 = 0x005b_ada0;
/// The pass the loading screen sets on the renderer's `+0x48`
/// (`0x00ccc8dc`).
pub const LOADING_PASS: u32 = 0x00cc_c8dc;
/// The threshold the clear colour is compared with (`0x00b92194`, f32
/// 0.5).
pub const HALF: u32 = 0x00b9_2194;
/// Bytes of [`loading_screen_render`]'s frame: the clear colour (`ebp -
/// 0x10`, three floats).
pub const LOADING_FRAME: u32 = 0x10;

/// `LoadingScreen_render(this; renderer)` (`0x006e4ea0`, thiscall, ret 4;
/// `frame` its colour at `ebp - 0x10`): when showing (byte `+0` or `+1`,
/// or the mode `+0x10` at least 2), with textures (`+0x20` count), a tip
/// cluster `+0x2c` and `+0x30`: the back buffer and no depth target set,
/// the post-process manager's 16:9 viewport, the clear colour from the
/// manager (`+0x1238c`..`+0x12394`) kept in the frame; for a mode 0..=2
/// it is set (alpha 1.0) and the scene begun clearing, else begun without
/// clearing. The renderer's pass `+0x48` set; the background instance
/// `+0x38` drawn only when the colour is dark: red, green and blue all
/// at most the image's 0.5 (none NaN); the main instance
/// `+0x40` drawn and the scene ended. The original's `eax` is not read
/// (`PhyreScene_render`, `PhyreScene_renderLoadingScreen`; check-clobber).
pub fn loading_screen_render(h: &mut impl Host, frame: u32, this: u32, renderer: u32) {
    let showing = h.read::<u8>(this) != 0
        || h.read::<u8>(this.wrapping_add(1)) != 0
        || h.read::<i32>(this.wrapping_add(0x10)) >= 2;
    if !showing
        // no textures: the count without its top bit is 0
        || h.read::<u32>(this.wrapping_add(0x20)).trailing_zeros() >= 31
        || h.read::<u32>(this.wrapping_add(0x2c)) == 0
        || h.read::<u32>(this.wrapping_add(0x30)) == 0
    {
        return;
    }
    h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[0, 0]);
    h.call_sym(SET_DEPTH_TARGET, Conv::Thiscall(renderer), &[0]);
    let manager = h.call_sym(POST_PROCESS_MANAGER, Conv::Cdecl, &[]);
    h.call_sym(SET_VIEWPORT, Conv::Thiscall(manager), &[renderer, 0]);
    let manager = h.call_sym(POST_PROCESS_MANAGER, Conv::Cdecl, &[]);
    let mut color = [0u32; 3];
    for (k, c) in color.iter_mut().enumerate() {
        *c = pass(h.read(manager.wrapping_add(0x1_238c + 4 * k as u32)));
        h.write(frame.wrapping_add(4 * k as u32), *c);
    }
    let mode: i32 = h.read(this.wrapping_add(0x10));
    let clear = if (0..=2).contains(&mode) {
        h.call_sym(
            SET_CLEAR_COLOR,
            Conv::Thiscall(renderer),
            &[color[0], color[1], color[2], 1.0f32.to_bits()],
        );
        1
    } else {
        0
    };
    h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[clear]);
    let half = load(h.read(h.rebase(HALF)));
    let pass_at = h.rebase(LOADING_PASS);
    h.write(renderer.wrapping_add(0x48), pass_at);
    // red and green: `fcomp` + `test ah, 0x41; jp` skips on an even
    // parity, i.e. above (C0 = C3 = 0) or unordered (both set): on only
    // when below or equal. Blue: `fcomp [blue]` + `test ah, 1; jne` skips
    // when 0.5 < blue or unordered. All three at most 0.5.
    let red = load(h.read(frame));
    let green = load(h.read(frame.wrapping_add(4)));
    let blue = load(h.read(frame.wrapping_add(8)));
    if red <= half && green <= half && blue <= half {
        let background: u32 = h.read(this.wrapping_add(0x38));
        h.call_sym(RENDER_FOR_PASS, Conv::Thiscall(renderer), &[background]);
    }
    let instance: u32 = h.read(this.wrapping_add(0x40));
    let materials: u32 = h.read(instance.wrapping_add(0x10));
    let render_pass: u32 = h.read(renderer.wrapping_add(0x13c));
    h.call_sym(
        RENDER_MESH_INSTANCE,
        Conv::Thiscall(renderer),
        &[instance, materials, 0, render_pass],
    );
    h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1]);
}
