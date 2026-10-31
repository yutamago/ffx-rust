//! M5 `system-a` (round 3): the rest of the PC save layer: the
//! `SaveData*` forwarders to the save manager (`0x00ce81f4`), its load,
//! save and debug-save entry points, the save folder set-up
//! (`SHGetFolderPathW` + three directories), the manager's
//! initialisation, the Eternal Calm file sizes, a `std::string` move and
//! the water-wave shader setters.

/// `g_SaveDataManager`, the instance `SaveDataManager_instance` returns.
pub const MANAGER: u32 = 0x00ce_81f4;
/// The save buffer the manager allocates (`initSaveDataSys`) and the
/// debug save writes.
pub const SAVE_BUFFER: u32 = 0x00ce_81f0;
/// `g_SaveFolderSet` (1 once [`SAVE_DIR`] holds the folder).
pub const FOLDER_SET: u32 = 0x00ce_72e0;
/// `g_SaveFolder` (UTF-16).
pub const SAVE_DIR: u32 = 0x00ce_6ee0;
/// The three folder names (`"SQUARE ENIX"`, the game, the user id):
/// pointers to UTF-16 strings.
pub const FOLDER_NAMES: [u32; 3] = [0x00c3_5780, 0x00c3_5784, 0x00c3_5788];
/// `"%s\\%s"`-style formats (UTF-16): a path and a name joined, and the
/// final folder.
pub const JOIN_FORMAT: u32 = 0x00b4_b694;
pub const DIR_FORMAT: u32 = 0x00b4_b6a0;
/// The save machine's words (`m5a_savedata`): size, buffer, busy, state,
/// tick base; the file-name format and the user id; the write mode.
pub const SIZE: u32 = 0x00ce_7304;
pub const BUFFER: u32 = 0x00ce_7308;
pub const BUSY: u32 = 0x00ce_7310;
pub const STATE: u32 = 0x00ce_72e8;
pub const TICK_BASE: u32 = 0x00ce_730c;
pub const NAME_FORMAT: u32 = 0x00b4_b680;
pub const USER_ID: u32 = 0x00c3_577c;
pub const MODE_WRITE: u32 = 0x00b7_ca8c;
/// `g_SaveData_DialogState`, `g_SaveData_LoadResult` (u8).
pub const DIALOG_STATE: u32 = 0x00cc_b998;
pub const LOAD_RESULT: u32 = 0x0159_7f74;
/// Two words of the PS2 pad state `SaveDataToSave` clears around the save
/// (u16 each).
pub const PAD_WORDS: [u32; 2] = [0x0159_8058, 0x0159_805a];
/// The two-character tag `SaveDataLocRemoveNumTag` cuts at (a pointer).
pub const NUM_TAG: u32 = 0x00c3_2300;
/// `initSaveDataSys`'s strings: the stream-prefix format, the path it
/// builds (`0x00ce81b0`), the file-name format and its two arguments.
pub const PREFIX_FORMAT: u32 = 0x00b4_bbac;
pub const PREFIX_PATH: u32 = 0x00ce_81b0;
pub const PATH_FORMAT: u32 = 0x00b4_2a3c;
pub const PATH_ARG1: u32 = 0x00c3_8dc0;
pub const PATH_ARG2: u32 = 0x00b4_bb58;
/// The Eternal Calm tables: the first map, the MGRP sizes and names, the
/// scene sizes and names (7 words each), the "can't open" message.
pub const CALM_FIRST: u32 = 0x00c3_3468;
pub const CALM_MGRP_SIZES: u32 = 0x00c3_34a0;
pub const CALM_MGRP_NAMES: u32 = 0x00c3_34d4;
pub const CALM_SCENE_SIZES: u32 = 0x00c3_3484;
pub const CALM_SCENE_NAMES: u32 = 0x00c3_34b8;
pub const CALM_MESSAGE: u32 = 0x00b4_51f0;
/// The water-wave shader parameter names (the `WaterWaveController` setters).
pub const BUOYANCY_FIX: u32 = 0x00b4_bd68;
pub const DRAUGHT_OFFSET: u32 = 0x00b4_bd58;
pub const FOAM_STRENGTH: u32 = 0x00b4_bce4;
pub const MULTI_TEX_BLEND: u32 = 0x00b4_bcd0;
pub const CIRCLE_WAVE: [u32; 2] = [0x00b4_bcf4, 0x00b4_bd14];
pub const SINE_WAVE: [u32; 2] = [0x00b4_bd30, 0x00b4_bd44];
/// `setScrollSpeed`'s two groups of four names.
pub const SCROLL_A: [u32; 4] = [0x00b4_bd74, 0x00b4_bd88, 0x00b4_bd9c, 0x00b4_bdb0];
pub const SCROLL_B: [u32; 4] = [0x00b4_bdc4, 0x00b4_bdd8, 0x00b4_bdec, 0x00b4_be00];

/// The originals over game memory and the original callees.
pub mod bridge {
    #[allow(clippy::wildcard_imports)] // the module's addresses
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    /// `SaveDataManager_instance` (cdecl).
    pub const INSTANCE: u32 = 0x006f_2070;
    /// The manager's methods (thiscall): `SetWord0_006f1db0`,
    /// `isAlbhedLoaded`, `isDeletingFinished`, `isLoadingFinished`,
    /// `isSavingFinished`, `SaveData_ClearWord8_006f24d0`,
    /// `setCbResult` (`ret 4`), `isSaveEnabled`.
    pub const MARK_ALBHED: u32 = 0x006f_1db0;
    pub const IS_ALBHED: u32 = 0x006f_2080;
    pub const IS_DELETING: u32 = 0x006f_20d0;
    pub const IS_LOADING: u32 = 0x006f_2150;
    pub const IS_SAVING: u32 = 0x006f_21a0;
    pub const RESET_IDLE: u32 = 0x006f_24d0;
    pub const SET_CB_RESULT: u32 = 0x006f_2500;
    pub const IS_SAVE_ENABLED: u32 = 0x006f_2190;
    /// The ported entry points this list calls through the game.
    pub const DEBUG_SAVE_INTERNAL: u32 = 0x006f_0510;
    pub const CHECK_DEBUG_SAVE: u32 = 0x006f_1c40;
    pub const PRE_DEBUG_SAVE: u32 = 0x006f_2410;
    pub const SET_INFO: u32 = 0x006f_2560;
    pub const START_LOAD: u32 = 0x006f_01d0;
    pub const DO_LOAD: u32 = 0x006f_1ce0;
    pub const START_LOAD_ALL: u32 = 0x006e_feb0;
    pub const LOAD_ALL: u32 = 0x006f_1d10;
    pub const START_SAVE: u32 = 0x006f_07a0;
    pub const DO_SAVE: u32 = 0x006f_1d70;
    pub const SET_UP_FOLDER: u32 = 0x006f_0330;
    pub const CALM_FILE_SIZE: u32 = 0x0067_a9f0;
    /// The menu calls of a load or save start: `SaveLoadMenu_open_00702a80`,
    /// `SaveDataMachine_as3Clear_00702f80`, `SaveDataMachine_showPopUp_00702e90`.
    pub const MENU_OPEN: u32 = 0x0070_2a80;
    pub const MENU_CLEAR: u32 = 0x0070_2f80;
    pub const SHOW_POPUP: u32 = 0x0070_2e90;
    /// `SaveDataWriteCrc` (cdecl).
    pub const WRITE_CRC: u32 = 0x0064_8f20;
    /// `operator_new_006304c0` (cdecl).
    pub const NEW_ARRAY: u32 = 0x0063_04c0;
    /// `crt_memcpy_thunk`, `crt_memset_thunk` (cdecl).
    pub const MEMCPY: u32 = 0x0094_9256;
    pub const MEMSET: u32 = 0x0094_964c;
    /// `Phyre_PSerialization_PStreamFile_GetStreamPrefix` (cdecl).
    pub const STREAM_PREFIX: u32 = 0x0060_7d30;
    /// The debug load and save (`ffx_menu`; cdecl `(slot, buf, size)`).
    pub const DEBUG_LOAD: u32 = 0x0064_6b30;
    pub const DEBUG_LOAD_ALL: u32 = 0x0064_6ca0;
    pub const DEBUG_SAVE: u32 = 0x0064_6df0;
    /// `sceOpen`, `sceFileSize`, `sceClose` (cdecl).
    pub const SCE_OPEN: u32 = 0x0062_fa30;
    pub const SCE_FILE_SIZE: u32 = 0x0062_f760;
    pub const SCE_CLOSE: u32 = 0x0062_f610;
    /// `ShaderParams_FindByName` (thiscall, `ret 4`) and
    /// `ShaderParamBuffer_SetValue2` (thiscall, `ret 8`).
    pub const FIND_PARAM: u32 = 0x0056_cbf0;
    pub const SET_VALUE: u32 = 0x0063_86c0;
    /// Imports: `GetTickCount`, `wsprintfW`, `_wfopen`, `fwrite`,
    /// `fclose`, `strncpy`, `memmove`, `sprintf`, `SHGetFolderPathW`,
    /// `GetFileAttributesW`, `CreateDirectoryW`, `__iob_func`, `fprintf`.
    pub const GET_TICK_COUNT_IAT: u32 = 0x00b0_c0a0;
    pub const WSPRINTFW_IAT: u32 = 0x00b0_c674;
    pub const WFOPEN_IAT: u32 = 0x00b0_c47c;
    pub const FWRITE_IAT: u32 = 0x00b0_c428;
    pub const FCLOSE_IAT: u32 = 0x00b0_c3ec;
    pub const STRNCPY_IAT: u32 = 0x00b0_c368;
    pub const MEMMOVE_IAT: u32 = 0x00b0_c34c;
    pub const SPRINTF_IAT: u32 = 0x00b0_c3dc;
    pub const SH_GET_FOLDER_PATH_IAT: u32 = 0x00b0_c628;
    pub const GET_FILE_ATTRIBUTES_IAT: u32 = 0x00b0_c18c;
    pub const CREATE_DIRECTORY_IAT: u32 = 0x00b0_c194;
    pub const IOB_FUNC_IAT: u32 = 0x00b0_c350;
    pub const FPRINTF_IAT: u32 = 0x00b0_c38c;

    fn rd(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    fn wr(h: &mut impl Host, sym: u32, v: u32) {
        let at = h.rebase(sym);
        h.write(at, v);
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    fn manager(h: &mut impl Host) -> u32 {
        cdecl(h, INSTANCE, &[])
    }

    /// The manager's dialog object (`[0x00ce81f4]`) word `off` = `v`.
    fn dialog_word(h: &mut impl Host, off: u32, v: u32) {
        let d = rd(h, MANAGER);
        h.write(d.wrapping_add(off), v);
    }

    /// `SaveDataManager_MarkAlbhedLoaded()` (cdecl): a tail jump to
    /// `SetWord0_006f1db0(manager)` (`+0` = 1), which leaves `eax`
    /// alone: returns the manager.
    pub fn mark_albhed_loaded(h: &mut impl Host) -> u32 {
        let m = manager(h);
        h.call_sym(MARK_ALBHED, Conv::Thiscall(m), &[]);
        m
    }

    /// `SaveDataIsAlbhedLoaded()` (cdecl): 0 without a manager, else its
    /// `isAlbhedLoaded` (the manager asked again; a tail jump).
    pub fn is_albhed_loaded(h: &mut impl Host) -> u32 {
        if manager(h) == 0 {
            return 0;
        }
        let m = manager(h);
        h.call_sym(IS_ALBHED, Conv::Thiscall(m), &[])
    }

    /// A tail jump to the manager's `method` (thiscall, no words):
    /// `SaveDataIsDeletingFinished`, `IsLoadingFinished`,
    /// `IsSavingFinished`, `SaveDataResetToIdle`.
    pub fn manager_call(h: &mut impl Host, method: u32) -> u32 {
        let m = manager(h);
        h.call_sym(method, Conv::Thiscall(m), &[])
    }

    /// `SaveDataSetState(v)` (cdecl): the manager's `+8` = `v`; returns
    /// the manager.
    pub fn set_state(h: &mut impl Host, v: u32) -> u32 {
        let m = manager(h);
        h.write(m.wrapping_add(8), v);
        m
    }

    /// `SaveDataLocRemoveNumTag(src, dst)` (cdecl): `strncpy(dst, src,
    /// n)` and `dst[n] = 0`, `n` the index of the last place before the
    /// end where the two characters at `[0x00c32300]` occur (0 when
    /// none). Returns `strncpy`'s `eax`.
    pub fn loc_remove_num_tag(h: &mut impl Host, src: u32, dst: u32) -> u32 {
        let tag: u32 = rd(h, NUM_TAG);
        let mut len = 0u32;
        while h.read::<u8>(src.wrapping_add(len)) != 0 {
            len = len.wrapping_add(1);
        }
        let last = len.wrapping_sub(1).cast_signed();
        let mut cut = 0u32;
        let mut i = 0i32;
        while i < last {
            let at = src.wrapping_add(i.cast_unsigned());
            let matches = (0..2u32)
                .all(|j| h.read::<u8>(at.wrapping_add(j)) == h.read::<u8>(tag.wrapping_add(j)));
            if matches {
                cut = i.cast_unsigned();
            }
            i += 1;
        }
        let r = import(h, STRNCPY_IAT, Conv::Cdecl, &[dst, src, cut]);
        h.write(dst.wrapping_add(cut), 0u8);
        r
    }

    /// `SaveDataManager_setSaveDataInfo(this; title, detail, text)`
    /// (thiscall, `ret 0xc`): each non-null string `strncpy`'d to `+0x10`
    /// (0x40 bytes), `+0x50` (0x80), `+0xd0` (0x200). Returns the last
    /// `strncpy`'s `eax`, or `text` (0) when that one is null.
    pub fn set_save_data_info(
        h: &mut impl Host,
        this: u32,
        title: u32,
        detail: u32,
        text: u32,
    ) -> u32 {
        let mut r = 0;
        for (s, off, n) in [
            (title, 0x10, 0x40),
            (detail, 0x50, 0x80),
            (text, 0xd0, 0x200),
        ] {
            r = s;
            if s != 0 {
                r = import(h, STRNCPY_IAT, Conv::Cdecl, &[this.wrapping_add(off), s, n]);
            }
        }
        r
    }

    /// `SaveDataSetSaveDataInfo(a, b, c)` (cdecl): the manager's
    /// `setSaveDataInfo`.
    pub fn set_save_data_info_forward(
        h: &mut impl Host,
        title: u32,
        detail: u32,
        text: u32,
    ) -> u32 {
        let m = manager(h);
        h.call_sym(SET_INFO, Conv::Thiscall(m), &[title, detail, text])
    }

    /// `SaveDataManager_setSaveDataIconString(this; s)` (thiscall, `ret
    /// 4`): `strncpy(+0x2d0, s, 0x40)` when `s` is set. Returns 0.
    pub fn set_icon_string(h: &mut impl Host, this: u32, s: u32) -> u32 {
        if s != 0 {
            import(
                h,
                STRNCPY_IAT,
                Conv::Cdecl,
                &[this.wrapping_add(0x2d0), s, 0x40],
            );
        }
        0
    }

    /// The menu side of a load or save start: the save folder, the menu
    /// opened and cleared, popup 0x1fc. Returns 0.
    fn start_menu(h: &mut impl Host) -> u32 {
        cdecl(h, SET_UP_FOLDER, &[]);
        cdecl(h, MENU_OPEN, &[]);
        cdecl(h, MENU_CLEAR, &[]);
        cdecl(h, SHOW_POPUP, &[0x1fc]);
        0
    }

    /// `SaveDataMachine_StartLoad(size, buf)` / `0x006efeb0` (cdecl): the
    /// machine's size, busy flag and buffer set, state `state` (0xb a
    /// load, 0x15 the Al Bhed load-all), then [`start_menu`]. Returns 0.
    pub fn start_load(h: &mut impl Host, state: u32, size: u32, buf: u32) -> u32 {
        wr(h, SIZE, size);
        wr(h, BUSY, 1);
        wr(h, BUFFER, buf);
        wr(h, STATE, state);
        start_menu(h)
    }

    /// `0x006f07a0(size, buf)` (cdecl): busy, `SaveDataWriteCrc(buf)`, a
    /// copy of the `size` bytes in a new block (the machine's buffer),
    /// state 1, then [`start_menu`]. Returns 0.
    pub fn start_save(h: &mut impl Host, size: u32, buf: u32) -> u32 {
        wr(h, BUSY, 1);
        cdecl(h, WRITE_CRC, &[buf]);
        wr(h, SIZE, size);
        let copy = cdecl(h, NEW_ARRAY, &[size]);
        wr(h, BUFFER, copy);
        cdecl(h, MEMCPY, &[copy, buf, size]);
        wr(h, STATE, 1);
        start_menu(h)
    }

    /// `SaveDataManager_doLoad(this)` (`ecx`): `+0xc` = -1, the dialog
    /// state 3, `SaveDataMachine_StartLoad(+0x354, +0x350)`.
    pub fn do_load(h: &mut impl Host, this: u32) -> u32 {
        h.write(this.wrapping_add(0xc), u32::MAX);
        dialog_word(h, 8, 3);
        let (size, buf) = size_and_buffer(h, this);
        cdecl(h, START_LOAD, &[size, buf])
    }

    /// `0x006f1d10(this)` (`ecx`): byte `+0x364` = 0, the dialog state 3
    /// and its `+0xc` = -1, `0x006efeb0(+0x354, +0x350)`.
    pub fn load_all(h: &mut impl Host, this: u32) -> u32 {
        h.write(this.wrapping_add(0x364), 0u8);
        dialog_word(h, 8, 3);
        dialog_word(h, 0xc, u32::MAX);
        let (size, buf) = size_and_buffer(h, this);
        cdecl(h, START_LOAD_ALL, &[size, buf])
    }

    /// `SaveDataManager_doSave(this)` (`ecx`): 0 unless saving is
    /// enabled (`+0`); else the dialog state 1, `0x006f07a0(+0x354,
    /// +0x350)`.
    pub fn do_save(h: &mut impl Host, this: u32) -> u32 {
        if h.read::<u32>(this) == 0 {
            return 0;
        }
        dialog_word(h, 8, 1);
        let (size, buf) = size_and_buffer(h, this);
        cdecl(h, START_SAVE, &[size, buf])
    }

    /// The manager's save size (`+0x354`) and buffer (`+0x350`).
    fn size_and_buffer(h: &impl Host, this: u32) -> (u32, u32) {
        (
            h.read(this.wrapping_add(0x354)),
            h.read(this.wrapping_add(0x350)),
        )
    }

    /// `SaveDataToLoad()` (cdecl): the load result byte and the dialog
    /// state cleared, `setCbResult(0)`, a tail jump to `doLoad`.
    pub fn to_load(h: &mut impl Host) -> u32 {
        let at = h.rebase(LOAD_RESULT);
        h.write(at, 0u8);
        wr(h, DIALOG_STATE, 0);
        let m = manager(h);
        h.call_sym(SET_CB_RESULT, Conv::Thiscall(m), &[0]);
        let m = manager(h);
        h.call_sym(DO_LOAD, Conv::Thiscall(m), &[])
    }

    /// `SaveDataToLoadAllAlBhed()` (cdecl): a tail jump to `0x006f1d10`.
    pub fn to_load_all(h: &mut impl Host) -> u32 {
        manager_call(h, LOAD_ALL)
    }

    /// `SaveDataToSave()` (cdecl): the dialog state cleared,
    /// `setCbResult(0)`; saving disabled: dialog state 5, 0 (the
    /// `isSaveEnabled` answer). Else `doSave` with the two pad words
    /// at `0x01598058` (set to 0xffff) and `0x0159805a` (0) for the call,
    /// put back after it; `doSave`'s `eax`.
    pub fn to_save(h: &mut impl Host) -> u32 {
        wr(h, DIALOG_STATE, 0);
        let m = manager(h);
        h.call_sym(SET_CB_RESULT, Conv::Thiscall(m), &[0]);
        let m = manager(h);
        let enabled = h.call_sym(IS_SAVE_ENABLED, Conv::Thiscall(m), &[]);
        if enabled == 0 {
            wr(h, DIALOG_STATE, 5);
            return enabled;
        }
        let (pad0, pad1) = (h.rebase(PAD_WORDS[0]), h.rebase(PAD_WORDS[1]));
        let old0: u16 = h.read(pad0);
        h.write(pad0, 0xffffu16);
        let old1: u16 = h.read(pad1);
        h.write(pad1, 0u16);
        let m = manager(h);
        let r = h.call_sym(DO_SAVE, Conv::Thiscall(m), &[]);
        h.write(pad1, old1);
        h.write(pad0, old0);
        r
    }

    /// `SaveDataManager_preDebugSave(this; _)` (thiscall, `ret 4`):
    /// `memcpy([0x00ce81f0], +0x350, +0x354)`'s `eax`.
    pub fn pre_debug_save(h: &mut impl Host, this: u32) -> u32 {
        let (size, buf) = size_and_buffer(h, this);
        let dst = rd(h, SAVE_BUFFER);
        cdecl(h, MEMCPY, &[dst, buf, size])
    }

    /// `SaveDataPreDebugSave(x)` (cdecl): the manager's `preDebugSave(x)`.
    pub fn pre_debug_save_forward(h: &mut impl Host, x: u32) -> u32 {
        let m = manager(h);
        h.call_sym(PRE_DEBUG_SAVE, Conv::Thiscall(m), &[x])
    }

    /// The frame `SaveDataManager_debugSave_Internal_6f0650` needs: the
    /// path at `ebp - 0x404`.
    pub const DEBUG_SAVE_FRAME: u32 = 0x404;

    /// `SaveDataManager_debugSave_Internal_6f0650(size, buf)` (cdecl): the
    /// machine's tick base = `GetTickCount()`, `SaveDataWriteCrc(buf)`,
    /// the save folder, slot 0's path (`wsprintfW(ebp - 0x404, ..)`), and
    /// when `_wfopen` gives a file, `fwrite(buf, 1, size)` and `fclose`.
    /// Returns 0.
    pub fn debug_save_internal(h: &mut impl Host, ebp: u32, size: u32, buf: u32) -> u32 {
        let path = ebp.wrapping_sub(DEBUG_SAVE_FRAME);
        let tick = import(h, GET_TICK_COUNT_IAT, Conv::Stdcall, &[]);
        wr(h, TICK_BASE, tick);
        cdecl(h, WRITE_CRC, &[buf]);
        cdecl(h, SET_UP_FOLDER, &[]);
        let fmt = h.rebase(NAME_FORMAT);
        let dir = h.rebase(SAVE_DIR);
        let user = rd(h, USER_ID);
        import(h, WSPRINTFW_IAT, Conv::Cdecl, &[path, fmt, dir, user, 0]);
        let mode = h.rebase(MODE_WRITE);
        let f = import(h, WFOPEN_IAT, Conv::Cdecl, &[path, mode]);
        if f != 0 {
            import(h, FWRITE_IAT, Conv::Cdecl, &[buf, 1, size, f]);
            import(h, FCLOSE_IAT, Conv::Cdecl, &[f]);
        }
        0
    }

    /// `0x006f1c40(this; _)` (thiscall, `ret 4`): when the save buffer's
    /// (`+0x350`) word `+0x18` is 0x1b, `al` = 0 (the rest of `eax` the
    /// pointer's); else that word copied into the debug buffer
    /// (`[0x00ce81f0] + 0x18`), the debug save of it with size `+0x354`,
    /// and 1 when that returned 0, else 0.
    pub fn check_debug_save(h: &mut impl Host, this: u32) -> u32 {
        let save: u32 = h.read(this.wrapping_add(0x350));
        let word: u16 = h.read(save.wrapping_add(0x18));
        if word == 0x1b {
            return save & !0xff;
        }
        let dbg = rd(h, SAVE_BUFFER);
        h.write(dbg.wrapping_add(0x18), word);
        let size: u32 = h.read(this.wrapping_add(0x354));
        let r = cdecl(h, DEBUG_SAVE_INTERNAL, &[size, dbg]);
        u32::from(r == 0)
    }

    /// `AutoSave_DebugSave_00647e10(x)` (cdecl): the manager's
    /// `0x006f1c40(x)`: 0 when its `al` is set, else -1.
    pub fn auto_debug_save(h: &mut impl Host, x: u32) -> u32 {
        let m = manager(h);
        let r = h.call_sym(CHECK_DEBUG_SAVE, Conv::Thiscall(m), &[x]);
        if r & 0xff != 0 { 0 } else { u32::MAX }
    }

    /// `debugLoad_Thunk_006492b0` / `debugSave_Thunk_006492e0(slot, buf,
    /// size)`: tail jumps to `debugLoad_00646b30` / `debugSave` with the
    /// caller's three words.
    pub fn debug_thunk(h: &mut impl Host, target: u32, slot: u32, buf: u32, size: u32) -> u32 {
        cdecl(h, target, &[slot, buf, size])
    }

    /// `debugSavedataLoadAll(slot)` (cdecl): `debugLoad_00646ca0(slot,
    /// 0x01597f70, 0x6900)`.
    pub fn debug_load_all(h: &mut impl Host, slot: u32) -> u32 {
        let buf = h.rebase(0x0159_7f70);
        cdecl(h, DEBUG_LOAD_ALL, &[slot, buf, 0x6900])
    }

    /// The frame `SetUpDefaultSaveFolder` needs: the documents folder at
    /// `ebp - 0x1004`, then the three paths at `ebp - 0xc04`, `- 0x404`
    /// and `- 0x804` (0x400 bytes each).
    pub const FOLDER_FRAME: u32 = 0x1004;

    /// Whether `path` is a directory, or else could be made one:
    /// `GetFileAttributesW` (not -1, bit 0x10), else
    /// `CreateDirectoryW(path, 0)`'s answer.
    fn ensure_dir(h: &mut impl Host, path: u32) -> u32 {
        let attr = import(h, GET_FILE_ATTRIBUTES_IAT, Conv::Stdcall, &[path]);
        if attr != u32::MAX && attr & 0x10 != 0 {
            return 1;
        }
        import(h, CREATE_DIRECTORY_IAT, Conv::Stdcall, &[path, 0])
    }

    /// `SetUpDefaultSaveFolder()` (cdecl): once (`g_SaveFolderSet`): the
    /// documents folder (`SHGetFolderPathW(0, CSIDL_PERSONAL, 0, 0, ebp -
    /// 0x1004)`, nothing on failure), then the three folder names joined
    /// on in turn (`wsprintfW(next, "%s\\%s", prev, name)`), each made a
    /// directory ([`ensure_dir`]; stop when one fails); the last copied
    /// to `g_SaveFolder` (`wsprintfW`) and the flag set.
    pub fn set_up_default_save_folder(h: &mut impl Host, ebp: u32) {
        if rd(h, FOLDER_SET) != 0 {
            return;
        }
        let docs = ebp.wrapping_sub(FOLDER_FRAME);
        let hr = import(
            h,
            SH_GET_FOLDER_PATH_IAT,
            Conv::Stdcall,
            &[0, 5, 0, 0, docs],
        );
        if hr.cast_signed() < 0 {
            return;
        }
        let join = h.rebase(JOIN_FORMAT);
        let mut prev = docs;
        for (name, off) in FOLDER_NAMES.into_iter().zip([0xc04u32, 0x404, 0x804]) {
            let path = ebp.wrapping_sub(off);
            let name = rd(h, name);
            import(h, WSPRINTFW_IAT, Conv::Cdecl, &[path, join, prev, name]);
            if ensure_dir(h, path) == 0 {
                return;
            }
            prev = path;
        }
        let dir = h.rebase(SAVE_DIR);
        let fmt = h.rebase(DIR_FORMAT);
        import(h, WSPRINTFW_IAT, Conv::Cdecl, &[dir, fmt, prev]);
        wr(h, FOLDER_SET, 1);
    }

    /// The frame `SaveDataManager_initSaveDataSys` needs: the file name at
    /// `ebp - 0x84` (0x80 bytes).
    pub const INIT_FRAME: u32 = 0x84;

    /// `SaveDataManager_initSaveDataSys(this; buf, size)` (thiscall, `ret
    /// 8`): the stream prefix formatted into `0x00ce81b0`, the save file
    /// name built in the zeroed frame (`sprintf`) and `strncpy`'d to
    /// `+0x310` (0x40 bytes), the debug buffer (`[0x00ce81f0]`) a new
    /// block of `size`, `+0x350` = `buf`, `+0x354` = `size`, the dialog
    /// state 0. Returns 0.
    pub fn init_save_data_sys(h: &mut impl Host, ebp: u32, this: u32, buf: u32, size: u32) -> u32 {
        let name = ebp.wrapping_sub(INIT_FRAME);
        cdecl(h, MEMSET, &[name, 0, 0x80]);
        let prefix = cdecl(h, STREAM_PREFIX, &[]);
        let path = h.rebase(PREFIX_PATH);
        let fmt = h.rebase(PREFIX_FORMAT);
        import(h, SPRINTF_IAT, Conv::Cdecl, &[path, fmt, prefix]);
        let fmt = h.rebase(PATH_FORMAT);
        let (a1, a2) = (h.rebase(PATH_ARG1), h.rebase(PATH_ARG2));
        import(h, SPRINTF_IAT, Conv::Cdecl, &[name, fmt, path, a1, a2]);
        import(
            h,
            STRNCPY_IAT,
            Conv::Cdecl,
            &[this.wrapping_add(0x310), name, 0x40],
        );
        let block = cdecl(h, NEW_ARRAY, &[size]);
        wr(h, SAVE_BUFFER, block);
        h.write(this.wrapping_add(0x350), buf);
        h.write(this.wrapping_add(0x354), size);
        dialog_word(h, 8, 0);
        0
    }

    /// `0x006f2630(this)` (`ecx`): the manager's `+0x3ac` = 0, `+0x3a9`
    /// = 0x100 (u16), `+0x3a8` = 0; then for each of the 200 bytes at
    /// `+0x3bc` that is set, the 32-byte record at `+0x370` zeroed with
    /// its id (`+6`, u16) the index, and stored over the first entry of
    /// the table (`+0x368`, `+0x3a0` entries, signed) with that id
    /// (whose `+0x18` byte becomes 2), or appended.
    pub fn rebuild_table(h: &mut impl Host, this: u32) {
        h.write(this.wrapping_add(0x3ac), 0u32);
        h.write(this.wrapping_add(0x3a9), 0x100u16);
        h.write(this.wrapping_add(0x3a8), 0u8);
        let rec = this.wrapping_add(0x370);
        for i in 0..200u32 {
            if h.read::<u8>(this.wrapping_add(0x3bc + i)) == 0 {
                continue;
            }
            for k in 0..8 {
                h.write(rec.wrapping_add(4 * k), 0u32);
            }
            h.write(rec.wrapping_add(6), u16::try_from(i).unwrap_or_default());
            let id = u32::from(h.read::<u16>(this.wrapping_add(0x376)));
            let n: u32 = h.read(this.wrapping_add(0x3a0));
            let table: u32 = h.read(this.wrapping_add(0x368));
            let mut j = 0u32;
            let mut found = None;
            while j.cast_signed() < n.cast_signed() {
                let e = table.wrapping_add(j.wrapping_mul(0x20));
                if u32::from(h.read::<u16>(e.wrapping_add(6))) == id {
                    found = Some(j);
                    break;
                }
                j = j.wrapping_add(1);
            }
            let slot = match found {
                Some(j) => j,
                None => {
                    h.write(this.wrapping_add(0x3a0), n.wrapping_add(1));
                    n
                }
            };
            let table: u32 = h.read(this.wrapping_add(0x368));
            let dst = table.wrapping_add(slot.wrapping_mul(0x20));
            for k in 0..8 {
                let w: u32 = h.read(rec.wrapping_add(4 * k));
                h.write(dst.wrapping_add(4 * k), w);
            }
            if found.is_some() {
                let table: u32 = h.read(this.wrapping_add(0x368));
                h.write(
                    table
                        .wrapping_add(slot.wrapping_mul(0x20))
                        .wrapping_add(0x18),
                    2u8,
                );
            }
        }
    }

    /// `0x006494d0` / `0x00649540(this; src, _)` (thiscall, `ret 8`): a
    /// `std::string` (`+0x10` size, `+0x14` capacity) move-constructed
    /// from `src` (a short one copied with `memmove(this, src, size +
    /// 1)`, a long one's pointer taken), `src` left empty, then the word
    /// `+0x18` copied. Returns `this`.
    pub fn string_move(h: &mut impl Host, this: u32, src: u32) -> u32 {
        h.write(this.wrapping_add(0x14), 0xfu32);
        h.write(this.wrapping_add(0x10), 0u32);
        h.write(this, 0u8);
        if h.read::<u32>(src.wrapping_add(0x14)) < 0x10 {
            let n = h.read::<u32>(src.wrapping_add(0x10)).wrapping_add(1);
            if n != 0 {
                import(h, MEMMOVE_IAT, Conv::Cdecl, &[this, src, n]);
            }
        } else {
            let p: u32 = h.read(src);
            h.write(this, p);
            h.write(src, 0u32);
        }
        for off in [0x10, 0x14] {
            let w: u32 = h.read(src.wrapping_add(off));
            h.write(this.wrapping_add(off), w);
        }
        h.write(src.wrapping_add(0x14), 0xfu32);
        h.write(src.wrapping_add(0x10), 0u32);
        h.write(src, 0u8);
        let w: u32 = h.read(src.wrapping_add(0x18));
        h.write(this.wrapping_add(0x18), w);
        this
    }

    /// `EternalCalm_FileSize_0067a9f0(name)` (cdecl): `sceOpen(name, 1)`;
    /// open: `sceFileSize` rounded up to 16 (a signed round of `size +
    /// 15` toward zero), `sceClose`; else `fprintf(stderr, msg, name)` and
    /// -1.
    pub fn calm_file_size(h: &mut impl Host, name: u32) -> u32 {
        let f = cdecl(h, SCE_OPEN, &[name, 1]);
        if f == 0 {
            let iob = import(h, IOB_FUNC_IAT, Conv::Cdecl, &[]);
            let msg = h.rebase(CALM_MESSAGE);
            import(
                h,
                FPRINTF_IAT,
                Conv::Cdecl,
                &[iob.wrapping_add(0x40), msg, name],
            );
            return u32::MAX;
        }
        let x = cdecl(h, SCE_FILE_SIZE, &[f]).wrapping_add(15);
        let size = x.wrapping_add(if x.cast_signed() < 0 { 15 } else { 0 }) & !0xf;
        cdecl(h, SCE_CLOSE, &[f]);
        size
    }

    /// `getEternalCalmMgrpSize(map)` / `getEternalCalmSceneSize(map)`
    /// (cdecl): -1 outside the 7 maps from `g_EternalCalmFirstMap`; else
    /// the cached size (`sizes[i]`), measured once
    /// ([`calm_file_size`] of `names[i]`) while negative.
    pub fn calm_size(h: &mut impl Host, sizes: u32, names: u32, map: u32) -> u32 {
        let i = map.wrapping_sub(rd(h, CALM_FIRST)).cast_signed();
        if !(0..7).contains(&i) {
            return u32::MAX;
        }
        let i = i.cast_unsigned();
        let at = h.rebase(sizes).wrapping_add(4 * i);
        if h.read::<u32>(at).cast_signed() < 0 {
            let name: u32 = h.read(h.rebase(names).wrapping_add(4 * i));
            let size = cdecl(h, CALM_FILE_SIZE, &[name]);
            h.write(at, size);
        }
        h.read(at)
    }

    /// `ShaderParams_FindByName(params; name)`.
    fn find_param(h: &mut impl Host, params: u32, name: u32) -> u32 {
        let name = h.rebase(name);
        h.call_sym(FIND_PARAM, Conv::Thiscall(params), &[name])
    }

    /// A water-wave controller (`[c]` the shader parameters, `[c + 4]`
    /// the buffer): parameter `name` set from the value at `value` when
    /// the parameters have it.
    fn set_param(h: &mut impl Host, c: u32, name: u32, value: u32) {
        let params: u32 = h.read(c);
        let p = find_param(h, params, name);
        if p != 0 {
            let buf: u32 = h.read(c.wrapping_add(4));
            h.call_sym(SET_VALUE, Conv::Thiscall(buf), &[p, value]);
        }
    }

    /// `WaterWaveController_setBuoyancyFix` / `setDraughtOffset` /
    /// `setFoamStrength` / `setMultiTexBlendFactor(c, v)` (stdcall, `ret
    /// 8`): parameter `name` set from the caller's `v` (by its slot,
    /// `slots + 4`; `slots` the address of `c`'s).
    pub fn set_one(h: &mut impl Host, name: u32, c: u32, slots: u32) {
        set_param(h, c, name, slots.wrapping_add(4));
    }

    /// `WaterWaveController_setCircleWaveStrength` /
    /// `setSineWaveStrength(c, a, b)` (stdcall, `ret 0xc`): the two
    /// parameters `names` set from the vectors `a` and `b`, each when not
    /// null.
    pub fn set_two(h: &mut impl Host, names: [u32; 2], c: u32, a: u32, b: u32) {
        for (name, v) in names.into_iter().zip([a, b]) {
            if v != 0 {
                set_param(h, c, name, v);
            }
        }
    }

    /// `WaterWaveController_setScrollSpeed(c, x0, y0, x1, y1)` (stdcall,
    /// `ret 0x14`): for each group of four names (first `x0, y0`, then
    /// `x1, y1`), all four looked up first; when all are found they are
    /// set from the slots of `x, y, x, y` in that order. The original keeps
    /// three lookups in its own argument slots (`slots`, `slots + 4`,
    /// `slots + 8`, the first group's results only in the second's
    /// lookup); the port writes them there alike.
    pub fn set_scroll_speed(h: &mut impl Host, c: u32, slots: u32) {
        let x0 = slots.wrapping_add(4);
        let y0 = slots.wrapping_add(8);
        let x1 = slots.wrapping_add(12);
        let y1 = slots.wrapping_add(16);
        let params: u32 = h.read(c);
        let mut found = [0u32; 4];
        for (k, name) in SCROLL_A.into_iter().enumerate() {
            found[k] = find_param(h, params, name);
            if k == 0 {
                // the first result is kept in the `c` slot
                h.write(slots, found[0]);
            }
        }
        if found.iter().all(|&p| p != 0) {
            for (p, v) in found.into_iter().zip([x0, y0, x0, y0]) {
                let buf: u32 = h.read(c.wrapping_add(4));
                h.call_sym(SET_VALUE, Conv::Thiscall(buf), &[p, v]);
            }
        }
        let params: u32 = h.read(c);
        for (k, name) in SCROLL_B.into_iter().enumerate() {
            found[k] = find_param(h, params, name);
            // the first, third and fourth results are kept in the `x0`,
            // `y0` and `c` slots
            match k {
                0 => h.write(x0, found[0]),
                2 => h.write(y0, found[2]),
                3 => h.write(slots, found[3]),
                _ => {}
            }
        }
        if found.iter().all(|&p| p != 0) {
            for (p, v) in found.into_iter().zip([x1, y1, x1, y1]) {
                let buf: u32 = h.read(c.wrapping_add(4));
                h.call_sym(SET_VALUE, Conv::Thiscall(buf), &[p, v]);
            }
        }
    }
}
