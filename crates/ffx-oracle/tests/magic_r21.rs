//! Round 21's own ports (`ffx_magic::r21`) against each original (the
//! copies bound to earlier ports are tested with those ports). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

mod magic;
mod magic_owna;
mod magic_r11;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

/// Junk words at the globals the operands `ops` of `f` name.
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, (0x5a5a_5a5a ^ o) | 1);
    }
}

/// Slots recorded with an answer from their first word.
fn recs(t: &mut Twin, slots: &[(u32, usize)]) {
    for &(s, n) in slots {
        rec(t, s, n, move |_, a| {
            (s << 20) ^ a.first().copied().unwrap_or(0)
        });
    }
}

/// `r21::start_0431` (`magic_0431` 0x10004390) with the task list empty
/// and not, and its load step `r21::load_0431` (0x10001250): busy, the
/// sounds not ready, asking, not loaded, loaded (the start-up runs on
/// both sides).
#[test]
fn start_0431() {
    let Some(mut t) = twin(431) else { return };
    let f = t.at(0x4390);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    recs(&mut t, &[(719, 3), (711, 3), (710, 4), (271, 3)]);
    // the camera start's slots (`magic_0431.rs`' camera test)
    let cam = t.img.rel32_target(f + 0x79);
    rec(&mut t, 262, 4, |m, a| {
        for (i, at) in (0u32..).zip(&a[1..4]) {
            m.w32(*at, 0x0262_0000 + i);
        }
        0x262
    });
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, |_, _| 0.5);
    let fx = SCRATCH + 0x8000;
    t.set_u32(t.img.word(cam + 0x08), fx);
    let starts = |t: &mut Twin, listed: bool| {
        junk_ops(
            t,
            f,
            &[
                0x16, 0x20, 0x2a, 0x34, 0x3e, 0x48, 0x52, 0x5c, 0x6a, 0x84, 0x8e,
            ],
        );
        t.set_u32(w(t, 0x99), 0x5a5a_5a5a);
        t.set_u32(w(t, 0xa1), 0x5a5a_5a5a);
        magic_r11::pools(t, f, 325);
        t.write(fx + 0x810, &[0x33; 0x20]);
        t.write(fx + 0xc60, &[0x55; 0xc0]);
        t.write(w(t, 0xea), &[0x5c; 0x40]);
        t.write(SCRATCH + 0x600, &[0x44; 0x10]);
        t.set_u32(w(t, 0xbf), if listed { SCRATCH + 0x600 } else { 0 });
        t.set_u32(w(t, 0xd3), SCRATCH + 0x600);
    };
    for listed in [false, true] {
        starts(&mut t, listed);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r21::start_0431,
        );
    }
    // the load step
    let at = t.at(0x1250);
    let (busy, loaded, sound) = (SCRATCH + 0x5e0, SCRATCH + 0x5e4, SCRATCH + 0x5e8);
    rec(&mut t, 225, 1, move |m, _| m.r32(busy));
    rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
    rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
    rec(&mut t, 5, 1, move |m, _| m.r32(sound));
    rec(&mut t, 28, 2, |_, _| 0);
    t.set_u32(t.img.word(at + 0x32), 0x0000_5e5e);
    for (b, s, asked, l) in [
        (1u32, 0u32, 0u32, 1u32),
        (0, 3, 0, 1),
        (0, 0, 0, 0),
        (0, 0, 1, 2),
    ] {
        starts(&mut t, false);
        t.set_u32(busy, b);
        t.set_u32(sound, s);
        t.set_u32(loaded, l);
        t.set_u32(t.img.word(at + 0x22), asked);
        junk_ops(&mut t, at, &[0x6d, 0x77]);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// Floats `v[i]` at `at` (four bytes apart).
fn floats_at(t: &mut Twin, at: u32, v: &[f32]) {
    for (i, x) in (0u32..).zip(v) {
        t.set_u32(at + 4 * i, x.to_bits());
    }
}

/// `r21::step_0382` (`magic_0382` 0x10001270): the counter at 0 (slot
/// 440, the counter read again), below 0x19, at 0x18 and 0x19, between,
/// at 0x31 (the throw: slot 453's value positive, negative, out of the
/// `int` range and NaN) and past it.
#[test]
fn step_0382() {
    let Some(mut t) = twin(382) else { return };
    let f = t.at(0x1270);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let a = SCRATCH + 0x800;
    t.set_u32(w(&t, 0x5), a);
    t.set_u32(a + 0x198, 0x0198_0001);
    t.set_u32(a + 0x1a0, 0x01a0_0001);
    let counter = w(&t, 0x5a);
    let x = SCRATCH + 0x700;
    recs(&mut t, &[(474, 4), (457, 3), (434, 6), (461, 8)]);
    // slot 440 moves the counter on (the original reads it again)
    rec(&mut t, 440, 4, move |m, a| {
        m.w32(counter, 7);
        a[0] ^ 0x440
    });
    rec(&mut t, 453, 4, |_, a| a[0] ^ 0x453);
    t.stub_st0(453, move |m, _| f64::from(f32::from_bits(m.r32(x))));
    let obj = SCRATCH + 0x1000;
    for (k, (n, xv)) in [
        (0u32, 1.5f32),
        (3, 1.5),
        (0x17, 1.5),
        (0x18, 1.5),
        (0x20, 1.5),
        (0x31, 2.75),
        (0x31, -9.5),
        (0x31, 3.0e9),
        (0x31, f32::NAN),
        (0x40, 0.5),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        junk_ops(
            &mut t,
            f,
            &[
                0x28, 0x32, 0x3c, 0x46, 0x101, 0x119, 0x131, 0x149, 0x184, 0x199, 0x1ef,
            ],
        );
        for (o, base) in [(0x1du32, 1.25f32), (0xd4, -3.5)] {
            let at = w(&t, o);
            floats_at(&mut t, at, &[base + k as f32, base * 2.0, 0.5 - base, 4.0]);
        }
        t.set_u32(w(&t, 0x87), 0x00a5_0000 + k);
        t.set_u32(counter, n);
        t.set_u32(x, xv.to_bits());
        t.write(obj, &[0x5d; 0x200]);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

/// `r21::throw_0382` (0x100014e0): `*G_I` even, odd, negative odd and 0;
/// slot 512 writes its output from its inputs.
#[test]
fn throw_0382() {
    let Some(mut t) = twin(382) else { return };
    let f = t.at(0x14e0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    recs(&mut t, &[(464, 1), (434, 6), (450, 8), (451, 10)]);
    rec(&mut t, 512, 4, |m, a| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, (v * 0.75 - 0.5).to_bits());
        }
        0x512
    });
    let r = SCRATCH + 0x900;
    t.set_u32(w(&t, 0x208), r);
    let obj = SCRATCH + 0x1000;
    for (k, n) in [4i32, 7, -5, 0].into_iter().enumerate() {
        let k = k as u32;
        t.set_u32(w(&t, 0x8f), n.cast_unsigned());
        let q = w(&t, 0x14);
        floats_at(&mut t, q, &[2.5 + k as f32, -1.5, 0.25, 3.0]);
        let p1 = w(&t, 0x135);
        floats_at(&mut t, p1, &[0.75, 1.75 + k as f32, -2.0, 9.0]);
        t.set_u32(w(&t, 0x1d3), 0x00a0_0000 + k);
        t.write(obj, &[0x5d; 0x200]);
        t.write(r, &[0x4c; 0x100]);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

/// `r21::throw_start_0382` (0x100018c0): `rand` drawn four times on
/// both sides from the same state, over two sources.
#[test]
fn throw_start_0382() {
    let Some(mut t) = twin(382) else { return };
    let f = t.at(0x18c0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    recs(&mut t, &[(458, 3), (450, 5), (449, 7)]);
    let src = SCRATCH + 0x900;
    t.set_u32(w(&t, 0x35), src);
    let obj = SCRATCH + 0x1000;
    for k in 0..3u32 {
        floats_at(
            &mut t,
            src + 0xd0,
            &[
                1.5,
                -2.25 + k as f32,
                0.125,
                7.0,
                3.0 * k as f32,
                -0.5,
                2.0,
                1.0,
            ],
        );
        t.set_u32(w(&t, 0x14e), 0x00a0_0000 + k);
        t.write(obj, &[0x5d + k as u8; 0x200]);
        // the flags word without bit 14 (the junk bytes have it set)
        t.set_u32(obj + 0xbc, 0x1234_8000 + k);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

/// `r21::set_up_0198` (`magic_0198` 0x100010e0): two arguments, the
/// vector slots writing their outputs from their inputs, the load and
/// the part start on both sides.
#[test]
fn set_up_0198() {
    let Some(mut t) = twin(198) else { return };
    let f = t.at(0x10e0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let byte = SCRATCH + 0x600;
    recs(
        &mut t,
        &[
            (168, 1),
            (215, 5),
            (217, 7),
            (232, 12),
            (710, 11),
            (718, 1),
            (719, 3),
            (209, 5),
            (208, 5),
            (221, 4),
            (714, 5),
        ],
    );
    rec(&mut t, 218, 2, move |_, _| byte);
    // slot 711 logs the lock word it runs under
    rec(&mut t, 711, 7, |m, _| {
        let l = m.r32(SCRATCH + 0x7f0);
        m.w32(SCRATCH + 0x624, l);
        0x711
    });
    rec(&mut t, 234, 3, |m, a| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) - f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x234
    });
    rec(&mut t, 233, 3, |m, a| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) * f32::from_bits(a[2]);
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x233
    });
    for (s, x) in [(251u32, 0.375f64), (216, -1.5)] {
        rec(&mut t, s, 1, move |_, a| a[0] ^ (s << 16));
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 7));
    }
    let table = SCRATCH + 0x700;
    for i in 0..4 {
        t.set_u32(table + 4 * i, 0x0941_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    let sp = t.img.rel32_target(f + 0x1fe);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x7f8);
    t.set_u32(t.img.word(sp + 0x29), SCRATCH + 0x7f8);
    t.set_u32(w(&t, 0x182), SCRATCH + 0x7f0);
    t.set_u32(w(&t, 0x1a8), SCRATCH + 0x7f0);
    t.set_u32(w(&t, 0x1d0), SCRATCH + 0x7e0);
    for (k, a) in [0x0abc_0001u32, 0x0abc_0002].into_iter().enumerate() {
        let k = k as u32;
        t.set_u8(byte, 3 + k as u8);
        // a non-zero byte above the read one
        t.set_u8(byte + 1, 0x7f);
        t.set_u32(SCRATCH + 0x7f0, 0x55 + k);
        t.set_u32(SCRATCH + 0x7e0, 0x66 + k);
        for o in [0x114u32, 0x81] {
            let at = w(&t, o);
            floats_at(&mut t, at, &[1.5 + k as f32, -0.25, 3.0, 1.0]);
        }
        let q = w(&t, 0x99);
        floats_at(&mut t, q, &[0.5, 2.5 + k as f32, -4.0, 1.0]);
        junk_ops(
            &mut t,
            f,
            &[0x15, 0x28, 0x4d, 0x65, 0x77, 0x14a, 0x214, 0x224],
        );
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}

/// `r21::step_0198` (0x10001360): the first-time waits (busy, asking, not
/// loaded), slot 253 answering 0 and not, the count at 0x10 (slot 228
/// moving it to 0x1e), 0x1e, inside the blend, at its end 0x44, at 0x55
/// and outside.
#[test]
fn step_0198() {
    let Some(mut t) = twin(198) else { return };
    let f = t.at(0x1360);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let (busy, loaded, idle) = (SCRATCH + 0x600, SCRATCH + 0x604, SCRATCH + 0x608);
    let count = w(&t, 0xb1);
    rec(&mut t, 225, 1, move |m, _| m.r32(busy));
    rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
    rec(&mut t, 253, 0, move |m, _| m.r32(idle));
    rec(&mut t, 228, 2, move |m, a| {
        if a[1] == 0xb0 {
            m.w32(count, 0x1e);
        }
        0x228
    });
    recs(
        &mut t,
        &[
            (226, 2),
            (254, 4),
            (255, 2),
            (249, 2),
            (237, 6),
            (256, 5),
            (220, 3),
            (257, 5),
            (239, 8),
        ],
    );
    let flag = SCRATCH + 0x700;
    t.set_u32(w(&t, 0xa5), flag);
    let tab = SCRATCH + 0x1_0000;
    for i in 0..0x1000u32 {
        t.set_u32(tab + 4 * i, (0.001 * i as f32 - 1.5).to_bits());
    }
    t.set_u32(w(&t, 0x143), tab);
    // (S0, S1, S2, busy, loaded, slot 253, count)
    for (k, (s0, s1, s2, b, l, i, n)) in [
        (0u32, 0u32, 0u32, 1u32, 1u32, 0u32, 0x1eu32),
        (0, 0, 0, 0, 0, 0, 0x1e),
        (0, 1, 0, 0, 1, 0, 0x10),
        (1, 1, 1, 0, 1, 5, 0x1e),
        (1, 1, 1, 0, 1, 0, 0x30),
        (1, 1, 1, 0, 1, 0, 0x44),
        (1, 1, 1, 0, 1, 0, 0x55),
        (1, 1, 1, 0, 1, 0, 0x60),
        (1, 1, 1, 0, 1, 0, 0x2),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(w(&t, 0x12), s0);
        t.set_u32(w(&t, 0x3f), s1);
        t.set_u32(w(&t, 0x6a), s2);
        t.set_u32(busy, b);
        t.set_u32(loaded, l);
        t.set_u32(idle, i);
        t.set_u32(count, n);
        t.set_u32(flag, 0x5a5a_5a5a);
        for (o, base) in [(0x16cu32, 0.75f32), (0x1a5, -2.5)] {
            let at = w(&t, o);
            floats_at(&mut t, at, &[base + k as f32, base * 3.0, 1.25 - base, 0.5]);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `r21::end_0198` (0x10001670): not loaded; the count below 0x53 and
/// from it, slot 712 answering 0 and not, the stop flag set and not.
#[test]
fn end_0198() {
    let Some(mut t) = twin(198) else { return };
    let f = t.at(0x1670);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let answer = SCRATCH + 0x600;
    let lock = SCRATCH + 0x708;
    // slot 712 answers and logs the lock word it runs under
    rec(&mut t, 712, 12, move |m, _| {
        let l = m.r32(lock);
        m.w32(SCRATCH + 0x620, l);
        m.r32(answer)
    });
    recs(
        &mut t,
        &[
            // (the caller's `esi` lies under the first part table's word)
            (941, 3),
            (723, 4),
            (724, 5),
            (258, 5),
            (231, 8),
            (230, 11),
            (254, 4),
            (249, 6),
            (256, 5),
            (213, 7),
            (214, 8),
        ],
    );
    let (g7, g12, stop) = (SCRATCH + 0x700, SCRATCH + 0x704, SCRATCH + 0x70c);
    t.set_u32(w(&t, 0x6b), g7);
    t.set_u32(w(&t, 0xed), g7);
    t.set_u32(w(&t, 0xe0), g12);
    t.set_u32(w(&t, 0xfd), lock);
    t.set_u32(w(&t, 0x118), lock);
    t.set_u32(w(&t, 0x1ca), stop);
    // (loaded, count, slot 712, stop)
    for (k, (s2, n, a, s)) in [
        (0u32, 0x60u32, 1u32, 0u32),
        (1, 0x52, 1, 0),
        (1, 0x53, 0, 0),
        (1, 0x60, 1, 0),
        (1, 0x60, 1, 1),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(w(&t, 0x2), s2);
        t.set_u32(w(&t, 0xe), n);
        t.set_u32(answer, a);
        t.set_u32(stop, s);
        for at in [g7, g12] {
            t.set_u32(at, 0x5a5a_0000 + k);
        }
        t.set_u32(lock, 0x77 + k);
        junk_ops(&mut t, f, &[0x36, 0x4e, 0x66]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `n` bytes of junk at `at` (varied by `k`).
fn junk(t: &mut Twin, at: u32, n: u32, k: u32) {
    let b: Vec<u8> = (0..n)
        .map(|i| (i.wrapping_mul(0x3d) ^ (0x71 + k)).to_le_bytes()[0])
        .collect();
    t.write(at, &b);
}

/// `r21::fade_0276` (`magic_0276` 0x10003330): the wait running; each
/// state with the step above 1 and at 1 (moving on); another state; the
/// slot skipped with `*D` set. The colour bytes' upper bytes in the frame
/// are 0 on both sides (junk-read rule).
#[test]
fn fade_0276() {
    let Some(mut t) = twin(276) else { return };
    let f = t.at(0x3330);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let lock = SCRATCH + 0x700;
    // slot 860 logs the lock word it runs under
    rec(&mut t, 860, 7, move |m, _| {
        let l = m.r32(lock);
        m.w32(SCRATCH + 0x720, l);
        0x860
    });
    t.set_u32(w(&t, 0x162), lock);
    t.set_u32(w(&t, 0x18d), lock);
    // (wait, state, step, D)
    for (k, (wait, state, step, d)) in [
        (3u32, 0u32, 5u32, 0u32),
        (0, 0, 9, 0),
        (0, 0, 1, 0),
        (0, 1, 0x1c, 0),
        (0, 1, 1, 0),
        (0, 2, 0x11, 0),
        (0, 2, 1, 0),
        (0, 3, 4, 0),
        (0, 0xffff_fff0, 4, 0),
        (0, 1, 7, 1),
        (0, 2, 0x30, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(w(&t, 0x4), wait);
        t.set_u32(w(&t, 0x1a), state);
        t.set_u32(w(&t, 0x40), step);
        t.set_u32(w(&t, 0x30), d);
        t.set_u32(lock, 0x40 + k);
        // the frame's three words (only their low bytes are written)
        let entry = magic::STACK_TOP - 0x100;
        t.write(entry - 0x10, &[0; 0xc]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `r21::load_0276` (0x10002cf0) and `r21::start_up_0276` (0x10002e20):
/// busy, not loaded, then the start-up with no, one and three actors,
/// the sound flag both ways, the task list empty and not, the free ring
/// with one node more than taken and exactly two (the second take empties
/// it).
#[test]
fn load_0276() {
    let Some(mut t) = twin(276) else { return };
    let f = t.at(0x2cf0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let start = t.img.rel32_target(f + 0x5b);
    let op = |t: &Twin, o: u32| t.img.word(start + o);
    let (busy, loaded) = (SCRATCH + 0x100, SCRATCH + 0x108);
    let list = SCRATCH + 0x180;
    // (busy, loaded, actors, sound flag, free nodes, task list)
    let cases: [(u32, u32, u32, u32, u16, bool); 6] = [
        (1, 1, 1, 0, 3, false),
        (0, 0, 1, 0, 3, false),
        (0, 1, 0, 0, 3, false),
        (0, 1, 1, 1, 2, true),
        (0, 1, 3, 0, 5, false),
        (0, 1, 3, 1, 4, true),
    ];
    for (k, (b, l, n, on, free, listed)) in (0u32..).zip(cases) {
        magic_r11::object_slots(&mut t);
        magic_r11::pools(&mut t, start, 0x60);
        let so = t.img.rel32_target(start + 0x66);
        magic_r11::object_start_auto(&mut t, so);
        rec(&mut t, 333, 1, |_, a| a[0] ^ 0x333);
        t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.5 - 7.0);
        rec(&mut t, 262, 4, move |m, a| {
            let c = m.r32(magic_owna::row(262)) as f32;
            for (i, &p) in (1u32..).zip(&a[1..4]) {
                m.w32(p, (0.5 * i as f32 + c * 0.75 - k as f32).to_bits());
            }
            0x262
        });
        rec(&mut t, 268, 3, |m, a| {
            let c = m.r32(magic_owna::row(268)) as f32;
            for i in 0..4 {
                m.w32(a[2] + 4 * i, (1.25 - i as f32 + c * 0.5).to_bits());
            }
            0x268
        });
        recs(&mut t, &[(413, 1), (215, 1)]);
        t.set_u32(op(&t, 0x19e), list);
        for i in 0..8u8 {
            t.set_u8(list + u32::from(i), 1 + 2 * i);
        }
        for i in 0..0x20 {
            t.set_u32(op(&t, 0x1a9) + 4 * i, 0x00c0_0000 + i);
        }
        t.set_u32(op(&t, 0x86), 0x00ca_0086);
        t.set_u32(op(&t, 0x93), 0x00ca_0093);
        t.set_u32(op(&t, 0x15e), n);
        t.set_u32(op(&t, 0x29f), 0x0005_0000 + k);
        t.set_u32(op(&t, 0x299), on);
        let objs = op(&t, 0x169) - 0xd4;
        junk(&mut t, objs, 0x130 * 3, k);
        let first = op(&t, 0x5a);
        junk(&mut t, first, 0x130, k + 1);
        junk_ops(
            &mut t,
            start,
            &[
                0x2d, 0x37, 0x45, 0x99, 0xee, 0xf4, 0xfe, 0x108, 0x112, 0x11c, 0x126, 0x130, 0x13a,
                0x144, 0x14e, 0x154, 0x2a4, 0x2b0, 0x2ba, 0x2c4, 0x2ce, 0x2d8, 0x2e2, 0x30c, 0x312,
                0x31c, 0x326, 0x330,
            ],
        );
        let (count, head) = (op(&t, 0x340), op(&t, 0x35a));
        // junk before the ring, where a stray store would land
        t.write(SCRATCH + 0x13e0, &[0x5a; 0x20]);
        magic_r11::ring(&mut t, count, head, free, SCRATCH + 0x1400);
        let other = SCRATCH + 0x5f0;
        t.write(other, &[0x44; 0x20]);
        t.set_u32(op(&t, 0x3b5), if listed { other } else { 0 });
        t.set_u32(op(&t, 0x3c5), other);
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
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(magic_owna::row(268)) >= 7, "the actors' objects ran");
    assert!(t.u32(magic_owna::row(413)) >= 1, "the sound flag's way");
}

/// `r21::restart_0352` (`magic_0352` 0x10005410): the fifth object in
/// the list and not (the restart), the task list empty and not; the four
/// object starts and the fan's task on both sides.
#[test]
fn restart_0352() {
    let Some(mut t) = twin(352) else { return };
    let f = t.at(0x5410);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    recs(&mut t, &[(904, 2), (713, 1)]);
    rec(&mut t, 282, 0, |_, _| 0x282);
    t.stub_st0(282, |m, _| f64::from(m.r32(SCRATCH + 0x7a0)) * 0.25 + 0.5);
    magic_r11::object_slots(&mut t);
    for at in [0x97u32, 0xd7, 0x117, 0x157] {
        let so = t.img.rel32_target(f + at);
        magic_r11::object_start_auto(&mut t, so);
    }
    magic_r11::pools(&mut t, f, 716);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x19), s);
    let p = SCRATCH + 0x7c0;
    t.set_u32(w(&t, 0x37), p);
    // the list `list_remove` walks: one node holding `S + 0x5f0` or none
    let list = w(&t, 0x29);
    let node = SCRATCH + 0x7d0;
    for (k, (listed, tasks)) in [(true, false), (false, true), (true, true)]
        .into_iter()
        .enumerate()
    {
        let k = k as u32;
        junk(&mut t, s, 0xc00, k);
        t.set_u32(SCRATCH + 0x7a0, 3 + k);
        t.set_u32(p, 0x5a5a_0000 + k);
        t.write(node, &[0x33; 0x10]);
        t.set_u32(node, 0);
        t.set_u32(node + 4, 0);
        t.set_u32(node + 8, if listed { s + 0x5f0 } else { 0x0bad_0000 });
        t.set_u32(list, node);
        let other = SCRATCH + 0x5f0;
        t.write(other, &[0x44; 0x10]);
        t.set_u32(w(&t, 0x1ef), if tasks { other } else { 0 });
        t.set_u32(w(&t, 0x202), other);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(magic_owna::row(713)) >= 2, "the restart ran");
}

/// `r21::task_0435` (`magic_0435` 0x10001570): the wait running, ending
/// in and outside a battle (actor below 0x20 and not); the objects busy
/// or done; the first script's flag unset; the scripts' halfwords set and
/// clear.
#[test]
fn task_0435() {
    let Some(mut t) = twin(435) else { return };
    let f = t.at(0x1570);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    recs(&mut t, &[(296, 2), (257, 2), (213, 2)]);
    let ro = t.img.rel32_target(f + 0x66);
    let list = t.img.word(ro + 0x7c);
    let (node, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    let (r1, r2) = (w(&t, 0x52), w(&t, 0x5c));
    let task = SCRATCH + 0x600;
    let rec = SCRATCH + 0x680;
    // (wait, battle flag, actor, objects busy, R1 +0x24, halfwords)
    for (k, (wait, battle, n, busy, ready, h)) in [
        (2u32, 0u32, 3u32, true, 1u32, 0u16),
        (1, 0, 5, false, 0, 0),
        (1, 1, 5, false, 1, 0),
        (0, 0, 0x20, false, 1, 5),
        (1, 0, 0x20, false, 1, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(w(&t, 0x4), wait);
        t.set_u32(w(&t, 0x18), battle);
        t.set_u32(w(&t, 0x20), n);
        t.set_u32(w(&t, 0x42), 0x00a0_0042 + k);
        for r in [r1, r2] {
            junk(&mut t, r, 0x28, k);
            // the scripts done (`+0x14` 1: nothing to run)
            t.set_u32(r + 0x14, 1);
            t.set_u16(r + 0x1c, h);
        }
        t.set_u32(r1 + 0x24, ready);
        // one object on its delay (it runs: the objects' answer is not 1)
        t.set_u32(node, 0);
        t.set_u32(node + 4, 0);
        t.set_u32(node + 8, obj);
        t.set_u16(obj + 0x124, 0x8000);
        t.set_u32(obj + 0x114, 5);
        t.set_u32(list + 8, if busy { node } else { 0 });
        // run_objects starts the list from its head global: without it
        // the object never runs and the answer is always 1
        t.set_u32(t.img.word(ro + 0x14), if busy { node } else { 0 });
        junk(&mut t, task, 0x20, k);
        t.set_u32(task + 8, rec);
        junk(&mut t, rec, 0x20, k + 1);
        junk_ops(&mut t, f, &[0xa3, 0xca]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
        if busy {
            // the object's delay counted: run_objects answered 0 (`return r`)
            assert_eq!(t.u32(obj + 0x114), 4, "the object ran");
        }
    }
    assert!(t.u32(magic_owna::row(213)) >= 2, "the scripts' slots");
}

/// `r21::height_0262` (`magic_0262` 0x100024d0): stopped (bit 0 set: the
/// cookie word, junk) and running over three table angles; slot 217
/// writes a point from its first word, slot 234 the difference.
#[test]
fn height_0262() {
    let Some(mut t) = twin(262) else { return };
    let f = t.at(0x24d0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 217, 2, |m, a| {
        for i in 0..4 {
            let v = (a[0] & 0xff) as f32 * 0.25 + i as f32 - 1.5;
            m.w32(a[1] + 4 * i, v.to_bits());
        }
        0x217
    });
    rec(&mut t, 234, 3, |m, a| {
        for i in 0..3 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) - f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x234
    });
    let tab = SCRATCH + 0x1_0000;
    for i in 0..0x1000u32 {
        t.set_u32(tab + 4 * i, (0.5 + 0.0007 * i as f32).to_bits());
    }
    t.set_u32(w(&t, 0x2c), tab);
    let (rec_at, obj) = (SCRATCH + 0x600, SCRATCH + 0x800);
    for (k, (flags, e)) in [(1u8, 0x100i32), (0, 0x100), (0, 0x7ff0), (0, -0x3000)]
        .into_iter()
        .enumerate()
    {
        let k = k as u32;
        junk(&mut t, obj, 0x100, k);
        t.set_u8(obj + 0x90, flags | 0x10);
        t.set_u32(obj + 0x50, 0x0000_0013 + k);
        junk(&mut t, rec_at, 0x20, k);
        t.set_u32(rec_at, obj);
        t.set_u32(rec_at + 8, 0x0000_0042 + k);
        t.set_u32(rec_at + 0xc, e.cast_unsigned());
        t.check_entry(f, Call::this(rec_at), &[], Eax::Junk);
    }
}

/// `r21::spark_0246` (`magic_0246` 0x100019d0): the turned point above
/// the ground (a free flag first and sixth, none free), below it and NaN.
#[test]
fn spark_0246() {
    let Some(mut t) = twin(246) else { return };
    let f = t.at(0x19d0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    recs(&mut t, &[(554, 4), (552, 6)]);
    // slot 229 moves the point by the matrix's diagonal and a height
    let lift = SCRATCH + 0x7a0;
    rec(&mut t, 229, 9, move |m, a| {
        let h = f32::from_bits(m.r32(lift));
        for i in 0..3 {
            let v = f32::from_bits(m.r32(a[2] + 4 * i)) * f32::from_bits(m.r32(a[1] + 0x14 * i))
                + if i == 1 { h } else { 0.0 };
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x229
    });
    magic_r11::object_slots(&mut t);
    let so = t.img.rel32_target(f + 0x10d);
    magic_r11::object_start_auto(&mut t, so);
    magic_r11::pools(&mut t, f, 349);
    let flags = w(&t, 0xd6);
    let obj = SCRATCH + 0x800;
    for (k, (h, free)) in [
        (5.0f32, Some(0u32)),
        (5.0, Some(5)),
        (5.0, None),
        (-50.0, Some(0)),
        (f32::NAN, Some(0)),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(lift, h.to_bits());
        for i in 0..0xc {
            t.set_u8(flags + i, u8::from(free != Some(i)));
        }
        junk(&mut t, obj, 0x100, k);
        for (i, o) in [0xe0u32, 0xe4, 0xe8].into_iter().enumerate() {
            t.set_u32(obj + o, (1.5 * i as f32 - 0.25 + k as f32).to_bits());
        }
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

/// `r21::part_object_0375` (`magic_0375` 0x10001c80): the objects of
/// index 0..=4, one past them and one below the base.
#[test]
fn part_object_0375() {
    let Some(mut t) = twin(375) else { return };
    let f = t.at(0x1c80);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let table = SCRATCH + 0x700;
    for i in 0..4 {
        t.set_u32(table + 4 * i, 0x0941_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5)]);
    let sp = t.img.rel32_target(f + 0x36);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x7f8);
    t.set_u32(t.img.word(sp + 0x29), SCRATCH + 0x7f8);
    let d = SCRATCH + 0x780;
    t.set_u32(w(&t, 0x10), d);
    t.set_u32(d + 0x10, 0x00d0_0010);
    let p = SCRATCH + 0x7c0;
    t.set_u32(w(&t, 0x4), p);
    let base = w(&t, 0x3d);
    for (k, i) in [0u32, 1, 2, 3, 4, 5, 0xffff_ffff].into_iter().enumerate() {
        let k = k as u32;
        let obj = base.wrapping_add(i.wrapping_mul(0x80));
        junk(&mut t, obj, 0x80, k);
        t.set_u32(p, 0x5a5a_0000 + k);
        t.set_u32(SCRATCH + 0x7f8, 0x77 + k);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

/// `r21::sparks_0419` (`magic_0419` 0x10001b60): the count at 0 (the task
/// given back), then two sparks from a pool with records, with one and
/// with none.
#[test]
fn sparks_0419() {
    let Some(mut t) = twin(419) else { return };
    let f = t.at(0x1b60);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    recs(&mut t, &[(719, 3), (237, 4)]);
    for (s, x) in [(307u32, 0.375f64), (314, 1.25), (404, -0.5)] {
        rec(&mut t, s, 1, move |m, a| {
            let n = m.r32(magic_owna::row(s));
            a.first().copied().unwrap_or(0) ^ (s << 16) ^ n
        });
        t.stub_st0(s, move |m, a| {
            x + f64::from(m.r32(magic_owna::row(s)) % 5) * 0.125
                + f64::from(a.first().copied().unwrap_or(0) & 3)
        });
    }
    let take = t.img.rel32_target(f + 0x25);
    let pw = |t: &Twin, o: u32| t.img.word(take + o);
    let (idx, n, table, page) = (pw(&t, 0x08), pw(&t, 0x0f), pw(&t, 0x37), pw(&t, 0x2b));
    let recs_at = SCRATCH + 0x8000;
    t.set_u32(table, recs_at);
    // the give-back's node: the task alone
    let give = t.img.rel32_target(f + 0x11);
    let task = SCRATCH + 0x600;
    for (k, (count, records)) in [(0i32, 4u32), (3, 4), (1, 1), (2, 0)]
        .into_iter()
        .enumerate()
    {
        let k = k as u32;
        t.set_u32(w(&t, 0x6), count.cast_unsigned());
        t.set_u32(w(&t, 0x82), 0x00a0_0000 + k);
        for i in 0..4 {
            t.write(recs_at + 0x74 * i, &[0x5a; 0x74]);
            t.set_u32(recs_at + 0x74 * i + 0x6c, 0);
        }
        t.set_u32(n, records);
        t.set_u32(idx, 0);
        t.set_u32(page, 0);
        t.write(task, &[0; 0x1c]);
        t.set_u32(task + 8, task);
        for op in [0x48u32, 0x55, 0x64, 0x74] {
            t.set_u32(t.img.word(give + op), SCRATCH + 0x3000 + op);
        }
        t.set_u32(SCRATCH + 0x3074, SCRATCH + 0x3074);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `r21::lift_0301` (`magic_0301` 0x10001a80): two records from slot 434
/// (their current entries 0 and 3), slot 474 writing the point from the
/// vector it is handed.
#[test]
fn lift_0301() {
    let Some(mut t) = twin(301) else { return };
    let f = t.at(0x1a80);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    recs(&mut t, &[(444, 1), (450, 12), (451, 12)]);
    let (r1, r2) = (SCRATCH + 0x1000, SCRATCH + 0x1400);
    rec(&mut t, 434, 3, move |m, _| {
        let n = m.r32(magic_owna::row(434));
        if n % 2 == 1 { r1 } else { r2 }
    });
    rec(&mut t, 474, 7, |m, a| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[2] + 4 * i)) * 0.5 + (a[1] as f32);
            m.w32(a[3] + 4 * i, v.to_bits());
        }
        0x474
    });
    let a = SCRATCH + 0x800;
    t.set_u32(w(&t, 0x11), a);
    t.set_u32(a + 0x198, 0x0198_0001);
    let obj = SCRATCH + 0x600;
    for (k, (c1, c2)) in [(0u32, 3u32), (2, 0)].into_iter().enumerate() {
        let k = k as u32;
        junk(&mut t, r1, 0x200, k);
        junk(&mut t, r2, 0x200, k + 1);
        t.set_u32(r1 + 0xac, c1);
        t.set_u32(r2 + 0xac, c2);
        t.check_entry(f, Call::default(), &[obj + k], Eax::Checked);
    }
}

/// `r21::burst_0305` (`magic_0305` 0x10001cf0): four sparks from a pool
/// of 6, 2 and 0 records; slot 314 answering from its argument and count.
#[test]
fn burst_0305() {
    let Some(mut t) = twin(305) else { return };
    let f = t.at(0x1cf0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 314, 1, |m, a| m.r32(magic_owna::row(314)) ^ a[0]);
    t.stub_st0(314, |m, a| {
        f64::from(f32::from_bits(a[0])) * 0.375 + f64::from(m.r32(magic_owna::row(314)) % 7)
    });
    let take = t.img.rel32_target(f + 0x11);
    let pw = |t: &Twin, o: u32| t.img.word(take + o);
    let (idx, n, table, page) = (pw(&t, 0x08), pw(&t, 0x0f), pw(&t, 0x37), pw(&t, 0x2b));
    let recs_at = SCRATCH + 0x8000;
    t.set_u32(table, recs_at);
    let parts = SCRATCH + 0x700;
    t.set_u32(w(&t, 0x21), parts);
    let obj = SCRATCH + 0x1000;
    for (k, records) in [6u32, 2, 0].into_iter().enumerate() {
        let k = k as u32;
        for i in 0..4 {
            t.set_u32(parts + 4 * i, 0x0aa0_0000 + i + k);
        }
        for i in 0..6 {
            t.write(recs_at + 0x74 * i, &[0x5a; 0x74]);
            t.set_u32(recs_at + 0x74 * i + 0x6c, 0);
        }
        t.set_u32(n, records);
        t.set_u32(idx, 0);
        t.set_u32(page, 0);
        junk(&mut t, obj, 0x100, k);
        for i in 0..4u32 {
            t.set_u32(
                obj + 0xd0 + 4 * i,
                (1.5 * i as f32 - 2.0 + k as f32).to_bits(),
            );
        }
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}
