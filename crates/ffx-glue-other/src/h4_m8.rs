//! Continuation round 11 of list `h4-glue-other`, batch 3: the models'
//! VFX render data buffers.

use ffx_bridge::Global;
use ffx_graphic::host::{Conv, Host};

/// `memset` through its thunk (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `graphicLevelGetNameHash()` (cdecl).
pub const LEVEL_HASH: u32 = 0x0064_1da0;
/// `MagicFile_GetMagicId()` (cdecl).
pub const MAGIC_ID: u32 = 0x009d_a3b0;
/// `get_ptr32_func(p)` (cdecl).
pub const PTR32: u32 = 0x00a4_4650;
/// `sprintf` and `strstr` through their import slots (cdecl).
pub const SPRINTF: Global<u32> = Global::new(0x00b0_c3dc);
/// See [`SPRINTF`].
pub const STRSTR: Global<u32> = Global::new(0x00b0_c378);
/// `graphicVFXIsTextureWithBpLoaded(number, depth, a, b, w, h, name)`
/// (cdecl) -> nonzero when loaded.
pub const IS_TEXTURE_BP_LOADED: u32 = 0x0064_5450;
/// `graphicVFXIsTextureLoaded(name, out, a, b, c, d)` (cdecl).
pub const IS_TEXTURE_LOADED: u32 = 0x0064_5400;
/// `Ch_GetChrRenderInsatanceFromOEF2Content(record)` (cdecl) -> the
/// monster's render instance, 0, or -1.
pub const CHR_INSTANCE: u32 = 0x0082_6af0;
/// `graphicGetMonsterEtTextureName(instance, number)` (cdecl).
pub const MONSTER_TEXTURE_NAME: u32 = 0x0064_1330;
/// `MsGetBattleScene()` (cdecl).
pub const BATTLE_SCENE: u32 = 0x0078_2970;
/// `dbgPrintf(format, ...)` (cdecl).
pub const DBG_PRINTF: u32 = 0x0062_f500;
/// `graphicVFXCreate(lo, hi, flags)` (cdecl).
pub const VFX_CREATE: u32 = 0x0064_4de0;
/// `useMultipleInstances_model(model, data)` (cdecl) -> 0, 1 or 2.
pub const USE_MULTIPLE: u32 = 0x0071_e610;
/// `graphicVFXCreateDataBuffer(lo, hi, n, vertices, indices, names,
/// loaded)` (cdecl).
pub const CREATE_DATA_BUFFER: u32 = 0x0064_5040;
/// `graphicVFXGetRenderDataByInstance(lo, hi)` (cdecl).
pub const RENDER_DATA: u32 = 0x0064_53d0;
/// `ModelBuffer_AllocateBuffer(data)` (cdecl).
pub const ALLOCATE_BUFFER: u32 = 0x0072_ac20;
/// A flag word (`0x0113335c`) that enables the magic special cases.
pub const MAGIC_CASES: u32 = 0x0113_335c;
/// A word (`0x00c3a504`) checked against -1 on map 0x188.
pub const MAP_188_WORD: u32 = 0x00c3_a504;
/// `g_CurrentMapNo` (`0x012fbc40`).
pub const CURRENT_MAP: u32 = 0x012f_bc40;
/// Strings: `"NoTexture"` (`0x00b41b50`), `"FrameBuffer"`
/// (`0x00b45b3c`), the missing texture message (`0x00b4ff04`), magic
/// 0x18d's texture name (`0x00b4ff38`) and its message (`0x00b4ff58`).
pub const NO_TEXTURE: u32 = 0x00b4_1b50;
/// See [`NO_TEXTURE`].
pub const FRAME_BUFFER: u32 = 0x00b4_5b3c;
/// See [`NO_TEXTURE`].
pub const MISSING_TEXTURE: u32 = 0x00b4_ff04;
/// See [`NO_TEXTURE`].
pub const MAGIC_18D_TEXTURE: u32 = 0x00b4_ff38;
/// See [`NO_TEXTURE`].
pub const MAGIC_18D_MESSAGE: u32 = 0x00b4_ff58;
/// Bytes of [`create_render_data_buffer`]'s frame, from the original's
/// `ebp - 0x23dc` up to its cookie: the second instance set's vertex
/// counts (`+0x38`, four groups of 30 words, 0x78 apart) and index
/// counts (`+0x218`), magic 0x18d's texture name (`+0x2f8`, 0x100), the
/// segments' kinds (`+0x3f8`), vertex counts (`+0x470`), loaded flags
/// (`+0x560`), index counts (`+0x4e8`) and texture names (`+0x5d8`, 30 of
/// 0x100 bytes).
pub const BUFFER_FRAME: u32 = 0x23d8;

const KINDS: u32 = 0x3f8;
const VERTICES: u32 = 0x470;
const INDICES: u32 = 0x4e8;
const LOADED: u32 = 0x560;
const NAMES: u32 = 0x5d8;
const NAME_18D: u32 = 0x2f8;
const GROUP_VERTICES: u32 = 0x38;
const GROUP_INDICES: u32 = 0x218;

/// `strcpy(dst, src)` as the original's inline byte loop.
fn copy_text(h: &mut impl Host, dst: u32, src: u32) {
    let mut k = 0u32;
    loop {
        let b: u8 = h.read(src.wrapping_add(k));
        h.write(dst.wrapping_add(k), b);
        if b == 0 {
            return;
        }
        k = k.wrapping_add(1);
    }
}

/// A texture descriptor (the record's 64-bit `+8`): number (bits 0..13),
/// depth (20..25), log2 width (26..29), log2 height (30..33).
#[derive(Clone, Copy)]
struct Texture {
    number: u32,
    depth: u32,
    width: u32,
    height: u32,
}

fn texture(h: &impl Host, record: u32) -> Texture {
    let d = u64::from(h.read::<u32>(record + 8)) | u64::from(h.read::<u32>(record + 0xc)) << 32;
    Texture {
        number: (d & 0x3fff) as u32,
        depth: ((d >> 20) & 0x3f) as u32,
        width: 1 << ((d >> 26) & 0xf),
        height: 1 << ((d >> 30) & 0xf),
    }
}

/// What a texture check leaves: go on, or the function's 0.
enum Next {
    Go,
    Fail,
}

/// The texture of segment `k`: `"FrameBuffer"` for numbers 0xd00 and
/// 0x1a00, else the loaded check (failing: the message and 0). The
/// segment is marked loaded either way.
fn check_texture(h: &mut impl Host, frame: u32, k: u32, t: Texture) -> Next {
    let name = frame + NAMES + 0x100 * k;
    if t.number << 6 == 0x68000 || t.number << 6 == 0x34000 {
        let src = h.rebase(FRAME_BUFFER);
        for w in 0..3 {
            let v: u32 = h.read(src + 4 * w);
            h.write(name + 4 * w, v);
        }
        h.write(frame + LOADED + 4 * k, 1u32);
        return Next::Go;
    }
    h.write(frame + LOADED + 4 * k, 1u32);
    let r = h.call_sym(
        IS_TEXTURE_BP_LOADED,
        Conv::Cdecl,
        &[t.number, t.depth, 0, 0, t.width, t.height, name],
    );
    if r == 0 {
        let m = h.rebase(MISSING_TEXTURE);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[m, t.number]);
        return Next::Fail;
    }
    Next::Go
}

/// `createVFXRenderDataBuffer(lo, hi, effect, data, flags)`
/// (`0x00713790`, cdecl; `frame` see [`BUFFER_FRAME`]) -> 1, or 0 when a
/// texture is missing, a monster is missing (-1), the model has 30
/// segments or more, a segment kind is unknown or a magic excludes it.
/// Each segment of the model (`get_ptr32_func([effect + 0x1c])`, `+0x10`
/// primitive count, records from `+[+4]`) gets its counts (by kind 0..7:
/// 3n or 4n vertices and 3n or 6n indices), a texture name and its
/// loaded check; then the instance's flags (8 when every name holds
/// `"NoTexture"`, magic and map cases), `graphicVFXCreate`, the data
/// buffers and the render data (one set; or up to four groups of 6000
/// indices for `useMultipleInstances_model` 2, ids 10000 apart).
#[allow(clippy::too_many_lines)] // the original's eight kinds, then the instances
pub fn create_render_data_buffer(
    h: &mut impl Host,
    frame: u32,
    lo: u32,
    hi: u32,
    effect: u32,
    data: u32,
    flags: u32,
) -> u32 {
    let mut flags = flags;
    for base in [VERTICES, INDICES, LOADED] {
        h.write(frame + base, 0u32);
        h.call_sym(MEMSET, Conv::Cdecl, &[frame + base + 4, 0, 0x74]);
    }
    let mut number = 0u32;
    let level = h.call_sym(LEVEL_HASH, Conv::Cdecl, &[]);
    let magic = h.call_sym(MAGIC_ID, Conv::Cdecl, &[]);
    let model = {
        let p: u32 = h.read(effect + 0x1c);
        h.call_sym(PTR32, Conv::Cdecl, &[p])
    };
    let mut remaining = i32::from(h.read::<i16>(model + 0x10));
    let mut record = model.wrapping_add(h.read(model + 4));
    let cases: u32 = h.read(h.rebase(MAGIC_CASES));
    let mut depth_b7 = 0u32;
    if level == 0x25ca && cases == 0 && h.read::<u32>(record + 8) & 0x3fff == 0x3600 {
        depth_b7 = 0xa;
    }
    let no_texture = h.rebase(NO_TEXTURE);
    let sprintf = SPRINTF.get(h);
    let mut count = 0u32;
    while remaining > 0 {
        count += 1;
        if count >= 0x1e {
            return 0;
        }
        let k = count - 1;
        let kind = i32::from(h.read::<i8>(record + 1));
        let counted = record + 2;
        let n = i32::from(h.read::<i16>(counted));
        let mut next = record + 0x10;
        let name = frame + NAMES + 0x100 * k;
        let cases: u32 = h.read(h.rebase(MAGIC_CASES));
        // the kind's strides (bytes per primitive), vertex and index factors
        let (stride, vf, ifac, textured) = match kind {
            0 => (20, 3, 3, false),
            1 => (28, 3, 3, false),
            2 => (32, 3, 3, true),
            3 => (40, 3, 3, true),
            4 => (24, 4, 6, false),
            5 => (32, 4, 6, false),
            6 => (40, 4, 6, true),
            7 => (48, 4, 6, true),
            _ => return 0,
        };
        if matches!(kind, 0 | 4) {
            if cases != 0 && matches!(magic, 0x12a | 0x111 | 0xc9 | 0xcb) {
                return 0;
            }
            if matches!(magic, 0x203 | 0x150 | 0x29b) && h.read::<u8>(data + 8) == 0x44 {
                flags |= 0x400;
            }
        }
        if textured {
            let one = if h.read::<u8>(model) & 4 != 0 {
                1u32
            } else {
                2
            };
            h.write(frame + KINDS + 4 * k, one);
        }
        let (vertices, indices) = if n > 0 {
            next = next.wrapping_add((stride * n).cast_unsigned());
            ((vf * n).cast_unsigned(), (ifac * n).cast_unsigned())
        } else {
            (0, 0)
        };
        h.write(frame + VERTICES + 4 * k, vertices);
        h.write(frame + INDICES + 4 * k, indices);
        if textured {
            let mut t = texture(h, record);
            number = t.number;
            let remapped =
                matches!(kind, 2 | 6) && magic == 0x122 && matches!(t.number, 0x3680 | 0x3420);
            if remapped {
                t.number = 0x3a00;
                number = 0x3a00;
            } else if kind == 6 {
                if matches!(magic, 0x66 | 0x67) {
                    if t.number == 0x3400 {
                        flags |= 0x400;
                    }
                } else if magic == 0x1ba && t.number == 0x3400 {
                    flags |= 0x400;
                }
            }
            let monster = matches!(kind, 2 | 6) && !remapped && t.number >= 0x3f00;
            let mut done = false;
            if monster {
                let instance = h.call_sym(CHR_INSTANCE, Conv::Cdecl, &[record]);
                if instance == u32::MAX {
                    return 0;
                }
                if instance != 0 {
                    let src = h.call_sym(MONSTER_TEXTURE_NAME, Conv::Cdecl, &[instance, t.number]);
                    if src != 0 {
                        if kind == 2 {
                            let scene = h.call_sym(BATTLE_SCENE, Conv::Cdecl, &[]);
                            if scene == 0x0259_0069 && h.read::<u8>(data + 8) == 0x42 {
                                return 0;
                            }
                        }
                        copy_text(h, name, src);
                        if kind == 6 {
                            h.write(frame + LOADED + 4 * k, 1u32);
                        }
                        done = true;
                    }
                }
            }
            if !done && matches!(check_texture(h, frame, k, t), Next::Fail) {
                return 0;
            }
        } else {
            h.call(sprintf, Conv::Cdecl, &[name, no_texture]);
            if kind == 0 && level == 0x25b0 && h.read::<u16>(model + 0x10) == 0x96 {
                h.write(frame + LOADED + 4 * k, 1u32);
                let r = h.call_sym(
                    IS_TEXTURE_BP_LOADED,
                    Conv::Cdecl,
                    &[0x271a, 0x13, 0, 0, 0x80, 0x80, name],
                );
                if r == 0 {
                    let m = h.rebase(MISSING_TEXTURE);
                    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[m, number]);
                    return 0;
                }
            }
        }
        record = next;
        remaining -= i32::from(h.read::<i16>(counted));
    }
    // flags
    let strstr = STRSTR.get(h);
    for k in 0..count {
        let name = frame + NAMES + 0x100 * k;
        if h.call(strstr, Conv::Cdecl, &[name, no_texture]) == 0 {
            flags |= 8;
            break;
        }
    }
    if h.read::<u8>(data) & 0x40 != 0 && h.call_sym(MAGIC_ID, Conv::Cdecl, &[]) == 0x207 {
        flags |= 0x2_0000;
    }
    let map: u32 = h.read(h.rebase(CURRENT_MAP));
    let word: u32 = h.read(h.rebase(MAP_188_WORD));
    if word != u32::MAX && map == 0x188 {
        flags |= 0x8_0000;
    } else if map == 0x46 && (number == 0x3800 || number == 0) {
        flags &= !1;
    }
    if h.read::<u32>(h.rebase(MAGIC_CASES)) != 0 {
        if magic == 0x18d && flags & 8 != 0 {
            for k in 0..count {
                let loaded = frame + LOADED + 4 * k;
                if h.read::<u32>(loaded).cast_signed() > 0 {
                    continue;
                }
                h.write(loaded, 1u32);
                let src = h.rebase(MAGIC_18D_TEXTURE);
                let r = h.call_sym(
                    IS_TEXTURE_LOADED,
                    Conv::Cdecl,
                    &[src, frame + NAME_18D, 0, 0, 0, 0],
                );
                let name = frame + NAMES + 0x100 * k;
                if r == 0 {
                    let m = h.rebase(MAGIC_18D_MESSAGE);
                    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[m, name]);
                    return 0;
                }
                copy_text(h, name, frame + NAME_18D);
                h.write(frame + KINDS + 4 * k, 1u32);
            }
        }
        if h.read::<u32>(h.rebase(MAGIC_CASES)) != 0
            && magic == 0x207
            && h.read::<u8>(data) & 0x40 != 0
        {
            for k in 0..count {
                let loaded = frame + LOADED + 4 * k;
                if h.read::<u32>(loaded).cast_signed() > 0 {
                    continue;
                }
                let name = frame + NAMES + 0x100 * k;
                let r = h.call_sym(
                    IS_TEXTURE_BP_LOADED,
                    Conv::Cdecl,
                    &[0x7530, 0x13, 0, 0, 0x80, 0x80, name],
                );
                if r == 0 {
                    let m = h.rebase(MISSING_TEXTURE);
                    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[m, 0x7530]);
                    let src = h.rebase(NO_TEXTURE);
                    for w in 0..2 {
                        let v: u32 = h.read(src + 4 * w);
                        h.write(name + 4 * w, v);
                    }
                    let v: u16 = h.read(src + 8);
                    h.write(name + 8, v);
                } else {
                    h.write(loaded, 1u32);
                    h.write(frame + KINDS + 4 * k, 1u32);
                }
            }
        }
    }
    if flags & 0x1000_0000 != 0 {
        flags = 0x1000_0000;
    }
    if flags.cast_signed() < 0 {
        flags = 0x8000_0000;
    }
    if matches!(magic, 0xbc | 0x15f) && number.wrapping_sub(0x3ac0) <= 0x80 {
        flags |= 0x40;
    }
    // the instance and its render data
    h.call_sym(VFX_CREATE, Conv::Cdecl, &[lo, hi, flags]);
    let multiple = h.call_sym(USE_MULTIPLE, Conv::Cdecl, &[data, model]);
    let names = frame + NAMES;
    let loaded = frame + LOADED;
    if multiple != 2 {
        h.call_sym(
            CREATE_DATA_BUFFER,
            Conv::Cdecl,
            &[
                lo,
                hi,
                count,
                frame + VERTICES,
                frame + INDICES,
                names,
                loaded,
            ],
        );
    }
    let rd = h.call_sym(RENDER_DATA, Conv::Cdecl, &[lo, hi]);
    if rd == 0 {
        return 1;
    }
    if depth_b7 != 0 {
        h.write(rd + 0xb7, depth_b7 as u8);
    }
    if multiple == 0 {
        set_buffer_id(h, rd, data, model);
        if flags & 0x2000 != 0 {
            h.write(rd + 0x84, 1u32);
        }
        set_kinds(h, frame, rd, count);
        return 1;
    }
    h.write(rd + 0xc0, multiple as u8);
    h.write(rd + 0xc4, if multiple == 1 { 4u16 } else { 0 });
    if multiple == 2 {
        // up to four groups of 6000 indices per segment
        let mut most = 0i32;
        for k in 0..count {
            let n = h.read::<u32>(frame + INDICES + 4 * k).cast_signed();
            let groups = n.wrapping_sub(1) / 6000 + 1;
            let rest = n.wrapping_sub(groups.wrapping_mul(6000)).wrapping_add(6000);
            if most < groups {
                most = groups;
            }
            let last = groups - 1;
            for g in 0..4i32 {
                let v = match last.cmp(&g) {
                    std::cmp::Ordering::Greater => 6000,
                    std::cmp::Ordering::Equal => rest,
                    std::cmp::Ordering::Less => 3,
                };
                let at = 0x78 * g.cast_unsigned() + 4 * k;
                h.write(frame + GROUP_INDICES + at, v.cast_unsigned());
                h.write(frame + GROUP_VERTICES + at, v.cast_unsigned());
            }
        }
        h.write(rd + 0xc4, most as u16);
    }
    let groups = u32::from(h.read::<u16>(rd + 0xc4));
    for j in 0..groups {
        let id = (u64::from(hi) << 32 | u64::from(lo)).wrapping_add(u64::from(j * 10000));
        let (lo2, hi2) = (id as u32, (id >> 32) as u32);
        if multiple == 2 {
            if j != 0 {
                h.call_sym(VFX_CREATE, Conv::Cdecl, &[lo2, hi2, flags]);
            }
            let at = 0x78 * j;
            h.call_sym(
                CREATE_DATA_BUFFER,
                Conv::Cdecl,
                &[
                    lo2,
                    hi2,
                    count,
                    frame + GROUP_VERTICES + at,
                    frame + GROUP_INDICES + at,
                    names,
                    loaded,
                ],
            );
        } else if j != 0 {
            h.call_sym(VFX_CREATE, Conv::Cdecl, &[lo2, hi2, flags]);
            h.call_sym(
                CREATE_DATA_BUFFER,
                Conv::Cdecl,
                &[
                    lo2,
                    hi2,
                    count,
                    frame + VERTICES,
                    frame + INDICES,
                    names,
                    loaded,
                ],
            );
        }
        let r = h.call_sym(RENDER_DATA, Conv::Cdecl, &[lo2, hi2]);
        if r != 0 {
            set_buffer_id(h, r, data, model);
            set_kinds(h, frame, r, count);
        }
    }
    1
}

/// The render data's model buffer (`+0xc2`): 0xffff with the data's
/// `+0x38` set, else `ModelBuffer_AllocateBuffer(model)`.
fn set_buffer_id(h: &mut impl Host, rd: u32, data: u32, model: u32) {
    let id = if h.read::<u32>(data + 0x38) != 0 {
        0xffff
    } else {
        h.call_sym(ALLOCATE_BUFFER, Conv::Cdecl, &[model]) as u16
    };
    h.write(rd + 0xc2, id);
}

/// Each segment's kind word into the render data's segment `+0x24`
/// (0x6c-byte records at `[rd + 0x94]`). Kinds 0, 1, 4 and 5 never write
/// theirs: junk read: [ebp-0x1fe4 + 4k] never written; 0 (brief
/// 2026-10-24; the shim's frame starts zeroed).
fn set_kinds(h: &mut impl Host, frame: u32, rd: u32, count: u32) {
    for k in 0..count {
        let v: u32 = h.read(frame + KINDS + 4 * k);
        let segments: u32 = h.read(rd + 0x94);
        h.write(segments.wrapping_add(0x6c * k + 0x24), v);
    }
}
