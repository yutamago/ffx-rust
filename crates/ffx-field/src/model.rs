//! `rcbg` scene objects: models and their primitive packets, layers,
//! texture transfers (`trx`), palettes, animations and particle
//! simulations (`docs/formats/field.md`).

use ffx_bridge::Global;

/// Bytes per primitive record of each packet type (`g_GuidePacketStrides`,
/// 8 bytes, in `.data`).
pub const PACKET_STRIDES: Global<u8> = Global::new(0x00c5_e790);

/// A model (`rcbgModel`).
pub mod obj {
    /// `+0x14`: its OMD blob (`rcbgModelCreateFromOMD`).
    pub const OMD: u32 = 0x14;
    /// `+0xc0`: the vertex-to-world matrix (`Guide_MeshObject`'s users).
    pub const MATRIX: u32 = 0xc0;
}

/// A model's OMD blob: primitive packets and vertices.
pub mod omd {
    /// `+0x4`: offset of the first packet (`rcbgModelGetPrimHeader`).
    pub const PACKETS: u32 = 0x4;
    /// `+0x8`: offset of the vertices (three `i16` each).
    pub const VERTICES: u32 = 0x8;
    /// `+0x10`: primitives in all packets (`i16`, `rcbgModelSetAlpha`).
    pub const PRIMS: u32 = 0x10;
    /// `+0x12`: vertex count (`u16`).
    pub const VERTEX_COUNT: u32 = 0x12;
    /// `+0x20`: where `rcbgModelSetAlpha` starts walking the packets (it
    /// does not read [`PACKETS`]).
    pub const FIRST_PACKET: u32 = 0x20;
}

/// A primitive packet: a 16-byte header, then `count` records of the
/// type's stride ([`super::PACKET_STRIDES`]).
pub mod packet {
    /// `+0x1`: the type (`i8`, 0..=7). Types 0..=3 have three vertices, 4..=7
    /// four (`rcbgModelSetAlpha`); type 0 records keep their three vertex
    /// indices at `+0xc` (the minimap).
    pub const TYPE: u32 = 0x1;
    /// `+0x2`: records in the packet (`i16`).
    pub const COUNT: u32 = 0x2;
    /// Header size.
    pub const HEADER: u32 = 0x10;
    /// A record starts with one RGBA colour per vertex; the alpha bytes.
    pub const ALPHAS: [u32; 4] = [0x3, 0x7, 0xb, 0xf];
}

/// Bytes a packet takes, header included: `0x10 + stride * count`, with
/// the original's signed arithmetic.
#[must_use]
pub fn packet_size(stride: u8, count: i16) -> u32 {
    #[allow(clippy::cast_sign_loss)] // two's complement, as the original adds it
    let records = (i32::from(stride) * i32::from(count)) as u32;
    packet::HEADER.wrapping_add(records)
}

/// A layer (`rcbgLayerLoad`).
pub mod layer {
    /// `+0x4`: width (`i32`).
    pub const WIDTH: u32 = 0x4;
    /// `+0x8`: height (`i32`).
    pub const HEIGHT: u32 = 0x8;
    /// Where `rcbgLayerLoadCalcCenter` stores half the width.
    pub const CENTER_X: [u32; 3] = [0x68, 0x64, 0x4c];
    /// Where it stores half the height.
    pub const CENTER_Y: [u32; 3] = [0x70, 0x6c, 0x50];
}

/// Half of a layer dimension, as the original computes it: the `i32`
/// rounded to `f32`, then halved.
#[must_use]
#[allow(clippy::cast_precision_loss)] // the original rounds the same way
pub fn half(n: i32) -> u32 {
    crate::x87::store(f64::from(n as f32) * 0.5)
}

/// A texture transfer object (`rcbgTrxInit`, `0x40` bytes).
pub mod trx {
    /// `+0x0`: the object type.
    pub const KIND: u32 = 0x0;
    /// The type `rcbgTrxInit` gives it.
    pub const KIND_TRX: u32 = 0x0002_0002;
    /// `+0x4`: the delete function (`rcbgTrxDelete`, `0x0092d450`).
    pub const DELETE: u32 = 0x4;
    /// `+0xc`: the draw function (`rcbgTrxDraw`, `0x0092d470`).
    pub const DRAW: u32 = 0xc;
    /// `+0x30`: entries (`rcbgTrxGetCnt`).
    pub const COUNT: u32 = 0x30;
    /// `+0x34`: bytes per entry.
    pub const ENTRY_SIZE: u32 = 0x34;
    /// `+0x38`: the entries (`rcbgTrxGetEntry`); an entry's data follows
    /// its 16-byte header (`rcbgTrxGetEntryData`).
    pub const ENTRIES: u32 = 0x38;
    /// `+0x3c`: the transfer to run on the next draw; -1 for none
    /// (`rcbgTrxSetNo`, cleared by `rcbgTrxDraw`).
    pub const NO: u32 = 0x3c;
    /// The original's delete function.
    pub const DELETE_FN: u32 = 0x0092_d450;
    /// The original's draw function.
    pub const DRAW_FN: u32 = 0x0092_d470;
    /// Words `rcbgTrxInit` clears (`+0x14` it leaves).
    pub const CLEARED: [u32; 11] = [
        0x8, 0x10, 0x18, 0x1c, 0x20, 0x24, 0x28, 0x2c, 0x30, 0x34, 0x38,
    ];
}

/// A palette (`rcbgPalInit`, 16 bytes).
pub mod pal {
    /// `+0x8`: the palette number (`u16`, `rcbgPalSetNo`).
    pub const NO: u32 = 0x8;
}

/// An animation (`rcbgAnimeInit`, 32 bytes).
pub mod anime {
    /// `+0x14`: set to `0x100` by `rcbgAnimeInit` (the rest is cleared).
    pub const RATE: u32 = 0x14;
}

/// A particle simulation (`rcbgPSimInit`, `0xb0` bytes).
pub mod psim {
    /// Size.
    pub const SIZE: u32 = 0xb0;
    /// The object type `rcbgPSimInit` gives it.
    pub const KIND_PSIM: u32 = 0x0002_0004;
    /// Its delete, process and draw functions (`+4`, `+8`, `+0xc`; the
    /// second is `rcbgPSimProc`).
    pub const FUNCTIONS: [u32; 3] = [0x0091_fd40, 0x0091_fe70, 0x0091_fd60];
    /// Where its model's blob keeps the input particles (`rcbgPSimGetOmdIn`).
    pub const OMD_IN: u32 = 0x20;
    /// `+0x14`: its model.
    pub const MODEL: u32 = 0x14;
    /// `+0xa4`: the initial count (`rcbgPSimSetInitCnt`).
    pub const INIT_CNT: u32 = 0xa4;
}

/// A model's scale (`rcbgModelSetScale` stores ten times it on the
/// matrix diagonal; `rcbgModelGetScale` divides by 10).
pub const SCALE_FACTOR: f64 = 10.0;

/// `rcPad16(n)`: bytes from `n` to the next multiple of 16, with the C
/// remainder (a negative `n` gives 17..=31).
#[must_use]
pub const fn pad16(n: i32) -> i32 {
    let r = n % 16;
    if r != 0 { 16 - r } else { 0 }
}

/// `rcLog2Ceil(n)`: the smallest `k` in 1..=16 with `n <= 1 << k`
/// (16 when none is).
#[must_use]
pub fn log2_ceil(n: i32) -> u32 {
    let mut k = 1u32;
    let mut p = 2u32;
    loop {
        if n <= p.cast_signed() {
            return k;
        }
        p = p.rotate_left(1);
        k += 1;
        if k >= 16 {
            return k;
        }
    }
}

/// The scene objects where the game keeps them, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{
        PACKET_STRIDES, anime, half, layer, obj, omd, packet, packet_size, pal, psim, trx,
    };

    fn stride(mem: &impl Mem, kind: i8) -> u8 {
        #[allow(clippy::cast_sign_loss)]
        // a negative type indexes before the table, as the original
        let at = PACKET_STRIDES
            .ptr(mem)
            .addr
            .wrapping_add(i32::from(kind) as u32);
        mem.read(at)
    }

    /// `rcbgModelCreateFromOMD(model, omd)` (`0x00927d70`).
    pub fn rcbg_model_create_from_omd(mem: &mut impl Mem, m: u32, blob: u32) {
        mem.write(m + obj::OMD, blob);
    }

    /// `rcbgModelGetPrimHeader(model, n)` (`0x009282c0`): the header of
    /// packet `n` (0 or negative: the first).
    pub fn rcbg_model_get_prim_header(mem: &impl Mem, m: u32, n: i32) -> u32 {
        let blob = mem.read::<u32>(m + obj::OMD);
        let mut p = blob.wrapping_add(mem.read::<u32>(blob + omd::PACKETS));
        for _ in 0..n.max(0) {
            let s = stride(mem, mem.read(p + packet::TYPE));
            p = p.wrapping_add(packet_size(s, mem.read(p + packet::COUNT)));
        }
        p
    }

    /// `rcbgModelGetPrim(model, n)` (`0x009282a0`): packet `n`'s records.
    pub fn rcbg_model_get_prim(mem: &impl Mem, m: u32, n: i32) -> u32 {
        rcbg_model_get_prim_header(mem, m, n).wrapping_add(packet::HEADER)
    }

    /// `rcbgModelSetAlpha(model, alpha)` (`0x00928460`): every vertex
    /// colour's alpha, packet by packet until the blob's primitive count is
    /// used up.
    pub fn rcbg_model_set_alpha(mem: &mut impl Mem, m: u32, alpha: u8) {
        let blob = mem.read::<u32>(m + obj::OMD);
        let mut left = i32::from(mem.read::<i16>(blob + omd::PRIMS));
        let mut p = blob + omd::FIRST_PACKET;
        while left > 0 {
            let kind = mem.read::<i8>(p + packet::TYPE);
            let count = i32::from(mem.read::<i16>(p + packet::COUNT));
            let mut r = p.wrapping_add(packet::HEADER);
            for _ in 0..count.max(0) {
                let corners = if kind >= 4 { 4 } else { 3 };
                for &o in &packet::ALPHAS[..corners] {
                    mem.write(r.wrapping_add(o), alpha);
                }
                r = r.wrapping_add(u32::from(stride(mem, kind)));
            }
            left -= count;
            p = r;
        }
    }

    /// `rcbgLayerLoadCalcCenter(layer)` (`0x0092b270`): half the width and
    /// height, each stored three times.
    pub fn rcbg_layer_load_calc_center(mem: &mut impl Mem, l: u32) {
        let x = half(mem.read(l + layer::WIDTH));
        let y = half(mem.read(l + layer::HEIGHT));
        for o in layer::CENTER_X {
            mem.write(l + o, x);
        }
        for o in layer::CENTER_Y {
            mem.write(l + o, y);
        }
    }

    /// `rcbgTrxInit(trx)` (`0x0092d510`).
    pub fn rcbg_trx_init(mem: &mut impl Mem, t: u32) {
        mem.write(t + trx::KIND, trx::KIND_TRX);
        let delete = mem.rebase(trx::DELETE_FN);
        let draw = mem.rebase(trx::DRAW_FN);
        mem.write(t + trx::DELETE, delete);
        mem.write(t + trx::DRAW, draw);
        for o in trx::CLEARED {
            mem.write(t + o, 0u32);
        }
        mem.write(t + trx::NO, u32::MAX);
    }

    /// `rcbgTrxSetNo(trx, no)` (`0x0092d670`).
    pub fn rcbg_trx_set_no(mem: &mut impl Mem, t: u32, no: u32) {
        mem.write(t + trx::NO, no);
    }

    /// `rcbgPalInit(pal)` (`0x009299b0`).
    pub fn rcbg_pal_init(mem: &mut impl Mem, p: u32) {
        mem.write(p, [0u32; 4]);
    }

    /// `rcbgPalSetNo(pal, no)` (`0x00929ab0`).
    pub fn rcbg_pal_set_no(mem: &mut impl Mem, p: u32, no: u16) {
        mem.write(p + pal::NO, no);
    }

    /// `rcbgAnimeInit(anime)` (`0x009308a0`).
    pub fn rcbg_anime_init(mem: &mut impl Mem, a: u32) {
        mem.write(a, [0u32; 8]);
        mem.write(a + anime::RATE, 0x100u32);
    }

    /// `rcbgPSimSetInitCnt(psim, n)` (`0x0091fef0`).
    pub fn rcbg_psim_set_init_cnt(mem: &mut impl Mem, p: u32, n: u32) {
        mem.write(p + psim::INIT_CNT, n);
    }

    /// `rcbgModelGetScale(model)` (`0x00928300`): the matrix's first
    /// element over 10.
    pub fn rcbg_model_get_scale(mem: &impl Mem, m: u32) -> f32 {
        let x = crate::x87::load(mem.read(m + obj::MATRIX));
        f32::from_bits(crate::x87::store(x / super::SCALE_FACTOR))
    }

    /// `rcbgTrxGetCnt(trx)` (`0x0092d4c0`).
    pub fn rcbg_trx_get_cnt(mem: &impl Mem, t: u32) -> u32 {
        mem.read(t + trx::COUNT)
    }

    /// `rcbgTrxGetEntry(trx, n)` (`0x0092d4d0`).
    pub fn rcbg_trx_get_entry(mem: &impl Mem, t: u32, n: u32) -> u32 {
        let size = mem.read::<u32>(t + trx::ENTRY_SIZE);
        size.wrapping_mul(n)
            .wrapping_add(mem.read(t + trx::ENTRIES))
    }

    /// `rcbgTrxGetEntryData(trx, n)` (`0x0092d4f0`).
    pub fn rcbg_trx_get_entry_data(mem: &impl Mem, t: u32, n: u32) -> u32 {
        rcbg_trx_get_entry(mem, t, n).wrapping_add(0x10)
    }

    /// `rcbgPSimInit(psim)` (`0x0091fe00`).
    pub fn rcbg_psim_init(mem: &mut impl Mem, p: u32) {
        mem.write_bytes(p, &[0; psim::SIZE as usize]);
        mem.write(p, psim::KIND_PSIM);
        for (i, f) in (1u32..).zip(psim::FUNCTIONS) {
            let at = mem.rebase(f);
            mem.write(p + 4 * i, at);
        }
    }

    /// `rcbgPSimGetOmdIn(psim)` (`0x0091fdc0`).
    pub fn rcbg_psim_get_omd_in(mem: &impl Mem, p: u32) -> u32 {
        mem.read::<u32>(p + psim::MODEL).wrapping_add(psim::OMD_IN)
    }

    /// `rcbgFPm173End(fp)` (`0x00922b50`): `+0x24` = 1.
    pub fn rcbg_fpm173_end(mem: &mut impl Mem, f: u32) {
        mem.write(f + 0x24, 1u32);
    }

    /// `rcbgPSimClearOMD(psim)` (`0x0091f9b0`): detaches its model's blob.
    pub fn rcbg_psim_clear_omd(mem: &mut impl Mem, p: u32) {
        let m = mem.read::<u32>(p + psim::MODEL);
        if m != 0 {
            mem.write(m + obj::OMD, 0u32);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_sizes_are_signed() {
        assert_eq!(packet_size(0x14, 3), 0x10 + 0x3c);
        assert_eq!(packet_size(0x14, -1), 0x10u32.wrapping_sub(0x14));
    }

    #[test]
    fn halves_round_like_the_original() {
        assert_eq!(f32::from_bits(half(640)), 320.0);
        assert_eq!(f32::from_bits(half(16_777_217)), 8_388_608.0);
    }
}
