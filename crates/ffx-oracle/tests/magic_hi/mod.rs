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

/// A model of [`models`]: its parts `(kind, byte, entries)`, or none.
pub type Model<'a> = Option<&'a [(u32, u8, i16)]>;

/// A model table: `n` models (0 = none) at `models`, each with its
/// parts `(kind, byte, entries)`.
pub fn models(t: &mut Twin, g_at: u32, models: &[Model]) {
    let owner = SCRATCH + 0x100;
    let table = SCRATCH + 0x180;
    let fill: Vec<u8> = (0..0x2000u32)
        .map(|i| u8::try_from((i * 37 + 5) & 0xff).unwrap())
        .collect();
    t.write(SCRATCH + 0x2000, &fill);
    t.write(owner, &[0; 0x40]);
    t.set_u32(g_at, owner);
    t.set_u32(owner + 0x18, table);
    t.set_u16(owner + 0xa, u16::try_from(models.len()).unwrap());
    for (i, m) in (0u32..).zip(models) {
        let Some(parts) = m else {
            t.set_u32(table + 4 * i, 0);
            continue;
        };
        let model = SCRATCH + 0x2000 + 0x400 * i;
        t.set_u32(table + 4 * i, model);
        t.set_u16(model + 6, u16::try_from(parts.len()).unwrap());
        for (j, &(kind, byte, n)) in (0u32..).zip(parts.iter()) {
            let off = 0x80 + 0x100 * j;
            t.set_u16(model + 0x10 + 8 * j, u16::try_from(off).unwrap());
            let part = model + off;
            t.set_u32(part + 0x10, kind);
            t.set_u16(part + 2, n.cast_unsigned());
            t.set_u32(part + 4, 0x20);
            t.set_u8(part, byte);
            for k in 0..4 {
                t.set_u8(part + 0x24 + 8 * k, if k == 2 { 0x47 } else { byte });
            }
        }
    }
}
