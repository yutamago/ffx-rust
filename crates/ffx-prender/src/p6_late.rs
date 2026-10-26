//! List p6-render-c, continuation round 3: the post-processing module's
//! `Bind`, the window's focus change, and other late functions.
//!
//! Conventions are read from the disassembly; `eax` is returned wherever
//! the original defines it.

use ffx_graphic::host::{Conv, Host};

/// `Phyre_PTypeDispenser_RegisterType(desc)` (cdecl) and
/// `Phyre_PClassDescriptor_addToNamespace(desc)` (thiscall, no words).
pub const REGISTER_TYPE: u32 = 0x0043_95b0;
pub const ADD_TO_NAMESPACE: u32 = 0x0043_c140;

/// A step of [`POST_BIND`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindStep {
    /// A plain call (`cdecl`, no words): a class's `Bind`.
    Call(u32),
    /// A class descriptor registered and added to its namespace.
    Register(u32),
}

/// `0x00a18020`'s steps after the utility's set-up, in order.
pub const POST_BIND: &[BindStep] = &{
    use BindStep::{Call, Register};
    [
        Call(0x00a0_3ce0),
        Call(0x00a0_3500),
        Call(0x00a0_37c0),
        Register(0x0194_4510),
        Call(0x00a0_3990),
        Register(0x0194_3df0),
        Call(0x00a0_3c20),
        Register(0x0194_3bd8),
        Call(0x00a0_41c0),
        Call(0x00a0_3dc0),
        Register(0x0194_40e8),
        Register(0x0194_37b0),
        Register(0x0194_3978),
        Register(0x0194_4640),
        Register(0x0194_4218),
        Register(0x0194_3f20),
        Register(0x0194_3b40),
        Register(0x0194_3d08),
        Register(0x0194_4808),
        Register(0x0194_3718),
        Register(0x0194_38e0),
        Register(0x0194_45a8),
        Register(0x0194_4180),
        Register(0x0194_3e88),
        Register(0x0194_3aa8),
        Register(0x0194_3c70),
        Register(0x0194_4770),
    ]
};
/// `PUtility`'s set-up flag (`u8`) and the object whose virtual slot 1
/// runs when it is clear.
pub const UTILITY_READY: u32 = 0x00c0_a09c;
pub const UTILITY: u32 = 0x00c0_a37c;

/// `setFocus`' callees: `PhyrePrintf(level, fmt, ...)` (cdecl) and its
/// format `"[PWindowWin32Base] has focus %d\n"`, the CRT's `memset`, the
/// settings pointer (`+4`/`+6` the full-screen height and width as `i16`,
/// `+8` the window mode, 1 = full screen) and the window handle.
pub const PRINTF: u32 = 0x0043_52f0;
pub const FOCUS_FORMAT: u32 = 0x00b4_0d18;
pub const MEMSET: u32 = 0x0094_964c;
pub const SETTINGS: u32 = 0x022f_b504;
pub const HWND: u32 = 0x00cc_9ce8;
/// The user32 import slots (stdcall): `ChangeDisplaySettingsA`,
/// `GetWindowLongA`, `SetWindowLongA`, `GetClientRect`,
/// `AdjustWindowRect`, `SetWindowPos`.
pub const CHANGE_DISPLAY_SETTINGS: u32 = 0x00b0_c658;
pub const GET_WINDOW_LONG: u32 = 0x00b0_c6c0;
pub const SET_WINDOW_LONG: u32 = 0x00b0_c6bc;
pub const GET_CLIENT_RECT: u32 = 0x00b0_c680;
pub const ADJUST_WINDOW_RECT: u32 = 0x00b0_c6cc;
pub const SET_WINDOW_POS: u32 = 0x00b0_c6e0;
/// `setFocus`' frame from `ebp - 0xb0`: the `DEVMODEA` (0x9c bytes) at
/// `+0`, the `RECT` at `+0x9c`.
pub const FOCUS_FRAME: u32 = 0xac;

/// `PGlowGPUBase`'s vtable; its render targets released through
/// `0x00a0e520` (`+0x90..+0x98`) and inline (`+0x9c..+0xa8`); then
/// `PGlowBase`'s destructor `0x009fe020`, inlined.
pub const GLOW_GPU_VTABLE: u32 = 0x00b7_53e8;
pub const GLOW_GPU_TARGETS: [u32; 3] = [0x90, 0x94, 0x98];
pub const GLOW_GPU_INLINE: [u32; 4] = [0x9c, 0xa0, 0xa4, 0xa8];
pub const GLOW_BASE_DTOR: u32 = 0x009f_e020;

/// `PApplication::resize`' callees (thiscall): the lock object's
/// constructor and destructor on the frame, the render state cache
/// (`GetRenderStateCache`, cdecl), its viewport size `0x00594f40(cache;
/// &w, &h, &x)`, the back buffer's resize `0x00599210(cache; w, h)`, the
/// renderer's `setViewport` `0x005bc060(renderer; x, y, w, h)` and a view's
/// rectangle `0x0062cba0(view; x, y, w, h)`.
pub const LOCK_CTOR: u32 = 0x0046_3d50;
pub const UNLOCK: u32 = 0x0046_3e90;
pub const RENDER_STATE_CACHE: u32 = 0x004f_d8b0;
pub const VIEWPORT_OF: u32 = 0x0059_4f40;
pub const RESIZE_BACK_BUFFER: u32 = 0x0059_9210;
pub const SET_VIEWPORT: u32 = 0x005b_c060;
pub const SET_RECT: u32 = 0x0062_cba0;
/// `resize`'s frame from `ebp - 0x2c`.
pub const RESIZE_FRAME: u32 = 0x20;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ADD_TO_NAMESPACE, ADJUST_WINDOW_RECT, BindStep, CHANGE_DISPLAY_SETTINGS, Conv,
        FOCUS_FORMAT, GET_CLIENT_RECT, GET_WINDOW_LONG, HWND, Host, MEMSET, POST_BIND, PRINTF,
        REGISTER_TYPE, SET_WINDOW_LONG, SET_WINDOW_POS, SETTINGS, UTILITY, UTILITY_READY,
    };
    use super::{GLOW_BASE_DTOR, GLOW_GPU_INLINE, GLOW_GPU_TARGETS, GLOW_GPU_VTABLE};
    use super::{
        LOCK_CTOR, RENDER_STATE_CACHE, RESIZE_BACK_BUFFER, SET_RECT, SET_VIEWPORT, UNLOCK,
        VIEWPORT_OF,
    };
    use crate::p6_post::{TARGET_RELEASE, bridge::post_dtor, post_dtor as post_dtor_of};
    use crate::p6_small::bridge::target_release;

    /// `PGlowGPUBase`'s destructor (`0x009fe0d0`, thiscall): its vtable,
    /// its targets released (three through `0x00a0e520`, four inline),
    /// then `PGlowBase`'s destructor's body. `eax`: the last member
    /// destruction's.
    pub fn glow_gpu_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(GLOW_GPU_VTABLE);
        h.write(this, vt);
        for off in GLOW_GPU_TARGETS {
            let t: u32 = h.read(this + off);
            h.call_sym(TARGET_RELEASE, Conv::Cdecl, &[t]);
        }
        for off in GLOW_GPU_INLINE {
            let t: u32 = h.read(this + off);
            target_release(h, t);
        }
        post_dtor(h, this, &const { post_dtor_of(GLOW_BASE_DTOR) })
    }

    /// `0x00a18020()` (cdecl; `PUtilityPostProcessing`'s vtable slot 1):
    /// `PUtility`'s set-up (virtual slot 1 of `0x00c0a37c`) unless done,
    /// then [`POST_BIND`]. 0.
    pub fn post_bind(h: &mut impl Host) -> u32 {
        let ready: u8 = h.read(h.rebase(UTILITY_READY));
        if ready == 0 {
            let utility = h.rebase(UTILITY);
            h.call_virtual(utility, 1, &[]);
        }
        for &step in POST_BIND {
            match step {
                BindStep::Call(f) => {
                    h.call_sym(f, Conv::Cdecl, &[]);
                }
                BindStep::Register(d) => {
                    let d = h.rebase(d);
                    h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[d]);
                    h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(d), &[]);
                }
            }
        }
        0
    }

    /// A user32 function through its import slot (stdcall).
    fn user32(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    /// `PWindowWin32Base::setFocus(window; focus)` (`0x0062a460`, thiscall,
    /// `ret 4`; `this` unused): logs the change; in full-screen mode
    /// (settings `+8` = 1) switches the display to the settings' size
    /// (32 bits per pixel) on focus or back to the registry's mode on
    /// losing it, makes the window a popup sized to that client area,
    /// and puts it on top of the window order or at the bottom. `eax`:
    /// the settings pointer when not full screen, else `SetWindowPos`'.
    ///
    /// `frame`: the original's `ebp - 0xb0` (the `DEVMODEA`, then the
    /// `RECT` at `+0x9c`), which the user32 calls see.
    pub fn set_focus(h: &mut impl Host, frame: u32, focus: u32) -> u32 {
        let on = focus as u8;
        let format = h.rebase(FOCUS_FORMAT);
        h.call_sym(PRINTF, Conv::Cdecl, &[2, format, u32::from(on)]);
        let settings: u32 = h.read(h.rebase(SETTINGS));
        let mode: u32 = h.read(settings + 8);
        if mode != 1 {
            return settings;
        }
        let width = i32::from(h.read::<i16>(settings + 6)) as u32;
        let height = i32::from(h.read::<i16>(settings + 4)) as u32;
        h.call_sym(MEMSET, Conv::Cdecl, &[frame, 0, 0x9c]);
        h.write(frame + 0x24, 0x9cu16); // dmSize
        h.write(frame + 0x6c, width); // dmPelsWidth
        h.write(frame + 0x70, height); // dmPelsHeight
        h.write(frame + 0x68, 0x20u32); // dmBitsPerPel
        h.write(frame + 0x28, 0x001c_0000u32); // dmFields
        let hwnd_at = h.rebase(HWND);
        if on == 0 {
            user32(h, CHANGE_DISPLAY_SETTINGS, &[0, 0x4000_0000]);
        } else {
            user32(h, CHANGE_DISPLAY_SETTINGS, &[frame, 4]);
        }
        let hwnd: u32 = h.read(hwnd_at);
        user32(h, GET_WINDOW_LONG, &[hwnd, 0xffff_fff0]);
        let hwnd: u32 = h.read(hwnd_at);
        user32(h, SET_WINDOW_LONG, &[hwnd, 0xffff_fff0, 0x8000_0000]);
        let rect = frame + 0x9c;
        let hwnd: u32 = h.read(hwnd_at);
        user32(h, GET_CLIENT_RECT, &[hwnd, rect]);
        for (k, v) in [0, 0, width, height].into_iter().enumerate() {
            h.write(rect + 4 * k as u32, v);
        }
        user32(h, ADJUST_WINDOW_RECT, &[rect, 0x8000_0000, 0]);
        let left: u32 = h.read(rect);
        let top: u32 = h.read(rect + 4);
        let right: u32 = h.read(rect + 8);
        let bottom: u32 = h.read(rect + 0xc);
        let after = if on == 0 { 0xffff_fffe } else { 0 };
        let hwnd: u32 = h.read(hwnd_at);
        user32(
            h,
            SET_WINDOW_POS,
            &[
                hwnd,
                after,
                0,
                0,
                right.wrapping_sub(left),
                bottom.wrapping_sub(top),
                0x40,
            ],
        )
    }

    /// `PApplication::resize(app)` (`0x0062c260`, thiscall; vtable slot 9,
    /// and `FFXApplication_resize`'s tail): under the lock object in the
    /// frame (`0x00463d50`/`0x00463e90`), the render state cache's viewport
    /// size read into the frame (`0x00594f40(cache; &w, &h, &x)`), the
    /// back buffer resized to the application's size (`+0x26c`, `+0x270`;
    /// `0x00599210`), the renderer's viewport set (`+0x124`,
    /// `0x005bc060(0, 0, w, h)`; its error returned); then every view
    /// (`+0x2bc` count, pointers at `+0x2c0`) has its rectangle (`+0x14..
    /// +0x20`) scaled from the old size to the new one when they differ
    /// (`0x0062cba0`) and its virtual slot 4 called, the first error
    /// returned. 0.
    ///
    /// `frame`: the original's `ebp - 0x2c` (x, the lock at `+4`, height
    /// `+0x18`, width `+0x1c`). The original divides by the old size; a 0
    /// there is a divide fault in it, and 0 here (shims never panic).
    pub fn resize(h: &mut impl Host, app: u32, frame: u32) -> u32 {
        let lock = frame + 4;
        h.call_sym(LOCK_CTOR, Conv::Thiscall(lock), &[]);
        let cache = h.call_sym(RENDER_STATE_CACHE, Conv::Cdecl, &[]);
        let (w_at, h_at) = (frame + 0x1c, frame + 0x18);
        h.call_sym(VIEWPORT_OF, Conv::Thiscall(cache), &[w_at, h_at, frame]);
        let cache = h.call_sym(RENDER_STATE_CACHE, Conv::Cdecl, &[]);
        let (w, ht): (u32, u32) = (h.read(app + 0x26c), h.read(app + 0x270));
        h.call_sym(RESIZE_BACK_BUFFER, Conv::Thiscall(cache), &[w, ht]);
        let (w, ht): (u32, u32) = (h.read(app + 0x26c), h.read(app + 0x270));
        let mut e = h.call_sym(SET_VIEWPORT, Conv::Thiscall(app + 0x124), &[0, 0, w, ht]);
        if e == 0 {
            let mut i = 0u32;
            while i < h.read::<u32>(app + 0x2bc) & 0x7fff_ffff {
                let w: u32 = h.read(app + 0x26c);
                let old_w: u32 = h.read(w_at);
                let old_h: u32 = h.read(h_at);
                if w != old_w || h.read::<u32>(app + 0x270) != old_h {
                    let views: u32 = h.read(app + 0x2c0);
                    let view: u32 = h.read(views + 4 * i);
                    let div = |a: u32, b: u32| a.checked_div(b).unwrap_or(0);
                    let x = div(w.wrapping_mul(h.read(view + 0x14)), old_w);
                    let ht: u32 = h.read(app + 0x270);
                    let y = div(ht.wrapping_mul(h.read(view + 0x18)), old_h);
                    let w: u32 = h.read(app + 0x26c);
                    let vw = div(w.wrapping_mul(h.read(view + 0x1c)), old_w);
                    let vh = div(ht.wrapping_mul(h.read(view + 0x20)), old_h);
                    h.call_sym(SET_RECT, Conv::Thiscall(view), &[x, y, vw, vh]);
                }
                let views: u32 = h.read(app + 0x2c0);
                let view: u32 = h.read(views + 4 * i);
                e = h.call_virtual(view, 4, &[]);
                if e != 0 {
                    break;
                }
                i += 1;
            }
        }
        h.call_sym(UNLOCK, Conv::Thiscall(lock), &[]);
        e
    }
}
