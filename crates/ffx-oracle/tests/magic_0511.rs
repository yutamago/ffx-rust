//! `magic_0511.dll` against its Rust port (`ffx_magic::m0511`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::many_single_char_names
)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_hi::{list, pool};
use magic_owna::twin;

const N: u16 = 511;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10002be0: the count running and run out.
#[test]
fn fade() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2be0);
    let fx = SCRATCH + 0x1000;
    t.set_u32(t.img.word(at + 0x2), fx);
    for n in [5u32, 1, 0, 0xffff_fff0] {
        t.write(fx + 0xe90, &[0x1d; 0x20]);
        t.set_u32(fx + 0x225c, n);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10002c50 and 0x10002da0: an empty list and one with a node.
#[test]
fn starts() {
    let Some(mut t) = twin(N) else { return };
    for rva in [0x2c50u32, 0x2da0] {
        let at = t.at(rva);
        let fx = SCRATCH + 0x8000;
        t.set_u32(t.img.word(at + 0x4), fx);
        let take = t.img.rel32_target(at + 0x67);
        for full in [false, true] {
            pool(&mut t, take, SCRATCH + 0x2000, 3);
            let (head, tail) = (t.img.word(at + 0x6d), t.img.word(at + 0x83));
            list(&mut t, head, tail, SCRATCH + 0x3000, full);
            t.write(fx + 0x1b00, &[0x5a; 0x200]);
            magic_h6::scrub(&mut t);
            t.check_entry(at, Call::default(), &[], Eax::Checked);
        }
    }
}

/// 0x100042f0: an empty list and one with a node.
#[test]
fn sprite_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x42f0);
    let s = SCRATCH + 0x100;
    t.set_u32(t.img.word(at + 0x11), s);
    t.set_u32(s + 0x28, 0x00a0_0028);
    let fx = SCRATCH + 0x8000;
    t.set_u32(t.img.word(at + 0x18), fx);
    let b = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x21), b);
    t.set_u32(b, 3);
    magic_owna::rec(&mut t, 219, 2, |_, a| 0x0219_0000 ^ a[0]);
    magic_owna::rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            m.w32(a[2] + 4 * i, 0x1111_1111 * (i + 1));
        }
        0x268
    });
    let take = t.img.rel32_target(at + 0xa5);
    for full in [false, true] {
        pool(&mut t, take, SCRATCH + 0x2000, 2);
        let (head, tail) = (t.img.word(at + 0xbe), t.img.word(at + 0xab));
        list(&mut t, head, tail, SCRATCH + 0x3000, full);
        t.write(fx + 0x2000, &[0x5a; 0x280]);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
