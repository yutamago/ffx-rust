//! `magic_0263.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Frame, Game, Image, Ret, Sig};

/// A node's set-up (0x10006dc0, `ecx` = the node, cdecl `(a)`): the pass
/// `+0x1a` 0xff, its own child ring (`+0x0c`, `+0x10`), the parent `R`
/// (the immediate `+0x12`, also `+0x04`), linked at the end of `R`'s
/// list (the last child `*L`, `+0x1e`; the first `*F` (`+0x33`) when the
/// list was empty, else the old last's `+0x04`), the count `*N` (`+0x3f`,
/// a halfword) up and `*L` = the node; `a` into `+0x2c`, the immediate
/// `+0x52` into `+0x30` and the vtable (`+0x58`). Returns `a`.
pub fn node_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let n = fr.ecx;
    g.write::<u16>(n + 0x1a, 0xff);
    g.write::<u32>(n + 0x10, n);
    g.write::<u32>(n + 0xc, n);
    g.write::<u32>(n + 0x14, w(0x12));
    g.write::<u32>(n + 4, w(0x19));
    let last = g.read::<u32>(w(0x1e));
    g.write::<u32>(n + 8, last);
    let last = g.read::<u32>(w(0x26));
    if last == w(0x2b) {
        g.write::<u32>(w(0x33), n);
    } else {
        g.write::<u32>(last + 4, n);
    }
    let c = g.read::<u16>(w(0x3f));
    g.write::<u16>(w(0x3f), c.wrapping_add(1));
    let a = fr.arg(g, 0);
    g.write::<u32>(w(0x48), n);
    g.write::<u32>(n + 0x2c, a);
    g.write::<u32>(n + 0x30, w(0x52));
    g.write::<u32>(n, w(0x58));
    Ret::eax(a)
}

/// [`node_setup`] (94 bytes, 1 overlay).
pub const NODE_SETUP: Sig = Sig {
    len: 94,
    hash: 0xd0be_bccf_5323_4ea5,
    calls: &[],
    imports: &[],
};
