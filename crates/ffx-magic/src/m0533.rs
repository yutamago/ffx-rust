//! `magic_0533.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Sig};

/// A node's own list fields, as the DLL writes them: pass byte `+0x1a`
/// (a word) 0xff, an empty child list (`+0x10`, `+0xc` itself) and no
/// list (`+0x14` 0).
fn lone<G: Game>(g: &mut G, node: u32) {
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, 0);
}

/// `node` appended to `obj`'s child list (no empty-list case: the list
/// object is its own last node when empty).
fn append<G: Game>(g: &mut G, node: u32, obj: u32) {
    g.write::<u32>(node + 4, obj);
    let last = g.read::<u32>(obj + 0x10);
    g.write::<u32>(node + 8, last);
    let last = g.read::<u32>(obj + 0x10);
    g.write::<u32>(last + 4, node);
    g.write::<u32>(obj + 0x10, node);
    g.write::<u32>(node + 0x14, obj);
    let n = g.read::<u16>(obj + 0x18);
    g.write::<u16>(obj + 0x18, n.wrapping_add(1));
}

/// Sets up a model object (0x10003720, internal: `ecx` = the group
/// `[obj, parent, d, a, b, c, e, f, g, h]`): three child nodes of the
/// object, `n3 = obj + 0x1d4` (handlers `+0x16`), `n2 = obj + 0x1a8` and
/// `n1 = obj + 0x17c` (both `+0x6c`/`+0x90`), the group's `+0xc..+0x14`
/// set to `n1, n3, n2` for the big node init
/// ([`node_init_big`](crate::m0042::node_init_big)) of the object; then
/// `n3`, `n2`, `n1` appended to the object; `+0x15c`, `+0x164`, `+0x160`,
/// `+0x168` = the group's `+0x1c`, `+0x18`, `+0x24`, `+0x20`; the
/// object's counters and scales reset; slot 281 `(1)`, `+0x170` = slot
/// 287 `(1, 0, 7)` and slot 283 `(h, 0)`, `(h, 1)`; the object's
/// handlers `+0x1ed` last. Returns the object.
pub fn model_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let group = fr.ecx;
    let obj = g.read::<u32>(group);
    let n3 = obj + 0x1d4;
    g.write::<u32>(n3, w(0x16));
    g.write::<u16>(n3 + 0x1a, 0xff);
    g.write::<u32>(obj + 0x1e8, 0);
    g.write::<u32>(n3 + 0x10, n3);
    g.write::<u32>(n3 + 0xc, n3);
    g.write::<u32>(group + 0xc, obj + 0x17c);
    g.write::<u32>(group + 0x10, n3);
    let n2 = obj + 0x1a8;
    lone(g, n2);
    g.write::<u32>(n2, w(0x6c));
    let n1 = obj + 0x17c;
    lone(g, n1);
    g.write::<u32>(n1, w(0x90));
    g.write::<u32>(group + 0x14, n2);
    crate::m0042::node_init_big(g, group);
    append(g, n3, obj);
    append(g, n2, obj);
    append(g, n1, obj);
    for (src, dst) in [(0x1cu32, 0x15cu32), (0x18, 0x164), (0x24, 0x160)] {
        let v = g.read::<u32>(group + src);
        g.write::<u32>(obj + dst, v);
    }
    let e = g.read::<u32>(group + 0x20);
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
        g.write::<u32>(obj + o, v);
    }
    g.write::<u8>(obj + 0x178, 0);
    g.write::<u32>(obj + 0x168, e);
    let t = slot_target(g, w(0x193), 281);
    g.call(t, 0, &[1]);
    let t = slot_target(g, w(0x1a0), 287);
    let h = g.call(t, 0, &[1, 0, 7]).eax;
    g.write::<u32>(obj + 0x170, h);
    let t = slot_target(g, w(0x1c5), 283);
    let mut stack = vec![h, 0];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x1d2), 283);
    let h = g.read::<u32>(obj + 0x170);
    stack.splice(0..0, [h, 1]);
    g.call(t, 0, &stack);
    let obj = g.read::<u32>(group);
    g.write::<u32>(obj, w(0x1ed));
    obj
}

/// [`model_setup`] (504 bytes, 1 overlay).
pub const MODEL_SETUP: Sig = Sig {
    len: 504,
    hash: 0x468b_d541_c583_509d,
    calls: &[(0x98, Callee::Sig(&crate::m0042::NODE_INIT_BIG))],
    imports: &[],
};
