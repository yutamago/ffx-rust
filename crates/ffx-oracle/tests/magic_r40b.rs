//! Round 40's `magic_0174` ports (`ffx_magic::r40`) against the original,
//! with `magic_r17.rs`'s set-up for the `magic_0172` shapes they share.
//! Needs `FFX_GAME_DIR`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::too_many_lines
)]

mod magic;
mod magic_owna;
mod magic_r11;

use ffx_magic::r40;
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

/// The vector and matrix slots (`magic_r17.rs`'s): 234 the difference,
/// 248 the sum, 325 a copy, 235 a matrix, 328/329 one word plus the
/// angle, 229 the matrix word halved plus the vector's, 236 a bone matrix
/// (16 words), 237 a point.
fn vector_slots(t: &mut Twin) {
    rec(t, 234, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x234
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
    for s in [235u32, 236] {
        let at = if s == 235 { 0 } else { 2 };
        rec(t, s, at + 1, move |m, a| {
            let k = m.r32(row(s)) as f32;
            for i in 0..16u32 {
                let v = if i % 5 == 0 {
                    1.0 + k * 0.5
                } else {
                    0.125 * (i as f32) - k * 0.25
                };
                wf(m, a[at] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 237, 4, |m, a| {
        let k = m.r32(row(237)) as f32;
        for i in 0..3 {
            wf(m, a[3] + 4 * i, 0.75 * i as f32 - k);
        }
        0x237
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

/// The float slots: 12 (the ground `g`) and 277 (a scale), ST0.
fn float_slots(t: &mut Twin, g: f64) {
    rec(t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, move |_, _| g);
    rec(t, 277, 1, |_, a| 0x0277_0000 ^ a[0]);
    t.stub_st0(277, move |_, _| g * 0.5 + 2.0);
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

/// The slots `magic_0174`'s functions call (as `magic_r17.rs`'s
/// `slots_0172`) and the flashes (slot 256).
fn slots_0174(t: &mut Twin, k: u32) {
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
    rec(t, 256, 5, |_, a| 0x0256_0000 ^ a[0] ^ a[4]);
}

/// `magic_0174` 0x10001210 (`r40::load_0174`, the start-up
/// `r40::start_up_0174`): busy, not loaded, then the start-up with none to
/// three targets, slot 413 or 215, the free ring of one or three, the task
/// list empty or not.
#[test]
fn load_0174() {
    let Some(mut t) = twin(174) else { return };
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
        slots_0174(&mut t, k);
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
        magic_r11::pools(&mut t, start, 1052);
        for so in [0x66u32, 0x152] {
            let so = t.img.rel32_target(start + so);
            magic_r11::object_start_setup(&mut t, so, (0x84, 0x9d));
        }
        for o in [0x72u32, 0x15e, 0x1c2] {
            t.set_u32(op(&t, o), 0x0a00_0000 + o);
        }
        t.set_u32(op(&t, 0x1f7), n);
        t.set_u32(op(&t, 0x2d8), n);
        let sel = SCRATCH + 0x5000;
        t.write(sel, &[2, 1, 0, 3]);
        t.set_u32(op(&t, 0x214), sel);
        for i in 0..4 {
            t.set_u32(op(&t, 0x21f) + 4 * i, 0x0c0c_0100 + i);
        }
        let recs = op(&t, 0x202) - 4;
        junk(&mut t, recs, 3 * 0x160, k);
        // one global for both tests (+0x2e4, +0x324)
        t.set_u32(op(&t, 0x2e4), flag);
        t.set_u32(op(&t, 0x2ea), 0x027f_0000 + k);
        junk_ops(&mut t, start, &[0x143, 0x353, 0x35c, 0x2ef, 0x32f, 0x349]);
        let (count, head) = (op(&t, 0x363), op(&t, 0x372));
        magic_r11::ring(&mut t, count, head, free, SCRATCH + 0x1c000);
        task_list(&mut t, start, (0x3d9, 0x3e8), listed, k);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(row(234)) >= 4, "the target loop ran");
}

/// `magic_0174` 0x10001760 (`r40::summon_frame_0174`, with
/// `r40::hits_0174`): no objects (done), the motion done or waiting, the
/// hits off, and on for targets hit (counting down, at 0, below, at -8),
/// near and far.
#[test]
fn summon_frame_0174() {
    let Some(mut t) = twin(174) else { return };
    let f = t.at(0x1760);
    let hits = t.img.rel32_target(f + 0xc);
    let ro = t.img.rel32_target(f + 0x5);
    let ms = t.img.rel32_target(f + 0x11);
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    for (k, (on, done, n)) in [
        (0u32, 1u32, 2u32),
        (1, 1, 3),
        (1, 0, 2),
        (2, 1, 0),
        (1, 1, 1),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        slots_0174(&mut t, k);
        let so = t.img.rel32_target(hits + 0x1c3);
        magic_r11::object_start_setup(&mut t, so, (0x84, 0x9d));
        t.set_u32(op(&t, ro, 0x1a), 0);
        t.set_u32(op(&t, ro, 0x25), SCRATCH + 0x60);
        let list = op(&t, ro, 0x7c);
        t.write(list, &[0; 0x10]);
        t.set_u32(op(&t, ro, 0x14), 0);
        t.set_u32(op(&t, ms, 0x02), done);
        t.set_u16(op(&t, ms, 0x11), 5);
        t.set_u32(op(&t, hits, 0x16), on);
        t.set_u32(op(&t, hits, 0x3f), n);
        let sel = SCRATCH + 0x5000;
        t.write(sel, &[2, 1, 3, 1]);
        t.set_u32(op(&t, hits, 0x7d), sel);
        for i in 0..4 {
            t.set_u32(op(&t, hits, 0x88) + 4 * i, 0x0c0c_0100 + i);
        }
        let recs = op(&t, hits, 0x4a);
        junk(&mut t, recs - 0xd4, 3 * 0x160 + 0xd4, k);
        for i in 0..3u32 {
            let r = recs + 0x160 * i;
            // every record but the second one hit, its count by case
            let hit = i != 1;
            t.set_u32(r + 0x84, u32::from(hit));
            t.set_u32(
                r + 0x80,
                [3u32, 1, 0, 0xffff_fffd, 0xffff_fff8][((i + 2 * k) % 5) as usize],
            );
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
    assert!(t.u32(row(268)) >= 1, "no hit ran");
}

/// `magic_0174` 0x10001c50 (`r40::object_turn_0174`): past the frame
/// limit, and the turn with the yaw wrapped down, up and kept.
#[test]
fn object_turn_0174() {
    let Some(mut t) = twin(174) else { return };
    let f = t.at(0x1c50);
    let obj = SCRATCH + 0x3000;
    for (k, (frames, ground)) in [
        (0x51u32, 0.5f64),
        (0x50, 0.5),
        (3, -2.0),
        (0, 4.0),
        (7, 1.0e-3),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        slots_0174(&mut t, k);
        float_slots(&mut t, ground);
        junk(&mut t, obj, 0x140, k);
        t.set_u32(obj + 0x118, frames);
        for o in [0x4bu32, 0x5e, 0x225, 0x239] {
            t.set_u32(t.img.word(f + o), 0x0c40_0000 + k);
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

/// `r40::start_up_0174` alone (internal) on a ring of one.
#[test]
fn start_up_0174_alone() {
    let Some(mut t) = twin(174) else { return };
    let f = t.at(0x1340);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    slots_0174(&mut t, 0);
    magic_r11::pools(&mut t, f, 1052);
    for so in [0x66u32, 0x152] {
        let so = t.img.rel32_target(f + so);
        magic_r11::object_start_setup(&mut t, so, (0x84, 0x9d));
    }
    t.set_u32(op(&t, 0x1f7), 1);
    t.set_u32(op(&t, 0x2d8), 1);
    let sel = SCRATCH + 0x5000;
    t.write(sel, &[2, 1, 0, 3]);
    t.set_u32(op(&t, 0x214), sel);
    let (count, head) = (op(&t, 0x363), op(&t, 0x372));
    magic_r11::ring(&mut t, count, head, 1, SCRATCH + 0x1c000);
    task_list(&mut t, f, (0x3d9, 0x3e8), false, 0);
    scrub(&mut t);
    t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
        r40::start_up_0174(g, img, fr)
    });
}
