//! M5 stage 1, list `menu`, batch 6: the menu's packet-drawing wrappers
//! (`g_TkMenuPacket` moved by 16 for each call's header), the cross
//! button icon, the UI rectangles drawn through `graphicDrawUIElement`
//! and the event GUI's selector glue.
//!
//! - **UI element record** (`graphicDrawUIElement`'s first argument, the
//!   caller's frame at `ebp - 0x9c`): `+0x00`/`+0x04` the top left
//!   (`f32`), `+0x10..+0x1c` the first colour (four `i32` bytes),
//!   `+0x20`/`+0x24` the bottom right (`f32`), `+0x30..+0x3c` the second
//!   colour, `+0x80` 1000, `+0x88`, `+0x90` and `+0x94` 0 (the rectangle
//!   draws; `DrawBlackBackGround` leaves those four unwritten: junk reads
//!   of the callee, 0 in the port, brief 2026-10-24).

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

pub mod bridge {
    use ffx_bridge::Global;
    use ffx_graphic::host::{Conv, Host};

    /// The menu GS packet pointer.
    pub const MENU_PACKET: Global<u32> = Global::new(0x0187_1530);
    /// The event GUI's selector and its value.
    pub const EVE_SELECTOR: Global<u32> = Global::new(0x0132_592c);
    pub const EVE_VALUE: Global<u32> = Global::new(0x0132_5928);

    pub const GRAPHIC_DRAW_UI_ELEMENT: u32 = 0x0063_eee0;
    pub const GRAPHIC_GET_ABMAP: u32 = 0x0064_1060;
    pub const TO_GET_WIDTH_EXT_MES_1612: u32 = 0x008b_8070;
    pub const PKT_MES_008E9FD0: u32 = 0x008e_9fd0;
    pub const PKT_MES_MACRO_008EA5C0: u32 = 0x008e_a5c0;
    pub const PKT_WINDOW_008EADF0: u32 = 0x008e_adf0;
    pub const PKT_SP3R_008EBBD0: u32 = 0x008e_bbd0;
    pub const TK_MENU_DRAW_SCISSOR_RECT: u32 = 0x008d_d460;
    pub const DRAW_CROSS_BUTTON_ICON_RGBA: u32 = 0x008e_6070;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const SG_GET_RESOURCE: u32 = 0x0083_fbe0;
    pub const SG_SET_RESOURCE: u32 = 0x0083_fe10;
    pub const SG_SET_RESOURCE_STR: u32 = 0x0083_fe60;
    pub const GU_SELECTOR_GET_ITEM_VALUE: u32 = 0x0084_9d10;
    pub const GU_SELECTOR_GET_ITEM_NAME: u32 = 0x0084_9d30;
    pub const GU_WINDOW_CONTROL_SET_TEXT: u32 = 0x0084_a710;
    pub const ATEL_SET_EVENT_JUMP: u32 = 0x0086_fec0;
    /// `gui_eve_Init`'s banner and the black background's corner, in
    /// `.rdata`.
    pub const EVE_BANNER: u32 = 0x00b5_b410;
    pub const SCREEN_RIGHT: u32 = 0x00b4_1d9c;
    pub const SCREEN_BOTTOM: u32 = 0x00b4_1d98;

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// The packet wrappers' pattern: the menu packet moves 16 bytes for
    /// the call's header (returned: the callee gets it first) ...
    fn packet_begin(h: &mut impl Host) -> u32 {
        let p = MENU_PACKET.get(h).wrapping_add(0x10);
        MENU_PACKET.set(h, p);
        p
    }

    /// ... and the callee returns where the packet goes on.
    fn packet_end(h: &mut impl Host, r: u32) -> u32 {
        MENU_PACKET.set(h, r);
        r
    }

    /// `TkMenuDrawSp3R(x, y, flags)` (`0x008dca60`; `TkMenuDrawSp3R0` calls
    /// it with flags 0).
    pub fn draw_sp3_r0(h: &mut impl Host, a: u32, b: u32, c: u32) -> u32 {
        let p = packet_begin(h);
        let r = cdecl(h, PKT_SP3R_008EBBD0, &[p, a, b, c]);
        packet_end(h, r)
    }

    /// `TkMenuDraw1612WidthHeight(text, flags)` (`0x008dc9f0`):
    /// `TOGetWidthExtMes1612(flags, text)` (its arguments swapped).
    pub fn draw_1612_width_height(h: &mut impl Host, text: u32, flags: u32) -> u32 {
        cdecl(h, TO_GET_WIDTH_EXT_MES_1612, &[flags, text])
    }

    /// `TkMenuDrawEasyMes1612(x, y, text)` (`0x008dcb80`).
    pub fn draw_easy_mes_1612(h: &mut impl Host, x: u32, y: u32, text: u32) -> u32 {
        let p = packet_begin(h);
        let r = cdecl(h, PKT_MES_008E9FD0, &[p, x, y, text]);
        packet_end(h, r)
    }

    /// `TkMenuDrawEasyMes1612WithMacro(x, y, text, macro)` (`0x008dcc70`):
    /// the callee takes `y` before `x`.
    pub fn draw_easy_mes_1612_with_macro(
        h: &mut impl Host,
        x: u32,
        y: u32,
        text: u32,
        m: u32,
    ) -> u32 {
        let p = packet_begin(h);
        let r = cdecl(h, PKT_MES_MACRO_008EA5C0, &[p, y, x, text, m]);
        packet_end(h, r)
    }

    /// `TkMenuDrawWindow1612(x, y, w, h)` (`0x008dcf40`, name ours): the window, and
    /// when larger than 36x10 the scissor of its inside
    /// (`TkMenuDrawScissorRect(x + 18, y + 5, w - 36, h - 10)`). Returns
    /// whether it clipped.
    pub fn draw_window_1612(h: &mut impl Host, x: u32, y: u32, w: u32, ht: u32) -> u32 {
        let p = packet_begin(h);
        let r = cdecl(h, PKT_WINDOW_008EADF0, &[p, x, y, w, ht]);
        packet_end(h, r);
        if (w as i32) > 0x24 && (ht as i32) > 0xa {
            let args = [
                x.wrapping_add(0x12),
                y.wrapping_add(5),
                w.wrapping_sub(0x24),
                ht.wrapping_sub(0xa),
            ];
            cdecl(h, TK_MENU_DRAW_SCISSOR_RECT, &args);
            1
        } else {
            0
        }
    }

    /// An `f32` word as `fld dword` / `fstp dword` copy it: a signalling
    /// NaN comes out quiet.
    fn fcopy(bits: u32) -> u32 {
        if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
            bits | 0x0040_0000
        } else {
            bits
        }
    }

    /// `DrawCrossButtonIcon(id, x, y, w, h)` (`0x008e6030`): the RGBA
    /// variant in grey (`0x80808080` twice).
    pub fn draw_cross_button_icon(
        h: &mut impl Host,
        id: u32,
        x: u32,
        y: u32,
        w: u32,
        ht: u32,
    ) -> u32 {
        let args = [
            id,
            fcopy(x),
            fcopy(y),
            fcopy(w),
            fcopy(ht),
            0x8080_8080,
            0x8080_8080,
        ];
        cdecl(h, DRAW_CROSS_BUTTON_ICON_RGBA, &args)
    }

    /// A colour's four bytes as the record's four words.
    fn colour(h: &mut impl Host, at: u32, c: u32) {
        for k in 0..4 {
            h.write(at.wrapping_add(4 * k), (c >> (8 * k)) & 0xff);
        }
    }

    /// Fills the UI element record at `ebp - 0x9c` and draws it
    /// (`graphicDrawUIElement(record, 0, 1, 0, 0)`). `tail` writes the
    /// four trailing words (the rectangles do; the background does not).
    fn draw_record(
        h: &mut impl Host,
        ebp: u32,
        corners: [u32; 4],
        c0: u32,
        c1: u32,
        tail: bool,
    ) -> u32 {
        let rec = ebp.wrapping_sub(0x9c);
        h.write(rec, corners[0]);
        h.write(rec.wrapping_add(4), corners[1]);
        colour(h, rec.wrapping_add(0x10), c0);
        h.write(rec.wrapping_add(0x20), corners[2]);
        h.write(rec.wrapping_add(0x24), corners[3]);
        colour(h, rec.wrapping_add(0x30), c1);
        if tail {
            h.write(ebp.wrapping_sub(0xc), 0u32);
            h.write(ebp.wrapping_sub(8), 0u32);
            h.write(ebp.wrapping_sub(0x1c), 1000u32);
            h.write(ebp.wrapping_sub(0x14), 0u32);
        }
        cdecl(h, GRAPHIC_DRAW_UI_ELEMENT, &[rec, 0, 1, 0, 0])
    }

    /// `TkMn2DrawCrossRect(x, y, w, h, c0, c1)` (`0x008c0930`, `f32`
    /// rectangle; frame at `ebp`): corners `(x, y)` and `(x + w, y + h)`,
    /// each sum rounded once to `f32`.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_cross_rect(
        h: &mut impl Host,
        ebp: u32,
        x: u32,
        y: u32,
        w: u32,
        ht: u32,
        c0: u32,
        c1: u32,
    ) -> u32 {
        let f = |b: u32| f64::from(f32::from_bits(b));
        let right = ((f(x) + f(w)) as f32).to_bits();
        let bottom = ((f(y) + f(ht)) as f32).to_bits();
        draw_record(h, ebp, [fcopy(x), fcopy(y), right, bottom], c0, c1, true)
    }

    /// `TkMn2DrawRect(x, y, w, h, c0, c1)` (`0x008c1280`, `i32`
    /// rectangle; frame at `ebp`): the corners converted to `f32`, the
    /// right and bottom edges summed as integers first.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_rect(
        h: &mut impl Host,
        ebp: u32,
        x: u32,
        y: u32,
        w: u32,
        ht: u32,
        c0: u32,
        c1: u32,
    ) -> u32 {
        let f = |v: u32| (f64::from(v as i32) as f32).to_bits();
        let corners = [f(x), f(y), f(x.wrapping_add(w)), f(y.wrapping_add(ht))];
        // the original builds the sums in its own ebp - 0xa0 on the way
        h.write(ebp.wrapping_sub(0xa0), y.wrapping_add(ht));
        draw_record(h, ebp, corners, c0, c1, true)
    }

    /// `DrawBlackBackGround()` (`0x008db870`, frame at `ebp`): unless the
    /// ability map is up (`graphicGetAbmap() == 1`, returned), the whole
    /// screen in half-transparent black.
    pub fn draw_black_background(h: &mut impl Host, ebp: u32) -> u32 {
        let abmap = cdecl(h, GRAPHIC_GET_ABMAP, &[]);
        if abmap == 1 {
            return 1;
        }
        let (r, b) = (h.rebase(SCREEN_RIGHT), h.rebase(SCREEN_BOTTOM));
        let corners = [0, 0, fcopy(h.read(r)), fcopy(h.read(b))];
        // junk reads: ebp-0xc, -8, -0x1c, -0x14 never written; 0 (brief 2026-10-24)
        draw_record(h, ebp, corners, 0x8000_0000, 0x8000_0000, false)
    }

    /// `gui_eve_Init()` (`0x00854e60`): the banner printed and the event
    /// GUI's value variable created (`0x0083fbe0(4, 0)`). Returns it.
    pub fn eve_init(h: &mut impl Host) -> u32 {
        let banner = h.rebase(EVE_BANNER);
        cdecl(h, DBG_PRINTF, &[banner]);
        let v = cdecl(h, SG_GET_RESOURCE, &[4, 0]);
        EVE_VALUE.set(h, v);
        v
    }

    /// `gui_eve_GWM_SELECTOR(window, msg)` (`0x00854df0`): on a selection
    /// (0x2c) the selector's value and name go to the window's text
    /// (`0x0084a710(window, 0x2d, name)`), the variables 4 and 3, and the
    /// event (`0x0086fec0(value, 0)`). Returns the last call's `eax`, or
    /// `None` for other messages, where the original leaves `eax` as its
    /// caller had it.
    pub fn eve_gwm_selector(h: &mut impl Host, window: u32, msg: u32) -> Option<u32> {
        if msg != 0x2c {
            return None;
        }
        let sel = EVE_SELECTOR.get(h);
        let v = cdecl(h, GU_SELECTOR_GET_ITEM_VALUE, &[sel]);
        let sel = EVE_SELECTOR.get(h);
        EVE_VALUE.set(h, v);
        let name = cdecl(h, GU_SELECTOR_GET_ITEM_NAME, &[sel]);
        cdecl(h, GU_WINDOW_CONTROL_SET_TEXT, &[window, 0x2d, name]);
        let v = EVE_VALUE.get(h);
        cdecl(h, SG_SET_RESOURCE, &[4, v]);
        let sel = EVE_SELECTOR.get(h);
        let name = cdecl(h, GU_SELECTOR_GET_ITEM_NAME, &[sel]);
        cdecl(h, SG_SET_RESOURCE_STR, &[3, name]);
        let v = EVE_VALUE.get(h);
        Some(cdecl(h, ATEL_SET_EVENT_JUMP, &[v, 0]))
    }

    pub const TK_MENU_DRAW_KICK: u32 = 0x008d_ce50;
    pub const TK_VU1_SYNC_PATH_THUNK: u32 = 0x008c_1790;
    pub const TK_MN2_DRAW_INIT_PACKET: u32 = 0x008c_0be0;
    pub const TK_MN2_DRAW_KICK_SYNC_PACKET: u32 = 0x008c_0cc0;
    pub const MENU_MN2_STEP_DRAW_Y80: u32 = 0x008c_0dd0;
    pub const SCE_GS_SYNC_PATH: u32 = 0x0062_f880;
    pub const SCE_GS_SET_DEF_LOAD_IMAGE: u32 = 0x0062_f820;
    pub const SCE_GS_EXEC_LOAD_IMAGE: u32 = 0x0062_f7a0;
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    /// The load-image descriptor and the image's strips (13 of 0xc000
    /// bytes: 512x32 at 8 bits... 0x1a00 words).
    pub const LOAD_IMAGE: u32 = 0x0184_1cd0;
    pub const SCREEN_IMAGE: u32 = 0x0141_0870;

    /// `TkMn2DrawScreenStrips(a, b, y)` (`0x008a9510`): the menu packets
    /// kicked twice, the VU1 path synced, the draw packet set up, then 13
    /// strips of the saved screen image (`0x01410870`, 0xc000 bytes each)
    /// loaded into the GS (`sceGsSetDefLoadImage(desc, 0x1a00, 8, 1, 0, 0,
    /// 0x200, 0x20)`, `sceGsExecLoadImage`) and drawn 512 lines apart
    /// (`MenuMn2_StepDrawY80(y, line, a, b)`, the packet kicked). Returns
    /// the last `sceGsSyncPath`'s `eax`.
    pub fn draw_screen_strips(h: &mut impl Host, a: u32, b: u32, y: u32) -> u32 {
        cdecl(h, TK_MENU_DRAW_KICK, &[]);
        cdecl(h, TK_MENU_DRAW_KICK, &[]);
        cdecl(h, TK_VU1_SYNC_PATH_THUNK, &[]);
        cdecl(h, TK_MN2_DRAW_INIT_PACKET, &[]);
        cdecl(h, SCE_GS_SYNC_PATH, &[0, 0]);
        let desc = h.rebase(LOAD_IMAGE);
        let mut image = h.rebase(SCREEN_IMAGE);
        let mut line = y;
        for _ in 0..13 {
            cdecl(
                h,
                SCE_GS_SET_DEF_LOAD_IMAGE,
                &[desc, 0x1a00, 8, 1, 0, 0, 0x200, 0x20],
            );
            cdecl(h, FLUSH_CACHE, &[0]);
            cdecl(h, SCE_GS_SYNC_PATH, &[0, 0]);
            cdecl(h, SCE_GS_EXEC_LOAD_IMAGE, &[desc, image]);
            cdecl(h, SCE_GS_SYNC_PATH, &[0, 0]);
            cdecl(h, MENU_MN2_STEP_DRAW_Y80, &[y, line, a, b]);
            cdecl(h, TK_MN2_DRAW_KICK_SYNC_PACKET, &[]);
            image = image.wrapping_add(0xc000);
            line = line.wrapping_add(0x200);
        }
        cdecl(h, SCE_GS_SYNC_PATH, &[0, 0])
    }
}
