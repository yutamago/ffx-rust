//! M5 stage 1, the menu, battle and script leftovers (round 3), batch 2:
//! the menu screens' table entries (`g_MenuHelpExtArg`,
//! `g_NkzAeonStats`, `g_TkMenuStatusStats`: the open, close and draw steps
//! of the config, summon, limit, status, customize and shop screens) and
//! the last jump thunks. None is in the dump; each was read from the
//! machine code.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

pub mod bridge {
    use crate::m5m_h::bridge as h5;
    use ffx_bridge::{Global, x87};
    use ffx_graphic::host::{Conv, Host};

    pub const TK_SND_UPDATE: u32 = 0x0088_7b60;
    pub const TO_GET_FFX_LANG_JMP: u32 = 0x008a_c2f0;
    pub const GET_U32_023CBEB0: u32 = 0x00a7_ae90;
    /// The config screen's open step the row setup tail-jumps to.
    pub const CONFIG_OPEN: u32 = 0x008c_c050;
    /// Summon, equipment, limit and customize screens' bodies the thunks
    /// jump to (not in the dump).
    pub const SUMMON_INIT: u32 = 0x008c_eef0;
    pub const WEAPON_INIT: u32 = 0x008c_fc40;
    pub const LIMIT_INIT: u32 = 0x008d_0cd0;
    pub const KAIZOU_INIT: u32 = 0x008d_6be0;
    pub const TK_MENU_SUMMON_CLOSE_WINDOW: u32 = 0x008c_cb40;
    pub const TK_MN2_SEQ_STOP_SCENE: u32 = 0x008c_2c10;
    pub const MENU_DRAW_DIVIDER_SHAPES: u32 = 0x008e_71d0;
    pub const DRAW_LIMIT_PAGE_OVER_DRIVE: u32 = 0x008d_2270;
    pub const TK_MENU_LIMIT_CLOSE_WINDOW: u32 = 0x008d_0c90;
    pub const TK_MENU_LIMIT_CLOSE_WINDOW2: u32 = 0x008d_0cb0;
    pub const TK_MN2_GET_STATUS_POS: u32 = 0x008d_4920;
    pub const MENU_STATUS_CLOSE_WINDOW: u32 = 0x008d_2680;
    pub const TK_MN2_INIT_STATUS_BG: u32 = 0x008d_4ed0;
    pub const TK_MS_SET_SAVE_PARAM_ALL: u32 = 0x008c_30d0;
    pub const TK_MENU_KAIZOU_CLOSE_WINDOW: u32 = 0x008d_57f0;
    pub const ATEL_REGIST_CALLBACK_QUE2: u32 = 0x0087_51b0;
    pub const TK_MN2_ARMS_REFRESH_ARMS: u32 = 0x008c_fd70;
    pub const MENU_WSHOP_CLOSE_SUB_WINDOW4: u32 = 0x008d_7670;
    pub const TK_MENU_WSHOP_CLOSE_WINDOW3: u32 = 0x008d_9b90;

    /// `g_TkMenuConfigRows`, `g_TkMenuConfigRowList`.
    pub const CONFIG_ROWS: Global<u32> = Global::new(0x0186_a448);
    pub const CONFIG_ROW_LIST: Global<u32> = Global::new(0x0186_a44c);
    /// The four row lists (`.data`): Japanese layout 1/other, then the
    /// other languages' 1/other.
    pub const CONFIG_LIST_JP_1: u32 = 0x00c5_a598;
    pub const CONFIG_LIST_JP: u32 = 0x00c5_a5c0;
    pub const CONFIG_LIST_1: u32 = 0x00c5_a554;
    pub const CONFIG_LIST: u32 = 0x00c5_a578;
    /// `g_TkMenuSummonRestartCount`.
    pub const SUMMON_RESTART_COUNT: Global<i32> = Global::new(0x0186_a5fc);
    /// The equipment screen's three scenes (names in `.rdata`).
    pub const WEAPON_SCENES: [u32; 3] = [0x00b6_5e30, 0x00b6_5e38, 0x00b6_5e40];
    /// The limit page's plate y (`f32`, design units).
    pub const LIMIT_PLATE_Y: u32 = 0x00b6_5af8;
    /// The status screen's six windows.
    pub const STATUS_WINDOWS: u32 = 0x0186_a820;
    /// The customize screen's callback argument (`.rdata`).
    pub const KAIZOU_CALLBACK_ARG: u32 = 0x00b6_5e28;
    /// `g_TkMenuTutoChosen`.
    pub const TUTO_CHOSEN: Global<u32> = Global::new(0x025d_16bc);

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `TkSndUpdate_thunk_008ca470()`: `jmp TkSndUpdate`.
    pub fn tk_snd_update_thunk(h: &mut impl Host) -> u32 {
        cdecl(h, TK_SND_UPDATE, &[])
    }
    /// `code_008cc1a0()`: `jmp 0x008ceef0` (the summon screen's init).
    pub fn summon_init_thunk(h: &mut impl Host) -> u32 {
        cdecl(h, SUMMON_INIT, &[])
    }
    /// `code_008cf820()`: `jmp 0x008cfc40` (the equipment screen's init).
    pub fn weapon_init_thunk(h: &mut impl Host) -> u32 {
        cdecl(h, WEAPON_INIT, &[])
    }
    /// `code_008d0a10()`: `jmp 0x008d0cd0` (the limit screen's init).
    pub fn limit_init_thunk(h: &mut impl Host) -> u32 {
        cdecl(h, LIMIT_INIT, &[])
    }
    /// `code_008d55d0()`: `jmp 0x008d6be0` (the customize screen's init).
    pub fn kaizou_init_thunk(h: &mut impl Host) -> u32 {
        cdecl(h, KAIZOU_INIT, &[])
    }
    /// `TkMenuDrawScissorRect(x, y, w, h)`: `jmp TODrawScissorXYWH`.
    pub fn menu_draw_scissor_rect(h: &mut impl Host, x: u32, y: u32, w: u32, hh: u32) -> u32 {
        h5::draw_scissor_rect_thunk(h, x, y, w, hh)
    }
    /// `code_008dde90()` (`g_TkVibrateCurves + 0xc14`): the image address
    /// `0x00b5f46c`.
    pub fn vibrate_curve_008dde90(h: &impl Host) -> u32 {
        h5::vibrate_curve_008a9850(h)
    }

    /// `code_008cb350()` (`g_MenuHelpExtArg + 0x564`, the config screen's
    /// open): 8 rows of the row list for the language
    /// (`TOGetFFXLangJmp`, 0 Japanese) and the layout word
    /// (`GetU32_023cbeb0() == 1`), then a tail jump to `0x008cc050`. The
    /// other languages' path stores the first list before the layout test,
    /// as the original does.
    pub fn config_open(h: &mut impl Host) -> u32 {
        let lang = cdecl(h, TO_GET_FFX_LANG_JMP, &[]);
        let layout = cdecl(h, GET_U32_023CBEB0, &[]);
        CONFIG_ROWS.set(h, 8);
        let list = if lang == 0 {
            if layout == 1 {
                CONFIG_LIST_JP_1
            } else {
                CONFIG_LIST_JP
            }
        } else if layout == 1 {
            CONFIG_LIST_1
        } else {
            CONFIG_LIST
        };
        let list = h.rebase(list);
        CONFIG_ROW_LIST.set(h, list);
        cdecl(h, CONFIG_OPEN, &[])
    }

    /// `code_008cc300()` (`g_NkzAeonStats + 0x12c`): past the first
    /// restart (`g_TkMenuSummonRestartCount > 1`) the sub-command window
    /// closes; then a tail jump to `TkMenuSummonCloseWindow`.
    pub fn summon_close(h: &mut impl Host) -> u32 {
        if SUMMON_RESTART_COUNT.get(h) > 1 {
            cdecl(h, h5::TK_MENU_CLOSE_SUB_CMD_WINDOW, &[]);
        }
        cdecl(h, TK_MENU_SUMMON_CLOSE_WINDOW, &[])
    }

    /// `code_008cf910()` (`g_NkzAeonStats + 0x150`): the equipment
    /// screen's three scenes stopped (`TkMn2SeqStopScene`); returns the
    /// last one's `eax`.
    pub fn weapon_stop_scenes(h: &mut impl Host) -> u32 {
        let mut r = 0;
        for name in WEAPON_SCENES {
            let name = h.rebase(name);
            r = cdecl(h, TK_MN2_SEQ_STOP_SCENE, &[name]);
        }
        r
    }

    /// `code_008d09b0()` (`g_NkzAeonStats + 0x168`, the limit screen's
    /// draw): the prologue, `TODrawMenuBG`, the dividers
    /// (`MenuDrawDividerShapes(5)`), the current player's status plate at
    /// (`remapX2(f32 0x00b5ef1c)`, `remapY2(f32 0x00b65af8)`),
    /// `DrawLimitPageOverDrive(player)`, then a tail jump to
    /// `TkMn2DrawKickSyncPacket`.
    pub fn limit_draw(h: &mut impl Host) -> u32 {
        let who = cdecl(h, h5::TK_MENU_GET_CURRENT_PLAYER, &[]);
        cdecl(h, h5::TK_MENU_DRAW_KICK, &[]);
        cdecl(h, h5::TK_MENU_DRAW_KICK, &[]);
        cdecl(h, h5::TK_VU1_SYNC_PATH_THUNK, &[]);
        cdecl(h, h5::TK_MN2_DRAW_INIT_PACKET, &[]);
        cdecl(h, h5::TO_DRAW_MENU_BG, &[]);
        cdecl(h, MENU_DRAW_DIVIDER_SHAPES, &[5]);
        h5::status_plate(h, who, LIMIT_PLATE_Y);
        cdecl(h, DRAW_LIMIT_PAGE_OVER_DRIVE, &[who]);
        cdecl(h, h5::TK_MN2_DRAW_KICK_SYNC_PACKET, &[])
    }

    /// `code_008d0c70()` (`g_NkzAeonStats + 0x174`): the sub-command and
    /// the limit screen's two windows closed (a tail jump to the second).
    pub fn limit_close(h: &mut impl Host) -> u32 {
        cdecl(h, h5::TK_MENU_CLOSE_SUB_CMD_WINDOW, &[]);
        cdecl(h, TK_MENU_LIMIT_CLOSE_WINDOW, &[]);
        cdecl(h, TK_MENU_LIMIT_CLOSE_WINDOW2, &[])
    }

    /// `code_008d1d30()` (`g_TkMenuStatusStats + 0x50`): the six status
    /// windows forgotten (`0x0186a820..0x0186a834` = 0), then a tail jump
    /// to `TkMn2GetStatusPos`.
    pub fn status_open(h: &mut impl Host) -> u32 {
        for i in 0..6 {
            let at = h.rebase(STATUS_WINDOWS + 4 * i);
            h.write(at, 0u32);
        }
        cdecl(h, TK_MN2_GET_STATUS_POS, &[])
    }

    /// `code_008d1dd0()` (`g_TkMenuStatusStats + 0x58`): the six status
    /// windows closed (`MenuStatus_CloseWindow(0..6)`), a tail jump to
    /// `TkMn2InitStatusBG`.
    pub fn status_close(h: &mut impl Host) -> u32 {
        for i in 0..6 {
            cdecl(h, MENU_STATUS_CLOSE_WINDOW, &[i]);
        }
        cdecl(h, TK_MN2_INIT_STATUS_BG, &[])
    }

    /// `code_008d55c0()` (`g_TkMenuStatusStats + 0x70`): `TkVU1SyncPath`
    /// (thunk), `TkMn2DrawInitPacket`, a tail jump to
    /// `TkMn2DrawKickSyncPacket`.
    pub fn draw_sync_only(h: &mut impl Host) -> u32 {
        cdecl(h, h5::TK_VU1_SYNC_PATH_THUNK, &[]);
        cdecl(h, h5::TK_MN2_DRAW_INIT_PACKET, &[]);
        cdecl(h, h5::TK_MN2_DRAW_KICK_SYNC_PACKET, &[])
    }

    /// `code_008d5650()` (`g_TkMenuStatusStats + 0x7c`, the customize
    /// screen's exit): `TkMsSetSaveParamAll`, `TkMenuKaizouCloseWindow`,
    /// `AtelRegistCallbackQue2(0, TkMn2ArmsRefreshArms, 0x00b65e28)`;
    /// returns its `eax`.
    pub fn kaizou_close(h: &mut impl Host) -> u32 {
        cdecl(h, TK_MS_SET_SAVE_PARAM_ALL, &[]);
        cdecl(h, TK_MENU_KAIZOU_CLOSE_WINDOW, &[]);
        let f = h.rebase(TK_MN2_ARMS_REFRESH_ARMS);
        let arg = h.rebase(KAIZOU_CALLBACK_ARG);
        cdecl(h, ATEL_REGIST_CALLBACK_QUE2, &[0, f, arg])
    }

    /// `code_008d7580()`, `code_008d9ac0()`: `TkMn2GetIShopPos`,
    /// `TkMn2GetWShopPos`, a tail jump to `TkMn2GetStatusPos`.
    pub fn shop_get_pos(h: &mut impl Host) -> u32 {
        cdecl(h, h5::TK_MN2_GET_ISHOP_POS, &[]);
        cdecl(h, h5::TK_MN2_GET_WSHOP_POS, &[]);
        cdecl(h, TK_MN2_GET_STATUS_POS, &[])
    }

    /// `code_008d75b0()`: `MenuWShop_CloseSubWindow4`, a tail jump to
    /// `TkMsSetSaveParamAll`.
    pub fn wshop_close(h: &mut impl Host) -> u32 {
        cdecl(h, MENU_WSHOP_CLOSE_SUB_WINDOW4, &[]);
        cdecl(h, TK_MS_SET_SAVE_PARAM_ALL, &[])
    }

    /// `code_008d9ae0()`: `TkMenuWShopCloseWindow3`, a tail jump to
    /// `TkMenuCloseSubCmdWindow`.
    pub fn wshop_close3(h: &mut impl Host) -> u32 {
        cdecl(h, TK_MENU_WSHOP_CLOSE_WINDOW3, &[]);
        cdecl(h, h5::TK_MENU_CLOSE_SUB_CMD_WINDOW, &[])
    }

    /// `code_008db7d0()`: `g_TkMenuTutoChosen` = 0.
    pub fn tuto_clear(h: &mut impl Host) {
        TUTO_CHOSEN.set(h, 0);
    }

    // ---- the cursor and list-row drawers (x87) ----

    pub const TK_MN2_DRAW_CROSS_CMD_CURSOR: u32 = 0x008c_0580;
    pub const TK_MN2_DRAW_CROSS_CURSOR: u32 = 0x008c_0670;
    pub const TO_DRAW_MENU_PLATE_XYWH_TYPE: u32 = 0x008f_5f70;
    pub const TO_DRAW_CROSS_MENU_ICON_COMMAND_NAME: u32 = 0x008f_5000;
    pub const TO_DRAW_CROSS_MENU_ICON_ABILITY_NAME: u32 = 0x008f_4f10;
    pub const SUMMON_ROW_DRAW: u32 = 0x008c_dab0;
    pub const KAIZOU_ROW_DRAW: u32 = 0x008d_5e60;
    /// The aeon/customize list's rows: `u16` id at `+0`, `u8` kind at
    /// `+2`, `u8` at `+3` (0xff: empty), four bytes a row.
    pub const ROW_TABLE: u32 = 0x0159_7770;
    /// `f64` 1/4096 the scroll offset is scaled by.
    pub const SCROLL_SCALE: u32 = 0x00b5_0190;

    /// `remapY2(f32 [sym])` as the original keeps it in `ST0`.
    fn remap_y(h: &mut impl Host, sym: u32) -> f64 {
        let v: u32 = h.read(h.rebase(sym));
        h.call_sym_st0_f64(h5::GRAPHIC_UI_REMAP_Y2, Conv::Cdecl, &[v])
    }

    /// `remapX2(f32 [sym])` as the original keeps it in `ST0`.
    fn remap_x(h: &mut impl Host, sym: u32) -> f64 {
        let v: u32 = h.read(h.rebase(sym));
        h.call_sym_st0_f64(h5::GRAPHIC_UI_REMAP_X2, Conv::Cdecl, &[v])
    }

    /// An `i32` through `fild` / `fstp dword` (rounded to `f32`).
    fn int_f32(v: u32) -> f64 {
        f64::from(v.cast_signed() as f32)
    }

    /// A window's scroll (`i16 +0x48 - i16 +0x32`) as `f32`.
    fn scroll(h: &impl Host, w: u32) -> f64 {
        let a = i32::from(h.read::<i16>(w.wrapping_add(0x48)));
        let b = i32::from(h.read::<i16>(w.wrapping_add(0x32)));
        int_f32(a.wrapping_sub(b).cast_unsigned())
    }

    /// `(w, x, y)` cursor drawers `0x008d5df0` and `0x008d65f0`:
    /// `TkMn2DrawCrossCmdCursor(f32(x), f32(ftol(r * scroll + y)), 0)`
    /// with `r = remapY2(f32 0x00b5eec8)`. The original keeps its scratch
    /// in its own argument words (no caller reads them).
    pub fn row_cursor(h: &mut impl Host, w: u32, x: u32, y: u32) -> u32 {
        let c = remap_y(h, 0x00b5_eec8);
        let d = scroll(h, w);
        let q = x87::add(x87::mul(c, d), f64::from(y.cast_signed()));
        let z = x87::ftol(h, q);
        let args = [x87::store(int_f32(x)), x87::store(int_f32(z)), 0];
        cdecl(h, TK_MN2_DRAW_CROSS_CMD_CURSOR, &args)
    }

    /// `FUN_008c9940(w, x, y)`: `X = ftol(remapX2(f32 0x00b65b10) +
    /// f32(x))`, `Y = ftol(remapY2(f32 0x00b5f394) + f32(y))`, `Z =
    /// ftol(remapY2(f32 0x00b5eecc) * scroll + Y)`;
    /// `TkMn2DrawCrossCursor(f32(X), f32(Z), 0)`.
    pub fn list_cursor(h: &mut impl Host, w: u32, x: u32, y: u32) -> u32 {
        let a = remap_x(h, 0x00b6_5b10);
        let xx = x87::ftol(h, x87::add(a, int_f32(x)));
        let b = remap_y(h, 0x00b5_f394);
        let yy = x87::ftol(h, x87::add(b, int_f32(y)));
        let c = remap_y(h, 0x00b5_eecc);
        let d = scroll(h, w);
        let z = x87::ftol(h, x87::add(x87::mul(c, d), f64::from(yy.cast_signed())));
        let args = [x87::store(int_f32(xx)), x87::store(int_f32(z)), 0];
        cdecl(h, TK_MN2_DRAW_CROSS_CURSOR, &args)
    }

    /// `FUN_008cda20(w)` (the summon grow window's cursor; the caller's
    /// two other words are only overwritten): `X = ftol(remapX2(f32
    /// 0x00b515d0))`, `Z = ftol(remapY2(f32 0x00b65d8c) + remapY2(f32
    /// 0x00b5f314) * scroll)`; `TkMn2DrawCrossCmdCursor(f32(X), f32(Z),
    /// 0)`.
    pub fn summon_cursor(h: &mut impl Host, w: u32) -> u32 {
        let a = remap_x(h, 0x00b5_15d0);
        let xx = x87::ftol(h, a);
        let c = remap_y(h, 0x00b5_f314);
        let e = x87::mul(c, scroll(h, w));
        let r = remap_y(h, 0x00b6_5d8c);
        let z = x87::ftol(h, x87::add(r, e));
        let args = [x87::store(int_f32(xx)), x87::store(int_f32(z)), 0];
        cdecl(h, TK_MN2_DRAW_CROSS_CMD_CURSOR, &args)
    }

    /// Which list a row drawer serves.
    #[derive(Clone, Copy)]
    pub enum Rows {
        /// `0x008cdab0` (summon): plate kind 0xe, the icon when byte `+2`
        /// is set, `TODrawCrossMenuIconCommandName`, flag `+2 != 4`.
        Summon,
        /// `0x008d5e60` (customize): plate kind 7, the icon unless byte
        /// `+3` is 0xff, `TODrawCrossMenuIconAbilityName`, flag `+2 != 0xb`.
        Kaizou,
    }

    /// The row drawers `(w, x, y, row, fixed)`: unless `fixed`, `y -=
    /// remapY2(f32 c) * i16 w+0x46 * (f64 0x00b50190)` (stored as `f32`);
    /// a row in `0..u16 w+0x30` gets its plate
    /// (`TODrawMenuPlateXYWHType(x, y, f32 remapX2(f32 0x00b5eff4), f32 remapY2(f32
    /// p), kind)`) and, by the table at `0x01597770`, its icon at
    /// `(x, f32(y - remapY2(f32 0x00b92268)))`.
    pub fn row_draw(h: &mut impl Host, which: Rows, w: u32, x: u32, y: u32, row: u32, fixed: u32) {
        let (c, p, kind) = match which {
            Rows::Summon => (0x00b5_f314, 0x00b5_eea4, 0xe),
            Rows::Kaizou => (0x00b5_eec8, 0x00b3_eb9c, 7),
        };
        let mut y = y;
        if fixed == 0 {
            let base = x87::load(y);
            let r = remap_y(h, c);
            let k = f64::from(h.read::<i16>(w.wrapping_add(0x46)));
            let scale: f64 = h.read(h.rebase(SCROLL_SCALE));
            let t = x87::mul(x87::mul(r, k), scale);
            y = x87::store(x87::sub(base, t));
        }
        let rows = i32::from(h.read::<u16>(w.wrapping_add(0x30)));
        let r = row.cast_signed();
        if r < 0 || r >= rows {
            return;
        }
        let py = x87::store(remap_y(h, p));
        let px = x87::store(remap_x(h, 0x00b5_eff4));
        let xr = x87::store(x87::load(x));
        let yr = x87::store(x87::load(y));
        cdecl(h, TO_DRAW_MENU_PLATE_XYWH_TYPE, &[xr, yr, px, py, kind]);
        let entry = h.rebase(ROW_TABLE).wrapping_add(row.wrapping_mul(4));
        let b2: u8 = h.read(entry.wrapping_add(2));
        let (draw, flag, icon) = match which {
            Rows::Summon => (
                b2 != 0,
                u32::from(b2 != 4),
                TO_DRAW_CROSS_MENU_ICON_COMMAND_NAME,
            ),
            Rows::Kaizou => {
                let b3: u8 = h.read(entry.wrapping_add(3));
                (
                    b3 != 0xff,
                    u32::from(b2 != 0xb),
                    TO_DRAW_CROSS_MENU_ICON_ABILITY_NAME,
                )
            }
        };
        if !draw {
            return;
        }
        let base = x87::load(y);
        let r = remap_y(h, 0x00b9_2268);
        let v = x87::store(x87::sub(base, r));
        let id = u32::from(h.read::<u16>(entry));
        cdecl(h, icon, &[id, xr, v, flag]);
    }

    /// The list drawers `(w, a, b, dx, dy)` over eleven rows `b - 1 ..
    /// b + 9`: `x = f32(remapX2(f32 cx) + dx)`, `y = f32(remapY2(f32 cy) +
    /// dy)`, each row drawn at `(x, y)` (`row_draw(w, x, y, b + i, a)`),
    /// then `y = f32(remapY2(f32 step) + y)`. The summon list takes `dx`
    /// and `dy` as `f32`, the customize list as integers (`fild`, rounded
    /// to `f32`).
    pub fn list_draw(h: &mut impl Host, which: Rows, w: u32, a: u32, b: u32, dx: u32, dy: u32) {
        let (cx, cy, step, sub) = match which {
            Rows::Summon => (0x00b5_ef1c, 0x00b4_bf08, 0x00b5_f314, SUMMON_ROW_DRAW),
            Rows::Kaizou => (0x00b6_6364, 0x00b6_5890, 0x00b5_eec8, KAIZOU_ROW_DRAW),
        };
        let (dx, dy) = match which {
            Rows::Summon => (x87::load(dx), x87::load(dy)),
            Rows::Kaizou => (int_f32(dx), int_f32(dy)),
        };
        let rx = remap_x(h, cx);
        let x = x87::store(x87::add(rx, dx));
        let ry = remap_y(h, cy);
        let mut y = x87::store(x87::add(ry, dy));
        for i in -1i32..10 {
            let row = b.wrapping_add(i.cast_unsigned());
            cdecl(h, sub, &[w, x, y, row, a]);
            let r = remap_y(h, step);
            y = x87::store(x87::add(r, x87::load(y)));
        }
    }
}
