//! Continuation round 6 of list `h4-glue-other`, batch 2: the loading
//! screen's constructor, the broken-screen effect's render target, the
//! boot text's creation, and the distortion effect's setup and shader
//! callback.
//!
//! Engine objects stay original: clusters (`Phyre_PCluster_create`,
//! `allocateInstanceList`), textures and the async loader are called
//! with the `ecx` the original leaves. The SEH frames are not kept.

use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::pass;

/// `Phyre_PhyreMalloc(bytes)` (`0x0042fb40`, cdecl).
pub const PHYRE_MALLOC1: u32 = 0x0042_fb40;
/// `Phyre_PhyreFree(block)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `Phyre_PCluster_operator_new(this)` (`PCluster`'s constructor,
/// thiscall).
pub const PCLUSTER_CTOR: u32 = crate::h4_h1::PCLUSTER_CTOR;
/// `operator_new(bytes)` (cdecl).
pub const OPERATOR_NEW: u32 = crate::h4_h1::OPERATOR_NEW;

/// A new `PCluster` (`PhyreMalloc(0x50)`, its constructor, the list head
/// `+0x48` pointing at itself), or 0 without memory.
fn new_cluster(h: &mut impl Host) -> u32 {
    let block = h.call_sym(PHYRE_MALLOC1, Conv::Cdecl, &[0x50]);
    if block == 0 {
        return 0;
    }
    h.call_sym(PCLUSTER_CTOR, Conv::Thiscall(block), &[]);
    let head = block.wrapping_add(0x48);
    h.write(head, head);
    h.write(head.wrapping_add(4), head);
    block
}

// ---- the loading screen

/// `LoadingScreen_LoadingScreen(this)` (`0x006e45e0`, thiscall): the
/// half-word `+0` 0, the floats `+4`/`+8` 0.0, the words `+0xc`, `+0x10`,
/// `+0x18`..`+0x30`, `+0x38`..`+0x40` 0 (`+0x14` is left), a new cluster
/// at `+0x34` (0 without memory). Returns `this`.
pub fn loading_screen_ctor(h: &mut impl Host, this: u32) -> u32 {
    h.write(this, 0u16);
    for off in [4u32, 8, 0xc, 0x10, 0x18, 0x1c, 0x20, 0x24, 0x28, 0x2c, 0x30] {
        h.write(this.wrapping_add(off), 0u32);
    }
    for off in [0x38u32, 0x3c, 0x40] {
        h.write(this.wrapping_add(off), 0u32);
    }
    let cluster = new_cluster(h);
    h.write(this.wrapping_add(0x34), cluster);
    this
}

// ---- the broken-screen effect

/// `Phyre_PRendering_PRenderer_syncRender(renderer)` (thiscall).
pub const SYNC_RENDER: u32 = 0x005b_c3f0;
/// `RenderTarget_release_00440ea0(cluster)` (thiscall).
pub const CLUSTER_CLEAR: u32 = 0x0044_0ea0;
/// `Phyre_PCluster_create(cluster; n, class, 0)` (thiscall, ret 0xc).
pub const CLUSTER_CREATE: u32 = 0x0044_1010;
/// `Phyre_PCluster_allocateInstanceList(cluster; n, class, 0)`
/// (thiscall, ret 0xc).
pub const ALLOCATE_INSTANCE_LIST: u32 = 0x0044_0b70;
/// `Phyre_PRendering_PTexture2DBase_setDimensions(texture; w, h, format,
/// 0)` (thiscall, ret 0x10).
pub const SET_DIMENSIONS: u32 = 0x004c_40c0;
/// `Phyre_PRendering_PTexture2DBase_updateMaxLevel(texture)` (thiscall).
pub const UPDATE_MAX_LEVEL: u32 = 0x004c_4250;
/// `Phyre_PRendering_Ctor_00583a60(target; texture)` (thiscall, ret 4).
pub const RENDER_TARGET_CTOR: u32 = 0x0058_3a60;
/// The texture class (`0x00ca37d0`).
pub const TEXTURE_CLASS: u32 = 0x00ca_37d0;
/// The pixel format (`0x00ca310c`).
pub const FORMAT: u32 = 0x00ca_310c;
/// The render-target class (`0x00ca4200`).
pub const RENDER_TARGET_CLASS: u32 = 0x00ca_4200;
/// `"BrokenScreen_capturedFrame"` (`0x00b49ae0`).
pub const CAPTURED_FRAME_NAME: u32 = 0x00b4_9ae0;

/// `BrokenScreenEffect_createRenderTarget(this)` (`0x006d5050`,
/// thiscall): with one already (byte `+0x74`), the renderer synced
/// (`+0x2c`), the cluster `+0x54` cleared, the byte and the target `+0`
/// 0. Then a texture in the cluster (`Phyre_PCluster_create(1,
/// TEXTURE_CLASS, 0)`; none: 0xd) sized `+0x6c` x `+0x70` in `FORMAT`,
/// its flag bit 1 (`+0x14`) cleared, byte `+5` 2, its max level updated;
/// a render target (`allocateInstanceList(1, RENDER_TARGET_CLASS, 0)`,
/// its `+0x34`) built on it, stored at `+0`, named
/// `"BrokenScreen_capturedFrame"`, byte `+0x74` 1. Returns 0, or 0xd
/// without a texture or a target.
pub fn broken_screen_create_render_target(h: &mut impl Host, this: u32) -> u32 {
    if h.read::<u8>(this.wrapping_add(0x74)) != 0 {
        let renderer: u32 = h.read(this.wrapping_add(0x2c));
        h.call_sym(SYNC_RENDER, Conv::Thiscall(renderer), &[]);
        let cluster: u32 = h.read(this.wrapping_add(0x54));
        h.call_sym(CLUSTER_CLEAR, Conv::Thiscall(cluster), &[]);
        h.write(this.wrapping_add(0x74), 0u8);
        h.write(this, 0u32);
    }
    let cluster: u32 = h.read(this.wrapping_add(0x54));
    let class = h.rebase(TEXTURE_CLASS);
    let texture = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, class, 0]);
    if texture == 0 {
        return 0xd;
    }
    let w: u32 = h.read(this.wrapping_add(0x6c));
    let hh: u32 = h.read(this.wrapping_add(0x70));
    let format = h.rebase(FORMAT);
    h.call_sym(SET_DIMENSIONS, Conv::Thiscall(texture), &[w, hh, format, 0]);
    let flags: u32 = h.read(texture.wrapping_add(0x14));
    h.write(texture.wrapping_add(0x14), flags & 0xffff_fffd);
    h.write(texture.wrapping_add(5), 2u8);
    h.call_sym(UPDATE_MAX_LEVEL, Conv::Thiscall(texture), &[]);
    let cluster: u32 = h.read(this.wrapping_add(0x54));
    let class = h.rebase(RENDER_TARGET_CLASS);
    let list = h.call_sym(
        ALLOCATE_INSTANCE_LIST,
        Conv::Thiscall(cluster),
        &[1, class, 0],
    );
    let target = if list == 0 {
        0
    } else {
        h.read(list.wrapping_add(0x34))
    };
    if target != 0 {
        h.call_sym(RENDER_TARGET_CTOR, Conv::Thiscall(target), &[texture]);
    }
    h.write(this, target);
    if target == 0 {
        return 0xd;
    }
    let name = h.rebase(CAPTURED_FRAME_NAME);
    h.write(target.wrapping_add(0xc), name);
    h.write(this.wrapping_add(0x74), 1u8);
    0
}

// ---- the boot text

/// `TextUtil_TextUtil(this)` (thiscall).
pub const TEXT_UTIL_CTOR: u32 = 0x006f_5b50;
/// `TextUtil_init(this)` (thiscall).
pub const TEXT_UTIL_INIT: u32 = 0x006f_6370;
/// `TextUtil_setTitleText(this; text)` (thiscall, ret 4).
pub const SET_TITLE_TEXT: u32 = 0x006f_6930;
/// `TextUtil_setPosition_006f6910(this; x, y)` (thiscall, ret 8).
pub const SET_POSITION: u32 = 0x006f_6910;
/// `TextUtil_setEnabled(this; on)` (thiscall, ret 4).
pub const SET_ENABLED: u32 = 0x006f_68f0;
/// `sprintf_s`'s import slot (cdecl).
pub const SPRINTF_S: ffx_bridge::Global<u32> = crate::h4_d1::SPRINTF_S;
/// The five strings the title's format gets (`0x00b7c998`).
pub const TITLE_ARG: u32 = 0x00b7_c998;
/// The position's x (`0x00b92268`, f32 2.0) and y (`0x00b4a64c`, f32 17.0).
pub const POSITION: [u32; 2] = [0x00b9_2268, 0x00b4_a64c];

/// `BootText_create_006ddc80(this)` (thiscall): a `TextUtil`
/// (`operator_new(0x90)`, its constructor; 0 without memory) at `+0`;
/// unless 0, `TextUtil_init`; when that returns 0, the title
/// `sprintf_s(+6, 0x80, +0xd6, s, s, s, s, s)` set, the position set to
/// the two image floats, and the text enabled; else disabled. Returns
/// `setEnabled`'s `eax`, 0 without a `TextUtil`.
pub fn boot_text_create(h: &mut impl Host, this: u32) -> u32 {
    let block = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x90]);
    let text = if block == 0 {
        0
    } else {
        h.call_sym(TEXT_UTIL_CTOR, Conv::Thiscall(block), &[])
    };
    h.write(this, text);
    if text == 0 {
        return 0;
    }
    let on = if h.call_sym(TEXT_UTIL_INIT, Conv::Thiscall(text), &[]) == 0 {
        let title = this.wrapping_add(6);
        let s = h.rebase(TITLE_ARG);
        let f = SPRINTF_S.get(h);
        h.call(
            f,
            Conv::Cdecl,
            &[title, 0x80, this.wrapping_add(0xd6), s, s, s, s, s],
        );
        let text: u32 = h.read(this);
        h.call_sym(SET_TITLE_TEXT, Conv::Thiscall(text), &[title]);
        let x = pass(h.read(h.rebase(POSITION[0])));
        let y = pass(h.read(h.rebase(POSITION[1])));
        let text: u32 = h.read(this);
        h.call_sym(SET_POSITION, Conv::Thiscall(text), &[x, y]);
        1
    } else {
        0
    };
    let text: u32 = h.read(this);
    h.call_sym(SET_ENABLED, Conv::Thiscall(text), &[on])
}

// ---- the distortion effect

/// `PTextureD3D11_register_0058cf80(this)` (thiscall).
pub const TEXTURE_REGISTER: u32 = 0x0058_cf80;
/// `PostProcessManager_GetInstance()` (cdecl).
pub const POST_PROCESS_MANAGER: u32 = 0x0069_2be0;
/// `fiosUnifyFilename(name, out, n)` (cdecl).
pub const FIOS_UNIFY: u32 = 0x0067_9820;
/// `AsyncLoadManager_sGetInstance()` (cdecl).
pub const ASYNC_LOADER: u32 = 0x0069_1b40;
/// `AsyncLoadManager_loadCluster(loader, 0, path, callback, 1, user_lo,
/// user_hi)` (cdecl; the original pushes the six words before it gets the
/// loader, so they are the call's last six).
pub const LOAD_CLUSTER: u32 = 0x0069_1660;
/// `"/FFX_Data/GameData/PS3Data/Shaders/GCM/PhyreDistortionShader.cgfx.phyre"`
/// (`0x00b49308`).
pub const DISTORTION_SHADER: u32 = 0x00b4_9308;
/// `Distortion_onShaderClusterLoaded_006c28b0`, the load's callback.
pub const DISTORTION_LOADED: u32 = 0x006c_28b0;
/// Bytes of [`distortion_initialize`]'s frame: the unified path.
pub const DISTORTION_FRAME: u32 = 0x100;

/// `Distortion_initialize_006c2710(this; a, b, c, d, e)` (thiscall, ret
/// 0x14; `frame` its 0x100-byte path buffer at `ebp - 0x110`): `+0` a,
/// `+4` d, `+8` e, `+0xc` b, `+0x10` c; three new clusters (`+0x14`,
/// `+0x18`, `+0x1c`); unless bit 7 of `+0x60`, bytes `+0x44`/`+0x45` 1;
/// the texture `+0x44` registered. Without the post-process manager's
/// shader flag (byte `+0x123b4`) 0x15; else the shader path unified into
/// the frame and loaded (`AsyncLoadManager_loadCluster`, callback
/// `0x006c28b0`, `this` as a 64-bit user word), 0.
pub fn distortion_initialize(h: &mut impl Host, frame: u32, this: u32, a: [u32; 5]) -> u32 {
    for (off, v) in [
        (0u32, a[0]),
        (4, a[3]),
        (8, a[4]),
        (0xc, a[1]),
        (0x10, a[2]),
    ] {
        h.write(this.wrapping_add(off), v);
    }
    for off in [0x14u32, 0x18, 0x1c] {
        let cluster = new_cluster(h);
        h.write(this.wrapping_add(off), cluster);
    }
    let texture = this.wrapping_add(0x44);
    if h.read::<u32>(texture.wrapping_add(0x1c)) >> 7 & 1 == 0 {
        h.write(texture, 1u8);
        h.write(texture.wrapping_add(1), 1u8);
    }
    h.call_sym(TEXTURE_REGISTER, Conv::Thiscall(texture), &[]);
    let manager = h.call_sym(POST_PROCESS_MANAGER, Conv::Cdecl, &[]);
    if h.read::<u8>(manager.wrapping_add(0x1_23b4)) == 0 {
        return 0x15;
    }
    let path = h.rebase(DISTORTION_SHADER);
    h.call_sym(FIOS_UNIFY, Conv::Cdecl, &[path, frame, 0x100]);
    let loader = h.call_sym(ASYNC_LOADER, Conv::Cdecl, &[]);
    let callback = h.rebase(DISTORTION_LOADED);
    let hi = (this.cast_signed() >> 31).cast_unsigned();
    h.call_sym(
        LOAD_CLUSTER,
        Conv::Cdecl,
        &[loader, 0, frame, callback, 1, this, hi],
    );
    0
}

/// `PhyreSceneCursor_Seek3_0065ec40(cursor)` (thiscall).
pub const CURSOR_SEEK: u32 = 0x0065_ec40;
/// The class the cursor looks for (`0x00ca5ff0`).
pub const EFFECT_VARIANT_CLASS: u32 = 0x00ca_5ff0;
/// `Phyre_PRendering_PEffectVariant_createMaterial(variant; cluster, 0,
/// 0)` (thiscall, ret 0xc).
pub const CREATE_MATERIAL: u32 = 0x004e_6a40;
/// `Distortion_createRenderTarget(this)` (thiscall).
pub const DISTORTION_CREATE_RT: u32 = 0x006c_2350;
/// `Distortion_setupDistortionGrid(this)` (thiscall).
pub const SETUP_GRID: u32 = 0x006c_2ff0;
/// `PSharray_setWord_0048c540(array; i, word)` (thiscall, ret 8).
pub const SET_WORD: u32 = 0x0048_c540;
/// `DynamicMesh_setStreams_00674740(mesh; a, b)` (thiscall, ret 8).
pub const SET_STREAMS: u32 = 0x0067_4740;
/// `DistortionGridDynamicMeshInstance_create_00673e20(cluster; &mesh)`
/// (thiscall, ret 4).
pub const GRID_INSTANCE: u32 = 0x0067_3e20;
/// `Phyre_PCluster_allocateMeshInstance_00566680(cluster; mesh)`
/// (thiscall, ret 4).
pub const MESH_INSTANCE: u32 = 0x0056_6680;
/// The classes of the three objects created: the material list
/// (`0x00c998e8`), the mesh (`0x00c99c80`), the dynamic mesh
/// (`0x00ccbbe0`).
pub const CLASSES: [u32; 3] = [0x00c9_98e8, 0x00c9_9c80, 0x00cc_bbe0];
/// Bytes of [`distortion_on_shaders`]'s frame: the cluster cursor.
pub const CURSOR_FRAME: u32 = 0x20;

/// `Distortion_onShadersLoaded_006c2520(this)` (thiscall; `frame` its
/// cursor at `ebp - 0x20`; tail-called by `0x006c28b0`): a cursor over the
/// shader cluster's (`+0x20`) list (`+0x1c`) for the effect-variant
/// class, seeked; without a hit (cursor `+0xc`) or a variant (cursor `+0`
/// plus `+0x10`, stored at `+0x2c`) 0x13. Then the material
/// (`createMaterial(+0x14, 0, 0)`, `+0x30`), the render target, a
/// material list (`Phyre_PCluster_create` in `+0x14`; none: 0xd) resized
/// to one entry inline and set to the material, the mesh (`+0x34`), the
/// grid, the dynamic mesh (`+0x3c`, the object 4 bytes before the
/// created one; streams 0x51, 0x180), its instance (`+0x40`), the mesh
/// instance (`+0x38`, its `+0x10` the material list and `+0x18` the
/// dynamic instance). Returns 0.
pub fn distortion_on_shaders(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let cluster: u32 = h.read(this.wrapping_add(0x20));
    let list = cluster.wrapping_add(0x1c);
    let first: u32 = h.read(list);
    let head = if first == list { 0 } else { first };
    for k in 0..6u32 {
        h.write(frame.wrapping_add(4 * k), 0u32);
    }
    h.write(frame.wrapping_add(0x18), head);
    h.write(frame.wrapping_add(0x14), list);
    let class = h.rebase(EFFECT_VARIANT_CLASS);
    h.write(frame.wrapping_add(0x1c), class);
    h.call_sym(CURSOR_SEEK, Conv::Thiscall(frame), &[]);
    if h.read::<u32>(frame.wrapping_add(0xc)) == 0 {
        return 0x13;
    }
    let variant = h
        .read::<u32>(frame)
        .wrapping_add(h.read(frame.wrapping_add(0x10)));
    h.write(this.wrapping_add(0x2c), variant);
    if variant == 0 {
        return 0x13;
    }
    let scene: u32 = h.read(this.wrapping_add(0x14));
    let material = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(variant), &[scene, 0, 0]);
    h.write(this.wrapping_add(0x30), material);
    h.call_sym(DISTORTION_CREATE_RT, Conv::Thiscall(this), &[]);
    let scene: u32 = h.read(this.wrapping_add(0x14));
    let class = h.rebase(CLASSES[0]);
    let materials = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(scene), &[1, class, 0]);
    if materials == 0 {
        return 0xd;
    }
    resize_to_one(h, materials);
    let material: u32 = h.read(this.wrapping_add(0x30));
    h.call_sym(SET_WORD, Conv::Thiscall(materials), &[0, material]);
    let scene: u32 = h.read(this.wrapping_add(0x14));
    let class = h.rebase(CLASSES[1]);
    let mesh = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(scene), &[1, class, 0]);
    h.write(this.wrapping_add(0x34), mesh);
    h.call_sym(SETUP_GRID, Conv::Thiscall(this), &[]);
    let scene: u32 = h.read(this.wrapping_add(0x14));
    let class = h.rebase(CLASSES[2]);
    let created = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(scene), &[1, class, 0]);
    let dynamic = if created == 0 {
        0
    } else {
        created.wrapping_sub(4)
    };
    let slot = this.wrapping_add(0x3c);
    h.write(slot, dynamic);
    h.call_sym(SET_STREAMS, Conv::Thiscall(dynamic), &[0x51, 0x180]);
    let scene: u32 = h.read(this.wrapping_add(0x14));
    let instance = h.call_sym(GRID_INSTANCE, Conv::Thiscall(scene), &[slot]);
    h.write(this.wrapping_add(0x40), instance);
    let scene: u32 = h.read(this.wrapping_add(0x14));
    let mesh: u32 = h.read(this.wrapping_add(0x34));
    let mesh_instance = h.call_sym(MESH_INSTANCE, Conv::Thiscall(scene), &[mesh]);
    h.write(this.wrapping_add(0x38), mesh_instance);
    h.write(mesh_instance.wrapping_add(0x10), materials);
    let mesh_instance: u32 = h.read(this.wrapping_add(0x38));
    let instance: u32 = h.read(this.wrapping_add(0x40));
    h.write(mesh_instance.wrapping_add(0x18), instance);
    0
}

/// The small array at `array` (a count, top bit: not owned; one entry
/// inline at `+4`, more in a block there) resized to one entry, inline:
/// unless the count is 1, the entry cleared, the old block freed when it
/// is not the inline slot and owned (a count of 0 frees null), the count
/// 1.
fn resize_to_one(h: &mut impl Host, array: u32) {
    let count: u32 = h.read(array);
    if count == 1 {
        return;
    }
    let n = count & 0x7fff_ffff;
    let inline = array.wrapping_add(4);
    let data = if n > 1 {
        h.read(inline)
    } else if n != 0 {
        inline
    } else {
        0
    };
    h.write(inline, 0u32);
    if data != inline && h.read::<u32>(array) >> 31 == 0 {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
    }
    h.write(array, 1u32);
}
