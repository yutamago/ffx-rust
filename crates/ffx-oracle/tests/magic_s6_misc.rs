//! The shared-6 list's ports (`ffx_magic::s6_misc`) against the DLLs
//! they were ported from. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

mod magic;
#[path = "magic/sa.rs"]
mod sa;

use ffx_magic::{Ret, Xmm, s6_misc};
use magic::{Call, Eax, SCRATCH, STACK_LEN, STACK_TOP, StubMem, Twin};
use sa::rec;

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

/// The word at `f + off` of the code.
fn op(t: &Twin, f: u32, off: u32) -> u32 {
    t.img.word(f + off)
}

/// `n` bytes that differ from their neighbours (`seed` picks the pattern).
fn varied(n: u32, seed: u32) -> Vec<u8> {
    (0..n).map(|i| (i * 7 + seed * 13 + 1) as u8).collect()
}

/// The frame area below the entry's stack pointer filled with a pattern
/// before a run (the stores of the run before, or zeros, would hide a
/// port's dropped store from the stubs).
fn scrub(t: &mut Twin) {
    let top = STACK_TOP - 0x100;
    t.write(top - 0x3000, &varied(0x3000, 0x5b));
}

/// Whether `a` is in the stack or the scratch area (a pointer a stub may
/// follow).
fn followable(a: u32) -> bool {
    (STACK_TOP - STACK_LEN..STACK_TOP - 0x40).contains(&a)
        || (SCRATCH..SCRATCH + 0x1_f000).contains(&a)
}

/// Records each call of `slot` (a row of its own per call, 8 rows): its
/// first `n` words and, for each that points into the stack or the
/// scratch area, the `words` words there. With `fill = Some(i)`, the
/// memory argument `i` points at gets `words` new words. Answers the
/// call's number. Uses `SCRATCH + 0x2_0000..`: don't mix with [`rec`]
/// for one slot.
fn rec_mem(t: &mut Twin, slot: u32, n: usize, words: u32, fill: Option<usize>) {
    let base = SCRATCH + 0x3_0000 + 0x800 * (slot % 0x10);
    t.stub(slot, move |m: &mut dyn StubMem, a: &[u32]| {
        let k = m.r32(base);
        m.w32(base, k + 1);
        let row = base + 4 + 0x100 * (k % 8);
        let mut at = row;
        for &x in &a[..n.min(a.len())] {
            m.w32(at, x);
            at += 4;
            if followable(x) {
                for i in 0..words {
                    let v = m.r32(x + 4 * i);
                    m.w32(at, v);
                    at += 4;
                }
            }
        }
        if let Some(j) = fill.filter(|&j| followable(a[j])) {
            for i in 0..words {
                m.w32(
                    a[j] + 4 * i,
                    0x3f80_0000 + (slot << 12) + (k << 8) + i * 0x11,
                );
            }
        }
        (slot << 16) + k
    });
}

/// Row 843: the staged nodes task of `magic_0401` and its copy in
/// `magic_0426` (another colour immediate), through every step: 0 (the
/// model and colour nodes, the first move), 10, 11 below and past the
/// count with the index below and at the limit (both list states), 12
/// below and past its count, 0x2710 (the node leaves its ring) and steps
/// no case takes.
#[test]
#[allow(clippy::too_many_lines)] // the slots the callees reach
fn staged_nodes_tasks() {
    // (0400, 0404 and 0428: the same code in own rows of those DLLs)
    for (dll, rva) in [
        (401u16, 0x6960u32),
        (426, 0x7130),
        (400, 0x6960),
        (404, 0x6960),
        (428, 0x6960),
    ] {
        staged_nodes_in(dll, rva);
    }
}

fn staged_nodes_in(dll: u16, rva: u32) {
    let Some(mut t) = twin(dll) else { return };
    let f = t.at(rva);
    // the model nodes' slots (`s3_summon::model_nodes`)
    let models = SCRATCH + 0x7a0;
    t.write(models, &[4, 0x81, 7, 0xff]);
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 2);
        models
    });
    rec(&mut t, 413, 1, |_, a| 0x4130_0000 ^ a[0]);
    rec(&mut t, 215, 1, |_, a| 0x2150_0000 ^ a[0]);
    rec(&mut t, 14, 4, |_, a| 0x0014_0000 ^ a[0]);
    t.stub_st0(333, |_, a| 0.625 + f64::from(a[0] & 3));
    // the colour node's (`s3_misc::colour_node`)
    rec_mem(&mut t, 331, 1, 4, None);
    rec_mem(&mut t, 558, 1, 4, Some(0));
    rec_mem(&mut t, 353, 1, 8, None);
    rec(&mut t, 271, 1, |_, a| 0x0271_0000 ^ a[0]);
    // the unit block's float (`sa_misc::unit_block`)
    rec(&mut t, 790, 0, |_, _| 0x0790);
    t.stub_st0(790, |_, _| 0.375);
    // the option point
    rec_mem(&mut t, 268, 3, 4, Some(2));
    // the particle's (`sb_nodes::particle_node`) and the effect node's
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec_mem(&mut t, 941, 1, 0, None);
    t.stub(941, move |_, _| table);
    rec_mem(&mut t, 714, 5, 0, None);
    rec_mem(&mut t, 325, 2, 4, Some(0));
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    let pn = t.img.rel32_target(f + 0x2c9);
    let start = t.img.rel32_target(pn + 0x88);
    t.set_u32(op(&t, start, 0x04), SCRATCH + 0x2f0);
    let task = SCRATCH + 0x300;
    let s = SCRATCH + 0x400;
    let node = SCRATCH + 0x380;
    let actor = SCRATCH + 0x5000;
    // the DLL's list: its tail is the sentinel (empty) or a node of its own
    let sentinel = op(&t, f, 0x16e);
    let tail = op(&t, f, 0x17a);
    let limit = op(&t, f, 0x342);
    for (k, (step, count, index, listed)) in [
        (0u32, 0u32, 0u32, false),
        (0, 0x50, 3, true),
        (10, 0, 1, false),
        (11, 3, 0, false),
        (11, 0xf, 0, true),
        (11, 0x10, 0, false),
        (11, 0x40, 1, true),
        (11, 0x40, 2, false),
        (12, 0x4f, 0, false),
        (12, 0x50, 0, false),
        (12, 0x51, 0, true),
        (0x2710, 7, 0, false),
        (5, 1, 0, false),
        (0xd, 1, 0, false),
        (0x2711, 1, 0, false),
        (0xffff_fff0, 1, 0, false),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.write(s, &varied(0x40, k));
        t.set_u32(s + 0x18, step);
        t.set_u32(s + 0x1c, (0.25 * k as f32).to_bits());
        t.set_u32(s + 0x20, (-1.5 + k as f32).to_bits());
        t.set_u32(s + 0x24, index);
        t.set_u32(s + 0x28, count);
        t.set_u32(task, node);
        t.set_u32(task + 4, s);
        t.write(node, &varied(0x10, k + 3));
        t.set_u32(node + 4, SCRATCH + 0x700);
        t.set_u32(node + 8, SCRATCH + 0x740);
        t.write(SCRATCH + 0x700, &varied(0x80, k));
        // the index limit: the index after this one reaches it or not
        t.set_u32(limit, index + 1 + (k & 1));
        t.write(sentinel - 0x14, &varied(0x2c, k + 9));
        t.set_u32(tail, if listed { SCRATCH + 0x580 } else { sentinel });
        t.write(SCRATCH + 0x580, &varied(0x20, k));
        // the record the option point is read from
        let r = op(&t, f, 0x1c2) + 0x98 * index;
        t.set_u32(r + 0x54, 0x0540_0000 + k);
        // the colour node's parent (a node of the DLL's): its list empty
        // (its tail itself) or holding a node
        let parent = op(&t, f, 0xab);
        t.write(parent, &varied(0x1c, k + 5));
        t.set_u32(parent + 0x10, if listed { SCRATCH + 0x5c0 } else { parent });
        t.write(SCRATCH + 0x5c0, &varied(0x20, k + 1));
        // the model group's actor and the particle's owner
        t.set_u32(op(&t, f, 0x63), actor);
        t.write(actor, &varied(0x40, k));
        t.set_u32(op(&t, f, 0x28f), SCRATCH + 0x900);
        t.write(SCRATCH + 0x900, &varied(0x80, k + 4));
        // the effect nodes' flag words
        let e = op(&t, f, 0x32a) + 0x38 * index;
        t.set_u32(e, 0xffff_0002 ^ k);
        t.set_u32(e + 4, 0x0350_0000 + k);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// Row 845: the parts' centres of `magic_0563`'s 18 models: models with
/// no polygons, a negative count, triangles, quads, strips of another
/// kind and empty strips, the division and sum slots seeing (and
/// changing) the locals.
#[test]
fn part_centres_found() {
    let Some(mut t) = twin(563) else { return };
    let f = t.at(0x1b20);
    let log = SCRATCH + 0x6000;
    t.stub(564, move |m, a| {
        let k = m.r32(log);
        m.w32(log, k + 1);
        let row = log + 4 + 0x20 * (k % 0x80);
        let by = f32::from_bits(a[2]);
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(row + 4 * i, x);
            m.w32(a[0] + 4 * i, (f32::from_bits(x) / by).to_bits());
        }
        m.w32(row + 0x10, a[0]);
        m.w32(row + 0x14, a[1]);
        m.w32(row + 0x18, a[2]);
        0x0564_0000 + k
    });
    t.stub(248, move |m, a| {
        let k = m.r32(log);
        m.w32(log, k + 1);
        let row = log + 4 + 0x20 * (k % 0x80);
        for i in 0..4 {
            let (x, y) = (m.r32(a[1] + 4 * i), m.r32(a[2] + 4 * i));
            m.w32(row + 4 * i, y);
            m.w32(
                a[0] + 4 * i,
                (f32::from_bits(x) + f32::from_bits(y)).to_bits(),
            );
        }
        m.w32(row + 0x10, a[0]);
        m.w32(row + 0x18, a[2]);
        0x0248_0000 + k
    });
    let table = SCRATCH + 0x300;
    let s = SCRATCH + 0x8000;
    t.set_u32(op(&t, f, 0x31), table);
    t.set_u32(op(&t, f, 0x14), s);
    let model = |i: u32| SCRATCH + 0x1000 + 0x400 * i;
    // (kind, count) strips per model, and the model's polygon count
    let shapes: [(&[(u8, i16)], i16); 5] = [
        (&[], 0),
        (&[(2, 2), (6, 1)], 3),
        (&[(6, 2), (3, 2), (2, 0), (2, 1)], 5),
        (&[(2, 1)], -0x7fff),
        (&[(7, 1), (6, 1)], 2),
    ];
    for k in 0..3u32 {
        t.write(s + 0x2ec0, &varied(0x60 * 18, k));
        t.set_u32(log, 0);
        for (i, (strips, n)) in (0u32..).zip(shapes) {
            let m = model(i);
            t.write(m, &varied(0x400, i + k));
            t.set_u32(m + 4, 0x40);
            t.set_u32(m + 8, 0x300);
            t.set_u16(m + 0x10, n.cast_unsigned());
            let mut e = m + 0x40;
            for &(kind, count) in strips {
                t.set_u8(e + 1, kind);
                t.set_u16(e + 2, count.cast_unsigned());
                let size = match kind {
                    2 => 0x20,
                    6 => 0x28,
                    _ => 0,
                };
                // vertex numbers 0..15 (the table has 0x20 vertices)
                for p in 0..u32::from(count.cast_unsigned()) {
                    for j in 0..4u32 {
                        let at = e + 0x10 + size * p + if kind == 2 { 0xc } else { 0x10 };
                        t.set_u16(at + 2 * j, ((p * 5 + j * 3 + i + k) % 16) as u16);
                    }
                }
                e += 0x10 + size * u32::from(count.cast_unsigned());
            }
        }
        for c in 0..18u32 {
            t.set_u32(table + 0x23c + 0x20 * c, model((c + k) % 5));
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// A free ring of `n` nodes for the pool's take at `take` (its count and
/// head operands), 0x40 bytes apart from `at`.
fn free_ring(t: &mut Twin, take: u32, at: u32, n: u32) {
    let node = |i: u32| at + 0x40 * i;
    for i in 1..=n {
        t.write(node(i), &varied(0x1c, i + 0x30));
        t.set_u32(node(i), node(if i == 1 { n } else { i - 1 }));
        t.set_u32(node(i) + 4, node(if i == n { 1 } else { i + 1 }));
    }
    t.set_u32(t.img.word(take + 0x17), node(1));
    t.set_u16(t.img.word(take + 0x04), n as u16);
}

/// Row 846: `magic_0168`'s second spin stage: the two objects on the
/// object list or not (one of them the list's head), the battle scene
/// that lifts the point and another, the task list empty or not.
#[test]
#[allow(clippy::too_many_lines)] // the slots the callees reach
fn spin_stages_two() {
    let Some(mut t) = twin(168) else { return };
    let f = t.at(0x43d0);
    rec(&mut t, 371, 2, |_, a| 0x0371_0000 ^ a[0]);
    rec_mem(&mut t, 713, 1, 0, None);
    let scene = SCRATCH + 0x7f0;
    rec(&mut t, 520, 0, move |m, _| m.r32(scene));
    rec_mem(&mut t, 235, 1, 16, Some(0));
    for s in [328u32, 329, 330] {
        rec_mem(&mut t, s, 3, 16, Some(0));
    }
    // (the point's 4 words: 16 would reach the frame's cookie)
    rec_mem(&mut t, 229, 3, 4, Some(0));
    rec_mem(&mut t, 248, 3, 4, Some(0));
    for s in [277u32, 15, 16, 17] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| f64::from(s) / 64.0 + f64::from(a[0] & 3));
    }
    rec(&mut t, 557, 0, |_, _| 0x0557_0010);
    rec(&mut t, 560, 1, |_, a| 0x0560_0000 ^ a[0]);
    // the objects' start (`s4_misc::start_object_anim`)
    for call in [0x2b5u32, 0x31f] {
        let soa = t.img.rel32_target(f + call);
        t.set_u32(op(&t, soa, 0x04), SCRATCH + 0x200);
        t.set_u32(op(&t, soa, 0x5e), SCRATCH + 0x280);
        let pa = t.img.rel32_target(soa + 0x99);
        t.set_u16(op(&t, pa, 0x04), 9);
    }
    for i in 0..0x30 {
        t.set_u32(SCRATCH + 0x3400 + 4 * i, 0x1111 * (i + 1));
    }
    t.stub(941, |_, _| SCRATCH + 0x3400);
    rec_mem(&mut t, 714, 5, 0, None);
    let take = t.img.rel32_target(f + 0x3b5);
    let b = SCRATCH + 0x8000;
    t.set_u32(op(&t, f, 0x3d), b);
    t.set_u32(op(&t, f, 0x60), SCRATCH + 0x2f0);
    let ch = 0x0c40_0001;
    t.set_u32(op(&t, f, 0x1bb), ch);
    let list = op(&t, f, 0x4d);
    let (head, tail) = (op(&t, f, 0x3bf), op(&t, f, 0x3d2));
    let nodes = SCRATCH + 0x600;
    for (k, (first, second, battle, listed)) in [
        (false, false, 0x018b_0046u32, false),
        (true, false, 0x018b_0045, true),
        (true, true, 0x018b_0046, true),
        (false, true, 0, false),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.write(b, &varied(0x3200, k));
        for (o, v) in [(0x3150u32, 0.5f32), (0xaa0, -0.25), (0xaa8, 1.5)] {
            t.set_u32(b + o, (v + k as f32).to_bits());
        }
        t.set_u32(scene, battle);
        // the object list: the objects' nodes, a node of another first
        let mut items = vec![0x0bad_0000 + k];
        if first {
            items.push(b + 0x720);
        }
        if second {
            items.push(b + 0x850);
        }
        if k == 2 {
            items.reverse();
        }
        for (i, item) in (0u32..).zip(&items) {
            let n = nodes + 0x10 * i;
            t.set_u32(n, if i == 0 { 0 } else { n - 0x10 });
            let at_end = i as usize + 1 == items.len();
            t.set_u32(n + 4, if at_end { 0 } else { n + 0x10 });
            t.set_u32(n + 8, *item);
        }
        t.set_u32(list, nodes);
        t.set_u32(list + 4, nodes + 0x10 * (items.len() as u32 - 1));
        t.set_u32(list + 8, nodes);
        free_ring(&mut t, take, SCRATCH + 0x1000, 4);
        t.write(SCRATCH + 0x1400, &varied(0x1c, 9));
        let old = if listed { SCRATCH + 0x1400 } else { 0 };
        t.set_u32(head, old);
        t.set_u32(tail, old);
        let fader = t.img.rel32_target(f + 0x3f9);
        let (fh, ft) = (op(&t, fader, 0x37), op(&t, fader, 0x4f));
        if fh != head {
            t.set_u32(fh, old);
            t.set_u32(ft, old);
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// Row 847: `magic_0511`'s three points around the summoner, for two
/// yaws and heights, the matrix and vector slots seeing (and filling)
/// the locals.
#[test]
fn three_points_set_around() {
    let Some(mut t) = twin(511) else { return };
    let f = t.at(0x4d80);
    rec_mem(&mut t, 235, 1, 16, Some(0));
    for s in [328u32, 329] {
        rec_mem(&mut t, s, 3, 16, Some(0));
    }
    rec_mem(&mut t, 229, 3, 4, Some(0));
    rec_mem(&mut t, 248, 3, 4, Some(0));
    // the objects' start (`s4_misc::start_object_anim`)
    for call in [0x38bu32, 0x405] {
        let soa = t.img.rel32_target(f + call);
        t.set_u32(op(&t, soa, 0x04), SCRATCH + 0x200);
        t.set_u32(op(&t, soa, 0x5e), SCRATCH + 0x280);
        let pa = t.img.rel32_target(soa + 0x99);
        t.set_u16(op(&t, pa, 0x04), 9);
    }
    for i in 0..0x30 {
        t.set_u32(SCRATCH + 0x3400 + 4 * i, 0x1111 * (i + 1));
    }
    t.stub(941, |_, _| SCRATCH + 0x3400);
    rec_mem(&mut t, 714, 5, 0, None);
    let b = SCRATCH + 0x8000;
    t.set_u32(op(&t, f, 0x1f), b);
    for (k, (yaw, h)) in [(0.5f32, 2.0f32), (-1.25, -0.75)].into_iter().enumerate() {
        let k = k as u32;
        t.write(b, &varied(0x2300, k));
        t.set_u32(b + 0x2258, yaw.to_bits());
        for i in 0..4u32 {
            t.set_u32(b + 0x1a60 + 4 * i, (1.5 * i as f32 - 2.0 + yaw).to_bits());
        }
        t.set_u32(op(&t, f, 0x17), h.to_bits());
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// Rows 848 and 213: the sprite fan of `magic_0598` and `magic_0162`
/// (another identity in the census: other constants), its start (the
/// task list empty and not) and its task through the countdown, the fade
/// steps and the end (the pad bit both ways).
#[test]
fn sprite_fans() {
    for (dll, rva) in [(598u16, 0x4530u32), (162, 0x89b0)] {
        sprite_fans_in(dll, rva);
    }
}

#[allow(clippy::too_many_lines)] // the start and the task
fn sprite_fans_in(dll: u16, rva: u32) {
    let Some(mut t) = twin(dll) else { return };
    let f = t.at(rva);
    let task_f = f + 0x70;
    let take = t.img.rel32_target(f + 0x30);
    let (head, tail) = (op(&t, f, 0x36), op(&t, f, 0x4a));
    let s = SCRATCH + 0x4000;
    for listed in [false, true] {
        free_ring(&mut t, take, SCRATCH + 0x1000, 3);
        t.write(s, &varied(0x140, u32::from(listed)));
        t.write(SCRATCH + 0x1400, &varied(0x1c, 9));
        let last = if listed { SCRATCH + 0x1400 } else { 0 };
        t.set_u32(head, last);
        t.set_u32(tail, last);
        scrub(&mut t);
        t.check_entry(f, Call::this(s), &[], Eax::Checked);
    }
    // the task
    let pad = SCRATCH + 0x7f0;
    rec(&mut t, 245, 1, move |m, a| {
        let v = m.r32(pad);
        m.w32(a[0], v);
        0x245
    });
    rec(&mut t, 821, 1, |_, a| 0x0821_0000 ^ a[0]);
    rec(&mut t, 822, 4, |_, a| 0x0822_0000 ^ a[0]);
    rec(&mut t, 823, 2, |_, a| 0x0823_0000 ^ a[0]);
    rec(&mut t, 824, 4, |_, a| 0x0824_0000 ^ a[0]);
    let c = SCRATCH + 0x5000;
    t.set_u32(op(&t, task_f, 0x1e2), c);
    let task = SCRATCH + 0x300;
    for (k, (wait, m, r, bit)) in [
        (3i32, 5i32, 2i32, 0u32),
        (0, 5, 2, 1),
        (0, 1, 2, 0),
        (0, 0, 1, 1),
        (0, 0, 0, 0),
        (-1, -2, -1, 1),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.write(s, &varied(0x140, k + 2));
        t.write(c, &varied(0x60, k));
        // the screen place's mode: one with no slots
        t.set_u16(s + 0x124, 9);
        t.set_u32(s + 0x110, wait.cast_unsigned());
        t.set_u32(s + 0x104, 50);
        t.set_u32(s + 0x108, m.cast_unsigned());
        t.set_u32(s + 0x10c, r.cast_unsigned());
        t.set_u32(s + 0x114, (0x123 + 0x40 * k).cast_signed().cast_unsigned());
        t.set_u32(s + 0x118, 0x0456 - 0x80 * k);
        for (o, v) in [(0xf8u32, 1.5f32), (0xfc, -0.5), (0x100, 2.25)] {
            t.set_u32(s + o, (v + k as f32).to_bits());
        }
        t.set_u32(pad, 0x0100_0000 | bit);
        // the task node at rest for the give-back
        t.write(task, &varied(0x20, k));
        t.set_u32(task, 0);
        t.set_u32(task + 4, 0);
        t.set_u32(task + 8, SCRATCH + 0x340);
        t.set_u32(SCRATCH + 0x34c, 0);
        t.set_u32(task + 0x14, s);
        scrub(&mut t);
        t.check_entry(task_f, Call::default(), &[task], Eax::Checked);
    }
}

/// Row 81: `magic_0341`'s part start with its motion (internal: `ecx`,
/// `edx`, `xmm2`, `xmm3` and seven stack floats), truncations in range,
/// negative and out of range.
#[test]
fn parts_given_motion() {
    let Some(mut t) = twin(341) else { return };
    let f = t.at(0x1000);
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    let start = t.img.rel32_target(f + 0x43);
    t.set_u32(op(&t, start, 0x04), SCRATCH + 0x7f8);
    t.set_u32(op(&t, f, 0x7), SCRATCH + 0x2f0);
    t.set_u32(op(&t, f, 0x1a), 0x0dd0_0001);
    let part = SCRATCH + 0x1000;
    for (k, (index, b, c, d)) in [
        (0u32, 2.75f32, -1.5f32, 100.9f32),
        (3, -0.5, 3.0e9, f32::NAN),
        (7, 65535.9, -2.0e10, -7.25),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.write(part, &varied(0x80, k));
        t.set_u32(SCRATCH + 0x2f0, 0x02f0_0000 + k);
        let c_ = Call {
            ecx: part,
            edx: index,
            ..Call::default()
        }
        .with_xmm(&[
            Xmm::f32(9.0),
            Xmm::f32(8.0),
            Xmm::f32(0.25 + k as f32),
            Xmm::f32(-3.5 * k as f32),
        ]);
        let args = [
            1.5f32.to_bits() + k,
            b.to_bits(),
            c.to_bits(),
            d.to_bits(),
            (-0.75f32).to_bits(),
            12.5f32.to_bits() ^ k,
            0x4000_0000 + k,
        ];
        scrub(&mut t);
        t.check(f, c_, &args, Eax::Checked, |g, img, fr| {
            Ret::eax(s6_misc::part_motion(g, img, fr))
        });
    }
}

/// [`rec`] that also records the word at `lock` (a game lock the
/// original holds around the call) in the row's last word.
fn rec_lock(
    t: &mut Twin,
    slot: u32,
    n: usize,
    lock: u32,
    ret: impl Fn(&mut dyn StubMem, &[u32]) -> u32 + 'static,
) {
    let row = SCRATCH + 0x2_0000 + 0x80 * (slot % 0x400);
    t.stub(slot, move |m, a| {
        let k = m.r32(row);
        m.w32(row, k + 1);
        for (i, w) in (0u32..).zip(&a[..n]) {
            m.w32(row + 4 + 4 * i, *w ^ k);
        }
        let held = m.r32(lock);
        m.w32(row + 0x7c, held ^ k);
        ret(m, a)
    });
}

/// Row 82: `magic_0070`'s motion sequence step: past the last step, a
/// loaded model and one that is not (the strip's byte 0 and not), the
/// time wrapping into the next step and not; the lock and the local the
/// slots see.
#[test]
fn motion_sequence_steps() {
    let Some(mut t) = twin(70) else { return };
    let f = t.at(0x15b0);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(op(&t, f, 0x24), lock);
    let loaded = SCRATCH + 0x7e0;
    rec_lock(&mut t, 779, 2, lock, |_, _| 0x779);
    rec_lock(&mut t, 773, 2, lock, move |m, _| m.r32(loaded));
    rec_lock(&mut t, 774, 5, lock, |m, a| {
        // the local the slot gets (the sequence) logged too
        let q = m.r32(a[2]);
        m.w32(SCRATCH + 0x7c0, q);
        0x774
    });
    rec_lock(&mut t, 775, 2, lock, |_, _| 0x775);
    rec(&mut t, 776, 1, |_, a| 0x0776_0000 | (a[0] << 4));
    rec(&mut t, 777, 1, |_, a| 0x0777_0001 ^ a[0]);
    rec_lock(&mut t, 778, 6, lock, |m, a| {
        let q = m.r32(a[2]);
        m.w32(SCRATCH + 0x7c4, q);
        0x778
    });
    let s = SCRATCH + 0x300;
    let q = SCRATCH + 0x1000;
    let m = SCRATCH + 0x600;
    for (k, (step, time, count, is_loaded, byte, len)) in [
        (3u16, 0u16, 3i16, 0u32, 0u8, 0x100i16),
        (5, 0, 3, 1, 0, 0x100),
        (0, 0x80, 3, 1, 0, 0x200),
        (1, 0x1f0, 3, 0, 0, 0x200),
        (2, 0x10, 3, 0, 0x2a, 0x110),
        (0, 0x7f00, 2, 0, 9, -0x10),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.write(q, &varied(0x200, k));
        t.set_u16(q + 6, count.cast_unsigned());
        for i in 0..4u32 {
            let rec_at = q + 0x10 + 8 * i;
            t.set_u16(rec_at, (0x80 + 0x20 * i) as u16);
            t.set_u16(rec_at + 2, len.cast_unsigned());
            let strip = q + 0x80 + 0x20 * i;
            t.set_u32(strip + 4, 0x10);
            t.set_u8(strip + 0x14, byte);
        }
        t.write(s, &varied(0x10, k));
        t.set_u16(s, step);
        t.set_u16(s + 2, time);
        t.write(m, &varied(0x40, k + 1));
        t.set_u32(loaded, is_loaded);
        t.set_u32(lock, 0x5a5a_0000 + k);
        let c = Call {
            ecx: s,
            edx: q,
            ..Call::default()
        };
        scrub(&mut t);
        t.check(f, c, &[m], Eax::Checked, |g, img, fr| {
            Ret::eax(s6_misc::motion_sequence_step(g, img, fr))
        });
    }
}

/// Row 89: `magic_0102`'s object start with the two-word animation
/// reset: the object list empty and not, the pool empty.
#[test]
fn objects_started_anim_2() {
    let Some(mut t) = twin(102) else { return };
    let f = t.at(0x2140);
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    let start = t.img.rel32_target(f + 0x84);
    t.set_u32(op(&t, start, 0x04), SCRATCH + 0x7f8);
    t.set_u32(op(&t, f, 0x04), SCRATCH + 0x2f0);
    t.set_u32(op(&t, f, 0x5e), SCRATCH + 0x280);
    let pa = t.img.rel32_target(f + 0x99);
    let free = op(&t, pa, 0x04);
    let tail = op(&t, f, 0xa4);
    let obj = SCRATCH + 0x1000;
    for (k, (index, nodes, listed)) in [(0u32, 5u16, false), (3, 2, true), (1, 0, true)]
        .into_iter()
        .enumerate()
    {
        let k = k as u32;
        t.write(obj, &varied(0x140, k));
        t.set_u32(SCRATCH + 0x2f0, 0x02f0_0000 + k);
        t.set_u16(free, nodes);
        t.write(SCRATCH + 0x1400, &varied(0x10, k));
        t.set_u32(tail, if listed { SCRATCH + 0x1400 } else { 0 });
        let c = Call {
            edx: obj,
            ecx: 0x0ecc_0000 + k,
            ..Call::default()
        }
        .with_xmm(&[
            Xmm::f32(9.0),
            Xmm::f32(8.0),
            Xmm::f32(7.0),
            Xmm::f32(1.25 + k as f32),
        ]);
        let args = [index, 0x0bad_0001, 0x0a0a_0000 + k, 0x0b0b_0000 + k];
        scrub(&mut t);
        t.check(f, c, &args, Eax::Checked, |g, img, fr| {
            Ret::eax(s6_misc::start_object_anim_2(g, img, fr))
        });
    }
}

/// Row 90: `magic_0430`'s objects freed: an empty list, one and three
/// objects, the same object twice (the cursor stepping again), and the
/// free slot changing the list so that the node found is in the middle,
/// is the tail, or is not there.
#[test]
fn objects_all_freed() {
    let Some(mut t) = twin(430) else { return };
    let f = t.at(0x7f70);
    let poison = SCRATCH + 0x7e0;
    rec(&mut t, 713, 1, move |m, a| {
        // the game's free may take the object off a node itself
        let p = m.r32(poison);
        if p != 0 {
            m.w32(p + 8, 0x0bad_0713);
            m.w32(poison, 0);
        }
        0x0713_0000 ^ a[0]
    });
    t.set_u32(op(&t, f, 0x31), SCRATCH + 0x2f0);
    let list = op(&t, f, 0x8);
    let node = |i: u32| SCRATCH + 0x1000 + 0x20 * i;
    let obj = |i: u32| SCRATCH + 0x2000 + 0x200 * i;
    // (the objects the nodes hold, the node the free slot changes)
    let cases: [(&[u32], Option<u32>); 6] = [
        (&[], None),
        (&[1], None),
        (&[1, 2, 3], None),
        (&[1, 1, 2], None),
        (&[1, 1, 2], Some(0)),
        (&[1, 1], Some(0)),
    ];
    for (k, (items, changed)) in (0u32..).zip(cases) {
        let n = items.len() as u32;
        for (i, &o) in (0u32..).zip(items) {
            t.write(node(i), &varied(0x20, i + k));
            t.set_u32(node(i), if i == 0 { 0 } else { node(i - 1) });
            t.set_u32(node(i) + 4, if i + 1 == n { 0 } else { node(i + 1) });
            t.set_u32(node(i) + 8, obj(o));
            t.write(obj(o), &varied(0x200, o + k));
            t.set_u16(obj(o) + 0x124, 0x8123 + o as u16);
        }
        let (head, tail) = if n == 0 {
            (0, 0)
        } else {
            (node(0), node(n - 1))
        };
        t.set_u32(list, head);
        t.set_u32(list + 4, tail);
        t.set_u32(list + 8, 0x0bad_0008);
        t.set_u32(list + 0xc, n + 5);
        t.set_u32(poison, changed.map_or(0, node));
        t.set_u32(SCRATCH + 0x2f0, 0x02f0_0000 + k);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// Row 91: `magic_0370`'s sprite system step: no sprites, a sprite that
/// ends (released and cleared), a dead one, and held ones with gravity
/// and the speed cap on and off (velocities above, below and at 0
/// against the limit). The original's `eax` after the square root is
/// the CRT's (junk).
#[test]
fn sprite_systems_stepped() {
    let Some(mut t) = twin(370) else { return };
    let f = t.at(0x2d90);
    rec(&mut t, 248, 3, |m, a| {
        for i in 0..3 {
            let x = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, x.to_bits());
        }
        0x248
    });
    let table = SCRATCH + 0x3400;
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(op(&t, f, 0x58), lock);
    rec_lock(&mut t, 779, 2, lock, |_, _| 0x779);
    let sys = SCRATCH + 0x300;
    let parts = SCRATCH + 0x1000;
    for (k, (count, gravity, cap)) in [
        (0u32, 0u32, 0u32),
        (4, 1, 0),
        (4, 0, 1),
        (4, 1, 1),
        (3, 0, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(lock, 0x5a5a_0000 + k);
        t.write(sys, &varied(0x20, k));
        for (o, v) in [(0u32, count), (4, parts), (8, gravity), (0x10, cap)] {
            t.set_u32(sys + o, v);
        }
        t.set_u32(sys + 0xc, (-0.5f32).to_bits());
        t.set_u32(sys + 0x14, 2.0f32.to_bits());
        // (live, held, the velocity)
        for (i, (live, hold, v)) in [
            (1u32, 0u32, [1.0f32, 2.0, 2.0]),
            (0, 1, [1.0, 1.0, 1.0]),
            (1, 1, [3.0, 4.0, 0.0]),
            (1, 1, [0.5, 0.25, -0.5]),
        ]
        .into_iter()
        .enumerate()
        {
            let p = parts + 0x94 * i as u32;
            t.write(p, &varied(0x94, i as u32 + k));
            t.set_u32(p, live * (0x100 + i as u32));
            t.set_u32(p + 0x2c, hold);
            t.set_u32(p + 0x28, 1);
            t.set_u32(p + 0x30, 0x0300_0000 + i as u32);
            for (j, x) in (0u32..).zip(v) {
                t.set_u32(p + 0x14 + 4 * j, x.to_bits());
            }
        }
        // a sprite at rest: the cap's zero length
        if k == 3 {
            for j in 0..3u32 {
                t.set_u32(parts + 0x94 + 0x14 + 4 * j, 0);
            }
            t.set_u32(parts + 0x94, 0x101);
            t.set_u32(sys + 0xc, 0);
        }
        let eax = if cap != 0 { Eax::Junk } else { Eax::Checked };
        let c = Call {
            ecx: sys,
            eax: 0x0eaa_0000 + k,
            ..Call::default()
        };
        scrub(&mut t);
        t.check(f, c, &[], eax, |g, img, fr| {
            Ret::eax(s6_misc::sprites_step(g, img, fr))
        });
    }
}

/// Row 103: `magic_0393`'s countdown tint, before and after its first
/// call, for counts below, at and past 10 (negative quotients truncate).
#[test]
fn countdown_tints() {
    let Some(mut t) = twin(393) else { return };
    let f = t.at(0x13e0);
    rec(&mut t, 628, 1, |_, a| 0x0628_0000 ^ a[0]);
    rec(&mut t, 821, 1, |_, a| 0x0821_0000 ^ a[0]);
    rec(&mut t, 915, 5, |_, a| 0x0915_0000 ^ a[3]);
    rec(&mut t, 802, 6, |_, a| 0x0802_0000 ^ a[0]);
    // the tint's scale: one that is not 1
    t.set_u32(op(&t, f, 0x70), 0.75f32.to_bits());
    for (on, count) in [
        (0u32, 0i32),
        (1, 3),
        (1, 10),
        (0, 17),
        (1, -0x0200_0001),
        (1, i32::MIN),
    ] {
        t.set_u32(op(&t, f, 0x2), on);
        t.set_u32(op(&t, f, 0x3e), count.cast_unsigned());
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// Row 104: `magic_0053`'s timed actor task through its three waits:
/// counting, each one's moment (the actor -1 and a selected one), and
/// the end (the node given back).
#[test]
fn actor_timed_tasks() {
    let Some(mut t) = twin(53) else { return };
    let f = t.at(0x2b60);
    rec(&mut t, 256, 5, |_, a| 0x0256_0000 ^ a[0]);
    let sel = SCRATCH + 0x700;
    t.write(sel, &[3, 1, 2, 0]);
    let a256 = t.img.rel32_target(f + 0x9b);
    for (fo, base) in [(f, (0x31u32, 0x38u32, 0x43u32)), (a256, (0x06, 0x0d, 0x18))] {
        t.set_u32(op(&t, fo, base.0), 0x0ab0_0000);
        t.set_u32(op(&t, fo, base.1), sel);
        for i in 0..4 {
            t.set_u32(op(&t, fo, base.2) + 4 * i, 0x0cc0_0000 + i);
        }
    }
    let task = SCRATCH + 0x300;
    let s = SCRATCH + 0x1000;
    for (k, (a, b, c, actor)) in [
        (2i32, 0i32, 0i32, 1u32),
        (0, 0, 0, u32::MAX),
        (0, 0, 0, 2),
        (-1, 3, 0, 1),
        (-1, 0, 0, u32::MAX),
        (-1, 0, 0, 3),
        (-1, -1, 4, 0),
        (-1, -1, 0, 0),
        (-1, -1, -2, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.write(s, &varied(0x40, k));
        t.set_u32(s + 0x1c, a.cast_unsigned());
        t.set_u32(s + 0x24, b.cast_unsigned());
        t.set_u32(s + 0x2c, c.cast_unsigned());
        t.set_u32(s + 0x28, actor);
        t.write(task, &varied(0x20, k));
        t.set_u32(task, 0);
        t.set_u32(task + 4, 0);
        t.set_u32(task + 8, SCRATCH + 0x340);
        t.set_u32(SCRATCH + 0x34c, 0);
        t.set_u32(task + 0x14, s);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// Row 106: `magic_0106`'s mean target offset of a point in `edx`: no
/// targets, some live, none live (0/0), all live, a negative count.
#[test]
fn targets_mean_offsets_of() {
    let Some(mut t) = twin(106) else { return };
    let f = t.at(0x69b0);
    let live = SCRATCH + 0x7e0;
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x325
    });
    rec(&mut t, 684, 1, move |m, a| m.r32(live) >> (a[0] & 7) & 1);
    let log = SCRATCH + 0x3000;
    rec(&mut t, 531, 2, move |m, a| {
        let k = m.r32(log);
        m.w32(log, k + 1);
        for i in 0..3 {
            let old = m.r32(a[1] + 4 * i);
            m.w32(log + 4 + 0x10 * (k % 8) + 4 * i, old);
            m.w32(a[1] + 4 * i, (a[0] as f32 * 1.25 - i as f32).to_bits());
        }
        0x531 ^ a[0]
    });
    // the mean's scale: one that is not 1
    t.set_u32(op(&t, f, 0xfe), 1.5f32.to_bits());
    let out = SCRATCH + 0x300;
    let point = SCRATCH + 0x340;
    let ids = op(&t, f, 0x55);
    for (n, mask) in [
        (0u32, 0u32),
        (3, 0b101),
        (5, 0),
        (4, 0xff),
        (0x8000_0000, 1),
    ] {
        for i in 0..8u32 {
            t.set_u8(ids + i, (i * 3 % 8) as u8);
        }
        t.set_u32(op(&t, f, 0x48), n);
        t.set_u32(live, mask);
        t.write(out, &varied(0x10, n));
        for (i, v) in (0u32..).zip([0.5f32, -2.0, 3.25, 7.0]) {
            t.set_u32(point + 4 * i, (v + n as f32).to_bits());
        }
        let c = Call {
            ecx: out,
            edx: point,
            ..Call::default()
        };
        scrub(&mut t);
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(s6_misc::targets_mean_offset_of(g, img, fr))
        });
    }
}

/// The trap of table word `s` (a callback the harness answers as slot `s`).
fn trap(s: u32) -> u32 {
    magic::TRAPS + 16 * s
}

/// Rows 114 and 115: `magic_0070`'s spring walk (and its thunk): a node already in the pass, one
/// with no children and ones with three and two (their methods logging
/// the cursor), the spring's scale below and from its bound.
#[test]
fn spring_walks() {
    let Some(mut t) = twin(70) else { return };
    let f = t.at(0x5ac0);
    let thunk = t.at(0x5c00);
    // the children's and the parent's method: a table word answered as
    // slot 1000
    t.set_u32(magic::TABLE + 4 * 1000, trap(1000));
    let log = SCRATCH + 0x6000;
    t.stub(1000, move |m, a| {
        let k = m.r32(log);
        m.w32(log, k + 1);
        let row = log + 4 + 0x10 * (k % 0x20);
        m.w32(row, a[0]);
        m.w32(row + 4, m.r32(a[0]));
        m.w32(row + 8, m.r32(a[0] + 4));
        // a child's walk marks its cursor moved
        m.w32(a[0] + 4, m.r32(a[0] + 4) ^ 0x0100);
        0x1000 + k
    });
    rec(&mut t, 234, 3, |m, a| {
        for i in 0..3 {
            let x = f32::from_bits(m.r32(a[1] + 4 * i)) - f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, x.to_bits());
        }
        0x234
    });
    rec(&mut t, 233, 3, |m, a| {
        for i in 0..3 {
            let x = f32::from_bits(m.r32(a[1] + 4 * i)) * f32::from_bits(a[2]);
            m.w32(a[0] + 4 * i, x.to_bits());
        }
        0x233
    });
    let vt = SCRATCH + 0x700;
    t.set_u32(vt + 0x1c, trap(1000));
    let c = SCRATCH + 0x300;
    let n = SCRATCH + 0x1000;
    let parent = SCRATCH + 0x1400;
    let child = |i: u32| SCRATCH + 0x1800 + 0x40 * i;
    for o in [0x8du32, 0xb2] {
        t.set_u32(op(&t, f, o), 0.25f32.to_bits());
    }
    for (k, (children, same, s44)) in [
        (0u32, true, 1.0f32),
        (0, false, 0.5),
        (3, false, 2.0),
        (2, false, 0.1),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.write(c, &varied(8, k));
        t.set_u32(c, n);
        t.set_u8(c + 5, 7);
        t.write(n, &varied(0x60, k));
        t.set_u8(n + 0x1a, if same { 7 } else { 3 });
        t.set_u32(n + 0x10, if children == 0 { n } else { child(0) });
        for i in 0..children {
            t.write(child(i), &varied(0x20, i));
            t.set_u32(child(i), vt);
            t.set_u32(
                child(i) + 8,
                if i + 1 == children { n } else { child(i + 1) },
            );
        }
        t.set_u32(n + 0x3c, parent);
        t.write(parent, &varied(0x40, k + 1));
        t.set_u32(parent, vt);
        for (o, v) in [(0x44u32, s44), (0x48, 0.5), (0x4c, 0.25), (0x50, 1.5)] {
            t.set_u32(n + o, (v + 0.125 * k as f32).to_bits());
        }
        for i in 0..4u32 {
            t.set_u32(n + 0x2c + 4 * i, (1.0 + i as f32).to_bits());
            t.set_u32(parent + 0x1c + 4 * i, (3.0 - 2.0 * i as f32).to_bits());
        }
        let call = Call {
            ecx: c,
            eax: 0x0eaa_0000 + k,
            ..Call::default()
        };
        scrub(&mut t);
        // and through row 115, the register thunk over it
        t.check_thunked(f, thunk, false, call, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(s6_misc::spring_walk(g, img, fr))
        });
    }
}

/// A counter's set-up: (state, timer, value, step).
type Counter = (u32, i32, i32, i32);

/// Row 142: `magic_0644`'s three counters through every state: done,
/// counting (and at the end of the count), waiting (another's turn, the
/// flag, slot 651 at 0 and not), the run down (low half above the step
/// and not), the bounce (the table and its end), a negative value and an
/// unknown state.
#[test]
fn counters_stepped() {
    let Some(mut t) = twin(644) else { return };
    let f = t.at(0x5b80);
    let answer = SCRATCH + 0x7e0;
    rec(&mut t, 651, 0, |_, _| 0x651);
    t.stub_st0(651, move |m, _| f64::from(m.r32(answer)) * 0.5);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(&mut t, 220, 3, |_, a| 0x0220_0000 ^ a[0]);
    let table = op(&t, f, 0x145);
    for i in 0..10u32 {
        t.set_u32(table + 4 * i, 0x0100 * i + 3);
    }
    let flags = op(&t, f, 0x91);
    let o = SCRATCH + 0x1000;
    let words = SCRATCH + 0x700;
    // (state, timer, value, step) per counter; whose turn; flags; slot 651
    let cases: [([Counter; 3], u32, u16, u32); 8] = [
        ([(0, 0, 5, 0), (0, 0, 7, 0), (0, 0, 9, 0)], 0, 0, 0),
        ([(1, 5, 0x50, 0), (1, 1, 0x40, 0), (9, 0, -3, 0)], 0, 0, 0),
        ([(2, 0, 0x30, 2), (2, 0, 0x30, 2), (2, 0, 0x30, 2)], 1, 0, 1),
        ([(2, 0, 0x30, 2), (2, 0, 0x30, 2), (0, 0, 0, 0)], 0, 0x20, 1),
        (
            [(2, 0, 0x0003_0001, 2), (0, 0, 0, 0), (0, 0, 0, 0)],
            0,
            0,
            0,
        ),
        (
            [(3, 0, 0x0002_0010, 4), (3, 0, 0x0002_0003, 4), (4, 3, 0, 0)],
            2,
            0,
            0,
        ),
        ([(4, 1, 5, 0), (4, 9, -1, 0), (3, 0, -0x10, 4)], 0, 0, 0),
        (
            [(2, 0, -0x7fff_0000, 0x10), (1, 0, 0, 0), (5, 0, 1, 1)],
            0,
            0,
            1,
        ),
    ];
    for (k, (counters, turn, fl, zero)) in (0u32..).zip(cases) {
        t.write(o, &varied(0x70, k));
        t.set_u32(o + 0x54, 0x40 + k);
        t.set_u32(o + 0x64, turn);
        t.set_u16(flags, fl | 0x0100);
        t.set_u32(answer, 1 - zero);
        for (i, (state, timer, value, step)) in (0u32..).zip(counters) {
            let c = o + 0x1c * i;
            t.set_u32(c, words + 4 * i);
            t.set_u32(words + 4 * i, 0x11 * (i + 1) + k);
            t.set_u32(c + 4, value.cast_unsigned());
            t.set_u32(c + 8, step.cast_unsigned());
            t.set_u32(c + 0xc, timer.cast_unsigned());
            t.set_u32(c + 0x18, state);
        }
        let call = Call {
            ecx: o,
            ..Call::default()
        };
        scrub(&mut t);
        t.check(f, call, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(s6_misc::counters_step(g, img, fr))
        });
    }
}

/// Row 164: `magic_0565`'s selection groups: lists with and without a
/// handle, entries found in the selection and not, an empty selection.
#[test]
fn selection_groups_matched() {
    let Some(mut t) = twin(565) else { return };
    let f = t.at(0x6ba0);
    let lists = SCRATCH + 0x600;
    let plan = SCRATCH + 0x7c0;
    // per k: the handle (0: none) and the count, from the plan
    rec(&mut t, 645, 4, move |m, a| {
        let k = a[2];
        let h = m.r32(plan + 8 * k);
        m.w32(a[3], h);
        m.w32(a[1], m.r32(plan + 8 * k + 4));
        lists + 0x20 * k
    });
    let s = SCRATCH + 0x1_0000;
    t.set_u32(op(&t, f, 0x7), s);
    t.set_u32(op(&t, f, 0x40), 0x0aaa_0001);
    let sel = SCRATCH + 0x700;
    t.set_u32(op(&t, f, 0x8b), sel);
    t.write(sel, &[5, 3, 9, 3, 7]);
    for (k, (n, plans)) in [
        (5u32, [(0x11u32, 3u32), (0, 2), (0x13, 5), (0x14, 0)]),
        (0, [(0x21, 2), (0x22, 1), (0, 0), (0, 0)]),
        (3, [(0, 0), (0, 0), (0, 0), (0x31, 4)]),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(op(&t, f, 0x18), n);
        for (i, (h, c)) in (0u32..).zip(plans) {
            t.set_u32(plan + 8 * i, h);
            t.set_u32(plan + 8 * i + 4, c);
        }
        for i in 0..0x80u32 {
            t.set_u8(lists + i, [3u8, 7, 5, 1, 9, 0, 3, 2][(i + k) as usize % 8]);
        }
        t.write(s + 0xebf0, &varied(0xb0, k));
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// Row 168: `magic_0515`'s path follower: the last frame (the last key),
/// a time inside the path and one past its end (held below `n - D`).
/// After the samples the original's `eax` is their scratch (junk).
#[test]
fn paths_stepped() {
    let Some(mut t) = twin(515) else { return };
    let f = t.at(0x1c00);
    let s = SCRATCH + 0x300;
    let out = SCRATCH + 0x380;
    let keys = SCRATCH + 0x1000;
    let weights = SCRATCH + 0x400;
    // the mix's constants: ones that are not 1
    t.set_u32(op(&t, f, 0x29), 3.5f32.to_bits());
    for (k, (left, time, step, n)) in [
        (1u32, 0.0f32, 0.0f32, 4u32),
        (5, 0.25, 0.125, 4),
        (5, 0.5, 0.45, 3),
        (3, 0.75, 0.5, 6),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.write(s, &varied(0x40, k));
        t.set_u32(s + 0x20, keys);
        t.set_u32(s + 0x28, weights);
        t.set_u32(s + 0x2c, time.to_bits());
        t.set_u32(s + 0x30, step.to_bits());
        t.set_u32(s + 0x34, left);
        t.set_u32(s + 0x38, n);
        for i in 0..24u32 {
            t.set_u32(keys + 4 * i, (i as f32 * 0.75 - 3.0 + k as f32).to_bits());
        }
        for (i, v) in (0u32..).zip([0.5f32, 1.25, 2.0, 3.5, 4.0]) {
            t.set_u32(weights + 4 * i, (v * (k as f32 + 1.0)).to_bits());
        }
        t.write(out, &varied(0x10, k));
        let eax = if left == 1 { Eax::Checked } else { Eax::Junk };
        let c = Call {
            ecx: s,
            edx: out,
            ..Call::default()
        };
        scrub(&mut t);
        t.check(f, c, &[], eax, |g, img, fr| {
            Ret::eax(s6_misc::path_step(g, img, fr))
        });
    }
}

/// Row 171: `magic_0188`'s motion script step: the script's end, a
/// motion started (and one that fails, skipping to the end), the frame
/// reached by the record's frame and by the motion's, not reached, the
/// next motion started and the end after a step, with and without the
/// callback.
#[test]
fn motion_scripts_stepped() {
    let Some(mut t) = twin(188) else { return };
    let f = t.at(0x3970);
    let started = SCRATCH + 0x7e0;
    let frame = SCRATCH + 0x7e4;
    rec(&mut t, 37, 3, |_, a| 0x0037_0000 ^ a[2]);
    rec(&mut t, 18, 1, move |m, _| m.r32(started));
    rec(&mut t, 22, 1, move |m, _| m.r32(frame));
    rec(&mut t, 30, 1, |_, _| 0x30);
    t.set_u32(magic::TABLE + 4 * 1001, trap(1001));
    rec(&mut t, 1001, 1, |_, a| 0x1001_0000 ^ a[0]);
    let tab = SCRATCH + 0x1000;
    t.set_u32(op(&t, f, 0xb), tab);
    t.set_u32(op(&t, f, 0x2f), 0x0c40_0001);
    // (step, playing, slot 18's answer, slot 22's frame word, the records:
    // motion, frame, callback)
    let cases: [(u32, u32, u32, u32, [(u32, u32, bool); 4]); 8] = [
        (
            1,
            0,
            0,
            0,
            [(5, 0, true), (0, 0, false), (0, 0, false), (0, 0, false)],
        ),
        (
            0,
            0,
            7,
            0x300,
            [(5, 4, true), (6, 0, false), (0, 0, false), (0, 0, false)],
        ),
        (
            0,
            0,
            u32::MAX,
            0x300,
            [(5, 4, true), (6, 0, true), (0, 0, true), (0, 0, false)],
        ),
        (
            0,
            9,
            0,
            0x300,
            [(5, 4, true), (6, 0, true), (0, 0, false), (0, 0, false)],
        ),
        (
            0,
            9,
            0,
            0x450,
            [(5, 4, false), (6, 0, true), (0, 0, false), (0, 0, false)],
        ),
        (
            1,
            4,
            0x44,
            0x380,
            [(5, 4, true), (6, 0, true), (0, 0, false), (0, 0, false)],
        ),
        (
            1,
            3,
            0,
            0x380,
            [(5, 4, true), (6, 0, false), (7, 0, true), (0, 0, false)],
        ),
        (
            0,
            9,
            0x55,
            0x5ff,
            [(5, 4, true), (6, 0, true), (0, 0, true), (0, 0, false)],
        ),
    ];
    for (k, (step, playing, answer, fw, recs_)) in (0u32..).zip(cases) {
        t.set_u32(op(&t, f, 0x2), step);
        t.set_u32(op(&t, f, 0x1e), playing);
        t.set_u32(started, answer);
        t.set_u32(frame, fw);
        for (i, (m, fr_, cb)) in (0u32..).zip(recs_) {
            let r = tab + 0x10 * i;
            t.set_u32(r, m);
            t.set_u32(r + 4, fr_);
            t.set_u32(r + 8, if cb { trap(1001) } else { 0 });
            t.set_u32(r + 0xc, 0x0a00_0000 + 0x10 * k + i);
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// Row 304: `magic_0070`'s node release: no parent, the node first,
/// last, alone and in the middle of its parent's children, motion lists
/// empty and not; the lock the slot sees.
#[test]
fn nodes_released() {
    let Some(mut t) = twin(70) else { return };
    let f = t.at(0x1f10);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(op(&t, f, 0x6b), lock);
    rec_lock(&mut t, 779, 2, lock, |m, a| {
        m.w32(SCRATCH + 0x7f0, 0x0779_0000 ^ a[0]);
        0x779
    });
    let holder = SCRATCH + 0x300;
    let n = SCRATCH + 0x1000;
    let p = SCRATCH + 0x1400;
    let (a, b) = (SCRATCH + 0x1800, SCRATCH + 0x1840);
    let recs_at = SCRATCH + 0x2000;
    // (parent, prev, next, the two counts)
    for (k, (parent, prev, next, c0, c1)) in [
        (false, a, b, 2u16, 1u16),
        (true, p, b, 0, 3),
        (true, a, p, 1, 0),
        (true, p, p, 2, 2),
        (true, a, b, 0, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(lock, 0x5a5a_0000 + k);
        t.set_u32(holder, n);
        t.write(n, &varied(0x40, k));
        t.write(p, &varied(0x20, k + 1));
        t.write(a, &varied(0x20, k + 2));
        t.write(b, &varied(0x20, k + 3));
        t.set_u32(n + 0x14, if parent { p } else { 0 });
        t.set_u32(n + 4, prev);
        t.set_u32(n + 8, next);
        t.set_u32(n + 0x24, recs_at);
        t.set_u32(n + 0x28, recs_at + 0x400);
        t.set_u16(n + 0x2c, c0);
        t.set_u16(n + 0x2e, c1);
        let c = Call {
            ecx: holder,
            ..Call::default()
        };
        scrub(&mut t);
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(s6_misc::node_release(g, img, fr))
        });
    }
}

/// Row 306: `magic_0515`'s parts' axes for no parts (the cookie word in
/// `eax`), one and three parts.
#[test]
fn parts_axes_turned() {
    let Some(mut t) = twin(515) else { return };
    let f = t.at(0x26f0);
    rec_mem(&mut t, 247, 2, 12, Some(0));
    t.stub(233, |m, a| {
        for i in 0..4 {
            let x = f32::from_bits(m.r32(a[1] + 4 * i)) * f32::from_bits(a[2]);
            m.w32(a[0] + 4 * i, x.to_bits());
        }
        0x0233_0000 ^ a[0]
    });
    let base = SCRATCH + 0x1000;
    for (k, n) in [0u32, 1, 3].into_iter().enumerate() {
        let k = k as u32;
        t.write(base, &varied(0x400, k));
        for i in 0..n {
            let p = base + 0xac + 0xcc * i;
            t.set_u16(p - 0x96, (0x1234 * (i + 1) + k) as u16);
            t.set_u32(p - 0x84, (1.5 + i as f32).to_bits());
        }
        let c = Call {
            ecx: base,
            edx: n,
            ..Call::default()
        };
        scrub(&mut t);
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(s6_misc::parts_axes(g, img, fr))
        });
    }
}
