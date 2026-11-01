//! `magic_0529.dll`'s own functions (besides the runtime and the shared
//! code): steps of a battle sequence (as `magic_0510`'s: each a callback
//! with the sequence object `seq`, `S = *G_S` the battle's state block).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

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

/// A spark's start (0x10001330, a callback, cdecl `(obj)`; a /GS frame):
/// a direction `v` of three `rand()` values (`x = r / R * S - T`, `y` the
/// same with `U` and a double offset, `z` like `x`) in the local at `ebp -
/// 0x14`, normalised by slot 232 `(&v, &v)`; `e = v * K`; a speed
/// `rand() / R * A + B` scales `v` (`y` then by `C`, `z` by `D`); a size
/// `rand() / R * E + F`. The object's flags `+0xbc |= 0x2000`, slot 442 `(obj,
/// actor, 0)` (the actor `(*G_A)[0x198]`); `+0xd0..` = `v`, `+0xe0..` =
/// `e`, `+0xdc`/`+0xec` 0, `+0x100` 90.0, `+0x104..+0x10c` 0, `+0x130..`
/// the size (`+0x13c` 0); slot 440 `(*G_A, obj, 0, 1)`, slot 450 `(obj,
/// CB1)`, slot 449 `(obj, CB2)` (each call seeing the earlier words).
/// Returns slot 449's `eax`.
pub fn spark<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, off: u32| f32::from_bits(g.read::<u32>(w(off)));
    let call = |g: &mut G, p: &Pushes, slot: u32| {
        let t = slot_target(g, w(0x99), slot);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0).eax
    };
    let obj = fr.arg(g, 0);
    let actor = g.read::<u32>(g.read::<u32>(w(0x11)).wrapping_add(0x198));
    let v = fr.esp.wrapping_sub(0x18);
    let r = g.rand();
    let x = sub(mul(div(cvt(r), k(g, 0x34)), k(g, 0x3c)), k(g, 0x44));
    g.write::<u32>(v, x.to_bits());
    let r = g.rand();
    let y = mul(div(cvt(r), k(g, 0x5a)), k(g, 0x62));
    let off = f64::from_bits(g.read::<u64>(w(0x6d)));
    // cvtps2pd, subsd, cvtpd2ps
    let y = (f64::from(y) - off) as f32;
    g.write::<u32>(v + 4, y.to_bits());
    let r = g.rand();
    let z = sub(mul(div(cvt(r), k(g, 0x8a)), k(g, 0x93)), k(g, 0xa1));
    g.write::<u32>(v + 8, z.to_bits());
    let mut p = Pushes::default();
    p.push_all(&[v, v]);
    call(g, &p, 232);
    let fl = |g: &G, at: u32| f32::from_bits(g.read::<u32>(at));
    let ek = k(g, 0xb6);
    let e = [
        mul(fl(g, v), ek),
        mul(fl(g, v + 4), ek),
        mul(fl(g, v + 8), ek),
    ];
    let r = g.rand();
    let speed = add(mul(div(cvt(r), k(g, 0xf6)), k(g, 0x103)), k(g, 0x10b));
    let vx = mul(fl(g, v), speed);
    let vz = mul(mul(fl(g, v + 8), speed), k(g, 0x125));
    let vy = mul(mul(fl(g, v + 4), speed), k(g, 0x136));
    let r = g.rand();
    let size = add(mul(div(cvt(r), k(g, 0x151)), k(g, 0x163)), k(g, 0x173));
    let flags = g.read::<u32>(obj.wrapping_add(0xbc)) | 0x2000;
    g.write::<u32>(obj.wrapping_add(0xbc), flags);
    p.push_all(&[0, actor, obj]);
    call(g, &p, 442);
    for (off, x) in [
        (0xd0u32, vx),
        (0xd4, vy),
        (0xd8, vz),
        (0xe0, e[0]),
        (0xe4, e[1]),
        (0xe8, e[2]),
    ] {
        g.write::<u32>(obj.wrapping_add(off), x.to_bits());
    }
    p.push_all(&[1, 0, obj, g.read::<u32>(w(0x1df))]);
    for (off, x) in [
        (0xdcu32, 0u32),
        (0xec, 0),
        (0x100, 0x42b4_0000),
        (0x104, 0),
        (0x108, 0),
        (0x10c, 0),
        (0x130, size.to_bits()),
        (0x134, size.to_bits()),
        (0x138, size.to_bits()),
        (0x13c, 0),
    ] {
        g.write::<u32>(obj.wrapping_add(off), x);
    }
    call(g, &p, 440);
    p.push_all(&[w(0x24f), obj]);
    call(g, &p, 450);
    p.push_all(&[w(0x262), obj]);
    Ret::eax(call(g, &p, 449))
}

/// [`spark`] (643 bytes, 1 overlay).
pub const SPARK: Sig = Sig {
    len: 643,
    hash: 0x6b8e_420d_3f76_982c,
    // __security_check_cookie
    calls: &[(0x27b, Callee::Any)],
    imports: &[(0x1c, "rand")],
};
