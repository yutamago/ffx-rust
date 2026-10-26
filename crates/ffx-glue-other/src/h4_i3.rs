//! Continuation round 7 of list `h4-glue-other`, batch 3: the blur
//! effect's constructor, the motion blur's render, the save/load menu's
//! open, the boot text's update and `GDraw`'s resource limits and setup.

use ffx_bridge::x87::{load, store};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::pass;
use crate::h4_i1::MEMSET;
use crate::h4_i2::{BEGIN_SCENE, END_SCENE, OPERATOR_NEW, RENDER_MESH_INSTANCE};

// ---- BlurEffect's constructor

/// `Phyre_PhyreMalloc(n)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb40;
/// `Phyre_PCluster_operator_new(cluster)` (thiscall): the cluster's
/// constructor.
pub const CLUSTER_CTOR: u32 = 0x0057_6630;
/// `PostEffectParams_ctor_00702ff0(params)` (thiscall).
pub const POST_EFFECT_PARAMS_CTOR: u32 = 0x0070_2ff0;
/// `RadialLine_ctor_006f7a30(line)` (thiscall).
pub const RADIAL_LINE_CTOR: u32 = 0x006f_7a30;
/// The image's 0.5 (`0x00b92194`).
pub const HALF: u32 = 0x00b9_2194;

/// `BlurEffect_ctor_006c32d0(this)` (thiscall, from
/// `BlurEffect_createInstance`; the SEH frame is not kept): byte `+0xc`
/// 0, `+0x10` 2, `+0x14`, `+0x28`..`+0x30` and `+0x20` 0; the colour
/// blocks `+0x3c`..`+0x80` set to 1.0 except `+0x48`, `+0x58` and `+0x68`
/// (the image's 0.5, through `fld`/`fst`); a 0x50-byte cluster
/// (`PhyreMalloc`, its list head `+0x48` empty) at `+0x24`, the post
/// effect parameters (0x70 bytes) at `+0x18` and a radial line (0x78
/// bytes) at `+0x1c`, each 0 when its allocation fails. Returns `this`.
pub fn blur_effect_ctor(h: &mut impl Host, this: u32) -> u32 {
    let at = |off: u32| this.wrapping_add(off);
    let one = 1.0f32.to_bits();
    h.write(at(0xc), 0u8);
    h.write(at(0x10), 2u32);
    h.write(at(0x14), 0u32);
    for off in [0x28u32, 0x2c, 0x30] {
        h.write(at(off), 0u32);
    }
    for off in [0x3cu32, 0x40, 0x44] {
        h.write(at(off), one);
    }
    let half = pass(h.read(h.rebase(HALF)));
    h.write(at(0x48), half);
    h.write(at(0x58), half);
    for off in [0x4cu32, 0x50, 0x54, 0x5c, 0x60, 0x64] {
        h.write(at(off), one);
    }
    h.write(at(0x68), half);
    for off in [0x6cu32, 0x70, 0x74, 0x78, 0x7c, 0x80] {
        h.write(at(off), one);
    }
    h.write(at(0x20), 0u32);
    let cluster = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x50]);
    if cluster != 0 {
        h.call_sym(CLUSTER_CTOR, Conv::Thiscall(cluster), &[]);
        let head = cluster.wrapping_add(0x48);
        h.write(head, head);
        h.write(head.wrapping_add(4), head);
    }
    h.write(at(0x24), cluster);
    let block = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x70]);
    let params = if block == 0 {
        0
    } else {
        h.call_sym(POST_EFFECT_PARAMS_CTOR, Conv::Thiscall(block), &[])
    };
    h.write(at(0x18), params);
    let block = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x78]);
    let line = if block == 0 {
        0
    } else {
        h.call_sym(RADIAL_LINE_CTOR, Conv::Thiscall(block), &[])
    };
    h.write(at(0x1c), line);
    this
}

// ---- MotionBlur_renderMotionBlur

/// `PostProcessManager_GetInstance()` (cdecl).
pub const POST_PROCESS_MANAGER: u32 = 0x0069_2be0;
/// `PostProcessManager_copyBuffer(manager; renderer, src, dst, 1, 0, 0)`
/// (thiscall, ret 0x18).
pub const COPY_BUFFER: u32 = 0x0069_2c40;
/// `Phyre_PRendering_PRenderer_setColorTarget(renderer; surface, i)`
/// (thiscall, ret 8).
pub const SET_COLOR_TARGET: u32 = 0x005b_b130;
/// `Phyre_PRendering_PRenderSurface_PRenderSurface(surface; target, 0)`
/// (thiscall, ret 8).
pub const SURFACE_CTOR: u32 = 0x005b_35c0;
/// `Phyre_PRendering_PRenderSurface_dtor_PRenderSurface(surface)`
/// (thiscall, a bare `ret`).
pub const SURFACE_DTOR: u32 = 0x005b_3640;
/// `ShaderParams_FindByName(params; name)` (thiscall, ret 4).
pub const FIND_PARAMETER: u32 = 0x0056_cbf0;
/// `ShaderParamBuffer_SetValue(buffer; parameter, values)` (thiscall,
/// ret 8).
pub const SET_VALUE: u32 = 0x0063_8780;
/// `"UvScaleBias"` (`0x00b425e4`).
pub const UV_SCALE_BIAS: u32 = 0x00b4_25e4;
/// The motion blur's render pass (`0x00cdec0c`).
pub const MOTION_PASS: u32 = 0x00cd_ec0c;
/// Bytes of [`motion_blur_render`]'s frame, from `ebp - 0x2c`: the
/// render surface (`+0`, 0x10 bytes) and the scale-bias values (`+0x10`).
pub const MOTION_FRAME: u32 = 0x20;

/// `MotionBlur_renderMotionBlur(this; renderer, target)` (`0x006c5410`,
/// thiscall, ret 8; `frame` see [`MOTION_FRAME`]; the SEH frame is not
/// kept): 0 unless enabled (`+0x58`); a pending skip (`+0x59`) is cleared
/// and 0 returned; 0x13 without shader parameters (`+0x64`). Else the
/// post-process manager's current buffer copied to its `+0xac`; the
/// colour target set to `target` (through a render surface, with the
/// shader's `UvScaleBias` set to (1, 1, 0, 0)) or to none; the scene
/// begun, the motion pass set, the quad `+0x54` drawn and the scene
/// ended. Returns `beginScene`'s result when it fails, else `endScene`'s.
pub fn motion_blur_render(
    h: &mut impl Host,
    frame: u32,
    this: u32,
    renderer: u32,
    target: u32,
) -> u32 {
    let at = |off: u32| this.wrapping_add(off);
    if h.read::<u8>(at(0x58)) == 0 {
        return 0;
    }
    if h.read::<u8>(at(0x59)) != 0 {
        h.write(at(0x59), 0u8);
        return 0;
    }
    if h.read::<u32>(at(0x64)) == 0 {
        return 0x13;
    }
    let manager = h.call_sym(POST_PROCESS_MANAGER, Conv::Cdecl, &[]);
    let current: u32 = h.read(manager.wrapping_add(0x8c));
    let src = if h.read::<u8>(current.wrapping_add(4)) == 0 {
        h.read(current)
    } else {
        0
    };
    let dst: u32 = h.read(manager.wrapping_add(0xac));
    h.call_sym(
        COPY_BUFFER,
        Conv::Thiscall(manager),
        &[renderer, src, dst, 1, 0, 0],
    );
    if target == 0 {
        h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[0, 0]);
    } else {
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(frame), &[target, 0]);
        h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[frame, 0]);
        let params: u32 = h.read(at(0x64));
        let name = h.rebase(UV_SCALE_BIAS);
        let p = h.call_sym(FIND_PARAMETER, Conv::Thiscall(params), &[name]);
        if p != 0 {
            let values = frame.wrapping_add(0x10);
            let one = 1.0f32.to_bits();
            h.write(values, one);
            h.write(values.wrapping_add(4), one);
            h.write(values.wrapping_add(8), 0u32);
            h.write(values.wrapping_add(0xc), 0u32);
            let shader: u32 = h.read(at(0x60));
            let buffer: u32 = h.read(shader.wrapping_add(4));
            h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[p, values]);
        }
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(frame), &[]);
    }
    let r = h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[0]);
    if r != 0 {
        return r;
    }
    let render_pass: u32 = h.read(renderer.wrapping_add(0x13c));
    let pass_at = h.rebase(MOTION_PASS);
    h.write(renderer.wrapping_add(0x48), pass_at);
    let quad: u32 = h.read(at(0x54));
    let materials: u32 = h.read(quad.wrapping_add(0x10));
    h.call_sym(
        RENDER_MESH_INSTANCE,
        Conv::Thiscall(renderer),
        &[quad, materials, 0, render_pass],
    );
    h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1])
}

// ---- the save/load menu's open

/// The save/load menu (`0x00ceb248`).
pub const SAVE_LOAD_MENU: u32 = 0x00ce_b248;
/// Its `.swf` path buffer (`0x00ceb048`, 0x200 bytes).
pub const SAVE_LOAD_PATH: u32 = 0x00ce_b048;
/// `TOGetFFXLangJmp()` (cdecl).
pub const GET_LANG: u32 = 0x008a_c2f0;
/// `fiosUnifyFilename(path, out, n)` (cdecl).
pub const UNIFY_FILENAME: u32 = 0x0067_9820;
/// The `.swf` paths: Chinese (language 10), Korean (9), the rest
/// (`saveload_ch.swf`, `saveload_kr.swf`, `saveload_jp.swf`).
pub const SAVE_LOAD_SWF: [u32; 3] = [0x00b4_ca78, 0x00b4_cc78, 0x00b4_c878];
/// `IggyMenu_ctor_00a288f0(menu; path, x, y, w, h, flags)` (thiscall, ret
/// 0x18).
pub const IGGY_MENU_CTOR: u32 = 0x00a2_88f0;
/// `IggyMenu_setEnable(menu; on)` (thiscall, ret 4).
pub const SET_ENABLE: u32 = 0x00a2_8ef0;
/// `iggyDataValueSet_00a28db0(value, b)` (cdecl): a boolean value.
pub const VALUE_BOOL: u32 = 0x00a2_8db0;
/// `iggyDataValueSet_00a28d70(value, text)` (cdecl): a string value.
pub const VALUE_STRING: u32 = 0x00a2_8d70;
/// `IggyMenu_callIggyAS3External(menu; 0, name, n, values, 1)` (thiscall,
/// ret 0x14).
pub const CALL_AS3: u32 = 0x00a2_8ba0;
/// `"setMouseVisible"` (`0x00b46f20`).
pub const SET_MOUSE_VISIBLE: u32 = 0x00b4_6f20;
/// `"showSaveLoad"` (`0x00b4ce78`).
pub const SHOW_SAVE_LOAD: u32 = 0x00b4_ce78;
/// `"SetLanguage"` (`0x00b46e64`).
pub const SET_LANGUAGE: u32 = 0x00b4_6e64;
/// The language names: Chinese, Korean, the rest (`0x00b46e88`,
/// `0x00b46e84`, `0x00b46e60`).
pub const LANGUAGE_NAMES: [u32; 3] = [0x00b4_6e88, 0x00b4_6e84, 0x00b4_6e60];
/// Bytes of [`save_load_open`]'s frame, from `ebp - 0x30`: the boolean
/// value (`+0`, 0x10 bytes) and the language value (`+0x10`).
pub const SAVE_LOAD_FRAME: u32 = 0x20;

/// Language 10, 9 or another: index 0, 1 or 2.
fn by_language(lang: u32) -> usize {
    match lang {
        10 => 0,
        9 => 1,
        _ => 2,
    }
}

/// `SaveLoadMenu_open_00702a80()` (cdecl; `frame` see
/// [`SAVE_LOAD_FRAME`]; the SEH frame is not kept): once (the menu at
/// `0x00ceb248` is 0): the save/load `.swf` of the language unified into
/// its path buffer, the menu made (an `IggyMenu` over the whole screen;
/// 0 when `operator new` fails) and enabled, then the movie's
/// `setMouseVisible(true)`, `showSaveLoad()` and `SetLanguage(name)`.
pub fn save_load_open(h: &mut impl Host, frame: u32) {
    let menu_at = h.rebase(SAVE_LOAD_MENU);
    if h.read::<u32>(menu_at) != 0 {
        return;
    }
    let lang = h.call_sym(GET_LANG, Conv::Cdecl, &[]);
    let path = h.rebase(SAVE_LOAD_PATH);
    let swf = h.rebase(SAVE_LOAD_SWF[by_language(lang)]);
    h.call_sym(UNIFY_FILENAME, Conv::Cdecl, &[swf, path, 0x200]);
    let block = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x9c]);
    let menu = if block == 0 {
        0
    } else {
        let one = 1.0f32.to_bits();
        h.call_sym(
            IGGY_MENU_CTOR,
            Conv::Thiscall(block),
            &[path, 0, 0, one, one, 0],
        )
    };
    h.write(menu_at, menu);
    h.call_sym(SET_ENABLE, Conv::Thiscall(menu), &[1]);
    let flag = frame;
    let language = frame.wrapping_add(0x10);
    h.call_sym(VALUE_BOOL, Conv::Cdecl, &[flag, 0]);
    let menu: u32 = h.read(menu_at);
    let name = h.rebase(SET_MOUSE_VISIBLE);
    h.call_sym(CALL_AS3, Conv::Thiscall(menu), &[0, name, 1, flag, 1]);
    let menu: u32 = h.read(menu_at);
    let name = h.rebase(SHOW_SAVE_LOAD);
    h.call_sym(CALL_AS3, Conv::Thiscall(menu), &[0, name, 0, 0, 1]);
    let text = h.rebase(LANGUAGE_NAMES[by_language(lang)]);
    h.call_sym(VALUE_STRING, Conv::Cdecl, &[language, text]);
    let menu: u32 = h.read(menu_at);
    let name = h.rebase(SET_LANGUAGE);
    h.call_sym(CALL_AS3, Conv::Thiscall(menu), &[0, name, 1, language, 1]);
}

// ---- the boot text's update

/// The cycle threshold (`0x00b4a648`, f32 0.33).
pub const CYCLE: u32 = 0x00b4_a648;
/// The fade limit (`0x00b3eb44`, f32 0.3).
pub const FADE_LIMIT: u32 = 0x00b3_eb44;
/// `sprintf_s`'s import slot (cdecl).
pub const SPRINTF_S_IAT: u32 = 0x00b0_c394;
/// `"%d,%03d,%03d"` (`0x00b4a3c0`).
pub const VERSION_FORMAT: u32 = 0x00b4_a3c0;
/// `TextUtil_setTitleText(text; title)` (thiscall, ret 4).
pub const SET_TITLE_TEXT: u32 = 0x006f_6930;

/// `BootText_update_006de010(this; dt)` (thiscall, ret 4, from
/// `PhyreScene_animate`): while on (`+5`): when the timer `+0x160` passes
/// the image's 0.33 (ordered) it restarts at 0, and with the title on
/// (`+4`) the current slot (`+0x17c`: 0, 1 or 4 of the five words at
/// `+0x168`) is cleared, the five words printed as `"%d,%03d,%03d"`
/// (bits 20.., 10..19, 0..9) into 16-byte strings at `+0x86`, those
/// formatted with the format at `+0xd6` into `+6` and set as the title;
/// the slot then advances mod 5. The fade `+0x164` grows by `dt` while at
/// most the image's 0.3; the timer always does.
pub fn boot_text_update(h: &mut impl Host, this: u32, dt: u32) {
    let at = |off: u32| this.wrapping_add(off);
    if h.read::<u8>(at(5)) == 0 {
        return;
    }
    let cycle = load(h.read(h.rebase(CYCLE)));
    if cycle < load(h.read(at(0x160))) {
        h.write(at(0x160), 0u32);
        if h.read::<u8>(at(4)) != 0 {
            match h.read::<u32>(at(0x17c)) {
                0 => h.write(at(0x168), 0u32),
                1 => h.write(at(0x16c), 0u32),
                4 => h.write(at(0x178), 0u32),
                _ => {}
            }
            h.call_sym(MEMSET, Conv::Cdecl, &[at(6), 0, 0x80]);
            h.call_sym(MEMSET, Conv::Cdecl, &[at(0x86), 0, 0x50]);
            let sprintf_s: u32 = h.read(h.rebase(SPRINTF_S_IAT));
            let format = h.rebase(VERSION_FORMAT);
            for k in 0..5u32 {
                let v: u32 = h.read(at(0x168 + 4 * k));
                h.call(
                    sprintf_s,
                    Conv::Cdecl,
                    &[
                        at(0x86 + 0x10 * k),
                        0x10,
                        format,
                        v >> 20,
                        (v >> 10) & 0x3ff,
                        v & 0x3ff,
                    ],
                );
            }
            let sprintf_s: u32 = h.read(h.rebase(SPRINTF_S_IAT));
            h.call(
                sprintf_s,
                Conv::Cdecl,
                &[
                    at(6),
                    0x80,
                    at(0xd6),
                    at(0x86),
                    at(0x96),
                    at(0xa6),
                    at(0xb6),
                    at(0xc6),
                ],
            );
            let text: u32 = h.read(this);
            h.call_sym(SET_TITLE_TEXT, Conv::Thiscall(text), &[at(6)]);
        }
        let slot: i32 = h.read(at(0x17c));
        h.write(at(0x17c), slot.wrapping_add(1).wrapping_rem(5));
    }
    let limit = load(h.read(h.rebase(FADE_LIMIT)));
    let dt = load(dt);
    let fade = load(h.read(at(0x164)));
    // `fcomp` + `test ah, 1; jne`: skipped when 0.3 < fade or unordered
    if fade <= limit {
        h.write(at(0x164), store(fade + dt));
    }
    let timer = load(h.read(at(0x160)));
    h.write(at(0x160), store(dt + timer));
}

// ---- GDraw's resource limits and setup

/// `g_GDraw` (`0x01a858b4`): `GDraw`'s D3D11 state.
pub const GDRAW: u32 = 0x01a8_58b4;
/// The resource limits (`0x00c6d448`: per kind the handle count and the
/// byte count).
pub const LIMITS: u32 = 0x00c6_d448;
/// `FUN_00a36300(stats)` (cdecl): frees the vertex buffers.
pub const FREE_VERTEX_BUFFERS: u32 = 0x00a3_6300;
/// `op_node_pool_init_00a39bc0(set, n, bytes)` (cdecl).
pub const NODE_POOL_INIT: u32 = 0x00a3_9bc0;
/// `GDraw_pool_freeAll_00a3b380(pool, stats)` (cdecl).
pub const POOL_FREE_ALL: u32 = 0x00a3_b380;
/// `IggyGDrawFree`'s import slot (stdcall, 1 word).
pub const GDRAW_FREE_IAT: u32 = 0x00b0_c870;
/// `gdraw_newPool_00a3bcd0(kind)` (cdecl).
pub const NEW_POOL: u32 = 0x00a3_bcd0;
/// `gdraw_clearBindings_00a3d280()` (cdecl).
pub const CLEAR_BINDINGS: u32 = 0x00a3_d280;
/// `ComRelease_Slot_00a34910(slot)` (cdecl).
pub const RELEASE_SLOT: u32 = 0x00a3_4910;
/// `State_InitBuffers_00a34b10(n)` (cdecl): `al`.
pub const INIT_BUFFERS: u32 = 0x00a3_4b10;
/// `gdraw_D3D11_CreateContext_00a35780(device, context, w, h)` (cdecl).
pub const CREATE_CONTEXT: u32 = 0x00a3_5780;
/// `gdraw_D3D11_SetResourceLimits_00a38180(kind, handles, bytes)` (cdecl).
pub const SET_RESOURCE_LIMITS: u32 = 0x00a3_8180;
/// `IggySetGDraw`'s import slot (stdcall, 1 word).
pub const SET_GDRAW_IAT: u32 = 0x00b0_c868;
/// Bytes of the stats block both functions keep at their frame's bottom.
pub const STATS_FRAME: u32 = 0x30;

/// A pool kind (1 textures, 2 render targets) rebuilt: the old pool
/// freed into `stats` and released, a new one made. Whether it was made.
fn renew_pool(h: &mut impl Host, stats: u32, kind: u32) -> bool {
    let off = if kind == 1 { 0xab0 } else { 0xab4 };
    let gdraw: u32 = h.read(h.rebase(GDRAW));
    let pool: u32 = h.read(gdraw.wrapping_add(off));
    if pool != 0 {
        h.call_sym(POOL_FREE_ALL, Conv::Cdecl, &[pool, stats]);
        let gdraw: u32 = h.read(h.rebase(GDRAW));
        let pool: u32 = h.read(gdraw.wrapping_add(off));
        let free: u32 = h.read(h.rebase(GDRAW_FREE_IAT));
        h.call(free, Conv::Stdcall, &[pool]);
    }
    let pool = h.call_sym(NEW_POOL, Conv::Cdecl, &[kind]);
    let gdraw: u32 = h.read(h.rebase(GDRAW));
    h.write(gdraw.wrapping_add(off), pool);
    pool != 0
}

/// The dynamic buffers rebuilt for `bytes`: `al` of `State_InitBuffers`.
fn renew_buffers(h: &mut impl Host, bytes: u32) -> u8 {
    h.call_sym(CLEAR_BINDINGS, Conv::Cdecl, &[]);
    for off in [0x998u32, 0x9a8] {
        let gdraw: u32 = h.read(h.rebase(GDRAW));
        h.call_sym(RELEASE_SLOT, Conv::Cdecl, &[gdraw.wrapping_add(off)]);
    }
    h.call_sym(INIT_BUFFERS, Conv::Cdecl, &[bytes]) as u8
}

/// A zeroed stats block (`u16` 0, then `memset` of 0x2e bytes).
fn clear_stats(h: &mut impl Host, stats: u32) {
    h.write(stats, 0u16);
    h.call_sym(MEMSET, Conv::Cdecl, &[stats.wrapping_add(2), 0, 0x2e]);
}

/// `gdraw_D3D11_SetResourceLimits_00a38180(kind, handles, bytes)` (cdecl;
/// `stats` its 0x30-byte frame at `ebp - 0x30`): kind 0 uses 9 handles.
/// Unchanged limits return 1; else they are stored, and with `GDraw` up the
/// kind is rebuilt: 0 the vertex buffers (freed, the node pool at
/// `+0x290` set up again; 1), 1 the texture pool and 2 the render-target
/// pool (whether made), 3 the dynamic buffers (`State_InitBuffers`'s
/// `al`); other kinds 0. Without `GDraw`, 1.
pub fn set_resource_limits(
    h: &mut impl Host,
    stats: u32,
    kind: u32,
    handles: u32,
    bytes: u32,
) -> u32 {
    clear_stats(h, stats);
    let handles = if kind == 0 { 9 } else { handles };
    let entry = h.rebase(LIMITS).wrapping_add(kind.wrapping_mul(8));
    if h.read::<u32>(entry) == handles && h.read::<u32>(entry.wrapping_add(4)) == bytes {
        return 1;
    }
    let gdraw: u32 = h.read(h.rebase(GDRAW));
    h.write(entry, handles);
    h.write(entry.wrapping_add(4), bytes);
    if gdraw == 0 {
        return 1;
    }
    match kind {
        0 => {
            h.call_sym(FREE_VERTEX_BUFFERS, Conv::Cdecl, &[stats]);
            let gdraw: u32 = h.read(h.rebase(GDRAW));
            h.call_sym(
                NODE_POOL_INIT,
                Conv::Cdecl,
                &[gdraw.wrapping_add(0x290), handles, bytes],
            );
            1
        }
        1 | 2 => u32::from(renew_pool(h, stats, kind)),
        3 => u32::from(renew_buffers(h, bytes)),
        _ => 0,
    }
}

/// `IggyGDraw_setup_00a3baf0(this; device, limits)` (thiscall, ret 8;
/// `stats` its 0x30-byte frame at `ebp - 0x34`): 0x15 without the
/// device's `+0x6c`/`+0x70`. Unless `device + 0x1f0` is 0, `GDraw`'s
/// context is created (`+0x6c`, `+0x70`, the size `+0x200`/`+0x204`)
/// into `[this]` (9 when it fails). Then the limits (eight words): the
/// render targets' (`[4]`, `[5]`) rebuild that pool when changed, the
/// textures' (`[7]`, `[6]`) and vertex buffers' (`[1]`, `[0]`) go
/// through `gdraw_D3D11_SetResourceLimits`, the dynamic buffers' (`[3]`,
/// `[2]`) rebuild them when changed; any failure returns 9. Then
/// `IggySetGDraw([this])` and 0.
pub fn gdraw_setup(h: &mut impl Host, stats: u32, this: u32, device: u32, limits: u32) -> u32 {
    let context: u32 = h.read(device.wrapping_add(0x70));
    let dev: u32 = h.read(device.wrapping_add(0x6c));
    if dev == 0 || context == 0 {
        return 0x15;
    }
    let size = device.wrapping_add(0x1f0);
    if size != 0 {
        let w: u32 = h.read(size.wrapping_add(0x10));
        let hh: u32 = h.read(size.wrapping_add(0x14));
        let c = h.call_sym(CREATE_CONTEXT, Conv::Cdecl, &[dev, context, w, hh]);
        h.write(this, c);
        if c == 0 {
            return 9;
        }
    }
    let word = |k: u32| limits.wrapping_add(4 * k);
    let bytes: u32 = h.read(word(4));
    let handles: u32 = h.read(word(5));
    clear_stats(h, stats);
    let targets = h.rebase(LIMITS).wrapping_add(0x10);
    if h.read::<u32>(targets) != handles || h.read::<u32>(targets.wrapping_add(4)) != bytes {
        let gdraw: u32 = h.read(h.rebase(GDRAW));
        h.write(targets, handles);
        h.write(targets.wrapping_add(4), bytes);
        if gdraw != 0 && !renew_pool(h, stats, 2) {
            return 9;
        }
    }
    let (n, b) = (h.read::<u32>(word(7)), h.read::<u32>(word(6)));
    if h.call_sym(SET_RESOURCE_LIMITS, Conv::Cdecl, &[1, n, b]) == 0 {
        return 9;
    }
    let (n, b) = (h.read::<u32>(word(1)), h.read::<u32>(word(0)));
    if h.call_sym(SET_RESOURCE_LIMITS, Conv::Cdecl, &[0, n, b]) == 0 {
        return 9;
    }
    let handles: u32 = h.read(word(3));
    let bytes: u32 = h.read(word(2));
    let dynamic = h.rebase(LIMITS).wrapping_add(0x18);
    if h.read::<u32>(dynamic) != handles || h.read::<u32>(dynamic.wrapping_add(4)) != bytes {
        let gdraw: u32 = h.read(h.rebase(GDRAW));
        h.write(dynamic, handles);
        h.write(dynamic.wrapping_add(4), bytes);
        if gdraw != 0 && renew_buffers(h, bytes) == 0 {
            return 9;
        }
    }
    let context: u32 = h.read(this);
    let set: u32 = h.read(h.rebase(SET_GDRAW_IAT));
    h.call(set, Conv::Stdcall, &[context]);
    0
}
