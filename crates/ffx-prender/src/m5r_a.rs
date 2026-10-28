//! M5 stage 1, list `render`, batch 1: what the frame and the shutdown
//! reach of the scene that no coverage run ran. The `graphic*` forwarders
//! of the per-frame update, the render-delegation removal, the scene's
//! special-list clear, the post-process target resize, ATL's
//! `CStringW` resource loading (its error checks throw C++ exceptions
//! through `ThrowStdException_0064fa50`), the `PhyreScene` array reserves
//! and the release half of `graphicFinalize` (clusters, text, morphs,
//! texture animations, characters).
//!
//! The SEH frames and stack cookies of the destructors are left out:
//! nothing on their paths throws. Callees stay calls to the originals
//! (through their hooks where they are ported), with the calling
//! convention their `ret` says.

use ffx_bridge::Global;

/// `g_PhyreScene`.
pub const SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// `g_FreeCamera`, made on demand.
pub const FREE_CAMERA: Global<u32> = Global::new(0x00cc_c830);
/// `g_RenderDelegationHelper`.
pub const DELEGATIONS: Global<u32> = Global::new(0x00cc_ca98);
/// The delegation helper's mutex handle.
pub const DELEGATION_MUTEX: Global<u32> = Global::new(0x00cc_caa0);
/// The character instance list's mutex handle (`ClassCharacter`).
pub const CHARACTER_MUTEX: Global<u32> = Global::new(0x00cc_ca78);
/// ATL's `_AtlBaseModule` (`CAtlBaseModule`), whose resource instances
/// `GetHInstanceAt` walks.
pub const ATL_BASE_MODULE: u32 = 0x0193_fcc4;
/// The `ThrowInfo` of the `HRESULT` exception `ThrowStdException_0064fa50`
/// throws (ATL's `CAtlException`).
pub const ATL_THROW_INFO: u32 = 0x00bf_749c;

// ---- imports (slots) ----

/// `KERNEL32!WaitForSingleObject` (stdcall, 2 words).
pub const WAIT_FOR_SINGLE_OBJECT: u32 = 0x00b0_c0f8;
/// `KERNEL32!ReleaseMutex` (stdcall, 1 word).
pub const RELEASE_MUTEX: u32 = 0x00b0_c1c0;
/// `KERNEL32!InterlockedCompareExchange` (stdcall, 3 words).
pub const INTERLOCKED_COMPARE_EXCHANGE: u32 = 0x00b0_c12c;
/// `MSVCR110!__iob_func` (cdecl).
pub const IOB_FUNC: u32 = 0x00b0_c350;
/// `MSVCR110!fprintf` (cdecl, varargs).
pub const FPRINTF: u32 = 0x00b0_c38c;
/// `KERNEL32!FindResourceExW` (stdcall, 4 words).
pub const FIND_RESOURCE_EX_W: u32 = 0x00b0_c1cc;
/// `KERNEL32!FindResourceW` (stdcall, 3 words).
pub const FIND_RESOURCE_W: u32 = 0x00b0_c17c;
/// `MSVCR110!memcpy_s` (cdecl, 4 words).
pub const MEMCPY_S: u32 = 0x00b0_c434;

// ---- CRT thunks (`jmp [import]`, cdecl) ----

/// `_CxxThrowException(object, info)` (never returns).
pub const CXX_THROW: u32 = 0x0094_924a;
/// `memcpy(dst, src, n)`.
pub const MEMCPY: u32 = 0x0094_9256;
/// `memset(dst, value, n)`.
pub const MEMSET: u32 = 0x0094_964c;
/// `wmemcpy_s(dst, size, src, n)` (the `errno_t` that
/// [`ATL_CRT_ERROR_CHECK`] checks).
pub const WMEMCPY_S: u32 = 0x0094_9d56;

// ---- game functions called ----

/// `PhyreScene_animate(scene)`.
pub const SCENE_ANIMATE: u32 = 0x0065_7140;
/// `Phyre_PPostProcessing_renderTargetResize_00a0e4b0(target, w, h,
/// force)` (cdecl).
pub const RENDER_TARGET_RESIZE: u32 = 0x00a0_e4b0;
/// `PostProcessManager_resizeTarget_006980a0(this; w, h)`.
pub const POST_RESIZE_TARGET: u32 = 0x0069_80a0;
/// `PhyreScene_resizePostTarget_0066c670(scene)`.
pub const SCENE_RESIZE_POST_TARGET: u32 = 0x0066_c670;
/// `RenderDelegationHelper_compact(this)`.
pub const DELEGATIONS_COMPACT: u32 = 0x006a_d590;
/// `RenderDelegationHelper_remove_006ae240(this; delegation)`.
pub const DELEGATIONS_REMOVE: u32 = 0x006a_e240;
/// `RenderDelegationHelper_removeFlag1000_006ae530()`.
pub const DELEGATIONS_REMOVE_FLAG_1000: u32 = 0x006a_e530;
/// `operator_new(size)` / `operator_delete(p)` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `FreeCamera_FreeCamera(this)` -> `this`.
pub const FREE_CAMERA_CTOR: u32 = 0x0067_fe70;
/// `user_malloc(n)` / `user_free_thunk(p)` (cdecl).
pub const USER_MALLOC: u32 = 0x0068_7020;
pub const USER_FREE: u32 = 0x0068_7010;
/// `Phyre_PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `Array_Reserve_00668100(array; n)`, the scene's 4-byte lists'.
pub const RESERVE_4: u32 = 0x0066_8100;
/// `PhyreScene_clearSpecialRenderList_00654b70(scene)`.
pub const SCENE_CLEAR_SPECIAL: u32 = 0x0065_4b70;
/// `CAtlBaseModule::GetHInstanceAt(module; i)`.
pub const GET_HINSTANCE_AT: u32 = 0x009d_9db5;
/// `AtlGetStringResourceImage(instance, resource, id)` (cdecl).
pub const STRING_RESOURCE_IMAGE: u32 = 0x0061_b3c0;
/// `ThrowStdException_0064fa50(hresult)` (never returns).
pub const THROW: u32 = 0x0064_fa50;
/// `AtlCrtErrorCheck_0064f8d0(errno)`.
pub const ATL_CRT_ERROR_CHECK: u32 = 0x0064_f8d0;
/// `AtlThrowOutOfMemory_00654950()`.
pub const THROW_OUT_OF_MEMORY: u32 = 0x0065_4950;
/// `CSimpleString_Fork_00651460(this; n)`.
pub const STRING_FORK: u32 = 0x0065_1460;
/// `CSimpleString_Reallocate_006528d0(this; n)`.
pub const STRING_REALLOCATE: u32 = 0x0065_28d0;
/// `CSimpleString_PrepareWrite2_00652600(this; n)`.
pub const STRING_PREPARE_WRITE2: u32 = 0x0065_2600;
/// `TextureAnimationManager_unregisterChararcterInstance(this; inst)`.
pub const TEXANIM_UNREGISTER_CHARACTER: u32 = 0x006b_4330;
/// `TextureAnimationManager_unregisterByName_006b1a70(this; name)`.
pub const TEXANIM_UNREGISTER_BY_NAME: u32 = 0x006b_1a70;
/// `TextureAnimationMaterial_dtor_006af690(this)`.
pub const TEXANIM_MATERIAL_DTOR: u32 = 0x006a_f690;
/// `ClassLoadedCharacterTable_GetInstance()`.
pub const LOADED_CHARACTERS: u32 = 0x0069_f6c0;
/// The loaded-character table's tree (`0x006a2220`, thiscall) -> the
/// address of the tree's head pointer.
pub const LOADED_CHARACTER_TREE: u32 = 0x006a_2220;
/// `graphicCharacterInstanceDestroy(inst, -1)` (cdecl).
pub const CHARACTER_INSTANCE_DESTROY: u32 = 0x0063_a030;
/// `ClassCharacter_clearInstanceList(this)`.
pub const CHARACTER_CLEAR_INSTANCES: u32 = 0x006a_1200;
/// The loaded-character table's clear (`0x006a11d0`).
pub const LOADED_CHARACTERS_CLEAR: u32 = 0x006a_11d0;
/// The tree erase the clear calls (`0x006a04e0`, thiscall; the root).
pub const LOADED_CHARACTERS_ERASE: u32 = 0x006a_04e0;
/// The scene's character-cluster release (`0x00670960`).
pub const SCENE_RELEASE_CHARACTERS: u32 = 0x0067_0960;
/// The post-process manager's members' releases (thiscall).
pub const POST_RELEASE_A: u32 = 0x00a2_4c90;
pub const POST_RELEASE_B: u32 = 0x00a2_4d60;
pub const RENDER_TARGET_RELEASE: u32 = 0x0044_0ea0;
pub const SAMPLER_RELEASE: u32 = 0x0059_c770;
/// `ClusterManager_GetInstance()` and its `releasePCluster(this;
/// cluster)`, `0x0069b890(this; cluster)` (al: the cluster is held).
pub const CLUSTER_MANAGER: u32 = 0x0069_a1a0;
pub const CLUSTER_RELEASE: u32 = 0x0069_bdb0;
pub const CLUSTER_HELD: u32 = 0x0069_b890;
/// `Phyre_PRendering_PSamplerStateD3D11_dtor(this)`.
pub const SAMPLER_DTOR: u32 = 0x0058_5230;
/// `TextUtil_terminate(this)`.
pub const TEXT_UTIL_TERMINATE: u32 = 0x006f_6950;
/// `PArray_releaseTaggedWords_00434360(data, count)` (cdecl).
pub const RELEASE_TAGGED_WORDS: u32 = 0x0043_4360;
/// `Phyre_PStringBuilder_dtor_PStringBuilder(this)`.
pub const STRING_BUILDER_DTOR: u32 = 0x0044_4ab0;
/// `TextUtilEntry_dtor_00692950(this)`.
pub const TEXT_ENTRY_DTOR: u32 = 0x0069_2950;

/// The `HRESULT`s ATL throws.
pub const E_OUTOFMEMORY: u32 = 0x8007_000e;
pub const E_INVALIDARG: u32 = 0x8007_0057;
pub const E_FAIL: u32 = 0x8000_4005;

/// The scene's cluster words that `graphicFinalize`'s releases
/// (`0x006708e0`..`0x00671240`) drop, in their order in the list.
pub const SCENE_CLUSTERS: [(u32, u32); 10] = [
    (0x0067_08e0, 0xfa70),
    (0x0067_0920, 0xfa74),
    (0x0067_0ea0, 0xfa7c),
    (0x0067_10a0, 0xfa80),
    (0x0067_10e0, 0xfa84),
    (0x0067_1140, 0xfa88),
    (0x0067_1180, 0xfa8c),
    (0x0067_11c0, 0xfa78),
    (0x0067_1200, 0xfa90),
    (0x0067_1240, 0xfa94),
];

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    /// The import in `slot`, called with its convention.
    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    fn wait(h: &mut impl Host, mutex: Global<u32>) -> u32 {
        let m = mutex.get(h);
        import(h, WAIT_FOR_SINGLE_OBJECT, Conv::Stdcall, &[m, 0xffff_ffff])
    }

    fn release(h: &mut impl Host, mutex: Global<u32>) -> u32 {
        let m = mutex.get(h);
        import(h, RELEASE_MUTEX, Conv::Stdcall, &[m])
    }

    fn byte(h: &impl Mem, at: u32) -> u8 {
        h.read(at)
    }

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    // ---- the per-frame forwarders ----

    /// `graphicSceneAnimate_00639300()`: `PhyreScene_animate(g_PhyreScene)`
    /// (a tail jump), its `eax`.
    pub fn scene_animate(h: &mut impl Host) -> u32 {
        let scene = SCENE.get(h);
        h.call_sym(SCENE_ANIMATE, Conv::Thiscall(scene), &[])
    }

    /// `PostProcessManager_resizeTarget_006980a0(this; w, height)` (`ret
    /// 8`) -> 0: with one of the flags `+0x123b0..+0x123b3` set and a
    /// target `+0xac` whose texture (`[target]` unless the kind byte
    /// `+4` is set, then 0) is not `w` x `height` (`+0x1c`, `+0x20`),
    /// `renderTargetResize(target, w, height, 1)`. A target of another
    /// kind reads the size at 0 (a fault there, as in the original).
    pub fn post_resize_target(h: &mut impl Host, this: u32, w: u32, height: u32) -> u32 {
        let flagged = (0..4).any(|i| byte(h, this + 0x1_23b0 + i) != 0);
        if !flagged {
            return 0;
        }
        let target = word(h, this + 0xac);
        if target == 0 {
            return 0;
        }
        let texture = |h: &dyn Fn(u32) -> u32| {
            if h(target + 4).to_le_bytes()[0] == 0 {
                h(target)
            } else {
                0
            }
        };
        let rd = |at: u32| word(h, at);
        let same = w == rd(texture(&rd) + 0x1c) && height == rd(texture(&rd) + 0x20);
        if !same {
            h.call_sym(RENDER_TARGET_RESIZE, Conv::Cdecl, &[target, w, height, 1]);
        }
        0
    }

    /// `PhyreScene_resizePostTarget_0066c670(scene)`: the post-process
    /// manager `+0x10cc` resized to the scene's size (`+0x10dd0`,
    /// `+0x10dd4`); its `eax`.
    pub fn scene_resize_post_target(h: &mut impl Host, scene: u32) -> u32 {
        let w = word(h, scene + 0x1_0dd0);
        let height = word(h, scene + 0x1_0dd4);
        let post = word(h, scene + 0x10cc);
        h.call_sym(POST_RESIZE_TARGET, Conv::Thiscall(post), &[w, height])
    }

    /// `graphicResizePostTarget_00642ce0()`: on `g_PhyreScene` (a tail
    /// jump).
    pub fn resize_post_target(h: &mut impl Host) -> u32 {
        let scene = SCENE.get(h);
        h.call_sym(SCENE_RESIZE_POST_TARGET, Conv::Thiscall(scene), &[])
    }

    // ---- render delegations ----

    /// `RenderDelegationHelper_remove_006ae240(this; delegation)` (`ret
    /// 4`) -> `ReleaseMutex`'s result. Under the mutex: the delegation's
    /// index in the array (count `+0` low 31 bits, data `+4`); found, its
    /// task among the 64 running pairs `+8` (task, busy word `+0xc`):
    /// running, the busy word is taken back (`InterlockedCompareExchange(
    /// busy, 0, 1)`, a warning on stderr when it wasn't 1) and the
    /// delegation is not freed; not running, its data (`+4`) and the
    /// delegation are deleted (`+8` cleared first). Either way the slot
    /// is cleared and the array compacted.
    pub fn delegations_remove(h: &mut impl Host, this: u32, delegation: u32) -> u32 {
        wait(h, DELEGATION_MUTEX);
        let n = word(h, this) & 0x7fff_ffff;
        let data = word(h, this + 4);
        let Some(i) = (0..n).find(|&i| word(h, data + 4 * i) == delegation) else {
            return release(h, DELEGATION_MUTEX);
        };
        let task = word(h, word(h, this + 4) + 4 * i);
        let running = (0..0x40).find(|&j| word(h, this + 8 + 8 * j) == task);
        if let Some(j) = running {
            let busy = this + 0xc + 8 * j;
            let was = import(
                h,
                INTERLOCKED_COMPARE_EXCHANGE,
                Conv::Stdcall,
                &[busy, 0, 1],
            );
            if was != 1 {
                let iob = import(h, IOB_FUNC, Conv::Cdecl, &[]);
                let fmt = h.rebase(0x00b4_83d8);
                import(h, FPRINTF, Conv::Cdecl, &[iob + 0x40, fmt, j]);
            }
        } else {
            let d = word(h, word(h, this + 4) + 4 * i);
            if d != 0 {
                let inner = word(h, d + 4);
                if inner != 0 {
                    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[inner]);
                }
                h.write(d + 8, 0_u32);
                h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[d]);
            }
        }
        let at = word(h, this + 4) + 4 * i;
        h.write(at, 0_u32);
        h.call_sym(DELEGATIONS_COMPACT, Conv::Thiscall(this), &[]);
        release(h, DELEGATION_MUTEX)
    }

    /// `RenderDelegationHelper_removeFlag1000_006ae530()`: every
    /// delegation of `g_RenderDelegationHelper` up to the first empty slot
    /// whose flags (`+0xc`) have `0x1000` is removed
    /// ([`delegations_remove`], on the helper read again after each), the
    /// index staying put. `eax` is what the original leaves: 0 at an empty
    /// slot, else the last count (callers read none of it; an empty array
    /// leaves the caller's `eax`, which the port can't, so it returns 0
    /// there).
    pub fn delegations_remove_flag_1000(h: &mut impl Host) -> u32 {
        let helper = DELEGATIONS.get(h);
        let mut this = helper;
        let mut i = 0_u32;
        let mut n = word(h, helper) & 0x7fff_ffff;
        while i < n {
            let d = word(h, word(h, helper + 4) + 4 * i);
            if d == 0 {
                return 0;
            }
            if word(h, d + 0xc) & 0x1000 != 0 {
                h.call_sym(DELEGATIONS_REMOVE, Conv::Thiscall(this), &[d]);
                this = DELEGATIONS.get(h);
                i = i.wrapping_sub(1);
            }
            i = i.wrapping_add(1);
            n = word(h, helper) & 0x7fff_ffff;
        }
        n
    }

    /// `graphicRemoveDelegations_0063dab0()`: a tail jump to
    /// [`delegations_remove_flag_1000`].
    pub fn remove_delegations(h: &mut impl Host) -> u32 {
        h.call_sym(DELEGATIONS_REMOVE_FLAG_1000, Conv::Cdecl, &[])
    }

    /// `graphicGetFreeCameraType()` -> 2 when the free camera's bytes
    /// `+0x338` or `+0x33c` are set, else its type word `+0`. The camera
    /// is made on demand (`operator_new(0x340)`, `FreeCamera_FreeCamera`;
    /// a failed allocation stores 0 and reads at 0 then).
    pub fn free_camera_type(h: &mut impl Host) -> u32 {
        let mut cam = FREE_CAMERA.get(h);
        if cam == 0 {
            let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x340]);
            cam = if p == 0 {
                0
            } else {
                h.call_sym(FREE_CAMERA_CTOR, Conv::Thiscall(p), &[])
            };
            FREE_CAMERA.set(h, cam);
        }
        if byte(h, cam + 0x338) != 0 || byte(h, cam + 0x33c) != 0 {
            2
        } else {
            word(h, cam)
        }
    }

    // ---- the scene's arrays ----

    /// `Array_Reserve_*(array; n)` (`ret 4`) for elements of `elem`
    /// bytes (array: count `+0`, capacity `+4`, data `+8`). Without data,
    /// `user_malloc(n * elem)` becomes the data with capacity `n`, and the
    /// allocation's `eax` is returned. With data, a new block for a
    /// positive (signed) `n`, the `count` elements `memcpy`'d into it when
    /// it was allocated; the old block `user_free_thunk`'d; capacity `n`,
    /// data the new block (0 for `n <= 0` or a failed allocation); the
    /// free's `eax`.
    pub fn reserve(h: &mut impl Host, this: u32, n: u32, elem: u32) -> u32 {
        if word(h, this + 8) == 0 {
            let p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[n.wrapping_mul(elem)]);
            h.write(this + 4, n);
            h.write(this + 8, p);
            return p;
        }
        let mut block = 0;
        if n.cast_signed() > 0 {
            block = h.call_sym(USER_MALLOC, Conv::Cdecl, &[n.wrapping_mul(elem)]);
            if block != 0 {
                let count = word(h, this);
                let old = word(h, this + 8);
                h.call_sym(MEMCPY, Conv::Cdecl, &[block, old, count.wrapping_mul(elem)]);
            }
        }
        let old = word(h, this + 8);
        let r = h.call_sym(USER_FREE, Conv::Cdecl, &[old]);
        h.write(this + 4, n);
        h.write(this + 8, block);
        r
    }

    /// `PhyreScene_clearSpecialRenderList_00654b70(scene)` -> 0: the two
    /// 0x3000-byte lists `+0x38cc` and `+0x68cc` cleared with their counts
    /// (`+0xf8cc`, `+0xf8d0`); the four 4-byte arrays `+0x3878`, `+0x3854`,
    /// `+0x3860`, `+0x386c` emptied (reserved to 0 first when both count
    /// and capacity are negative); the words of `+0x3884`'s array cleared
    /// and `+0x388c`; the 16 pairs at `+0xf8d4`/`+0xf914`, `+0xf954` and
    /// `+0x64..+0x6c` zeroed.
    pub fn clear_special_render_list(h: &mut impl Host, scene: u32) -> u32 {
        h.call_sym(MEMSET, Conv::Cdecl, &[scene + 0x38cc, 0, 0x3000]);
        h.write(scene + 0xf8cc, 0_u32);
        h.call_sym(MEMSET, Conv::Cdecl, &[scene + 0x68cc, 0, 0x3000]);
        h.write(scene + 0xf8d0, 0_u32);
        for off in [0x3878, 0x3854, 0x3860, 0x386c] {
            let a = scene + off;
            if word(h, a).cast_signed() < 0 && word(h, a + 4).cast_signed() < 0 {
                h.call_sym(RESERVE_4, Conv::Thiscall(a), &[0]);
            }
            h.write(a, 0_u32);
        }
        let n = (word(h, scene + 0x3884) & 0x7fff_ffff) << 2;
        let data = word(h, scene + 0x3888);
        h.call_sym(MEMSET, Conv::Cdecl, &[data, 0, n]);
        h.write(scene + 0x388c, 0_u32);
        for k in 0..16 {
            h.write(scene + 0xf8d4 + 4 * k, 0_u32);
            h.write(scene + 0xf914 + 4 * k, 0_u32);
        }
        h.write(scene + 0xf954, 0_u32);
        h.write(scene + 0x64, 0_u32);
        h.write(scene + 0x68, 0_u32);
        h.write(scene + 0x6c, 0_u32);
        0
    }

    /// `graphicClearSpecialRenderList()`: on `g_PhyreScene` (a tail jump).
    pub fn graphic_clear_special_render_list(h: &mut impl Host) -> u32 {
        let scene = SCENE.get(h);
        h.call_sym(SCENE_CLEAR_SPECIAL, Conv::Thiscall(scene), &[])
    }

    // ---- ATL strings ----

    /// `AtlFindStringResourceInstance_0064f970(id, language)` -> the first
    /// of `_AtlBaseModule`'s resource instances (`GetHInstanceAt(0, 1,
    /// ...)` until 0) holding string `id`: `FindResourceExW(instance,
    /// RT_STRING, id / 16 + 1 (16 bits), language)` and
    /// `AtlGetStringResourceImage(instance, resource, id)`; 0 if none.
    pub fn find_string_resource_instance(h: &mut impl Host, id: u32, language: u32) -> u32 {
        let module = h.rebase(ATL_BASE_MODULE);
        let mut inst = h.call_sym(GET_HINSTANCE_AT, Conv::Thiscall(module), &[0]);
        let mut next = 1;
        let block = ((id >> 4) + 1) & 0xffff;
        while inst != 0 {
            let r = import(
                h,
                FIND_RESOURCE_EX_W,
                Conv::Stdcall,
                &[inst, 6, block, language],
            );
            if r != 0 && h.call_sym(STRING_RESOURCE_IMAGE, Conv::Cdecl, &[inst, r, id]) != 0 {
                return inst;
            }
            inst = h.call_sym(GET_HINSTANCE_AT, Conv::Thiscall(module), &[next]);
            next += 1;
        }
        0
    }

    /// `ThrowStdException_0064fa50(hresult)`: `_CxxThrowException(&slot,
    /// ATL's throw info)` with the code stored back into its own argument
    /// slot (the exception object). Never returns in the game; the port
    /// returns the call's `eax` should it.
    pub fn throw(h: &mut impl Host, slot: u32, hresult: u32) -> u32 {
        h.write(slot, hresult);
        let info = h.rebase(ATL_THROW_INFO);
        h.call_sym(CXX_THROW, Conv::Cdecl, &[slot, info])
    }

    /// `AtlCrtErrorCheck_0064f8d0(errno)` -> `errno` for 0 and
    /// `STRUNCATE` (80); throws `E_OUTOFMEMORY` for `ENOMEM` (12),
    /// `E_INVALIDARG` for `EINVAL` (22) and `ERANGE` (34), `E_FAIL` for
    /// anything else (the original's byte-indexed jump table at
    /// `0x0064f91c`).
    pub fn crt_error_check(h: &mut impl Host, errno: u32) -> u32 {
        let code = match errno {
            0 | 80 => return errno,
            12 => E_OUTOFMEMORY,
            22 | 34 => E_INVALIDARG,
            _ => E_FAIL,
        };
        h.call_sym(THROW, Conv::Cdecl, &[code])
    }

    /// `AtlThrowOutOfMemory_00654950()`: throws `E_OUTOFMEMORY`.
    pub fn throw_out_of_memory(h: &mut impl Host) -> u32 {
        h.call_sym(THROW, Conv::Cdecl, &[E_OUTOFMEMORY])
    }

    /// `CSimpleString_Fork_00651460(this; n)` (`ret 4`) -> `this`: the
    /// string data (`[this]`, header at `-0x10`: manager, length,
    /// allocated, references) copied into a new buffer of the manager's
    /// clone (`manager->vtable[4]()`, then `vtable[0](n, 2)`; none:
    /// [`throw_out_of_memory`]) with `min(length, n) + 1` characters
    /// (`memcpy_s`), the length kept; the old header released (references
    /// down by one; at 0 or below `manager->vtable[1](header)`).
    pub fn string_fork(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let header = word(h, this).wrapping_sub(0x10);
        let manager = word(h, header);
        let length = word(h, header + 4);
        let clone = h.call_virtual(manager, 4, &[]);
        let new = h.call_virtual(clone, 0, &[n, 2]);
        if new == 0 {
            return h.call_sym(THROW_OUT_OF_MEMORY, Conv::Cdecl, &[]);
        }
        let keep = if length.cast_signed() >= n.cast_signed() {
            n
        } else {
            length
        };
        let bytes = keep.wrapping_mul(2).wrapping_add(2);
        let dst = new + 0x10;
        import(
            h,
            MEMCPY_S,
            Conv::Cdecl,
            &[dst, bytes, header + 0x10, bytes],
        );
        h.write(new + 4, length);
        let refs = word(h, header + 0xc).wrapping_sub(1);
        h.write(header + 0xc, refs);
        if refs.cast_signed() <= 0 {
            let manager = word(h, header);
            h.call_virtual(manager, 1, &[header]);
        }
        h.write(this, dst);
        this
    }

    /// `CSimpleString_Reallocate_006528d0(this; n)` (`ret 4`) -> the new
    /// data: with the allocated length (`header +8`) below a positive
    /// `n`, `manager->vtable[2](header, n, 2)`'s block (+0x10) becomes the
    /// data; otherwise, or when that fails, [`throw_out_of_memory`].
    pub fn string_reallocate(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let header = word(h, this).wrapping_sub(0x10);
        let manager = word(h, header);
        if word(h, header + 8).cast_signed() < n.cast_signed() && n.cast_signed() > 0 {
            let new = h.call_virtual(manager, 2, &[header, n, 2]);
            if new != 0 {
                let data = new + 0x10;
                h.write(this, data);
                return data;
            }
        }
        h.call_sym(THROW_OUT_OF_MEMORY, Conv::Cdecl, &[])
    }

    /// `CSimpleString_PrepareWrite2_00652600(this; n)` (`ret 4`): `n`
    /// raised to the length; a shared string (references > 1) is forked
    /// ([`string_fork`]); a short buffer (allocated below `n`) grows to
    /// the larger of `n` and 1.5 times the allocation (plus 1 MB past 1
    /// G characters) ([`string_reallocate`]). `eax`: the callee's, or the
    /// length when nothing was needed.
    pub fn string_prepare_write2(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let data = word(h, this);
        let length = word(h, data.wrapping_sub(0xc));
        let n = if length.cast_signed() > n.cast_signed() {
            length
        } else {
            n
        };
        if word(h, data.wrapping_sub(4)).cast_signed() > 1 {
            return h.call_sym(STRING_FORK, Conv::Thiscall(this), &[n]);
        }
        let alloc = word(h, data.wrapping_sub(8));
        if alloc.cast_signed() >= n.cast_signed() {
            return length;
        }
        let grown = if alloc.cast_signed() > 0x4000_0000 {
            alloc.wrapping_add(0x10_0000)
        } else {
            alloc.wrapping_add((alloc.cast_signed() / 2).cast_unsigned())
        };
        let size = if grown.cast_signed() < n.cast_signed() {
            n
        } else {
            grown
        };
        h.call_sym(STRING_REALLOCATE, Conv::Thiscall(this), &[size])
    }

    /// `CStringW_LoadString_00652310(this; instance, id)` (`ret 8`) -> 1
    /// when string `id` was loaded from `instance`, else 0:
    /// `FindResourceW(instance, id / 16 + 1, RT_STRING)`,
    /// `AtlGetStringResourceImage`; the image's length (`u16`) made room
    /// for ([`string_prepare_write2`] when shared or short), the
    /// characters `wmemcpy_s`'d (checked by [`crt_error_check`]), the
    /// length set (`E_INVALIDARG` thrown past the allocation) and the
    /// terminator written.
    pub fn load_string(h: &mut impl Host, this: u32, instance: u32, id: u32) -> u32 {
        let block = ((id >> 4) + 1) & 0xffff;
        let r = import(h, FIND_RESOURCE_W, Conv::Stdcall, &[instance, block, 6]);
        if r == 0 {
            return 0;
        }
        let image = h.call_sym(STRING_RESOURCE_IMAGE, Conv::Cdecl, &[instance, r, id]);
        if image == 0 {
            return 0;
        }
        let length = u32::from(h.read::<u16>(image));
        let data = word(h, this);
        let refs = word(h, data.wrapping_sub(4));
        let alloc = word(h, data.wrapping_sub(8));
        if (1_u32.wrapping_sub(refs) | alloc.wrapping_sub(length)).cast_signed() < 0 {
            h.call_sym(STRING_PREPARE_WRITE2, Conv::Thiscall(this), &[length]);
        }
        let count = u32::from(h.read::<u16>(image));
        let data = word(h, this);
        let e = h.call_sym(WMEMCPY_S, Conv::Cdecl, &[data, length, image + 2, count]);
        h.call_sym(ATL_CRT_ERROR_CHECK, Conv::Cdecl, &[e]);
        let data = word(h, this);
        if length.cast_signed() > word(h, data.wrapping_sub(8)).cast_signed() {
            return h.call_sym(THROW, Conv::Cdecl, &[E_INVALIDARG]);
        }
        h.write(data.wrapping_sub(0xc), length);
        let data = word(h, this);
        h.write(data + 2 * length, 0_u16);
        1
    }

    // ---- graphicFinalize's releases ----

    /// `TextureAnimationManager_unregisterAll_006b1b60(this)`: every
    /// character instance (`+0x6c` count, 12-byte entries `+0x70`, the
    /// instance at `+4`) and every material (`+0x64` count, 32-byte
    /// entries `+0x68`, the tagged name word `+0x18` less its tag bit)
    /// unregistered; then the material array destroyed (each
    /// `TextureAnimationMaterial_dtor`, `Phyre_PhyreFree`, unless the
    /// count's top bit marks it borrowed) and cleared. `eax` is dropped by
    /// the only caller.
    pub fn texanim_unregister_all(h: &mut impl Host, this: u32) {
        let n = word(h, this + 0x6c) & 0x7fff_ffff;
        for k in 0..n {
            let inst = word(h, word(h, this + 0x70) + 12 * k + 4);
            h.call_sym(TEXANIM_UNREGISTER_CHARACTER, Conv::Thiscall(this), &[inst]);
        }
        let m = word(h, this + 0x64) & 0x7fff_ffff;
        for k in 0..m {
            let at = word(h, this + 0x68) + 0x20 * k + 0x18;
            let tagged = word(h, at);
            let name = tagged.wrapping_sub(u32::from(byte(h, at) & 1));
            h.call_sym(TEXANIM_UNREGISTER_BY_NAME, Conv::Thiscall(this), &[name]);
        }
        let c = word(h, this + 0x64);
        if c == 0 {
            return;
        }
        let data = word(h, this + 0x68);
        if data != 0 && c >> 31 == 0 {
            for k in 0..(c & 0x7fff_ffff) {
                h.call_sym(TEXANIM_MATERIAL_DTOR, Conv::Thiscall(data + 0x20 * k), &[]);
            }
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
        h.write(this + 0x68, 0_u32);
        h.write(this + 0x64, 0_u32);
    }

    /// `ClassLoadedCharacterTable_clear_006a11d0(this)`: the tree (head
    /// `[this]`) erased from its root (`[head + 4]`), the head linked to
    /// itself (`+4`, `+0`, `+8`) and the size `+4` cleared; `eax` the head.
    pub fn loaded_characters_clear(h: &mut impl Host, this: u32) -> u32 {
        let root = word(h, word(h, this) + 4);
        h.call_sym(LOADED_CHARACTERS_ERASE, Conv::Thiscall(this), &[root]);
        let head = word(h, this);
        h.write(head + 4, head);
        let head = word(h, this);
        h.write(head, head);
        let head = word(h, this);
        h.write(head + 8, head);
        h.write(this + 4, 0_u32);
        head
    }

    /// `ClassCharacter_clearInstanceList(this)`: under the character
    /// mutex the list's end (`+0xc`) set back to its start (`+8`);
    /// `ReleaseMutex`'s result.
    pub fn character_clear_instances(h: &mut impl Host, this: u32) -> u32 {
        wait(h, CHARACTER_MUTEX);
        let start = word(h, this + 8);
        h.write(this + 0xc, start);
        release(h, CHARACTER_MUTEX)
    }

    /// The in-order successor of `node` in an `std::map` tree (left `+0`,
    /// parent `+4`, right `+8`, nil byte `+0xd`); a nil node stays.
    pub fn tree_next(h: &impl Mem, node: u32) -> u32 {
        if byte(h, node + 0xd) != 0 {
            return node;
        }
        let right = word(h, node + 8);
        if byte(h, right + 0xd) == 0 {
            let mut n = right;
            loop {
                let left = word(h, n);
                if byte(h, left + 0xd) != 0 {
                    return n;
                }
                n = left;
            }
        }
        let mut n = node;
        let mut p = word(h, n + 4);
        while byte(h, p + 0xd) == 0 && n == word(h, p + 8) {
            n = p;
            p = word(h, p + 4);
        }
        p
    }

    /// A character's instance count: its vector's bytes (`+0xc - +8`)
    /// shifted right signed by 2, compared unsigned.
    fn instance_count(h: &impl Mem, chr: u32) -> u32 {
        (word(h, chr + 0xc)
            .wrapping_sub(word(h, chr + 8))
            .cast_signed()
            >> 2)
            .cast_unsigned()
    }

    /// `graphicCharacterDestroyAll_006378b0()`: for every loaded character
    /// (the table's tree, value `+0x28`): each of its instances (vector
    /// `+8..+0xc`) destroyed (`graphicCharacterInstanceDestroy(inst, -1)`),
    /// its instance list cleared and the character deleted (virtual 0 with
    /// 1); then the table cleared and the scene's character clusters
    /// released (`0x00670960`), whose `eax` it returns.
    pub fn character_destroy_all(h: &mut impl Host) -> u32 {
        let table = h.call_sym(LOADED_CHARACTERS, Conv::Cdecl, &[]);
        let head = h.call_sym(LOADED_CHARACTER_TREE, Conv::Thiscall(table), &[]);
        let mut node = word(h, word(h, head));
        while node != word(h, head) {
            let chr = word(h, node + 0x28);
            if instance_count(h, chr) != 0 {
                let mut k = 0_u32;
                loop {
                    let inst = word(h, word(h, chr + 8) + 4 * k);
                    h.call_sym(
                        CHARACTER_INSTANCE_DESTROY,
                        Conv::Cdecl,
                        &[inst, 0xffff_ffff],
                    );
                    k += 1;
                    if k >= instance_count(h, chr) {
                        break;
                    }
                }
            }
            h.call_sym(CHARACTER_CLEAR_INSTANCES, Conv::Thiscall(chr), &[]);
            h.call_virtual(chr, 0, &[1]);
            node = tree_next(h, node);
        }
        let table = h.call_sym(LOADED_CHARACTERS, Conv::Cdecl, &[]);
        h.call_sym(LOADED_CHARACTERS_CLEAR, Conv::Thiscall(table), &[]);
        let scene = SCENE.get(h);
        h.call_sym(SCENE_RELEASE_CHARACTERS, Conv::Thiscall(scene), &[])
    }

    /// `PostProcessManager_release_00698f10(this)`: the members' releases
    /// (`+0x140`, `+0x1100`, the render target `+0x3c`, the sampler
    /// `+0x123dc`) and the 14 clusters `+4..+0x38` released.
    pub fn post_release(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(POST_RELEASE_A, Conv::Thiscall(this + 0x140), &[]);
        h.call_sym(POST_RELEASE_B, Conv::Thiscall(this + 0x1100), &[]);
        h.call_sym(RENDER_TARGET_RELEASE, Conv::Thiscall(this + 0x3c), &[]);
        h.call_sym(SAMPLER_RELEASE, Conv::Thiscall(this + 0x1_23dc), &[]);
        let mut r = 0;
        for k in 0..14 {
            let cluster = word(h, this + 4 + 4 * k);
            let m = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
            r = h.call_sym(CLUSTER_RELEASE, Conv::Thiscall(m), &[cluster]);
        }
        r
    }

    /// `TextUtilAsset_release_006f5c40(this)`: the inline array `+4`
    /// (count low 31 bits; data `[+8]` past one element, else inline at
    /// `+8`) freed when it is on the heap and owned (top bit clear), its
    /// count cleared; the sampler `+0xc` destroyed; then an owned array of
    /// more than one element freed, data and count cleared.
    pub fn text_asset_release(h: &mut impl Host, this: u32) {
        let c = word(h, this + 4);
        if c != 0 {
            let n = c & 0x7fff_ffff;
            let data = if n > 1 {
                word(h, this + 8)
            } else if n != 0 {
                this + 8
            } else {
                0
            };
            if data != this + 8 && c >> 31 == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
            h.write(this + 4, 0_u32);
        }
        h.call_sym(SAMPLER_DTOR, Conv::Thiscall(this + 0xc), &[]);
        let c = word(h, this + 4);
        if c >> 31 == 0 && c & 0x7fff_ffff > 1 {
            let data = word(h, this + 8);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
        h.write(this + 8, 0_u32);
        h.write(this + 4, 0_u32);
    }

    /// `PostProcessTweakUtil_terminate(this)`: `TextUtil_terminate(this +
    /// 8)` (a tail jump), its `eax`.
    pub fn tweak_terminate(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(TEXT_UTIL_TERMINATE, Conv::Thiscall(this + 8), &[])
    }

    /// `MorphManager_clear_006acbd0(this)`: the four arrays (count `+0`,
    /// `+8`, `+0x10`, `+0x18` low 31 bits; data the next word) zeroed
    /// (8-byte elements, 16 for the last), then each freed when it has
    /// data and is owned (top bit clear) and cleared.
    pub fn morph_clear(h: &mut impl Host, this: u32) {
        for (off, elem) in [(0, 8), (8, 8), (0x10, 8), (0x18, 16)] {
            let n = word(h, this + off) & 0x7fff_ffff;
            for k in 0..n {
                let at = word(h, this + off + 4) + elem * k;
                for w in 0..elem / 4 {
                    h.write(at + 4 * w, 0_u32);
                }
            }
        }
        for off in [0, 8, 0x10, 0x18] {
            let c = word(h, this + off);
            if c == 0 {
                continue;
            }
            let data = word(h, this + off + 4);
            if data != 0 && c >> 31 == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
            h.write(this + off + 4, 0_u32);
            h.write(this + off, 0_u32);
        }
    }

    /// `BootText_releaseBuffer_006ddfa0(this)`: with the byte `+1` set,
    /// the buffer `+0x10c` freed and cleared; the half-word `+0` cleared.
    pub fn boot_text_release_buffer(h: &mut impl Host, this: u32) {
        if byte(h, this + 1) != 0 {
            let p = word(h, this + 0x10c);
            if p != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
            }
            h.write(this + 0x10c, 0_u32);
        }
        h.write(this, 0_u16);
    }

    /// `TextUtilEntry_dtor_00692950(this)` -> the string builder's
    /// destructor's `eax`: the tagged buffer `+0x7c` freed unless its tag
    /// bit is set; the owned word array `+0x78` (count `+0x74`, top bit
    /// clear) released; both cleared; the string builder `+0x14`
    /// destroyed.
    pub fn text_entry_dtor(h: &mut impl Host, this: u32) -> u32 {
        if byte(h, this + 0x7c) & 1 == 0 {
            let p = word(h, this + 0x7c);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
        }
        let c = word(h, this + 0x74);
        if c >> 31 == 0 {
            let data = word(h, this + 0x78);
            h.call_sym(RELEASE_TAGGED_WORDS, Conv::Cdecl, &[data, c & 0x7fff_ffff]);
        }
        h.write(this + 0x78, 0_u32);
        h.write(this + 0x74, 0_u32);
        h.call_sym(STRING_BUILDER_DTOR, Conv::Thiscall(this + 0x14), &[])
    }

    /// `BootText_terminate_006ddfd0(this)`: with the byte `+5` set and a
    /// text util `[+0]`: the half-word `+4` cleared, `TextUtil_terminate`,
    /// the entry destroyed ([`text_entry_dtor`]) and deleted, `[+0]`
    /// cleared.
    pub fn boot_text_terminate(h: &mut impl Host, this: u32) {
        if byte(h, this + 5) == 0 {
            return;
        }
        let util = word(h, this);
        if util == 0 {
            return;
        }
        h.write(this + 4, 0_u16);
        h.call_sym(TEXT_UTIL_TERMINATE, Conv::Thiscall(util), &[]);
        let e = word(h, this);
        if e != 0 {
            h.call_sym(TEXT_ENTRY_DTOR, Conv::Thiscall(e), &[]);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[e]);
        }
        h.write(this, 0_u32);
    }

    /// `PhyreScene_releaseCluster_*(scene)` -> 0, for the cluster word at
    /// `scene + off` ([`SCENE_CLUSTERS`]): when set and the cluster
    /// manager holds it (`0x0069b890`, `al`), released.
    pub fn scene_release_cluster(h: &mut impl Host, scene: u32, off: u32) -> u32 {
        let c = word(h, scene + off);
        if c != 0 {
            let m = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
            if h.call_sym(CLUSTER_HELD, Conv::Thiscall(m), &[c]) & 0xff != 0 {
                let c = word(h, scene + off);
                let m = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
                h.call_sym(CLUSTER_RELEASE, Conv::Thiscall(m), &[c]);
            }
        }
        0
    }
}
