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
use ffx_system::m5a_assets::bridge as al;
use ffx_system::m5a_files::bridge as fl;
use ffx_system::m5a_fixup::bridge as fx;
use ffx_system::m5a_glue::bridge as gl;
use ffx_system::m5a_input::bridge as ip;
use ffx_system::m5a_platform::bridge as pf;
use ffx_system::m5a_savedata::{self as sv, bridge as sa};
use ffx_system::m5a_savedata2::{self as s2, bridge as s2b};
use ffx_system::m5a_savedata3::bridge as s3;
use ffx_system::m5a_settings::bridge as st;
use ffx_system::m5a_std::{self as sd, bridge as sb};
use ffx_system::m5a_ui::bridge as ui;
use ffx_system::m5a_vfs::bridge as vf;

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
    /// `(this)`.
    SyncManager_releaseData @ 0x006f_ad90 => extern "thiscall" fn sync_release_data(this: u32) -> u32 {
        pf::sync_release_data(&mut mem(), this)
    }
    /// `(this)`: the 5-byte `jmp SyncManager_releaseData`.
    code_006fa8c0 @ 0x006f_a8c0 => extern "thiscall" fn sync_release_thunk(this: u32) -> u32 {
        pf::sync_release_thunk(&mut mem(), this)
    }
    /// `(this; key, v, kbd)`.
    PInputMapper_setKey @ 0x0062_c990 => extern "thiscall" fn set_key(this: u32, key: u32, v: u32, kbd: u32) -> u32 {
        pf::set_key(&mut mem(), this, key, v, kbd)
    }
    /// `()` -> `ST0`.
    inputGetCurTime @ 0x0063_0c60 => extern "C" fn cur_time() -> f32 {
        pf::cur_time(&mut mem())
    }
    /// `(this)`: `eax` junk to the caller.
    InputManager_ClearHoldTimers @ 0x0063_1270 => extern "thiscall" fn clear_hold_timers(this: u32) {
        pf::clear_hold_timers(&mut mem(), this);
    }
    /// `(this)`.
    InputManager_UpdateHoldTimers @ 0x0063_30e0 => extern "thiscall" fn update_hold_timers(this: u32) -> u32 {
        pf::update_hold_timers(&mut mem(), this)
    }
    /// `()`.
    inputODUpdate @ 0x0063_0d50 => extern "C" fn od_update() -> u32 {
        pf::od_update(&mut mem())
    }
    /// `(this; n)`.
    PairArray_reserve_00631450 @ 0x0063_1450 => extern "thiscall" fn pair_array_reserve(this: u32, n: u32) -> u32 {
        pf::pair_array_reserve(&mut mem(), this, n)
    }
    /// `(this; other)`.
    PairArray_assign_00631110 @ 0x0063_1110 => extern "thiscall" fn pair_array_assign(this: u32, other: u32) -> u32 {
        pf::pair_array_assign(&mut mem(), this, other)
    }
    /// `SaveData_Init()`.
    SaveData_Init @ 0x0064_7fb0 => extern "C" fn save_data_init() -> u32 {
        pf::save_data_init(&mut mem())
    }
    /// `()`.
    TrophyManager_create_0067ac10 @ 0x0067_ac10 => extern "C" fn trophy_manager_create() -> u32 {
        pf::trophy_manager_create(&mut mem())
    }
    /// `()`.
    SaveDataManager_create_006f1c10 @ 0x006f_1c10 => extern "C" fn save_manager_create() -> u32 {
        pf::save_manager_create(&mut mem())
    }
    /// `StatsAndAchievements_Create()`.
    StatsAndAchievements_Create @ 0x006f_1430 => extern "C" fn stats_create() -> u32 {
        pf::stats_create(&mut mem())
    }
    /// `(this)`: the save manager's constructor.
    SaveDataManager_ctor_006f1980 @ 0x006f_1980 => extern "thiscall" fn save_manager_ctor(this: u32) -> u32 {
        pf::save_manager_ctor(&mut mem(), this)
    }
    /// `(this)`.
    PRedBlackTree_reset_0061d8e0 @ 0x0061_d8e0 => extern "thiscall" fn tree_reset(this: u32) -> u32 {
        pf::tree_reset(&mut mem(), this)
    }
    /// `(this; path, size)`.
    VbfStream_Open @ 0x0061_acc0 => extern "thiscall" fn vbf_stream_open(this: u32, path: u32, size: u32) -> u32 {
        pf::vbf_stream_open(&mut mem(), this, path, size)
    }
    /// `iSyncInitialize()`.
    iSyncInitialize @ 0x0067_a960 => extern "C" fn sync_initialize() -> u32 {
        pf::sync_initialize(&mut mem())
    }
    /// `()`.
    LocalizationManager_create_006dafc0 @ 0x006d_afc0 => extern "C" fn localization_create() -> u32 {
        pf::localization_create(&mut mem())
    }
    /// `(this; &digest)`: the copy at `ebp - 0x10`.
    Vbf_Contains @ 0x0061_dec0 => extern "thiscall" fn vbf_contains(this: u32, digest: u32) -> u32 {
        let mut f = StackFrame::<{ (pf::CONTAINS_FRAME / 4) as usize }>::new();
        pf::vbf_contains(&mut mem(), f.at() + pf::CONTAINS_FRAME, this, digest)
    }
    /// `()`.
    PClassLayoutInterface_static_0059e530 @ 0x0059_e530 => extern "C" fn layout_interface() -> u32 {
        pf::layout_interface(&mut mem())
    }
    /// `(this)`.
    BigFileStream_ctor @ 0x0061_bdd0 => extern "thiscall" fn big_file_stream_ctor(this: u32) -> u32 {
        pf::big_file_stream_ctor(&mut mem(), this)
    }
    /// `(this; n)`.
    BigFileStream_initSlots_0061c2c0 @ 0x0061_c2c0 => extern "thiscall" fn stream_slots_init(this: u32, n: u32) -> u32 {
        pf::stream_slots_init(&mut mem(), this, n)
    }
    /// `BigFileStream_createInstance()`.
    BigFileStream_createInstance @ 0x0061_b590 => extern "C" fn big_file_stream_create() -> u32 {
        pf::big_file_stream_create(&mut mem())
    }
    /// `(this; node)`.
    VbfTree_link_0061d210 @ 0x0061_d210 => extern "thiscall" fn digest_tree_link(this: u32, node: u32) -> u32 {
        pf::digest_tree_link(&mut mem(), this, node)
    }
    /// `(this; &digest, &entry)`.
    Vbf_InsertEntry @ 0x0061_de30 => extern "thiscall" fn vbf_insert_entry(this: u32, digest: u32, entry: u32) -> u32 {
        pf::vbf_insert_entry(&mut mem(), this, digest, entry)
    }
    /// `(a, b)`.
    Word4Greater_0060add0 @ 0x0060_add0 => extern "C" fn word4_greater_add0(a: u32, b: u32) -> u32 {
        pf::word4_greater(&mut mem(), a, b)
    }
    /// `(a, b)`: a copy of `0x0060add0`.
    Word4Greater_0060ae00 @ 0x0060_ae00 => extern "C" fn word4_greater_ae00(a: u32, b: u32) -> u32 {
        pf::word4_greater(&mut mem(), a, b)
    }
    /// `(rec, object, cursor, flags)`.
    Fixup_SetArrayObjectAndRead @ 0x0060_ae50 => extern "C" fn set_array_object_and_read(rec: u32, object: u32, cursor: u32, flags: u32) -> u32 {
        pf::set_array_object_and_read(&mut mem(), rec, object, cursor, flags)
    }
    /// `(rec, object, cursor, flags)`.
    Fixup_SetObjectAndRead @ 0x0060_ae90 => extern "C" fn set_object_and_read(rec: u32, object: u32, cursor: u32, flags: u32) -> u32 {
        pf::set_object_and_read(&mut mem(), rec, object, cursor, flags)
    }
    /// `(this; flags)`.
    DeletingDtor_0060e8a0 @ 0x0060_e8a0 => extern "thiscall" fn deleting_dtor_60e8a0(this: u32, flags: u32) -> u32 {
        pf::deleting_dtor_60e8a0(&mut mem(), this, flags)
    }
    /// `(&s, &out, &len)`.
    StringWithLength_0060e8e0 @ 0x0060_e8e0 => extern "C" fn string_with_length(sp: u32, out: u32, len: u32) -> u32 {
        pf::string_with_length(&mut mem(), sp, out, len)
    }
    /// `(out, name)`.
    FindEnum_0060e9b0 @ 0x0060_e9b0 => extern "C" fn find_enum(out: u32, name: u32) -> u32 {
        pf::find_enum(&mut mem(), out, name)
    }
    /// `(this; flags)`.
    DeletingDtor_0061a740 @ 0x0061_a740 => extern "thiscall" fn deleting_dtor_61a740(this: u32, flags: u32) -> u32 {
        pf::deleting_dtor_61a740(&mut mem(), this, flags)
    }
    /// `(this)`.
    VFile_fileClose @ 0x0061_aa50 => extern "thiscall" fn vfile_close(this: u32) -> u32 {
        pf::vfile_close(&mut mem(), this)
    }
    /// `(this)`.
    StreamSlot_ctor_0061af20 @ 0x0061_af20 => extern "thiscall" fn slot_ctor(this: u32) -> u32 {
        pf::slot_ctor(&mut mem(), this)
    }
    /// `(this)`.
    FreeBstr_0061b0b0 @ 0x0061_b0b0 => extern "thiscall" fn free_bstr(this: u32) -> u32 {
        pf::free_bstr(&mut mem(), this)
    }
    /// `(this, a, b, c)`.
    Forward_0061d0b0 @ 0x0061_d0b0 => extern "C" fn forward_61d030(this: u32, a: u32, b: u32, c: u32) -> u32 {
        pf::forward_61d030(&mut mem(), this, a, b, c)
    }
    /// `(p)`.
    SetFlag39_00624d30 @ 0x0062_4d30 => extern "C" fn set_flag39(p: u32) -> u32 {
        pf::set_flag39(&mut mem(), p)
    }
    /// `(instance, ctx)`.
    Input_EnumPadsCallback @ 0x0062_4d40 => extern "stdcall" fn enum_pads_callback(instance: u32, ctx: u32) -> u32 {
        let _ = ctx;
        pf::enum_pads_callback(&mut mem(), instance)
    }
    /// `(this)`.
    VFile_dtor_VFile @ 0x0061_a960 => extern "thiscall" fn vfile_dtor(this: u32) -> u32 {
        pf::vfile_dtor(&mut mem(), this)
    }
    /// `(this; vfile)`.
    BigFileHandle_releaseVFile @ 0x0061_df70 => extern "thiscall" fn release_vfile(this: u32, vfile: u32) -> u32 {
        pf::release_vfile(&mut mem(), this, vfile)
    }
    /// `(this)`.
    StreamSlotPool_term_0061af40 @ 0x0061_af40 => extern "thiscall" fn slot_pool_term(this: u32) -> u32 {
        pf::slot_pool_term(&mut mem(), this)
    }
    /// `(this; &digest)`: the copy at `ebp - 0x10`.
    BigFileHandle_getNewVFile @ 0x0061_df10 => extern "thiscall" fn get_new_vfile(this: u32, digest: u32) -> u32 {
        let mut f = StackFrame::<{ (pf::GET_VFILE_FRAME / 4) as usize }>::new();
        pf::get_new_vfile(&mut mem(), f.at() + pf::GET_VFILE_FRAME, this, digest)
    }
    /// `(this; id)`.
    ComBstr_loadString_0061b330 @ 0x0061_b330 => extern "thiscall" fn load_bstr(this: u32, id: u32) -> u32 {
        pf::load_bstr(&mut mem(), this, id)
    }
    /// `user_free_thunk(p)`: the 5-byte `jmp user_free`.
    user_free_thunk @ 0x0068_7010 => extern "C" fn user_free_thunk(p: u32) -> u32 {
        sa::user_free_thunk(&mut mem(), p)
    }
    /// `code_006f2740()`: the 5-byte `jmp SaveDataMachine_Update`.
    code_006f2740 @ 0x006f_2740 => extern "C" fn machine_update_thunk() -> u32 {
        sa::machine_update_thunk(&mut mem())
    }

    // ---- round 3: the input forwarders, clock, seek, voice glue, dtors ----
    /// `0x0062f5b0(out)`: frame `ebp - 0x14` (the `SYSTEMTIME`).
    Rng_ReadSystemClock_0062f5b0 @ 0x0062_f5b0 => extern "C" fn read_system_clock(out: u32) -> u32 {
        let mut f = StackFrame::<{ (gl::SYSTEM_TIME_FRAME / 4) as usize }>::new();
        gl::system_clock(&mut mem(), f.at() + gl::SYSTEM_TIME_FRAME, out)
    }
    /// `sceLseek(f, offset, whence)`: the jump thunk to `fiosLseek`.
    sceLseek @ 0x0062_f8e0 => extern "C" fn sce_lseek(f: u32, offset: u32, whence: u32) -> u64 {
        gl::fios_lseek(&mut mem(), f, offset, whence)
    }
    /// `0x00630b10()`.
    Input_ShutdownDevice_00630b10 @ 0x0063_0b10 => extern "C" fn input_shutdown_device() -> u32 {
        gl::input_shutdown(&mut mem())
    }
    /// `inputClear()`.
    inputClear @ 0x0063_0c30 => extern "C" fn input_clear() -> u32 {
        gl::input_clear(&mut mem())
    }
    /// `inputClearAllInputInfo()`.
    inputClearAllInputInfo @ 0x0063_0c40 => extern "C" fn clear_all_input_info() -> u32 {
        gl::clear_all_input_info(&mut mem())
    }
    /// `inputGetCurrDeviceButtonInfo()`.
    inputGetCurrDeviceButtonInfo @ 0x0063_0c80 => extern "C" fn button_info() -> u32 {
        gl::button_info(&mut mem())
    }
    /// `Input_AsyncPoll()`.
    Input_AsyncPoll @ 0x0063_0cb0 => extern "C" fn async_poll() -> u32 {
        gl::async_poll(&mut mem())
    }
    /// `inputIsReleaseButton(b)`.
    inputIsReleaseButton @ 0x0063_0d10 => extern "C" fn is_release_button(b: u32) -> u32 {
        gl::input_button(&mut mem(), gl::INPUT_IS_RELEASE, b)
    }
    /// `inputIsRepButton(b)`.
    inputIsRepButton @ 0x0063_0d30 => extern "C" fn is_rep_button(b: u32) -> u32 {
        gl::input_button(&mut mem(), gl::INPUT_IS_REP, b)
    }
    /// `0x00631230(this; keep)`.
    UserBuffer_reset_00631230 @ 0x0063_1230 => extern "thiscall" fn user_buffer_reset(this: u32, keep: u32) {
        gl::buffer_reset(&mut mem(), this, keep);
    }
    /// `0x00631060` (`ecx`; unwinding).
    UserBuffer_dtor_00631060 @ 0x0063_1060 => extern "thiscall" fn user_buffer_dtor(this: u32) -> u32 {
        gl::free_member8(&mut mem(), this)
    }
    /// `Virtuos_InputManager_setVibration(this; pad, small, large)`.
    Virtuos_InputManager_setVibration @ 0x0063_29c0 => extern "thiscall" fn set_vibration(this: u32, pad: u32, small: u32, large: u32) -> u32 {
        gl::set_vibration(&mut mem(), this, pad, small, large)
    }
    /// `0x00630f40` (`ecx` = the input manager `0x00ccb170`).
    Virtuos_InputManager_ctor_00630f40 @ 0x0063_0f40 => extern "thiscall" fn input_manager_ctor(this: u32) -> u32 {
        gl::controller_ctor(&mut mem(), this)
    }
    /// `0x00633f70` (`ecx`; unwinding).
    TreeSet_dtor_00633f70 @ 0x0063_3f70 => extern "thiscall" fn tree_set_dtor(this: u32) {
        gl::tree_set_dtor(&mut mem(), this);
    }
    /// `0x00634200` (`ecx`; unwinding).
    PString_release_00634200 @ 0x0063_4200 => extern "thiscall" fn pstring_release(this: u32) {
        gl::phyre_free_member_c(&mut mem(), this);
    }
    /// `0x00634340` (`ecx`; unwinding).
    Unwind_deleteMember0_00634340 @ 0x0063_4340 => extern "thiscall" fn delete_member0(this: u32) {
        gl::delete_member0(&mut mem(), this);
    }
    /// `0x00634360`: the 5-byte `jmp ListVector_dtor_00634220`.
    ListVector_dtor_thunk_00634360 @ 0x0063_4360 => extern "thiscall" fn list_vector_dtor_thunk(this: u32) -> u32 {
        gl::list_vector_dtor_thunk(&mut mem(), this)
    }
    /// `0x00634370` (`ecx`; unwinding).
    StdList_dtor_00634370 @ 0x0063_4370 => extern "thiscall" fn std_list_dtor(this: u32) {
        gl::list_dtor(&mut mem(), this);
    }
    /// `0x006343b0` (`ecx`; unwinding): frame `ebp - 4`.
    StdTree_dtor_006343b0 @ 0x0063_43b0 => extern "thiscall" fn std_tree_dtor_6343b0(this: u32) {
        let mut f = StackFrame::<1>::new();
        gl::tree_dtor(&mut mem(), f.at() + gl::TREE_DTOR_FRAME, gl::TREE_ERASE_636D60, this);
    }
    /// `0x006343e0` (`ecx`; unwinding): frame `ebp - 4`.
    StdTree_dtor_006343e0 @ 0x0063_43e0 => extern "thiscall" fn std_tree_dtor_6343e0(this: u32) {
        let mut f = StackFrame::<1>::new();
        gl::tree_dtor(&mut mem(), f.at() + gl::TREE_DTOR_FRAME, gl::TREE_ERASE_6370D0, this);
    }
    /// `0x00634500` (`ecx`; unwinding).
    StdVector_dtor_00634500 @ 0x0063_4500 => extern "thiscall" fn std_vector_dtor(this: u32) -> u32 {
        gl::vector_dtor(&mut mem(), this)
    }
    /// `SaveDataGetLoc(id, out)`.
    SaveDataGetLoc @ 0x0064_12d0 => extern "C" fn save_data_get_loc(id: u32, out: u32) -> u32 {
        gl::save_data_get_loc(&mut mem(), id, out)
    }
    /// `iVoiceVolumeDirect(v)`.
    iVoiceVolumeDirect @ 0x0067_a7b0 => extern "C" fn voice_volume_direct(v: u32) -> u32 {
        gl::voice_volume_direct(&mut mem(), v)
    }
    /// `iVoiceVolumeTranslate(v, t)`.
    iVoiceVolumeTranslate @ 0x0067_a7d0 => extern "C" fn voice_volume_translate(v: u32, t: u32) -> u32 {
        gl::voice_volume_translate(&mut mem(), v, t)
    }
    /// `0x0067a9b0(id, b, c, d)`.
    VoiceTable_selectForward_0067a9b0 @ 0x0067_a9b0 => extern "C" fn voice_select_forward(id: u32, b: u32, c: u32, d: u32) -> u32 {
        gl::voice_select_forward(&mut mem(), id, b, c, d)
    }
    /// `0x0067a9d0(a, b)`.
    VoiceTable_findForward_0067a9d0 @ 0x0067_a9d0 => extern "C" fn voice_find_forward(a: u32, b: u32) -> u32 {
        gl::voice_find_forward(&mut mem(), a, b)
    }
    /// `fiosLseek(f, offset, whence)`.
    fiosLseek @ 0x0067_9710 => extern "C" fn fios_lseek(f: u32, offset: u32, whence: u32) -> u64 {
        gl::fios_lseek(&mut mem(), f, offset, whence)
    }
    /// `0x00686bb0(next, prev)`.
    StdList_buyNode_00686bb0 @ 0x0068_6bb0 => extern "stdcall" fn buy_list_node(next: u32, prev: u32) -> u32 {
        gl::buy_list_node(&mut mem(), next, prev)
    }
    /// `vsnprintf1K_00687070(buffer, count, fmt, ...)`: the variadic
    /// words from the caller's fourth slot.
    vsnprintf1K_00687070 @ 0x0068_7070 => extern "C" fn vsnprintf_1k(buffer: u32, count: u32, fmt: u32; slots: u32) -> u32 {
        gl::vsnprintf_1k(&mut mem(), buffer, count, fmt, slots + 12)
    }
    /// `0x006fb380(this; id, b, c, d)`.
    VoiceTable_select_006fb380 @ 0x006f_b380 => extern "thiscall" fn voice_select(this: u32, id: u32, b: u32, c: u32, d: u32) -> u32 {
        gl::voice_select(&mut mem(), this, id, b, c, d)
    }
    /// `0x006fb410(this; a, b)`.
    VoiceTable_find_006fb410 @ 0x006f_b410 => extern "thiscall" fn voice_find(this: u32, a: u32, b: u32) -> u32 {
        gl::voice_find(&mut mem(), this, a, b)
    }
    /// `code_006793b0()`: the 5-byte `jmp PhyreFIOS_LoadPrxIdle`.
    code_006793b0 @ 0x0067_93b0 => extern "C" fn prx_idle_thunk() -> u32 {
        gl::jump_thunk(&mut mem(), gl::PRX_IDLE)
    }
    /// `code_006793c0()`: the 5-byte `jmp PhyreFIOS_LoadPrxInProgress`.
    code_006793c0 @ 0x0067_93c0 => extern "C" fn prx_in_progress_thunk() -> u32 {
        gl::jump_thunk(&mut mem(), gl::PRX_IN_PROGRESS)
    }
    /// `code_006793d0()`: the 5-byte `jmp Compare_006f9b70` (PRX state 3).
    code_006793d0 @ 0x0067_93d0 => extern "C" fn prx_loaded_thunk() -> u32 {
        gl::jump_thunk(&mut mem(), gl::PRX_LOADED)
    }

    // ---- round 3: the save manager's forwarders, load/save starts, debug save ----
    /// `SetUpDefaultSaveFolder()`: frame `ebp - 0x1004`.
    SetUpDefaultSaveFolder @ 0x006f_0330 => extern "C" fn set_up_default_save_folder() {
        let mut f = StackFrame::<{ (s2::bridge::FOLDER_FRAME / 4) as usize }>::new();
        s2b::set_up_default_save_folder(&mut mem(), f.at() + s2::bridge::FOLDER_FRAME);
    }
    /// `SaveDataManager_initSaveDataSys(this; buf, size)`: frame `ebp - 0x84`.
    SaveDataManager_initSaveDataSys @ 0x006f_1fa0 => extern "thiscall" fn init_save_data_sys(this: u32, buf: u32, size: u32) -> u32 {
        let mut f = StackFrame::<{ (s2::bridge::INIT_FRAME / 4) as usize }>::new();
        s2b::init_save_data_sys(&mut mem(), f.at() + s2::bridge::INIT_FRAME, this, buf, size)
    }
    /// `SaveDataManager_debugSave_Internal_6f0650(size, buf)`: frame `ebp - 0x404`.
    SaveDataManager_debugSave_Internal_6f0650 @ 0x006f_0510 => extern "C" fn debug_save_internal(size: u32, buf: u32) -> u32 {
        let mut f = StackFrame::<{ (s2::bridge::DEBUG_SAVE_FRAME / 4) as usize }>::new();
        s2b::debug_save_internal(&mut mem(), f.at() + s2::bridge::DEBUG_SAVE_FRAME, size, buf)
    }
    /// `0x006f1c40(this; _)`.
    SaveDataManager_checkDebugSave_006f1c40 @ 0x006f_1c40 => extern "thiscall" fn check_debug_save(this: u32, _x: u32) -> u32 {
        s2b::check_debug_save(&mut mem(), this)
    }
    /// `AutoSave_DebugSave_00647e10(x)`.
    AutoSave_DebugSave_00647e10 @ 0x0064_7e10 => extern "C" fn auto_debug_save(x: u32) -> u32 {
        s2b::auto_debug_save(&mut mem(), x)
    }
    /// `SaveDataManager_MarkAlbhedLoaded()`.
    SaveDataManager_MarkAlbhedLoaded @ 0x0064_7e40 => extern "C" fn mark_albhed_loaded() -> u32 {
        s2b::mark_albhed_loaded(&mut mem())
    }
    /// `SaveDataIsAlbhedLoaded()`.
    SaveDataIsAlbhedLoaded @ 0x0064_8020 => extern "C" fn is_albhed_loaded() -> u32 {
        s2b::is_albhed_loaded(&mut mem())
    }
    /// `SaveDataIsDeletingFinished()`.
    SaveDataIsDeletingFinished @ 0x0064_8060 => extern "C" fn is_deleting_finished() -> u32 {
        s2b::manager_call(&mut mem(), s2b::IS_DELETING)
    }
    /// `SaveDataIsLoadingFinished()`.
    SaveDataIsLoadingFinished @ 0x0064_8090 => extern "C" fn is_loading_finished() -> u32 {
        s2b::manager_call(&mut mem(), s2b::IS_LOADING)
    }
    /// `SaveDataIsSavingFinished()`.
    SaveDataIsSavingFinished @ 0x0064_80f0 => extern "C" fn is_saving_finished() -> u32 {
        s2b::manager_call(&mut mem(), s2b::IS_SAVING)
    }
    /// `SaveDataLocRemoveNumTag(src, dst)`.
    SaveDataLocRemoveNumTag @ 0x0064_8150 => extern "C" fn loc_remove_num_tag(src: u32, dst: u32) -> u32 {
        s2b::loc_remove_num_tag(&mut mem(), src, dst)
    }
    /// `SaveDataManager_preDebugSave(this; _)`.
    SaveDataManager_preDebugSave @ 0x006f_2410 => extern "thiscall" fn pre_debug_save(this: u32, _x: u32) -> u32 {
        s2b::pre_debug_save(&mut mem(), this)
    }
    /// `SaveDataPreDebugSave(x)`.
    SaveDataPreDebugSave @ 0x0064_84d0 => extern "C" fn pre_debug_save_forward(x: u32) -> u32 {
        s2b::pre_debug_save_forward(&mut mem(), x)
    }
    /// `SaveDataResetToIdle()`.
    SaveDataResetToIdle @ 0x0064_8530 => extern "C" fn reset_to_idle() -> u32 {
        s2b::manager_call(&mut mem(), s2b::RESET_IDLE)
    }
    /// `SaveDataManager_setSaveDataInfo(this; title, detail, text)`.
    SaveDataManager_setSaveDataInfo @ 0x006f_2560 => extern "thiscall" fn set_save_data_info(this: u32, title: u32, detail: u32, text: u32) -> u32 {
        s2b::set_save_data_info(&mut mem(), this, title, detail, text)
    }
    /// `SaveDataSetSaveDataInfo(title, detail, text)`.
    SaveDataSetSaveDataInfo @ 0x0064_86c0 => extern "C" fn set_save_data_info_forward(title: u32, detail: u32, text: u32) -> u32 {
        s2b::set_save_data_info_forward(&mut mem(), title, detail, text)
    }
    /// `SaveDataSetState(v)`.
    SaveDataSetState @ 0x0064_8710 => extern "C" fn set_state(v: u32) -> u32 {
        s2b::set_state(&mut mem(), v)
    }
    /// `SaveDataMachine_StartLoad(size, buf)`.
    SaveDataMachine_StartLoad @ 0x006f_01d0 => extern "C" fn start_load(size: u32, buf: u32) -> u32 {
        s2b::start_load(&mut mem(), 0xb, size, buf)
    }
    /// `SaveDataManager_doLoad(this)`.
    SaveDataManager_doLoad @ 0x006f_1ce0 => extern "thiscall" fn do_load(this: u32) -> u32 {
        s2b::do_load(&mut mem(), this)
    }
    /// `SaveDataToLoad()`.
    SaveDataToLoad @ 0x0064_8760 => extern "C" fn to_load() -> u32 {
        s2b::to_load(&mut mem())
    }
    /// `0x006efeb0(size, buf)`.
    SaveDataMachine_StartLoadAll_006efeb0 @ 0x006e_feb0 => extern "C" fn start_load_all(size: u32, buf: u32) -> u32 {
        s2b::start_load(&mut mem(), 0x15, size, buf)
    }
    /// `0x006f1d10(this)`.
    SaveDataManager_doLoadAll_006f1d10 @ 0x006f_1d10 => extern "thiscall" fn do_load_all(this: u32) -> u32 {
        s2b::load_all(&mut mem(), this)
    }
    /// `SaveDataToLoadAllAlBhed()`.
    SaveDataToLoadAllAlBhed @ 0x0064_8790 => extern "C" fn to_load_all() -> u32 {
        s2b::to_load_all(&mut mem())
    }
    /// `0x006f07a0(size, buf)`.
    SaveDataMachine_StartSave_006f07a0 @ 0x006f_07a0 => extern "C" fn start_save(size: u32, buf: u32) -> u32 {
        s2b::start_save(&mut mem(), size, buf)
    }
    /// `SaveDataManager_doSave(this)`.
    SaveDataManager_doSave @ 0x006f_1d70 => extern "thiscall" fn do_save(this: u32) -> u32 {
        s2b::do_save(&mut mem(), this)
    }
    /// `SaveDataToSave()`.
    SaveDataToSave @ 0x0064_87a0 => extern "C" fn to_save() -> u32 {
        s2b::to_save(&mut mem())
    }
    /// `SaveDataManager_setSaveDataIconString(this; s)`.
    SaveDataManager_setSaveDataIconString @ 0x006f_2530 => extern "thiscall" fn set_icon_string(this: u32, s: u32) -> u32 {
        s2b::set_icon_string(&mut mem(), this, s)
    }
    /// `debugLoad_Thunk_006492b0(slot, buf, size)`.
    debugLoad_Thunk_006492b0 @ 0x0064_92b0 => extern "C" fn debug_load_thunk(slot: u32, buf: u32, size: u32) -> u32 {
        s2b::debug_thunk(&mut mem(), s2b::DEBUG_LOAD, slot, buf, size)
    }
    /// `debugSavedataLoadAll(slot)`.
    debugSavedataLoadAll @ 0x0064_92c0 => extern "C" fn debug_load_all(slot: u32) -> u32 {
        s2b::debug_load_all(&mut mem(), slot)
    }
    /// `debugSave_Thunk_006492e0(slot, buf, size)`.
    debugSave_Thunk_006492e0 @ 0x0064_92e0 => extern "C" fn debug_save_thunk(slot: u32, buf: u32, size: u32) -> u32 {
        s2b::debug_thunk(&mut mem(), s2b::DEBUG_SAVE, slot, buf, size)
    }
    /// `0x006f2630(this)`.
    SaveDataManager_rebuildTable_006f2630 @ 0x006f_2630 => extern "thiscall" fn rebuild_table(this: u32) {
        s2b::rebuild_table(&mut mem(), this);
    }
    /// `0x006494d0(this; src, _)`.
    NamedValue_moveCtor_006494d0 @ 0x0064_94d0 => extern "thiscall" fn named_value_move_6494d0(this: u32, src: u32, _x: u32) -> u32 {
        s2b::string_move(&mut mem(), this, src)
    }
    /// `0x00649540(this; src, _)`.
    NamedValue_moveCtor_00649540 @ 0x0064_9540 => extern "thiscall" fn named_value_move_649540(this: u32, src: u32, _x: u32) -> u32 {
        s2b::string_move(&mut mem(), this, src)
    }
    /// `EternalCalm_FileSize_0067a9f0(name)`.
    EternalCalm_FileSize_0067a9f0 @ 0x0067_a9f0 => extern "C" fn calm_file_size(name: u32) -> u32 {
        s2b::calm_file_size(&mut mem(), name)
    }
    /// `getEternalCalmMgrpSize(map)`.
    getEternalCalmMgrpSize @ 0x0067_aa70 => extern "C" fn calm_mgrp_size(map: u32) -> u32 {
        s2b::calm_size(&mut mem(), s2::CALM_MGRP_SIZES, s2::CALM_MGRP_NAMES, map)
    }
    /// `getEternalCalmSceneSize(map)`.
    getEternalCalmSceneSize @ 0x0067_aae0 => extern "C" fn calm_scene_size(map: u32) -> u32 {
        s2b::calm_size(&mut mem(), s2::CALM_SCENE_SIZES, s2::CALM_SCENE_NAMES, map)
    }
    /// `WaterWaveController_setBuoyancyFix(c, v)`: `v` by its slot.
    WaterWaveController_setBuoyancyFix @ 0x006f_5760 => extern "stdcall" fn set_buoyancy_fix(c: u32, _v: u32; slots: u32) {
        s2b::set_one(&mut mem(), s2::BUOYANCY_FIX, c, slots);
    }
    /// `WaterWaveController_setCircleWaveStrength(c, a, b)`.
    WaterWaveController_setCircleWaveStrength @ 0x006f_5790 => extern "stdcall" fn set_circle_wave(c: u32, a: u32, b: u32) {
        s2b::set_two(&mut mem(), s2::CIRCLE_WAVE, c, a, b);
    }
    /// `WaterWaveController_setDraughtOffset(c, v)`: `v` by its slot.
    WaterWaveController_setDraughtOffset @ 0x006f_57e0 => extern "stdcall" fn set_draught_offset(c: u32, _v: u32; slots: u32) {
        s2b::set_one(&mut mem(), s2::DRAUGHT_OFFSET, c, slots);
    }
    /// `WaterWaveController_setFoamStrength(c, v)`: `v` by its slot.
    WaterWaveController_setFoamStrength @ 0x006f_5810 => extern "stdcall" fn set_foam_strength(c: u32, _v: u32; slots: u32) {
        s2b::set_one(&mut mem(), s2::FOAM_STRENGTH, c, slots);
    }
    /// `WaterWaveController_setMultiTexBlendFactor(c, v)`: `v` by its slot.
    WaterWaveController_setMultiTexBlendFactor @ 0x006f_5840 => extern "stdcall" fn set_multi_tex_blend(c: u32, _v: u32; slots: u32) {
        s2b::set_one(&mut mem(), s2::MULTI_TEX_BLEND, c, slots);
    }
    /// `WaterWaveController_setScrollSpeed(c, x0, y0, x1, y1)`: by the slots.
    WaterWaveController_setScrollSpeed @ 0x006f_5870 => extern "stdcall" fn set_scroll_speed(c: u32, _x0: u32, _y0: u32, _x1: u32, _y1: u32; slots: u32) {
        s2b::set_scroll_speed(&mut mem(), c, slots);
    }
    /// `WaterWaveController_setSineWaveStrength(c, a, b)`.
    WaterWaveController_setSineWaveStrength @ 0x006f_59a0 => extern "stdcall" fn set_sine_wave(c: u32, a: u32, b: u32) {
        s2b::set_two(&mut mem(), s2::SINE_WAVE, c, a, b);
    }

    // ---- round 3: the cluster pointer-fixup unpackers and their runs ----
    /// `Fixup_RunBitmask16(desc; cb, tmpl, &cursor, &out)`.
    Fixup_RunBitmask16 @ 0x0060_cac0 => extern "thiscall" fn run_bitmask16(desc: u32, cb: u32, tmpl: u32, cursor: u32, out: u32) {
        fx::run_bitmask(&mut mem(), fx::Run { desc, cb, tmpl, cursor, out, size: 0x10 });
    }
    /// `Fixup_RunExcept16(desc; cb, tmpl, &cursor, &out)`.
    Fixup_RunExcept16 @ 0x0060_cc20 => extern "thiscall" fn run_except16(desc: u32, cb: u32, tmpl: u32, cursor: u32, out: u32) -> u32 {
        fx::run_except(&mut mem(), fx::Run { desc, cb, tmpl, cursor, out, size: 0x10 })
    }
    /// `Fixup_RunListed16(desc; cb, tmpl, &cursor, &out)`.
    Fixup_RunListed16 @ 0x0060_ce70 => extern "thiscall" fn run_listed16(desc: u32, cb: u32, tmpl: u32, cursor: u32, out: u32) -> u32 {
        fx::run_listed(&mut mem(), fx::Run { desc, cb, tmpl, cursor, out, size: 0x10 })
    }
    /// `Fixup_RunArithmetic16(desc; cb, tmpl, &cursor, &out)`.
    Fixup_RunArithmetic16 @ 0x0060_d000 => extern "thiscall" fn run_arithmetic16(desc: u32, cb: u32, tmpl: u32, cursor: u32, out: u32) -> u32 {
        fx::run_arithmetic(&mut mem(), fx::Run { desc, cb, tmpl, cursor, out, size: 0x10 })
    }
    /// `Fixup_RunBitmask(desc; cb, tmpl, &cursor, &out)`.
    Fixup_RunBitmask @ 0x0060_cb70 => extern "thiscall" fn run_bitmask(desc: u32, cb: u32, tmpl: u32, cursor: u32, out: u32) {
        fx::run_bitmask(&mut mem(), fx::Run { desc, cb, tmpl, cursor, out, size: 0x1c });
    }
    /// `Fixup_RunExcept(desc; cb, tmpl, &cursor, &out)`.
    Fixup_RunExcept @ 0x0060_cd50 => extern "thiscall" fn run_except(desc: u32, cb: u32, tmpl: u32, cursor: u32, out: u32) -> u32 {
        fx::run_except(&mut mem(), fx::Run { desc, cb, tmpl, cursor, out, size: 0x1c })
    }
    /// `Fixup_RunListed(desc; cb, tmpl, &cursor, &out)`.
    Fixup_RunListed @ 0x0060_cf40 => extern "thiscall" fn run_listed(desc: u32, cb: u32, tmpl: u32, cursor: u32, out: u32) -> u32 {
        fx::run_listed(&mut mem(), fx::Run { desc, cb, tmpl, cursor, out, size: 0x1c })
    }
    /// `Fixup_RunArithmetic(desc; cb, tmpl, &cursor, &out)`.
    Fixup_RunArithmetic @ 0x0060_d080 => extern "thiscall" fn run_arithmetic(desc: u32, cb: u32, tmpl: u32, cursor: u32, out: u32) -> u32 {
        fx::run_arithmetic(&mut mem(), fx::Run { desc, cb, tmpl, cursor, out, size: 0x1c })
    }
    /// `Fixup_UnpackArrayList(&cursor, records, n, count)`: frame `ebp -
    /// 0x44`; the output cursor is the caller's `records` slot.
    Fixup_UnpackArrayList @ 0x0060_a340 => extern "C" fn unpack_array_list(cursor: u32, records: u32, n: u32, count: u32; slots: u32) {
        let mut f = StackFrame::<{ (fx::ARRAY_FRAME / 4) as usize }>::new();
        fx::unpack_array_list(&mut mem(), f.at() + fx::ARRAY_FRAME, cursor, records, slots + 4, n, count);
    }
    /// `Fixup_UnpackList(&cursor, records, n, count)`: frame `ebp -
    /// 0x58`; the output cursor is the caller's `records` slot.
    Fixup_UnpackList @ 0x0060_a750 => extern "C" fn unpack_list(cursor: u32, records: u32, n: u32, count: u32; slots: u32) {
        let mut f = StackFrame::<{ (fx::LIST_FRAME / 4) as usize }>::new();
        fx::unpack_list(&mut mem(), f.at() + fx::LIST_FRAME, cursor, records, slots + 4, n, count);
    }

    // ---- round 3: archive read requests, VFile, member caches, tree dtors ----
    /// `0x00573b70(L)`: the Lua vector's negation.
    PVector4_luaUnm_00573b70 @ 0x0057_3b70 => extern "C" fn lua_vector_negate(lua: u32) -> u32 {
        fl::lua_vector_negate(&mut mem(), lua)
    }
    /// `0x0057a200(this; &cursor, a, b, c)`.
    MemberCache_findMember_0057a200 @ 0x0057_a200 => extern "thiscall" fn cached_find_member(this: u32, cursor: u32, a: u32, b: u32, c: u32) -> u32 {
        fl::cached_find_member(&mut mem(), this, cursor, a, b, c)
    }
    /// `Fixup_FindMember(this; index, &cursor, a, &element, b)`.
    Fixup_FindMember @ 0x0057_a280 => extern "thiscall" fn fixup_find_member(this: u32, index: u32, cursor: u32, a: u32, element: u32, b: u32) -> u32 {
        fl::fixup_find_member(&mut mem(), this, index, cursor, a, element, b)
    }
    /// `0x0060ac50(a, b)`: a sort predicate.
    FixupRecord_less_0060ac50 @ 0x0060_ac50 => extern "C" fn record_less_60ac50(a: u32, b: u32) -> u32 {
        fl::record_less(&mem(), &fl::LESS_60AC50, a, b)
    }
    /// `0x0060ad00(a, b)`: a sort predicate.
    FixupRecord_less_0060ad00 @ 0x0060_ad00 => extern "C" fn record_less_60ad00(a: u32, b: u32) -> u32 {
        fl::record_less(&mem(), &fl::LESS_60AD00, a, b)
    }
    /// `0x0060d460` (`ecx`; unwinding).
    PRedBlackTree_dtor_0060d460 @ 0x0060_d460 => extern "thiscall" fn tree_dtor_60d460(this: u32) {
        fl::tree_dtor(&mut mem(), this);
    }
    /// `0x0060d550` (`ecx`; unwinding).
    PooledTree_dtor_0060d550 @ 0x0060_d550 => extern "thiscall" fn pooled_tree_dtor(this: u32) {
        fl::pooled_tree_dtor(&mut mem(), this);
    }
    /// `0x0060d5d0` (`ecx`; unwinding).
    OwnedArrayTree_dtor_0060d5d0 @ 0x0060_d5d0 => extern "thiscall" fn owned_array_dtor(this: u32) {
        fl::owned_array_dtor(&mut mem(), this);
    }
    /// `0x0061a690(this)`: frame `ebp - 0x40` (the identity it converts).
    Phyre_PWorldMatrixController_ctor_0061a690 @ 0x0061_a690 => extern "thiscall" fn world_matrix_controller_ctor(this: u32) -> u32 {
        let mut f = StackFrame::<{ (fl::MATRIX_FRAME / 4) as usize }>::new();
        fl::controller_ctor(&mut mem(), f.at() + fl::MATRIX_FRAME, this)
    }
    /// `0x0061a770(this; out)` (vtable slot 4): frame `ebp - 0x44`.
    Phyre_PWorldMatrixController_getAxis_0061a770 @ 0x0061_a770 => extern "thiscall" fn world_matrix_controller_axis(this: u32, out: u32) -> u32 {
        let mut f = StackFrame::<{ (fl::AXIS_FRAME / 4) as usize }>::new();
        fl::controller_axis(&mut mem(), f.at() + fl::AXIS_FRAME, this, out)
    }
    /// `VFile_VFile(this)`.
    VFile_VFile @ 0x0061_a860 => extern "thiscall" fn vfile_ctor(this: u32) -> u32 {
        fl::vfile_ctor(&mut mem(), this)
    }
    /// `0x0061b030(this)`.
    VFile_freeBuffers_0061b030 @ 0x0061_b030 => extern "thiscall" fn vfile_free_buffers(this: u32) {
        fl::vfile_free_buffers(&mut mem(), this);
    }
    /// `0x0061b270(id)`.
    AtlFindStringResource_0061b270 @ 0x0061_b270 => extern "C" fn find_string_resource(id: u32) -> u32 {
        fl::find_string_resource(&mut mem(), id)
    }
    /// `0x0061b740(this; lo, hi, size, total)`.
    BigFileStream_setUpSlot_0061b740 @ 0x0061_b740 => extern "thiscall" fn set_up_slot(this: u32, lo: u32, hi: u32, size: u32, total: u32) -> u32 {
        fl::set_up_slot(&mut mem(), this, lo, hi, size, total)
    }
    /// `FileSys_NewReadRequest(this; file, length, req, lo, hi, info)`.
    FileSys_NewReadRequest @ 0x0061_b7e0 => extern "thiscall" fn new_read_request(this: u32, file: u32, length: u32, req: u32, lo: u32, hi: u32, info: u32) -> u32 {
        fl::new_read_request(&mut mem(), this, file, length, req, lo, hi, info)
    }

    // ---- round 3: the archive name lookups, the synchronous read, the VFile pool ----
    /// `FileSys_Exists(this; name)`: frame `ebp - 0x1c`.
    FileSys_Exists @ 0x0061_be40 => extern "thiscall" fn file_sys_exists(this: u32, name: u32) -> u8 {
        let mut f = StackFrame::<{ (vf::DIGEST_FRAME / 4) as usize }>::new();
        vf::exists(&mut mem(), f.at() + vf::DIGEST_FRAME, this, name)
    }
    /// `BigFileStream_openFile(this; name)`: frame `ebp - 0x1c`.
    BigFileStream_openFile @ 0x0061_bf10 => extern "thiscall" fn big_file_open(this: u32, name: u32) -> u32 {
        let mut f = StackFrame::<{ (vf::DIGEST_FRAME / 4) as usize }>::new();
        vf::open_file(&mut mem(), f.at() + vf::DIGEST_FRAME, this, name)
    }
    /// `BigFileStream_readVFile(this; file, length, req, lo, hi, info)`.
    BigFileStream_readVFile @ 0x0061_bfe0 => extern "thiscall" fn read_vfile(this: u32, file: u32, length: u32, req: u32, lo: u32, hi: u32, info: u32) -> u32 {
        vf::read_vfile(&mut mem(), this, file, length, req, lo, hi, info)
    }
    /// `0x0061d9c0(this; digest)`.
    BigFileHandle_newVFile_0061d9c0 @ 0x0061_d9c0 => extern "thiscall" fn new_vfile(this: u32, digest: u32) -> u32 {
        vf::new_vfile(&mut mem(), this, digest)
    }
    /// `BigFileStream_fileTaskRead(_, slot, _)` (stdcall).
    BigFileStream_fileTaskRead @ 0x0061_b8d0 => extern "stdcall" fn task_read(_a: u32, slot: u32, _c: u32) {
        vf::task_read(&mut mem(), slot);
    }
    /// `0x0061add0(this)`: a read slot's constructor.
    BigFileStream_slotCtor_0061add0 @ 0x0061_add0 => extern "thiscall" fn read_slot_ctor(this: u32) -> u32 {
        vf::slot_ctor(&mut mem(), this)
    }

    // ---- round 3: DirectInput's start, the archive handles' set-up ----
    /// `Input_InitDirectInput()`.
    Input_InitDirectInput @ 0x0062_4350 => extern "C" fn init_direct_input() -> u32 {
        ip::init_direct_input(&mut mem())
    }
    /// `Input_EnumDevicesCallback(instance, ctx)`.
    Input_EnumDevicesCallback @ 0x0062_3330 => extern "stdcall" fn enum_devices_callback(instance: u32, _ctx: u32) -> u32 {
        ip::enum_devices_callback(&mut mem(), instance)
    }
    /// `Input_EnumAxesSetRange(object, device)`: frame `ebp - 0x1c`.
    Input_EnumAxesSetRange @ 0x0062_33a0 => extern "stdcall" fn enum_axes_set_range(object: u32, device: u32) -> u32 {
        let mut f = StackFrame::<{ (ip::RANGE_FRAME / 4) as usize }>::new();
        ip::enum_axes_set_range(&mut mem(), f.at() + ip::RANGE_FRAME, object, device)
    }
    /// `0x00623420(out)`.
    IntrusiveList_copy_00623420 @ 0x0062_3420 => extern "C" fn copy_list(out: u32) -> u32 {
        ip::copy_list(&mut mem(), out)
    }
    /// `0x00623da0(s, prefix)`.
    CmdLine_afterPrefix_00623da0 @ 0x0062_3da0 => extern "C" fn after_prefix(s: u32, prefix: u32) -> u32 {
        ip::after_prefix(&mut mem(), s, prefix)
    }
    /// `BigFileHandle_BigFileHandle(this)`.
    BigFileHandle_BigFileHandle @ 0x0061_d460 => extern "thiscall" fn big_file_handle_ctor(this: u32) -> u32 {
        ip::handle_ctor(&mut mem(), this)
    }
    /// `BigFileStream_registerBigFile(this; path)`.
    BigFileStream_registerBigFile @ 0x0061_c150 => extern "thiscall" fn register_big_file(this: u32, path: u32) -> u8 {
        ip::register_big_file(&mut mem(), this, path)
    }
    /// `Steam_Init()`.
    Steam_Init @ 0x0062_3470 => extern "C" fn steam_init() -> u32 {
        ip::steam_init(&mut mem())
    }
    /// `0x006255d0(table, name, default)`.
    CmdLine_findOption_006255d0 @ 0x0062_55d0 => extern "C" fn find_option(table: u32, name: u32, default: u32) -> u32 {
        ip::find_option(&mut mem(), table, name, default)
    }
    /// `Input_Shutdown()`.
    Input_Shutdown @ 0x0062_5680 => extern "C" fn input_shutdown_all() -> u32 {
        ip::input_shutdown(&mut mem())
    }
    /// `Settings_SetThreadUILanguage()`.
    Settings_SetThreadUILanguage @ 0x0062_cad0 => extern "C" fn set_thread_ui_language() -> u32 {
        ip::set_thread_ui_language(&mut mem())
    }
    /// `0x00624810(app, key)`: a script key-down command.
    ScriptKey_down_00624810 @ 0x0062_4810 => extern "C" fn script_key_down(app: u32, key: u32) -> u32 {
        ip::set_script_key(&mut mem(), app, key, 1)
    }
    /// `0x00624850(app, key)`: a script key-up command.
    ScriptKey_up_00624850 @ 0x0062_4850 => extern "C" fn script_key_up(app: u32, key: u32) -> u32 {
        ip::set_script_key(&mut mem(), app, key, 0)
    }
    /// `0x00624d70(obj, arg)`: a script command.
    ScriptCommand_00624d70 @ 0x0062_4d70 => extern "C" fn script_624d70(obj: u32, arg: u32) -> u32 {
        ip::script_624d70(&mut mem(), obj, arg)
    }
    /// `Input_AddKeyboard(instance)`: frame `ebp - 4`.
    Input_AddKeyboard @ 0x0062_3fa0 => extern "C" fn add_keyboard(instance: u32) {
        let mut f = StackFrame::<{ (ip::KEYBOARD_FRAME / 4) as usize }>::new();
        ip::add_keyboard(&mut mem(), f.at() + ip::KEYBOARD_FRAME, instance);
    }
    /// `Input_AddMouse(instance)`: frame `ebp - 0x10`.
    Input_AddMouse @ 0x0062_4020 => extern "C" fn add_mouse(instance: u32) {
        let mut f = StackFrame::<{ (ip::MOUSE_FRAME / 4) as usize }>::new();
        ip::add_mouse(&mut mem(), f.at() + ip::MOUSE_FRAME, instance);
    }
    /// `0x00624890(app, key)`: a script key-press command.
    ScriptKey_press_00624890 @ 0x0062_4890 => extern "C" fn script_key_press(app: u32, key: u32) -> u32 {
        ip::script_key_press(&mut mem(), app, key)
    }
    /// `0x00624910(app, text)`: frame `ebp - 8`.
    ScriptMouse_set_00624910 @ 0x0062_4910 => extern "C" fn script_mouse_set(app: u32, text: u32) -> u32 {
        let mut f = StackFrame::<{ (ip::MOUSE_SET_FRAME / 4) as usize }>::new();
        ip::script_mouse_set(&mut mem(), f.at() + ip::MOUSE_SET_FRAME, app, text)
    }

    // ---- round 3: the asset list reader, the Steam stats ctor, FileSys_Read ----
    /// `AssetListReader_copy_0064d9e0(this; src)`: frame `ebp - 0x14`.
    AssetListReader_copy_0064d9e0 @ 0x0064_d9e0 => extern "thiscall" fn asset_list_copy(this: u32, src: u32) -> u32 {
        let mut f = StackFrame::<{ (al::COPY_FRAME / 4) as usize }>::new();
        al::copy(&mut mem(), f.at() + al::COPY_FRAME, this, src)
    }
    /// `AssetListReader_dtor_AssetListReader(this)`: frame `ebp - 4`.
    AssetListReader_dtor_AssetListReader @ 0x0063_4610 => extern "thiscall" fn asset_list_dtor(this: u32) {
        let mut f = StackFrame::<1>::new();
        al::dtor(&mut mem(), f.at() + al::DTOR_FRAME, this);
    }
    /// `StatsAndAchievements_ctor(this)`.
    StatsAndAchievements_ctor @ 0x006f_11a0 => extern "thiscall" fn stats_ctor(this: u32) -> u32 {
        al::stats_ctor(&mut mem(), this)
    }
    /// `FileSys_Read(vfile; dst, len)`.
    FileSys_Read @ 0x0061_aa90 => extern "thiscall" fn file_sys_read(vfile: u32, dst: u32, len: u32) -> u32 {
        al::file_read(&mut mem(), vfile, dst, len)
    }
    /// `PApplicationScriptProxy_buttonRelease(app, name)`.
    PApplicationScriptProxy_buttonRelease @ 0x0062_2ed0 => extern "C" fn button_release(app: u32, name: u32) -> u32 {
        ip::button_release(&mut mem(), app, name)
    }
    /// `0x00623060(app, name)`: the press command.
    PApplicationScriptProxy_buttonPress_00623060 @ 0x0062_3060 => extern "C" fn button_press(app: u32, name: u32) -> u32 {
        ip::button_press(&mut mem(), app, name)
    }
    /// `Input_AddPadDevice(instance)`: frame `ebp - 0x10`.
    Input_AddPadDevice @ 0x0062_40e0 => extern "C" fn add_pad_device(instance: u32) {
        let mut f = StackFrame::<{ (ip::PAD_FRAME / 4) as usize }>::new();
        ip::add_pad_device(&mut mem(), f.at() + ip::PAD_FRAME, instance);
    }
    /// `0x0067ad30(this)`: the graphic UI object's constructor.
    GraphicUi_ctor_0067ad30 @ 0x0067_ad30 => extern "thiscall" fn graphic_ui_ctor(this: u32) -> u32 {
        ui::ctor(&mut mem(), this)
    }
    /// `SaveDataMachine_ScanFiles()`: frame `ebp - 0x4dc`.
    SaveDataMachine_ScanFiles @ 0x006f_0a70 => extern "C" fn scan_files() -> u32 {
        let mut f = StackFrame::<{ (s3::SCAN_FRAME / 4) as usize }>::new();
        s3::scan_files(&mut mem(), f.at() + s3::SCAN_FRAME)
    }
    /// `SaveDataMachine_ReadHeaders(first, end)`: frame `ebp - 0x444`.
    SaveDataMachine_ReadHeaders @ 0x006f_0c60 => extern "C" fn read_headers(first: u32, end: u32) {
        let mut f = StackFrame::<{ (s3::HEADERS_FRAME / 4) as usize }>::new();
        s3::read_headers(&mut mem(), f.at() + s3::HEADERS_FRAME, first, end);
    }
    /// `BigFileHandle_initialize(this; path)`: frame `ebp - 0x54`; the
    /// header size goes to the caller's `path` slot.
    BigFileHandle_initialize @ 0x0061_dac0 => extern "thiscall" fn big_file_handle_initialize(this: u32, path: u32; slots: u32) -> u8 {
        let mut f = StackFrame::<{ (vf::INIT_FRAME / 4) as usize }>::new();
        vf::initialize(&mut mem(), f.at() + vf::INIT_FRAME, this, path, slots)
    }
    /// `BigFileStream_fileTaskUncompress(_, slot, _, _)`: frame `ebp -
    /// 0x10`; the error text goes to the caller's third slot.
    BigFileStream_fileTaskUncompress @ 0x0061_ba80 => extern "stdcall" fn task_uncompress(_a: u32, slot: u32, _c: u32, _d: u32; slots: u32) {
        let mut f = StackFrame::<{ (vf::UNCOMPRESS_FRAME / 4) as usize }>::new();
        vf::task_uncompress(&mut mem(), f.at() + vf::UNCOMPRESS_FRAME, slot, slots);
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
