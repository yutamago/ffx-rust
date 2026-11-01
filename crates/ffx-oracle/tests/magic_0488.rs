//! `magic_0488.dll` against its Rust port (`ffx_magic::m0488`). Needs
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

use ffx_magic::{Ret, m0488};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, recs, twin};

const N: u16 = 488;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001350: slot 712 saying 0 and 2.
#[test]
fn draw() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1350);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 712, 2, move |m, _| m.r32(cell));
    recs(&mut t, &[(258, 1), (231, 3)]);
    for answer in [0u32, 2] {
        t.set_u32(cell, answer);
        let c = Call {
            ecx: SCRATCH + 0x1000,
            edx: SCRATCH + 0x2000,
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0488::draw(g, img, fr.at, fr.ecx, fr.edx))
        });
    }
}

/// 0x100014e0: the start-up.
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
    rec(&mut t, 260, 1, |_, a| 0x0260_0000 ^ a[0]);
    recs(&mut t, &[(718, 3), (222, 4), (711, 3), (710, 4)]);
    let at = t.at(0x14e0);
    let blk = t.img.word(at + 0x140);
    t.write(blk, &[0x5e; 0x110]);
    t.check_entry(at, Call::default(), &[0x00a6_0000], Eax::Checked);
}
