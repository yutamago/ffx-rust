//! `magic_0686.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Places the effect's model and steps its motion script (0x10002020, an
/// overlay table entry, no arguments; a cookie frame): slot 235 fills the
/// local matrix `M` (`ebp - 0x44`), whose diagonal becomes 0.00075;
/// slot 231 `(D, S, M)` (the immediates `+0x4b`, `+0x46`), slot 211
/// `(*G_M, D)` (`+0x5d`, `+0x57`); the motion script steps
/// ([`motion_script_step`](crate::s6_misc::motion_script_step), the
/// earlier words under it). Returns whether `G_DONE` (`+0x78`) is 0.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let m = ebp.wrapping_sub(0x44);
    let t = slot_target(g, w(0x15), 235);
    g.call(t, 0, &[m]);
    for o in [0x28, 0x14, 0] {
        g.write::<u32>(m + o, 0x3a44_9ba6);
    }
    let t = slot_target(g, w(0x26), 231);
    g.call(t, 0, &[w(0x4b), w(0x46), m, m]);
    let t = slot_target(g, w(0x52), 211);
    let model = g.read::<u32>(w(0x5d));
    g.call(t, 0, &[model, w(0x57), w(0x4b), w(0x46), m, m]);
    // sub esp, 0x44 below ebp; the return address
    let step = Frame {
        at: img.rel32_target(f + 0x6d),
        esp: ebp.wrapping_sub(0x48),
        ..*fr
    };
    crate::s6_misc::motion_script_step(g, img, &step);
    Ret::eax(u32::from(g.read::<u32>(w(0x78)) == 0))
}

/// [`frame`] (138 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 138,
    hash: 0x9d55_714a_fe55_840f,
    calls: &[
        (0x6d, Callee::Sig(&crate::s6_misc::MOTION_SCRIPT_STEP)),
        // __security_check_cookie
        (0x82, Callee::Any),
    ],
    imports: &[],
};
