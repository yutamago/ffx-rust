//! `magic_0531.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::part_draw::start_part;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object's start (0x10001140, `(obj)`): program 0 started
/// ([`start_part`]), slot 268 `(*G_H, 16, obj + 0x40)` places it and
/// slot 268 `(*G_H, 16, P)` fills the light's point `P` (operand
/// `+0x58`; the earlier words left on the stack); the scales 0.23
/// (`+0x6c` 1.0), `+0x50..` 0, `+0x5c` 1; when the flag at `+0xa9` is
/// set, slot 393 `(*G_L, P, 0x100)`. Returns 1.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let below = start_part(g, img, f, obj, [0x4, 0xb, 0x10, 0x18, 0x35], 0, &[]);
    let t = slot_target(g, w(0x3e), 268);
    let mut stack = vec![g.read::<u32>(w(0x46)), 0x10, obj + 0x40];
    stack.extend(below);
    // call width: 7
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x53), 268);
    let h = g.read::<u32>(w(0x60));
    for (off, v) in [
        (0x60u32, 0x3e6b_851fu32),
        (0x64, 0x3e6b_851f),
        (0x68, 0x3e6b_851f),
        (0x6c, 0x3f80_0000),
        (0x50, 0),
        (0x54, 0),
        (0x58, 0),
        (0x5c, 1),
    ] {
        g.write::<u32>(obj + off, v);
    }
    stack.splice(0..0, [h, 0x10, w(0x58)]);
    // call width: 10
    g.call(t, 0, &stack);
    if g.read::<u32>(w(0xa9)) != 0 {
        let t = slot_target(g, w(0xb2), 393);
        let l = g.read::<u32>(w(0xc8));
        g.call(t, 0, &[l, w(0xc2), 0x100]);
    }
    Ret::eax(1)
}

/// 0x10001140 (216 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 216,
    hash: 0x315c_d72c_9bbb_81f4,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
