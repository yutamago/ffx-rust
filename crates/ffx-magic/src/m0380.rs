//! `magic_0380.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's task (0x10003810, cdecl `(arg)`): `arg` kept (`+0x07`),
/// eight words, two halfwords and one more word cleared or set (`+0x3c`
/// its immediate); a pool node ([`take`](crate::sa_pool::take), owner
/// the callback `+0x0e`) holding the block `+0x78` (`+0x14`) goes last
/// on the task list (first `+0x70`, last `+0x87`). Returns the node.
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(w(0x07), fr.arg(g, 0));
    for at in [0x14, 0x1e, 0x28, 0x32] {
        g.write::<u32>(w(at), 0);
    }
    g.write::<u32>(w(0x3c), w(0x40));
    g.write::<u32>(w(0x46), 0);
    g.write::<u32>(w(0x50), 0);
    g.write::<u16>(w(0x5a), 0);
    g.write::<u32>(w(0x5f), 0);
    g.write::<u16>(w(0x65), 0);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x6a), w(0x0e));
    let head = g.read::<u32>(w(0x70));
    g.write::<u32>(node + 0x14, w(0x78));
    g.write::<u32>(node + 4, 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0xa1), node);
        g.write::<u32>(w(0xa6), node);
    } else {
        let last = g.read::<u32>(w(0x87));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
        g.write::<u32>(w(0x91), node);
    }
    g.write::<u32>(node + 8, node);
    Ret::eax(node)
}

/// [`task`] (175 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 175,
    hash: 0x82ec_ca34_995d_35dd,
    calls: &[(0x6a, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`two_scripts_step`](crate::m0381::two_scripts_step) with bit 0 of
/// the halfword `+0x37` and bit 2 of `+0x43` cleared (0x10001340, cdecl
/// `(task)`).
pub fn scripts_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::m0381::two_scripts_step(g, img, fr, &[(0x37, 0xfffe), (0x43, 0xfffb)], 0x57)
}

/// [`scripts_step`] (95 bytes, 1 overlay).
pub const SCRIPTS_STEP: Sig = Sig {
    len: 95,
    hash: 0xf4a4_539c_b5a8_a6a3,
    calls: &[
        (0xb, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x17, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x1e, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};
