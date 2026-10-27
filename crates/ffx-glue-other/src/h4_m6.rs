//! Continuation round 11 of list `h4-glue-other`, batch 2: the dynamic
//! geometry's materials.

use ffx_bridge::x87::{load, store};
use ffx_graphic::host::{Conv, Host};

use crate::h4_m1::{
    CLUSTER_BY_NAME, CLUSTER_MANAGER, COLOUR_PARAM, CURSOR_SEEK, CURSOR_TAG, FIND_PARAM,
    FRAME_BUFFER, LINEAR_SAMPLER, PHYRE_SCENE, POST_PROCESS, SAMPLER_PARAM, SET_VALUE,
    SET_VALUE_DESC, SURFACE_FORMAT, TEXTURE_ITERATOR, TEXTURE_PARAM, same_text,
};

/// `Phyre_PArray_resizeInline_0048c3b0(array; n)` (thiscall, ret 4) -> 0
/// when done.
pub const RESIZE: u32 = 0x0048_c3b0;
/// `PhyreScene_createMaterial_00659710(scene; shader, variant, &a, &b)`
/// (thiscall, ret 0x10): `*a`, `*b` the material's pool handles.
pub const CREATE_MATERIAL: u32 = 0x0065_9710;
/// `user_malloc(size)` (cdecl).
pub const MALLOC: u32 = 0x0068_7020;
/// `memset` through its thunk (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `ShaderParamBuffer_SetValue2(buffer; param, src)` (thiscall, ret 8).
pub const SET_VALUE2: u32 = 0x0063_86c0;
/// `PhyreScene_GetLivePointer00ac(manager)` (thiscall): the D3D11 frame
/// buffer's texture.
pub const LIVE_POINTER: u32 = 0x0065_ded0;
/// `ClassDynamicGeometry_getReadyA8_006a2010(manager)` (thiscall): the
/// frame buffer's texture.
pub const READY_A8: u32 = 0x006a_2010;
/// `Phyre_PParameterBuffer_setTextureChecked_0056e670(buffer; param,
/// record)` (thiscall, ret 8).
pub const SET_TEXTURE: u32 = 0x0056_e670;
/// `graphicSoftParticleSetupMaterial(material, shader, depth, a, b)`
/// (cdecl).
pub const SOFT_PARTICLE: u32 = 0x0064_3e20;
/// Parameter names: `"TargetTexture"` (`0x00b47ed8`), `"UvScaleBias"`
/// (`0x00b425e4`), `"CCTexture"` (`0x00b47ee8`), `"doCameraFadeout"`
/// (`0x00b47ef4`), `"cameraFadeoutFactor"` (`0x00b47f04`).
pub const TARGET_PARAM: u32 = 0x00b4_7ed8;
/// See [`TARGET_PARAM`].
pub const UV_SCALE_PARAM: u32 = 0x00b4_25e4;
/// See [`TARGET_PARAM`].
pub const CC_PARAM: u32 = 0x00b4_7ee8;
/// See [`TARGET_PARAM`].
pub const FADE_PARAM: u32 = 0x00b4_7ef4;
/// See [`TARGET_PARAM`].
pub const FADE_FACTOR_PARAM: u32 = 0x00b4_7f04;
/// Texture names: `"D3D11FrameBuffer"` (`0x00b47f18`), `"RWTT"`
/// (`0x00b4366c`).
pub const D3D11_FRAME_BUFFER: u32 = 0x00b4_7f18;
/// See [`D3D11_FRAME_BUFFER`].
pub const RWTT: u32 = 0x00b4_366c;
/// The soft particle constants (`f32` at `0x00b10f38` and `0x00b92268`).
pub const SOFT_A: u32 = 0x00b1_0f38;
/// See [`SOFT_A`].
pub const SOFT_B: u32 = 0x00b9_2268;
/// Bytes of [`setup_material`]'s frame, from the original's `ebp - 0xf8`:
/// the texture iterator (`+0`), the texture records (`{kind, 0, sampler,
/// texture}`) for `"D3D11FrameBuffer"` (`+0x20`), the colour (`+0x30`),
/// the CC texture (`+0x40`), the target (`+0x50`), a cluster's texture
/// (`+0x60`), no texture (`+0x70`), `"FrameBuffer"` (`+0x80`), `"RWTT"`
/// (`+0x90`), the cursor (`+0xa0`, its last words reused for the uv scale
/// at `+0xb0`), the material's two handles (`+0xc4`, `+0xd0`) and the
/// fade values (`+0xd8`, `+0xe8`).
pub const MATERIAL_FRAME: u32 = 0xf0;

/// `ShaderParams_FindByName(shader; name)` of the image string `name`.
fn find(h: &mut impl Host, shader: u32, name: u32) -> u32 {
    let name = h.rebase(name);
    h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[name])
}

/// A post-process buffer's texture (`0` when its `+4` byte is set).
fn buffer_texture(h: &impl Host, buffer: u32) -> u32 {
    if h.read::<u8>(buffer + 4) == 0 {
        h.read(buffer)
    } else {
        0
    }
}

/// The scene's linear sampler for segment record `offset` of the mesh
/// (`[[this+0x24]+0x94] + offset`, its `+0x24`).
fn sampler(h: &mut impl Host, this: u32, offset: u32) -> u32 {
    let instance: u32 = h.read(this + 0x24);
    let records: u32 = h.read(instance + 0x94);
    let which: u32 = h.read(records.wrapping_add(offset + 0x24));
    let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
    h.call_sym(LINEAR_SAMPLER, Conv::Thiscall(scene), &[which])
}

/// A texture record `{kind, 0, sampler, texture}` at `at` bound to `param`
/// of the material's buffer; the binding's result.
#[allow(clippy::too_many_arguments)]
fn bind(
    h: &mut impl Host,
    this: u32,
    offset: u32,
    material: u32,
    param: u32,
    at: u32,
    texture: u32,
) -> u32 {
    h.write(at, 0xffu32);
    h.write(at + 0xc, texture);
    h.write(at + 4, 0u32);
    let s = sampler(h, this, offset);
    h.write(at + 8, s);
    let buffer: u32 = h.read(material + 4);
    h.call_sym(SURFACE_FORMAT, Conv::Thiscall(buffer), &[param, at])
}

/// `ClassDynamicGeometry_setupMaterial(this; n, names, clusters)`
/// (`0x006a3b50`, thiscall, ret 0xc; `frame` see [`MATERIAL_FRAME`]):
/// finds the shader in the cluster at `+0x20` (none: 0x13; it goes to
/// `+0x28`), sizes the material array `+0x14` to `n` (its error
/// returned), and per segment makes a scene material (its pool handles
/// into `+0x18`/`+0x1c`, made on first need: inline for one segment)
/// coloured white, with the camera fade from the geometry's `+0xb7`, and
/// binds its texture: none without a cluster, else the CC texture, the
/// sampler, and the texture cluster `names[i]` or one of the frame buffers
/// (`"D3D11FrameBuffer"`, `"FrameBuffer"`, `"RWTT"`; another name sets the
/// result to 0xa), then the target texture and the uv scale, then the
/// soft particle setup. Returns 0, 0xa, 3 (the array too short), 0x13 or
/// a callee's error.
#[allow(clippy::too_many_lines)] // the original's per-segment run
pub fn setup_material(
    h: &mut impl Host,
    frame: u32,
    this: u32,
    n: u32,
    names: u32,
    clusters: u32,
) -> u32 {
    let iter = frame;
    let d3d11 = frame + 0x20;
    let colour = frame + 0x30;
    let cc = frame + 0x40;
    let target = frame + 0x50;
    let textured = frame + 0x60;
    let untextured = frame + 0x70;
    let frame_buffer = frame + 0x80;
    let rwtt = frame + 0x90;
    let cursor = frame + 0xa0;
    let uv = frame + 0xb0;
    let handle_a = frame + 0xc4;
    let handle_b = frame + 0xd0;
    let fade = frame + 0xd8;
    let factor = frame + 0xe8;
    let mut result = 0u32;
    let owner: u32 = h.read(this + 0x20);
    if owner == 0 {
        return 0x13;
    }
    let list = owner.wrapping_add(0x1c);
    let head: u32 = h.read(list);
    let first = if head == list { 0 } else { head };
    for k in 0..5 {
        h.write(cursor + 4 * k, 0u32);
    }
    h.write(cursor + 0x14, list);
    h.write(cursor + 0x18, first);
    h.write(cursor + 0x1c, h.rebase(CURSOR_TAG));
    h.call_sym(CURSOR_SEEK, Conv::Thiscall(cursor), &[]);
    if h.read::<u32>(cursor + 0xc) == 0 {
        return 0x13;
    }
    let shader: u32 = {
        let base: u32 = h.read(cursor + 0x10);
        h.read(base.wrapping_add(h.read(cursor)))
    };
    h.write(this + 0x28, shader);
    let p_texture = find(h, shader, TEXTURE_PARAM);
    let p_sampler = find(h, shader, SAMPLER_PARAM);
    let p_target = find(h, shader, TARGET_PARAM);
    let p_uv = find(h, shader, UV_SCALE_PARAM);
    let p_cc = find(h, shader, CC_PARAM);
    let array: u32 = h.read(this + 0x14);
    let r = h.call_sym(RESIZE, Conv::Thiscall(array), &[n]);
    if r != 0 {
        return r;
    }
    let one = 1f32.to_bits();
    let scene_ptr = h.rebase(PHYRE_SCENE);
    for i in 0..n.cast_signed().max(0).cast_unsigned() {
        let offset = 0x6c * i;
        let scene: u32 = h.read(scene_ptr);
        let variant: u32 = h.read(this + 0x2c);
        let material = h.call_sym(
            CREATE_MATERIAL,
            Conv::Thiscall(scene),
            &[shader, variant, handle_a, handle_b],
        );
        let a: u32 = h.read(handle_a);
        if a != 0 || h.read::<u32>(handle_b) != 0 {
            if h.read::<u32>(this + 0x18) == 0 || h.read::<u32>(this + 0x1c) == 0 {
                let bytes = n.wrapping_mul(4);
                let second = if n.wrapping_mul(8) <= 8 {
                    h.write(this + 0x18, this + 0x31);
                    this + 0x35
                } else {
                    let p = h.call_sym(MALLOC, Conv::Cdecl, &[bytes]);
                    h.write(this + 0x18, p);
                    h.call_sym(MALLOC, Conv::Cdecl, &[bytes])
                };
                h.write(this + 0x1c, second);
                let p: u32 = h.read(this + 0x18);
                h.call_sym(MEMSET, Conv::Cdecl, &[p, 0, bytes]);
                let p: u32 = h.read(this + 0x1c);
                h.call_sym(MEMSET, Conv::Cdecl, &[p, 0, bytes]);
            }
            let a: u32 = h.read(handle_a);
            let p: u32 = h.read(this + 0x18);
            h.write(p.wrapping_add(4 * i), a);
            let b: u32 = h.read(handle_b);
            let p: u32 = h.read(this + 0x1c);
            h.write(p.wrapping_add(4 * i), b);
        }
        // white, and the camera fade
        let p = find(h, shader, COLOUR_PARAM);
        if p != 0 {
            for k in 0..4 {
                h.write(colour + 4 * k, one);
            }
            let buffer: u32 = h.read(material + 4);
            h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[p, colour]);
        }
        let p_fade = find(h, shader, FADE_PARAM);
        let p_factor = find(h, shader, FADE_FACTOR_PARAM);
        if p_factor != 0 && p_fade != 0 {
            h.write(fade, 0u32);
            h.write(factor, one);
            let geometry: u32 = h.read(this + 0x24);
            if geometry != 0 {
                let depth: u8 = h.read(geometry + 0xb7);
                if depth != 0 {
                    h.write(fade, one);
                    h.write(factor, store(f64::from(depth)));
                }
            }
            let buffer: u32 = h.read(material + 4);
            h.call_sym(SET_VALUE2, Conv::Thiscall(buffer), &[p_fade, fade]);
            let buffer: u32 = h.read(material + 4);
            h.call_sym(SET_VALUE2, Conv::Thiscall(buffer), &[p_factor, factor]);
        }
        // the material into the array
        let array: u32 = h.read(this + 0x14);
        let count = h.read::<u32>(array) & 0x7fff_ffff;
        if i >= count {
            return 3;
        }
        let elements = match count {
            0 => 0,
            1 => array + 4,
            _ => h.read(array + 4),
        };
        h.write(elements.wrapping_add(4 * i), material);
        // the texture
        if p_texture != 0 {
            let cluster: u32 = h.read(clusters.wrapping_add(4 * i));
            let name_at = names.wrapping_add(4 * i);
            if cluster == 0 {
                let r = bind(h, this, offset, material, p_texture, untextured, 0);
                if r != 0 {
                    return r;
                }
            } else {
                if p_cc != 0 {
                    let manager = h.call_sym(POST_PROCESS, Conv::Cdecl, &[]);
                    let buffer: u32 = h.read(manager + 0x9c);
                    let tex = buffer_texture(h, buffer);
                    let r = bind(h, this, offset, material, p_cc, cc, tex);
                    if r != 0 {
                        return r;
                    }
                }
                let name: u32 = h.read(name_at);
                let manager = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
                let texture_cluster = h.call_sym(CLUSTER_BY_NAME, Conv::Thiscall(manager), &[name]);
                if p_sampler != 0 {
                    let s = sampler(h, this, offset);
                    let buffer: u32 = h.read(material + 4);
                    h.call_sym(SET_VALUE_DESC, Conv::Thiscall(buffer), &[p_sampler, s]);
                }
                let name: u32 = h.read(name_at);
                let r = if texture_cluster != 0 {
                    h.call_sym(TEXTURE_ITERATOR, Conv::Thiscall(iter), &[texture_cluster]);
                    let base: u32 = h.read(iter);
                    let tex = base.wrapping_add(h.read(iter + 0x10));
                    bind(h, this, offset, material, p_texture, textured, tex)
                } else if same_text(h, name, h.rebase(D3D11_FRAME_BUFFER)) {
                    let manager = h.call_sym(POST_PROCESS, Conv::Cdecl, &[]);
                    let tex = h.call_sym(LIVE_POINTER, Conv::Thiscall(manager), &[]);
                    bind(h, this, offset, material, p_texture, d3d11, tex)
                } else if same_text(h, name, h.rebase(FRAME_BUFFER)) {
                    let manager = h.call_sym(POST_PROCESS, Conv::Cdecl, &[]);
                    let tex = h.call_sym(READY_A8, Conv::Thiscall(manager), &[]);
                    bind(h, this, offset, material, p_texture, frame_buffer, tex)
                } else if same_text(h, name, h.rebase(RWTT)) {
                    let scene: u32 = h.read(scene_ptr);
                    let t: u32 = h.read(scene.wrapping_add(0xfc70));
                    let tex = if t == 0 { 0 } else { buffer_texture(h, t) };
                    bind(h, this, offset, material, p_texture, rwtt, tex)
                } else {
                    result = 0xa;
                    0
                };
                if r != 0 {
                    return r;
                }
            }
        }
        // the target and its scale
        if p_target != 0 && p_uv != 0 {
            h.write(target, 0x42u32);
            let manager = h.call_sym(POST_PROCESS, Conv::Cdecl, &[]);
            let buffer: u32 = h.read(manager + 0xac);
            let tex = buffer_texture(h, buffer);
            h.write(target + 0xc, tex);
            let s = sampler(h, this, offset);
            h.write(target + 8, s);
            h.write(target + 4, 0u32);
            let buffer: u32 = h.read(material + 4);
            let r = h.call_sym(SET_TEXTURE, Conv::Thiscall(buffer), &[p_target, target]);
            if r != 0 {
                return r;
            }
            h.write(uv, one);
            h.write(uv + 4, one);
            h.write(uv + 8, 0u32);
            h.write(uv + 0xc, 0u32);
            let buffer: u32 = h.read(material + 4);
            let r = h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[p_uv, uv]);
            if r != 0 {
                return r;
            }
        }
        let geometry: u32 = h.read(this + 0x24);
        let depth = u32::from(h.read::<u8>(geometry + 0xb6));
        let shader: u32 = h.read(this + 0x28);
        let a = store(load(h.read(h.rebase(SOFT_A))));
        let b = store(load(h.read(h.rebase(SOFT_B))));
        h.call_sym(SOFT_PARTICLE, Conv::Cdecl, &[material, shader, depth, a, b]);
    }
    result
}
