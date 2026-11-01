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
mod magic_h6;
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
    // slot 712 keeps the part pointer's cell and the game flag
    let fd = t.at(0x1170);
    let (gp, lock) = (magic_h6::cell(&t, fd, 0x7a), magic_h6::cell(&t, fd, 0x8a));
    magic_h6::rec_cells(&mut t, 712, 2, &[gp, lock], move |m, _| m.r32(cell));
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
            t.set_u32(gp, 0x7777_7777);
            t.check_entry(t.at(rva), Call::default(), &[obj], Eax::Checked);
        }
    }
}

/// 0x10001a00: off, and on with slot 261's answers, the task started
/// once (the timelines live, so running or stopping one, or the wrong
/// one, shows).
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let cell = SCRATCH + 0x10;
    rec(&mut t, 261, 1, move |m, a| (m.r32(cell) >> (a[0] & 3)) & 1);
    rec(&mut t, 417, 4, |_, _| 0x0041_7000);
    rec(&mut t, 253, 0, |_, _| 1);
    recs(&mut t, &[(723, 1), (724, 2), (418, 1), (214, 2)]);
    let at = t.at(0x1a00);
    let started = t.img.word(at + 0x62);
    // the word the frame copies and the one slot 214 gets, distinct
    magic_h6::distinct(&mut t, at, &[0x44, 0x11d]);
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
        for (k, op) in (0u32..).zip([0xa6, 0xc6, 0xf9, 0x103, 0x10d]) {
            let tl = t.img.word(at + op);
            magic_h6::live_timeline(&mut t, tl, SCRATCH + 0x600 + 0x20 * k, 0x40 + k);
        }
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001660: the effect's set-up; the handle a block with three
/// program words, slot 711 keeping the game flag, the timeline blocks
/// non-zero first.
#[test]
fn setup() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1660);
    let blk = SCRATCH + 0x600;
    for i in 0..8u32 {
        t.set_u32(blk + 4 * i, 0x0b00_0000 + 0x11 * i);
    }
    let lock = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x83), lock);
    t.set_u32(t.img.word(at + 0xaa), lock);
    rec(&mut t, 803, 2, |_, _| 0x2100_0000);
    rec(&mut t, 719, 3, move |_, _| blk);
    rec(&mut t, 711, 3, move |m, _| {
        let l = m.r32(lock);
        m.w32(SCRATCH + 0x2_0400, l);
        0x711
    });
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    rec(&mut t, 260, 1, |_, a| 0x0260_0000 ^ a[0]);
    recs(&mut t, &[(718, 1), (222, 4), (710, 4)]);
    for op in [0x127u32, 0x13d, 0x18a] {
        let a = t.img.word(at + op);
        t.write(a, &[0x5b; 0x110]);
    }
    t.set_u32(lock, 0x7777);
    t.set_u32(t.img.word(at + 0xb5), 0x00a5_00b5);
    t.check_entry(at, Call::default(), &[0x00a6_0000], Eax::Checked);
}
