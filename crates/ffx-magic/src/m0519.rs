//! `magic_0519.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sb_sse::{cvt, mul};
use crate::{Frame, Game, Image, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Places the effect's light (0x100022b0, internal: `ecx` = a point):
/// the flag byte at operand `+0x08` 1, slot 247 `(M, -0x4000)` turns the
/// matrix `M` (`+0x19`), the point goes to its translation (`+0x2e..`)
/// and slot 327 `(N, M)` copies it to `N` (`+0x65`; 247's words left on
/// the stack). Returns slot 327's `eax`.
pub fn place<G: Game>(g: &mut G, img: &Image, f: u32, p: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 247);
    g.write::<u8>(w(0x8), 1);
    g.call(t, 0, &[w(0x19), 0xffff_c000]);
    for (i, op) in (0u32..).zip([0x2e, 0x3b, 0x48, 0x55]) {
        let v = g.read::<u32>(p + 4 * i);
        g.write::<u32>(w(op), v);
    }
    let t = slot_target(g, w(0x26), 327);
    g.call(t, 0, &[w(0x65), w(0x60), w(0x19), 0xffff_c000]).eax
}

/// [`place`] (112 bytes, 1 overlay).
pub const PLACE: Sig = Sig {
    len: 112,
    hash: 0x3068_bc60_605f_339b,
    calls: &[],
    imports: &[],
};

/// Sets the screen's colour filter (0x10002600, internal: `ecx` = a
/// level `n`): nothing for `n` not above 0 (returns `eax` as given);
/// else slot 821 `(0)`, slot 822 `(1, 1, 1, n * K)`, slot 823 `(1.0078,
/// 1.0096)`, slot 824 `(0.5, 0.5, 0, 0)` (the words written over the
/// earlier ones as the original reuses its stack). Returns slot 824's
/// `eax`.
pub fn filter<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let n = fr.ecx;
    if n.cast_signed() <= 0 {
        return fr.eax;
    }
    let t = slot_target(g, w(0xc), 821);
    g.call(t, 0, &[0]);
    let t = slot_target(g, w(0x1b), 822);
    let x = mul(cvt(n), g.read::<f32>(w(0x33)));
    g.call(t, 0, &[ONE, ONE, ONE, x.to_bits()]);
    let t = slot_target(g, w(0x57), 823);
    g.call(t, 0, &[0x3f81_0000, 0x3f81_3b14]);
    let t = slot_target(g, w(0x76), 824);
    g.call(t, 0, &[0x3f00_0000, 0x3f00_0000, 0, 0]).eax
}

/// [`filter`] (168 bytes, 1 overlay).
pub const FILTER: Sig = Sig {
    len: 168,
    hash: 0x5761_d35b_484b_2454,
    calls: &[],
    imports: &[],
};

/// Fades a model's colour (0x100026b0, internal: `ecx` = the frame `n`):
/// nothing from frame 19 on (returns `n`); the level `b` rises over
/// frames 0..4 (`n * 128 / 4`) and falls after (`(19 - n) * 128 / 15`,
/// signed divisions); slot 776 `(0x48)` gives the model and slot 915
/// `(1, 1, 1, (b & 0xff) * K, model)` sets its colour. Returns slot 915's
/// `eax`.
pub fn fade<G: Game>(g: &mut G, img: &Image, f: u32, n: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let n = n.cast_signed();
    if n >= 0x13 {
        return n.cast_unsigned();
    }
    let b = if n <= 4 {
        n.wrapping_shl(7) / 4
    } else {
        0x13i32.wrapping_sub(n).wrapping_shl(7) / 15
    };
    let t = slot_target(g, w(0x3e), 776);
    let model = g.call(t, 0, &[0x48]).eax;
    let t = slot_target(g, w(0x58), 915);
    let x = mul(cvt(b.cast_unsigned() & 0xff), g.read::<f32>(w(0x69)));
    g.call(t, 0, &[ONE, ONE, ONE, x.to_bits(), model]).eax
}

/// [`fade`] (148 bytes, 1 overlay).
pub const FADE: Sig = Sig {
    len: 148,
    hash: 0xec64_c776_e105_b6a4,
    calls: &[],
    imports: &[],
};
