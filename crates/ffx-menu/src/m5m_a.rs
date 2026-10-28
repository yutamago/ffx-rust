//! M5 stage 1, list `menu`, batch 1: the `Gu_` debug GUI's frame (the
//! mouse handling of `Gu_VSync`, the windows' hit tests and control
//! messages, the packet's closing tag and the mouse cursor), reached from
//! `Sg_MainDebug` > `Gu_Main`.
//!
//! Layouts as in [`crate::gu`]; what these functions add:
//!
//! - **Window:** `+0x04` the hit-test list's next window (`Gu_ListCopyNext`
//!   copies `+0x00` into it before each pass), `+0x8c` a scroll position
//!   (`i16`, 0..=0x80, moved by the mouse wheel when flag `0x80` is set),
//!   `+0x8e` the vertical scale handed to `Gu_SetHScale`.
//! - **Control** (`0x44` bytes): `+0x00..+0x08` its rectangle (`i16` x,
//!   y, w, h, relative to the window's `+0x0c`/`+0x0e`), `+0x1a` its
//!   highlight (0x80 idle, 0xc0 under the mouse), `+0x20` pressed (`u16`),
//!   `+0x2c` kind (`i16`: 1 button, 3 static, 4 vertical scroll bar, 6
//!   client, 7 resize grip, 8 title bar), `+0x2e` id, `+0x40` flags (bit 0
//!   hidden, bit 1 disabled).
//! - **Mouse** ([`bridge::MOUSE_BUTTONS`], `Sg_MouseButtonMain`): per
//!   button (left bits 0-3, right bits 8-11, middle bits 16-19) down,
//!   pressed, released, double-clicked; [`bridge::MOUSE_WHEEL`] the wheel's
//!   steps since the last frame.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

pub mod bridge {
    use ffx_bridge::{Global, Mem};
    use ffx_graphic::host::{Conv, Host};

    use crate::gu::bridge::{
        DRAG_WINDOW, H_SCALE, HIDDEN, MOUSE_X, MOUSE_Y, PACKET, PACKET_COUNT, WINDOW_LIST,
    };

    /// `Sg_MouseButtonMain`'s button bits (down, pressed, released, double
    /// click for left, right and middle).
    pub const MOUSE_BUTTONS: Global<u32> = Global::new(0x023c_bc10);
    /// The mouse wheel's steps (`Sg_MouseMain` adds to it).
    pub const MOUSE_WHEEL: Global<i32> = Global::new(0x023c_bc04);
    /// The screen height and width (`Sg_SetScreenWH`).
    pub const SCREEN_H: Global<i32> = Global::new(0x023c_bbe0);
    /// The screen width.
    pub const SCREEN_W: Global<i32> = Global::new(0x023c_bbe4);
    /// The window being resized (`Gu_VSync` moves its grip).
    pub const RESIZE_WINDOW: Global<u32> = Global::new(0x0130_373c);
    /// The selector's popup window (`Gu_SetPopupWindow`).
    pub const POPUP_WINDOW: Global<u32> = Global::new(0x0130_3740);
    /// The drag's grab offset x, y (from the window's position).
    pub const DRAG_GRAB: Global<[i32; 2]> = Global::new(0x0130_3744);
    /// The resize's grab offset x, y (from the window's size).
    pub const RESIZE_GRAB: Global<[i32; 2]> = Global::new(0x0130_374c);
    /// The window holding the mouse capture (`Gu_MouseSetCapture`'s first
    /// word; -1 the desktop, for the camera).
    pub const CAPTURE: Global<u32> = Global::new(0x0130_3754);
    /// The id of the control holding the mouse capture.
    pub const CAPTURE_ID: Global<u32> = Global::new(0x0130_3758);
    /// The GUI's fade (`GetGlobal01303730_00848f10`).
    pub const FADE: Global<i32> = Global::new(0x0130_3730);
    /// Where `Gu_SetPrim`, `Gu_SetRgbaq` and friends append.
    pub const PACKET_DATA: Global<u32> = Global::new(0x0130_36c8);
    /// The drawing origin (two `i32`).
    pub const PACKET_ORIGIN: Global<[i32; 2]> = Global::new(0x0130_36d4);

    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    pub const SCE_DMA_SEND: u32 = 0x0062_f710;
    pub const SCE_GS_SYNC_PATH: u32 = 0x0062_f880;
    pub const SG_CONTROL_MOUSE_CAMERA: u32 = 0x0083_f6f0;
    pub const GU_DRAW_SPRITE: u32 = 0x0084_6840;
    pub const GU_SET_TEST: u32 = 0x0084_85d0;
    pub const GU_SET_H_SCALE: u32 = 0x0084_8b30;
    pub const GU_GET_FADE: u32 = 0x0084_8f10;
    pub const GUI_WINDOW_SET_GLOBAL_TRANS: u32 = 0x0084_8f20;
    pub const GU_MOUSE_IN_RECT: u32 = 0x0084_9440;
    pub const GU_WINDOW_CONTROL_SET_RECT: u32 = 0x0084_96d0;
    pub const GU_WINDOW_START_DRAG: u32 = 0x0084_a580;
    pub const GU_LIST_COPY_NEXT: u32 = 0x0084_a600;
    pub const GU_WINDOW_TO_FRONT: u32 = 0x0084_b770;
    pub const SG_SET_DEBUG_TEX_0: u32 = 0x0085_1160;
    pub const SG_MOUSE_GET_MOVE: u32 = 0x0085_1910;

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn i16_at(mem: &impl Mem, p: u32, o: u32) -> i32 {
        i32::from(mem.read::<i16>(at(p, o)))
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `Gu_WindowControlSetRect(window, id, x, y, w, h)`.
    fn set_rect(h: &mut impl Host, window: u32, id: u32, x: i32, y: i32, w: i32, hgt: i32) {
        let args = [window, id, x as u32, y as u32, w as u32, hgt as u32];
        cdecl(h, GU_WINDOW_CONTROL_SET_RECT, &args);
    }

    /// An `XYZ2` quadword at `p`: the screen position, then 0, 4, 0.
    fn xyz(mem: &mut impl Mem, p: u32, (x, y): (u16, u16)) {
        mem.write(p, [x, y]);
        mem.write(at(p, 4), [0u32, 4, 0]);
    }

    /// Writes a quadword (four words) at `p`.
    fn quad(mem: &mut impl Mem, p: u32, q: [u32; 4]) {
        mem.write(p, q);
    }

    /// `Gu_DrawEnd(channel)` (`0x008479d0`): closes the packet with an
    /// end-of-packet `A+D` quadword (`0x8001`, `0x10000000`, `0xf`, 0),
    /// writes the DMA tag `(count - 1) | 0x70000000` at its start, and sends
    /// it (`FlushCache(0)`, `sceDmaSend(channel, packet)`,
    /// `sceGsSyncPath(0, 0)`). Returns `sceGsSyncPath`'s `eax`.
    pub fn draw_end(h: &mut impl Host, channel: u32) -> u32 {
        let n = PACKET_COUNT.get(h);
        let p = PACKET.get(h);
        quad(h, at(p, n.wrapping_mul(16)), [0x8001, 0x1000_0000, 0xf, 0]);
        PACKET_COUNT.set(h, n.wrapping_add(2));
        h.write(p, n.wrapping_add(1) | 0x7000_0000);
        cdecl(h, FLUSH_CACHE, &[0]);
        let p = PACKET.get(h);
        cdecl(h, SCE_DMA_SEND, &[channel, p]);
        cdecl(h, SCE_GS_SYNC_PATH, &[0, 0])
    }

    /// `Gu_DrawMouseCursor()` (`0x00847a40`): a GIF tag quadword, `Gu_SetTest
    /// (1, 7, 0, ...)`, the debug texture (`Sg_SetDebugTEX_0` on the next
    /// quadword of the packet as it was on entry), an `RGBAQ`-like quadword
    /// in the data stream, then the 32x16 arrow sprite at the mouse
    /// (`0x00846840(x, y, 0x20, 0x10, 0, 0x30, 0x10, 0x10, 0)`). Returns its
    /// `eax`.
    pub fn draw_mouse_cursor(h: &mut impl Host) -> u32 {
        let n = PACKET_COUNT.get(h);
        let p = PACKET.get(h);
        quad(
            h,
            at(p, n.wrapping_mul(16)),
            [1, 0xf089_c000, 0x3434_3eee, 0x0434_3434],
        );
        PACKET_COUNT.set(h, n.wrapping_add(1));
        cdecl(h, GU_SET_TEST, &[1, 7, 0, 0, 0, 0, 0, 1]);
        let n = PACKET_COUNT.get(h);
        PACKET_COUNT.set(h, n.wrapping_add(1));
        cdecl(h, SG_SET_DEBUG_TEX_0, &[at(p, n.wrapping_mul(16))]);
        let n = PACKET_COUNT.get(h);
        let d = PACKET_DATA.get(h);
        quad(h, at(d, n.wrapping_mul(16)), [0xff00_ff00, 0x8000, 1, 0]);
        PACKET_COUNT.set(h, n.wrapping_add(1));
        let (x, y) = (MOUSE_X.get(h) as u32, MOUSE_Y.get(h) as u32);
        cdecl(
            h,
            GU_DRAW_SPRITE,
            &[x, y, 0x20, 0x10, 0, 0x30, 0x10, 0x10, 0],
        )
    }

    /// `0x00849510(window)`: a client control's mouse: with the middle
    /// button down, the wheel scrolls (window flag `0x80`: its `+0x8c`, 0..=
    /// 0x80; else the GUI's fade through `Gui_WindowSetGlobalTrans`), 24
    /// per step; a left press grabs the window for a drag, a double click
    /// toggles its flag bit 0.
    pub fn client_mouse(h: &mut impl Host, window: u32) {
        let mut b = MOUSE_BUTTONS.get(h);
        if b & 0x1_0000 != 0 {
            if h.read::<u8>(at(window, 0x88)) & 0x80 == 0 {
                let fade = cdecl(h, GU_GET_FADE, &[]) as i32;
                let step = MOUSE_WHEEL.get(h).wrapping_mul(24);
                cdecl(
                    h,
                    GUI_WINDOW_SET_GLOBAL_TRANS,
                    &[fade.wrapping_sub(step) as u32],
                );
            } else {
                let step = MOUSE_WHEEL.get(h).wrapping_mul(24);
                let v = i16_at(h, window, 0x8c).wrapping_sub(step);
                let v = v.clamp(0, 0x80);
                h.write(at(window, 0x8c), v as u16);
            }
            b = MOUSE_BUTTONS.get(h);
        }
        if b & 2 != 0 {
            let gx = MOUSE_X.get(h).wrapping_sub(i16_at(h, window, 0xc));
            let gy = MOUSE_Y.get(h).wrapping_sub(i16_at(h, window, 0xe));
            DRAG_GRAB.set(h, [gx, gy]);
            DRAG_WINDOW.set(h, window);
        }
        if b & 8 != 0 {
            let f: u32 = h.read(at(window, 0x88));
            h.write(at(window, 0x88), f ^ 1);
        }
    }

    /// `Gu_ControlMouseButtonHandle(window, id)` (`0x0084b960`): sends the
    /// window's callback `+0x7c` one message per button event in
    /// [`MOUSE_BUTTONS`] (left 0x8005 down, 0x8007 pressed, 0x8006 released,
    /// 0x8008 double; right 0x8009, 0x800b, 0x800a, 0x800c; middle 0x800d,
    /// 0x800f, 0x800e, 0x8010), each `(window, message, id)`. Returns what
    /// the original leaves in `eax`: the last callback's result, else the
    /// buttons (or the null callback when only the middle double click is
    /// set).
    pub fn control_mouse_button(h: &mut impl Host, window: u32, id: u32) -> u32 {
        const EVENTS: [(u32, u32); 11] = [
            (1, 0x8005),
            (2, 0x8007),
            (4, 0x8006),
            (8, 0x8008),
            (0x100, 0x8009),
            (0x200, 0x800b),
            (0x400, 0x800a),
            (0x800, 0x800c),
            (0x1_0000, 0x800d),
            (0x2_0000, 0x800f),
            (0x4_0000, 0x800e),
        ];
        let mut eax = MOUSE_BUTTONS.get(h);
        for (bit, msg) in EVENTS {
            if eax & bit != 0 {
                let f: u32 = h.read(at(window, 0x7c));
                if f != 0 {
                    h.call(f, Conv::Cdecl, &[window, msg, id]);
                    eax = MOUSE_BUTTONS.get(h);
                }
            }
        }
        if eax & 0x8_0000 != 0 {
            let f: u32 = h.read(at(window, 0x7c));
            eax = f;
            if f != 0 {
                eax = h.call(f, Conv::Cdecl, &[window, 0x8010, id]);
            }
        }
        eax
    }

    /// `0x0084b7f0(window, control)`: the mouse over one control, by kind
    /// (`+0x2c`): 1 a button (highlighted, pressed while any button is
    /// down, then its messages), 3 a static (its messages), 4 the scroll
    /// bar (a left press captures the mouse with the bar's rectangle), 6
    /// the client area ([`client_mouse`]), 7 the resize grip (a left press
    /// grabs the size), 8 the title bar (a left release starts the drag).
    /// Disabled controls (flag bit 1) do nothing. Returns 0.
    pub fn control_mouse(h: &mut impl Host, window: u32, control: u32) -> u32 {
        if h.read::<u8>(at(control, 0x40)) & 2 != 0 {
            return 0;
        }
        let kind = i16_at(h, control, 0x2c);
        match kind {
            1 | 3 => {
                if kind == 1 {
                    h.write(at(control, 0x1a), 0xc0u8);
                    for bit in [1, 0x100, 0x1_0000] {
                        if MOUSE_BUTTONS.get(h) & bit != 0 {
                            h.write(at(control, 0x20), 1u16);
                        }
                    }
                }
                let id = u32::from(h.read::<u16>(at(control, 0x2e)));
                control_mouse_button(h, window, id);
            }
            4 => {
                if MOUSE_BUTTONS.get(h) & 1 != 0 {
                    let x = i16_at(h, window, 0x84).wrapping_sub(0x10);
                    let y = MOUSE_Y
                        .get(h)
                        .wrapping_sub(i16_at(h, window, 0xe))
                        .wrapping_sub(10);
                    let hgt = i16_at(h, window, 0x86).wrapping_sub(0x12);
                    let args = [window, 0x8003, x as u32, y as u32, 0x10, hgt as u32];
                    cdecl(h, GU_WINDOW_CONTROL_SET_RECT, &args);
                    let id = u32::from(h.read::<u16>(at(control, 0x2e)));
                    CAPTURE.set(h, window);
                    CAPTURE_ID.set(h, id);
                }
            }
            6 => {
                h.write(at(control, 0x1a), 0xc0u8);
                client_mouse(h, window);
            }
            7 => {
                h.write(at(control, 0x1a), 0xc0u8);
                if MOUSE_BUTTONS.get(h) & 2 != 0 {
                    let gx = MOUSE_X.get(h).wrapping_sub(i16_at(h, window, 0x10));
                    let gy = MOUSE_Y.get(h).wrapping_sub(i16_at(h, window, 0x12));
                    RESIZE_GRAB.set(h, [gx, gy]);
                    RESIZE_WINDOW.set(h, window);
                }
            }
            8 => {
                h.write(at(control, 0x1a), 0xc0u8);
                if MOUSE_BUTTONS.get(h) & 4 != 0 {
                    cdecl(h, GU_WINDOW_START_DRAG, &[window]);
                }
            }
            _ => {}
        }
        0
    }

    /// The controls part of a window's hit test, shared by
    /// [`window_mouse`] and [`windows_mouse`]: the capture's control first
    /// (`Some(1)`), then every control reset (not pressed, idle), then each
    /// visible control under the mouse gets [`control_mouse`] (in a modal
    /// window, flag bit 0, only the client area). `None` when a modal window
    /// had no control under the mouse (the caller returns or moves on);
    /// else the wheel message `0x8017` when the window is under the mouse
    /// (`inside`) and no client area was hit, and the no-control message
    /// ([`control_mouse_button`] with id -1) when no control was hit.
    fn controls(h: &mut impl Host, window: u32, inside: bool) -> Option<u32> {
        let capture = CAPTURE.get(h);
        if capture != 0 {
            let count: i32 = h.read(at(window, 0x90));
            let list: u32 = h.read(at(window, 0x94));
            let id = CAPTURE_ID.get(h);
            for i in 0..count.max(0) as u32 {
                let c = at(list, i * 0x44);
                if u32::from(h.read::<u16>(at(c, 0x2e))) == id {
                    control_mouse(h, window, c);
                    return Some(1);
                }
            }
        }
        let mut i = 0i32;
        while i < h.read::<i32>(at(window, 0x90)) {
            let c = at(
                h.read::<u32>(at(window, 0x94)),
                (i as u32).wrapping_mul(0x44),
            );
            h.write(at(c, 0x20), 0u16);
            h.write(at(c, 0x1a), 0x80u8);
            i += 1;
        }
        let mut hit = false;
        let mut hit_client = false;
        let mut i = 0i32;
        while i < h.read::<i32>(at(window, 0x90)) {
            let c = at(
                h.read::<u32>(at(window, 0x94)),
                (i as u32).wrapping_mul(0x44),
            );
            i += 1;
            if h.read::<u8>(at(c, 0x40)) & 1 != 0 {
                continue;
            }
            let client = h.read::<i16>(at(c, 0x2c)) == 6;
            if h.read::<u8>(at(window, 0x88)) & 1 != 0 && !client {
                continue;
            }
            let x = i16_at(h, c, 0).wrapping_add(i16_at(h, window, 0xc));
            let y = i16_at(h, c, 2).wrapping_add(i16_at(h, window, 0xe));
            let mx = MOUSE_X.get(h);
            if mx <= x || mx >= i16_at(h, c, 4).wrapping_add(x) {
                continue;
            }
            let my = MOUSE_Y.get(h);
            if my <= y || my >= i16_at(h, c, 6).wrapping_add(y) {
                continue;
            }
            if client {
                hit_client = true;
            }
            control_mouse(h, window, c);
            hit = true;
        }
        if !hit && h.read::<u8>(at(window, 0x88)) & 1 != 0 {
            return None;
        }
        if inside && !hit_client {
            let wheel = MOUSE_WHEEL.get(h);
            let f: u32 = h.read(at(window, 0x7c));
            if wheel != 0 && f != 0 {
                h.call(f, Conv::Cdecl, &[window, 0x8017, wheel as u32]);
            }
        }
        if !hit {
            control_mouse_button(h, window, 0xffff_ffff);
        }
        Some(0)
    }

    /// `0x0084afd0(window)`: the mouse over one window (the capture's or
    /// the popup's): nothing (0) while it is closing, hidden or moving
    /// (flags 2, 8, 0x10); `Gu_SetHScale(+0x8e)`; a left press inside
    /// brings it to the front; 1 while the mouse is captured by the
    /// desktop (-1); then its controls. Returns 1 when a captured control
    /// was found, 0 for a modal window with no hit, else whether the mouse
    /// is inside.
    pub fn window_mouse(h: &mut impl Host, window: u32) -> u32 {
        if h.read::<u32>(at(window, 0x88)) & 0x1a != 0 {
            return 0;
        }
        cdecl(h, GU_SET_H_SCALE, &[i16_at(h, window, 0x8e) as u32]);
        let mut inside = false;
        if cdecl(h, GU_MOUSE_IN_RECT, &[at(window, 0xc), 0, 0]) != 0 {
            inside = true;
            if MOUSE_BUTTONS.get(h) & 2 != 0 {
                cdecl(h, GU_WINDOW_TO_FRONT, &[window]);
            }
        }
        if CAPTURE.get(h) == u32::MAX {
            return 1;
        }
        match controls(h, window, inside) {
            None => 0,
            Some(1) => 1,
            Some(_) => u32::from(inside),
        }
    }

    /// `0x0084b2e0()`: the hit test down the window list (through `+0x04`):
    /// each window not closing, hidden or moving gets `Gu_SetHScale`, the
    /// front on a left press inside (the mouse strictly inside `+0x0c..`),
    /// and its controls as [`window_mouse`]. Returns 1 at the first window
    /// under the mouse, a captured control or the desktop capture (-1);
    /// 0 when no window took the mouse.
    pub fn windows_mouse(h: &mut impl Host) -> u32 {
        let mut w = WINDOW_LIST.get(h);
        while w != 0 {
            if h.read::<u8>(at(w, 0x88)) & 0x1a == 0 {
                cdecl(h, GU_SET_H_SCALE, &[i16_at(h, w, 0x8e) as u32]);
                let x = i16_at(h, w, 0xc);
                let y = i16_at(h, w, 0xe);
                let mx = MOUSE_X.get(h);
                let mut inside = false;
                if mx > x && mx < i16_at(h, w, 0x10).wrapping_add(x) {
                    let my = MOUSE_Y.get(h);
                    if my > y && my < i16_at(h, w, 0x12).wrapping_add(y) {
                        inside = true;
                        if MOUSE_BUTTONS.get(h) & 2 != 0 {
                            cdecl(h, GU_WINDOW_TO_FRONT, &[w]);
                        }
                    }
                }
                if CAPTURE.get(h) == u32::MAX {
                    return 1;
                }
                match controls(h, w, inside) {
                    Some(1) => return 1,
                    Some(_) if inside => return 1,
                    _ => {}
                }
            }
            w = h.read(at(w, 4));
        }
        0
    }

    /// `Gu_VSync()` (`0x0084a260`): the GUI's mouse frame. While hidden it
    /// waits for the mouse to move (`Sg_MouseGetMove`), then shows again;
    /// the dragged window follows the mouse (all three copies of its
    /// position); the resized window's size follows it, its client size and
    /// its four frame controls (title 0x8001, close 0x8000, grip 0x8002,
    /// scroll bar 0x8003) are laid out again; then the hit test: the
    /// capture's window, else the popup, else the window list (front to
    /// back after `Gu_ListCopyNext`), and a press nobody took captures the
    /// mouse for the camera (`Sg_ControlMouseCamera`). With no button down
    /// every drag and capture ends.
    pub fn vsync(h: &mut impl Host) {
        if HIDDEN.get(h) != 0 {
            if cdecl(h, SG_MOUSE_GET_MOVE, &[]) == 0 {
                return;
            }
            HIDDEN.set(h, 0);
        }
        let drag = DRAG_WINDOW.get(h);
        if drag != 0 {
            let [gx, gy] = DRAG_GRAB.get(h);
            let y = MOUSE_Y.get(h).wrapping_sub(gy) as u16;
            let x = MOUSE_X.get(h).wrapping_sub(gx) as u16;
            for o in [0xc, 0x14, 0x1c] {
                h.write(at(drag, o), x);
            }
            for o in [0xe, 0x16, 0x1e] {
                h.write(at(drag, o), y);
            }
        }
        let w = RESIZE_WINDOW.get(h);
        if w != 0 {
            let [gx, gy] = RESIZE_GRAB.get(h);
            let cw = MOUSE_X.get(h).wrapping_sub(gx) as u16;
            let ch = MOUSE_Y.get(h).wrapping_sub(gy) as u16;
            for o in [0x10, 0x18, 0x20] {
                h.write(at(w, o), cw);
            }
            let left = h.read::<u16>(at(w, 0x80));
            h.write(at(w, 0x84), cw.wrapping_sub(left.wrapping_add(left)));
            for o in [0x12, 0x1a, 0x22] {
                h.write(at(w, o), ch);
            }
            let top = h.read::<u16>(at(w, 0x82));
            let width = i16_at(h, w, 0x10);
            h.write(at(w, 0x86), ch.wrapping_sub(top).wrapping_sub(2));
            let left = i32::from(left as i16);
            let top = i32::from(top as i16);
            set_rect(h, w, 0x8001, -left, -top, width, 10);
            let top = i16_at(h, w, 0x82);
            let x = i16_at(h, w, 0x10)
                .wrapping_sub(i16_at(h, w, 0x80))
                .wrapping_sub(0x10);
            set_rect(h, w, 0x8000, x, -top, 0xc, 8);
            let y = i16_at(h, w, 0x86).wrapping_sub(8);
            let x = i16_at(h, w, 0x84).wrapping_sub(0x10);
            set_rect(h, w, 0x8002, x, y, 0x10, 8);
            let hgt = i16_at(h, w, 0x86).wrapping_sub(0x12);
            let x = i16_at(h, w, 0x84).wrapping_sub(0x10);
            set_rect(h, w, 0x8003, x, 10, 0x10, hgt);
        }
        let capture = CAPTURE.get(h);
        let camera = if capture == 0 {
            let popup = POPUP_WINDOW.get(h);
            if popup != 0 {
                window_mouse(h, popup);
                false
            } else {
                let list = WINDOW_LIST.get(h);
                if list != 0 {
                    let first: u32 = h.read(list);
                    h.write(at(list, 4), first);
                    cdecl(h, GU_LIST_COPY_NEXT, &[first]);
                }
                if windows_mouse(h) == 0 && MOUSE_BUTTONS.get(h) & 0x1_0101 != 0 {
                    CAPTURE.set(h, u32::MAX);
                    true
                } else {
                    false
                }
            }
        } else if capture == u32::MAX {
            true
        } else {
            window_mouse(h, capture);
            false
        };
        if camera {
            cdecl(h, SG_CONTROL_MOUSE_CAMERA, &[]);
        }
        if MOUSE_BUTTONS.get(h) & 0x3_0303 == 0 {
            DRAG_WINDOW.set(h, 0);
            RESIZE_WINDOW.set(h, 0);
            CAPTURE.set(h, 0);
            CAPTURE_ID.set(h, 0);
        }
    }

    /// One step of `0x00847420`'s easing: `v` moves towards `target` by
    /// `(target - v) / min(|target - v|, n)` (a step of 1 once closer than
    /// `n`).
    fn ease(v: i32, target: i32, n: i32) -> i32 {
        let d = target.wrapping_sub(v);
        if d == 0 {
            return v;
        }
        let a = d.wrapping_abs();
        let div = if a < n { a } else { n };
        if div == 0 {
            return v;
        }
        v.wrapping_add(d.wrapping_div(div))
    }

    /// `0x00847420(cursor, dx, dy)`: the selection cursor's frame: its
    /// rectangle `+0x08..+0x0e` (`i16` x, y, w, h) eases towards `+0x10..
    /// +0x16` in `+0x24` steps (2 when 0), its colour bytes `+0x1c..+0x1e`
    /// a third of the way towards `+0x18..+0x1a`; then a packet: a GIF tag,
    /// `Gu_SetTest(1, 1, 0, ...)`, an alpha quadword (`0x80 - fade / 3`),
    /// and the rectangle's two corners moved by `(dx, dy)` as `XYZ`
    /// quadwords in screen space (centred in 4096, y by the vertical
    /// scale). The tag's count is patched last. Returns the tag's address.
    pub fn cursor_draw(h: &mut impl Host, cursor: u32, dx: u32, dy: u32) -> u32 {
        let s = i16_at(h, cursor, 0x24);
        let n = if s == 0 { 2 } else { s };
        let x = ease(i16_at(h, cursor, 0x8), i16_at(h, cursor, 0x10), n);
        h.write(at(cursor, 0x8), x as u16);
        let y = ease(i16_at(h, cursor, 0xa), i16_at(h, cursor, 0x12), n);
        h.write(at(cursor, 0xa), y as u16);
        let w = ease(i16_at(h, cursor, 0xc), i16_at(h, cursor, 0x14), n);
        h.write(at(cursor, 0xc), w as u16);
        let hgt = ease(i16_at(h, cursor, 0xe), i16_at(h, cursor, 0x16), n);
        for (cur, target) in [(0x1d, 0x19), (0x1e, 0x1a), (0x1c, 0x18)] {
            let c = h.read::<u8>(at(cursor, cur));
            let t = h.read::<u8>(at(cursor, target));
            let step = (i32::from(t) - i32::from(c) + 2) / 3;
            h.write(at(cursor, cur), c.wrapping_add(step as u8));
        }
        let x0 = i32::from(x as i16).wrapping_add(dx as i32);
        let y0 = i32::from(y as i16).wrapping_add(dy as i32);
        let x1 = i32::from(w as i16).wrapping_add(x0);
        let y1 = i32::from(hgt as i16).wrapping_add(y0);
        let start = PACKET_COUNT.get(h);
        let tag = at(PACKET.get(h), start.wrapping_mul(16));
        h.write(at(cursor, 0xe), hgt as u16);
        quad(h, tag, [0x3039, 0x1023_4000, 0xe, 0]);
        PACKET_ORIGIN.set(h, [0, 0]);
        PACKET_COUNT.set(h, start.wrapping_add(1));
        cdecl(h, GU_SET_TEST, &[1, 1, 0, 0, 0, 0, 0, 0]);
        let n = PACKET_COUNT.get(h);
        let d = PACKET_DATA.get(h);
        let alpha = 0x80i32.wrapping_sub(FADE.get(h) / 3);
        quad(h, at(d, n.wrapping_mul(16)), [0xa9, alpha as u32, 0x42, 0]);
        let scale = H_SCALE.get(h);
        let half_w = (0x1000i32.wrapping_sub(SCREEN_W.get(h))) / 2;
        let half_h = (0x1000i32.wrapping_sub(SCREEN_H.get(h))) / 2;
        let screen = |x: i32, y: i32| {
            let sx = (half_w.wrapping_add(x) << 4) as u16;
            let sy = ((scale.wrapping_mul(y) >> 8).wrapping_add(half_h) << 4) as u16;
            (sx, sy)
        };
        xyz(h, at(d, n.wrapping_add(1).wrapping_mul(16)), screen(x0, y0));
        let d = PACKET_DATA.get(h);
        PACKET_COUNT.set(h, n.wrapping_add(3));
        xyz(h, at(d, n.wrapping_add(2).wrapping_mul(16)), screen(x1, y1));
        let used = n.wrapping_add(3).wrapping_sub(start).wrapping_sub(1);
        let t: u32 = h.read(tag);
        h.write(tag, t ^ ((used ^ t) & 0x7fff));
        tag
    }

    /// The text size the print derives its metrics from (1/256 units).
    pub const PRINT_SIZE: Global<i32> = Global::new(0x0130_3728);
    /// The glyph advance (`size * 8 >> 8`, plus one).
    pub const PRINT_ADVANCE: Global<i32> = Global::new(0x0130_36f0);
    /// The glyph width (`size * 8 >> 8`).
    pub const PRINT_GLYPH_W: Global<i32> = Global::new(0x0130_36fc);
    /// `size * 4 >> 8`.
    pub const PRINT_HALF: Global<i32> = Global::new(0x0130_371c);
    /// `size >> 8`.
    pub const PRINT_SCALE: Global<i32> = Global::new(0x0130_3724);

    pub const SEND_DEBUG_FONT: u32 = 0x0085_10b0;
    pub const SG_MOUSE_BUTTON_MAIN: u32 = 0x0085_17c0;
    pub const GU_DRAW_FRAME: u32 = 0x0084_6b10;
    pub const GU_DRAW_START: u32 = 0x0084_7b00;
    pub const GU_DRAW_CLIENT: u32 = 0x0084_7b50;
    pub const GU_FONT_SETUP_WINDOW: u32 = 0x0084_8310;
    pub const GU_PRINTF: u32 = 0x0084_8880;
    pub const GU_SET_SCISSOR: u32 = 0x0084_8b40;
    pub const GU_PRINT_GET_WIDTH: u32 = 0x0084_8d60;
    pub const GU_SET_TEX_FLUSH: u32 = 0x0084_8d10;
    pub const GU_PARSE_HEX: u32 = 0x0084_8f60;
    pub const GU_PARSE_DEC: u32 = 0x0084_8fe0;
    pub const GU_WINDOW_SET_HOKAN_ENABLE: u32 = 0x0084_8f00;
    pub const GU_WINDOW_CLOSE_TREE: u32 = 0x0084_ab80;
    pub const GU_WINDOW_FREE_CLOSED: u32 = 0x0084_abd0;

    /// `Gu_PrintBegin()` (`0x00849170`): the text's packet head: a GIF tag
    /// quadword, `Gu_SetTest(1, 5, 0x7f, 0, 0, 0, 0, 1)`, the debug font
    /// texture (`Sg_SetDebugTEX_0`, with its `TEX0` bits 5-6 cleared), an
    /// `A+D` tag, and the colour as a 64-bit `RGBAQ` built from the three
    /// colour words (each `or`-ed in sign-extended, alpha `0x80`, `Q` bits
    /// `0xff`). Returns the red word (what the original leaves in `eax`).
    pub fn print_begin(h: &mut impl Host) -> u32 {
        let n = PACKET_COUNT.get(h);
        let p = PACKET.get(h);
        quad(h, at(p, n.wrapping_mul(16)), [1, 0x4000_0000, 0xeeee, 0]);
        PACKET_COUNT.set(h, n.wrapping_add(1));
        cdecl(h, GU_SET_TEST, &[1, 5, 0x7f, 0, 0, 0, 0, 1]);
        let n = PACKET_COUNT.get(h);
        let tex = at(p, n.wrapping_mul(16));
        PACKET_COUNT.set(h, n.wrapping_add(1));
        cdecl(h, SG_SET_DEBUG_TEX_0, &[tex]);
        let w: u32 = h.read(at(tex, 4));
        h.write(at(tex, 4), w & 0xffff_ff9f);
        let n = PACKET_COUNT.get(h);
        let p = PACKET.get(h);
        quad(h, at(p, n.wrapping_mul(16)), [1, 0, 0x14, 0]);
        let [r, g, b] = crate::gu::bridge::PRINT_COLOR.get(h);
        let sx = |v: u32| i64::from(v as i32) as u64;
        let mut v = sx(b) | 0x8000_ff00;
        v = (v << 8) | sx(g);
        v = (v << 8) | sx(r);
        let d = PACKET_DATA.get(h);
        quad(
            h,
            at(d, n.wrapping_add(1).wrapping_mul(16)),
            [v as u32, (v >> 32) as u32, 1, 0],
        );
        PACKET_COUNT.set(h, n.wrapping_add(2));
        r
    }

    /// Sets the print metrics from `size` (`Gu_PrintSetSize`'s derived
    /// globals; the size itself too).
    fn print_metrics(mem: &mut impl Mem, size: i32) {
        let glyph = size.wrapping_mul(8) >> 8;
        PRINT_GLYPH_W.set(mem, glyph);
        PRINT_ADVANCE.set(mem, glyph.wrapping_add(1));
        PRINT_SIZE.set(mem, size);
        PRINT_HALF.set(mem, size.wrapping_mul(4) >> 8);
        PRINT_SCALE.set(mem, size >> 8);
    }

    /// `strlen` of the C string at `s`.
    fn strlen(mem: &impl Mem, s: u32) -> u32 {
        let mut n = 0u32;
        while mem.read::<u8>(at(s, n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// `Gu_PrintGetWidth(text)` (`0x00848d60`): the text's width in pixels,
    /// its length times the glyph advance. A `%~` prefix holds codes (`~b`
    /// bold, `~i` italic, `~c<hex>` colour, `~w<dec>` size, `~.` end), each
    /// code's argument parsed into `slot` (the original's own argument
    /// word) by `0x00848f60` / `0x00848fe0`, which return where the text
    /// goes on. The advance used is the one before the codes; afterwards
    /// the metrics are set again from the size as it was and bold and
    /// italic restored. A text that ends inside the codes is 0 wide (and
    /// keeps the codes' bold and italic).
    pub fn print_get_width(h: &mut impl Host, text: u32, slot: u32) -> u32 {
        if h.read::<u8>(text) != b'%' || h.read::<u8>(at(text, 1)) != b'~' {
            let n = strlen(h, text);
            return n.wrapping_mul(PRINT_ADVANCE.get(h) as u32);
        }
        let bold = crate::gu::bridge::PRINT_BOLD.get(h);
        let italic = crate::gu::bridge::PRINT_ITALIC.get(h);
        let size = PRINT_SIZE.get(h);
        let mut p = at(text, 1);
        loop {
            p = at(p, 1);
            match h.read::<u8>(p) {
                0 => return 0,
                b'.' => {
                    p = at(p, 1);
                    break;
                }
                b'b' => crate::gu::bridge::PRINT_BOLD.set(h, 1),
                b'c' => p = cdecl(h, GU_PARSE_HEX, &[at(p, 1), slot]),
                b'i' => crate::gu::bridge::PRINT_ITALIC.set(h, 1),
                b'w' => p = cdecl(h, GU_PARSE_DEC, &[at(p, 1), slot]),
                _ => break,
            }
            if h.read::<u8>(p) != b'~' {
                break;
            }
        }
        let n = strlen(h, p);
        let width = n.wrapping_mul(PRINT_ADVANCE.get(h) as u32);
        print_metrics(h, size);
        crate::gu::bridge::PRINT_BOLD.set(h, bold);
        crate::gu::bridge::PRINT_ITALIC.set(h, italic);
        width
    }

    /// The print cursor for a control's label: x centred in the control
    /// (`text_w` wide; left-aligned with flag 0x10), y centred on its
    /// 8-pixel line, both relative to the window's client origin.
    fn label_cursor(h: &mut impl Host, control: u32, window: u32, dx: u32, dy: u32, text_w: i32) {
        use crate::gu::bridge::{PRINT_BASE_X, PRINT_BASE_Y, PRINT_X, PRINT_Y};
        let y = i16_at(h, control, 0xa)
            .wrapping_add((i16_at(h, control, 0xe) - 8) / 2)
            .wrapping_add(dy as i32);
        if h.read::<u8>(at(control, 0x40)) & 0x10 != 0 {
            let x = i16_at(h, control, 0x8).wrapping_add(PRINT_BASE_X.get(h));
            PRINT_X.set(h, x);
        } else {
            let x = (i16_at(h, control, 0xc).wrapping_sub(text_w) / 2)
                .wrapping_sub(i16_at(h, window, 0x80))
                .wrapping_sub(i16_at(h, window, 0x14))
                .wrapping_add(PRINT_BASE_X.get(h))
                .wrapping_add(i16_at(h, control, 0x8))
                .wrapping_add(dx as i32);
            PRINT_X.set(h, x);
        }
        let base = PRINT_BASE_Y
            .get(h)
            .wrapping_sub(i16_at(h, window, 0x82))
            .wrapping_sub(i16_at(h, window, 0x16));
        PRINT_Y.set(h, y.wrapping_add(1).wrapping_add(base));
    }

    /// `Gu_DrawControl(control, dx, dy, window)` (`0x00847720`): draws one
    /// control by kind (`+0x2c`): 1 a button (its frame, `0x00846b10`, a
    /// toggle shows its value as pressed, then its label `+0x28` centred by
    /// `Gu_PrintGetWidth`), 2 a static label (9 pixels a character), 4, 7
    /// and 8 the frame only, 6 the client area (`0x00847b50`, then the
    /// window's title `+0x5c` in bold). Disabled labels print grey
    /// (colour 0x20, back to 0 after).
    pub fn draw_control(h: &mut impl Host, control: u32, dx: u32, dy: u32, window: u32) {
        use crate::gu::bridge::{
            PRINT_BASE_X, PRINT_BASE_Y, PRINT_BOLD, PRINT_COLOR, PRINT_ITALIC, PRINT_X, PRINT_Y,
        };
        let label: u32 = h.read(at(control, 0x28));
        match i16_at(h, control, 0x2c) {
            kind @ (1 | 2) => {
                let text_w = if kind == 1 {
                    let flags: u32 = h.read(at(control, 0x40));
                    if flags & 4 != 0 && h.read::<u16>(at(control, 0x20)) == 0 {
                        h.write(at(control, 0x20), ((flags << 0x1c) as i32 >> 0x1f) as u16);
                    }
                    cdecl(h, GU_DRAW_FRAME, &[control, dx, dy]);
                    cdecl(h, GU_PRINT_GET_WIDTH, &[label]) as i32
                } else {
                    (strlen(h, label) as i32).wrapping_mul(9)
                };
                label_cursor(h, control, window, dx, dy, text_w);
                let grey = h.read::<u8>(at(control, 0x40)) & 2 != 0;
                if grey {
                    PRINT_COLOR.set(h, [0x20; 3]);
                }
                PRINT_BOLD.set(h, 0);
                PRINT_ITALIC.set(h, 0);
                cdecl(h, GU_PRINTF, &[label]);
                if h.read::<u8>(at(control, 0x40)) & 2 != 0 {
                    PRINT_COLOR.set(h, [0; 3]);
                }
            }
            4 | 7 | 8 => {
                cdecl(h, GU_DRAW_FRAME, &[control, dx, dy]);
            }
            6 => {
                cdecl(h, GU_DRAW_CLIENT, &[control, dx, dy]);
                let title: u32 = h.read(at(window, 0x5c));
                let x = PRINT_BASE_X
                    .get(h)
                    .wrapping_sub(i16_at(h, window, 0x80))
                    .wrapping_add(6);
                PRINT_X.set(h, x);
                let y = PRINT_BASE_Y
                    .get(h)
                    .wrapping_sub(i16_at(h, window, 0x82))
                    .wrapping_add(1);
                PRINT_Y.set(h, y);
                PRINT_BOLD.set(h, 1);
                PRINT_ITALIC.set(h, 0);
                cdecl(h, GU_PRINTF, &[title]);
            }
            _ => {}
        }
    }

    /// `Gu_SetScissor(x0, x1, y0, y1)`.
    fn scissor(h: &mut impl Host, x0: i32, x1: i32, y0: i32, y1: i32) {
        cdecl(
            h,
            GU_SET_SCISSOR,
            &[x0 as u32, x1 as u32, y0 as u32, y1 as u32],
        );
    }

    /// The window's client area: font set up (`Gu_FontSetupWindow`), its
    /// paint message `0x8011`, font set up again, then its visible
    /// controls drawn (only the client area when `client_only`).
    fn draw_controls(h: &mut impl Host, window: u32, client_only: bool) {
        cdecl(h, GU_FONT_SETUP_WINDOW, &[window]);
        let f: u32 = h.read(at(window, 0x7c));
        if f != 0 {
            h.call(f, Conv::Cdecl, &[window, 0x8011, 0]);
        }
        cdecl(h, GU_FONT_SETUP_WINDOW, &[window]);
        let mut i = 0i32;
        while i < h.read::<i32>(at(window, 0x90)) {
            let c = at(
                h.read::<u32>(at(window, 0x94)),
                (i as u32).wrapping_mul(0x44),
            );
            i += 1;
            if h.read::<u8>(at(c, 0x40)) & 1 != 0 {
                continue;
            }
            if client_only && h.read::<i16>(at(c, 0x2c)) != 6 {
                continue;
            }
            let (x, y) = (i16_at(h, window, 0x14), i16_at(h, window, 0x16));
            draw_control(h, c, x as u32, y as u32, window);
        }
    }

    /// `Gu_DrawWindowTree(window)` (`0x0084ac80`): draws the window list
    /// from `window` on, back to front (the rest of the list first). A
    /// closing window (flag 2) is skipped; one fading out (flag 0x10) whose
    /// target width `+0x20` is 0 and width under 8 is flagged closed (and
    /// its tree closed with flag 0x20); a growing one (flag 8) shrinks its
    /// width and height to the targets and stops when there. Then, under
    /// its vertical scale: minimised (flag 1) the title strip only (9
    /// pixels, client controls only); else the frame eased and drawn
    /// (`Gu_DrawCursor`), the frame's outline without interpolation, the
    /// client scissor and all controls, with the window's own fade (flag
    /// 0x80) around it. The scissor ends as the whole screen.
    pub fn draw_window_tree(h: &mut impl Host, window: u32) {
        if window == 0 {
            return;
        }
        let next: u32 = h.read(window);
        draw_window_tree(h, next);
        let mut flags: u32 = h.read(at(window, 0x88));
        if flags & 2 != 0 {
            return;
        }
        let frame = at(window, 0xc);
        if flags & 0x10 != 0
            && h.read::<u16>(at(window, 0x20)) == 0
            && h.read::<i16>(at(window, 0x18)) < 8
        {
            flags = (flags & !0x10) | 2;
            h.write(at(window, 0x88), flags);
            if flags & 0x20 != 0 {
                cdecl(h, GU_WINDOW_CLOSE_TREE, &[window]);
            }
        }
        if h.read::<u8>(at(window, 0x88)) & 8 != 0 {
            let tw = h.read::<i16>(at(window, 0x20));
            if tw < h.read::<i16>(at(window, 0x18)) {
                h.write(at(window, 0x18), tw);
            }
            let th = h.read::<i16>(at(window, 0x22));
            if th < h.read::<i16>(at(window, 0x1a)) {
                h.write(at(window, 0x1a), th);
            }
            if tw == h.read::<i16>(at(window, 0x18)) && th == h.read::<i16>(at(window, 0x1a)) {
                let f: u32 = h.read(at(window, 0x88));
                h.write(at(window, 0x88), f & !8);
            }
        }
        cdecl(h, GU_SET_H_SCALE, &[i16_at(h, window, 0x8e) as u32]);
        let flags: u32 = h.read(at(window, 0x88));
        if flags & 1 != 0 {
            let (x0, y0) = (i16_at(h, window, 0x14), i16_at(h, window, 0x16));
            let w = i16_at(h, window, 0x18);
            scissor(h, x0, w.wrapping_add(x0), y0, y0.wrapping_add(9));
            cdecl(h, GU_DRAW_FRAME, &[frame, 0, 0]);
            draw_controls(h, window, true);
        } else {
            let mut fade = 0;
            if flags & 0x80 != 0 {
                fade = cdecl(h, GU_GET_FADE, &[]);
                cdecl(
                    h,
                    GUI_WINDOW_SET_GLOBAL_TRANS,
                    &[i16_at(h, window, 0x8c) as u32],
                );
            }
            cursor_draw(h, frame, 6, 3);
            cdecl(h, GU_WINDOW_SET_HOKAN_ENABLE, &[0]);
            cdecl(h, GU_DRAW_FRAME, &[frame, 0, 0]);
            cdecl(h, GU_WINDOW_SET_HOKAN_ENABLE, &[1]);
            let (x0, y0) = (i16_at(h, window, 0x14), i16_at(h, window, 0x16));
            let (w, ht) = (i16_at(h, window, 0x18), i16_at(h, window, 0x1a));
            scissor(h, x0, w.wrapping_add(x0), y0, ht.wrapping_add(y0));
            draw_controls(h, window, false);
            if h.read::<u8>(at(window, 0x88)) & 0x80 != 0 {
                cdecl(h, GUI_WINDOW_SET_GLOBAL_TRANS, &[fade]);
            }
        }
        scissor(h, 0, 0x200, 0, 0x1a0);
    }

    /// `Gu_Main(channel, buffer)` (`0x00849850`): the GUI's frame: the
    /// mouse buttons (`Sg_MouseButtonMain`'s result), [`vsync`]; unless
    /// hidden, the debug font sent, the packet started in `buffer`, the
    /// texture flushed, the whole-screen scissor, the windows drawn, the
    /// mouse cursor (when enabled, at scale 0x200), the packet sent on
    /// `channel` ([`draw_end`]), closed windows freed and the wheel
    /// cleared.
    pub fn gu_main(h: &mut impl Host, channel: u32, buffer: u32) {
        let buttons = cdecl(h, SG_MOUSE_BUTTON_MAIN, &[]);
        MOUSE_BUTTONS.set(h, buttons);
        vsync(h);
        if HIDDEN.get(h) != 0 {
            return;
        }
        cdecl(h, SEND_DEBUG_FONT, &[]);
        cdecl(h, GU_DRAW_START, &[buffer]);
        cdecl(h, GU_SET_TEX_FLUSH, &[]);
        scissor(h, 0, 0x200, 0, 0x1a0);
        let list = WINDOW_LIST.get(h);
        draw_window_tree(h, list);
        if crate::gu::bridge::MOUSE_ENABLE.get(h) != 0 {
            cdecl(h, GU_SET_H_SCALE, &[0x200]);
            draw_mouse_cursor(h);
        }
        draw_end(h, channel);
        cdecl(h, GU_WINDOW_FREE_CLOSED, &[]);
        MOUSE_WHEEL.set(h, 0);
    }
}
