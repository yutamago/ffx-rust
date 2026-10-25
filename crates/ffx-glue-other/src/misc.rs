//! Forwarders, menu, Iggy, text and PS2-emulation helpers of the glue:
//! jumps to another function, calls through function pointers, the
//! menus' small getters and setters.

use ffx_bridge::{Global, Mem, x87};
use ffx_graphic::host::{Conv, Host};

/// `PhyreFIOS_UnloadPrx(handle)` (cdecl).
pub const UNLOAD_PRX: u32 = 0x006f_9d20;
/// `ClusterManager_loadPCluster(this; path)` (thiscall).
pub const LOAD_PCLUSTER: u32 = 0x0069_b940;
/// `MsSaveAddAP_00784610(chr, n)` (cdecl).
pub const SAVE_ADD_AP: u32 = 0x0078_4610;
/// `IggyMenuManager_SetViewport_00a26a20(this; w, h)` (thiscall).
pub const SET_VIEWPORT: u32 = 0x00a2_6a20;
/// `TOClosePktbuff()` (a bare `ret`).
pub const CLOSE_PKTBUFF: u32 = 0x008f_3930;
/// `TODrawKick()`.
pub const DRAW_KICK: u32 = 0x008f_5bc0;
/// `dbgPrintf(format, ...)` (a bare `ret`).
pub const DBG_PRINTF: u32 = 0x0062_f500;
/// `sceDmaSync(chan, mode, timeout)`.
pub const DMA_SYNC: u32 = 0x0062_f730;
/// `sceDmaGetChan(2)`'s channel (`Sg_MainInit` stores it).
pub const DMA_CHAN2: Global<u32> = Global::new(0x023c_bbf4);
/// `g_GsRegisterSink`: bit 8 is the busy flag `Sg_MFIFOSpadSync` waits on.
pub const GS_REGISTER_SINK: Global<u32> = Global::new(0x0230_56e8);
/// `IggyMenu_setEnable(this; on)` (thiscall).
pub const IGGY_SET_ENABLE: u32 = 0x00a2_8ef0;
/// The save menu's `IggyMenu` (`0x00ceb248`).
pub const SAVE_MENU: Global<u32> = Global::new(0x00ce_b248);
/// `Virtuos_InputManager_getInstance()`.
pub const INPUT_MANAGER: u32 = 0x0063_1300;
/// `Virtuos_InputManager_getVibrationFlag(this)` (`al`).
pub const VIBRATION_FLAG: u32 = 0x0063_1320;
/// `TextUtil +0x80`: enabled (byte).
pub const TEXT_ENABLED: u32 = 0x80;
/// `FFEscMenu_buildResolutionList(this)`.
pub const BUILD_RESOLUTION_LIST: u32 = 0x006f_edf0;
/// `FFEscMenu_initializeVideoSettingPage(this)`.
pub const INIT_VIDEO_PAGE: u32 = 0x006f_cca0;
/// The import thunk slot of `_IggyValueRefFree@8` (`0x00b0c820`).
pub const IGGY_VALUE_REF_FREE: Global<u32> = Global::new(0x00b0_c820);
/// `g_PhyreScene`.
pub const PHYRE_SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// `PhyreScene_setMeshVisible(scene; mesh, on)` (thiscall).
pub const SET_MESH_VISIBLE: u32 = 0x0066_d910;
/// `ClassDynamicGeometry +0xc`: its mesh.
pub const GEOMETRY_MESH: u32 = 0xc;
/// `MenuManager +0x5b7c`: which count [`menu_count`] returns.
pub const MENU_ALT: u32 = 0x5b7c;
/// `MenuManager +0x2c50`: the menu count.
pub const MENU_COUNT: u32 = 0x2c50;
/// `MenuManager +0x5840`: the other count.
pub const MENU_ALT_COUNT: u32 = 0x5840;
/// `operator_new(n)`.
pub const OPERATOR_NEW: u32 = 0x0063_0310;
/// The one-byte object `graphicInitialize` creates once (`0x00ce5194`).
pub const ONE_BYTE: Global<u32> = Global::new(0x00ce_5194);
/// `TextUtil +0x84`, `+0x88`: the two floats `0x006f6910` sets.
pub const TEXT_POSITION: u32 = 0x84;
/// `g_IggyMenuManager`.
pub const IGGY_MENU_MANAGER: Global<u32> = Global::new(0x0198_4c6c);
/// `IggyMenuManager_addCallback` `0x00a266b0` (thiscall; menu, a, b).
pub const ADD_CALLBACK: u32 = 0x00a2_66b0;
/// `Phyre_PString_Assign(this; s)` (thiscall).
pub const PSTRING_ASSIGN: u32 = 0x0044_5cf0;
/// `TextUtil +0x7c`: the title string; `+0x70` its dirty flag.
pub const TEXT_TITLE: u32 = 0x7c;
/// `TextUtil +0x70`.
pub const TEXT_TITLE_DIRTY: u32 = 0x70;
/// `TextUtil_render(this; a)` (thiscall).
pub const TEXT_RENDER: u32 = 0x006f_65b0;
/// `TOGetInternationalBaseDicType()`.
pub const BASE_DIC_TYPE: u32 = 0x008a_c760;
/// `TOGetFontWidth(a, b, c, dic)` (cdecl).
pub const FONT_WIDTH: u32 = 0x008b_9330;
/// `PhyreScene_GetSplitScreenStat(scene)`.
pub const GET_SPLIT_SCREEN: u32 = 0x0065_17d0;
/// `PhyreScene_SetSplitScreenStat(scene; v)`.
pub const SET_SPLIT_SCREEN: u32 = 0x0065_4550;
/// `SleepThread()`.
pub const SLEEP_THREAD: u32 = 0x006f_2d10;
/// `g_TOPktCursor`.
pub const PKT_CURSOR: Global<u32> = Global::new(0x0187_16cc);
/// `Identity_008f2f70(x)` -> `x`.
pub const IDENTITY: u32 = 0x008f_2f70;
/// `TOCtrlPkt()`.
pub const CTRL_PKT: u32 = 0x008f_3a30;

/// `PhyreFIOS_UnloadMagicfilePrx(handle)` (0x006794f0): a jump to
/// `PhyreFIOS_UnloadPrx`.
pub fn unload_magicfile_prx(h: &mut impl Host, handle: u32) -> u32 {
    h.call_sym(UNLOAD_PRX, Conv::Cdecl, &[handle])
}

/// `ClusterManager_loadPClusterJump_0069bb00(this; path)` (thiscall): a
/// jump to `ClusterManager_loadPCluster`.
pub fn load_pcluster_jump(h: &mut impl Host, this: u32, path: u32) -> u32 {
    h.call_sym(LOAD_PCLUSTER, Conv::Thiscall(this), &[path])
}

/// `MsSaveAddAPJump_008ba390(chr, n)`: a jump to `MsSaveAddAP_00784610`.
pub fn save_add_ap_jump(h: &mut impl Host, chr: u32, n: u32) -> u32 {
    h.call_sym(SAVE_ADD_AP, Conv::Cdecl, &[chr, n])
}

/// `IggyMenuManager_SetViewportJump_00a26960(this; w, h)`: a jump to
/// `IggyMenuManager_SetViewport_00a26a20`.
pub fn set_viewport_jump(h: &mut impl Host, this: u32, w: u32, hh: u32) -> u32 {
    h.call_sym(SET_VIEWPORT, Conv::Thiscall(this), &[w, hh])
}

/// `AsyncLoadManager_RegistTaskRequest_run_00691350(this)` (thiscall,
/// slot 1 of `AsyncLoadManager_RegistTaskRequest_vftable`):
/// `this->fn(this->arg)` (cdecl, `+4` and `+8`); its `eax`.
pub fn regist_task_request_run(h: &mut impl Host, this: u32) -> u32 {
    let arg = h.read::<u32>(this + 8);
    let f = h.read::<u32>(this + 4);
    h.call(f, Conv::Cdecl, &[arg])
}

/// `TOMenuDrawKickTmp()` (0x008bea20): `TOClosePktbuff()`, then a jump to
/// `TODrawKick()`, whose `eax` it returns.
pub fn menu_draw_kick_tmp(h: &mut impl Host) -> u32 {
    h.call_sym(CLOSE_PKTBUFF, Conv::Cdecl, &[]);
    h.call_sym(DRAW_KICK, Conv::Cdecl, &[])
}

/// `movie_err`, `movie_init`, `movie_start_prog`, `movie_stop_prog`: the
/// PS2 movie stubs, `dbgPrintf(message)` (an `.rdata` string; `dbgPrintf`
/// is a bare `ret`). Callers read no result.
pub fn movie_message(h: &mut impl Host, message: u32) {
    let m = h.rebase(message);
    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[m]);
}

/// `movie_err`'s message.
pub const MOVIE_ERR: u32 = 0x00b5_2658;
/// `movie_init`'s message.
pub const MOVIE_INIT: u32 = 0x00b5_2268;
/// `movie_start_prog`'s message.
pub const MOVIE_START_PROG: u32 = 0x00b5_2570;
/// `movie_stop_prog`'s message.
pub const MOVIE_STOP_PROG: u32 = 0x00b5_23a0;

/// `Sg_MFIFOSpadSync()` (0x008432d0): waits while bit 8 of
/// `g_GsRegisterSink` is set; returns the word it last read.
pub fn mfifo_spad_sync(h: &impl Mem) -> u32 {
    loop {
        let v = GS_REGISTER_SINK.get(h);
        if v & 0x100 == 0 {
            return v;
        }
    }
}

/// `Sg_GsSync()` (0x00841d60): `sceDmaSync(channel 2, 0, 0)`'s `eax`.
pub fn gs_sync(h: &mut impl Host) -> u32 {
    let chan = DMA_CHAN2.get(h);
    h.call_sym(DMA_SYNC, Conv::Cdecl, &[chan, 0, 0])
}

/// `SaveDataMachine_disableMenu_00702a70()`: `IggyMenu_setEnable(save
/// menu; 0)`'s `eax`.
pub fn save_menu_disable(h: &mut impl Host) -> u32 {
    let m = SAVE_MENU.get(h);
    h.call_sym(IGGY_SET_ENABLE, Conv::Thiscall(m), &[0])
}

/// `IggyMenuHandlers_callCdecl_00a28c00(ctx, f, arg)` (cdecl, the
/// handler `IggyMenuHandlers_addCall` stores): `f(arg)`'s `eax`.
pub fn iggy_call_cdecl(h: &mut impl Host, f: u32, arg: u32) -> u32 {
    h.call(f, Conv::Cdecl, &[arg])
}

/// `FFEscMenu_callMember_0068dfa0(this, f, arg)` (cdecl, the escape menu's
/// sound callbacks): `f` as a thiscall method of `this` with one word;
/// its `eax`.
pub fn esc_menu_call_member(h: &mut impl Host, this: u32, f: u32, arg: u32) -> u32 {
    h.call(f, Conv::Thiscall(this), &[arg])
}

/// `RemapButtonIcon_getCurrentControlMachine()` (0x006e6450): the input
/// manager's vibration flag (`al`, zero-extended).
pub fn current_control_machine(h: &mut impl Host) -> u32 {
    let m = h.call_sym(INPUT_MANAGER, Conv::Cdecl, &[]);
    h.call_sym(VIBRATION_FLAG, Conv::Thiscall(m), &[]) & 0xff
}

/// `TextUtil_setEnabled(this; on)` (0x006f68f0, thiscall): the byte at
/// `+0x80`. Callers read no result.
pub fn text_set_enabled(h: &mut impl Mem, this: u32, on: u32) {
    h.write(this + TEXT_ENABLED, on as u8);
}

/// `FFEscMenu_rebuildVideoSettingPage_006ff020(this)` (thiscall):
/// `FFEscMenu_buildResolutionList(this)`, then a jump to
/// `FFEscMenu_initializeVideoSettingPage(this)`, whose `eax` it returns.
pub fn rebuild_video_setting_page(h: &mut impl Host, this: u32) -> u32 {
    h.call_sym(BUILD_RESOLUTION_LIST, Conv::Thiscall(this), &[]);
    h.call_sym(INIT_VIDEO_PAGE, Conv::Thiscall(this), &[])
}

/// `crossThreadImpl_run_006f29f0(this)` (thiscall, slot 1 of
/// `crossThreadImpl_vftable`): `this->fn(this->arg)` (cdecl, `+0x10` and
/// `+0x14`) when `fn` is set; returns 0.
pub fn cross_thread_run(h: &mut impl Host, this: u32) -> u32 {
    let f = h.read::<u32>(this + 0x10);
    if f != 0 {
        let arg = h.read::<u32>(this + 0x14);
        h.call(f, Conv::Cdecl, &[arg]);
    }
    0
}

/// `IggyValueRef_free_00a28a20(this)` (thiscall): when the byte at `+0`
/// is set, `IggyValueRefFree(this->+0x38, this->+4)` (stdcall import).
/// Callers read no result.
pub fn iggy_value_ref_free(h: &mut impl Host, this: u32) {
    if h.read::<u8>(this) == 0 {
        return;
    }
    let a = h.read::<u32>(this + 0x38);
    let b = h.read::<u32>(this + 4);
    let f = IGGY_VALUE_REF_FREE.get(h);
    h.call(f, Conv::Stdcall, &[a, b]);
}

/// `ClassDynamicGeometry_hide` (0x006a2370) / `_show` (0x006a49e0)
/// (thiscall): `PhyreScene_setMeshVisible(g_PhyreScene; mesh, on)`'s
/// `eax` when the geometry has a mesh, else 0 (the tested mesh).
pub fn geometry_set_visible(h: &mut impl Host, this: u32, on: u32) -> u32 {
    let mesh = h.read::<u32>(this + GEOMETRY_MESH);
    if mesh == 0 {
        return 0;
    }
    let scene = PHYRE_SCENE.get(h);
    h.call_sym(SET_MESH_VISIBLE, Conv::Thiscall(scene), &[mesh, on])
}

/// `MenuManager_GetMenuCount(this)` (0x0067b970, thiscall).
pub fn menu_count(h: &impl Mem, this: u32) -> u32 {
    if h.read::<u32>(this + MENU_ALT) == 0 {
        h.read(this + MENU_COUNT)
    } else {
        h.read(this + MENU_ALT_COUNT)
    }
}

/// `IggyValue_setType9_00a28dd0(value, v)` (cdecl): unless `value` is
/// null, type word 9 at `+0` and `v` at `+8`. Callers read no result.
pub fn iggy_value_set_type9(h: &mut impl Mem, value: u32, v: u32) {
    if value != 0 {
        h.write(value, 9u32);
        h.write(value + 8, v);
    }
}

/// `graphicInitialize_allocOnce_006e5490()`: `operator_new(1)` into
/// `0x00ce5194` unless it is set. Its caller reads no result.
pub fn alloc_once(h: &mut impl Host) {
    if ONE_BYTE.get(h) == 0 {
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[1]);
        ONE_BYTE.set(h, p);
    }
}

/// `TextUtil_setPosition_006f6910(this; x, y)` (thiscall): two floats
/// through `fld`/`fstp` at `+0x84`, `+0x88`. Callers read no result.
pub fn text_set_position(h: &mut impl Mem, this: u32, x: u32, y: u32) {
    let x = x87::round_trip(f32::from_bits(x));
    h.write(this + TEXT_POSITION, x.to_bits());
    let y = x87::round_trip(f32::from_bits(y));
    h.write(this + TEXT_POSITION + 4, y.to_bits());
}

/// `IggyMenu_addCallback_00a28e90(this; a, b)` (thiscall):
/// `IggyMenuManager_addCallback(g_IggyMenuManager; this, a, b)`'s `eax`.
pub fn iggy_menu_add_callback(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
    let m = IGGY_MENU_MANAGER.get(h);
    h.call_sym(ADD_CALLBACK, Conv::Thiscall(m), &[this, a, b])
}

/// `TextUtil_setTitleText(this; s)` (0x006f6930, thiscall):
/// `Phyre_PString_Assign(this + 0x7c; s)`, then the dirty byte at `+0x70`;
/// the assignment's `eax`.
pub fn text_set_title(h: &mut impl Host, this: u32, s: u32) -> u32 {
    let r = h.call_sym(PSTRING_ASSIGN, Conv::Thiscall(this + TEXT_TITLE), &[s]);
    h.write(this + TEXT_TITLE_DIRTY, 1u8);
    r
}

/// `VideoText_render_006ddeb0(this; a)` (thiscall): when the bytes at `+4`
/// and `+5` are both set, a jump to `TextUtil_render(this->+0; a)`.
/// Callers read no result.
pub fn video_text_render(h: &mut impl Host, this: u32, a: u32) {
    if h.read::<u8>(this + 4) == 0 || h.read::<u8>(this + 5) == 0 {
        return;
    }
    let text = h.read::<u32>(this);
    h.call_sym(TEXT_RENDER, Conv::Thiscall(text), &[a]);
}

/// `TOGetFontWidthInterMode(a, b, c)` (0x008b7be0):
/// `TOGetFontWidth(a, b, c, TOGetInternationalBaseDicType())`'s `eax`.
pub fn font_width_inter_mode(h: &mut impl Host, a: u32, b: u32, c: u32) -> u32 {
    let dic = h.call_sym(BASE_DIC_TYPE, Conv::Cdecl, &[]);
    h.call_sym(FONT_WIDTH, Conv::Cdecl, &[a, b, c, dic])
}

/// `graphicResetSplitScreen_00638d20()`: when the scene's split-screen
/// state is 1, `PhyreScene_SetSplitScreenStat(scene; 0)`'s `eax`; else the
/// state.
pub fn reset_split_screen(h: &mut impl Host) -> u32 {
    let scene = PHYRE_SCENE.get(h);
    let stat = h.call_sym(GET_SPLIT_SCREEN, Conv::Thiscall(scene), &[]);
    if stat != 1 {
        return stat;
    }
    let scene = PHYRE_SCENE.get(h);
    h.call_sym(SET_SPLIT_SCREEN, Conv::Thiscall(scene), &[0])
}

/// `AsyncLoadManager_waitTasks_00691d40(this)` (thiscall): `SleepThread()`
/// while `+0x20` or `+0x2c` is set and `+0x14` too. Its caller reads no
/// result.
pub fn wait_tasks(h: &mut impl Host, this: u32) {
    loop {
        let busy = h.read::<u32>(this + 0x20) != 0 || h.read::<u32>(this + 0x2c) != 0;
        if !busy || h.read::<u32>(this + 0x14) == 0 {
            return;
        }
        h.call_sym(SLEEP_THREAD, Conv::Cdecl, &[]);
    }
}

/// `TOMkpSetEnvSubFont()` (0x00903300): `g_TOPktCursor =
/// Identity_008f2f70(g_TOPktCursor)`, then a jump to `TOCtrlPkt()`, whose
/// `eax` it returns.
pub fn set_env_sub_font(h: &mut impl Host) -> u32 {
    let c = PKT_CURSOR.get(h);
    let c = h.call_sym(IDENTITY, Conv::Cdecl, &[c]);
    PKT_CURSOR.set(h, c);
    h.call_sym(CTRL_PKT, Conv::Cdecl, &[])
}
