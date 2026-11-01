//! `magic_0429.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The frame callback (0x100014f0, no arguments): nothing (0) until
/// started (`*S`, `+0x02`); else `**P` (`P` `+0x0d`) = the immediate
/// `+0x19`, slot 723 `(*A)` (`A` `+0x13`) and slot 724 `(*B)` (`B`
/// `+0x31`), `**Q` (`Q` `+0x3f`) = `*V` (`+0x44`); while the timeline
/// runs ([`timeline`](crate::sa_misc::timeline), `+0x4e`) the frame count
/// `*F` (`+0x79`) goes up, else it stops
/// ([`timeline_stop_global`](crate::sb_misc::timeline_stop_global),
/// `+0x57`) and slot 214 `(*X)` (`X` `+0x62`) runs. Returns 1. Each slot
/// call passes the words the original leaves on the stack.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x02)) == 0 {
        return Ret::eax(0);
    }
    let p = g.read::<u32>(w(0x0d));
    let mut st = crate::pushes::Pushes::default();
    st.push(g.read::<u32>(w(0x13)));
    g.write::<u32>(p, w(0x19));
    let s = slot_target(g, w(0x1e), 0xb4c / 4);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    st.push(g.read::<u32>(w(0x31)));
    let s = slot_target(g, w(0x2b), 0xb50 / 4);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    let q = g.read::<u32>(w(0x3f));
    let v = g.read::<u32>(w(0x44));
    g.write::<u32>(q, v);
    if crate::sa_misc::timeline(g, img, img.rel32_target(f + 0x4e)) != 0 {
        let n = g.read::<u32>(w(0x79));
        g.write::<u32>(w(0x79), n.wrapping_add(1));
        return Ret::eax(1);
    }
    crate::sb_misc::timeline_stop_global(g, img, img.rel32_target(f + 0x57));
    let s = slot_target(g, w(0x5c), 0x358 / 4);
    let x = g.read::<u32>(w(0x62));
    g.call(s, 0, &[x]);
    Ret::eax(1)
}

/// [`frame`] (131 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 131,
    hash: 0x5fcf_944a_8045_190b,
    calls: &[
        (0x4e, Callee::Sig(&crate::sa_misc::TIMELINE)),
        (0x57, Callee::Sig(&crate::sb_misc::TIMELINE_STOP_GLOBAL)),
    ],
    imports: &[],
};
