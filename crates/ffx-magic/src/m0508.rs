//! `magic_0508.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object's callback (0x10001b40, `(obj)`, pushed by 0x100017bd; a
/// cookie frame): with the actor `a = T[b[0]]` (`b = *G_B`, operands
/// `+0x11` and `+0x1d`), slot 262 `(a, &v.x, &v.y, &v.z)` and slot 268
/// `(a, 0x10, &v)` fill the local point `v` (`ebp - 0x14`), whose `y`
/// goes to the object's `+0xd4`; `+0x104` 0. Returns slot 268's `eax`.
pub fn e1b40<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let v = fr.esp.wrapping_sub(4).wrapping_sub(0x14);
    let b = g.read::<u32>(w(0x11));
    let i = u32::from(g.read::<u8>(b));
    let a = g.read::<u32>(w(0x1d).wrapping_add(4 * i));
    let obj = fr.arg(g, 0);
    let t = slot_target(g, w(0x31), 262);
    let mut stack = vec![a, v, v + 4, v + 8];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x43), 268);
    stack.splice(0..0, [a, 0x10, v]);
    let r = g.call(t, 0, &stack).eax;
    let y = g.read::<u32>(v + 4);
    g.write::<u32>(obj + 0xd4, y);
    g.write::<u32>(obj + 0x104, 0);
    Ret::eax(r)
}

/// 0x10001b40 (124 bytes, 1 overlay).
pub const E1B40: Sig = Sig {
    len: 124,
    hash: 0x061c_dd0e_27bf_e7ce,
    calls: &[
        // __security_check_cookie
        (0x74, Callee::Any),
    ],
    imports: &[],
};
