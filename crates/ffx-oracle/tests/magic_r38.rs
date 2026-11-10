//! Round 38's own ports (`ffx_magic::r38`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
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

/// `magic_0350`'s state block `S` (at the operand `op` of `f`), junk.
fn state_0350(t: &mut Twin, f: u32, op: u32, k: u32) -> u32 {
    let s = SCRATCH + 0x1_0000;
    junk(t, s + 0x100, 0xe00, k);
    t.set_u32(t.img.word(f + op), s);
    s
}

/// `magic_0350` 0x100059a0 (`r38::task_0350`, cdecl `(node)`): the script
/// running or ended, the motion waiting or done; both done (the lights,
/// models and parts, the node given back).
#[test]
fn task_0350() {
    let Some(mut t) = twin(350) else { return };
    all_stubs(&mut t);
    let f = t.at(0x59a0);
    let run = t.img.rel32_target(f + 0x33);
    let ms = t.img.rel32_target(f + 0x3a);
    let lm = t.img.rel32_target(f + 0x4e);
    let give = t.img.rel32_target(f + 0x59);
    recs(
        &mut t,
        &[(590, 2), (589, 2), (296, 2), (941, 1), (772, 2), (779, 2)],
    );
    let bytes = SCRATCH + 0x600;
    t.write(bytes, &[3, 5, 7, 9]);
    rec(&mut t, 288, 2, move |m, a| {
        m.w32(a[1], 2);
        bytes
    });
    // (script mode, motion done)
    for (k, (mode, done)) in (0u32..).zip([(4u8, false), (0, false), (4, true), (0, true)]) {
        // the lights' block is `S + 0xc18` (the same state global)
        let s = state_0350(&mut t, f, 0x07, k);
        assert_eq!(t.img.word(lm + 0x6), t.img.word(f + 0x07));
        let e = s + t.img.word(lm + 0xe);
        t.set_u8(t.img.word(run + 0x01), mode);
        t.set_u32(t.img.word(run + 0x07), 0x0bad_0007);
        magic_hi::motion(&mut t, ms, done);
        let blk = t.img.word(f + 0x10);
        junk(&mut t, blk, 0x30, k);
        t.set_u32(t.img.word(f + 0x26), 0x0026_0000 + k);
        t.write(e, &[0x5a; 0x240]);
        t.set_u32(e + 0x228, 0);
        let node = SCRATCH + 0x900;
        magic_hi::lone_node(&mut t, give, node);
        ownb::dirty_stack(&mut t, 0x59a0 + k, &[]);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

/// `magic_0350` 0x10005c60 (`r38::packed_0350`, cdecl `(node)`): the
/// fade-out over (the node given back); the count above 0 and not; the
/// angle wrapping and not; the texture filled and not.
#[test]
fn packed_0350() {
    let Some(mut t) = twin(350) else { return };
    all_stubs(&mut t);
    let f = t.at(0x5c60);
    let give = t.img.rel32_target(f + 0x2e);
    let desc = SCRATCH + 0x700;
    rec(&mut t, 719, 3, move |_, _| desc);
    filler(&mut t, 235, 1, 16);
    rec(&mut t, 210, 2, |m, a| {
        for i in 0..16 {
            m.w32(a[1] + 4 * i, a[0] ^ i);
        }
        0x210
    });
    let answer = SCRATCH + 0x7e0;
    rec(&mut t, 885, 7, move |m, _| m.r32(answer));
    rec(&mut t, 905, 9, |m, a| {
        let c = m.r32(SCRATCH + 0xb20);
        m.w32(SCRATCH + 0xb20, c + 1);
        0x0905_0000 ^ a[0]
    });
    rec(&mut t, 776, 1, |_, a| 0x0776_0000 ^ a[0]);
    // the draw's lock word (so a filled texture reaches slot 905)
    let draw = t.img.rel32_target(f + 0x130);
    t.set_u32(t.img.word(draw + 0x3c), SCRATCH + 0x7f0);
    t.set_u32(SCRATCH + 0x7f0, 0x5a5a_0001);
    // (out, count, total, angle, filled)
    for (k, (out, c, total, angle, filled)) in (0u32..).zip([
        (0i32, 3i32, 9i32, 1.0f32, 0u32),
        (-2, 3, 9, 1.0, 1),
        (5, 3, 9, 1.0, 1),
        (5, 1, 9, -0.25, 0),
        (5, 0, 9, 0.0, 1),
        (2, -4, 9, 3.0, 1),
        (9, 0, -3, -5.0, 7),
    ]) {
        let s = state_0350(&mut t, f, 0x19, k);
        t.set_u32(s + 0xe50, out.cast_unsigned());
        t.set_u32(s + 0xe48, c.cast_unsigned());
        t.set_u32(s + 0xe4c, total.cast_unsigned());
        t.set_u32(s + 0xe54, (7 + k).cast_signed().cast_unsigned());
        t.set_u32(s + 0xe44, angle.to_bits());
        t.set_u32(desc, 0x1234_5678 ^ k);
        t.set_u32(desc + 4, 0x9abc_def0 ^ k);
        t.set_u32(answer, filled);
        t.set_u32(t.img.word(f + 0xba), SCRATCH + 0x3000);
        junk(&mut t, SCRATCH + 0x3000, 0x900, k);
        t.set_u32(t.img.word(f + 0xf3), 0x00f3_0000 + k);
        let node = SCRATCH + 0x900;
        magic_hi::lone_node(&mut t, give, node);
        ownb::dirty_stack(&mut t, 0x5c60 + k, &[]);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
    assert!(t.u32(SCRATCH + 0xb20) > 0, "the texture was drawn");
}

/// `magic_0350` 0x10005db0 (`r38::set_up_0350`): the task list empty and
/// not. Its two never-written frame words read 0 on both sides.
#[test]
fn set_up_0350() {
    let Some(mut t) = twin(350) else { return };
    all_stubs(&mut t);
    let f = t.at(0x5db0);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0x2f);
    magic_r11::object_start_auto(&mut t, start);
    for listed in [false, true] {
        let k = u32::from(listed);
        let s = state_0350(&mut t, f, 0x1d, k);
        let _ = s;
        magic_r11::pools(&mut t, f, 1232);
        t.write(SCRATCH + 0x600, &[0x44; 0x10]);
        t.set_u32(
            t.img.word(f + 0xe1),
            if listed { SCRATCH + 0x600 } else { 0 },
        );
        t.set_u32(t.img.word(f + 0xfb), SCRATCH + 0x600);
        ownb::dirty_stack(&mut t, 0x5db0 + k, &[0x10, 0xc]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0350` 0x10006320 (`r38::sparks_0350`): the count out, and in
/// with two, one and no records per bone; the pool running out.
#[test]
fn sparks_0350() {
    let Some(mut t) = twin(350) else { return };
    spark_stubs(&mut t);
    let f = t.at(0x6320);
    let take = t.img.rel32_target(f + 0x87);
    let recs_at = SCRATCH + 0x8000;
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4u32 {
            wf(
                m,
                a[3] + 4 * i,
                (a[1] & 0xff) as f32 * 0.5 + i as f32 - 1.25,
            );
        }
        0x0237_0000 ^ a[1]
    });
    let h = SCRATCH + 0x7c0;
    junk(&mut t, h, 0x20, 1);
    t.set_u32(t.img.word(f + 0x97), h);
    t.set_u32(t.img.word(f + 0x67), 0x0067_0000);
    // (count, total, free records)
    for (k, (c, total, free)) in (0u32..).zip([
        (0i32, 4i32, 40u32),
        (1, 4, 40),
        (3, 4, 40),
        (4, 2, 40),
        (6, 3, 40),
        (8, 3, 5),
        (2, -1, 40),
    ]) {
        let s = state_0350(&mut t, f, 0x13, k);
        t.set_u32(s + 0xe58, c.cast_unsigned());
        t.set_u32(s + 0xe5c, total.cast_unsigned());
        let busy: Vec<u32> = (free..40).collect();
        pool_74(&mut t, take, recs_at, 40, &busy, 0, k);
        t.set_u32(SCRATCH + 0xa00, k);
        ownb::dirty_stack(&mut t, 0x6320 + k, &[]);
        ownb::reset_counters(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0350` 0x10006830 (`r38::keys_0350`): the copies and the scale
/// task's node appended to an empty and a non-empty list.
#[test]
fn keys_0350() {
    let Some(mut t) = twin(350) else { return };
    all_stubs(&mut t);
    let f = t.at(0x6830);
    let task = t.img.rel32_target(f + 0x349);
    for listed in [false, true] {
        let k = u32::from(listed);
        state_0350(&mut t, f, 0x02, k);
        magic_r11::pools(&mut t, task, 191);
        let w = |t: &Twin, o: u32| t.img.word(task + o);
        t.write(SCRATCH + 0x600, &[0x44; 0x10]);
        let head = w(&t, 0x81);
        t.set_u32(head, if listed { SCRATCH + 0x600 } else { 0 });
        let tail = w(&t, 0x99);
        t.set_u32(tail, SCRATCH + 0x600);
        ownb::dirty_stack(&mut t, 0x6830 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0275`'s selection (two bytes) and character table (every
/// entry a junk character of its own).
fn sel_0275(t: &mut Twin, b0: u8, b1: u8, k: u32) {
    let sel = SCRATCH + 0x180;
    t.write(sel, &[b0, b1, 0x77, 0x66]);
    t.set_u32(0x1022_8568, sel);
    for i in 0..0x20u32 {
        let c = SCRATCH + 0x2_0000 + 0x900 * i;
        t.set_u32(0x1022_85a4 + 4 * i, c);
    }
    for b in [b0, b1] {
        let c = SCRATCH + 0x2_0000 + 0x900 * u32::from(b);
        junk(t, c + 0x800, 0x40, k + u32::from(b));
    }
}

/// `magic_0275`'s slots: points, vectors and matrices written from their
/// words, floats in ST0, no node with the same id.
fn stubs_0275(t: &mut Twin, k: u32) {
    all_stubs(t);
    rec(t, 262, 4, move |m, a| {
        for i in 0..3u32 {
            let v = ((a[0] ^ (a[0] >> 11)) & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
            wf(m, a[1 + i as usize], v);
        }
        0x262
    });
    rec(t, 268, 3, |m, a| {
        for i in 0..4u32 {
            let v = (a[0] & 0xff) as f32 * -0.25 + i as f32 * 0.75 + a[1] as f32;
            wf(m, a[2] + 4 * i, v);
        }
        0x268
    });
    filler(t, 235, 1, 16);
    for s in [210u32, 723, 724] {
        let at = usize::from(s == 210);
        rec(t, s, at + 1, move |m, a| {
            for i in 0..16u32 {
                wf(m, a[at] + 4 * i, (s & 0xf) as f32 + i as f32 * 0.375 - 2.0);
            }
            s
        });
    }
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[2] + 4 * i) * 0.5
                + rf(m, a[1] + 4 * i)
                + i as f32
                + rf(m, a[2] + 12) * 0.125;
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    rec(t, 231, 3, |m, a| {
        // what it reads, folded into a word the comparison sees (its
        // output is a frame local)
        let mut h = m.r32(SCRATCH + 0xb10);
        for i in 0..16 {
            h = h.rotate_left(3) ^ m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i).rotate_left(11);
        }
        m.w32(SCRATCH + 0xb10, h);
        for i in 0..16 {
            let v = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * (15 - i));
            wf(m, a[0] + 4 * i, v);
        }
        0x231
    });
    rec(t, 329, 3, |m, a| {
        let ang = rf(m, a[2]);
        for i in 0..16 {
            let v = rf(m, a[1] + 4 * i) + ang * (i + 1) as f32 / 8.0;
            wf(m, a[0] + 4 * i, v);
        }
        0x329
    });
    let yaw = [0.25f64, 3.0, 9.5, -7.0][k as usize % 4];
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |_, _| yaw);
    rec(t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, a| 1.25 + f64::from(a[0] & 7));
    for s in [852u32, 853] {
        rec(t, s, 0, |_, _| 0);
    }
}

/// `magic_0275` 0x10003f90 (`r38::show_0275`, internal): `sel[0]` 13, 8
/// and other.
#[test]
fn show_0275() {
    let Some(mut t) = twin(275) else { return };
    let f = t.at(0x3f90);
    for (k, b0) in (0u32..).zip([13u8, 8, 3]) {
        stubs_0275(&mut t, k);
        sel_0275(&mut t, b0, 5, k);
        junk_ops(&mut t, f, &[0x44, 0x4a, 0xb9, 0x170]);
        t.set_u32(t.img.word(f + 0x3d), 0x5a5a_5a5a);
        ownb::dirty_stack(&mut t, 0x3f90 + k, &[]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::r38::show_0275(g, img, fr.at))
        });
    }
}

/// `magic_0275` 0x10004190 (`r38::matrix_0275`, internal, `ecx` = the
/// matrix): `sel[0]` 13 and not.
#[test]
fn matrix_0275() {
    let Some(mut t) = twin(275) else { return };
    let f = t.at(0x4190);
    let x = SCRATCH + 0x3000;
    for (k, b0) in (0u32..).zip([13u8, 2]) {
        stubs_0275(&mut t, k);
        sel_0275(&mut t, b0, 6, k);
        junk(&mut t, x, 0x40, k);
        junk_ops(&mut t, f, &[0x2f]);
        ownb::dirty_stack(&mut t, 0x4190 + k, &[]);
        t.check(f, Call::this(x), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::r38::matrix_0275(g, img, fr.at, fr.esp, fr.ecx))
        });
    }
}

/// What `r38::start_up_0275` (`magic_0275` 0x100036a0) needs: the pools,
/// the part set-up's and the object start's buffers, the caster, the
/// selection, the task list (empty or not).
fn start_up_0275_setup(t: &mut Twin, f: u32, k: u32, listed: bool) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    stubs_0275(t, k);
    magic_r11::pools(t, f, 849);
    magic_r11::object_slots(t);
    stubs_0275(t, k);
    let start = t.img.rel32_target(f + 0xda);
    magic_r11::object_start_auto(t, start);
    sel_0275(t, [13u8, 8, 4, 13][k as usize % 4], 7, k);
    t.set_u32(w(t, 0x6e), 0x0000_0040 + k);
    t.set_u32(w(t, 0xe5), 0x0000_0041 + k);
    // the state block: the global the start-up sets to its immediate
    junk(t, w(t, 0x54), 0x100, k);
    junk_ops(
        t,
        f,
        &[
            0x28, 0x32, 0x3c, 0x46, 0x5e, 0x10e, 0x122, 0x12e, 0x134, 0x210, 0x33f,
        ],
    );
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    let head = w(t, 0x2d3);
    t.set_u32(head, if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x2e6), SCRATCH + 0x600);
}

/// `magic_0275` 0x100036a0 (`r38::start_up_0275`, internal) and its load
/// step 0x10003570 (`r38::load_0275`): the task list empty and not, the
/// yaws above, inside and below their wraps.
#[test]
fn start_up_0275() {
    let Some(mut t) = twin(275) else { return };
    let f = t.at(0x36a0);
    for (k, listed) in [(0u32, false), (1, true), (2, true), (3, false)] {
        start_up_0275_setup(&mut t, f, k, listed);
        ownb::dirty_stack(&mut t, 0x36a0 + k, &[]);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r38::start_up_0275,
        );
    }
    let load = t.at(0x3570);
    load_cases(&mut t, load, &|t| start_up_0275_setup(t, f, 2, true));
}

/// `magic_0275` 0x10003a40 (`r38::task_0275`, cdecl `(node)`): every
/// state with its count running and at its end, a state past the table,
/// `sel[0]` 13 and not.
#[test]
fn task_0275() {
    let Some(mut t) = twin(275) else { return };
    let f = t.at(0x3a40);
    let give = t.img.rel32_target(f + 0x341);
    let s = SCRATCH + 0x1_0000;
    // (state, count, sel[0])
    for (k, (state, n, b0)) in (0u32..).zip([
        (0u32, 5u32, 13u8),
        (0, 1, 2),
        (1, 7, 2),
        (1, 1, 13),
        (2, 9, 2),
        (2, 1, 2),
        (3, 6, 13),
        (3, 1, 2),
        (7, 3, 2),
        (1, 0, 2),
    ]) {
        stubs_0275(&mut t, k);
        magic_r11::object_slots(&mut t);
        stubs_0275(&mut t, k);
        for call in [0xccu32, 0x13c, 0x282] {
            let start = t.img.rel32_target(f + call);
            magic_r11::object_start_auto(&mut t, start);
        }
        sel_0275(&mut t, b0, 9, k);
        junk(&mut t, s, 0x100, k);
        t.set_u32(t.img.word(f + 0x1f), s);
        t.set_u32(s + 0xdc, (1.5 + k as f32 * 0.25).to_bits());
        // the summon's facing and place (`S + 0x4`, `+0x10`..`+0x1c`)
        for (o, v) in [
            (4u32, 0.75f32),
            (0x10, 2.5),
            (0x14, -1.25),
            (0x18, 3.75),
            (0x1c, 1.0),
        ] {
            t.set_u32(s + o, (v + k as f32 * 0.5).to_bits());
        }
        t.set_u32(s + 0xe0, state);
        t.set_u32(s + 0xe4, n);
        t.set_u32(t.img.word(f + 0x3cb), 0x03cb_0000 + k);
        junk_ops(
            &mut t,
            f,
            &[
                0xe1, 0xee, 0x106, 0x119, 0x137, 0x18e, 0x194, 0x1fa, 0x205, 0x28f, 0x2b1, 0x2be,
                0x2cb,
            ],
        );
        let node = SCRATCH + 0x900;
        magic_hi::lone_node(&mut t, give, node);
        ownb::dirty_stack(&mut t, 0x3a40 + k, &[]);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

/// `magic_0270`'s selection (two bytes at `*G_SEL`) and character
/// table (`G_T`), each entry its own junk character.
fn sel_0270(t: &mut Twin, b0: u8, b1: u8, k: u32) {
    let sel = SCRATCH + 0x180;
    t.write(sel, &[b0, b1, 0x77, 0x66]);
    t.set_u32(0x1026_3508, sel);
    for i in 0..0x20u32 {
        t.set_u32(0x1026_3544 + 4 * i, SCRATCH + 0x2_0000 + 0x900 * i);
    }
    for b in [b0, b1] {
        let c = SCRATCH + 0x2_0000 + 0x900 * u32::from(b);
        junk(t, c + 0x800, 0x40, k + u32::from(b));
    }
}

/// The 0x50-byte record ring `take` searches (`sa_misc::ring_take`): `n`
/// records at `recs`, those in `busy` taken, the search from `from`.
fn ring_50(t: &mut Twin, take: u32, recs: u32, n: u32, busy: &[u32], from: u32, k: u32) {
    let w = |t: &Twin, o: u32| t.img.word(take + o);
    let (next, count, base) = (w(t, 0x01), w(t, 0x09), w(t, 0x16));
    for i in 0..n {
        junk(t, recs + 0x50 * i, 0x50, k + i);
        t.set_u32(recs + 0x50 * i + 0x48, u32::from(busy.contains(&i)));
    }
    t.set_u32(next, from);
    t.set_u32(count, n);
    t.set_u32(base, recs);
}

/// `magic_0270`'s slots: as [`stubs_0275`] plus the random floats and
/// the handles of the sparks.
fn stubs_0270(t: &mut Twin, k: u32) {
    stubs_0275(t, k);
    for (s, x) in [(314u32, 0.375f64), (404, -0.625)] {
        rec(t, s, 1, move |m, a| {
            let c = m.r32(SCRATCH + 0xa00);
            m.w32(SCRATCH + 0xa00, c + 1);
            (s << 16) ^ a[0] ^ c
        });
        t.stub_st0(s, move |m, a| {
            x * f64::from(m.r32(SCRATCH + 0xa00) % 5) + f64::from(f32::from_bits(a[0]))
        });
    }
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0] ^ (a[2] << 8));
    rec(t, 328, 3, |m, a| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i).rotate_left(3) ^ a[2] ^ 0x328;
            m.w32(a[0] + 4 * i, v);
        }
        0x328
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x248
    });
}

/// `magic_0270` 0x100018a0 (`r38::burst_0270`, cdecl `(node)`) and
/// 0x10001460 (`r38::drop_0270`): the count running and out (the node
/// given back), the ring free, full and running out part way; the drop's
/// actor the caster and a selected one.
#[test]
fn bursts_0270() {
    let Some(mut t) = twin(270) else { return };
    let burst = t.at(0x18a0);
    let drop = t.at(0x1460);
    let recs = SCRATCH + 0x8000;
    // (count, free records, the drop's actor index)
    for (k, (count, free, idx)) in (0u32..).zip([
        (5i32, 8u32, u32::MAX),
        (1, 8, 1),
        (0, 8, 0),
        (3, 0, 2),
        (4, 8, 1),
        (2, 3, u32::MAX),
        (-4, 8, 1),
    ]) {
        stubs_0270(&mut t, k);
        sel_0270(&mut t, 4, 9, k);
        t.set_u32(0x1026_3510, 0x0000_0047);
        for (f, take_at, back_at) in [(burst, 0xa1u32, 0x20u32), (drop, 0x27d, 0x23)] {
            let take = t.img.rel32_target(f + take_at);
            let busy: Vec<u32> = (free..8).collect();
            ring_50(&mut t, take, recs, 8, &busy, k % 8, k);
            t.set_u32(SCRATCH + 0xa00, k);
            let node = SCRATCH + 0x900;
            let give = t.img.rel32_target(f + back_at);
            magic_hi::lone_node(&mut t, give, node);
            let r = SCRATCH + 0x980;
            junk(&mut t, r, 0x10, k);
            t.set_u32(r, idx);
            t.set_u32(r + 4, count.cast_unsigned());
            t.set_u32(
                node + 0x14,
                if f == burst { count.cast_unsigned() } else { r },
            );
            ownb::dirty_stack(&mut t, f + k, &[]);
            ownb::reset_counters(&mut t);
            t.check_entry(f, Call::default(), &[node], Eax::Checked);
        }
    }
}

/// `magic_0270` 0x10001cc0 (`r38::aim_0270`, internal): modes 0..4; in
/// mode 2 the two targets near and far apart in angle (both folds).
#[test]
fn aim_0270() {
    let Some(mut t) = twin(270) else { return };
    let f = t.at(0x1cc0);
    // (mode, second target's byte)
    for (k, (mode, b1)) in (0u32..).zip([
        (1u32, 9u8),
        (3, 9),
        (0, 9),
        (4, 9),
        (2, 9),
        (2, 0x13),
        (2, 4),
        (2, 0x1c),
    ]) {
        stubs_0270(&mut t, k);
        sel_0270(&mut t, 4, b1, k);
        t.set_u32(0x1026_3510, 0x0000_0047);
        t.set_u32(0x1026_3504, mode);
        junk_ops(&mut t, f, &[0x24b, 0x251]);
        t.set_u32(t.img.word(f + 0x25c), 0x5a5a_5a5a);
        ownb::dirty_stack(&mut t, 0x1cc0 + k, &[]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::r38::aim_0270(g, img, fr.at, fr.esp))
        });
    }
}

/// What `r38::start_up_0270` (`magic_0270` 0x10001f40) needs: the pools,
/// the part set-up's and the objects' buffers, the block's bytes, the
/// caster and the selection, the actor flags, the task list.
fn start_up_0270_setup(t: &mut Twin, f: u32, k: u32, listed: bool) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    stubs_0270(t, k);
    magic_r11::pools(t, f, 1058);
    magic_r11::object_slots(t);
    stubs_0270(t, k);
    for call in [0x7cu32, 0x15d, 0x1dc] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(t, start);
    }
    let reset = t.img.rel32_target(f + 0x3d4);
    magic_r11::pools(t, reset, 0xd0);
    sel_0270(t, 4, 9, k);
    t.set_u32(0x1026_3510, 0x0000_0047 + k);
    t.set_u32(w(t, 0x08), 0x0000_0049);
    t.set_u32(0x1026_3504, [1u32, 2, 3, 0][k as usize % 4]);
    // the block's entry bytes (its count is the mode word `+0x4`)
    t.write(SCRATCH + 0x1a0, &[3, 5, 7, 9]);
    rec(t, 407, 3, |_, _| SCRATCH + 0x1a0);
    t.set_u32(w(t, 0x30a), 0x030a_0000 + k);
    t.set_u32(w(t, 0x313), k & 1);
    t.set_u32(w(t, 0x34f), (k >> 1) & 1);
    junk_ops(
        t,
        f,
        &[
            0x1c, 0x26, 0x30, 0x61, 0xe2, 0xec, 0xfe, 0x108, 0x112, 0x11c, 0x126, 0x130, 0x13a,
            0x144, 0x154, 0x1a0, 0x1c4, 0x226, 0x230, 0x273, 0x31a, 0x326, 0x330, 0x35a, 0x360,
            0x36a, 0x374,
        ],
    );
    for o in [0x2a9u32, 0x2cd] {
        let at = w(t, o);
        t.write(at, &[0x5c; 0x38]);
    }
    // the colour bytes and the side halfword it clears
    t.write(w(t, 0x244), &[0x5c; 0x20]);
    t.set_u32(w(t, 0x37e), 0x5a5a_5a5a);
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    let head = w(t, 0x39c);
    t.set_u32(head, if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x3b0), SCRATCH + 0x600);
}

/// `magic_0270` 0x10001f40 (`r38::start_up_0270`, internal) and its load
/// step 0x100012b0 (`r38::load_0270`): the task list empty and not, both
/// actor slots and sides, the aim's modes.
#[test]
fn start_up_0270() {
    let Some(mut t) = twin(270) else { return };
    let f = t.at(0x1f40);
    for (k, listed) in [(0u32, false), (1, true), (2, true), (3, false)] {
        start_up_0270_setup(&mut t, f, k, listed);
        ownb::dirty_stack(&mut t, 0x1f40 + k, &[]);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r38::start_up_0270,
        );
    }
    let load = t.at(0x12b0);
    load_cases(&mut t, load, &|t| start_up_0270_setup(t, f, 2, true));
}

/// `magic_0380`'s selection (bytes at `*G_SEL`), character table and the
/// target `*G_T`.
fn sel_0380(t: &mut Twin, k: u32) {
    let sel = SCRATCH + 0x180;
    t.write(sel, &[4, 9, 0x13, 2]);
    t.set_u32(0x101d_b448, sel);
    for i in 0..0x20u32 {
        t.set_u32(0x101d_b484 + 4 * i, SCRATCH + 0x2_0000 + 0x900 * i);
    }
    t.set_u32(0x101d_b450, SCRATCH + 0x2_0000 + 0x900 * 7 + k);
    t.set_u32(0x101d_b440, 0x0000_0047 + k);
}

/// `magic_0380`'s slots: as [`stubs_0270`] plus the bones (slot 237 a
/// point, 236 a matrix) and a scale (slot 277).
fn stubs_0380(t: &mut Twin, k: u32) {
    stubs_0270(t, k);
    rec(t, 237, 4, |m, a| {
        for i in 0..4u32 {
            let v =
                rf(m, a[2] + 4 * i) * 0.25 + ((a[0] ^ (a[0] >> 11)) & 0x3f) as f32 - 3.0 * i as f32;
            wf(m, a[3] + 4 * i, v);
        }
        0x0237_0000 ^ a[1]
    });
    rec(t, 236, 3, |m, a| {
        for i in 0..16u32 {
            wf(
                m,
                a[2] + 4 * i,
                (a[0] & 0x1f) as f32 * 0.5 + i as f32 * 0.25 - 1.0,
            );
        }
        0x236
    });
    rec(t, 277, 1, |_, a| 0x0277_0000 ^ a[0]);
    t.stub_st0(277, |_, a| 0.5 + f64::from(a[0] & 7));
}

/// `magic_0380` 0x10001a90 (`r38::at_bone_0380`, cdecl `(obj)`) with its
/// bone matrix 0x10001b60.
#[test]
fn at_bone_0380() {
    let Some(mut t) = twin(380) else { return };
    let f = t.at(0x1a90);
    for k in 0..2u32 {
        stubs_0380(&mut t, k);
        sel_0380(&mut t, k);
        let obj = SCRATCH + 0x3000;
        junk(&mut t, obj, 0x140, k);
        ownb::dirty_stack(&mut t, 0x1a90 + k, &[]);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

/// `magic_0380` 0x10001770 (`r38::waits_0380`) with the facing test
/// 0x10001800: no actors; delays running and out; flags off (facing and
/// not) and on with counts running, at their end and out.
#[test]
fn waits_0380() {
    let Some(mut t) = twin(380) else { return };
    let f = t.at(0x1770);
    let recs = t.img.word(f + 0x13) - 4;
    // (actors, records: delay, count, flag)
    let cases: [(u32, [(i32, i32, u32); 3]); 4] = [
        (0, [(0, 0, 0); 3]),
        (3, [(2, 5, 0), (0, 3, 0), (0, 2, 1)]),
        (3, [(0, 1, 1), (0, 0, 1), (-1, -2, 1)]),
        (2, [(0, 4, 0), (1, 1, 1), (0, 0, 0)]),
    ];
    for (k, (n, rs)) in (0u32..).zip(cases) {
        stubs_0380(&mut t, k);
        sel_0380(&mut t, k);
        t.set_u32(t.img.word(f + 0xc), n);
        // the second actor (byte 9) stands where the bone's aim points:
        // its facing test answers 1
        let aimed = SCRATCH + 0x2_0000 + 0x900 * 9;
        rec(&mut t, 229, 3, |m, a| {
            for i in 0..4u32 {
                wf(m, a[0] + 4 * i, [3.5f32, 4.0, 5.25, 1.0][i as usize]);
            }
            0x229
        });
        rec(&mut t, 262, 4, move |m, a| {
            for i in 0..3u32 {
                let v = if a[0] == aimed {
                    [3.5f32, 4.0, 5.25][i as usize]
                } else {
                    -2.0 - i as f32
                };
                wf(m, a[1 + i as usize], v);
            }
            0x262
        });
        for (i, (d, c, fl)) in (0u32..).zip(rs) {
            t.set_u32(recs + 0xc * i, d.cast_unsigned());
            t.set_u32(recs + 0xc * i + 4, c.cast_unsigned());
            t.set_u32(recs + 0xc * i + 8, fl);
        }
        ownb::dirty_stack(&mut t, 0x1770 + k, &[]);
        let c = Call {
            eax: 0x0eaa_0000 + k,
            ..Call::default()
        };
        t.check_entry(f, c, &[], Eax::Checked);
    }
}

/// `magic_0380` 0x100013a0 (`r38::throw_0380`, cdecl `(node)`): the
/// delay running, and out (the objects, the emitter, the node given
/// back).
#[test]
fn throw_0380() {
    let Some(mut t) = twin(380) else { return };
    let f = t.at(0x13a0);
    let give = t.img.rel32_target(f + 0x3b3);
    let s = SCRATCH + 0x1_0000;
    for (k, delay) in (0u32..).zip([3i32, 0, -2]) {
        stubs_0380(&mut t, k);
        magic_r11::object_slots(&mut t);
        stubs_0380(&mut t, k);
        for call in [0x56u32, 0x313] {
            let start = t.img.rel32_target(f + call);
            magic_r11::object_start_auto(&mut t, start);
        }
        let aim = t.img.rel32_target(f + 0x277);
        let node = SCRATCH + 0x900;
        magic_hi::lone_node(&mut t, give, node);
        sel_0380(&mut t, k);
        junk(&mut t, s, 0x500, k);
        for (o, v) in [
            (0xc0u32, 0.5f32),
            (0xc4, -2.5),
            (0xd0, 1.0),
            (0xd4, 2.0),
            (0xd8, -3.0),
            (0xdc, 1.0),
        ] {
            t.set_u32(s + o, (v + k as f32).to_bits());
        }
        t.set_u32(s + 0x4d0, delay.cast_unsigned());
        magic_r11::pools(&mut t, aim, 0x230);
        t.set_u32(node + 0x14, s);
        ownb::dirty_stack(&mut t, 0x13a0 + k, &[]);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

/// What `r38::start_up_0380` (`magic_0380` 0x10001ca0) needs: the ring
/// (16 nodes), the objects' and part set-up's buffers, the actors, the
/// task list (empty or not).
fn start_up_0380_setup(t: &mut Twin, f: u32, k: u32, listed: bool) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    stubs_0380(t, k);
    magic_r11::pools(t, f, 1565);
    magic_r11::object_slots(t);
    stubs_0380(t, k);
    for call in [0x56u32, 0x96] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(t, start);
    }
    let task = t.img.rel32_target(f + 0x1a2);
    magic_r11::pools(t, task, 0xb0);
    magic_r11::ring(t, w(t, 0xa6), w(t, 0xc1), 16, SCRATCH + 0x5000);
    sel_0380(t, k);
    t.set_u32(w(t, 0x174), [3u32, 0, 2, 0xffff_fffe][k as usize % 4]);
    t.set_u32(w(t, 0x1b0), k & 1);
    t.set_u32(w(t, 0x1e8), k & 1);
    t.set_u32(w(t, 0x265), (k >> 1) & 1);
    // junk in the globals it stores to
    junk_ops(
        t,
        f,
        &[
            0x1d, 0x27, 0x35, 0x64, 0x1b6, 0x1c2, 0x1cc, 0x1f5, 0x1fb, 0x205, 0x20f, 0x22e, 0x23a,
            0x244, 0x26b, 0x271, 0x27b, 0x285, 0x2c9, 0x2d3, 0x2dd, 0x2e7, 0x2f1, 0x2f7, 0x3b2,
            0x3bc, 0x3c6, 0x3d0, 0x3da, 0x490, 0x49a, 0x4a4, 0x4ae, 0x4b8, 0x4c2, 0x4cc,
        ],
    );
    for o in [
        0x21bu32, 0x224, 0x28f, 0x298, 0x2be, 0x2fd, 0x3a9, 0x3e4, 0x489,
    ] {
        let at = w(t, o);
        t.set_u32(at, 0x5a5a_5a5a ^ o);
    }
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    let first = w(t, 0x12b);
    t.set_u32(first, if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x13d), SCRATCH + 0x600);
}

/// `magic_0380` 0x10001ca0 (`r38::start_up_0380`, internal) and its load
/// step 0x10001210 (`r38::load_0380`): the task list empty and not, the
/// actor counts, both slots and sides.
#[test]
fn start_up_0380() {
    let Some(mut t) = twin(380) else { return };
    let f = t.at(0x1ca0);
    for (k, listed) in [(0u32, false), (1, true), (2, true), (3, false)] {
        start_up_0380_setup(&mut t, f, k, listed);
        ownb::dirty_stack(&mut t, 0x1ca0 + k, &[]);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r38::start_up_0380,
        );
    }
    let load = t.at(0x1210);
    load_cases(&mut t, load, &|t| start_up_0380_setup(t, f, 2, true));
}

/// `magic_0298`'s selection (bytes at `*G_SEL`), character table, caster
/// and actor count.
fn sel_0298(t: &mut Twin, n: u32, k: u32) {
    let sel = SCRATCH + 0x180;
    t.write(sel, &[4, 9, 0x13, 2, 6]);
    t.set_u32(0x1030_7608, sel);
    for i in 0..0x20u32 {
        t.set_u32(0x1030_7644 + 4 * i, SCRATCH + 0x2_0000 + 0x900 * i);
    }
    t.set_u32(0x1030_7610, 0x0000_0047 + k);
    t.set_u32(0x1030_7600, 0x0000_0051 + k);
    t.set_u32(0x1030_7604, n);
}

/// `magic_0298` 0x10001b90 (`r38::pose_0298`, internal).
#[test]
fn pose_0298() {
    let Some(mut t) = twin(298) else { return };
    let f = t.at(0x1b90);
    for k in 0..2u32 {
        stubs_0380(&mut t, k);
        sel_0298(&mut t, 2, k);
        // the pose matrix as slot 327 sees it, folded into a compared word
        rec(&mut t, 327, 2, |m, a| {
            let mut h = m.r32(SCRATCH + 0xb10);
            for i in 0..16 {
                h = h.rotate_left(5) ^ m.r32(a[1] + 4 * i);
            }
            m.w32(SCRATCH + 0xb10, h);
            0x327
        });
        for (o, v) in [
            (0x45u32, 1.5f32),
            (0x5d, 2.5),
            (0x6a, -3.5),
            (0x77, 1.0),
            (0xbc, 0.75),
            (0xe1, -1.25),
        ] {
            t.set_u32(t.img.word(f + o), (v + k as f32).to_bits());
        }
        ownb::dirty_stack(&mut t, 0x1b90 + k, &[]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::r38::pose_0298(g, img, fr.at, fr.esp))
        });
    }
}

/// `magic_0298` 0x10001e60 (`r38::actor_step_0298`, cdecl `(node)`): the
/// delay running; the motion script waiting, ending (slot 239) and done;
/// the actor the caster or a selected one; the count running, at its end
/// with the record done or not.
#[test]
fn actor_step_0298() {
    let Some(mut t) = twin(298) else { return };
    let f = t.at(0x1e60);
    let give = t.img.rel32_target(f + 0x166);
    let s = SCRATCH + 0x1_0000;
    let cmds = SCRATCH + 0x5000;
    // (delay, done, wait, actor, count)
    for (k, (delay, done, wait, actor, count)) in (0u32..).zip([
        (2i32, 0u32, 0u16, 1u32, 5i32),
        (0, 0, 3, 1, 5),
        (0, 0, 0, u32::MAX, 2),
        (0, 1, 0, 0, 1),
        (0, 1, 0, 2, 0),
        (0, 2, 1, 1, -3),
        (0, 0, 0, 1, 1),
    ]) {
        stubs_0380(&mut t, k);
        sel_0298(&mut t, 2, k);
        rec(&mut t, 411, 1, |_, a| 0x0411_0000 ^ a[0]);
        junk(&mut t, s, 0x60, k);
        t.write(cmds, &[0x5a; 8]);
        t.set_u32(cmds + 8, u32::MAX);
        t.set_u32(cmds + 12, u32::MAX);
        t.set_u32(s, cmds);
        t.set_u32(s + 0x10, 0);
        t.set_u32(s + 0x14, done);
        t.set_u16(s + 0x18, wait);
        t.set_u16(s + 0x1c, 0);
        for (i, v) in (0u32..).zip([0.5f32, -1.5, 2.25, 0.0, 0.0, 0.0, 0.125, 0.75, -0.5]) {
            t.set_u32(s + 0x20 + 4 * i, (v + k as f32 * 0.25).to_bits());
        }
        t.set_u32(s + 0x40, delay.cast_unsigned());
        t.set_u32(s + 0x44, count.cast_unsigned());
        t.set_u32(s + 0x48, actor);
        t.set_u32(s + 0x4c, 0x004c_0000 + k);
        let node = SCRATCH + 0x900;
        magic_hi::lone_node(&mut t, give, node);
        t.set_u32(node + 0x14, s);
        ownb::dirty_stack(&mut t, 0x1e60 + k, &[]);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

/// What `r38::start_up_0298` (`magic_0298` 0x10001390) needs: the ring
/// (16 nodes), the pools, the part set-up's and the object start's
/// buffers, the actors, the task list (empty or not).
fn start_up_0298_setup(t: &mut Twin, f: u32, k: u32, listed: bool) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    stubs_0380(t, k);
    magic_r11::pools(t, f, 2033);
    magic_r11::object_slots(t);
    stubs_0380(t, k);
    let start = t.img.rel32_target(f + 0x6f);
    magic_r11::object_start_auto(t, start);
    let rn = t.img.rel32_target(f + 0x51b);
    magic_r11::pools(t, rn, 0x88);
    magic_r11::ring(t, w(t, 0x419), w(t, 0x469), 16, SCRATCH + 0x5000);
    sel_0298(t, [2u32, 0, 3, 1][k as usize % 4], k);
    let bytes = SCRATCH + 0x1a0;
    t.write(bytes, &[3, 5, 7]);
    rec(t, 288, 2, move |m, a| {
        m.w32(a[1], 2);
        bytes
    });
    t.set_u32(0x1030_760c, k & 1);
    junk(t, w(t, 0x1d0) - 4, 0x54 * 4, k);
    junk_ops(
        t,
        f,
        &[
            0x33, 0x3d, 0x47, 0x190, 0x6c3, 0x6cf, 0x6d9, 0x6fe, 0x704, 0x70e, 0x718, 0x726, 0x55,
            0x108, 0x122, 0x12c, 0x154, 0x15e, 0x168, 0x172, 0x17c, 0x186, 0x196,
        ],
    );
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    let first = w(t, 0x4d0);
    t.set_u32(first, if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x4e5), SCRATCH + 0x600);
}

/// `magic_0298` 0x10001390 (`r38::start_up_0298`, internal) and its load
/// step 0x10001260 (`r38::load_0298`): the task list empty and not, no
/// actors and up to three, both slots and sides.
#[test]
fn start_up_0298() {
    let Some(mut t) = twin(298) else { return };
    let f = t.at(0x1390);
    for (k, listed) in [(0u32, false), (1, true), (2, true), (3, false)] {
        start_up_0298_setup(&mut t, f, k, listed);
        ownb::dirty_stack(&mut t, 0x1390 + k, &[]);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r38::start_up_0298,
        );
    }
    let load = t.at(0x1260);
    load_cases(&mut t, load, &|t| start_up_0298_setup(t, f, 2, true));
}

/// `magic_0430`'s selection (bytes at `*G_SEL`), character table, the
/// state block `S` (junk with real floats at its point, facing and
/// scales).
fn state_0430(t: &mut Twin, k: u32) -> u32 {
    let sel = SCRATCH + 0x180;
    t.write(sel, &[4, 9, 0x13, 2]);
    t.set_u32(0x102d_5ba8, sel);
    for i in 0..0x20u32 {
        t.set_u32(0x102d_5be4 + 4 * i, SCRATCH + 0x2_0000 + 0x900 * i);
    }
    let s = SCRATCH + 0x1_0000;
    junk(t, s + 0xa00, 0x300, k);
    for (o, v) in [
        (0xcc4u32, 1.5f32),
        (0xcc8, -2.0),
        (0xccc, 3.25),
        (0xcd0, 1.0),
        (0xeb4, 0.75),
        (0xc84, 0.5),
        (0xc88, -0.25),
        (0xc8c, 1.25),
    ] {
        t.set_u32(s + o, (v + k as f32 * 0.5).to_bits());
    }
    t.set_u32(0x100f_f7ac, s);
    s
}

/// `magic_0430` 0x10001730 (`r38::task_0430`, cdecl `(node)`) with the
/// effect end 0x10001ef0: the first script running and done (the end
/// with its flags and actors both ways, the held node and none).
#[test]
fn task_0430() {
    let Some(mut t) = twin(430) else { return };
    let f = t.at(0x1730);
    let end = t.img.rel32_target(f + 0x14b);
    let give = t.img.rel32_target(f + 0x156);
    let sr = t.img.rel32_target(f + 0x38);
    let ms = t.img.rel32_target(f + 0x116);
    let rigs = t.img.rel32_target(f + 0x13c);
    // (script done, flags, held node)
    for (k, (done, flag, held)) in (0u32..).zip([
        (false, 0u32, false),
        (true, 0, true),
        (true, 1, false),
        (true, 0, false),
    ]) {
        stubs_0380(&mut t, k);
        recs(
            &mut t,
            &[
                (217, 2),
                (255, 2),
                (211, 2),
                (213, 2),
                (296, 2),
                (257, 2),
                (371, 2),
            ],
        );
        // the party run's list: two actors
        let party = SCRATCH + 0x1c0;
        t.write(party, &[3, 6]);
        rec(&mut t, 288, 2, move |m, a| {
            m.w32(a[1], 2);
            party
        });
        let s = state_0430(&mut t, k);
        let _ = s;
        // the scripts: the first's context `w(0x2e)` done or running
        let op = |t: &Twin, o: u32| t.img.word(f + o);
        let ctx1 = op(&t, 0x2e);
        let ctx2 = op(&t, 0x3d);
        junk(&mut t, ctx1, 0x60, k);
        junk(&mut t, ctx2, 0x60, k + 1);
        t.set_u8(ctx1 + 0x56, if done { 0 } else { 4 });
        t.set_u8(ctx2 + 0x56, 4);
        let _ = sr;
        for o in [0x10bu32, 0x11b, 0x125, 0x12f] {
            let at = op(&t, o);
            junk(&mut t, at, 0x20, k + o);
            t.set_u32(at + 0x14, 1);
        }
        let _ = (ms, rigs);
        // the end's flags and actors
        let ew = |t: &Twin, o: u32| t.img.word(end + o);
        for o in [0x60u32, 0xbd, 0x112, 0x1b5, 0x20b] {
            let at = ew(&t, o);
            t.set_u32(at, flag);
        }
        t.set_u32(ew(&t, 0xb4), 0x00b4_0000);
        t.set_u32(ew(&t, 0xc6), 3 + 0x40 * flag);
        t.set_u32(ew(&t, 0x213), 5 + 0x40 * flag);
        t.set_u32(ew(&t, 0x235), 0x0235_0000);
        t.set_u16(ew(&t, 0x168), u16::try_from(flag).unwrap_or(0));
        t.set_u16(ew(&t, 0x18a), u16::try_from(k & 1).unwrap_or(0));
        let lock = SCRATCH + 0xa90;
        t.set_u32(ew(&t, 0x2af), lock);
        t.set_u32(lock, 0x0bad_0000 + k);
        let node = SCRATCH + 0x900;
        magic_hi::lone_node(&mut t, give, node);
        let held_node = SCRATCH + 0x9c0;
        let back2 = t.img.rel32_target(end + 0x274);
        magic_hi::lone_node(&mut t, back2, held_node);
        t.set_u32(ew(&t, 0x268), if held { held_node } else { 0 });
        ownb::dirty_stack(&mut t, 0x1730 + k, &[]);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

/// `magic_0430` 0x100022e0 (`r38::spark_0430`, cdecl `(obj)`): past 0x78
/// steps; a free record and none.
#[test]
fn spark_0430() {
    let Some(mut t) = twin(430) else { return };
    let f = t.at(0x22e0);
    let take = t.img.rel32_target(f + 0x52);
    let recs_at = SCRATCH + 0x8000;
    for (k, (steps, busy)) in (0u32..).zip([
        (0x79i32, &[][..]),
        (0x78, &[][..]),
        (3, &[0, 1, 2][..]),
        (-5, &[1][..]),
    ]) {
        stubs_0270(&mut t, k);
        state_0430(&mut t, k);
        pool_74(&mut t, take, recs_at, 3, busy, k % 3, k);
        t.set_u32(SCRATCH + 0xa00, k);
        let obj = SCRATCH + 0x3000;
        junk(&mut t, obj, 0x140, k);
        t.set_u32(obj + 0x118, steps.cast_unsigned());
        ownb::dirty_stack(&mut t, 0x22e0 + k, &[]);
        ownb::reset_counters(&mut t);
        let c = Call {
            eax: 0x0eaa_0000 + k,
            ..Call::default()
        };
        t.check_entry(f, c, &[obj], Eax::Checked);
    }
}

/// `magic_0430` 0x10002750 (`r38::ring_object_0430`), 0x10002bd0
/// (`r38::objects_0430`), 0x10003350 (`r38::summon_matrix_0430`) and
/// 0x10002ea0 (`r38::glow_draw_0430`).
#[test]
fn objects_0430() {
    let Some(mut t) = twin(430) else { return };
    for (k, rva) in (0u32..).zip([0x2750u32, 0x2bd0, 0x3350, 0x3350, 0x2ea0, 0x2ea0, 0x2ea0]) {
        let f = t.at(rva);
        stubs_0380(&mut t, k);
        magic_r11::object_slots(&mut t);
        for p in f..f + 0x1a0 {
            if t.img.word(p).to_le_bytes()[0] == 0xe8 {
                let c = t.img.rel32_target(p + 1);
                if t.img.matches(c, &ffx_magic::s4_misc::START_OBJECT_ANIM) {
                    magic_r11::object_start_auto(&mut t, c);
                }
            }
        }
        let s = state_0430(&mut t, k);
        rec(
            &mut t,
            520,
            0,
            move |_, _| if k == 2 { 0x018b_0046 } else { 0x520 },
        );
        for (sl, n) in [
            (330u32, 3usize),
            (15, 1),
            (16, 1),
            (17, 1),
            (567, 5),
            (723, 1),
            (724, 1),
        ] {
            rec(&mut t, sl, n, move |_, a| (sl << 16) ^ a[0]);
        }
        for sl in [15u32, 16, 17] {
            t.stub_st0(sl, move |_, a| 0.5 + f64::from(a[0] & 3) + f64::from(sl));
        }
        t.set_u32(s + 0xec4, [0u32, 0, 0, 0, 3, 0, 0xffff_fff0][k as usize]);
        // the draw descriptor's never-written words read 0 on both sides
        ownb::dirty_stack(&mut t, rva + k, &[0x148, 0x120, 0x104]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0430` 0x10002a60 (`r38::part_matrix_0430`, cdecl `(a, b)`).
#[test]
fn part_matrix_0430() {
    let Some(mut t) = twin(430) else { return };
    let f = t.at(0x2a60);
    for k in 0..2u32 {
        stubs_0380(&mut t, k);
        state_0430(&mut t, k);
        let (a, b, p) = (SCRATCH + 0x3000, SCRATCH + 0x3400, SCRATCH + 0x3800);
        junk(&mut t, a, 0x180, k);
        junk(&mut t, b, 0x100, k + 1);
        let e = SCRATCH + 0x3900;
        t.set_u32(a + 8, e);
        t.set_u8(e + 0xe, 2 + u8::try_from(k).unwrap_or(0));
        for (o, v) in [(0x60u32, 1.5f32), (0x64, 2.0), (0x68, 0.5)] {
            t.set_u32(b + o, (v + k as f32).to_bits());
        }
        for i in 0..3u32 {
            t.set_u32(p + 4 * i, (1.0 + i as f32 - k as f32).to_bits());
        }
        rec(&mut t, 941, 1, move |_, _| p);
        rec(&mut t, 554, 3, |m, a| {
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * (i % 4));
                wf(m, a[0] + 4 * i, v);
            }
            0x554
        });
        rec(&mut t, 552, 2, |m, a| {
            for i in 0..4u32 {
                let v = rf(m, a[1] + 4 * (i % 4)) - 1.0;
                wf(m, a[0] + 4 * i, v);
            }
            0x552
        });
        ownb::dirty_stack(&mut t, 0x2a60 + k, &[]);
        t.check_entry(f, Call::default(), &[a, b], Eax::Checked);
    }
}
