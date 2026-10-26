//! The render-side glue, fourteenth batch (M4 port round 5, list
//! `h3-glue-render`): the scene's pool claims and cursor walks, mesh and
//! character-instance tear-down, texture-animation bookkeeping, the
//! post-process target resize, the shadow-map aspect fit (x87) and the
//! character load request. Exception frames and cookies left out, as in
//! [`crate::glue10`]; frames passed to callees as in [`crate::glue5`].

use ffx_bridge::Global;

pub use crate::glue::SCENE;

/// `Phyre_PhyrePrintf(level, format, ...)` (cdecl).
pub const PHYRE_PRINTF: u32 = crate::glue10::PHYRE_PRINTF;
/// `Phyre_PhyreFree(p)`, `operator delete(p)`, `operator delete[](p)`,
/// `operator new(size)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
pub const ARRAY_DELETE: u32 = 0x0063_0540;
pub const OPERATOR_NEW: u32 = 0x0063_0310;
/// `user_free_thunk(p)` (cdecl).
pub const USER_FREE: u32 = 0x0068_7010;

/// `PhyreSceneCursor_Seek1(this)` / `PhyreSceneCursor_Next1(this)` and
/// `PhyreSceneCursor_Begin_00637730(this)` /
/// `PhyreSceneCursor_Next_006377a0(this)`: the scene list cursors.
pub const CURSOR_SEEK1: u32 = 0x0065_eaf0;
pub const CURSOR_NEXT1: u32 = 0x0065_ed20;
pub const CURSOR_BEGIN: u32 = 0x0063_7730;
pub const CURSOR_NEXT: u32 = 0x0063_77a0;
/// The class descriptors the two walks look for.
pub const SEEK1_CLASS: u32 = 0x00c9_3608;
pub const BEGIN_CLASS: u32 = 0x00ca_7228;
/// The name the field map's global object carries.
pub const GLOBAL_NAME: u32 = 0x00b4_232c;

/// `Phyre_PCluster_create(this, n, class, 0)` and
/// `Phyre_PCluster_allocateInstanceList(this, n, class, 0)` (`ret 0xc`).
pub const CLUSTER_CREATE: u32 = crate::glue9::CLUSTER_CREATE;
pub const CLUSTER_ALLOCATE_INSTANCE_LIST: u32 = 0x0044_0b70;
/// The classes the 28- and 16-byte pools fall back to.
pub const POOL_CLASS_28: u32 = 0x00cc_ba18;
pub const POOL_CLASS_16: u32 = 0x00cc_bb10;
/// `ClassDynamicMesh_ClassDynamicMesh_00672630(this)` and
/// `ClassDynamicMeshInstance_ctor(this, mesh)`.
pub const DYNAMIC_MESH_CTOR: u32 = 0x0067_2630;
pub const DYNAMIC_MESH_INSTANCE_CTOR: u32 = 0x0067_35f0;

/// The post-process resize's callees: `0x00a233f0(this; w, h)`,
/// `0x00a23200(this; w, h)` and `0x00a0e4b0(target, w, h, 1)` (cdecl).
pub const RESIZE_A233F0: u32 = 0x00a2_33f0;
pub const RESIZE_A23200: u32 = 0x00a2_3200;
pub const RESIZE_TARGET: u32 = 0x00a0_e4b0;

/// `SearchTree_findSigned_0064b9a0(this, &key)`,
/// `Phyre_PCluster_deleteObject(this, object)`,
/// `Phyre_PRedBlackTree_remove(this, node)`,
/// `Phyre_PFreeList_deallocate(this, node)`.
pub const TREE_FIND_SIGNED: u32 = 0x0064_b9a0;
pub const CLUSTER_DELETE_OBJECT: u32 = 0x0044_1240;
pub const TREE_REMOVE: u32 = 0x0057_5000;
pub const FREE_LIST_DEALLOCATE: u32 = 0x0049_c980;

/// `"unregister(%u)\n"`.
pub const UNREGISTER_MESSAGE: u32 = 0x00b4_8618;

/// The character-instance tear-down's callees.
pub const CHARACTER_LIGHTING_INSTANCE: u32 = 0x006a_6e30;
pub const DESTROY_UNIQUE_MATERIAL_PARAMETERS: u32 = 0x006a_8210;
pub const TEXTURE_ANIMATION_INSTANCE: u32 = 0x006b_02e0;
pub const UNREGISTER_CHARACTER_INSTANCE: u32 = 0x006b_4330;
pub const SHADOW_INSTANCE: u32 = 0x006d_2390;
pub const SHADOW_REMOVE: u32 = 0x006c_e9d0;
pub const RENDER_DELEGATION_HELPER: Global<u32> = Global::new(0x00cc_ca98);
pub const UNREGISTER_DELEGATION: u32 = 0x006a_e340;
/// The delegation task the instance was registered with.
pub const CHARACTER_DELEGATION_TASK: u32 = 0x0063_7c10;
pub const SCENE_REMOVE_CHR_FROM_PIPELINE: u32 = 0x0066_8640;
pub const DESTROY_CLOTH_INSTANCES: u32 = 0x006a_1590;
pub const CHARACTER_REMOVE_INSTANCE: u32 = 0x006a_2b90;
pub const DESTROY_MESHES: u32 = 0x006a_15e0;
pub const SCENE_REMOVE_MESH_INSTANCE: u32 = 0x0066_8680;
pub const CHARACTER_UNREGIST_LOADING: u32 = 0x0063_7b30;

/// The asset callback's `Array_Reserve_00668080(this, n)`,
/// `AssetListReader_isModel(this, id)` and the non-model push
/// `0x00667620(this, &asset)`.
pub const ARRAY_RESERVE: u32 = 0x0066_8080;
pub const IS_MODEL: u32 = 0x006f_46b0;
pub const PUSH_ASSET: u32 = 0x0066_7620;

/// The character load: `IsCharacterCancleLoad(id)` (cdecl),
/// `ClassLoadedCharacterTable_GetInstance`,
/// `ClassLoadedCharacterTable_getPrototypeByFilename(this, name)`,
/// `ClassCharacter_ClassCharacter(this, shadow)`,
/// `PhyreScene_loadCharacterAsync(this, name, character)`, `sprintf`'s
/// import slot and the error's format.
pub const IS_CANCEL_LOAD: u32 = 0x0063_5a70;
pub const LOADED_CHARACTER_TABLE: u32 = 0x0069_f6c0;
pub const PROTOTYPE_BY_FILENAME: u32 = 0x006a_2110;
pub const CHARACTER_CTOR: u32 = 0x0069_e030;
pub const SCENE_LOAD_CHARACTER_ASYNC: u32 = 0x0066_42f0;
pub const SPRINTF_SLOT: u32 = crate::glue9::SPRINTF;
pub const CHARACTER_LOAD_ERROR: u32 = 0x00b4_1778;

/// The dynamic geometry's pool objects: `PhyreScene_newPoolArray`,
/// `PhyreScene_newPoolObject56`, the 28-byte and 16-byte claims and
/// `0x006599c0(this, a, cluster)`.
pub const NEW_POOL_ARRAY: u32 = 0x0065_9860;
pub const NEW_POOL_OBJECT56: u32 = 0x0065_9910;
pub const NEW_POOL_OBJECT28: u32 = 0x0065_94c0;
pub const NEW_POOL_OBJECT16: u32 = 0x0065_95d0;
pub const NEW_POOL_6599C0: u32 = 0x0065_99c0;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::glue9::bridge::claim_slot;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// A scene cursor's walk after its first step (frame `cursor`: base
    /// `+0`, block `+4`, stride `+8`, count `+0xc`): `body` on each base,
    /// the cursor advanced as the originals do it in registers (an
    /// element equal to the block pointer moves to the next block and is
    /// skipped), `next` called when a run ends, until the count is 0.
    fn walk<H: Host>(h: &mut H, cursor: u32, next: u32, mut body: impl FnMut(&mut H, u32)) {
        if h.read::<u32>(cursor + 0xc) == 0 {
            return;
        }
        loop {
            let mut base: u32 = h.read(cursor);
            let mut block: u32 = h.read(cursor + 4);
            let stride: u32 = h.read(cursor + 8);
            let mut count: u32 = h.read(cursor + 0xc);
            'run: loop {
                body(h, base);
                if count == 0 {
                    return;
                }
                loop {
                    base = base.wrapping_add(stride);
                    count = count.wrapping_sub(1);
                    h.write(cursor, base);
                    h.write(cursor + 0xc, count);
                    if block == base {
                        block = h.read(block);
                        h.write(cursor + 4, block);
                        if count == 0 {
                            break 'run;
                        }
                    } else if count == 0 {
                        break 'run;
                    } else {
                        continue 'run;
                    }
                }
            }
            h.call_sym(next, Conv::Thiscall(cursor), &[]);
            if h.read::<u32>(cursor + 0xc) == 0 {
                return;
            }
        }
    }

    /// Sets up a cursor over `owner`'s list (`+0x1c`) for `class`.
    fn cursor_over(h: &mut impl Host, cursor: u32, owner: u32, class: u32) {
        let list = owner.wrapping_add(0x1c);
        let first: u32 = h.read(list);
        let first = if first == list { 0 } else { first };
        for k in 0..5 {
            h.write(cursor + 4 * k, 0u32);
        }
        h.write(cursor + 0x14, list);
        h.write(cursor + 0x18, first);
        let class = h.rebase(class);
        h.write(cursor + 0x1c, class);
    }

    /// Whether the C strings at `a` and `b` are equal.
    fn str_eq(h: &impl Host, a: u32, b: u32) -> bool {
        let mut k = 0u32;
        loop {
            let x: u8 = h.read(a.wrapping_add(k));
            if x != h.read::<u8>(b.wrapping_add(k)) {
                return false;
            }
            if x == 0 {
                return true;
            }
            k = k.wrapping_add(1);
        }
    }

    /// `FUN_006518c0(scene)` (frame: the cursor at `ebp - 0x28`, the
    /// scene at `ebp - 8`, the object at `ebp - 4`): with a field map
    /// (`+0x1128`) and its flag `+0x93c`, the map's objects of class
    /// `0x00c93608` are walked; the one named `GLOBAL_NAME` (its name
    /// pointer `+0xd0`, low bit a tag; the same pointer, or equal text)
    /// becomes the scene's `+0x4f8`. Returns 0.
    pub fn scene_find_global_object(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        let map: u32 = h.read(scene + 0x1128);
        h.write(ebp - 8, scene);
        if map == 0 || h.read::<u8>(scene + 0x93c) == 0 {
            return 0;
        }
        let cursor = ebp - 0x28;
        cursor_over(h, cursor, map, SEEK1_CLASS);
        h.call_sym(CURSOR_SEEK1, Conv::Thiscall(cursor), &[]);
        let name = h.rebase(GLOBAL_NAME);
        walk(h, cursor, CURSOR_NEXT1, |h, base| {
            let offset: u32 = h.read(cursor + 0x10);
            let at = offset.wrapping_add(base);
            if at == 0 {
                return;
            }
            let object = at.wrapping_sub(4);
            h.write(ebp - 4, object);
            if object == 0 {
                return;
            }
            let tag = u32::from(h.read::<u8>(object.wrapping_add(0xd0)) & 1);
            let s = h.read::<u32>(object.wrapping_add(0xd0)).wrapping_sub(tag);
            if s == name || (s != 0 && str_eq(h, s, name)) {
                let scene: u32 = h.read(ebp - 8);
                let object: u32 = h.read(ebp - 4);
                h.write(scene + 0x4f8, object);
            }
        });
        0
    }

    /// `FUN_00697f20(this, w, h)` (`ret 8`): with any of the four flags
    /// `+0x123b0..+0x123b3` and the target `+0x8c` (whose texture, `*p`
    /// unless its byte `+4` is set, is not already `w` x `h`, `+0x1c` /
    /// `+0x20`): the two helpers at `+0x1100` (at twice the size) and
    /// `+0x1190` resized, then the ten targets. Returns 0.
    pub fn post_resize_targets(h: &mut impl Host, this: u32, w: u32, ht: u32) -> u32 {
        let flags: u32 = h.read(this + 0x123b0);
        if flags == 0 {
            return 0;
        }
        let target: u32 = h.read(this + 0x8c);
        if target == 0 {
            return 0;
        }
        let texture: u32 = if h.read::<u8>(target + 4) == 0 {
            h.read(target)
        } else {
            0
        };
        if w == h.read::<u32>(texture.wrapping_add(0x1c)) {
            let texture: u32 = if h.read::<u8>(target + 4) == 0 {
                h.read(target)
            } else {
                0
            };
            if ht == h.read::<u32>(texture.wrapping_add(0x20)) {
                return 0;
            }
        }
        h.call_sym(
            RESIZE_A233F0,
            Conv::Thiscall(this + 0x1100),
            &[w.wrapping_mul(2), ht.wrapping_mul(2)],
        );
        h.call_sym(RESIZE_A23200, Conv::Thiscall(this + 0x1190), &[w, ht]);
        for off in [
            0x8c, 0x90, 0x94, 0x9c, 0xa4, 0xa8, 0x1232c, 0x12330, 0x98, 0x122c0,
        ] {
            let t: u32 = h.read(this + off);
            h.call_sym(RESIZE_TARGET, Conv::Cdecl, &[t, w, ht, 1]);
        }
        0
    }

    /// `FUN_006594c0(this, cluster)` (`ret 4`): from the scene's own pool
    /// (cluster `+0x12d4`; slots `+0xfba4`, 28 bytes; use bytes `+0xfbb0`;
    /// counter `+0xfbb4`) a dynamic mesh constructed in its slot (0 for a
    /// null slot), else `PCluster_create(cluster, 1, class, 0)` less 4
    /// (0 when it fails).
    pub fn scene_new_dynamic_mesh(h: &mut impl Host, this: u32, cluster: u32) -> u32 {
        if cluster == this.wrapping_add(0x12d4)
            && let Some(p) = claim_slot(h, this, 0xfbb4, 0xfba4, 0xfbb0, 0x1c)
        {
            if p == 0 {
                return 0;
            }
            return h.call_sym(DYNAMIC_MESH_CTOR, Conv::Thiscall(p), &[]);
        }
        let class = h.rebase(POOL_CLASS_28);
        let p = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, class, 0]);
        if p == 0 { 0 } else { p.wrapping_sub(4) }
    }

    /// `FUN_006595d0(this, mesh, cluster)` (`ret 8`): nothing (0) without
    /// a mesh; from the scene's own pool (cluster `+0x12d4`; slots
    /// `+0xfbb8`, 16 bytes; use bytes `+0xfbc4`; counter `+0xfbc8`) a mesh
    /// instance constructed in its slot (0 for a null slot; the slot is
    /// claimed after the constructor), else one in the instance list
    /// `PCluster_allocateInstanceList(cluster, 1, class, 0)` gives
    /// (`+0x34`; 0 when none).
    pub fn scene_new_mesh_instance(h: &mut impl Host, this: u32, mesh: u32, cluster: u32) -> u32 {
        if mesh == 0 {
            return 0;
        }
        let slot: i32 = h.read(this + 0xfbc8);
        if cluster == this.wrapping_add(0x12d4) && slot < 3000 {
            let start: u32 = h.read(this + 0xfbb8);
            let p = (slot as u32).wrapping_shl(4).wrapping_add(start);
            let made = if p == 0 {
                0
            } else {
                h.call_sym(DYNAMIC_MESH_INSTANCE_CTOR, Conv::Thiscall(p), &[mesh])
            };
            let bytes: u32 = h.read(this + 0xfbc4);
            let slot: u32 = h.read(this + 0xfbc8);
            h.write(bytes.wrapping_add(slot), 1u8);
            let mut k = h.read::<i32>(this + 0xfbc8).wrapping_add(1);
            let bytes: u32 = h.read(this + 0xfbc4);
            while k < 3000 {
                if h.read::<u8>(bytes.wrapping_add(k as u32)) == 0 {
                    break;
                }
                k += 1;
            }
            h.write(this + 0xfbc8, k.min(3000));
            return made;
        }
        let class = h.rebase(POOL_CLASS_16);
        let list = h.call_sym(
            CLUSTER_ALLOCATE_INSTANCE_LIST,
            Conv::Thiscall(cluster),
            &[1, class, 0],
        );
        if list == 0 {
            return 0;
        }
        let p: u32 = h.read(list + 0x34);
        if p == 0 {
            return 0;
        }
        h.call_sym(DYNAMIC_MESH_INSTANCE_CTOR, Conv::Thiscall(p), &[mesh])
    }

    /// Frees an owned array's storage when it is empty: the word `count`
    /// keeps only its "not owned" bit; owned and now 0, its data `data`
    /// is freed and cleared.
    fn release_owned(h: &mut impl Host, count: u32, data: u32) {
        let word = h.read::<u32>(count) & 0x8000_0000;
        h.write(count, word);
        if word == 0 {
            let p: u32 = h.read(data);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
            h.write(data, 0u32);
        }
    }

    /// `TextureAnimationManager_sceenTextureAnimationClear(this)`: the
    /// word arrays `+0x5c/+0x60` and `+0x74/+0x78` zeroed and emptied
    /// (`+0x78` first), the 32-byte objects `+0x7c/+0x80` destroyed (their
    /// virtual 0 with 0) and emptied, each owned storage freed; `+4` = 0.
    pub fn texture_animation_clear(h: &mut impl Host, this: u32) {
        for (count, data) in [(0x5c, 0x60), (0x74, 0x78)] {
            let mut k = 0u32;
            while k < h.read::<u32>(this + count) & 0x7fff_ffff {
                let p: u32 = h.read(this + data);
                h.write(p.wrapping_add(4 * k), 0u32);
                k += 1;
            }
        }
        release_owned(h, this + 0x74, this + 0x78);
        release_owned(h, this + 0x5c, this + 0x60);
        let word: u32 = h.read(this + 0x7c);
        h.write(this + 0x7c, word & 0x8000_0000);
        if word & 0x8000_0000 == 0 {
            let n = word & 0x7fff_ffff;
            for k in 0..n {
                let objects: u32 = h.read(this + 0x80);
                let object = objects.wrapping_add(0x20 * k);
                let vtable: u32 = h.read(object);
                let f: u32 = h.read(vtable);
                h.call(f, Conv::Thiscall(object), &[0]);
            }
            if h.read::<u32>(this + 0x7c) == 0 {
                let p: u32 = h.read(this + 0x80);
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
                h.write(this + 0x80, 0u32);
            }
        }
        h.write(this + 4, 0u32);
    }

    /// `PhyreScene_removeMeshInstance(this, id)` (`ret 4`; `key` is the
    /// address of `id`'s argument slot, which the tree search reads): the
    /// instance's meshes (tree `+0xf9d4`, its entry `+0x10`: count,
    /// records `+8`) each have their three objects (`+0`, `+8`, `+4`)
    /// deleted from both clusters (`+0x11e4`, `+0x1234`), are deleted and
    /// cleared; then the tree's node for `id` (signed keys `+0xc`) is
    /// removed, its buffer (`+0x18`) freed and the node returned to the
    /// free list (`+0x18`). `eax` as the original leaves it: the found
    /// entry, the last records pointer, the last search step's direction,
    /// or the deallocation's.
    pub fn scene_remove_mesh_instance(h: &mut impl Host, this: u32, key: u32) -> u32 {
        let tree = this + 0xf9d4;
        let found = h.call_sym(TREE_FIND_SIGNED, Conv::Thiscall(tree), &[key]);
        if found == 0 {
            return 0;
        }
        let entry = found.wrapping_add(0x10);
        if entry == 0 {
            return found;
        }
        let a = this + 0x11e4;
        let b = this + 0x1234;
        let mut eax = found;
        let mut k = 0u32;
        while k < h.read::<u32>(entry) {
            let records: u32 = h.read(entry + 8);
            let record: u32 = h.read(records.wrapping_add(4 * k));
            for (cluster, off) in [(a, 0), (a, 8), (a, 4), (b, 0), (b, 8), (b, 4)] {
                let object: u32 = h.read(record.wrapping_add(off));
                h.call_sym(CLUSTER_DELETE_OBJECT, Conv::Thiscall(cluster), &[object]);
            }
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[record]);
            let records: u32 = h.read(entry + 8);
            h.write(records.wrapping_add(4 * k), 0u32);
            eax = records;
            k += 1;
        }
        let mut node: u32 = h.read(tree + 0xc);
        if node == tree {
            return eax;
        }
        let id: i32 = h.read(key);
        loop {
            let k: i32 = h.read(node + 0xc);
            if k == id {
                h.call_sym(TREE_REMOVE, Conv::Thiscall(tree), &[node]);
                let buffer: u32 = h.read(node + 0x18);
                if buffer != 0 {
                    h.call_sym(USER_FREE, Conv::Cdecl, &[buffer]);
                }
                return h.call_sym(FREE_LIST_DEALLOCATE, Conv::Thiscall(tree + 0x18), &[node]);
            }
            let right = u32::from(k < id);
            node = h.read(node + 4 * right);
            if node == tree {
                return right;
            }
        }
    }

    /// `TextureAnimationManager_unregisterChararcterInstance(this, chr)`
    /// (`ret 4`): after the message, the registration of `chr` (12-byte
    /// records `+0x70`, count `+0x6c`; the instance at `+4`): its texture
    /// list (`+8`: count, 8-byte pairs at `+4`) has each pair's second
    /// word's two buffers `delete[]`d and the object deleted; the list's
    /// owned storage and the list itself freed; the record becomes
    /// `{-1, 0, 0}`. `eax`: the count when not found, else the records.
    pub fn texture_animation_unregister<H: Host>(h: &mut H, this: u32, chr: u32) -> u32 {
        let message = h.rebase(UNREGISTER_MESSAGE);
        h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[2, message, chr]);
        let n = h.read::<u32>(this + 0x6c) & 0x7fff_ffff;
        let records: u32 = h.read(this + 0x70);
        let Some(i) = (0..n).find(|&i| h.read::<u32>(records.wrapping_add(12 * i + 4)) == chr)
        else {
            return n;
        };
        let list_of = |h: &H| -> u32 {
            let records: u32 = h.read(this + 0x70);
            h.read(records.wrapping_add(12 * i + 8))
        };
        let mut list: u32 = h.read(records.wrapping_add(12 * i + 8));
        if list != 0 {
            let mut k = 0u32;
            while k < h.read::<u32>(list) & 0x7fff_ffff {
                let pairs: u32 = h.read(list + 4);
                let object: u32 = h.read(pairs.wrapping_add(8 * k + 4));
                if object != 0 {
                    for off in [0, 4] {
                        let p: u32 = h.read(object + off);
                        if p != 0 {
                            h.call_sym(ARRAY_DELETE, Conv::Cdecl, &[p]);
                        }
                    }
                    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[object]);
                }
                k += 1;
                list = list_of(h);
            }
        }
        let list = list_of(h);
        if list != 0 {
            let word: u32 = h.read(list);
            let data: u32 = h.read(list + 4);
            if word & 0x8000_0000 == 0 && data != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
            h.write(list + 4, 0u32);
            h.write(list, 0u32);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[list]);
        }
        for (off, v) in [(8, 0u32), (4, 0), (0, 0xffff_ffff)] {
            let records: u32 = h.read(this + 0x70);
            h.write(records.wrapping_add(12 * i + off), v);
        }
        h.read(this + 0x70)
    }

    /// `graphicCharacterInstanceDestroy(chr, id)` (cdecl; `slot` is the
    /// address of `chr`'s argument slot, which the delegation search
    /// reads): an instance has its unique material parameters destroyed
    /// (its lighting list `+0/+4`), is unregistered from the texture
    /// animations, the shadow map, the delegations (task `0x00637c10`,
    /// 4 bytes at `slot`) and the scene's pipeline, has its cloth and
    /// meshes destroyed, leaves its character (`+0x10`), has its two
    /// owned arrays (`+8/+0xc`, `+0/+4`) freed and is deleted; then the
    /// scene drops its mesh instance (`+0x18`). Without one, the load of
    /// `id` is unregistered and the scene drops `id`. `eax`: the scene's.
    pub fn character_instance_destroy(h: &mut impl Host, slot: u32, chr: u32, id: u32) -> u32 {
        if chr == 0 {
            h.call_sym(CHARACTER_UNREGIST_LOADING, Conv::Cdecl, &[id]);
            let scene = SCENE.get(h);
            return h.call_sym(SCENE_REMOVE_MESH_INSTANCE, Conv::Thiscall(scene), &[id]);
        }
        let character: u32 = h.read(chr + 0x10);
        let mesh: u32 = h.read(chr + 0x18);
        let mut k = 0u32;
        while k < h.read::<u32>(chr) & 0x7fff_ffff {
            let materials: u32 = h.read(chr + 4);
            let material: u32 = h.read(materials.wrapping_add(4 * k));
            let lighting = h.call_sym(CHARACTER_LIGHTING_INSTANCE, Conv::Cdecl, &[]);
            h.call_sym(
                DESTROY_UNIQUE_MATERIAL_PARAMETERS,
                Conv::Thiscall(lighting),
                &[material],
            );
            k += 1;
        }
        let animations = h.call_sym(TEXTURE_ANIMATION_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(
            UNREGISTER_CHARACTER_INSTANCE,
            Conv::Thiscall(animations),
            &[chr],
        );
        let shadows = h.call_sym(SHADOW_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(SHADOW_REMOVE, Conv::Thiscall(shadows), &[chr]);
        let helper = RENDER_DELEGATION_HELPER.get(h);
        h.write(slot, chr);
        let task = h.rebase(CHARACTER_DELEGATION_TASK);
        h.call_sym(
            UNREGISTER_DELEGATION,
            Conv::Thiscall(helper),
            &[task, slot, 4],
        );
        let scene = SCENE.get(h);
        h.call_sym(
            SCENE_REMOVE_CHR_FROM_PIPELINE,
            Conv::Thiscall(scene),
            &[chr],
        );
        h.call_sym(DESTROY_CLOTH_INSTANCES, Conv::Thiscall(chr), &[]);
        h.call_sym(CHARACTER_REMOVE_INSTANCE, Conv::Thiscall(character), &[chr]);
        h.call_sym(DESTROY_MESHES, Conv::Thiscall(chr), &[]);
        h.write(chr + 0x10, 0u32);
        for (count, data) in [(8, 0xc), (0, 4)] {
            if h.read::<u32>(chr + count) & 0x8000_0000 == 0 {
                let p: u32 = h.read(chr + data);
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
            }
            h.write(chr + data, 0u32);
            h.write(chr + count, 0u32);
        }
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[chr]);
        let scene = SCENE.get(h);
        h.call_sym(SCENE_REMOVE_MESH_INSTANCE, Conv::Thiscall(scene), &[mesh])
    }

    /// `FUN_0065ae40(out, reader, array, asset, id)` (stdcall; frame: a
    /// cursor at `ebp - 0x20`; `slots` is the address of the argument
    /// slots, `asset`'s at `+0xc`): `asset` is appended to `array` (count
    /// `+0`, capacity `+4`, data `+8`; grown by an eighth and 2 when
    /// full); a model (`AssetListReader_isModel(reader, id)`) becomes
    /// `*out` and its objects of class `0x00ca7228` have `+0x1c` cleared;
    /// another asset is pushed to `out + 4` by address. Returns 0.
    #[allow(clippy::too_many_arguments)] // the original's five and two addresses
    pub fn scene_on_character_asset(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        out: u32,
        reader: u32,
        array: u32,
        asset: u32,
        id: u32,
    ) -> u32 {
        let n = h.read::<i32>(array).wrapping_add(1);
        h.write(array, n);
        if n > h.read::<i32>(array + 4) {
            let grow = n.wrapping_add(2).wrapping_add(n / 8);
            h.call_sym(ARRAY_RESERVE, Conv::Thiscall(array), &[grow as u32]);
        }
        let n: u32 = h.read(array);
        let data: u32 = h.read(array + 8);
        let at = data.wrapping_add(n.wrapping_mul(4)).wrapping_sub(4);
        if at != 0 {
            h.write(at, asset);
        }
        if asset == 0 {
            return 0;
        }
        if h.call_sym(IS_MODEL, Conv::Thiscall(reader), &[id])
            .to_le_bytes()[0]
            == 0
        {
            h.call_sym(
                PUSH_ASSET,
                Conv::Thiscall(out.wrapping_add(4)),
                &[slots + 0xc],
            );
            return 0;
        }
        h.write(out, asset);
        let cursor = ebp - 0x20;
        cursor_over(h, cursor, asset, BEGIN_CLASS);
        h.call_sym(CURSOR_BEGIN, Conv::Thiscall(cursor), &[]);
        walk(h, cursor, CURSOR_NEXT, |h, base| {
            let offset: u32 = h.read(cursor + 0x10);
            let object: u32 = h.read(offset.wrapping_add(base).wrapping_add(0x1c));
            h.write(object.wrapping_add(0x1c), 0u32);
        });
        0
    }

    /// `FUN_006d23d0(w, h, out_w, out_h, n)` (stdcall; `slots` is the
    /// address of the argument slots, which it reuses: `h`'s gets the
    /// ratio and `w`'s `n` as a float): fits the larger side of `w` x `h`
    /// to `n` keeping the ratio (`ftol` toward zero; a 0 ratio gives 0);
    /// when that side is smaller than `n` (or a NaN), both sides grow by
    /// one instead. The comparisons are the x87's: a NaN `w` or `h` takes
    /// the "`h` is the larger" path.
    #[allow(clippy::too_many_arguments)] // the original's five and the slots
    pub fn shadow_fit_size(
        h: &mut impl Host,
        slots: u32,
        w: u32,
        ht: u32,
        out_w: u32,
        out_h: u32,
        n: u32,
    ) {
        let a = x87::load(w);
        let b = x87::load(ht);
        let (big, small, out_big, out_small) = if b < a {
            (a, b, out_w, out_h)
        } else {
            (b, a, out_h, out_w)
        };
        let ratio = if small == 0.0 {
            0.0
        } else {
            x87::div(big, small)
        };
        let ratio = x87::store(ratio);
        h.write(slots + 4, ratio);
        #[allow(clippy::cast_precision_loss)] // fild, then fstp dword: rounded once
        let nf = (n as i32 as f32).to_bits();
        h.write(slots, nf);
        let nf = x87::load(nf);
        if nf <= big {
            h.write(out_big, n);
            let ratio = x87::load(ratio);
            let v = if ratio == 0.0 {
                0
            } else {
                x87::ftol(&*h, x87::div(nf, ratio))
            };
            h.write(out_small, v);
        } else {
            let big1 = x87::ftol(&*h, x87::add(big, 1.0));
            let small1 = x87::ftol(&*h, x87::add(small, 1.0));
            h.write(out_big, big1);
            h.write(out_small, small1);
        }
    }

    /// `graphicCharacterLoad(name, id)` (frame: the shadow flag at
    /// `ebp - 0x108`, a dword whose low byte alone is written, and the
    /// message at `ebp - 0x104`): nothing without a name or when `id`'s
    /// load was cancelled; a character not loaded yet (by file name) is
    /// made (`operator new(0x24)`, the shadow flag set for ids 0x10fb,
    /// 0x10fc, 0x10fd, 0x1100 and 801..899) and loaded asynchronously; a
    /// failure prints the message and destroys it (its virtual 0 with 1).
    /// `eax`: the prototype found, the new character, or 0.
    pub fn character_load(h: &mut impl Host, ebp: u32, name: u32, id: u32) -> u32 {
        if name == 0 {
            return 0;
        }
        if h.call_sym(IS_CANCEL_LOAD, Conv::Cdecl, &[id]) & 0xff != 0 {
            return 0;
        }
        let table = h.call_sym(LOADED_CHARACTER_TABLE, Conv::Cdecl, &[]);
        let prototype = h.call_sym(PROTOTYPE_BY_FILENAME, Conv::Thiscall(table), &[name]);
        if prototype != 0 {
            return prototype;
        }
        let signed = id as i32;
        let shadow =
            matches!(signed, 0x10fb | 0x10fc | 0x10fd | 0x1100) || (801..900).contains(&signed);
        let flag = ebp - 0x108;
        h.write(flag, u8::from(shadow));
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x24]);
        let character = if p == 0 {
            0
        } else {
            // the dword's upper bytes are frame words never written
            let flag: u32 = h.read(flag);
            h.call_sym(CHARACTER_CTOR, Conv::Thiscall(p), &[flag])
        };
        let scene = SCENE.get(h);
        if h.call_sym(
            SCENE_LOAD_CHARACTER_ASYNC,
            Conv::Thiscall(scene),
            &[name, character],
        ) == 0
        {
            return character;
        }
        let message = ebp - 0x104;
        let format = h.rebase(CHARACTER_LOAD_ERROR);
        let sprintf: u32 = h.read(h.rebase(SPRINTF_SLOT));
        h.call(sprintf, Conv::Cdecl, &[message, format, name]);
        h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[1, message]);
        if character != 0 {
            virtual_call(h, character, 0, &[1]);
        }
        0
    }

    /// `FUN_006a46e0(this, a, cluster)` (`ret 8`; frame: four 30-word
    /// arrays at `ebp - 0x1e4`, `- 0x16c`, `- 0xf4`, `- 0x7c`): `+0x20` =
    /// `a`, `+0x2c` = `cluster`; the shader's slots (`+0x24`: count
    /// `+0x90`, 0x6c-byte records `+0x94`) give the words `+0x20`, `+8`,
    /// `+4`, `+0` of each to the arrays; then the scene's pool objects
    /// are made for the geometry (`+0x14`, `+4`, `+8`, `+0x10`, `+0xc`)
    /// between its virtuals `0x18` (a 0 result sets `+0x30`), `0x14`,
    /// `0x20`, `0x24` and `0x1c`. Returns 0. (More than 30 slots overrun
    /// the original's arrays into its cookie, a fail-fast; the port stops
    /// at 30.)
    pub fn geometry_set_shader(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        a: u32,
        cluster: u32,
    ) -> u32 {
        h.write(this + 0x20, a);
        let shader: u32 = h.read(this + 0x24);
        h.write(this + 0x2c, cluster);
        let n: i32 = h.read(shader + 0x90);
        let records: u32 = h.read(shader + 0x94);
        let (w1e4, w16c, wf4, w7c) = (ebp - 0x1e4, ebp - 0x16c, ebp - 0xf4, ebp - 0x7c);
        for k in 0..n.clamp(0, 30) as u32 {
            let r = records.wrapping_add(0x6c * k);
            for (to, off) in [(wf4, 4), (w16c, 8), (w1e4, 0x20), (w7c, 0)] {
                let v: u32 = h.read(r.wrapping_add(off));
                h.write(to + 4 * k, v);
            }
        }
        let n = n as u32;
        let scene = SCENE.get(h);
        let v = h.call_sym(NEW_POOL_ARRAY, Conv::Thiscall(scene), &[cluster]);
        h.write(this + 0x14, v);
        if virtual_call(h, this, 0x18, &[n, w1e4, w7c]) == 0 {
            h.write(this + 0x30, 1u8);
        }
        let cluster: u32 = h.read(this + 0x2c);
        let scene = SCENE.get(h);
        let v = h.call_sym(NEW_POOL_OBJECT56, Conv::Thiscall(scene), &[cluster]);
        h.write(this + 4, v);
        virtual_call(h, this, 0x14, &[n, wf4, w16c]);
        let cluster: u32 = h.read(this + 0x2c);
        let scene = SCENE.get(h);
        let v = h.call_sym(NEW_POOL_OBJECT28, Conv::Thiscall(scene), &[cluster]);
        h.write(this + 8, v);
        virtual_call(h, this, 0x20, &[]);
        let cluster: u32 = h.read(this + 0x2c);
        let mesh: u32 = h.read(this + 8);
        let scene = SCENE.get(h);
        let v = h.call_sym(NEW_POOL_OBJECT16, Conv::Thiscall(scene), &[mesh, cluster]);
        h.write(this + 0x10, v);
        virtual_call(h, this, 0x24, &[]);
        let cluster: u32 = h.read(this + 0x2c);
        let first: u32 = h.read(this + 4);
        let scene = SCENE.get(h);
        let v = h.call_sym(NEW_POOL_6599C0, Conv::Thiscall(scene), &[first, cluster]);
        h.write(this + 0xc, v);
        virtual_call(h, this, 0x1c, &[]);
        0
    }

    /// `this`'s virtual at byte offset `off`.
    fn virtual_call(h: &mut impl Host, this: u32, off: u32, args: &[u32]) -> u32 {
        let vtable: u32 = h.read(this);
        let f: u32 = h.read(vtable + off);
        h.call(f, Conv::Thiscall(this), args)
    }
}
