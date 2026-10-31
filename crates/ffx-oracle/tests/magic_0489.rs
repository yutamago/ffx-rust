//! `magic_0489.dll` against its Rust port (`ffx_magic::m0489`). Needs
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

const N: u16 = 489;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The object callbacks' stubs: 941 a program table (`G_O` set), 712
/// from `cell`, 237 fills its point.
fn stubs(t: &mut Twin, cell: u32) {
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(0x1007_a79c, o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(t, 941, 1, move |_, _| table);
    rec(t, 712, 2, move |m, _| m.r32(cell));
    rec(t, 237, 4, |m, a| {
        for i in 0..4 {
            m.w32(a[3] + 4 * i, f(i as f32 * 1.5 + 0.25));
        }
        0x237
    });
    recs(t, &[(714, 5), (258, 1), (231, 3), (230, 2)]);
}

/// The object callbacks on records of each array (first, second, other
/// indices), slot 712 saying 0 and 1, the flag clear and set.
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
    for (rva, base_op, recs) in cases {
        let at = t.at(rva);
        let base = t.img.word(at + base_op);
        for (k, &rec) in recs.iter().enumerate() {
            let obj = base + 0x80 * rec;
            t.set_u32(cell, k as u32 & 1);
            t.set_u32(flag, (k as u32 >> 1) & 1);
            t.check_entry(at, Call::default(), &[obj], Eax::Checked);
        }
    }
}

/// 0x10001e20 then 0x10002240: the start-up, then the frame off and on
/// with slot 261's answers and the trail clock at 0, below 170 and
/// above (the timelines idle).
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
    let su = t.at(0x1e20);
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
    let at = t.at(0x2240);
    for op in [0x6b, 0x2a7, 0x2c7, 0x2e7, 0x307] {
        let tl = t.img.word(at + op);
        t.set_u32(tl + 8, 0);
    }
    let clock = t.img.word(at + 0x8f);
    for i in 0..0x28u32 {
        t.set_u32(0x1007_a260 + 4 * i, f(i as f32 * 0.5 - 4.0));
    }
    let cell = SCRATCH + 0x10;
    // slot 261 (k) answers bit k of the cell
    rec(&mut t, 261, 1, move |m, a| (m.r32(cell) >> (a[0] & 7)) & 1);
    for (on, bits, c) in [
        (0u32, 0u32, 0u32),
        (1, 0, 0),
        (1, 0b00_0011, 0),
        (1, 0b01_1110, 0xa9),
        (1, 0b00_0010, 0xaa),
        (1, 0b11_1111, 5),
    ] {
        t.set_u32(t.img.word(at + 0x2), on);
        t.set_u32(cell, bits);
        t.set_u32(clock, c);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
