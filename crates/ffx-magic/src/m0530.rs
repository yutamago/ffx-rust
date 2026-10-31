//! `magic_0530.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::part_draw::start_part;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object's start (0x10001140, `(obj)`): program 0 started
/// ([`start_part`]), slot 268 `(*G_H, 16, obj + 0x40)` places it (the
/// start's words left on the stack), the scales 0.23 (`+0x6c` 1.0),
/// `+0x50..` 0, `+0x5c` 1; when the flag at operand `+0x58` is set, slot
/// 393 `(*G_L, obj + 0x40, 0x100)`. Returns 1.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let below = start_part(g, img, f, obj, [0x4, 0xb, 0x10, 0x19, 0x36], 0, &[]);
    let t = slot_target(g, w(0x3b), 268);
    let mut args = vec![g.read::<u32>(w(0x4d)), 0x10, obj + 0x40];
    args.extend(below);
    // call width: 7
    g.call(t, 0, &args);
    let flag = g.read::<u32>(w(0x58));
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
    if flag != 0 {
        let t = slot_target(g, w(0x98), 393);
        let l = g.read::<u32>(w(0xaa));
        g.call(t, 0, &[l, obj + 0x40, 0x100]);
    }
    Ret::eax(1)
}

/// 0x10001140 (188 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 188,
    hash: 0xdae0_c328_f07a_b3e3,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
