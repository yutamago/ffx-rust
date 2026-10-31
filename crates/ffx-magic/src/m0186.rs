//! `magic_0186.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

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
