//! `magic_0489.dll` against its Rust port (`ffx_magic::m0489`). Needs
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

use magic::{Call, Eax, SCRATCH, Twin};
use magic_hi::f;
use magic_owna::{rec, recs, twin};

const N: u16 = 489;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The object callbacks' stubs: 941 a program table (`G_O` set), 712
/// from `cell` (keeping the part pointer's cell and the game flag), 237
/// keeps the point's words as it finds them, then fills it.
fn stubs(t: &mut Twin, cell: u32) {
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(0x1007_a79c, o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(t, 941, 1, move |_, _| table);
    let e1980 = t.at(0x1980);
    let (gp, lock) = (
        magic_h6::cell(t, e1980, 0x8d),
        magic_h6::cell(t, e1980, 0x9a),
    );
    magic_h6::rec_cells(t, 712, 2, &[gp, lock], move |m, _| m.r32(cell));
    rec(t, 237, 4, |m, a| {
        magic_owna::dump(m, 237, a[3], 4);
        for i in 0..4 {
            m.w32(a[3] + 4 * i, f(i as f32 * 1.5 + 0.25));
        }
        0x237
    });
    recs(t, &[(714, 5), (258, 1), (231, 3), (230, 2)]);
    // the words the callbacks pass to slot 237, distinct
    for (rva, ops) in [
        (0x1980u32, &[0x35u32, 0x3d][..]),
        (0x1a80, &[0x1d]),
        (0x1c20, &[0xb4, 0xc6]),
        (0x1d00, &[0x67]),
    ] {
        let at = t.at(rva);
        magic_h6::distinct(t, at, ops);
    }
}

/// The object callbacks on records of each array (first, second, other
/// indices; each record filled with a pattern first, so a store left out
/// or moved shows), slot 712 saying 0 and 1, the flag clear and set.
#[test]
fn object_callbacks() {
    let Some(mut t) = twin(N) else { return };
    let cell = SCRATCH + 0x10;
    stubs(&mut t, cell);
    let flag = SCRATCH + 0x20;
    let e1980 = t.at(0x1980);
    t.set_u32(t.img.word(e1980 + 0x4), flag);
    // the records each callback is given (inside their arrays: further
    // records would lie on the DLL's other globals)
    let cases: [(u32, u32, &[u32]); 4] = [
        (0x1980, 0x15, &[0, 1, 2]),
        (0x1a80, 0x26, &[0, 1]),
        (0x1c20, 0x8f, &[0, 1]),
        (0x1d00, 0xb9, &[0, 1, 2, 3]),
    ];
    let gp = magic_h6::cell(&t, e1980, 0x8d);
    for (rva, base_op, recs) in cases {
        let at = t.at(rva);
        let base = t.img.word(at + base_op);
        for (k, &rec) in recs.iter().enumerate() {
            let obj = base + 0x80 * rec;
            t.write(obj, &[0x5b; 0x80]);
            t.set_u32(gp, 0x7777_7777);
            t.set_u32(cell, k as u32 & 1);
            t.set_u32(flag, (k as u32 >> 1) & 1);
            t.check_entry(at, Call::default(), &[obj], Eax::Checked);
        }
    }
}

/// 0x10001e20 then 0x10002240: the start-up, then the frame off and on
/// with slot 261's answers and the trail clock at 0, below 170 and
/// above (the five timelines live, so running, stopping or skipping one
/// shows; slot 237 gives each point its own words and may move the clock,
/// which the frame reads again after the fill).
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
    let at = t.at(0x2240);
    let clock = t.img.word(at + 0x8f);
    // the clock slot 237 leaves (0: as it was)
    let new_clock = SCRATCH + 0x30;
    rec(&mut t, 237, 4, move |m, a| {
        let k = m.r32(magic_owna::row(237));
        for i in 0..4 {
            m.w32(a[3] + 4 * i, f(i as f32 * 1.5 + 0.25 + k as f32 * 0.125));
        }
        let c = m.r32(new_clock);
        if c != 0 {
            m.w32(clock, c);
        }
        0x237
    });
    // slot 711 keeps the game flag
    let su = t.at(0x1e20);
    let lock = magic_h6::cell(&t, su, 0xa6);
    magic_h6::rec_cells(&mut t, 711, 3, &[lock], |_, a| {
        0x5a5a_0000 ^ a.first().copied().unwrap_or(0)
    });
    rec(&mut t, 253, 0, |_, _| 1);
    recs(
        &mut t,
        &[(718, 3), (222, 4), (710, 4), (214, 1), (723, 1), (724, 2)],
    );
    t.set_u32(lock, 0x00a1_0000);
    for (op, n) in [
        (0x147u32, 0x30usize),
        (0x168, 0x110),
        (0x179, 0x110),
        (0x1d0, 0x110),
        (0x213, 0x110),
        (0x263, 0x110),
    ] {
        let blk = t.img.word(su + op);
        t.write(blk, &vec![0x5e; n]);
    }
    t.check_entry(su, Call::default(), &[0x00a6_0000], Eax::Checked);
    for i in 0..0x28u32 {
        t.set_u32(0x1007_a260 + 4 * i, f(i as f32 * 0.5 - 4.0));
    }
    let cell = SCRATCH + 0x10;
    // slot 261 (k) answers bit k of the cell
    rec(&mut t, 261, 1, move |m, a| (m.r32(cell) >> (a[0] & 7)) & 1);
    for (on, bits, c, nc) in [
        (0u32, 0u32, 0u32, 0u32),
        (1, 0, 0, 0),
        (1, 0b00_0011, 0, 0),
        (1, 0b01_1110, 0xa9, 0),
        (1, 0b00_0010, 0xaa, 0),
        (1, 0b00_0010, 0, 0xb0),
        (1, 0b00_0010, 0xa9, 0),
        (1, 0b10_0000, 0xab, 0),
        (1, 0b00_0001, 0xab, 0),
        (1, 0b00_0100, 0xab, 0),
        (1, 0b00_1000, 0xab, 0),
        (1, 0b01_0000, 0xab, 0),
        (1, 0b11_1111, 5, 0),
    ] {
        for (k, op) in (0u32..).zip([0x6b, 0x2a7, 0x2c7, 0x2e7, 0x307]) {
            let tl = t.img.word(at + op);
            magic_h6::live_timeline(&mut t, tl, SCRATCH + 0x600 + 0x20 * k, 0x40 + k);
        }
        t.set_u32(new_clock, nc);
        t.set_u32(t.img.word(at + 0x2), on);
        t.set_u32(cell, bits);
        t.set_u32(clock, c);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
