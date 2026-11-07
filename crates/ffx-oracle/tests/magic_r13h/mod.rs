//! Helpers shared by round 13's tests (`magic_r13.rs`, and the load steps
//! of its start-ups in `magic_r9_load.rs`).
#![allow(dead_code, clippy::cast_precision_loss)]

use crate::magic::{self, SCRATCH};
use crate::magic_owna::rec;
use crate::magic_r11;

/// The actor lookup (`sb_misc::lookup_actors` at `lookup`): two actor
/// bytes, slots 215 and 413 answering their argument.
pub fn lookup(t: &mut magic::Twin, lookup: u32) {
    rec(t, 215, 1, |_, a| 0x0215_0000 ^ a[0]);
    rec(t, 413, 1, |_, a| 0x0413_0000 ^ a[0]);
    let n = t.img.word(lookup + 0x34);
    t.set_u32(n, 2);
    let bytes = t.img.word(lookup + 0x4a);
    t.write(bytes, &[3, 5]);
}

/// Slot 268 writes its point (`a[2]`, three floats from the character
/// `a[0]` and the point number `a[1]`).
pub fn point_268(t: &mut magic::Twin) {
    rec(t, 268, 3, |m, a| {
        for i in 0..4u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + a[1] as f32 + i as f32 * 1.25;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        a[0] ^ 0x268
    });
}

/// What `r13_start::two_objects_0318` (at `f`) needs: the pools, the
/// slots, the lookup, both object starts, `G_H` and the task list.
pub fn two_objects_0318(t: &mut magic::Twin, f: u32, listed: bool) {
    let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 400);
    magic_r11::object_slots(t);
    point_268(t);
    let at = t.img.rel32_target(f + 0x3a);
    lookup(t, at);
    for s in [0x7fu32, 0xc7] {
        let start = t.img.rel32_target(f + s);
        magic_r11::object_start_auto(t, start);
    }
    t.set_u32(op(t, 0x9f), 0x0318_0042);
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(op(t, 0x151), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(op(t, 0x162), SCRATCH + 0x600);
}
