//! `magic_0526.dll`'s own functions (besides the runtime and the shared
//! code): steps of a battle sequence (as `magic_0510`'s: each a callback
//! with the sequence object `seq`, `S = *G_S` the battle's state block).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

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

/// A spark aimed between two bones (0x10001450, a callback, cdecl
/// `(obj)`; a /GS frame): slot 444 `(0)`; the object's bone index `s` (its
/// record `obj + 16 * obj[0xac] + 0x2c`) and the part `P` = `(*G_A)[0x1a0 +
/// 4 s]`; slot 434 `(*G_A, CB)` gives a new object, whose record gets
/// `s`; slot 456 `(actor, 0x31, &a)` and slot 457 `(P, 6, &b)` two
/// points (the actor `(*G_A)[0x198]`; locals `a` at `ebp - 0x24`, `b` at
/// `ebp - 0x34`), `d = b - a` (`ebp - 0x14`, four lanes) normalised by
/// slot 453 `(&d)` (its ST0 dropped); `obj + 0xd0..` = `a`, `+0xe0..` =
/// `d * K` (operand `+0xe5`), `+0xdc`/`+0xec` 0, the scale `+0x130..` 0.2
/// (`+0x13c` 0); slot 440 `(*G_A, obj, 0, 2)`, slot 450 `(obj, CB2)` (each
/// call seeing the earlier words), then slot 451 `(obj, 8)`. Returns slot
/// 451's `eax`.
pub fn aim_spark<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{mul, sub};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let call = |g: &mut G, p: &Pushes, slot: u32| {
        let t = slot_target(g, w(0x1d), slot);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0)
    };
    let fl = |g: &G, at: u32| f32::from_bits(g.read::<u32>(at));
    let ebp = fr.esp.wrapping_sub(4);
    let (a, b, d) = (ebp - 0x24, ebp - 0x34, ebp - 0x14);
    let actor = g.read::<u32>(g.read::<u32>(w(0x11)).wrapping_add(0x198));
    let obj = fr.arg(g, 0);
    let mut p = Pushes::default();
    p.push(0);
    call(g, &p, 444);
    let i = g.read::<u32>(obj.wrapping_add(0xac));
    let s = g.read::<u32>(obj.wrapping_add(i.wrapping_mul(16)).wrapping_add(0x2c));
    p.push(w(0x3c));
    let ga = g.read::<u32>(w(0x45));
    p.push(ga);
    let part = g.read::<u32>(ga.wrapping_add(s.wrapping_mul(4)).wrapping_add(0x1a0));
    let new = call(g, &p, 434).eax;
    let c = g.read::<u32>(new.wrapping_add(0xac));
    g.write::<u32>(new.wrapping_add(c.wrapping_mul(16)).wrapping_add(0x2c), s);
    p.push_all(&[a, 0x31, actor]);
    call(g, &p, 456);
    p.push_all(&[b, 6, part]);
    call(g, &p, 457);
    for i in 0..4 {
        let v = sub(fl(g, b + 4 * i), fl(g, a + 4 * i));
        g.write::<u32>(d + 4 * i, v.to_bits());
    }
    p.push(d);
    call(g, &p, 453);
    let k = fl(g, w(0xe5));
    let dir: [f32; 4] = [0u32, 4, 8, 12].map(|o| mul(fl(g, d + o), k));
    for (i, x) in (0u32..).zip(dir) {
        g.write::<u32>(d + 4 * i, x.to_bits());
    }
    for i in 0..3 {
        let v = g.read::<u32>(a + 4 * i);
        g.write::<u32>(obj.wrapping_add(0xd0 + 4 * i), v);
    }
    g.write::<u32>(obj.wrapping_add(0xdc), 0);
    for (i, x) in (0u32..).zip(&dir[..3]) {
        g.write::<u32>(obj.wrapping_add(0xe0 + 4 * i), x.to_bits());
    }
    g.write::<u32>(obj.wrapping_add(0xec), 0);
    p.push_all(&[2, 0, obj, g.read::<u32>(w(0x183))]);
    for off in [0x130u32, 0x134, 0x138] {
        g.write::<u32>(obj.wrapping_add(off), 0x3e4c_cccd);
    }
    g.write::<u32>(obj.wrapping_add(0x13c), 0);
    call(g, &p, 440);
    p.push_all(&[w(0x1bd), obj]);
    call(g, &p, 450);
    let mut p = Pushes::default();
    p.push_all(&[8, obj]);
    Ret::eax(call(g, &p, 451).eax)
}

/// [`aim_spark`] (497 bytes, 1 overlay).
pub const AIM_SPARK: Sig = Sig {
    len: 497,
    hash: 0x1efc_53b2_554a_374b,
    // __security_check_cookie
    calls: &[(0x1e9, Callee::Any)],
    imports: &[],
};
