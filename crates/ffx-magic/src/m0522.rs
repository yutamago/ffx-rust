//! `magic_0522.dll`'s own functions (besides the runtime and the shared
//! code): steps of a battle sequence (as `magic_0510`'s: each a callback
//! with the sequence object `seq`, `S = *G_S` the battle's state block).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Libm, Ret, Sig};

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

/// A spark's start at a bone (0x10001480, a callback, cdecl `(obj)`; a
/// /GS frame): from `rand()` values (each `r / R * K..`): a spin `w`
/// (`ebp - 0x28`), a size `s` (`- 0x24`); the bone 0x0f for an odd draw,
/// else 0x1a, whose point slot 456 `(actor, bone, &p)` puts at `ebp -
/// 0x14` (the actor `(*G_A)[0x198]`); `p.y` raised by a draw; an angle
/// `a` and a radius `d`, with `(cos a * d, h, sin a * d)` (MSVCR110's
/// `_libm_sse2_cos/sin_precise`) the velocity, `h` a draw. The object's
/// flags `+0xbc |= 0x4000`; `+0xd0..` = `p`, `+0xe0..` the velocity,
/// `+0x108` = `w`, the scale `+0x130..` = `s`, `+0x140..` 0.02, `+0x18c`
/// -0.045, the rest of those rows 0; slot 458 `(*G_A, obj, 0)`, slot 450
/// `(obj, CB)` and slot 451 `(obj, 0x14)` (each seeing the earlier
/// words). Returns slot 451's `eax`.
pub fn bone_spark<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, div, mul};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, off: u32| f32::from_bits(g.read::<u32>(w(off)));
    let fl = |g: &G, at: u32| f32::from_bits(g.read::<u32>(at));
    let call = |g: &mut G, p: &Pushes, slot: u32| {
        let t = slot_target(g, w(0x89), slot);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0).eax
    };
    // a draw: `rand() / R * K`
    let draw = |g: &mut G, r: u32, m: u32| {
        let x = g.rand();
        mul(div(cvt(x), k(g, r)), k(g, m))
    };
    let obj = fr.arg(g, 0);
    let actor = g.read::<u32>(g.read::<u32>(w(0x11)).wrapping_add(0x198));
    let spin = draw(g, 0x34, 0x3c);
    let size = add(draw(g, 0x52, 0x5a), k(g, 0x62));
    let bone = if g.rand().is_multiple_of(2) {
        0x1a
    } else {
        0x0f
    };
    let p = fr.esp.wrapping_sub(0x18);
    let mut st = Pushes::default();
    st.push_all(&[p, bone, actor]);
    call(g, &st, 456);
    let up = draw(g, 0xa3, 0xab);
    g.write::<u32>(p + 4, add(up, fl(g, p + 4)).to_bits());
    let angle = draw(g, 0xc6, 0xce);
    let radius = draw(g, 0xe4, 0xec);
    let vx = mul(g.libm(Libm::Cos, f64::from(angle)) as f32, radius);
    let vy = draw(g, 0x11d, 0x125);
    let s_angle = g.libm(Libm::Sin, f64::from(angle)) as f32;
    let flags = g.read::<u32>(obj.wrapping_add(0xbc)) | 0x4000;
    g.write::<u32>(obj.wrapping_add(0xbc), flags);
    let vz = mul(s_angle, radius);
    for i in 0..3 {
        let v = g.read::<u32>(p + 4 * i);
        g.write::<u32>(obj.wrapping_add(0xd0 + 4 * i), v);
    }
    for (off, x) in [
        (0xe0u32, vx.to_bits()),
        (0xe4, vy.to_bits()),
        (0x108, spin.to_bits()),
        (0xdc, 0),
        (0xe8, vz.to_bits()),
        (0xec, 0),
        (0x100, 0),
        (0x104, 0),
        (0x10c, 0),
        (0x130, size.to_bits()),
        (0x134, size.to_bits()),
        (0x138, size.to_bits()),
        (0x13c, 0),
        (0x140, 0x3ca3_d70a),
        (0x144, 0x3ca3_d70a),
        (0x148, 0x3ca3_d70a),
        (0x14c, 0),
        (0x180, 0),
        (0x184, 0),
        (0x188, 0),
        (0x18c, 0xbd38_51ec),
    ] {
        g.write::<u32>(obj.wrapping_add(off), x);
    }
    st.push_all(&[0, obj, g.read::<u32>(w(0x25a))]);
    call(g, &st, 458);
    st.push_all(&[w(0x26c), obj]);
    call(g, &st, 450);
    st.push_all(&[0x14, obj]);
    Ret::eax(call(g, &st, 451))
}

/// [`bone_spark`] (669 bytes, 1 overlay).
pub const BONE_SPARK: Sig = Sig {
    len: 669,
    hash: 0xc5b1_b988_fdcb_fcdf,
    calls: &[
        (0xfe, Callee::Import("_libm_sse2_cos_precise")),
        (0x137, Callee::Import("_libm_sse2_sin_precise")),
        // __security_check_cookie
        (0x295, Callee::Any),
    ],
    imports: &[(0x23, "rand")],
};
