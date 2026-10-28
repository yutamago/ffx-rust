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
use ffx_system::m5a_platform::bridge as pf;
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

    // ---- round 2: the custom convention and the jump thunks ----
    /// `0x00401690(text; out, max)`: `ecx`, `edx` and a stack word the
    /// caller pops.
    Settings_ParseIntList_00401690 @ 0x0040_1690 => extern "fastcall-caller-pops" fn parse_int_list(text: u32, out: u32, max: u32) -> u32 {
        st::parse_list(&mut mem(), text, out, max)
    }
    /// `Settings_ValidateBindings(list; n, filter)`: `ecx`, `edx` and a
    /// stack word the caller pops.
    Settings_ValidateBindings @ 0x0040_5bb0 => extern "fastcall-caller-pops" fn validate_bindings(list: u32, n: u32, filter: u32) -> u32 {
        st::validate_bindings(&mut mem(), list, n, filter)
    }
    /// `Settings_JoinBindings(out; list, n)`: `ecx`, `edx` and a stack
    /// word the caller pops; frame `ebp - 0xd0`.
    Settings_JoinBindings @ 0x0040_16e0 => extern "fastcall-caller-pops" fn join_bindings(out: u32, list: u32, n: u32) -> u32 {
        let mut f = StackFrame::<{ (st::JOIN_FRAME / 4) as usize }>::new();
        st::join_bindings(&mut mem(), f.at() + st::JOIN_FRAME, out, list, n)
    }
    /// `Settings_SetValue(this; key, std::string value)`: the string's six
    /// words read where the caller pushed them (`slots`), frame
    /// `ebp - 0x50`.
    Settings_SetValue @ 0x0040_1930 => extern "thiscall" fn set_value(this: u32, key: u32, s0: u32, s1: u32, s2: u32, s3: u32, s4: u32, s5: u32; slots: u32) -> u32 {
        let _ = (s0, s1, s2, s3, s4, s5);
        let mut f = StackFrame::<{ (st::SET_VALUE_FRAME / 4) as usize }>::new();
        st::set_value(&mut mem(), f.at() + st::SET_VALUE_FRAME, this, key, slots + 4)
    }
    /// `Settings_Save(this)`: the original's `alloca` buffer is a zeroed
    /// heap block here.
    Settings_Save @ 0x0040_1d80 => extern "thiscall" fn settings_save(this: u32) -> u32 {
        let mut store = Vec::new();
        st::save(&mut mem(), this, |n| zeroed(&mut store, n))
    }
    /// `Settings_Load(this)`: the same for the file's bytes (and the 0x20
    /// spare ones, junk in the original).
    Settings_Load @ 0x0040_1cb0 => extern "thiscall" fn settings_load(this: u32) -> u32 {
        let mut store = Vec::new();
        st::load(&mut mem(), this, |n| zeroed(&mut store, n))
    }
    // ---- the platform glue (ffx_system::m5a_platform) ----
    /// `fiosClose(f)`.
    fiosClose @ 0x0067_9510 => extern "C" fn fios_close(f: u32) -> u32 {
        pf::fios_close(&mut mem(), f)
    }
    /// `sceClose(fd)`.
    sceClose @ 0x0062_f610 => extern "C" fn sce_close(fd: u32) -> u32 {
        pf::sce_close(&mut mem(), fd)
    }
    /// `sceWrite(fd, buf, ..)`: only `buf` is read.
    sceWrite @ 0x0062_fdc0 => extern "C" fn sce_write(fd: u32, buf: u32) -> u32 {
        let _ = fd;
        pf::sce_write(&mut mem(), buf)
    }
    /// `0x0061b3c0(module, info, id)`.
    AtlStringResourceImage_0061b3c0 @ 0x0061_b3c0 => extern "C" fn string_resource_image(module: u32, info: u32, id: u32) -> u32 {
        pf::string_resource_image(&mut mem(), module, info, id)
    }
    /// `SaveDataIsSaveEnabled()`.
    SaveDataIsSaveEnabled @ 0x0064_80e0 => extern "C" fn is_save_enabled() -> u32 {
        pf::is_save_enabled(&mut mem())
    }
    /// `(this; id)`: `eax` junk to the caller.
    StatsAndAchievements_SetAchievement_006f1900 @ 0x006f_1900 => extern "thiscall" fn set_achievement(this: u32, id: u32) {
        pf::set_achievement(&mut mem(), this, id);
    }
    /// `(this; id)`.
    StatsAndAchievements_Unlock_006f1510 @ 0x006f_1510 => extern "thiscall" fn unlock(this: u32, id: u32) {
        pf::unlock(&mut mem(), this, id);
    }
    /// `(this)`: `ecx` is passed on to the unlock.
    AchievementTryObtainPlat @ 0x006f_1550 => extern "thiscall" fn try_obtain_platinum(this: u32) {
        pf::try_obtain_platinum(&mut mem(), this);
    }
    /// `SaveDataUnlockAchievement(id)`.
    SaveDataUnlockAchievement @ 0x0064_88c0 => extern "C" fn unlock_achievement(id: u32) {
        pf::unlock_achievement(&mut mem(), id);
    }
    /// `fiosOpen(name, mode)`: the error path's `CString` lives in the
    /// caller's `mode` slot (`slots + 4`), as in the original.
    fiosOpen @ 0x0067_9730 => extern "C" fn fios_open(name: u32, mode: u32; slots: u32) -> u32 {
        pf::fios_open(&mut mem(), name, mode, slots + 4)
    }
    /// `sceOpen(name, mode)`: frame `ebp - 0x804`.
    sceOpen @ 0x0062_fa30 => extern "C" fn sce_open(name: u32, mode: u32) -> u32 {
        let mut f = StackFrame::<{ (pf::SCE_OPEN_FRAME / 4) as usize }>::new();
        pf::sce_open(&mut mem(), f.at() + pf::SCE_OPEN_FRAME, name, mode)
    }
    /// `TOSetEnvMovieMessage()`.
    TOSetEnvMovieMessage @ 0x008a_f590 => extern "C" fn set_env_movie_message() -> u32 {
        pf::set_env_movie_message(&mut mem())
    }
    /// `TOGetFFXLangJmp()`: the jump to `TOGetFFXLang`.
    TOGetFFXLangJmp @ 0x008a_c2f0 => extern "C" fn get_ffx_lang_jmp() -> u32 {
        pf::get_ffx_lang_jmp(&mut mem())
    }
    /// `ToDrawEternalCalmCopyRight(frame)`: the quad at `ebp - 0x9c`.
    ToDrawEternalCalmCopyRight @ 0x0090_4a70 => extern "C" fn draw_copyright(frame: u32) -> u32 {
        let mut f = StackFrame::<{ (pf::COPYRIGHT_FRAME / 4) as usize }>::new();
        pf::draw_copyright(&mut mem(), f.at() + pf::COPYRIGHT_FRAME, frame)
    }
    /// `(this; queue)`: `eax` junk to the callers.
    JobSchedule_JobScheduler_waitForComplete @ 0x006d_bf50 => extern "thiscall" fn wait_for_complete(this: u32, queue: u32) {
        pf::wait_for_complete(&mut mem(), this, queue);
    }
    /// `()`: queue 0 of the cloth scheduler.
    JobScheduleWait_Ch_Cloth_PerCharacter @ 0x00a4_4b30 => extern "C" fn wait_cloth() {
        pf::wait_cloth(&mut mem());
    }
    /// `(this)`.
    LoadingScreen_unloadTextures @ 0x006e_5050 => extern "thiscall" fn unload_textures(this: u32) -> u32 {
        pf::unload_textures(&mut mem(), this)
    }
    /// `(this)`.
    LoadingScreen_terminate @ 0x006e_5020 => extern "thiscall" fn terminate(this: u32) -> u32 {
        pf::terminate(&mut mem(), this)
    }
    /// `(this)`: `eax` junk to the callers.
    LoadingScreen_dtor_00634790 @ 0x0063_4790 => extern "thiscall" fn loading_screen_dtor(this: u32) {
        pf::loading_screen_dtor(&mut mem(), this);
    }
    /// `(this)`.
    CDataFrameQueue_dtor @ 0x00a3_dd50 => extern "thiscall" fn data_frame_queue_dtor(this: u32) {
        pf::data_frame_queue_dtor(&mut mem(), this);
    }
    /// `(this)`: `eax` junk to the callers.
    VbfMkvReader_dtor @ 0x00a3_e040 => extern "thiscall" fn vbf_mkv_reader_dtor(this: u32) {
        pf::vbf_mkv_reader_dtor(&mut mem(), this);
    }
    /// `(this)`: `eax` junk to the callers.
    BufferTree_dtor_00633fe0 @ 0x0063_3fe0 => extern "thiscall" fn buffer_tree_dtor(this: u32) {
        pf::buffer_tree_dtor(&mut mem(), this);
    }
    /// `(this)`.
    ListVector_dtor_00634220 @ 0x0063_4220 => extern "thiscall" fn list_vector_dtor(this: u32) -> u32 {
        pf::list_vector_dtor(&mut mem(), this)
    }
    /// `(fd, fmt, ...)`: the variable arguments where the caller pushed
    /// them (`slots + 8`); frame `ebp - 0x404`.
    SaveDataTtyWrite2 @ 0x0064_8850 => extern "C" fn tty_write2(fd: u32, fmt: u32; slots: u32) -> u32 {
        let _ = fd;
        let mut f = StackFrame::<{ (pf::TTY_FRAME / 4) as usize }>::new();
        pf::tty_write2(&mut mem(), f.at() + pf::TTY_FRAME, fmt, slots + 8)
    }
    /// `(this)`.
    FogEffectManager_resetAll_006e3f10 @ 0x006e_3f10 => extern "thiscall" fn fog_reset_all(this: u32) -> u32 {
        pf::fog_reset_all(&mut mem(), this)
    }
    /// `()`.
    AsyncLoadManager_shutdown_00691b00 @ 0x0069_1b00 => extern "C" fn async_load_shutdown() -> u32 {
        pf::async_load_shutdown(&mut mem())
    }
    /// `()`: `eax` junk to the callers.
    ClusterManager_shutdown_0069a0e0 @ 0x0069_a0e0 => extern "C" fn cluster_manager_shutdown() {
        pf::cluster_manager_shutdown(&mut mem());
    }
    /// `(p)`: the 5-byte `jmp user_aligned_free`.
    code_00686fe0 @ 0x0068_6fe0 => extern "C" fn aligned_free_thunk(p: u32) -> u32 {
        pf::aligned_free_thunk(&mut mem(), p)
    }
    /// `(this; id)`: `al` the answer, the upper bytes `ReleaseMutex`'s.
    MapHasActive_0069b890 @ 0x0069_b890 => extern "thiscall" fn map_has_active(this: u32, id: u32) -> u32 {
        pf::map_has_active(&mut mem(), this, id)
    }
    /// `(this)`.
    SamplerHolder_clear_00703070 @ 0x0070_3070 => extern "thiscall" fn clear_sampler_holder(this: u32) -> u32 {
        pf::clear_sampler_holder(&mut mem(), this)
    }
    /// `()`: `eax` junk to the callers.
    LocalizationManager_shutdown_006db040 @ 0x006d_b040 => extern "C" fn localization_shutdown() {
        pf::localization_shutdown(&mut mem());
    }
    /// `fiosFinalize()`.
    fiosFinalize @ 0x0067_9590 => extern "C" fn fios_finalize() -> u32 {
        pf::fios_finalize(&mut mem())
    }
    /// `()`.
    SyncManager_shutdown_0067a900 @ 0x0067_a900 => extern "C" fn sync_manager_shutdown() -> u32 {
        pf::sync_manager_shutdown(&mut mem())
    }
    /// `()`.
    FreeBlock_0067ac40 @ 0x0067_ac40 => extern "C" fn free_block_ccc7cc() -> u32 {
        pf::free_block_ccc7cc(&mut mem())
    }
    /// `()`.
    SamplerStates_release_006f5450 @ 0x006f_5450 => extern "C" fn release_samplers() -> u32 {
        pf::release_samplers(&mut mem())
    }
    /// `(this; job, queue)`: the queue's address goes into the caller's
    /// `queue` slot (`slots + 4`), as in the original.
    JobScheduler_pushJob_006dbd10 @ 0x006d_bd10 => extern "thiscall" fn push_job(this: u32, job: u32, queue: u32; slots: u32) -> u32 {
        pf::push_job(&mut mem(), this, job, queue, Some(slots + 4))
    }
    /// `(this)`.
    JobScheduler_shutdown_006dbe80 @ 0x006d_be80 => extern "thiscall" fn scheduler_shutdown(this: u32) -> u32 {
        pf::scheduler_shutdown(&mut mem(), this)
    }
    /// `(this; timeout)`.
    BigFileStream_claimSlot_0061b4a0 @ 0x0061_b4a0 => extern "thiscall" fn claim_slot(this: u32, timeout: u32) -> u32 {
        pf::claim_slot(&mut mem(), this, timeout)
    }
    /// `(this)`: `eax` junk to the callers.
    PRedBlackTree_dtor_0061d5c0 @ 0x0061_d5c0 => extern "thiscall" fn tree_dtor(this: u32) {
        pf::tree_dtor(&mut mem(), this);
    }
    /// `(this)`: `eax` junk to the callers.
    BigFileHandle_deinitialize @ 0x0061_d910 => extern "thiscall" fn big_file_deinitialize(this: u32) {
        pf::big_file_deinitialize(&mut mem(), this);
    }
    /// `(this)`: `eax` junk to the callers.
    BigFileHandle_dtor_BigFileHandle @ 0x0061_d670 => extern "thiscall" fn big_file_handle_dtor(this: u32) {
        pf::big_file_handle_dtor(&mut mem(), this);
    }
    /// `(this)`.
    BigFileStream_term_0061b620 @ 0x0061_b620 => extern "thiscall" fn big_file_stream_term(this: u32) -> u32 {
        pf::big_file_stream_term(&mut mem(), this)
    }
    /// `()`: `eax` junk to the caller.
    BigFileStream_shutdown_0061b690 @ 0x0061_b690 => extern "C" fn big_file_stream_shutdown() {
        pf::big_file_stream_shutdown(&mut mem());
    }
    /// `user_free_thunk(p)`: the 5-byte `jmp user_free`.
    user_free_thunk @ 0x0068_7010 => extern "C" fn user_free_thunk(p: u32) -> u32 {
        sa::user_free_thunk(&mut mem(), p)
    }
    /// `code_006f2740()`: the 5-byte `jmp SaveDataMachine_Update`.
    code_006f2740 @ 0x006f_2740 => extern "C" fn machine_update_thunk() -> u32 {
        sa::machine_update_thunk(&mut mem())
    }
}

/// `n` zero bytes in `store` (replacing what it held), for an original's
/// `alloca` buffer; returns their address, or 0 when the allocation fails
/// (no panic in the game: the original's `alloca` of that size fails too,
/// at its stack guard).
fn zeroed(store: &mut Vec<u8>, n: u32) -> u32 {
    store.clear();
    let n = n as usize;
    if store.try_reserve_exact(n).is_err() {
        return 0;
    }
    store.resize(n, 0);
    u32::try_from(store.as_mut_ptr() as usize).unwrap_or(0)
}
