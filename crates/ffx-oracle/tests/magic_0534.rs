//! `magic_0534.dll` against its Rust port (`ffx_magic::m0534`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::many_single_char_names
)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, recs, twin};

const N: u16 = 534;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The steps, the target count below and at the number of targets.
#[test]
fn steps() {
    let Some(mut t) = twin(N) else { return };
    let s = SCRATCH + 0x100;
    t.set_u32(0x1007_c2b0, s);
    t.set_u32(s + 0x198, 0x00ac_7000);
    t.set_u32(s + 0x19c, 3);
    let other = SCRATCH + 0x2000;
    rec(&mut t, 434, 2, move |_, _| other);
    recs(
        &mut t,
        &[
            (450, 2),
            (451, 2),
            (447, 2),
            (459, 3),
            (449, 2),
            (441, 3),
            (443, 2),
            (440, 4),
        ],
    );
    let seq = SCRATCH + 0x1000;
    let count = t.img.word(t.at(0x11a0) + 0x4);
    for (k, n) in [0u32, 1, 2, 7].into_iter().enumerate() {
        t.set_u32(count, n);
        t.write(seq, &[0; 0x140]);
        // the words past the colour non-zero, so the cleared `+0x13c`
        // shows (and a store one word on)
        t.set_u32(seq + 0x13c, 0x5a5a_5a5a);
        t.set_u32(seq + 0x140, 0x5a5a_5a5a);
        t.write(other, &[0; 0x100]);
        t.set_u32(seq + 0xac, k as u32);
        t.set_u32(other + 0xac, 3 - k as u32);
        for rva in [0x11a0u32, 0x12b0, 0x1390] {
            magic_h6::scrub(&mut t);
            t.check_entry(t.at(rva), Call::default(), &[seq], Eax::Checked);
        }
    }
}

/// 0x10001430: the aim step for two targets and lengths (the step count
/// 0 and not); the point stubs fill their outputs, slot 453 answers the
/// length in ST0.
#[test]
fn aim() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1430);
    let cell = SCRATCH + 0x10;
    let s = SCRATCH + 0x2000;
    t.set_u32(t.img.word(at + 0x12), s);
    t.write(s, &[0x2b; 0x200]);
    for i in 0..4u32 {
        t.set_u32(s + 0x1a0 + 4 * i, 0x00a0_0000 + i);
    }
    // the handle and the word after it distinct
    t.set_u32(s + 0x198, 0x00a0_0198);
    t.set_u32(s + 0x19c, 0x00a0_019c);
    rec(&mut t, 474, 4, |m, a| {
        // the vector A as it gets it
        magic_owna::dump(m, 474, a[2], 4);
        for i in 0..4u32 {
            m.w32(a[3] + 4 * i, (1.5 + i as f32).to_bits());
        }
        0x474
    });
    rec(&mut t, 457, 3, |m, a| {
        for i in 0..4u32 {
            m.w32(
                a[2] + 4 * i,
                (10.25 - 2.0 * i as f32 + (a[0] & 0xf) as f32).to_bits(),
            );
        }
        0x457
    });
    rec(&mut t, 453, 1, |m, a| {
        magic_owna::dump(m, 453, a[0], 4);
        0x453
    });
    t.stub_st0(453, move |m, _| f64::from(f32::from_bits(m.r32(cell))));
    // slot 444 keeps the vector D it gets
    rec(&mut t, 444, 2, |m, a| {
        magic_owna::dump(m, 444, a[1], 4);
        0x444
    });
    recs(&mut t, &[(440, 4), (450, 2)]);
    let seq = SCRATCH + 0x1000;
    for (k, len) in [(0u32, 33.5f32), (1, 0.25), (0, 900.0)] {
        t.write(seq, &[0x3c; 0x140]);
        t.set_u32(seq + 0xac, k);
        t.set_u32(seq + 0x2c + 16 * k, 2 - k);
        t.set_u32(cell, len.to_bits());
        magic_h6::scrub(&mut t);
        // junk read: A's fourth word (ebp - 0x38) is never written; 0 on
        // both sides (brief 2026-10-24)
        t.set_u32(magic::STACK_TOP - 0x100 - 4 - 0x38, 0);
        t.check_entry(at, Call::default(), &[seq], Eax::Checked);
    }
}
