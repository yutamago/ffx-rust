//! Round 31's ports (`ffx_magic::r31`) against each original, and the
//! step-0 sweep's copies with relocated immediates. Needs `FFX_GAME_DIR`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    clippy::type_complexity
)]

mod magic;
mod magic_hi;
mod magic_owna;
#[path = "magic/ownb.rs"]
mod ownb;

use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use ownb::rec;

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

fn rf(m: &dyn StubMem, at: u32) -> f32 {
    f32::from_bits(m.r32(at))
}

fn wf(m: &mut dyn StubMem, at: u32, v: f32) {
    m.w32(at, v.to_bits());
}

/// `n` bytes of junk at `at` (varied by `k`).
fn junk(t: &mut Twin, at: u32, n: u32, k: u32) {
    let b: Vec<u8> = (0..n)
        .map(|i| (i.wrapping_mul(0x3d) ^ (0x71 + k)).to_le_bytes()[0])
        .collect();
    t.write(at, &b);
}

/// `n` floats at `at`: `x0 + 0.25 i` (no NaNs).
fn floats(t: &mut Twin, at: u32, n: u32, x0: f32) {
    for i in 0..n {
        t.set_u32(at + 4 * i, (x0 + 0.25 * i as f32).to_bits());
    }
}

/// Every slot answering `first word ^ (slot << 20)`.
fn all_stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
}

/// The matrix slots with float results (no NaNs from finite inputs):
/// 235 a unit matrix; 330/328/329 `out = in + angle (i + 1) / 8`; 229
/// `out = row0 + row3 + 2 v`; 248 `out = a - b`; the scale slots
/// 15/16/17/277 and the random 404/314 floats in ST0.
fn float_matrix_stubs(t: &mut Twin) {
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    for s in [330u32, 328, 329] {
        rec(t, s, 3, move |m, a| {
            let ang = f32::from_bits(a[2]);
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) + ang * (i + 1) as f32 / 8.0 + s as f32 / 1024.0;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[1] + 0x30 + 4 * i) + 2.0 * rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    for (s, x) in [(15u32, 1.5f64), (16, -0.75), (17, 3.0), (277, 40.0)] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    for (s, x) in [(404u32, 0.0625f64), (314, -0.125)] {
        rec(t, s, 1, move |m, a| {
            let k = m.r32(SCRATCH + 0xa00);
            m.w32(SCRATCH + 0xa00, k + 1);
            (s << 16) ^ a[0]
        });
        t.stub_st0(s, move |m, a| {
            x * f64::from(m.r32(SCRATCH + 0xa00) % 5) + f64::from(f32::from_bits(a[0]))
        });
    }
}

/// Junk words at the globals the operands `ops` of `f` name.
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, (0x5a5a_5a5a ^ o) | 1);
    }
}

/// `static_node_link_b` / `_c` in the copies whose last stores carry
/// relocated immediates (addresses; `magic_0152`/`0374`'s store 0, and
/// the `Sig` masks the difference): `magic_0155` 0x10009900 (one
/// address), `magic_0577` 0x1000a650 and 0x1000d870 (two each).
#[test]
fn static_links_store_their_immediates() {
    for (n, addrs, stores) in [
        (155u16, &[0x9900u32][..], &[0x4cu32, 0x56, 0x60][..]),
        (577, &[0xa650][..], &[0x4c, 0x56, 0x60][..]),
        (577, &[0xd870][..], &[0x4c, 0x56, 0x60, 0x6a][..]),
    ] {
        let Some(mut t) = twin(n) else { return };
        for &a in addrs {
            let f = t.at(a);
            let last = SCRATCH + 0x300;
            t.write(last, &[0x77; 0x10]);
            t.set_u32(t.img.word(f + 0x1), last);
            junk_ops(&mut t, f, &[0x7, 0x11, 0x1b, 0x24, 0x2a, 0x3b, 0x42]);
            junk_ops(&mut t, f, stores);
            assert!(
                stores.iter().any(|&o| t.img.word(f + o + 4) != 0),
                "magic_{n:04} {a:#x}: no relocated immediate"
            );
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// A spinning piece's record: (delay, blend count, mode, `E + 0xa4`, `E +
/// 0xac`, done, wait, angle base).
type PieceRec = (i32, i32, u32, u32, u32, u32, u16, f32);

/// `magic_0605` 0x100082f0 and `magic_0307` 0x10003770
/// (`r31::spin_pieces_0605`, family P015): records waiting, blending,
/// spun by the random slots, by constants and by the three-angle step,
/// each wrap both ways; "all done" set, cleared by a script or by `E +
/// 0xac`; no records (count 0 and below).
#[test]
fn spin_pieces_0605() {
    for (n, at) in [(605u16, 0x82f0u32), (307, 0x3770)] {
        let Some(mut t) = twin(n) else { return };
        all_stubs(&mut t);
        float_matrix_stubs(&mut t);
        // slot 411: the motion not ended
        rec(&mut t, 411, 1, |_, a| a[0] & 0x100);
        let f = t.at(at);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let scale = t.img.rel32_target(f + 0x371);
        // the actor scale's list and objects
        for o in [0x28u32, 0x74] {
            let l = t.img.word(scale + o);
            t.set_u32(l, SCRATCH + 0x700);
        }
        for o in [0x33u32, 0x7f] {
            let l = t.img.word(scale + o);
            for i in 0..8 {
                t.set_u32(l + 4 * i, 0x0b00_0000 + i);
            }
        }
        t.write(SCRATCH + 0x700, &[3, 1, 4, 1, 5, 2, 6, 0]);
        let s = SCRATCH + 0x1_0000;
        let d = w(&t, 0x23);
        let (gs, gn, pl, fl) = (w(&t, 0x1a), w(&t, 0x12), w(&t, 0x46a), w(&t, 0x4e9));
        // per case: the count; per record: (delay, blend count, mode,
        // a4, ac, done, wait, angle base)
        let cases: [(i32, &[PieceRec]); 6] = [
            (
                5,
                &[
                    (2, 3, 1, 0, 1, 1, 0, 0.5),
                    (0, 3, 1, 0, 1, 0, 3, 3.0),
                    (-1, 0, 1, 1, 1, 1, 0, -3.0),
                    (0, 5, 0, 0, 1, 0, 0, 3.1),
                    (0, -2, 2, 1, 1, 1, 0, -3.1),
                ],
            ),
            (
                3,
                &[
                    (0, 1, 1, 0, 1, 1, 0, -3.0),
                    (0, 0, 1, 1, 1, 1, 0, 3.0),
                    (0, 2, 0, 0, 2, 1, 0, -3.1),
                ],
            ),
            (1, &[(0, 0, 1, 0, 0, 1, 0, 0.25)]),
            (
                2,
                &[(1, 0, 1, 0, 1, 1, 0, 3.0), (0, 4, 0, 1, 1, 1, 0, 3.12)],
            ),
            (0, &[]),
            (-2, &[]),
        ];
        for (k, (count, recs)) in (0u32..).zip(cases) {
            t.set_u32(gs, s);
            t.set_u32(gn, count.cast_unsigned());
            junk(&mut t, s + d, 0xd4 * 6, k);
            floats(&mut t, s + pl, 4, -1.5 + k as f32);
            t.set_u32(s + fl, 0x0bad_0000 + k);
            t.set_u32(SCRATCH + 0xa00, k);
            for (i, &(delay, bl, mode, a4, ac, done, wait, base)) in (0u32..).zip(recs) {
                let r = s + d + 0xd4 * i;
                let e = r + 0x24;
                t.set_u32(r + 0x14, done);
                t.set_u16(r + 0x18, wait);
                t.set_u16(r + 0x1a, 7);
                t.set_u16(r + 0x1c, 0);
                t.set_u32(r + 8, 0x0008_0000 + 0x100 * (i & 1));
                floats(&mut t, r + 0x20, 0x9c / 4 + 1, base - 2.0);
                for (o, v) in [(0x5cu32, base), (0x60, -base), (0x64, base * 0.5)] {
                    t.set_u32(e + o, v.to_bits());
                }
                t.set_u32(e + 0x8c, base.to_bits());
                t.set_u32(e + 0x90, (base * 0.25).to_bits());
                t.set_u32(e + 0x9c, bl.cast_unsigned());
                t.set_u32(e + 0xa0, delay.cast_unsigned());
                t.set_u32(e + 0xa4, a4);
                t.set_u32(e + 0xa8, mode);
                t.set_u32(e + 0xac, ac);
            }
            ownb::dirty_stack(&mut t, at + k, &[]);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `magic_0605` 0x100087f0 (`r31::spin_start_0605`): three, one and no
/// records; the pieces far (scaled down) and near; the yaws wrapped both
/// ways; the free ring with one node and with three, the task list empty
/// and not.
#[test]
fn spin_start_0605() {
    let Some(mut t) = twin(605) else { return };
    all_stubs(&mut t);
    float_matrix_stubs(&mut t);
    let f = t.at(0x87f0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    // slot 262: the actor's place from its object word
    rec(&mut t, 262, 4, |m, a| {
        let k = (a[0] & 0xff) as f32;
        let scale = rf(m, SCRATCH + 0xa04);
        wf(m, a[1], scale * (k - 2.0));
        wf(m, a[2], 0.5 * k);
        wf(m, a[3], scale * (3.0 - k));
        0x262
    });
    for s in [12u32, 333] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xf));
    t.stub_st0(333, |m, a| {
        f64::from(rf(m, SCRATCH + 0xa08)) * f64::from((a[0] & 0xff) + 1)
    });
    rec(&mut t, 528, 2, |_, a| 0x0528_0000 ^ a[0] ^ a[1]);
    let mode = t.img.rel32_target(f + 0x2f8);
    for o in [0x54u32, 0x309] {
        let l = w(&t, o);
        t.set_u32(l, SCRATCH + 0x700);
    }
    t.write(SCRATCH + 0x700, &[3, 1, 4, 1, 5, 2, 6, 0]);
    for i in 0..8 {
        let at = w(&t, 0x5f) + 4 * i;
        t.set_u32(at, 0x0b00_0010 + 3 * i);
    }
    for o in [0x21u32, 0x2e] {
        let l = t.img.word(mode + o);
        if o == 0x21 {
            t.set_u32(l, SCRATCH + 0x700);
        } else {
            for i in 0..8 {
                t.set_u32(l + 4 * i, 0x0c00_0000 + i);
            }
        }
    }
    let s = SCRATCH + 0x1_0000;
    let (gs, gn) = (w(&t, 0x1c), w(&t, 0x15));
    let (free, head, tail, last) = (w(&t, 0x381), w(&t, 0x390), w(&t, 0x3f7), w(&t, 0x406));
    let nodes = SCRATCH + 0x9000;
    // (count, place scale, slot 333's angle step, free nodes, list tail)
    let cases: [(i32, f32, f32, u16, u32); 5] = [
        (3, 40.0, 1.5, 3, SCRATCH + 0x9800),
        (1, 0.25, -2.0, 1, 0),
        (2, 9.0, 0.25, 2, SCRATCH + 0x9800),
        (0, 1.0, 1.0, 1, 0),
        (-1, 1.0, 1.0, 2, SCRATCH + 0x9800),
    ];
    for (k, (count, scale, step, nfree, tl)) in (0u32..).zip(cases) {
        t.set_u32(gs, s);
        t.set_u32(gn, count.cast_unsigned());
        junk(&mut t, s + 0x1800, 0x600, k);
        junk(&mut t, s + 0x3000, 0x100, k + 1);
        floats(&mut t, s + 0x14a0, 4, 0.75 - k as f32);
        t.set_u32(SCRATCH + 0xa04, scale.to_bits());
        t.set_u32(SCRATCH + 0xa08, step.to_bits());
        t.set_u32(SCRATCH + 0xa00, k);
        // the free ring: nodes 0x40 apart, linked both ways
        for i in 0..3u32 {
            let n = nodes + 0x40 * i;
            junk(&mut t, n, 0x40, k + i);
            t.set_u32(n, nodes + 0x40 * ((i + 2) % 3));
            t.set_u32(n + 4, nodes + 0x40 * ((i + 1) % 3));
        }
        t.set_u16(free, nfree);
        t.set_u32(head, nodes + 0x40);
        t.set_u32(tail, tl);
        junk(&mut t, SCRATCH + 0x9800, 0x10, k);
        t.set_u32(last, SCRATCH + 0x9800);
        ownb::dirty_stack(&mut t, 0x87f0 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// The globals the object start at `so` reads (as `magic_r27.rs`'s
/// `start_env_at`): its item and model words, slot 941's part table, the
/// pool (`count` free nodes, its call at `pool_op`) and the object list's
/// tail `tl` (operand `tail_op`).
fn start_env_at(t: &mut Twin, so: u32, (pool_op, tail_op): (u32, u32), count: u16, tl: u32) {
    t.set_u32(t.img.word(so + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(so + 0x5e), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    t.stub(941, |m, a| {
        m.w32(SCRATCH + 0x900, a[0]);
        SCRATCH + 0x400
    });
    t.stub(714, |m, a| {
        for (i, w) in (0u32..).zip(&a[..5]) {
            m.w32(SCRATCH + 0x910 + 4 * i, *w);
        }
        0x714
    });
    let pa = t.img.rel32_target(so + pool_op);
    let free = t.img.word(pa + 0x04);
    let nodes = t.img.word(pa + 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    t.set_u16(free, count);
    t.set_u32(t.img.word(so + tail_op), tl);
}

/// A free ring of three nodes (0x40 apart at `nodes`, linked both ways)
/// with `nfree` counted, its head the second node.
fn free_ring(t: &mut Twin, nodes: u32, (free, head): (u32, u32), nfree: u16, k: u32) {
    for i in 0..3u32 {
        let n = nodes + 0x40 * i;
        junk(t, n, 0x40, k + i);
        t.set_u32(n, nodes + 0x40 * ((i + 2) % 3));
        t.set_u32(n + 4, nodes + 0x40 * ((i + 1) % 3));
    }
    t.set_u16(free, nfree);
    t.set_u32(head, nodes + 0x40);
}

/// `magic_0605` 0x1000a540 (`r31::spin_actors_0605`): two, one and no
/// records; `rand` varying the owners; the task list empty and not; in
/// and out of a battle, the caster below and above 0x20, `*G_C` set and
/// not; the others shown one by one and all at once.
#[test]
fn spin_actors_0605() {
    let Some(mut t) = twin(605) else { return };
    all_stubs(&mut t);
    float_matrix_stubs(&mut t);
    let f = t.at(0xa540);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 262, 4, |m, a| {
        let k = (a[0] & 0xff) as f32;
        wf(m, a[1], 2.0 * (k - 2.0));
        wf(m, a[2], 0.5 * k);
        wf(m, a[3], 3.0 - k);
        0x262
    });
    rec(&mut t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xf) * 0.75);
    for (s_, n_) in [
        (217u32, 2usize),
        (255, 4),
        (211, 2),
        (213, 2),
        (293, 1),
        (288, 2),
    ] {
        rec(&mut t, s_, n_, move |_, a| (s_ << 16) ^ a[0]);
    }
    // slot 296: one actor or each of group 0xb
    rec(&mut t, 296, 2, |_, a| 0x0296_0000 ^ a[0] ^ a[1]);
    let so = t.img.rel32_target(f + 0x20a);
    let show = t.img.rel32_target(f + 0x427);
    let scale = t.img.rel32_target(f + 0xff);
    for o in [0x28u32, 0x74] {
        let l = t.img.word(scale + o);
        t.set_u32(l, SCRATCH + 0x700);
    }
    for o in [0x33u32, 0x7f] {
        let l = t.img.word(scale + o);
        for i in 0..8 {
            t.set_u32(l + 4 * i, 0x0b00_0000 + i);
        }
    }
    let list = w(&t, 0x64);
    t.set_u32(list, SCRATCH + 0x700);
    t.write(SCRATCH + 0x700, &[3, 1, 4, 1, 5, 2, 6, 0]);
    for i in 0..8 {
        let at = w(&t, 0x6f) + 4 * i;
        t.set_u32(at, 0x0b00_0010 + 3 * i);
    }
    let s = SCRATCH + 0x1_0000;
    let (gs, gn) = (w(&t, 0x13), w(&t, 0x23));
    let ring = (w(&t, 0x287), w(&t, 0x293));
    let (tail, last_w) = (w(&t, 0x301), w(&t, 0x317));
    let (ga, gb, gself, gc, gd, ge) = (
        w(&t, 0x3a1),
        w(&t, 0x3b0),
        w(&t, 0x3a8),
        w(&t, 0x3f6),
        w(&t, 0x408),
        w(&t, 0x41e),
    );
    // the others shown: their actor word is `*G_B` (operand `+0x14`)
    assert_eq!(t.img.word(show + 0x14), gb);
    // (count, task list tail, battle, caster, *G_C, slot 293's answer)
    let cases: [(i32, u32, u32, u32, u16, u32); 5] = [
        (2, SCRATCH + 0x9800, 0, 3, 0, 0),
        (1, 0, 0, 0x20, 1, 1),
        (3, SCRATCH + 0x9800, 1, 5, 0, 1),
        (0, 0, 0, 0x1f, 0, 0),
        (-1, 0, 1, 0, 2, 0),
    ];
    for (k, (count, tl, battle, caster, cflag, any)) in (0u32..).zip(cases) {
        start_env_at(
            &mut t,
            so,
            (0x9d, 0xa8),
            6,
            if k == 1 { SCRATCH + 0x680 } else { 0 },
        );
        t.set_u32(gs, s);
        t.set_u32(gn, count.cast_unsigned());
        junk(&mut t, s + 0x800, 0x2600, k);
        floats(&mut t, s + 0x1850, 0x200, -6.0 + k as f32);
        free_ring(&mut t, SCRATCH + 0x9000, ring, 3, k);
        t.set_u32(tail, tl);
        junk(&mut t, SCRATCH + 0x9800, 0x10, k);
        t.set_u32(last_w, SCRATCH + 0x9800);
        t.set_u32(ga, battle);
        t.set_u32(gb, caster);
        t.set_u32(gself, 0x05e1_f000 + k);
        t.set_u16(gc, cflag);
        t.set_u32(gd, 0x0dd0_0000 + k);
        t.set_u32(ge, 0x0bad_0000 + k);
        rec(&mut t, 293, 1, move |_, _| any);
        t.stub(288, move |m, a| {
            m.w32(a[1], 2 + k);
            SCRATCH + 0x700
        });
        ownb::dirty_stack(&mut t, 0xa540 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// The 0x74-byte record pool `take` searches (`sb_misc::pool_take`): `n`
/// records at `recs`, those in `busy` taken, the search from `from`.
fn pool_74(t: &mut Twin, take: u32, recs: u32, n: u32, busy: &[u32], from: u32, k: u32) {
    let w = |t: &Twin, o: u32| t.img.word(take + o);
    let (p37, p61, n0f, n97, i08, pg) = (
        w(t, 0x37),
        w(t, 0x61),
        w(t, 0x0f),
        w(t, 0x97),
        w(t, 0x08),
        w(t, 0x2b),
    );
    t.set_u32(p37, recs);
    t.set_u32(p61, recs);
    for i in 0..n {
        junk(t, recs + 0x74 * i, 0x74, k + i);
        let taken = u32::from(busy.contains(&i));
        t.set_u32(recs + 0x74 * i + 0x6c, taken);
    }
    t.set_u32(n0f, n);
    t.set_u32(n97, n);
    t.set_u32(i08, from);
    t.set_u32(pg, 0);
}

/// `magic_0605` 0x1000a980 (`r31::sparks_0605`): all six records, the
/// pool running out after three and at once; the actors' mean from one
/// and three actors.
#[test]
fn sparks_0605() {
    let Some(mut t) = twin(605) else { return };
    all_stubs(&mut t);
    float_matrix_stubs(&mut t);
    let f = t.at(0xa980);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 262, 4, |m, a| {
        let k = (a[0] & 0xff) as f32;
        wf(m, a[1], 2.0 * (k - 2.0));
        wf(m, a[2], 0.5 * k);
        wf(m, a[3], 3.0 - k);
        0x262
    });
    rec(&mut t, 719, 3, |_, a| {
        0x0719_0000 ^ a[0] ^ (a[1] << 8) ^ (a[2] << 12)
    });
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    let mean = t.img.rel32_target(f + 0x32);
    let take = t.img.rel32_target(f + 0x41);
    let (cnt, tab, sel) = (
        t.img.word(mean + 0x11),
        t.img.word(mean + 0x58),
        t.img.word(mean + 0x4d),
    );
    t.set_u32(sel, SCRATCH + 0x700);
    t.write(SCRATCH + 0x700, &[3, 1, 4, 1, 5, 2, 6, 0]);
    for i in 0..8 {
        t.set_u32(tab + 4 * i, 0x0b00_0010 + 3 * i);
    }
    let s = SCRATCH + 0x1_0000;
    let gs = w(&t, 0x15);
    let recs = SCRATCH + 0x2_0000;
    // (actors, pool size, taken records, search start)
    let cases: [(u32, u32, &[u32], u32); 4] = [
        (3, 8, &[1, 6], 5),
        (1, 6, &[0, 2, 4], 1),
        (2, 2, &[0, 1], 0),
        (1, 7, &[], 0),
    ];
    for (k, (actors, n, busy, from)) in (0u32..).zip(cases) {
        t.set_u32(gs, s);
        floats(&mut t, s + 0x14a0, 8, 1.5 - k as f32);
        t.set_u32(cnt, actors);
        pool_74(&mut t, take, recs, n, busy, from, k);
        t.set_u32(SCRATCH + 0xa00, k);
        ownb::dirty_stack(&mut t, 0xa980 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// The cursor `c`, its node `node` and `children` child nodes (their
/// visit the slot `visit`'s trap through `class + 0x1c`) for a visit test;
/// the cursor's mark byte equal to the node's when `marked`.
fn visit_cursor(
    t: &mut Twin,
    k: u32,
    (c, node, class): (u32, u32, u32),
    children: usize,
    marked: bool,
) {
    let kids = [SCRATCH + 0x3200, SCRATCH + 0x3240];
    t.set_u8(node + 0x1a, 0xf0);
    t.set_u32(c, node);
    t.set_u8(c + 4, 0x55);
    t.set_u8(c + 5, if marked { 0xf0 } else { 3 + k as u8 });
    let mut prev = node;
    for &kid in kids.iter().take(children).rev() {
        t.write(kid, &[0x22; 0x40]);
        t.set_u32(kid, class);
        t.set_u32(kid + 8, prev);
        prev = kid;
    }
    t.set_u32(node + 0x10, prev);
}

/// The child visit stub (slot `visit`): logs the cursor's node and mark.
fn visit_stub(t: &mut Twin, visit: u32, class: u32) {
    t.stub(visit, move |m, a| {
        let k = m.r32(SCRATCH + 0x3800);
        m.w32(SCRATCH + 0x3800, k + 1);
        let cur = m.r32(a[0]);
        m.w32(SCRATCH + 0x3810 + 8 * (k % 8), cur);
        m.w32(SCRATCH + 0x3814 + 8 * (k % 8), m.r32(a[0] + 4));
        0x1000 + k
    });
    t.set_u32(class + 0x1c, magic::TRAPS + 16 * visit);
}

/// One `visit_0577` case: marked, children, flags `+0xe4`, the rate
/// `+0xcc`, the fade `+0xd0`, its end `+0xd4`, the top `+0xd8`, the scale
/// `+0xdc` and its step `+0xe0`.
#[derive(Clone, Copy)]
struct Visit {
    marked: bool,
    children: usize,
    flags: u32,
    rate: f32,
    fade: f32,
    end: f32,
    top: f32,
    scale: f32,
    step: f32,
}

/// `magic_0577` 0x10001fc0 (`r31::visit_0577`) directly and through its
/// thunk 0x100025e0: marked; the ease (bit 0); the fade moving and at its
/// end; the scale (bit 3, capped, bit 4); the top crossed both ways (bit
/// 1); detached and drawn (bit 2, the fade at the end).
#[test]
fn visit_0577() {
    let Some(mut t) = twin(577) else { return };
    all_stubs(&mut t);
    // the vectors' words folded into a word per slot (the stubs' answers
    // alone do not reach memory)
    for (s, n) in [(353u32, 1usize), (332, 1), (669, 1), (267, 1), (271, 1)] {
        rec(&mut t, s, n, move |m, a| {
            let mut x = (s << 16) ^ a[0];
            for i in 0..4 {
                x = x.rotate_left(3) ^ m.r32(a[0] + 4 * i);
            }
            let at = SCRATCH + 0xb00 + 4 * (s % 0x40);
            let y = m.r32(at).rotate_left(5) ^ x;
            m.w32(at, y);
            x
        });
    }
    let (visit, class) = (1000u32, SCRATCH + 0x3700);
    visit_stub(&mut t, visit, class);
    let f = t.at(0x1fc0);
    let (c, node) = (SCRATCH + 0x3000, SCRATCH + 0x3100);
    let v = |marked, children, flags, rate, fade, end, top, scale, step| Visit {
        marked,
        children,
        flags,
        rate,
        fade,
        end,
        top,
        scale,
        step,
    };
    let cases = [
        v(true, 0, 0, 0.5, 0.5, 2.0, 0.0, 0.0, 0.0),
        v(false, 2, 1, 0.25, 0.5, 2.0, -9.0, 0.0, 0.0),
        v(false, 1, 0, 0.25, 0.5, 2.0, 9.0, 0.0, 0.0),
        v(false, 0, 0, 0.75, 0.5, 2.0, -9.0, 0.0, 0.0),
        v(false, 0, 2, 0.25, 0.5, 2.0, -9.0, 0.0, 0.0),
        v(false, 0, 2, 0.25, 0.5, 2.0, 9.0, 0.0, 0.0),
        v(false, 0, 8, 0.25, 0.5, 2.0, 0.0, 0.5, 0.25),
        v(false, 0, 0x18, 0.25, 0.5, 2.0, 0.0, 0.75, 0.5),
        v(false, 0, 8, 0.25, 0.5, 2.0, 0.0, 0.75, 0.5),
        v(false, 0, 4, 0.75, 0.5, 1.0, 0.0, 0.0, 0.0),
        v(false, 0, 5, 0.25, 0.5, 0.5, 0.0, 0.0, 0.0),
        v(false, 0, 4, 0.25, 0.5, 2.0, 0.0, 0.0, 0.0),
        v(false, 0, 0, 0.25, f32::NAN, 2.0, 0.0, 0.0, 0.0),
        // the top equal to the highest axis (bit 1) and to the lowest
        v(false, 0, 2, 0.25, 0.5, 2.0, 100.0, 0.0, 0.0),
        v(false, 0, 0, 0.25, 0.5, 2.0, -100.0, 0.0, 0.0),
    ];
    let list = SCRATCH + 0x3600;
    for (k, p) in (0u32..).zip(cases) {
        for thunk in [false, true] {
            t.write(node, &[0x44; 0x100]);
            for i in 0..40u32 {
                let x = 0.25 * (k + i) as f32 - 1.5;
                t.set_u32(node + 0x1c + 4 * i, x.to_bits());
            }
            for (i, n) in [3i32, -2, 5, 7].into_iter().enumerate() {
                t.set_u32(node + 0xbc + 4 * i as u32, (n + k as i32).cast_unsigned());
            }
            // 100 / -100: the top on the third / first axis exactly
            let top = match p.top {
                100.0 => f32::from_bits(t.u32(node + 0x24)),
                -100.0 => f32::from_bits(t.u32(node + 0x1c)),
                x => x,
            };
            for (o, x) in [
                (0xccu32, p.rate),
                (0xd0, p.fade),
                (0xd4, p.end),
                (0xd8, top),
                (0xdc, p.scale),
                (0xe0, p.step),
            ] {
                t.set_u32(node + o, x.to_bits());
            }
            t.set_u32(node + 0xe4, p.flags | 0x0100);
            // the list the draw detaches the node from
            t.write(list, &[0x33; 0x20]);
            t.set_u32(node + 0x14, if k % 2 == 0 { list } else { 0 });
            t.set_u32(node + 4, list);
            t.set_u32(node + 8, list);
            visit_cursor(&mut t, k, (c, node, class), p.children, p.marked);
            ownb::dirty_stack(&mut t, 0x1fc0 + k, &[]);
            if thunk {
                t.check_entry(t.at(0x25e0), Call::default(), &[c], Eax::Checked);
            } else {
                t.check(f, Call::this(c), &[], Eax::Checked, |g, img, fr| {
                    ffx_magic::r31::visit_0577(g, img, fr)
                });
            }
        }
    }
}

/// A light visit case: marked, children, parts, place mode, time, colour
/// off, halfword `+0x98`, spin mode, spin step sign, flags `+0x12c`.
type LightCase = (bool, usize, usize, u8, f32, u8, u16, u8, f32, u32);

/// `magic_0577` 0x100038d0 (`r31::light_visit_0577`) directly and through
/// its thunk 0x10003de0: marked; both parts, one and none; the place mode
/// 0, 1 (time running and run out) and 2; the colour eased or not; the
/// halfword `+0x98` set or not; the spin mode 0, 1 (crossing 0 both ways)
/// and 2; flags bits 3 and 0.
#[test]
fn light_visit_0577() {
    let Some(mut t) = twin(577) else { return };
    all_stubs(&mut t);
    float_matrix_stubs(&mut t);
    for (s, n) in [(284u32, 3usize), (285, 2), (286, 2)] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a[0] ^ a[1].rotate_left(5)
        });
    }
    rec(&mut t, 233, 3, |m, a| {
        let q = f32::from_bits(a[2]);
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * (i % 4)) * q + i as f32;
            wf(m, a[0] + 4 * i, x);
        }
        0x233
    });
    rec(&mut t, 234, 3, |m, a| {
        for i in 0..3 {
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x234
    });
    let (visit, class) = (1000u32, SCRATCH + 0x3700);
    visit_stub(&mut t, visit, class);
    let f = t.at(0x38d0);
    let rot = t.img.rel32_target(f + 0x3ac);
    let sines = t.img.word(rot + 0x0c);
    // the sine table the rotation reads: 4096 entries
    let table = SCRATCH + 0x4_0000;
    for i in 0..0x1000u32 {
        let x = (i as f32 * std::f32::consts::TAU / 4096.0).sin();
        t.set_u32(table + 4 * i, x.to_bits());
    }
    t.set_u32(sines, table);
    let (c, node) = (SCRATCH + 0x3000, SCRATCH + 0x5000);
    let parts = [SCRATCH + 0x3400, SCRATCH + 0x3480];
    let cases: [LightCase; 10] = [
        (true, 0, 0, 0, 0.0, 0, 0, 0, 1.0, 0),
        (false, 1, 2, 0, 0.0, 0, 0, 0, 1.0, 9),
        (false, 0, 1, 1, 0.5, 1, 1, 1, 1.0, 8),
        (false, 0, 0, 1, 0.001, 0, 0, 1, -1.0, 1),
        (false, 2, 2, 2, 0.0, 1, 1, 2, 1.0, 9),
        (false, 0, 2, 0, 0.0, 0, 0, 1, 1.0, 0x19),
        (false, 0, 0, 1, 0.75, 0, 0, 1, -1.0, 9),
        (false, 0, 1, 3, 0.0, 0, 2, 0, 1.0, 1),
        (false, 0, 0, 0, 0.0, 0, 0, 1, f32::NAN, 9),
        (false, 0, 0, 0, 0.0, 0, 0, 1, 0.0, 9),
    ];
    for (k, (marked, children, nparts, place, time, coff, hw, spin, sign, flags)) in
        (0u32..).zip(cases)
    {
        for thunk in [false, true] {
            t.write(node, &[0x44; 0x140]);
            for i in 0..0x40u32 {
                let x = 0.25 * (k + i) as f32 - 3.0;
                t.set_u32(node + 0x1c + 4 * i, x.to_bits());
            }
            for (i, d) in [0x1234u16, 0xfe00, 0x0321].into_iter().enumerate() {
                t.set_u16(node + 0x8c + 2 * i as u32, d.wrapping_add(k as u16));
            }
            // spins and their steps: crossing 0 or not
            for i in 0..3u32 {
                let x = if i == 1 { 0.25 } else { -0.5 + 0.5 * k as f32 };
                t.set_u32(node + 0x6c + 4 * i, (x * sign.signum()).to_bits());
                t.set_u32(node + 0x7c + 4 * i, (0.375 * sign).to_bits());
            }
            t.set_u32(node + 0x11c, time.to_bits());
            t.set_u32(node + 0x120, 0.5f32.to_bits());
            t.set_u8(node + 0x134, spin);
            t.set_u8(node + 0x135, coff);
            t.set_u8(node + 0x136, place);
            t.set_u16(node + 0x98, hw);
            t.set_u32(node + 0x12c, flags);
            t.set_u32(node + 0x130, 0x0130_0000 + k);
            for (i, &p) in parts.iter().enumerate() {
                let on = i < nparts;
                t.set_u32(node + 0x124 + 4 * i as u32, if on { p } else { 0 });
                t.write(p, &[0x11; 0x40]);
                t.set_u32(p, class);
                for j in 0..4u32 {
                    t.set_u32(p + 0x1c + 4 * j, (1.5 * j as f32 - k as f32).to_bits());
                }
            }
            visit_cursor(&mut t, k, (c, node, class), children, marked);
            ownb::dirty_stack(&mut t, 0x38d0 + k, &[]);
            if thunk {
                t.check_entry(t.at(0x3de0), Call::default(), &[c], Eax::Checked);
            } else {
                t.check(f, Call::this(c), &[], Eax::Checked, |g, img, fr| {
                    ffx_magic::r31::light_visit_0577(g, img, fr)
                });
            }
        }
    }
}

/// `magic_0577` 0x1000ac90 (`r31::motion_task_0577`): every state of the
/// table and the ones above it, each waiting and at its timer's end, the
/// motion ended and not, the model's flag set and not, the shake still
/// going and done, and states the table leaves alone.
#[test]
fn motion_task_0577() {
    let Some(mut t) = twin(577) else { return };
    all_stubs(&mut t);
    float_matrix_stubs(&mut t);
    // slot 411: the answer at `SCRATCH + 0xa10`
    rec(&mut t, 411, 1, |m, a| {
        m.r32(SCRATCH + 0xa10) ^ (a[0] & 0x100)
    });
    for (s, n) in [
        (228u32, 2usize),
        (257, 2),
        (296, 2),
        (325, 2),
        (213, 2),
        (211, 2),
        (255, 4),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a[0] ^ a[1].rotate_left(7)
        });
    }
    rec(&mut t, 248, 3, |m, a| {
        for i in 0..3 {
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x248
    });
    let f = t.at(0xac90);
    let (a, s, o, b) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x2000,
        SCRATCH + 0x3000,
    );
    let (pts, body, node) = (SCRATCH + 0x3400, SCRATCH + 0x3500, SCRATCH + 0x3600);
    // (state, timer, slot 411's answer, the model's flag, shake, step)
    let cases: [(u32, i32, u32, u8, i32, i32); 31] = [
        (0, 5, 0, 0, 0, 0),
        (1, 3, 0, 0, 0, 0),
        (1, 0, 0, 0, 0, 0),
        (2, 0, 0, 0, 0, 0),
        (2, 0, 1, 0, 0, 0),
        (30, 4, 0, 0, 0, 0),
        (31, 2, 0, 0, 0, 0),
        (31, -1, 0, 0, 0, 0),
        (32, 0, 0, 1, 0, 0),
        (32, 0, 1, 1, 0, 0),
        (32, 0, 1, 0, 0, 0),
        (10, 0, 0, 0, 0, 0),
        (11, 1, 0, 0, 0, 0),
        (11, 0, 0, 1, 0, 0),
        (11, 0, 0, 0, 0, 0),
        (12, 2, 0, 0, 0x100, 0x40),
        (12, 0, 0, 0, 0x100, -0x200),
        (12, 0, 0, 0, -0x300, 0x80),
        (12, 0, 0, 0, 0x100, 0x40),
        (13, 3, 0, 0, 0x50, 0x28),
        (13, 0, 0, 0, 0x50, -0x30),
        (14, 0, 0, 0, 0, 0),
        (14, 0, 1, 1, 0, 0),
        (0x2710, 0, 0, 0, 0, 0),
        (0x2711, 2, 0, 0, 0, 0),
        (0x2711, 0, 0, 0, 0, 0),
        (0x2712, 0, 0, 0, 0, 0),
        (0x2712, 0, 1, 0, 0, 0),
        (0x2715, 0, 0, 0, 0, 0),
        (5, 0, 0, 0, 0, 0),
        (100, 7, 0, 0, 0, 0),
    ];
    for (k, (state, timer, answer, flag, shake, step)) in (0u32..).zip(cases) {
        junk(&mut t, s, 0x60, k);
        t.set_u32(a, node);
        t.set_u32(a + 4, s);
        // the task's node in a ring of three
        let (n0, n1) = (SCRATCH + 0x3700, SCRATCH + 0x3740);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        t.set_u32(s + 0x28, state);
        t.set_u32(s + 0x2c, timer.cast_unsigned());
        t.set_u32(s + 0x30, shake.cast_unsigned());
        t.set_u32(s + 0x34, step.cast_unsigned());
        t.set_u32(s + 0x38, 0x0038_0000 + k);
        t.set_u32(s + 0x40, (1.5f32 + k as f32).to_bits());
        t.set_u32(s + 0x44, (-0.75f32).to_bits());
        t.set_u32(s + 0x48, o);
        t.set_u32(s + 0x4c, b);
        junk(&mut t, o, 0x100, k + 1);
        t.set_u32(o + 0x50, 0x0050_0000 + k);
        t.set_u32(o + 0x54, 0x0054_0000 + k);
        t.set_u8(o + 0x90, flag | 0x10);
        t.set_u32(o + 0x4c, SCRATCH + 0x3800);
        junk(&mut t, b, 0x150, k + 2);
        floats(&mut t, b + 0x9c, 16, 0.5 - k as f32);
        t.set_u32(b + 0x13c, pts);
        t.set_u32(b + 0x144, body);
        floats(&mut t, pts, 16, -1.0 + k as f32);
        junk(&mut t, body, 0x40, k + 3);
        t.set_u32(SCRATCH + 0xa10, answer);
        ownb::dirty_stack(&mut t, 0xac90 + k, &[]);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}

/// The vector slots of `magic_0512`'s visits as float stubs: 232
/// `out = in / 2`, 233 `out = in s`, 234 `out = a - b`, 266 `out = a x b`
/// (cross), 230 `out = m[0..4] + v`.
fn vector_stubs(t: &mut Twin) {
    rec(t, 232, 2, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * 0.5;
            wf(m, a[0] + 4 * i, x);
        }
        0x232
    });
    rec(t, 233, 3, |m, a| {
        let s = f32::from_bits(a[2]);
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * s;
            wf(m, a[0] + 4 * i, x);
        }
        0x233
    });
    rec(t, 234, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x234
    });
    rec(t, 266, 3, |m, a| {
        let u = [0, 1, 2].map(|i| rf(m, a[1] + 4 * i));
        let v = [0, 1, 2].map(|i| rf(m, a[2] + 4 * i));
        let c = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        for (i, x) in (0u32..).zip(c) {
            wf(m, a[0] + 4 * i, x);
        }
        0x266
    });
    rec(t, 230, 2, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[0] + 4 * i) + rf(m, a[1] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x230
    });
}

/// `magic_0512` 0x100037a0 (`r31::aim_visit_0512`) directly and through
/// its thunk 0x10003b50: marked; children; the flat part on and off; the
/// blend `t` 0, between and 1.
#[test]
fn aim_visit_0512() {
    let Some(mut t) = twin(512) else { return };
    all_stubs(&mut t);
    vector_stubs(&mut t);
    rec(&mut t, 325, 2, |_, a| {
        0x0325_0000 ^ a[0] ^ a[1].rotate_left(5)
    });
    let (visit, class) = (1000u32, SCRATCH + 0x3700);
    visit_stub(&mut t, visit, class);
    let f = t.at(0x37a0);
    let (c, node) = (SCRATCH + 0x3000, SCRATCH + 0x5000);
    let parts = [SCRATCH + 0x5400, SCRATCH + 0x5480, SCRATCH + 0x5500];
    // (marked, children, flat, t)
    let cases: [(bool, usize, u32, f32); 6] = [
        (true, 0, 0, 0.5),
        (false, 2, 0, 0.5),
        (false, 0, 1, 0.25),
        (false, 1, 1, 0.0),
        (false, 0, 0, 1.0),
        (false, 0, 7, 0.75),
    ];
    for (k, (marked, children, flat, tw)) in (0u32..).zip(cases) {
        for thunk in [false, true] {
            t.write(node, &[0x44; 0x160]);
            floats(&mut t, node + 0x1c, 0x48, 0.5 - k as f32);
            for (i, &p) in (0u32..).zip(&parts) {
                t.write(p, &[0x11; 0x40]);
                t.set_u32(p, class);
                for j in 0..4u32 {
                    let x = 1.5 * j as f32 - (k + i) as f32 + 0.25 * (i * j) as f32;
                    t.set_u32(p + 0x1c + 4 * j, x.to_bits());
                }
            }
            t.set_u32(node + 0x13c, parts[0]);
            t.set_u32(node + 0x154, parts[1]);
            t.set_u32(node + 0x140, parts[2]);
            t.set_u32(node + 0x150, flat);
            t.set_u32(node + 0x14c, tw.to_bits());
            t.set_u32(node + 0x138, (2.0f32 - k as f32).to_bits());
            visit_cursor(&mut t, k, (c, node, class), children, marked);
            ownb::dirty_stack(&mut t, 0x37a0 + k, &[]);
            if thunk {
                t.check_entry(t.at(0x3b50), Call::default(), &[c], Eax::Checked);
            } else {
                t.check(f, Call::this(c), &[], Eax::Checked, |g, img, fr| {
                    ffx_magic::r31::aim_visit_0512(g, img, fr)
                });
            }
        }
    }
}

/// `magic_0512` 0x10006aa0 (`r31::matrix_product_0512`): the product
/// into another block and into its own source, a NaN and infinities in
/// the source.
#[test]
fn matrix_product_0512() {
    let Some(mut t) = twin(512) else { return };
    let f = t.at(0x6aa0);
    let m = t.img.word(f + 0x2b);
    let (a, out) = (SCRATCH + 0x100, SCRATCH + 0x200);
    for k in 0..4u32 {
        for i in 0..16u32 {
            let x = 0.375 * (i as f32) - 2.0 + 0.125 * k as f32;
            t.set_u32(m + 4 * i, x.to_bits());
            let y = 1.25 - 0.5 * ((i * 7 + k) % 9) as f32;
            t.set_u32(a + 4 * i, y.to_bits());
        }
        if k == 2 {
            t.set_u32(a + 4 * 5, f32::NAN.to_bits());
            t.set_u32(a + 4 * 15, f32::INFINITY.to_bits());
            t.set_u32(m + 4 * 15, f32::NEG_INFINITY.to_bits());
        }
        t.write(out, &[0x5a; 0x40]);
        let dst = if k == 3 { a } else { out };
        t.check(f, Call::this(dst), &[a], Eax::Checked, |g, img, fr| {
            ffx_magic::r31::matrix_product_0512(g, img, fr)
        });
    }
}

/// `magic_0512` 0x10009150 (`r31::object_setup_0512`): `b` 0 and 1; the
/// model's flag set (slot 333) and not (slot 601); the sizes below 10,
/// between and above 18; the parent's child list empty and not.
#[test]
fn object_setup_0512() {
    let Some(mut t) = twin(512) else { return };
    all_stubs(&mut t);
    let f = t.at(0x9150);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            wf(m, a[2] + 4 * i, 0.5 * i as f32 + (a[0] & 7) as f32);
        }
        0x268
    });
    rec(&mut t, 601, 2, |m, a| {
        wf(m, a[1], 0.25 * (a[0] & 0xf) as f32 - 1.0);
        0x601
    });
    for s in [12u32, 333, 251] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xf) * 0.5);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xf) * 0.25 + 0.5);
    // slot 251: the size from a counter (5, 12, 25, ...)
    t.stub_st0(251, |m, _| {
        let k = m.r32(SCRATCH + 0xa00);
        m.w32(SCRATCH + 0xa00, k + 1);
        [5.0, 12.0, 25.0, 18.0, 10.0][(k % 5) as usize]
    });
    let (obj, x, a) = (SCRATCH + 0x6000, SCRATCH + 0x7000, SCRATCH + 0x7200);
    let parent = w(&t, 0x5c);
    let tail = w(&t, 0x24);
    // (b, the model's flag, the parent's list empty)
    let cases: [(u32, u8, bool); 4] = [(0, 0, true), (1, 1, false), (0, 1, false), (1, 0, true)];
    for (k, (b, flag, empty)) in (0u32..).zip(cases) {
        junk(&mut t, obj, 0x400, k);
        junk(&mut t, x, 0x100, k + 1);
        junk(&mut t, a, 0x100, k + 2);
        t.set_u32(x + 0x50, 0x0050_0000 + k);
        t.set_u32(x + 0x54, 0x0054_0003 + k);
        t.set_u8(a + 0x90, flag);
        t.set_u32(a + 0x50, 0x0a50_0005 + k);
        t.set_u32(a + 0x54, 0x0a54_0006 + k);
        junk(&mut t, SCRATCH + 0x7400, 0x20, k);
        t.set_u32(tail, SCRATCH + 0x7400);
        t.set_u32(w(&t, 0x42), 0xffff_0003 + k);
        // the parent: empty, or with one child
        let kid = SCRATCH + 0x7500;
        junk(&mut t, kid, 0x20, k + 3);
        t.set_u32(parent + 0x10, if empty { parent } else { kid });
        t.set_u32(parent + 0xc, if empty { parent } else { kid });
        t.set_u16(parent + 0x18, 2 + k as u16);
        t.set_u32(SCRATCH + 0xa00, k);
        ownb::dirty_stack(&mut t, 0x9150 + k, &[]);
        let c = Call {
            ecx: obj,
            edx: x,
            ..Call::default()
        };
        t.check(f, c, &[a, b], Eax::Checked, |g, img, fr| {
            ffx_magic::r31::object_setup_0512(g, img, fr)
        });
    }
}
