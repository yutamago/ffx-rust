//! `magic_0529.dll` against its Rust port (`ffx_magic::m0529`). Needs
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

const N: u16 = 529;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The steps, the counting ones at counts around their limits and
/// periods.
#[test]
fn steps() {
    let Some(mut t) = twin(N) else { return };
    let s = SCRATCH + 0x100;
    t.set_u32(0x100b_1f10, s);
    t.set_u32(s + 0x198, 0x00ac_7000);
    let fl = SCRATCH + 0x300;
    t.set_u32(0x100b_1f9c, fl);
    rec(&mut t, 481, 1, |_, a| 0x0481_0000 ^ a[0]);
    recs(
        &mut t,
        &[
            (278, 3),
            (450, 2),
            (449, 2),
            (444, 1),
            (525, 1),
            (8, 2),
            (434, 2),
            (526, 0),
            (467, 1),
            (446, 2),
            (276, 2),
            (296, 2),
            (466, 1),
            (527, 1),
        ],
    );
    let seq = SCRATCH + 0x1000;
    let count = 0x100b_1f98;
    t.write(seq, &[0x51; 0x40]);
    for rva in [0x1000u32, 0x1100] {
        t.check_entry(t.at(rva), Call::default(), &[seq], Eax::Checked);
    }
    for n in [
        0u32,
        3,
        4,
        7,
        11,
        23,
        0x47,
        0x4a,
        0x4f,
        0x50,
        0x81,
        0x82,
        0x95,
        0x96,
        0xffff_fff3,
    ] {
        for rva in [0x1790u32, 0x16c0, 0x1800] {
            t.set_u32(count, n);
            t.set_u32(0x100b_1f9c, fl);
            t.set_u32(fl, 0x7777_7777);
            t.set_u32(fl + 0x10, 0xff);
            t.set_u32(fl + 4, 0x00f1_0004);
            t.set_u32(seq, 0x10);
            t.check_entry(t.at(rva), Call::default(), &[seq], Eax::Checked);
        }
    }
}
