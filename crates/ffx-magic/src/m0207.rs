//! `magic_0207.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A node copied from a model node (0x10006a80, internal: `ecx` = the
/// node, the stack word `src`): the node's vtable the immediate `+0x0e`;
/// [`node_copy`](crate::s4_small::node_copy) of the local `[node, P,
/// src]` (`ebp - 0xc`, `P` the immediate `+0x18`). Returns its `eax`.
pub fn copy_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let node = fr.ecx;
    let src = fr.arg(g, 0);
    let rec = fr.esp.wrapping_sub(0x10);
    g.write::<u32>(node, w(0xe));
    for (i, v) in (0u32..).zip([node, w(0x18), src]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    let callee = Frame {
        at: img.rel32_target(f + 0x20),
        esp: rec.wrapping_sub(4),
        ecx: rec,
        ..*fr
    };
    crate::s4_small::node_copy(g, img, &callee)
}

/// [`copy_node`] (40 bytes, 1 overlay).
pub const COPY_NODE: Sig = Sig {
    len: 40,
    hash: 0x12d9_7f14_383f_5b12,
    calls: &[(0x20, Callee::Sig(&crate::s4_small::NODE_COPY))],
    imports: &[],
};
