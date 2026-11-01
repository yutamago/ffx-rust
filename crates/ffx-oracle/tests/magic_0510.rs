//! `magic_0510.dll` against its Rust port (`ffx_magic::m0510`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::many_single_char_names
)]

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, recs, twin};

const N: u16 = 510;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The sequence steps, with the flags clear and set and slot 520's two
/// answers.
#[test]
fn steps() {
    let Some(mut t) = twin(N) else { return };
    let s = SCRATCH + 0x100;
    t.set_u32(0x1007_7a90, s);
    t.set_u32(s + 4, 0x0050_0004);
    t.set_u32(s + 0x198, 0x00ac_7000);
    t.set_u32(s + 0x1a0, 0x00ac_7001);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 367, 8, move |m, a| {
        let v = m.r32(cell);
        m.w32(a[4], v);
        0x367
    });
    rec(&mut t, 520, 0, move |m, _| {
        if m.r32(cell) & 1 == 0 { 0x0205_0000 } else { 7 }
    });
    recs(
        &mut t,
        &[
            (523, 2),
            (481, 1),
            (14, 4),
            (448, 2),
            (450, 2),
            (449, 2),
            (444, 1),
            (522, 2),
            (434, 2),
            (486, 11),
            (447, 2),
            (451, 2),
            (442, 3),
        ],
    );
    let seq = SCRATCH + 0x1000;
    for (k, v) in (0u32..).zip([0x100u32, 0x1fe, 0x0123_0001, 0x7fff_ff00, 0x8000_0100]) {
        t.set_u32(cell, v);
        // the flags at 0x100011b0 +0x39 and +0x5f, 0x10001380 +0x5
        for (i, (rva, op)) in (0u32..).zip([(0x11b0u32, 0x39u32), (0x11b0, 0x5f), (0x1380, 0x5)]) {
            let a = t.img.word(t.at(rva) + op);
            t.set_u32(a, (k >> i) & 1);
        }
        t.write(seq, &[0x51; 0x200]);
        t.set_u32(seq + 0xac, k);
        for rva in [0x1000u32, 0x11b0, 0x12a0, 0x1380, 0x1680] {
            t.check_entry(t.at(rva), Call::default(), &[seq], Eax::Checked);
        }
    }
}

/// 0x10001b10 and 0x10001db0: the camera steps, early, at step 5, at
/// their last step.
#[test]
fn camera_steps() {
    let Some(mut t) = twin(N) else { return };
    let s = SCRATCH + 0x100;
    t.set_u32(0x1007_7a90, s);
    t.set_u32(s + 0x1a0, 0x00ac_7001);
    rec(&mut t, 480, 2, |_, a| 0x0480_0000 ^ a[0]);
    rec(&mut t, 477, 1, |_, _| 0x477);
    t.stub_st0(477, |_, a| f64::from(a[0] & 0xff) * 0.5 + 1.25);
    rec(&mut t, 479, 4, |m, a| {
        let v = m.r32(a[3] + 4);
        m.w32(a[3] + 4, v ^ 0x40);
        0x479
    });
    recs(
        &mut t,
        &[(388, 3), (484, 3), (485, 9), (446, 2), (483, 1), (450, 2)],
    );
    for i in 0..0x18u32 {
        t.set_u32(0x1007_7b48 + 4 * i, (0.5 * i as f32 - 3.0).to_bits());
    }
    for i in 0..0x10u32 {
        t.set_u32(0x1007_7b84 + 4 * i, (0.25 * i as f32 + 1.0).to_bits());
    }
    let seq = SCRATCH + 0x1000;
    let count = 0x1007_7b68;
    for (rva, n) in [
        (0x1b10u32, 0u32),
        (0x1b10, 5),
        (0x1b10, 0x1e),
        (0x1b10, 0x1d),
        (0x1db0, 0),
        (0x1db0, 7),
        (0x1db0, 0x14),
    ] {
        t.set_u32(count, n);
        t.set_u32(seq, 0x10);
        t.check_entry(t.at(rva), Call::default(), &[seq], Eax::Checked);
    }
}
