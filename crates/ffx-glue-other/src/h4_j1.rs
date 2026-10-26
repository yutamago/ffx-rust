//! Continuation round 8 of list `h4-glue-other`, batch 1: the PS2 GS
//! layer's first effect packet, the settings pages' key-name tables, a
//! UI button's remap and the character mesh instances' name suffixes.

use ffx_bridge::x87::{add, load, mul, store};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::pass;
pub use crate::h4_e4::{INPUT_INSTANCE, INPUT_VIBRATION};
pub use crate::h4_i2::OPERATOR_NEW_ARRAY;
use crate::menu::LOC_TEXT;
pub use crate::r3::{REMAP_PAD_UV, REMAP_PC_UV};

// ---- op_gs1_init

/// `op_fog_param_set()` (`0x007ff100`, cdecl).
pub const FOG_PARAM_SET: u32 = 0x007f_f100;
/// The two GS register packets `op_gs1_init` fills (`0x011a0720`, 0xb0
/// bytes apart; `g_OpMdGsWordsA/B/C` are inside them).
pub const GS1_PACKETS: u32 = 0x011a_0720;
/// The fog colour bytes (`g_OpFog`, `0x011a0010`..`0x011a0012`).
pub const FOG_COLOUR: u32 = 0x011a_0010;
/// A copy of the frame word `+0xc` (`0x011a0888`).
pub const GS1_FRAME_COPY: u32 = 0x011a_0888;

/// `op_gs1_init(work, flags)` (`0x007eb760`, cdecl): `op_fog_param_set`,
/// then two GS register packets at [`GS1_PACKETS`] from the environment
/// at `work + 0x120` (flags bit 0) or `work + 0x60`: its words `+0`,
/// `+0xc`, `+0x18`, `+0x24` sign-extended as the frame, test, alpha and
/// zbuf-like registers 0x4c, 0x4e, 0x18, 0x40, then 0x5000d (0x47),
/// 0x21, 0x14, 0x80 << 32 (0x3b), 0 (8) and the fog colour (0x3d). The
/// first packet's `+0xc` word also goes to [`GS1_FRAME_COPY`] and gets
/// bit 32 set; the second's is zero-extended and its colour's high word
/// is written before the rest. Returns `+0x24` (left in `eax`).
pub fn op_gs1_init(h: &mut impl Host, work: u32, flags: u32) -> u32 {
    let env = work.wrapping_add(if flags & 1 == 0 { 0x60 } else { 0x120 });
    let e = |off: u32| env.wrapping_add(off);
    h.call_sym(FOG_PARAM_SET, Conv::Cdecl, &[]);
    let base = h.rebase(GS1_PACKETS);
    let at = |off: u32| base + off;
    let sext = |v: u32| {
        u64::from(v)
            | if v & 0x8000_0000 == 0 {
                0
            } else {
                0xffff_ffff << 32
            }
    };
    let fog = h.rebase(FOG_COLOUR);
    let mut last = 0;
    for (k, packet) in [0u32, 0xb0].into_iter().enumerate() {
        let p = |off: u32| at(packet + off);
        h.write(p(0), 0x8001u32);
        h.write(p(8), 0xeeee_eeeeu32);
        let frame: u32 = h.read(e(0));
        h.write(p(0x10), sext(frame));
        h.write(p(0x18), 0x4cu64);
        let test: u32 = h.read(e(0xc));
        if k == 0 {
            h.write(h.rebase(GS1_FRAME_COPY), test);
            h.write(p(0x20), sext(test) | 1 << 32);
        } else {
            h.write(p(0x20), u64::from(test));
        }
        h.write(p(0x28), 0x4eu64);
        let alpha: u32 = h.read(e(0x18));
        h.write(p(0x30), sext(alpha));
        h.write(p(0x38), 0x18u64);
        // the colour's high word (always 0: three zero-extended bytes)
        if k == 1 {
            h.write(p(0x94), 0u32);
        }
        last = h.read(e(0x24));
        h.write(p(0x40), sext(last));
        let colour = u32::from(h.read::<u8>(fog + 2)) << 16
            | u32::from(h.read::<u8>(fog + 1)) << 8
            | u32::from(h.read::<u8>(fog));
        for (off, v) in [
            (0x48, 0x40u64),
            (0x50, 0x5_000d),
            (0x58, 0x47),
            (0x60, 0x21),
            (0x68, 0x14),
            (0x80, 0),
            (0x88, 8),
            (0x70, 0x80 << 32),
            (0x78, 0x3b),
        ] {
            h.write(p(off), v);
        }
        if k == 0 {
            h.write(p(0x90), u64::from(colour));
        } else {
            h.write(p(0x90), colour);
        }
        h.write(p(0x98), 0x3du64);
    }
    last
}

// ---- the settings pages' key names

/// `graphicGetLocTextWithLength(id, buffer, size)` (cdecl), as
/// [`LOC_TEXT`].
pub const GET_LOC_TEXT: u32 = LOC_TEXT;
/// `KeyNames_setController_00405b60(slot, name)` (cdecl): stores
/// `name` in slot `slot` of `0x00c8a680` (0..0x7f, an empty slot is
/// kept); `al` 1 when stored.
pub const SET_CONTROLLER_NAME: u32 = 0x0040_5b60;
/// `KeyNames_setKeyboard_00405b30(slot, name)` (cdecl): the same for
/// `0x00c8abe0`.
pub const SET_KEYBOARD_NAME: u32 = 0x0040_5b30;
/// The controller names' text buffer (`0x00cea040`, 0x1000 bytes).
pub const CONTROLLER_NAMES: u32 = 0x00ce_a040;
/// The keyboard names' text buffer (`0x00ce9038`, 0x1000 bytes).
pub const KEYBOARD_NAMES: u32 = 0x00ce_9038;
/// Bytes of each name buffer.
pub const NAMES_LEN: u32 = 0x1000;

/// The controller page's (message, slot) pairs, in call order.
pub const CONTROLLER_PAIRS: [(u16, u8); 16] = [
    (0x367, 0xb),
    (0x368, 0xc),
    (0x369, 0xd),
    (0x36a, 0xe),
    (0x36b, 0xf),
    (0x36c, 0x10),
    (0x36d, 0x11),
    (0x36e, 0x12),
    (0x36f, 0x13),
    (0x370, 0x14),
    (0x371, 0x15),
    (0x372, 0x16),
    (0x373, 0x17),
    (0x375, 0x19),
    (0x376, 0x1a),
    (0x374, 0x18),
];

/// The keyboard page's (message, slot) pairs: messages 0x30c..0x366 in
/// order; slots from 0x1c, skipping 0x73 (message 0x363 goes to 0x74).
#[must_use]
pub fn keyboard_pairs() -> Vec<(u16, u8)> {
    (0x30c_u16..=0x366)
        .map(|msg| {
            let slot = msg - 0x30c + 0x1c;
            (msg, (if slot >= 0x73 { slot + 1 } else { slot }) as u8)
        })
        .collect()
}

fn strlen(h: &impl Host, s: u32) -> u32 {
    let mut n = 0u32;
    while h.read::<u8>(s.wrapping_add(n)) != 0 {
        n = n.wrapping_add(1);
    }
    n
}

/// The name tables' fill (`0x00702250` for the controller,
/// `0x006ffc80` for the keyboard; cdecl): for each pair, the message's
/// text into the buffer at the cursor (the space left as its size),
/// then the slot set to it with `setter`, and the cursor moved past its
/// NUL. Returns the last `setter` call's `eax`.
pub fn fill_key_names(h: &mut impl Host, buffer: u32, setter: u32, pairs: &[(u16, u8)]) -> u32 {
    let end = buffer.wrapping_add(NAMES_LEN);
    let mut at = buffer;
    let mut eax = 0;
    for (k, &(msg, slot)) in pairs.iter().enumerate() {
        h.call_sym(
            GET_LOC_TEXT,
            Conv::Cdecl,
            &[u32::from(msg), at, end.wrapping_sub(at)],
        );
        // the last name's length is never taken
        let next = if k + 1 < pairs.len() {
            at.wrapping_add(strlen(h, at)).wrapping_add(1)
        } else {
            at
        };
        eax = h.call_sym(setter, Conv::Cdecl, &[u32::from(slot), at]);
        at = next;
    }
    eax
}

/// `FFEscMenu_initControllerKeyNames_00702250()`.
pub fn fill_controller_names(h: &mut impl Host) -> u32 {
    let buffer = h.rebase(CONTROLLER_NAMES);
    let setter = SET_CONTROLLER_NAME;
    fill_key_names(h, buffer, setter, &CONTROLLER_PAIRS)
}

/// `FFEscMenu_initKeyboardKeyNames_006ffc80()`.
pub fn fill_keyboard_names(h: &mut impl Host) -> u32 {
    let buffer = h.rebase(KEYBOARD_NAMES);
    fill_key_names(h, buffer, SET_KEYBOARD_NAME, &keyboard_pairs())
}

// ---- RemapButtonIcon_remapUIButtonData

/// `RemapButtonIcon_remapIconUV(kind, rec, u, v)` (`0x006e6f20`, ret
/// 0x10; the caller leaves the icon in `ecx`).
pub const REMAP_ICON_UV: u32 = 0x006e_6f20;
/// `RemapButtonIcon_remapUIButtonPosition(kind, rec, key, index)`
/// (`0x006e7470`, ret 0x10; `ecx` the icon).
pub const REMAP_UI_BUTTON_POSITION: u32 = 0x006e_7470;
/// The PC and pad button textures' paths (`0x00c35030`, `0x00c35070`).
pub const PC_ICON_PATH: u32 = 0x00c3_5030;
/// See [`PC_ICON_PATH`].
pub const PAD_ICON_PATH: u32 = 0x00c3_5070;
/// The wide icon's width factor (`0x00b922b0`, f64 10.0).
pub const WIDE_W_SCALE: u32 = 0x00b9_22b0;
/// The icon's width factor (`0x00b43d90`, f64 9.0).
pub const W_SCALE: u32 = 0x00b4_3d90;
/// The icon's height factor (`0x00b4b468`, f64 12.0).
pub const H_SCALE: u32 = 0x00b4_b468;
/// Bytes of [`remap_ui_button_data`]'s frame, from the original's `ebp -
/// 4`: the UV's first word (`+0`) and its argument slot `ebp + 0x10`
/// (`+0x14`), which hold `this` (the `push ecx`) and the path pointer
/// argument until the UV call writes over them.
pub const REMAP_FRAME: u32 = 0x18;

/// `RemapButtonIcon_remapUIButtonData(this; key, rec, path, kind)`
/// (`0x006e7300`, thiscall, ret 0x10; `frame` see [`REMAP_FRAME`]):
/// nothing when `*key` and the pending bit `+0x15c` are both -1. The key
/// is looked up in the 20 pairs at `+0xb0` (key, index); a found index
/// other than -1 goes on, else only a pending bit does (index -1). The
/// texture's path goes to `*path` by the input's vibration flag (PC or
/// pad). A pending bit (`1 << c`, c < 20) gives index c; a negative index
/// other than -1 is kept in `+0x15c` without its sign and becomes the
/// wide 0x80000000, else `+0x15c` is reset to -1. The index's UV (PC or
/// pad, by the flag again) into the frame, `remapIconUV(kind, rec, u,
/// v)`, the far corner `x + this+0x160 * (10 wide, else 9)`, `y +
/// this+0x164 * 12` (`rec + 0x20`, `+0x24`), `remapUIButtonPosition(kind,
/// rec, *key, index)` and `*key` = -1. Returns 0.
#[allow(clippy::too_many_arguments)]
pub fn remap_ui_button_data(
    h: &mut impl Host,
    frame: u32,
    this: u32,
    key: u32,
    rec: u32,
    path: u32,
    kind: u32,
) -> u32 {
    let at = |off: u32| this.wrapping_add(off);
    let slot_v = frame.wrapping_add(0x14);
    // `push ecx` makes the first word: `this` until the UV call writes it
    h.write(frame, this);
    h.write(slot_v, path);
    let wanted: u32 = h.read(key);
    if wanted == u32::MAX && h.read::<u32>(at(0x15c)) == u32::MAX {
        return 0;
    }
    let mut index = u32::MAX;
    let mut found = None;
    for i in 0..20 {
        if h.read::<u32>(at(0xb0 + 8 * i)) == wanted {
            found = Some(h.read::<u32>(at(0xb4 + 8 * i)));
            break;
        }
    }
    match found {
        Some(i) if i != u32::MAX => index = i,
        _ => {
            if h.read::<u32>(at(0x15c)) == u32::MAX {
                return 0;
            }
        }
    }
    let pad = vibration(h);
    let text = h.rebase(if pad { PAD_ICON_PATH } else { PC_ICON_PATH });
    h.write(path, text);
    let pending: u32 = h.read(at(0x15c));
    if pending != u32::MAX
        && let Some(c) = (0..20).find(|&c| pending == 1u32 << c)
    {
        index = c;
    }
    if (index as i32) < 0 && index != u32::MAX {
        h.write(at(0x15c), index & 0x7fff_ffff);
        index = 0x8000_0000;
    } else {
        h.write(at(0x15c), u32::MAX);
    }
    let pad = vibration(h);
    let uv = if pad { REMAP_PAD_UV } else { REMAP_PC_UV };
    h.call_sym(uv, Conv::Thiscall(this), &[index, frame, slot_v]);
    let v = pass(h.read(slot_v));
    let u = pass(h.read(frame));
    h.call_sym(REMAP_ICON_UV, Conv::Thiscall(this), &[kind, rec, u, v]);
    vibration(h);
    let scale = if index == 0x8000_0000 {
        WIDE_W_SCALE
    } else {
        W_SCALE
    };
    let kw: f64 = h.read(h.rebase(scale));
    let x = store(add(mul(load(h.read(at(0x160))), kw), load(h.read(rec))));
    h.write(rec.wrapping_add(0x20), x);
    let kh: f64 = h.read(h.rebase(H_SCALE));
    let y = store(add(
        mul(load(h.read(at(0x164))), kh),
        load(h.read(rec.wrapping_add(4))),
    ));
    h.write(rec.wrapping_add(0x24), y);
    let k: u32 = h.read(key);
    h.call_sym(
        REMAP_UI_BUTTON_POSITION,
        Conv::Thiscall(this),
        &[kind, rec, k, index],
    );
    h.write(key, u32::MAX);
    0
}

/// The input manager's vibration flag (`al`).
fn vibration(h: &mut impl Host) -> bool {
    let manager = h.call_sym(INPUT_INSTANCE, Conv::Cdecl, &[]);
    h.call_sym(INPUT_VIBRATION, Conv::Thiscall(manager), &[]) & 0xff != 0
}

// ---- the mesh instances' name suffixes

/// `PhyreSceneCursor_Begin_006a2260(cursor)` (thiscall).
pub const CURSOR_BEGIN: u32 = 0x006a_2260;
/// `PhyreSceneCursor_Next_006a22d0(cursor)` (thiscall).
pub const CURSOR_NEXT: u32 = 0x006a_22d0;
/// The cursors' class (`0x00ca7228`, `PMeshInstance`).
pub const CURSOR_CLASS: u32 = 0x00ca_7228;
/// `atoi`'s import slot (`0x00b0c40c`, cdecl).
pub const ATOI_IAT: u32 = 0x00b0_c40c;
/// Bytes of [`build_instance_suffixes`]'s frame: the cursor at the
/// original's `ebp - 0x34` (position, link, stride, count, element base,
/// list end, node, class).
pub const SUFFIX_FRAME: u32 = 0x20;

/// Sets the cursor at `cur` up on the list ending at `end` and begins it.
fn cursor_begin(h: &mut impl Host, cur: u32, end: u32) -> u32 {
    h.write(cur.wrapping_add(0x14), end);
    for k in 0..5u32 {
        h.write(cur.wrapping_add(4 * k), 0u32);
    }
    let first: u32 = h.read(end);
    h.write(cur.wrapping_add(0x18), if first == end { 0 } else { first });
    let class = h.rebase(CURSOR_CLASS);
    h.write(cur.wrapping_add(0x1c), class);
    h.call_sym(CURSOR_BEGIN, Conv::Thiscall(cur), &[])
}

/// The original's inline walk over the cursor's elements: while a node
/// counts, `visit` sees the element slot `base + position`; the position
/// then steps by the stride, and where it meets the link the link
/// follows itself (that slot is skipped); at the count's end
/// `PhyreSceneCursor_Next`. Returns the last cursor call's `eax`.
fn walk<H: Host>(h: &mut H, cur: u32, first: u32, mut visit: impl FnMut(&mut H, u32)) -> u32 {
    let mut eax = first;
    let field = |off: u32| cur.wrapping_add(off);
    loop {
        let mut remaining: u32 = h.read(field(0xc));
        if remaining == 0 {
            return eax;
        }
        'element: loop {
            let pos: u32 = h.read(field(0));
            let base: u32 = h.read(field(0x10));
            visit(h, base.wrapping_add(pos));
            let mut pos = pos;
            loop {
                let link: u32 = h.read(field(4));
                pos = pos.wrapping_add(h.read(field(8)));
                remaining = remaining.wrapping_sub(1);
                h.write(field(0), pos);
                h.write(field(0xc), remaining);
                if link == pos {
                    let next: u32 = h.read(link);
                    h.write(field(4), next);
                    if remaining == 0 {
                        break 'element;
                    }
                } else if remaining == 0 {
                    break 'element;
                } else {
                    continue 'element;
                }
            }
        }
        eax = h.call_sym(CURSOR_NEXT, Conv::Thiscall(cur), &[]);
    }
}

/// A name pointer without its tag bit.
fn untag(h: &impl Host, slot: u32) -> u32 {
    let p: u32 = h.read(slot);
    p.wrapping_sub(p & 1)
}

/// `n * 4` as `operator new[]` gets it: -1 when it overflows.
fn array_bytes(n: u32) -> u32 {
    n.saturating_mul(4)
}

/// `ChrMeshInstances_buildSuffixes_006a2390(this)` (thiscall; `frame` see
/// [`SUFFIX_FRAME`]): counts the `PMeshInstance`s of the cluster `[this +
/// 4]` into `+0x18` (a cursor walk), allocates that many words
/// (`operator new[]`) into `+0x14`, and walks again: an instance with no
/// names (`+0x18` without its top bit) gets 0, else an array of that
/// many words whose entries are `atoi` of each name (`+0x24`, tagged
/// pointers) past the prefix ending at the second name's first `_` (none
/// with a single name). Returns the last cursor call's `eax`.
pub fn build_instance_suffixes<H: Host>(h: &mut H, frame: u32, this: u32) -> u32 {
    let at = |off: u32| this.wrapping_add(off);
    h.write(at(0x18), 0u32);
    let cluster: u32 = h.read(at(4));
    let first = cursor_begin(h, frame, cluster.wrapping_add(0x1c));
    walk(h, frame, first, |h, _| {
        let n: u32 = h.read(at(0x18));
        h.write(at(0x18), n.wrapping_add(1));
    });
    let count: u32 = h.read(at(0x18));
    let arrays = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[array_bytes(count)]);
    h.write(at(0x14), arrays);
    let cluster: u32 = h.read(at(4));
    let first = cursor_begin(h, frame, cluster.wrapping_add(0x1c));
    let atoi: u32 = h.read(h.rebase(ATOI_IAT));
    let mut k = 0u32;
    walk(h, frame, first, |h, slot| {
        // `[this + 0x14] + 4k`, read again at each use
        let entry = |h: &H| h.read::<u32>(at(0x14)).wrapping_add(4 * k);
        let instance: u32 = h.read(slot);
        let n = h.read::<u32>(instance.wrapping_add(0x18)) & 0x7fff_ffff;
        if n == 0 {
            let e = entry(h);
            h.write(e, 0u32);
        } else {
            let names: u32 = h.read(instance.wrapping_add(0x24));
            let mut skip = 0u32;
            if n > 1 {
                let s = untag(h, names.wrapping_add(4));
                let len = strlen(h, s);
                loop {
                    let c: u8 = h.read(s.wrapping_add(skip));
                    skip = skip.wrapping_add(1);
                    if c == b'_' || (skip as i32) >= (len as i32) {
                        break;
                    }
                }
            }
            let array = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[array_bytes(n)]);
            let e = entry(h);
            h.write(e, array);
            for j in 0..n {
                let name = untag(h, names.wrapping_add(4 * j));
                let v = h.call(atoi, Conv::Cdecl, &[name.wrapping_add(skip)]);
                let e = entry(h);
                let array: u32 = h.read(e);
                h.write(array.wrapping_add(4 * j), v);
            }
        }
        k = k.wrapping_add(1);
    })
}
