//! M5 stage 1, list `render`, batch 2: `graphicFinalize` and the
//! destructors and singleton releases it runs (post-process manager,
//! distortion, radial blur, broken screen, dynamic-geometry pools, the
//! delegation helper, the menu texture tree), the scene's exit and render
//! forwarders, the `PCluster` cursor and instance-list helpers, the
//! loader's request ring, and the character HSV switches.
//!
//! SEH frames and stack cookies of the destructors are left out: nothing
//! on their paths throws. Callees stay calls to the originals.

use ffx_bridge::Global;

/// `g_PhyreScene`.
pub const SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// `g_MenuTextureManager`.
pub const MENU_TEXTURE_MANAGER: Global<u32> = Global::new(0x00cc_c820);
/// `g_PostProcessManager`.
pub const POST_PROCESS_MANAGER: Global<u32> = Global::new(0x00cc_c8a0);
/// `g_TextureAnimationManager`.
pub const TEXTURE_ANIMATION_MANAGER: Global<u32> = Global::new(0x00cc_caa8);
/// `g_RenderDelegationHelper`.
pub const DELEGATIONS: Global<u32> = Global::new(0x00cc_ca98);
/// The delegation helper's mutex handle.
pub const DELEGATION_MUTEX: Global<u32> = Global::new(0x00cc_caa0);
/// The menu texture tree's "released" byte.
pub const MENU_TREE_RELEASED: Global<u8> = Global::new(0x00cc_ca54);
/// The menu texture tree (`Phyre::PRedBlackTree`): its head, the word at
/// `+0xc` (the first node's chain) and its free list `+0x18`.
pub const MENU_TREE: u32 = 0x00c3_419c;
/// The distortion, radial-blur, blur-ex, broken-screen-data and
/// specular-map singletons `graphicFinalize` deletes.
pub const DISTORTION: Global<u32> = Global::new(0x00cd_eb88);
pub const RADIAL_BLUR: Global<u32> = Global::new(0x00cd_eba0);
pub const BLUR_EX: Global<u32> = Global::new(0x00cd_ebfc);
pub const POST_EFFECT: Global<u32> = Global::new(0x00cd_ec04);
pub const SPECULAR_MAP_PARAMS: Global<u32> = Global::new(0x00cd_ec1c);
/// `g_DynGeoFillBuffer`.
pub const DYN_GEO_FILL: Global<u32> = Global::new(0x00cd_edf0);
/// `g_IggyMenuManager`.
pub const IGGY_MENU_MANAGER: Global<u32> = Global::new(0x0198_4c6c);
/// `g_LoadingScreen`.
pub const LOADING_SCREEN: Global<u32> = Global::new(0x00ce_517c);
/// The graphics-initialised word `graphicFinalize` tests and clears.
pub const GRAPHIC_INITIALISED: Global<u32> = Global::new(0x00cc_b45c);
/// The cloth job system `graphicFinalize` stops and deletes.
pub const CLOTH_JOBS: Global<u32> = Global::new(0x00cd_ede4);
/// `g_VFXRenderDataTable`.
pub const VFX_RENDER_DATA_TABLE: Global<u32> = Global::new(0x00cc_ca6c);
/// `g_ChrClothFlip`, an f32 `FUN_00637c10` passes as the HSV value.
pub const CHR_CLOTH_FLIP: u32 = 0x00b9_2340;

// ---- imports (slots) ----
pub const WAIT_FOR_SINGLE_OBJECT: u32 = 0x00b0_c0f8;
pub const RELEASE_MUTEX: u32 = 0x00b0_c1c0;
/// `KERNEL32!CloseHandle` (stdcall, 1 word).
pub const CLOSE_HANDLE: u32 = 0x00b0_c0dc;
/// `KERNEL32!ReleaseSemaphore` (stdcall, 3 words).
pub const RELEASE_SEMAPHORE: u32 = 0x00b0_c0f4;

// ---- callees ----
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `operator_delete_00630540(p)` (cdecl).
pub const OPERATOR_DELETE_2: u32 = 0x0063_0540;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const USER_FREE: u32 = 0x0068_7010;
pub const MENU_UNLOAD_EXTRA: u32 = 0x0067_ec50;
pub const TREE_SUCCESSOR: u32 = 0x0057_5160;
pub const TREE_ORPHAN_ALL: u32 = 0x0057_4970;
pub const TREE_CLEAR: u32 = 0x0057_4760;
pub const FREE_LIST_RESET: u32 = 0x0049_cc10;
pub const FREE_LIST_DTOR: u32 = 0x0049_c5b0;
pub const SCENE_DELETE_MATERIAL: u32 = 0x0065_a2c0;
pub const SCENE_DELETE_OBJECT: u32 = 0x0065_a250;
pub const SCENE_DELETE_POOL_28: u32 = 0x0065_a1d0;
pub const SCENE_RELEASE_POOL_ARRAY: u32 = 0x0065_a3a0;
pub const SCENE_DELETE_POOL_56: u32 = 0x0065_a430;
pub const SAMPLER_DTOR: u32 = 0x0058_5230;
pub const SAMPLER_RELEASE: u32 = 0x0059_c770;
pub const CLUSTER_DTOR: u32 = 0x0043_fa30;
pub const FXAA_DTOR: u32 = 0x009f_df60;
pub const POST_MEMBER_DTOR: u32 = 0x009f_de10;
pub const MLAA_DTOR: u32 = 0x009f_e2a0;
pub const POST_MANAGER_DTOR: u32 = 0x0069_2820;
pub const SET_DATA_TO_FREE_LIST: u32 = 0x006a_e1a0;
/// `ScreenFadeDelegations_processRenderDelegation`, the task the clear
/// keeps.
pub const SCREEN_FADE_TASK: u32 = 0x006a_e5b0;
pub const CLEAR_DELEGATIONS: u32 = 0x006a_df70;
pub const CLUSTER_MANAGER: u32 = 0x0069_a1a0;
pub const CLUSTER_RELEASE: u32 = 0x0069_bdb0;
pub const CLUSTER_HELD: u32 = 0x0069_b890;
pub const DISTORTION_RELEASE: u32 = 0x006c_2410;
pub const DISTORTION_TERMINATE: u32 = 0x006c_31c0;
pub const RADIAL_BLUR_MEMBER_DTOR: u32 = 0x0070_3070;
/// `code_006f7b10`, a jump to `RadialLine_destroyRadialLine`.
pub const RADIAL_LINE_DESTROY_THUNK: u32 = 0x006f_7b10;
pub const RENDER_TARGET_RELEASE: u32 = 0x0044_0ea0;
pub const RADIAL_BLUR_RELEASE: u32 = 0x006c_3cd0;
pub const BLUR_EX_RESET: u32 = 0x006c_40c0;
pub const BROKEN_SCREEN_RESET: u32 = 0x006d_5970;
pub const DYN_GEO_POOL_DTOR: u32 = 0x006d_e420;
pub const DYN_GEO_RELEASE_POOL: u32 = 0x006d_e860;
pub const SAVE_DATA_TTY_WRITE2: u32 = 0x0064_8850;
pub const SCENE_EXIT_RELEASE: u32 = 0x0065_b820;
pub const POST_COPY_BUFFER: u32 = 0x0069_2c40;
pub const SCENE_COPY_POST_BUFFER: u32 = 0x0065_82a0;
pub const SCENE_RENDER_CAPTURE: u32 = 0x0066_b130;
pub const IGGY_RENDER: u32 = 0x00a2_67f0;
pub const IGGY_EXIT: u32 = 0x00a2_6270;
pub const IGGY_MENU_DTOR: u32 = 0x00a2_8a40;
pub const ARRAY_DATA: u32 = 0x004d_45a0;
pub const COPY_ARRAY: u32 = 0x0063_5630;
pub const SET_PASS_REMAPPING: u32 = 0x004d_8960;
pub const CHANGE_TO_HSV_PASS: u32 = 0x0064_faa0;
pub const SET_HSV_PARAM: u32 = 0x0064_3340;
pub const ENABLE_HSV: u32 = 0x0064_0100;
/// The scene render pass the HSV switch remaps (`0x00c169d4`) and its HSV
/// pass (`0x00c16b44`).
pub const PASS_DEFAULT: u32 = 0x00c1_69d4;
pub const PASS_HSV: u32 = 0x00c1_6b44;
pub const CURSOR_IS_VALID: u32 = 0x0044_1e40;
pub const CURSOR_SET_NODE: u32 = 0x0044_1dc0;
pub const WORLD_REMOVE_LIST: u32 = 0x0044_3c20;
pub const WORLD_ADD_LIST: u32 = 0x0044_3320;
pub const MOVE_ASSETS: u32 = 0x0049_c730;
pub const LIST_REBIND: u32 = 0x0045_da20;
/// The Lua binding's push (`0x00436e80(L, object, class)`), its class
/// record, Lua's `0x0094ac00(L, 1)` and `lua_pushnil(L)` (cdecl).
pub const LUA_PUSH_CLASS_OBJECT: u32 = 0x0043_6e80;
pub const LUA_CLASS: u32 = 0x00c9_0b90;
pub const LUA_0094AC00: u32 = 0x0094_ac00;
pub const LUA_PUSHNIL: u32 = 0x0094_b730;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    fn byte(h: &impl Mem, at: u32) -> u8 {
        h.read(at)
    }

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    fn method(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    /// Deletes the singleton in `global` with its destructor `dtor`
    /// (thiscall) and `operator_delete`, clearing the global; nothing
    /// without one.
    fn delete_singleton(h: &mut impl Host, global: Global<u32>, dtor: u32) {
        let p = global.get(h);
        if p != 0 {
            method(h, dtor, p, &[]);
            cdecl(h, OPERATOR_DELETE, &[p]);
            global.set(h, 0);
        }
    }

    /// The cluster at `cluster`: destroyed (`Phyre_PCluster_dtor`),
    /// freed and the word cleared, when set.
    fn free_cluster(h: &mut impl Host, at: u32) {
        let c = word(h, at);
        if c != 0 {
            method(h, CLUSTER_DTOR, c, &[]);
            cdecl(h, PHYRE_FREE, &[c]);
            h.write(at, 0_u32);
        }
    }

    /// The pointer at `at`, when set, `operator_delete_00630540`'d and
    /// cleared.
    fn delete2(h: &mut impl Host, at: u32) {
        let p = word(h, at);
        if p != 0 {
            cdecl(h, OPERATOR_DELETE_2, &[p]);
            h.write(at, 0_u32);
        }
    }

    /// `graphicMenuUnloadExtraTexture()`: a tail jump to
    /// `MenuTextureManager_UnLoadExtraMenuTexture(g_MenuTextureManager)`.
    pub fn menu_unload_extra_texture(h: &mut impl Host) -> u32 {
        let m = MENU_TEXTURE_MANAGER.get(h);
        method(h, MENU_UNLOAD_EXTRA, m, &[])
    }

    /// `MenuTextureTree_release_006a1230(this)` -> 0: once (the byte
    /// `0x00ccca54`): every node of the menu texture tree from the end of
    /// the chain at `+0xc` on (in successor order), its buffer `+0xc`
    /// deleted and its material `+0x10` deleted from the cluster `this +
    /// 0x2c` (`PhyreScene_deleteMaterialInCluster(material, cluster, 0,
    /// 0)`); then the tree orphaned and its free list reset.
    pub fn menu_tree_release(h: &mut impl Host, this: u32) -> u32 {
        if MENU_TREE_RELEASED.get(h) != 0 {
            return 0;
        }
        let head = h.rebase(MENU_TREE);
        let first = word(h, head + 0xc);
        let mut node = first;
        if first != head && first != 0 {
            loop {
                let next = word(h, node);
                if next == head {
                    break;
                }
                node = next;
            }
        }
        if node != head {
            let mut next = method(h, TREE_SUCCESSOR, head, &[node]);
            while node != head {
                let buf = word(h, node + 0xc);
                if buf != 0 {
                    cdecl(h, OPERATOR_DELETE, &[buf]);
                }
                let scene = SCENE.get(h);
                let material = word(h, node + 0x10);
                let cluster = word(h, this + 0x2c);
                method(h, SCENE_DELETE_MATERIAL, scene, &[material, cluster, 0, 0]);
                node = next;
                if next != head {
                    next = method(h, TREE_SUCCESSOR, head, &[next]);
                }
            }
        }
        method(h, TREE_ORPHAN_ALL, head, &[]);
        method(h, FREE_LIST_RESET, head + 0x18, &[]);
        MENU_TREE_RELEASED.set(h, 1);
        0
    }

    /// `PostProcessManager_dtor_00692820(this)`: the members destroyed in
    /// reverse order (samplers `+0x123dc`, `+0x122f0`, `+0x122cc`, the
    /// cluster `+0x12334`, the tree `+0x1226c` with its free list, the FXAA
    /// `+0x1190`, the samplers `+0x1124`, `+0x1100`, the member `+0x140`,
    /// the MLAA `+0xb0`, the cluster `+0x3c`); the last destructor's
    /// `eax`.
    pub fn post_manager_dtor(h: &mut impl Host, this: u32) -> u32 {
        method(h, SAMPLER_DTOR, this + 0x1_23dc, &[]);
        method(h, CLUSTER_DTOR, this + 0x1_2334, &[]);
        method(h, SAMPLER_DTOR, this + 0x1_22f0, &[]);
        method(h, SAMPLER_DTOR, this + 0x1_22cc, &[]);
        let tree = this + 0x1_226c;
        method(h, TREE_ORPHAN_ALL, tree, &[]);
        method(h, FREE_LIST_RESET, tree + 0x18, &[]);
        method(h, FREE_LIST_DTOR, tree + 0x18, &[]);
        method(h, TREE_CLEAR, tree, &[]);
        method(h, TREE_CLEAR, tree, &[]);
        method(h, FXAA_DTOR, this + 0x1190, &[]);
        method(h, SAMPLER_DTOR, this + 0x1124, &[]);
        method(h, SAMPLER_DTOR, this + 0x1100, &[]);
        method(h, POST_MEMBER_DTOR, this + 0x140, &[]);
        method(h, MLAA_DTOR, this + 0xb0, &[]);
        method(h, CLUSTER_DTOR, this + 0x3c, &[])
    }

    /// `PostProcessManager_destroyInstance_00692b90()`.
    pub fn post_manager_destroy(h: &mut impl Host) {
        delete_singleton(h, POST_PROCESS_MANAGER, POST_MANAGER_DTOR);
    }

    /// `ClassDynamicGeometry_clear(this)`: virtual 4 first; the pooled
    /// objects `+0x10` (`deleteObjectInCluster`) and `+8` (pool 0x28)
    /// deleted and cleared; the material array `+0x14` (count low 31 bits,
    /// data past one element, else inline): with the per-material words
    /// `+0x18`/`+0x1c` each material deleted with them, the two blocks
    /// freed unless inline (`+0x31`, `+0x35`) and cleared; without, each
    /// with 0, 0; the array released; byte `+0x30` cleared; the pooled
    /// `+4` (pool 0x56) deleted; `+0x20` cleared.
    pub fn dyn_geo_clear(h: &mut impl Host, this: u32) {
        h.call_virtual(this, 4, &[]);
        for (off, f) in [(0x10, SCENE_DELETE_OBJECT), (8, SCENE_DELETE_POOL_28)] {
            let p = word(h, this + off);
            if p != 0 {
                let cluster = word(h, this + 0x2c);
                let scene = SCENE.get(h);
                method(h, f, scene, &[p, cluster]);
                h.write(this + off, 0_u32);
            }
        }
        let arr = word(h, this + 0x14);
        if arr != 0 {
            let per_material = word(h, this + 0x18) != 0 && word(h, this + 0x1c) != 0;
            let mut i = 0_u32;
            while i < word(h, word(h, this + 0x14)) & 0x7fff_ffff {
                let a = word(h, this + 0x14);
                let n = word(h, a) & 0x7fff_ffff;
                let data = if n > 1 {
                    word(h, a + 4)
                } else if n != 0 {
                    a + 4
                } else {
                    0
                };
                if per_material {
                    let m = word(h, data + 4 * i);
                    if m != 0 {
                        let x = word(h, word(h, this + 0x1c) + 4 * i);
                        let y = word(h, word(h, this + 0x18) + 4 * i);
                        let cluster = word(h, this + 0x2c);
                        let scene = SCENE.get(h);
                        method(h, SCENE_DELETE_MATERIAL, scene, &[m, cluster, y, x]);
                    }
                } else {
                    let m = word(h, data + 4 * i);
                    let cluster = word(h, this + 0x2c);
                    let scene = SCENE.get(h);
                    method(h, SCENE_DELETE_MATERIAL, scene, &[m, cluster, 0, 0]);
                }
                i += 1;
            }
            if per_material {
                for (off, inline) in [(0x18, 0x31), (0x1c, 0x35)] {
                    let b = word(h, this + off);
                    if b != this + inline {
                        cdecl(h, USER_FREE, &[b]);
                    }
                }
                h.write(this + 0x18, 0_u32);
                h.write(this + 0x1c, 0_u32);
            }
            let cluster = word(h, this + 0x2c);
            let scene = SCENE.get(h);
            let a = word(h, this + 0x14);
            method(h, SCENE_RELEASE_POOL_ARRAY, scene, &[a, cluster]);
            h.write(this + 0x14, 0_u32);
        }
        let p = word(h, this + 4);
        h.write(this + 0x30, 0_u8);
        if p != 0 {
            let cluster = word(h, this + 0x2c);
            let scene = SCENE.get(h);
            method(h, SCENE_DELETE_POOL_56, scene, &[p, cluster]);
            h.write(this + 4, 0_u32);
        }
        h.write(this + 0x20, 0_u32);
    }

    /// `RenderDelegationHelper_clearDelegations_006adf70(this; word)` (`ret
    /// 4`; the word is never read) -> `ReleaseMutex`'s result. Under the
    /// mutex every delegation but the screen fades (flags `0x1001` or task
    /// `ScreenFadeDelegations_processRenderDelegation`) is handed to
    /// `setDataToFreeList`; where that is negative its data `+4` and the
    /// delegation are deleted (`+8` cleared first); the slot is cleared.
    /// Then the 64 running pairs' busy words are cleared except the screen
    /// fades' (the pair's task is read unchecked).
    pub fn clear_delegations(h: &mut impl Host, this: u32) -> u32 {
        let m = DELEGATION_MUTEX.get(h);
        import(h, WAIT_FOR_SINGLE_OBJECT, &[m, 0xffff_ffff]);
        let fade = h.rebase(SCREEN_FADE_TASK);
        let mut i = 0;
        while i < word(h, this) & 0x7fff_ffff {
            let slot = word(h, this + 4) + 4 * i;
            let d = word(h, slot);
            if d != 0 && word(h, d + 0xc) != 0x1001 && word(h, d + 0x10) != fade {
                let r = method(h, SET_DATA_TO_FREE_LIST, this, &[d]);
                if r.cast_signed() < 0 {
                    let e = word(h, word(h, this + 4) + 4 * i);
                    if e != 0 {
                        let data = word(h, e + 4);
                        if data != 0 {
                            cdecl(h, OPERATOR_DELETE, &[data]);
                        }
                        h.write(e + 8, 0_u32);
                        cdecl(h, OPERATOR_DELETE, &[e]);
                    }
                }
                let slot = word(h, this + 4) + 4 * i;
                h.write(slot, 0_u32);
            }
            i += 1;
        }
        for j in 0..0x40 {
            let pair = this + 8 + 8 * j;
            let task = word(h, pair);
            if word(h, task + 0xc) != 0x1001 && word(h, task + 0x10) != fade {
                h.write(pair + 4, 0_u32);
            }
        }
        let m = DELEGATION_MUTEX.get(h);
        import(h, RELEASE_MUTEX, &[m])
    }

    /// `RenderDelegationHelper_dtor_006ad180(this)`: the delegations
    /// cleared ([`clear_delegations`] with -1), the array freed when owned
    /// (top bit of the count clear) and cleared.
    pub fn delegations_dtor(h: &mut impl Host, this: u32) {
        method(h, CLEAR_DELEGATIONS, this, &[0xffff_ffff]);
        if word(h, this) >> 31 == 0 {
            let data = word(h, this + 4);
            cdecl(h, PHYRE_FREE, &[data]);
        }
        h.write(this, 0_u32);
        h.write(this + 4, 0_u32);
    }

    /// `TextureAnimationManager_destroyInstance_006b0120()`: the samplers
    /// `+0x14` and `+0x38` released, the manager deleted (virtual 0 with
    /// 1, on the global read again) and the global cleared.
    pub fn texanim_destroy(h: &mut impl Host) {
        let m = TEXTURE_ANIMATION_MANAGER.get(h);
        if m == 0 {
            return;
        }
        method(h, SAMPLER_RELEASE, m + 0x14, &[]);
        method(h, SAMPLER_RELEASE, m + 0x38, &[]);
        let m = TEXTURE_ANIMATION_MANAGER.get(h);
        if m != 0 {
            h.call_virtual(m, 0, &[1]);
        }
        TEXTURE_ANIMATION_MANAGER.set(h, 0);
    }

    /// `VfxShaderManager_releaseClusters_006c1f20(this)`: every node of the
    /// manager's tree (head `this`, from the end of the chain at `+0xc`,
    /// in successor order) whose cluster `+0x10` the cluster manager holds
    /// is released; the tree orphaned; a tail jump to its free list's
    /// reset (`+0x18`), whose `eax` it returns.
    pub fn vfx_shader_release_clusters(h: &mut impl Host, this: u32) -> u32 {
        let first = word(h, this + 0xc);
        let mut node = first;
        if first != this && first != 0 {
            loop {
                let next = word(h, node);
                if next == this {
                    break;
                }
                node = next;
            }
        }
        if node != this {
            let mut next = method(h, TREE_SUCCESSOR, this, &[node]);
            while node != this {
                let c = word(h, node + 0x10);
                if c != 0 {
                    let m = cdecl(h, CLUSTER_MANAGER, &[]);
                    if method(h, CLUSTER_HELD, m, &[c]) & 0xff != 0 {
                        let m = cdecl(h, CLUSTER_MANAGER, &[]);
                        method(h, CLUSTER_RELEASE, m, &[c]);
                    }
                }
                node = next;
                if next != this {
                    next = method(h, TREE_SUCCESSOR, this, &[next]);
                }
            }
        }
        method(h, TREE_ORPHAN_ALL, this, &[]);
        method(h, FREE_LIST_RESET, this + 0x18, &[])
    }

    /// `Distortion_releaseBuffers_006c2410(this)`: the buffers `+0x74`,
    /// `+0x68`, `+0x6c`, `+0x70` deleted, the clusters `+0x14`, `+0x18`
    /// freed.
    pub fn distortion_release(h: &mut impl Host, this: u32) {
        for off in [0x74, 0x68, 0x6c, 0x70] {
            delete2(h, this + off);
        }
        free_cluster(h, this + 0x14);
        free_cluster(h, this + 0x18);
    }

    /// `Distortion_terminate(this)` -> 0: [`distortion_release`], the
    /// cluster `+0x1c`, the sampler `+0x44` released, the clusters
    /// `+0x14`, `+0x18` (again).
    pub fn distortion_terminate(h: &mut impl Host, this: u32) -> u32 {
        method(h, DISTORTION_RELEASE, this, &[]);
        free_cluster(h, this + 0x1c);
        method(h, SAMPLER_RELEASE, this + 0x44, &[]);
        free_cluster(h, this + 0x14);
        free_cluster(h, this + 0x18);
        0
    }

    /// `Distortion_destroyInstance_006c2210()`: the distortion terminated,
    /// its sampler `+0x44` destroyed, deleted, the global cleared.
    pub fn distortion_destroy(h: &mut impl Host) {
        let d = DISTORTION.get(h);
        if d == 0 {
            return;
        }
        method(h, DISTORTION_TERMINATE, d, &[]);
        method(h, SAMPLER_DTOR, d + 0x44, &[]);
        cdecl(h, OPERATOR_DELETE, &[d]);
        DISTORTION.set(h, 0);
    }

    /// `RadialLine_destroyRadialLine(this)`: the buffers `+0x60`, `+0x50`,
    /// `+0x54`, `+0x58`, `+0x5c`, `+0x64` deleted and cleared.
    pub fn radial_line_destroy(h: &mut impl Host, this: u32) {
        for off in [0x60, 0x50, 0x54, 0x58, 0x5c, 0x64] {
            delete2(h, this + off);
        }
    }

    /// `RadialBlur_release_006c3cd0(this)` -> 0: the members `+0x18`
    /// (`0x00703070`) and `+0x1c` (the radial line, through
    /// `code_006f7b10`) destroyed and deleted; the render target `+0x24`
    /// released, its cluster destroyed and freed, cleared.
    pub fn radial_blur_release(h: &mut impl Host, this: u32) -> u32 {
        for (off, dtor) in [
            (0x18, RADIAL_BLUR_MEMBER_DTOR),
            (0x1c, RADIAL_LINE_DESTROY_THUNK),
        ] {
            let p = word(h, this + off);
            if p != 0 {
                method(h, dtor, p, &[]);
                cdecl(h, OPERATOR_DELETE, &[p]);
                h.write(this + off, 0_u32);
            }
        }
        let t = word(h, this + 0x24);
        if t != 0 {
            method(h, RENDER_TARGET_RELEASE, t, &[]);
            let t = word(h, this + 0x24);
            if t != 0 {
                method(h, CLUSTER_DTOR, t, &[]);
                cdecl(h, PHYRE_FREE, &[t]);
            }
            h.write(this + 0x24, 0_u32);
        }
        0
    }

    /// `RadialBlur_destroyInstance_006c3550()`.
    pub fn radial_blur_destroy(h: &mut impl Host) {
        delete_singleton(h, RADIAL_BLUR, RADIAL_BLUR_RELEASE);
    }

    /// `BlurEx_destroyInstance_006c4a30()`: the effect `[0]` reset
    /// (`ResetAndConstruct_006c40c0`) and deleted, the holder deleted, the
    /// global cleared.
    pub fn blur_ex_destroy(h: &mut impl Host) {
        let holder = BLUR_EX.get(h);
        if holder == 0 {
            return;
        }
        let e = word(h, holder);
        if e != 0 {
            method(h, BLUR_EX_RESET, e, &[]);
            cdecl(h, OPERATOR_DELETE, &[e]);
            h.write(holder, 0_u32);
        }
        cdecl(h, OPERATOR_DELETE, &[holder]);
        BLUR_EX.set(h, 0);
    }

    /// `PostEffect_destroyInstance_006c50b0()`: the sampler `+0x68`
    /// released and destroyed, the cluster `+0` destroyed, deleted, the
    /// global cleared.
    pub fn post_effect_destroy(h: &mut impl Host) {
        let e = POST_EFFECT.get(h);
        if e == 0 {
            return;
        }
        method(h, SAMPLER_RELEASE, e + 0x68, &[]);
        method(h, SAMPLER_DTOR, e + 0x68, &[]);
        method(h, CLUSTER_DTOR, e, &[]);
        cdecl(h, OPERATOR_DELETE, &[e]);
        POST_EFFECT.set(h, 0);
    }

    /// `SpecularMapParamsManager_delete_006c57c0()` -> the delete's `eax`
    /// (0 without one): the manager deleted, the global left as it is.
    pub fn specular_map_params_delete(h: &mut impl Host) -> u32 {
        let p = SPECULAR_MAP_PARAMS.get(h);
        if p == 0 {
            return 0;
        }
        cdecl(h, OPERATOR_DELETE, &[p])
    }

    /// `BrokenScreenEffect_release_006d5a00(this)` -> 0: reset, the
    /// clusters `+0x54`, `+0x50`, `+0x3c` freed.
    pub fn broken_screen_release(h: &mut impl Host, this: u32) -> u32 {
        method(h, BROKEN_SCREEN_RESET, this, &[]);
        for off in [0x54, 0x50, 0x3c] {
            free_cluster(h, this + off);
        }
        0
    }

    /// `DynGeoPool_dtor_006de420(this)`: an owned block array `+0xc`
    /// (data `+0x10`) freed when it has data, cleared; the handle `+0x14`
    /// closed; then freed again when owned (top bit clear), cleared.
    pub fn dyn_geo_pool_dtor(h: &mut impl Host, this: u32) {
        let c = word(h, this + 0xc);
        if c != 0 {
            let d = word(h, this + 0x10);
            if d != 0 && c >> 31 == 0 {
                cdecl(h, PHYRE_FREE, &[d]);
            }
            h.write(this + 0x10, 0_u32);
            h.write(this + 0xc, 0_u32);
        }
        let handle = word(h, this + 0x14);
        import(h, CLOSE_HANDLE, &[handle]);
        if word(h, this + 0xc) >> 31 == 0 {
            let d = word(h, this + 0x10);
            cdecl(h, PHYRE_FREE, &[d]);
        }
        h.write(this + 0xc, 0_u32);
        h.write(this + 0x10, 0_u32);
    }

    /// `DynGeoMemManager_releasePool_006de860(this; i)` (`ret 4`): pool
    /// `[this + 4 + 4i]`'s first two buffers (`+0x10[k]` while `k < +8`
    /// and set): data `+4` deleted, `+8` cleared, handle `+0xc` closed,
    /// the buffer deleted; then the pool: the current pool `[this]`
    /// cleared when it is this one, destroyed ([`dyn_geo_pool_dtor`]) and
    /// deleted, the slot cleared.
    pub fn dyn_geo_release_pool(h: &mut impl Host, this: u32, i: u32) {
        let slot = this.wrapping_add(i.wrapping_mul(4)) + 4;
        for k in 0..2_u32 {
            let pool = word(h, slot);
            if pool == 0 || k.cast_signed() >= word(h, pool + 8).cast_signed() {
                continue;
            }
            let buf = word(h, word(h, pool + 0x10) + 4 * k);
            if buf == 0 {
                continue;
            }
            delete2(h, buf + 4);
            let handle = word(h, buf + 0xc);
            h.write(buf + 8, 0_u32);
            import(h, CLOSE_HANDLE, &[handle]);
            cdecl(h, OPERATOR_DELETE, &[buf]);
        }
        let pool = word(h, slot);
        if pool == 0 {
            return;
        }
        if word(h, this) == pool {
            h.write(this, 0_u32);
        }
        let pool = word(h, slot);
        if pool != 0 {
            method(h, DYN_GEO_POOL_DTOR, pool, &[]);
            cdecl(h, OPERATOR_DELETE, &[pool]);
        }
        h.write(slot, 0_u32);
    }

    /// `DynGeoMemManager_destroyInstance_006dec20()`: pool 0 released, the
    /// handle `+0xc` closed, the manager deleted, the global cleared.
    pub fn dyn_geo_destroy(h: &mut impl Host) {
        let m = DYN_GEO_FILL.get(h);
        if m == 0 {
            return;
        }
        method(h, DYN_GEO_RELEASE_POOL, m, &[0]);
        let handle = word(h, m + 0xc);
        import(h, CLOSE_HANDLE, &[handle]);
        cdecl(h, OPERATOR_DELETE, &[m]);
        DYN_GEO_FILL.set(h, 0);
    }

    /// `graphicFinalize()` -> 0: with the graphics up (`0x00ccb45c`,
    /// cleared), every manager and singleton the scene made is released
    /// and deleted in the original's order, the scene last; a log line
    /// (`SaveDataTtyWrite2(10, ...)`) before and after (another one when
    /// there was nothing to do).
    #[allow(clippy::too_many_lines)] // one call per manager
    pub fn graphic_finalize(h: &mut impl Host) -> u32 {
        tty(h, 0x00b4_1604);
        if GRAPHIC_INITIALISED.get(h) == 0 {
            tty(h, 0x00b4_1624);
            return 0;
        }
        let jobs = CLOTH_JOBS.get(h);
        GRAPHIC_INITIALISED.set(h, 0);
        method(h, 0x006d_be80, jobs, &[]);
        delete_singleton(h, CLOTH_JOBS, 0x006d_b730);
        cdecl(h, 0x0063_78b0, &[]);
        for f in [
            0x0067_1280,
            0x0067_10e0,
            0x0067_1140,
            0x0067_1180,
            0x0067_1200,
            0x0067_0920,
            0x0067_08e0,
            0x0067_11c0,
            0x0067_0ea0,
            0x0067_10a0,
            0x0067_1240,
            0x0065_9d20,
        ] {
            let scene = SCENE.get(h);
            method(h, f, scene, &[]);
        }
        cdecl(h, 0x006d_fdd0, &[]);
        let vfx = cdecl(h, 0x006c_14c0, &[]);
        method(h, 0x006c_1f20, vfx, &[]);
        cdecl(h, 0x006c_14a0, &[]);
        let table = cdecl(h, 0x0069_f6c0, &[]);
        method(h, 0x006a_11d0, table, &[]);
        for f in [0x0069_f580, 0x0068_db80, 0x0069_1b00] {
            cdecl(h, f, &[]);
        }
        let t = VFX_RENDER_DATA_TABLE.get(h);
        method(h, 0x0069_f000, t, &[]);
        for f in [
            0x0069_f5c0,
            0x0069_f540,
            0x0069_2b90,
            0x0069_2bc0,
            0x006b_4990,
            0x006b_cc70,
            0x006a_beb0,
        ] {
            cdecl(h, f, &[]);
        }
        let d = DELEGATIONS.get(h);
        if d != 0 {
            method(h, 0x006a_d180, d, &[]);
            cdecl(h, OPERATOR_DELETE, &[d]);
            DELEGATIONS.set(h, 0);
        }
        for f in [
            0x006a_6dd0,
            0x006a_6e00,
            0x006b_62d0,
            0x006b_6210,
            0x006b_62a0,
            0x006b_6240,
            0x006b_61e0,
            0x006b_60f0,
            0x006b_61b0,
            0x006b_6180,
            0x006b_6270,
            0x006b_0120,
            0x006d_7370,
            0x006c_2210,
            0x006c_50b0,
            0x006c_57c0,
            0x006c_4a30,
            0x006c_3550,
            0x006e_76f0,
            0x006d_4db0,
            0x006e_32a0,
        ] {
            cdecl(h, f, &[]);
        }
        let ls = LOADING_SCREEN.get(h);
        if ls != 0 {
            method(h, 0x0063_4790, ls, &[]);
            cdecl(h, OPERATOR_DELETE, &[ls]);
        }
        LOADING_SCREEN.set(h, 0);
        for f in [
            0x0068_0000,
            0x0067_b820,
            0x006e_54b0,
            0x006d_ec20,
            0x006e_56d0,
        ] {
            cdecl(h, f, &[]);
        }
        let m = cdecl(h, CLUSTER_MANAGER, &[]);
        method(h, 0x0069_a9c0, m, &[]);
        cdecl(h, 0x0069_a0e0, &[]);
        cdecl(h, 0x006d_c870, &[]);
        let scene = SCENE.get(h);
        if scene != 0 {
            method(h, 0x0063_4850, scene, &[]);
            cdecl(h, OPERATOR_DELETE, &[scene]);
        }
        SCENE.set(h, 0);
        tty(h, 0x00b4_1640);
        0
    }

    /// `PhyreScene_exitRelease_0065b820(scene)` -> 0: the samplers
    /// `+0x940` and the eight at `+0x964` (0x24 apart) released,
    /// `0x006f5450`, the Iggy menu `+0x10ddc` destroyed and deleted (and
    /// cleared), `TipSystem_DestroyInstance`, `0x006f7820`.
    pub fn scene_exit_release(h: &mut impl Host, scene: u32) -> u32 {
        method(h, SAMPLER_RELEASE, scene + 0x940, &[]);
        for k in 0..8 {
            method(h, SAMPLER_RELEASE, scene + 0x964 + 0x24 * k, &[]);
        }
        cdecl(h, 0x006f_5450, &[]);
        let menu = word(h, scene + 0x1_0ddc);
        if menu != 0 {
            method(h, IGGY_MENU_DTOR, menu, &[]);
            cdecl(h, OPERATOR_DELETE, &[menu]);
        }
        h.write(scene + 0x1_0ddc, 0_u32);
        cdecl(h, 0x006e_8700, &[]);
        cdecl(h, 0x006f_7820, &[]);
        0
    }

    /// `graphicExitScene_006401c0()` -> [`scene_exit_release`]'s result,
    /// after `0x00a26270`.
    pub fn graphic_exit_scene(h: &mut impl Host) -> u32 {
        let scene = SCENE.get(h);
        let r = method(h, SCENE_EXIT_RELEASE, scene, &[]);
        cdecl(h, IGGY_EXIT, &[]);
        r
    }

    /// `PhyreScene_copyPostBuffer_006582a0(scene)`: the post-process
    /// manager's `copyBuffer(scene +0xd0, texture of +0x8c, +0xac, 1, 0,
    /// 0)` (6 words, callee pops), its `eax`.
    pub fn scene_copy_post_buffer(h: &mut impl Host, scene: u32) -> u32 {
        let post = word(h, scene + 0x10cc);
        let t = word(h, post + 0x8c);
        let tex = if byte(h, t + 4) == 0 { word(h, t) } else { 0 };
        let dst = word(h, post + 0xac);
        let src = word(h, scene + 0xd0);
        method(h, POST_COPY_BUFFER, post, &[src, tex, dst, 1, 0, 0])
    }

    /// `graphicCopyPostBuffer_00639d10()`: on `g_PhyreScene` (a tail jump).
    pub fn copy_post_buffer(h: &mut impl Host) -> u32 {
        let scene = SCENE.get(h);
        method(h, SCENE_COPY_POST_BUFFER, scene, &[])
    }

    /// `graphicRenderCaptureAndIggy_00642be0()` ->
    /// `PhyreScene_renderCaptureTexture`'s result, after the Iggy menus'
    /// render.
    pub fn render_capture_and_iggy(h: &mut impl Host) -> u32 {
        let scene = SCENE.get(h);
        let r = method(h, SCENE_RENDER_CAPTURE, scene, &[]);
        let m = IGGY_MENU_MANAGER.get(h);
        method(h, IGGY_RENDER, m, &[]);
        r
    }

    /// `PCluster_moveInstanceLists_004409e0(this; other)` (`ret 4`): the
    /// other cluster's assets moved in (`0x0049c730(this + 0x28; other +
    /// 0x28)`); each of its instance lists (ring `+0x1c`) taken out of its
    /// world (`+0x40`, `Phyre_PWorld_removeInstanceList`), unlinked, linked
    /// at this ring's end, added to this world (`addInstanceList`),
    /// re-owned (`+0x28`) and rebound (`0x0045da20`); its free chain
    /// `+0x24` pushed onto this one's, the other's cleared. `eax` (read by
    /// callers through the pointer): `other` after a free chain, else the
    /// other cluster's world.
    pub fn cluster_move_instance_lists(h: &mut impl Host, this: u32, other: u32) -> u32 {
        method(h, MOVE_ASSETS, this + 0x28, &[other + 0x28]);
        let mine = word(h, this + 0x40);
        let theirs = word(h, other + 0x40);
        let ring = other + 0x1c;
        loop {
            let list = word(h, ring);
            if list == ring || list == 0 {
                break;
            }
            if theirs != 0 {
                method(h, WORLD_REMOVE_LIST, theirs, &[list]);
            }
            let next = word(h, list);
            let prev = word(h, list + 4);
            h.write(next + 4, prev);
            h.write(prev, next);
            h.write(list, list);
            h.write(list + 4, list);
            let head = this + 0x1c;
            let tail = word(h, head + 4);
            h.write(list, head);
            h.write(list + 4, tail);
            h.write(tail, list);
            h.write(head + 4, list);
            if mine != 0 {
                method(h, WORLD_ADD_LIST, mine, &[list]);
            }
            h.write(list + 0x28, this);
            cdecl(h, LIST_REBIND, &[list]);
        }
        let mut free = word(h, other + 0x24);
        if free == 0 {
            h.write(other + 0x24, 0_u32);
            return theirs;
        }
        while free != 0 {
            let next = word(h, free);
            let top = word(h, this + 0x24);
            h.write(free, top);
            h.write(this + 0x24, free);
            free = next;
        }
        h.write(other + 0x24, 0_u32);
        other
    }

    /// `PhyreCursor_seekValid_00441bf0(cursor)`: while the cursor is not
    /// valid (`PhyreCursor_IsValid_00441e40`), its node `+0x18` steps to
    /// the next one (`[node]`, 0 at the end `+0x14`); a node left is set
    /// (`PhyreSceneCursor_SetNode`, whose `eax` it returns); none clears
    /// the cursor's five words.
    pub fn cursor_seek_valid(h: &mut impl Host, cursor: u32) -> u32 {
        if method(h, CURSOR_IS_VALID, cursor, &[]).to_le_bytes()[0] == 0 {
            loop {
                let node = word(h, cursor + 0x18);
                if node != 0 {
                    let next = word(h, node);
                    let next = if next == word(h, cursor + 0x14) {
                        0
                    } else {
                        next
                    };
                    h.write(cursor + 0x18, next);
                }
                if method(h, CURSOR_IS_VALID, cursor, &[]) & 0xff != 0 {
                    break;
                }
            }
        }
        let node = word(h, cursor + 0x18);
        if node != 0 {
            return method(h, CURSOR_SET_NODE, cursor, &[node]);
        }
        for k in 0..5 {
            h.write(cursor + 4 * k, 0_u32);
        }
        0
    }

    /// `StdString_tidy_006335a0(s)` (stdcall): a long string's buffer
    /// (capacity `+0x14` >= 16) deleted; capacity 15, size 0, an empty
    /// inline buffer.
    pub fn std_string_tidy(h: &mut impl Host, s: u32) {
        if word(h, s + 0x14) >= 0x10 {
            let p = word(h, s);
            cdecl(h, OPERATOR_DELETE, &[p]);
        }
        h.write(s + 0x14, 0xf_u32);
        h.write(s + 0x10, 0_u32);
        h.write(s, 0_u8);
    }

    /// `LoadQueue_push_00633670(this; a, b, c, d, lane)` (`ret 0x14`):
    /// lane `this + lane * 0x1e8`: a free request is taken from the ring
    /// of 8 at `+0x1d0` (index `+0x1f0` + 7 mod 8, signed; count `+0x1f8`
    /// down) under its semaphores, filled (`+0xc` a, `+0x14` b, `+0x20` c,
    /// `+0x24` d, `+0x18`/`+0x1c` pointing at them, state 3) and queued
    /// in the ring at `+0x200` (index `+0x224` likewise, count `+0x228`
    /// up); the semaphores signalled. Stores the lane into its own last
    /// argument slot (dead after the `ret`).
    pub fn load_queue_push(
        h: &mut impl Host,
        this: u32,
        a: u32,
        b: u32,
        c: u32,
        d: u32,
        lane: u32,
    ) {
        let l = lane.wrapping_mul(0x1e8).wrapping_add(this);
        wait_at(h, l + 0x234);
        wait_at(h, l + 0x1fc);
        let i = ring_next(word(h, l + 0x1f0));
        let count = word(h, l + 0x1f8).wrapping_sub(1);
        h.write(l + 0x1f8, count);
        h.write(l + 0x1f0, i);
        let req = word(h, l.wrapping_add(i.wrapping_mul(4)) + 0x1d0);
        release_at(h, l + 0x1fc);
        h.write(req + 0x20, c);
        h.write(req + 0x24, d);
        h.write(req + 0xc, a);
        h.write(req + 0x14, b);
        h.write(req + 0x18, req + 0x20);
        h.write(req + 0x1c, req + 0x24);
        h.write(req + 0x10, 3_u32);
        wait_at(h, l + 0x22c);
        let j = ring_next(word(h, l + 0x224));
        h.write(l + 0x224, j);
        h.write(l.wrapping_add(j.wrapping_mul(4)) + 0x200, req);
        let n = word(h, l + 0x228).wrapping_add(1);
        h.write(l + 0x228, n);
        release_at(h, l + 0x22c);
        release_at(h, l + 0x230);
    }

    /// `WaitForSingleObject([at], INFINITE)`.
    fn wait_at(h: &mut impl Host, at: u32) {
        let s = word(h, at);
        import(h, WAIT_FOR_SINGLE_OBJECT, &[s, 0xffff_ffff]);
    }

    /// `ReleaseSemaphore([at], 1, NULL)`.
    fn release_at(h: &mut impl Host, at: u32) {
        let s = word(h, at);
        import(h, RELEASE_SEMAPHORE, &[s, 1, 0]);
    }

    /// `SaveDataTtyWrite2(10, text)`, the log line at `text`.
    fn tty(h: &mut impl Host, text: u32) {
        let s = h.rebase(text);
        cdecl(h, SAVE_DATA_TTY_WRITE2, &[10, s]);
    }

    /// `(i + 7) % 8` as MSVC's signed remainder.
    fn ring_next(i: u32) -> u32 {
        (i.wrapping_add(7).cast_signed() % 8).cast_unsigned()
    }

    /// `PArray_copyFrom_006337e0(this; src)` (`ret 4`) -> `this`: the
    /// source's elements (count low 31 bits, data `+4`) copied
    /// (`0x00635630(count, data)`) into `+4`; the count `+0` that count,
    /// or 0 when the copy failed.
    pub fn array_copy_from(h: &mut impl Host, this: u32, src: u32) -> u32 {
        h.write(this, 0_u32);
        let n = word(h, src) & 0x7fff_ffff;
        let data = word(h, src + 4);
        let p = cdecl(h, COPY_ARRAY, &[n, data]);
        h.write(this + 4, p);
        h.write(this, if p == 0 { 0 } else { n });
        this
    }

    /// `PRedBlackTreePair_dtor_*(this)`: `Phyre_PRedBlackTree_clear` twice
    /// (the two members' destructors at one address); its `eax`.
    pub fn tree_pair_dtor(h: &mut impl Host, this: u32) -> u32 {
        method(h, TREE_CLEAR, this, &[]);
        method(h, TREE_CLEAR, this, &[])
    }

    /// `PRedBlackTreeFree_dtor_00633ef0(this)`: orphaned, the free list
    /// `+0x18` reset and destroyed, cleared twice; the last `eax`.
    pub fn tree_free_dtor(h: &mut impl Host, this: u32) -> u32 {
        method(h, TREE_ORPHAN_ALL, this, &[]);
        method(h, FREE_LIST_RESET, this + 0x18, &[]);
        method(h, FREE_LIST_DTOR, this + 0x18, &[]);
        method(h, TREE_CLEAR, this, &[]);
        method(h, TREE_CLEAR, this, &[])
    }

    /// `PhyreScene_ChangeToHSVPass(material, on)`: on (byte), the default
    /// pass remapped to the HSV pass
    /// (`PMaterial_setSceneRenderPassRemapping(material; default, hsv)`);
    /// off, a remapping to the HSV pass (`+0xc` when `+8` is the default
    /// pass) is dropped (0, 0). `eax`: the call's, or what the compare
    /// left (the remapped pass, or the default pass).
    pub fn change_to_hsv_pass(h: &mut impl Host, material: u32, on: u32) -> u32 {
        let default = h.rebase(PASS_DEFAULT);
        let hsv = h.rebase(PASS_HSV);
        if on & 0xff != 0 {
            return method(h, SET_PASS_REMAPPING, material, &[default, hsv]);
        }
        let to = if word(h, material + 8) == default {
            word(h, material + 0xc)
        } else {
            default
        };
        if to == hsv {
            return method(h, SET_PASS_REMAPPING, material, &[0, 0]);
        }
        to
    }

    /// `graphicEnableHSV(model, on)`: the model's mesh array copied
    /// (`0x00635630`), every material of every mesh (`+0x10`'s array,
    /// `PArray_Data_004d45a0`) switched ([`change_to_hsv_pass`]; the flag
    /// word is the mesh index with `on != 0` in its low byte, as the
    /// original's `setne bl` leaves it), the copy freed when owned. The
    /// mesh index is kept in its own first argument slot (`slot`).
    pub fn enable_hsv(h: &mut impl Host, slot: u32, model: u32, on: u32) {
        if model == 0 {
            return;
        }
        let n = word(h, model) & 0x7fff_ffff;
        let src = word(h, model + 4);
        let meshes = cdecl(h, COPY_ARRAY, &[n, src]);
        let count = if meshes == 0 { 0 } else { n };
        h.write(slot, 0_u32);
        let mut k = 0_u32;
        while k < count & 0x7fff_ffff {
            let materials = word(h, word(h, meshes + 4 * k) + 0x10);
            let mut i = 0;
            while i < word(h, materials) & 0x7fff_ffff {
                let flag = (k & !0xff) | u32::from(on != 0);
                let data = method(h, ARRAY_DATA, materials, &[]);
                let m = word(h, data + 4 * i);
                cdecl(h, CHANGE_TO_HSV_PASS, &[m, flag]);
                i += 1;
            }
            k += 1;
            h.write(slot, k);
        }
        if count >> 31 == 0 {
            cdecl(h, PHYRE_FREE, &[meshes]);
        }
    }

    /// `graphicResetHSV_00637c10(chr)`: for a character `[chr]`,
    /// `graphicSetHSVParam(model, 0, 0, 0, g_ChrClothFlip)` (the f32
    /// through the x87 stack: a signalling NaN comes out quiet) and
    /// `graphicEnableHSV(model, 0)`.
    pub fn reset_hsv(h: &mut impl Host, chr: u32) {
        if chr == 0 {
            return;
        }
        let flip: u32 = h.read(h.rebase(CHR_CLOTH_FLIP));
        let flip = if flip & 0x7f80_0000 == 0x7f80_0000 && flip & 0x007f_ffff != 0 {
            flip | 0x0040_0000
        } else {
            flip
        };
        let model = word(h, chr);
        cdecl(h, SET_HSV_PARAM, &[model, 0, 0, 0, flip]);
        let model = word(h, chr);
        cdecl(h, ENABLE_HSV, &[model, 0]);
    }

    /// `LuaBind_pushObject_00441e70(L, p)` (stdcall): an object `[p]` is
    /// pushed with its class (`0x00436e80(L, object, 0x00c90b90)`); none
    /// pushes nil after `0x0094ac00(L, 1)`. The last call's `eax`.
    pub fn lua_push_object(h: &mut impl Host, l: u32, p: u32) -> u32 {
        let obj = word(h, p);
        if obj != 0 {
            let class = h.rebase(LUA_CLASS);
            return cdecl(h, LUA_PUSH_CLASS_OBJECT, &[l, obj, class]);
        }
        cdecl(h, LUA_0094AC00, &[l, 1]);
        cdecl(h, LUA_PUSHNIL, &[l])
    }
}
