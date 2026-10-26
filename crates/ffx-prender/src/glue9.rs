//! The render-side glue, ninth batch (M4 port round 3, list
//! `h3-glue-render`): the scene's mesh-instance and pool-array
//! bookkeeping, the field-map load completion, the VFX geometry pool, the
//! broken-screen pass, the shadow map update (a 16-byte result through a
//! hidden pointer), the distortion constructor, the capture texture, the
//! resolution scale (x87) and a few lookups. Frames passed to callees: as
//! in [`crate::glue5`]; security cookies left out, as in
//! [`crate::glue8`].

use ffx_bridge::Global;

pub use crate::glue::SCENE;

/// `Phyre_PFramework_PApplication_GetApplication`.
pub const GET_APPLICATION: u32 = crate::glue::GET_APPLICATION;
/// `Phyre_PCluster_deleteObject(this, object)`.
pub const CLUSTER_DELETE_OBJECT: u32 = crate::glue5::CLUSTER_DELETE_OBJECT;
/// `Phyre_PCluster_create(this, n, class, 0)` (`ret 0xc`).
pub const CLUSTER_CREATE: u32 = 0x0044_1010;
/// The classes the scene pools fall back to creating in a cluster.
pub const POOL_CLASS_8: u32 = 0x00c9_98e8;
pub const POOL_CLASS_56: u32 = 0x00c9_9c80;
/// `PhyreCtor_zero56_00433700(this)`.
pub const CTOR_ZERO56: u32 = 0x0043_3700;
/// `Phyre_PRendering_PMeshInstance_dtor_PMeshInstance(this)`.
pub const MESH_INSTANCE_DTOR: u32 = 0x004f_2c50;
/// `graphicVFXRemoveDeletedPoolMeshInstances()`.
pub const VFX_REMOVE_DELETED_POOL_MESH_INSTANCES: u32 = 0x0064_5620;
/// `PhyreScene_deletePMeshInstanceInCluster(this, mesh, cluster)`.
pub const SCENE_DELETE_PMESH_INSTANCE: u32 = 0x0065_a4c0;
/// `PhyreScene_removeVFXFromRenderArray(this, mesh)`.
pub const SCENE_REMOVE_VFX_FROM_RENDER_ARRAY: u32 = 0x0066_87a0;
/// `crt_memset_thunk(p, c, n)` (cdecl).
pub const MEMSET: u32 = crate::glue8::MEMSET;
/// `AssetListReader_getAssetCount(this)`.
pub const ASSET_LIST_COUNT: u32 = crate::glue6::ASSET_LIST_COUNT;
/// `PApplication_FixupClusters(clusters, n)` (cdecl).
pub const FIXUP_CLUSTERS: u32 = crate::glue5::FIXUP_CLUSTERS;
/// `PhyreScene_registClustersToAddToWorld(this, a, b)`.
pub const SCENE_REGIST_CLUSTERS_TO_ADD: u32 = 0x0066_8550;
/// `FinishReadPhyreBattleMap()`.
pub const FINISH_READ_BATTLE_MAP: u32 = 0x0078_2fd0;
/// `VFXDynamicGeometry_VFXDynamicGeometry(this)`.
pub const VFX_DYN_GEO_CTOR: u32 = 0x0069_e3e0;
/// `Phyre_PFreeList_allocateNewBlock(this, n)` and `deallocate(this, node)`.
pub const FREE_LIST_ALLOCATE_BLOCK: u32 = crate::glue6::FREE_LIST_ALLOCATE_BLOCK;
pub const FREE_LIST_DEALLOCATE: u32 = crate::glue6::FREE_LIST_DEALLOCATE;
/// `PArray_assign_0064ead0(this, &array)`.
pub const PARRAY_ASSIGN: u32 = 0x0064_ead0;
/// The tree's insert `0x0064bfa0(this, node)` (non-zero: a duplicate).
pub const TREE_INSERT_64BFA0: u32 = 0x0064_bfa0;
/// `user_free_thunk(p)` (cdecl).
pub const USER_FREE: u32 = 0x0068_7010;
/// `Phyre_PRedBlackTree_postInsertFinalize(this, node)`.
pub const TREE_POST_INSERT: u32 = crate::glue6::TREE_POST_INSERT;
/// `Phyre_PRedBlackTree_successor(this, node)`.
pub const TREE_SUCCESSOR: u32 = 0x0057_5160;
/// `PSamplerStateD3D11_ctor(this)`.
pub const SAMPLER_STATE_CTOR: u32 = crate::glue6::SAMPLER_STATE_CTOR;
/// The distortion's default strength (`float` in `.rdata`).
pub const DISTORTION_DEFAULT: Global<u32> = Global::new(0x00b4_3d18);
/// The renderer methods `renderCaptureTexture` calls.
pub const RENDERER_SET_COLOR_TARGET: u32 = 0x005b_b130;
pub const RENDERER_SET_VIEWPORT: u32 = 0x005b_c060;
pub const RENDERER_SET_CLEAR_COLOR: u32 = crate::glue6::RENDERER_SET_CLEAR_COLOR;
pub const RENDERER_BEGIN_SCENE: u32 = crate::glue6::RENDERER_BEGIN_SCENE;
pub const RENDERER_END_SCENE: u32 = crate::glue6::RENDERER_END_SCENE;
/// `PostProcessManager_renderFullScreenTexture(this, renderer, texture, 0)`.
pub const POST_RENDER_FULL_SCREEN_TEXTURE: u32 = 0x0069_59d0;
/// `MagicFile_GetHandle()` and `MagicFile_GetMagicId()`.
pub const MAGIC_FILE_HANDLE: u32 = 0x009d_a390;
pub const MAGIC_FILE_ID: u32 = 0x009d_a3b0;
/// `graphicVFXDestroy(lo, hi)`.
pub const VFX_DESTROY: u32 = 0x0064_5280;
/// `PhyreScene_attachChrClusterTo3DWorld(this)`.
pub const SCENE_ATTACH_CHR_CLUSTER: u32 = 0x0065_78e0;
/// The two buffers `graphicCheckChrClusterRelationship` frees
/// (`0x00686fe0`, cdecl) and the word it clears.
pub const CHR_RELATION_BUFFERS: [Global<u32>; 2] =
    [Global::new(0x012e_9618), Global::new(0x012e_961c)];
pub const CHR_RELATION_WORD: Global<u32> = Global::new(0x012e_9620);
pub const FREE_686FE0: u32 = 0x0068_6fe0;
/// `fiosUnifyFilename(name, out, size)` (cdecl).
pub const FIOS_UNIFY_FILENAME: u32 = crate::glue8::FIOS_UNIFY_FILENAME;
/// The map VFX texture's file name and the format that adds the map id.
pub const MAP_VFX_NAME: u32 = 0x00b4_3b18;
pub const MAP_VFX_FORMAT: u32 = 0x00b4_3b34;
/// `sprintf`'s import slot (cdecl).
pub const SPRINTF: u32 = 0x00b0_c3dc;
/// `PhyreScene_loadVFXTextureAsync(this, id, name)`.
pub const SCENE_LOAD_VFX_TEXTURE_ASYNC: u32 = 0x0066_5f60;
/// `strstr`'s and `sscanf`'s import slots (cdecl).
pub const STRSTR: u32 = crate::glue8::STRSTR;
pub const SSCANF: u32 = 0x00b0_c448;
/// The marker of a texture without layout, and the layout's format.
pub const VFX_TEXTURE_NO_LAYOUT: u32 = 0x00b4_2a18;
pub const VFX_TEXTURE_LAYOUT_FORMAT: u32 = 0x00b4_2a20;
/// The resolution scale's constants (`double`s in `.rdata`).
pub const ASPECT_LIMIT: Global<f64> = Global::new(0x00b4_3d68);
pub const RESOLUTION_DIVISOR: Global<f64> = Global::new(0x00b4_3d90);
pub const VU0_FIXED_SCALE: Global<f64> = Global::new(0x00b3_13a0);
pub const RESOLUTION_FACTOR: Global<f64> = Global::new(0x00b9_21e0);
/// `BrokenScreenEffect_createRenderTarget(this)`.
pub const BROKEN_CREATE_RENDER_TARGET: u32 = 0x006d_5050;
/// The broken-screen pass draw `0x006d3fb0(this, 6 words)`.
pub const BROKEN_PASS_6D3FB0: u32 = 0x006d_3fb0;
/// `BrokenScreenEffect_setColour_006d4620(this, &colour)`.
pub const BROKEN_SET_COLOUR: u32 = 0x006d_4620;
/// `0x006d4540(this, texture, 0)`.
pub const BROKEN_SET_TEXTURE_6D4540: u32 = 0x006d_4540;
/// `ShadowManager_getInstance()` and `updateShadowMap(this, 12 words)`.
pub const SHADOW_MANAGER_INSTANCE: u32 = 0x006d_2390;
pub const SHADOW_UPDATE_MAP: u32 = 0x006d_3000;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::glue::bridge::instance;
    use ffx_bridge::x87::{ftol, round_trip};
    use ffx_graphic::host::{Conv, Host};

    /// Calls the import whose slot is at `slot` (cdecl); its `eax`.
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let at = h.rebase(slot);
        let f: u32 = h.read(at);
        h.call(f, Conv::Cdecl, args)
    }

    /// `PhyreScene_deletePMeshInstanceInCluster(this, mesh, cluster)`
    /// `0x0065a4c0` (`ret 8`): a mesh instance of the scene's own pool
    /// (cluster `+0x12d4`, slot `(mesh - [+0xfbcc]) / 132` up to 3000) is
    /// destroyed in place, its use byte (`[+0xfbe0]`) cleared and the
    /// lowest free slot (`+0xfbe4`) lowered; any other is deleted from
    /// `cluster` with its two parts (`+4`, `+0x1c`, when set).
    pub fn scene_delete_pmesh_instance(h: &mut impl Host, this: u32, mesh: u32, cluster: u32) {
        if cluster == this.wrapping_add(0x12d4) {
            let base: u32 = h.read(this + 0xfbcc);
            let slot = (mesh.wrapping_sub(base) as i32) / 132;
            if slot as u32 <= 0xbb7 {
                let used: u32 = h.read(this + 0xfbe0);
                h.write(used.wrapping_add(slot as u32), 0u8);
                h.call_sym(MESH_INSTANCE_DTOR, Conv::Thiscall(mesh), &[]);
                if slot < h.read::<i32>(this + 0xfbe4) {
                    h.write(this + 0xfbe4, slot);
                }
                return;
            }
        }
        for part in [4, 0x1c] {
            let p: u32 = h.read(mesh.wrapping_add(part));
            if p != 0 {
                h.call_sym(CLUSTER_DELETE_OBJECT, Conv::Thiscall(cluster), &[p]);
            }
        }
        h.call_sym(CLUSTER_DELETE_OBJECT, Conv::Thiscall(cluster), &[mesh]);
    }

    /// `PhyreScene_doDeleteMeshInstances(this)` `0x0065a620`: the VFX
    /// pool's deleted instances first, then each queued mesh instance
    /// (`+0x3894`, clusters `+0x389c`, count `+0x38a0`) deleted and taken
    /// out of the render array; both queues zeroed (capacities `+0x3890`,
    /// `+0x3898`) and emptied.
    pub fn scene_do_delete_mesh_instances(h: &mut impl Host, this: u32) {
        h.call_sym(VFX_REMOVE_DELETED_POOL_MESH_INSTANCES, Conv::Cdecl, &[]);
        let mut i = 0i32;
        while i < h.read::<i32>(this + 0x38a0) {
            let at = 4 * i as u32;
            let meshes: u32 = h.read(this + 0x3894);
            let mesh: u32 = h.read(meshes.wrapping_add(at));
            if mesh != 0 {
                let clusters: u32 = h.read(this + 0x389c);
                let cluster: u32 = h.read(clusters.wrapping_add(at));
                h.call_sym(
                    SCENE_DELETE_PMESH_INSTANCE,
                    Conv::Thiscall(this),
                    &[mesh, cluster],
                );
                let meshes: u32 = h.read(this + 0x3894);
                let mesh: u32 = h.read(meshes.wrapping_add(at));
                h.call_sym(
                    SCENE_REMOVE_VFX_FROM_RENDER_ARRAY,
                    Conv::Thiscall(this),
                    &[mesh],
                );
            }
            i += 1;
        }
        for (capacity, data) in [(0x3890, 0x3894), (0x3898, 0x389c)] {
            let n = (h.read::<u32>(this + capacity) & 0x7fff_ffff) << 2;
            let p: u32 = h.read(this + data);
            h.call_sym(MEMSET, Conv::Cdecl, &[p, 0, n]);
        }
        h.write(this + 0x38a0, 0u32);
    }

    /// `PhyreScene_doOnAllFieldMapAssetsLoaded(this, a, b, list, reader)`
    /// `0x0065a6c0` (`ret 0x10`): the list's clusters (`[list + 8]`) fixed
    /// up (the reader's asset count of them; a failure is returned), handed
    /// to `registClustersToAddToWorld(a, list)`; with both map kinds
    /// settled (`+0x10d1` clear or `+0x10d8` set, `+0x10d0` clear or
    /// `+0x112c` set) `+0x1194` = 1, else `+0xd8` = 0; then
    /// `FinishReadPhyreBattleMap`. 0.
    pub fn scene_do_on_all_field_map_assets_loaded(
        h: &mut impl Host,
        this: u32,
        a: u32,
        list: u32,
        reader: u32,
    ) -> u32 {
        h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let clusters: u32 = h.read(list.wrapping_add(8));
        let count = h.call_sym(ASSET_LIST_COUNT, Conv::Thiscall(reader), &[]);
        let r = h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[clusters, count]);
        if r != 0 {
            return r;
        }
        h.call_sym(
            SCENE_REGIST_CLUSTERS_TO_ADD,
            Conv::Thiscall(this),
            &[a, list],
        );
        let first = h.read::<u8>(this + 0x10d1) == 0 || h.read::<u32>(this + 0x10d8) != 0;
        let second = h.read::<u8>(this + 0x10d0) == 0 || h.read::<u32>(this + 0x112c) != 0;
        if first && second {
            h.write(this + 0x1194, 1u8);
        } else {
            h.write(this + 0xd8, 0u32);
        }
        h.call_sym(FINISH_READ_BATTLE_MAP, Conv::Cdecl, &[]);
        0
    }

    /// `ClassVFXRenderDataTable_GetVFXDynamicGeometryFromPool(this,
    /// &segment, &geometry)` `0x0069f750` (`ret 8`): with both outputs and
    /// a free slot (`+0x38` below 3000), its 0xd0-byte block (`+0x2c`,
    /// zeroed) and its 0xf8-byte geometry (`+0x30`, constructed; 0 for a
    /// null address), the use byte (`+0x34`) set and `+0x38` moved to the
    /// next unused slot. 1, or 0.
    pub fn vfx_geometry_from_pool(h: &mut impl Host, this: u32, out: u32, geometry: u32) -> u32 {
        if out == 0 || geometry == 0 {
            return 0;
        }
        let next: i32 = h.read(this + 0x38);
        if next >= 3000 {
            return 0;
        }
        let blocks: u32 = h.read(this + 0x2c);
        let block = blocks.wrapping_add((next as u32).wrapping_mul(0xd0));
        h.write(out, block);
        h.call_sym(MEMSET, Conv::Cdecl, &[block, 0, 0xd0]);
        let next: u32 = h.read(this + 0x38);
        let geos: u32 = h.read(this + 0x30);
        let geo = next.wrapping_mul(0xf8).wrapping_add(geos);
        let made = if geo == 0 {
            0
        } else {
            h.call_sym(VFX_DYN_GEO_CTOR, Conv::Thiscall(geo), &[])
        };
        h.write(geometry, made);
        let used: u32 = h.read(this + 0x34);
        let next: u32 = h.read(this + 0x38);
        h.write(next.wrapping_add(used), 1u8);
        let mut k = h.read::<i32>(this + 0x38).wrapping_add(1);
        if k < 3000 {
            let used: u32 = h.read(this + 0x34);
            while h.read::<u8>((k as u32).wrapping_add(used)) != 0 {
                k += 1;
                if k >= 3000 {
                    break;
                }
            }
        }
        h.write(this + 0x38, k);
        1
    }

    /// `ParallelArray_erase_006b2ca0(this, value)` (`ret 4`): `value`
    /// looked up in the word array `+0`..`+8`; when found it is removed
    /// there and its 20-byte partner (`+4`..`+0xc`) too, both ends moved
    /// down, and the freed word set to -1 while the count stays below the
    /// capacity `+0x10`. `eax`: the new count, or where the search stopped.
    pub fn parallel_erase(h: &mut impl Host, this: u32, value: u32) -> u32 {
        let mut at: u32 = h.read(this);
        let end: u32 = h.read(this + 8);
        let mut partner: u32 = h.read(this + 4);
        if at >= end {
            return at;
        }
        loop {
            if h.read::<u32>(at) == value {
                break;
            }
            at = at.wrapping_add(4);
            partner = partner.wrapping_add(0x14);
            if at >= end {
                return at;
            }
        }
        at = at.wrapping_add(4);
        if at < end {
            loop {
                let w: u32 = h.read(at);
                h.write(at.wrapping_sub(4), w);
                at = at.wrapping_add(4);
                if at >= h.read::<u32>(this + 8) {
                    break;
                }
            }
        }
        let mut from = partner.wrapping_add(0x14);
        if from < h.read::<u32>(this + 0xc) {
            let mut to = from.wrapping_sub(0x14);
            loop {
                for k in 0..5 {
                    let w: u32 = h.read(from + 4 * k);
                    h.write(to + 4 * k, w);
                }
                from = from.wrapping_add(0x14);
                to = to.wrapping_add(0x14);
                if from >= h.read::<u32>(this + 0xc) {
                    break;
                }
            }
        }
        let end = h.read::<u32>(this + 8).wrapping_sub(4);
        h.write(this + 8, end);
        let end2 = h.read::<u32>(this + 0xc).wrapping_sub(0x14);
        h.write(this + 0xc, end2);
        let begin: u32 = h.read(this);
        let count = (end.wrapping_sub(begin) as i32) >> 2;
        if count < h.read::<i32>(this + 0x10) {
            h.write(end, u32::MAX);
        }
        count as u32
    }

    /// `BrokenScreenEffect_setVertex_006d49f0(this, i, p0, .., p7)` (`ret
    /// 0x24`): each given float (copied through the x87) into vertex `i`:
    /// `p0..p2` the position (`[+0x40] + 12 i`), `p3..p5` the second
    /// vector (`[+0x44] + 12 i`), `p6, p7` the texture coordinate
    /// (`[+0x4c] + 8 i`).
    pub fn broken_set_vertex(h: &mut impl Host, this: u32, i: u32, p: [u32; 8]) {
        for (k, &src) in p.iter().enumerate() {
            if src == 0 {
                continue;
            }
            let (array, at) = match k {
                0..=2 => (0x40, i.wrapping_mul(12).wrapping_add(4 * k as u32)),
                3..=5 => (0x44, i.wrapping_mul(12).wrapping_add(4 * (k as u32 - 3))),
                _ => (0x4c, i.wrapping_mul(8).wrapping_add(4 * (k as u32 - 6))),
            };
            let value = round_trip(f32::from_bits(h.read(src))).to_bits();
            let base: u32 = h.read(this + array);
            h.write(base.wrapping_add(at), value);
        }
    }

    /// `PRedBlackTree_insertArray_0065fca0(this, &key, &array)` (`ret 8`):
    /// a node from the tree's free list (`+0x18`, refilled by
    /// `allocateNewBlock(+0x20)`; 0xe when none), linked to itself, the key
    /// at `+0xc`, an empty array at `+0x10` assigned from `array`; inserted
    /// (`0x0064bfa0`): a duplicate frees the array's storage and the node
    /// (0x12), else the tree is rebalanced (0).
    pub fn tree_insert_array(h: &mut impl Host, this: u32, key: u32, array: u32) -> u32 {
        let list = this + 0x18;
        let mut node: u32 = h.read(list);
        if node == 0 {
            let n: u32 = h.read(this + 0x20);
            h.call_sym(FREE_LIST_ALLOCATE_BLOCK, Conv::Thiscall(list), &[n]);
            node = h.read(list);
            if node == 0 {
                return 0xe;
            }
        }
        let next: u32 = h.read(node);
        h.write(list, next);
        h.write(node, node);
        h.write(node + 4, node);
        h.write(node + 8, node);
        let k: u32 = h.read(key);
        h.write(node + 0xc, k);
        for off in [0x10, 0x14, 0x18] {
            h.write(node + off, 0u32);
        }
        h.call_sym(PARRAY_ASSIGN, Conv::Thiscall(node + 0x10), &[array]);
        if h.call_sym(TREE_INSERT_64BFA0, Conv::Thiscall(this), &[node]) != 0 {
            let data: u32 = h.read(node + 0x18);
            if data != 0 {
                h.call_sym(USER_FREE, Conv::Cdecl, &[data]);
            }
            h.call_sym(FREE_LIST_DEALLOCATE, Conv::Thiscall(list), &[node]);
            return 0x12;
        }
        h.call_sym(TREE_POST_INSERT, Conv::Thiscall(this), &[node]);
        0
    }

    /// A scene pool's slot claim (`0x00659860`, `0x00659910`): for the
    /// scene's own cluster with a free slot (counter below 3000), the
    /// slot's address, its use byte set, and the counter moved to the next
    /// unused slot (3000 when none).
    pub(crate) fn claim_slot(
        h: &mut impl Host,
        this: u32,
        counter: u32,
        base: u32,
        used: u32,
        stride: u32,
    ) -> Option<u32> {
        let slot: i32 = h.read(this + counter);
        if slot >= 3000 {
            return None;
        }
        let start: u32 = h.read(this + base);
        let p = start.wrapping_add((slot as u32).wrapping_mul(stride));
        let bytes: u32 = h.read(this + used);
        h.write(bytes.wrapping_add(slot as u32), 1u8);
        let mut k = h.read::<i32>(this + counter).wrapping_add(1);
        let bytes: u32 = h.read(this + used);
        while k < 3000 {
            if h.read::<u8>(bytes.wrapping_add(k as u32)) == 0 {
                break;
            }
            k += 1;
        }
        h.write(this + counter, k.min(3000));
        Some(p)
    }

    /// `PhyreScene_newPoolArray_00659860(this, cluster)` (`ret 4`): from
    /// the scene's own pool (cluster `+0x12d4`; slots `+0xfb7c`, use bytes
    /// `+0xfb88`, counter `+0xfb8c`) an emptied 8-byte array, else
    /// `PCluster_create(cluster, 1, class, 0)`.
    pub fn scene_new_pool_array(h: &mut impl Host, this: u32, cluster: u32) -> u32 {
        if cluster == this.wrapping_add(0x12d4)
            && let Some(p) = claim_slot(h, this, 0xfb8c, 0xfb7c, 0xfb88, 8)
        {
            if p == 0 {
                return 0;
            }
            h.write(p, 0u32);
            h.write(p + 4, 0u32);
            return p;
        }
        let class = h.rebase(POOL_CLASS_8);
        h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, class, 0])
    }

    /// `PhyreScene_newPoolObject56_00659910(this, cluster)` (`ret 4`): as
    /// [`scene_new_pool_array`] for the 56-byte objects (slots `+0xfb90`,
    /// use bytes `+0xfb9c`, counter `+0xfba0`), constructed
    /// (`PhyreCtor_zero56`).
    pub fn scene_new_pool_object56(h: &mut impl Host, this: u32, cluster: u32) -> u32 {
        if cluster == this.wrapping_add(0x12d4)
            && let Some(p) = claim_slot(h, this, 0xfba0, 0xfb90, 0xfb9c, 56)
        {
            if p == 0 {
                return 0;
            }
            return h.call_sym(CTOR_ZERO56, Conv::Thiscall(p), &[]);
        }
        let class = h.rebase(POOL_CLASS_56);
        h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, class, 0])
    }

    /// `Distortion_ctor_006c2010(this)`: its fields cleared, the sampler
    /// state at `+0x44` constructed, the strength `+0x78` the default.
    /// Returns `this`.
    pub fn distortion_ctor(h: &mut impl Host, this: u32) -> u32 {
        for off in [0, 4, 8, 0x24, 0x28, 0x2c, 0x30, 0x34, 0x38, 0x3c, 0x40] {
            h.write(this + off, 0u32);
        }
        h.call_sym(SAMPLER_STATE_CTOR, Conv::Thiscall(this + 0x44), &[]);
        let strength = round_trip(f32::from_bits(DISTORTION_DEFAULT.get(h))).to_bits();
        h.write(this + 0x78, strength);
        h.write(this + 0x7c, 0u16);
        h.write(this + 0x7e, 0u8);
        for off in [0x20, 0x14, 0x18, 0x1c, 0x68, 0x6c, 0x70, 0x74] {
            h.write(this + off, 0u32);
        }
        this
    }

    /// `PhyreScene_renderCaptureTexture(this)` `0x0066b130`: on the
    /// renderer (`+0xd0`) the default colour target, the full-window
    /// viewport, clear colour (0, 0, 0, 1), an empty scene, then the
    /// post-process manager (`+0x10cc`) draws its capture texture (`+0xac`
    /// entry, 0 when flagged) full screen (its `eax`).
    pub fn scene_render_capture_texture(h: &mut impl Host, this: u32) -> u32 {
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(RENDERER_SET_COLOR_TARGET, Conv::Thiscall(renderer), &[0, 0]);
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let height: u32 = h.read(app.wrapping_add(0x270));
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let renderer: u32 = h.read(this + 0xd0);
        let width: u32 = h.read(app.wrapping_add(0x26c));
        h.call_sym(
            RENDERER_SET_VIEWPORT,
            Conv::Thiscall(renderer),
            &[0, 0, width, height],
        );
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(
            RENDERER_SET_CLEAR_COLOR,
            Conv::Thiscall(renderer),
            &[0, 0, 0, 1f32.to_bits()],
        );
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(RENDERER_BEGIN_SCENE, Conv::Thiscall(renderer), &[1]);
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(RENDERER_END_SCENE, Conv::Thiscall(renderer), &[1]);
        let post: u32 = h.read(this + 0x10cc);
        let entry: u32 = h.read(post.wrapping_add(0xac));
        let texture = if h.read::<u8>(entry.wrapping_add(4)) == 0 {
            h.read(entry)
        } else {
            0
        };
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(
            POST_RENDER_FULL_SCREEN_TEXTURE,
            Conv::Thiscall(post),
            &[renderer, texture, 0],
        )
    }

    /// `graphicCheckChrClusterRelationship()` `0x0063d9c0`: during magic
    /// 0x9a or 0x1a6 (a magic file open) and unless the scene's `+0xfb45`
    /// is set, the magic's two effect instances `(0, id)` and `(1, id)`
    /// destroyed, the character cluster attached to the 3D world, and the
    /// two relation buffers freed and cleared. Returns 0.
    pub fn check_chr_cluster_relationship(h: &mut impl Host) -> u32 {
        if h.call_sym(MAGIC_FILE_HANDLE, Conv::Cdecl, &[]) == u32::MAX {
            return 0;
        }
        if h.call_sym(MAGIC_FILE_ID, Conv::Cdecl, &[]) != 0x9a
            && h.call_sym(MAGIC_FILE_ID, Conv::Cdecl, &[]) != 0x1a6
        {
            return 0;
        }
        let scene = SCENE.get(h);
        if h.read::<u8>(scene + 0xfb45) != 0 {
            return 0;
        }
        let id = h.call_sym(MAGIC_FILE_ID, Conv::Cdecl, &[]);
        h.call_sym(VFX_DESTROY, Conv::Cdecl, &[0, id]);
        let id = h.call_sym(MAGIC_FILE_ID, Conv::Cdecl, &[]);
        h.call_sym(VFX_DESTROY, Conv::Cdecl, &[1, id]);
        let scene = SCENE.get(h);
        h.call_sym(SCENE_ATTACH_CHR_CLUSTER, Conv::Thiscall(scene), &[]);
        for buffer in CHR_RELATION_BUFFERS {
            let p = buffer.get(h);
            if p != 0 {
                h.call_sym(FREE_686FE0, Conv::Cdecl, &[p]);
                buffer.set(h, 0);
            }
        }
        CHR_RELATION_WORD.set(h, 0);
        0
    }

    /// `PhyreScene_loadMapVFXTexture(this)` `0x00664f60` (frame: two
    /// 256-byte names at `ebp - 0x104` and `ebp - 0x204`): unless already
    /// requested (`+0x25`), the map's texture name (the unified file name
    /// formatted with the map id `+0xfaf0`, colour bit cleared; 5 without
    /// a map) loaded asynchronously as texture 1, `+0x25` set. 0 when
    /// requested before, else the load's `eax`.
    pub fn scene_load_map_vfx_texture(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let file = ebp.wrapping_sub(0x104);
        let name = ebp.wrapping_sub(0x204);
        let base = h.rebase(MAP_VFX_NAME);
        h.call_sym(FIOS_UNIFY_FILENAME, Conv::Cdecl, &[base, file, 0x100]);
        if h.read::<u8>(this + 0x25) != 0 {
            return 0;
        }
        let word: u32 = h.read(this + 0xfaf0);
        let map = word.wrapping_sub(u32::from(h.read::<u8>(this + 0xfaf0)) & 1);
        if map == 0 {
            return 5;
        }
        let format = h.rebase(MAP_VFX_FORMAT);
        import(h, SPRINTF, &[name, format, file, map]);
        h.write(this + 0x25, 1u8);
        h.call_sym(
            SCENE_LOAD_VFX_TEXTURE_ASYNC,
            Conv::Thiscall(this),
            &[1, name],
        )
    }

    /// `PhyreScene_setVFXTextureLayout_00656ae0(id, kind, name)` (stdcall,
    /// `ret 0xc`; `args`: its two argument words, which `sscanf` writes;
    /// `locals`: the four words below `ebp`, from `ebp - 0x10`): entry
    /// `id` (20 bytes) of the scene's table `+0x48` gets `kind`; for kinds
    /// up to 2, -1 when the name holds the no-layout marker, else the
    /// layout read from the name: id word, then five half-words.
    pub fn scene_set_vfx_texture_layout(
        h: &mut impl Host,
        args: u32,
        locals: u32,
        id: u32,
        kind: u32,
        name: u32,
    ) {
        let scene = SCENE.get(h);
        let table: u32 = h.read(scene + 0x48);
        let entry = table.wrapping_add(id.wrapping_mul(20));
        h.write(entry, kind);
        if kind as i32 > 2 {
            return;
        }
        let marker = h.rebase(VFX_TEXTURE_NO_LAYOUT);
        if import(h, STRSTR, &[name, marker]) != 0 {
            h.write(entry + 4, u32::MAX);
            return;
        }
        let format = h.rebase(VFX_TEXTURE_LAYOUT_FORMAT);
        import(
            h,
            SSCANF,
            &[
                name,
                format,
                args,
                args + 4,
                locals + 0xc,
                locals + 8,
                locals + 4,
                locals,
            ],
        );
        // the four locals are what sscanf wrote, or (fields it didn't
        // match) never written: junk read: [ebp-0x10..-4] never written;
        // 0 (brief 2026-10-24) in the shim's zeroed frame
        let word: u32 = h.read(args);
        h.write(entry + 4, word);
        let half: u16 = h.read(args + 4);
        h.write(entry + 8, half);
        for (k, off) in [0xc, 8, 4, 0].into_iter().enumerate() {
            let half: u16 = h.read(locals + off);
            h.write(entry + 0xa + 2 * k as u32, half);
        }
    }

    /// `PhyreScene_findNamedTexture_00660420(this, name)` (`ret 4`): the
    /// tree at `+0x10d54` walked in order from its leftmost node; the
    /// value (`+0x10`, colour bit cleared) of the first node whose key
    /// (`+0xc`, likewise) equals `name`, or 0.
    pub fn scene_find_named(h: &mut impl Host, this: u32, name: u32) -> u32 {
        let head = this + 0x1_0d54;
        let first: u32 = h.read(head + 0xc);
        let mut node = if first == head || first == 0 {
            first
        } else {
            let mut n = first;
            loop {
                let left: u32 = h.read(n);
                if left == head {
                    break n;
                }
                n = left;
            }
        };
        if node == head {
            return 0;
        }
        let mut next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(head), &[node]);
        while node != head {
            let key = untag(h, node.wrapping_add(0xc));
            if same_string(h, name, key) {
                return untag(h, node.wrapping_add(0x10));
            }
            node = next;
            if next != head {
                next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(head), &[next]);
            }
        }
        0
    }

    /// A word with its low (colour) bit cleared, as the original takes it
    /// from the word and its first byte.
    fn untag(h: &mut impl Host, at: u32) -> u32 {
        let w: u32 = h.read(at);
        w.wrapping_sub(u32::from(h.read::<u8>(at)) & 1)
    }

    /// The inlined `strcmp(a, b) == 0`.
    fn same_string(h: &mut impl Host, a: u32, b: u32) -> bool {
        let mut k = 0u32;
        loop {
            let x: u8 = h.read(a.wrapping_add(k));
            if x != h.read::<u8>(b.wrapping_add(k)) {
                return false;
            }
            if x == 0 {
                return true;
            }
            k += 1;
        }
    }

    /// `PhyreScene_setResolutionScale_00652910(this)`: the window size
    /// into `+0x10dd0` (width) and `+0x10dd4` (height); then, by the
    /// aspect ratio `w / h` (f32), a wide window's width becomes
    /// `ftol(h / divisor * g_Vu0FixedScale)`, any other's height
    /// `ftol(w * factor * divisor)`. `eax`: that `ftol`.
    #[allow(clippy::cast_precision_loss)] // fild then fstp m32: the same rounding
    pub fn scene_set_resolution_scale(h: &mut impl Host, this: u32) -> u32 {
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let width: u32 = h.read(app.wrapping_add(0x26c));
        h.write(this + 0x1_0dd0, width);
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let fw = h.read::<i32>(this + 0x1_0dd0) as f32;
        let height: u32 = h.read(app.wrapping_add(0x270));
        h.write(this + 0x1_0dd4, height);
        let fh = height as i32 as f32;
        let ratio = (f64::from(fw) / f64::from(fh)) as f32;
        if f64::from(ratio) > ASPECT_LIMIT.get(h) {
            let v = f64::from(fh) / RESOLUTION_DIVISOR.get(h) * VU0_FIXED_SCALE.get(h);
            let r = ftol(h, v);
            h.write(this + 0x1_0dd0, r);
            r
        } else {
            let v = f64::from(fw) * RESOLUTION_FACTOR.get(h) * RESOLUTION_DIVISOR.get(h);
            let r = ftol(h, v);
            h.write(this + 0x1_0dd4, r);
            r
        }
    }

    /// `BrokenScreenEffect_renderPass_006d55b0(this, i, a, b)` (`ret 0xc`;
    /// frame: the colour (1, 1, 1, 1) at `ebp - 0x10`): the render target
    /// made when missing (`+0x74`), `+0x75` set; pass `i`'s first object
    /// (`[[+0x38] + 4i]`) draws with `(a, b)` and gets the white colour and
    /// the source texture (`[this]`, 0 when flagged); the second object
    /// draws with `(a, r)`, `r` the first draw's result (its `eax`).
    pub fn broken_render_pass(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        i: u32,
        a: u32,
        b: u32,
    ) -> u32 {
        if h.read::<u8>(this + 0x74) == 0 {
            h.call_sym(BROKEN_CREATE_RENDER_TARGET, Conv::Thiscall(this), &[]);
        }
        let colour = ebp.wrapping_sub(0x10);
        let one = 1f32.to_bits();
        for k in 0..4 {
            h.write(colour + 4 * k, one);
        }
        let objects: u32 = h.read(this + 0x38);
        let x50: u32 = h.read(this + 0x50);
        h.write(this + 0x75, 1u8);
        let x2c: u32 = h.read(this + 0x2c);
        let pair: u32 = h.read(objects.wrapping_add(4 * i));
        let x44: u32 = h.read(this + 0x44);
        let first: u32 = h.read(pair);
        let x40: u32 = h.read(this + 0x40);
        let r = h.call_sym(
            BROKEN_PASS_6D3FB0,
            Conv::Thiscall(first),
            &[x40, x44, x2c, x50, a, b],
        );
        let first = pass_object(h, this, i, 0);
        h.call_sym(BROKEN_SET_COLOUR, Conv::Thiscall(first), &[colour]);
        let source: u32 = h.read(this);
        let texture = if h.read::<u8>(source.wrapping_add(4)) == 0 {
            h.read(source)
        } else {
            0
        };
        let first = pass_object(h, this, i, 0);
        h.call_sym(
            BROKEN_SET_TEXTURE_6D4540,
            Conv::Thiscall(first),
            &[texture, 0],
        );
        let second = pass_object(h, this, i, 4);
        let x50: u32 = h.read(this + 0x50);
        let x2c: u32 = h.read(this + 0x2c);
        let x44: u32 = h.read(this + 0x44);
        let x40: u32 = h.read(this + 0x40);
        h.call_sym(
            BROKEN_PASS_6D3FB0,
            Conv::Thiscall(second),
            &[x40, x44, x2c, x50, a, r],
        )
    }

    fn pass_object(h: &mut impl Host, this: u32, i: u32, which: u32) -> u32 {
        let objects: u32 = h.read(this + 0x38);
        let pair: u32 = h.read(objects.wrapping_add(4 * i));
        h.read(pair.wrapping_add(which))
    }

    /// `graphicUpdateShadowMap(out, a1, .., a11)` `0x00644a00` (frame: the
    /// 16-byte result at `ebp - 0x14`): `ShadowManager::updateShadowMap`
    /// into the frame (`a4`, `a8`, `a9` pass through the x87 as floats),
    /// the 16 bytes it returns a pointer to copied to `out`. Returns
    /// `out`.
    pub fn update_shadow_map(h: &mut impl Host, ebp: u32, out: u32, a: [u32; 11]) -> u32 {
        let result = ebp.wrapping_sub(0x14);
        let f = |bits: u32| round_trip(f32::from_bits(bits)).to_bits();
        let args = [
            result,
            a[0],
            a[1],
            a[2],
            f(a[3]),
            a[4],
            a[5],
            a[6],
            f(a[7]),
            f(a[8]),
            a[9],
            a[10],
        ];
        let manager = instance(h, SHADOW_MANAGER_INSTANCE);
        let p = h.call_sym(SHADOW_UPDATE_MAP, Conv::Thiscall(manager), &args);
        for k in 0..4 {
            let w: u32 = h.read(p.wrapping_add(4 * k));
            h.write(out.wrapping_add(4 * k), w);
        }
        out
    }
}
