//! `magic_0528.dll`'s own functions (besides the runtime and the shared
//! code): steps of a battle sequence (as `magic_0510`'s: each a callback
//! with the sequence object `seq`, `S = *G_S` the battle's state block).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// A step (0x10001000, `(seq)`, pushed by 0x100016a5): slot 481
/// `(actor)` gives a handle `h`, slot 21 `(h, 1, 1.0)` and slot 278 `(h,
/// 1, 0)`, the flag at operand `+0x5b` 0, slot 450 `(seq, NEXT)` and 449
/// `(seq, END)` (the earlier words left on the stack, 481's overwritten
/// with 1.0). Returns slot 449's `eax`.
pub fn e1000<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let s = g.read::<u32>(w(0x4));
    let actor = g.read::<u32>(s + 0x198);
    let t = slot_target(g, w(0x10), 481);
    let h = g.call(t, 0, &[actor]).eax;
    let t = slot_target(g, w(0x1e), 21);
    let mut stack = vec![h, 1, 0x3f80_0000];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x34), 278);
    stack.splice(0..0, [h, 1, 0]);
    // call width: 6
    g.call(t, 0, &stack);
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0x46), 450);
    g.write::<u32>(w(0x5b), 0);
    stack.splice(0..0, [seq, w(0x54)]);
    // call width: 8
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x66), 449);
    stack.splice(0..0, [seq, w(0x6b)]);
    // call width: 10
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x10001000 (126 bytes, 1 overlay).
pub const E1000: Sig = Sig {
    len: 126,
    hash: 0x1f20_5ca8_4974_64d1,
    calls: &[],
    imports: &[],
};

/// A step (0x100014f0, `(seq)`, pushed by 0x100011bf): the sequence's
/// flag bit 4 (`+0xbc`), slot 441 `(seq, actor, 0xeb)`, its points `+0xd0`
/// `(0, 15, 0, 0)` and `+0x100` `(-30, 0, 0, 0)` and scales `+0x130`
/// `(0.65, 0.65, 0.65, 0)`; slot 440 `(*G_S, seq, 0, 2)`, slot 450
/// `(seq, NEXT)` and 449 `(seq, END)` (the earlier words left on the
/// stack). Returns slot 449's `eax`.
pub fn e14f0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let s = g.read::<u32>(w(0x4));
    let actor = g.read::<u32>(s + 0x198);
    let seq = fr.arg(g, 0);
    let fl = g.read::<u32>(seq + 0xbc);
    g.write::<u32>(seq + 0xbc, fl | 0x10);
    let t = slot_target(g, w(0x20), 441);
    let mut stack = vec![seq, actor, 0xeb];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x2e), 440);
    let gs = g.read::<u32>(w(0x39));
    for (off, v) in [
        (0xd0u32, 0u32),
        (0xd4, 0x4170_0000),
        (0xd8, 0),
        (0xdc, 0),
        (0x100, 0xc1f0_0000),
        (0x104, 0),
        (0x108, 0),
        (0x10c, 0),
        (0x130, 0x3f26_6666),
        (0x134, 0x3f26_6666),
        (0x138, 0x3f26_6666),
        (0x13c, 0),
    ] {
        g.write::<u32>(seq + off, v);
    }
    stack.splice(0..0, [gs, seq, 0, 2]);
    // call width: 7
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xbe), 450);
    stack.splice(0..0, [seq, w(0xc3)]);
    // call width: 9
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xd1), 449);
    stack.splice(0..0, [seq, w(0xd6)]);
    // call width: 11
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x100014f0 (233 bytes, 1 overlay).
pub const E14F0: Sig = Sig {
    len: 233,
    hash: 0xaf02_f97e_a4a8_c802,
    calls: &[],
    imports: &[],
};
