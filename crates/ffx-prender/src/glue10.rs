//! The render-side glue, tenth batch (M4 port round 3, list
//! `h3-glue-render`): the scene's ten shader-bin loaders (one shape, a
//! file name and a scene slot each), the character instantiation queue's
//! removal, VFX set-up, the render delegation pass, the shadow textures'
//! sizes, the VFX dynamic geometry constructor and delete pool, the
//! element-array grows and the per-buffer draw data reset.
//!
//! The loaders and the array grows set up a C++ exception frame
//! (`fs:[0]`) and a security cookie; nothing they call throws on these
//! paths, so the ports leave both out ([`crate::glue6`]).

use ffx_bridge::Global;

/// `fiosUnifyFilename(name, out, size)` (cdecl).
pub const FIOS_UNIFY_FILENAME: u32 = crate::glue8::FIOS_UNIFY_FILENAME;
/// `Phyre_PSerialization_PStreamReaderFile` methods: the constructor
/// (`this, path`), `isOpen` (`al`), `close` and the destructor.
pub const READER_CTOR: u32 = 0x0057_66d0;
pub const READER_IS_OPEN: u32 = 0x0057_6980;
pub const READER_CLOSE: u32 = 0x0057_6930;
pub const READER_DTOR: u32 = 0x0057_67c0;
/// `ClusterManager_GetInstance()` and `loadPCluster(this, path)`.
pub const CLUSTER_MANAGER_INSTANCE: u32 = 0x0069_a1a0;
pub const CLUSTER_MANAGER_LOAD: u32 = 0x0069_b940;
/// `Phyre_PFramework_PApplication_GetApplication`.
pub const GET_APPLICATION: u32 = crate::glue::GET_APPLICATION;
/// `PApplication_FixupClusters(&cluster, n)` (cdecl).
pub const FIXUP_CLUSTERS: u32 = crate::glue5::FIXUP_CLUSTERS;

/// One of the scene's shader-bin loaders: its file name (`.rdata`) and
/// the scene word its cluster goes into.
#[derive(Debug, Clone, Copy)]
pub struct ShaderBin {
    pub name: u32,
    pub slot: u32,
}

/// `PhyreScene_loadAbConeShader` `0x00663d40`.
pub const AB_CONE: ShaderBin = ShaderBin {
    name: 0x00b4_2708,
    slot: 0xfa70,
};
/// `PhyreScene_loadAbWaveShader` `0x00663e20`.
pub const AB_WAVE: ShaderBin = ShaderBin {
    name: 0x00b4_2750,
    slot: 0xfa74,
};
/// `PhyreScene_loadChrShaShader` `0x00664360`.
pub const CHR_SHA: ShaderBin = ShaderBin {
    name: 0x00b4_2950,
    slot: 0xfa7c,
};
/// `PhyreScene_loadMenuNoTextureShader` `0x00665010`.
pub const MENU_NO_TEXTURE: ShaderBin = ShaderBin {
    name: 0x00b4_29d0,
    slot: 0xfa80,
};
/// `PhyreScene_loadMenuShader` `0x006650f0`.
pub const MENU: ShaderBin = ShaderBin {
    name: 0x00b4_2830,
    slot: 0xfa84,
};
/// `PhyreScene_loadNAMenuShader` `0x00665200`.
pub const NA_MENU: ShaderBin = ShaderBin {
    name: 0x00b4_2878,
    slot: 0xfa88,
};
/// `PhyreScene_loadNBMenuShader` `0x006652e0`.
pub const NB_MENU: ShaderBin = ShaderBin {
    name: 0x00b4_28c0,
    slot: 0xfa8c,
};
/// `PhyreScene_loadNTMenuZTestShader` `0x006653c0`.
pub const NT_MENU_ZTEST: ShaderBin = ShaderBin {
    name: 0x00b4_2798,
    slot: 0xfa78,
};
/// `PhyreScene_loadNTNBMenuShader` `0x006654a0`.
pub const NTNB_MENU: ShaderBin = ShaderBin {
    name: 0x00b4_2908,
    slot: 0xfa90,
};
/// `PhyreScene_loadOptimizedFontShader` `0x00665580`.
pub const OPTIMIZED_FONT: ShaderBin = ShaderBin {
    name: 0x00b4_27e8,
    slot: 0xfa94,
};

/// `g_GraphicCharacterLoading`: count (top bit: borrowed storage), data.
pub const CHARACTER_LOADING: Global<u32> = Global::new(0x00cc_b6d4);
pub const CHARACTER_LOADING_DATA: Global<u32> = Global::new(0x00cc_b6d8);
/// `operator_delete(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `Phyre_PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = crate::glue8::PHYRE_FREE;
/// `Phyre_PhyreMalloc_0042fb80(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = crate::glue8::PHYRE_MALLOC;
/// `g_VFXRenderDataTable` and its instance lookup.
pub const VFX_RENDER_DATA_TABLE: Global<u32> = crate::glue4::VFX_RENDER_DATA_TABLE;
pub const VFX_GET_DYN_GEO: u32 = crate::glue8::VFX_GET_DYN_GEO;
/// `Phyre_PhyrePrintf(level, format)` (cdecl) and `graphicVFXSetup`'s
/// two messages.
pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
pub const VFX_SETUP_NO_GEOMETRY: u32 = 0x00b4_1a2c;
pub const VFX_SETUP_NO_INSTANCE: u32 = 0x00b4_1a6c;
/// The imports: `WaitForSingleObject`, `ReleaseMutex` (stdcall),
/// `__iob_func`, `fprintf` (cdecl).
pub const WAIT_FOR_SINGLE_OBJECT: u32 = crate::glue::WAIT_FOR_SINGLE_OBJECT;
pub const RELEASE_MUTEX: u32 = crate::glue::RELEASE_MUTEX;
pub const IOB_FUNC: u32 = crate::glue4::IOB_FUNC;
pub const FPRINTF: u32 = crate::glue4::FPRINTF;
/// The message for a missing cluster.
pub const MISSING_CLUSTER_MESSAGE: u32 = 0x00b4_24d0;
/// `Phyre_PWorld_addCluster(this, cluster)`.
pub const WORLD_ADD_CLUSTER: u32 = 0x0044_3250;
/// `Array_Reserve_00668300(this, n)`.
pub const ARRAY_RESERVE_668300: u32 = crate::glue6::ARRAY_RESERVE_668300;
/// The render delegation helper's mutex.
pub const DELEGATION_MUTEX: Global<u32> = Global::new(0x00cc_caa0);
/// `RenderDelegationHelper_setDataToFreeList(this, data)` and the
/// array compaction `0x006ad590(this)`.
pub const DELEGATION_SET_TO_FREE_LIST: u32 = 0x006a_e1a0;
pub const DELEGATION_COMPACT: u32 = 0x006a_d590;
/// The shadow textures' set-up: `0x0059c650(this)`,
/// `Phyre_PRendering_PTexture2DBase_setDimensions(this, w, h, format,
/// 0)`, `g_ShadowSize` and the texture format (`.data`).
pub const SHADOW_TEXTURE_PREPARE: u32 = 0x0059_c650;
pub const TEXTURE_SET_DIMENSIONS: u32 = 0x004c_40c0;
pub const SHADOW_SIZE: Global<i32> = crate::glue2::SHADOW_SIZE;
pub const SHADOW_TEXTURE_FORMAT: u32 = 0x00ca_310c;
/// `VFXDynamicGeometry_vftable` and `g_ChrClothFlip` (float).
pub const VFX_DYN_GEO_VTABLE: u32 = 0x00b4_7e40;
pub const CHR_CLOTH_FLIP: Global<u32> = Global::new(0x00b9_2340);
/// The map VFX texture's names (as in `loadMapVFXTexture`) and
/// `PhyreScene_unloadVFXTexture(this, name, -1)`.
pub const MAP_VFX_NAME: u32 = crate::glue9::MAP_VFX_NAME;
pub const MAP_VFX_FORMAT: u32 = crate::glue9::MAP_VFX_FORMAT;
pub const SPRINTF: u32 = crate::glue9::SPRINTF;
pub const SCENE_UNLOAD_VFX_TEXTURE: u32 = 0x0067_1300;
/// The delete pool's array appends (`this; dst, &value`): instance ids
/// (8 bytes), and two word arrays.
pub const PUSH_INTO_6A2790: u32 = 0x006a_2790;
pub const PUSH_INTO_6A25E0: u32 = 0x006a_25e0;
pub const PUSH_INTO_6A2670: u32 = 0x006a_2670;
/// The element-array grows' helpers: the copy into the new buffer
/// (cdecl `(dst, src, n)`), the element constructor from a value
/// (`this, &value`) and the old storage's destroy (cdecl `(p, n)`).
#[derive(Debug, Clone, Copy)]
pub struct ElementArray {
    pub size: u32,
    pub copy: u32,
    pub construct: u32,
    pub destroy: u32,
}

/// `PArray_pushInto32_006b2730`.
pub const ELEMENTS_32: ElementArray = ElementArray {
    size: 32,
    copy: 0x006a_e9c0,
    construct: 0x006a_f0d0,
    destroy: 0x006b_01f0,
};
/// `PArray_pushInto64_006b2800`.
pub const ELEMENTS_64: ElementArray = ElementArray {
    size: 64,
    copy: 0x006a_eb20,
    construct: 0x006a_f270,
    destroy: 0x006b_0230,
};
/// `graphicDynGeoMemManagerGetCurrentBufferId()`.
pub const DYN_GEO_CURRENT_BUFFER_ID: u32 = 0x0063_ff50;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::glue::bridge::instance;
    use ffx_bridge::x87::round_trip;
    use ffx_graphic::host::{Conv, Host};

    /// Calls the import whose slot is at `slot`; its `eax`.
    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let at = h.rebase(slot);
        let f: u32 = h.read(at);
        h.call(f, conv, args)
    }

    /// The scene's shader-bin loaders (`PhyreScene_loadAbConeShader` and
    /// the nine like it; frame: the unified path at `ebp - 0x110`, a
    /// stream reader at `ebp - 0x12c`): when the file opens, the cluster
    /// the cluster manager loads from it goes into the scene word
    /// `bin.slot` and is fixed up (its result); 5 when it doesn't.
    pub fn scene_load_shader(h: &mut impl Host, ebp: u32, this: u32, bin: ShaderBin) -> u32 {
        let path = ebp.wrapping_sub(0x110);
        let reader = ebp.wrapping_sub(0x12c);
        let name = h.rebase(bin.name);
        h.call_sym(FIOS_UNIFY_FILENAME, Conv::Cdecl, &[name, path, 0x100]);
        h.call_sym(READER_CTOR, Conv::Thiscall(reader), &[path]);
        let open = h.call_sym(READER_IS_OPEN, Conv::Thiscall(reader), &[]) as u8;
        let result = if open == 0 {
            5
        } else {
            h.call_sym(READER_CLOSE, Conv::Thiscall(reader), &[]);
            let slot = this.wrapping_add(bin.slot);
            // the path is pushed before the getter (which takes nothing)
            let manager = instance(h, CLUSTER_MANAGER_INSTANCE);
            let cluster = h.call_sym(CLUSTER_MANAGER_LOAD, Conv::Thiscall(manager), &[path]);
            h.write(slot, cluster);
            h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
            h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[slot, 1])
        };
        h.call_sym(READER_DTOR, Conv::Thiscall(reader), &[]);
        result
    }

    /// `graphicCharacterUnregistLoading_00637b30(chr)`: each pending
    /// instantiation request for `chr` (`+8`) is deleted and replaced by
    /// the last one, the count dropping by one (the storage freed once
    /// owned and empty); the index moves on, so a moved request is not
    /// checked again.
    pub fn character_unregist_loading(h: &mut impl Host, chr: u32) {
        let mut word = CHARACTER_LOADING.get(h);
        let mut i = 0u32;
        while i < word & 0x7fff_ffff {
            let data = CHARACTER_LOADING_DATA.get(h);
            let request: u32 = h.read(data.wrapping_add(4 * i));
            if h.read::<u32>(request.wrapping_add(8)) == chr {
                h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[request]);
                let count = CHARACTER_LOADING.get(h) & 0x7fff_ffff;
                let data = CHARACTER_LOADING_DATA.get(h);
                let last: u32 = h.read(data.wrapping_add(4 * count).wrapping_sub(4));
                h.write(data.wrapping_add(4 * i), last);
                let old = CHARACTER_LOADING.get(h);
                let count = old & 0x7fff_ffff;
                let fewer = count.saturating_sub(1);
                word = (old & 0x8000_0000) | fewer;
                CHARACTER_LOADING.set(h, word);
                if word >> 31 == 0 && word == 0 {
                    let data = CHARACTER_LOADING_DATA.get(h);
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
                    word = CHARACTER_LOADING.get(h);
                    CHARACTER_LOADING_DATA.set(h, 0);
                }
            }
            i += 1;
        }
    }

    /// `graphicVFXSetup(lo, hi)` `0x00645780`: each part `k` of the effect
    /// instance (count as in `graphicVFXDestroy`; instance `(lo, hi) +
    /// 10000 k`) gets virtual `+4 (0, 0)` on its geometry; 0 when one of
    /// them fails or the instance has no geometry or render data (with a
    /// message), else 1.
    pub fn vfx_setup(h: &mut impl Host, lo: u32, hi: u32) -> u32 {
        if lo | hi == 0 {
            let message = h.rebase(VFX_SETUP_NO_INSTANCE);
            h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[1, message]);
            return 0;
        }
        let table = VFX_RENDER_DATA_TABLE.get(h);
        let geo = h.call_sym(VFX_GET_DYN_GEO, Conv::Thiscall(table), &[lo, hi]);
        let data = if geo == 0 { 0 } else { h.read(geo + 0x24) };
        if data == 0 {
            let message = h.rebase(VFX_SETUP_NO_GEOMETRY);
            h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[1, message]);
            return 0;
        }
        let parts = if h.read::<u8>(data + 0xc0) == 0 {
            1
        } else {
            i32::from(h.read::<u16>(data + 0xc4))
        };
        let base = (u64::from(hi) << 32) | u64::from(lo);
        let mut k = 0i32;
        for _ in 0..parts {
            let id = base.wrapping_add(i64::from(k).cast_unsigned());
            let (id_lo, id_hi) = (id as u32, (id >> 32) as u32);
            let table = VFX_RENDER_DATA_TABLE.get(h);
            let part = h.call_sym(VFX_GET_DYN_GEO, Conv::Thiscall(table), &[id_lo, id_hi]);
            if part != 0 && h.call_virtual(part, 1, &[0, 0]) != 0 {
                return 0;
            }
            k = k.wrapping_add(10000);
        }
        1
    }

    /// `PhyreScene_detachChrClusterFrom3DWorld(this)` `0x006566c0`: under
    /// the scene's mutex (`+0xfb3c`), each queued `(world, list)` pair
    /// (`+0x1188`, data `+0x1190`) adds the list's clusters to the world
    /// (a missing one is reported on stderr); the queue is emptied
    /// (released when count and capacity are negative).
    pub fn scene_detach_chr_cluster(h: &mut impl Host, this: u32) {
        let mutex: u32 = h.read(this + 0xfb3c);
        import(h, WAIT_FOR_SINGLE_OBJECT, Conv::Stdcall, &[mutex, u32::MAX]);
        let queue = this + 0x1188;
        let mut i = 0u32;
        while i < h.read::<u32>(queue) {
            let pairs: u32 = h.read(this + 0x1190);
            let list: u32 = h.read(pairs.wrapping_add(8 * i).wrapping_add(4));
            let world: u32 = h.read(pairs.wrapping_add(8 * i));
            let mut j = 0u32;
            while j < h.read::<u32>(list) {
                let clusters: u32 = h.read(list.wrapping_add(8));
                let cluster: u32 = h.read(clusters.wrapping_add(4 * j));
                if cluster == 0 {
                    // the message is pushed before the stream getter
                    let message = h.rebase(MISSING_CLUSTER_MESSAGE);
                    let iob = import(h, IOB_FUNC, Conv::Cdecl, &[]);
                    import(h, FPRINTF, Conv::Cdecl, &[iob.wrapping_add(0x40), message]);
                } else {
                    h.call_sym(WORLD_ADD_CLUSTER, Conv::Thiscall(world), &[cluster]);
                }
                j += 1;
            }
            i += 1;
        }
        if h.read::<i32>(queue) < 0 && h.read::<i32>(queue + 4) < 0 {
            h.call_sym(ARRAY_RESERVE_668300, Conv::Thiscall(queue), &[0]);
        }
        h.write(queue, 0u32);
        let mutex: u32 = h.read(this + 0xfb3c);
        import(h, RELEASE_MUTEX, Conv::Stdcall, &[mutex]);
    }

    /// `RenderDelegationHelper_beforeVFX(this)` `0x006adb80`: under the
    /// helper's mutex, each delegated entry (`[+4]`, count `+0`) flagged
    /// 0x2000 runs its callback (`+0x10`, cdecl, with `+0`); a one-shot
    /// entry (flag 1) then goes back to the free list (or, when that
    /// fails, is deleted with its buffer `+4`), its slot cleared and the
    /// array compacted, the index staying put.
    pub fn delegation_before_vfx(h: &mut impl Host, this: u32) {
        let mutex = DELEGATION_MUTEX.get(h);
        import(h, WAIT_FOR_SINGLE_OBJECT, Conv::Stdcall, &[mutex, u32::MAX]);
        let mut i = 0u32;
        while i < h.read::<u32>(this) & 0x7fff_ffff {
            let entry = entry_at(h, this, i);
            if entry != 0 && h.read::<u32>(entry + 0xc) & 0x2000 != 0 {
                let arg: u32 = h.read(entry);
                let callback: u32 = h.read(entry + 0x10);
                h.call(callback, Conv::Cdecl, &[arg]);
                let entry = entry_at(h, this, i);
                if h.read::<u8>(entry.wrapping_add(0xc)) & 1 != 0 {
                    let r = h.call_sym(DELEGATION_SET_TO_FREE_LIST, Conv::Thiscall(this), &[entry]);
                    if (r as i32) < 0 {
                        let dead = entry_at(h, this, i);
                        if dead != 0 {
                            let buffer: u32 = h.read(dead + 4);
                            if buffer != 0 {
                                h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[buffer]);
                            }
                            h.write(dead + 8, 0u32);
                            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[dead]);
                        }
                    }
                    let entries: u32 = h.read(this + 4);
                    h.write(entries.wrapping_add(4 * i), 0u32);
                    i = i.wrapping_sub(1);
                    h.call_sym(DELEGATION_COMPACT, Conv::Thiscall(this), &[]);
                }
            }
            i = i.wrapping_add(1);
        }
        let mutex = DELEGATION_MUTEX.get(h);
        import(h, RELEASE_MUTEX, Conv::Stdcall, &[mutex]);
    }

    fn entry_at(h: &mut impl Host, this: u32, i: u32) -> u32 {
        let entries: u32 = h.read(this + 4);
        h.read(entries.wrapping_add(4 * i))
    }

    /// `ShadowManager_resizeShadowTextures_006d2df0(this)`: 1 unless all
    /// four shadow textures exist (`+0x74`, `+0x78`, `+0x80`, `+0x7c`);
    /// else each is prepared (`+0x6c`, `+0x70`, `+0x60`, `+0x64`) and
    /// resized to `g_ShadowSize` squared, stopping at the first failure
    /// (its result; the last one's otherwise).
    pub fn shadow_resize_textures(h: &mut impl Host, this: u32) -> u32 {
        for off in [0x74, 0x78, 0x80, 0x7c] {
            if h.read::<u32>(this + off) == 0 {
                return 1;
            }
        }
        let mut r = 0;
        for (source, texture) in [(0x6c, 0x74), (0x70, 0x78), (0x60, 0x7c), (0x64, 0x80)] {
            let s: u32 = h.read(this + source);
            h.call_sym(SHADOW_TEXTURE_PREPARE, Conv::Thiscall(s), &[]);
            let size = SHADOW_SIZE.get(h) as u32;
            let t: u32 = h.read(this + texture);
            let format = h.rebase(SHADOW_TEXTURE_FORMAT);
            r = h.call_sym(
                TEXTURE_SET_DIMENSIONS,
                Conv::Thiscall(t),
                &[size, size, format, 0],
            );
            if r != 0 {
                return r;
            }
        }
        r
    }

    /// `VFXDynamicGeometry_VFXDynamicGeometry(this)` `0x0069e3e0`: the
    /// vtable, cleared fields, `+0x3c` = -1, `+0xdc` = `g_ChrClothFlip`,
    /// the four floats at `+0xe8` = 1. Returns `this`.
    pub fn vfx_dyn_geo_ctor(h: &mut impl Host, this: u32) -> u32 {
        let zero = 0f32.to_bits();
        h.write(this + 4, 0u32);
        h.write(this + 0x4c, zero);
        h.write(this + 8, 0u32);
        h.write(this + 0xd4, zero);
        h.write(this + 0xc, 0u32);
        let flip = round_trip(f32::from_bits(CHR_CLOTH_FLIP.get(h))).to_bits();
        h.write(this + 0x10, 0u32);
        h.write(this + 0xdc, flip);
        for off in [0x14, 0x18, 0x1c, 0x20, 0x24, 0x28, 0x2c] {
            h.write(this + off, 0u32);
        }
        h.write(this + 0x30, 0u8);
        let vtable = h.rebase(VFX_DYN_GEO_VTABLE);
        h.write(this, vtable);
        h.write(this + 0x3c, u32::MAX);
        h.write(this + 0x40, 0u8);
        h.write(this + 0x44, 0u32);
        h.write(this + 0x48, 0u32);
        h.write(this + 0x50, 0u8);
        h.write(this + 0x52, 0u8);
        h.write(this + 0xd8, 0u8);
        h.write(this + 0xe0, zero);
        h.write(this + 0xe4, zero);
        let one = 1f32.to_bits();
        for off in [0xe8, 0xec, 0xf0, 0xf4] {
            h.write(this + off, one);
        }
        this
    }

    /// `PhyreScene_unloadMapVFXTexture(this)` `0x00670fe0` (frame as
    /// `loadMapVFXTexture`'s): when requested (`+0x25`) and with a map,
    /// the request flag cleared and the map's texture unloaded (its
    /// `eax`); 0 when not requested, 5 without a map.
    pub fn scene_unload_map_vfx_texture(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let file = ebp.wrapping_sub(0x104);
        let name = ebp.wrapping_sub(0x204);
        let base = h.rebase(MAP_VFX_NAME);
        h.call_sym(FIOS_UNIFY_FILENAME, Conv::Cdecl, &[base, file, 0x100]);
        if h.read::<u8>(this + 0x25) == 0 {
            return 0;
        }
        if map_id(h, this) == 0 {
            return 5;
        }
        h.write(this + 0x25, 0u8);
        let map = map_id(h, this);
        let format = h.rebase(MAP_VFX_FORMAT);
        import(h, SPRINTF, Conv::Cdecl, &[name, format, file, map]);
        h.call_sym(
            SCENE_UNLOAD_VFX_TEXTURE,
            Conv::Thiscall(this),
            &[name, u32::MAX],
        )
    }

    /// The scene's map id (`+0xfaf0`, colour bit cleared).
    fn map_id(h: &mut impl Host, this: u32) -> u32 {
        let word: u32 = h.read(this + 0xfaf0);
        word.wrapping_sub(u32::from(h.read::<u8>(this + 0xfaf0)) & 1)
    }

    /// `ClassVFXRenderDataTable_InsertToDeletePool(this, lo, hi, a, b)`
    /// `0x0069f900` (`ret 0x10`; `args`: where its four argument words
    /// are, which the appends read by address): the first free slot (id
    /// 0) of the pool (`+8` ids, `+0x10`/`+0x18` words) takes them (`eax`
    /// `b`); with none, each array grows by one into a new buffer
    /// (`eax`: the last append's).
    pub fn vfx_insert_to_delete_pool(
        h: &mut impl Host,
        args: u32,
        this: u32,
        lo: u32,
        hi: u32,
        a: u32,
        b: u32,
    ) -> u32 {
        let n = h.read::<u32>(this + 8) & 0x7fff_ffff;
        let ids: u32 = h.read(this + 0xc);
        for k in 0..n {
            let at = ids.wrapping_add(8 * k);
            if h.read::<u32>(at) | h.read::<u32>(at + 4) == 0 {
                h.write(at, lo);
                h.write(at + 4, hi);
                let words: u32 = h.read(this + 0x14);
                h.write(words.wrapping_add(4 * k), a);
                let words: u32 = h.read(this + 0x1c);
                h.write(words.wrapping_add(4 * k), b);
                return b;
            }
        }
        let buffer = h.call_sym(
            PHYRE_MALLOC,
            Conv::Cdecl,
            &[n.wrapping_mul(8).wrapping_add(8), 8],
        );
        h.call_sym(PUSH_INTO_6A2790, Conv::Thiscall(this + 8), &[buffer, args]);
        let mut r = 0;
        for (array, push, value) in [
            (0x10, PUSH_INTO_6A25E0, args + 8),
            (0x18, PUSH_INTO_6A2670, args + 12),
        ] {
            let count = (h.read::<u32>(this + array) & 0x7fff_ffff).wrapping_add(1);
            let buffer = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[count << 2, 4]);
            r = h.call_sym(push, Conv::Thiscall(this + array), &[buffer, value]);
        }
        r
    }

    /// `PArray_pushInto32_006b2730(this, dst, &value)` and the 64-byte
    /// one `0x006b2800` (`ret 8`): 0xd for a null `dst`; else the `count`
    /// elements copied into `dst`, a new one constructed from `value`
    /// after them (at a non-null address), the old storage destroyed when
    /// owned and not `dst`, then `count + 1` (owned) and `dst`. 0.
    pub fn parray_push_into_elements(
        h: &mut impl Host,
        this: u32,
        dst: u32,
        value: u32,
        e: ElementArray,
    ) -> u32 {
        if dst == 0 {
            return 0xd;
        }
        let count = h.read::<u32>(this) & 0x7fff_ffff;
        let old: u32 = h.read(this + 4);
        h.call_sym(e.copy, Conv::Cdecl, &[dst, old, count]);
        let slot = count.wrapping_mul(e.size).wrapping_add(dst);
        if slot != 0 {
            h.call_sym(e.construct, Conv::Thiscall(slot), &[value]);
        }
        let old: u32 = h.read(this + 4);
        let word: u32 = h.read(this);
        if old != dst && word >> 31 == 0 {
            h.call_sym(e.destroy, Conv::Cdecl, &[old, word & 0x7fff_ffff]);
        }
        h.write(this, count.wrapping_add(1));
        h.write(this + 4, dst);
        0
    }

    /// `ClassVFXRenderDataTable_ClearVFXDrawData(this)` `0x0069f2b0`: for
    /// the dynamic-geometry buffer in use (none when negative), each
    /// effect in the table's map (in order) has its segments' draw word
    /// for that buffer (`+0x34 + 28 id` in each 0x6c-byte segment of its
    /// render data `+0x94`, count `+0x90`) cleared.
    pub fn vfx_clear_draw_data(h: &mut impl Host, this: u32) {
        let id = h.call_sym(DYN_GEO_CURRENT_BUFFER_ID, Conv::Cdecl, &[]);
        if (id as i32) < 0 {
            return;
        }
        let head: u32 = h.read(this);
        let mut node: u32 = h.read(head);
        if node == head {
            return;
        }
        loop {
            let value: u32 = h.read(node.wrapping_add(0x18));
            let data: u32 = h.read(value.wrapping_add(0x24));
            let segments: i32 = h.read(data.wrapping_add(0x90));
            let mut offset = 0u32;
            for _ in 0..segments.max(0) {
                let base: u32 = h.read(data.wrapping_add(0x94));
                let at = offset.wrapping_add(id.wrapping_mul(28)).wrapping_add(base);
                h.write(at.wrapping_add(0x34), 0u32);
                offset = offset.wrapping_add(0x6c);
            }
            node = successor(h, node);
            if node == h.read::<u32>(this) {
                break;
            }
        }
    }

    /// The next node in order of an MSVC `std::map` (`+0` left, `+4`
    /// parent, `+8` right, `+0xd` the head flag); the head stays put.
    fn successor(h: &mut impl Host, node: u32) -> u32 {
        if is_nil(h, node) {
            return node;
        }
        let right: u32 = h.read(node.wrapping_add(8));
        if !is_nil(h, right) {
            let mut n = right;
            loop {
                let left: u32 = h.read(n);
                if is_nil(h, left) {
                    return n;
                }
                n = left;
            }
        }
        let mut n = node;
        let mut parent: u32 = h.read(node.wrapping_add(4));
        while !is_nil(h, parent) && n == h.read::<u32>(parent.wrapping_add(8)) {
            n = parent;
            parent = h.read(parent.wrapping_add(4));
        }
        parent
    }

    /// Whether a map node is the head (`+0xd` set).
    fn is_nil(h: &mut impl Host, node: u32) -> bool {
        h.read::<u8>(node.wrapping_add(0xd)) != 0
    }
}
