//! The morph manager's weight tables and the material parameter writes
//! the lighting and morph code share.
//!
//! A material parameter (`ShaderParams_FindByName`'s result) holds its
//! byte offset at `+8` (`u16`) and its size at `+0xa` (`u16 & 0x1fff`) in
//! the instance's parameter buffer, whose first word is its size; the
//! original writes it only when `offset + size` fits and the offset is
//! past that word.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::methods::quiet;

/// `PArray_Data_004d45a0` (thiscall `(array)`): the elements.
pub const ARRAY_DATA: u32 = 0x004d_45a0;
/// `ShaderParams_FindByName` (thiscall `(shader, name)`).
pub const FIND_PARAM: u32 = 0x0056_cbf0;
/// `memcpy` (cdecl `(dst, src, n)`).
pub const MEMCPY: u32 = 0x0094_9256;
/// `"CharacterShadeCoeff"`.
pub const SHADE_COEFF_NAME: u32 = 0x00b4_8120;
/// `"CharacterShadeColor"`.
pub const SHADE_COLOR_NAME: u32 = 0x00b4_8134;
/// `0x004fc880` (thiscall `(inst)`): the instance's morph array.
pub const MORPH_ARRAY: u32 = 0x004f_c880;
/// `Phyre_PArray_PBase_const_getAt_005b2c70` (thiscall `(array, i)`).
pub const ARRAY_GET_AT: u32 = 0x005b_2c70;
/// `Phyre_PhyreMalloc_0042fb80` (cdecl `(size, align)`).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;

const COUNT: u32 = 0x7fff_ffff;

/// The scale `setShadeCoeff` applies to the global intensity
/// (`0x00b48300`, a double).
pub const SHADE_COEFF_SCALE: f64 = 0.208_125_002_682_209_01;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ARRAY_DATA, ARRAY_GET_AT, COUNT, Conv, FIND_PARAM, Host, MEMCPY, MORPH_ARRAY, PHYRE_MALLOC,
        SHADE_COEFF_NAME, SHADE_COEFF_SCALE, SHADE_COLOR_NAME, quiet, x87,
    };

    /// For each element of the instance's material list (`[inst + 0x10]`,
    /// its elements through `ARRAY_DATA`), the parameter `name` of the
    /// element's shader (`[e]`) in its buffer (`[e + 4]`) gets `size`
    /// bytes from `src` when it fits. The list's count is read once.
    fn write_param(h: &mut impl Host, inst: u32, name: u32, src: u32, skip_null_src: bool) {
        let list: u32 = h.read(inst + 0x10);
        let count = h.read::<u32>(list) & COUNT;
        let data_fn = h.rebase(ARRAY_DATA);
        let find = h.rebase(FIND_PARAM);
        let copy = h.rebase(MEMCPY);
        for i in 0..count {
            let data = h.call(data_fn, Conv::Thiscall(list), &[]);
            let e: u32 = h.read(data.wrapping_add(4 * i));
            let buf: u32 = h.read(e + 4);
            let shader: u32 = h.read(e);
            let p = h.call(find, Conv::Thiscall(shader), &[name]);
            if p == 0 || (skip_null_src && src == 0) {
                continue;
            }
            let off = u32::from(h.read::<u16>(p + 8));
            let size = u32::from(h.read::<u16>(p + 0xa) & 0x1fff);
            if off + size > h.read::<u32>(buf) || off < 4 {
                continue;
            }
            h.call(copy, Conv::Cdecl, &[buf.wrapping_add(off), src, size]);
        }
    }

    /// `Material_setParameter(inst, name, src)` (`0x006e60d0`, cdecl): see
    /// [`write_param`]; a null `src` writes nothing.
    pub fn set_parameter(h: &mut impl Host, inst: u32, name: u32, src: u32) {
        write_param(h, inst, name, src, true);
    }

    /// `CharacterLighting::setShadeCoeff(cl, inst, coeff)` (`0x006aaef0`,
    /// `ret 8`): on an initialised manager, `"CharacterShadeCoeff"` gets
    /// `f32(global intensity +0x1a0 * 0.208125)`. The `coeff` argument
    /// is overwritten by that value before use, so it is ignored.
    ///
    /// The source is the argument slot: a parameter larger than 4 bytes
    /// would also copy the caller's stack above it; `scratch` holds the
    /// value then zeros (junk read: the caller's frame, brief 2026-10-24).
    pub fn set_shade_coeff(h: &mut impl Host, cl: u32, inst: u32, scratch: u32) {
        if h.read::<u8>(cl + 0x1cc) == 0 {
            return;
        }
        let v = x87::store(x87::mul(x87::load(h.read(cl + 0x1a0)), SHADE_COEFF_SCALE));
        h.write(scratch, v);
        let name = h.rebase(SHADE_COEFF_NAME);
        write_param(h, inst, name, scratch, false);
    }

    /// `CharacterLighting::setShadeColor(cl, inst, r, g, b)` (`0x006ab010`,
    /// `ret 0x10`): on an initialised manager, `"CharacterShadeColor"`
    /// gets `(r, g, b)` from a local block. `scratch` is laid out as the
    /// original's frame from `ebp - 0x14`: the colour, a word it never
    /// writes (junk read: `[ebp-0x8]` never written; 0, brief
    /// 2026-10-24), then the material list (`[ebp-0x4]`).
    pub fn set_shade_color(h: &mut impl Host, cl: u32, inst: u32, rgb: [u32; 3], scratch: u32) {
        if h.read::<u8>(cl + 0x1cc) == 0 {
            return;
        }
        for (k, c) in rgb.into_iter().enumerate() {
            h.write(scratch + 4 * k as u32, quiet(c));
        }
        h.write(scratch + 0xc, 0u32);
        let list: u32 = h.read(inst + 0x10);
        h.write(scratch + 0x10, list);
        let name = h.rebase(SHADE_COLOR_NAME);
        write_param(h, inst, name, scratch, false);
    }

    /// `Material_hasParameter(inst, name)` (`0x006ac4a0`, `ret 8`) -> `al`:
    /// whether a non-null element of the material list (`[inst + 0x10]`:
    /// count > 1 its `+4` pointer, 1 inline at `+4`) has the parameter.
    /// A null `inst` gives 0.
    pub fn has_parameter(h: &mut impl Host, inst: u32, name: u32) -> u8 {
        if inst == 0 {
            return 0;
        }
        let list: u32 = h.read(inst + 0x10);
        let count = h.read::<u32>(list) & COUNT;
        let data = match count {
            0 => 0,
            1 => list + 4,
            _ => h.read(list + 4),
        };
        let find = h.rebase(FIND_PARAM);
        for i in 0..count {
            let e: u32 = h.read(data.wrapping_add(4 * i));
            if e == 0 {
                continue;
            }
            let shader: u32 = h.read(e);
            if h.call(find, Conv::Thiscall(shader), &[name]) != 0 {
                return 1;
            }
        }
        0
    }

    /// `MorphManager_setMorphWeight(inst, w)` (`0x006ac900`, `ret 8`): for
    /// each non-null morph of the instance (`MORPH_ARRAY(inst)`) with a
    /// weight array (`getAt(0)`), weight 0 = `1 - w`, weight 1 = `w` (when
    /// there are that many).
    pub fn set_morph_weight(h: &mut impl Host, inst: u32, w: u32) {
        if inst == 0 {
            return;
        }
        let f = h.rebase(MORPH_ARRAY);
        let arr = h.call(f, Conv::Thiscall(inst), &[]);
        let n = h.read::<u32>(arr) & COUNT;
        let get_at = h.rebase(ARRAY_GET_AT);
        let wv = x87::load(w);
        for i in 0..n {
            let elems: u32 = h.read(arr + 4);
            let e: u32 = h.read(elems.wrapping_add(4 * i));
            if e == 0 {
                continue;
            }
            let weights = h.call(get_at, Conv::Thiscall(e), &[0]);
            if weights == 0 {
                continue;
            }
            if h.read::<u32>(weights) & COUNT != 0 {
                let p: u32 = h.read(weights + 4);
                h.write(p, x87::store(x87::sub(1.0, wv)));
            }
            if h.read::<u32>(weights) & COUNT > 1 {
                let p: u32 = h.read(weights + 4);
                h.write(p + 4, x87::store(wv));
            }
        }
    }

    /// `MorphManager_addPair(arr, key, value)` (`0x006ab8c0`, `ret 0xc`)
    /// -> 0, or the push's result: a present key returns 0; else the
    /// first empty slot (key 0) takes the pair; else the array grows by
    /// one (`PhyreMalloc(8 count + 8, 4)` and the pair push `0x006ac380`,
    /// its pair the two argument words; `item` holds them).
    pub fn add_pair(h: &mut impl Host, arr: u32, key: u32, value: u32, item: u32) -> u32 {
        let count = h.read::<u32>(arr) & COUNT;
        let data: u32 = h.read(arr + 4);
        if (0..count).any(|i| h.read::<u32>(data.wrapping_add(8 * i)) == key) {
            return 0;
        }
        if let Some(i) = (0..count).find(|i| h.read::<u32>(data.wrapping_add(8 * i)) == 0) {
            h.write(data.wrapping_add(8 * i), key);
            h.write(data.wrapping_add(8 * i + 4), value);
            return 0;
        }
        let f = h.rebase(PHYRE_MALLOC);
        let buffer = h.call(f, Conv::Cdecl, &[count.wrapping_mul(8).wrapping_add(8), 4]);
        h.write(item, key);
        h.write(item + 4, value);
        crate::p4_light::bridge::array_push(h, arr, buffer, item, 2, true)
    }

    /// `MorphManager_setWeights(mm, id, &a, &b)` (`0x006abfc0`, `ret 0xc`):
    /// in the three pair tables (`+0`, `+8`, `+0x10`: count, pointer) each
    /// entry whose object's `+0x58` is `id` gets `*a`; in the 16-byte
    /// table `+0x18` such entries get `*a` and `*b`.
    pub fn set_weights(h: &mut impl Host, mm: u32, id: u32, a: u32, b: u32) {
        for table in [0u32, 8, 0x10] {
            let n = h.read::<u32>(mm + table) & COUNT;
            for i in 0..n {
                let data: u32 = h.read(mm + table + 4);
                let obj: u32 = h.read(data.wrapping_add(8 * i));
                if obj != 0 && h.read::<u32>(obj + 0x58) == id {
                    let v = quiet(h.read(a));
                    h.write(data.wrapping_add(8 * i + 4), v);
                }
            }
        }
        let n = h.read::<u32>(mm + 0x18) & COUNT;
        for i in 0..n {
            let data: u32 = h.read(mm + 0x1c);
            let at = data.wrapping_add(16 * i);
            let obj: u32 = h.read(at);
            if obj != 0 && h.read::<u32>(obj + 0x58) == id {
                let v = quiet(h.read(a));
                h.write(at + 4, v);
                let data: u32 = h.read(mm + 0x1c);
                let v = quiet(h.read(b));
                h.write(data.wrapping_add(16 * i + 8), v);
            }
        }
    }
}
