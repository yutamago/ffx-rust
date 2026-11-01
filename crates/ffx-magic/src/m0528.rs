//! `magic_0528.dll`'s own functions (besides the runtime and the shared
//! code): steps of a battle sequence (as `magic_0510`'s: each a callback
//! with the sequence object `seq`, `S = *G_S` the battle's state block).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

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

/// The effect's per-frame step (0x10001120, cdecl `(arg)`): the actor
/// `c` = slot 481 `(*(*G_A + 0x198))`; the frame `n = *G_N` (operand
/// `+0x1f`). Every 8th frame below 20 (`n % 8`, C's signed remainder),
/// every 5th below 70, every 3rd after, slot 434 `(*G_A, SPARK)` adds a
/// spark. Below frame 100 the frame counts and the `eax` the code left is
/// returned (the spark's answer, `c` below frame 20, else the remainder's
/// multiple). At frame 100 slot 434 `(*G_A, FLASH)`. Below 120 slot 21
/// `(c, 1, K - (n - 100) / D)` fades the actor; at 120 and after slot 278
/// `(c, 1, 1)`, then slot 450 `(arg, CB1)` and slot 449 `(arg, CB2)` (each
/// call seeing the earlier words). The frame counts; returns the last
/// slot's `eax`.
pub fn step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, off: u32| g.read::<u32>(w(off));
    let tbl = 0x11;
    let call = |g: &mut G, p: &Pushes, slot: u32| {
        let t = slot_target(g, w(tbl), slot);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0).eax
    };
    let a = r(g, 0x4);
    let mut p = Pushes::default();
    p.push(g.read::<u32>(a.wrapping_add(0x198)));
    let actor = call(g, &p, 481);
    let mut n = r(g, 0x1f).cast_signed();
    let (rem, mut eax) = if n < 0x14 {
        (n % 8, actor)
    } else if n < 0x46 {
        (n % 5, (n / 5 * 5).cast_unsigned())
    } else {
        (n % 3, (n / 3 * 3).cast_unsigned())
    };
    if rem == 0 {
        let mut p = Pushes::default();
        p.push_all(&[w(0x73), r(g, 0x79)]);
        eax = call(g, &p, 434);
        n = r(g, 0x87).cast_signed();
    }
    if n < 0x64 {
        g.write::<u32>(w(0x149), n.wrapping_add(1).cast_unsigned());
        return Ret::eax(eax);
    }
    if n == 0x64 {
        let mut p = Pushes::default();
        p.push_all(&[w(0x9f), r(g, 0xa5)]);
        call(g, &p, 434);
        n = r(g, 0xb3).cast_signed();
    }
    let eax = if n < 0x78 {
        let t = div(
            cvt(n.wrapping_sub(0x64).cast_unsigned()),
            f32::from_bits(r(g, 0xd5)),
        );
        let x = sub(f32::from_bits(r(g, 0xcd)), t);
        let mut p = Pushes::default();
        p.push_all(&[x.to_bits(), 1, actor]);
        call(g, &p, 21)
    } else {
        let arg = fr.arg(g, 0);
        let mut p = Pushes::default();
        p.push_all(&[1, 1, actor]);
        call(g, &p, 278);
        p.push_all(&[w(0x11e), arg]);
        call(g, &p, 450);
        p.push_all(&[w(0x12b), arg]);
        call(g, &p, 449)
    };
    let at = w(0xf5);
    let v = g.read::<u32>(at).wrapping_add(1);
    g.write::<u32>(at, v);
    Ret::eax(eax)
}

/// [`step`] (336 bytes, 1 overlay).
pub const STEP: Sig = Sig {
    len: 336,
    hash: 0x42e4_fe79_7d5b_5226,
    calls: &[],
    imports: &[],
};

/// A spark's start (0x100012a0, a callback, cdecl `(obj)`; a /GS frame):
/// a direction of three `rand()` values `r / R * S - T` (operands
/// `+0x34`, `+0x3c`, `+0x44`) in the local at `ebp - 0x14`, normalised by
/// slot 232 `(&v, &v)`, a speed `rand() / R * U + V` (`+0xca`, `+0xdc`);
/// the object's flags `+0xbc |= 0x2000`, slot 441 `(obj, *(*G_A + 0x198),
/// 0xeb)`; then `+0xd0..` the direction times the speed, `+0xe0..` times
/// `W` (`+0x11a`), `+0xdc`/`+0xec` 0, `+0x100` 90.0, `+0x104..+0x10c` 0,
/// the scale `+0x130..` 0.75 (`+0x13c` 0); slot 440 `(*G_A, obj, 0, 0)`,
/// slot 450 `(obj, CB1)`, slot 449 `(obj, CB2)` (each call seeing the
/// earlier words). Returns slot 449's `eax`.
pub fn spark<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, off: u32| f32::from_bits(g.read::<u32>(w(off)));
    let tbl = 0x92;
    let call = |g: &mut G, p: &Pushes, slot: u32| {
        let t = slot_target(g, w(tbl), slot);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0).eax
    };
    let obj = fr.arg(g, 0);
    let a = g.read::<u32>(w(0x11));
    let actor = g.read::<u32>(a.wrapping_add(0x198));
    let v = fr.esp.wrapping_sub(0x18);
    for (i, ops) in [
        (0x34u32, 0x3cu32, 0x44u32),
        (0x5a, 0x62, 0x6a),
        (0x83, 0x8c, 0x9a),
    ]
    .into_iter()
    .enumerate()
    {
        let r = cvt(g.rand());
        let x = sub(mul(div(r, k(g, ops.0)), k(g, ops.1)), k(g, ops.2));
        g.write::<u32>(v + 4 * i as u32, x.to_bits());
    }
    let mut p = Pushes::default();
    p.push_all(&[v, v]);
    call(g, &p, 232);
    let r = cvt(g.rand());
    let speed = add(mul(div(r, k(g, 0xbd)), k(g, 0xca)), k(g, 0xdc));
    let dir = [0u32, 4, 8].map(|o| f32::from_bits(g.read::<u32>(v + o)));
    let flags = g.read::<u32>(obj.wrapping_add(0xbc)) | 0x2000;
    g.write::<u32>(obj.wrapping_add(0xbc), flags);
    p.push_all(&[0xeb, actor, obj]);
    call(g, &p, 441);
    let wk = k(g, 0x11a);
    for (i, d) in (0u32..).zip(dir) {
        g.write::<u32>(obj.wrapping_add(0xd0 + 4 * i), mul(d, speed).to_bits());
        g.write::<u32>(obj.wrapping_add(0xe0 + 4 * i), mul(d, wk).to_bits());
    }
    for (off, x) in [
        (0xdcu32, 0u32),
        (0xec, 0),
        (0x100, 0x42b4_0000),
        (0x104, 0),
        (0x108, 0),
        (0x10c, 0),
        (0x130, 0x3f40_0000),
        (0x134, 0x3f40_0000),
        (0x138, 0x3f40_0000),
        (0x13c, 0),
    ] {
        g.write::<u32>(obj.wrapping_add(off), x);
    }
    p.push_all(&[0, 0, obj, g.read::<u32>(w(0x1b8))]);
    call(g, &p, 440);
    p.push_all(&[w(0x210), obj]);
    call(g, &p, 450);
    p.push_all(&[w(0x223), obj]);
    Ret::eax(call(g, &p, 449))
}

/// [`spark`] (580 bytes, 1 overlay).
pub const SPARK: Sig = Sig {
    len: 580,
    hash: 0x2c24_4b0f_59f1_2724,
    // __security_check_cookie
    calls: &[(0x23c, Callee::Any)],
    imports: &[(0x1c, "rand")],
};
