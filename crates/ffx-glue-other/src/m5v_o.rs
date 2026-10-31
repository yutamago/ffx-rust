//! M5 stage 1, list `vfx`, round 5 batch 10: the VFX shader manager's
//! special shader parameters (`VfxShaderManager::setupSpecialShaderParameters`,
//! Fahrenheit's name): a shader's extra parameters by name (textures from
//! the current level's clusters, loaded asynchronously when missing, and
//! half floats).
//!
//! The original keeps a copy of the shader's parameter map, iterators and
//! records in its frame and hands them to callees; the bridge takes its
//! `ebp` (the shim passes a `StackFrame`). SEH frames and the stack
//! cookie are left out.

/// `g_PhyreScene`.
pub const SCENE: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00cc_b9d8);
/// The type names the parameters are matched by.
pub const SAMPLER_2D: u32 = 0x00b4_9284;
pub const SAMPLER_CUBE: u32 = 0x00b4_9290;
pub const HALF4: u32 = 0x00b4_929c;
pub const HALF3: u32 = 0x00b4_92a4;
pub const HALF: u32 = 0x00b4_92ac;
/// The async load callbacks for 2D and cube textures.
pub const LOADED_2D: u32 = 0x006c_1280;
pub const LOADED_CUBE: u32 = 0x006c_1320;
/// `MSVCR110` slots: `strstr`, `atof` (cdecl; `atof` in ST0).
pub const STRSTR_SLOT: u32 = 0x00b0_c378;
pub const ATOF_SLOT: u32 = 0x00b0_c408;
/// The frame: `ebp` minus its lowest local (the map copy).
pub const FRAME: u32 = 0x1a4;

// ---- callees ----

/// `Phyre_PhyreMalloc_0042fb80(n, align)`, `memcpy`, `Phyre_PhyreFree(p)`
/// (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
pub const MEMCPY: u32 = 0x0094_9256;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `PStringTree_Find(tree; &key)`, `Phyre_PString_PString(s; text)`,
/// `VfxShaderManager_copyShaderMap_006bee80(map; src)`,
/// `PTreeIterator_begin_006bf2a0(it; tree)`, `ShaderParams_FindByName(params;
/// name)`, `Phyre_PRedBlackTree_successor(tree; node)`,
/// `Phyre_PRedBlackTree_remove(tree; node)` (thiscall, `ret 4`).
pub const TREE_FIND: u32 = 0x006b_e750;
pub const PSTRING_CTOR: u32 = 0x0042_f090;
pub const COPY_SHADER_MAP: u32 = 0x006b_ee80;
pub const ITERATOR_BEGIN: u32 = 0x006b_f2a0;
pub const FIND_PARAM: u32 = 0x0056_cbf0;
pub const TREE_SUCCESSOR: u32 = 0x0057_5160;
pub const TREE_REMOVE: u32 = 0x0057_5000;
/// `VfxShaderManager_combineTextureNameInCurrentLevel(m; name, out)`
/// (`ret 8`), `ClusterManager_GetInstance()` (cdecl),
/// `ClusterManager_getPClusterByName(cm; name)` (`ret 4`),
/// `AsyncLoadManager_sGetInstance()`, `AsyncLoadManager_loadCluster(m, 0,
/// name, callback, 3, five 64-bit words...)` (cdecl, 11 words).
pub const COMBINE_TEXTURE_NAME: u32 = 0x006c_0560;
pub const CLUSTER_MANAGER: u32 = 0x0069_a1a0;
pub const CLUSTER_BY_NAME: u32 = 0x0069_b4b0;
pub const ASYNC_LOAD_MANAGER: u32 = 0x0069_1b40;
pub const LOAD_CLUSTER: u32 = 0x0069_1660;
/// `PTextureIterator_ctor_00463bf0(it; cluster)`, `FUN_00463cb0(it;
/// cluster)` (the cube one), `PhyreScene_getLinearSampler(scene; which)`
/// (`ret 4`), `Render_SurfaceFormat_0066e4f0(buffer; param, record)`,
/// `VfxShader_writeTextureParameter_006c14d0(buffer; param, record)`,
/// `ShaderParamBuffer_SetValue2(buffer; param, src)` (`ret 8`).
pub const TEXTURE_ITERATOR: u32 = 0x0046_3bf0;
pub const CUBE_ITERATOR: u32 = 0x0046_3cb0;
pub const LINEAR_SAMPLER: u32 = 0x0065_e050;
pub const WRITE_VECTOR_PARAMETER: u32 = 0x0066_e4f0;
pub const WRITE_TEXTURE_PARAMETER: u32 = 0x006c_14d0;
pub const SET_VALUE2: u32 = 0x0063_86c0;
/// `Phyre_PFreeList_reset(list)`, `Phyre_PFreeList_dtor_PFreeList(list)`,
/// `Phyre_PRedBlackTree_clear(tree)` (thiscall).
pub const FREE_LIST_RESET: u32 = 0x0049_cc10;
pub const FREE_LIST_DTOR: u32 = 0x0049_c5b0;
pub const TREE_CLEAR: u32 = 0x0057_4760;

pub mod bridge {
    use super::{
        ASYNC_LOAD_MANAGER, ATOF_SLOT, CLUSTER_BY_NAME, CLUSTER_MANAGER, COMBINE_TEXTURE_NAME,
        COPY_SHADER_MAP, CUBE_ITERATOR, FIND_PARAM, FREE_LIST_DTOR, FREE_LIST_RESET, HALF, HALF3,
        HALF4, ITERATOR_BEGIN, LINEAR_SAMPLER, LOAD_CLUSTER, LOADED_2D, LOADED_CUBE, MEMCPY,
        PHYRE_FREE, PHYRE_MALLOC, PSTRING_CTOR, SAMPLER_2D, SAMPLER_CUBE, SCENE, SET_VALUE2,
        STRSTR_SLOT, TEXTURE_ITERATOR, TREE_CLEAR, TREE_FIND, TREE_REMOVE, TREE_SUCCESSOR,
        WRITE_TEXTURE_PARAMETER, WRITE_VECTOR_PARAMETER,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn method(h: &mut impl Host, sym: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Thiscall(this), args)
    }

    fn strstr(h: &mut impl Host, s: u32, needle: u32) -> u32 {
        let f: u32 = h.read(h.rebase(STRSTR_SLOT));
        let needle = h.rebase(needle);
        h.call(f, Conv::Cdecl, &[s, needle])
    }

    /// A tagged string word: the pointer with its flag bit (bit 0, "not
    /// owned") taken off.
    fn untag(h: &impl Host, at: u32) -> u32 {
        let v: u32 = h.read(at);
        v.wrapping_sub(v & 1)
    }

    /// Frees a tagged string unless its flag bit is set.
    fn free_tagged(h: &mut impl Host, v: u32) {
        if v & 1 == 0 {
            cdecl(h, PHYRE_FREE, &[v]);
        }
    }

    /// A texture parameter: the level's cluster named by `value` (its
    /// full name at `ebp - 0x110`) bound through `record` (`kind` and the
    /// sampler), or, when the cluster isn't loaded, an async load that
    /// calls `loaded` with the material, parameter and `which`.
    #[allow(clippy::too_many_arguments)]
    fn texture(
        h: &mut impl Host,
        ebp: u32,
        manager: u32,
        material: u32,
        which: u32,
        param: u32,
        value: u32,
        cube: bool,
    ) {
        let name = ebp - 0x110;
        method(h, COMBINE_TEXTURE_NAME, manager, &[value, name]);
        let cm = cdecl(h, CLUSTER_MANAGER, &[]);
        let cluster = method(h, CLUSTER_BY_NAME, cm, &[name]);
        if cluster == 0 {
            let words = |v: u32| [v, (v.cast_signed() >> 31).cast_unsigned()];
            let loaded = h.rebase(if cube { LOADED_CUBE } else { LOADED_2D });
            let am = cdecl(h, ASYNC_LOAD_MANAGER, &[]);
            let [w0, w1] = words(which);
            let [m0, m1] = words(material);
            let [p0, p1] = words(param);
            cdecl(
                h,
                LOAD_CLUSTER,
                &[am, 0, name, loaded, 3, w0, w1, m0, m1, p0, p1],
            );
            return;
        }
        let it = ebp - 0x174;
        method(
            h,
            if cube {
                CUBE_ITERATOR
            } else {
                TEXTURE_ITERATOR
            },
            it,
            &[cluster],
        );
        let texture = h.read::<u32>(it).wrapping_add(h.read(it + 0x10));
        let record = if cube { ebp - 0x144 } else { ebp - 0x154 };
        h.write(record, if cube { 0x44_u32 } else { 0xff });
        h.write(record + 0xc, texture);
        h.write(record + 4, 0_u32);
        let scene = SCENE.get(h);
        let sampler = method(h, LINEAR_SAMPLER, scene, &[which]);
        h.write(record + 8, sampler);
        let buffer: u32 = h.read(material + 4);
        let write = if cube {
            WRITE_TEXTURE_PARAMETER
        } else {
            WRITE_VECTOR_PARAMETER
        };
        method(h, write, buffer, &[param, record]);
    }

    /// `VfxShaderManager_setupSpecialShaderParameters(m; name, material,
    /// params, which)` (`ret 0x10`; `ebp` the original's, its frame below):
    /// with a material, the shader named `name` is looked up in the
    /// manager's trees (`+0x30` by a copied key, then `+0x68` by a
    /// `PString`); its parameter map (`[entry] + 4`) is copied to the frame
    /// and walked: each entry the shader's `params` know gets, by its type
    /// name, a 2D or cube texture, nothing (half4, half3) or a half float
    /// (`atof` of its value, `ShaderParamBuffer_SetValue2`). The copy is
    /// torn down after.
    pub fn setup_special(
        h: &mut impl Host,
        ebp: u32,
        manager: u32,
        name: u32,
        material: u32,
        params: u32,
        which: u32,
    ) {
        if material == 0 {
            return;
        }
        let key_at = ebp - 0x114;
        let key = if name == 0 {
            0
        } else {
            let mut n = 0_u32;
            while h.read::<u8>(name.wrapping_add(n)) != 0 {
                n += 1;
            }
            let p = cdecl(h, PHYRE_MALLOC, &[n + 1, 2]);
            if p != 0 {
                cdecl(h, MEMCPY, &[p, name, n + 1]);
            }
            p
        };
        h.write(key_at, key);
        let node = method(h, TREE_FIND, manager + 0x30, &[key_at]);
        let mut entry = if node == 0 { 0 } else { node + 0x10 };
        free_tagged(h, key);
        if entry == 0 {
            let s = method(h, PSTRING_CTOR, key_at, &[name]);
            let node = method(h, TREE_FIND, manager + 0x68, &[s]);
            entry = if node == 0 { 0 } else { node + 0x10 };
            let v: u32 = h.read(key_at);
            free_tagged(h, v);
            if entry == 0 {
                return;
            }
        }
        let map = ebp - 0x1a4;
        let src = h.read::<u32>(entry).wrapping_add(4);
        method(h, COPY_SHADER_MAP, map, &[src]);
        let it = ebp - 0x134;
        method(h, ITERATOR_BEGIN, it, &[map]);
        let head: u32 = h.read(it);
        let mut node: u32 = h.read(it + 4);
        if node != head {
            let mut next: u32 = h.read(it + 8);
            loop {
                let key = untag(h, node + 0xc);
                let param = method(h, FIND_PARAM, params, &[key]);
                if param != 0 {
                    let v: u32 = h.read(node + 0x10);
                    let kind = untag(h, v);
                    let value = untag(h, v + 4);
                    h.write(key_at, value);
                    if strstr(h, kind, SAMPLER_2D) != 0 {
                        texture(h, ebp, manager, material, which, param, value, false);
                    } else if strstr(h, kind, SAMPLER_CUBE) != 0 {
                        texture(h, ebp, manager, material, which, param, value, true);
                    } else if strstr(h, kind, HALF4) == 0
                        && strstr(h, kind, HALF3) == 0
                        && strstr(h, kind, HALF) != 0
                    {
                        let atof: u32 = h.read(h.rebase(ATOF_SLOT));
                        let f = h.call_st0(atof, Conv::Cdecl, &[value]);
                        h.write(key_at, f.to_bits());
                        let buffer: u32 = h.read(material + 4);
                        method(h, SET_VALUE2, buffer, &[param, key_at]);
                    }
                }
                let head: u32 = h.read(it);
                node = next;
                if next != head {
                    next = method(h, TREE_SUCCESSOR, head, &[next]);
                }
                if node == head {
                    break;
                }
            }
        }
        loop {
            let first: u32 = h.read(map + 0xc);
            if first == map || first == 0 {
                break;
            }
            method(h, TREE_REMOVE, map, &[first]);
            let key: u32 = h.read(first + 0xc);
            free_tagged(h, key);
        }
        method(h, FREE_LIST_RESET, map + 0x18, &[]);
        method(h, FREE_LIST_DTOR, map + 0x18, &[]);
        method(h, TREE_CLEAR, map, &[]);
        method(h, TREE_CLEAR, map, &[]);
    }
}
