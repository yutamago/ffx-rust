//! `magic_0525.dll` against its Rust port (`ffx_magic::m0525`). Needs
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

const N: u16 = 525;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The sequence steps with the flags clear and set and slot 492's
/// answers 0, 1, 2.
#[test]
fn steps() {
    let Some(mut t) = twin(N) else { return };
    let s = SCRATCH + 0x100;
    t.set_u32(0x1005_bab0, s);
    t.set_u32(s + 0x198, 0x00ac_7000);
    t.set_u32(s + 0x1a0, 0x00ac_7001);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 492, 1, move |m, _| m.r32(cell));
    recs(
        &mut t,
        &[
            (486, 11),
            (434, 2),
            (450, 2),
            (451, 2),
            (446, 2),
            (442, 3),
            (443, 2),
            (440, 4),
            (449, 2),
            (447, 2),
        ],
    );
    let flag = t.img.word(t.at(0x11c0) + 0x6c);
    let seq = SCRATCH + 0x1000;
    for (k, answer) in [0u32, 1, 2].into_iter().enumerate() {
        t.set_u32(cell, answer);
        t.set_u32(flag, k as u32 & 1);
        t.write(seq, &[0x51; 0x140]);
        t.set_u32(seq + 0xac, k as u32);
        for rva in [0x10b0u32, 0x11c0, 0x12f0, 0x1650, 0x16d0] {
            t.check_entry(t.at(rva), Call::default(), &[seq], Eax::Checked);
        }
    }
}
