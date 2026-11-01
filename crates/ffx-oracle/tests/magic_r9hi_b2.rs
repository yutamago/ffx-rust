//! Round 9 (own rows from `magic_0450` up), batch 2: own functions of
//! `magic_0700` and later DLLs against their Rust ports. Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::type_complexity,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

mod magic;
mod magic_l3;
#[path = "magic/ownb.rs"]
mod ownb;

use magic::{Call, Eax, SCRATCH, Twin};
use ownb::{fill, rec};

/// Every slot in `slots` recorded with its word count, answering a
/// distinct value mixed with its first word.
fn recs(t: &mut Twin, slots: &[(u32, usize)]) {
    for &(s, n) in slots {
        rec(t, s, n, move |_, a| {
            (0x0abc_0000 | s) ^ a.first().copied().unwrap_or(0)
        });
    }
}

/// `m0700::start` (0x10001110): the actor's mode byte through slot 218,
/// the lock held around slot 711, the part data's programs, the clut.
#[test]
fn start_0700() {
    let Some(mut t) = magic_l3::twin(700) else {
        return;
    };
    let f = t.at(0x1110);
    let modes = SCRATCH + 0x700;
    t.set_u32(modes, 0x5a5a_5a37);
    rec(&mut t, 218, 2, move |_, _| modes);
    let table = SCRATCH + 0x800;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0b00_0000 + 0x11 * i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(t.img.word(f + 0x7e), lock);
    rec(&mut t, 711, 3, move |m, a| {
        m.w32(SCRATCH + 0x7f8, m.r32(lock));
        0x0711_0000 ^ a[0]
    });
    recs(
        &mut t,
        &[
            (168, 1),
            (215, 1),
            (710, 4),
            (718, 1),
            (222, 4),
            (221, 4),
            (719, 3),
            (714, 5),
            (268, 3),
            (345, 1),
            (350, 1),
            (351, 3),
            (349, 2),
        ],
    );
    let v = t.img.word(f + 0x10c);
    fill(&mut t, v, 12, 3);
    for k in 0..2u32 {
        t.set_u32(lock, 0x4444 + k);
        // the words the start sets: old values
        let flag = t.img.word(f + 0x11f);
        t.set_u32(flag, 0x5a5a_5a5a);
        let one = t.img.word(f + 0x160);
        t.set_u32(one, 0x1234);
        t.check_entry(f, Call::default(), &[0x0a0a_0000 + k], Eax::Checked);
    }
}

/// `m0700::frame` (0x10001320): each wait, the step negative (slot 347
/// even and odd) and every fade step around both ranges.
#[test]
fn frame_0700() {
    let Some(mut t) = magic_l3::twin(700) else {
        return;
    };
    let f = t.at(0x1320);
    let g = |t: &Twin, op: u32| t.img.word(f + op);
    rec(&mut t, 225, 1, |m, _| m.r32(SCRATCH + 0x10));
    rec(&mut t, 227, 1, |m, _| m.r32(SCRATCH + 0x14));
    rec(&mut t, 347, 1, |m, _| m.r32(SCRATCH + 0x18));
    recs(&mut t, &[(226, 2), (220, 3), (239, 3), (349, 2), (348, 6)]);
    for op in [0x10u32, 0xc1, 0xed, 0xf8] {
        t.set_u32(g(&t, op), 0x00a5_0000 | op);
    }
    // (go, once, run, slot 225, slot 227, slot 347, step)
    let mut cases = vec![
        (0u32, 0u32, 0u32, 1u32, 1u32, 1u32, 0u32),
        (0, 0, 0, 0, 0, 1, 0),
        (0, 0, 0, 0, 1, 2, 0xffff_ffff),
        (1, 1, 1, 0, 0, 3, 0xffff_ffff),
        (1, 1, 1, 0, 0, 0, 0x8000_0000),
    ];
    for n in [0u32, 1, 2, 7, 14, 15, 16, 17, 30, 44, 45, 46, 60] {
        cases.push((1, 1, 1, 0, 0, 0, n));
    }
    for (go, once, run, s225, s227, s347, step) in cases {
        t.set_u32(g(&t, 0x2), go);
        t.set_u32(g(&t, 0x2f), once);
        t.set_u32(g(&t, 0x5a), run);
        t.set_u32(SCRATCH + 0x10, s225);
        t.set_u32(SCRATCH + 0x14, s227);
        t.set_u32(SCRATCH + 0x18, s347);
        t.set_u32(g(&t, 0x88), step);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// Slot 169's float (ST0) from its argument, and its call recorded.
fn shade_stub(t: &mut Twin) {
    rec(t, 169, 1, |_, a| 0x0169_0000 ^ a[0]);
    t.stub_st0(169, |_, a| f64::from(a[0] & 0xff) * 0.37 + 0.5);
}

/// `m0707::effect_start` (0x10001210): the actor 0xff and others, the
/// lock held around slot 711.
#[test]
fn effect_start_0707() {
    let Some(mut t) = magic_l3::twin(707) else {
        return;
    };
    let f = t.at(0x1210);
    rec(&mut t, 168, 1, |m, _| m.r32(SCRATCH + 0x10));
    shade_stub(&mut t);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(t.img.word(f + 0x8f), lock);
    rec(&mut t, 711, 3, move |m, a| {
        m.w32(SCRATCH + 0x7f8, m.r32(lock));
        0x0711_0000 ^ a[0]
    });
    recs(
        &mut t,
        &[(215, 1), (710, 4), (718, 1), (222, 4), (221, 4), (719, 3)],
    );
    for (k, actor) in [0xffu32, 3, 0x1fe, 0].into_iter().enumerate() {
        t.set_u32(SCRATCH + 0x10, actor);
        t.set_u32(lock, 0x4444 + k as u32);
        t.set_u32(t.img.word(f + 0x81), 7);
        t.check_entry(f, Call::default(), &[0x0a0a_0000 + k as u32], Eax::Checked);
    }
}

/// `m0707::frame` (0x10001310): each wait, paused, the first frame (the
/// sound id 0xff or not), the sound's steps, part frames with free and
/// busy parts (slot 307's floats in ST0), frame 5's light.
#[test]
fn frame_0707() {
    let Some(mut t) = magic_l3::twin(707) else {
        return;
    };
    let f = t.at(0x1310);
    let g = |t: &Twin, op: u32| t.img.word(f + op);
    rec(&mut t, 225, 1, |m, _| m.r32(SCRATCH + 0x10));
    rec(&mut t, 227, 1, |m, _| m.r32(SCRATCH + 0x14));
    rec(&mut t, 253, 0, |m, _| m.r32(SCRATCH + 0x18));
    rec(&mut t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, |m, _| {
        let k = m.r32(SCRATCH + 0x20);
        m.w32(SCRATCH + 0x20, k + 1);
        f64::from(k % 11) * 0.125 + 0.0625
    });
    rec(&mut t, 247, 2, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, (a[1] ^ i).wrapping_mul(0x0101_0101));
        }
        0x247
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[2] + 4 * i) ^ m.r32(a[1] + 8 * i) ^ (i << 4);
            m.w32(a[0] + 4 * i, v);
        }
        0x229
    });
    let table = SCRATCH + 0x800;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0b00_0000 + 0x11 * i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(226, 2), (228, 2), (278, 3), (220, 3), (714, 5)]);
    let parts = g(&t, 0x19e);
    // (go, once, run, 225, 227, 253, time, sound kind, sound time, frame, free part)
    let cases: Vec<(
        u32,
        u32,
        u32,
        u32,
        u32,
        u32,
        u32,
        u32,
        u32,
        u32,
        Option<u32>,
    )> = vec![
        (0, 0, 0, 1, 1, 1, 0, 0, 0, 0, None),
        (0, 0, 0, 0, 0, 1, 0, 0, 0, 0, None),
        (0, 0, 0, 0, 1, 0, 0, 0, 0, 0, None),
        (1, 1, 1, 0, 1, 1, 0, 0xff, 0, 0, None),
        (1, 1, 1, 0, 1, 1, 7, 5, 0, 0, Some(0)),
        (1, 1, 1, 0, 1, 1, 7, 0xff, 0, 0, Some(0)),
        (1, 1, 1, 0, 1, 1, 0, 5, 0, 5, Some(0)),
        (1, 1, 1, 0, 1, 1, 3, 0xc8, 0x40, 0xd, Some(5)),
        (1, 1, 1, 0, 1, 1, 0x40, 0xc8, 0x40, 0xd, None),
        (1, 1, 1, 0, 1, 1, 0x5a, 0xc9, 0, 0x15, Some(15)),
        (1, 1, 1, 0, 1, 1, 0x59, 0xc9, 0, 0x16, Some(3)),
        (1, 1, 1, 0, 1, 1, 0x5a, 0xca, 0, 0x131, Some(2)),
        (1, 1, 1, 0, 1, 1, 0x5a, 0xca, 0, 0x135, Some(2)),
        (1, 1, 1, 0, 1, 1, 0x5a, 0xca, 0, 4, Some(2)),
    ];
    for (go, once, run, s225, s227, s253, time, kind, stime, frame, free) in cases {
        t.set_u32(g(&t, 0x12), go);
        t.set_u32(g(&t, 0x3f), once);
        t.set_u32(g(&t, 0x6a), run);
        t.set_u32(SCRATCH + 0x10, s225);
        t.set_u32(SCRATCH + 0x14, s227);
        t.set_u32(SCRATCH + 0x18, s253);
        // the time G_T, the sound record (+0x0 id .. +0x10)
        t.set_u32(g(&t, 0x15d), time);
        t.set_u32(g(&t, 0xd2), kind);
        t.set_u32(g(&t, 0x13e), kind);
        t.set_u32(g(&t, 0x163), stime);
        t.set_u32(g(&t, 0x17b), frame);
        // the words the first frame sets: old values
        t.set_u32(g(&t, 0x108), 0x7777);
        for i in 0..16u32 {
            let busy = free.is_none_or(|k| i < k);
            t.write(parts + 0x80 * i, &[0x33; 0x80]);
            t.set_u16(parts + 0x80 * i + 0x1e, u16::from(busy));
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `m0707::draw` (0x10001670): not running, the time negative; the bone
/// matrix moving the live parts; the sound's end.
#[test]
fn draw_0707() {
    let Some(mut t) = magic_l3::twin(707) else {
        return;
    };
    let f = t.at(0x1670);
    let g = |t: &Twin, op: u32| t.img.word(f + op);
    rec(&mut t, 236, 3, |m, a| {
        for i in 0..16 {
            m.w32(a[2] + 4 * i, 0x3f00_0000 + i);
        }
        0x236
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[2] + 4 * i) ^ m.r32(a[1] + 4 * i) ^ (i << 8);
            m.w32(a[0] + 4 * i, v ^ 0x0100);
            m.w32(a[2] + 4 * i, v);
        }
        0x229
    });
    recs(&mut t, &[(724, 1), (278, 3), (213, 2), (214, 1)]);
    // slot 723 records the environment pointer the draw sets
    let env = t.u32(g(&t, 0x37));
    rec(&mut t, 723, 1, move |m, a| {
        m.w32(SCRATCH + 0x7c0, m.r32(env));
        m.w32(env, 0x0bad);
        0x0723 ^ a[0]
    });
    let pos = g(&t, 0x8a);
    for (on, time, kind, a0, live) in [
        (0u32, 0u32, 0u32, 0u32, 0u32),
        (1, 0xffff_ffff, 0, 0, 0),
        (1, 0, 0, 0, 0x0001),
        (1, 4, 0xca, 1, 0x8421),
        (1, 4, 0xca, 2, 0xffff),
        (1, 5, 0xc9, 2, 0x0f0f),
    ] {
        t.set_u32(g(&t, 0x12), on);
        t.set_u32(g(&t, 0x1f), time);
        t.set_u32(g(&t, 0xe6), kind);
        t.set_u32(g(&t, 0xf3), a0);
        t.set_u32(g(&t, 0xf9), 2);
        for i in 0..16u32 {
            let p = pos + 0x80 * i - 0x40;
            t.write(p, &[0x21 + i as u8; 0x80]);
            t.set_u16(p + 0x1e, u16::from(live >> i & 1 == 1));
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `m0704::start` (0x100011d0): the actor at, below and two below 0xff
/// (each of the three shades skipped in turn), the lock around slot 711.
#[test]
fn start_0704() {
    let Some(mut t) = magic_l3::twin(704) else {
        return;
    };
    let f = t.at(0x11d0);
    rec(&mut t, 168, 1, |m, _| m.r32(SCRATCH + 0x10));
    let modes = SCRATCH + 0x700;
    t.set_u32(modes, 0x5a5a_5a37);
    rec(&mut t, 218, 2, move |_, _| modes);
    shade_stub(&mut t);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(t.img.word(f + 0x1b0), lock);
    rec(&mut t, 711, 3, move |m, a| {
        m.w32(SCRATCH + 0x7f8, m.r32(lock));
        0x0711_0000 ^ a[0]
    });
    recs(
        &mut t,
        &[
            (215, 1),
            (710, 4),
            (718, 1),
            (222, 4),
            (221, 4),
            (719, 3),
            (278, 3),
        ],
    );
    for (k, actor) in [0x10u32, 0xff, 0xfe, 0xfd].into_iter().enumerate() {
        t.set_u32(SCRATCH + 0x10, actor);
        let flag = t.img.word(f + 0x1a2);
        t.set_u32(flag, 0x5a5a);
        t.set_u32(lock, 0x4444 + k as u32);
        t.check_entry(f, Call::default(), &[0x0a0a_0000 + k as u32], Eax::Checked);
    }
}

/// `m0704::draw` (0x100018e0): not running, the time negative; the light
/// off, fading in, held, fading out (with the fade's steps) and done; the
/// sounds' end.
#[test]
fn draw_0704() {
    let Some(mut t) = magic_l3::twin(704) else {
        return;
    };
    let f = t.at(0x18e0);
    let g = |t: &Twin, op: u32| t.img.word(f + op);
    recs(
        &mut t,
        &[
            (723, 1),
            (724, 1),
            (352, 3),
            (776, 1),
            (915, 5),
            (213, 2),
            (214, 1),
        ],
    );
    let rec0 = g(&t, 0xe4);
    for i in 0..3 {
        t.set_u32(rec0 + 0x18 * i, 0x0050_0000 + i);
    }
    for (on, time, flag, t8, kind, a0) in [
        (0u32, 0u32, 0u16, 0u32, 0u32, 0u32),
        (1, 0x8000_0000, 1, 0, 0, 0),
        (1, 0, 0, 0, 0, 0),
        (1, 1, 0, 0, 0xb2, 7),
        (1, 1, 1, 0x1_d000, 0xb2, 6),
        (1, 1, 1, 0x1_e000, 0xb1, 6),
        (1, 1, 1, 0x4_f000, 0, 0),
        (1, 1, 1, 0x5_0000, 0, 0),
        (1, 1, 1, 0x5_1000, 0, 0),
        (1, 1, 1, 0x5_4000, 0, 0),
        (1, 1, 1, 0x5_9000, 0, 0),
        (1, 1, 1, 0x5_a000, 0, 0),
        (1, 1, 1, 0xffff_f000, 0, 0),
    ] {
        t.set_u32(g(&t, 0x2), on);
        t.set_u32(g(&t, 0xf), time);
        t.set_u16(g(&t, 0x5c), flag);
        t.set_u16(g(&t, 0x5c) + 2, 0x5a5a);
        for op in [0xb5u32, 0xbf] {
            let part = g(&t, op);
            t.set_u16(part + 0x1e, u16::from(t8 & 0x1000 != 0));
        }
        t.set_u32(g(&t, 0x65), t8);
        t.set_u32(g(&t, 0xca), kind);
        t.set_u32(g(&t, 0xd5), a0);
        t.set_u32(g(&t, 0xdb), 6);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `m0705::start` (0x100014d0): the actor 0xff and not; the target list
/// empty, of one, of three, and of four (the fourth's record overlaps the
/// centre's sums) with a 0xff among them; slot 289's floats.
#[test]
fn start_0705() {
    let Some(mut t) = magic_l3::twin(705) else {
        return;
    };
    let f = t.at(0x14d0);
    let g = |t: &Twin, op: u32| t.img.word(f + op);
    rec(&mut t, 168, 1, |m, _| m.r32(SCRATCH + 0x10));
    let modes = SCRATCH + 0x700;
    t.set_u32(modes, 0x5a5a_5a37);
    let list = SCRATCH + 0x720;
    t.write(list, &[3, 0xff, 9, 0x20, 0, 0, 0, 0]);
    let count = g(&t, 0xee);
    // slot 218: the mode byte for the actor, the list (and its count)
    rec(&mut t, 218, 2, move |m, a| {
        if a[1] == count {
            m.w32(count, m.r32(SCRATCH + 0x14));
            list
        } else {
            modes
        }
    });
    shade_stub(&mut t);
    rec(&mut t, 289, 1, |_, a| 0x0289_0000 ^ a[0]);
    t.stub_st0(289, |_, a| f64::from(a[0] & 0x3f) * -0.25 + 1.5);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(g(&t, 0x286), lock);
    rec(&mut t, 711, 3, move |m, a| {
        m.w32(SCRATCH + 0x7f8, m.r32(lock));
        0x0711_0000 ^ a[0]
    });
    recs(
        &mut t,
        &[
            (215, 1),
            (217, 2),
            (710, 4),
            (271, 1),
            (718, 1),
            (222, 4),
            (221, 4),
            (719, 3),
        ],
    );
    let recs0 = g(&t, 0x134) - 0x18;
    for (k, (actor, n)) in [
        (0xffu32, 0u32),
        (5, 1),
        (0x10, 4),
        (0x11, 3),
        (0xff, 0xffff_fffe),
        (6, 0),
    ]
    .into_iter()
    .enumerate()
    {
        t.set_u32(SCRATCH + 0x10, actor);
        t.set_u32(SCRATCH + 0x14, n);
        t.set_u32(lock, 0x4444 + k as u32);
        for op in [0x25au32, 0x264, 0x278] {
            let a = g(&t, op);
            t.set_u32(a, 0x5a5a_0000 | op);
        }
        for i in 0..4u32 {
            t.write(recs0 + 0x28 * i, &[0x11 * (i as u8 + 1); 0x28]);
            t.set_u32(recs0 + 0x28 * i + 0x18, (1.0f32 + i as f32).to_bits());
            t.set_u32(recs0 + 0x28 * i + 0x20, (0.5f32 * i as f32).to_bits());
        }
        t.check_entry(f, Call::default(), &[0x0a0a_0000 + k as u32], Eax::Checked);
    }
}

/// `m0704::frame` (0x10001400): each wait, paused, the first frame (one
/// sound id 0xff), the sounds' steps (0xb0 to 0xb1 at their time, 0xb1 to
/// 0xb2 at frame 0x5a), the light's start at the first sound's frame 0x22,
/// the second part's start at the light's time 0x50000 and its run (its
/// time 0xf000 and not).
#[test]
fn frame_0704() {
    let Some(mut t) = magic_l3::twin(704) else {
        return;
    };
    let f = t.at(0x1400);
    let g = |t: &Twin, op: u32| t.img.word(f + op);
    rec(&mut t, 225, 1, |m, _| m.r32(SCRATCH + 0x10));
    rec(&mut t, 227, 1, |m, _| m.r32(SCRATCH + 0x14));
    rec(&mut t, 253, 0, |m, _| m.r32(SCRATCH + 0x18));
    let table = SCRATCH + 0x800;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0b00_0000 + 0x11 * i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 247, 2, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, (a[1] ^ i).wrapping_mul(0x0101_0101));
        }
        0x247
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[2] + 4 * i) ^ m.r32(a[1] + 8 * i) ^ (i << 4);
            m.w32(a[0] + 4 * i, v);
        }
        0x229
    });
    rec(&mut t, 289, 1, |_, a| 0x0289_0000 ^ a[0]);
    t.stub_st0(289, |_, a| f64::from(a[0] & 0x3f) * 0.25 - 1.0);
    recs(
        &mut t,
        &[
            (226, 2),
            (228, 2),
            (220, 3),
            (714, 5),
            (237, 4),
            (217, 2),
            (239, 3),
        ],
    );
    let lock = SCRATCH + 0x7f0;
    let sp = t.img.rel32_target(f + 0x20f);
    t.set_u32(t.img.word(sp + 0x04), lock);
    let rec0 = g(&t, 0x1a9);
    // (go, once, run, 225, 227, 253, first, [(step, time, frame)], light, part, part time)
    let s3 = |a: (u32, u32, u32)| [a, (0, 0, 0), (0, 0, 0)];
    let cases: Vec<(
        u32,
        u32,
        u32,
        u32,
        u32,
        u32,
        bool,
        [(u32, u32, u32); 3],
        (u16, u32),
        u16,
        u32,
    )> = vec![
        (0, 0, 0, 1, 1, 1, false, s3((0, 0, 0)), (0, 0), 0, 0),
        (0, 0, 0, 0, 0, 1, false, s3((0, 0, 0)), (0, 0), 0, 0),
        (0, 0, 0, 0, 1, 0, false, s3((0, 0, 0)), (0, 0), 0, 0),
        (1, 1, 1, 0, 1, 1, true, s3((0, 0, 0)), (0, 0), 0, 0),
        (
            1,
            1,
            1,
            0,
            1,
            1,
            false,
            [(0xb0, 7, 7), (0xb1, 0, 0x5a), (0xb0, 3, 4)],
            (1, 0x5_0000),
            0,
            0,
        ),
        (
            1,
            1,
            1,
            0,
            1,
            1,
            false,
            [(0xb1, 0, 0x22), (0xb1, 0, 0x59), (0xb2, 0, 0)],
            (1, 0x4_0000),
            1,
            0xf000,
        ),
        (
            1,
            1,
            1,
            0,
            1,
            1,
            false,
            [(0xb1, 0, 0x5a), (0xb0, 9, 9), (0xb1, 0, 0x5a)],
            (0, 0x5_0000),
            1,
            0xe000,
        ),
        (
            1,
            1,
            1,
            0,
            1,
            1,
            false,
            [(0xb1, 0, 0x23), (0, 0, 0), (0, 0, 0)],
            (0, 0),
            0,
            0,
        ),
    ];
    for (k, (go, once, run, s225, s227, s253, first, sounds, light, part, ptime)) in
        cases.into_iter().enumerate()
    {
        t.set_u32(g(&t, 0x12), go);
        t.set_u32(g(&t, 0x3f), once);
        t.set_u32(g(&t, 0x6a), run);
        t.set_u32(SCRATCH + 0x10, s225);
        t.set_u32(SCRATCH + 0x14, s227);
        t.set_u32(SCRATCH + 0x18, s253);
        t.set_u32(g(&t, 0xc4), u32::from(!first));
        for (i, (step, time, frame)) in (0u32..).zip(sounds) {
            let at = rec0 + 0x18 * i;
            t.set_u32(at - 0x10, if i == 1 && k == 3 { 0xff } else { 5 + i });
            t.set_u32(at - 8, step);
            t.set_u32(at - 4, time);
            t.set_u32(at, frame);
            t.set_u32(at + 4, 0x1234_0000 + i);
        }
        // the shade the light's angle comes from
        t.set_u16(rec0 + 6, 0x0123 + k as u16);
        let lp = g(&t, 0x390);
        t.set_u16(lp, light.0);
        t.set_u32(g(&t, 0x3a0), light.1);
        t.set_u16(g(&t, 0x43b), part);
        t.set_u32(g(&t, 0x482), ptime);
        t.set_u32(lock, 0x4444 + k as u32);
        // the light's words the start clears: old values
        for op in [0x258u32, 0x25e] {
            let a = g(&t, op);
            t.set_u32(a, 0x5a5a_0000 | op);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}
