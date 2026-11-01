//! `magic_0372.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A part start (0x10001000, cdecl `(obj)`; `m0351`'s kin):
/// [`part_head`](crate::m0351::part_head) with the list's word 1, slot
/// 410 `(*X, obj + 0x40)` (`X` `+0x45`), the height `+0x44` down by `K`
/// (`+0x5a`), [`part_motion`](crate::m0351::part_motion) with the scale
/// 0.075. Returns slot 410's `eax`.
pub fn part_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let (obj, mut p) = crate::m0351::part_head(g, img, fr, 1, 0x36);
    p.push_all(&[obj + 0x40, g.read::<u32>(w(0x45))]);
    let s = slot_target(g, w(0x3f), 0x668 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let eax = p.call(g, s, 0).eax;
    let h = crate::sb_sse::sub(g.read::<f32>(obj + 0x44), g.read::<f32>(w(0x5a)));
    g.write::<f32>(obj + 0x44, h);
    crate::m0351::part_motion(g, obj, 0x3d99_999a);
    Ret::eax(eax)
}

/// [`part_start`] (161 bytes, 1 overlay).
pub const PART_START: Sig = Sig {
    len: 161,
    hash: 0x7b25_baf0_02bd_1cdb,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
