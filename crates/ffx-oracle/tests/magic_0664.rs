//! `magic_0664.dll` against its Rust port (`ffx_magic::m0664`). Needs
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

const N: u16 = 664;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001810: the array's first part, its last byte and the next part.
#[test]
fn ring_part() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1810);
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(t.img.word(at + 0x10), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    t.set_u32(t.img.word(at + 0x4), SCRATCH + 0x24);
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5)]);
    let base = t.img.word(at + 0x3d);
    for obj in [base, base + 0x10, base + 0x80] {
        t.write(obj, &[0x5b; 0x80]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x100012f0: slot 712 saying 0 and 1.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x12f0);
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(t.img.word(at + 0xcc), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    let lock = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x7e), lock);
    t.set_u32(lock, 0x00a1_0000);
    t.set_u32(t.img.word(at + 0x71), SCRATCH + 0x24);
    for (i, op) in (0u32..).zip([0x1du32, 0x3a, 0x45, 0x51, 0x59, 0x64]) {
        t.set_u32(t.img.word(at + op), 0x00c0_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 712, 2, move |m, _| m.r32(cell));
    recs(
        &mut t,
        &[(714, 5), (237, 4), (258, 5), (231, 8), (230, 10), (713, 1)],
    );
    let obj = SCRATCH + 0x1000;
    for answer in [0u32, 1] {
        t.set_u32(cell, answer);
        t.write(obj, &[0x5b; 0x80]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x10002200: the flag clear; slot 261's three answers in every mix.
#[test]
fn effect_frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2200);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 261, 1, move |m, a| m.r32(cell) >> a[0] & 1);
    recs(&mut t, &[(723, 1), (724, 2), (214, 1)]);
    for op in [0x61, 0x81] {
        let tl = t.img.word(at + op);
        t.write(tl, &[0; 0x20]);
    }
    t.set_u32(t.img.word(at + 0xd), SCRATCH + 0x20);
    t.set_u32(t.img.word(at + 0x3f), SCRATCH + 0x24);
    for (on, bits) in [(0u32, 7u32), (1, 0), (1, 0b101), (1, 0b010), (1, 0b111)] {
        t.set_u32(t.img.word(at + 0x2), on);
        t.set_u32(cell, bits);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
