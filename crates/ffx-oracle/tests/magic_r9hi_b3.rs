//! Round 9 (own rows from `magic_0450` up), batch 3: the throw tasks of
//! `magic_0684` and `magic_0493`, `magic_0493`'s particle start and
//! `magic_0459`'s model task against their Rust ports. Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
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

/// `m0684::step` (0x10006e30): every step: the default, the list unlink,
/// step 12 before and at frame 0x3c, the set-up, the counts of three
/// targets, and the throws (waiting, slot busy, the last throw, a throw
/// with the shares recomputed).
#[test]
fn step_0684() {
    let Some(mut t) = magic_l3::twin(684) else {
        return;
    };
    let f = t.at(0x6e30);
    let g = |t: &Twin, op: u32| t.img.word(f + op);
    rec(&mut t, 291, 5, |m, a| {
        let k = m.r32(SCRATCH + 0x20);
        m.w32(SCRATCH + 0x20, k + 1);
        m.w32(a[2], 0x2910_0000 + k);
        1 + k % 3
    });
    rec(&mut t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, |m, _| {
        let k = m.r32(SCRATCH + 0x24);
        m.w32(SCRATCH + 0x24, k + 1);
        // draws (less the code's 0.99) falling on each target, past the
        // last, and below the first
        [1.0, 1.9, 1.45, 2.5, 1.3, 1.7, 0.5][k as usize % 7]
    });
    let list = SCRATCH + 0x8000;
    t.write(list, &[5, 7, 9, 11]);
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 0);
        list
    });
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0xffff_fff0u32.wrapping_add(0x11 * i));
        }
        0x558
    });
    for s in [331u32, 353, 271, 390, 534, 268, 12, 714, 325, 719] {
        rec(&mut t, s, 1, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * 0.25);
    for (s, x) in [
        (277u32, 0.5f64),
        (328, 1.25),
        (329, -0.75),
        (330, 2.0),
        (790, 0.375),
        (333, 1.5),
    ] {
        rec(&mut t, s, 0, move |_, _| s);
        t.stub_st0(s, move |_, _| x);
    }
    let task = SCRATCH + 0x100;
    let s = SCRATCH + 0x4000;
    let node = SCRATCH + 0x200;
    let (prev, next) = (SCRATCH + 0x300, SCRATCH + 0x340);
    let targets = g(&t, 0x25e);
    let count = g(&t, 0xeb);
    // (step, frame, wait, left, slot of left % 5 busy, targets)
    let cases = [
        (1u32, 0u32, 0u32, 0u32, false, 0u32),
        (0x2710, 3, 0, 0, false, 0),
        (0xc, 0x3b, 0, 0, false, 0),
        (0xc, 0x3c, 0, 0, false, 0),
        (0, 4, 2, 5, false, 0),
        (0xa, 4, 2, 5, false, 3),
        (0xb, 0, 2, 4, false, 3),
        (0xb, 0, 0, 4, true, 3),
        (0xb, 5, 0, 1, false, 3),
        (0xb, 0, 0, 7, false, 3),
        (0xb, 0, 0, 6, false, 0),
        (0xb, 0, 0, 9, false, 3),
        (0xb, 0, 0, 8, false, 3),
        (0xb, 0, 0, 12, false, 3),
        (0xb, 0, 0, 11, false, 3),
        (0xb, 0, 0, 13, false, 3),
    ];
    for (k, (step, frame, wait, left, busy, n)) in cases.into_iter().enumerate() {
        t.write(s, &[0x11; 0x1400]);
        t.set_u32(task, node);
        t.set_u32(task + 4, s);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.set_u32(prev + 4, node);
        t.set_u32(next + 8, node);
        t.set_u32(s + 0x18, step);
        t.set_u32(s + 0x40, frame);
        t.set_u32(s + 0x3c, wait);
        t.set_u32(s + 0x44, left);
        for i in 0..6 {
            // busy before the set-up, which frees them all
            let free = !(busy && i == left % 5) && step == 0xb;
            t.set_u32(s + 0x340 + 0x310 * i, if free { 0xff } else { 3 });
        }
        for i in 0..4 {
            t.set_u32(s + 0x48 + 4 * i, 2 + i);
            t.set_u32(s + 0x1c + 4 * i, (0.25f32 + 0.05 * i as f32).to_bits());
        }
        // past the targets' shares: small, so a loop running too far goes on
        t.set_u32(s + 0x1c + 4 * n, 0.001f32.to_bits());
        t.set_u32(s + 0x20 + 4 * n, 0.001f32.to_bits());
        t.set_u32(count, n);
        // the thrown effect's lists (`s5_misc::thrown_effect`'s operands)
        let te = t.img.rel32_target(f + 0x26d);
        let w = |t: &Twin, o: u32| t.img.word(te + o);
        let (tail, sentinel) = (w(&t, 0x6c), w(&t, 0x85));
        t.write(SCRATCH + 0x900, &[0x23; 0x80]);
        t.set_u32(w(&t, 0x232), SCRATCH + 0x900);
        t.set_u32(w(&t, 0x28), SCRATCH + 0x580);
        t.write(sentinel - 0x10, &[0x31; 0x2c]);
        t.set_u32(sentinel + 0x18, 0);
        t.set_u32(tail, sentinel);
        let plist = w(&t, 0x248);
        t.write(plist, &[0x41; 0x1c]);
        t.set_u32(plist + 0x10, plist);
        let pn = t.img.rel32_target(te + 0x259);
        let start = t.img.rel32_target(pn + 0x88);
        t.set_u32(t.img.word(start + 0x04), SCRATCH + 0x2f0);
        // the set-up's two parents: their last child themselves
        for op in [0x48u32, 0x76] {
            let p = g(&t, op);
            t.write(p, &[0x77; 0x40]);
            t.set_u32(p + 0x10, p);
        }
        t.write(targets, &[0x5a; 0x98 * 4]);
        t.set_u32(SCRATCH + 0x20, k as u32);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `m0493::step` (0x10006c30): every step: the default, step 10000 with
/// the sound playing and done, the set-up, the counts of three targets,
/// and the throws (waiting, slot busy, the last throw, a throw with the
/// shares recomputed).
#[test]
fn step_0493() {
    let Some(mut t) = magic_l3::twin(493) else {
        return;
    };
    let f = t.at(0x6c30);
    let g = |t: &Twin, op: u32| t.img.word(f + op);
    rec(&mut t, 291, 5, |m, a| {
        let k = m.r32(SCRATCH + 0x20);
        m.w32(SCRATCH + 0x20, k + 1);
        m.w32(a[2], 0x2910_0000 + k);
        1 + k % 3
    });
    rec(&mut t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, |m, _| {
        let k = m.r32(SCRATCH + 0x24);
        m.w32(SCRATCH + 0x24, k + 1);
        // draws (less the code's 0.99) falling on each target, past the
        // last, and below the first
        [1.0, 1.9, 1.45, 2.5, 1.3, 1.7, 0.5][k as usize % 7]
    });
    let list = SCRATCH + 0x8000;
    t.write(list, &[5, 7, 9, 11]);
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 0);
        list
    });
    rec(&mut t, 411, 1, |m, _| m.r32(SCRATCH + 0x28));
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            m.w32(a[2] + 4 * i, (i as f32 * 2.5 - 1.0).to_bits());
        }
        0x268
    });
    for s in [213u32, 228, 719, 333, 14, 413, 215] {
        rec(&mut t, s, 1, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    t.stub_st0(333, |_, _| 1.5);
    let task = SCRATCH + 0x100;
    let s = SCRATCH + 0x1_0000;
    let node = SCRATCH + 0x200;
    let (prev, next) = (SCRATCH + 0x300, SCRATCH + 0x340);
    let targets = g(&t, 0x1ea);
    let count = g(&t, 0x71);
    let init = t.img.rel32_target(f + 0x1f9);
    let wi = |t: &Twin, op: u32| t.img.word(init + op);
    // (step, sound done, wait, left, slot of left % 5 busy, targets)
    let cases = [
        (1u32, 0u32, 0u32, 0u32, false, 0u32),
        (0xc, 0, 0, 0, false, 0),
        (0x2710, 0, 0, 0, false, 0),
        (0x2710, 1, 0, 0, false, 0),
        (0, 4, 2, 5, false, 0),
        (0xa, 4, 2, 5, false, 3),
        (0xb, 0, 2, 4, false, 3),
        (0xb, 0, 0, 4, true, 3),
        (0xb, 5, 0, 1, false, 3),
        (0xb, 0, 0, 7, false, 3),
        (0xb, 0, 0, 6, false, 0),
        (0xb, 0, 0, 9, false, 3),
        (0xb, 0, 0, 8, false, 3),
        (0xb, 0, 0, 12, false, 3),
        (0xb, 0, 0, 11, false, 3),
        (0xb, 0, 0, 13, false, 3),
    ];
    for (k, (step, done, wait, left, busy, n)) in cases.into_iter().enumerate() {
        t.write(s, &vec![0x11; 0x4400]);
        t.set_u32(task, node);
        t.set_u32(task + 4, s);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.set_u32(prev + 4, node);
        t.set_u32(next + 8, node);
        t.set_u32(s + 0x18, step);
        t.set_u32(s + 0x3c, wait);
        t.set_u32(s + 0x40, left);
        t.set_u32(SCRATCH + 0x28, done);
        for i in 0..6 {
            // busy before the set-up, which frees them all
            let free = !(busy && i == left % 5) && step == 0xb;
            t.set_u32(s + 0xb58 + 0xb30 * i, if free { 0xff } else { 3 });
        }
        for i in 0..4 {
            t.set_u32(s + 0x44 + 4 * i, 2 + i);
            t.set_u32(s + 0x1c + 4 * i, (0.25f32 + 0.05 * i as f32).to_bits());
        }
        // past the targets' shares: small, so a loop running too far goes on
        t.set_u32(s + 0x1c + 4 * n, 0.001f32.to_bits());
        t.set_u32(s + 0x20 + 4 * n, 0.001f32.to_bits());
        t.set_u32(count, n);
        t.write(targets, &[0x5a; 0x98 * 4]);
        for i in 0..4 {
            t.set_u32(targets + 0x98 * i + 0x54, 0x00e5_0004 + i);
        }
        // the throw's set-up: its ring and node list (`m0493::init`)
        let ring = SCRATCH + 0x4000;
        t.write(ring, &[0x1b; 0x20]);
        t.set_u32(wi(&t, 0x27), ring);
        let sentinel = wi(&t, 0x7b);
        t.set_u32(wi(&t, 0x6b), sentinel);
        t.set_u16(wi(&t, 0x8f), 3);
        // the set-up's parent: its last child itself
        let p = g(&t, 0x41);
        t.write(p, &[0x77; 0x40]);
        t.set_u32(p + 0x10, p);
        t.set_u32(SCRATCH + 0x20, k as u32);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `m0493::ppp_start` (0x10006f10): `s5_misc::summon_ppp_start` with the
/// part data kind 2; no targets and two, the lock held around slot 711.
#[test]
fn ppp_start_0493() {
    let Some(mut t) = magic_l3::twin(493) else {
        return;
    };
    let f = t.at(0x6f10);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let ti = t.img.rel32_target(f + 0x8);
    let statics = t.img.rel32_target(ti + 0xe0);
    let root = t.img.word(statics + 0x10d);
    let list = SCRATCH + 0x6000;
    rec(&mut t, 168, 1, |_, a| 0x0168_0000 + a[0]);
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], m.r32(list + 0x7c));
        list
    });
    rec(&mut t, 531, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[1] + 4 * i, a[0].wrapping_mul(0x0101_0101) ^ i);
        }
        0x531
    });
    rec(&mut t, 289, 1, |_, _| 0x289);
    t.stub_st0(289, |_, a| f64::from(a[0] & 0xff) * -0.5);
    let lock = SCRATCH + 0x7f0;
    rec(&mut t, 711, 3, move |m, a| {
        m.w32(SCRATCH + 0x7f8, m.r32(lock));
        0x0711_0000 ^ a[0]
    });
    let data = SCRATCH + 0x800;
    rec(&mut t, 719, 3, move |_, _| data);
    for (s, n) in [(718u32, 1usize), (710, 4), (222, 4), (221, 4)] {
        rec(&mut t, s, n, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    for count in [0u32, 2] {
        // the statics the start sets: other values before
        for (i, o) in [
            0x13u32, 0x1d, 0x27, 0x31, 0x3b, 0x45, 0x58, 0x62, 0x6c, 0x76, 0x89, 0x93, 0x9d, 0xb2,
            0x1a8,
        ]
        .into_iter()
        .enumerate()
        {
            let a = w(&t, o);
            t.set_u32(a, 0x5a5a_0000 + i as u32 + count);
        }
        let a = w(&t, 0x50);
        t.set_u16(a, 0x1234);
        let a = w(&t, 0x81);
        t.set_u16(a, 0x1234);
        t.write(list, &[0x80, 0x81, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88]);
        t.set_u32(list + 0x7c, count);
        t.write(root, &[0x5a; 0x150]);
        t.set_u32(root + 4, 0);
        let a = w(&t, 0xd4);
        t.set_u32(a, lock);
        t.set_u32(lock, 0x0bad_0000 + count);
        // the ring's tail before: a record of its own
        t.write(SCRATCH + 0x7a0, &[0x3c; 0x10]);
        let a = w(&t, 0x15a);
        t.set_u32(a, SCRATCH + 0x7a0);
        let a = w(&t, 0x197);
        t.set_u32(a, 0xffff_ff0f + count);
        t.check_entry(f, Call::default(), &[0x0342_0040 + count], Eax::Checked);
    }
}

/// `m0459::step` (0x10006960): every step: the default, step 10000, step
/// 13 before and at frame 0x51, step 12 before frame 0x15 and with models
/// left and none, step 11 before frame 0x10 and with the node list empty
/// and not, the set-up and step 10.
#[test]
fn step_0459() {
    let Some(mut t) = magic_l3::twin(459) else {
        return;
    };
    let f = t.at(0x6960);
    let g = |t: &Twin, op: u32| t.img.word(f + op);
    let list = SCRATCH + 0x8000;
    t.write(list, &[5, 7, 9, 11]);
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 0);
        list
    });
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0xffff_fff0u32.wrapping_add(0x11 * i));
        }
        0x558
    });
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            m.w32(a[2] + 4 * i, (i as f32 * 2.5 - 1.0).to_bits());
        }
        0x268
    });
    for s in [331u32, 353, 271, 714, 325, 719, 239] {
        rec(&mut t, s, 1, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    for (s, x) in [(790u32, 0.375f64), (333, 1.5)] {
        rec(&mut t, s, 0, move |_, _| s);
        t.stub_st0(s, move |_, _| x);
    }
    let pn = t.img.rel32_target(f + 0x333);
    let start = t.img.rel32_target(pn + 0x88);
    t.set_u32(t.img.word(start + 0x04), SCRATCH + 0x2f0);
    let task = SCRATCH + 0x100;
    let s = SCRATCH + 0x1_0000;
    let node = SCRATCH + 0x200;
    let (prev, next) = (SCRATCH + 0x300, SCRATCH + 0x340);
    let n_models = g(&t, 0x1ec);
    // the particle's owner (`*G_O`): its program data at +0x10, vectors after
    let owner = SCRATCH + 0x900;
    t.write(owner, &[0x19; 0x80]);
    t.set_u32(g(&t, 0x2f9), owner);
    // (step, frame before, model index, models, the list holding a node)
    let cases = [
        (1u32, 0u32, 0u32, 2u32, false),
        (0x2710, 3, 0, 2, false),
        (0xd, 0x4f, 0, 2, false),
        (0xd, 0x50, 0, 2, false),
        (0xc, 0x13, 0, 2, false),
        (0xc, 0x14, 0, 2, false),
        (0xc, 0x14, 1, 2, false),
        (0xb, 0xe, 0, 2, false),
        (0xb, 0xf, 0, 2, false),
        (0xb, 0xf, 1, 2, true),
        (0, 0, 1, 2, true),
        (0xa, 0, 1, 2, false),
    ];
    for (k, (step, frame, i, models, listed)) in cases.into_iter().enumerate() {
        t.write(s, &[0x11; 0x40]);
        t.set_u32(s + 0x1c, (0.5f32 + k as f32).to_bits());
        t.set_u32(s + 0x20, (-1.25f32).to_bits());
        t.set_u32(task, node);
        t.set_u32(task + 4, s);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.set_u32(prev + 4, node);
        t.set_u32(next + 8, node);
        t.set_u32(s + 0x18, step);
        t.set_u32(s + 0x28, frame);
        t.set_u32(s + 0x24, i);
        t.set_u32(n_models, models);
        // the node list: empty (the tail the head) or holding a node
        let head = g(&t, 0x188);
        let tail = if listed { SCRATCH + 0x5000 } else { head };
        t.write(SCRATCH + 0x5000, &[0x2b; 0x20]);
        t.set_u32(g(&t, 0x16e), tail);
        // the parents: their last child themselves
        for op in [0x6bu32, 0x9f, 0x2d7] {
            let p = g(&t, op);
            t.write(p, &[0x77; 0x40]);
            t.set_u32(p + 0x10, p);
        }
        // what the step reads and sets, distinct words: the targets, the
        // model table, the nodes N and M, the parts, the effect nodes
        for (op, n, seed) in [
            (0x22cu32, 0x98 * 2 / 4, 1u32),
            (0x1ca, 0x98 * 2 / 4, 2),
            (0x14b, 0x2c * 2 / 4, 3),
            (0x151, 0x2c * 2 / 4, 4),
            (0x133, 0x14c * 2 / 4, 5),
            (0x352, 56 * 2 / 4, 6),
        ] {
            let a = g(&t, op);
            fill(&mut t, a, n, seed + k as u32);
        }
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}
