//! `magic_0523.dll`'s own functions (besides the runtime and the shared
//! code): the object callbacks and the light of its table.

use crate::part_draw::start_part;
use crate::runtime::slot_target;
use crate::sb_sse::{cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;
/// The light's point (`x`, `y`, `z`).
const LIGHT: [u32; 3] = [0x419f_32d4, 0xc143_32df, 0x4245_9968];

/// A random value: slot 324's answer's low 15 bits `r` over `D` times
/// `M` (the operands of `D` and `M`). Returns it and `r`.
fn random<G: Game>(g: &mut G, img: &Image, f: u32, t: u32, d: u32, m: u32) -> (f32, u32) {
    let w = |off: u32| img.word(f + off);
    let s = slot_target(g, w(t), 324);
    let r = g.call(s, 0, &[]).eax & 0x7fff;
    (
        mul(div(cvt(r), g.read::<f32>(w(d))), g.read::<f32>(w(m))),
        r,
    )
}

/// Places the object `obj` with slot 237 `(*G_H, T[k], 0, obj + 0x40)`,
/// `k` its record index in the array at the operand `base` (`sar 7`, the
/// table `T` at `table`). Returns the words left on the stack.
fn place<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    obj: u32,
    (base, table, t, h): (u32, u32, u32, u32),
) -> [u32; 4] {
    let w = |off: u32| img.word(f + off);
    let k = obj.wrapping_sub(w(base)).cast_signed() >> 7;
    let x = g.read::<u32>(w(table).wrapping_add(k.cast_unsigned().wrapping_mul(4)));
    let s = slot_target(g, w(t), 237);
    let args = [g.read::<u32>(w(h)), x, 0, obj + 0x40];
    g.call(s, 0, &args);
    args
}

/// An object's start (0x10001000, `(obj)`): program 0 started
/// ([`start_part`]) and the object placed ([`place`]; the start's words
/// left on the stack); `+0x50..` 0, `+0x5c` 1, the scales 0.0132
/// (`+0x6c` 1.0); then the table of random points from operand `+0x9f`
/// up to `+0x138` (16 bytes each, `w` 1.0) filled from slot 324. Returns
/// the last draw's low 15 bits.
pub fn e1000<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let below = start_part(g, img, f, obj, [0x4, 0xb, 0x10, 0x18, 0x35], 0, &[]);
    let k = obj.wrapping_sub(w(0x40)).cast_signed() >> 7;
    let x = g.read::<u32>(w(0x4c).wrapping_add(k.cast_unsigned().wrapping_mul(4)));
    let s = slot_target(g, w(0x51), 237);
    let mut args = vec![g.read::<u32>(w(0x57)), x, 0, obj + 0x40];
    args.extend(below);
    // call width: 8
    g.call(s, 0, &args);
    for (off, v) in [
        (0x50u32, 0u32),
        (0x54, 0),
        (0x58, 0),
        (0x5c, 1),
        (0x60, 0x3c58_383b),
        (0x64, 0x3c58_383b),
        (0x68, 0x3c58_383b),
        (0x6c, ONE),
    ] {
        g.write::<u32>(obj + off, v);
    }
    let mut p = w(0x9f);
    loop {
        let (x, _) = random(g, img, f, 0xa4, 0xc5, 0xcd);
        g.write::<f32>(p.wrapping_sub(4), x);
        let (y, _) = random(g, img, f, 0xbd, 0xf3, 0xfb);
        g.write::<f32>(p, y);
        let (z, r) = random(g, img, f, 0xeb, 0x122, 0x12d);
        g.write::<u32>(p + 8, ONE);
        p = p.wrapping_add(0x10);
        g.write::<f32>(p.wrapping_sub(0xc), z);
        if p.cast_signed() >= w(0x138).cast_signed() {
            break Ret::eax(r);
        }
    }
}

/// 0x10001000 (325 bytes, 1 overlay).
pub const E1000: Sig = Sig {
    len: 325,
    hash: 0x1a5a_af15_d1f3_6ddd,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// An object's frame (0x10001150, `(obj)`): the object placed
/// ([`place`]), slot 258 `(obj)`, two slot 231 with the camera globals,
/// `*G_P = ITEM` and slot 712 `(obj, 0x30)` with the game flag `*G_LOCK`
/// 1 (restored after; the earlier words left on the stack). Returns 1
/// when slot 712 says 0, else 0.
pub fn e1150<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let mut stack = place(g, img, f, obj, (0xf, 0x1b, 0x20, 0x26)).to_vec();
    let t = slot_target(g, w(0x33), 258);
    stack.insert(0, obj);
    // call width: 5
    g.call(t, 0, &stack);
    for (ops, tab) in [([0x42u32, 0x4d, 0x59], 0x47u32), ([0x61, 0x6c, 0x78], 0x66)] {
        for op in ops {
            stack.insert(0, g.read::<u32>(w(op)));
        }
        let t = slot_target(g, w(tab), 231);
        // call width: 11
        g.call(t, 0, &stack);
    }
    let gp = g.read::<u32>(w(0x7f));
    g.write::<u32>(gp, w(0x87));
    let lock = g.read::<u32>(w(0x8c));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x9a), 712);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 13
    let r = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0xa8));
    g.write::<u32>(lock, saved);
    Ret::eax(u32::from(r == 0))
}

/// 0x10001150 (190 bytes, 1 overlay).
pub const E1150: Sig = Sig {
    len: 190,
    hash: 0xce33_c742_3b6b_bd18,
    calls: &[],
    imports: &[],
};

/// The light's start (0x10001240, an overlay table entry, `(obj)`):
/// program 1 started ([`start_part`]), the object at the light's point
/// (`w` 1.0, `+0x50..` 0, `+0x5c` 1), a random scale `((r / D - S) * M)`
/// widened to `f64`, `* A + A` and narrowed (`+0x6c` 1.0); slot 400
/// `(1)`, slot 401 `(0, P, 1000.0, 0xff, 0, 0, 0)` and slot 398 `(*G_H,
/// 1)`. Returns slot 398's `eax`.
pub fn e1240<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let below = start_part(g, img, f, obj, [0x4, 0xb, 0x10, 0x18, 0x36], 4, &[]);
    let t = slot_target(g, w(0x3b), 324);
    for (i, v) in (0u32..).zip(LIGHT) {
        g.write::<u32>(obj + 0x40 + 4 * i, v);
    }
    for (off, v) in [(0x4cu32, ONE), (0x50, 0), (0x54, 0), (0x58, 0), (0x5c, 1)] {
        g.write::<u32>(obj + off, v);
    }
    let r = g.call(t, 0, &[]).eax & 0x7fff;
    let x = div(cvt(r), g.read::<f32>(w(0x94)));
    g.write::<u32>(obj + 0x6c, ONE);
    let x = mul(sub(x, g.read::<f32>(w(0xa3))), g.read::<f32>(w(0xad)));
    let a = g.read::<f64>(w(0xb8));
    let wide = f64::from(x) * a;
    let wide = wide + g.read::<f64>(w(0xc0));
    // cvtpd2ps: rounded to the nearest f32
    #[allow(clippy::cast_possible_truncation)]
    let s = wide as f32;
    for off in [0x60, 0x64, 0x68] {
        g.write::<f32>(obj + off, s);
    }
    let t = slot_target(g, w(0x8c), 400);
    let mut args = vec![1];
    args.extend(below);
    // call width: 5
    g.call(t, 0, &args);
    let t = slot_target(g, w(0xe0), 401);
    let mut stack = vec![0, LIGHT[0], LIGHT[1], LIGHT[2], 0x447a_0000, 0xff, 0, 0, 0];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x11f), 398);
    stack.splice(0..0, [g.read::<u32>(w(0x127)), 1]);
    // call width: 11
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x10001240 (313 bytes, 1 overlay).
pub const E1240: Sig = Sig {
    len: 313,
    hash: 0x1c9d_3856_5d81_f8bb,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The light's frame (0x10001380, an overlay table entry, `(obj)`): slot
/// 401 `(0, P, level, 200, 128, 128, 0)` with the level `T[*G_N] * K`
/// truncated, slot 398 `(*G_H, 1)`; the object drawn (slot 258, two slot
/// 231, slot 712 `(obj, 0x30)` under the game flag, restored after; the
/// earlier words left on the stack); when 712 says nonzero, slot 713
/// `(obj)` removes it and program 1 starts again ([`start_part`]).
/// Returns 1.
pub fn e1380<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let n = g.read::<u32>(w(0x4));
    let v = g.read::<f32>(w(0xe).wrapping_add(n.wrapping_mul(4)));
    let level = cvt(crate::s4_summon::trunc(mul(v, g.read::<f32>(w(0x16)))));
    let t = slot_target(g, w(0x2b), 401);
    let mut stack = vec![
        0,
        LIGHT[0],
        LIGHT[1],
        LIGHT[2],
        level.to_bits(),
        0xc8,
        0x80,
        0x80,
        0,
    ];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x67), 398);
    stack.splice(0..0, [g.read::<u32>(w(0x6f)), 1]);
    // call width: 11
    g.call(t, 0, &stack);
    let obj = fr.arg(g, 0);
    let t = slot_target(g, w(0x7c), 258);
    stack.insert(0, obj);
    // call width: 12
    g.call(t, 0, &stack);
    for (ops, tab) in [([0x8eu32, 0x99, 0xa5], 0x93u32), ([0xad, 0xb8, 0xc4], 0xb2)] {
        for op in ops {
            stack.insert(0, g.read::<u32>(w(op)));
        }
        let t = slot_target(g, w(tab), 231);
        // call width: 18
        g.call(t, 0, &stack);
    }
    let gp = g.read::<u32>(w(0xcb));
    g.write::<u32>(gp, w(0xd4));
    let lock = g.read::<u32>(w(0xd9));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0xe8), 712);
    let r = g.call(t, 0, &[obj, 0x30]).eax;
    let lock = g.read::<u32>(w(0xf7));
    g.write::<u32>(lock, saved);
    if r != 0 {
        let gp = g.read::<u32>(w(0x105));
        g.write::<u32>(gp, w(0x10c));
        let t = slot_target(g, w(0x111), 713);
        g.call(t, 0, &[obj]);
        start_part(
            g,
            img,
            f,
            obj,
            [0x11e, 0x124, 0x129, 0x131, 0x14c],
            4,
            &[obj],
        );
    }
    Ret::eax(1)
}

/// 0x10001380 (348 bytes, 1 overlay).
pub const E1380: Sig = Sig {
    len: 348,
    hash: 0xca89_effc_e025_6ce8,
    calls: &[(0x14c, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
