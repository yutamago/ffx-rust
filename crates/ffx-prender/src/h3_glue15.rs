//! The render-side glue, fifteenth batch (M4 port round 5, list
//! `h3-glue-render`): delegation registration, the guild map's geometry,
//! the cloth vertex buffers' release, the scene's material pool, the
//! render-to-texture set-up, the field-map load and the main menu's
//! water. Cookies left out, as in [`crate::glue8`]; frames passed to
//! callees as in [`crate::glue5`].

use ffx_bridge::Global;

pub use crate::glue::SCENE;

/// `operator new(size)`, `operator delete(p)`, `operator new[](size)`,
/// `Phyre_PhyreMalloc_0042fb80(size, align)` (cdecl), `memset` and
/// `memcpy` (the import thunks, cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
pub const ARRAY_NEW: u32 = 0x0063_04c0;
pub const PHYRE_MALLOC_ALIGNED: u32 = 0x0042_fb80;
pub const MEMSET: u32 = crate::glue8::MEMSET;
pub const MEMCPY: u32 = crate::glue11::MEMCPY;
/// The delegation list's mutex, the imports `InterlockedCompareExchange`,
/// `WaitForSingleObject` and `ReleaseMutex` (their slots, stdcall), and
/// the list's grow-and-push `0x006add90(this, storage, &delegation)`.
pub const DELEGATION_MUTEX: Global<u32> = crate::glue10::DELEGATION_MUTEX;
pub const INTERLOCKED_CAS: u32 = 0x00b0_c12c;
pub const WAIT_FOR_SINGLE_OBJECT: u32 = crate::glue::WAIT_FOR_SINGLE_OBJECT;
pub const RELEASE_MUTEX: u32 = crate::glue::RELEASE_MUTEX;
pub const DELEGATION_GROW_PUSH: u32 = 0x006a_dd90;

/// `g_VFXRenderDataTable`, `ClassVFXRenderDataTable_GetDynamicGeometryByInstance(this, a, b)`,
/// `GuildMapDynamicGeometry_GuildMapDynamicGeometry(this)`,
/// `ClassVFXRenderDataTable_Insert(this, a, b, geometry)`, and the
/// render data's far corners (-10000, 10000).
pub const VFX_RENDER_DATA_TABLE: Global<u32> = Global::new(0x00cc_ca6c);
pub const GET_DYNAMIC_GEOMETRY: u32 = 0x0069_f620;
pub const GUILD_MAP_GEOMETRY_CTOR: u32 = 0x0069_e2b0;
pub const VFX_TABLE_INSERT: u32 = 0x0069_f890;
pub const FAR_LOW: Global<f32> = Global::new(0x00b4_1db0);
pub const FAR_HIGH: Global<f32> = Global::new(0x00b1_0f4c);

/// The cloth vertex buffers' heap: `mspace_free(mspace, p)` for pointers
/// in its megabyte at `CLOTH_BASE`, `user_free_thunk(p)` for the others;
/// the cloth instance table (count, then the instances).
pub const MSPACE_FREE: u32 = 0x006e_bee0;
pub const USER_FREE: u32 = 0x0068_7010;
pub const CLOTH_MSPACE: Global<u32> = Global::new(0x00cc_b454);
pub const CLOTH_BASE: Global<u32> = Global::new(0x00cc_b458);
pub const CLOTH_INSTANCE_COUNT: Global<u32> = Global::new(0x00cc_b250);
pub const CLOTH_INSTANCES: u32 = 0x00cc_b254;

/// `Phyre_PRendering_PEffectVariant_createMaterialWithPreallocatedBuffer(this, material, buffer)`
/// and `Phyre_PRendering_PEffectVariant_createMaterial(this, cluster, a, b)`.
pub const CREATE_MATERIAL_PREALLOCATED: u32 = 0x004e_6b40;
pub const CREATE_MATERIAL: u32 = 0x004e_6a40;

/// `PMatrix4_Copy_004c9430(this, m)`, `sprintf`'s import slot and `"%d"`.
pub const MATRIX_COPY: u32 = 0x004c_9430;
pub const SPRINTF_SLOT: u32 = crate::glue9::SPRINTF;
pub const FORMAT_D: u32 = 0x00b0_f604;

/// The field-map load's callees and strings.
pub const CHR_F117_HIDE_COUNT: Global<u32> = Global::new(0x012f_fad4);
pub const LEVEL_ANALYSIS_NAME: u32 = 0x0064_1d00;
pub const CURRENT_LEVEL_NAME: u32 = 0x0065_de70;
pub const VIEWER_MANAGER_INSTANCE: u32 = 0x006b_6370;
pub const FORMAT_S: u32 = 0x00b0_e160;
pub const PRINTF_SLOT: u32 = 0x00b0_c380;
pub const LEVEL_NAME_MESSAGE: u32 = 0x00b4_165c;
pub const FILE_NAME_MESSAGE: u32 = 0x00b4_1688;
pub const TEXTURE_VIDEO_LOAD: u32 = 0x0066_fb70;
pub const STRSTR_SLOT: u32 = 0x00b0_c378;
pub const HIKU21: u32 = 0x00b4_16b0;
pub const GET_LANGUAGE: u32 = 0x008a_c2f0;
/// The language switch's case bytes (for languages 1..18) and, per case,
/// the six bytes copied over "hiku21" (the jump table's targets, in
/// case order; case 9 copies nothing).
pub const LANGUAGE_CASES: u32 = 0x0064_0394;
pub const HIKU_NAMES: [u32; 9] = [
    0x00b4_16b8,
    0x00b4_16d8,
    0x00b4_16c0,
    0x00b4_16c8,
    0x00b4_16d0,
    0x00b4_16e8,
    0x00b4_16f0,
    0x00b4_16f8,
    0x00b4_16e0,
];
pub const FIELD_MAP_LOAD_ASYNC: u32 = 0x0065_ce90;

/// The main menu's water: `fiosUnifyFilename(src, out, size)`, the
/// directory and file-name format, the frame index and counter bytes,
/// the frames' texture names, the UI element's corner (512, 175) and
/// `graphicDrawUIElement(element, texture, 1, 0, 0)`.
pub const UNIFY_FILENAME: u32 = 0x0067_9820;
pub const WATER_DIRECTORY: u32 = 0x00b4_3bdc;
pub const WATER_FORMAT: u32 = 0x00b4_3c08;
pub const WATER_FRAME: Global<u8> = Global::new(0x00cc_ba09);
pub const WATER_COUNTER: Global<u8> = Global::new(0x00cc_ba0a);
pub const WATER_TEXTURES: u32 = 0x00c5_9c40;
pub const WATER_X: Global<f32> = Global::new(0x00b4_1d9c);
pub const WATER_Y: Global<f32> = Global::new(0x00b4_3dc0);
pub const DRAW_UI_ELEMENT: u32 = 0x0063_eee0;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87::round_trip;
    use ffx_graphic::host::{Conv, Host};

    /// The function in the import slot `slot`.
    fn import_at(h: &impl Host, slot: u32) -> u32 {
        h.read(h.rebase(slot))
    }

    /// `RenderDelegationHelper_registerDelegation(this, data, size, task,
    /// arg)` (`ret 0x10`; frame: the delegation at `ebp - 4`, which the
    /// list's grow sees): a free-list entry claimed by compare-exchange
    /// (64 flags `+0xc`, entries `+8`), else a new 20-byte one (0 when
    /// the allocation fails); its buffer (`+4`, capacity `+8`) grown
    /// for `size`, cleared and filled with `data` (`+0` points at it),
    /// or with no data `+0` = 0 and the buffer cleared; `+0xc` = `task`,
    /// `+0x10` = `arg`. Under the mutex it takes the list's first null
    /// slot (count `+0`, the slots `+4`), else the list grows by one.
    /// `eax`: the delegation.
    #[allow(clippy::too_many_arguments)] // the original's four and the frame
    pub fn delegation_register(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        data: u32,
        size: u32,
        task: u32,
        arg: u32,
    ) -> u32 {
        let cas = import_at(h, INTERLOCKED_CAS);
        let mut d = 0u32;
        for i in 0..0x40u32 {
            if h.call(cas, Conv::Stdcall, &[this + 0xc + 8 * i, 1, 0]) == 0 {
                d = h.read(this + 8 + 8 * i);
                break;
            }
        }
        h.write(ebp - 4, d);
        if d == 0 {
            d = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x14]);
            if d != 0 {
                for off in [0, 4, 8] {
                    h.write(d + off, 0u32);
                }
            }
            h.write(ebp - 4, d);
        }
        if data == 0 {
            h.write(d, 0u32);
            let buffer: u32 = h.read(d + 4);
            if buffer != 0 {
                let n: u32 = h.read(d + 8);
                h.call_sym(MEMSET, Conv::Cdecl, &[buffer, 0, n]);
            }
        } else {
            if h.read::<u32>(d + 8) < size {
                let old: u32 = h.read(d + 4);
                if old != 0 {
                    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[old]);
                }
                let buffer = h.call_sym(ARRAY_NEW, Conv::Cdecl, &[size]);
                h.write(d + 4, buffer);
                h.write(d + 8, size);
            }
            let buffer: u32 = h.read(d + 4);
            h.write(d, buffer);
            let n: u32 = h.read(d + 8);
            h.call_sym(MEMSET, Conv::Cdecl, &[buffer, 0, n]);
            let to: u32 = h.read(d);
            h.call_sym(MEMCPY, Conv::Cdecl, &[to, data, size]);
        }
        h.write(d + 0xc, task);
        h.write(d + 0x10, arg);
        let mutex = DELEGATION_MUTEX.get(h);
        let wait = import_at(h, WAIT_FOR_SINGLE_OBJECT);
        h.call(wait, Conv::Stdcall, &[mutex, u32::MAX]);
        let n = h.read::<u32>(this) & 0x7fff_ffff;
        let slots: u32 = h.read(this + 4);
        match (0..n).find(|&i| h.read::<u32>(slots.wrapping_add(4 * i)) == 0) {
            Some(i) => h.write(slots.wrapping_add(4 * i), d),
            None => {
                let storage = h.call_sym(
                    PHYRE_MALLOC_ALIGNED,
                    Conv::Cdecl,
                    &[n.wrapping_mul(4).wrapping_add(4), 4],
                );
                h.call_sym(
                    DELEGATION_GROW_PUSH,
                    Conv::Thiscall(this),
                    &[storage, ebp - 4],
                );
            }
        }
        let mutex = DELEGATION_MUTEX.get(h);
        let release = import_at(h, RELEASE_MUTEX);
        h.call(release, Conv::Stdcall, &[mutex]);
        d
    }

    /// `graphicGuildmapCreate(a, b, arg)`: for an instance `(a, b)` (not
    /// both 0) with no dynamic geometry yet, a 0xd0-byte render data (an
    /// identity 4x4 at `+0` and `+0x40`, the far corners at `+0x98` and
    /// `+0xa4`, kind 2 `+0xb8`) and a guild-map geometry (`+0x24` the
    /// data, `+0x3c` = `arg`) inserted into the table. `eax`: 0 for the
    /// null instance, the geometry found, or the insert's.
    pub fn guildmap_create(h: &mut impl Host, a: u32, b: u32, arg: u32) -> u32 {
        if a | b == 0 {
            return 0;
        }
        let table = VFX_RENDER_DATA_TABLE.get(h);
        let found = h.call_sym(GET_DYNAMIC_GEOMETRY, Conv::Thiscall(table), &[a, b]);
        if found != 0 {
            return found;
        }
        let data = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0xd0]);
        h.write(data + 0xb8, 2u32);
        h.write(data + 0xbc, 0u32);
        for m in [0, 0x40] {
            for k in 0..16 {
                let one = k % 5 == 0;
                h.write(data + m + 4 * k, if one { 1f32 } else { 0f32 });
            }
        }
        let low = round_trip(FAR_LOW.get(h));
        let high = round_trip(FAR_HIGH.get(h));
        for k in 0..3 {
            h.write(data + 0x98 + 4 * k, low);
            h.write(data + 0xa4 + 4 * k, high);
        }
        h.write(data + 0x80, 0u32);
        h.write(data + 0x84, 0u32);
        h.write(data + 0xc0, 0u8);
        h.write(data + 0xc4, 1u16);
        h.write(data + 0xb6, 0u16);
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x40]);
        let geometry = if p == 0 {
            0
        } else {
            h.call_sym(GUILD_MAP_GEOMETRY_CTOR, Conv::Thiscall(p), &[])
        };
        h.write(geometry.wrapping_add(0x24), data);
        h.write(geometry.wrapping_add(0x3c), arg);
        let table = VFX_RENDER_DATA_TABLE.get(h);
        h.call_sym(VFX_TABLE_INSERT, Conv::Thiscall(table), &[a, b, geometry])
    }

    /// Frees a cloth allocation: from its heap (the megabyte at
    /// `CLOTH_BASE`) or the general one.
    fn cloth_free(h: &mut impl Host, p: u32) -> u32 {
        let base = CLOTH_BASE.get(h);
        if p >= base && p < base.wrapping_add(0x10_0000) {
            let mspace = CLOTH_MSPACE.get(h);
            h.call_sym(MSPACE_FREE, Conv::Cdecl, &[mspace, p])
        } else {
            h.call_sym(USER_FREE, Conv::Cdecl, &[p])
        }
    }

    /// `graphicCharacterInstanceDestroyClothVertexBuffers(cloth)` (cdecl;
    /// `slot` is the address of its argument's slot, the loop counter):
    /// each of the `+0x14` vertex-buffer lists (`+0x30`, a count and the
    /// buffers) has its buffers' storage (`+0x4c`) and the buffers freed,
    /// then the list; the cloth leaves the instance table and is freed.
    /// `eax`: the last free's; 0 without a cloth (the original leaves its
    /// caller's `eax`, which no shim can return).
    pub fn cloth_vertex_buffers_destroy(h: &mut impl Host, slot: u32, cloth: u32) -> u32 {
        if cloth == 0 {
            return 0;
        }
        let mut i = 0u32;
        h.write(slot, i);
        while i < h.read::<u32>(cloth + 0x14) {
            let at = cloth + 0x30 + 4 * i;
            let list: u32 = h.read(at);
            if list != 0 {
                let mut j = 0u32;
                while j < h.read::<u32>(h.read::<u32>(at)) {
                    let list: u32 = h.read(at);
                    let buffer: u32 = h.read(list.wrapping_add(4 + 4 * j));
                    if buffer != 0 {
                        let storage: u32 = h.read(buffer + 0x4c);
                        cloth_free(h, storage);
                        let list: u32 = h.read(at);
                        let buffer: u32 = h.read(list.wrapping_add(4 + 4 * j));
                        cloth_free(h, buffer);
                    }
                    j += 1;
                }
                let list: u32 = h.read(at);
                cloth_free(h, list);
            }
            i += 1;
            h.write(slot, i);
        }
        let mut n = CLOTH_INSTANCE_COUNT.get(h);
        let table = h.rebase(CLOTH_INSTANCES);
        let mut k = 0u32;
        while k < n {
            if h.read::<u32>(table + 4 * k) == cloth {
                h.write(table + 4 * k, 0u32);
                n = CLOTH_INSTANCE_COUNT.get(h);
            }
            k += 1;
        }
        cloth_free(h, cloth)
    }

    /// `PhyreScene_createMaterial_00659710(this, variant, cluster, a, b)`
    /// (`ret 0x10`): in another cluster, `variant`'s own material; in the
    /// scene's (`+0x12d4`), a material in a free slot of the pool (4000;
    /// materials `+0xfbec`, 0x18 bytes; buffers `+0xfbe8`, 0xe0 bytes; use
    /// bytes `+0xfbf8`; counter `+0xfbfc`; `*a`, `*b` = 0 when given)
    /// unless the variant's buffer (`+0x30`) is over 0xe0 bytes or the
    /// pool is full: then one in the least used of the 64 clusters at
    /// `+0x1374` (0x50 bytes, use counts `+0x2774`), whose index goes to
    /// the material's `+0x14`.
    pub fn scene_create_material(
        h: &mut impl Host,
        this: u32,
        variant: u32,
        cluster: u32,
        a: u32,
        b: u32,
    ) -> u32 {
        if cluster != this + 0x12d4 {
            return h.call_sym(CREATE_MATERIAL, Conv::Thiscall(variant), &[cluster, a, b]);
        }
        let slot: i32 = h.read(this + 0xfbfc);
        if h.read::<u16>(variant + 0x30) <= 0xe0 && slot < 4000 {
            let slot = slot as u32;
            let buffers: u32 = h.read(this + 0xfbe8);
            let materials: u32 = h.read(this + 0xfbec);
            let buffer = slot.wrapping_mul(0xe0).wrapping_add(buffers);
            let material = materials.wrapping_add(slot.wrapping_mul(0x18));
            let made = h.call_sym(
                CREATE_MATERIAL_PREALLOCATED,
                Conv::Thiscall(variant),
                &[material, buffer],
            );
            let used: u32 = h.read(this + 0xfbf8);
            let slot: u32 = h.read(this + 0xfbfc);
            h.write(slot.wrapping_add(used), 1u8);
            let mut k = h.read::<i32>(this + 0xfbfc).wrapping_add(1);
            let used: u32 = h.read(this + 0xfbf8);
            while k < 4000 {
                if h.read::<u8>(used.wrapping_add(k as u32)) == 0 {
                    break;
                }
                k += 1;
            }
            h.write(this + 0xfbfc, k.min(4000));
            for out in [a, b] {
                if out != 0 {
                    h.write(out, 0u32);
                }
            }
            return made;
        }
        let mut least = 100_000i32;
        for k in 0..64 {
            least = least.min(h.read::<i32>(this + 0x2774 + 4 * k));
        }
        let at = (least as u32)
            .wrapping_mul(0x50)
            .wrapping_add(0x1374)
            .wrapping_add(this);
        let made = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(variant), &[at, a, b]);
        h.write(made + 0x14, least);
        let count = this
            .wrapping_add(0x2774)
            .wrapping_add((least as u32).wrapping_mul(4));
        let n = h.read::<u32>(count).wrapping_add(1);
        h.write(count, n);
        made
    }

    /// `PhyreScene_setEnableRenderWorldToTexture(this, on, m, f0..f7, n)`
    /// (`ret 0x2c`; frame: the matrix copy at `ebp - 0x40`): the 4x4 `m`
    /// (copied through the frame) to `+0xfc04`, the eight floats to
    /// `+0xfc44`, `n` to `+0xfc64` and as text to `+0xfc68`; the flag
    /// `+0xfc00` takes `on`, `+0xfc01` saying whether it changed to on
    /// (1 when it did not change). Returns 0.
    pub fn scene_set_render_world_to_texture(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        on: u32,
        m: u32,
        floats: [u32; 8],
        n: u32,
    ) -> u32 {
        let copy = ebp - 0x40;
        h.call_sym(MATRIX_COPY, Conv::Thiscall(copy), &[m]);
        for k in 0..16 {
            let v = round_trip(h.read(copy + 4 * k));
            h.write(this + 0xfc04 + 4 * k, v);
        }
        for (k, v) in (0u32..).zip(floats) {
            h.write(this + 0xfc44 + 4 * k, round_trip(f32::from_bits(v)));
        }
        h.write(this + 0xfc64, n);
        let format = h.rebase(FORMAT_D);
        let sprintf = import_at(h, SPRINTF_SLOT);
        h.call(sprintf, Conv::Cdecl, &[this + 0xfc68, format, n]);
        let on = on.to_le_bytes()[0];
        if on == h.read::<u8>(this + 0xfc00) {
            h.write(this + 0xfc01, 1u8);
        } else {
            h.write(this + 0xfc01, u8::from(on != 0));
            h.write(this + 0xfc00, on);
        }
        0
    }

    /// `graphicFieldMapLoad(path, flag)`: the hide count cleared, the
    /// level name analysed and copied to the viewer manager (`+0x51`),
    /// the texture video loaded, and (with two messages) a "hiku21" in
    /// `path` renamed for the language; then the scene loads the map
    /// (asynchronously, `flag != 0`). `eax`: that load's.
    pub fn field_map_load(h: &mut impl Host, path: u32, flag: u32) -> u32 {
        CHR_F117_HIDE_COUNT.set(h, 0);
        h.call_sym(LEVEL_ANALYSIS_NAME, Conv::Cdecl, &[path]);
        let scene = SCENE.get(h);
        let name = h.call_sym(CURRENT_LEVEL_NAME, Conv::Thiscall(scene), &[]);
        let format = h.rebase(FORMAT_S);
        let viewer = h.call_sym(VIEWER_MANAGER_INSTANCE, Conv::Cdecl, &[]);
        let sprintf = import_at(h, SPRINTF_SLOT);
        h.call(
            sprintf,
            Conv::Cdecl,
            &[viewer.wrapping_add(0x51), format, name],
        );
        let scene = SCENE.get(h);
        let name = h.call_sym(CURRENT_LEVEL_NAME, Conv::Thiscall(scene), &[]);
        let printf = import_at(h, PRINTF_SLOT);
        let message = h.rebase(LEVEL_NAME_MESSAGE);
        h.call(printf, Conv::Cdecl, &[message, name]);
        let scene = SCENE.get(h);
        h.call_sym(TEXTURE_VIDEO_LOAD, Conv::Thiscall(scene), &[path]);
        let message = h.rebase(FILE_NAME_MESSAGE);
        h.call(printf, Conv::Cdecl, &[message, path]);
        let strstr = import_at(h, STRSTR_SLOT);
        let hiku = h.rebase(HIKU21);
        if h.call(strstr, Conv::Cdecl, &[path, hiku]) != 0 {
            let language = h.call_sym(GET_LANGUAGE, Conv::Cdecl, &[]);
            let strstr = import_at(h, STRSTR_SLOT);
            let at = h.call(strstr, Conv::Cdecl, &[path, hiku]);
            let case = language.wrapping_sub(1);
            if case <= 0x11 {
                let k: u8 = h.read(h.rebase(LANGUAGE_CASES) + case);
                if let Some(&name) = HIKU_NAMES.get(usize::from(k)) {
                    let name = h.rebase(name);
                    let head: u32 = h.read(name);
                    h.write(at, head);
                    let tail: u16 = h.read(name + 4);
                    h.write(at.wrapping_add(4), tail);
                }
            }
        }
        let scene = SCENE.get(h);
        h.call_sym(
            FIELD_MAP_LOAD_ASYNC,
            Conv::Thiscall(scene),
            &[path, u32::from(flag != 0)],
        )
    }

    /// `PhyreScene_drawMainMenuWaveEffect()` (frame: the UI element at
    /// `ebp - 0x29c`, the file name at `ebp - 0x204`, the directory at
    /// `ebp - 0x104`; words at `ebp - 0x214`, `- 0x20c`, `- 0x208`
    /// cleared): the current water frame (`WATER_FRAME`, its name built
    /// though unused) is drawn as a full white element at (512, 175)
    /// with alpha 100; the counter steps and the frame wraps at 70.
    /// `eax`: the draw's upper bytes over the new frame.
    pub fn scene_draw_main_menu_wave(h: &mut impl Host, ebp: u32) -> u32 {
        h.write(ebp - 0x20c, 0u32);
        h.write(ebp - 0x208, 0u32);
        let directory = h.rebase(WATER_DIRECTORY);
        h.call_sym(
            UNIFY_FILENAME,
            Conv::Cdecl,
            &[directory, ebp - 0x104, 0x100],
        );
        let frame = u32::from(WATER_FRAME.get(h));
        let format = h.rebase(WATER_FORMAT);
        let sprintf = import_at(h, SPRINTF_SLOT);
        h.call(
            sprintf,
            Conv::Cdecl,
            &[ebp - 0x204, format, ebp - 0x104, frame],
        );
        let frame = u32::from(WATER_FRAME.get(h));
        let element = ebp - 0x29c;
        for k in 0..4 {
            h.write(element + 4 * k, 0f32);
        }
        h.write(element + 0x20, round_trip(WATER_X.get(h)));
        h.write(ebp - 0x214, 0u32);
        h.write(element + 0x24, round_trip(WATER_Y.get(h)));
        for (off, v) in [
            (0x10, 0xffu32),
            (0x14, 0xff),
            (0x18, 0xff),
            (0x1c, 100),
            (0x30, 0xff),
            (0x34, 0xff),
            (0x38, 0xff),
            (0x3c, 100),
        ] {
            h.write(element + off, v);
        }
        h.write(element + 0x28, 1f32);
        h.write(element + 0x2c, 1f32);
        let texture: u32 = h.read(h.rebase(WATER_TEXTURES) + 4 * frame);
        let drawn = h.call_sym(DRAW_UI_ELEMENT, Conv::Cdecl, &[element, texture, 1, 0, 0]);
        let counter = WATER_COUNTER.get(h).wrapping_add(1);
        WATER_COUNTER.set(h, counter);
        let next = WATER_FRAME.get(h).wrapping_add(1);
        let next = if next < 0x46 { next } else { 0 };
        WATER_FRAME.set(h, next);
        (drawn & 0xffff_ff00) | u32::from(next)
    }
}
