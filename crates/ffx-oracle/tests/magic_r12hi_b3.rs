//! Round 12 (own rows from `magic_0450` up), batch 3: own functions of
//! `magic_0645` and later DLLs against their Rust ports. Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_l3;
mod magic_owna;
#[path = "magic/ownb.rs"]
mod ownb;

use magic::{Call, Eax, SCRATCH, Twin};
use ownb::rec;

/// `m0645::caster_fade` and `caster_fade_ramp` (0x10006f50, 0x100072c0):
/// every step, each wait running and
/// done (the character's fade with its flags on and off).
#[test]
fn caster_fade_0645() {
    for rva in [0x6f50u32, 0x72c0] {
        caster_fade_in(rva);
    }
}

fn caster_fade_in(rva: u32) {
    let Some(mut t) = magic_l3::twin(645) else {
        return;
    };
    let f = t.at(rva);
    rec(&mut t, 307, 0, |m, _| {
        let k = m.r32(SCRATCH + 0x20);
        m.w32(SCRATCH + 0x20, k + 1);
        0x0307_0000 + k
    });
    t.stub_st0(307, |m, _| f64::from(m.r32(SCRATCH + 0x20) % 5) * 0.25);
    for (s, n) in [(257u32, 2usize), (38, 3), (296, 2), (541, 3)] {
        rec(&mut t, s, n, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    let task = SCRATCH + 0x300;
    let s = SCRATCH + 0x1_0000;
    let node = SCRATCH + 0x380;
    let chr = SCRATCH + 0x900;
    for (k, (step, wait, flags)) in [
        (0u32, 0i32, 0u16),
        (1, 3, 0),
        (1, 0, 0),
        (0x14, 5, 0),
        (0x14, -1, 1),
        (0x15, 2, 0),
        (0x15, 0, 1),
        (0x15, 0, 0),
        (0x16, 4, 0),
        (0x16, 0, 0),
        (0x2710, 0, 0),
        (7, 0, 0),
        (0x30, 0, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(SCRATCH + 0x20, k);
        t.write(s, &vec![0x3c + k as u8; 0x1100]);
        t.set_u32(s + 0x18, step);
        t.set_u32(s + 0x1c, wait.cast_unsigned());
        for o in [0x28u32, 0x2c] {
            t.set_u32(s + o, (0.5f32 + o as f32).to_bits());
        }
        t.set_u32(s + 0x1084, chr);
        t.set_u32(s + 0x1058, chr);
        t.write(chr, &[0x11; 0xa0]);
        t.set_u16(chr + 0x90, flags);
        t.set_u32(task, node);
        t.set_u32(task + 4, s);
        t.write(node, &[0x33; 0x10]);
        t.set_u32(node + 4, SCRATCH + 0x700);
        t.set_u32(node + 8, SCRATCH + 0x740);
        t.write(SCRATCH + 0x700, &[0x77; 0x80]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `m0669::spark_record` and `spark_record_short` (0x100011b0,
/// 0x10001400): no record free, and records taken with several draws.
#[test]
fn spark_records_0669() {
    let Some(mut t) = magic_l3::twin(669) else {
        return;
    };
    let hdr = SCRATCH + 0x400;
    t.write(hdr, &[0x99; 0x20]);
    rec(&mut t, 942, 1, |_, a| 0x9420_0000 ^ a[0]);
    rec(&mut t, 941, 1, move |_, _| hdr);
    rec(&mut t, 324, 0, |m, _| {
        let k = m.r32(SCRATCH + 0x20);
        m.w32(SCRATCH + 0x20, k + 1);
        k.wrapping_mul(0x9e37_79b9) >> 9
    });
    rec(&mut t, 403, 1, |_, a| 0x0403_0000 ^ a[0]);
    t.stub_st0(403, |_, a| f64::from(f32::from_bits(a[0])) * 1.5 - 0.25);
    let recs_at = SCRATCH + 0x1000;
    let point = SCRATCH + 0x200;
    for rva in [0x11b0u32, 0x1400] {
        let f = t.at(rva);
        let take = t.img.rel32_target(f + 0x0c);
        t.set_u32(t.img.word(take + 0xb), recs_at);
        t.set_u32(t.img.word(f + 0x06), 0x00a1_0000);
        for (k, (n, used)) in [(0u32, 0u32), (3, 3), (3, 1), (2, 0)]
            .into_iter()
            .enumerate()
        {
            t.write(recs_at, &[0x22; 0x94 * 3]);
            for i in 0..3 {
                t.set_u32(recs_at + 0x94 * i, u32::from(i < used));
            }
            t.set_u32(t.img.word(take + 0x2), n);
            for i in 0..3u32 {
                t.set_u32(point + 4 * i, (1.5f32 + i as f32 - k as f32).to_bits());
            }
            t.set_u32(SCRATCH + 0x20, 5 * k as u32);
            t.check_entry(f, Call::default(), &[point], Eax::Checked);
        }
    }
}

/// `m0673::effect_start` and its cue start (0x10002080, 0x10001200): the
/// task list empty and not; the cue's waits.
#[test]
fn effect_start_0673() {
    let Some(mut t) = magic_l3::twin(673) else {
        return;
    };
    let f = t.at(0x2080);
    let la = t.img.rel32_target(f + 0x3d);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    for (s, n) in [(711u32, 3usize), (710, 7), (413, 1), (215, 1)] {
        rec(&mut t, s, n, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    let pb = t.img.rel32_target(f + 0x60);
    t.set_u32(t.img.word(pb + 0x01), SCRATCH + 0x800);
    let ri = t.img.rel32_target(f + 0x102);
    let take = t.img.rel32_target(f + 0xc4);
    for (k, busy) in [false, true].into_iter().enumerate() {
        t.set_u32(t.img.word(la + 0x34), 0);
        t.set_u32(t.img.word(la + 0x01), 0x0106_0000 + k as u32);
        t.set_u32(t.img.word(la + 0x12), 0x0c12_0000 + k as u32);
        let node = |i: u32| SCRATCH + 0x5000 + 0x40 * i;
        for i in 0..3u32 {
            t.write(node(i), &[0x33; 0x40]);
            t.set_u32(node(i), node((i + 2) % 3));
            t.set_u32(node(i) + 4, node((i + 1) % 3));
        }
        t.set_u32(t.img.word(ri + 0x3e), node(0));
        t.set_u16(t.img.word(ri + 0x20), 3);
        t.set_u32(t.img.word(ri + 0xa6), 0);
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
        let last = SCRATCH + 0x3400;
        t.write(last, &[0x34; 0x20]);
        t.set_u32(t.img.word(f + 0xca), if busy { last } else { 0 });
        t.set_u32(t.img.word(f + 0xde), last);
        // the record block the start points the record global at
        t.write(t.img.word(f + 0x24), &vec![0x24; 0x6000]);
        for op in [0x87u32, 0x92, 0x9a, 0xab, 0xb6, 0xbe] {
            t.set_u32(t.img.word(f + op), 0x5a5a_5a5a);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    let cue = t.at(0x1200);
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

/// The 0x74-record pool of `sb_misc::pool_take` at `take`: `count`
/// records from `recs`, the ones in `busy` taken.
fn pool74(t: &mut Twin, take: u32, recs: u32, count: u32, busy: u32) {
    let w = |t: &Twin, o: u32| t.img.word(take + o);
    t.set_u32(w(t, 0x37), recs);
    for i in 0..8 {
        t.write(recs + 0x74 * i, &[0x5a; 0x74]);
        t.set_u32(recs + 0x74 * i + 0x30, 0x300 + i);
        t.set_u32(recs + 0x74 * i + 0x6c, (busy >> i) & 1);
    }
    t.set_u32(w(t, 0x0f), count);
    t.set_u32(w(t, 0x08), 0);
    t.set_u32(w(t, 0x2b), 0);
}

/// `m0673::drift_record` and `timers_start` (0x100013b0, 0x10001700):
/// no record free and records taken; the task list empty and not.
#[test]
fn drift_and_timers_0673() {
    let Some(mut t) = magic_l3::twin(673) else {
        return;
    };
    let f = t.at(0x13b0);
    let take = t.img.rel32_target(f + 0x06);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(&mut t, 941, 1, |_, a| a[0] + 1);
    rec(&mut t, 237, 7, |_, a| 0x0237_0000 ^ a[3]);
    rec(&mut t, 314, 1, |m, a| {
        let k = m.r32(SCRATCH + 0x20);
        m.w32(SCRATCH + 0x20, k + 1);
        0x0314_0000 ^ a[0] ^ k
    });
    t.stub_st0(314, |m, a| {
        let k = m.r32(SCRATCH + 0x20);
        f64::from(f32::from_bits(a[0])) * (0.125 + f64::from(k % 7) * 0.0625)
    });
    t.set_u32(t.img.word(f + 0x57), 0x0057_0057);
    let recs = SCRATCH + 0x1000;
    for (count, busy) in [(0u32, 0u32), (4, 0), (4, 0b0011)] {
        pool74(&mut t, take, recs, count, busy);
        t.set_u32(SCRATCH + 0x20, count + busy);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    let f = t.at(0x1700);
    let take = t.img.rel32_target(f + 0x77);
    let r = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0x01), r);
    for busy in [false, true] {
        t.write(r, &vec![0x42; 0x5a00]);
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
        let last = SCRATCH + 0x3400;
        t.write(last, &[0x34; 0x20]);
        t.set_u32(t.img.word(f + 0x7d), if busy { last } else { 0 });
        t.set_u32(t.img.word(f + 0x93), last);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `m0666::cue_frame` (0x10002a90): not running, and each cue mask (the
/// stop-once flags clear and set); the three timelines live each time.
#[test]
fn cue_frame_0666() {
    let Some(mut t) = magic_l3::twin(666) else {
        return;
    };
    let f = t.at(0x2a90);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 261, 1, |m, a| {
        (m.r32(SCRATCH + 0x10) >> (a[0] & 3)) & 1
    });
    rec(&mut t, 253, 0, |_, _| 1);
    for (s, n) in [(723u32, 1usize), (724, 2), (214, 1)] {
        rec(&mut t, s, n, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    t.set_u32(w(&t, 0x0d), SCRATCH + 0x800);
    t.set_u32(w(&t, 0x3f), SCRATCH + 0x804);
    for (op, v) in [
        (0x13u32, 0x0013_0013u32),
        (0x31, 0x0031_0031),
        (0x44, 0x0044_0044),
        (0x10b, 0x010b_010b),
    ] {
        t.set_u32(w(&t, op), v);
    }
    for (k, (run, cues, flags)) in [
        (0u32, 0u32, 0u32),
        (1, 0b0000, 0),
        (1, 0b0001, 0),
        (1, 0b0010, 0),
        (1, 0b0010, 1),
        (1, 0b0100, 0),
        (1, 0b0100, 2),
        (1, 0b1000, 0),
        (1, 0b1111, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(w(&t, 0x02), run);
        t.set_u32(SCRATCH + 0x10, cues);
        t.set_u32(w(&t, 0x82), flags & 1);
        t.set_u32(w(&t, 0xbf), flags >> 1);
        t.set_u32(w(&t, 0x122), 0x40 + k);
        for (i, op) in (0u32..).zip([0x61u32, 0x9e, 0xdb]) {
            let tl = w(&t, op);
            magic_h6::live_timeline(&mut t, tl, SCRATCH + 0x900 + 0x40 * i, 3 + k);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `m0666::look_draw` (0x10001330): slot 712 0 and not; the first object
/// and another.
#[test]
fn look_draw_0666() {
    let Some(mut t) = magic_l3::twin(666) else {
        return;
    };
    for (s, n) in [
        (231, 3),
        (714, 5),
        (229, 3),
        (237, 4),
        (266, 3),
        (235, 2),
        (325, 2),
        (233, 3),
    ] {
        rec(&mut t, s, n, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    rec(&mut t, 941, 2, |_, _| SCRATCH + 0x400);
    rec(&mut t, 230, 2, |m, a| {
        let mut h = 0u32;
        for i in 0..16 {
            h = h.rotate_left(5) ^ m.r32(a[1] + 4 * i);
        }
        m.w32(SCRATCH + 0x83c, h);
        0x0230_0000 ^ a[0]
    });
    rec(&mut t, 712, 2, |m, _| {
        m.w32(SCRATCH + 0x830, m.r32(SCRATCH + 0x800));
        // the item word it sees, then moved on (as the game's part code may)
        m.w32(SCRATCH + 0x838, m.r32(SCRATCH + 0x810));
        m.w32(SCRATCH + 0x810, 0x0712_0712);
        m.r32(SCRATCH + 0x10)
    });
    rec(&mut t, 713, 1, |m, a| {
        m.w32(SCRATCH + 0x834, m.r32(SCRATCH + 0x810));
        m.w32(SCRATCH + 0x810, 0x0713_0713);
        0x0713_0000 ^ a[0]
    });
    for i in 0..16 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x0f00_0000 + i);
    }
    let f = t.at(0x1330);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    for op in [0xa4u32, 0xc0] {
        t.set_u32(w(&t, op), SCRATCH + 0x800);
    }
    for op in [0x97u32, 0xcf, 0xe8] {
        t.set_u32(w(&t, op), SCRATCH + 0x810);
    }
    let q = SCRATCH + 0x820;
    t.set_u32(w(&t, 0xf3), q);
    t.set_u32(q + 0x10, 0x0010_0010);
    t.set_u32(w(&t, 0x1a), 0x001a_0000);
    t.set_u32(w(&t, 0x3a), 0x003a_0000);
    let base = w(&t, 0x2a);
    let other = SCRATCH + 0x7000;
    for (k, s712) in [0u32, 1].into_iter().enumerate() {
        t.set_u32(SCRATCH + 0x10, s712);
        t.set_u32(SCRATCH + 0x800, 0x4444 + k as u32);
        for obj in [base, other] {
            t.write(obj, &[0x40 + k as u8; 0x80]);
            t.set_u32(obj + 0x60, (1.5f32 + k as f32).to_bits());
            t.check_entry(f, Call::default(), &[obj], Eax::Checked);
        }
    }
}
