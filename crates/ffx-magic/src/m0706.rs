//! `magic_0706.dll`'s own functions (besides the runtime, the OPU host
//! and the shared code).

use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Table `+0x0` (0x10001690, `(arg)`):
/// [`opu_start`](crate::m0709::opu_start) with slot 173 `(data, TABLE)`
/// (the immediate `+0x46`) after the registration.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    crate::m0709::opu_start(
        g,
        img,
        fr,
        &[(0x41, 173, [w(0x51), w(0x46)])],
        (0x58, 0x7f, 0x95),
    )
}

/// [`start`] (153 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 153,
    hash: 0xa036_7d7f_0c47_66cc,
    calls: &[(0x95, Callee::Sig(&crate::opu::SETUP))],
    imports: &[],
};
