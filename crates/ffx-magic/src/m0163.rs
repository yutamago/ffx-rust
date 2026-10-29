//! `magic_0163.dll`'s own functions (besides the runtime, the OPU host
//! and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The x87 indefinite an `fstp` of an empty stack stores (a call-out
/// that left no float).
const INDEFINITE: f32 = f32::from_bits(0xffc0_0000);

/// A particle's step callback (stored by the code at 0x100015fa;
/// cdecl `(p)`): while the counter `+0x118` is below 16, `+0xd4 += A`;
/// below 18, `+0xd4 = counter / B * C + +0xd4`; from 18 on, `+0x104` is
/// cleared. `A`, `B`, `C` are `f32` constants (operands `+0x1d`, `+0x3b`,
/// `+0x43`). Returns `p`.
pub fn e1d10<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = fr.arg(g, 0);
    let n = g.read::<u32>(p + 0x118).cast_signed();
    if n < 0x10 {
        let v = g.read::<f32>(p + 0xd4) + g.read::<f32>(w(0x1d));
        g.write::<f32>(p + 0xd4, v);
    } else if n < 0x12 {
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps rounds the same
        let x = n as f32 / g.read::<f32>(w(0x3b)) * g.read::<f32>(w(0x43));
        let v = x + g.read::<f32>(p + 0xd4);
        g.write::<f32>(p + 0xd4, v);
    } else {
        g.write::<u32>(p + 0x104, 0);
    }
    Ret::eax(p)
}

/// 0x10001d10 (101 bytes, 1 overlay).
pub const E1D10: Sig = Sig {
    len: 101,
    hash: 0x3f55_e331_5ad2_9472,
    calls: &[],
    imports: &[],
};

/// The angle to a target (`ecx` = its index, -1 for the caster): the
/// character is `G_SELF` (operand `+0x0a`) for -1, else
/// `G_CHRS[G_MAP[index]]` (the byte table at `+0x11`, the word table at
/// `+0x1c`); `x = A - Ch_GetRot(chr)` (slot 333, ST0 stored as `f32`; `A`
/// at `+0x35`), wrapped once into range: `x - T` when `x > HI` (`T` at
/// `+0x4e`, `HI` `+0x44`), `x + T` (`+0x67`) when `LO > x` (`LO` at
/// `+0x5a`). Returns `x` (`xmm0`).
///
/// 0x10001e00 (111 bytes, 1 overlay).
pub fn target_angle<G: Game>(g: &mut G, img: &Image, f: u32, index: u32) -> f32 {
    let w = |off: u32| img.word(f + off);
    let chr = if index == 0xffff_ffff {
        g.read::<u32>(w(0x0a))
    } else {
        let map = g.read::<u32>(w(0x11));
        let k = u32::from(g.read::<u8>(map.wrapping_add(index)));
        g.read::<u32>(w(0x1c).wrapping_add(4 * k))
    };
    let s333 = slot_target(g, w(0x22), 333);
    let rot = g
        .call(s333, index, &[chr])
        .st0
        .map_or(INDEFINITE, |v| v as f32);
    let x = g.read::<f32>(w(0x35)) - rot;
    // comiss + jbe: taken when below, equal or unordered
    if x > g.read::<f32>(w(0x44)) {
        return x - g.read::<f32>(w(0x4e));
    }
    if g.read::<f32>(w(0x5a)) > x {
        return x + g.read::<f32>(w(0x67));
    }
    x
}

/// [`target_angle`].
pub const TARGET_ANGLE: Sig = Sig {
    len: 111,
    hash: 0xff5a_2c8f_f197_8bb4,
    calls: &[],
    imports: &[],
};
