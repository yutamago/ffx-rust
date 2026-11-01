//! `magic_0612.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::sb_sse::sub;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Starts the object `*G_FX + 0x5f0` (0x10009a20, an overlay table
/// entry, no arguments): [`start_object_anim`](crate::s4_misc::start_object_anim)
/// (scale `*K` `+0x4`, index 5, 0) at the point `+0x2b30` lowered by `K`
/// (`+0x66`), its `+0xc4` the angle `+0x3ba4`; its point is copied back
/// to `+0x2b40`. Returns the point's `w`.
pub fn object_5f0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let scale = g.read::<f32>(w(0x4));
    let fx = g.read::<u32>(w(0xc));
    let o = fx.wrapping_add(0x5f0);
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x20), o, scale, 5, 0);
    for i in 0..4 {
        let v = g.read::<u32>(fx + 0x2b30 + 4 * i);
        g.write::<u32>(o + 0xd0 + 4 * i, v);
    }
    let y = sub(g.read::<f32>(o + 0xd4), g.read::<f32>(w(0x66)));
    let a = g.read::<u32>(fx + 0x3ba4);
    g.write::<u32>(o + 0xc4, a);
    g.write::<f32>(o + 0xd4, y);
    let mut last = 0;
    for i in 0..4 {
        last = g.read::<u32>(o + 0xd0 + 4 * i);
        g.write::<u32>(fx + 0x2b40 + 4 * i, last);
    }
    Ret::eax(last)
}

/// [`object_5f0`] (174 bytes, 1 overlay).
pub const OBJECT_5F0: Sig = Sig {
    len: 174,
    hash: 0x3e26_3fbf_ad44_a0c0,
    calls: &[(0x20, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM))],
    imports: &[],
};
