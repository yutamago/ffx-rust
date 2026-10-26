//! Continuation round 7 of list `h4-glue-other`, batch 6: the broken
//! screen effect's frame capture.

use ffx_bridge::x87::{ftol, load, mul, store};
use ffx_graphic::host::{Conv, Host};

use crate::h4_i2::{BEGIN_SCENE, END_SCENE, RENDER_MESH_INSTANCE};
use crate::h4_i3::{
    FIND_PARAMETER, POST_PROCESS_MANAGER, SET_COLOR_TARGET, SET_VALUE, SURFACE_CTOR, SURFACE_DTOR,
    UV_SCALE_BIAS,
};

/// `BrokenScreenEffect_createRenderTarget(this)` (thiscall).
pub const CREATE_RENDER_TARGET: u32 = 0x006d_5050;
/// `Phyre_PRendering_PRenderer_setDepthStencilTarget(renderer; target)`
/// (thiscall, ret 4).
pub const SET_DEPTH_TARGET: u32 = 0x005b_b5b0;
/// `"OutputScale"` (`0x00b4948c`).
pub const OUTPUT_SCALE: u32 = 0x00b4_948c;
/// `"ColorBuffer"` (`0x00b425d8`).
pub const COLOR_BUFFER: u32 = 0x00b4_25d8;
/// `g_PhyreScene` (`0x00ccb9d8`).
pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;
/// `PhyreScene_getLinearSampler(scene; i)` (thiscall, ret 4).
pub const LINEAR_SAMPLER: u32 = 0x0065_e050;
/// `Phyre_PParameterBuffer_setTextureChecked_0056e670(buffer; parameter,
/// binding)` (thiscall, ret 8).
pub const SET_TEXTURE: u32 = 0x0056_e670;
/// `PostProcessManager_SetFirstEntry(manager; 0)` (thiscall, ret 4).
pub const SET_FIRST_ENTRY: u32 = 0x0069_8400;
/// Set when the next capture is the first of the effect (`0x0112a05c`):
/// a black output scale.
pub const FIRST_CAPTURE: u32 = 0x0112_a05c;
/// The current event and its state (`0x00c421d0`, `0x00c421d4`): event
/// 0x23 in state 1 keeps a white output scale.
pub const EVENT_ID: u32 = 0x00c4_21d0;
/// See [`EVENT_ID`].
pub const EVENT_STATE: u32 = 0x00c4_21d4;
/// The capture pass (`0x00cded20`) and the plain one (`0x00cdeca0`).
pub const CAPTURE_PASS: u32 = 0x00cd_ed20;
/// See [`CAPTURE_PASS`].
pub const PLAIN_PASS: u32 = 0x00cd_eca0;
/// Bytes of [`capture_frame`]'s frame, from `ebp - 0x38`: the render
/// surface (`+0`, 0x10 bytes) and the four parameter words (`+0x10`).
pub const CAPTURE_FRAME: u32 = 0x20;

/// The four words at `values`, set as `p` on `buffer`.
fn set_values(h: &mut impl Host, buffer: u32, p: u32, values: u32, v: [u32; 4]) {
    for (k, w) in v.iter().enumerate() {
        h.write(values.wrapping_add(4 * k as u32), *w);
    }
    h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[p, values]);
}

/// `BrokenScreenEffect_captureFrame(this)` (`0x006d4e20`, thiscall, from
/// `PhyreScene_render`; `frame` see [`CAPTURE_FRAME`]; the SEH frame is
/// not kept): byte `+0x75` cleared; when a capture is pending (`+4`): the
/// render target made if missing (`+0x74`), `+0x75` set, the target
/// `[this]` bound as colour target of the renderer `+0x2c` (no depth),
/// the quad's (`+0x58`) first material's shader parameters set:
/// `OutputScale` (1, 1, 1, 1), `UvScaleBias` (1, 1, 0, 0), `ColorBuffer`
/// the texture `+0x5c` with the scene's linear sampler. The first capture
/// (`0x0112a05c`, then cleared) scales the output by (0, 0, 0, 1) and
/// event 0x23 in state 1 by (1, 1, 1, 1), both with the capture pass;
/// otherwise the plain pass. The scene begun, the quad drawn, the scene
/// ended, `+8` 1.0, the pending flag cleared and the post-process
/// manager's first entry reset.
pub fn capture_frame(h: &mut impl Host, frame: u32, this: u32) {
    let at = |off: u32| this.wrapping_add(off);
    h.write(at(0x75), 0u8);
    if h.read::<u8>(at(4)) == 0 {
        return;
    }
    if h.read::<u8>(at(0x74)) == 0 {
        h.call_sym(CREATE_RENDER_TARGET, Conv::Thiscall(this), &[]);
    }
    h.write(at(0x75), 1u8);
    let target: u32 = h.read(this);
    h.call_sym(SURFACE_CTOR, Conv::Thiscall(frame), &[target, 0]);
    let renderer: u32 = h.read(at(0x2c));
    h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[frame, 0]);
    let renderer: u32 = h.read(at(0x2c));
    h.call_sym(SET_DEPTH_TARGET, Conv::Thiscall(renderer), &[0]);
    let quad: u32 = h.read(at(0x58));
    let list: u32 = h.read(quad.wrapping_add(0x10));
    let n = h.read::<u32>(list) & 0x7fff_ffff;
    let first = if n > 1 {
        h.read(list.wrapping_add(4))
    } else if n == 0 {
        0
    } else {
        list.wrapping_add(4)
    };
    let material: u32 = h.read(first);
    let params: u32 = h.read(material);
    let name = h.rebase(OUTPUT_SCALE);
    let output = h.call_sym(FIND_PARAMETER, Conv::Thiscall(params), &[name]);
    let name = h.rebase(UV_SCALE_BIAS);
    let uv = h.call_sym(FIND_PARAMETER, Conv::Thiscall(params), &[name]);
    let name = h.rebase(COLOR_BUFFER);
    let colour = h.call_sym(FIND_PARAMETER, Conv::Thiscall(params), &[name]);
    let values = frame.wrapping_add(0x10);
    let one = 1.0f32.to_bits();
    if output != 0 {
        let buffer: u32 = h.read(material.wrapping_add(4));
        set_values(h, buffer, output, values, [one; 4]);
    }
    if uv != 0 {
        let buffer: u32 = h.read(material.wrapping_add(4));
        set_values(h, buffer, uv, values, [one, one, 0, 0]);
    }
    if colour != 0 {
        h.write(values, 0x42u32);
        let texture: u32 = h.read(at(0x5c));
        h.write(values.wrapping_add(0xc), texture);
        h.write(values.wrapping_add(4), 0u32);
        let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
        let sampler = h.call_sym(LINEAR_SAMPLER, Conv::Thiscall(scene), &[2]);
        h.write(values.wrapping_add(8), sampler);
        let buffer: u32 = h.read(material.wrapping_add(4));
        h.call_sym(SET_TEXTURE, Conv::Thiscall(buffer), &[colour, values]);
    }
    let first_at = h.rebase(FIRST_CAPTURE);
    let scale = if h.read::<u32>(first_at) != 0 {
        h.write(first_at, 0u32);
        Some([0, 0, 0, one])
    } else if h.read::<u32>(h.rebase(EVENT_ID)) == 0x23 && h.read::<u32>(h.rebase(EVENT_STATE)) == 1
    {
        Some([one; 4])
    } else {
        None
    };
    let pass = if let Some(v) = scale {
        if output != 0 {
            let buffer: u32 = h.read(material.wrapping_add(4));
            set_values(h, buffer, output, values, v);
        }
        CAPTURE_PASS
    } else {
        PLAIN_PASS
    };
    let renderer: u32 = h.read(at(0x2c));
    h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[0]);
    let renderer: u32 = h.read(at(0x2c));
    let pass_at = h.rebase(pass);
    h.write(renderer.wrapping_add(0x48), pass_at);
    let renderer: u32 = h.read(at(0x2c));
    let quad: u32 = h.read(at(0x58));
    let render_pass: u32 = h.read(renderer.wrapping_add(0x13c));
    let materials: u32 = h.read(quad.wrapping_add(0x10));
    h.call_sym(
        RENDER_MESH_INSTANCE,
        Conv::Thiscall(renderer),
        &[quad, materials, 0, render_pass],
    );
    let renderer: u32 = h.read(at(0x2c));
    h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1]);
    h.write(at(8), one);
    h.write(at(4), 0u8);
    let manager = h.call_sym(POST_PROCESS_MANAGER, Conv::Cdecl, &[]);
    h.call_sym(SET_FIRST_ENTRY, Conv::Thiscall(manager), &[0]);
    h.call_sym(SURFACE_DTOR, Conv::Thiscall(frame), &[]);
}

// ---- PhyFMVPlayerManager_UpdateTexture

/// `PVideoPlaybackWin32_isPlaying(player)` (thiscall): `al`.
pub const IS_PLAYING: u32 = crate::h4_i2::IS_PLAYING;
/// `Phyre_PVideo_PVideoPlaybackWin32_getCurrentTime(player)` (thiscall):
/// `edx:eax`.
pub const CURRENT_TIME: u32 = 0x00a2_7bd0;
/// `Phyre_PVideo_PVideoPlaybackWin32_getEndTime(player)` (thiscall):
/// `edx:eax`.
pub const END_TIME: u32 = 0x00a2_7c40;
/// `operator_new(n)` (cdecl).
pub const OPERATOR_NEW: u32 = crate::h4_i2::OPERATOR_NEW;
/// `operator_delete(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = crate::h4_i2::OPERATOR_DELETE;
/// `Phyre_PFramework_PApplication_GetApplication()` (cdecl).
pub const GET_APPLICATION: u32 = 0x0062_3b90;
/// `PRenderCommandQueue_queueCallback(queue; f, arg)` (thiscall, ret 8).
pub const QUEUE_CALLBACK: u32 = 0x005b_5880;
/// `PhyFMVPlayerManager_updateVideoCallback` (`0x006dae00`).
pub const UPDATE_CALLBACK: u32 = 0x006d_ae00;
/// `Phyre_PVideo_PVideoPlaybackWin32_updateTexture(player; y, u, v)`
/// (thiscall, ret 0xc): `al`.
pub const UPDATE_PLANES: u32 = 0x00a2_84a0;
/// `PVideoPlaybackWin32_togglePause(player)` (thiscall).
pub const TOGGLE_PAUSE: u32 = 0x00a2_7e90;
/// The frames the game waits by time (`g_WaitFramesByTime`,
/// `0x0112a008`).
pub const WAIT_FRAMES_BY_TIME: u32 = 0x0112_a008;
/// The time scale constants (`0x00b4a178`, `0x00b47c68`, f64): the
/// movie time in frames.
pub const TIME_SCALE: [u32; 2] = [0x00b4_a178, 0x00b4_7c68];

/// The movie's end: unless kept (`+0x720`), the frame `+0x6d8` 0xfffe,
/// the texture flag `+0x6d0` cleared and `+0x710` set.
fn movie_end(h: &mut impl Host, this: u32) {
    if h.read::<u8>(this.wrapping_add(0x720)) == 0 {
        h.write(this.wrapping_add(0x6d8), 0xfffeu32);
        h.write(this.wrapping_add(0x6d0), 0u8);
        h.write(this.wrapping_add(0x710), 1u8);
    }
}

/// `PhyFMVPlayerManager_UpdateTexture(this)` (`0x006d7680`, thiscall):
/// nothing unless on (`+0x6d2`) and not already showing a paused frame
/// (`+0x6e4` with `+0x6fc`). A movie not playing ends it (when it showed a
/// frame); one at or past its end time, or with movies suppressed, ends it
/// too (unless `+0x6fe`, or before its first frame). Else a 0x24-byte
/// update record (the player, its three planes `+0x38c`/`+0x400`/`+0x474`,
/// the frame words `+0x6d8`/`+0x6e0`, the flags `+0x6e4`/`+0x6d0` and
/// `+0x6f0`) is queued to the render thread (`+0x570`), or run here: with
/// the texture flag, new planes copy the frame to `+0x6e0` and set the
/// frame from the movie time (`u64` time to f32, times the two image
/// constants, `ftol2_sse`) while below 0xfffe, and mark a frame shown;
/// the record is freed. A shown frame with the pause request (`+0x6fd`,
/// no frame wait, not yet paused) pauses the movie (`+0x6fc`). Returns 0.
pub fn fmv_update_texture(h: &mut impl Host, this: u32) -> u32 {
    let at = |off: u32| this.wrapping_add(off);
    if h.read::<u8>(at(0x6d2)) == 0 {
        return 0;
    }
    let shown = at(0x6e4);
    if h.read::<u8>(shown) != 0 && h.read::<u8>(at(0x6fc)) != 0 {
        return 0;
    }
    let player = at(0x38);
    if h.call_sym(IS_PLAYING, Conv::Thiscall(player), &[]) as u8 == 0 {
        if h.read::<u8>(shown) != 0 {
            movie_end(h, this);
        }
        return 0;
    }
    if h.read::<u8>(at(0x6fe)) == 0 && h.read::<u8>(shown) != 0 {
        let now = h.call_sym_u64(CURRENT_TIME, Conv::Thiscall(player), &[]);
        let end = h.call_sym_u64(END_TIME, Conv::Thiscall(player), &[]);
        if now >= end || h.read::<u8>(h.rebase(crate::h4_i2::MOVIES_OFF)) != 0 {
            movie_end(h, this);
            return 0;
        }
    }
    let record = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x24]);
    if record != 0 {
        for k in 0..9u32 {
            h.write(record.wrapping_add(4 * k), 0u32);
        }
    }
    for (k, off) in [
        (1u32, 0x38cu32),
        (2, 0x400),
        (3, 0x474),
        (4, 0x6d8),
        (5, 0x6e0),
    ] {
        h.write(record.wrapping_add(4 * k), at(off));
    }
    h.write(record, player);
    h.write(record.wrapping_add(0x18), shown);
    h.write(record.wrapping_add(0x1c), at(0x6d0));
    h.write(record.wrapping_add(0x20), at(0x6f0));
    if h.read::<u8>(at(0x570)) != 0 {
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let f = h.rebase(UPDATE_CALLBACK);
        h.call_sym(
            QUEUE_CALLBACK,
            Conv::Thiscall(app.wrapping_add(0x124)),
            &[f, record],
        );
    } else {
        if h.read::<u8>(at(0x6d0)) != 0
            && h.call_sym(
                UPDATE_PLANES,
                Conv::Thiscall(player),
                &[at(0x38c), at(0x400), at(0x474)],
            ) as u8
                != 0
        {
            let frame_at: u32 = h.read(record.wrapping_add(0x10));
            let frame: u32 = h.read(frame_at);
            if (frame as i32) < 0xfffe {
                let last_at: u32 = h.read(record.wrapping_add(0x14));
                h.write(last_at, frame);
                let player: u32 = h.read(record);
                let now = h.call_sym_u64(CURRENT_TIME, Conv::Thiscall(player), &[]);
                // fild of the low 63 bits plus 2^63 for the top one, one
                // 53-bit rounding: the u64 as f64; then stored as f32
                #[allow(clippy::cast_precision_loss)] // the original rounds it too
                let t = load(store(now as f64));
                let c0 = f64::from_bits(h.read::<u64>(h.rebase(TIME_SCALE[0])));
                let c1 = f64::from_bits(h.read::<u64>(h.rebase(TIME_SCALE[1])));
                let v = ftol(h, mul(mul(t, c0), c1));
                let frame_at: u32 = h.read(record.wrapping_add(0x10));
                h.write(frame_at, v);
            }
            let flag_at: u32 = h.read(record.wrapping_add(0x18));
            h.write(flag_at, 1u8);
        }
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[record]);
    }
    if h.read::<u8>(shown) != 0
        && h.read::<u8>(at(0x6fd)) != 0
        && h.read::<u32>(h.rebase(WAIT_FRAMES_BY_TIME)) == 0
        && h.read::<u8>(at(0x6fc)) == 0
    {
        h.call_sym(TOGGLE_PAUSE, Conv::Thiscall(player), &[]);
        h.write(at(0x6fc), 1u8);
    }
    0
}
