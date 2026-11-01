//! `magic_0381.dll`'s own functions, and `magic_0383`'s start-up
//! (besides the runtime, the object host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The two actors' poses (0x10001430, no arguments, no frame): with the
/// battle flag `*B` (`+0x02`, again `+0x52`) clear, a party member (below
/// 0x20) gets slot 541 `(who, 0.0, 0x2d)`, anyone else slot 38 `(obj,
/// 0.0, 0x2d)`; first for the caster `*C` (`+0x0a`, the object `*O`
/// `+0x42`), then for the target byte `**T` (`+0x49`, its object from the
/// table `+0x89`). Then slot 525 `(*O)` (`+0xaa`). Returns slot 525's
/// `eax`.
pub fn poses<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let pose = |g: &mut G, who: u32, obj: &dyn Fn(&G) -> u32, (b, t541, t38): (u32, u32, u32)| {
        if g.read::<u32>(w(b)) == 0 && who.cast_signed() < 0x20 {
            let s = slot_target(g, w(t541), 0x874 / 4);
            g.call(s, 0, &[who, 0, 0x2d]);
        } else {
            let s = slot_target(g, w(t38), 0x98 / 4);
            let o = obj(g);
            g.call(s, 0, &[o, 0, 0x2d]);
        }
    };
    let caster = g.read::<u32>(w(0x0a));
    pose(
        g,
        caster,
        &|g: &G| g.read::<u32>(w(0x42)),
        (0x02, 0x1f, 0x2c),
    );
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x49))));
    pose(
        g,
        b,
        &|g: &G| g.read::<u32>(w(0x89).wrapping_add(4 * b)),
        (0x52, 0x62, 0x8e),
    );
    let s = slot_target(g, w(0x9b), 0x834 / 4);
    let o = g.read::<u32>(w(0xaa));
    Ret::eax(g.call(s, 0, &[o]).eax)
}

/// [`poses`] (178 bytes, 1 overlay).
pub const POSES: Sig = Sig {
    len: 178,
    hash: 0x36e6_ca1b_96a7_a1ca,
    calls: &[],
    imports: &[],
};

/// The start-up (`magic_0383` 0x100013a0, no arguments, no frame): two
/// words their immediates (`+0x16`, `+0x20`), slot 719 `(F, 8, 0)` (`F`
/// `+0x10`) into `+0x2e`, the part's set-up
/// ([`part_init`](crate::l5_objhost::part_init)), `+0x3c` its immediate,
/// `+0x46` 0, a u16 0 and a byte 1; a node off the free list
/// ([`pop_node`](crate::m0032::pop_node); byte `+0x18` 2) goes last on
/// the task list (first `+0x65`, last `+0x79`). Returns 0.
pub fn start_up_0383<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = slot_target(g, w(0x01), 0xb3c / 4);
    g.write::<u32>(w(0x16), w(0x1a));
    g.write::<u32>(w(0x20), w(0x24));
    let h = g.call(s, 0, &[w(0x10), 8, 0]).eax;
    g.write::<u32>(w(0x2e), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x33));
    g.write::<u32>(w(0x3c), w(0x40));
    g.write::<u32>(w(0x46), 0);
    g.write::<u16>(w(0x51), 0);
    g.write::<u8>(w(0x59), 1);
    let node = crate::m0032::pop_node(g, img, img.rel32_target(f + 0x5f));
    let head = g.read::<u32>(w(0x65));
    g.write::<u8>(node + 0x18, 2);
    g.write::<u32>(node + 4, 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x94), node);
        g.write::<u32>(node + 8, node);
        g.write::<u32>(w(0x9c), node);
    } else {
        let last = g.read::<u32>(w(0x79));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
        g.write::<u32>(node + 8, node);
        g.write::<u32>(w(0x86), node);
    }
    Ret::eax(0)
}

/// [`start_up_0383`] (163 bytes, 1 overlay).
pub const START_UP_0383: Sig = Sig {
    len: 163,
    hash: 0x087b_2eb5_3a84_f284,
    calls: &[
        (0x33, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x5f, Callee::Sig(&crate::m0032::POP_NODE)),
    ],
    imports: &[],
};
