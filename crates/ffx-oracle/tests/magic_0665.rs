//! `magic_0665.dll` against its Rust port (`ffx_magic::m0665`). Needs
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

use ffx_magic::{Ret, m0665};
use magic::{Call, Eax};
use magic_owna::twin;

const N: u16 = 665;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10002bd0: a dirty block.
#[test]
fn timeline_reset() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2bd0);
    t.write(t.img.word(at + 0x8), &[0x6d; 0x120]);
    t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0665::timeline_reset(g, img, fr))
    });
}

/// 0x10001970: slot 712 saying 0 and 1.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1970);
    let table = magic::SCRATCH + 0x200;
    for i in 0..16 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = magic::SCRATCH + 0x100;
    t.set_u32(t.img.word(at + 0xcc), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    let lock = magic::SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x7e), lock);
    t.set_u32(lock, 0x00a1_0000);
    t.set_u32(t.img.word(at + 0x71), magic::SCRATCH + 0x24);
    for (i, op) in (0u32..).zip([0x1du32, 0x3a, 0x45, 0x51, 0x59, 0x64]) {
        t.set_u32(t.img.word(at + op), 0x00c0_0000 + i);
    }
    magic_owna::rec(&mut t, 941, 1, move |_, _| table);
    let cell = magic::SCRATCH + 0x10;
    magic_owna::rec(&mut t, 712, 2, move |m, _| m.r32(cell));
    magic_owna::recs(
        &mut t,
        &[(714, 5), (237, 4), (258, 5), (231, 8), (230, 10), (713, 1)],
    );
    let obj = magic::SCRATCH + 0x1000;
    for answer in [0u32, 1] {
        t.set_u32(cell, answer);
        t.write(obj, &[0x5b; 0x80]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}
