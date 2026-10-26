//! The render-side glue, eighth batch (M4 port round 3, list
//! `h3-glue-render`): the scene's cluster-object release loops, the VFX
//! dynamic-geometry pool, sampler set-up, the broken-screen draw, the
//! character instantiation queue, and the frame-passing functions round 1
//! left (file names unified into the frame, scene cursors, the cloth
//! batch counters), which take their frame at the original's `ebp` as in
//! [`crate::glue5`].
//!
//! The functions with a security cookie leave it out: the frame word is
//! the function's own, nothing reads it after the return.

use ffx_bridge::Global;

/// `Phyre_PFramework_PApplication_GetApplication`.
pub const GET_APPLICATION: u32 = crate::glue::GET_APPLICATION;
/// `Phyre_PCluster_deleteObject(this, object)`.
pub const CLUSTER_DELETE_OBJECT: u32 = crate::glue5::CLUSTER_DELETE_OBJECT;
/// `Phyre_PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `Phyre_PhyreMalloc_0042fb80(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `operator_new(n)` (cdecl).
pub const OPERATOR_NEW: u32 = crate::glue6::OPERATOR_NEW;
/// `crt_memset_thunk(p, c, n)` (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `Phyre_PCamera_updateViewMatrices(this)`.
pub const CAMERA_UPDATE_VIEW: u32 = 0x0067_20c0;
/// `Array_Reserve_00667f80(this, n)` and `Array_Reserve_00668280`.
pub const ARRAY_RESERVE_667F80: u32 = 0x0066_7f80;
pub const ARRAY_RESERVE_668280: u32 = 0x0066_8280;
/// The sampler state creation (`0x0058cf80`, thiscall on the
/// description).
pub const SAMPLER_CREATE: u32 = crate::glue6::SAMPLER_CREATE;
/// `BrokenScreenEffect_setConstant_006d46a0(this, 4 words)` and
/// `_006d4710`.
pub const BROKEN_SET_CONSTANT_A0: u32 = 0x006d_46a0;
pub const BROKEN_SET_CONSTANT_A4: u32 = 0x006d_4710;
/// `Phyre_PRendering_PRendererBase_renderMeshInstance(this, 4 words)`.
pub const RENDER_MESH_INSTANCE: u32 = crate::glue6::RENDER_MESH_INSTANCE;
/// `Ch_IsLoadingClothFile(chr)` (cdecl).
pub const CH_IS_LOADING_CLOTH: u32 = 0x0082_a100;
/// `graphicCharacterRegistHelper_00637880(a, chr, c, d)` (cdecl).
pub const CHR_REGIST_HELPER: u32 = 0x0063_7880;
/// `g_GraphicCharacterLoading`: the `PArray` of pending instantiations.
pub const CHARACTER_LOADING: u32 = 0x00cc_b6d4;
/// `PArray_PushInto` on the pending array (`0x00637e30`, thiscall).
pub const PUSH_INTO_637E30: u32 = 0x0063_7e30;
/// `strstr`'s import slot (cdecl).
pub const STRSTR: u32 = 0x00b0_c378;
/// `strncpy`-style copy's import slot (`(dst, src, n)`, cdecl).
pub const STRNCPY: u32 = 0x00b0_c368;
/// The markers `graphicLevelAnalysisName` cuts the name between.
pub const LEVEL_MARK_START: u32 = 0x00b4_1b1c;
pub const LEVEL_MARK_END: u32 = 0x00b4_1b24;
/// `graphicLevelSetName(name)` (cdecl).
pub const LEVEL_SET_NAME: u32 = 0x0064_1db0;
/// `report_rangecheckfailure()` (does not return).
pub const RANGE_CHECK_FAILURE: u32 = 0x0094_9b02;
/// `FUN_00652910(this)` (the scene's set-up `0x006656e0` calls first).
pub const SCENE_652910: u32 = 0x0065_2910;
/// `Instance_006dc800_Get()` and the method `0x006ddbf0` on it.
pub const INSTANCE_6DC800: u32 = 0x006d_c8a0;
pub const INSTANCE_6DDBF0: u32 = 0x006d_dbf0;
/// `PostProcessManager_GetInstance()`.
pub const POST_PROCESS_INSTANCE: u32 = crate::glue2::POST_PROCESS_INSTANCE;
/// `SetFields12400_00693be0(this, 4 words)`.
pub const POST_SET_FIELDS_12400: u32 = 0x0069_3be0;
/// `PostProcessManager_SetByte123b3(this, b)` and `SetByte123b0`.
pub const POST_SET_BYTE_123B3: u32 = 0x0069_85b0;
pub const POST_SET_BYTE_123B0: u32 = 0x0069_83f0;
/// `PostProcessTweakUtil_GetInstance()` and
/// `initWithSceneContext(this, context)`.
pub const TWEAK_INSTANCE: u32 = 0x0069_2bf0;
pub const TWEAK_INIT: u32 = 0x0069_38d0;
/// `PhyreScene_loadCutbufferShader(this)`.
pub const SCENE_LOAD_CUTBUFFER_SHADER: u32 = 0x0066_4440;
/// `g_LoadingScreen` and `LoadingScreen_initialize(this)`.
pub const LOADING_SCREEN: Global<u32> = Global::new(0x00ce_517c);
pub const LOADING_SCREEN_INITIALIZE: u32 = 0x006e_4720;
/// `g_VFXRenderDataTable`.
pub const VFX_RENDER_DATA_TABLE: Global<u32> = crate::glue4::VFX_RENDER_DATA_TABLE;
/// `ClassVFXRenderDataTable_GetDynamicGeometryByInstance(this, lo, hi)`.
pub const VFX_GET_DYN_GEO: u32 = 0x0069_f620;
/// `ClassVFXRenderDataTable_Remove_0069fc10(this, lo, hi)`.
pub const VFX_REMOVE: u32 = 0x0069_fc10;
/// `PhyreSceneCursor_Seek3(this)` and the class the post-process
/// shader-bin check looks for.
pub const CURSOR_SEEK3: u32 = crate::glue6::CURSOR_SEEK3;
pub const POST_LOOKUP_CLASS: u32 = crate::glue6::POST_LOOKUP_CLASS;
/// `PApplication_FixupClusters(&cluster, n)` (cdecl).
pub const FIXUP_CLUSTERS: u32 = crate::glue5::FIXUP_CLUSTERS;
/// `fiosUnifyFilename(name, out, size)` (cdecl).
pub const FIOS_UNIFY_FILENAME: u32 = 0x0067_9820;
/// The persistent VFX textures' two file names.
pub const PERSISTENT_VFX_NAME_A: u32 = 0x00b4_3a80;
pub const PERSISTENT_VFX_NAME_B: u32 = 0x00b4_3ad0;
/// `PhyreScene_loadVFXTexture(this, 3 words)`.
pub const SCENE_LOAD_VFX_TEXTURE: u32 = 0x0066_5770;
/// `EighthAtLeastOne(n, 0)` (stdcall).
pub const EIGHTH_AT_LEAST_ONE: u32 = 0x006d_b870;
/// `batchSetClothVertexBuffers(0, &start, &end)` (cdecl).
pub const BATCH_SET_CLOTH_VERTEX_BUFFERS: u32 = 0x0063_65f0;
/// The cloth character table: a count, then the instances.
pub const CLOTH_COUNT: Global<u32> = Global::new(0x00cc_b250);
pub const CLOTH_TABLE: u32 = 0x00cc_b254;

/// The values `0x0065efd0` accepts.
pub const LISTED_65EFD0: [u32; 8] = [7, 8, 0x20, 0x28, 0x2e, 0x30, 0x32, 0x195];

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::glue::bridge::instance;
    use ffx_graphic::host::{Conv, Host};

    /// Calls the import whose slot is at `slot`; its `eax`.
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let at = h.rebase(slot);
        let f: u32 = h.read(at);
        h.call(f, Conv::Cdecl, args)
    }

    /// `PhyreScene_releaseClusterObjects_00658f40(this)`: for each of the
    /// `+0xfb24` slots holding an object (`+0xfb2c`), its reference count
    /// (`+0xfb20`) drops by one; at 0 or below the object is deleted from
    /// its cluster (`+0xfb38`) and the slot cleared (object 0, count -1,
    /// cluster 0).
    pub fn scene_release_cluster_objects(h: &mut impl Host, this: u32) {
        let mut i = 0u32;
        while i < h.read::<u32>(this + 0xfb24) {
            let at = 4 * i;
            let objects: u32 = h.read(this + 0xfb2c);
            if h.read::<u32>(objects.wrapping_add(at)) != 0 {
                let counts: u32 = h.read(this + 0xfb20);
                let n = h.read::<i32>(counts.wrapping_add(at)).wrapping_sub(1);
                h.write(counts.wrapping_add(at), n);
                let counts: u32 = h.read(this + 0xfb20);
                if h.read::<i32>(counts.wrapping_add(at)) <= 0 {
                    let objects: u32 = h.read(this + 0xfb2c);
                    let object: u32 = h.read(objects.wrapping_add(at));
                    if object != 0 {
                        let clusters: u32 = h.read(this + 0xfb38);
                        let cluster: u32 = h.read(clusters.wrapping_add(at));
                        h.call_sym(CLUSTER_DELETE_OBJECT, Conv::Thiscall(cluster), &[object]);
                    }
                    let objects: u32 = h.read(this + 0xfb2c);
                    h.write(objects.wrapping_add(at), 0u32);
                    let counts: u32 = h.read(this + 0xfb20);
                    h.write(counts.wrapping_add(at), u32::MAX);
                    let clusters: u32 = h.read(this + 0xfb38);
                    h.write(clusters.wrapping_add(at), 0u32);
                }
            }
            i += 1;
        }
    }

    /// `0x0065efd0(v)` (stdcall): whether `v` is one of
    /// [`LISTED_65EFD0`]; `eax` 1 or 0.
    #[must_use]
    pub fn is_listed_65efd0(v: u32) -> u32 {
        u32::from(LISTED_65EFD0.contains(&v))
    }

    /// `PhyreScene_releasePoolArray_0065a3a0(this, array, cluster)`
    /// (`ret 8`): an array of the scene's pool (cluster `this + 0x12d4`,
    /// slot `(array - [+0xfb7c]) / 8` up to 3000) is handed back: its use
    /// byte (`[+0xfb88]`) cleared, the lowest free slot (`+0xfb8c`)
    /// lowered, its owned storage freed and the array emptied. Anything
    /// else is deleted from `cluster` (when not 0).
    pub fn scene_release_pool_array(h: &mut impl Host, this: u32, array: u32, cluster: u32) {
        if cluster == this.wrapping_add(0x12d4) {
            let base: u32 = h.read(this + 0xfb7c);
            let slot = (array.wrapping_sub(base) as i32) >> 3;
            if slot as u32 <= 0xbb7 {
                let used: u32 = h.read(this + 0xfb88);
                h.write(used.wrapping_add(slot as u32), 0u8);
                if slot < h.read::<i32>(this + 0xfb8c) {
                    h.write(this + 0xfb8c, slot);
                }
                let word: u32 = h.read(array);
                if word >> 31 == 0 && word & 0x7fff_ffff > 1 {
                    let data: u32 = h.read(array.wrapping_add(4));
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
                }
                h.write(array.wrapping_add(4), 0u32);
                h.write(array, 0u32);
                return;
            }
        }
        if array != 0 {
            h.call_sym(CLUSTER_DELETE_OBJECT, Conv::Thiscall(cluster), &[array]);
        }
    }

    /// `ClassVFXRenderDataTable_GetVFXDynamicGeometrySegmentFromPool(this,
    /// &segment, &data)` `0x0069f7f0` (`ret 8`): with both outputs and a
    /// free slot (`+0x48` below 4000), the slot's 0x6c-byte segment
    /// (`+0x3c`, zeroed) and 256-byte data (`+0x40`, first byte 0), its
    /// use byte (`+0x44`) set, and `+0x48` moved to the next unused slot.
    /// 1, or 0.
    pub fn vfx_segment_from_pool(h: &mut impl Host, this: u32, out: u32, data: u32) -> u32 {
        if out == 0 || data == 0 {
            return 0;
        }
        let next: i32 = h.read(this + 0x48);
        if next >= 4000 {
            return 0;
        }
        let segments: u32 = h.read(this + 0x3c);
        h.write(out, (next as u32).wrapping_mul(0x6c).wrapping_add(segments));
        let next: u32 = h.read(this + 0x48);
        let blocks: u32 = h.read(this + 0x40);
        h.write(data, (next << 8).wrapping_add(blocks));
        let segment: u32 = h.read(out);
        h.call_sym(MEMSET, Conv::Cdecl, &[segment, 0, 0x6c]);
        let block: u32 = h.read(data);
        h.write(block, 0u8);
        let used: u32 = h.read(this + 0x44);
        let next: u32 = h.read(this + 0x48);
        h.write(next.wrapping_add(used), 1u8);
        let mut k = h.read::<i32>(this + 0x48).wrapping_add(1);
        if k < 4000 {
            let used: u32 = h.read(this + 0x44);
            while h.read::<u8>((k as u32).wrapping_add(used)) != 0 {
                k += 1;
                if k >= 4000 {
                    break;
                }
            }
        }
        h.write(this + 0x48, k);
        1
    }

    /// `PhyreScene_clearArrays_00659000(this)`: the arrays at `+0xfb48`
    /// and `+0xfb54` emptied (released first when both their count and
    /// capacity are negative), `+0xfb60..+0xfb78` zeroed. Returns 0.
    pub fn scene_clear_arrays(h: &mut impl Host, this: u32) -> u32 {
        for (array, reserve) in [
            (this + 0xfb48, ARRAY_RESERVE_667F80),
            (this + 0xfb54, ARRAY_RESERVE_668280),
        ] {
            if h.read::<i32>(array) < 0 && h.read::<i32>(array + 4) < 0 {
                h.call_sym(reserve, Conv::Thiscall(array), &[0]);
            }
            h.write(array, 0u32);
        }
        for k in 0..7 {
            h.write(this + 0xfb60 + 4 * k, 0u32);
        }
        0
    }

    /// `PhyreScene_updateCameras_00671be0(this)`: each of the scene's
    /// cameras (`+0xdc`, `+0x654`, and `+0x238` when `+0x10d8c` is 1) gets
    /// its view matrix (`+0xfa04`, `+0x7b0`, `+0x10d94`) at `+0xb4` and
    /// `updateViewMatrices` on its camera part (`+4`).
    pub fn scene_update_cameras(h: &mut impl Host, this: u32) {
        for (camera, view) in [(0xdc, 0xfa04), (0x654, 0x7b0)] {
            update_camera(h, this, camera, view);
        }
        if h.read::<u32>(this + 0x1_0d8c) == 1 {
            update_camera(h, this, 0x238, 0x1_0d94);
        }
    }

    fn update_camera(h: &mut impl Host, this: u32, camera: u32, view: u32) {
        let c: u32 = h.read(this + camera);
        if c == 0 {
            return;
        }
        h.write(c.wrapping_add(0xb4), this.wrapping_add(view));
        let c: u32 = h.read(this + camera);
        h.call_sym(CAMERA_UPDATE_VIEW, Conv::Thiscall(c.wrapping_add(4)), &[]);
    }

    /// `PRedBlackTree_findOrLink_006be960(this, node)` (`ret 4`): the
    /// node's colour word (`+8`) gets its low bit set, its parent and
    /// first child (`+0`, `+4`) the head; then down from the root (`+0xc`)
    /// by the unsigned key (`+0xc`): an equal key returns that node;
    /// otherwise the node is linked under the last one (parent bits kept
    /// in `+8`, as the left child `+0` when that node's key is greater)
    /// and 0 returned.
    pub fn tree_find_or_link(h: &mut impl Host, this: u32, node: u32) -> u32 {
        let colour: u32 = h.read(node + 8);
        let bit = u32::from(h.read::<u8>(node + 8)) & 1;
        h.write(node + 8, colour.wrapping_add(1 - bit));
        h.write(node, this);
        h.write(node + 4, this);
        let mut parent = this.wrapping_add(0xc);
        let mut right = 0u32;
        let mut at: u32 = h.read(parent);
        while at != this {
            let key: u32 = h.read(node.wrapping_add(0xc));
            let here: u32 = h.read(at.wrapping_add(0xc));
            parent = at;
            if here == key {
                return at;
            }
            right = u32::from(here < key);
            at = h.read(at.wrapping_add(4 * right));
        }
        let bit = u32::from(h.read::<u8>(node + 8)) & 1;
        h.write(node + 8, bit.wrapping_add(parent));
        h.write(parent.wrapping_add(4 * right), node);
        0
    }

    /// `BrokenScreenEffect_render_006d44c0(this)`: the constants `+0xa0`
    /// (the colour at `+0x30`) and `+0xa4` (1, 1, 0, 0), then the mesh
    /// instance `+0x24` drawn by the renderer `+0xc` (its pass `+0x13c`).
    /// Returns 0.
    pub fn broken_render(h: &mut impl Host, this: u32) -> u32 {
        let colour: [u32; 4] = [
            h.read(this + 0x30),
            h.read(this + 0x34),
            h.read(this + 0x38),
            h.read(this + 0x3c),
        ];
        h.call_sym(BROKEN_SET_CONSTANT_A0, Conv::Thiscall(this), &colour);
        let one = 1f32.to_bits();
        h.call_sym(
            BROKEN_SET_CONSTANT_A4,
            Conv::Thiscall(this),
            &[one, one, 0, 0],
        );
        let renderer: u32 = h.read(this + 0xc);
        let mesh: u32 = h.read(this + 0x24);
        let pass: u32 = h.read(renderer.wrapping_add(0x13c));
        let segment: u32 = h.read(mesh.wrapping_add(0x10));
        h.call_sym(
            RENDER_MESH_INSTANCE,
            Conv::Thiscall(renderer),
            &[mesh, segment, 0, pass],
        );
        0
    }

    /// `SamplerPair_init_006b3f40(this)`: the two sampler descriptions at
    /// `+0x14` and `+0x38`, each unless flagged (bit 7 of its `+0x1c`):
    /// filter 5 and the address modes (1, 1, 1 for the first; 1, 2, 2,
    /// 2 for the second), then created. `eax`: the second creation's.
    pub fn sampler_pair_init(h: &mut impl Host, this: u32) -> u32 {
        sampler_desc(h, this + 0x14, 1);
        h.call_sym(SAMPLER_CREATE, Conv::Thiscall(this + 0x14), &[]);
        sampler_desc(h, this + 0x38, 2);
        h.call_sym(SAMPLER_CREATE, Conv::Thiscall(this + 0x38), &[])
    }

    fn sampler_desc(h: &mut impl Host, d: u32, mode: u8) {
        if !flagged(h, d) {
            h.write(d, 5u8);
            h.write(d + 1, 1u8);
        }
        if !flagged(h, d) {
            h.write(d + 2, mode);
            h.write(d + 3, mode);
        }
        if !flagged(h, d) {
            h.write(d + 4, mode);
        }
    }

    /// Bit 7 of a sampler description's `+0x1c` (reread before each
    /// group of stores, as the original does).
    fn flagged(h: &mut impl Host, d: u32) -> bool {
        (h.read::<u32>(d + 0x1c) >> 7) & 1 != 0
    }

    /// `graphicCharacterRegistToInstantiate(a, chr, c, d)` `0x0063d370`
    /// (`slots`: the caller's argument words): a character whose cloth is
    /// loaded (`+0x1c` set, `Ch_IsLoadingClothFile` 0) is registered at
    /// once (`graphicCharacterRegistHelper`, its `eax`); otherwise a
    /// 16-byte request `(a, chr, c, d)` (or 0) goes into the `chr`
    /// argument slot and is appended to `g_GraphicCharacterLoading`
    /// through a new buffer (`PArray_PushInto`'s `eax`). `None` for a null
    /// `chr` (nothing done; `eax` is the caller's).
    pub fn character_regist_to_instantiate(
        h: &mut impl Host,
        slots: u32,
        a: u32,
        chr: u32,
        c: u32,
        d: u32,
    ) -> Option<u32> {
        if chr == 0 {
            return None;
        }
        let loading = h.call_sym(CH_IS_LOADING_CLOTH, Conv::Cdecl, &[chr]);
        if h.read::<u8>(chr + 0x1c) != 0 && loading == 0 {
            return Some(h.call_sym(CHR_REGIST_HELPER, Conv::Cdecl, &[a, chr, c, d]));
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x10]);
        if p != 0 {
            h.write(p, a);
            h.write(p + 8, c);
            h.write(p + 4, chr);
            h.write(p + 0xc, d);
        }
        h.write(slots + 4, p);
        let array = h.rebase(CHARACTER_LOADING);
        let n = (h.read::<u32>(array) & 0x7fff_ffff).wrapping_add(1);
        let buffer = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n << 2, 4]);
        Some(h.call_sym(
            PUSH_INTO_637E30,
            Conv::Thiscall(array),
            &[buffer, slots + 4],
        ))
    }

    /// `graphicLevelAnalysisName(path)` `0x00641d00` (frame: a 30-byte
    /// name at `ebp - 0x24`): the text between the two markers, when both
    /// are found, copied into the frame and set as the level name; a name
    /// of 30 bytes or more ends the process (`report_rangecheckfailure`).
    pub fn level_analysis_name(h: &mut impl Host, ebp: u32, path: u32) {
        if path == 0 {
            return;
        }
        let mark = h.rebase(LEVEL_MARK_START);
        let start = import(h, STRSTR, &[path, mark]);
        if start == 0 {
            return;
        }
        let mark = h.rebase(LEVEL_MARK_END);
        let end = import(h, STRSTR, &[path, mark]);
        if end == 0 {
            return;
        }
        let len = end.wrapping_sub(start).wrapping_sub(4);
        let name = ebp.wrapping_sub(0x24);
        import(h, STRNCPY, &[name, start.wrapping_add(4), len]);
        if len >= 0x1e {
            h.call_sym(RANGE_CHECK_FAILURE, Conv::Cdecl, &[]);
            return;
        }
        h.write(name.wrapping_add(len), 0u8);
        h.call_sym(LEVEL_SET_NAME, Conv::Cdecl, &[name]);
    }

    /// `PhyreScene_initPostProcess_006656e0(this)`: `0x00652910`, the
    /// `0x006dc800` instance's `0x006ddbf0`, the scene context
    /// (`+0xcc` = the application `+0x50`), the post-process manager
    /// (`+0x10cc`) set up from `+0x10dd0`/`+0x10dd4` (on success its bytes
    /// `0x123b3` = 1, `0x123b0` = 0), the tweak utility, the cut-buffer
    /// shader, then `LoadingScreen_initialize` (its `eax`).
    pub fn scene_init_post_process(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(SCENE_652910, Conv::Thiscall(this), &[]);
        let inst = instance(h, INSTANCE_6DC800);
        h.call_sym(INSTANCE_6DDBF0, Conv::Thiscall(inst), &[]);
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.write(this + 0xcc, app.wrapping_add(0x50));
        let post = instance(h, POST_PROCESS_INSTANCE);
        let context: u32 = h.read(this + 0xcc);
        let b: u32 = h.read(this + 0x1_0dd4);
        h.write(this + 0x10cc, post);
        let a: u32 = h.read(this + 0x1_0dd0);
        let r = h.call_sym(
            POST_SET_FIELDS_12400,
            Conv::Thiscall(post),
            &[a, b, context, 0],
        );
        if r == 0 {
            let post: u32 = h.read(this + 0x10cc);
            h.call_sym(POST_SET_BYTE_123B3, Conv::Thiscall(post), &[1]);
            let post: u32 = h.read(this + 0x10cc);
            h.call_sym(POST_SET_BYTE_123B0, Conv::Thiscall(post), &[0]);
        }
        // the context is pushed before the getter (which takes nothing)
        // and consumed by initWithSceneContext
        let context: u32 = h.read(this + 0xcc);
        let tweak = instance(h, TWEAK_INSTANCE);
        h.call_sym(TWEAK_INIT, Conv::Thiscall(tweak), &[context]);
        h.call_sym(SCENE_LOAD_CUTBUFFER_SHADER, Conv::Thiscall(this), &[]);
        let screen = LOADING_SCREEN.get(h);
        h.call_sym(LOADING_SCREEN_INITIALIZE, Conv::Thiscall(screen), &[])
    }

    /// `graphicVFXDestroy(lo, hi)` `0x00645280`: for the effect instance
    /// `(lo, hi)` (not 0), its dynamic geometry's part count (1, or the
    /// render data's `+0xc4` when `+0xc0` is set); each part `k`'s
    /// instance `(lo, hi) + 10000 k` (64-bit) gets virtual `+0x10` on its
    /// geometry (when it has one) and is removed from the table.
    pub fn vfx_destroy(h: &mut impl Host, lo: u32, hi: u32) {
        if lo | hi == 0 {
            return;
        }
        let table = VFX_RENDER_DATA_TABLE.get(h);
        let geo = h.call_sym(VFX_GET_DYN_GEO, Conv::Thiscall(table), &[lo, hi]);
        if geo == 0 {
            return;
        }
        let mut parts = 1i32;
        let data: u32 = h.read(geo + 0x24);
        if data != 0 && h.read::<u8>(data + 0xc0) != 0 {
            parts = i32::from(h.read::<u16>(data + 0xc4));
        }
        let base = (u64::from(hi) << 32) | u64::from(lo);
        let mut k = 0i32;
        while parts > 0 {
            let id = base.wrapping_add(i64::from(k).cast_unsigned());
            let (id_lo, id_hi) = (id as u32, (id >> 32) as u32);
            let table = VFX_RENDER_DATA_TABLE.get(h);
            let part = h.call_sym(VFX_GET_DYN_GEO, Conv::Thiscall(table), &[id_lo, id_hi]);
            if part != 0 {
                h.call_virtual(part, 4, &[]);
            }
            let table = VFX_RENDER_DATA_TABLE.get(h);
            h.call_sym(VFX_REMOVE, Conv::Thiscall(table), &[id_lo, id_hi]);
            k = k.wrapping_add(10000);
            parts -= 1;
        }
    }

    /// `PostProcessManager_checkShaderBin_00693540(this, i)` (`ret 4`;
    /// frame: an 8-word cursor at `ebp - 0x20`): shader bin `i`'s cluster
    /// (`+4 + 4i`) fixed up, then searched for the class at `0x00ca5ff0`
    /// as in `post_lookup`. 0 when found, else 0x13.
    pub fn post_check_shader_bin(h: &mut impl Host, ebp: u32, this: u32, i: u32) -> u32 {
        let slot = this.wrapping_add(i.wrapping_add(1).wrapping_mul(4));
        if h.read::<u32>(slot) == 0 {
            return 0x13;
        }
        h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[slot, 1]);
        let cluster: u32 = h.read(slot);
        let list = cluster.wrapping_add(0x1c);
        let first: u32 = h.read(list);
        let first = if first == list { 0 } else { first };
        let cursor = ebp.wrapping_sub(0x20);
        for k in 0..5 {
            h.write(cursor + 4 * k, 0u32);
        }
        h.write(cursor + 0x14, list);
        h.write(cursor + 0x18, first);
        let class = h.rebase(POST_LOOKUP_CLASS);
        h.write(cursor + 0x1c, class);
        h.call_sym(CURSOR_SEEK3, Conv::Thiscall(cursor), &[]);
        if h.read::<u32>(cursor + 0xc) == 0 {
            return 0x13;
        }
        let object: u32 = h.read(cursor + 0x10);
        let base: u32 = h.read(cursor);
        if object.wrapping_add(base) == 0 {
            return 0x13;
        }
        0
    }

    /// `PhyreScene_loadPersistentVFXTexture(this)` `0x00665660` (frame:
    /// two 256-byte names at `ebp - 0x104` and `ebp - 0x204`): both file
    /// names unified into the frame, then `loadVFXTexture(0, name, 0)`
    /// with the first, and with the second when the first returned 0.
    /// The last result.
    pub fn scene_load_persistent_vfx_texture(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let a = ebp.wrapping_sub(0x104);
        let b = ebp.wrapping_sub(0x204);
        let name = h.rebase(PERSISTENT_VFX_NAME_A);
        h.call_sym(FIOS_UNIFY_FILENAME, Conv::Cdecl, &[name, a, 0x100]);
        let name = h.rebase(PERSISTENT_VFX_NAME_B);
        h.call_sym(FIOS_UNIFY_FILENAME, Conv::Cdecl, &[name, b, 0x100]);
        let r = h.call_sym(SCENE_LOAD_VFX_TEXTURE, Conv::Thiscall(this), &[0, a, 0]);
        if r != 0 {
            return r;
        }
        h.call_sym(SCENE_LOAD_VFX_TEXTURE, Conv::Thiscall(this), &[0, b, r])
    }

    /// `graphicCharacterInstanceSetClothVertexBuffers()` `0x0063b740`
    /// (frame: the index at `ebp - 8`, the batch start at `ebp - 4`): the
    /// cloth characters (table `0x00ccb254`, count `0x00ccb250`) that are
    /// ready (set, `+0` set, `+8` 0, `+0xc` 2) go to
    /// `batchSetClothVertexBuffers(0, &start, &index)` in batches of
    /// `EighthAtLeastOne(count, 0)`.
    pub fn character_set_cloth_vertex_buffers(h: &mut impl Host, ebp: u32) {
        let index = ebp.wrapping_sub(8);
        let start = ebp.wrapping_sub(4);
        let count = CLOTH_COUNT.get(h);
        h.write(index, 0u32);
        let batch = h.call_sym(EIGHTH_AT_LEAST_ONE, Conv::Stdcall, &[count, 0]);
        let mut count = CLOTH_COUNT.get(h);
        let mut ready = 0u32;
        h.write(start, 0u32);
        let table = h.rebase(CLOTH_TABLE);
        let mut i = 0u32;
        while i < count {
            let c: u32 = h.read(table.wrapping_add(4 * i));
            if c != 0
                && h.read::<u32>(c) != 0
                && h.read::<u32>(c + 8) == 0
                && h.read::<u32>(c + 0xc) == 2
            {
                ready = ready.wrapping_add(1);
                if ready == batch {
                    h.call_sym(
                        BATCH_SET_CLOTH_VERTEX_BUFFERS,
                        Conv::Cdecl,
                        &[0, start, index],
                    );
                    count = CLOTH_COUNT.get(h);
                    ready = 0;
                    h.write(start, i + 1);
                }
            }
            i += 1;
            h.write(index, i);
        }
    }
}
