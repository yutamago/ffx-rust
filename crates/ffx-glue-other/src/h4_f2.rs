//! Continuation round 4 of list `h4-glue-other`, batch 2: the render
//! thread's shader-pass switch, the menu manager's material update and
//! buffer reset, the button-icon vertex info lookup, a sound event's
//! constructor and release.

use ffx_graphic::host::{Conv, Host};

use crate::frames::{FIOS_UNIFY, OPERATOR_DELETE, PHYRE_PRINTF};
use crate::h4_c2::USER_FREE_THUNK;
use crate::h4_c3::EVENT_SET_CALLBACK;
use crate::h4_d2::strcmp;
use crate::h4_d4::{MEMCPY, USER_MALLOC};
use crate::h4_f1::{NO_TEXTURE, PHYRE_SCENE, REAL_FRAME_BUFFER};

// ---- the shader-pass switch

/// `PShaderProgramD3D11_set_0059ac40(program; state)`.
pub const PROGRAM_SET: u32 = 0x0059_ac40;
/// `PShaderPassStateD3D11_apply_0059ad20(program; state)`.
pub const PASS_APPLY: u32 = 0x0059_ad20;
/// `Phyre_PRendering_ReturnZero3_00598a20(program; state, pass, slot)`.
pub const RETURN_ZERO3: u32 = 0x0059_8a20;
/// `PShaderResourceTables_unbindAll_00598a80(program; state, pass, slot)`.
pub const UNBIND_ALL: u32 = 0x0059_8a80;
/// `Phyre_PRendering_ReturnZero2_00598a70(program; state, pass)`.
pub const RETURN_ZERO2: u32 = 0x0059_8a70;
/// `Phyre_PRendering_ReturnZero1_00598a60(program; state)`.
pub const RETURN_ZERO1: u32 = 0x0059_8a60;
/// `Phyre_PRendering_PShaderPassNVN_releaseShaderPass(program; state)`.
pub const RELEASE_PASS: u32 = 0x0059_8a30;
/// `PShaderProgramD3D11_bindTables_0059ab50(program; state, pass, slot)`.
pub const BIND_TABLES: u32 = 0x0059_ab50;
/// `PShaderProgramD3D11_bindTextures_0059b010(program; state, pass, slot)`.
pub const BIND_TEXTURES: u32 = 0x0059_b010;
/// `Phyre_PRendering_ForwardIfBound_0059aff0(program; state, pass)`.
pub const FORWARD_IF_BOUND: u32 = 0x0059_aff0;
/// `PShaderResourceTables_bindAll_005934c0(program; state, pass)`.
pub const BIND_ALL: u32 = 0x0059_34c0;

/// `PRenderThread_setShaderPass_0061c400(this; pass)` (thiscall, ret 4;
/// the render thread): `this + 0` the current pass, `this + 4` the
/// device state the programs (pass `+0x14`) set themselves into. Without
/// a current pass the new program is set and applied, all four stages
/// changed. Else a stage `k` (0..3) changes when the programs differ or
/// its table (`+0x20 + 4k`) does; each changed stage of the old program
/// is unbound (`ReturnZero3`, `unbindAll`; the last also `ReturnZero2`),
/// and a new program replaces the old (`ReturnZero1`, apply, release,
/// set). Then `this + 0xc` = pass `+0x10`, the changed stages are bound
/// (tables and textures; the last also `ForwardIfBound`), all tables
/// bound (`bindAll`) and the pass becomes current. Returns `bindAll`'s
/// `eax`.
pub fn set_shader_pass(h: &mut impl Host, this: u32, pass: u32) -> u32 {
    let state = this.wrapping_add(4);
    let new: u32 = h.read(pass.wrapping_add(0x14));
    let prev: u32 = h.read(this);
    let mut changed = 0xfu32;
    if prev == 0 {
        h.call_sym(PROGRAM_SET, Conv::Thiscall(new), &[state]);
        h.call_sym(PASS_APPLY, Conv::Thiscall(new), &[state]);
    } else {
        let old: u32 = h.read(prev.wrapping_add(0x14));
        if old == new {
            changed = 0;
            for k in 0..4u32 {
                let a: u32 = h.read(prev.wrapping_add(0x20 + 4 * k));
                let b: u32 = h.read(pass.wrapping_add(0x20 + 4 * k));
                if a != b {
                    changed |= 1 << k;
                }
            }
        }
        for k in 0..4u32 {
            if changed & (1 << k) == 0 {
                continue;
            }
            let cur: u32 = h.read(this);
            h.call_sym(RETURN_ZERO3, Conv::Thiscall(old), &[state, cur, k]);
            let cur: u32 = h.read(this);
            h.call_sym(UNBIND_ALL, Conv::Thiscall(old), &[state, cur, k]);
            if k == 3 {
                let cur: u32 = h.read(this);
                h.call_sym(RETURN_ZERO2, Conv::Thiscall(old), &[state, cur]);
            }
        }
        if old != new {
            h.call_sym(RETURN_ZERO1, Conv::Thiscall(old), &[state]);
            h.call_sym(PASS_APPLY, Conv::Thiscall(new), &[state]);
            h.call_sym(RELEASE_PASS, Conv::Thiscall(old), &[state]);
            h.call_sym(PROGRAM_SET, Conv::Thiscall(new), &[state]);
        }
    }
    let v: u32 = h.read(pass.wrapping_add(0x10));
    h.write(this.wrapping_add(0xc), v);
    for k in 0..4u32 {
        if changed & (1 << k) == 0 {
            continue;
        }
        h.call_sym(BIND_TABLES, Conv::Thiscall(new), &[state, pass, k]);
        h.call_sym(BIND_TEXTURES, Conv::Thiscall(new), &[state, pass, k]);
        if k == 3 {
            h.call_sym(FORWARD_IF_BOUND, Conv::Thiscall(new), &[state, pass]);
        }
    }
    let r = h.call_sym(BIND_ALL, Conv::Thiscall(new), &[state, pass]);
    h.write(this, pass);
    r
}

// ---- the menu manager

/// `ClassDynamicGeometry_hide(geom)` (thiscall).
pub const GEOMETRY_HIDE: u32 = 0x006a_2370;
/// `0x006a3100(geom; target, name, mesh)` (thiscall): sets the menu
/// geometry's material.
pub const GEOMETRY_SET_MATERIAL: u32 = 0x006a_3100;
/// `"FrameBuffer"` (`0x00b45b3c`).
pub const FRAME_BUFFER: u32 = 0x00b4_5b3c;
/// The message `MenuInstanceUpdateMaterialShader` prints without a
/// record (`0x00b45b48`).
pub const NO_RECORD_MESSAGE: u32 = 0x00b4_5b48;
/// The texture name a failed material keeps (`0x00c33618`).
pub const FAILED_NAME: u32 = 0x00c3_3618;
/// Bytes of [`menu_update_material`]'s frame: the unified file name at
/// `ebp - 0x104`.
pub const UPDATE_MATERIAL_FRAME: u32 = 0x100;

/// `MenuManager_MenuInstanceUpdateMaterialShader(this; slot, rec)`
/// (thiscall, ret 8; `frame` its 0x100-byte name buffer): an empty slot
/// (`+0`, `+4` both 0) returns 0; no record prints `0x00b45b48` (at
/// level 1) and returns `PhyrePrintf`'s `eax`. A record whose texture name
/// (vertex info `+0x94`, its `+0x20`) is the slot's (`+8`) only takes the
/// geometry (`slot + 0`). Else the target: a frame-buffer record
/// (`+0xbc`) the scene's `+0xfa94` unless its byte `+0xfaa0`;
/// `"RealFrameBuffer"` `+0xfa88`; `"NoTexture"` `+0xfa80` unless
/// `+0xfa9c`; else `+0xfa84` unless `+0xfa9d`. A set byte hides the
/// geometry (`ClassDynamicGeometry_hide`, its `eax`). Otherwise the
/// material is set (`0x006a3100(target, name, info + 0)`; a name other
/// than `"RealFrameBuffer"` and `"FrameBuffer"` unified into the buffer
/// first, 0x100 bytes) and the slot keeps the name (`+8`; on failure
/// `0x00c33618`'s). Then the geometry takes the record (`+0x24`), the
/// record the info's `+0x28` (16 bits, `+0xb4`); returns those bits over
/// the info pointer's upper half.
pub fn menu_update_material(h: &mut impl Host, frame: u32, slot: u32, rec: u32) -> u32 {
    if h.read::<u32>(slot) | h.read::<u32>(slot.wrapping_add(4)) == 0 {
        return 0;
    }
    let geom: u32 = h.read(slot);
    if rec == 0 {
        let msg = h.rebase(NO_RECORD_MESSAGE);
        return h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[1, msg]);
    }
    let info: u32 = h.read(rec.wrapping_add(0x94));
    let name: u32 = h.read(info.wrapping_add(0x20));
    let cur: u32 = h.read(slot.wrapping_add(8));
    if strcmp(h, name, cur) != 0 {
        let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
        let (target, hidden) = if h.read::<u32>(rec.wrapping_add(0xbc)) != 0 {
            let flag: u8 = h.read(scene.wrapping_add(0xfaa0));
            (h.read::<u32>(scene.wrapping_add(0xfa94)), flag != 0)
        } else if strcmp(h, name, h.rebase(REAL_FRAME_BUFFER)) == 0 {
            (h.read::<u32>(scene.wrapping_add(0xfa88)), false)
        } else if strcmp(h, name, h.rebase(NO_TEXTURE)) == 0 {
            let flag: u8 = h.read(scene.wrapping_add(0xfa9c));
            (h.read::<u32>(scene.wrapping_add(0xfa80)), flag != 0)
        } else {
            let flag: u8 = h.read(scene.wrapping_add(0xfa9d));
            (h.read::<u32>(scene.wrapping_add(0xfa84)), flag != 0)
        };
        if hidden {
            return h.call_sym(GEOMETRY_HIDE, Conv::Thiscall(geom), &[]);
        }
        let mut file = name;
        if name != 0 && strcmp(h, name, h.rebase(REAL_FRAME_BUFFER)) != 0 {
            let info: u32 = h.read(rec.wrapping_add(0x94));
            let name2: u32 = h.read(info.wrapping_add(0x20));
            if strcmp(h, name2, h.rebase(FRAME_BUFFER)) != 0 {
                let info: u32 = h.read(rec.wrapping_add(0x94));
                let src: u32 = h.read(info.wrapping_add(0x20));
                h.call_sym(FIOS_UNIFY, Conv::Cdecl, &[src, frame, 0x100]);
                file = frame;
            }
        }
        let info: u32 = h.read(rec.wrapping_add(0x94));
        let mesh: u32 = h.read(info);
        let r = h.call_sym(
            GEOMETRY_SET_MATERIAL,
            Conv::Thiscall(geom),
            &[target, file, mesh],
        );
        let kept = if r == 0 {
            let info: u32 = h.read(rec.wrapping_add(0x94));
            h.read::<u32>(info.wrapping_add(0x20))
        } else {
            h.read::<u32>(h.rebase(FAILED_NAME))
        };
        h.write(slot.wrapping_add(8), kept);
    }
    h.write(geom.wrapping_add(0x24), rec);
    let info: u32 = h.read(rec.wrapping_add(0x94));
    let id: u16 = h.read(info.wrapping_add(0x28));
    h.write(rec.wrapping_add(0xb4), id);
    (info & 0xffff_0000) | u32::from(id)
}

/// The four record lists `MenuManager_resetMenuBufferChain_0067dfc0`
/// empties: (records, count); the first two for chain state 1, the last
/// two for state 0.
pub const RESET_LISTS: [(u32, u32); 4] = [
    (0x15e4, 0x2bf8),
    (0x15ec, 0x2bfc),
    (0x423c, 0x5838),
    (0x4244, 0x583c),
];
/// `"Unitialized Text"` (`0x00b45ba0`).
pub const UNINITIALIZED_TEXT: u32 = 0x00b4_5ba0;

/// `MenuManager_resetMenuBufferChain_0067dfc0(this)` (thiscall; jumped
/// to from `graphicMenuReset`): chain state (`+0x5b7c`) 1 empties the
/// first chain's two lists, state 0 the second's (each record's vertex
/// info (`+0x94`) `+4`, `+8`, `+0x20`, `+0`, `+0x28` and the record's
/// `+0xb4` (16 bits), `+0xbc` cleared; counts 0) and flips the state (and
/// `+0x2c50` or `+0x5840` 0); then `+0x5b88` = `"Unitialized Text"`,
/// `+0x5b78` 0, byte `+0x5b74` 0, `+0x5844` 0. Its caller reads no
/// result.
pub fn menu_reset_buffer_chain(h: &mut impl Host, this: u32) {
    let state: u32 = h.read(this.wrapping_add(0x5b7c));
    let (lists, next, count) = match state {
        1 => (&RESET_LISTS[..2], 0u32, 0x2c50u32),
        0 => (&RESET_LISTS[2..], 1, 0x5840),
        _ => (&RESET_LISTS[..0], 0, 0),
    };
    for &(recs, n) in lists {
        let mut i = 0u32;
        while i < h.read::<u32>(this.wrapping_add(n)) {
            for off in [4u32, 8, 0x20, 0, 0x28] {
                let arr: u32 = h.read(this.wrapping_add(recs));
                let rec: u32 = h.read(arr.wrapping_add(4 * i));
                let info: u32 = h.read(rec.wrapping_add(0x94));
                h.write(info.wrapping_add(off), 0u32);
            }
            let arr: u32 = h.read(this.wrapping_add(recs));
            let rec: u32 = h.read(arr.wrapping_add(4 * i));
            h.write(rec.wrapping_add(0xb4), 0u16);
            let arr: u32 = h.read(this.wrapping_add(recs));
            let rec: u32 = h.read(arr.wrapping_add(4 * i));
            h.write(rec.wrapping_add(0xbc), 0u32);
            i += 1;
        }
    }
    if !lists.is_empty() {
        h.write(this.wrapping_add(0x5b7c), next);
        for &(_, n) in lists {
            h.write(this.wrapping_add(n), 0u32);
        }
        h.write(this.wrapping_add(count), 0u32);
    }
    let text = h.rebase(UNINITIALIZED_TEXT);
    h.write(this.wrapping_add(0x5b88), text);
    h.write(this.wrapping_add(0x5b78), 0u32);
    h.write(this.wrapping_add(0x5b74), 0u8);
    h.write(this.wrapping_add(0x5844), 0u32);
}

// ---- the button icons

/// `RemapButtonIcon_sGetInstance()`.
pub const REMAP_INSTANCE: u32 = 0x006e_7710;
/// `RemapButtonIcon_getCurrentControlMachine(remap)` (thiscall).
pub const REMAP_CONTROL_MACHINE: u32 = 0x006e_6450;
/// `strstr`'s import slot (`0x00b0c378`).
pub const STRSTR_IAT: u32 = 0x00b0_c378;
/// `"pad_icon"` (`0x00b41b74`).
pub const PAD_ICON: u32 = 0x00b4_1b74;

/// Whether `name` is set, the current control machine is not 0 and the
/// name contains `"pad_icon"`.
fn pad_icon(h: &mut impl Host, name: u32) -> bool {
    if name == 0 {
        return false;
    }
    let remap = h.call_sym(REMAP_INSTANCE, Conv::Cdecl, &[]);
    if h.call_sym(REMAP_CONTROL_MACHINE, Conv::Thiscall(remap), &[]) == 0 {
        return false;
    }
    let strstr: u32 = h.read(h.rebase(STRSTR_IAT));
    let needle = h.rebase(PAD_ICON);
    h.call(strstr, Conv::Cdecl, &[name, needle]) != 0
}

/// `AbmapManager_GetVertexInfo(this; name, kind)` (thiscall, ret 8): the
/// vertex info of `kind` (0..14; others 0): 0-3 `+0x88`..`+0x94`; 4, 5,
/// 13, 14 one of two by the wide layout (`+4 > 0`, signed): `+0xb0`/
/// `+0x98`, `+0xb8`/`+0xa0`, `+0xb4`/`+0x9c`, `+0xc0`/`+0xa8`; 6 and 11
/// the button icons, wide `+0xc4` for a pad icon name else `+0xbc`,
/// narrow `+0xac`/`+0xa4`; 7 `+0xc8`, 8 `+0xcc`, 10 `+0xe0`; 9 and 12 by
/// `+8 > 0`: `+0xd8`/`+0xd0`, `+0xdc`/`+0xd4`.
pub fn abmap_vertex_info(h: &mut impl Host, this: u32, name: u32, kind: u32) -> u32 {
    let wide = h.read::<u32>(this.wrapping_add(4)).cast_signed() > 0;
    let tall = h.read::<u32>(this.wrapping_add(8)).cast_signed() > 0;
    let off = match kind {
        0..=3 => 0x88 + 4 * kind,
        4 => {
            if wide {
                0xb0
            } else {
                0x98
            }
        }
        5 => {
            if wide {
                0xb8
            } else {
                0xa0
            }
        }
        6 | 11 => match (wide, pad_icon(h, name)) {
            (true, true) => 0xc4,
            (true, false) => 0xbc,
            (false, true) => 0xac,
            (false, false) => 0xa4,
        },
        7 => 0xc8,
        8 => 0xcc,
        9 => {
            if tall {
                0xd8
            } else {
                0xd0
            }
        }
        10 => 0xe0,
        12 => {
            if tall {
                0xdc
            } else {
                0xd4
            }
        }
        13 => {
            if wide {
                0xb4
            } else {
                0x9c
            }
        }
        14 => {
            if wide {
                0xc0
            } else {
                0xa8
            }
        }
        _ => return 0,
    };
    h.read(this.wrapping_add(off))
}

// ---- the sound event

/// `Array8_reserve_0070edb0(array; n)` (thiscall).
pub const ARRAY8_RESERVE: u32 = 0x0070_edb0;

/// `SfxEvent_ctor_0070e710(this; src, volume)` (thiscall, ret 8): an
/// empty instance array (`+0` count, `+4` capacity, `+8` 8-byte
/// entries), `+0xc` 0.0, `+0x10`/`+0x14`/`+0x18`/`+0x1c` 0, `+0x20` 1.0,
/// `+0x24` the volume (f32). The capacity is reserved for `src`'s count
/// (`Array8_reserve`) when above it, then each of `src`'s events (`+8`, 4
/// bytes each) is appended with its flag word 0 (grown to `n + n / 8 + 2`
/// when full: `user_malloc`, the entries copied (`memcpy` of the new
/// count's bytes), the old freed). Returns `this`.
pub fn sfx_event_ctor(h: &mut impl Host, this: u32, src: u32, volume: u32) -> u32 {
    for off in [0u32, 4, 8, 0xc, 0x10] {
        h.write(this.wrapping_add(off), 0u32);
    }
    h.write(this.wrapping_add(0x20), 1.0f32.to_bits());
    h.write(this.wrapping_add(0x14), 0u32);
    h.write(this.wrapping_add(0x24), crate::h4_e1::pass(volume));
    h.write(this.wrapping_add(0x18), 0u32);
    h.write(this.wrapping_add(0x1c), 0u32);
    let n: u32 = h.read(src);
    if n.cast_signed() > h.read::<u32>(this.wrapping_add(4)).cast_signed() {
        h.call_sym(ARRAY8_RESERVE, Conv::Thiscall(this), &[n]);
    }
    let mut i = 0u32;
    while i.cast_signed() < n.cast_signed() {
        let events: u32 = h.read(src.wrapping_add(8));
        let event: u32 = h.read(events.wrapping_add(4 * i));
        let count = h.read::<u32>(this).wrapping_add(1);
        h.write(this, count);
        if count.cast_signed() > h.read::<u32>(this.wrapping_add(4)).cast_signed() {
            let cap = count
                .cast_signed()
                .wrapping_div(8)
                .wrapping_add(2)
                .wrapping_add(count.cast_signed())
                .cast_unsigned();
            let data: u32 = h.read(this.wrapping_add(8));
            if data == 0 {
                let p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[cap.wrapping_mul(8)]);
                h.write(this.wrapping_add(8), p);
            } else {
                let mut p = 0u32;
                if cap.cast_signed() > 0 {
                    p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[cap.wrapping_mul(8)]);
                    if p != 0 {
                        let bytes = h.read::<u32>(this).wrapping_mul(8);
                        let old: u32 = h.read(this.wrapping_add(8));
                        h.call_sym(MEMCPY, Conv::Cdecl, &[p, old, bytes]);
                    }
                }
                let old: u32 = h.read(this.wrapping_add(8));
                h.call_sym(USER_FREE_THUNK, Conv::Cdecl, &[old]);
                h.write(this.wrapping_add(8), p);
            }
            h.write(this.wrapping_add(4), cap);
        }
        let count: u32 = h.read(this);
        let data: u32 = h.read(this.wrapping_add(8));
        let e = data.wrapping_add(count.wrapping_sub(1).wrapping_mul(8));
        if e != 0 {
            h.write(e, event);
            // junk read: [ebp-0xb..-0x9] never written; 0 (brief
            // 2026-10-24): the original stores a stack dword whose low
            // byte it zeroed
            h.write(e.wrapping_add(4), 0u32);
        }
        i += 1;
    }
    this
}

/// `SfxEvent_dtor_0070e850(this)` (thiscall; without its SEH frame):
/// every instance's callback is cleared (`Event::setCallback(event, 0,
/// 0)`, unchecked; count `+0` signed), `+0xc` 0.0, `+0x10`/`+0x14` 0, the
/// fades `+0x18` and `+0x1c` deleted and cleared, the entries (`+8`)
/// freed (`user_free`). Its callers read no result.
pub fn sfx_event_dtor(h: &mut impl Host, this: u32) {
    let n: u32 = h.read(this);
    let mut i = 0u32;
    while i.cast_signed() < n.cast_signed() {
        let data: u32 = h.read(this.wrapping_add(8));
        let event: u32 = h.read(data.wrapping_add(8 * i));
        h.call_sym(EVENT_SET_CALLBACK, Conv::Stdcall, &[event, 0, 0]);
        i += 1;
    }
    for off in [0xcu32, 0x10, 0x14] {
        h.write(this.wrapping_add(off), 0u32);
    }
    for off in [0x18u32, 0x1c] {
        let p: u32 = h.read(this.wrapping_add(off));
        if p != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
            h.write(this.wrapping_add(off), 0u32);
        }
    }
    let data: u32 = h.read(this.wrapping_add(8));
    if data != 0 {
        h.call_sym(USER_FREE_THUNK, Conv::Cdecl, &[data]);
    }
}
