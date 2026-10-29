//! M5 stage 1, the menu, battle and script leftovers (round 3), batch 1:
//! the menu's small pieces the static call graph reaches. Most are code
//! Ghidra left outside its function list (jump thunks, the menu screens'
//! table entries, the steps of the battle-end and Tk boot code that the
//! dump folded into a label); each was read from the machine code.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

pub mod bridge {
    use ffx_bridge::{Global, x87};
    use ffx_graphic::host::{Conv, Host};

    // ---- jump targets and callees ----
    pub const ATEL_DRAW_SEMI_TRANS: u32 = 0x0087_2ca0;
    pub const TK_SND_IS_READ: u32 = 0x0088_6600;
    pub const TK_GET_LANG: u32 = 0x0088_7d00;
    pub const TK_GET_NOW_EVENT_JUMP_MAP_NO: u32 = 0x0088_d690;
    pub const TK_GET_NOW_EVENT_JUMP_INDEX: u32 = 0x0088_d670;
    pub const MS_GET_SAVE_EVENT_ADDRESS: u32 = 0x0078_5240;
    pub const MS_IMPORTANT_RESET: u32 = 0x0079_07a0;
    pub const TK_VU1_SYNC_PATH: u32 = 0x0088_ec60;
    pub const MS_GET_GIL: u32 = 0x0078_4e80;
    pub const TO_DRAW_SCISSOR_XYWH: u32 = 0x008f_9230;
    pub const TO_MENU_GET_ABILITYMAP_WORK_ADRS: u32 = 0x008b_ea30;
    pub const TK_MN2_BUILD_ITEM_LIST: u32 = 0x008c_2390;
    pub const MS_SET_SAVE_PARAM_ALL: u32 = 0x0078_6900;
    pub const TK_MC_SET_EXEC_PORT: u32 = 0x0088_8430;
    pub const TK_MC_SET_DEBUG_LOCK: u32 = 0x0088_8420;
    pub const TK_SYNC_VDI: u32 = 0x0088_7f20;
    pub const TK_SYNC_VEI: u32 = 0x0088_7f40;
    pub const TK_SET_VSYNC_HANDLER: u32 = 0x0088_8680;
    pub const GET_CD: u32 = 0x0076_d0d0;
    pub const TK_CD_READ_WITH_MALLOC: u32 = 0x0088_d1d0;
    pub const SND_STANDBY_SHOUT_STREAM: u32 = 0x0081_e3b0;
    pub const SND_DATA_SEND_SYNC: u32 = 0x0088_5ee0;
    pub const SQS_REPORT_STANDBY_STATUS: u32 = 0x0081_e2d0;
    pub const SQS_SHOUT_STREAMING_DATA_SEND_THUNK: u32 = 0x0088_7560;
    pub const USER_FREE: u32 = 0x0068_7010;
    pub const SG_MEM_CHECK: u32 = 0x0083_b7b0;
    pub const GRAPHIC_GET_TIME_US: u32 = 0x0064_1420;
    pub const GRAPHIC_SET_FREE_CAMERA_MODE: u32 = 0x0064_0e20;
    pub const SG_SET_PAUSE_ALL_SOUNDS: u32 = 0x0082_1cc0;
    pub const PAUSE_INPUT_RESUME: u32 = 0x006f_32d0;
    pub const INPUT_UPDATE_ALL: u32 = 0x0063_0e80;
    pub const MOVIE_WITH_3D_SCENE: u32 = 0x0076_f0c0;
    pub const GRAPHIC_VIDEO_PAUSE: u32 = 0x0064_5de0;
    pub const MATH_SIN_LOOKUP: u32 = 0x0088_e730;
    pub const MATH_COS_LOOKUP: u32 = 0x0088_e750;
    pub const BTL_END_DRAW_ITEM_PAGE: u32 = 0x008b_bca0;
    pub const BTL_END_DRAW_WEAPON_PAGE: u32 = 0x008b_bed0;
    pub const BTL_END_DRAW_GIL_PAGE: u32 = 0x008b_bb10;
    pub const BTL_END_DRAW_SPOIL_LIST_PAGE: u32 = 0x008b_d280;
    pub const BTL_END_DRAW_EXCHANGE_PAGE: u32 = 0x008b_d670;
    pub const BTL_END_DRAW_SPOIL_PAGE: u32 = 0x008b_d020;
    pub const MS_SAVE_ITEM_USE: u32 = 0x0079_0550;
    pub const TO_DRAW_BOX_XYWHC2: u32 = 0x008f_3db0;
    pub const TO_MENU_DRAW_KICK_TMP: u32 = 0x008b_ea20;
    pub const TO_DAPS_DRAW: u32 = 0x0089_dc10;
    pub const TK_MENU_MAIN_SLEEP_MENU: u32 = 0x008a_ad70;
    pub const TO_MENU_INIT_BRUSH: u32 = 0x0090_0f40;
    pub const TK_MENU_DRAW_KICK: u32 = 0x008d_ce50;
    pub const TK_VU1_SYNC_PATH_THUNK: u32 = 0x008c_1790;
    pub const TK_MN2_DRAW_INIT_PACKET: u32 = 0x008c_0be0;
    pub const TK_MN2_DRAW_KICK_SYNC_PACKET: u32 = 0x008c_0cc0;
    pub const TO_DRAW_MENU_BG: u32 = 0x008f_5c10;
    pub const TK_MENU_SET_SCREEN_X2_FLAG: u32 = 0x008a_b020;
    pub const MENU_ITEM_GET_LINE_POS: u32 = 0x008c_9b60;
    pub const TO_SEQ_GET_RECT: u32 = 0x008a_e4b0;
    pub const TK_MENU_ITEM_CLOSE_WINDOW: u32 = 0x008c_6b60;
    pub const TK_MENU_ITEM_CLOSE_WINDOW2: u32 = 0x008c_6b80;
    pub const TK_MENU_ITEM_CLOSE_WINDOW3: u32 = 0x008c_6b40;
    pub const TK_MENU_ITEM_CLOSE_WINDOW4: u32 = 0x008c_6ba0;
    pub const TO_SEQ_STOP_SCENE: u32 = 0x008a_f4f0;
    pub const TK_MENU_CLOSE_SUB_CMD_WINDOW: u32 = 0x008e_36f0;
    pub const TK_MENU_GET_CURRENT_PLAYER: u32 = 0x008a_9860;
    pub const GRAPHIC_UI_REMAP_X2: u32 = 0x0064_47e0;
    pub const GRAPHIC_UI_REMAP_Y2: u32 = 0x0064_4820;
    pub const TO_MENU_DRAW_CHR_STATUS_PLATE: u32 = 0x008f_f490;
    pub const TK_MN2_GET_WSHOP_POS: u32 = 0x008d_9190;
    pub const TK_MN2_GET_ISHOP_POS: u32 = 0x008d_b080;

    // ---- globals ----
    /// `g_PhyreScene`: its camera at `+0xdc` (near plane `+0xb8`).
    pub const PHYRE_SCENE: Global<u32> = Global::new(0x00cc_b9d8);
    pub const MC_DEBUG_PORT: Global<u32> = Global::new(0x0133_022c);
    pub const PAUSE_ENTERED: Global<u32> = Global::new(0x0134_084c);
    pub const PAUSE_REQUEST: Global<u32> = Global::new(0x0134_0850);
    pub const PAUSE_ON: Global<u32> = Global::new(0x0134_0828);
    /// The time spent in the pause (µs, 64 bits) and when it began.
    pub const PAUSE_TIME_TOTAL: Global<u64> = Global::new(0x012f_b818);
    pub const PAUSE_TIME_START: Global<u64> = Global::new(0x012f_b820);
    pub const MOVIE_PICTURE_ON: Global<u32> = Global::new(0x0112_a00c);
    /// `g_BtlEndFadeGil` and its speed, and the spoil page's pair.
    pub const BTL_END_FADE_GIL: Global<i16> = Global::new(0x0186_a160);
    pub const BTL_END_FADE_GIL_SPEED: Global<i16> = Global::new(0x0186_a164);
    pub const BTL_END_FADE_SPOIL: Global<i16> = Global::new(0x0186_a168);
    pub const BTL_END_FADE_SPOIL_SPEED: Global<i16> = Global::new(0x0186_a16c);
    /// The battle-end spoils: a signed byte count, `u16` item ids at
    /// `+4`, `u8` amounts at `+0x14`.
    pub const BTL_END_SPOIL_COUNT: Global<i8> = Global::new(0x0231_0f70);
    pub const BTL_END_SPOIL_ITEMS: u32 = 0x0231_0f74;
    pub const BTL_END_SPOIL_AMOUNTS: u32 = 0x0231_0f84;
    /// The menu's per-call function word at `0x00c416b0` (`menu_list_draw`).
    pub const MENU_LIST_DRAW_PTR: Global<u32> = Global::new(0x00c4_16b0);
    pub const MENU_HELP_WINDOW: Global<u32> = Global::new(0x0186_a278);
    pub const MENU_HELP_FLAG: Global<u32> = Global::new(0x0187_1538);
    pub const MENU_ITEM_DRAW_STEP: Global<u8> = Global::new(0x0186_a390);

    /// Image addresses the code pushes or returns (relocated).
    pub const MEM_CHECK_FILE: u32 = 0x00b5_e968;
    pub const VIBRATE_CURVE_0087AA30: u32 = 0x00b5_46a0;
    pub const VIBRATE_CURVE_008A9850: u32 = 0x00b5_f46c;
    pub const ITEM_RECT_NAME: u32 = 0x00b6_5a7c;
    pub const ITEM_SCENE_A: u32 = 0x00b6_5a94;
    pub const ITEM_SCENE_B: u32 = 0x00b6_5a9c;
    pub const ITEM_RECT_NAME2: u32 = 0x00b6_5aac;
    pub const ITEM_RECT_OUT: u32 = 0x0186_a410;
    pub const ITEM_RECT_OUT2: u32 = 0x0186_a418;
    /// The status plate's position (`f32`, design units).
    pub const ITEM_PLATE_X: u32 = 0x00b5_ef1c;
    pub const ITEM_PLATE_Y: u32 = 0x00b6_59b4;

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    // ---- 5-byte jump thunks: the target with the caller's words ----

    /// `code_0088d500()`: `jmp AtelDrawSemiTrans`.
    pub fn atel_draw_semi_trans_thunk(h: &mut impl Host) -> u32 {
        cdecl(h, ATEL_DRAW_SEMI_TRANS, &[])
    }
    /// `code_00887ae0()`: `jmp TkSndIsRead`.
    pub fn tk_snd_is_read_thunk(h: &mut impl Host) -> u32 {
        cdecl(h, TK_SND_IS_READ, &[])
    }
    /// `TkGetLang_thunk()`: `jmp TkGetLang`.
    pub fn tk_get_lang_thunk(h: &mut impl Host) -> u32 {
        cdecl(h, TK_GET_LANG, &[])
    }
    /// `TkGetNowEventJumpMapNo_0086c1f0()`: `jmp TkGetNowEventJumpMapNo`.
    pub fn tk_get_now_event_jump_map_no_thunk(h: &mut impl Host) -> u32 {
        cdecl(h, TK_GET_NOW_EVENT_JUMP_MAP_NO, &[])
    }
    /// `code_0086c1d0()`: `jmp TkGetNowEventJumpIndex`.
    pub fn tk_get_now_event_jump_index_thunk(h: &mut impl Host) -> u32 {
        cdecl(h, TK_GET_NOW_EVENT_JUMP_INDEX, &[])
    }
    /// `TkGetSaveEventAddress()`: `jmp MsGetSaveEventAddress`.
    pub fn tk_get_save_event_address(h: &mut impl Host) -> u32 {
        cdecl(h, MS_GET_SAVE_EVENT_ADDRESS, &[])
    }
    /// `code_0088e780(id)`: `jmp MsImportantReset`.
    pub fn ms_important_reset_thunk(h: &mut impl Host, id: u32) -> u32 {
        cdecl(h, MS_IMPORTANT_RESET, &[id])
    }
    /// `code_008c1790()`: `jmp TkVU1SyncPath`.
    pub fn tk_vu1_sync_path_thunk(h: &mut impl Host) -> u32 {
        cdecl(h, TK_VU1_SYNC_PATH, &[])
    }
    /// `TkMenuGetGil_008a98b0()` and `code_008bda30()`: `jmp MsGetGIL`.
    pub fn ms_get_gil_thunk(h: &mut impl Host) -> u32 {
        cdecl(h, MS_GET_GIL, &[])
    }
    /// `TkMn2DrawScissorRect(x, y, w, h)`: `jmp TODrawScissorXYWH`.
    pub fn draw_scissor_rect_thunk(h: &mut impl Host, x: u32, y: u32, w: u32, hh: u32) -> u32 {
        cdecl(h, TO_DRAW_SCISSOR_XYWH, &[x, y, w, hh])
    }
    /// `code_008c1b10()`: `jmp TOMenuGetAbilitymapWorkAdrs`.
    pub fn get_abilitymap_work_thunk(h: &mut impl Host) -> u32 {
        cdecl(h, TO_MENU_GET_ABILITYMAP_WORK_ADRS, &[])
    }
    /// `code_008c2380(list, kind)`: `jmp TkMn2BuildItemList_008c2390`.
    pub fn build_item_list_thunk(h: &mut impl Host, list: u32, kind: u32) -> u32 {
        cdecl(h, TK_MN2_BUILD_ITEM_LIST, &[list, kind])
    }
    /// `TkMsSetSaveParamAll()`: `jmp MsSetSaveParamAll`.
    pub fn tk_ms_set_save_param_all(h: &mut impl Host) -> u32 {
        cdecl(h, MS_SET_SAVE_PARAM_ALL, &[])
    }

    // ---- constants and flags ----

    /// `code_0087aa30()` (a `g_TkVibrateCurves` entry): the image
    /// address `0x00b546a0`.
    pub fn vibrate_curve_0087aa30(h: &impl Host) -> u32 {
        h.rebase(VIBRATE_CURVE_0087AA30)
    }
    /// `code_008a9850()` and `code_008dde90()` (`g_TkVibrateCurves`
    /// entries): the image address `0x00b5f46c`.
    pub fn vibrate_curve_008a9850(h: &impl Host) -> u32 {
        h.rebase(VIBRATE_CURVE_008A9850)
    }
    /// `code_008c3380()` / `code_008c33e0()`: the help flag word
    /// `0x01871538` = `on`.
    pub fn set_help_flag(h: &mut impl Host, on: u32) {
        MENU_HELP_FLAG.set(h, on);
    }

    // ---- small bodies ----

    /// `setNearPlane(near)` (`0x00645f00`): the scene camera's
    /// (`g_PhyreScene + 0xdc`) near plane `+0xb8` = `near` (`fld`/`fstp`,
    /// so a signalling NaN is quieted), then a tail jump to the camera's
    /// virtual method 1 with `ecx` = the camera (it recomputes the
    /// projection; it reads no stack word).
    pub fn set_near_plane(h: &mut impl Host, near: u32) -> u32 {
        let scene = PHYRE_SCENE.get(h);
        let camera: u32 = h.read(scene.wrapping_add(0xdc));
        let v = x87::round_trip(f32::from_bits(near));
        h.write(camera.wrapping_add(0xb8), v);
        h.call_virtual(camera, 1, &[])
    }

    /// `code_00888310()` (after `TkSyncVEI`): `TkMcSetExecPort(0)`, the
    /// word `0x0133022c` = 0, `TkMcSetDebugLock(0)`; returns its `eax`.
    pub fn mc_reset_ports(h: &mut impl Host) -> u32 {
        cdecl(h, TK_MC_SET_EXEC_PORT, &[0]);
        MC_DEBUG_PORT.set(h, 0);
        cdecl(h, TK_MC_SET_DEBUG_LOCK, &[0])
    }

    /// `code_00888760()` (after `TkRefreshHDDMode`): `TkSyncVDI`,
    /// `TkSyncVEI`, `TkSetVsyncHandler(0)`; returns its `eax`.
    pub fn sync_and_clear_vsync(h: &mut impl Host) -> u32 {
        cdecl(h, TK_SYNC_VDI, &[]);
        cdecl(h, TK_SYNC_VEI, &[]);
        cdecl(h, TK_SET_VSYNC_HANDLER, &[0])
    }

    /// The CD object's first two hooks (`GetCD() + 0x10`, `+0x14`), cdecl
    /// with one word each; returns the CD object and the second's `eax`.
    fn cd_hooks(h: &mut impl Host, file: u32, arg: u32) -> (u32, u32) {
        let cd = cdecl(h, GET_CD, &[]);
        let f: u32 = h.read(cd.wrapping_add(0x10));
        h.call(f, Conv::Cdecl, &[file]);
        let f: u32 = h.read(cd.wrapping_add(0x14));
        (cd, h.call(f, Conv::Cdecl, &[arg]))
    }

    /// The two `Sg_MemCheck` calls after a load (`0x00b5e968`, the
    /// source file, lines 0x1e0 and 0x1e1).
    fn mem_checks(h: &mut impl Host) -> u32 {
        let file = h.rebase(MEM_CHECK_FILE);
        cdecl(h, SG_MEM_CHECK, &[0, file, 0x1e0]);
        cdecl(h, SG_MEM_CHECK, &[1, file, 0x1e1])
    }

    /// `code_0088de60()` (after `TkInitScene`): CD file 0x18 through the
    /// CD hooks (`+0x10(0x18)`, `+0x14(0)`), `TkCdReadWithMalloc(0x18,
    /// 0)`, the shout stream stood by (`SndStandbyShoutStream(data)`),
    /// `SndDataSendSync(1)`, `SqsReportStandbyStatus(1)`, the data freed
    /// and the memory checked; returns the last check's `eax`.
    pub fn load_shout_stream(h: &mut impl Host) -> u32 {
        cd_hooks(h, 0x18, 0);
        let data = cdecl(h, TK_CD_READ_WITH_MALLOC, &[0x18, 0]);
        cdecl(h, SND_STANDBY_SHOUT_STREAM, &[data]);
        cdecl(h, SND_DATA_SEND_SYNC, &[1]);
        cdecl(h, SQS_REPORT_STANDBY_STATUS, &[1]);
        cdecl(h, USER_FREE, &[data]);
        mem_checks(h)
    }

    /// `code_0088ded0()` (after `TkInitScene`): CD file 0x18 part 5
    /// (`+0x10(0x18)`, `size = +0x14(5)`), `TkCdReadWithMalloc(0x18, 5)`,
    /// `SqsShoutStreamingDataSend_thunk(data, size)`,
    /// `SndDataSendSync(1)`; a loaded block is freed and the memory
    /// checked. Returns the last call's `eax`.
    pub fn send_shout_stream(h: &mut impl Host) -> u32 {
        let (_, size) = cd_hooks(h, 0x18, 5);
        let data = cdecl(h, TK_CD_READ_WITH_MALLOC, &[0x18, 5]);
        cdecl(h, SQS_SHOUT_STREAMING_DATA_SEND_THUNK, &[data, size]);
        let r = cdecl(h, SND_DATA_SEND_SYNC, &[1]);
        if data == 0 {
            return r;
        }
        cdecl(h, USER_FREE, &[data]);
        mem_checks(h)
    }

    /// `Pause_Leave_008ab2d0()` (`Pause_Update`'s exits): the pause flags
    /// cleared, the time spent paused added to the total (64-bit µs:
    /// `graphicGetTimeInMicroseconds() - start`), the free camera off,
    /// the sounds resumed, the pause's input released (`0x006f32d0`), the
    /// input updated; with a 3D-scene movie (`movie_with3DScene`) or a
    /// movie picture, a tail jump to `graphicVideoPause`. Returns that
    /// `eax`, else 0 (the movie test's).
    pub fn pause_leave(h: &mut impl Host) -> u32 {
        PAUSE_ENTERED.set(h, 0);
        PAUSE_REQUEST.set(h, 0);
        PAUSE_ON.set(h, 0);
        let now = h.call_sym_u64(GRAPHIC_GET_TIME_US, Conv::Cdecl, &[]);
        let spent = now.wrapping_sub(PAUSE_TIME_START.get(h));
        let total = PAUSE_TIME_TOTAL.get(h).wrapping_add(spent);
        PAUSE_TIME_TOTAL.set(h, total);
        cdecl(h, GRAPHIC_SET_FREE_CAMERA_MODE, &[0]);
        cdecl(h, SG_SET_PAUSE_ALL_SOUNDS, &[0]);
        cdecl(h, PAUSE_INPUT_RESUME, &[]);
        cdecl(h, INPUT_UPDATE_ALL, &[]);
        let movie = cdecl(h, MOVIE_WITH_3D_SCENE, &[]);
        if movie != 0 || MOVIE_PICTURE_ON.get(h) != 0 {
            return cdecl(h, GRAPHIC_VIDEO_PAUSE, &[]);
        }
        movie
    }

    /// The battle-end slide of a fade word (`0..=0x1000`) and its speed:
    /// 0 at 0x1000; fading in (speed > 0) `Math_CosLookup(fade)`, else
    /// `(0x1000 - Math_SinLookup(fade)) * 5 / 4` (both lookups' low 16
    /// bits, zero-extended); then `* 512 / 4096` (signed, towards 0).
    fn slide(h: &mut impl Host, fade: i16, speed: i16) -> i32 {
        let x = if fade == 0x1000 {
            0
        } else if speed > 0 {
            i32::from(cdecl(h, MATH_COS_LOOKUP, &[i32::from(fade) as u32]) as u16)
        } else {
            let s = i32::from(cdecl(h, MATH_SIN_LOOKUP, &[i32::from(fade) as u32]) as u16);
            (0x1000 - s) * 5 / 4
        };
        (x << 9) / 4096
    }

    /// `code_008bc080()` (the battle-end step after `BtlEnd_StepEntry`):
    /// with `g_BtlEndFadeGil` set, the item, gear and gil pages drawn
    /// shifted by the slide (`-d`, `d`, `-d`).
    pub fn draw_gil_pages(h: &mut impl Host) {
        let fade = BTL_END_FADE_GIL.get(h);
        if fade == 0 {
            return;
        }
        let d = slide(h, fade, BTL_END_FADE_GIL_SPEED.get(h));
        cdecl(h, BTL_END_DRAW_ITEM_PAGE, &[d.wrapping_neg() as u32]);
        cdecl(h, BTL_END_DRAW_WEAPON_PAGE, &[d as u32]);
        cdecl(h, BTL_END_DRAW_GIL_PAGE, &[d.wrapping_neg() as u32]);
    }

    /// `code_008bd940()`: as [`draw_gil_pages`] for the spoil fade
    /// (`0x0186a168`/`0x0186a16c`): the spoil list, exchange and spoil
    /// pages (`-d`, `d`, `-d`).
    pub fn draw_spoil_pages(h: &mut impl Host) {
        let fade = BTL_END_FADE_SPOIL.get(h);
        if fade == 0 {
            return;
        }
        let d = slide(h, fade, BTL_END_FADE_SPOIL_SPEED.get(h));
        cdecl(h, BTL_END_DRAW_SPOIL_LIST_PAGE, &[d.wrapping_neg() as u32]);
        cdecl(h, BTL_END_DRAW_EXCHANGE_PAGE, &[d as u32]);
        cdecl(h, BTL_END_DRAW_SPOIL_PAGE, &[d.wrapping_neg() as u32]);
    }

    /// `code_008bd9c0()`: every battle-end spoil (`i8` count, re-read each
    /// round) into the save: `MsSaveItemUse(u16 item, u8 amount)`.
    pub fn take_spoils(h: &mut impl Host) {
        let mut i = 0u32;
        while i32::from(BTL_END_SPOIL_COUNT.get(h)) > i as i32 {
            let amount: u8 = h.read(h.rebase(BTL_END_SPOIL_AMOUNTS + i));
            let item: u16 = h.read(h.rebase(BTL_END_SPOIL_ITEMS + 2 * i));
            cdecl(h, MS_SAVE_ITEM_USE, &[u32::from(item), u32::from(amount)]);
            i += 1;
        }
    }

    /// `code_008beff0()` (`g_MenuHelpExtArg + 8`): the help screen's
    /// draw: `TkVU1SyncPath`, `0x008bef30`, the full scissor and a dark
    /// box (`TODrawBoxXYWHC2(0, 0, 0x200, 0x1a0, 0x80000000 x2)`),
    /// `TOMenuDrawKickTmp`, the list drawn through `[0x00c416b0]`
    /// between two syncs, then a tail jump to `TODapsDraw`.
    pub fn help_draw(h: &mut impl Host) -> u32 {
        cdecl(h, TK_VU1_SYNC_PATH, &[]);
        cdecl(h, HELP_DRAW_PREPARE, &[]);
        cdecl(h, TO_DRAW_SCISSOR_XYWH, &[0, 0, 0x200, 0x1a0]);
        cdecl(
            h,
            TO_DRAW_BOX_XYWHC2,
            &[0, 0, 0x200, 0x1a0, 0x8000_0000, 0x8000_0000],
        );
        cdecl(h, TO_MENU_DRAW_KICK_TMP, &[]);
        cdecl(h, TK_VU1_SYNC_PATH, &[]);
        let draw = MENU_LIST_DRAW_PTR.get(h);
        h.call(draw, Conv::Cdecl, &[]);
        cdecl(h, TK_VU1_SYNC_PATH, &[]);
        cdecl(h, TO_DAPS_DRAW, &[])
    }
    pub const HELP_DRAW_PREPARE: u32 = 0x008b_ef30;

    /// `code_008c3340()` (`g_MenuHelpExtArg + 0x28`): with the help
    /// window's `u16 +0x30` set, `TkMenuMainSleepMenu(0xe)` and a tail
    /// jump to `TOMenuInitBrush`; else returns the window.
    pub fn help_close(h: &mut impl Host) -> u32 {
        let w = MENU_HELP_WINDOW.get(h);
        if h.read::<u16>(w.wrapping_add(0x30)) == 0 {
            return w;
        }
        cdecl(h, TK_MENU_MAIN_SLEEP_MENU, &[0xe]);
        cdecl(h, TO_MENU_INIT_BRUSH, &[])
    }

    /// The menu draw prologue: `TkMenuDrawKick` twice, `TkVU1SyncPath`
    /// (thunk `0x008c1790`), `TkMn2DrawInitPacket`.
    fn draw_begin(h: &mut impl Host) {
        cdecl(h, TK_MENU_DRAW_KICK, &[]);
        cdecl(h, TK_MENU_DRAW_KICK, &[]);
        cdecl(h, TK_VU1_SYNC_PATH_THUNK, &[]);
        cdecl(h, TK_MN2_DRAW_INIT_PACKET, &[]);
    }

    /// `code_008c3360()`, `code_008d7560()`, `code_008d9aa0()`: an empty
    /// screen's draw: the prologue, `TkMn2DrawKickSyncPacket`, a tail
    /// jump to `TODrawMenuBG`.
    pub fn draw_empty_screen(h: &mut impl Host) -> u32 {
        draw_begin(h);
        cdecl(h, TK_MN2_DRAW_KICK_SYNC_PACKET, &[]);
        cdecl(h, TO_DRAW_MENU_BG, &[])
    }

    /// `code_008c5ee0()`: the item screen's draw step byte `0x0186a390`
    /// incremented, a tail jump to `TODrawMenuBG`.
    pub fn item_step_draw_bg(h: &mut impl Host) -> u32 {
        let s = MENU_ITEM_DRAW_STEP.get(h);
        MENU_ITEM_DRAW_STEP.set(h, s.wrapping_add(1));
        cdecl(h, TO_DRAW_MENU_BG, &[])
    }

    /// `code_008c5ef0()`: `TkMenuSetScreenX2Flag(1)`,
    /// `MenuItem_GetLinePos`, `TOSeqGetRect(1, name, x, y, w, h)` of the
    /// item list rectangle into `0x0186a410..0x0186a416`; returns its
    /// `eax`.
    pub fn item_open(h: &mut impl Host) -> u32 {
        cdecl(h, TK_MENU_SET_SCREEN_X2_FLAG, &[1]);
        cdecl(h, MENU_ITEM_GET_LINE_POS, &[]);
        seq_get_rect(h, ITEM_RECT_NAME, ITEM_RECT_OUT)
    }

    /// `TOSeqGetRect(1, name, out, out + 2, out + 4, out + 6)`.
    fn seq_get_rect(h: &mut impl Host, name: u32, out: u32) -> u32 {
        let name = h.rebase(name);
        let o = h.rebase(out);
        cdecl(h, TO_SEQ_GET_RECT, &[1, name, o, o + 2, o + 4, o + 6])
    }

    /// `code_008c6050()`: the item screen's exit: its four windows closed
    /// (1, 3, 4), the two scenes stopped (`TOSeqStopScene(1, ..)`), the
    /// windows 2, 3, 2, 4 closed again, a tail jump to
    /// `TkMenuCloseSubCmdWindow`.
    pub fn item_close(h: &mut impl Host) -> u32 {
        cdecl(h, TK_MENU_ITEM_CLOSE_WINDOW, &[]);
        cdecl(h, TK_MENU_ITEM_CLOSE_WINDOW3, &[]);
        cdecl(h, TK_MENU_ITEM_CLOSE_WINDOW4, &[]);
        let a = h.rebase(ITEM_SCENE_A);
        cdecl(h, TO_SEQ_STOP_SCENE, &[1, a]);
        let b = h.rebase(ITEM_SCENE_B);
        cdecl(h, TO_SEQ_STOP_SCENE, &[1, b]);
        cdecl(h, TK_MENU_ITEM_CLOSE_WINDOW2, &[]);
        cdecl(h, TK_MENU_ITEM_CLOSE_WINDOW3, &[]);
        cdecl(h, TK_MENU_ITEM_CLOSE_WINDOW2, &[]);
        cdecl(h, TK_MENU_ITEM_CLOSE_WINDOW4, &[]);
        cdecl(h, TK_MENU_CLOSE_SUB_CMD_WINDOW, &[])
    }

    /// `code_008c60a0()`: `TkMenuSetScreenX2Flag(0)`; returns its `eax`.
    pub fn screen_x2_off(h: &mut impl Host) -> u32 {
        cdecl(h, TK_MENU_SET_SCREEN_X2_FLAG, &[0])
    }

    /// `code_008c64a0()`: the step byte incremented, `TODrawMenuBG`, the
    /// current player's status plate at (`remapX2(f32 0x00b5ef1c)`,
    /// `remapY2(f32 0x00b659b4)`); returns its `eax`.
    pub fn item_draw_plate(h: &mut impl Host) -> u32 {
        let s = MENU_ITEM_DRAW_STEP.get(h);
        MENU_ITEM_DRAW_STEP.set(h, s.wrapping_add(1));
        cdecl(h, TO_DRAW_MENU_BG, &[]);
        let who = cdecl(h, TK_MENU_GET_CURRENT_PLAYER, &[]);
        status_plate(h, who, ITEM_PLATE_Y)
    }

    /// `TOMenuDrawChrStatusPlate(who, remapX2(f32 0x00b5ef1c),
    /// remapY2(f32 y))`: the remaps run Y first, as the original does;
    /// each result is stored as `f32` (`fstp dword`).
    pub fn status_plate(h: &mut impl Host, who: u32, y: u32) -> u32 {
        let y: u32 = h.read(h.rebase(y));
        let y = h.call_sym_st0(GRAPHIC_UI_REMAP_Y2, Conv::Cdecl, &[y]);
        let x: u32 = h.read(h.rebase(ITEM_PLATE_X));
        let x = h.call_sym_st0(GRAPHIC_UI_REMAP_X2, Conv::Cdecl, &[x]);
        cdecl(
            h,
            TO_MENU_DRAW_CHR_STATUS_PLATE,
            &[who, x.to_bits(), y.to_bits()],
        )
    }

    /// `code_008c64f0()`: `TkMn2GetWShopPos`, `TkMn2GetIShopPos`,
    /// `TOSeqGetRect(1, name, ..)` into `0x0186a418..0x0186a41e`.
    pub fn shop_open_rect(h: &mut impl Host) -> u32 {
        cdecl(h, TK_MN2_GET_WSHOP_POS, &[]);
        cdecl(h, TK_MN2_GET_ISHOP_POS, &[]);
        seq_get_rect(h, ITEM_RECT_NAME2, ITEM_RECT_OUT2)
    }
}
