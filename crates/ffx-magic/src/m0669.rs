//! `magic_0669.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sb_sse::add;
use crate::{Frame, Game, Image, Ret, Sig};

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
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let t = slot_target(g, w(0x4), 258);
    for off in [0x60, 0x64, 0x68] {
        g.write::<u32>(obj + off, 0x3f40_0000);
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
    Ret::eax(u32::from(r == 0))
}

/// [`draw`] (170 bytes, 1 overlay).
pub const DRAW: Sig = Sig {
    len: 170,
    hash: 0xde41_f577_47ba_8385,
    calls: &[],
    imports: &[],
};
