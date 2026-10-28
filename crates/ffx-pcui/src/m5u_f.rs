//! M5 stage 1, list `pc-ui`, batch 6: the Esc menu's audio refresh, its
//! member-call trampolines, the key bindings reset, the key-incomplete
//! pop-up queue, an Iggy string read and the save menu's enable.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the originals' argument words
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    use crate::m5u_a::bridge::OPERATOR_DELETE;
    use crate::m5u_c::bridge::{
        GET_DEREF, HANDLERS_ADD_CALL, IGGY_MENU_SET_ENABLE, SETTINGS, VALUE_REF_CREATE,
        VALUE_REF_FREE,
    };

    pub const RT_DYNAMIC_CAST: u32 = 0x0094_9d86;
    /// The `IggyMenu` and `FFEscMenu` RTTI descriptors the cast takes.
    pub const RTTI_IGGY_MENU: u32 = 0x00c3_3f60;
    pub const RTTI_ESC_MENU: u32 = 0x00c3_3f78;
    pub const GET_ARRAY_S32: u32 = 0x00a2_8b20;
    pub const SET_WORD_AT: u32 = 0x0040_2a00;
    /// The audio sliders (4 records of 0x18 bytes: `+0` value index, `+4`
    /// text, `+8` maximum, `+0xc` settings key, `+0x10` name, `+0x14`
    /// handler).
    pub const AUDIO_SLIDERS: u32 = 0x00c3_9628;
    pub const AUDIO_REFRESHED: u32 = 0x006f_f760;
    pub const GET_APPLICATION: u32 = 0x0062_3b90;
    pub const INPUT_SET_KEY: u32 = 0x0062_c990;
    pub const SWAP_STATE: u32 = 0x0063_0df0;
    pub const KEY_INCOMPLETE_POPUP: u32 = 0x0070_1470;
    /// `iggy_w32`: `IggyValuePathMakeArrayRef@12`,
    /// `IggyValueGetStringUTF8RS@24` (stdcall).
    pub const PATH_ARRAY_IAT: u32 = 0x00b0_c830;
    pub const GET_STRING_UTF8_IAT: u32 = 0x00b0_c858;
    pub const SAVE_LOAD_MENU: u32 = 0x00ce_b248;

    fn rd(h: &impl Host, at: u32) -> u32 {
        h.read(at)
    }

    fn g(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    /// `FFEscMenu_onAudioSettingRefresh(menu, args)`: the call's four
    /// slider values (an `IggyValueRef` at `frame`, the original's `ebp -
    /// 0x5c`, from the menu cast to its `IggyMenu`; values kept by index
    /// at `frame + 0x3c`) written to the settings, then
    /// `0x006ff760` queued on the menu.
    pub fn on_audio_setting_refresh(h: &mut impl Host, menu: u32, args: u32, frame: u32) -> u32 {
        let iggy = h.rebase(RTTI_IGGY_MENU);
        let esc = h.rebase(RTTI_ESC_MENU);
        let cast = h.call_sym(RT_DYNAMIC_CAST, Conv::Cdecl, &[menu, 0, iggy, esc, 0]);
        let temp = rd(h, args + 4);
        let player = h.call_sym(GET_DEREF, Conv::Thiscall(cast), &[]);
        h.call_sym(VALUE_REF_CREATE, Conv::Thiscall(frame), &[player, temp]);
        let table = h.rebase(AUDIO_SLIDERS);
        let values = frame + 0x3c;
        for k in 0..4 {
            let idx = rd(h, table + 0x18 * k);
            let v = h.call_sym(GET_ARRAY_S32, Conv::Thiscall(frame), &[idx]);
            let idx = rd(h, table + 0x18 * k);
            h.write(values.wrapping_add(idx.wrapping_mul(4)), v);
        }
        let settings = g(h, SETTINGS);
        for k in 0..4 {
            let idx = rd(h, table + 0x18 * k);
            let v = rd(h, values.wrapping_add(idx.wrapping_mul(4)));
            let key = rd(h, table + 0x18 * k + 0xc);
            h.call_sym(SET_WORD_AT, Conv::Thiscall(settings), &[key, v]);
        }
        let f = h.rebase(AUDIO_REFRESHED);
        h.call_sym(HANDLERS_ADD_CALL, Conv::Thiscall(menu), &[f, 0]);
        h.call_sym(VALUE_REF_FREE, Conv::Thiscall(frame), &[])
    }

    /// `FFEscMenu_callMember1_006ffbb0(this, f, arg)`: `f(this; arg)` (a
    /// member function, thiscall with one word); its `eax`. `frame` is the
    /// original's `ebp - 8` (its first word zeroed, unused).
    pub fn call_member1(h: &mut impl Host, this: u32, f: u32, arg: u32, frame: u32) -> u32 {
        h.write(frame, 0u32);
        h.call(f, Conv::Thiscall(this), &[arg])
    }

    /// `FFEscMenu_callBoundMember_006ffc30(bound)`: `bound[0](bound[2] +
    /// bound[1])` (a member pointer with its `this` adjustment), then the
    /// record deleted.
    pub fn call_bound_member(h: &mut impl Host, bound: u32) {
        let this = rd(h, bound + 8).wrapping_add(rd(h, bound + 4));
        let f = rd(h, bound);
        h.call(f, Conv::Thiscall(this), &[]);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[bound]);
    }

    /// `FFEscMenu_clearKeys_006ffc50()`: keys 0..=0x7b unbound
    /// (`PInputMapper_setKey(application; key, 0, 0)`).
    pub fn clear_keys(h: &mut impl Host) {
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        for key in 0..=0x7b {
            h.call_sym(INPUT_SET_KEY, Conv::Thiscall(app), &[key, 0, 0]);
        }
    }

    /// `FFEscMenu_resetInputState_00701930(this)`: `+0xfc` = 0 and the
    /// input manager's state swapped for zeros (`inputSwapState_00630df0(
    /// frame, 0, 0, 0)`, `frame` the original's `ebp - 0xc`).
    pub fn reset_input_state(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        h.write(this + 0xfc, 0u8);
        h.call_sym(SWAP_STATE, Conv::Cdecl, &[frame, 0, 0, 0])
    }

    /// `FFEscMenu_onKeyIncomplete_00701660(menu)`:
    /// `FFEscMenu_ShowKeyIncompletePopUp_00701470(menu)` queued.
    pub fn on_key_incomplete(h: &mut impl Host, menu: u32) -> u32 {
        let f = h.rebase(KEY_INCOMPLETE_POPUP);
        h.call_sym(HANDLERS_ADD_CALL, Conv::Thiscall(menu), &[f, menu])
    }

    /// `IggyValue_getStringUTF8_00a28b60(this; i, buf, size)` (`ret 0xc`)
    /// -> the length: element `i` of the array value `this + 8` (a path at
    /// `frame`, the original's `ebp - 0x18`) as UTF-8 into `buf`; the
    /// length goes through the argument slot `slot` (the original's `i`).
    pub fn get_string_utf8(
        h: &mut impl Host,
        this: u32,
        i: u32,
        buf: u32,
        size: u32,
        slot: u32,
        frame: u32,
    ) -> u32 {
        import(h, PATH_ARRAY_IAT, Conv::Stdcall, &[frame, this + 8, i]);
        import(
            h,
            GET_STRING_UTF8_IAT,
            Conv::Stdcall,
            &[frame, 0, 0, size, buf, slot],
        );
        rd(h, slot)
    }

    /// `IggyMenu_setEnableCeb248_007029e0(on)`: the save/load menu, when
    /// there is one, enabled (`on` != 0) or disabled; its `eax` (0 without
    /// one: the original's is its caller's).
    pub fn save_menu_set_enable(h: &mut impl Host, on: u32) -> u32 {
        let menu = g(h, SAVE_LOAD_MENU);
        if menu == 0 {
            return 0;
        }
        h.call_sym(
            IGGY_MENU_SET_ENABLE,
            Conv::Thiscall(menu),
            &[u32::from(on != 0)],
        )
    }
}
