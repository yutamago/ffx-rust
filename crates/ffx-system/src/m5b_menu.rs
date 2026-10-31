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

/// Round 3, batch 3: the glyph packets. `TOGsXY2`; the doubles
/// `0x00b92270` (2.0), `0x00b41d20` (5.0), `0x00b922a0` (7.0),
/// `0x00b41d00` (1.5); the floats `0x00b47de8` (12.0), `0x00b92314`,
/// `0x00b54f90`, `0x00b9231c` (a glyph's size and UV box); the movie
/// fonts' `TOMakePktFontLItalicEdge`/`Font` and
/// `TOMakePktMovieFont1612Edge`/`Font` (cdecl, two words).
pub const GS_XY2: u32 = 0x008f_aca0;
pub const D_2: u32 = 0x00b9_2270;
pub const D_5: u32 = 0x00b4_1d20;
pub const D_7: u32 = 0x00b9_22a0;
pub const D_1_5: u32 = 0x00b4_1d00;
pub const F_12: u32 = 0x00b4_7de8;
pub const GLYPH_BOX: [u32; 3] = [0x00b9_2314, 0x00b5_4f90, 0x00b9_231c];
pub const ITALIC_EDGE: u32 = 0x008f_ce90;
pub const ITALIC_FONT: u32 = 0x008f_d0a0;
pub const MOVIE_EDGE: u32 = 0x008f_d9f0;
pub const MOVIE_FONT: u32 = 0x008f_dc00;
/// The UI quads: `graphicDrawUIElement`, `BeginScissor_XY`,
/// `TOGetShapTextureName`, `TOGetImageWH`, the doubles 512.0
/// (`0x00b41d38`) and 416.0 (`0x00b41d30`); the scroll bar's
/// `FUN_008fb090`; `FUN_0092cc40`'s cells `0x00c8f980`/`0x00c8f970`,
/// `VFX_Copy16From`, `VFX_Copy16To_0072f3c0`, `pppVu0MulX_007500d0`,
/// `VFX_Copy16To` (cdecl).
pub const DRAW_UI_ELEMENT: u32 = 0x0063_eee0;
pub const BEGIN_SCISSOR: u32 = 0x008e_58e0;
pub const SHAP_TEXTURE_NAME: u32 = 0x008a_c8c0;
pub const IMAGE_WH: u32 = 0x008a_c400;
pub const D_512: u32 = 0x00b4_1d38;
pub const D_416: u32 = 0x00b4_1d30;
pub const UI_RECT: u32 = 0x008f_b090;
pub const VEC_CELL: u32 = 0x00c8_f980;
pub const VEC_CELL_V: u32 = 0x00c8_f970;
pub const COPY16_FROM_SYM: u32 = 0x0071_0e30;
pub const COPY16_TO_B: u32 = 0x0072_f3c0;
pub const VU0_MUL_X: u32 = 0x0075_00d0;
pub const COPY16_TO_SYM: u32 = 0x0071_0e70;

/// Round 3, batch 4: `TOWaitBtlWinMain`, `TOAddPktMenuIcon`,
/// `FUN_008f2e10`, `FUN_008ebc00`, `TOAddPktMesFont1612_008ea060`,
/// `FUN_008ea680`, the CRT's `vsprintf` (import slot), `TODapsString`,
/// `TOGsTEST`; `sceGsSetDefStoreImage`, `FlushCache`, `sceGsSyncPath`,
/// `sceGsExecStoreImage` (cdecl).
pub const WAIT_WIN_MAIN: u32 = 0x008a_8a70;
pub const MENU_ICON: u32 = 0x008e_b240;
pub const MOVIE_MESSAGE_IMAGE: u32 = 0x008f_2e10;
pub const FONT_8EBC00: u32 = 0x008e_bc00;
pub const MES_FONT_1612: u32 = 0x008e_a060;
pub const FONT_8EA680: u32 = 0x008e_a680;
pub const VSPRINTF_IAT: u32 = 0x00b0_c430;
pub const DAPS_STRING: u32 = 0x0089_dec0;
pub const GS_TEST: u32 = 0x008f_abe0;
pub const SET_STORE_IMAGE: u32 = 0x0062_f830;
pub const GS_FLUSH_CACHE: u32 = 0x0062_f4c0;
pub const GS_SYNC_PATH: u32 = 0x0062_f880;
pub const EXEC_STORE_IMAGE: u32 = 0x0062_f7b0;

/// `FUN_008a92a0`'s: `0x00641350`, the colour words `0x00c59548`/`0x00c5954c`,
/// the screen size floats `0x00b41d9c`/`0x00b41d98`, `RealFrameBuffer`.
pub const FRAME_BUFFER_ARG: u32 = 0x0064_1350;
pub const FRAME_BUFFER_RGB: u32 = 0x00c5_9548;
pub const FRAME_BUFFER_ALPHA: u32 = 0x00c5_954c;
pub const FRAME_BUFFER_W: u32 = 0x00b4_1d9c;
pub const FRAME_BUFFER_H: u32 = 0x00b4_1d98;
pub const FRAME_BUFFER_NAME: u32 = 0x00b4_5b2c;

/// `FUN_008adaf0`'s: `MsGetSaveMenu`, the option word `0x00c59558`,
/// `Sg_GetDispMode`, the capture flags `0x01841d40..42`, the buffer words
/// `0x01865f08..18`, its two messages and format, `TOTransImageLH`,
/// `TOSetMessageWindowMode`, the halfwords `0x01841d44`/`0x01841d48`.
pub const SAVE_MENU: u32 = 0x0078_5290;
pub const CAPTURE_OPTION: u32 = 0x00c5_9558;
pub const DISP_MODE: u32 = 0x0082_0550;
pub const CAPTURE_FLAGS: [u32; 3] = [0x0184_1d40, 0x0184_1d41, 0x0184_1d42];
pub const CAPTURE_BUF: [u32; 5] = [
    0x0186_5f08,
    0x0186_5f0c,
    0x0186_5f10,
    0x0186_5f14,
    0x0186_5f18,
];
pub const CAPTURE_MSG: [u32; 3] = [0x00b6_2d20, 0x00b6_2d0c, 0x00b6_2d5c];
pub const TRANS_IMAGE_LH: u32 = 0x008b_0da0;
pub const MESSAGE_WINDOW_MODE: u32 = 0x008a_f840;
pub const CAPTURE_XY: [u32; 2] = [0x0184_1d44, 0x0184_1d48];

/// `printf2`'s: the CRT's `vsprintf` and `strncpy` (import slots), the
/// 128-line ring (`0x01128be8`, 40 bytes a line, the index `0x01128abc`),
/// `sceGsSetDefDrawEnv2` (the debug-output switch), `TkGetFFXCofingAdrs`,
/// the filter `0x01129fe8` and its sense `0x01128ac1`, the output mode
/// `0x01128aaf`, its two outputs `0x00888160` ("FFXDEBUG") and
/// `printf2_dump_0076e890`, the flag `0x01128aae`.
pub const STRNCPY_IAT: u32 = 0x00b0_c368;
pub const LOG_RING: u32 = 0x0112_8be8;
pub const LOG_INDEX: u32 = 0x0112_8abc;
pub const DEBUG_SWITCH: u32 = 0x0088_7c60;
pub const FFX_CONFIG: u32 = 0x0088_d5e0;
pub const LOG_FILTER: u32 = 0x0112_9fe8;
pub const LOG_FILTER_SENSE: u32 = 0x0112_8ac1;
pub const LOG_MODE: u32 = 0x0112_8aaf;
pub const LOG_SEND: u32 = 0x0088_8160;
pub const LOG_SEND_TAG: u32 = 0x00b5_21dc;
pub const LOG_DUMP: u32 = 0x0076_e890;
pub const LOG_WRITTEN: u32 = 0x0112_8aae;

/// Round 3, batch 6: `TODrawBoxXYWHC2FW`'s and `FUN_00774520`'s quads;
/// the latter's UV inset (f64 `0x00b4c168`).
pub const UV_INSET: u32 = 0x00b4_c168;

/// The battle plate's layout (`i16` from `0x025d0aa2`, the frame colour
/// bytes `0x025d0ac8..cb`), `TOMakePktBoxXYWHRGBA`, `MsGetChrLimitGaugeMax`,
/// `MsGetChrLimitGauge`, `MsGetRamChrHPmax`, `MsGetRamChrHP`,
/// `MsCalcWeakLevel`, `MsGetRamChrMP`, `TOBtlDrawLimitGauge`,
/// `TOMkpValueFontMClutRight`, `TODrawBoxXYWHC2FW`.
pub const PLATE_LAYOUT: u32 = 0x025d_0aa2;
pub const BOX_RGBA: u32 = 0x008f_b150;
pub const LIMIT_MAX: u32 = 0x0079_5590;
pub const LIMIT_NOW: u32 = 0x0079_5550;
pub const HP_MAX: u32 = 0x0079_adf0;
pub const HP_NOW: u32 = 0x0079_add0;
pub const WEAK_LEVEL: u32 = 0x0078_bf00;
pub const MP_NOW: u32 = 0x0079_ae50;
pub const LIMIT_GAUGE: u32 = 0x0089_5480;
pub const VALUE_FONT_M_CLUT_RIGHT: u32 = 0x0090_42e0;
pub const BOX_FILL_WIDTH: u32 = 0x008f_3eb0;

/// `TOAdpEasyMesFontLClutRGBAInterMode`'s: the glyph record `0x01841d50`
/// (0x40 bytes; its scales `+0x1c`/`+0x20`), the pen `0x025d09a0`
/// (colour bytes, x `+4`, y `+8`, flags `+0x1a`/`+0x1b`), the glyph
/// lookup `0x008b78b0`, the two passes `0x008fcca0`/`0x008fcad0`, the
/// next character `0x008dfd80`, the font spacing `0x018716d8`, the width
/// factor `0x00c5d778` and the doubles `0x00b922e0`, `0x00b48320`.
pub const GLYPH_RECORD: u32 = 0x0184_1d50;
pub const PEN: u32 = 0x025d_09a0;
pub const GLYPH_LOOKUP: u32 = 0x008b_78b0;
pub const PASS_EDGE: u32 = 0x008f_cca0;
pub const PASS_FONT: u32 = 0x008f_cad0;
pub const NEXT_CHAR: u32 = 0x008d_fd80;
pub const FONT_SPACING: u32 = 0x0187_16d8;
pub const WIDTH_FACTOR: u32 = 0x00c5_d778;
pub const SCALE_BASE: u32 = 0x00b9_22e0;
pub const ADVANCE_FACTOR: u32 = 0x00b4_8320;
pub const CRT_MEMSET: u32 = 0x0094_964c;

/// `TOMakePktMovieFont1612Font`'s: `TOGsTEX0` (12 words), `TOGsXYZOffset`,
/// its two font pages 0x2eb8/0x2ebc.
pub const GS_TEX0: u32 = 0x008f_ac10;
pub const GS_XYZ_OFFSET: u32 = 0x008f_ad50;

/// `FUN_008f1b50`'s window colour pairs (`0x025d16a0`/`9c`, `98`/`94`,
/// `90`/`8c`, `88`/`84`) and its four shape builders.
pub const KIND_COLOURS: [u32; 4] = [0x025d_16a0, 0x025d_1698, 0x025d_1690, 0x025d_1688];
pub const KIND_SHAPES: [u32; 4] = [0x008f_1af0, 0x008f_1a90, 0x008f_1990, 0x008f_1a10];

/// `TODrawCrossMenuIconImportantName`'s: `MsImportantPtr`,
/// `graphicUiRemapY2`/`graphicUiRemapX2` (float results), its seven
/// float constants, `DrawCrossMenuIconXYWHRGBA`, `ToMakeBtlEasyFont`.
pub const IMPORTANT_PTR: u32 = 0x0079_08a0;
pub const REMAP_Y2: u32 = 0x0064_4820;
pub const REMAP_X2: u32 = 0x0064_47e0;
pub const ICON_CONSTS: [u32; 7] = [
    0x00b5_f304,
    0x00b5_9830,
    0x00b5_f2e8,
    0x00b5_a480,
    0x00b5_ee10,
    0x00b9_2314,
    0x00b5_ef30,
];
pub const CROSS_MENU_ICON: u32 = 0x008e_6af0;
pub const BTL_EASY_FONT: u32 = 0x0090_5ab0;

/// `FUN_008a1e50`'s: `TOMkpShapeXYWHUV`, the shared floats `0x00b5f434`
/// (a height) and `0x00b5ef1c` (a width), and per piece its `x` (remapped),
/// `u0` (remapped), `u1` and `v1` floats.
pub const SHAPE_XYWHUV: u32 = 0x0090_3bb0;
pub const PIECE_H: u32 = 0x00b5_f434;
pub const PIECE_W: u32 = 0x00b5_ef1c;
pub const PIECES: [[u32; 4]; 4] = [
    [0x00b4_1d88, 0x00b5_f354, 0x00b5_f38c, 0x00b5_f3a0],
    [0x00b9_2314, 0x00b5_f3b4, 0x00b5_f3a4, 0x00b5_f3b8],
    [0x00b5_f39c, 0x00b5_f3bc, 0x00b5_f3b8, 0x00b5_f3d4],
    [0x00b9_2318, 0x00b5_f3e0, 0x00b5_f3d4, 0x00b5_efa0],
];

/// `TODrawMovieChangeMacro2`'s: `TOGetChangeMacroAddress`, the character
/// class `0x008df610`, `TODrawChangeMacro2`, `TOGetFontInfo`,
/// `TOMakePktMovieFont1612`.
pub const CHANGE_MACRO_ADDRESS: u32 = 0x008b_6660;
pub const CHAR_CLASS: u32 = 0x008d_f610;
pub const CHANGE_MACRO2: u32 = 0x008b_5960;
pub const FONT_INFO: u32 = 0x008b_7240;
pub const MOVIE_FONT_1612: u32 = 0x008f_d960;

/// `FUN_008ea680`'s: `TOMakePktFont1612Shadow_00906290`,
/// `TOMakePktFont1612Font_00906270`, `setButtonData`, `resetButtonData`,
/// `TOAdpButtonIcon_008e9970`, `TOAddPktChangeMacro`, the line step 16.0
/// (`0x00b313a0`) and the icon width 22.0 (`0x00b5f1c8`).
pub const FONT1612_SHADOW: u32 = 0x0090_6290;
pub const FONT1612_FONT: u32 = 0x0090_6270;
pub const SET_BUTTON_DATA: u32 = 0x008b_1350;
pub const RESET_BUTTON_DATA: u32 = 0x008b_1330;
pub const BUTTON_ICON: u32 = 0x008e_9970;
pub const CHANGE_MACRO: u32 = 0x008e_9b80;
pub const LINE_STEP: u32 = 0x00b3_13a0;
pub const ICON_STEP: u32 = 0x00b5_f1c8;

/// `FUN_008ab860`'s: `TOClearMessageSlot_008ab800`, the movie message
/// flags `0x01841d40..42`, the emulated GS `PMODE` (absolute
/// `0x12000000`), the image `0x01865f0c`, `TOTransImageHL`.
pub const CLEAR_MESSAGE_SLOT: u32 = 0x008a_b800;
pub const GS_PMODE: u32 = 0x1200_0000;
pub const MOVIE_IMAGE: u32 = 0x0186_5f0c;

/// `TODrawAlphaBG`'s band edges (floats): 0x00b43db4, 0x00b58cd8,
/// 0x00b65898, 0x00b65d8c, and the screen size 0x00b41d9c/0x00b41d98.
pub const ALPHA_BG_EDGES: [u32; 4] = [0x00b4_3db4, 0x00b5_8cd8, 0x00b6_5898, 0x00b6_5d8c];
pub const SCREEN_W: u32 = 0x00b4_1d9c;
pub const SCREEN_H: u32 = 0x00b4_1d98;

/// `TODrawWaitBtlWinPrepare`'s: `graphicGetTime` (`ST0`),
/// `TOBtlDrawCommandWindow`, `TOBtlSetCurPos`, `g_SgFrame`, the cursor
/// floats `0x00b5ef64` (x step) and `0x00b5ee9c` (y step).
pub const GET_TIME: u32 = 0x0064_1410;
pub const DRAW_COMMAND_WINDOW: u32 = 0x0089_f370;
pub const SET_CUR_POS: u32 = 0x0089_99f0;
pub const SG_FRAME: u32 = 0x023c_bbf0;
pub const CURSOR_X_STEP: u32 = 0x00b5_ef64;
pub const CURSOR_Y_STEP: u32 = 0x00b5_ee9c;

/// `TODrawMovieMessage`'s: `Sg_GetDBuffDC`, `TODapsDraw`,
/// `TOTransTexture`, `TOSetPktBuffAdrs`, `TOOpenPktbuff`,
/// `TOGsFramePacket`, `TOXyoffsetPacket`, `TOMkpFlatRect`,
/// `TOSetRegFont1612`, `TODrawWindow`, `TODrawMessage`, `TOClosePktbuff`,
/// `TOMkpMovieMessageImage`; the packet buffer's address and size, the
/// flag set after the draw.
pub const SG_DBUFF: u32 = 0x0082_0530;
pub const DAPS_DRAW: u32 = 0x0089_dc10;
pub const TRANS_TEXTURE: u32 = 0x008b_1020;
pub const SET_PKT_BUFF: u32 = 0x0090_47c0;
pub const OPEN_PKT: u32 = 0x0090_4600;
pub const FRAME_PKT: u32 = 0x008f_d1f0;
pub const XYOFFSET: u32 = 0x008f_f020;
pub const FLAT_RECT: u32 = 0x008f_c6a0;
pub const SET_REG_FONT1612: u32 = 0x0090_48e0;
pub const DRAW_WINDOW: u32 = 0x0090_6bf0;
pub const DRAW_MESSAGE: u32 = 0x008b_5dc0;
pub const CLOSE_PKT: u32 = 0x008f_3930;
pub const MKP_MOVIE_IMAGE: u32 = 0x0090_2840;
pub const PKT_BUFF_ADRS: [u32; 2] = [0x0186_5f14, 0x0186_5f18];
pub const MOVIE_DRAWN: u32 = 0x025d_09c0;

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
    // ---- round 3, batch 3: glyph packets ----

    use ffx_bridge::x87;

    /// The f32 at `p`, widened (`fld dword`).
    fn f32_at(h: &impl Host, p: u32) -> f64 {
        x87::load(h.read(p))
    }

    /// The f64 at the `ffx.sym` address `sym` (`fadd qword`).
    fn f64_global(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read::<u64>(h.rebase(sym)))
    }

    /// `ftol2_sse` of `x`.
    fn ftol(h: &impl Host, x: f64) -> u32 {
        ffx_bridge::x87::ftol(h, x)
    }

    /// The four header words both glyph passes start with (a GS tag and
    /// the alpha/test registers 0x44, 0x42, 0x47).
    const GLYPH_HEAD: [u32; 12] = [
        0x8001,
        0x90ab_4000,
        0x3431_eeee,
        4,
        0x44,
        0x80,
        0x42,
        0,
        0x0003_000f,
        0,
        0x47,
        0,
    ];

    /// `FUN_00905ef0(pos, glyph)`: a glyph's edge pass at `g_TOPktCursor`
    /// (20 words: the header, TEX0 `0xa1313d80`/`0x2005d705` and a CLUT
    /// word pair built from the glyph's `i16 +4` (plus twice the low
    /// nibble of `pos + 0x1b`) and `i16 +2`), then `TOGsWords4` of the
    /// colour bytes `pos[0..4]`, the UV and screen corners
    /// (`TOGsXY2(ftol(glyph +8), ftol(+0xc))`, `TOGsXYZ2Offset(ftol(pos
    /// +4), ftol(+8))`, `TOGsXY2(ftol(+0x10), ftol(+0x14))`,
    /// `TOGsXYZ2Offset(ftol(+0x1c + pos.x), ftol(+0x20 + pos.y))`) and the
    /// tail jump to `TOCtrlPkt`.
    pub fn glyph_edge(h: &mut impl Host, pos: u32, glyph: u32) -> u32 {
        let cell = h.rebase(PKT_CURSOR);
        let p: u32 = h.read(cell);
        let at = put_words(h, p, &GLYPH_HEAD);
        let at = put_words(h, at, &[0xa131_3d80, 0x2005_d705, 6, 0]);
        let nibble = u32::from(h.read::<u8>(pos.wrapping_add(0x1b))) & 0xf;
        let page = i32::from(h.read::<i16>(glyph.wrapping_add(4)))
            .cast_unsigned()
            .wrapping_add(nibble * 2);
        let base = i32::from(h.read::<i16>(glyph.wrapping_add(2)));
        let sign = if base < 0 { u32::MAX } else { 0 };
        let low = base.cast_unsigned() | 0x5d70_0000;
        // the 64-bit word (page << 4 | sign) : low, shifted left by 20
        let high = (((page << 4) | sign) << 20) | (low >> 12);
        h.write(cell, at.wrapping_add(0x10));
        put_words(h, at, &[low << 20, high, 0x16, 0]);
        let mut colour = [0u32; 4];
        for (k, c) in (0u32..).zip(&mut colour) {
            *c = u32::from(h.read::<u8>(pos.wrapping_add(k)));
        }
        cdecl(h, super::GS_WORDS4, &colour);
        let (u, v) = (
            f32_at(h, glyph.wrapping_add(8)),
            f32_at(h, glyph.wrapping_add(0xc)),
        );
        let (u, v) = (ftol(h, u), ftol(h, v));
        cdecl(h, super::GS_XY2, &[u, v]);
        let (x, y) = (
            f32_at(h, pos.wrapping_add(4)),
            f32_at(h, pos.wrapping_add(8)),
        );
        let (x, y) = (ftol(h, x), ftol(h, y));
        cdecl(h, super::GS_XYZ2_OFFSET, &[x, y]);
        let (u, v) = (
            f32_at(h, glyph.wrapping_add(0x10)),
            f32_at(h, glyph.wrapping_add(0x14)),
        );
        let (u, v) = (ftol(h, u), ftol(h, v));
        cdecl(h, super::GS_XY2, &[u, v]);
        let x = x87::add(
            f32_at(h, glyph.wrapping_add(0x1c)),
            f32_at(h, pos.wrapping_add(4)),
        );
        let y = x87::add(
            f32_at(h, glyph.wrapping_add(0x20)),
            f32_at(h, pos.wrapping_add(8)),
        );
        let (x, y) = (ftol(h, x), ftol(h, y));
        let r = cdecl(h, super::GS_XYZ2_OFFSET, &[x, y]);
        ctrl_pkt(h, r)
    }

    /// `FUN_009060f0(pos, glyph)`: the glyph's fill pass: the header, TEX0
    /// `0xa1313d80`/`0x4005d785` and `0xa1413d80`/`page << 24 | 0x5d785`
    /// (page: `i16 glyph +4` plus twice the high nibble of the byte `pos +
    /// 0x1b`) at `g_TOPktCursor`, `TOGsWords4(0x80 x4)`, the UV and
    /// screen corners (offset by 2.0 and 1.0: the shadow) and the tail
    /// jump to `TOCtrlPkt`.
    pub fn glyph_fill(h: &mut impl Host, pos: u32, glyph: u32) -> u32 {
        let cell = h.rebase(PKT_CURSOR);
        let p: u32 = h.read(cell);
        let at = put_words(h, p, &GLYPH_HEAD);
        let at = put_words(h, at, &[0xa131_3d80, 0x4005_d785, 6, 0]);
        let nibble = u32::from(h.read::<u8>(pos.wrapping_add(0x1b))) >> 4;
        let page = i32::from(h.read::<i16>(glyph.wrapping_add(4)))
            .cast_unsigned()
            .wrapping_add(nibble * 2);
        let at = put_words(h, at, &[0xa141_3d80, (page << 24) | 0x5_d785, 6, 0]);
        h.write(cell, at);
        cdecl(h, super::GS_WORDS4, &[0x80, 0x80, 0x80, 0x80]);
        let (u, v) = (
            f32_at(h, glyph.wrapping_add(8)),
            f32_at(h, glyph.wrapping_add(0xc)),
        );
        let (u, v) = (ftol(h, u), ftol(h, v));
        cdecl(h, super::GS_XY2, &[u, v]);
        let two = f64_global(h, super::D_2);
        let x = x87::add(f32_at(h, pos.wrapping_add(4)), two);
        let y = x87::add(f32_at(h, pos.wrapping_add(8)), 1.0);
        let (x, y) = (ftol(h, x), ftol(h, y));
        cdecl(h, super::GS_XYZ2_OFFSET, &[x, y]);
        let (u, v) = (
            f32_at(h, glyph.wrapping_add(0x10)),
            f32_at(h, glyph.wrapping_add(0x14)),
        );
        let (u, v) = (ftol(h, u), ftol(h, v));
        cdecl(h, super::GS_XY2, &[u, v]);
        let x = x87::add(
            f32_at(h, glyph.wrapping_add(0x1c)),
            f32_at(h, pos.wrapping_add(4)),
        );
        let x = x87::add(x, two);
        let y = x87::add(
            f32_at(h, glyph.wrapping_add(0x20)),
            f32_at(h, pos.wrapping_add(8)),
        );
        let y = x87::add(y, 1.0);
        let (x, y) = (ftol(h, x), ftol(h, y));
        let r = cdecl(h, super::GS_XYZ2_OFFSET, &[x, y]);
        ctrl_pkt(h, r)
    }

    /// Rounds `x` to `f32` as `fstp dword` does.
    fn to_f32(x: f64) -> u32 {
        x87::store(x)
    }

    /// `FUN_00906050(pos, glyph)` with its frame at `ebp`: the glyph's
    /// box set (size 12, UVs from the three floats at `0x00b92314..`),
    /// its flag `+4` = 1 unless bit 0 of `+0x26`; a copy of `pos` at `ebp -
    /// 0x1c` moved by (5.0, -7.0) draws the fill pass (`0x009060f0`), then
    /// the edge pass (`0x00905ef0`); `pos.x` += 5.0. The edge pass's
    /// `eax`.
    pub fn glyph_draw(h: &mut impl Host, ebp: u32, pos: u32, glyph: u32) -> u32 {
        let local = ebp.wrapping_sub(0x1c);
        for i in 0..7u32 {
            let w: u32 = h.read(pos.wrapping_add(4 * i));
            h.write(local.wrapping_add(4 * i), w);
        }
        let flag = h.read::<u8>(glyph.wrapping_add(0x26)) & 1;
        let twelve: u32 = global(h, super::F_12);
        h.write(glyph.wrapping_add(8), twelve);
        for (at, sym) in (0xcu32..).step_by(4).zip(super::GLYPH_BOX) {
            let v: u32 = global(h, sym);
            h.write(glyph.wrapping_add(at), v);
        }
        h.write(glyph.wrapping_add(0x1c), twelve);
        let y = x87::sub(f32_at(h, local.wrapping_add(8)), f64_global(h, super::D_7));
        h.write(local.wrapping_add(8), to_f32(y));
        let five = f64_global(h, super::D_5);
        let x = x87::add(f32_at(h, local.wrapping_add(4)), five);
        h.write(local.wrapping_add(4), to_f32(x));
        h.write(glyph.wrapping_add(4), u16::from(flag == 0));
        cdecl(h, super::GLYPH_A, &[local, glyph]);
        let r = cdecl(h, super::GLYPH_B, &[local, glyph]);
        let x = x87::add(f32_at(h, pos.wrapping_add(4)), five);
        h.write(pos.wrapping_add(4), to_f32(x));
        r
    }

    /// `TOMakePktMovieFont1612(pos, glyph)` with its frame at `ebp`: a
    /// copy of `pos` at `ebp - 0x1c` one pixel lower; type 3 (`pos +
    /// 0x1a`) draws the italic edge and font passes, any other the movie
    /// font's, each for `pos` then the copy; then `pos.x` += the glyph's
    /// advance (`+0x1c`) plus 1.5. The last pass's `eax`.
    pub fn movie_font_1612(h: &mut impl Host, ebp: u32, pos: u32, glyph: u32) -> u32 {
        let local = ebp.wrapping_sub(0x1c);
        let kind = h.read::<u8>(pos.wrapping_add(0x1a));
        for i in 0..7u32 {
            let w: u32 = h.read(pos.wrapping_add(4 * i));
            h.write(local.wrapping_add(4 * i), w);
        }
        let y = x87::add(
            f32_at(h, local.wrapping_add(8)),
            f64_global(h, super::ADVANCE_PAD),
        );
        h.write(local.wrapping_add(8), to_f32(y));
        let (edge, font) = if kind == 3 {
            (super::ITALIC_EDGE, super::ITALIC_FONT)
        } else {
            (super::MOVIE_EDGE, super::MOVIE_FONT)
        };
        cdecl(h, edge, &[pos, glyph]);
        cdecl(h, edge, &[local, glyph]);
        cdecl(h, font, &[pos, glyph]);
        let r = cdecl(h, font, &[local, glyph]);
        let x = x87::add(
            f32_at(h, glyph.wrapping_add(0x1c)),
            f64_global(h, super::D_1_5),
        );
        let x = x87::add(x, f32_at(h, pos.wrapping_add(4)));
        h.write(pos.wrapping_add(4), to_f32(x));
        r
    }

    // ---- round 3, batch 3: UI quads (`graphicDrawUIElement`) ----
    //
    // These build a quad on their own frame (base `ebp - 0x9c`: vertex 0
    // at +0, vertex 1 at +0x20, each x, y, u, v, r, g, b, a; more fields
    // at +0x80..) and hand it to `graphicDrawUIElement`. The words they
    // never write (the rest of the quad) are junk reads where the callee
    // reads them: the port leaves them 0 (StackFrame), brief 2026-10-24.

    /// The quad's base below `ebp`.
    const QUAD: u32 = 0x9c;

    /// The colour bytes of `c` (low first) into the vertex at `at`'s
    /// r, g, b, a words (`+0x10`..).
    fn quad_colour(h: &mut impl Host, at: u32, c: u32) {
        for k in 0..4u32 {
            h.write(at.wrapping_add(0x10 + 4 * k), (c >> (8 * k)) & 0xff);
        }
    }

    /// `FUN_008fd390(x, y, w, h, c0, c1)` with its frame at `ebp`: a plain
    /// quad `(x, y)`, `(x + w, y + h)` (ints as floats), vertex colours
    /// `c0` and `c1` (bytes), `+0x80` = 1000, `+0x88`, `+0x90`, `+0x94` 0,
    /// drawn by `graphicDrawUIElement(quad, 0, 1, 0, 0)`; its `eax`.
    /// junk read: [ebp-0x94], [ebp-0x90], [ebp-0x74], [ebp-0x70] (the UVs)
    /// and vertices 2-3 never written; 0 (brief 2026-10-24)
    pub fn ui_box(h: &mut impl Host, ebp: u32, args: [u32; 6]) -> u32 {
        let [x, y, w, hh, c0, c1] = args;
        let q = ebp.wrapping_sub(QUAD);
        h.write(q, int_f32(x).to_bits());
        h.write(q.wrapping_add(4), int_f32(y).to_bits());
        quad_colour(h, q, c0);
        let (x1, y1) = (x.wrapping_add(w), y.wrapping_add(hh));
        h.write(ebp.wrapping_sub(0xa0), y1);
        h.write(q.wrapping_add(0x20), int_f32(x1).to_bits());
        h.write(q.wrapping_add(0x24), int_f32(y1).to_bits());
        quad_colour(h, q.wrapping_add(0x20), c1);
        for (off, v) in [(0xc, 0), (8, 0), (0x1c, 0x3e8), (0x14, 0)] {
            h.write(ebp.wrapping_sub(off), v);
        }
        cdecl(h, super::DRAW_UI_ELEMENT, &[q, 0, 1, 0, 0])
    }

    /// `FUN_008fb090(x0, y0, x1, y1, r, g, b, a)` with its frame at `ebp`:
    /// a plain quad in one colour, clipped by `BeginScissor_XY(&x0, &y0,
    /// &x1, &y1)` (floats in the quad); drawn by
    /// `graphicDrawUIElement(quad, 0, 1, 0, 0)` unless that returns 0.
    /// Returns the last call's `eax`.
    /// junk read: the quad's UVs ([ebp-0x94], [ebp-0x90], [ebp-0x74],
    /// [ebp-0x70]), +0x80 and vertices 2-3 never written; 0 (brief
    /// 2026-10-24)
    pub fn ui_rect(h: &mut impl Host, ebp: u32, args: [u32; 8]) -> u32 {
        let [x0, y0, x1, y1, r, g, b, a] = args;
        let q = ebp.wrapping_sub(QUAD);
        h.write(q, int_f32(x0).to_bits());
        h.write(q.wrapping_add(4), int_f32(y0).to_bits());
        h.write(q.wrapping_add(0x20), int_f32(x1).to_bits());
        h.write(q.wrapping_add(0x24), int_f32(y1).to_bits());
        for v in [q, q.wrapping_add(0x20)] {
            for (at, c) in (0x10u32..).step_by(4).zip([r, g, b, a]) {
                h.write(v.wrapping_add(at), c);
            }
        }
        for off in [0xc, 8, 0x14] {
            h.write(ebp.wrapping_sub(off), 0u32);
        }
        let ptrs = [
            q,
            q.wrapping_add(4),
            q.wrapping_add(0x20),
            q.wrapping_add(0x24),
        ];
        let shown = cdecl(h, super::BEGIN_SCISSOR, &ptrs);
        if shown == 0 {
            return shown;
        }
        cdecl(h, super::DRAW_UI_ELEMENT, &[q, 0, 1, 0, 0])
    }

    /// `FUN_008f2bf0(cursor, x, y, c0, c1)` with its frame at `ebp`: the
    /// full-screen picture 0x2eac (`TOGetShapTextureName`, its size from
    /// `TOGetImageWH(0x2eac, ebp - 0xa8, ebp - 0xa4)`) as a quad `(x,
    /// y)`, `(x + 0x200, y + 0x1a0)`, UVs `0 / w`, `0 / h`, `512 / w`, `416 /
    /// h` (x87 division), colours `c0`, `c1`; drawn by
    /// `graphicDrawUIElement(quad, name, 1, 0, 0)`. Returns `cursor`.
    /// junk read: the quad's +0x80, +0x88 ([ebp-0x1c], [ebp-0x14]) and
    /// vertices 2-3 never written; 0 (brief 2026-10-24)
    pub fn ui_picture(h: &mut impl Host, ebp: u32, args: [u32; 5]) -> u32 {
        use ffx_bridge::x87;
        let [cursor, x, y, c0, c1] = args;
        h.write(ebp.wrapping_sub(0xc), 0u32);
        h.write(ebp.wrapping_sub(8), 0u32);
        let name = cdecl(h, super::SHAP_TEXTURE_NAME, &[0x2eac]);
        let (w_at, h_at) = (ebp.wrapping_sub(0xa8), ebp.wrapping_sub(0xa4));
        cdecl(h, super::IMAGE_WH, &[0x2eac, w_at, h_at]);
        let q = ebp.wrapping_sub(QUAD);
        h.write(q, int_f32(x).to_bits());
        h.write(q.wrapping_add(4), int_f32(y).to_bits());
        quad_colour(h, q, c0);
        let width = x87::load(h.read(w_at));
        h.write(q.wrapping_add(8), x87::store(x87::div(0.0, width)));
        let height = x87::load(h.read(h_at));
        h.write(q.wrapping_add(0xc), x87::store(x87::div(0.0, height)));
        let (x1, y1) = (x.wrapping_add(0x200), y.wrapping_add(0x1a0));
        h.write(q.wrapping_add(0x20), int_f32(x1).to_bits());
        h.write(ebp.wrapping_sub(0xa0), y1);
        h.write(q.wrapping_add(0x24), int_f32(y1).to_bits());
        quad_colour(h, q.wrapping_add(0x20), c1);
        let right = x87::div(f64_global(h, super::D_512), width);
        h.write(q.wrapping_add(0x28), x87::store(right));
        let bottom = x87::div(f64_global(h, super::D_416), height);
        h.write(q.wrapping_add(0x2c), x87::store(bottom));
        cdecl(h, super::DRAW_UI_ELEMENT, &[q, name, 1, 0, 0]);
        cursor
    }

    /// `FUN_00902700(x, y, w, h, first, count, total; slots)`: a scroll
    /// bar: when `total > count` (signed), the track
    /// (`0x008fb090(x, y, x + w, y + h, 0x11 x3, 0x80)`) and the thumb
    /// (`0x008fb090(x + 1, top, x + w - 1, bottom, 0xf0 x3, 0x80)`, the
    /// ends through `ftol`): per = `(f64(y + h) - y) / total`, top =
    /// `y + first * per` (`y` when `first` is 0), bottom =
    /// `top + count * per` (`y + h` when `first + count >= total`); then
    /// `TOCtrlPkt`. The original keeps `x + 1`, then
    /// `x + w - 1` in its `h` argument slot (`slots + 0xc`). Else it
    /// returns `count`.
    pub fn scroll_bar(h: &mut impl Host, args: [u32; 7], slots: u32) -> u32 {
        use ffx_bridge::x87;
        let [x, y, w, hh, first, count, total] = args;
        let bottom_edge = y.wrapping_add(hh);
        let left = x.wrapping_add(1);
        let right_edge = w.wrapping_add(x);
        let right = right_edge.wrapping_sub(1);
        h.write(slots.wrapping_add(0xc), right);
        if total.cast_signed() <= count.cast_signed() {
            return count;
        }
        let as_f64 = |v: u32| f64::from(v.cast_signed());
        let (yf, y2) = (as_f64(y), as_f64(bottom_edge));
        let per = x87::div(x87::sub(y2, yf), as_f64(total));
        let start = x87::add(x87::mul(as_f64(first), per), yf);
        let end = x87::add(start, x87::mul(as_f64(count), per));
        let top = if first == 0 { yf } else { start };
        let full = first.wrapping_add(count).cast_signed() >= total.cast_signed();
        let bottom = if full { y2 } else { end };
        let track = super::UI_RECT;
        cdecl(
            h,
            track,
            &[x, y, right_edge, bottom_edge, 0x11, 0x11, 0x11, 0x80],
        );
        let thumb = [
            ftol(h, as_f64(left)),
            ftol(h, top),
            ftol(h, as_f64(right)),
            ftol(h, bottom),
            0xf0,
            0xf0,
            0xf0,
            0x80,
        ];
        let r = cdecl(h, track, &thumb);
        ctrl_pkt(h, r)
    }

    /// `FUN_0092cc40(dst, src, v0, v1, v2, v3; slots)`: `src` into the cell
    /// `0x00c8f980` (`VFX_Copy16From`), the by-value vector (its argument
    /// slots, `slots + 8`) into `0x00c8f970` (`VFX_Copy16To_0072f3c0`),
    /// `pppVu0MulX_007500d0(cell, cell, that vector by value)`, then the
    /// cell into `dst` (`VFX_Copy16To`); its `eax`.
    pub fn vec_mul_x(h: &mut impl Host, dst: u32, src: u32, slots: u32) -> u32 {
        let cell = h.rebase(super::VEC_CELL);
        let v = h.rebase(super::VEC_CELL_V);
        cdecl(h, super::COPY16_FROM_SYM, &[cell, src]);
        cdecl(h, super::COPY16_TO_B, &[slots.wrapping_add(8), v]);
        let mut w = [0u32; 4];
        for (k, x) in (0u32..).zip(&mut w) {
            *x = h.read(v.wrapping_add(4 * k));
        }
        cdecl(h, super::VU0_MUL_X, &[cell, cell, w[0], w[1], w[2], w[3]]);
        cdecl(h, super::COPY16_TO_SYM, &[cell, dst])
    }
    // ---- round 3, batch 4: small wrappers ----

    /// `FUN_008a8750(win)`: the wait window's state (`+1` = 4) and its
    /// handler (`+0x8c` = `TOWaitBtlWinMain`), which then runs once
    /// (`TOWaitBtlWinMain(win)`). Returns 0.
    pub fn wait_win_start(h: &mut impl Host, win: u32) -> u32 {
        h.write(win.wrapping_add(1), 4u8);
        let main = h.rebase(super::WAIT_WIN_MAIN);
        h.write(win.wrapping_add(0x8c), main);
        cdecl(h, super::WAIT_WIN_MAIN, &[win]);
        0
    }

    /// `FUN_008fd4d0(a, b, c)`: `g_TOPktCursor` through `TOAddPktMenuIcon`,
    /// then the tail jump to `TOCtrlPkt`.
    pub fn menu_icon(h: &mut impl Host, args: [u32; 3]) -> u32 {
        wrap_ctrl(h, super::MENU_ICON, &args)
    }

    /// `TOMkpMovieMessageImage(a, b, c, d, e)`: `g_TOPktCursor` through
    /// `FUN_008f2e10`; the new cursor.
    pub fn movie_message_image(h: &mut impl Host, args: [u32; 5]) -> u32 {
        wrap(h, super::MOVIE_MESSAGE_IMAGE, &args)
    }

    /// `FUN_008ebbd0(a, b, c, d)`: `FUN_008ebc00(a, b, c, d, 0x80 x4)`.
    pub fn font_8ebc00(h: &mut impl Host, args: [u32; 4]) -> u32 {
        let [a, b, c, d] = args;
        cdecl(h, super::FONT_8EBC00, &[a, b, c, d, 0x80, 0x80, 0x80, 0x80])
    }

    /// `FUN_008e9fd0(cursor, text, x, y)`:
    /// `TOAddPktMesFont1612_008ea060(cursor, text, x, y, 0, 0x80 x4)`.
    pub fn mes_font_1612(h: &mut impl Host, args: [u32; 4]) -> u32 {
        let [a, b, c, d] = args;
        cdecl(
            h,
            super::MES_FONT_1612,
            &[a, b, c, d, 0, 0x80, 0x80, 0x80, 0x80],
        )
    }

    /// `FUN_008ea5c0(a, b, c, d, e)`: `FUN_008ea680(a, b, c, d, e, 0, 0x80
    /// x4)`.
    pub fn font_8ea680(h: &mut impl Host, args: [u32; 5]) -> u32 {
        let [a, b, c, d, e] = args;
        cdecl(
            h,
            super::FONT_8EA680,
            &[a, b, c, d, e, 0, 0x80, 0x80, 0x80, 0x80],
        )
    }

    /// `TOGetWidthExtMes1612(a, text)` with its frame at `ebp`: the message
    /// size (`TOGetMessageWH(a, text, ebp - 8, ebp - 4, 0, 0)`), returned
    /// as `i16 [ebp - 4] << 16 | (ftol(f32 width) as i16)`.
    pub fn width_ext_mes_1612(h: &mut impl Host, ebp: u32, a: u32, text: u32) -> u32 {
        let (w_at, h_at) = (ebp.wrapping_sub(8), ebp.wrapping_sub(4));
        cdecl(h, super::MESSAGE_WH, &[a, text, w_at, h_at, 0, 0]);
        let width = ffx_bridge::x87::load(h.read(w_at));
        let high = i32::from(h.read::<i16>(h_at)).cast_unsigned() << 16;
        let low = ffx_bridge::x87::ftol(h, width);
        #[allow(clippy::cast_possible_truncation)] // movsx ax
        let low = i32::from((low as u16).cast_signed()).cast_unsigned();
        low | high
    }

    /// `TODapsPrintf(fmt, ...; args)` with its frame at `ebp`: the text
    /// formatted into `ebp - 0x104` (the CRT's `vsprintf` with the
    /// caller's words from `args`) and shown by `TODapsString`; its `eax`.
    pub fn daps_printf(h: &mut impl Host, ebp: u32, fmt: u32, args: u32) -> u32 {
        let buf = ebp.wrapping_sub(0x104);
        crt(h, super::VSPRINTF_IAT, &[buf, fmt, args]);
        cdecl(h, super::DAPS_STRING, &[buf])
    }

    /// `FUN_008fc6a0(x, y, w, h, r, g, b, a)`: a flat rectangle at
    /// `g_TOPktCursor`: a GS tag (`0x8001`, `0x40034000`, `0x441e`, 0),
    /// `TOGsTEST(1, 1, 0 x6)`, the colour (`TOGsWords4(r, g, b, a)`) and
    /// the corners `(x, y)`, `(x + w, y + h)` (`TOGsXYZ2Offset`), then
    /// the tail jump to `TOCtrlPkt`.
    pub fn flat_rect(h: &mut impl Host, args: [u32; 8]) -> u32 {
        let [x, y, w, hh, r, g, b, a] = args;
        let cell = h.rebase(PKT_CURSOR);
        let p: u32 = h.read(cell);
        let p = put_words(h, p, &[0x8001, 0x4003_4000, 0x441e, 0]);
        h.write(cell, p);
        cdecl(h, super::GS_TEST, &[1, 1, 0, 0, 0, 0, 0, 0]);
        cdecl(h, super::GS_WORDS4, &[r, g, b, a]);
        cdecl(h, super::GS_XYZ2_OFFSET, &[x, y]);
        let last = cdecl(
            h,
            super::GS_XYZ2_OFFSET,
            &[w.wrapping_add(x), hh.wrapping_add(y)],
        );
        ctrl_pkt(h, last)
    }

    /// `TOTransImageLH(page, a1, a2, a3, a4, a5, a6)` with its (16-byte
    /// aligned) frame at `ebp`: a store-image transfer set up at `ebp -
    /// 0x70` (`sceGsSetDefStoreImage(buf, a1, a5 / 64, a2, a3, a4, a5,
    /// a6)`, signed division), `FlushCache(0)`, `sceGsSyncPath(0, 0)`,
    /// `sceGsExecStoreImage(buf, page)`, `sceGsSyncPath(0, 0)`; its `eax`.
    pub fn trans_image_lh(h: &mut impl Host, ebp: u32, args: [u32; 7]) -> u32 {
        let [page, a1, a2, a3, a4, a5, a6] = args;
        let buf = ebp.wrapping_sub(0x70);
        let blocks = a5.cast_signed().wrapping_div(64).cast_unsigned();
        cdecl(
            h,
            super::SET_STORE_IMAGE,
            &[buf, a1, blocks, a2, a3, a4, a5, a6],
        );
        cdecl(h, super::GS_FLUSH_CACHE, &[0]);
        cdecl(h, super::GS_SYNC_PATH, &[0, 0]);
        cdecl(h, super::EXEC_STORE_IMAGE, &[buf, page]);
        cdecl(h, super::GS_SYNC_PATH, &[0, 0])
    }
    /// `FUN_008a92a0()` with its frame at `ebp`: the `RealFrameBuffer`
    /// picture as a full-screen quad (`(0, 0)`, `(512, 416)` from
    /// `0x00b41d9c`/`0x00b41d98`, UVs (0, 1), (1, 0), the colour words
    /// `0x00c59548` (rgb) and `0x00c5954c` (alpha), `+0x80` =
    /// `0x00641350()`), drawn by `graphicDrawUIElement(quad, name, 1, 0,
    /// 0)`; its `eax`.
    /// junk read: the quad's vertices 2-3 and +0x84.. never written; 0
    /// (brief 2026-10-24)
    pub fn real_frame_buffer(h: &mut impl Host, ebp: u32) -> u32 {
        use ffx_bridge::x87;
        let extra = cdecl(h, super::FRAME_BUFFER_ARG, &[]);
        let rgb = global(h, super::FRAME_BUFFER_RGB);
        let q = ebp.wrapping_sub(QUAD);
        let one = 1.0f32.to_bits();
        put_words(h, q, &[0, 0, 0, one]);
        h.write(ebp.wrapping_sub(0x1c), extra);
        let alpha = global(h, super::FRAME_BUFFER_ALPHA);
        let right = x87::store(x87::load(global(h, super::FRAME_BUFFER_W)));
        let bottom = x87::store(x87::load(global(h, super::FRAME_BUFFER_H)));
        put_words(h, q.wrapping_add(0x20), &[right, bottom, one, 0]);
        put_words(h, q.wrapping_add(0x10), &[rgb, rgb, rgb, alpha]);
        put_words(h, q.wrapping_add(0x30), &[rgb, rgb, rgb, alpha]);
        let name = h.rebase(super::FRAME_BUFFER_NAME);
        cdecl(h, super::DRAW_UI_ELEMENT, &[q, name, 1, 0, 0])
    }

    /// `FUN_008adaf0(x, y)`: with the save menu's flag `0x200` or the
    /// option `0x00c59558` at 1, the screen captured: flag `0x01841d40`
    /// set; in a display mode (`Sg_GetDispMode()` nonzero) the capture
    /// buffer laid out (`0x01865f0c..18` from `0x01865f08`, two log
    /// lines), and unless the mode is 6 the screen stored
    /// (`TOTransImageLH(buffer, 0x2100, 0x1b, 0, 0, 0x200, 0x80)`, flags
    /// `0x01841d41`/`42`), `TOSetMessageWindowMode(1)` and `(x, y)` kept as
    /// halfwords. Returns the `eax` the original leaves: `MsGetSaveMenu`'s,
    /// the mode, or the window mode call's with `y`'s halfword over it.
    pub fn capture_screen(h: &mut impl Host, x: u32, y: u32) -> u32 {
        let menu = cdecl(h, super::SAVE_MENU, &[]);
        if h.read::<u32>(menu) & 0x200 == 0 && global(h, super::CAPTURE_OPTION) != 1 {
            return menu;
        }
        let [taken, stored, shown] = super::CAPTURE_FLAGS.map(|s| h.rebase(s));
        h.write(taken, 1u8);
        let mode = cdecl(h, super::DISP_MODE, &[]);
        if mode == 0 {
            return 0;
        }
        let [fmt, m1, m2] = super::CAPTURE_MSG.map(|s| h.rebase(s));
        cdecl(h, DBG_PRINTF, &[fmt, m1]);
        cdecl(h, DBG_PRINTF, &[fmt, m2]);
        let [base, buf, size, end, total] = super::CAPTURE_BUF.map(|s| h.rebase(s));
        let b: u32 = h.read(base);
        h.write(buf, b);
        h.write(size, 0x2_0000u32);
        h.write(total, 0x4_0000u32);
        h.write(end, b.wrapping_add(0x2_0000));
        let mode = cdecl(h, super::DISP_MODE, &[]);
        if mode == 6 {
            return 6;
        }
        let image: u32 = h.read(buf);
        h.write(stored, 1u8);
        cdecl(
            h,
            super::TRANS_IMAGE_LH,
            &[image, 0x2100, 0x1b, 0, 0, 0x200, 0x80],
        );
        h.write(shown, 1u8);
        let r = cdecl(h, super::MESSAGE_WINDOW_MODE, &[1]);
        let [at_x, at_y] = super::CAPTURE_XY.map(|s| h.rebase(s));
        #[allow(clippy::cast_possible_truncation)] // the halfwords
        let (x, y) = (x as u16, y as u16);
        h.write(at_x, x);
        h.write(at_y, y);
        (r & 0xffff_0000) | u32::from(y)
    }
    /// `printf2(fmt, args)` with its frame at `ebp`: the line formatted
    /// (`vsprintf(ebp - 0x204, fmt, args)`) and its first 39 bytes kept in
    /// the 128-line ring; shown (`dbgPrintf`) when the debug output is on
    /// (`sceGsSetDefDrawEnv2() == 1`, else the config's byte `+0xb`) and,
    /// with a filter (`0x01129fe8`), whether the filter is a proper prefix
    /// of the line equals its sense (`i8 0x01128ac1`); sent on by mode
    /// `0x01128aaf` (3: `0x00888160("FFXDEBUG", line, len)`; 4, debug on:
    /// `printf2_dump(line, len)`; empty lines not). `0x01128aae` = 1.
    /// Returns the `eax` the original leaves.
    pub fn printf2(h: &mut impl Host, ebp: u32, fmt: u32, args: u32) -> u32 {
        let buf = ebp.wrapping_sub(0x204);
        crt(h, super::VSPRINTF_IAT, &[buf, fmt, args]);
        let index = global(h, super::LOG_INDEX);
        let line = h
            .rebase(super::LOG_RING)
            .wrapping_add(index.wrapping_mul(40));
        crt(h, super::STRNCPY_IAT, &[line, buf, 0x27]);
        h.write(line.wrapping_add(0x27), 0u8);
        let index = global(h, super::LOG_INDEX).wrapping_add(1) & 0x7f;
        let at = h.rebase(super::LOG_INDEX);
        h.write(at, index);
        let on = if cdecl(h, super::DEBUG_SWITCH, &[]) == 1 {
            true
        } else {
            let config = cdecl(h, super::FFX_CONFIG, &[]);
            h.read::<u8>(config.wrapping_add(0xb)) != 0
        };
        if on {
            let filter = h.rebase(super::LOG_FILTER);
            let show = if h.read::<u8>(filter) == 0 {
                true
            } else {
                let sense = i32::from(h.read::<i8>(h.rebase(super::LOG_FILTER_SENSE)));
                i32::from(proper_prefix(h, filter, line)) == sense
            };
            if show {
                cdecl(h, DBG_PRINTF, &[buf]);
            }
        }
        let mode = i32::from(h.read::<i8>(h.rebase(super::LOG_MODE)));
        let mut eax = mode.wrapping_sub(4).cast_unsigned();
        if mode == 3 {
            eax = 0;
            let len = strlen(h, line);
            if len != 0 {
                let tag = h.rebase(super::LOG_SEND_TAG);
                eax = cdecl(h, super::LOG_SEND, &[tag, line, len]);
            }
        } else if mode == 4 {
            eax = cdecl(h, super::DEBUG_SWITCH, &[]);
            if eax == 1 {
                eax = 0;
                let len = strlen(h, line);
                if len != 0 {
                    eax = cdecl(h, super::LOG_DUMP, &[line, len]);
                }
            }
        }
        let at = h.rebase(super::LOG_WRITTEN);
        h.write(at, 1u8);
        eax
    }

    /// Whether the string at `filter` is a proper prefix of the nonempty
    /// string at `line` (`printf2`'s filter test).
    fn proper_prefix(h: &impl Host, filter: u32, line: u32) -> bool {
        if h.read::<u8>(line) == 0 {
            return false;
        }
        let mut k = 0u32;
        loop {
            let f: u8 = h.read(filter.wrapping_add(k));
            if f == 0 {
                return true;
            }
            if h.read::<u8>(line.wrapping_add(k)) != f {
                return false;
            }
            k = k.wrapping_add(1);
            if h.read::<u8>(line.wrapping_add(k)) == 0 {
                return false;
            }
        }
    }
    /// `i32` division as `idiv` does it; the original faults on a zero
    /// divisor, the port gives 0 there.
    fn idiv(a: u32, b: u32) -> u32 {
        a.cast_signed()
            .checked_div(b.cast_signed())
            .map_or(0, i32::cast_unsigned)
    }

    /// `TODrawBoxXYWHC2FW(x, y, w, h, c0, c1, full)` with its frame at
    /// `ebp`: a box `(x, y)`..`(x + w, y + h)` shaded from `c0` (its alpha
    /// byte too) to the colour `w / full` of the way to `c1` (each of r,
    /// g, b: `c0 + (c1 - c0) * w / full`, signed; alpha 0x80), drawn by
    /// `graphicDrawUIElement(quad, 0, 1, 0, 0)`. Its `eax`.
    /// junk read: the quad's UVs and vertices 2-3 never written; 0 (brief
    /// 2026-10-24)
    pub fn box_fill_width(h: &mut impl Host, ebp: u32, args: [u32; 7]) -> u32 {
        let [x, y, w, hh, c0, c1, full] = args;
        let q = ebp.wrapping_sub(QUAD);
        let channel = |c: u32, k: u32| (c >> (8 * k)) & 0xff;
        h.write(q, int_f32(x).to_bits());
        h.write(q.wrapping_add(4), int_f32(y).to_bits());
        h.write(q.wrapping_add(0x1c), c0 >> 24);
        let [r0, g0, b0] = [0, 1, 2].map(|k| channel(c0, k));
        h.write(ebp.wrapping_sub(0xa0), b0);
        h.write(q.wrapping_add(0x18), b0);
        h.write(ebp.wrapping_sub(0xa4), g0);
        h.write(q.wrapping_add(0x14), g0);
        for off in [0x14u32, 0xc, 8] {
            h.write(ebp.wrapping_sub(off), 0u32);
        }
        h.write(q.wrapping_add(0x10), r0);
        h.write(q.wrapping_add(0x3c), 0x80u32);
        for (k, base) in [(0u32, r0), (1, g0), (2, b0)] {
            let to = channel(c1, k);
            let v = idiv(to.wrapping_sub(base).wrapping_mul(w), full).wrapping_add(base);
            h.write(q.wrapping_add(0x30 + 4 * k), v);
        }
        h.write(ebp.wrapping_sub(0xa0), x.wrapping_add(w));
        h.write(q.wrapping_add(0x20), int_f32(x.wrapping_add(w)).to_bits());
        let bottom = y.wrapping_add(hh);
        h.write(ebp.wrapping_sub(0xa0), bottom);
        h.write(q.wrapping_add(0x24), int_f32(bottom).to_bits());
        cdecl(h, super::DRAW_UI_ELEMENT, &[q, 0, 1, 0, 0])
    }

    /// `FUN_00774520(x, y, w, h, texture)` with its frame at `ebp`: a
    /// textured quad `(x, y)`..`(x + w, y + h)` in grey (0x80 x4), UVs
    /// `0 / w`, `0 / h` and `w / w - inset`, `h / h - inset` (`idiv`, as
    /// floats; the inset f64 `0x00b4c168`), drawn by
    /// `graphicDrawUIElement(quad, texture, 1, 0, 0)`. Its `eax`.
    /// junk read: vertices 2-3 never written; 0 (brief 2026-10-24)
    pub fn ui_textured(h: &mut impl Host, ebp: u32, args: [u32; 5]) -> u32 {
        use ffx_bridge::x87;
        let [x, y, w, hh, texture] = args;
        let q = ebp.wrapping_sub(QUAD);
        let tmp = ebp.wrapping_sub(0xa0);
        h.write(q, int_f32(x).to_bits());
        h.write(q.wrapping_add(4), int_f32(y).to_bits());
        for off in [0xcu32, 8, 0x14] {
            h.write(ebp.wrapping_sub(off), 0u32);
        }
        put_words(h, q.wrapping_add(0x10), &[0x80; 4]);
        let u0 = idiv(0, w);
        h.write(tmp, u0);
        h.write(q.wrapping_add(8), int_f32(u0).to_bits());
        let v0 = idiv(0, hh);
        h.write(tmp, v0);
        h.write(q.wrapping_add(0xc), int_f32(v0).to_bits());
        let x1 = x.wrapping_add(w);
        h.write(tmp, x1);
        h.write(q.wrapping_add(0x20), int_f32(x1).to_bits());
        let y1 = y.wrapping_add(hh);
        h.write(tmp, y1);
        h.write(q.wrapping_add(0x24), int_f32(y1).to_bits());
        let inset = f64_global(h, super::UV_INSET);
        let u1 = int_f32(idiv(w, w)).to_bits();
        h.write(tmp, u1);
        h.write(
            q.wrapping_add(0x28),
            x87::store(x87::sub(x87::load(u1), inset)),
        );
        let v1 = int_f32(idiv(hh, hh)).to_bits();
        h.write(tmp, v1);
        h.write(
            q.wrapping_add(0x2c),
            x87::store(x87::sub(x87::load(v1), inset)),
        );
        put_words(h, q.wrapping_add(0x30), &[0x80; 4]);
        cdecl(h, super::DRAW_UI_ELEMENT, &[q, texture, 1, 0, 0])
    }
    /// The battle plate's layout halfword at `0x025d0aa2 + off`.
    fn plate(h: &impl Host, off: u32) -> u16 {
        h.read(h.rebase(super::PLATE_LAYOUT).wrapping_add(off))
    }

    /// A 16-bit value sign-extended, as `movsx`/`cwde` push it.
    fn sx(v: u16) -> u32 {
        i32::from(v.cast_signed()).cast_unsigned()
    }

    /// `TOBtlDrawLimitGauge(chr, x, y; slots)`: the gauge's frame
    /// (`TOMakePktBoxXYWHRGBA(x, y, +0x10, +0xe, 0, 0, 0, 0x80)` of the
    /// layout) and, unless the gauge's maximum or value is 0, its bar at
    /// `(x + layout[6] - 1, y + layout[4])`: full
    /// (`TOMakePktBoxXYWHRGBA(.., layout[2], layout[0], 0xf7, 0x79, 0x2e,
    /// 0x80)`) when the value reaches the maximum, else
    /// `TODrawBoxXYWHC2FW` with the width `ftol(value * layout[2] / max)`
    /// (x87) shaded over the full width `layout[2]`. The original keeps
    /// 16-bit sums in its argument slots (`x`'s high half the layout's
    /// `+0`, `y`'s the bar's `y`; the shaded bar overwrites `y`'s slot
    /// with `layout[2]`). Returns the bar's `eax`; without a bar the
    /// original returns the FPU status word in `ax` (`None`).
    pub fn limit_gauge(h: &mut impl Host, chr: u32, x: u32, y: u32, slots: u32) -> Option<u32> {
        #[allow(clippy::cast_possible_truncation)] // the 16-bit coordinates
        let (x16, y16) = (x as u16, y as u16);
        let [l0, l2, l4, l6, le, l10] = [0, 2, 4, 6, 0xe, 0x10].map(|off| plate(h, off));
        cdecl(
            h,
            super::BOX_RGBA,
            &[sx(x16), sx(y16), sx(l10), sx(le), 0, 0, 0, 0x80],
        );
        let bar_y = l4.wrapping_add(y16);
        let bar_x = l6.wrapping_sub(1).wrapping_add(x16);
        h.write(slots.wrapping_add(0xa), bar_y);
        h.write(slots.wrapping_add(6), l0);
        let max = cdecl(h, super::LIMIT_MAX, &[chr]).cast_signed();
        let now = cdecl(h, super::LIMIT_NOW, &[chr]).cast_signed();
        if max == 0 || now == 0 {
            return None;
        }
        if max <= now {
            return Some(cdecl(
                h,
                super::BOX_RGBA,
                &[sx(bar_x), sx(bar_y), sx(l2), sx(l0), 0xf7, 0x79, 0x2e, 0x80],
            ));
        }
        let full = sx(l2);
        h.write(slots.wrapping_add(8), full);
        let width = ffx_bridge::x87::div(
            ffx_bridge::x87::mul(f64::from(now), f64::from(full.cast_signed())),
            f64::from(max),
        );
        let width = ffx_bridge::x87::ftol(h, width);
        Some(cdecl(
            h,
            super::BOX_FILL_WIDTH,
            &[
                sx(bar_x),
                sx(bar_y),
                width,
                sx(l0),
                0x808a_f8ff,
                0x8017_77e3,
                full,
            ],
        ))
    }

    /// `TOBtlDrawParamPlate(chr, x, y; slots)`: the plate's frame
    /// (`TOMakePktBoxXYWHRGBA` with the layout's size and colour bytes),
    /// the HP icon `0xe9`, the HP (`TOMkpValueFontMClutRight`, its colour
    /// from `MsCalcWeakLevel(hp, max)`: 1-2 give 2, 3 gives 0x53, others
    /// 0), the MP icon `0xea` and value, then `TOBtlDrawLimitGauge`; all
    /// at the layout's offsets (16-bit sums; the HP's `y` kept in the high
    /// half of `y`'s argument slot). The gauge's `eax`.
    pub fn param_plate(h: &mut impl Host, chr: u32, x: u32, y: u32, slots: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the 16-bit coordinates
        let (x16, y16) = (x as u16, y as u16);
        let colours = [0x26u32, 0x27, 0x28, 0x29]
            .map(|k| u32::from(h.read::<u8>(h.rebase(super::PLATE_LAYOUT).wrapping_add(k))));
        let (w, hh) = (plate(h, 0x2c), plate(h, 0x2a));
        cdecl(
            h,
            super::BOX_RGBA,
            &[
                sx(x16),
                sx(y16),
                sx(w),
                sx(hh),
                colours[3],
                colours[2],
                colours[1],
                colours[0],
            ],
        );
        let hp_x = plate(h, 0x20).wrapping_add(x16);
        let hp_y = plate(h, 0x1e).wrapping_add(y16);
        h.write(slots.wrapping_add(0xa), hp_y);
        let icon = [
            plate(h, 0x24).wrapping_add(x16),
            plate(h, 0x22).wrapping_add(y16),
        ];
        cdecl(h, super::MKP_SHAPE_XY, &[0xe9, sx(icon[0]), sx(icon[1])]);
        let hp_max = cdecl(h, super::HP_MAX, &[chr]);
        let hp = cdecl(h, super::HP_NOW, &[chr]);
        let colour = match cdecl(h, super::WEAK_LEVEL, &[hp, hp_max]) {
            1 | 2 => 2,
            3 => 0x53,
            _ => 0,
        };
        let hp = cdecl(h, super::HP_NOW, &[chr]);
        let hp_y: u16 = h.read(slots.wrapping_add(0xa));
        cdecl(
            h,
            super::VALUE_FONT_M_CLUT_RIGHT,
            &[hp, sx(hp_x), sx(hp_y), colour],
        );
        let mp_icon = [
            plate(h, 0x1c).wrapping_add(x16),
            plate(h, 0x1a).wrapping_add(y16),
        ];
        cdecl(
            h,
            super::MKP_SHAPE_XY,
            &[0xea, sx(mp_icon[0]), sx(mp_icon[1])],
        );
        let mp = cdecl(h, super::MP_NOW, &[chr]);
        let mp_at = [
            plate(h, 0x18).wrapping_add(x16),
            plate(h, 0x16).wrapping_add(y16),
        ];
        cdecl(
            h,
            super::VALUE_FONT_M_CLUT_RIGHT,
            &[mp, sx(mp_at[0]), sx(mp_at[1]), 0],
        );
        let gauge = [
            plate(h, 0x14).wrapping_add(x16),
            plate(h, 0x12).wrapping_add(y16),
        ];
        cdecl(h, super::LIMIT_GAUGE, &[chr, sx(gauge[0]), sx(gauge[1])])
    }
    /// `FUN_008f2e10(cursor, page, x, y, w, h)`: a textured sprite packet
    /// at `cursor` (a GS tag with the alpha/test registers, `TOAdpTEX0(..,
    /// page, 8, 0, 9, 7, 1, 0 x5, 2)`), then four corners, each grey
    /// (`TOAdpWords4(0x80 x4)`), its UV (`TOAdpXY2`: (0, 0), (0x200, 0),
    /// (0, 0x80), (0x200, 0x80)) and screen point (`TOAdpXYZ2Offset`: `(x,
    /// y)`, `(x + w, y)`, `(x, y + h)`, `(x + w, y + h)`). Returns the
    /// packet's end.
    pub fn movie_image(h: &mut impl Host, args: [u32; 6]) -> u32 {
        let [cursor, page, x, y, w, hh] = args;
        let q = put_words(
            h,
            cursor,
            &[
                0x8001,
                0xf08e_4000,
                0x3143_1eee,
                0x0431_4314,
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
        let mut q = h.call_sym_wide(super::ADP_TEX0, &[q, page, 8, 0, 9, 7, 1, 0, 0, 0, 0, 0, 2]);
        let (right, bottom) = (w.wrapping_add(x), y.wrapping_add(hh));
        for (uv, at) in [
            ([0, 0], [x, y]),
            ([0x200, 0], [right, y]),
            ([0, 0x80], [x, bottom]),
            ([0x200, 0x80], [right, bottom]),
        ] {
            q = cdecl(h, super::ADP_WORDS4, &[q, 0x80, 0x80, 0x80, 0x80]);
            q = cdecl(h, super::ADP_XY2, &[q, uv[0], uv[1]]);
            q = cdecl(h, super::ADP_XYZ2_OFFSET, &[q, at[0], at[1]]);
        }
        q
    }

    /// `FUN_008ebc00(cursor, x, y, w, r, g, b, a; slots)`: a three-piece
    /// bar at `cursor` / `g_TOPktCursor`: a GS tag,
    /// `TOGsALPHA(0, 1, 0, 1, 0x80)`, the test/texture registers and a
    /// sprite tag, then the left cap, the middle and the right cap (each
    /// `TOGsWords4` of the colour bytes, `TOGsXY2`/`TOGsXYZ2Offset`
    /// corners: UVs 0x67..0x75, 0x75..0x78, 0x79..0x85 by 0..0x13; screen
    /// x from `x - 0xe` to `x`, `x` to `x + w`, `x + w` to `x + w + 0xc`,
    /// y from `y - 3` to `y + 0x10`). The colour bytes go back into their
    /// argument slots zero-extended, `y - 3` into the cursor's. Returns
    /// `g_TOPktCursor`.
    pub fn bar_caps(h: &mut impl Host, args: [u32; 8], slots: u32) -> u32 {
        let [cursor, x, y, w, ..] = args;
        let cell = h.rebase(PKT_CURSOR);
        let p = put_words(h, cursor, &[1, 0x3000_0000, 0xeee, 0]);
        h.write(cell, p);
        cdecl(h, super::GS_ALPHA, &[0, 1, 0, 1, 0x80]);
        let q: u32 = h.read(cell);
        let colour = [
            args[4] & 0xff,
            args[5] & 0xff,
            args[6] & 0xff,
            args[7] & 0xff,
        ];
        let q = put_words(
            h,
            q,
            &[
                0xf,
                0,
                0x47,
                0,
                0x2131_3e00,
                0x4005_d406,
                6,
                0,
                0x8003,
                0x50af_4000,
                0x0004_3431,
                0,
            ],
        );
        h.write(cell, q);
        for (k, c) in [4u32, 5, 6, 7].into_iter().zip(colour) {
            h.write(slots.wrapping_add(4 * k), c);
        }
        let top = y.wrapping_sub(3);
        let low = y.wrapping_add(0x10);
        let right = w.wrapping_add(x);
        let pieces = [
            ([0x67, 0x75], [x.wrapping_sub(0xe), x]),
            ([0x75, 0x78], [x, right]),
            ([0x79, 0x85], [right, right.wrapping_add(0xc)]),
        ];
        for (i, (u, sx)) in pieces.into_iter().enumerate() {
            let colour =
                [0x10u32, 0x14, 0x18, 0x1c].map(|off| h.read::<u32>(slots.wrapping_add(off)));
            cdecl(h, super::GS_WORDS4, &colour);
            cdecl(h, super::GS_XY2, &[u[0], 0]);
            if i == 0 {
                h.write(slots, top);
            }
            let top: u32 = h.read(slots);
            cdecl(h, super::GS_XYZ2_OFFSET, &[sx[0], top]);
            cdecl(h, super::GS_XY2, &[u[1], 0x13]);
            cdecl(h, super::GS_XYZ2_OFFSET, &[sx[1], low]);
        }
        h.read(cell)
    }
    /// `TOAdpEasyMesFontLClutRGBAInterMode(cursor, text, x, y, flag, r, g,
    /// b, a, kind, scale; slots)`: the glyph record cleared, the pen set
    /// (`(x, y)` as floats, the colour bytes, flags 0 and `flag`),
    /// `g_TOPktCursor` = `cursor`; then per character of `text`: its glyph
    /// (`0x008b78b0(text, record, kind)`), with a positive `scale` the
    /// record's scales multiplied by `base / scale` (`0x00b922e0`) and the
    /// passes drawn with 4 and 5, else both with `scale`; the pen moved on
    /// (positive scale: `(spacing + 1) * base / scale + scale_x * width *
    /// 0x00b48320`; else `scale_x * width + 1 + spacing`), x87. `f32
    /// scale` goes into the cursor's argument slot. Returns
    /// `g_TOPktCursor`.
    pub fn mes_font_inter(h: &mut impl Host, args: [u32; 11], slots: u32) -> u32 {
        use ffx_bridge::x87;
        let [cursor, text, x, y, flag, r, g, b, a, kind, scale] = args;
        let record = h.rebase(super::GLYPH_RECORD);
        cdecl(h, super::CRT_MEMSET, &[record, 0, 0x40]);
        let pen = h.rebase(super::PEN);
        for off in [0u32, 0x18, 4] {
            h.write(pen.wrapping_add(off), 0u32);
        }
        h.write(pen.wrapping_add(4), int_f32(x).to_bits());
        for off in [8u32, 0xc, 0x10, 0x14] {
            h.write(pen.wrapping_add(off), 0u32);
        }
        h.write(pen.wrapping_add(8), int_f32(y).to_bits());
        for (off, c) in [(0u32, r), (1, g), (2, b), (3, a), (0x1b, flag)] {
            h.write(pen.wrapping_add(off), c.to_le_bytes()[0]);
        }
        h.write(pen.wrapping_add(0x1a), 0u8);
        let cell = h.rebase(PKT_CURSOR);
        h.write(cell, cursor);
        if h.read::<u8>(text) == 0 {
            return cursor;
        }
        let base = f64_global(h, super::SCALE_BASE);
        let scaled = scale.cast_signed() > 0;
        let mut text = text;
        loop {
            cdecl(h, super::GLYPH_LOOKUP, &[text, record, kind]);
            if scaled {
                let sf = int_f32(scale).to_bits();
                h.write(slots, sf);
                let k = x87::div(base, x87::load(sf));
                let sx = record.wrapping_add(0x1c);
                let v = x87::mul(x87::load(h.read(sx)), k);
                h.write(sx, x87::store(v));
                let sy = record.wrapping_add(0x20);
                let v = x87::mul(k, x87::load(h.read(sy)));
                h.write(sy, x87::store(v));
                cdecl(h, super::PASS_EDGE, &[pen, record, 4]);
                cdecl(h, super::PASS_FONT, &[pen, record, 5]);
            } else {
                cdecl(h, super::PASS_EDGE, &[pen, record, scale]);
                cdecl(h, super::PASS_FONT, &[pen, record, scale]);
            }
            let spacing = x87::load(global(h, super::FONT_SPACING));
            let width = x87::load(global(h, super::WIDTH_FACTOR));
            let advance = if scaled {
                let sf = int_f32(scale).to_bits();
                h.write(slots, sf);
                let k = x87::div(base, x87::load(sf));
                let lead = x87::mul(x87::add(spacing, 1.0), k);
                let scale_x = x87::load(h.read(record.wrapping_add(0x1c)));
                let wide = x87::mul(
                    x87::mul(scale_x, width),
                    f64_global(h, super::ADVANCE_FACTOR),
                );
                x87::add(lead, wide)
            } else {
                let scale_x = x87::load(h.read(record.wrapping_add(0x1c)));
                x87::add(x87::add(x87::mul(scale_x, width), 1.0), spacing)
            };
            let px = pen.wrapping_add(4);
            let v = x87::add(advance, x87::load(h.read(px)));
            h.write(px, x87::store(v));
            text = cdecl(h, super::NEXT_CHAR, &[text]);
            if h.read::<u8>(text) == 0 {
                return h.read(cell);
            }
        }
    }
    /// `TOMakePktMovieFont1612Font(pos, glyph; slots)`: a glyph of the
    /// movie font at `g_TOPktCursor`: a GS tag, `TOGsTEST(1, 7, 0 x6)`,
    /// `TOGsALPHA(0, 1, 0, 1, 0x80)`, two `TOGsTEX0` (the page 0x2eb8 or,
    /// for the low nibble of `pos + 0x1b` from 8, 0x2ebc with the nibble
    /// less 8; the CLUT row `i16 glyph +4` plus twice the nibble), the
    /// colour (`pos` bytes 0..3, alpha 0x60) and the corners (UVs from
    /// the glyph's floats, screen `pos` plus the glyph's offsets, the
    /// first with Z 1), then the tail jump to `TOCtrlPkt`. The page goes
    /// into `pos`'s argument slot.
    pub fn movie_font_glyph(h: &mut impl Host, pos: u32, glyph: u32, slots: u32) -> u32 {
        use ffx_bridge::x87;
        let cell = h.rebase(PKT_CURSOR);
        let p: u32 = h.read(cell);
        let p = put_words(h, p, &[0x8001, 0x90ab_4000, 0x3431_eeee, 4]);
        h.write(cell, p);
        let mut nibble = u32::from(h.read::<u8>(pos.wrapping_add(0x1b))) & 0xf;
        let page = if nibble < 8 {
            0x2eb8
        } else {
            nibble -= 8;
            0x2ebc
        };
        h.write(slots, page);
        cdecl(h, super::GS_TEST, &[1, 7, 0, 0, 0, 0, 0, 0]);
        cdecl(h, super::GS_ALPHA, &[0, 1, 0, 1, 0x80]);
        let g0 = i16_at(h, glyph, 0);
        cdecl(
            h,
            super::GS_TEX0,
            &[g0, 2, 0x13, 7, 0xa, 1, 0, page, 0, 0, 0, 1],
        );
        let row = i16_at(h, glyph, 4).wrapping_add(nibble * 2);
        let g2 = i16_at(h, glyph, 2);
        let page: u32 = h.read(slots);
        let g0 = i16_at(h, glyph, 0);
        cdecl(
            h,
            super::GS_TEX0,
            &[g0, 2, g2, 7, 0xa, 1, 0, page, 0, 0, row, 0],
        );
        let colour = [0u32, 1, 2].map(|k| u32::from(h.read::<u8>(pos.wrapping_add(k))));
        cdecl(
            h,
            super::GS_WORDS4,
            &[colour[0], colour[1], colour[2], 0x60],
        );
        let (u, v) = (
            f32_at(h, glyph.wrapping_add(8)),
            f32_at(h, glyph.wrapping_add(0xc)),
        );
        let (u, v) = (ftol(h, u), ftol(h, v));
        cdecl(h, super::GS_XY2, &[u, v]);
        let y = x87::add(
            f32_at(h, pos.wrapping_add(8)),
            f32_at(h, glyph.wrapping_add(0x18)),
        );
        let y = ftol(h, y);
        let x = ftol(h, f32_at(h, pos.wrapping_add(4)));
        cdecl(h, super::GS_XYZ_OFFSET, &[x, y, 1]);
        let (u, v) = (
            f32_at(h, glyph.wrapping_add(0x10)),
            f32_at(h, glyph.wrapping_add(0x14)),
        );
        let (u, v) = (ftol(h, u), ftol(h, v));
        cdecl(h, super::GS_XY2, &[u, v]);
        let y = x87::add(
            x87::add(
                f32_at(h, pos.wrapping_add(8)),
                f32_at(h, glyph.wrapping_add(0x18)),
            ),
            f32_at(h, glyph.wrapping_add(0x20)),
        );
        let y = ftol(h, y);
        let x = ftol(
            h,
            x87::add(
                f32_at(h, glyph.wrapping_add(0x1c)),
                f32_at(h, pos.wrapping_add(4)),
            ),
        );
        let r = cdecl(h, super::GS_XYZ2_OFFSET, &[x, y]);
        ctrl_pkt(h, r)
    }
    /// `FUN_008f1b50(cursor, x, y, kind)`: for kinds 0..3, 8..11, 16..19
    /// and 24..27 (the original's switch), the shape builder `kind & 3`
    /// (`0x008f1af0`, `0x008f1a90`, `0x008f1990`, `0x008f1a10`) called
    /// with `(cursor, x, y, 0xd1)` and colour pair `kind >> 3`; its
    /// cursor. Other kinds return `cursor`.
    pub fn shape_by_kind(h: &mut impl Host, args: [u32; 4]) -> u32 {
        let [cursor, x, y, kind] = args;
        if kind > 0x1b || kind & 4 != 0 {
            return cursor;
        }
        let pair = h.rebase(super::KIND_COLOURS[(kind >> 3) as usize]);
        let second: u32 = h.read(pair.wrapping_sub(4));
        let first: u32 = h.read(pair);
        let shape = super::KIND_SHAPES[(kind & 3) as usize];
        cdecl(h, shape, &[cursor, x, y, 0xd1, first, second])
    }
    /// `TODrawCrossBoxXYWHC2FW(x, y, w, h, c0, c1, full)` (floats `x`, `y`,
    /// `w`, `h`) with its frame at `ebp`: as `TODrawBoxXYWHC2FW` in float
    /// coordinates: vertex 1's r, g, b = `ftol((c1 - c0) * w / f32(full) +
    /// c0)` per channel (each difference and base as `f32`, x87), alpha
    /// 0x80; the corners `(x, y)`, `(x + w, y + h)`. The temporaries the
    /// original leaves in its frame (`ebp - 0xa0..0xb0`) are written too.
    /// Its `eax`.
    /// junk read: the quad's UVs and vertices 2-3 never written; 0 (brief
    /// 2026-10-24)
    pub fn cross_box_fill_width(h: &mut impl Host, ebp: u32, args: [u32; 7]) -> u32 {
        use ffx_bridge::x87;
        let [x, y, w, hh, c0, c1, full] = args;
        let q = ebp.wrapping_sub(QUAD);
        let (fx, fy) = (x87::load(x), x87::load(y));
        h.write(q, x87::store(fx));
        h.write(q.wrapping_add(4), x87::store(fy));
        let channel = |c: u32, k: u32| (c >> (8 * k)) & 0xff;
        h.write(q.wrapping_add(0x1c), c0 >> 24);
        for off in [0x14u32, 0xc, 8] {
            h.write(ebp.wrapping_sub(off), 0u32);
        }
        let span_bits = int_f32(full).to_bits();
        let span = x87::load(span_bits);
        let width = x87::load(w);
        let mut bases = [0u32; 3];
        let mut diffs = [0u32; 3];
        let mut out = [0u32; 3];
        for k in 0..3u32 {
            let base = channel(c0, k);
            h.write(q.wrapping_add(0x10 + 4 * k), base);
            let i = k as usize;
            bases[i] = int_f32(base).to_bits();
            diffs[i] = int_f32(channel(c1, k).wrapping_sub(base)).to_bits();
            let scaled = x87::div(x87::mul(x87::load(diffs[i]), width), span);
            out[i] = ftol(h, x87::add(scaled, x87::load(bases[i])));
        }
        // the frame's temporaries as the original leaves them
        for (off, v) in [
            (0xb0u32, span_bits),
            (0xa8, diffs[0]),
            (0xa0, diffs[1]),
            (0xa4, diffs[2]),
            (0xac, bases[2]),
        ] {
            h.write(ebp.wrapping_sub(off), v);
        }
        put_words(h, q.wrapping_add(0x30), &[out[0], out[1], out[2], 0x80]);
        h.write(q.wrapping_add(0x20), x87::store(x87::add(fx, width)));
        h.write(
            q.wrapping_add(0x24),
            x87::store(x87::add(fy, x87::load(hh))),
        );
        cdecl(h, super::DRAW_UI_ELEMENT, &[q, 0, 1, 0, 0])
    }
    /// A UI remap (`graphicUiRemapX2`/`Y2`) of the float `c`: its `ST0`.
    fn remap(h: &mut impl Host, f: u32, c: u32) -> f64 {
        h.call_sym_st0_f64(f, Conv::Cdecl, &[c])
    }

    /// `TODrawCrossMenuIconImportantName(item, x, y, colour; slots)` (`x`,
    /// `y` floats) with its frame at `ebp`: the key item's icon and name.
    /// The record (`MsImportantPtr(item, ebp - 4)`; name `u16 +0 + [ebp -
    /// 4]`, icon byte `+0x12`); the icon by `DrawCrossMenuIconXYWHRGBA(x +
    /// remapX(k4), y + remapY(k3), remapX(k2), remapY(k1), icon, 0x80
    /// x4)`, the name by `ToMakeBtlEasyFont(name, x + remapX(k7), y +
    /// remapY(k6), colour, k5, 1.0)` (`graphicUiRemapX2`/`Y2` of the
    /// constants, sums rounded to `f32`; both of the last two go through
    /// the `y` argument slot, which keeps the name's `x`; the icon's `x`
    /// stays at `ebp - 8`). The name call's
    /// `eax`.
    pub fn important_icon_name(h: &mut impl Host, ebp: u32, args: [u32; 4], slots: u32) -> u32 {
        use ffx_bridge::x87;
        let [item, x, y, colour] = args;
        let out = ebp.wrapping_sub(4);
        let record = cdecl(h, super::IMPORTANT_PTR, &[item, out]);
        let k = super::ICON_CONSTS.map(|s| global(h, s));
        let name = u32::from(h.read::<u16>(record)).wrapping_add(h.read::<u32>(out));
        let icon = u32::from(h.read::<u8>(record.wrapping_add(0x12)));
        let h1 = x87::store(remap(h, super::REMAP_Y2, k[0]));
        let w2 = x87::store(remap(h, super::REMAP_X2, k[1]));
        let y3 = x87::store(x87::add(remap(h, super::REMAP_Y2, k[2]), x87::load(y)));
        h.write(ebp.wrapping_sub(8), y3);
        let x4 = x87::store(x87::add(remap(h, super::REMAP_X2, k[3]), x87::load(x)));
        h.write(ebp.wrapping_sub(8), x4);
        cdecl(
            h,
            super::CROSS_MENU_ICON,
            &[x4, y3, w2, h1, icon, 0x80, 0x80, 0x80, 0x80],
        );
        let yy = x87::store(x87::add(remap(h, super::REMAP_Y2, k[5]), x87::load(y)));
        h.write(slots.wrapping_add(8), yy);
        let xx = x87::store(x87::add(remap(h, super::REMAP_X2, k[6]), x87::load(x)));
        h.write(slots.wrapping_add(8), xx);
        let scale = x87::store(x87::load(k[4]));
        cdecl(
            h,
            super::BTL_EASY_FONT,
            &[name, xx, yy, colour, scale, 1.0f32.to_bits()],
        )
    }
    /// The float at `sym` as `fld dword`/`fstp dword` copy it.
    fn float_copy(h: &impl Host, sym: u32) -> u32 {
        ffx_bridge::x87::store(ffx_bridge::x87::load(global(h, sym)))
    }

    /// `FUN_008a1e50()` (the Wakka limit window's frame): four pieces by
    /// `TOMkpShapeXYWHUV(-1, x, y, w, h, u0, v0, u1, v1)` with `x` =
    /// `graphicUiRemapX2(0x00b5ef1c)`, `y` = `graphicUiRemapY2(piece[1])`,
    /// `w` = `graphicUiRemapX2(0x00b5f434)`, `h` =
    /// `graphicUiRemapY2(piece[0])`, `u0` = 0, `v0` = `piece[2]`, `u1` =
    /// `0x00b5f434`, `v1` = `piece[3]` (floats; the original builds the
    /// words in place on its stack, the remaps in this order). The last
    /// shape's `eax`.
    pub fn wakka_frame(h: &mut impl Host) -> u32 {
        use ffx_bridge::x87;
        let mut r = 0;
        for piece in super::PIECES {
            let a = x87::store(remap(h, super::REMAP_Y2, float_copy(h, piece[0])));
            let height = float_copy(h, super::PIECE_H);
            let b = x87::store(remap(h, super::REMAP_X2, height));
            let c = x87::store(remap(h, super::REMAP_Y2, float_copy(h, piece[1])));
            let width = float_copy(h, super::PIECE_W);
            let d = x87::store(remap(h, super::REMAP_X2, width));
            let words = [
                u32::MAX,
                d,
                c,
                b,
                a,
                0,
                float_copy(h, piece[2]),
                float_copy(h, super::PIECE_H),
                float_copy(h, piece[3]),
            ];
            r = cdecl(h, super::SHAPE_XYWHUV, &words);
        }
        r
    }
    /// `TODrawMovieChangeMacro2(win, macro, pos; slots)`: the glyph record
    /// cleared, the macro's text (`TOGetChangeMacroAddress(i16 win +4,
    /// macro)`) drawn while the window's count (`+0x9c`) is below its
    /// limit (`+0xa0`): code 7 with `0xe0..0xef` after it moves `pos.x`
    /// by that less 0xe0 (through `win`'s argument slot as a float); a
    /// class-3 character (`0x008df610`) is a nested macro
    /// (`TODrawChangeMacro2(win, text, pos)`); others are glyphs
    /// (`TOGetFontInfo`, `TOMakePktMovieFont1612(pos, record)`), counted,
    /// and while neither `+0xb4` nor `+0xac` is set the limit grows up to
    /// `+0xa8`. Each step goes on with `0x008dfd80`. Returns the `eax` the
    /// original leaves.
    pub fn movie_change_macro(
        h: &mut impl Host,
        win: u32,
        macro_id: u32,
        pos: u32,
        slots: u32,
    ) -> u32 {
        use ffx_bridge::x87;
        let record = h.rebase(super::GLYPH_RECORD);
        cdecl(h, super::CRT_MEMSET, &[record, 0, 0x40]);
        let id = i16_at(h, win, 4);
        let mut text = cdecl(h, super::CHANGE_MACRO_ADDRESS, &[id, macro_id]);
        if text == 0 || h.read::<u8>(text) == 0 {
            return text;
        }
        let at = |off: u32| win.wrapping_add(off);
        loop {
            let mut eax: u32 = h.read(at(0x9c));
            if eax.cast_signed() >= h.read::<u32>(at(0xa0)).cast_signed() {
                return eax;
            }
            let c: u8 = h.read(text);
            if c == 7 {
                let next: u8 = h.read(text.wrapping_add(1));
                if (0xe0..0xf0).contains(&next) {
                    let step = int_f32(u32::from(next) - 0xe0).to_bits();
                    h.write(slots, step);
                    let px = pos.wrapping_add(4);
                    let v = x87::add(x87::load(step), x87::load(h.read(px)));
                    h.write(px, x87::store(v));
                }
                text = cdecl(h, super::NEXT_CHAR, &[text]);
                eax = text;
            } else if cdecl(h, super::CHAR_CLASS, &[u32::from(c)]) == 3 {
                cdecl(h, super::CHANGE_MACRO2, &[win, text, pos]);
                text = cdecl(h, super::NEXT_CHAR, &[text]);
                eax = text;
            } else {
                cdecl(h, super::FONT_INFO, &[text, record]);
                cdecl(h, super::MOVIE_FONT_1612, &[pos, record]);
                text = cdecl(h, super::NEXT_CHAR, &[text]);
                eax = text;
                let count = h.read::<u32>(at(0x9c)).wrapping_add(1);
                h.write(at(0x9c), count);
                if h.read::<u32>(at(0xb4)) == 0 && h.read::<u32>(at(0xac)) == 0 {
                    eax = h.read(at(0xa0));
                    if eax.cast_signed() < h.read::<u32>(at(0xa8)).cast_signed() {
                        eax = eax.wrapping_add(1);
                        h.write(at(0xa0), eax);
                    }
                }
            }
            if h.read::<u8>(text) == 0 {
                return eax;
            }
        }
    }
    /// `FUN_008ea680(cursor, key, text, x, y, flag, r, g, b, a; slots)`:
    /// the 16x12 font's text at `g_TOPktCursor` = `cursor`: the glyph
    /// record cleared, the pen set (`(x, y)` as floats, `f32 x` also kept
    /// in `cursor`'s argument slot, the colour bytes, flags 0 and
    /// `flag`); then per character by its class (`0x008df610`): 1, a new
    /// line (x back, y + 16.0); 2, a control code: `0x10` + 0x50..0x57 a
    /// macro, `0x0a` the colour nibbles of the pen's flags, `0x0b` a button
    /// icon (`setButtonData(code, -1)`, `TOAdpButtonIcon(cursor, code,
    /// ftol x, ftol y)`, x + 22.0), others `resetButtonData()`; 3, a macro
    /// (`TOAddPktChangeMacro(key, text, pen)`); 4, a glyph (`TOGetFontInfo`,
    /// shadow and font passes, x plus the scale `+0x1c` and 1). Returns `g_TOPktCursor`.
    pub fn font_1612_text(h: &mut impl Host, args: [u32; 10], slots: u32) -> u32 {
        use ffx_bridge::x87;
        let [cursor, key, text, x, y, flag, r, g, b, a] = args;
        let record = h.rebase(super::GLYPH_RECORD);
        cdecl(h, super::CRT_MEMSET, &[record, 0, 0x40]);
        let pen = h.rebase(super::PEN);
        for off in [0u32, 0x18, 4, 8, 0xc, 0x10, 0x14] {
            h.write(pen.wrapping_add(off), 0u32);
        }
        let cell = h.rebase(PKT_CURSOR);
        h.write(cell, cursor);
        let fx = int_f32(x).to_bits();
        h.write(slots, fx);
        for (off, c) in [(0u32, r), (1, g), (2, b), (3, a)] {
            h.write(pen.wrapping_add(off), c.to_le_bytes()[0]);
        }
        h.write(pen.wrapping_add(4), x87::store(x87::load(fx)));
        h.write(pen.wrapping_add(8), int_f32(y).to_bits());
        h.write(pen.wrapping_add(0x1a), 0u8);
        h.write(pen.wrapping_add(0x1b), flag.to_le_bytes()[0]);
        let px = pen.wrapping_add(4);
        let py = pen.wrapping_add(8);
        let flags_at = pen.wrapping_add(0x1b);
        let mut text = text;
        while h.read::<u8>(text) != 0 {
            let c: u8 = h.read(text);
            let class = cdecl(h, super::CHAR_CLASS, &[u32::from(c)]);
            match class {
                1 => {
                    let line = x87::load(h.read(slots));
                    h.write(px, x87::store(line));
                    let v = x87::add(x87::load(h.read(py)), f64_global(h, super::LINE_STEP));
                    h.write(py, x87::store(v));
                }
                2 if !(c == 0x10
                    && (0x50..=0x57).contains(&h.read::<u8>(text.wrapping_add(1)))) =>
                {
                    let code: u8 = h.read(text.wrapping_add(1));
                    if c == 0xa {
                        let mut f: u8 = h.read(flags_at);
                        if code & 0xf != 0 {
                            f = (f & 0xf0) | ((code & 0xf) - 1);
                            h.write(flags_at, f);
                        }
                        if code & 0xf0 >= 0x40 {
                            f = (f & 0xf) | ((code & 0xf0) - 0x40);
                            h.write(flags_at, f);
                        }
                    } else if c == 0xb {
                        let code = u32::from(code);
                        cdecl(h, super::SET_BUTTON_DATA, &[code, u32::MAX]);
                        let fy = ftol(h, x87::load(h.read(py)));
                        let fx = ftol(h, x87::load(h.read(px)));
                        let cur: u32 = h.read(cell);
                        let cur = cdecl(h, super::BUTTON_ICON, &[cur, code, fx, fy]);
                        let v = x87::add(x87::load(h.read(px)), f64_global(h, super::ICON_STEP));
                        h.write(cell, cur);
                        h.write(px, x87::store(v));
                    } else {
                        cdecl(h, super::RESET_BUTTON_DATA, &[]);
                    }
                }
                2 | 3 => {
                    cdecl(h, super::CHANGE_MACRO, &[key, text, pen]);
                }
                4 => {
                    cdecl(h, super::FONT_INFO, &[text, record]);
                    cdecl(h, super::FONT1612_SHADOW, &[pen, record]);
                    cdecl(h, super::FONT1612_FONT, &[pen, record]);
                    let sx = x87::load(h.read(record.wrapping_add(0x1c)));
                    let v = x87::add(x87::add(sx, 1.0), x87::load(h.read(px)));
                    h.write(px, x87::store(v));
                }
                _ => {}
            }
            text = cdecl(h, super::NEXT_CHAR, &[text]);
        }
        h.read(cell)
    }
    /// `FUN_008ab860()` with its frame at `ebp` (the movie message's
    /// end): message slot 0 cleared, the flag `0x01841d40` cleared; with
    /// `0x01841d41` set, `PMODE` (the 64-bit word at the absolute
    /// `0x12000000`) set from the frame's never-written words (`ebp - 8`
    /// masked `0xffff805e` with `0x8042`, `ebp - 4`), the saved image
    /// restored when `0x01841d42` is 1 (`TOTransImageHL(image, 0x2100,
    /// 0x1b, 0, 0, 0x200, 0x80)`), `TOSetMessageWindowMode(0)` and both
    /// flags cleared. The last call's `eax`.
    /// junk read: [ebp-0x8], [ebp-0x4] never written; 0 (brief 2026-10-24)
    pub fn movie_message_end(h: &mut impl Host, ebp: u32) -> u32 {
        let r = cdecl(h, super::CLEAR_MESSAGE_SLOT, &[0]);
        let [taken, stored, shown] = super::CAPTURE_FLAGS.map(|s| h.rebase(s));
        let on = h.read::<u8>(stored);
        h.write(taken, 0u8);
        if on == 0 {
            return r;
        }
        let lo = h.read::<u32>(ebp.wrapping_sub(8)) & 0xffff_805e | 0x8042;
        h.write(super::GS_PMODE, lo);
        let hi: u32 = h.read(ebp.wrapping_sub(4));
        h.write(super::GS_PMODE + 4, hi);
        if h.read::<u8>(shown) == 1 {
            let image = global(h, super::MOVIE_IMAGE);
            cdecl(h, TRANS_IMAGE_HL, &[image, 0x2100, 0x1b, 0, 0, 0x200, 0x80]);
        }
        let r = cdecl(h, super::MESSAGE_WINDOW_MODE, &[0]);
        h.write(stored, 0u8);
        h.write(shown, 0u8);
        r
    }
    /// `TOMakePktMovieFont1612Edge(pos, glyph)` with its frame at `ebp`: the
    /// movie font's outline: a GS tag and two TEX0 words at
    /// `g_TOPktCursor` (from the glyph's `i16 +0`, `+2`, `+4` and the high
    /// nibble of `pos + 0x1b`), `TOGsALPHA(0, 1, 0, 1, 0x80)`,
    /// `TOGsTEST(1, 7, 0 x6)`, `TOGsWords4(0x80, 0x80, 0x80, 0x28)`, a
    /// sprite tag; then the glyph drawn at ten offsets (the signed byte
    /// pairs at `ebp - 0x18`: (-1, -1), (0, -1), (1, -1), (-1, 0), ...),
    /// each `TOGsXY2`/`TOGsXYZ2Offset` of the UV box and the screen box
    /// moved by the offset (x87, `ftol`); then `TOCtrlPkt`. Its `eax`.
    pub fn movie_font_edge(h: &mut impl Host, ebp: u32, pos: u32, glyph: u32) -> u32 {
        use ffx_bridge::x87;
        let cell = h.rebase(PKT_CURSOR);
        let p: u32 = h.read(cell);
        let sign = |v: u32| if v.cast_signed() < 0 { u32::MAX } else { 0 };
        let g0 = i16_at(h, glyph, 0);
        let g2 = i16_at(h, glyph, 2);
        let nibble = u32::from(h.read::<u8>(pos.wrapping_add(0x1b))) >> 4;
        let page = i16_at(h, glyph, 4).wrapping_add(nibble * 2);
        let hi = (((page << 4) | sign(g2)) << 20) | (g2 >> 12) | sign(g0) | 0x5_d706;
        let lo = (g2 << 20) | g0 | 0x9c00_8000;
        put_words(
            h,
            p,
            &[
                0x8001,
                0x5000_4000,
                0x1_eeee,
                0,
                g0 | 0x9d30_8000,
                sign(g0) | 0x2005_d786,
                6,
                0,
                lo,
                hi,
                6,
                0,
            ],
        );
        let offsets = [
            0xff00_ffffu32,
            0x00fe_ff01,
            0x00ff_0002,
            0x01ff_0001,
            0x0101_0100,
        ];
        put_words(h, ebp.wrapping_sub(0x18), &offsets);
        h.write(cell, p.wrapping_add(0x30));
        cdecl(h, super::GS_ALPHA, &[0, 1, 0, 1, 0x80]);
        cdecl(h, super::GS_TEST, &[1, 7, 0, 0, 0, 0, 0, 0]);
        cdecl(h, super::GS_WORDS4, &[0x80, 0x80, 0x80, 0x28]);
        let q: u32 = h.read(cell);
        put_words(h, q, &[0x800a, 0x40af_4000, 0x4343, 0]);
        h.write(cell, q.wrapping_add(0x10));
        let mut r = 0;
        for i in 0..10u32 {
            let (u, v) = (
                f32_at(h, glyph.wrapping_add(8)),
                f32_at(h, glyph.wrapping_add(0xc)),
            );
            let (u, v) = (ftol(h, u), ftol(h, v));
            cdecl(h, super::GS_XY2, &[u, v]);
            let dy = h.read::<i8>(ebp.wrapping_sub(0x17).wrapping_add(2 * i));
            let dx = h.read::<i8>(ebp.wrapping_sub(0x18).wrapping_add(2 * i));
            h.write(ebp.wrapping_sub(0x1c), i32::from(dx));
            let (fy, fx) = (f64::from(dy), f64::from(dx));
            h.write(ebp.wrapping_sub(0x28), fy.to_bits());
            h.write(ebp.wrapping_sub(0x20), fx.to_bits());
            let px = x87::load(h.read(pos.wrapping_add(4)));
            let py = x87::load(h.read(pos.wrapping_add(8)));
            let y = x87::add(x87::add(py, f32_at(h, glyph.wrapping_add(0x18))), fy);
            let y = ftol(h, y);
            let x = ftol(h, x87::add(fx, px));
            cdecl(h, super::GS_XYZ2_OFFSET, &[x, y]);
            let (u, v) = (
                f32_at(h, glyph.wrapping_add(0x10)),
                f32_at(h, glyph.wrapping_add(0x14)),
            );
            let (u, v) = (ftol(h, u), ftol(h, v));
            cdecl(h, super::GS_XY2, &[u, v]);
            let y = x87::add(
                x87::add(
                    x87::add(py, f32_at(h, glyph.wrapping_add(0x18))),
                    f32_at(h, glyph.wrapping_add(0x20)),
                ),
                fy,
            );
            let y = ftol(h, y);
            let x = ftol(
                h,
                x87::add(x87::add(f32_at(h, glyph.wrapping_add(0x1c)), px), fx),
            );
            r = cdecl(h, super::GS_XYZ2_OFFSET, &[x, y]);
        }
        ctrl_pkt(h, r)
    }
    /// `TODrawAlphaBG(alpha)` with its frame at `ebp`: four black bands
    /// across the screen (`graphicDrawUIElement(quad, 0, 1, 2, 0)` with
    /// the quad at `ebp - 0x9c`; `+0x80` = 1000): a fade from `alpha` to 0
    /// between the edges `e0` and `e1`, a fade from 0 to `alpha` between
    /// `e2` and `e3`, and solid `alpha` from the top to `e0` and from
    /// `e3` to the bottom. The edges are the floats `0x00b43db4`,
    /// `0x00b58cd8`, `0x00b65898`, `0x00b65d8c`; the screen size
    /// `0x00b41d9c` by `0x00b41d98`. The last draw's `eax`.
    /// junk read: the quad's UVs and vertices 2-3 never written; 0 (brief
    /// 2026-10-24)
    pub fn alpha_bg(h: &mut impl Host, ebp: u32, alpha: u32) -> u32 {
        let a = alpha & 0xff;
        let q = ebp.wrapping_sub(QUAD);
        let [e0, e1, e2, e3] = super::ALPHA_BG_EDGES.map(|s| float_copy(h, s));
        let w = float_copy(h, super::SCREEN_W);
        let bottom = float_copy(h, super::SCREEN_H);
        for (off, v) in [(0xcu32, 0u32), (8, 0), (0x1c, 0x3e8), (0x14, 0)] {
            h.write(ebp.wrapping_sub(off), v);
        }
        let bands = [
            (e0, e1, a, 0),
            (e2, e3, 0, a),
            (0, e0, a, a),
            (e3, bottom, a, a),
        ];
        let mut r = 0;
        for (y0, y1, a0, a1) in bands {
            put_words(h, q, &[0, y0]);
            put_words(h, q.wrapping_add(0x20), &[w, y1]);
            put_words(h, q.wrapping_add(0x10), &[0, 0, 0, a0]);
            put_words(h, q.wrapping_add(0x30), &[0, 0, 0, a1]);
            r = cdecl(h, super::DRAW_UI_ELEMENT, &[q, 0, 1, 2, 0]);
        }
        r
    }
    /// Whether a wait window shows `cell`: from `i16 +0x40`, `+0x38 *
    /// +0x3a` cells.
    fn wait_cell_shown(h: &impl Host, win: u32, cell: i32) -> bool {
        let first = i32::from(h.read::<i16>(win.wrapping_add(0x40)));
        let cols = i32::from(h.read::<i16>(win.wrapping_add(0x38)));
        let rows = i32::from(h.read::<i16>(win.wrapping_add(0x3a)));
        cell >= first && cell < cols.wrapping_mul(rows).wrapping_add(first)
    }

    /// A wait window's cell cursor position: `x` = `ftol(remapX(xstep) *
    /// f32(cell % cols) + ftol(win + 0x78))`, `y` = `ftol(remapY(ystep) *
    /// f32(cell / cols) + win + 0x7c)` (`cell` less the first shown,
    /// `i16 +0x40`; `cols` `i16 +0x38`; `idiv`). `ystep` `None` uses
    /// `f32(row * 0x34 + 5)` remapped instead.
    fn wait_cell_pos(h: &mut impl Host, win: u32, cell: i32, ystep: Option<u32>) -> (u32, u32) {
        use ffx_bridge::x87;
        let left = ftol(h, f32_at(h, win.wrapping_add(0x78))).cast_signed();
        let cols = i32::from(h.read::<i16>(win.wrapping_add(0x38)));
        let step_x = float_copy(h, super::CURSOR_X_STEP);
        let rx = remap(h, super::REMAP_X2, step_x);
        let col = f64::from(int_f32(cell.wrapping_rem(cols).cast_unsigned()));
        let x = ftol(h, x87::add(x87::mul(rx, col), f64::from(left)));
        let first = i32::from(h.read::<i16>(win.wrapping_add(0x40)));
        let row = cell.wrapping_sub(first).wrapping_div(cols);
        let (ry, factor) = match ystep {
            Some(sym) => {
                let step = float_copy(h, sym);
                (
                    remap(h, super::REMAP_Y2, step),
                    f64::from(int_f32(row.cast_unsigned())),
                )
            }
            None => {
                let v = int_f32(row.wrapping_mul(0x34).wrapping_add(5).cast_unsigned());
                (remap(h, super::REMAP_Y2, v.to_bits()), 1.0)
            }
        };
        let y = x87::add(x87::mul(ry, factor), f32_at(h, win.wrapping_add(0x7c)));
        (x, ftol(h, y))
    }

    /// `TODrawWaitBtlWinPrepare(win)`: the window's open time (`+0xe0` =
    /// `graphicGetTime() - +0xdc` when `+0xdc` is above 0, else 0), the
    /// command window drawn (`TOBtlDrawCommandWindow`), then every 2nd
    /// frame (`g_SgFrame` bit 0) a cursor at each marked cell (the `i8
    /// +0xb` halfwords from `+0xe` inside the shown range: from `i16
    /// +0x40`, `+0x38 * +0x3a` cells) by `TOBtlSetCurPos(0, x - 5, y -
    /// 2)`; and the current cell's (`i16 +0x42`, when shown) by
    /// `TOBtlSetCurPos(0, x, y)` with the row's own `y`. Returns 0.
    pub fn wait_win_prepare(h: &mut impl Host, win: u32) -> u32 {
        use ffx_bridge::x87;
        let opened = x87::load(h.read(win.wrapping_add(0xdc)));
        let open = if 0.0 < opened {
            x87::sub(
                h.call_sym_st0_f64(super::GET_TIME, Conv::Cdecl, &[]),
                opened,
            )
        } else {
            0.0
        };
        h.write(win.wrapping_add(0xe0), x87::store(open));
        cdecl(h, super::DRAW_COMMAND_WINDOW, &[win]);
        let mut k = 0i32;
        while k < i32::from(h.read::<i8>(win.wrapping_add(0xb))) {
            let at = win.wrapping_add(0xe).wrapping_add(2 * k.cast_unsigned());
            let cell = i32::from(h.read::<u16>(at));
            let frame: u8 = h.read(h.rebase(super::SG_FRAME));
            if wait_cell_shown(h, win, cell) && frame & 1 != 0 {
                let (x, y) = wait_cell_pos(h, win, cell, Some(super::CURSOR_Y_STEP));
                cdecl(
                    h,
                    super::SET_CUR_POS,
                    &[0, x.wrapping_sub(5), y.wrapping_sub(2)],
                );
            }
            k += 1;
        }
        let current = i32::from(h.read::<i16>(win.wrapping_add(0x42)));
        let first = i32::from(h.read::<i16>(win.wrapping_add(0x40)));
        if current < first || !wait_cell_shown(h, win, current) {
            return 0;
        }
        let (x, y) = wait_cell_pos(h, win, current, None);
        cdecl(h, super::SET_CUR_POS, &[0, x, y]);
        0
    }
    /// `TODrawMovieMessage()` with its frame at `ebp` (`ebp - 4` the
    /// display buffer's field word `Sg_GetDBuffDC` writes): `TODapsDraw`
    /// when a display mode is set; with the movie message up
    /// (`0x01841d40`) and either the save menu's `0x200` bit or the capture
    /// option 1, and a display mode: `TOTransTexture`, the packet buffer
    /// (`TOSetPktBuffAdrs(adrs, size, size / 4 * 3)`, `TOOpenPktbuff`),
    /// mode 6 a full scissor, else a frame packet `(0x108, 8, 0, 0)`, the
    /// offset `(0x7000, 0x7300)`, a 512x127 scissor and a black 512x128
    /// rect; the font, window 0 and message 0 drawn, closed and kicked.
    /// Then (a mode, and a second read not 6) a second packet: the frame from the
    /// display buffer's field (`+0x60`/`+0x64`, odd fields `+0x190`/
    /// `+0x194`: `fbp & 0x1ff`, `fbw`, `psm`, the mask), the offset, a
    /// 640x416 scissor, `TOMkpMovieMessageImage(0x2100, 0, 0x120, 0x280,
    /// 0x80)`, the field's offset (`u16 +0x78`/`+0x7c`, odd `+0x1a8`/
    /// `+0x1ac`), closed and kicked. `0x025d09c0` set to 1 after a draw.
    /// The last call's `eax`.
    pub fn draw_movie_message(h: &mut impl Host, ebp: u32) -> u32 {
        let field = ebp.wrapping_sub(4);
        cdecl(h, super::SG_DBUFF, &[field]);
        let mut r = cdecl(h, super::DISP_MODE, &[]);
        if r != 0 {
            r = cdecl(h, super::DAPS_DRAW, &[]);
        }
        if h.read::<u8>(h.rebase(super::CAPTURE_FLAGS[0])) == 0 {
            return r;
        }
        r = cdecl(h, super::SAVE_MENU, &[]);
        if h.read::<u32>(r) & 0x200 == 0 && global(h, super::CAPTURE_OPTION) != 1 {
            return r;
        }
        r = cdecl(h, super::DISP_MODE, &[]);
        if r == 0 {
            return r;
        }
        cdecl(h, super::TRANS_TEXTURE, &[]);
        let dc = cdecl(h, super::SG_DBUFF, &[field]);
        let [adrs, size] = super::PKT_BUFF_ADRS.map(|s| global(h, s));
        let quarter = size.cast_signed().wrapping_div(4).cast_unsigned();
        cdecl(
            h,
            super::SET_PKT_BUFF,
            &[adrs, size, quarter.wrapping_mul(3)],
        );
        cdecl(h, super::OPEN_PKT, &[]);
        if cdecl(h, super::DISP_MODE, &[]) == 6 {
            cdecl(h, super::PKT_SCISSOR, &[0, 0, 0x200, 0x1a0]);
        } else {
            cdecl(h, super::FRAME_PKT, &[0x108, 8, 0, 0]);
            cdecl(h, super::XYOFFSET, &[0x7000, 0x7300]);
            cdecl(h, super::PKT_SCISSOR, &[0, 0, 0x200, 0x7f]);
            cdecl(h, super::FLAT_RECT, &[0, 0, 0x200, 0x80, 0, 0, 0, 0]);
        }
        cdecl(h, super::SET_REG_FONT1612, &[]);
        cdecl(h, super::DRAW_WINDOW, &[0]);
        cdecl(h, super::DRAW_MESSAGE, &[0]);
        cdecl(h, super::CLOSE_PKT, &[]);
        cdecl(h, DRAW_KICK, &[]);
        let mut mode = cdecl(h, super::DISP_MODE, &[]);
        if mode != 0 {
            mode = cdecl(h, super::DISP_MODE, &[]);
        }
        if mode != 0 && mode != 6 {
            cdecl(h, super::OPEN_PKT, &[]);
            let odd = h.read::<u8>(field) & 1 != 0;
            let (frame, offset) = if odd { (0x190, 0x1a8) } else { (0x60, 0x78) };
            let fb: u32 = h.read(dc.wrapping_add(frame));
            let mask: u32 = h.read(dc.wrapping_add(frame + 4));
            cdecl(
                h,
                super::FRAME_PKT,
                &[fb & 0x1ff, (fb >> 16) & 0x3f, (fb >> 24) & 0x3f, mask],
            );
            cdecl(h, super::XYOFFSET, &[0x7000, 0x7300]);
            cdecl(h, super::PKT_SCISSOR, &[0, 0, 0x27f, 0x1a0]);
            cdecl(h, super::MKP_MOVIE_IMAGE, &[0x2100, 0, 0x120, 0x280, 0x80]);
            let x = u32::from(h.read::<u16>(dc.wrapping_add(offset)));
            let y = u32::from(h.read::<u16>(dc.wrapping_add(offset + 4)));
            cdecl(h, super::XYOFFSET, &[x, y]);
            cdecl(h, super::CLOSE_PKT, &[]);
            r = cdecl(h, DRAW_KICK, &[]);
        } else {
            r = mode;
        }
        h.write(h.rebase(super::MOVIE_DRAWN), 1u8);
        r
    }
    /// `TOMkpWindow1612Frame_008eadf0(packet, x, y, w, h)` (the frame of
    /// `TkMenuDrawWindow1612`): a GS tag and the frame's TEX0/TEX1/ALPHA
    /// words at `packet` (12 words), the packet cursor after them, then
    /// nine sprites of the 1612 window texture: the corners, edges and
    /// middle on columns `x`, `x + 0xb`, `x + w - 0xb`, `x + w` and rows
    /// `y`, `y + 4`, `y + h - 5`, `y + h` (UVs 0xd8..0xf2 by 0x28..0x35;
    /// the bottom-left's second UV `0xe2`), each `TOGsWords4(0x80 x4)`,
    /// `TOGsXY2`, `TOGsXYZ2Offset` pairs. The columns and the third row
    /// go into argument slots `+0`, `+0xc`, `+0x10`, `+8` as the original
    /// keeps them. The packet cursor.
    pub fn window1612_frame(h: &mut impl Host, args: [u32; 5], slots: u32) -> u32 {
        let [packet, x, y, w, ht] = args;
        put_words(
            h,
            packet,
            &[
                1,
                0x1000_0000,
                0xe,
                0,
                0x2131_3e00,
                0x4005_d406,
                6,
                0,
                0x8009,
                0x50ab_4000,
                0x4_3431,
                0,
            ],
        );
        h.write(h.rebase(PKT_CURSOR), packet.wrapping_add(0x30));
        let dx = w.wrapping_sub(0x16);
        let dy = ht.wrapping_sub(9);
        let grey = |h: &mut _| {
            cdecl(h, super::GS_WORDS4, &[0x80, 0x80, 0x80, 0x80]);
        };
        let uv = |h: &mut _, u: u32, v: u32| {
            cdecl(h, super::GS_XY2, &[u, v]);
        };
        let at = |h: &mut _, px: u32, py: u32| {
            cdecl(h, super::GS_XYZ2_OFFSET, &[px, py]);
        };
        let y4 = y.wrapping_add(4);
        grey(h);
        uv(h, 0xd8, 0x28);
        at(h, x, y);
        uv(h, 0xe3, 0x2c);
        let x1 = x.wrapping_add(0xb);
        h.write(slots, x1);
        at(h, x1, y4);
        grey(h);
        uv(h, 0xe3, 0x28);
        at(h, x1, y);
        uv(h, 0xe6, 0x2c);
        let x2 = x.wrapping_add(0xb).wrapping_add(dx);
        h.write(slots.wrapping_add(0xc), x2);
        at(h, x2, y4);
        grey(h);
        uv(h, 0xe7, 0x28);
        at(h, x2, y);
        uv(h, 0xf2, 0x2c);
        let x3 = x.wrapping_add(0x16).wrapping_add(dx);
        h.write(slots.wrapping_add(0x10), x3);
        at(h, x3, y4);
        grey(h);
        uv(h, 0xd8, 0x2c);
        at(h, x, y4);
        uv(h, 0xe3, 0x2f);
        let y1 = y4.wrapping_add(dy);
        h.write(slots.wrapping_add(8), y1);
        at(h, x1, y1);
        grey(h);
        uv(h, 0xe3, 0x2c);
        at(h, x1, y4);
        uv(h, 0xe6, 0x2f);
        at(h, x2, y1);
        grey(h);
        uv(h, 0xe7, 0x2c);
        at(h, x2, y4);
        uv(h, 0xf2, 0x2f);
        at(h, x3, y1);
        grey(h);
        uv(h, 0xd8, 0x30);
        at(h, x, y1);
        uv(h, 0xe2, 0x35);
        let y2 = y.wrapping_add(9).wrapping_add(dy);
        at(h, x1, y2);
        grey(h);
        uv(h, 0xe3, 0x30);
        at(h, x1, y1);
        uv(h, 0xe6, 0x35);
        at(h, x2, y2);
        grey(h);
        uv(h, 0xe7, 0x30);
        at(h, x2, y1);
        uv(h, 0xf2, 0x35);
        at(h, x3, y2);
        h.read(h.rebase(PKT_CURSOR))
    }
}
