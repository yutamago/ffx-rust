//! `magic_0527.dll` against its Rust port (`ffx_magic::m0527`). Needs
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
use magic_hi::f;
use magic_owna::{rec, recs, twin};

const N: u16 = 527;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001070 (slot 712 saying 0 and 1) and 0x10001410 (off, on with
/// slot 261's answers; the timeline idle).
#[test]
fn draw_and_frame() {
    let Some(mut t) = twin(N) else { return };
    let cell = SCRATCH + 0x10;
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            m.w32(a[3] + 4 * i, f(i as f32 * 1.5 + 0.25));
        }
        0x237
    });
    rec(&mut t, 712, 2, move |m, _| m.r32(cell) & 1);
    rec(&mut t, 253, 0, move |m, _| (m.r32(cell) >> 4) & 1);
    rec(&mut t, 261, 1, move |m, a| (m.r32(cell) >> (a[0] & 3)) & 1);
    recs(
        &mut t,
        &[(723, 1), (724, 2), (258, 1), (231, 3), (230, 2), (214, 1)],
    );
    let at = t.at(0x1410);
    let ftl = t.img.rel32_target(at + 0x41);
    let fst = t.img.rel32_target(at + 0x5c);
    t.set_u32(t.img.word(ftl + 0x06), 0);
    t.set_u32(t.img.word(fst + 0x02), 0);
    // the words the frame copies and passes, distinct from their
    // neighbours (and the draw's slot 237 word)
    for (rva, op) in [(0x1070u32, 0x57u32), (0x1410, 0x24), (0x1410, 0x67)] {
        let g = t.img.word(t.at(rva) + op);
        t.set_u32(g, 0x00a5_0000 + op);
    }
    for (on, bits) in [
        (1u32, 0b0_0000u32),
        (0, 0b1_0000),
        (1, 0b1_0001),
        (1, 0b1_0010),
        (1, 0b1_0011),
    ] {
        t.set_u32(cell, bits);
        t.check_entry(t.at(0x1070), Call::default(), &[], Eax::Checked);
        t.set_u32(t.img.word(at + 0x16), on);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
