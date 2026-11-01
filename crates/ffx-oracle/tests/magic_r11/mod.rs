//! Helpers shared by round 11's tests (`magic_r11_*.rs`, and the load
//! steps of its start-ups in `magic_r9_load.rs`).
#![allow(dead_code)]

use crate::magic::{self, SCRATCH};
use crate::magic_owna::rec;
use ffx_magic::{Image, l5_objhost, m0032, sa_pool};

/// The pools (`count`, `head` operands) and part set-ups the `rel32`
/// callees of `at..at + len` reach, `depth` calls deep.
fn needs(
    img: &Image,
    at: u32,
    len: u32,
    depth: u32,
    rings: &mut Vec<(u32, u32)>,
    parts: &mut Vec<u32>,
) {
    for p in at..at + len {
        if img.word(p).to_le_bytes()[0] != 0xe8 {
            continue;
        }
        let t = img.rel32_target(p + 1);
        if !img.in_text(t) {
            continue;
        }
        let ring = if img.matches(t, &sa_pool::TAKE) {
            Some((img.word(t + 0x04), img.word(t + 0x17)))
        } else if img.matches(t, &m0032::POP_NODE) {
            Some((img.word(t + 0x04), img.word(t + 0x13)))
        } else {
            None
        };
        if let Some(r) = ring {
            if !rings.contains(&r) {
                rings.push(r);
            }
        } else if img.matches(t, &l5_objhost::PART_INIT) {
            if !parts.contains(&t) {
                parts.push(t);
            }
        } else if depth > 0 {
            needs(img, t, 0x200, depth - 1, rings, parts);
        }
    }
}

/// Free rings of 8 nodes (0x40 bytes of junk each) for every pool the
/// code at `f` (`len` bytes) takes from, and buffers for its part
/// set-ups.
pub fn pools(t: &mut magic::Twin, f: u32, len: u32) {
    let (mut rings, mut parts) = (Vec::new(), Vec::new());
    needs(&t.img, f, len, 1, &mut rings, &mut parts);
    for (k, (count, head)) in (0u32..).zip(rings) {
        let at = SCRATCH + 0xc000 + 0x400 * k;
        let node = |i: u32| at + 0x40 * i;
        for i in 0..8 {
            t.write(node(i), &[0x5a; 0x40]);
            t.set_u32(node(i), node((i + 7) % 8));
            t.set_u32(node(i) + 4, node((i + 1) % 8));
        }
        t.set_u32(head, node(0));
        t.set_u16(count, 8);
    }
    for (k, p) in (0u32..).zip(parts) {
        for (j, o) in (0u32..).zip([0x01u32, 0x07, 0x11]) {
            let at = t.img.word(p + o);
            t.set_u32(at, SCRATCH + 0x4000 + 0x200 * k + 0x40 * j);
        }
    }
}

/// The first actor's selector (operand `sel`, byte 3) and the characters
/// (operand `chr`).
pub fn first_actor(t: &mut magic::Twin, f: u32, sel: u32, chr: u32) {
    let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
    let s = SCRATCH + 0x180;
    t.set_u32(op(t, sel), s);
    for i in 0..8u8 {
        t.set_u8(s + u32::from(i), 3 + 5 * i);
    }
    for i in 0..0x40 {
        t.set_u32(op(t, chr) + 4 * i, 0x00c0_0000 + i);
    }
}

/// What `r11_fades::object_two_fades` (`magic_0105`, at `f`) needs.
pub fn object_two_fades_setup(t: &mut magic::Twin, f: u32, listed: bool) {
    let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
    pools(t, f, 0x1a6);
    object_slots(t);
    let start = t.img.rel32_target(f + 0x49);
    object_start_setup(t, start, (0x7d, 0x95));
    first_actor(t, f, 0x4e, 0x62);
    // junk in the records the start-up and the fades fill
    for o in [0xa5u32, 0x130, 0xb5] {
        t.write(op(t, o), &[0x5c; 0x40]);
    }
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(op(t, 0x16b), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(op(t, 0x17c), SCRATCH + 0x600);
    rec(t, 271, 1, |_, a| a[0] ^ 0x271);
}

/// What an object start (`start`: its part start's `rel32` at `sp_at`,
/// the pool's at `pa_at`) needs: `G_P`, `G_D` and the pool's nodes.
pub fn object_start_setup(t: &mut magic::Twin, start: u32, (sp_at, pa_at): (u32, u32)) {
    let op = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
    let sp = t.img.rel32_target(start + sp_at);
    t.set_u32(op(t, sp, 0x04), SCRATCH + 0x7f8);
    let pa = t.img.rel32_target(start + pa_at);
    let nodes = op(t, pa, 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    t.set_u16(op(t, pa, 0x04), 8);
    // `G_P` and `G_D` three bytes later after a frame's set-up
    let d = if t.img.word(start).to_le_bytes()[0] == 0x55 {
        3
    } else {
        0
    };
    t.set_u32(op(t, start, 0x01 + d), SCRATCH + 0x2f0);
    t.set_u32(op(t, start, 0x5b + d), SCRATCH + 0x280);
}

/// The slots an object start-up calls: 941 (a part table), 714, 719,
/// 262 (fills the point), 268, 271.
pub fn object_slots(t: &mut magic::Twin) {
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(t, 941, 1, move |_, _| table);
    rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(t, 262, 4, |m, a| {
        for i in 0..3 {
            m.w32(a[1 + i as usize], 0x3f00_0000 + 0x10_0000 * i);
        }
        0x262
    });
    rec(t, 268, 3, |_, a| a[0] ^ 0x268);
    rec(t, 271, 1, |_, a| a[0] ^ 0x271);
}

/// A free ring of `free` nodes (0x20 bytes of junk each) at `at` whose
/// count (`u16`) and head are the globals `count`, `head`.
pub fn ring(t: &mut magic::Twin, count: u32, head: u32, free: u16, at: u32) {
    let k = u32::from(free.max(1));
    let node = |i: u32| at + 0x20 * i;
    for i in 0..k {
        t.write(node(i), &[0x5a; 0x20]);
        t.set_u32(node(i), node((i + k - 1) % k));
        t.set_u32(node(i) + 4, node((i + 1) % k));
    }
    t.set_u32(head, node(0));
    t.set_u16(count, free);
}

/// What `r11_objects::three_nodes` (at `f`) needs: the part set-up's
/// buffers, the object start's, the actors (`n`; their selector bytes and
/// characters), the free ring (`free` nodes) and the task list (empty or
/// not).
pub fn three_nodes_setup(t: &mut magic::Twin, f: u32, n: u32, free: u16, listed: bool) {
    let op = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
    let pi = t.img.rel32_target(f + 0x3c);
    for (j, o) in (0u32..).zip([0x01u32, 0x07, 0x11]) {
        t.set_u32(op(t, pi, o), SCRATCH + 0x4000 + 0x40 * j);
    }
    object_slots(t);
    object_start_setup(t, t.img.rel32_target(f + 0x7e), (0x80, 0x9b));
    let n_at = if t.img.word(f + 0x45).to_le_bytes()[0] == 0xb8 {
        0x4f
    } else {
        0x4c
    };
    t.set_u32(op(t, f, n_at), n);
    // the actors' bytes (the two operands are one global here)
    let sel = SCRATCH + 0x180;
    t.set_u32(op(t, f, 0x99), sel);
    for i in 0..8u8 {
        t.set_u8(sel + u32::from(i), 3 + 5 * i);
    }
    for i in 0..0x40 {
        t.set_u32(op(t, f, 0xa4) + 4 * i, 0x00c0_0000 + i);
    }
    t.write(op(t, f, 0x10b), &[0x5c; 0x34 * 4]);
    nodes_setup(t, f, free, listed);
}

/// The three task nodes' needs (`f` where `three_nodes`' numbering
/// starts): junk in the words they set, the free ring (`free` nodes) and
/// the task list (empty or not).
pub fn nodes_setup(t: &mut magic::Twin, f: u32, free: u16, listed: bool) {
    let op = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
    // junk in the words the start-up sets
    for at in (0..12).map(|k| 0x169 + 10 * k).chain([0x297, 0x2a4]) {
        t.set_u32(op(t, f, at + 2), 0x5a5a_5a5b);
    }
    for at in (0..8).map(|k| 0x2ae + 10 * k).chain([0x303, 0x30d, 0x317]) {
        t.set_u32(op(t, f, at + 2), 0x5a5a_5a5b);
    }
    t.set_u32(op(t, f, 0x2ff), 0x5a5a_5a5b);
    ring(t, op(t, f, 0x165), op(t, f, 0x1ed), free, SCRATCH + 0x8000);
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(op(t, f, 0x252), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(op(t, f, 0x26d), SCRATCH + 0x600);
}

/// What `r11_objects::two_per_actor` (at `f`) needs: the part set-up's
/// buffers, the object starts', the actors (`n`), the records and the
/// three nodes ([`nodes_setup`]).
pub fn two_per_actor_setup(t: &mut magic::Twin, f: u32, n: u32, free: u16, listed: bool) {
    let op = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
    let pi = t.img.rel32_target(f + 0x46);
    for (j, o) in (0u32..).zip([0x01u32, 0x07, 0x11]) {
        t.set_u32(op(t, pi, o), SCRATCH + 0x4000 + 0x40 * j);
    }
    object_slots(t);
    let d = u32::from(t.img.word(f + 0x1de).to_le_bytes()[0] != 0x6a);
    for at in [0xe5, 0x1ed - d] {
        object_start_setup(t, t.img.rel32_target(f + at), (0x84, 0x9d));
    }
    t.set_u32(op(t, f, 0x4b), n);
    let sel = SCRATCH + 0x180;
    t.set_u32(op(t, f, 0x80), sel);
    for i in 0..8u8 {
        t.set_u8(sel + u32::from(i), 3 + 5 * i);
    }
    for i in 0..0x40 {
        t.set_u32(op(t, f, 0xaa) + 4 * i, 0x00c0_0000 + i);
    }
    t.write(op(t, f, 0x191), &[0x5c; 0x34 * 4]);
    nodes_setup(t, f + 0x13a - d, free, listed);
}

/// What `r11_objects::three_nodes_height` (`magic_0120`, at `f`) needs:
/// [`three_nodes_setup`]'s, the records 0x30 bytes later, slot 12's
/// height.
pub fn three_nodes_height_setup(t: &mut magic::Twin, f: u32, n: u32, free: u16, listed: bool) {
    let op = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
    let pi = t.img.rel32_target(f + 0x3c);
    for (j, o) in (0u32..).zip([0x01u32, 0x07, 0x11]) {
        t.set_u32(op(t, pi, o), SCRATCH + 0x4000 + 0x40 * j);
    }
    object_slots(t);
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * 1.25);
    object_start_setup(t, t.img.rel32_target(f + 0x7e), (0x80, 0x9b));
    t.set_u32(op(t, f, 0x4c), n);
    let sel = SCRATCH + 0x180;
    t.set_u32(op(t, f, 0x99), sel);
    for i in 0..8u8 {
        t.set_u8(sel + u32::from(i), 3 + 5 * i);
    }
    for i in 0..0x40 {
        t.set_u32(op(t, f, 0xa4) + 4 * i, 0x00c0_0000 + i);
    }
    t.write(op(t, f, 0x13b), &[0x5c; 0x34 * 4]);
    nodes_setup(t, f + 0x30, free, listed);
}

/// The 0x50000 parts buffer (`buf` its function): the lock, slots 711
/// and 710.
pub fn buffer_setup(t: &mut magic::Twin, buf: u32) {
    let op = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
    t.set_u32(op(t, buf, 0x01), SCRATCH + 0x7e0);
    t.set_u32(SCRATCH + 0x7e0, 0x0033_0033);
    t.set_u32(op(t, buf, 0x07), 0x0711_0711);
    t.set_u32(op(t, buf, 0x11), 0x0d0d_0d0d);
    rec(t, 711, 3, |_, a| a[1] ^ 0x711);
    rec(t, 710, 4, |_, a| a[0] ^ 0x710);
}

/// The actors for the scaled start-ups: `n` of them, the selector bytes
/// at `sel` (operand `sel_at`), the characters (`chr_at`), slots 364 and
/// 16 (floats from the character; one actor's scale below `LO`, one
/// above `HI`) and 12 (a height).
fn scaled_actors(t: &mut magic::Twin, f: u32, n: u32, (n_at, sel_at, chr_at): (u32, u32, u32)) {
    let op = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
    t.set_u32(op(t, f, n_at), n);
    let sel = SCRATCH + 0x180;
    t.set_u32(op(t, f, sel_at), sel);
    for i in 0..8u8 {
        t.set_u8(sel + u32::from(i), 3 + 5 * i);
    }
    for i in 0..0x40 {
        t.set_u32(op(t, f, chr_at) + 4 * i, 0x00c0_0000 + i);
    }
    t.stub_st0(364, |_, a| match a[0] & 0xff {
        3 => 1e-6,
        8 => 1e6,
        x => f64::from(x) * 0.75,
    });
    t.stub_st0(16, |_, a| f64::from(a[0] & 0xff) * 0.5 + 1.0);
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * 0.0625);
}

/// What `r11_objects::three_nodes_kind` (`magic_0126`, at `f`) needs.
pub fn three_nodes_kind_setup(t: &mut magic::Twin, f: u32, n: u32, free: u16, listed: bool) {
    let op = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
    buffer_setup(t, t.img.rel32_target(f + 0x3c));
    object_slots(t);
    object_start_setup(t, t.img.rel32_target(f + 0x111), (0x80, 0x9b));
    scaled_actors(t, f, n, (0x41, 0x7e, 0x89));
    // the kind check: its own actor and list, slot 390 a listed kind for
    // every other actor
    let kc = t.img.rel32_target(f + 0xee);
    t.set_u32(op(t, kc, 0x09), 0x11);
    t.set_u32(op(t, kc, 0x10), SCRATCH + 0x180);
    let first = t.u32(op(t, kc, 0x30));
    rec(
        t,
        390,
        1,
        move |_, a| if a[0] & 1 == 0 { first } else { 0x7777_7777 },
    );
    rec(t, 534, 1, |_, a| 0x5340_0000 ^ a[0]);
    t.write(op(t, f, 0x1a4), &[0x5c; 0x34 * 4]);
    nodes_setup(t, f + 0xec, free, listed);
}

/// What `r11_objects::three_nodes_scaled` (`magic_0127`, at `f`) needs.
pub fn three_nodes_scaled_setup(t: &mut magic::Twin, f: u32, n: u32, free: u16, listed: bool) {
    let op = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
    buffer_setup(t, t.img.rel32_target(f + 0x3c));
    object_slots(t);
    object_start_setup(t, t.img.rel32_target(f + 0xfa), (0x80, 0x9b));
    scaled_actors(t, f, n, (0x41, 0x6f, 0x87));
    t.write(op(t, f, 0x1a8), &[0x5c; 0x34 * 4]);
    nodes_setup(t, f + 0x93, free, listed);
}
