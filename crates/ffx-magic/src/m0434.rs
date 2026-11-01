//! `magic_0434.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The actor's colour reset and show (0x10005d10, internal, no
/// arguments): `m0431`'s [`colours`](crate::m0431::colours) (the same
/// operands), then slot 399 `(T[*B], 1)` (`B` = `*P` read again,
/// `+0x77`; `T` `+0x86`). Returns slot 399's `eax`.
pub fn colour_show<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    crate::m0431::colours(g, img, fr.at);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x77))));
    let a = g.read::<u32>(w(0x86).wrapping_add(b * 4));
    let s = slot_target(g, w(0x8b), 0x63c / 4);
    Ret::eax(g.call(s, 0, &[a, 1]).eax)
}

/// [`colour_show`] (156 bytes, 1 overlay).
pub const COLOUR_SHOW: Sig = Sig {
    len: 156,
    hash: 0x7208_7192_9f9d_cb7b,
    calls: &[],
    imports: &[],
};
