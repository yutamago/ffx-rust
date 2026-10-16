//! `rcPacketData`: a PS2 DMA packet being written (`docs/formats/field.md`).
//!
//! `rcPacketDataOpenAdr` reserves the first quadword of a buffer for a DMA
//! tag and writes after it; `rcPacketDataCloseAdr` appends a GIF tag with
//! the end-of-packet bit and fills the DMA tag in: ID 7 ("end") and the
//! quadword count after it.

/// `+0x0`: where the packet starts (its DMA tag).
pub const START: u32 = 0x0;
/// `+0x4`: where the next quadword goes.
pub const CURSOR: u32 = 0x4;

/// The closing GIF tag's four words: `EOP` (bit 15) and `NREG` = 1.
pub const END_GIF_TAG: [u32; 4] = [0x8000, 0x1000_0000, 0, 0];

/// The DMA tag of a packet from `start` to `end` (the cursor after the GIF
/// tag): `QWC` = quadwords after the tag, `ID` = 7.
#[must_use]
pub const fn dma_end_tag(start: u32, end: u32) -> u32 {
    (end.wrapping_sub(start) >> 4).wrapping_sub(1) | 0x7000_0000
}

/// The packets where the game keeps them, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{CURSOR, END_GIF_TAG, START, dma_end_tag};

    /// `rcPacketDataInit(packet)` (`0x00927a20`): no buffer.
    pub fn rc_packet_data_init(mem: &mut impl Mem, p: u32) {
        mem.write(p + CURSOR, 0u32);
        mem.write(p + START, 0u32);
    }

    /// `rcPacketDataOpenAdr(packet, buffer)` (`0x00927a70`): starts a packet
    /// at `buffer`, its first quadword kept for the DMA tag.
    pub fn rc_packet_data_open_adr(mem: &mut impl Mem, p: u32, buffer: u32) {
        mem.write(p + START, buffer);
        mem.write(p + CURSOR, buffer.wrapping_add(0x10));
    }

    /// `rcPacketDataCloseAdr(packet)` (`0x009275b0`): appends the end GIF
    /// tag and writes the DMA tag's first word.
    pub fn rc_packet_data_close_adr(mem: &mut impl Mem, p: u32) {
        let at = mem.read::<u32>(p + CURSOR);
        mem.write(at, END_GIF_TAG);
        let end = at.wrapping_add(0x10);
        mem.write(p + CURSOR, end);
        let start = mem.read::<u32>(p + START);
        mem.write(start, dma_end_tag(start, end));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tag_counts_quadwords_after_it() {
        // tag, two quadwords of data, the GIF tag: 3 after the tag
        assert_eq!(dma_end_tag(0x1000, 0x1040), 0x7000_0003);
    }
}
