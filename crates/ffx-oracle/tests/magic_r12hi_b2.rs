//! Round 12 (own rows from `magic_0450` up), batch 2: own functions of
//! `magic_0476` and later DLLs against their Rust ports. Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

mod magic;
mod magic_hi;
mod magic_l3;
#[path = "magic/ownb.rs"]
mod ownb;

use magic::{Call, Eax, SCRATCH, Twin};
use ownb::rec;

/// The anim object start (`s4_misc::start_object_anim` at `s`) set up:
/// the item and program words, `count` pool nodes free, the object
/// list's tail `tail`.
fn anim_setup(t: &mut Twin, s: u32, count: u16, tail: u32) {
    t.set_u32(t.img.word(s + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(s + 0x5e), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    let pa = t.img.rel32_target(s + 0x99);
    let nodes = t.img.word(pa + 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    t.set_u16(t.img.word(pa + 0x04), count);
    t.write(SCRATCH + 0x600, &[0x44; 0x1c]);
    t.set_u32(t.img.word(s + 0xa4), tail);
}

/// `m0476::start` and its cue start (0x100013e0, 0x100012b0): every
/// angle branch, the task list empty and not, the object pool free and
/// empty.
#[test]
fn start_0476() {
    let Some(mut t) = magic_l3::twin(476) else {
        return;
    };
    let f = t.at(0x13e0);
    let la = t.img.rel32_target(f + 0x43);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    for (s, n) in [
        (711u32, 3usize),
        (710, 4),
        (413, 1),
        (215, 1),
        (714, 5),
        (349, 2),
        (271, 1),
        (525, 1),
        (220, 3),
    ] {
        rec(&mut t, s, n, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    rec(&mut t, 345, 1, |m, a| {
        m.w32(SCRATCH + 0x8f0, m.r32(SCRATCH + 0x8f0) ^ a[0]);
        0x0345_0000 ^ a[0]
    });
    rec(&mut t, 941, 1, |_, _| SCRATCH + 0x400);
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3 {
            m.w32(
                a[1 + i],
                (1.5f32 + i as f32 * 0.75 + (a[0] & 3) as f32).to_bits(),
            );
        }
        0x262
    });
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, |m, _| f64::from(f32::from_bits(m.r32(SCRATCH + 0x20))));
    let part_init = t.img.rel32_target(f + 0x66);
    t.set_u32(t.img.word(part_init + 0x01), SCRATCH + 0x800);
    let take = t.img.rel32_target(f + 0x344);
    let objs = [0xffu32, 0x163, 0x1c0, 0x225];
    for (k, angle) in [-5.0f32, 0.5, 3.0, 9.0].into_iter().enumerate() {
        let busy = k % 2 == 1;
        t.set_u32(SCRATCH + 0x20, angle.to_bits());
        t.set_u32(t.img.word(la + 0x34), 0);
        for &o in &objs {
            let s = t.img.rel32_target(f + o);
            anim_setup(
                &mut t,
                s,
                if k == 2 { 0 } else { 3 },
                if busy { SCRATCH + 0x600 } else { 0 },
            );
        }
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 6);
        let last = SCRATCH + 0x3400;
        t.write(last, &[0x34; 0x20]);
        t.set_u32(t.img.word(f + 0x34a), if busy { last } else { 0 });
        t.set_u32(t.img.word(f + 0x35d), last);
        for op in [0x154u32, 0x15a, 0x1bb, 0x1fc, 0x220, 0x271, 0x294] {
            t.set_u32(t.img.word(f + op), 0x0a0a_0000 + op);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    // the cue start: the waits only (the start ran above)
    let cue = t.at(0x12b0);
    rec(&mut t, 225, 1, |m, _| m.r32(SCRATCH + 0x10));
    rec(&mut t, 226, 2, |_, a| 0x226 ^ a[0]);
    rec(&mut t, 227, 1, |_, _| 0);
    t.set_u32(t.img.word(cue + 0x26), 0x00a5_0026);
    for (go, once) in [(1u32, 0u32), (0, 0), (0, 1)] {
        t.set_u32(SCRATCH + 0x10, go);
        t.set_u32(t.img.word(cue + 0x18), once);
        t.check_entry(cue, Call::default(), &[], Eax::Checked);
    }
}

/// `m0589::objects_by_record` (0x10003c50, 0x10003e00): the task list
/// empty and not, the object pool free and empty.
#[test]
fn objects_by_record_0589() {
    let Some(mut t) = magic_l3::twin(589) else {
        return;
    };
    rec(&mut t, 941, 1, |_, _| SCRATCH + 0x400);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[0]);
    let r = SCRATCH + 0x1_0000;
    for rva in [0x3c50u32, 0x3e00] {
        let f = t.at(rva);
        t.set_u32(t.img.word(f + 0x0c), r);
        let s = t.img.rel32_target(f + 0x20);
        let take = t.img.rel32_target(f + 0xd9);
        for (k, busy) in [false, true, false].into_iter().enumerate() {
            anim_setup(
                &mut t,
                s,
                if k == 2 { 0 } else { 3 },
                if busy { SCRATCH + 0x600 } else { 0 },
            );
            t.write(r, &vec![0x31 + k as u8; 0x2400]);
            magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 6);
            let last = SCRATCH + 0x3400;
            t.write(last, &[0x34; 0x20]);
            t.set_u32(t.img.word(f + 0xdf), if busy { last } else { 0 });
            t.set_u32(t.img.word(f + 0xf2), last);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `m0488::spark_take` (0x10001000): no sprite free, and sprites taken
/// with several draws.
#[test]
fn spark_take_0488() {
    let Some(mut t) = magic_l3::twin(488) else {
        return;
    };
    let f = t.at(0x1000);
    for s in [942u32, 941] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    rec(&mut t, 324, 0, |m, _| {
        let k = m.r32(SCRATCH + 0x20);
        m.w32(SCRATCH + 0x20, k + 1);
        k.wrapping_mul(0x9e37_79b9) >> 7
    });
    rec(&mut t, 403, 1, |_, a| 0x0403_0000 ^ a[0]);
    t.stub_st0(403, |_, a| f64::from(f32::from_bits(a[0])) * 1.5 - 0.25);
    t.set_u32(t.img.word(f + 0x06), 0x0e5e_0001);
    let pool = SCRATCH + 0x100;
    let sprites = SCRATCH + 0x1000;
    let point = SCRATCH + 0x200;
    for (k, (n, busy)) in [(0u32, 0usize), (3, 0), (3, 2), (2, 2)]
        .into_iter()
        .enumerate()
    {
        t.write(sprites, &[0x5a; 0x94 * 3]);
        for i in 0..3u32 {
            t.set_u32(sprites + 0x94 * i, u32::from((i as usize) < busy));
        }
        t.set_u32(pool, n);
        t.set_u32(pool + 4, sprites);
        for i in 0..3u32 {
            t.set_u32(point + 4 * i, (1.5f32 + i as f32 - k as f32).to_bits());
        }
        t.set_u32(SCRATCH + 0x20, 7 * k as u32);
        t.check_entry(f, Call::this(pool), &[point], Eax::Checked);
    }
}

/// `r12h_fam::object_swap` (`magic_0511` 0x10003000, `magic_0589`
/// 0x10002be0): the first object listed (as the list's only node, first
/// of two) and not, the object pool free and empty.
#[test]
fn object_swaps() {
    for (n, rva) in [(511u16, 0x3000u32), (589, 0x2be0)] {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(rva);
        rec(&mut t, 941, 1, |_, _| SCRATCH + 0x400);
        rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[0]);
        rec(&mut t, 713, 1, |m, a| {
            m.w32(SCRATCH + 0x8f0, m.r32(SCRATCH + 0x8e0));
            0x0713_0000 ^ a[0]
        });
        rec(&mut t, 332, 1, |m, a| {
            for i in 0..4 {
                m.w32(SCRATCH + 0x8c0 + 4 * i, m.r32(a[0] + 4 * i));
            }
            0x332
        });
        t.set_u32(t.img.word(f + 0x2f), SCRATCH + 0x8e0);
        let r = SCRATCH + 0x1_0000;
        t.set_u32(t.img.word(f + 0x14), r);
        let o1 = r + t.img.word(f + 0x1f);
        let list = t.img.word(f + 0x19);
        let s = t.img.rel32_target(f + 0x6e);
        let take = t.img.rel32_target(t.img.rel32_target(f + 0x130) + 0x75);
        for op in [0xd1u32, 0xde, 0xf1] {
            t.set_u32(t.img.word(f + op), (0.25f32 * op as f32).to_bits());
        }
        for (k, (listed, two)) in [(true, false), (false, false), (true, true)]
            .into_iter()
            .enumerate()
        {
            anim_setup(&mut t, s, if k == 1 { 0 } else { 3 }, 0);
            t.write(r, &vec![0x31 + k as u8; 0x2400]);
            // the halfword whose bit 15 the swap clears
            t.set_u16(o1 + 0x124, 0x8123);
            magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
            let (n1, n2) = (SCRATCH + 0x5000, SCRATCH + 0x5040);
            t.write(n1, &[0x51; 0x20]);
            t.write(n2, &[0x52; 0x20]);
            t.set_u32(n1, 0);
            t.set_u32(n1 + 4, if two { n2 } else { 0 });
            t.set_u32(n1 + 8, if listed { o1 } else { SCRATCH + 0x7777 });
            t.set_u32(n2, n1);
            t.set_u32(n2 + 4, 0);
            t.set_u32(n2 + 8, SCRATCH + 0x6666);
            t.set_u32(list, n1);
            t.set_u32(list + 4, if two { n2 } else { n1 });
            t.set_u32(list + 8, n1);
            t.set_u32(list + 0xc, 1 + u32::from(two));
            t.set_u32(SCRATCH + 0x8e0, 0x5a5a_5a5a);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `m0599::start` (0x10005ae0): the object block's slots answered, the
/// words the start sets holding old values first.
#[test]
fn start_0599() {
    let Some(mut t) = magic_l3::twin(599) else {
        return;
    };
    let f = t.at(0x5ae0);
    t.stub_st0(790, |_, _| 1.5);
    let block = SCRATCH + 0x1000;
    t.write(block, &[0x42; 0x40]);
    rec(&mut t, 168, 1, move |_, _| block);
    rec(&mut t, 218, 2, |_, _| SCRATCH + 0x1800);
    for (s, n) in [
        (215u32, 1usize),
        (413, 1),
        (556, 1),
        (555, 2),
        (331, 3),
        (720, 2),
        (718, 3),
        (222, 7),
        (221, 11),
        (551, 6),
    ] {
        rec(&mut t, s, n, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(&mut t, 790, 0, |_, _| 0x790);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    for o in [0x57u32, 0x61, 0x6b, 0x14a, 0x77] {
        t.set_u32(w(&t, o), 0x77);
    }
    t.set_u16(w(&t, 0xab), 0x7777);
    t.set_u16(w(&t, 0xa3), 0x7777);
    t.write(w(&t, 0x81), &vec![0x5a; 12 * 0x3ff]);
    t.check_entry(f, Call::default(), &[0x00ab_cdef], Eax::Checked);
}

/// `m0599::sprites_setup` (0x10006e70): the node already held, and taken
/// from a ring of 3 and 1 (the list empty and not).
#[test]
fn sprites_setup_0599() {
    let Some(mut t) = magic_l3::twin(599) else {
        return;
    };
    let f = t.at(0x6e70);
    rec(&mut t, 371, 2, |_, a| 0x0371_0000 ^ a[0] ^ a[1]);
    let b = SCRATCH + 0x1_0000;
    let tex = SCRATCH + 0x700;
    t.set_u32(tex + 0x20, 0x0e0e_0020);
    for op in [0x31u32, 0x102] {
        t.set_u32(t.img.word(f + op), b);
    }
    t.set_u32(t.img.word(f + 0x60), tex);
    let nodes = |i: u32| SCRATCH + 0x5000 + 0x20 * i;
    for (k, (held, n, busy)) in [
        (1u32, 3u16, false),
        (0, 3, false),
        (0, 1, true),
        (0, 0, true),
    ]
    .into_iter()
    .enumerate()
    {
        t.write(b, &vec![0x5b + k as u8; 0x3400]);
        t.set_u32(b + 0x3148, held * (SCRATCH + 0x7000));
        for i in 0..3u32 {
            t.write(nodes(i), &[0x66; 0x20]);
            t.set_u32(nodes(i), nodes((i + 2) % 3));
            t.set_u32(nodes(i) + 4, nodes((i + 1) % 3));
        }
        t.set_u16(t.img.word(f + 0x120), n);
        t.set_u32(t.img.word(f + 0x12f), nodes(0));
        let last = SCRATCH + 0x5400;
        t.write(last, &[0x54; 0x20]);
        t.set_u32(t.img.word(f + 0x196), if busy { last } else { 0 });
        t.set_u32(t.img.word(f + 0x1a5), last);
        if n == 0 {
            // an empty ring: the original writes through node 0 (`+4`),
            // which the harness leaves unmapped; not run
            continue;
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}
