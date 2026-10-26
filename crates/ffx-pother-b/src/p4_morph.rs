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

/// `MorphManager_addPair_006ab8c0` (stdcall `(arr, key, value)`).
pub const ADD_PAIR: u32 = 0x006a_b8c0;
/// `Material_hasParameter_006ac4a0` (stdcall `(inst, name)` -> `al`).
pub const HAS_PARAMETER: u32 = 0x006a_c4a0;
/// `0x006ab950` (stdcall `(arr, inst, a, b, c)`, `ret 0x14`): appends a
/// 16-byte UV-offset record.
pub const ADD_UV_OFFSET: u32 = 0x006a_b950;
/// `PhyreSceneCursor_Begin_00637730` (thiscall `(cursor)`).
pub const CURSOR_BEGIN: u32 = 0x0063_7730;
/// `PhyreSceneCursor_Next_006377a0` (thiscall `(cursor)`).
pub const CURSOR_NEXT: u32 = 0x0063_77a0;
/// `PMeshInstance`'s class descriptor, what the cursor looks for.
pub const MESH_INSTANCE_DESC: u32 = 0x00ca_7228;
/// The material parameter names the morph kinds need (`.rdata`):
/// vertex colour (kind 4), UV (kind 3), UV offset (flag `0x400`).
pub const MORPH_PARAMS: [u32; 3] = [0x00b4_8398, 0x00b4_83b0, 0x00b4_83c0];

pub mod bridge3 {
    //! The original over game memory.

    use super::{
        ADD_PAIR, ADD_UV_OFFSET, CURSOR_BEGIN, CURSOR_NEXT, Conv, HAS_PARAMETER, Host,
        MESH_INSTANCE_DESC, MORPH_PARAMS,
    };

    /// `Material_hasParameter(inst, name)`'s `al`. The original leaves the
    /// morph manager in `ecx` for the call (the callee only uses `cl`
    /// after clearing it).
    fn has(h: &mut impl Host, mm: u32, inst: u32, name: u32) -> bool {
        let name = h.rebase(name);
        h.call_sym(HAS_PARAMETER, Conv::Thiscall(mm), &[inst, name]) & 0xff != 0
    }

    /// One mesh instance: the render-order bump and its morph
    /// registrations.
    fn register(h: &mut impl Host, mm: u32, inst: u32) {
        if h.read::<u32>(inst + 0x4c) == 1 {
            let bounds: u32 = h.read(inst + 0x1c);
            let order: u32 = h.read(bounds + 0x20);
            h.write(bounds + 0x20, order.wrapping_add(100_000_000));
        }
        if h.read::<i32>(inst + 0x30) <= 0 {
            return;
        }
        let kind: u32 = h.read(inst + 0x44);
        let [color, uv, offset] = MORPH_PARAMS;
        if kind == 1 && h.read::<u32>(inst + 0x18) != 0 {
            h.call_sym(ADD_PAIR, Conv::Stdcall, &[mm, inst, 0]);
            return;
        }
        if kind == 4 {
            if has(h, mm, inst, color) {
                h.call_sym(ADD_PAIR, Conv::Stdcall, &[mm + 8, inst, 0]);
                return;
            }
        } else if kind == 3 && has(h, mm, inst, uv) {
            h.call_sym(ADD_PAIR, Conv::Stdcall, &[mm + 0x10, inst, 0]);
            return;
        }
        if h.read::<u32>(inst + 0x54) & 0x400 != 0 && has(h, mm, inst, offset) {
            // a 16-byte record by value: the instance, 0.0, 0.0, 0
            h.call_sym(ADD_UV_OFFSET, Conv::Stdcall, &[mm + 0x18, inst, 0, 0, 0]);
        }
    }

    /// `MorphManager::registerClusterMorphs(mm, cluster)` (`0x006aca10`,
    /// `ret 4`) -> 0: walks the cluster's `PMeshInstance`s with a scene
    /// cursor at `ebp - 0x40` (current, node, stride, remaining, offset,
    /// the instance lists `cluster + 0x1c`, the first list or 0, the
    /// class); for each: `m_DObjKind` (`+0x4c`) 1 adds 100000000 to its
    /// bounds' render order (`[+0x1c] + 0x20`); with morphs (`+0x30 >
    /// 0`) it registers by kind (`+0x44`): 1 with a dynamic mesh
    /// (`+0x18`) into `mm + 0`, 4 with a vertex-colour parameter into `+8`,
    /// 3 with a UV parameter into `+0x10` (weight 0.0), else a UV offset
    /// (`+0x54 & 0x400` with its parameter) into `+0x18`. The cursor steps
    /// as the original's inline loop does (a node equal to the current
    /// element is followed, and the element after it taken), with
    /// `PhyreSceneCursor_Next` when a block runs out. Also returns the
    /// word the original leaves in its argument's slot (the last instance
    /// visited, else `cluster`).
    pub fn register_cluster_morphs(
        h: &mut impl Host,
        ebp: u32,
        mm: u32,
        cluster: u32,
    ) -> (u32, u32) {
        let cur = ebp - 0x40;
        let lists = cluster.wrapping_add(0x1c);
        let first: u32 = h.read(lists);
        let first = if first == lists { 0 } else { first };
        for at in [0, 4, 8, 0xc, 0x10] {
            h.write(cur + at, 0u32);
        }
        h.write(cur + 0x14, lists);
        h.write(cur + 0x18, first);
        h.write(cur + 0x1c, h.rebase(MESH_INSTANCE_DESC));
        h.call_sym(CURSOR_BEGIN, Conv::Thiscall(cur), &[]);
        let mut slot = cluster;
        let mut left: u32 = h.read(cur + 0xc);
        if left == 0 {
            return (0, slot);
        }
        let mut at: u32 = h.read(cur);
        let mut node: u32 = h.read(cur + 4);
        loop {
            let off: u32 = h.read(cur + 0x10);
            let inst = off.wrapping_add(at);
            slot = inst;
            register(h, mm, inst);
            // the original's inline step
            let mut next_block = false;
            loop {
                let stride: u32 = h.read(cur + 8);
                at = at.wrapping_add(stride);
                left -= 1;
                h.write(cur, at);
                h.write(cur + 0xc, left);
                if node == at {
                    node = h.read(node);
                    h.write(cur + 4, node);
                    if left != 0 {
                        continue;
                    }
                    next_block = true;
                } else if left == 0 {
                    next_block = true;
                }
                break;
            }
            if next_block {
                h.call_sym(CURSOR_NEXT, Conv::Thiscall(cur), &[]);
                left = h.read(cur + 0xc);
                if left == 0 {
                    return (0, slot);
                }
                at = h.read(cur);
                node = h.read(cur + 4);
            }
        }
    }
}

pub mod bridge4 {
    //! The original over game memory.

    use super::{Conv, Host, PHYRE_MALLOC};
    use crate::PHYRE_FREE;

    /// Puts `p` (16 elements) into the array at `arr` (count, pointer),
    /// freeing the old owned storage as the originals do.
    fn replace(h: &mut impl Host, arr: u32, p: u32) {
        let old: u32 = h.read(arr + 4);
        let count: u32 = h.read(arr);
        if old != p && count >> 31 == 0 && old != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(arr + 4, p);
        h.write(arr, 0x10u32);
    }

    /// Gives the four arrays 16 fresh elements each where they don't
    /// have 16 (`Phyre_PhyreMalloc(size, 4)`): the weight arrays (`+0`,
    /// `+8`, `+0x10`: `{key 0, weight 0.0}`) and the UV-offset array
    /// (`+0x18`: `{0, 0.0, 0.0, 0}`). A failed allocation stops
    /// (`initialize`, `Err`) or skips that array (`reset`).
    fn reallocate(h: &mut impl Host, mm: u32, stop: bool) -> Result<(), ()> {
        for (k, arr) in [0u32, 8, 0x10].into_iter().enumerate() {
            if h.read::<u32>(mm + arr) == 0x10 {
                continue;
            }
            let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x80, 4]);
            if p == 0 {
                if stop {
                    return Err(());
                }
                continue;
            }
            for e in 0..0x10 {
                let at = p + 8 * e;
                if k == 0 {
                    // the first array stores the weight first
                    h.write(at + 4, 0u32);
                    h.write(at, 0u32);
                } else {
                    h.write(at, 0u32);
                    h.write(at + 4, 0u32);
                }
            }
            replace(h, mm + arr, p);
        }
        if h.read::<u32>(mm + 0x18) != 0x10 {
            let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x100, 4]);
            if p == 0 {
                return if stop { Err(()) } else { Ok(()) };
            }
            for e in 0..0x10 {
                let at = p + 0x10 * e;
                for off in [0, 4, 0xc, 8] {
                    h.write(at + off, 0u32);
                }
            }
            replace(h, mm + 0x18, p);
        }
        Ok(())
    }

    /// `MorphManager::initialize(mm, x)` (`0x006ac1b0`, thiscall, `ret 4`)
    /// -> 0, or 0xd when an allocation fails (the arrays after it stay as
    /// they are): `+0x24 = x`, then the arrays as [`reallocate`] makes
    /// them.
    pub fn initialize(h: &mut impl Host, mm: u32, x: u32) -> u32 {
        h.write(mm + 0x24, x);
        match reallocate(h, mm, true) {
            Ok(()) => 0,
            Err(()) => 0xd,
        }
    }

    /// `MorphManager::reset(mm)` (`0x006ac530`, fastcall): every element
    /// of the four arrays cleared in place (counts `& 0x7fffffff`), then
    /// [`reallocate`], skipping an array whose allocation fails.
    pub fn reset(h: &mut impl Host, mm: u32) {
        for arr in [0u32, 8, 0x10] {
            let n = h.read::<u32>(mm + arr) & 0x7fff_ffff;
            let data: u32 = h.read(mm + arr + 4);
            for e in 0..n {
                let at = data.wrapping_add(8 * e);
                h.write(at, 0u32);
                h.write(at + 4, 0u32);
            }
        }
        let n = h.read::<u32>(mm + 0x18) & 0x7fff_ffff;
        for e in 0..n {
            // the pointer is reread for each element
            let data: u32 = h.read(mm + 0x1c);
            let at = data.wrapping_add(0x10 * e);
            for off in [0, 4, 8, 0xc] {
                h.write(at + off, 0u32);
            }
        }
        let _ = reallocate(h, mm, false);
    }
}
