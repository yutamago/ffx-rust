//! Continuation round 6 of list `h4-glue-other`, batch 5: two more
//! shader-cluster setups, the motion blur's and the movie player's.
//!
//! Both follow [`crate::h4_h4`]'s pattern: a cluster cursor in the
//! original's frame, parameters bound by name, a one-entry material list
//! ([`crate::h4_h2::resize_to_one`]) on a full-screen mesh. Their
//! texture parameters go through descriptors built in the cursor's upper
//! half.

use ffx_graphic::host::{Conv, Host};

use crate::h4_h2::{CLASSES, CLUSTER_CREATE, CREATE_MATERIAL, TEXTURE_REGISTER, resize_to_one};
use crate::h4_h4::{
    FIND_PARAMETER, FIXUP_CLUSTERS, FULLSCREEN_MESH, GET_APPLICATION, SET_TEXTURE, SET_WORD,
    seek_variant,
};

/// `ShaderParamBuffer_SetValue2(buffer; parameter, src)` (thiscall, ret 8).
pub const SET_VALUE2: u32 = 0x0063_86c0;
/// `ShaderParamBuffer_SetValue(buffer; parameter, src)` (thiscall, ret 8).
pub const SET_VALUE: u32 = 0x0063_8780;
/// The motion blur's parameters: `"BaseTexture"`, `"TargetTexture"`,
/// `"BlendWeight"`, `"UvScaleBias"`.
pub const MOTION_PARAMETERS: [u32; 4] = [0x00b4_95cc, 0x00b4_7ed8, 0x00b4_95d8, 0x00b4_25e4];
/// Bytes of [`motion_blur_setup`]'s frame: the cursor (its upper half
/// later the parameter values).
pub const MOTION_FRAME: u32 = 0x20;

/// A `{0x42, 0, sampler, texture}` descriptor at `desc` bound to
/// `parameter` of the material at `this + material_at`
/// (`Phyre_PParameterBuffer_setTextureChecked_0056e670` on its buffer
/// `+4`).
fn bind_texture(
    h: &mut impl Host,
    desc: u32,
    material: u32,
    parameter: u32,
    [sampler, texture]: [u32; 2],
) {
    h.write(desc.wrapping_add(0xc), texture);
    h.write(desc, 0x42u32);
    h.write(desc.wrapping_add(8), sampler);
    h.write(desc.wrapping_add(4), 0u32);
    let buffer: u32 = h.read(material.wrapping_add(4));
    h.call_sym(SET_TEXTURE, Conv::Thiscall(buffer), &[parameter, desc]);
}

/// `MotionBlur_setupShaders_006c5150(this; base, target)` (thiscall, ret
/// 8; called by `MotionBlur_onShaderClusterLoaded_006c53e0`; `frame` its
/// cursor at `ebp - 0x20`): the shader cluster `+0x50` fixed up, its
/// variant (`+0x64`; no hit: 0x13); the sampler `+0x68` (bytes `+0`/`+1`
/// set unless bit 7 of its `+0x1c`) registered; the material (`+0x60`;
/// `this` is the scene cluster); `BaseTexture` and `TargetTexture` bound
/// to `base` and `target` with the sampler, `BlendWeight` from `+0x5c`,
/// `UvScaleBias` (1, 1, 0, 0); a one-entry material list in `this` (none:
/// 0xd) on a full-screen mesh (`+0x54`). Returns 0.
pub fn motion_blur_setup(h: &mut impl Host, frame: u32, this: u32, base: u32, target: u32) -> u32 {
    h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
    let shaders = this.wrapping_add(0x50);
    h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[shaders, 1]);
    let cluster: u32 = h.read(shaders);
    let variant = seek_variant(h, frame, cluster);
    if h.read::<u32>(frame.wrapping_add(0xc)) == 0 {
        return 0x13;
    }
    h.write(this.wrapping_add(0x64), variant);
    let sampler = this.wrapping_add(0x68);
    if h.read::<u32>(sampler.wrapping_add(0x1c)) >> 7 & 1 == 0 {
        h.write(sampler, 1u8);
        h.write(sampler.wrapping_add(1), 1u8);
    }
    h.call_sym(TEXTURE_REGISTER, Conv::Thiscall(sampler), &[]);
    let variant: u32 = h.read(this.wrapping_add(0x64));
    let material = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(variant), &[this, 0, 0]);
    h.write(this.wrapping_add(0x60), material);
    let desc = frame.wrapping_add(0x10);
    for (k, texture) in [(0, base), (1, target)] {
        let variant: u32 = h.read(this.wrapping_add(0x64));
        let name = h.rebase(MOTION_PARAMETERS[k]);
        let p = h.call_sym(FIND_PARAMETER, Conv::Thiscall(variant), &[name]);
        if p != 0 {
            let material: u32 = h.read(this.wrapping_add(0x60));
            bind_texture(h, desc, material, p, [sampler, texture]);
        }
    }
    let variant: u32 = h.read(this.wrapping_add(0x64));
    let name = h.rebase(MOTION_PARAMETERS[2]);
    let p = h.call_sym(FIND_PARAMETER, Conv::Thiscall(variant), &[name]);
    if p != 0 {
        let material: u32 = h.read(this.wrapping_add(0x60));
        let buffer: u32 = h.read(material.wrapping_add(4));
        h.call_sym(
            SET_VALUE2,
            Conv::Thiscall(buffer),
            &[p, this.wrapping_add(0x5c)],
        );
    }
    let variant: u32 = h.read(this.wrapping_add(0x64));
    let name = h.rebase(MOTION_PARAMETERS[3]);
    let p = h.call_sym(FIND_PARAMETER, Conv::Thiscall(variant), &[name]);
    if p != 0 {
        let one = 1.0f32.to_bits();
        h.write(desc, one);
        h.write(desc.wrapping_add(4), one);
        h.write(desc.wrapping_add(8), 0u32);
        h.write(desc.wrapping_add(0xc), 0u32);
        let material: u32 = h.read(this.wrapping_add(0x60));
        let buffer: u32 = h.read(material.wrapping_add(4));
        h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[p, desc]);
    }
    let class = h.rebase(CLASSES[0]);
    let list = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(this), &[1, class, 0]);
    if list == 0 {
        return 0xd;
    }
    resize_to_one(h, list);
    let material: u32 = h.read(this.wrapping_add(0x60));
    h.call_sym(SET_WORD, Conv::Thiscall(list), &[0, material]);
    let mesh = h.call_sym(FULLSCREEN_MESH, Conv::Cdecl, &[this]);
    h.write(this.wrapping_add(0x54), mesh);
    h.write(mesh.wrapping_add(0x10), list);
    0
}

// ---- the movie player

/// `PhyreSceneCursor_Seek4_0065ecb0(cursor)` (thiscall).
pub const CURSOR_SEEK4: u32 = 0x0065_ecb0;
/// The class the movie player's cursor looks for (`0x00ca5268`).
pub const MATERIAL_CLASS: u32 = 0x00ca_5268;
/// `Render_SurfaceFormat_0066e4f0(buffer; parameter, &desc)` (thiscall,
/// ret 8): binds a movie plane.
pub const BIND_PLANE: u32 = 0x0066_e4f0;
/// The planes' parameters (`"YChannel"`, `"UChannel"`, `"VChannel"`) and
/// their textures' offsets in the player.
pub const PLANES: [(u32, u32); 3] = [
    (0x00b4_9d7c, 0x38c),
    (0x00b4_9d88, 0x400),
    (0x00b4_9d94, 0x474),
];
/// `"imageWidth"`, `"imageHeight"`.
pub const IMAGE_SIZE: [u32; 2] = [0x00b4_9da0, 0x00b4_9dac];
/// Bytes of [`fmv_on_shaders`]'s frame (from `ebp - 0x28`): the cursor
/// (its upper half later a plane descriptor), the material `+0x20`, the
/// width parameter `+0x24`.
pub const FMV_FRAME: u32 = 0x28;

/// `PhyFMVPlayerManager_doOnShaderBinFilesLoaded(this)` (`0x006d86e0`,
/// thiscall; `frame` from `ebp - 0x28`): the shader cluster `+0x360` fixed
/// up (a non-zero result is returned); a cursor for the material class
/// (`PhyreSceneCursor_Seek4`; no hit 0x13): the material (`+0x20` of the
/// frame) binds `YChannel`, `UChannel`, `VChannel` to the player's plane
/// textures (`+0x38c`, `+0x400`, `+0x474`) through `{0xff, 0, this +
/// 0x364, texture}` descriptors (a non-zero result is returned), sets
/// `imageWidth`/`imageHeight` from `+0x6f4`/`+0x6f8` when both exist; a
/// one-entry list in the cluster `+0x310` (none 0xd) set to the material
/// (a non-zero result is returned) on a full-screen mesh (`+0x388`).
/// Returns 0.
pub fn fmv_on_shaders(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
    let shaders = this.wrapping_add(0x360);
    let r = h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[shaders, 1]);
    if r != 0 {
        return r;
    }
    let cluster: u32 = h.read(shaders);
    let list = cluster.wrapping_add(0x1c);
    let first: u32 = h.read(list);
    let head = if first == list { 0 } else { first };
    for k in 0..5u32 {
        h.write(frame.wrapping_add(4 * k), 0u32);
    }
    h.write(frame.wrapping_add(0x14), list);
    h.write(frame.wrapping_add(0x18), head);
    let class = h.rebase(MATERIAL_CLASS);
    h.write(frame.wrapping_add(0x1c), class);
    h.call_sym(CURSOR_SEEK4, Conv::Thiscall(frame), &[]);
    if h.read::<u32>(frame.wrapping_add(0xc)) == 0 {
        return 0x13;
    }
    let material = h
        .read::<u32>(frame.wrapping_add(0x10))
        .wrapping_add(h.read(frame));
    h.write(frame.wrapping_add(0x20), material);
    let params: u32 = h.read(material);
    let desc = frame.wrapping_add(0x10);
    for (name, texture) in PLANES {
        let name = h.rebase(name);
        let p = h.call_sym(FIND_PARAMETER, Conv::Thiscall(params), &[name]);
        if p != 0 {
            h.write(desc.wrapping_add(0xc), this.wrapping_add(texture));
            h.write(desc.wrapping_add(8), this.wrapping_add(0x364));
            h.write(desc, 0xffu32);
            h.write(desc.wrapping_add(4), 0u32);
            let buffer: u32 = h.read(material.wrapping_add(4));
            let r = h.call_sym(BIND_PLANE, Conv::Thiscall(buffer), &[p, desc]);
            if r != 0 {
                return r;
            }
        }
    }
    let name = h.rebase(IMAGE_SIZE[0]);
    let width = h.call_sym(FIND_PARAMETER, Conv::Thiscall(params), &[name]);
    h.write(frame.wrapping_add(0x24), width);
    let name = h.rebase(IMAGE_SIZE[1]);
    let height = h.call_sym(FIND_PARAMETER, Conv::Thiscall(params), &[name]);
    if width != 0 && height != 0 {
        let buffer: u32 = h.read(material.wrapping_add(4));
        h.call_sym(
            SET_VALUE2,
            Conv::Thiscall(buffer),
            &[width, this.wrapping_add(0x6f4)],
        );
        let buffer: u32 = h.read(material.wrapping_add(4));
        h.call_sym(
            SET_VALUE2,
            Conv::Thiscall(buffer),
            &[height, this.wrapping_add(0x6f8)],
        );
    }
    let scene = this.wrapping_add(0x310);
    let class = h.rebase(CLASSES[0]);
    let list = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(scene), &[1, class, 0]);
    if list == 0 {
        return 0xd;
    }
    resize_to_one(h, list);
    let r = h.call_sym(SET_WORD, Conv::Thiscall(list), &[0, material]);
    if r != 0 {
        return r;
    }
    let mesh = h.call_sym(FULLSCREEN_MESH, Conv::Cdecl, &[scene]);
    h.write(this.wrapping_add(0x388), mesh);
    h.write(mesh.wrapping_add(0x10), list);
    0
}

// ---- the broken-screen effect

/// `fiosUnifyFilename(name, out, n)` (cdecl).
pub const FIOS_UNIFY: u32 = crate::h4_h2::FIOS_UNIFY;
/// `Phyre_PCluster_LoadAssetFile(cluster, path, 0)` (cdecl).
pub const LOAD_ASSET_FILE: u32 = 0x0044_06c0;
/// The broken-screen shader's path pointer (`0x00c3440c`).
pub const BROKEN_SCREEN_SHADER: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00c3_440c);
/// `BrokenScreenEffect_createRenderTarget(this)` (thiscall).
pub const BROKEN_CREATE_RT: u32 = 0x006d_5050;
/// `"ColorBuffer"` (`0x00b425d8`).
pub const COLOR_BUFFER: u32 = 0x00b4_25d8;
/// Bytes of [`broken_screen_initialize`]'s frame (from `ebp - 0x12c`):
/// the first material `+0`, the cursor `+4` (its `+0x10` later a texture
/// descriptor), the variant and then the second material `+0x24`, the
/// unified path `+0x28` (0x100 bytes).
pub const BROKEN_FRAME: u32 = 0x128;

/// `BrokenScreenEffect_initialize(this; a, b, c, d, e, w, h)`
/// (`0x006d51d0`, thiscall, ret 0x1c; `frame` from `ebp - 0x12c`): `+0x2c`
/// a, `+0x30` b, `+0x5c` c (the colour texture), `+0x60` d, `+0x64` e; the
/// shader (`[0x00c3440c]` unified into the frame) loaded into the cluster
/// `+0x3c` (failure: 0xa) and fixed up; its variant (`+0x40`; no hit 0x13);
/// a material with a one-entry list (`+0x44`) and a full-screen mesh
/// (`+0x58`; a missing material, list or mesh: 0xd); a second material in
/// a second one-entry list on that mesh, its `ColorBuffer` bound to `+0x5c`
/// through a `{0x42, 0, 0, texture}` descriptor; `+0x6c` w, `+0x70` h and
/// the render target made (`BrokenScreenEffect_createRenderTarget`).
/// Returns 0.
pub fn broken_screen_initialize(h: &mut impl Host, frame: u32, this: u32, a: [u32; 7]) -> u32 {
    for (off, v) in [
        (0x2cu32, a[0]),
        (0x30, a[1]),
        (0x5c, a[2]),
        (0x60, a[3]),
        (0x64, a[4]),
    ] {
        h.write(this.wrapping_add(off), v);
    }
    let path = frame.wrapping_add(0x28);
    let shader = BROKEN_SCREEN_SHADER.get(h);
    h.call_sym(FIOS_UNIFY, Conv::Cdecl, &[shader, path, 0x100]);
    let shaders = this.wrapping_add(0x3c);
    if h.call_sym(LOAD_ASSET_FILE, Conv::Cdecl, &[shaders, path, 0]) != 0 {
        return 0xa;
    }
    h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
    h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[shaders, 1]);
    let cursor = frame.wrapping_add(4);
    let cluster: u32 = h.read(shaders);
    let variant = seek_variant(h, cursor, cluster);
    if h.read::<u32>(cursor.wrapping_add(0xc)) == 0 {
        return 0x13;
    }
    let saved = frame.wrapping_add(0x24);
    h.write(saved, variant);
    h.write(this.wrapping_add(0x40), variant);
    let scene: u32 = h.read(this.wrapping_add(0x50));
    let material = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(variant), &[scene, 0, 0]);
    h.write(frame, material);
    if material == 0 {
        return 0xd;
    }
    let scene: u32 = h.read(this.wrapping_add(0x50));
    let class = h.rebase(CLASSES[0]);
    let list = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(scene), &[1, class, 0]);
    h.write(this.wrapping_add(0x44), list);
    if list == 0 {
        return 0xd;
    }
    resize_to_one(h, list);
    let list: u32 = h.read(this.wrapping_add(0x44));
    let material: u32 = h.read(frame);
    h.call_sym(SET_WORD, Conv::Thiscall(list), &[0, material]);
    let scene: u32 = h.read(this.wrapping_add(0x50));
    let mesh = h.call_sym(FULLSCREEN_MESH, Conv::Cdecl, &[scene]);
    h.write(this.wrapping_add(0x58), mesh);
    if mesh == 0 {
        return 0xd;
    }
    let scene: u32 = h.read(this.wrapping_add(0x50));
    let second = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(variant), &[scene, 0, 0]);
    h.write(saved, second);
    let scene: u32 = h.read(this.wrapping_add(0x50));
    let list = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(scene), &[1, class, 0]);
    resize_to_one(h, list);
    h.call_sym(SET_WORD, Conv::Thiscall(list), &[0, second]);
    let mesh: u32 = h.read(this.wrapping_add(0x58));
    h.write(mesh.wrapping_add(0x10), list);
    let params: u32 = h.read(second);
    let name = h.rebase(COLOR_BUFFER);
    let p = h.call_sym(FIND_PARAMETER, Conv::Thiscall(params), &[name]);
    if p != 0 {
        let texture: u32 = h.read(this.wrapping_add(0x5c));
        bind_texture(h, cursor.wrapping_add(0x10), second, p, [0, texture]);
    }
    h.write(this.wrapping_add(0x6c), a[5]);
    h.write(this.wrapping_add(0x70), a[6]);
    h.call_sym(BROKEN_CREATE_RT, Conv::Thiscall(this), &[]);
    0
}
