//! `magic_0486.dll` against its Rust port (`ffx_magic::m0486`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_hi::f;
use magic_owna::{rec, recs, twin};

const N: u16 = 486;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001070, 0x10001250, 0x100013f0: the three draws.
#[test]
fn draws() {
    let Some(mut t) = twin(N) else { return };
    for i in 0..0x180u32 {
        t.set_u32(0x1003_f760 + 4 * i, f(0.75 * i as f32 - 3.0));
    }
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            m.w32(a[3] + 4 * i, f(i as f32 * 1.5 + 0.25));
        }
        0x237
    });
    recs(
        &mut t,
        &[(723, 1), (724, 2), (258, 1), (231, 3), (230, 2), (712, 2)],
    );
    for rva in [0x1070u32, 0x1250, 0x13f0] {
        t.check_entry(t.at(rva), Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001500 then 0x10001790: the start-up, then the frame off (slot
/// 253 0, the flag clear) and on with slot 261's answers (the timelines
/// idle).
#[test]
fn start_up_and_frame() {
    let Some(mut t) = twin(N) else { return };
    let d = SCRATCH + 0x400;
    for i in 0..8 {
        t.set_u32(d + 4 * i, 0x0d00_0000 + i);
    }
    rec(&mut t, 803, 2, |_, _| 0x2100_0000);
    rec(&mut t, 719, 3, move |_, _| d);
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    recs(
        &mut t,
        &[(718, 3), (222, 4), (711, 3), (710, 4), (413, 1), (214, 1)],
    );
    let su = t.at(0x1500);
    for op in [0x131, 0x147] {
        let blk = t.img.word(su + op);
        t.write(blk, &[0x5e; 0x110]);
    }
    t.check_entry(su, Call::default(), &[0x00a6_0000], Eax::Checked);
    let at = t.at(0x1790);
    for op in [0x4b, 0x6b] {
        let tl = t.img.word(at + op);
        t.set_u32(tl + 8, 0);
    }
    let cell = SCRATCH + 0x10;
    rec(&mut t, 253, 0, move |m, _| m.r32(cell) & 1);
    // slot 261 (k) answers bit k of the cell
    rec(&mut t, 261, 1, move |m, a| (m.r32(cell) >> (a[0] & 3)) & 1);
    for (on, bits) in [(1u32, 0u32), (0, 1), (1, 0b0001), (1, 0b0011), (1, 0b1101)] {
        t.set_u32(t.img.word(at + 0x16), on);
        t.set_u32(cell, bits);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
