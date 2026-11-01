//! `magic_0487.dll` against its Rust port (`ffx_magic::m0487`). Needs
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

use magic::{Call, Eax, SCRATCH, Twin};
use magic_hi::f;
use magic_owna::{rec, recs, twin};

const N: u16 = 487;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The part stubs: 941 a program table (`G_O` set), 712 from `cell`.
fn stubs(t: &mut Twin, cell: u32) {
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(0x1003_d588, o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(t, 941, 1, move |_, _| table);
    rec(t, 712, 2, move |m, _| m.r32(cell));
    recs(t, &[(714, 5), (258, 1), (231, 3), (230, 2), (713, 1)]);
}

/// 0x10001170: a direction to normalize, a zero one, a NaN one; then
/// 0x100012b0 with slot 712 saying 0 and 1.
#[test]
fn start_and_step() {
    let Some(mut t) = twin(N) else { return };
    let cell = SCRATCH + 0x10;
    stubs(&mut t, cell);
    let start = t.at(0x1170);
    let step = t.at(0x12b0);
    for (k, d) in [
        [3.0f32, 4.0, 12.0],
        [0.0, -0.0, 0.0],
        [f32::NAN, 1.0, 2.0],
        [1.0e-20, 0.0, 0.0],
    ]
    .into_iter()
    .enumerate()
    {
        for (i, v) in (0u32..).zip(d) {
            t.set_u32(0x1003_cd28 + 4 * i, f(v));
        }
        t.check_entry(start, Call::default(), &[], Eax::Checked);
        t.set_u32(cell, k as u32 & 1);
        t.check_entry(step, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001490 then 0x10001730: the start-up, then the frame off and on
/// with slot 261's answers (the timelines idle).
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
    rec(&mut t, 260, 1, |_, a| 0x0260_0000 ^ a[0]);
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            m.w32(a[3] + 4 * i, f(i as f32 * 1.5 + 0.25));
        }
        0x237
    });
    recs(
        &mut t,
        &[
            (718, 3),
            (222, 4),
            (711, 3),
            (710, 4),
            (214, 1),
            (723, 1),
            (724, 2),
        ],
    );
    let su = t.at(0x1490);
    for op in [0x127, 0x13d] {
        let blk = t.img.word(su + op);
        t.write(blk, &[0x5e; 0x110]);
    }
    t.check_entry(su, Call::default(), &[0x00a6_0000], Eax::Checked);
    let at = t.at(0x1730);
    for op in [0x22d, 0x24d] {
        let tl = t.img.word(at + op);
        t.set_u32(tl + 8, 0);
    }
    for i in 0..0x24u32 {
        t.set_u32(0x1003_d240 + 4 * i, f(i as f32 * 0.5 - 4.0));
    }
    let cell = SCRATCH + 0x10;
    // slot 261 (k) answers bit k of the cell
    rec(&mut t, 261, 1, move |m, a| (m.r32(cell) >> (a[0] & 3)) & 1);
    for (on, bits) in [(0u32, 0u32), (1, 0), (1, 0b001), (1, 0b011), (1, 0b110)] {
        t.set_u32(t.img.word(at + 0x2), on);
        t.set_u32(cell, bits);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
