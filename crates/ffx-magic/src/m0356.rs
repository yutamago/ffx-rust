//! `magic_0356.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The copies' step (0x10001160, cdecl `(obj)`): slot 434 `(*C, F)` twice
/// (`C` `+0x0f`/`+0x27`, `F` `+0x09`/`+0x21`); the counter `*N` (`+0x34`)
/// goes up and from 60 on slot 450 `(obj, F2)` (`+0x4c`) moves the object
/// on. Returns slot 450's `eax`, else the counter. Each slot call passes
/// the words the original leaves on the stack.
///
/// The census counts the callback after it (0x100011c0, already ported
/// as "motion entry 0xb4") as part of this function.
pub fn copies_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut p = Pushes::default();
    for (cb, list, table) in [(0x09, 0x0f, 0x04), (0x21, 0x27, 0x1c)] {
        p.push_all(&[w(cb), g.read::<u32>(w(list))]);
        let s = slot_target(g, w(table), 0x6c8 / 4);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, s, 0);
    }
    p.pop(4);
    let n = g.read::<u32>(w(0x34)).wrapping_add(1);
    g.write::<u32>(w(0x3d), n);
    if n.cast_signed() < 0x3c {
        return Ret::eax(n);
    }
    p.push_all(&[w(0x4c), fr.arg(g, 0)]);
    let s = slot_target(g, w(0x47), 0x708 / 4);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(p.call(g, s, 0).eax)
}

/// [`copies_step`] (96 bytes, 1 overlay).
pub const COPIES_STEP: Sig = Sig {
    len: 96,
    hash: 0x872e_0ec8_6626_2912,
    calls: &[],
    imports: &[],
};
