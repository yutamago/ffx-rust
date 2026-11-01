//! `magic_0295.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Frame, Game, Image, Ret, Sig};

/// A node under the static parent `P` (0x100074e0, internal: `ecx` =
/// the node; no frame): `*G` (`+0x02`) read first; the node's vtable
/// (`+0x08`), pass halfword 0xff, an empty child list, parent `P`
/// (`+0x1b`, `+0x22`) and the parent's last (`+0x27`) as its previous;
/// the parent's first (`+0x3c`, when its last `+0x2f` is `P` itself) or
/// old last's `+4` gets the node, its count (`+0x48`) goes up and the
/// node is its last (`+0x4e`). Then `+0x28` 1.0, `+0x2c` = `*G`, `+0x30`
/// 13. Returns the old last.
pub fn node_under<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let n = fr.ecx;
    let v = g.read::<u32>(w(0x02));
    g.write::<u32>(n, w(0x08));
    g.write::<u16>(n + 0x1a, 0xff);
    g.write::<u32>(n + 0x10, n);
    g.write::<u32>(n + 0xc, n);
    g.write::<u32>(n + 0x14, w(0x1b));
    g.write::<u32>(n + 4, w(0x22));
    g.write::<u32>(n + 8, g.read::<u32>(w(0x27)));
    let last = g.read::<u32>(w(0x2f));
    if last == w(0x34) {
        g.write::<u32>(w(0x3c), n);
    } else {
        g.write::<u32>(last + 4, n);
    }
    let c = g.read::<u16>(w(0x48));
    g.write::<u16>(w(0x48), c.wrapping_add(1));
    g.write::<u32>(w(0x4e), n);
    g.write::<u32>(n + 0x28, 0x3f80_0000);
    g.write::<u32>(n + 0x2c, v);
    g.write::<u32>(n + 0x30, 0xd);
    Ret::eax(last)
}

/// [`node_under`] (100 bytes, 1 overlay).
pub const NODE_UNDER: Sig = Sig {
    len: 100,
    hash: 0x24ae_e7a2_3801_5f54,
    calls: &[],
    imports: &[],
};
