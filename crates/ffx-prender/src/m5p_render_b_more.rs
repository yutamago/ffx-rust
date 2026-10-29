//! M5 stage 1, list `phyre-render-b`, batch 5: medium methods of the
//! renderer (render-interface colour and compute bindings, the texture
//! buffer count, the render ring's job slots, the LOD lookup, the
//! dynamic-geometry blend), the character shader and lighting managers,
//! the bitmap font's text and kerning, and the post-processing geometry
//! and motion-blur target.
//!
//! Floats follow the x87 at 53-bit precision: each operation in `f64` in
//! the original's order, rounded to `f32` where the original stores.

use ffx_bridge::x87;

/// `Phyre_PhyreFree` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `Phyre_PSemaphoreWin32_wait` (thiscall).
pub const SEMAPHORE_WAIT: u32 = 0x0043_02b0;
/// `FullscreenMesh_createTriangle_00a05e70(cluster)`, `0x00a06ab0(cluster,
/// n)`, `0x00a04b20(cluster, n)` (cdecl): the lighting geometry.
pub const CREATE_TRIANGLE: u32 = 0x00a0_5e70;
pub const CREATE_SPHERE: u32 = 0x00a0_6ab0;
pub const CREATE_CONE: u32 = 0x00a0_4b20;
/// The texture class `0x00a17da0` accepts (`PTexture2D`'s descriptor).
pub const TEXTURE_2D_CLASS: u32 = 0x00ca_3364;
/// The kerning scale (a `double`).
pub const KERNING_SCALE: u32 = 0x00b5_1648;
/// The colour scale of `0x00599cc0` (a `double`, 1/255).
pub const COLOUR_SCALE: u32 = 0x00b3_4258;
/// `Phyre_PRendering_PTexture2DD3D11_unbind` (thiscall), `0x00599540(array;
/// n)` (thiscall, the buffers' resize), `PTexture2DD3D11_CreateDeviceTexture_0058fc20`
/// (thiscall, four words).
pub const UNBIND: u32 = 0x0059_cc40;
pub const BUFFERS_RESIZE: u32 = 0x0059_9540;
pub const CREATE_DEVICE_TEXTURE: u32 = 0x0058_fc20;
/// The light kinds `0x006a8440`/`0x006a8b40` accept (class descriptors).
pub const LIGHT_KIND_DIRECTIONAL: u32 = 0x00c1_0828;
pub const LIGHT_KIND_POINT: u32 = 0x00c1_0814;
/// The two constants `0x006a8bc0` adds and clamps with (`double`,
/// `float`).
pub const SHADE_OFFSET: u32 = 0x00b0_db90;
pub const SHADE_MIN: u32 = 0x00b3_eb44;
/// `Phyre_PFreeList_allocateNewBlock(list; n)`, `DynGeoTree_Insert(tree;
/// node)`, `Phyre_PFreeList_deallocate(list; node)`,
/// `Phyre_PRedBlackTree_postInsertFinalize(tree; node)` (thiscall).
pub const FREE_LIST_GROW: u32 = 0x0049_c8d0;
pub const TREE_INSERT: u32 = 0x006d_ee50;
pub const FREE_LIST_FREE: u32 = 0x0049_c980;
pub const TREE_FINALIZE: u32 = 0x0057_4980;
/// `Phyre_PCluster_create(cluster; count, class, flags)`,
/// `Phyre_PRendering_PTexture2DBase_setDimensions(texture; w, h, format, 0)`,
/// `Phyre_PRendering_PTexture2DBase_updateMaxLevel(texture)`,
/// `0x0064adf0(cluster; texture)` (thiscall), the render-target class and
/// the vtable `0x00a06a30` stores.
pub const CLUSTER_CREATE: u32 = 0x0044_1010;
pub const SET_DIMENSIONS: u32 = 0x004c_40c0;
pub const UPDATE_MAX_LEVEL: u32 = 0x004c_4250;
pub const MAKE_TARGET: u32 = 0x0064_adf0;
pub const TEXTURE_CLASS: u32 = 0x00ca_37d0;
pub const TARGET_VTABLE: u32 = 0x00b7_6848;
/// `PArray_Data_004d45a0(array)`, `ShaderParams_FindByName(shader;
/// name)` (thiscall), `crt_memcpy_thunk` (cdecl).
pub const ARRAY_DATA: u32 = 0x004d_45a0;
pub const FIND_PARAM: u32 = 0x0056_cbf0;
pub const MEMCPY: u32 = 0x0094_9256;
/// The IAT slot of `MSVCR110!strncmp` (cdecl).
pub const STRNCMP: u32 = 0x00b0_c3e4;
/// `PBitmapFontText_setCapacity_00a2a110(text; n)`,
/// `Phyre_PText_PBitmapFontText_createMeshGeometry(text; n)` (thiscall).
pub const TEXT_SET_CAPACITY: u32 = 0x00a2_a110;
pub const TEXT_CREATE_MESH: u32 = 0x00a2_9770;
/// The character shader map (a red-black tree), its free list, the
/// second tree and free list `resetCharacterShader` empties.
pub const SHADER_TREE: u32 = 0x00c3_4b4c;
pub const SHADER_TREE_FIRST: u32 = 0x00c3_4b58;
pub const SHADER_FREE_LIST: u32 = 0x00c3_4b64;
pub const SHADER_TREE_2: u32 = 0x00c3_4b1c;
pub const SHADER_FREE_LIST_2: u32 = 0x00c3_4b34;
/// `Phyre_PRedBlackTree_successor(tree; node)`, `_orphanAllContents`,
/// `Phyre_PFreeList_reset` (thiscall), and
/// `Phyre_CharacterShaderManager_restoreCharacterShader(inst)` (cdecl).
pub const TREE_SUCCESSOR: u32 = 0x0057_5160;
pub const TREE_ORPHAN_ALL: u32 = 0x0057_4970;
pub const FREE_LIST_RESET: u32 = 0x0049_cc10;
pub const RESTORE_SHADER: u32 = 0x006d_f6d0;
/// The UAV/SRV fetches of `0x0058ebb0` (thiscall, `(i)`).
pub const UAV_OF: u32 = 0x0059_3f90;
pub const SRV_OF: u32 = 0x0059_3d10;

/// A `float` through `fld dword; fstp dword` (a signalling NaN quieted).
fn quiet(bits: u32) -> u32 {
    x87::round_trip(f32::from_bits(bits)).to_bits()
}

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    fn sharray_data(h: &mut impl Host, at: u32, count: u32) -> u32 {
        if count > 1 {
            h.read(at + 4)
        } else if count != 0 {
            at + 4
        } else {
            0
        }
    }

    /// `Phyre_PPostProcessing_PDeferredLightingBase_initializeGeometry(effect;
    /// cluster)`: the triangle, the sphere (8) and the cone (16) meshes at
    /// `+0x3edc..0x3ee4`; 0.
    pub fn lighting_geometry(h: &mut impl Host, this: u32, cluster: u32) -> u32 {
        let t = cdecl(h, CREATE_TRIANGLE, &[cluster]);
        h.write(this + 0x3edc, t);
        let s = cdecl(h, CREATE_SPHERE, &[cluster, 8]);
        h.write(this + 0x3ee0, s);
        let c = cdecl(h, CREATE_CONE, &[cluster, 0x10]);
        h.write(this + 0x3ee4, c);
        0
    }

    /// `0x00614720(ring; out)`: after the semaphore (`+0x15c`), the next
    /// of the 16 20-byte job slots from `+0x10` (its first two words
    /// cleared) into `*out`, the cursor `+0x154` advanced (wrapping at
    /// 16); 0.
    pub fn next_slot(h: &mut impl Host, this: u32, out: u32) -> u32 {
        this_call(h, SEMAPHORE_WAIT, this + 0x15c, &[]);
        let i: u32 = h.read(this + 0x154);
        let slot = this.wrapping_add(0x10).wrapping_add(i.wrapping_mul(20));
        let next = i.wrapping_add(1);
        h.write(slot, [0u32; 2]);
        h.write(this + 0x154, if next == 16 { 0 } else { next });
        h.write(out, slot);
        0
    }

    /// `0x00a17da0(blur; cluster, target)`: 0x16 when the blur already
    /// has a target (`+0xd4c`); 5 unless the target holds a 2D texture;
    /// else the target kept and a fullscreen triangle made (`+0xcdc`): 0,
    /// or 0xe when that fails.
    pub fn blur_set_target(h: &mut impl Host, this: u32, cluster: u32, target: u32) -> u32 {
        if h.read::<u32>(this + 0xd4c) != 0 {
            return 0x16;
        }
        let tex = if h.read::<u8>(target + 4) == 0 {
            h.read(target)
        } else {
            0
        };
        let class = h.rebase(TEXTURE_2D_CLASS);
        if h.read::<u32>(tex) != class {
            return 5;
        }
        h.write(this + 0xd4c, target);
        let mesh = cdecl(h, CREATE_TRIANGLE, &[cluster]);
        h.write(this + 0xcdc, mesh);
        if mesh == 0 { 0xe } else { 0 }
    }

    /// `0x005a7410(lod)`: the first non-null entry of the levels' lists
    /// (`+0x40`, 0x24-byte levels, a `PSharray` at `+4` each), its word
    /// `+4`; 0 when there is none.
    pub fn lod_first(h: &mut impl Host, this: u32) -> u32 {
        let levels = h.read::<u32>(this + 0x3c) & 0x7fff_ffff;
        let base: u32 = h.read(this + 0x40);
        for l in 0..levels {
            let at = base.wrapping_add(4).wrapping_add(l.wrapping_mul(0x24));
            let n = h.read::<u32>(at) & 0x7fff_ffff;
            for j in 0..n {
                let data = sharray_data(h, at, n);
                let e: u32 = h.read(data.wrapping_add(4 * j));
                if e != 0 {
                    return h.read(e + 4);
                }
            }
        }
        0
    }

    /// `Phyre_PText_PBitmapFontText_getKerning_00a29700(text; pair,
    /// glyph)` -> `ST0`: the `[pair + 4]` 12-byte kerning entries from
    /// word `[pair + 8]` of the font's table (`[[[text + 0x158] + 0x30] +
    /// 0x1c]`; the start is a word index, the step 12 bytes) searched for
    /// `*glyph`; its amount times the kerning scale, or 0.
    pub fn kerning(h: &mut impl Host, this: u32, pair: u32, glyph: u32) -> f32 {
        let n: u32 = h.read(pair + 4);
        if (n as i32) <= 0 {
            return 0.0;
        }
        let first: u32 = h.read(pair + 8);
        let key: u32 = h.read(glyph);
        let font: u32 = h.read(this + 0x158);
        let face: u32 = h.read(font + 0x30);
        let table: u32 = h.read(face + 0x1c);
        let start = table.wrapping_add(first.wrapping_mul(4));
        for i in 0..n {
            let e = start.wrapping_add(i.wrapping_mul(12));
            let amount: u32 = h.read(e + 4);
            if h.read::<u32>(e) == key {
                // fild, fstp dword, fld, fmul qword, fstp dword
                let v = f64::from(amount as i32) as f32;
                let scale: f64 = h.read(h.rebase(KERNING_SCALE));
                return (f64::from(v) * scale) as f32;
            }
        }
        0.0
    }

    /// `0x005cb8d0(weights, n, sources, out)` (cdecl): `n` floats of
    /// `w0 a + w1 b + w2 c` from the three source streams, in that order
    /// at 53-bit precision, each rounded to `f32`, to `*out`. (`n` is
    /// never 0 at the callers; the original would run 2^32 times.)
    pub fn blend3(h: &mut impl Host, weights: u32, n: u32, sources: u32, out: u32) {
        let w = [0, 4, 8].map(|k| x87::load(quiet(h.read(weights + k))));
        let s: [u32; 3] = [0, 4, 8].map(|k| h.read(sources + k));
        let dst: u32 = h.read(out);
        for i in 0..n {
            let at = |p: u32| p.wrapping_add(4 * i);
            let a = x87::load(h.read(at(s[0])));
            let b = x87::load(h.read(at(s[1])));
            let c = x87::load(h.read(at(s[2])));
            let ab = x87::add(x87::mul(a, w[0]), x87::mul(b, w[1]));
            let v = x87::add(ab, x87::mul(c, w[2]));
            h.write(dst.wrapping_add(4 * i), x87::store(v));
        }
    }

    /// `PTexture2DD3D11_setBufferCount_00599b70(texture; n)`: a bound
    /// texture (`+0x70`) unbound first (its error returned); the buffers
    /// resized (`+0x24`, its error returned); a texture without a first
    /// buffer gets its device texture (`(0, 1, bit 10 of +0x14, 0)`);
    /// `+0x70` = 1; 0.
    pub fn set_buffer_count(h: &mut impl Host, this: u32, n: u32) -> u32 {
        if h.read::<u32>(this + 0x70) != 0 {
            let r = this_call(h, UNBIND, this, &[]);
            if r != 0 {
                return r;
            }
        }
        let r = this_call(h, BUFFERS_RESIZE, this + 0x24, &[n]);
        if r != 0 {
            return r;
        }
        let count = h.read::<u32>(this + 0x24) & 0x7fff_ffff;
        let data = sharray_data(h, this + 0x24, count);
        if h.read::<u32>(data) == 0 {
            let bit = (h.read::<u32>(this + 0x14) >> 10) & 1;
            this_call(h, CREATE_DEVICE_TEXTURE, this, &[0, 1, bit, 0]);
        }
        h.write(this + 0x70, 1u32);
        0
    }

    /// The light record `i` of the manager when it is set up (byte
    /// `+0x1cc`) and of `kind` (the record read before the set-up test,
    /// as `0x006a8440` does).
    fn light_of(h: &mut impl Host, this: u32, i: u32, kind: u32) -> Option<u32> {
        let lights: u32 = h.read(this + 0xf0);
        let list: u32 = h.read(lights + 4);
        let e: u32 = h.read(list.wrapping_add(i.wrapping_mul(4)));
        let kind = h.rebase(kind);
        if h.read::<u8>(this + 0x1cc) != 0 && h.read::<u32>(e + 0x18) == kind {
            Some(e)
        } else {
            None
        }
    }

    /// `0x006a8440(lighting; out, i)`: light 3 is the ambient colour
    /// (`+0x190..0x19c`); a directional light `i` gives its first four
    /// words; anything else 0.0 x 4; `out`.
    pub fn light_colour(h: &mut impl Host, this: u32, out: u32, i: u32) -> u32 {
        if i == 3 {
            let v: [u32; 4] = h.read(this + 0x190);
            h.write(out, v);
            return out;
        }
        if let Some(e) = light_of(h, this, i, LIGHT_KIND_DIRECTIONAL) {
            let v: [u32; 4] = h.read(e);
            h.write(out, v);
        } else {
            h.write(out, [0u32; 4]);
        }
        out
    }

    /// `0x006a8b40(lighting; out, i)`: a point light `i` gives the
    /// position of its node (`[e + 0x10] + 0x20..0x28`) with w = 1;
    /// anything else (0, 0, 0, 1); `out`. The set-up test comes first.
    pub fn light_position(h: &mut impl Host, this: u32, out: u32, i: u32) -> u32 {
        let one = 1f32.to_bits();
        let set_up = h.read::<u8>(this + 0x1cc) != 0;
        if set_up && let Some(e) = light_of(h, this, i, LIGHT_KIND_POINT) {
            let node: u32 = h.read(e + 0x10);
            let p = [0x20, 0x24, 0x28].map(|k| quiet(h.read(node + k)));
            h.write(out, [p[0], p[1], p[2], one]);
        } else {
            h.write(out, [0, 0, 0, one]);
        }
        out
    }

    /// `0x006a8bc0(tw)` -> `ST0`: the four shade coefficients
    /// (`+0x1d0..0x1dc`) summed with the offset (each sum rounded to
    /// `f32`), at most 1, at least the minimum (a NaN stays).
    pub fn shade_sum(h: &mut impl Host, this: u32) -> f32 {
        let offset: f64 = h.read(h.rebase(SHADE_OFFSET));
        let k: [u32; 4] = [0x1d0, 0x1d4, 0x1d8, 0x1dc].map(|at| h.read(this + at));
        let mut s = x87::store(x87::add(x87::load(k[0]), offset));
        for &v in &k[1..] {
            s = x87::store(x87::add(x87::load(s), x87::load(v)));
        }
        let s = x87::load(s);
        let min = x87::load(h.read(h.rebase(SHADE_MIN)));
        let r = if s > 1.0 {
            1.0
        } else if min > s {
            min
        } else {
            s
        };
        f32::from_bits(x87::store(r))
    }

    /// `0x006df340(manager; key, value)`: a node from the free list
    /// (`+0x18`, grown by `[+0x20]` when empty; 0xe when still empty)
    /// linked to itself with `*key` and `*value`, inserted in the tree:
    /// on a clash the node goes back and 0x12; else the tree is fixed up,
    /// 0.
    pub fn shader_insert(h: &mut impl Host, this: u32, key: u32, value: u32) -> u32 {
        let list = this + 0x18;
        let mut node: u32 = h.read(list);
        if node == 0 {
            let n: u32 = h.read(this + 0x20);
            this_call(h, FREE_LIST_GROW, list, &[n]);
            node = h.read(list);
            if node == 0 {
                return 0xe;
            }
        }
        let next: u32 = h.read(node);
        h.write(list, next);
        h.write(node, [node, node, node]);
        let k: u32 = h.read(key);
        h.write(node + 0xc, k);
        let v: u32 = h.read(value);
        h.write(node + 0x10, v);
        if this_call(h, TREE_INSERT, this, &[node]) != 0 {
            this_call(h, FREE_LIST_FREE, list, &[node]);
            return 0x12;
        }
        this_call(h, TREE_FINALIZE, this, &[node]);
        0
    }

    /// `0x00a06a30(cluster, w, h, format, flag1, flag2, out)` (cdecl): a
    /// texture created in the cluster (`*out` = 0, then the dimensions'
    /// result; `out` may be null), its flag byte `+5`, `+0x14` bit 1
    /// cleared and its levels updated; a render target made for it with
    /// the second flag and the vtable; the target, or 0.
    pub fn make_target(h: &mut impl Host, args: [u32; 7], slot: u32) -> u32 {
        let [cluster, w, hgt, format, flag1, flag2, out] = args;
        let out = if out == 0 { slot } else { out };
        h.write(out, 0u32);
        let class = h.rebase(TEXTURE_CLASS);
        let tex = this_call(h, CLUSTER_CREATE, cluster, &[1, class, 0]);
        if tex == 0 {
            return 0;
        }
        let r = this_call(h, SET_DIMENSIONS, tex, &[w, hgt, format, 0]);
        h.write(out, r);
        if r != 0 {
            return 0;
        }
        let flags: u32 = h.read(tex + 0x14);
        h.write(tex + 0x14, flags & !2);
        h.write(tex + 5, flag1 as u8);
        this_call(h, UPDATE_MAX_LEVEL, tex, &[]);
        let target = this_call(h, MAKE_TARGET, cluster, &[tex]);
        if target == 0 {
            return 0;
        }
        h.write(target + 5, flag2 as u8);
        let vt = h.rebase(TARGET_VTABLE);
        h.write(target + 0xc, vt);
        target
    }

    /// `0x00599cc0(state; argb)`: the colour's bytes as floats times
    /// 1/255 to `+0x37c..0x388` (r, g, b, a).
    pub fn set_colour(h: &mut impl Host, this: u32, argb: u32) {
        let scale: f64 = h.read(h.rebase(COLOUR_SCALE));
        for (at, shift) in [(0x37c, 16), (0x380, 8), (0x384, 0), (0x388, 24)] {
            let v = f64::from((argb >> shift) & 0xff);
            h.write(this + at, (v * scale) as f32);
        }
    }

    /// `Phyre_CharacterShaderManager_setShaderParameter(inst, name,
    /// data)` (cdecl): for each material of the instance's list (`+0x10`)
    /// the named parameter's bytes (offset `+8`, size `+0xa` & 0x1fff,
    /// offset at least 4, within the buffer's size word) copied from
    /// `data`.
    pub fn set_shader_parameter(h: &mut impl Host, inst: u32, name: u32, data: u32) {
        let arr: u32 = h.read(inst + 0x10);
        let count = h.read::<u32>(arr) & 0x7fff_ffff;
        for i in 0..count {
            let list = this_call(h, ARRAY_DATA, arr, &[]);
            let e: u32 = h.read(list.wrapping_add(4 * i));
            let buffer: u32 = h.read(e + 4);
            let material: u32 = h.read(e);
            let p = this_call(h, FIND_PARAM, material, &[name]);
            if p == 0 || data == 0 {
                continue;
            }
            let off = u32::from(h.read::<u16>(p + 8));
            let size = u32::from(h.read::<u16>(p + 0xa) & 0x1fff);
            if off.wrapping_add(size) > h.read::<u32>(buffer) || off < 4 {
                continue;
            }
            cdecl(h, MEMCPY, &[buffer.wrapping_add(off), data, size]);
        }
    }

    /// `0x009eccb0(array; name)`: the first object of the `PArray` whose
    /// name (`[object + 4]`, its bit 0 a flag) equals `name`; 0.
    pub fn find_by_name(h: &mut impl Host, this: u32, name: u32) -> u32 {
        let count = h.read::<u32>(this) & 0x7fff_ffff;
        let data: u32 = h.read(this + 4);
        for i in 0..count {
            let obj: u32 = h.read(data.wrapping_add(4 * i));
            let word: u32 = h.read(obj + 4);
            let s = word.wrapping_sub(u32::from(h.read::<u8>(obj + 4) & 1));
            let mut k = 0u32;
            loop {
                let a: u8 = h.read(name.wrapping_add(k));
                let b: u8 = h.read(s.wrapping_add(k));
                if a != b {
                    break;
                }
                if a == 0 {
                    return obj;
                }
                k = k.wrapping_add(1);
            }
        }
        0
    }

    /// `0x00a2aa30(text; s)`: 1 for a null string; the text set to `s`
    /// (`strncmp` against the buffer at `+0x160` when it fits the
    /// capacity `+0x15c`, nothing to do when equal; else the capacity
    /// grown, its error returned), copied with its terminator and the mesh
    /// rebuilt (its `eax`).
    pub fn set_text(h: &mut impl Host, this: u32, s: u32) -> u32 {
        if s == 0 {
            return 1;
        }
        let mut len = 0u32;
        while h.read::<u8>(s.wrapping_add(len)) != 0 {
            len = len.wrapping_add(1);
        }
        let buffer: u32 = h.read(this + 0x160);
        let cap = (h.read::<u32>(this + 0x15c) & 0x7fff_ffff).wrapping_sub(1);
        if buffer != 0 && len <= cap {
            let f: u32 = h.read(h.rebase(STRNCMP));
            if h.call(f, Conv::Cdecl, &[buffer, s, len + 1]) == 0 {
                return 0;
            }
        } else {
            let r = this_call(h, TEXT_SET_CAPACITY, this, &[len]);
            if r != 0 {
                return r;
            }
        }
        let buffer: u32 = h.read(this + 0x160);
        cdecl(h, MEMCPY, &[buffer, s, len + 1]);
        this_call(h, TEXT_CREATE_MESH, this, &[len])
    }

    /// `Phyre_CharacterShaderManager_resetCharacterShader()`: every
    /// instance in the shader map restored (in the tree's order from the
    /// last node of the `+0xc` chain), then both trees orphaned and both
    /// free lists reset (the last a tail call: its `eax`).
    pub fn reset_shaders(h: &mut impl Host) -> u32 {
        let tree = h.rebase(SHADER_TREE);
        let first_at = h.rebase(SHADER_TREE_FIRST);
        let first: u32 = h.read(first_at);
        let mut node = if first == tree || first == 0 {
            first
        } else {
            let mut cur = first;
            loop {
                let next: u32 = h.read(cur);
                if next == tree {
                    break cur;
                }
                cur = next;
            }
        };
        if node != tree {
            let mut next = this_call(h, TREE_SUCCESSOR, tree, &[node]);
            while node != tree {
                let inst: u32 = h.read(node + 0xc);
                cdecl(h, RESTORE_SHADER, &[inst]);
                node = next;
                if next != tree {
                    next = this_call(h, TREE_SUCCESSOR, tree, &[next]);
                }
            }
        }
        this_call(h, TREE_ORPHAN_ALL, tree, &[]);
        let list = h.rebase(SHADER_FREE_LIST);
        this_call(h, FREE_LIST_RESET, list, &[]);
        let tree2 = h.rebase(SHADER_TREE_2);
        this_call(h, TREE_ORPHAN_ALL, tree2, &[]);
        let list2 = h.rebase(SHADER_FREE_LIST_2);
        this_call(h, FREE_LIST_RESET, list2, &[])
    }

    /// `0x0058ebb0(state; srv, uav)`: 5 unless the UAV (`+8` bits 1-2) and
    /// the buffer (`+8` bit 3, stride 4, at least 4 bytes) fit; else the
    /// context binds the UAV (method `+0xc4`) and the SRV (method
    /// `+0xbc`) at the state's slot `+0x378`; 0.
    pub fn bind_compute_buffer(h: &mut impl Host, this: u32, buf: u32, uav: u32) -> u32 {
        if h.read::<u8>(uav + 8) & 6 == 0 {
            return 5;
        }
        if h.read::<u8>(buf + 8) & 8 == 0 || h.read::<u32>(buf) < 4 || h.read::<u32>(buf + 4) != 4 {
            return 5;
        }
        let ctx: u32 = h.read(this + 0x70);
        let vt: u32 = h.read(ctx);
        let i: u32 = h.read(uav + 0xc);
        let v = this_call(h, UAV_OF, uav, &[i]);
        let slot: u32 = h.read(this + 0x378);
        let f: u32 = h.read(vt + 0xc4);
        h.call(f, Conv::Stdcall, &[ctx, slot, 0, v]);
        let slot: u32 = h.read(this + 0x378);
        let ctx: u32 = h.read(this + 0x70);
        let vt: u32 = h.read(ctx);
        let i: u32 = h.read(buf + 0xc);
        let w = this_call(h, SRV_OF, buf, &[i]);
        let f: u32 = h.read(vt + 0xbc);
        h.call(f, Conv::Stdcall, &[ctx, w, slot]);
        0
    }
}
