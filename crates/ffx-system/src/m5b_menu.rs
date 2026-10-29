//! M5 `system-b` (rounds 2-3): small pieces of the menu layer (`Tk*`/`TO*`):
//! the `TOMkp*` packet wrappers around `g_TOPktCursor`, and the one-jump
//! and short entries of the menu tables (not in the dump).

/// `g_TOPktCursor`. The wrappers end in a tail jump to `TOCtrlPkt`
/// (`0x008f3a30`), whose head the bridge runs itself (its port is void,
/// so a call would lose the `eax` the wrappers return): while packets are
/// added (`0x00c5d780`) and the cursor has reached the limit
/// (`g_TOPktBuff[2]`, `0x018716c8`, unsigned), a tail jump to
/// `TODrawKick` (`0x008f5bc0`, cdecl).
pub const PKT_CURSOR: u32 = 0x0187_16cc;
pub const ADD_PKT: u32 = 0x00c5_d780;
pub const PKT_LIMIT: u32 = 0x0187_16c8;
pub const DRAW_KICK: u32 = 0x008f_5bc0;
/// The wrappers' workers (cdecl, each returns the new cursor):
/// `0x008f3280`, `TOAdpEasyShapeXYWHC2`, `TOAddPktBaseMesFont_008ed0a0`,
/// `0x008ed120`, `0x008edfc0`, `0x008f2bf0`; `TOBtlDrawChangeSeqAnm0`,
/// `TOTransImageHL`; `0x008edfc0`'s `Identity_008f2f70` and per-character
/// `TOAdpFontM_ASCII`.
pub const VALUE_FONT_M: u32 = 0x008f_3280;
pub const EASY_SHAPE: u32 = 0x008e_d380;
pub const BASE_MES_FONT: u32 = 0x008e_d0a0;
pub const MES_FONT_8ED120: u32 = 0x008e_d120;
pub const STR_FONT_M: u32 = 0x008e_dfc0;
pub const FONT_8F2BF0: u32 = 0x008f_2bf0;
pub const CHANGE_SEQ_ANM: u32 = 0x0089_28b0;
pub const TRANS_IMAGE_HL: u32 = 0x008b_0b10;
pub const IDENTITY: u32 = 0x008f_2f70;
pub const FONT_M_ASCII: u32 = 0x008e_e240;
/// `TOMakePktSeqPrimMes`'s format string.
pub const SEQ_MES_FORMAT: u32 = 0x00b6_6b50;

/// Constant returns: two strings in `.rdata`.
pub const STR_B62CF4: u32 = 0x00b6_2cf4;
pub const STR_B66B30: u32 = 0x00b6_6b30;

/// Table-entry callees (cdecl): `TkCdSizeKBCached_008741d0` into
/// `g_TkMenuSaveNeed`, `TkMcSetDebugLock`, `FUN_008eb4e0` (the packet
/// worker of `0x008fd500`), the two pointers `0x00c416a8`/`0x00c416b4`
/// and the words `0x0186a22c` (u16)/`0x0186a230` cleared before the
/// first.
pub const CD_SIZE_KB: u32 = 0x0087_41d0;
pub const SAVE_NEED: u32 = 0x0186_63c0;
pub const MC_SET_DEBUG_LOCK: u32 = 0x0088_8420;
pub const PKT_8EB4E0: u32 = 0x008e_b4e0;
pub const HDD_MENU_ENTER: u32 = 0x00c4_16a8;
pub const HDD_MENU_FUNC: u32 = 0x00c4_16b4;
pub const WORD_186A22C: u32 = 0x0186_a22c;
pub const WORD_186A230: u32 = 0x0186_a230;
/// `TkMenuItemCloseWindow`, `...3`, `...2`, `TkMenuCloseSubCmdWindow`,
/// `TOSeqStopScene(1, 0x00b65aa4)`.
pub const CLOSE_WINDOW: u32 = 0x008c_6b60;
pub const CLOSE_WINDOW3: u32 = 0x008c_6b40;
pub const CLOSE_WINDOW2: u32 = 0x008c_6b80;
pub const CLOSE_SUB_CMD: u32 = 0x008e_36f0;
pub const SEQ_STOP_SCENE: u32 = 0x008a_f4f0;
pub const STOP_SCENE_NAME: u32 = 0x00b6_5aa4;
pub const ITEM_LIST_MAX: u32 = 0x008c_8c30;
/// The option menu's: `TkMenuDrawShadeRatioDiff`,
/// `TkMenuSetDrawShadeRatioStep`, `TkMenuMainGetOptionMenu`,
/// `TkMenuMainHelp`, `TkMenuSetHelpMessage`, `TkMenuSetScreenX2Flag`,
/// `TkMenuMainSetIgnoreJoinFlag`, `TOSeqInitScene`, `TOSeqStopExecScene`;
/// its words `0x01871540` (a window), `0x01871544` (u16), `0x01871548`,
/// `0x01871554`, `0x0187155c`, `0x01871560`, and `0x0134085c`.
pub const SHADE_RATIO_DIFF: u32 = 0x008a_9670;
pub const SHADE_RATIO_STEP: u32 = 0x008a_9680;
pub const OPTION_MENU: u32 = 0x008e_2270;
pub const MAIN_HELP: u32 = 0x008d_df60;
pub const SET_HELP: u32 = 0x008a_aeb0;
pub const SCREEN_X2: u32 = 0x008a_b020;
pub const IGNORE_JOIN: u32 = 0x008a_ad60;
pub const SEQ_INIT_SCENE: u32 = 0x008a_eec0;
pub const SEQ_STOP_EXEC: u32 = 0x008a_f380;
pub const OPT_WINDOW: u32 = 0x0187_1540;
pub const OPT_WORD_44: u32 = 0x0187_1544;
pub const OPT_ITEM: u32 = 0x0187_1548;
pub const OPT_WORD_54: u32 = 0x0187_1554;
pub const OPT_WORD_5C: u32 = 0x0187_155c;
pub const OPT_WORD_60: u32 = 0x0187_1560;
pub const WORD_134085C: u32 = 0x0134_085c;
/// `eiAbmExit` and the byte `0x01340858`; the sphere grid's enter and
/// leave (`0x00a56030`, `0x00a54760`, ported inline): `op_et_chr_show`,
/// `op_tex_vdisp_load_triger`, `pppInitData` (cdecl), `g_AbmapRunning`,
/// the leave's once-flag `0x01a85f64`, the record pointer `0x02305800`
/// and the data `0x00c86080`; `TOMakePktFont1612Edge` (2 words);
/// `dbgPrintf` with `0x00b53060`; `TOGetFFXLangJmp`; the object
/// `0x01865f04` whose `+8` is a cdecl function.
pub const ABM_EXIT: u32 = 0x00a5_4720;
pub const BYTE_1340858: u32 = 0x0134_0858;
pub const ET_CHR_SHOW: u32 = 0x007f_f090;
pub const TEX_VDISP_LOAD: u32 = 0x007e_5210;
pub const PPP_INIT_DATA: u32 = 0x0071_1fa0;
pub const ABM_RUNNING: u32 = 0x01a8_5f70;
pub const ABM_LEAVE_DONE: u32 = 0x01a8_5f64;
pub const ABM_PPP_RECORD: u32 = 0x0230_5800;
pub const ABM_PPP_DATA: u32 = 0x00c8_6080;
pub const FONT_1612_EDGE: u32 = 0x008f_c870;
pub const DBG_PRINTF: u32 = 0x0062_f500;
pub const MSG_B53060: u32 = 0x00b5_3060;
pub const GET_FFX_LANG_JMP: u32 = 0x008a_c2f0;
pub const CALLBACK_OBJECT: u32 = 0x0186_5f04;

/// `0x008ed120`'s worker `0x008ed190` (11 words); `0x008f8870`'s
/// `0x008f1880` (8 words).
pub const MES_FONT_8ED190: u32 = 0x008e_d190;
pub const FONT_8F1880: u32 = 0x008f_1880;
/// `SetWakkalimitMagicFileTexturesUnloaded`: the count `0x018716a0`, the
/// names from `0x01875710` (0x100 bytes each) and the unload
/// `0x00642360` (cdecl).
pub const WAKKA_COUNT: u32 = 0x0187_16a0;
pub const WAKKA_NAMES: u32 = 0x0187_5710;
pub const UNLOAD_TEXTURE: u32 = 0x0064_2360;
/// `0x008fc770`'s callees `0x009060f0`, `0x00905ef0`, `0x00906050`
/// (cdecl, two words) and its f64 pad `0x00b92248`.
pub const GLYPH_A: u32 = 0x0090_60f0;
pub const GLYPH_B: u32 = 0x0090_5ef0;
pub const GLYPH_C: u32 = 0x0090_6050;
pub const ADVANCE_PAD: u32 = 0x00b9_2248;
/// The shape infos: `0x008fa7d0(n)` and `0x008fa4f0(id)` (cdecl); the
/// width at `+8` (u16).
pub const SHAPE_INFO: u32 = 0x008f_a7d0;
pub const SHAPE_INFO_ID: u32 = 0x008f_a4f0;

/// Round 3, batch 2: the packet builders' callees (cdecl): `TOGsTEXFLUSH`,
/// `TOGsALPHA`, `TOGsWords4`, `TOGsXYZ2Offset`, `TOAdpTEST`, `TOAdpTEX0`
/// (13 words), `TOAdpWords4`, `TOAdpXY2`, `TOAdpXYZ2Offset`;
/// `TOGetWindowLayout`, `TOMkpShapeXYWHC2`, `TOMkpShapeXYC2`,
/// `TOMkpShapeXY`, `0x00901c10` (a shape with its own colours), the
/// window colours `0x025d16b4`/`0x025d16b0`.
pub const GS_TEXFLUSH: u32 = 0x008f_ac80;
pub const GS_ALPHA: u32 = 0x008f_aa20;
pub const GS_WORDS4: u32 = 0x008f_ab80;
pub const GS_XYZ2_OFFSET: u32 = 0x008f_acf0;
pub const ADP_TEST: u32 = 0x008e_f970;
pub const ADP_TEX0: u32 = 0x008e_fa00;
pub const ADP_WORDS4: u32 = 0x008e_f8e0;
pub const ADP_XY2: u32 = 0x008e_fb90;
pub const ADP_XYZ2_OFFSET: u32 = 0x008e_fbf0;
pub const WINDOW_LAYOUT: u32 = 0x008f_a4f0;
pub const MKP_SHAPE_XYWHC2: u32 = 0x0090_3b70;
pub const MKP_SHAPE_XYC2: u32 = 0x0090_3a50;
pub const MKP_SHAPE_XY: u32 = 0x0090_3a10;
pub const SHAPE_901C10: u32 = 0x0090_1c10;
pub const WINDOW_COLOUR: u32 = 0x025d_16b4;
pub const WINDOW_COLOUR2: u32 = 0x025d_16b0;
/// The battle plate's callees: `TOMakePktSampleBlueWindowXY2`,
/// `TOGetSaveChrName`, `TOMkpEasyMesFontLCenter`, `TOBtlDrawParamPlate`;
/// the save slot's: `TkMenuGetSaveFileInfo`, `g_TkSaveSlots`,
/// `TOMenuTransSmallFacePlyTex`, `TOMakePktScissor`, `0x008f8870`.
pub const BLUE_WINDOW: u32 = 0x008f_de50;
pub const SAVE_CHR_NAME: u32 = 0x008a_c850;
pub const MES_FONT_L_CENTER: u32 = 0x0090_1c50;
pub const PARAM_PLATE: u32 = 0x0089_5940;
pub const SAVE_FILE_INFO: u32 = 0x008b_4e40;
pub const SAVE_SLOTS: u32 = 0x0186_6290;
pub const SMALL_FACE: u32 = 0x0090_1160;
pub const PKT_SCISSOR: u32 = 0x008f_dee0;
pub const FONT_8F8870: u32 = 0x008f_8870;
/// The number printers: the CRT's `sprintf` (import slot), the formats
/// `0x00b5f090` and `0x00b66b48`; `TODrawFont3d80String_008ee060`,
/// `TOValueToMesFontL`, `TOGetFFXLangJmp`, `MsGetSaveConfigEnglish`,
/// `TOAddPktBaseMesFontLClutTypeRGBA`.
pub const SPRINTF_IAT: u32 = 0x00b0_c3dc;
pub const FMT_B5F090: u32 = 0x00b5_f090;
pub const FMT_B66B48: u32 = 0x00b6_6b48;
pub const FONT3D80_STRING: u32 = 0x008e_e060;
pub const VALUE_TO_MES_FONT_L: u32 = 0x008b_9140;
pub const SAVE_CONFIG_ENGLISH: u32 = 0x0078_51d0;
pub const BASE_MES_FONT_RGBA: u32 = 0x008e_92a0;
/// The small and centred string wrappers: `TOAdpCrossEasyStrFontSClutRGBA`
/// (its scale `0x00b5ee10`), `TOGetMessageWH`,
/// `TOAddPktBaseMesFontPlain_008ed060`, `g_Vu0FixedRound` (f64 0.5).
pub const CROSS_STR_FONT_S: u32 = 0x008e_c2f0;
pub const STR_FONT_S_SCALE: u32 = 0x00b5_ee10;
pub const MESSAGE_WH: u32 = 0x008b_9650;
pub const BASE_MES_FONT_PLAIN: u32 = 0x008e_d060;
pub const HALF: u32 = 0x00b9_2220;
/// The font paths: `str_replace`, `fiosUnifyFilename`, the CRT's
/// `strrchr` (import slot), `user_free_thunk`; the two path buffers and
/// the format/language strings.
pub const STR_REPLACE: u32 = 0x00a4_4bd0;
pub const UNIFY_FILENAME: u32 = 0x0067_9820;
pub const STRRCHR_IAT: u32 = 0x00b0_c3a4;
pub const USER_FREE: u32 = 0x0068_7010;
/// A font path's strings: what `str_replace` replaces for languages 0, 9
/// and 10, their folders, and the buffer the path goes to.
pub struct FontPaths {
    pub what: u32,
    pub with: [u32; 3],
    pub dest: u32,
}
/// `TOInitBattleFontPath`'s.
pub const BATTLE_FONTS: FontPaths = FontPaths {
    what: 0x00b6_2f00,
    with: [0x00b6_2ed4, 0x00b6_2f50, 0x00b6_2f24],
    dest: 0x025d_06a0,
};
/// `TOInitEventFontPath`'s.
pub const EVENT_FONTS: FontPaths = FontPaths {
    what: 0x00b6_2e48,
    with: [0x00b6_2e1c, 0x00b6_2e94, 0x00b6_2e6c],
    dest: 0x025d_08a0,
};
/// The other languages' replacement (`what`, `with`).
pub const FONT_OTHER: (u32, u32) = (0x00b6_2ebc, 0x00b4_3b18);

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the originals' parameters: x, y, w, colours
pub mod bridge {
    use super::{
        ABM_EXIT, ABM_LEAVE_DONE, ABM_PPP_DATA, ABM_PPP_RECORD, ABM_RUNNING, ADD_PKT,
        BASE_MES_FONT, BYTE_1340858, CALLBACK_OBJECT, CD_SIZE_KB, CHANGE_SEQ_ANM, CLOSE_SUB_CMD,
        CLOSE_WINDOW, CLOSE_WINDOW2, CLOSE_WINDOW3, DBG_PRINTF, DRAW_KICK, EASY_SHAPE, ET_CHR_SHOW,
        FONT_8F2BF0, FONT_1612_EDGE, FONT_M_ASCII, GET_FFX_LANG_JMP, HDD_MENU_ENTER, HDD_MENU_FUNC,
        IDENTITY, IGNORE_JOIN, ITEM_LIST_MAX, MAIN_HELP, MC_SET_DEBUG_LOCK, MES_FONT_8ED120,
        MSG_B53060, OPT_ITEM, OPT_WINDOW, OPT_WORD_5C, OPT_WORD_44, OPT_WORD_54, OPT_WORD_60,
        OPTION_MENU, PKT_8EB4E0, PKT_CURSOR, PKT_LIMIT, PPP_INIT_DATA, SAVE_NEED, SCREEN_X2,
        SEQ_INIT_SCENE, SEQ_MES_FORMAT, SEQ_STOP_EXEC, SEQ_STOP_SCENE, SET_HELP, SHADE_RATIO_DIFF,
        SHADE_RATIO_STEP, STOP_SCENE_NAME, STR_FONT_M, TEX_VDISP_LOAD, TRANS_IMAGE_HL,
        VALUE_FONT_M, WORD_186A22C, WORD_186A230, WORD_134085C,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn put(h: &mut impl Host, sym: u32, v: u32) {
        let at = h.rebase(sym);
        h.write(at, v);
    }

    // ---- the TOMkp wrappers ----

    /// `g_TOPktCursor` = `worker(g_TOPktCursor, args...)`; the new cursor.
    fn wrap(h: &mut impl Host, worker: u32, args: &[u32]) -> u32 {
        let cell = h.rebase(PKT_CURSOR);
        let mut words = vec![h.read::<u32>(cell)];
        words.extend_from_slice(args);
        let cur = cdecl(h, worker, &words);
        h.write(cell, cur);
        cur
    }

    /// The tail jump to `TOCtrlPkt` with `eax` = `eax`: the `eax` it
    /// leaves. Packets not added: `eax` itself; the cursor below the limit:
    /// the cursor; else `TODrawKick()`'s.
    fn ctrl_pkt(h: &mut impl Host, eax: u32) -> u32 {
        if h.read::<u32>(h.rebase(ADD_PKT)) == 0 {
            return eax;
        }
        let cursor: u32 = h.read(h.rebase(PKT_CURSOR));
        if cursor < h.read::<u32>(h.rebase(PKT_LIMIT)) {
            return cursor;
        }
        cdecl(h, DRAW_KICK, &[])
    }

    /// [`wrap`], then the tail jump to `TOCtrlPkt` ([`ctrl_pkt`]).
    fn wrap_ctrl(h: &mut impl Host, worker: u32, args: &[u32]) -> u32 {
        let cur = wrap(h, worker, args);
        ctrl_pkt(h, cur)
    }

    /// `TOMkpValueFontMClutRight(a, b, c, d)`: through `0x008f3280`.
    pub fn value_font_m_clut_right(h: &mut impl Host, args: [u32; 4]) -> u32 {
        wrap(h, VALUE_FONT_M, &args)
    }

    /// `FUN_00902810(a, b, c, d)`: through `0x008f2bf0`.
    pub fn font_902810(h: &mut impl Host, args: [u32; 4]) -> u32 {
        wrap(h, FONT_8F2BF0, &args)
    }

    /// `TOMkpShapeXYWHC2(id, x, y, w, h, c0, c1)`: through
    /// `TOAdpEasyShapeXYWHC2`, then `TOCtrlPkt`.
    pub fn shape_xywhc2(h: &mut impl Host, args: [u32; 7]) -> u32 {
        wrap_ctrl(h, EASY_SHAPE, &args)
    }

    /// `FUN_00901cb0(text, x, y, colour)`: through
    /// `TOAddPktBaseMesFont_008ed0a0`, then `TOCtrlPkt`.
    pub fn base_mes_font(h: &mut impl Host, args: [u32; 4]) -> u32 {
        wrap_ctrl(h, BASE_MES_FONT, &args)
    }

    /// `FUN_00901d10(a, b, c, d, e)`: through `0x008ed120`, then
    /// `TOCtrlPkt`.
    pub fn mes_font_901d10(h: &mut impl Host, args: [u32; 5]) -> u32 {
        wrap_ctrl(h, MES_FONT_8ED120, &args)
    }

    /// `FUN_00901f80(text, out, x, y)`: through `0x008edfc0` with the four
    /// colours 0x80, then `TOCtrlPkt`.
    pub fn str_font_m4(h: &mut impl Host, args: [u32; 4]) -> u32 {
        let [a, b, c, d] = args;
        wrap_ctrl(h, STR_FONT_M, &[a, b, c, d, 0x80, 0x80, 0x80, 0x80])
    }

    /// `TOMkpEasyStrFontM(text, x, y)`: through `0x008edfc0` with 0 and
    /// the four colours 0x80, then `TOCtrlPkt`.
    pub fn easy_str_font_m(h: &mut impl Host, text: u32, x: u32, y: u32) -> u32 {
        wrap_ctrl(h, STR_FONT_M, &[text, x, y, 0, 0x80, 0x80, 0x80, 0x80])
    }

    /// The signed halfword at `p + off`, widened.
    fn i16_at(h: &impl Host, p: u32, off: u32) -> u32 {
        i32::from(h.read::<i16>(p.wrapping_add(off))).cast_unsigned()
    }

    /// `TOMakePktSeqPrimFunc(prim)`: `TOBtlDrawChangeSeqAnm0(+0x1c, +0xc,
    /// +0xe)` (signed halfwords), then `TOCtrlPkt`.
    pub fn seq_prim_func(h: &mut impl Host, prim: u32) -> u32 {
        let args = [
            i16_at(h, prim, 0x1c),
            i16_at(h, prim, 0xc),
            i16_at(h, prim, 0xe),
        ];
        let r = cdecl(h, CHANGE_SEQ_ANM, &args);
        ctrl_pkt(h, r)
    }

    /// `TOMakePktSeqPrimMes(prim)`: `TOMkpEasyStrFontM(fmt, +0xc, +0xe)`
    /// (signed halfwords), then `TOCtrlPkt`.
    pub fn seq_prim_mes(h: &mut impl Host, prim: u32) -> u32 {
        let fmt = h.rebase(SEQ_MES_FORMAT);
        let (x, y) = (i16_at(h, prim, 0xc), i16_at(h, prim, 0xe));
        let r = easy_str_font_m(h, fmt, x, y);
        ctrl_pkt(h, r)
    }

    /// `TOMenuTransSmallFacePlyTex()`: `TOTransImageHL(0x01cc0000,
    /// 0x3100, 0x13, 0, 0, 0x100, 0x80)`.
    pub fn trans_small_face(h: &mut impl Host) -> u32 {
        cdecl(
            h,
            TRANS_IMAGE_HL,
            &[0x01cc_0000, 0x3100, 0x13, 0, 0, 0x100, 0x80],
        )
    }

    /// `FUN_008edfc0(cursor, text, out, a, b, c, d, e, f)`: the cursor
    /// through `Identity_008f2f70`, then each byte of `text` up to its NUL
    /// drawn by `TOAdpFontM_ASCII(cursor, char, out, a..f)` with `out`
    /// stepping by 12. Returns the last cursor.
    pub fn str_font_m(h: &mut impl Host, cursor: u32, text: u32, out: u32, rest: [u32; 6]) -> u32 {
        let mut cur = cdecl(h, IDENTITY, &[cursor]);
        let (mut s, mut o) = (text, out);
        while h.read::<u8>(s) != 0 {
            let mut args = vec![cur, s, o];
            args.extend_from_slice(&rest);
            cur = cdecl(h, FONT_M_ASCII, &args);
            s = s.wrapping_add(1);
            o = o.wrapping_add(12);
        }
        cur
    }

    // ---- menu table entries ----

    /// `code_008ac7a0` and its copies: a constant image address.
    pub fn constant(h: &mut impl Host, sym: u32) -> u32 {
        h.rebase(sym)
    }

    /// `code_008b34f0()`: `g_TkMenuSaveNeed` = `TkCdSizeKBCached()`.
    pub fn update_save_need(h: &mut impl Host) -> u32 {
        let v = cdecl(h, CD_SIZE_KB, &[]);
        put(h, SAVE_NEED, v);
        v
    }

    /// `code_008b3600()`: `TkMcSetDebugLock(0)`.
    pub fn mc_unlock(h: &mut impl Host) -> u32 {
        cdecl(h, MC_SET_DEBUG_LOCK, &[0])
    }

    /// `code_008be250()`: `jmp 0x008fd500`, code the dump has no function
    /// for and only this jump reaches (`tools/refs.py`): `g_TOPktCursor`
    /// through `FUN_008eb4e0`, then the tail jump to `TOCtrlPkt`. No
    /// stack words.
    pub fn pkt_8eb4e0(h: &mut impl Host) -> u32 {
        wrap_ctrl(h, PKT_8EB4E0, &[])
    }

    /// `code_008bf050()`: the u16 at `0x0186a22c` and the word at
    /// `0x0186a230` cleared, then a jump through `0x00c416a8`.
    pub fn hdd_menu_enter(h: &mut impl Host) -> u32 {
        let at = h.rebase(WORD_186A22C);
        h.write(at, 0u16);
        put(h, WORD_186A230, 0);
        let f: u32 = h.read(h.rebase(HDD_MENU_ENTER));
        h.call(f, Conv::Cdecl, &[])
    }

    /// `code_008bf0a0()`: a jump through `0x00c416b4`.
    pub fn hdd_menu_func(h: &mut impl Host) -> u32 {
        let f: u32 = h.read(h.rebase(HDD_MENU_FUNC));
        h.call(f, Conv::Cdecl, &[])
    }

    /// `code_008c66c0()`: the item windows and the sub-command window
    /// closed, then `TOSeqStopScene(1, name)`.
    pub fn close_item_windows(h: &mut impl Host) -> u32 {
        for f in [CLOSE_WINDOW, CLOSE_WINDOW3, CLOSE_WINDOW2, CLOSE_SUB_CMD] {
            cdecl(h, f, &[]);
        }
        let name = h.rebase(STOP_SCENE_NAME);
        cdecl(h, SEQ_STOP_SCENE, &[1, name])
    }

    /// `code_008c6700()`: `jmp TOAmapGetItemListMax`.
    pub fn item_list_max(h: &mut impl Host) -> u32 {
        cdecl(h, ITEM_LIST_MAX, &[])
    }

    /// The option menu's help line: `TkMenuSetHelpMessage(
    /// TkMenuMainHelp(TkMenuMainGetOptionMenu(item)))`.
    fn option_help(h: &mut impl Host, item: u32) {
        let o = cdecl(h, OPTION_MENU, &[item]);
        let m = cdecl(h, MAIN_HELP, &[o]);
        cdecl(h, SET_HELP, &[m]);
    }

    /// `code_008e1910()`: the shade ratio's diff and step 0, the help line
    /// of option 0, the u16 at `0x01871544` cleared,
    /// `TkMenuSetScreenX2Flag(0)`.
    pub fn option_enter(h: &mut impl Host) -> u32 {
        cdecl(h, SHADE_RATIO_DIFF, &[0]);
        cdecl(h, SHADE_RATIO_STEP, &[0]);
        option_help(h, 0);
        let at = h.rebase(OPT_WORD_44);
        h.write(at, 0u16);
        cdecl(h, SCREEN_X2, &[0])
    }

    /// `code_008e1950()`: the help line of the current option
    /// (`0x01871548`), `0x01871554` = 1, `0x0187155c`/`0x01871560` = -1,
    /// `TkMenuMainSetIgnoreJoinFlag(0)`, `0x0134085c` = 0,
    /// `TOSeqInitScene(1)`.
    pub fn option_open(h: &mut impl Host) -> u32 {
        let item: u32 = h.read(h.rebase(OPT_ITEM));
        option_help(h, item);
        put(h, OPT_WORD_54, 1);
        put(h, OPT_WORD_5C, u32::MAX);
        put(h, OPT_WORD_60, u32::MAX);
        cdecl(h, IGNORE_JOIN, &[0]);
        put(h, WORD_134085C, 0);
        cdecl(h, SEQ_INIT_SCENE, &[1])
    }

    /// `code_008e19b0()`: `0x01871554` = 0, `TOSeqStopExecScene(1)`.
    pub fn option_close(h: &mut impl Host) -> u32 {
        put(h, OPT_WORD_54, 0);
        cdecl(h, SEQ_STOP_EXEC, &[1])
    }

    /// `code_008e19d0()`: the window at `0x01871540`, when set, gets its
    /// close byte (`+0x41`) and is forgotten; `TkMenuSetScreenX2Flag(0)`.
    pub fn option_leave(h: &mut impl Host) -> u32 {
        let cell = h.rebase(OPT_WINDOW);
        let w: u32 = h.read(cell);
        if w != 0 {
            h.write(w.wrapping_add(0x41), 1u8);
            h.write(cell, 0u32);
        }
        cdecl(h, SCREEN_X2, &[0])
    }

    /// `code_008e27b0()`: `eiAbmExit()`, the byte `0x01340858` cleared; the
    /// exit's `eax`.
    pub fn abm_exit(h: &mut impl Host) -> u32 {
        let r = cdecl(h, ABM_EXIT, &[]);
        let at = h.rebase(BYTE_1340858);
        h.write(at, 0u8);
        r
    }

    /// `code_008e27c0()`: `jmp 0x00a56030`, code the dump has no function
    /// for and only this jump reaches: `op_et_chr_show(1)`,
    /// `op_tex_vdisp_load_triger(1)`; the second's `eax`.
    pub fn abm_enter(h: &mut impl Host) -> u32 {
        cdecl(h, ET_CHR_SHOW, &[1]);
        cdecl(h, TEX_VDISP_LOAD, &[1])
    }

    /// `code_008e27d0()`: `jmp 0x00a54760`, code the dump has no function
    /// for and only this jump reaches: `g_AbmapRunning` = 0,
    /// `op_et_chr_show(0)`, `op_tex_vdisp_load_triger(0)`; done there
    /// once `0x01a85f64` is set (that call's `eax`). Else the flag is set,
    /// the five halfwords `+4`, `+8`, `+6`, `+0xa`, `+0xc` of the record
    /// at `[0x02305800]` cleared unless its first byte is set (the pointer
    /// read again before each), and `pppInitData(record, 0x00c86080, 1)`;
    /// its `eax`.
    pub fn abm_leave(h: &mut impl Host) -> u32 {
        put(h, ABM_RUNNING, 0);
        cdecl(h, ET_CHR_SHOW, &[0]);
        let r = cdecl(h, TEX_VDISP_LOAD, &[0]);
        let flag = h.rebase(ABM_LEAVE_DONE);
        if h.read::<u32>(flag) != 0 {
            return r;
        }
        let cell = h.rebase(ABM_PPP_RECORD);
        let p: u32 = h.read(cell);
        h.write(flag, 1u32);
        if h.read::<u8>(p) == 0 {
            for off in [4u32, 8, 6, 0xa, 0xc] {
                let p: u32 = h.read(cell);
                h.write(p.wrapping_add(off), 0u16);
            }
        }
        let p: u32 = h.read(cell);
        let data = h.rebase(ABM_PPP_DATA);
        cdecl(h, PPP_INIT_DATA, &[p, data, 1])
    }

    /// `code_00906260(a, b)`: `jmp TOMakePktFont1612Edge`.
    pub fn font_1612_edge(h: &mut impl Host, a: u32, b: u32) -> u32 {
        cdecl(h, FONT_1612_EDGE, &[a, b])
    }

    /// `code_0076ef90()`: `dbgPrintf(0x00b53060)`.
    pub fn print_b53060(h: &mut impl Host) -> u32 {
        let msg = h.rebase(MSG_B53060);
        cdecl(h, DBG_PRINTF, &[msg])
    }

    /// `TOGetFFXLang_008dfe10()`: `jmp TOGetFFXLangJmp`.
    pub fn get_ffx_lang(h: &mut impl Host) -> u32 {
        cdecl(h, GET_FFX_LANG_JMP, &[])
    }

    /// `FUN_008abb00()`: the object at `0x01865f04`'s `+8` called with 1.
    pub fn callback_one(h: &mut impl Host) -> u32 {
        let o: u32 = h.read(h.rebase(CALLBACK_OBJECT));
        let f: u32 = h.read(o.wrapping_add(8));
        h.call(f, Conv::Cdecl, &[1])
    }
    /// `FUN_008ed120(cursor, a, b, c, d, e)`: `0x008ed190(cursor, a, b,
    /// c, d, 0x80, 0x80, 0x80, 0x80, e, 0)`.
    pub fn mes_font_8ed120(h: &mut impl Host, args: [u32; 6]) -> u32 {
        let [cursor, a, b, c, d, e] = args;
        cdecl(
            h,
            super::MES_FONT_8ED190,
            &[cursor, a, b, c, d, 0x80, 0x80, 0x80, 0x80, e, 0],
        )
    }

    /// `FUN_008f8870(a, b, c)`: `g_TOPktCursor` through `0x008f1880` with
    /// the four colours 0x80.
    pub fn font_8f8870(h: &mut impl Host, a: u32, b: u32, c: u32) -> u32 {
        wrap(h, super::FONT_8F1880, &[a, b, c, 0x80, 0x80, 0x80, 0x80])
    }

    /// `SetWakkalimitMagicFileTexturesUnloaded()`: each of the
    /// `0x018716a0` loaded names (0x100 bytes apart from `0x01875710`,
    /// the count signed) unloaded (`0x00642360`), the count cleared.
    /// Returns the last unload's `eax`; with none the original leaves
    /// `eax` alone (`None`).
    pub fn wakka_textures_unloaded(h: &mut impl Host) -> Option<u32> {
        let cell = h.rebase(super::WAKKA_COUNT);
        let mut r = None;
        let mut i = 0u32;
        while i.cast_signed() < h.read::<u32>(cell).cast_signed() {
            let name = h
                .rebase(super::WAKKA_NAMES)
                .wrapping_add(i.wrapping_mul(0x100));
            r = Some(cdecl(h, super::UNLOAD_TEXTURE, &[name]));
            i = i.wrapping_add(1);
        }
        h.write(cell, 0u32);
        r
    }

    /// `FUN_008fc770(pos, glyph)`: `0x009060f0(pos, glyph)`,
    /// `0x00905ef0(pos, glyph)`, `0x00906050(pos, glyph)` when the flags
    /// `+0x26` have bit 0 or 1; then `pos.x` (`+4`) += the advance
    /// (`+0x1c`) plus a constant (f64 `0x00b92248`), in f64, stored as f32.
    /// Returns the flags halfword or `0x00906050`'s `eax`.
    pub fn glyph_advance(h: &mut impl Host, pos: u32, glyph: u32) -> u32 {
        cdecl(h, super::GLYPH_A, &[pos, glyph]);
        cdecl(h, super::GLYPH_B, &[pos, glyph]);
        let flags = u32::from(h.read::<u16>(glyph.wrapping_add(0x26)));
        let r = if flags & 3 != 0 {
            cdecl(h, super::GLYPH_C, &[pos, glyph])
        } else {
            flags
        };
        let adv = f64::from(f32::from_bits(h.read(glyph.wrapping_add(0x1c))));
        let k = f64::from_bits(h.read::<u64>(h.rebase(super::ADVANCE_PAD)));
        let x = f64::from(f32::from_bits(h.read(pos.wrapping_add(4))));
        #[allow(clippy::cast_possible_truncation)]
        let sum = ((adv + k) + x) as f32;
        h.write(pos.wrapping_add(4), sum.to_bits());
        r
    }

    /// A two-part shape: `first` at `(x, y)` with no size, then `second`
    /// right of it (`x` + the first part's width `info + 8`, u16) with
    /// the rest of `w` (`TOAdpEasyShapeXYWHC2` twice; colours `c, c` and
    /// `c, c1`). Returns the cursor.
    fn shape_pair(
        h: &mut impl Host,
        cursor: u32,
        ids: [u32; 2],
        info: u32,
        xyw: [u32; 3],
        c: u32,
        c1: u32,
    ) -> u32 {
        let [x, y, w] = xyw;
        let cur = cdecl(h, EASY_SHAPE, &[cursor, ids[0], x, y, 0, 0, c, c]);
        let ww = u32::from(h.read::<u16>(info.wrapping_add(8)));
        cdecl(
            h,
            EASY_SHAPE,
            &[
                cur,
                ids[1],
                ww.wrapping_add(x),
                y,
                w.wrapping_sub(ww),
                0,
                c,
                c1,
            ],
        )
    }

    /// `FUN_008f1a90` / `FUN_008f1af0(cursor, x, y, w, c, c1)`: shapes
    /// 0xda and 0xdb, the first's width from `0x008fa7d0(0x12)` (looked up
    /// after the first is drawn).
    pub fn shape_da_db(h: &mut impl Host, args: [u32; 6]) -> u32 {
        let [cursor, x, y, w, c, c1] = args;
        let cur = cdecl(h, EASY_SHAPE, &[cursor, 0xda, x, y, 0, 0, c, c]);
        let info = cdecl(h, super::SHAPE_INFO, &[0x12]);
        let ww = u32::from(h.read::<u16>(info.wrapping_add(8)));
        cdecl(
            h,
            EASY_SHAPE,
            &[
                cur,
                0xdb,
                ww.wrapping_add(x),
                y,
                w.wrapping_sub(ww),
                0,
                c,
                c1,
            ],
        )
    }

    /// `FUN_008f0990(cursor, id1, id2, x, y, w, c, c1)`: shapes `id1` and
    /// `id2`, the first's width from `0x008fa4f0(id1)` (looked up first).
    pub fn shape_ids(h: &mut impl Host, args: [u32; 8]) -> u32 {
        let [cursor, id1, id2, x, y, w, c, c1] = args;
        let info = cdecl(h, super::SHAPE_INFO_ID, &[id1]);
        shape_pair(h, cursor, [id1, id2], info, [x, y, w], c, c1)
    }
    // ---- round 3, batch 2: packet builders and menu draws ----

    /// A CRT import (cdecl) through its import slot.
    fn crt(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// Writes `words` from `p` on; returns `p` past them.
    fn put_words(h: &mut impl Host, p: u32, words: &[u32]) -> u32 {
        let mut at = p;
        for &w in words {
            h.write(at, w);
            at = at.wrapping_add(4);
        }
        at
    }

    /// The length of the NUL-terminated string at `s`.
    fn strlen(h: &impl Host, s: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// The word at the `ffx.sym` address `sym`.
    fn global(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    /// `FUN_008eb4e0(cursor)`: a GS tag with `TEXFLUSH` (through
    /// `TOGsTEXFLUSH`, which moves `g_TOPktCursor`), then the alpha, test
    /// and register words of the menu's plain drawing; `g_TOPktCursor`
    /// past them, returned.
    pub fn tex_flush_alpha(h: &mut impl Host, cursor: u32) -> u32 {
        let cell = h.rebase(PKT_CURSOR);
        let p = put_words(h, cursor, &[0x8005, 0x1000_0000, 0xe, 0]);
        h.write(cell, p);
        cdecl(h, super::GS_TEXFLUSH, &[]);
        let p: u32 = h.read(cell);
        let end = put_words(
            h,
            p,
            &[
                0x0200_0000,
                0x01a0_0000,
                0x40,
                0,
                0,
                0,
                8,
                0,
                0x44,
                0x80,
                0x42,
                0,
                0xf,
                0,
                0x47,
                0,
            ],
        );
        h.write(cell, end);
        end
    }

    /// `FUN_009046e0()`: a GS tag and five register writes at
    /// `g_TOPktCursor` (the cursor past them), then the tail jump to
    /// `TOCtrlPkt`.
    pub fn plain_draw_regs(h: &mut impl Host) -> u32 {
        let cell = h.rebase(PKT_CURSOR);
        let p: u32 = h.read(cell);
        let end = put_words(
            h,
            p,
            &[
                0x8005,
                0x1000_0000,
                0xe,
                0,
                0,
                0,
                0x3f,
                0,
                0x44,
                0x80,
                0x42,
                0,
                0xf,
                0,
                0x47,
                0,
                0x60,
                0,
                0x14,
                0,
                5,
                0,
                8,
                0,
            ],
        );
        h.write(cell, end);
        ctrl_pkt(h, end)
    }

    /// `FUN_008f1990` / `FUN_008f1a10(cursor, x, y, w, c, c1)`:
    /// [`shape_da_db`], then shape `id` (0x10b / 0x10a) at `(x + 0x61, y)`
    /// with no size and colours 0x80000000.
    pub fn shape_da_db_icon(h: &mut impl Host, args: [u32; 6], id: u32) -> u32 {
        let [_, x, y, ..] = args;
        let cur = shape_da_db(h, args);
        cdecl(
            h,
            EASY_SHAPE,
            &[
                cur,
                id,
                x.wrapping_add(0x61),
                y,
                0,
                0,
                0x8000_0000,
                0x8000_0000,
            ],
        )
    }

    /// `FUN_008f5e90(a, b, x, y, w, c, c1)`: shape `a` at `(x, y)`, `w`
    /// less `b`'s layout width wide (colours `c, c1`), then shape `b` right
    /// of it with no size (`c1, c1`), both at `g_TOPktCursor` (moved and
    /// returned).
    pub fn shape_left_right(h: &mut impl Host, args: [u32; 7]) -> u32 {
        let [a, b, x, y, w, c, c1] = args;
        let info = cdecl(h, super::WINDOW_LAYOUT, &[b]);
        let ww = u32::from(h.read::<u16>(info.wrapping_add(8)));
        let cell = h.rebase(PKT_CURSOR);
        let cur: u32 = h.read(cell);
        let cur = cdecl(h, EASY_SHAPE, &[cur, a, x, y, w.wrapping_sub(ww), 0, c, c1]);
        h.write(cell, cur);
        let x2 = x.wrapping_sub(ww).wrapping_add(w);
        let cur = cdecl(h, EASY_SHAPE, &[cur, b, x2, y, 0, 0, c1, c1]);
        h.write(cell, cur);
        cur
    }

    /// `FUN_008f9b60(x, y, w, h, c, c1; slots)`: a window frame of shapes
    /// 0xc8..0xcb (`TOMkpShapeXYWHC2`; the corner sizes from layout
    /// 0xc8's halfwords `+8`/`+0xa`), `w` at least 16 (stored back into
    /// its argument slot `slots + 8`), then `TOCtrlPkt`.
    pub fn window_frame(h: &mut impl Host, args: [u32; 6], slots: u32) -> u32 {
        let [x, y, mut w, hh, c, c1] = args;
        if w.cast_signed() < 0x10 {
            w = 0x10;
            h.write(slots.wrapping_add(8), w);
        }
        let info = cdecl(h, super::WINDOW_LAYOUT, &[0xc8]).wrapping_add(4);
        let shape = super::MKP_SHAPE_XYWHC2;
        cdecl(h, shape, &[0xc8, x, y, 0, 0, c, c]);
        let cw = u32::from(h.read::<u16>(info.wrapping_add(4)));
        cdecl(
            h,
            shape,
            &[0xc9, x.wrapping_add(cw), y, w.wrapping_sub(cw), 0, c, c1],
        );
        let ch = u32::from(h.read::<u16>(info.wrapping_add(6)));
        cdecl(
            h,
            shape,
            &[0xca, x, y.wrapping_add(ch), 0, hh.wrapping_sub(ch), c, c],
        );
        let ch = u32::from(h.read::<u16>(info.wrapping_add(6)));
        let cw = u32::from(h.read::<u16>(info.wrapping_add(4)));
        let r = cdecl(
            h,
            shape,
            &[
                0xcb,
                x.wrapping_add(cw),
                y.wrapping_add(ch),
                w.wrapping_sub(cw),
                hh.wrapping_sub(ch),
                c,
                c1,
            ],
        );
        ctrl_pkt(h, r)
    }

    /// `FUN_008f9a40(x, y, w, h; slots)`: a window in the window colours
    /// (`0x025d16b4`, `0x025d16b0`): under 16 wide, two `0x00901c10`
    /// shapes (0xc8 and 0xca); else four `TOMkpShapeXYWHC2` (0xc8..0xcb,
    /// the corner sizes from layout 0xc8, `w` less the corner stored into
    /// its argument slot `slots + 8`). Then `TOCtrlPkt`.
    pub fn window_plain(h: &mut impl Host, args: [u32; 4], slots: u32) -> u32 {
        let [x, y, w, hh] = args;
        let info = cdecl(h, super::WINDOW_LAYOUT, &[0xc8]);
        let cw = u32::from(h.read::<u16>(info.wrapping_add(8)));
        let ch = u32::from(h.read::<u16>(info.wrapping_add(0xa)));
        let r = if w.cast_signed() < 0x10 {
            cdecl(h, super::WINDOW_LAYOUT, &[0xc8]);
            let a = global(h, super::WINDOW_COLOUR);
            cdecl(h, super::SHAPE_901C10, &[0xc8, x, y, w, ch, a, a]);
            let a = global(h, super::WINDOW_COLOUR);
            cdecl(
                h,
                super::SHAPE_901C10,
                &[0xca, x, ch.wrapping_add(y), w, hh.wrapping_sub(ch), a, a],
            )
        } else {
            let shape = super::MKP_SHAPE_XYWHC2;
            let a = global(h, super::WINDOW_COLOUR);
            cdecl(h, shape, &[0xc8, x, y, 0, 0, a, a]);
            let b0 = global(h, super::WINDOW_COLOUR2);
            let a = global(h, super::WINDOW_COLOUR);
            let rest = w.wrapping_sub(cw);
            h.write(slots.wrapping_add(8), rest);
            cdecl(h, shape, &[0xc9, cw.wrapping_add(x), y, rest, 0, a, b0]);
            let a = global(h, super::WINDOW_COLOUR);
            let (y2, h2) = (y.wrapping_add(ch), hh.wrapping_sub(ch));
            cdecl(h, shape, &[0xca, x, y2, 0, h2, a, a]);
            let b0 = global(h, super::WINDOW_COLOUR2);
            let a = global(h, super::WINDOW_COLOUR);
            let rest: u32 = h.read(slots.wrapping_add(8));
            cdecl(h, shape, &[0xcb, cw.wrapping_add(x), y2, rest, h2, a, b0])
        };
        ctrl_pkt(h, r)
    }

    /// `TOMakePktSampleBlueWindowXY2(x0, y0, x1, y1)`: a GS tag, alpha
    /// (`TOGsALPHA(0, 1, 0, 1, 0x80)`) and two gradient strips
    /// (`TOGsWords4` colours, `TOGsXYZ2Offset` corners `(x0, y0)`, `(x1,
    /// y0)`, then `(x0, y1)`, `(x1, y1)`), then the tail jump to
    /// `TOCtrlPkt`.
    pub fn sample_blue_window(h: &mut impl Host, args: [u32; 4]) -> u32 {
        let [x0, y0, x1, y1] = args;
        let cell = h.rebase(PKT_CURSOR);
        let p: u32 = h.read(cell);
        let p = put_words(h, p, &[0x8001, 0x70a6_4000, 0x0441_441e, 0]);
        h.write(cell, p);
        cdecl(h, super::GS_ALPHA, &[0, 1, 0, 1, 0x80]);
        cdecl(h, super::GS_WORDS4, &[0x10, 0x10, 0x70, 0x58]);
        cdecl(h, super::GS_XYZ2_OFFSET, &[x0, y0]);
        cdecl(h, super::GS_XYZ2_OFFSET, &[x1, y0]);
        cdecl(h, super::GS_WORDS4, &[0x50, 0x50, 0x50, 0x60]);
        cdecl(h, super::GS_XYZ2_OFFSET, &[x0, y1]);
        let r = cdecl(h, super::GS_XYZ2_OFFSET, &[x1, y1]);
        ctrl_pkt(h, r)
    }

    /// `TOBtlDrawChangeSeqAnm0(chr, x, y)`: the blue window
    /// `(x, y)..(x + 0xa2, y + 0x2c)`, the face shape (`chr + 0x1b8`) at
    /// `(x + 2, y + 2)`, the name centred at `(x + 0x61, y + 2)` and the
    /// parameter plate at `(x + 2, y + 0x17)`; the plate's `eax`.
    pub fn change_seq_anm(h: &mut impl Host, chr: u32, x: u32, y: u32) -> u32 {
        cdecl(
            h,
            super::BLUE_WINDOW,
            &[x, y, x.wrapping_add(0xa2), y.wrapping_add(0x2c)],
        );
        let (x2, y2) = (x.wrapping_add(2), y.wrapping_add(2));
        cdecl(h, super::MKP_SHAPE_XY, &[chr.wrapping_add(0x1b8), x2, y2]);
        let name = cdecl(h, super::SAVE_CHR_NAME, &[chr]);
        cdecl(
            h,
            super::MES_FONT_L_CENTER,
            &[name, x.wrapping_add(0x61), y2],
        );
        cdecl(h, super::PARAM_PLATE, &[chr, x2, y.wrapping_add(0x17)])
    }

    /// `FUN_008b37d0(save, x, y; slots)`: the save file `i16 save + 0x48`'s
    /// info (`TkMenuGetSaveFileInfo`, kept in the argument slot `slots`);
    /// when its slot state (`g_TkSaveSlots`) is 1, the small faces
    /// (`TOMenuTransSmallFacePlyTex`), a full-screen scissor and the
    /// party's faces: each of the 7 bytes from `info + 5` but 0xff drawn
    /// by `0x008f8870(face, x, y)` (face 6 as 7 when `info + 0x1e` is
    /// set), `x` stepping 0x3b per drawn face. Returns the `eax` the
    /// original leaves: the info, or the last face call's with the low
    /// byte of a trailing 0xff byte read over it.
    pub fn save_party_faces(h: &mut impl Host, save: u32, x: u32, y: u32, slots: u32) -> u32 {
        let idx = i32::from(h.read::<i16>(save.wrapping_add(0x48))).cast_unsigned();
        let info = cdecl(h, super::SAVE_FILE_INFO, &[idx]);
        h.write(slots, info);
        let state: u8 = h.read(h.rebase(super::SAVE_SLOTS).wrapping_add(idx));
        if state != 1 {
            return info;
        }
        cdecl(h, super::SMALL_FACE, &[]);
        let mut eax = cdecl(h, super::PKT_SCISSOR, &[0, 0, 0x200, 0x1a0]);
        let mut at = x;
        for i in 0..7u32 {
            let b: u8 = h.read(info.wrapping_add(5 + i));
            eax = (eax & !0xff) | u32::from(b);
            if b == 0xff {
                continue;
            }
            let mut face = u32::from(b);
            // the flag through the info the slot keeps (reread after each call)
            let kept: u32 = h.read(slots);
            if face == 6 && h.read::<u8>(kept.wrapping_add(0x1e)) != 0 {
                face = 7;
            }
            eax = cdecl(h, super::FONT_8F8870, &[face, at, y]);
            at = at.wrapping_add(0x3b);
        }
        eax
    }

    /// `FUN_008f1880(p, slot, x, y, c0, c1, c2, c3; slots)`: for a face
    /// slot 0..7 (unsigned), a sprite packet at `p`: a GS tag, `TOAdpTEST`,
    /// `TOAdpTEX0` of the face page, a tag and the colours
    /// (`TOAdpWords4`), then the face's cell (`(slot % 4) * 64`,
    /// `(slot / 4) * 64`, the first stored into the argument slot
    /// `slots + 4`) and the screen corners `(x, y)`, `(x + 0x40, y +
    /// 0x40)`. Returns the packet's end, or `p` for another slot.
    pub fn face_sprite(h: &mut impl Host, args: [u32; 8], slots: u32) -> u32 {
        let [p, slot, x, y, c0, c1, c2, c3] = args;
        if slot > 7 {
            return p;
        }
        let q = put_words(h, p, &[2, 0x1000_0000, 0xe, 0]);
        let q = cdecl(h, super::ADP_TEST, &[q, 1, 7, 0, 0, 0, 0, 0, 0]);
        let q = h.call_sym_wide(
            super::ADP_TEX0,
            &[q, 0x3100, 4, 0x13, 8, 7, 1, 0, 0x2ed8, 0, 0, 0, 2],
        );
        let q = put_words(h, q, &[0x8001, 0x508f_4000, 0x0004_3431, 0]);
        let q = cdecl(h, super::ADP_WORDS4, &[q, c0, c1, c2, c3]);
        let v = (slot >> 2) << 6;
        let u = (slot & 3) << 6;
        h.write(slots.wrapping_add(4), u);
        let q = cdecl(h, super::ADP_XY2, &[q, u, v]);
        let q = cdecl(h, super::ADP_XYZ2_OFFSET, &[q, x, y]);
        let u: u32 = h.read(slots.wrapping_add(4));
        let q = cdecl(
            h,
            super::ADP_XY2,
            &[q, u.wrapping_add(0x40), v.wrapping_add(0x40)],
        );
        cdecl(
            h,
            super::ADP_XYZ2_OFFSET,
            &[q, x.wrapping_add(0x40), y.wrapping_add(0x40)],
        )
    }

    /// `sprintf(buf, fmt, value)` into `buf`; the string's length.
    fn print_value(h: &mut impl Host, buf: u32, fmt: u32, value: u32) -> u32 {
        let fmt = h.rebase(fmt);
        crt(h, super::SPRINTF_IAT, &[buf, fmt, value]);
        strlen(h, buf)
    }

    /// `FUN_008f3280(cursor, value, x, a, b)` with its frame at `ebp`:
    /// `value` printed (`sprintf` with `0x00b5f090`) into `ebp - 0x24`
    /// and drawn right-aligned (`x - 12 * len + 2`) by `0x008edfc0` with
    /// the four colours 0x80; its cursor.
    pub fn value_font_m(h: &mut impl Host, ebp: u32, args: [u32; 5]) -> u32 {
        let [cursor, value, x, a, b] = args;
        let buf = ebp.wrapping_sub(0x24);
        let len = print_value(h, buf, super::FMT_B5F090, value);
        let x = x.wrapping_sub(len.wrapping_mul(12)).wrapping_add(2);
        cdecl(
            h,
            STR_FONT_M,
            &[cursor, buf, x, a, b, 0x80, 0x80, 0x80, 0x80],
        )
    }

    /// `FUN_008f3390(cursor, value, x, a, b)` with its frame at `ebp`: as
    /// [`value_font_m`], 10 pixels a digit, drawn by
    /// `TODrawFont3d80String_008ee060`.
    pub fn value_font_3d80(h: &mut impl Host, ebp: u32, args: [u32; 5]) -> u32 {
        let [cursor, value, x, a, b] = args;
        let buf = ebp.wrapping_sub(0x24);
        let len = print_value(h, buf, super::FMT_B5F090, value);
        let x = x.wrapping_sub(len.wrapping_mul(10)).wrapping_add(2);
        cdecl(
            h,
            super::FONT3D80_STRING,
            &[cursor, buf, x, a, b, 0x80, 0x80, 0x80, 0x80],
        )
    }

    /// `TOMkpBtlLimitLuluMidareCounter(value, x, y, c)` with its frame at
    /// `ebp`: `value` printed (`0x00b66b48`) into `ebp - 0xc`, each of
    /// its first 8 characters that is a digit drawn as shape `0x1ba +
    /// char` at `(x + 16 i, y)` (`TOMkpShapeXYC2`, colours `c, c`), then
    /// `TOCtrlPkt`. Returns the `eax` the original leaves (the last
    /// character read sits in its low byte when no shape follows it).
    pub fn lulu_counter(h: &mut impl Host, ebp: u32, args: [u32; 4]) -> u32 {
        let [value, x, y, c] = args;
        let buf = ebp.wrapping_sub(0xc);
        let fmt = h.rebase(super::FMT_B66B48);
        let mut eax = crt(h, super::SPRINTF_IAT, &[buf, fmt, value]);
        for i in 0..8u32 {
            let ch: u8 = h.read(buf.wrapping_add(i));
            eax = (eax & !0xff) | u32::from(ch);
            if ch == 0 {
                break;
            }
            let ch = ch.cast_signed();
            if (0x30..=0x39).contains(&ch) {
                let id = i32::from(ch).cast_unsigned().wrapping_add(0x1ba);
                eax = cdecl(
                    h,
                    super::MKP_SHAPE_XYC2,
                    &[id, x.wrapping_add(16 * i), y, c, c],
                );
            }
        }
        ctrl_pkt(h, eax)
    }

    /// `TOAdpValueFontLClutRight(cursor, value, x, a, b)` with its frame
    /// at `ebp`: `value` as message-font digits
    /// (`TOValueToMesFontL(value, ebp - 0x24, 0)`), drawn right-aligned
    /// by `TOAddPktBaseMesFontLClutTypeRGBA(cursor, -1, text, x', a, b,
    /// 0, 0x80 x4)`: `x' = x - 12 len` for Japanese text (language 0 and
    /// not the English config), else `x - 9 len`.
    pub fn value_font_l_clut_right(h: &mut impl Host, ebp: u32, args: [u32; 5]) -> u32 {
        let [cursor, value, x, a, b] = args;
        let buf = ebp.wrapping_sub(0x24);
        cdecl(h, super::VALUE_TO_MES_FONT_L, &[value, buf, 0]);
        let wide =
            cdecl(h, GET_FFX_LANG_JMP, &[]) == 0 && cdecl(h, super::SAVE_CONFIG_ENGLISH, &[]) == 0;
        let len = strlen(h, buf);
        let step = if wide { 12 } else { 9 };
        let x = x.wrapping_sub(len.wrapping_mul(step));
        cdecl(
            h,
            super::BASE_MES_FONT_RGBA,
            &[cursor, u32::MAX, buf, x, a, b, 0, 0x80, 0x80, 0x80, 0x80],
        )
    }

    /// `i` as `fild`/`fstp dword` makes it: the nearest `f32`.
    #[allow(clippy::cast_precision_loss)]
    fn int_f32(i: u32) -> f32 {
        i.cast_signed() as f32
    }

    /// `TOMkpEasyStrFontS(text, x, y; slots)`:
    /// `TOAdpCrossEasyStrFontSClutRGBA(text, f32 x, f32 y, 0, 0x80 x4,
    /// scale, 1.0)` (the scale at `0x00b5ee10`); the original converts
    /// through its `y` argument slot (`slots + 8`), which keeps `f32 x`.
    pub fn easy_str_font_s(h: &mut impl Host, text: u32, x: u32, y: u32, slots: u32) -> u32 {
        let yf = int_f32(y).to_bits();
        let xf = int_f32(x).to_bits();
        h.write(slots.wrapping_add(8), xf);
        let scale: u32 = global(h, super::STR_FONT_S_SCALE);
        cdecl(
            h,
            super::CROSS_STR_FONT_S,
            &[
                text,
                xf,
                yf,
                0,
                0x80,
                0x80,
                0x80,
                0x80,
                scale,
                1.0f32.to_bits(),
            ],
        )
    }

    /// `TOMkpEasyMesFontLCenter(text, x, y; slots)` with its frame at
    /// `ebp`: the text's width (`TOGetMessageWH(0, text, ebp - 8, ebp - 4,
    /// 0, 0)`), drawn at `ftol(f32 x - width * 0.5)` (`f32 x` kept in the
    /// argument slot `slots + 4`) by `TOAddPktBaseMesFontPlain_008ed060`
    /// at `g_TOPktCursor`, then `TOCtrlPkt`.
    pub fn easy_mes_font_l_center(h: &mut impl Host, ebp: u32, args: [u32; 3], slots: u32) -> u32 {
        let [text, x, y] = args;
        let (w_at, h_at) = (ebp.wrapping_sub(8), ebp.wrapping_sub(4));
        cdecl(h, super::MESSAGE_WH, &[0, text, w_at, h_at, 0, 0]);
        let xf = int_f32(x);
        h.write(slots.wrapping_add(4), xf.to_bits());
        let w = f64::from(f32::from_bits(h.read(w_at)));
        let half = f64::from_bits(h.read::<u64>(h.rebase(super::HALF)));
        let left = ffx_bridge::x87::ftol(h, f64::from(xf) - w * half);
        let cell = h.rebase(PKT_CURSOR);
        let cur: u32 = h.read(cell);
        let cur = cdecl(h, super::BASE_MES_FONT_PLAIN, &[cur, text, left, y]);
        h.write(cell, cur);
        ctrl_pkt(h, cur)
    }

    /// `TOInitBattleFontPath(path)` / `TOInitEventFontPath(path)` with the
    /// frame at `ebp`: `path` with the language's font folder put in
    /// (`str_replace(path, what, with)`: `what` for languages 0, 9 and 10
    /// with their folder, else `0x00b62ebc` with `0x00b43b18`), unified
    /// (`fiosUnifyFilename(s, ebp - 0x204, 0x200)`), cut after its last
    /// `/` (the CRT's `strrchr`) and copied to `dest` with its NUL; the
    /// replaced string freed (`user_free_thunk`), whose `eax` is returned.
    pub fn init_font_path(h: &mut impl Host, ebp: u32, path: u32, table: &super::FontPaths) -> u32 {
        let lang = cdecl(h, GET_FFX_LANG_JMP, &[]);
        let (what, with) = match lang {
            0 => (table.what, table.with[0]),
            9 => (table.what, table.with[1]),
            10 => (table.what, table.with[2]),
            _ => super::FONT_OTHER,
        };
        let (what, with) = (h.rebase(what), h.rebase(with));
        let s = cdecl(h, super::STR_REPLACE, &[path, what, with]);
        let buf = ebp.wrapping_sub(0x204);
        cdecl(h, super::UNIFY_FILENAME, &[s, buf, 0x200]);
        let slash = crt(h, super::STRRCHR_IAT, &[buf, 0x2f]);
        h.write(slash.wrapping_add(1), 0u8);
        let dest = h.rebase(table.dest);
        let mut i = 0u32;
        loop {
            let b: u8 = h.read(buf.wrapping_add(i));
            h.write(dest.wrapping_add(i), b);
            i = i.wrapping_add(1);
            if b == 0 {
                break;
            }
        }
        cdecl(h, super::USER_FREE, &[s])
    }
}
