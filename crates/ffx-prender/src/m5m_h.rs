//! M5 stage 1, list `map`, round 5 batch 4: the debug map tool's VRAM
//! fill (`rcuiVramWrite`): a GS image transfer of random bytes.

use ffx_bridge::Global;

/// `g_RcuiWork`, `g_RcPrim2DPacket`.
pub const RCUI_WORK: Global<u32> = Global::new(0x0193_c0f0);
pub const PRIM_2D_PACKET: Global<u32> = Global::new(0x00c4_98dc);
/// The pixel-format size classes (one byte per PSM up to 0x3a): 0 four
/// bytes a pixel, 1 two, 2 a half, else one.
pub const PSM_CLASS: u32 = 0x0093_73bc;
/// The PSM codes the tool's format index picks.
pub const PSM_CODES: [u32; 13] = [
    0, 1, 2, 0xa, 0x13, 0x14, 0x1b, 0x24, 0x2c, 0x30, 0x31, 0x32, 0x3a,
];
/// `MSVCR110!rand` (the slot, cdecl).
pub const RAND_SLOT: u32 = 0x00b0_c420;

// ---- callees ----

/// `crt_memset_thunk(p, v, n)`, `sceDmaGetChan(n)`, `FlushCache(n)`,
/// `sceDmaSend(ch, packet)`, `sceDmaSync(ch, mode, timeout)` (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
pub const DMA_GET_CHAN: u32 = 0x0062_f6e0;
pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
pub const DMA_SEND: u32 = 0x0062_f710;
pub const DMA_SYNC: u32 = 0x0062_f730;

pub mod bridge {
    use super::{
        DMA_GET_CHAN, DMA_SEND, DMA_SYNC, FLUSH_CACHE, MEMSET, PRIM_2D_PACKET, PSM_CLASS,
        PSM_CODES, RAND_SLOT, RCUI_WORK,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `rcuiVramWrite()` -> `sceDmaSync`'s `eax`: `w x h` pixels (`+0xafc`,
    /// `+0xb00`) of one random byte, in bytes by the format (`+0xaec`),
    /// after the packet buffer's first KiB, sent to VRAM at `+0xae4`
    /// (width `+0xae8`) by a GIF packet: BITBLTBUF, TRXPOS 0, TRXREG,
    /// TRXDIR, then the image (quadwords). The format index reads the
    /// original's 13-entry PSM table (`frame`, its `ebp - 0x38`); an index
    /// past it reads the original's stack beyond the table (junk read; the
    /// tool's index is 0..12), and the size class is looked up by the
    /// index, not the PSM, as the original does.
    pub fn vram_write(h: &mut impl Host, frame: u32) -> u32 {
        let w = RCUI_WORK.get(h);
        let mut size = h.read::<u32>(w + 0xb00).wrapping_mul(h.read(w + 0xafc));
        let format: u32 = h.read(w + 0xaec);
        let packet = PRIM_2D_PACKET.get(h);
        let image = packet + 0x400;
        for (k, v) in PSM_CODES.iter().enumerate() {
            h.write(frame + 4 * k as u32, *v);
        }
        if format <= 0x3a {
            let class: u8 = h.read(h.rebase(PSM_CLASS) + format);
            size = match class {
                0 => size << 2,
                1 => size.wrapping_add(size),
                2 => {
                    let s = size.cast_signed();
                    ((s - (s >> 31)) >> 1).cast_unsigned()
                }
                _ => size,
            };
        }
        let rand: u32 = h.read(h.rebase(RAND_SLOT));
        let fill = h.call(rand, Conv::Cdecl, &[]) & 0xff;
        cdecl(h, MEMSET, &[image, fill, size]);
        for (k, v) in [0x1000_0005_u32, 0, 0, 0].into_iter().enumerate() {
            h.write(packet + 4 * k as u32, v);
        }
        let p = packet + 0x10;
        let w = RCUI_WORK.get(h);
        let index: u32 = h.read(w + 0xaec);
        let psm: u32 = h.read(frame.wrapping_add(index.wrapping_mul(4)));
        let width: u32 = h.read(w + 0xae8);
        let base: u32 = h.read(w + 0xae4);
        let bitblt = (((psm << 8) | width) << 16) | base;
        for (k, v) in [0, bitblt, 0x50, 0, 0, 0, 0x51, 0].into_iter().enumerate() {
            h.write(p + 4 * k as u32, v);
        }
        let p = p + 0x20;
        let w = RCUI_WORK.get(h);
        let height: u32 = h.read(w + 0xb00);
        let row: u32 = h.read(w + 0xafc);
        let row_sign = (row.cast_signed() >> 31).cast_unsigned();
        for (k, v) in [row, height | row_sign, 0x52, 0, 0, 0, 0x53, 0]
            .into_iter()
            .enumerate()
        {
            h.write(p + 4 * k as u32, v);
        }
        let s = size.cast_signed();
        let quads = (s + ((s >> 31) & 0xf)) >> 4;
        h.write(p + 0x20, quads as u16);
        h.write(p + 0x22, 0_u16);
        h.write(p + 0x24, image);
        h.write(p + 0x28, 0_u32);
        h.write(p + 0x2c, 0_u32);
        let ch = cdecl(h, DMA_GET_CHAN, &[2]);
        cdecl(h, FLUSH_CACHE, &[0]);
        let packet = PRIM_2D_PACKET.get(h);
        cdecl(h, DMA_SEND, &[ch, packet]);
        cdecl(h, DMA_SYNC, &[ch, 0, 0])
    }
}
