//! `magic_0350.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// [`lights_models`](crate::m0348::lights_models), then the parts
/// registered ([`register_parts`](crate::sa_misc::register_parts) with
/// its `eax`; 0x10005b70, internal, no arguments). Returns their result.
pub fn lights_models_parts<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let eax = crate::m0348::lights_models(g, img, fr);
    Ret::eax(crate::sa_misc::register_parts(
        g,
        img,
        img.rel32_target(fr.at + 0x5e),
        eax,
    ))
}

/// [`lights_models_parts`] (104 bytes, 1 overlay).
pub const LIGHTS_MODELS_PARTS: Sig = Sig {
    len: 104,
    hash: 0xffd0_721b_3c38_d77e,
    calls: &[
        (0x13, Callee::Sig(&crate::s6_misc::LIGHTS_AND_GIVE_BACK)),
        (0x5e, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};
