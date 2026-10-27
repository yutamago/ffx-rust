//! M5 stage 1, list `system-a`: the `<system_error>` categories and the
//! string streams' unwinding destructors (`ffx_system::m5a_std`), the
//! settings object and the PC menus' key-name tables
//! (`ffx_system::m5a_settings`), the PC save/load screen's state machine
//! and its input and trophy glue (`ffx_system::m5a_savedata`).
//!
//! Threads: the main thread (settings at start-up, the menus) and
//! whichever thread unwinds a string stream; the shims keep no state,
//! frames are per call.

use super::{StackFrame, mem, ported};
use ffx_system::m5a_savedata::{self as sv, bridge as sa};
use ffx_system::m5a_settings::bridge as st;
use ffx_system::m5a_std::{self as sd, bridge as sb};

ported! {
    // ---- <system_error>, <ios> and the string streams ----
    /// `0x004011d0` (`ecx` = the facet holder; unwinding).
    StdLocale_releaseFacet_004011d0 @ 0x0040_11d0 => extern "thiscall" fn release_facet(holder: u32) {
        sb::release_facet(&mut mem(), holder);
    }
    /// `std::error_category`'s deleting destructor `(this; flags)`.
    StdErrorCategory_deletingDtor_00401260 @ 0x0040_1260 => extern "thiscall" fn category_delete_00401260(this: u32, flags: u32) -> u32 {
        sb::category_deleting_dtor(&mut mem(), this, flags)
    }
    /// `error_category::equivalent(this; value, &cond)`: frame `ebp - 8`.
    StdErrorCategory_equivalentValue_00401330 @ 0x0040_1330 => extern "thiscall" fn equivalent_value(this: u32, value: u32, cond: u32) -> u32 {
        let mut f = StackFrame::<2>::new();
        sb::equivalent_code(&mut mem(), f.at() + 8, this, value, cond)
    }
    /// `error_category::equivalent(this; &code, value)`.
    StdErrorCategory_equivalentCode_00401370 @ 0x0040_1370 => extern "thiscall" fn equivalent_code(this: u32, code: u32, value: u32) -> u32 {
        sb::equivalent_cond(&mut mem(), this, code, value)
    }
    /// `_Generic_error_category::message(out, code)` (`ecx` unused).
    StdGenericErrorCategory_message_004013b0 @ 0x0040_13b0 => extern "stdcall" fn generic_message(out: u32, code: u32) -> u32 {
        sb::generic_message(&mut mem(), out, code)
    }
    /// `_Generic_error_category`'s deleting destructor `(this; flags)`.
    StdGenericErrorCategory_deletingDtor_00401430 @ 0x0040_1430 => extern "thiscall" fn category_delete_00401430(this: u32, flags: u32) -> u32 {
        sb::category_deleting_dtor(&mut mem(), this, flags)
    }
    /// `_Iostream_error_category::message(out, code)` (`ecx` unused).
    StdIostreamErrorCategory_message_00401490 @ 0x0040_1490 => extern "stdcall" fn iostream_message(out: u32, code: u32) -> u32 {
        sb::iostream_message(&mut mem(), out, code)
    }
    /// `_Iostream_error_category`'s deleting destructor `(this; flags)`.
    StdIostreamErrorCategory_deletingDtor_004014e0 @ 0x0040_14e0 => extern "thiscall" fn category_delete_004014e0(this: u32, flags: u32) -> u32 {
        sb::category_deleting_dtor(&mut mem(), this, flags)
    }
    /// `_System_error_category::message(out, code)` (`ecx` unused).
    StdSystemErrorCategory_message_00401540 @ 0x0040_1540 => extern "stdcall" fn system_message(out: u32, code: u32) -> u32 {
        sb::system_message(&mut mem(), out, code)
    }
    /// `_System_error_category::default_error_condition(out, code)`.
    StdSystemErrorCategory_defaultErrorCondition_004015c0 @ 0x0040_15c0 => extern "stdcall" fn default_condition(out: u32, code: u32) -> u32 {
        sb::default_condition(&mut mem(), out, code)
    }
    /// `_System_error_category`'s deleting destructor `(this; flags)`.
    StdSystemErrorCategory_deletingDtor_00401600 @ 0x0040_1600 => extern "thiscall" fn category_delete_00401600(this: u32, flags: u32) -> u32 {
        sb::category_deleting_dtor(&mut mem(), this, flags)
    }
    /// `0x00401660` (`ecx` = the stream holder; unwinding).
    StdOstreamSentry_unlock_00401660 @ 0x0040_1660 => extern "thiscall" fn sentry_unlock(holder: u32) {
        sb::sentry_unlock(&mut mem(), holder);
    }
    /// `0x00401840` (`ecx` = the `std::stringstream`; unwinding).
    StdStringstream_dtor_00401840 @ 0x0040_1840 => extern "thiscall" fn stringstream_dtor(this: u32) -> u32 {
        sb::stream_dtor(&mut mem(), this, sd::STRINGSTREAM_DTOR)
    }
    /// `0x004022b0` (`ecx` = the `std::istringstream`; unwinding).
    StdIstringstream_dtor_004022b0 @ 0x0040_22b0 => extern "thiscall" fn istringstream_dtor(this: u32) -> u32 {
        sb::stream_dtor(&mut mem(), this, sd::ISTRINGSTREAM_DTOR)
    }
    /// `0x004029c0` (`ecx` = the `std::ostringstream`; unwinding).
    StdOstringstream_dtor_004029c0 @ 0x0040_29c0 => extern "thiscall" fn ostringstream_dtor(this: u32) -> u32 {
        sb::stream_dtor(&mut mem(), this, sd::OSTRINGSTREAM_DTOR)
    }
    /// `0x00405a77`: a vtordisp thunk (`ecx`, no stack words).
    StdIstream_vtordispThunk_00405a77 @ 0x0040_5a77 => extern "thiscall" fn vtordisp_istream(this: u32) -> u32 {
        sb::vtordisp_istream(&mut mem(), this)
    }
    /// `0x00405a83`: a vtordisp thunk `(this; flags)`.
    StdStringstream_vtordispDelete_00405a83 @ 0x0040_5a83 => extern "thiscall" fn vtordisp_delete(this: u32, flags: u32) -> u32 {
        sb::vtordisp_stringstream_delete(&mut mem(), this, flags)
    }

    // ---- the settings and the key-name tables ----
    /// `Settings_Construct(this; path)`.
    Settings_Construct @ 0x0040_1880 => extern "thiscall" fn settings_construct(this: u32, path: u32) -> u32 {
        st::construct(&mut mem(), this, path)
    }
    /// `Settings_DetectQuality(this)`.
    Settings_DetectQuality @ 0x0040_2a20 => extern "thiscall" fn detect_quality(this: u32) -> u32 {
        st::detect_quality(&mut mem(), this)
    }
    /// `0x00405a90(slot)`: whether the keyboard table names `slot`.
    KeyNames_hasKeyboard_00405a90 @ 0x0040_5a90 => extern "C" fn has_keyboard(slot: u32) -> u32 {
        st::has_name(&mem(), st::KEYBOARD, slot)
    }
    /// `0x00405ac0(slot)`: whether the controller table names `slot`.
    KeyNames_hasController_00405ac0 @ 0x0040_5ac0 => extern "C" fn has_controller(slot: u32) -> u32 {
        st::has_name(&mem(), st::CONTROLLER, slot)
    }
    /// `KeyNames_getKeyboard_00405af0(slot)`.
    KeyNames_getKeyboard_00405af0 @ 0x0040_5af0 => extern "C" fn get_keyboard(slot: u32) -> u32 {
        st::get_name(&mem(), st::KEYBOARD, slot)
    }
    /// `KeyNames_getController_00405b10(slot)`.
    KeyNames_getController_00405b10 @ 0x0040_5b10 => extern "C" fn get_controller(slot: u32) -> u32 {
        st::get_name(&mem(), st::CONTROLLER, slot)
    }
    /// `KeyNames_setKeyboard_00405b30(slot, name)` -> `al`.
    KeyNames_setKeyboard_00405b30 @ 0x0040_5b30 => extern "C" fn set_keyboard(slot: u32, name: u32) -> u8 {
        st::set_name(&mut mem(), st::KEYBOARD, slot, name)
    }
    /// `KeyNames_setController_00405b60(slot, name)` -> `al`.
    KeyNames_setController_00405b60 @ 0x0040_5b60 => extern "C" fn set_controller(slot: u32, name: u32) -> u8 {
        st::set_name(&mut mem(), st::CONTROLLER, slot, name)
    }
    /// `ReturnConst80_00405b90()`: the key-name tables' size.
    ReturnConst80_00405b90 @ 0x0040_5b90 => extern "C" fn key_name_slots() -> u32 {
        0x80
    }
    /// `Settings_IsFfxBindingSlot(slot)`.
    Settings_IsFfxBindingSlot @ 0x0040_5cb0 => extern "C" fn is_ffx_slot(slot: u32) -> u32 {
        st::is_ffx_slot(slot)
    }
    /// `Settings_IsFfx2BindingSlot(slot)`.
    Settings_IsFfx2BindingSlot @ 0x0040_5d00 => extern "C" fn is_ffx2_slot(slot: u32) -> u32 {
        st::is_ffx2_slot(slot)
    }
    /// `Settings_IsLmBindingSlot(slot)`.
    Settings_IsLmBindingSlot @ 0x0040_5d80 => extern "C" fn is_lm_slot(slot: u32) -> u32 {
        st::is_lm_slot(slot)
    }

    // ---- the save/load screen, input and trophies ----
    /// `0x0067ac60(this)` (9 bytes; its caller enters with `eax` = `ecx`).
    TrophyManager_errorFlag_0067ac60 @ 0x0067_ac60 => extern "thiscall" fn trophy_flag(this: u32) -> u32 {
        sa::trophy_flag(&mem(), this)
    }
    /// `TrophyGetError()`.
    TrophyGetError @ 0x0064_8ff0 => extern "C" fn trophy_get_error() -> u32 {
        sa::trophy_get_error(&mut mem())
    }
    /// `Virtuos_InputManager_setInputInfo(this)`: frame `ebp - 0x18`.
    Virtuos_InputManager_setInputInfo @ 0x0063_1550 => extern "thiscall" fn set_input_info(this: u32) -> u32 {
        let mut f = StackFrame::<6>::new();
        sa::set_input_info(&mut mem(), f.at() + 0x18, this)
    }
    /// `0x00648010()`.
    SaveDataTakeLoadedFlag_00648010 @ 0x0064_8010 => extern "C" fn take_loaded_flag() -> u32 {
        sa::save_take_byte364(&mut mem())
    }
    /// `TrophyManager_Update()`.
    TrophyManager_Update @ 0x0064_1680 => extern "C" fn trophy_manager_update() {
        sa::trophy_manager_update(&mut mem());
    }
    /// `0x006f0670()` -> `ST0`.
    SaveDataMachine_waitFraction_006f0670 @ 0x006f_0670 => extern "C" fn wait_fraction() -> f32 {
        sa::wait_fraction(&mut mem())
    }
    /// `0x00702a30()`.
    SaveDataMachine_as3Enable_00702a30 @ 0x0070_2a30 => extern "C" fn as3_enable() -> u32 {
        sa::as3_call(&mut mem(), sv::AS3_ENABLE)
    }
    /// `SaveDataMachine_CheckFile()`.
    SaveDataMachine_CheckFile @ 0x006e_fbd0 => extern "C" fn check_file() -> u32 {
        sa::check_file(&mut mem())
    }
    /// `inputGetCurrDeviceStickInfo()`.
    inputGetCurrDeviceStickInfo @ 0x0063_0c90 => extern "C" fn stick_info() -> u32 {
        sa::stick_info(&mut mem())
    }
    /// `inputIsHoldButton(b)`.
    inputIsHoldButton @ 0x0063_0cd0 => extern "C" fn is_hold(b: u32) -> u32 {
        sa::input_button(&mut mem(), sv::INPUT_IS_HOLD, b)
    }
    /// `inputIsPressButton(b)`.
    inputIsPressButton @ 0x0063_0cf0 => extern "C" fn is_press(b: u32) -> u32 {
        sa::input_button(&mut mem(), sv::INPUT_IS_PRESS, b)
    }
    /// `0x00702a50()`.
    SaveDataMachine_as3Disable_00702a50 @ 0x0070_2a50 => extern "C" fn as3_disable() -> u32 {
        sa::as3_call(&mut mem(), sv::AS3_DISABLE)
    }
    /// `0x006efc50()`.
    SaveDataMachine_overwriteYesNo_006efc50 @ 0x006e_fc50 => extern "C" fn overwrite_yes_no() -> u32 {
        sa::yes_no(&mut mem(), sv::YES_NO_OVERWRITE)
    }
    /// `SaveDataEnableSave()`.
    SaveDataEnableSave @ 0x0064_7d40 => extern "C" fn enable_save() -> u32 {
        sa::enable_save(&mut mem())
    }
    /// `0x00647d50(v)`.
    SaveDataSetSaveMode_00647d50 @ 0x0064_7d50 => extern "C" fn set_save_mode(v: u32) -> u32 {
        sa::set_save_mode(&mut mem(), v)
    }
    /// `0x006f1aa0(this; v)`.
    SaveDataManager_setResult_006f1aa0 @ 0x006f_1aa0 => extern "thiscall" fn set_result(this: u32, v: u32) -> u32 {
        sa::set_result(&mut mem(), this, v)
    }
    /// `SaveDataSaveLoadSucceed(kind)`.
    SaveDataSaveLoadSucceed @ 0x0064_8540 => extern "C" fn save_load_succeed(kind: u32) -> u32 {
        sa::save_load_succeed(&mut mem(), kind)
    }
    /// `0x00702fa0()`.
    SaveDataMachine_releaseMenu_00702fa0 @ 0x0070_2fa0 => extern "C" fn release_menu() {
        sa::release_menu(&mut mem());
    }
    /// `0x006efcd0()`.
    SaveDataMachine_saveDone1_006efcd0 @ 0x006e_fcd0 => extern "C" fn save_done1() -> u32 {
        sa::save_done(&mut mem(), 1)
    }
    /// `SaveDataMachine_MergeFileFlags_006efd10(slot)`: frame `ebp - 0x6d04`.
    SaveDataMachine_MergeFileFlags_006efd10 @ 0x006e_fd10 => extern "C" fn merge_file_flags(slot: u32) -> u32 {
        let mut f = StackFrame::<{ (sv::bridge::MERGE_FRAME / 4) as usize }>::new();
        sa::merge_file_flags(&mut mem(), f.at() + sv::bridge::MERGE_FRAME, slot)
    }
    /// `0x006efde0()`.
    SaveDataMachine_saveDone0_006efde0 @ 0x006e_fde0 => extern "C" fn save_done0() -> u32 {
        sa::save_done(&mut mem(), 0)
    }
    /// `0x006f0290()`.
    SaveDataMachine_cursorDown_006f0290 @ 0x006f_0290 => extern "C" fn cursor_down() -> u32 {
        sa::cursor_down(&mut mem())
    }
    /// `0x006ef630(code)`.
    SaveDataMachine_moveCursor_006ef630 @ 0x006e_f630 => extern "C" fn move_cursor(code: u32) -> u32 {
        sa::move_cursor(&mut mem(), code)
    }
    /// `0x006efe20()`.
    SaveDataMachine_overwriteListInput_006efe20 @ 0x006e_fe20 => extern "C" fn overwrite_list_input() -> u32 {
        sa::list_input(&mut mem(), sv::LIST_OVERWRITE)
    }
    /// `SaveDataGetLoc_00647f30(index, out)`.
    SaveDataGetLoc_00647f30 @ 0x0064_7f30 => extern "C" fn get_loc(index: u32, out: u32) -> u32 {
        sa::get_loc(&mut mem(), index, out)
    }
    /// `0x00702be0()`.
    SaveDataMachine_newFileRow_00702be0 @ 0x0070_2be0 => extern "C" fn new_file_row() -> u32 {
        sa::new_file_row(&mut mem())
    }
    /// `0x00702d90(n, s1..s9, picture)`.
    SaveDataMachine_fileRow_00702d90 @ 0x0070_2d90 => extern "C" fn file_row(
        n: u32, s1: u32, s2: u32, s3: u32, s4: u32, s5: u32, s6: u32, s7: u32, s8: u32, s9: u32, picture: u32
    ) -> u32 {
        sa::file_row(&mut mem(), n, [s1, s2, s3, s4, s5, s6, s7, s8, s9], picture)
    }
    /// `0x006eff00()`.
    SaveDataMachine_buildOverwriteList_006eff00 @ 0x006e_ff00 => extern "C" fn build_overwrite_list() -> u32 {
        sa::build_list(&mut mem(), sv::BUILD_OVERWRITE)
    }
    /// `0x006eff70()`.
    SaveDataMachine_waitEnable_006eff70 @ 0x006e_ff70 => extern "C" fn wait_enable() -> u32 {
        sa::wait_then_enable(&mut mem())
    }
    /// `0x006effa0()`.
    SaveDataMachine_loadYesNo_006effa0 @ 0x006e_ffa0 => extern "C" fn load_yes_no() -> u32 {
        sa::yes_no(&mut mem(), sv::YES_NO_LOAD)
    }
    /// `SaveDataMachine_LoadDone()`.
    SaveDataMachine_LoadDone @ 0x006f_0020 => extern "C" fn load_done() -> u32 {
        sa::load_done(&mut mem())
    }
    /// `0x006f0050()`.
    SaveDataMachine_confirmPopup_006f0050 @ 0x006f_0050 => extern "C" fn confirm_popup() -> u32 {
        sa::confirm_popup(&mut mem())
    }
    /// `SaveDataMachine_ReadFile(slot)`: frame `ebp - 0x408`.
    SaveDataMachine_ReadFile @ 0x006f_0070 => extern "C" fn read_file(slot: u32) -> u32 {
        let mut f = StackFrame::<{ (sv::bridge::READ_FRAME / 4) as usize }>::new();
        sa::read_file(&mut mem(), f.at() + sv::bridge::READ_FRAME, slot)
    }
    /// `0x006f0140()`.
    SaveDataMachine_loadListInput_006f0140 @ 0x006f_0140 => extern "C" fn load_list_input() -> u32 {
        sa::list_input(&mut mem(), sv::LIST_LOAD)
    }
    /// `SaveDataMachine_BuildList()`.
    SaveDataMachine_BuildList @ 0x006f_0220 => extern "C" fn build_load_list() -> u32 {
        sa::build_list(&mut mem(), sv::BUILD_LOAD)
    }
    /// `0x00702f80()`.
    SaveDataMachine_as3Clear_00702f80 @ 0x0070_2f80 => extern "C" fn as3_clear() -> u32 {
        sa::as3_call(&mut mem(), sv::AS3_CLEAR)
    }
    /// `0x006f04c0()`.
    SaveDataMachine_saved_006f04c0 @ 0x006f_04c0 => extern "C" fn saved() -> u32 {
        sa::saved(&mut mem())
    }
    /// `0x006f05b0()`.
    SaveDataMachine_saveYesNo_006f05b0 @ 0x006f_05b0 => extern "C" fn save_yes_no() -> u32 {
        sa::yes_no(&mut mem(), sv::YES_NO_SAVE)
    }
    /// `0x006f0630()`.
    SaveDataMachine_saveFinished_006f0630 @ 0x006f_0630 => extern "C" fn save_finished() -> u32 {
        sa::save_finished(&mut mem())
    }
    /// `0x006f06e0()`.
    SaveDataMachine_saveListInput_006f06e0 @ 0x006f_06e0 => extern "C" fn save_list_input() -> u32 {
        sa::save_list_input(&mut mem())
    }
    /// `0x006f0800()`.
    SaveDataMachine_buildSaveList_006f0800 @ 0x006f_0800 => extern "C" fn build_save_list() -> u32 {
        sa::build_list(&mut mem(), sv::BUILD_SAVE)
    }
    /// `SaveDataMachine_WriteFile(slot)`: frame `ebp - 0x404`.
    SaveDataMachine_WriteFile @ 0x006f_0860 => extern "C" fn write_file(slot: u32) -> u32 {
        let mut f = StackFrame::<{ (sv::bridge::WRITE_FRAME / 4) as usize }>::new();
        sa::write_file(&mut mem(), f.at() + sv::bridge::WRITE_FRAME, slot)
    }
    /// `SaveDataMachine_Update()`.
    SaveDataMachine_Update @ 0x006f_0950 => extern "C" fn machine_update() -> u32 {
        sa::machine_update(&mut mem())
    }
    /// `SaveData_Update()`.
    SaveData_Update @ 0x0064_8480 => extern "C" fn save_data_update() -> u32 {
        sa::save_data_update(&mut mem())
    }
}
