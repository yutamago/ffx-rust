//! `magic_0526.dll`'s own functions (besides the runtime and the shared
//! code): steps of a battle sequence (as `magic_0510`'s: each a callback
//! with the sequence object `seq`, `S = *G_S` the battle's state block).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The sequence's current target: `S[0x1a0 + 4 * seq[0x2c + 16 *
/// seq[0xac]]]` (`S` through the operand `op`).
fn target<G: Game>(g: &G, img: &Image, f: u32, seq: u32, op: u32) -> u32 {
    let n = g.read::<u32>(seq + 0xac).wrapping_mul(2);
    let i = g.read::<u32>(seq.wrapping_add(n.wrapping_mul(8)).wrapping_add(0x2c));
    let s = g.read::<u32>(img.word(f + op));
    g.read::<u32>(s.wrapping_add(i.wrapping_mul(4)).wrapping_add(0x1a0))
}

/// A step (0x10001140, `(seq)`, pushed by 0x10001121): slot 434 `(S,
/// CB)`, the count at operand `+0x2a` up; slot 450 `(seq, NEXT)` with the
/// step at `+0x49` when there is more than one target (`S[0x19c]`), else
/// `+0x50`; 449 `(seq, END)`; the actor recorded in the sequence
/// (`seq[0x2c + 16 * seq[0xac]]`). Returns twice `seq[0xac]`.
pub fn e1140<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let s = g.read::<u32>(w(0x4));
    let actor = g.read::<u32>(s + 0x198);
    let t = slot_target(g, w(0x17), 434);
    g.call(t, 0, &[s, w(0x11)]);
    let n = g.read::<u32>(w(0x2a));
    g.write::<u32>(w(0x2a), n.wrapping_add(1));
    let seq = fr.arg(g, 0);
    let s = g.read::<u32>(w(0x24));
    let next = if g.read::<i32>(s + 0x19c) > 1 {
        w(0x49)
    } else {
        w(0x50)
    };
    let t = slot_target(g, w(0x3c), 450);
    g.call(t, 0, &[seq, next]);
    let t = slot_target(g, w(0x58), 449);
    g.call(t, 0, &[seq, w(0x5d), seq, next]);
    let k = g.read::<u32>(seq + 0xac).wrapping_mul(2);
    g.write::<u32>(
        seq.wrapping_add(k.wrapping_mul(8)).wrapping_add(0x2c),
        actor,
    );
    Ret::eax(k)
}

/// 0x10001140 (125 bytes, 1 overlay).
pub const E1140: Sig = Sig {
    len: 125,
    hash: 0x1f17_2975_737b_aebf,
    calls: &[],
    imports: &[],
};

/// A step (0x10001650, `(seq)`, pushed by 0x1000148c): slot 442 `(seq,
/// target, 6)` ([`target`]), slot 443 `(seq, 16)`, the scales `+0x130`
/// `(0.15, 0.15, 0.15, 0)`, slot 450 `(seq, NEXT)` (the next step,
/// [`e16e0`]) and 451 `(seq, 8)` (the earlier words left on the stack).
/// Returns slot 451's `eax`. (The census counts this and the next step
/// as one 258-byte function.)
pub fn e1650<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let seq = fr.arg(g, 0);
    let x = target(g, img, f, seq, 0x16);
    let t = slot_target(g, w(0x22), 442);
    let mut stack = vec![seq, x, 6];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x30), 443);
    stack.splice(0..0, [seq, 0x10]);
    // call width: 5
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x40), 450);
    for off in [0x130, 0x134, 0x138] {
        g.write::<u32>(seq + off, 0x3e19_999a);
    }
    g.write::<u32>(seq + 0x13c, 0);
    stack.splice(0..0, [seq, w(0x73)]);
    // call width: 7
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x7b), 451);
    stack.splice(0..0, [seq, 8]);
    // call width: 9
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x10001650 (144 bytes, 1 overlay).
pub const E1650: Sig = Sig {
    len: 144,
    hash: 0xed0d_a85b_5ba6_79c8,
    calls: &[],
    imports: &[],
};

/// A step (0x100016e0, `(seq)`, pushed by [`e1650`]): slot 446 `(target,
/// 0)`, slot 440 `(*G_S, seq, 0, 1)`, slot 450 `(seq, NEXT)` and 449
/// `(seq, END)` (the earlier words left on the stack). Returns slot
/// 449's `eax`.
pub fn e16e0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let seq = fr.arg(g, 0);
    let x = target(g, img, f, seq, 0x16);
    let t = slot_target(g, w(0x22), 446);
    let mut stack = vec![x, 0];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x2f), 440);
    stack.splice(0..0, [g.read::<u32>(w(0x40)), seq, 0, 1]);
    // call width: 6
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x47), 450);
    stack.splice(0..0, [seq, w(0x4c)]);
    // call width: 8
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x5a), 449);
    stack.splice(0..0, [seq, w(0x5f)]);
    // call width: 10
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x100016e0 (114 bytes, 1 overlay).
pub const E16E0: Sig = Sig {
    len: 114,
    hash: 0x17fa_6716_a428_e7e3,
    calls: &[],
    imports: &[],
};
