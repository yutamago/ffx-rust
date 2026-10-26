//! Continuation round 2 of list `h4-glue-other`, batch 2: the escape
//! menu's callbacks (sound kinds, the callback table, the language
//! call), the Iggy callback table's add, the button-remap tables, the
//! menu manager's buffer gathering and UI-instance unload, the touch
//! input's reset, and the two scene walks that rebind a shader texture
//! parameter.
//!
//! Bridges that hand callees a local take `frame`, the block laid out as
//! the original's frame from its lowest local up.

use ffx_bridge::Global;
use ffx_graphic::host::{Conv, Host};

// ---- the escape menu

/// `IggyMenuHandlers_add(menu; menu, handler, kind, call)` (thiscall,
/// ret 0x10).
pub const HANDLERS_ADD: u32 = 0x00a2_8e40;
/// `FFEscMenu_playMenuSound`, the handler.
pub const PLAY_MENU_SOUND: u32 = 0x0068_f9b0;
/// `FFEscMenu_callMember_0068dfa0`, the call thunk.
pub const CALL_MEMBER: u32 = 0x0068_dfa0;
/// The sound names and their kinds: `"change"`, `"click"` (1),
/// `"cancel"` (4), `"error"` (3).
pub const SOUND_NAMES: [(u32, u32); 4] = [
    (0x00b4_6f30, 1),
    (0x00b4_6f38, 1),
    (0x00b2_d720, 4),
    (0x00b4_6f40, 3),
];

/// `strcmp`'s sign (-1, 0, 1) of two C strings, unsigned bytes.
fn strcmp(h: &impl Host, a: u32, b: u32) -> u32 {
    let mut k = 0u32;
    loop {
        let x: u8 = h.read(a.wrapping_add(k));
        let y: u8 = h.read(b.wrapping_add(k));
        if x != y {
            return if x < y { u32::MAX } else { 1 };
        }
        if x == 0 {
            return 0;
        }
        k = k.wrapping_add(1);
    }
}

/// `FFEscMenu_playSound(menu, value)` (cdecl, an Iggy callback): the
/// name (`value + 8`) `"change"` or `"click"` registers the menu-sound
/// handler with kind 1, `"cancel"` 4, `"error"` 3
/// (`IggyMenuHandlers_add(menu; menu, playMenuSound, kind, callMember)`).
/// Returns that call's `eax`, or for another name its `strcmp` sign
/// against `"error"`.
pub fn esc_play_sound(h: &mut impl Host, menu: u32, value: u32) -> u32 {
    let name: u32 = h.read(value.wrapping_add(8));
    let mut sign = 0;
    for (text, kind) in SOUND_NAMES {
        sign = strcmp(h, name, h.rebase(text));
        if sign == 0 {
            let handler = h.rebase(PLAY_MENU_SOUND);
            let call = h.rebase(CALL_MEMBER);
            return h.call_sym(
                HANDLERS_ADD,
                Conv::Thiscall(menu),
                &[menu, handler, kind, call],
            );
        }
    }
    sign
}

/// `IggyMenu_addCallback_00a28e90(menu; name, fn)` (thiscall, ret 8).
pub const ADD_CALLBACK: u32 = 0x00a2_8e90;
/// `FFEscMenu_registerAudioPreviewCallbacks(menu)` (thiscall).
pub const REGISTER_AUDIO_PREVIEW: u32 = 0x006f_fa20;
/// The escape menu's sixteen Flash callbacks: (name, handler).
pub const ESC_CALLBACKS: [(u32, u32); 16] = [
    (0x00b4_6cd0, 0x0068_fe60),
    (0x00b4_6ce0, 0x0068_fb00),
    (0x00b4_6cec, 0x006f_f1e0),
    (0x00b4_6d00, 0x006f_f4b0),
    (0x00b4_6d1c, 0x006f_f660),
    (0x00b4_6d38, 0x0070_1680),
    (0x00b4_6d58, 0x0070_1e60),
    (0x00b4_6d78, 0x0070_13e0),
    (0x00b4_6d90, 0x0070_1be0),
    (0x00b4_6db0, 0x0068_e5b0),
    (0x00b4_6dc8, 0x0068_f610),
    (0x00b4_6ddc, 0x0068_f8f0),
    (0x00b4_6df4, 0x0070_1660),
    (0x00b4_6e18, 0x0070_1e40),
    (0x00b4_6e3c, 0x0068_f6f0),
    (0x00b4_6e50, 0x0068_f9e0),
];

/// `FFEscMenu_registerCallbacks(menu)` (thiscall): the sixteen callbacks
/// added (`IggyMenu_addCallback(menu; name, fn)`), then the audio
/// preview's (a tail call). Returns that call's `eax`.
pub fn esc_register_callbacks(h: &mut impl Host, menu: u32) -> u32 {
    for (name, f) in ESC_CALLBACKS {
        let name = h.rebase(name);
        let f = h.rebase(f);
        h.call_sym(ADD_CALLBACK, Conv::Thiscall(menu), &[name, f]);
    }
    h.call_sym(REGISTER_AUDIO_PREVIEW, Conv::Thiscall(menu), &[])
}

/// `LocalizationManager_GetInstance()` (cdecl).
pub const LOCALIZATION: u32 = 0x006d_b070;
/// `iggyDataValueSet_00a28d70(&value, text)` (cdecl): a 16-byte Iggy
/// string value.
pub const IGGY_VALUE_SET: u32 = 0x00a2_8d70;
/// `IggyMenu_callFlashFunction_1(menu, name, value)` (cdecl, the value
/// by value: 4 words).
pub const CALL_FLASH_1: u32 = 0x0068_dd40;
/// `"SetLanguage"`.
pub const SET_LANGUAGE: u32 = 0x00b4_6e64;
/// Each language's code and the frame offset (from `ebp - 0x90`) of the
/// value the original builds it in: 0 `"JPN"`, 1 `"INT"`, 2 `"FRA"`, 3
/// `"SPA"`, 4 `"DEU"`, 5 `"ITA"`, 9 `"KOR"`, 10 `"CHT"`; 6..8 and the
/// others `"INT"` at `+0`.
pub const LANGUAGE_CODES: [(u32, u32); 11] = [
    (0x00b4_6e60, 0x70),
    (0x00b4_6e70, 0x30),
    (0x00b4_6e74, 0x50),
    (0x00b4_6e78, 0x10),
    (0x00b4_6e7c, 0x80),
    (0x00b4_6e80, 0x60),
    (0x00b4_6e70, 0),
    (0x00b4_6e70, 0),
    (0x00b4_6e70, 0),
    (0x00b4_6e84, 0x40),
    (0x00b4_6e88, 0x20),
];
/// Bytes of [`esc_set_language`]'s frame (nine value slots).
pub const SET_LANGUAGE_FRAME: u32 = 0x90;

/// `FFEscMenu_setLanguage_0068f400(menu)` (thiscall; `frame` its locals
/// from `ebp - 0x90`): the localization's language (`+4`) as its code
/// string in an Iggy value, passed to the Flash `SetLanguage`. Returns
/// that call's `eax`.
pub fn esc_set_language(h: &mut impl Host, frame: u32, menu: u32) -> u32 {
    let loc = h.call_sym(LOCALIZATION, Conv::Cdecl, &[]);
    let lang: u32 = h.read(loc.wrapping_add(4));
    let (code, at) = LANGUAGE_CODES
        .get(lang as usize)
        .copied()
        .unwrap_or(LANGUAGE_CODES[6]);
    let value = frame + at;
    let code = h.rebase(code);
    h.call_sym(IGGY_VALUE_SET, Conv::Cdecl, &[value, code]);
    let words: [u32; 4] = std::array::from_fn(|k| h.read(value + 4 * k as u32));
    let name = h.rebase(SET_LANGUAGE);
    h.call_sym(
        CALL_FLASH_1,
        Conv::Cdecl,
        &[menu, name, words[0], words[1], words[2], words[3]],
    )
}

// ---- the Iggy callback table

/// `Phyre_PhyreMalloc_0042fb80(bytes, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `Phyre_PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;

/// The `DJB` hash of a C string (signed bytes), without its top bit.
fn djb_hash(h: &impl Host, s: u32) -> u32 {
    let mut hash = 0x1505u32;
    let mut k = 0u32;
    loop {
        let c: i8 = h.read(s.wrapping_add(k));
        if c == 0 {
            return hash & 0x7fff_ffff;
        }
        hash = hash
            .wrapping_mul(33)
            .wrapping_add(i32::from(c).cast_unsigned());
        k = k.wrapping_add(1);
    }
}

/// `IggyCallbackTable_add_00a266b0(table; menu, name, fn)` (thiscall,
/// ret 0xc): a 16-byte record `{used 1, hash(name), fn, menu}` in the
/// first unused record of the array (`+0x14`, count `+0x10` without its
/// top bit; returns `menu`), else appended: new storage of `(count + 1)`
/// records (`PhyreMalloc(.., 4)`; none: returns 0), the records copied,
/// the old storage freed when owned (count's top bit clear); returns the
/// new count.
pub fn callback_table_add(h: &mut impl Host, this: u32, menu: u32, name: u32, f: u32) -> u32 {
    let hash = djb_hash(h, name);
    let n = h.read::<u32>(this + 0x10) & 0x7fff_ffff;
    let records: u32 = h.read(this + 0x14);
    for i in 0..n {
        if h.read::<u8>(records.wrapping_add(16 * i)) != 0 {
            continue;
        }
        let at = 16 * i;
        h.write(records.wrapping_add(at + 4), hash);
        let records: u32 = h.read(this + 0x14);
        h.write(records.wrapping_add(at + 8), f);
        let records: u32 = h.read(this + 0x14);
        h.write(records.wrapping_add(at), 1u8);
        let records: u32 = h.read(this + 0x14);
        h.write(records.wrapping_add(at + 0xc), menu);
        return menu;
    }
    let bytes = n.wrapping_add(1).wrapping_mul(16);
    let storage = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[bytes, 4]);
    if storage == 0 {
        return 0;
    }
    let n = h.read::<u32>(this + 0x10) & 0x7fff_ffff;
    let mut src: u32 = h.read(this + 0x14);
    for i in 0..n {
        let dst = storage.wrapping_add(16 * i);
        if dst != 0 {
            for w in 0..4 {
                let v: u32 = h.read(src.wrapping_add(4 * w));
                h.write(dst.wrapping_add(4 * w), v);
            }
            src = src.wrapping_add(16);
        }
    }
    let slot = storage.wrapping_add(16 * n);
    if slot != 0 {
        // junk read: [ebp-0x17..-0x15] never written (the original
        // stores the byte 1 at [ebp-0x18] and copies the word); 0
        // (brief 2026-10-24)
        h.write(slot, 1u32);
        h.write(slot + 4, hash);
        h.write(slot + 8, f);
        h.write(slot + 0xc, menu);
    }
    let old: u32 = h.read(this + 0x14);
    let count: u32 = h.read(this + 0x10);
    if old != storage && count >> 31 == 0 && old != 0 {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
    }
    h.write(this + 0x14, storage);
    h.write(this + 0x10, n.wrapping_add(1));
    n.wrapping_add(1)
}

// ---- the button remap

/// `RemapButtonIcon_initPs2MessageInputPcMap`'s words from `+0`.
pub const MESSAGE_INPUT_MAP: [u32; 44] = [
    0x41,
    0,
    0x44,
    1,
    0x48,
    2,
    0x42,
    3,
    0x31,
    4,
    0x32,
    5,
    0x30,
    6,
    0x33,
    7,
    0x34,
    8,
    0x36,
    9,
    0x35,
    0xa,
    0x37,
    0xb,
    0x39,
    0xd,
    0x38,
    0xc,
    0x43,
    9,
    0x46,
    0xa,
    0x4c,
    6,
    0x49,
    5,
    0x40,
    0x4000_0000,
    0x20,
    0x26,
    0x21,
    0x24,
    0x22,
    0x25,
];
/// `RemapButtonIcon_initPs2UIButtonPcMap`'s words from `+0xb0`.
pub const UI_BUTTON_MAP: [u32; 32] = [
    0x1b6, 0x24, 0x1b7, 0x25, 1, 9, 2, 0xb, 4, 8, 8, 0xa, 0x10, 6, 0x40, 5, 0x20, 4, 0x80, 7,
    0x100, 0xd, 0x800, 0xc, 0x1000, 0, 0x2000, 3, 0x4000, 1, 0x8000, 2,
];

/// `RemapButtonIcon_initPs2MessageInputPcMap(this)` (thiscall): the
/// message-input pairs (PS2 code, PC slot) from `+0`. The original leaves
/// `eax` alone.
pub fn init_message_input_map(h: &mut impl Host, this: u32) {
    for (k, &v) in MESSAGE_INPUT_MAP.iter().enumerate() {
        h.write(this.wrapping_add(4 * k as u32), v);
    }
}

/// `RemapButtonIcon_initPs2UIButtonPcMap(this)` (thiscall): the UI
/// button pairs (pad bit, icon) from `+0xb0`. The original leaves `eax`
/// alone.
pub fn init_ui_button_map(h: &mut impl Host, this: u32) {
    for (k, &v) in UI_BUTTON_MAP.iter().enumerate() {
        h.write(this.wrapping_add(0xb0 + 4 * k as u32), v);
    }
}

// ---- the menu manager

/// The per-slot group flags (`0x00ccc804`, words).
pub const MENU_SLOT_GROUPS: u32 = 0x00cc_c804;

/// `MenuManager_gatherMenuBuffers_0067b530(this; which)` (thiscall, ret
/// 4): for chain 0 (`+0x15ec` entries from `+0x2bfc`, output from word
/// `+0x2c50 - n`) or chain 1 (`+0x4244` from `+0x583c`, output at word
/// `0xb16 + (+0x5840) - n`), with `n = +0x5844` slots, the entries of the
/// slots whose group flag is 0 and then those whose flag is 1 are copied
/// one after another into the output words. Returns `n` (re-read), or
/// `which - 1` for another chain.
pub fn gather_menu_buffers(h: &mut impl Host, this: u32, which: u32) -> u32 {
    let (entries, first, out_base, out_word) = match which {
        0 => (0x15ec, 0x2bfc, 0u32, 0x2c50),
        1 => (0x4244, 0x583c, 0xb16, 0x5840),
        _ => return which.wrapping_sub(1),
    };
    let groups = h.rebase(MENU_SLOT_GROUPS);
    let n: u32 = h.read(this + 0x5844);
    let mut out = h.read::<u32>(this + out_word).wrapping_sub(n);
    for group in 0..2u32 {
        let mut i = 0u32;
        loop {
            let n: u32 = h.read(this + 0x5844);
            if i >= n {
                break;
            }
            if h.read::<u32>(groups.wrapping_add(4 * i)) == group {
                let k = h.read::<u32>(this + first).wrapping_sub(n).wrapping_add(i);
                let list: u32 = h.read(this + entries);
                let v: u32 = h.read(list.wrapping_add(k.wrapping_mul(4)));
                h.write(
                    this.wrapping_add(out_base.wrapping_add(out).wrapping_mul(4)),
                    v,
                );
                out = out.wrapping_add(1);
            }
            i += 1;
        }
    }
    h.read(this + 0x5844)
}

/// `MenuDynamicGeometry_resetMaterials(geometry)` (thiscall).
pub const RESET_MATERIALS: u32 = 0x006a_2c60;
/// The menu's unloaded-texture names (`0x00c33610`, `0x00c33614`).
pub const UNLOADED_NAMES: [u32; 2] = [0x00c3_3610, 0x00c3_3614];

/// `MenuManager_SetMenuUiInstanceUnloaded(this; name)` (thiscall, ret 4):
/// in the four instance arrays (16-byte records: geometry `+0`, name
/// `+8`): `+0x2bd4` and `+0x582c` of 0x258 records, `+0x2bdc` and `+0x5834`
/// of 0x320, each record named `name` (`strcmp`) has its geometry's
/// materials reset and its name set to the array's unloaded name.
/// Returns what the original leaves in `eax`: from the last record, 0 (no
/// name), the `strcmp` sign, or the unloaded name.
pub fn set_menu_ui_instance_unloaded(h: &mut impl Host, this: u32, name: u32) -> u32 {
    let mut eax = 0;
    for (array, n, unloaded) in [
        (0x2bd4, 0x258, UNLOADED_NAMES[0]),
        (0x2bdc, 0x320, UNLOADED_NAMES[1]),
        (0x582c, 0x258, UNLOADED_NAMES[0]),
        (0x5834, 0x320, UNLOADED_NAMES[1]),
    ] {
        let unloaded = Global::<u32>::new(unloaded);
        for i in 0..n {
            let records: u32 = h.read(this + array);
            let rec = records.wrapping_add(16 * i);
            let text: u32 = h.read(rec.wrapping_add(8));
            if text == 0 {
                eax = 0;
                continue;
            }
            eax = strcmp(h, text, name);
            if eax != 0 {
                continue;
            }
            let geometry: u32 = h.read(rec);
            if geometry != 0 {
                h.call_sym(RESET_MATERIALS, Conv::Thiscall(geometry), &[]);
            }
            let records: u32 = h.read(this + array);
            eax = unloaded.get(h);
            h.write(records.wrapping_add(16 * i + 8), eax);
        }
    }
    eax
}

// ---- touch input

/// `TouchRecords_Init(base, n)` (cdecl).
pub const TOUCH_RECORDS_INIT: u32 = 0x0068_9a10;
/// `TouchTracks_Init(base, n)` (cdecl).
pub const TOUCH_TRACKS_INIT: u32 = 0x0068_98d0;
/// `GuiTouch_initGroup(group)` (thiscall).
pub const TOUCH_INIT_GROUP: u32 = 0x0068_a420;

/// A `PArray`'s storage replaced by `fresh` (`count` at `this + at`, the
/// storage after it): the old one freed when it differs, is owned (count's
/// top bit clear) and set. Returns the last call's `eax`, else the old
/// count.
fn replace_storage(h: &mut impl Host, this: u32, at: u32, fresh: u32, count: u32) -> u32 {
    let old: u32 = h.read(this + at + 4);
    let n: u32 = h.read(this + at);
    let mut eax = n;
    if old != fresh && n >> 31 == 0 && old != 0 {
        eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
    }
    h.write(this + at + 4, fresh);
    h.write(this + at, count);
    eax
}

/// `GuiTouch_reset_0068a660(this)` (thiscall): the touch records (`+0x150`
/// count, `+0x154` storage) made 8 (`PhyreMalloc(0xa80, 4)`,
/// `TouchRecords_Init(p, 8)`) unless already 8, each 0x150-byte record
/// cleared (`+0x14c` 0, u16 `+0` 0, `+4` -1, `+8` 0, then 32 ten-byte
/// points from `+0xc`: byte `+0`, words `+2` and `+6`); the button groups
/// (`+0x158`/`+0x15c`) made 2 (`PhyreMalloc(0x390, 4)`,
/// `TouchTracks_Init(p, 2)`), each 0x1c8-byte group reset
/// (`GuiTouch_initGroup`). Returns what the original leaves in `eax` (0
/// where that is its caller's).
pub fn touch_reset(h: &mut impl Host, this: u32) -> u32 {
    let mut eax = 0;
    for (at, want, bytes, init) in [
        (0x150u32, 8u32, 0xa80u32, TOUCH_RECORDS_INIT),
        (0x158, 2, 0x390, TOUCH_TRACKS_INIT),
    ] {
        if h.read::<u32>(this + at) != want {
            let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[bytes, 4]);
            eax = p;
            if p != 0 {
                h.call_sym(init, Conv::Cdecl, &[p, want]);
                eax = replace_storage(h, this, at, p, want);
            }
        }
        let mut i = 0u32;
        loop {
            let n = h.read::<u32>(this + at) & 0x7fff_ffff;
            if i >= n {
                if i != 0 {
                    eax = n;
                }
                break;
            }
            let storage: u32 = h.read(this + at + 4);
            if at == 0x150 {
                let rec = storage.wrapping_add(0x150 * i);
                h.write(rec.wrapping_add(0x14c), 0u32);
                h.write(rec, 0u16);
                h.write(rec.wrapping_add(4), u32::MAX);
                h.write(rec.wrapping_add(8), 0u32);
                for k in 0..32u32 {
                    let p = rec.wrapping_add(0xc + 10 * k);
                    h.write(p, 0u8);
                    h.write(p.wrapping_add(2), 0u32);
                    h.write(p.wrapping_add(6), 0u32);
                }
            } else {
                let group = storage.wrapping_add(0x1c8 * i);
                h.call_sym(TOUCH_INIT_GROUP, Conv::Thiscall(group), &[]);
            }
            i += 1;
        }
    }
    eax
}

// ---- the shader texture rebinding walks

/// `PhyreSceneCursor_Seek4(cursor)` (thiscall).
pub const CURSOR_SEEK: u32 = 0x0065_ecb0;
/// `PhyreSceneCursor_Next4(cursor)` (thiscall).
pub const CURSOR_NEXT: u32 = 0x0065_ee40;
/// The class the cursor walks (`0x00ca5268`).
pub const WALK_CLASS: u32 = 0x00ca_5268;
/// `ShaderParams_FindByName(shader; name)` (thiscall, ret 4).
pub const FIND_PARAM: u32 = 0x0056_cbf0;
/// The texture parameter's name (`0x00b3017c`).
pub const WALK_PARAM: u32 = 0x00b3_017c;
/// `Render_SurfaceFormat_0066e4f0(block; param, &slot)` (thiscall, ret
/// 8).
pub const SURFACE_FORMAT: u32 = 0x0066_e4f0;
/// Bytes of [`rebind_textures`]'s frame (0x2c: the cursor and three
/// words) and [`rebind_textures_alt`]'s (0x28: the cursor and two).
pub const REBIND_FRAME: u32 = 0x2c;
/// The same for the alternative walk.
pub const REBIND_ALT_FRAME: u32 = 0x28;

/// The four texture offsets (both formats 2; first only; second only;
/// neither) and the forced one of a walk.
struct Targets {
    by_format: [u32; 4],
    forced: u32,
}

/// The walk's other locals: the record and the parameter (offsets in
/// `frame`), and what it keeps in its caller's argument slots (`slots`,
/// the address of the first; `slot_param` an offset from there, the
/// parameter's offset at `slot_offset`).
struct Locals {
    record: u32,
    param: Option<u32>,
    slots: u32,
    slot_param: Option<u32>,
    slot_offset: u32,
}

/// The shared walk of [`rebind_textures`] and [`rebind_textures_alt`]:
/// the cursor (`frame + 0`, 0x20 bytes) over `owner`'s list (`*owner +
/// 0x1c`) of the walk class, seeked and stepped by the engine; each record
/// (`+0x10 + cur`: shader `+0`, parameter block `+4`) whose shader has the
/// texture parameter gets its slot's texture (`+8`) set to `this` plus the
/// offset its current texture's two format bytes (`+2`, `+3` == 2) choose
/// (or the forced one with `force`), and the block re-formatted
/// (`Render_SurfaceFormat`).
fn walk(
    h: &mut impl Host,
    frame: u32,
    locals: &Locals,
    this: u32,
    owner: u32,
    force: u8,
    targets: &Targets,
) {
    let list = h.read::<u32>(owner).wrapping_add(0x1c);
    let first: u32 = h.read(list);
    let head = if first == list { 0 } else { first };
    for k in 0..5u32 {
        h.write(frame + 4 * k, 0u32);
    }
    h.write(frame + 0x14, list);
    h.write(frame + 0x18, head);
    h.write(frame + 0x1c, h.rebase(WALK_CLASS));
    h.call_sym(CURSOR_SEEK, Conv::Thiscall(frame), &[]);
    let mut left: u32 = h.read(frame + 0xc);
    while left != 0 {
        let mut cur: u32 = h.read(frame);
        let mut end: u32 = h.read(frame + 4);
        loop {
            let record = h.read::<u32>(frame + 0x10).wrapping_add(cur);
            h.write(frame + locals.record, record);
            rebind(h, frame, locals, this, record, force, targets);
            // step to the next entry; at a block's end, the next block
            loop {
                cur = cur.wrapping_add(h.read(frame + 8));
                left = left.wrapping_sub(1);
                h.write(frame, cur);
                h.write(frame + 0xc, left);
                if end != cur {
                    break;
                }
                end = h.read(end);
                h.write(frame + 4, end);
                if left == 0 {
                    break;
                }
            }
            if left == 0 {
                break;
            }
        }
        h.call_sym(CURSOR_NEXT, Conv::Thiscall(frame), &[]);
        left = h.read(frame + 0xc);
    }
}

fn rebind(
    h: &mut impl Host,
    frame: u32,
    locals: &Locals,
    this: u32,
    record: u32,
    force: u8,
    targets: &Targets,
) {
    let shader: u32 = h.read(record);
    let name = h.rebase(WALK_PARAM);
    let param = h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[name]);
    if let Some(at) = locals.param {
        h.write(frame + at, param);
    }
    if let Some(at) = locals.slot_param {
        h.write(locals.slots.wrapping_add(at), param);
    }
    if param == 0 {
        return;
    }
    let offset = u32::from(h.read::<u16>(param.wrapping_add(8)));
    h.write(locals.slots.wrapping_add(locals.slot_offset), offset);
    let len = u32::from(h.read::<u16>(param.wrapping_add(0xa)) & 0x1fff);
    let block: u32 = h.read(record.wrapping_add(4));
    let slot = if len + offset > h.read::<u32>(block) || offset < 4 {
        0
    } else {
        offset.wrapping_add(block)
    };
    let old: u32 = h.read(slot.wrapping_add(8));
    let pick = if old == 0 {
        3
    } else {
        match (
            h.read::<u8>(old.wrapping_add(2)) == 2,
            h.read::<u8>(old.wrapping_add(3)) == 2,
        ) {
            (true, true) => 0,
            (true, false) => 1,
            (false, true) => 2,
            (false, false) => 3,
        }
    };
    h.write(
        slot.wrapping_add(8),
        this.wrapping_add(targets.by_format[pick]),
    );
    if force != 0 {
        h.write(slot.wrapping_add(8), this.wrapping_add(targets.forced));
    }
    let block: u32 = h.read(record.wrapping_add(4));
    h.call_sym(SURFACE_FORMAT, Conv::Thiscall(block), &[param, slot]);
}

/// `ShaderTexture_rebind_006f6c00(this; owner, force)` (thiscall,
/// ret 8; `frame` its locals from `ebp - 0x2c`: the cursor, the record
/// `+0x20`, the parameter `+0x24`, `this` `+0x28`): [`walk`] with the
/// offsets 0x988, 0x9d0, 0x9ac, 0x964 (forced 0x988). Each parameter's
/// offset is also stored in the `owner` argument slot (`slots`, the
/// caller's argument area). Its `eax` is left over from the walk; no
/// caller reads it.
pub fn rebind_textures(
    h: &mut impl Host,
    frame: u32,
    slots: u32,
    this: u32,
    owner: u32,
    force: u32,
) {
    h.write(frame + 0x28, this);
    let locals = Locals {
        record: 0x20,
        param: Some(0x24),
        slots,
        slot_param: None,
        slot_offset: 0,
    };
    let targets = Targets {
        by_format: [0x988, 0x9d0, 0x9ac, 0x964],
        forced: 0x988,
    };
    walk(h, frame, &locals, this, owner, force as u8, &targets);
}

/// `ShaderTexture_rebindAlt_006f6a90(this; owner, alt, force)`
/// (thiscall, ret 0xc; `frame` its locals from `ebp - 0x28`: the cursor,
/// the record `+0x20`, `this` `+0x24`): without `alt` (byte)
/// [`rebind_textures`]`(this; owner, force)`; with it [`walk`] with the
/// offsets 0xa18, 0xa60, 0xa3c, 0x9f4 (forced 0xa18), keeping each
/// parameter and its offset in the `owner` and `alt` argument slots
/// (`slots`, the caller's argument area).
pub fn rebind_textures_alt(
    h: &mut impl Host,
    frame: u32,
    slots: u32,
    this: u32,
    owner: u32,
    alt: u32,
    force: u32,
) {
    h.write(frame + 0x24, this);
    if alt as u8 == 0 {
        h.call_sym(REBIND, Conv::Thiscall(this), &[owner, force]);
        return;
    }
    let locals = Locals {
        record: 0x20,
        param: None,
        slots,
        slot_param: Some(0),
        slot_offset: 4,
    };
    let targets = Targets {
        by_format: [0xa18, 0xa60, 0xa3c, 0x9f4],
        forced: 0xa18,
    };
    walk(h, frame, &locals, this, owner, force as u8, &targets);
}

/// `ShaderTexture_rebind_006f6c00`.
pub const REBIND: u32 = 0x006f_6c00;
