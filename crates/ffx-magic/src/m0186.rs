//! `magic_0186.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// A node from a record (0x100056f0, internal: `ecx` = `[node,
/// parent]`): pass byte `+0x1a` 0xff, an empty child list, the parent
/// `+0x14` (appended to the parent's list when there is one), then
/// `+0x1c..+0x40` cleared but for `+0x28`. Returns the parent's old last
/// child (as read the second time), or the caller's `eax` with no parent.
pub fn node_cleared(g: &mut impl crate::Mem, rec: u32, eax: u32) -> u32 {
    let node = g.read::<u32>(rec);
    let eax = node_cleared_link(g, rec, node).unwrap_or(eax);
    for off in [0x40, 0x3c, 0x38, 0x34, 0x30, 0x2c, 0x24, 0x20, 0x1c] {
        g.write::<u32>(node + off, 0);
    }
    eax
}

/// [`node_cleared`] (119 bytes, 1 overlay).
pub const NODE_CLEARED: Sig = Sig {
    len: 119,
    hash: 0xa343_6799_4695_31a0,
    calls: &[],
    imports: &[],
};

/// A node handler (0x10005770, `ecx` = the cursor): the walk
/// ([`walk`](crate::m0003::walk)); when the cursor moved, the node's
/// integer motion: `+0x2c..+0x34` (velocity) += `+0x38..+0x40`
/// (acceleration), `+0x1c..+0x24` (position) += the velocity, axis by
/// axis. Returns the new `+0x34`, else the walk's `eax`.
pub fn int_motion<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let cur = fr.ecx;
    let eax = crate::m0003::walk(g, cur, fr.eax);
    if g.read::<u8>(cur + 4) & 1 == 0 {
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(cur);
    let mut last = 0;
    for i in 0..3 {
        let v = g
            .read::<u32>(n + 0x2c + 4 * i)
            .wrapping_add(g.read::<u32>(n + 0x38 + 4 * i));
        g.write::<u32>(n + 0x2c + 4 * i, v);
        let p = g.read::<u32>(n + 0x1c + 4 * i).wrapping_add(v);
        g.write::<u32>(n + 0x1c + 4 * i, p);
        last = v;
    }
    Ret::eax(last)
}

/// [`int_motion`] (114 bytes, 1 overlay).
pub const INT_MOTION: Sig = Sig {
    len: 114,
    hash: 0x09eb_bff6_30eb_1cba,
    calls: &[],
    imports: &[],
};

/// The node of a record `[node, parent]` linked: pass byte `+0x1a` 0xff,
/// an empty child list, the parent `+0x14`, appended to the parent's list
/// when there is one. Returns the parent's old last child (as read the
/// second time) when there is a parent.
fn node_cleared_link(g: &mut impl crate::Mem, rec: u32, node: u32) -> Option<u32> {
    let parent = g.read::<u32>(rec + 4);
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, parent);
    if parent == 0 {
        return None;
    }
    g.write::<u32>(node + 4, parent);
    let last = g.read::<u32>(parent + 0x10);
    g.write::<u32>(node + 8, last);
    let last = g.read::<u32>(parent + 0x10);
    if last == parent {
        g.write::<u32>(parent + 0xc, node);
    } else {
        g.write::<u32>(last + 4, node);
    }
    let n = g.read::<u16>(parent + 0x18);
    g.write::<u16>(parent + 0x18, n.wrapping_add(1));
    g.write::<u32>(parent + 0x10, node);
    Some(last)
}

/// A node from a record (0x10003c60, internal: `ecx` = `[node, parent,
/// a, b, c]`): as [`node_cleared`] links it (pass byte, empty child list,
/// appended to the parent's list), then `+0x54` = `a`, `+0x4c` = `b`,
/// `+0x50` = `c`, the scale `+0x28` 1.0, u16 `+0x5c` 3, `+0x58` 6 and
/// `+0x3c..+0x48` 0. Returns `c`.
pub fn node_abc(g: &mut impl crate::Mem, rec: u32) -> u32 {
    let node = g.read::<u32>(rec);
    node_cleared_link(g, rec, node);
    g.write::<u32>(node + 0x54, g.read::<u32>(rec + 8));
    g.write::<u32>(node + 0x4c, g.read::<u32>(rec + 0xc));
    let c = g.read::<u32>(rec + 0x10);
    g.write::<u32>(node + 0x28, 0x3f80_0000);
    g.write::<u16>(node + 0x5c, 3);
    g.write::<u32>(node + 0x58, 6);
    for off in [0x3c, 0x40, 0x44, 0x48] {
        g.write::<u32>(node + off, 0);
    }
    g.write::<u32>(node + 0x50, c);
    c
}

/// [`node_abc`] (124 bytes, 1 overlay).
pub const NODE_ABC: Sig = Sig {
    len: 124,
    hash: 0xae9c_0927_9cd1_2774,
    calls: &[],
    imports: &[],
};

/// A node under its parent as the record set-ups place it: pass byte
/// and halfword `+0x1a` 0xff, an empty child list (`+0xc`, `+0x10` the
/// node), the parent `+0x14`; with a parent the node is its last child
/// (`+4` the parent, `+8` the old last; the old last's `+4`, or the
/// parent's first `+0xc` when it had none, the node) and the parent's
/// count `+0x18` goes up.
pub(crate) fn node_under<G: Game>(g: &mut G, node: u32, parent: u32) {
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, parent);
    if parent == 0 {
        return;
    }
    g.write::<u32>(node + 4, parent);
    g.write::<u32>(node + 8, g.read::<u32>(parent + 0x10));
    let last = g.read::<u32>(parent + 0x10);
    if last == parent {
        g.write::<u32>(parent + 0xc, node);
    } else {
        g.write::<u32>(last + 4, node);
    }
    let n = g.read::<u16>(parent + 0x18);
    g.write::<u16>(parent + 0x18, n.wrapping_add(1));
    g.write::<u32>(parent + 0x10, node);
}

/// A model node's record set-up (0x10004db0, internal: `ecx` = `[node,
/// parent, a, b, c, model, part, d]`): [`node_under`]; `+0x13c` = a,
/// `+0x144` = b, `+0x140` = c, `+0x1f4` = d, `+0x1e8`/`+0x1ec` the model
/// and part, `+0x1f8`/`+0x1f0` 0, `+0x1e4` 1.0; slot 236 `(model, part,
/// M)` fills the matrix `M` = `+0x14c`, slot 233 `(v, v, 1e6)` for its
/// three rows `+0x14c`, `+0x15c`, `+0x16c`; slot 325 `(+0x1cc, +0x17c)`;
/// `+0x10c..+0x114` 0 and `+0x118` 1.0. Returns slot 325's `eax`. Each
/// slot call passes the words the original leaves on the stack.
pub fn model_record<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let r = fr.ecx;
    let node = g.read::<u32>(r);
    let parent = g.read::<u32>(r + 4);
    node_under(g, node, parent);
    let a = g.read::<u32>(r + 8);
    let model = g.read::<u32>(r + 0x14);
    let part = g.read::<u32>(r + 0x18);
    g.write::<u32>(node + 0x13c, a);
    g.write::<u32>(node + 0x144, g.read::<u32>(r + 0xc));
    g.write::<u32>(node + 0x140, g.read::<u32>(r + 0x10));
    g.write::<u32>(node + 0x1f4, g.read::<u32>(r + 0x1c));
    let m = node + 0x14c;
    let mut p = crate::pushes::Pushes::default();
    p.push_all(&[m, part, model]);
    g.write::<u32>(node + 0x1e8, model);
    g.write::<u32>(node + 0x1ec, part);
    g.write::<u32>(node + 0x1f8, 0);
    g.write::<u32>(node + 0x1f0, 0);
    g.write::<u32>(node + 0x1e4, 0x3f80_0000);
    let s = slot_target(g, w(0x66), 0x3b0 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    for (row, table) in [(0x14c, 0xa6), (0x15c, 0xd1), (0x16c, 0xf0)] {
        // add esp, 8, and 1e6 written over the word left on top
        p.pop(2);
        p.set_top(0x4974_2400);
        let v = node + row;
        p.push_all(&[v, v]);
        let s = slot_target(g, w(table), 0x3a4 / 4);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, s, 0);
    }
    p.push_all(&[node + 0x17c, node + 0x1cc]);
    let s = slot_target(g, w(0x10b), 0x514 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let eax = p.call(g, s, 0).eax;
    for off in [0x114, 0x110, 0x10c] {
        g.write::<u32>(node + off, 0);
    }
    g.write::<u32>(node + 0x118, 0x3f80_0000);
    Ret::eax(eax)
}

/// [`model_record`] (325 bytes, 1 overlay).
pub const MODEL_RECORD: Sig = Sig {
    len: 325,
    hash: 0x8cec_a6ad_d946_fd87,
    calls: &[],
    imports: &[],
};

/// A model node from its record (0x10007700, internal: `ecx` = the node,
/// the stack words 0, 1 and 3 `a`, `b`, `d`): the node's vtable the
/// immediate `+0x1a`; [`model_record`] of the local `[node, P, a, b, Q,
/// d, 0x63, 0]` (`ebp - 0x20`; `P`, `Q` the immediates `+0x24`, `+0x2b`).
/// Returns its `eax`.
pub fn model_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let node = fr.ecx;
    let (a, b, d) = (fr.arg(g, 0), fr.arg(g, 1), fr.arg(g, 3));
    let rec = fr.esp.wrapping_sub(0x24);
    g.write::<u32>(node, w(0x1a));
    for (i, v) in (0u32..).zip([node, w(0x24), a, b, w(0x2b), d, 0x63, 0]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    let callee = Frame {
        at: img.rel32_target(f + 0x41),
        esp: rec.wrapping_sub(4),
        ecx: rec,
        ..*fr
    };
    model_record(g, img, &callee)
}

/// [`model_node`] (73 bytes, 1 overlay).
pub const MODEL_NODE: Sig = Sig {
    len: 73,
    hash: 0xd996_7138_a223_68d5,
    calls: &[(0x41, crate::Callee::Sig(&MODEL_RECORD))],
    imports: &[],
};

/// A cleared node (0x10007750, internal: `ecx` = the node): [`node_cleared`]
/// of the local `[node, P]` (`ebp - 8`, `P` the immediate `+0x12`), then
/// the node's vtable the immediate `+0x1d`. Returns `node_cleared`'s
/// `eax`.
pub fn cleared_node(
    g: &mut impl crate::Mem,
    img: &Image,
    f: u32,
    node: u32,
    esp: u32,
    eax: u32,
) -> u32 {
    let w = |o: u32| img.word(f + o);
    let rec = esp.wrapping_sub(0xc);
    g.write::<u32>(rec, node);
    g.write::<u32>(rec + 4, w(0x12));
    let eax = node_cleared(g, rec, eax);
    g.write::<u32>(node, w(0x1d));
    eax
}

/// [`cleared_node`] (38 bytes, 1 overlay).
pub const CLEARED_NODE: Sig = Sig {
    len: 38,
    hash: 0xebcf_113b_81cc_a98f,
    calls: &[(0x17, crate::Callee::Sig(&NODE_CLEARED))],
    imports: &[],
};
