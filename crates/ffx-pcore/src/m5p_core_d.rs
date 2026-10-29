//! M5 stage 1, list `phyre-core`, batch 4: maths and small utilities.
//!
//! - `PWorldMatrix`'s look-at and 3x4 copy, the Euler rotation matrix,
//!   the 3-vector lerp (x87 at 53 bits: f64 in the original's order,
//!   rounded where it stores);
//! - the character lighting's added-light parameter writes (shader
//!   parameters found by name, copied out of the frame);
//! - the shadow caster type's unlink, the red-black tree's node count,
//!   byte-swaps, a `PSharray` resize, the LOD group's level switches.

/// `PMatrix4_FromMatrix3x4_00451080(out; m)`, `PMatrix4_lookAt(m; eye,
/// target, up)` and `PMatrix4_toWorldMatrix3x4(dst; m)` (thiscall).
pub const FROM_3X4: u32 = 0x0045_1080;
pub const LOOK_AT: u32 = 0x0045_1c30;
pub const TO_WORLD_MATRIX: u32 = 0x0045_1190;
/// The shadow caster types' list head (links at `+8`).
pub const SHADOW_CASTER_TYPES: u32 = 0x00ca_43a8;
/// `_CIsin` / `_CIcos`'s import thunks (`ST0` in and out).
pub const CI_SIN: u32 = 0x0094_98c8;
pub const CI_COS: u32 = 0x0094_98c2;
/// `Phyre_PRedBlackTree_successor_005751d0(tree; node)` (thiscall).
pub const TREE_SUCCESSOR: u32 = 0x0057_51d0;
/// `Phyre_PBase_GetClassDescriptor()` (cdecl) and `Phyre_PhyreFree(p)`.
pub const PBASE_DESCRIPTOR: u32 = 0x0043_5740;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `PSharray<T>`'s reallocator `0x0058c590(this; n, storage, keep)`
/// (thiscall, `ret 0xc`).
pub const SHARRAY_REALLOC: u32 = 0x0058_c590;
/// The character lighting: the shade coefficient (`ST0`, `ecx` the
/// lighting), its store `Phyre_CharacterLighting_setShadeCoeff(cl; inst,
/// coeff)`, the intensity scale (a double), `PArray_Data(array)`, the
/// shader's parameter lookup `(shader; name)` (thiscall), the CRT's
/// `memcpy`, and the parameter names (`AddLgtClr`, `Shininess`,
/// `MaterialColour`).
pub const SHADE_COEFF: u32 = 0x006a_8c40;
pub const SET_SHADE_COEFF: u32 = 0x006a_aef0;
pub const INTENSITY_SCALE: u32 = 0x00b4_8318;
pub const ARRAY_DATA: u32 = 0x004d_45a0;
pub const FIND_PARAMETER: u32 = 0x0056_cbf0;
pub const MEMCPY: u32 = 0x0094_9256;
pub const ADD_LIGHT_COLOUR: u32 = 0x00b4_8108;
pub const SHININESS: u32 = 0x00b4_8114;
pub const MATERIAL_COLOUR: u32 = 0x00b4_1980;
/// The LOD group's members: the levels' reset `0x005a7990(group)` and a
/// level's owner store `0x005a83a0(level; flag)` (thiscall).
pub const LOD_LEVELS_RESET: u32 = 0x005a_7990;
pub const LOD_LEVEL_OWNERS: u32 = 0x005a_83a0;

/// The byte-swap of a word, and what the originals leave in `eax` after
/// swapping `x` (the swap's high half before its last `or`).
#[must_use]
pub const fn swap_and_eax(x: u32) -> (u32, u32) {
    let hi = ((x & 0xff00) | (x << 16)) << 8;
    let lo = ((x & 0x00ff_0000) | (x >> 16)) >> 8;
    (hi | lo, hi)
}

/// `PMatrix4_Copy_00453340(out; m)` and `Phyre_PString_Assign(this;
/// text)` (thiscall).
pub const MATRIX_COPY: u32 = 0x0045_3340;
pub const STRING_ASSIGN: u32 = 0x0044_5cf0;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn thiscall(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// The float at `at` as `fld dword` loads it.
    fn ld(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    /// Stores `x` as `fstp dword` does; returns the stored value.
    fn st(h: &mut impl Host, at: u32, x: f64) -> f64 {
        let w = x87::store(x);
        h.write(at, w);
        x87::load(w)
    }

    /// `x` rounded to `f32` (an `fstp dword` into a local), as loaded back.
    fn round(x: f64) -> f64 {
        x87::load(x87::store(x))
    }

    /// Copies the float at `from` to `to` (`fld`/`fstp dword`).
    fn copy_float(h: &mut impl Host, from: u32, to: u32) {
        let v: u32 = h.read(from);
        h.write(to, x87::store(x87::load(v)));
    }

    /// `PWorldMatrix`'s look-at `0x00452520(this; &m)` ->
    /// `PMatrix4_toWorldMatrix3x4`'s `eax`: the world matrix as a 4x4 in
    /// the frame at `frame` (0x70 bytes, the original's `ebp - 0x70`),
    /// then `PMatrix4_lookAt` from its own position (row 3) at `m`'s row 2
    /// (`+0x20`) with its row 1 as up, back into `this`.
    pub fn world_look_at(h: &mut impl Host, this: u32, m: u32, frame: u32) -> u32 {
        let local = frame;
        thiscall(h, FROM_3X4, local, &[this]);
        let up = frame + 0x60;
        let target = frame + 0x50;
        let eye = frame + 0x40;
        for k in 0..3 {
            copy_float(h, local + 0x10 + 4 * k, up + 4 * k);
        }
        for k in 0..3 {
            copy_float(h, m + 0x20 + 4 * k, target + 4 * k);
        }
        for k in 0..3 {
            copy_float(h, local + 0x30 + 4 * k, eye + 4 * k);
        }
        thiscall(h, LOOK_AT, local, &[eye, target, up]);
        thiscall(h, TO_WORLD_MATRIX, this, &[local])
    }

    /// `PWorldMatrix`'s 3x4 copy `0x00452740(this; &m)` -> `m` (the
    /// original leaves its argument in `eax`): twelve floats.
    pub fn copy_3x4(h: &mut impl Host, this: u32, m: u32) -> u32 {
        for k in 0..12 {
            copy_float(h, m + 4 * k, this + 4 * k);
        }
        m
    }

    /// `0x004cc050(type)` -> 0: unlinks `type` from the shadow caster
    /// types' list (links at `+8`) when it is on it.
    pub fn shadow_type_unlink(h: &mut impl Host, ty: u32) -> u32 {
        let mut slot = h.rebase(SHADOW_CASTER_TYPES);
        let mut node: u32 = h.read(slot);
        while node != 0 {
            if node == ty {
                let next: u32 = h.read(node.wrapping_add(8));
                h.write(slot, next);
                h.write(node.wrapping_add(8), 0u32);
                return 0;
            }
            slot = node.wrapping_add(8);
            node = h.read(slot);
        }
        0
    }

    /// `Phyre_Vec3Lerp_006a9b40(out, t, &a, &b)` -> `out`: `a + t (b -
    /// a)` per component, each difference and product stored as `f32`.
    pub fn lerp3(h: &mut impl Host, out: u32, t: u32, a: u32, b: u32) -> u32 {
        let t = x87::load(t);
        let d: [f64; 3] = core::array::from_fn(|k| {
            let k = k as u32 * 4;
            round(x87::sub(ld(h, b + k), ld(h, a + k)))
        });
        let s: [f64; 3] = core::array::from_fn(|k| round(x87::mul(d[k], t)));
        let x = x87::add(ld(h, a), s[0]);
        st(h, out, x);
        let y = x87::add(ld(h, a + 4), s[1]);
        st(h, out + 4, y);
        let z = x87::add(s[2], ld(h, a + 8));
        st(h, out + 8, z);
        out
    }

    /// The rotation matrix of the Euler angles at `angles` (`0x00574520(out,
    /// &angles)` -> `out`): sines and cosines through `_CIsin`/`_CIcos`
    /// (each stored as `f32`), the products in the original's order and
    /// roundings; a zero translation and `w` 1.
    pub fn euler_matrix(h: &mut impl Host, out: u32, angles: u32) -> u32 {
        let sin = h.rebase(CI_SIN);
        let cos = h.rebase(CI_COS);
        let mut sc = |at: u32| {
            let x = ld(h, at);
            let s = h.call_intrinsic(sin, x);
            let c = h.call_intrinsic(cos, x);
            (x87::load(s.to_bits()), x87::load(c.to_bits()))
        };
        let (sx, cx) = sc(angles);
        let (sy, cy) = sc(angles + 4);
        let (sz, cz) = sc(angles + 8);
        let a = round(x87::mul(sy, cz));
        let b = round(x87::mul(sy, sz));
        let m20 = x87::add(x87::mul(sx, sz), x87::mul(a, cx));
        let m24 = x87::sub(x87::mul(b, cx), x87::mul(sx, cz));
        let m28 = x87::mul(cy, cx);
        let m14 = x87::sub(x87::mul(a, sx), x87::mul(sz, cx));
        let m10 = x87::add(x87::mul(b, sx), x87::mul(cx, cz));
        let mc = x87::mul(sx, cy);
        let p = x87::mul(cz, cy);
        let q = x87::mul(sz, cy);
        let words = [
            round(p),
            round(q),
            round(-sy),
            0.0,
            round(m14),
            round(m10),
            round(mc),
            0.0,
            round(m20),
            round(m24),
            round(m28),
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
        ];
        for (i, w) in words.into_iter().enumerate() {
            st(h, out + 4 * i as u32, w);
        }
        out
    }

    /// `Phyre_PRedBlackTree_countNodes(tree)` -> the number of nodes: the
    /// first node's successor through `0x005751d0`, the rest walked here
    /// (the same steps, inline in the original).
    pub fn count_nodes(h: &mut impl Host, tree: u32) -> u32 {
        let root: u32 = h.read(tree + 0xc);
        let mut node = root;
        if root != tree && root != 0 {
            loop {
                let left: u32 = h.read(node);
                if left == tree {
                    break;
                }
                node = left;
            }
        }
        if node == tree {
            return 0;
        }
        let mut next = thiscall(h, TREE_SUCCESSOR, tree, &[node]);
        let mut count = 0u32;
        loop {
            count = count.wrapping_add(1);
            let at = next;
            if next != tree {
                next = successor(h, tree, next);
            }
            if at == tree {
                return count;
            }
        }
    }

    /// The red-black tree's successor of `node` (`0x005751d0`'s steps):
    /// the right subtree's leftmost node, else the first ancestor reached
    /// from a left child; the tree itself after the last.
    fn successor(h: &impl Host, tree: u32, node: u32) -> u32 {
        let right: u32 = h.read(node.wrapping_add(4));
        if right != tree {
            let mut n = right;
            loop {
                let left: u32 = h.read(n);
                if left == tree {
                    return n;
                }
                n = left;
            }
        }
        let parent_of = |n: u32| {
            let p: u32 = h.read(n.wrapping_add(8));
            p - (p & 1)
        };
        let mut child = node;
        let mut parent = parent_of(node);
        while child == h.read::<u32>(parent.wrapping_add(4)) {
            child = parent;
            parent = parent_of(parent);
        }
        if parent == tree + 0xc { tree } else { parent }
    }

    /// `0x0055afc0(this)` -> `eax`: two class words (`+0x50`, `+0x64`)
    /// set to `PBase`'s descriptor with zero words before them, the name
    /// `+0x54` freed unless borrowed and cleared, `+0x5c` -1. `eax` is
    /// `PhyreFree`'s, or the descriptor with its low byte the name test's.
    pub fn reset_names(h: &mut impl Host, this: u32) -> u32 {
        let d = h.call_sym(PBASE_DESCRIPTOR, Conv::Cdecl, &[]);
        h.write(this + 0x4c, 0u32);
        h.write(this + 0x50, d);
        h.write(this + 0x60, 0u32);
        h.write(this + 0x64, d);
        let name: u32 = h.read(this + 0x54);
        let test = !(name as u8);
        let eax = if test & 1 != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[name])
        } else {
            d & !0xff | u32::from(test)
        };
        h.write(this + 0x54, 0u32);
        h.write(this + 0x5c, u32::MAX);
        eax
    }

    /// `0x00576c70(p)` -> the swap's `eax`: four words byte-swapped in place.
    pub fn swap4(h: &mut impl Host, p: u32) -> u32 {
        let mut eax = 0;
        for k in 0..4 {
            let (v, e) = swap_and_eax(h.read(p + 4 * k));
            h.write(p + 4 * k, v);
            eax = e;
        }
        eax
    }

    /// `0x00576d20(p)` -> the swap's `eax`: words 1, 2 and 0 byte-swapped.
    pub fn swap3(h: &mut impl Host, p: u32) -> u32 {
        let mut eax = 0;
        for k in [1, 2, 0] {
            let (v, e) = swap_and_eax(h.read(p + 4 * k));
            h.write(p + 4 * k, v);
            eax = e;
        }
        eax
    }

    /// `PSharray<T>::resize(this; n)` (`0x00599540`) -> 0 or 0xd: as
    /// `0x0048c470`, through `0x0058c590`, the old storage freed only when
    /// it is not null.
    pub fn sharray_resize(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let word: u32 = h.read(this);
        if n == word {
            return 0;
        }
        let count = word & 0x7fff_ffff;
        let old = match count {
            0 => 0,
            1 => this + 4,
            _ => h.read(this + 4),
        };
        let keep = if n < count { n } else { count };
        let block = thiscall(h, SHARRAY_REALLOC, this, &[n, old, keep]);
        if block == 0 && n != 0 {
            return 0xd;
        }
        if old != block && old != this + 4 && h.read::<u32>(this) >> 31 == 0 && old != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        if block != this + 4 {
            h.write(this + 4, block);
        }
        h.write(this, n);
        0
    }

    /// `0x006ac0b0(this; key, f)` -> the count: each of the `+8` (less
    /// bit 31) pairs at `+0xc` whose object's `+0x58` is `key` gets `f`.
    pub fn set_by_key(h: &mut impl Host, this: u32, key: u32, f: u32) -> u32 {
        let n = h.read::<u32>(this + 8) & 0x7fff_ffff;
        let v = x87::store(x87::load(f));
        for i in 0..n {
            let pairs: u32 = h.read(this + 0xc);
            let e = pairs.wrapping_add(8 * i);
            let obj: u32 = h.read(e);
            if obj != 0 && h.read::<u32>(obj.wrapping_add(0x58)) == key {
                h.write(e.wrapping_add(4), v);
            }
        }
        n
    }

    /// A `PSharray`'s storage for count word `word`.
    fn storage(h: &impl Host, at: u32) -> u32 {
        match h.read::<u32>(at) & 0x7fff_ffff {
            0 => 0,
            1 => at + 4,
            _ => h.read(at + 4),
        }
    }

    /// `0x005a7990(group)` -> `group`: in each level (`+0x40`, `+0x3c`
    /// of them, 0x24 bytes) every 12-byte element of the `+0x14`
    /// `PSharray` gets 4 at `+8`.
    pub fn lod_levels_reset(h: &mut impl Host, this: u32) -> u32 {
        let n = h.read::<u32>(this + 0x3c) & 0x7fff_ffff;
        for k in 0..n {
            let level = h.read::<u32>(this + 0x40).wrapping_add(0x24 * k);
            let m = h.read::<u32>(level.wrapping_add(0x14)) & 0x7fff_ffff;
            for j in 0..m {
                let s = storage(h, level.wrapping_add(0x14));
                h.write(s.wrapping_add(12 * j + 8), 4u32);
            }
        }
        this
    }

    /// `0x005a83a0(level; flag)`: each object of the `+4` `PSharray` with
    /// a target `+0x1c` gets the object (flag set) or 0 at that target's
    /// `+0x1c`. The original's `eax` is a scratch value (never read).
    pub fn lod_level_owners(h: &mut impl Host, this: u32, flag: u32) {
        let n = h.read::<u32>(this + 4) & 0x7fff_ffff;
        for i in 0..n {
            let s = storage(h, this + 4);
            let obj: u32 = h.read(s.wrapping_add(4 * i));
            if obj == 0 {
                continue;
            }
            let t: u32 = h.read(obj.wrapping_add(0x1c));
            if t != 0 {
                h.write(t.wrapping_add(0x1c), if flag as u8 == 0 { 0 } else { obj });
            }
        }
    }

    /// `0x005a7d10(group; flag)`: a change of the byte `+0x44`: on, the
    /// levels' reset; off, each level's owners cleared; then the byte. The
    /// original's `eax` is the flag over the caller's (never read).
    pub fn lod_group_enable(h: &mut impl Host, this: u32, flag: u32) {
        let on = flag as u8;
        if h.read::<u8>(this + 0x44) == on {
            return;
        }
        if on != 0 {
            thiscall(h, LOD_LEVELS_RESET, this, &[]);
        } else {
            let n = h.read::<u32>(this + 0x3c) & 0x7fff_ffff;
            for k in 0..n {
                let level = h.read::<u32>(this + 0x40).wrapping_add(0x24 * k);
                thiscall(h, LOD_LEVEL_OWNERS, level, &[0]);
            }
        }
        h.write(this + 0x44, on);
    }

    /// Copies a shader parameter `q` (offset `+8`, size `+0xa` low 13
    /// bits) from `src` into the parameter buffer `buf` (its size at
    /// `+0`), when it fits and does not overlap the size word.
    fn copy_parameter(h: &mut impl Host, buf: u32, q: u32, src: u32) {
        let off = u32::from(h.read::<u16>(q + 8));
        let size = u32::from(h.read::<u16>(q + 0xa) & 0x1fff);
        if off + size > h.read::<u32>(buf) || off < 4 {
            return;
        }
        h.call_sym(MEMCPY, Conv::Cdecl, &[buf.wrapping_add(off), src, size]);
    }

    /// The shade coefficient through `0x006a8c40` into
    /// `setShadeCoeff(cl; inst, coeff)`; the intensity scaled by the
    /// image's double, stored as `f32`; the shader instances' array.
    fn shade_and_scale(h: &mut impl Host, this: u32, inst: u32, intensity: u32) -> (u32, u32) {
        let coeff = h.call_sym_st0(SHADE_COEFF, Conv::Thiscall(this), &[]);
        thiscall(h, SET_SHADE_COEFF, this, &[inst, coeff.to_bits()]);
        let at = h.rebase(INTENSITY_SCALE);
        let lo: u32 = h.read(at);
        let hi: u32 = h.read(at + 4);
        let scale = f64::from_bits(u64::from(hi) << 32 | u64::from(lo));
        let k = x87::store(x87::mul(x87::load(intensity), scale));
        let arr: u32 = h.read(inst + 0x10);
        (k, arr)
    }

    /// Writes the floats `vals` (each through `fld`/`fstp dword`) at `at`.
    fn put(h: &mut impl Host, at: u32, vals: &[u32]) {
        for (i, &v) in vals.iter().enumerate() {
            h.write(at + 4 * i as u32, x87::store(x87::load(v)));
        }
    }

    /// The shader instances of `arr` and their parameter buffers:
    /// `(shader, buffer)` of instance `i` (`PArray_Data`'s pointers).
    fn instance(h: &mut impl Host, arr: u32, i: u32) -> (u32, u32) {
        let data = thiscall(h, ARRAY_DATA, arr, &[]);
        let e: u32 = h.read(data.wrapping_add(4 * i));
        (h.read(e), h.read(e.wrapping_add(4)))
    }

    /// `0x006a9bb0(cl; inst, r, g, b, intensity)` (`ret 0x14`): the shade
    /// coefficient, then in every shader instance `AddLgtClr` = (r, g, b,
    /// intensity * 0.4) and `Shininess` = (r, g, b), copied out of the
    /// frame at `frame` (0x24 bytes as the original's `ebp - 0x24`: the
    /// three at `+0`, a never-written word at `+0xc`, the four at `+0x10`).
    pub fn add_light(
        h: &mut impl Host,
        this: u32,
        inst: u32,
        rgb: [u32; 3],
        intensity: u32,
        frame: u32,
    ) {
        let (k, arr) = shade_and_scale(h, this, inst, intensity);
        let n = h.read::<u32>(arr) & 0x7fff_ffff;
        if n == 0 {
            return;
        }
        let four = frame + 0x10;
        let three = frame;
        put(h, four, &[rgb[0], rgb[1], rgb[2], k]);
        for i in 0..n {
            let (shader, buf) = instance(h, arr, i);
            let name = h.rebase(ADD_LIGHT_COLOUR);
            let q = thiscall(h, FIND_PARAMETER, shader, &[name]);
            if q != 0 {
                copy_parameter(h, buf, q, four);
            }
            let name = h.rebase(SHININESS);
            let q = thiscall(h, FIND_PARAMETER, shader, &[name]);
            put(h, three, &rgb);
            if q != 0 {
                copy_parameter(h, buf, q, three);
            }
        }
    }

    /// `0x006a9cd0(cl; inst, c0, c1, c2, c3, s0, s1, s2, intensity)` (`ret
    /// 0x24`): as [`add_light`] with `AddLgtClr` = (s, intensity * 0.4),
    /// `Shininess` = s and `MaterialColour` = c; the frame (0x34 bytes as
    /// `ebp - 0x34`): the shininess at `+0`, a never-written word at `+0xc`,
    /// the material colour at `+0x10`, the light at `+0x20`.
    pub fn add_light_material(
        h: &mut impl Host,
        this: u32,
        inst: u32,
        colour: [u32; 4],
        shine: [u32; 3],
        intensity: u32,
        frame: u32,
    ) {
        let (k, arr) = shade_and_scale(h, this, inst, intensity);
        let n = h.read::<u32>(arr) & 0x7fff_ffff;
        if n == 0 {
            return;
        }
        let light = frame + 0x20;
        let material = frame + 0x10;
        let three = frame;
        put(h, light, &[shine[0], shine[1], shine[2], k]);
        put(h, material, &colour);
        for i in 0..n {
            let (shader, buf) = instance(h, arr, i);
            let name = h.rebase(ADD_LIGHT_COLOUR);
            let q = thiscall(h, FIND_PARAMETER, shader, &[name]);
            if q != 0 {
                copy_parameter(h, buf, q, light);
            }
            let name = h.rebase(SHININESS);
            let q = thiscall(h, FIND_PARAMETER, shader, &[name]);
            put(h, three, &shine);
            if q != 0 {
                copy_parameter(h, buf, q, three);
            }
            let name = h.rebase(MATERIAL_COLOUR);
            let q = thiscall(h, FIND_PARAMETER, shader, &[name]);
            if q != 0 {
                copy_parameter(h, buf, q, material);
            }
        }
    }

    /// `PCamera`'s copy assignment `0x004533b0(dst; &src)` -> `dst`: the
    /// floats (through `fld`/`fstp`), the two 4x4s at `+0x30` and `+0x70`
    /// (`PMatrix4_Copy`), the word `+0xb0`, the name `+0xcc` (assigned
    /// unless it is the same object; the tag bit dropped) and the rest of
    /// the floats; `+0xdc` and `+0xfc` are left alone.
    pub fn camera_assign(h: &mut impl Host, dst: u32, src: u32) -> u32 {
        for k in 0..12 {
            copy_float(h, src + 4 * k, dst + 4 * k);
        }
        thiscall(h, MATRIX_COPY, dst + 0x30, &[src + 0x30]);
        thiscall(h, MATRIX_COPY, dst + 0x70, &[src + 0x70]);
        let w: u32 = h.read(src + 0xb0);
        h.write(dst + 0xb0, w);
        for off in (0xb4..=0xc4).step_by(4) {
            copy_float(h, src + off, dst + off);
        }
        if dst != src {
            let tagged: u32 = h.read(src + 0xcc);
            thiscall(h, STRING_ASSIGN, dst + 0xcc, &[tagged - (tagged & 1)]);
        }
        for off in (0xd0..=0x148).step_by(4) {
            // `+0xdc` and `+0xfc` are not the camera's to copy
            if off != 0xdc && off != 0xfc {
                copy_float(h, src + off, dst + off);
            }
        }
        dst
    }
}
