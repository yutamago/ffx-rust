//! `magic_0539.dll` against its Rust port (`ffx_magic::m0539`). Needs
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

const N: u16 = 539;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100010a0 (slot 712 saying 0 and 1, the clock at 31 and not) and
/// 0x10001640 (off, on with slot 261's answers; the timelines idle).
#[test]
fn frames() {
    let Some(mut t) = twin(N) else { return };
    let cell = SCRATCH + 0x10;
    rec(&mut t, 712, 2, move |m, _| m.r32(cell) & 1);
    rec(&mut t, 261, 1, move |m, a| (m.r32(cell) >> (a[0] & 3)) & 1);
    recs(
        &mut t,
        &[(258, 1), (231, 3), (230, 2), (723, 1), (724, 2), (214, 1)],
    );
    let fo = t.at(0x10a0);
    let at = t.at(0x1640);
    for op in [0x61, 0x81] {
        let tl = t.img.word(at + op);
        t.set_u32(tl + 8, 0);
    }
    let clock = t.img.word(fo + 0x8e);
    let obj = SCRATCH + 0x1000;
    for (on, bits, c) in [
        (0u32, 0b000u32, 0x1fu32),
        (1, 0b001, 0x1f),
        (1, 0b010, 3),
        (1, 0b100, 0x1f),
        (1, 0b111, 0),
    ] {
        t.set_u32(cell, bits);
        t.set_u32(clock, c);
        t.check_entry(fo, Call::default(), &[obj], Eax::Checked);
        t.set_u32(t.img.word(at + 0x2), on);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
