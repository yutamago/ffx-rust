//! `magic_0480.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sa_pool::give_back;
use crate::{Callee, Game, Image, Sig};

/// Clears the effect's lights and frees its node (0x10002760, internal,
/// no arguments): for each 16-byte entry `e` of the table at operand
/// `+0x05` (up to `+0x3a`), slot 590 `(i, e - 0x30)` and slot 589 `(i,
/// e)`; then slot 589 `(n, T + 16 n)` for the index past the table. The
/// node `*G` (`+0x5b`), when there is one, goes back to the pool as
/// [`give_back`] does it, inlined (its children through `give_back`
/// itself, the pool's globals at the operands here), and `*G` becomes 0.
/// Returns the `eax` the original leaves (the free count in the low half
/// when a node was freed).
pub fn clear<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let mut i = 0u32;
    let mut e = w(0x05);
    loop {
        let s590 = slot_target(g, w(0x15), 590);
        g.call(s590, 0, &[i, e.wrapping_sub(0x30)]);
        let s589 = slot_target(g, w(0x23), 589);
        g.call(s589, 0, &[i, e, i, e.wrapping_sub(0x30)]);
        e = e.wrapping_add(0x10);
        i += 1;
        if e.cast_signed() >= w(0x3a).cast_signed() {
            break;
        }
    }
    let s589 = slot_target(g, w(0x4c), 589);
    let mut eax = g.call(s589, 0, &[i, (i << 4).wrapping_add(w(0x46))]).eax;
    let node = g.read::<u32>(w(0x5b));
    if node == 0 {
        return eax;
    }
    eax = g.read::<u32>(node + 8);
    if g.read::<u32>(eax + 0xc) != 0 {
        loop {
            let child = g.read::<u32>(node + 8);
            let next = g.read::<u32>(child + 0xc);
            g.write::<u32>(child + 0xc, 0);
            let c = g.read::<u32>(node + 8);
            eax = give_back(g, img, img.rel32_target(f + 0x84), c);
            g.write::<u32>(node + 8, next);
            if g.read::<u32>(next + 0xc) == 0 {
                break;
            }
        }
    }
    let prev = g.read::<u32>(node);
    if prev != 0 {
        eax = g.read::<u32>(node + 4);
        g.write::<u32>(prev + 4, eax);
    }
    let next = g.read::<u32>(node + 4);
    if next != 0 {
        eax = g.read::<u32>(node);
        g.write::<u32>(next, eax);
    }
    let mut cur = g.read::<u32>(w(0xaa));
    if cur == node {
        cur = g.read::<u32>(node + 4);
    }
    if g.read::<u32>(w(0xb7)) == node {
        eax = g.read::<u32>(node);
        g.write::<u32>(w(0xc0), eax);
    }
    if g.read::<u32>(w(0xc6)) == node {
        eax = g.read::<u32>(node + 4);
        g.write::<u32>(w(0xd0), eax);
    }
    let head = g.read::<u32>(w(0xd6));
    if head == 0 {
        g.write::<u32>(node + 4, node);
        g.write::<u32>(node, node);
    } else {
        g.write::<u32>(node + 4, head);
        eax = g.read::<u32>(head);
        g.write::<u32>(node, eax);
        g.write::<u32>(head, node);
    }
    let count = g.read::<u16>(w(0xf0)).wrapping_add(1);
    if count == 0x100 {
        cur = 0;
    }
    g.write::<u32>(w(0x105), node);
    g.write::<u16>(w(0x10b), count);
    g.write::<u32>(w(0x111), cur);
    g.write::<u32>(w(0x117), 0);
    (eax & 0xffff_0000) | u32::from(count)
}

/// [`clear`] (286 bytes, 1 overlay).
pub const CLEAR: Sig = Sig {
    len: 286,
    hash: 0x83f3_6084_63a9_14a4,
    calls: &[(0x84, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};
