//! M5 stage 1, list `vfx`, batch 2: the character shader renderer's
//! creation and shadow end, the VFX dynamic geometry's parameter setters
//! (point light, RGBA power, UV offset, YIQ), the VFX shader manager's
//! tree and string-node helpers and its special texture parameters, the
//! Phyre tree destructors the unwind funclets call, and the broken-screen
//! effect's object release.
//!
//! SEH frames and stack cookies are left out: nothing on these paths
//! throws. Where the original hands a callee the address of a frame
//! word, the bridge takes the original's `ebp` and keeps the words at
//! their offsets (the shim passes a `StackFrame`).

use ffx_bridge::Global;

pub use crate::m5v_a::quiet;

/// `g_ChrShaderRender`.
pub const CHR_SHADER_RENDER: Global<u32> = Global::new(0x00cc_ca74);
/// `g_PhyreScene`.
pub const SCENE: Global<u32> = Global::new(0x00cc_b9d8);

/// The shadow end's warning (`Phyre_PhyrePrintf` format).
pub const SHADOW_WARNING: u32 = 0x00b4_80a0;
/// Shader parameter names: the point light's two matrices and its
/// float, the YIQ pair.
pub const POINT_LIGHT_NAME_A: u32 = 0x00b4_1c00;
pub const POINT_LIGHT_NAME_B: u32 = 0x00b4_1c0c;
pub const POINT_LIGHT_NAME_C: u32 = 0x00b4_7f90;
pub const YIQ_NAME_A: u32 = 0x00b4_3c98;
pub const YIQ_NAME_B: u32 = 0x00b4_3ca4;
/// `g_ClassDesc_PTexture2D_00ca37d0` / `g_ClassDesc_PTextureCubeMap_00ca3ef0`.
pub const CLASS_TEXTURE_2D: u32 = 0x00ca_37d0;
pub const CLASS_TEXTURE_CUBE: u32 = 0x00ca_3ef0;

// ---- callees ----

/// `operator_new(n)` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
/// `ClassChrShaderRender_VertexInstanceSetup(r)` (thiscall).
pub const VERTEX_INSTANCE_SETUP: u32 = 0x006a_0100;
/// `SetFields_0069e0c0(p)` (thiscall) -> `p`.
pub const SET_FIELDS: u32 = 0x0069_e0c0;
/// `ChrY0Shadow_createMesh(r)` (thiscall).
pub const CREATE_MESH: u32 = 0x0069_f9d0;
/// `PhyreScene_setMeshVisible(scene; mesh, visible)` (`ret 8`).
pub const SET_MESH_VISIBLE: u32 = 0x0066_d910;
/// `Phyre_PhyrePrintf(level, format, ...)` (cdecl).
pub const PRINTF: u32 = 0x0043_52f0;
/// `ShaderParams_FindByName(params; name)` (thiscall, `ret 4`).
pub const FIND_PARAM: u32 = 0x0056_cbf0;
/// `ShaderParamBuffer_SetValue2(buffer; param, src)` (thiscall, `ret 8`).
pub const SET_VALUE2: u32 = 0x0063_86c0;
/// `PhyreScene_getYIQParam(scene; 6 pointers)` (thiscall, `ret 0x18`).
pub const GET_YIQ_PARAM: u32 = 0x0065_e9e0;
/// `Phyre_PhyreMalloc_0042fb80(n, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `memcpy` (the CRT thunk, cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
/// `Phyre_PRedBlackTree_successor(tree; node)` (thiscall, `ret 4`).
pub const TREE_SUCCESSOR: u32 = 0x0057_5160;
/// `TrophyManager_Install(this; arg)` (thiscall, `ret 4`; a texture
/// sampler check here).
pub const SAMPLER_CHECK: u32 = 0x0059_e520;
/// `PParameterBuffer_writeParameter_004d57d0(buffer; param, src)`
/// (thiscall, `ret 8`).
pub const WRITE_PARAMETER: u32 = 0x004d_57d0;
/// `Phyre_PRedBlackTree_clear(tree)` (thiscall).
pub const TREE_CLEAR: u32 = 0x0057_4760;
/// `Phyre_PRedBlackTree_orphanAllContents(tree)` (thiscall).
pub const TREE_ORPHAN_ALL: u32 = 0x0057_4970;
/// `Phyre_PFreeList_reset(list)` / `Phyre_PFreeList_dtor_PFreeList(list)`.
pub const FREE_LIST_RESET: u32 = 0x0049_cc10;
pub const FREE_LIST_DTOR: u32 = 0x0049_c5b0;
/// `PTreeFreeList_clear_006c04e0(t)` / `PTreeFreeList_clear_006c0520(t)`.
pub const TREE_FREE_LIST_CLEAR_04E0: u32 = 0x006c_04e0;
pub const TREE_FREE_LIST_CLEAR_0520: u32 = 0x006c_0520;
/// `Phyre_PCluster_deleteObject(cluster; obj)` (thiscall, `ret 4`).
pub const DELETE_OBJECT: u32 = 0x0044_1240;
/// `AsyncLoadManager_sGetInstance()` (cdecl).
pub const ASYNC_LOAD_MANAGER: u32 = 0x0069_1b40;
/// `Phyre_PClusterHeader_iterSeek_00464f30(it)` /
/// `Phyre_PClusterHeader_iterSeek_00465010(it)` (thiscall).
pub const ITER_SEEK_2D: u32 = 0x0046_4f30;
pub const ITER_SEEK_CUBE: u32 = 0x0046_5010;
/// `PhyreScene_getLinearSampler(scene; which)` (thiscall, `ret 4`).
pub const LINEAR_SAMPLER: u32 = 0x0065_e050;
/// `Render_SurfaceFormat_0066e4f0(buffer; param, record)` (thiscall,
/// `ret 8`; writes a vector parameter).
pub const WRITE_VECTOR_PARAMETER: u32 = 0x0066_e4f0;

pub mod bridge {
    use super::{
        ASYNC_LOAD_MANAGER, CHR_SHADER_RENDER, CLASS_TEXTURE_2D, CLASS_TEXTURE_CUBE, CREATE_MESH,
        DELETE_OBJECT, FIND_PARAM, FREE_LIST_DTOR, FREE_LIST_RESET, GET_YIQ_PARAM, ITER_SEEK_2D,
        ITER_SEEK_CUBE, LINEAR_SAMPLER, MEMCPY, OPERATOR_NEW, PHYRE_MALLOC, POINT_LIGHT_NAME_A,
        POINT_LIGHT_NAME_B, POINT_LIGHT_NAME_C, PRINTF, SAMPLER_CHECK, SCENE, SET_FIELDS,
        SET_MESH_VISIBLE, SET_VALUE2, SHADOW_WARNING, TREE_CLEAR, TREE_FREE_LIST_CLEAR_04E0,
        TREE_FREE_LIST_CLEAR_0520, TREE_ORPHAN_ALL, TREE_SUCCESSOR, VERTEX_INSTANCE_SETUP,
        WRITE_PARAMETER, WRITE_VECTOR_PARAMETER, YIQ_NAME_A, YIQ_NAME_B, quiet,
    };
    use ffx_graphic::host::{Conv, Host};

    fn method(h: &mut impl Host, sym: u32, this: u32) -> u32 {
        h.call_sym(sym, Conv::Thiscall(this), &[])
    }

    /// `ChrShaderRender_CreateInstance_0069f3a0()`: without
    /// `g_ChrShaderRender`, a 0x14-byte renderer (`operator_new`; 0 when
    /// it fails) set up (`0x006a0100`), its `+8` a 0x3c-byte object
    /// (`0x0069e0c0`, 0 when that allocation fails), its shadow mesh made
    /// (`ChrY0Shadow_createMesh`); it becomes the global.
    pub fn chr_shader_create_instance(h: &mut impl Host) {
        if CHR_SHADER_RENDER.get(h) != 0 {
            return;
        }
        let r = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x14]);
        if r != 0 {
            method(h, VERTEX_INSTANCE_SETUP, r);
            let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x3c]);
            let v = if p != 0 { method(h, SET_FIELDS, p) } else { 0 };
            h.write(r + 8, v);
            method(h, CREATE_MESH, r);
        }
        CHR_SHADER_RENDER.set(h, r);
    }

    /// `ChrShaderRender_ShadowEnd_0069fdc0(r)`: with the `+8` object: the
    /// current mesh (`r[r[+0x10]]`) goes to its `+0x24` (none: a warning
    /// through `Phyre_PhyrePrintf(1, ..)` and nothing else); the shadow
    /// mesh remade when the scene's byte `+0xfa9b` and `r`'s byte `+0xc`
    /// are both 0; with `+0xc` clear the object's `+0xc` mesh hidden
    /// (`PhyreScene_setMeshVisible(.., 0)`); then the object's virtual 2
    /// with 0.
    pub fn chr_shadow_end(h: &mut impl Host, r: u32) {
        let obj: u32 = h.read(r + 8);
        if obj == 0 {
            return;
        }
        let index: u32 = h.read(r + 0x10);
        let mesh: u32 = h.read(r.wrapping_add(index.wrapping_mul(4)));
        if mesh == 0 {
            let fmt = h.rebase(SHADOW_WARNING);
            h.call_sym(PRINTF, Conv::Cdecl, &[1, fmt]);
            return;
        }
        h.write(obj + 0x24, mesh);
        let scene = SCENE.get(h);
        let scene_flag: u8 = h.read(scene + 0xfa9b);
        let own: u8 = h.read(r + 0xc);
        let scene = if scene_flag == 0 && own == 0 {
            method(h, CREATE_MESH, r);
            SCENE.get(h)
        } else {
            scene
        };
        if h.read::<u8>(r + 0xc) == 0 {
            let obj: u32 = h.read(r + 8);
            let m: u32 = h.read(obj + 0xc);
            if m != 0 {
                h.call_sym(SET_MESH_VISIBLE, Conv::Thiscall(scene), &[m, 0]);
            }
        }
        let obj: u32 = h.read(r + 8);
        h.call_virtual(obj, 2, &[0]);
    }

    /// The `n`-th geometry of a dynamic geometry's list (`+0x14`: a count
    /// whose top bit is a flag, then the data inline for one entry or a
    /// pointer for more).
    fn geometry_at(h: &mut impl Host, list: u32, i: u32) -> u32 {
        let n = h.read::<u32>(list) & 0x7fff_ffff;
        let data = if n > 1 {
            h.read(list + 4)
        } else if n != 0 {
            list + 4
        } else {
            0
        };
        h.read(data.wrapping_add(i.wrapping_mul(4)))
    }

    /// For each geometry of `g`'s list: each `(name, src)` whose parameter
    /// `g`'s shader params (`+0x28`) has gets `src` (`SetValue2` on the
    /// geometry's `+4` buffer). The count is read once, before the loop.
    fn set_params(h: &mut impl Host, g: u32, n: u32, params: &[(u32, u32)]) {
        let mut i = 0_i32;
        while i < n as i32 {
            let list: u32 = h.read(g + 0x14);
            let geo = geometry_at(h, list, i as u32);
            for &(name, src) in params {
                let shader: u32 = h.read(g + 0x28);
                let name = h.rebase(name);
                let p = h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[name]);
                if p != 0 {
                    let buffer: u32 = h.read(geo + 4);
                    h.call_sym(SET_VALUE2, Conv::Thiscall(buffer), &[p, src]);
                }
            }
            i += 1;
        }
    }

    /// `VFXDynamicGeometry_setPointLightParameters(g; &m0, &m1, f)`: byte
    /// `+0x52` set, the two 16-word matrices copied to `+0x54` and
    /// `+0x94`, the float to `+0xd4` (quiet); with a list (`+0x14`), the
    /// three parameters set on every geometry. Its first argument slot
    /// ends up holding the count.
    pub fn set_point_light_parameters(
        h: &mut impl Host,
        slots: u32,
        g: u32,
        m0: u32,
        m1: u32,
        f: u32,
    ) {
        h.write(g + 0x52, 1_u8);
        for (src, dst) in [(m0, g + 0x54), (m1, g + 0x94)] {
            for k in 0..16 {
                let v: u32 = h.read(src.wrapping_add(4 * k));
                h.write(dst + 4 * k, v);
            }
        }
        h.write(g + 0xd4, quiet(f));
        let list: u32 = h.read(g + 0x14);
        if list == 0 {
            return;
        }
        let n = h.read::<u32>(list) & 0x7fff_ffff;
        h.write(slots, n);
        set_params(
            h,
            g,
            n,
            &[
                (POINT_LIGHT_NAME_A, g + 0x54),
                (POINT_LIGHT_NAME_B, g + 0x94),
                (POINT_LIGHT_NAME_C, g + 0xd4),
            ],
        );
    }

    /// `VFXDynamicGeometry_setRGBAPower(g; &rgba)`: four floats to
    /// `+0xe8` (quiet).
    pub fn set_rgba_power(h: &mut impl Host, g: u32, rgba: u32) {
        for k in 0..4 {
            let v: u32 = h.read(rgba + 4 * k);
            h.write(g + 0xe8 + 4 * k, quiet(v));
        }
    }

    /// `VFXDynamicGeometry_setUVOffset(g; &uv)`: two floats to `+0xe0`.
    pub fn set_uv_offset(h: &mut impl Host, g: u32, uv: u32) {
        for k in 0..2 {
            let v: u32 = h.read(uv + 4 * k);
            h.write(g + 0xe0 + 4 * k, quiet(v));
        }
    }

    /// `VFXDynamicGeometry_setYIQEnable(g; on)` (frame: four 1.0 at
    /// `ebp - 0x14`, four 0.0 at `ebp - 0x24`, the count at `ebp - 0x28`):
    /// byte `+0xd8` = `on`; with it and a list, the scene's YIQ values
    /// (`PhyreScene_getYIQParam` into the frame's words) go to every
    /// geometry's two parameters.
    pub fn set_yiq_enable(h: &mut impl Host, ebp: u32, g: u32, on: u8) {
        h.write(g + 0xd8, on);
        if on == 0 {
            return;
        }
        let list: u32 = h.read(g + 0x14);
        if list == 0 {
            return;
        }
        let scene = SCENE.get(h);
        for k in 0..4 {
            h.write(ebp - 0x14 + 4 * k, 1.0_f32.to_bits());
            h.write(ebp - 0x24 + 4 * k, 0_u32);
        }
        let n = h.read::<u32>(list) & 0x7fff_ffff;
        h.write(ebp - 0x28, n);
        h.call_sym(
            GET_YIQ_PARAM,
            Conv::Thiscall(scene),
            &[
                ebp - 0x14,
                ebp - 0x10,
                ebp - 0xc,
                ebp - 0x24,
                ebp - 0x20,
                ebp - 0x1c,
            ],
        );
        set_params(
            h,
            g,
            n,
            &[(YIQ_NAME_A, ebp - 0x14), (YIQ_NAME_B, ebp - 0x24)],
        );
    }

    /// `VfxShaderNode_copy_006bf170(node; &src)` -> `node`: links to
    /// itself, and a copy of the source's name (`+0xc`, a pointer whose
    /// low bit is a flag) in a new `Phyre_PhyreMalloc` block (none for
    /// no name; 0 when the allocation fails). Its argument slot ends up
    /// holding the copy's length.
    pub fn shader_node_copy(h: &mut impl Host, slots: u32, node: u32, src: u32) -> u32 {
        for k in 0..3 {
            h.write(node + 4 * k, node);
        }
        let tagged: u32 = h.read(src + 0xc);
        let name = tagged.wrapping_sub(tagged & 1);
        if name == 0 {
            h.write(node + 0xc, 0_u32);
            return node;
        }
        let mut len = 0_u32;
        while h.read::<u8>(name.wrapping_add(len)) != 0 {
            len += 1;
        }
        let n = len + 1;
        h.write(slots, n);
        let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n, 2]);
        if p != 0 {
            h.call_sym(MEMCPY, Conv::Cdecl, &[p, name, n]);
        }
        h.write(node + 0xc, p);
        node
    }

    /// `PTreeIterator_begin_006bf2a0(it; tree)` -> `it`: {tree, the last
    /// node of the `+0xc` chain before the tree (the chain's start when
    /// it is the tree or 0), that node's successor (or the tree)}.
    pub fn tree_iterator_begin(h: &mut impl Host, it: u32, tree: u32) -> u32 {
        h.write(it, tree);
        let first: u32 = h.read(tree + 0xc);
        let node = if first == tree || first == 0 {
            first
        } else {
            let mut node = first;
            loop {
                let next: u32 = h.read(node);
                if next == tree {
                    break node;
                }
                node = next;
            }
        };
        h.write(it + 4, node);
        let next = if node == tree {
            node
        } else {
            h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(tree), &[node])
        };
        h.write(it + 8, next);
        it
    }

    /// `VfxShader_writeTextureParameter_006c14d0(buffer; param, value)` ->
    /// 5 unless the parameter's type byte (`+2`) is 0x44 (a texture);
    /// then a value with a sampler (`+8`) and texture (`+0xc`) that the
    /// sampler check (`0x0059e520`) accepts returns that check's `eax`,
    /// else `PParameterBuffer_writeParameter`'s.
    pub fn write_texture_parameter(h: &mut impl Host, buffer: u32, param: u32, value: u32) -> u32 {
        if h.read::<u8>(param + 2) != 0x44 {
            return 5;
        }
        if value != 0 {
            let sampler: u32 = h.read(value + 8);
            if sampler != 0 {
                let tex: u32 = h.read(value + 0xc);
                if tex != 0 {
                    let r = h.call_sym(SAMPLER_CHECK, Conv::Thiscall(sampler), &[tex]);
                    if r != 0 {
                        return r;
                    }
                }
            }
        }
        h.call_sym(WRITE_PARAMETER, Conv::Thiscall(buffer), &[param, value])
    }

    /// The tree destructors of the unwind funclets (`0x006bf590`,
    /// `0x006bf5f0`, `0x006bf650`): `Phyre_PRedBlackTree_clear` twice.
    pub fn tree_dtor(h: &mut impl Host, t: u32) {
        method(h, TREE_CLEAR, t);
        method(h, TREE_CLEAR, t);
    }

    /// `PTreeFreeList_dtor_006bf6b0(t)`: orphaned, its free list (`+0x18`)
    /// reset and destroyed, the tree cleared twice.
    pub fn tree_free_list_dtor_orphan(h: &mut impl Host, t: u32) {
        method(h, TREE_ORPHAN_ALL, t);
        method(h, FREE_LIST_RESET, t + 0x18);
        method(h, FREE_LIST_DTOR, t + 0x18);
        method(h, TREE_CLEAR, t);
        method(h, TREE_CLEAR, t);
    }

    /// `PTreeFreeList_dtor_006bf730(t)` (`clear` 0x006c04e0) and
    /// `PTreeFreeList_dtor_006bf7a0(t)` (0x006c0520): emptied, its free
    /// list destroyed, the tree cleared twice.
    pub fn tree_free_list_dtor(h: &mut impl Host, t: u32, cube: bool) {
        let clear = if cube {
            TREE_FREE_LIST_CLEAR_0520
        } else {
            TREE_FREE_LIST_CLEAR_04E0
        };
        method(h, clear, t);
        method(h, FREE_LIST_DTOR, t + 0x18);
        method(h, TREE_CLEAR, t);
        method(h, TREE_CLEAR, t);
    }

    /// `BrokenScreenEffect_releaseObjects_006d3e00(e)`: `+0x50` and
    /// `+0x40..+0x4c` cleared; the cluster's (`+0x14`) objects at `+0x2c`
    /// and `+0x28` (their `+4`) and `+0x24`, `+0x20` deleted when set.
    pub fn broken_screen_release_objects(h: &mut impl Host, e: u32) {
        for off in [0x50_u32, 0x40, 0x44, 0x48, 0x4c] {
            if h.read::<u32>(e + off) != 0 {
                h.write(e + off, 0_u32);
            }
        }
        for (off, add) in [(0x2c_u32, 4_u32), (0x28, 4), (0x24, 0), (0x20, 0)] {
            let obj: u32 = h.read(e + off);
            if obj != 0 {
                let cluster: u32 = h.read(e + 0x14);
                h.call_sym(DELETE_OBJECT, Conv::Thiscall(cluster), &[obj + add]);
            }
        }
    }

    /// `BrokenScreen_vertexStride_006d3f50(e)` -> 0x18 for kind (`+8`) 6,
    /// 0x24 for 8, else 0.
    pub fn vertex_stride(h: &mut impl Host, e: u32) -> u32 {
        match h.read::<u32>(e + 8) {
            6 => 0x18,
            8 => 0x24,
            _ => 0,
        }
    }

    /// `VfxShader_setClusterTexture_006c1280(cluster, &args)` and
    /// `VfxShader_setClusterCubeMap_006c1320(cluster, &args)` (frame: the
    /// texture iterator at `ebp - 0x30`, the parameter record at `ebp -
    /// 0x10`): `args` = {sampler kind, holder, param}. The cluster's first
    /// texture of the class (the iterator seeded with the cluster's
    /// `+0x1c` list) and the scene's linear sampler make the record
    /// {0xff, 0, sampler, texture} (cube: 0x44), written to the holder's
    /// `+4` buffer (2D: as a vector parameter; cube: only for a 0x44
    /// parameter, the sampler check first as in [`write_texture_parameter`]).
    /// The cube form's second argument slot ends up holding `param`.
    pub fn set_cluster_texture(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        cluster: u32,
        args: u32,
        cube: bool,
    ) -> u32 {
        h.call_sym(ASYNC_LOAD_MANAGER, Conv::Cdecl, &[]);
        let kind: u32 = h.read(args);
        let holder: u32 = h.read(args + 4);
        let param: u32 = h.read(args + 8);
        if cube {
            h.write(slots + 4, param);
        }
        let head = cluster + 0x1c;
        let first: u32 = h.read(head);
        let it = ebp - 0x30;
        h.write(it, 0_u32);
        let first = if first == head { 0 } else { first };
        for k in 1..5 {
            h.write(it + 4 * k, 0_u32);
        }
        h.write(it + 0x14, head);
        h.write(it + 0x18, first);
        let class = h.rebase(if cube {
            CLASS_TEXTURE_CUBE
        } else {
            CLASS_TEXTURE_2D
        });
        h.write(it + 0x1c, class);
        method(h, if cube { ITER_SEEK_CUBE } else { ITER_SEEK_2D }, it);
        let base: u32 = h.read(it + 0x10);
        let cursor: u32 = h.read(it);
        let tex = base.wrapping_add(cursor);
        let scene = SCENE.get(h);
        h.write(ebp - 0x10, if cube { 0x44_u32 } else { 0xff });
        h.write(ebp - 4, tex);
        h.write(ebp - 0xc, 0_u32);
        let sampler = h.call_sym(LINEAR_SAMPLER, Conv::Thiscall(scene), &[kind]);
        h.write(ebp - 8, sampler);
        let buffer: u32 = h.read(holder + 4);
        if !cube {
            return h.call_sym(
                WRITE_VECTOR_PARAMETER,
                Conv::Thiscall(buffer),
                &[param, ebp - 0x10],
            );
        }
        if h.read::<u8>(param + 2) != 0x44 {
            return sampler;
        }
        if sampler != 0 && tex != 0 {
            let r = h.call_sym(SAMPLER_CHECK, Conv::Thiscall(sampler), &[tex]);
            if r != 0 {
                return r;
            }
        }
        h.call_sym(
            WRITE_PARAMETER,
            Conv::Thiscall(buffer),
            &[param, ebp - 0x10],
        )
    }
}
