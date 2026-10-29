//! The node pool most effect overlays share (ported from `magic_0009`;
//! addresses are that DLL's): 0x1c-byte nodes on a free ring (`G_FREE`
//! 0x101bb9ac, the u16 count of free nodes `G_COUNT` 0x101bb9b0). A node
//! in use is `{prev, next, children, first child, owner, _, flag byte}`:
//! `+0x0`/`+0x4` the links of its sibling list, `+0x8` the node itself at
//! rest or the child list's node, `+0xc` the next node to free with it,
//! `+0x10` the owner value given at the take. Three globals track lists
//! that a freed node must leave (`G_HEAD` 0x101bb9a0, `G_CUR` 0x101bb9a4,
//! `G_TAIL` 0x101bb9a8). Both functions are internal (`ecx` = the node or
//! the owner, LTCG).

use crate::{Image, Mem, Sig};

/// Takes a node off the free ring (0x100039a0, `ecx` = the owner value):
/// 0 when none is left; else the node leaves the ring (`prev.next =
/// next`, `next.prev = prev`), the ring's head moves to its next (or 0
/// when it was the last), the count drops, and the node is reset: links,
/// `+0xc` and `+0x14` zero, `+0x8` itself, `+0x10` the owner, byte
/// `+0x18` 0. Returns the node (`eax`).
pub fn take(g: &mut impl Mem, img: &Image, f: u32, owner: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let count = g.read::<u16>(w(0x04));
    if count == 0 {
        return 0;
    }
    let node = g.read::<u32>(w(0x17));
    let prev = g.read::<u32>(node);
    let next = g.read::<u32>(node + 4);
    g.write::<u32>(prev + 4, next);
    let next = g.read::<u32>(node + 4);
    let prev = g.read::<u32>(node);
    g.write::<u32>(next, prev);
    if count > 1 {
        let next = g.read::<u32>(node + 4);
        g.write::<u32>(w(0x34), next);
    } else {
        g.write::<u32>(w(0x3c), 0);
    }
    g.write::<u32>(node + 0x10, owner);
    g.write::<u16>(w(0x53), count.wrapping_sub(1));
    g.write::<u32>(node, 0);
    g.write::<u32>(node + 4, 0);
    g.write::<u32>(node + 0xc, 0);
    g.write::<u32>(node + 0x14, 0);
    g.write::<u32>(node + 8, node);
    g.write::<u8>(node + 0x18, 0);
    node
}

/// `take` (125 bytes, 139 overlays).
pub const TAKE: Sig = Sig {
    len: 125,
    hash: 0x5d95_40db_67e0_e5ff,
    calls: &[],
    imports: &[],
};

/// Gives a node back (0x10003a20, `ecx` = the node): first every node
/// chained to it through the child node's `+0xc` (each unhooked from the
/// chain and given back, recursively, the node's `+0x8` then moving to
/// it); then the node leaves its sibling list, the three tracking globals
/// step past it (`G_CUR` to its next, cleared when the pool is full again
/// at 0x100 free nodes; `G_HEAD` to its prev; `G_TAIL` to its next), and
/// it goes in front of the free ring's head (or makes a ring of its own),
/// becoming the head; the count rises.
///
/// Returns the `eax` the original leaves (the count in the low half, the
/// last word loaded above it).
pub fn give_back(g: &mut impl Mem, img: &Image, f: u32, node: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let mut eax = g.read::<u32>(node + 8);
    if g.read::<u32>(eax + 0xc) != 0 {
        loop {
            let child = g.read::<u32>(node + 8);
            let next = g.read::<u32>(child + 0xc);
            g.write::<u32>(child + 0xc, 0);
            let c = g.read::<u32>(node + 8);
            eax = give_back(g, img, f, c);
            g.write::<u32>(node + 8, next);
            if g.read::<u32>(next + 0xc) == 0 {
                break;
            }
        }
    }
    let prev = g.read::<u32>(node);
    if prev != 0 {
        eax = g.read::<u32>(node + 4);
        g.write::<u32>(prev + 4, eax);
    }
    let next = g.read::<u32>(node + 4);
    if next != 0 {
        eax = g.read::<u32>(node);
        g.write::<u32>(next, eax);
    }
    let mut cur = g.read::<u32>(w(0x48));
    if cur == node {
        cur = g.read::<u32>(node + 4);
    }
    if g.read::<u32>(w(0x55)) == node {
        eax = g.read::<u32>(node);
        g.write::<u32>(w(0x5e), eax);
    }
    if g.read::<u32>(w(0x64)) == node {
        eax = g.read::<u32>(node + 4);
        g.write::<u32>(w(0x6e), eax);
    }
    let head = g.read::<u32>(w(0x74));
    if head == 0 {
        g.write::<u32>(node + 4, node);
        g.write::<u32>(node, node);
    } else {
        g.write::<u32>(node + 4, head);
        eax = g.read::<u32>(head);
        g.write::<u32>(node, eax);
        g.write::<u32>(head, node);
    }
    let count = g.read::<u16>(w(0x8e)).wrapping_add(1);
    g.write::<u32>(w(0x94), node);
    if count == 0x100 {
        cur = 0;
    }
    g.write::<u16>(w(0xa9), count);
    g.write::<u32>(w(0xaf), cur);
    (eax & 0xffff_0000) | u32::from(count)
}

/// `give_back` (181 bytes, 176 overlays; the `rel32` at `+0x21`, its own
/// recursive call, is the same in every copy and stays in the hash).
pub const GIVE_BACK: Sig = Sig {
    len: 181,
    hash: 0x6ac2_fab9_0034_5932,
    calls: &[],
    imports: &[],
};
