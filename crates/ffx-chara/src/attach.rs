//! Attachment points: a model block's list of named points on elements
//! (`Ch_ElemGetAttr`), binding a character to one (`Ch_BindToElem`), and
//! the optional positions (`Ch_OptposGet`).
//!
//! The list is at the block's `+0x6c` (pointer): entries of 16 bytes from
//! the pointer at `+0x20`, count at `+0x24`. An entry has a `u16` with the
//! point id in its low 14 bits and a kind in the top two, the `i16`
//! element at `+2`, and an offset x, y, z at `+4`.

/// What `Ch_ElemGetAttr` makes of an entry's kind bits: (result, whether
/// the offset is written).
#[must_use]
pub const fn attr_kind(kind: u16) -> (u32, bool) {
    match kind & 0xc000 {
        0x4000 => (1, true),
        0x8000 => (2, false),
        _ => (0, false),
    }
}

/// The originals, on the characters.
pub mod bridge {
    use ffx_bridge::Mem;

    use super::attr_kind;
    use crate::chr::bridge::{blk, f};
    use crate::layout as l;
    use crate::x87::Reg;

    /// Model block: the attachment point list.
    pub const BLK_ATTRS: u32 = 0x6c;
    /// Optional positions: 16 bytes each (x, y, z, w) from here; one
    /// exists when a component is not 0 (`Ch_OptposGetExist` tests w).
    pub const OPTPOS: u32 = 0x524;
    /// `f32` x, y, z: where `Ch_OptposGet` falls back to when position 0
    /// is empty too.
    pub const OPTPOS_FALLBACK: u32 = 0x200;

    /// The entry of point `id`, if the list has it.
    fn find(mem: &impl Mem, list: u32, id: u32) -> Option<u32> {
        let mut n: u32 = mem.read(list + 0x24);
        let mut e: u32 = mem.read(list + 0x20);
        while n != 0 {
            #[allow(clippy::cast_sign_loss)] // movsx then masked
            let key = i32::from(mem.read::<i16>(e)) as u32 & 0x3fff;
            n -= 1;
            if key == id {
                return Some(e);
            }
            e = e.wrapping_add(16);
        }
        None
    }

    fn element(mem: &impl Mem, e: u32) -> u32 {
        #[allow(clippy::cast_sign_loss)] // movsx
        let v = i32::from(mem.read::<i16>(e + 2)) as u32;
        v
    }

    fn unit(mem: &mut impl Mem, out: u32) {
        mem.write(out, [0.0f32, 0.0, 0.0, 1.0]);
    }

    /// `Ch_ElemGetAttr(chr, id, &elem, &offset)` (`0x00833a70`): point
    /// `id`'s element and, for kind 1, its offset scaled by 10000 / scale²
    /// × the bind scale (`offset` is a vector of four, w = 1; null skips
    /// it). Returns the kind: 1, 2, or 0 (also when there is no such
    /// point).
    pub fn ch_elem_get_attr(mem: &mut impl Mem, chr: u32, id: u32, elem: u32, out: u32) -> u32 {
        elem_get_attr(mem, chr, id, Some(elem), out).0
    }

    /// [`ch_elem_get_attr`] with the element also returned (and stored
    /// only when `elem` is given): (kind, element).
    fn elem_get_attr(
        mem: &mut impl Mem,
        chr: u32,
        id: u32,
        elem: Option<u32>,
        out: u32,
    ) -> (u32, u32) {
        if let Some(at) = elem {
            mem.write(at, 0u32);
        }
        if out != 0 {
            unit(mem, out);
        }
        let b = blk(mem, chr);
        let list: u32 = mem.read(b + BLK_ATTRS);
        if list == 0 {
            return (0, 0);
        }
        let Some(e) = find(mem, list, id) else {
            return (0, 0);
        };
        let v = element(mem, e);
        if let Some(at) = elem {
            mem.write(at, v);
        }
        let (kind, offset) = attr_kind(mem.read(e));
        if mem.read::<u16>(e) & 0xc000 == 0 {
            if out != 0 {
                unit(mem, out);
            }
            return (0, v);
        }
        if !offset || out == 0 {
            return (kind, v);
        }
        let s = f(mem, b + l::BLK_DATA_SCALE);
        let k = Reg::constant(1.0e4)
            .div_st(Reg::load(s))
            .div_m32(s)
            .mul_m32(f(mem, b + l::BLK_BIND_SCALE))
            .store();
        let k = Reg::load(k);
        let x = Reg::load(f(mem, e + 4)).mul_st(k).store();
        let y = Reg::load(f(mem, e + 8)).mul_st(k).store();
        let z = k.mul_m32(f(mem, e + 0xc)).store();
        mem.write(out, [x, y, z, 1.0]);
        (kind, v)
    }

    /// `Ch_ElemGetDirect(chr, id)` (`0x00833bf0`): point `id`'s element,
    /// or 0. The list must exist.
    pub fn ch_elem_get_direct(mem: &impl Mem, chr: u32, id: u32) -> u32 {
        let list: u32 = mem.read(blk(mem, chr) + BLK_ATTRS);
        find(mem, list, id).map_or(0, |e| element(mem, e))
    }

    /// `Ch_BindToElem(chr, parent, id)` (`0x00832630`): binds to the
    /// parent's point `id` (element and offset by `Ch_ElemGetAttr`, whose
    /// result it returns); a null parent only unbinds (and returns
    /// nothing; 0 here).
    pub fn ch_bind_to_elem(mem: &mut impl Mem, chr: u32, parent: u32, id: u32) -> u32 {
        mem.write(chr + l::BIND_PARENT, parent);
        if parent == 0 {
            mem.write(chr + l::BIND_PARENT_ID, u32::MAX);
            return 0;
        }
        let pid: u32 = mem.read(parent + l::BIND_SELF);
        mem.write(chr + l::BIND_PARENT_ID, pid);
        ch_elem_get_attr(mem, parent, id, chr + l::BIND_ELEM, chr + l::BIND_OFFSET)
    }

    /// `Ch_OptposGetElement(chr, id, &offset)` (`0x00835660`): point
    /// `id`'s element (`Ch_ElemGetAttr`, the element in a local).
    pub fn ch_optpos_get_element(mem: &mut impl Mem, chr: u32, id: u32, out: u32) -> u32 {
        elem_get_attr(mem, chr, id, None, out).1
    }

    /// `Ch_OptposGet(chr, i, out)` (`0x008354f0`): optional position `i`
    /// when any of its components is not 0, else position 0 when its w is
    /// not 0, else the fallback point with w = 1.
    pub fn ch_optpos_get(mem: &mut impl Mem, chr: u32, i: u32, out: u32) {
        let at = |k: u32| chr.wrapping_add(OPTPOS).wrapping_add(k.wrapping_mul(16));
        let p = at(i);
        // w first, then x, y, z; NaN counts as not 0
        let set = [12u32, 0, 4, 8]
            .iter()
            .any(|&c| f(mem, p.wrapping_add(c)) != 0.0);
        if set {
            let v: [u32; 4] = mem.read(p);
            mem.write(out, v);
        } else if f(mem, at(0) + 12) != 0.0 {
            let v: [u32; 4] = mem.read(at(0));
            mem.write(out, v);
        } else {
            let v = [0, 4, 8].map(|c| crate::x87::load(f(mem, chr + OPTPOS_FALLBACK + c)));
            mem.write(out, [v[0], v[1], v[2], 1.0]);
        }
    }

    /// `Ch_OptposGetBase(chr, out)` (`0x008355e0`): `Ch_OptposGet(chr, 0,
    /// out)`.
    pub fn ch_optpos_get_base(mem: &mut impl Mem, chr: u32, out: u32) {
        ch_optpos_get(mem, chr, 0, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        assert_eq!(attr_kind(0x4001), (1, true));
        assert_eq!(attr_kind(0x8001), (2, false));
        assert_eq!(attr_kind(0xc001), (0, false));
        assert_eq!(attr_kind(0x0001), (0, false));
    }
}
