//! Continuation round 8 of list `h4-glue-other`, batch 3: the menu
//! texture clusters' load, the message button remap and the escape
//! menu's special-feature page.

use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::pass;
use crate::h4_j1::{
    PAD_ICON_PATH, PC_ICON_PATH, REMAP_ICON_UV, REMAP_PAD_UV, REMAP_PC_UV, vibration,
};
use crate::h4_j2::{SETTINGS, VALUE_INT, flash_args, h_value_string as value_string, value};

/// `TOGetFFXLangJmp()` (`0x008ac2f0`, cdecl): the game language.
pub const GET_LANG: u32 = 0x008a_c2f0;
/// `NameList_pushBack_00681ad0(name)` (cdecl): appends a name to the
/// list at [`NAME_LIST`].
pub const NAME_LIST_PUSH: u32 = 0x0068_1ad0;
/// The name list (`0x00ccc840`, the head node's pointer): nodes `{next,
/// prev, std::string name (+8: text or its pointer, +0x18 size, +0x1c
/// capacity), loaded flag +0x20}`.
pub const NAME_LIST: u32 = 0x00cc_c840;
/// `ClusterManager_GetInstance()` (cdecl).
pub const CLUSTER_MANAGER: u32 = 0x0069_a1a0;
/// `ClusterManager_loadPCluster(manager; name)` (thiscall, ret 4).
pub const LOAD_PCLUSTER: u32 = 0x0069_b940;
/// `PApplication::GetApplication()` (cdecl).
pub const GET_APPLICATION: u32 = 0x0062_3b90;
/// `PApplication_FixupClusters(clusters, n)` (cdecl).
pub const FIXUP_CLUSTERS: u32 = 0x0062_3580;
/// `"NoTexture"` (`0x00b41b50`), the one name not loaded.
pub const NO_TEXTURE: u32 = 0x00b4_1b50;
/// The eleven texture paths per language set: Japanese
/// (`0x00c339e8`, `0x00c33a24`) and the others (`0x00c33980`,
/// `0x00c339bc`).
pub const JAPANESE_PATHS: [u32; 2] = [0x00c3_39e8, 0x00c3_3a24];
/// See [`JAPANESE_PATHS`].
pub const OTHER_PATHS: [u32; 2] = [0x00c3_3980, 0x00c3_39bc];
/// Bytes of [`load_texture_clusters`]'s frame: the cluster word at the
/// original's `ebp - 4`, which `FixupClusters` reads.
pub const LOAD_FRAME: u32 = 4;

/// The one path word of the language's first switch (`0x00684f3f`).
fn first_path(lang: u32) -> u32 {
    match lang {
        2 => 0x00c3_39b8,
        3 => 0x00c3_39b0,
        5 => 0x00c3_39b4,
        11 => 0x00c3_3a14,
        _ => 0x00c3_39ac,
    }
}

/// The four path words of the language's second switch (`0x00684f83`).
fn second_paths(lang: u32) -> [u32; 4] {
    match lang {
        1..=5 | 18 => [0x00c5_95e8, 0x00c5_95ec, 0x00c5_9628, 0x00c5_962c],
        9 => [0x00c5_9768, 0x00c5_976c, 0x00c5_97a8, 0x00c5_97ac],
        10 => [0x00c5_9668, 0x00c5_966c, 0x00c5_96e8, 0x00c5_96ec],
        11 => [0x00c5_96a8, 0x00c5_96ac, 0x00c5_9728, 0x00c5_972c],
        _ => [0x00c5_9568, 0x00c5_956c, 0x00c5_95a8, 0x00c5_95ac],
    }
}

/// The two path words every language adds last.
pub const LAST_PATHS: [u32; 2] = [0x00c5_9d5c, 0x00c5_9d64];

/// `GraphicAbmap_loadTextureClusters_00684ef0()` (cdecl; `frame` see
/// [`LOAD_FRAME`]): eleven pairs of texture paths (Japanese or not, the
/// language asked each time), one by the language's first switch, four
/// by its second and the two last, each appended to the name list; then
/// every list entry not loaded yet (`+0x20`) whose name is not exactly
/// `"NoTexture"` is loaded (`ClusterManager_loadPCluster`, the cluster
/// through the frame word to `FixupClusters(&cluster, 1)`) and every one
/// marked loaded. Returns the list head (left in `eax`).
pub fn load_texture_clusters(h: &mut impl Host, frame: u32) -> u32 {
    for k in 0..11u32 {
        let lang = h.call_sym(GET_LANG, Conv::Cdecl, &[]);
        let set = if lang == 0xb {
            JAPANESE_PATHS
        } else {
            OTHER_PATHS
        };
        for table in set {
            push_path(h, table + 4 * k);
        }
    }
    let lang = h.call_sym(GET_LANG, Conv::Cdecl, &[]);
    push_path(h, first_path(lang));
    for sym in second_paths(lang).into_iter().chain(LAST_PATHS) {
        push_path(h, sym);
    }
    let list = h.rebase(NAME_LIST);
    let mut head: u32 = h.read(list);
    let mut node: u32 = h.read(head);
    while node != head {
        if h.read::<u32>(node + 0x20) == 0 {
            let name = node + 8;
            if !is_no_texture(h, name) {
                let text = if h.read::<u32>(name + 0x14) < 0x10 {
                    name
                } else {
                    h.read(name)
                };
                let manager = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
                let cluster = h.call_sym(LOAD_PCLUSTER, Conv::Thiscall(manager), &[text]);
                h.write(frame, cluster);
                h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
                h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[frame, 1]);
            }
            h.write(node + 0x20, 1u32);
            head = h.read(list);
        }
        node = h.read(node);
    }
    head
}

/// `NameList_pushBack` of the path word at image address `sym`.
fn push_path(h: &mut impl Host, sym: u32) {
    let name: u32 = h.read(h.rebase(sym));
    h.call_sym(NAME_LIST_PUSH, Conv::Cdecl, &[name]);
}

/// Whether the `std::string` at `s` is exactly `"NoTexture"` (its
/// `compare(0, size, "NoTexture", 9)` is 0).
fn is_no_texture(h: &impl Host, s: u32) -> bool {
    let size: u32 = h.read(s + 0x10);
    let text = if h.read::<u32>(s + 0x14) < 0x10 {
        s
    } else {
        h.read(s)
    };
    let literal = h.rebase(NO_TEXTURE);
    let n = size.min(9);
    (0..n).all(|k| h.read::<u8>(text + k) == h.read::<u8>(literal + k)) && size == 9
}

// ---- RemapButtonIcon_remapMessgeInputData

/// The input mode countdown both remaps share (`0x01841d24`).
pub const INPUT_COUNTDOWN: u32 = 0x0184_1d24;
/// `"RealFrameBuffer"` (`0x00b45b2c`): a message texture not remapped.
pub const REAL_FRAME_BUFFER: u32 = 0x00b4_5b2c;
/// `RemapButtonIcon_remapIconSize(this; kind, rec)` (`0x006e6db0`,
/// thiscall, ret 8).
pub const REMAP_ICON_SIZE: u32 = 0x006e_6db0;
/// `RemapButtonIcon_remapIconPosition(this; kind, rec, key)`
/// (`0x006e6a50`, thiscall, ret 0xc).
pub const REMAP_ICON_POSITION: u32 = 0x006e_6a50;
/// Bytes of [`remap_message_input_data`]'s frame, from the original's
/// `ebp - 8`: the UV's first word (`+0`, written by the UV call) and the
/// path argument's slot `ebp + 0x10` (`+0x18`), which the UV call
/// writes the second word over.
pub const MESSAGE_FRAME: u32 = 0x1c;

/// `strcmp(a, b) == 0` over game memory.
fn same_text(h: &impl Host, a: u32, b: u32) -> bool {
    let mut k = 0u32;
    loop {
        let (x, y) = (
            h.read::<u8>(a.wrapping_add(k)),
            h.read::<u8>(b.wrapping_add(k)),
        );
        if x != y {
            return false;
        }
        if x == 0 {
            return true;
        }
        k = k.wrapping_add(1);
    }
}

/// `RemapButtonIcon_remapMessgeInputData(this; key, rec, path, kind)`
/// (`0x006e7050`, thiscall, ret 0x10; `frame` see [`MESSAGE_FRAME`]).
///
/// A pad key `*key` in 0x40..0x4e: a single bit in its low four sets the
/// countdown [`INPUT_COUNTDOWN`] to 4; key 0x40 sets it to 4 too and is
/// kept in `+0x158`; another whose low four bits are a multiple of 3 is
/// kept when the mode `+0x150` is not 3 (mode 4). The result is 1 when
/// the countdown is 1..3 and `+0x158` is not 0x40, or the mode is 1..2.
/// The countdown then steps down (to 0xffff from 0; 0 past 4), the mode
/// too (0..4: one less, 16 bits; else 0); with mode 2 or a kept 0x40 the
/// kept key goes to `*key`. Unless `*key` is -1, `*path` is null or
/// `"RealFrameBuffer"`: `*path` set to the PC or pad texture, the key
/// looked up in the 22 pairs at `+0` (index 0 when missing), in modes 2
/// and 3 the index replaced by the first of bits 1, 2, 8, 4 it holds
/// that is not the remembered `1 << +0x154` (0, 1, 3, 2, remembered); with
/// a kept 0x40 and the countdown at most 3 the index is the countdown (0
/// forgets the key); the index's UV into the frame, `remapIconUV`,
/// `remapIconSize`, `remapIconPosition(kind, rec, *key)` and `*key` = -1.
/// Returns the result.
#[allow(clippy::too_many_arguments)]
pub fn remap_message_input_data(
    h: &mut impl Host,
    frame: u32,
    this: u32,
    key: u32,
    rec: u32,
    path: u32,
    kind: u32,
) -> u32 {
    let at = |off: u32| this.wrapping_add(off);
    let slot_v = frame.wrapping_add(0x18);
    h.write(slot_v, path);
    let countdown = h.rebase(INPUT_COUNTDOWN);
    let mut result = 0;
    let k: u32 = h.read(key);
    if (0x40..0x4f).contains(&(k as i32)) {
        let low = k & 0xf;
        if (0..4).any(|c| low == 1 << c) {
            h.write(countdown, 4u32);
        }
        let k: u32 = h.read(key);
        if k == 0x40 {
            h.write(countdown, 4u32);
            let k: u32 = h.read(key);
            h.write(at(0x158), k);
        } else if (k & 0xf).is_multiple_of(3) && h.read::<u32>(at(0x150)) != 3 {
            h.write(at(0x150), 4u32);
            let k: u32 = h.read(key);
            h.write(at(0x158), k);
        }
    }
    let g: u32 = h.read(countdown);
    let less = g.wrapping_sub(1);
    if less <= 2 && h.read::<u32>(at(0x158)) != 0x40 {
        result = 1;
    }
    let mode: u32 = h.read(at(0x150));
    if (mode as i32) < 3 && (mode as i32) > 0 {
        result = 1;
    }
    let g: u32 = h.read(countdown);
    h.write(countdown, if g <= 4 { less & 0xffff } else { 0 });
    let mode: u32 = h.read(at(0x150));
    let mode = if (0..=4).contains(&(mode as i32)) {
        mode.wrapping_sub(1) & 0xffff
    } else {
        0
    };
    h.write(at(0x150), mode);
    if mode == 2 || h.read::<u32>(at(0x158)) == 0x40 {
        let kept: u32 = h.read(at(0x158));
        h.write(key, kept);
    }
    if h.read::<u32>(key) == u32::MAX {
        return result;
    }
    let text: u32 = h.read(path);
    if text == 0 || same_text(h, text, h.rebase(REAL_FRAME_BUFFER)) {
        return result;
    }
    let pad = vibration(h);
    let texture = h.rebase(if pad { PAD_ICON_PATH } else { PC_ICON_PATH });
    h.write(path, texture);
    let wanted: u32 = h.read(key);
    let mut index = 0u32;
    if let Some(i) = (0..22).find(|&i| h.read::<u32>(at(8 * i)) == wanted) {
        index = h.read(at(8 * i + 4));
    }
    let mode: u32 = h.read(at(0x150));
    if (2..=3).contains(&(mode as i32)) {
        for (bit, to) in [(1u32, 0u32), (2, 1), (8, 3), (4, 2)] {
            let remembered = 1u32.wrapping_shl(h.read(at(0x154)));
            if index & bit != 0 && remembered != index & bit {
                index = to;
                h.write(at(0x154), to);
                break;
            }
        }
    }
    if h.read::<u32>(at(0x158)) == 0x40 {
        let g: u32 = h.read(countdown);
        if g <= 3 {
            index = g;
            if g == 0 {
                h.write(at(0x158), u32::MAX);
            }
        }
    }
    let pad = vibration(h);
    let uv = if pad { REMAP_PAD_UV } else { REMAP_PC_UV };
    h.call_sym(uv, Conv::Thiscall(this), &[index, frame, slot_v]);
    let v = pass(h.read(slot_v));
    let u = pass(h.read(frame));
    h.call_sym(REMAP_ICON_UV, Conv::Thiscall(this), &[kind, rec, u, v]);
    h.call_sym(REMAP_ICON_SIZE, Conv::Thiscall(this), &[kind, rec]);
    let k: u32 = h.read(key);
    h.call_sym(REMAP_ICON_POSITION, Conv::Thiscall(this), &[kind, rec, k]);
    h.write(key, u32::MAX);
    result
}

// ---- FFEscMenu_updateSpecialFeaturePage

/// `crt_memset_thunk(p, c, n)` (`0x0094964c`, cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `graphicGetLocTextWithLength(id, buffer, size)` (`0x006412f0`, cdecl).
pub const GET_LOC_TEXT: u32 = 0x0064_12f0;
/// `thunk_FUN_006e9700(src, dst, size)` (`0x006e96b0`, cdecl): a message
/// text's button tags turned into icons, into `dst`.
pub const REMAP_TEXT: u32 = 0x006e_96b0;
/// `strstr`'s import slot (`0x00b0c378`, cdecl).
pub const STRSTR_IAT: u32 = 0x00b0_c378;
/// `KeyNames_getKeyboard_00405af0(slot)` (cdecl).
pub const KEYBOARD_NAME: u32 = 0x0040_5af0;
/// `String_replaceFirst_00702590(text, from, to)` (cdecl).
pub const REPLACE_FIRST: u32 = 0x0070_2590;
/// `IggyMenu_callFlashFunction_5_0068def0(menu, name, v0..v4)` (cdecl,
/// 22 words).
pub const CALL_FLASH_5: u32 = 0x0068_def0;
/// The page's rows: 5 records of 0x18 bytes (`0x00c3a2c0`): binding,
/// three message ids (0 for none).
pub const FEATURE_ROWS: u32 = 0x00c3_a2c0;
/// `"@el@"` (`0x00b41e68`): splits the third message into two lines.
pub const LINE_SPLIT: u32 = 0x00b4_1e68;
/// `"$key$"` (`0x00b4c850`): replaced by the binding's key name.
pub const KEY_TAG: u32 = 0x00b4_c850;
/// `"addSpecialFeaturePageItem"` (`0x00b4c858`).
pub const FEATURE_FUNCTION: u32 = 0x00b4_c858;
/// Bytes of [`special_feature_page`]'s frame, from the original's `ebp -
/// 0x598` up to `ebp`: the texts at `ebp - 0x104`, `-0x204`, `-0x304`,
/// `-0x404` (0x100 each), the raw second message at `-0x504`, the values
/// at `-0x598`, `-0x588`, `-0x578`, `-0x568`, `-0x558`.
pub const FEATURE_FRAME: u32 = 0x598;

/// `FFEscMenu_updateSpecialFeaturePage(this)` (`0x00702660`, thiscall;
/// `frame` see [`FEATURE_FRAME`]): per row `i` (binding `b`, messages
/// `m1`..`m3`): four cleared 0x100-byte texts; `m1`'s text, `m2`'s
/// through the button-tag remap, `m3`'s split at `"@el@"` (the second
/// line moved to its own text); `"$key$"` in the second text replaced by
/// the keyboard name of the settings' binding `b` (`[settings + 0x60 +
/// 4b]`); and `callFlashFunction_5(this, "addSpecialFeaturePageItem", i,
/// m1, m2, m3's first line, its second)`. Returns the last call's `eax`.
pub fn special_feature_page(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let ebp = frame.wrapping_add(FEATURE_FRAME);
    let local = |off: u32| ebp.wrapping_sub(off);
    let settings: u32 = h.read(h.rebase(SETTINGS));
    let rows = h.rebase(FEATURE_ROWS);
    let mut eax = 0;
    for index in 0..5u32 {
        let row = rows + 0x18 * index;
        for text in [0x304, 0x204, 0x104, 0x404] {
            h.write(local(text), 0u8);
            h.call_sym(MEMSET, Conv::Cdecl, &[local(text - 1), 0, 0xff]);
        }
        let m1: u32 = h.read(row + 4);
        if m1 != 0 {
            h.call_sym(GET_LOC_TEXT, Conv::Cdecl, &[m1, local(0x304), 0x100]);
        }
        let m2: u32 = h.read(row + 8);
        if m2 != 0 {
            h.call_sym(GET_LOC_TEXT, Conv::Cdecl, &[m2, local(0x504), 0x100]);
            h.call_sym(
                REMAP_TEXT,
                Conv::Cdecl,
                &[local(0x504), local(0x204), 0x100],
            );
        }
        let m3: u32 = h.read(row + 0xc);
        if m3 != 0 {
            h.call_sym(GET_LOC_TEXT, Conv::Cdecl, &[m3, local(0x104), 0x100]);
        }
        let strstr: u32 = h.read(h.rebase(STRSTR_IAT));
        let split = h.rebase(LINE_SPLIT);
        let at = h.call(strstr, Conv::Cdecl, &[local(0x104), split]);
        if at != 0 {
            h.write(at, 0u8);
            let mut k = 0u32;
            loop {
                let c: u8 = h.read(at.wrapping_add(4 + k));
                h.write(local(0x404).wrapping_add(k), c);
                if c == 0 {
                    break;
                }
                k = k.wrapping_add(1);
            }
        }
        let binding: u32 = h.read(row);
        let code: u32 = h.read(
            settings
                .wrapping_add(0x60)
                .wrapping_add(binding.wrapping_mul(4)),
        );
        let key = h.call_sym(KEYBOARD_NAME, Conv::Cdecl, &[code]);
        let tag = h.rebase(KEY_TAG);
        h.call_sym(REPLACE_FIRST, Conv::Cdecl, &[local(0x204), tag, key]);
        let v1 = value_string(h, local(0x558), local(0x404));
        let v2 = value_string(h, local(0x598), local(0x104));
        let v3 = value_string(h, local(0x588), local(0x204));
        let v4 = value_string(h, local(0x568), local(0x304));
        h.call_sym(VALUE_INT, Conv::Cdecl, &[local(0x578), index]);
        let v5 = value(h, local(0x578));
        let name = h.rebase(FEATURE_FUNCTION);
        let args = flash_args::<22>(this, name, &[v5, v4, v3, v2, v1]);
        eax = h.call_sym_wide(CALL_FLASH_5, &args);
    }
    eax
}
