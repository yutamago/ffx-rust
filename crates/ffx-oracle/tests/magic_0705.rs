//! `magic_0705.dll` against its Rust port (`ffx_magic::m0705`). Needs
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

use ffx_magic::{Ret, m0705};
use magic::{Call, Eax};
use magic_owna::{rec, recs, twin};

const N: u16 = 705;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100011b0: negative, low, middle, last and too large steps.
#[test]
fn fade() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x11b0);
    rec(&mut t, 776, 1, |_, a| a[0] ^ 0x0776_0000);
    recs(&mut t, &[(915, 5)]);
    for i in [-3i32, 0, 1, 2, 17, 30, 31, 40] {
        let c = Call {
            eax: 0x00ea_0000,
            ecx: i.cast_unsigned(),
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0705::fade(g, img, fr))
        });
    }
}

/// 0x100010d0: times outside, rising, holding and falling.
#[test]
fn tint() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x10d0);
    recs(&mut t, &[(353, 1)]);
    let time = t.img.word(at + 0x11);
    let colour = [0.25f32, -1.5, 2.0, f32::NAN].map(f32::to_bits);
    for tm in [-5i32, 0, 1, 30, 60, 61, 270, 271, 329, 330, 400] {
        t.set_u32(time, tm.cast_unsigned());
        let args = [0x0a, 0x0b, 0x0c, colour[0], colour[1], colour[2], colour[3]];
        t.check(at, Call::default(), &args, Eax::Checked, |g, img, fr| {
            Ret::eax(m0705::tint(g, img, fr))
        });
    }
}

/// 0x10001240: times outside, at 1, holding, falling to 0 and past it.
#[test]
fn glow() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1240);
    recs(&mut t, &[(821, 1), (822, 4), (823, 2), (824, 4)]);
    let time = t.img.word(at + 0x1);
    let d = magic_hi::konst(&t, at, 0x42);
    #[allow(clippy::cast_possible_truncation)]
    let zero_at = 100 + d as i32;
    for tm in [-1i32, 0, 1, 2, 50, 100, 101, zero_at, zero_at + 1, 119, 120] {
        t.set_u32(time, tm.cast_unsigned());
        let c = Call {
            eax: 0x00ea_0000,
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0705::glow(g, img, fr))
        });
    }
}
