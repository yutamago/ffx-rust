//! M5 stage 1, round 6 (list `left6`, from `render-pcui4.left`): the
//! character menu window's buttons (`WMainProc`'s character page).
//!
//! Callees stay calls to the originals (cdecl, as their `ret` says). Tail
//! jumps become calls with the words the original hands on; where the
//! original writes its own argument slots before one, the bridge writes
//! them too (`slots`).

/// The originals over game memory.
pub mod bridge {
    use ffx_bridge::{Mem, x87};
    use ffx_graphic::host::{Conv, Host};

    // ---- globals (`0x01303764..`: the character page's toggles) ----

    /// The wind toggle's counter (bit 0: on).
    pub const WIND_TOGGLE: u32 = 0x0130_3764;
    /// The character page's selector strings (by the mode at
    /// [`LIGHT_MODE`]).
    pub const CHR_STR_A: u32 = 0x0130_376c;
    pub const CHR_STR_B: u32 = 0x0130_3770;
    /// The light mode byte (0/1) and the icon mode byte (0..3).
    pub const LIGHT_MODE: u32 = 0x0130_3774;
    pub const ICON_MODE: u32 = 0x0130_3775;
    /// The character and motion counts (`ChrMenu_listCharacters`,
    /// `ChrMenu_listMotions` fill them).
    pub const CHR_COUNT: u32 = 0x023c_bc40;
    pub const MOTION_COUNT: u32 = 0x023c_bc50;
    /// `g_DebugLine`.
    pub const DEBUG_LINE: u32 = 0x023c_4000;
    /// The "no characters" message, the icon mode format, pi (f32), 0.2
    /// (f32).
    pub const NO_CHR_TEXT: u32 = 0x00b5_accc;
    pub const ICON_FORMAT: u32 = 0x00b5_adf8;
    pub const PI_F32: u32 = 0x00b5_ae68;
    pub const WIND_X: u32 = 0x00b4_12c4;
    /// `MSVCR110` `sprintf` slot.
    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

    // ---- callees ----

    pub const SG_SET_RESOURCE: u32 = 0x0083_fe10;
    pub const UPDATE_LIGHT_TEXTS: u32 = 0x0084_d540;
    pub const SELECTOR_ALLOCATE: u32 = 0x0084_9bb0;
    pub const LIST_CHARACTERS: u32 = 0x0084_d4a0;
    pub const SELECTOR_SET_SELECT_STR: u32 = 0x0084_a110;
    pub const SELECTOR_OPEN: u32 = 0x0084_9d60;
    pub const LIST_MOTIONS: u32 = 0x0084_d3f0;
    pub const SG_GET_RESOURCE_STR: u32 = 0x0083_fc30;
    pub const MESSAGE_BOX: u32 = 0x0084_9970;
    pub const FILL_SELECTOR: u32 = 0x0084_ce10;
    pub const WINDOW_CREATE: u32 = 0x0084_a920;
    pub const SPEC_WINDOW_PROC: u32 = 0x0084_ccc0;
    pub const CALC_NORMAL_GET: u32 = 0x0083_4060;
    pub const CALC_NORMAL_SET_ALL: u32 = 0x0083_4090;
    pub const BUTTON_SET_TOGGLE: u32 = 0x0084_93e0;
    pub const DBG_WINDOW_A: u32 = 0x0085_1de0;
    pub const DBG_WINDOW_B: u32 = 0x0085_1db0;
    pub const DBG_WINDOW_C: u32 = 0x0085_1e40;
    pub const SET_DRAW_ICON: u32 = 0x0083_0c70;
    pub const SET_DRAW_ICON2: u32 = 0x0083_0c80;
    pub const WINDOW_SET_TEXT: u32 = 0x0084_a710;
    pub const TOGGLE_GLOBAL: u32 = 0x0084_3590;
    pub const MOTION_HALF_GET: u32 = 0x0083_9870;
    pub const MOTION_HALF_SET: u32 = 0x0083_9950;
    pub const WRAP_ANGLE: u32 = 0x0084_d330;
    pub const SET_WIND: u32 = 0x0082_4e30;

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn g8(h: &impl Mem, sym: u32) -> u8 {
        h.read(h.rebase(sym))
    }

    fn g32(h: &impl Mem, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    /// A toggle through `get`/`set` (`1 - get()`), then the button `id`
    /// shown with the new value: the original writes `id` and the value
    /// into its own argument slots and tail-jumps to
    /// `Gu_ButtonSetToggleValue(window, id, value)`.
    fn toggle(h: &mut impl Host, slots: u32, window: u32, id: u32, get: u32, set: u32) -> u32 {
        let v = cdecl(h, get, &[]);
        cdecl(h, set, &[1_u32.wrapping_sub(v)]);
        let v = cdecl(h, get, &[]);
        h.write(slots.wrapping_add(8), v);
        h.write(slots.wrapping_add(4), id);
        cdecl(h, BUTTON_SET_TOGGLE, &[window, id, v])
    }

    /// `FUN_0084cf60(window, id, state; slots)` -> `eax` as the original:
    /// the character page's button handler (`WMainProc`). `state` holds
    /// the page's two selectors (`+0`, `+4`). Buttons: 1 the character
    /// view selector (`ChrMenu_fillSelector_0084ce10`, opened when it
    /// returns 0), 2 the motion list and 3 the character list (a message
    /// box when empty), 0x36 the specular window, 0x43 a bare `ret`
    /// (nothing called: `eax` the table index 4), 0x45/0x64 the normal and
    /// half-motion toggles, 0x47/0x5d the icon mode, 0x4e/0x5a/0x68/0x69
    /// tail jumps (no words read), 0x54 the light mode, 0x65 the wind.
    /// Other ids leave the table index (14) or `id - 1` in `eax`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn chr_buttons(h: &mut impl Host, slots: u32, window: u32, id: u32, state: u32) -> u32 {
        match id {
            0x54 => {
                let at = h.rebase(LIGHT_MODE);
                let m = h.read::<u8>(at).wrapping_sub(1) & 1;
                h.write(at, m);
                cdecl(h, SG_SET_RESOURCE, &[0xf, u32::from(m)]);
                cdecl(h, UPDATE_LIGHT_TEXTS, &[window])
            }
            2 | 3 => {
                let sel = cdecl(h, SELECTOR_ALLOCATE, &[]);
                h.write(state.wrapping_add(4), sel);
                let (list, count) = if id == 3 {
                    (LIST_CHARACTERS, CHR_COUNT)
                } else {
                    (LIST_MOTIONS, MOTION_COUNT)
                };
                cdecl(h, list, &[sel]);
                if g32(h, count).cast_signed() <= 0 {
                    let text = h.rebase(NO_CHR_TEXT);
                    return cdecl(h, MESSAGE_BOX, &[text]);
                }
                let s = if id == 3 {
                    if g8(h, LIGHT_MODE) != 0 {
                        g32(h, CHR_STR_B)
                    } else {
                        g32(h, CHR_STR_A)
                    }
                } else {
                    cdecl(h, SG_GET_RESOURCE_STR, &[0xb])
                };
                let sel: u32 = h.read(state.wrapping_add(4));
                cdecl(h, SELECTOR_SET_SELECT_STR, &[sel, s]);
                let sel: u32 = h.read(state.wrapping_add(4));
                let arg = if id == 3 { 3 } else { 0xf };
                cdecl(h, SELECTOR_OPEN, &[sel, window, arg])
            }
            1 => {
                let r = cdecl(h, FILL_SELECTOR, &[state]);
                if r != 0 {
                    return r;
                }
                let sel: u32 = h.read(state);
                cdecl(h, SELECTOR_OPEN, &[sel, window, 0x3e])
            }
            0x36 => {
                let proc_ = h.rebase(SPEC_WINDOW_PROC);
                cdecl(h, WINDOW_CREATE, &[0x64, 0x28, 0x5a, 0x28, 6, proc_])
            }
            // a tail jump to a bare `ret` at 0x00836f00
            0x43 => 4,
            0x45 => toggle(h, slots, window, 0x45, CALC_NORMAL_GET, CALC_NORMAL_SET_ALL),
            0x64 => toggle(h, slots, window, 0x64, MOTION_HALF_GET, MOTION_HALF_SET),
            0x69 => cdecl(h, DBG_WINDOW_A, &[]),
            0x5a => cdecl(h, DBG_WINDOW_B, &[]),
            0x68 => cdecl(h, DBG_WINDOW_C, &[]),
            0x4e => cdecl(h, TOGGLE_GLOBAL, &[]),
            0x47 | 0x5d => {
                let at = h.rebase(ICON_MODE);
                let m = h.read::<u8>(at).wrapping_add(1);
                h.write(at, m);
                let m = |h: &mut _| -> u32 {
                    i32::from(g8(h, ICON_MODE).cast_signed()).cast_unsigned()
                };
                let v = m(h) & 1;
                cdecl(h, SET_DRAW_ICON, &[v]);
                let v = (m(h) & 2) >> 1;
                cdecl(h, SET_DRAW_ICON2, &[v]);
                let v = m(h) & 3;
                let line = h.rebase(DEBUG_LINE);
                let fmt = h.rebase(ICON_FORMAT);
                let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
                h.call(f, Conv::Cdecl, &[line, fmt, v]);
                cdecl(h, WINDOW_SET_TEXT, &[window, 0x47, line]);
                cdecl(h, WINDOW_SET_TEXT, &[window, 0x5d, line])
            }
            0x65 => {
                let at = h.rebase(WIND_TOGGLE);
                let c = h.read::<u32>(at).wrapping_add(1);
                h.write(at, c);
                if c & 1 != 0 {
                    let pi = x87::store(x87::load(g32(h, PI_F32)));
                    let f = h.rebase(WRAP_ANGLE);
                    let y = h.call_st0(f, Conv::Cdecl, &[pi]).to_bits();
                    let x = x87::store(x87::load(g32(h, WIND_X)));
                    cdecl(h, SET_WIND, &[x, y, 0])
                } else {
                    cdecl(h, SET_WIND, &[0, 0, 0])
                }
            }
            _ if id.wrapping_sub(1) <= 0x68 => 14,
            _ => id.wrapping_sub(1),
        }
    }
}
