//! `magic_0568.dll` against its Rust port (`ffx_magic::m0568`). Needs
//! `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss)]

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::twin;

const N: u16 = 568;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001c30: the sway at times inside the table, at its last key and
/// past it.
#[test]
fn sway() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1c30);
    let s = SCRATCH + 0x4000;
    t.set_u32(t.img.word(at + 0x3), s);
    let tab = t.img.word(at + 0x17);
    for i in 0..6u32 {
        t.set_u32(tab + 4 * i, (0.5 + 1.25 * i as f32).to_bits());
    }
    for time in [0.0f32, 1.3, 3.75, 4.0, 4.5] {
        t.write(s, &[0x2b; 0x1800]);
        t.set_u32(s + 0x16c4, time.to_bits());
        t.set_u32(s + 0x143c, 2.5f32.to_bits());
        t.check(at, Call::default(), &[], Eax::Junk, |g, img, fr| {
            ffx_magic::m0568::sway(g, img, fr)
        });
    }
}
