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

/// A static node appended to the root list (0x10006e80, no arguments,
/// no frame; `m0262::static_node_6e90`'s kin): its vtable (`+0x07`),
/// links and pass byte, then `+0x72` its block, five words 0 and `+0xae`
/// 1.0. Returns the root list's old last.
pub fn static_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let last = g.read::<u32>(img.word(f + 0x01));
    crate::m0078::each_node(
        g,
        img,
        f,
        &crate::m0262::root_node(0x01, 0x1a, (0x07, 0x0b), 0x12, 0x62, (0x68, 0x6c)),
    );
    crate::m0078::stores(
        g,
        img,
        f,
        &[
            (0x72, 0x76),
            (0x7c, 0x80),
            (0x86, 0x8a),
            (0x90, 0x94),
            (0x9a, 0x9e),
            (0xa4, 0xa8),
            (0xae, 0xb2),
        ],
    );
    Ret::eax(last)
}

/// [`static_node`] (183 bytes, 1 overlay).
pub const STATIC_NODE: Sig = Sig {
    len: 183,
    hash: 0x9a6a_0442_d13b_9d43,
    calls: &[],
    imports: &[],
};

/// The model start (0x10007530, no arguments, a frame of 0x14 bytes;
/// `m0262::model_start`'s kin): the node at the ring's tail, the root's
/// flag cleared, the model group (`+0x51`, vtable `+0x77`) set up by
/// [`model_nodes`](crate::s3_summon::model_nodes) from `[node, parent,
/// *A, 0, flags 1]` (`A` `+0x40`); then the model whose number (`+0x0`
/// of the model nodes from `+0xaa`, 0x98 bytes apart, `*N` of them, `N`
/// `+0x99`) is slot 703 `(*X)`'s byte (`X` `+0x8b`): its node (`+0xd5`
/// on) goes into `*P` (`+0xda`). `+0x18` of the group (`+0xc1`/`+0xe0`/
/// `+0xee`) is cleared. Returns the found node, else the count searched.
pub fn model_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let tail = g.read::<u32>(w(0x07));
    g.write::<u32>(w(0x0c), tail);
    crate::m0078::stores(
        g,
        img,
        f,
        &[(0x12, 0x16), (0x1c, 0x20), (0x26, 0x2a), (0x30, 0x34)],
    );
    g.write::<u32>(tail + 4, w(0x3b));
    let a = g.read::<u32>(w(0x40));
    let fl = g.read::<u32>(w(0x46));
    g.write::<u32>(w(0x46), fl & !1);
    crate::m0078::stores(g, img, f, &[(0x51, 0x55)]);
    let ebp = fr.esp.wrapping_sub(4);
    let rec = ebp.wrapping_sub(0x14);
    for (k, v) in (0u32..).zip([w(0x5c), w(0x63), a, 0]) {
        g.write::<u32>(rec + 4 * k, v);
    }
    g.write::<u8>(rec + 0x10, 1);
    crate::m0078::stores(g, img, f, &[(0x77, 0x7b)]);
    // `push esi` under the frame: the call's esp
    crate::s3_summon::model_nodes(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x80),
            esp: ebp.wrapping_sub(0x1c),
            ecx: rec,
            ..Frame::default()
        },
    );
    let s = crate::runtime::slot_target(g, w(0x85), 0xafc / 4);
    let x = g.read::<u32>(w(0x8b));
    let number = g.call(s, 0, &[x]).eax & 0xff;
    let n = g.read::<i32>(w(0x99));
    let mut i = 0i32;
    while i < n {
        let at = w(0xaa).wrapping_add(0x98 * i.cast_unsigned());
        if g.read::<u32>(at) == number {
            let node = (i.cast_unsigned()).wrapping_mul(0x98).wrapping_add(w(0xd5));
            g.write::<u32>(w(0xda), node);
            g.write::<u32>(w(0xe0), 0);
            return Ret::eax(node);
        }
        i += 1;
    }
    let off = if n > 0 { 0xc1 } else { 0xee };
    g.write::<u32>(w(off), 0);
    Ret::eax(i.max(0).cast_unsigned())
}

/// [`model_start`] (247 bytes, 1 overlay).
pub const MODEL_START: Sig = Sig {
    len: 247,
    hash: 0xffff_ba77_e2ef_3278,
    calls: &[(0x80, crate::Callee::Sig(&crate::s3_summon::MODEL_NODES))],
    imports: &[],
};
