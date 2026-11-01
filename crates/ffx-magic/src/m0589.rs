//! `magic_0589.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::o4_more::push_head;
use crate::sa_pool::take;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Starts the flash (0x100028b0, an overlay table entry, no arguments;
/// the record `R = *G_R`, operand `+0x13`): a fade task on `R + 0x1bb0`
/// ([`fade_task`](crate::s3_misc::fade_task): `edx` 1, `xmm1`/`xmm2` the
/// constants at `+0x0c`/`+0x04`, the stack word 0); the part `P = R +
/// 0x1a70` gets its colour (the original's overlapping byte, half and
/// word stores in order), kind `+0x98` 0x48, sizes `+0x8c`/`+0x90` 75,
/// `+0x88`/`+0x94` 0; a pool node ([`take`], the owner the immediate at
/// `+0x33`) holding it, put at the head of the list ([`push_head`]: head
/// `+0xa4`/`+0xae`, the empty test `+0x90`, set `+0xbd`/`+0xc2`).
/// Returns the node.
pub fn flash<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let (x2, x1) = (g.read::<u32>(w(0x4)), g.read::<u32>(w(0xc)));
    let r = g.read::<u32>(w(0x13));
    // push esi; push 0; the call
    g.write::<u32>(fr.esp.wrapping_sub(8), 0);
    let callee = Frame {
        at: img.rel32_target(f + 0x25),
        esp: fr.esp.wrapping_sub(0xc),
        eax: fr.eax,
        ecx: r.wrapping_add(0x1bb0),
        edx: 1,
        xmm: [0, u128::from(x1), u128::from(x2), 0],
    };
    crate::s3_misc::fade_task(g, img, &callee);
    let p = r.wrapping_add(0x1a70);
    g.write::<u32>(p + 0x81, 0x0080_ffff);
    g.write::<u8>(p + 0x80, 0xff);
    g.write::<u16>(p + 0x86, 0);
    g.write::<u8>(p + 0x85, 0);
    g.write::<u32>(p + 0x88, 0);
    g.write::<u16>(p + 0x98, 0x48);
    g.write::<u32>(p + 0x90, 0x4b);
    g.write::<u32>(p + 0x8c, 0x4b);
    g.write::<u32>(p + 0x94, 0);
    let node = take(g, img, img.rel32_target(f + 0x8a), w(0x33));
    let empty = g.read::<u32>(w(0x90)) == 0;
    g.write::<u32>(node + 0x14, p);
    g.write::<u32>(node + 4, 0);
    push_head(g, node, empty, (w(0xa4), w(0xae)), (w(0xbd), w(0xc2)));
    Ret::eax(node)
}

/// [`flash`] (202 bytes, 1 overlay).
pub const FLASH: Sig = Sig {
    len: 202,
    hash: 0xa8cf_9104_f224_a718,
    calls: &[
        (0x25, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x8a, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// 0x10002980 (an overlay table entry):
/// [`two_parts`](crate::m0511::two_parts) on the parts `+0x1a70` and
/// `+0x1b10`, the second 30 frames.
pub fn two_parts<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::m0511::two_parts(g, img, fr.at, (0x1a70, 0x1b10, 0x1e))
}

/// [`two_parts`] (322 bytes, 1 overlay).
pub const TWO_PARTS: Sig = Sig {
    len: 322,
    hash: 0x4b31_e8f2_69d6_9608,
    calls: &[
        (0x67, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x10d, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};
