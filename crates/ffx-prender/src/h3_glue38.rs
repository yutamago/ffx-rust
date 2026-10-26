//! The render-side glue, thirty-eighth batch (M4 port round 9, list
//! `h3-glue-render`): the field map's load, whose cluster requests take
//! nineteen stack words (`ffx_graphic::host::Host::call_wide`).

use ffx_bridge::Global;

/// `strstr` (cdecl, its import slot) and the four names it looks for:
/// `"znkd12"`, `"lchb08"`, `"lchb12"`, `"grid00"`.
pub const STRSTR: u32 = crate::h3_glue34::STRSTR;
pub const ZNKD12: u32 = crate::h3_glue34::ZNKD12;
pub const LCHB08: u32 = 0x00b4_24b8;
pub const LCHB12: u32 = 0x00b4_24c0;
pub const GRID00: u32 = 0x00b4_24c8;

/// `LoadCameraData(movie)` and `UnloadCameraData()` (cdecl).
pub const LOAD_CAMERA_DATA: u32 = 0x006f_6dc0;
pub const UNLOAD_CAMERA_DATA: u32 = 0x006f_6f80;

/// The maps with a camera file, and the camera movie each loads.
pub const CAMERA_MAPS: [(u32, u32); 4] =
    [(0x1f20, 0x2e), (0x220, 0x48), (0xc04, 0x20), (0xa6c, 0x46)];

/// `PhyreScene_readAssetHeader(this; path, b)` (`ret 8`).
pub const READ_ASSET_HEADER: u32 = 0x0066_7bb0;

/// `g_CurrentMapNo`, the scene's flag for maps 0x163 and 0x164,
/// `g_PhyreScene3D` (a byte) and the 30 maps that load as 2D.
pub const CURRENT_MAP_NO: Global<u32> = crate::glue5::CURRENT_MAP_NO;
pub const MAP_163_164: Global<u32> = Global::new(0x00cc_b9c4);
pub const PHYRE_SCENE_3D: Global<u8> = crate::h3_glue34::PHYRE_SCENE_3D;
pub const MAPS_2D: u32 = 0x00b4_21c8;
pub const MAPS_2D_LEN: u32 = 30;

/// `AssetListReader_getAssetCount(this)` and `getAssetPath(this; i)`
/// (`ret 4`), `AsyncLoadManager_sGetInstance()` (cdecl),
/// `createChildrenRequestList(this; n)` (`ret 4`), `loadCluster`
/// (cdecl, 19 words: the manager, the list, the path, the callback, its
/// argument count and its seven 64-bit arguments) and the callback
/// `PhyreScene_onFieldMapAssetLoaded`.
pub const ASSET_COUNT: u32 = 0x006f_4170;
pub const ASSET_PATH: u32 = 0x006f_41b0;
pub const ASYNC_LOAD_INSTANCE: u32 = crate::h3_glue34::ASYNC_LOAD_INSTANCE;
pub const CHILDREN_REQUEST_LIST: u32 = 0x0069_0f70;
pub const LOAD_CLUSTER: u32 = 0x0069_1660;
pub const ON_ASSET_LOADED: u32 = 0x0066_6630;

/// `g_FreeCamera`, `operator new` (cdecl), `FreeCamera_FreeCamera(this)`,
/// `FreeCamera_setType(this; type)` (`ret 4`), the renderer's
/// `syncRender(this)`.
pub const FREE_CAMERA: Global<u32> = Global::new(0x00cc_c830);
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const FREE_CAMERA_CTOR: u32 = 0x0067_fe70;
pub const FREE_CAMERA_SET_TYPE: u32 = 0x0068_0410;
pub const SYNC_RENDER: u32 = 0x005b_c3f0;

/// `ClusterManager_GetInstance()` (cdecl), `releasePCluster(this;
/// cluster)` (`ret 4`), `AssetListReader_clear(this)`,
/// `Phyre_PClusterBase_clear(this)`.
pub const CLUSTER_MANAGER_INSTANCE: u32 = 0x0069_a1a0;
pub const RELEASE_CLUSTER: u32 = 0x0069_bdb0;
pub const ASSET_LIST_CLEAR: u32 = 0x006f_4030;
pub const CLUSTER_BASE_CLEAR: u32 = 0x0044_0ea0;

/// `user_malloc(size)`, `user_free_thunk(p)`, `operator delete(p)`
/// (cdecl); the red-black tree's `successor(this; node)` (`ret 4`) and
/// `orphanAllContents(this)`, `Phyre_PFreeList_reset(this)`.
pub const USER_MALLOC: u32 = 0x0068_7020;
pub const USER_FREE: u32 = 0x0068_7010;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
pub const TREE_SUCCESSOR: u32 = 0x0057_5160;
pub const TREE_ORPHAN_ALL: u32 = 0x0057_4970;
pub const FREE_LIST_RESET: u32 = 0x0049_cc10;

/// The singletons reset at the end: `Phyre_CharacterLighting_GetInstance`
/// / `reset`, `Phyre_FogEffectManager_GetInstance` / `reset`,
/// `Instance_006dc800_Get` / `0x006dcdc0` (getters cdecl, methods
/// `thiscall` without arguments).
pub const CHARACTER_LIGHTING_INSTANCE: u32 = 0x006a_6e30;
pub const CHARACTER_LIGHTING_RESET: u32 = 0x006a_a010;
pub const FOG_INSTANCE: u32 = 0x006e_32c0;
pub const FOG_RESET: u32 = 0x006e_34f0;
pub const INSTANCE_6DC800: u32 = 0x006d_c8a0;
pub const RESET_6DC800: u32 = 0x006d_cdc0;

/// The shader maps' copy constructor `0x006bee80(this; source)` (`ret
/// 4`), the red-black tree's `remove(this; node)` (`ret 4`) and
/// `clear(this)`, `Phyre_PFreeList_dtor_PFreeList(this)`,
/// `Phyre_PhyreFree(p)` (cdecl).
pub const SHADER_MAP_COPY: u32 = 0x006b_ee80;
pub const TREE_REMOVE: u32 = 0x0057_5000;
pub const TREE_CLEAR: u32 = 0x0057_4760;
pub const FREE_LIST_DTOR: u32 = 0x0049_c5b0;
pub const PHYRE_FREE: u32 = 0x0042_fb20;

/// One of `VfxShaderManager`'s two special shader tables: the map's
/// header offset, its "loaded" byte, and whether it is the magic file's
/// (which also resets `+0x64` and keeps `this`, not the header, in its
/// frame).
#[derive(Debug, Clone, Copy)]
pub struct ShaderTable {
    pub map: u32,
    pub loaded: u32,
    pub magic: bool,
}

/// `VfxShaderManager_unloadSpecialShaderTable`'s table.
pub const SPECIAL_TABLE: ShaderTable = ShaderTable {
    map: 0x30,
    loaded: 0x60,
    magic: false,
};
/// `VfxShaderManager_unloadSpecialShaderTable_magic`'s table.
pub const MAGIC_TABLE: ShaderTable = ShaderTable {
    map: 0x68,
    loaded: 0x98,
    magic: true,
};

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    /// A cdecl import through its slot.
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// The high word of a sign-extended `cdq`.
    fn high(v: u32) -> u32 {
        if v & 0x8000_0000 == 0 { 0 } else { u32::MAX }
    }

    /// `PhyreScene_fieldMapLoad(this, path, b; slots)` (thiscall, `ret 8`):
    /// `znkd12` maps and map 0x220 clear `+0xf9c4`; the map's camera data
    /// is loaded (four maps) or unloaded; `+0` is whether the path holds
    /// `lchb08`, `lchb12` sets `+0x10d2e`, `+0xf9c5` is whether it lacks
    /// `grid00`; the 0x163/0x164 flag. Then the asset header: an error
    /// sets `+0x1194` when both kinds (3D `+0x10d1`/`+0x10d8`, 2D
    /// `+0x10d0`/`+0x112c`) are idle or loaded, clears `+0x1195` and is
    /// returned. Else `b`'s low byte 1 picks the 3D set (`+0xac`,
    /// `+0x10d4`, `+0x10d8`, the list `+0x10e4`; `g_PhyreScene3D` set),
    /// another the 2D one (`+0x8c`, `+0x1128`, `+0x112c`, `+0x1144`;
    /// `+0x93c` set); a map of the 2D table clears `g_PhyreScene3D`;
    /// `+0xd8` is cleared, `+0x1195` set, and every asset of the list is
    /// requested as a cluster of a children list. Returns 0.
    ///
    /// The original keeps the third pointer in `path`'s slot, then its
    /// high word there and `b`'s low byte in `b`'s slot (the loop's
    /// pushes read them).
    pub fn field_map_load(h: &mut impl Host, slots: u32, this: u32, path: u32, b: u32) -> u32 {
        let znkd = h.rebase(ZNKD12);
        if import(h, STRSTR, &[path, znkd]) != 0 || h.read::<u32>(this + 0xfaf4) == 0x220 {
            h.write(this + 0xf9c4, 0u8);
        }
        let map: u32 = h.read(this + 0xfaf4);
        match CAMERA_MAPS.iter().find(|&&(m, _)| m == map) {
            Some(&(_, movie)) => {
                h.call_sym(LOAD_CAMERA_DATA, Conv::Cdecl, &[movie]);
            }
            None => {
                h.call_sym(UNLOAD_CAMERA_DATA, Conv::Cdecl, &[]);
            }
        }
        let name = h.rebase(LCHB08);
        let found = import(h, STRSTR, &[path, name]) != 0;
        h.write(this, u8::from(found));
        let name = h.rebase(LCHB12);
        if import(h, STRSTR, &[path, name]) != 0 {
            h.write(this + 0x10d2e, 1u8);
        }
        let name = h.rebase(GRID00);
        let grid = import(h, STRSTR, &[path, name]) != 0;
        h.write(this + 0xf9c5, u8::from(!grid));
        let current = CURRENT_MAP_NO.get(h);
        MAP_163_164.set(h, u32::from(current == 0x163 || current == 0x164));
        let error = h.call_sym(READ_ASSET_HEADER, Conv::Thiscall(this), &[path, b]);
        if error != 0 {
            let three_d = h.read::<u8>(this + 0x10d1) == 0 || h.read::<u32>(this + 0x10d8) != 0;
            let two_d = h.read::<u8>(this + 0x10d0) == 0 || h.read::<u32>(this + 0x112c) != 0;
            if three_d && two_d {
                h.write(this + 0x1194, 1u8);
            }
            h.write(this + 0x1195, 0u8);
            return error;
        }
        let (c, d, e, list) = if b as u8 == 1 {
            PHYRE_SCENE_3D.set(h, 1);
            (this + 0xac, this + 0x10d4, this + 0x10d8, this + 0x10e4)
        } else {
            h.write(this + 0x93c, 1u8);
            (this + 0x8c, this + 0x1128, this + 0x112c, this + 0x1144)
        };
        h.write(slots, e);
        let table = h.rebase(MAPS_2D);
        let map: u32 = h.read(this + 0xfaf4);
        if (0..MAPS_2D_LEN).any(|k| h.read::<u32>(table + 4 * k) == map) {
            PHYRE_SCENE_3D.set(h, 0);
        }
        h.write(this + 0xd8, 0u32);
        h.write(this + 0x1195, 1u8);
        let n = h.call_sym(ASSET_COUNT, Conv::Thiscall(list), &[]);
        let manager = h.call_sym(ASYNC_LOAD_INSTANCE, Conv::Cdecl, &[]);
        let children = h.call_sym(CHILDREN_REQUEST_LIST, Conv::Thiscall(manager), &[n]);
        let count = h.call_sym(ASSET_COUNT, Conv::Thiscall(list), &[]);
        if count as i32 <= 0 {
            return 0;
        }
        let low = b & 0xff;
        h.write(slots + 4, low);
        h.write(slots, high(e));
        let callback = h.rebase(ON_ASSET_LOADED);
        let mut i = 0u32;
        loop {
            let asset = h.call_sym(ASSET_PATH, Conv::Thiscall(list), &[i]);
            let manager = h.call_sym(ASYNC_LOAD_INSTANCE, Conv::Cdecl, &[]);
            h.call_sym_wide(
                LOAD_CLUSTER,
                &[
                    manager,
                    children,
                    asset,
                    callback,
                    7,
                    this,
                    high(this),
                    c,
                    high(c),
                    d,
                    high(d),
                    e,
                    high(e),
                    list,
                    high(list),
                    i,
                    high(i),
                    low,
                    0,
                ],
            );
            i = i.wrapping_add(1);
            let count = h.call_sym(ASSET_COUNT, Conv::Thiscall(list), &[]);
            if i as i32 >= count as i32 {
                return 0;
            }
        }
    }

    /// A `PArray`'s reset (count `+0`, capacity `+4`, data `+8`): with
    /// both negative (borrowed storage), the data is freed (or, when
    /// there is none, `user_malloc(0)` taken) and the capacity cleared;
    /// the count cleared either way.
    fn reset_array(h: &mut impl Host, a: u32) {
        if (h.read::<u32>(a) as i32) < 0 && (h.read::<u32>(a + 4) as i32) < 0 {
            let data: u32 = h.read(a + 8);
            if data == 0 {
                let p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[0]);
                h.write(a + 8, p);
            } else {
                h.call_sym(USER_FREE, Conv::Cdecl, &[data]);
                h.write(a + 8, 0u32);
            }
            h.write(a + 4, 0u32);
        }
        h.write(a, 0u32);
    }

    /// Releases the non-null clusters of the array at `this + count`
    /// (count, data at `+8`) and clears their slots.
    fn release_clusters(h: &mut impl Host, this: u32, count: u32) {
        let mut i = 0u32;
        while i < h.read::<u32>(this + count) {
            let data: u32 = h.read(this + count + 8);
            let cluster: u32 = h.read(data.wrapping_add(4 * i));
            if cluster != 0 {
                let manager = h.call_sym(CLUSTER_MANAGER_INSTANCE, Conv::Cdecl, &[]);
                h.call_sym(RELEASE_CLUSTER, Conv::Thiscall(manager), &[cluster]);
                let data: u32 = h.read(this + count + 8);
                h.write(data.wrapping_add(4 * i), 0u32);
            }
            i = i.wrapping_add(1);
        }
    }

    /// `PhyreScene_fieldMapUnload(this)` (thiscall, no stack words; its
    /// SEH frame and cookie left out): the free camera (made when there is
    /// none) set to type 0; the renderer synced; the 3D and 2D clusters
    /// released; both asset lists and the cluster base cleared; the
    /// arrays `+0x10d8`, `+0x112c`, `+0x1138` reset and `+0x10d4`,
    /// `+0x1128` cleared; every value of the tree at `+0x1198` freed (its
    /// `+8` block, then itself), the tree orphaned and the free list
    /// `+0x11b0` reset; the array `+0x11c8` reset; the character
    /// lighting, fog and `0x006dc800` singletons reset; the 2D/3D flags
    /// and `+0xfb14`, `+0xf9bc`, `+0xf9c0` (0.0) cleared. Returns 0.
    pub fn field_map_unload(h: &mut impl Host, this: u32) -> u32 {
        let mut camera = FREE_CAMERA.get(h);
        if camera == 0 {
            let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x340]);
            camera = if p == 0 {
                0
            } else {
                h.call_sym(FREE_CAMERA_CTOR, Conv::Thiscall(p), &[])
            };
            FREE_CAMERA.set(h, camera);
        }
        h.call_sym(FREE_CAMERA_SET_TYPE, Conv::Thiscall(camera), &[0]);
        let renderer: u32 = h.read(this + 0xd0);
        if renderer != 0 {
            h.call_sym(SYNC_RENDER, Conv::Thiscall(renderer), &[]);
        }
        release_clusters(h, this, 0x10d8);
        release_clusters(h, this, 0x112c);
        h.call_sym(ASSET_LIST_CLEAR, Conv::Thiscall(this + 0x10e4), &[]);
        h.call_sym(ASSET_LIST_CLEAR, Conv::Thiscall(this + 0x1144), &[]);
        h.call_sym(CLUSTER_BASE_CLEAR, Conv::Thiscall(this + 0xf96c), &[]);
        reset_array(h, this + 0x10d8);
        reset_array(h, this + 0x112c);
        reset_array(h, this + 0x1138);
        h.write(this + 0x10d4, 0u32);
        h.write(this + 0x1128, 0u32);
        // the tree's values (root slot `+0x11a4`)
        let header = this + 0x1198;
        walk(h, header, |h, node| {
            let value: u32 = h.read(node.wrapping_add(0x10));
            if value != 0 {
                let block: u32 = h.read(value + 8);
                if block != 0 {
                    h.call_sym(USER_FREE, Conv::Cdecl, &[block]);
                }
                h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[value]);
            }
        });
        h.call_sym(TREE_ORPHAN_ALL, Conv::Thiscall(header), &[]);
        h.call_sym(FREE_LIST_RESET, Conv::Thiscall(this + 0x11b0), &[]);
        reset_array(h, this + 0x11c8);
        for (getter, reset) in [
            (CHARACTER_LIGHTING_INSTANCE, CHARACTER_LIGHTING_RESET),
            (FOG_INSTANCE, FOG_RESET),
            (INSTANCE_6DC800, RESET_6DC800),
        ] {
            let instance = h.call_sym(getter, Conv::Cdecl, &[]);
            h.call_sym(reset, Conv::Thiscall(instance), &[]);
        }
        h.write(this + 0xfb14, 0u32);
        h.write(this + 0x93c, 0u8);
        PHYRE_SCENE_3D.set(h, 0);
        h.write(this + 0xf9bc, 0f32.to_bits());
        h.write(this + 0xf9c0, 0f32.to_bits());
        h.write(this + 0x93d, 0u16);
        0
    }

    /// A red-black tree's first node: from its root slot (`header +
    /// 0xc`) down the chain of `+0` links to the node before the header;
    /// the slot's value itself when it is the header or 0.
    fn leftmost(h: &impl Host, header: u32) -> u32 {
        let mut node: u32 = h.read(header + 0xc);
        if node != header && node != 0 {
            loop {
                let next: u32 = h.read(node);
                if next == header {
                    break;
                }
                node = next;
            }
        }
        node
    }

    /// Calls `f` on every node of the tree at `header`, from
    /// [`leftmost`] through `successor`, which is asked for the next
    /// node before `f` runs.
    fn walk<H: Host>(h: &mut H, header: u32, mut f: impl FnMut(&mut H, u32)) {
        let mut node = leftmost(h, header);
        if node == header {
            return;
        }
        let mut next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(header), &[node]);
        loop {
            f(h, node);
            node = next;
            if node == header {
                return;
            }
            next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(header), &[node]);
        }
    }

    /// Empties the tree at `header`: removes its root (`+0xc`) until it
    /// is the header or 0, freeing a node's `+0xc` block unless its low
    /// bit is set; then resets its free list (`+0x18`), whose `eax` it
    /// returns.
    fn drain(h: &mut impl Host, header: u32) -> u32 {
        loop {
            let node: u32 = h.read(header + 0xc);
            if node == header || node == 0 {
                break;
            }
            h.call_sym(TREE_REMOVE, Conv::Thiscall(header), &[node]);
            let block: u32 = h.read(node + 0xc);
            if block & 1 == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[block]);
            }
        }
        h.call_sym(FREE_LIST_RESET, Conv::Thiscall(header + 0x18), &[])
    }

    /// Destroys the map at `header`: [`drain`], its free list's
    /// destructor and the tree's `clear` twice.
    fn destroy_map(h: &mut impl Host, header: u32) {
        drain(h, header);
        h.call_sym(FREE_LIST_DTOR, Conv::Thiscall(header + 0x18), &[]);
        h.call_sym(TREE_CLEAR, Conv::Thiscall(header), &[]);
        h.call_sym(TREE_CLEAR, Conv::Thiscall(header), &[]);
    }

    /// `VfxShaderManager_unloadSpecialShaderTable(this)` and
    /// `_magic(this)` (thiscall, no stack words; SEH frame and cookie
    /// left out; frame: a copy of an entry's map at `ebp - 0x4c`, the
    /// entry's value at `- 0x14` (`- 0x18` for the magic table), the
    /// header at `- 0x18` (`this` at `- 0x14` for the magic table), the
    /// last pointer freed at `- 0x1c`; the original's next entry at `-
    /// 0x10` stays a local, as `walk`'s): the
    /// table's "loaded" byte cleared; for each entry of the table's map,
    /// its value's map (`value + 4`) is copied into the frame, each name
    /// pair there freed (the two words' blocks unless their low bits are
    /// set, then the pair) and the copy drained; the value's own map
    /// destroyed and the value deleted; the copy destroyed. Then the
    /// table's map is drained; the magic table's `+0x64` becomes -1
    /// unless negative. Returns what the original leaves in `eax`: the
    /// last free-list reset's, or `this` for the magic table
    /// (`graphicMagicVFXUnloadSpecialShaderTable` returns it on).
    pub fn unload_special_shader_table(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        table: ShaderTable,
    ) -> u32 {
        let header = this + table.map;
        let value_at = if table.magic { 0x18 } else { 0x14 };
        if table.magic {
            h.write(ebp - 0x14, this);
        }
        h.write(this + table.loaded, 0u8);
        if !table.magic {
            h.write(ebp - 0x18, header);
        }
        let copy = ebp - 0x4c;
        walk(h, header, |h, node| {
            let value: u32 = h.read(node.wrapping_add(0x10));
            h.write(ebp - value_at, value);
            h.call_sym(
                SHADER_MAP_COPY,
                Conv::Thiscall(copy),
                &[value.wrapping_add(4)],
            );
            walk(h, copy, |h, n| {
                let pair: u32 = h.read(n.wrapping_add(0x10));
                h.write(ebp - 0x1c, pair);
                if pair != 0 {
                    for at in [pair + 4, pair] {
                        let block: u32 = h.read(at);
                        if block & 1 == 0 {
                            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[block]);
                        }
                    }
                    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[pair]);
                }
            });
            drain(h, copy);
            if value != 0 {
                let map = value + 4;
                h.write(ebp - 0x1c, map);
                destroy_map(h, map);
                h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[value]);
            }
            destroy_map(h, copy);
        });
        let eax = drain(h, header);
        if !table.magic {
            return eax;
        }
        if (h.read::<u32>(this + 0x64) as i32) > -1 {
            h.write(this + 0x64, u32::MAX);
        }
        this
    }
}
