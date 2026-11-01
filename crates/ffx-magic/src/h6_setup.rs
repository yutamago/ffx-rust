//! Shapes the own set-ups and task steps of `magic_0512` and up share
//! (magic r6 own, 0450 and up): an object joining the DLL's ring, a
//! child of a static node list, the inlined mode walk and the task
//! node's unlink. Each takes the operand offsets of the instructions
//! that hold the DLL's addresses.

use crate::l3_common::{ListAt, attach, node_init};
use crate::{Game, Image};

/// An object joins the DLL's ring at its end: its links itself, then
/// vtable and step (operands `ops[0]`, `ops[1]`), `+0x14` itself, `+0x8`
/// the ring's last (`ops[2]`), `+0x4` the ring's sentinel (`ops[3]`), the
/// old last's `+0x4` the object (`ops[4]`), the ring flags' bit 0 cleared
/// (`ops[5]`) and the object the last (`ops[6]`).
pub fn ring_join<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32, ops: [u32; 7]) {
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(obj + 8, obj);
    g.write::<u32>(obj + 4, obj);
    g.write::<u32>(obj, w(ops[0]));
    g.write::<u32>(obj + 0x10, w(ops[1]));
    g.write::<u32>(obj + 0x14, obj);
    let last = g.read::<u32>(w(ops[2]));
    g.write::<u32>(obj + 8, last);
    g.write::<u32>(obj + 4, w(ops[3]));
    let last = g.read::<u32>(w(ops[4]));
    g.write::<u32>(last + 4, obj);
    let flags = g.read::<u32>(w(ops[5]));
    g.write::<u32>(w(ops[5]), flags & !1);
    g.write::<u32>(w(ops[6]), obj);
}

/// A child of a static node list: [`node_init`] with the list (operand
/// `ops[0]`), then [`attach`] to the list `ops[1]` through its words (the
/// six operands of [`ListAt`]: last, last again, sentinel, first, count,
/// last written).
pub fn static_child<G: Game>(g: &mut G, img: &Image, f: u32, node: u32, ops: [u32; 8]) {
    let w = |off: u32| img.word(f + off);
    node_init(g, node, w(ops[0]));
    let at = ListAt {
        last: w(ops[2]),
        last2: w(ops[3]),
        sentinel: w(ops[4]),
        first: w(ops[5]),
        count: w(ops[6]),
        last_w: w(ops[7]),
    };
    attach(g, node, w(ops[1]), at);
}

/// The node takes the DLL's mode, as the set-ups inline the walk: the
/// local group `[node, .., mode]` (the mode byte at `+5`); unless the
/// node has the mode already, each child (from `+0x10` along `+0x8`) gets
/// its vtable's `+0x1c` method with the group (`ecx` too), and the node's
/// `+0x1a` the mode (read back from the group after a child).
pub fn mode_walk<G: Game>(g: &mut G, group: u32, node: u32, mode: u8) {
    g.write::<u32>(group, node);
    g.write::<u8>(group + 5, mode);
    if g.read::<u8>(node + 0x1a) == mode {
        return;
    }
    let mut cur = g.read::<u32>(node + 0x10);
    let mut m = mode;
    if cur != node {
        while cur != node {
            g.write::<u32>(group, cur);
            let next = g.read::<u32>(cur + 8);
            let method = g.read::<u32>(g.read::<u32>(cur) + 0x1c);
            g.call(method, group, &[group]);
            cur = next;
        }
        m = g.read::<u8>(group + 5);
    }
    g.write::<u8>(node + 0x1a, m);
}

/// The task's node `*task` leaves its ring (the neighbours joined, the
/// node linked to itself). Returns the next node as read last (the
/// original's `eax`).
pub fn unlink_task<G: Game>(g: &mut G, task: u32) -> u32 {
    let node = g.read::<u32>(task);
    let next = g.read::<u32>(node + 4);
    let prev = g.read::<u32>(node + 8);
    g.write::<u32>(next + 8, prev);
    let prev = g.read::<u32>(node + 8);
    let next = g.read::<u32>(node + 4);
    g.write::<u32>(prev + 4, next);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    next
}
