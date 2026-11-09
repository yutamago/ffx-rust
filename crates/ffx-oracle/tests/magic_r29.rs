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
        trail_block(&mut t, s + 0x2030, 2 + k % 3, k);
        // the fade `S + 0x2254` is the trail's sprite count (`+0x224`)
        t.set_u32(s + 0x2260, count.cast_unsigned());
        t.set_u32(s + 0x2254, fade.cast_unsigned());
        t.set_u32(s + 0x2264, parts.cast_unsigned());
        t.set_u32(op(&t, 0x16), s);
        t.set_u32(op(&t, 0x5e), actor);
        // (the byte after it junk: the actor is a byte)
        t.write(actor, &[0x41 + k as u8, 0x77]);
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

/// `magic_0602` 0x100078b0 (`r29::summon_frames_0602`): the sounds'
/// handles, the yaw and places varied, the selected actors varied.
#[test]
fn summon_frames_0602() {
    let Some(mut t) = twin(602) else { return };
    all_stubs(&mut t);
    matrix_stubs(&mut t);
    rec(&mut t, 0, 1, |_, a| 0x00a0_0000 ^ a[0]);
    for (s_, n_) in [(26u32, 2usize), (10, 2), (31, 2), (210, 2), (211, 2)] {
        rec(&mut t, s_, n_, move |_, a| (s_ << 16) ^ a[0] ^ a[1]);
    }
    let f = t.at(0x78b0);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let s = SCRATCH + 0x1_0000;
    let sel = SCRATCH + 0x600;
    for (k, yaw) in (0u32..).zip([0.5f32, -2.0, 3.5]) {
        t.set_u32(op(&t, 0x1f), s);
        junk(&mut t, s + 0x1800, 0x300, k);
        for i in 0..0x70u32 {
            t.set_u32(
                s + 0x1880 + 4 * i,
                (0.25 * i as f32 - 3.0 + k as f32).to_bits(),
            );
        }
        for i in 0..4u32 {
            t.set_u32(s + 0x1a60 + 4 * i, (1.5 * i as f32 + k as f32).to_bits());
        }
        t.set_u32(s + 0x2258, yaw.to_bits());
        t.set_u32(op(&t, 0x369), sel);
        t.write(sel, &[7, 9, 9, (k % 3) as u8, ((k + 1) % 3) as u8, 2, 9, 9]);
        for i in 0..4u32 {
            t.set_u32(op(&t, 0x37d) + 4 * i, 0x00ac_0000 + 0x10 * i + k);
        }
        junk_ops(&mut t, f, &[0x45, 0x4c, 0x81, 0x88, 0xbd, 0xc4, 0xd2]);
        ownb::dirty_stack(&mut t, 0x78b0 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0605` 0x100092e0 and `magic_0307` 0x10005da0
/// (`r29::spin_setup_0605`, family P011; `magic_r25.rs`'s `spin_setup_0612`
/// cases on each copy's layout, read from the code): the angle wrapped
/// up, down and not, the count running out and not, the pool full at
/// once, a record's point above the ground and below it.
#[test]
fn spin_setup_0605() {
    for (n, addr) in [(605u16, 0x92e0u32), (307, 0x5da0)] {
        let Some(mut t) = twin(n) else { return };
        all_stubs(&mut t);
        let f = t.at(addr);
        let op = |t: &Twin, o: u32| t.img.word(f + o);
        rec(&mut t, 237, 4, |m, a| {
            for i in 0..4 {
                let v = 0.5 * a[1] as f32 - 0.75 * i as f32 + (a[0] & 0xf) as f32;
                wf(m, a[3] + 4 * i, v);
            }
            0x237
        });
        rec(&mut t, 235, 1, |m, a| {
            for i in 0..16 {
                wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
            }
            0x235
        });
        for s in [330u32, 328, 329, 231] {
            rec(&mut t, s, 3, move |m, a| {
                for i in 0..16 {
                    let v = m.r32(a[1] + 4 * i) ^ (a[2].rotate_left(i) & 0x0000_ff00);
                    m.w32(a[0] + 4 * i, v);
                    fold(m, s, i % 4, v);
                }
                s
            });
        }
        rec(&mut t, 229, 3, |m, a| {
            for i in 0..4 {
                let v = rf(m, a[2] + 4 * i) * 0.5 + rf(m, a[1] + 4 * i) * 0.25 + 0.5 + i as f32;
                wf(m, a[0] + 4 * i, v);
            }
            0x229
        });
        rec(&mut t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
        let ground = SCRATCH + 0x7f0;
        t.stub_st0(12, move |m, _| f64::from(rf(m, ground)));
        rec(&mut t, 314, 1, |m, a| {
            let k = m.r32(magic_owna::row(314));
            m.w32(magic_owna::row(314), k + 1);
            fold(m, 314, 0, a[0]);
            0x0314_0000 ^ k
        });
        t.stub_st0(314, |m, a| {
            let k = m.r32(magic_owna::row(314));
            f64::from(f32::from_bits(a[0])) * (f64::from(k % 5) * 0.2 + 0.1)
        });
        let take = t.img.rel32_target(f + 0x309);
        let recs = SCRATCH + 0x4000;
        let s = SCRATCH + 0x1_0000;
        let x = SCRATCH + 0x900;
        let (pt, turn, count, place) = (op(&t, 0xc4), op(&t, 0x161), op(&t, 0x544), op(&t, 0x552));
        // (angle, count, free records (busy mask), the ground)
        let cases: [(f32, u32, u32, f32); 6] = [
            (0.5, 3, 0, -10.0),
            (3.2, 1, 0b10, 50.0),
            (-3.3, 2, 0b1, -10.0),
            (0.0, 1, 0xff, 0.0),
            (f32::NAN, 5, 0, 5.0),
            (6.0, 1, 0b100, -100.0),
        ];
        for (k, (angle, cnt, busy, gr)) in (0u32..).zip(cases) {
            junk(&mut t, s + 0xb00, 0x2000, k);
            for i in 0..12 {
                t.set_u32(
                    s + place + 4 * i,
                    (2.5 * i as f32 - 7.0 + k as f32).to_bits(),
                );
                t.set_u32(s + pt + 4 * i, (0.125 * i as f32 + 0.5).to_bits());
            }
            t.set_u32(s + turn, angle.to_bits());
            t.set_u32(s + count, cnt);
            t.set_u32(op(&t, 0x16), s);
            t.set_u32(op(&t, 0x30), 0x00cb_0000 + k);
            junk(&mut t, x, 0x40, k + 1);
            t.set_u32(op(&t, 0x266), x);
            t.set_u32(ground, gr.to_bits());
            pool74(&mut t, take, recs, 3, busy);
            ownb::dirty_stack(&mut t, addr + k, &[]);
            t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
                ffx_magic::Ret::eax(ffx_magic::r29::spin_setup_0605(g, img, fr.at, fr.esp))
            });
        }
        assert!(t.u32(ownb::LOG) > 0x100, "the set-up's calls");
    }
}

/// The slots of the models' draw (`magic_s5_misc.rs`'s
/// `models_drawn_around`): the matrices filled, the handle's `edx`, the
/// lock word logged.
fn around_stubs(t: &mut Twin, lock: u32) {
    magic_owna::filler(t, 235, 1, 16);
    for s in [330u32, 328, 329, 231] {
        rec(t, s, 3, move |m, a| {
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) * 0.75 + (a[2] & 0xff) as f32 * 0.01 + s as f32 * 0.001;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[2] + 4 * i) * 0.5 + rf(m, a[1] + 4 * i) * 0.25 + 0.5 + i as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    for s in [723u32, 724] {
        rec(t, s, 1, move |m, a| {
            for i in 0..16 {
                wf(m, a[0] + 4 * i, 0.5 * i as f32 + 0.5);
            }
            s
        });
    }
    rec(t, 562, 0, |_, _| 0x562);
    rec(t, 772, 2, |_, a| 0x0772_0000 ^ a[0]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    rec(t, 773, 2, move |m, a| (a[0] & 1) ^ (m.r32(lock) & 0x10));
    for (s, n) in [(819u32, 4usize), (870, 6), (871, 5)] {
        rec(t, s, n, move |m, _| (s << 16) ^ m.r32(lock));
    }
}

/// `magic_0605` 0x10008030 (`r29::models_around_0605`): the records and
/// the models varied.
#[test]
fn models_around_0605() {
    let Some(mut t) = twin(605) else { return };
    all_stubs(&mut t);
    let lock = SCRATCH + 0x7f0;
    around_stubs(&mut t, lock);
    let f = t.at(0x8030);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let s = SCRATCH + 0x1_0000;
    let table = SCRATCH + 0x400;
    let draw = t.img.rel32_target(f + 0x22d);
    t.set_u32(t.img.word(draw + 0x97), lock);
    for k in 0..2u32 {
        t.set_u32(op(&t, 0x17), s);
        t.set_u32(op(&t, 0x36), table);
        junk(&mut t, s + 0x1e00, 0x900, k);
        for i in 0..0x120u32 {
            t.set_u32(
                s + 0x1f00 + 4 * i,
                ((i % 7) as f32 * 0.5 - 1.0 + k as f32).to_bits(),
            );
        }
        junk(&mut t, table + 0x180, 0x300, k + 1);
        t.set_u32(lock, 0x5a5a_0000 + k);
        ownb::dirty_stack(&mut t, 0x8030 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0605` 0x1000a0c0 (`r29::spin_states_0605`): every state before
/// the models' draw (the set-up with an empty pool, the step counting,
/// the pieces' move, others).
#[test]
fn spin_states_0605() {
    let Some(mut t) = twin(605) else { return };
    all_stubs(&mut t);
    let lock = SCRATCH + 0x7f0;
    around_stubs(&mut t, lock);
    let f = t.at(0xa0c0);
    let s = SCRATCH + 0x1_0000;
    let setup = t.img.rel32_target(f + 0x2b);
    let around = t.img.rel32_target(f + 0x30);
    let moves = t.img.rel32_target(f + 0x17);
    let table = SCRATCH + 0x400;
    t.set_u32(t.img.word(t.img.rel32_target(around + 0x22d) + 0x97), lock);
    rec(&mut t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, _| -10.0);
    for st in [3u32, 2, 1, 0, 0xffff_ffff, 7] {
        t.write(s + 0x1000, &vec![0; 0x1c00]);
        for i in 0..0x400u32 {
            t.set_u32(s + 0x1e00 + 4 * i, ((i % 7) as f32 * 0.5 - 1.0).to_bits());
        }
        t.set_u32(s + t.img.word(f + 7), st);
        t.set_u32(s + 0x26b4, 3);
        for at in [f + 1, around + 0x17, setup + 0x16, moves + 1] {
            t.set_u32(t.img.word(at), s);
        }
        t.set_u32(t.img.word(around + 0x36), table);
        t.write(table, &[0x22; 0x400]);
        let x = SCRATCH + 0x900;
        junk(&mut t, x, 0x40, st);
        t.set_u32(t.img.word(setup + 0x266), x);
        let take = t.img.rel32_target(setup + 0x309);
        pool74(&mut t, take, SCRATCH + 0x4000, 0, 0);
        ownb::dirty_stack(&mut t, 0xa0c0 + st, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// One case of [`two_part_visit_0609`]'s test.
#[derive(Clone, Copy)]
struct TwoPart {
    marked: bool,
    children: usize,
    flags: u8,
    x28: f32,
    speed: f32,
    step: f32,
    fade: f32,
}

/// Lays out case `k` (as `magic_o6_fam.rs`'s `two_part_setup`: the
/// cursor, the node, two children, the parts whose class's visit is the
/// trap `class`; the light's object at the second part's `+0x9c`).
fn two_part_setup(t: &mut Twin, k: u32, p: TwoPart, class: u32) {
    let (c, node) = (SCRATCH + 0x3000, SCRATCH + 0x3100);
    let kids = [SCRATCH + 0x3200, SCRATCH + 0x3240];
    let (pa, pb) = (SCRATCH + 0x3300, SCRATCH + 0x3400);
    t.write(node, &[0x44; 0x80]);
    for i in 0..19u32 {
        let x = 0.25 * (k + i) as f32 - 1.5;
        t.set_u32(node + 0x1c + 4 * i, x.to_bits());
    }
    t.set_u32(node + 0x4c, 0.375f32.to_bits());
    t.set_u32(node + 0x54, 0.125f32.to_bits());
    t.set_u32(node + 0x50, p.step.to_bits());
    t.set_u32(node + 0x58, p.fade.to_bits());
    t.set_u32(node + 0x28, p.x28.to_bits());
    t.set_u32(node + 0x5c, p.speed.to_bits());
    t.set_u8(node + 0x68, p.flags);
    t.set_u8(node + 0x1a, 0xf0);
    t.set_u32(c, node);
    t.set_u8(c + 4, 0x55);
    t.set_u8(c + 5, if p.marked { 0xf0 } else { 3 + k as u8 });
    let mut prev = node;
    for &kid in kids.iter().take(p.children).rev() {
        t.write(kid, &[0x22; 0x10]);
        t.set_u32(kid, class);
        t.set_u32(kid + 8, prev);
        prev = kid;
    }
    t.set_u32(node + 0x10, prev);
    for part in [pa, pb] {
        t.write(part, &[0x11; 0xc0]);
        t.set_u32(part, class);
        for i in 0..3u32 {
            t.set_u32(part + 0x1c + 4 * i, (0.5 * (i + k) as f32).to_bits());
        }
    }
    t.set_u32(node + 0x6c, pa);
    t.set_u32(node + 0x70, pb);
}

/// `magic_0609` 0x10001000 (`r29::two_part_visit_0609`) and its thunk
/// 0x10001370: the node already marked (`eax` junk there: the /GS
/// cookie's bits); no children, one or two; the flags 0..3 with `+0x28`
/// above and below the light's bound (NaN once); the step and fade before,
/// across and at their end; the light counter `*G_C` varied.
#[test]
fn two_part_visit_0609() {
    let Some(mut t) = twin(609) else { return };
    all_stubs(&mut t);
    for (s, n) in [
        (231u32, 3usize),
        (245, 1),
        (563, 4),
        (821, 1),
        (822, 4),
        (823, 2),
        (824, 4),
    ] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    let visit = 1000u32;
    t.stub(visit, move |m, a| {
        let k = m.r32(SCRATCH + 0x3800);
        m.w32(SCRATCH + 0x3800, k + 1);
        let cur = m.r32(a[0]);
        m.w32(SCRATCH + 0x3810 + 8 * (k % 8), cur);
        m.w32(SCRATCH + 0x3814 + 8 * (k % 8), m.r32(a[0] + 4));
        0x1000 + k
    });
    let class = SCRATCH + 0x3700;
    t.set_u32(class + 0x1c, magic::TRAPS + 16 * visit);
    let f = t.at(0x1000);
    let c = SCRATCH + 0x3000;
    let bound = f32::from_bits(t.u32(t.img.word(f + 0xab)));
    let case = |marked, children, flags, x28, speed, step, fade| TwoPart {
        marked,
        children,
        flags,
        x28,
        speed,
        step,
        fade,
    };
    let cases = [
        case(true, 0, 0, 1.0, 1.0, 0.0, 0.0),
        case(false, 0, 0, 1.0, 1.0, 0.25, 0.5),
        case(false, 2, 1, bound + 1.0, 2.0, 0.25, 0.5),
        case(false, 2, 3, bound + 1.0, 2.0, 0.25, 0.5),
        case(false, 1, 1, bound + 1.0, -2.0, 1.0, 1.0),
        case(false, 0, 1, bound - 1.0, 2.0, 0.75, 0.5),
        case(false, 0, 1, bound, 2.0, 0.75, 0.5),
        case(false, 0, 2, 1.0, 2.0, 0.75, 0.5),
        case(false, 0, 0, 1.0, 2.0, 0.75, 0.95),
        case(false, 0, 0, 1.0, 2.0, 1.5, 0.25),
        case(false, 0, 1, f32::NAN, 2.0, 0.25, 0.5),
        case(false, 0, 1, bound + 2.0, 0.0, 0.25, 0.5),
    ];
    let counter = t.img.word(f + 0xe7);
    for (k, p) in (0u32..).zip(cases) {
        let eax = if p.marked { Eax::Junk } else { Eax::Checked };
        for thunk in [false, true] {
            two_part_setup(&mut t, k, p, class);
            t.set_u32(counter, SCRATCH + 0x5000 + 0x40 * k);
            junk_ops(&mut t, f, &[0xc2]);
            ownb::dirty_stack(&mut t, 0x1000 + k, &[]);
            if thunk {
                t.check_entry(t.at(0x1370), Call::default(), &[c], eax);
            } else {
                t.check(f, Call::this(c), &[], eax, |g, img, fr| {
                    ffx_magic::r29::two_part_visit_0609(g, img, fr)
                });
            }
        }
    }
}

/// `magic_0563` 0x10003310 (`r29::sparks_eight_0563`): the pool with room
/// for all eight, for three and for none.
#[test]
fn sparks_eight_0563() {
    let Some(mut t) = twin(563) else { return };
    all_stubs(&mut t);
    magic_owna::filler(&mut t, 235, 1, 16);
    rec(&mut t, 329, 3, |m, a| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i) ^ (a[2].rotate_left(i) & 0x00ff_0000);
            m.w32(a[0] + 4 * i, v);
        }
        0x329
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[2] + 4 * i) * 0.5 + rf(m, a[1] + 4 * i) * 0.25 + 0.5 + i as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    for s in [314u32, 404] {
        rec(&mut t, s, 1, move |m, a| {
            let k = m.r32(magic_owna::row(s));
            m.w32(magic_owna::row(s), k + 1);
            (s << 16) ^ k ^ a[0]
        });
        t.stub_st0(s, move |m, a| {
            let k = m.r32(magic_owna::row(s));
            f64::from(f32::from_bits(a[0])) * (f64::from(k % 7) * 0.15 + 0.1) - 0.03
        });
    }
    let f = t.at(0x3310);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let take = t.img.rel32_target(f + 0x35);
    let s = SCRATCH + 0x1_0000;
    let fx = SCRATCH + 0x900;
    for (k, n) in (0u32..).zip([12u32, 3, 0]) {
        junk(&mut t, s + 0x2b00, 0x100, k);
        for i in 0..4u32 {
            t.set_u32(
                s + 0x2b40 + 4 * i,
                (1.5 * i as f32 - 2.0 + k as f32).to_bits(),
            );
        }
        t.set_u32(op(&t, 0x14), s);
        t.set_u32(op(&t, 0x45), fx);
        t.set_u32(fx + 0x14, 0x7e00_0000 + k);
        pool74(&mut t, take, SCRATCH + 0x4000, n, 0);
        ownb::dirty_stack(&mut t, 0x3310 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0563` 0x10003000 (`r29::light_start_0563`): the task list empty
/// and not, the object pool with nodes and empty, no target bytes, two
/// and a negative count.
#[test]
fn light_start_0563() {
    let Some(mut t) = twin(563) else { return };
    all_stubs(&mut t);
    rec(&mut t, 388, 3, |_, a| {
        0x0388_0000 ^ a[0] ^ (a[1] << 8) ^ (a[2] << 12)
    });
    let f = t.at(0x3000);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let so = t.img.rel32_target(f + 0x1c4);
    let take = t.img.rel32_target(f + 0x6e);
    let s = SCRATCH + 0x1_0000;
    let bytes = SCRATCH + 0x7e00;
    let last = SCRATCH + 0x3400;
    // (listed, object pool count, target count)
    for (k, (listed, count, nb)) in (0u32..).zip([
        (false, 3u16, 0i32),
        (true, 3, 2),
        (true, 0, -1),
        (false, 1, 3),
    ]) {
        start_env(&mut t, so, count, if k == 1 { SCRATCH + 0x680 } else { 0 });
        t.set_u32(op(&t, 0x9), s);
        junk(&mut t, s + 0x400, 0x3800, k);
        for i in 0..8u32 {
            t.set_u32(
                s + 0x2b30 + 4 * i,
                (0.75 * i as f32 - 1.0 + k as f32).to_bits(),
            );
        }
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
        t.write(last, &[0x34; 0x20]);
        t.set_u32(op(&t, 0x74), if listed { last } else { 0 });
        t.set_u32(op(&t, 0x89), last);
        t.set_u32(op(&t, 0x2c3), nb.cast_unsigned());
        t.set_u32(op(&t, 0x2dd), bytes);
        t.set_u32(op(&t, 0x2d6), 0x0bad_0000 + k);
        for i in 0..8u8 {
            t.set_u8(bytes + u32::from(i), 0x31 + 7 * i + k as u8);
        }
        ownb::dirty_stack(&mut t, 0x3000 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}
