//! `magic_0497.dll` against its Rust port (`ffx_magic::m0497`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, recs, twin};

const N: u16 = 497;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100012f0, then 0x10001000 and 0x100010d0.
#[test]
fn start_up_and_lights() {
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
        &[
            (718, 3),
            (222, 4),
            (711, 3),
            (710, 4),
            (400, 1),
            (401, 9),
            (398, 2),
            (399, 4),
        ],
    );
    let su = t.at(0x12f0);
    let blk = t.img.word(su + 0x127);
    t.write(blk, &[0x5e; 0x110]);
    t.check_entry(su, Call::default(), &[0x00a6_0000], Eax::Checked);
    t.check_entry(t.at(0x1000), Call::default(), &[], Eax::Checked);
    t.check_entry(t.at(0x10d0), Call::default(), &[], Eax::Checked);
}
