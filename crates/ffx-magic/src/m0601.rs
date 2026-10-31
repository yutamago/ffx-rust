//! `magic_0601.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// Sets up the DLL's static node `N` (operand `+0x0e`) under the parent
/// `edx` (0x10007d30, internal, cdecl `(a, b)`): its own empty child list
/// (`+0x1a` 0xff as a halfword, `+0x10`/`+0xc` itself), the parent
/// `+0x14`; with a parent it is appended to the parent's children (its
/// `+0x4` the parent, `+0x8` the old last; the old last's `+0x4` or the
/// parent's `+0xc` the node, the count `+0x18` up, `+0x10` the node).
/// Then the words at `+0x5e`/`+0x66` take `a`/`b` and the node its
/// vtable (`+0x70`). Returns `b`.
pub fn static_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let n = w(0x12);
    let parent = fr.edx;
    g.write::<u16>(w(0x6), 0xff);
    g.write::<u32>(w(0xe), n);
    g.write::<u32>(w(0x18), n);
    g.write::<u32>(w(0x22), parent);
    if parent != 0 {
        g.write::<u32>(w(0x2c), parent);
        let last = g.read::<u32>(parent + 0x10);
        g.write::<u32>(w(0x34), last);
        let last = g.read::<u32>(parent + 0x10);
        if last == parent {
            g.write::<u32>(parent + 0xc, w(0x42));
        } else {
            g.write::<u32>(last + 4, w(0x4b));
        }
        let c = g.read::<u16>(parent + 0x18);
        g.write::<u16>(parent + 0x18, c.wrapping_add(1));
        g.write::<u32>(parent + 0x10, w(0x56));
    }
    let a = fr.arg(g, 0);
    g.write::<u32>(w(0x5e), a);
    let b = fr.arg(g, 1);
    g.write::<u32>(w(0x66), b);
    g.write::<u32>(w(0x6c), w(0x70));
    b
}

/// [`static_node`] (118 bytes, 1 overlay).
pub const STATIC_NODE: Sig = Sig {
    len: 118,
    hash: 0x2350_d164_6347_a119,
    calls: &[],
    imports: &[],
};

/// The effect's end (0x1000a3b0, an overlay table entry, no arguments):
/// slots 694 `(0, 0, 0, 0)`, 384 `(G_A)`, 24 `(0x300f, 0)`, 25 `(0x300f,
/// 1)`, 211 `(G_B, 0)` and `(G_C, 0)`, 688 `(G_D)` and 213 `(G_D, 0)`
/// (the globals at operands `+0x1c`, `+0x53`, `+0x68`, `+0x7b`/`+0x90`),
/// each seeing the earlier calls' words. Returns 0.
pub fn end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let t = slot_target(g, w(0x1), 694);
    let mut stack = vec![0, 0, 0, 0];
    g.call(t, 0, &stack);
    let a = g.read::<u32>(w(0x1c));
    let t = slot_target(g, w(0x16), 384);
    stack.insert(0, a);
    // call width: 5 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    for (op, slot, k) in [(0x29, 24, 0), (0x3a, 25, 1)] {
        let t = slot_target(g, w(op), slot);
        stack.splice(0..0, [0x300f, k]);
        // call width: 9 at most (the earlier calls' words stay on the stack)
        g.call(t, 0, &stack);
    }
    for (op, at) in [(0x4b, 0x53), (0x60, 0x68)] {
        let x = g.read::<u32>(w(at));
        let t = slot_target(g, w(op), 211);
        stack.splice(0..0, [x, 0]);
        // call width: 13 at most (the earlier calls' words stay on the stack)
        g.call(t, 0, &stack);
    }
    let d = g.read::<u32>(w(0x7b));
    let t = slot_target(g, w(0x75), 688);
    stack.insert(0, d);
    // call width: 14 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    let d = g.read::<u32>(w(0x90));
    let t = slot_target(g, w(0x88), 213);
    stack.splice(0..0, [d, 0]);
    // call width: 16 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    Ret::eax(0)
}

/// [`end`] (162 bytes, 1 overlay).
pub const END: Sig = Sig {
    len: 162,
    hash: 0x6fb2_4f96_b081_1fc4,
    calls: &[],
    imports: &[],
};
