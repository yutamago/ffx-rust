//! Continuation round 8 of list `h4-glue-other`, batch 2: the escape
//! menu's controller, keyboard, audio and parameter-change pages.
//!
//! Both pages build Iggy values in their frames with
//! `iggyDataValueSet_00a28d70` (a string: words 0, 2 and 3) and
//! `iggyDataValueSet` (`0x00a28d30`, an integer as a double: words 0, 2
//! and 3) and pass them on by value. Neither callee writes word 1, so
//! the originals pass stale stack words there; the ports pass what their
//! frames hold (0 in the game; junk-read policy, brief 2026-10-24).

use ffx_graphic::host::{Conv, Host};

use crate::h4_d4::call_flash_8;
use crate::h4_j1::{FFESC_INIT_CONTROLLER_KEY_NAMES, FFESC_INIT_KEYBOARD_KEY_NAMES};

/// `g_Settings` (`0x022fb504`).
pub const SETTINGS: u32 = 0x022f_b504;
/// `Table60_Get(settings; i)` (`0x006311b0`, thiscall, ret 4): the
/// settings' binding `i`.
pub const TABLE60_GET: u32 = 0x0063_11b0;
/// `GetTable_00c8a538(i)` (`0x00402be0`, ret 4; the callers leave the
/// settings in `ecx`).
pub const GET_TABLE_C8A538: u32 = 0x0040_2be0;
/// `KeyNames_getController_00405b10(slot)` (cdecl): the controller key
/// name of a slot, or 0.
pub const CONTROLLER_NAME: u32 = 0x0040_5b10;
/// `KeyNames_getKeyboard_00405af0(slot)` (cdecl): the keyboard key name of
/// a slot, or 0.
pub const KEYBOARD_NAME: u32 = 0x0040_5af0;
/// `iggyDataValueSet_00a28d70(value, text)` (cdecl): a string value.
pub const VALUE_STRING: u32 = 0x00a2_8d70;
/// `iggyDataValueSet(value, n)` (`0x00a28d30`, cdecl): an integer value.
pub const VALUE_INT: u32 = 0x00a2_8d30;
/// `LocString_Load_0068e180(loc; id)` (thiscall, ret 4) -> `loc`.
pub const LOC_LOAD: u32 = 0x0068_e180;
/// `LocString_ToValue_0068e270(loc; value)` (thiscall, ret 4).
pub const LOC_TO_VALUE: u32 = 0x0068_e270;
/// `IggyMenu_callFlashFunction_6_006fcaf0(menu, name, v0..v5)` (cdecl,
/// 26 words).
pub const CALL_FLASH_6: u32 = 0x006f_caf0;
/// The `"[ ]"` text both pages pass (`0x00b4c7e4`).
pub const EMPTY_TEXT: u32 = 0x00b4_c7e4;
/// The controller page's rows: 14 records of 12 bytes (`0x00c39ebc` up
/// to `0x00c39f64`): binding, message.
pub const CONTROLLER_ROWS: u32 = 0x00c3_9ebc;
/// The end of [`CONTROLLER_ROWS`].
pub const CONTROLLER_ROWS_END: u32 = 0x00c3_9f64;
/// The controller rows' second text (`0x00b4c820`).
pub const CONTROLLER_TEXT: u32 = 0x00b4_c820;
/// The controller rows' Flash function name (`0x00b4c838`).
pub const CONTROLLER_FUNCTION: u32 = 0x00b4_c838;
/// The keyboard page's rows: 23 records of 16 bytes (`0x00c399ec` up to
/// `0x00c39b5c`): two bindings, message.
pub const KEYBOARD_ROWS: u32 = 0x00c3_99ec;
/// The end of [`KEYBOARD_ROWS`] (the loop's `0x00c39b60` less 4).
pub const KEYBOARD_ROWS_END: u32 = 0x00c3_9b5c;
/// The keyboard rows' second text (`0x00b4c7e8`).
pub const KEYBOARD_TEXT: u32 = 0x00b4_c7e8;
/// The keyboard rows' Flash function name (`0x00b4c7fc`).
pub const KEYBOARD_FUNCTION: u32 = 0x00b4_c7fc;
/// A missing key name's text (`0x00b8fbeb`).
pub const NO_NAME: u32 = 0x00b8_fbeb;

/// Bytes of [`controller_page`]'s frame, from the original's `ebp -
/// 0x1b8` up to `ebp`: the values at `ebp - 0x168`, `-0x1b8`, `-0x188`,
/// `-0x178`, `-0x1a8`, `-0x198` and the text object at `-0x114`.
pub const CONTROLLER_FRAME: u32 = 0x1b8;
/// Bytes of [`keyboard_page`]'s frame, from the original's `ebp - 0x348`
/// up to `ebp`: the eight-value call's copy at `+0` (that callee's `ebp -
/// 0x84`, as the original's call leaves it), the values at `ebp -
/// 0x1b8`, `-0x208`, `-0x228`, `-0x1d8`, `-0x218`, `-0x1c8`, `-0x1f8`,
/// `-0x1e8`, the names at `-0x1a4`, `-0x184`, `-0x1a8`, `-0x198`, the
/// text object at `-0x134`.
pub const KEYBOARD_FRAME: u32 = 0x348;

/// The four words of the value at `at`. Word 1 is never written by the
/// value setters: junk read, `[ebp - off + 4]` never written; 0 (brief
/// 2026-10-24) — the frame holds 0 in the game.
fn value(h: &impl Host, at: u32) -> [u32; 4] {
    [h.read(at), h.read(at + 4), h.read(at + 8), h.read(at + 0xc)]
}

/// `FFEscMenu_initializeControllerSettingPage(this)` (`0x00701fa0`,
/// thiscall; `frame` see [`CONTROLLER_FRAME`]): the controller key names
/// filled, then per row `i` (binding `b`, message `m`): the binding's
/// name `n1` (`Table60_Get(settings; b)`, then its key name) and the
/// default's `n2` (`GetTable_00c8a538(b)`, then its key name), and
/// `callFlashFunction_6(this, name, i, text, m's text, n1, "[ ]", n2)`.
/// Returns the settings pointer (left in `eax` by the loop).
pub fn controller_page(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let ebp = frame.wrapping_add(CONTROLLER_FRAME);
    let local = |off: u32| ebp.wrapping_sub(off);
    h.call_sym(FFESC_INIT_CONTROLLER_KEY_NAMES, Conv::Cdecl, &[]);
    let settings: u32 = h.read(h.rebase(SETTINGS));
    let end = h.rebase(CONTROLLER_ROWS_END);
    let mut row = h.rebase(CONTROLLER_ROWS);
    let mut index = 0u32;
    while (row as i32) < (end as i32) {
        let binding: u32 = h.read(row);
        let bound = h.call_sym(TABLE60_GET, Conv::Thiscall(settings), &[binding]);
        let default = h.call_sym(GET_TABLE_C8A538, Conv::Thiscall(settings), &[binding]);
        let bound_name = h.call_sym(CONTROLLER_NAME, Conv::Cdecl, &[bound]);
        let default_name = h.call_sym(CONTROLLER_NAME, Conv::Cdecl, &[default]);
        h.call_sym(VALUE_STRING, Conv::Cdecl, &[local(0x168), default_name]);
        let v1 = value(h, local(0x168));
        let empty = h.rebase(EMPTY_TEXT);
        h.call_sym(VALUE_STRING, Conv::Cdecl, &[local(0x1b8), empty]);
        let v2 = value(h, local(0x1b8));
        h.call_sym(VALUE_STRING, Conv::Cdecl, &[local(0x188), bound_name]);
        let v3 = value(h, local(0x188));
        let message: u32 = h.read(row + 4);
        let loc = h.call_sym(LOC_LOAD, Conv::Thiscall(local(0x114)), &[message]);
        h.call_sym(LOC_TO_VALUE, Conv::Thiscall(loc), &[local(0x178)]);
        let text = h.rebase(CONTROLLER_TEXT);
        h.call_sym(VALUE_STRING, Conv::Cdecl, &[local(0x1a8), text]);
        let v5 = value(h, local(0x1a8));
        h.call_sym(VALUE_INT, Conv::Cdecl, &[local(0x198), index]);
        let v4 = value(h, local(0x178));
        let v6 = value(h, local(0x198));
        let name = h.rebase(CONTROLLER_FUNCTION);
        let args = flash_args::<26>(this, name, &[v6, v5, v4, v3, v2, v1]);
        h.call_sym_wide(CALL_FLASH_6, &args);
        index = index.wrapping_add(1);
        row = row.wrapping_add(12);
    }
    settings
}

/// `FFEscMenu_initializeKeyboardSettingPage(this)` (`0x00700da0`,
/// thiscall; `frame` see [`KEYBOARD_FRAME`]): the keyboard key names
/// filled, then per row `i` (bindings `a`, `b`, message `m`): the key
/// names of `Table60_Get(settings; a)` and `(b)` and of
/// `GetTable_00c8a538(a)` and `(b)` (a missing one is
/// [`NO_NAME`]'s text), and `callFlashFunction_8(this, name, i, text, m's
/// text, a's, b's, "[ ]", default a's, default b's)`. Its 34 words are past
/// the host's widest call, so the port runs that function's port
/// ([`call_flash_8`]) with its copy where the original's call puts it.
/// Returns the last call's `eax`.
pub fn keyboard_page(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let ebp = frame.wrapping_add(KEYBOARD_FRAME);
    let local = |off: u32| ebp.wrapping_sub(off);
    h.call_sym(FFESC_INIT_KEYBOARD_KEY_NAMES, Conv::Cdecl, &[]);
    let settings: u32 = h.read(h.rebase(SETTINGS));
    let end = h.rebase(KEYBOARD_ROWS_END);
    let mut row = h.rebase(KEYBOARD_ROWS);
    let mut index = 0u32;
    let mut eax = 0;
    while ((row + 4) as i32) < ((end + 4) as i32) {
        let first: u32 = h.read(row);
        let bound_a = h.call_sym(TABLE60_GET, Conv::Thiscall(settings), &[first]);
        let second: u32 = h.read(row + 4);
        let bound_b = h.call_sym(TABLE60_GET, Conv::Thiscall(settings), &[second]);
        let default_a = h.call_sym(GET_TABLE_C8A538, Conv::Thiscall(settings), &[first]);
        let second: u32 = h.read(row + 4);
        let default_b = h.call_sym(GET_TABLE_C8A538, Conv::Thiscall(settings), &[second]);
        // the pointer/value pairs at ebp - 0x24
        let pairs = [
            (local(0x1a4), bound_a),
            (local(0x184), bound_b),
            (local(0x1a8), default_a),
            (local(0x198), default_b),
        ];
        for (k, &(at, key)) in pairs.iter().enumerate() {
            let k = k as u32;
            h.write(local(0x24 - 8 * k), at);
            h.write(local(0x20 - 8 * k), key);
        }
        for (at, key) in pairs {
            let name = h.call_sym(KEYBOARD_NAME, Conv::Cdecl, &[key]);
            h.write(at, name);
            if name == 0 {
                let none = h.rebase(NO_NAME);
                h.write(at, none);
            }
        }
        let t: u32 = h.read(local(0x198));
        let a = h_value_string(h, local(0x1b8), t);
        let t: u32 = h.read(local(0x1a8));
        let b = h_value_string(h, local(0x208), t);
        let empty = h.rebase(EMPTY_TEXT);
        let c = h_value_string(h, local(0x228), empty);
        let t: u32 = h.read(local(0x184));
        let d = h_value_string(h, local(0x1d8), t);
        let t: u32 = h.read(local(0x1a4));
        let e = h_value_string(h, local(0x218), t);
        let message: u32 = h.read(row + 8);
        let loc = h.call_sym(LOC_LOAD, Conv::Thiscall(local(0x134)), &[message]);
        h.call_sym(LOC_TO_VALUE, Conv::Thiscall(loc), &[local(0x1c8)]);
        let text = h.rebase(KEYBOARD_TEXT);
        let g = h_value_string(h, local(0x1f8), text);
        h.call_sym(VALUE_INT, Conv::Cdecl, &[local(0x1e8), index]);
        let f = value(h, local(0x1c8));
        let i = value(h, local(0x1e8));
        let mut values = [0u32; 32];
        for (k, v) in [i, g, f, e, d, c, b, a].into_iter().enumerate() {
            values[4 * k..4 * k + 4].copy_from_slice(&v);
        }
        let name = h.rebase(KEYBOARD_FUNCTION);
        eax = call_flash_8(h, frame, this, name, &values);
        index = index.wrapping_add(1);
        row = row.wrapping_add(16);
    }
    eax
}

/// A Flash call's words: the menu, the function name, the values.
fn flash_args<const N: usize>(menu: u32, name: u32, values: &[[u32; 4]]) -> [u32; N] {
    let mut args = [0u32; N];
    args[0] = menu;
    args[1] = name;
    for (k, v) in values.iter().enumerate() {
        args[2 + 4 * k..6 + 4 * k].copy_from_slice(v);
    }
    args
}

/// `iggyDataValueSet_00a28d70(out, text)`, then the value's words.
fn h_value_string(h: &mut impl Host, out: u32, text: u32) -> [u32; 4] {
    h.call_sym(VALUE_STRING, Conv::Cdecl, &[out, text]);
    value(h, out)
}

/// `sprintf`'s import slot (`0x00b0c3dc`, cdecl).
pub const SPRINTF_IAT: u32 = 0x00b0_c3dc;
/// `"%d"` (`0x00b0f604`).
pub const PERCENT_D: u32 = 0x00b0_f604;
/// The audio page's rows: 4 records of 0x18 bytes (`0x00c3962c`):
/// message, volume range, settings word, text.
pub const AUDIO_ROWS: u32 = 0x00c3_962c;
/// The audio rows' second text (`0x00b4c71c`).
pub const AUDIO_TEXT: u32 = 0x00b4_c71c;
/// The audio rows' Flash function name (`0x00b4c7d0`,
/// `"addAudioPageItem"`).
pub const AUDIO_FUNCTION: u32 = 0x00b4_c7d0;
/// Bytes of [`audio_page`]'s frame, from the original's `ebp - 0x1d8` up
/// to `ebp`: the number text at `ebp - 0x24` (0x20 bytes), the values at
/// `ebp - 0x188`, `-0x1b8`, `-0x1d8`, `-0x1c8`, `-0x1a8`, `-0x198`, the
/// text object at `-0x134`.
pub const AUDIO_FRAME: u32 = 0x1d8;

/// `FFEscMenu_initializeAudioSettingPage(this)` (`0x006ff770`, thiscall;
/// `frame` see [`AUDIO_FRAME`]): per row `i` (message `m`, range `r`,
/// settings word `s`, text `t`): the setting's value as `"%d"`, and
/// `callFlashFunction_6(this, "addAudioPageItem", i, text, m's text, the
/// number, r, t)`. Returns the last call's `eax`.
pub fn audio_page(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let ebp = frame.wrapping_add(AUDIO_FRAME);
    let local = |off: u32| ebp.wrapping_sub(off);
    let settings: u32 = h.read(h.rebase(SETTINGS));
    let rows = h.rebase(AUDIO_ROWS);
    let mut eax = 0;
    for index in 0..4u32 {
        let row = rows + 0x18 * index;
        let word: u32 = h.read(row + 8);
        let setting: u32 = h.read(settings.wrapping_add(word.wrapping_mul(4)));
        let sprintf: u32 = h.read(h.rebase(SPRINTF_IAT));
        let format = h.rebase(PERCENT_D);
        h.call(sprintf, Conv::Cdecl, &[local(0x24), format, setting]);
        let text: u32 = h.read(row + 0xc);
        let v1 = h_value_string(h, local(0x188), text);
        let range: u32 = h.read(row + 4);
        h.call_sym(VALUE_INT, Conv::Cdecl, &[local(0x1b8), range]);
        let v2 = value(h, local(0x1b8));
        let v3 = h_value_string(h, local(0x1d8), local(0x24));
        let message: u32 = h.read(row);
        let loc = h.call_sym(LOC_LOAD, Conv::Thiscall(local(0x134)), &[message]);
        h.call_sym(LOC_TO_VALUE, Conv::Thiscall(loc), &[local(0x1c8)]);
        let text = h.rebase(AUDIO_TEXT);
        let v5 = h_value_string(h, local(0x1a8), text);
        h.call_sym(VALUE_INT, Conv::Cdecl, &[local(0x198), index]);
        let v4 = value(h, local(0x1c8));
        let v6 = value(h, local(0x198));
        let name = h.rebase(AUDIO_FUNCTION);
        let args = flash_args::<26>(this, name, &[v6, v5, v4, v3, v2, v1]);
        eax = h.call_sym_wide(CALL_FLASH_6, &args);
    }
    eax
}

/// `IggyMenu_callFlashFunction_4_0068de50(menu, name, v0..v3)` (cdecl,
/// 18 words).
pub const CALL_FLASH_4: u32 = 0x0068_de50;
/// `"spinnerButton"` (`0x00b4c504`).
pub const SPINNER_BUTTON: u32 = 0x00b4_c504;
/// `"addChangeParaPageItem"` (`0x00b4c514`).
pub const PARAMETER_FUNCTION: u32 = 0x00b4_c514;
/// Bytes of [`parameter_page`]'s frame, from the original's `ebp -
/// 0x26c` up to `ebp`: the values at `ebp - 0x26c`, `-0x25c`, `-0x24c`,
/// `-0x23c`, the text objects at `-0x224` and `-0x114`.
pub const PARAMETER_FRAME: u32 = 0x26c;

/// `FFEscMenu_initializeParameterChangePage(this)` (`0x006fc3f0`,
/// thiscall; `frame` see [`PARAMETER_FRAME`]): three
/// `callFlashFunction_4(this, "addChangeParaPageItem", k,
/// "spinnerButton", m's text, message 0x2c8's text)` for k = 0, 1, 2 and
/// m = 0x2b6, 0x2bd, 0x2c4. The first item builds its values in other
/// frame slots than the next two (both texts and the two values swap
/// places). Returns the last call's `eax`.
pub fn parameter_page(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let ebp = frame.wrapping_add(PARAMETER_FRAME);
    let local = |off: u32| ebp.wrapping_sub(off);
    let mut eax = 0;
    for (k, message) in [0x2b6u32, 0x2bd, 0x2c4].into_iter().enumerate() {
        // (text object, value) of message 0x2c8 and of `message`, the
        // string and the integer
        let (head, item, string, int) = if k == 0 {
            ((0x114, 0x23c), (0x224, 0x26c), 0x24c, 0x25c)
        } else {
            ((0x224, 0x26c), (0x114, 0x23c), 0x25c, 0x24c)
        };
        for (loc, out, id) in [(head.0, head.1, 0x2c8), (item.0, item.1, message)] {
            let loc = h.call_sym(LOC_LOAD, Conv::Thiscall(local(loc)), &[id]);
            h.call_sym(LOC_TO_VALUE, Conv::Thiscall(loc), &[local(out)]);
        }
        let spinner = h.rebase(SPINNER_BUTTON);
        let s = h_value_string(h, local(string), spinner);
        h.call_sym(VALUE_INT, Conv::Cdecl, &[local(int), k as u32]);
        let i = value(h, local(int));
        let m = value(h, local(item.1));
        let first = value(h, local(head.1));
        let name = h.rebase(PARAMETER_FUNCTION);
        let args = flash_args::<18>(this, name, &[i, s, m, first]);
        eax = h.call_sym_wide(CALL_FLASH_4, &args);
    }
    eax
}
