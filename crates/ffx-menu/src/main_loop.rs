//! `TkMenuMainLoop_008a9ca0` (name ours): one frame of the main menu, from
//! `TkMenuStart` (`0x008ab030`) while `g_MovieFading` (here: the menu is
//! open) is set. The first frame opens the menu, each frame runs and draws
//! it, the frame `MenuMain_BusyMask` reports idle closes it.

/// The callees, by address.
pub mod callee {
    pub const SAVE_VRAM: u32 = 0x008a_ab90;
    pub const SAVE_FRAME_BUFFER: u32 = 0x008a_ac80;
    pub const SET_FRAME_RATIO_DIFF: u32 = 0x008a_b260;
    pub const INIT_MENU_GRAPH: u32 = 0x008b_e330;
    pub const SEQ_INIT_SCENE: u32 = 0x008a_eec0;
    pub const ENTER_MENU_CALLBACK: u32 = 0x0086_55e0;
    pub const MAIN_RESET: u32 = 0x008a_a570;
    pub const OPEN_MODE: u32 = 0x008a_a780;
    pub const INIT_BRUSH: u32 = 0x0090_0f40;
    pub const BUSY_MASK: u32 = 0x008a_a610;
    pub const MOVIE_HAVE_CAMERA: u32 = 0x0076_efc0;
    pub const VIDEO_PAUSE: u32 = 0x0064_5de0;
    pub const SET_FLIP_VSYNC: u32 = 0x0064_30e0;
    pub const MENU_RESET: u32 = 0x0064_2320;
    pub const SYNC_V: u32 = 0x0088_7f00;
    pub const EXEC_VIBRATE: u32 = 0x0087_39a0;
    pub const IGGY_MENU_UPDATE: u32 = 0x00a5_6050;
    pub const GET_DBUFF_DC: u32 = 0x0082_0530;
    pub const GET_PAD_STATUS: u32 = 0x008b_e500;
    pub const FILE_READ_MAIN: u32 = 0x0076_cfd0;
    pub const EXEC_MENU_CTRL: u32 = 0x0087_3600;
    pub const CLOSE_ALL: u32 = 0x008a_a200;
    pub const CONTROL_MESSAGE_WINDOW: u32 = 0x008a_b9a0;
    /// `jmp TkSndUpdate`.
    pub const SND_UPDATE: u32 = 0x008c_a470;
    pub const TRANS_TEXTURE: u32 = 0x008b_1190;
    pub const ABMAP_RESET_SEGMENTS: u32 = 0x0063_90c0;
    pub const GET_ABMAP: u32 = 0x0064_1060;
    pub const DRAW_BLACK_BACKGROUND: u32 = 0x008d_b870;
    pub const DRAW_ALL: u32 = 0x008a_a290;
    pub const VU1_SYNC_PATH: u32 = 0x0088_ec60;
    pub const OPEN_PKT_BUFF_TMP: u32 = 0x008b_ef30;
    pub const DRAW_SCISSOR_XYWH: u32 = 0x008f_9230;
    pub const GET_DRAW_SHADE_RATIO: u32 = 0x008a_98a0;
    pub const DRAW_BOX_XYWH_C2: u32 = 0x008f_3db0;
    pub const DRAW_KICK_TMP: u32 = 0x008b_ea20;
    pub const DRAW_MESSAGE_WINDOW: u32 = 0x008a_be40;
    pub const DAPS_DRAW: u32 = 0x0089_dc10;
    pub const ABMAP_UPDATE: u32 = 0x0063_90f0;
    pub const DRAW_KICK: u32 = 0x008d_ce50;
    pub const GS_SYNC_PATH: u32 = 0x0062_f880;
    pub const FRAME_FADE: u32 = 0x008a_9510;
    pub const MENU_UPDATE: u32 = 0x0064_23b0;
    pub const REMOVE_ALL_MENUS: u32 = 0x008a_add0;
    pub const ATEL_RELEASE: u32 = 0x008b_e350;
    pub const RECOVER_FROM_MENU: u32 = 0x0086_e700;
    pub const RESTORE_VRAM: u32 = 0x008a_a690;
    pub const DRAW_INIT_STRING: u32 = 0x008d_ce30;
    pub const DRAW_ENV: u32 = 0x008d_cca0;
}

/// The globals, by address.
pub mod global {
    /// Set for modes `0x800001`/`0x800002`: a black background instead of
    /// the menu's own (unless `graphicGetAbmap() == 1`).
    pub const BLACK_BACK: u32 = 0x0134_0820;
    /// `g_MovieFading`: the menu is open.
    pub const OPEN: u32 = 0x0134_0824;
    /// Set for mode `0x800003`.
    pub const MODE_800003: u32 = 0x0134_0854;
    /// `u8`: the Iggy (PC) menu runs: `TkSyncV(2)`, a second vibration
    /// step, `IggyMenu_update`, the message window every frame.
    pub const IGGY: u32 = 0x0134_0858;
    /// Three words cleared each frame.
    pub const CLEARED: u32 = 0x025d_16d4;
    /// `g_MoviePictureOn`.
    pub const MOVIE_PICTURE: u32 = 0x0112_a00c;
    /// `g_GraphicSkipFrame`.
    pub const SKIP_FRAME: u32 = 0x012f_b7d0;
    /// `u8 g_TkMenuTimer`.
    pub const TIMER: u32 = 0x0184_0884;
    /// A frame counter.
    pub const FRAMES: u32 = 0x0184_1c60;
    /// `g_TkMenuDrawSkipMask`.
    pub const DRAW_SKIP_MASK: u32 = 0x0184_08f0;
    /// `g_TkMenuDrawShadeRatio` and its step (`+4`).
    pub const SHADE_RATIO: u32 = 0x0184_087c;
    /// `g_TkMenuMainActiveMask` (bit 1: the shade box).
    pub const ACTIVE_MASK: u32 = 0x0184_08ec;
    /// `g_TkMn2MainFrameRatioDiff`.
    pub const FRAME_RATIO_DIFF: u32 = 0x0184_1c64;
    /// `g_TkMn2MainFrameRatio`.
    pub const FRAME_RATIO: u32 = 0x0184_1c68;
}

/// [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{callee as c, global as g};

    /// `ratio += step`, then 0 when the sum's sign bit is set (`jns`),
    /// at most 0x80.
    fn step_ratio<M: Mem>(mem: &mut M, ratio: u32, step: u32) -> u32 {
        let at = mem.rebase(ratio);
        let v = mem
            .read::<u32>(at)
            .wrapping_add(mem.read::<u32>(mem.rebase(step)));
        let v = if v.cast_signed() < 0 {
            0
        } else if v.cast_signed() > 0x80 {
            0x80
        } else {
            v
        };
        mem.write(at, v);
        v
    }

    /// `TkMenuMainLoop_008a9ca0(mode)` (`0x008a9ca0`): mode
    /// `0x800001`/`0x800002` sets [`g::BLACK_BACK`], `0x800003` sets
    /// [`g::MODE_800003`] (each clears the other; -1 keeps both). When
    /// the menu is not open yet ([`g::OPEN`]): the VRAM and frame buffer
    /// saved (unless the mode's high half is `0x10000`), the frame ratio
    /// 0x80 with no step, `TOInitMenuGraph`, `TOSeqInitScene(1)`,
    /// `AtelEnterMenuCallback(mode)`, `MenuMain_Reset` and
    /// `TkMenuMainOpenMode` (high half, low half), `TOMenuInitBrush`;
    /// idle (`MenuMain_BusyMask` 0) closes at once, else it opens
    /// (movie paused when one plays, flip on vsync 1). Then the frame:
    /// timers, pad, reads, script menu control, closes, the shade and
    /// frame ratios, the sound, the draws and the frame fade
    /// (`0x008a9510`); idle closes (vsync 2). Closing: the menus removed,
    /// the script recovered, the VRAM restored and two frames drawn
    /// (unless `0x10000`). `Sg_GetDBuffDC` writes the argument slot
    /// (`slot`). Returns the last callee's `eax` while open
    /// (`MenuMain_BusyMask`'s), else the last `TkSyncV(0)`'s.
    ///
    /// The original also calls two empty functions (`0x008ab250`, its
    /// `(dc, timer)` draw, and `0x008ca480`): both a bare `ret`.
    #[allow(clippy::too_many_lines)] // the original's frame in turn
    pub fn tk_menu_main_loop<M, C>(mem: &mut M, mode: u32, slot: u32, call: &mut C) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let mut f = |mem: &mut M, addr: u32, args: &[u32]| {
            let a = mem.rebase(addr);
            call(mem, a, args)
        };
        let gl = |mem: &M, at: u32| mem.read::<u32>(mem.rebase(at));
        let put = |mem: &mut M, at: u32, v: u32| {
            let a = mem.rebase(at);
            mem.write(a, v);
        };
        match mode {
            0x0080_0001 | 0x0080_0002 => {
                put(mem, g::BLACK_BACK, 1);
                put(mem, g::MODE_800003, 0);
            }
            0x0080_0003 => {
                put(mem, g::BLACK_BACK, 0);
                put(mem, g::MODE_800003, 1);
            }
            u32::MAX => {}
            _ => {
                put(mem, g::BLACK_BACK, 0);
                put(mem, g::MODE_800003, 0);
            }
        }
        let (high, low) = (mode & 0xffff_0000, mode & 0xffff);
        for k in [8, 4, 0] {
            put(mem, g::CLEARED + k, 0);
        }
        let mut open = gl(mem, g::OPEN) != 0;
        if !open {
            if high != 0x10000 {
                f(mem, c::SAVE_VRAM, &[]);
                f(mem, c::SAVE_FRAME_BUFFER, &[]);
            }
            put(mem, g::FRAME_RATIO, 0x80);
            f(mem, c::SET_FRAME_RATIO_DIFF, &[0]);
            f(mem, c::INIT_MENU_GRAPH, &[]);
            f(mem, c::SEQ_INIT_SCENE, &[1]);
            f(mem, c::ENTER_MENU_CALLBACK, &[mode]);
            f(mem, c::MAIN_RESET, &[high, low]);
            f(mem, c::OPEN_MODE, &[high, low]);
            f(mem, c::INIT_BRUSH, &[]);
            if f(mem, c::BUSY_MASK, &[]) != 0 {
                put(mem, g::OPEN, 1);
                if f(mem, c::MOVIE_HAVE_CAMERA, &[]) != 0 || gl(mem, g::MOVIE_PICTURE) != 0 {
                    f(mem, c::VIDEO_PAUSE, &[]);
                }
                f(mem, c::SET_FLIP_VSYNC, &[1]);
                open = true;
            }
        }
        if open {
            if gl(mem, g::SKIP_FRAME) == 0 {
                f(mem, c::MENU_RESET, &[]);
            }
            let timer_at = mem.rebase(g::TIMER);
            let timer = mem.read::<u8>(timer_at).wrapping_add(1);
            mem.write(timer_at, timer);
            let frames = gl(mem, g::FRAMES).wrapping_add(1);
            put(mem, g::FRAMES, frames);
            let iggy = mem.read::<u8>(mem.rebase(g::IGGY)) != 0;
            put(mem, g::DRAW_SKIP_MASK, 0);
            f(mem, c::SYNC_V, &[if iggy { 2 } else { 0 }]);
            // the timer as it is after TkSyncV
            let timer = mem.read::<u8>(timer_at);
            f(mem, c::EXEC_VIBRATE, &[u32::from(timer)]);
            if iggy {
                f(mem, c::EXEC_VIBRATE, &[1]);
                f(mem, c::IGGY_MENU_UPDATE, &[]);
            }
            f(mem, c::GET_DBUFF_DC, &[slot]);
            f(mem, c::GET_PAD_STATUS, &[]);
            f(mem, c::FILE_READ_MAIN, &[]);
            f(mem, c::EXEC_MENU_CTRL, &[]);
            f(mem, c::CLOSE_ALL, &[]);
            step_ratio(mem, g::SHADE_RATIO, g::SHADE_RATIO + 4);
            if mem.read::<u8>(mem.rebase(g::IGGY)) != 0
                || mem.read::<u8>(mem.rebase(g::TIMER)) & 1 != 0
            {
                f(mem, c::CONTROL_MESSAGE_WINDOW, &[]);
            }
            f(mem, c::SND_UPDATE, &[]);
            f(mem, c::TRANS_TEXTURE, &[]);
            if gl(mem, g::SKIP_FRAME) == 0 {
                f(mem, c::ABMAP_RESET_SEGMENTS, &[]);
            }
            if gl(mem, g::BLACK_BACK) == 0 || f(mem, c::GET_ABMAP, &[]) == 1 {
                f(mem, c::DRAW_ALL, &[]);
            } else {
                f(mem, c::DRAW_BLACK_BACKGROUND, &[]);
            }
            if mem.read::<u8>(mem.rebase(g::ACTIVE_MASK)) & 2 != 0 {
                f(mem, c::VU1_SYNC_PATH, &[]);
                f(mem, c::OPEN_PKT_BUFF_TMP, &[]);
                f(mem, c::DRAW_SCISSOR_XYWH, &[0, 0, 0x200, 0x1a0]);
                if f(mem, c::GET_DRAW_SHADE_RATIO, &[]) != 0 {
                    let colour = gl(mem, g::SHADE_RATIO) << 24;
                    f(
                        mem,
                        c::DRAW_BOX_XYWH_C2,
                        &[0, 0, 0x200, 0x1a0, colour, colour],
                    );
                }
                f(mem, c::DRAW_KICK_TMP, &[]);
                f(mem, c::VU1_SYNC_PATH, &[]);
            }
            f(mem, c::DRAW_MESSAGE_WINDOW, &[]);
            f(mem, c::DAPS_DRAW, &[]);
            if gl(mem, g::SKIP_FRAME) == 0 {
                f(mem, c::ABMAP_UPDATE, &[]);
            }
            f(mem, c::DRAW_KICK, &[]);
            f(mem, c::DRAW_KICK, &[]);
            let ratio = gl(mem, g::FRAME_RATIO);
            if ratio != 0 && high != 0x10000 {
                f(mem, c::GS_SYNC_PATH, &[0, 0]);
                let ratio = gl(mem, g::FRAME_RATIO);
                f(mem, c::FRAME_FADE, &[ratio, 0x80, 0, 0]);
                f(mem, c::GS_SYNC_PATH, &[0, 0]);
            }
            step_ratio(mem, g::FRAME_RATIO, g::FRAME_RATIO_DIFF);
            if gl(mem, g::SKIP_FRAME) == 0 {
                f(mem, c::MENU_UPDATE, &[]);
            }
            let mut last = f(mem, c::BUSY_MASK, &[]);
            if last == 0 {
                put(mem, g::OPEN, 0);
                if f(mem, c::MOVIE_HAVE_CAMERA, &[]) != 0 || gl(mem, g::MOVIE_PICTURE) != 0 {
                    f(mem, c::VIDEO_PAUSE, &[]);
                }
                last = f(mem, c::SET_FLIP_VSYNC, &[2]);
            }
            if gl(mem, g::OPEN) != 0 {
                return last;
            }
        }
        // closing
        f(mem, c::DRAW_KICK, &[]);
        f(mem, c::REMOVE_ALL_MENUS, &[]);
        f(mem, c::ATEL_RELEASE, &[]);
        f(mem, c::RECOVER_FROM_MENU, &[]);
        f(mem, c::DRAW_KICK, &[]);
        f(mem, c::GS_SYNC_PATH, &[0, 0]);
        if high != 0x10000 {
            f(mem, c::GS_SYNC_PATH, &[0, 0]);
            f(mem, c::RESTORE_VRAM, &[]);
            f(mem, c::FRAME_FADE, &[0x80, 0x80, 0, 0]);
            f(mem, c::GET_DBUFF_DC, &[slot]);
            for _ in 0..2 {
                f(mem, c::SYNC_V, &[0]);
                f(mem, c::DRAW_INIT_STRING, &[]);
                f(mem, c::DRAW_ENV, &[]);
                f(mem, c::DRAW_KICK, &[]);
                f(mem, c::VU1_SYNC_PATH, &[]);
                f(mem, c::FRAME_FADE, &[0x80, 0x80, 0, 0]);
            }
        }
        f(mem, c::DRAW_KICK, &[]);
        f(mem, c::GS_SYNC_PATH, &[0, 0]);
        f(mem, c::SYNC_V, &[0])
    }
}
