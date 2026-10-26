//! Continuation round 8 of list `h4-glue-other`, batch 4: the escape
//! menu's main setting page.

use ffx_graphic::host::{Conv, Host};

use crate::h4_j2::{flash_args, h_value_string as value_string, value};
use crate::h4_j3::{GET_LOC_TEXT, REMAP_TEXT};

/// `IggyMenu_callFlashFunction_1(menu, name, v0)` (`0x0068dd40`, cdecl).
pub const CALL_FLASH_1: u32 = 0x0068_dd40;
/// `IggyMenu_callFlashFunction_2_0068dd80` (cdecl, 10 words).
pub const CALL_FLASH_2: u32 = 0x0068_dd80;
/// `IggyMenu_callFlashFunction_3_0068ddd0` (cdecl, 14 words).
pub const CALL_FLASH_3: u32 = 0x0068_ddd0;
/// `IggyMenu_callFlashFunction_4_0068de50` (cdecl, 18 words).
pub const CALL_FLASH_4: u32 = 0x0068_de50;
/// `IggyMenu_callFlashFunction_5_0068def0` (cdecl, 22 words).
pub const CALL_FLASH_5: u32 = 0x0068_def0;
/// `iggyDataValueSet_00a28db0(value, b)` (cdecl): a boolean value (words
/// 0 and 2).
pub const VALUE_BOOL: u32 = 0x00a2_8db0;
/// `"setMouseVisible"` (`0x00b46f20`).
pub const SET_MOUSE_VISIBLE: u32 = 0x00b4_6f20;
/// Bytes of [`main_setting_page`]'s frame, from the original's `ebp -
/// 0x69c` up to `ebp`: the boolean value at `+0`, the raw message text at
/// `ebp - 0x104`, the five value slots at `ebp - 0x114`, `-0x224`,
/// `-0x334`, `-0x444`, `-0x554`, each with its text 0x100 bytes below it
/// (`ebp - 0x214` ..).
pub const MAIN_FRAME: u32 = 0x69c;

/// One `setXxx` call of the page: the Flash function's name, its
/// messages in the order their values are built, and whether the values
/// go into the slots from the highest (`ebp - 0x554`) down, as the first
/// call's do, or from `ebp - 0x114` up.
struct Item {
    name: u32,
    messages: &'static [u32],
    high_first: bool,
}

/// The page's calls (`0x0068e6f0`): `setHomePage`, `setSysSettingPage`,
/// `setAudioPage`, `setVideoPage`, `setKeyBindingPage`,
/// `setKeyboardPage`, `setControllerPage`, `setControlBar`,
/// `setSubTitle` (names at `0x00b46e8c`..).
const ITEMS: [Item; 9] = [
    Item {
        name: 0x00b4_6e8c,
        messages: &[0x21f, 0x21d, 0x21b, 0x219, 0x217],
        high_first: true,
    },
    Item {
        name: 0x00b4_6e98,
        messages: &[0x225, 0x223, 0x221],
        high_first: false,
    },
    Item {
        name: 0x00b4_6eac,
        messages: &[0x221],
        high_first: false,
    },
    Item {
        name: 0x00b4_6ebc,
        messages: &[0x223],
        high_first: false,
    },
    Item {
        name: 0x00b4_6ecc,
        messages: &[0x270, 0x26e],
        high_first: false,
    },
    Item {
        name: 0x00b4_6ee0,
        messages: &[0x275, 0x274, 0x26e],
        high_first: false,
    },
    Item {
        name: 0x00b4_6ef0,
        messages: &[0x270, 0x270],
        high_first: false,
    },
    Item {
        name: 0x00b4_6f04,
        messages: &[0x215, 0x213, 0x20e],
        high_first: false,
    },
    Item {
        name: 0x00b4_6f14,
        messages: &[0x225, 0x21d, 0x21b, 0x219],
        high_first: false,
    },
];

/// The value slots (`ebp` offsets), lowest first.
const SLOTS: [u32; 5] = [0x114, 0x224, 0x334, 0x444, 0x554];

/// `FFEscMenu_initializeMainSettingPage(this)` (`0x0068e6f0`, thiscall;
/// `frame` see [`MAIN_FRAME`]): for each of [`ITEMS`], each message's
/// text (`graphicGetLocTextWithLength` into `ebp - 0x104`, then the
/// button-tag remap into the slot's text) as a string value, and the
/// Flash call with the values last built first; then
/// `callFlashFunction_1(this, "setMouseVisible", true)`. Returns that
/// call's `eax`.
///
/// Junk reads (brief 2026-10-24): the value setters never write word 1
/// (nor word 3 of the boolean); the port passes its frame's words there,
/// 0 in the game.
pub fn main_setting_page(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let ebp = frame.wrapping_add(MAIN_FRAME);
    let local = |off: u32| ebp.wrapping_sub(off);
    let raw = local(0x104);
    for item in &ITEMS {
        let n = item.messages.len();
        let mut values = Vec::with_capacity(n);
        for (k, &message) in item.messages.iter().enumerate() {
            let slot = if item.high_first {
                SLOTS[n - 1 - k]
            } else {
                SLOTS[k]
            };
            let text = local(slot + 0x100);
            h.call_sym(GET_LOC_TEXT, Conv::Cdecl, &[message, raw, 0x100]);
            h.call_sym(REMAP_TEXT, Conv::Cdecl, &[raw, text, 0x100]);
            values.push(value_string(h, local(slot), text));
        }
        values.reverse();
        let name = h.rebase(item.name);
        let f = [
            CALL_FLASH_1,
            CALL_FLASH_2,
            CALL_FLASH_3,
            CALL_FLASH_4,
            CALL_FLASH_5,
        ][n - 1];
        match n {
            1 => h.call_sym_wide(f, &flash_args::<6>(this, name, &values)),
            2 => h.call_sym_wide(f, &flash_args::<10>(this, name, &values)),
            3 => h.call_sym_wide(f, &flash_args::<14>(this, name, &values)),
            4 => h.call_sym_wide(f, &flash_args::<18>(this, name, &values)),
            _ => h.call_sym_wide(f, &flash_args::<22>(this, name, &values)),
        };
    }
    h.call_sym(VALUE_BOOL, Conv::Cdecl, &[frame, 1]);
    let visible = value(h, frame);
    let name = h.rebase(SET_MOUSE_VISIBLE);
    h.call_sym_wide(CALL_FLASH_1, &flash_args::<6>(this, name, &[visible]))
}
