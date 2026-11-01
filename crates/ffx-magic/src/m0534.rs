//! `magic_0534.dll`'s own functions (besides the runtime and the shared
//! code): steps of a battle sequence (as `magic_0510`'s: each a callback
//! with the sequence object `seq`, `S = *G_S` the battle's state block).

use crate::m0018::st0_f32;
use crate::runtime::slot_target;
use crate::s4_summon::trunc;
use crate::sb_sse::{cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `seq[0x2c + 16 * seq[0xac]] = x`; returns twice `seq[0xac]`.
fn record<G: Game>(g: &mut G, seq: u32, x: u32) -> u32 {
    let n = g.read::<u32>(seq + 0xac).wrapping_mul(2);
    g.write::<u32>(seq.wrapping_add(n.wrapping_mul(8)).wrapping_add(0x2c), x);
    n
}

/// A step (0x100011a0, `(seq)`, pushed by 0x10001182): the target count
/// at operand `+0x04` goes up; while it is below `S[0x19c]`, slot 450
/// `(seq, AGAIN)` and 451 `(seq, 15)` (returns 451's `eax`); else slot
/// 447 `(actor, 0xc2)`, slot 459 `(actor, 2, 4)`, slot 450 `(seq, NEXT)`
/// and 449 `(seq, END)` (the earlier words left on the stack), the actor
/// recorded in the sequence (returns twice `seq[0xac]`).
pub fn e11a0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let n = g.read::<u32>(w(0x4)).wrapping_add(1);
    let s = g.read::<u32>(w(0xa));
    let actor = g.read::<u32>(s + 0x198);
    g.write::<u32>(w(0x1e), n);
    let seq = fr.arg(g, 0);
    if n.cast_signed() < g.read::<i32>(s + 0x19c) {
        let t = slot_target(g, w(0x23), 450);
        g.call(t, 0, &[seq, w(0x8f)]);
        let t = slot_target(g, w(0x97), 451);
        return Ret::eax(g.call(t, 0, &[seq, 0xf, seq, w(0x8f)]).eax);
    }
    let t = slot_target(g, w(0x23), 447);
    let mut stack = vec![actor, 0xc2];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x38), 459);
    stack.splice(0..0, [actor, 2, 4]);
    // call width: 5
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x4a), 450);
    stack.splice(0..0, [seq, w(0x58)]);
    // call width: 7
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x60), 449);
    stack.splice(0..0, [seq, w(0x65)]);
    // call width: 9
    g.call(t, 0, &stack);
    Ret::eax(record(g, seq, actor))
}

/// 0x100011a0 (173 bytes, 1 overlay).
pub const E11A0: Sig = Sig {
    len: 173,
    hash: 0xff5f_d7ca_fac6_4f82,
    calls: &[],
    imports: &[],
};

/// A step (0x100012b0, `(seq)`, pushed by 0x10001282): two slot 434 `(S,
/// CB)`; the second's answer (a sequence) records the target count (the
/// global at operand `+0x43`); slot 447 `(actor, 0xc2)`, slot 450 `(seq,
/// NEXT)` and 449 `(seq, END)` (the earlier words left on the stack); the
/// actor recorded in this sequence. Returns twice `seq[0xac]`.
pub fn e12b0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let s = g.read::<u32>(w(0x4));
    let actor = g.read::<u32>(s + 0x198);
    let t = slot_target(g, w(0x17), 434);
    let mut stack = vec![s, w(0x11)];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x24), 434);
    stack.splice(0..0, [g.read::<u32>(w(0x2f)), w(0x29)]);
    // call width: 4
    let other = g.call(t, 0, &stack).eax;
    let n = g.read::<u32>(w(0x43));
    record(g, other, n);
    let t = slot_target(g, w(0x53), 447);
    stack.splice(0..0, [actor, 0xc2]);
    // call width: 6
    g.call(t, 0, &stack);
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0x61), 450);
    stack.splice(0..0, [seq, w(0x6f)]);
    // call width: 8
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x77), 449);
    stack.splice(0..0, [seq, w(0x7c)]);
    // call width: 10
    g.call(t, 0, &stack);
    Ret::eax(record(g, seq, actor))
}

/// 0x100012b0 (156 bytes, 1 overlay).
pub const E12B0: Sig = Sig {
    len: 156,
    hash: 0x9201_07fe_ef93_dd01,
    calls: &[],
    imports: &[],
};

/// A step (0x10001390, `(seq)`, pushed by 0x10001149 and 0x100012c1):
/// slot 441 `(seq, actor, 0x48)`, slot 443 `(seq, 16)`, the scales
/// `+0x130` `(0.55, 0.55, 0.55, 0)`, slot 440 `(S, seq, 0, 0)`, slot 450
/// `(seq, NEXT)` and 449 `(seq, END)` (the earlier words left on the
/// stack). Returns slot 449's `eax`.
pub fn e1390<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let s = g.read::<u32>(w(0x4));
    let seq = fr.arg(g, 0);
    let actor = g.read::<u32>(s + 0x198);
    let t = slot_target(g, w(0x15), 441);
    let mut stack = vec![seq, actor, 0x48];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x23), 443);
    stack.splice(0..0, [seq, 0x10]);
    // call width: 5
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x33), 440);
    let gs = g.read::<u32>(w(0x3e));
    for off in [0x130, 0x134, 0x138] {
        g.write::<u32>(seq + off, 0x3f0c_cccd);
    }
    g.write::<u32>(seq + 0x13c, 0);
    stack.splice(0..0, [gs, seq, 0, 0]);
    // call width: 9
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x73), 450);
    stack.splice(0..0, [seq, w(0x78)]);
    // call width: 11
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x86), 449);
    stack.splice(0..0, [seq, w(0x8b)]);
    // call width: 13
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x10001390 (158 bytes, 1 overlay).
pub const E1390: Sig = Sig {
    len: 158,
    hash: 0x2dd9_7f1a_365d_9312,
    calls: &[],
    imports: &[],
};

/// A step (0x10001430, `(seq)`, a sequence step; a /GS frame): the
/// target `T = S[0x1a0 + 4 idx]` (`S = *G_S`, `idx` the record's target
/// `seq[0x2c + 16 k]`, `k = seq[0xac]`); slot 474 `(S[0x198], 0x48, &A,
/// &B)` the source point `B` (`A` = 0), slot 457 `(T, 6, &C)` the target
/// point; `D = C - B` (four floats), slot 453 `(&D)` its length `l` (ST0,
/// as `f32`); the step count `n = l / K1` truncated, the step `D * (K2 /
/// n)`; the record's `+0xd0..` = `(B, 0)`, `+0xe0..` = `(step, 0)`,
/// `+0x130..` = `(0.17, 0.17, 0.17, 0)`, `seq[0x30 + 16 k]` = `n`; slots
/// 444 `(0)`, 440 `(*G_S, seq, 0, 1)` and 450 `(seq, NEXT)`, each seeing
/// the words the earlier ones left on the stack. Locals: `A` `ebp - 0x44`,
/// `C` `- 0x34`, `B` `- 0x24`, `D` `- 0x14`. Returns slot 450's `eax`.
pub fn aim<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = g.read::<u32>(w(0x12));
    let seq = fr.arg(g, 0);
    let k = g.read::<u32>(seq + 0xac);
    let idx = g.read::<u32>(seq + 0x2c + 16 * k);
    let target = g.read::<u32>(s + 0x1a0 + 4 * idx);
    let ebp = fr.esp.wrapping_sub(4);
    let (a, c, b, d) = (
        ebp.wrapping_sub(0x44),
        ebp.wrapping_sub(0x34),
        ebp.wrapping_sub(0x24),
        ebp.wrapping_sub(0x14),
    );
    g.write::<u32>(a + 8, 0);
    // junk read: [ebp-0x38] (A's fourth word) never written; 0 (brief 2026-10-24)
    g.write::<u32>(a + 0xc, 0);
    let t = slot_target(g, w(0x3e), 474);
    let h = g.read::<u32>(s + 0x198);
    g.write::<u32>(a + 4, 0);
    g.write::<u32>(a, 0);
    let mut stack = vec![h, 0x48, a, b];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x65), 457);
    stack.splice(0..0, [target, 6, c]);
    // call width: 7
    g.call(t, 0, &stack);
    for i in 0..4u32 {
        let x = sub(g.read::<f32>(c + 4 * i), g.read::<f32>(b + 4 * i));
        g.write::<f32>(d + 4 * i, x);
    }
    let t = slot_target(g, w(0x83), 453);
    stack.insert(0, d);
    // call width: 8
    let len = st0_f32(g.call(t, 0, &stack).st0);
    let n = trunc(div(len, g.read::<f32>(w(0xcd))));
    let step = div(g.read::<f32>(w(0xd5)), cvt(n));
    let dv = [0u32, 1, 2, 3].map(|i| mul(g.read::<f32>(d + 4 * i), step));
    g.write::<f32>(d + 0xc, dv[3]);
    for i in 0..3u32 {
        let v = g.read::<u32>(b + 4 * i);
        g.write::<u32>(seq + 0xd0 + 4 * i, v);
    }
    g.write::<u32>(seq + 0xdc, 0);
    for (i, x) in (0u32..).zip(&dv[..3]) {
        g.write::<f32>(seq + 0xe0 + 4 * i, *x);
    }
    g.write::<u32>(seq + 0xec, 0);
    for o in [0x130u32, 0x134, 0x138] {
        g.write::<u32>(seq + o, 0x3e2e_147b);
    }
    g.write::<u32>(seq + 0x13c, 0);
    let k = g.read::<u32>(seq + 0xac);
    for (i, x) in (0u32..).zip(&dv[..3]) {
        g.write::<f32>(d + 4 * i, *x);
    }
    g.write::<u32>(seq + 16 * (k + 3), n);
    let t = slot_target(g, w(0x1ac), 444);
    stack.insert(0, 0);
    // call width: 9
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x1b9), 440);
    let sg = g.read::<u32>(w(0x1ca));
    stack.splice(0..0, [sg, seq, 0, 1]);
    // call width: 13
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x1d1), 450);
    stack.splice(0..0, [seq, w(0x1d6)]);
    // call width: 15
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// [`aim`] (502 bytes, 1 overlay).
pub const AIM: Sig = Sig {
    len: 502,
    hash: 0xe13d_e7b2_b478_653c,
    calls: &[
        // __security_check_cookie
        (0x1ee, Callee::Any),
    ],
    imports: &[],
};
