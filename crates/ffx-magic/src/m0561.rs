//! `magic_0561.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::part_draw::{DrawOps, draw_restart};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object's frame (0x100010a0, `(obj)`): the object's four colour
/// halfwords `+0x70..+0x76` from the effect's colour (four floats at
/// operand `+0x07` and on, truncated); then it is drawn and restarted
/// with program 0 when slot 712 says so ([`draw_restart`]). Returns 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    for (i, op) in (0u32..).zip([0x7, 0x19, 0x25, 0x31]) {
        let v = crate::s4_summon::trunc(g.read::<f32>(w(op)));
        #[allow(clippy::cast_possible_truncation)]
        g.write::<u16>(obj + 0x70 + 2 * i, v as u16);
    }
    let ops = DrawOps {
        t258: 0x3a,
        a231: [0x48, 0x53, 0x5f],
        t231: 0x4d,
        a230: [0x67, 0x72],
        t230: 0x6c,
        lock: 0x80,
        gp: 0x85,
        item: 0x91,
        t712: 0x96,
        lock2: 0xaf,
        gp2: 0xbd,
        item2: 0xc4,
        t713: 0xc9,
        start: [0xd6, 0xdc, 0xe1, 0xe9, 0x103],
        index: 0,
        lock3: 0x108,
    };
    draw_restart(g, img, f, obj, &ops);
    Ret::eax(1)
}

/// 0x100010a0 (301 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 301,
    hash: 0xcc81_5fa2_56a4_d184,
    calls: &[(0x103, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
