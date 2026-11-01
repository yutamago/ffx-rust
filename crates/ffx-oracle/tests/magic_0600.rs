//! `magic_0600.dll` against its Rust port (`ffx_magic::m0600`). Needs
//! `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss)]

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::twin;

const N: u16 = 600;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The actor matrix frame (shared with `magic_0588`): two actors, the
/// float slots answering in ST0, the matrix slots keeping what they get.
#[test]
fn actor_matrix() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x73e0);
    let s = SCRATCH + 0x4000;
    t.set_u32(t.img.word(at + 0x18), s);
    let actors = SCRATCH + 0x300;
    t.set_u32(t.img.word(at + 0x11), actors);
    let hs = t.img.word(at + 0x25);
    for i in 0..8u32 {
        t.set_u32(hs + 4 * i, 0x00ac_0000 + i);
    }
    for (slot, n) in [(277u32, 1usize), (15, 1), (16, 1), (17, 1)] {
        magic_owna::rec(&mut t, slot, n, move |_, _| slot);
        t.stub_st0(slot, move |_, a| {
            0.375 + f64::from(slot) / 64.0 + f64::from(a[0] & 7)
        });
    }
    for (slot, n) in [
        (210u32, 2usize),
        (211, 2),
        (235, 1),
        (330, 3),
        (328, 3),
        (329, 3),
        (257, 2),
    ] {
        magic_owna::rec(&mut t, slot, n, move |m, a| {
            if slot == 210 {
                for i in 0..16u32 {
                    m.w32(a[1] + 4 * i, (1.5 + i as f32).to_bits());
                }
            }
            if slot >= 328 {
                magic_owna::dump(m, 0x100 + slot % 0x40, a[0], 16);
            }
            slot
        });
    }
    for b in [1u8, 3] {
        t.write(s, &[0x2b; 0x1000]);
        for i in 0..0x40u32 {
            t.set_u32(s + 0xb00 + 4 * i, (0.5 + 0.25 * i as f32).to_bits());
        }
        t.write(actors, &[b; 4]);
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::m0588::actor_matrix(g, img, fr)
        });
    }
}

/// 0x10007160: the fade started, the list empty and not.
#[test]
fn fade_start() {
    let Some(mut t) = twin(600) else { return };
    let at = t.at(0x7160);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0xe), fx);
    magic_owna::recs(&mut t, &[(371, 4)]);
    let st = t.img.rel32_target(at + 0x49);
    let take1 = t.img.rel32_target(st + 0x7b);
    let take2 = t.img.rel32_target(at + 0xae);
    let last = SCRATCH + 0x4000;
    for full in [false, true] {
        for tk in [take1, take2] {
            magic_hi::pool(&mut t, tk, SCRATCH + 0x6000, 4);
        }
        t.write(fx + 0xce0, &[0x3d; 0x200]);
        t.write(last, &[0x4c; 0x1c]);
        t.set_u32(t.img.word(at + 0xb4), if full { last } else { 0 });
        t.set_u32(t.img.word(at + 0xc8), if full { last } else { 0 });
        t.check_entry(at, magic::Call::default(), &[], magic::Eax::Checked);
    }
}
