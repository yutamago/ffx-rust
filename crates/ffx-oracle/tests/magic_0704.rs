//! `magic_0704.dll` against its Rust port (`ffx_magic::m0704`). Needs
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

use ffx_magic::{Ret, m0704};
use magic::{Call, Eax};
use magic_owna::{rec, recs, twin};

const N: u16 = 704;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001000: negative, low, middle, last and too large steps.
#[test]
fn fade() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1000);
    rec(&mut t, 776, 1, |_, a| a[0] ^ 0x0776_0000);
    recs(&mut t, &[(915, 5)]);
    for i in [-3i32, 0, 1, 2, 5, 8, 9, 12] {
        let c = Call {
            eax: 0x00ea_0000,
            ecx: i.cast_unsigned(),
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0704::fade(g, img, fr))
        });
    }
}

/// 0x10001080: one load.
#[test]
fn load() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1080);
    rec(&mut t, 719, 3, |_, a| a[2] ^ 0x0719_0000);
    recs(&mut t, &[(718, 1), (222, 4), (221, 4)]);
    t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0704::load(g, img, fr))
    });
}
