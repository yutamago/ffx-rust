//! `magic_0479.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::o4_more::push_head;
use crate::runtime::slot_target;
use crate::{Callee, Game, Image, Sig};

/// The effect's start (0x10003c60, internal, no arguments): the two
/// buffer globals (operands `+0x16`/`+0x1a`, `+0x20`/`+0x24`) set, slot
/// 719 `(NAME, 8, 0)` gives the data (`+0x2e`), the part's set-up
/// ([`part_init`](crate::l5_objhost::part_init)), the object's callback
/// (`+0x3c`/`+0x40`), `+0x46` 0, the halfword `+0x51` 0, the byte `+0x59`
/// 1; then a free-list node ([`pop_node`](crate::m0032::pop_node), `+0x4`
/// 0) at the head of the list (`G` at `+0x65` 0: empty). Returns 0.
pub fn start<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let s719 = slot_target(g, w(0x1), 719);
    g.write::<u32>(w(0x16), w(0x1a));
    g.write::<u32>(w(0x20), w(0x24));
    let d = g.call(s719, 0, &[w(0x10), 8, 0]).eax;
    g.write::<u32>(w(0x2e), d);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x33));
    g.write::<u32>(w(0x3c), w(0x40));
    g.write::<u32>(w(0x46), 0);
    g.write::<u16>(w(0x51), 0);
    g.write::<u8>(w(0x59), 1);
    let node = crate::m0032::pop_node(g, img, img.rel32_target(f + 0x5f));
    let empty = g.read::<u32>(w(0x65)) == 0;
    g.write::<u32>(node.wrapping_add(4), 0);
    push_head(g, node, empty, (w(0x75), w(0x82)), (w(0x90), w(0x98)));
    0
}

/// [`start`] (159 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 159,
    hash: 0x85b9_ad2e_c05a_3b8c,
    calls: &[
        (0x33, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x5f, Callee::Sig(&crate::m0032::POP_NODE)),
    ],
    imports: &[],
};
