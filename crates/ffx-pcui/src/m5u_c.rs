//! M5 stage 1, list `pc-ui`, batch 3: the input thread
//! (`AsyncControllerTaskManager`: its constructor, `sCreateInstance` and
//! thread loop), the touch record's and the 16-array's teardown, the Esc
//! menu's constructor and two jump thunks.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the originals' argument words
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    use crate::m5u_a::bridge::{
        ASYNC_CONTROLLER, ASYNC_CONTROLLER_ON, CLEAR_U32, OPERATOR_DELETE, OPERATOR_NEW, PARRAY16,
        PHYRE_FREE, TOUCH_RECORD, XBAD_ALLOC_IAT,
    };

    pub const RESET_ARRAYS: u32 = 0x0068_9620;
    pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
    pub const PARRAY_RESET: u32 = 0x006f_bcd0;
    pub const PARRAY16_CLEAR: u32 = 0x0068_bae0;
    /// `KERNEL32!CreateMutexW` (stdcall, 3 words).
    pub const CREATE_MUTEX_IAT: u32 = 0x00b0_c198;
    pub const INPUT_CUR_TIME: u32 = 0x0063_0c60;
    pub const CREATE_THREAD: u32 = 0x006f_2e10;
    pub const START_THREAD: u32 = 0x006f_2d30;
    pub const THREAD_MAIN: u32 = 0x0068_d740;
    /// `"AsyncControllerTask"`-style thread name.
    pub const THREAD_NAME: u32 = 0x00b4_6bd4;
    pub const CONTROLLER_CTOR: u32 = 0x0068_c830;
    pub const SLEEP_MS: u32 = 0x006f_2d20;
    pub const ASYNC_POLL: u32 = 0x0063_0cb0;
    pub const FREE_CAMERA_CTOR: u32 = 0x0067_fe70;
    pub const THREAD_EXIT: u32 = 0x006f_2f00;
    /// `g_FreeCamera`.
    pub const FREE_CAMERA: u32 = 0x00cc_c830;
    /// The poll interval (`f32` 1/60 s) and the sleep base (`f64`).
    pub const POLL_INTERVAL: u32 = 0x00b4_6c04;
    pub const SLEEP_BASE: u32 = 0x00b4_6c08;
    pub const ESC_UPDATE_SPECIAL_FEATURE: u32 = 0x0070_2660;
    pub const CONVERT_LINE_BREAKS: u32 = 0x006e_9700;
    pub const IGGY_MENU_CTOR: u32 = 0x00a2_88f0;
    pub const IGGY_MENU_SET_ENABLE: u32 = 0x00a2_8ef0;
    pub const WINDOW_EVENT_INSERT: u32 = 0x0062_5c70;
    /// `FFEscMenu`'s vtables (the menu, its `IWindowEvent` base).
    pub const IWINDOW_EVENT_VTABLE: u32 = 0x00b4_6c14;
    pub const ESC_MENU_VTABLE: u32 = 0x00b4_6c20;
    pub const ESC_MENU_EVENT_VTABLE: u32 = 0x00b4_6c2c;
    /// The Esc menu's constructor calls, in order (thiscall, no words).
    pub const ESC_MENU_INIT: [u32; 9] = [
        0x0068_fd40, // registerCallbacks
        0x0068_f400, // setLanguage
        0x0068_e6f0, // initializeMainSettingPage
        0x006f_f020, // rebuildVideoSettingPage
        0x006f_f770, // initializeAudioSettingPage
        0x0070_1fa0, // initializeControllerSettingPage
        0x0070_0da0, // initializeKeyboardSettingPage
        0x0070_2560, // the jump to updateSpecialFeaturePage
        0x006f_c3f0, // initializeParameterChangePage
    ];

    fn g(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    fn set_g(h: &mut impl Host, sym: u32, v: u32) {
        let at = h.rebase(sym);
        h.write(at, v);
    }

    fn rd(h: &impl Host, at: u32) -> u32 {
        h.read(at)
    }

    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    fn delete(h: &mut impl Host, p: u32) {
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
    }

    fn owned(h: &impl Host, at: u32) -> bool {
        rd(h, at) >> 31 == 0
    }

    /// `0x00689830()`: [`TOUCH_RECORD`] torn down (`ResetArrays_00689620`),
    /// deleted and cleared when set.
    pub fn touch_record_destroy(h: &mut impl Host) {
        let p = g(h, TOUCH_RECORD);
        if p != 0 {
            h.call_sym(RESET_ARRAYS, Conv::Thiscall(p), &[]);
            delete(h, p);
            set_g(h, TOUCH_RECORD, 0);
        }
    }

    /// `0x0068b830(this)` (`PArray` reset): the buffer `+4` freed when owned
    /// (`+0` bit 31 clear); both words 0.
    pub fn parray_free(h: &mut impl Host, this: u32) {
        if owned(h, this) {
            let p = rd(h, this + 4);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
        }
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
    }

    /// `0x0068bae0(this)`: the array of 16 pointers made (unless `+0` is
    /// already 16: a zeroed 0x40-byte block from `PhyreMalloc(0x40, 4)`,
    /// the old buffer freed when owned and different), then every entry
    /// reset (`PArray_Reset_006fbcd0`), deleted and cleared.
    pub fn parray16_clear(h: &mut impl Host, this: u32) {
        if rd(h, this) != 0x10 {
            let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x40, 4]);
            if p != 0 {
                for i in 0..16 {
                    h.write(p + 4 * i, 0u32);
                }
                let old = rd(h, this + 4);
                if old != p && owned(h, this) {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
                }
                h.write(this + 4, p);
                h.write(this, 0x10u32);
            }
        }
        let mut i = 0;
        while i < rd(h, this) & 0x7fff_ffff {
            let at = rd(h, this + 4).wrapping_add(4 * i);
            let e = rd(h, at);
            if e != 0 {
                h.call_sym(PARRAY_RESET, Conv::Thiscall(e), &[]);
                delete(h, e);
                let at = rd(h, this + 4).wrapping_add(4 * i);
                h.write(at, 0u32);
            }
            i += 1;
        }
    }

    /// `0x0068bba0()`: [`PARRAY16`] cleared (`0x0068bae0`), its buffer
    /// freed when owned, both words 0, deleted and cleared.
    pub fn parray16_destroy(h: &mut impl Host) {
        let p = g(h, PARRAY16);
        if p == 0 {
            return;
        }
        h.call_sym(PARRAY16_CLEAR, Conv::Thiscall(p), &[]);
        if owned(h, p) {
            let b = rd(h, p + 4);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[b]);
        }
        h.write(p + 4, 0u32);
        h.write(p, 0u32);
        delete(h, p);
        set_g(h, PARRAY16, 0);
    }

    /// An empty `std::deque` at `at` with its container proxy (8 bytes,
    /// `operator new`; `_Xbad_alloc` on failure).
    fn deque_init(h: &mut impl Host, at: u32) {
        for i in 0..5 {
            h.write(at + 4 * i, 0u32);
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[8]);
        if p == 0 {
            import(h, XBAD_ALLOC_IAT, Conv::Cdecl, &[]);
        }
        h.write(at, p);
        h.write(p, 0u32);
        h.write(p + 4, 0u32);
        let proxy = rd(h, at);
        h.write(proxy, at);
    }

    /// `AsyncControllerTaskManager_AsyncControllerTaskManager(this)` ->
    /// `this`: thread id 0, run flag 1, both deques, `+0x38`/`+0x3c` 0, a
    /// new mutex `+0x40`, [`ASYNC_CONTROLLER`] = `this`, both times
    /// `+8`/`+0xc` = `inputGetCurTime()`, and the thread
    /// (`CreateThread_ps2` with the parameter block `frame`, the
    /// original's `ebp - 0x40`: entry `+4` the thread loop, stack `+0xc`
    /// 0x8000, priority `+0x14` 999, `+0x20` 0) started when its id is
    /// not negative.
    pub fn controller_ctor(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        h.write(this, 0u32);
        h.write(this + 4, 1u32);
        deque_init(h, this + 0x10);
        deque_init(h, this + 0x24);
        h.write(this + 0x38, 0u32);
        h.write(this + 0x3c, 0u32);
        let m = import(h, CREATE_MUTEX_IAT, Conv::Stdcall, &[0, 0, 0]);
        h.write(this + 0x40, m);
        set_g(h, ASYNC_CONTROLLER, this);
        let t = h.call_sym_st0(INPUT_CUR_TIME, Conv::Cdecl, &[]);
        h.write(this + 8, x87::store(f64::from(t)));
        let t = h.call_sym_st0(INPUT_CUR_TIME, Conv::Cdecl, &[]);
        h.write(this + 0xc, x87::store(f64::from(t)));
        let entry = h.rebase(THREAD_MAIN);
        h.write(frame + 4, entry);
        h.write(frame + 0x14, 0x3e7u32);
        h.write(frame + 0xc, 0x8000u32);
        h.write(frame + 0x20, 0u32);
        let name = h.rebase(THREAD_NAME);
        let id = h.call_sym(CREATE_THREAD, Conv::Cdecl, &[frame, name]);
        h.write(this, id);
        if id.cast_signed() >= 0 {
            h.call_sym(START_THREAD, Conv::Cdecl, &[id, 0]);
        }
        this
    }

    /// `AsyncControllerTaskManager_sCreateInstance()`: when
    /// [`ASYNC_CONTROLLER`] is unset, a new 0x44-byte manager (its
    /// constructor's `eax`), or 0.
    pub fn controller_create(h: &mut impl Host) {
        if g(h, ASYNC_CONTROLLER) != 0 {
            return;
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x44]);
        let m = if p == 0 {
            0
        } else {
            h.call_sym(CONTROLLER_CTOR, Conv::Thiscall(p), &[])
        };
        set_g(h, ASYNC_CONTROLLER, m);
    }

    /// `0x0068ca50(this)` (the button deque's destructor): `_Tidy`, the
    /// proxy deleted and cleared.
    pub fn deque_u32_dtor(h: &mut impl Host, this: u32) {
        h.call_sym(CLEAR_U32, Conv::Thiscall(this), &[]);
        let p = rd(h, this);
        delete(h, p);
        h.write(this, 0u32);
    }

    /// One pass of `AsyncControllerTaskManager_ThreadMain`'s loop on the
    /// manager `m`: the time `+0xc` = now; with the queues off, `+8` = now
    /// and a 16 ms sleep. Else, when at least the poll interval passed and
    /// [`ASYNC_CONTROLLER_ON`] is 1 and the free camera (made on demand)
    /// is idle (`+0x338`, `+0x33c`, `+0` clear): `+8` = `+0xc` and a
    /// poll (`Input_AsyncPoll`); otherwise a sleep of `max(1, ftol(base -
    /// elapsed))` ms.
    pub fn controller_tick(h: &mut impl Host, m: u32) {
        let now = h.call_sym_st0(INPUT_CUR_TIME, Conv::Cdecl, &[]);
        let now = x87::load(x87::store(f64::from(now)));
        h.write(m + 0xc, x87::store(now));
        let on = g(h, ASYNC_CONTROLLER_ON);
        let elapsed = x87::load(x87::store(x87::sub(now, x87::load(rd(h, m + 8)))));
        if on == 0 {
            h.write(m + 8, x87::store(now));
            h.call_sym(SLEEP_MS, Conv::Cdecl, &[0x10]);
            return;
        }
        let interval = x87::load(g(h, POLL_INTERVAL));
        // fcomp: ordered and not (interval > elapsed)
        let due = elapsed >= interval;
        if due && on == 1 {
            let mut cam = g(h, FREE_CAMERA);
            if cam == 0 {
                let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x340]);
                cam = if p == 0 {
                    0
                } else {
                    h.call_sym(FREE_CAMERA_CTOR, Conv::Thiscall(p), &[])
                };
                set_g(h, FREE_CAMERA, cam);
            }
            if h.read::<u8>(cam + 0x338) == 0 && h.read::<u8>(cam + 0x33c) == 0 && rd(h, cam) == 0 {
                let t = rd(h, m + 0xc);
                h.write(m + 8, x87::store(x87::load(t)));
                h.call_sym(ASYNC_POLL, Conv::Cdecl, &[]);
                return;
            }
        }
        let base = f64::from_bits(h.read::<u64>(h.rebase(SLEEP_BASE)));
        let left = x87::sub(base, elapsed);
        let ms = x87::ftol(h, left).cast_signed().max(1).cast_unsigned();
        h.call_sym(SLEEP_MS, Conv::Cdecl, &[ms]);
    }

    /// `AsyncControllerTaskManager_ThreadMain()`: [`controller_tick`] while
    /// the manager's run flag `+4` is set, then `+0x3c` = 1 (stopped) and
    /// the thread's exit (`0x006f2f00(thread id)`, its `eax`).
    pub fn controller_thread(h: &mut impl Host) -> u32 {
        while rd(h, g(h, ASYNC_CONTROLLER) + 4) != 0 {
            let m = g(h, ASYNC_CONTROLLER);
            controller_tick(h, m);
        }
        let m = g(h, ASYNC_CONTROLLER);
        let id = rd(h, m);
        h.write(m + 0x3c, 1u32);
        h.call_sym(THREAD_EXIT, Conv::Cdecl, &[id])
    }

    /// `0x00702560(this)`: a jump to `FFEscMenu_updateSpecialFeaturePage`.
    pub fn esc_update_special_feature_jump(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(ESC_UPDATE_SPECIAL_FEATURE, Conv::Thiscall(this), &[])
    }

    /// `0x006e96b0(src, dst, len)`: a jump to
    /// `LocText_ConvertLineBreaks_006e9700`.
    pub fn convert_line_breaks_jump(h: &mut impl Host, src: u32, dst: u32, len: u32) -> u32 {
        h.call_sym(CONVERT_LINE_BREAKS, Conv::Cdecl, &[src, dst, len])
    }

    /// `FFEscMenu_ctor_0068dfd0(this; path, x, y, w, h)` (`ret 0x14`) ->
    /// `this`: the Iggy menu (`IggyMenu_ctor(this; path, x, y, w, h, 2)`,
    /// the floats through the x87), the vtables, the fields, every page
    /// set up, the menu disabled, `+0xa0` = 1 and the window event
    /// handler (`+0x9c`) registered.
    pub fn esc_menu_ctor(h: &mut impl Host, this: u32, path: u32, rect: [u32; 4]) -> u32 {
        let q = rect.map(|w| x87::store(x87::load(w)));
        h.call_sym(
            IGGY_MENU_CTOR,
            Conv::Thiscall(this),
            &[path, q[0], q[1], q[2], q[3], 2],
        );
        let vt = h.rebase(IWINDOW_EVENT_VTABLE);
        h.write(this + 0x9c, vt);
        let vt = h.rebase(ESC_MENU_VTABLE);
        h.write(this, vt);
        let vt = h.rebase(ESC_MENU_EVENT_VTABLE);
        h.write(this + 0x9c, vt);
        for (off, v) in [(0xd4, 0u32), (0xd8, 0xffff_ffff), (0xdc, 0)] {
            h.write(this + off, v);
        }
        h.write(this + 0xe0, 1u8);
        h.write(this + 0xe4, 0xffff_ffffu32);
        h.write(this + 0xe8, 0u8);
        h.write(this + 0xf0, 0u32);
        h.write(this + 0xf4, 0u32);
        h.write(this + 0xfc, 0u8);
        for off in [0x100, 0x104, 0x108, 0x10c, 0x110, 0x114] {
            h.write(this + off, 0u32);
        }
        h.write(this + 0x118, 0u8);
        h.write(this + 0x12c, 0u32);
        h.write(this + 0x130, 1u8);
        h.write(this + 0x234, 0xffff_ffffu32);
        for f in ESC_MENU_INIT {
            h.call_sym(f, Conv::Thiscall(this), &[]);
        }
        h.call_sym(IGGY_MENU_SET_ENABLE, Conv::Thiscall(this), &[0]);
        h.write(this + 0xa0, 1u8);
        h.call_sym(WINDOW_EVENT_INSERT, Conv::Cdecl, &[this + 0x9c]);
        this
    }

    // ---- the Esc menu's Iggy callbacks ----

    pub const GET_DEREF: u32 = 0x00a2_8d10;
    pub const VALUE_REF_CREATE: u32 = 0x00a2_88b0;
    pub const GET_ARRAY_S32: u32 = 0x00a2_8b20;
    pub const HANDLERS_ADD_CALL: u32 = 0x00a2_8df0;
    pub const VALUE_REF_FREE: u32 = 0x00a2_8a20;
    pub const IGGY_CALL_AS3: u32 = 0x00a2_8ba0;
    pub const GET_LOC_TEXT: u32 = 0x0064_12f0;
    pub const DATA_VALUE_SET: u32 = 0x00a2_8d70;
    pub const CALL_FLASH_3: u32 = 0x0068_ddd0;
    pub const SETTINGS_SAVE: u32 = 0x0040_1d80;
    pub const RESET_TO_LAUNCHER: u32 = 0x0042_f4d0;
    /// `USER32!ShowCursor` (stdcall, 1 word).
    pub const SHOW_CURSOR_IAT: u32 = 0x00b0_c688;
    /// `g_FFEscMenu`, `g_Settings`.
    pub const ESC_MENU: u32 = 0x00cc_c870;
    pub const SETTINGS: u32 = 0x022f_b504;
    /// `g_EscParameterButtonArg` (name ours): the second array value
    /// `0x0068f610` hands `onParameterButtonClicked`.
    pub const PARAMETER_BUTTON_ARG: u32 = 0x00cc_c884;
    pub const ON_PARAMETER_BUTTON_CLICKED: u32 = 0x006f_c7b0;
    pub const CALL_ON_DEFAULT_KEY: u32 = 0x0068_f6d0;
    pub const SHOW_DEFAULT_KEY_POPUP: u32 = 0x0068_f710;
    pub const QUEUE_DEFAULT_KEY: u32 = 0x0068_f6b0;
    pub const SHOW_QUIT_POPUP: u32 = 0x0068_fb20;
    /// `"onDefaultKey"`, `"showPopUpWithBtn"`.
    pub const ON_DEFAULT_KEY: u32 = 0x00b4_6cc0;
    pub const SHOW_POPUP_WITH_BTN: u32 = 0x00b4_6cac;

    /// `FFEscMenu_onParameterButton_0068f610(menu, args)`: the call's
    /// argument array (`IggyValueRef` at `frame`, the original's `ebp -
    /// 0x48`, made from `(*menu, args[1])`); value 1 goes to
    /// [`PARAMETER_BUTTON_ARG`], and when value 0's low 16 bits are 13 the
    /// parameter button's handler is queued with it.
    pub fn on_parameter_button(h: &mut impl Host, menu: u32, args: u32, frame: u32) {
        let temp = rd(h, args + 4);
        let player = h.call_sym(GET_DEREF, Conv::Thiscall(menu), &[]);
        h.call_sym(VALUE_REF_CREATE, Conv::Thiscall(frame), &[player, temp]);
        let kind = h.call_sym(GET_ARRAY_S32, Conv::Thiscall(frame), &[0]);
        let v = h.call_sym(GET_ARRAY_S32, Conv::Thiscall(frame), &[1]);
        set_g(h, PARAMETER_BUTTON_ARG, v);
        if kind & 0xffff == 0xd {
            let f = h.rebase(ON_PARAMETER_BUTTON_CLICKED);
            let arg = h.rebase(PARAMETER_BUTTON_ARG);
            h.call_sym(HANDLERS_ADD_CALL, Conv::Thiscall(menu), &[f, arg]);
        }
        h.call_sym(VALUE_REF_FREE, Conv::Thiscall(frame), &[]);
    }

    /// `FFEscMenu_queueDefaultKey_0068f6b0(yes, menu)`: on yes,
    /// `0x0068f6d0(menu)` queued on the menu.
    pub fn queue_default_key(h: &mut impl Host, yes: u32, menu: u32) -> u32 {
        if yes == 0 {
            return yes;
        }
        let f = h.rebase(CALL_ON_DEFAULT_KEY);
        h.call_sym(HANDLERS_ADD_CALL, Conv::Thiscall(menu), &[f, menu])
    }

    /// `FFEscMenu_callOnDefaultKey_0068f6d0(menu)`: the AS3 `onDefaultKey`.
    pub fn call_on_default_key(h: &mut impl Host, menu: u32) -> u32 {
        let name = h.rebase(ON_DEFAULT_KEY);
        h.call_sym(IGGY_CALL_AS3, Conv::Thiscall(menu), &[0, name, 0, 0, 1])
    }

    /// `FFEscMenu_onSpecialFeatureHelpString(menu)`: `0x0068f710(menu)`
    /// queued.
    pub fn on_special_feature_help(h: &mut impl Host, menu: u32) -> u32 {
        let f = h.rebase(SHOW_DEFAULT_KEY_POPUP);
        h.call_sym(HANDLERS_ADD_CALL, Conv::Thiscall(menu), &[f, menu])
    }

    /// `FFEscMenu_onQuit_0068fb00(menu)`: `FFEscMenu_ShowQuitPopUp_0068fb20`
    /// queued (argument 0).
    pub fn on_quit(h: &mut impl Host, menu: u32) -> u32 {
        let f = h.rebase(SHOW_QUIT_POPUP);
        h.call_sym(HANDLERS_ADD_CALL, Conv::Thiscall(menu), &[f, 0])
    }

    /// One text of the pop-up: localised text `id` into `frame + 0x348`
    /// (the original's `ebp - 0x104`), its line breaks converted into
    /// `out`, the Iggy value `value` (four words) set from it.
    fn popup_text(h: &mut impl Host, frame: u32, id: u32, out: u32, value: u32) -> [u32; 4] {
        let text = frame + 0x348;
        h.call_sym(GET_LOC_TEXT, Conv::Cdecl, &[id, text, 0x100]);
        h.call_sym(LINE_BREAKS_JUMP, Conv::Cdecl, &[text, out, 0x100]);
        h.call_sym(DATA_VALUE_SET, Conv::Cdecl, &[value, out]);
        [0, 4, 8, 0xc].map(|o| rd(h, value + o))
    }

    pub const LINE_BREAKS_JUMP: u32 = 0x006e_96b0;

    /// `FFEscMenu_showDefaultKeyPopUp_0068f710(menu)`: texts 0x212, 0x211
    /// and 0x210 into Iggy values, `showPopUpWithBtn(menu, t210, t211,
    /// t212)` (14 words: the values by value), then the answer callback
    /// (`g_FFEscMenu` `+0x10c` = `0x0068f6b0`, `+0x110` = `menu`). `frame`
    /// is the original's `ebp - 0x44c`.
    pub fn show_default_key_popup(h: &mut impl Host, menu: u32, frame: u32) {
        h.write(frame + 0xc, menu);
        let v1 = popup_text(h, frame, 0x212, frame + 0x238, frame + 0x338);
        for (k, w) in (0u32..).zip(v1) {
            let at = [0, 4, 0x10, 8][k as usize];
            h.write(frame + at, w);
        }
        let v2 = popup_text(h, frame, 0x211, frame + 0x18, frame + 0x118);
        h.write(frame + 0x14, v2[3]);
        let v3 = popup_text(h, frame, 0x210, frame + 0x128, frame + 0x228);
        let name = h.rebase(SHOW_POPUP_WITH_BTN);
        let words = [
            menu, name, v3[0], v3[1], v3[2], v3[3], v2[0], v2[1], v2[2], v2[3], v1[0], v1[1],
            v1[2], v1[3],
        ];
        h.call_sym_wide(CALL_FLASH_3, &words);
        let esc = g(h, ESC_MENU);
        h.write(esc + 0x110, menu);
        let cb = h.rebase(QUEUE_DEFAULT_KEY);
        h.write(esc + 0x10c, cb);
    }

    /// `FFEscMenu_onPopUpButton_0068f8f0(menu, args)`: with an answer
    /// callback (`g_FFEscMenu` `+0x10c`), it gets `(ftol(args[1] as f64),
    /// +0x110)` and both are cleared; its `eax`.
    pub fn on_popup_button(h: &mut impl Host, args: u32) -> u32 {
        let esc = g(h, ESC_MENU);
        let cb = rd(h, esc + 0x10c);
        if cb == 0 {
            return 0;
        }
        let arg = rd(h, esc + 0x110);
        let x = f64::from_bits(h.read::<u64>(args + 8));
        let v = x87::ftol(h, x);
        let eax = h.call(cb, Conv::Cdecl, &[v, arg]);
        h.write(esc + 0x10c, 0u32);
        h.write(esc + 0x110, 0u32);
        eax
    }

    /// `FFEscMenu_QuitConfirmed_0068fcf0(yes)`: on yes the menu disabled,
    /// the cursor shown (when `+0x130` wasn't 1; then 1), the settings
    /// saved and a tail jump to `ResetToLauncher` (its `eax`).
    pub fn quit_confirmed(h: &mut impl Host, yes: u32) -> u32 {
        if yes == 0 {
            return yes;
        }
        let esc = g(h, ESC_MENU);
        h.call_sym(IGGY_MENU_SET_ENABLE, Conv::Thiscall(esc), &[0]);
        let esc = g(h, ESC_MENU);
        if h.read::<u8>(esc + 0x130) != 1 {
            h.write(esc + 0x130, 1u8);
            import(h, SHOW_CURSOR_IAT, Conv::Stdcall, &[1]);
        }
        let s = g(h, SETTINGS);
        h.call_sym(SETTINGS_SAVE, Conv::Thiscall(s), &[]);
        h.call_sym(RESET_TO_LAUNCHER, Conv::Cdecl, &[])
    }
}
