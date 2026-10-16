//! `Abmap`: helpers of the sphere grid (the ability map), placed with the
//! map module by `tools/progress.py`.

use ffx_bridge::Global;

/// The sphere grid data (`LoadAbmap`): nodes of `0x28` bytes from
/// `+0x808`, links of `0x14` bytes, node kinds of `0x30` bytes.
pub const ABMAP: Global<u32> = Global::new(0x0230_5834);

/// Offsets in the sphere grid data.
pub mod layout {
    /// `+0x4`: the link count (`i16`).
    pub const LINK_COUNT: u32 = 0x4;
    /// `+0x808`: the nodes.
    pub const NODES: u32 = 0x808;
    /// Bytes per node: `+0` x and `+2` y (`i16`), `+6` the kind (`u16`),
    /// `+0x21` the panel bits (`abmap_lost_panel`).
    pub const NODE_SIZE: u32 = 0x28;
    /// `+0xa816`: the first link's flags byte.
    pub const LINK_FLAGS: u32 = 0xa816;
    /// Bytes per link.
    pub const LINK_SIZE: u32 = 0x14;
    /// `+0xf834`: the node kinds' sizes (`i16`, twice the radius), `0x30`
    /// bytes per kind.
    pub const KIND_SIZE: u32 = 0xf834;
    /// Bytes per node kind.
    pub const KIND_STRIDE: u32 = 0x30;
}

/// The sphere grid where the game keeps it.
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{ABMAP, layout};

    /// `AbmapAllLinksLimitFlag(mask)` (`0x00a473d0`): every link's flags
    /// byte masked with `mask`. Returns the `eax` it ends with: the data
    /// pointer with no links, else past the last link's flags.
    pub fn abmap_all_links_limit_flag(mem: &mut impl Mem, mask: u8) -> u32 {
        let g = ABMAP.get(mem);
        let n = mem.read::<i16>(g.wrapping_add(layout::LINK_COUNT));
        if n == 0 {
            return g;
        }
        #[allow(clippy::cast_sign_loss)] // the original counts down from it
        let n = i32::from(n) as u32;
        let mut at = g.wrapping_add(layout::LINK_FLAGS);
        for _ in 0..n {
            let b = mem.read::<u8>(at);
            mem.write(at, b & mask);
            at = at.wrapping_add(layout::LINK_SIZE);
        }
        at
    }

    /// `abmap_lost_panel(bit, node)` (`0x00a458c0`): clears panel bit
    /// `bit` (an 8-bit shift: none from 8 on) of the node. Returns the data
    /// pointer's upper bytes with the mask in `al`.
    pub fn abmap_lost_panel(mem: &mut impl Mem, bit: u32, node: u32) -> u32 {
        let g = ABMAP.get(mem);
        let n = bit & 31;
        let mask = !(if n < 8 { 1u8 << n } else { 0 });
        let at = g
            .wrapping_add(node.wrapping_mul(layout::NODE_SIZE))
            .wrapping_add(0x829);
        let b = mem.read::<u8>(at);
        mem.write(at, b & mask);
        (g & !0xff) | u32::from(mask)
    }

    /// `AbmapGetNodePosRadius(node, out)` (`0x00a5b4c0`): the node's x, y
    /// (`i16`s as floats), 0.0, 1.0; returns half its kind's size (`int`
    /// division) as a float. Node `0xffff` gives the origin and 0.0.
    pub fn abmap_get_node_pos_radius(mem: &mut impl Mem, node: u16, out: u32) -> f32 {
        if node == 0xffff {
            mem.write(out, [0f32, 0.0, 0.0, 1.0]);
            return 0.0;
        }
        let g = ABMAP.get(mem);
        let n = g
            .wrapping_add(layout::NODES)
            .wrapping_add(u32::from(node) * layout::NODE_SIZE);
        let x = mem.read::<i16>(n);
        let y = mem.read::<i16>(n.wrapping_add(2));
        mem.write(out, [f32::from(x), f32::from(y), 0.0, 1.0]);
        let kind = u32::from(mem.read::<u16>(n.wrapping_add(6)));
        let k = mem.read::<i16>(
            g.wrapping_add(layout::KIND_SIZE)
                .wrapping_add(kind * layout::KIND_STRIDE),
        );
        // cdq; sub; sar 1: toward zero
        f32::from(k / 2)
    }
}
