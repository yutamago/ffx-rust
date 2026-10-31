//! `magic_0665.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::{Callee, Frame, Game, Image, Sig};

/// Resets the timeline block (0x10002bd0, internal, no arguments): the
/// 0x110 bytes at operand `+0x08` zeroed (`memset`), its callback word
/// `+0x16` the immediate `+0x1a`, the counts `+0x20`/`+0x2a` 9 and the
/// nine flags (`+0x34` ... `+0x84`) 1. Returns the block (`memset`'s
/// `eax`).
pub fn timeline_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let block = w(0x8);
    g.write_bytes(block, &[0u8; 0x110]);
    g.write::<u32>(w(0x16), w(0x1a));
    g.write::<u32>(w(0x20), 9);
    g.write::<u32>(w(0x2a), 9);
    for op in [0x34, 0x3e, 0x48, 0x52, 0x5c, 0x66, 0x70, 0x7a, 0x84] {
        g.write::<u32>(w(op), 1);
    }
    block
}

/// [`timeline_reset`] (141 bytes, 1 overlay).
pub const TIMELINE_RESET: Sig = Sig {
    len: 141,
    hash: 0xf969_ca9a_59d9_ac1b,
    calls: &[(0xd, Callee::Import("memset"))],
    imports: &[],
};
