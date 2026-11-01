//! `magic_0269.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// A sprite block's set-up (0x100016c0, no arguments, no frame): the
/// size word (`+0x07`) 0x80, slot 562 `()`'s answer into `+0x18`, the
/// kind halfword `+0x21` 0x8044 and byte `+0x29` 0, the colour `+0x3f`
/// 0x80808080, the flags `+0x62` 0x90000 and every other word 0. Returns
/// 0.
pub fn sprite_block<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let s = slot_target(g, w(0x01), 0x8c8 / 4);
    g.write::<u32>(w(0x07), 0x80);
    let r = g.call(s, 0, &[]).eax;
    g.write::<u32>(w(0x18), r);
    g.write::<u16>(w(0x21), 0x8044);
    g.write::<u8>(w(0x29), 0);
    for at in [0x2f, 0x34, 0x39] {
        g.write::<u32>(w(at), 0);
    }
    g.write::<u32>(w(0x3f), 0x8080_8080);
    for at in [0x48, 0x4d, 0x52, 0x57, 0x5c] {
        g.write::<u32>(w(at), 0);
    }
    g.write::<u32>(w(0x62), 0x0009_0000);
    g.write::<u32>(w(0x6b), 0);
    g.write::<u32>(w(0x70), 0);
    Ret::eax(0)
}

/// [`sprite_block`] (117 bytes, 1 overlay).
pub const SPRITE_BLOCK: Sig = Sig {
    len: 117,
    hash: 0x41ed_eba1_3d7e_65e9,
    calls: &[],
    imports: &[],
};
