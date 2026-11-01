//! `magic_0586.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Starts the effect's fade (0x10002ad0, an overlay table entry, no
/// arguments): the scale task of `*G_FX + 0x83c8`
/// ([`scale_task_edx`](crate::s4_small::scale_task_edx): `edx` = 30,
/// `xmm1`/`xmm2` the constants `+0xc`/`+0x4`; the entry's `ecx` pushed
/// under it), then the fade state `S = *G_FX + 0x8330` = `(0, 100, 0, 0,
/// 150, 150)` (`+0x10` and `+0x14` 150) with the half-word `+0x18` 0. A
/// pool node ([`take`](crate::sa_pool::take), owner the immediate
/// `+0x34`) runs it (`+0x14` = `S`, byte `+0x18` 1, `+0x4` 0), appended
/// to the list (tail `+0x6c`/`+0x84`, head `+0x96`; `+0x9e` the new tail).
/// Last slot 371 `(0, 0x10000)` and `(1, 0x8000)`, whose `eax` it
/// returns.
pub fn fade_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x13));
    // push esi, ecx, the return address
    let esp = fr.esp.wrapping_sub(0xc);
    g.write::<u32>(esp + 4, fr.ecx);
    let mut xmm = fr.xmm;
    xmm[2] = u128::from(g.read::<u32>(w(0x4)));
    xmm[1] = u128::from(g.read::<u32>(w(0xc)));
    let task = Frame {
        at: img.rel32_target(f + 0x24),
        esp,
        ecx: fx.wrapping_add(0x83c8),
        edx: 0x1e,
        xmm,
        ..*fr
    };
    crate::s4_small::scale_task_edx(g, img, &task);
    let s = fx.wrapping_add(0x8330);
    for (o, v) in [
        (0, 0),
        (4, 0x64),
        (8, 0),
        (0xc, 0),
        (0x14, 0x96),
        (0x10, 0x96),
    ] {
        g.write::<u32>(s + o, v);
    }
    g.write::<u16>(s + 0x18, 0);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x66), w(0x34));
    let tail = g.read::<u32>(w(0x6c));
    g.write::<u32>(node.wrapping_add(0x14), s);
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(4), 0);
    if tail == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x96), node);
    } else {
        let last = g.read::<u32>(w(0x84));
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x9e), node);
    let t = slot_target(g, w(0xa3), 371);
    g.call(t, 0, &[0, 0x1_0000]);
    let t = slot_target(g, w(0xb7), 371);
    Ret::eax(g.call(t, 0, &[1, 0x8000, 0, 0x1_0000]).eax)
}

/// [`fade_start`] (206 bytes, 1 overlay).
pub const FADE_START: Sig = Sig {
    len: 206,
    hash: 0x7443_ef39_e0c1_4f81,
    calls: &[
        (0x24, Callee::Sig(&crate::s4_small::SCALE_TASK_EDX)),
        (0x66, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};
