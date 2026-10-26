//! The render-side glue, sixth batch (M4 port round 2, list
//! `h3-glue-render`): the singleton creators `graphicInitialize` calls,
//! the scene's asset-loaded callbacks, the pool deletes of the dynamic
//! geometry, the opening screen, the free-list tree inserts and a few
//! more.
//!
//! The singleton creators and the sampler holder's constructor set up a
//! C++ exception frame (`fs:[0]`, the security cookie); nothing they call
//! throws on these paths, so the ports leave it out, as the M4 pilot did.
//! Frames passed to callees: as in [`crate::glue5`].

use ffx_bridge::Global;

pub use crate::glue::SCENE;
pub use crate::glue4::VFX_RENDER_DATA_TABLE;

/// `operator_new(n)` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;

/// A singleton `graphicInitialize` (and others) create on demand: its
/// global, the object's size and its constructor (thiscall, returns the
/// object).
#[derive(Debug, Clone, Copy)]
pub struct Singleton {
    pub global: Global<u32>,
    pub size: u32,
    pub ctor: u32,
}

/// `ClassVFXRenderDataTable_CreateInstance` `0x0069f4c0`.
pub const VFX_RENDER_DATA_TABLE_SINGLETON: Singleton = Singleton {
    global: VFX_RENDER_DATA_TABLE,
    size: 0x50,
    ctor: 0x0069_e140,
};
/// `BrokenScreenEffect_CreateInstance_006d4d30`.
pub const BROKEN_SCREEN_SINGLETON: Singleton = Singleton {
    global: Global::new(0x00cd_ec98),
    size: 0x78,
    ctor: 0x006d_4ba0,
};
/// `FreeCamera_CreateInstance_0042f260`.
pub const FREE_CAMERA_SINGLETON: Singleton = Singleton {
    global: Global::new(0x00cc_c830),
    size: 0x340,
    ctor: 0x0067_fe70,
};
/// `ClusterManager_CreateInstance_0069a060`.
pub const CLUSTER_MANAGER_SINGLETON: Singleton = Singleton {
    global: Global::new(0x00cc_ca4c),
    size: 0x6420,
    ctor: 0x0069_9ab0,
};
/// `VfxShaderManager_CreateInstance_006c1420`.
pub const VFX_SHADER_MANAGER_SINGLETON: Singleton = Singleton {
    global: Global::new(0x00cd_eb80),
    size: 0x9c,
    ctor: 0x006b_f4a0,
};
/// `Distortion_CreateInstance_006c2190`.
pub const DISTORTION_SINGLETON: Singleton = Singleton {
    global: Global::new(0x00cd_eb88),
    size: 0x80,
    ctor: 0x006c_2010,
};
/// `ShadowManager_CreateInstance_006c9500`.
pub const SHADOW_MANAGER_SINGLETON: Singleton = Singleton {
    global: Global::new(0x00cd_ec28),
    size: 0x4068,
    ctor: 0x006c_7d80,
};
/// `g_VFXRenderMap_00ccca70`: an 8-byte holder of a `std::map` head.
pub const VFX_RENDER_MAP: Global<u32> = Global::new(0x00cc_ca70);
/// The map's head node allocation (`0x006a0360`).
pub const MAP_BUY_HEAD_NODE: u32 = 0x006a_0360;

/// `AsyncLoadManager_sGetInstance`.
pub const ASYNC_LOAD_INSTANCE: u32 = crate::glue::ASYNC_LOAD_INSTANCE;
/// `PhyreScene_doOnFieldMapAssetLoaded(this, 7 words)`.
pub const SCENE_DO_ON_FIELD_MAP_ASSET_LOADED: u32 = 0x0065_af60;
/// `AssetListReader_getAssetCount(this)`.
pub const ASSET_LIST_COUNT: u32 = 0x006f_4170;
/// `PhyreScene_doOnAllFieldMapAssetsLoaded(this, 4 words)`.
pub const SCENE_DO_ON_ALL_FIELD_MAP_ASSETS_LOADED: u32 = 0x0065_a6c0;
/// `Phyre_PFramework_PApplication_GetApplication`.
pub const GET_APPLICATION: u32 = crate::glue::GET_APPLICATION;
/// `PApplication_FixupClusters(&cluster, n)` (cdecl).
pub const FIXUP_CLUSTERS: u32 = crate::glue5::FIXUP_CLUSTERS;
/// The scene's VFX texture registration (`0x00656ae0`, `ret 0xc`).
pub const SCENE_REGIST_VFX_TEXTURE: u32 = 0x0065_6ae0;
/// The PS2 data loader's mutex.
pub const PS2_LOAD_MUTEX: Global<u32> = Global::new(0x00cc_b6d0);
/// The PS2 data loader's pending list: count, (capacity), data.
pub const PS2_LOAD_LIST: u32 = 0x00cc_b6e0;
/// `sa_read_callback(entry)` (cdecl).
pub const SA_READ_CALLBACK: u32 = 0x0076_e640;
/// The pending list's reset (`0x00638240`, thiscall, `ret 4`).
pub const PS2_LOAD_LIST_RESET: u32 = 0x0063_8240;
/// `WaitForSingleObject`'s import slot.
pub const WAIT_FOR_SINGLE_OBJECT: u32 = crate::glue::WAIT_FOR_SINGLE_OBJECT;
/// `ReleaseMutex`'s import slot.
pub const RELEASE_MUTEX: u32 = crate::glue::RELEASE_MUTEX;
/// `Phyre_PRendering_PRenderer_beginScene(this, flags)`.
pub const RENDERER_BEGIN_SCENE: u32 = 0x005b_5e30;
/// `Phyre_PRendering_PRendererBase_renderMeshInstance(this, 4 words)`.
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
/// `Phyre_PRendering_PRenderer_endScene(this, flags)`.
pub const RENDERER_END_SCENE: u32 = 0x005b_6950;
/// The scene render pass (an address in `.data`).
pub const PASS_SCENE: u32 = crate::glue4::PASS_SCENE;
/// `PSamplerStateD3D11_ctor(this)`.
pub const SAMPLER_STATE_CTOR: u32 = 0x0058_3b20;
/// The 56-byte pool objects' destructor (`0x00433af0`, thiscall).
pub const POOL56_DTOR: u32 = 0x0043_3af0;
/// `Phyre_PCluster_deleteObject(this, object)`.
pub const CLUSTER_DELETE_OBJECT: u32 = crate::glue5::CLUSTER_DELETE_OBJECT;
/// `hashU(string)` (cdecl).
pub const HASH_U: u32 = 0x0071_6570;
/// `"%s"`-style message with the level name (stdout).
pub const LEVEL_NAME_MESSAGE: u32 = 0x00b4_1af4;
/// The message with the level name's hash.
pub const LEVEL_HASH_MESSAGE: u32 = 0x00b4_1b04;
/// `__iob_func`'s import slot (cdecl).
pub const IOB_FUNC: u32 = crate::glue4::IOB_FUNC;
/// `fprintf`'s import slot (cdecl).
pub const FPRINTF: u32 = crate::glue4::FPRINTF;
/// `PostProcessManager_GetInstance`.
pub const POST_PROCESS_INSTANCE: u32 = crate::glue2::POST_PROCESS_INSTANCE;
/// `PostProcessManager_setHDAOSetting(this, name)`.
pub const POST_SET_HDAO_SETTING: u32 = 0x0069_85d0;
/// `PhyreScene_setCurrentLevelNameHash(this, hash)`.
pub const SCENE_SET_LEVEL_NAME_HASH: u32 = 0x0066_cf90;
/// `PhyreScene_setCurrentLevelName(this, name)`.
pub const SCENE_SET_LEVEL_NAME: u32 = 0x0066_cf80;
/// `PhyreSceneCursor_Seek3(this)` (`0x0065ec40`).
pub const CURSOR_SEEK3: u32 = 0x0065_ec40;
/// The class the post-process lookup's cursor matches (`.data`).
pub const POST_LOOKUP_CLASS: u32 = 0x00ca_5ff0;
/// `Array_Reserve_00668300(this, n)` (8-byte entries).
pub const ARRAY_RESERVE_668300: u32 = 0x0066_8300;
/// `IggyPlayerOpen_006e93f0(path, 4 floats)` (cdecl).
pub const IGGY_PLAYER_OPEN: u32 = 0x006e_93f0;
/// The opening screen's Iggy file.
pub const OPENING_PATH: u32 = 0x00b4_3c5c;
/// The Iggy callback registration (`0x00a28e90`, `ret 8`).
pub const IGGY_SET_CALLBACK: u32 = 0x00a2_8e90;
/// The callback's name.
pub const OPENING_CALLBACK_NAME: u32 = 0x00b4_3c90;
/// The callback (`0x00652400`).
pub const OPENING_CALLBACK: u32 = 0x0065_2400;
/// `Phyre_PRendering_PRendererBase_setClearColor(this, 4 floats)`.
pub const RENDERER_SET_CLEAR_COLOR: u32 = 0x005b_ada0;
/// `g_PhyreSceneOpeningPlaying` (byte).
pub const OPENING_PLAYING: Global<u8> = Global::new(0x00cc_b9c2);
/// The constant the broken screen's colour fade subtracts from.
pub const BROKEN_FADE_BASE: Global<f64> = Global::new(0x00b4_99c0);
/// `Phyre_PFreeList_allocateNewBlock(this, n)`.
pub const FREE_LIST_ALLOCATE_BLOCK: u32 = 0x0049_c8d0;
/// `Phyre_PFreeList_deallocate(this, node)`.
pub const FREE_LIST_DEALLOCATE: u32 = 0x0049_c980;
/// `Phyre_PRedBlackTree_postInsertFinalize(this, node)`.
pub const TREE_POST_INSERT: u32 = 0x0057_4980;
/// `PRedBlackTree_insertNew_0065fd50`'s insert (`0x0064c020`).
pub const TREE_INSERT_64C020: u32 = 0x0064_c020;
/// `PRedBlackTree_insertNew_006c0810`'s insert (`0x006be960`).
pub const TREE_INSERT_6BE960: u32 = 0x006b_e960;
/// `g_IggyMenuManager`.
pub const IGGY_MENU_MANAGER: Global<u32> = crate::glue2::IGGY_MENU_MANAGER;
/// `IggyMenuManager_Render_00a267f0(this)`.
pub const IGGY_MENU_RENDER: u32 = 0x00a2_67f0;
/// `Phyre_PRendering_PRenderer_flip(this)`.
pub const RENDERER_FLIP: u32 = 0x005b_6af0;
/// `graphicProcessWindowsMessage()`.
pub const PROCESS_WINDOWS_MESSAGE: u32 = 0x0064_2860;
/// `ExitProcess`'s import slot (stdcall).
pub const EXIT_PROCESS: u32 = 0x00b0_c1c8;
/// `SleepThreadInMillisecond(ms)` (cdecl).
pub const SLEEP_MS: u32 = 0x006f_2d20;
/// `PhyreScene_doDeleteMeshInstances(this)`.
pub const SCENE_DO_DELETE_MESH_INSTANCES: u32 = 0x0065_a620;
/// The sampler state descriptions `PSamplerStates_init_006f5470` sets up
/// (`.data`), and their creation (`0x0058cf80`, thiscall).
pub const SAMPLER_DESC_A: u32 = 0x00ce_8208;
pub const SAMPLER_DESC_B: u32 = 0x00ce_822c;
pub const SAMPLER_CREATE: u32 = 0x0058_cf80;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::glue::bridge::{instance, on_global};
    use ffx_bridge::x87::round_trip;
    use ffx_graphic::host::{Conv, Host};

    /// Calls the import whose slot is at `slot`; its `eax`.
    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let at = h.rebase(slot);
        let f: u32 = h.read(at);
        h.call(f, conv, args)
    }

    /// The singleton creators (`ClassVFXRenderDataTable_CreateInstance`
    /// and the others above): without an instance, `operator new(size)`
    /// and the constructor (0 when the allocation fails). Their `eax` is
    /// the new object, 0, or frame junk (the cookie) when the instance
    /// exists; no caller reads it.
    pub fn create_singleton(h: &mut impl Host, s: Singleton) {
        if s.global.get(h) != 0 {
            return;
        }
        new_singleton(h, s);
    }

    /// Creates the singleton; the new object (or 0).
    fn new_singleton(h: &mut impl Host, s: Singleton) -> u32 {
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[s.size]);
        let obj = if p == 0 {
            0
        } else {
            h.call_sym(s.ctor, Conv::Thiscall(p), &[])
        };
        s.global.set(h, obj);
        obj
    }

    /// `FreeCamera_CreateInstance_0042f260()`: as [`create_singleton`],
    /// but it loads the global into `eax` first, so it returns the
    /// instance (its callers use it as the getter).
    pub fn free_camera_instance(h: &mut impl Host) -> u32 {
        let camera = FREE_CAMERA_SINGLETON.global.get(h);
        if camera != 0 {
            return camera;
        }
        new_singleton(h, FREE_CAMERA_SINGLETON)
    }

    /// `VFXRenderMap_CreateInstance_0069f440()`: an 8-byte holder (`+0`
    /// the map's head node, `+4` its size 0).
    pub fn create_vfx_render_map(h: &mut impl Host) {
        if VFX_RENDER_MAP.get(h) != 0 {
            return;
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[8]);
        if p != 0 {
            h.write(p, 0u32);
            h.write(p + 4, 0u32);
            let head = h.call_sym(MAP_BUY_HEAD_NODE, Conv::Thiscall(p), &[]);
            h.write(p, head);
        }
        VFX_RENDER_MAP.set(h, p);
    }

    /// `PhyreScene_onFieldMapAssetLoaded(asset, task)` `0x00666630`: the
    /// task `(scene, a, b, list, reader, c, flag)` passes on to
    /// `doOnFieldMapAssetLoaded`; once the list's count (`[list]`)
    /// reaches the reader's asset count, `doOnAllFieldMapAssetsLoaded`.
    /// `eax`: the count, or the last callee's.
    pub fn scene_on_field_map_asset_loaded(h: &mut impl Host, asset: u32, task: u32) -> u32 {
        instance(h, ASYNC_LOAD_INSTANCE);
        let b: u32 = h.read(task + 8);
        let flag = u32::from(h.read::<u32>(task + 0x18) != 0);
        let reader: u32 = h.read(task + 0x10);
        let list: u32 = h.read(task + 0xc);
        let a: u32 = h.read(task + 4);
        let scene: u32 = h.read(task);
        let c: u32 = h.read(task + 0x14);
        h.call_sym(
            SCENE_DO_ON_FIELD_MAP_ASSET_LOADED,
            Conv::Thiscall(scene),
            &[asset, a, b, list, reader, c, flag],
        );
        let loaded: u32 = h.read(list);
        let count = h.call_sym(ASSET_LIST_COUNT, Conv::Thiscall(reader), &[]);
        if loaded != count {
            return count;
        }
        h.call_sym(
            SCENE_DO_ON_ALL_FIELD_MAP_ASSETS_LOADED,
            Conv::Thiscall(scene),
            &[a, b, list, reader],
        )
    }

    /// `PhyreScene_onVfxTextureLoaded_00666880(cluster, task)` (frame: the
    /// cluster at `ebp + 8`): the task `(id, a, b, c)`;
    /// `FixupClusters(&cluster, 1)`, then the scene's arrays `+0x60`
    /// (1), `+0x3c` (`a`) and `+0x54` (`b`) at `id`, and the registration
    /// `(id, c, a)` (its `eax`).
    pub fn scene_on_vfx_texture_loaded(
        h: &mut impl Host,
        ebp: u32,
        cluster: u32,
        task: u32,
    ) -> u32 {
        h.write(ebp + 8, cluster);
        instance(h, ASYNC_LOAD_INSTANCE);
        let id: u32 = h.read(task);
        let a: u32 = h.read(task + 4);
        let b: u32 = h.read(task + 8);
        let c: u32 = h.read(task + 0xc);
        h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[ebp + 8, 1]);
        let at = id.wrapping_mul(4);
        let scene = SCENE.get(h);
        let flags: u32 = h.read(scene + 0x60);
        h.write(flags.wrapping_add(at), 1u32);
        let scene = SCENE.get(h);
        let names: u32 = h.read(scene + 0x3c);
        h.write(names.wrapping_add(at), a);
        let scene = SCENE.get(h);
        let sizes: u32 = h.read(scene + 0x54);
        h.write(sizes.wrapping_add(at), b);
        let scene = SCENE.get(h);
        h.call_sym(SCENE_REGIST_VFX_TEXTURE, Conv::Thiscall(scene), &[id, c, a])
    }

    /// `graphicProcessPs2DataLoadingCallback()` `0x006427f0`: under the
    /// loader's mutex, `sa_read_callback` for each pending entry, then
    /// the list emptied (and reset when both its count and capacity are
    /// negative). Returns 0.
    pub fn process_ps2_data_loading_callback(h: &mut impl Host) -> u32 {
        let mutex = PS2_LOAD_MUTEX.get(h);
        import(
            h,
            WAIT_FOR_SINGLE_OBJECT,
            Conv::Stdcall,
            &[mutex, 0xffff_ffff],
        );
        let list = h.rebase(PS2_LOAD_LIST);
        let mut count: u32 = h.read(list);
        let mut i = 0u32;
        while i < count {
            let data: u32 = h.read(list + 8);
            let entry: u32 = h.read(data.wrapping_add(4 * i));
            h.call_sym(SA_READ_CALLBACK, Conv::Cdecl, &[entry]);
            count = h.read(list);
            i += 1;
        }
        if (count as i32) < 0 && h.read::<i32>(list + 4) < 0 {
            h.call_sym(PS2_LOAD_LIST_RESET, Conv::Thiscall(list), &[0]);
        }
        let mutex = PS2_LOAD_MUTEX.get(h);
        h.write(list, 0u32);
        import(h, RELEASE_MUTEX, Conv::Stdcall, &[mutex]);
        0
    }

    /// `PhyreScene_RenderDTTVFX(this)` `0x006531b0`: on the renderer
    /// (`+0xd0`), unless `beginScene(0)` fails: the scene pass, each mesh
    /// instance of the list (`+0x3854` count, `+0x385c` data) rendered
    /// with its `+0x10` and the renderer's `+0x13c`, then `endScene(1)`.
    pub fn scene_render_dtt_vfx(h: &mut impl Host, this: u32) -> u32 {
        let renderer: u32 = h.read(this + 0xd0);
        let r = h.call_sym(RENDERER_BEGIN_SCENE, Conv::Thiscall(renderer), &[0]);
        if r != 0 {
            return r;
        }
        let renderer: u32 = h.read(this + 0xd0);
        let pass = h.rebase(PASS_SCENE);
        h.write(renderer.wrapping_add(0x48), pass);
        let mut i = 0u32;
        while i < h.read::<u32>(this + 0x3854) {
            let data: u32 = h.read(this + 0x385c);
            let mesh: u32 = h.read(data.wrapping_add(4 * i));
            if mesh != 0 {
                let renderer: u32 = h.read(this + 0xd0);
                let extra: u32 = h.read(renderer.wrapping_add(0x13c));
                let m10: u32 = h.read(mesh.wrapping_add(0x10));
                h.call_sym(
                    RENDER_MESH_INSTANCE,
                    Conv::Thiscall(renderer),
                    &[mesh, m10, 0, extra],
                );
            }
            i += 1;
        }
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(RENDERER_END_SCENE, Conv::Thiscall(renderer), &[1])
    }

    /// `SamplerHolder_ctor_006f5ae0(this, owner)` (`ret 4`): three words 0,
    /// a `PSamplerStateD3D11` at `+0xc`, the owner at `+0x30`, `+0x34` 0;
    /// returns `this`.
    pub fn sampler_holder_ctor(h: &mut impl Host, this: u32, owner: u32) -> u32 {
        for k in 0..3 {
            h.write(this + 4 * k, 0u32);
        }
        h.call_sym(SAMPLER_STATE_CTOR, Conv::Thiscall(this + 0xc), &[]);
        h.write(this + 0x30, owner);
        h.write(this + 0x34, 0u32);
        this
    }

    /// The pool index of `object` from `base` as the original divides
    /// (`magic` and `shift`: by 56 or 28, signed, rounded toward 0), and
    /// what that leaves in `eax` (the low half of its multiply).
    fn pool_index(object: u32, base: u32, magic: u32, shift: u32) -> (i32, u32) {
        let d = object.wrapping_sub(base) as i32;
        let product = i64::from(magic as i32) * i64::from(d);
        let low = product as u32;
        let high = ((product >> 32) as i32).wrapping_add(d) >> shift;
        (high.wrapping_add((high as u32 >> 31) as i32), low)
    }

    /// `PhyreScene_deletePoolObject56_0065a430(this, object, cluster)`
    /// (`ret 8`): an object of the scene's 56-byte pool (from `+0xfb90`,
    /// used bytes `[+0xfb9c]`, lowest free `+0xfba0`) frees its slot and
    /// runs `0x00433af0` on it; others `cluster->deleteObject(object)`.
    pub fn scene_delete_pool_object56(
        h: &mut impl Host,
        this: u32,
        object: u32,
        cluster: u32,
    ) -> u32 {
        let mut eax = this + 0x12d4;
        if cluster == eax {
            let base: u32 = h.read(this + 0xfb90);
            let (i, low) = pool_index(object, base, 0x9249_2493, 5);
            eax = low;
            if i as u32 <= 0xbb7 {
                let used: u32 = h.read(this + 0xfb9c);
                h.write(used.wrapping_add(i as u32), 0u8);
                if i < h.read::<i32>(this + 0xfba0) {
                    h.write(this + 0xfba0, i);
                }
                return h.call_sym(POOL56_DTOR, Conv::Thiscall(object), &[]);
            }
        }
        if object != 0 {
            return h.call_sym(CLUSTER_DELETE_OBJECT, Conv::Thiscall(cluster), &[object]);
        }
        eax
    }

    /// `PhyreScene_deletePoolObject28_0065a1d0(this, object, cluster)`
    /// (`ret 8`): the 28-byte pool (from `+0xfba4`, used `[+0xfbb0]`,
    /// lowest free `+0xfbb4`); its objects run their virtual destructor
    /// (slot 0, 0); others `cluster->deleteObject(object + 4)`.
    pub fn scene_delete_pool_object28(
        h: &mut impl Host,
        this: u32,
        object: u32,
        cluster: u32,
    ) -> u32 {
        let mut eax = this + 0x12d4;
        if cluster == eax {
            let base: u32 = h.read(this + 0xfba4);
            let (i, low) = pool_index(object, base, 0x9249_2493, 4);
            eax = low;
            if i as u32 <= 0xbb7 {
                let used: u32 = h.read(this + 0xfbb0);
                h.write(used.wrapping_add(i as u32), 0u8);
                if i < h.read::<i32>(this + 0xfbb4) {
                    h.write(this + 0xfbb4, i);
                }
                return h.call_virtual(object, 0, &[0]);
            }
        }
        if object != 0 && object.wrapping_add(4) != 0 {
            return h.call_sym(
                CLUSTER_DELETE_OBJECT,
                Conv::Thiscall(cluster),
                &[object.wrapping_add(4)],
            );
        }
        eax
    }

    /// `graphicLevelSetName(name)` `0x00641db0`: prints the name and its
    /// `hashU`, sets the post-process HDAO setting for it, and the
    /// scene's level hash and name (its `eax`).
    pub fn level_set_name(h: &mut impl Host, name: u32) -> u32 {
        let hash = h.call_sym(HASH_U, Conv::Cdecl, &[name]);
        let message = h.rebase(LEVEL_NAME_MESSAGE);
        let iob = import(h, IOB_FUNC, Conv::Cdecl, &[]);
        import(
            h,
            FPRINTF,
            Conv::Cdecl,
            &[iob.wrapping_add(0x20), message, name],
        );
        let message = h.rebase(LEVEL_HASH_MESSAGE);
        let iob = import(h, IOB_FUNC, Conv::Cdecl, &[]);
        import(
            h,
            FPRINTF,
            Conv::Cdecl,
            &[iob.wrapping_add(0x20), message, hash],
        );
        let post = instance(h, POST_PROCESS_INSTANCE);
        h.call_sym(POST_SET_HDAO_SETTING, Conv::Thiscall(post), &[name]);
        on_global(h, SCENE, SCENE_SET_LEVEL_NAME_HASH, &[hash]);
        on_global(h, SCENE, SCENE_SET_LEVEL_NAME, &[name])
    }

    /// `PostProcessManager_lookup_006935f0(this, i)` (`ret 4`; frame: an
    /// 8-word cursor at `ebp - 0x20`): entry `+4 + 4i`'s list (`+0x1c`)
    /// searched for the class at `0x00ca5ff0` (`PhyreSceneCursor_Seek3`);
    /// the found object (`+0x10`, plus the cursor's base `+0`), or 0.
    pub fn post_lookup(h: &mut impl Host, ebp: u32, this: u32, i: u32) -> u32 {
        let entry: u32 = h.read(this.wrapping_add(i.wrapping_mul(4)).wrapping_add(4));
        if entry == 0 {
            return 0;
        }
        let list = entry.wrapping_add(0x1c);
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
            return 0;
        }
        let object: u32 = h.read(cursor + 0x10);
        let base: u32 = h.read(cursor);
        object.wrapping_add(base)
    }

    /// `PhyreScene_registClustersToAddToWorld(this, a, b)` `0x00668550`
    /// (`ret 8`): under the scene's mutex (`+0xfb3c`), appends `(a, b)` to
    /// the array at `+0x1188` (8-byte entries). `eax`: `ReleaseMutex`'s.
    pub fn scene_regist_clusters_to_add(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        let mutex: u32 = h.read(this + 0xfb3c);
        import(
            h,
            WAIT_FOR_SINGLE_OBJECT,
            Conv::Stdcall,
            &[mutex, 0xffff_ffff],
        );
        let array = this + 0x1188;
        let n = h.read::<i32>(array).wrapping_add(1);
        h.write(array, n);
        if n > h.read::<i32>(array + 4) {
            let grown = (n / 8).wrapping_add(n).wrapping_add(2);
            h.call_sym(ARRAY_RESERVE_668300, Conv::Thiscall(array), &[grown as u32]);
        }
        let n: u32 = h.read(array);
        let data: u32 = h.read(array + 8);
        let slot = data.wrapping_add(n.wrapping_mul(8)).wrapping_sub(8);
        if slot != 0 {
            h.write(slot, a);
            h.write(slot + 4, b);
        }
        let mutex: u32 = h.read(this + 0xfb3c);
        import(h, RELEASE_MUTEX, Conv::Stdcall, &[mutex])
    }

    /// `PhyreScene_beginRenderOpeningScreen(this)` `0x006579d0`: opens the
    /// opening's Iggy player (`+0x10dc8`; at 0,0 size 1,1); with one, its
    /// callback, a black clear colour and the "opening playing" byte.
    /// `eax`: `setClearColor`'s, or 0.
    pub fn scene_begin_render_opening_screen(h: &mut impl Host, this: u32) -> u32 {
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let one = 1f32.to_bits();
        let path = h.rebase(OPENING_PATH);
        let player = h.call_sym(IGGY_PLAYER_OPEN, Conv::Cdecl, &[path, 0, 0, one, one]);
        h.write(this + 0x0001_0dc8, player);
        if player == 0 {
            return 0;
        }
        let name = h.rebase(OPENING_CALLBACK_NAME);
        let callback = h.rebase(OPENING_CALLBACK);
        h.call_sym(IGGY_SET_CALLBACK, Conv::Thiscall(player), &[name, callback]);
        let r = h.call_sym(
            RENDERER_SET_CLEAR_COLOR,
            Conv::Thiscall(app.wrapping_add(0x124)),
            &[0, 0, 0, one],
        );
        OPENING_PLAYING.set(h, 1);
        r
    }

    /// `BrokenScreenEffect_setColour_006d4620(this, colour)` (`ret 4`):
    /// copies the 4 floats to `+0x30`; `v = K - (1 - a)^2` (`K` the double
    /// at `0x00b499c0`, `a` the alpha `+0x3c`) as a float, at most 1.
    /// Mode `+0` 1 stores `v` as the alpha; mode 2 also sets the colour
    /// to 1.
    pub fn broken_set_colour(h: &mut impl Host, this: u32, colour: u32) {
        for k in 0..4 {
            let v: f32 = h.read(colour + 4 * k);
            h.write(this + 0x30 + 4 * k, round_trip(v));
        }
        let alpha: f32 = h.read(this + 0x3c);
        let t = 1.0 - f64::from(alpha);
        let v = (BROKEN_FADE_BASE.get(h) - t * t) as f32;
        // fcom with 1: above it becomes 1 (NaN stays)
        let v = if v > 1.0 { 1.0 } else { v };
        match h.read::<u32>(this) {
            1 => h.write(this + 0x3c, v),
            2 => {
                for k in 0..3 {
                    h.write(this + 0x30 + 4 * k, 1f32);
                }
                h.write(this + 0x3c, v);
            }
            _ => {}
        }
    }

    /// `PRedBlackTree_insertNew_0065fd50` / `_006c0810(this, &key, &value)`
    /// (`ret 8`): a node from the free list (`+0x18`, refilled with
    /// `[+0x20]`; 0xe when that fails) linked to itself, holding `*key` and
    /// `*value`; `insert` (0x12 and the node returned to the list when it
    /// refuses) else `postInsertFinalize` and 0.
    pub fn tree_insert_new(h: &mut impl Host, insert: u32, this: u32, key: u32, value: u32) -> u32 {
        let free = this + 0x18;
        let mut node: u32 = h.read(free);
        if node == 0 {
            let n: u32 = h.read(this + 0x20);
            h.call_sym(FREE_LIST_ALLOCATE_BLOCK, Conv::Thiscall(free), &[n]);
            node = h.read(free);
            if node == 0 {
                return 0xe;
            }
        }
        let next: u32 = h.read(node);
        h.write(free, next);
        for k in 0..3 {
            h.write(node + 4 * k, node);
        }
        let k: u32 = h.read(key);
        h.write(node + 0xc, k);
        let v: u32 = h.read(value);
        h.write(node + 0x10, v);
        if h.call_sym(insert, Conv::Thiscall(this), &[node]) != 0 {
            h.call_sym(FREE_LIST_DEALLOCATE, Conv::Thiscall(free), &[node]);
            return 0x12;
        }
        h.call_sym(TREE_POST_INSERT, Conv::Thiscall(this), &[node]);
        0
    }

    /// `PhyreScene_renderOpeningScreen(this)` `0x0066b2e0`: an empty scene
    /// (3/1), the Iggy menus, `flip`; unless it or the renderer's virtual
    /// slot 2 fails: the application's slot 10, window messages (exits on
    /// `WM_QUIT`), a 33 ms sleep and the mesh deletes; 0. Else the
    /// failure.
    pub fn scene_render_opening_screen(h: &mut impl Host, this: u32) -> u32 {
        let sleep = h.rebase(SLEEP_MS);
        scene_render_opening_screen_with(h, this, sleep)
    }

    /// [`scene_render_opening_screen`] with its 33 ms sleep sent to
    /// `sleep` (a rebased `cdecl(ms)` address) instead of
    /// `SleepThreadInMillisecond`: the harness's fast mode patches that
    /// call in the original (`0x0066b345`) to move the game's clock
    /// instead of sleeping, and the hook passes the same target.
    pub fn scene_render_opening_screen_with(h: &mut impl Host, this: u32, sleep: u32) -> u32 {
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let renderer = app.wrapping_add(0x124);
        h.call_sym(RENDERER_BEGIN_SCENE, Conv::Thiscall(renderer), &[3]);
        h.call_sym(RENDERER_END_SCENE, Conv::Thiscall(renderer), &[1]);
        let menus = IGGY_MENU_MANAGER.get(h);
        if menus != 0 {
            h.call_sym(IGGY_MENU_RENDER, Conv::Thiscall(menus), &[]);
        }
        let r = h.call_sym(RENDERER_FLIP, Conv::Thiscall(renderer), &[]);
        if r != 0 {
            return r;
        }
        let r = h.call_virtual(renderer, 2, &[]);
        if r != 0 {
            return r;
        }
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.call_virtual(app, 10, &[]);
        if h.call_sym(PROCESS_WINDOWS_MESSAGE, Conv::Cdecl, &[]) != 0 {
            import(h, EXIT_PROCESS, Conv::Stdcall, &[0]);
        }
        h.call(sleep, Conv::Cdecl, &[0x21]);
        h.call_sym(SCENE_DO_DELETE_MESH_INSTANCES, Conv::Thiscall(this), &[]);
        0
    }

    /// One sampler description's defaults: unless the flag word's bit 7
    /// is set, its address modes `+2`/`+3` 2 (and, for the first, the
    /// filters `+0`/`+1` 1).
    fn sampler_defaults(h: &mut impl Host, desc: u32, flags: u32, filters_set: bool) -> bool {
        let set = h.read::<u32>(flags) >> 7 & 1 != 0;
        if !set {
            h.write(desc + 2, 2u8);
            h.write(desc + 3, 2u8);
            if filters_set {
                h.write(desc, 1u8);
                h.write(desc + 1, 1u8);
            }
        }
        set
    }

    /// `PSamplerStates_init_006f5470()`: the two sampler descriptions'
    /// defaults (flags `+0x1c`), each created (`0x0058cf80`); the second's
    /// filters `+0`/`+1` cleared unless its bit 7 is set. `eax`: the
    /// second creation's.
    pub fn sampler_states_init(h: &mut impl Host) -> u32 {
        let a = h.rebase(SAMPLER_DESC_A);
        sampler_defaults(h, a, a + 0x1c, true);
        h.call_sym(SAMPLER_CREATE, Conv::Thiscall(a), &[]);
        let b = h.rebase(SAMPLER_DESC_B);
        if !sampler_defaults(h, b, b + 0x1c, false) {
            h.write(b, 0u8);
            h.write(b + 1, 0u8);
        }
        h.call_sym(SAMPLER_CREATE, Conv::Thiscall(b), &[])
    }
}
