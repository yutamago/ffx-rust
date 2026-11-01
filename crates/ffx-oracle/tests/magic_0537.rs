//! `magic_0537.dll` against its Rust port (`ffx_magic::m0537`). Needs
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

use ffx_magic::{Ret, m0537};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, recs, twin};

const N: u16 = 537;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001000, 0x10001150 (slot 712 saying 0 and 1) and 0x10001580 (off,
/// on with slot 261's answers; the timeline idle).
#[test]
fn part_and_frame() {
    let Some(mut t) = twin(N) else { return };
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(0x1003_0fa4, o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 324, 0, move |m, _| m.r32(cell) ^ 0x5a5a);
    // slot 712 keeps the part pointer's cell and the game flag
    let fd = t.at(0x1150);
    let (gp, lock) = (magic_h6::cell(&t, fd, 0x4b), magic_h6::cell(&t, fd, 0x58));
    magic_h6::rec_cells(&mut t, 712, 2, &[gp, lock], move |m, _| m.r32(cell) & 1);
    rec(&mut t, 253, 0, move |m, _| (m.r32(cell) >> 4) & 1);
    rec(&mut t, 261, 1, move |m, a| (m.r32(cell) >> (a[0] & 3)) & 1);
    recs(
        &mut t,
        &[
            (714, 5),
            (258, 1),
            (231, 3),
            (230, 2),
            (713, 1),
            (723, 1),
            (724, 2),
            (214, 1),
        ],
    );
    let at = t.at(0x1580);
    let ftl = t.img.rel32_target(at + 0x72);
    let fst = t.img.rel32_target(at + 0x8d);
    t.set_u32(t.img.word(ftl + 0x06), 0);
    t.set_u32(t.img.word(fst + 0x02), 0);
    // the word the frame copies and the one it passes to slot 214
    magic_h6::distinct(&mut t, at, &[0x55, 0x98]);
    let part = SCRATCH + 0x1000;
    for (on, bits) in [
        (1u32, 0b0_0000u32),
        (0, 0b1_0001),
        (1, 0b1_0001),
        (1, 0b1_0010),
        (1, 0b1_0011),
    ] {
        t.set_u32(cell, bits);
        t.write(part, &[0x5b; 0x80]);
        t.set_u32(gp, 0x7777_7777);
        t.check(
            t.at(0x1000),
            Call::this(part),
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0537::start(g, img, fr.at, fr.ecx)),
        );
        t.set_u32(gp, 0x7777_7777);
        t.check(
            t.at(0x1150),
            Call::this(part),
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0537::draw(g, img, fr.at, fr.ecx)),
        );
        t.set_u32(t.img.word(at + 0x16), on);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x100012a0: the effect's set-up; the point and its goal distinct
/// floats, the handle a block with three program words, slot 711 keeping
/// the game flag, the timeline block non-zero first.
#[test]
fn setup() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x12a0);
    let blk = SCRATCH + 0x600;
    for i in 0..8u32 {
        t.set_u32(blk + 4 * i, 0x0b00_0000 + 0x11 * i);
    }
    for (i, op) in [0x7u32, 0x36, 0x64].into_iter().enumerate() {
        let a = t.img.word(at + op);
        t.set_u32(a, (1.25 + i as f32).to_bits());
    }
    for (i, op) in [0xfu32, 0x52, 0x80].into_iter().enumerate() {
        let a = t.img.word(at + op);
        t.set_u32(a, (-7.5 + 3.0 * i as f32).to_bits());
    }
    let lock = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x115), lock);
    t.set_u32(t.img.word(at + 0x13c), lock);
    rec(&mut t, 803, 2, |_, _| 0x2100_0000);
    rec(&mut t, 719, 3, move |_, _| blk);
    rec(&mut t, 711, 3, move |m, _| {
        let l = m.r32(lock);
        m.w32(SCRATCH + 0x2_0400, l);
        0x711
    });
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    recs(&mut t, &[(718, 1), (222, 4), (710, 4)]);
    let tl = t.img.word(at + 0x1b9);
    t.write(tl, &[0x5b; 0x110]);
    t.set_u32(lock, 0x7777);
    t.set_u32(t.img.word(at + 0x147), 0x00a5_0147);
    t.check_entry(at, Call::default(), &[0x00a6_0000], Eax::Checked);
}
