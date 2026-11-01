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
mod magic_h6;
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
    let gp = SCRATCH + 0x24;
    t.set_u32(t.img.word(at + 0x71), gp);
    for (i, op) in (0u32..).zip([0x1du32, 0x3a, 0x45, 0x51, 0x59, 0x64]) {
        t.set_u32(t.img.word(at + op), 0x00c0_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    let cell = SCRATCH + 0x10;
    // slots 712 and 713 keep the part cell and the game flag; 712 moves
    // the part cell on
    magic_h6::rec_cells(&mut t, 712, 2, &[gp, lock], move |m, _| {
        m.w32(gp, 0x6666_0712);
        m.r32(cell)
    });
    magic_h6::rec_cells(&mut t, 713, 1, &[gp, lock], |_, a| {
        0x5a5a_0713 ^ a.first().copied().unwrap_or(0)
    });
    recs(&mut t, &[(714, 5), (237, 4), (258, 5), (231, 8), (230, 10)]);
    let obj = SCRATCH + 0x1000;
    for answer in [0u32, 1] {
        t.set_u32(cell, answer);
        t.set_u32(gp, 0x7777_7777);
        t.set_u32(lock, 0x00a1_0000);
        t.write(obj, &[0x5b; 0x80]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x10002200: the flag clear; slot 261's three answers in every mix
/// (the timelines live, so running or stopping one, or the wrong one,
/// shows; the copied word and slot 214's word distinct).
#[test]
fn effect_frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2200);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 261, 1, move |m, a| m.r32(cell) >> a[0] & 1);
    recs(&mut t, &[(723, 1), (724, 2), (214, 1)]);
    rec(&mut t, 253, 0, |_, _| 1);
    t.set_u32(t.img.word(at + 0xd), SCRATCH + 0x20);
    t.set_u32(t.img.word(at + 0x3f), SCRATCH + 0x24);
    magic_h6::distinct(&mut t, at, &[0x44, 0xbb]);
    for (on, bits) in [
        (0u32, 7u32),
        (1, 0),
        (1, 0b101),
        (1, 0b010),
        (1, 0b111),
        (1, 0b001),
        (1, 0b100),
    ] {
        t.set_u32(t.img.word(at + 0x2), on);
        t.set_u32(cell, bits);
        for (k, op) in (0u32..).zip([0x61, 0x81, 0xa1, 0xab]) {
            let tl = t.img.word(at + op);
            magic_h6::live_timeline(&mut t, tl, SCRATCH + 0x600 + 0x20 * k, 0x40 + k);
        }
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001c60: the frame at a bone for the array's first object and
/// another, slot 712 saying 0 and 1; the globals distinct, slot 712
/// keeping the game flag and the part cell.
#[test]
fn frame_bone() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1c60);
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(t.img.word(at + 0xe8), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    let (gp, lock) = (SCRATCH + 0x30, SCRATCH + 0x20);
    t.set_u32(t.img.word(at + 0x8c), gp);
    t.set_u32(t.img.word(at + 0x99), lock);
    t.set_u32(t.img.word(at + 0xb5), lock);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 712, 2, move |m, _| {
        let l = m.r32(lock);
        let p = m.r32(gp);
        m.w32(SCRATCH + 0x2_0400, l);
        m.w32(SCRATCH + 0x2_0404, p);
        // the part cell moved on, as the game's part code may
        m.w32(gp, 0x6666_0712);
        m.r32(cell) & 1
    });
    magic_h6::rec_cells(&mut t, 713, 1, &[gp, lock], |_, a| {
        0x5a5a_0713 ^ a.first().copied().unwrap_or(0)
    });
    recs(&mut t, &[(237, 4), (258, 1), (231, 3), (230, 2), (714, 5)]);
    for op in [0x34u32, 0x55, 0x60, 0x6c, 0x74, 0x7f] {
        let g = t.img.word(at + op);
        t.set_u32(g, 0x00a5_0000 + op);
    }
    let base = t.img.word(at + 0xa);
    for (k, bits) in [(0u32, 0u32), (1, 1), (0, 1)] {
        let obj = base + 0x80 * k;
        t.set_u32(cell, bits);
        t.set_u32(gp, 0x7777);
        t.set_u32(lock, 0x6666);
        t.write(obj, &[0x5b; 0x80]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}
