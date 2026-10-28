//! M5 stage 1, list `pc-ui`, batch 5: opening an Iggy player (the
//! `.swf` path unified, the `IggyMenu` made at a rectangle, the three
//! game callbacks registered) and the Esc menu's settings pages.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the originals' argument words
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    use crate::m5u_a::bridge::{OPERATOR_NEW, UNIFY_FILENAME};
    use crate::m5u_c::bridge::IGGY_MENU_CTOR;

    pub const IGGY_ADD_CALLBACK: u32 = 0x00a2_8e90;
    /// The two divisors of `IggyPlayerOpenScaled` (`f64`): heights by
    /// the first, widths by the second (the reference screen).
    pub const SCALE_H: u32 = 0x00b4_1d48;
    pub const SCALE_W: u32 = 0x00b4_1d60;
    /// The callbacks every player gets: `(name, function)`.
    pub const CALLBACKS: [(u32, u32); 3] = [
        (0x00b4_b518, 0x006e_9590),
        (0x00b4_b524, 0x006e_9570),
        (0x00b4_b530, 0x006e_9510),
    ];

    fn konst(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read::<u64>(h.rebase(sym)))
    }

    /// The path unified into `buf` (256 bytes, the original's `ebp -
    /// 0x110`), a new 0x9c-byte `IggyMenu(buf, rect, 0)` (or none), and
    /// the three callbacks on it; the menu.
    fn open(h: &mut impl Host, path: u32, buf: u32, r: [u32; 4]) -> u32 {
        h.call_sym(UNIFY_FILENAME, Conv::Cdecl, &[path, buf, 0x100]);
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x9c]);
        let menu = if p == 0 {
            0
        } else {
            h.call_sym(
                IGGY_MENU_CTOR,
                Conv::Thiscall(p),
                &[buf, r[0], r[1], r[2], r[3], 0],
            )
        };
        for (name, f) in CALLBACKS {
            let name = h.rebase(name);
            let f = h.rebase(f);
            h.call_sym(IGGY_ADD_CALLBACK, Conv::Thiscall(menu), &[name, f]);
        }
        menu
    }

    /// An `i32` through `fild`/`fstp dword`.
    fn i32_f32(v: u32) -> f64 {
        x87::load(x87::store(f64::from(v.cast_signed())))
    }

    /// `IggyPlayerOpenScaled_006e9250(path, x, y, w, h)` -> the menu: the
    /// integer rectangle scaled to the reference screen (`f32(f32(x) /
    /// sw)`, `f32(f32(y) / sh)`, `f32(f32(w) / sw)`, `f32(f32(h) / sh)`).
    pub fn open_scaled(h: &mut impl Host, path: u32, rect: [u32; 4], buf: u32) -> u32 {
        let sh = konst(h, SCALE_H);
        let sw = konst(h, SCALE_W);
        let [x, y, w, hh] = rect;
        let r = [
            x87::store(x87::div(i32_f32(x), sw)),
            x87::store(x87::div(i32_f32(y), sh)),
            x87::store(x87::div(i32_f32(w), sw)),
            x87::store(x87::div(i32_f32(hh), sh)),
        ];
        open(h, path, buf, r)
    }

    /// `IggyPlayerOpen_006e93f0(path, x, y, w, h)` -> the menu: the `f32`
    /// rectangle as it is (through the x87).
    pub fn open_at(h: &mut impl Host, path: u32, rect: [u32; 4], buf: u32) -> u32 {
        open(h, path, buf, rect.map(|w| x87::store(x87::load(w))))
    }

    // ---- the Esc menu's parameter button ----

    pub const MOVIE_FADING: u32 = 0x0134_0824;
    pub const CURRENT_MAP: u32 = 0x012f_bc40;
    pub const SCENARIO_FLAG: u32 = 0x0078_7360;
    pub const IS_ETERNAL_CALM: u32 = 0x0067_ab50;
    pub const LOC_STRING_LOAD: u32 = 0x0068_e180;
    pub const LOC_STRING_TO_VALUE: u32 = 0x0068_e270;
    pub const SET_POPUP_CALLBACK: u32 = 0x0068_ff10;
    pub const CALL_FLASH_1: u32 = 0x0068_dd40;
    pub const ON_DEBUG_FILL: u32 = 0x006f_c970;
    /// `"Function not Available"`-style text and the AS3 function the
    /// unavailable case calls.
    pub const NOT_AVAILABLE_TEXT: u32 = 0x00b4_c52c;
    pub const NOT_AVAILABLE_FN: u32 = 0x00b4_c548;

    /// Text `id` loaded into the string at `buf` and turned into the Iggy
    /// value at `value` (`LocString_Load(buf; id)`, then `ToValue(that;
    /// value)`); the value's four words.
    fn loc_value(h: &mut impl Host, id: u32, buf: u32, value: u32) -> [u32; 4] {
        let s = h.call_sym(LOC_STRING_LOAD, Conv::Thiscall(buf), &[id]);
        h.call_sym(LOC_STRING_TO_VALUE, Conv::Thiscall(s), &[value]);
        [0, 4, 8, 0xc].map(|o| h.read::<u32>(value + o))
    }

    /// `FFEscMenu_onParameterButtonClicked(arg)`: when no movie fades,
    /// the scenario flag is set and the map isn't an Eternal Calm scene,
    /// the pop-up with texts 0x2c9..0x2cb (`showPopUpWithBtn`, 14 words)
    /// and its answer `0x006fc970(.., arg)`; else the "not available"
    /// AS3 call with its text. `frame` is the original's `ebp - 0x364`
    /// (values at `+0`, `+0x10`, `+0x20`; strings at `+0x30`, `+0x140`,
    /// `+0x250`).
    pub fn on_parameter_button_clicked(h: &mut impl Host, arg: u32, frame: u32) {
        let calm = u32::from(h.read::<u32>(h.rebase(MOVIE_FADING)) == 0);
        let flag = h.call_sym(SCENARIO_FLAG, Conv::Cdecl, &[]);
        let ok = if flag & 0xffff != 0 { calm } else { 0 };
        let map = h.read::<u32>(h.rebase(CURRENT_MAP));
        let eternal = h.call_sym(IS_ETERNAL_CALM, Conv::Cdecl, &[map]);
        if eternal == 0 && ok != 0 {
            let v1 = loc_value(h, 0x2cb, frame + 0x250, frame + 0x10);
            let v2 = loc_value(h, 0x2ca, frame + 0x30, frame);
            let v3 = loc_value(h, 0x2c9, frame + 0x140, frame + 0x20);
            let name = h.rebase(crate::m5u_c::bridge::SHOW_POPUP_WITH_BTN);
            let esc = h.read::<u32>(h.rebase(crate::m5u_c::bridge::ESC_MENU));
            let words = [
                esc, name, v3[0], v3[1], v3[2], v3[3], v2[0], v2[1], v2[2], v2[3], v1[0], v1[1],
                v1[2], v1[3],
            ];
            h.call_sym_wide(crate::m5u_c::bridge::CALL_FLASH_3, &words);
            let esc = h.read::<u32>(h.rebase(crate::m5u_c::bridge::ESC_MENU));
            let f = h.rebase(ON_DEBUG_FILL);
            h.call_sym(SET_POPUP_CALLBACK, Conv::Thiscall(esc), &[f, arg]);
            return;
        }
        let value = frame + 0x20;
        let text = h.rebase(NOT_AVAILABLE_TEXT);
        h.call_sym(
            crate::m5u_c::bridge::DATA_VALUE_SET,
            Conv::Cdecl,
            &[value, text],
        );
        let v = [0, 4, 8, 0xc].map(|o| h.read::<u32>(value + o));
        let name = h.rebase(NOT_AVAILABLE_FN);
        let esc = h.read::<u32>(h.rebase(crate::m5u_c::bridge::ESC_MENU));
        h.call_sym(
            CALL_FLASH_1,
            Conv::Cdecl,
            &[esc, name, v[0], v[1], v[2], v[3]],
        );
    }

    // ---- the Esc menu's video settings ----

    pub const PLAY_MENU_SOUND: u32 = 0x0068_f9b0;
    pub const HANDLERS_ADD: u32 = 0x00a2_8e40;
    pub const OPERATOR_NEW_ARRAY: u32 = 0x0063_04c0;
    pub const OPERATOR_DELETE_ARRAY: u32 = 0x0063_0540;
    pub const READ_S32_ARRAY12: u32 = 0x006f_ed60;
    pub const VIDEO_CONFIRM: u32 = 0x006f_f060;
    pub const VIDEO_ANSWER: u32 = 0x006f_f490;
    pub const CALL_MEMBER: u32 = 0x0068_dfa0;
    /// The AS3 functions the video page calls when the change is
    /// cancelled / confirmed.
    pub const VIDEO_CANCEL_FN: u32 = 0x00b4_6c7c;
    pub const VIDEO_APPLIED_FN: u32 = 0x00b4_c6d8;

    /// The three-text pop-up the Esc menu's confirmations share: texts
    /// `ids` (into the values at `frame + 0x10`, `+0`, `+0x20` through the
    /// strings at `+0x250`, `+0x30`, `+0x140`; `frame` the original's
    /// `ebp - 0x364`) shown on `menu` with `showPopUpWithBtn`.
    pub fn loc_popup3(h: &mut impl Host, menu: u32, ids: [u32; 3], frame: u32) {
        let v1 = loc_value(h, ids[0], frame + 0x250, frame + 0x10);
        let v2 = loc_value(h, ids[1], frame + 0x30, frame);
        let v3 = loc_value(h, ids[2], frame + 0x140, frame + 0x20);
        let name = h.rebase(crate::m5u_c::bridge::SHOW_POPUP_WITH_BTN);
        let words = [
            menu, name, v3[0], v3[1], v3[2], v3[3], v2[0], v2[1], v2[2], v2[3], v1[0], v1[1],
            v1[2], v1[3],
        ];
        h.call_sym_wide(crate::m5u_c::bridge::CALL_FLASH_3, &words);
    }

    /// `FFEscMenu_confirmVideoSettings_006ff060(this; changed)` (`ret 4`):
    /// with a change, the confirmation pop-up (texts 0x379..0x377), its
    /// answer `0x006ff490` and menu sound 1; else the page's cancel AS3
    /// call.
    pub fn confirm_video_settings(h: &mut impl Host, this: u32, changed: u32, frame: u32) -> u32 {
        if changed == 0 {
            let name = h.rebase(VIDEO_CANCEL_FN);
            let as3 = crate::m5u_c::bridge::IGGY_CALL_AS3;
            return h.call_sym(as3, Conv::Thiscall(this), &[0, name, 0, 0, 1]);
        }
        loc_popup3(h, this, [0x379, 0x378, 0x377], frame);
        let f = h.rebase(VIDEO_ANSWER);
        h.call_sym(SET_POPUP_CALLBACK, Conv::Thiscall(this), &[f, 0]);
        h.call_sym(PLAY_MENU_SOUND, Conv::Thiscall(this), &[1])
    }

    /// `FFEscMenu_onVideoApplied_006ff1b0(yes)`: on yes the page's AS3
    /// "applied" call on `g_FFEscMenu`.
    pub fn on_video_applied(h: &mut impl Host, yes: u32) -> u32 {
        if yes == 0 {
            return yes;
        }
        let esc = h.read::<u32>(h.rebase(crate::m5u_c::bridge::ESC_MENU));
        let name = h.rebase(VIDEO_APPLIED_FN);
        h.call_sym(
            crate::m5u_c::bridge::IGGY_CALL_AS3,
            Conv::Thiscall(esc),
            &[0, name, 0, 0, 1],
        )
    }

    /// `FFEscMenu_onVideoSettingsChanged_006ff1e0(menu, args)`: the call's
    /// 12 values read into a new 0x30-byte buffer (`0x006fed60`),
    /// compared with the menu's `+0xa4`, and `0x006ff060(menu; changed)`
    /// queued through `g_FFEscMenu`'s handlers (`IggyMenuHandlers_add(..;
    /// menu, 0x006ff060, changed, FFEscMenu_callMember_0068dfa0)`); the
    /// buffer deleted.
    pub fn on_video_settings_changed(h: &mut impl Host, menu: u32, args: u32) -> u32 {
        let buf = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[0x30]);
        h.call_sym(READ_S32_ARRAY12, Conv::Thiscall(menu), &[buf, args]);
        let mut changed = 0;
        for i in 0..12 {
            if h.read::<u32>(buf + 4 * i) != h.read::<u32>(menu + 0xa4 + 4 * i) {
                changed = 1;
                break;
            }
        }
        let esc = h.read::<u32>(h.rebase(crate::m5u_c::bridge::ESC_MENU));
        let f = h.rebase(VIDEO_CONFIRM);
        let call = h.rebase(CALL_MEMBER);
        h.call_sym(HANDLERS_ADD, Conv::Thiscall(esc), &[menu, f, changed, call]);
        h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[buf])
    }

    // ---- the Esc menu's volume sliders and the video answer ----

    pub const SET_WORD_AT: u32 = 0x0040_2a00;
    pub const APPLY_VIDEO_SETTINGS: u32 = 0x006f_f290;
    pub const ON_VIDEO_APPLIED: u32 = 0x006f_f1b0;
    /// The volume sliders' `(handler, settings key)` globals: voice,
    /// music, sound effects, master.
    pub const VOLUMES: [(u32, u32); 4] = [
        (0x00c3_9654, 0x00c3_964c),
        (0x00c3_966c, 0x00c3_9664),
        (0x00c3_9684, 0x00c3_967c),
        (0x00c3_963c, 0x00c3_9634),
    ];

    /// `FFEscMenu_on*VolumeChanged(menu, args)`: the slider value
    /// (`ftol(args[2..3] as f64)`) to the volume's handler (`addCall(menu;
    /// *handler, v)`) and to the settings (`SetWordAt(g_Settings; *key,
    /// v)`).
    pub fn on_volume_changed(h: &mut impl Host, menu: u32, args: u32, which: (u32, u32)) -> u32 {
        let x = f64::from_bits(h.read::<u64>(args + 8));
        let settings = h.read::<u32>(h.rebase(crate::m5u_c::bridge::SETTINGS));
        let v = x87::ftol(h, x);
        let f = h.read::<u32>(h.rebase(which.0));
        let add = crate::m5u_c::bridge::HANDLERS_ADD_CALL;
        h.call_sym(add, Conv::Thiscall(menu), &[f, v]);
        let key = h.read::<u32>(h.rebase(which.1));
        h.call_sym(SET_WORD_AT, Conv::Thiscall(settings), &[key, v])
    }

    /// `FFEscMenu_onVideoAnswer_006ff490(yes)`: on yes
    /// `FFEscMenu_onVideoApplied_006ff1b0(yes)` queued.
    pub fn on_video_answer(h: &mut impl Host, yes: u32) -> u32 {
        if yes == 0 {
            return yes;
        }
        let esc = h.read::<u32>(h.rebase(crate::m5u_c::bridge::ESC_MENU));
        let f = h.rebase(ON_VIDEO_APPLIED);
        let add = crate::m5u_c::bridge::HANDLERS_ADD_CALL;
        h.call_sym(add, Conv::Thiscall(esc), &[f, yes])
    }

    /// `FFEscMenu_onVideoSettingRefresh(menu, args)`: the 12 values into a
    /// new 0x30-byte buffer and `FFEscMenu_applyVideoSettings(menu;
    /// buffer)` queued on the menu (it owns the buffer then).
    pub fn on_video_setting_refresh(h: &mut impl Host, menu: u32, args: u32) -> u32 {
        let buf = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[0x30]);
        h.call_sym(READ_S32_ARRAY12, Conv::Thiscall(menu), &[buf, args]);
        let f = h.rebase(APPLY_VIDEO_SETTINGS);
        let call = h.rebase(CALL_MEMBER);
        h.call_sym(HANDLERS_ADD, Conv::Thiscall(menu), &[menu, f, buf, call])
    }
}
