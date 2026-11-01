//! `magic_0704.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sb_sse::{cvt, mul};
use crate::{Frame, Game, Image, Sig};

/// Sets the fade for step `i` (0x10001000, internal: `ecx` = `i`):
/// nothing from step 9 (returns the caller's `eax`). The level byte is
/// `i << 7` (its low byte) up to step 1, else `((9 - i) << 7) / 8`; slot
/// 776 `(0x44)` gives a handle `h`, slot 915 `(0x3fff0000 three times,
/// level * K, h)` (`K` at operand `+0x52`). Returns slot 915's `eax`.
pub fn fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let i = fr.ecx.cast_signed();
    if i >= 9 {
        return fr.eax;
    }
    let level = if i <= 1 {
        (fr.ecx << 7) & 0xff
    } else {
        // cdq; and edx, 7; sar 3: a signed divide by 8
        (((9 - i) << 7) / 8).cast_unsigned() & 0xff
    };
    let t = slot_target(g, w(0x27), 776);
    let h = g.call(t, 0, &[0x44]).eax;
    let x = mul(cvt(level), g.read::<f32>(w(0x52)));
    let t = slot_target(g, w(0x41), 915);
    let c = 0x3fff_0000;
    g.call(t, 0, &[c, c, c, x.to_bits(), h]).eax
}

/// [`fade`] (125 bytes, 1 overlay).
pub const FADE: Sig = Sig {
    len: 125,
    hash: 0x78fb_4e91_aca8_bc7d,
    calls: &[],
    imports: &[],
};

/// Loads the effect data (0x10001080, internal, no arguments; the data
/// at the immediate `+0x06`): slot 718 `(data)`, slots 222 and 221
/// `(data, 0, 1000, 0x100)`, three slot 719 `(data, 6, k)` into the
/// words at `+0x65`, `+0x80` and `+0x9e`, each call seeing the earlier
/// calls' words. Returns the last entry.
pub fn load<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let data = w(0x6);
    let t = slot_target(g, w(0x1), 718);
    let mut stack = vec![data];
    g.call(t, 0, &stack);
    for (op, slot) in [(0x13, 222), (0x31, 221)] {
        let t = slot_target(g, w(op), slot);
        stack.splice(0..0, [data, 0, 0x3e8, 0x100]);
        // call width: 9 at most (the earlier calls' words stay on the stack)
        g.call(t, 0, &stack);
    }
    let mut eax = 0;
    for (op, k, out) in [(0x4f, 0, 0x65), (0x6a, 1, 0x80), (0x85, 2, 0x9e)] {
        let t = slot_target(g, w(op), 719);
        stack.splice(0..0, [data, 6, k]);
        // call width: 18 at most (the earlier calls' words stay on the stack)
        eax = g.call(t, 0, &stack).eax;
        g.write::<u32>(w(out), eax);
    }
    eax
}

/// [`load`] (163 bytes, 1 overlay).
pub const LOAD: Sig = Sig {
    len: 163,
    hash: 0x8d00_92ba_efe4_6f03,
    calls: &[],
    imports: &[],
};
