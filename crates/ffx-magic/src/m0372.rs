//! `magic_0372.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A part start (0x10001000, cdecl `(obj)`; `m0351`'s kin):
/// [`part_head`](crate::m0351::part_head) with the list's word 1, slot
/// 410 `(*X, obj + 0x40)` (`X` `+0x45`), the height `+0x44` down by `K`
/// (`+0x5a`), [`part_motion`](crate::m0351::part_motion) with the scale
/// 0.075. Returns slot 410's `eax`.
pub fn part_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let (obj, mut p) = crate::m0351::part_head(g, img, fr, 1, 0x36);
    p.push_all(&[obj + 0x40, g.read::<u32>(w(0x45))]);
    let s = slot_target(g, w(0x3f), 0x668 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let eax = p.call(g, s, 0).eax;
    let h = crate::sb_sse::sub(g.read::<f32>(obj + 0x44), g.read::<f32>(w(0x5a)));
    g.write::<f32>(obj + 0x44, h);
    crate::m0351::part_motion(g, obj, 0x3d99_999a);
    Ret::eax(eax)
}

/// [`part_start`] (161 bytes, 1 overlay).
pub const PART_START: Sig = Sig {
    len: 161,
    hash: 0x7b25_baf0_02bd_1cdb,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The frame callback (0x10001490, no arguments, no frame): 0 while the
/// effect is not running (`*R`, `+0x02`); else the pointer `*P`
/// (`+0x0d`) its block, slots 723 `(*M)` and 724 `(*N)` (`+0x13`,
/// `+0x31`; the words stay on the stack), `*Q = *V` (`+0x3f`, `+0x44`),
/// and slot 261 `(0)`: when it answers non-zero and the timeline
/// ([`timeline`](crate::sa_misc::timeline)) has nothing live, slot 214
/// `(*S)` (`+0x70`); else the frame counter `*C` (`+0x87`) goes up.
/// Returns 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x02)) == 0 {
        return Ret::eax(0);
    }
    let mut p = crate::pushes::Pushes::default();
    let ptr = g.read::<u32>(w(0x0d));
    p.push(g.read::<u32>(w(0x13)));
    g.write::<u32>(ptr, w(0x19));
    let s = slot_target(g, w(0x1e), 0xb4c / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push(g.read::<u32>(w(0x31)));
    let s = slot_target(g, w(0x2b), 0xb50 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let q = g.read::<u32>(w(0x3f));
    g.write::<u32>(q, g.read::<u32>(w(0x44)));
    p.push(0);
    let s = slot_target(g, w(0x4b), 0x414 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let on = p.call(g, s, 0).eax;
    if on != 0 && crate::sa_misc::timeline(g, img, img.rel32_target(f + 0x61)) == 0 {
        let s = slot_target(g, w(0x6a), 0x358 / 4);
        let x = g.read::<u32>(w(0x70));
        g.call(s, 0, &[x]);
        return Ret::eax(1);
    }
    let c = g.read::<u32>(w(0x87));
    g.write::<u32>(w(0x87), c.wrapping_add(1));
    Ret::eax(1)
}

/// [`frame`] (145 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 145,
    hash: 0xb5a9_2aaf_3561_b5f3,
    calls: &[(0x61, Callee::Sig(&crate::sa_misc::TIMELINE))],
    imports: &[],
};
