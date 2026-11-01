//! `magic_0474.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::o4_more::push_head;
use crate::sa_pool::take;
use crate::{Callee, Image, Mem, Sig};

/// Starts a timed object (0x10001f00, internal: `ecx` = the object):
/// `+0x90` and `+0x8c` 40, `+0x94` 0; a pool node ([`take`], owner the
/// callback at `+0x04`) holds the object (`+0x14`, `+0x4` 0) at the head
/// of the list (`G` at `+0x2d` 0: empty). Returns the node.
pub fn timed_start(g: &mut impl Mem, img: &Image, f: u32, obj: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(obj + 0x90, 0x28);
    g.write::<u32>(obj + 0x8c, 0x28);
    g.write::<u32>(obj + 0x94, 0);
    let node = take(g, img, img.rel32_target(f + 0x27), w(0x04));
    let empty = g.read::<u32>(w(0x2d)) == 0;
    g.write::<u32>(node.wrapping_add(0x14), obj);
    g.write::<u32>(node.wrapping_add(4), 0);
    push_head(g, node, empty, (w(0x41), w(0x4b)), (w(0x5a), w(0x5f)));
    node
}

/// [`timed_start`] (103 bytes, 1 overlay).
pub const TIMED_START: Sig = Sig {
    len: 103,
    hash: 0xafa7_f563_a7c6_806c,
    calls: &[(0x27, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};
