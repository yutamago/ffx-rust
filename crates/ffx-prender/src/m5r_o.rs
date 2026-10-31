//! M5 stage 1, list `render-pcui` (round 5), batch 2: the specular map
//! parameter setters and more scene methods.

use crate::m5r_c::quiet;
use ffx_bridge::Global;

pub const ARRAY_DATA: u32 = 0x004d_45a0;
pub const FIND_PARAM: u32 = 0x0056_cbf0;
pub const CLUSTER_MANAGER: u32 = 0x0069_a1a0;
pub const CLUSTER_BY_NAME: u32 = 0x0069_b4b0;
pub const TEXTURE_ITERATOR_CTOR: u32 = 0x0046_3bf0;
pub const ARRAY_COPY_FROM: u32 = 0x0063_37e0;
pub const LINEAR_SAMPLER: u32 = 0x0065_e050;
pub const SET_TEXTURE_CHECKED: u32 = 0x0056_e670;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `g_PhyreScene`.
pub const SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// The parameter types a sampler value is written into.
pub const SAMPLER_TYPES: [u8; 6] = [0x42, 0xd9, 0xda, 0xdb, 0xdc, 0x12];

/// `"SpecularMapSampler"`, `"SpecularUvOffset"`,
/// `"Specular_SpecialFloat1"`, `"Specular_SpecialMat1"`,
/// `"Specular_SpecialVector1"`, `"Specular_SpecialVector2"`,
/// `"SpecularMapSamplerState"`.
pub const SPEC_SAMPLER: u32 = 0x00b4_95e8;
pub const SPEC_UV_OFFSET: u32 = 0x00b4_9614;
pub const SPEC_FLOAT1: u32 = 0x00b4_9638;
pub const SPEC_MAT1: u32 = 0x00b4_9650;
pub const SPEC_VECTOR1: u32 = 0x00b4_9680;
pub const SPEC_VECTOR2: u32 = 0x00b4_9668;
pub const SPEC_SAMPLER_STATE: u32 = 0x00b4_95fc;
/// `"Shininess"`, `"SpecularPower"` and the power's `.rdata` 0.5f.
pub const SHININESS: u32 = 0x00b4_8114;
pub const SPEC_POWER: u32 = 0x00b4_9628;
pub const SPEC_POWER_VALUE: u32 = 0x00b9_2194;

pub const IS_DEBUG_MODE: u32 = 0x0082_2550;
pub const GET_DB_NUM: u32 = 0x0082_9d80;
pub const READ_VIEWER_ANIMATION: u32 = 0x0083_69a0;
pub const GET_PLAYER: u32 = 0x0082_d860;
pub const SET_MOTION_TYPE: u32 = 0x0082_b3a0;
pub const SET_VIEWER_MOTION: u32 = 0x0083_7e10;
pub const MOTION_SET_LOOP: u32 = 0x0083_9960;
/// `g_ViewerManager`.
pub const VIEWER_MANAGER: u32 = 0x00cc_cac0;
/// MSVCR110 import slots: `sprintf_s`, `atoi`, `strstr`.
pub const SPRINTF_S: u32 = 0x00b0_c394;
pub const ATOI: u32 = 0x00b0_c40c;
pub const STRSTR: u32 = 0x00b0_c378;
pub const MEMSET: u32 = 0x0094_964c;
pub const TEXT_CHANGE_LINE: u32 = 0x006f_6070;
/// `FUN_006b7140(this; line, buffer)`: a texture animation viewer line.
pub const TEXANIM_LINE: u32 = 0x006b_7140;
/// `"init failed"`, `">"`, `" "`, `"%s%s%s"`, `"%s%s%d"`, `"on"`, `"off"`,
/// `"grid00"`.
pub const TEXT_INIT_FAILED: u32 = 0x00b4_8244;
pub const MARK_SELECTED: u32 = 0x00b4_8250;
pub const MARK_OTHER: u32 = 0x00b2_0b30;
pub const FMT_SSS: u32 = 0x00b4_2a3c;
pub const FMT_SSD: u32 = 0x00b4_88e0;
pub const TEXT_ON: u32 = 0x00b4_88d8;
pub const TEXT_OFF: u32 = 0x00b4_88dc;
pub const GRID_NAME: u32 = 0x00b4_24c8;
/// The motion names the animation panel prints (by `+0xc`).
pub const MOTION_NAMES: u32 = 0x00c3_4318;

/// `FUN_00654f00(a, b)` (stdcall): a new list node (`0xc` bytes; a head
/// pointing at itself when `a` is 0).
pub const LIST_NEW_HEAD: u32 = 0x0065_4f00;
/// `FUN_00655460(vector; result, at, n, value)` (`ret 0x10`).
pub const VECTOR_FILL: u32 = 0x0065_5460;
/// `AtlConvAllocMemory(buffer, length, fixed, fixed length)` (cdecl).
pub const CONV_ALLOC: u32 = 0x0064_96d0;
/// KERNEL32 import slots: `MultiByteToWideChar`, `GetLastError`.
pub const MULTI_BYTE_TO_WIDE: u32 = 0x00b0_c1d0;
pub const GET_LAST_ERROR: u32 = 0x00b0_c030;
pub const ATL_THROW_LAST_ERROR: u32 = 0x0064_fa70;
pub const GRID_GET: u32 = 0x0067_4100;
pub const GRID_ASSIGN_8: u32 = 0x0048_1940;
pub const GRID_ASSIGN_10: u32 = 0x0049_6aa0;

pub const CURSOR_BEGIN: u32 = 0x0063_7730;
pub const CURSOR_NEXT: u32 = 0x0063_77a0;
/// `g_ClassDesc_00ca7228`: the class the field map's walk selects.
pub const MAP_OBJECT_CLASS: u32 = 0x00ca_7228;
pub const MAGIC_ID: u32 = 0x009d_a3b0;
/// The magic files `PhyreScene_GetVFXSortMode` sorts with mode 1, and 2.
pub const SORT_ONE_MAGIC: [u32; 22] = [
    0x119, 0x133, 0x6a, 0x1ae, 0x25d, 0x46, 0x23a, 0x263, 0x1e7, 0x2a1, 0xbc, 0x150, 0x15f, 0x182,
    0x233, 0x264, 0x20a, 0x29c, 0x1bd, 0xe, 0x49, 0x9d,
];
pub const SORT_TWO_MAGIC: [u32; 2] = [0x203, 0x29b];
/// The level hashes (`+0xfaf4`) sorted with mode 0.
pub const SORT_ZERO_LEVELS: [u32; 5] = [0x1eb6, 0x482, 0x1165, 0x1e48, 0x20a6];

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::h3_glue14::bridge::{cursor_over, walk};
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    /// `memcpy(buffer + offset, value, size)` for the parameter `p` (`+8`
    /// offset, `+0xa` size in its low 13 bits) when it fits the buffer
    /// (its size at `+0`) and the offset is at least 4. The original
    /// copies `size` bytes from its local; past the local's end it copies
    /// its frame (junk read: the port writes 0 there; the game's
    /// parameters are the local's size).
    pub(crate) fn write_param(h: &mut impl Mem, buffer: u32, p: u32, value: &[u8]) {
        let offset = u32::from(h.read::<u16>(p.wrapping_add(8)));
        let size = u32::from(h.read::<u16>(p.wrapping_add(0xa)) & 0x1fff);
        if offset + size > word(h, buffer) || offset < 4 {
            return;
        }
        for b in 0..size {
            // junk read: the bytes past the local are the original's frame; 0 (brief 2026-10-24)
            let v = value.get(b as usize).copied().unwrap_or(0);
            h.write(buffer.wrapping_add(offset + b), v);
        }
    }

    /// [`write_param`] from memory at `src` (read first).
    pub(crate) fn copy_param(h: &mut impl Mem, buffer: u32, p: u32, src: u32) {
        let size = u32::from(h.read::<u16>(p.wrapping_add(0xa)) & 0x1fff);
        let bytes: Vec<u8> = (0..size).map(|b| h.read(src.wrapping_add(b))).collect();
        write_param(h, buffer, p, &bytes);
    }

    fn words_bytes(w: &[u32]) -> Vec<u8> {
        w.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    /// Each material of the meshes of `list` (`count`, `data`): `f(h,
    /// material)` for the entries of each mesh's material array (`+0x10`).
    pub(crate) fn each_mesh_material<H: Host>(
        h: &mut H,
        count: u32,
        data: u32,
        mut f: impl FnMut(&mut H, u32),
    ) {
        for i in 0..count {
            let arr = word(h, word(h, data.wrapping_add(4 * i)).wrapping_add(0x10));
            let mut j = 0;
            while j < word(h, arr) & 0x7fff_ffff {
                let d = h.call_sym(ARRAY_DATA, Conv::Thiscall(arr), &[]);
                let mat = word(h, d.wrapping_add(4 * j));
                f(h, mat);
                j += 1;
            }
        }
    }

    /// Every material's `SpecularMapSampler` of a sampler type reset to
    /// `{0x42, 0, 0, 0}` (the mesh list's data read once).
    pub(crate) fn reset_specular_samplers(h: &mut impl Host, inst: u32) {
        let mut value = [0_u8; 16];
        value[0] = 0x42;
        let data = word(h, inst + 4);
        let mut i = 0;
        while i < word(h, inst) & 0x7fff_ffff {
            each_mesh_material(h, 1, data.wrapping_add(4 * i), |h, mat| {
                let s = h.rebase(SPEC_SAMPLER);
                let p = h.call_sym(FIND_PARAM, Conv::Thiscall(word(h, mat)), &[s]);
                if p != 0 && SAMPLER_TYPES.contains(&h.read::<u8>(p.wrapping_add(2))) {
                    let buffer = word(h, mat.wrapping_add(4));
                    write_param(h, buffer, p, &value);
                }
            });
            i += 1;
        }
    }

    /// `ShaderParams_FindByName(params; name)` for an `.rdata` name.
    pub(crate) fn find(h: &mut impl Host, params: u32, name: u32) -> u32 {
        let s = h.rebase(name);
        h.call_sym(FIND_PARAM, Conv::Thiscall(params), &[s])
    }

    /// The named cluster's first texture through the iterator at `iter`
    /// (`+0` plus `+0x10`), or `None` without a cluster or a texture.
    pub(crate) fn cluster_texture(h: &mut impl Host, iter: u32, name: u32) -> Option<u32> {
        let cm = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
        let cluster = h.call_sym(CLUSTER_BY_NAME, Conv::Thiscall(cm), &[name]);
        if cluster == 0 {
            return None;
        }
        h.call_sym(TEXTURE_ITERATOR_CTOR, Conv::Thiscall(iter), &[cluster]);
        if word(h, iter + 0xc) == 0 {
            return None;
        }
        Some(word(h, iter).wrapping_add(word(h, iter + 0x10)))
    }

    /// The `{0x47, 0, linear sampler, 0}` sampler state into `p` (the
    /// buffer read before the sampler is asked for).
    pub(crate) fn write_sampler_state(h: &mut impl Host, mat: u32, p: u32) {
        let buffer = word(h, mat.wrapping_add(4));
        let scene = SCENE.get(h);
        let s = h.call_sym(LINEAR_SAMPLER, Conv::Thiscall(scene), &[1]);
        write_param(h, buffer, p, &words_bytes(&[0x47, 0, s, 0]));
    }

    /// `SpecularMapParamsManager_setShadeSpecularMapParams(inst, name, u0,
    /// u1, shininess)` (stdcall; the frame from the original's `ebp -
    /// 0x7c` at `frame`: the iterator at `+0x10`, the sampler value at
    /// `+0x20`, the list copy at `+0x58`): without a name the samplers are
    /// reset as in [`set_specular_material`]; with one (and a texture), on
    /// a copy of the mesh list: `SpecularMapSamplerState` gets `{0x47, 0,
    /// linear sampler, 0}`, `SpecularMapSampler` the texture (`{0x42, 0,
    /// 0, texture}`, no sampler), `SpecularUvOffset` `(u0, u1)`,
    /// `Shininess` the value three times, `SpecularPower` 0.5 (`.rdata`);
    /// the copy freed unless owned elsewhere.
    pub fn set_shade_specular(
        h: &mut impl Host,
        frame: u32,
        inst: u32,
        name: u32,
        uv: [u32; 2],
        shininess: u32,
    ) {
        if inst == 0 {
            return;
        }
        if name == 0 {
            reset_specular_samplers(h, inst);
            return;
        }
        let uv = words_bytes(&[quiet(uv[0]), quiet(uv[1])]);
        let s = quiet(shininess);
        let shine = words_bytes(&[s, s, s]);
        let Some(texture) = cluster_texture(h, frame + 0x10, name) else {
            return;
        };
        let copy = frame + 0x58;
        h.call_sym(ARRAY_COPY_FROM, Conv::Thiscall(copy), &[inst]);
        let count = word(h, copy) & 0x7fff_ffff;
        let data = word(h, copy + 4);
        let power = quiet(h.read::<u32>(h.rebase(SPEC_POWER_VALUE))).to_le_bytes();
        each_mesh_material(h, count, data, |h, mat| {
            let params = word(h, mat);
            let sampler_p = find(h, params, SPEC_SAMPLER);
            let state_p = find(h, params, SPEC_SAMPLER_STATE);
            if state_p != 0 {
                write_sampler_state(h, mat, state_p);
            }
            let uv_p = find(h, params, SPEC_UV_OFFSET);
            let shine_p = find(h, params, SHININESS);
            if sampler_p != 0 {
                let value = frame + 0x20;
                h.write(value + 0xc, texture);
                let buffer = word(h, mat.wrapping_add(4));
                h.write(value, 0x42_u32);
                h.write(value + 8, 0_u32);
                h.write(value + 4, 0_u32);
                h.call_sym(
                    SET_TEXTURE_CHECKED,
                    Conv::Thiscall(buffer),
                    &[sampler_p, value],
                );
            }
            let buffer = word(h, mat.wrapping_add(4));
            if uv_p != 0 {
                write_param(h, buffer, uv_p, &uv);
            }
            if shine_p != 0 {
                write_param(h, buffer, shine_p, &shine);
            }
            let power_p = find(h, params, SPEC_POWER);
            if power_p != 0 {
                let buffer = word(h, mat.wrapping_add(4));
                write_param(h, buffer, power_p, &power);
            }
        });
        if word(h, copy) >> 31 == 0 {
            let data = word(h, copy + 4);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
    }

    /// `SpecularMapParamsManager_setSpecularMaterial_006c6390(inst, name,
    /// uv0, uv1, vector1, float1, mat1, v2x, v2y, v2w)` (stdcall; the
    /// frame from the original's `ebp - 0x80` at `frame`: the vector
    /// `{v2x, v2y, 1, v2w}` at `+0`, the texture iterator at `+0x10`, the
    /// sampler value at `+0x20`, the state value at `+0x30`, the list copy
    /// at `+0x58`): without a cluster name, every material's
    /// `SpecularMapSampler` of a sampler type is reset to `{0x42, 0, 0,
    /// 0}`. With one (and a texture in the cluster), on a copy of the mesh
    /// list: `SpecularMapSamplerState` gets `{0x47, 0, linear sampler,
    /// 0}`, `SpecularMapSampler` the cluster's texture with the linear
    /// sampler, `SpecularUvOffset` `(uv0, uv1)`, `Specular_SpecialFloat1`
    /// `float1`, `Specular_SpecialMat1` and `Specular_SpecialVector1` the
    /// bytes at the non-null `mat1` and `vector1`,
    /// `Specular_SpecialVector2` the vector; the copy freed unless owned
    /// elsewhere (top bit).
    #[allow(clippy::too_many_arguments)] // the original's ten
    pub fn set_specular_material(
        h: &mut impl Host,
        frame: u32,
        inst: u32,
        name: u32,
        uv: [u32; 2],
        vector1: u32,
        float1: u32,
        mat1: u32,
        v2: [u32; 3],
    ) {
        if inst == 0 {
            return;
        }
        if name == 0 {
            reset_specular_samplers(h, inst);
            return;
        }
        let uv = words_bytes(&[quiet(uv[0]), quiet(uv[1])]);
        let vector2 = [quiet(v2[0]), quiet(v2[1]), 1.0_f32.to_bits(), quiet(v2[2])];
        let float1 = quiet(float1).to_le_bytes();
        for (k, &w) in (0u32..).zip(&vector2) {
            h.write(frame + 4 * k, w);
        }
        let Some(texture) = cluster_texture(h, frame + 0x10, name) else {
            return;
        };
        let copy = frame + 0x58;
        h.call_sym(ARRAY_COPY_FROM, Conv::Thiscall(copy), &[inst]);
        let count = word(h, copy) & 0x7fff_ffff;
        let data = word(h, copy + 4);
        let names = [
            SPEC_SAMPLER,
            SPEC_UV_OFFSET,
            SPEC_FLOAT1,
            SPEC_MAT1,
            SPEC_VECTOR1,
            SPEC_VECTOR2,
            SPEC_SAMPLER_STATE,
        ];
        let vector2 = words_bytes(&vector2);
        each_mesh_material(h, count, data, |h, mat| {
            let params = word(h, mat);
            let mut p = [0_u32; 7];
            for (x, n) in p.iter_mut().zip(names) {
                let s = h.rebase(n);
                *x = h.call_sym(FIND_PARAM, Conv::Thiscall(params), &[s]);
            }
            let [sampler_p, uv_p, float_p, mat_p, vec1_p, vec2_p, state_p] = p;
            if state_p != 0 {
                write_sampler_state(h, mat, state_p);
            }
            if sampler_p != 0 {
                let value = frame + 0x20;
                h.write(value, 0x42_u32);
                h.write(value + 0xc, texture);
                let scene = SCENE.get(h);
                let s = h.call_sym(LINEAR_SAMPLER, Conv::Thiscall(scene), &[1]);
                let buffer = word(h, mat.wrapping_add(4));
                h.write(value + 8, s);
                h.write(value + 4, 0_u32);
                h.call_sym(
                    SET_TEXTURE_CHECKED,
                    Conv::Thiscall(buffer),
                    &[sampler_p, value],
                );
            }
            let buffer = word(h, mat.wrapping_add(4));
            if uv_p != 0 {
                write_param(h, buffer, uv_p, &uv);
            }
            if float_p != 0 {
                write_param(h, buffer, float_p, &float1);
            }
            if mat_p != 0 && mat1 != 0 {
                copy_param(h, buffer, mat_p, mat1);
            }
            if vec1_p != 0 && vector1 != 0 {
                copy_param(h, buffer, vec1_p, vector1);
            }
            if vec2_p != 0 {
                write_param(h, buffer, vec2_p, &vector2);
            }
        });
        if word(h, copy) >> 31 == 0 {
            let data = word(h, copy + 4);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
    }
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f = word(h, h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// `CharacterAnimationTweakUtil_updateAnimation(this)` (the 20-byte
    /// name buffer at `frame`, the original's `ebp - 0x18`): in debug
    /// mode the viewer's animation name (`[g_ViewerManager] + 0x1c`, as
    /// the `sprintf_s` format) gives a motion id (`sg_getdbnum(first
    /// char) << 12 | atoi(rest)`), read with
    /// `ChN_ReadSystemMGRP_ViewerAnimation(id, +0xc)` into `+8`; with a
    /// result and a player, the player's motion type, viewer animation
    /// (`+4` gets it) and loop are set; else `+8` is cleared.
    pub fn update_animation(h: &mut impl Host, frame: u32, this: u32) {
        if h.call_sym(IS_DEBUG_MODE, Conv::Cdecl, &[]) as u8 == 0 {
            return;
        }
        let name = word(h, h.rebase(VIEWER_MANAGER)).wrapping_add(0x1c);
        import(h, SPRINTF_S, &[frame, 0x14, name]);
        let c = i32::from(h.read::<i8>(frame)) as u32;
        let db = h.call_sym(GET_DB_NUM, Conv::Cdecl, &[c]);
        let n = import(h, ATOI, &[frame + 1]);
        let id = n | db << 12;
        let kind = word(h, this + 0xc);
        let r = h.call_sym(READ_VIEWER_ANIMATION, Conv::Cdecl, &[id, kind]);
        h.write(this + 8, r);
        if r == 0 || h.call_sym(GET_PLAYER, Conv::Cdecl, &[]) == 0 {
            h.write(this + 8, 0_u32);
            return;
        }
        let player = h.call_sym(GET_PLAYER, Conv::Cdecl, &[]);
        let kind = word(h, this + 0xc);
        h.call_sym(SET_MOTION_TYPE, Conv::Cdecl, &[player, kind]);
        let last = word(h, this + 8).wrapping_sub(1);
        let (kind, cur, extra) = (word(h, this + 0xc), word(h, this + 4), word(h, this + 0x10));
        let player = h.call_sym(GET_PLAYER, Conv::Cdecl, &[]);
        let m = h.call_sym(
            SET_VIEWER_MOTION,
            Conv::Cdecl,
            &[player, kind, cur, last, extra],
        );
        h.write(this + 4, m);
        let player = h.call_sym(GET_PLAYER, Conv::Cdecl, &[]);
        h.call_sym(MOTION_SET_LOOP, Conv::Cdecl, &[player, 0]);
    }

    /// The cursor mark of a tweak panel line: `">"` on the selected one,
    /// `" "` elsewhere.
    fn mark(h: &impl Mem, selected: bool) -> u32 {
        h.rebase(if selected { MARK_SELECTED } else { MARK_OTHER })
    }

    fn on_off(h: &impl Mem, on: bool) -> u32 {
        h.rebase(if on { TEXT_ON } else { TEXT_OFF })
    }

    /// `CharacterAnimationTweakUtil_updateText_006bc750(this)` (the line
    /// buffer at `frame`, the original's `ebp - 0x38`): while enabled
    /// (`+0x14`), its four lines are printed (`"%s%s%s"` / `"%s%s%d"`:
    /// the mark, the label `+0xa8 + 4 line`, then the motion name from the
    /// table at `0x00c34318` by `+0xc`, on/off by `+0`, `+4`, `+8`) and
    /// handed to `TextUtil_changeLine(+0x18; line, buffer)`.
    pub fn animation_text(h: &mut impl Host, frame: u32, this: u32) {
        if h.read::<u8>(this + 0x14) == 0 {
            return;
        }
        for line in 0..4_u32 {
            h.call_sym(MEMSET, Conv::Cdecl, &[frame, 0, 0x32]);
            if h.read::<u8>(this + 0x14) == 0 {
                import(h, SPRINTF_S, &[frame, 0x28, h.rebase(TEXT_INIT_FAILED)]);
            } else {
                let m = mark(h, word(h, this + 0xbc) == line);
                let label = word(h, this + 0xa8 + 4 * line);
                let (fmt, value) = match line {
                    0 => {
                        let table = h.rebase(MOTION_NAMES);
                        let k = word(h, this + 0xc);
                        (FMT_SSS, word(h, table.wrapping_add(4 * k)))
                    }
                    1 => (FMT_SSS, on_off(h, h.read::<u8>(this) != 0)),
                    2 => (FMT_SSD, word(h, this + 4)),
                    _ => (FMT_SSD, word(h, this + 8)),
                };
                import(h, SPRINTF_S, &[frame, 0x28, h.rebase(fmt), m, label, value]);
            }
            h.call_sym(
                TEXT_CHANGE_LINE,
                Conv::Thiscall(this + 0x18),
                &[line, frame],
            );
        }
    }

    /// `CharacterTextureAnimationTweakUtil_updateText_006bc850(this)` (the
    /// line buffer at `frame`, the original's `ebp - 0x38`): while enabled
    /// (`+0x1c`): in debug mode on a viewer map named like `"grid00"`, six
    /// lines from `FUN_006b7140(this; line, buffer)`; otherwise one line
    /// (mark by `+0xcc`, label `+0xb0`, on/off by `+0x11`), each handed to
    /// `TextUtil_changeLine(+0x20; line, buffer)`.
    pub fn texanim_text(h: &mut impl Host, frame: u32, this: u32) {
        if h.read::<u8>(this + 0x1c) == 0 {
            return;
        }
        if h.call_sym(IS_DEBUG_MODE, Conv::Cdecl, &[]) as u8 != 0 {
            let map = word(h, h.rebase(VIEWER_MANAGER)).wrapping_add(0x30);
            if import(h, STRSTR, &[map, h.rebase(GRID_NAME)]) != 0 {
                for line in 0..6 {
                    h.call_sym(MEMSET, Conv::Cdecl, &[frame, 0, 0x32]);
                    h.call_sym(TEXANIM_LINE, Conv::Thiscall(this), &[line, frame]);
                    h.call_sym(
                        TEXT_CHANGE_LINE,
                        Conv::Thiscall(this + 0x20),
                        &[line, frame],
                    );
                }
                return;
            }
        }
        h.call_sym(MEMSET, Conv::Cdecl, &[frame, 0, 0x32]);
        if h.read::<u8>(this + 0x1c) == 0 {
            import(h, SPRINTF_S, &[frame, 0x14, h.rebase(TEXT_INIT_FAILED)]);
        } else {
            let m = mark(h, word(h, this + 0xcc) == 0);
            let label = word(h, this + 0xb0);
            let v = on_off(h, h.read::<u8>(this + 0x11) != 0);
            import(h, SPRINTF_S, &[frame, 0x32, h.rebase(FMT_SSS), m, label, v]);
        }
        h.call_sym(TEXT_CHANGE_LINE, Conv::Thiscall(this + 0x20), &[0, frame]);
    }
    /// `ObjectList_ctor_0064cec0(this; 2 unread words)` (`ret 8`; the
    /// frame from the original's `ebp - 0x18` at `frame`: the insert
    /// result at `+0`, the head at `+8`): `+0`, `+4` cleared, `+0` the new
    /// list head (`FUN_00654f00(0, 0)`), `+8..+0x10` cleared, `+0x1c` 1.0,
    /// `+0xc` = `+8`, the bucket vector filled
    /// (`FUN_00655460(+8; result, +8, 0x10, &head)`), `+0x14` 7, `+0x18`
    /// 8. -> `this`.
    pub fn object_list_ctor(h: &mut impl Host, frame: u32, this: u32) -> u32 {
        h.write(this, 0_u32);
        h.write(this + 4, 0_u32);
        let head = h.call_sym(LIST_NEW_HEAD, Conv::Stdcall, &[0, 0]);
        h.write(this, head);
        let v = this + 8;
        for k in 0..3 {
            h.write(v + 4 * k, 0_u32);
        }
        h.write(this + 0x1c, 1.0_f32.to_bits());
        h.write(frame + 8, word(h, this));
        let first = word(h, v);
        h.write(v + 4, first);
        h.call_sym(
            VECTOR_FILL,
            Conv::Thiscall(v),
            &[frame, first, 0x10, frame + 8],
        );
        h.write(this + 0x14, 7_u32);
        h.write(this + 0x18, 8_u32);
        this
    }

    /// `CA2W_ctor_00651800(this; text, codepage)`: a null text gives a
    /// null result; otherwise the buffer (`+0`, fixed storage `+4` of 0x80
    /// characters, `AtlConvAllocMemory`) takes `MultiByteToWideChar` of
    /// the text with its NUL; when that fails with
    /// `ERROR_INSUFFICIENT_BUFFER` the length is asked for, the buffer
    /// grown and the conversion retried; a failure left throws
    /// (`AtlThrowLastError_0064fa70`).
    pub fn ca2w(h: &mut impl Host, this: u32, text: u32, codepage: u32) {
        if text == 0 {
            h.write(this, 0_u32);
            return;
        }
        let mut len = 1_u32;
        while h.read::<u8>(text.wrapping_add(len - 1)) != 0 {
            len += 1;
        }
        h.call_sym(CONV_ALLOC, Conv::Cdecl, &[this, len, this + 4, 0x80]);
        let buf = word(h, this);
        let r = import_std(h, MULTI_BYTE_TO_WIDE, &[codepage, 0, text, len, buf, len]);
        if r != 0 {
            return;
        }
        let mut failed = true;
        if import_std(h, GET_LAST_ERROR, &[]) == 0x7a {
            let n = import_std(h, MULTI_BYTE_TO_WIDE, &[codepage, 0, text, len, 0, 0]);
            h.call_sym(CONV_ALLOC, Conv::Cdecl, &[this, n, this + 4, 0x80]);
            let buf = word(h, this);
            failed = import_std(h, MULTI_BYTE_TO_WIDE, &[codepage, 0, text, len, buf, n]) == 0;
        }
        if failed {
            h.call_sym(ATL_THROW_LAST_ERROR, Conv::Cdecl, &[]);
        }
    }

    fn import_std(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f = word(h, h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    /// `DistortionGridDynamicMesh_getValue_006746e0(object, out)`
    /// (stdcall): the accessor's mesh (`..._Get(object)`) copied into
    /// `out`: `+4`, the two members at `+8` and `+0x10` (their assignment
    /// operators), `+0x18`. -> `out + 0x18` (the original's dead `rep
    /// lods` leaves it in `eax`).
    pub fn grid_get_value(h: &mut impl Host, object: u32, out: u32) -> u32 {
        let src = h.call_sym(GRID_GET, Conv::Cdecl, &[object]);
        let v = word(h, src.wrapping_add(4));
        h.write(out.wrapping_add(4), v);
        h.call_sym(
            GRID_ASSIGN_8,
            Conv::Thiscall(out.wrapping_add(8)),
            &[src.wrapping_add(8)],
        );
        h.call_sym(
            GRID_ASSIGN_10,
            Conv::Thiscall(out.wrapping_add(0x10)),
            &[src.wrapping_add(0x10)],
        );
        let v = word(h, src.wrapping_add(0x18));
        h.write(out.wrapping_add(0x18), v);
        word(h, out.wrapping_add(0x18))
    }
    /// `PhyreScene_setLayerGroupVisible_006589e0(scene; group)` -> `al`:
    /// each layer list of `+0xfb50` (`+0xfb48` of them): when its group
    /// byte (`+0x15`, signed) is `group`, each entry's object (`[entry +
    /// 0x1c]`) gets the entry (layer `+0x14` set) or 0 at `+0x1c` and the
    /// result is 1; other groups' objects get 0.
    pub fn set_layer_group(h: &mut impl Host, this: u32, group: u32) -> u8 {
        let mut found = 0_u8;
        let mut i = 0;
        while i < word(h, this + 0xfb48) {
            let slot = word(h, this + 0xfb50).wrapping_add(4 * i);
            let layer = word(h, slot);
            if layer != 0 {
                let mine = i32::from(h.read::<i8>(layer.wrapping_add(0x15))) == group as i32;
                let on = h.read::<u8>(layer.wrapping_add(0x14)) != 0;
                if mine {
                    found = 1;
                }
                let mut j = 0;
                while j < word(h, word(h, word(h, this + 0xfb50).wrapping_add(4 * i))) & 0x7fff_ffff
                {
                    let list = word(
                        h,
                        word(h, word(h, this + 0xfb50).wrapping_add(4 * i)).wrapping_add(4),
                    );
                    let entry = word(h, list.wrapping_add(4 * j));
                    if entry != 0 {
                        let o = word(h, entry.wrapping_add(0x1c));
                        if o != 0 {
                            let v = if mine && on { entry } else { 0 };
                            h.write(o.wrapping_add(0x1c), v);
                        }
                    }
                    j += 1;
                }
            }
            i += 1;
        }
        found
    }

    /// `PhyreScene_linkMapObjects_0066f380(scene)` (the cursor at `ebp -
    /// 0x28`): as `PhyreScene_resetMapObjects_0065f0e0`, but each walked
    /// element's object (`[element + 0x1c]`) gets the element at `+0x1c`.
    pub fn map_objects_link(h: &mut impl Host, ebp: u32, scene: u32) {
        if h.read::<u8>(scene + 0x93c) == 0 || word(h, scene + 0x1128) == 0 {
            return;
        }
        let mut i = 0u32;
        while i < word(h, scene + 0x112c) {
            let objects = word(h, scene + 0x1134);
            let object = word(h, objects.wrapping_add(4 * i));
            let cursor = ebp - 0x28;
            cursor_over(h, cursor, object, MAP_OBJECT_CLASS);
            h.call_sym(CURSOR_BEGIN, Conv::Thiscall(cursor), &[]);
            walk(h, cursor, CURSOR_NEXT, |h, base| {
                let e = word(h, cursor + 0x10).wrapping_add(base);
                let o = word(h, e.wrapping_add(0x1c));
                if o != 0 {
                    h.write(o.wrapping_add(0x1c), e);
                }
            });
            i += 1;
        }
    }

    /// `PhyreScene_GetVFXSortMode(scene)`: 1 for the listed magic files, 2
    /// for 0x203 and 0x29b; else 0 on the listed levels (`+0xfaf4`), 1
    /// elsewhere.
    pub fn vfx_sort_mode(h: &mut impl Host, this: u32) -> u32 {
        let id = h.call_sym(MAGIC_ID, Conv::Cdecl, &[]);
        if SORT_ONE_MAGIC.contains(&id) {
            return 1;
        }
        if SORT_TWO_MAGIC.contains(&id) {
            return 2;
        }
        let level = word(h, this + 0xfaf4);
        u32::from(!SORT_ZERO_LEVELS.contains(&level))
    }
}
