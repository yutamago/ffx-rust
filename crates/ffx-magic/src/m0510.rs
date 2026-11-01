//! `magic_0510.dll`'s own functions (besides the runtime and the shared
//! code): the steps of a battle sequence. Each step is a callback the
//! game calls with its sequence object `seq`; it calls the sequence
//! slots (434, 449, 450, 451, ...) and names the next step. `S = *G_S`
//! (operand `+0x04`/`+0x05`/`+0x0b`) is the battle's state block (`+0x198`
//! the actor, `+0x1a0` the target, `+0x4` a handle).

use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// The sequence's first step (0x10001000, `(seq)`, pushed by
/// 0x10002005): thirteen state words 0; slot 367 `(S[4], target, 0, 0,
/// &l, 0, 0, 0)` fills the local `l` (`ebp - 4`), whose bit 8 goes to
/// the flag at operand `+0xbc`; slot 523 `(0, flag)`, slot 481
/// `(target)` a handle `h` (`+0xe8`) and slot 14 `(h, &A, &B, &C)` its
/// three words; slot 448 `(1, 0)`; the flag at `+0x13d` says whether
/// slot 520 `()` answers 0x2050000; slot 450 `(seq, NEXT)` and 449
/// `(seq, END)` (the earlier words left on the stack). Returns slot
/// 449's `eax`.
pub fn e1000<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let l = fr.esp.wrapping_sub(8);
    let s = g.read::<u32>(w(0x5));
    let target = g.read::<u32>(s + 0x1a0);
    for op in [
        0x20, 0x2a, 0x34, 0x3e, 0x48, 0x52, 0x5c, 0x66, 0x70, 0x7a, 0x84, 0x8e,
    ] {
        g.write::<u32>(w(op), 0);
    }
    let a = g.read::<u32>(s + 4);
    let t = slot_target(g, w(0x9b), 367);
    g.write::<u32>(w(0xa1), 0);
    let mut stack = vec![a, target, 0, 0, l, 0, 0, 0];
    g.call(t, 0, &stack);
    let flag = (g.read::<u32>(l).cast_signed() >> 8).cast_unsigned() & 1;
    g.write::<u32>(w(0xbc), flag);
    let t = slot_target(g, w(0xc1), 523);
    stack.splice(0..0, [0, flag]);
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xd0), 481);
    stack.insert(0, target);
    // call width: 17
    let h = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0xe8), h);
    let t = slot_target(g, w(0xf3), 14);
    stack.splice(0..0, [h, w(0xed), w(0xe3), w(0xde)]);
    // call width: 17
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xfd), 448);
    stack.splice(0..0, [1, 0]);
    // call width: 17
    g.call(t, 0, &stack);
    // add esp, 0x44
    let t = slot_target(g, w(0x10e), 520);
    let r = g.call(t, 0, &[]).eax;
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0x128), 450);
    g.write::<u32>(w(0x13d), u32::from(r == 0x0205_0000));
    let mut stack = vec![seq, w(0x136)];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x144), 449);
    stack.splice(0..0, [seq, w(0x149)]);
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x10001000 (350 bytes, 1 overlay).
pub const E1000: Sig = Sig {
    len: 350,
    hash: 0x7408_f7a3_4e7c_8a40,
    calls: &[],
    imports: &[],
};

/// A step (0x100011b0, `(seq)`, pushed by 0x10001178): slot 444 `(0)`,
/// slot 522 `(0, 0)`; slot 434 `(*G_S, CB)` with `CB` the callback at
/// operand `+0x47` when the flag at `+0x39` is set, else `+0x4e`; unless
/// the flag at `+0x5f` is set, slot 486 `(actor, 2, 0, 0, 20, 32, 32,
/// 15.0, 0, 0, 0)`; slot 447 `(actor, 200)`, slot 450 `(seq, NEXT)` and
/// 451 `(seq, 0x13)` (the earlier words left on the stack). Returns
/// slot 451's `eax`.
pub fn e11b0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = g.read::<u32>(w(0x4));
    let actor = g.read::<u32>(s + 0x198);
    let t = slot_target(g, w(0x10), 444);
    g.call(t, 0, &[0]);
    let t = slot_target(g, w(0x1f), 522);
    g.call(t, 0, &[0, 0, 0]);
    let t = slot_target(g, w(0x30), 434);
    let cb = if g.read::<u32>(w(0x39)) != 0 {
        w(0x47)
    } else {
        w(0x4e)
    };
    let gs = g.read::<u32>(w(0x54));
    g.call(t, 0, &[gs, cb]);
    if g.read::<u32>(w(0x5f)) == 0 {
        let t = slot_target(g, w(0x67), 486);
        g.call(
            t,
            0,
            &[actor, 2, 0, 0, 0x14, 0x20, 0x20, 0x4170_0000, 0, 0, 0],
        );
    }
    let t = slot_target(g, w(0xac), 447);
    let mut stack = vec![actor, 0xc8];
    g.call(t, 0, &stack);
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0xbf), 450);
    stack.splice(0..0, [seq, w(0xcd)]);
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xd5), 451);
    stack.splice(0..0, [seq, 0x13]);
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x100011b0 (234 bytes, 1 overlay).
pub const E11B0: Sig = Sig {
    len: 234,
    hash: 0x9e9b_9021_4e00_2ec8,
    calls: &[],
    imports: &[],
};

/// A step (0x100012a0, `(seq)`, pushed by 0x1000127d): slot 522 `(0,
/// 1)`, three slot 434 `(*G_S, CB)` callbacks (operands `+0x1a`,
/// `+0x32`, `+0x4a`), slot 450 `(seq, NEXT)` and 451 `(seq, 0x18)` (the
/// earlier words left on the stack). Returns slot 451's `eax`.
pub fn e12a0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x4), 522);
    let mut stack = vec![0, 1];
    g.call(t, 0, &stack);
    for (tab, cb, gs) in [
        (0x15u32, 0x1au32, 0x20u32),
        (0x2d, 0x32, 0x38),
        (0x45, 0x4a, 0x50),
    ] {
        let t = slot_target(g, w(tab), 434);
        let s = g.read::<u32>(w(gs));
        stack.splice(0..0, [s, w(cb)]);
        g.call(t, 0, &stack);
    }
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0x5d), 450);
    stack.splice(0..0, [seq, w(0x62)]);
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x72), 451);
    stack.splice(0..0, [seq, 0x18]);
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x100012a0 (136 bytes, 1 overlay).
pub const E12A0: Sig = Sig {
    len: 136,
    hash: 0x13e3_5c0d_379f_2013,
    calls: &[],
    imports: &[],
};

/// A step (0x10001380, `(seq)`, pushed by 0x10001349): unless the flag
/// at operand `+0x05` is set, slot 486 `(actor, 1, 0, 0, 10, 32, 32, 0,
/// 1.0, 0.4, 0)`; slot 447 `(actor, 201)`, slot 450 `(seq, NEXT)` and
/// 449 `(seq, END)` (the earlier words left on the stack); the actor
/// goes to the sequence's entry `seq[0x2c + 16 * seq[0xac]]`. Returns
/// twice `seq[0xac]`.
pub fn e1380<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let flag = g.read::<u32>(w(0x5));
    let s = g.read::<u32>(w(0xb));
    let actor = g.read::<u32>(s + 0x198);
    if flag == 0 {
        let t = slot_target(g, w(0x1a), 486);
        g.call(
            t,
            0,
            &[actor, 1, 0, 0, 0xa, 0x20, 0x20, 0, ONE, 0x3ecc_cccd, 0],
        );
    }
    let t = slot_target(g, w(0x5f), 447);
    let mut stack = vec![actor, 0xc9];
    g.call(t, 0, &stack);
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0x72), 450);
    stack.splice(0..0, [seq, w(0x80)]);
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x88), 449);
    stack.splice(0..0, [seq, w(0x8d)]);
    g.call(t, 0, &stack);
    let n = g
        .read::<u32>(seq + 0xac)
        .wrapping_add(g.read::<u32>(seq + 0xac));
    g.write::<u32>(
        seq.wrapping_add(n.wrapping_mul(8)).wrapping_add(0x2c),
        actor,
    );
    Ret::eax(n)
}

/// 0x10001380 (173 bytes, 1 overlay).
pub const E1380: Sig = Sig {
    len: 173,
    hash: 0x47a8_49b9_b2c5_abf5,
    calls: &[],
    imports: &[],
};

/// A step (0x10001680, `(seq)`, pushed by 0x100012ea): slot 442 `(seq,
/// actor, 0)`; the sequence's point `+0xd0` `(0, -12, -30, 0)` and
/// scales `+0x130` `(0.1, 0.1, 0.1, 0)`; slot 450 `(seq, NEXT)` and 451
/// `(seq, 0x17)` (the earlier words left on the stack). Returns slot
/// 451's `eax`.
pub fn e1680<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = g.read::<u32>(w(0x4));
    let seq = fr.arg(g, 0);
    let actor = g.read::<u32>(s + 0x198);
    let t = slot_target(g, w(0x15), 442);
    let mut stack = vec![seq, actor, 0];
    g.call(t, 0, &stack);
    for (off, v) in [
        (0xd0u32, 0u32),
        (0xd4, 0xc140_0000),
        (0xd8, 0xc1f0_0000),
        (0xdc, 0),
        (0x130, 0x3dcc_cccd),
        (0x134, 0x3dcc_cccd),
        (0x138, 0x3dcc_cccd),
        (0x13c, 0),
    ] {
        g.write::<u32>(seq + off, v);
    }
    let t = slot_target(g, w(0x23), 450);
    stack.splice(0..0, [seq, w(0x7e)]);
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x86), 451);
    stack.splice(0..0, [seq, 0x17]);
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x10001680 (155 bytes, 1 overlay).
pub const E1680: Sig = Sig {
    len: 155,
    hash: 0x9a76_0f39_9a34_bf91,
    calls: &[],
    imports: &[],
};

/// The camera's drift (both copies, the same operands): with the old
/// values read first, `P += V` for the point `P` (operands `+0x90`/`+0xc0`
/// and on: `0x10077b48..`), `V += D` for the velocity (`0x10077b94..`,
/// `D` = `0x10077b58..`), and `D = A + D` for the drift (`A` =
/// `0x10077b84..`), per axis (`addss` in the original's operand order).
fn drift<G: Game>(g: &mut G, img: &Image, f: u32) {
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, op: u32| g.read::<f32>(w(op));
    let p0 = add(r(g, 0x90), r(g, 0x98));
    let d = [r(g, 0xa0), r(g, 0xa8), r(g, 0xb0), r(g, 0xb8)];
    g.write::<f32>(w(0xc0), p0);
    let p1 = add(r(g, 0xc8), r(g, 0xd0));
    g.write::<f32>(w(0xe5), p1);
    let p2 = add(r(g, 0xed), r(g, 0xf5));
    g.write::<f32>(w(0xfd), p2);
    let p3 = add(r(g, 0x105), r(g, 0x10d));
    g.write::<f32>(w(0x115), p3);
    for (k, (src, dst)) in [
        (0x120u32, 0x128u32),
        (0x133, 0x13b),
        (0x146, 0x14e),
        (0x159, 0x161),
    ]
    .into_iter()
    .enumerate()
    {
        let v = add(d[k], r(g, src));
        g.write::<f32>(w(dst), v);
    }
    for (k, (src, dst)) in [
        (0x169u32, 0x175u32),
        (0x17d, 0x189),
        (0x191, 0x19d),
        (0x1a5, 0x1b1),
    ]
    .into_iter()
    .enumerate()
    {
        let v = add(r(g, src), d[k]);
        g.write::<f32>(w(dst), v);
    }
}

/// A step (0x10001b10, `(seq)`, pushed by 0x10001aec; a cookie frame):
/// for the target `t = S[0x1a0]`, slot 480 `(t, 6)` a bone `b`, slot 477
/// `(t)` a height `h`, the local point `(0, h, c * K1 / K2 + K3)` (`c` the
/// step count at operand `+0x50`; `ebp - 0x14`) and slot 479 `(t, b, CAM,
/// &p)`; the camera drifts ([`drift`]) and slot 484 `(t, CAM, 0)`; at
/// step 5 slot 485 `(255, 255, 255, 64, 0, 0, 0, 0, 16)`. The count goes
/// up; past 30 slot 446 `(t, 0)`, 483 `(t)` and 450 `(seq, NEXT)` (the
/// earlier words left on the stack). Returns the last call's `eax`, or
/// the count.
pub fn e1b10<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = fr.esp.wrapping_sub(0x18);
    let s = g.read::<u32>(w(0x11));
    let seq = fr.arg(g, 0);
    let target = g.read::<u32>(s + 0x1a0);
    let t = slot_target(g, w(0x22), 480);
    let bone = g.call(t, 0, &[target, 6]).eax;
    let t = slot_target(g, w(0x33), 477);
    g.write::<u32>(p, 0);
    let h = crate::ob_model::st0_f32(g.call(t, 0, &[target, target, 6]).st0);
    g.write::<f32>(p + 4, h);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let z = cvt(g.read::<u32>(w(0x50)));
    let z = add(div(mul(z, k(g, 0x5e)), k(g, 0x6c)), k(g, 0x79));
    g.write::<f32>(p + 8, z);
    let t = slot_target(g, w(0x64), 479);
    let mut stack = vec![target, bone, w(0x71), p, target, target, 6];
    // call width: 7
    g.call(t, 0, &stack);
    drift(g, img, f);
    let t = slot_target(g, w(0xd5), 484);
    stack.splice(0..0, [target, w(0xdc), 0]);
    // call width: 10
    g.call(t, 0, &stack);
    if g.read::<u32>(w(0x1be)) == 5 {
        let t = slot_target(g, w(0x1cb), 485);
        g.call(t, 0, &[0xff, 0xff, 0xff, 0x40, 0, 0, 0, 0, 0x10]);
    }
    let n = g.read::<u32>(w(0x1f3)).wrapping_add(1);
    g.write::<u32>(w(0x1fc), n);
    if n.cast_signed() <= 0x1e {
        return Ret::eax(n);
    }
    let t = slot_target(g, w(0x206), 446);
    g.call(t, 0, &[target, 0]);
    let t = slot_target(g, w(0x216), 483);
    g.call(t, 0, &[target, target, 0]);
    let t = slot_target(g, w(0x224), 450);
    Ret::eax(g.call(t, 0, &[seq, w(0x229), target, target, 0]).eax)
}

/// 0x10001b10 (586 bytes, 1 overlay).
pub const E1B10: Sig = Sig {
    len: 586,
    hash: 0x4417_eec9_f47a_cd5d,
    calls: &[
        // __security_check_cookie
        (0x242, Callee::Any),
    ],
    imports: &[],
};

/// A step (0x10001db0, `(seq)`, pushed by 0x10001d69; a cookie frame):
/// for the target `t = S[0x1a0]`, slot 480 `(t, 6)` a bone `b`; on the
/// first step slot 388 `(t, 5, 1)`; slot 477 `(t)` a height `h`, the local
/// point `(0, h, 0)` (`ebp - 0x14`) and slot 479 `(t, b, CAM, &p)`; the
/// camera drifts ([`drift`]) and slot 484 `(t, CAM, 0)` (the earlier
/// words left on the stack). The count goes up; past 20 slot 446 `(t,
/// 0)`, 483 `(t)` and the sequence's flag bit 2. Returns slot 483's
/// `eax`, or the count.
pub fn e1db0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = fr.esp.wrapping_sub(0x18);
    let s = g.read::<u32>(w(0x11));
    let target = g.read::<u32>(s + 0x1a0);
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0x1e), 480);
    let bone = g.call(t, 0, &[target, 6]).eax;
    if g.read::<u32>(w(0x36)) == 0 {
        let t = slot_target(g, w(0x41), 388);
        g.call(t, 0, &[target, 5, 1]);
    }
    let t = slot_target(g, w(0x56), 477);
    g.write::<u32>(p, 0);
    let h = crate::ob_model::st0_f32(g.call(t, 0, &[target]).st0);
    g.write::<f32>(p + 4, h);
    g.write::<u32>(p + 8, 0);
    let t = slot_target(g, w(0x72), 479);
    let mut stack = vec![target, bone, w(0x77), p, target];
    // call width: 5
    g.call(t, 0, &stack);
    drift(g, img, f);
    let t = slot_target(g, w(0xd5), 484);
    stack.splice(0..0, [target, w(0xdc), 0]);
    // call width: 8
    g.call(t, 0, &stack);
    let n = g.read::<u32>(w(0x1be)).wrapping_add(1);
    g.write::<u32>(w(0x1c7), n);
    if n.cast_signed() <= 0x14 {
        return Ret::eax(n);
    }
    let t = slot_target(g, w(0x1d1), 446);
    g.call(t, 0, &[target, 0]);
    let t = slot_target(g, w(0x1e1), 483);
    let r = g.call(t, 0, &[target, target, 0]).eax;
    let fl = g.read::<u32>(seq);
    g.write::<u32>(seq, fl | 4);
    Ret::eax(r)
}

/// 0x10001db0 (517 bytes, 1 overlay).
pub const E1DB0: Sig = Sig {
    len: 517,
    hash: 0xd8b8_476d_3c25_1727,
    calls: &[
        // __security_check_cookie
        (0x1fd, Callee::Any),
    ],
    imports: &[],
};
