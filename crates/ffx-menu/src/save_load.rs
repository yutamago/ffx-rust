//! The save/load menu's control, `TkMenuCtrlSaveLoad` (`0x008b15d0`, the
//! save and the load menu slots' ctrl): a state machine on the slot's
//! `+0x1c` (0..0x71, jump table `0x008b3310`; larger states do nothing).
//!
//! The PS2 memory card steps (`TkMc*`, `MenuSave_McSync`) are kept as the
//! original has them; on the PC they finish at once. The PC's own save and
//! load are the `debugSave`/`debugLoad` thunks (`0x006492e0`/`0x006492b0`)
//! of state 0x25: the port calls them through the game, as the original
//! does.
//!
//! The original's frame holds 7 path buffers and one out word per
//! `MenuSave_McSync` call, each dead after its state; the port uses one
//! of each ([`FRAME_WORDS`]). The stack cookie check is left out.

/// Words of the port's frame: a 0x80-byte path buffer, then the out word.
pub const FRAME_WORDS: usize = 0x21;

/// The callees, by address.
pub mod callee {
    pub const GET_CONTROL_PAD: u32 = 0x008b_e410;
    pub const GET_CONTROL_PAD_REP: u32 = 0x008b_e470;
    pub const GET_CONTROL_PAD_TRG: u32 = 0x008b_e4b0;
    pub const GET_CD: u32 = 0x0076_d0d0;
    pub const POLL_CARD: u32 = 0x008b_3d20;
    pub const SND_SEP_PLAY_SIMPLE: u32 = 0x0088_6e80;
    pub const CLOSE_HELP_WINDOW: u32 = 0x008b_3790;
    pub const GET_LIST_MAX2: u32 = 0x008e_3390;
    pub const CLOSE_YES_NO_WINDOW: u32 = 0x008e_2de0;
    pub const MC_INIT_DIR: u32 = 0x0088_8030;
    pub const MC_GET_DIR: u32 = 0x0088_8200;
    pub const MC_GET_INFO: u32 = 0x0088_82c0;
    pub const MC_GET_LOAD_FILE_POS: u32 = 0x0088_8300;
    pub const SYNC_TOGGLE: u32 = 0x008b_4da0;
    pub const MC_SYNC: u32 = 0x0088_84a0;
    pub const PATH_DIR: u32 = 0x008b_3ff0;
    pub const MC_MKDIR: u32 = 0x0088_8330;
    pub const PATH_ICON: u32 = 0x008b_4110;
    pub const MC_OPEN: u32 = 0x0088_8360;
    pub const MC_WRITE: u32 = 0x0088_8570;
    pub const MC_CLOSE: u32 = 0x0088_8010;
    pub const PATH_ICON_SYS: u32 = 0x008b_4230;
    pub const GET_SAVE_DIC: u32 = 0x0086_c410;
    /// `jmp MsGetSaveEventAddress`.
    pub const GET_SAVE_EVENT_ADDRESS: u32 = 0x0088_c830;
    pub const PACK_SAVE: u32 = 0x008b_3e60;
    pub const BUILD_ICON_SYS: u32 = 0x008b_42e0;
    pub const PATH_DATA: u32 = 0x008b_4060;
    pub const SAVE_MES: u32 = 0x008d_e080;
    pub const SET_HELP_MESSAGE: u32 = 0x008a_aeb0;
    pub const SUB_CMD_OPEN_C: u32 = 0x008e_3690;
    pub const MC_READ: u32 = 0x0088_83a0;
    pub const MC_SET_EXEC_PORT: u32 = 0x0088_8430;
    pub const OPEN_SUB_CMD_WINDOW: u32 = 0x008e_33a0;
    pub const CLOSE_WINDOW: u32 = 0x008b_3e40;
    pub const SET_HELP_RATIO_DIFF2: u32 = 0x008a_aee0;
    pub const STOP_SCENE: u32 = 0x008a_f4f0;
    pub const RESET_TOGGLES: u32 = 0x008b_4eb0;
    pub const OPEN_WINDOW_3C20: u32 = 0x008b_3c20;
    pub const OPEN_WINDOW2: u32 = 0x008b_4fb0;
    pub const OPEN_CHOICE: u32 = 0x008b_3c70;
    pub const SUB_CMD_OPEN_A: u32 = 0x008e_3650;
    pub const SAVE_LOAD_RESET: u32 = 0x008b_4e70;
    pub const FIRST_READ_END: u32 = 0x008a_8e50;
    pub const GET_GLOBAL_1841C0C: u32 = 0x008a_98e0;
    pub const OPEN_WINDOW: u32 = 0x008b_5000;
    pub const CLEAR_SELECT_WINDOW: u32 = 0x008a_a1e0;
    pub const DEBUG_SAVE: u32 = 0x0064_92e0;
    pub const DEBUG_LOAD: u32 = 0x0064_92b0;
    pub const SWITCH_GPU_MODE: u32 = 0x0064_3e70;
    pub const MC_SET_LOAD_FILE_POS: u32 = 0x0088_8480;
    pub const COPY_LOADED_SAVE: u32 = 0x008b_54a0;
    pub const REFRESH_SOUND_MODE: u32 = 0x0087_24f0;
    pub const REFRESH_HDD: u32 = 0x0087_24a0;
    /// `eiAbmParaGet` (no dump row).
    pub const ABM_PARA_GET: u32 = 0x008a_97e0;
    pub const MC_FORMAT: u32 = 0x0088_81c0;
    pub const MARK_SLOTS: u32 = 0x008b_54c0;
    pub const GET_SAVE_FILE_INFO: u32 = 0x008b_4e40;
    pub const CRC: u32 = 0x008b_1450;
    pub const HELP_SET_EXT_MODE: u32 = 0x008b_ef10;
    pub const GET_MENU_MODE: u32 = 0x008a_b240;
    pub const SET_MAIN_FRAME_RATIO_DIFF: u32 = 0x008a_b260;
    pub const GET_MAIN_FRAME_RATIO: u32 = 0x008a_b230;
    pub const LANG_SWAP_FLAG: u32 = 0x008a_c390;
    pub const SAVE_CONFIG_ENGLISH: u32 = 0x0078_51d0;
    pub const SWAP_ENV_INIT: u32 = 0x008b_0920;
    pub const SWAP_ENV_EXEC: u32 = 0x008b_0500;
    pub const CLOSE_WINDOW2: u32 = 0x008b_3d90;
    pub const MAIN_SLEEP_MENU: u32 = 0x008a_ad70;
    pub const MAIN_ACTIVE_MENU: u32 = 0x008a_a100;
}

/// The globals, by address.
pub mod global {
    /// `g_SaveFileBuffer` (0x6900 bytes).
    pub const SAVE_FILE: u32 = 0x0159_7f70;
    /// The save block the CRC covers (`g_SaveFileBuffer + 0x40`).
    pub const SAVE_BODY: u32 = 0x0159_7fb0;
    /// `u16` the CRC stored in the loaded save.
    pub const SAVE_CRC: u32 = 0x0159_e464;
    /// The icon.sys buffer (0x3c4 bytes).
    pub const ICON_SYS: u32 = 0x0159_e870;
    /// The icon file buffer (`START_WORD`'s count of bytes).
    pub const ICON: u32 = 0x015a_0870;
    /// `u32`: the language swap asked for after a load.
    pub const LANG_SWAP: u32 = 0x0186_6240;
    /// `g_TkMenuSaveLoadWindow`.
    pub const WINDOW: u32 = 0x0186_6254;
    /// `g_TkMenuSaveChoiceRow` (the card port).
    pub const ROW: u32 = 0x0186_6264;
    /// `u8[2] g_TkMenuSaveChoiceEnabled`.
    pub const ENABLED: u32 = 0x0186_6268;
    /// `u8 g_TkMenuSaveLoadToggles`.
    pub const TOGGLES: u32 = 0x0186_626a;
    /// The card type (`TkMcGetInfo`'s second out).
    pub const CARD_TYPE: u32 = 0x0186_626c;
    /// The card's formatted flag (`TkMcGetInfo`'s fourth out).
    pub const CARD_FORMAT: u32 = 0x0186_6270;
    /// The card's free space (`TkMcGetInfo`'s third out).
    pub const CARD_FREE: u32 = 0x0186_6274;
    /// `g_TkSaveSlotsUsed`.
    pub const SLOTS_USED: u32 = 0x0186_6278;
    /// The default load cursor row.
    pub const DEFAULT_POS: u32 = 0x0186_6280;
    /// The load cursor row.
    pub const LOAD_POS: u32 = 0x0186_6284;
    /// `u8`: nonzero in the save menu (0: load).
    pub const SAVING: u32 = 0x0186_628a;
    /// `g_TkMenuSaveSlot`.
    pub const SLOT: u32 = 0x0186_628c;
    /// `u8[99] g_TkSaveSlots`.
    pub const SLOTS: u32 = 0x0186_6290;
    /// The open card file.
    pub const FD: u32 = 0x0186_62f4;
    /// `u8[99]`: each slot's size in card clusters.
    pub const SLOT_SIZES: u32 = 0x0186_62f8;
    /// The directory buffer `TkMcGetDir` fills.
    pub const DIR: u32 = 0x0186_6380;
    /// The icon file's size on the card (from the directory entry).
    pub const DIR_ICON_SIZE: u32 = 0x0186_6390;
    /// The clusters one save needs.
    pub const NEED: u32 = 0x0186_63c0;
    /// `g_TkMenuSaveLoadActive` (a save or a load happened).
    pub const ACTIVE: u32 = 0x0186_63c4;
    /// The busy sub-command window (`MenuSubCmd_OpenC`).
    pub const BUSY_WINDOW: u32 = 0x0186_63c8;
    /// The icon file's size (`g_TkMenuStartWord`, 0x83a0).
    pub const ICON_SIZE: u32 = 0x025d_16d0;
    /// `g_TkMn2ListMax2` (the yes/no window).
    pub const YES_NO: u32 = 0x023c_c120;
    /// The card directory names `TkMcGetDir` reads.
    pub const CARD_DIR: u32 = 0x023c_bda0;
    pub const CARD_DIR_CLEAR: u32 = 0x023c_bea0;
    pub const CARD_DIR_SRC: u32 = 0x023c_c0a0;
    pub const CARD_DIR_OUT: u32 = 0x0132_5b00;
    /// `u8 g_FFXHddMode`.
    pub const HDD_MODE: u32 = 0x0133_c913;
    /// `"scene3"`, `"scene33"`, `"scene34"`.
    pub const SCENES: [u32; 3] = [0x00b6_2fcc, 0x00b6_2fd4, 0x00b6_2fdc];
}

/// [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{callee as c, global as g};

    /// The out word's offset in the frame.
    const OUT: u32 = 0x80;

    fn go<M, C>(mem: &mut M, call: &mut C, addr: u32, args: &[u32]) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let a = mem.rebase(addr);
        call(mem, a, args)
    }

    fn gl<M: Mem>(mem: &M, at: u32) -> u32 {
        mem.read(mem.rebase(at))
    }

    fn gb<M: Mem>(mem: &M, at: u32) -> u8 {
        mem.read(mem.rebase(at))
    }

    fn put<M: Mem>(mem: &mut M, at: u32, v: u32) {
        let a = mem.rebase(at);
        mem.write(a, v);
    }

    /// `TkMenuSaveMes(id)`.
    fn mes<M, C>(mem: &mut M, call: &mut C, id: u32) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        go(mem, call, c::SAVE_MES, &[id])
    }

    /// The three save/load scenes stopped.
    fn stop_scenes<M, C>(mem: &mut M, call: &mut C)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        for s in g::SCENES {
            let name = mem.rebase(s);
            go(mem, call, c::STOP_SCENE, &[1, name]);
        }
    }

    /// The choice row's card is usable.
    fn row_enabled<M: Mem>(mem: &M) -> bool {
        let row = gl(mem, g::ROW);
        mem.read::<u8>(mem.rebase(g::ENABLED).wrapping_add(row)) != 0
    }

    /// The yes/no window's outcome byte.
    fn yes_no<M: Mem>(mem: &M) -> (i8, u32) {
        let w = gl(mem, g::YES_NO);
        (mem.read::<i8>(w.wrapping_add(0x45)), w)
    }

    /// The loaded save taken over (states 0x25 and 0x68): the load
    /// position kept, the windows closed, the save copied in, the sound
    /// and HDD modes and the sphere grid refreshed.
    fn take_loaded<M, C>(mem: &mut M, call: &mut C)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        go(mem, call, c::SND_SEP_PLAY_SIMPLE, &[0x8000_006a]);
        let slot = gl(mem, g::SLOT);
        put(mem, g::ACTIVE, 1);
        let row = gl(mem, g::ROW);
        go(mem, call, c::MC_SET_LOAD_FILE_POS, &[row, slot]);
        go(mem, call, c::CLOSE_WINDOW, &[]);
        go(mem, call, c::CLOSE_YES_NO_WINDOW, &[]);
        let buffer = mem.rebase(g::SAVE_FILE);
        let event = go(mem, call, c::GET_SAVE_EVENT_ADDRESS, &[buffer]);
        go(mem, call, c::COPY_LOADED_SAVE, &[event, buffer]);
        let m = mes(mem, call, 0x2008);
        go(mem, call, c::SET_HELP_MESSAGE, &[m]);
        go(mem, call, c::REFRESH_SOUND_MODE, &[]);
        go(mem, call, c::REFRESH_HDD, &[]);
        stop_scenes(mem, call);
        go(mem, call, c::ABM_PARA_GET, &[]);
    }

    /// The save packed and its icon.sys built: `(dic, packed)` order as
    /// the original's calls.
    fn pack_save<M, C>(mem: &mut M, call: &mut C) -> (u32, u32)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let dic = go(mem, call, c::GET_SAVE_DIC, &[]);
        let slot = gl(mem, g::SLOT);
        let event = go(mem, call, c::GET_SAVE_EVENT_ADDRESS, &[slot]);
        let buffer = mem.rebase(g::SAVE_FILE);
        let packed = go(mem, call, c::PACK_SAVE, &[buffer, event, slot]);
        (dic, packed)
    }

    /// `MenuSubCmd_OpenC(TkMenuSaveMes(0x2010), 0, 0)` into the busy
    /// window.
    fn open_busy<M, C>(mem: &mut M, call: &mut C)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let m = mes(mem, call, 0x2010);
        let w = go(mem, call, c::SUB_CMD_OPEN_C, &[m, 0, 0]);
        put(mem, g::BUSY_WINDOW, w);
    }

    /// A notice (`TkMenuOpenSubCmdWindow(mes, 0, 0)`) after closing the
    /// yes/no window, then the save/load window and the scenes closed and
    /// no help (states 0x5e, 0x6b).
    fn notice<M, C>(mem: &mut M, call: &mut C, id: u32)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        go(mem, call, c::CLOSE_YES_NO_WINDOW, &[]);
        let m = mes(mem, call, id);
        go(mem, call, c::OPEN_SUB_CMD_WINDOW, &[m, 0, 0]);
        go(mem, call, c::CLOSE_WINDOW, &[]);
        stop_scenes(mem, call);
        go(mem, call, c::SET_HELP_MESSAGE, &[0]);
    }

    /// `TkMenuCtrlSaveLoad(m)` (`0x008b15d0`): one step of the save or
    /// load menu (see the module). `frame`: [`super::FRAME_WORDS`] words
    /// of scratch (a path buffer, `MenuSave_McSync`'s out word).
    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)] // one state machine
    pub fn tk_menu_ctrl_save_load<M, C>(mem: &mut M, m: u32, frame: u32, call: &mut C)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let pad = go(mem, call, c::GET_CONTROL_PAD, &[]) & 0xffff;
        let rep = go(mem, call, c::GET_CONTROL_PAD_REP, &[]) & 0xffff;
        let trg = go(mem, call, c::GET_CONTROL_PAD_TRG, &[]) & 0xffff;
        let state_at = m.wrapping_add(0x1c);
        let set = |mem: &mut M, v: u32| mem.write(state_at, v);
        let path = frame;
        let out = frame + OUT;
        // `MenuSave_McSync(1, 0, &out)`: `Some(out)` when it finished
        // (1), else its result
        let sync = |mem: &mut M, call: &mut C| -> Result<i32, i32> {
            let r = go(mem, call, c::MC_SYNC, &[1, 0, out]);
            if r == 1 {
                Ok(mem.read::<i32>(out))
            } else {
                Err(r.cast_signed())
            }
        };
        let mut state = mem.read::<u32>(state_at);
        loop {
            if state > 0x71 {
                return;
            }
            // `Some(next)` runs the next state now; `None` returns
            let next: Option<u32> = match state {
                0x00 => {
                    go(mem, call, c::RESET_TOGGLES, &[]);
                    go(mem, call, c::OPEN_WINDOW_3C20, &[]);
                    set(mem, 1);
                    put(mem, g::ROW, 0);
                    let at = mem.rebase(g::ENABLED);
                    mem.write(at, 0u16);
                    None
                }
                0x01 | 0x03 => {
                    if state == 1 {
                        set(mem, 3);
                    }
                    go(mem, call, c::POLL_CARD, &[]);
                    if gb(mem, g::TOGGLES) != 0 {
                        set(mem, 4);
                    }
                    None
                }
                0x02 => None,
                0x04..=0x06 => {
                    go(mem, call, c::POLL_CARD, &[]);
                    let on = gb(mem, g::TOGGLES) != 0;
                    match (state, on) {
                        (4, false) => set(mem, 5),
                        (5, true) => set(mem, 6),
                        (6, false) => {
                            go(mem, call, c::OPEN_WINDOW2, &[0]);
                            set(mem, 8);
                        }
                        _ => {}
                    }
                    None
                }
                0x07 => {
                    go(mem, call, c::RESET_TOGGLES, &[]);
                    let at = mem.rebase(g::ENABLED);
                    mem.write(at, 0u16);
                    go(mem, call, c::SET_HELP_RATIO_DIFF2, &[0xffff_fe00]);
                    set(mem, 8);
                    None
                }
                0x08 => {
                    go(mem, call, c::GET_CD, &[]);
                    set(mem, 9);
                    Some(9)
                }
                0x09 => {
                    let h = mes(mem, call, 0x2000);
                    go(mem, call, c::SET_HELP_MESSAGE, &[h]);
                    set(mem, 0xa);
                    go(mem, call, c::OPEN_CHOICE, &[m]);
                    None
                }
                0x0a => {
                    go(mem, call, c::POLL_CARD, &[]);
                    if trg & 0x20 != 0 {
                        if row_enabled(mem) {
                            go(mem, call, c::SND_SEP_PLAY_SIMPLE, &[0x8000_0001]);
                            let row = gl(mem, g::ROW);
                            go(mem, call, c::MC_SET_EXEC_PORT, &[row]);
                            set(mem, 0x10);
                        } else {
                            go(mem, call, c::SND_SEP_PLAY_SIMPLE, &[0x8000_0003]);
                        }
                        None
                    } else if trg & 0x40 == 0 {
                        if rep & 0x5000 != 0 {
                            go(mem, call, c::SND_SEP_PLAY_SIMPLE, &[0x8000_0001]);
                            let flipped = u32::from(gl(mem, g::ROW) == 0);
                            put(mem, g::ROW, flipped);
                        }
                        None
                    } else {
                        go(mem, call, c::SND_SEP_PLAY_SIMPLE, &[0x8000_0004]);
                        go(mem, call, c::CLOSE_HELP_WINDOW, &[]);
                        if gb(mem, g::SAVING) != 0 && gl(mem, g::ACTIVE) == 0 {
                            Some(0xb)
                        } else {
                            Some(0x6d)
                        }
                    }
                }
                0x0b => {
                    let a = mes(mem, call, 0x202f);
                    let b = mes(mem, call, 0x2006);
                    let d = mes(mem, call, 0x2007);
                    go(mem, call, c::SET_HELP_MESSAGE, &[0]);
                    go(mem, call, c::OPEN_SUB_CMD_WINDOW, &[a, b, d]);
                    set(mem, 0xc);
                    None
                }
                0x0c => {
                    let w = go(mem, call, c::GET_LIST_MAX2, &[]);
                    let b = mem.read::<i8>(w.wrapping_add(0x45));
                    match b.cmp(&0) {
                        std::cmp::Ordering::Less => {
                            go(mem, call, c::CLOSE_YES_NO_WINDOW, &[]);
                            Some(7)
                        }
                        std::cmp::Ordering::Equal => None,
                        std::cmp::Ordering::Greater => {
                            let v = mem.read::<i16>(w.wrapping_add(0x48));
                            go(mem, call, c::CLOSE_YES_NO_WINDOW, &[]);
                            Some(if v != 0 { 7 } else { 0x6d })
                        }
                    }
                }
                0x0d | 0x0e => {
                    go(mem, call, c::RESET_TOGGLES, &[]);
                    if state == 0xd {
                        go(mem, call, c::CLOSE_WINDOW, &[]);
                        go(mem, call, c::SET_HELP_MESSAGE, &[0]);
                    } else {
                        go(mem, call, c::SET_HELP_MESSAGE, &[0]);
                        go(mem, call, c::CLOSE_WINDOW, &[]);
                    }
                    go(mem, call, c::CLOSE_YES_NO_WINDOW, &[]);
                    stop_scenes(mem, call);
                    let first = gl(mem, g::ROW) == 0;
                    let id = match (state, first) {
                        (0xd, true) => 0x200a,
                        (0xd, false) => 0x202d,
                        (_, true) => 0x2012,
                        (_, false) => 0x2030,
                    };
                    let t = mes(mem, call, id);
                    if state == 0xd {
                        go(mem, call, c::SET_HELP_RATIO_DIFF2, &[0xffff_fe00]);
                    }
                    go(mem, call, c::SUB_CMD_OPEN_A, &[t, 0, 0]);
                    set(mem, 0xf);
                    None
                }
                0x0f => {
                    go(mem, call, c::POLL_CARD, &[]);
                    if trg != 0 {
                        go(mem, call, c::CLOSE_YES_NO_WINDOW, &[]);
                        go(mem, call, c::SET_HELP_RATIO_DIFF2, &[0xffff_fe00]);
                        go(mem, call, c::SND_SEP_PLAY_SIMPLE, &[0x8000_0001]);
                        set(mem, 7);
                    }
                    None
                }
                0x10 | 0x11 => {
                    if state == 0x10 {
                        open_busy(mem, call);
                    }
                    go(mem, call, c::SET_HELP_RATIO_DIFF2, &[0x200]);
                    let row = gl(mem, g::ROW);
                    go(mem, call, c::SAVE_LOAD_RESET, &[row]);
                    go(mem, call, c::CLOSE_HELP_WINDOW, &[]);
                    if state == 0x11 {
                        go(mem, call, c::CLOSE_WINDOW, &[]);
                        stop_scenes(mem, call);
                    }
                    let h = mes(mem, call, 0x2011);
                    go(mem, call, c::SET_HELP_MESSAGE, &[h]);
                    set(mem, if state == 0x10 { 0x14 } else { 0x12 });
                    None
                }
                0x12 => {
                    let row = gl(mem, g::ROW);
                    let ty = mem.rebase(g::CARD_TYPE);
                    let r = go(mem, call, c::MC_GET_INFO, &[row, ty, 0, 0]);
                    if r == 0 {
                        set(mem, 0x13);
                    } else if r.cast_signed() < 0 {
                        set(mem, 0xe);
                    }
                    None
                }
                0x13 => {
                    match sync(mem, call) {
                        Ok(o) => {
                            if o < -2 || gl(mem, g::CARD_TYPE) != 2 {
                                set(mem, 0xe);
                            } else {
                                set(mem, 0x14);
                            }
                        }
                        Err(r) if r < 0 => set(mem, 0xe),
                        Err(_) => {}
                    }
                    None
                }
                0x14 | 0x15 => {
                    if state == 0x14 {
                        set(mem, 0x15);
                    }
                    set(mem, 0x16);
                    Some(0x16)
                }
                0x16 => {
                    for k in 0..4 {
                        put(mem, g::CARD_DIR_CLEAR + 4 * k, 0);
                    }
                    let (dir, src, dout) = (
                        mem.rebase(g::CARD_DIR),
                        mem.rebase(g::CARD_DIR_SRC),
                        mem.rebase(g::CARD_DIR_OUT),
                    );
                    go(mem, call, c::MC_INIT_DIR, &[dir, src]);
                    if go(mem, call, c::MC_GET_DIR, &[dir, 0, 1, dout]) != 0 {
                        set(mem, 0xe);
                        None
                    } else {
                        set(mem, 0x18);
                        Some(0x18)
                    }
                }
                0x17 => {
                    let (dir, dout) = (mem.rebase(g::CARD_DIR), mem.rebase(g::CARD_DIR_OUT));
                    if go(mem, call, c::MC_GET_DIR, &[dir, 1, 1, dout]) != 0 {
                        set(mem, 0xe);
                        set(mem, 0x19);
                        Some(0x19)
                    } else {
                        set(mem, 0x18);
                        Some(0x18)
                    }
                }
                0x18 => {
                    set(mem, 0x19);
                    Some(0x19)
                }
                0x19 | 0x1e => {
                    if state == 0x1e {
                        go(mem, call, c::CLOSE_YES_NO_WINDOW, &[]);
                        let row = gl(mem, g::ROW);
                        let p = go(mem, call, c::MC_GET_LOAD_FILE_POS, &[row]);
                        let v = if p.cast_signed() < 0 {
                            gl(mem, g::DEFAULT_POS)
                        } else {
                            let row = gl(mem, g::ROW);
                            go(mem, call, c::MC_GET_LOAD_FILE_POS, &[row])
                        };
                        put(mem, g::LOAD_POS, v);
                    }
                    let row = gl(mem, g::ROW);
                    let r = go(mem, call, c::MC_GET_INFO, &[row, 0, 0, 0]);
                    if r == 0 {
                        let n = if state == 0x19 { 0x1a } else { 0x1f };
                        set(mem, n);
                        Some(n)
                    } else {
                        if r.cast_signed() < 0 {
                            set(mem, 0xe);
                        }
                        None
                    }
                }
                0x1a | 0x1b => {
                    if state == 0x1a {
                        set(mem, 0x1b);
                    }
                    if go(mem, call, c::FIRST_READ_END, &[]) == 1 {
                        set(mem, 0x1e);
                    }
                    None
                }
                0x1c => {
                    go(mem, call, c::SET_HELP_RATIO_DIFF2, &[0xffff_fe00]);
                    set(mem, 0x1d);
                    None
                }
                0x1d => {
                    if go(mem, call, c::GET_GLOBAL_1841C0C, &[]) == 0 {
                        set(mem, 7);
                    }
                    None
                }
                0x1f => {
                    set(mem, 0x20);
                    Some(0x20)
                }
                0x20 => {
                    put(mem, g::SLOTS_USED, 1);
                    set(mem, 0x21);
                    Some(0x21)
                }
                0x21 => {
                    go(mem, call, c::SND_SEP_PLAY_SIMPLE, &[0x8000_0010]);
                    go(mem, call, c::OPEN_WINDOW, &[m]);
                    let w = gl(mem, g::WINDOW);
                    let pos = mem.read::<u16>(mem.rebase(g::LOAD_POS));
                    let rows = mem.read::<u16>(w.wrapping_add(0x30));
                    let shown = mem.read::<u16>(w.wrapping_add(0x3a));
                    mem.write(w.wrapping_add(0x48), pos);
                    mem.write(w.wrapping_add(0x32), 0u16);
                    if i32::from(rows) - i32::from(shown) <= 0 {
                        mem.write(w.wrapping_add(0x32), rows.wrapping_sub(shown));
                    }
                    if mem.read::<i16>(w.wrapping_add(0x32)) < 0 {
                        mem.write(w.wrapping_add(0x32), 0u16);
                    }
                    let v = mem.read::<u16>(w.wrapping_add(0x32));
                    mem.write(w.wrapping_add(0x34), v);
                    go(mem, call, c::RESET_TOGGLES, &[]);
                    set(mem, 0x25);
                    None
                }
                0x22 => {
                    go(mem, call, c::RESET_TOGGLES, &[]);
                    set(mem, 0x23);
                    None
                }
                0x23 => {
                    go(mem, call, c::POLL_CARD, &[]);
                    if !row_enabled(mem) {
                        go(mem, call, c::CLOSE_YES_NO_WINDOW, &[]);
                        go(mem, call, c::SET_HELP_RATIO_DIFF2, &[0xffff_fe00]);
                        set(mem, 0xd);
                    } else if yes_no(mem).0 != 0 {
                        go(mem, call, c::CLOSE_YES_NO_WINDOW, &[]);
                        go(mem, call, c::SET_HELP_RATIO_DIFF2, &[0xffff_fe00]);
                        set(mem, 7);
                    }
                    None
                }
                0x24 | 0x25 => {
                    if state == 0x24 {
                        let w = gl(mem, g::WINDOW);
                        go(mem, call, c::CLEAR_SELECT_WINDOW, &[w]);
                        go(mem, call, c::RESET_TOGGLES, &[]);
                        let id = if gb(mem, g::SAVING) == 0 {
                            0x201f
                        } else {
                            0x2004
                        };
                        let h = mes(mem, call, id);
                        go(mem, call, c::SET_HELP_MESSAGE, &[h]);
                        set(mem, 0x25);
                    }
                    pick_file(mem, call, pad, set);
                    None
                }
                0x26 => {
                    go(mem, call, c::POLL_CARD, &[]);
                    if !row_enabled(mem) {
                        go(mem, call, c::CLOSE_YES_NO_WINDOW, &[]);
                        go(mem, call, c::CLOSE_WINDOW, &[]);
                        go(mem, call, c::SET_HELP_RATIO_DIFF2, &[0xffff_fe00]);
                        stop_scenes(mem, call);
                        set(mem, 0xd);
                        None
                    } else if yes_no(mem).0 != 0 {
                        go(mem, call, c::CLOSE_YES_NO_WINDOW, &[]);
                        set(mem, 0x29);
                        Some(0x29)
                    } else {
                        None
                    }
                }
                0x27 => {
                    let slot_at = mem.rebase(g::SLOTS).wrapping_add(gl(mem, g::SLOT));
                    let s = mem.read::<u8>(slot_at);
                    if s == 1 || s == 2 {
                        save_over(mem, call, set);
                        None
                    } else {
                        set(mem, 0x2b);
                        Some(0x2b)
                    }
                }
                0x28 | 0x34 => {
                    go(mem, call, c::POLL_CARD, &[]);
                    if row_enabled(mem) {
                        let (b, w) = yes_no(mem);
                        if b == 0 {
                            None
                        } else {
                            let yes = mem.read::<i16>(w.wrapping_add(0x48)) == 0 && b >= 0;
                            go(mem, call, c::CLOSE_YES_NO_WINDOW, &[]);
                            let n = match (state, yes) {
                                (0x28, true) => 0x3f,
                                (0x28, false) => 0x29,
                                (_, true) => 0x37,
                                (_, false) => 0x35,
                            };
                            set(mem, n);
                            Some(n)
                        }
                    } else {
                        go(mem, call, c::CLOSE_YES_NO_WINDOW, &[]);
                        set(mem, 0xd);
                        None
                    }
                }
                0x29 | 0x2a | 0x35 | 0x36 => {
                    if state == 0x29 {
                        set(mem, 0x2a);
                    }
                    if state == 0x35 {
                        set(mem, 0x36);
                    }
                    set(mem, 0x24);
                    None
                }
                0x2b | 0x2c | 0x31 | 0x32 | 0x37 | 0x38 | 0x40 => {
                    let (mark, done) = match state {
                        0x2b | 0x2c => (0x2c, 0x2d),
                        0x31 | 0x32 => (0x32, 0x33),
                        0x37 | 0x38 => (0x38, 0x39),
                        _ => (0x40, 0x41),
                    };
                    if state != mark && state != 0x40 {
                        set(mem, mark);
                    }
                    if go(mem, call, c::SYNC_TOGGLE, &[]) == 0 {
                        None
                    } else {
                        set(mem, done);
                        Some(done)
                    }
                }
                0x2d | 0x2f => {
                    let row = gl(mem, g::ROW);
                    let (ty, fm) = (mem.rebase(g::CARD_TYPE), mem.rebase(g::CARD_FORMAT));
                    let r = go(mem, call, c::MC_GET_INFO, &[row, ty, 0, fm]);
                    if r == 0 {
                        set(mem, state + 1);
                    } else if r.cast_signed() < 0 {
                        set(mem, 0x5e);
                    }
                    None
                }
                0x2e | 0x30 => match sync(mem, call) {
                    Ok(o) => {
                        if o != 0 || gl(mem, g::CARD_TYPE) != 2 {
                            set(mem, 0x5e);
                        }
                        if gl(mem, g::CARD_FORMAT) == 0 {
                            set(mem, state + 1);
                            Some(state + 1)
                        } else {
                            set(mem, 0x3f);
                            None
                        }
                    }
                    Err(r) => {
                        if r < 0 {
                            set(mem, 0x5e);
                        }
                        None
                    }
                },
                0x33 => {
                    let d = mes(mem, call, 0x2007);
                    let b = mes(mem, call, 0x2006);
                    let id = gl(mem, g::ROW).wrapping_add(0x200b);
                    let a = mes(mem, call, id);
                    go(mem, call, c::OPEN_SUB_CMD_WINDOW, &[a, b, d]);
                    set(mem, 0x34);
                    None
                }
                0x39 => {
                    let r = go(mem, call, c::MC_FORMAT, &[]);
                    let h = mes(mem, call, 0x200d);
                    go(mem, call, c::SET_HELP_MESSAGE, &[h]);
                    open_busy(mem, call);
                    if r == 0 {
                        set(mem, 0x3a);
                    }
                    None
                }
                0x3a => match sync(mem, call) {
                    Ok(o) => {
                        let n = if o == 0 { 0x3b } else { 0x3d };
                        set(mem, n);
                        Some(n)
                    }
                    Err(_) => None,
                },
                0x3b => {
                    go(mem, call, c::CLOSE_YES_NO_WINDOW, &[]);
                    set(mem, 0x3c);
                    Some(0x3c)
                }
                0x3c => {
                    set(mem, 0x3f);
                    Some(0x3f)
                }
                0x3d => {
                    go(mem, call, c::CLOSE_YES_NO_WINDOW, &[]);
                    let t = mes(mem, call, 0x200f);
                    go(mem, call, c::OPEN_SUB_CMD_WINDOW, &[t, 0, 0]);
                    go(mem, call, c::CLOSE_WINDOW, &[]);
                    go(mem, call, c::SET_HELP_MESSAGE, &[0]);
                    stop_scenes(mem, call);
                    set(mem, 0x3e);
                    None
                }
                0x3e | 0x5f | 0x6c => {
                    if yes_no(mem).0 != 0 {
                        go(mem, call, c::CLOSE_YES_NO_WINDOW, &[]);
                        set(mem, 7);
                    }
                    None
                }
                0x3f => {
                    let h = mes(mem, call, 0x2009);
                    go(mem, call, c::SET_HELP_MESSAGE, &[h]);
                    open_busy(mem, call);
                    go(mem, call, c::SYNC_TOGGLE, &[]);
                    set(mem, 0x40);
                    None
                }
                0x41..=0x58 => write_save(mem, call, state, path, set, &sync),
                0x59 => {
                    set(mem, 0x5a);
                    None
                }
                0x5a => {
                    let row = gl(mem, g::ROW);
                    let free = mem.rebase(g::CARD_FREE);
                    if go(mem, call, c::MC_GET_INFO, &[row, 0, free, 0]) == 0 {
                        set(mem, 0x5b);
                        Some(0x5b)
                    } else {
                        None
                    }
                }
                0x5b => {
                    if let Ok(o) = sync(mem, call) {
                        if o == 0 {
                            set(mem, 0x5c);
                        } else {
                            if o == -2 {
                                put(mem, g::CARD_FREE, 0x2000);
                            }
                            set(mem, 0x5e);
                        }
                        mark_slots(mem, call);
                    }
                    None
                }
                0x5c | 0x5d => {
                    if state == 0x5c {
                        go(mem, call, c::CLOSE_YES_NO_WINDOW, &[]);
                        let slot = gl(mem, g::SLOT);
                        let info = go(mem, call, c::GET_SAVE_FILE_INFO, &[slot]);
                        // the save file's 0x40-byte header into its entry
                        let mut header = [0u8; 0x40];
                        let from = mem.rebase(g::SAVE_FILE);
                        for (k, b) in (0u32..).zip(header.iter_mut()) {
                            *b = mem.read::<u8>(from + k);
                        }
                        mem.write_bytes(info, &header);
                        // the original's `edx`: the slot, as read before
                        let at = mem.rebase(g::SLOTS).wrapping_add(slot);
                        mem.write(at, 1u8);
                        put(mem, g::ACTIVE, 1);
                        let row = gl(mem, g::ROW);
                        go(mem, call, c::MC_SET_LOAD_FILE_POS, &[row, slot]);
                        go(mem, call, c::SND_SEP_PLAY_SIMPLE, &[0x8000_002b]);
                        set(mem, 0x5d);
                    }
                    mark_slots(mem, call);
                    set(mem, 0x24);
                    None
                }
                0x5e => {
                    notice(mem, call, 0x2015);
                    set(mem, 0x5f);
                    None
                }
                0x60 | 0x61 => {
                    if state == 0x60 {
                        set(mem, 0x61);
                    }
                    if go(mem, call, c::SYNC_TOGGLE, &[]) != 0 {
                        set(mem, 0x62);
                    }
                    None
                }
                0x62..=0x67 => read_save(mem, call, state, path, set, &sync),
                0x68 => {
                    let slot = gl(mem, g::SLOT);
                    let info = go(mem, call, c::GET_SAVE_FILE_INFO, &[slot]);
                    let crc_at = mem.rebase(g::SAVE_CRC);
                    let ok = if mem.read::<u16>(info.wrapping_add(0x1a)) == 0 {
                        mem.read::<u16>(crc_at) == 0
                    } else {
                        let stored = mem.read::<u16>(crc_at);
                        mem.write(crc_at, 0u32);
                        let body = mem.rebase(g::SAVE_BODY);
                        #[allow(clippy::cast_possible_truncation)]
                        let crc = go(mem, call, c::CRC, &[0x68c0, body]) as u16;
                        let want = mem.read::<u16>(info.wrapping_add(0x1a));
                        want == crc && want == stored
                    };
                    if ok {
                        take_loaded(mem, call);
                        set(mem, 0x69);
                    } else {
                        set(mem, 0x6b);
                    }
                    None
                }
                0x69 => {
                    set(mem, 0x6d);
                    put(mem, g::LANG_SWAP, 1);
                    None
                }
                0x6a => {
                    go(mem, call, c::CLOSE_YES_NO_WINDOW, &[]);
                    None
                }
                0x6b => {
                    notice(mem, call, 0x201d);
                    go(mem, call, c::SND_SEP_PLAY_SIMPLE, &[0x8000_0003]);
                    set(mem, 0x6c);
                    None
                }
                0x6d | 0x6e => {
                    if state == 0x6d {
                        let active = gl(mem, g::ACTIVE);
                        go(mem, call, c::HELP_SET_EXT_MODE, &[0, active]);
                        if go(mem, call, c::GET_MENU_MODE, &[]) & 0xffff_0000 != 0 {
                            go(mem, call, c::SET_MAIN_FRAME_RATIO_DIFF, &[0x10]);
                        }
                        set(mem, 0x6e);
                    }
                    if go(mem, call, c::GET_MENU_MODE, &[]) & 0xffff_0000 == 0 {
                        set(mem, 0x6f);
                        Some(0x6f)
                    } else {
                        if go(mem, call, c::GET_MAIN_FRAME_RATIO, &[]).cast_signed() >= 0x80 {
                            set(mem, 0x6f);
                        }
                        None
                    }
                }
                0x6f => {
                    if go(mem, call, c::LANG_SWAP_FLAG, &[]) == 1 && gl(mem, g::LANG_SWAP) == 1 {
                        let english = go(mem, call, c::SAVE_CONFIG_ENGLISH, &[]);
                        go(mem, call, c::SWAP_ENV_INIT, &[english]);
                        set(mem, 0x70);
                    } else {
                        set(mem, 0x71);
                    }
                    None
                }
                0x70 => {
                    if go(mem, call, c::SWAP_ENV_EXEC, &[]) == 1 {
                        set(mem, 0x71);
                        put(mem, g::LANG_SWAP, 0);
                    }
                    None
                }
                _ => {
                    // 0x71: back to the main menu (save menu) or slot 0x10
                    go(mem, call, c::CLOSE_WINDOW2, &[]);
                    if gb(mem, g::SAVING) != 0 {
                        go(mem, call, c::MAIN_SLEEP_MENU, &[0xf]);
                        go(mem, call, c::MAIN_ACTIVE_MENU, &[1, 0]);
                    } else {
                        go(mem, call, c::MAIN_SLEEP_MENU, &[0x10]);
                    }
                    None
                }
            };
            match next {
                Some(n) => state = n,
                None => return,
            }
        }
    }

    /// `MenuSaveLoad_MarkSlots(free, need)`.
    fn mark_slots<M, C>(mem: &mut M, call: &mut C)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let (free, need) = (gl(mem, g::CARD_FREE), gl(mem, g::NEED));
        go(mem, call, c::MARK_SLOTS, &[free, need]);
    }

    /// States 0x24/0x25: the file list. A confirm (pad bits 0/2) in the
    /// save menu saves at once (slot 1, `debugSave`); in the load menu it
    /// loads (`debugLoad`, bit 0 clear: 1) and takes the save over when
    /// that returns 0. Else (or after a failed load) the card is polled:
    /// no card ends (0xd), a pick keeps the cursor row as the slot (0x27
    /// save, 0x60 load), a cancel closes (0x1c).
    fn pick_file<M, C>(mem: &mut M, call: &mut C, pad: u32, set: impl Fn(&mut M, u32))
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        if pad & 5 != 0 {
            let buffer = mem.rebase(g::SAVE_FILE);
            if gb(mem, g::SAVING) != 0 {
                put(mem, g::SLOT, 1);
                let (dic, packed) = pack_save(mem, call);
                go(mem, call, c::DEBUG_SAVE, &[1, buffer, 0x6900]);
                let slot = gl(mem, g::SLOT);
                go(mem, call, c::BUILD_ICON_SYS, &[slot, packed, dic]);
                return;
            }
            let flag = u32::from(pad & 1 == 0);
            if go(mem, call, c::DEBUG_LOAD, &[flag, buffer, 0x6900]) == 0 {
                go(mem, call, c::SWITCH_GPU_MODE, &[]);
                let w = gl(mem, g::WINDOW);
                put(mem, g::SLOT, 1);
                mem.write(w.wrapping_add(0x45), 1u8);
                take_loaded(mem, call);
                let at = mem.rebase(g::HDD_MODE);
                mem.write(at, 0u8);
                set(mem, 0x69);
                return;
            }
        }
        go(mem, call, c::POLL_CARD, &[]);
        if !row_enabled(mem) {
            set(mem, 0xd);
            return;
        }
        let w = gl(mem, g::WINDOW);
        let b = mem.read::<i8>(w.wrapping_add(0x45));
        if b > 0 {
            let row = i32::from(mem.read::<i16>(w.wrapping_add(0x2c))).cast_unsigned();
            put(mem, g::SLOT, row);
            set(mem, if gb(mem, g::SAVING) != 0 { 0x27 } else { 0x60 });
        } else if b < 0 {
            go(mem, call, c::CLOSE_WINDOW, &[]);
            stop_scenes(mem, call);
            set(mem, 0x1c);
        }
    }

    /// State 0x27 on a used slot: a slot that is the game's own (2) and
    /// too big for the card's free space refuses (0x26); else the
    /// overwrite question (0x28).
    fn save_over<M, C>(mem: &mut M, call: &mut C, set: impl Fn(&mut M, u32))
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let slot = gl(mem, g::SLOT);
        let state = mem.read::<u8>(mem.rebase(g::SLOTS).wrapping_add(slot));
        if state == 2 {
            let size = u32::from(mem.read::<u8>(mem.rebase(g::SLOT_SIZES).wrapping_add(slot)));
            let room = gl(mem, g::NEED).wrapping_sub(size);
            if gl(mem, g::CARD_FREE).cast_signed() < room.cast_signed() {
                go(mem, call, c::SND_SEP_PLAY_SIMPLE, &[0x8000_0003]);
                let id = if gl(mem, g::ROW) == 0 { 0x202b } else { 0x202e };
                let t = mes(mem, call, id);
                go(mem, call, c::OPEN_SUB_CMD_WINDOW, &[t, 0, 0]);
                go(mem, call, c::SET_HELP_MESSAGE, &[0]);
                set(mem, 0x26);
                return;
            }
        }
        let d = mes(mem, call, 0x2007);
        let b = mes(mem, call, 0x2006);
        let a = mes(mem, call, 0x2005);
        go(mem, call, c::OPEN_SUB_CMD_WINDOW, &[a, b, d]);
        set(mem, 0x28);
    }

    /// The card steps of a save (0x41..0x58): the directory (made when
    /// missing), the icon (written unless the card's one has the size),
    /// icon.sys, the data; each call then `MenuSave_McSync`. A call that
    /// fails with a negative result, or a sync whose result is an error,
    /// goes to 0x5e (the error notice).
    #[allow(clippy::too_many_lines)] // the steps in turn
    fn write_save<M, C, S>(
        mem: &mut M,
        call: &mut C,
        state: u32,
        path: u32,
        set: impl Fn(&mut M, u32),
        sync: &S,
    ) -> Option<u32>
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
        S: Fn(&mut M, &mut C) -> Result<i32, i32>,
    {
        // a card call: 0 goes on to `next`, a negative result to 0x5e
        let step = |mem: &mut M, r: u32, next: u32| {
            if r == 0 {
                set(mem, next);
                Some(next)
            } else {
                if r.cast_signed() < 0 {
                    set(mem, 0x5e);
                }
                None
            }
        };
        let slot = |mem: &M| gl(mem, g::SLOT);
        let fd = |mem: &M| gl(mem, g::FD);
        match state {
            0x41 | 0x45 => {
                let s = slot(mem);
                let builder = if state == 0x41 {
                    c::PATH_DIR
                } else {
                    c::PATH_ICON
                };
                go(mem, call, builder, &[path, s]);
                let dir = mem.rebase(g::DIR);
                let r = go(mem, call, c::MC_GET_DIR, &[path, 0, 1, dir]);
                step(mem, r, state + 1)
            }
            0x43 => {
                let s = slot(mem);
                go(mem, call, c::PATH_DIR, &[path, s]);
                let r = go(mem, call, c::MC_MKDIR, &[path]);
                step(mem, r, 0x44)
            }
            0x47 | 0x4d | 0x53 => {
                let s = slot(mem);
                let builder = match state {
                    0x47 => c::PATH_ICON,
                    0x4d => c::PATH_ICON_SYS,
                    _ => c::PATH_DATA,
                };
                go(mem, call, builder, &[path, s]);
                let r = go(mem, call, c::MC_OPEN, &[path, 0x202]);
                step(mem, r, state + 1)
            }
            0x49 => {
                let (f, icon, n) = (fd(mem), mem.rebase(g::ICON), gl(mem, g::ICON_SIZE));
                let r = go(mem, call, c::MC_WRITE, &[f, icon, n]);
                step(mem, r, 0x4a)
            }
            0x4f => {
                let (dic, packed) = pack_save(mem, call);
                let s = slot(mem);
                go(mem, call, c::BUILD_ICON_SYS, &[s, packed, dic]);
                let (f, icon_sys) = (fd(mem), mem.rebase(g::ICON_SYS));
                let r = go(mem, call, c::MC_WRITE, &[f, icon_sys, 0x3c4]);
                step(mem, r, 0x50)
            }
            0x55 => {
                let (f, data) = (fd(mem), mem.rebase(g::SAVE_FILE));
                let r = go(mem, call, c::MC_WRITE, &[f, data, 0x6900]);
                step(mem, r, 0x56)
            }
            0x4b | 0x51 | 0x57 => {
                let f = fd(mem);
                let r = go(mem, call, c::MC_CLOSE, &[f]);
                step(mem, r, state + 1)
            }
            _ => {
                // the syncs
                let o = match sync(mem, call) {
                    Ok(o) => o,
                    Err(r) => {
                        if r < 0 {
                            set(mem, 0x5e);
                        }
                        return None;
                    }
                };
                let next = match state {
                    0x42 => {
                        if o >= 1 {
                            0x45
                        } else if o == -4 || o == 0 {
                            0x43
                        } else {
                            set(mem, 0x5e);
                            return None;
                        }
                    }
                    0x44 => {
                        if o != -4 && o != 0 {
                            set(mem, 0x5e);
                            return None;
                        }
                        0x45
                    }
                    0x46 => {
                        if o < 1 {
                            if o != -4 && o != 0 {
                                set(mem, 0x5e);
                                return None;
                            }
                            0x47
                        } else {
                            let have = gl(mem, g::DIR_ICON_SIZE).wrapping_add(0x3ff) >> 10;
                            let want = gl(mem, g::ICON_SIZE).cast_signed().wrapping_add(0x3ff);
                            let want = (want + ((want >> 31) & 0x3ff)) >> 10;
                            if have.cast_signed() == want {
                                0x4d
                            } else {
                                0x47
                            }
                        }
                    }
                    // the writes' syncs: a negative count ends quietly
                    0x4a | 0x50 | 0x56 => {
                        if o < 0 {
                            return None;
                        }
                        state + 1
                    }
                    0x48 | 0x4e | 0x54 => {
                        if o < 0 {
                            set(mem, 0x5e);
                            return None;
                        }
                        put(mem, g::FD, o.cast_unsigned());
                        state + 1
                    }
                    _ => {
                        // 0x4c, 0x52, 0x58
                        if o < 0 {
                            set(mem, 0x5e);
                            return None;
                        }
                        state + 1
                    }
                };
                set(mem, next);
                Some(next)
            }
        }
    }

    /// The card steps of a load (0x62..0x67): the busy window, the data
    /// file opened, read and closed, each then synced; a negative result
    /// goes to 0x6a.
    fn read_save<M, C, S>(
        mem: &mut M,
        call: &mut C,
        state: u32,
        path: u32,
        set: impl Fn(&mut M, u32),
        sync: &S,
    ) -> Option<u32>
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
        S: Fn(&mut M, &mut C) -> Result<i32, i32>,
    {
        let step = |mem: &mut M, r: u32, next: u32| {
            if r == 0 {
                set(mem, next);
                Some(next)
            } else {
                if r.cast_signed() < 0 {
                    set(mem, 0x6a);
                }
                None
            }
        };
        match state {
            0x62 => {
                let h = mes(mem, call, 0x2008);
                go(mem, call, c::SET_HELP_MESSAGE, &[h]);
                open_busy(mem, call);
                let s = gl(mem, g::SLOT);
                go(mem, call, c::PATH_DATA, &[path, s]);
                let r = go(mem, call, c::MC_OPEN, &[path, 1]);
                step(mem, r, 0x63)
            }
            0x64 => {
                let (f, data) = (gl(mem, g::FD), mem.rebase(g::SAVE_FILE));
                let r = go(mem, call, c::MC_READ, &[f, data, 0x6900]);
                step(mem, r, 0x65)
            }
            0x66 => {
                let f = gl(mem, g::FD);
                let r = go(mem, call, c::MC_CLOSE, &[f]);
                step(mem, r, 0x67)
            }
            _ => {
                let o = match sync(mem, call) {
                    Ok(o) => o,
                    Err(r) => {
                        if r < 0 {
                            set(mem, 0x6a);
                        }
                        return None;
                    }
                };
                if o < 0 {
                    // the read's sync ends quietly
                    if state != 0x65 {
                        set(mem, 0x6a);
                    }
                    return None;
                }
                if state == 0x63 {
                    put(mem, g::FD, o.cast_unsigned());
                }
                set(mem, state + 1);
                Some(state + 1)
            }
        }
    }
}
