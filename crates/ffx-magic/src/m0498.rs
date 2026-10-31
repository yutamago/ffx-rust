//! `magic_0498.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::part_draw::start_up_head;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's start-up (0x10001200, an overlay table entry, `(arg)`):
/// the common head ([`start_up_head`]), then the timeline's defaults
/// (its callback `+0x13a`/`+0x13e`, 2, 2, 1, 1). Returns 1.
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    start_up_head(g, img, fr);
    // add esp, 0x18
    g.write::<u32>(w(0x13a), w(0x13e));
    for (op, v) in [(0x144u32, 2u32), (0x14e, 2), (0x158, 1), (0x162, 1)] {
        g.write::<u32>(w(op), v);
    }
    Ret::eax(1)
}

/// 0x10001200 (370 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 370,
    hash: 0x30b4_f72a_afd1_f504,
    calls: &[(0x131, Callee::Import("memset"))],
    imports: &[],
};
