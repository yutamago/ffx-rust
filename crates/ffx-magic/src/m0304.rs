//! `magic_0304.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::sb_sse::add;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object's rise (0x10001cd0, cdecl `(obj)`): from frame 0x54
/// (`+0x118`) on, slot 12 `(T[b])`'s float is dropped (`b` the byte
/// `**P`, `P` `+0x11`, `T` `+0x22`), the effect's speed `+0x1720` (`E`
/// = `*G`, `+0x1b`) grows by `K` (`+0x3e`) and the object's height
/// `+0xd4` by the speed. Returns slot 12's `eax` (the caller's before).
pub fn rise<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    if g.read::<i32>(obj + 0x118) < 0x54 {
        return Ret::eax(fr.eax);
    }
    let b = g.read::<u8>(g.read::<u32>(w(0x11)));
    let fx = g.read::<u32>(w(0x1b));
    let x = g.read::<u32>(w(0x22).wrapping_add(4 * u32::from(b)));
    let s = slot_target(g, w(0x27), 0x30 / 4);
    let eax = g.call(s, 0, &[x]).eax;
    let speed = add(g.read::<f32>(fx + 0x1720), g.read::<f32>(w(0x3e)));
    g.write::<f32>(fx + 0x1720, speed);
    let h = add(g.read::<f32>(obj + 0xd4), speed);
    g.write::<f32>(obj + 0xd4, h);
    Ret::eax(eax)
}

/// [`rise`] (101 bytes, 1 overlay).
pub const RISE: Sig = Sig {
    len: 101,
    hash: 0xf2d3_7558_0eff_428f,
    calls: &[],
    imports: &[],
};

/// A battle-mode drift (0x10002150, no arguments): slot 520 `()`; for
/// the mode 0x018b0046 the effect's (`*G`, `+0x15`) `+0x334` grows by `B`
/// (`+0x33`) and `+0x204` by `A` (`+0x2b`), else both by 0. Returns slot
/// 520's `eax`.
pub fn drift<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let s = slot_target(g, w(0x05), 0x820 / 4);
    let fx = g.read::<u32>(w(0x15));
    let mode = g.call(s, 0, &[]).eax;
    let (a, b) = if mode == 0x018b_0046 {
        (g.read::<f32>(w(0x2b)), g.read::<f32>(w(0x33)))
    } else {
        (0.0, 0.0)
    };
    let v = add(g.read::<f32>(fx + 0x334), b);
    g.write::<f32>(fx + 0x334, v);
    let v = add(g.read::<f32>(fx + 0x204), a);
    g.write::<f32>(fx + 0x204, v);
    Ret::eax(mode)
}

/// [`drift`] (111 bytes, 1 overlay).
pub const DRIFT: Sig = Sig {
    len: 111,
    hash: 0x0cb6_c3f2_3148_1a6b,
    calls: &[],
    imports: &[],
};

/// The eased height (0x100021c0, no arguments, no frame): with `E` =
/// `*G` (`+0x03`), the ease of the key table `+0x17` (5 keys, stride 1;
/// [`ease_sample`](crate::sb_misc::ease_sample)) at the time `+0x1724`,
/// which then moves on by `K` (`+0x27`); with `h` = `+0x149c`: `+0x464` =
/// `h + A + e`, `+0x6c4` = `h + A' + e` (`A`, `A'` `+0x45`, `+0x61`),
/// `+0x146c` and `+0x140c` = `h + e`, `+0x148c` = `h + B + e` (`B`
/// `+0x80`). Returns 0 (the ease's scratch `eax` in the original, which
/// no caller reads).
pub fn eased_height<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let fx = g.read::<u32>(w(0x03));
    let t = g.read::<f32>(fx + 0x1724);
    // push esi; push 1 (the stride): the call's return address below
    let stride = fr.esp.wrapping_sub(8);
    g.write::<u32>(stride, 1);
    let e = crate::sb_misc::ease_sample(
        g,
        img,
        img.rel32_target(fr.at + 0x1f),
        fr.esp.wrapping_sub(0xc),
        w(0x17),
        5,
        t,
    );
    let t = add(t, g.read::<f32>(w(0x27)));
    g.write::<f32>(fx + 0x1724, t);
    let h = |g: &G| g.read::<f32>(fx + 0x149c);
    let v = add(add(h(g), g.read::<f32>(w(0x45))), e);
    g.write::<f32>(fx + 0x464, v);
    let v = add(add(h(g), g.read::<f32>(w(0x61))), e);
    g.write::<f32>(fx + 0x6c4, v);
    let x = h(g);
    let hb = add(add(x, g.read::<f32>(w(0x80))), e);
    let he = add(x, e);
    g.write::<f32>(fx + 0x146c, he);
    g.write::<f32>(fx + 0x148c, hb);
    g.write::<f32>(fx + 0x140c, he);
    Ret::eax(0)
}

/// [`eased_height`] (166 bytes, 1 overlay).
pub const EASED_HEIGHT: Sig = Sig {
    len: 166,
    hash: 0x31fe_c1d4_22a7_1c3c,
    calls: &[(0x1f, Callee::Sig(&crate::sb_misc::EASE_SAMPLE))],
    imports: &[],
};
