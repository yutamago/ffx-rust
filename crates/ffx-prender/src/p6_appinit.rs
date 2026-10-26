//! `PApplication::onInit` (`0x0062a600`) of list p6-render-c
//! (continuation round 5): the application's start-up.
//!
//! In the original's order:
//! - the clock fields reset (`+0x20` 0, `+0x28` = `GetTime() -
//!   g_AppStartTime`, `+0x3c` = the fixed step global `0x00cc9d10`), the
//!   quit byte, the frame count, the "first frame" byte `+0x14`, and the
//!   frame function `+0x268` (`PApplication::frame`);
//! - a script: a script buffer (`0x00cc9cfc`, its proxy built with
//!   `operator new(0x28)`), or a script server (`0x00cc9d08`, given the
//!   fixed step in text; a zero step becomes `0x00b412c0` with a
//!   message), or a script file (`0x00cc9d0c`, its name formatted into a
//!   `PhyreMalloc` buffer handed to the proxy as a string). A proxy with
//!   commands (`+8`) makes the frame function the script's
//!   (`0x0062c6f0`);
//! - under a render scope lock (`ebp - 0x114`): the renderer singleton
//!   (`0x00c94f00`, built once), its window rectangle (`+0x2a0..+0x2ac`,
//!   or `+0x2b0`/`+0x2b4` placed by the renderer's windowed byte), the
//!   window (`initWindow`), the texture quality;
//! - the world, the render thread, `DirectInput`, the virtual `onInit`
//!   (slot 2, the function's two words), the renderer (`+0x124`, with a
//!   2.5 MB `PhyreMalloc` block at `+0x2d4`), the preprocess ring
//!   (`+0x2d8`, 512 KB), the job pool (`+0x40`, more than one thread),
//!   the profiler timeline (`+0xc`), the timer, the inputs, the virtual
//!   slot 3.
//!
//! Every failing step's `eax` is the function's (some printed first);
//! `0xd` when the renderer's block cannot be allocated; 0 at the end.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// The application's globals: the fixed step (f32), the script buffer,
/// the script server address, the script file name.
pub const FIXED_STEP: u32 = 0x00cc_9d10;
pub const SCRIPT_BUFFER: u32 = 0x00cc_9cfc;
pub const SCRIPT_SERVER: u32 = 0x00cc_9d08;
pub const SCRIPT_FILE: u32 = 0x00cc_9d0c;
/// `Phyre_PTimerWin32_GetTime()` and `g_AppStartTime`.
pub const GET_TIME: u32 = 0x0042_fd80;
pub const APP_START_TIME: u32 = 0x00c9_0278;
/// The frame functions: `PApplication::frame` and the script's.
pub const FRAME: u32 = 0x0062_7940;
pub const SCRIPT_FRAME: u32 = 0x0062_c6f0;
/// `operator new(size)` (cdecl) and the script proxy: its buffer and
/// file constructors (thiscall), its vtable, its command registration.
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const PROXY_FROM_BUFFER: u32 = 0x0061_fc20;
pub const PROXY_FROM_FILE: u32 = 0x0061_fac0;
pub const PROXY_VTABLE: u32 = 0x00b3_fc9c;
pub const REGISTER_COMMANDS: u32 = 0x0062_bb60;
/// The script server: its connection (cdecl `(address)`) and send
/// (thiscall `(server; text, length)`).
pub const SERVER_CONNECT: u32 = 0x009f_0810;
pub const SERVER_SEND: u32 = 0x009f_06a0;
/// The default fixed step (f32), the message's step (f64).
pub const DEFAULT_STEP: u32 = 0x00b4_12c0;
pub const DEFAULT_STEP_TEXT: u32 = 0x00b4_12d0;
/// `0x006285b0(name)` (cdecl): the script file's number (plus one in the
/// name).
pub const FILE_NUMBER: u32 = 0x0062_85b0;
/// `PhyreMalloc(size, align)`, `PhyreFree(p)` (cdecl).
pub const MALLOC: u32 = 0x0042_fb80;
pub const FREE: u32 = 0x0042_fb20;
/// `sprintf_s`'s import slot (cdecl).
pub const SPRINTF_S: u32 = 0x00b0_c394;
/// `PhyrePrintf(level, format, ...)` (cdecl) and the formats.
pub const PRINTF: u32 = 0x0043_52f0;
pub const STEP_MESSAGE: u32 = 0x00b4_0788;
pub const STEP_FORMAT: u32 = 0x00b4_07b8;
pub const FILE_FORMAT: u32 = 0x00b4_07c8;
pub const WINDOW_ERROR: u32 = 0x00b4_07e8;
pub const THREAD_ERROR: u32 = 0x00b4_0824;
pub const INIT_ERROR: u32 = 0x00b4_0860;
pub const TIMELINE_NAME: u32 = 0x00b4_0894;
pub const LATE_ERROR: u32 = 0x00b4_08a0;
pub const ERROR_TEXT: u32 = 0x00c9_6298;
/// The render scope lock.
pub const LOCK_ENTER: u32 = 0x0046_3d50;
pub const LOCK_LEAVE: u32 = 0x0046_3e90;
/// The D3D11 renderer singleton, its guard (bit 0), constructor and
/// `atexit` destructor; its window rectangle (four words) and windowed
/// byte (`+0`).
pub const RENDERER: u32 = 0x00c9_4f00;
pub const RENDERER_GUARD: u32 = 0x00c9_628c;
pub const RENDERER_CTOR: u32 = 0x004f_ee70;
pub const RENDERER_ATEXIT: u32 = 0x00af_b710;
/// The CRT's `atexit(f)` (cdecl).
pub const ATEXIT: u32 = 0x0094_9207;
pub const RENDERER_RECT: u32 = 0x00c9_4f5c;
/// `PDynamicProcessBufferAllocation`'s constructor (thiscall, on a frame
/// slot).
pub const ALLOCATION_CTOR: u32 = 0x004f_ee50;
/// `PRendererD3D11::initWindow(this; window)`, `setTextureQuality`.
pub const INIT_WINDOW: u32 = 0x0059_5750;
pub const SET_TEXTURE_QUALITY: u32 = 0x0058_e000;
pub const SETTINGS: u32 = 0x022f_b504;
/// `PWorld::initialize`, the render thread, `DirectInput`.
pub const WORLD_INIT: u32 = 0x0044_3a40;
pub const THREAD_INSTANCE: u32 = 0x0061_4410;
pub const THREAD_INIT: u32 = 0x0061_5140;
pub const INIT_DIRECT_INPUT: u32 = 0x0062_4350;
/// `PRendererBase::initialize(this; block, a, b)`, the preprocess ring
/// and its range, the thread count and the job pool's set-up.
pub const RENDERER_INIT: u32 = 0x005b_8da0;
pub const RING: u32 = 0x005c_d010;
pub const RING_INIT: u32 = 0x0062_9840;
pub const THREAD_COUNT: u32 = 0x0044_cde0;
pub const POOL_INIT: u32 = 0x005b_bc30;
/// The profiler: the timeline's set-up and its registration with the
/// collection `0x00c2e2cc`.
pub const TIMELINE_INIT: u32 = 0x0060_ed70;
pub const PROFILER: u32 = 0x00c2_e2cc;
pub const TIMELINE_ADD: u32 = 0x0060_ed40;
/// `PApplication::stepTimer(this)`, `setupInputs(mapper)`.
pub const STEP_TIMER: u32 = 0x0062_8df0;
pub const SETUP_INPUTS: u32 = 0x0062_9870;

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)]
    use super::*;

    fn f32_at(h: &mut impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    /// `PhyrePrintf(level, format, text)`; `error`.
    fn report(h: &mut impl Host, level: u32, format: u32, error: u32) -> u32 {
        let format = h.rebase(format);
        let text = h.rebase(ERROR_TEXT);
        h.call_sym(PRINTF, Conv::Cdecl, &[level, format, text]);
        error
    }

    fn strlen(h: &mut impl Host, s: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// The proxy at `proxy` (0 for a failed `new`) becomes the
    /// application's (`+0x2c`); one with commands takes the frame.
    fn take_proxy(h: &mut impl Host, app: u32, proxy: u32) {
        h.write(app.wrapping_add(0x2c), proxy);
        if proxy != 0 && h.read::<u32>(proxy.wrapping_add(8)) != 0 {
            let f = h.rebase(SCRIPT_FRAME);
            h.write(app.wrapping_add(0x268), f);
        }
    }

    /// The proxy's vtable and its commands, after a constructor.
    fn finish_proxy(h: &mut impl Host, proxy: u32) {
        let vt = h.rebase(PROXY_VTABLE);
        h.write(proxy, vt);
        h.call_sym(REGISTER_COMMANDS, Conv::Thiscall(proxy), &[]);
    }

    /// The script: see the module.
    fn script(h: &mut impl Host, app: u32, ebp: u32) {
        let buffer: u32 = h.read(h.rebase(SCRIPT_BUFFER));
        if buffer != 0 {
            let proxy = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x28]);
            if proxy != 0 {
                h.call_sym(PROXY_FROM_BUFFER, Conv::Thiscall(proxy), &[buffer]);
                for (k, v) in [(0x14, 0), (0x18, 0), (0x1c, 0), (0x20, app), (0x24, 0)] {
                    h.write(proxy.wrapping_add(k), v);
                }
                finish_proxy(h, proxy);
            }
            take_proxy(h, app, proxy);
        }
        let server_at: u32 = h.read(h.rebase(SCRIPT_SERVER));
        if server_at != 0 {
            let server = h.call_sym(SERVER_CONNECT, Conv::Cdecl, &[server_at]);
            h.write(app.wrapping_add(0x29c), server);
            let step: u32 = h.read(app.wrapping_add(0x3c));
            if x87::load(step) == 0.0 {
                let d = x87::store(f32_at(h, h.rebase(DEFAULT_STEP)));
                h.write(app.wrapping_add(0x3c), d);
                let text: u64 = h.read(h.rebase(DEFAULT_STEP_TEXT));
                let msg = h.rebase(STEP_MESSAGE);
                #[allow(clippy::cast_possible_truncation)] // the double's halves
                h.call_sym(
                    PRINTF,
                    Conv::Cdecl,
                    &[2, msg, text as u32, (text >> 32) as u32],
                );
            }
            let server: u32 = h.read(app.wrapping_add(0x29c));
            if server != 0 {
                let step = f32_at(h, app.wrapping_add(0x3c)).to_bits();
                let buf = ebp.wrapping_sub(0x110);
                let sprintf: u32 = h.read(h.rebase(SPRINTF_S));
                let format = h.rebase(STEP_FORMAT);
                #[allow(clippy::cast_possible_truncation)] // the double's halves
                h.call(
                    sprintf,
                    Conv::Cdecl,
                    &[buf, 0xff, format, step as u32, (step >> 32) as u32],
                );
                let n = strlen(h, buf);
                h.call_sym(SERVER_SEND, Conv::Thiscall(server), &[buf, n]);
            }
            return;
        }
        if buffer != 0 {
            return;
        }
        let file: u32 = h.read(h.rebase(SCRIPT_FILE));
        if file == 0 {
            return;
        }
        let number = h
            .call_sym(FILE_NUMBER, Conv::Cdecl, &[file])
            .wrapping_add(1);
        let len = strlen(h, file).wrapping_add(0x23);
        let text = if len == 0 {
            0
        } else {
            h.call_sym(MALLOC, Conv::Cdecl, &[len, 1])
        };
        let size = if text == 0 { 0 } else { len };
        // the string the proxy's constructor takes: (size, text)
        h.write(ebp.wrapping_sub(0x11c), text);
        h.write(ebp.wrapping_sub(0x120), size);
        let sprintf: u32 = h.read(h.rebase(SPRINTF_S));
        let format = h.rebase(FILE_FORMAT);
        h.call(sprintf, Conv::Cdecl, &[text, len, format, file, number]);
        let proxy = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x28]);
        if proxy != 0 {
            h.call_sym(
                PROXY_FROM_FILE,
                Conv::Thiscall(proxy),
                &[app, ebp.wrapping_sub(0x120)],
            );
            finish_proxy(h, proxy);
        }
        take_proxy(h, app, proxy);
        // the string's release: owned while the size's top bit is clear
        if text != 0 && (size as i32) >= 0 {
            h.call_sym(FREE, Conv::Cdecl, &[text]);
        }
    }

    /// The renderer singleton, its window and texture quality under the
    /// lock at `lock`; the window's error.
    fn window(h: &mut impl Host, app: u32, ebp: u32) -> u32 {
        let guard_at = h.rebase(RENDERER_GUARD);
        let guard: u32 = h.read(guard_at);
        let renderer = h.rebase(RENDERER);
        if guard & 1 == 0 {
            h.write(guard_at, guard | 1);
            h.call_sym(RENDERER_CTOR, Conv::Thiscall(renderer), &[]);
            let f = h.rebase(RENDERER_ATEXIT);
            h.call_sym(ATEXIT, Conv::Cdecl, &[f]);
        }
        let rect: [u32; 4] = std::array::from_fn(|k| h.read(app + 0x2a0 + 4 * k as u32));
        let window: u32 = h.read(app.wrapping_add(0x264));
        let rect = if rect.iter().any(|&w| w != 0) {
            rect
        } else {
            let a: u32 = h.read(app.wrapping_add(0x2b0));
            let b: u32 = h.read(app.wrapping_add(0x2b4));
            h.call_sym(
                ALLOCATION_CTOR,
                Conv::Thiscall(ebp.wrapping_sub(0x128)),
                &[],
            );
            let rect = if h.read::<u8>(renderer) != 0 {
                [a, b, 0, 0]
            } else {
                [0, 0, a, b]
            };
            for (k, &w) in rect.iter().enumerate() {
                h.write(ebp.wrapping_sub(0x128) + 4 * k as u32, w);
            }
            rect
        };
        let at = h.rebase(RENDERER_RECT);
        for (k, &w) in rect.iter().enumerate() {
            h.write(at + 4 * k as u32, w);
        }
        h.call_sym(INIT_WINDOW, Conv::Thiscall(renderer), &[window])
    }

    /// `PApplication::onInit(this; a, b)` (thiscall, `ret 8`) with the
    /// original's frame at `ebp` (the text buffer `ebp - 0x110`, the
    /// lock `ebp - 0x114`, the file name string `ebp - 0x120`, the
    /// allocation `ebp - 0x128`): see the module.
    pub fn on_init(h: &mut impl Host, app: u32, a: u32, b: u32, ebp: u32) -> u32 {
        let at = |k: u32| app.wrapping_add(k);
        h.write(at(0x20), 0u32);
        h.write(at(0x39), 0u8);
        h.write(at(0x34), 0u32);
        h.write(at(0x14), 1u8);
        let now = h.call_sym_st0_f64(GET_TIME, Conv::Cdecl, &[]);
        let start = f64::from_bits(h.read(h.rebase(APP_START_TIME)));
        let frame = h.rebase(FRAME);
        h.write(at(0x268), frame);
        h.write(at(0x28), x87::store(x87::sub(now, start)));
        let step = x87::store(f32_at(h, h.rebase(FIXED_STEP)));
        h.write(at(0x3c), step);
        script(h, app, ebp);
        let lock = ebp.wrapping_sub(0x114);
        h.call_sym(LOCK_ENTER, Conv::Thiscall(lock), &[]);
        let r = window(h, app, ebp);
        if r != 0 {
            report(h, 2, WINDOW_ERROR, r);
            h.call_sym(LOCK_LEAVE, Conv::Thiscall(lock), &[]);
            return r;
        }
        let settings: u32 = h.read(h.rebase(SETTINGS));
        let quality: u32 = h.read(settings.wrapping_add(0x34));
        let renderer = h.rebase(RENDERER);
        h.call_sym(SET_TEXTURE_QUALITY, Conv::Thiscall(renderer), &[quality]);
        h.call_sym(LOCK_LEAVE, Conv::Thiscall(lock), &[]);
        let r = h.call_sym(WORLD_INIT, Conv::Thiscall(at(0x2c4)), &[]);
        if r != 0 {
            return r;
        }
        let thread = h.call_sym(THREAD_INSTANCE, Conv::Cdecl, &[]);
        let r = h.call_sym(THREAD_INIT, Conv::Thiscall(thread), &[]);
        if r != 0 {
            return report(h, 2, THREAD_ERROR, r);
        }
        let r = h.call_sym(INIT_DIRECT_INPUT, Conv::Cdecl, &[]);
        if r != 0 {
            return r;
        }
        let r = h.call_virtual(app, 2, &[a, b]);
        if r != 0 {
            return report(h, 2, INIT_ERROR, r);
        }
        let block = h.call_sym(MALLOC, Conv::Cdecl, &[0x28_0000, 0x10]);
        h.write(at(0x2d4), block);
        if block == 0 {
            return 0xd;
        }
        let r = h.call_sym(
            RENDERER_INIT,
            Conv::Thiscall(at(0x124)),
            &[block, 0x20_0000, 0x8_0000],
        );
        if r != 0 {
            return r;
        }
        let ring_block = h.call_sym(MALLOC, Conv::Cdecl, &[0x8_0000, 0x10]);
        h.write(at(0x2d8), ring_block);
        let ring = h.call_sym(RING, Conv::Cdecl, &[]);
        h.call_sym(RING_INIT, Conv::Thiscall(ring), &[ring_block, 0x8_0000]);
        let threads = h.call_sym(THREAD_COUNT, Conv::Cdecl, &[]);
        if threads > 1 {
            let r = h.call_sym(POOL_INIT, Conv::Thiscall(at(0x40)), &[threads, at(0x50)]);
            if r != 0 {
                return r;
            }
        }
        let name = h.rebase(TIMELINE_NAME);
        h.call_sym(TIMELINE_INIT, Conv::Thiscall(at(0xc)), &[name, 0x2000]);
        let profiler = h.rebase(PROFILER);
        let r = h.call_sym(TIMELINE_ADD, Conv::Thiscall(profiler), &[at(0xc)]);
        if r != 0 {
            return r;
        }
        h.call_sym(STEP_TIMER, Conv::Thiscall(app), &[]);
        let paused = f32_at(h, at(0x28));
        let clock = f32_at(h, at(0x20));
        let mapper: u32 = h.read(at(0x2b8));
        h.write(at(0x28), x87::store(x87::add(paused, clock)));
        h.write(at(0x20), 0u32);
        h.call_sym(SETUP_INPUTS, Conv::Thiscall(mapper), &[]);
        let r = h.call_virtual(app, 3, &[]);
        if r != 0 {
            return report(h, 3, LATE_ERROR, r);
        }
        0
    }
}
