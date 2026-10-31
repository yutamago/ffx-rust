//! `magic_0476.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sa_pool::give_back;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A fading node's step (0x100017f0, `(node)`, a node callback): the
/// count `n` at operand `+0x07` goes down by one (stored before the
/// call); with `d = n - 120`, slot 348 `(*G_A, *G_B, (120 - n) * 768 /
/// 120, d * 96 / 120, d * 52 / 120, 0)` (`G_A` at `+0x72`, `G_B` at
/// `+0x67`; signed divisions). While the count is above 0 that answer is
/// returned; at 0 the node goes back to the pool ([`give_back`], a tail
/// jump), whose `eax` it returns.
pub fn fade_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let n = g.read::<i32>(w(0x7)).wrapping_sub(1);
    let d = n.wrapping_sub(0x78);
    let a = d.wrapping_mul(0x34) / 120;
    let b = d.wrapping_mul(0x60) / 120;
    let c = 0x78i32.wrapping_sub(n).wrapping_mul(0x300) / 120;
    let ga = g.read::<u32>(w(0x72));
    let gb = g.read::<u32>(w(0x67));
    let s348 = slot_target(g, w(0x6c), 348);
    g.write::<i32>(w(0x7e), n);
    let r = g.call(
        s348,
        0,
        &[
            ga,
            gb,
            c.cast_unsigned(),
            b.cast_unsigned(),
            a.cast_unsigned(),
            0,
        ],
    );
    if g.read::<i32>(w(0x89)) > 0 {
        return Ret::eax(r.eax);
    }
    let node = fr.arg(g, 0);
    Ret::eax(give_back(g, img, img.rel32_target(f + 0x97), node))
}

/// [`fade_step`] (157 bytes, 1 overlay).
pub const FADE_STEP: Sig = Sig {
    len: 157,
    hash: 0x9895_0f14_a590_1ebd,
    calls: &[(0x97, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};
