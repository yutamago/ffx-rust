//! `magic_0672.dll` against its Rust port (`ffx_magic::m0672`). Needs
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

use ffx_magic::{Ret, m0672};
use magic::{Call, Eax};
use magic_owna::{recs, twin};

const N: u16 = 672;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100016a0: one camera.
#[test]
fn camera_reset() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x16a0);
    recs(&mut t, &[(357, 3), (356, 6), (399, 14)]);
    t.set_u32(t.img.word(at + 0x8), 0x00ca_0000);
    t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0672::camera_reset(g, img, fr))
    });
}
