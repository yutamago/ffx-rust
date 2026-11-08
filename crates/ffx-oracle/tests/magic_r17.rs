//! Round 17's ports (`ffx_magic::r17`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

mod magic;
mod magic_owna;
mod magic_r11;

use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use magic_owna::{rec, row, twin};

/// Junk in the stack below the entry (the frames' locals start as it).
fn scrub(t: &mut Twin) {
    let entry = magic::STACK_TOP - 0x100;
    let junk: Vec<u8> = (0..0x1000u32)
        .map(|i| (i.wrapping_mul(0x2b) ^ 0x5b).to_le_bytes()[0])
        .collect();
    t.write(entry - 0x1000, &junk);
}

/// Junk words at the globals the operands `ops` of `f` name.
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, 0x5a5a_5a5a ^ o);
    }
}

/// `n` bytes of junk at `at` (varied by `k`).
fn junk(t: &mut Twin, at: u32, n: u32, k: u32) {
    let b: Vec<u8> = (0..n)
        .map(|i| (i.wrapping_mul(0x3d) ^ (0x71 + k)).to_le_bytes()[0])
        .collect();
    t.write(at, &b);
}

/// Folds `v` into word `i` of `slot`'s dump row (the outputs a stub writes
/// to the stack, which the comparison skips).
fn fold(m: &mut dyn StubMem, slot: u32, i: u32, v: u32) {
    let at = SCRATCH + 0x2_0000 + 0x40 * (slot % 0x400) + 4 * i;
    let x = m.r32(at).rotate_left(5) ^ v;
    m.w32(at, x);
}

fn rf(m: &dyn StubMem, at: u32) -> f32 {
    f32::from_bits(m.r32(at))
}

fn wf(m: &mut dyn StubMem, at: u32, v: f32) {
    m.w32(at, v.to_bits());
}

/// The vector and matrix slots as float maths both sides share: 234
/// (`a - b`), 564 (`/ f`), 248 (`a + b`), 325 (copy), 235 (a matrix varied
/// by its call), 328/329 (one word of the matrix plus the angle), 229
/// (`M[i] * 0.5 + v[i]`).
fn vector_slots(t: &mut Twin) {
    rec(t, 234, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x234
    });
    rec(t, 564, 3, |m, a| {
        for i in 0..3 {
            let v = rf(m, a[1] + 4 * i) / f32::from_bits(a[2]);
            wf(m, a[0] + 4 * i, v);
        }
        0x564
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
            fold(m, 248, i, v.to_bits());
        }
        0x248
    });
    rec(t, 325, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x325
    });
    rec(t, 235, 1, |m, a| {
        let k = m.r32(row(235)) as f32;
        for i in 0..16u32 {
            let v = if i % 5 == 0 {
                1.0
            } else {
                0.125 * (i as f32) - k * 0.25
            };
            wf(m, a[0] + 4 * i, v);
        }
        0x235
    });
    for s in [328u32, 329] {
        rec(t, s, 3, move |m, a| {
            let k = m.r32(row(s)) % 16;
            let v = rf(m, a[1] + 4 * k) + f32::from_bits(a[2]);
            wf(m, a[0] + 4 * k, v);
            s
        });
    }
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
            fold(m, 229, i, v.to_bits());
        }
        0x229
    });
}

/// Float slots: 314 (`op_random`: the argument times a value varied by
/// the call) and 404 likewise, 251, 19, 17 and 12 (the ground `g`).
fn float_slots(t: &mut Twin, g: f64) {
    for s in [314u32, 404] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |m, a| {
            let k = f64::from(m.r32(row(s)) % 7);
            f64::from(f32::from_bits(a[0])) * (0.3 + 0.17 * k) - 0.4
        });
    }
    for (s, v) in [(251u32, 1.5f64), (19, 0.75), (17, 2.5)] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| v + f64::from(a[0] & 3));
    }
    rec(t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, move |_, _| g);
}

/// What `r17::debris_burst_0018` (at `f`) needs: the slots (592 gives
/// `n0`; 237 the source `p`, 268 the bone `x`), the object starts with a
/// bigger pool, the aim's free ring, the actors (selector byte `b`, flag
/// `flag` at `+0x3a4`), the records (junk), the task (`left`).
#[allow(clippy::too_many_arguments)]
fn burst_setup(
    t: &mut Twin,
    f: u32,
    k: u32,
    (left, n0, flag, b): (u32, i32, u32, u8),
    p: [f32; 3],
    x: [f32; 3],
    ground: f64,
) -> u32 {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    magic_r11::object_slots(t);
    vector_slots(t);
    float_slots(t, ground);
    let start = t.img.rel32_target(f + 0x2ea);
    magic_r11::object_start_setup(t, start, (0x84, 0x9d));
    let pa = t.img.rel32_target(start + 0x9d);
    t.set_u16(op(t, pa, 0x04), 0x40);
    let aim = t.img.rel32_target(f + 0x6db);
    let take = t.img.rel32_target(aim + 0x1e7);
    magic_r11::ring(
        t,
        op(t, take, 0x04),
        op(t, take, 0x17),
        32,
        SCRATCH + 0x1c000,
    );
    rec(t, 592, 1, move |_, _| n0.cast_unsigned());
    rec(t, 237, 4, move |m, a| {
        for i in 0..3 {
            wf(m, a[3] + 4 * i, p[i as usize]);
        }
        m.w32(a[3] + 0xc, 0x5a5a_5a5a);
        0x237
    });
    rec(t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], 0.5 * i as f32 - 3.0);
        }
        0x262
    });
    rec(t, 268, 3, move |m, a| {
        for i in 0..3 {
            wf(m, a[2] + 4 * i, x[i as usize]);
        }
        m.w32(a[2] + 0xc, 0x6b6b_6b6b);
        0x268
    });
    t.set_u32(op(t, f, 0x38), 1);
    t.set_u32(op(t, f, 0x78), 0x0000_6950);
    let sel = SCRATCH + 0x5000;
    t.write(sel, &[b, 1, 0, 3]);
    t.set_u32(op(t, f, 0x89), sel);
    for i in 0..0x40 {
        t.set_u32(op(t, f, 0x9b) + 4 * i, 0x0c0c_0100 + i);
    }
    t.set_u32(op(t, f, 0x3a4), flag);
    let recs = SCRATCH + 0x14000;
    junk(t, recs, 24 * 0x4f0, k);
    t.set_u32(op(t, f, 0x392), recs);
    junk(t, op(t, f, 0x2cf) - 0xd4, 3 * 0x130, k + 1);
    let task = SCRATCH + 0x700;
    junk(t, task, 0x40, k + 2);
    junk(t, SCRATCH + 0x780, 0x40, k + 3);
    t.set_u32(task + 0x14, left);
    t.set_u32(task + 8, SCRATCH + 0x780);
    task
}

/// `magic_0018` 0x10001340 and `magic_0019` 0x10003850
/// (`r17::debris_burst_0018`): the countdown, `n` below 0, 0, 1,234,
/// 3,000 and above 9,999 (no to three sparks, up to 24 pieces), each
/// radius path, the yaw wrapped both ways, a cost table that ends the
/// pieces at 24 with some left and one whose costs are all too high.
#[test]
fn debris_burst_0018() {
    for (dll, rva) in [(18u16, 0x1340u32), (19, 0x3850)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let cost = t.img.word(f + 0x452);
        let costs: Vec<u32> = (0..7).map(|i| t.u32(cost + 4 * i)).collect();
        let k_yaw = t.img.word(f + 0x183);
        let k0 = t.u32(k_yaw);
        // (left, n0, flag, byte), P, X, ground, cost table, yaw K
        let cases: [((u32, i32, u32, u8), [f32; 3], [f32; 3], f64, u32, u32); 8] = [
            ((3, 5000, 0, 3), [0.0; 3], [1.0; 3], 0.0, 0, k0),
            (
                (1, -5, 0, 3),
                [1.0, 2.0, 3.0],
                [4.0, -1.0, 0.5],
                -0.25,
                0,
                k0,
            ),
            (
                (1, 1234, 1, 3),
                [0.5, 0.0, -2.0],
                [-3.0, 1.5, 4.0],
                0.75,
                0,
                k0,
            ),
            (
                (0, 3000, 0, 0x20),
                [-1.0, 1.0, 1.0],
                [2.0, 3.0, -6.0],
                1.5,
                0,
                k0,
            ),
            (
                (1, 20000, 0, 2),
                [2.0, -1.0, 0.0],
                [-2.0, 0.5, -3.0],
                0.25,
                0,
                (-6.0f32).to_bits(),
            ),
            ((1, 0, 1, 3), [1.0, 1.0, 1.0], [1.0, 2.0, 1.5], 0.0, 0, k0),
            ((1, 30, 0, 4), [0.0, 0.5, 1.0], [3.0, 3.0, 3.0], 1.0, 1, k0),
            (
                (1, 5, 1, 5),
                [0.0, 0.5, 1.0],
                [-3.0, 3.0, 3.0],
                1.0,
                100,
                k0,
            ),
        ];
        for (k, (args, p, x, ground, table, kyaw)) in cases.into_iter().enumerate() {
            let task = burst_setup(&mut t, f, k as u32, args, p, x, ground);
            junk_ops(
                &mut t,
                f,
                &[
                    0x665, 0x66b, 0x671, 0x677, 0x67d, 0x687, 0x691, 0x697, 0x69d,
                ],
            );
            for i in 0..7 {
                let c = if table == 0 { costs[i as usize] } else { table };
                t.set_u32(cost + 4 * i, c);
            }
            t.set_u32(k_yaw, kyaw);
            let spins = t.u32(row(314));
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[task], Eax::Checked);
            // the loops ran (a struct pointer the test also set would hide them)
            let pieces = t.u32(row(314)) - spins;
            let want = match k {
                0 | 5 | 7 => 0,
                1 => 6,
                2 => 1 + 10 * 7 - 1,
                3 => 2 + 3 * 7 - 1,
                4 => 3 + 24 * 7 - 1,
                _ => 24 * 7 - 1,
            };
            assert_eq!(pieces, want, "magic_{dll:04} case {k}: slot 314 calls");
        }
    }
}

/// What the piece sparks need: the object start, the slots, the actors,
/// the piece `P` (`i`, its yaw `yaw`, its target) and the records.
fn spark_setup(t: &mut Twin, f: u32, start_at: u32, k: u32, i: u32, yaw: f32) -> u32 {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    magic_r11::object_slots(t);
    vector_slots(t);
    float_slots(t, 0.5 - f64::from(k));
    let start = t.img.rel32_target(f + start_at);
    magic_r11::object_start_setup(t, start, (0x84, 0x9d));
    let recs = SCRATCH + 0x14000;
    junk(t, recs, 4 * 0x4f0, k);
    t.set_u32(op(t, f, 0x27), recs);
    let piece = recs + 0x4f0 * i;
    t.set_u32(piece + 0x390, i);
    t.set_u32(piece + 0xc4, yaw.to_bits());
    for e in 0..4 {
        t.set_u32(
            piece + 0x358 + 4 * e,
            (0.5 * e as f32 - 1.0 + k as f32).to_bits(),
        );
    }
    t.set_u32(piece + 0x368, (1.25 + k as f32).to_bits());
    t.set_u32(piece + 0x370, (-0.75 * k as f32).to_bits());
    t.set_u32(piece + 0x4e0, (0.01 * (k + 1) as f32).to_bits());
    piece
}

/// `magic_0018` 0x10001bc0 (`r17::piece_spark_0018`) and `magic_0019`
/// 0x100040d0 (`r17::piece_spark_0019`): the first piece (shared record
/// and code pointer) and later ones, the yaw wrapped down, up and kept.
#[test]
fn piece_sparks() {
    for (dll, rva, start_at, b, sel) in [
        (
            18u16,
            0x1bc0u32,
            0x52u32,
            0x127u32,
            Some((0x285u32, 0x28fu32)),
        ),
        (19, 0x40d0, 0x4f, 0x132, None),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let rec_at = SCRATCH + 0x1d000;
        for (k, (i, yaw)) in [(0u32, 0.5f32), (1, 3.0), (2, -6.0), (0, -1.0), (3, 7.0)]
            .into_iter()
            .enumerate()
        {
            let k = k as u32;
            let piece = spark_setup(&mut t, f, start_at, k, i, yaw);
            let ops: &[u32] = if dll == 18 {
                &[0xc1, 0xcb, 0xd5, 0xdf, 0xe9, 0xf3, 0xfd, 0x107, 0x111]
            } else {
                &[0xbe, 0xc8, 0xd2, 0xdc, 0xe6, 0xf0, 0xfa, 0x104, 0x10e]
            };
            junk_ops(&mut t, f, ops);
            junk(&mut t, rec_at, 0x200, k + 5);
            t.set_u32(t.img.word(f + b), rec_at);
            if let Some((s, chrs)) = sel {
                let at = SCRATCH + 0x5000;
                t.write(at, &[2 + k as u8, 1, 0, 3]);
                t.set_u32(t.img.word(f + s), at);
                for j in 0..8 {
                    t.set_u32(t.img.word(f + chrs) + 4 * j, 0x0c0c_0100 + j);
                }
            }
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[piece], Eax::Checked);
        }
    }
}

/// The `magic_0168` family's functions in each DLL: the matrices, the
/// start-up, the load step, the trail, the ribbons' step, their set-up.
const F168: [(u16, [u32; 6]); 2] = [
    (168, [0x1620, 0x4840, 0x1360, 0x1ee0, 0x2610, 0x3930]),
    (599, [0x6010, 0x80d0, 0x5d50, 0x68d0, 0x6d60, 0x71c0]),
];
/// The effect block `*G_E` of the 0168 family (0x3200 bytes).
const E168: u32 = SCRATCH + 0x8000;
/// Slot 333's ST0 and slot 520's answer.
const YAW_CELL: u32 = SCRATCH + 0x90;
const VERSION_CELL: u32 = SCRATCH + 0x94;

/// The slots the 0168 family calls (beyond [`vector_slots`] and
/// [`float_slots`]).
fn slots_168(t: &mut Twin, k: u32) {
    magic_r11::object_slots(t);
    vector_slots(t);
    float_slots(t, 0.25 + f64::from(k));
    rec(t, 262, 4, move |m, a| {
        for i in 1..4 {
            wf(m, a[i], 0.75 * i as f32 - k as f32);
        }
        0x262
    });
    for s in [210u32, 211] {
        rec(t, s, 2, move |m, a| {
            for i in 0..16 {
                let v = (a[0] & 0xff) as f32 * 0.01 + i as f32 * 0.1 + s as f32 * 0.001;
                wf(m, a[1] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 330, 3, |m, a| {
        let k = m.r32(row(330)) % 16;
        let v = rf(m, a[1] + 4 * k) + f32::from_bits(a[2]);
        wf(m, a[0] + 4 * k, v);
        330
    });
    rec(t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, |m, _| f64::from(rf(m, YAW_CELL)));
    for s in [277u32, 15, 16] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| {
            0.5 + f64::from(a[0] & 7) * 0.25 + f64::from(s) * 0.01
        });
    }
    for (s, n) in [
        (238u32, 2usize),
        (296, 2),
        (257, 2),
        (217, 2),
        (255, 2),
        (357, 3),
        (356, 3),
        (718, 1),
        (711, 3),
        (710, 7),
        (271, 1),
        (371, 2),
        (287, 3),
        (283, 2),
        (281, 1),
        (713, 1),
        (775, 2),
        (776, 1),
        (777, 1),
    ] {
        rec(t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    for (s, n) in [(774u32, 5usize), (778, 6)] {
        rec(t, s, n, move |m, a| {
            let mat = m.r32(a[3] + 0x10);
            for i in 0..12 {
                let v = m.r32(mat + 4 * i);
                fold(m, s, i, v);
            }
            s << 16
        });
    }
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(t, 520, 0, |m, _| m.r32(VERSION_CELL));
    rec(t, 772, 2, |_, a| a[0] ^ 0x772);
    t.stub_edx(772, |_, a| a[1] ^ 0x0077_2000);
    rec(t, 773, 4, |m, _| m.r32(row(773)) & 1);
    for s in [723u32, 724] {
        rec(t, s, 1, move |m, a| {
            for i in 0..16 {
                wf(m, a[0] + 4 * i, 0.25 * i as f32 - s as f32 * 0.001);
            }
            s
        });
    }
    rec(t, 237, 4, |m, a| {
        let k = m.r32(row(237)) as f32;
        for i in 0..4 {
            wf(m, a[3] + 4 * i, 0.5 * i as f32 + k * 0.75 - 1.0);
        }
        0x237
    });
}

/// What `r17::mats_0168` (at `f`) needs: `*G_E`, the first actor
/// (selector byte `b`), the flag `*G_F`, the second actor and index.
fn mats_setup(t: &mut Twin, f: u32, k: u32, (flag, idx, b): (u32, u32, u8), yaw: f32) {
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    slots_168(t, k);
    t.set_u32(YAW_CELL, yaw.to_bits());
    junk(t, E168, 0x3200, k);
    t.set_u32(op(t, 0x1b), E168);
    let sel = SCRATCH + 0x5000;
    t.write(sel, &[b, 1, 0, 3]);
    t.set_u32(op(t, 0x11), sel);
    for i in 0..0x40 {
        t.set_u32(op(t, 0x2e) + 4 * i, 0x0c0c_0100 + i);
    }
    t.set_u32(op(t, 0x167), flag);
    t.set_u32(op(t, 0x33c), 0x0b0b_0000 + k);
    t.set_u32(op(t, 0x36c), idx);
}

/// `magic_0168` 0x10001620 and `magic_0599` 0x10006010
/// (`r17::mats_0168`): slot 296 or 257 by the flag and the index, the
/// yaw wrapped down, kept and up.
#[test]
fn mats_0168() {
    for (dll, fs) in F168 {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(fs[0]);
        for (k, (args, yaw)) in [
            ((0u32, 3u32, 2u8), -1.0f32),
            ((1, 3, 5), 6.0),
            ((0, 0x40, 7), 9.0),
            ((0, 0xffff_fff0, 1), 2.0),
        ]
        .into_iter()
        .enumerate()
        {
            mats_setup(&mut t, f, k as u32, args, yaw);
            scrub(&mut t);
            t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
                ffx_magic::Ret::eax(ffx_magic::r17::mats_0168(g, img, fr.at, fr.esp))
            });
        }
    }
}

/// `magic_0168` 0x10001360 and `magic_0599` 0x10005d50
/// (`r9_load::load_0168`, the start-up `r17::start_up_0168`): busy, not
/// loaded, then the start-up with each version fix-up and the task list
/// empty or not.
#[test]
fn load_0168() {
    for (dll, fs) in F168 {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(fs[2]);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let start = t.img.rel32_target(f + 0x5b);
        let mats = t.img.rel32_target(start + 0x6f);
        let part = t.img.rel32_target(start + 0x6a);
        let (busy, loaded) = (SCRATCH + 0x100, SCRATCH + 0x108);
        for (k, (b, asked, l, version, listed)) in [
            (1u32, 0u32, 1u32, 0u32, false),
            (0, 0, 0, 0, false),
            (0, 1, 1, 0x0209_0000, false),
            (0, 0, 2, 0x0209_0046, true),
            (0, 1, 1, 0x0205_0000, true),
            (0, 1, 1, 0x00dc_5555, false),
            (0, 1, 1, 0x00dd_0000, true),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            mats_setup(&mut t, mats, k, (k & 1, 3, 2), 1.0);
            rec(&mut t, 225, 1, move |m, _| m.r32(busy));
            rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
            rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
            t.set_u32(busy, b);
            t.set_u32(loaded, l);
            t.set_u32(w(&t, 0x18), asked);
            t.set_u32(w(&t, 0x26), 0x0000_5e5e);
            for o in [0x61u32, 0x6b] {
                t.set_u32(w(&t, o), 0x0bad_0000 + o);
            }
            t.set_u32(VERSION_CELL, version);
            magic_r11::pools(&mut t, start, 612);
            let lock = SCRATCH + 0x98;
            t.set_u32(lock, 0x7777);
            t.set_u32(t.img.word(part + 0x04), lock);
            let list = SCRATCH + 0x600;
            junk(&mut t, list, 0x20, k);
            let op = |t: &Twin, o: u32| t.img.word(start + o);
            t.set_u32(op(&t, 0x138), if listed { list } else { 0 });
            t.set_u32(op(&t, 0x14f), list);
            junk_ops(
                &mut t,
                start,
                &[
                    0x2a, 0x34, 0x3e, 0x48, 0x52, 0xd9, 0xe3, 0xee, 0xf6, 0xfd, 0x107, 0x112, 0x11a,
                ],
            );
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// A ribbon at `r` for the trail: the animation cursor (not running),
/// the 128 points (every fifth one repeated: segments of length 0), the
/// head `h`, `n` pieces `step` apart, the flags of `flags` points before
/// the head (wrapping) and colours with the alpha `alpha`s.
fn ribbon(t: &mut Twin, r: u32, k: u32, (h, n, step, flags): (u32, i32, f32, u32), alpha: [u8; 2]) {
    junk(t, r, 0x8a8, k);
    t.set_u32(r, 0x1234 + k);
    t.set_u32(r + 4, 0);
    t.set_u16(r + 8, 1);
    for j in 0..128u32 {
        let s = if j % 5 == 0 && j > 0 { j - 1 } else { j } as f32;
        for (e, v) in [s * 0.5 - k as f32, (s * 0.37).sin(), s * -0.25, 1.0]
            .into_iter()
            .enumerate()
        {
            t.set_u32(r + 0xc + 16 * j + 4 * e as u32, v.to_bits());
        }
    }
    t.write(r + 0x828, &[0; 0x80]);
    for j in 1..=flags {
        t.set_u8(r + 0x828 + (h + 128 - j) % 128, 1);
    }
    t.set_u32(r + 0x820, h);
    t.set_u32(r + 0x824, n.cast_unsigned());
    t.set_u32(r + 0x81c, step.to_bits());
    t.set_u32(r + 0x814, 1.5f32.to_bits());
    t.set_u32(r + 0x818, 0.25f32.to_bits());
    t.write(
        r + 0x80c,
        &[0x10, 0x80, 0xf0, alpha[0], 0xff, 0x20, 0x40, alpha[1]],
    );
}

/// `magic_0168` 0x10001ee0 and `magic_0599` 0x100068d0
/// (`r17::trail_0168`): no pieces, a short and a long trail (stopping at
/// a point never written, after 127 segments, on segments of length 0),
/// pieces drawn and not (alpha 0), the head wrapping.
#[test]
fn trail_0168() {
    for (dll, fs) in F168 {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(fs[3]);
        let (r, m, v) = (SCRATCH + 0xb400, SCRATCH + 0x600, SCRATCH + 0x680);
        for (k, (shape, alpha)) in [
            ((10u32, 0i32, 0.5f32, 5u32), [0x80u8, 0x40u8]),
            ((10, 20, 0.7, 8), [0x80, 0x40]),
            ((127, 60, 0.3, 128), [0, 0]),
            ((3, 200, 0.05, 128), [0x10, 0]),
            ((64, 40, 1.25, 30), [0, 0x90]),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            slots_168(&mut t, k);
            ribbon(&mut t, r, k, shape, alpha);
            for i in 0..16 {
                t.set_u32(
                    m + 4 * i,
                    (0.5 + 0.125 * i as f32 - k as f32 * 0.25).to_bits(),
                );
            }
            for (i, x) in [1.0f32, -2.0, 0.5, 1.0].into_iter().enumerate() {
                t.set_u32(v + 4 * i as u32, (x + k as f32).to_bits());
            }
            t.set_u32(t.img.word(f + 0x435), 2 + k);
            junk_ops(&mut t, f, &[0x5f, 0x70, 0x20b, 0x233, 0x25b, 0x286]);
            let draws = t.u32(row(229));
            let c = Call {
                ecx: r,
                edx: m,
                ..Call::default()
            };
            scrub(&mut t);
            t.check(f, c, &[v], Eax::Checked, ffx_magic::r17::trail_0168);
            if shape.1 > 0 {
                assert!(
                    t.u32(row(229)) > draws,
                    "magic_{dll:04} case {k}: no piece drawn"
                );
            }
        }
    }
}

/// `magic_0168` 0x10002610 and `magic_0599` 0x10006d60
/// (`r17::ribbons_0168`): the four ribbons with their fade bytes (0, 1,
/// 2, 0x18) counted down or kept.
#[test]
fn ribbons_0168() {
    for (dll, fs) in F168 {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(fs[4]);
        for (k, keep) in [0u32, 1, 0].into_iter().enumerate() {
            let k = k as u32;
            slots_168(&mut t, k);
            junk(&mut t, E168, 0x3200, k);
            t.set_u32(t.img.word(f + 0x17), E168);
            for (i, fade) in [0u8, 1, 2, 0x18].into_iter().enumerate() {
                let r = E168 + 0xea8 + 0x8a8 * i as u32;
                ribbon(
                    &mut t,
                    r,
                    k + i as u32,
                    (5 + 9 * i as u32, 6 + k.cast_signed(), 0.4, 12),
                    [0x80, 0x40],
                );
                t.set_u8(r + 0x80f, fade);
            }
            t.set_u32(E168 + 0x315c, keep);
            t.set_u32(t.img.word(f + 0x6d), 0x0b0b_0000 + k);
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[SCRATCH + 0x700], Eax::Checked);
        }
    }
}

/// `magic_0168` 0x10003930 and `magic_0599` 0x100071c0
/// (`r17::ribbons_setup_0168`): the model on the list (first, last) or
/// not, the free ring with one or three nodes, the task list empty or
/// not, the yaw wrapped or kept.
#[test]
fn ribbons_setup_0168() {
    for (dll, fs) in F168 {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(fs[5]);
        let op = |t: &Twin, o: u32| t.img.word(f + o);
        for (k, (on_list, free, listed, yaw)) in [
            (0u32, 1u16, false, 0.5f32),
            (1, 3, true, -6.0),
            (2, 2, false, 4.0),
            (1, 1, true, 1.0),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            slots_168(&mut t, k);
            for at in [0x116u32, 0x198] {
                let start = t.img.rel32_target(f + at);
                magic_r11::object_start_setup(&mut t, start, (0x84, 0x99));
            }
            junk(&mut t, E168, 0x3200, k);
            t.set_u32(op(&t, 0xe), E168);
            t.set_u16(E168 + 0x130 + 0x124, 0xffff);
            for (i, x) in [1.5f32, 2.5, -3.5, 1.0].into_iter().enumerate() {
                t.set_u32(E168 + 0xac0 + 4 * i as u32, x.to_bits());
            }
            t.set_u32(op(&t, 0x266), 0x0b0b_0000 + k);
            t.set_u32(E168 + 0x3150, yaw.to_bits());
            let d = SCRATCH + 0x640;
            t.set_u32(d + 0x20, 0x0d0d_0000 + k);
            t.set_u32(op(&t, 0x7), d);
            let p = SCRATCH + 0x660;
            t.set_u32(op(&t, 0xd7), p);
            // the list: two nodes, the model in the first (1), the last
            // (2) or neither (0)
            let model = E168 + 0x130;
            let (list, n1, n2) = (op(&t, 0xb7), SCRATCH + 0x6a0, SCRATCH + 0x6c0);
            for (n, prev, next, item) in [
                (n1, 0, n2, if on_list == 1 { model } else { 0x5151 }),
                (n2, n1, 0, if on_list == 2 { model } else { 0x5252 }),
            ] {
                t.set_u32(n, prev);
                t.set_u32(n + 4, next);
                t.set_u32(n + 8, item);
            }
            for (i, x) in [n1, n2, n1, 2].into_iter().enumerate() {
                t.set_u32(list + 4 * i as u32, x);
            }
            let (count, head) = (op(&t, 0x3e4), op(&t, 0x4b2));
            magic_r11::ring(&mut t, count, head, free, SCRATCH + 0x1c000);
            let tl = SCRATCH + 0x740;
            junk(&mut t, tl, 0x20, k);
            t.set_u32(op(&t, 0x519), if listed { tl } else { 0 });
            t.set_u32(op(&t, 0x530), tl);
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `E168` with junk, then real floats at the summon's words (`E + 0xa80`
/// to `+0xb00`, `+0x3150`, `+0x6c0`/`+0x6c8`) and the model's halfword.
fn e168(t: &mut Twin, k: u32) {
    junk(t, E168, 0x3200, k);
    for i in 0..0x20u32 {
        let v = 0.375 * i as f32 - 4.0 + 0.5 * k as f32;
        t.set_u32(E168 + 0xa80 + 4 * i, v.to_bits());
    }
    for (o, v) in [(0x3150u32, 0.75f32), (0x6c0, 3.0), (0x6c8, -2.0)] {
        t.set_u32(E168 + o, (v + k as f32).to_bits());
    }
    t.set_u16(E168 + 0x664, 0x40 + k as u16);
}

/// The task list (operands `head`, `tail` of `f`): empty or one node.
fn task_list(t: &mut Twin, f: u32, (head, tail): (u32, u32), listed: bool, k: u32) {
    let tl = SCRATCH + 0x740;
    junk(t, tl, 0x20, k);
    let h = t.img.word(f + head);
    t.set_u32(h, if listed { tl } else { 0 });
    let l = t.img.word(f + tail);
    t.set_u32(l, tl);
}

/// `magic_0168` 0x100023a0 (`r17::lights_0168`): the light records and
/// their task nodes, the task list empty or not.
#[test]
fn lights_0168() {
    let Some(mut t) = twin(168) else { return };
    let f = t.at(0x23a0);
    for (k, listed) in [false, true].into_iter().enumerate() {
        let k = k as u32;
        slots_168(&mut t, k);
        e168(&mut t, k);
        for o in [0x2du32, 0xbc, 0x13a] {
            t.set_u32(t.img.word(f + o), E168);
        }
        magic_r11::pools(&mut t, f, 422);
        task_list(&mut t, f, (0x8b, 0xa1), listed, k);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0168` 0x100026f0 (`r17::summon_throw_0168`): the angles
/// wrapped down, up and kept, the node taken (ring of one or three) or
/// not (`E + 0x3148` set), the task list empty or not.
#[test]
fn summon_throw_0168() {
    let Some(mut t) = twin(168) else { return };
    let f = t.at(0x26f0);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    for (k, (aa0, aa8, set, free, listed)) in [
        (0.5f32, 0.5f32, 0u32, 1u16, false),
        (5.0, -8.0, 0, 3, true),
        (-5.0, -0.5, 1, 2, false),
        (1.0, 1.0, 0, 2, true),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        slots_168(&mut t, k);
        let start = t.img.rel32_target(f + 0x33);
        magic_r11::object_start_setup(&mut t, start, (0x84, 0x99));
        e168(&mut t, k);
        t.set_u32(op(&t, 0x1b), E168);
        t.set_u32(E168 + 0xaa0, aa0.to_bits());
        t.set_u32(E168 + 0xaa8, aa8.to_bits());
        t.set_u32(E168 + 0x3148, set);
        for o in [0x1b6u32, 0x1e7, 0x3ef] {
            t.set_u32(op(&t, o), 0x0b0b_0000 + k);
        }
        let d = SCRATCH + 0x640;
        t.set_u32(d + 0x20, 0x0d0d_0000 + k);
        t.set_u32(op(&t, 0x3fc), d);
        let (count, head) = (op(&t, 0x4be), op(&t, 0x4cd));
        magic_r11::ring(&mut t, count, head, free, SCRATCH + 0x1c000);
        task_list(&mut t, f, (0x534, 0x543), listed, k);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0168` 0x10002c70 (`r17::summon_flight_0168`): the spin wrapped
/// down, up and kept.
#[test]
fn summon_flight_0168() {
    let Some(mut t) = twin(168) else { return };
    let f = t.at(0x2c70);
    for (k, spin) in [0.5f32, 4.0, -9.0].into_iter().enumerate() {
        let k = k as u32;
        slots_168(&mut t, k);
        e168(&mut t, k);
        t.set_u32(t.img.word(f + 0x23), E168);
        t.set_u32(E168 + 0xae4, spin.to_bits());
        for o in [0xccu32, 0x103] {
            t.set_u32(t.img.word(f + o), 0x0b0b_0000 + k);
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0168` 0x10002fb0 (`r17::summon_landing_0168`): the model on the
/// list or not, the free ring of one or three, the task list empty or
/// not.
#[test]
fn summon_landing_0168() {
    let Some(mut t) = twin(168) else { return };
    let f = t.at(0x2fb0);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    for (k, (on_list, free, listed)) in [(true, 1u16, false), (false, 3, true), (true, 2, true)]
        .into_iter()
        .enumerate()
    {
        let k = k as u32;
        slots_168(&mut t, k);
        let start = t.img.rel32_target(f + 0x68);
        magic_r11::object_start_setup(&mut t, start, (0x84, 0x99));
        e168(&mut t, k);
        t.set_u32(op(&t, 0x9), E168);
        let model = E168 + 0x5f0;
        t.set_u16(model + 0x124, 0xffff);
        let (list, n1) = (op(&t, 0x18), SCRATCH + 0x6a0);
        t.set_u32(n1, 0);
        t.set_u32(n1 + 4, 0);
        t.set_u32(n1 + 8, if on_list { model } else { 0x5151 });
        for (i, x) in [n1, n1, n1, 1].into_iter().enumerate() {
            t.set_u32(list + 4 * i as u32, x);
        }
        t.set_u32(op(&t, 0x29), SCRATCH + 0x660);
        for o in [0xf7u32, 0x128, 0x2e9] {
            t.set_u32(op(&t, o), 0x0b0b_0000 + k);
        }
        let d = SCRATCH + 0x640;
        t.set_u32(d + 0x20, 0x0d0d_0000 + k);
        t.set_u32(op(&t, 0x216), d);
        let (count, head) = (op(&t, 0x2f5), op(&t, 0x33c));
        magic_r11::ring(&mut t, count, head, free, SCRATCH + 0x1c000);
        task_list(&mut t, f, (0x3a3, 0x3bc), listed, k);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0168` 0x100036c0 (`r17::summon_spin_0168`): the yaw wrapped
/// down, up and kept, fade bytes below, at and above 0x18.
#[test]
fn summon_spin_0168() {
    let Some(mut t) = twin(168) else { return };
    let f = t.at(0x36c0);
    for (k, (yaw, fades)) in [
        (3.0f32, [0x17u8, 0x18, 0, 0xff]),
        (-6.0, [0x18, 0x17, 0x20, 2]),
        (0.0, [1, 2, 3, 0x17]),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        slots_168(&mut t, k);
        e168(&mut t, k);
        t.set_u32(t.img.word(f + 0x7), E168);
        t.set_u32(E168 + 0xaa4, yaw.to_bits());
        for (o, b) in [0x16b7u32, 0x1f5f, 0x2807, 0x30af].into_iter().zip(fades) {
            t.set_u8(E168 + o, b);
        }
        for o in [0x7cu32, 0xad] {
            t.set_u32(t.img.word(f + o), 0x0b0b_0000 + k);
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// Slot 534's answer (`model_direct`'s second way) for the summon tests.
const DIRECT_CELL: u32 = SCRATCH + 0x98;

/// What `r17::summon_task_0333` (at `f`) needs: the summon's nodes (slot
/// 412 gives `targets` targets), the list `LIST` empty, the targets'
/// handles and actors, the model's way (slots 390, 534), the particle
/// node's part start and owner, the effect node's slot 719, the vector
/// slots; `R` (`state`, counter `n`, target `i`) and the task's node in a
/// ring of two.
fn summon_0333_setup(
    t: &mut Twin,
    f: u32,
    k: u32,
    (state, n, i): (u32, u32, u32),
    targets: u32,
) -> u32 {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    vector_slots(t);
    float_slots(t, 0.5 + f64::from(k));
    let list = SCRATCH + 0x300;
    t.write(list, &[2, 0, 1, 3]);
    let count = SCRATCH + 0x9c;
    t.set_u32(count, targets);
    rec(t, 412, 3, move |m, a| {
        m.w32(a[1], m.r32(count));
        list
    });
    for s in [413u32, 215, 14] {
        rec(t, s, 1, move |_, _| s);
    }
    rec(t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, |_, _| 0.5);
    rec(t, 390, 1, |_, a| 0x2000 + (a[0] & 0xf));
    rec(t, 534, 1, |m, _| m.r32(DIRECT_CELL));
    rec(t, 268, 3, |m, a| {
        for i in 0..3 {
            wf(m, a[2] + 4 * i, 0.5 * i as f32 + (a[0] & 0xf) as f32);
        }
        0x268
    });
    rec(t, 239, 3, |_, a| a[1] ^ 0x239);
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    let table = SCRATCH + 0x3400;
    for j in 0..0x10 {
        t.set_u32(table + 4 * j, 0x1111 * (j + 1));
    }
    rec(t, 941, 1, move |_, _| table);
    // the summon's nodes, the list, the targets
    let (node, slist) = (op(t, f, 0x60), op(t, f, 0x68));
    t.write(node, &vec![0x66; 0x2640]);
    t.write(slist, &[0; 0x20]);
    t.set_u32(slist + 0x10, slist);
    t.set_u32(slist + 0xc, slist);
    t.set_u32(op(t, f, 0x54), 0x0a0a_0000 + k);
    for j in 0..4 {
        t.set_u32(op(t, f, 0x157) + 0x98 * j, 0x0b0b_0000 + j);
        t.set_u32(op(t, f, 0x172) + 0x98 * j + 0x54, 0x0c0c_0000 + j);
        t.set_u32(op(t, f, 0x2a0) + 0x98 * j, 0x0d0d_0000 + j);
    }
    t.set_u32(op(t, f, 0x2ab), 0x0e0e_0000 + k);
    t.set_u32(op(t, f, 0x2c2), targets);
    for (a, len) in [(0xc3u32, 0x2c), (0xec, 0x2c), (0xcf, 0x14c), (0x223, 56)] {
        junk(t, op(t, f, a), 4 * len, k + a);
    }
    // the particle node's part start and owner
    let pnw = t.img.rel32_target(f + 0x205);
    let pn = t.img.rel32_target(pnw + 0x35);
    let wrapper = t.img.rel32_target(pn + 0x88);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(op(t, wrapper, 0x04), lock);
    t.set_u32(lock, 0x77);
    t.set_u32(op(t, pn, 0x45), SCRATCH + 0x7f10);
    let plist = op(t, pnw, 0x26);
    t.write(plist, &[0; 0x20]);
    t.set_u32(plist + 0x10, plist);
    t.set_u32(plist + 0xc, plist);
    let owner = SCRATCH + 0x7a00;
    t.set_u32(op(t, pnw, 0x0d), owner);
    for j in 0..0x40 {
        t.set_u32(owner + 4 * j, (0.25 * j as f32).to_bits());
    }
    // the task and its record
    let (task, r, tn) = (SCRATCH + 0x700, SCRATCH + 0x780, SCRATCH + 0x740);
    let (a, b) = (SCRATCH + 0x7c0, SCRATCH + 0x7e0);
    junk(t, r, 0x40, k);
    t.set_u32(r + 0x18, state);
    t.set_u32(r + 0x1c, 1.5f32.to_bits());
    t.set_u32(r + 0x20, (-0.5f32).to_bits());
    t.set_u32(r + 0x24, i);
    t.set_u32(r + 0x28, n);
    for (x, next, prev) in [(tn, a, b), (a, b, tn), (b, tn, a)] {
        t.write(x, &[0x44; 0x10]);
        t.set_u32(x + 4, next);
        t.set_u32(x + 8, prev);
    }
    t.set_u32(task, tn);
    t.set_u32(task + 4, r);
    task
}

/// `magic_0333` 0x10006650 (`r17::summon_task_0333`): every state, the
/// counters at and past their bounds, the model's two ways, the last
/// target's unlink, a state no case takes.
#[test]
fn summon_task_0333() {
    let Some(mut t) = twin(333) else { return };
    let f = t.at(0x6650);
    for (k, (st, direct, targets)) in [
        ((0u32, 0x10u32, 0u32), 0u32, 2u32),
        ((0, 0x10, 0), 0, 3),
        ((10, 3, 0), 0, 2),
        ((11, 0x19, 0), 0, 2),
        ((11, 0x1a, 0), 1, 2),
        ((11, 0x40, 1), 0, 2),
        ((12, 0x14, 0), 0, 2),
        ((12, 0x13, 0), 0, 2),
        ((11, 0x18, 0), 0, 2),
        ((12, 0x30, 1), 0, 2),
        ((5, 7, 0), 0, 2),
        ((13, 7, 0), 0, 2),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let task = summon_0333_setup(&mut t, f, k, st, targets);
        t.set_u32(DIRECT_CELL, direct);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    // the paths ran: the nodes twice, two targets' nodes, three state-12 calls
    for (s, n) in [(412u32, 2u32), (268, 2), (12, 1), (534, 2), (239, 2)] {
        assert!(t.u32(row(s)) >= n, "slot {s}: {} calls", t.u32(row(s)));
    }
}

/// What `r17::summon_task_0332` (at `f`) needs: as
/// [`summon_0333_setup`], with the particle node called directly (its
/// owner a global's word, its list `LIST2`) and the targets' table of
/// slot 239.
fn summon_0332_setup(
    t: &mut Twin,
    f: u32,
    k: u32,
    (state, n, i): (u32, u32, u32),
    targets: u32,
) -> u32 {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    vector_slots(t);
    float_slots(t, 0.5 + f64::from(k));
    let list = SCRATCH + 0x300;
    t.write(list, &[2, 0, 1, 3]);
    let count = SCRATCH + 0x9c;
    t.set_u32(count, targets);
    rec(t, 412, 3, move |m, a| {
        m.w32(a[1], m.r32(count));
        list
    });
    for s in [413u32, 215, 14] {
        rec(t, s, 1, move |_, _| s);
    }
    rec(t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, |_, _| 0.5);
    rec(t, 268, 3, |m, a| {
        for i in 0..3 {
            wf(m, a[2] + 4 * i, 0.5 * i as f32 + (a[0] & 0xf) as f32);
        }
        0x268
    });
    rec(t, 239, 3, |_, a| a[1] ^ 0x239);
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    let table = SCRATCH + 0x3400;
    for j in 0..0x10 {
        t.set_u32(table + 4 * j, 0x1111 * (j + 1));
    }
    rec(t, 941, 1, move |_, _| table);
    let (node, slist) = (op(t, f, 0x67), op(t, f, 0x6f));
    t.write(node, &vec![0x66; 0x2640]);
    for l in [slist, op(t, f, 0x249)] {
        t.write(l, &[0; 0x20]);
        t.set_u32(l + 0x10, l);
        t.set_u32(l + 0xc, l);
    }
    t.set_u32(op(t, f, 0x5b), 0x0a0a_0000 + k);
    for j in 0..4 {
        t.set_u32(op(t, f, 0xe6) + 0x98 * j, 0x0b0b_0000 + j);
        t.set_u32(op(t, f, 0x166) + 0x98 * j + 0x54, 0x0c0c_0000 + j);
    }
    t.set_u32(op(t, f, 0x2df), targets);
    for (a, len) in [(0xd2u32, 0x2c), (0xd8, 0x2c), (0xc1, 0x14c), (0x289, 56)] {
        junk(t, op(t, f, a), 4 * len, k + a);
    }
    let pn = t.img.rel32_target(f + 0x266);
    let wrapper = t.img.rel32_target(pn + 0x88);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(op(t, wrapper, 0x04), lock);
    t.set_u32(lock, 0x77);
    t.set_u32(op(t, pn, 0x45), SCRATCH + 0x7f10);
    let owner = SCRATCH + 0x7a00;
    t.set_u32(op(t, f, 0x22c), owner);
    for j in 0..0x40 {
        t.set_u32(owner + 4 * j, (0.25 * j as f32).to_bits());
    }
    let (task, r, tn) = (SCRATCH + 0x700, SCRATCH + 0x780, SCRATCH + 0x740);
    let (a, b) = (SCRATCH + 0x7c0, SCRATCH + 0x7e0);
    junk(t, r, 0x40, k);
    t.set_u32(r + 0x18, state);
    t.set_u32(r + 0x1c, 1.5f32.to_bits());
    t.set_u32(r + 0x20, (-0.5f32).to_bits());
    t.set_u32(r + 0x24, i);
    t.set_u32(r + 0x28, n);
    for (x, next, prev) in [(tn, a, b), (a, b, tn), (b, tn, a)] {
        t.write(x, &[0x44; 0x10]);
        t.set_u32(x + 4, next);
        t.set_u32(x + 8, prev);
    }
    t.set_u32(task, tn);
    t.set_u32(task + 4, r);
    task
}

/// `magic_0332` 0x10006550 (`r17::summon_task_0332`): every state, the
/// counter at and past its bound, a middle and the last target (the
/// list's first child and a later one), the unlink state, states no case
/// takes.
#[test]
fn summon_task_0332() {
    let Some(mut t) = twin(332) else { return };
    let f = t.at(0x6550);
    let lst = t.img.word(f + 0x6f);
    let kid = SCRATCH + 0x7880;
    for (k, (st, targets, child)) in [
        ((0u32, 0x10u32, 0u32), 2u32, false),
        ((10, 3, 0), 2, false),
        ((11, 0x19, 0), 2, false),
        ((11, 0x18, 0), 2, false),
        ((11, 0x1a, 0), 2, false),
        ((11, 0x40, 1), 2, true),
        ((11, 0x40, 0), 3, true),
        ((0x2710, 7, 0), 2, false),
        ((5, 7, 0), 2, false),
        ((12, 7, 0), 2, false),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let task = summon_0332_setup(&mut t, f, k, st, targets);
        if child {
            t.write(kid, &[0x3c; 0x40]);
            t.set_u32(lst + 0x10, kid);
            t.set_u32(lst + 0xc, kid);
            t.set_u16(lst + 0x18, 1);
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    for (s, n) in [(412u32, 1u32), (268, 3), (239, 3), (719, 3)] {
        assert!(t.u32(row(s)) >= n, "slot {s}: {} calls", t.u32(row(s)));
    }
}

/// What `r17::summon_task_0399` (at `f`) needs: the model nodes (slot 218
/// gives `targets` models), the colour node's slots, the lists, the
/// targets' handles and actors, the model's way, the particle node's part
/// start and owner, slot 239's targets; `R` and the task's node.
fn summon_0399_setup(
    t: &mut Twin,
    f: u32,
    k: u32,
    (state, n, i): (u32, u32, u32),
    targets: u32,
) -> u32 {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    vector_slots(t);
    float_slots(t, 0.5 + f64::from(k));
    let list = SCRATCH + 0x300;
    t.write(list, &[2, 0, 1, 3]);
    let count = SCRATCH + 0x9c;
    t.set_u32(count, targets);
    rec(t, 218, 2, move |m, a| {
        m.w32(a[1], m.r32(count));
        list
    });
    for s in [413u32, 215, 14, 331, 353, 271] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    rec(t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |_, _| 0.625);
    rec(t, 558, 2, |m, a| {
        for j in 0..4 {
            m.w32(a[1] + 4 * j, 0x20 + 3 * j);
        }
        0x558
    });
    rec(t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, |_, _| 0.5);
    rec(t, 390, 1, |_, a| 0x2000 + (a[0] & 0xf));
    rec(t, 534, 1, |m, _| m.r32(DIRECT_CELL));
    rec(t, 268, 3, |m, a| {
        for i in 0..3 {
            wf(m, a[2] + 4 * i, 0.5 * i as f32 + (a[0] & 0xf) as f32);
        }
        0x268
    });
    rec(t, 239, 3, |_, a| a[1] ^ 0x239);
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    let table = SCRATCH + 0x3400;
    for j in 0..0x10 {
        t.set_u32(table + 4 * j, 0x1111 * (j + 1));
    }
    rec(t, 941, 1, move |_, _| table);
    let node = op(t, f, 0x63);
    t.write(node, &vec![0x66; 0x500]);
    t.write(op(t, f, 0x97), &[0x55; 0xb0]);
    for l in [op(t, f, 0x6b), op(t, f, 0x9f), op(t, f, 0x399)] {
        t.write(l, &[0; 0x20]);
        t.set_u32(l + 0x10, l);
        t.set_u32(l + 0xc, l);
    }
    t.set_u32(op(t, f, 0x57), 0x0a0a_0000 + k);
    t.set_u32(op(t, f, 0x1d3), 0x0e0e_0000 + k);
    for j in 0..4 {
        t.set_u32(op(t, f, 0x2c7) + 0x98 * j, 0x0b0b_0000 + j);
        t.set_u32(op(t, f, 0x2de) + 0x98 * j + 0x54, 0x0c0c_0000 + j);
        t.set_u32(op(t, f, 0x1c8) + 0x98 * j, 0x0d0d_0000 + j);
    }
    t.set_u32(op(t, f, 0x1ea), targets);
    for (a, len) in [(0x133u32, 0x2c), (0x13f, 0x2c), (0x16f, 0x14c), (0x3cd, 56)] {
        junk(t, op(t, f, a), 4 * len, k + a);
    }
    let pn = t.img.rel32_target(f + 0x3ae);
    let wrapper = t.img.rel32_target(pn + 0x88);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(op(t, wrapper, 0x04), lock);
    t.set_u32(lock, 0x77);
    t.set_u32(op(t, pn, 0x45), SCRATCH + 0x7f10);
    let owner = SCRATCH + 0x7a00;
    t.set_u32(op(t, f, 0x37f), owner);
    for j in 0..0x40 {
        t.set_u32(owner + 4 * j, (0.25 * j as f32).to_bits());
    }
    let (task, r, tn) = (SCRATCH + 0x700, SCRATCH + 0x780, SCRATCH + 0x740);
    let (a, b) = (SCRATCH + 0x7c0, SCRATCH + 0x7e0);
    junk(t, r, 0x40, k);
    t.set_u32(r + 0x18, state);
    t.set_u32(r + 0x1c, 1.5f32.to_bits());
    t.set_u32(r + 0x20, (-0.5f32).to_bits());
    t.set_u32(r + 0x24, i);
    t.set_u32(r + 0x28, n);
    for (x, next, prev) in [(tn, a, b), (a, b, tn), (b, tn, a)] {
        t.write(x, &[0x44; 0x10]);
        t.set_u32(x + 4, next);
        t.set_u32(x + 8, prev);
    }
    t.set_u32(task, tn);
    t.set_u32(task + 4, r);
    task
}

/// `magic_0399` 0x100069d0 (`r17::summon_task_0399`): every state, the
/// counters at and past their bounds, the model's two ways, the last
/// target, states no case takes.
#[test]
fn summon_task_0399() {
    let Some(mut t) = twin(399) else { return };
    let f = t.at(0x69d0);
    for (k, (st, direct, targets)) in [
        ((0u32, 0x10u32, 0u32), 0u32, 2u32),
        ((0, 0x10, 1), 1, 3),
        ((10, 3, 1), 0, 2),
        ((11, 0x19, 0), 0, 2),
        ((11, 0x18, 0), 0, 2),
        ((11, 0x40, 1), 1, 2),
        ((12, 0x13, 0), 0, 2),
        ((13, 0x4f, 0), 0, 2),
        ((12, 0x14, 0), 0, 2),
        ((12, 0x15, 0), 0, 2),
        ((12, 0x30, 1), 0, 2),
        ((13, 0x50, 0), 0, 2),
        ((13, 0x51, 0), 0, 2),
        ((0x2710, 7, 0), 0, 2),
        ((5, 7, 0), 0, 2),
        ((14, 7, 0), 0, 2),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let task = summon_0399_setup(&mut t, f, k, st, targets);
        t.set_u32(DIRECT_CELL, direct);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    for (s, n) in [
        (218u32, 2u32),
        (268, 3),
        (12, 1),
        (239, 2),
        (719, 3),
        (558, 2),
    ] {
        assert!(t.u32(row(s)) >= n, "slot {s}: {} calls", t.u32(row(s)));
    }
}

/// The slots and globals `magic_0330`'s summon functions share: the model
/// nodes (slot 412 gives `targets` models), slot 268, 239, 719, 714, 941,
/// 325 (vector slots), 307 (a random in ST0), the list `LIST` empty, the
/// object ring empty, the targets' actors, the particle node's part start
/// and owner.
fn summon_0330_common(t: &mut Twin, k: u32, targets: u32) {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    vector_slots(t);
    float_slots(t, 0.5 + f64::from(k));
    let list = SCRATCH + 0x300;
    t.write(list, &[2, 0, 1, 3]);
    let count = SCRATCH + 0x9c;
    t.set_u32(count, targets);
    rec(t, 412, 3, move |m, a| {
        m.w32(a[1], m.r32(count));
        list
    });
    for s in [413u32, 215, 14] {
        rec(t, s, 1, move |_, _| s);
    }
    rec(t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, |_, _| 0.5);
    rec(t, 268, 3, |m, a| {
        for i in 0..3 {
            wf(m, a[2] + 4 * i, 0.5 * i as f32 + (a[0] & 0xf) as f32);
        }
        0x268
    });
    rec(t, 239, 3, |_, a| a[1] ^ 0x239);
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    rec(t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, |m, _| f64::from(m.r32(row(307)) % 5) * 0.375 + 0.25);
    let table = SCRATCH + 0x3400;
    for j in 0..0x10 {
        t.set_u32(table + 4 * j, 0x1111 * (j + 1));
    }
    rec(t, 941, 1, move |_, _| table);
    let task = t.at(0x6a40);
    let obj = t.at(0x68f0);
    let step = t.at(0x6550);
    for l in [op(t, task, 0x65), op(t, step, 0x340)] {
        t.write(l, &[0; 0x20]);
        t.set_u32(l + 0x10, l);
        t.set_u32(l + 0xc, l);
    }
    let ring = op(t, obj, 0x2f);
    t.write(ring, &[0; 0x10]);
    t.set_u32(ring + 4, ring);
    t.set_u32(ring + 8, ring);
    for j in 0..4 {
        t.set_u32(op(t, obj, 0x97) + 0x98 * j + 0x54, 0x0c0c_0000 + j);
        t.set_u32(op(t, step, 0x52) + 0x98 * j, 0x0d0d_0000 + j);
    }
    junk(t, op(t, obj, 0xd9), 4 * 56, k);
    t.set_u32(op(t, step, 0x5d), 0x0e0e_0000 + k);
    let pn = t.img.rel32_target(step + 0x35f);
    let wrapper = t.img.rel32_target(pn + 0x88);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(op(t, wrapper, 0x04), lock);
    t.set_u32(lock, 0x77);
    t.set_u32(op(t, pn, 0x45), SCRATCH + 0x7f10);
    let owner = SCRATCH + 0x7a00;
    t.set_u32(op(t, step, 0x330), owner);
    for j in 0..0x40 {
        t.set_u32(owner + 4 * j, (0.25 * j as f32).to_bits());
    }
}

/// A task at `SCRATCH + 0x700` for `obj` (its node in a ring of three).
fn summon_task_at(t: &mut Twin, obj: u32) -> u32 {
    let (task, tn, a, b) = (
        SCRATCH + 0x700,
        SCRATCH + 0x740,
        SCRATCH + 0x7c0,
        SCRATCH + 0x7e0,
    );
    for (x, next, prev) in [(tn, a, b), (a, b, tn), (b, tn, a)] {
        t.write(x, &[0x44; 0x10]);
        t.set_u32(x + 4, next);
        t.set_u32(x + 8, prev);
    }
    t.set_u32(task, tn);
    t.set_u32(task + 4, obj);
    task
}

/// `magic_0330` 0x10006a40 (`r17::summon_task_0330`, with
/// `r17::summon_object_0330`): every state, the counter at and past its
/// bound, a middle and the last target.
#[test]
fn summon_task_0330() {
    let Some(mut t) = twin(330) else { return };
    let f = t.at(0x6a40);
    let objs = t.img.word(f + 0xb8);
    for (k, ((state, n, i, j), targets)) in [
        ((0u32, 0x10u32, 1u32, 3u32), 2u32),
        ((10, 3, 1, 3), 2),
        ((11, 0x19, 0, 0), 2),
        ((11, 0x18, 0, 0), 2),
        ((11, 0x40, 0, 0), 2),
        ((11, 0x40, 1, 1), 2),
        ((0x2710, 7, 0, 0), 2),
        ((5, 7, 0, 0), 2),
        ((12, 7, 0, 0), 2),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        summon_0330_common(&mut t, k, targets);
        t.write(t.img.word(f + 0x5e), &vec![0x66; 0x2640]);
        t.set_u32(t.img.word(f + 0x54), 0x0a0a_0000 + k);
        t.set_u32(t.img.word(f + 0xcc), targets);
        junk(&mut t, objs, 2 * 0xaf0, k);
        let r = SCRATCH + 0x780;
        junk(&mut t, r, 0x40, k);
        for (o, v) in [(0x18u32, state), (0x24, i), (0x28, n), (0x2c, j)] {
            t.set_u32(r + o, v);
        }
        t.set_u32(r + 0x1c, 1.5f32.to_bits());
        let task = summon_task_at(&mut t, r);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    for (s, n) in [(412u32, 1u32), (268, 2), (719, 2)] {
        assert!(t.u32(row(s)) >= n, "slot {s}: {} calls", t.u32(row(s)));
    }
}

/// `magic_0330` 0x10006550 (`r17::summon_step_0330`): before and at each
/// stage's time, stage 2's slot 239, the last stage's unlink.
#[test]
fn summon_step_0330() {
    let Some(mut t) = twin(330) else { return };
    let f = t.at(0x6550);
    let times = t.img.word(f + 0x33);
    for (k, (stage, early)) in [
        (0u32, true),
        (0, false),
        (1, false),
        (2, false),
        (3, false),
        (2, true),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        summon_0330_common(&mut t, k, 2);
        let o = SCRATCH + 0x10000;
        junk(&mut t, o, 0xb00, k);
        let tm = f32::from_bits(t.u32(times + 4 * stage));
        let clock = if early { tm - 0.25 } else { tm + 0.5 };
        t.set_u32(o + 0xadc, clock.to_bits());
        t.set_u32(o + 0xae0, stage);
        t.set_u32(o + 0xae4, 1);
        for (i, x) in [1.5f32, -2.0, 0.25, 1.0].into_iter().enumerate() {
            t.set_u32(o + 0x59c + 4 * i as u32, x.to_bits());
        }
        let task = summon_task_at(&mut t, o);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    for (s, n) in [(307u32, 8u32), (239, 1), (325, 3)] {
        assert!(t.u32(row(s)) >= n, "slot {s}: {} calls", t.u32(row(s)));
    }
}

/// What `r17::summon_task_0159` (at `f`) needs: the slots its callees
/// make, the summon list and the particle list empty, the actors, the
/// facing's flag (the monster way), the bone's pass byte equal to the
/// model node's (so the bone update stops at its walk), the particle
/// node's part start and owner; `R` (`state`, counters) and the task.
fn summon_0159_setup(t: &mut Twin, f: u32, k: u32, (state, n28, n2c): (u32, u32, u32)) -> u32 {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    vector_slots(t);
    float_slots(t, 0.5 + f64::from(k));
    for (s, n) in [
        (413u32, 1usize),
        (215, 1),
        (14, 1),
        (211, 2),
        (228, 2),
        (695, 1),
        (257, 2),
        (296, 2),
        (38, 3),
        (541, 3),
        (538, 2),
        (213, 2),
        (35, 3),
        (34, 2),
        (547, 1),
        (276, 1),
        (219, 2),
        (601, 2),
    ] {
        rec(t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    for s in [333u32, 277] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| {
            0.5 + f64::from(a[0] & 7) * 0.25 + f64::from(s) * 0.001
        });
    }
    rec(t, 262, 4, move |m, a| {
        for i in 1..4 {
            wf(m, a[i], 0.75 * i as f32 - k as f32);
        }
        0x262
    });
    rec(t, 237, 4, |m, a| {
        for i in 0..4 {
            wf(m, a[3] + 4 * i, 0.5 * i as f32 - 1.0);
        }
        0x237
    });
    rec(t, 411, 1, |m, _| m.r32(DIRECT_CELL));
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    let table = SCRATCH + 0x3400;
    for j in 0..0x10 {
        t.set_u32(table + 4 * j, 0x1111 * (j + 1));
    }
    rec(t, 941, 1, move |_, _| table);
    let list = op(t, f, 0x43);
    let pf = t.img.rel32_target(f + 0x520);
    for l in [list, op(t, pf, 0x20)] {
        t.write(l, &[0; 0x20]);
        t.set_u32(l + 0x10, l);
        t.set_u32(l + 0xc, l);
    }
    t.set_u32(op(t, f, 0x2f), 0x0a0a_0000 + k);
    for o in [0xd1u32, 0x38e] {
        t.set_u32(op(t, f, o), 0x0b0b_0000 + k);
    }
    let fa = t.img.rel32_target(f + 0x18a);
    t.set_u8(op(t, fa, 0x16), 1);
    t.set_u32(op(t, fa, 0x24), 0x0c0c_0000 + k);
    t.set_u8(op(t, f, 0x1c8), 0xff);
    let pn = t.img.rel32_target(pf + 0x36);
    let wrapper = t.img.rel32_target(pn + 0x88);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(op(t, wrapper, 0x04), lock);
    t.set_u32(lock, 0x77);
    t.set_u32(op(t, pn, 0x45), SCRATCH + 0x7f10);
    let owner = SCRATCH + 0x7a00;
    t.set_u32(op(t, pf, 0x0d), owner);
    for j in 0..0x40 {
        t.set_u32(owner + 4 * j, (0.25 * j as f32).to_bits());
    }
    let node = op(t, f, 0x3b);
    t.write(node, &[0x22; 0x100]);
    let r = SCRATCH + 0x780;
    junk(t, r, 0x40, k);
    for (o, v) in [(0x18u32, state), (0x28, n28), (0x2c, n2c)] {
        t.set_u32(r + o, v);
    }
    summon_task_at(t, r)
}

/// `magic_0159` 0x10006750 (`r17::summon_task_0159`): every state, the
/// counts at their bounds, slot 411's two answers, states no case takes.
#[test]
fn summon_task_0159() {
    let Some(mut t) = twin(159) else { return };
    let f = t.at(0x6750);
    for (k, (st, answer)) in [
        ((0u32, 3u32, 7u32), 0u32),
        ((10, 3, 7), 0),
        ((11, 3, 0xe), 0),
        ((11, 3, 0xf), 0),
        ((12, 3, 4), 0),
        ((12, 3, 5), 1),
        ((13, 3, 0), 0),
        ((13, 3, 0), 1),
        ((0x2710, 3, 0), 0),
        ((5, 3, 0), 0),
        ((14, 3, 0), 0),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let task = summon_0159_setup(&mut t, f, k, st);
        t.set_u32(DIRECT_CELL, answer);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    for (s, n) in [(262u32, 2u32), (237, 1), (411, 4), (228, 2), (695, 1)] {
        assert!(t.u32(row(s)) >= n, "slot {s}: {} calls", t.u32(row(s)));
    }
}

/// What `r17::summon_task_0379` (at `f`) needs: the slots its callees
/// make, the summon list and the particle lists empty, the actor, the
/// flag byte (`flag`), the particle nodes' part starts and owners; `R`
/// (`state`, count `n`) and the task.
fn summon_0379_setup(t: &mut Twin, f: u32, k: u32, (state, n): (u32, u32), flag: u8) -> u32 {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    vector_slots(t);
    float_slots(t, 0.5 + f64::from(k));
    let list = SCRATCH + 0x300;
    t.write(list, &[2, 0, 1, 3]);
    let count = SCRATCH + 0x9c;
    t.set_u32(count, 2);
    rec(t, 218, 2, move |m, a| {
        m.w32(a[1], m.r32(count));
        list
    });
    for (s, n) in [
        (413u32, 1usize),
        (215, 1),
        (14, 1),
        (331, 1),
        (353, 1),
        (271, 1),
        (366, 2),
        (228, 2),
        (257, 2),
        (296, 2),
        (38, 3),
        (541, 3),
    ] {
        rec(t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    rec(t, 558, 2, |m, a| {
        for j in 0..4 {
            m.w32(a[1] + 4 * j, 0x20 + 3 * j);
        }
        0x558
    });
    rec(t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |_, _| 0.625);
    rec(t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, |_, _| 0.5);
    rec(t, 411, 1, |m, _| m.r32(DIRECT_CELL));
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    let table = SCRATCH + 0x3400;
    for j in 0..0x10 {
        t.set_u32(table + 4 * j, 0x1111 * (j + 1));
    }
    rec(t, 941, 1, move |_, _| table);
    let pnw = t.img.rel32_target(f + 0x493);
    for l in [op(t, f, 0x7e), op(t, pnw, 0x26), op(t, f, 0x134)] {
        t.write(l, &[0; 0x20]);
        t.set_u32(l + 0x10, l);
        t.set_u32(l + 0xc, l);
    }
    for o in [0x2dfu32, 0xa1, 0x16c, 0x3b, 0x383, 0x3eb, 0x4a0] {
        t.set_u32(op(t, f, o), 0x0a0a_0000 + k);
    }
    t.set_u32(op(t, f, 0x323), 0x0d0d_0034 + k);
    t.set_u8(op(t, f, 0x8e), flag);
    for (pn, owner_op, owner) in [
        (t.img.rel32_target(f + 0x15f), None, SCRATCH + 0x7a00),
        (
            t.img.rel32_target(pnw + 0x35),
            Some(op(t, pnw, 0x0d)),
            SCRATCH + 0x7b00,
        ),
    ] {
        let wrapper = t.img.rel32_target(pn + 0x88);
        let lock = SCRATCH + 0x7f00;
        t.set_u32(op(t, wrapper, 0x04), lock);
        t.set_u32(lock, 0x77);
        t.set_u32(op(t, pn, 0x45), SCRATCH + 0x7f10);
        let at = owner_op.unwrap_or(op(t, f, 0x119));
        t.set_u32(at, owner);
        for j in 0..0x40 {
            t.set_u32(owner + 4 * j, (0.25 * j as f32).to_bits());
        }
    }
    // the camera block's sources: real floats
    for o in [
        0x1cdu32, 0x202, 0x23d, 0x278, 0x28b, 0x29e, 0x2a6, 0x2b6, 0x2be,
    ] {
        t.set_u32(op(t, f, o), (0.125 * o as f32 - 20.0).to_bits());
    }
    t.write(op(t, f, 0x3bc), &[0x22; 0x100]);
    junk_ops(t, f, &[0x2ce, 0x2c6, 0x280]);
    let r = SCRATCH + 0x780;
    junk(t, r, 0x40, k);
    t.set_u32(r + 0x18, state);
    t.set_u32(r + 0x2c, n);
    summon_task_at(t, r)
}

/// `magic_0379` 0x10006af0 (`r17::summon_task_0379`): every state, the
/// counts at their bounds, slot 411's two answers, the flag's two ways,
/// states no case takes.
#[test]
fn summon_task_0379() {
    let Some(mut t) = twin(379) else { return };
    let f = t.at(0x6af0);
    for (k, (st, flag, answer)) in [
        ((0u32, 3u32), 0u8, 0u32),
        ((10, 3), 0, 0),
        ((10, 3), 1, 0),
        ((11, 3), 0, 0),
        ((11, 3), 0, 1),
        ((20, 3), 0, 0),
        ((21, 3), 0, 0),
        ((21, 3), 0, 1),
        ((22, 0x1d), 0, 0),
        ((22, 0x1e), 0, 0),
        ((23, 0x1d), 0, 0),
        ((23, 0x1e), 0, 0),
        ((0x2710, 3), 0, 0),
        ((5, 3), 0, 0),
        ((15, 3), 0, 0),
        ((30, 3), 0, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let task = summon_0379_setup(&mut t, f, k, st, flag);
        t.set_u32(DIRECT_CELL, answer);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    for (s, n) in [(218u32, 1u32), (366, 1), (228, 3), (411, 4), (719, 2)] {
        assert!(t.u32(row(s)) >= n, "slot {s}: {} calls", t.u32(row(s)));
    }
}

/// The slots `magic_0172`'s functions call and the actors (`n` targets).
fn slots_0172(t: &mut Twin, k: u32) {
    magic_r11::object_slots(t);
    vector_slots(t);
    float_slots(t, 0.5 - f64::from(k));
    rec(t, 262, 4, move |m, a| {
        let c = m.r32(row(262)) as f32;
        for i in 1..4 {
            wf(m, a[i], 0.5 * i as f32 + c * 0.75 - k as f32);
        }
        0x262
    });
    rec(t, 268, 3, |m, a| {
        let c = m.r32(row(268)) as f32;
        for i in 0..4 {
            wf(m, a[2] + 4 * i, 1.25 - i as f32 + c * 0.5);
        }
        0x268
    });
    for s in [413u32, 215, 723, 724] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
}

/// `magic_0172` 0x10001210 (`r9_load::load_0172`, the start-up
/// `r17::start_up_0172`): busy, not loaded, then the start-up with none to
/// three targets, slot 413 or 215, the free ring of one or three, the task
/// list empty or not.
#[test]
fn load_0172() {
    let Some(mut t) = twin(172) else { return };
    let f = t.at(0x1210);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let start = t.img.rel32_target(f + 0x5b);
    let op = |t: &Twin, o: u32| t.img.word(start + o);
    let (busy, loaded) = (SCRATCH + 0x100, SCRATCH + 0x108);
    for (k, (b, l, n, flag, free, listed)) in [
        (1u32, 1u32, 0u32, 0u32, 1u16, false),
        (0, 0, 0, 0, 1, false),
        (0, 1, 0, 0, 1, false),
        (0, 1, 2, 1, 3, true),
        (0, 2, 3, 0, 2, true),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        slots_0172(&mut t, k);
        rec(&mut t, 225, 1, move |m, _| m.r32(busy));
        rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
        rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
        t.set_u32(busy, b);
        t.set_u32(loaded, l);
        t.set_u32(w(&t, 0x18), k & 1);
        t.set_u32(w(&t, 0x26), 0x0000_5e5e);
        for o in [0x61u32, 0x6b] {
            t.set_u32(w(&t, o), 0x0bad_0000 + o);
        }
        magic_r11::pools(&mut t, start, 945);
        let so = t.img.rel32_target(start + 0x66);
        magic_r11::object_start_setup(&mut t, so, (0x84, 0x9d));
        for o in [0x72u32, 0xd6] {
            t.set_u32(op(&t, o), 0x0a00_0000 + o);
        }
        t.set_u32(op(&t, 0x17f), n);
        let sel = SCRATCH + 0x5000;
        t.write(sel, &[2, 1, 0, 3]);
        t.set_u32(op(&t, 0x1a0), sel);
        for i in 0..4 {
            t.set_u32(op(&t, 0x1ab) + 4 * i, 0x0c0c_0100 + i);
        }
        let recs = op(&t, 0x18a) - 4;
        junk(&mut t, recs, 3 * 0x160, k);
        t.set_u32(op(&t, 0x279), flag);
        t.set_u32(op(&t, 0x27f), 0x027f_0000 + k);
        junk_ops(&mut t, start, &[0xdc, 0x2e8, 0x100, 0x10a]);
        let (count, head) = (op(&t, 0x2f8), op(&t, 0x307));
        magic_r11::ring(&mut t, count, head, free, SCRATCH + 0x1c000);
        task_list(&mut t, start, (0x36e, 0x37d), listed, k);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(row(234)) >= 4, "the target loop ran");
}

/// `magic_0172` 0x10001700 (`r17::frame_0172`, with `r17::throws_0172`):
/// no objects (done), the motion done or waiting, the throws off, and on
/// for targets thrown, near and far.
#[test]
fn frame_0172() {
    let Some(mut t) = twin(172) else { return };
    let f = t.at(0x1700);
    let throws = t.img.rel32_target(f + 0xc);
    let ro = t.img.rel32_target(f + 0x5);
    let ms = t.img.rel32_target(f + 0x11);
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    for (k, (on, done, n)) in [(0u32, 1u32, 2u32), (1, 1, 3), (1, 0, 2), (2, 1, 0)]
        .into_iter()
        .enumerate()
    {
        let k = k as u32;
        slots_0172(&mut t, k);
        let so = t.img.rel32_target(throws + 0x129);
        magic_r11::object_start_setup(&mut t, so, (0x84, 0x9d));
        // run_objects: an empty list, its pointer globals
        t.set_u32(op(&t, ro, 0x1a), 0);
        t.set_u32(op(&t, ro, 0x25), SCRATCH + 0x60);
        let list = op(&t, ro, 0x7c);
        t.write(list, &[0; 0x10]);
        t.set_u32(op(&t, ro, 0x14), 0);
        // motion_step: done, or waiting
        t.set_u32(op(&t, ms, 0x02), done);
        t.set_u16(op(&t, ms, 0x11), 5);
        // the throws
        t.set_u32(op(&t, throws, 0x16), on);
        t.set_u32(op(&t, throws, 0x27), n);
        let sel = SCRATCH + 0x5000;
        t.write(sel, &[2, 1, 0, 3]);
        t.set_u32(op(&t, throws, 0x13e), sel);
        for i in 0..4 {
            t.set_u32(op(&t, throws, 0x149) + 4 * i, 0x0c0c_0100 + i);
        }
        let recs = op(&t, throws, 0x32);
        junk(&mut t, recs - 0xd4, 3 * 0x160 + 0xd4, k);
        for i in 0..3u32 {
            let r = recs + 0x160 * i;
            t.set_u32(r + 0x80, u32::from(i == 2));
            t.set_u32(
                r + 0x7c,
                (2.0 + 998.0 * i as f32 - k as f32 * 0.5).to_bits(),
            );
            for e in 0..4 {
                t.set_u32(r + 0x6c + 4 * e, (0.5 * e as f32).to_bits());
                t.set_u32(
                    r + 0x5c + 4 * e,
                    (0.25 * e as f32 + 3.0 * i as f32).to_bits(),
                );
            }
        }
        let task = SCRATCH + 0x700;
        junk(&mut t, task, 0x20, k);
        t.set_u32(task + 8, SCRATCH + 0x780);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    assert!(t.u32(row(268)) >= 1, "no throw ran");
}
