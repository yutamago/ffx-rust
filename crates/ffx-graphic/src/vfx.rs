//! The effects' render data (`docs/formats/render.md`): the
//! `ClassVFXRenderDataTable` that maps an effect instance (a 64-bit key)
//! to its dynamic geometry, and the `graphicVFX*` calls the effect code
//! makes through it (`graphicVFXUpdate`, `InitVFXRenderData`, ...).
//!
//! The dynamic geometry objects are `PhyreEngine`-side C++ (their virtual
//! methods stay original and are called through [`Host`]); the table, its
//! one-entry cache and the per-instance walks are the game's.

use crate::host::{Conv, Host};

/// Instances of one multi-part effect are this far apart in the key space
/// (`graphicVFXUpdate`: `add ebx, 0x2710`).
pub const PART_STRIDE: i32 = 10_000;

/// Level hashes on which, while the filter flag is set, an effect whose
/// geometry has `+0x51 == 1` is not updated (`graphicVFXUpdate`'s list).
pub const FILTERED_LEVELS: [u32; 21] = [
    0x0a3e, 0x0ab0, 0x0e9c, 0x1bbd, 0x24a6, 0x18cd, 0x18e7, 0x22f2, 0x1477, 0x06e1, 0x25fd, 0x07b4,
    0x1e8a, 0x21e8, 0x0822, 0x1975, 0x1883, 0x1505, 0x223f, 0x0c04, 0x0405,
];

/// Whether `hash` is one of [`FILTERED_LEVELS`].
#[must_use]
pub fn level_filtered(hash: u32) -> bool {
    FILTERED_LEVELS.contains(&hash)
}

/// The key of part `index` of the effect whose first part is `key`: the
/// 64-bit sum with the sign-extended `index * 10000`, as the originals'
/// `cdq; add; adc` does.
#[must_use]
#[allow(clippy::cast_sign_loss)] // two's complement, as the adc
pub fn part_key(key: u64, index: i32) -> u64 {
    key.wrapping_add(i64::from(index.wrapping_mul(PART_STRIDE)) as u64)
}

/// The originals, over game memory.
pub mod bridge {
    use super::{Conv, Host, level_filtered, part_key};
    use ffx_bridge::{Global, Mem};

    /// `g_PhyreScene`: the remaster's scene object (`PhyreScene`).
    pub const SCENE: Global<u32> = Global::new(0x00cc_b9d8);
    /// `g_VFXRenderDataTable`: the `ClassVFXRenderDataTable` instance.
    pub const TABLE: Global<u32> = Global::new(0x00cc_ca6c);
    /// `g_GraphicSkipFrame`: set while the game skips drawing a frame;
    /// the effect updates do nothing then.
    pub const SKIP_FRAME: Global<u32> = Global::new(0x012f_b7d0);
    /// `g_GraphicLevelFilter`: the byte that turns the level filter of
    /// `graphicVFXUpdate` on (`== 1`).
    pub const LEVEL_FILTER: Global<u8> = Global::new(0x00cc_b460);

    /// `PhyreScene_isVFXTextureLoaded_00661c80` (thiscall, six arguments).
    pub const SCENE_IS_VFX_TEXTURE_LOADED: u32 = 0x0066_1c80;
    /// `ClusterManager_GetInstance` (cdecl, none).
    pub const CLUSTER_MANAGER_GET_INSTANCE: u32 = 0x0069_a1a0;
    /// `ClusterManager_getPClusterByName` (thiscall, one argument).
    pub const CLUSTER_MANAGER_GET_CLUSTER: u32 = 0x0069_b4b0;
    /// `createVFXRenderDataBuffer` (cdecl, five arguments).
    pub const CREATE_BUFFER: u32 = 0x0071_3790;
    /// `createVFXRenderDataBufferShp` (cdecl, five arguments).
    pub const CREATE_BUFFER_SHP: u32 = 0x0071_5170;
    /// `graphicVFXSetup(lo, hi)`.
    pub const SETUP: u32 = 0x0064_5780;
    /// `graphicVFXDestroy(lo, hi)`.
    pub const DESTROY: u32 = 0x0064_5280;
    /// `updateVFXRenderData(lo, hi, part, obj)`.
    pub const UPDATE_RENDER_DATA: u32 = 0x0071_8610;
    /// `updateVFXRenderDataShp(lo, hi, part, obj, shape)`.
    pub const UPDATE_RENDER_DATA_SHP: u32 = 0x0071_b8a0;

    // ---- the table ----
    // ClassVFXRenderDataTable: +0 the std::map's head node, +0x20/+0x24 the
    // last key looked up, +0x28 its geometry (the one-entry cache).
    // Map nodes: +0 left, +4 parent, +8 right, +0xd is-nil, +0x10 the key
    // (u64), +0x18 the dynamic geometry; geometry +0x24 the render data.

    /// The `std::map<u64, geometry>::find` of the table
    /// (`ClassVFXRenderDataTable_MapFind`): the node holding `key`, else
    /// the head.
    pub fn map_find(m: &impl Mem, table: u32, key: u64) -> u32 {
        let head: u32 = m.read(table);
        let mut found = head;
        let mut node: u32 = m.read(head + 4);
        while m.read::<u8>(node + 0xd) == 0 {
            let k: u64 = m.read(node + 0x10);
            if k < key {
                node = m.read(node + 8);
            } else {
                found = node;
                node = m.read(node);
            }
        }
        if found != head && key >= m.read::<u64>(found + 0x10) {
            found
        } else {
            head
        }
    }

    /// `ClassVFXRenderDataTable_MapFind(this, out, key_ptr)`: stores the
    /// node in `*out`, returns `out`.
    pub fn map_find_out(m: &mut impl Mem, table: u32, out: u32, key_ptr: u32) -> u32 {
        let key: u64 = m.read(key_ptr);
        let node = map_find(m, table, key);
        m.write(out, node);
        out
    }

    /// The geometry for `key` through the table's cache; refills the cache
    /// on a hit in the map. `None` when the map does not hold it.
    fn lookup(m: &mut impl Mem, table: u32, key: u64) -> Option<u32> {
        let cached: u64 = m.read(table + 0x20);
        if cached != 0 && cached == key {
            return Some(m.read(table + 0x28));
        }
        let node = map_find(m, table, key);
        if node == m.read::<u32>(table) {
            return None;
        }
        let geometry: u32 = m.read(node + 0x18);
        m.write(table + 0x20, key);
        m.write(table + 0x28, geometry);
        Some(geometry)
    }

    /// `ClassVFXRenderDataTable_GetDynamicGeometryByInstance(this, lo, hi)`:
    /// the instance's dynamic geometry, 0 when it has none.
    pub fn get_dynamic_geometry(m: &mut impl Mem, table: u32, lo: u32, hi: u32) -> u32 {
        lookup(m, table, key(lo, hi)).unwrap_or(0)
    }

    /// `ClassVFXRenderDataTable_GetVFXDataByInstance(this, lo, hi)`: the
    /// instance's render data (its geometry's `+0x24`), 0 when it has none.
    pub fn get_vfx_data(m: &mut impl Mem, table: u32, lo: u32, hi: u32) -> u32 {
        match lookup(m, table, key(lo, hi)) {
            Some(g) => m.read(g + 0x24),
            None => 0,
        }
    }

    fn key(lo: u32, hi: u32) -> u64 {
        u64::from(hi) << 32 | u64::from(lo)
    }

    fn halves(key: u64) -> (u32, u32) {
        #[allow(clippy::cast_possible_truncation)] // the halves
        (key as u32, (key >> 32) as u32)
    }

    // ---- graphicVFX ----

    /// `graphicVFXGetRenderDataByInstance(lo, hi)`: 0 for the null key.
    pub fn get_render_data_by_instance(m: &mut impl Mem, lo: u32, hi: u32) -> u32 {
        if lo | hi == 0 {
            return 0;
        }
        let table = TABLE.get(m);
        get_vfx_data(m, table, lo, hi)
    }

    /// `graphicDynamicGeometryUpdateTexture(lo, hi)`: the geometry's
    /// virtual `+0xc` (tail-called; its `eax` is returned), else 0.
    pub fn dynamic_geometry_update_texture(h: &mut impl Host, lo: u32, hi: u32) -> u32 {
        let table = TABLE.get(h);
        match get_dynamic_geometry(h, table, lo, hi) {
            0 => 0,
            g => h.call_virtual(g, 3, &[]),
        }
    }

    /// `graphicVFXUpdateWithType(lo, hi, flags)`: hands `flags` (with bit
    /// 30 when the render data's `+0xc1` is set) to the virtual `+8` of
    /// every part's geometry the render data's mask (`+0xc8`) selects.
    pub fn update_with_type(h: &mut impl Host, lo: u32, hi: u32, mut flags: u32) {
        let scene = SCENE.get(h);
        let level = crate::scene::bridge::current_level_name_hash(h, scene);
        if SKIP_FRAME.get(h) != 0 || lo | hi == 0 {
            return;
        }
        let table = TABLE.get(h);
        let first = get_dynamic_geometry(h, table, lo, hi);
        if LEVEL_FILTER.get(h) == 1 && level_filtered(level) && h.read::<u8>(first + 0x51) == 1 {
            return;
        }
        if first == 0 {
            return;
        }
        let data: u32 = h.read(first + 0x24);
        if data == 0 {
            return;
        }
        if h.read::<u8>(data + 0xc1) != 0 {
            flags |= 0x4000_0000;
        }
        let parts = if h.read::<u8>(data + 0xc0) != 0 {
            i32::from(h.read::<u16>(data + 0xc4))
        } else {
            1
        };
        let mut bit = 1u32;
        for i in 0..parts {
            if h.read::<u32>(data + 0xc8) & bit != 0 {
                let (plo, phi) = halves(part_key(key(lo, hi), i));
                let table = TABLE.get(h);
                let g = get_dynamic_geometry(h, table, plo, phi);
                if g != 0 {
                    h.call_virtual(g, 2, &[flags]);
                }
            }
            bit = bit.rotate_left(1);
        }
    }

    /// `graphicVFXUpdate(lo, hi)`.
    pub fn update(h: &mut impl Host, lo: u32, hi: u32) {
        update_with_type(h, lo, hi, 0);
    }

    /// `graphicVFX2DUpdate(lo, hi)`: type 1.
    pub fn update_2d(h: &mut impl Host, lo: u32, hi: u32) {
        update_with_type(h, lo, hi, 1);
    }

    /// `graphicVFXDttUpdate(lo, hi)`: type 2.
    pub fn update_dtt(h: &mut impl Host, lo: u32, hi: u32) {
        update_with_type(h, lo, hi, 2);
    }

    /// `graphicVFXGetDynamicGeometryDataByInstance(lo, hi)`: 0 for the null
    /// key.
    pub fn get_dynamic_geometry_by_instance(m: &mut impl Mem, lo: u32, hi: u32) -> u32 {
        if lo | hi == 0 {
            return 0;
        }
        let table = TABLE.get(m);
        get_dynamic_geometry(m, table, lo, hi)
    }

    /// The instance's geometry, for the wrappers that do nothing without
    /// one.
    fn geometry(h: &mut impl Host, lo: u32, hi: u32) -> Option<u32> {
        match get_dynamic_geometry_by_instance(h, lo, hi) {
            0 => None,
            g => Some(g),
        }
    }

    /// `ClassDynamicGeometry_show(this)`.
    pub const SHOW: u32 = 0x006a_49e0;
    /// `ClassDynamicGeometry_hide(this)`.
    pub const HIDE: u32 = 0x006a_2370;
    /// `VFXDynamicGeometry_setYIQEnable(this, on)`.
    pub const SET_YIQ_ENABLE: u32 = 0x006a_3a20;
    /// `VFXDynamicGeometry_setPointLightParameters(this, matrix, color, power)`.
    pub const SET_POINT_LIGHT: u32 = 0x006a_2fd0;
    /// `VFXDynamicGeometry_setTkRuntimeClutParameters(this, a, b, f)`.
    pub const SET_CLUT: u32 = 0x006a_37f0;
    /// `VFXDynamicGeometry_setZCorrectionValue(this, f)`.
    pub const SET_Z_CORRECTION: u32 = 0x006a_3b20;
    /// `VFXDynamicGeometry_setUVOffset(this, uv)`.
    pub const SET_UV_OFFSET: u32 = 0x006a_3a00;
    /// `VFXDynamicGeometry_setRGBAPower(this, rgba)`.
    pub const SET_RGBA_POWER: u32 = 0x006a_30d0;
    /// `Phyre_PString_Assign(this, text)`: a `PString`'s copy.
    pub const PSTRING_ASSIGN: u32 = 0x0044_5cf0;
    /// `PhyreScene_VFXSyncPath(this)`.
    pub const SCENE_SYNC_PATH: u32 = 0x0065_4cc0;
    /// `ClassVFXRenderDataTable_Clear(this)`.
    pub const TABLE_CLEAR: u32 = 0x0069_f000;
    /// `ClassVFXRenderDataTable_ClearByType(this, keep)`.
    pub const TABLE_CLEAR_BY_TYPE: u32 = 0x0069_f110;
    /// `ClassVFXRenderDataTable_RemoveDeletedPoolMeshInstances(this)`.
    pub const TABLE_REMOVE_DELETED: u32 = 0x0069_fc70;
    /// `ClassVFXRenderDataTable_UpdateDeletePool(this)`.
    pub const TABLE_UPDATE_DELETE_POOL: u32 = 0x0069_fe50;
    /// `PhyreScene_loadPersistentVFXTexture(this)`.
    pub const SCENE_LOAD_PERSISTENT_TEXTURE: u32 = 0x0066_5660;
    /// `PhyreScene_loadMapVFXTexture(this)`.
    pub const SCENE_LOAD_MAP_TEXTURE: u32 = 0x0066_4f60;
    /// `PhyreScene_unloadMapVFXTexture(this)`.
    pub const SCENE_UNLOAD_MAP_TEXTURE: u32 = 0x0067_0fe0;
    /// `PhyreScene_releaseVFXMeshes(this)` (`0x00659090`: on the scene's
    /// `+0x10d54`).
    pub const SCENE_RELEASE_VFX: u32 = 0x0065_9090;
    /// `VfxShaderManager_sGetInstance()`.
    pub const SHADER_MANAGER: u32 = 0x006c_14c0;
    /// `VfxShaderManager_loadSpecialShaderTable(this)`: an empty method.
    pub const SHADER_LOAD_SPECIAL: u32 = 0x006c_0fa0;
    /// `VfxShaderManager_unloadSpecialShaderTable(this)`.
    pub const SHADER_UNLOAD_SPECIAL: u32 = 0x006c_19b0;

    /// `graphicVFXSetVisible(lo, hi, visible)`: shows or hides the
    /// geometry; returns the method's `eax`, else 0.
    pub fn set_visible(h: &mut impl Host, lo: u32, hi: u32, visible: u32) -> u32 {
        let Some(g) = geometry(h, lo, hi) else {
            return 0;
        };
        let f = if visible != 0 { SHOW } else { HIDE };
        h.call_sym(f, Conv::Thiscall(g), &[])
    }

    /// `graphicVFXEnableYIQ(lo, hi)`.
    pub fn enable_yiq(h: &mut impl Host, lo: u32, hi: u32) -> u32 {
        let Some(g) = geometry(h, lo, hi) else {
            return 0;
        };
        h.call_sym(SET_YIQ_ENABLE, Conv::Thiscall(g), &[1])
    }

    /// `graphicVFXSetPointLightParameters(lo, hi, a, b, f)` (`SET_POINT_LIGHT`)
    /// and `graphicVFXSetClutParameters` (`SET_CLUT`): the float goes
    /// through the x87 (`fld`/`fstp`).
    #[allow(clippy::many_single_char_names)] // the arguments
    pub fn set_parameters(
        h: &mut impl Host,
        method: u32,
        lo: u32,
        hi: u32,
        a: u32,
        b: u32,
        f: f32,
    ) -> u32 {
        let Some(g) = geometry(h, lo, hi) else {
            return 0;
        };
        let f = ffx_bridge::x87::round_trip(f).to_bits();
        h.call_sym(method, Conv::Thiscall(g), &[a, b, f])
    }

    /// `graphicVFXSetZCorrectionValue(lo, hi, f)`.
    pub fn set_z_correction(h: &mut impl Host, lo: u32, hi: u32, f: f32) -> u32 {
        let Some(g) = geometry(h, lo, hi) else {
            return 0;
        };
        let f = ffx_bridge::x87::round_trip(f).to_bits();
        h.call_sym(SET_Z_CORRECTION, Conv::Thiscall(g), &[f])
    }

    /// `graphicVFXSetMeshInstanceName(lo, hi, name)`: the name of the
    /// geometry's mesh instance (`+0xc`, its `PString` at `+0x2c`). The
    /// original does not check for a missing geometry.
    pub fn set_mesh_instance_name(h: &mut impl Host, lo: u32, hi: u32, name: u32) -> u32 {
        if lo | hi == 0 {
            return 0;
        }
        let table = TABLE.get(h);
        let g = get_dynamic_geometry(h, table, lo, hi);
        let mesh: u32 = h.read(g + 0xc);
        h.call_sym(
            PSTRING_ASSIGN,
            Conv::Thiscall(mesh.wrapping_add(0x2c)),
            &[name],
        )
    }

    /// `graphicVFXUpdateParameterOnly(g, z, uv, rgba)`: shows the geometry,
    /// sets its Z correction and, when given, its UV offset and RGBA power,
    /// then its virtual `+0x28` with 0.
    pub fn update_parameter_only(h: &mut impl Host, g: u32, z: f32, uv: u32, rgba: u32) {
        if g == 0 {
            return;
        }
        h.call_sym(SHOW, Conv::Thiscall(g), &[]);
        let z = ffx_bridge::x87::round_trip(z).to_bits();
        h.call_sym(SET_Z_CORRECTION, Conv::Thiscall(g), &[z]);
        if uv != 0 {
            h.call_sym(SET_UV_OFFSET, Conv::Thiscall(g), &[uv]);
        }
        if rgba != 0 {
            h.call_sym(SET_RGBA_POWER, Conv::Thiscall(g), &[rgba]);
        }
        h.call_virtual(g, 10, &[0]);
    }

    /// `graphicVFXSyncPath()`: the scene's path sync, unless the frame is
    /// skipped.
    pub fn sync_path(h: &mut impl Host) {
        if SKIP_FRAME.get(h) == 0 {
            let scene = SCENE.get(h);
            h.call_sym(SCENE_SYNC_PATH, Conv::Thiscall(scene), &[]);
        }
    }

    /// The table-wide calls: `graphicVFXDestroyAll` (`TABLE_CLEAR`),
    /// `graphicVFXRemoveDeletedPoolMeshInstances`, `graphicVFXUpdateDeletedPool`.
    pub fn table_call(h: &mut impl Host, method: u32) -> u32 {
        let table = TABLE.get(h);
        h.call_sym(method, Conv::Thiscall(table), &[])
    }

    /// `graphicVFXDestroyAllExceptFieldAndEternal()`: clears all but type
    /// 0xb.
    pub fn destroy_all_except_field_and_eternal(h: &mut impl Host) -> u32 {
        let table = TABLE.get(h);
        h.call_sym(TABLE_CLEAR_BY_TYPE, Conv::Thiscall(table), &[0xb])
    }

    /// `graphicVFXLoadPersistentTexture()`.
    pub fn load_persistent_texture(h: &mut impl Host) -> u32 {
        let scene = SCENE.get(h);
        h.call_sym(SCENE_LOAD_PERSISTENT_TEXTURE, Conv::Thiscall(scene), &[])
    }

    /// `graphicVFXLoadLevelData()`: the map's effect textures, then the
    /// shader manager's special table.
    pub fn load_level_data(h: &mut impl Host) -> u32 {
        let scene = SCENE.get(h);
        h.call_sym(SCENE_LOAD_MAP_TEXTURE, Conv::Thiscall(scene), &[]);
        let shaders = h.call_sym(SHADER_MANAGER, Conv::Cdecl, &[]);
        h.call_sym(SHADER_LOAD_SPECIAL, Conv::Thiscall(shaders), &[])
    }

    /// `graphicVFXUnloadLevelData()`: the reverse, then the scene's effect
    /// meshes.
    pub fn unload_level_data(h: &mut impl Host) -> u32 {
        let scene = SCENE.get(h);
        h.call_sym(SCENE_UNLOAD_MAP_TEXTURE, Conv::Thiscall(scene), &[]);
        let shaders = h.call_sym(SHADER_MANAGER, Conv::Cdecl, &[]);
        h.call_sym(SHADER_UNLOAD_SPECIAL, Conv::Thiscall(shaders), &[]);
        let scene = SCENE.get(h);
        h.call_sym(SCENE_RELEASE_VFX, Conv::Thiscall(scene), &[])
    }

    /// `graphicVFXPath3Update(lo, hi)`: with type bit 31.
    pub fn path3_update(h: &mut impl Host, lo: u32, hi: u32) {
        update_with_type(h, lo, hi, 0x8000_0000);
    }

    /// `graphicVFXIsTextureLoaded(a, name, c, d, e, f)`: 2 as the scene
    /// says; 1 when the scene says 1 and the cluster `name` exists; else 0.
    pub fn is_texture_loaded(h: &mut impl Host, args: [u32; 6]) -> u32 {
        let scene = SCENE.get(h);
        match h.call_sym(SCENE_IS_VFX_TEXTURE_LOADED, Conv::Thiscall(scene), &args) {
            2 => 2,
            1 => {
                let manager = h.call_sym(CLUSTER_MANAGER_GET_INSTANCE, Conv::Cdecl, &[]);
                let cluster = h.call_sym(
                    CLUSTER_MANAGER_GET_CLUSTER,
                    Conv::Thiscall(manager),
                    &[args[1]],
                );
                u32::from(cluster != 0)
            }
            _ => 0,
        }
    }

    // ---- set-up ----

    /// `InitVFXRenderData(lo, hi, part, obj, e)` (and
    /// `InitVFXPath3RenderData` with `path3`): makes the instance's buffer
    /// and render data, fills them and updates them; destroys the
    /// instance when a step fails.
    pub fn init_render_data(h: &mut impl Host, args: [u32; 5], path3: bool) {
        let [lo, hi, part, obj, _] = args;
        if h.call_sym(CREATE_BUFFER, Conv::Cdecl, &args) != 0
            && h.call_sym(SETUP, Conv::Cdecl, &[lo, hi]) != 0
        {
            h.call_sym(UPDATE_RENDER_DATA, Conv::Cdecl, &[lo, hi, part, obj]);
            if path3 {
                path3_update(h, lo, hi);
            } else {
                update(h, lo, hi);
            }
            return;
        }
        h.call_sym(DESTROY, Conv::Cdecl, &[lo, hi]);
    }

    /// `InitVFXRenderDataShp(lo, hi, part, obj, shape, f)` (and
    /// `InitVFXPath3RenderDataShp` with `path3`): as
    /// [`init_render_data`] for a shape; halves every segment's `+8`
    /// after the set-up, even when the set-up failed.
    pub fn init_render_data_shp(h: &mut impl Host, args: [u32; 6], path3: bool) {
        let [lo, hi, part, obj, shape, f] = args;
        if h.call_sym(CREATE_BUFFER_SHP, Conv::Cdecl, &[lo, hi, part, shape, f]) != 0 {
            let ok = h.call_sym(SETUP, Conv::Cdecl, &[lo, hi]);
            let data = get_render_data_by_instance(h, lo, hi);
            if data != 0 {
                let mut i = 0u32;
                #[allow(clippy::cast_possible_wrap)] // the original's signed compare
                while (i as i32) < h.read::<i32>(data + 0x90) {
                    let at = h.read::<u32>(data + 0x94) + 0x6c * i + 8;
                    let v: u32 = h.read(at);
                    h.write(at, v >> 1);
                    i += 1;
                }
            }
            if ok != 0 {
                h.call_sym(
                    UPDATE_RENDER_DATA_SHP,
                    Conv::Cdecl,
                    &[lo, hi, part, obj, shape],
                );
                if path3 {
                    path3_update(h, lo, hi);
                } else {
                    update(h, lo, hi);
                }
                return;
            }
        }
        h.call_sym(DESTROY, Conv::Cdecl, &[lo, hi]);
    }
}
