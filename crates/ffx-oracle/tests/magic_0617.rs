//! `magic_0617.dll` against its Rust port (`ffx_magic::m0617`). Needs
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

const N: u16 = 617;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100010a0: slot 712 saying 0 and 1.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x10a0);
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(t.img.word(at + 0xaa), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    let lock = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x5c), lock);
    t.set_u32(lock, 0x00a1_0000);
    let gp = SCRATCH + 0x24;
    t.set_u32(t.img.word(at + 0x4f), gp);
    for (i, op) in (0u32..).zip([0x18u32, 0x23, 0x2f, 0x37, 0x42]) {
        t.set_u32(t.img.word(at + op), 0x00c0_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    let cell = SCRATCH + 0x10;
    // slot 712 keeps the part pointer's cell and the game flag, and moves
    // the part pointer on as the game's does; 713 keeps the cell too
    magic_h6::rec_cells(&mut t, 712, 2, &[gp, lock], move |m, _| {
        m.w32(gp, 0x6666_0712);
        m.r32(cell)
    });
    magic_h6::rec_cells(&mut t, 713, 1, &[gp, lock], |_, a| {
        0x5a5a_0000 ^ a.first().copied().unwrap_or(0)
    });
    recs(&mut t, &[(714, 5), (258, 1), (231, 4), (230, 6)]);
    let obj = SCRATCH + 0x1000;
    for answer in [0u32, 1] {
        t.set_u32(cell, answer);
        t.set_u32(gp, 0x7777_7777);
        t.write(obj, &[0x5b; 0x80]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}
