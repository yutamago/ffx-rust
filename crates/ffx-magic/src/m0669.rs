//! `magic_0669.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sb_sse::add;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's frame (0x10001030, an overlay table entry, no
/// arguments): the level `*G_L` (operand `+0x04`) clamped to 0..`H`
/// (`+0x18`; above it the double at `+0x22` rounded to a float; a NaN
/// comes back quiet from the round trip through a double); slot 270
/// `(A, *G_S, B, C, 3)` (the immediates `+0x5d`, `+0x42`, `+0x3d`, the
/// scale `+0x53`); when `*G_M` (`+0x67`) equals `K` (`+0x71`) the scale
/// grows by `D` (`+0x87`). Returns whether the scale is at most `E`
/// (`+0x98`) or unordered with it.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let x = g.read::<f32>(w(0x4));
    // comiss 0, x; jbe: below, equal or unordered
    let x = if 0.0 > x {
        0.0
    } else if x > g.read::<f32>(w(0x18)) {
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps rounds
        let k = g.read::<f64>(w(0x22)) as f32;
        k
    } else if x.is_nan() {
        // cvtps2pd quiets a signalling NaN
        f32::from_bits(x.to_bits() | 0x0040_0000)
    } else {
        x
    };
    g.write::<f32>(w(0x4b), x);
    let s = g.read::<u32>(w(0x53));
    let t = slot_target(g, w(0x2c), 270);
    g.call(t, 0, &[w(0x5d), s, w(0x42), w(0x3d), 3]);
    let m = g.read::<f32>(w(0x67));
    let mut y = g.read::<f32>(w(0x79));
    let k = g.read::<f32>(w(0x71));
    if m.partial_cmp(&k) == Some(core::cmp::Ordering::Equal) {
        y = add(y, g.read::<f32>(w(0x87)));
        g.write::<f32>(w(0x8f), y);
    }
    let e = g.read::<f32>(w(0x98));
    // setbe: below, equal or unordered
    Ret::eax(u32::from(
        y.partial_cmp(&e) != Some(core::cmp::Ordering::Greater),
    ))
}

/// [`frame`] (160 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 160,
    hash: 0x98f0_68c5_861b_d480,
    calls: &[],
    imports: &[],
};

/// An object's frame (0x10001c30, an overlay table entry, `(obj)`): the
/// colour `+0x60..+0x68` 0.75, `+0x6c` 1.0; slot 258 `(obj)`, slot 231
/// with the globals at `+0x4b`, `+0x3f`, `+0x34` and slot 230 with
/// `+0x5e`, `+0x53` (the earlier words left on the stack); `*G_P = ITEM`
/// (`+0x6b`, `+0x73`), slot 712 `(obj, 0x30)` with the game flag
/// `*G_LOCK` (`+0x78`) 1 and restored after (`+0x94`). Returns whether
/// 712 said 0.
pub fn draw<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let obj = fr.arg(g, 0);
    Ret::eax(u32::from(draw_body(g, img, fr, obj, 0x3f40_0000) == 0))
}

/// [`draw`] (170 bytes, 1 overlay).
pub const DRAW: Sig = Sig {
    len: 170,
    hash: 0xde41_f577_47ba_8385,
    calls: &[],
    imports: &[],
};

/// [`draw`]'s body with the colour word `c`: returns slot 712's answer.
fn draw_body<G: Game>(g: &mut G, img: &Image, fr: &Frame, obj: u32, c: u32) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let t = slot_target(g, w(0x4), 258);
    for off in [0x60, 0x64, 0x68] {
        g.write::<u32>(obj + off, c);
    }
    g.write::<u32>(obj + 0x6c, 0x3f80_0000);
    let mut stack = vec![obj];
    g.call(t, 0, &stack);
    for op in [0x34, 0x3f, 0x4b] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x39), 231);
    // call width: 4 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    for op in [0x53, 0x5e] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x58), 230);
    // call width: 6 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    let gp = g.read::<u32>(w(0x6b));
    g.write::<u32>(gp, w(0x73));
    let lock = g.read::<u32>(w(0x78));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x86), 712);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 8 (the earlier calls' words stay on the stack)
    let r = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x94));
    g.write::<u32>(lock, saved);
    r
}

/// An object's frame with restart (0x10001a90, an overlay table entry,
/// `(obj)`): [`draw`]'s body with the colour 0x3e02a8ca; when 712 says
/// nonzero, `*G_P = ITEM` (`+0xa2`, `+0xa9`), slot 713 `(obj)` removes it
/// and the part starts again with program 2
/// ([`start_part`](crate::part_draw::start_part)). Returns 1.
pub fn draw_restart<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    if draw_body(g, img, fr, obj, 0x3e02_a8ca) == 0 {
        return Ret::eax(1);
    }
    let gp = g.read::<u32>(w(0xa2));
    g.write::<u32>(gp, w(0xa9));
    let t = slot_target(g, w(0xae), 713);
    g.call(t, 0, &[obj]);
    crate::part_draw::start_part(g, img, f, obj, [0xbb, 0xc1, 0xc6, 0xce, 0xe9], 8, &[obj]);
    Ret::eax(1)
}

/// [`draw_restart`] (249 bytes, 1 overlay).
pub const DRAW_RESTART: Sig = Sig {
    len: 249,
    hash: 0x92ac_7669_2c26_2934,
    calls: &[(0xe9, crate::Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// An object's frame placed at a point (0x100018b0, an overlay table
/// entry, `(obj)`): slot 325 `(obj + 0x40, POINT)` with the scales
/// 0.1276 (`+0x6c` 1.0), the object drawn (slot 258, slot 231 `(*G_A,
/// *G_B, *G_C)`, slot 230 `(*G_D, *G_A)`, slot 712 `(obj, 0x30)` under
/// the game flag with `**G_P = ITEM`; the earlier words left on the
/// stack); when 712 says nonzero, slot 713 `(obj)` removes it and the
/// part starts again with program 1
/// ([`start_part`](crate::part_draw::start_part)). Returns 1.
pub fn frame_at<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, op: u32| g.read::<u32>(w(op));
    let obj = fr.arg(g, 0);
    let t = slot_target(g, w(0x12), 325);
    for o in [0x60u32, 0x64, 0x68] {
        g.write::<u32>(obj + o, 0x3e02_a8ca);
    }
    g.write::<u32>(obj + 0x6c, 0x3f80_0000);
    let mut stack = vec![obj + 0x40, w(0x9)];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x3b), 258);
    stack.insert(0, obj);
    // call width: 18 at most (the earlier calls' words)
    g.call(t, 0, &stack);
    let a = r(g, 0x4a);
    let t = slot_target(g, w(0x4f), 231);
    let b = r(g, 0x55);
    let c = r(g, 0x61);
    stack.splice(0..0, [c, b, a]);
    // call width: 6
    g.call(t, 0, &stack);
    let a = r(g, 0x69);
    let t = slot_target(g, w(0x6e), 230);
    let d = r(g, 0x74);
    stack.splice(0..0, [d, a]);
    // call width: 8
    g.call(t, 0, &stack);
    let gp = r(g, 0x81);
    g.write::<u32>(gp, w(0x89));
    let lock = r(g, 0x8e);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x9c), 712);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 10
    let res = g.call(t, 0, &stack).eax;
    let lock = r(g, 0xaa);
    g.write::<u32>(lock, saved);
    if res != 0 {
        let gp = r(g, 0xb8);
        g.write::<u32>(gp, w(0xbf));
        let t = slot_target(g, w(0xc4), 713);
        g.call(t, 0, &[obj]);
        crate::part_draw::start_part(g, img, f, obj, [0xd1, 0xd7, 0xdc, 0xe4, 0xff], 4, &[obj]);
    }
    Ret::eax(1)
}

/// [`frame_at`] (271 bytes, 1 overlay).
pub const FRAME_AT: Sig = Sig {
    len: 271,
    hash: 0x1d3e_960a_0b5a_7be2,
    calls: &[(0xff, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
