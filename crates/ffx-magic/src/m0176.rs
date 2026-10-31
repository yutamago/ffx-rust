//! `magic_0176.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// An object's colour (0x10001710, cdecl `(obj)`): slot 237 `(*X, 0x34,
/// 0, obj + 0xd0)` (`X` at `+0x1f`) and the colour's alpha `+0xdc` 1.0.
/// Returns slot 237's `eax`.
pub fn part_colour<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let c = fr.arg(g, 0).wrapping_add(0xd0);
    let x = g.read::<u32>(w(0x1f));
    let s = slot_target(g, w(0x04), 237);
    let eax = g.call(s, 0, &[x, 0x34, 0, c]).eax;
    g.write::<u32>(c + 0xc, 0x3f80_0000);
    Ret::eax(eax)
}

/// [`part_colour`] (50 bytes, 1 overlay).
pub const PART_COLOUR: Sig = Sig {
    len: 50,
    hash: 0x3f23_fa2e_34b5_397b,
    calls: &[],
    imports: &[],
};
