//! The render-side glue, fifth batch (M4 port round 2, list
//! `h3-glue-render`): the scene cursors' `begin`/`next`, small array and
//! vector pushes, the chr cluster delay release, the 2D VFX setup, and
//! the functions that hand the addresses of their own locals or
//! arguments to callees.
//!
//! Frames: a bridge whose original passes `&local` or `&argument` takes
//! `ebp`, the address standing for the original's frame pointer, and lays
//! the locals out at the original's `ebp` offsets below it and the
//! arguments at `ebp + 8` and up (docs/porting.md, "Library code"). The
//! shims pass a [`StackFrame`](../../ffx_hook) of their own; the oracle
//! tests place it at the original's address (`Oracle::set_call_top`).

use ffx_bridge::Global;

pub use crate::glue::SCENE;
pub use crate::glue4::VFX_RENDER_DATA_TABLE;

/// `g_DreamOn`.
pub const DREAM_ON: Global<u32> = Global::new(0x012f_b8f0);
/// `g_DreamPhase`.
pub const DREAM_PHASE: Global<i32> = Global::new(0x012f_b8f4);
/// `g_CurrentMapNo`.
pub const CURRENT_MAP_NO: Global<u32> = Global::new(0x012f_bc40);
/// `g_GraphicGameOverNotify`.
pub const GAME_OVER_NOTIFY: Global<u32> = Global::new(0x00cc_b468);

/// `PhyreSceneCursor_SetNode(this, node)`.
pub const CURSOR_SET_NODE: u32 = 0x0044_1dc0;
/// `PhyreCursor_IsValid_copy_00637f80(this)` (`al`).
pub const CURSOR_IS_VALID_637F80: u32 = 0x0063_7f80;
/// `PhyreCursor_IsValid_00660270(this)`.
pub const CURSOR_IS_VALID_660270: u32 = 0x0066_0270;
/// `PhyreCursor_IsValid_006b28f0(this)`.
pub const CURSOR_IS_VALID_6B28F0: u32 = 0x006b_28f0;
/// `Array_Reserve_00668200(this, n)` (20-byte entries).
pub const ARRAY_RESERVE_668200: u32 = 0x0066_8200;
/// `Phyre_PCluster_deleteObject(this, object)`.
pub const CLUSTER_DELETE_OBJECT: u32 = 0x0044_1240;
/// The vector's growth (`0x00655c60`, thiscall, `ret 4`).
pub const VECTOR_GROW: u32 = 0x0065_5c60;
/// `PhyreScene_getCurrentLevelNameHash(this)`.
pub const SCENE_LEVEL_NAME_HASH: u32 = 0x0065_de90;
/// `ClusterManager_GetInstance`.
pub const CLUSTER_INSTANCE: u32 = crate::glue2::CLUSTER_INSTANCE;
/// `ChrResourceCacheManager_enableChrDelayRelease(this)`.
pub const CHR_ENABLE_DELAY_RELEASE: u32 = 0x0069_ae80;
/// `ChrResourceCacheManager_isChrDelayedRelease(this)`.
pub const CHR_IS_DELAYED_RELEASE: u32 = 0x0069_b7b0;
/// `ChrResourceCacheManager_applyDelayedRelease(this)`.
pub const CHR_APPLY_DELAYED_RELEASE: u32 = 0x0069_a6b0;
/// The message `graphicEnableChrClusterDelayRelease` prints before the
/// delayed release.
pub const DELAYED_RELEASE_MESSAGE: u32 = 0x00b4_1808;
/// `__iob_func`'s import slot (cdecl).
pub const IOB_FUNC: u32 = crate::glue4::IOB_FUNC;
/// `fprintf`'s import slot (cdecl).
pub const FPRINTF: u32 = crate::glue4::FPRINTF;
/// `ClassVFXRenderDataTable_GetDynamicGeometryByInstance(this, lo, hi)`.
pub const VFX_GET_DYN_GEO_BY_INSTANCE: u32 = crate::glue4::VFX_GET_DYN_GEO_BY_INSTANCE;
/// `Phyre_PhyrePrintf(level, format, ...)` (cdecl).
pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
/// `graphicVFX2DSetup`'s message for a null instance.
pub const VFX_2D_NULL_MESSAGE: u32 = 0x00b4_1a6c;
/// `Phyre_PFramework_PApplication_GetApplication`.
pub const GET_APPLICATION: u32 = crate::glue::GET_APPLICATION;
/// `PApplication_FixupClusters(&cluster, n)` (cdecl).
pub const FIXUP_CLUSTERS: u32 = 0x0062_3580;
/// `AsyncLoadManager_sGetInstance`.
pub const ASYNC_LOAD_INSTANCE: u32 = crate::glue::ASYNC_LOAD_INSTANCE;
/// The VFX shader manager's shader registration (`0x006c0810`, thiscall,
/// `ret 8`).
pub const VFX_SHADER_REGISTER: u32 = 0x006c_0810;
/// `ShaderParams_FindByName(this, name)` (`0x0056cbf0`).
pub const SHADER_PARAMS_FIND: u32 = 0x0056_cbf0;
/// The shader parameter name `setMaterialVisibilityBySetAlpha` looks up.
pub const ALPHA_PARAM_NAME: u32 = 0x00b4_1980;
/// `ShaderParamBuffer_SetValue(this, param, value)`.
pub const SHADER_PARAM_SET_VALUE: u32 = 0x0063_8780;
/// `PeekMessageW`'s import slot (stdcall).
pub const PEEK_MESSAGE: u32 = 0x00b0_c64c;
/// `TranslateMessage`'s import slot (stdcall).
pub const TRANSLATE_MESSAGE: u32 = 0x00b0_c690;
/// `DispatchMessageW`'s import slot (stdcall).
pub const DISPATCH_MESSAGE: u32 = 0x00b0_c650;
/// `memcpy` (cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
/// `ClassVFXRenderDataTable_MapFind(this, &it, &key)`.
pub const VFX_MAP_FIND: u32 = 0x006a_1f60;
/// `ClassVFXRenderDataTable_InsertToDeletePool(this, lo, hi, n, mesh)`.
pub const VFX_INSERT_TO_DELETE_POOL: u32 = 0x0069_f900;
/// The map's erase (`0x006a1a70`, thiscall `(&it, node)`).
pub const VFX_MAP_ERASE: u32 = 0x006a_1a70;
/// The map's node allocation (`0x0069c5f0`, thiscall `(&value)`).
pub const VFX_MAP_NEW_NODE: u32 = 0x0069_c5f0;
/// The map's insert (`0x0069cfe0`, thiscall, 4 words).
pub const VFX_MAP_INSERT: u32 = 0x0069_cfe0;
/// `PMatrix4_FromMatrix3x4_00451080(this, m)`.
pub const MATRIX4_FROM_3X4: u32 = 0x0045_1080;
/// `PMatrix4_Multiply_004ca360(out, a, b)` (cdecl, returns `out`).
pub const MATRIX4_MULTIPLY: u32 = 0x004c_a360;
/// `BoundingBox_IsVisible_00663a30(this, matrix)`.
pub const BOUNDS_IS_VISIBLE: u32 = 0x0066_3a30;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::glue::bridge::{instance, on_global};
    use ffx_graphic::host::{Conv, Host};

    /// Calls the import whose slot is at `slot`; its `eax`.
    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let at = h.rebase(slot);
        let f: u32 = h.read(at);
        h.call(f, conv, args)
    }

    /// One step of a scene cursor from `node` (the cursor's `+0x18` as
    /// the loop holds it): the next node (`[node]`), or 0 at the end
    /// (`+0x14`), stored back. Returns the new node.
    fn cursor_step(h: &mut impl Host, this: u32, node: u32) -> u32 {
        if node == 0 {
            return 0;
        }
        let next: u32 = h.read(node);
        let end: u32 = h.read(this + 0x14);
        let node = if next == end { 0 } else { next };
        h.write(this + 0x18, node);
        node
    }

    /// Whether the cursor is on a valid node (`is_valid`'s `al`).
    fn cursor_valid(h: &mut impl Host, this: u32, is_valid: u32) -> bool {
        h.call_sym(is_valid, Conv::Thiscall(this), &[]) & 0xff != 0
    }

    /// The cursor's end: `SetNode(+0x18)` (its `eax`), or the first five
    /// words cleared when there is no node (`eax` 0).
    fn cursor_settle(h: &mut impl Host, this: u32) -> u32 {
        let node: u32 = h.read(this + 0x18);
        if node != 0 {
            return h.call_sym(CURSOR_SET_NODE, Conv::Thiscall(this), &[node]);
        }
        for k in 0..5 {
            h.write(this + 4 * k, 0u32);
        }
        0
    }

    /// `PhyreSceneCursor_Next_006377a0` and its copy
    /// `PhyreSceneCursor_Next_006b22e0(this)`: steps until `is_valid`,
    /// then settles.
    pub fn cursor_next(h: &mut impl Host, this: u32, is_valid: u32) -> u32 {
        let mut node: u32 = h.read(this + 0x18);
        loop {
            node = cursor_step(h, this, node);
            if cursor_valid(h, this, is_valid) {
                break;
            }
        }
        cursor_settle(h, this)
    }

    /// `PhyreSceneCursor_Begin_00637730` and its copies `_0065eb60`,
    /// `_006b2230(this)`: unless already valid, steps until `is_valid`;
    /// then settles.
    pub fn cursor_begin(h: &mut impl Host, this: u32, is_valid: u32) -> u32 {
        if !cursor_valid(h, this, is_valid) {
            let mut node: u32 = h.read(this + 0x18);
            loop {
                node = cursor_step(h, this, node);
                if cursor_valid(h, this, is_valid) {
                    break;
                }
            }
        }
        cursor_settle(h, this)
    }

    /// `PArray_pushBack20_006562f0(this, entry)` (`ret 4`): appends the
    /// 20-byte entry (count `+0`, capacity `+4`, data `+8`;
    /// `Array_Reserve_00668200(n + 2 + n / 8)` when full); returns 0.
    pub fn parray_push_back20(h: &mut impl Host, this: u32, entry: u32) -> u32 {
        let n = h.read::<i32>(this).wrapping_add(1);
        h.write(this, n);
        if n > h.read::<i32>(this + 4) {
            let grown = (n / 8).wrapping_add(n).wrapping_add(2);
            h.call_sym(ARRAY_RESERVE_668200, Conv::Thiscall(this), &[grown as u32]);
        }
        let n: u32 = h.read(this);
        let data: u32 = h.read(this + 8);
        let slot = data.wrapping_add(n.wrapping_mul(20)).wrapping_sub(20);
        if slot != 0 {
            for k in 0..5 {
                let v: u32 = h.read(entry + 4 * k);
                h.write(slot + 4 * k, v);
            }
        }
        0
    }

    /// `PhyreScene_deleteObjectInCluster_0065a250(this, object, cluster)`
    /// (`ret 8`): an object of the scene's own pool cluster (`+0x12d4`,
    /// 16-byte slots from `+0xfbb8`, at most 3000) frees its slot (byte
    /// `[+0xfbc4] + i` = 0, lowest free index `+0xfbc8`) and runs its
    /// virtual destructor (slot 0, argument 0; its `eax`); others go to
    /// `cluster->deleteObject(object + 4)`. `eax` otherwise:
    /// `this + 0x12d4`.
    pub fn scene_delete_object_in_cluster(
        h: &mut impl Host,
        this: u32,
        object: u32,
        cluster: u32,
    ) -> u32 {
        let pool = this + 0x12d4;
        if cluster == pool {
            let base: u32 = h.read(this + 0xfbb8);
            let i = (object.wrapping_sub(base) as i32) >> 4;
            if i as u32 <= 0xbb7 {
                let used: u32 = h.read(this + 0xfbc4);
                h.write(used.wrapping_add(i as u32), 0u8);
                if i < h.read::<i32>(this + 0xfbc8) {
                    h.write(this + 0xfbc8, i);
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
        pool
    }

    /// `std_vector_push_back_00667620(this, value)` (`ret 4`): a
    /// `std::vector` of words (`+0` begin, `+4` end, `+8` capacity end)
    /// appends `*value`; a `value` inside the vector is re-read by index
    /// after the growth (`0x00655c60(1)` when full).
    pub fn vector_push_back(h: &mut impl Host, this: u32, value: u32) {
        let end: u32 = h.read(this + 4);
        let begin: u32 = h.read(this);
        let inside = value < end && begin <= value;
        let index = (value.wrapping_sub(begin) as i32) >> 2;
        if end == h.read::<u32>(this + 8) {
            h.call_sym(VECTOR_GROW, Conv::Thiscall(this), &[1]);
        }
        let slot: u32 = h.read(this + 4);
        if slot != 0 {
            let v: u32 = if inside {
                let begin: u32 = h.read(this);
                h.read(begin.wrapping_add((index as u32).wrapping_mul(4)))
            } else {
                h.read(value)
            };
            h.write(slot, v);
        }
        let end: u32 = h.read(this + 4);
        h.write(this + 4, end.wrapping_add(4));
    }

    /// `graphicEnableChrClusterDelayRelease()` `0x00640090`: outside maps
    /// 0x17 and 0x9f and without a game over, enables the chr resource
    /// cache's delayed release; else applies a pending one (with a
    /// message on stdout). `eax`: the callee's, or 0 with nothing
    /// pending.
    pub fn enable_chr_cluster_delay_release(h: &mut impl Host) -> u32 {
        on_global(h, SCENE, SCENE_LEVEL_NAME_HASH, &[]);
        let map = CURRENT_MAP_NO.get(h);
        if map != 0x17 && GAME_OVER_NOTIFY.get(h) == 0 && map != 0x9f {
            let cache = instance(h, CLUSTER_INSTANCE);
            return h.call_sym(CHR_ENABLE_DELAY_RELEASE, Conv::Thiscall(cache), &[]);
        }
        let cache = instance(h, CLUSTER_INSTANCE);
        if h.call_sym(CHR_IS_DELAYED_RELEASE, Conv::Thiscall(cache), &[]) == 0 {
            return 0;
        }
        let message = h.rebase(DELAYED_RELEASE_MESSAGE);
        let iob = import(h, IOB_FUNC, Conv::Cdecl, &[]);
        import(h, FPRINTF, Conv::Cdecl, &[iob.wrapping_add(0x20), message]);
        let cache = instance(h, CLUSTER_INSTANCE);
        h.call_sym(CHR_APPLY_DELAYED_RELEASE, Conv::Thiscall(cache), &[])
    }

    /// `graphicVFX2DSetup(lo, hi)` `0x00644b50`: the instance's dynamic
    /// geometry, unless its virtual slot 1 accepts the scene's 2D target
    /// (`+0x1324`, 0), gets slot 2 (1) and the result 1; else 0. A null
    /// instance prints a message.
    pub fn vfx_2d_setup(h: &mut impl Host, lo: u32, hi: u32) -> u32 {
        if lo | hi == 0 {
            let message = h.rebase(VFX_2D_NULL_MESSAGE);
            h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[1, message]);
            return 0;
        }
        let table = VFX_RENDER_DATA_TABLE.get(h);
        let geo = h.call_sym(
            VFX_GET_DYN_GEO_BY_INSTANCE,
            Conv::Thiscall(table),
            &[lo, hi],
        );
        if geo == 0 {
            return 0;
        }
        let target = SCENE.get(h).wrapping_add(0x1324);
        if h.call_virtual(geo, 1, &[target, 0]) != 0 {
            return 0;
        }
        h.call_virtual(geo, 2, &[1]);
        1
    }

    /// `PhyreScene_onMenuTextureLoaded(cluster)` `0x00666740` (frame:
    /// the argument at `ebp + 8`): `FixupClusters(&cluster, 1)`; during
    /// the dream effect, advances its phase up to 5. `eax`: the phase, or
    /// the fixup's.
    pub fn scene_on_menu_texture_loaded(h: &mut impl Host, ebp: u32, cluster: u32) -> u32 {
        h.write(ebp + 8, cluster);
        h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let r = h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[ebp + 8, 1]);
        if DREAM_ON.get(h) == 0 {
            return r;
        }
        let mut phase = DREAM_PHASE.get(h);
        if phase < 5 {
            phase += 1;
            DREAM_PHASE.set(h, phase);
        }
        phase as u32
    }

    /// `VfxShaderManager_onShaderLoaded_006c13e0(cluster, task)` (frame:
    /// the arguments at `ebp + 8`, `+0xc`): with a cluster, `task =
    /// (manager, id)`; the task slot becomes `id`, then
    /// `manager->register(&id, &cluster)` and `FixupClusters(&cluster,
    /// 1)` (its `eax`; else the loader instance).
    pub fn vfx_shader_on_loaded(h: &mut impl Host, ebp: u32, cluster: u32, task: u32) -> u32 {
        h.write(ebp + 8, cluster);
        h.write(ebp + 0xc, task);
        let loader = instance(h, ASYNC_LOAD_INSTANCE);
        if cluster == 0 {
            return loader;
        }
        let id: u32 = h.read(task + 4);
        let manager: u32 = h.read(task);
        h.write(ebp + 0xc, id);
        h.call_sym(
            VFX_SHADER_REGISTER,
            Conv::Thiscall(manager),
            &[ebp + 0xc, ebp + 8],
        );
        h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[ebp + 8, 1])
    }

    /// `PhyreScene_setMaterialVisibilityBySetAlpha(material, visible)`
    /// `0x0066d5c0` (stdcall; frame: the colour at `ebp - 0x10`): sets the
    /// material's alpha parameter (found in `[material]`, set through
    /// `[material + 4]`) to (1, 1, 1, `visible` as a float).
    pub fn scene_set_material_visibility(h: &mut impl Host, ebp: u32, material: u32, visible: u32) {
        if material == 0 {
            return;
        }
        let params: u32 = h.read(material);
        let name = h.rebase(ALPHA_PARAM_NAME);
        let param = h.call_sym(SHADER_PARAMS_FIND, Conv::Thiscall(params), &[name]);
        if param == 0 {
            return;
        }
        let colour = ebp.wrapping_sub(0x10);
        for k in 0..3 {
            h.write(colour + 4 * k, 0x3f80_0000u32);
        }
        // fild dword, fstp dword: the integer rounded once to a float
        #[allow(clippy::cast_precision_loss)] // the original rounds it too
        let alpha = (visible as i32) as f32;
        h.write(colour + 12, alpha);
        let buffer: u32 = h.read(material + 4);
        h.call_sym(
            SHADER_PARAM_SET_VALUE,
            Conv::Thiscall(buffer),
            &[param, colour],
        );
    }

    /// `graphicProcessWindowsMessage()` `0x00642860` (frame: the `MSG` at
    /// `ebp - 0x1c`): peeks and removes one message, translates and
    /// dispatches it; 1 for `WM_QUIT` (0x12), else 0.
    pub fn process_windows_message(h: &mut impl Host, ebp: u32) -> u32 {
        let msg = ebp.wrapping_sub(0x1c);
        h.write(msg + 4, 0u32);
        if import(h, PEEK_MESSAGE, Conv::Stdcall, &[msg, 0, 0, 0, 1]) == 0 {
            return 0;
        }
        import(h, TRANSLATE_MESSAGE, Conv::Stdcall, &[msg]);
        import(h, DISPATCH_MESSAGE, Conv::Stdcall, &[msg]);
        u32::from(h.read::<u32>(msg + 4) == 0x12)
    }

    /// `BrokenScreenEffect_setConstant_006d46a0` (`field` `0xa0`) and
    /// `_006d4710` (`0xa4`) `(this, 4 words)` (`ret 0x10`; frame: the
    /// words at `ebp + 8`): copies the words into the constant buffer
    /// (`[[+0x1c] + 4]`, size at `+0`) at the entry's offset (`+8`) and
    /// length (`+0xa & 0x1fff`). 0x13 without the entry, 3 when it does
    /// not fit or starts below 4, else 0.
    ///
    /// The original copies from its own argument slots: a length above 16
    /// reads the caller's stack past them (none in the game's data).
    pub fn broken_set_constant(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        words: [u32; 4],
        field: u32,
    ) -> u32 {
        for (k, w) in (0u32..).zip(words) {
            h.write(ebp + 8 + 4 * k, w);
        }
        let entry: u32 = h.read(this + field);
        if entry == 0 {
            return 0x13;
        }
        let owner: u32 = h.read(this + 0x1c);
        let offset = u32::from(h.read::<u16>(entry + 8));
        let buffer: u32 = h.read(owner.wrapping_add(4));
        let len = u32::from(h.read::<u16>(entry + 0xa) & 0x1fff);
        if offset + len > h.read::<u32>(buffer) || offset < 4 {
            return 3;
        }
        h.call_sym(
            MEMCPY,
            Conv::Cdecl,
            &[offset.wrapping_add(buffer), ebp + 8, len],
        );
        0
    }

    /// `ClassVFXRenderDataTable_Remove_0069fc10(this, lo, hi)` (`ret 8`;
    /// frame: the iterator at `ebp - 4`, the key at `ebp + 8`): a found
    /// entry's mesh (`+0x18`) goes to the delete pool (3) and the entry
    /// is erased; the cached key (`+0x20`) is cleared when it matches.
    /// Returns `lo`.
    pub fn vfx_table_remove(h: &mut impl Host, ebp: u32, this: u32, lo: u32, hi: u32) -> u32 {
        h.write(ebp + 8, lo);
        h.write(ebp + 0xc, hi);
        let it = ebp.wrapping_sub(4);
        h.call_sym(VFX_MAP_FIND, Conv::Thiscall(this), &[it, ebp + 8]);
        let node: u32 = h.read(it);
        let hi: u32 = h.read(ebp + 0xc);
        if node != h.read::<u32>(this) {
            let lo: u32 = h.read(ebp + 8);
            let mesh: u32 = h.read(node.wrapping_add(0x18));
            h.call_sym(
                VFX_INSERT_TO_DELETE_POOL,
                Conv::Thiscall(this),
                &[lo, hi, 3, mesh],
            );
            h.call_sym(VFX_MAP_ERASE, Conv::Thiscall(this), &[it, node]);
        }
        let lo: u32 = h.read(ebp + 8);
        if h.read::<u32>(this + 0x20) == lo && h.read::<u32>(this + 0x24) == hi {
            h.write(this + 0x20, 0u32);
            h.write(this + 0x24, 0u32);
        }
        lo
    }

    /// `ClassVFXRenderDataTable_Insert(this, lo, hi, geometry)`
    /// `0x0069f890` (`ret 0xc`; frame: the value `(lo, hi, geometry)` at
    /// `ebp - 0x10`, the iterator at `ebp - 4`, the key at `ebp + 8`): a
    /// found entry gets the geometry (`+0x18`), else a new node is
    /// inserted; the key and geometry are cached (`+0x20`..`+0x28`).
    /// Returns `lo`.
    pub fn vfx_table_insert(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        lo: u32,
        hi: u32,
        geometry: u32,
    ) -> u32 {
        h.write(ebp + 8, lo);
        h.write(ebp + 0xc, hi);
        h.write(ebp + 0x10, geometry);
        let it = ebp.wrapping_sub(4);
        h.call_sym(VFX_MAP_FIND, Conv::Thiscall(this), &[it, ebp + 8]);
        let node: u32 = h.read(it);
        let hi: u32 = h.read(ebp + 0xc);
        let geometry: u32 = h.read(ebp + 0x10);
        if node == h.read::<u32>(this) {
            let value = ebp.wrapping_sub(0x10);
            let lo: u32 = h.read(ebp + 8);
            h.write(value, lo);
            h.write(value + 4, hi);
            h.write(value + 8, geometry);
            let new = h.call_sym(VFX_MAP_NEW_NODE, Conv::Thiscall(this), &[value]);
            h.call_sym(
                VFX_MAP_INSERT,
                Conv::Thiscall(this),
                &[value + 8, 0, new.wrapping_add(0x10), new],
            );
        } else {
            h.write(node.wrapping_add(0x18), geometry);
        }
        let lo: u32 = h.read(ebp + 8);
        h.write(this + 0x28, geometry);
        h.write(this + 0x24, hi);
        h.write(this + 0x20, lo);
        lo
    }

    /// `MeshInstance_isVisible_00663cc0(this, matrix)` (`ret 4`; frame:
    /// two matrices at `ebp - 0x80`, `ebp - 0x40`): the bounds test with
    /// `matrix` times the instance's 3x4 world matrix (`+0xc`), or with a
    /// copy of `matrix` when there is none.
    pub fn mesh_instance_is_visible(h: &mut impl Host, ebp: u32, this: u32, matrix: u32) -> u32 {
        let world: u32 = h.read(this + 0xc);
        let m40 = ebp.wrapping_sub(0x40);
        if world != 0 {
            h.call_sym(MATRIX4_FROM_3X4, Conv::Thiscall(m40), &[world]);
            let m80 = ebp.wrapping_sub(0x80);
            let r = h.call_sym(MATRIX4_MULTIPLY, Conv::Cdecl, &[m80, matrix, m40]);
            return h.call_sym(BOUNDS_IS_VISIBLE, Conv::Thiscall(this), &[r]);
        }
        for k in 0..16 {
            let v: u32 = h.read(matrix.wrapping_add(4 * k));
            h.write(m40 + 4 * k, v);
        }
        h.call_sym(BOUNDS_IS_VISIBLE, Conv::Thiscall(this), &[m40])
    }
}
