//! M5 stage 1, round 6 (list `left6`, from `render-pcui4.left`), batch 2:
//! the debug performance meter window's procedure (`FUN_00855570`, with
//! its jump table's case 0x8011, `caseD_8011` `0x008555e9`).
//!
//! Callees stay calls to the originals (cdecl). The frame's rectangle
//! (`ebp - 8`) comes in as the original's `ebp`; the procedure keeps the
//! meter's width in its `msg` argument slot (`slots`).

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the originals' coordinates
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    // ---- globals ----

    /// The meter's hover text (`char[0x40]`), the window it belongs to,
    /// the segment hovered, the scale, the width, the cursor and the
    /// reference, the close flag, the last frame time.
    pub const HOVER_TEXT: u32 = 0x0132_5958;
    pub const METER_WINDOW: u32 = 0x0132_5948;
    pub const HOVER_SEGMENT: u32 = 0x0132_5998;
    pub const SCALE: u32 = 0x0132_59e8;
    pub const WIDTH: u32 = 0x0132_59f0;
    pub const CURSOR: u32 = 0x0132_59f4;
    pub const REFERENCE: u32 = 0x0132_59f8;
    pub const OPEN: u32 = 0x0132_59fc;
    pub const LAST_TIME: u32 = 0x0132_5a00;
    /// The tick colour's base (`<< (k + 4)`).
    pub const TICK_BASE: u32 = 0x023c_bbf0;
    /// The seven segments (8 bytes: name, timer index) and the texts:
    /// the close button, the header, the time, the frame, the hover line.
    pub const SEGMENTS: u32 = 0x00c5_0018;
    pub const CLOSE_TEXT: u32 = 0x00b8_fbeb;
    pub const HEADER_TEXT: u32 = 0x00b5_b9b8;
    pub const TIME_FORMAT: u32 = 0x00b5_4c6c;
    pub const FRAME_FORMAT: u32 = 0x00b5_b9bc;
    pub const HOVER_FORMAT: u32 = 0x00b5_b9c4;
    /// `MSVCR110` `sprintf` slot.
    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

    // ---- callees ----

    pub const ADD_BUTTON: u32 = 0x0084_a470;
    pub const START_DRAG: u32 = 0x0084_a580;
    pub const SG_SET_RESOURCE: u32 = 0x0083_fe10;
    pub const MOUSE_IN: u32 = 0x0084_9490;
    pub const CLIENT_MOUSE: u32 = 0x0084_9510;
    pub const SET_FLAG1: u32 = 0x0084_b680;
    pub const MOUSE_GET_Y: u32 = 0x0084_9af0;
    pub const MOUSE_GET_X: u32 = 0x0084_9ae0;
    pub const SET_POS: u32 = 0x0084_b220;
    pub const PRINT_SET_SIZE_ONLY: u32 = 0x0084_87d0;
    pub const PRINT_SET_ITALIC: u32 = 0x0084_8760;
    pub const PRINT_SET_BOLD: u32 = 0x0084_8720;
    pub const PRINT_SET_XY: u32 = 0x0084_8840;
    pub const GU_PRINTF: u32 = 0x0084_8880;
    pub const TIMER: u32 = 0x0082_0580;
    pub const PRINT_SET_COLOR: u32 = 0x0084_8740;
    pub const GET_SIZE_W: u32 = 0x0084_af90;
    pub const CONTROL_SET_RECT: u32 = 0x0084_96d0;
    pub const GET_CLIENT_W: u32 = 0x0084_aee0;
    pub const PACKET_BEGIN_TAG: u32 = 0x0084_8530;
    pub const SET_TEST: u32 = 0x0084_85d0;
    pub const MOUSE_WINDOW_Y: u32 = 0x0084_9aa0;
    pub const MOUSE_WINDOW_X: u32 = 0x0084_9ac0;
    pub const GET_SIZE_H: u32 = 0x0084_af10;
    pub const SET_PRIM: u32 = 0x0084_8480;
    pub const DRAW_BAR_SEGMENT: u32 = 0x0085_5a30;
    pub const SET_FLAG2: u32 = 0x0084_b600;
    pub const TO_FRONT: u32 = 0x0084_b770;
    pub const SET_RGBAQ: u32 = 0x0084_84c0;
    pub const SET_XYZF2: u32 = 0x0084_8680;
    pub const DRAW_RECT_STRIP: u32 = 0x0085_4f60;
    pub const PACKET_END_TAG: u32 = 0x0084_8460;
    pub const FRAME_TIME: u32 = 0x0082_0590;

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn g32(h: &impl Mem, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    fn set32(h: &mut impl Mem, sym: u32, v: u32) {
        let at = h.rebase(sym);
        h.write(at, v);
    }

    fn i(v: u32) -> i32 {
        v.cast_signed()
    }

    /// `a * b / c`, signed (`imul`, `idiv`); 0 where the original would
    /// fault (`c` 0 or the quotient out of range).
    fn mul_div(a: u32, b: u32, c: u32) -> u32 {
        i(a.wrapping_mul(b))
            .checked_div(i(c))
            .unwrap_or(0)
            .cast_unsigned()
    }

    /// `FUN_00855570(window, msg, param; slots)` -> 0: the performance
    /// meter window's procedure. 0x8001 adds the close button; 0x8003
    /// clears the open flag; 0x8006 button 1 starts the window drag and
    /// sets resource 2; 0x8011 draws it ([`meter_draw`]).
    pub fn meter_proc(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        window: u32,
        msg: u32,
        param: u32,
    ) -> u32 {
        match msg {
            0x8001 => {
                let text = h.rebase(CLOSE_TEXT);
                cdecl(h, ADD_BUTTON, &[window, 1, text, 0, 0, 8, 8]);
            }
            0x8006 => {
                if param == 1 {
                    cdecl(h, START_DRAG, &[window]);
                    cdecl(h, SG_SET_RESOURCE, &[2, 1]);
                }
            }
            0x8003 => set32(h, OPEN, 0),
            0x8011 => meter_draw(h, ebp, slots, window),
            _ => {}
        }
        0
    }

    /// The hover test of the seven segments: each drawn
    /// (`Native_DrawBarSegment_00855a30` into the frame's rectangle), the
    /// one under the mouse (`x` against the cursor less its width) named
    /// with its time into the hover text.
    fn meter_segments(h: &mut impl Host, rect: u32, mouse_x: u32) {
        let mut seg = h.rebase(SEGMENTS);
        for _ in 0..7 {
            let w = cdecl(h, DRAW_BAR_SEGMENT, &[rect, seg]);
            let end = g32(h, CURSOR);
            let start = end.wrapping_sub(w);
            if i(mouse_x) > i(start) && i(mouse_x) <= i(end) {
                let k = h.read::<u8>(seg.wrapping_add(4));
                let t = if k == 0 {
                    let t = cdecl(h, TIMER, &[0]);
                    t.wrapping_sub(g32(h, REFERENCE))
                } else {
                    cdecl(h, TIMER, &[u32::from(k)])
                };
                let name: u32 = h.read(seg);
                let text = h.rebase(HOVER_TEXT);
                let fmt = h.rebase(HOVER_FORMAT);
                let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
                h.call(f, Conv::Cdecl, &[text, fmt, name, t]);
                set32(h, HOVER_SEGMENT, seg);
            }
            seg = seg.wrapping_add(8);
        }
    }

    /// The meter's 0x8011: follows the mouse when inside, prints the
    /// header, the frame time (red past 0x200) and the last frame's; sizes
    /// the close button; the scale (0x200 up by 0x100 past the time, to
    /// 0x800), the bar's width (client width less 0x5c, kept in the `msg`
    /// slot and `0x013259f0`); the segments and the hover text (the window
    /// brought to the front with one); the bar's frame, the reference
    /// line and the ticks; then the last frame time stored.
    #[allow(clippy::too_many_lines)]
    fn meter_draw(h: &mut impl Host, ebp: u32, slots: u32, window: u32) {
        let rect = ebp.wrapping_sub(8);
        if cdecl(h, MOUSE_IN, &[window]) != 0 {
            cdecl(h, CLIENT_MOUSE, &[window]);
            cdecl(h, SET_FLAG1, &[window, 0]);
            let y = cdecl(h, MOUSE_GET_Y, &[]).wrapping_sub(0xa);
            let x = cdecl(h, MOUSE_GET_X, &[]);
            let w = g32(h, METER_WINDOW);
            cdecl(h, SET_POS, &[w, x, y]);
        }
        cdecl(h, PRINT_SET_SIZE_ONLY, &[0xa0]);
        cdecl(h, PRINT_SET_ITALIC, &[1]);
        cdecl(h, PRINT_SET_BOLD, &[1]);
        cdecl(h, PRINT_SET_XY, &[1, 0]);
        let header = h.rebase(HEADER_TEXT);
        cdecl(h, GU_PRINTF, &[header]);
        let t = cdecl(h, TIMER, &[0]);
        if i(t) > 0x200 {
            cdecl(h, PRINT_SET_COLOR, &[0x40, 0, 0, 0xff]);
        }
        let fmt = h.rebase(TIME_FORMAT);
        cdecl(h, GU_PRINTF, &[fmt, t]);
        cdecl(h, PRINT_SET_COLOR, &[0, 0, 0, 0xff]);
        cdecl(h, PRINT_SET_XY, &[1, 6]);
        let fmt = h.rebase(FRAME_FORMAT);
        let last = g32(h, LAST_TIME);
        cdecl(h, GU_PRINTF, &[fmt, last]);
        let sw = cdecl(h, GET_SIZE_W, &[window]);
        cdecl(
            h,
            CONTROL_SET_RECT,
            &[window, 1, sw.wrapping_sub(0x13), 0, 0xc, 7],
        );
        cdecl(h, GET_CLIENT_W, &[window]);
        h.write(rect, 0x4c_u32);
        let cw = cdecl(h, GET_CLIENT_W, &[window]).wrapping_sub(0x5c);
        #[allow(clippy::cast_possible_truncation)] // the original's `si`
        let w16 = cw as u16;
        h.write(rect.wrapping_add(4), w16);
        h.write(rect.wrapping_add(6), 5_u16);
        let t = cdecl(h, TIMER, &[0]);
        let mut scale = 0x200_u32;
        set32(h, SCALE, scale);
        if i(t) > 0x200 {
            loop {
                scale = scale.wrapping_add(0x100);
                if scale == 0x800 || i(t) <= i(scale) {
                    break;
                }
            }
            set32(h, SCALE, scale);
        }
        let width = i32::from(w16.cast_signed()).cast_unsigned();
        h.write(slots.wrapping_add(4), width);
        set32(h, WIDTH, width);
        set32(h, CURSOR, 0);
        set32(h, REFERENCE, 0);
        cdecl(h, PACKET_BEGIN_TAG, &[window]);
        cdecl(h, SET_TEST, &[1, 5, 0x7f, 0, 0, 0, 0, 1]);
        let mut mouse_x = cdecl(h, MOUSE_WINDOW_Y, &[window]).wrapping_sub(0x4c);
        if i(cdecl(h, MOUSE_WINDOW_X, &[window])) < 0 {
            mouse_x = u32::MAX;
        } else {
            let half = i(cdecl(h, GET_SIZE_H, &[window])) / 2;
            if i(cdecl(h, MOUSE_WINDOW_X, &[window])) > half {
                mouse_x = u32::MAX;
            }
        }
        let text = h.rebase(HOVER_TEXT);
        h.write(text, 0_u8);
        cdecl(h, SET_PRIM, &[6]);
        meter_segments(h, rect, mouse_x);
        let hover = h.read::<u8>(text) != 0;
        let w = g32(h, METER_WINDOW);
        cdecl(h, SET_FLAG2, &[w, u32::from(!hover)]);
        if h.read::<u8>(text) != 0 {
            let w = g32(h, METER_WINDOW);
            cdecl(h, TO_FRONT, &[w]);
        }
        cdecl(h, SET_RGBAQ, &[0xc0, 0xc0, 0xe0, 0xff]);
        let c = g32(h, CURSOR).wrapping_add(0x4c);
        cdecl(h, SET_XYZF2, &[c, 0]);
        let right = h.read::<u32>(slots.wrapping_add(4)).wrapping_add(0x4c);
        cdecl(h, SET_XYZF2, &[right, 5]);
        cdecl(h, SET_PRIM, &[2]);
        cdecl(h, SET_RGBAQ, &[0, 0, 0, 0xff]);
        cdecl(h, DRAW_RECT_STRIP, &[rect]);
        h.write(rect, 0x0006_004d_u32);
        cdecl(h, SET_PRIM, &[6]);
        cdecl(h, SET_RGBAQ, &[0xc0, 0xc0, 0xff, 0xff]);
        cdecl(h, SET_XYZF2, &[0x4c, 6]);
        cdecl(h, SET_XYZF2, &[right, 0xb]);
        cdecl(h, SET_PRIM, &[6]);
        let q = mul_div(g32(h, WIDTH), g32(h, LAST_TIME), g32(h, SCALE));
        cdecl(h, SET_RGBAQ, &[0x40, 0x80, 0x30, 0xff]);
        cdecl(h, SET_XYZF2, &[0x4d, 6]);
        cdecl(h, SET_XYZF2, &[q.wrapping_add(0x4d), 0xb]);
        cdecl(h, SET_PRIM, &[2]);
        cdecl(h, SET_RGBAQ, &[0, 0, 0, 0xff]);
        cdecl(h, DRAW_RECT_STRIP, &[rect]);
        cdecl(h, SET_PRIM, &[1]);
        let mut tick = 0x200_u32;
        while i(tick) < i(g32(h, SCALE)) {
            let x = mul_div(g32(h, WIDTH), tick, g32(h, SCALE)).wrapping_add(0x4d);
            let k = tick.wrapping_sub(0x100).cast_signed() >> 8;
            let colour = if k == 0 {
                0xff
            } else {
                g32(h, TICK_BASE).wrapping_shl(k.cast_unsigned().wrapping_add(4)) & 0xff
            };
            cdecl(h, SET_RGBAQ, &[colour, 0, 0, 0xff]);
            for dx in [0_u32, u32::MAX, 1] {
                let xx = x.wrapping_add(dx);
                cdecl(h, SET_XYZF2, &[xx, 0]);
                cdecl(h, SET_XYZF2, &[xx, 0xb]);
            }
            tick = tick.wrapping_add(0x100);
        }
        cdecl(h, PACKET_END_TAG, &[]);
        cdecl(h, PRINT_SET_SIZE_ONLY, &[0x100]);
        let t = cdecl(h, FRAME_TIME, &[]);
        set32(h, LAST_TIME, t);
    }
}
