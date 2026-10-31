//! `magic_0184.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The parts' release (0x10001900, no arguments): for each of four
/// objects (the immediates `+0x06`, `+0x23`, `+0x40`, `+0x5d`), `**Q` the
/// immediate after it (`Q` at `+0x01`..) and slot 713 `(obj)`, each call
/// seeing the earlier ones' words. Returns 0.
pub fn release_four<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut stack: Vec<u32> = Vec::new();
    for (q, obj, imm, table) in [
        (0x01, 0x06, 0x0c, 0x11),
        (0x1e, 0x23, 0x29, 0x2e),
        (0x3b, 0x40, 0x46, 0x4b),
        (0x58, 0x5d, 0x63, 0x68),
    ] {
        let qp = g.read::<u32>(w(q));
        stack.insert(0, w(obj));
        g.write::<u32>(qp, w(imm));
        let s = slot_target(g, w(table), 713);
        // call width: 4 (the earlier calls' words left on the stack)
        g.call(s, 0, &stack);
    }
    Ret::eax(0)
}

/// [`release_four`] (122 bytes, 1 overlay).
pub const RELEASE_FOUR: Sig = Sig {
    len: 122,
    hash: 0x00ec_4b97_734f_8a8a,
    calls: &[],
    imports: &[],
};
