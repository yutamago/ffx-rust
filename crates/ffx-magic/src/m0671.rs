//! `magic_0671.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::o4_more::push_head;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's start (0x10001520, internal, no arguments): the two
/// buffer globals (operands `+0x16`/`+0x1a`, `+0x20`/`+0x24`) set, slot
/// 719 `(NAME, 8, 0)` gives the data (`+0x2e`), the part's set-up
/// ([`part_init`](crate::l5_objhost::part_init)); `+0x3c` -1, `+0x46` 0,
/// the object's callback (`+0x50`/`+0x54`), `+0x5a` 0, the halfword
/// `+0x65` 0, the byte `+0x6d` 1; then a free-list node
/// ([`pop_node`](crate::m0032::pop_node); byte `+0x18` 2, `+0x4` 0) at
/// the head of the list ([`push_head`]: the empty test `+0x79`, head
/// `+0x8d`/`+0x9a`, set `+0xa8`/`+0xb0`). Returns 0.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s719 = slot_target(g, w(0x1), 719);
    g.write::<u32>(w(0x16), w(0x1a));
    g.write::<u32>(w(0x20), w(0x24));
    let d = g.call(s719, 0, &[w(0x10), 8, 0]).eax;
    g.write::<u32>(w(0x2e), d);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x33));
    g.write::<u32>(w(0x3c), u32::MAX);
    g.write::<u32>(w(0x46), 0);
    g.write::<u32>(w(0x50), w(0x54));
    g.write::<u32>(w(0x5a), 0);
    g.write::<u16>(w(0x65), 0);
    g.write::<u8>(w(0x6d), 1);
    let node = crate::m0032::pop_node(g, img, img.rel32_target(f + 0x73));
    let empty = g.read::<u32>(w(0x79)) == 0;
    g.write::<u8>(node.wrapping_add(0x18), 2);
    g.write::<u32>(node.wrapping_add(4), 0);
    push_head(g, node, empty, (w(0x8d), w(0x9a)), (w(0xa8), w(0xb0)));
    0
}

/// [`start`] (183 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 183,
    hash: 0x0eed_e424_4f4b_9c70,
    calls: &[
        (0x33, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x73, Callee::Sig(&crate::m0032::POP_NODE)),
    ],
    imports: &[],
};

/// The effect's end (0x100014e0, cdecl `(obj)`): unless the flag at
/// operand `+0x5` is 1 nothing (the caller's `eax` returned); else the
/// objects freed ([`objects_freed`](crate::s6_misc::objects_freed)), slot
/// 311 `(*G_H)`, `G_H` -1 and the object's byte `+0x56` 1. Returns the
/// object.
pub fn end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    if g.read::<u32>(w(0x5)) != 1 {
        return Ret::eax(fr.eax);
    }
    // push ebp, the return address
    let callee = Frame {
        at: img.rel32_target(fr.at + 0xd),
        esp: fr.esp.wrapping_sub(8),
        ..*fr
    };
    crate::s6_misc::objects_freed(g, img, &callee);
    let t = slot_target(g, w(0x12), 311);
    let h = g.read::<u32>(w(0x18));
    g.call(t, 0, &[h]);
    let obj = fr.arg(g, 0);
    g.write::<u32>(w(0x2c), 0xffff_ffff);
    g.write::<u8>(obj.wrapping_add(0x56), 1);
    Ret::eax(obj)
}

/// [`end`] (58 bytes, 1 overlay).
pub const END: Sig = Sig {
    len: 58,
    hash: 0x120c_1486_6d71_37ba,
    calls: &[(0xd, Callee::Sig(&crate::s6_misc::OBJECTS_FREED))],
    imports: &[],
};
