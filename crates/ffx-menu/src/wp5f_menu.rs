//! M5 finish, WP5 part 5 (wp5f): the menu functions ported code still
//! called in the original: the config, equipment, limit and customize
//! screens' layout reads (reached by the `m5m_i` jump thunks and the
//! config open step's tail jump) and the main menu's help bar. None is in
//! the dump; each was read from the machine code.

#[allow(clippy::many_single_char_names, clippy::similar_names)] // the operands, named by their constants
pub mod bridge {
    use ffx_bridge::Global;
    use ffx_graphic::host::{Conv, Host};

    pub const GET_U32_023CBEB0: u32 = 0x00a7_ae90;
    pub const TK_MN2_GET_COMMON_PLATE: u32 = 0x008c_1a60;
    pub const TK_MN2_SEQ_GET_NUM: u32 = 0x008c_2b10;
    pub const TK_MN2_SEQ_GET_RECT: u32 = 0x008c_2b90;
    pub const TK_MN2_SEQ_GET_POS: u32 = 0x008c_2b50;
    pub const TK_MN2_SEQ_GET_COLOR: u32 = 0x008c_2ad0;

    pub const CONFIG_INIT_LAYOUT: u32 = 0x008c_c050;
    pub const EQUIP_INIT_LAYOUT: u32 = 0x008c_fc40;
    pub const LIMIT_INIT_LAYOUT: u32 = 0x008d_0cd0;
    pub const KAIZOU_INIT_LAYOUT: u32 = 0x008d_6be0;
    pub const DRAW_HELP_BAR: u32 = 0x008a_9350;

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `TkMn2SeqGetRect(name, out)` with both image addresses.
    fn rect(h: &mut impl Host, name: u32, out: u32) -> u32 {
        let (name, out) = (h.rebase(name), h.rebase(out));
        cdecl(h, TK_MN2_SEQ_GET_RECT, &[name, out])
    }

    /// `TkMn2SeqGetPos(name, out)` with both image addresses.
    fn pos(h: &mut impl Host, name: u32, out: u32) -> u32 {
        let (name, out) = (h.rebase(name), h.rebase(out));
        cdecl(h, TK_MN2_SEQ_GET_POS, &[name, out])
    }

    /// `*out = TkMn2SeqGetColor(name)`; the colour.
    fn color(h: &mut impl Host, name: u32, out: u32) -> u32 {
        let name = h.rebase(name);
        let c = cdecl(h, TK_MN2_SEQ_GET_COLOR, &[name]);
        h.write(h.rebase(out), c);
        c
    }

    /// `TkMn2GetCommonPlate(plate, rect, c1, c2, a, b)`, the four
    /// addresses in the image.
    fn plate(h: &mut impl Host, at: [u32; 4], a: u32, b: u32) -> u32 {
        let [p, r, c1, c2] = at.map(|x| h.rebase(x));
        cdecl(h, TK_MN2_GET_COMMON_PLATE, &[p, r, c1, c2, a, b])
    }

    /// `MenuConfig_InitLayout_008cc050()`: the config screen's layout.
    /// The name plate `r_cnameplt` (or its `_nonhdd` twin when
    /// `GetU32_023cbeb0() == 1`) with the choice plate colours and shapes
    /// 0xcc/0xcd into `0x0186a450`, the row count `0x0186a48c` (its
    /// layout's number), three rects, three positions and four colours.
    /// Returns the last colour (the original's `eax`).
    pub fn config_init_layout(h: &mut impl Host) -> u32 {
        let alt = cdecl(h, GET_U32_023CBEB0, &[]) == 1;
        let (rect_name, num) = if alt {
            (0x00b6_5b50, 0x00b6_5b5c)
        } else {
            (0x00b6_5b6c, 0x00b6_5b80)
        };
        plate(
            h,
            [0x0186_a450, rect_name, 0x00b6_2abc, 0x00b6_2ac8],
            0xcc,
            0xcd,
        );
        let num = h.rebase(num);
        let n = cdecl(h, TK_MN2_SEQ_GET_NUM, &[num]);
        h.write(h.rebase(0x0186_a48c), n);
        rect(h, 0x00b6_5b94, 0x0186_a468);
        rect(h, 0x00b6_5ba4, 0x0186_a474);
        rect(h, 0x00b6_2ae0, 0x0186_a490);
        pos(h, 0x00b6_5bb0, 0x0186_a470);
        pos(h, 0x00b6_2ad4, 0x0186_a480);
        pos(h, 0x00b6_5bbc, 0x0186_a47c);
        color(h, 0x00b6_5bc8, 0x0186_a484);
        color(h, 0x00b6_5bd4, 0x0186_a488);
        color(h, 0x00b6_29b4, 0x0186_a498);
        color(h, 0x00b6_29c4, 0x0186_a49c)
    }

    /// `MenuEquip_InitLayout_008cfc40()`: the equipment screen's layout
    /// (`g_TkMenuEquipLayout` `0x0186a634`): three rects and two
    /// positions. Returns the last call's `eax`.
    pub fn equip_init_layout(h: &mut impl Host) -> u32 {
        rect(h, 0x00b6_5db8, 0x0186_a634);
        pos(h, 0x00b6_5dc4, 0x0186_a63c);
        rect(h, 0x00b6_5dd0, 0x0186_a640);
        pos(h, 0x00b6_5de0, 0x0186_a648);
        rect(h, 0x00b6_5dec, 0x0186_a64c)
    }

    /// `MenuLimit_InitLayout_008d0cd0()`: the Overdrive screen's layout
    /// (`g_TkMenuLimitLayout` `0x0186a6d8`): a position, a rect, the
    /// common plate `rect_lwplt` (`a`, `b` 0), two positions and two
    /// rects. Returns the last call's `eax`.
    pub fn limit_init_layout(h: &mut impl Host) -> u32 {
        pos(h, 0x00b6_5e60, 0x0186_a6d8);
        rect(h, 0x00b6_5e70, 0x0186_a6dc);
        plate(
            h,
            [0x0186_a6e4, 0x00b6_5e94, 0x00b6_5e88, 0x00b6_5e7c],
            0,
            0,
        );
        pos(h, 0x00b6_5ea0, 0x0186_a6fc);
        pos(h, 0x00b6_5eac, 0x0186_a700);
        rect(h, 0x00b6_5eb8, 0x0186_a704);
        rect(h, 0x00b6_5ec8, 0x0186_a70c)
    }

    /// `MenuKaizou_InitLayout_008d6be0()`: the customize screen's layout
    /// (`g_TkMenuKaizouLayout` `0x0186aa40` and the plates at
    /// `0x023cc364..`): the common plate `r_kbackplt` into `0x023cc3a0`
    /// (`a`, `b` 0), then rects and positions in the original's order.
    /// Returns the last call's `eax`.
    pub fn kaizou_init_layout(h: &mut impl Host) -> u32 {
        plate(
            h,
            [0x023c_c3a0, 0x00b6_625c, 0x00b6_5c34, 0x00b6_5c24],
            0,
            0,
        );
        pos(h, 0x00b6_6268, 0x0186_aa40);
        rect(h, 0x00b6_6278, 0x0186_aa44);
        rect(h, 0x00b6_6284, 0x0186_aa4c);
        pos(h, 0x00b6_6290, 0x0186_aa54);
        pos(h, 0x00b6_629c, 0x0186_aa58);
        rect(h, 0x00b6_62ac, 0x0186_aa5c);
        rect(h, 0x00b6_62b8, 0x023c_c370);
        pos(h, 0x00b6_62c4, 0x023c_c3b8);
        pos(h, 0x00b6_62d4, 0x023c_c388);
        rect(h, 0x00b6_62e0, 0x023c_c380);
        pos(h, 0x00b6_62ec, 0x023c_c378);
        pos(h, 0x00b6_62f8, 0x023c_c368);
        pos(h, 0x00b6_6304, 0x023c_c364);
        pos(h, 0x00b6_6310, 0x0186_aa64)
    }

    pub const TK_MENU_MAIN_GET_MENU_ACTIVE_MASK: u32 = 0x008a_a460;
    pub const TK_VU1_SYNC_PATH: u32 = 0x0088_ec60;
    pub const TO_MENU_OPEN_PKT_BUFF_TMP: u32 = 0x008b_ef30;
    pub const MENU_TUTO_CHOICE_IS5: u32 = 0x008d_b860;
    pub const TO_MENU_DRAW_KICK_TMP: u32 = 0x008b_ea20;
    pub const TO_MKP_SHAPE_XYWHUVC2: u32 = 0x0090_3ee0;
    pub const TO_MKP_SHAPE_XYWHUV: u32 = 0x0090_3bb0;
    pub const TO_MKP_CROSS_EXT_MES_FONT_LCLUT: u32 = 0x0090_16b0;
    pub const GRAPHIC_UI_REMAP_X2: u32 = 0x0064_47e0;
    pub const GRAPHIC_UI_REMAP_Y2: u32 = 0x0064_4820;

    /// `g_TkMenuHelpMessage` (the current message), `g_TkMenuMode800003`,
    /// and the draw callbacks (`+0x10`) of `g_TkMenuWindows`' windows 1
    /// and 2 (0x98 bytes each).
    pub const HELP_MESSAGE: Global<u32> = Global::new(0x0184_1c00);
    pub const MODE_800003: Global<u32> = Global::new(0x0134_0854);
    pub const WINDOW1_DRAW: Global<u32> = Global::new(0x0184_09a8);
    pub const WINDOW2_DRAW: Global<u32> = Global::new(0x0184_0a40);

    /// An `f32` constant of the image, as its bits.
    fn konst(h: &impl Host, at: u32) -> u32 {
        h.read(h.rebase(at))
    }

    /// `graphicUiRemapX2`/`Y2` of the image constant at `at`: the result
    /// stored as `f32` (`fstp dword`).
    fn remap(h: &mut impl Host, sym: u32, at: u32) -> u32 {
        let v = konst(h, at);
        h.call_sym_st0(sym, Conv::Cdecl, &[v]).to_bits()
    }

    /// `TkMenuMain_DrawHelpBar_008a9350(window)` (cdecl; the window is not
    /// read): nothing while `TkMenuMainGetMenuActiveMask()` has bit 11 or
    /// 19 (its value returned). Else in a temporary packet buffer
    /// (`TkVU1SyncPath`, `TOMenuOpenPktBuffTmp` .. `TOMenuDrawKickTmp`,
    /// then a tail jump to `TkVU1SyncPath`, whose `eax` is returned): with
    /// a help message, the bar's two HD plates (`TOMkpShapeXYWHUVC2`
    /// shape -1 in grey, `TOMkpShapeXYWHUV` shape 0x3e; positions and
    /// sizes through `graphicUiRemapY2`/`X2` in the original's order)
    /// unless mode 0x800003 runs with window 1 closed, or the tutorial
    /// choice is 5 with windows 1 and 2 closed (`MenuTuto_ChoiceIs5`
    /// asked once more then); then the message
    /// (`TOMkpCrossExtMesFontLClut(0, msg, x, y, 0, size, 1.0)`).
    pub fn draw_help_bar(h: &mut impl Host) -> u32 {
        let mask = cdecl(h, TK_MENU_MAIN_GET_MENU_ACTIVE_MASK, &[]);
        if mask & 0x8_0800 != 0 {
            return mask;
        }
        cdecl(h, TK_VU1_SYNC_PATH, &[]);
        cdecl(h, TO_MENU_OPEN_PKT_BUFF_TMP, &[]);
        let msg = HELP_MESSAGE.get(h);
        let plates = if MODE_800003.get(h) != 0 && WINDOW1_DRAW.get(h) == 0 {
            false
        } else if cdecl(h, MENU_TUTO_CHOICE_IS5, &[]) != 0
            && (WINDOW2_DRAW.get(h) != 0 || WINDOW1_DRAW.get(h) != 0)
        {
            true
        } else {
            cdecl(h, MENU_TUTO_CHOICE_IS5, &[]) == 0
        };
        if msg != 0 {
            if plates {
                draw_plates(h);
            }
            let size = konst(h, 0x00b5_ee10);
            let y = remap(h, GRAPHIC_UI_REMAP_Y2, 0x00b5_eee4);
            let x = remap(h, GRAPHIC_UI_REMAP_X2, 0x00b5_ef2c);
            cdecl(
                h,
                TO_MKP_CROSS_EXT_MES_FONT_LCLUT,
                &[0, msg, x, y, 0, size, 1.0f32.to_bits()],
            );
        }
        cdecl(h, TO_MENU_DRAW_KICK_TMP, &[]);
        cdecl(h, TK_VU1_SYNC_PATH, &[])
    }

    /// The help bar's two plates (`TkMenuMain_DrawHelpBar`): the remaps
    /// run in the original's order (each argument's slot overwritten by
    /// its result).
    fn draw_plates(h: &mut impl Host) {
        let a = remap(h, GRAPHIC_UI_REMAP_Y2, 0x00b5_eea8);
        let b = remap(h, GRAPHIC_UI_REMAP_X2, 0x00b5_f030);
        let c = remap(h, GRAPHIC_UI_REMAP_Y2, 0x00b4_3db8);
        let d = remap(h, GRAPHIC_UI_REMAP_X2, 0x00b5_eef0);
        let args = [
            u32::MAX,
            d,
            c,
            b,
            a,
            0,
            konst(h, 0x00b5_efb0),
            konst(h, 0x00b5_f030),
            konst(h, 0x00b5_efcc),
            0x8080_8080,
            0x0080_8080,
        ];
        cdecl(h, TO_MKP_SHAPE_XYWHUVC2, &args);
        let ee64 = remap(h, GRAPHIC_UI_REMAP_Y2, 0x00b5_ee64);
        let eedc = remap(h, GRAPHIC_UI_REMAP_X2, 0x00b5_eedc);
        let eed0 = remap(h, GRAPHIC_UI_REMAP_Y2, 0x00b5_eed0);
        let eef0 = remap(h, GRAPHIC_UI_REMAP_X2, 0x00b5_eef0);
        let args = [
            0x3e,
            eef0,
            eed0,
            eedc,
            ee64,
            konst(h, 0x00b5_edf4),
            konst(h, 0x00b5_ee20),
            konst(h, 0x00b5_ee0c),
            konst(h, 0x00b5_ee2c),
        ];
        cdecl(h, TO_MKP_SHAPE_XYWHUV, &args);
    }
}
