//! `magic_0271.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A point far below (0x100020c0, cdecl `(a)`, a /GS frame): slot 237
/// `(*G_M, 0x18, &V, a + 0xd0)` with the local `V = (-2000, 0, 0, 1)`
/// (`ebp - 0x14`; `*G_M` operand `+0x28`), then `a + 0xdc` 1.0. Returns
/// slot 237's `eax`.
pub fn point_below<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let out = fr.arg(g, 0).wrapping_add(0xd0);
    let v = ebp.wrapping_sub(0x14);
    for (i, x) in (0u32..).zip([0xc4fa_0000u32, 0, 0, 0x3f80_0000]) {
        g.write::<u32>(v + 4 * i, x);
    }
    let t = slot_target(g, w(0x20), 237);
    let m = g.read::<u32>(w(0x28));
    let eax = g.call(t, 0, &[m, 0x18, v, out]).eax;
    g.write::<u32>(out + 0xc, 0x3f80_0000);
    Ret::eax(eax)
}

/// [`point_below`] (105 bytes, 1 overlay; the `rel32` at `+0x61` is
/// `__security_check_cookie`).
pub const POINT_BELOW: Sig = Sig {
    len: 105,
    hash: 0x7419_2a35_99f1_4cd0,
    calls: &[(0x61, Callee::Any)],
    imports: &[],
};
