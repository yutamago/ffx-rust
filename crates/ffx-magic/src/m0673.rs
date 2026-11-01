//! `magic_0673.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::o4_more::push_head;
use crate::sa_pool::take;
use crate::sb_sse::{cvt, div};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Starts a fade entry (0x10001dd0, an overlay table entry, `(ctx)`; 18
/// table entries): with `k = ctx[0x38]`, the template `E` (32 bytes at
/// the immediate `+0x1d` + 32 k) and the record `R = *G_R + 0x59f8 +
/// 48 k` (`+0x23`): `R[0x28]` = the frame count `E[0x18]`, the step
/// `R[0x20]` = `K / count` (`+0x0a`), `+0x0`, `+0x1c`, `+0x24` 0 and the
/// flags halfword `+0x2c` 0; a pool node ([`take`], the owner the
/// immediate at `+0x41`; byte `+0x18` 1, `+0x14` = `R`, `+0x4` 0) at the
/// head of the list ([`push_head`]: the empty test `+0x6e`, head
/// `+0x85`/`+0x9f`, set `+0x97`/`+0x9f`); `R[0x4..0x18]` = `E[0..0x14]`,
/// flags 3, `R[0x24]` = `E[0x1c]`. Returns `E[0x1c]`.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ctx = fr.arg(g, 0);
    let k = g.read::<u32>(ctx + 0x38);
    let e = (k << 5).wrapping_add(w(0x1d));
    let r = g
        .read::<u32>(w(0x23))
        .wrapping_add(0x59f8)
        .wrapping_add(k.wrapping_mul(0x30));
    let count = g.read::<u32>(e.wrapping_add(0x18));
    let x = div(g.read::<f32>(w(0xa)), cvt(count));
    g.write::<u32>(r + 0x28, count);
    g.write::<u32>(r, 0);
    g.write::<u32>(r + 0x1c, 0);
    g.write::<f32>(r + 0x20, x);
    g.write::<u32>(r + 0x24, 0);
    g.write::<u16>(r + 0x2c, 0);
    let node = take(g, img, img.rel32_target(f + 0x68), w(0x41));
    let empty = g.read::<u32>(w(0x6e)) == 0;
    g.write::<u8>(node + 0x18, 1);
    g.write::<u32>(node + 0x14, r);
    g.write::<u32>(node + 4, 0);
    push_head(g, node, empty, (w(0x85), w(0x9f)), (w(0x97), w(0x9f)));
    for i in 0..6 {
        let v = g.read::<u32>(e.wrapping_add(4 * i));
        g.write::<u32>(r + 4 + 4 * i, v);
    }
    let fl = g.read::<u16>(r + 0x2c);
    g.write::<u16>(r + 0x2c, fl | 3);
    let v = g.read::<u32>(e.wrapping_add(0x1c));
    g.write::<u32>(r + 0x24, v);
    Ret::eax(v)
}

/// [`start`] (213 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 213,
    hash: 0xba7f_f0b5_06ac_63d1,
    calls: &[(0x68, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};
