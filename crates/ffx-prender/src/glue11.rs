//! The render-side glue, eleventh batch (M4 port round 3, list
//! `h3-glue-render`): two more tree inserts, a render-texture holder's
//! set-up, the cut-buffer shader load, the mesh-group visibility walk,
//! the dynamic-geometry buffer swap, the render-to-texture target, the
//! material delete and the named-pair copy. Exception frames and cookies
//! left out, as in [`crate::glue10`].

use ffx_bridge::Global;

pub use crate::glue::SCENE;

/// `Phyre_PFreeList_allocateNewBlock(this, n)`.
pub const FREE_LIST_ALLOCATE_BLOCK: u32 = crate::glue6::FREE_LIST_ALLOCATE_BLOCK;
/// The tree's node constructor `0x0064cb00(node; a, b)`, insert
/// `0x0064c0a0(this; node)` (non-zero: a duplicate) and the duplicate's
/// release `0x0064b8c0(list; node)`.
pub const NODE_CTOR_64CB00: u32 = 0x0064_cb00;
pub const TREE_INSERT_64C0A0: u32 = 0x0064_c0a0;
pub const NODE_RELEASE_64B8C0: u32 = 0x0064_b8c0;
/// `Phyre_PRedBlackTree_postInsertFinalize(this, node)`.
pub const TREE_POST_INSERT: u32 = crate::glue6::TREE_POST_INSERT;
/// The array copy `0x005281e0(n, data)` (cdecl, a new buffer).
pub const ARRAY_COPY_5281E0: u32 = 0x0052_81e0;
/// `Phyre_PhyreFree(p)`, `Phyre_PhyreMalloc(size)` (cdecl).
pub const PHYRE_FREE: u32 = crate::glue8::PHYRE_FREE;
pub const PHYRE_MALLOC_SIZE: u32 = 0x0042_fb40;
/// `Phyre_PhyreMalloc_0042fb80(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = crate::glue8::PHYRE_MALLOC;
/// `Phyre_PString_Assign(this, text)`.
pub const PSTRING_ASSIGN: u32 = 0x0044_5cf0;
/// The shader-bin load pieces ([`crate::glue10`]).
pub const FIOS_UNIFY_FILENAME: u32 = crate::glue8::FIOS_UNIFY_FILENAME;
pub const READER_CTOR: u32 = crate::glue10::READER_CTOR;
pub const READER_IS_OPEN: u32 = crate::glue10::READER_IS_OPEN;
pub const READER_CLOSE: u32 = crate::glue10::READER_CLOSE;
pub const READER_DTOR: u32 = crate::glue10::READER_DTOR;
/// The cut-buffer shader's file name and its loaded callback.
pub const CUTBUFFER_NAME: u32 = 0x00b4_2990;
pub const ON_CUTBUFFER_LOADED: u32 = 0x0066_6610;
/// `AsyncLoadManager_sGetInstance` and `loadCluster(manager, 6 words)`.
pub const ASYNC_LOAD_INSTANCE: u32 = crate::glue::ASYNC_LOAD_INSTANCE;
pub const LOAD_CLUSTER: u32 = crate::glue7::LOAD_CLUSTER;
/// `PRedBlackTree_iteratorBegin_0064dae0(iterator; tree)` and
/// `Phyre_PRedBlackTree_successor(this, node)`.
pub const TREE_ITERATOR_BEGIN: u32 = 0x0064_dae0;
pub const TREE_SUCCESSOR: u32 = crate::glue9::TREE_SUCCESSOR;
/// The swap's state: `g_SaveData_ScreenState`, `g_PauseOn`,
/// `g_GraphicCharacterLoading`.
pub const SCREEN_STATE: Global<u32> = Global::new(0x00cc_b994);
pub const PAUSE_ON: Global<u32> = Global::new(0x0134_0828);
pub const CHARACTER_LOADING: Global<u32> = crate::glue10::CHARACTER_LOADING;
/// `graphicIsMapInList()`, `AsyncLoadManager_isAllRequestDone(this)`
/// (`al`), `PostProcessManager_GetInstance()`.
pub const IS_MAP_IN_LIST: u32 = 0x0064_2610;
pub const ALL_REQUESTS_DONE: u32 = 0x0069_1630;
pub const POST_PROCESS_INSTANCE: u32 = crate::glue2::POST_PROCESS_INSTANCE;
/// `FreeCamera_CreateInstance_0042f260()` (the getter).
pub const FREE_CAMERA_INSTANCE: u32 = 0x0042_f260;
/// `g_DynGeoFillBuffer`, `DynGeoMemManager_swapDynGeoMemory(this)`.
pub const DYN_GEO_FILL: Global<u32> = crate::glue::DYN_GEO_FILL;
pub const DYN_GEO_SWAP: u32 = 0x006d_ed40;
/// `g_VFXRenderDataTable`, `ClearVFXDrawData(this)`.
pub const VFX_RENDER_DATA_TABLE: Global<u32> = crate::glue4::VFX_RENDER_DATA_TABLE;
pub const VFX_CLEAR_DRAW_DATA: u32 = 0x0069_f2b0;
/// The render-to-texture target: `Phyre_PCluster_create(this; n, class,
/// 0)`, the texture's `setDimensions` and `updateMaxLevel`, the
/// instance list allocation (`ret 0xc`) and the target constructor
/// `0x00583a60(this; texture)`; the classes and the target's name.
pub const CLUSTER_CREATE: u32 = crate::glue9::CLUSTER_CREATE;
pub const TEXTURE_SET_DIMENSIONS: u32 = crate::glue10::TEXTURE_SET_DIMENSIONS;
pub const TEXTURE_UPDATE_MAX_LEVEL: u32 = 0x004c_4250;
pub const CLUSTER_ALLOCATE_INSTANCE_LIST: u32 = 0x0044_0b70;
pub const TARGET_CTOR: u32 = 0x0058_3a60;
pub const TEXTURE_CLASS: u32 = 0x00ca_37d0;
pub const TEXTURE_FORMAT: u32 = crate::glue10::SHADOW_TEXTURE_FORMAT;
pub const TARGET_CLASS: u32 = 0x00ca_4200;
pub const TARGET_NAME: u32 = 0x00b4_3c40;
/// `Phyre_PCluster_deleteObjectWithInstanceList(this; object, list)`.
pub const CLUSTER_DELETE_WITH_LIST: u32 = 0x0044_13a0;
/// `memcpy` (cdecl).
pub const MEMCPY: u32 = 0x0094_9256;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::glue::bridge::instance;
    use crate::glue7::cdq;
    use ffx_bridge::x87::round_trip;
    use ffx_graphic::host::{Conv, Host};

    /// `PRedBlackTree_insertPair_0065fdd0(this, a, b)` (`ret 8`): a node
    /// from the free list (`+0x18`, refilled from `+0x20`; 0xe when none)
    /// built by `0x0064cb00(a, b)` and inserted (`0x0064c0a0`): a
    /// duplicate goes back (`0x0064b8c0`, 0x12), else the tree is
    /// rebalanced (0).
    pub fn tree_insert_pair(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        let list = this + 0x18;
        let mut node: u32 = h.read(list);
        if node == 0 {
            let n: u32 = h.read(this + 0x20);
            h.call_sym(FREE_LIST_ALLOCATE_BLOCK, Conv::Thiscall(list), &[n]);
            node = h.read(list);
        }
        if node == 0 {
            return 0xe;
        }
        let next: u32 = h.read(node);
        h.write(list, next);
        let built = h.call_sym(NODE_CTOR_64CB00, Conv::Thiscall(node), &[a, b]);
        if h.call_sym(TREE_INSERT_64C0A0, Conv::Thiscall(this), &[built]) != 0 {
            h.call_sym(NODE_RELEASE_64B8C0, Conv::Thiscall(list), &[built]);
            return 0x12;
        }
        h.call_sym(TREE_POST_INSERT, Conv::Thiscall(this), &[built]);
        0
    }

    /// `RenderTextureInfo_set_006b2400(this, &array, name, path, f0..f4)`
    /// (`ret 0x20`): the array copied into a new buffer (`+0xc`, count
    /// `+8`; the old owned one freed), the two strings assigned (`+4`,
    /// `+0x20`), the floats copied through the x87 (`+0x24..+0x30`,
    /// `+0x10`) and two empty 8-byte holders made (`+0x38`, `+0x3c`; 0
    /// when the allocation fails). `eax`: the second holder.
    pub fn render_texture_info_set(
        h: &mut impl Host,
        this: u32,
        array: u32,
        name: u32,
        path: u32,
        f: [u32; 5],
    ) -> u32 {
        let count = h.read::<u32>(array) & 0x7fff_ffff;
        let data: u32 = h.read(array + 4);
        let copy = h.call_sym(ARRAY_COPY_5281E0, Conv::Cdecl, &[count, data]);
        let old: u32 = h.read(this + 0xc);
        if old != copy && h.read::<u32>(this + 8) >> 31 == 0 && old != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(this + 0xc, copy);
        h.write(this + 8, count);
        h.call_sym(PSTRING_ASSIGN, Conv::Thiscall(this + 4), &[name]);
        h.call_sym(PSTRING_ASSIGN, Conv::Thiscall(this + 0x20), &[path]);
        for (off, bits) in [
            (0x24, f[0]),
            (0x28, f[1]),
            (0x2c, f[2]),
            (0x30, f[3]),
            (0x10, f[4]),
        ] {
            h.write(this + off, round_trip(f32::from_bits(bits)).to_bits());
        }
        let mut holder = 0;
        for off in [0x38, 0x3c] {
            holder = h.call_sym(PHYRE_MALLOC_SIZE, Conv::Cdecl, &[8]);
            if holder != 0 {
                h.write(holder, 0u32);
                h.write(holder + 4, 0u32);
            }
            h.write(this + off, holder);
        }
        holder
    }

    /// `PhyreScene_loadCutbufferShader(this)` `0x00664440` (frame as the
    /// shader-bin loaders'): when the file opens, an asynchronous cluster
    /// load (`onCutbufferShaderLoaded`, task `(i64) this`), 0; else 5.
    pub fn scene_load_cutbuffer_shader(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let path = ebp.wrapping_sub(0x110);
        let reader = ebp.wrapping_sub(0x12c);
        let name = h.rebase(CUTBUFFER_NAME);
        h.call_sym(FIOS_UNIFY_FILENAME, Conv::Cdecl, &[name, path, 0x100]);
        h.call_sym(READER_CTOR, Conv::Thiscall(reader), &[path]);
        let open = h.call_sym(READER_IS_OPEN, Conv::Thiscall(reader), &[]) as u8;
        let result = if open == 0 {
            5
        } else {
            h.call_sym(READER_CLOSE, Conv::Thiscall(reader), &[]);
            let [lo, hi] = cdq(this);
            let callback = h.rebase(ON_CUTBUFFER_LOADED);
            let manager = instance(h, ASYNC_LOAD_INSTANCE);
            h.call_sym(
                LOAD_CLUSTER,
                Conv::Cdecl,
                &[manager, 0, path, callback, 1, lo, hi],
            );
            0
        };
        h.call_sym(READER_DTOR, Conv::Thiscall(reader), &[]);
        result
    }

    /// `PhyreScene_setMeshInstanceGroupIdVisible(this, mask)` `0x0066d610`
    /// (`ret 4`; frame: `this` at `ebp - 4`, a tree iterator (end,
    /// current, next) at `ebp - 0x10`): unless the group tree (`+0x1198`,
    /// root `+0x11a4`) is empty or the mask (`+0xfb14`) is unchanged, each
    /// mesh instance of each group whose bit (`1 << +0x48`) changes gets
    /// its draw link (`[+0x1c] + 0x1c`) set to itself when shown, 0 when
    /// hidden; then the new mask is kept.
    pub fn scene_set_group_visible(h: &mut impl Host, ebp: u32, this: u32, mask: u32) {
        h.write(ebp.wrapping_sub(4), this);
        let tree = this + 0x1198;
        let root: u32 = h.read(this + 0x11a4);
        if root == tree || root == 0 {
            return;
        }
        if h.read::<u32>(this + 0xfb14) == mask {
            return;
        }
        let it = ebp.wrapping_sub(0x10);
        h.call_sym(TREE_ITERATOR_BEGIN, Conv::Thiscall(it), &[tree]);
        let mut node: u32 = h.read(it + 4);
        if node != h.read::<u32>(it) {
            let mut next: u32 = h.read(it + 8);
            loop {
                let group: u32 = h.read(node.wrapping_add(0x10));
                let mut k = 0u32;
                while k < h.read::<u32>(group) {
                    let meshes: u32 = h.read(group.wrapping_add(8));
                    let mesh: u32 = h.read(meshes.wrapping_add(4 * k));
                    let bit = 1u32.wrapping_shl(h.read(mesh.wrapping_add(0x48)));
                    let changed = h.read::<u32>(this + 0xfb14) ^ mask;
                    if bit & changed != 0 && mesh != 0 {
                        let link: u32 = h.read(mesh + 0x1c);
                        if link != 0 {
                            let to = if mask & bit != 0 { mesh } else { 0 };
                            h.write(link + 0x1c, to);
                        }
                    }
                    k += 1;
                }
                let end: u32 = h.read(it);
                node = next;
                if next != end {
                    next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(end), &[next]);
                }
                if node == end {
                    break;
                }
            }
        }
        h.write(this + 0xfb14, mask);
    }

    /// `graphicDynGeoMemManagerSwapBuffer(keep)` `0x0063ff70`: while
    /// nothing holds the frame (a screen state 0, not paused, no busy
    /// load: the field map's pending flags `+0x1195..+0x1197`, characters
    /// loading for a listed map, or unfinished requests with the
    /// post-process flag `+0x123a0`; and the free camera idle), the
    /// dynamic geometry buffers swap and the VFX draw data reset (its
    /// `eax`); otherwise only the swap, unless `keep` (`eax`: the busy
    /// flag or the camera, whichever the original tested last).
    pub fn dyn_geo_swap_buffer(h: &mut impl Host, keep: u32) -> u32 {
        let screen_free = SCREEN_STATE.get(h) == 0;
        let scene = SCENE.get(h);
        let busy = load_busy(h, scene);
        let mut last = u32::from(busy);
        if PAUSE_ON.get(h) == 0 && screen_free && !busy {
            let camera = h.call_sym(FREE_CAMERA_INSTANCE, Conv::Cdecl, &[]);
            last = camera;
            if h.read::<u8>(camera + 0x338) == 0
                && h.read::<u8>(camera + 0x33c) == 0
                && h.read::<u32>(camera) != 2
            {
                let fill = DYN_GEO_FILL.get(h);
                h.call_sym(DYN_GEO_SWAP, Conv::Thiscall(fill), &[]);
                let table = VFX_RENDER_DATA_TABLE.get(h);
                return h.call_sym(VFX_CLEAR_DRAW_DATA, Conv::Thiscall(table), &[]);
            }
        }
        if keep != 0 {
            return last;
        }
        let fill = DYN_GEO_FILL.get(h);
        h.call_sym(DYN_GEO_SWAP, Conv::Thiscall(fill), &[])
    }

    /// The loads that hold the swap back.
    fn load_busy(h: &mut impl Host, scene: u32) -> bool {
        if h.read::<u8>(scene + 0x1195) != 0 && pending(h, scene) {
            return true;
        }
        if CHARACTER_LOADING.get(h) & 0x7fff_ffff != 0
            && h.call_sym(IS_MAP_IN_LIST, Conv::Cdecl, &[]) != 0
            && pending(h, scene)
        {
            return true;
        }
        let manager = instance(h, ASYNC_LOAD_INSTANCE);
        if h.call_sym(ALL_REQUESTS_DONE, Conv::Thiscall(manager), &[]) as u8 != 0 {
            return false;
        }
        let post = instance(h, POST_PROCESS_INSTANCE);
        h.read::<u8>(post.wrapping_add(0x1_23a0)) != 0
    }

    /// The field map's two pending flags (`+0x1196`, `+0x1197`) clear.
    fn pending(h: &mut impl Host, scene: u32) -> bool {
        h.read::<u8>(scene + 0x1196) == 0 && h.read::<u8>(scene + 0x1197) == 0
    }

    /// `PhyreScene_createRWTTTarget(this)` `0x00659c30`: a 512x512 texture
    /// in the scene's cluster (`+0x11e4`; flag 2 of `+0x14` cleared) and a
    /// render target over it (`+0xfc70`, named); 0, or 0xd when either
    /// can't be made.
    pub fn scene_create_rwtt_target(h: &mut impl Host, this: u32) -> u32 {
        let cluster = this + 0x11e4;
        let class = h.rebase(TEXTURE_CLASS);
        let texture = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, class, 0]);
        if texture == 0 {
            return 0xd;
        }
        let format = h.rebase(TEXTURE_FORMAT);
        h.call_sym(
            TEXTURE_SET_DIMENSIONS,
            Conv::Thiscall(texture),
            &[0x200, 0x200, format, 0],
        );
        let flags: u32 = h.read(texture + 0x14);
        h.write(texture + 0x14, flags & !2);
        h.call_sym(TEXTURE_UPDATE_MAX_LEVEL, Conv::Thiscall(texture), &[]);
        let class = h.rebase(TARGET_CLASS);
        let list = h.call_sym(
            CLUSTER_ALLOCATE_INSTANCE_LIST,
            Conv::Thiscall(cluster),
            &[1, class, 0],
        );
        let target = if list == 0 { 0 } else { h.read(list + 0x34) };
        if target != 0 {
            h.call_sym(TARGET_CTOR, Conv::Thiscall(target), &[texture]);
        }
        h.write(this + 0xfc70, target);
        if target == 0 {
            return 0xd;
        }
        let name = h.rebase(TARGET_NAME);
        h.write(target + 0xc, name);
        0
    }

    /// `PhyreScene_deleteMaterialInCluster(this, material, cluster, a, b)`
    /// `0x0065a2c0` (`ret 0x10`): a material of the scene's pool (cluster
    /// `+0x12d4`, slot `(material - [+0xfbec]) / 24` up to 4000) is handed
    /// back (use byte `[+0xfbf8]` cleared, owned storage `+0x10` freed,
    /// lowest free slot `+0xfbfc` lowered); any other is deleted with its
    /// part `+4` from its instance list: the scene's list for its kind
    /// (`+0x14`, -1 from 64 up; `+0x1374 + 80 kind`, count `+0x2774 + 4
    /// kind` lowered) or `cluster`'s.
    pub fn scene_delete_material(
        h: &mut impl Host,
        this: u32,
        material: u32,
        cluster: u32,
        a: u32,
        b: u32,
    ) {
        let mut list = cluster;
        let mut kind = None;
        if cluster == this.wrapping_add(0x12d4) {
            let base: u32 = h.read(this + 0xfbec);
            let slot = (material.wrapping_sub(base) as i32) / 24;
            if slot as u32 <= 0xf9f {
                let used: u32 = h.read(this + 0xfbf8);
                h.write(used.wrapping_add(slot as u32), 0u8);
                if h.read::<u8>(material + 0x10) & 1 == 0 {
                    let storage: u32 = h.read(material + 0x10);
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[storage]);
                }
                if slot < h.read::<i32>(this + 0xfbfc) {
                    h.write(this + 0xfbfc, slot);
                }
                return;
            }
            let k: u32 = h.read(material + 0x14);
            let k = if k >= 0x40 { u32::MAX } else { k };
            list = this.wrapping_add(0x1374).wrapping_add(k.wrapping_mul(80));
            kind = Some(k);
        }
        let part: u32 = h.read(material.wrapping_add(4));
        h.call_sym(CLUSTER_DELETE_WITH_LIST, Conv::Thiscall(list), &[part, b]);
        h.call_sym(
            CLUSTER_DELETE_WITH_LIST,
            Conv::Thiscall(list),
            &[material, a],
        );
        if let Some(k) = kind {
            let at = this.wrapping_add(k.wrapping_mul(4)).wrapping_add(0x2774);
            let n = h.read::<u32>(at).wrapping_sub(1);
            h.write(at, n);
        }
    }

    /// `CopyNamedPairs_006991f0(dst, src, n)` (cdecl; `slots`: its
    /// argument words, which it uses as counters): `n` pairs `(word,
    /// name)` copied to `dst` (8 bytes each), each name (colour bit
    /// cleared) duplicated with `PhyreMalloc(len, 2)` (0 for none or a
    /// failed allocation); a null destination slot is skipped without
    /// moving the source on. Leaves `dst + 8 n` and 0 in the `dst` and
    /// `n` slots.
    pub fn copy_named_pairs(h: &mut impl Host, slots: u32, dst: u32, src: u32, n: u32) {
        let mut dst = dst;
        let mut from = src.wrapping_add(4);
        let mut left = n;
        while left != 0 {
            let at = dst;
            dst = dst.wrapping_add(8);
            h.write(slots, dst);
            if at != 0 {
                let word: u32 = h.read(from.wrapping_sub(4));
                h.write(at, word);
                let tagged: u32 = h.read(from);
                let name = tagged.wrapping_sub(u32::from(h.read::<u8>(from)) & 1);
                let copy = if name == 0 {
                    0
                } else {
                    let mut len = 0u32;
                    while h.read::<u8>(name.wrapping_add(len)) != 0 {
                        len += 1;
                    }
                    let size = len + 1;
                    let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[size, 2]);
                    if p != 0 {
                        h.call_sym(MEMCPY, Conv::Cdecl, &[p, name, size]);
                    }
                    p
                };
                h.write(at + 4, copy);
                from = from.wrapping_add(8);
            }
            left -= 1;
            h.write(slots + 8, left);
        }
    }
}
