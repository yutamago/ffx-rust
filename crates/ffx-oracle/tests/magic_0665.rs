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
