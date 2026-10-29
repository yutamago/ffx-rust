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

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
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
    fn each_mesh_material<H: Host>(
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
    fn reset_specular_samplers(h: &mut impl Host, inst: u32) {
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
    fn find(h: &mut impl Host, params: u32, name: u32) -> u32 {
        let s = h.rebase(name);
        h.call_sym(FIND_PARAM, Conv::Thiscall(params), &[s])
    }

    /// The named cluster's first texture through the iterator at `iter`
    /// (`+0` plus `+0x10`), or `None` without a cluster or a texture.
    fn cluster_texture(h: &mut impl Host, iter: u32, name: u32) -> Option<u32> {
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
    fn write_sampler_state(h: &mut impl Host, mat: u32, p: u32) {
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
}
