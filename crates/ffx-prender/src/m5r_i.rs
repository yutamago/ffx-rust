//! M5 stage 1, list `render`, batch 9: the scene's unique-add and DTT
//! VFX lists, the tagged-string tree nodes' constructors, the texture
//! animation info's destructor, the specular textures, the float sort's
//! median of nine, the dyn-geo manager's creation and a sampler-pair
//! destructor. Twins of earlier ports are hooked onto those bridges.
//!
//! SEH frames and cookies are left out: nothing on these paths throws.

use ffx_bridge::Global;

/// `g_DynGeoFillBuffer`.
pub const DYN_GEO_FILL: Global<u32> = Global::new(0x00cd_edf0);

pub const USER_MALLOC: u32 = 0x0068_7020;
pub const USER_FREE: u32 = 0x0068_7010;
pub const MEMCPY: u32 = 0x0094_9256;
pub const RESERVE_4: u32 = 0x0066_8100;
pub const PHYRE_MALLOC_ALIGNED: u32 = 0x0042_fb80;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const STRING_NODE_CTOR: u32 = 0x0064_cba0;
/// `TextureAnimationInfo_vftable`.
pub const VT_TEXANIM_INFO: u32 = 0x00b4_8420;
pub const FIND_PARAM: u32 = 0x0056_cbf0;
pub const SET_TEXTURE_CHECKED: u32 = 0x0056_e670;
/// The two specular textures' parameter names.
pub const SPECULAR_TEX_A: u32 = 0x00b4_95cc;
pub const SPECULAR_TEX_B: u32 = 0x00b4_7ed8;
pub const MEDIAN3: u32 = 0x006c_74d0;
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const CREATE_MUTEX_W: u32 = 0x00b0_c198;
pub const CREATE_POOL: u32 = 0x006d_e720;
pub const SAMPLER_DTOR: u32 = 0x0058_5230;
/// `TaggedTreeInsert_00691e60`, the insert of the tree add twin
/// `0x00693c50`.
pub const TREE_INSERT_691E60: u32 = 0x0069_1e60;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn method(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    /// `PhyreScene_addUnique*(scene; v)` (`ret 4`) for the word array at
    /// `scene + base` (count `+0`, capacity `+4`, data `+8`):
    /// `PhyreScene_addUniqueToArray386c_00656a00`'s code, `v` appended
    /// unless there (grown to `n + n / 8 + 2`, `user_malloc` / `memcpy` /
    /// `user_free`); `eax` the entry found or the new entry's address.
    pub fn add_unique(h: &mut impl Host, scene: u32, base: u32, v: u32) -> u32 {
        let a = scene + base;
        let count = word(h, a);
        let data = word(h, a + 8);
        for k in 0..count {
            let at = data.wrapping_add(4 * k);
            if word(h, at) == v {
                return at;
            }
        }
        let count = count.wrapping_add(1);
        h.write(a, count);
        if count.cast_signed() > word(h, a + 4).cast_signed() {
            let n = count.cast_signed();
            let cap = (n / 8 + 2).wrapping_add(n);
            let old = word(h, a + 8);
            if old == 0 {
                let p = cdecl(h, USER_MALLOC, &[cap.cast_unsigned() << 2]);
                h.write(a + 8, p);
            } else {
                let mut p = 0;
                if cap > 0 {
                    p = cdecl(h, USER_MALLOC, &[cap.cast_unsigned() << 2]);
                    if p != 0 {
                        let n = word(h, a);
                        let old = word(h, a + 8);
                        cdecl(h, MEMCPY, &[p, old, n << 2]);
                    }
                }
                let old = word(h, a + 8);
                cdecl(h, USER_FREE, &[old]);
                h.write(a + 8, p);
            }
            h.write(a + 4, cap.cast_unsigned());
        }
        let n = word(h, a);
        let data = word(h, a + 8);
        let at = data.wrapping_add(n.wrapping_mul(4)).wrapping_sub(4);
        if at != 0 {
            h.write(at, v);
        }
        at
    }

    /// `PhyreScene_AddVFXToDTTList(scene; vfx)` (`ret 4`): `vfx` appended
    /// to the 4-byte array `+0x3854` unless there (an empty array always
    /// appends; grown by `Array_Reserve_00668100` to `count + 2 + count /
    /// 8`); `eax` the index found, or the new slot (0 when null).
    pub fn add_vfx_to_dtt(h: &mut impl Host, scene: u32, vfx: u32) -> u32 {
        let a = scene + 0x3854;
        let n = word(h, a);
        if n.cast_signed() > 0 {
            let data = word(h, a + 8);
            if let Some(i) = (0..n).find(|&i| word(h, data + 4 * i) == vfx) {
                return i;
            }
        }
        let c = n.wrapping_add(1);
        h.write(a, c);
        if c.cast_signed() > word(h, a + 4).cast_signed() {
            let grown = c
                .wrapping_add(2)
                .wrapping_add((c.cast_signed() / 8).cast_unsigned());
            method(h, RESERVE_4, a, &[grown]);
        }
        let c = word(h, a);
        let slot = word(h, a + 8)
            .wrapping_add(c.wrapping_mul(4))
            .wrapping_sub(4);
        if slot != 0 {
            h.write(slot, vfx);
        }
        slot
    }

    /// The tagged string `[src]` (less its tag bit) copied into a new
    /// block (`Phyre_PhyreMalloc_0042fb80(len + 1, 2)`, `memcpy`); 0 for
    /// none or a failed allocation.
    fn dup_tagged(h: &mut impl Host, src: u32) -> u32 {
        let w = word(h, src);
        let s = w.wrapping_sub(w & 1);
        if s == 0 {
            return 0;
        }
        let mut len = 0_u32;
        while h.read::<u8>(s.wrapping_add(len)) != 0 {
            len += 1;
        }
        let p = cdecl(h, PHYRE_MALLOC_ALIGNED, &[len + 1, 2]);
        if p == 0 {
            return 0;
        }
        cdecl(h, MEMCPY, &[p, s, len + 1]);
        p
    }

    /// `StringTreeNode_ctor_0064cba0(this; name)` (`ret 4`) -> `this`: the
    /// links to itself, the copied name at `+0xc`.
    pub fn string_node_ctor(h: &mut impl Host, this: u32, name: u32) -> u32 {
        h.write(this, this);
        h.write(this + 4, this);
        h.write(this + 8, this);
        let p = dup_tagged(h, name);
        h.write(this + 0xc, p);
        this
    }

    /// `StringPairTreeNode_ctor_0064cb00(this; key, value)` (`ret 8`) ->
    /// `this`: [`string_node_ctor`] with `key`, the copied value at
    /// `+0x10`.
    pub fn string_pair_node_ctor(h: &mut impl Host, this: u32, key: u32, value: u32) -> u32 {
        method(h, STRING_NODE_CTOR, this, &[key]);
        let p = dup_tagged(h, value);
        h.write(this + 0x10, p);
        this
    }

    /// `TextureAnimationInfo_dtor_006af760(this)`: the vtable stored, the
    /// owned array `+8` (data `+0xc`) freed when set and cleared, the
    /// tagged buffer `+4` freed unless its tag bit is set.
    pub fn texanim_info_dtor(h: &mut impl Host, this: u32) {
        let vt = h.rebase(VT_TEXANIM_INFO);
        h.write(this, vt);
        if word(h, this + 8) >> 31 == 0 {
            let d = word(h, this + 0xc);
            if d != 0 {
                cdecl(h, PHYRE_FREE, &[d]);
            }
        }
        h.write(this + 0xc, 0_u32);
        h.write(this + 8, 0_u32);
        if h.read::<u8>(this + 4) & 1 == 0 {
            let p = word(h, this + 4);
            cdecl(h, PHYRE_FREE, &[p]);
        }
    }

    /// `SpecularMapParams_setTextures_006c5570(this; a, b)` (`ret 8`;
    /// frame: the 16-byte texture record at `ebp - 0x10`, {0x42, 0, the
    /// parameter buffer `+0x68`, the texture}): each texture (when set)
    /// whose parameter the material `+0x64` has goes into the buffer
    /// (`setTextureChecked` on `[+0x60] + 4`).
    pub fn set_specular_textures(h: &mut impl Host, ebp: u32, this: u32, a: u32, b: u32) {
        for (tex, name) in [(a, SPECULAR_TEX_A), (b, SPECULAR_TEX_B)] {
            if tex == 0 {
                continue;
            }
            let params = word(h, this + 0x64);
            let name = h.rebase(name);
            let p = method(h, FIND_PARAM, params, &[name]);
            if p == 0 {
                continue;
            }
            let rec = ebp - 0x10;
            h.write(rec + 8, this + 0x68);
            h.write(rec, 0x42_u32);
            h.write(rec + 0xc, tex);
            h.write(rec + 4, 0_u32);
            let buf = word(h, word(h, this + 0x60) + 4);
            method(h, SET_TEXTURE_CHECKED, buf, &[p, rec]);
        }
    }

    /// `FloatSort_Median9_006c7560(first, mid, last, pred)`: more than 40
    /// floats: the median of three on three spread triples and of their
    /// medians (`FloatSort_Median3`), the step `(n + 1) / 8` (signed) in
    /// floats stored into its first argument slot (`slot`, as bytes times
    /// two); otherwise one median of three.
    pub fn median9(
        h: &mut impl Host,
        slot: u32,
        first: u32,
        mid: u32,
        last: u32,
        pred: u32,
    ) -> u32 {
        let n = (last.wrapping_sub(first).cast_signed() >> 2).cast_unsigned();
        if n.cast_signed() <= 0x28 {
            return cdecl(h, MEDIAN3, &[first, mid, last, pred]);
        }
        let step = (n.wrapping_add(1).cast_signed() / 8).cast_unsigned();
        let s4 = step.wrapping_mul(4);
        let s8 = step.wrapping_mul(8);
        h.write(slot, s8);
        let m1 = first.wrapping_add(s4);
        cdecl(h, MEDIAN3, &[first, m1, first.wrapping_add(s8), pred]);
        cdecl(
            h,
            MEDIAN3,
            &[mid.wrapping_sub(s4), mid, mid.wrapping_add(s4), pred],
        );
        let l1 = last.wrapping_sub(s4);
        cdecl(h, MEDIAN3, &[last.wrapping_sub(s8), l1, last, pred]);
        cdecl(h, MEDIAN3, &[m1, mid, l1, pred])
    }

    /// `DynGeoMemManager_createInstance_006deb90()`: without one, a
    /// 0x10-byte manager (`operator_new`) with a mutex (`CreateMutexW(0,
    /// 0, 0)` at `+0xc`), `+0`, `+4`, `+8` cleared, pool 0 made
    /// (`createPool(0)`) and `+0 = +4`, becomes `g_DynGeoFillBuffer` (0
    /// when the allocation fails).
    pub fn dyn_geo_create(h: &mut impl Host) {
        if DYN_GEO_FILL.get(h) != 0 {
            return;
        }
        let m = cdecl(h, OPERATOR_NEW, &[0x10]);
        if m != 0 {
            let f: u32 = h.read(h.rebase(CREATE_MUTEX_W));
            let mx = h.call(f, Conv::Stdcall, &[0, 0, 0]);
            h.write(m + 0xc, mx);
            h.write(m, 0_u32);
            h.write(m + 4, 0_u32);
            h.write(m + 8, 0_u32);
            method(h, CREATE_POOL, m, &[0]);
            let v = word(h, m + 4);
            h.write(m, v);
        }
        DYN_GEO_FILL.set(h, m);
    }

    /// `SamplerPair_dtor_00692680`/`_006927a0(this)`: the samplers `+0x24`
    /// and `+0` destroyed; the last destructor's `eax`.
    pub fn sampler_pair_dtor(h: &mut impl Host, this: u32) -> u32 {
        method(h, SAMPLER_DTOR, this + 0x24, &[]);
        method(h, SAMPLER_DTOR, this, &[])
    }
}
