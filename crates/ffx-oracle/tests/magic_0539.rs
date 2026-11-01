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
mod magic_h6;
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
    // slot 712 keeps the part pointer's cell and the game flag
    let fo = t.at(0x10a0);
    let (gp, lock) = (magic_h6::cell(&t, fo, 0x4d), magic_h6::cell(&t, fo, 0x5d));
    magic_h6::rec_cells(&mut t, 712, 2, &[gp, lock], move |m, _| m.r32(cell) & 1);
    rec(&mut t, 261, 1, move |m, a| (m.r32(cell) >> (a[0] & 3)) & 1);
    recs(
        &mut t,
        &[(258, 1), (231, 3), (230, 2), (723, 1), (724, 2), (214, 1)],
    );
    let at = t.at(0x1640);
    // the word the frame copies and the one slot 214 gets, distinct
    magic_h6::distinct(&mut t, at, &[0x44, 0xbb]);
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
        t.set_u32(gp, 0x7777_7777);
        t.check_entry(fo, Call::default(), &[obj], Eax::Checked);
        t.set_u32(t.img.word(at + 0x2), on);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001170: the effect's start; the handle a block with four program
/// words, the globals it passes distinct, slots 711 and 714 keeping the
/// game flag they run under.
#[test]
fn start_effect() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1170);
    let blk = SCRATCH + 0x600;
    for i in 0..8u32 {
        t.set_u32(blk + 4 * i, 0x0b00_0000 + 0x11 * i);
    }
    let table = SCRATCH + 0x700;
    t.set_u32(table, 0x0f00_0000);
    let lock = SCRATCH + 0x20;
    let part = SCRATCH + 0x30;
    t.set_u32(t.img.word(at + 0x77), lock);
    t.set_u32(t.img.word(at + 0x9e), lock);
    t.set_u32(t.img.word(at + 0x119), part);
    let wrapper = t.img.rel32_target(at + 0x149);
    t.set_u32(t.img.word(wrapper + 0x4), lock);
    t.set_u32(t.img.word(wrapper + 0x29), lock);
    rec(&mut t, 719, 3, move |_, _| blk);
    rec(&mut t, 941, 1, move |_, a| {
        if a[0] == 0x0b00_0044 {
            table
        } else {
            0x0941_0000 ^ a[0]
        }
    });
    for s in [711u32, 714] {
        rec(&mut t, s, 3, move |m, _| {
            let k = m.r32(magic_owna::row(s));
            let l = m.r32(lock);
            let p = m.r32(part);
            m.w32(SCRATCH + 0x2_0400 + 0x10 * (s % 4) + 8 * (k % 2), l);
            m.w32(SCRATCH + 0x2_0404 + 0x10 * (s % 4) + 8 * (k % 2), p);
            s
        });
    }
    recs(&mut t, &[(923, 1), (718, 1), (222, 4), (378, 1), (710, 4)]);
    // the buffer word slot 710 gets, distinct
    let buf = t.img.word(at + 0xa9);
    t.set_u32(buf, 0x00a5_00a9);
    for k in 0..2u32 {
        t.set_u32(lock, 0x7777 + k);
        t.set_u32(part, 0x6666);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x100013b0: the effect's set-up; the handle a block with three
/// program words, slot 711 keeping the game flag, the timeline blocks
/// non-zero first.
#[test]
fn setup() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x13b0);
    let blk = SCRATCH + 0x600;
    for i in 0..8u32 {
        t.set_u32(blk + 4 * i, 0x0b00_0000 + 0x11 * i);
    }
    let lock = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x93), lock);
    t.set_u32(t.img.word(at + 0xba), lock);
    rec(&mut t, 803, 2, |_, _| 0x2100_0000);
    rec(&mut t, 719, 3, move |_, _| blk);
    rec(&mut t, 711, 3, move |m, _| {
        let l = m.r32(lock);
        m.w32(SCRATCH + 0x2_0400, l);
        0x711
    });
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    recs(&mut t, &[(923, 1), (718, 1), (222, 4), (710, 4)]);
    for op in [0x137u32, 0x17f] {
        let a = t.img.word(at + op);
        t.write(a, &[0x5b; 0x110]);
    }
    t.set_u32(lock, 0x7777);
    t.check_entry(at, Call::default(), &[0x00a6_0000], Eax::Checked);
}
