//! Helpers for the parity tests of the r5 own-code ports from
//! `magic_0450` up (`magic_NNNN.rs`; docs/formats/magic.md, "Porting the
//! overlays").
#![allow(dead_code)]

use crate::magic::{SCRATCH, Twin};

/// `f32` bits.
#[must_use]
pub fn f(x: f32) -> u32 {
    x.to_bits()
}

/// The `f32` at the address in the operand `op` of the code at `at`.
#[must_use]
pub fn konst(t: &Twin, at: u32, op: u32) -> f32 {
    f32::from_bits(t.u32(t.img.word(at + op)))
}

/// A free ring of `n` pool nodes (0x1c bytes each, at `nodes`) for the
/// pool take at `take` (`sa_pool::take`: count `+0x04`, head `+0x17`),
/// the nodes dirty first.
pub fn pool(t: &mut Twin, take: u32, nodes: u32, n: u32) {
    t.write(nodes - 0x20, &[0x6d; 0x20 + 0x1c * 8]);
    for i in 0..n {
        let node = nodes + 0x1c * i;
        t.set_u32(node, nodes + 0x1c * ((i + n - 1) % n));
        t.set_u32(node + 4, nodes + 0x1c * ((i + 1) % n));
    }
    t.set_u16(t.img.word(take + 0x04), u16::try_from(n).unwrap());
    t.set_u32(t.img.word(take + 0x17), nodes);
}

/// A list (head global `head`, tail global `tail`) holding one node at
/// `node` when `full`, empty otherwise.
pub fn list(t: &mut Twin, head: u32, tail: u32, node: u32, full: bool) {
    t.write(node, &[0x3a; 0x1c]);
    t.set_u32(head, if full { node } else { 0 });
    t.set_u32(tail, if full { node } else { 0 });
}

/// A lone node for the pool's give-back (`sa_pool::give_back`): no
/// children, no siblings, the pool's tracking globals elsewhere and an
/// empty free ring.
pub fn lone_node(t: &mut Twin, give: u32, node: u32) {
    t.write(node, &[0; 0x1c]);
    t.set_u32(node + 8, node);
    for op in [0x48, 0x55, 0x64] {
        t.set_u32(t.img.word(give + op), SCRATCH + 0x3f00);
    }
    t.set_u32(t.img.word(give + 0x74), 0);
}

/// The objects' list of `run_objects` (`l5_objhost`, at `ro`) empty, or
/// holding one object that still runs.
pub fn objects(t: &mut Twin, ro: u32, running: bool) {
    let (node, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    t.set_u32(node + 8, obj);
    t.set_u16(obj + 0x124, 0x8001);
    let list = t.img.word(ro + 0x7c);
    t.set_u32(list + 8, if running { node } else { 0 });
    t.set_u32(t.img.word(ro + 0x14), if running { node } else { 0 });
}

/// The motion step (`s3_motion::motion_step` at `ms`) answering 1 (done)
/// or 0 (a wait running).
pub fn motion(t: &mut Twin, ms: u32, done: bool) {
    t.set_u32(t.img.word(ms + 0x02), u32::from(done));
    t.set_u16(t.img.word(ms + 0x11), 5);
}

/// A free ring of `n` nodes for the free-list pop (`m0032::pop_node` at
/// `pop`: count `+0x04`, head `+0x13`).
pub fn pop_pool(t: &mut Twin, pop: u32, nodes: u32, n: u32) {
    t.write(nodes - 0x20, &[0x6d; 0x20 + 0x1c * 8]);
    for i in 0..n {
        let node = nodes + 0x1c * i;
        t.set_u32(node, nodes + 0x1c * ((i + n - 1) % n));
        t.set_u32(node + 4, nodes + 0x1c * ((i + 1) % n));
    }
    t.set_u16(t.img.word(pop + 0x04), u16::try_from(n).unwrap());
    t.set_u32(t.img.word(pop + 0x13), nodes);
}
