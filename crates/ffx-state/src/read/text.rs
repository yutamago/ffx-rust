//! The message texts' macros as the game expands them when it draws a
//! window (`TOGetChangeMacroAddress` `0x008b6660`, `TODrawChangeMacro2`;
//! docs/formats/text.md, "Control codes"), read only: the game's version
//! writes the numbers into its buffers (`0x01869ec4`, `0x01869ee4`) and
//! recomputes the field pickup's spoils (`MsFieldItemGetMes`); this one
//! builds the same bytes from what those read.

use super::{Mem, ReadOnly};
use crate::MessageVar;

/// `g_MsgVars`: 32 pointers per message window to the variables' 8-byte
/// records (`+0` the type the script set, `+1` the type the text reads,
/// `i32 +4` the value; `TOResetMessageVars` copies `+0` to `+1`).
const MSG_VARS: u32 = 0x0186_9ab0;
const VARS_PER_WINDOW: u32 = 32;
/// The variable kind of a slot not in use.
const NO_KIND: u8 = 0xff;
/// `0x025d16c4`: the window strings' table (`{VAR:n}` with `n` from 0x50).
const WINDOW_STRINGS: u32 = 0x025d_16c4;
/// `0x018676f0`: the TO windows' records (`i16 +0x20` the strings' base).
const TO_WINDOWS: u32 = 0x0186_76f0;
/// `TOIntToMes`'s powers of ten (`0x00c5a044 + 4 digits`).
const POWERS: u32 = 0x00c5_a044;
/// The field pickup's spoils (`MsFieldItemGetMes`'s `0x02310ce0`): the
/// counts of items, key items and gear `+0xd0`..`+0xd2`, the gear's id
/// `+0xee`.
const PICKUP_ITEMS: u32 = 0x0231_0db0;
const PICKUP_KEYS: u32 = 0x0231_0db1;
const PICKUP_GEAR: u32 = 0x0231_0db2;
const PICKUP_GEAR_ID: u32 = 0x0231_0dce;
/// `g_LocalizationManager` (`+4` the language, `TOGetFFXLang`).
const LOCALIZATION: u32 = 0x00cd_ed58;
/// How deep macros expand into macros, and the longest text built.
const MAX_DEPTH: u32 = 4;
const MAX_LEN: usize = 4096;

/// `TOSkipCode` (`0x008dfd80`, as `ffx_system::msg::skip_code`): how many
/// bytes the code at the start of `t` takes (at least 1).
fn code_len(t: &[u8]) -> usize {
    let b = |i: usize| t.get(i).copied().unwrap_or(0);
    let n = match b(0) {
        4 => {
            if b(1) > 0x2f {
                2
            } else {
                3
            }
        }
        6..=0x0b | 0x0d | 0x0e | 0x10 | 0x12..=0x2f => 2,
        b'G' => {
            let mut n = 1;
            while b(n) == b'G' {
                n += 1;
            }
            n
        }
        _ => 1,
    };
    n.min(t.len()).max(1)
}

/// `text` with its macros expanded for message window `window`: the
/// variables (`0x12`), the characters' names (`0x13`), items (`0x16`,
/// `0x17`) and key items (`0x23`), each expansion expanded in turn; other
/// codes and macros that expand to no address are kept.
pub(super) fn expand(m: &Mem<'_>, window: u32, text: &[u8]) -> Vec<u8> {
    let mut budget = MAX_EXPANSIONS;
    expand_at(m, window, text, 0, &mut budget)
}

/// At most this many macros are expanded in one text (with every nested
/// one): a text of empty expansions adds no output, so `MAX_LEN` alone does
/// not bound the work (a stale pointer or a mod's text read every frame).
const MAX_EXPANSIONS: u32 = 256;

#[allow(clippy::many_single_char_names)] // the game's own short names
fn expand_at(m: &Mem<'_>, window: u32, text: &[u8], depth: u32, budget: &mut u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() && out.len() < MAX_LEN {
        let n = code_len(&text[i..]);
        let code = &text[i..i + n];
        match (depth < MAX_DEPTH && *budget > 0, code) {
            (true, &[c, arg]) if matches!(c, 0x12 | 0x13 | 0x16 | 0x17 | 0x23) => {
                *budget -= 1;
                match macro_text(m, window, c, arg) {
                    Some(t) => out.extend(expand_at(m, window, &t, depth + 1, budget)),
                    None => out.extend_from_slice(code),
                }
            }
            _ => out.extend_from_slice(code),
        }
        i += n;
    }
    out.truncate(MAX_LEN);
    out
}

/// The text a macro code (`code`, its argument `arg`) stands for, as
/// `TOGetChangeMacroAddress` points at it; `None` where it gives no
/// address (0) or the code itself, or what it reads is not mapped.
fn macro_text(m: &Mem<'_>, window: u32, code: u8, arg: u8) -> Option<Vec<u8>> {
    let n = u32::from(arg.wrapping_sub(0x30));
    let at = match code {
        0x12 if arg >= 0x50 => {
            let strings = m.u32(WINDOW_STRINGS);
            let w = m.u32(TO_WINDOWS.wrapping_add(window.wrapping_mul(4)));
            if strings == 0 || w == 0 {
                return None;
            }
            let index = i32::from(m.i16(w.wrapping_add(0x20))) + i32::from(arg) * 2;
            let entry = strings
                .wrapping_add((index * 2).cast_unsigned())
                .wrapping_sub(0x140);
            u32::from(m.u16(entry)).wrapping_add(strings)
        }
        // an unset slot (kind 0xff) gives the game's empty text (`0x00c5a040`)
        0x12 => return Some(var(m, window, n).map(|v| v.text).unwrap_or_default()),
        0x13 => chr_name(m, n),
        0x16 | 0x17 => item_text(m, n),
        0x23 => ffx_battle::minor::bridge::ms_important_name(&ReadOnly(m.0), n),
        _ => return None,
    };
    readable(m, at)
}

/// The text at `at`, `None` at 0 or where nothing is mapped.
fn readable(m: &Mem<'_>, at: u32) -> Option<Vec<u8>> {
    (at != 0 && m.mapped::<1>(at)).then(|| m.text(at))
}

/// `TOGetSaveChrName(i)` (`0x008ac850`): from 0x14 `MsGetRamChrName`, else
/// `MsGetSaveChrName` (whose English check has no effect on the name).
fn chr_name(m: &Mem<'_>, i: u32) -> u32 {
    let r = ReadOnly(m.0);
    if i.cast_signed() >= 0x14 {
        ffx_battle::actor::bridge::ms_get_ram_chr_name(&r, i)
    } else {
        ffx_battle::menu::bridge::ms_get_save_chr_name(&r, i)
    }
}

/// `MsGetRomItem(i, &strings)`'s record `u16 +0` plus the strings
/// (`TOStdGetItemName`, and the `0x16`/`0x17` codes).
fn item_text(m: &Mem<'_>, id: u32) -> u32 {
    let r = ReadOnly(m.0);
    if ffx_kernel::bridge::ITEM.get(&r) == 0 {
        return 0;
    }
    let (record, strings) = ffx_kernel::bridge::rom_strings(&r, ffx_kernel::bridge::ITEM, id);
    u32::from(m.u16(record)).wrapping_add(strings)
}

/// `TOGetAbilityName_008b8d90(id)`: `MsGetRomAbility`'s record `u16 +0`
/// plus its strings.
fn ability_text(m: &Mem<'_>, id: u32) -> u32 {
    let r = ReadOnly(m.0);
    if ffx_kernel::bridge::A_ABILITY.get(&r) == 0 {
        return 0;
    }
    let (record, strings) = ffx_kernel::bridge::rom_strings(&r, ffx_kernel::bridge::A_ABILITY, id);
    u32::from(m.u16(record)).wrapping_add(strings)
}

/// `TOGetComName_008b8d70(id)`: `MsGetComData(id, &strings)`'s record
/// `u16 +0` plus the strings (0 for an id without a name).
fn command_text(m: &Mem<'_>, id: u32) -> u32 {
    let mut r = ReadOnly(m.0);
    match ffx_battle::stat::bridge::com_data(&mut r, id, true) {
        (record, Some(strings)) if record != 0 => u32::from(m.u16(record)).wrapping_add(strings),
        _ => 0,
    }
}

/// `TOStdGetWeaponName(id)`: `MsGetSaveWeaponName(id)`.
pub(super) fn weapon_text(m: &Mem<'_>, id: u32) -> u32 {
    ffx_battle::save::bridge::ms_get_save_weapon_name(&mut ReadOnly(m.0), id)
}

/// `TOGetFFXLang()`: the localisation manager's language (0 without one).
fn language(m: &Mem<'_>) -> u32 {
    let lm = m.u32(LOCALIZATION);
    if lm == 0 {
        0
    } else {
        m.u32(lm.wrapping_add(4))
    }
}

/// `TOIntToMes(value, out, digits, 0)` (`0x008b8290`, mode 0, the only one
/// the variables use): a negative value as 0; more digits than `digits`
/// keep the low ones (`value % POWERS[digits]`), fewer are padded on the
/// left (`]` for Japanese without the English config, `:` for Korean and
/// Chinese, else 0xfe); then the last `digits` characters are kept.
#[allow(clippy::many_single_char_names)] // the game's own short names
fn int_to_mes(m: &Mem<'_>, value: i32, digits: u32) -> Vec<u8> {
    let mut v = value.max(0);
    let d = usize::try_from(digits).unwrap_or(0);
    let mut s = v.to_string().into_bytes();
    if s.len() > d {
        let p = m.i32(POWERS.wrapping_add(digits.wrapping_mul(4)));
        v = v.checked_rem(p).unwrap_or(0);
        s = v.to_string().into_bytes();
    }
    let mut out = Vec::new();
    if s.len() < d {
        let lang = language(m);
        // `MsGetSaveConfigEnglish` is `xor eax, eax; ret` on PC (0x007851d0):
        // Japanese always pads with 0x5d
        let pad = if lang == 0 {
            0x5d
        } else if lang == 10 || lang == 9 {
            0x3a
        } else {
            0xfe
        };
        out.resize(d - s.len(), pad);
        out.extend_from_slice(&s);
        return out;
    }
    // the last `digits` characters (none for 0 digits)
    s[s.len() - d..].to_vec()
}

/// `TOFuncMacroCmdValue(cmd)` (`0x008b6210`): the value in decimal, with
/// kind 2 the digit-group mark 0x46 before each group of three from the
/// right (the count includes a minus sign).
fn macro_value(value: i32, grouped: bool) -> Vec<u8> {
    let text = value.to_string().into_bytes();
    let mut out = Vec::with_capacity(text.len() + 4);
    let mut left = text.len();
    for (i, &c) in text.iter().enumerate() {
        if left.is_multiple_of(3) && i > 0 && grouped {
            out.push(0x46);
        }
        out.push(c);
        left -= 1;
    }
    out
}

/// The text variables of message window `window` that are set.
pub(super) fn vars(m: &Mem<'_>, window: u32) -> Vec<MessageVar> {
    (0..VARS_PER_WINDOW)
        .filter_map(|n| var(m, window, n))
        .collect()
}

/// Variable `n` of window `window` and its expansion, `None` when its
/// record is not mapped or not in use (kind 0xff).
#[allow(clippy::many_single_char_names)] // the game's own short names
fn var(m: &Mem<'_>, window: u32, n: u32) -> Option<MessageVar> {
    let slot = MSG_VARS.wrapping_add(
        window
            .wrapping_mul(VARS_PER_WINDOW)
            .wrapping_add(n)
            .wrapping_mul(4),
    );
    let e = m.u32(slot);
    if e == 0 || !m.mapped::<8>(e) {
        return None;
    }
    let kind = m.u8(e.wrapping_add(1));
    if kind == NO_KIND {
        return None;
    }
    let value = m.i32(e.wrapping_add(4));
    let mut v = MessageVar {
        slot: u8::try_from(n).unwrap_or(0),
        kind,
        value,
        pickup: NO_KIND,
        ..MessageVar::default()
    };
    let id = value.cast_unsigned();
    v.text = match kind {
        0 => readable(m, id).unwrap_or_default(),
        1 | 2 => macro_value(value, kind == 2),
        0x33 => readable(m, item_text(m, id)).unwrap_or_default(),
        0x34 => readable(m, weapon_text(m, id)).unwrap_or_default(),
        0x35 => readable(m, command_text(m, id)).unwrap_or_default(),
        0x36 => readable(m, ability_text(m, id)).unwrap_or_default(),
        0x37 => pickup(m, &mut v),
        // a macro code: `value / 0xd0` added to it, `value % 0xd0` its
        // argument (the game folds the record so on the first draw)
        0x13..=0x23 => {
            let code = i32::from(kind) + value / 0xd0;
            let arg = (value % 0xd0 + 0x30).to_le_bytes()[0];
            match u8::try_from(code) {
                // the macro's text as it stands: the window's expansion
                // expands it in turn (once, within its budget)
                Ok(c @ (0x13 | 0x16 | 0x17 | 0x23)) => {
                    macro_text(m, window, c, arg).unwrap_or_default()
                }
                _ => Vec::new(),
            }
        }
        0x2b..=0x32 => int_to_mes(m, value, u32::from(kind) - 0x2a),
        _ => Vec::new(),
    };
    Some(v)
}

/// The field pickup (`0x37`): treasure `value`'s record (`msFieldItemGet`:
/// the `takara` table, `+0` kind, `+1` amount, `u16 +2` id) and its name
/// as `TOGetChangeMacroAddress` gives it from the pickup's spoils: an item's
/// or a key item's name, the gear's from the spoils the last draw made
/// (`MsFieldItemGetMes` remakes them at every draw), none for gil (its
/// `TOIntToMes(gil, out, 0, 0)` keeps no digit).
fn pickup(m: &Mem<'_>, v: &mut MessageVar) -> Vec<u8> {
    let r = ReadOnly(m.0);
    let table = ffx_battle::reward::bridge::FIELD_ITEMS.get(&r);
    if table == 0 {
        return Vec::new();
    }
    let (rec, _) = ffx_kernel::bridge::excel_data(&r, v.value, table);
    let kind = m.u8(rec);
    let amount = m.u8(rec.wrapping_add(1));
    let id = m.u16(rec.wrapping_add(2));
    v.pickup = kind;
    match kind {
        2 => {
            v.pickup_id = id;
            v.pickup_count = u32::from(amount);
            readable(m, item_text(m, u32::from(id))).unwrap_or_default()
        }
        10 => {
            v.pickup_id = id;
            v.pickup_count = 1;
            readable(
                m,
                ffx_battle::minor::bridge::ms_important_name(&r, u32::from(id)),
            )
            .unwrap_or_default()
        }
        // the gear the last draw put into the spoils
        5 => {
            if m.u8(PICKUP_GEAR) == 0 || m.u8(PICKUP_ITEMS) != 0 || m.u8(PICKUP_KEYS) != 0 {
                return Vec::new();
            }
            v.pickup_id = m.u16(PICKUP_GEAR_ID);
            v.pickup_count = 1;
            readable(m, weapon_text(m, u32::from(v.pickup_id))).unwrap_or_default()
        }
        // gil: `TOIntToMes(gil, out, 0, 0)` keeps no digit
        _ => {
            v.pickup_count = u32::from(amount) * 100;
            Vec::new()
        }
    }
}
