//! `magic_0536.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::part_draw::start_part;
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object's start (0x10001000, `(obj)`): program 1 started
/// ([`start_part`]), slot 237 `(*G_H, rand() % 184, 0, obj + 0x40)`
/// places it at a random bone (the start's words left on the stack),
/// `+0x50..` 0, `+0x5c` 1, a random scale `r / D * M + A` with `r` slot
/// 324's low 15 bits (`+0x6c` 1.0). Returns `r`.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let below = start_part(g, img, f, obj, [0x4, 0xb, 0x10, 0x18, 0x36], 4, &[]);
    // cdq; idiv 0xb8: the signed remainder
    let bone = (g.rand().cast_signed() % 0xb8).cast_unsigned();
    let t = slot_target(g, w(0x4d), 237);
    let mut args = vec![g.read::<u32>(w(0x5c)), bone, 0, obj + 0x40];
    args.extend(below);
    // call width: 8
    g.call(t, 0, &args);
    let t = slot_target(g, w(0x63), 324);
    for (off, v) in [(0x50u32, 0u32), (0x54, 0), (0x58, 0), (0x5c, 1)] {
        g.write::<u32>(obj + off, v);
    }
    let r = g.call(t, 0, &[]).eax & 0x7fff;
    g.write::<u32>(obj + 0x6c, 0x3f80_0000);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let s = add(mul(div(cvt(r), k(g, 0xa5)), k(g, 0xad)), k(g, 0xb5));
    for off in [0x60, 0x64, 0x68] {
        g.write::<f32>(obj + off, s);
    }
    Ret::eax(r)
}

/// 0x10001000 (203 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 203,
    hash: 0x3724_1183_d562_2813,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[(0x3c, "rand")],
};

/// An object's frame (0x100012a0, `(obj)`): slot 315 `(0)`, slot 258
/// `(obj)`, slots 231 and 230 with the camera globals, `*G_P = ITEM` and
/// slot 712 `(obj, 0x30)` with the game flag `*G_LOCK` 1 (restored after;
/// the earlier words left on the stack). Returns 1 when slot 712 says 0,
/// else 0.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let t = slot_target(g, w(0x4), 315);
    let mut stack = vec![0];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x14), 258);
    stack.insert(0, obj);
    // call width: 9
    g.call(t, 0, &stack);
    for op in [0x25, 0x30, 0x3c] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x2a), 231);
    // call width: 5
    g.call(t, 0, &stack);
    for op in [0x44, 0x4f] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x49), 230);
    // call width: 7
    g.call(t, 0, &stack);
    let gp = g.read::<u32>(w(0x5c));
    g.write::<u32>(gp, w(0x67));
    let lock = g.read::<u32>(w(0x6c));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x79), 712);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 9
    let r = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x87));
    g.write::<u32>(lock, saved);
    Ret::eax(u32::from(r == 0))
}

/// 0x100012a0 (156 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 156,
    hash: 0x28c0_d916_ac07_40aa,
    calls: &[],
    imports: &[],
};

/// The effect's start (0x10001340, an overlay table entry, cdecl
/// `(arg)`): [`effect_head`](crate::m0523::effect_head) with the half at
/// `+0x100000` and the block defaults
/// ([`block_defaults`](crate::s4_small::block_defaults)); then slot 260
/// `(0x14)` into the global at operand `+0x13f` and slot 38 `(that, 0,
/// 0)`. Returns 1.
pub fn effect_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut p = crate::m0523::effect_head(g, img, fr, 0x10_0000, |g, callee| {
        crate::s4_small::block_defaults(g, img, callee);
    });
    p.push(0x14);
    let t = slot_target(g, w(0x4), 260);
    // call width: 12 (Pushes caps the words at 12)
    let v = p.call(g, t, 0).eax;
    p.pop(2);
    g.write::<u32>(w(0x13f), v);
    // push 0; push ecx; mov [esp], 0; push eax
    p.push_all(&[0, 0, v]);
    let t = slot_target(g, w(0x14f), 38);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    Ret::eax(1)
}

/// [`effect_start`] (358 bytes, 1 overlay).
pub const EFFECT_START: Sig = Sig {
    len: 358,
    hash: 0xc0d9_b63f_af24_b5b4,
    calls: &[(0x128, Callee::Sig(&crate::s4_small::BLOCK_DEFAULTS))],
    imports: &[],
};
