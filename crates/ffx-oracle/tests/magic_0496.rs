//! `magic_0496.dll` against its Rust port (`ffx_magic::m0496`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_hi::f;
use magic_owna::{rec, recs, twin};

const N: u16 = 496;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The object starts on records 0..5 of their arrays.
#[test]
fn object_starts() {
    let Some(mut t) = twin(N) else { return };
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(0x1005_86b0, o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 324, 0, move |m, _| m.r32(cell));
    recs(&mut t, &[(714, 5)]);
    for (rva, base_op, pts) in [
        (0x1000u32, 0x2au32, 0x8cu32),
        (0x11a0, 0x2a, 0x59),
        (0x1310, 0x3e, 0x4d),
        (0x13c0, 0x3e, 0x4d),
    ] {
        let at = t.at(rva);
        let base = t.img.word(at + base_op);
        let p = t.img.word(at + pts);
        for i in 0..0x18u32 {
            t.set_u32(p + 4 * i, f(i as f32 * 0.75 - 2.0));
        }
        for (k, r) in [0u32, 1, 4, 5].into_iter().enumerate() {
            let obj = base + 0x80 * r;
            t.write(obj, &[0x5b; 0x80]);
            t.set_u32(cell, 0x1234_5678 * (k as u32 + 1));
            t.check_entry(at, Call::default(), &[obj], Eax::Checked);
        }
    }
}

/// 0x100014b0 then 0x10001800: the start-up, then the frame off and on
/// with slot 261's answers (the timelines live, so running or stopping
/// one, or the wrong one, shows).
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
    recs(
        &mut t,
        &[(718, 3), (222, 4), (710, 4), (214, 1), (723, 1), (724, 2)],
    );
    let su = t.at(0x14b0);
    // slot 711 keeps the game flag it runs under
    let lock = magic_h6::cell(&t, su, 0x83);
    t.set_u32(lock, 0x00a1_0000);
    magic_h6::rec_cells(&mut t, 711, 3, &[lock], |_, a| {
        0x5a5a_0711 ^ a.first().copied().unwrap_or(0)
    });
    for op in [0x127, 0x178, 0x189] {
        let blk = t.img.word(su + op);
        t.write(blk, &[0x5e; 0x110]);
    }
    t.check_entry(su, Call::default(), &[0x00a6_0000], Eax::Checked);
    let at = t.at(0x1800);
    rec(&mut t, 253, 0, |_, _| 1);
    let cell = SCRATCH + 0x10;
    // slot 261 (k) answers bit k of the cell
    rec(&mut t, 261, 1, move |m, a| (m.r32(cell) >> (a[0] & 3)) & 1);
    for (on, bits) in [(0u32, 0u32), (1, 0), (1, 0b01), (1, 0b10), (1, 0b11)] {
        t.set_u32(t.img.word(at + 0x2), on);
        t.set_u32(cell, bits);
        for (k, op) in (0u32..).zip([0x61, 0x6b, 0x75, 0x95, 0x9f, 0xa9]) {
            let tl = t.img.word(at + op);
            magic_h6::live_timeline(&mut t, tl, SCRATCH + 0x600 + 0x20 * k, 0x40 + k);
        }
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
