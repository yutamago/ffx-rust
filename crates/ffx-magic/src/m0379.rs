//! `magic_0379.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Frame, Game, Image, Ret, Sig};

/// A node under the static parent `P` (0x10006a30, internal: `ecx` = the
/// node, cdecl `(x)`; `m0295::node_under`'s kin): its vtable (`+0x05`),
/// pass halfword 0xff, an empty child list, parent `P` (`+0x18`, `+0x1f`)
/// and the parent's last (`+0x24`) as its previous; the parent's first
/// (`+0x39`, when its last `+0x2c` is `P` itself) or old last's `+4` gets
/// the node, its count (`+0x45`) goes up and the node is its last
/// (`+0x4e`). Then `+0x3c` = x and `+0x2c..+0x38` 1.0. Returns x.
pub fn node_under<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let n = fr.ecx;
    g.write::<u32>(n, w(0x05));
    g.write::<u16>(n + 0x1a, 0xff);
    g.write::<u32>(n + 0x10, n);
    g.write::<u32>(n + 0xc, n);
    g.write::<u32>(n + 0x14, w(0x18));
    g.write::<u32>(n + 4, w(0x1f));
    g.write::<u32>(n + 8, g.read::<u32>(w(0x24)));
    let last = g.read::<u32>(w(0x2c));
    if last == w(0x31) {
        g.write::<u32>(w(0x39), n);
    } else {
        g.write::<u32>(last + 4, n);
    }
    let c = g.read::<u16>(w(0x45));
    g.write::<u16>(w(0x45), c.wrapping_add(1));
    let x = fr.arg(g, 0);
    g.write::<u32>(w(0x4e), n);
    g.write::<u32>(n + 0x3c, x);
    for off in [0x2c, 0x30, 0x34, 0x38] {
        g.write::<u32>(n + off, 0x3f80_0000);
    }
    Ret::eax(x)
}

/// [`node_under`] (115 bytes, 1 overlay).
pub const NODE_UNDER: Sig = Sig {
    len: 115,
    hash: 0x84e9_c8d2_4e03_8b17,
    calls: &[],
    imports: &[],
};
