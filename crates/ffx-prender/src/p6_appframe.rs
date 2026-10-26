//! `PApplication::frame` (`0x00627940`) of list p6-render-c (continuation
//! round 5): one frame of the application's loop.
//!
//! The steps, in the original's order:
//! - the frame time: with a fixed step (`+0x3c` above 0) the elapsed time
//!   `+0x24` is that step and the clock `+0x20` advances by it; else the
//!   clock is `GetTime() - g_AppStartTime - +0x28` and the elapsed time
//!   its change. `+0x120` keeps a copy of the clock;
//! - byte `+0x39` set: 9 (quit);
//! - the first frame (`+0x34` 0) has an elapsed time of 0 and runs the
//!   virtual `onFirstFrame` (slot 9; an error ends the frame);
//! - the profiler reset (`0x0060ee00` on `0x00c2e2cc`), the frame
//!   counters, the keyboard log and the input mapper's latch;
//! - the virtual `update` (slot 7);
//! - the frame-rate counter: a second's worth (the constant `0x00b92268`,
//!   2.0) of elapsed time prints the rate (`PhyrePrintf`) and sets
//!   `+0x30` and byte `+0x38`;
//! - the virtual `animate` (slot 4; an error is printed and ends the
//!   frame);
//! - pending settings (bytes `+0x14` to `+0x18`, `+0x1d`): after a sync of
//!   the renderer (`+0x124`), the virtual slot 9 again, texture quality,
//!   anisotropy, display mode and vertical sync (twice: `+0x18`, `+0x1d`),
//!   each under a render scope lock in the frame (`ebp - 0x10` to
//!   `ebp - 0x1c`);
//! - each viewport's virtual render (slot 3; an error is printed and ends
//!   the frame), the renderer's viewport put back after each;
//! - the virtual `render` (slot 8), the flip, the renderer's slot 2 and
//!   the application's slot 10; 0.
//!
//! An error is the callee's `eax`, returned as the frame's.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// `Phyre_PTimerWin32_GetTime()` (cdecl, a double in `ST0`).
pub const GET_TIME: u32 = 0x0042_fd80;
/// `g_AppStartTime` (f64).
pub const APP_START_TIME: u32 = 0x00c9_0278;
/// The profiler collection and its reset (`this`; timeline, elapsed).
pub const PROFILER: u32 = 0x00c2_e2cc;
pub const PROFILER_RESET: u32 = 0x0060_ee00;
/// The global frame counter and the "no window" flag byte.
pub const FRAME_COUNT: u32 = 0x012f_b8c4;
pub const NO_WINDOW: u32 = 0x00cc_9cf4;
/// `logKeyboardInput(this)`, `PInputMapper_latchButtons(mapper)`.
pub const LOG_KEYBOARD: u32 = 0x0062_9e40;
pub const LATCH_BUTTONS: u32 = 0x0062_8320;
/// The frame-rate counter: frames and seconds since the last print, the
/// print interval (f32), 1000.0 (f64) and 2^32 (f32, the unsigned fix-up).
pub const RATE_FRAMES: u32 = 0x00cc_a1b0;
pub const RATE_TIME: u32 = 0x00cc_a1b4;
pub const RATE_INTERVAL: u32 = 0x00b9_2268;
pub const MS_PER_SECOND: u32 = 0x00b0_dba0;
pub const TWO_POW_32: u32 = 0x00b1_1f14;
/// `PhyrePrintf(level, format, ...)` (cdecl) and its formats.
pub const PRINTF: u32 = 0x0043_52f0;
pub const RATE_FORMAT: u32 = 0x00b4_0a34;
/// `SaveDataTtyWrite2(level, format, ...)` (cdecl).
pub const TTY_WRITE: u32 = 0x0064_8850;
/// The error string the messages print (`%s`).
pub const ERROR_TEXT: u32 = 0x00c9_6298;
/// The three error reports: (printf format, tty format), for animate,
/// the settings' slot 9, a viewport's render and render.
pub const ANIMATE_ERROR: (u32, u32) = (0x00b4_0a50, 0x00b4_0a7c);
pub const SETTINGS_ERROR: (u32, u32) = (0x00b4_0abc, 0x00b4_0ae8);
pub const VIEWPORT_ERROR: (u32, u32) = (0x00b4_0b28, 0x00b4_0b60);
pub const RENDER_ERROR: (u32, u32) = (0x00b4_0ba8, 0x00b4_0bd4);
/// `PRenderer::syncRender` (with the renderer in `ecx`).
pub const SYNC_RENDER: u32 = 0x005b_c3f0;
/// The render scope lock: enter, the renderer context, leave.
pub const LOCK_ENTER: u32 = 0x0046_3d50;
pub const CONTEXT: u32 = 0x0046_49d0;
pub const LOCK_LEAVE: u32 = 0x0046_3e90;
/// `g_Settings` (a pointer).
pub const SETTINGS: u32 = 0x022f_b504;
/// The settings' setters.
pub const SET_TEXTURE_QUALITY: u32 = 0x0058_e000;
pub const SET_ANISOTROPY: u32 = 0x0058_de30;
pub const SET_DISPLAY_MODE: u32 = 0x0058_df00;
pub const SET_VSYNC: u32 = 0x0058_e170;
/// `PRendererBase::setViewport(this; x, y, w, h)`, `PRenderer::flip`.
pub const SET_VIEWPORT: u32 = 0x005b_c060;
pub const FLIP: u32 = 0x005b_6af0;
/// The renderer inside the application.
pub const RENDERER: u32 = 0x124;

pub mod bridge {
    //! The original over game memory.

    use super::{
        ANIMATE_ERROR, APP_START_TIME, CONTEXT, Conv, ERROR_TEXT, FLIP, FRAME_COUNT, GET_TIME,
        Host, LATCH_BUTTONS, LOCK_ENTER, LOCK_LEAVE, LOG_KEYBOARD, MS_PER_SECOND, NO_WINDOW,
        PRINTF, PROFILER, PROFILER_RESET, RATE_FORMAT, RATE_FRAMES, RATE_INTERVAL, RATE_TIME,
        RENDER_ERROR, RENDERER, SET_ANISOTROPY, SET_DISPLAY_MODE, SET_TEXTURE_QUALITY,
        SET_VIEWPORT, SET_VSYNC, SETTINGS, SETTINGS_ERROR, SYNC_RENDER, TTY_WRITE, TWO_POW_32,
        VIEWPORT_ERROR, x87,
    };

    fn f32_at(h: &mut impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    /// The error report: `PhyrePrintf(3, format, text)`, then
    /// `SaveDataTtyWrite2(0xc, tty, error, text)`; the error.
    fn report(h: &mut impl Host, (format, tty): (u32, u32), error: u32) -> u32 {
        let text = h.rebase(ERROR_TEXT);
        let format = h.rebase(format);
        h.call_sym(PRINTF, Conv::Cdecl, &[3, format, text]);
        let tty = h.rebase(tty);
        h.call_sym(TTY_WRITE, Conv::Cdecl, &[0xc, tty, error, text]);
        error
    }

    /// The settings word at `+off` of `g_Settings`.
    fn setting(h: &mut impl Host, off: u32) -> u32 {
        let settings: u32 = h.read(h.rebase(SETTINGS));
        h.read(settings.wrapping_add(off))
    }

    /// The vertical sync value: 0 when the application forces it off
    /// (byte `+0x1c`), else whether the setting `+0x28` is on.
    fn vsync(h: &mut impl Host, app: u32) -> u32 {
        let off = h.read::<u8>(app.wrapping_add(0x1c)) != 0;
        let v = if off { 0 } else { setting(h, 0x28) };
        u32::from(v != 0)
    }

    /// `PApplication::frame(this)` (thiscall, no words) with the
    /// original's frame at `ebp` (its render scope locks at `ebp - 0x10`
    /// to `ebp - 0x1c`): see the module.
    #[allow(clippy::too_many_lines)] // one original
    pub fn frame(h: &mut impl Host, app: u32, ebp: u32) -> u32 {
        let at = |k: u32| app.wrapping_add(k);
        // the frame time
        let step: u32 = h.read(at(0x3c));
        let step_f = x87::load(step);
        if 0.0 < step_f {
            h.write(at(0x24), x87::store(step_f));
            let clock = f32_at(h, at(0x20));
            h.write(at(0x20), x87::store(x87::add(clock, step_f)));
        } else {
            let now = h.call_sym_st0_f64(GET_TIME, Conv::Cdecl, &[]);
            let start: f64 = f64::from_bits(h.read(h.rebase(APP_START_TIME)));
            let base = f32_at(h, at(0x28));
            let t = x87::sub(x87::sub(now, start), base);
            let clock = f32_at(h, at(0x20));
            h.write(at(0x24), x87::store(x87::sub(t, clock)));
            h.write(at(0x20), x87::store(t));
        }
        let clock: u32 = h.read(at(0x20));
        h.write(at(0x120), x87::store(x87::load(clock)));
        if h.read::<u8>(at(0x39)) != 0 {
            return 9;
        }
        if h.read::<u32>(at(0x34)) == 0 {
            h.write(at(0x24), 0u32);
            let r = h.call_virtual(app, 9, &[]);
            if r != 0 {
                return r;
            }
        }
        let renderer = at(RENDERER);
        let elapsed = x87::store(f32_at(h, at(0x24)));
        let profiler = h.rebase(PROFILER);
        h.call_sym(
            PROFILER_RESET,
            Conv::Thiscall(profiler),
            &[renderer, elapsed],
        );
        let count = h.rebase(FRAME_COUNT);
        let n: u32 = h.read(count);
        h.write(count, n.wrapping_add(1));
        let frames: u32 = h.read(at(0x34));
        h.write(at(0x34), frames.wrapping_add(1));
        let no_window = h.read::<u8>(at(0x288)) == 0;
        h.write(h.rebase(NO_WINDOW), u8::from(no_window));
        if h.read::<u32>(at(0x29c)) != 0 {
            h.call_sym(LOG_KEYBOARD, Conv::Thiscall(app), &[]);
        }
        let mapper: u32 = h.read(at(0x2b8));
        h.call_sym(LATCH_BUTTONS, Conv::Thiscall(mapper), &[]);
        let r = h.call_virtual(app, 7, &[]);
        if r != 0 {
            return r;
        }
        // the frame rate
        let time_at = h.rebase(RATE_TIME);
        let frames_at = h.rebase(RATE_FRAMES);
        let acc = x87::store(x87::add(f32_at(h, time_at), f32_at(h, at(0x24))));
        let rate_frames = h.read::<u32>(frames_at).wrapping_add(1);
        h.write(frames_at, rate_frames);
        h.write(time_at, acc);
        let interval = f32_at(h, h.rebase(RATE_INTERVAL));
        if interval < x87::load(acc) {
            let mut count = f64::from(rate_frames as i32);
            if (rate_frames as i32) < 0 {
                count = x87::add(count, f32_at(h, h.rebase(TWO_POW_32)));
            }
            let count = x87::load(x87::store(count));
            h.write(at(0x38), 1u8);
            let rate = x87::store(x87::div(count, x87::load(acc)));
            h.write(at(0x30), rate);
            let rate = x87::load(rate);
            let ms: f64 = f64::from_bits(h.read(h.rebase(MS_PER_SECOND)));
            let per = x87::div(ms, rate).to_bits();
            let rate = rate.to_bits();
            let format = h.rebase(RATE_FORMAT);
            #[allow(clippy::cast_possible_truncation)] // the double's halves
            h.call_sym(
                PRINTF,
                Conv::Cdecl,
                &[
                    2,
                    format,
                    rate as u32,
                    (rate >> 32) as u32,
                    per as u32,
                    (per >> 32) as u32,
                ],
            );
            h.write(time_at, 0u32);
            h.write(frames_at, 0u32);
        }
        let r = h.call_virtual(app, 4, &[]);
        if r != 0 {
            return report(h, ANIMATE_ERROR, r);
        }
        // pending settings
        if (0x14..=0x18).any(|k| pending(h, app, k)) {
            h.call_virtual(renderer, 2, &[]);
            h.call_sym(SYNC_RENDER, Conv::Thiscall(renderer), &[]);
            if pending(h, app, 0x14) {
                let r = h.call_virtual(app, 9, &[]);
                if r != 0 {
                    return report(h, SETTINGS_ERROR, r);
                }
                h.write(at(0x14), 0u8);
            }
            if pending(h, app, 0x15) {
                let lock = ebp.wrapping_sub(0x10);
                if let Some(ctx) = enter(h, lock) {
                    let q = setting(h, 0x34);
                    h.call_sym(SET_TEXTURE_QUALITY, Conv::Thiscall(ctx), &[q]);
                }
                h.write(at(0x15), 0u8);
                h.call_sym(LOCK_LEAVE, Conv::Thiscall(lock), &[]);
            }
            if pending(h, app, 0x16) {
                let a = setting(h, 0x2c);
                h.call_sym(SET_ANISOTROPY, Conv::Cdecl, &[a]);
                h.write(at(0x16), 0u8);
            }
            if pending(h, app, 0x17) {
                let lock = ebp.wrapping_sub(0x14);
                if let Some(ctx) = enter(h, lock) {
                    let mode = setting(h, 4);
                    let w = ((mode as i32) >> 16) as u32;
                    let hgt = i32::from(mode as i16) as u32;
                    h.call_sym(SET_DISPLAY_MODE, Conv::Thiscall(ctx), &[w, hgt]);
                }
                h.write(at(0x17), 0u8);
                h.call_sym(LOCK_LEAVE, Conv::Thiscall(lock), &[]);
            }
            if pending(h, app, 0x18) {
                set_vsync(h, app, ebp.wrapping_sub(0x18));
                h.write(at(0x18), 0u8);
                h.call_sym(LOCK_LEAVE, Conv::Thiscall(ebp.wrapping_sub(0x18)), &[]);
            }
        }
        if pending(h, app, 0x1d) {
            set_vsync(h, app, ebp.wrapping_sub(0x1c));
            h.write(at(0x1d), 0u8);
            h.call_sym(LOCK_LEAVE, Conv::Thiscall(ebp.wrapping_sub(0x1c)), &[]);
        }
        // the viewports
        let viewport: [u32; 4] = std::array::from_fn(|k| h.read(renderer + 8 + 4 * k as u32));
        let mut i = 0;
        while i < h.read::<u32>(at(0x2bc)) & 0x7fff_ffff {
            let list: u32 = h.read(at(0x2c0));
            let view: u32 = h.read(list.wrapping_add(4 * i));
            let r = h.call_virtual(view, 3, &[]);
            if r != 0 {
                return report(h, VIEWPORT_ERROR, r);
            }
            h.call_sym(SET_VIEWPORT, Conv::Thiscall(renderer), &viewport);
            i += 1;
        }
        let r = h.call_virtual(app, 8, &[]);
        if r != 0 {
            return report(h, RENDER_ERROR, r);
        }
        let r = h.call_sym(FLIP, Conv::Thiscall(renderer), &[]);
        if r != 0 {
            return r;
        }
        let r = h.call_virtual(renderer, 2, &[]);
        if r != 0 {
            return r;
        }
        h.call_virtual(app, 10, &[]);
        0
    }

    /// Whether the application's byte `+k` (a pending setting) is set.
    fn pending(h: &mut impl Host, app: u32, k: u32) -> bool {
        h.read::<u8>(app.wrapping_add(k)) != 0
    }

    /// The render scope lock at `lock` entered; the renderer context, or
    /// `None` for none.
    fn enter(h: &mut impl Host, lock: u32) -> Option<u32> {
        h.call_sym(LOCK_ENTER, Conv::Thiscall(lock), &[]);
        let ctx = h.call_sym(CONTEXT, Conv::Thiscall(lock), &[]);
        (ctx != 0).then_some(ctx)
    }

    /// The vertical sync set under the lock at `lock` (entered here; the
    /// caller clears its byte and leaves).
    fn set_vsync(h: &mut impl Host, app: u32, lock: u32) {
        if let Some(ctx) = enter(h, lock) {
            let v = vsync(h, app);
            h.call_sym(SET_VSYNC, Conv::Thiscall(ctx), &[v]);
        }
    }
}
