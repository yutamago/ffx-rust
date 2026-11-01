//! `magic_0616.dll` against its Rust port (`ffx_magic::m0616`). Needs
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

const N: u16 = 616;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100013b0: the flag clear; slot 261 saying 0; the timeline running
/// on and done.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x13b0);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 261, 1, move |m, _| m.r32(cell));
    recs(&mut t, &[(723, 1), (724, 2), (214, 1), (253, 0)]);
    let tl = t.img.rel32_target(at + 0x61);
    let stop = t.img.rel32_target(at + 0x6a);
    t.set_u32(t.img.word(tl + 0x25), 0);
    t.set_u32(t.img.word(stop + 0x02), 0);
    let gp = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0xd), gp);
    let v = SCRATCH + 0x24;
    t.set_u32(t.img.word(at + 0x3f), v);
    for (on, answer, live) in [(0u32, 1u32, 0u32), (1, 0, 0), (1, 1, 1), (1, 1, 0)] {
        t.set_u32(t.img.word(at + 0x2), on);
        t.set_u32(cell, answer);
        t.set_u32(t.img.word(tl + 0x06), live);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001160: one start.
#[test]
fn start_up() {
    let Some(mut t) = twin(N) else { return };
    let d = SCRATCH + 0x400;
    for i in 0..8 {
        t.set_u32(d + 4 * i, 0x0d00_0000 + i);
    }
    rec(&mut t, 803, 2, |_, _| 0x2100_0000);
    rec(&mut t, 719, 3, move |_, _| d);
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    recs(&mut t, &[(718, 3), (222, 4), (711, 3), (710, 4), (260, 7)]);
    let at = t.at(0x1160);
    let blk = t.img.word(at + 0x127);
    t.write(blk, &[0x5e; 0x110]);
    t.check_entry(at, Call::default(), &[0x00a6_0000], Eax::Checked);
}
