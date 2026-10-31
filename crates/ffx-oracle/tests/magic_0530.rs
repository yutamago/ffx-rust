//! `magic_0530.dll` against its Rust port (`ffx_magic::m0530`). Needs
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

const N: u16 = 530;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001140: the object's start with the light flag clear and set.
#[test]
fn start() {
    let Some(mut t) = twin(N) else { return };
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let at = t.at(0x1140);
    let o = SCRATCH + 0x100;
    t.set_u32(t.img.word(at + 0x10), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            m.w32(a[2] + 4 * i, 0x4000_0000 + i);
        }
        0x268
    });
    recs(&mut t, &[(714, 5), (393, 3)]);
    let obj = SCRATCH + 0x1000;
    for flag in [0u32, 1] {
        t.set_u32(t.img.word(at + 0x58), flag);
        t.write(obj, &[0x5b; 0x80]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}
