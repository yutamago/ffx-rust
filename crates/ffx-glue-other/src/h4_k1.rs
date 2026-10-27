//! Continuation round 9 of list `h4-glue-other`, batch 1: the VFX line
//! render data's update.

use ffx_bridge::Global;
use ffx_bridge::x87::{add, div, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::pass;

/// `graphicVFXGetRenderDataByInstance(a, b)` (`0x006453d0`, cdecl).
pub const GET_RENDER_DATA: u32 = 0x0064_53d0;
/// `graphicDynGeoMemManagerGetBufferForSegment(data, segment, vertices,
/// indices)` (`0x0063fdb0`, cdecl).
pub const GET_BUFFER: u32 = 0x0063_fdb0;
/// `graphicVFXSetVisible(a, b, visible)` (`0x00645710`, cdecl).
pub const SET_VISIBLE: u32 = 0x0064_5710;
/// `__iob_func`'s import slot.
pub const IOB_FUNC: Global<u32> = Global::new(0x00b0_c350);
/// `fprintf`'s import slot.
pub const FPRINTF: Global<u32> = Global::new(0x00b0_c38c);
/// `"Please enlarge MAX_LINE_COUNT \n"` (`0x00b500dc`).
pub const ENLARGE: u32 = 0x00b5_00dc;
/// The bounds' start values: the minimum's (f32 `0x00b17504`, the largest
/// float) and the maximum's (f32 `0x00b1f2d0`, its negation).
pub const LOW_START: u32 = 0x00b1_7504;
/// See [`LOW_START`].
pub const HIGH_START: u32 = 0x00b1_f2d0;
/// The colour scale (f64 `0x00b50198`, 1/256).
pub const COLOUR_SCALE: u32 = 0x00b5_0198;
/// `g_AlphaScale` (f32 `0x00c3a4d0`): the alpha divisor.
pub const ALPHA_SCALE: u32 = 0x00c3_a4d0;
/// Lines the buffer holds (`MAX_LINE_COUNT`).
pub const MAX_LINES: i32 = 0x9c4;

/// `updateVFXRenderDataLine(a, b, lines)` (`0x0071a0c0`, cdecl): the
/// render data of instance (`a`, `b`); none: 0. `lines` is {line array,
/// count, matrix (16 words)}; a line is 0x38 bytes: position 0 (3
/// floats), colour 0 (r, g, b scaled by 1/256, a divided by
/// `g_AlphaScale`), position 1 at `+0x1c`, colour 1 at `+0x28`. Over [`MAX_LINES`] it prints a warning
/// to `stderr` and goes on. The segment (`+0x94`) gets 2 vertices per line
/// (`+4`, `+8`) and its buffer; none: 0. Each line's positions go to `+0xc`
/// (6 floats), its colours to `+0x14` (8), its two indices to `+0x1c`;
/// the matrix to the data's first 16 words with column 3 set to (0, 0,
/// 0, 1), the positions' bounds to `+0x98`..`+0xac`; the instance made
/// visible. Returns 1.
#[allow(clippy::too_many_lines)] // the copy loop and the bounds, in the original's order
pub fn update_line(h: &mut impl Host, a: u32, b: u32, lines: u32) -> u32 {
    let low_start = pass(h.read(h.rebase(LOW_START)));
    let mut low = [low_start; 3];
    let high_start = pass(h.read(h.rebase(HIGH_START)));
    let mut high = [high_start; 3];
    let mut src: u32 = h.read(lines);
    let data = h.call_sym(GET_RENDER_DATA, Conv::Cdecl, &[a, b]);
    if data == 0 {
        return 0;
    }
    if count(h, lines) > MAX_LINES {
        let iob = IOB_FUNC.get(h);
        let files = h.call(iob, Conv::Cdecl, &[]);
        let fprintf = FPRINTF.get(h);
        let text = h.rebase(ENLARGE);
        h.call(fprintf, Conv::Cdecl, &[files.wrapping_add(0x40), text]);
    }
    let segment: u32 = h.read(data.wrapping_add(0x94));
    let n = count(h, lines).wrapping_mul(2) as u32;
    h.write(segment.wrapping_add(8), n);
    let n = count(h, lines).wrapping_mul(2) as u32;
    h.write(segment.wrapping_add(4), n);
    let segment: u32 = h.read(data.wrapping_add(0x94));
    let vertices: u32 = h.read(segment.wrapping_add(4));
    let indices: u32 = h.read(segment.wrapping_add(8));
    if h.call_sym(GET_BUFFER, Conv::Cdecl, &[data, 0, vertices, indices]) == 0 {
        return 0;
    }
    let segment: u32 = h.read(data.wrapping_add(0x94));
    let mut pos: u32 = h.read(segment.wrapping_add(0xc));
    let mut colour: u32 = h.read(segment.wrapping_add(0x14));
    let index: u32 = h.read(segment.wrapping_add(0x1c));
    let mut i: i32 = 0;
    if count(h, lines) > 0 {
        let scale = f64::from_bits(h.read::<u64>(h.rebase(COLOUR_SCALE)));
        loop {
            for (to, from) in [(0, 0), (1, 1), (2, 2), (3, 7), (4, 8), (5, 9)] {
                let v = pass(h.read(src.wrapping_add(4 * from)));
                h.write(pos.wrapping_add(4 * to), v);
            }
            for (to, from) in [
                (0, 3),
                (1, 4),
                (2, 5),
                (3, 6),
                (4, 10),
                (5, 11),
                (6, 12),
                (7, 13),
            ] {
                let x = load(h.read(src.wrapping_add(4 * from)));
                let v = if to % 4 == 3 {
                    let alpha = load(h.read(h.rebase(ALPHA_SCALE)));
                    div(x, alpha)
                } else {
                    mul(x, scale)
                };
                h.write(colour.wrapping_add(4 * to), store(v));
            }
            let first = (i as u32).wrapping_mul(2) as u16;
            let slot = index.wrapping_add((i as u32).wrapping_mul(4));
            h.write(slot, first);
            h.write(slot.wrapping_add(2), first.wrapping_add(1));
            for point in 0..2u32 {
                let at = pos.wrapping_add(12 * point);
                for k in 0..3u32 {
                    let v: u32 = h.read(at.wrapping_add(4 * k));
                    if load(v) < load(low[k as usize]) {
                        low[k as usize] = pass(v);
                    }
                }
                for k in 0..3u32 {
                    let v: u32 = h.read(at.wrapping_add(4 * k));
                    if load(v) > load(high[k as usize]) {
                        high[k as usize] = pass(v);
                    }
                }
            }
            i += 1;
            pos = pos.wrapping_add(0x18);
            src = src.wrapping_add(0x38);
            colour = colour.wrapping_add(0x20);
            if i >= count(h, lines) {
                break;
            }
        }
    }
    for k in 0..16u32 {
        let v: u32 = h.read(lines.wrapping_add(8 + 4 * k));
        h.write(data.wrapping_add(4 * k), v);
    }
    for off in [0xc, 0x1c, 0x2c] {
        h.write(data.wrapping_add(off), 0u32);
    }
    h.write(data.wrapping_add(0x3c), 1.0f32.to_bits());
    for k in 0..3u32 {
        h.write(data.wrapping_add(0x98 + 4 * k), low[k as usize]);
    }
    for k in 0..3u32 {
        h.write(data.wrapping_add(0xa4 + 4 * k), high[k as usize]);
    }
    h.call_sym(SET_VISIBLE, Conv::Cdecl, &[a, b, 1]);
    1
}

/// The line count (`lines + 4`, signed).
fn count(h: &impl Host, lines: u32) -> i32 {
    h.read(lines.wrapping_add(4))
}

// ---- BlurEffectEx_blur

/// `g_PhyreScene` (`0x00ccb9d8`).
pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;
/// `PhyreScene_getViewProjectionMatrix(scene; out)` (`0x0065e660`,
/// thiscall, ret 4).
pub const VIEW_PROJECTION: u32 = 0x0065_e660;
/// `PMatrix4_RowTimes_004c9c20(m; out, v)` (thiscall, ret 8).
pub const ROW_TIMES: u32 = 0x004c_9c20;
/// `BlurEffectEx_setParam4c_006c4680(this; x, y, z, w)` (thiscall, ret
/// 0x10).
pub const SET_PARAM_4C: u32 = 0x006c_4680;
/// `BlurEffectEx_setTextureParam_006c44e0(this; texture, sampler)`
/// (thiscall, ret 8).
pub const SET_TEXTURE: u32 = 0x006c_44e0;
/// `BlurEffectEx_setParam50_006c4730(this; x, y, z, w)` (thiscall, ret
/// 0x10).
pub const SET_PARAM_50: u32 = 0x006c_4730;
/// `Phyre_PRendering_PRenderSurface_PRenderSurface(surface; target, 0)`
/// (`0x005b35c0`, thiscall, ret 8).
pub const SURFACE_CTOR: u32 = 0x005b_35c0;
/// `Phyre_PRendering_PRenderer_setColorTarget(renderer; surface, 0)`
/// (`0x005bb130`, thiscall, ret 8).
pub const SET_COLOR_TARGET: u32 = 0x005b_b130;
/// `Phyre_PRendering_PRenderer_beginScene(renderer; 0)` (`0x005b5e30`,
/// thiscall, ret 4).
pub const BEGIN_SCENE: u32 = 0x005b_5e30;
/// `Phyre_PRendering_PRendererBase_renderMeshInstance(renderer; instance,
/// mesh, 0, context)` (`0x005b9a90`, thiscall, ret 0x10).
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
/// `Phyre_PRendering_PRenderer_endScene(renderer; 1)` (`0x005b6950`,
/// thiscall, ret 4).
pub const END_SCENE: u32 = 0x005b_6950;
/// `Phyre_PRendering_PRenderSurface_dtor_PRenderSurface(surface)`
/// (`0x005b3640`, an empty thiscall).
pub const SURFACE_DTOR: u32 = 0x005b_3640;
/// The blur pass's states (`0x00cdebc8`, `0x00cdebe8` for mode 2), stored
/// at the renderer's `+0x48`.
pub const STATE: u32 = 0x00cd_ebc8;
/// See [`STATE`].
pub const STATE_MODE_2: u32 = 0x00cd_ebe8;
/// One half (f64 `0x00b92220`): the clip space to texture scale.
pub const HALF: u32 = 0x00b9_2220;
/// Bytes of [`blur`]'s frame, from the original's `ebp - 0x90`: the view
/// projection matrix (`+0`), the render surface (`+0x40`), the projected
/// point (`+0x50`), the centre (`+0x60`, `+0x64`) and the point
/// (`+0x68`, 4 floats; later the `setParam50` words).
pub const BLUR_FRAME: u32 = 0x80;

/// `BlurEffectEx_blur_006c4290(this; texture, target)` (thiscall, ret 8;
/// `frame` see [`BLUR_FRAME`]; the SEH frame is not kept): the blur's
/// centre (u, v): with `+0x80` zero `+0x74`, `+0x78`; else the point
/// (-`+0x74`, -`+0x78`, `+0x7c`, 1) through the view projection matrix,
/// x/w and y/w mapped from -1..1 to 0..1. Then the parameters: `+0x5c`..
/// `+0x68` to `setParam4c`, the texture with sampler `+0x24`, and (1/a,
/// 1/b, (1 - 1/a) u, (1 - 1/b) v) for the scale (a, b) = (`+0x6c`,
/// `+0x70`) to `setParam50`. The pass renders the mesh instance `+0x18`
/// with the renderer `+0` into `target` (state by mode `+0x84`); a
/// finished pass sets the mode to 0. Returns `beginScene`'s or
/// `endScene`'s result. The original also spills `+0x6c` to its own
/// `texture` argument slot (popped; nobody reads it).
pub fn blur(h: &mut impl Host, frame: u32, this: u32, texture: u32, target: u32) -> u32 {
    let at = |off: u32| this.wrapping_add(off);
    let matrix = frame;
    let surface = frame + 0x40;
    let projected = frame + 0x50;
    let point = frame + 0x68;
    let (u, v) = if load(h.read(at(0x80))) == 0.0 {
        (pass(h.read(at(0x74))), pass(h.read(at(0x78))))
    } else {
        let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
        h.call_sym(VIEW_PROJECTION, Conv::Thiscall(scene), &[matrix]);
        let z = pass(h.read(at(0x7c)));
        let y = store(-load(h.read(at(0x78))));
        let x = store(-load(h.read(at(0x74))));
        h.write(point, x);
        h.write(point + 4, y);
        h.write(point + 8, z);
        h.write(point + 0xc, 1.0f32.to_bits());
        h.call_sym(ROW_TIMES, Conv::Thiscall(matrix), &[projected, point]);
        let w = load(h.read(projected + 0xc));
        let half = f64::from_bits(h.read::<u64>(h.rebase(HALF)));
        let px = load(store(div(load(h.read(projected)), w)));
        let u = store(mul(add(px, 1.0), half));
        let w = load(store(w));
        let py = load(store(div(load(h.read(projected + 4)), w)));
        (u, store(mul(add(py, 1.0), half)))
    };
    h.write(frame + 0x60, u);
    h.write(frame + 0x64, v);
    let words: [u32; 4] = std::array::from_fn(|k| h.read(at(0x5c + 4 * k as u32)));
    h.call_sym(SET_PARAM_4C, Conv::Thiscall(this), &words);
    h.call_sym(SET_TEXTURE, Conv::Thiscall(this), &[texture, at(0x24)]);
    let b = load(h.read(at(0x70)));
    let a = load(h.read(at(0x6c)));
    let inv_a = div(1.0, a);
    let inv_b = div(1.0, b);
    let words = [
        store(inv_a),
        store(inv_b),
        store(mul(sub(1.0, inv_a), load(u))),
        store(mul(sub(1.0, inv_b), load(v))),
    ];
    for (k, &w) in words.iter().enumerate() {
        h.write(point + 4 * k as u32, w);
    }
    h.call_sym(SET_PARAM_50, Conv::Thiscall(this), &words);
    h.call_sym(SURFACE_CTOR, Conv::Thiscall(surface), &[target, 0]);
    let renderer: u32 = h.read(at(0));
    h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[surface, 0]);
    let renderer: u32 = h.read(at(0));
    let mut result = h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[0]);
    if result == 0 {
        let mode: u32 = h.read(at(0x84));
        let renderer: u32 = h.read(at(0));
        let state = h.rebase(if mode == 2 { STATE_MODE_2 } else { STATE });
        h.write(renderer.wrapping_add(0x48), state);
        let renderer: u32 = h.read(at(0));
        let instance: u32 = h.read(at(0x18));
        let context: u32 = h.read(renderer.wrapping_add(0x13c));
        let mesh: u32 = h.read(instance.wrapping_add(0x10));
        h.call_sym(
            RENDER_MESH_INSTANCE,
            Conv::Thiscall(renderer),
            &[instance, mesh, 0, context],
        );
        let renderer: u32 = h.read(at(0));
        result = h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1]);
        if result == 0 {
            h.write(at(0x84), 0u32);
        }
    }
    h.call_sym(SURFACE_DTOR, Conv::Thiscall(surface), &[]);
    result
}

// ---- BlurEffect_render_006c3970

/// `PostProcessManager_GetInstance()` (`0x00692be0`).
pub const POST_GET_INSTANCE: u32 = 0x0069_2be0;
/// `PostProcessManager_copyBuffer(manager; renderer, a, buffer, 1, 0, 0)`
/// (`0x00692c40`, thiscall, ret 0x18).
pub const COPY_BUFFER: u32 = 0x0069_2c40;
/// `CopyFloats4_00703750(pass; floats)` (thiscall, ret 4).
pub const COPY_FLOATS_4: u32 = 0x0070_3750;
/// `CopyFloats2_007037e0(pass; floats)` (thiscall, ret 4).
pub const COPY_FLOATS_2: u32 = 0x0070_37e0;
/// `0x007030e0(pass; texture, flag)` (thiscall, ret 8): the blur pass's
/// source texture.
pub const PASS_SET_TEXTURE: u32 = 0x0070_30e0;
/// `0x00703480(pass; buffer)` (thiscall, ret 4): the blur pass's run.
pub const PASS_RUN: u32 = 0x0070_3480;
/// `0x006f8b10(pass; floats)` (thiscall, ret 4).
pub const SECOND_FLOATS_4: u32 = 0x006f_8b10;
/// `0x006f8ba0(pass; floats)` (thiscall, ret 4).
pub const SECOND_FLOATS_2: u32 = 0x006f_8ba0;
/// `0x006f8850(pass; buffer)` (thiscall, ret 4): the second pass's run.
pub const SECOND_RUN: u32 = 0x006f_8850;
/// `ShaderParams_FindByName(shader; name)` (`0x0056cbf0`, thiscall, ret
/// 4).
pub const FIND_BY_NAME: u32 = 0x0056_cbf0;
/// `Phyre_PParameterBuffer_setTextureChecked_0056e670(buffer; parameter,
/// value)` (thiscall, ret 8).
pub const SET_TEXTURE_CHECKED: u32 = 0x0056_e670;
/// `ShaderParamBuffer_SetValue(buffer; parameter, value)` (`0x00638780`,
/// thiscall, ret 8).
pub const SET_VALUE: u32 = 0x0063_8780;
/// `"ColorBuffer"`, `"OutputScale"`, `"UvScaleBias"` (`0x00b425d8`,
/// `0x00b4948c`, `0x00b425e4`).
pub const COLOR_BUFFER: u32 = 0x00b4_25d8;
/// See [`COLOR_BUFFER`].
pub const OUTPUT_SCALE: u32 = 0x00b4_948c;
/// See [`COLOR_BUFFER`].
pub const UV_SCALE_BIAS: u32 = 0x00b4_25e4;
/// The fade's start time (f32 `0x00b3eb98`), and the f64s it is
/// measured with: the time subtracted (`0x00b41d20`) and the scale
/// (`0x00b34f90`).
pub const FADE_START: u32 = 0x00b3_eb98;
/// See [`FADE_START`].
pub const FADE_OFFSET: u32 = 0x00b4_1d20;
/// See [`FADE_START`].
pub const FADE_SCALE: u32 = 0x00b3_4f90;
/// Bytes of [`render`]'s frame, from the original's `ebp - 0x2c`: the
/// render surface (`+0`) and the parameter value (`+0x10`, 4 words).
pub const RENDER_FRAME: u32 = 0x20;

/// A buffer's texture: its first word unless its byte `+4` is set (0
/// then).
fn texture_of(h: &impl Host, buffer: u32) -> u32 {
    if h.read::<u8>(buffer.wrapping_add(4)) == 0 {
        h.read(buffer)
    } else {
        0
    }
}

/// `PostProcessManager_copyBuffer` on the post-process manager.
fn copy_buffer(h: &mut impl Host, renderer: u32, a: u32, buffer: u32) {
    let manager = h.call_sym(POST_GET_INSTANCE, Conv::Cdecl, &[]);
    h.call_sym(
        COPY_BUFFER,
        Conv::Thiscall(manager),
        &[renderer, a, buffer, 1, 0, 0],
    );
}

/// `BlurEffect_render_006c3970(this)` (thiscall, no stack words; `frame`
/// see [`RENDER_FRAME`]; the SEH frame is not kept): while active (byte
/// `+0xc`) the two buffers `+0`/`+4` swap; for mode `+0x10` 2 the history
/// buffer `+8` and the new back buffer are copied from the scene; while
/// `+0x10` is positive the front buffer too, and `+0x10` counts down. The
/// first pass (`+0x18`: floats `+0x3c`, `+0x6c`, the manager's buffer
/// `+0x8c` as texture, run on `+8`), the second (`+0x1c`: `+0x4c`,
/// `+0x74`, run on the front buffer), the first again (`+0x5c`, `+0x7c`,
/// the back buffer's texture, run on the front buffer); then the mesh
/// instance `+0x28`'s first material gets `ColorBuffer` (the front
/// buffer's texture), `OutputScale` (1, 1, 1, s: s is 1, below 1 past the
/// fade's start) and `UvScaleBias` (1, 1, 0, 0), and is rendered with the
/// renderer `+0x14` into the manager's buffer, which is copied to the
/// front buffer. Returns 0.
#[allow(clippy::too_many_lines)] // three passes, three parameters and the mesh pass
pub fn render(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let at = |off: u32| this.wrapping_add(off);
    if h.read::<u8>(at(0xc)) == 0 {
        return 0;
    }
    let mode: i32 = h.read(at(0x10));
    let front: u32 = h.read(at(0));
    let back: u32 = h.read(at(4));
    h.write(at(0), back);
    h.write(at(4), front);
    if mode == 2 {
        for buffer in [at(8), at(4)] {
            let renderer: u32 = h.read(at(0x14));
            let a: u32 = h.read(at(0x2c));
            let buffer: u32 = h.read(buffer);
            copy_buffer(h, renderer, a, buffer);
        }
    }
    if h.read::<i32>(at(0x10)) > 0 {
        let renderer: u32 = h.read(at(0x14));
        let a: u32 = h.read(at(0x2c));
        let buffer: u32 = h.read(at(0));
        copy_buffer(h, renderer, a, buffer);
        let n: u32 = h.read(at(0x10));
        h.write(at(0x10), n.wrapping_sub(1));
    }
    let first: u32 = h.read(at(0x18));
    h.call_sym(COPY_FLOATS_4, Conv::Thiscall(first), &[at(0x3c)]);
    let first: u32 = h.read(at(0x18));
    h.call_sym(COPY_FLOATS_2, Conv::Thiscall(first), &[at(0x6c)]);
    let manager = h.call_sym(POST_GET_INSTANCE, Conv::Cdecl, &[]);
    let target: u32 = h.read(manager.wrapping_add(0x8c));
    let texture = texture_of(h, target);
    let first: u32 = h.read(at(0x18));
    h.call_sym(PASS_SET_TEXTURE, Conv::Thiscall(first), &[texture, 1]);
    let history: u32 = h.read(at(8));
    let first: u32 = h.read(at(0x18));
    h.call_sym(PASS_RUN, Conv::Thiscall(first), &[history]);
    let second: u32 = h.read(at(0x1c));
    h.call_sym(SECOND_FLOATS_4, Conv::Thiscall(second), &[at(0x4c)]);
    let second: u32 = h.read(at(0x1c));
    h.call_sym(SECOND_FLOATS_2, Conv::Thiscall(second), &[at(0x74)]);
    let front: u32 = h.read(at(0));
    let second: u32 = h.read(at(0x1c));
    h.call_sym(SECOND_RUN, Conv::Thiscall(second), &[front]);
    let first: u32 = h.read(at(0x18));
    h.call_sym(COPY_FLOATS_4, Conv::Thiscall(first), &[at(0x5c)]);
    let first: u32 = h.read(at(0x18));
    h.call_sym(COPY_FLOATS_2, Conv::Thiscall(first), &[at(0x7c)]);
    let back: u32 = h.read(at(4));
    let texture = texture_of(h, back);
    let first: u32 = h.read(at(0x18));
    h.call_sym(PASS_SET_TEXTURE, Conv::Thiscall(first), &[texture, 0]);
    let front: u32 = h.read(at(0));
    let first: u32 = h.read(at(0x18));
    h.call_sym(PASS_RUN, Conv::Thiscall(first), &[front]);
    // the mesh instance's first material: {shader parameters, buffer}
    let instance: u32 = h.read(at(0x28));
    let materials: u32 = h.read(instance.wrapping_add(0x10));
    let n = h.read::<u32>(materials) & 0x7fff_ffff;
    let list = if n > 1 {
        h.read(materials.wrapping_add(4))
    } else if n != 0 {
        materials.wrapping_add(4)
    } else {
        0
    };
    let material: u32 = h.read(list);
    let value = frame + 0x10;
    let shader: u32 = h.read(material);
    let name = h.rebase(COLOR_BUFFER);
    let p = h.call_sym(FIND_BY_NAME, Conv::Thiscall(shader), &[name]);
    if p != 0 {
        let front: u32 = h.read(at(0));
        h.write(value, 0x42u32);
        let texture = texture_of(h, front);
        h.write(value + 0xc, texture);
        h.write(value + 8, 0u32);
        h.write(value + 4, 0u32);
        let buffer: u32 = h.read(material.wrapping_add(4));
        h.call_sym(SET_TEXTURE_CHECKED, Conv::Thiscall(buffer), &[p, value]);
    }
    let shader: u32 = h.read(material);
    let name = h.rebase(OUTPUT_SCALE);
    let p = h.call_sym(FIND_BY_NAME, Conv::Thiscall(shader), &[name]);
    let one = 1.0f32.to_bits();
    if p != 0 {
        for k in 0..4 {
            h.write(value + 4 * k, one);
        }
        let start = load(h.read(h.rebase(FADE_START)));
        let time = load(h.read(at(0x84)));
        if start < time {
            let offset = f64::from_bits(h.read::<u64>(h.rebase(FADE_OFFSET)));
            let scale = f64::from_bits(h.read::<u64>(h.rebase(FADE_SCALE)));
            let s = sub(1.0, mul(sub(time, offset), scale));
            h.write(value + 0xc, store(s));
        }
        let buffer: u32 = h.read(material.wrapping_add(4));
        h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[p, value]);
    }
    let shader: u32 = h.read(material);
    let name = h.rebase(UV_SCALE_BIAS);
    let p = h.call_sym(FIND_BY_NAME, Conv::Thiscall(shader), &[name]);
    if p != 0 {
        for (k, w) in [one, one, 0, 0].into_iter().enumerate() {
            h.write(value + 4 * k as u32, w);
        }
        let buffer: u32 = h.read(material.wrapping_add(4));
        h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[p, value]);
    }
    let surface = frame;
    let manager = h.call_sym(POST_GET_INSTANCE, Conv::Cdecl, &[]);
    let target: u32 = h.read(manager.wrapping_add(0x8c));
    h.call_sym(SURFACE_CTOR, Conv::Thiscall(surface), &[target, 0]);
    let renderer: u32 = h.read(at(0x14));
    h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[surface, 0]);
    let renderer: u32 = h.read(at(0x14));
    h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[0]);
    let renderer: u32 = h.read(at(0x14));
    let state = h.rebase(STATE);
    h.write(renderer.wrapping_add(0x48), state);
    let renderer: u32 = h.read(at(0x14));
    let instance: u32 = h.read(at(0x28));
    let context: u32 = h.read(renderer.wrapping_add(0x13c));
    let mesh: u32 = h.read(instance.wrapping_add(0x10));
    h.call_sym(
        RENDER_MESH_INSTANCE,
        Conv::Thiscall(renderer),
        &[instance, mesh, 0, context],
    );
    let renderer: u32 = h.read(at(0x14));
    h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1]);
    let renderer: u32 = h.read(at(0x14));
    let a: u32 = h.read(at(0x2c));
    let front: u32 = h.read(at(0));
    copy_buffer(h, renderer, a, front);
    h.call_sym(SURFACE_DTOR, Conv::Thiscall(surface), &[]);
    0
}

// ---- BlurEffect_update_006c3d40

/// `_CIfmod`'s import thunk (`0x009498e0`): `ST1` modulo `ST0`.
pub const CI_FMOD: u32 = 0x0094_98e0;
/// The update's constants: the time step (f64 `0x00b494c8`), the time the
/// fade turns (f32 `0x00b922f8`, 4) and the same as f64 (`0x00b43d80`),
/// the rising slope (`0x00b494c0`) and start (`0x00b48340`), the clamp
/// (f64 `0x00b3eb70`, 0.99) and its f32 value (`0x00b482dc`), the late
/// start (`0x00b48328`) and slopes (`0x00b3eb60`, `0x00b921f0`), the late
/// floor (f32 `0x00b3eb48`), the wobble's scales (`0x00b494a8`,
/// `0x00b494b0`) and periods (`0x00b494b8`, `0x00b412e8`), the late
/// zoom slopes (`0x00b49498`, `0x00b494a0`) and bases (`0x00b494d0`,
/// `0x00b48338`), the end time (f32 `0x00b92318`).
pub const STEP: u32 = 0x00b4_94c8;
/// See [`STEP`].
pub const TURN: u32 = 0x00b9_22f8;
/// See [`STEP`].
pub const TURN_F64: u32 = 0x00b4_3d80;
/// See [`STEP`].
pub const EARLY_SLOPE: u32 = 0x00b4_94c0;
/// See [`STEP`].
pub const EARLY_START: u32 = 0x00b4_8340;
/// See [`STEP`].
pub const CLAMP: u32 = 0x00b3_eb70;
/// See [`STEP`].
pub const CLAMP_F32: u32 = 0x00b4_82dc;
/// See [`STEP`].
pub const LATE_START: u32 = 0x00b4_8328;
/// See [`STEP`].
pub const LATE_SLOPE_48: u32 = 0x00b3_eb60;
/// See [`STEP`].
pub const LATE_SLOPE_68: u32 = 0x00b9_21f0;
/// See [`STEP`].
pub const LATE_FLOOR: u32 = 0x00b3_eb48;
/// See [`STEP`].
pub const WOBBLE_SCALE_A: u32 = 0x00b4_94a8;
/// See [`STEP`].
pub const WOBBLE_SCALE_B: u32 = 0x00b4_94b0;
/// See [`STEP`].
pub const WOBBLE_PERIOD_A: u32 = 0x00b4_94b8;
/// See [`STEP`].
pub const WOBBLE_PERIOD_B: u32 = 0x00b4_12e8;
/// See [`STEP`].
pub const ZOOM_SLOPE_A: u32 = 0x00b4_9498;
/// See [`STEP`].
pub const ZOOM_SLOPE_B: u32 = 0x00b4_94a0;
/// See [`STEP`].
pub const ZOOM_BASE_A: u32 = 0x00b4_94d0;
/// See [`STEP`].
pub const ZOOM_BASE_B: u32 = 0x00b4_8338;
/// See [`STEP`].
pub const END: u32 = 0x00b9_2318;

/// The f64 at image address `sym`.
fn image_f64(h: &impl Host, sym: u32) -> f64 {
    f64::from_bits(h.read::<u64>(h.rebase(sym)))
}

/// The f32 at image address `sym`, as `fld` loads it.
fn image_f32(h: &impl Host, sym: u32) -> f64 {
    load(h.read(h.rebase(sym)))
}

/// `BlurEffect_update_006c3d40(this; dt)` (thiscall, ret 4; `dt` is
/// never read): while active (byte `+0xc`) the time `+0x84` moves on by a
/// fixed step; the colour blocks `+0x3c`..`+0x64` (and `+0x74`, `+0x78`)
/// become 1 except `+0x48` and `+0x68`, which fade with the time (before
/// the turn up from a start, clamped to 0.99; after it `+0x48` towards 1,
/// `+0x68` down to a floor); the zooms `+0x6c`, `+0x70`, `+0x7c`, `+0x80`
/// wobble before the turn (1 + the time scaled modulo a period, through
/// `fmod`: `_CIfmod(a, b)`) and grow after it. With no hold (`+0x10` 0)
/// the effect ends past the end time. Comparisons with a NaN time take
/// the late branches and never clamp. Returns 0.
pub fn update<H: Host>(
    h: &mut H,
    this: u32,
    fmod: &mut impl FnMut(&mut H, f64, f64) -> f64,
) -> u32 {
    let at = |off: u32| this.wrapping_add(off);
    if h.read::<u8>(at(0xc)) == 0 {
        return 0;
    }
    let time = store(add(load(h.read(at(0x84))), image_f64(h, STEP)));
    h.write(at(0x84), time);
    let x = load(time);
    let one = 1.0f32.to_bits();
    for off in [0x3c, 0x40, 0x44] {
        h.write(at(off), one);
    }
    // `fcom`: the early branches only while the turn is above the time
    let early = image_f32(h, TURN) > x;
    let clamp = image_f64(h, CLAMP);
    if early {
        let v = store(sub(
            image_f64(h, EARLY_START),
            mul(image_f64(h, EARLY_SLOPE), x),
        ));
        h.write(at(0x48), v);
        if load(h.read(at(0x48))) > clamp {
            h.write(at(0x48), store(image_f32(h, CLAMP_F32)));
        }
    } else {
        let v = store(add(
            mul(sub(x, image_f64(h, TURN_F64)), image_f64(h, LATE_SLOPE_48)),
            image_f64(h, LATE_START),
        ));
        h.write(at(0x48), v);
        if 1.0 < load(h.read(at(0x48))) {
            h.write(at(0x48), one);
        }
    }
    for off in (0x4c..=0x64).step_by(4) {
        h.write(at(off), one);
    }
    if early {
        let v = store(sub(
            image_f64(h, EARLY_START),
            mul(image_f64(h, EARLY_SLOPE), x),
        ));
        h.write(at(0x68), v);
        if load(h.read(at(0x68))) > clamp {
            h.write(at(0x68), store(image_f32(h, CLAMP_F32)));
        }
    } else {
        let v = store(sub(
            image_f64(h, LATE_START),
            mul(sub(x, image_f64(h, TURN_F64)), image_f64(h, LATE_SLOPE_68)),
        ));
        h.write(at(0x68), v);
        let floor = image_f32(h, LATE_FLOOR);
        if floor > load(h.read(at(0x68))) {
            h.write(at(0x68), store(floor));
        }
    }
    h.write(at(0x74), one);
    h.write(at(0x78), one);
    if early {
        for (off, scale, period) in [
            (0x6c, WOBBLE_SCALE_A, WOBBLE_PERIOD_A),
            (0x70, WOBBLE_SCALE_A, WOBBLE_PERIOD_A),
            (0x7c, WOBBLE_SCALE_B, WOBBLE_PERIOD_B),
            (0x80, WOBBLE_SCALE_B, WOBBLE_PERIOD_B),
        ] {
            let t = store(mul(x, image_f64(h, scale)));
            let period = image_f64(h, period);
            let r = store(fmod(h, load(t), period));
            h.write(at(off), store(add(load(r), 1.0)));
        }
    } else {
        let turn = image_f64(h, TURN_F64);
        for (off, slope, base) in [
            (0x6c, ZOOM_SLOPE_A, ZOOM_BASE_A),
            (0x70, ZOOM_SLOPE_A, ZOOM_BASE_A),
            (0x7c, ZOOM_SLOPE_B, ZOOM_BASE_B),
            (0x80, ZOOM_SLOPE_B, ZOOM_BASE_B),
        ] {
            let v = add(mul(sub(x, turn), image_f64(h, slope)), image_f64(h, base));
            h.write(at(off), store(v));
        }
    }
    if h.read::<u32>(at(0x10)) == 0 && image_f32(h, END) < x {
        h.write(at(0xc), 0u8);
    }
    0
}
