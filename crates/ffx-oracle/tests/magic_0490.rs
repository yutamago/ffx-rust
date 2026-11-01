//! `magic_0490.dll` against its Rust port (`ffx_magic::m0490`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, recs, twin};

const N: u16 = 490;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001130 then 0x10001370: the start-up, then the frame (bound to
/// `s4_misc::frame_step`) off and on with slot 261's answers (the
/// timeline idle).
#[test]
fn start_up_and_frame() {
    let Some(mut t) = twin(N) else { return };
    let d = SCRATCH + 0x400;
    for i in 0..8 {
        t.set_u32(d + 4 * i, 0x0d00_0000 + i);
    }
    rec(&mut t, 803, 2, |_, _| 0x2100_0000);
    rec(&mut t, 719, 3, move |_, _| d);
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    rec(&mut t, 260, 1, |_, a| 0x0260_0000 ^ a[0]);
    recs(
        &mut t,
        &[(718, 3), (222, 4), (710, 4), (214, 1), (723, 1), (724, 2)],
    );
    let su = t.at(0x1130);
    // slot 711 keeps the game flag it runs under
    let lock = magic_h6::cell(&t, su, 0x83);
    t.set_u32(lock, 0x00a1_0000);
    magic_h6::rec_cells(&mut t, 711, 3, &[lock], |_, a| {
        0x5a5a_0711 ^ a.first().copied().unwrap_or(0)
    });
    let blk = t.img.word(su + 0x127);
    t.write(blk, &[0x5e; 0x110]);
    t.check_entry(su, Call::default(), &[0x00a6_0000], Eax::Checked);
    let at = t.at(0x1370);
    let ftl = t.img.rel32_target(at + 0x61);
    let fst = t.img.rel32_target(at + 0x7c);
    t.set_u32(t.img.word(ftl + 0x06), 0);
    t.set_u32(t.img.word(fst + 0x02), 0);
    let cell = SCRATCH + 0x10;
    // slot 261 (k) answers bit k of the cell
    rec(&mut t, 261, 1, move |m, a| (m.r32(cell) >> (a[0] & 3)) & 1);
    for (on, bits) in [(0u32, 0u32), (1, 0), (1, 0b01), (1, 0b10), (1, 0b11)] {
        t.set_u32(t.img.word(at + 0x2), on);
        t.set_u32(cell, bits);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
