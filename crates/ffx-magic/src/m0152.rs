//! `magic_0152.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::m0042::{NODE_INIT_BIG, node_init_big};
use crate::runtime::slot_target;
use crate::{Callee, Game, Image, Sig};

/// Builds an effect node's three children (0x10003720, internal: `ecx`
/// = the [`m0042::build`](crate::m0042::build) record `[node, parent, d,
/// _, _, _, a, b, c, e]`): as `build` does (the children `+0x1d4`,
/// `+0x1a8`, `+0x17c` with the vtables at operands `+0x16`, `+0x6c`,
/// `+0x90`, [`node_init_big`], the three linked under the node, the
/// node's words), then slot 287 `(1, 0, 7)` gives a handle into `+0x170`,
/// slot 283 `(handle, 0)` and `(handle, 1)`, and the node takes the
/// vtable `+0x1db`. Returns the node.
///
/// Each slot call passes the words the original leaves on the stack.
pub fn build_287<G: Game>(g: &mut G, img: &Image, f: u32, args: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let node = g.read::<u32>(args);
    let (c1, c2, c3) = (node + 0x1d4, node + 0x1a8, node + 0x17c);
    g.write::<u32>(c1, w(0x16));
    g.write::<u16>(node + 0x1ee, 0xff);
    g.write::<u32>(node + 0x1e8, 0);
    g.write::<u32>(c1 + 0x10, c1);
    g.write::<u32>(c1 + 0xc, c1);
    g.write::<u32>(args + 0xc, c3);
    g.write::<u32>(args + 0x10, c1);
    for (c, t) in [(c2, 0x6cu32), (c3, 0x90)] {
        g.write::<u16>(c + 0x1a, 0xff);
        g.write::<u32>(c + 0x10, c);
        g.write::<u32>(c + 0xc, c);
        g.write::<u32>(c + 0x14, 0);
        g.write::<u32>(c, w(t));
    }
    g.write::<u32>(args + 0x14, c2);
    node_init_big(g, args);
    for c in [c1, c2, c3] {
        g.write::<u32>(c + 4, node);
        let last = g.read::<u32>(node + 0x10);
        g.write::<u32>(c + 8, last);
        let last = g.read::<u32>(node + 0x10);
        g.write::<u32>(last + 4, c);
        g.write::<u32>(node + 0x10, c);
        g.write::<u32>(c + 0x14, node);
        let n = g.read::<u16>(node + 0x18).wrapping_add(1);
        g.write::<u16>(node + 0x18, n);
    }
    for (dst, src) in [
        (0x15cu32, 0x1cu32),
        (0x164, 0x18),
        (0x160, 0x24),
        (0x168, 0x20),
    ] {
        let v = g.read::<u32>(args + src);
        g.write::<u32>(node + dst, v);
    }
    for (o, v) in [
        (0x1f8u32, 0u32),
        (0x1f4, 0),
        (0x1f0, 0),
        (0x1fc, 1),
        (0x1a0, 0),
        (0x19c, 0),
        (0x198, 0),
        (0x1a4, 0x3f80_0000),
        (0x1d0, 0x3f80_0000),
        (0x1cc, 0x3f80_0000),
        (0x1c8, 0x3f80_0000),
        (0x1c4, 0x3f80_0000),
    ] {
        g.write::<u32>(node + o, v);
    }
    g.write::<u8>(node + 0x178, 0);
    let s = slot_target(g, w(0x193), 287);
    let h = g.call(s, 0, &[1, 0, 7]).eax;
    g.write::<u32>(node + 0x170, h);
    let s = slot_target(g, w(0x1b3), 283);
    g.call(s, 0, &[h, 0]);
    let h2 = g.read::<u32>(node + 0x170);
    let s = slot_target(g, w(0x1c0), 283);
    g.call(s, 0, &[h2, 1, h, 0]);
    let node = g.read::<u32>(args);
    g.write::<u32>(node, w(0x1db));
    node
}

/// [`build_287`] (486 bytes, 1 overlay).
pub const BUILD_287: Sig = Sig {
    len: 486,
    hash: 0x5486_bb04_f22e_9782,
    calls: &[(0x98, Callee::Sig(&NODE_INIT_BIG))],
    imports: &[],
};

/// Builds a node with [`build_287`] (0x10008170, internal: `ecx` the
/// node, the stack words 4 and 6 `a`, `b`): the record `[node, V1, V2,
/// 0, 0, 0, a, V3, b, V2]` (the vtables at operands `+0x18`, `+0x1f`,
/// `+0x3b`, `+0x45`) is a local where the original's frame has it
/// (`esp - 0x2c`). Returns the node.
pub fn build_node<G: Game>(g: &mut G, img: &Image, f: u32, node: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let a = g.read::<u32>(esp + 0x14);
    let b = g.read::<u32>(esp + 0x1c);
    let rec = esp.wrapping_sub(0x2c);
    let words = [node, w(0x18), w(0x1f), 0, 0, 0, a, w(0x3b), b, w(0x45)];
    for (i, v) in (0u32..).zip(words) {
        g.write::<u32>(rec + 4 * i, v);
    }
    build_287(g, img, img.rel32_target(f + 0x4a), rec)
}

/// [`build_node`] (82 bytes, 1 overlay).
pub const BUILD_NODE: Sig = Sig {
    len: 82,
    hash: 0x6c70_0d28_44d4_8098,
    calls: &[(0x4a, Callee::Sig(&BUILD_287))],
    imports: &[],
};
