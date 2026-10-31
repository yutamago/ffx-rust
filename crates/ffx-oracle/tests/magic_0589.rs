//! `magic_0589.dll` against its Rust port (`ffx_magic::m0589`). Needs
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
use magic_owna::twin;

const N: u16 = 589;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100028b0: the list (shared with the fade task) empty and not.
#[test]
fn flash() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x28b0);
    let r = SCRATCH + 0x8000;
    t.set_u32(t.img.word(at + 0x13), r);
    let fade = t.img.rel32_target(at + 0x25);
    let take = t.img.rel32_target(at + 0x8a);
    for empty in [true, false] {
        pool(&mut t, take, SCRATCH + 0x1000, 3);
        t.write(r + 0x1a00, &[0x3b; 0x300]);
        let (fh, ft) = (t.img.word(fade + 0x7b), t.img.word(fade + 0x93));
        list(&mut t, fh, ft, SCRATCH + 0x3000, !empty);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10002980: an empty list and one with a node.
#[test]
fn two_parts() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2980);
    let fx = SCRATCH + 0x8000;
    t.set_u32(t.img.word(at + 0x4), fx);
    let take = t.img.rel32_target(at + 0x67);
    for full in [false, true] {
        pool(&mut t, take, SCRATCH + 0x2000, 3);
        let (head, tail) = (t.img.word(at + 0x6d), t.img.word(at + 0x83));
        list(&mut t, head, tail, SCRATCH + 0x3000, full);
        t.write(fx + 0x1a00, &[0x5a; 0x200]);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
