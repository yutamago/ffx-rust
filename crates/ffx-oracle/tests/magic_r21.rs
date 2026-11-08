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
            (711, 7),
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
    rec(&mut t, 712, 12, move |m, _| m.r32(answer));
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
    let (g7, g12, lock, stop) = (
        SCRATCH + 0x700,
        SCRATCH + 0x704,
        SCRATCH + 0x708,
        SCRATCH + 0x70c,
    );
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
