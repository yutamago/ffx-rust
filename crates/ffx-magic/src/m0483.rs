//! `magic_0483.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sb_sse::sub;
use crate::{Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;
/// `0.25f32`'s bits.
const QUARTER: u32 = 0x3e80_0000;

/// The effect's start (0x10001040, an overlay table entry, `(arg)`):
/// `arg` to the global at operand `+0x07`; slot 803 `(0x100000, 0x10)`
/// gives the buffer (`+0x25`), slot 861 `(buf, NAME, 0x2be50)` loads the
/// file into it, slot 259, slot 260 `(0x14)` a handle (`+0x59`), slots
/// 718 `(NAME)`, 222 and 221 `(NAME, 0, 1000, 0x100)` (each with the
/// earlier words on the stack); then seven state words 0. Returns 0.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x7), arg);
    let t = slot_target(g, w(0xc), 803);
    let mut stack = vec![0x10_0000, 0x10];
    let buf = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x25), buf);
    stack.splice(0..0, [buf, w(0x2a), 0x2_be50]);
    let t = slot_target(g, w(0x30), 861);
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x3d), 259);
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x4a), 260);
    stack.insert(0, 0x14);
    // call width: 15
    let h = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x59), h);
    let t = slot_target(g, w(0x5e), 718);
    stack.insert(0, w(0x63));
    // call width: 15
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x70), 222);
    stack.splice(0..0, [w(0x87), 0, 0x3e8, 0x100]);
    // call width: 15
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x8e), 221);
    stack.splice(0..0, [w(0xa5), 0, 0x3e8, 0x100]);
    // call width: 15
    g.call(t, 0, &stack);
    for op in [0xb0, 0xba, 0xc4, 0xce, 0xd8, 0xe2, 0xec] {
        g.write::<u32>(w(op), 0);
    }
    Ret::eax(0)
}

/// [`start`] (248 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 248,
    hash: 0x2e18_3738_204d_6e3b,
    calls: &[],
    imports: &[],
};

/// The effect's end (0x10001140, an overlay table entry, no arguments):
/// the three parts removed (slot 713 `(part)`, `*G_P = ITEM` before
/// each; the earlier words left on the stack), slot 804 frees the buffer
/// `*G_BUF` (operand `+0x5e`), which becomes 0. Returns 0.
pub fn end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut stack = Vec::new();
    for (gp, part, item, t) in [
        (0x1u32, 0x6u32, 0xcu32, 0x11u32),
        (0x1e, 0x23, 0x29, 0x2e),
        (0x3b, 0x40, 0x46, 0x4b),
    ] {
        let p = g.read::<u32>(w(gp));
        stack.insert(0, w(part));
        g.write::<u32>(p, w(item));
        let s = slot_target(g, w(t), 713);
        // call width: 4
        g.call(s, 0, &stack);
    }
    let t = slot_target(g, w(0x58), 804);
    stack.insert(0, g.read::<u32>(w(0x5e)));
    // call width: 4
    g.call(t, 0, &stack);
    g.write::<u32>(w(0x6f), 0);
    Ret::eax(0)
}

/// [`end`] (122 bytes, 1 overlay).
pub const END: Sig = Sig {
    len: 122,
    hash: 0x719e_de89_6eda_a6d1,
    calls: &[],
    imports: &[],
};

/// Draws a part at an actor's bone (0x10001260, internal: `ecx` = the
/// part, `edx` the bone, `xmm2` a drop): nothing while the part's
/// halfword `+0x1e` is 0 (returns `eax` as given). Else the scales
/// `+0x60..+0x68` 0.25 (`+0x6c` 1.0), `+0x50..+0x58` 0, slot 237 `(*G_A,
/// bone, 0, p)` fills the point `p = +0x40`, which moves by `(-drop, -K1,
/// -K2)`; slot 258 `(part)`, slots 231 and 230 with the camera globals,
/// slot 712 `(part, 0x30)` with the game flag `*G_LOCK` 1 (each with the
/// earlier words on the stack); when 712 says nonzero the halfword
/// `+0x1e` is cleared. The flag ends 0. Returns 0.
pub fn draw<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let (part, bone) = (fr.ecx, fr.edx);
    let drop = fr.xmm_f32(2);
    if g.read::<u16>(part + 0x1e) == 0 {
        return fr.eax;
    }
    let p = part + 0x40;
    let t = slot_target(g, w(0x18), 237);
    let a = g.read::<u32>(w(0x26));
    for off in [0x68, 0x64, 0x60] {
        g.write::<u32>(part + off, QUARTER);
    }
    g.write::<u32>(part + 0x6c, ONE);
    for off in [0x58, 0x54, 0x50] {
        g.write::<u32>(part + off, 0);
    }
    let mut stack = vec![a, bone, 0, p];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x6d), 258);
    let x = sub(g.read::<f32>(p), drop);
    g.write::<f32>(p, x);
    for (o, k) in [(0x44u32, 0x7fu32), (0x48, 0x91)] {
        let v = sub(g.read::<f32>(part + o), g.read::<f32>(w(k)));
        g.write::<f32>(part + o, v);
    }
    stack.insert(0, part);
    // call width: 12
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xa9), 231);
    for op in [0xa4, 0xaf, 0xbb] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    // call width: 12
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xc8), 230);
    for op in [0xc3, 0xce] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    // call width: 12
    g.call(t, 0, &stack);
    let lock = g.read::<u32>(w(0xdc));
    let gp = g.read::<u32>(w(0xe1));
    g.write::<u32>(lock, 1);
    g.write::<u32>(gp, w(0xed));
    let t = slot_target(g, w(0xf2), 712);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    stack.splice(0..0, [part, 0x30]);
    // call width: 12
    let r = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x10b));
    g.write::<u32>(lock, saved);
    if r != 0 {
        g.write::<u16>(part + 0x1e, 0);
    }
    g.write::<u32>(lock, 0);
    0
}

/// [`draw`] (298 bytes, 1 overlay).
pub const DRAW: Sig = Sig {
    len: 298,
    hash: 0xe3fa_d22e_b3dc_0245,
    calls: &[],
    imports: &[],
};
