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

/// A node leaves its parent and registers its handles (0x10001e80,
/// internal: `ecx` = a cursor `[node]`): with a parent (`+0x14`), its
/// count `u16 +0x18` goes down, the sibling links are joined (the
/// parent's first `+0x10` / last `+0xc` where the node was one), the
/// node's links point to itself and `+0x14` is 0. Then for the two lists
/// (`u16 +0x2c`/`+0x2e` entries of 0x30 bytes from `node[0x24]`/`[0x28]`
/// plus 0x10, the count read again each time) each entry `e` goes to slot
/// 779 `(e, e >> 31)` with `*G_LOCK` (operand `+0x6b`/`+0x90`) 1 around
/// the call. Returns `node + 0x30`.
pub fn leave_and_register<G: Game>(g: &mut G, img: &Image, f: u32, cur: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let node = g.read::<u32>(cur);
    let parent = g.read::<u32>(node + 0x14);
    if parent != 0 {
        let n = g.read::<u16>(parent + 0x18);
        g.write::<u16>(parent + 0x18, n.wrapping_add(0xffff));
        let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
        if prev == parent {
            g.write::<u32>(parent + 0x10, next);
        } else {
            g.write::<u32>(prev + 8, next);
        }
        let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
        if next == parent {
            g.write::<u32>(parent + 0xc, prev);
        } else {
            g.write::<u32>(next + 4, prev);
        }
        g.write::<u32>(node + 8, node);
        g.write::<u32>(node + 4, node);
        g.write::<u32>(node + 0x14, 0);
    }
    for k in 0..2u32 {
        let count = node + 0x2c + 2 * k;
        let mut e = g.read::<u32>(node + 0x24 + 4 * k).wrapping_add(0x10);
        let mut lock = g.read::<u32>(w(0x6b));
        let mut j = 0u32;
        while j < u32::from(g.read::<u16>(count)) {
            let old = g.read::<u32>(lock);
            g.write::<u32>(lock, 1);
            let t = crate::runtime::slot_target(g, w(0x7c), 779);
            g.call(t, 0, &[e, (e.cast_signed() >> 31).cast_unsigned()]);
            lock = g.read::<u32>(w(0x90));
            g.write::<u32>(lock, old);
            j += 1;
            e = e.wrapping_add(0x30);
        }
    }
    node + 0x30
}

/// [`leave_and_register`] (201 bytes, 1 overlay).
pub const LEAVE_AND_REGISTER: Sig = Sig {
    len: 201,
    hash: 0x8e05_d43b_deb5_4449,
    calls: &[],
    imports: &[],
};
