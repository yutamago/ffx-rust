//! `magic_0206.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The start-up's records (0x10001460, a table entry, no arguments):
/// slot 719 `(*G_D, 4, 1)` and `(*G_D, 4, 0)` (operands `+0x11`, `+0x2d`;
/// the first call's words left under the second's) into `+0x18` and
/// `+0x3d`; the 0x30-byte block at `+0x38` cleared (`memset`), its
/// halfword `+0x50` 0x80 and the word `+0x56` 0x80808080; the records
/// cleared ([`records_clear`](crate::s4_small::records_clear)); the words
/// `+0x68`, `+0x72`, `+0x7c` 0. Returns 1.
pub fn records_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = slot_target(g, w(0x1), 719);
    let d = g.read::<u32>(w(0x11));
    let a = g.call(t, 0, &[d, 4, 1]).eax;
    g.write::<u32>(w(0x18), a);
    let t = slot_target(g, w(0x1d), 719);
    let d2 = g.read::<u32>(w(0x2d));
    let b = g.call(t, 0, &[d2, 4, 0, d, 4, 1]).eax;
    g.write::<u32>(w(0x3d), b);
    g.write_bytes(w(0x38), &[0u8; 0x30]);
    g.write::<u16>(w(0x50), 0x80);
    g.write::<u32>(w(0x56), 0x8080_8080);
    // one of the nine words is still pushed at the call
    let clear = Frame {
        at: img.rel32_target(f + 0x5f),
        esp: fr.esp.wrapping_sub(8),
        ..*fr
    };
    crate::s4_small::records_clear(g, img, &clear);
    for o in [0x68, 0x72, 0x7c] {
        g.write::<u32>(w(o), 0);
    }
    Ret::eax(1)
}

/// [`records_start`] (138 bytes, 1 overlay).
pub const RECORDS_START: Sig = Sig {
    len: 138,
    hash: 0x5440_d5e8_0949_ba8b,
    calls: &[
        (0x42, Callee::Import("memset")),
        (0x5f, Callee::Sig(&crate::s4_small::RECORDS_CLEAR_500)),
    ],
    imports: &[],
};
