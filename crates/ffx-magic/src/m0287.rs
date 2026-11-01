//! `magic_0287.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The main smoke object (0x10004e00, cdecl `(p)`): the object at the
/// immediate `+0x18` starts ([`start_object_owners`](crate::s4_misc::start_object_owners),
/// scale the float at `+0x07`, index 1, owners 0, 0 and the immediate
/// `+0x0d`); slot 325 `(P, p + 0x24c)` (`P` the immediate `+0x31`; the
/// start's words left under it); then a tail jump into
/// [`smoke_puff`](crate::s6_misc::smoke_puff) with `ecx` = `p`. Returns
/// its result.
pub fn smoke_main<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let scale = g.read::<f32>(w(0x7));
    crate::s4_misc::start_object_owners(
        g,
        img,
        img.rel32_target(f + 0x1d),
        w(0x18),
        scale,
        1,
        0,
        (0, w(0xd)),
    );
    let p = fr.arg(g, 0);
    let t = slot_target(g, w(0x2c), 325);
    let eax = g.call(t, 0, &[w(0x31), p + 0x24c, 1, 0, 0, w(0xd)]).eax;
    let callee = Frame {
        at: img.rel32_target(f + 0x45),
        ecx: p,
        eax,
        ..*fr
    };
    Ret::eax(crate::s6_misc::smoke_puff(g, img, &callee))
}

/// [`smoke_main`] (73 bytes, 1 overlay).
pub const SMOKE_MAIN: Sig = Sig {
    len: 73,
    hash: 0xe815_7121_41a0_44ba,
    calls: &[
        (0x1d, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x45, Callee::Sig(&crate::s6_misc::SMOKE_PUFF)),
    ],
    imports: &[],
};
