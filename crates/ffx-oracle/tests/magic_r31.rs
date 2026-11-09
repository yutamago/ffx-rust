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
    let me = t.img.word(show + 0x14);
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
        t.set_u32(me, 0x0000_0e00 + k);
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
