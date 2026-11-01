//! `magic_0689.dll`'s own functions (besides the runtime and the shared
//! code; its cue start is [`crate::m0638::cue_start`]).

use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Starts the DLL's object (0x10003230, no arguments): the object `*G_O`
/// (operand `+0xb`) started with the scale at operand `+0x4` and the
/// index at `+0x16`
/// ([`start_object_plain`](crate::s4_misc::start_object_plain), the
/// `rel32` at `+0x1b`); then its colour `+0xd0..+0xdc` and `+0xc4` from
/// the constants at `+0x23`, `+0x33`, `+0x43`, `+0x53`, `+0x63`, its
/// callback `+0x104` the immediate at `+0x78` and `+0x120` 0xa4. Returns
/// the start's `eax`.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let scale = f32::from_bits(g.read::<u32>(w(0x4)));
    let obj = g.read::<u32>(w(0xb));
    let index = g.read::<u32>(w(0x16));
    let r =
        crate::s4_misc::start_object_plain(g, img, img.rel32_target(f + 0x1b), obj, scale, index);
    for (dst, src) in [
        (0xd0u32, 0x23u32),
        (0xd4, 0x33),
        (0xd8, 0x43),
        (0xdc, 0x53),
        (0xc4, 0x63),
    ] {
        let v = g.read::<u32>(w(src));
        g.write::<u32>(obj.wrapping_add(dst), v);
    }
    g.write::<u32>(obj.wrapping_add(0x104), w(0x78));
    g.write::<u32>(obj.wrapping_add(0x120), 0xa4);
    Ret::eax(r)
}

/// [`start`] (136 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 136,
    hash: 0xa13a_be6d_a0f1_e252,
    calls: &[(0x1b, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN))],
    imports: &[],
};
