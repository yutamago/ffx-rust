//! "Mapping": fixing up the pointers of loaded character data after it
//! moved (each structure keeps its own address in its first word; the
//! difference to where it is now is added to every pointer in it), and
//! filling a model block from a character archive (`Ch_CbArcMapping`).
//!
//! A character archive (`.chr`) starts with its own address and an entry
//! count; entries are (pointer, size) pairs from `+0x10`: 0 the model
//! header (`Ch_ModelMapping`), 1 the texture data (`Ch_TexDataMapping`), 2
//! the attachment points, 4 a pointer list (`Ch_RelElemMapping`), 9 the
//! model parameters, 10 one more table.

/// `Chr_ModelEntry(i, base, version)` (`0x00828be0`): entry `i` of a model
/// header's model table: 24 bytes each for versions `0x831..0x1314`, else
/// 40.
#[must_use]
pub const fn model_entry(i: u32, base: u32, version: u32) -> u32 {
    #[allow(clippy::cast_possible_wrap)] // signed compares
    let v = version as i32;
    let size = if v < 0x1314 && v >= 0x831 { 24 } else { 40 };
    base.wrapping_add(i.wrapping_mul(size))
}

/// The originals, on the loaded data.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::model_entry;
    use crate::chr::bridge::f;
    use crate::layout as l;
    use crate::parts::bridge::ch_rel_elem_mapping;
    use crate::x87::{self, Reg};

    /// `"c001"`, Tidus's model (`Ch_CbMapping`).
    pub const NAME_C001: Global<u8> = Global::new(0x00b4_86bc);
    /// `"c101"`, Tidus's other model.
    pub const NAME_C101: Global<u8> = Global::new(0x00b5_94ec);
    /// The character with Tidus's model (`Ch_CbMapping`).
    pub const TIDUS: Global<u32> = Global::new(0x012f_bc60);
    /// `Ch_CbArcMapping`'s default for block `+0x2c`.
    pub const BLK_DEFAULT_2C: Global<f32> = Global::new(0x00b9_2314);
    /// `Ch_CbArcMapping`'s default for block `+0x5c` (2.0).
    pub const BLK_DEFAULT_5C: Global<f32> = Global::new(0x00b9_2268);
    /// `Ch_CbArcMapping`'s default for block `+0x3c`.
    pub const BLK_DEFAULT_3C: Global<f32> = Global::new(0x00b1_f2cc);

    fn add(mem: &mut impl Mem, at: u32, d: u32) {
        let v: u32 = mem.read(at);
        mem.write(at, v.wrapping_add(d));
    }

    fn add_nonzero(mem: &mut impl Mem, at: u32, d: u32) {
        let v: u32 = mem.read(at);
        if v != 0 {
            mem.write(at, v.wrapping_add(d));
        }
    }

    fn u16_at(mem: &impl Mem, at: u32) -> u32 {
        u32::from(mem.read::<u16>(at))
    }

    /// The self-pointer difference: `p - p[0]`, and `p[0] = p` after it.
    fn delta(mem: &impl Mem, p: u32) -> u32 {
        p.wrapping_sub(mem.read(p))
    }

    /// `Ch_ModelMapping(header)` (`0x00827590`): a model header and its
    /// models (meshes, cloths, tables).
    pub fn ch_model_mapping(mem: &mut impl Mem, m: u32) {
        let d = delta(mem, m);
        if d == 0 {
            return;
        }
        let p14: u32 = mem.read(m + 0x14);
        add(mem, m + 0x10, d);
        add(mem, m + 0x1c, d);
        mem.write(m, m);
        if p14 != 0 {
            mem.write(m + 0x14, p14.wrapping_add(d));
        }
        add_nonzero(mem, m + 0x24, d);
        let p30: u32 = mem.read(m + 0x30);
        add(mem, m + 0x18, d);
        if p30 != 0 {
            mem.write(m + 0x30, p30.wrapping_add(d));
        }
        let mut i = 0;
        while i < u16_at(mem, m + 6) {
            let e = model_entry(i, mem.read(m + 0x10), u16_at(mem, m + 4));
            add(mem, e + 8, d);
            add(mem, e + 0xc, d);
            add(mem, e + 0x10, d);
            let mut j = 0u32;
            while j < u16_at(mem, e + 6) {
                let t = mem.read::<u32>(e + 0x10).wrapping_add(j.wrapping_mul(12));
                add(mem, t + 8, d);
                j += 1;
            }
            add(mem, e + 0x1c, d);
            let mut j = 0u32;
            while j < u16_at(mem, e + 0x18) {
                let t = mem.read::<u32>(e + 0x1c).wrapping_add(j.wrapping_mul(0x24));
                add(mem, t + 0xc, d);
                add(mem, t + 8, d);
                add_nonzero(mem, t + 0x14, d);
                add_nonzero(mem, t + 0x20, d);
                j += 1;
            }
            i += 1;
        }
        let mut i = 0u32;
        while i < u16_at(mem, m + 0xc) {
            let t = mem.read::<u32>(m + 0x18).wrapping_add(4 * i);
            add(mem, t, d);
            i += 1;
        }
        let t14: u32 = mem.read(m + 0x14);
        let mut i = 0u32;
        while i < u16_at(mem, m + 8) {
            add(mem, t14.wrapping_add(8 + 12 * i), d);
            i += 1;
        }
        let t24: u32 = mem.read(m + 0x24);
        let mut i = 0u32;
        while i < u16_at(mem, m + 0x20) {
            add_nonzero(mem, t24.wrapping_add(8 + 12 * i), d);
            i += 1;
        }
    }

    fn zero_key(mem: &mut impl Mem, p: u32, key: [u8; 3]) {
        let b: [u8; 3] = mem.read(p);
        if b == key {
            mem.write(p, [0u8; 4]);
        }
    }

    /// `Ch_TexDataMapping(tex)` (`0x0083cba0`): texture data (versions from
    /// `0x1029`): its tables, each clut, and each image's data pointer. A
    /// clut that starts with the colour key `00 ff 00` or `ff 00 00` gets
    /// its first entry cleared.
    pub fn ch_tex_data_mapping(mem: &mut impl Mem, t: u32) {
        if t == 0 {
            return;
        }
        let version = u16_at(mem, t + 0x10);
        if version < 0x1029 {
            return;
        }
        let d = delta(mem, t);
        if d == 0 {
            return;
        }
        add(mem, t + 0x14, d);
        add(mem, t + 0x18, d);
        add(mem, t + 0x1c, d);
        mem.write(t, t);
        if version >= 0x1126 {
            add_nonzero(mem, t + 0x20, d);
        }
        let mut i = 0u32;
        while i < u16_at(mem, t + 8) {
            let c = mem.read::<u32>(t + 0x18).wrapping_add(8 * i);
            add(mem, c + 4, d);
            let p: u32 = mem.read(mem.read::<u32>(t + 0x18).wrapping_add(8 * i) + 4);
            zero_key(mem, p, [0, 0xff, 0]);
            let p: u32 = mem.read(mem.read::<u32>(t + 0x18).wrapping_add(8 * i) + 4);
            zero_key(mem, p, [0xff, 0, 0]);
            i += 1;
        }
        let mut i = 0u32;
        while i < u16_at(mem, t + 0xc) {
            let e = mem.read::<u32>(t + 0x1c).wrapping_add(16 * i);
            add_nonzero(mem, e + 0xc, d);
            i += 1;
        }
    }

    /// `Ch_ArcMapping(arc)` (`0x008256f0`): a character archive's entries,
    /// then the model header (entry 0), texture data (1) and pointer list
    /// (4) inside them.
    pub fn ch_arc_mapping(mem: &mut impl Mem, arc: u32) {
        let d = delta(mem, arc);
        if d == 0 {
            return;
        }
        mem.write(arc, arc);
        let mut i = 0u32;
        while i < mem.read::<u32>(arc + 4) {
            let at = arc.wrapping_add(0x10 + 8 * i);
            let p: u32 = mem.read(at);
            if p != 0 {
                mem.write(at, p.wrapping_add(d));
                match i {
                    0 => {
                        let h: u32 = mem.read(arc + 0x10);
                        ch_model_mapping(mem, h);
                    }
                    1 => {
                        let t: u32 = mem.read(arc + 0x18);
                        ch_tex_data_mapping(mem, t);
                    }
                    4 => ch_rel_elem_mapping(mem, arc + 0x30, d),
                    _ => {}
                }
            }
            i += 1;
        }
    }

    /// `Ch_CbArcMapping(blk, arc)` (`0x00825ee0`): fixes up `arc` and fills
    /// the model block from it, from the model parameters (entry 9) when
    /// there are any, else with defaults.
    pub fn ch_cb_arc_mapping(mem: &mut impl Mem, b: u32, arc: u32) {
        ch_arc_mapping(mem, arc);
        mem.write(b + 0x6c, arc);
        let v: u32 = mem.read(arc + 0x10);
        mem.write(b + 4, v);
        let v: u32 = mem.read(arc + 0x18);
        mem.write(b + l::BLK_TEX, v);
        let v: u32 = mem.read(arc + 0x1c);
        mem.write(b + 0x18, v);
        let v: u16 = mem.read(arc + 0x34);
        mem.write(b + 0x110, v);
        let v: u32 = mem.read(arc + 0x30);
        mem.write(b + 0x114, v);
        let n: u32 = mem.read(arc + 4);
        let v: u32 = if n > 10 { mem.read(arc + 0x60) } else { 0 };
        mem.write(b + 0x120, v);
        let kind = mem.read::<u32>(b) & 0xf000;
        let put = |mem: &mut _, at: u32, v: f32| crate::chr::bridge::put_f(mem, b + at, v);
        put(mem, 0x1c, 1.0);
        put(mem, 0x20, 1.0);
        mem.write(b + 0x42, 0x102u16);
        put(mem, 0x24, 1.0);
        mem.write(b + 0x40, 0u8);
        mem.write(b + 0x60, 0x7f80_8080u32);
        let c = BLK_DEFAULT_2C.get(mem);
        put(mem, 0x2c, c);
        mem.write(b + 0x64, 0u16);
        for at in [0x30, 0x34, 0x38] {
            put(mem, at, 1.0);
        }
        for at in [0x4c, 0x48, 0x44] {
            put(mem, at, 0.0);
        }
        put(mem, 0x50, 1.0);
        put(mem, 0x58, 1.0);
        let c = BLK_DEFAULT_5C.get(mem);
        put(mem, 0x5c, c);
        if kind == 0x2000 {
            mem.write(b + 0x43, 0u8);
            mem.write(b + 0x40, 1u8);
        }
        mem.write(b + 0x54, 0u32);
        let c = BLK_DEFAULT_3C.get(mem);
        put(mem, 0x3c, c);
        if n <= 9 {
            return;
        }
        let p: u32 = mem.read(arc + 0x58);
        let v: u32 = mem.read(p);
        mem.write(b + 0x54, v);
        let scale = x87::load(f(mem, p + 0xc));
        mem.write(b + l::BLK_DATA_SCALE, scale);
        let inv = Reg::constant(1000.0).div_st(Reg::load(scale)).store();
        mem.write(b + 0x3c, inv);
        for (to, from) in [
            (0x28, 0x14),
            (0x2c, 8),
            (0x1c, 4),
            (0x20, 0x10),
            (0x24, 0x18),
            (0x34, 0x1c),
            (0x38, 0x20),
        ] {
            let v = f(mem, p + from);
            put(mem, to, v);
        }
        for (to, from) in [(0x40, 0x24), (0x42, 0x26), (0x43, 0x27)] {
            let v: u8 = mem.read(p + from);
            mem.write(b + to, v);
        }
        for (to, from) in [(0x44, 0x28), (0x48, 0x2c), (0x4c, 0x30), (0x50, 0x34)] {
            let v = f(mem, p + from);
            put(mem, to, v);
        }
        if mem.read::<i32>(p) < 0x1624 {
            return;
        }
        for (to, from) in [(0x58, 0x38), (0x5c, 0x3c)] {
            let v = f(mem, p + from);
            put(mem, to, v);
        }
        let v: [u8; 6] = mem.read(p + 0x40);
        mem.write(b + 0x60, v);
    }

    /// Whether the C string at `a` equals the one at `b`.
    fn str_eq(mem: &impl Mem, a: u32, b: u32) -> bool {
        let mut i = 0u32;
        loop {
            let (x, y): (u8, u8) = (mem.read(a.wrapping_add(i)), mem.read(b.wrapping_add(i)));
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
            i += 1;
        }
    }

    /// `Ch_CbMapping(chr, blk)` (`0x00826070`): a character takes its model
    /// block: id, name, model header, and its models' entries; the one with
    /// Tidus's model is noted in [`TIDUS`].
    pub fn ch_cb_mapping(mem: &mut impl Mem, chr: u32, b: u32) {
        let id: u16 = mem.read(b);
        mem.write(chr + l::ID, id);
        let name = b + l::BLK_NAME;
        mem.write(chr + l::NAME, name);
        if str_eq(mem, name, NAME_C001.ptr(mem).addr) || str_eq(mem, name, NAME_C101.ptr(mem).addr)
        {
            TIDUS.set(mem, chr);
        }
        let models: u32 = mem.read(chr + l::MODELS);
        let h: u32 = mem.read(b + 4);
        mem.write(chr + l::MODEL_HEADER, h);
        if models == 0 {
            return;
        }
        let mut i = 0u32;
        while i < u16_at(mem, h + 6) {
            let e = model_entry(i, mem.read(h + 0x10), u16_at(mem, h + 4));
            let models: u32 = mem.read(chr + l::MODELS);
            mem.write(models.wrapping_add(i.wrapping_mul(l::MODEL_SIZE)), e);
            i += 1;
        }
    }

    /// `Ch_ElemRemapping(chr, delta)` (`0x00826980`): each element's data
    /// pointer, except those at the character's own `+0x1c8`.
    pub fn ch_elem_remapping(mem: &mut impl Mem, chr: u32, d: u32) {
        let mut e: u32 = mem.read(chr + l::ELEMS);
        let own = chr + 0x1c8;
        let mut i = 0u32;
        while i < u16_at(
            mem,
            mem.read::<u32>(chr + l::MODEL_HEADER) + l::HEADER_ELEMS,
        ) {
            let p: u32 = mem.read(e);
            if p != own {
                mem.write(e, p.wrapping_add(d));
            }
            e = e.wrapping_add(l::ELEM_SIZE);
            i += 1;
        }
    }

    /// `Ch_MotionRemappingFcurve(chr, delta)` (`0x00839890`): the motion
    /// curve pointer at `+0x74c` and the nine curves per element at
    /// `+0x748` (0x2c bytes each).
    pub fn ch_motion_remapping_fcurve(mem: &mut impl Mem, chr: u32, d: u32) {
        let p: u32 = mem.read(chr + 0x74c);
        if p == 0 {
            return;
        }
        mem.write(chr + 0x74c, p.wrapping_add(d));
        let mut e: u32 = mem.read(chr + 0x748);
        let mut i = 0u32;
        while i < 9 * u16_at(
            mem,
            mem.read::<u32>(chr + l::MODEL_HEADER) + l::HEADER_ELEMS,
        ) {
            add_nonzero(mem, e, d);
            e = e.wrapping_add(0x2c);
            i += 1;
        }
    }

    /// `Ch_MoveMgrp(chr, delta)` (`0x00837590`): the character's motion
    /// group pointers after the group moved.
    pub fn ch_move_mgrp(mem: &mut impl Mem, chr: u32, d: u32) {
        let p714: u32 = mem.read(chr + 0x714);
        add(mem, chr + 0x7e8, d);
        add(mem, chr + 0x70c, d);
        add(mem, chr + 0x710, d);
        if p714 != 0 && mem.read::<u8>(chr + 0x738) == 0 {
            mem.write(chr + 0x714, p714.wrapping_add(d));
        }
        add_nonzero(mem, chr + 0x7f4, d);
        let mut n: u32 = mem.read(chr + 0x74c);
        while n != 0 {
            add(mem, n + 8, d);
            add(mem, n + 0xc, d);
            n = mem.read(n);
        }
    }

    /// `Ch_ClutMapping(chr, copy)` (`0x0083ca20`): lays the cluts out after
    /// the clut table (each at a 128-byte boundary, from the texture data's
    /// clut list), copying them there when `copy`.
    pub fn ch_clut_mapping(mem: &mut impl Mem, chr: u32, copy: u32) {
        let tex: u32 = mem.read(crate::chr::bridge::blk(mem, chr) + l::BLK_TEX);
        let cluts: u32 = mem.read(chr + l::CLUTS);
        if cluts == 0 {
            return;
        }
        let n = i32::from(mem.read::<i16>(chr + l::CLUT_TEXTURE_COUNT));
        let mut off = n.cast_unsigned().wrapping_shl(4);
        let mut i = 0u32;
        if n <= 0 {
            return;
        }
        let mut entry = cluts;
        loop {
            while off.wrapping_add(0x20) & 0x7f != 0 {
                off = off.wrapping_add(1);
            }
            let list: u32 = mem.read(tex + 0x18);
            let org: u32 = mem.read(list.wrapping_add(8 * i + 4));
            let ptr = cluts.wrapping_add(0x20).wrapping_add(off);
            mem.write(entry, ptr);
            let size = ((mem.read::<u32>(org.wrapping_sub(0x10)) & 0x7fff) + 2) << 4;
            mem.write(entry + 8, size);
            off = off.wrapping_add(0x20);
            let list: u32 = mem.read(tex + 0x18);
            let org2: u32 = mem.read(list.wrapping_add(8 * i + 4));
            mem.write(entry + 4, org2);
            let list: u32 = mem.read(tex + 0x18);
            let first: u32 = mem.read(list.wrapping_add(8 * i));
            mem.write(entry + 0xc, first);
            if copy != 0 {
                let len: u32 = mem.read(entry + 8);
                let src = mem.read::<u32>(entry + 4).wrapping_sub(0x20);
                let dst = mem.read::<u32>(entry).wrapping_sub(0x20);
                let mut buf = vec![0u8; len as usize];
                mem.read_bytes(src, &mut buf);
                mem.write_bytes(dst, &buf);
            }
            off = off.wrapping_add(mem.read(entry + 8));
            i += 1;
            entry += 16;
            if i.cast_signed() >= i32::from(mem.read::<i16>(chr + l::CLUT_TEXTURE_COUNT)) {
                break;
            }
        }
    }

    /// `Ch_MemRelinkBack(heap)` (`0x0083c6a0`): the block list from
    /// `heap+0xc` (`+0` next, `+4` previous, kind in the low 4 bits of
    /// `+0xc`, 0xf ends it): free blocks (kind 0) absorb the free ones
    /// after them, each block's next gets its previous pointer back;
    /// then `heap`'s first byte is cleared.
    pub fn ch_mem_relink_back(mem: &mut impl Mem, heap: u32) {
        let mut b: u32 = mem.read(heap + 0xc);
        while mem.read::<u8>(b + 0xc) & 0xf != 0xf {
            // kind 0: free
            if mem.read::<u8>(b + 0xc).trailing_zeros() >= 4 {
                loop {
                    let next: u32 = mem.read(b);
                    if mem.read::<u8>(next + 0xc) & 0xf != 0 {
                        break;
                    }
                    let after: u32 = mem.read(next);
                    mem.write(b, after);
                }
            }
            let next: u32 = mem.read(b);
            mem.write(next + 4, b);
            b = mem.read(b);
        }
        mem.write(heap, 0u8);
    }

    /// `Ch_MoveChrBlk(new, old)` (`0x008347a0`): a character archive moved
    /// from `old` to `new`: the (last) model block holding `old` takes
    /// `new` (`Ch_CbArcMapping`), and each active character on that block
    /// is mapped again (`Ch_CbMapping`, `Ch_TextureReMapping`, its texture
    /// animation moved, its other-character motion `+0x758` moved too).
    pub fn ch_move_chr_blk(mem: &mut impl Mem, new: u32, old: u32) {
        let base = crate::handle::bridge::CHR_BLK.ptr(mem).addr;
        let mut b = 0u32;
        for i in 0..40 {
            let at = base + i * l::BLK_SIZE;
            if mem.read::<u32>(at) != u32::MAX && mem.read::<u32>(at + 0x6c) == old {
                b = at;
            }
        }
        if b == 0 {
            return;
        }
        ch_cb_arc_mapping(mem, b, new);
        let d = new.wrapping_sub(old);
        crate::handle::bridge::each_active(mem, |m, c| {
            if m.read::<u32>(c + l::BLK) != b {
                return;
            }
            ch_cb_mapping(m, c, b);
            crate::texanim::bridge::ch_texture_re_mapping(m, c);
            if m.read::<u32>(c + 0x6fc) != 0 {
                let tex: u32 = m.read(m.read::<u32>(c + l::BLK) + l::BLK_TEX);
                let i: u32 = m.read(c + 0x700);
                crate::texanim::bridge::chr_texanim_memory_move(m, tex, i);
            }
            let mo: u32 = m.read(c + 0x758);
            if mo != 0 {
                m.write(c + 0x758, mo.wrapping_add(d));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries() {
        assert_eq!(model_entry(2, 0x1000, 0x831), 0x1030);
        assert_eq!(model_entry(2, 0x1000, 0x1314), 0x1050);
        assert_eq!(model_entry(2, 0x1000, 0x830), 0x1050);
    }
}
