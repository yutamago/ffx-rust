//! `magic_0168.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The camera's frame (0x10003890, no arguments, an aligned /GS frame):
/// the countdown `P + 0x3158` (`P` = `*` operand `+0x1d`) drops while
/// positive; slot 562, slot 723 `(a)` and 724 `(b, a)` fill two local
/// matrices (`a` at `esp + 0x44`, `b` at `esp + 4` of the aligned frame),
/// slot 231 `(c, b, P + 0x9c0)` combines them into `c` (`esp + 0x84`).
/// Returns slot 231's `eax`.
///
/// Each slot call passes the words the original leaves on the stack.
pub fn camera_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xcc);
    let (a, b, c) = (e + 0x44, e + 4, e + 0x84);
    let p = g.read::<u32>(w(0x1d));
    let n = g.read::<i32>(p + 0x3158);
    if n > 0 {
        g.write::<i32>(p + 0x3158, n - 1);
    }
    let s = slot_target(g, w(0x33), 562);
    g.call(s, 0, &[]);
    let s = slot_target(g, w(0x45), 723);
    g.call(s, 0, &[a]);
    let s = slot_target(g, w(0x57), 724);
    g.call(s, 0, &[b, a]);
    let s = slot_target(g, w(0x78), 231);
    Ret::eax(g.call(s, 0, &[c, b, p + 0x9c0, b, a]).eax)
}

/// [`camera_frame`] (154 bytes, 1 overlay; the `rel32` at `+0x92` is
/// `__security_check_cookie`).
pub const CAMERA_FRAME: Sig = Sig {
    len: 154,
    hash: 0xf299_e16d_f015_cdb0,
    calls: &[(0x92, Callee::Any)],
    imports: &[],
};
