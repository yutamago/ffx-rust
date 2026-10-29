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
}
