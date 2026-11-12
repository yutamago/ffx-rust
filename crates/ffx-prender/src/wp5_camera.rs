//! M5 finish, WP5 part 2: `FreeCamera_handleInput` (`0x00680030`), the
//! debug free camera's per-frame input (8.8 k calls, 70 k emulated blocks
//! in gate run 1, from `PhyreScene_animate`). Its machine code, in order:
//! the on/off and pause toggles' messages and sounds, the mouse capture
//! (the cursor clipped to the window's client area and centred, or freed
//! and shown), the camera's update and its matrix.
//!
//! The free camera object: `+0` the capture counter, `+4` its last value,
//! `+8` a speed (`f32`), `+0xc` the controller, `+0x10` the matrix
//! block, `+0x2bc` the camera, the bytes `+0x338` (on), `+0x339` (its
//! last value), `+0x33a` (the cursor shown), `+0x33b` (the pause it
//! started).

use ffx_graphic::host::{Conv, Host};

/// Callees (cdecl unless noted).
pub mod at {
    /// `()` -> the message box; its `show(id)` (thiscall, `ret 4`).
    pub const MESSAGE: u32 = 0x006e_8750;
    pub const MESSAGE_SHOW: u32 = 0x006e_8770;
    /// `IconPanel_get()` -> the icon panel; `showIcon(i)`/`hideIcon(i)` on
    /// it (thiscall, `ret 4`).
    pub const SOUND: u32 = 0x006f_78b0;
    pub const SOUND_PLAY: u32 = 0x006f_7970;
    pub const SOUND_STOP: u32 = 0x006f_7900;
    /// `()` -> the pause controller; its byte `+0x6fc`; its toggle
    /// (thiscall).
    pub const PAUSE: u32 = 0x006d_73a0;
    pub const PAUSE_TOGGLE: u32 = 0x006d_a4b0;
    /// `(mask)` -> whether the key is held.
    pub const KEY_HELD: u32 = 0x0063_0cf0;
    /// `()` -> the input device; its window (thiscall, the `HWND` holder);
    /// its mouse centre (thiscall `(x, y)`); the camera step (thiscall
    /// `(camera, dt, speed, 1.0)`).
    pub const INPUT: u32 = 0x0062_3b90;
    pub const INPUT_WINDOW: u32 = 0x0062_97a0;
    pub const INPUT_SET_MOUSE: u32 = 0x0062_bea0;
    pub const INPUT_CAMERA_STEP: u32 = 0x0062_e3a0;
    /// The controller's update (thiscall `(x)`), the camera's follow
    /// (thiscall `(controller)`), the matrix block's update (thiscall).
    pub const CONTROLLER_UPDATE: u32 = 0x0045_6430;
    pub const CAMERA_FOLLOW: u32 = 0x0067_fda0;
    pub const MATRIX_UPDATE: u32 = 0x0067_20c0;
    /// `USER32` import slots: `ClipCursor`, `ShowCursor`, `GetClientRect`,
    /// `ClientToScreen`, `SetCursorPos` (stdcall).
    pub const CLIP_CURSOR_IAT: u32 = 0x00b0_c6c4;
    pub const SHOW_CURSOR_IAT: u32 = 0x00b0_c688;
    pub const GET_CLIENT_RECT_IAT: u32 = 0x00b0_c680;
    pub const CLIENT_TO_SCREEN_IAT: u32 = 0x00b0_c66c;
    pub const SET_CURSOR_POS_IAT: u32 = 0x00b0_c678;
    /// The word that frees the mouse when set; `g_GameSettings` (its `+8`
    /// the window mode).
    pub const MOUSE_FREE: u32 = 0x0112_a008;
    pub const GAME_SETTINGS: u32 = 0x022f_b504;
}

/// The frame words the port needs: the client rect (`+0`), the point
/// (`+0x10`), the clip rect (`+0x18`).
pub const FRAME_WORDS: usize = 10;

fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
    let f: u32 = h.read(h.rebase(slot));
    h.call(f, Conv::Stdcall, args)
}

fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
    h.call_sym(f, Conv::Cdecl, args)
}

fn this(h: &mut impl Host, f: u32, obj: u32, args: &[u32]) -> u32 {
    h.call_sym(f, Conv::Thiscall(obj), args)
}

/// The message `id` shown and icon `icon` shown or hidden.
fn notice(h: &mut impl Host, id: u32, icon: u32, show: bool) {
    let m = cdecl(h, at::MESSAGE, &[]);
    this(h, at::MESSAGE_SHOW, m, &[id]);
    let p = cdecl(h, at::SOUND, &[]);
    let f = if show { at::SOUND_PLAY } else { at::SOUND_STOP };
    this(h, f, p, &[icon]);
}

/// The cursor freed (`ClipCursor(NULL)`) and shown once.
fn release(h: &mut impl Host, cam: u32) {
    import(h, at::CLIP_CURSOR_IAT, &[0]);
    if h.read::<u8>(cam + 0x33a) == 0 {
        import(h, at::SHOW_CURSOR_IAT, &[1]);
        h.write(cam + 0x33a, 1u8);
    }
}

/// `FreeCamera_handleInput(this; dt, x)` (thiscall, `ret 8`; nothing
/// returned: the caller reads no `eax`). `frame`: [`FRAME_WORDS`] words
/// of game memory for the rects the Win32 calls fill.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)] // the original's int maths
pub fn handle_input(h: &mut impl Host, cam: u32, dt: u32, x: u32, frame: u32) {
    let on: u8 = h.read(cam + 0x338);
    if on != h.read::<u8>(cam + 0x339) {
        if on != 0 {
            notice(h, 0x2f3, 9, true);
            h.write(cam + 0x33b, 0u8);
            if cdecl(h, at::PAUSE, &[]) != 0 {
                let p = cdecl(h, at::PAUSE, &[]);
                if h.read::<u8>(p + 0x6fc) == 0 {
                    let p = cdecl(h, at::PAUSE, &[]);
                    this(h, at::PAUSE_TOGGLE, p, &[]);
                    h.write(cam + 0x33b, 1u8);
                }
            }
        } else {
            notice(h, 0x2f4, 9, false);
            if cdecl(h, at::PAUSE, &[]) != 0 && h.read::<u8>(cam + 0x33b) != 0 {
                let p = cdecl(h, at::PAUSE, &[]);
                this(h, at::PAUSE_TOGGLE, p, &[]);
            }
        }
        let on: u8 = h.read(cam + 0x338);
        h.write(cam + 0x339, on);
    }
    let count: u32 = h.read(cam);
    if count != h.read::<u32>(cam + 4) {
        if count != 0 {
            notice(h, 0x2ed, 8, true);
        } else {
            notice(h, 0x2ee, 8, false);
        }
        let count: u32 = h.read(cam);
        h.write(cam + 4, count);
    }
    if h.read::<u32>(h.rebase(at::MOUSE_FREE)) != 0 {
        h.write(cam, 0u32);
        release(h, cam);
    }
    if cdecl(h, at::KEY_HELD, &[0x4_0000]) != 0 {
        let count = h.read::<u32>(cam).wrapping_sub(1);
        h.write(cam, count);
        if (count as i32) < 0 {
            h.write(cam, 1u32);
            capture(h, cam, frame);
        } else {
            release(h, cam);
        }
        if h.read::<u32>(cam) == 1 && h.read::<u8>(cam + 0x338) == 0 {
            this(h, at::CONTROLLER_UPDATE, cam + 0xc, &[x]);
            this(h, at::CAMERA_FOLLOW, cam + 0x2bc, &[cam + 0xc]);
        }
    }
    if h.read::<u32>(cam) != 0 {
        let input = cdecl(h, at::INPUT, &[]);
        let speed: u32 = h.read(cam + 8);
        this(
            h,
            at::INPUT_CAMERA_STEP,
            input,
            &[cam + 0x2bc, dt, speed, 0x3f80_0000],
        );
        this(h, at::MATRIX_UPDATE, cam + 0x10, &[]);
    }
}

/// A signed word.
#[allow(clippy::cast_possible_wrap)]
fn rd(h: &impl Host, a: u32) -> i32 {
    h.read::<u32>(a) as i32
}

/// The mouse captured: the cursor clipped to the client area (4 pixels in
/// at the right and bottom unless windowed), centred, hidden once; the
/// input's mouse centre set.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
fn capture(h: &mut impl Host, cam: u32, frame: u32) {
    let input = cdecl(h, at::INPUT, &[]);
    let window = this(h, at::INPUT_WINDOW, input, &[]);
    let (rect, point, clip) = (frame, frame + 0x10, frame + 0x18);
    for i in 0..4 {
        h.write(rect + 4 * i, 0u32);
    }
    let hwnd: u32 = h.read(window);
    import(h, at::GET_CLIENT_RECT_IAT, &[hwnd, rect]);
    let height = rd(h, rect + 0xc).wrapping_sub(rd(h, rect + 4));
    let width = rd(h, rect + 8).wrapping_sub(rd(h, rect));
    h.write(point, 0u32);
    h.write(point + 4, 0u32);
    let hwnd: u32 = h.read(window);
    import(h, at::CLIENT_TO_SCREEN_IAT, &[hwnd, point]);
    let top = rd(h, rect + 4).wrapping_add(rd(h, point + 4));
    let left = rd(h, point).wrapping_add(rd(h, rect));
    let bottom = height.wrapping_add(top);
    let right = left.wrapping_add(width);
    for (i, v) in [left, top, right, bottom].into_iter().enumerate() {
        h.write(clip + 4 * i as u32, v as u32);
    }
    let cy = bottom.wrapping_add(top) >> 1;
    let cx = right.wrapping_add(left) >> 1;
    import(h, at::SET_CURSOR_POS_IAT, &[cx as u32, cy as u32]);
    if h.read::<u8>(cam + 0x33a) != 0 {
        import(h, at::SHOW_CURSOR_IAT, &[0]);
        h.write(cam + 0x33a, 0u8);
    }
    let mut my = height >> 1;
    let mut mx = width >> 1;
    let settings: u32 = h.read(h.rebase(at::GAME_SETTINGS));
    if h.read::<u32>(settings + 8) == 0 {
        h.write(clip + 0xc, bottom.wrapping_sub(4) as u32);
        mx = mx.wrapping_add(4);
        my = my.wrapping_add(4);
        h.write(clip + 8, right.wrapping_sub(4) as u32);
    }
    import(h, at::CLIP_CURSOR_IAT, &[clip]);
    let input = cdecl(h, at::INPUT, &[]);
    this(h, at::INPUT_SET_MOUSE, input, &[mx as u32, my as u32]);
}
