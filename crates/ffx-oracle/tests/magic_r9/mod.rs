//! Helpers shared by round 9's family tests (`magic_r9_*.rs`).
#![allow(dead_code)]

use crate::magic::{self, SCRATCH};
use crate::magic_owna::rec;

/// What `r9_misc::objects_start`'s callees need: the part set-up's buffers, the
/// object start's part table, lock and pool, the actors (`n` of them,
/// their selector bytes and characters) and the free ring (`free`
/// nodes; the task list empty or not).
pub fn objects_setup(t: &mut magic::Twin, f: u32, n: u32, free: u16, listed: bool) {
    let op = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
    let pi = t.img.rel32_target(f + 0x3a);
    for (j, o) in (0u32..).zip([0x01u32, 0x07, 0x11]) {
        t.set_u32(op(t, pi, o), SCRATCH + 0x4000 + 0x40 * j);
    }
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(t, 941, 1, move |_, _| table);
    rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(t, 262, 4, |m, a| {
        for i in 0..3 {
            m.w32(a[1] + 4 * i, 0x3f00_0000 + i);
        }
        0x262
    });
    rec(t, 268, 3, |_, a| a[0] ^ 0x268);
    let start = t.img.rel32_target(f + 0x7e);
    let sp = t.img.rel32_target(start + 0x80);
    t.set_u32(op(t, sp, 0x04), SCRATCH + 0x7f8);
    let pa = t.img.rel32_target(start + 0x9b);
    let nodes = op(t, pa, 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    t.set_u16(op(t, pa, 0x04), 8);
    t.set_u32(op(t, start, 0x04), SCRATCH + 0x2f0);
    t.set_u32(op(t, start, 0x5e), SCRATCH + 0x280);
    t.set_u32(op(t, f, 0x4a), n);
    // the actors' bytes (the two operands are one global here)
    let (sel_a, sel_b) = (SCRATCH + 0x180, SCRATCH + 0x180);
    t.set_u32(op(t, f, 0x99), sel_a);
    t.set_u32(op(t, f, 0xec), sel_b);
    for i in 0..8u8 {
        t.set_u8(sel_a + u32::from(i), 3 + 5 * i);
    }
    for i in 0..0x40 {
        t.set_u32(op(t, f, 0xa4) + 4 * i, 0x00c0_0000 + i);
    }
    // the actors' records hold junk
    t.write(op(t, f, 0x10b), &[0x5c; 0x34 * 4]);
    let node = |i: u32| SCRATCH + 0x8000 + 0x20 * i;
    for i in 0..3 {
        t.write(node(i), &[0x5a; 0x20]);
        t.set_u32(node(i), node((i + 2) % 3));
        t.set_u32(node(i) + 4, node((i + 1) % 3));
    }
    t.set_u32(op(t, f, 0x176), node(0));
    t.set_u16(op(t, f, 0x167), free);
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(op(t, f, 0x1dd), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(op(t, f, 0x1ed), SCRATCH + 0x600);
}

/// What `r9_misc::effect_start`'s callees need: the part set-up's
/// buffers, the object start's part table, lock and pool, the yaw slot
/// (`yaw` as its float) and the free list (`free` nodes; the task list
/// empty or not).
pub fn effect_setup(t: &mut magic::Twin, f: u32, yaw: f64, free: u16, listed: bool) {
    let op = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
    let pi = t.img.rel32_target(f + 0x37);
    for (j, o) in (0u32..).zip([0x01u32, 0x07, 0x11]) {
        t.set_u32(op(t, pi, o), SCRATCH + 0x4000 + 0x40 * j);
    }
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(t, 941, 1, move |_, _| table);
    rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(t, 215, 1, |_, a| 0x0215_0000 ^ a[0]);
    rec(t, 413, 1, |_, a| 0x0413_0000 ^ a[0]);
    rec(t, 262, 4, |m, a| {
        for i in 0..3 {
            m.w32(a[1] + 4 * i, 0x3f00_0000 + 0x10_0000 * i);
        }
        0x262
    });
    rec(t, 333, 5, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, move |_, _| yaw);
    let start = t.img.rel32_target(f + 0x4c);
    let sp = t.img.rel32_target(start + 0x7d);
    t.set_u32(op(t, sp, 0x04), SCRATCH + 0x7f8);
    let pa = t.img.rel32_target(start + 0x8f);
    let nodes = op(t, pa, 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    t.set_u16(op(t, pa, 0x04), 8);
    t.set_u32(op(t, start, 0x01), SCRATCH + 0x2f0);
    t.set_u32(op(t, start, 0x5b), SCRATCH + 0x280);
    t.set_u32(op(t, f, 0x6c), 0x6c6c);
    t.set_u32(op(t, f, 0x89), 0x8989);
    let pop = t.img.rel32_target(f + 0x164);
    let node = |i: u32| SCRATCH + 0x8000 + 0x20 * i;
    for i in 0..3 {
        t.write(node(i), &[0x5a; 0x20]);
        t.set_u32(node(i), node((i + 2) % 3));
        t.set_u32(node(i) + 4, node((i + 1) % 3));
    }
    t.set_u32(op(t, pop, 0x13), node(0));
    t.set_u16(op(t, pop, 0x04), free);
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(op(t, f, 0x16a), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(op(t, f, 0x17a), SCRATCH + 0x600);
    for o in [
        0x8fu32, 0xf3, 0xfd, 0x107, 0x111, 0x12f, 0x139, 0x143, 0x153,
    ] {
        t.set_u32(op(t, f, o), 0x5a5a_5a5b);
    }
}
