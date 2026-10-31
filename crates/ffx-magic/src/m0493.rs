//! `magic_0493.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::l3_common::{ListAt, attach, node_init};
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Sets up the effect object (0x10006ae0, internal: `ecx` = the object,
/// `edx` = the effect, `xmm2` a float; a frame with locals). The object
/// becomes a task (vtable `+0x18`, callback `+0x1f`) appended to the
/// task ring (tail `+0x27`, the ring's flag bit cleared); its node
/// `+0x580` joins the node list (vtable `+0xa7`); slot 268 `(e[0x54],
/// 0x11, v)` fills `v = +0x59c` (`w` 1.0); the effect node
/// ([`effect_node`](crate::sb_nodes::effect_node)) of the record
/// `[+0xaf0, LIST, NAME, node, DATA, 0]` in the frame at `ebp - 0x20`
/// gets the vtable `+0x106`, flag bit 0 of `+0x2c` and `+0x30` 0; the
/// float to `+0xae0`, the effect to `+0xaec`, `+0xadc..+0xae8` 0.
/// Returns the effect.
pub fn init<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let (obj, e) = (fr.ecx, fr.edx);
    let x = fr.xmm[2] as u32;
    g.write::<u32>(obj + 8, obj);
    g.write::<u32>(obj + 4, obj);
    g.write::<u32>(obj, w(0x18));
    g.write::<u32>(obj + 0x10, w(0x1f));
    g.write::<u32>(obj + 0x14, obj);
    let tail = g.read::<u32>(w(0x27));
    g.write::<u32>(obj + 8, tail);
    g.write::<u32>(obj + 4, w(0x31));
    let tail = g.read::<u32>(w(0x36));
    g.write::<u32>(tail + 4, obj);
    let flags = g.read::<u32>(w(0x45));
    g.write::<u32>(w(0x45), flags & !1);
    g.write::<u32>(w(0x4c), obj);
    let b = obj + 0x580;
    node_init(g, b, w(0x5f));
    let at = ListAt {
        last: w(0x6b),
        last2: w(0x73),
        sentinel: w(0x7b),
        first: w(0x83),
        count: w(0x8f),
        last_w: w(0xa0),
    };
    attach(g, b, w(0x66), at);
    let t = slot_target(g, w(0x94), 268);
    g.write::<u32>(b, w(0xa7));
    let v = obj + 0x59c;
    let m = g.read::<u32>(e + 0x54);
    g.call(t, 0, &[m, 0x11, v]);
    g.write::<u32>(v + 0xc, ONE);
    g.write::<u32>(obj + 0x5a8, ONE);
    let rec = ebp - 0x20;
    let node = obj + 0xaf0;
    for (i, v) in (0u32..).zip([node, w(0xdb), w(0xe2), b, w(0xec), 0]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0xf8), rec);
    let n = g.read::<u32>(rec);
    g.write::<u32>(n, w(0x106));
    let fl = g.read::<u32>(node + 0x2c);
    g.write::<u32>(node + 0x2c, fl | 1);
    g.write::<u32>(node + 0x30, 0);
    g.write::<u32>(obj + 0xae0, x);
    g.write::<u32>(obj + 0xaec, e);
    for off in [0xadc, 0xae4, 0xae8] {
        g.write::<u32>(obj + off, 0);
    }
    e
}

/// [`init`] (331 bytes, 1 overlay).
pub const INIT: Sig = Sig {
    len: 331,
    hash: 0x5824_fc5c_202e_7a1a,
    calls: &[(0xf8, Callee::Sig(&crate::sb_nodes::EFFECT_NODE))],
    imports: &[],
};
