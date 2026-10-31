//! `magic_0588.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sa_pool::take;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The current actor's word in the actor table: `TABLE[(*G_E)[1]]` (the
/// byte `+0x1` of the record at `*G_E`, the table at `table`).
fn actor<G: Game>(g: &mut G, e_at: u32, table: u32) -> u32 {
    let e = g.read::<u32>(e_at);
    let b = u32::from(g.read::<u8>(e.wrapping_add(1)));
    g.read::<u32>(table.wrapping_add(4 * b))
}

/// Starts the sound (0x10001e10, no arguments): slot 0 `(0x3016)` gives a
/// handle, slot 26 `(0x3016, 1)`; the byte at operand `+0x2a` 0x20, the
/// handle into `+0x31`; slot 10 `(handle, 0)`; `+0x44` = 2; slot 257
/// `(actor, 1)` ([`actor`]: `+0x3e`, `+0x55`). Each call sees the
/// earlier calls' words. Returns slot 257's `eax`.
pub fn sound<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let t = slot_target(g, w(0x1), 0);
    let mut stack = vec![0x3016];
    let h = g.call(t, 0, &stack).eax;
    let t = slot_target(g, w(0x11), 26);
    stack.splice(0..0, [0x3016, 1]);
    // call width: 3 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    g.write::<u8>(w(0x2a), 0x20);
    g.write::<u32>(w(0x31), h);
    let t = slot_target(g, w(0x24), 10);
    stack.splice(0..0, [h, 0]);
    // call width: 5 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    let e = g.read::<u32>(w(0x3e));
    g.write::<u32>(w(0x44), 2);
    let b = u32::from(g.read::<u8>(e.wrapping_add(1)));
    let a = g.read::<u32>(w(0x55).wrapping_add(4 * b));
    let t = slot_target(g, w(0x5a), 257);
    stack.splice(0..0, [a, 1]);
    // call width: 7 (the earlier calls' words stay on the stack)
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// [`sound`] (107 bytes, 1 overlay).
pub const SOUND: Sig = Sig {
    len: 107,
    hash: 0xd81b_b38f_5548_db09,
    calls: &[],
    imports: &[],
};

/// Starts the effect's part (0x10002510, no arguments; the record `R =
/// *G_R`, operand `+0x03`): `R[0xe00]` 1; a pool node ([`take`], the
/// owner the immediate at `+0x08`) with byte `+0x18` 1 and `+0x4` 0 put
/// at the head of the list (head `+0x31`/`+0x44`, the empty test `+0x1d`,
/// set `+0x5a`/`+0x5f`), `R[0xdec]` the node. Returns the node.
pub fn part_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = g.read::<u32>(w(0x3));
    g.write::<u32>(r + 0xe00, 1);
    let node = take(g, img, img.rel32_target(f + 0x17), w(0x8));
    let empty = g.read::<u32>(w(0x1d)) == 0;
    g.write::<u8>(node + 0x18, 1);
    g.write::<u32>(node + 4, 0);
    if empty {
        g.write::<u32>(node, 0);
        g.write::<u32>(node + 8, node);
        g.write::<u32>(r + 0xdec, node);
        g.write::<u32>(w(0x5a), node);
        g.write::<u32>(w(0x5f), node);
    } else {
        let head = g.read::<u32>(w(0x31));
        g.write::<u32>(node, head);
        g.write::<u32>(head.wrapping_add(4), node);
        g.write::<u32>(node + 8, node);
        g.write::<u32>(r + 0xdec, node);
        g.write::<u32>(w(0x44), node);
    }
    Ret::eax(node)
}

/// [`part_start`] (101 bytes, 1 overlay).
pub const PART_START: Sig = Sig {
    len: 101,
    hash: 0x6c00_f50b_c24b_9da4,
    calls: &[(0x17, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// Sets the actor's colours (0x10002800, no arguments): for the current
/// actor ([`actor`]: `+0x01`, `+0x12`) slots 357 `(a, -1, 0x3540)`, 356
/// `(a, -1, 0x3bd0)`, 355 `(a, -1)` and 354 `(a, -1)`, each seeing the
/// earlier calls' words. Returns slot 354's `eax`.
pub fn colours<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let a = actor(g, w(0x1), w(0x12));
    let mut stack: Vec<u32> = Vec::new();
    let mut eax = 0;
    for (op, slot, k) in [
        (0x17, 357, Some(0x3540)),
        (0x27, 356, Some(0x3bd0)),
        (0x3c, 355, None),
        (0x4c, 354, None),
    ] {
        if let Some(k) = k {
            stack.insert(0, k);
        }
        stack.splice(0..0, [a, u32::MAX]);
        let t = slot_target(g, w(op), slot);
        // call width: 10 at most (the earlier calls' words stay on the stack)
        eax = g.call(t, 0, &stack).eax;
    }
    Ret::eax(eax)
}

/// [`colours`] (96 bytes, 1 overlay).
pub const COLOURS: Sig = Sig {
    len: 96,
    hash: 0x1b62_3950_7bdc_7d08,
    calls: &[],
    imports: &[],
};
