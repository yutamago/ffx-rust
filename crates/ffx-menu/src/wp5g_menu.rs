//! M5 finish, WP5 part 6 (wp5g): menu functions in gaps of the dump that
//! the build without Unicorn stopped at or reaches: the summon screen's
//! layout read (`code_008cc1a0` jumps to it), the window callbacks the
//! window tables of `crate::tk` and `ffx_system::tolib` register (the
//! main command window's and the top window's control, the summon,
//! customize and weapon-shop windows' draws and tests, two close steps)
//! and the battle cursors' drawer (`TODrawBtlWindow`'s tail jump). The
//! main command window's draw callback `+0x10` is
//! `MenuMain_DrawCmdWindow`, ported in `crate::tk`. Read from the machine
//! code.

#[allow(clippy::similar_names, clippy::many_single_char_names)]
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    pub const TK_MN2_GET_COMMON_PLATE: u32 = 0x008c_1a60;
    pub const TK_MN2_SEQ_GET_NUM: u32 = 0x008c_2b10;
    pub const TK_MN2_SEQ_GET_RECT: u32 = 0x008c_2b90;
    pub const TK_MN2_SEQ_GET_POS: u32 = 0x008c_2b50;

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn rect(h: &mut impl Host, name: u32, out: u32) -> u32 {
        let (name, out) = (h.rebase(name), h.rebase(out));
        cdecl(h, TK_MN2_SEQ_GET_RECT, &[name, out])
    }

    fn pos(h: &mut impl Host, name: u32, out: u32) -> u32 {
        let (name, out) = (h.rebase(name), h.rebase(out));
        cdecl(h, TK_MN2_SEQ_GET_POS, &[name, out])
    }

    fn plate(h: &mut impl Host, at: [u32; 4]) -> u32 {
        let [p, r, c1, c2] = at.map(|x| h.rebase(x));
        cdecl(h, TK_MN2_GET_COMMON_PLATE, &[p, r, c1, c2, 0, 0])
    }

    /// `MenuSummon_InitLayout_008ceef0()`: the summon screen's layout:
    /// two plates (shapes 0, 0), four positions into `0x023cc424..`, three
    /// rects and seven positions into `0x0186a5cc..`, and the row count
    /// into `0x0186a5dc`, which it returns.
    pub fn summon_init_layout(h: &mut impl Host) -> u32 {
        plate(h, [0x0186_a5b4, 0x00b6_2b10, 0x00b6_2b1c, 0x00b6_2b28]);
        plate(h, [0x023c_c440, 0x00b6_5c44, 0x00b6_5c34, 0x00b6_5c24]);
        for (name, out) in [
            (0x00b6_5c50, 0x023c_c430),
            (0x00b6_5c5c, 0x023c_c42c),
            (0x00b6_5c68, 0x023c_c428),
            (0x00b6_5c74, 0x023c_c424),
        ] {
            pos(h, name, out);
        }
        for (name, out) in [
            (0x00b6_5c80, 0x0186_a608),
            (0x00b6_5c90, 0x0186_a5e8),
            (0x00b6_5ca0, 0x0186_a5f0),
        ] {
            rect(h, name, out);
        }
        for (name, out) in [
            (0x00b6_5cb0, 0x0186_a5f8),
            (0x00b6_2b34, 0x0186_a5cc),
            (0x00b6_5cb8, 0x0186_a5d0),
            (0x00b6_5cc4, 0x0186_a5d4),
            (0x00b6_5cd4, 0x0186_a5d8),
            (0x00b6_28dc, 0x0186_a5e0),
            (0x00b6_5ce4, 0x0186_a5e4),
        ] {
            pos(h, name, out);
        }
        let num = h.rebase(0x00b6_5cf0);
        let n = cdecl(h, TK_MN2_SEQ_GET_NUM, &[num]);
        h.write(h.rebase(0x0186_a5dc), n);
        n
    }

    /// The main command window's globals (`ffx.sym`): done flag, slide
    /// (i32), busy mask (u16), option cursor, row delays and row slides
    /// (u16[8] each).
    pub const CMD_DONE: u32 = 0x0187_1554;
    pub const CMD_SLIDE: u32 = 0x0187_1558;
    pub const CMD_BUSY: u32 = 0x0187_1564;
    pub const OPTION_CURSOR: u32 = 0x0187_1548;
    pub const ROW_DELAY: u32 = 0x0187_1668;
    pub const ROW_SLIDE: u32 = 0x0187_1678;

    /// `TkMenuMainCmdWindow_ctrl_008e2040()`: with the done flag set the
    /// slide grows by 0x155 up to 0x1000 (signed), and each of the eight
    /// rows counts its delay down, or once it is 0 slides by 0xe3 up to
    /// 0x1000 (signed 16-bit); the busy mask is 0x1000 and'ed with every
    /// row's slide (0x1000 once all are in). Returns 0x10 (the loop's
    /// end). Without the flag the slide shrinks by 0x155 down to 0; that
    /// slide is the result.
    pub fn cmd_window_ctrl(h: &mut impl Host) -> u32 {
        let slide = h.rebase(CMD_SLIDE);
        if h.read::<u32>(h.rebase(CMD_DONE)) == 0 {
            let v = h.read::<u32>(slide).wrapping_sub(0x155);
            let v = if v.cast_signed() < 0 { 0 } else { v };
            h.write(slide, v);
            return v;
        }
        let v = h.read::<u32>(slide).wrapping_add(0x155);
        h.write(slide, if v.cast_signed() > 0x1000 { 0x1000 } else { v });
        let (delay, row) = (h.rebase(ROW_DELAY), h.rebase(ROW_SLIDE));
        let mut busy = 0x1000u16;
        for i in 0..8 {
            let d = delay.wrapping_add(2 * i);
            let s = row.wrapping_add(2 * i);
            let c: u16 = h.read(d);
            if c == 0 {
                let v = h.read::<u16>(s).wrapping_add(0xe3);
                h.write(s, if v.cast_signed() > 0x1000 { 0x1000 } else { v });
            } else {
                h.write(d, c - 1);
            }
            busy &= h.read::<u16>(s);
        }
        h.write(h.rebase(CMD_BUSY), busy);
        0x10
    }

    /// `remapY2(f32 [sym])` / `remapX2(f32 [sym])` as the `f32` the
    /// callers store.
    fn remap(h: &mut impl Host, f: u32, sym: u32) -> u32 {
        let v: u32 = h.read(h.rebase(sym));
        h.call_sym_st0(f, Conv::Cdecl, &[v]).to_bits()
    }

    pub const UI_REMAP_Y2: u32 = 0x0064_4820;
    pub const UI_REMAP_X2: u32 = 0x0064_47e0;
    pub const TO_MAKE_PKT_SHAPE_CURSOR: u32 = 0x008f_eb00;
    /// The battle cursors queued this frame: count (`i16`) and records
    /// (8 bytes: kind, `x`, `y` as `i16` at `+0`, `+4`, `+6`).
    pub const BTL_CURSOR_COUNT: u32 = 0x0133_d6f0;
    pub const BTL_CURSORS: u32 = 0x0133_f7b4;

    /// `TOBtlDrawCursors_008930f0()` (`TODrawBtlWindow`'s tail jump): each
    /// queued cursor drawn (`TOMakePktShapeCursor(kind, x, y)`, signed
    /// halves), the count reread each turn, then cleared; 0.
    pub fn btl_draw_cursors(h: &mut impl Host) -> u32 {
        let count = h.rebase(BTL_CURSOR_COUNT);
        let recs = h.rebase(BTL_CURSORS);
        let mut i = 0u32;
        while i.cast_signed() < i32::from(h.read::<i16>(count)) {
            let r = recs.wrapping_add(8 * i);
            let kind = i32::from(h.read::<i16>(r)).cast_unsigned();
            let x = i32::from(h.read::<i16>(r.wrapping_add(4))).cast_unsigned();
            let y = i32::from(h.read::<i16>(r.wrapping_add(6))).cast_unsigned();
            cdecl(h, TO_MAKE_PKT_SHAPE_CURSOR, &[kind, x, y]);
            i += 1;
        }
        h.write(count, 0u16);
        0
    }

    /// `GetGlobal01871558_008e2250()` and the top window's fade words
    /// (`0x01841c08`: the fade, `0x01841c0c`: the slide, `0x01841c10`: its
    /// step).
    pub const GET_CMD_SLIDE: u32 = 0x008e_2250;
    pub const TOP_FADE: u32 = 0x0184_1c08;
    pub const TOP_SLIDE: u32 = 0x0184_1c0c;
    pub const TOP_SLIDE_STEP: u32 = 0x0184_1c10;

    /// `TOMenuTopWindow_ctrl_008a91d0()` (the main window
    /// `menu_main_window_open` opens, `+0xc`): `0x008e2250()`, the fade
    /// += 0x555 (held at 0x1000 from 0x1000 up), the slide += its step
    /// (held in 0..=0x1000, signed); the new fade (before the hold).
    pub fn top_window_ctrl(h: &mut impl Host) -> u32 {
        cdecl(h, GET_CMD_SLIDE, &[]);
        let (fade, slide) = (h.rebase(TOP_FADE), h.rebase(TOP_SLIDE));
        let a = h.read::<u32>(fade).wrapping_add(0x555);
        let step: u32 = h.read(h.rebase(TOP_SLIDE_STEP));
        let c = h.read::<u32>(slide).wrapping_add(step);
        h.write(fade, a);
        h.write(slide, c);
        if c.cast_signed() < 0 {
            h.write(slide, 0u32);
        } else if c.cast_signed() > 0x1000 {
            h.write(slide, 0x1000u32);
        }
        if a.cast_signed() >= 0x1000 {
            h.write(fade, 0x1000u32);
        }
        a
    }

    pub const TO_GET_FFX_LANG_JMP: u32 = 0x008a_c2f0;
    pub const PASSWORD_TEXT_DISPOSE: u32 = 0x0079_02f0;

    /// `0x008c4490()` (a window's `+0x14` close callback): with the
    /// language jump 9 or 10, `PasswordTextDispose()`; 1.
    pub fn password_close(h: &mut impl Host) -> u32 {
        if cdecl(h, TO_GET_FFX_LANG_JMP, &[]) == 9 || cdecl(h, TO_GET_FFX_LANG_JMP, &[]) == 10 {
            cdecl(h, PASSWORD_TEXT_DISPOSE, &[]);
        }
        1
    }

    pub const TK_MENU_GET_CURRENT_SUMMON: u32 = 0x008a_9880;
    /// The summon screen's cursor window (`0x023cc128`) and its rows'
    /// summon bytes (`0x0186a5b0`).
    pub const SUMMON_CURSOR_WINDOW: u32 = 0x023c_c128;
    pub const SUMMON_ROW_BYTES: u32 = 0x0186_a5b0;

    /// `0x008ccad0()` (the summon window's `+0x1c` test): 0 when the row
    /// under the cursor window (`+0x48`, signed) holds 1 and the current
    /// summon is 14 or more (signed), else 1.
    pub fn summon_row_test(h: &mut impl Host) -> u32 {
        let w: u32 = h.read(h.rebase(SUMMON_CURSOR_WINDOW));
        let v = if w == 0 {
            0
        } else {
            let row = i32::from(h.read::<i16>(w.wrapping_add(0x48))).cast_unsigned();
            h.read::<u8>(h.rebase(SUMMON_ROW_BYTES).wrapping_add(row))
        };
        let e = cdecl(h, TK_MENU_GET_CURRENT_SUMMON, &[]);
        if v != 1 {
            return 1;
        }
        u32::from(e.cast_signed() < 0xe)
    }

    pub const TO_DRAW_MENU_BG: u32 = 0x008f_5c10;
    pub const MENU_DRAW_DIVIDER_SHAPES: u32 = 0x008e_71d0;
    pub const TK_MENU_DRAW_KICK: u32 = 0x008d_ce50;
    pub const TK_MN2_DRAW_INIT_PACKET: u32 = 0x008c_0be0;
    pub const MENU_STATUS_DRAW_PAGE: u32 = 0x008d_4ee0;
    pub const TK_MN2_MAIN_STATUS_PLAYER_STATUS: u32 = 0x008d_5180;
    pub const TO_MENU_DRAW_CHR_STATUS_PLATE: u32 = 0x008f_f490;
    pub const TK_MN2_GET_STATUS_BG: u32 = 0x008d_4910;
    pub const TK_MN2_DRAW_STATUS_BG: u32 = 0x008d_4170;
    pub const TK_MN2_DRAW_KICK_SYNC_PACKET: u32 = 0x008c_0cc0;

    /// `0x008cd080()` (the summon status window's draw, `+0x10`): the menu
    /// background, divider shapes 6, the current summon's status page
    /// (`MenuStatus_DrawPage(0, 0, s, 1, ...0x023cc3c8.., 0x1000)`), its
    /// player status at `ftol(remapY2(k) + 0.5)`, its status plate at the
    /// remapped position, the status background, then a tail jump to
    /// `TkMn2DrawKickSyncPacket` (its `eax`).
    pub fn summon_status_draw(h: &mut impl Host) -> u32 {
        cdecl(h, TO_DRAW_MENU_BG, &[]);
        cdecl(h, MENU_DRAW_DIVIDER_SHAPES, &[6]);
        let s = cdecl(h, TK_MENU_GET_CURRENT_SUMMON, &[]);
        cdecl(h, TK_MENU_DRAW_KICK, &[]);
        cdecl(h, TK_MENU_DRAW_KICK, &[]);
        cdecl(h, TK_MN2_DRAW_INIT_PACKET, &[]);
        let p = [0x023c_c3c8, 0x023c_c3c4, 0x023c_c3c0, 0x023c_c3bc].map(|a| h.rebase(a));
        cdecl(
            h,
            MENU_STATUS_DRAW_PAGE,
            &[0, 0, s, 1, p[0], p[1], p[2], p[3], 0x1000],
        );
        let k: u32 = h.read(h.rebase(0x00b5_eea8));
        let y = h.call_sym_st0_f64(UI_REMAP_Y2, Conv::Cdecl, &[k]);
        let half = f64::from_bits(h.read(h.rebase(0x00b0_db90)));
        let y = ffx_bridge::x87::ftol(h, ffx_bridge::x87::add(y, half));
        cdecl(h, TK_MN2_MAIN_STATUS_PLAYER_STATUS, &[0, y, s, 0x1000]);
        let y = remap(h, UI_REMAP_Y2, 0x00b6_5d7c);
        let x = remap(h, UI_REMAP_X2, 0x00b5_ef1c);
        cdecl(h, TO_MENU_DRAW_CHR_STATUS_PLATE, &[s, x, y]);
        let bg = cdecl(h, TK_MN2_GET_STATUS_BG, &[1]);
        cdecl(h, TK_MN2_DRAW_STATUS_BG, &[bg, 1]);
        cdecl(h, TK_MN2_DRAW_KICK_SYNC_PACKET, &[])
    }

    /// `0x008d56e0()` (a customize window's `+8` callback): `0x0186aa68`
    /// cleared (no result: the caller's `eax` stays).
    pub fn kaizou_clear(h: &mut impl Host) {
        h.write(h.rebase(0x0186_aa68), 0u32);
    }

    pub const READ_MODULE_FILES: u32 = 0x0077_7970;

    /// `0x008e2910()` (a window's close step): the module files read
    /// (`Thunk_hdd_ReadModuleFiles`), `0x023cc130` = 1; the read's `eax`.
    pub fn module_files_close(h: &mut impl Host) -> u32 {
        let r = cdecl(h, READ_MODULE_FILES, &[]);
        h.write(h.rebase(0x023c_c130), 1u32);
        r
    }

    pub const MS_GET_GIL: u32 = 0x0078_4e80;
    pub const TK_VU1_SYNC_PATH_THUNK: u32 = 0x008c_1790;
    pub const TK_MN2_DRAW_GILL_PLATE: u32 = 0x008c_0a20;

    /// The gil plate the shop windows draw: `TkMn2DrawGillPlate(x, y, w,
    /// h, gil)` at the remapped `0x00b66500`, `0x00b664d8`, `0x00b5ef4c`,
    /// `0x00b41d84` (remapped last first, as the original pushes them).
    fn gil_plate(h: &mut impl Host, gil: u32) -> u32 {
        let hh = remap(h, UI_REMAP_Y2, 0x00b4_1d84);
        let w = remap(h, UI_REMAP_X2, 0x00b5_ef4c);
        let y = remap(h, UI_REMAP_Y2, 0x00b6_64d8);
        let x = remap(h, UI_REMAP_X2, 0x00b6_6500);
        cdecl(h, TK_MN2_DRAW_GILL_PLATE, &[x, y, w, hh, gil])
    }

    /// The shop draws' start: the gil (`MsGetGIL`), two draw kicks, the
    /// VU1 sync and the packet's start.
    fn shop_draw_start(h: &mut impl Host) -> u32 {
        let gil = cdecl(h, MS_GET_GIL, &[]);
        cdecl(h, TK_MENU_DRAW_KICK, &[]);
        cdecl(h, TK_MENU_DRAW_KICK, &[]);
        cdecl(h, TK_VU1_SYNC_PATH_THUNK, &[]);
        cdecl(h, TK_MN2_DRAW_INIT_PACKET, &[]);
        gil
    }

    /// `0x008da6b0()` (the weapon shop's window 3 draw, `+0x10`): the gil
    /// plate, then a tail jump to `TkMn2DrawKickSyncPacket` (its `eax`).
    pub fn wshop_window3_draw(h: &mut impl Host) -> u32 {
        let gil = shop_draw_start(h);
        gil_plate(h, gil);
        cdecl(h, TK_MN2_DRAW_KICK_SYNC_PACKET, &[])
    }

    pub const TO_MKP_SHAPE_XYWH_UV: u32 = 0x0090_3bb0;
    pub const TO_MKP_ROTATE_SHAPE_XYWH_UVC2: u32 = 0x0090_2ef0;
    pub const MENU_ITEM_DRAW_CHR_SMALL_PLATE: u32 = 0x008c_1260;
    pub const MENU_WSHOP_DRAW_COMPARE: u32 = 0x008d_8620;
    /// The sub window 4's character, and the two compare windows.
    pub const WSHOP_CHR: u32 = 0x0186_aba0;
    pub const WSHOP_COMPARE: [u32; 2] = [0x0186_aaf0, 0x0186_aaf4];

    fn f32_at(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    /// `0x008d7f50()` (the weapon shop's sub window 4 draw, `+0x10`): an
    /// arrow shape (`TOMkpShapeXYWHUV(-1, ...)`, remapped corners) and its
    /// rotated copy (`TOMkpRotateShapeXYWHUVC2(-1, ..., grey, grey,
    /// [0x00b482f8])`, raw floats), divider shapes 2, the character's small
    /// plate, the compare of the first open compare window (1) or the
    /// second (0), the gil plate, then a tail jump to
    /// `TkMn2DrawKickSyncPacket` (its `eax`).
    pub fn wshop_sub_window4_draw(h: &mut impl Host) -> u32 {
        let gil = shop_draw_start(h);
        let uv = [0x00b5_f3a4, 0x00b5_eea0, 0x00b6_64e4, 0x00b5_eea8].map(|s| f32_at(h, s));
        let hh = remap(h, UI_REMAP_Y2, 0x00b9_2268);
        let w = remap(h, UI_REMAP_X2, 0x00b6_5c14);
        let y = remap(h, UI_REMAP_Y2, 0x00b6_64ec);
        let x = remap(h, UI_REMAP_X2, 0x00b6_5724);
        cdecl(
            h,
            TO_MKP_SHAPE_XYWH_UV,
            &[u32::MAX, x, y, w, hh, uv[0], uv[1], uv[2], uv[3]],
        );
        let raw = [
            0x00b6_64e8,
            0x00b6_5c10,
            0x00b6_64f8,
            0x00b9_2268,
            0x00b5_f3a4,
            0x00b5_eea0,
            0x00b6_64e4,
            0x00b5_eea8,
        ]
        .map(|s| f32_at(h, s));
        let angle = f32_at(h, 0x00b4_82f8);
        let [r0, r1, r2, r3, r4, r5, r6, r7] = raw;
        cdecl(
            h,
            TO_MKP_ROTATE_SHAPE_XYWH_UVC2,
            &[
                u32::MAX,
                r0,
                r1,
                r2,
                r3,
                r4,
                r5,
                r6,
                r7,
                0x8080_8080,
                0x8080_8080,
                angle,
            ],
        );
        cdecl(h, MENU_DRAW_DIVIDER_SHAPES, &[2]);
        let chr: u32 = h.read(h.rebase(WSHOP_CHR));
        let y = remap(h, UI_REMAP_Y2, 0x00b6_5af8);
        let x = remap(h, UI_REMAP_X2, 0x00b5_ef1c);
        cdecl(h, MENU_ITEM_DRAW_CHR_SMALL_PLATE, &[x, y, chr]);
        let first: u32 = h.read(h.rebase(WSHOP_COMPARE[0]));
        if first != 0 {
            cdecl(h, MENU_WSHOP_DRAW_COMPARE, &[first, 1]);
        } else {
            let second: u32 = h.read(h.rebase(WSHOP_COMPARE[1]));
            if second != 0 {
                cdecl(h, MENU_WSHOP_DRAW_COMPARE, &[second, 0]);
            }
        }
        gil_plate(h, gil);
        cdecl(h, TK_MN2_DRAW_KICK_SYNC_PACKET, &[])
    }
}
