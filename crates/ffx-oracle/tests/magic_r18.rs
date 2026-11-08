//! Round 18's ports (`ffx_magic::r18`, and the shared ports it extends)
//! against each original. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod magic;
mod magic_owna;
mod magic_r11;
#[path = "magic/sa.rs"]
mod sa;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

/// Each slot recorded with `n` words, answering a number of its own.
fn recs(t: &mut magic::Twin, slots: &[(u32, usize)]) {
    for &(s, n) in slots {
        rec(t, s, n, move |_, a| {
            (s << 20) ^ a.first().copied().unwrap_or(0)
        });
    }
}

/// `s4_small::walk_reset` in `magic_0577` (0x10004720, through its thunk
/// 0x100047b0): every ring shape, the owner's bit 0 of `+0x24` clear and
/// set; set, the summon model set-up with each of its flag paths and the
/// owner's unlink with and without a parent (both neighbour shapes).
#[test]
fn walk_reset_0577() {
    let Some(mut t) = twin(577) else { return };
    sa::handler_stubs(&mut t);
    recs(
        &mut t,
        &[
            (275, 1),
            (538, 2),
            (213, 2),
            (211, 2),
            (35, 5),
            (34, 2),
            (547, 1),
            (276, 2),
            (846, 0),
        ],
    );
    // slot 219: a part for part 0, none for part 1
    rec(
        &mut t,
        219,
        2,
        |_, a| if a[1] == 0 { 0x0219_0000 } else { 0 },
    );
    let f = t.at(0x47b0);
    let child = SCRATCH + 0x7000;
    let parent = SCRATCH + 0x7400;
    let other = SCRATCH + 0x7800;
    for &(c, p, cp) in sa::RINGS {
        for (bit, flags, motion, parented, first) in [
            (0u8, 0u16, 0u32, false, false),
            (1, 0, 5, true, true),
            (1, 0x10, u32::MAX, false, false),
            (1, 0x810, 7, true, false),
            (1, 0x811, 7, true, true),
        ] {
            sa::ring(&mut t, c, p, cp, 0x30);
            let owner = sa::node(0);
            t.set_u8(owner + 0x24, bit);
            t.set_u32(owner + 0x20, 0x0020_0020);
            // the owner's node: its model set-up record
            t.write(child, &[0x3a; 0x100]);
            t.set_u32(owner + 0x1c, child);
            t.set_u32(child + 0x14, 0);
            t.set_u16(child + 0x90, flags);
            t.set_u32(child + 0x64, motion);
            // the owner's parent and neighbours
            t.write(parent, &[0x4b; 0x40]);
            t.write(other, &[0x4c; 0x40]);
            t.set_u32(owner + 0x14, if parented { parent } else { 0 });
            let (prev, next) = if first {
                (parent, other)
            } else {
                (other, parent)
            };
            t.set_u32(owner + 4, prev);
            t.set_u32(owner + 8, next);
            let call = Call {
                eax: 0xaabb_ccdd,
                ecx: 0x0bad_0ecc,
                ..Call::default()
            };
            t.check_entry(f, call, &[sa::CURSOR, 0x0bad_f00e], Eax::Checked);
        }
    }
}

/// `r13_misc::scripts_step_motion` in `magic_0587` (0x100014e0): both
/// scripts done (the models' end, with and without the held node and
/// the actor flag), the second waiting, the first waiting (the record's
/// words, the motion step, the integer fade's step, the rigs' step).
#[test]
fn scripts_step_motion_0587() {
    let Some(mut t) = twin(587) else { return };
    let f = t.at(0x14e0);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    for (s, n) in [
        (941u32, 1usize),
        (772, 2),
        (779, 4),
        (326, 4),
        (229, 3),
        (284, 3),
        (285, 2),
        (286, 2),
        (235, 1),
        (330, 3),
        (328, 3),
        (329, 3),
        (413, 1),
        (215, 1),
        (19, 1),
        (21, 2),
        (332, 1),
        (6, 2),
        (547, 1),
        (295, 2),
        (541, 3),
        (38, 3),
        (568, 1),
        (566, 1),
        (546, 1),
        (271, 1),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    rec(&mut t, 314, 1, |_, a| a[0]);
    t.stub_st0(314, |_, a| f64::from(a[0] & 0xff) * 0.0625 - 7.0);
    let r = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x26), 0x00c0_0026);
    let task = SCRATCH + 0x5000;
    let end = t.img.rel32_target(f + 0x9f);
    let held = SCRATCH + 0x5400;
    for (k, (s1, s2, flag, node)) in [
        (0u8, 0u8, 0u32, true),
        (0, 0, 1, false),
        (0, 2, 0, false),
        (2, 0, 0, false),
        (2, 2, 0, false),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.write(r, &vec![0; 0x4000]);
        for (i, x) in (0u32..).zip([1.5f32, -2.0, 0.25, 1.0]) {
            t.set_u32(r + w(&t, 0x4e) + 4 * i, (x + k as f32).to_bits());
        }
        // the integer fade: 4 of 9 steps left, three channels
        let fade = r + w(&t, 0x83);
        for i in 0..3 {
            t.set_u32(fade + 0x10 + 4 * i, 0x10 + 0x20 * i);
            t.set_u32(fade + 0x20 + 4 * i, (0x80 + 7 * i).wrapping_neg());
        }
        t.set_u32(fade + 0x30, 9);
        t.set_u32(fade + 0x34, 4 + k);
        t.write(w(&t, 0x10), &[0x5c; 0x30]);
        for (ctx, st) in [(w(&t, 0x2e), s1), (w(&t, 0x3d), s2)] {
            t.write(ctx, &[0; 0x58]);
            t.set_u32(ctx, SCRATCH + 0x700);
            t.set_u8(ctx + 0x56, st);
            t.set_u16(ctx + 0x54, 0xffff);
        }
        t.set_u32(w(&t, 0x07), r);
        // the models' end: the model, the actor, the flag, the held node
        t.set_u32(t.img.word(end + 0x8), 0x0008_0001);
        t.set_u32(t.img.word(end + 0x2b), 0x0000_0005 + k);
        t.set_u32(t.img.word(end + 0x45), 0x0000_0005 + 0x10 * k);
        t.set_u32(t.img.word(end + 0x3c), flag);
        t.set_u32(t.img.word(end + 0x7d), 0x0008_0002);
        t.write(held, &[0x21; 0x20]);
        t.set_u32(held, 0);
        t.set_u32(held + 4, 0);
        t.set_u32(held + 8, SCRATCH + 0x5500);
        t.set_u32(SCRATCH + 0x550c, 0);
        t.set_u32(t.img.word(end + 0x85), if node { held } else { 0 });
        // the groups' load (the same record): two entries in group 0
        t.set_u32(r + 0x166c, 2);
        t.set_u32(r + 0x166c - 0x824, 0x0e48_0000);
        let load = t.img.rel32_target(end + 0xe3);
        assert_eq!(t.img.word(load + 0x8), w(&t, 0x07));
        t.set_u32(t.img.word(load + 0x5b), SCRATCH + 0x5600);
        t.write(task, &[0x21; 0x20]);
        t.set_u32(task, 0);
        t.set_u32(task + 4, 0);
        t.set_u32(task + 8, SCRATCH + 0x5100);
        t.set_u32(SCRATCH + 0x510c, 0);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// The plain object start at `s` (`s4_misc::start_object_plain`): its
/// globals, the part table (slot 941), the pool with `count` nodes free.
fn plain_setup(t: &mut magic::Twin, s: u32, count: u16) {
    t.set_u32(t.img.word(s + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(s + 0x5e), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    rec(t, 941, 1, |_, _| SCRATCH + 0x400);
    rec(t, 714, 5, |_, _| 0x714);
    let pa = t.img.rel32_target(s + 0x96);
    let nodes = t.img.word(pa + 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    t.set_u16(t.img.word(pa + 0x04), count);
    t.set_u32(t.img.word(s + 0xa1), 0);
}

/// `n` bytes that differ from word to word (seeded by `seed`).
fn varied(n: usize, seed: u8) -> Vec<u8> {
    (0..n)
        .map(|i| u8::try_from((i * 7 + usize::from(seed)) % 251).unwrap())
        .collect()
}

/// `r18::seventh_start_0610` (`magic_0610` 0x1000bcb0): the object pool
/// free and empty, the task list empty and not.
#[test]
fn seventh_start_0610() {
    let Some(mut t) = twin(610) else { return };
    let f = t.at(0xbcb0);
    let s = t.img.rel32_target(f + 0x3d);
    let task = t.img.rel32_target(f + 0xa4);
    let op = |t: &magic::Twin, o: u32| t.img.word(task + o);
    rec(&mut t, 282, 0, |_, _| 0x282);
    t.stub_st0(282, |m, _| f64::from(m.r32(SCRATCH + 0x20)) * 0.125 + 0.3);
    let blk = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0x13), blk);
    for (k, (count, listed)) in [(3u16, false), (0, true)].into_iter().enumerate() {
        plain_setup(&mut t, s, count);
        magic_r11::pools(&mut t, f, 174);
        t.set_u32(SCRATCH + 0x20, k as u32 + 3);
        t.write(blk, &varied(0x2000, 0x5a));
        t.write(SCRATCH + 0x600, &[0x44; 0x10]);
        t.set_u32(op(&t, 0x36), if listed { SCRATCH + 0x600 } else { 0 });
        t.set_u32(op(&t, 0x4a), SCRATCH + 0x600);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `r18::caster_place_0588` (`magic_0588` 0x10001820): the yaw above,
/// below and inside its bounds; `*G_F` 0 and 1 and the actor index below
/// and above 0x20 (slot 296 or 257). Slot 262 writes the point, the
/// float slots answer from their inputs.
#[test]
fn caster_place_0588() {
    let Some(mut t) = twin(588) else { return };
    let f = t.at(0x1820);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    let e = SCRATCH + 0x8000;
    t.set_u32(w(&t, 0x13), e);
    rec(&mut t, 262, 4, |m, a| {
        for (i, v) in [1.5f32, -2.25, 0.75].into_iter().enumerate() {
            m.w32(a[1 + i], (v + (a[0] & 0xf) as f32).to_bits());
        }
        0x0262
    });
    let yaw_in = SCRATCH + 0x714;
    rec(&mut t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(yaw_in))));
    for (s, x) in [(277u32, 0.5f64), (15, 1.25), (16, -0.75), (17, 2.5)] {
        rec(&mut t, s, 1, move |_, a| a[0] ^ (s << 16));
        t.stub_st0(s, move |_, a| f64::from(a[0] & 0xff) * 0.0625 + x);
    }
    recs(
        &mut t,
        &[
            (210, 2),
            (211, 2),
            (235, 1),
            (330, 3),
            (328, 3),
            (329, 3),
            (238, 2),
            (296, 2),
            (257, 2),
        ],
    );
    let kv = f32::from_bits(t.img.word(w(&t, 0x88)));
    for (c, (yaw, flag, index)) in [
        (-3.0f32, 0u32, 3u32),
        (3.0, 0, 0x20),
        (0.0, 1, 5),
        (0.5, 0, 0x8000_0001),
    ]
    .into_iter()
    .enumerate()
    {
        let c = c as u32;
        t.write(e, &varied(0x1000, c as u8));
        t.set_u32(w(&t, 0x2a), 0x00a0_0000 + c);
        t.set_u32(yaw_in, (kv - yaw * 7.0).to_bits());
        t.set_u32(w(&t, 0x14a), flag);
        t.set_u32(w(&t, 0x153), index);
        t.set_u32(w(&t, 0x334), 0x5555_5555);
        t.set_u32(w(&t, 0x33e), 0x6666_6666);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `r18::effect_end_0588` (`magic_0588` 0x10001c00): `*G_F` 0 and 1, the
/// actor index below and above 0x20, the halfword `*G_H` 0 and not, a
/// held node or none, the object list empty or with one object. Slot 288
/// answers empty actor lists (the party and the others run through no
/// actors).
#[test]
fn effect_end_0588() {
    let Some(mut t) = twin(588) else { return };
    let f = t.at(0x1c00);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    recs(
        &mut t,
        &[
            (384, 1),
            (25, 2),
            (24, 2),
            (217, 2),
            (255, 2),
            (211, 2),
            (213, 2),
            (293, 1),
            (296, 2),
            (257, 2),
            (238, 2),
            (371, 2),
            (772, 2),
            (779, 4),
            (713, 1),
            (941, 1),
        ],
    );
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    // slot 779 logs the lock word around it
    let lock_g = t.img.word(f + 0x19d);
    rec(&mut t, 779, 4, move |m, _| {
        m.w32(magic_owna::row(779) + 0x20, m.r32(m.r32(lock_g)));
        0x0779
    });
    rec(&mut t, 288, 2, |m, a| {
        m.w32(a[1], 0);
        0x288
    });
    let give = t.img.rel32_target(f + 0x16f);
    let freed = t.img.rel32_target(f + 0x17e);
    let list = t.img.word(freed + 0x8);
    t.set_u32(t.img.word(freed + 0x31), SCRATCH + 0x2f0);
    let kb = SCRATCH + 0x600;
    t.write(kb, &[7, 2, 9, 4]);
    t.set_u32(w(&t, 0x15), kb);
    for i in 0..4 {
        t.set_u32(w(&t, 0x22) + 4 * i, 0x0022_0000 + i);
    }
    let lock = SCRATCH + 0x640;
    t.set_u32(w(&t, 0x19d), lock);
    for (c, (flag, index, half, held, objects)) in [
        (0u32, 3u32, 0u16, true, 1u32),
        (0, 0x20, 1, false, 0),
        (1, 5, 0, false, 1),
        (0, 0xffff_fff0, 2, true, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let c = c as u32;
        t.set_u32(w(&t, 0x60), flag);
        t.set_u32(w(&t, 0x69), index);
        t.set_u16(w(&t, 0xb1), half);
        t.set_u32(w(&t, 0x57), 0x0057_0000 + c);
        t.set_u32(w(&t, 0xc1), 0x00c1_0000 + c);
        t.set_u32(w(&t, 0x11a), 0x011a_0000 + c);
        t.set_u32(lock, 0x4c00 + c);
        // the held node: its own give-back ring (one free node)
        let node = SCRATCH + 0x9000;
        let ring = node + 0x100;
        t.write(node, &varied(0x80, c as u8));
        t.set_u32(node, 0);
        t.set_u32(node + 4, 0);
        t.set_u32(node + 8, node + 0x40);
        t.set_u32(node + 0x48, node + 0x40);
        t.set_u32(node + 0x4c, 0);
        t.write(ring, &varied(0x1c, c as u8 + 1));
        t.set_u32(ring, ring);
        t.set_u32(ring + 4, ring);
        let gw = |o: u32| t.img.word(give + o);
        let (cur, head, tail, free, count) = (gw(0x48), gw(0x55), gw(0x64), gw(0x74), gw(0x8e));
        t.set_u32(free, ring);
        t.set_u16(count, 0x10 + c as u16);
        t.set_u32(cur, node);
        t.set_u32(head, node + 0x200);
        t.set_u32(tail, node);
        t.set_u32(w(&t, 0x163), if held { node } else { 0 });
        // the object list: empty or one object
        let (ln, obj) = (SCRATCH + 0xa000, SCRATCH + 0xa100);
        t.write(ln, &varied(0x20, 3));
        t.set_u32(ln, 0);
        t.set_u32(ln + 4, 0);
        t.set_u32(ln + 8, obj);
        t.write(obj, &varied(0x200, 4));
        t.set_u16(obj + 0x124, 0x8123);
        let first = if objects == 0 { 0 } else { ln };
        t.set_u32(list, first);
        t.set_u32(list + 4, first);
        t.set_u32(list + 8, 0x0bad_0008);
        t.set_u32(list + 0xc, objects + 5);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `r18::wave_tick_0588` (`magic_0588` 0x10001ec0): the states 0, 1 and
/// another, each with the count reaching 0 and not (and a count at 0
/// already); the table from slot 562, slot 862's handle from slot 772.
#[test]
fn wave_tick_0588() {
    let Some(mut t) = twin(588) else { return };
    let f = t.at(0x1ec0);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    let e = SCRATCH + 0x8000;
    t.set_u32(w(&t, 0x19), e);
    let out = SCRATCH + 0x2000;
    recs(&mut t, &[(311, 1), (772, 2)]);
    // slot 862 logs the colour it is handed and the lock word
    let lock_g = t.img.word(f + 0x18c);
    rec(&mut t, 862, 6, move |m, a| {
        let row = magic_owna::row(862) + 0x20;
        for i in 0..4 {
            m.w32(row + 4 * i, m.r32(a[3] + 4 * i));
        }
        m.w32(row + 0x10, m.r32(m.r32(lock_g)));
        0x0862
    });
    rec(&mut t, 562, 0, move |_, _| out);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    let lock = SCRATCH + 0x640;
    t.set_u32(w(&t, 0x18c), lock);
    for (c, (state, count)) in [
        (0u32, 7i32),
        (0, 1),
        (1, 9),
        (1, 1),
        (2, 3),
        (1, 0),
        (0, -4),
    ]
    .into_iter()
    .enumerate()
    {
        let c = c as u32;
        t.write(e + 0xdf0, &[0x3c; 0x20]);
        t.set_u32(e + 0xdf8, count.cast_unsigned());
        t.set_u32(e + 0xdfc, state);
        t.set_u32(w(&t, 0xb3), 0x00b3_0000 + c);
        t.set_u32(lock, 0x4c00 + c);
        t.write(out, &varied(33 * 33 * 8, c as u8));
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `r18::caster_object_0588` (`magic_0588` 0x100020d0): the object pool
/// free and empty, the task list empty and not.
#[test]
fn caster_object_0588() {
    let Some(mut t) = twin(588) else { return };
    let f = t.at(0x20d0);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    let s = t.img.rel32_target(f + 0x2e);
    let e = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x1c), e);
    rec(&mut t, 332, 1, |m, a| {
        let row = magic_owna::row(332) + 0x20;
        for i in 0..4 {
            m.w32(row + 4 * i, m.r32(a[0] + 4 * i));
        }
        0x0332
    });
    for (k, (count, listed)) in [(3u16, false), (0, true)].into_iter().enumerate() {
        plain_setup(&mut t, s, count);
        magic_r11::pools(&mut t, f, 347);
        t.write(e, &varied(0x1000, k as u8 + 9));
        t.write(SCRATCH + 0x600, &[0x44; 0x10]);
        t.set_u32(w(&t, 0xea), if listed { SCRATCH + 0x600 } else { 0 });
        t.set_u32(w(&t, 0xff), SCRATCH + 0x600);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// The caster placement's slots (`caster_place_0588`): slot 262 writes
/// the point, the float slots answer from their inputs, the others are
/// recorded.
fn caster_slots(t: &mut magic::Twin) {
    rec(t, 262, 4, |m, a| {
        for (i, v) in [1.5f32, -2.25, 0.75].into_iter().enumerate() {
            m.w32(a[1 + i], (v + (a[0] & 0xf) as f32).to_bits());
        }
        0x0262
    });
    rec(t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.125 - 1.0);
    for (s, x) in [(277u32, 0.5f64), (15, 1.25), (16, -0.75), (17, 2.5)] {
        rec(t, s, 1, move |_, a| a[0] ^ (s << 16));
        t.stub_st0(s, move |_, a| f64::from(a[0] & 0xff) * 0.0625 + x);
    }
    recs(
        t,
        &[
            (210, 2),
            (211, 2),
            (235, 1),
            (330, 3),
            (328, 3),
            (329, 3),
            (238, 2),
            (296, 2),
            (257, 2),
        ],
    );
}

/// `r18::start_up_0588` (`magic_0588` 0x10002c70) with the task list
/// empty and not, and its load step (0x10001340): busy, asking, not
/// loaded, loaded (the start-up runs on both sides).
#[test]
fn start_up_0588() {
    let Some(mut t) = twin(588) else { return };
    let f = t.at(0x2c70);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    caster_slots(&mut t);
    recs(&mut t, &[(719, 3), (711, 3), (710, 4), (271, 1)]);
    for listed in [false, true] {
        for o in [
            0x16u32, 0x20, 0x2a, 0x42, 0x61, 0x6b, 0x76, 0x7e, 0x85, 0x8f, 0x9a, 0xa2,
        ] {
            t.set_u32(w(&t, o), 0x5a5a_5a5a);
        }
        for o in [0xb6u32, 0xc0, 0xe3, 0xee] {
            t.set_u32(w(&t, o), 0x5a5a_5a5a);
        }
        magic_r11::pools(&mut t, f, 387);
        t.write(w(&t, 0xc9), &[0x5c; 0x140 * 0x10]);
        t.write(SCRATCH + 0x600, &[0x44; 0x10]);
        t.set_u32(w(&t, 0xfd), if listed { SCRATCH + 0x600 } else { 0 });
        t.set_u32(w(&t, 0x111), SCRATCH + 0x600);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r18::start_up_0588,
        );
    }
    // the load step
    let at = t.at(0x1340);
    let busy = SCRATCH + 0x5e0;
    let loaded = SCRATCH + 0x5e4;
    rec(&mut t, 225, 1, move |m, _| m.r32(busy));
    rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
    rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
    t.set_u32(t.img.word(at + 0x26), 0x0000_5e5e);
    for (b, asked, l) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 2)] {
        magic_r11::pools(&mut t, f, 387);
        t.set_u32(busy, b);
        t.set_u32(loaded, l);
        t.set_u32(t.img.word(at + 0x18), asked);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `r18::four_parts_0493` (`magic_0493` 0x100065a0): state 0 waiting
/// (below and at the start time), starting each part 0..3 (part 2's slot
/// 239, the fourth moving the state on); state 1 with the flags' bit 2
/// missing in one and set in all (the nodes leaving their parents, with a
/// parent and without); another state.
#[test]
#[allow(clippy::too_many_lines)] // nine cases over every branch's set-up
fn four_parts_0493() {
    let Some(mut t) = twin(493) else { return };
    let f = t.at(0x65a0);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    let s = SCRATCH + 0x1_0000;
    let task = SCRATCH + 0x5000;
    let tnode = SCRATCH + 0x5100;
    recs(&mut t, &[(239, 3), (233, 3), (248, 3), (325, 2), (714, 5)]);
    rec(&mut t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, |m, _| {
        let k = m.r32(SCRATCH + 0x700);
        m.w32(SCRATCH + 0x700, k + 1);
        f64::from(k % 7) * 0.375 - 0.5
    });
    // the particle node: its pointer global, the wrapper's lock, slot
    // 941's program table
    let pn = t.img.rel32_target(f + 0x516);
    t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x7f10);
    let wrapper = t.img.rel32_target(pn + 0x88);
    t.set_u32(t.img.word(wrapper + 0x04), SCRATCH + 0x7f00);
    let table = SCRATCH + 0x7c00;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x9000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    let owner = SCRATCH + 0x7a00;
    t.write(owner, &varied(0x100, 5));
    t.set_u32(w(&t, 0x4e7), owner);
    let starts = w(&t, 0x230);
    for i in 0..4u32 {
        t.set_u32(starts + 4 * i, (1.0 + i as f32).to_bits());
    }
    let lists = [w(&t, 0x26f), w(&t, 0x3bc), w(&t, 0x465)];
    assert!(lists.iter().all(|&l| l == lists[0]));
    let mk = SCRATCH + 0x7800;
    // (state, part, clock, flags all, parented)
    for (c, (state, part, clock, all, parented)) in [
        (0u32, 0u32, 0.25f32, false, false),
        (0, 0, 1.0, false, false),
        (0, 1, 2.5, false, false),
        (0, 2, 3.0, false, false),
        (0, 3, 9.0, false, false),
        (1, 0, 0.0, false, false),
        (1, 0, 0.0, true, true),
        (1, 0, 0.0, true, false),
        (7, 0, 0.0, false, false),
    ]
    .into_iter()
    .enumerate()
    {
        let c = c as u32;
        t.write(s, &varied(0xb10, c as u8));
        t.set_u32(s + 0xadc, clock.to_bits());
        t.set_u32(s + 0xae0, (1.5f32).to_bits());
        t.set_u32(s + 0xae4, part);
        t.set_u32(s + 0xae8, state);
        t.set_u32(s + 0xaec, mk);
        t.set_u32(mk + 0x50, 0x0050_0000 + c);
        for (k, o) in (0u32..).zip([0x108u32, 0x208, 0x308, 0x408]) {
            let v = if all || k != 2 { 0x0f } else { 0x0b };
            t.set_u32(s + o, v);
        }
        // every node a parent of its own, or none
        let mut nodes: Vec<u32> = Vec::new();
        for p in 0..4u32 {
            nodes.extend([0x420 + 0x2c * p, 0x4d0 + 0x2c * p, 0x5ac + 0x14c * p]);
        }
        nodes.extend([0x580, 0xaf0]);
        for (k, &o) in (0u32..).zip(&nodes) {
            let n = s + o;
            let par = SCRATCH + 0x9000 + 0x40 * k;
            t.write(par, &[0x31; 0x20]);
            t.set_u32(
                n + 4,
                if k % 2 == 0 {
                    par
                } else {
                    SCRATCH + 0x8800 + 0x20 * k
                },
            );
            t.set_u32(
                n + 8,
                if k % 3 == 0 {
                    par
                } else {
                    SCRATCH + 0x8c00 + 0x20 * k
                },
            );
            t.set_u32(n + 0x14, if parented { par } else { 0 });
        }
        // the task's ring node
        t.write(tnode, &[0x2a; 0x20]);
        t.set_u32(tnode + 4, SCRATCH + 0x5200);
        t.set_u32(tnode + 8, SCRATCH + 0x5300);
        t.set_u32(task, tnode);
        t.set_u32(task + 4, s);
        // the parts' list (both appends and the insert name it): empty
        // or with a child
        let l = lists[0];
        t.write(l, &[0; 0x20]);
        t.write(SCRATCH + 0x7600, &[0x26; 0x20]);
        let last = if c.is_multiple_of(2) {
            l
        } else {
            SCRATCH + 0x7600
        };
        t.set_u32(l + 0xc, last);
        t.set_u32(l + 0x10, last);
        // the particle nodes' list: empty
        let pl = w(&t, 0x4f7);
        t.write(pl, &[0; 0x20]);
        t.set_u32(pl + 0xc, pl);
        t.set_u32(pl + 0x10, pl);
        t.set_u32(SCRATCH + 0x700, c);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `r18::start_up_0564` (`magic_0564` 0x10001340) and its load step
/// (0x10001210): both sides of slot 293; the three records' kinds 1, 0
/// and -1 in turn (and an unknown kind), slot 640 busy for some actors
/// (slot 541 for a near actor, slot 38 for a far one or with `*G_F` set,
/// the caster's own model for kind -1), `*G_F` 0 and 1; the free ring
/// with three nodes and one, the task list empty and not.
#[test]
fn start_up_0564() {
    let Some(mut t) = twin(564) else { return };
    let f = t.at(0x1340);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(&mut t, f, 1614);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0x1a2);
    magic_r11::object_start_auto(&mut t, start);
    let side = SCRATCH + 0x720;
    let busy = SCRATCH + 0x724;
    rec(&mut t, 293, 1, move |m, _| m.r32(side));
    rec(&mut t, 640, 1, move |m, a| {
        u32::from(m.r32(busy) >> (a[0] & 0x1f) & 1 != 0)
    });
    for (s, n) in [
        (215u32, 1usize),
        (413, 1),
        (541, 3),
        (38, 3),
        (220, 3),
        (711, 3),
        (710, 4),
    ] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    for (s, x) in [(12u32, 0.75f64), (333, -1.5)] {
        rec(&mut t, s, 1, move |_, a| a[0] ^ (s << 16));
        t.stub_st0(s, move |_, a| f64::from(a[0] & 0xff) * 0.5 + x);
    }
    let l = SCRATCH + 0x600;
    t.write(l, &[4, 6, 0x21, 9, 1, 2, 3, 5]);
    for i in 0..0x40 {
        t.set_u32(w(&t, 0x1bc) + 4 * i, 0x0b00_0000 + i);
    }
    let kinds = w(&t, 0xc4);
    let free_head = w(&t, 0x5a2);
    let free_count = w(&t, 0x590);
    let (first, last) = (w(&t, 0x609), w(&t, 0x61b));
    let other = SCRATCH + 0x680;
    // (side, kinds, busy actors, *G_F, free nodes, listed)
    let cases: [(u32, [u32; 3], u32, u32, u16, bool); 6] = [
        (0, [1, 0, u32::MAX], 0, 0, 3, false),
        (1, [u32::MAX, 1, 0], 0, 1, 1, true),
        (0, [2, 2, 1], 1 << 6 | 1 << 9, 0, 3, true),
        (
            1,
            [0, 1, u32::MAX],
            1 << 4 | 1 << 6 | 1 << 0x10,
            0,
            2,
            false,
        ),
        (0, [u32::MAX, 2, 0], 1 << 0x10 | 1 << 4, 1, 3, false),
        (1, [2, u32::MAX, 3], 1 << 0x13, 0, 3, true),
    ];
    for (c, (sd, ks, b, flag, free, listed)) in (0u32..).zip(cases) {
        t.set_u32(side, sd);
        t.set_u32(busy, b);
        for (j, &k) in (0u32..).zip(&ks) {
            t.set_u32(kinds + 4 * j, k);
        }
        t.set_u32(w(&t, 0xc), 0x10 + 3 * c);
        t.set_u32(w(&t, 0xd5), l);
        t.set_u32(w(&t, 0x123), flag);
        t.set_u32(w(&t, 0x367), 0x0013_0000 + c);
        t.set_u32(w(&t, 0x513), 0x0051_3000 + c);
        t.write(w(&t, 0xbc) - 8, &[0x5d; 0x60]);
        // the 1.0 words the placed objects set
        for o in [0x1ebu32, 0x2c8, 0x3a1, 0x466] {
            t.set_u32(w(&t, o), 0x5e5e_5e5e);
        }
        magic_r11::ring(&mut t, free_count, free_head, free, SCRATCH + 0x1400);
        t.write(other, &[0x44; 0x20]);
        t.set_u32(first, if listed { other } else { 0 });
        t.set_u32(last, other);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r18::start_up_0564,
        );
    }
    // the load step
    let at = t.at(0x1210);
    let busy_l = SCRATCH + 0x5e0;
    let loaded = SCRATCH + 0x5e4;
    rec(&mut t, 225, 1, move |m, _| m.r32(busy_l));
    rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
    rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
    t.set_u32(t.img.word(at + 0x26), 0x0000_5e5e);
    for (bz, asked, ld) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 2)] {
        magic_r11::ring(&mut t, free_count, free_head, 3, SCRATCH + 0x1400);
        t.set_u32(busy_l, bz);
        t.set_u32(loaded, ld);
        t.set_u32(t.img.word(at + 0x18), asked);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `r18::start_up_0562` (`magic_0562` 0x100013e0) and its load step
/// (0x100012b0): the yaw above, below and inside its bounds, the task
/// list empty and not; the caster's point from slot 262.
#[test]
fn start_up_0562() {
    let Some(mut t) = twin(562) else { return };
    let f = t.at(0x13e0);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0xf2);
    magic_r11::object_start_auto(&mut t, start);
    let yaw_in = SCRATCH + 0x714;
    rec(&mut t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(yaw_in))));
    recs(
        &mut t,
        &[
            (357, 3),
            (356, 3),
            (362, 4),
            (855, 2),
            (6, 2),
            (548, 1),
            (271, 1),
            (711, 3),
            (710, 4),
            (213, 2),
            (215, 1),
            (413, 1),
        ],
    );
    let kv = f32::from_bits(t.img.word(w(&t, 0x9b)));
    let (first, last) = (w(&t, 0x4be), w(&t, 0x4d5));
    let other = SCRATCH + 0x680;
    for (c, (yaw, listed)) in [(-3.0f32, false), (3.0, true), (0.0, false)]
        .into_iter()
        .enumerate()
    {
        let c = c as u32;
        magic_r11::pools(&mut t, f, 1518);
        t.set_u32(w(&t, 0x6e), 0x00a0_0000 + c);
        t.set_u32(yaw_in, (kv - yaw * 7.0).to_bits());
        t.write(other, &[0x44; 0x20]);
        t.set_u32(first, if listed { other } else { 0 });
        t.set_u32(last, other);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r18::start_up_0562,
        );
    }
    // the load step
    let at = t.at(0x12b0);
    let busy_l = SCRATCH + 0x5e0;
    let loaded = SCRATCH + 0x5e4;
    rec(&mut t, 225, 1, move |m, _| m.r32(busy_l));
    rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
    rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
    t.set_u32(t.img.word(at + 0x26), 0x0000_5e5e);
    for (bz, asked, ld) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 2)] {
        magic_r11::pools(&mut t, f, 1518);
        t.set_u32(busy_l, bz);
        t.set_u32(loaded, ld);
        t.set_u32(t.img.word(at + 0x18), asked);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `r18::model_start_0603` (`magic_0603` 0x1000cb10): the node list empty
/// (its last the list itself) and not; the second node's object byte.
#[test]
fn model_start_0603() {
    let Some(mut t) = twin(603) else { return };
    let f = t.at(0xcb10);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    recs(&mut t, &[(413, 1), (215, 1), (14, 2)]);
    rec(&mut t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.25);
    let prev = SCRATCH + 0x700;
    let other = SCRATCH + 0x740;
    let list = w(&t, 0x93);
    for (c, (empty, flags)) in [(true, 0u8), (false, 1), (true, 7)].into_iter().enumerate() {
        let c = c as u32;
        t.write(prev, &[0x31; 0x20]);
        t.set_u32(w(&t, 0x4), prev);
        t.write(other, &[0x32; 0x20]);
        t.set_u32(w(&t, 0x3d), if empty { list } else { other });
        t.set_u32(w(&t, 0x43), 0x0000_0f0f + c);
        t.set_u16(w(&t, 0xaf), 3 + c as u16);
        t.set_u8(w(&t, 0x169), flags);
        // the set-ups' parents and objects: the record's words
        for o in [0x148u32, 0x17a] {
            let p = w(&t, o);
            t.write(p, &[0; 0x20]);
            t.set_u32(p + 0x10, p);
            t.set_u32(p + 0xc, p);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `r18::lights_start_0610` (`magic_0610` 0x1000be90): the object pool
/// free and empty, the task list empty and not.
#[test]
fn lights_start_0610() {
    let Some(mut t) = twin(610) else { return };
    let f = t.at(0xbe90);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    let s = t.img.rel32_target(f + 0x43);
    rec(&mut t, 282, 0, |_, _| 0x282);
    t.stub_st0(282, |m, _| f64::from(m.r32(SCRATCH + 0x20)) * 0.125 + 0.3);
    let blk = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0xe), blk);
    for (k, (count, listed)) in [(3u16, false), (0, true), (2, true)]
        .into_iter()
        .enumerate()
    {
        plain_setup(&mut t, s, count);
        magic_r11::pools(&mut t, f, 407);
        t.set_u32(SCRATCH + 0x20, k as u32 + 3);
        t.write(blk, &varied(0x1600, 0x5a + k as u8));
        t.write(SCRATCH + 0x600, &[0x44; 0x10]);
        t.set_u32(w(&t, 0xbf), if listed { SCRATCH + 0x600 } else { 0 });
        t.set_u32(w(&t, 0xd5), SCRATCH + 0x600);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `r18::caster_place_0588` in `magic_0610` (0x1000a800): the yaw above,
/// below and inside its bounds; the others shown (slot 293 says the DLL's
/// actor has a group or not; slot 288's list of two).
#[test]
fn caster_place_0610() {
    let Some(mut t) = twin(610) else { return };
    let f = t.at(0xa800);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    let e = SCRATCH + 0x8000;
    t.set_u32(w(&t, 0x15), e);
    caster_slots(&mut t);
    let yaw_in = SCRATCH + 0x714;
    t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(yaw_in))));
    let group = SCRATCH + 0x718;
    rec(&mut t, 293, 1, move |m, _| m.r32(group));
    let list = SCRATCH + 0x760;
    t.write(list, &[5, 0, 9, 0, 0x22, 0, 0, 0]);
    rec(&mut t, 288, 2, move |m, a| {
        m.w32(a[1], 3);
        list
    });
    let kv = f32::from_bits(t.img.word(w(&t, 0x8a)));
    for (c, (yaw, grp)) in [(-3.0f32, 0u32), (3.0, 1), (0.0, 0)]
        .into_iter()
        .enumerate()
    {
        let c = c as u32;
        t.write(e, &varied(0x1c00, c as u8));
        t.set_u32(w(&t, 0x2c), 0x00a0_0000 + c);
        t.set_u32(yaw_in, (kv - yaw * 7.0).to_bits());
        t.set_u32(group, grp);
        t.set_u32(w(&t, 0x2f5), 0x5555_5555);
        t.set_u32(w(&t, 0x2ff), 0x6666_6666);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `r18::turned_records_0612` (`magic_0612` 0x10008370): the angle above,
/// below and inside its bounds; the task list empty and not.
#[test]
fn turned_records_0612() {
    let Some(mut t) = twin(612) else { return };
    let f = t.at(0x8370);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    let e = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x4), e);
    caster_slots(&mut t);
    let kv = f32::from_bits(t.img.word(w(&t, 0x14)));
    let (first, last) = (w(&t, 0x156), w(&t, 0x16b));
    let other = SCRATCH + 0x680;
    for (c, (ang, listed)) in [(-3.0f32, false), (3.0, true), (0.0, true)]
        .into_iter()
        .enumerate()
    {
        let c = c as u32;
        magic_r11::pools(&mut t, f, 401);
        t.write(e + 0x2a00, &varied(0x1200, c as u8));
        t.set_u32(e + 0x3ba4, (kv + ang * 7.0).to_bits());
        t.write(other, &[0x44; 0x20]);
        t.set_u32(first, if listed { other } else { 0 });
        t.set_u32(last, other);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `r18::load_steps_0597` (`magic_0597` 0x1000ad70): each step waiting
/// and passing on (the slots answering by a counter), step 3 not ready
/// and ready (to the end: the ring and the two walks), a step past the
/// table and a negative one.
#[test]
fn load_steps_0597() {
    let Some(mut t) = twin(597) else { return };
    let f = t.at(0xad70);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    let answer = SCRATCH + 0x720;
    for s in [225u32, 313, 549, 227] {
        rec(&mut t, s, 1, move |m, _| {
            let k = m.r32(answer);
            m.w32(answer, k >> 4);
            k & 0xf
        });
    }
    recs(&mut t, &[(226, 2), (673, 0)]);
    let ring = t.img.word(f + 0xfc);
    let p = SCRATCH + 0x740;
    t.set_u32(w(&t, 0xdc), p);
    // (step, the slots' answers four bits each, last first)
    for (c, (step, ans)) in [
        (0u32, 0x1u32),
        (0, 0x1_000),
        (1, 0x10),
        (2, 0x1),
        (3, 0x0),
        (3, 0x1),
        (2, 0x10),
        (5, 0),
        (u32::MAX, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let c = c as u32;
        t.set_u32(w(&t, 0x4), step);
        t.set_u32(answer, ans);
        t.set_u8(w(&t, 0xe2), 3 + c as u8);
        // an empty ring and empty node groups
        t.write(ring, &[0; 0x14]);
        t.set_u32(ring + 4, ring + 4);
        t.set_u32(ring + 8, ring + 4);
        t.set_u32(ring + 0xc, ring + 4);
        for o in [0x125u32, 0x13f] {
            let grp = w(&t, o);
            t.write(grp, &[0; 0x20]);
            t.set_u32(grp + 0x10, grp);
            t.set_u32(grp + 0xc, grp);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `r18::motion_task_0512` (`magic_0512` 0x10007420): every step (1 and
/// 10002 not ready and ready, 10001 still waiting and done) and steps
/// outside the table.
#[test]
fn motion_task_0512() {
    let Some(mut t) = twin(512) else { return };
    let f = t.at(0x7420);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    let ready = SCRATCH + 0x720;
    rec(&mut t, 411, 1, move |m, _| m.r32(ready));
    recs(&mut t, &[(228, 2), (213, 2), (262, 4), (719, 3)]);
    let rec_f = t.img.rel32_target(f + 0x144);
    let lists = [w(&t, 0x107), t.img.word(rec_f + 0x18)];
    let b = SCRATCH + 0x8000;
    let o = SCRATCH + 0x8400;
    let task = SCRATCH + 0x5000;
    let tnode = SCRATCH + 0x5100;
    for (c, (step, wait, rd)) in [
        (0u32, 3u32, 0u32),
        (1, 3, 0),
        (1, 0, 1),
        (10000, 9, 0),
        (10001, 2, 0),
        (10001, 0, 0),
        (10002, 1, 0),
        (10002, 1, 1),
        (5, 1, 0),
        (10007, 1, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let c = c as u32;
        t.write(b, &varied(0x100, c as u8));
        t.write(o, &varied(0x60, c as u8 + 1));
        t.set_u32(b + 0x18, step);
        t.set_u32(b + 0x1c, wait);
        t.set_u32(b + 0x20, o);
        t.set_u32(ready, rd);
        for &l in &lists {
            t.write(l, &[0; 0x20]);
            t.set_u32(l + 0x10, l);
            t.set_u32(l + 0xc, l);
        }
        t.write(tnode, &[0x2a; 0x20]);
        t.set_u32(tnode + 4, SCRATCH + 0x5200);
        t.set_u32(tnode + 8, SCRATCH + 0x5300);
        t.set_u32(task, tnode);
        t.set_u32(task + 4, b);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `r18::particle_task_0512` (`magic_0512` 0x10007660): steps 0..3 (1
/// and 2 waiting and not; 3 held by the flag and passing to the end), the
/// end with `P` marked and not and its nodes parented and not, steps
/// outside the table.
#[test]
fn particle_task_0512() {
    let Some(mut t) = twin(512) else { return };
    let f = t.at(0x7660);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    recs(&mut t, &[(714, 5), (325, 2), (713, 1)]);
    let pn = t.img.rel32_target(f + 0xb0);
    t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x7f10);
    let wrapper = t.img.rel32_target(pn + 0x88);
    t.set_u32(t.img.word(wrapper + 0x04), SCRATCH + 0x7f00);
    let table = SCRATCH + 0x7c00;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x9000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    let owner = SCRATCH + 0x7a00;
    t.write(owner, &varied(0x100, 5));
    t.set_u32(w(&t, 0x87), owner);
    let fu = t.img.rel32_target(f + 0x130);
    t.set_u32(t.img.word(fu + 0x01), SCRATCH + 0x7e00);
    t.set_u32(w(&t, 0x13e), SCRATCH + 0x7e40);
    let b = SCRATCH + 0x8000;
    let task = SCRATCH + 0x5000;
    let tnode = SCRATCH + 0x5100;
    let list = w(&t, 0x9a);
    for (c, (step, wait, mark, parented)) in [
        (0u32, 0u32, 0u8, false),
        (1, 2, 0, false),
        (1, 0, 0, false),
        (2, 1, 0, false),
        (2, 0, 0, false),
        (3, 0, 1, false),
        (3, 0, 0, true),
        (10000, 0, 4, false),
        (10000, 0, 0, true),
        (7, 0, 0, false),
        (20000, 0, 0, false),
    ]
    .into_iter()
    .enumerate()
    {
        let c = c as u32;
        t.write(b, &varied(0x400, c as u8));
        t.set_u32(b + 0x18, step);
        t.set_u32(b + 0x1c, wait);
        t.set_u32(b + 0x20, 0x10 + c);
        t.set_u8(b + 0x2e8, mark & 1);
        let p = b + 0x200;
        t.set_u8(p + 0xe8, mark);
        // the nodes: a parent of their own or none
        for (k, o) in (0u32..).zip([0x24u32, 0x19c, 0x170, 0x1c8, 0x200]) {
            let n = b + o;
            let par = SCRATCH + 0x9000 + 0x40 * k;
            t.write(par, &[0x31; 0x20]);
            t.set_u32(n + 4, par);
            t.set_u32(n + 8, par);
            t.set_u32(n + 0x14, if parented { par } else { 0 });
        }
        t.write(list, &[0; 0x20]);
        t.set_u32(list + 0x10, list);
        t.set_u32(list + 0xc, list);
        t.write(tnode, &[0x2a; 0x20]);
        t.set_u32(tnode + 4, SCRATCH + 0x5200);
        t.set_u32(tnode + 8, SCRATCH + 0x5300);
        t.set_u32(task, tnode);
        t.set_u32(task + 4, b);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}
