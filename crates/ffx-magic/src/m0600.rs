//! `magic_0600.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sa_pool::take;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Starts the effect's fade (0x10007160, an overlay table entry, no
/// arguments): slot 371 `(0, 0x10000)` and `(1, 0x20000)`, the scale
/// task of `*G_FX + 0xe3c`
/// ([`scale_task_fixed`](crate::s4_small::scale_task_fixed): `xmm1`/`xmm2`
/// the constants `+0x3b`/`+0x33`, slot 371's 0x10000 under it); the fade
/// state `S = *G_FX + 0xce0`: the colour bytes `+0x80..+0x87` `ff ff ff
/// 80 ff ff ff 00`, `+0x88` 0, the half-word `+0x98` 0x44, `+0x90` and
/// `+0x8c` 20, `+0x94` 0. A pool node ([`take`], owner the immediate
/// `+0x57`) runs it (`+0x14` = `S`, `+0x4` 0), appended to the list
/// (head `+0xb4`/`+0xe1`, tail `+0xc8`/`+0xd2`/`+0xe6`). Returns the
/// node.
pub fn fade_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 371);
    let fx = g.read::<u32>(w(0xe));
    g.call(t, 0, &[0, 0x1_0000]);
    let t = slot_target(g, w(0x1c), 371);
    g.call(t, 0, &[1, 0x2_0000, 0, 0x1_0000]);
    // push esi, the 0x10000 left, the return address
    let esp = fr.esp.wrapping_sub(0xc);
    g.write::<u32>(esp + 4, 0x1_0000);
    let mut xmm = fr.xmm;
    xmm[2] = u128::from(g.read::<u32>(w(0x33)));
    xmm[1] = u128::from(g.read::<u32>(w(0x3b)));
    let task = Frame {
        at: img.rel32_target(f + 0x49),
        esp,
        ecx: fx.wrapping_add(0xe3c),
        xmm,
        ..*fr
    };
    crate::s4_small::scale_task_fixed(g, img, &task);
    let s = fx.wrapping_add(0xce0);
    g.write::<u32>(s + 0x81, 0xff80_ffff);
    g.write::<u8>(s + 0x80, 0xff);
    g.write::<u16>(s + 0x85, 0xffff);
    g.write::<u8>(s + 0x87, 0);
    g.write::<u32>(s + 0x88, 0);
    g.write::<u16>(s + 0x98, 0x44);
    g.write::<u32>(s + 0x90, 0x14);
    g.write::<u32>(s + 0x8c, 0x14);
    g.write::<u32>(s + 0x94, 0);
    let node = take(g, img, img.rel32_target(f + 0xae), w(0x57));
    let listed = g.read::<u32>(w(0xb4)) != 0;
    g.write::<u32>(node.wrapping_add(0x14), s);
    g.write::<u32>(node.wrapping_add(4), 0);
    if listed {
        let last = g.read::<u32>(w(0xc8));
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
        g.write::<u32>(w(0xd2), node);
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0xe1), node);
        g.write::<u32>(w(0xe6), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    Ret::eax(node)
}

/// [`fade_start`] (238 bytes, 1 overlay).
pub const FADE_START: Sig = Sig {
    len: 238,
    hash: 0x12a4_36d6_3af5_3e1d,
    calls: &[
        (0x49, Callee::Sig(&crate::s4_small::SCALE_TASK_1)),
        (0xae, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};
