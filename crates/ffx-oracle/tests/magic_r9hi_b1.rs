//! Round 9 (own rows from `magic_0450` up), batch 1: `magic_0481`'s
//! per-frame code against its Rust port (`ffx_magic::m0481`). Needs
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

/// The cue mask slot 261 answers from (bit `k` for cue `k`).
const CUES: u32 = SCRATCH + 0x10;
/// Slot 253's answer (0 pauses the effect).
const S253: u32 = SCRATCH + 0x14;
/// Slot 712's answer.
const S712: u32 = SCRATCH + 0x18;
/// Slot 941's table (`+4` the shapes).
const P941: u32 = SCRATCH + 0x2000;
const SHAPES: u32 = SCRATCH + 0x3000;
const SPR: u32 = SCRATCH + 0x1000;

/// The per-frame code's stubs: every slot it, its callees and their
/// callees reach, recording their words and writing their outputs.
fn burst_stubs(t: &mut Twin) {
    rec(t, 261, 1, |m, a| (m.r32(CUES) >> (a[0] & 31)) & 1);
    rec(t, 253, 0, |m, _| m.r32(S253));
    for (s, n) in [
        (711u32, 3usize),
        (710, 4),
        (723, 1),
        (724, 1),
        (267, 1),
        (271, 1),
        (713, 1),
        (714, 5),
        (268, 3),
        (269, 3),
        (214, 1),
        (258, 1),
        (231, 3),
        (263, 1),
        (237, 4),
        (265, 1),
    ] {
        rec(t, s, n, move |_, a| {
            (0x0abc_0000 | s) ^ a.first().copied().unwrap_or(0)
        });
    }
    rec(t, 230, 2, |m, a| {
        // a matrix product into the first: the second's words changed
        if a[0] >= SCRATCH && a[1] >= SCRATCH {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i);
                m.w32(a[0] + 4 * i, v.wrapping_add(i));
            }
        }
        0x230
    });
    rec(t, 712, 2, |m, _| m.r32(S712));
    rec(t, 941, 1, |_, _| P941);
    rec(t, 264, 1, |_, _| SPR);
    rec(t, 242, 2, |m, a| m.r32(a[1] + 0x14) + 0x40);
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, 0x3f00_0000 + i);
        }
        0
    });
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[2] + 4 * i) ^ m.r32(a[1]);
            m.w32(a[0] + 4 * i, v);
        }
        0
    });
    // a random float into the cell
    rec(t, 241, 1, |m, a| {
        let k = m.r32(SCRATCH + 0x20);
        m.w32(SCRATCH + 0x20, k + 1);
        let r = 0.125 + 0.0625 * (k % 7) as f32;
        m.w32(a[0], r.to_bits());
        0x241
    });
    // the path points: the float and its neighbours
    rec(t, 270, 5, |m, a| {
        let x = f32::from_bits(a[1]);
        for (i, v) in [x, x + 1.5, -x * 2.0, 1.0].into_iter().enumerate() {
            m.w32(a[0] + 4 * i as u32, v.to_bits());
        }
        0x270
    });
    // the direction between two points
    rec(t, 234, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i) ^ (0x10 << i);
            m.w32(a[0] + 4 * i, v);
        }
        0x234
    });
    // "normalise": the source's bits changed into the destination
    rec(t, 232, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ (0x0101_0000 << i);
            m.w32(a[0] + 4 * i, v);
        }
        0
    });
    rec(t, 266, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i).rotate_left(3) ^ m.r32(a[2] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0
    });
}

/// 0x10001920: the per-frame code through every stage, first and later
/// frames, paused and not, the counters at their limits.
#[test]
fn burst_frame() {
    let Some(mut t) = magic_l3::twin(481) else {
        return;
    };
    let f = t.at(0x1920);
    let g = |t: &Twin, op: u32| t.img.word(f + op);
    burst_stubs(&mut t);
    for i in 0..16 {
        t.set_u32(P941 + 4 * i, 0x0941_0000 + 0x10 * i);
    }
    t.set_u32(P941 + 4, SHAPES);
    for s in 0..8u32 {
        t.set_u16(SHAPES + 0x10 + 8 * s, (s * 0x40) as u16);
    }
    fill(&mut t, SHAPES + 0x100, 0x80, 7);
    fill(&mut t, SPR, 0x30, 0x55);
    // the globals the calls pass, distinct
    for op in [0x59u32, 0x8e, 0xad, 0xbb, 0x134, 0x60a, 0xf69, 0xa21, 0xa2c] {
        let a = g(&t, op);
        t.set_u32(a, 0x00a5_0000 | op);
    }
    // the shade's count, the camera vector the stage moves
    let v = g(&t, 0x63b);
    t.set_u32(v, 2.5f32.to_bits());
    // the eight parts' flags: some live
    let parts = g(&t, 0xb73);
    let set_parts = |t: &mut Twin, live: u32| {
        for i in 0..8 {
            t.set_u16(parts + 0x80 * i + 0x1e, u16::from(live >> i & 1 == 1));
            // the halfword after the flag: a wider access shows
            t.set_u16(parts + 0x80 * i + 0x20, 0x5a5a);
        }
    };
    let on = g(&t, 0x12);
    let (ppp, lock) = (t.u32(g(&t, 0x106)), t.u32(g(&t, 0x28)));
    // the slots that run after `*G_PPP = ENV` or under the lock record
    // those words and leave them changed
    for (s, n) in [
        (713u32, 1usize),
        (941, 1),
        (714, 5),
        (269, 3),
        (711, 3),
        (710, 4),
    ] {
        rec(&mut t, s, n, move |m, a| {
            let k = m.r32(SCRATCH + 0x40);
            m.w32(SCRATCH + 0x40, k + 1);
            m.w32(SCRATCH + 0x8000 + 8 * (k % 0x200), m.r32(ppp));
            m.w32(SCRATCH + 0x8004 + 8 * (k % 0x200), m.r32(lock));
            m.w32(ppp, 0x0bad_1000 + k);
            if s == 941 {
                P941
            } else {
                (0x0abc_0000 | s) ^ a[0]
            }
        });
    }
    let frame = g(&t, 0x1f);
    // (cue mask, slot 253, slot 712, settings (operand, value))
    let cases: Vec<(u32, u32, u32, Vec<(u32, u32)>)> = vec![
        (1 << 10, 1, 0, vec![(0x12, 0)]),
        (1 << 10, 1, 0, vec![(0x1f, 0), (0xe8, 0), (0x197, 0x59)]),
        (1 << 10, 1, 0, vec![(0x197, 0x5a)]),
        (1 << 10 | 1 << 15, 0, 0, vec![(0x197, 0x60)]),
        (1 << 11, 1, 0, vec![(0x1e6, 0)]),
        (1 << 11 | 1 << 7, 0, 0, vec![]),
        (1 << 8 | 1 << 7, 1, 0, vec![]),
        (1 << 7, 1, 0, vec![(0x2f7, 0)]),
        (1 << 7, 0, 0, vec![]),
        (1 << 7 | 1 << 15, 1, 0, vec![]),
        (1 << 6, 1, 1, vec![(0x4a3, 0), (0x5dd, 0)]),
        (1 << 6, 1, 0, vec![(0x5dd, 0x21)]),
        (1 << 6, 0, 1, vec![(0x5dd, 0x22)]),
        (1 << 6, 1, 0, vec![(0x5dd, 0xffff_fff0)]),
        (1 << 5, 1, 0, vec![(0x6ce, 0)]),
        (1 << 5, 0, 0, vec![]),
        (1 << 9 | 1 << 5, 1, 0, vec![]),
        (1 << 4, 1, 0, vec![(0x7a6, 0x2f), (0x8a1, 0)]),
        (1 << 4, 1, 0, vec![(0x7a6, 0x30), (0x7b3, 0), (0x852, 0x18)]),
        (1 << 4, 1, 0, vec![(0x852, 0x19)]),
        (1 << 4, 0, 0, vec![(0x852, 0x30)]),
        (1 << 3, 1, 0, vec![(0x8a1, 0), (0x969, 3)]),
        (1 << 3, 1, 1, vec![]),
        (1 << 3, 1, 0, vec![]),
        (1 << 3, 0, 0, vec![(0x95a, 1)]),
        (1 << 2, 1, 0, vec![(0xa9f, 0), (0xc85, 0)]),
        (1 << 2, 1, 0, vec![(0xa9f, 1), (0xaac, 0), (0xb2a, 0xdb)]),
        (1 << 2, 1, 0, vec![(0xb2a, 0xdc), (0xb62, 0)]),
        (1 << 2, 1, 0, vec![(0xb2a, 0xdd)]),
        (1 << 2, 1, 0, vec![(0xb2a, 0xe1), (0xb62, 7)]),
        (1 << 2, 0, 0, vec![(0xb2a, 0xe6)]),
        (1 << 1, 1, 0, vec![(0xc85, 0), (0xd55, 5)]),
        (1 << 1 | 1 << 2, 1, 1, vec![(0xa9f, 0)]),
        (1 << 1, 0, 0, vec![(0xd46, 1)]),
        (1 << 1, 1, 0, vec![(0xd46, 0)]),
        (1, 1, 0, vec![(0xeaf, 0), (0xf1e, 0xa9)]),
        (1, 1, 0, vec![(0xf1e, 0xaa)]),
        (1 | 1 << 15, 0, 0, vec![(0xf1e, 0xb0)]),
        (0, 1, 0, vec![]),
        (1 << 15, 1, 0, vec![]),
    ];
    for (k, (cues, s253, s712, sets)) in cases.into_iter().enumerate() {
        t.set_u32(on, 1);
        t.set_u32(frame, 3);
        t.set_u32(CUES, cues);
        t.set_u32(S253, s253);
        t.set_u32(S712, s712);
        set_parts(&mut t, 0xa5 ^ k as u32);
        // the halfwords after the u16 flags, the environment and lock
        // words the slots record: old values
        for op in [0x185u32, 0x28e, 0x94c, 0xd3d] {
            let a = g(&t, op);
            t.set_u16(a + 2, 0x5a5a);
        }
        t.set_u32(ppp, 0x0bad_0000 | k as u32);
        t.set_u32(lock, 0x4444_0000 | k as u32);
        for (op, v) in sets {
            let a = g(&t, op);
            if matches!(op, 0x95a | 0xd46) {
                t.set_u16(a, v as u16);
            } else {
                t.set_u32(a, v);
            }
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// A free ring of `n` pool nodes at `at` for the take whose code is at
/// `take` (its head and count operands).
fn free_ring(t: &mut Twin, take: u32, at: u32, n: u32) {
    let node = |i: u32| at + 0x40 * i;
    for i in 1..=n {
        t.write(node(i), &[0x5a; 0x1c]);
        t.set_u32(node(i), node(if i == 1 { n } else { i - 1 }));
        t.set_u32(node(i) + 4, node(if i == n { 1 } else { i + 1 }));
    }
    let head = t.img.word(take + 0x17);
    t.set_u32(head, node(1));
    let count = t.img.word(take + 0x04);
    t.set_u16(count, n as u16);
}

/// The two-node start's set-up (`o6_fam_pairs3::two_node_start` at
/// `start`): its part set-up's lock at a scratch word, a free pool, the
/// list empty (`busy` false) or holding a node.
fn two_node_setup(t: &mut Twin, start: u32, busy: bool) {
    let part_init = t.img.rel32_target(start + 0x3f);
    let lock = SCRATCH + 0x800;
    t.set_u32(t.img.word(part_init + 0x01), lock);
    t.set_u32(lock, 0x5555);
    let take = t.img.rel32_target(start + 0x7f);
    free_ring(t, take, SCRATCH + 0x1000, 4);
    let node = SCRATCH + 0x3400;
    t.write(node, &[0x33; 0x20]);
    t.set_u32(t.img.word(start + 0x85), if busy { node } else { 0 });
    t.set_u32(t.img.word(start + 0x9c), node);
}

/// `m0638::cue_start` (0x10003070 in `magic_0638` and `magic_0689`):
/// slot 225 says wait or go, the once-flag clear and set, slot 227 below,
/// at and above 1 (the two nodes started from a free pool).
#[test]
fn cue_start() {
    for n in [638u16, 689] {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(0x3070);
        let g = |t: &Twin, op: u32| t.img.word(f + op);
        let start = t.img.rel32_target(f + 0x5b);
        rec(&mut t, 225, 1, |m, _| m.r32(SCRATCH + 0x10));
        rec(&mut t, 226, 2, |_, a| 0x226 ^ a[0]);
        rec(&mut t, 227, 1, |m, _| m.r32(SCRATCH + 0x14));
        rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[1]);
        let h = g(&t, 0x26);
        t.set_u32(h, 0x00a5_0026);
        for (k, (go, once, r227)) in [
            (1u32, 0u32, 1u32),
            (0, 0, 0),
            (0, 1, 1),
            (0, 0, 2),
            (0, 1, 0xffff_ffff),
        ]
        .into_iter()
        .enumerate()
        {
            two_node_setup(&mut t, start, k % 2 == 1);
            t.set_u32(SCRATCH + 0x10, go);
            t.set_u32(SCRATCH + 0x14, r227);
            t.set_u32(g(&t, 0x18), once);
            for op in [0x61u32, 0x6b] {
                t.set_u32(g(&t, op), 0x7777_0000 | op);
            }
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// The object start's set-up (`s4_misc::start_object_plain` at `s`):
/// the lock and the part block at scratch words, slot 941's table and
/// slot 714 recording, the pool's eight nodes free and `count` of them
/// counted, the list's tail `tail`.
fn object_setup(t: &mut Twin, s: u32, count: u16, tail: u32) {
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
    t.set_u32(t.img.word(s + 0xa1), tail);
}

/// `m0689::start` (0x10003230): the object started with a free pool and
/// a tail, and with the pool empty; its fields hold old values.
#[test]
fn start_0689() {
    let Some(mut t) = magic_l3::twin(689) else {
        return;
    };
    let f = t.at(0x3230);
    let s = t.img.rel32_target(f + 0x1b);
    let obj = SCRATCH + 0x5000;
    t.set_u32(t.img.word(f + 0xb), obj);
    t.set_u32(t.img.word(f + 0x16), 3);
    for (count, tail) in [(3u16, 0u32), (2, SCRATCH + 0x600), (0, 0)] {
        object_setup(&mut t, s, count, tail);
        t.write(obj, &[0x5a; 0x130]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `m0591::cue_start` (0x100044e0): each of the three waits, the once
/// flag clear and set, slot 227 below and at 1 (the targets and tasks set
/// up: two target bytes moved, a free ring of four nodes, the list empty
/// or not).
#[test]
fn cue_start_0591() {
    let Some(mut t) = magic_l3::twin(591) else {
        return;
    };
    let f = t.at(0x44e0);
    let g = |t: &Twin, op: u32| t.img.word(f + op);
    let s = t.img.rel32_target(f + 0x94);
    let gs = |t: &Twin, op: u32| t.img.word(s + op);
    rec(&mut t, 225, 1, |m, _| m.r32(SCRATCH + 0x10));
    rec(&mut t, 5, 1, |m, _| m.r32(SCRATCH + 0x14));
    rec(&mut t, 28, 2, |m, _| m.r32(SCRATCH + 0x18));
    rec(&mut t, 226, 2, |_, a| 0x226 ^ a[0]);
    rec(&mut t, 227, 1, |m, _| m.r32(SCRATCH + 0x1c));
    let data = SCRATCH + 0x800;
    rec(&mut t, 719, 3, move |_, _| data);
    for (sl, n) in [(711u32, 3usize), (710, 4), (271, 1), (333, 1), (12, 1)] {
        rec(&mut t, sl, n, move |_, a| (0x0abc_0000 | sl) ^ a[0]);
    }
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xf) * 0.5 - 2.0);
    t.stub_st0(12, |_, _| 0.75);
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3 {
            m.w32(a[1 + i], (1.5f32 + i as f32).to_bits());
        }
        0x262
    });
    let lock = SCRATCH + 0x7f0;
    t.set_u32(gs(&t, 0x66), lock);
    let face = t.img.rel32_target(s + 0xde);
    t.set_u32(t.img.word(face + 0xe), SCRATCH + 0x4000);
    let h = g(&t, 0x5f);
    t.set_u32(h, 0x00a5_005f);
    let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    for (k, (w225, w5, w28, once, r227)) in [
        (1u32, 0u32, 0u32, 0u32, 1u32),
        (0, 1, 0, 0, 1),
        (0, 0, 2, 0, 1),
        (0, 0, 0, 0, 0),
        (0, 0, 0, 1, 1),
        (0, 0, 0, 0, 3),
    ]
    .into_iter()
    .enumerate()
    {
        t.set_u32(SCRATCH + 0x10, w225);
        t.set_u32(SCRATCH + 0x14, w5);
        t.set_u32(SCRATCH + 0x18, w28);
        t.set_u32(SCRATCH + 0x1c, r227);
        t.set_u32(g(&t, 0x50), once);
        t.set_u32(gs(&t, 0x1), 2);
        t.set_u32(lock, 0x5a5a_0000 + k as u32);
        for i in 0..4u32 {
            t.write(node(i), &[0x40 + i as u8; 0x1c]);
            t.set_u32(node(i), node((i + 3) % 4));
            t.set_u32(node(i) + 4, node((i + 1) % 4));
        }
        t.set_u32(gs(&t, 0x134), node(0));
        t.set_u16(gs(&t, 0x118), 4);
        let last = if k % 2 == 1 { SCRATCH + 0x1400 } else { 0 };
        t.write(SCRATCH + 0x1400, &[0x66; 0x1c]);
        t.set_u32(gs(&t, 0x18f), last);
        t.set_u32(gs(&t, 0x1a4), last);
        for op in [0x9au32, 0xa4] {
            t.set_u32(g(&t, op), 0x7777_0000 | op);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `m0591::scale_up_a`/`_b` (0x10004a50, 0x10005110): the scale task
/// from a free ring, the list empty and holding a node.
#[test]
fn scale_up_0591() {
    let Some(mut t) = magic_l3::twin(591) else {
        return;
    };
    let block = SCRATCH + 0x8000;
    for rva in [0x4a50u32, 0x5110] {
        let f = t.at(rva);
        t.set_u32(t.img.word(f + 0x13), block);
        let s = t.img.rel32_target(f + 0x1f);
        let take = t.img.rel32_target(s + 0x7b);
        for listed in [false, true] {
            free_ring(&mut t, take, SCRATCH + 0x4000, 3);
            t.write(block + 0x1a40, &[0x44; 0x60]);
            if listed {
                t.write(SCRATCH + 0x5000, &[0x22; 0x1c]);
                t.set_u32(t.img.word(s + 0x81), SCRATCH + 0x5000);
                t.set_u32(t.img.word(s + 0x99), SCRATCH + 0x5000);
            } else {
                t.set_u32(t.img.word(s + 0x81), 0);
            }
            t.check_entry(f, Call::this(0x0bad_0ecc), &[], Eax::Checked);
        }
    }
}

/// `m0528::step` (0x10001120): frames on and off each spark period
/// (negative, below 20, below 70, after), the flash frame 100, the fade
/// and its end at 120.
#[test]
fn step_0528() {
    let Some(mut t) = magic_l3::twin(528) else {
        return;
    };
    let f = t.at(0x1120);
    let a = SCRATCH + 0x600;
    t.set_u32(t.img.word(f + 0x4), a);
    t.set_u32(a + 0x198, 0x0198_0198);
    t.set_u32(a + 0x194, 0x0194_0194);
    rec(&mut t, 481, 1, |_, a| 0x0481_0000 ^ a[0]);
    for (s, n) in [(434u32, 2usize), (21, 3), (278, 3), (450, 2), (449, 2)] {
        rec(&mut t, s, n, move |_, a| (0x0abc_0000 | s) ^ a[0] ^ a[1]);
    }
    let fr = t.img.word(f + 0x1f);
    for n in [
        0xffff_fffdu32,
        0xffff_fff8,
        0,
        5,
        0x13,
        0x14,
        0x15,
        0x45,
        0x46,
        0x48,
        0x63,
        0x64,
        0x65,
        0x77,
        0x78,
        0x90,
    ] {
        t.set_u32(fr, n);
        t.check_entry(f, Call::default(), &[0x0a0a_0a0a + n], Eax::Checked);
    }
}

/// `m0528::spark` (0x100012a0) and `m0529::spark` (0x10001330): the
/// object with old values, the random direction normalised by slot 232.
#[test]
fn spark_0528() {
    // (dll, rva, the slot of the first call); `*G_A` is also slot 440's
    // first word
    for (n, rva, first) in [(528u16, 0x12a0u32, 441u32), (529, 0x1330, 442)] {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(rva);
        let a = SCRATCH + 0x600;
        t.set_u32(t.img.word(f + 0x11), a);
        t.set_u32(a + 0x198, 0x0198_0198);
        rec(&mut t, 232, 2, |m, a| {
            let v: Vec<f32> = (0..3)
                .map(|i| f32::from_bits(m.r32(a[1] + 4 * i)))
                .collect();
            let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            for (i, x) in (0u32..).zip(v) {
                m.w32(a[0] + 4 * i, (x / l).to_bits());
            }
            0x232
        });
        for (s, k) in [(first, 3usize), (440, 4), (450, 2), (449, 2)] {
            rec(&mut t, s, k, move |_, a| (0x0abc_0000 | s) ^ a[0]);
        }
        let obj = SCRATCH + 0x1000;
        for k in 0..3u8 {
            t.write(obj, &[0x5a ^ k; 0x140]);
            t.check_entry(f, Call::default(), &[obj], Eax::Checked);
        }
    }
}

/// The effect starts of `magic_0523` (0x10001520) and `magic_0536`
/// (0x10001340): the buffer, the part data with three program words, the
/// lock held around slot 711, the reset's block holding old bytes.
#[test]
fn effect_starts() {
    for (n, rva, block_op) in [(523u16, 0x1520u32, 0x08u32), (536, 0x1340, 0x08)] {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(rva);
        let d = SCRATCH + 0x600;
        for i in 0..8 {
            t.set_u32(d + 4 * i, 0x0d00_0000 + 0x11 * i);
        }
        rec(&mut t, 803, 2, |_, _| 0x2100_0000);
        rec(&mut t, 719, 3, move |_, _| d);
        rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
        rec(&mut t, 260, 1, |_, a| 0x0260_0000 ^ a[0]);
        let lock = SCRATCH + 0x7f0;
        t.set_u32(t.img.word(f + 0x83), lock);
        rec(&mut t, 711, 3, move |m, a| {
            m.w32(SCRATCH + 0x7f8, m.r32(lock));
            0x0711_0000 ^ a[0]
        });
        for (s, k) in [(718u32, 1usize), (222, 4), (710, 4), (38, 3)] {
            rec(&mut t, s, k, move |_, a| (0x0abc_0000 | s) ^ a[0]);
        }
        t.set_u32(t.img.word(f + 0x64), 0x00a5_0064);
        let reset = t.img.rel32_target(f + 0x128);
        let block = t.img.word(reset + block_op);
        for k in 0..2u32 {
            t.write(block, &[0x5a ^ k as u8; 0x110]);
            t.set_u32(lock, 0x4444 + k);
            t.check_entry(f, Call::default(), &[0x0a0a_0000 + k], Eax::Checked);
        }
    }
}

/// `m0526::aim_spark` (0x10001450): two bone indices, the points from
/// slots 456/457 and the direction normalised by slot 453 (a float in
/// ST0 the code drops); the objects hold old values.
#[test]
fn aim_spark_0526() {
    let Some(mut t) = magic_l3::twin(526) else {
        return;
    };
    let f = t.at(0x1450);
    let a = SCRATCH + 0x400;
    t.set_u32(t.img.word(f + 0x11), a);
    for i in 0..0x80 {
        t.set_u32(a + 0x190 + 4 * i, 0x0a00_0000 + i);
    }
    let new = SCRATCH + 0x2000;
    rec(&mut t, 434, 2, move |_, _| new);
    for (s, k) in [(444u32, 1usize), (440, 4), (450, 2), (451, 2)] {
        rec(&mut t, s, k, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    for (s, base) in [(456u32, 1.5f32), (457, -0.25)] {
        rec(&mut t, s, 3, move |m, a| {
            for i in 0..4 {
                let x = base + (a[0] & 0xf) as f32 + i as f32 * 0.75;
                m.w32(a[2] + 4 * i, x.to_bits());
            }
            s
        });
    }
    rec(&mut t, 453, 1, |m, a| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[0] + 4 * i));
            m.w32(a[0] + 4 * i, (v * 0.5).to_bits());
        }
        0x453
    });
    t.stub_st0(453, |_, _| 2.5);
    let obj = SCRATCH + 0x1000;
    for (k, idx) in [(0u32, 1u32), (1, 3)] {
        t.write(obj, &[0x5a ^ k as u8; 0x140]);
        t.write(new, &[0x3c ^ k as u8; 0x140]);
        t.set_u32(obj + 0xac, k);
        t.set_u32(obj + 16 * k + 0x2c, idx);
        t.set_u32(new + 0xac, 2 - k);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

/// `m0522::bone_spark` (0x10001480): enough calls for both bones (the
/// draws' parity varies), slot 456 writing the point from its bone.
#[test]
fn bone_spark_0522() {
    let Some(mut t) = magic_l3::twin(522) else {
        return;
    };
    let f = t.at(0x1480);
    let a = SCRATCH + 0x600;
    t.set_u32(t.img.word(f + 0x11), a);
    t.set_u32(a + 0x198, 0x0198_0198);
    rec(&mut t, 456, 3, |m, a| {
        for i in 0..4 {
            m.w32(a[2] + 4 * i, (a[1] as f32 * 0.5 + i as f32).to_bits());
        }
        0x456
    });
    for (s, k) in [(458u32, 3usize), (450, 2), (451, 2)] {
        rec(&mut t, s, k, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    let obj = SCRATCH + 0x1000;
    for k in 0..6u8 {
        t.write(obj, &[0x5a ^ k; 0x190]);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}
