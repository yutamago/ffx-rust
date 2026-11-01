//! `magic_0525.dll`'s own functions (besides the runtime and the shared
//! code): steps of a battle sequence (as `magic_0510`'s: each a callback
//! with the sequence object `seq`, `S = *G_S` the battle's state block,
//! `+0x198` the actor, `+0x1a0` the target).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// `seq[0x2c + 16 * seq[0xac]] = x`; returns twice `seq[0xac]` (the
/// original's `eax`).
fn record<G: Game>(g: &mut G, seq: u32, x: u32) -> u32 {
    let n = g
        .read::<u32>(seq + 0xac)
        .wrapping_add(g.read::<u32>(seq + 0xac));
    g.write::<u32>(seq.wrapping_add(n.wrapping_mul(8)).wrapping_add(0x2c), x);
    n
}

/// A step (0x100010b0, `(seq)`, pushed by 0x10001078): slot 486
/// `(actor, 11, 0, 0, 0x54, 32, 32, 20.0, 0, 0, 0)`, slot 434 `(*G_S,
/// CB)`, slot 450 `(seq, NEXT)` and 451 `(seq, 13)` (the earlier words
/// left on the stack). Returns slot 451's `eax`.
pub fn e10b0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let s = g.read::<u32>(w(0x4));
    let actor = g.read::<u32>(s + 0x198);
    let t = slot_target(g, w(0x43), 486);
    let mut stack = vec![actor, 0xb, 0, 0, 0x54, 0x20, 0x20, 0x41a0_0000, 0, 0, 0];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x50), 434);
    stack.splice(0..0, [g.read::<u32>(w(0x5b)), w(0x55)]);
    // call width: 13
    g.call(t, 0, &stack);
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0x68), 450);
    stack.splice(0..0, [seq, w(0x6d)]);
    // call width: 15
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x7d), 451);
    stack.splice(0..0, [seq, 0xd]);
    // call width: 17
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x100010b0 (147 bytes, 1 overlay).
pub const E10B0: Sig = Sig {
    len: 147,
    hash: 0xc972_242f_1e80_8c18,
    calls: &[],
    imports: &[],
};

/// A step (0x100011c0, `(seq)`, pushed by 0x10001199): slot 486 `(actor,
/// 1, 0, 0, 0x68, 32, 32, 0, 0, 0, 0)`, slot 434 `(*G_S, CB)` (486's
/// words left on the stack); unless the flag at operand `+0x6c` is set,
/// slot 434 `(*G_S, CB2)`; slot 450 `(seq, NEXT)` and 451 `(seq, 13)`.
/// Returns slot 451's `eax`.
pub fn e11c0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let s = g.read::<u32>(w(0x4));
    let actor = g.read::<u32>(s + 0x198);
    let t = slot_target(g, w(0x43), 486);
    let mut stack = vec![actor, 1, 0, 0, 0x68, 0x20, 0x20, 0, 0, 0, 0];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x50), 434);
    stack.splice(0..0, [g.read::<u32>(w(0x5b)), w(0x55)]);
    // call width: 13
    g.call(t, 0, &stack);
    if g.read::<u32>(w(0x6c)) == 0 {
        let t = slot_target(g, w(0x74), 434);
        let gs = g.read::<u32>(w(0x7f));
        g.call(t, 0, &[gs, w(0x79)]);
    }
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0x8f), 450);
    g.call(t, 0, &[seq, w(0x94)]);
    let t = slot_target(g, w(0xa4), 451);
    Ret::eax(g.call(t, 0, &[seq, 0xd, seq, w(0x94)]).eax)
}

/// 0x100011c0 (186 bytes, 1 overlay).
pub const E11C0: Sig = Sig {
    len: 186,
    hash: 0x59ed_2a3e_4bb2_7788,
    calls: &[],
    imports: &[],
};

/// A step (0x100012f0, `(seq)`, pushed by 0x10001215): slot 446
/// `(target, 2)` (0 when the flag at operand `+0x05` is set), slot 442
/// `(seq, target, 0)`, slot 443 `(seq, 16)`, the scales `+0x130` `(0.05,
/// 0.05, 0.05, 0)`, slot 440 `(*G_S, seq, 0, 0)`, slot 450 `(seq, NEXT)`
/// and 449 `(seq, END)` (the earlier words left on the stack). Returns
/// slot 449's `eax`.
pub fn e12f0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let flag = g.read::<u32>(w(0x5));
    let s = g.read::<u32>(w(0xb));
    let target = g.read::<u32>(s + 0x1a0);
    let t = slot_target(g, w(0x17), 446);
    g.call(t, 0, &[target, if flag == 0 { 2 } else { 0 }]);
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0x2d), 442);
    let mut stack = vec![seq, target, 0];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x44), 443);
    stack.splice(0..0, [seq, 0x10]);
    // call width: 5
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x54), 440);
    let gs = g.read::<u32>(w(0x5f));
    for off in [0x130, 0x134, 0x138] {
        g.write::<u32>(seq + off, 0x3d4c_cccd);
    }
    g.write::<u32>(seq + 0x13c, 0);
    stack.splice(0..0, [gs, seq, 0, 0]);
    // call width: 9
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x94), 450);
    stack.splice(0..0, [seq, w(0x99)]);
    // call width: 11
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xa7), 449);
    stack.splice(0..0, [seq, w(0xac)]);
    // call width: 13
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x100012f0 (191 bytes, 1 overlay).
pub const E12F0: Sig = Sig {
    len: 191,
    hash: 0x10b9_762c_0cea_ccd5,
    calls: &[],
    imports: &[],
};

/// A step (0x10001650, `(seq)`, pushed by 0x10001623): slot 447 `(target,
/// 0x4a)`, slot 492 `(target)`; slot 450 `(seq, NEXT)` with the step at
/// operand `+0x4c` when 492 said 2 or more, else `+0x45`; 449 `(seq,
/// END)`; the target recorded in the sequence ([`record`]). Returns
/// twice `seq[0xac]`.
pub fn e1650<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let s = g.read::<u32>(w(0x4));
    let target = g.read::<u32>(s + 0x1a0);
    let t = slot_target(g, w(0x11), 447);
    g.call(t, 0, &[target, 0x4a]);
    let t = slot_target(g, w(0x21), 492);
    let r = g.call(t, 0, &[target, target, 0x4a]).eax;
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0x38), 450);
    let next = if r.cast_signed() >= 2 {
        w(0x4c)
    } else {
        w(0x45)
    };
    g.call(t, 0, &[seq, next]);
    let t = slot_target(g, w(0x54), 449);
    g.call(t, 0, &[seq, w(0x59), seq, next]);
    Ret::eax(record(g, seq, target))
}

/// 0x10001650 (121 bytes, 1 overlay).
pub const E1650: Sig = Sig {
    len: 121,
    hash: 0xb010_b6d9_ce1e_4fc5,
    calls: &[],
    imports: &[],
};

/// A step (0x100016d0, `(seq)`, pushed by 0x10001695): slot 492
/// `(target)`, slot 447 `(target, 0x24)` when it said 1 or more, else
/// `(target, 0x23)`; slot 450 `(seq, NEXT)`, 449 `(seq, END)`; the target
/// recorded in the sequence ([`record`]). Returns twice `seq[0xac]`.
pub fn e16d0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let s = g.read::<u32>(w(0x4));
    let target = g.read::<u32>(s + 0x1a0);
    let t = slot_target(g, w(0x11), 492);
    let r = g.call(t, 0, &[target]).eax;
    let t = slot_target(g, w(0x25), 447);
    g.call(
        t,
        0,
        &[target, if r.cast_signed() >= 1 { 0x24 } else { 0x23 }],
    );
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0x3b), 450);
    g.call(t, 0, &[seq, w(0x4c)]);
    let t = slot_target(g, w(0x54), 449);
    g.call(t, 0, &[seq, w(0x59), seq, w(0x4c)]);
    Ret::eax(record(g, seq, target))
}

/// 0x100016d0 (121 bytes, 1 overlay).
pub const E16D0: Sig = Sig {
    len: 121,
    hash: 0x49de_e7c1_89aa_6180,
    calls: &[],
    imports: &[],
};
