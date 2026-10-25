//! The render-side glue, fourth batch (M4 port round 2, list
//! `h3-glue-render`): the scene's YIQ reset, the abmap and broken-screen
//! forwarders, the VFX render table's loops, material pass remapping, the
//! scene's draw-event and pipeline arrays, and small `PArray`/tree helpers.
//!
//! Results are what the originals leave in `eax` on every path, where
//! that is defined by the function; functions that leave the caller's
//! `eax` on some path return nothing (no caller reads it, check-clobber).

use ffx_bridge::Global;

pub use crate::glue::SCENE;
pub use crate::glue2::{GRAPHIC_ABMAP, IGGY_MENU_MANAGER};

/// `g_VFXRenderDataTable`.
pub const VFX_RENDER_DATA_TABLE: Global<u32> = Global::new(0x00cc_ca6c);
/// `g_UseOptimizedVFXShaderArg`: the second argument
/// `graphicUseOptimizedVFXShaderByVFXInstance` passes on, set to 1 after
/// each call.
pub const USE_OPTIMIZED_VFX_SHADER_ARG: Global<u32> = Global::new(0x00c3_a4dc);

/// `PhyreScene_resetYIQ` (thiscall, no arguments).
pub const SCENE_RESET_YIQ: u32 = 0x0066_bf40;
/// `PhyreScene_updateMapYIQ` (thiscall, no arguments).
pub const SCENE_UPDATE_MAP_YIQ: u32 = 0x0067_1da0;
/// `Phyre_PRendering_PMeshInstance_updateBoundsFromSkeleton` (thiscall).
pub const MESH_UPDATE_BOUNDS_FROM_SKELETON: u32 = 0x004f_96a0;
/// `PhyreScene_setAbmapCamera(this, 5 floats)`.
pub const SCENE_SET_ABMAP_CAMERA: u32 = 0x0066_ca60;
/// Deletes `g_GraphicAbmap` and clears it (code at `0x00684ae0`, cdecl,
/// inside the dump's `FUN_00684a60`).
pub const ABMAP_DELETE: u32 = 0x0068_4ae0;
/// The abmap's update (`0x00685840`, thiscall).
pub const ABMAP_UPDATE: u32 = 0x0068_5840;
/// The post-process manager's per-shader-bin step (`0x00693540`,
/// thiscall, `ret 4`).
pub const POST_SHADER_BIN_STEP: u32 = 0x0069_3540;
/// `PostProcessManager_onAllShaderBinFilesLoaded` (thiscall).
pub const POST_ON_ALL_SHADER_BINS: u32 = 0x0069_3d70;
/// The broken screen's polygon lookup (`0x006d5190`, thiscall, 2 words).
pub const BROKEN_POLYGON: u32 = crate::glue2::BROKEN_POLYGON;
/// The polygon's update (`0x006d49f0`, thiscall, 9 words).
pub const BROKEN_POLYGON_UPDATE: u32 = 0x006d_49f0;
/// The scene's mesh visibility test (`0x00663cc0`, thiscall, `ret 4`).
pub const SCENE_MESH_VISIBLE_TEST: u32 = 0x0066_3cc0;
/// `PhyreScene_isMeshVisible_006603c0` (thiscall, `ret 4`).
pub const SCENE_IS_MESH_VISIBLE: u32 = 0x0066_03c0;
/// `ClassVFXRenderDataTable_GetDynamicGeometryByInstance(this, lo, hi)`.
pub const VFX_GET_DYN_GEO_BY_INSTANCE: u32 = 0x0069_f620;
/// `ClassVFXRenderDataTable_GetVFXDataByInstance(this, lo, hi)`.
pub const VFX_GET_DATA_BY_INSTANCE: u32 = 0x0069_f6e0;
/// `graphicUseOptimizedVFXShader(geometry, on)` (cdecl).
pub const USE_OPTIMIZED_VFX_SHADER: u32 = 0x0064_4ac0;
/// `Phyre_PWorld_removeCluster(this, cluster)`.
pub const WORLD_REMOVE_CLUSTER: u32 = 0x0044_3b70;
/// `Phyre_PWorld_addCluster(this, cluster)`.
pub const WORLD_ADD_CLUSTER: u32 = 0x0044_3250;
/// `Phyre_PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `MagicFile_GetMagicId()`.
pub const MAGIC_ID: u32 = crate::glue3::MAGIC_ID;
/// `Phyre_PRendering_PMaterial_setSceneRenderPassRemapping(this, from,
/// to)`.
pub const MATERIAL_SET_PASS_REMAPPING: u32 = 0x004d_8960;
/// The scene render pass the remapping starts from (an address in
/// `.data`).
pub const PASS_SCENE: u32 = 0x00c1_69d4;
/// The optimized-transparency pass.
pub const PASS_OPTIMIZED_TRANS: u32 = 0x00c1_6b84;
/// The forced-transparency pass.
pub const PASS_FORCE_TRANSPARENT: u32 = 0x00c1_6b14;
/// The pass that keeps a material as it is under forced transparency.
pub const PASS_KEEP: u32 = 0x00c1_6b44;
/// `Distortion_GetInstance`.
pub const DISTORTION_INSTANCE: u32 = crate::glue2::DISTORTION_INSTANCE;
/// `Distortion_updateDistortionVertex(this, a, b, 6 floats)`.
pub const DISTORTION_UPDATE_VERTEX: u32 = 0x006c_3240;
/// `PostProcessManager_renderFXAA(this, a, b)`.
pub const POST_RENDER_FXAA: u32 = 0x0069_5740;
/// `PostProcessManager_renderSMAA(this, a)`.
pub const POST_RENDER_SMAA: u32 = 0x0069_6af0;
/// `PArray_Data_004d45a0(this)`.
pub const PARRAY_DATA: u32 = 0x004d_45a0;
/// `PhyreScene_UseVFXOptimizedTrans(material, on)` (cdecl).
pub const SCENE_USE_VFX_OPTIMIZED_TRANS: u32 = 0x0065_4b20;
/// `Array_Reserve_00667f00(this, n)`.
pub const ARRAY_RESERVE_667F00: u32 = 0x0066_7f00;
/// `Array_Reserve_00668080(this, n)`.
pub const ARRAY_RESERVE_668080: u32 = 0x0066_8080;
/// `Array_Reserve_00667f80(this, n)`.
pub const ARRAY_RESERVE_667F80: u32 = 0x0066_7f80;
/// `Array_Reserve_00668280(this, n)` (16-byte entries).
pub const ARRAY_RESERVE_668280: u32 = 0x0066_8280;
/// `IggyMenu_dtor_IggyMenu(this)`.
pub const IGGY_MENU_DTOR: u32 = crate::glue2::IGGY_MENU_DTOR;
/// `operator_delete(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = crate::glue2::OPERATOR_DELETE;
/// `IggyPlayerOpenScaled_006e9250(path, x, y, w, h)` (cdecl).
pub const IGGY_PLAYER_OPEN_SCALED: u32 = 0x006e_9250;
/// The auto-save prompt's Iggy file name.
pub const AUTO_SAVE_PROMPT_PATH: u32 = 0x00b4_3cb0;
/// `PhyreScene_isVFXTextureLoaded(this, 7 words)`.
pub const SCENE_IS_VFX_TEXTURE_LOADED: u32 = 0x0066_04d0;
/// `ClusterManager_GetInstance`.
pub const CLUSTER_INSTANCE: u32 = crate::glue2::CLUSTER_INSTANCE;
/// `ClusterManager_getPClusterByName(this, name)`.
pub const CLUSTER_BY_NAME: u32 = 0x0069_b4b0;
/// `VFXDrawEventArray_Push(this, event)`.
pub const VFX_DRAW_EVENT_PUSH: u32 = 0x0065_6350;
/// `Phyre_PRedBlackTree_successor(this, node)`.
pub const TREE_SUCCESSOR: u32 = 0x0057_5160;
/// The float the broken screen's second fill writes (`.rdata`).
pub const BROKEN_FILL_VALUE: Global<u32> = Global::new(0x00b4_12c4);
/// `InterlockedCompareExchange`'s import slot (stdcall).
pub const INTERLOCKED_COMPARE_EXCHANGE: u32 = 0x00b0_c12c;
/// `__iob_func`'s import slot (cdecl).
pub const IOB_FUNC: u32 = 0x00b0_c350;
/// `fprintf`'s import slot (cdecl).
pub const FPRINTF: u32 = 0x00b0_c38c;
/// `"CAS failed at RenderDelegationHelper::setDataToFreeList - index:%d"`.
pub const CAS_FAILED_FORMAT: u32 = 0x00b4_83d8;

/// The count of a `PArray`-style header (`& 0x7fffffff`).
const COUNT_MASK: u32 = 0x7fff_ffff;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::glue::bridge::{instance, on_global};
    use ffx_bridge::x87::round_trip;
    use ffx_graphic::host::{Conv, Host};

    /// Floats (bits) through `fld`/`fstp`.
    fn rt<const N: usize>(f: [u32; N]) -> [u32; N] {
        f.map(|x| round_trip(f32::from_bits(x)).to_bits())
    }

    /// Calls the import whose slot is at `slot`; its `eax`.
    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let at = h.rebase(slot);
        let f: u32 = h.read(at);
        h.call(f, conv, args)
    }

    /// `graphicResetYIQ()` `0x00642cc0`: `g_PhyreScene->resetYIQ()`.
    pub fn graphic_reset_yiq(h: &mut impl Host) -> u32 {
        on_global(h, SCENE, SCENE_RESET_YIQ, &[])
    }

    /// `PhyreScene_resetYIQ(this)` `0x0066bf40`: the YIQ matrix rows
    /// (`+0xfc8c`..`+0xfcc8`) to 1,1,1,1 / 0,0,0,0 / 1,1,1,1 / 0,0,0,0,
    /// byte `+0xfc88` = 0, then `updateMapYIQ` (its `eax`).
    pub fn scene_reset_yiq(h: &mut impl Host, this: u32) -> u32 {
        for row in 0..4u32 {
            let v: u32 = if row % 2 == 0 { 0x3f80_0000 } else { 0 };
            for k in 0..4u32 {
                h.write(this + 0xfc8c + 16 * row + 4 * k, v);
            }
        }
        h.write(this + 0xfc88, 0u8);
        h.call_sym(SCENE_UPDATE_MAP_YIQ, Conv::Thiscall(this), &[])
    }

    /// `graphicCharacterInstanceUpdateSkeletonounds(list)` `0x0063c500`:
    /// `updateBoundsFromSkeleton` on each mesh instance of the `PArray`
    /// (count `+0` re-read each time, elements at `+4`).
    pub fn update_skeleton_bounds(h: &mut impl Host, list: u32) {
        if list == 0 {
            return;
        }
        let mut i = 0u32;
        while i < h.read::<u32>(list) & COUNT_MASK {
            let data: u32 = h.read(list + 4);
            let mesh: u32 = h.read(data.wrapping_add(4 * i));
            h.call_sym(MESH_UPDATE_BOUNDS_FROM_SKELETON, Conv::Thiscall(mesh), &[]);
            i += 1;
        }
    }

    /// `graphicSetAbmap(5 floats)` `0x00642da0`:
    /// `g_PhyreScene->setAbmapCamera(...)`.
    pub fn set_abmap(h: &mut impl Host, f: [u32; 5]) -> u32 {
        let [a, b, c, d, e] = rt(f);
        on_global(h, SCENE, SCENE_SET_ABMAP_CAMERA, &[a, b, c, d, e])
    }

    /// `graphicAbmapUpdate()` `0x006390f0`: with an abmap, delete it when
    /// its state `+4` is 4, else update it. (Without one the original
    /// leaves the caller's `eax`; its caller reads none.)
    pub fn abmap_update(h: &mut impl Host) {
        let abmap = GRAPHIC_ABMAP.get(h);
        if abmap == 0 {
            return;
        }
        if h.read::<u32>(abmap + 4) == 4 {
            h.call_sym(ABMAP_DELETE, Conv::Cdecl, &[]);
        } else {
            h.call_sym(ABMAP_UPDATE, Conv::Thiscall(abmap), &[]);
        }
    }

    /// `PostProcessManager_onShaderBinLoaded_00694c20(data, task)`, the
    /// async loader's callback: `task = (manager, i)`; stores `data` at
    /// the manager's `+4 + 4i`, runs the step for `i`, and after the last
    /// of 14 `onAllShaderBinFilesLoaded` (its `eax`; else `i + 1`).
    pub fn post_on_shader_bin_loaded(h: &mut impl Host, data: u32, task: u32) -> u32 {
        instance(h, crate::glue::ASYNC_LOAD_INSTANCE);
        let manager: u32 = h.read(task);
        let i: u32 = h.read(task + 4);
        h.write(
            manager.wrapping_add(i.wrapping_mul(4)).wrapping_add(4),
            data,
        );
        h.call_sym(POST_SHADER_BIN_STEP, Conv::Thiscall(manager), &[i]);
        let next = i.wrapping_add(1);
        if next == 14 {
            return h.call_sym(POST_ON_ALL_SHADER_BINS, Conv::Thiscall(manager), &[]);
        }
        next
    }

    /// `graphicBrokenScreenUpdatePolygonData(a, b, 9 words)` `0x00639500`:
    /// the broken screen's polygon `(a, b)`, updated with the rest.
    pub fn broken_screen_update_polygon_data(h: &mut impl Host, a: &[u32; 11]) -> u32 {
        let broken = instance(h, crate::glue::BROKEN_SCREEN_INSTANCE);
        let polygon = h.call_sym(BROKEN_POLYGON, Conv::Thiscall(broken), &a[..2]);
        h.call_sym(BROKEN_POLYGON_UPDATE, Conv::Thiscall(polygon), &a[2..])
    }

    /// `ClassVFXRenderDataTable_RemoveDeletedPoolMeshInstances(this)`
    /// `0x0069fc70`: for each delete-pool entry whose 64-bit key (`+0xc`,
    /// 8 bytes each) is set, clears the mesh instance's `+0xc` (`+0x1c`).
    /// Returns the final index (the count).
    pub fn vfx_remove_deleted_pool_mesh_instances(h: &mut impl Host, this: u32) -> u32 {
        let mut i = 0u32;
        while i < h.read::<u32>(this + 8) & COUNT_MASK {
            let keys: u32 = h.read(this + 0xc);
            let lo: u32 = h.read(keys.wrapping_add(8 * i));
            let hi: u32 = h.read(keys.wrapping_add(8 * i + 4));
            if lo | hi != 0 {
                let meshes: u32 = h.read(this + 0x1c);
                let mesh: u32 = h.read(meshes.wrapping_add(4 * i));
                h.write(mesh.wrapping_add(0xc), 0u32);
            }
            i += 1;
        }
        i
    }

    /// `PhyreScene_isMeshVisible_006603c0(this, entry)` (`ret 4`): `al` =
    /// 0 without an entry; 1 when `+0x10d8c` is set or the entry has no
    /// mesh (`+0x1c`); else the mesh test with the camera's `+0x74`
    /// (`[+0xdc]`). The rest of `eax` is the entry's (or the test's).
    pub fn scene_is_mesh_visible(h: &mut impl Host, this: u32, entry: u32) -> u32 {
        if entry == 0 {
            return 0;
        }
        if h.read::<u32>(this + 0x0001_0d8c) != 0 {
            return entry & !0xff | 1;
        }
        let mesh: u32 = h.read(entry.wrapping_add(0x1c));
        if mesh == 0 {
            return entry & !0xff | 1;
        }
        let camera: u32 = h.read(this + 0xdc);
        h.call_sym(
            SCENE_MESH_VISIBLE_TEST,
            Conv::Thiscall(mesh),
            &[camera.wrapping_add(0x74)],
        )
    }

    /// `graphicUseOptimizedVFXShaderByVFXInstance(lo, hi)` `0x00644b10`:
    /// the instance's dynamic geometry gets `graphicUseOptimizedVFXShader`
    /// with the flag global, which is then set to 1.
    pub fn use_optimized_vfx_shader_by_instance(h: &mut impl Host, lo: u32, hi: u32) -> u32 {
        let mut r = lo | hi;
        if r != 0 {
            let table = VFX_RENDER_DATA_TABLE.get(h);
            let geo = h.call_sym(
                VFX_GET_DYN_GEO_BY_INSTANCE,
                Conv::Thiscall(table),
                &[lo, hi],
            );
            r = geo;
            if geo != 0 {
                let on = USE_OPTIMIZED_VFX_SHADER_ARG.get(h);
                r = h.call_sym(USE_OPTIMIZED_VFX_SHADER, Conv::Cdecl, &[geo, on]);
            }
        }
        USE_OPTIMIZED_VFX_SHADER_ARG.set(h, 1);
        r
    }

    /// `PhyreScene_moveClusterTo7c_00658f00(this)`: unless `+0xfb60` is
    /// above 0 or byte `+0xfb44` is set, moves the cluster `+0x11e4` from
    /// world `+0x9c` to world `+0x7c` and sets the byte.
    pub fn scene_move_cluster(h: &mut impl Host, this: u32) {
        if h.read::<i32>(this + 0xfb60) > 0 || h.read::<u8>(this + 0xfb44) != 0 {
            return;
        }
        let cluster = this + 0x11e4;
        h.call_sym(
            WORLD_REMOVE_CLUSTER,
            Conv::Thiscall(this + 0x9c),
            &[cluster],
        );
        h.call_sym(WORLD_ADD_CLUSTER, Conv::Thiscall(this + 0x7c), &[cluster]);
        h.write(this + 0xfb44, 1u8);
    }

    /// `PhyreScene_clearList_00658fc0(this)`: the list count `+0x10d84`
    /// keeps only its top bit ("not owned"); an owned list frees its data
    /// `+0x10d88` and clears the pointer. `eax`: `0xfe` (not owned) or
    /// `PhyreFree`'s.
    pub fn scene_clear_list(h: &mut impl Host, this: u32) -> u32 {
        let at = this + 0x0001_0d84;
        let v = h.read::<u32>(at) & 0x8000_0000;
        h.write(at, v);
        if v != 0 {
            return 0xfe;
        }
        let data: u32 = h.read(this + 0x0001_0d88);
        let r = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        h.write(this + 0x0001_0d88, 0u32);
        r
    }

    /// `graphicVFXisAtelShpInstanced(lo, hi)` `0x00645d40`: the first VFX
    /// data of the instance plus 0, 10000, ... 50000 (64-bit sums), or 0.
    pub fn vfx_is_atel_shp_instanced(h: &mut impl Host, lo: u32, hi: u32) -> u32 {
        if lo | hi == 0 {
            return 0;
        }
        let key = u64::from(lo) | u64::from(hi) << 32;
        for step in 0..6u64 {
            let k = key.wrapping_add(step * 10_000);
            let table = VFX_RENDER_DATA_TABLE.get(h);
            let r = h.call_sym(
                VFX_GET_DATA_BY_INSTANCE,
                Conv::Thiscall(table),
                &[k as u32, (k >> 32) as u32],
            );
            if r != 0 {
                return r;
            }
        }
        0
    }

    /// `PhyreScene_isSpecialMagic_0065f1f0()`: `al` = whether the magic id
    /// is one of 0x6a, 0x119, 0x203, 0x241, 0x20a, 0x29b, 0x1bd, 0x10e; the
    /// rest of `eax` is the id's.
    pub fn scene_is_special_magic(h: &mut impl Host, this: u32) -> u32 {
        let id = h.call_sym(MAGIC_ID, Conv::Thiscall(this), &[]);
        let hit = matches!(
            id,
            0x6a | 0x119 | 0x203 | 0x241 | 0x20a | 0x29b | 0x1bd | 0x10e
        );
        id & !0xff | u32::from(hit)
    }

    /// The material's current pass: `+0xc` when its remapping starts
    /// from the scene pass (`+8`), else the scene pass.
    fn current_pass(h: &mut impl Host, material: u32) -> u32 {
        let scene = h.rebase(PASS_SCENE);
        if h.read::<u32>(material + 8) == scene {
            h.read(material + 0xc)
        } else {
            scene
        }
    }

    /// `PhyreScene_UseVFXOptimizedTrans(material, on)` `0x00654b20` (cdecl;
    /// `on` read as a byte): on remaps the scene pass to the optimized
    /// transparency pass; off clears a remapping to it. `eax`: the
    /// remapping's, or the current pass.
    pub fn use_vfx_optimized_trans(h: &mut impl Host, material: u32, on: u32) -> u32 {
        let scene = h.rebase(PASS_SCENE);
        let trans = h.rebase(PASS_OPTIMIZED_TRANS);
        if on & 0xff != 0 {
            return h.call_sym(
                MATERIAL_SET_PASS_REMAPPING,
                Conv::Thiscall(material),
                &[scene, trans],
            );
        }
        let pass = current_pass(h, material);
        if pass != trans {
            return pass;
        }
        h.call_sym(
            MATERIAL_SET_PASS_REMAPPING,
            Conv::Thiscall(material),
            &[0, 0],
        )
    }

    /// `PhyreScene_ForceTransparent(material, on)` `0x00651400` (cdecl; `on`
    /// a byte): on remaps the scene pass to forced transparency unless the
    /// material is on the keep pass; off clears a forced remapping.
    pub fn force_transparent(h: &mut impl Host, material: u32, on: u32) -> u32 {
        let scene = h.rebase(PASS_SCENE);
        let force = h.rebase(PASS_FORCE_TRANSPARENT);
        let keep = h.rebase(PASS_KEEP);
        let pass = current_pass(h, material);
        if on & 0xff != 0 {
            if pass == keep {
                return pass;
            }
            return h.call_sym(
                MATERIAL_SET_PASS_REMAPPING,
                Conv::Thiscall(material),
                &[scene, force],
            );
        }
        if pass != force {
            return pass;
        }
        h.call_sym(
            MATERIAL_SET_PASS_REMAPPING,
            Conv::Thiscall(material),
            &[0, 0],
        )
    }

    /// `graphicUpdateDistortionVertex(a, b, 6 floats)` `0x00644940`.
    pub fn update_distortion_vertex(h: &mut impl Host, a: u32, b: u32, f: [u32; 6]) -> u32 {
        let distortion = instance(h, DISTORTION_INSTANCE);
        let [c, d, e, g, i, j] = rt(f);
        h.call_sym(
            DISTORTION_UPDATE_VERTEX,
            Conv::Thiscall(distortion),
            &[a, b, c, d, e, g, i, j],
        )
    }

    /// `PhyreScene_renderPostAA(this, mode)` `0x006535b0` (`ret 4`): mode
    /// 0 SMAA, 1 FXAA, on the post-process manager `+0x10cc` with the
    /// target `+0xd0`; returns 0.
    pub fn scene_render_post_aa(h: &mut impl Host, this: u32, mode: u32) -> u32 {
        match mode {
            0 => {
                let target: u32 = h.read(this + 0xd0);
                let post: u32 = h.read(this + 0x10cc);
                h.call_sym(POST_RENDER_SMAA, Conv::Thiscall(post), &[target]);
            }
            1 => {
                let post: u32 = h.read(this + 0x10cc);
                let b: u32 = h.read(post.wrapping_add(0x8c));
                let target: u32 = h.read(this + 0xd0);
                h.call_sym(POST_RENDER_FXAA, Conv::Thiscall(post), &[target, b]);
            }
            _ => {}
        }
        0
    }

    /// `graphicUseOptimizedVFXShader(geometry, on)` `0x00644ac0`:
    /// `UseVFXOptimizedTrans(material, on != 0)` on each material of the
    /// geometry's (`[+0xc] +0x10`) array. `eax`: 0 without geometry, else
    /// the final count (or `[+0xc]` when the array is empty).
    pub fn use_optimized_vfx_shader(h: &mut impl Host, geometry: u32, on: u32) -> u32 {
        if geometry == 0 {
            return 0;
        }
        let inner: u32 = h.read(geometry + 0xc);
        let materials: u32 = h.read(inner.wrapping_add(0x10));
        if h.read::<u32>(materials) & COUNT_MASK == 0 {
            return inner;
        }
        let flag = u32::from(on != 0);
        let mut i = 0u32;
        loop {
            let data = h.call_sym(PARRAY_DATA, Conv::Thiscall(materials), &[]);
            let material: u32 = h.read(data.wrapping_add(4 * i));
            h.call_sym(
                SCENE_USE_VFX_OPTIMIZED_TRANS,
                Conv::Cdecl,
                &[material, flag],
            );
            i += 1;
            let n = h.read::<u32>(materials) & COUNT_MASK;
            if i >= n {
                return n;
            }
        }
    }

    /// `PArray_assign_0064ead0` / `_0064eb20(this, other)` (`ret 4`): a
    /// word array (`+0` count, `+8` data) copied from `other` after
    /// `reserve(count)`; returns `this`.
    pub fn parray_assign(h: &mut impl Host, reserve: u32, this: u32, other: u32) -> u32 {
        if other == this {
            return this;
        }
        h.write(this, 0u32);
        let n: u32 = h.read(other);
        h.call_sym(reserve, Conv::Thiscall(this), &[n]);
        let mut i = 0i32;
        while i < h.read::<i32>(other) {
            let off = (i as u32).wrapping_mul(4);
            let dst = h.read::<u32>(this + 8).wrapping_add(off);
            if dst != 0 {
                let src: u32 = h.read(other + 8);
                let v: u32 = h.read(src.wrapping_add(off));
                h.write(dst, v);
            }
            i += 1;
        }
        let n: u32 = h.read(other);
        h.write(this, n);
        this
    }

    /// `PhyreScene_showAutoSavingPrompt(this)` `0x0066f480`: deletes the
    /// old prompt (`+0x10ddc`) and, with an Iggy menu manager, opens the
    /// new one (1220x660 at 50,50).
    pub fn scene_show_auto_saving_prompt(h: &mut impl Host, this: u32) {
        let at = this + 0x0001_0ddc;
        let old: u32 = h.read(at);
        if old != 0 {
            h.call_sym(IGGY_MENU_DTOR, Conv::Thiscall(old), &[]);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[old]);
        }
        if IGGY_MENU_MANAGER.get(h) != 0 {
            let path = h.rebase(AUTO_SAVE_PROMPT_PATH);
            let menu = h.call_sym(
                IGGY_PLAYER_OPEN_SCALED,
                Conv::Cdecl,
                &[path, 0x4c4, 0x294, 0x32, 0x32],
            );
            h.write(at, menu);
        }
    }

    /// `graphicVFXIsTextureWithBpLoaded(7 words)` `0x00645450`: the
    /// scene's `isVFXTextureLoaded`; 2 stays 2, 1 becomes 1 only when the
    /// cluster named by the last word exists; else 0.
    pub fn vfx_is_texture_with_bp_loaded(h: &mut impl Host, a: &[u32; 7]) -> u32 {
        let r = on_global(h, SCENE, SCENE_IS_VFX_TEXTURE_LOADED, a);
        if r == 2 {
            return 2;
        }
        if r == 1 {
            let clusters = instance(h, CLUSTER_INSTANCE);
            if h.call_sym(CLUSTER_BY_NAME, Conv::Thiscall(clusters), &[a[6]]) != 0 {
                return 1;
            }
        }
        0
    }

    /// `graphicCharacterInstanceIsVisible(list)` `0x0063b230`: 1 when the
    /// scene finds any entry of the `PArray` (`+4` elements) visible.
    pub fn character_instance_is_visible(h: &mut impl Host, list: u32) -> u32 {
        if list == 0 {
            return 0;
        }
        let mut i = 0u32;
        while i < h.read::<u32>(list) & COUNT_MASK {
            let data: u32 = h.read(list + 4);
            let entry: u32 = h.read(data.wrapping_add(4 * i));
            let scene = SCENE.get(h);
            let r = h.call_sym(SCENE_IS_MESH_VISIBLE, Conv::Thiscall(scene), &[entry]);
            if r & 0xff != 0 {
                return 1;
            }
            i += 1;
        }
        0
    }

    /// Grows an array (`+0` count, `+4` capacity, `+8` data) by one slot
    /// as the scene's arrays do: past the capacity, `reserve(n + 2 +
    /// n / 8)`. Returns the new last slot's address (0 when the data is
    /// null and the slot falls on 0).
    fn push_slot(h: &mut impl Host, array: u32, reserve: u32, size: u32) -> u32 {
        let n = h.read::<i32>(array).wrapping_add(1);
        h.write(array, n);
        if n > h.read::<i32>(array + 4) {
            let grown = (n / 8).wrapping_add(n).wrapping_add(2);
            h.call_sym(reserve, Conv::Thiscall(array), &[grown as u32]);
        }
        let n: u32 = h.read(array);
        let data: u32 = h.read(array + 8);
        data.wrapping_add(n.wrapping_mul(size)).wrapping_sub(size)
    }

    /// `PhyreScene_addToPipeline_00656590(this, entry)` (`ret 4`): clears
    /// the entry's `+0x1c` and appends it to the array at `+0xfb48`.
    pub fn scene_add_to_pipeline(h: &mut impl Host, this: u32, entry: u32) {
        if entry == 0 {
            return;
        }
        h.write(entry + 0x1c, 0u32);
        let slot = push_slot(h, this + 0xfb48, ARRAY_RESERVE_667F80, 4);
        if slot != 0 {
            h.write(slot, entry);
        }
    }

    /// `VFXDrawEventArray_Push(this, event)` `0x00656350` (`ret 4`): appends
    /// the 16-byte event; returns 0.
    pub fn vfx_draw_event_push(h: &mut impl Host, this: u32, event: u32) -> u32 {
        let slot = push_slot(h, this, ARRAY_RESERVE_668280, 16);
        if slot != 0 {
            for k in 0..4 {
                let v: u32 = h.read(event + 4 * k);
                h.write(slot + 4 * k, v);
            }
        }
        0
    }

    /// `PhyreScene_addVFXDrawEvent(this, event)` `0x00656940` (`ret 4`): an
    /// event of type 0..6 (`+0`) gets its order key `+4` (`+0xf8cc`, plus
    /// the current layer's offset `+0xf910 + 4 * [+0xf954]` when that is
    /// above 0), is pushed onto `+0xfb54` and counted in `+0xfb60 + 4 *
    /// type`. `eax`: the event's type word, re-read.
    pub fn scene_add_vfx_draw_event(h: &mut impl Host, this: u32, event: u32) -> u32 {
        let kind: i32 = h.read(event);
        if !(0..7).contains(&kind) {
            return kind as u32;
        }
        let base: u32 = h.read(this + 0xf8cc);
        h.write(event + 4, base);
        let layer: i32 = h.read(this + 0xf954);
        if layer > 0 {
            let off: u32 = h.read(this.wrapping_add((layer as u32).wrapping_mul(4)) + 0xf910);
            h.write(event + 4, off.wrapping_add(base));
        }
        h.call_sym(VFX_DRAW_EVENT_PUSH, Conv::Thiscall(this + 0xfb54), &[event]);
        let kind: u32 = h.read(event);
        let at = this.wrapping_add(kind.wrapping_mul(4)).wrapping_add(0xfb60);
        let c: u32 = h.read(at);
        h.write(at, c.wrapping_add(1));
        kind
    }

    /// `PRedBlackTree_iteratorBegin_0064dae0(this, tree)` (`ret 4`): the
    /// iterator `(tree, node, next)` at the leftmost node (from the root
    /// `+0xc`, following `+0` until it reaches the header); `next` is the
    /// node's successor, or the header itself. Returns `this`.
    pub fn tree_iterator_begin(h: &mut impl Host, this: u32, tree: u32) -> u32 {
        h.write(this, tree);
        let mut node: u32 = h.read(tree.wrapping_add(0xc));
        if node != tree && node != 0 {
            loop {
                let left: u32 = h.read(node);
                if left == tree {
                    break;
                }
                node = left;
            }
        }
        h.write(this + 4, node);
        let next = if node == tree {
            node
        } else {
            h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(tree), &[node])
        };
        h.write(this + 8, next);
        this
    }

    /// `BrokenScreenEffect_fillFloats_006d4150(this)`: mode `+0` 1 fills
    /// the `+8 * 4` floats at `[+0x48]` with 1.0, mode 2 with the constant
    /// at `0x00b412c4` (count and pointer re-read per store). `eax`: the
    /// last count read, or the mode.
    pub fn broken_fill_floats(h: &mut impl Host, this: u32) -> u32 {
        let mode: u32 = h.read(this);
        let value = match mode {
            1 => 0x3f80_0000,
            2 => round_trip(f32::from_bits(BROKEN_FILL_VALUE.get(h))).to_bits(),
            _ => return mode,
        };
        let mut n = h.read::<u32>(this + 8).wrapping_shl(2);
        let mut i = 0u32;
        while i < n {
            let data: u32 = h.read(this + 0x48);
            h.write(data.wrapping_add(4 * i), value);
            i += 1;
            n = h.read::<u32>(this + 8).wrapping_shl(2);
        }
        n
    }

    /// `ShadowDynamicMesh_descriptorDestroy_006789c0(p)` (stdcall, a class
    /// descriptor slot): the object's virtual destructor (slot 0, argument
    /// 0) on `p - 4` (or on 0 for a null `p`); `al` = 1 over its `eax`.
    pub fn shadow_dynamic_mesh_destroy(h: &mut impl Host, p: u32) -> u32 {
        let obj = if p == 0 { 0 } else { p.wrapping_sub(4) };
        let r = h.call_virtual(obj, 0, &[0]);
        r & !0xff | 1
    }

    /// `RenderDelegationHelper_setDataToFreeList(this, data)` `0x006ae1a0`
    /// (`ret 4`): the slot of 64 (`+8`, 8 bytes each) holding `data`, its
    /// busy word (`+0xc`) swapped from 1 to 0 atomically (a message on
    /// stderr when it was not 1); the index, or -1.
    pub fn set_data_to_free_list(h: &mut impl Host, this: u32, data: u32) -> u32 {
        for i in 0..64u32 {
            if h.read::<u32>(this + 8 + 8 * i) != data {
                continue;
            }
            let busy = this + 0xc + 8 * i;
            if import(
                h,
                INTERLOCKED_COMPARE_EXCHANGE,
                Conv::Stdcall,
                &[busy, 0, 1],
            ) != 1
            {
                let format = h.rebase(CAS_FAILED_FORMAT);
                let iob = import(h, IOB_FUNC, Conv::Cdecl, &[]);
                import(
                    h,
                    FPRINTF,
                    Conv::Cdecl,
                    &[iob.wrapping_add(0x40), format, i],
                );
            }
            return i;
        }
        0xffff_ffff
    }
}
