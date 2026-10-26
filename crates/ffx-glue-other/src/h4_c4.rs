//! Continuation round of list `h4-glue-other`, batch 4: the extended
//! blur's render, `FmodVoice`'s event check, the escape menu's
//! six-value Flash call and string replace, the menu wave textures'
//! unload and the menu vertex-info move.
//!
//! Bridges that hand callees a local take `frame`, the block laid out as
//! the original's frame from its lowest local up.

use ffx_bridge::Global;
use ffx_graphic::host::{Conv, Host};

use crate::frames::{ERRCHECK, FMOD_GET_STATE, FMOD_VOICE_FILE, MEMCPY, MEMSET};
use crate::misc::POST_PROCESS_INSTANCE;

// ---- BlurEffectEx

/// `PostProcessManager_copyBuffer(mgr; a, b, c, d, e, f)` (thiscall, ret
/// 0x18).
pub const COPY_BUFFER: u32 = 0x0069_2c40;
/// `BlurEffectEx_blur_006c4290(effect; src, dst)` (thiscall, ret 8).
pub const BLUR_PASS: u32 = 0x006c_4290;

fn copy_buffer(h: &mut impl Host, args: [u32; 6]) {
    let mgr = h.call_sym(POST_PROCESS_INSTANCE, Conv::Cdecl, &[]);
    h.call_sym(COPY_BUFFER, Conv::Thiscall(mgr), &args);
}

/// The texture of render target `rt` (`+0`), or 0 when it is not ready
/// (byte `+4` set).
fn target_texture(h: &impl Host, rt: u32) -> u32 {
    if h.read::<u8>(rt + 4) == 0 {
        h.read(rt)
    } else {
        0
    }
}

/// `BlurEffectEx_render(this)` (`0x006c4cf0`, thiscall): when requested
/// (byte `+0x28`), by mode `+0x2c`: with `+0x3c == 1` first the source
/// copy `copyBuffer(+4, +8, +0x18, 1, 0, 0)`; mode 1 then copies the
/// work target's texture to `+0xc` (`(+4, tex(+0x18), +0xc, 0, 1, 0)`),
/// mode 0 blurs it into `+0xc` (`BlurEffectEx_blur_006c4290(+0; tex(+0x18),
/// +0xc)`) and copies that back (`(+4, tex(+0xc), +0x18, 1, 0, 0)`); the
/// request is then cleared. Other modes do nothing. Returns 0.
pub fn blur_ex_render(h: &mut impl Host, this: u32) -> u32 {
    if h.read::<u8>(this + 0x28) == 0 {
        return 0;
    }
    let mode: u32 = h.read(this + 0x2c);
    if mode > 1 {
        return 0;
    }
    if h.read::<u32>(this + 0x3c) == 1 {
        let [a, b, c] = [4, 8, 0x18].map(|o| h.read::<u32>(this + o));
        copy_buffer(h, [a, b, c, 1, 0, 0]);
    }
    let work: u32 = h.read(this + 0x18);
    let tex = target_texture(h, work);
    if mode == 1 {
        let out: u32 = h.read(this + 0xc);
        let a: u32 = h.read(this + 4);
        copy_buffer(h, [a, tex, out, 0, 1, 0]);
    } else {
        let out: u32 = h.read(this + 0xc);
        let effect: u32 = h.read(this);
        h.call_sym(BLUR_PASS, Conv::Thiscall(effect), &[tex, out]);
        let out: u32 = h.read(this + 0xc);
        let tex = target_texture(h, out);
        let work: u32 = h.read(this + 0x18);
        let a: u32 = h.read(this + 4);
        copy_buffer(h, [a, tex, work, 1, 0, 0]);
    }
    h.write(this + 0x28, 0u8);
    0
}

// ---- FmodVoice

/// `FmodVoice_read(this; id, system)` (thiscall, ret 8).
pub const VOICE_READ: u32 = 0x0070_add0;

/// `FmodVoice_checkEvent_0070a4a0(this; system)` (thiscall, ret 4;
/// `frame` its state word, which starts as `this`): with an event
/// (`+0x14`) whose state query fails (`ERRCHECK` line 0xae), the event is
/// freed through the group `+0x10` (its virtual `+8`, stdcall `(group,
/// event, 1)`, line 0xb4; both set), `+0x14`, `+0x10`, `+0x54` cleared; a
/// voice to repeat (`+0x60 == 1`) with an id (`+0x5c`) counts the repeat
/// (`+0x64`) and is read again (`FmodVoice_read(this; id, system)`), else
/// its play words are cleared. Its one caller (`FmodVoice_updateState`)
/// reads no result.
pub fn voice_check_event(h: &mut impl Host, frame: u32, this: u32, system: u32) {
    h.write(frame, this);
    let event: u32 = h.read(this + 0x14);
    if event == 0 {
        return;
    }
    let file = h.rebase(FMOD_VOICE_FILE);
    let r = h.call_sym(FMOD_GET_STATE, Conv::Stdcall, &[event, frame]);
    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0xae]);
    if r == 0 {
        return;
    }
    let event: u32 = h.read(this + 0x14);
    if event != 0 {
        let group: u32 = h.read(this + 0x10);
        if group != 0 {
            let vtable: u32 = h.read(group);
            let f: u32 = h.read(vtable.wrapping_add(8));
            let r = h.call(f, Conv::Stdcall, &[group, event, 1]);
            h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0xb4]);
        }
    }
    let repeat = h.read::<u32>(this + 0x60) == 1;
    for off in [0x14, 0x10, 0x54] {
        h.write(this + off, 0u32);
    }
    if repeat {
        let id: u32 = h.read(this + 0x5c);
        if id != 0 {
            let n: u32 = h.read(this + 0x64);
            h.write(this + 0x64, n.wrapping_add(1));
            h.call_sym(VOICE_READ, Conv::Thiscall(this), &[id, system]);
            return;
        }
    }
    for off in [0x5c, 0x40, 0x44, 0x60, 0x64, 0x58, 0x48, 0x6c, 0x70, 0x74] {
        h.write(this + off, 0u32);
    }
}

// ---- the escape menu

/// `IggyMenu_callIggyAS3External(menu; target, name, n, values, unused)`
/// (thiscall, ret 0x14).
pub const CALL_AS3: u32 = 0x00a2_8ba0;
/// The bytes of [`call_flash_6`]'s frame: six 16-byte Iggy values, the
/// cookie word.
pub const FLASH_6_FRAME: u32 = 0x64;

/// `IggyMenu_callFlashFunction_6_006fcaf0(menu, name, v0..v5)` (cdecl,
/// the six 16-byte Iggy values by value; `frame` its copy of them, at
/// `ebp - 0x64`): `IggyMenu_callIggyAS3External(menu; 0, name, 6,
/// &values, 1)`. Returns its `eax`.
pub fn call_flash_6(
    h: &mut impl Host,
    frame: u32,
    menu: u32,
    name: u32,
    values: &[u32; 24],
) -> u32 {
    for (k, &v) in values.iter().enumerate() {
        h.write(frame + 4 * k as u32, v);
    }
    h.call_sym(CALL_AS3, Conv::Thiscall(menu), &[0, name, 6, frame, 1])
}

/// `strstr` (the import slot `0x00b0c378`; cdecl).
pub const STRSTR: Global<u32> = Global::new(0x00b0_c378);

fn strlen(h: &impl Host, s: u32) -> u32 {
    let mut n = 0u32;
    while h.read::<u8>(s.wrapping_add(n)) != 0 {
        n = n.wrapping_add(1);
    }
    n
}

/// `String_replaceFirst_00702590(text, from, to)` (cdecl): the first
/// `from` in `text` (`strstr`) replaced by `to` in place, the tail moved
/// down (a shorter `to`) or up from the end (a longer one; the byte at
/// the match itself is not moved), then `to` copied in (`memcpy`).
/// Returns 1 in `al`, 0 when `from` does not occur (its one caller,
/// `FFEscMenu_updateSpecialFeaturePage`, reads no result).
pub fn string_replace_first(h: &mut impl Host, text: u32, from: u32, to: u32) -> u8 {
    let from_len = strlen(h, from);
    let to_len = strlen(h, to);
    let text_len = strlen(h, text);
    let f = STRSTR.get(h);
    let at = h.call(f, Conv::Cdecl, &[text, from]);
    if at == 0 {
        return 0;
    }
    let d = from_len.wrapping_sub(to_len).cast_signed();
    if d > 0 {
        let d = d.cast_unsigned();
        let mut p = at.wrapping_add(to_len);
        loop {
            let c = h.read::<u8>(p.wrapping_add(d));
            if c == 0 {
                break;
            }
            h.write(p, c);
            p = p.wrapping_add(1);
        }
        h.write(p, 0u8);
    } else if d < 0 {
        let grow = d.unsigned_abs();
        let end = text.wrapping_add(text_len);
        h.write(end.wrapping_add(grow), 0u8);
        let mut src = end.wrapping_sub(1);
        if src > at {
            let mut dst = src.wrapping_add(grow);
            while src > at {
                let c = h.read::<u8>(src);
                h.write(dst, c);
                src = src.wrapping_sub(1);
                dst = dst.wrapping_sub(1);
            }
        }
    }
    h.call_sym(MEMCPY, Conv::Cdecl, &[at, to, to_len]);
    1
}

// ---- menu textures

/// `g_GraphicUi` (`0x00ccc81c`).
pub const GRAPHIC_UI: Global<u32> = Global::new(0x00cc_c81c);
/// `g_PhyreScene` (`0x00ccb9d8`).
pub const PHYRE_SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// The main menu's wave texture names (`0x00c59c40`, 70 pointers).
pub const WAVE_TEXTURES: u32 = 0x00c5_9c40;
/// `MenuManager_SetMenuUiInstanceUnloaded(ui; name)` (thiscall).
pub const SET_UI_UNLOADED: u32 = 0x0067_e4f0;
/// `fiosUnifyFilename(name, out, size)` (cdecl).
pub const UNIFY_FILENAME: u32 = 0x0067_9820;
/// `MenuDynamicGeometry_setTextureInMaterialUnloaded(name)` (stdcall).
pub const SET_MATERIAL_UNLOADED: u32 = 0x006a_3740;
/// `PhyreScene_forceUnloadMenuTexture(scene; name)` (thiscall).
pub const FORCE_UNLOAD_MENU_TEXTURE: u32 = 0x0065_d780;
/// The bytes of [`unload_main_menu_wave_texture`]'s frame: the unified
/// name (0x100) and the cookie word.
pub const WAVE_FRAME: u32 = 0x104;

/// `MenuTextureManager_UnloadMainMenuWaveTexture()` (`0x0067efa0`;
/// `frame` the name buffer at `ebp - 0x104`): for each of the 70 wave
/// texture names that is set, the UI instance marked unloaded, the name
/// unified into the buffer (`fiosUnifyFilename(name, buf, 0x100)`), the
/// menu material marked unloaded and the scene's texture force-unloaded.
/// Its one caller (`MenuTextureManager_UnloadSpecialMenuTexture`) reads no
/// result. The geometry call is stdcall; the original leaves the
/// geometry's first word in `ecx` for it, which it does not read.
pub fn unload_main_menu_wave_texture(h: &mut impl Host, frame: u32) {
    let table = h.rebase(WAVE_TEXTURES);
    for k in 0..70u32 {
        let name: u32 = h.read(table + 4 * k);
        if name == 0 {
            continue;
        }
        let ui = GRAPHIC_UI.get(h);
        h.call_sym(SET_UI_UNLOADED, Conv::Thiscall(ui), &[name]);
        h.call_sym(UNIFY_FILENAME, Conv::Cdecl, &[name, frame, 0x100]);
        h.call_sym(SET_MATERIAL_UNLOADED, Conv::Stdcall, &[frame]);
        let scene = PHYRE_SCENE.get(h);
        h.call_sym(FORCE_UNLOAD_MENU_TEXTURE, Conv::Thiscall(scene), &[frame]);
    }
}

// ---- menu vertex info

/// The default word `MenuVertexInfo_move_0067dc70` leaves at `+0x20`
/// (`0x00c33610`).
pub const VERTEX_INFO_DEFAULT: Global<u32> = Global::new(0x00c3_3610);

fn copy_words(h: &mut impl Host, dst: u32, src: u32, n: u32) {
    for k in 0..n {
        let v: u32 = h.read(src.wrapping_add(4 * k));
        h.write(dst.wrapping_add(4 * k), v);
    }
}

/// `MenuVertexInfo_move_0067dc70(from, to)` (stdcall): the vertex info
/// of record `from` (`+0x94`) copied into `to`'s: the words `+4`, `+8`,
/// `+0x20`, `+0`, then the arrays behind `+0x14` (16 words), `+0x1c` (3),
/// `+0x10` (12), `+0x18` (8), `+0xc` (12), in that order; then `from`'s
/// cleared: `+4`, `+8`, `+0x20` = the default word, `+0`, and its arrays
/// zeroed (`memset` for `+0x14`, `+0x10`, `+0xc`). Returns the last
/// `memset`'s `eax`.
pub fn vertex_info_move(h: &mut impl Host, from: u32, to: u32) -> u32 {
    let a: u32 = h.read(from + 0x94);
    let b: u32 = h.read(to + 0x94);
    for off in [4, 8, 0x20, 0] {
        let v: u32 = h.read(a.wrapping_add(off));
        h.write(b.wrapping_add(off), v);
    }
    for (off, n) in [(0x14, 16), (0x1c, 3), (0x10, 12), (0x18, 8), (0xc, 12)] {
        let src: u32 = h.read(a.wrapping_add(off));
        let dst: u32 = h.read(b.wrapping_add(off));
        copy_words(h, dst, src, n);
    }
    h.write(a.wrapping_add(4), 0u32);
    h.write(a.wrapping_add(8), 0u32);
    let v = VERTEX_INFO_DEFAULT.get(h);
    h.write(a.wrapping_add(0x20), v);
    h.write(a, 0u32);
    let p: u32 = h.read(a.wrapping_add(0x14));
    h.call_sym(MEMSET, Conv::Cdecl, &[p, 0, 0x40]);
    let p: u32 = h.read(a.wrapping_add(0x1c));
    for k in 0..3 {
        h.write(p.wrapping_add(4 * k), 0u32);
    }
    let p: u32 = h.read(a.wrapping_add(0x10));
    h.call_sym(MEMSET, Conv::Cdecl, &[p, 0, 0x30]);
    let p: u32 = h.read(a.wrapping_add(0x18));
    for k in 0..8 {
        h.write(p.wrapping_add(4 * k), 0u32);
    }
    let p: u32 = h.read(a.wrapping_add(0xc));
    h.call_sym(MEMSET, Conv::Cdecl, &[p, 0, 0x30])
}
