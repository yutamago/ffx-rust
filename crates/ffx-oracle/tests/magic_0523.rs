//! `magic_0523.dll` against its Rust port (`ffx_magic::m0523`). Needs
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

const N: u16 = 523;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The table's functions on records 0 and 1, slot 712 saying 0 and 1.
#[test]
fn entries() {
    let Some(mut t) = twin(N) else { return };
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(0x1002_7754, o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 324, 0, move |m, _| {
        let k = m.r32(cell);
        m.w32(cell, k.wrapping_mul(0x0101_0101).wrapping_add(0x1357));
        k
    });
    rec(&mut t, 712, 2, move |m, _| m.r32(cell) & 1);
    recs(
        &mut t,
        &[
            (714, 5),
            (237, 4),
            (258, 1),
            (231, 3),
            (713, 1),
            (400, 1),
            (401, 9),
            (398, 2),
        ],
    );
    let base = t.img.word(t.at(0x1000) + 0x40);
    let n = t.img.word(t.at(0x1380) + 0x4);
    for (k, r) in [0u32, 1].into_iter().enumerate() {
        let obj = base + 0x80 * r;
        t.set_u32(cell, 0x4321 + k as u32);
        t.set_u32(n, r);
        for rva in [0x1000u32, 0x1150, 0x1240, 0x1380] {
            t.check_entry(t.at(rva), Call::default(), &[obj], Eax::Checked);
        }
    }
}
