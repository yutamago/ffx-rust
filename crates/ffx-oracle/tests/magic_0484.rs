//! `magic_0484.dll` against its Rust port (`ffx_magic::m0484`). Needs
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
use magic_hi::{f, konst};
use magic_owna::{rec, recs, twin};

const N: u16 = 484;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The object callbacks' stubs: 941 a program table, 712 from `cell`,
/// 237 fills its point, the rest recorded.
fn stubs(t: &mut Twin, cell: u32) {
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(t.img.word(t.at(0x1000) + 0x10), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(t, 941, 1, move |_, _| table);
    rec(t, 712, 2, move |m, _| m.r32(cell));
    rec(t, 237, 4, |m, a| {
        for i in 0..4 {
            m.w32(a[3] + 4 * i, f(i as f32 * 0.75 + 2.0));
        }
        0x237
    });
    recs(
        t,
        &[
            (714, 5),
            (400, 2),
            (401, 9),
            (398, 2),
            (325, 2),
            (258, 1),
            (231, 3),
            (230, 2),
            (713, 1),
            (391, 4),
        ],
    );
}

/// 0x10001000, 0x10001360: the starts; 0x100013f0: the frame with slot
/// 712 saying 0 and 1, the count above 6 and below.
#[test]
fn object_callbacks() {
    let Some(mut t) = twin(N) else { return };
    let cell = SCRATCH + 0x10;
    stubs(&mut t, cell);
    let obj = SCRATCH + 0x1000;
    for rva in [0x1000u32, 0x1360] {
        t.write(obj, &[0x5b; 0x80]);
        t.check_entry(t.at(rva), Call::default(), &[obj], Eax::Checked);
    }
    let at = t.at(0x13f0);
    let n = t.img.word(at + 0x10a);
    for (answer, count) in [(0u32, 20u32), (1, 3), (1, 6), (0, 0x8000_0003)] {
        t.set_u32(cell, answer);
        t.set_u32(n, count);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x10001100: before frame 0x9c (the size capped and not) and after,
/// slot 712 saying 0 and 1.
#[test]
fn object_frame() {
    let Some(mut t) = twin(N) else { return };
    let cell = SCRATCH + 0x10;
    stubs(&mut t, cell);
    let at = t.at(0x1100);
    let (k, limit) = (konst(&t, at, 0xbd), konst(&t, at, 0xc7));
    let obj = SCRATCH + 0x1000;
    let w = |t: &Twin, op: u32| t.img.word(at + op);
    for (frame, size, answer) in [
        (0x9bu32, limit - k + 0.5, 0u32),
        (3, limit - k - 0.5, 1),
        (0xffff_fff0, 0.0, 0),
        (0x9c, 0.0, 1),
        (0x200, 0.0, 0),
    ] {
        t.write(obj, &[0x5b; 0x80]);
        for i in 0..4 {
            t.set_u32(obj + 0x40 + 4 * i, f(1.5 * i as f32 - 1.0));
        }
        t.set_u32(w(&t, 0x12), frame);
        t.set_u32(w(&t, 0x62), f(size));
        t.set_u32(w(&t, 0xe5), f(0.375));
        t.set_u32(cell, answer);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x10001520 and 0x10001790: the start-up, then the frame off, with
/// slot 261 saying 0, with it saying 1 then 0, and 1 throughout (the
/// timelines idle).
#[test]
fn start_up_and_frame() {
    let Some(mut t) = twin(N) else { return };
    let d = SCRATCH + 0x400;
    for i in 0..8 {
        t.set_u32(d + 4 * i, 0x0d00_0000 + i);
    }
    rec(&mut t, 803, 2, |_, _| 0x2100_0000);
    rec(&mut t, 719, 3, move |_, _| d);
    rec(&mut t, 941, 3, |_, a| 0x0941_0000 ^ a[0]);
    rec(&mut t, 260, 1, |_, a| 0x0260_0000 ^ a[0]);
    let cell = SCRATCH + 0x10;
    // slot 261 (k) answers bit k of the cell
    rec(&mut t, 261, 1, move |m, a| (m.r32(cell) >> (a[0] & 3)) & 1);
    recs(
        &mut t,
        &[
            (718, 3),
            (222, 4),
            (711, 3),
            (710, 4),
            (399, 2),
            (723, 1),
            (724, 2),
            (214, 3),
        ],
    );
    let su = t.at(0x1520);
    let blk = t.img.word(su + 0x127);
    t.write(blk, &[0x5e; 0x110]);
    t.check_entry(su, Call::default(), &[0x00a6_0000], Eax::Checked);
    let at = t.at(0x1790);
    let ftl = t.img.rel32_target(at + 0x61);
    let fst = t.img.rel32_target(at + 0x8b);
    t.set_u32(t.img.word(ftl + 0x06), 0);
    t.set_u32(t.img.word(fst + 0x02), 0);
    for (on, bits) in [(0u32, 0u32), (1, 0), (1, 0b001), (1, 0b111)] {
        t.set_u32(t.img.word(at + 0x2), on);
        t.set_u32(cell, bits);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
