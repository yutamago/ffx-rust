//! `magic_0431.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// Whether both sounds are ready (0x10001620, no arguments): for the ids
/// 0x3019 and 0x301f in turn slot 5 `(id)` must answer 0 and slot 28
/// `(id, 1)` must answer 0. Returns 1 when all four do, else 0.
pub fn sounds_ready<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    for (t5, t28, id) in [(0x01, 0x17, 0x3019), (0x35, 0x4b, 0x301f)] {
        let s = slot_target(g, w(t5), 0x14 / 4);
        if g.call(s, 0, &[id]).eax != 0 {
            return Ret::eax(0);
        }
        let s = slot_target(g, w(t28), 0x70 / 4);
        if g.call(s, 0, &[id, 1]).eax != 0 {
            return Ret::eax(0);
        }
    }
    Ret::eax(1)
}

/// [`sounds_ready`] (108 bytes, 1 overlay).
pub const SOUNDS_READY: Sig = Sig {
    len: 108,
    hash: 0x07bf_afc4_11c6_1002,
    calls: &[],
    imports: &[],
};

/// The actor's colour reset (0x10001a10, no arguments): the actor `a` =
/// `T[*B]` (`B` = `*P`, `+0x01`; `T` `+0x11`): slot 357 `(a, -1,
/// 0x3c00)`, slot 356 `(a, -1, 0x2ee0)`, slot 362 `(*X, a, 1, 0)` (`X`
/// `+0x4c`), slot 357 `(a, -1, -1)` and slot 356 `(a, -1, -1)`. Returns
/// the last slot's `eax`. Each slot call passes the words the original
/// leaves on the stack.
pub fn colour_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    Ret::eax(colours(g, img, fr.at))
}

/// [`colour_reset`]'s body at `f` (also `m0434`'s colour show, same
/// operands). Returns the last slot's `eax`.
pub(crate) fn colours<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x01))));
    let a = g.read::<u32>(w(0x11).wrapping_add(b * 4));
    let mut p = crate::pushes::Pushes::default();
    p.push_all(&[0x3c00, u32::MAX, a]);
    let s = slot_target(g, w(0x16), 0x594 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[0x2ee0, u32::MAX, a]);
    let s = slot_target(g, w(0x26), 0x590 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[0, 1, a, g.read::<u32>(w(0x4c))]);
    let s = slot_target(g, w(0x3b), 0x5a8 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[u32::MAX, u32::MAX, a]);
    let s = slot_target(g, w(0x53), 0x594 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[u32::MAX, u32::MAX, a]);
    let s = slot_target(g, w(0x65), 0x590 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0).eax
}

/// [`colour_reset`] (123 bytes, 1 overlay).
pub const COLOUR_RESET: Sig = Sig {
    len: 123,
    hash: 0xc807_db32_941a_c1f4,
    calls: &[],
    imports: &[],
};
