//! `magic_0564.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Sig};

/// Hides or fades an actor (0x10001b40, internal: `ecx` = an index into
/// the actor bytes `b = *G_B`, -1 for the effect's own actor `*G_A`):
/// unless the flag at operand `+0x02` is set, an actor number below 32
/// (`b[i]`, or `*G_A`) goes to slot 296 `(n, 0)`; otherwise slot 257
/// `(model, 1)` with the effect's model (`+0x3c`) or the actor's
/// (`T[b[i]]`, `T` at `+0x5d`). Returns the slot's `eax`.
pub fn hide<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let i = fr.ecx;
    let b = g.read::<u32>(w(0x9));
    if g.read::<u32>(w(0x2)) == 0 {
        let n = if i == u32::MAX {
            g.read::<u32>(w(0x10))
        } else {
            u32::from(g.read::<u8>(b.wrapping_add(i)))
        };
        if n.cast_signed() < 0x20 {
            let t = slot_target(g, w(0x26), 296);
            return g.call(t, 0, &[n, 0]).eax;
        }
    }
    if i == u32::MAX {
        let t = slot_target(g, w(0x44), 257);
        let m = g.read::<u32>(w(0x3c));
        return g.call(t, 0, &[m, 1]).eax;
    }
    let k = u32::from(g.read::<u8>(b.wrapping_add(i)));
    let m = g.read::<u32>(w(0x5d).wrapping_add(4 * k));
    let t = slot_target(g, w(0x63), 257);
    g.call(t, 0, &[m, 1]).eax
}

/// [`hide`] (115 bytes, 1 overlay).
pub const HIDE: Sig = Sig {
    len: 115,
    hash: 0x5c9d_6a63_8604_5dce,
    calls: &[],
    imports: &[],
};
