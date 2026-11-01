//! `magic_0262.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::m0078::{EachNode, each_node, stores};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A node leaves its parent (`node + 0x14`; nothing when there is none):
/// the parent's count `+0x18` drops, the node's `+4`/`+8` neighbours are
/// joined (the parent's first `+0xc` / last `+0x10` where the node was
/// at an end), and the node is left alone (`+4`, `+8` itself, no parent).
/// Returns the `eax` the code leaves: the last neighbour word read, or
/// `absent` without a parent.
fn leave<G: Game>(g: &mut G, node: u32, absent: u32) -> u32 {
    let parent = g.read::<u32>(node + 0x14);
    if parent == 0 {
        return absent;
    }
    let n = g.read::<u16>(parent + 0x18);
    g.write::<u16>(parent + 0x18, n.wrapping_add(0xffff));
    let prev = g.read::<u32>(node + 4);
    let next = g.read::<u32>(node + 8);
    if prev == parent {
        g.write::<u32>(parent + 0x10, next);
    } else {
        g.write::<u32>(prev + 8, next);
    }
    let next = g.read::<u32>(node + 8);
    let prev = g.read::<u32>(node + 4);
    if next == parent {
        g.write::<u32>(parent + 0xc, prev);
    } else {
        g.write::<u32>(next + 4, prev);
    }
    prev
}

/// The model nodes leave their parents (0x10006290, internal: `ecx` =
/// `[group]`): each of the group's `+0x4dc` model nodes (`+0x1c + 0x98
/// i`, the count read again each time) that has a parent leaves it and
/// is reset (links itself, `+0x14` 0), then the group node itself (whose
/// reset needs a parent too). Returns the last word the code read into
/// `eax` (the caller's when nothing ran).
pub fn models_leave<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let grp = g.read::<u32>(fr.ecx);
    let mut eax = fr.eax;
    if g.read::<i32>(grp + 0x4dc) > 0 {
        let mut i = 0u32;
        loop {
            let node = grp + 0x1c + 0x98 * i;
            if g.read::<u32>(node + 0x14) != 0 {
                leave(g, node, 0);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                g.write::<u32>(node + 0x14, 0);
            }
            i += 1;
            if i.cast_signed() >= g.read::<i32>(grp + 0x4dc) {
                break;
            }
        }
        // `eax` walked the nodes' `+4` words, one past the last
        eax = grp + 0x20 + 0x98 * i;
    }
    if g.read::<u32>(grp + 0x14) == 0 {
        return Ret::eax(eax);
    }
    let eax = leave(g, grp, eax);
    g.write::<u32>(grp + 8, grp);
    g.write::<u32>(grp + 4, grp);
    g.write::<u32>(grp + 0x14, 0);
    Ret::eax(eax)
}

/// [`models_leave`] (181 bytes, 1 overlay).
pub const MODELS_LEAVE: Sig = Sig {
    len: 181,
    hash: 0x4ad9_f749_ffcf_6e2c,
    calls: &[],
    imports: &[],
};

/// The root list's static nodes these functions append (`m0078::each_node`'s
/// operands: the list's last at `last_r`, the node's stores from `base`;
/// the vtable store before or after them).
pub(crate) const fn root_node(
    last_r: u32,
    base: u32,
    vt: (u32, u32),
    half: u32,
    count: u32,
    last_w: (u32, u32),
) -> EachNode {
    EachNode {
        last_r,
        half,
        st: [
            (base, base + 4),
            (base + 0xa, base + 0xe),
            (base + 0x14, base + 0x18),
            (base + 0x1e, base + 0x22),
        ],
        last_w1: base + 0x27,
        sentinel: base + 0x2c,
        first: (base + 0x34, base + 0x38),
        next: base + 0x41,
        count,
        last_w,
        vt,
    }
}

/// A static node appended to the root list (0x10006d20, cdecl `(x)`):
/// its vtable (`+0x0a`), links, pass byte; then the words `+0x78` (1.0),
/// `+0x81` = x and `+0x87` (15). Returns x.
pub fn static_node_6d20<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    each_node(
        g,
        img,
        f,
        &root_node(0x04, 0x1d, (0x0a, 0x0e), 0x15, 0x68, (0x6e, 0x72)),
    );
    let x = fr.arg(g, 0);
    stores(g, img, f, &[(0x78, 0x7c), (0x87, 0x8b)]);
    g.write::<u32>(img.word(f + 0x81), x);
    Ret::eax(x)
}

/// [`static_node_6d20`] (145 bytes, 1 overlay).
pub const STATIC_NODE_6D20: Sig = Sig {
    len: 145,
    hash: 0x9ea4_f7e8_0598_a77c,
    calls: &[],
    imports: &[],
};

/// A static node appended to the root list (0x10006dc0, cdecl `(x)`):
/// links and pass byte, `+0x6d` = x, `+0x73` its block, then its vtable
/// (`+0x7d`). Returns x.
pub fn static_node_6dc0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    each_node(
        g,
        img,
        f,
        &root_node(0x04, 0x13, (0x7d, 0x81), 0x0b, 0x5e, (0x64, 0x68)),
    );
    let x = fr.arg(g, 0);
    g.write::<u32>(img.word(f + 0x6d), x);
    stores(g, img, f, &[(0x73, 0x77)]);
    Ret::eax(x)
}

/// [`static_node_6dc0`] (135 bytes, 1 overlay).
pub const STATIC_NODE_6DC0: Sig = Sig {
    len: 135,
    hash: 0xaccc_00c9_4d8d_9cd4,
    calls: &[],
    imports: &[],
};

/// A static node appended to the root list (0x10006e90, cdecl `(x)`):
/// its vtable (`+0x0a`), links, pass byte; `+0x77` = x, five words 0 and
/// `+0xaf` 1.0. Returns x.
pub fn static_node_6e90<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    each_node(
        g,
        img,
        f,
        &root_node(0x04, 0x1d, (0x0a, 0x0e), 0x15, 0x68, (0x6e, 0x72)),
    );
    let x = fr.arg(g, 0);
    g.write::<u32>(img.word(f + 0x77), x);
    stores(
        g,
        img,
        f,
        &[
            (0x7d, 0x81),
            (0x87, 0x8b),
            (0x91, 0x95),
            (0x9b, 0x9f),
            (0xa5, 0xa9),
            (0xaf, 0xb3),
        ],
    );
    Ret::eax(x)
}

/// [`static_node_6e90`] (185 bytes, 1 overlay).
pub const STATIC_NODE_6E90: Sig = Sig {
    len: 185,
    hash: 0x1da2_5646_765b_9c25,
    calls: &[],
    imports: &[],
};

/// The model start (0x10007440, no arguments, a frame of 0x18 bytes): a
/// node put at the ring's tail (`+0x07`..`+0x3b`), the root's flag
/// cleared (`+0x46`), the model group (`+0x50`, vtable `+0x76`) set up by
/// [`model_nodes`](crate::s3_summon::model_nodes) from a record on the
/// frame `[node, parent, *A, 0, flags 1]` (`A` `+0x40`), then `+0x85` =
/// 0. Returns the model nodes' `eax`.
pub fn model_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let tail = g.read::<u32>(w(0x07));
    g.write::<u32>(w(0x0c), tail);
    stores(
        g,
        img,
        f,
        &[(0x12, 0x16), (0x1c, 0x20), (0x26, 0x2a), (0x30, 0x34)],
    );
    g.write::<u32>(tail + 4, w(0x3b));
    let a = g.read::<u32>(w(0x40));
    let fl = g.read::<u32>(w(0x46));
    g.write::<u32>(w(0x46), fl & !1);
    stores(g, img, f, &[(0x50, 0x54)]);
    let ebp = fr.esp.wrapping_sub(4);
    let rec = ebp.wrapping_sub(0x14);
    for (k, v) in (0u32..).zip([w(0x5b), w(0x62), a, 0]) {
        g.write::<u32>(rec + 4 * k, v);
    }
    g.write::<u8>(rec + 0x10, 1);
    stores(g, img, f, &[(0x76, 0x7a)]);
    let eax = crate::s3_summon::model_nodes(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x7f),
            esp: ebp.wrapping_sub(0x1c),
            ecx: rec,
            ..Frame::default()
        },
    );
    g.write::<u32>(w(0x85), 0);
    Ret::eax(eax)
}

/// [`model_start`] (145 bytes, 1 overlay).
pub const MODEL_START: Sig = Sig {
    len: 145,
    hash: 0xba77_4f99_2e83_3028,
    calls: &[(0x7f, Callee::Sig(&crate::s3_summon::MODEL_NODES))],
    imports: &[],
};
