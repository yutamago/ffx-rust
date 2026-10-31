//! `magic_0598.dll` against its Rust port (`ffx_magic::m0598`). Needs
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
use magic_hi::{list, pool};
use magic_owna::{recs, twin};

const N: u16 = 598;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100077f0: the lists empty and not.
#[test]
fn fade_out() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x77f0);
    recs(&mut t, &[(371, 4)]);
    let r = SCRATCH + 0x8000;
    t.set_u32(t.img.word(at + 0x13), r);
    let fade = t.img.rel32_target(at + 0x25);
    let take = t.img.rel32_target(at + 0x67);
    let fade_take = t.img.rel32_target(fade + 0x75);
    for empty in [true, false] {
        pool(&mut t, take, SCRATCH + 0x1000, 3);
        if fade_take != take {
            pool(&mut t, fade_take, SCRATCH + 0x2000, 3);
        }
        t.write(r + 0xa00, &[0x3b; 0x300]);
        let (fh, ft) = (t.img.word(fade + 0x7b), t.img.word(fade + 0x93));
        list(&mut t, fh, ft, SCRATCH + 0x3000, !empty);
        let (h, l) = (t.img.word(at + 0x6d), t.img.word(at + 0x85));
        list(&mut t, h, l, SCRATCH + 0x3100, !empty);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
