//! `magic_0540.dll` against its Rust port (`ffx_magic::m0540`). Needs
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

const N: u16 = 540;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The table's functions (slot 712 saying 0 and 1; records 0 and 1).
#[test]
fn entries() {
    let Some(mut t) = twin(N) else { return };
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(0x1009_2cfc, o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    let models = t.img.word(t.at(0x10a0) + 0x92);
    for i in 0..9 {
        t.set_u32(models + 4 * i, 0x00d0_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 712, 2, move |m, _| m.r32(cell));
    recs(
        &mut t,
        &[
            (714, 5),
            (276, 2),
            (845, 0),
            (382, 2),
            (315, 1),
            (258, 1),
            (231, 3),
            (230, 2),
            (275, 1),
            (713, 1),
            (846, 0),
        ],
    );
    let base = t.img.word(t.at(0x12c0) + 0xa);
    for (answer, r) in [(0u32, 0u32), (1, 1)] {
        t.set_u32(cell, answer);
        let obj = base + 0x80 * r;
        t.write(obj, &[0x5b; 0x80]);
        for rva in [0x10a0u32, 0x1170, 0x1250, 0x12c0] {
            t.check_entry(t.at(rva), Call::default(), &[obj], Eax::Checked);
        }
    }
}

/// 0x10001a00: off, and on with slot 261's answers, the task started
/// once (the timelines idle).
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let cell = SCRATCH + 0x10;
    rec(&mut t, 261, 1, move |m, a| (m.r32(cell) >> (a[0] & 3)) & 1);
    rec(&mut t, 417, 4, |_, _| 0x0041_7000);
    recs(&mut t, &[(723, 1), (724, 2), (418, 1), (214, 2)]);
    let at = t.at(0x1a00);
    for op in [0xa6, 0xc6, 0xf9] {
        let tl = t.img.word(at + op);
        t.set_u32(tl + 8, 0);
    }
    let started = t.img.word(at + 0x62);
    for (on, bits, s) in [
        (0u32, 0b0000u32, 0u32),
        (1, 0b1000, 0),
        (1, 0b1000, 1),
        (1, 0b0100, 0),
        (1, 0b1110, 0),
    ] {
        t.set_u32(t.img.word(at + 0x2), on);
        t.set_u32(cell, bits);
        t.set_u32(started, s);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
