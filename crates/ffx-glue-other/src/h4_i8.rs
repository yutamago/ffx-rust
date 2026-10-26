//! Continuation round 7 of list `h4-glue-other`, batch 8: the distortion
//! effect's render.

use ffx_bridge::x87::load;
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::pass;
use crate::h4_i2::{BEGIN_SCENE, END_SCENE, RENDER_MESH_INSTANCE, SET_CLEAR_COLOR};
use crate::h4_i3::{
    COPY_BUFFER, FIND_PARAMETER, POST_PROCESS_MANAGER, SET_COLOR_TARGET, SET_VALUE, SURFACE_CTOR,
    SURFACE_DTOR, UV_SCALE_BIAS,
};

/// `Distortion_createRenderTarget(this)` (thiscall).
pub const CREATE_RENDER_TARGET: u32 = 0x006c_2350;
/// `PostProcessManager_copyDepthBuffer(manager; renderer, src, dst,
/// shader, values, 1, 1, 0, 0)` (thiscall, ret 0x24).
pub const COPY_DEPTH_BUFFER: u32 = 0x0069_2e90;
/// `Phyre_PRendering_PRendererBase_setViewport(renderer; x, y, w, h)`
/// (thiscall, ret 0x10).
pub const SET_VIEWPORT: u32 = 0x005b_c060;
/// `PostProcessManager_degradeResolution(manager; renderer)` (thiscall,
/// ret 4).
pub const DEGRADE_RESOLUTION: u32 = 0x0069_3530;
/// The depth copy's shader (`0x00ccc8ec`).
pub const DEPTH_COPY_SHADER: u32 = 0x00cc_c8ec;
/// The distortion pass (`0x00cdeb90`).
pub const DISTORTION_PASS: u32 = 0x00cd_eb90;
/// The strength limit, f64 (`0x00b3eb68`) and f32 (`0x00b34254`).
pub const LIMIT64: u32 = 0x00b3_eb68;
/// See [`LIMIT64`].
pub const LIMIT32: u32 = 0x00b3_4254;
/// Bytes of [`render_distortion`]'s frame, from `ebp - 0x30`: the render
/// surface (`+0`, 0x10 bytes), the four parameter words (`+0x10`) and the
/// strength (`+0x20`).
pub const DISTORTION_FRAME: u32 = 0x24;

/// The post-process manager's current buffer: `[[pm + 0x8c]]` unless its
/// byte `+4` is set.
fn current_buffer(h: &mut impl Host) -> u32 {
    let pm = h.call_sym(POST_PROCESS_MANAGER, Conv::Cdecl, &[]);
    let current: u32 = h.read(pm.wrapping_add(0x8c));
    if h.read::<u8>(current.wrapping_add(4)) == 0 {
        h.read(current)
    } else {
        0
    }
}

/// A render target's texture: `[t]` unless its byte `+4` is set.
fn texture(h: &impl Host, t: u32) -> u32 {
    if h.read::<u8>(t.wrapping_add(4)) == 0 {
        h.read(t)
    } else {
        0
    }
}

/// `Distortion_renderDistortion(this)` (`0x006c2dc0`, thiscall; `frame`
/// see [`DISTORTION_FRAME`]; the SEH frame is not kept): 0 unless on
/// (`+0x7c`); a pending skip (`+0x7d`) turns into a refresh (`+0x7d` 0,
/// `+0x7e` 1) and 0. Else the render target made if missing (`+0x7f`);
/// with the refresh flag the current buffer copied to `+0x28` (the flag
/// cleared), else copied with its depth through the depth shader and the
/// values (1, 1, 1, strength): `+0x78` when below the image's f64 limit
/// (ordered), else the image's f32 limit. The colour target set to
/// `+8` (its size as viewport, the resolution degraded, `UvScaleBias`
/// (1, 1, 0, 0) on the grid's shader) or none; the clear colour black and
/// transparent, the scene begun clearing, the distortion pass set, the
/// grid `+0x38` drawn and the scene ended. Returns the first failing
/// renderer call's result or `endScene`'s.
pub fn render_distortion(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let at = |off: u32| this.wrapping_add(off);
    if h.read::<u8>(at(0x7c)) == 0 {
        return 0;
    }
    if h.read::<u8>(at(0x7d)) != 0 {
        h.write(at(0x7d), 0x100u16);
        return 0;
    }
    if h.read::<u8>(at(0x7f)) == 0 {
        h.call_sym(CREATE_RENDER_TARGET, Conv::Thiscall(this), &[]);
    }
    let values = frame.wrapping_add(0x10);
    let one = 1.0f32.to_bits();
    if h.read::<u8>(at(0x7e)) != 0 {
        let src = current_buffer(h);
        let renderer: u32 = h.read(this);
        let dst: u32 = h.read(at(0x28));
        let pm = h.call_sym(POST_PROCESS_MANAGER, Conv::Cdecl, &[]);
        h.call_sym(
            COPY_BUFFER,
            Conv::Thiscall(pm),
            &[renderer, src, dst, 1, 1, 0],
        );
        h.write(at(0x7e), 0u8);
    } else {
        let f: u32 = h.read(at(0x78));
        let limit = f64::from_bits(h.read::<u64>(h.rebase(LIMIT64)));
        // `fcomp` + `test ah, 5; jp`: C0 alone (below, ordered) keeps it
        let strength = if load(f) < limit {
            pass(f)
        } else {
            pass(h.read(h.rebase(LIMIT32)))
        };
        h.write(frame.wrapping_add(0x20), strength);
        for k in 0..3u32 {
            h.write(values.wrapping_add(4 * k), one);
        }
        let strength: u32 = h.read(frame.wrapping_add(0x20));
        h.write(values.wrapping_add(0xc), strength);
        let src = current_buffer(h);
        let renderer: u32 = h.read(this);
        let dst: u32 = h.read(at(0x28));
        let shader = h.rebase(DEPTH_COPY_SHADER);
        let pm = h.call_sym(POST_PROCESS_MANAGER, Conv::Cdecl, &[]);
        h.call_sym(
            COPY_DEPTH_BUFFER,
            Conv::Thiscall(pm),
            &[renderer, src, dst, shader, values, 1, 1, 0, 0],
        );
    }
    bind_target(h, frame, this);
    let renderer: u32 = h.read(this);
    let r = h.call_sym(SET_CLEAR_COLOR, Conv::Thiscall(renderer), &[0, 0, 0, 0]);
    if r != 0 {
        return r;
    }
    let renderer: u32 = h.read(this);
    let r = h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[1]);
    if r != 0 {
        return r;
    }
    let renderer: u32 = h.read(this);
    let pass_at = h.rebase(DISTORTION_PASS);
    h.write(renderer.wrapping_add(0x48), pass_at);
    let renderer: u32 = h.read(this);
    let grid: u32 = h.read(at(0x38));
    let render_pass: u32 = h.read(renderer.wrapping_add(0x13c));
    let materials: u32 = h.read(grid.wrapping_add(0x10));
    h.call_sym(
        RENDER_MESH_INSTANCE,
        Conv::Thiscall(renderer),
        &[grid, materials, 0, render_pass],
    );
    let renderer: u32 = h.read(this);
    h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1])
}

/// [`render_distortion`]'s colour target: `+8` through a render surface
/// at `frame` (its size as viewport, the resolution degraded,
/// `UvScaleBias` (1, 1, 0, 0) on the grid's shader), or none.
fn bind_target(h: &mut impl Host, frame: u32, this: u32) {
    let at = |off: u32| this.wrapping_add(off);
    let values = frame.wrapping_add(0x10);
    let one = 1.0f32.to_bits();
    let target: u32 = h.read(at(8));
    if target == 0 {
        let renderer: u32 = h.read(this);
        h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[0, 0]);
    } else {
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(frame), &[target, 0]);
        let renderer: u32 = h.read(this);
        h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[frame, 0]);
        let t: u32 = h.read(at(8));
        let tex_h = texture(h, t);
        let tex_w = texture(h, t);
        let height: u32 = h.read(tex_h.wrapping_add(0x20));
        let renderer: u32 = h.read(this);
        let width: u32 = h.read(tex_w.wrapping_add(0x1c));
        h.call_sym(
            SET_VIEWPORT,
            Conv::Thiscall(renderer),
            &[0, 0, width, height],
        );
        let renderer: u32 = h.read(this);
        let pm = h.call_sym(POST_PROCESS_MANAGER, Conv::Cdecl, &[]);
        h.call_sym(DEGRADE_RESOLUTION, Conv::Thiscall(pm), &[renderer]);
        h.write(values, one);
        h.write(values.wrapping_add(4), one);
        h.write(values.wrapping_add(8), 0u32);
        h.write(values.wrapping_add(0xc), 0u32);
        let params: u32 = h.read(at(0x2c));
        let name = h.rebase(UV_SCALE_BIAS);
        let p = h.call_sym(FIND_PARAMETER, Conv::Thiscall(params), &[name]);
        if p != 0 {
            let shader: u32 = h.read(at(0x30));
            let buffer: u32 = h.read(shader.wrapping_add(4));
            h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[p, values]);
        }
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(frame), &[]);
    }
}
