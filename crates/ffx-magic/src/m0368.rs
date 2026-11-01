//! `magic_0368.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The part's draw (0x10001070, no arguments, no frame): slot 258
/// `(OBJ)` (`+0x0d`), slot 231 `(*D, *E, *C)` twice (`+0x2c`, `+0x20`,
/// `+0x15`; then `+0x4b`, `+0x3f`, `+0x34`), the pointer `*P` (`+0x52`)
/// its block, and slot 712 `(OBJ, 0x30)` with the game flag `*L`
/// (`+0x5f`/`+0x7f`) 1 around it. Returns 1 when slot 712 answers 0.
/// Each slot call passes the words the original leaves on the stack.
pub fn draw<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut p = Pushes::default();
    p.push(w(0x0d));
    let s = slot_target(g, w(0x01), 0x408 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    for (c, table, e, d) in [(0x15, 0x1a, 0x20, 0x2c), (0x34, 0x39, 0x3f, 0x4b)] {
        p.push_all(&[
            g.read::<u32>(w(c)),
            g.read::<u32>(w(e)),
            g.read::<u32>(w(d)),
        ]);
        let s = slot_target(g, w(table), 0x39c / 4);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, s, 0);
    }
    let ptr = g.read::<u32>(w(0x52));
    p.push(0x30);
    g.write::<u32>(ptr, w(0x5a));
    let lock = g.read::<u32>(w(0x5f));
    p.push(w(0x64));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s = slot_target(g, w(0x71), 0xb20 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let r = p.call(g, s, 0).eax;
    let lock = g.read::<u32>(w(0x7f));
    g.write::<u32>(lock, saved);
    Ret::eax(u32::from(r == 0))
}

/// [`draw`] (147 bytes, 1 overlay).
pub const DRAW: Sig = Sig {
    len: 147,
    hash: 0xc2b7_fa9c_b060_39a0,
    calls: &[],
    imports: &[],
};
