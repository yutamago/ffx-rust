//! Round 38's own ports (`ffx_magic::r38`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_owna;
mod magic_r11;
#[path = "magic/ownb.rs"]
mod ownb;

use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use magic_owna::{filler, rec, recs};

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

/// Every slot answering `first word ^ (slot << 20)`.
fn all_stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
}

/// Junk words at the globals the operands `ops` of `f` name.
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, (0x5a5a_5a5a ^ o) | 1);
    }
}

/// The 0x74-byte record pool `take` searches (`sb_misc::pool_take`): `n`
/// records at `recs`, those in `busy` taken, the search from `from` (as
/// `magic_r34.rs`).
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

/// The sparks' slots: 262 a point from its first word, 12 a height, 719
/// a handle, 314 / 404 floats from their argument and a running count
/// (ST0), 235 a unit matrix, 329 / 231 / 229 mixing what they read.
fn spark_stubs(t: &mut Twin) {
    all_stubs(t);
    rec(t, 262, 4, |m, a| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0;
            wf(m, a[1 + i as usize], v);
        }
        0x262
    });
    rec(t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, a| 1.25 + f64::from(a[0] & 7));
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0] ^ (a[2] << 8));
    for (s, x) in [(314u32, 0.375f64), (404, -0.625), (333, 1.5)] {
        rec(t, s, 1, move |m, a| {
            let k = m.r32(SCRATCH + 0xa00);
            m.w32(SCRATCH + 0xa00, k + 1);
            (s << 16) ^ a[0] ^ k
        });
        t.stub_st0(s, move |m, a| {
            x * f64::from(m.r32(SCRATCH + 0xa00) % 5) + f64::from(f32::from_bits(a[0]))
        });
    }
    filler(t, 235, 1, 16);
    rec(t, 329, 3, |m, a| {
        let ang = rf(m, a[2]);
        for i in 0..16 {
            let v = rf(m, a[1] + 4 * i) + ang * (i + 1) as f32 / 8.0;
            wf(m, a[0] + 4 * i, v);
        }
        0x329
    });
    rec(t, 231, 3, |m, a| {
        for i in 0..16 {
            let v = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * (15 - i));
            wf(m, a[0] + 4 * i, v);
        }
        0x231
    });
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[2] + 4 * i) * 0.5 + rf(m, a[1] + 4 * i) + i as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
}

/// `magic_0316` 0x100014b0 (`r38::spark_tick_0316`, cdecl `(node)`): the
/// count above 1 (odd and even after the step), at 1 and below (the node
/// given back, then odd or even), the pool with a free record and none.
#[test]
fn spark_tick_0316() {
    let Some(mut t) = twin(316) else { return };
    spark_stubs(&mut t);
    let f = t.at(0x14b0);
    let take = t.img.rel32_target(f + 0x30);
    let back = t.img.rel32_target(f + 0x20);
    let recs = SCRATCH + 0x8000;
    t.set_u32(t.img.word(f + 0x40), 0x0000_0047);
    // (count, busy records)
    let cases: [(i32, &[u32]); 7] = [
        (3, &[]),
        (2, &[]),
        (1, &[1]),
        (0, &[]),
        (-2, &[2]),
        (5, &[0, 1, 2]),
        (8, &[]),
    ];
    for (k, (count, busy)) in (0u32..).zip(cases) {
        pool_74(&mut t, take, recs, 3, busy, k % 3, k);
        t.set_u32(SCRATCH + 0xa00, k);
        let node = SCRATCH + 0x900;
        let (prev, next) = (SCRATCH + 0x940, SCRATCH + 0x980);
        t.write(node, &[0x66; 0x20]);
        t.write(prev, &[0x67; 0x20]);
        t.write(next, &[0x68; 0x20]);
        t.set_u32(node, prev);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, node);
        t.set_u32(node + 0xc, 0);
        t.set_u32(node + 0x14, count.cast_unsigned());
        let (cnt, head) = (t.img.word(back + 0x8e), t.img.word(back + 0x74));
        magic_r11::ring(&mut t, cnt, head, 3, SCRATCH + 0xa40);
        ownb::dirty_stack(&mut t, 0x14b0 + k, &[]);
        ownb::reset_counters(&mut t);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

/// `magic_0316` 0x10001800 (`r38::spark_ring_0316`): the yaw above,
/// inside and below its wrap; the pool with 28 free records and runs out
/// at once and part way.
#[test]
fn spark_ring_0316() {
    let Some(mut t) = twin(316) else { return };
    spark_stubs(&mut t);
    let f = t.at(0x1800);
    let take = t.img.rel32_target(f + 0x189);
    let recs = SCRATCH + 0x8000;
    t.set_u32(t.img.word(f + 0x1a), 0x0000_0033);
    t.set_u32(t.img.word(f + 0x7a), 0x0000_0047);
    // (slot 333's base angle, free records)
    let busy_none: &[u32] = &[];
    let busy_all: Vec<u32> = (0..30).collect();
    let busy_some: Vec<u32> = (5..30).collect();
    let cases: [(f64, &[u32]); 4] = [
        (0.25, busy_none),
        (-9.0, busy_none),
        (9.0, &busy_all),
        (2.0, &busy_some),
    ];
    for (k, (yaw, busy)) in (0u32..).zip(cases) {
        pool_74(&mut t, take, recs, 30, busy, 0, k);
        t.set_u32(SCRATCH + 0xa00, k);
        rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
        t.stub_st0(333, move |_, _| yaw);
        ownb::dirty_stack(&mut t, 0x1800 + k, &[]);
        ownb::reset_counters(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// The load step of a start-up (at `load`): busy, asked or not, the data
/// loaded or not; `setup` prepares the start-up before each case (as
/// `magic_r26.rs`).
fn load_cases(t: &mut Twin, load: u32, setup: &dyn Fn(&mut Twin)) {
    for (busy, loaded, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        setup(t);
        rec(t, 225, 1, move |_, _| busy);
        rec(t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
        rec(t, 227, 1, move |_, _| ready);
        let op = |t: &Twin, o: u32| t.img.word(load + o);
        let at = op(t, 0x18);
        t.set_u32(at, loaded);
        for o in [0x26u32, 0x46] {
            let at = op(t, o);
            t.set_u32(at, 0x0b0b_0000 + o);
        }
        junk_ops(t, load, &[0x61, 0x6b]);
        magic_h6::scrub(t);
        t.check_entry(load, Call::default(), &[], Eax::Checked);
    }
}

/// What `r38::start_up_0316` (`magic_0316` 0x10001bf0) needs: the pools,
/// the part set-up's and the object starts' buffers, the actors' lookup
/// (two actor bytes), the caster, the fades' records, the task list
/// (empty or not), the actor's side flags.
fn start_up_0316_setup(t: &mut Twin, f: u32, k: u32, listed: bool) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    spark_stubs(t);
    magic_r11::pools(t, f, 1278);
    magic_r11::object_slots(t);
    rec(t, 262, 4, move |m, a| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
            wf(m, a[1 + i as usize], v);
        }
        0x262
    });
    let yaw = [0.25f64, 3.0, 9.5, -7.0][k as usize % 4];
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |_, _| yaw);
    rec(t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, move |_, _| 0.75 + f64::from(k));
    recs(t, &[(413, 1), (215, 1), (220, 3), (271, 1)]);
    for call in [0xd3u32, 0x1b7] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(t, start);
    }
    // the lookup: two actor bytes
    let lk = t.img.rel32_target(f + 0x37);
    let lw = |t: &Twin, o: u32| t.img.word(lk + o);
    for o in [0x34u32, 0x73] {
        let at = lw(t, o);
        t.set_u32(at, 2);
    }
    let at = lw(t, 0x4a);
    t.write(at, &[3 + k as u8, 9, 0x77]);
    let at = lw(t, 0x42);
    t.set_u32(at, k & 1);
    for o in [0x65u32, 0xde, 0x1d7] {
        t.set_u32(w(t, o), 0x0000_0040 + k);
    }
    // the actor record's word and flags
    t.set_u32(w(t, 0x387), 0x0387_0000 + k);
    t.set_u32(w(t, 0x390), k & 1);
    t.set_u32(w(t, 0x3ef), (k >> 1) & 1);
    // junk in the globals it stores constants to
    junk_ops(
        t,
        f,
        &[
            0x06, 0x10, 0x1a, 0x24, 0x2e, 0x55, 0x146, 0x158, 0x162, 0x16c, 0x176, 0x180, 0x18a,
            0x194, 0x19e, 0x1ae, 0x1ef, 0x223, 0x246, 0x258, 0x262, 0x26c, 0x276, 0x280, 0x28a,
            0x294, 0x29e, 0x2ae, 0x2b8, 0x2c2, 0x2cc, 0x2d8, 0x396, 0x3a2, 0x3ac, 0x3c6, 0x3d0,
            0x3fa, 0x400, 0x40a, 0x414,
        ],
    );
    for o in [0x1a8u32, 0x23b, 0x3b7, 0x41e, 0x427] {
        let at = w(t, o);
        t.set_u32(at, 0x5a5a_5a5a);
    }
    let at = w(t, 0x3bf);
    t.set_u32(at, 0x5a5a_5a5a);
    // the fades' records
    for o in [0x2f4u32, 0x318] {
        let at = w(t, o);
        t.write(at, &[0x5c; 0x40]);
    }
    // the task list
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    let head = w(t, 0x437);
    t.set_u32(head, if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x44b), SCRATCH + 0x600);
}

/// `magic_0316` 0x10001bf0 (`r38::start_up_0316`, internal) and its load
/// step 0x100012b0 (`r38::load_0316`): the task list empty and not, the
/// yaws above, inside and below their wraps, both actor slots and sides.
#[test]
fn start_up_0316() {
    let Some(mut t) = twin(316) else { return };
    let f = t.at(0x1bf0);
    for (k, listed) in [(0u32, false), (1, true), (2, true), (3, false)] {
        start_up_0316_setup(&mut t, f, k, listed);
        magic_h6::scrub(&mut t);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r38::start_up_0316,
        );
    }
    let load = t.at(0x12b0);
    load_cases(&mut t, load, &|t| start_up_0316_setup(t, f, 2, true));
}

/// `magic_0361`'s event slots: the rc/rcef functions answer from their
/// words and write what the game writes (503 a point, 500/235 matrices,
/// 236/498/231/494 matrices mixed from what they read, 496/504/464 scale
/// in place, 477/497 floats), each read input feeding the answer.
fn stubs_0361(t: &mut Twin) {
    all_stubs(t);
    filler(t, 235, 1, 16);
    filler(t, 500, 1, 16);
    for (s, n) in [
        (456u32, 3usize),
        (461, 2),
        (450, 2),
        (449, 2),
        (434, 2),
        (446, 2),
        (447, 2),
        (507, 2),
        (508, 2),
        (501, 1),
        (502, 1),
        (481, 1),
        (480, 2),
    ] {
        rec(t, s, n, move |_, a| {
            (s << 16) ^ a[0] ^ a.get(1).copied().unwrap_or(0).rotate_left(5)
        });
    }
    rec(t, 503, 2, |m, a| {
        for i in 0..4u32 {
            wf(
                m,
                a[1] + 4 * i,
                (a[0] & 0xff) as f32 * 0.25 + i as f32 * 1.5 - 3.0,
            );
        }
        0x503
    });
    for (s, scale) in [(464u32, 0.5f32), (496, -0.75)] {
        rec(t, s, 1, move |m, a| {
            for i in 0..4u32 {
                let v = rf(m, a[0] + 4 * i) * scale + i as f32;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 504, 1, |m, a| {
        for i in 0..12u32 {
            let v = rf(m, a[0] + 4 * i) * 3.0 - 1.0;
            wf(m, a[0] + 4 * i, v);
        }
        0x504
    });
    rec(t, 499, 3, |m, a| {
        let mut x = 0x499u32;
        for i in 0..8u32 {
            x = x.rotate_left(3) ^ m.r32(a[1 + (i / 4) as usize] + 4 * (i % 4));
        }
        m.w32(SCRATCH + 0xb04, x);
        // the hold word the step reads again after the call
        let hold = m.r32(SCRATCH + 0xb00);
        m.w32(0x100a_b3d8, hold);
        x
    });
    rec(t, 236, 3, |m, a| {
        for i in 0..16u32 {
            wf(
                m,
                a[2] + 4 * i,
                (a[0] & 0xf) as f32 + i as f32 * 0.5 + a[1] as f32,
            );
        }
        0x236
    });
    rec(t, 498, 4, |m, a| {
        for i in 0..16u32 {
            let v = rf(m, a[3] + 4 * (i % 4)) * (i + 1) as f32 + (a[1] & 0xf) as f32;
            wf(m, a[2] + 4 * i, v);
        }
        0x498
    });
    rec(t, 231, 3, |m, a| {
        for i in 0..16 {
            let v = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * (15 - i));
            wf(m, a[0] + 4 * i, v);
        }
        0x231
    });
    rec(t, 494, 4, |m, a| {
        for i in 0..16u32 {
            let v =
                rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * (i % 4)) * 2.0 - rf(m, a[3] + 4 * (i % 4));
            wf(m, a[0] + 4 * i, v);
        }
        0x494
    });
    rec(t, 484, 3, |m, a| {
        let mut x = 0x484u32 ^ a[2];
        for i in 0..12u32 {
            x = x.rotate_left(5) ^ m.r32(a[1] + 4 * i);
        }
        x
    });
    for (s, base) in [(477u32, 0.625f64), (497, 2.0)] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| {
            base + f64::from(f32::from_bits(a[0]).abs().min(64.0)) * 0.5 + f64::from(a[0] & 3)
        });
    }
}

/// The event block `*G_E` (caster `+0x198`, target `+0x1a0`) and the
/// event's record (`i` at `+0xac`, its word at `+0x2c + 16 i`).
fn event_0361(t: &mut Twin, f: u32, op: u32, i: u32, word: i32) -> u32 {
    let e = SCRATCH + 0x2000;
    junk(t, e + 0x180, 0x40, 3);
    t.set_u32(e + 0x198, 0x47);
    t.set_u32(e + 0x1a0, 0x53);
    t.set_u32(t.img.word(f + op), e);
    let ev = SCRATCH + 0x2400;
    junk(t, ev, 0x100, i);
    t.set_u32(ev + 0xac, i);
    t.set_u32(ev + 0x2c + 16 * i, word.cast_unsigned());
    ev
}

/// The place `V` (`0x100ab420`), velocity `U` (`+0x58`) and path `S`
/// (`+0x68`) globals and the matrix `M` (`0x100ab3e0`).
fn motion_0361(t: &mut Twin, v: [f32; 4], u: [f32; 4], s: [f32; 4], k: u32) {
    for i in 0..4u32 {
        t.set_u32(0x100a_b420 + 4 * i, v[i as usize].to_bits());
        t.set_u32(0x100a_b478 + 4 * i, u[i as usize].to_bits());
        t.set_u32(0x100a_b488 + 4 * i, s[i as usize].to_bits());
    }
    junk(t, 0x100a_b3e0, 0x40, k);
}

/// `magic_0361` 0x10001cb0 (`r38::aim_0361`, cdecl `(ev)`).
#[test]
fn aim_0361() {
    let Some(mut t) = twin(361) else { return };
    stubs_0361(&mut t);
    let f = t.at(0x1cb0);
    assert_eq!(t.img.word(f + 0x11), 0x100a_b350);
    for k in 0..3u32 {
        let ev = event_0361(&mut t, f, 0x11, k, 1);
        t.set_u32(0x100a_b3dc, 0x77);
        motion_0361(&mut t, [9.0; 4], [7.0; 4], [5.0; 4], k);
        ownb::dirty_stack(&mut t, 0x1cb0 + k, &[]);
        t.check_entry(f, Call::default(), &[ev], Eax::Checked);
    }
}

/// `magic_0361` 0x100027d0 (`r38::fade_in_0361`, cdecl `(ev)`): held or
/// not (and held by slot 499), the count below 10, at 10 and past it.
#[test]
fn fade_in_0361() {
    let Some(mut t) = twin(361) else { return };
    stubs_0361(&mut t);
    let f = t.at(0x27d0);
    // (hold, count, hold after slot 499)
    for (k, (hold, n, later)) in (0u32..).zip([
        (0u32, 0i32, 0u32),
        (0, 9, 0),
        (0, 10, 0),
        (0, 10, 1),
        (1, 10, 1),
        (1, 3, 1),
        (0, -4, 0),
        (0, 25, 0),
    ]) {
        let ev = event_0361(&mut t, f, 0x11, k % 3, 1);
        t.set_u32(0x100a_b3d8, hold);
        t.set_u32(SCRATCH + 0xb00, later);
        t.set_u32(0x100a_b3dc, n.cast_unsigned());
        ownb::dirty_stack(&mut t, 0x27d0 + k, &[]);
        t.check_entry(f, Call::default(), &[ev], Eax::Checked);
    }
}

/// `magic_0361` 0x100023b0 (`r38::flight_0361`, cdecl `(ev)`): `V.y` and
/// `S.y` above, at and below 0 and NaN; the first bounce and later ones;
/// the count reaching 3 and not.
#[test]
fn flight_0361() {
    let Some(mut t) = twin(361) else { return };
    stubs_0361(&mut t);
    let f = t.at(0x23b0);
    // (V.y + U.y, S.y + that, count)
    for (k, (vy, sy, n)) in (0u32..).zip([
        (1.5f32, 2.0f32, 0i32),
        (1.5, 2.0, 1),
        (1.5, 2.0, 2),
        (0.0, -1.5, 2),
        (-0.0, 0.0, 0),
        (-0.5, 4.0, 0),
        (2.0, -2.5, 0),
        (f32::NAN, 1.0, 0),
        (1.0, f32::NAN, 2),
        (3.0, 1.0, 7),
    ]) {
        let ev = event_0361(&mut t, f, 0x15, k % 3, 1);
        let u = [0.5, 0.25, -0.75, 1.25];
        let v = [2.0 + k as f32, vy - 0.25, 1.5, -2.0];
        let s = [3.0, sy - (vy - 0.25 + 0.25), -1.0, 0.5 * k as f32];
        motion_0361(&mut t, v, u, s, k);
        t.set_u32(0x100a_b3dc, n.cast_unsigned());
        ownb::dirty_stack(&mut t, 0x23b0 + k, &[]);
        t.check_entry(f, Call::default(), &[ev], Eax::Checked);
    }
}

/// `magic_0361` 0x10001df0 (`r38::throw_0361`, cdecl `(ev)`): the count
/// below 5, 5..9, at 10 and past it; the record's word positive, 0 and
/// negative; held or not.
#[test]
fn throw_0361() {
    let Some(mut t) = twin(361) else { return };
    stubs_0361(&mut t);
    let f = t.at(0x1df0);
    // (count, the record's word, hold)
    for (k, (n, word, hold)) in (0u32..).zip([
        (0i32, 1i32, 0u32),
        (4, 0, 0),
        (5, -3, 0),
        (7, 2, 1),
        (9, 0, 0),
        (10, 5, 0),
        (10, 0, 1),
        (11, 1, 1),
        (30, -1, 0),
        (-2, 7, 1),
    ]) {
        let ev = event_0361(&mut t, f, 0x14, k % 3, word);
        motion_0361(
            &mut t,
            [1.0, 2.0 + k as f32, -3.0, 0.5],
            [0.25, -0.5, 0.75, 1.5],
            [4.0, 1.0, 2.5, -1.0],
            k,
        );
        t.set_u32(0x100a_b3dc, n.cast_unsigned());
        t.set_u32(0x100a_b3d8, hold);
        t.set_u32(SCRATCH + 0xb00, hold);
        junk(&mut t, 0x100a_b438, 0x10, k);
        ownb::dirty_stack(&mut t, 0x1df0 + k, &[]);
        t.check_entry(f, Call::default(), &[ev], Eax::Checked);
    }
}
