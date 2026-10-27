//! M5 stage 1, list `system-a`: the `<system_error>` categories and the
//! string streams' unwinding destructors (`ffx_system::m5a_std`), the
//! settings object and the PC menus' key-name tables
//! (`ffx_system::m5a_settings`).
//!
//! Threads: the main thread (settings at start-up, the menus) and
//! whichever thread unwinds a string stream; the shims keep no state,
//! frames are per call.

use super::{StackFrame, mem, ported};
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
}
