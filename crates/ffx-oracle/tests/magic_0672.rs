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

/// 0x100015f0: no tracks, a negative count, and two tracks with no and
/// three keys.
#[test]
fn copy_keys() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x15f0);
    let cell = magic::SCRATCH + 0x10;
    let (src, dst) = (magic::SCRATCH + 0x1000, magic::SCRATCH + 0x2000);
    magic_owna::rec(&mut t, 586, 1, move |m, _| m.r32(cell));
    magic_owna::rec(&mut t, 584, 2, move |_, a| src + 0x100 * a[1]);
    magic_owna::rec(&mut t, 585, 2, move |m, a| {
        m.w32(a[2], 0x30 * a[1]);
        dst + 0x100 * a[1]
    });
    recs(&mut t, &[(346, 1), (399, 3)]);
    t.set_u32(t.img.word(at + 0x15), 0x00ca_0000);
    let pattern: Vec<u8> = (0..0x200u32)
        .map(|i| u8::try_from(i & 0xff).unwrap())
        .collect();
    for n in [0i32, -1, 2] {
        t.set_u32(cell, n.cast_unsigned());
        t.write(src, &pattern);
        t.write(dst, &[0xdd; 0x200]);
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0672::copy_keys(g, img, fr))
        });
    }
}
