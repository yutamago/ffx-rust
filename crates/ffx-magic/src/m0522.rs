//! `magic_0522.dll`'s own functions (besides the runtime and the shared
//! code): steps of a battle sequence (as `magic_0510`'s: each a callback
//! with the sequence object `seq`, `S = *G_S` the battle's state block).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// A step (0x10001140, `(seq)`, pushed by 0x10001111): six slot 434
/// `(*G_S, CB)` calls (callback at operand `+0x16`); the count at `+0x2f`
/// goes up, and from 10 on slot 450 `(seq, NEXT)` and 451 `(seq, 10)`.
/// Returns slot 451's `eax`, or the count.
pub fn e1140<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    for _ in 0..6 {
        let t = slot_target(g, w(0x11), 434);
        let s = g.read::<u32>(w(0x1c));
        g.call(t, 0, &[s, w(0x16)]);
    }
    let n = g.read::<u32>(w(0x2f)).wrapping_add(1);
    g.write::<u32>(w(0x35), n);
    if n.cast_signed() < 0xa {
        return Ret::eax(n);
    }
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0x3f), 450);
    g.call(t, 0, &[seq, w(0x4d)]);
    let t = slot_target(g, w(0x55), 451);
    Ret::eax(g.call(t, 0, &[seq, 0xa, seq, w(0x4d)]).eax)
}

/// 0x10001140 (106 bytes, 1 overlay).
pub const E1140: Sig = Sig {
    len: 106,
    hash: 0x3a4d_061f_5a53_587e,
    calls: &[],
    imports: &[],
};

/// A step (0x10001360, `(seq)`, pushed by 0x100011b9): slot 442 `(seq,
/// actor, 0)`, slot 443 `(seq, 16)`, the sequence's scales `+0x130`
/// `(0.8, 0.8, 0.8, 0)`, slot 440 `(*G_S, seq, 0, 0)`, slot 450 `(seq,
/// NEXT)` and 451 `(seq, 10)` (the earlier words left on the stack).
/// Returns slot 451's `eax`.
pub fn e1360<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let s = g.read::<u32>(w(0x4));
    let seq = fr.arg(g, 0);
    let actor = g.read::<u32>(s + 0x198);
    let t = slot_target(g, w(0x15), 442);
    let mut stack = vec![seq, actor, 0];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x23), 443);
    stack.splice(0..0, [seq, 0x10]);
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x33), 440);
    let gs = g.read::<u32>(w(0x3e));
    for off in [0x130, 0x134, 0x138] {
        g.write::<u32>(seq + off, 0x3f4c_cccd);
    }
    g.write::<u32>(seq + 0x13c, 0);
    stack.splice(0..0, [gs, seq, 0, 0]);
    // call width: 9
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x73), 450);
    stack.splice(0..0, [seq, w(0x78)]);
    // call width: 11
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x86), 451);
    stack.splice(0..0, [seq, 0xa]);
    // call width: 13
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x10001360 (155 bytes, 1 overlay).
pub const E1360: Sig = Sig {
    len: 155,
    hash: 0x37fa_af88_674a_ed7a,
    calls: &[],
    imports: &[],
};

/// A step (0x10001400, `(seq)`, pushed by 0x100013d8): slot 446 `(t,
/// 0)` for each of the `S[0x19c]` targets (`S[0x1a0..]`), slot 450
/// `(seq, NEXT)` and 449 `(seq, END)`. Returns slot 449's `eax`.
pub fn e1400<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut s = g.read::<u32>(w(0x4));
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(s + 0x19c) {
        let x = g.read::<u32>(s.wrapping_add(0x1a0 + 4 * i));
        let t = slot_target(g, w(0x26), 446);
        g.call(t, 0, &[x, 0]);
        s = g.read::<u32>(w(0x33));
        i += 1;
    }
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0x48), 450);
    g.call(t, 0, &[seq, w(0x56)]);
    let t = slot_target(g, w(0x5e), 449);
    Ret::eax(g.call(t, 0, &[seq, w(0x63), seq, w(0x56)]).eax)
}

/// 0x10001400 (118 bytes, 1 overlay).
pub const E1400: Sig = Sig {
    len: 118,
    hash: 0xd48b_0380_25b3_ce32,
    calls: &[],
    imports: &[],
};
