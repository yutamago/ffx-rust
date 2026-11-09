//! Round 29's ports (`ffx_magic::r29`) against each original. Needs
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
mod magic_hi;
mod magic_owna;
#[path = "magic/ownb.rs"]
mod ownb;

use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use magic_owna::twin;
use ownb::rec;

/// Junk words at the globals the operands `ops` of `f` name.
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, (0x5a5a_5a5a ^ o) | 1);
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

/// Every slot answering `first word ^ (slot << 20)` and logging its first
/// word (into `SCRATCH + 0xa00..`).
fn all_stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |m, a| {
            let k = m.r32(SCRATCH + 0xa00);
            if let Some(w) = a.first() {
                m.w32(SCRATCH + 0xa10 + 4 * (k % 0x100), *w ^ s);
            }
            m.w32(SCRATCH + 0xa00, k + 1);
            a.first().copied().unwrap_or(0) ^ (s << 20)
        });
    }
}

/// The matrix slots (as `magic_r27.rs`'s `stubs_0586`): 235 a unit
/// matrix, 330/328/329 mixing their angle into the source, 229/248 every
/// word read, the scale slots 15/16/17/277 floats.
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

/// The globals `start_object_anim` at `so` reads (as `magic_r27.rs`'s
/// `start_env_at`, operands `+0x99`/`+0xa4`): its item and model words,
/// slot 941's part table, the pool (`count` free nodes) and the object
/// list's tail `tl`.
fn start_env(t: &mut Twin, so: u32, count: u16, tl: u32) {
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
    let pa = t.img.rel32_target(so + 0x99);
    let free = t.img.word(pa + 0x04);
    let nodes = t.img.word(pa + 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    t.set_u16(free, count);
    t.set_u32(t.img.word(so + 0xa4), tl);
}

/// `magic_0511` 0x10003260 (`r29::three_objects_0511`): the yaw wrapped
/// both ways, kept and NaN, the height varied, the object pool with nodes
/// and empty.
#[test]
fn three_objects_0511() {
    let Some(mut t) = twin(511) else { return };
    all_stubs(&mut t);
    matrix_stubs(&mut t);
    let f = t.at(0x3260);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let so = t.img.rel32_target(f + 0x4c);
    let s = SCRATCH + 0x1_0000;
    for (k, (count, yaw, h)) in (0u32..).zip([
        (3u16, 0.5f32, 0.75f32),
        (0, 3.0, -2.5),
        (1, -3.0, 0.0),
        (2, f32::NAN, 1.25),
    ]) {
        start_env(&mut t, so, count, if k == 1 { SCRATCH + 0x680 } else { 0 });
        t.set_u32(op(&t, 0x1b), s);
        junk(&mut t, s, 0x2300, k);
        for i in 0..0x50u32 {
            t.set_u32(
                s + 0x1980 + 4 * i,
                (0.25 * i as f32 - 3.0 + k as f32).to_bits(),
            );
        }
        t.set_u32(s + 0x2258, yaw.to_bits());
        for o in [0x1ac, 0x2e0, 0x455] {
            t.set_u32(op(&t, o), h.to_bits());
        }
        ownb::dirty_stack(&mut t, 0x3260 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0511` 0x100036d0 (`r29::fading_summons_0511`): the count
/// running, at 0 and below; the selected actors varied.
#[test]
fn fading_summons_0511() {
    let Some(mut t) = twin(511) else { return };
    all_stubs(&mut t);
    matrix_stubs(&mut t);
    rec(&mut t, 31, 2, |_, a| 0x0031_0000 ^ a[0] ^ a[1]);
    let f = t.at(0x36d0);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let s = SCRATCH + 0x1_0000;
    let sel = SCRATCH + 0x600;
    let task = SCRATCH + 0x100;
    for (k, n) in (0u32..).zip([20i32, 1, 0, -3, 7]) {
        t.set_u32(op(&t, 0xf), s);
        junk(&mut t, s + 0x1800, 0x300, k);
        for i in 0..0x60u32 {
            t.set_u32(
                s + 0x18c0 + 4 * i,
                (0.25 * i as f32 - 3.0 + k as f32).to_bits(),
            );
        }
        for i in 0..0x18u32 {
            t.set_u32(s + 0x1a00 + 4 * i, 0x0001_0000 * (i + 1) + k);
        }
        junk(&mut t, task, 0x40, k + 1);
        t.set_u32(task + 0x2c, n.cast_unsigned());
        t.set_u32(op(&t, 0x47), sel);
        t.write(sel, &[7, 9, 9, (k % 3) as u8, ((k + 1) % 3) as u8, 2, 9, 9]);
        for i in 0..4u32 {
            t.set_u32(op(&t, 0x52) + 4 * i, 0x00ac_0000 + 0x10 * i + k);
        }
        ownb::dirty_stack(&mut t, 0x36d0 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// The trail draw's slots and globals (as `magic_r22.rs`'s
/// `flight_step_0491` set-up) for the trail draw at `td`.
fn trail_stubs(t: &mut Twin, td: u32) {
    let lock = SCRATCH + 0x9000;
    t.set_u32(t.img.word(td + 0x463), lock);
    t.set_u32(t.img.word(td + 0x520), 0x0cc0_0cc0);
    for s in [723u32, 724] {
        rec(t, s, 1, move |m, a| {
            for i in 0..16 {
                wf(m, a[0] + 4 * i, 0.5 * i as f32 + 0.5);
            }
            s
        });
    }
    rec(t, 234, 3, |m, a| {
        for i in 0..3 {
            let v = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x234
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..3 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
            fold(m, 248, i, v.to_bits());
        }
        0x248
    });
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[2] + 4 * i) * 0.5 + rf(m, a[1] + 4 * i) * 0.25 + 0.5 + i as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    let gp = t.img.word(td + 0x63);
    rec(t, 772, 2, move |m, a| {
        let sm = m.r32(gp);
        for j in 0..12 {
            let v = m.r32(sm + 4 * j);
            fold(m, 772, 3 + j, v);
        }
        0x7720_0000 ^ a[1]
    });
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    rec(t, 773, 4, |_, a| (a[3] >> 1) & 1);
    for s in [774u32, 775, 778, 779] {
        rec(t, s, 2, move |_, a| s ^ a[0]);
    }
    rec(t, 776, 1, |_, a| a[0] << 4);
    rec(t, 777, 1, |_, a| 0x0ff0 ^ a[0]);
}

/// The trail block at `obj`: `n` sprites.
fn trail_block(t: &mut Twin, obj: u32, n: u32, k: u32) {
    t.write(obj, &[0x3c; 0x230]);
    t.set_u32(obj + 4, 1);
    t.set_u16(obj + 8, (k % 3) as u16);
    t.set_u16(obj + 0xa, 0x100);
    for j in 0..32u32 {
        let r = 1.0 + j as f32 * 0.3;
        let a = j as f32 * 0.7;
        for (i, x) in (0u32..).zip([r * a.cos(), j as f32 * 0.2, r * a.sin(), 1.0]) {
            t.set_u32(obj + 0xc + 16 * j + 4 * i, x.to_bits());
        }
    }
    t.write(obj + 0x20c, &[10, 200, 0, 0x80, 250, 3, 128, 0x40]);
    for (off, x) in [(0x214u32, 1.5f32), (0x218, 0.25), (0x21c, 0.75)] {
        t.set_u32(obj + off, x.to_bits());
    }
    t.set_u32(obj + 0x220, 3 + k);
    t.set_u32(obj + 0x224, n);
}

/// The 0x74-record pool of `sb_misc::pool_take` at `take`: `count`
/// records from `recs`, the ones in `busy` taken.
fn pool74(t: &mut Twin, take: u32, recs: u32, count: u32, busy: u32) {
    let w = |t: &Twin, o: u32| t.img.word(take + o);
    t.set_u32(w(t, 0x37), recs);
    for i in 0..count {
        t.write(recs + 0x74 * i, &[0x5a; 0x74]);
        t.set_u32(recs + 0x74 * i + 0x30, 0x300 + i);
        t.set_u32(recs + 0x74 * i + 0x6c, (busy >> i) & 1);
    }
    t.set_u32(w(t, 0x0f), count);
    t.set_u32(w(t, 0x08), 0);
    t.set_u32(w(t, 0x2b), 0);
}

/// `magic_0511` 0x10003e70 (`r29::spray_0511`, cdecl `(task)`): the count
/// running and out with the fade above 8 and not (the task given back),
/// the particle count out, at 1..8 and above 8, the pool full, short
/// (one and three records) and empty.
#[test]
fn spray_0511() {
    let Some(mut t) = twin(511) else { return };
    all_stubs(&mut t);
    let f = t.at(0x3e70);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let td = t.img.rel32_target(f + 0xac);
    trail_stubs(&mut t, td);
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    rec(&mut t, 219, 2, |_, a| 0x0219_0000 ^ a[0]);
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            wf(m, a[2] + 4 * i, 0.5 * i as f32 - 1.25 + (a[0] & 3) as f32);
        }
        0x268
    });
    rec(&mut t, 314, 1, |m, a| {
        let k = m.r32(magic_owna::row(314));
        m.w32(magic_owna::row(314), k + 1);
        0x0314_0000 ^ k ^ a[0]
    });
    t.stub_st0(314, |m, a| {
        let k = m.r32(magic_owna::row(314));
        f64::from(f32::from_bits(a[0])) * (f64::from(k % 7) * 0.15 + 0.1) - 0.03
    });
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0] ^ a[1]);
    let take = t.img.rel32_target(f + 0xd1);
    let give = t.img.rel32_target(f + 0x46d);
    let s = SCRATCH + 0x1_0000;
    let recs = SCRATCH + 0x4000;
    let task = SCRATCH + 0x3e00;
    let tex = SCRATCH + 0x900;
    let actor = SCRATCH + 0x980;
    // (count, fade, particles, pool records (busy mask))
    let cases: [(i32, i32, i32, u32, u32); 9] = [
        (5, 30, 20, 8, 0),
        (1, 30, 9, 8, 0),
        (0, 20, 5, 8, 0),
        (0, 9, 3, 8, 0),
        (0, 8, 3, 8, 0),
        (-2, 3, 3, 8, 0),
        (3, 0, 0, 8, 0),
        (3, 0, 20, 3, 0b010),
        (3, 0, 20, 1, 0),
    ];
    for (k, (count, fade, parts, n, busy)) in (0u32..).zip(cases) {
        junk(&mut t, s + 0x2000, 0x300, k);
        t.set_u32(s + 0x2260, count.cast_unsigned());
        t.set_u32(s + 0x2254, fade.cast_unsigned());
        t.set_u32(s + 0x2264, parts.cast_unsigned());
        trail_block(&mut t, s + 0x2030, 2 + k % 3, k);
        t.set_u32(op(&t, 0x16), s);
        t.set_u32(op(&t, 0x5e), actor);
        t.set_u8(actor, 0x41 + k as u8);
        t.set_u32(op(&t, 0xe1), tex);
        t.set_u32(tex + 8, 0x7e00_0000 + k);
        pool74(&mut t, take, recs, n, busy);
        if k == 7 {
            pool74(&mut t, take, recs, 2, 0);
        }
        magic_hi::lone_node(&mut t, give, task);
        ownb::dirty_stack(&mut t, 0x3e70 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    assert!(t.u32(magic_owna::row(314)) >= 7 * 4, "the particles");
}

/// The slots the 0511/0602 start-up and the summons' matrices call (as
/// `magic_r28.rs`'s `mats_slots`, the load step's 225-227 from
/// `SCRATCH + 0x10/0x1c`).
fn start_stubs(t: &mut Twin, yaw: u32) {
    all_stubs(t);
    rec(t, 225, 1, |m, _| m.r32(SCRATCH + 0x10));
    rec(t, 226, 2, |_, a| 0x226 ^ a[0]);
    rec(t, 227, 1, |m, _| m.r32(SCRATCH + 0x1c));
    rec(t, 520, 0, |m, _| m.r32(SCRATCH + 0x20));
    for s in [5u32, 28] {
        rec(t, s, 1, move |m, a| m.r32(SCRATCH + 0x28) & (a[0] ^ s) & 1);
    }
    rec(t, 262, 4, move |m, a| {
        for i in 1..4 {
            wf(m, a[i], 0.75 * i as f32);
        }
        0x262
    });
    for s in [210u32, 211] {
        rec(t, s, 2, move |m, a| {
            for i in 0..16 {
                let v = (a[0] & 0xff) as f32 * 0.01 + i as f32 * 0.1 + s as f32 * 0.001;
                wf(m, a[1] + 4 * i, v);
            }
            s ^ a[0]
        });
    }
    magic_owna::filler(t, 235, 1, 16);
    for s in [328u32, 329, 330] {
        rec(t, s, 3, move |m, a| {
            let v = rf(m, a[1]) + f32::from_bits(a[2]);
            wf(m, a[0], v);
            s
        });
    }
    rec(t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, move |m, _| f64::from(rf(m, yaw)));
    for s in [277u32, 15, 16, 17] {
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
        (718, 1),
        (215, 1),
        (719, 3),
        (711, 3),
        (710, 4),
        (271, 3),
    ] {
        rec(t, s, n, move |_, a| (0x5a5a_0000 + s) ^ a[0]);
    }
}

/// The load steps `magic_0511` 0x100012e0 (`r29::load_0511`, the 118-byte
/// shape) and `magic_0602` 0x100068a0 (`r29::load_0602`, the sounds'
/// check), both running `r29::start_up_0511`: slot 225 busy, the data not
/// loaded yet (asked once), the start-up with the task list empty and not
/// and in the scene 0x18b0046 and not; `magic_0602`'s sounds busy too.
#[test]
fn load_start_up_0511() {
    for (n, rva, sounds) in [(511u16, 0x12e0u32, false), (602, 0x68a0, true)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        let w = |t: &Twin, at: u32, o: u32| t.img.word(at + o);
        let su = t.img.rel32_target(f + if sounds { 0x67 } else { 0x5b });
        let yaw = SCRATCH + 0x90;
        start_stubs(&mut t, yaw);
        let lock = SCRATCH + 0x7f0;
        let pi = t.img.rel32_target(su + 0xa7);
        t.set_u32(w(&t, pi, 0x04), lock);
        let take = t.img.rel32_target(su + 0x115);
        let mats = t.img.rel32_target(su + 0xac);
        let ri = t.img.rel32_target(su + 0x154);
        let e = SCRATCH + 0x8000;
        let sel = SCRATCH + 0x5000;
        // (225, asked, 227, task list, scene, sounds busy)
        let cases: [(u32, u32, u32, bool, u32, u32); 6] = [
            (1, 0, 1, false, 0, 0),
            (0, 0, 0, false, 0, 0),
            (0, 1, 1, false, 0x018b_0046, 0),
            (0, 0, 2, true, 0x018b_0047, 0),
            (0, 1, 1, true, 0x018b_0046, 0),
            (0, 1, 1, false, 0, 1),
        ];
        for (k, (w225, asked, r227, busy, scene, snd)) in (0u32..).zip(cases) {
            t.set_u32(SCRATCH + 0x10, w225);
            t.set_u32(SCRATCH + 0x1c, r227);
            t.set_u32(SCRATCH + 0x20, scene);
            t.set_u32(SCRATCH + 0x28, snd);
            t.set_u32(yaw, (0.5 + k as f32).to_bits());
            let asked_op = if sounds { 0x22 } else { 0x18 };
            t.set_u32(w(&t, f, asked_op), asked);
            t.set_u32(lock, 0x77 + k);
            t.set_u32(w(&t, su, 0x15), 3 + k);
            // the summons' matrices (as `magic_r28.rs`'s `mats_0511`)
            junk(&mut t, e + 0x1700, 0xc00, k);
            t.set_u32(w(&t, mats, 0x1b), e);
            t.write(sel, &[2, 1, (k % 3) as u8, 3]);
            t.set_u32(w(&t, mats, 0x11), sel);
            for i in 0..0x40 {
                t.set_u32(w(&t, mats, 0x2e) + 4 * i, 0x0c0c_0100 + i);
            }
            t.set_u32(w(&t, mats, 0x167), k & 1);
            t.set_u32(w(&t, mats, 0x36c), 3);
            junk_ops(&mut t, su, &[0x1ac, 0x1b6]);
            let node = |i: u32| SCRATCH + 0x6000 + 0x40 * i;
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
            t.set_u32(w(&t, su, 0x11b), if busy { last } else { 0 });
            t.set_u32(w(&t, su, 0x130), last);
            ownb::dirty_stack(&mut t, rva + k, &[]);
            ownb::reset_counters(&mut t);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
        assert!(t.u32(ownb::LOG) > 0x100, "the start-up's calls");
    }
}

/// `magic_0612` 0x10009650 and `magic_0563` 0x10004d20
/// (`r29::spin_states_0612`): every state (the set-up with an empty pool,
/// the step counting, the records' move, others) before the models' draw.
#[test]
fn spin_states_0612() {
    for (n, rva) in [(612u16, 0x9650u32), (563, 0x4d20)] {
        let Some(mut t) = twin(n) else { return };
        all_stubs(&mut t);
        let f = t.at(rva);
        let s = SCRATCH + 0x1_0000;
        let setup = t.img.rel32_target(f + 0x2b);
        let around = t.img.rel32_target(f + 0x30);
        let table = SCRATCH + 0x400;
        rec(&mut t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
        t.stub_st0(12, |_, _| -10.0);
        // the models' draw: slot 772's 64-bit handle
        rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[0]);
        t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
        rec(&mut t, 562, 0, |_, _| 0x562);
        for st in [3u32, 2, 1, 0, 0xffff_ffff, 7] {
            t.write(s + 0x2000, &vec![0; 0x1c00]);
            for i in 0..0x300u32 {
                t.set_u32(s + 0x2b00 + 4 * i, ((i % 7) as f32 * 0.5 - 1.0).to_bits());
            }
            t.set_u32(s + 0x3664, st);
            t.set_u32(s + 0x3660, 3);
            t.set_u32(t.img.word(f + 1), s);
            t.set_u32(t.img.word(around + 0x1e), s);
            t.set_u32(t.img.word(around + 0x36), table);
            t.write(table, &[0x22; 0x400]);
            t.set_u32(t.img.word(setup + 0x16), s);
            let x = SCRATCH + 0x900;
            junk(&mut t, x, 0x40, st);
            t.set_u32(t.img.word(setup + 0x266), x);
            let take = t.img.rel32_target(setup + 0x309);
            pool74(&mut t, take, SCRATCH + 0x4000, 0, 0);
            ownb::dirty_stack(&mut t, rva + st, &[]);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}
