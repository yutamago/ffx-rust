//! `magic_0378.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// An object's start (0x100011b0, cdecl `(obj)`): slot 442 `(obj,
/// (*A)[0x1a0], 0)` (`A` `+0x04`), the scale `+0x130..+0x138` 0.05
/// (`+0x13c` 0), slot 444 `(0)`, slot 440 `(*A, obj, 0, 0)`, slots 450
/// `(obj, F1)` and 449 `(obj, F2)` (`+0x77`, `+0x8a`). Returns slot
/// 449's `eax`. Each slot call passes the words the original leaves on
/// the stack.
pub fn object_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let mut p = Pushes::default();
    p.push_all(&[0, g.read::<u32>(g.read::<u32>(w(0x04)) + 0x1a0), obj]);
    let s = slot_target(g, w(0x15), 0x6e8 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    for (off, v) in [
        (0x130, 0x3d4c_cccd),
        (0x134, 0x3d4c_cccd),
        (0x138, 0x3d4c_cccd),
        (0x13c, 0),
    ] {
        g.write::<u32>(obj + off, v);
    }
    p.push(0);
    let s = slot_target(g, w(0x23), 0x6f0 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[0, 0, obj, g.read::<u32>(w(0x6b))]);
    let s = slot_target(g, w(0x5a), 0x6e0 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[w(0x77), obj]);
    let s = slot_target(g, w(0x72), 0x708 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[w(0x8a), obj]);
    let s = slot_target(g, w(0x85), 0x704 / 4);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(p.call(g, s, 0).eax)
}

/// [`object_start`] (157 bytes, 1 overlay).
pub const OBJECT_START: Sig = Sig {
    len: 157,
    hash: 0x3a87_e9ae_2106_2922,
    calls: &[],
    imports: &[],
};
