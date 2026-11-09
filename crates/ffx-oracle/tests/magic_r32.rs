//! Round 32's ports (`ffx_magic::r32`) against each original. Needs
//! `FFX_GAME_DIR`.

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
mod magic_h6;
mod magic_hi;
mod magic_owna;
mod magic_r11;
#[path = "magic/ownb.rs"]
mod ownb;

use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use ownb::rec;

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    magic_h6::scrub(&mut t);
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
/// `out = row0 + row3 + 2 v`; the scale slots 15/16/17/277 and the random
/// 404/314 floats in ST0.
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

/// The object start `so` (`s4_misc::start_object_at`) ready to run: its
/// pool with `count` free nodes, the task list's tail `tl`.
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

/// `magic_0307` 0x10007000 (`r32::spin_actors_0307`, `r31`'s 0605 actors
/// on 0307's records): two, one, three and no records; `rand` varying the
/// owners; the task list empty and not; in and out of a battle, the
/// caster below and above 0x20, `*G_C` set and not.
#[test]
fn spin_actors_0307() {
    let Some(mut t) = twin(307) else { return };
    all_stubs(&mut t);
    float_matrix_stubs(&mut t);
    let f = t.at(0x7000);
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
    rec(&mut t, 296, 2, |_, a| 0x0296_0000 ^ a[0] ^ a[1]);
    let so = t.img.rel32_target(f + 0x210);
    let show = t.img.rel32_target(f + 0x429);
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
    let (gs, gn) = (w(&t, 0x18), w(&t, 0x11));
    let ring = (w(&t, 0x289), w(&t, 0x295));
    let (tail, last_w) = (w(&t, 0x303), w(&t, 0x319));
    let (ga, gb, gself, gc, gd, ge) = (
        w(&t, 0x3a3),
        w(&t, 0x3b2),
        w(&t, 0x3aa),
        w(&t, 0x3f8),
        w(&t, 0x40a),
        w(&t, 0x420),
    );
    // show_others' operand `+0x14` is the caster word `gb` itself (0307's
    // `*G_B`), so the caster set below is also what it passes
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
        junk(&mut t, s, 0x2500, k);
        floats(&mut t, s + 0x1000, 0x200, -6.0 + k as f32);
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
        ownb::dirty_stack(&mut t, 0x7000 + k, &[]);
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

/// `magic_0307` 0x10007440 (`r32::sparks_0307`): all six records, the
/// pool running out after three and at once; the actors' mean from one
/// and three actors; the part table's two words varied per case.
#[test]
fn sparks_0307() {
    let Some(mut t) = twin(307) else { return };
    all_stubs(&mut t);
    float_matrix_stubs(&mut t);
    let f = t.at(0x7440);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 262, 4, |m, a| {
        let k = (a[0] & 0xff) as f32;
        wf(m, a[1], 2.0 * (k - 2.0));
        wf(m, a[2], 0.5 * k);
        wf(m, a[3], 3.0 - k);
        0x262
    });
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
    let (gp1, gp2) = (w(&t, 0x51), w(&t, 0x2bc));
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
        floats(&mut t, s + 0xc50, 8, 1.5 - k as f32);
        t.set_u32(cnt, actors);
        t.set_u32(gp1, SCRATCH + 0xb00 + 0x10 * k);
        t.set_u32(gp2, SCRATCH + 0xb04 + 0x10 * k);
        for i in 0..0x10 {
            t.set_u32(SCRATCH + 0xb00 + 4 * i, 0x0a00_0000 + 0x101 * i + k);
        }
        pool_74(&mut t, take, recs, n, busy, from, k);
        t.set_u32(SCRATCH + 0xa00, k);
        ownb::dirty_stack(&mut t, 0x7440 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0307` 0x10003c70 (`r32::spin_start_0307`): three, one, two and
/// no records; the pieces far (scaled down) and near; the yaws wrapped
/// both ways; the free ring with one node and more, the task list empty
/// and not.
#[test]
fn spin_start_0307() {
    let Some(mut t) = twin(307) else { return };
    all_stubs(&mut t);
    float_matrix_stubs(&mut t);
    let f = t.at(0x3c70);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    // slot 262: the actor's place from its object word
    rec(&mut t, 262, 4, |m, a| {
        let k = (a[0] & 0xff) as f32;
        let scale = rf(m, SCRATCH + 0xa04);
        wf(m, a[1], scale * (k - 2.0));
        wf(m, a[2], 0.5 * k + 0.375);
        wf(m, a[3], scale * (3.0 - k));
        0x262
    });
    for s in [12u32, 333] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xf) * 0.625);
    t.stub_st0(333, |m, a| {
        f64::from(rf(m, SCRATCH + 0xa08)) * f64::from((a[0] & 0xff) + 1)
    });
    rec(&mut t, 528, 2, |_, a| 0x0528_0000 ^ a[0] ^ a[1]);
    let mode = t.img.rel32_target(f + 0x315);
    for o in [0x54u32, 0x326] {
        let l = w(&t, o);
        t.set_u32(l, SCRATCH + 0x700);
    }
    t.write(SCRATCH + 0x700, &[3, 1, 4, 1, 5, 2, 6, 0]);
    for i in 0..8 {
        let at = w(&t, 0x5f) + 4 * i;
        t.set_u32(at, 0x0b00_0010 + 3 * i);
    }
    let l = t.img.word(mode + 0x21);
    t.set_u32(l, SCRATCH + 0x700);
    let l = t.img.word(mode + 0x2e);
    for i in 0..8 {
        t.set_u32(l + 4 * i, 0x0c00_0000 + i);
    }
    let s = SCRATCH + 0x1_0000;
    let (gs, gn) = (w(&t, 0x1c), w(&t, 0x15));
    let (free, head, tail, last) = (w(&t, 0x3a1), w(&t, 0x3b0), w(&t, 0x417), w(&t, 0x426));
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
        junk(&mut t, s + 0x1000, 0x600, k);
        junk(&mut t, s + 0x2800, 0x100, k + 1);
        floats(&mut t, s + 0xc50, 4, 0.75 - k as f32);
        t.set_u32(SCRATCH + 0xa04, scale.to_bits());
        t.set_u32(SCRATCH + 0xa08, step.to_bits());
        t.set_u32(SCRATCH + 0xa00, k);
        free_ring(&mut t, nodes, (free, head), nfree, k);
        t.set_u32(tail, tl);
        junk(&mut t, SCRATCH + 0x9800, 0x10, k);
        t.set_u32(last, SCRATCH + 0x9800);
        ownb::dirty_stack(&mut t, 0x3c70 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0307` 0x10003350 (`r32::aim_objects_0307`) and through it
/// 0x10003000 (`r32::aim_object_0307`): the bone points around the target
/// (the yaw wrapped both ways and kept), the task list empty and not, the
/// object pool with nodes and empty.
#[test]
fn aim_objects_0307() {
    let Some(mut t) = twin(307) else { return };
    all_stubs(&mut t);
    let bones = SCRATCH + 0x7a0;
    rec(&mut t, 237, 4, move |m, a| {
        for i in 0..4 {
            let v = rf(m, bones + 4 * i) + (a[1] & 0x3f) as f32 * 0.25 * (i as f32 - 1.0);
            wf(m, a[3] + 4 * i, v);
        }
        0x237
    });
    magic_owna::filler(&mut t, 235, 1, 16);
    for s in [328u32, 329] {
        rec(&mut t, s, 3, move |m, a| {
            for i in 0..16 {
                let v =
                    rf(m, a[1] + 4 * i) * 0.75 + f32::from_bits(a[2]) * 0.125 + s as f32 * 0.001;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(&mut t, 229, 3, |m, a| {
        let sum: f32 = (0..4).map(|i| rf(m, a[2] + 4 * i) * (i as f32 + 1.0)).sum();
        for i in 0..4 {
            let v = rf(m, a[1] + 0x10 * i) * 0.25 + 0.125 * sum + i as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    let f = t.at(0x3350);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let aim = t.img.rel32_target(f + 0x57);
    let so = t.img.rel32_target(aim + 0x30b);
    let take = t.img.rel32_target(f + 0x9d);
    let s = SCRATCH + 0x1_0000;
    let last = SCRATCH + 0x3400;
    // (the bone point, the target, listed, object pool count)
    let cases: [([f32; 3], [f32; 3], bool, u16); 4] = [
        ([1.0, 2.0, 3.0], [4.0, -1.0, -5.0], false, 3),
        ([1.0, 2.0, 3.0], [-4.0, 1.0, 0.5], true, 3),
        ([-2.0, 0.0, 1.0], [-6.0, 3.0, -0.5], true, 0),
        ([0.5, 0.5, 0.5], [0.5, 7.0, 9.5], false, 1),
    ];
    for (k, (b, q, listed, count)) in (0u32..).zip(cases) {
        start_env_at(
            &mut t,
            so,
            (0x9d, 0xa8),
            count,
            if k == 1 { SCRATCH + 0x680 } else { 0 },
        );
        t.set_u32(op(&t, 0x13), s);
        junk(&mut t, s + 0xc00, 0x1800, k);
        for (i, x) in (0u32..).zip(b.into_iter().chain([1.0])) {
            t.set_u32(bones + 4 * i, x.to_bits());
        }
        for (i, x) in (0u32..).zip(q.into_iter().chain([0.75])) {
            t.set_u32(s + 0xc40 + 4 * i, x.to_bits());
        }
        t.set_u32(op(&t, 0x2e), 0x00ac_0000 + k);
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
        t.write(last, &[0x34; 0x20]);
        t.set_u32(op(&t, 0xa3), if listed { last } else { 0 });
        t.set_u32(op(&t, 0xb6), last);
        ownb::dirty_stack(&mut t, 0x3350 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// The matrix slots as `magic_r29.rs`'s `matrix_stubs` (bit mixes, no
/// float maths: no NaNs meet): 235 a unit matrix, 330/328/329 mixing
/// their angle into the source, 229/248 every word read, the scale slots
/// 15/16/17/277 floats.
fn matrix_stubs(t: &mut Twin) {
    for (s, x) in [(15u32, 1.5f64), (16, -0.75), (17, 3.0), (277, 40.0)] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    for s in [330u32, 328, 329] {
        rec(t, s, 3, move |m, a| {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i);
                m.w32(a[0] + 4 * i, v ^ (a[2] & 0x00ff_0000) ^ (s << 8));
            }
            s
        });
    }
    rec(t, 229, 3, |m, a| {
        let mix = (0..4).fold(0u32, |x, i| x.rotate_left(7) ^ m.r32(a[2] + 4 * i));
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i)
                ^ m.r32(a[1] + 0x30 + 4 * i)
                ^ m.r32(a[2] + 4 * i).rotate_left(i + 1)
                ^ mix.rotate_left(i);
            m.w32(a[0] + 4 * i, v);
        }
        0x229
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i).rotate_left(3);
            m.w32(a[0] + 4 * i, v);
        }
        0x248
    });
}

/// A mesh at `m` for the mesh centres (as `magic_r29.rs`'s `mesh`): a
/// header, 32 points and one to three packets of faces.
fn mesh(t: &mut Twin, m: u32, k: u32) {
    t.write(m, &[0x5a; 0x400]);
    t.set_u32(m + 4, 0x40);
    t.set_u32(m + 8, 0x300);
    for i in 0..32u32 {
        for c in 0..3u32 {
            let x = ((i * 37 + c * 11 + k * 5) % 200) as i16 - 100;
            t.set_u16(m + 0x300 + 6 * i + 2 * c, x.cast_unsigned());
        }
    }
    // (type, count, face size, index offset, corners)
    let packets: &[(u8, u16, u32, u32, u32)] = match k % 3 {
        0 => &[(6, 2, 0x28, 0x10, 4), (2, 3, 0x20, 0xc, 3)],
        1 => &[(2, 1, 0x20, 0xc, 3), (5, 1, 0, 0, 0), (6, 1, 0x28, 0x10, 4)],
        _ => &[(6, 3, 0x28, 0x10, 4)],
    };
    let mut p = m + 0x40;
    let mut total = 0u16;
    for (j, &(kind, n, size, first, corners)) in (0u32..).zip(packets) {
        t.set_u8(p + 1, kind);
        t.set_u16(p + 2, n);
        total += n;
        let mut face = p + 0x10;
        for q in 0..u32::from(n) {
            for corner in 0..corners {
                let index = (q * 7 + corner * 3 + j + k) % 32;
                t.set_u16(face + first + 2 * corner, index as u16);
            }
            face += size;
        }
        p = face;
    }
    t.set_u16(m + 0x10, total);
}

/// `magic_0307` 0x100012f0 (`r32::load_0307`, the 118-byte shape) running
/// `r32::start_up_0307`: slot 225 busy, the data not loaded yet, the
/// start-up with the task list empty and not, and slot 520's battle ids
/// moving the corner globals each way (4 and 6 of group 0x24e, group
/// 0x1eb, another of 0x24e, another group).
#[test]
fn load_start_up_0307() {
    let Some(mut t) = twin(307) else { return };
    all_stubs(&mut t);
    matrix_stubs(&mut t);
    rec(&mut t, 225, 1, |m, _| m.r32(SCRATCH + 0x10));
    rec(&mut t, 226, 2, |_, a| 0x226 ^ a[0]);
    rec(&mut t, 227, 1, |m, _| m.r32(SCRATCH + 0x1c));
    rec(&mut t, 520, 0, |m, _| m.r32(SCRATCH + 0x20));
    for (s, n) in [(718u32, 1usize), (719, 3), (711, 3), (710, 4), (271, 3)] {
        rec(&mut t, s, n, move |_, a| (0x5a5a_0000 + s) ^ a[0]);
    }
    let (cx, qx) = (SCRATCH + 0x7a0, SCRATCH + 0x7c0);
    rec(&mut t, 262, 4, move |m, a| {
        for i in 0..3 {
            m.w32(a[1 + i], m.r32(cx + 4 * i as u32));
        }
        0x262
    });
    rec(&mut t, 292, 1, move |m, a| {
        for i in 0..3 {
            m.w32(a[0] + 4 * i, m.r32(qx + 4 * i));
        }
        0x292
    });
    rec(&mut t, 229, 3, |m, a| {
        let sum: f32 = (0..4).map(|i| rf(m, a[2] + 4 * i) * (i as f32 + 1.0)).sum();
        for i in 0..4 {
            let v = rf(m, a[1] + 0x10 * i) * 0.25 + 0.125 * sum + i as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    t.stub(248, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x248
    });
    t.stub(564, |m, a| {
        let k = f32::from_bits(a[2]);
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) / k;
            wf(m, a[0] + 4 * i, x);
        }
        0x564
    });
    rec(&mut t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xf) * -0.5);
    let f = t.at(0x12f0);
    let w = |t: &Twin, at: u32, o: u32| t.img.word(at + o);
    let su = t.img.rel32_target(f + 0x5b);
    let fp = t.img.rel32_target(su + 0x6d);
    let mc = t.img.rel32_target(fp + 0x71a);
    let lock = SCRATCH + 0x7f0;
    let pi = t.img.rel32_target(su + 0x68);
    t.set_u32(w(&t, pi, 0x04), lock);
    let take = t.img.rel32_target(su + 0xb2);
    let ri = t.img.rel32_target(su + 0xf0);
    let (table, meshes) = (SCRATCH + 0xc000, SCRATCH + 0x1_8000);
    let s = SCRATCH + 0x1_0000;
    // the corner globals: varied floats
    let corners = [0x1d7u32, 0x1eb, 0x1ff, 0x213, 0x227];
    // (225, asked, 227, task list, slot 520's id)
    let cases: [(u32, u32, u32, bool, u32); 8] = [
        (1, 0, 1, false, 0),
        (0, 0, 0, false, 0),
        (0, 1, 1, false, 0x24e_0004),
        (0, 0, 2, true, 0x24e_0006),
        (0, 1, 1, true, 0x1eb_0003),
        (0, 1, 1, false, 0x24e_0001),
        (0, 1, 1, false, 0x24f_0000),
        (0, 1, 1, true, 0x1eb_0000),
    ];
    for (k, (w225, asked, r227, busy, id)) in (0u32..).zip(cases) {
        t.set_u32(SCRATCH + 0x10, w225);
        t.set_u32(SCRATCH + 0x1c, r227);
        t.set_u32(SCRATCH + 0x20, id);
        t.set_u32(w(&t, f, 0x18), asked);
        t.set_u32(lock, 0x77 + k);
        for (i, &o) in (0u32..).zip(&corners) {
            t.set_u32(w(&t, su, o), (1.5 + i as f32 - 0.25 * k as f32).to_bits());
        }
        junk(&mut t, s + 0x400, 0x2e00, k);
        for i in 0..3u32 {
            t.set_u32(cx + 4 * i, (1.0 + i as f32).to_bits());
            t.set_u32(qx + 4 * i, (4.0 - 3.0 * i as f32 + k as f32).to_bits());
        }
        t.set_u32(w(&t, mc, 0x31), table);
        for j in 0..18u32 {
            let mm = meshes + 0x400 * j;
            t.set_u32(table + 0x20 * j + w(&t, mc, 0x38), mm);
            mesh(&mut t, mm, j + k);
        }
        let node = |i: u32| SCRATCH + 0x5000 + 0x40 * i;
        for i in 0..3u32 {
            t.write(node(i), &[0x33; 0x40]);
            t.set_u32(node(i), node((i + 2) % 3));
            t.set_u32(node(i) + 4, node((i + 1) % 3));
        }
        t.set_u32(w(&t, ri, 0x3e), node(0));
        t.set_u16(w(&t, ri, 0x20), 3);
        t.set_u32(w(&t, ri, 0xa6), 0);
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
        let last = SCRATCH + 0x3400;
        t.write(last, &[0x34; 0x20]);
        t.set_u32(w(&t, su, 0xb8), if busy { last } else { 0 });
        t.set_u32(w(&t, su, 0xcc), last);
        ownb::dirty_stack(&mut t, 0x12f0 + k, &[0xa4 + 0x10]);
        ownb::reset_counters(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// Finite float stubs for the bone offset (`r32::bone_offset_0307`):
/// slot 237 a bone point from its bone number, 235 a unit matrix,
/// 328/329 rows mixed with the angle, 229 rows and the vector, 248 a sum.
fn bone_stubs(t: &mut Twin) {
    rec(t, 237, 4, |m, a| {
        for i in 0..4 {
            wf(m, a[3] + 4 * i, (a[1] & 0x7f) as f32 * 0.25 - i as f32);
        }
        0x237
    });
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    for s in [328u32, 329] {
        rec(t, s, 3, move |m, a| {
            for i in 0..16 {
                let v =
                    rf(m, a[1] + 4 * i) * 0.75 + f32::from_bits(a[2]) * 0.125 + s as f32 * 0.001;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 229, 3, |m, a| {
        let sum: f32 = (0..4).map(|i| rf(m, a[2] + 4 * i) * (i as f32 + 1.0)).sum();
        for i in 0..4 {
            let v = rf(m, a[1] + 0x10 * i) * 0.25 + 0.125 * sum + i as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + 2.0 * rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x0248_0000 ^ a[0]
    });
}

/// `magic_0307` 0x100045b0 (`r32::late_bone_offset_0307`) and through it
/// 0x100044c0 (`r32::bone_offset_0307`): the object's `+0x118` above,
/// at and below 0x3a; the turn angles varied.
#[test]
fn late_bone_offset_0307() {
    let Some(mut t) = twin(307) else { return };
    all_stubs(&mut t);
    bone_stubs(&mut t);
    let f = t.at(0x45b0);
    let bo = t.img.rel32_target(f + 0x23);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(bo + 0x1e), s);
    let o = SCRATCH + 0x2_0000;
    for (k, n) in (0u32..).zip([0x3bu32, 0x3a, 0xffff_ff00, 0x7fff_ffff]) {
        floats(&mut t, s + 0xc00, 0x20, 0.5 - k as f32);
        floats(&mut t, o, 0x50, 1.0 + k as f32);
        t.set_u32(o + 0x118, n);
        t.set_u32(t.img.word(bo + 0x2c), 0x00ac_0000 + k);
        ownb::dirty_stack(&mut t, 0x45b0 + k, &[]);
        let regs = Call {
            eax: 0x00ea_0000 + k,
            ..Call::default()
        };
        t.check_entry(f, regs, &[o], Eax::Checked);
    }
}

/// `magic_0307` 0x100045e0 (`r32::swing_object_0307`): the count 0 and
/// below (the plain scale), 1 (the last blend step: slot 233 and the
/// fader's node, the list empty and not) and above.
#[test]
fn swing_object_0307() {
    let Some(mut t) = twin(307) else { return };
    all_stubs(&mut t);
    bone_stubs(&mut t);
    rec(&mut t, 233, 3, |_, a| 0x0233_0000 ^ a[0] ^ a[2]);
    let f = t.at(0x45e0);
    let bo = t.img.rel32_target(f + 0xa9);
    let nd = t.img.rel32_target(f + 0x9e);
    let take = t.img.rel32_target(nd + 0x15);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0x6), s);
    let o = SCRATCH + 0x2_0000;
    // (count, listed)
    for (k, (n, listed)) in (0u32..).zip([
        (0u32, false),
        (0xffff_fffe, false),
        (1, false),
        (1, true),
        (7, true),
        (0x40, false),
    ]) {
        floats(&mut t, s + 0xc00, 0x20, 0.5 - k as f32);
        floats(&mut t, s + 0x16a0, 0x10, -2.0 + k as f32);
        t.set_u32(s + 0x16c0, n);
        junk(&mut t, s + 0x2360, 0x40, k);
        floats(&mut t, o, 0x50, 1.0 + k as f32);
        t.set_u32(t.img.word(bo + 0x2c), 0x00ac_0000 + k);
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
        let last = SCRATCH + 0x3400;
        t.write(last, &[0x34; 0x20]);
        t.set_u32(t.img.word(nd + 0x1b), if listed { last } else { 0 });
        t.set_u32(t.img.word(nd + 0x33), last);
        ownb::dirty_stack(&mut t, 0x45e0 + k, &[]);
        let regs = Call {
            eax: 0x00ea_0000 + k,
            ..Call::default()
        };
        t.check_entry(f, regs, &[o], Eax::Checked);
    }
}

/// `magic_0307` 0x10004720 (`r32::late_objects_0307`): the object list
/// empty and not, the task list empty and not.
#[test]
fn late_objects_0307() {
    let Some(mut t) = twin(307) else { return };
    all_stubs(&mut t);
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i).rotate_left(i + 3);
            m.w32(a[0] + 4 * i, v);
        }
        0x229
    });
    let f = t.at(0x4720);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let so = t.img.rel32_target(f + 0x1d);
    let take = t.img.rel32_target(f + 0x124);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0xc), s);
    for (k, (objs, listed)) in (0u32..).zip([(false, false), (true, true), (false, true)]) {
        start_env_at(
            &mut t,
            so,
            (0x9d, 0xa8),
            6,
            if objs { SCRATCH + 0x680 } else { 0 },
        );
        junk(&mut t, s + 0xb00, 0x1900, k);
        let objs_at = w(&t, 0x18);
        junk(&mut t, objs_at, 0x260, k + 3);
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
        let last = SCRATCH + 0x3400;
        t.write(last, &[0x34; 0x20]);
        t.set_u32(w(&t, 0x12a), if listed { last } else { 0 });
        t.set_u32(w(&t, 0x143), last);
        ownb::dirty_stack(&mut t, 0x4720 + k, &[]);
        let regs = Call {
            ecx: 0x00ec_0000 + k,
            ..Call::default()
        };
        t.check_entry(f, regs, &[], Eax::Checked);
    }
}

/// `magic_0307` 0x100050f0 (`r32::emitter_start_0307`): no, one and three
/// actor records; the object list and the task list empty and not; the
/// target around the emitter.
#[test]
fn emitter_start_0307() {
    let Some(mut t) = twin(307) else { return };
    all_stubs(&mut t);
    bone_stubs(&mut t);
    let f = t.at(0x50f0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let so = t.img.rel32_target(f + 0x20);
    let aim = t.img.rel32_target(f + 0xe5);
    let take = t.img.rel32_target(aim + 0x1e7);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0xc), s);
    // (records, objects listed, task list listed, target)
    let cases: [(i32, bool, bool, [f32; 3]); 4] = [
        (0, false, false, [3.0, 1.0, -2.0]),
        (1, true, true, [-3.0, 0.5, 2.0]),
        (3, false, true, [0.25, -4.0, 0.75]),
        (-2, true, false, [-1.0, 2.0, -6.0]),
    ];
    for (k, (n, objs, listed, tg)) in (0u32..).zip(cases) {
        start_env_at(
            &mut t,
            so,
            (0x9d, 0xa8),
            6,
            if objs { SCRATCH + 0x680 } else { 0 },
        );
        junk(&mut t, s + 0x1000, 0x1300, k);
        floats(&mut t, s + 0xc00, 0x40, -1.5 + k as f32);
        for (i, x) in (0u32..).zip(tg) {
            t.set_u32(s + 0xc60 + 4 * i, x.to_bits());
        }
        t.set_u32(w(&t, 0xed), n.cast_unsigned());
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
        let last = SCRATCH + 0x3400;
        t.write(last, &[0x34; 0x20]);
        t.set_u32(t.img.word(aim + 0x1ed), if listed { last } else { 0 });
        t.set_u32(t.img.word(aim + 0x200), last);
        ownb::dirty_stack(&mut t, 0x50f0 + k, &[]);
        let regs = Call {
            ecx: 0x00ec_0000 + k,
            ..Call::default()
        };
        t.check_entry(f, regs, &[], Eax::Checked);
    }
}

/// `magic_0307` 0x100034c0 (`r32::scene_start_0307`): the task list empty
/// and not, the object list empty and not.
#[test]
fn scene_start_0307() {
    let Some(mut t) = twin(307) else { return };
    all_stubs(&mut t);
    let f = t.at(0x34c0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let so = t.img.rel32_target(f + 0x1c1);
    let take = t.img.rel32_target(f + 0x6e);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x9), s);
    for (k, (objs, listed)) in (0u32..).zip([(false, false), (true, true), (false, true)]) {
        start_env_at(
            &mut t,
            so,
            (0x9d, 0xa8),
            6,
            if objs { SCRATCH + 0x680 } else { 0 },
        );
        junk(&mut t, s + 0x2200, 0x200, k);
        floats(&mut t, s + 0xc40, 0x10, 2.5 - k as f32);
        floats(&mut t, s + 0x2860, 4, -0.5 + k as f32);
        for o in [0x1b7u32, 0x228] {
            let at = w(&t, o);
            junk(&mut t, at, 0x140, k + o);
        }
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
        let last = SCRATCH + 0x3400;
        t.write(last, &[0x34; 0x20]);
        t.set_u32(w(&t, 0x74), if listed { last } else { 0 });
        t.set_u32(w(&t, 0x89), last);
        junk_ops(&mut t, f, &[0x1bc]);
        ownb::dirty_stack(&mut t, 0x34c0 + k, &[]);
        let regs = Call {
            ecx: 0x00ec_0000 + k,
            ..Call::default()
        };
        t.check_entry(f, regs, &[], Eax::Checked);
    }
}

/// `magic_0307` 0x10004de0 (`r32::emitter_step_0307`): the emitter on the
/// host's list and not; no, one and three actor records; the task list
/// empty and not.
#[test]
fn emitter_step_0307() {
    let Some(mut t) = twin(307) else { return };
    all_stubs(&mut t);
    rec(&mut t, 713, 1, |_, a| 0x0713_0000 ^ a[0]);
    let f = t.at(0x4de0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let so = t.img.rel32_target(f + 0x1f3);
    let take = t.img.rel32_target(f + 0x61);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x9), s);
    let e = SCRATCH + 0x2_0000;
    let host = w(&t, 0x1a0);
    let item = w(&t, 0x19b);
    // (on the host's list, records, task list listed)
    let cases: [(bool, i32, bool); 4] = [
        (true, 0, false),
        (false, 1, true),
        (true, 3, true),
        (false, -1, false),
    ];
    for (k, (on, n, listed)) in (0u32..).zip(cases) {
        start_env_at(&mut t, so, (0x9d, 0xa8), 6, 0);
        junk(&mut t, s + 0x1000, 0x1500, k);
        floats(&mut t, s + 0x2860, 4, -0.5 + k as f32);
        floats(&mut t, e + 0x340, 0x10, 1.25 + k as f32);
        for o in [0x1eeu32, 0x255] {
            let at = w(&t, o);
            junk(&mut t, at, 0x140, k + o);
        }
        junk(&mut t, item, 0x140, k + 7);
        // the host's list: two nodes, the second holding the emitter
        let (n1, n2) = (SCRATCH + 0x6000, SCRATCH + 0x6040);
        t.write(n1, &[0x21; 0x10]);
        t.write(n2, &[0x22; 0x10]);
        // node: +0 the previous, +4 the next, +8 the item
        t.set_u32(n1, 0);
        t.set_u32(n1 + 4, n2);
        t.set_u32(n1 + 8, 0x0bad_0001);
        t.set_u32(n2, n1);
        t.set_u32(n2 + 4, 0);
        t.set_u32(n2 + 8, if on { item } else { 0x0bad_0002 });
        junk(&mut t, host, 0x10, k);
        t.set_u32(host, n1);
        t.set_u32(host + 4, n2);
        let p = SCRATCH + 0x6100;
        t.set_u32(w(&t, 0x1b3), p);
        t.set_u32(p, 0x5a5a_0000 + k);
        t.set_u32(w(&t, 0x2e6), n.cast_unsigned());
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
        let last = SCRATCH + 0x3400;
        t.write(last, &[0x34; 0x20]);
        t.set_u32(w(&t, 0x67), if listed { last } else { 0 });
        t.set_u32(w(&t, 0x7c), last);
        junk_ops(&mut t, f, &[0x1a5]);
        ownb::dirty_stack(&mut t, 0x4de0 + k, &[]);
        t.check_entry(f, Call::default(), &[e], Eax::Checked);
    }
}

/// `magic_0307` 0x100055e0 (`r32::summon_start_0307`): the turn wrapped
/// both ways and kept (and NaN), the task list empty and not, the object
/// list empty and not.
#[test]
fn summon_start_0307() {
    let Some(mut t) = twin(307) else { return };
    all_stubs(&mut t);
    matrix_stubs(&mut t);
    let f = t.at(0x55e0);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let so = t.img.rel32_target(f + 0x1b7);
    let take = t.img.rel32_target(f + 0x276);
    let s = SCRATCH + 0x1_0000;
    let last = SCRATCH + 0x3400;
    for (k, (turn, listed, objs)) in (0u32..).zip([
        (0.5f32, false, false),
        (12.0, true, true),
        (-12.0, true, false),
        (f32::NAN, false, true),
    ]) {
        start_env_at(
            &mut t,
            so,
            (0x9d, 0xa8),
            6,
            if objs { SCRATCH + 0x680 } else { 0 },
        );
        t.set_u32(op(&t, 0x9), s);
        junk(&mut t, s + 0xb00, 0x1f00, k);
        for i in 0..0x20u32 {
            t.set_u32(
                s + 0xbe0 + 4 * i,
                (0.5 * i as f32 - 2.0 + k as f32).to_bits(),
            );
            t.set_u32(s + 0xc80 + 4 * i, (0.25 * i as f32 + 1.0).to_bits());
        }
        t.set_u32(s + 0x2868, turn.to_bits());
        let obj = op(&t, 0x1b2);
        junk(&mut t, obj, 0x140, k + 5);
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 6);
        t.write(last, &[0x34; 0x20]);
        t.set_u32(op(&t, 0x27c), if listed { last } else { 0 });
        t.set_u32(op(&t, 0x295), last);
        junk_ops(&mut t, f, &[0x8d]);
        ownb::dirty_stack(&mut t, 0x55e0 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0307` 0x100040d0 (`r32::summon_turn_0307`): the places around
/// each other (the replaced yaw wrapped both ways and kept), the turn
/// varied.
#[test]
fn summon_turn_0307() {
    let Some(mut t) = twin(307) else { return };
    all_stubs(&mut t);
    matrix_stubs(&mut t);
    rec(&mut t, 543, 1, |_, a| 0x0543_0000 ^ a[0]);
    let f = t.at(0x40d0);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(op(&t, 0x17), s);
    for (k, (to, from, turn)) in (0u32..).zip([
        ([1.0f32, 2.0, 3.0], [4.0f32, -1.0, -5.0], 0.5f32),
        ([1.0, 2.0, 3.0], [-4.0, 1.0, 0.5], -2.5),
        ([-2.0, 0.0, 1.0], [-6.0, 3.0, -0.5], 7.0),
        ([0.5, 0.5, 0.5], [0.5, 7.0, 9.5], 0.0),
    ]) {
        junk(&mut t, s + 0xb00, 0x200, k);
        for i in 0..0x20u32 {
            t.set_u32(
                s + 0xbe0 + 4 * i,
                (0.5 * i as f32 - 2.0 + k as f32).to_bits(),
            );
        }
        for (i, x) in (0u32..).zip(to.into_iter().chain([1.0])) {
            t.set_u32(s + 0xc60 + 4 * i, x.to_bits());
        }
        for (i, x) in (0u32..).zip(from.into_iter().chain([0.75])) {
            t.set_u32(s + 0xc70 + 4 * i, x.to_bits());
        }
        t.set_u32(s + 0x2868, turn.to_bits());
        junk_ops(&mut t, f, &[0x163]);
        ownb::dirty_stack(&mut t, 0x40d0 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0307` 0x10004890 (`r32::burst_step_0307`): the object too old
/// (cleared), old enough for the first kind only, young (both kinds); the
/// pool running out in the first kind, in the second, and not at all.
#[test]
fn burst_step_0307() {
    let Some(mut t) = twin(307) else { return };
    all_stubs(&mut t);
    float_matrix_stubs(&mut t);
    let f = t.at(0x4890);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let take = t.img.rel32_target(f + 0x71);
    let (gp1, gp2) = (w(&t, 0x81), w(&t, 0x2c6));
    let o = SCRATCH + 0x2_0000;
    let recs = SCRATCH + 0x2_2000;
    // (age, pool size, taken records)
    let cases: [(u32, u32, &[u32]); 7] = [
        (0x15, 12, &[]),
        (0x14, 12, &[1]),
        (0xb, 12, &[]),
        (0xa, 12, &[0, 5]),
        (3, 6, &[2]),
        (0xffff_fff0, 10, &[3]),
        (7, 9, &[4]),
    ];
    for (k, (age, n, busy)) in (0u32..).zip(cases) {
        floats(&mut t, o, 0x50, 0.5 + k as f32);
        t.set_u32(o + 0x118, age);
        junk(&mut t, o + 0x104, 4, k);
        t.set_u32(gp1, SCRATCH + 0xb00 + 0x10 * k);
        t.set_u32(gp2, SCRATCH + 0xb40 + 0x10 * k);
        for i in 0..0x20 {
            t.set_u32(SCRATCH + 0xb00 + 4 * i, 0x0a00_0000 + 0x101 * i + k);
        }
        pool_74(&mut t, take, recs, n, busy, 0, k);
        t.set_u32(SCRATCH + 0xa00, k);
        ownb::dirty_stack(&mut t, 0x4890 + k, &[]);
        t.check_entry(f, Call::default(), &[o], Eax::Checked);
    }
}

/// What `r32::start_up_0055`/`_0067` (at `f`) need: the pools, the part
/// set-up's buffers, the target byte and its table, the path task's
/// vector slots, the task list (empty or not); slot 262 writes a point
/// from its first word, 229 a vector, 248 a sum, the float slots answer
/// from their word.
fn start_up_55_setup(t: &mut Twin, f: u32, len: u32, k: u32, (battle, listed): (u32, bool)) {
    let at = |o: u32| if len == 989 || o < 0xa8 { o } else { o - 0xc };
    let w = |t: &Twin, o: u32| t.img.word(f + at(o));
    magic_r11::pools(t, f, len);
    rec(t, 262, 4, move |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    for s in [251u32, 17, 19] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| {
            f64::from(a[0] & 0xff) * 0.375 - 1.5 + f64::from(s) * 0.125
        });
    }
    magic_owna::filler(t, 235, 1, 16);
    for s in [328u32, 329] {
        rec(t, s, 3, move |m: &mut dyn StubMem, a: &[u32]| {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i).rotate_left(3) ^ a[2] ^ s;
                m.w32(a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 229, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[2] + 4 * i)) * 0.5 + (m.r32(a[1] + 4 * i) & 3) as f32;
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x229
    });
    rec(t, 248, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x248
    });
    // the path task's vector slots (as `magic_s7_misc.rs`' test answers them)
    rec(t, 217, 2, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            m.w32(
                a[1] + 4 * i,
                (0.25 * i as f32 + (a[0] & 7) as f32).to_bits(),
            );
        }
        0x217
    });
    magic_owna::filler(t, 234, 3, 4);
    magic_owna::filler(t, 233, 3, 4);
    for (s, n) in [(413u32, 1usize), (215, 1), (711, 3), (710, 4), (719, 3)] {
        rec(t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    // the actor, the target byte (its neighbour junk) and its tables
    t.set_u32(w(t, 0x69), 0x0000_0040 + k);
    let idx = SCRATCH + 0x1e0;
    t.set_u8(idx, (2 + 0x10 * k) as u8);
    t.set_u8(idx + 1, 0x77);
    t.set_u32(w(t, 0x76), idx);
    t.set_u32(w(t, 0x14d), idx);
    for i in 0..0x40 {
        t.set_u32(w(t, 0x97) + 4 * i, 0x0000_0080 + 3 * i);
        t.set_u32(w(t, 0x18e) + 4 * i, 0x0000_0080 + 3 * i);
    }
    t.set_u32(w(t, 0x29d), k % 3);
    for i in 0..4 {
        t.set_u32(
            w(t, 0x2ac) + 4 * i,
            [9, 2, 0xffff_fff0, 7][i as usize] ^ (k & 1),
        );
    }
    for o in [0x146u32, 0x310, 0x356] {
        t.set_u32(w(t, o), battle);
    }
    junk_ops(
        t,
        f,
        &[
            0x2e7, 0x2f7, 0x300, 0x306, 0x31b, 0x327, 0x32d, 0x337, 0x35c, 0x362, 0x36c, 0x376,
            0x3cc,
        ]
        .map(at),
    );
    // the task list
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(w(t, 0x399), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x3a9), SCRATCH + 0x600);
}

/// `magic_0055` 0x10001340 / `magic_0067` 0x10003a60
/// (`r32::start_up_0055`/`_0067`, internal) and their load steps
/// 0x10001210 / 0x10003930: in and out of a battle (the height from slot
/// 251 or slots 17/19; a target index past 0x20), the task list empty and
/// not, the record count above and below 1; the load steps' waits and
/// their start.
#[test]
fn start_up_0055_0067() {
    for (n, rva, len, load, port) in [
        (
            55u16,
            0x1340u32,
            989u32,
            0x1210u32,
            ffx_magic::r32::start_up_0055 as fn(&mut _, &_, &_) -> _,
        ),
        (67, 0x3a60, 977, 0x3930, ffx_magic::r32::start_up_0067),
    ] {
        let Some(mut t) = twin(n) else { return };
        all_stubs(&mut t);
        let f = t.at(rva);
        for (k, battle, listed) in [
            (0u32, 0u32, false),
            (1, 1, true),
            (2, 0, true),
            (3, 1, false),
        ] {
            start_up_55_setup(&mut t, f, len, k, (battle, listed));
            ownb::dirty_stack(&mut t, rva + k, &[]);
            t.check(f, Call::default(), &[], Eax::Checked, port);
        }
        let load = t.at(load);
        for (busy, loaded, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
            start_up_55_setup(&mut t, f, len, 3, (1, true));
            rec(&mut t, 225, 1, move |_, _| busy);
            rec(&mut t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
            rec(&mut t, 227, 1, move |_, _| ready);
            let op = |t: &Twin, o: u32| t.img.word(load + o);
            let at = op(&t, 0x18);
            t.set_u32(at, loaded);
            for o in [0x26u32, 0x46] {
                let at = op(&t, o);
                t.set_u32(at, 0x0b0b_0000 + o);
            }
            junk_ops(&mut t, load, &[0x61, 0x6b]);
            t.check_entry(load, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `magic_0055` 0x10001720 / `magic_0067` 0x10003e40
/// (`r32::steps_0055`/`_0067`) and through them the path points
/// 0x100019a0 / 0x100040c0: the first count running and ending (the
/// actor block, the object start), the second count running, ending with
/// the path points (0055: slot 390's answer in its table and not) and
/// without (slot 239), and done; the scripts and the objects answering 1
/// or not, `*G_D` 1 or not.
#[test]
fn steps_0055_0067() {
    for (n, rva) in [(55u16, 0x1720u32), (67, 0x3e40)] {
        let Some(mut t) = twin(n) else { return };
        all_stubs(&mut t);
        float_matrix_stubs(&mut t);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let path = t.img.rel32_target(f + 0x1b4);
        let len = if n == 55 { 1309 } else { 1120 };
        let pw = |t: &Twin, o55: u32, o67: u32| t.img.word(path + if n == 55 { o55 } else { o67 });
        let ts = t
            .img
            .rel32_target(path + if n == 55 { 0x44b } else { 0x38f });
        let objs = t.img.rel32_target(f + 0x1f5);
        let start = t.img.rel32_target(f + 0xbb);
        magic_r11::object_start_auto(&mut t, start);
        rec(&mut t, 262, 4, |m, a| {
            let k = (a[0] & 0xff) as f32;
            wf(m, a[1], 2.0 * (k - 2.0));
            wf(m, a[2], 0.5 * k);
            wf(m, a[3], 3.0 - k);
            0x262
        });
        rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
        t.stub_st0(333, |_, a| f64::from(a[0] & 0xf) * 0.75 - 4.0);
        rec(&mut t, 390, 1, |m, _| m.r32(SCRATCH + 0xa04));
        for (s, k) in [(413u32, 1usize), (215, 1), (239, 3), (723, 1), (724, 2)] {
            rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
        }
        let idx = SCRATCH + 0x1e0;
        t.set_u8(idx, 3);
        t.set_u8(idx + 1, 0x77);
        let tabs = SCRATCH + 0x5000;
        for i in 0..0x40 {
            t.set_u32(tabs + 4 * i, 0x0000_0080 + 3 * i);
        }
        for (o55, o67) in [
            (0x1b, 0),
            (0x87, 0x3f),
            (0x14f, 0),
            (0x1f0, 0x13e),
            (0x3b7, 0x2fb),
        ] {
            if n == 55 || o67 != 0 {
                let at = pw(&t, o55, o67);
                t.set_u32(at, idx);
            }
        }
        let p = SCRATCH + 0x7e0;
        t.set_u32(t.img.word(objs + 0x25), p);
        t.set_u32(t.img.word(objs + 0x37), p + 4);
        // (c1, c2, flag word, 390's answer in the search table (0055), R1
        // done, objects, *G_D, listed)
        let cases: [(i32, i32, u32, u32, u32, bool, u32, bool); 9] = [
            (5, 0, 0, 0, 1, false, 1, false),
            (1, 0, 0, 0, 1, false, 1, true),
            (0, 4, 0, 0, 0, true, 1, false),
            (0, 1, 0x100, 1, 1, true, 1, true),
            (-3, 1, 0x1100, 0, 1, true, 0, false),
            (0, 1, 0x200, 0, 1, false, 1, false),
            (0, 0, 0, 0, 1, true, 1, false),
            (0, -2, 0, 0, 1, true, 0, true),
            (2, 1, 0x100, 1, 0, true, 1, true),
        ];
        for (k, (c1, c2, flag, ans, r1, empty, gd, listed)) in (0u32..).zip(cases) {
            magic_r11::pools(&mut t, f, 547);
            magic_r11::pools(&mut t, path, len);
            magic_r11::pools(&mut t, ts, 180);
            // junk in the globals the steps store to (the second count and
            // `*G_D` among them: set after)
            junk_ops(
                &mut t,
                f,
                &[0x2b, 0x37, 0x6b, 0x11e, 0x12d, 0x137, 0x190, 0x1d6],
            );
            t.set_u32(w(&t, 0x5), c1.cast_unsigned());
            t.set_u32(w(&t, 0x197), c2.cast_unsigned());
            t.set_u32(w(&t, 0x1a9), flag);
            t.set_u32(w(&t, 0x200), gd);
            let answer = if n == 55 && ans == 1 {
                let table = pw(&t, 0x3f, 0);
                t.u32(table + 4)
            } else {
                0x7fff_fff0
            };
            t.set_u32(SCRATCH + 0xa04, answer);
            for o in [0x9du32, 0x116] {
                let at = w(&t, o);
                t.set_u32(at, k % 3);
            }
            for o in [0xacu32, 0x125] {
                let at = w(&t, o);
                for i in 0..4 {
                    t.set_u32(at + 4 * i, 3 + 2 * i);
                }
            }
            // the two scripts: done or waiting
            for (o, done) in [(0x1dfu32, r1), (0x1e9, 1)] {
                let s = w(&t, o);
                junk(&mut t, s, 0x20, k + o);
                t.set_u32(s + 0x14, done);
                t.set_u16(s + 0x18, 5);
            }
            // the objects: none, or one that is counted but not run
            let head = t.img.word(objs + 0x14);
            let (node, obj) = (SCRATCH + 0x6200, SCRATCH + 0x6300);
            junk(&mut t, obj, 0x140, k);
            t.set_u16(obj + 0x124, 0x8001);
            t.set_u32(node, 0);
            t.set_u32(node + 4, 0);
            t.set_u32(node + 8, obj);
            t.set_u32(head, if empty { 0 } else { node });
            // the path task's free ring and the task list
            let ring = (pw(&t, 0x452, 0x396), pw(&t, 0x46e, 0x3b2));
            free_ring(&mut t, SCRATCH + 0x9000, ring, 3, k);
            let last = SCRATCH + 0x3400;
            t.write(last, &[0x34; 0x20]);
            let (head_op, tail_op) = (pw(&t, 0x4d5, 0x419), pw(&t, 0x4e8, 0x42c));
            t.set_u32(head_op, if listed { last } else { 0 });
            t.set_u32(tail_op, last);
            let th = t.img.word(ts + 0x75);
            t.set_u32(th, if listed { last } else { 0 });
            t.set_u32(t.img.word(ts + 0x8c), last);
            // the node's owner word
            let nd = SCRATCH + 0x6400;
            junk(&mut t, nd, 0x20, k + 9);
            t.set_u32(nd + 8, SCRATCH + 0x6500);
            junk(&mut t, SCRATCH + 0x6500, 0x20, k + 10);
            ownb::dirty_stack(&mut t, rva + k, &[]);
            t.check_entry(f, Call::default(), &[nd], Eax::Checked);
        }
    }
}

/// `magic_0165` 0x100017a0 (`r30::place_0165`) with the scene point's
/// index above 0 (r30's two open test gaps on the selection byte at that
/// index): the scene first in the scene point's list and not 0x18b0046, three
/// selected actors, slot 390 answering 0x1084 for the second or third
/// one only, the selection bytes no identity and their neighbours varied.
#[test]
fn place_0165_scene_index() {
    let Some(mut t) = twin(165) else { return };
    all_stubs(&mut t);
    let f = t.at(0x17a0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x20), s);
    let sparks = t.img.rel32_target(f + 0x214);
    t.set_u32(t.img.word(sparks + 0x1b), s);
    let sp = t.img.rel32_target(f + 0x1b3);
    let sel = SCRATCH + 0x180;
    t.write(sel - 8, &[0x21, 0x13, 0x35, 0x07, 0x2c, 0x19, 0x0e, 0x31]);
    t.write(sel, &[6, 3, 1, 7, 2, 5, 4, 0]);
    for op in [0x1dcu32, 0x22f] {
        t.set_u32(w(&t, op), sel);
    }
    for op in [0x5du32, 0x87, 0xdb] {
        let at = t.img.word(sp + op);
        t.set_u32(at, sel);
    }
    let n_at = t.img.word(sp + 0x4a);
    t.set_u32(n_at, 3);
    let scene = SCRATCH + 0x7e0;
    let rot = SCRATCH + 0x7e4;
    let hit = SCRATCH + 0x7e8;
    rec(&mut t, 520, 0, move |m, _| m.r32(scene));
    rec(&mut t, 390, 1, move |m, a| {
        if a[0] == m.r32(hit) {
            0x1084
        } else {
            0x0390_0000 ^ a[0]
        }
    });
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3u32 {
            wf(
                m,
                a[1 + i as usize],
                (a[0] & 0xff) as f32 * 0.5 + i as f32 - 1.0,
            );
        }
        0x262
    });
    rec(&mut t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(333, move |m, _| f64::from(rf(m, rot)));
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * 0.25 - 1.0);
    rec(&mut t, 293, 1, |_, _| 0);
    for (sl, v) in [(277u32, 40.0f64), (15, 1.5), (16, -0.75), (17, 3.0)] {
        t.stub_st0(sl, move |_, a| v + f64::from(a[0] & 3));
    }
    // the actors' words: distinct per selection byte
    for op in [0x1e7u32] {
        let at = w(&t, op);
        for i in 0..0x40 {
            t.set_u32(at + 4 * i, 0x0000_00a0 + 7 * i);
        }
    }
    // the scene point's first scene (its index search runs only for it)
    let first = t.u32(t.img.word(sp + 0x1a));
    assert_ne!(first, 0x018b_0046);
    // (the selected actor slot 390 picks: its byte, the turn, actors shown)
    for (k, (id, r, n)) in [(1u32, 0.5f32, 3u32), (7, -2.0, 2), (3, 9.0, 1)]
        .into_iter()
        .enumerate()
    {
        let k = k as u32;
        t.write(s, &magic_h6::varied(0x6000, k as u8 + 9));
        t.set_u32(scene, first);
        t.set_u32(rot, r.to_bits());
        t.set_u32(hit, id);
        t.set_u32(w(&t, 0x21c), n);
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::r30::place_0165(g, img, fr))
        });
    }
}
