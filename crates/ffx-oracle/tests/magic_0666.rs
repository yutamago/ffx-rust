//! `magic_0666.dll` against its Rust port (`ffx_magic::m0666`). Needs
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

const N: u16 = 666;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001bd0 and 0x10002370: each array's first part, its last byte and
/// the next part.
#[test]
fn pairs() {
    let Some(mut t) = twin(N) else { return };
    let table = SCRATCH + 0x200;
    for i in 0..16 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5)]);
    for rva in [0x1bd0u32, 0x2370] {
        let at = t.at(rva);
        let o = SCRATCH + 0x100;
        t.set_u32(t.img.word(at + 0x10), o);
        t.set_u32(o + 0x10, 0x00b0_0010);
        t.set_u32(t.img.word(at + 0x4), SCRATCH + 0x24);
        let base = t.img.word(at + 0x3d);
        for obj in [base, base + 0x7f, base + 0x80] {
            t.write(obj, &[0x5b; 0x80]);
            t.check_entry(at, Call::default(), &[obj], Eax::Checked);
        }
    }
}
