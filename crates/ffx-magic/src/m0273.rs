//! `magic_0273.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::m0078::stores;
use crate::{Frame, Game, Image, Ret, Sig};

/// A static node appended to its list (0x10007600, no arguments, no
/// frame): its vtable (`+0x07`), pass halfword (`+0x12`), links and
/// parent (`+0x1a`.. `+0x38`), the old last (`+0x01`) as its previous;
/// an empty list (last = the sentinel `+0x46`) gets it as first
/// (`+0x55`), else it follows the old last (`+0x6b`); the u16 count
/// (`+0x4f`/`+0x72`) goes up and it becomes the last (`+0x5f`/`+0x78`).
/// Returns the old last.
pub fn static_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let last = g.read::<u32>(w(0x01));
    stores(g, img, f, &[(0x07, 0x0b)]);
    g.write::<u16>(w(0x12), 0xff);
    stores(
        g,
        img,
        f,
        &[(0x1a, 0x1e), (0x24, 0x28), (0x2e, 0x32), (0x38, 0x3c)],
    );
    g.write::<u32>(w(0x41), last);
    let count = |g: &mut G, at: u32| {
        let c = g.read::<u16>(w(at));
        g.write::<u16>(w(at), c.wrapping_add(1));
    };
    if last == w(0x46) {
        count(g, 0x4f);
        stores(g, img, f, &[(0x55, 0x59), (0x5f, 0x63)]);
    } else {
        g.write::<u32>(last + 4, w(0x6b));
        count(g, 0x72);
        stores(g, img, f, &[(0x78, 0x7c)]);
    }
    Ret::eax(last)
}

/// [`static_node`] (129 bytes, 1 overlay).
pub const STATIC_NODE: Sig = Sig {
    len: 129,
    hash: 0xbc72_a8f4_7b65_6913,
    calls: &[],
    imports: &[],
};

/// A node put at the ring's tail (0x10008170, internal: one stack word
/// `x` and `edx`): the ring's last (`+0x04`) becomes the node's previous
/// (`+0x09`), the node's words (`+0x0f`.. `+0x2d`), the old last's `+4`
/// the node; the root's flag (`+0x41`) loses bit 0, the node is the last
/// (`+0x48`); `+0x51` = x, `+0x57` = `edx`, `+0x5d` 0. Returns x.
pub fn tail_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let last = g.read::<u32>(w(0x04));
    g.write::<u32>(w(0x09), last);
    stores(
        g,
        img,
        f,
        &[(0x0f, 0x13), (0x19, 0x1d), (0x23, 0x27), (0x2d, 0x31)],
    );
    g.write::<u32>(last + 4, w(0x38));
    let x = fr.arg(g, 0);
    let fl = g.read::<u32>(w(0x41));
    g.write::<u32>(w(0x41), fl & !1);
    stores(g, img, f, &[(0x48, 0x4c)]);
    g.write::<u32>(w(0x51), x);
    g.write::<u32>(w(0x57), fr.edx);
    g.write::<u32>(w(0x5d), 0);
    Ret::eax(x)
}

/// [`tail_node`] (103 bytes, 1 overlay).
pub const TAIL_NODE: Sig = Sig {
    len: 103,
    hash: 0x0dca_b6d2_bb66_bf55,
    calls: &[],
    imports: &[],
};
