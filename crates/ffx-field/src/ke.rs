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
