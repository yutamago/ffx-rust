//! Continuation round 6 of list `h4-glue-other`, batch 4: the blur
//! effects' shader setup: `BlurEffect`'s shader-cluster callback, the
//! extended blur's post-initialisation and its blur pass's setup.
//!
//! All three find the effect variant with a cluster cursor (a frame of
//! the original's, [`seek_variant`]), make a material
//! (`PEffectVariant_createMaterial`) and put it into a one-entry
//! material list ([`crate::h4_h2::resize_to_one`]) of a full-screen
//! mesh (`FullscreenMesh_createColoredTriangle_00a06120`).

use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::int_f32;
use crate::h4_h2::{
    CLASSES, CLUSTER_CREATE, CREATE_MATERIAL, CURSOR_SEEK, EFFECT_VARIANT_CLASS, FORMAT,
    SET_DIMENSIONS, TEXTURE_CLASS, TEXTURE_REGISTER, UPDATE_MAX_LEVEL, resize_to_one,
};

/// `FullscreenMesh_createColoredTriangle_00a06120(cluster)` (cdecl).
pub const FULLSCREEN_MESH: u32 = 0x00a0_6120;
/// `ShaderParams_FindByName(variant; name)` (thiscall, ret 4).
pub const FIND_PARAMETER: u32 = 0x0056_cbf0;
/// The blur pass's four parameters (`+0x48`..`+0x54`):
/// `"GaussianBlurBufferSize"`, `"OutputScale"`, `"UvScaleBias"`,
/// `"ColorBuffer"`.
pub const PASS_PARAMETERS: [u32; 4] = [0x00b4_94d8, 0x00b4_948c, 0x00b4_25e4, 0x00b4_25d8];
/// `BlurEffectEx_setParam48_006c45c0(this; x, y, z, w)` (thiscall, ret
/// 0x10).
pub const SET_PARAM48: u32 = 0x006c_45c0;
/// `BlurPass_initialize_006c40e0(this; a, cluster, scene, w, h)`.
pub const BLUR_PASS_INIT: u32 = 0x006c_40e0;
/// `BlurEffect_GetInstance()` (cdecl).
pub const BLUR_EFFECT: u32 = 0x006c_3580;
/// `Phyre_PFramework_PApplication_GetApplication()` (cdecl).
pub const GET_APPLICATION: u32 = 0x0062_3b90;
/// `PApplication_FixupClusters(clusters, n)` (cdecl).
pub const FIXUP_CLUSTERS: u32 = 0x0062_3580;
/// `RenderTarget_create_0064adf0(cluster; texture)` (thiscall, ret 4).
pub const CREATE_TARGET: u32 = 0x0064_adf0;
/// `RadialBlur_initialize(this; a, b, scene, w, h)` (thiscall, ret 0x14).
pub const RADIAL_BLUR_INIT: u32 = 0x0070_3230;
/// `RadialLine_initialize(this; a, b, scene, w, h, target)` (thiscall,
/// ret 0x18).
pub const RADIAL_LINE_INIT: u32 = 0x006f_7c10;
/// The names of `BlurEffect`'s three buffers (`+0`, `+4`, `+8`):
/// `"BlurEffect_frontBuffer"`, `"BlurEffect_backBuffer"`,
/// `"BlurEffect_accumulateBuffer"`.
pub const BUFFER_NAMES: [u32; 3] = [0x00b4_9440, 0x00b4_9458, 0x00b4_9470];
/// Bytes of the frames: a cluster cursor (0x20), then (`BlurEffect`'s
/// and the post-initialisation's) the material.
pub const PASS_FRAME: u32 = 0x20;
/// See [`PASS_FRAME`].
pub const EFFECT_FRAME: u32 = 0x24;

/// A cluster cursor at `frame` over `cluster`'s list (`+0x1c`) for the
/// effect-variant class, seeked (`PhyreSceneCursor_Seek3`). Returns the
/// variant (cursor `+0` plus `+0x10`).
pub fn seek_variant(h: &mut impl Host, frame: u32, cluster: u32) -> u32 {
    let list = cluster.wrapping_add(0x1c);
    let first: u32 = h.read(list);
    let head = if first == list { 0 } else { first };
    for k in 0..5u32 {
        h.write(frame.wrapping_add(4 * k), 0u32);
    }
    h.write(frame.wrapping_add(0x14), list);
    h.write(frame.wrapping_add(0x18), head);
    let class = h.rebase(EFFECT_VARIANT_CLASS);
    h.write(frame.wrapping_add(0x1c), class);
    h.call_sym(CURSOR_SEEK, Conv::Thiscall(frame), &[]);
    h.read::<u32>(frame)
        .wrapping_add(h.read(frame.wrapping_add(0x10)))
}

/// The first entry's address of the small array at `array` (count without
/// its top bit: 0 none, 1 inline at `+4`, more a block at `+4`).
fn first_entry(h: &impl Host, array: u32) -> Option<u32> {
    let n = h.read::<u32>(array) & 0x7fff_ffff;
    match n {
        0 => None,
        1 => Some(array.wrapping_add(4)),
        _ => Some(h.read(array.wrapping_add(4))),
    }
}

/// A material list in `scene` (`Phyre_PCluster_create(1, CLASSES[0],
/// 0)`) resized to one entry holding `material` (`None` when the list
/// came out empty), stored in `mesh + 0x10`.
fn material_list(h: &mut impl Host, list: u32, material: u32, mesh_at: u32) -> Option<()> {
    resize_to_one(h, list);
    let entry = first_entry(h, list)?;
    h.write(entry, material);
    let mesh: u32 = h.read(mesh_at);
    h.write(mesh.wrapping_add(0x10), list);
    Some(())
}

/// `BlurPass_initialize_006c40e0(this; a, cluster, scene, w, h)`
/// (thiscall, ret 0x14; `frame` its cursor at `ebp - 0x20`): `+0` a,
/// `+0x10` the shader cluster, `+0x14` the scene; the variant (`+0x1c`)
/// from the cluster, its material (`+0x20`), the four parameters
/// (`+0x48`..`+0x54`), the full-screen mesh (`+0x18`) with a one-entry
/// material list; `BlurEffectEx_setParam48_006c45c0(w, h, 0, 0)` (the
/// sizes as floats; also left in the cursor's `+0x10`..`+0x1c`); byte
/// `+0x24` 1 unless bit 7 of `+0x40`, the texture `+0x24` registered,
/// byte `+0x58` 0. Returns 0.
pub fn blur_pass_init(h: &mut impl Host, frame: u32, this: u32, a: [u32; 5]) -> u32 {
    let [first, cluster, scene, w, hh] = a;
    h.write(this, first);
    h.write(this.wrapping_add(0x14), scene);
    h.write(this.wrapping_add(0x10), cluster);
    let variant = seek_variant(h, frame, cluster);
    h.write(this.wrapping_add(0x1c), variant);
    let scene: u32 = h.read(this.wrapping_add(0x14));
    let material = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(variant), &[scene, 0, 0]);
    let variant: u32 = h.read(this.wrapping_add(0x1c));
    h.write(this.wrapping_add(0x20), material);
    for (k, &name) in PASS_PARAMETERS.iter().enumerate() {
        let name = h.rebase(name);
        let p = h.call_sym(FIND_PARAMETER, Conv::Thiscall(variant), &[name]);
        h.write(this.wrapping_add(0x48 + 4 * k as u32), p);
    }
    let scene: u32 = h.read(this.wrapping_add(0x14));
    let mesh = h.call_sym(FULLSCREEN_MESH, Conv::Cdecl, &[scene]);
    h.write(this.wrapping_add(0x18), mesh);
    let scene: u32 = h.read(this.wrapping_add(0x14));
    let class = h.rebase(CLASSES[0]);
    let list = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(scene), &[1, class, 0]);
    resize_to_one(h, list);
    if let Some(entry) = first_entry(h, list) {
        let material: u32 = h.read(this.wrapping_add(0x20));
        h.write(entry, material);
    }
    let x = int_f32(w.cast_signed());
    let y = int_f32(hh.cast_signed());
    let mesh: u32 = h.read(this.wrapping_add(0x18));
    h.write(mesh.wrapping_add(0x10), list);
    h.write(frame.wrapping_add(0x10), x);
    h.write(frame.wrapping_add(0x14), y);
    h.write(frame.wrapping_add(0x18), 0u32);
    h.write(frame.wrapping_add(0x1c), 0u32);
    h.call_sym(SET_PARAM48, Conv::Thiscall(this), &[x, y, 0, 0]);
    let texture = this.wrapping_add(0x24);
    if h.read::<u32>(this.wrapping_add(0x40)) >> 7 & 1 == 0 {
        h.write(texture, 1u8);
    }
    h.call_sym(TEXTURE_REGISTER, Conv::Thiscall(texture), &[]);
    h.write(this.wrapping_add(0x58), 0u8);
    0
}

/// `BlurEffectEx_postInitialize(this)` (`0x006c4b60`, thiscall; `frame`
/// from `ebp - 0x24`: the cursor, the material `+0x20`): once (`+0x10`
/// still 0), `BlurEffect`'s shader cluster, scene and buffers (its `+0`,
/// `+4`, `+8`, `+0x20` into `+0x10`, `+0x14`, `+0x18`, `+0x1c`); without
/// a cluster 0x13. Then the full-screen mesh (`+0x24`), the variant from
/// `+0x1c`, its material in a one-entry list (3 for an empty list), and
/// the blur pass (`[this]`) set up with `+4`, `+0x1c`, `+0x20`, `+0x30`,
/// `+0x34`. Returns 0 (also when done before).
pub fn blur_ex_post_initialize(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    if h.read::<u32>(this.wrapping_add(0x10)) != 0 {
        return 0;
    }
    for (to, from) in [(0x10u32, 0u32), (0x14, 4), (0x18, 8), (0x1c, 0x20)] {
        let blur = h.call_sym(BLUR_EFFECT, Conv::Cdecl, &[]);
        let v: u32 = h.read(blur.wrapping_add(from));
        h.write(this.wrapping_add(to), v);
    }
    if h.read::<u32>(this.wrapping_add(0x10)) == 0 {
        return 0x13;
    }
    let scene: u32 = h.read(this.wrapping_add(0x20));
    let mesh = h.call_sym(FULLSCREEN_MESH, Conv::Cdecl, &[scene]);
    h.write(this.wrapping_add(0x24), mesh);
    let cluster: u32 = h.read(this.wrapping_add(0x1c));
    let variant = seek_variant(h, frame, cluster);
    let scene: u32 = h.read(this.wrapping_add(0x20));
    let material = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(variant), &[scene, 0, 0]);
    h.write(frame.wrapping_add(0x20), material);
    let scene: u32 = h.read(this.wrapping_add(0x20));
    let class = h.rebase(CLASSES[0]);
    let list = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(scene), &[1, class, 0]);
    if material_list(h, list, material, this.wrapping_add(0x24)).is_none() {
        return 3;
    }
    let args = [0x04u32, 0x1c, 0x20, 0x30, 0x34].map(|off| h.read(this.wrapping_add(off)));
    let pass: u32 = h.read(this);
    h.call_sym(BLUR_PASS_INIT, Conv::Thiscall(pass), &args);
    0
}

/// `BlurEffect_doOnShaderClusterLoaded(this)` (`0x006c35d0`, thiscall;
/// `frame` from `ebp - 0x24`): the shader cluster `+0x20` fixed up, the
/// full-screen mesh (`+0x28`) in the scene `+0x24`; the variant (no hit:
/// 0x13), its material (none: 0xd) in a one-entry list (none: 0xd; empty:
/// 3); three `+0x34` x `+0x38` textures (flag bit 1 cleared, byte `+5`
/// 0) with render targets (`+0`, `+4`, `+8`, named; any missing: 0xd);
/// then `RadialBlur_initialize` (`+0x18`) and, when it returns 0,
/// `RadialLine_initialize` (`+0x1c`; the accumulate target's texture
/// unless its byte `+4`). Returns the last initialisation's result.
pub fn blur_effect_on_shaders(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
    let shaders = this.wrapping_add(0x20);
    h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[shaders, 1]);
    let scene: u32 = h.read(this.wrapping_add(0x24));
    let mesh = h.call_sym(FULLSCREEN_MESH, Conv::Cdecl, &[scene]);
    h.write(this.wrapping_add(0x28), mesh);
    let cluster: u32 = h.read(shaders);
    let variant = seek_variant(h, frame, cluster);
    if h.read::<u32>(frame.wrapping_add(0xc)) == 0 {
        return 0x13;
    }
    let scene: u32 = h.read(this.wrapping_add(0x24));
    let material = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(variant), &[scene, 0, 0]);
    h.write(frame.wrapping_add(0x20), material);
    if material == 0 {
        return 0xd;
    }
    let scene: u32 = h.read(this.wrapping_add(0x24));
    let class = h.rebase(CLASSES[0]);
    let list = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(scene), &[1, class, 0]);
    if list == 0 {
        return 0xd;
    }
    if material_list(h, list, material, this.wrapping_add(0x28)).is_none() {
        return 3;
    }
    for (k, &name) in BUFFER_NAMES.iter().enumerate() {
        let scene: u32 = h.read(this.wrapping_add(0x24));
        let class = h.rebase(TEXTURE_CLASS);
        let texture = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(scene), &[1, class, 0]);
        if texture == 0 {
            return 0xd;
        }
        let w: u32 = h.read(this.wrapping_add(0x34));
        let hh: u32 = h.read(this.wrapping_add(0x38));
        let format = h.rebase(FORMAT);
        h.call_sym(SET_DIMENSIONS, Conv::Thiscall(texture), &[w, hh, format, 0]);
        let flags: u32 = h.read(texture.wrapping_add(0x14));
        h.write(texture.wrapping_add(0x14), flags & 0xffff_fffd);
        h.write(texture.wrapping_add(5), 0u8);
        h.call_sym(UPDATE_MAX_LEVEL, Conv::Thiscall(texture), &[]);
        let scene: u32 = h.read(this.wrapping_add(0x24));
        let target = h.call_sym(CREATE_TARGET, Conv::Thiscall(scene), &[texture]);
        h.write(this.wrapping_add(4 * k as u32), target);
        if target == 0 {
            return 0xd;
        }
        let name = h.rebase(name);
        h.write(target.wrapping_add(0xc), name);
    }
    let args = [0x20u32, 0x14, 0x24, 0x34, 0x38].map(|off| h.read(this.wrapping_add(off)));
    let radial: u32 = h.read(this.wrapping_add(0x18));
    let r = h.call_sym(RADIAL_BLUR_INIT, Conv::Thiscall(radial), &args);
    if r != 0 {
        return r;
    }
    let accumulate: u32 = h.read(this.wrapping_add(8));
    let texture = if h.read::<u8>(accumulate.wrapping_add(4)) == 0 {
        h.read(accumulate)
    } else {
        0
    };
    let line: u32 = h.read(this.wrapping_add(0x1c));
    let [a, b, c, d, e] =
        [0x20u32, 0x14, 0x24, 0x34, 0x38].map(|off| h.read(this.wrapping_add(off)));
    h.call_sym(
        RADIAL_LINE_INIT,
        Conv::Thiscall(line),
        &[a, b, c, d, e, texture],
    )
}

// ---- the radial effects

/// `RadialBlur_setParam54_007036f0(this; x, y, z, w)` (thiscall, ret 0x10).
pub const RADIAL_SET_PARAM54: u32 = 0x0070_36f0;
/// `Phyre_PRendering_PSamplerStateBase_setMinFilter(sampler; filter)`
/// (thiscall, ret 4).
pub const SET_MIN_FILTER: u32 = 0x004c_07f0;
/// `"RadialBlur_capturedFrame"` (`0x00b4cf10`).
pub const RADIAL_BLUR_NAME: u32 = 0x00b4_cf10;
/// `"RadialLine_colorTarget"` (`0x00b4c150`).
pub const RADIAL_LINE_NAME: u32 = 0x00b4_c150;
/// `PSharray_setWord_0048c540(array; i, word)` (thiscall, ret 8).
pub const SET_WORD: u32 = crate::h4_h2::SET_WORD;
/// `PostProcessManager_GetInstance()` (cdecl).
pub const POST_PROCESS_MANAGER: u32 = crate::h4_h2::POST_PROCESS_MANAGER;
/// `PhyreSceneCursor_init_0064ce00(cursor; cluster)` (thiscall, ret 4):
/// a cursor over the cluster's effect variants, seeked.
pub const CURSOR_INIT: u32 = 0x0064_ce00;
/// `Phyre_PParameterBuffer_setTextureChecked_0056e670(buffer; parameter,
/// &desc)` (thiscall, ret 8).
pub const SET_TEXTURE: u32 = 0x0056_e670;
/// `RadialLine_setupMesh_006f8c50(this)` (thiscall).
pub const RADIAL_LINE_SETUP: u32 = 0x006f_8c50;
/// `PCluster_createInRoot_006f7a00(scene; n)` (thiscall, ret 4).
pub const CREATE_IN_ROOT: u32 = 0x006f_7a00;
/// `DynamicMesh_setStreams_00676680(mesh; owner, a, b)` (thiscall, ret
/// 0xc).
pub const SET_LINE_STREAMS: u32 = 0x0067_6680;
/// `RadialLineDynamicMeshInstance_create_00675d90(scene; &mesh)`
/// (thiscall, ret 4).
pub const LINE_INSTANCE: u32 = 0x0067_5d90;
/// `Phyre_PCluster_allocateMeshInstance_00566680(scene; mesh)` (thiscall,
/// ret 4).
pub const MESH_INSTANCE: u32 = crate::h4_h2::MESH_INSTANCE;

/// A texture in the scene at `this + scene_at` (`Phyre_PCluster_create(1,
/// TEXTURE_CLASS, 0)`) stored at `this + slot`, sized `w` x `h`
/// (`setDimensions`, whose non-zero result is returned), its flag bit 1
/// cleared and its max level updated, then a render target on it
/// (`0x0064adf0`) stored at `this + target_at` and named `name`.
/// `Err(code)` for a failure: 0xd for a missing object.
fn sized_target(
    h: &mut impl Host,
    this: u32,
    scene_at: u32,
    [slot, target_at]: [u32; 2],
    [w, hh]: [u32; 2],
    name: u32,
) -> Result<(), u32> {
    let scene: u32 = h.read(this.wrapping_add(scene_at));
    let class = h.rebase(TEXTURE_CLASS);
    let texture = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(scene), &[1, class, 0]);
    h.write(this.wrapping_add(slot), texture);
    if texture == 0 {
        return Err(0xd);
    }
    let format = h.rebase(FORMAT);
    let r = h.call_sym(SET_DIMENSIONS, Conv::Thiscall(texture), &[w, hh, format, 0]);
    if r != 0 {
        return Err(r);
    }
    let texture: u32 = h.read(this.wrapping_add(slot));
    let flags: u32 = h.read(texture.wrapping_add(0x14));
    h.write(texture.wrapping_add(0x14), flags & 0xffff_fffd);
    h.call_sym(UPDATE_MAX_LEVEL, Conv::Thiscall(texture), &[]);
    let texture: u32 = h.read(this.wrapping_add(slot));
    let scene: u32 = h.read(this.wrapping_add(scene_at));
    let target = h.call_sym(CREATE_TARGET, Conv::Thiscall(scene), &[texture]);
    h.write(this.wrapping_add(target_at), target);
    if target == 0 {
        return Err(0xd);
    }
    let name = h.rebase(name);
    h.write(target.wrapping_add(0xc), name);
    Ok(())
}

/// The four [`PASS_PARAMETERS`] of the variant at `this + variant_at`
/// looked up by name into `this + to`, `+4`, `+8`, `+0xc`.
fn find_parameters(h: &mut impl Host, this: u32, variant_at: u32, to: u32) {
    let variant: u32 = h.read(this.wrapping_add(variant_at));
    for (k, &name) in PASS_PARAMETERS.iter().enumerate() {
        let name = h.rebase(name);
        let p = h.call_sym(FIND_PARAMETER, Conv::Thiscall(variant), &[name]);
        h.write(this.wrapping_add(to + 4 * k as u32), p);
    }
}

/// `RadialBlur_initialize(this; cluster, a, scene, w, h)` (`0x00703230`,
/// thiscall, ret 0x14; `frame` its cursor at `ebp - 0x20`): `+0x1c` a,
/// `+0x20` the shader cluster (none: 5), `+0x24` the scene; the variant
/// (`+0x28`; no hit 0x13), its material (`+0x2c`; none 0xd), the four
/// parameters (`+0x54`..`+0x60`), the full-screen mesh (`+0x6c`) with a
/// one-entry material list; a `w` x `h` texture (`+0x64`) and its target
/// (`+0x68`, `"RadialBlur_capturedFrame"`); `setParam54(w, h, 0, 0)` (the
/// floats also left in the cursor's `+0x10`..); the sampler `+0x30`'s min
/// filter 1, its texture registered, byte `+0` 1. Returns 0, or the first
/// failure (a non-zero callee result included).
pub fn radial_blur_init(h: &mut impl Host, frame: u32, this: u32, a: [u32; 5]) -> u32 {
    let [cluster, b, scene, w, hh] = a;
    h.write(this.wrapping_add(0x1c), b);
    h.write(this.wrapping_add(0x20), cluster);
    h.write(this.wrapping_add(0x24), scene);
    if cluster == 0 {
        return 5;
    }
    let variant = seek_variant(h, frame, cluster);
    if h.read::<u32>(frame.wrapping_add(0xc)) == 0 {
        return 0x13;
    }
    h.write(this.wrapping_add(0x28), variant);
    let scene: u32 = h.read(this.wrapping_add(0x24));
    let material = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(variant), &[scene, 0, 0]);
    h.write(this.wrapping_add(0x2c), material);
    if material == 0 {
        return 0xd;
    }
    find_parameters(h, this, 0x28, 0x54);
    let scene: u32 = h.read(this.wrapping_add(0x24));
    let mesh = h.call_sym(FULLSCREEN_MESH, Conv::Cdecl, &[scene]);
    h.write(this.wrapping_add(0x6c), mesh);
    if mesh == 0 {
        return 0xd;
    }
    let scene: u32 = h.read(this.wrapping_add(0x24));
    let class = h.rebase(CLASSES[0]);
    let list = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(scene), &[1, class, 0]);
    if list == 0 {
        return 0xd;
    }
    resize_to_one(h, list);
    let material: u32 = h.read(this.wrapping_add(0x2c));
    h.call_sym(SET_WORD, Conv::Thiscall(list), &[0, material]);
    let mesh: u32 = h.read(this.wrapping_add(0x6c));
    h.write(mesh.wrapping_add(0x10), list);
    if let Err(r) = sized_target(h, this, 0x24, [0x64, 0x68], [w, hh], RADIAL_BLUR_NAME) {
        return r;
    }
    let x = int_f32(w.cast_signed());
    let y = int_f32(hh.cast_signed());
    h.write(frame.wrapping_add(0x10), x);
    h.write(frame.wrapping_add(0x14), y);
    h.write(frame.wrapping_add(0x18), 0u32);
    h.write(frame.wrapping_add(0x1c), 0u32);
    let r = h.call_sym(RADIAL_SET_PARAM54, Conv::Thiscall(this), &[x, y, 0, 0]);
    if r != 0 {
        return r;
    }
    let sampler = this.wrapping_add(0x30);
    h.call_sym(SET_MIN_FILTER, Conv::Thiscall(sampler), &[1]);
    h.call_sym(TEXTURE_REGISTER, Conv::Thiscall(sampler), &[]);
    h.write(this, 1u8);
    0
}

/// `RadialLine_initialize(this; cluster, a, scene, w, h, target)`
/// (`0x006f7c10`, thiscall, ret 0x18; `frame` its cursor at `ebp - 0x20`,
/// whose upper half later holds a texture descriptor): `+0` a, `+8` the
/// scene, `+4` the target, `+0x30`/`+0x34` the size, `+0xc` the shader
/// cluster. Without the post-process manager's shader flag 0x15. A
/// texture (`+0x10`) and its target (`+0x14`, `"RadialLine_colorTarget"`);
/// without a cluster or a variant hit 0x13; the variant (`+0x18`), its
/// material (`+0x1c`), the four parameters (`+0x68`..`+0x74`; with the
/// last, the texture bound through a `{0x42, 0, 0, texture}` descriptor);
/// a one-entry material list (none: 0xd), the mesh (`+0x20`), the line
/// mesh set up (`0x006f8c50`), a dynamic mesh (`+0x28`, streams `this`,
/// `4 (w + h)` twice), its instance (`+0x2c`), the mesh instance (`+0x24`,
/// its `+0x10` the list, `+0x18` the dynamic instance). Returns 0, or the
/// first failure.
pub fn radial_line_init(h: &mut impl Host, frame: u32, this: u32, a: [u32; 6]) -> u32 {
    let [cluster, first, scene, w, hh, target] = a;
    h.write(this, first);
    h.write(this.wrapping_add(8), scene);
    h.write(this.wrapping_add(4), target);
    h.write(this.wrapping_add(0x30), w);
    h.write(this.wrapping_add(0x34), hh);
    h.write(this.wrapping_add(0xc), cluster);
    let manager = h.call_sym(POST_PROCESS_MANAGER, Conv::Cdecl, &[]);
    if h.read::<u8>(manager.wrapping_add(0x1_23b4)) == 0 {
        return 0x15;
    }
    let size = [
        h.read(this.wrapping_add(0x30)),
        h.read(this.wrapping_add(0x34)),
    ];
    if let Err(r) = sized_target(h, this, 8, [0x10, 0x14], size, RADIAL_LINE_NAME) {
        return r;
    }
    let cluster: u32 = h.read(this.wrapping_add(0xc));
    if cluster == 0 {
        return 0x13;
    }
    h.call_sym(CURSOR_INIT, Conv::Thiscall(frame), &[cluster]);
    if h.read::<u32>(frame.wrapping_add(0xc)) == 0 {
        return 0x13;
    }
    let variant = h
        .read::<u32>(frame.wrapping_add(0x10))
        .wrapping_add(h.read(frame));
    h.write(this.wrapping_add(0x18), variant);
    let scene: u32 = h.read(this.wrapping_add(8));
    let material = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(variant), &[scene, 0, 0]);
    h.write(this.wrapping_add(0x1c), material);
    find_parameters(h, this, 0x18, 0x68);
    let parameter: u32 = h.read(this.wrapping_add(0x74));
    if parameter != 0 {
        let texture: u32 = h.read(this.wrapping_add(0x10));
        let desc = frame.wrapping_add(0x10);
        h.write(desc.wrapping_add(0xc), texture);
        h.write(desc, 0x42u32);
        h.write(desc.wrapping_add(8), 0u32);
        h.write(desc.wrapping_add(4), 0u32);
        let buffer: u32 = h.read(material.wrapping_add(4));
        h.call_sym(SET_TEXTURE, Conv::Thiscall(buffer), &[parameter, desc]);
    }
    let scene: u32 = h.read(this.wrapping_add(8));
    let class = h.rebase(CLASSES[0]);
    let list = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(scene), &[1, class, 0]);
    if list == 0 {
        return 0xd;
    }
    resize_to_one(h, list);
    let material: u32 = h.read(this.wrapping_add(0x1c));
    h.call_sym(SET_WORD, Conv::Thiscall(list), &[0, material]);
    let scene: u32 = h.read(this.wrapping_add(8));
    let class = h.rebase(CLASSES[1]);
    let mesh = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(scene), &[1, class, 0]);
    h.write(this.wrapping_add(0x20), mesh);
    h.call_sym(RADIAL_LINE_SETUP, Conv::Thiscall(this), &[]);
    let scene: u32 = h.read(this.wrapping_add(8));
    let dynamic = h.call_sym(CREATE_IN_ROOT, Conv::Thiscall(scene), &[1]);
    let n = h
        .read::<u32>(this.wrapping_add(0x34))
        .wrapping_add(h.read(this.wrapping_add(0x30)))
        << 2;
    let slot = this.wrapping_add(0x28);
    h.write(slot, dynamic);
    h.call_sym(SET_LINE_STREAMS, Conv::Thiscall(dynamic), &[this, n, n]);
    let scene: u32 = h.read(this.wrapping_add(8));
    let instance = h.call_sym(LINE_INSTANCE, Conv::Thiscall(scene), &[slot]);
    h.write(this.wrapping_add(0x2c), instance);
    let scene: u32 = h.read(this.wrapping_add(8));
    let mesh: u32 = h.read(this.wrapping_add(0x20));
    let mesh_instance = h.call_sym(MESH_INSTANCE, Conv::Thiscall(scene), &[mesh]);
    h.write(this.wrapping_add(0x24), mesh_instance);
    h.write(mesh_instance.wrapping_add(0x10), list);
    let mesh_instance: u32 = h.read(this.wrapping_add(0x24));
    let instance: u32 = h.read(this.wrapping_add(0x2c));
    h.write(mesh_instance.wrapping_add(0x18), instance);
    0
}
