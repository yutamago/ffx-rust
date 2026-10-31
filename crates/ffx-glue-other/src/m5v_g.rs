//! M5 stage 1, list `vfx`, round 5 batch 2: the VFX glue the earlier
//! rounds left: the blob shadow's mesh set-up, the dynamic geometry's
//! CLUT parameters and mesh-segment resize, the shader map copy, and the
//! particle programs' render-data buffers (rectangles, the guide map,
//! `Shp_OShapeVu1`, the mesh breaks).
//!
//! SEH frames and stack cookies are left out: nothing on these paths
//! throws. Where the original hands a callee frame buffers, the bridge
//! takes the original's `ebp` and keeps each buffer at its offset (the
//! shim passes a `StackFrame`).

use ffx_bridge::Global;

/// `g_PhyreScene`.
pub const SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// `g_VfxShapeEnlargeBufferFactor` and the VFX segment divisor.
pub const SHAPE_ENLARGE: Global<u32> = Global::new(0x00c3_a4d8);
pub const SEGMENT_DIVISOR: Global<u32> = Global::new(0x00c3_a484);

/// `"TextureSampler"`, `"TextureSamplerState"`.
pub const TEXTURE_SAMPLER_NAME: u32 = 0x00b3_017c;
pub const TEXTURE_SAMPLER_STATE_NAME: u32 = 0x00b4_7ec4;
/// The three CLUT parameter names.
pub const CLUT_NAMES: [u32; 3] = [0x00b4_7f7c, 0x00b4_7f84, 0x00b4_7f8c];
/// The free list's class descriptor (`PFreeList` ctor's last word).
pub const SHADER_MAP_LIST_DESC: u32 = 0x00b1_3bc8;
/// `sprintf` formats and texts: the default render-data name (format and
/// its argument), "no texture", the bp texture name, the error messages.
pub const NAME_FORMAT: u32 = 0x00b0_e160;
pub const NAME_DEFAULT: u32 = 0x00b4_ff8c;
pub const NO_TEXTURE: u32 = 0x00b4_1b50;
pub const BP_NAME_FORMAT: u32 = 0x00b5_0114;
pub const BP_MISSING_FORMAT: u32 = 0x00b4_ff04;
pub const EMPTY_BUFFER_TEXT: u32 = 0x00b5_0030;
pub const TEXTURE_MISSING_FORMAT: u32 = 0x00b4_ff58;
/// `createU16MeshBreakRenderDataBuffer`'s strings: the folder (31 bytes),
/// the extension (4 + 2 bytes), the name format, the per-letter path
/// formats (m, n, f, c, s, w) and the log format.
pub const BREAK_FOLDER: u32 = 0x00b5_8820;
pub const BREAK_EXTENSION: u32 = 0x00b4_4e14;
pub const BREAK_NAME_FORMAT: u32 = 0x00b5_8840;
pub const BREAK_PATH_FORMATS: [(u8, u32); 6] = [
    (b'm', 0x00b5_8850),
    (b'n', 0x00b5_8864),
    (b'f', 0x00b5_8878),
    (b'c', 0x00b5_888c),
    (b's', 0x00b5_88a0),
    (b'w', 0x00b5_88b4),
];
pub const BREAK_LOG_FORMAT: u32 = 0x00b5_88c8;
/// `MSVCR110` import slots: `sprintf`, `strstr`, `printf` (cdecl).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
pub const STRSTR_SLOT: u32 = 0x00b0_c378;
pub const PRINTF_SLOT: u32 = 0x00b0_c380;

/// The frames: `ebp` minus the lowest local of each original.
pub const RECTS_FRAME: u32 = 0x68b4;
pub const GUILD_MAP_FRAME: u32 = 0x1f7c;
pub const SHP_VU1_FRAME: u32 = 0x68bc;
pub const U16_BREAK_FRAME: u32 = 0x338;
pub const GT3_BREAK_FRAME: u32 = 0x320;

// ---- callees ----

/// `ShaderParams_FindByName(params; name)` (thiscall, `ret 4`).
pub const FIND_PARAM: u32 = 0x0056_cbf0;
/// `ShadowManager_getInstance()` (cdecl), `getTextureSampler(m)`,
/// `getShadowMap(m)` (thiscall).
pub const SHADOW_MANAGER: u32 = 0x006d_2390;
pub const SHADOW_SAMPLER: u32 = 0x006d_2520;
pub const SHADOW_MAP: u32 = 0x006d_2510;
/// `Phyre_PParameterBuffer_setValueDesc_0056e600(buffer; param, value)`
/// (thiscall, `ret 8`).
pub const SET_VALUE_DESC: u32 = 0x0056_e600;
/// `Render_SurfaceFormat_0066e4f0(buffer; param, record)` (thiscall,
/// `ret 8`).
pub const WRITE_VECTOR_PARAMETER: u32 = 0x0066_e4f0;
/// `PhyreScene_setMeshVisible(scene; mesh, visible)` (`ret 8`).
pub const SET_MESH_VISIBLE: u32 = 0x0066_d910;
/// `memcpy` / `crt_memset_thunk` (cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
pub const MEMSET: u32 = 0x0094_964c;
/// `Phyre_PhyreMalloc_0042fb80(n, align)`, `Phyre_PhyreFree(p)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `PMeshSegmentEntries_construct_00434430(data, n)` /
/// `PMeshSegmentEntries_destroy_00434210(data, n)` (cdecl).
pub const SEGMENTS_CONSTRUCT: u32 = 0x0043_4430;
pub const SEGMENTS_DESTROY: u32 = 0x0043_4210;
/// `Phyre_Ctor_0055dc40(b)`, `Phyre_WaitAndClear_0055dfa0(b)` (thiscall).
pub const BUFFER_CTOR: u32 = 0x0055_dc40;
pub const BUFFER_WAIT_AND_CLEAR: u32 = 0x0055_dfa0;
/// `Phyre_PGeometry_FormatSize_0055bc40(format)` (cdecl).
pub const FORMAT_SIZE: u32 = 0x0055_bc40;
/// `Phyre_PGeometry_PIndexDataBlockNVN_isBound(b)` (thiscall, `al`),
/// `PIndexBufferD3D11_release_005648c0(b)` (thiscall).
pub const INDEX_IS_BOUND: u32 = 0x0056_2db0;
pub const INDEX_RELEASE: u32 = 0x0056_48c0;
/// `Phyre_PFreeList_PFreeList(list; size, count, align, desc)` (`ret
/// 0x10`), `allocateNewBlock(list; n)` (`ret 4`), `deallocate(list;
/// item)` (`ret 4`).
pub const FREE_LIST_CTOR: u32 = 0x0049_c480;
pub const FREE_LIST_ALLOCATE_BLOCK: u32 = 0x0049_c8d0;
pub const FREE_LIST_DEALLOCATE: u32 = 0x0049_c980;
/// `Phyre_PRedBlackTree_countNodes(tree)`, `FUN_005751d0(tree; node)`
/// (the successor walk, `ret 4`), `postInsertFinalize(tree; node)`
/// (`ret 4`).
pub const TREE_COUNT_NODES: u32 = 0x0057_4860;
pub const TREE_NEXT: u32 = 0x0057_51d0;
pub const TREE_POST_INSERT: u32 = 0x0057_4980;
/// `VfxShaderNode_copy_006bf170(node; src)`, `FUN_006be9e0(tree; node)`
/// (the string-keyed insert: 0 when inserted) (thiscall, `ret 4`).
pub const SHADER_NODE_COPY: u32 = 0x006b_f170;
pub const SHADER_TREE_INSERT: u32 = 0x006b_e9e0;
/// `graphicVFXCreate(lo, hi, flags)`, `graphicVFXCreateDataBuffer(lo,
/// hi, n, counts, counts2, names, flags)`, `graphicGuildmapCreate(lo, hi,
/// flags)`, `graphicVFXIsTextureWithBpLoaded` (7 words),
/// `graphicVFXIsTextureLoaded` (6 words) (cdecl).
pub const VFX_CREATE: u32 = 0x0064_4de0;
pub const VFX_CREATE_DATA_BUFFER: u32 = 0x0064_5040;
pub const GUILDMAP_CREATE: u32 = 0x0064_1510;
pub const IS_TEXTURE_WITH_BP_LOADED: u32 = 0x0064_5450;
pub const IS_TEXTURE_LOADED: u32 = 0x0064_5400;
/// `calculateVFXSP_AlphaFunction(f)` (cdecl).
pub const ALPHA_FUNCTION: u32 = 0x0071_2c30;
/// `dbgPrintf(format, ...)` (cdecl).
pub const DBG_PRINTF: u32 = 0x0062_f500;
/// `Ch_GetName(chr)`, `fiosUnifyFilename(src, out, size)` (cdecl).
pub const CH_GET_NAME: u32 = 0x0082_6b90;
pub const UNIFY_FILENAME: u32 = 0x0067_9820;

pub mod bridge {
    use super::{
        ALPHA_FUNCTION, BP_MISSING_FORMAT, BP_NAME_FORMAT, BREAK_EXTENSION, BREAK_FOLDER,
        BREAK_LOG_FORMAT, BREAK_NAME_FORMAT, BREAK_PATH_FORMATS, BUFFER_CTOR,
        BUFFER_WAIT_AND_CLEAR, CH_GET_NAME, CLUT_NAMES, DBG_PRINTF, EMPTY_BUFFER_TEXT, FIND_PARAM,
        FORMAT_SIZE, FREE_LIST_ALLOCATE_BLOCK, FREE_LIST_CTOR, FREE_LIST_DEALLOCATE,
        GUILDMAP_CREATE, INDEX_IS_BOUND, INDEX_RELEASE, IS_TEXTURE_LOADED,
        IS_TEXTURE_WITH_BP_LOADED, MEMCPY, MEMSET, NAME_DEFAULT, NAME_FORMAT, NO_TEXTURE,
        PHYRE_FREE, PHYRE_MALLOC, PRINTF_SLOT, SCENE, SEGMENT_DIVISOR, SEGMENTS_CONSTRUCT,
        SEGMENTS_DESTROY, SET_MESH_VISIBLE, SET_VALUE_DESC, SHADER_MAP_LIST_DESC, SHADER_NODE_COPY,
        SHADER_TREE_INSERT, SHADOW_MANAGER, SHADOW_MAP, SHADOW_SAMPLER, SHAPE_ENLARGE,
        SPRINTF_SLOT, STRSTR_SLOT, TEXTURE_MISSING_FORMAT, TEXTURE_SAMPLER_NAME,
        TEXTURE_SAMPLER_STATE_NAME, TREE_COUNT_NODES, TREE_NEXT, TREE_POST_INSERT, UNIFY_FILENAME,
        VFX_CREATE, VFX_CREATE_DATA_BUFFER, WRITE_VECTOR_PARAMETER,
    };
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn method(h: &mut impl Host, sym: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Thiscall(this), args)
    }

    /// Calls the CRT import whose slot is `slot` (cdecl).
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// A `PArray`'s data: inline after the count word for one element,
    /// the pointer at `+4` for more, none for an empty one.
    fn array_data(h: &impl Host, array: u32) -> u32 {
        match h.read::<u32>(array) & 0x7fff_ffff {
            0 => 0,
            1 => array + 4,
            _ => h.read(array + 4),
        }
    }

    // ---- the blob shadow ----

    /// The material of the shadow renderer `r`'s current mesh slot.
    fn current_material(h: &impl Host, r: u32) -> u32 {
        let k: u32 = h.read(r + 0x10);
        let mesh: u32 = h.read(r.wrapping_add(k.wrapping_mul(4)));
        h.read(mesh + 0x94)
    }

    /// `ChrY0Shadow_createMesh(r)` (thiscall) -> the last virtual call's
    /// `eax` (`g_PhyreScene` without the shadow cluster): with the
    /// `ChrY0Shadow` cluster (scene `+0xfa7c`), slot 0's material gets
    /// list caps 500 / 1500 for the instancing (`+4`/`+8`, put back
    /// after), the geometry (`r + 8`) takes slot 0's mesh (`+0x24`) and
    /// instantiates the cluster into the scene's ground-shadow cluster
    /// (`+0x28c4`, virtual `+4`); its first material's `TextureSamplerState`
    /// and `TextureSampler` get the shadow manager's sampler and shadow
    /// map (the record at `rec`, the original's `ebp - 0x1c`); the mesh is
    /// made visible, the geometry finished (virtual `+8`, 0) and `+0xc` is
    /// 1.
    pub fn create_mesh(h: &mut impl Host, rec: u32, r: u32) -> u32 {
        let scene = SCENE.get(h);
        let cluster: u32 = h.read(scene + 0xfa7c);
        if cluster == 0 {
            return scene;
        }
        h.write(r + 0x10, 0_u32);
        let mesh0: u32 = h.read(r);
        let m: u32 = h.read(mesh0 + 0x94);
        let caps = (h.read::<u32>(m + 4), h.read::<u32>(m + 8));
        h.write(m + 4, 500_u32);
        let m = current_material(h, r);
        h.write(m + 8, 0x5dc_u32);
        let k: u32 = h.read(r + 0x10);
        let mesh: u32 = h.read(r.wrapping_add(k.wrapping_mul(4)));
        let geo: u32 = h.read(r + 8);
        h.write(geo + 0x24, mesh);
        let geo: u32 = h.read(r + 8);
        let scene = SCENE.get(h);
        h.call_virtual(geo, 1, &[cluster, scene + 0x28c4]);
        let geo: u32 = h.read(r + 8);
        let gmesh: u32 = h.read(geo + 0xc);
        let materials: u32 = h.read(gmesh + 0x10);
        let first: u32 = h.read(array_data(h, materials));
        let params: u32 = h.read(first);
        let buffer: u32 = h.read(first + 4);
        let name = h.rebase(TEXTURE_SAMPLER_NAME);
        let sampler_param = method(h, FIND_PARAM, params, &[name]);
        let name = h.rebase(TEXTURE_SAMPLER_STATE_NAME);
        let state_param = method(h, FIND_PARAM, params, &[name]);
        if state_param != 0 {
            let sm = cdecl(h, SHADOW_MANAGER, &[]);
            let sampler = method(h, SHADOW_SAMPLER, sm, &[]);
            method(h, SET_VALUE_DESC, buffer, &[state_param, sampler]);
        }
        if sampler_param != 0 {
            h.write(rec, 0xff_u32);
            let sm = cdecl(h, SHADOW_MANAGER, &[]);
            let map = method(h, SHADOW_MAP, sm, &[]);
            let texture = if h.read::<u8>(map + 4) == 0 {
                h.read(map)
            } else {
                0
            };
            h.write(rec + 0xc, texture);
            let sm = cdecl(h, SHADOW_MANAGER, &[]);
            let sampler = method(h, SHADOW_SAMPLER, sm, &[]);
            h.write(rec + 8, sampler);
            h.write(rec + 4, 0_u32);
            method(h, WRITE_VECTOR_PARAMETER, buffer, &[sampler_param, rec]);
        }
        let m = current_material(h, r);
        h.write(m + 4, caps.0);
        let m = current_material(h, r);
        h.write(m + 8, caps.1);
        let geo: u32 = h.read(r + 8);
        let gmesh: u32 = h.read(geo + 0xc);
        if gmesh != 0 {
            let scene = SCENE.get(h);
            method(h, SET_MESH_VISIBLE, scene, &[gmesh, 1]);
        }
        let geo: u32 = h.read(r + 8);
        let ret = h.call_virtual(geo, 2, &[0]);
        h.write(r + 0xc, 1_u8);
        ret
    }

    // ---- the dynamic geometry ----

    /// One CLUT parameter's copy: `size` bytes from `src` to the
    /// geometry buffer `buf` at the parameter's offset, when they fit and
    /// the offset is past the size word.
    fn copy_param(h: &mut impl Host, p: u32, buf: u32, src: u32) -> u32 {
        let off = u32::from(h.read::<u16>(p + 8));
        let size = u32::from(h.read::<u16>(p + 0xa) & 0x1fff);
        if off + size <= h.read::<u32>(buf) && off >= 4 {
            cdecl(h, MEMCPY, &[buf + off, src, size]);
        }
        off
    }

    /// Four colour bytes at `src` as floats at `dst`, through the argument
    /// slot `slot` as the original's `fild` does; the slot ends with the
    /// last byte.
    fn bytes_to_floats(h: &mut impl Host, slot: u32, src: u32, dst: u32) {
        for k in 0..4 {
            let b: u8 = h.read(src + k);
            h.write(slot, u32::from(b));
            h.write(dst + 4 * k, f32::from(b).to_bits());
        }
    }

    /// `VFXDynamicGeometry_setTkRuntimeClutParameters(g; c0, c1, f)`
    /// (`ret 0xc`; `frame` the original's 9 frame words at `ebp - 0x24`,
    /// `slots` its argument slots, reused): the two colours (`+0x44`, `+0x48`) and
    /// the float (`+0x4c`) kept, `+0x40` = 1; for each geometry of the list
    /// (`+0x14`) whose material buffer (`+4` of the entry) has them, the
    /// three CLUT parameters get the colours' bytes as floats and the
    /// float clamped to `-256..255` (NaN 255). A parameter wider than its
    /// source (16, 16 and 4 bytes) copies the original's frame past it
    /// (saved registers, the return address, the argument slots); the
    /// port copies its own frame there (the shaders' parameters are a
    /// float4, a float4 and a float).
    #[allow(clippy::too_many_arguments)]
    pub fn set_tk_runtime_clut(
        h: &mut impl Host,
        slots: u32,
        frame: u32,
        g: u32,
        c0: u32,
        c1: u32,
        f: u32,
    ) {
        let slot0 = slots;
        let slot1 = slots + 4;
        h.write(g + 0x44, c0);
        h.write(g + 0x4c, x87::store(x87::load(f)));
        h.write(g + 0x48, c1);
        h.write(g + 0x40, 1_u8);
        let list: u32 = h.read(g + 0x14);
        if list == 0 {
            return;
        }
        let n = h.read::<u32>(list) & 0x7fff_ffff;
        h.write(slots + 8, n);
        for i in 0..n {
            let list: u32 = h.read(g + 0x14);
            let data = array_data(h, list);
            let elem: u32 = h.read(data + 4 * i);
            h.write(slot0, elem);
            let params: u32 = h.read(g + 0x28);
            let name = h.rebase(CLUT_NAMES[0]);
            let p = method(h, FIND_PARAM, params, &[name]);
            if p != 0 {
                let buf: u32 = h.read(elem + 4);
                bytes_to_floats(h, slot1, g + 0x44, frame + 0x10);
                let off = u32::from(h.read::<u16>(p + 8));
                h.write(slot1, off);
                copy_param(h, p, buf, frame + 0x10);
            }
            let params: u32 = h.read(g + 0x28);
            let name = h.rebase(CLUT_NAMES[1]);
            let p = method(h, FIND_PARAM, params, &[name]);
            if p != 0 {
                let buf: u32 = h.read(elem + 4);
                bytes_to_floats(h, slot1, g + 0x48, frame);
                let off = u32::from(h.read::<u16>(p + 8));
                h.write(slot1, off);
                copy_param(h, p, buf, frame);
            }
            let params: u32 = h.read(g + 0x28);
            let name = h.rebase(CLUT_NAMES[2]);
            let p = method(h, FIND_PARAM, params, &[name]);
            if p != 0 {
                let v = x87::load(h.read(g + 0x4c));
                h.write(slot1, x87::store(v));
                // 255 wins unless v < 255 (NaN included), then -256 below
                // it
                let r = if v < 255.0 {
                    if -256.0 > v { -256.0 } else { v }
                } else {
                    255.0
                };
                let bits = x87::store(r);
                h.write(slot1, bits);
                h.write(frame + 0x20, bits);
                let buf: u32 = h.read(elem + 4);
                let off = u32::from(h.read::<u16>(p + 8));
                h.write(slot0, off);
                copy_param(h, p, buf, frame + 0x20);
            }
        }
    }

    /// `ClassDynamicGeometry_resizeSegments_006a44d0(g; n, _, _)` (`ret
    /// 0xc`) -> 0, or 0xd when an allocation fails: the segment array
    /// (`[g + 4]`) gets `n` 0x6c-byte entries when its count differs (new
    /// ones constructed, the old ones destroyed unless borrowed); each
    /// entry gets its index, the geometry's vertex block (`[g + 0x24] +
    /// 0xb8`) at `+0x10`, four 0x40-byte index buffers (`+0x2c`, count 4 at
    /// `+0x28`, the old ones cleared and freed), a 16-bit index format
    /// (`+0x30..+0x3c`) and its bound index data released (`+0x64`).
    pub fn resize_segments(h: &mut impl Host, g: u32, n: u32) -> u32 {
        let arr: u32 = h.read(g + 4);
        if n != h.read::<u32>(arr) {
            let mut data = 0;
            if n != 0 {
                data = cdecl(h, PHYRE_MALLOC, &[n.wrapping_mul(0x6c), 4]);
                if data == 0 {
                    return 0xd;
                }
                cdecl(h, SEGMENTS_CONSTRUCT, &[data, n]);
            }
            let old: u32 = h.read(arr);
            let old_data: u32 = h.read(arr + 4);
            if old_data != data && old >> 31 == 0 {
                cdecl(h, SEGMENTS_DESTROY, &[old_data, old & 0x7fff_ffff]);
            }
            h.write(arr + 4, data);
            h.write(arr, n);
        }
        let mut i = 0_u32;
        while i.cast_signed() < n.cast_signed() {
            let arr: u32 = h.read(g + 4);
            let e = h.read::<u32>(arr + 4).wrapping_add(0x6c * i);
            h.write(e, i);
            let geo: u32 = h.read(g + 0x24);
            let block: u32 = h.read(geo + 0xb8);
            h.write(e + 0x10, block);
            if h.read::<u32>(e + 0x28) != 4 {
                let buffers = cdecl(h, PHYRE_MALLOC, &[0x100, 4]);
                if buffers == 0 {
                    return 0xd;
                }
                for k in 0..4 {
                    let b = buffers.wrapping_add(0x40 * k);
                    if b != 0 {
                        method(h, BUFFER_CTOR, b, &[]);
                    }
                }
                let count: u32 = h.read(e + 0x28);
                let old: u32 = h.read(e + 0x2c);
                if old != buffers && count >> 31 == 0 && old != 0 {
                    for k in 0..count & 0x7fff_ffff {
                        method(h, BUFFER_WAIT_AND_CLEAR, old.wrapping_add(0x40 * k), &[]);
                    }
                    cdecl(h, PHYRE_FREE, &[old]);
                }
                h.write(e + 0x2c, buffers);
                h.write(e + 0x28, 4_u32);
            }
            h.write(e + 0x38, 0_u32);
            h.write(e + 0x3c, 0xc_u8);
            h.write(e + 0x30, 0_u32);
            h.write(e + 0x34, 0_u32);
            cdecl(h, FORMAT_SIZE, &[0xc]);
            if h.read::<u32>(e + 0x64) != 0 {
                if method(h, INDEX_IS_BOUND, e + 0x30, &[]) & 0xff != 0 {
                    method(h, INDEX_RELEASE, e + 0x30, &[]);
                }
                h.write(e + 0x64, 0_u32);
            }
            i += 1;
        }
        0
    }

    // ---- the shader map ----

    /// `VfxShaderManager_copyShaderMap_006bee80(map; src)` (`ret 4`) ->
    /// `map`: an empty red-black tree head (`+0`, `+0xc` pointing at
    /// itself) and a free list (`+0x18`: 0x14-byte nodes, 8 per block);
    /// with nodes in `src`, a block for all of them, then each node in
    /// order copied (`VfxShaderNode_copy_006bf170`, the value `+0x10`)
    /// and inserted; a duplicate key's copy is freed again.
    pub fn copy_shader_map(h: &mut impl Host, map: u32, src: u32) -> u32 {
        for k in 0..6 {
            h.write(map + 4 * k, map);
        }
        let list = map + 0x18;
        let desc = h.rebase(SHADER_MAP_LIST_DESC);
        method(h, FREE_LIST_CTOR, list, &[0x14, 8, 4, desc]);
        let n = method(h, TREE_COUNT_NODES, src, &[]);
        if n == 0 {
            return map;
        }
        method(h, FREE_LIST_ALLOCATE_BLOCK, list, &[n]);
        // the leftmost node: down the `+0` links to the head
        let mut node: u32 = h.read(src + 0xc);
        if node != src && node != 0 {
            loop {
                let next: u32 = h.read(node);
                if next == src {
                    break;
                }
                node = next;
            }
        }
        if node == src {
            return map;
        }
        let mut next = method(h, TREE_NEXT, src, &[node]);
        while node != src {
            let mut item: u32 = h.read(list);
            if item == 0 {
                let block: u32 = h.read(list + 8);
                method(h, FREE_LIST_ALLOCATE_BLOCK, list, &[block]);
                item = h.read(list);
            }
            if item != 0 {
                let after: u32 = h.read(item);
                h.write(list, after);
                method(h, SHADER_NODE_COPY, item, &[node]);
                let value: u32 = h.read(node + 0x10);
                h.write(item + 0x10, value);
                if method(h, SHADER_TREE_INSERT, map, &[item]) == 0 {
                    method(h, TREE_POST_INSERT, map, &[item]);
                } else {
                    let key: u32 = h.read(item + 0xc);
                    if key & 1 == 0 {
                        cdecl(h, PHYRE_FREE, &[key]);
                    }
                    method(h, FREE_LIST_DEALLOCATE, list, &[item]);
                }
            }
            node = next;
            if next != src {
                next = method(h, TREE_NEXT, src, &[next]);
            }
        }
        map
    }

    // ---- the render-data buffers ----

    /// `memset(at, 0, n)`.
    fn zero(h: &mut impl Host, at: u32, n: u32) {
        cdecl(h, MEMSET, &[at, 0, n]);
    }

    /// The bp texture fields of a 64-bit texture word: `(word & 0x3fff,
    /// (word >> 20) & 0x3f, 1 << ((word >> 26) & 0xf), 1 << ((word >> 30)
    /// & 0xf))`.
    fn bp_fields(v: u64) -> [u32; 4] {
        let low = |s: u32, m: u64| ((v >> s) & m) as u32;
        [
            low(0, 0x3fff),
            low(20, 0x3f),
            1 << low(26, 0xf),
            1 << low(30, 0xf),
        ]
    }

    /// `createVFXRenderDataBufferRects(lo, hi, n, flags)` -> 1 (frame
    /// `ebp - 0x68b4`): one data buffer of `4n` vertices and `6n` indices
    /// named by the default format.
    pub fn create_rects(h: &mut impl Host, ebp: u32, lo: u32, hi: u32, n: u32, flags: u32) -> u32 {
        zero(h, ebp - 0x6590, 0x18c);
        zero(h, ebp - 0x6720, 0x18c);
        h.write(ebp - 0x68b4, 1_u32);
        zero(h, ebp - 0x68b0, 0x18c);
        h.write(ebp - 0x6594, 0_u32);
        h.write(ebp - 0x6724, 0_u32);
        let (fmt, arg) = (h.rebase(NAME_FORMAT), h.rebase(NAME_DEFAULT));
        import(h, SPRINTF_SLOT, &[ebp - 0x6404, fmt, arg]);
        h.write(ebp - 0x6594, n.wrapping_mul(4));
        h.write(ebp - 0x6724, n.wrapping_mul(6));
        cdecl(h, VFX_CREATE, &[lo, hi, flags]);
        cdecl(
            h,
            VFX_CREATE_DATA_BUFFER,
            &[
                lo,
                hi,
                1,
                ebp - 0x6594,
                ebp - 0x6724,
                ebp - 0x6404,
                ebp - 0x68b4,
            ],
        );
        1
    }

    /// `createGuildMapRenderDataBuffer(lo, hi, model, info)` -> 1, or 0
    /// past 29 groups or on a group with `+1` set (frame `ebp - 0x1f7c`):
    /// one data buffer per group of the model's list (`+4` from the model,
    /// `+0x10` the vertex count), `3k` indices each (`k` its `+2`, tripled
    /// again for the second count), named "no texture"; the guide map
    /// made with `info`'s alpha function and flags.
    pub fn create_guild_map(
        h: &mut impl Host,
        ebp: u32,
        lo: u32,
        hi: u32,
        model: u32,
        info: u32,
    ) -> u32 {
        h.write(ebp - 0x1f7c, info);
        h.write(ebp - 0x1ef4, 0_u32);
        zero(h, ebp - 0x1ef0, 0x74);
        h.write(ebp - 0x1e7c, 0_u32);
        zero(h, ebp - 0x1e78, 0x74);
        h.write(ebp - 0x1f6c, 0_u32);
        zero(h, ebp - 0x1f68, 0x74);
        let mut left = i32::from(h.read::<i16>(model + 0x10));
        let mut group = model.wrapping_add(h.read(model + 4));
        h.write(ebp - 0x1f74, left);
        let mut n = 0_u32;
        if left > 0 {
            loop {
                n += 1;
                let name = ebp - 0x1e04 + 0x100 * (n - 1);
                if n >= 0x1e {
                    return 0;
                }
                h.write(ebp - 0x1f70, 0_u32);
                if h.read::<u8>(group + 1) != 0 {
                    return 0;
                }
                let k = i32::from(h.read::<i16>(group + 2));
                let mut next = group + 0x10;
                let mut count = 0_u32;
                h.write(ebp - 0x1f78, next);
                if k > 0 {
                    count = k.cast_unsigned().wrapping_mul(3);
                    h.write(ebp - 0x1f70, count);
                    next = next.wrapping_add(k.cast_unsigned().wrapping_mul(20));
                    h.write(ebp - 0x1f78, next);
                }
                h.write(ebp - 0x1ef4 + 4 * (n - 1), count);
                h.write(ebp - 0x1e7c + 4 * (n - 1), count);
                let text = h.rebase(NO_TEXTURE);
                import(h, SPRINTF_SLOT, &[name, text]);
                left = left.wrapping_sub(i32::from(h.read::<i16>(group + 2)));
                h.write(ebp - 0x1f74, left);
                group = next;
                if left <= 0 {
                    break;
                }
            }
        }
        let info: u32 = h.read(ebp - 0x1f7c);
        let alpha = u32::from(h.read::<u8>(info + 8));
        let mut flags = cdecl(h, ALPHA_FUNCTION, &[alpha]);
        if h.read::<u32>(info + 0x44) != 0 {
            flags |= 1;
        }
        if h.read::<u8>(info) & 8 != 0 {
            flags |= 0x40;
        }
        flags |= 0x800;
        for c in 0..n {
            let at = ebp - 0x1e7c + 4 * c;
            let v: u32 = h.read(at);
            h.write(at, v.wrapping_mul(3));
        }
        cdecl(h, GUILDMAP_CREATE, &[lo, hi, flags]);
        cdecl(
            h,
            VFX_CREATE_DATA_BUFFER,
            &[
                lo,
                hi,
                n,
                ebp - 0x1ef4,
                ebp - 0x1e7c,
                ebp - 0x1e04,
                ebp - 0x1f6c,
            ],
        );
        1
    }

    /// `createVFXRenderDataBufferShp_OShapeVu1(lo, hi, shape, flags)` -> 1,
    /// or 0 (reported) when its bp texture isn't loaded or a count is 0
    /// (frame `ebp - 0x68bc`): one data buffer of `12k` vertices and `6k`
    /// indices (`k` = the shape's `+2` times `g_VfxShapeEnlargeBufferFactor`;
    /// the second times the segment divisor), flag 8 added when the name
    /// isn't "no texture".
    pub fn create_shp_vu1(
        h: &mut impl Host,
        ebp: u32,
        lo: u32,
        hi: u32,
        shape: u32,
        flags: u32,
    ) -> u32 {
        h.write(ebp - 0x68b8, shape);
        zero(h, ebp - 0x6720, 0x18c);
        zero(h, ebp - 0x6590, 0x18c);
        h.write(ebp - 0x68b4, 1_u32);
        zero(h, ebp - 0x68b0, 0x18c);
        h.write(ebp - 0x6724, 0_u32);
        h.write(ebp - 0x6594, 0_u32);
        let (fmt, arg) = (h.rebase(NAME_FORMAT), h.rebase(NAME_DEFAULT));
        import(h, SPRINTF_SLOT, &[ebp - 0x6404, fmt, arg]);
        let k = i32::from(h.read::<i16>(shape + 2))
            .cast_unsigned()
            .wrapping_mul(SHAPE_ENLARGE.get(h));
        let vertices = k.wrapping_mul(12);
        if h.read::<u32>(ebp - 0x6724).cast_signed() < vertices.cast_signed() {
            h.write(ebp - 0x6724, vertices);
        }
        let indices = k.wrapping_mul(6);
        if h.read::<u32>(ebp - 0x6594).cast_signed() < indices.cast_signed() {
            h.write(ebp - 0x6594, indices);
        }
        // the texture word: `+0x14` high, `+0x10` sign-extended low
        let low: u32 = h.read(shape + 0x10);
        let high: u32 = h.read(shape + 0x14);
        let v = (u64::from(high) << 32).wrapping_add(i64::from(low.cast_signed()).cast_unsigned());
        let [tex, pal, w, hh] = bp_fields(v);
        h.write(ebp - 0x68b8, pal);
        h.write(ebp - 0x68bc, tex);
        let r = cdecl(
            h,
            IS_TEXTURE_WITH_BP_LOADED,
            &[tex, pal, 0, 0, w, hh, ebp - 0x6404],
        );
        if r == 0 {
            let fmt = h.rebase(BP_MISSING_FORMAT);
            cdecl(h, DBG_PRINTF, &[fmt, tex]);
            return 0;
        }
        if h.read::<u32>(ebp - 0x6724) == 0 || h.read::<u32>(ebp - 0x6594) == 0 {
            let text = h.rebase(EMPTY_BUFFER_TEXT);
            cdecl(h, DBG_PRINTF, &[text]);
            return 0;
        }
        let text = h.rebase(NO_TEXTURE);
        let mut flags = flags;
        if import(h, STRSTR_SLOT, &[ebp - 0x6404, text]) == 0 {
            flags |= 8;
        }
        let indices = SEGMENT_DIVISOR.get(h).wrapping_mul(h.read(ebp - 0x6594));
        h.write(ebp - 0x6594, indices);
        cdecl(h, VFX_CREATE, &[lo, hi, flags]);
        cdecl(
            h,
            VFX_CREATE_DATA_BUFFER,
            &[
                lo,
                hi,
                1,
                ebp - 0x6724,
                ebp - 0x6594,
                ebp - 0x6404,
                ebp - 0x68b4,
            ],
        );
        1
    }

    /// `createU16MeshBreakRenderDataBuffer(lo, hi, model, chr, flags)` -> 1
    /// (frame `ebp - 0x338`): the character's break model path (its name's
    /// first letter picks the format: m, n, f, c, s, w; another letter
    /// leaves the path buffer as it was, a never-written frame buffer:
    /// junk read, zeroed here, brief 2026-10-24), logged, unified, and one
    /// data buffer of `3 [model + 4]` vertices and indices.
    #[allow(clippy::too_many_arguments)]
    pub fn create_u16_mesh_break(
        h: &mut impl Host,
        ebp: u32,
        lo: u32,
        hi: u32,
        model: u32,
        chr: u32,
        flags: u32,
    ) -> u32 {
        // junk read: [ebp-0x12c] never written for other letters; 0 (brief
        // 2026-10-24)
        let mut folder = [0_u8; 31];
        h.read_bytes(h.rebase(BREAK_FOLDER), &mut folder);
        h.write_bytes(ebp - 0x2c, &folder[..30]);
        let ext: u32 = h.read(h.rebase(BREAK_EXTENSION));
        h.write(ebp - 0xc, ext);
        h.write(ebp - 0x330, 0_u32);
        h.write(ebp - 0x334, 0_u32);
        h.write(ebp - 0x338, 1_u32);
        h.write(ebp - 0x2c + 30, folder[30]);
        let ext2: u16 = h.read(h.rebase(BREAK_EXTENSION) + 4);
        h.write(ebp - 8, ext2);
        let name = cdecl(h, CH_GET_NAME, &[chr]);
        let fmt = h.rebase(BREAK_NAME_FORMAT);
        import(h, SPRINTF_SLOT, &[ebp - 0x22c, fmt, name]);
        let first: u8 = h.read(name);
        if let Some(&(_, fmt)) = BREAK_PATH_FORMATS.iter().find(|(c, _)| *c == first) {
            let fmt = h.rebase(fmt);
            import(
                h,
                SPRINTF_SLOT,
                &[ebp - 0x12c, fmt, ebp - 0x2c, name, ebp - 0xc, ebp - 0x22c],
            );
        }
        let fmt = h.rebase(BREAK_LOG_FORMAT);
        cdecl(h, DBG_PRINTF, &[fmt, ebp - 0x12c]);
        cdecl(h, UNIFY_FILENAME, &[ebp - 0x12c, ebp - 0x32c, 0x100]);
        let k = h.read::<u32>(model + 4).wrapping_mul(3);
        h.write(ebp - 0x330, k);
        h.write(ebp - 0x334, k);
        cdecl(h, VFX_CREATE, &[lo, hi, flags]);
        cdecl(
            h,
            VFX_CREATE_DATA_BUFFER,
            &[
                lo,
                hi,
                1,
                ebp - 0x330,
                ebp - 0x334,
                ebp - 0x32c,
                ebp - 0x338,
            ],
        );
        1
    }

    /// `createGT3MeshBreakRenderDataBuffer(lo, hi, model, flags)` -> 1, or
    /// 0 (printed) when the model's bp texture (`+8`, 64 bits) isn't loaded
    /// (frame `ebp - 0x320`): one data buffer of `3 [model + 4]` vertices
    /// and indices named after the loaded texture.
    pub fn create_gt3_mesh_break(
        h: &mut impl Host,
        ebp: u32,
        lo: u32,
        hi: u32,
        model: u32,
        flags: u32,
    ) -> u32 {
        let v = u64::from(h.read::<u32>(model + 8)) | (u64::from(h.read::<u32>(model + 0xc)) << 32);
        let [tex, pal, w, hh] = bp_fields(v);
        h.write(ebp - 0x308, 0_u32);
        h.write(ebp - 0x30c, 0_u32);
        h.write(ebp - 0x310, 1_u32);
        let fmt = h.rebase(BP_NAME_FORMAT);
        import(h, SPRINTF_SLOT, &[ebp - 0x104, fmt, tex, pal, w, hh]);
        let r = cdecl(
            h,
            IS_TEXTURE_LOADED,
            &[
                ebp - 0x104,
                ebp - 0x304,
                ebp - 0x320,
                ebp - 0x318,
                ebp - 0x314,
                ebp - 0x31c,
            ],
        );
        if r == 0 {
            let fmt = h.rebase(TEXTURE_MISSING_FORMAT);
            import(h, PRINTF_SLOT, &[fmt, ebp - 0x104]);
            return 0;
        }
        let fmt = h.rebase(NAME_FORMAT);
        import(h, SPRINTF_SLOT, &[ebp - 0x204, fmt, ebp - 0x304]);
        let k = h.read::<u32>(model + 4).wrapping_mul(3);
        h.write(ebp - 0x310, 1_u32);
        h.write(ebp - 0x308, k);
        h.write(ebp - 0x30c, k);
        cdecl(h, VFX_CREATE, &[lo, hi, flags]);
        cdecl(
            h,
            VFX_CREATE_DATA_BUFFER,
            &[
                lo,
                hi,
                1,
                ebp - 0x308,
                ebp - 0x30c,
                ebp - 0x204,
                ebp - 0x310,
            ],
        );
        1
    }
}
