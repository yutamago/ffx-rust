//! `magic_0513.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The effect's end (0x100089a0, an overlay table entry, no arguments):
/// the node `*G_N` (operand `+0x02`), when there is one, leaves its
/// parent's child list (the parent's count `+0x18` down; the list's own
/// first/last words `+0xc`/`+0x10` or the neighbours' links joined, the
/// neighbours read from the DLL's link globals `+0x15`/`+0x1a`), the
/// links reset to the list object, `*G_N` 0; then slot 250 `(0, 0)`.
/// Returns 0.
pub fn end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let n = g.read::<u32>(w(0x2));
    if n != 0 {
        let c = g.read::<u16>(n + 0x18);
        g.write::<u16>(n + 0x18, c.wrapping_add(0xffff));
        let next = g.read::<u32>(w(0x15));
        let prev = g.read::<u32>(w(0x1a));
        if next == n {
            g.write::<u32>(n + 0x10, prev);
        } else {
            g.write::<u32>(next + 8, prev);
        }
        let prev = g.read::<u32>(w(0x2c));
        let next = g.read::<u32>(w(0x31));
        if prev == n {
            g.write::<u32>(n + 0xc, next);
        } else {
            g.write::<u32>(prev + 4, next);
        }
        g.write::<u32>(w(0x43), w(0x47));
        g.write::<u32>(w(0x4d), w(0x51));
        g.write::<u32>(w(0x57), 0);
    }
    let t = slot_target(g, w(0x60), 250);
    g.call(t, 0, &[0, 0]);
    Ret::eax(0)
}

/// [`end`] (132 bytes, 1 overlay).
pub const END: Sig = Sig {
    len: 132,
    hash: 0x809f_0171_8f1f_eb70,
    calls: &[],
    imports: &[],
};
