//! `magic_0529.dll`'s own functions (besides the runtime and the shared
//! code): steps of a battle sequence (as `magic_0510`'s: each a callback
//! with the sequence object `seq`, `S = *G_S` the battle's state block).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// A step (0x10001000, `(seq)`, pushed by 0x10001965): the actor
/// (`S[0x198]`) to the global at operand `+0x0f`, slot 481 `(actor)` its
/// model (`+0x24`), slot 278 `(model, 0, 0)`, `(model, 1, 1)`, `(model,
/// 2, 0)`, the step counters `+0x7b`, `+0x85` 0, slot 450 `(seq, NEXT)`
/// and 449 `(seq, END)` (the earlier words left on the stack). Returns
/// slot 449's `eax`.
pub fn e1000<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let s = g.read::<u32>(w(0x4));
    let actor = g.read::<u32>(s + 0x198);
    g.write::<u32>(w(0xf), actor);
    let t = slot_target(g, w(0x15), 481);
    let mut stack = vec![actor];
    let model = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x24), model);
    let t = slot_target(g, w(0x2c), 278);
    stack.splice(0..0, [model, 0, 0]);
    // call width: 4
    g.call(t, 0, &stack);
    for (tab, op, a, b) in [(0x39u32, 0x49u32, 1u32, 1u32), (0x50, 0x60, 2, 0)] {
        let t = slot_target(g, w(tab), 278);
        stack.splice(0..0, [g.read::<u32>(w(op)), a, b]);
        // call width: 10
        g.call(t, 0, &stack);
    }
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0x67), 450);
    g.write::<u32>(w(0x7b), 0);
    g.write::<u32>(w(0x85), 0);
    stack.splice(0..0, [seq, w(0x6c)]);
    // call width: 12
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x90), 449);
    stack.splice(0..0, [seq, w(0x95)]);
    // call width: 14
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x10001000 (169 bytes, 1 overlay).
pub const E1000: Sig = Sig {
    len: 169,
    hash: 0xd613_d9cd_e48f_18c2,
    calls: &[],
    imports: &[],
};

/// A step (0x10001100, `(seq)`, pushed by 0x100010c8): slot 444 `(0)`,
/// slot 525 `(model)`, slot 8 `(model, 0)`, two slot 434 `(S, CB)` and
/// slot 450 `(seq, NEXT)` (the earlier words left on the stack). Returns
/// slot 450's `eax`.
pub fn e1100<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let t = slot_target(g, w(0x4), 444);
    let mut stack = vec![0];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x13), 525);
    stack.insert(0, g.read::<u32>(w(0x19)));
    // call width: 10
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x26), 8);
    stack.splice(0..0, [g.read::<u32>(w(0x2e)), 0]);
    // call width: 4
    g.call(t, 0, &stack);
    for (tab, cb, gs) in [(0x38u32, 0x3du32, 0x43u32), (0x50, 0x55, 0x5b)] {
        let t = slot_target(g, w(tab), 434);
        stack.splice(0..0, [g.read::<u32>(w(gs)), w(cb)]);
        // call width: 8
        g.call(t, 0, &stack);
    }
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0x68), 450);
    stack.splice(0..0, [seq, w(0x6d)]);
    // call width: 10
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x10001100 (129 bytes, 1 overlay).
pub const E1100: Sig = Sig {
    len: 129,
    hash: 0xbce7_d752_c387_654b,
    calls: &[],
    imports: &[],
};

/// The step's end shared by the counting steps: slot 526 `()` and slot
/// 467 `(*G_F)` (the operands of 526's and 467's tables and of `G_F`).
/// Returns slot 467's `eax`.
fn flash<G: Game>(g: &mut G, img: &Image, f: u32, (t526, t467, gf): (u32, u32, u32)) -> u32 {
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(t526), 526);
    g.call(t, 0, &[]);
    let t = slot_target(g, w(t467), 467);
    let x = g.read::<u32>(w(gf));
    g.call(t, 0, &[x]).eax
}

/// A counting step (0x10001790, `(seq)`, pushed by 0x10001741): the
/// count at operand `+0x04` goes up; past 4 the flag bit 3 of `*G_F`'s
/// `+0x10` is cleared, the count 0 and slot 450 `(seq, NEXT)`; then
/// [`flash`]. Returns slot 467's `eax`.
pub fn e1790<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let n = g.read::<u32>(w(0x4)).wrapping_add(1);
    g.write::<u32>(w(0xa), n);
    if n.cast_signed() > 4 {
        let o = g.read::<u32>(w(0x14));
        let fl = g.read::<u32>(o + 0x10);
        g.write::<u32>(o + 0x10, fl & !8);
        let seq = fr.arg(g, 0);
        let t = slot_target(g, w(0x22), 450);
        g.write::<u32>(w(0x31), 0);
        g.call(t, 0, &[seq, w(0x19)]);
    }
    Ret::eax(flash(g, img, f, (0x3f, 0x4c, 0x52)))
}

/// 0x10001790 (99 bytes, 1 overlay).
pub const E1790: Sig = Sig {
    len: 99,
    hash: 0x17bb_8e4e_d6fc_d3af,
    calls: &[],
    imports: &[],
};

/// A counting step (0x100016c0, `(seq)`, pushed by 0x100016a9): the
/// count at operand `+0x05` goes up; from 150 on slot 434 `(S, CB)`, the
/// count 0 and slot 450 `(seq, NEXT)`; before, slot 434 `(S, CB2)` every
/// 12th frame (below 75) or 8th (from 75); then [`flash`]. Returns slot
/// 467's `eax`.
pub fn e16c0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let n = g.read::<i32>(w(0x5)).wrapping_add(1);
    g.write::<i32>(w(0xc), n);
    if n >= 0x96 {
        let t = slot_target(g, w(0x64), 434);
        let gs = g.read::<u32>(w(0x6f));
        g.call(t, 0, &[gs, w(0x69)]);
        let seq = fr.arg(g, 0);
        let t = slot_target(g, w(0x7c), 450);
        g.write::<u32>(w(0x90), 0);
        g.call(t, 0, &[seq, w(0x81), gs, w(0x69)]);
    } else {
        let r = if n < 0x4b { n % 12 } else { n % 8 };
        if r == 0 {
            let t = slot_target(g, w(0x47), 434);
            let gs = g.read::<u32>(w(0x52));
            g.call(t, 0, &[gs, w(0x4c)]);
        }
    }
    Ret::eax(flash(g, img, f, (0x9e, 0xab, 0xb1)))
}

/// 0x100016c0 (194 bytes, 1 overlay).
pub const E16C0: Sig = Sig {
    len: 194,
    hash: 0xc9ab_b591_bdfa_25a7,
    calls: &[],
    imports: &[],
};

/// A counting step (0x10001800, `(seq)`, pushed by 0x100017a9): below
/// 50, every 8th frame slot 434 `(S, CB)`; the fade byte `*G_F + 0x2b`
/// becomes `(130 - n) * 128 / 130`; unless the count `n` is 130, slot
/// 526 `()`; slot 467 `(*G_F)`. The count goes up; past 130 slot 446
/// `(actor, 10)`, slot 276 `(F[1], 0)`, slot 296 `(actor, 0)`, slot 466
/// `(*G_F)`, slot 527 `(0)` (the earlier words left on the stack), the
/// sequence's flag bit 2 and `*G_F` 0. Returns the count, or `seq`.
pub fn e1800<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut n = g.read::<i32>(w(0x6));
    if n < 0x32 && n % 8 == 0 {
        let t = slot_target(g, w(0x20), 434);
        let gs = g.read::<u32>(w(0x2b));
        g.call(t, 0, &[gs, w(0x25)]);
        n = g.read::<i32>(w(0x39));
    }
    let b = 0x82i32.wrapping_sub(n).wrapping_shl(7) / 130;
    let mut fl = g.read::<u32>(w(0x53));
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    g.write::<u8>(fl.wrapping_add(0x2b), b as u8);
    if n != 0x82 {
        let t = slot_target(g, w(0x6e), 526);
        g.call(t, 0, &[]);
        fl = g.read::<u32>(w(0x7c));
    }
    let t = slot_target(g, w(0x81), 467);
    g.call(t, 0, &[fl]);
    let n = g.read::<i32>(w(0x8f)).wrapping_add(1);
    g.write::<i32>(w(0x98), n);
    if n <= 0x82 {
        return Ret::eax(n.cast_unsigned());
    }
    let actor = g.read::<u32>(w(0xac));
    let t = slot_target(g, w(0xa4), 446);
    let mut stack = vec![actor, 0xa];
    g.call(t, 0, &stack);
    let o = g.read::<u32>(w(0xb9));
    let t = slot_target(g, w(0xc3), 276);
    stack.splice(0..0, [g.read::<u32>(o + 4), 0]);
    // call width: 4
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xd0), 296);
    stack.splice(0..0, [g.read::<u32>(w(0xd8)), 0]);
    // call width: 6
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xe5), 466);
    stack.insert(0, g.read::<u32>(w(0xeb)));
    // call width: 7
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xf8), 527);
    stack.insert(0, 0);
    // call width: 8
    g.call(t, 0, &stack);
    let seq = fr.arg(g, 0);
    let s = g.read::<u32>(seq);
    g.write::<u32>(seq, s | 4);
    g.write::<u32>(w(0x111), 0);
    Ret::eax(seq)
}

/// 0x10001800 (283 bytes, 1 overlay).
pub const E1800: Sig = Sig {
    len: 283,
    hash: 0x1c04_a2ac_a96c_b866,
    calls: &[],
    imports: &[],
};
