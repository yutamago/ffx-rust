//! `magic_0284.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A smoke object for an emitter (0x10001ee0, cdecl `(p)`): a pool
/// object ([`pool_take_140`](crate::sb_misc::pool_take_140); none: 0) is
/// started ([`start_object_at`](crate::s4_misc::start_object_at), scale
/// the float at `+0x13`, index 0, owners 0 and the immediate `+0x19`, the
/// object among the words); slot 325 `(O + 0xd0, p + 0x24c)` (the
/// start's words left under it); `O + 0x120` = `*G_ME` (`+0x56`) for
/// actor `p[0x280]` -1, else the byte `(*G_L)[actor]` (`+0x6b`); then a
/// tail jump into [`smoke_puff`](crate::s6_misc::smoke_puff) with `ecx`
/// = `p`. Returns its result.
pub fn smoke_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let o = crate::sb_misc::pool_take_140(g, img, img.rel32_target(f + 0x5));
    if o == 0 {
        return Ret::eax(0);
    }
    let scale = g.read::<f32>(w(0x13));
    crate::s4_misc::start_object_at(g, img, img.rel32_target(f + 0x25), o, scale, 0, 0, w(0x19));
    let p = fr.arg(g, 0);
    let t = slot_target(g, w(0x3b), 325);
    // the start's words: `ecx` was the pool take's record (the object)
    g.call(t, 0, &[o + 0xd0, p + 0x24c, 0, 0, o, w(0x19)]);
    let i = g.read::<u32>(p + 0x280);
    let (v, jmp) = if i == u32::MAX {
        (g.read::<u32>(w(0x56)), 0x66)
    } else {
        let l = g.read::<u32>(w(0x6b));
        (u32::from(g.read::<u8>(l.wrapping_add(i))), 0x7f)
    };
    g.write::<u32>(o + 0x120, v);
    let callee = Frame {
        at: img.rel32_target(f + jmp),
        ecx: p,
        eax: v,
        ..*fr
    };
    Ret::eax(crate::s6_misc::smoke_puff(g, img, &callee))
}

/// [`smoke_start`] (134 bytes, 1 overlay).
pub const SMOKE_START: Sig = Sig {
    len: 134,
    hash: 0xf54d_2195_b828_5175,
    calls: &[
        (0x5, Callee::Sig(&crate::sb_misc::POOL_TAKE_140)),
        (0x25, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x66, Callee::Sig(&crate::s6_misc::SMOKE_PUFF)),
        (0x7f, Callee::Sig(&crate::s6_misc::SMOKE_PUFF)),
    ],
    imports: &[],
};
