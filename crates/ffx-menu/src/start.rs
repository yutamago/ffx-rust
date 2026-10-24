//! The Tk library's boot (`TkInit`, from `Sg_MainInit`) and the menu's
//! entry (`TkMenuStart`, from `Sg_MainCheckMenu` and `0x008be840`).
//!
//! `TkInit` reads its sound and vibration files through the CD interface
//! `GetCD()` returns, a table of function pointers:
//!
//! | offset | call |
//! |---|---|
//! | `+0x00` | read `(file, buffer, 0, 0)` |
//! | `+0x04` | (`(0)`: the open file's position, for the wave/music info) |
//! | `+0x08` | wait `(0)` |
//! | `+0x10` | pick the directory `(dir)` |
//! | `+0x14` | the size of file `(file)` in the picked directory |

/// The callees, by address.
pub mod callee {
    pub const INIT_WORK: u32 = 0x0088_e570;
    pub const IS_DEBUG: u32 = 0x0088_7d10;
    pub const INIT_DECI2: u32 = 0x0088_5240;
    pub const RESET_CLIP: u32 = 0x0088_c710;
    pub const INIT_ALL_SYNC_V_CALLBACK: u32 = 0x0088_86f0;
    pub const CLEAR_ALARM: u32 = 0x0088_c7d0;
    pub const PAD_INIT_WORKS: u32 = 0x0088_9340;
    pub const INIT_SCENE_FROM_SAVE: u32 = 0x0088_dd60;
    pub const SND_SET_MUSIC_MUTE: u32 = 0x0088_7160;
    pub const INIT_START_MAP: u32 = 0x0088_e880;
    pub const READ_THREAD_INIT: u32 = 0x0076_e630;
    pub const PRINTF_CUT: u32 = 0x0076_de90;
    pub const GET_CD: u32 = 0x0076_d0d0;
    pub const MEM_SET_ALIGN: u32 = 0x0083_c4a0;
    pub const USER_MALLOC: u32 = 0x0068_7020;
    pub const USER_FREE: u32 = 0x0068_7010;
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    pub const TO_INIT: u32 = 0x008a_d4d0;
    pub const ATEL_INIT_TOTAL: u32 = 0x0086_d6d0;
    pub const GET_LANG: u32 = 0x0088_7c30;
    pub const SET_PROJECT_BASE: u32 = 0x0088_7a90;
    pub const GET_VIDEO_MODE: u32 = 0x0088_7c40;
    /// `jmp SqsSetCycleMode`.
    pub const SET_CYCLE_MODE: u32 = 0x0088_7070;
    pub const SND_INIT_WORK: u32 = 0x0088_6550;
    pub const INIT_WAVE_INFO: u32 = 0x0081_d3f0;
    pub const INIT_MUSIC_INFO: u32 = 0x0081_d340;
    pub const MEM_CHECK: u32 = 0x0083_b7b0;
    pub const SORT_COMMON_WAVE: u32 = 0x0088_72a0;
    pub const STANDBY_SHOUT_STREAM: u32 = 0x0081_e3b0;
    pub const DATA_SEND_SYNC: u32 = 0x0088_5ee0;
    pub const REPORT_STANDBY_STATUS: u32 = 0x0081_e2d0;
    /// `jmp SqsShoutStreamingDataSend`.
    pub const SHOUT_STREAMING_DATA_SEND: u32 = 0x0088_7560;
    pub const GET_HDD_NAME: u32 = 0x0077_7eb0;
    pub const INIT_WAVE_INFO_HDD: u32 = 0x0081_d470;
    pub const INIT_MUSIC_INFO_HDD: u32 = 0x0081_d3c0;
    pub const ABM_LOAD: u32 = 0x00a5_4810;

    pub const MAIN_LOOP: u32 = 0x008a_9ca0;
    pub const SET_TK_ANTI_XYB: u32 = 0x0083_b110;
    pub const ENABLE_TK_ANTI: u32 = 0x0083_add0;
    pub const PAUSE_REQUEST: u32 = 0x008a_ae00;
    pub const MN2_INIT_WORK: u32 = 0x008c_1c50;
    pub const MN2_GET_MAIN_POS: u32 = 0x008e_2630;
    pub const SHADE_RATIO_DIFF: u32 = 0x008a_9670;
    pub const SET_SHADE_RATIO_STEP: u32 = 0x008a_9680;
    pub const TASK_BUFF_SIZE_GET: u32 = 0x0077_a180;
    pub const ASSERT: u32 = 0x0090_ae50;
    pub const TASK_INIT2: u32 = 0x0077_a1d0;
    pub const PLAYER_LIST_INIT: u32 = 0x008a_8f40;
    pub const SUMMON_LIST_INIT: u32 = 0x008a_9090;
    pub const GET_DBUFF_DC: u32 = 0x0082_0530;
    pub const CALC_IMPORTANT_LIST_MAX: u32 = 0x008c_6790;
    pub const SET_IGNORE_JOIN_FLAG: u32 = 0x008a_ad60;

    pub const MS_GET_SAVE_EVENT_ADDRESS: u32 = 0x0078_5240;
    pub const GET_SCENERIO_FLAG: u32 = 0x0078_7360;
    /// `jmp`s on into the graphics code.
    pub const ACTIVATE_LOADING_SCREEN: u32 = 0x0063_9130;
    pub const INIT_SCENE: u32 = 0x0088_e070;
    pub const ATEL_EVENT_FRAME: u32 = 0x0087_2bd0;
    pub const ATEL_CHECK_MEMORY: u32 = 0x0087_3c30;
    pub const ATEL_MOVE: u32 = 0x0086_3510;
    pub const SND_UPDATE2: u32 = 0x0088_7b80;
}

/// The globals and constants, by address.
pub mod global {
    /// `u8` 1 in `.rdata` (`g_GsDefDrawEnv2`): when 0, `TkInit` also
    /// prints (`printf_cut`) and reads the wave and music info from the
    /// disc (`0x2c`/`0x2d`).
    pub const GS_DEF_DRAW_ENV2: u32 = 0x00b5_ec4f;
    /// Set to 1 by `TkInit`.
    pub const INIT_FLAG: u32 = 0x0133_0848;
    /// `u8`: the language `TkInit` ends with.
    pub const LANG: u32 = 0x0133_0874;
    /// `g_FFXConfig`, 0x40 bytes.
    pub const CONFIG: u32 = 0x0133_8880;
    /// `u8 g_FFXHddMode`.
    pub const HDD_MODE: u32 = 0x0133_c913;
    /// The first file's buffer (file 4 of directory 0x18).
    pub const FILE4_BUFFER: u32 = 0x023c_bec0;
    /// `g_SndStreamTable`.
    pub const SND_STREAM_TABLE: u32 = 0x023c_bec4;
    /// The top of the `u16` decay table (`0x1000`, then `* 9 / 10` 64
    /// times downwards, then 0).
    pub const DECAY_TOP: u32 = 0x023c_bf60;
    /// `g_TkBattleMusicSize`.
    pub const BATTLE_MUSIC_SIZE: u32 = 0x0133_0880;
    /// `g_TkBattleMusicBuffer`.
    pub const BATTLE_MUSIC_BUFFER: u32 = 0x0133_c8c0;
    /// `g_TkCommonVibrate`.
    pub const COMMON_VIBRATE: u32 = 0x0133_88c0;
    /// `"/ffx/proj/sound/us"`.
    pub const PROJECT_US: u32 = 0x00b5_ea78;
    /// `"..\\ps2lib\\common\\tklib.c"`.
    pub const TKLIB_C: u32 = 0x00b5_e968;
    /// `""`.
    pub const EMPTY: u32 = 0x00b8_fbeb;

    /// `g_MovieFading`: the menu is open.
    pub const MENU_OPEN: u32 = 0x0134_0824;
    /// `g_PauseOn`.
    pub const PAUSE_ON: u32 = 0x0134_0828;
    /// The menu's work area cleared at each start, 5 MB.
    pub const MENU_WORK: u32 = 0x0134_0870;
    pub const MENU_WORK_SIZE: u32 = 0x0050_0000;
    /// `u8`, set to 1 at each start.
    pub const STARTED: u32 = 0x0184_0885;
    /// `g_TkMenuMainTask`.
    pub const MAIN_TASK: u32 = 0x0184_0874;
    /// `u8 g_TkMenuTimer`.
    pub const TIMER: u32 = 0x0184_0884;
    /// The main loop's frame counter (`TkMenuMainLoop_008a9ca0`).
    pub const FRAMES: u32 = 0x0184_1c60;
    /// `g_TkMenuFirstReadEnd`.
    pub const FIRST_READ_END: u32 = 0x0134_0859;
    /// Set to `0x83a0` at each start.
    pub const START_WORD: u32 = 0x025d_16d0;
    /// `g_TkMn2MenuMode`.
    pub const MENU_MODE: u32 = 0x0184_08f8;
    /// The task buffer `sa_task_init2` gets.
    pub const TASK_BUFFER: u32 = 0x0158_6f70;
    /// `"SA TASK MEMORY OVERFLOW"`.
    pub const TASK_OVERFLOW: u32 = 0x00b5_f490;
    /// `g_TkEventJumpRequest`: 1 while a jump into the save data's map
    /// waits (`TkUpdateEventJump`).
    pub const EVENT_JUMP_REQUEST: u32 = 0x0133_084c;
    /// `g_TkFadeOut`: two `i32` countdowns `TkUpdateEventJump` runs down
    /// before the jump (the first by 1 a frame, the second at once to 0).
    pub const FADE_OUT: u32 = 0x0133_0850;
}

/// [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{callee as c, global as g};

    /// Zeros for the menu work area's clear.
    static ZEROS: [u8; 0x1_0000] = [0; 0x1_0000];

    /// `(size + 0x7ff) / 0x800 * 0x800` in signed arithmetic (the
    /// original's `cdq` rounding).
    fn sectors(size: u32) -> u32 {
        let s = size.wrapping_add(0x7ff);
        let bias = if s.cast_signed() < 0 { 0x7ff } else { 0 };
        s.wrapping_add(bias) & 0xffff_f800
    }

    /// `TkInit()` (`0x0088d710`): the Tk library's boot. The work, the
    /// debug link (`TkIsDebug`), the vsync callbacks, alarms, pads and the
    /// save's scenes; `g_FFXConfig` cleared and set up (HDD mode kept);
    /// the start map; file 4 of directory 0x18 read into a buffer
    /// ([`g::FILE4_BUFFER`]); `TOInit`, `AtelInitTotal`; the decay table;
    /// the US sound project for languages 1..6, the cycle mode for video
    /// modes 0/1; the sound work; the shout stream (file 0) and the
    /// shout streaming data (file 5, its size from a first look-up) sent;
    /// the battle music (directory 0x26, file 0x10) and the stream table
    /// (directory 0x19, file 0, at least 0x8000 bytes, the old table
    /// freed); the wave and music info (HDD names when installed); the
    /// common vibration (file 8); `eiAbmLoad`. Returns `TkGetLang()`,
    /// whose low byte is kept at [`g::LANG`].
    ///
    /// With [`g::GS_DEF_DRAW_ENV2`] 0 (never: it is 1 in `.rdata`) it
    /// also calls `printf_cut` and reads the wave and music info from
    /// files 0x2c/0x2d. The original also calls two bare `ret`s
    /// (`EmptyStub_0081e400` 98 times, `TkMenuLoadIcon` `0x008a9c90`).
    #[allow(clippy::too_many_lines)] // the boot in turn
    pub fn tk_init<M, C>(mem: &mut M, call: &mut C) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        go(mem, call, c::INIT_WORK, &[]);
        if go(mem, call, c::IS_DEBUG, &[]) != 0 {
            go(mem, call, c::INIT_DECI2, &[]);
            go(mem, call, c::RESET_CLIP, &[]);
        }
        go(mem, call, c::INIT_ALL_SYNC_V_CALLBACK, &[]);
        go(mem, call, c::CLEAR_ALARM, &[]);
        go(mem, call, c::PAD_INIT_WORKS, &[]);
        go(mem, call, c::INIT_SCENE_FROM_SAVE, &[]);
        let at = mem.rebase(g::INIT_FLAG);
        mem.write(at, 1u32);
        let config = mem.rebase(g::CONFIG);
        mem.write_bytes(config, &[0; 0x40]);
        let hdd_at = mem.rebase(g::HDD_MODE);
        let hdd = mem.read::<u8>(hdd_at);
        mem.write(config + 0xe, hdd);
        mem.write(config + 0x11, 0u32);
        for (o, bit) in [(0x11, 2u8), (0x13, 0x20), (0x14, 0xa)] {
            let v = mem.read::<u8>(config + o) | bit;
            mem.write(config + o, v);
        }
        mem.write(config + 0x15, 0u32);
        let mute = i32::from(mem.read::<i8>(config + 0xd));
        let v = mem.read::<u8>(config + 0x17) | 2;
        mem.write(config + 0x17, v);
        mem.write(config + 0xf, 0x101u16);
        go(mem, call, c::SND_SET_MUSIC_MUTE, &[mute.cast_unsigned()]);
        let v = mem.read::<u8>(config + 0xe);
        mem.write(hdd_at, v);
        go(mem, call, c::INIT_START_MAP, &[]);
        go(mem, call, c::READ_THREAD_INIT, &[]);
        if disc_info(mem) {
            go(mem, call, c::PRINTF_CUT, &[]);
        }
        go(mem, call, c::GET_CD, &[]);
        let buffer = read_new(mem, call, 0x18, 4);
        let at = mem.rebase(g::FILE4_BUFFER);
        mem.write(at, buffer);
        go(mem, call, c::TO_INIT, &[]);
        go(mem, call, c::ATEL_INIT_TOTAL, &[]);
        let top = mem.rebase(g::DECAY_TOP);
        mem.write(top, 0x1000u16);
        let mut v = 0x1000i32;
        for k in 1..=0x40u32 {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            mem.write(top - 2 * k, v as u16);
            v = v * 9 / 10;
        }
        mem.write(top - 2 * 0x41, 0u16);
        if go(mem, call, c::GET_LANG, &[]).wrapping_sub(1) <= 5 {
            let p = mem.rebase(g::PROJECT_US);
            go(mem, call, c::SET_PROJECT_BASE, &[p]);
        }
        match go(mem, call, c::GET_VIDEO_MODE, &[]) {
            0 => {
                go(mem, call, c::SET_CYCLE_MODE, &[0]);
            }
            1 => {
                go(mem, call, c::SET_CYCLE_MODE, &[1]);
            }
            _ => {}
        }
        go(mem, call, c::SND_INIT_WORK, &[]);
        if disc_info(mem) {
            disc(mem, call, 0x2c, c::INIT_WAVE_INFO);
        }
        go(mem, call, c::SORT_COMMON_WAVE, &[0]);
        let cd = go(mem, call, c::GET_CD, &[]);
        entry(mem, call, cd, 0x10, &[0x18]);
        entry(mem, call, cd, 0x14, &[0]);
        let buffer = read_new(mem, call, 0x18, 0);
        go(mem, call, c::STANDBY_SHOUT_STREAM, &[buffer]);
        go(mem, call, c::DATA_SEND_SYNC, &[1]);
        go(mem, call, c::REPORT_STANDBY_STATUS, &[1]);
        free(mem, call, buffer);
        // EmptyStub_0081e400(i, 1), a bare `ret`, before each report
        for _ in 0..=0x61 {
            go(mem, call, c::REPORT_STANDBY_STATUS, &[1]);
        }
        if disc_info(mem) {
            disc(mem, call, 0x2d, c::INIT_MUSIC_INFO);
        }
        let cd = go(mem, call, c::GET_CD, &[]);
        entry(mem, call, cd, 0x10, &[0x18]);
        let size5 = entry(mem, call, cd, 0x14, &[5]);
        let buffer = read_new(mem, call, 0x18, 5);
        go(mem, call, c::SHOUT_STREAMING_DATA_SEND, &[buffer, size5]);
        go(mem, call, c::DATA_SEND_SYNC, &[1]);
        free(mem, call, buffer);
        let cd = go(mem, call, c::GET_CD, &[]);
        entry(mem, call, cd, 0x10, &[0x26]);
        let music = entry(mem, call, cd, 0x14, &[0x10]);
        let cd = go(mem, call, c::GET_CD, &[]);
        entry(mem, call, cd, 0x10, &[0x26]);
        entry(mem, call, cd, 0x14, &[0x10]);
        go(mem, call, c::FLUSH_CACHE, &[0]);
        let size_at = mem.rebase(g::BATTLE_MUSIC_SIZE);
        entry(mem, call, cd, 0, &[0x10, size_at, 0, 0]);
        entry(mem, call, cd, 8, &[0]);
        go(mem, call, c::FLUSH_CACHE, &[0]);
        let table_at = mem.rebase(g::SND_STREAM_TABLE);
        let old = mem.read::<u32>(table_at);
        let at = mem.rebase(g::BATTLE_MUSIC_BUFFER);
        mem.write(at, music);
        if old != 0 {
            free(mem, call, old);
        }
        let cd = go(mem, call, c::GET_CD, &[]);
        entry(mem, call, cd, 0x10, &[0x19]);
        let size = entry(mem, call, cd, 0x14, &[0]);
        let size = if size.cast_signed() < 0x8000 {
            0x8000
        } else {
            size
        };
        go(mem, call, c::MEM_SET_ALIGN, &[0x40]);
        let table = go(mem, call, c::USER_MALLOC, &[sectors(size)]);
        mem.write(table, 0u32);
        go(mem, call, c::FLUSH_CACHE, &[0]);
        entry(mem, call, cd, 0, &[0, table, 0, 0]);
        entry(mem, call, cd, 8, &[0]);
        go(mem, call, c::FLUSH_CACHE, &[0]);
        mem.write(table_at, table);
        for (file, init) in [
            (0x2c, c::INIT_WAVE_INFO_HDD),
            (0x2d, c::INIT_MUSIC_INFO_HDD),
        ] {
            let name = if mem.read::<u8>(hdd_at) == 0 {
                mem.rebase(g::EMPTY)
            } else {
                go(mem, call, c::GET_HDD_NAME, &[file])
            };
            go(mem, call, init, &[name]);
        }
        let cd = go(mem, call, c::GET_CD, &[]);
        entry(mem, call, cd, 0x10, &[0x18]);
        entry(mem, call, cd, 0x14, &[8]);
        go(mem, call, c::FLUSH_CACHE, &[0]);
        let vibrate = mem.rebase(g::COMMON_VIBRATE);
        entry(mem, call, cd, 0, &[8, vibrate, 0, 0]);
        entry(mem, call, cd, 8, &[0]);
        go(mem, call, c::FLUSH_CACHE, &[0]);
        go(mem, call, c::ABM_LOAD, &[]);
        // TkMenuLoadIcon (0x008a9c90) is a bare `ret`
        let lang = go(mem, call, c::GET_LANG, &[]);
        let at = mem.rebase(g::LANG);
        #[allow(clippy::cast_possible_truncation)]
        mem.write(at, lang as u8);
        lang
    }

    /// `TkMenuStart(mode)` (`0x008ab030`): with the menu open, one frame
    /// (`TkMenuMainLoop_008a9ca0(mode)`), and its `eax` while it stays
    /// open. Closed: under a pause (or with mode bit 24, after the started
    /// byte), `Pause_Request(mode & 0xffff)`. Else the menu starts on
    /// `mode & 0x3fffffff`: the Mn2 work, the 5 MB work area cleared, the
    /// shade ratios 0, the menu task (`sa_task_init2(2, ...)`, asserting
    /// the task buffer fits 0x8000), the player and summon lists, the
    /// timer and frame counter from the draw buffer's parity
    /// (`Sg_GetDBuffDC` into `local`), anti-aliasing off, the important
    /// list's size, the menu mode and the first frame. Each frame that
    /// closes the menu turns anti-aliasing back on (2, 1, 0x80) and ends
    /// the task (its `+4` entry, whose `eax` is returned; 0 without one).
    pub fn tk_menu_start<M, C>(mem: &mut M, mode: u32, local: u32, call: &mut C) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let gl = |mem: &M, at: u32| mem.read::<u32>(mem.rebase(at));
        let put = |mem: &mut M, at: u32, v: u32| {
            let a = mem.rebase(at);
            mem.write(a, v);
        };
        let put8 = |mem: &mut M, at: u32, v: u8| {
            let a = mem.rebase(at);
            mem.write(a, v);
        };
        if gl(mem, g::MENU_OPEN) != 0 {
            let r = go(mem, call, c::MAIN_LOOP, &[mode]);
            if gl(mem, g::MENU_OPEN) != 0 {
                return r;
            }
        } else {
            if gl(mem, g::PAUSE_ON) != 0 {
                return go(mem, call, c::PAUSE_REQUEST, &[mode & 0xffff]);
            }
            let mode = mode & 0x3fff_ffff;
            put8(mem, g::STARTED, 1);
            if mode & 0x0100_0000 != 0 {
                return go(mem, call, c::PAUSE_REQUEST, &[mode & 0xffff]);
            }
            go(mem, call, c::MN2_INIT_WORK, &[]);
            let work = mem.rebase(g::MENU_WORK);
            for k in (0..g::MENU_WORK_SIZE).step_by(ZEROS.len()) {
                mem.write_bytes(work + k, &ZEROS);
            }
            go(mem, call, c::MN2_GET_MAIN_POS, &[]);
            go(mem, call, c::SHADE_RATIO_DIFF, &[0]);
            go(mem, call, c::SET_SHADE_RATIO_STEP, &[0]);
            put(mem, g::MAIN_TASK, 0);
            if go(mem, call, c::TASK_BUFF_SIZE_GET, &[2]).cast_signed() >= 0x8000 {
                let (text, file) = (mem.rebase(g::TASK_OVERFLOW), mem.rebase(g::EMPTY));
                go(mem, call, c::ASSERT, &[1, text, file]);
            }
            let buffer = mem.rebase(g::TASK_BUFFER);
            let task = go(mem, call, c::TASK_INIT2, &[2, buffer, 0]);
            put(mem, g::MAIN_TASK, task);
            go(mem, call, c::PLAYER_LIST_INIT, &[mode]);
            go(mem, call, c::SUMMON_LIST_INIT, &[mode]);
            go(mem, call, c::GET_DBUFF_DC, &[local]);
            let parity = !mem.read::<u32>(local) & 1;
            #[allow(clippy::cast_possible_truncation)]
            put8(mem, g::TIMER, parity as u8);
            put(mem, g::FRAMES, parity);
            put8(mem, g::FIRST_READ_END, 0);
            go(mem, call, c::SET_TK_ANTI_XYB, &[0, 0, 0x80]);
            go(mem, call, c::ENABLE_TK_ANTI, &[0]);
            put(mem, g::START_WORD, 0x83a0);
            go(mem, call, c::CALC_IMPORTANT_LIST_MAX, &[]);
            go(mem, call, c::SET_IGNORE_JOIN_FLAG, &[0]);
            put(mem, g::MENU_MODE, mode);
            go(mem, call, c::MAIN_LOOP, &[mode]);
        }
        go(mem, call, c::SET_TK_ANTI_XYB, &[2, 1, 0x80]);
        go(mem, call, c::ENABLE_TK_ANTI, &[1]);
        let task = gl(mem, g::MAIN_TASK);
        let r = if task == 0 {
            0
        } else {
            let end = mem.read::<u32>(task.wrapping_add(4));
            call(mem, end, &[])
        };
        put(mem, g::MAIN_TASK, 0);
        r
    }

    /// A callee by its image address.
    fn go<M, C>(mem: &mut M, call: &mut C, addr: u32, args: &[u32]) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let a = mem.rebase(addr);
        call(mem, a, args)
    }

    /// The CD interface's entry at `cd + o` (a pointer the game holds).
    fn entry<M, C>(mem: &mut M, call: &mut C, cd: u32, o: u32, args: &[u32]) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let f = mem.read::<u32>(cd.wrapping_add(o));
        call(mem, f, args)
    }

    /// [`g::GS_DEF_DRAW_ENV2`] is 0.
    fn disc_info(mem: &impl Mem) -> bool {
        mem.read::<u8>(mem.rebase(g::GS_DEF_DRAW_ENV2)) == 0
    }

    /// File `file` of directory `dir` read into a new buffer (its size
    /// rounded up to sectors, 64-byte aligned, first word 0).
    fn read_new<M, C>(mem: &mut M, call: &mut C, dir: u32, file: u32) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let cd = go(mem, call, c::GET_CD, &[]);
        entry(mem, call, cd, 0x10, &[dir]);
        let size = entry(mem, call, cd, 0x14, &[file]);
        go(mem, call, c::MEM_SET_ALIGN, &[0x40]);
        let buffer = go(mem, call, c::USER_MALLOC, &[sectors(size)]);
        mem.write(buffer, 0u32);
        go(mem, call, c::FLUSH_CACHE, &[0]);
        entry(mem, call, cd, 0, &[file, buffer, 0, 0]);
        entry(mem, call, cd, 8, &[0]);
        buffer
    }

    /// `user_free(p)` and the two `Sg_MemCheck`s.
    fn free<M, C>(mem: &mut M, call: &mut C, p: u32)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let tklib = mem.rebase(g::TKLIB_C);
        go(mem, call, c::USER_FREE, &[p]);
        go(mem, call, c::MEM_CHECK, &[0, tklib, 0x1e0]);
        go(mem, call, c::MEM_CHECK, &[1, tklib, 0x1e1]);
    }

    /// The wave (`dir` 0x2c) or music (0x2d) info from the disc: file 1
    /// read, `init(buffer, size, position)`, freed.
    fn disc<M, C>(mem: &mut M, call: &mut C, dir: u32, init: u32)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let buffer = read_new(mem, call, dir, 1);
        let cd = go(mem, call, c::GET_CD, &[]);
        entry(mem, call, cd, 0x10, &[dir]);
        let size = entry(mem, call, cd, 0x14, &[1]);
        let cd = go(mem, call, c::GET_CD, &[]);
        entry(mem, call, cd, 0x10, &[dir]);
        let at = entry(mem, call, cd, 4, &[0]);
        go(mem, call, init, &[buffer, size, at]);
        free(mem, call, buffer);
    }

    /// The maps `TkInitScene` never autosaves on (besides 0x17 and the
    /// Eternal Calm scenes).
    pub const NO_AUTOSAVE_MAPS: [u32; 12] = [
        0x11a, 0xc2, 0x15c, 0x15d, 0x170, 0xff, 0xd1, 0xd4, 0x10e, 0x140, 0x77, 0xf7,
    ];

    /// `TkInitScene(map)` (`0x0088e070`; callers `Sg_MainInit`,
    /// `TkUpdateEventJump`): a new map's start. `graphicSetIsBattleMap(0)`,
    /// "TK:Init Scene:%d" (`printf` through its import), `MsScreenResetOffset
    /// (0.0, 0.0)`; the byte `0x0112d70f` 8 clears the achievement latch
    /// `0x00c58e8a`, 9 with the latch clear sets it and unlocks
    /// achievement 0x14. An Eternal Calm scene sets `0x018663d0` (when
    /// clear) and zeroes `0x018663d4`. The CD interface's wait (`GetCD()`
    /// `+8`, `(0)`), `fiosClearOps`, `SaveDataSetEventId(map)`,
    /// `g_GraphicMapId` = map, `MsCameraInit`, `TOAllCloseMessageWindow`,
    /// `TOInitSceneMessageWindow`, and when `0x01330858` is set another
    /// `TOInitSceneMessageWindow`, `AtelEventRemove` and the word cleared.
    /// `MagicFile_clearAllPreloadedPrx(-1)`, `graphicVFXDestroyAll`,
    /// `destroyMoviePicture`, `graphicResetYIQ`, `AtelEventSetUp(map)`,
    /// `gNeedAutoSave` 0. Map 0x17 (game over): `0x00c58e40` = 1,
    /// `0x01330840` = 0, `Sg_EventjumpDispose`, "Dispose all the chr cache
    /// elements." to `__iob_func() + 0x20` (`fprintf`),
    /// `graphicSetGameOverNotify(0)`. Any other map outside
    /// [`NO_AUTOSAVE_MAPS`] and not Eternal Calm autosaves, unless
    /// `0x00c58e40` is set and the counter `0x01330840` (+1) isn't 2 yet
    /// (at 2 the flag clears): `gNeedAutoSave` = 1, the save event record
    /// (`MsGetSaveEventAddress`) gets `u16 +0xb8` = `u8 +0xc`, `u16 +0xba`
    /// = `u16 +0`, `u16 +0xa8` = `SndGetMusicVolume(m)`'s low byte << 8 |
    /// `m` (`m` = `AtelGetMusicNumber()`), `u16 +0xaa` =
    /// `graphicGetTextureVideoInfo() + 1`; `TkBackupEventJump(record)`,
    /// `TkMenuPreSave`, `SaveDataPreDebugSave(0)`. Then
    /// `g_TkEventJumpRequest` 0, `0x01330858` 1, both `g_TkFadeOut` words
    /// 0, `g_TkEncountCounterEventStart` = `MsGetSaveBattleCount()`,
    /// `iSoundStageFlush`, `iVoiceDataChange` and `iSyncDataChange(map)`,
    /// whose `EAX` it returns. `call` gets absolute addresses.
    #[allow(clippy::too_many_lines)] // the start in turn
    pub fn tk_init_scene<M, C>(mem: &mut M, map: u32, call: &mut C) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let r = |mem: &M, a: u32| mem.rebase(a);
        let import = |mem: &M, slot: u32| mem.read::<u32>(mem.rebase(slot));
        go(mem, call, 0x0064_3490, &[0]);
        let printf = import(mem, 0x00b0_c380);
        let fmt = r(mem, 0x00b5_e984);
        call(mem, printf, &[fmt, map]);
        go(mem, call, 0x007b_c720, &[0, 0]);
        let latch = r(mem, 0x00c5_8e8a);
        match mem.read::<u8>(r(mem, 0x0112_d70f)) {
            8 => mem.write(latch, 0_u8),
            9 if mem.read::<u8>(latch) == 0 => {
                mem.write(latch, 1_u8);
                go(mem, call, 0x0082_2380, &[0x14]);
            }
            _ => {}
        }
        let busy = r(mem, 0x0186_63d0);
        if go(mem, call, 0x0067_ab50, &[map]) != 0 && mem.read::<u8>(busy) == 0 {
            mem.write(busy, 1_u8);
            mem.write(r(mem, 0x0186_63d4), 0_u32);
        }
        let cd = go(mem, call, c::GET_CD, &[]);
        entry(mem, call, cd, 8, &[0]);
        go(mem, call, 0x0067_9500, &[]);
        go(mem, call, 0x0064_8630, &[map]);
        mem.write(r(mem, 0x0133_0844), map);
        go(mem, call, 0x007b_d5d0, &[]);
        go(mem, call, 0x008a_b7b0, &[]);
        go(mem, call, 0x008a_dbf0, &[]);
        let event = r(mem, 0x0133_0858);
        if mem.read::<u32>(event) != 0 {
            go(mem, call, 0x008a_dbf0, &[]);
            go(mem, call, 0x0087_2d20, &[]);
            mem.write(event, 0_u32);
        }
        go(mem, call, 0x009d_aca0, &[u32::MAX]);
        go(mem, call, 0x0064_5310, &[]);
        go(mem, call, 0x0076_ece0, &[]);
        go(mem, call, 0x0064_2cc0, &[]);
        go(mem, call, 0x0087_2ef0, &[map]);
        let need = r(mem, 0x023c_bebc);
        mem.write(need, 0_u32);
        let over = r(mem, 0x00c5_8e40);
        let count = r(mem, 0x0133_0840);
        if map == 0x17 {
            mem.write(over, 1_u32);
            mem.write(count, 0_u32);
            go(mem, call, 0x0083_b420, &[]);
            let msg = r(mem, 0x00b5_e998);
            let iob = import(mem, 0x00b0_c350);
            let stream = call(mem, iob, &[]).wrapping_add(0x20);
            let fprintf = import(mem, 0x00b0_c38c);
            call(mem, fprintf, &[stream, msg]);
            go(mem, call, 0x0064_3260, &[0]);
        } else if !NO_AUTOSAVE_MAPS.contains(&map) && go(mem, call, 0x0067_ab50, &[map]) == 0 {
            let mut save = true;
            if mem.read::<u32>(over) != 0 {
                let n = mem.read::<u32>(count).wrapping_add(1);
                mem.write(count, n);
                save = n == 2;
                if save {
                    mem.write(over, 0_u32);
                }
            }
            if save {
                mem.write(need, 1_u32);
                let rec = go(mem, call, 0x0078_5240, &[]);
                let b = mem.read::<u8>(rec.wrapping_add(0xc));
                mem.write(rec.wrapping_add(0xb8), u16::from(b));
                let h = mem.read::<u16>(rec);
                mem.write(rec.wrapping_add(0xba), h);
                let music = go(mem, call, 0x0087_3b00, &[]);
                let volume = go(mem, call, 0x0088_63d0, &[music]);
                #[allow(clippy::cast_possible_truncation)] // AL and SI's low word
                let packed = (u16::from(volume as u8) << 8) | music as u16;
                mem.write(rec.wrapping_add(0xa8), packed);
                #[allow(clippy::cast_possible_truncation)] // AX
                let video = (go(mem, call, 0x0064_1400, &[]) as u16).wrapping_add(1);
                mem.write(rec.wrapping_add(0xaa), video);
                go(mem, call, 0x0088_e800, &[rec]);
                go(mem, call, 0x008b_50b0, &[]);
                go(mem, call, 0x0064_84d0, &[0]);
            }
        }
        mem.write(r(mem, 0x0133_084c), 0_u32);
        mem.write(event, 1_u32);
        mem.write(r(mem, 0x0133_0850), 0_u32);
        mem.write(r(mem, 0x0133_0854), 0_u32);
        let battles = go(mem, call, 0x0078_4f50, &[]);
        mem.write(r(mem, 0x0133_086c), battles);
        go(mem, call, 0x0067_a5d0, &[map]);
        go(mem, call, 0x0067_a6a0, &[map]);
        go(mem, call, 0x0067_a8e0, &[map])
    }

    /// `TkUpdateEventJump()` (`0x0088d3d0`, every frame of `Sg_MainLoop`):
    /// with [`g::EVENT_JUMP_REQUEST`] set, the record
    /// `MsGetSaveEventAddress` gives and both [`g::FADE_OUT`] countdowns
    /// (signed) at 0 or below, the record's map (`u16 +0`) is entered:
    /// `graphicActivateLoadingScreen` first unless the map is 0x17, 0x15c,
    /// or 0x10c with `getScenerioFlag()`'s `ax` 0x1a9, then
    /// `TkInitScene(map)` (the map read again). A positive first countdown
    /// drops by 1, else a positive second one becomes 0. Then
    /// `Atel_EventFrame`, `AtelCheckMemory` with [`g::INIT_FLAG`] 0 around
    /// it (1 after), `AtelMove(1)`, and the tail call `TkSndUpdate2`, whose
    /// `EAX` it returns. `call` gets absolute addresses.
    pub fn tk_update_event_jump<M, C>(mem: &mut M, call: &mut C) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let request = mem.rebase(g::EVENT_JUMP_REQUEST);
        if mem.read::<u32>(request) != 0 {
            let record = go(mem, call, c::MS_GET_SAVE_EVENT_ADDRESS, &[]);
            let fade = mem.rebase(g::FADE_OUT);
            let first = mem.read::<i32>(fade);
            if first > 0 {
                mem.write(fade, first - 1);
            } else if mem.read::<i32>(fade + 4) > 0 {
                mem.write(fade + 4, 0u32);
            } else {
                let map = mem.read::<u16>(record);
                let plain = map == 0x17
                    || map == 0x15c
                    || (map == 0x10c && lo16(go(mem, call, c::GET_SCENERIO_FLAG, &[])) == 0x1a9);
                if !plain {
                    go(mem, call, c::ACTIVATE_LOADING_SCREEN, &[]);
                }
                let map = mem.read::<u16>(record);
                go(mem, call, c::INIT_SCENE, &[u32::from(map)]);
            }
        }
        go(mem, call, c::ATEL_EVENT_FRAME, &[]);
        let done = mem.rebase(g::INIT_FLAG);
        mem.write(done, 0u32);
        go(mem, call, c::ATEL_CHECK_MEMORY, &[]);
        mem.write(done, 1u32);
        go(mem, call, c::ATEL_MOVE, &[1]);
        go(mem, call, c::SND_UPDATE2, &[])
    }

    /// The low 16 bits of `v` (`ax`).
    fn lo16(v: u32) -> u16 {
        u16::try_from(v & 0xffff).unwrap_or(0)
    }
}
