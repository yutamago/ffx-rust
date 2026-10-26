//! The dynamic geometries' texture parameters (vtable slot 3 of
//! `ClassDynamicGeometry` and its Menu, VFX and `GuildMap` kinds, h2p4
//! round).
//!
//! Layout used (`ClassDynamicGeometry`): `+0x14` the materials (`PArray`
//! of `{_, parameter buffer}` pointers), `+0x24` the render data (`+0x90`
//! the texture entries' count, `+0x94` the 0x6c-byte entries: `+0x20` the
//! texture's name, `+0x24` its sampler kind), `+0x28` the shader.

use ffx_graphic::host::{Conv, Host};

/// `ShaderParams_FindByName` (thiscall `(shader, name)` -> parameter).
pub const FIND_PARAM: u32 = 0x0056_cbf0;
/// `memcpy` (cdecl `(dst, src, n)`).
pub const MEMCPY: u32 = 0x0094_9256;
/// `MSVCR110!strstr`'s import slot.
pub const STRSTR_SLOT: u32 = 0x00b0_c378;
/// `g_PhyreScene`.
pub const G_SCENE: u32 = 0x00cc_b9d8;
/// `PhyreScene_getLinearSampler` (thiscall `(scene, which)`, `ret 4`).
pub const LINEAR_SAMPLER: u32 = 0x0065_e050;
/// `ClusterManager_GetInstance` (cdecl `()`).
pub const CLUSTER_MANAGER: u32 = 0x0069_a1a0;
/// `ClusterManager_getPClusterByName` (thiscall `(mgr, name)`, `ret 4`).
pub const CLUSTER_BY_NAME: u32 = 0x0069_b4b0;
/// `PTextureIterator_ctor_00463bf0` (thiscall `(it, cluster)`, `ret 4`).
pub const TEXTURE_ITERATOR: u32 = 0x0046_3bf0;
/// `Render_SurfaceFormat_0066e4f0` (thiscall `(buffer, param, desc)`,
/// `ret 8`) -> 0 or an error.
pub const SET_TEXTURE: u32 = 0x0066_e4f0;
/// `Phyre_PParameterBuffer_setTextureChecked_0056e670` (thiscall
/// `(buffer, param, desc)`, `ret 8`) -> 0 or an error.
pub const SET_TEXTURE_CHECKED: u32 = 0x0056_e670;
/// `TrophyManager_Install` (thiscall `(sampler, texture)`, `ret 4`) -> 0
/// (on PC).
pub const BIND_SAMPLER: u32 = 0x0059_e520;
/// `PhyreScene_GetField0dd8`, `PhyreScene_GetField0ddc`,
/// `PhyreScene_getVideoTexture` (thiscall `(scene)`): the movie's Y, U, V
/// planes.
pub const VIDEO_Y: u32 = 0x0065_e650;
pub const VIDEO_U: u32 = 0x0065_e630;
pub const VIDEO_V: u32 = 0x0065_e640;
/// The names: `"UvScaleBias"`, `"TextureSamplerState"`, `"TextureSampler"`,
/// `"VideoTexture"`, `"TextureSamplerY"`, `"TextureSamplerU"`,
/// `"TextureSamplerV"`.
pub const UV_SCALE_BIAS: u32 = 0x00b4_25e4;
pub const SAMPLER_STATE: u32 = 0x00b4_7ec4;
pub const SAMPLER: u32 = 0x00b3_017c;
pub const VIDEO_TEXTURE: u32 = 0x00b4_24a8;
pub const SAMPLER_Y: u32 = 0x00b4_7f2c;
pub const SAMPLER_U: u32 = 0x00b4_7f3c;
pub const SAMPLER_V: u32 = 0x00b4_7f4c;

const ONE: u32 = 0x3f80_0000;
const COUNT: u32 = 0x7fff_ffff;

pub mod bridge {
    //! The original over game memory.

    use super::{
        BIND_SAMPLER, CLUSTER_BY_NAME, CLUSTER_MANAGER, COUNT, Conv, FIND_PARAM, G_SCENE, Host,
        LINEAR_SAMPLER, MEMCPY, ONE, SAMPLER, SAMPLER_STATE, SAMPLER_U, SAMPLER_V, SAMPLER_Y,
        SET_TEXTURE, SET_TEXTURE_CHECKED, STRSTR_SLOT, TEXTURE_ITERATOR, UV_SCALE_BIAS,
        VIDEO_TEXTURE, VIDEO_U, VIDEO_V, VIDEO_Y,
    };

    /// The parameter `p`'s place in `buf` (its first word the size): its
    /// offset when it fits after the header word.
    fn place(h: &impl Host, p: u32, buf: u32) -> Option<u32> {
        let off = u32::from(h.read::<u16>(p.wrapping_add(8)));
        let size = u32::from(h.read::<u16>(p.wrapping_add(0xa)) & 0x1fff);
        (off + size <= h.read::<u32>(buf) && off >= 4).then_some(off)
    }

    fn size(h: &impl Host, p: u32) -> u32 {
        u32::from(h.read::<u16>(p.wrapping_add(0xa)) & 0x1fff)
    }

    /// Entry `base / 0x6c`'s sampler kind, re-read through the render
    /// data (`[[geo + 0x24] + 0x94] + base + 0x24`).
    fn kind(h: &impl Host, geo: u32, base: u32) -> u32 {
        let data: u32 = h.read(geo + 0x24);
        let entries: u32 = h.read(data.wrapping_add(0x94));
        h.read(entries.wrapping_add(base + 0x24))
    }

    fn find(h: &mut impl Host, shader: u32, name: u32) -> u32 {
        let name = h.rebase(name);
        h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[name])
    }

    /// `ClassDynamicGeometry` vtable slot 3 (`0x006a59d0`, fastcall
    /// `(geo)`) -> 0, 3 (a parameter that doesn't fit), 5 (a video
    /// sampler of the wrong type) or a callee's error. For each texture
    /// entry `i` with a name and a material (`+0x14`, element `i`), with
    /// the shader `+0x28`'s parameters: `"TextureSamplerState"` gets
    /// `{0x47, 0, linear sampler(kind), 0}` (when it fits); a texture
    /// cluster of that name gets `"TextureSampler"` (`{0xff, 0, sampler,
    /// first + count}` of its texture iterator at `ebp - 0x98`, set by
    /// `0x0066e4f0`); else a `"VideoTexture"` name gets the movie's
    /// planes: `"TextureSamplerY"` `{0x42, 0, sampler, Y}` (copied; its
    /// type byte must be one of 0x42, 0xd9..0xdc, 0x12; the sampler bound
    /// to Y first), `"TextureSamplerU"` `{0x42, 1, sampler, U}` and
    /// `"TextureSamplerV"` `{0x42, 2, sampler, V}` (set checked). Then
    /// `"UvScaleBias"` gets `(1, 1, 0, 0)`. The entry is copied to `ebp -
    /// 0x104` first, and every block handed on lives in the original's
    /// frame at its offset (`ebp` is that frame).
    #[allow(clippy::too_many_lines)]
    pub fn update_texture(h: &mut impl Host, ebp: u32, geo: u32) -> u32 {
        let mut data: u32 = h.read(geo + 0x24);
        if data == 0 {
            return 0;
        }
        h.write(ebp - 0x14, 0u32);
        if h.read::<i32>(data.wrapping_add(0x90)) <= 0 {
            return 0;
        }
        h.write(ebp - 8, 0u32);
        let scene_at = h.rebase(G_SCENE);
        let mut i = 0u32;
        loop {
            let base = 0x6c * i;
            let entries: u32 = h.read(data.wrapping_add(0x94));
            let entry = entries.wrapping_add(base);
            for k in 0..0x1b {
                let w: u32 = h.read(entry.wrapping_add(4 * k));
                h.write(ebp - 0x104 + 4 * k, w);
            }
            h.write(ebp - 4, entry);
            let name: u32 = h.read(ebp - 0xe4);
            if name != 0 {
                let arr: u32 = h.read(geo + 0x14);
                let elems = match h.read::<u32>(arr) & COUNT {
                    0 => 0,
                    1 => arr.wrapping_add(4),
                    _ => h.read(arr.wrapping_add(4)),
                };
                let mat: u32 = h.read(elems.wrapping_add(4 * i));
                h.write(ebp - 0xc, mat);
                if mat != 0 {
                    let r = texture(h, ebp, geo, i, entry, name, mat, scene_at);
                    if r != 0 {
                        return r;
                    }
                }
            }
            i += 1;
            h.write(ebp - 0x14, i);
            h.write(ebp - 8, 0x6c * i);
            data = h.read(geo + 0x24);
            if h.read::<i32>(data.wrapping_add(0x90)) <= i as i32 {
                return 0;
            }
        }
    }

    /// One entry of [`update_texture`]; 0 to go on.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn texture(
        h: &mut impl Host,
        ebp: u32,
        geo: u32,
        i: u32,
        entry: u32,
        name: u32,
        mat: u32,
        scene_at: u32,
    ) -> u32 {
        let base = 0x6c * i;
        let shader: u32 = h.read(geo + 0x28);
        let uvsb = find(h, shader, UV_SCALE_BIAS);
        h.write(ebp - 0x18, uvsb);
        let state = find(h, shader, SAMPLER_STATE);
        if state != 0 {
            let buf: u32 = h.read(mat + 4);
            h.write(ebp - 0x10, buf);
            let scene: u32 = h.read(scene_at);
            h.write(ebp - 0x28, 0x47u32);
            h.write(ebp - 0x24, 0u32);
            h.write(ebp - 0x1c, 0u32);
            let which: u32 = h.read(entry + 0x24);
            let s = h.call_sym(LINEAR_SAMPLER, Conv::Thiscall(scene), &[which]);
            h.write(ebp - 0x20, s);
            let off = u32::from(h.read::<u16>(state + 8));
            h.write(ebp - 4, off);
            if let Some(off) = place(h, state, buf) {
                let n = size(h, state);
                h.call_sym(MEMCPY, Conv::Cdecl, &[off.wrapping_add(buf), ebp - 0x28, n]);
            }
        }
        let mgr = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
        let cluster = h.call_sym(CLUSTER_BY_NAME, Conv::Thiscall(mgr), &[name]);
        h.write(ebp - 4, cluster);
        if cluster != 0 {
            let p = find(h, shader, SAMPLER);
            if p != 0 {
                h.call_sym(TEXTURE_ITERATOR, Conv::Thiscall(ebp - 0x98), &[cluster]);
                let a: u32 = h.read(ebp - 0x98);
                let b: u32 = h.read(ebp - 0x88);
                h.write(ebp - 0x3c, a.wrapping_add(b));
                let scene: u32 = h.read(scene_at);
                h.write(ebp - 0x48, 0xffu32);
                h.write(ebp - 0x44, 0u32);
                let which = kind(h, geo, base);
                let s = h.call_sym(LINEAR_SAMPLER, Conv::Thiscall(scene), &[which]);
                h.write(ebp - 0x40, s);
                let buf: u32 = h.read(mat + 4);
                let e = h.call_sym(SET_TEXTURE, Conv::Thiscall(buf), &[p, ebp - 0x48]);
                if e != 0 {
                    return e;
                }
            }
        } else {
            let strstr: u32 = h.read(h.rebase(STRSTR_SLOT));
            let video = h.rebase(VIDEO_TEXTURE);
            if h.call(strstr, Conv::Cdecl, &[name, video]) != 0 {
                let y = find(h, shader, SAMPLER_Y);
                h.write(ebp - 0x10, y);
                let scene: u32 = h.read(scene_at);
                h.write(ebp - 0x78, 0x42u32);
                let plane = h.call_sym(VIDEO_Y, Conv::Thiscall(scene), &[]);
                let which = kind(h, geo, base);
                let scene: u32 = h.read(scene_at);
                h.write(ebp - 0x6c, plane);
                h.write(ebp - 0x74, 0u32);
                let s = h.call_sym(LINEAR_SAMPLER, Conv::Thiscall(scene), &[which]);
                h.write(ebp - 0x70, s);
                let buf: u32 = h.read(mat + 4);
                h.write(ebp - 4, buf);
                let ty = h.read::<u8>(y + 2);
                if ![0x42, 0xd9, 0xda, 0xdb, 0xdc, 0x12].contains(&ty) {
                    return 5;
                }
                if s != 0 && plane != 0 {
                    let e = h.call_sym(BIND_SAMPLER, Conv::Thiscall(s), &[plane]);
                    if e != 0 {
                        return e;
                    }
                }
                let Some(off) = place(h, y, buf) else {
                    return 3;
                };
                let n = size(h, y);
                h.call_sym(MEMCPY, Conv::Cdecl, &[buf.wrapping_add(off), ebp - 0x78, n]);
                for (param, at, plane_fn, k) in [
                    (SAMPLER_U, 0x68, VIDEO_U, 1u32),
                    (SAMPLER_V, 0x38, VIDEO_V, 2),
                ] {
                    let p = find(h, shader, param);
                    let scene: u32 = h.read(scene_at);
                    h.write(ebp - at, 0x42u32);
                    let plane = h.call_sym(plane_fn, Conv::Thiscall(scene), &[]);
                    h.write(ebp - at + 0xc, plane);
                    h.write(ebp - at + 4, k);
                    let which = kind(h, geo, base);
                    let scene: u32 = h.read(scene_at);
                    let s = h.call_sym(LINEAR_SAMPLER, Conv::Thiscall(scene), &[which]);
                    h.write(ebp - at + 8, s);
                    let buf: u32 = h.read(mat + 4);
                    let e = h.call_sym(SET_TEXTURE_CHECKED, Conv::Thiscall(buf), &[p, ebp - at]);
                    if e != 0 {
                        return e;
                    }
                }
            }
        }
        let uvsb: u32 = h.read(ebp - 0x18);
        if uvsb != 0 {
            h.write(ebp - 0x58, ONE);
            h.write(ebp - 0x54, ONE);
            h.write(ebp - 0x50, 0u32);
            h.write(ebp - 0x4c, 0u32);
            let buf: u32 = h.read(mat + 4);
            let Some(off) = place(h, uvsb, buf) else {
                return 3;
            };
            let n = size(h, uvsb);
            h.call_sym(MEMCPY, Conv::Cdecl, &[off.wrapping_add(buf), ebp - 0x58, n]);
        }
        0
    }
}
