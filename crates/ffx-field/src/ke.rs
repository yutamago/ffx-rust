//! `Ke`: resource heaps and the lens flare objects of the `Ke` module.
//! Their pointers go through `set_ptr32`/`get_ptr32_func`, the identity
//! on PC (`ffx_system::stub`).

/// A resource heap header (`KeThResHd_Init`): `count` blocks of a stride
/// derived from `size` follow it from `+0x10`, linked in a free list.
pub mod res_hd {
    /// `+0x0`: blocks (`u8`).
    pub const COUNT: u32 = 0x0;
    /// `+0x1`: the size class (`u8`).
    pub const SIZE: u32 = 0x1;
    /// `+0x2`: bytes per block (`u16`).
    pub const STRIDE: u32 = 0x2;
    /// `+0x4`: the first free block.
    pub const FREE: u32 = 0x4;
    /// `+0x10`: the first block.
    pub const BLOCKS: u32 = 0x10;
}

/// Bytes per block for a size class: `(size - 1) * 48 + 0x80`, in the
/// original's 8- and 16-bit arithmetic.
#[must_use]
pub fn res_stride(size: u8) -> u16 {
    u16::from(size.wrapping_sub(1))
        .wrapping_mul(48)
        .wrapping_add(0x80)
}

/// The lens flares' default shape table (`KeLnsFls_Init` and siblings).
pub const LNS_DEFAULT: u32 = 0x00d5_2398;

/// The VFX packet position `KeLnsShp_Draw` records (`0x0230fd2c`).
pub const PACKET_POS: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0230_fd2c);
/// 65535.0, an `f64` in `.rdata` (`KeLnsClm_SetShp`).
pub const PHASE_SCALE: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b1_70d8);
/// The magic effects whose models share one instance
/// (`KeDttMdl_UseSingleInstance`).
pub const SINGLE_INSTANCE_MAGIC: [u32; 10] = [
    0x203, 0x15c, 0x15e, 0x182, 0x241, 0x20a, 0x23a, 0x29b, 0x1db, 0x46,
];

/// The `Ke` objects where the game keeps them, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_system::stub::ptr32;

    use super::{LNS_DEFAULT, res_hd, res_stride};

    /// `KeThResHd_Init(hd, count, size)` (`0x00764bb0`): the header and the
    /// free list through the blocks (`count` 0 still terminates at `+0x10`).
    pub fn ke_th_res_hd_init(mem: &mut impl Mem, hd: u32, count: u8, size: u8) {
        mem.write(hd + res_hd::SIZE, size);
        let stride = res_stride(size);
        mem.write(hd + res_hd::COUNT, count);
        mem.write(hd + res_hd::STRIDE, stride);
        mem.write(hd + res_hd::FREE, ptr32(0));
        let mut block = hd + res_hd::BLOCKS;
        mem.write(hd + res_hd::FREE, ptr32(block));
        for _ in 1..i32::from(count) {
            let next = block.wrapping_add(u32::from(stride));
            mem.write(block, ptr32(next));
            block = next;
        }
        mem.write(block, ptr32(0));
    }

    /// `KeThCp_Init(cp)` (`0x00764800`).
    pub fn ke_th_cp_init(mem: &mut impl Mem, cp: u32) {
        mem.write(cp, ptr32(0));
        mem.write(cp + 4, 0u8);
    }

    /// `KeLnsFls_Init(obj)` (`0x00769690`), and `KeLnsCrn_Init`
    /// (`0x0076a170`), the same code: the default shapes, no data.
    pub fn ke_lns_init(mem: &mut impl Mem, o: u32) {
        let shapes = mem.rebase(LNS_DEFAULT);
        mem.write(o, ptr32(shapes));
        mem.write(o + 4, ptr32(0));
    }

    /// `KeLnsClm_Init(obj)` (`0x00769ca0`): as [`ke_lns_init`], and byte
    /// `+9` cleared first.
    pub fn ke_lns_clm_init(mem: &mut impl Mem, o: u32) {
        mem.write(o + 9, 0u8);
        ke_lns_init(mem, o);
    }

    /// `KeLnsFlr_Init(obj, src)` (`0x0076a1d0`): the `u16` at `+0x12` of
    /// what `*src` points at, and byte `+2` cleared.
    pub fn ke_lns_flr_init(mem: &mut impl Mem, o: u32, src: u32) {
        let p = ptr32(mem.read(src));
        let v = mem.read::<u16>(p + 0x12);
        mem.write(o, v);
        mem.write(o + 2, 0u8);
    }

    // ---- round 4: lens flare shapes, the magic model instance rule ----

    /// `KeLnsLp_Init(obj)` (`0x0076a670`): its matrix at `+0x40` the
    /// identity, `+0xac` 1.0; returns `FFXVu0UnitMatrix`'s `eax`.
    pub fn ke_lns_lp_init(mem: &mut impl Mem, o: u32) -> u32 {
        let r = ffx_ps2::vu0::bridge::matrix_identity(mem, o.wrapping_add(0x40));
        mem.write(o.wrapping_add(0xac), 1f32);
        r
    }

    /// `KeLnsCrn_SetShp(obj, shp)` (`0x0076a1a0`): the shape, its first
    /// frame's duration (`u16` at `+0x12` of the shape's frame table) at
    /// `+8` and frame 0 at `+0xa`. Returns the table pointer's high half
    /// with that duration (what `eax` holds after `mov ax`).
    pub fn ke_lns_crn_set_shp(mem: &mut impl Mem, o: u32, shp: u32) -> u32 {
        mem.write(o.wrapping_add(4), ptr32(shp));
        let frames = ptr32(mem.read::<u32>(shp));
        let d = mem.read::<u16>(frames.wrapping_add(0x12));
        mem.write(o.wrapping_add(8), d);
        mem.write(o.wrapping_add(0xa), 0u8);
        (frames & 0xffff_0000) | u32::from(d)
    }

    /// `KeLnsShp_Draw(obj)` (`0x0076ab90`): reads its shape and state (as
    /// the original does, without using them), then `+0x18` = 0, `+0x28`
    /// = 0, `+0x24` = the packet position (`0x0230fd2c`), `+0x1c` = `0x11`,
    /// `+0x12` |= `0xb0`; returns `0xb0`.
    pub fn ke_lns_shp_draw(mem: &mut impl Mem, o: u32) -> u32 {
        let shp = ptr32(mem.read::<u32>(o));
        let _ = ptr32(mem.read::<u32>(shp));
        let _ = ptr32(mem.read::<u32>(o.wrapping_add(4)));
        mem.write(o.wrapping_add(0x18), 0u32);
        mem.write(o.wrapping_add(0x28), 0u8);
        let pk = super::PACKET_POS.get(mem);
        mem.write(o.wrapping_add(0x24), pk);
        mem.write(o.wrapping_add(0x1c), 0x11u16);
        let f = mem.read::<u16>(o.wrapping_add(0x12));
        mem.write(o.wrapping_add(0x12), f | 0xb0);
        0xb0
    }

    /// `KeLnsShp_Update(obj)` (`0x0076abe0`): the shape animation's clock
    /// (`u16` at the state's `+0`) moves 0x200 a frame; past the frame's
    /// duration (16-bit signed compares) it goes to the next frame, and
    /// past the last frame back to 0 when the frame's `+0x14` bit 7 is
    /// set, else stays on the last. Returns the `eax` it ends with: the
    /// new clock, the shape table, or 0.
    pub fn ke_lns_shp_update(mem: &mut impl Mem, o: u32) -> u32 {
        let st = ptr32(mem.read::<u32>(o.wrapping_add(4)));
        let shp = ptr32(mem.read::<u32>(o));
        let mut idx = mem.read::<u8>(st.wrapping_add(2));
        let table = ptr32(mem.read::<u32>(shp));
        let frame = table.wrapping_add(u32::from(idx) * 8);
        let t = mem.read::<u16>(st).wrapping_add(0x200);
        mem.write(st, t);
        let dur = mem.read::<u16>(frame.wrapping_add(0x12));
        #[allow(clippy::cast_possible_wrap)] // 16-bit signed compares
        if (t as i16) < (dur as i16) {
            return u32::from(t);
        }
        mem.write(st, t.wrapping_sub(dur));
        idx = idx.wrapping_add(1);
        mem.write(st.wrapping_add(2), idx);
        let shp = ptr32(mem.read::<u32>(o));
        let idx = mem.read::<u8>(st.wrapping_add(2));
        let hdr = ptr32(mem.read::<u32>(shp));
        #[allow(clippy::cast_possible_wrap)]
        if i16::from(idx) < (mem.read::<u16>(hdr.wrapping_add(6)) as i16) {
            return hdr;
        }
        mem.write(st, 0u16);
        if mem.read::<u8>(frame.wrapping_add(0x14)) & 0x80 != 0 {
            mem.write(st.wrapping_add(2), 0u8);
        } else {
            mem.write(st.wrapping_add(2), idx.wrapping_sub(1));
        }
        0
    }

    /// `KeLnsClm_SetShp(obj, shp, seed)` (`0x00769cd0`): the shape, each of
    /// the `+8` (`u8`) elements' duration (`+0x10 + 8i`) and frame 0; the
    /// first time (`+9` bit 0 clear) each element's phase (`+0x14 + 8i`)
    /// `s / 65535.0` (an `f64` in `.rdata`) with `s = s * 0x80d + 7` in 16
    /// bits from the seed, then sets the bit.
    pub fn ke_lns_clm_set_shp(mem: &mut impl Mem, o: u32, shp: u32, seed: u32) {
        mem.write(o.wrapping_add(4), ptr32(shp));
        let n = u32::from(mem.read::<u8>(o.wrapping_add(8)));
        for i in 0..n {
            let frames = ptr32(mem.read::<u32>(shp));
            let d = mem.read::<u16>(frames.wrapping_add(0x12));
            let e = o.wrapping_add(0x10 + 8 * i);
            mem.write(e, d);
            mem.write(e.wrapping_add(2), 0u8);
        }
        if mem.read::<u8>(o.wrapping_add(9)) & 1 != 0 {
            return;
        }
        let k = super::PHASE_SCALE.get(mem);
        let mut s = seed;
        let mut i = 0;
        while i < u32::from(mem.read::<u8>(o.wrapping_add(8))) {
            s = s.wrapping_mul(0x80d).wrapping_add(7) & 0xffff;
            // fild, fstp m32 (exact), fld, fdiv by the f64
            let v = f64::from(s) / k;
            mem.write(o.wrapping_add(0x14 + 8 * i), crate::x87::store(v));
            i += 1;
        }
        let f = mem.read::<u8>(o.wrapping_add(9));
        mem.write(o.wrapping_add(9), f | 1);
    }

    /// `KeDttMdl_UseSingleInstance()` (`0x0072e6d0`): 1 for the magic
    /// effects that share one model instance (ids `0x203`, `0x15c`,
    /// `0x15e`, `0x182`, `0x241`, `0x20a`, `0x23a`, `0x29b`, `0x1db`,
    /// `0x46`), else 0.
    pub fn ke_dtt_mdl_use_single_instance(mem: &impl Mem) -> u32 {
        let id = ffx_system::instance::bridge::magic_id(mem);
        u32::from(super::SINGLE_INSTANCE_MAGIC.contains(&id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strides_grow_by_48() {
        assert_eq!(res_stride(1), 0x80);
        assert_eq!(res_stride(2), 0xb0);
        assert_eq!(res_stride(0), 0x80u16.wrapping_add(255 * 48));
    }
}
