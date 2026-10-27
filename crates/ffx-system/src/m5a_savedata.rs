//! M5 `system-a`: the PC save/load screen's state machine
//! (`SaveDataMachine_Update`, its states' handlers and file access), the
//! input and trophy glue it uses, and `SaveData_Update`, which runs it
//! every frame.
//!
//! The machine's state is in globals (`0x00ce72e4..`): the state number,
//! a frame timer, the list's top row and cursor, the file count, the
//! yes/no choice, the save buffer and its size, and the file table (200
//! rows of `{file number, date, time}` at `0x00ce7318`, row 0 the
//! auto-save when its number is not negative). Every callee stays a call
//! to the original (`Host::call_sym`, also those this list ports: in the
//! game the call reaches the port through its hook); the DLL imports
//! (`wsprintfW`, `_wfopen`, `fread`, `fwrite`, `fclose`, `GetTickCount`)
//! are called through `FFX.exe`'s import table.
//!
//! Each bridge's doc names its original and the calling convention read
//! from the disassembly; `eax` is returned wherever the original defines
//! it, also where Ghidra says `void`.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// The machine's state (`SaveDataMachine_Update`'s switch).
pub const STATE: u32 = 0x00ce_72e8;
/// A frame timer the waiting states count down.
pub const TIMER: u32 = 0x00ce_72e4;
/// The list's first visible row.
pub const TOP: u32 = 0x00ce_72ec;
/// The cursor's row on screen (0..4).
pub const CURSOR: u32 = 0x00ce_72f0;
/// How many save files were found.
pub const COUNT: u32 = 0x00ce_72f4;
/// The list's length.
pub const LIST_LEN: u32 = 0x00ce_72f8;
/// Whether every file number is taken.
pub const FULL: u32 = 0x00ce_72fc;
/// The yes/no dialog's choice (1 yes).
pub const CHOICE: u32 = 0x00ce_7300;
/// The save buffer's size.
pub const SIZE: u32 = 0x00ce_7304;
/// The save buffer.
pub const BUFFER: u32 = 0x00ce_7308;
/// `GetTickCount` when the current wait started.
pub const TICK_BASE: u32 = 0x00ce_730c;
/// Cleared when the machine finishes.
pub const BUSY: u32 = 0x00ce_7310;
/// The file table: 200 rows of 12 bytes (`{file number, date, time}`).
pub const FILES: u32 = 0x00ce_7318;
/// Whether each file number exists (200 words, -1 for none).
pub const EXISTS: u32 = 0x00ce_7c78;
/// The save/load menu's `IggyMenu`.
pub const MENU: u32 = 0x00ce_b248;
/// The loaded game's flag word the merge ORs in (two copies).
pub const MERGED_FLAGS: [u32; 2] = [0x0132_5ac4, 0x0132_5b8c];
/// The loaded file's rename flag (`isNeedRenamePlayer`'s answer).
pub const RENAME: u32 = 0x0113_3350;
/// The save folder (wide string) and the user's id, for the file name.
pub const SAVE_DIR: u32 = 0x00ce_6ee0;
/// The user id word.
pub const USER_ID: u32 = 0x00c3_577c;
/// `L"%s/ffx_%03d"`-style file-name format (`0x00b4b680`).
pub const NAME_FORMAT: u32 = 0x00b4_b680;
/// `L"rb"` (`0x00b7ca84`).
pub const MODE_READ: u32 = 0x00b7_ca84;
/// `L"wb"` (`0x00b7ca8c`).
pub const MODE_WRITE: u32 = 0x00b7_ca8c;

/// Import slots.
pub const WSPRINTFW_IAT: u32 = 0x00b0_c674;
pub const WFOPEN_IAT: u32 = 0x00b0_c47c;
pub const FREAD_IAT: u32 = 0x00b0_c3f4;
pub const FWRITE_IAT: u32 = 0x00b0_c428;
pub const FCLOSE_IAT: u32 = 0x00b0_c3ec;
pub const GET_TICK_COUNT_IAT: u32 = 0x00b0_c0a0;

/// Originals the bridges call.
pub const TROPHY_MANAGER: u32 = 0x0067_ac90;
pub const TROPHY_FLAG: u32 = 0x0067_ac60;
pub const TROPHY_UPDATE_BODY: u32 = 0x0065_f1e0;
pub const TROPHY_POST_UPDATE: u32 = 0x0067_ad00;
pub const TROPHY_MANAGER_PTR: u32 = 0x00cc_b9d8;
pub const TROPHY_MANAGER_UPDATE: u32 = 0x0064_1680;
pub const CONTROLLER_MANAGER: u32 = 0x0068_dc10;
pub const CONTROLLER_INFO: u32 = 0x0068_ccd0;
pub const USER_FREE: u32 = 0x006f_b8b0;
pub const SAVE_MANAGER: u32 = 0x006f_2070;
pub const SAVE_TAKE_BYTE364: u32 = 0x006f_2120;
pub const SAVE_SET_BYTE364: u32 = 0x006f_1dc0;
pub const SAVE_SET_MEMBER_C: u32 = 0x006f_24f0;
pub const SAVE_SET_RESULT: u32 = 0x006f_1aa0;
pub const SAVE_REF_BUFFER: u32 = 0x006f_1ed0;
pub const SAVE_CHECK_CRC: u32 = 0x0064_7d70;
pub const MEMSET: u32 = 0x0094_964c;
pub const SHOW_POPUP: u32 = 0x0070_2e90;
pub const SHOW_POPUP2: u32 = 0x0070_2d20;
pub const SHOW_POPUP4: u32 = 0x0070_2ef0;
pub const SET_FOCUS: u32 = 0x0070_2cd0;
pub const MENU_DISABLE: u32 = 0x0070_2a70;
pub const MENU_OPEN: u32 = 0x0070_2a80;
pub const MENU_RELEASE: u32 = 0x0070_2fa0;
pub const IGGY_CALL: u32 = 0x00a2_8ba0;
pub const IGGY_SET_VALUE: u32 = 0x00a2_8d30;
pub const IGGY_SET_STRING: u32 = 0x00a2_8d70;
pub const IGGY_DTOR: u32 = 0x00a2_8a40;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
pub const OPERATOR_DELETE_ARRAY: u32 = 0x0063_0540;
pub const INPUT_MANAGER: u32 = 0x0063_1300;
pub const INPUT_STICK_INFO: u32 = 0x0063_1310;
pub const INPUT_IS_HOLD: u32 = 0x0063_1340;
pub const INPUT_IS_PRESS: u32 = 0x0063_1360;
pub const MENU_INPUT: u32 = 0x006e_f320;
pub const MOVE_CURSOR: u32 = 0x006e_f630;
pub const CURSOR_DOWN: u32 = 0x006f_0290;
pub const CLICK: u32 = 0x0088_6ba0;
pub const SE_PLAY: u32 = 0x0088_6e80;
pub const SET_DIALOG_STATE: u32 = 0x0064_8620;
pub const SAVE_LOAD_SUCCEED: u32 = 0x0064_8540;
pub const ENABLE_SAVE: u32 = 0x0064_7d40;
pub const SET_SAVE_MODE: u32 = 0x0064_7d50;
pub const VIDEO_INFO: u32 = 0x0064_3aa0;
pub const LOC_MANAGER: u32 = 0x006d_b070;
pub const LOC_TEXT: u32 = 0x006d_b1f0;
pub const GET_LOC: u32 = 0x0064_7f30;
pub const SCAN_FILES: u32 = 0x006f_0a70;
pub const SORT_FILES: u32 = 0x006f_0fb0;
pub const READ_HEADERS: u32 = 0x006f_0c60;
pub const NEED_RENAME: u32 = 0x0078_7370;
pub const MACHINE_UPDATE: u32 = 0x006f_0950;
pub const MACHINE_UPDATE_THUNK: u32 = 0x006f_2740;
pub const TICK_SECONDS: u32 = 0x006f_0670;
pub const PLAYER_POS: u32 = 0x0082_d880;
pub const PLAYER_POS_OUT: u32 = 0x00cc_b9a8;
pub const TROPHY_INTERVAL: u32 = 0x00b9_21d8;
pub const TICK_WRAP: u32 = 0x00b1_1f14;
pub const TICK_SCALE: u32 = 0x00b4_b6e8;
pub const GAME_FLAGS_OUT: u32 = 0x00c5_26d0;
pub const DIALOG_OBJ: u32 = 0x00ce_81f4;

/// The `IggyMenu_callIggyAS3External` calls `(0, name, 0, 0, 1)` on the
/// save menu: the name strings by function.
pub const AS3_YES_NO: u32 = 0x00b4_cec8; // 0x00702a10
pub const AS3_ENABLE: u32 = 0x00b4_ceec; // 0x00702a30
pub const AS3_DISABLE: u32 = 0x00b4_cefc; // 0x00702a50
pub const AS3_CLEAR: u32 = 0x00b4_ce88; // 0x00702f80
pub const AS3_SET_ROW: u32 = 0x00b4_cedc; // 0x00702be0, 0x00702d90
pub const YES_NO: u32 = 0x0070_2a10;
pub const ENABLE: u32 = 0x0070_2a30;
pub const DISABLE: u32 = 0x0070_2a50;
pub const CLEAR: u32 = 0x0070_2f80;

/// The yes/no handlers (`0x006efc50`, `0x006effa0`, `0x006f05b0`): the
/// state for yes, its popup, the state for no.
#[derive(Debug, Clone, Copy)]
pub struct YesNo {
    pub yes: u32,
    pub popup: u32,
    pub no: u32,
}

/// `0x006efc50` (save over a file).
pub const YES_NO_OVERWRITE: YesNo = YesNo {
    yes: 0x18,
    popup: 0x1fc,
    no: 0x16,
};
/// `0x006effa0` (load a file).
pub const YES_NO_LOAD: YesNo = YesNo {
    yes: 0x0e,
    popup: 0x1fc,
    no: 0x0c,
};
/// `0x006f05b0` (save to a file).
pub const YES_NO_SAVE: YesNo = YesNo {
    yes: 0x04,
    popup: 0x1fb,
    no: 0x02,
};

/// The list handlers that open a yes/no on confirm (`0x006efe20`,
/// `0x006f0140`): the state without files, the state with the dialog,
/// the state on cancel.
#[derive(Debug, Clone, Copy)]
pub struct ListInput {
    pub empty: u32,
    pub ask: u32,
    pub cancel: u32,
}

/// `0x006efe20`.
pub const LIST_OVERWRITE: ListInput = ListInput {
    empty: 0x1b,
    ask: 0x17,
    cancel: 0x1b,
};
/// `0x006f0140`.
pub const LIST_LOAD: ListInput = ListInput {
    empty: 0x11,
    ask: 0x0d,
    cancel: 0x11,
};

/// The list builders (`0x006eff00`, `SaveDataMachine_BuildList`,
/// `0x006f0800`): the popup, whether an empty list shows popup 0x20b,
/// the next state.
#[derive(Debug, Clone, Copy)]
pub struct Build {
    pub popup: u32,
    pub empty_popup: bool,
    pub state: u32,
}

/// `0x006eff00`.
pub const BUILD_OVERWRITE: Build = Build {
    popup: 0x1fd,
    empty_popup: true,
    state: 0x16,
};
/// `SaveDataMachine_BuildList` (`0x006f0220`).
pub const BUILD_LOAD: Build = Build {
    popup: 0x1fd,
    empty_popup: true,
    state: 0x0c,
};
/// `0x006f0800`.
pub const BUILD_SAVE: Build = Build {
    popup: 0x1fe,
    empty_popup: false,
    state: 0x02,
};

fn rd(h: &impl Host, sym: u32) -> u32 {
    h.read(h.rebase(sym))
}

fn wr(h: &mut impl Host, sym: u32, v: u32) {
    let at = h.rebase(sym);
    h.write(at, v);
}

fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
    let f: u32 = h.read(h.rebase(slot));
    h.call(f, conv, args)
}

fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
    h.call_sym(sym, Conv::Cdecl, args)
}

/// The originals over game memory and the original callees.
pub mod bridge {
    #[allow(clippy::wildcard_imports)] // the module's addresses and helpers
    use super::*;

    /// `0x0067ac60` (thiscall `(this)`, 9 bytes): `this` when byte
    /// `+1` is set, else 0. The original leaves `eax` alone in the first
    /// case; its only caller (`TrophyGetError`, `0x00649001`) enters with
    /// `eax` = `ecx`, so the port returns `this`.
    pub fn trophy_flag(h: &impl Host, this: u32) -> u32 {
        if h.read::<u8>(this.wrapping_add(1)) != 0 {
            this
        } else {
            0
        }
    }

    /// `TrophyGetError()` (cdecl): 0 without a trophy manager, else
    /// [`trophy_flag`] of it.
    pub fn trophy_get_error(h: &mut impl Host) -> u32 {
        if cdecl(h, TROPHY_MANAGER, &[]) == 0 {
            return 0;
        }
        // the tail jump to 0x0067ac60 with `eax` = `ecx` = the manager
        let m = cdecl(h, TROPHY_MANAGER, &[]);
        trophy_flag(h, m)
    }

    /// `TrophyManager_Update()` (cdecl): the empty `0x0065f1e0` on the
    /// manager at `0x00ccb9d8` (a tail jump; `eax` untouched, so the
    /// shim returns nothing).
    pub fn trophy_manager_update(h: &mut impl Host) {
        let m = rd(h, TROPHY_MANAGER_PTR);
        h.call_sym(TROPHY_UPDATE_BODY, Conv::Thiscall(m), &[]);
    }

    /// `Virtuos_InputManager_setInputInfo(this)` (thiscall): the
    /// controller info (`AsyncControllerTaskManager_GetControllerInfo`
    /// into the frame at `ebp - 0x18`: a word, then 4 floats) becomes
    /// the current one (`+0`, the old one to `+4`, the floats to `+0x8c`
    /// through the x87). Returns the info word (`eax`).
    pub fn set_input_info(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let info = ebp.wrapping_sub(0x18);
        let floats = ebp.wrapping_sub(0x14);
        let m = cdecl(h, CONTROLLER_MANAGER, &[]);
        h.call_sym(CONTROLLER_INFO, Conv::Thiscall(m), &[info, floats]);
        let old: u32 = h.read(this);
        for (i, off) in [(0u32, 0x8cu32), (1, 0x90)] {
            let v: u32 = h.read(floats.wrapping_add(4 * i));
            h.write(this.wrapping_add(off), x87::store(x87::load(v)));
        }
        h.write(this.wrapping_add(4), old);
        let word: u32 = h.read(info);
        let v: u32 = h.read(floats.wrapping_add(8));
        h.write(this.wrapping_add(0x94), x87::store(x87::load(v)));
        h.write(this, word);
        let v: u32 = h.read(floats.wrapping_add(12));
        h.write(this.wrapping_add(0x98), x87::store(x87::load(v)));
        word
    }

    /// `user_free_thunk(p)` (cdecl): a tail jump to `user_free(p)`.
    pub fn user_free_thunk(h: &mut impl Host, p: u32) -> u32 {
        cdecl(h, USER_FREE, &[p])
    }

    /// `0x00648010()` (cdecl): `SaveDataManager_takeByte364` of the
    /// manager, zero-extended.
    pub fn save_take_byte364(h: &mut impl Host) -> u32 {
        let m = cdecl(h, SAVE_MANAGER, &[]);
        h.call_sym(SAVE_TAKE_BYTE364, Conv::Thiscall(m), &[]) & 0xff
    }

    /// `0x006f0670()` (cdecl, `ST0`): the seconds since the wait began,
    /// `(GetTickCount() - [0x00ce730c])` as an unsigned count rounded to
    /// `f32`, divided by the double at `0x00b4b6e8` (to `f32`), clamped
    /// to 0..1.
    pub fn wait_fraction(h: &mut impl Host) -> f32 {
        let tick = import(h, GET_TICK_COUNT_IAT, Conv::Stdcall, &[]);
        let d = tick.wrapping_sub(rd(h, TICK_BASE)).cast_signed();
        let mut x = f64::from(d);
        if d < 0 {
            let wrap: f32 = h.read(h.rebase(TICK_WRAP));
            x += f64::from(wrap);
        }
        let x = f32::from_bits(x87::store(x));
        let scale: f64 = h.read(h.rebase(TICK_SCALE));
        let y = f32::from_bits(x87::store(f64::from(x) / scale));
        // the x87 compares: below 0 gives 0.0, above 1 gives 1.0 (y is
        // never NaN: a count over the positive scale)
        y.clamp(0.0, 1.0)
    }

    /// The save menu's `IggyMenu_callIggyAS3External(0, name, 0, 0, 1)`
    /// (`0x00702a10`, `0x00702a30`, `0x00702a50`, `0x00702f80`; cdecl).
    pub fn as3_call(h: &mut impl Host, name: u32) -> u32 {
        let m = rd(h, MENU);
        let name = h.rebase(name);
        h.call_sym(IGGY_CALL, Conv::Thiscall(m), &[0, name, 0, 0, 1])
    }

    /// The timer's countdown: `[TIMER]` decremented; `Some(new value)`
    /// while it was positive.
    fn tick_timer(h: &mut impl Host) -> Option<u32> {
        let t = rd(h, TIMER);
        wr(h, TIMER, t.wrapping_sub(1));
        (t.cast_signed() > 0).then_some(t.wrapping_sub(1))
    }

    /// `SaveDataMachine_CheckFile()` (cdecl; state 15): after the
    /// timer, the menu enabled and the loaded buffer's CRC checked: a bad
    /// one clears the buffer and shows popup 0x208, a file that wants the
    /// player renamed popup 0x209 (both state 0x10), else state 0x11.
    pub fn check_file(h: &mut impl Host) -> u32 {
        if let Some(t) = tick_timer(h) {
            return t;
        }
        cdecl(h, ENABLE, &[]);
        let crc = cdecl(h, SAVE_CHECK_CRC, &[]);
        if crc == 0 {
            let (buf, size) = (rd(h, BUFFER), rd(h, SIZE));
            cdecl(h, MEMSET, &[buf, 0, size]);
            let r = cdecl(h, SHOW_POPUP, &[0x208]);
            wr(h, STATE, 0x10);
            return r;
        }
        if rd(h, RENAME) != 0 {
            let r = cdecl(h, SHOW_POPUP, &[0x209]);
            wr(h, STATE, 0x10);
            return r;
        }
        wr(h, STATE, 0x11);
        crc
    }

    /// `inputGetCurrDeviceStickInfo()` (cdecl): the input manager's
    /// stick info (a tail jump).
    pub fn stick_info(h: &mut impl Host) -> u32 {
        let m = cdecl(h, INPUT_MANAGER, &[]);
        h.call_sym(INPUT_STICK_INFO, Conv::Thiscall(m), &[])
    }

    /// `inputIsHoldButton(b)` / `inputIsPressButton(b)` (cdecl): the
    /// input manager's answer (`method`), zero-extended.
    pub fn input_button(h: &mut impl Host, method: u32, b: u32) -> u32 {
        let m = cdecl(h, INPUT_MANAGER, &[]);
        h.call_sym(method, Conv::Thiscall(m), &[b]) & 0xff
    }

    /// A yes/no dialog's input (`0x006efc50`, `0x006effa0`,
    /// `0x006f05b0`; cdecl): left/right (3, 4) flip the choice, confirm
    /// (7) on yes goes to `yes` with its popup, cancel (8) or confirm on
    /// no to `no`. Returns the last callee's `eax`, or the input code.
    pub fn yes_no(h: &mut impl Host, d: YesNo) -> u32 {
        let code = cdecl(h, MENU_INPUT, &[]);
        let no = match code {
            3 | 4 => {
                cdecl(h, CLICK, &[1]);
                let c = 1u32.wrapping_sub(rd(h, CHOICE));
                wr(h, CHOICE, c);
                return cdecl(h, YES_NO, &[]);
            }
            7 => rd(h, CHOICE) != 1,
            8 => true,
            _ => return code,
        };
        if no {
            cdecl(h, CLICK, &[4]);
            wr(h, STATE, d.no);
        } else {
            cdecl(h, CLICK, &[1]);
            wr(h, STATE, d.yes);
            cdecl(h, SHOW_POPUP, &[d.popup]);
        }
        cdecl(h, DISABLE, &[])
    }

    /// `SaveDataEnableSave()` (cdecl): `SetByte364_006f1dc0` on the save
    /// manager (a tail jump to a callee that leaves `eax` alone). Returns
    /// the manager.
    pub fn enable_save(h: &mut impl Host) -> u32 {
        let m = cdecl(h, SAVE_MANAGER, &[]);
        // the callee leaves `eax` alone
        h.call_sym(SAVE_SET_BYTE364, Conv::Thiscall(m), &[]);
        m
    }

    /// `0x00647d50(v)` (cdecl): `SetMemberC_006f24f0(manager; v)`.
    pub fn set_save_mode(h: &mut impl Host, v: u32) -> u32 {
        let m = cdecl(h, SAVE_MANAGER, &[]);
        h.call_sym(SAVE_SET_MEMBER_C, Conv::Thiscall(m), &[v])
    }

    /// `0x006f1aa0(this; v)` (thiscall, `ret 4`): the dialog object's
    /// state (`[0x00ce81f4] + 8`) follows this one (`+8`: 3 -> 4, 1 ->
    /// 2, 5 -> 6), `+0x3a4` = `v`. Returns `v`.
    pub fn set_result(h: &mut impl Host, this: u32, v: u32) -> u32 {
        let next = match h.read::<u32>(this.wrapping_add(8)) {
            3 => Some(4u32),
            1 => Some(2),
            5 => Some(6),
            _ => None,
        };
        if let Some(n) = next {
            let d = rd(h, DIALOG_OBJ);
            h.write(d.wrapping_add(8), n);
        }
        h.write(this.wrapping_add(0x3a4), v);
        v
    }

    /// `SaveDataSaveLoadSucceed(kind)` (cdecl): the manager's result
    /// (`0x006f1aa0`); after a load (3) with no merged flags, the game's
    /// flag word (`+0xe8`, sign-extended, to `0x00c526d0`) and the video
    /// info from `+0xea` - 1 (`graphicSetTextureVideoInfo(lo, hi)`).
    /// Returns the last callee's `eax`.
    pub fn save_load_succeed(h: &mut impl Host, kind: u32) -> u32 {
        let m = cdecl(h, SAVE_MANAGER, &[]);
        let r = h.call_sym(SAVE_SET_RESULT, Conv::Thiscall(m), &[kind]);
        if rd(h, MERGED_FLAGS[0]) != 0 || kind != 3 {
            return r;
        }
        let m = cdecl(h, SAVE_MANAGER, &[]);
        let buf = h.call_sym(SAVE_REF_BUFFER, Conv::Thiscall(m), &[]);
        let flags = i32::from(h.read::<i16>(buf.wrapping_add(0xe8)));
        wr(h, GAME_FLAGS_OUT, flags.cast_unsigned());
        let info = i32::from(
            h.read::<u16>(buf.wrapping_add(0xea))
                .wrapping_sub(1)
                .cast_signed(),
        );
        let hi = (info >> 8).cast_unsigned();
        let lo = info.cast_unsigned() & 0xff;
        cdecl(h, VIDEO_INFO, &[lo, hi])
    }

    /// `0x00702fa0()` (cdecl): the save menu, when set, destroyed
    /// (`IggyMenu_dtor_IggyMenu`, `operator delete`) and cleared.
    pub fn release_menu(h: &mut impl Host) {
        let m = rd(h, MENU);
        if m != 0 {
            h.call_sym(IGGY_DTOR, Conv::Thiscall(m), &[]);
            cdecl(h, OPERATOR_DELETE, &[m]);
            wr(h, MENU, 0);
        }
    }

    /// `0x006efcd0()` / `0x006efde0()` (cdecl): the finished save over a
    /// file (`mode` 1) or into a new one (0): menu off and released,
    /// succeed(3), dialog state 0, save mode, saving enabled; state 0x14.
    /// Returns `SaveDataEnableSave`'s `eax`.
    pub fn save_done(h: &mut impl Host, mode: u32) -> u32 {
        cdecl(h, MENU_DISABLE, &[]);
        cdecl(h, MENU_RELEASE, &[]);
        cdecl(h, SAVE_LOAD_SUCCEED, &[3]);
        cdecl(h, SET_DIALOG_STATE, &[0]);
        cdecl(h, SET_SAVE_MODE, &[mode]);
        let r = cdecl(h, ENABLE_SAVE, &[]);
        wr(h, STATE, 0x14);
        wr(h, BUSY, 0);
        r
    }

    /// `SaveDataMachine_LoadDone()` (cdecl): menu off and released,
    /// succeed(3), dialog state 0; state 0xa. Returns the last callee's
    /// `eax`.
    pub fn load_done(h: &mut impl Host) -> u32 {
        cdecl(h, MENU_DISABLE, &[]);
        cdecl(h, MENU_RELEASE, &[]);
        cdecl(h, SAVE_LOAD_SUCCEED, &[3]);
        let r = cdecl(h, SET_DIALOG_STATE, &[0]);
        wr(h, STATE, 0xa);
        wr(h, BUSY, 0);
        r
    }

    /// `0x006f0630()` (cdecl): the finished save: menu off and released,
    /// the buffer freed (`operator delete[]`), succeed(1), dialog state
    /// 0; state 0. Returns the last callee's `eax`.
    pub fn save_finished(h: &mut impl Host) -> u32 {
        cdecl(h, MENU_DISABLE, &[]);
        cdecl(h, MENU_RELEASE, &[]);
        let buf = rd(h, BUFFER);
        cdecl(h, OPERATOR_DELETE_ARRAY, &[buf]);
        cdecl(h, SAVE_LOAD_SUCCEED, &[1]);
        let r = cdecl(h, SET_DIALOG_STATE, &[0]);
        wr(h, STATE, 0);
        wr(h, BUSY, 0);
        r
    }

    /// The file row `slot` reads (row 0 is the auto-save only when its
    /// number is not negative: otherwise the rows start at 1).
    fn row(h: &impl Host, slot: u32) -> u32 {
        let skip = u32::from(rd(h, FILES).cast_signed() < 0);
        slot.wrapping_add(skip)
    }

    /// The file name of file `number` into `path` (`wsprintfW`).
    fn file_name(h: &mut impl Host, path: u32, number: u32) {
        let fmt = h.rebase(NAME_FORMAT);
        let dir = h.rebase(SAVE_DIR);
        let user = rd(h, USER_ID);
        import(
            h,
            WSPRINTFW_IAT,
            Conv::Cdecl,
            &[path, fmt, dir, user, number],
        );
    }

    /// The frame `SaveDataMachine_MergeFileFlags_006efd10` needs:
    /// `ebp - 0x6d04` (the file's bytes from `ebp - 0x6d04`, the path at
    /// `ebp - 0x404`).
    pub const MERGE_FRAME: u32 = 0x6d04;

    /// `SaveDataMachine_MergeFileFlags_006efd10(slot)` (cdecl; state
    /// 24): the file of row `slot` read into the frame (`fread(ebp -
    /// 0x6d04, 1, [SIZE])`), its flag word at `+0x3d50` `OR`ed into the
    /// buffer's and copied to both [`MERGED_FLAGS`]; timer 30, state
    /// 0x19. Returns `fclose`'s `eax`, or 0 when the file doesn't open.
    ///
    /// junk read: `[ebp-0x2fb4]` (the file's `+0x3d50`) is never written
    /// when the file is shorter; 0 (brief 2026-10-24): the port's frame
    /// is zeroed.
    pub fn merge_file_flags(h: &mut impl Host, ebp: u32, slot: u32) -> u32 {
        let path = ebp.wrapping_sub(0x404);
        let data = ebp.wrapping_sub(MERGE_FRAME);
        let number: u32 = h.read(h.rebase(FILES).wrapping_add(12 * row(h, slot)));
        file_name(h, path, number);
        let mode = h.rebase(MODE_READ);
        let f = import(h, WFOPEN_IAT, Conv::Cdecl, &[path, mode]);
        let mut r = f;
        if f != 0 {
            let size = rd(h, SIZE);
            import(h, FREAD_IAT, Conv::Cdecl, &[data, 1, size, f]);
            let buf = rd(h, BUFFER);
            let flags =
                h.read::<u32>(buf.wrapping_add(0x3d50)) | h.read::<u32>(data.wrapping_add(0x3d50));
            h.write(buf.wrapping_add(0x3d50), flags);
            wr(h, MERGED_FLAGS[0], flags);
            wr(h, MERGED_FLAGS[1], flags);
            r = import(h, FCLOSE_IAT, Conv::Cdecl, &[f]);
        }
        wr(h, TIMER, 0x1e);
        wr(h, STATE, 0x19);
        r
    }

    /// The frame `SaveDataMachine_ReadFile` needs (`ebp - 0x408`).
    pub const READ_FRAME: u32 = 0x408;

    /// `SaveDataMachine_ReadFile(slot)` (cdecl; state 14): the file of
    /// row `slot` read into the save buffer, then `isNeedRenamePlayer`
    /// of its byte `+0xc` to `0x01133350`; timer 30, state 0xf. Returns
    /// that answer, or 0 when the file doesn't open.
    ///
    /// junk read: the original keeps the byte in `[ebp-0x408]` and
    /// passes the whole word, whose upper three bytes are never written;
    /// 0 (brief 2026-10-24).
    pub fn read_file(h: &mut impl Host, ebp: u32, slot: u32) -> u32 {
        let path = ebp.wrapping_sub(0x404);
        let number: u32 = h.read(h.rebase(FILES).wrapping_add(12 * row(h, slot)));
        file_name(h, path, number);
        let mode = h.rebase(MODE_READ);
        let f = import(h, WFOPEN_IAT, Conv::Cdecl, &[path, mode]);
        let mut r = f;
        if f != 0 {
            let (size, buf) = (rd(h, SIZE), rd(h, BUFFER));
            import(h, FREAD_IAT, Conv::Cdecl, &[buf, 1, size, f]);
            let buf = rd(h, BUFFER);
            let b = h.read::<u8>(buf.wrapping_add(0xc));
            let slot_word = ebp.wrapping_sub(0x408);
            h.write(slot_word, b);
            import(h, FCLOSE_IAT, Conv::Cdecl, &[f]);
            let word: u32 = h.read(slot_word);
            r = cdecl(h, NEED_RENAME, &[word]);
            wr(h, RENAME, r);
        }
        wr(h, TIMER, 0x1e);
        wr(h, STATE, 0xf);
        r
    }

    /// The frame `SaveDataMachine_WriteFile` needs (`ebp - 0x404`).
    pub const WRITE_FRAME: u32 = 0x404;

    /// `SaveDataMachine_WriteFile(slot)` (cdecl; state 4): the save
    /// buffer written to the file of row `slot` (one more when the list
    /// is full); row 0 of a list with room takes the first free number
    /// 1..199 (0 when none). State 5, timer 30; returns `fclose`'s `eax`.
    /// A file that doesn't open stores to address 0 (the original's
    /// deliberate crash).
    pub fn write_file(h: &mut impl Host, ebp: u32, slot: u32) -> u32 {
        let path = ebp.wrapping_sub(0x404);
        let mut row = slot;
        if rd(h, FULL) == 1 {
            row = row.wrapping_add(1);
        }
        let number = if row == 0 {
            let exists = h.rebase(EXISTS);
            (1..200u32)
                .find(|&i| h.read::<u32>(exists.wrapping_add(4 * i)).cast_signed() < 0)
                .unwrap_or(0)
        } else {
            h.read(h.rebase(FILES).wrapping_add(12 * row))
        };
        file_name(h, path, number);
        let mode = h.rebase(MODE_WRITE);
        let f = import(h, WFOPEN_IAT, Conv::Cdecl, &[path, mode]);
        if f == 0 {
            h.write(0, 1u8);
            return 0;
        }
        let (size, buf) = (rd(h, SIZE), rd(h, BUFFER));
        import(h, FWRITE_IAT, Conv::Cdecl, &[buf, 1, size, f]);
        let r = import(h, FCLOSE_IAT, Conv::Cdecl, &[f]);
        wr(h, STATE, 5);
        wr(h, TIMER, 0x1e);
        r
    }

    /// The list focus on row `TOP + CURSOR` (`0x00702cd0`).
    fn focus(h: &mut impl Host) -> u32 {
        let row = rd(h, TOP).wrapping_add(rd(h, CURSOR));
        cdecl(h, SET_FOCUS, &[row])
    }

    /// `0x006f0290()` (cdecl): the cursor one row down (the list
    /// scrolls at row 4), unless on the last row; then the focus.
    pub fn cursor_down(h: &mut impl Host) -> u32 {
        let cur = rd(h, CURSOR);
        let top = rd(h, TOP);
        let last = rd(h, LIST_LEN).wrapping_sub(1);
        if cur == 4 {
            if top.wrapping_add(4) != last {
                wr(h, TOP, top.wrapping_add(1));
            }
        } else if top.wrapping_add(cur) != last {
            wr(h, CURSOR, cur.wrapping_add(1));
        }
        focus(h)
    }

    /// The cursor one row up (the list scrolls at row 0), then the focus.
    fn cursor_up(h: &mut impl Host) {
        let cur = rd(h, CURSOR);
        if cur != 0 {
            wr(h, CURSOR, cur.wrapping_sub(1));
        } else {
            let top = rd(h, TOP);
            if top != 0 {
                wr(h, TOP, top.wrapping_sub(1));
            }
        }
        focus(h);
    }

    /// `0x006ef630(code)` (cdecl): up (1), down (2), five up (5) or five
    /// down (6); returns 1 for those, 0 for any other code.
    pub fn move_cursor(h: &mut impl Host, code: u32) -> u32 {
        match code {
            1 => cursor_up(h),
            2 => {
                cdecl(h, CURSOR_DOWN, &[]);
            }
            5 => (0..5).for_each(|_| cursor_up(h)),
            6 => (0..5).for_each(|_| {
                cdecl(h, CURSOR_DOWN, &[]);
            }),
            _ => return 0,
        }
        1
    }

    /// A list's input that asks before going on (`0x006efe20`,
    /// `0x006f0140`; cdecl): cursor moves, confirm (7): without files
    /// back (`empty`, the menu enabled), else the yes/no (`ask`, choice
    /// yes, popup 0x38c); cancel (8): `cancel`. Returns the last callee's
    /// `eax`, or 0.
    pub fn list_input(h: &mut impl Host, d: ListInput) -> u32 {
        let code = cdecl(h, MENU_INPUT, &[]);
        let moved = cdecl(h, MOVE_CURSOR, &[code]);
        if moved != 0 {
            return cdecl(h, CLICK, &[1]);
        }
        match code {
            7 => {
                cdecl(h, CLICK, &[1]);
                if rd(h, COUNT) == 0 {
                    let r = cdecl(h, ENABLE, &[]);
                    wr(h, STATE, d.empty);
                    return r;
                }
                wr(h, CHOICE, 1);
                wr(h, STATE, d.ask);
                cdecl(h, SHOW_POPUP4, &[0x38c]);
                cdecl(h, YES_NO, &[])
            }
            8 => {
                let r = cdecl(h, CLICK, &[4]);
                wr(h, STATE, d.cancel);
                r
            }
            _ => moved,
        }
    }

    /// `0x006f06e0()` (cdecl; state 2): the save list's input: cursor
    /// moves, confirm (7, state 3): row 0 of a list with room asks for a
    /// new file (popup 0x389, choice yes, the yes/no), any other row
    /// asks to overwrite (popup 0x203, choice no); cancel (8): state 6.
    pub fn save_list_input(h: &mut impl Host) -> u32 {
        let code = cdecl(h, MENU_INPUT, &[]);
        let moved = cdecl(h, MOVE_CURSOR, &[code]);
        if moved != 0 {
            return cdecl(h, CLICK, &[1]);
        }
        match code {
            7 => {
                cdecl(h, CLICK, &[1]);
                wr(h, STATE, 3);
                let row = rd(h, TOP).wrapping_add(rd(h, CURSOR));
                if row != 0 {
                    wr(h, CHOICE, 0);
                    return cdecl(h, SHOW_POPUP4, &[0x203]);
                }
                if rd(h, FULL) != 0 {
                    let r = cdecl(h, SHOW_POPUP4, &[0x203]);
                    wr(h, CHOICE, 0);
                    return r;
                }
                cdecl(h, SHOW_POPUP4, &[0x389]);
                wr(h, CHOICE, 1);
                cdecl(h, YES_NO, &[])
            }
            8 => {
                let r = cdecl(h, CLICK, &[4]);
                wr(h, STATE, 6);
                r
            }
            _ => moved,
        }
    }

    /// `SaveDataGetLoc_00647f30(index, out)` (cdecl):
    /// `LocalizationManager_getText(index, out, 0)`.
    pub fn get_loc(h: &mut impl Host, index: u32, out: u32) -> u32 {
        let lm = cdecl(h, LOC_MANAGER, &[]);
        h.call_sym(LOC_TEXT, Conv::Thiscall(lm), &[index, out, 0])
    }

    /// The Iggy value slots `0x00702be0` and `0x00702d90` fill (16 bytes
    /// each from `0x00cebee0` and `0x00cec020`).
    pub const EMPTY_ROW: u32 = 0x00ce_bee0;
    pub const FILE_ROW: u32 = 0x00ce_c020;
    /// `""` (`0x00b8fbeb`) and the auto-save row's title (`0x00b7c998`).
    pub const EMPTY_TEXT: u32 = 0x00b8_fbeb;
    pub const NEW_TEXT: u32 = 0x00b7_c998;
    /// Text buffers for the localised labels.
    pub const NEW_LABEL: u32 = 0x00ce_bfa0;
    pub const FILE_LABEL: u32 = 0x00ce_c0e0;

    /// `0x00702be0()` (cdecl): the "new file" row: value 0, the title,
    /// nine empty strings, the label (text 0x201) into the slots, then
    /// the menu's set-row call with 12 values. Returns its `eax`.
    pub fn new_file_row(h: &mut impl Host) -> u32 {
        let row = h.rebase(EMPTY_ROW);
        cdecl(h, IGGY_SET_VALUE, &[row, 0]);
        let title = h.rebase(NEW_TEXT);
        cdecl(h, IGGY_SET_STRING, &[row + 0x10, title]);
        let empty = h.rebase(EMPTY_TEXT);
        for i in 2..11u32 {
            cdecl(h, IGGY_SET_STRING, &[row + 0x10 * i, empty]);
        }
        let label = h.rebase(NEW_LABEL);
        cdecl(h, GET_LOC, &[0x201, label]);
        cdecl(h, IGGY_SET_STRING, &[row + 0xb0, label]);
        let m = rd(h, MENU);
        let name = h.rebase(AS3_SET_ROW);
        h.call_sym(IGGY_CALL, Conv::Thiscall(m), &[0, name, 0xc, row, 1])
    }

    /// `0x00702d90(n, s1, ..., s9, picture)` (cdecl, 11 words): a file
    /// row: value `n`, nine strings (`+0x10..+0x90`), the "file" label
    /// (text 0x200) at `+0xa0` when `picture` > 0 else an empty string,
    /// an empty string at `+0xb0`, then the set-row call with 12 values
    /// and `n == 0`. Returns its `eax`.
    pub fn file_row(h: &mut impl Host, n: u32, strings: [u32; 9], picture: u32) -> u32 {
        let row = h.rebase(FILE_ROW);
        cdecl(h, IGGY_SET_VALUE, &[row, n]);
        for (i, &s) in (1u32..).zip(strings.iter()) {
            cdecl(h, IGGY_SET_STRING, &[row + 0x10 * i, s]);
        }
        if picture.cast_signed() > 0 {
            let label = h.rebase(FILE_LABEL);
            cdecl(h, GET_LOC, &[0x200, label]);
            cdecl(h, IGGY_SET_STRING, &[row + 0xa0, label]);
        } else {
            let empty = h.rebase(EMPTY_TEXT);
            cdecl(h, IGGY_SET_STRING, &[row + 0xa0, empty]);
        }
        let empty = h.rebase(EMPTY_TEXT);
        cdecl(h, IGGY_SET_STRING, &[row + 0xb0, empty]);
        let m = rd(h, MENU);
        let name = h.rebase(AS3_SET_ROW);
        h.call_sym(
            IGGY_CALL,
            Conv::Thiscall(m),
            &[0, name, 0xc, row, u32::from(n == 0)],
        )
    }

    /// The list builders (`0x006eff00`, `SaveDataMachine_BuildList`,
    /// `0x006f0800`; cdecl): scan, sort, read the 200 headers, the count
    /// popup, the list at the top with the focus on row 0, the menu
    /// enabled, popup 0x20b for an empty list where `d` says; then
    /// `d.state`. Returns the last callee's `eax`.
    pub fn build_list(h: &mut impl Host, d: Build) -> u32 {
        cdecl(h, SCAN_FILES, &[]);
        cdecl(h, SORT_FILES, &[]);
        cdecl(h, READ_HEADERS, &[0, 0xc8]);
        let count = rd(h, COUNT);
        cdecl(h, SHOW_POPUP2, &[count, d.popup]);
        wr(h, TOP, 0);
        wr(h, CURSOR, 0);
        cdecl(h, SET_FOCUS, &[0]);
        let mut r = cdecl(h, ENABLE, &[]);
        if d.empty_popup && rd(h, COUNT) == 0 {
            r = cdecl(h, SHOW_POPUP, &[0x20b]);
        }
        wr(h, STATE, d.state);
        r
    }

    /// `0x006eff70()` (cdecl; state 25): after the timer, the menu
    /// enabled and state 0x1a.
    pub fn wait_then_enable(h: &mut impl Host) -> u32 {
        if let Some(t) = tick_timer(h) {
            return t;
        }
        let r = cdecl(h, ENABLE, &[]);
        wr(h, STATE, 0x1a);
        r
    }

    /// `0x006f0050()` (cdecl; state 16): confirm (7) enables the menu,
    /// state 0x11. Returns the input code otherwise.
    pub fn confirm_popup(h: &mut impl Host) -> u32 {
        let code = cdecl(h, MENU_INPUT, &[]);
        if code != 7 {
            return code;
        }
        let r = cdecl(h, ENABLE, &[]);
        wr(h, STATE, 0x11);
        r
    }

    /// `0x006f04c0()` (cdecl; state 5): after the timer, the save sound
    /// (`SndSepPlaySimple(0x8000006a)`), the menu re-opened and cleared,
    /// state 1, popup 0x1fc.
    pub fn saved(h: &mut impl Host) -> u32 {
        if let Some(t) = tick_timer(h) {
            return t;
        }
        cdecl(h, SE_PLAY, &[0x8000_006a]);
        cdecl(h, ENABLE, &[]);
        cdecl(h, MENU_RELEASE, &[]);
        cdecl(h, MENU_OPEN, &[]);
        cdecl(h, CLEAR, &[]);
        wr(h, STATE, 1);
        cdecl(h, SHOW_POPUP, &[0x1fc])
    }

    /// `SaveDataMachine_Update()` (cdecl): the handler of the current
    /// state (1..27; others return `state - 1`).
    pub fn machine_update(h: &mut impl Host) -> u32 {
        let state = rd(h, STATE);
        let target = match state {
            1 => 0x006f_0800,
            2 => 0x006f_06e0,
            3 => 0x006f_05b0,
            4 => return with_row(h, 0x006f_0860),
            5 => 0x006f_04c0,
            6 => 0x006f_0630,
            11 => 0x006f_0220,
            12 => 0x006f_0140,
            13 => 0x006e_ffa0,
            14 => return with_row(h, 0x006f_0070),
            15 => 0x006e_fbd0,
            16 => 0x006f_0050,
            17 => 0x006f_0020,
            21 => 0x006e_ff00,
            22 => 0x006e_fe20,
            23 => 0x006e_fc50,
            24 => return with_row(h, 0x006e_fd10),
            25 => 0x006e_ff70,
            26 => 0x006e_fcd0,
            27 => 0x006e_fde0,
            _ => return state.wrapping_sub(1),
        };
        cdecl(h, target, &[])
    }

    /// The handlers that take the selected row (`CURSOR + TOP`).
    fn with_row(h: &mut impl Host, target: u32) -> u32 {
        let row = rd(h, CURSOR).wrapping_add(rd(h, TOP));
        cdecl(h, target, &[row])
    }

    /// `code_006f2740()` (16 bytes): a tail jump to
    /// `SaveDataMachine_Update`.
    pub fn machine_update_thunk(h: &mut impl Host) -> u32 {
        cdecl(h, MACHINE_UPDATE, &[])
    }

    /// `SaveData_Update()` (cdecl, every frame): the trophy manager's
    /// update once the wait fraction (`0x006f0670`) reaches the float at
    /// `0x00b921d8`, the player's position (`Sg_GetPlayerPos`), the save
    /// machine, then the trophy manager's post-update (`0x0067ad00`, a
    /// bare `ret`: `eax` stays the manager).
    pub fn save_data_update(h: &mut impl Host) -> u32 {
        let f = h.call_sym_st0(TICK_SECONDS, Conv::Cdecl, &[]);
        let limit: f32 = h.read(h.rebase(TROPHY_INTERVAL));
        // `fcomp`; `test ah, 1`: C0 (below, or unordered) skips
        if !(f < limit || f.is_nan() || limit.is_nan()) {
            cdecl(h, TROPHY_MANAGER_UPDATE, &[]);
        }
        let out = h.rebase(PLAYER_POS_OUT);
        cdecl(h, PLAYER_POS, &[out]);
        let m = cdecl(h, SAVE_MANAGER, &[]);
        h.call_sym(MACHINE_UPDATE_THUNK, Conv::Thiscall(m), &[]);
        let t = cdecl(h, TROPHY_MANAGER, &[]);
        h.call_sym(TROPHY_POST_UPDATE, Conv::Thiscall(t), &[]);
        t
    }
}
