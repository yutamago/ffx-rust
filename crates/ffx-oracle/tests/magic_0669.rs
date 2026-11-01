//! `magic_0669.dll` against its Rust port (`ffx_magic::m0669`). Needs
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
use magic_hi::{f, konst};
use magic_owna::{rec, recs, twin};

const N: u16 = 669;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001030: the level below, inside, above its range and a
/// signalling NaN; `*G_M` at `K` and not; the scale around `E`.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1030);
    recs(&mut t, &[(270, 5)]);
    let (h, k, d, e) = (
        konst(&t, at, 0x18),
        konst(&t, at, 0x71),
        konst(&t, at, 0x87),
        konst(&t, at, 0x98),
    );
    let level = t.img.word(at + 0x4);
    let m = t.img.word(at + 0x67);
    let s = t.img.word(at + 0x79);
    for (x, mk, sc) in [
        (f(-1.0), true, e - d),
        (f(h * 0.5), true, e),
        (f(h + 1.0), false, e - 1.0),
        (0x7f80_0001u32, false, e + 1.0),
        (f(h), true, f32::NAN),
        (f(0.0), false, e),
    ] {
        t.set_u32(level, x);
        t.set_u32(m, f(if mk { k } else { k + 1.0 }));
        t.set_u32(s, f(sc));
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001c30: slot 712 saying 0 and 1.
#[test]
fn draw() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1c30);
    let lock = magic::SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x78), lock);
    t.set_u32(lock, 0x00a1_0000);
    t.set_u32(t.img.word(at + 0x6b), magic::SCRATCH + 0x24);
    for (i, op) in (0u32..).zip([0x34u32, 0x3f, 0x4b, 0x53, 0x5e]) {
        t.set_u32(t.img.word(at + op), 0x00c0_0000 + i);
    }
    let cell = magic::SCRATCH + 0x10;
    magic_owna::rec(&mut t, 712, 2, move |m, _| m.r32(cell));
    recs(&mut t, &[(258, 1), (231, 4), (230, 6)]);
    let obj = magic::SCRATCH + 0x1000;
    for answer in [0u32, 1] {
        t.set_u32(cell, answer);
        t.write(obj, &[0x5b; 0x80]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x10001a90: slot 712 saying 0 and 1.
#[test]
fn draw_restart() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1a90);
    let table = magic::SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = magic::SCRATCH + 0x100;
    t.set_u32(t.img.word(at + 0xc6), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    let lock = magic::SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x78), lock);
    t.set_u32(lock, 0x00a1_0000);
    t.set_u32(t.img.word(at + 0x6b), magic::SCRATCH + 0x24);
    for (i, op) in (0u32..).zip([0x34u32, 0x3f, 0x4b, 0x53, 0x5e]) {
        t.set_u32(t.img.word(at + op), 0x00c0_0000 + i);
    }
    magic_owna::rec(&mut t, 941, 1, move |_, _| table);
    let cell = magic::SCRATCH + 0x10;
    magic_owna::rec(&mut t, 712, 2, move |m, _| m.r32(cell));
    recs(&mut t, &[(714, 5), (258, 1), (231, 4), (230, 6), (713, 1)]);
    let obj = magic::SCRATCH + 0x1000;
    for answer in [0u32, 1] {
        t.set_u32(cell, answer);
        t.write(obj, &[0x5b; 0x80]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x100018b0: the frame at a point with slot 712 saying 0 and 1; the
/// globals distinct, slot 712 keeping the game flag and the part cell.
#[test]
fn frame_at() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x18b0);
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(t.img.word(at + 0xdc), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    let (gp, lock) = (SCRATCH + 0x30, SCRATCH + 0x20);
    t.set_u32(t.img.word(at + 0x81), gp);
    t.set_u32(t.img.word(at + 0x8e), lock);
    t.set_u32(t.img.word(at + 0xaa), lock);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 712, 2, move |m, _| {
        let l = m.r32(lock);
        let p = m.r32(gp);
        m.w32(SCRATCH + 0x2_0400, l);
        m.w32(SCRATCH + 0x2_0404, p);
        m.r32(cell) & 1
    });
    recs(
        &mut t,
        &[(325, 2), (258, 1), (231, 3), (230, 2), (713, 1), (714, 5)],
    );
    for op in [0x4au32, 0x55, 0x61, 0x69, 0x74] {
        let g = t.img.word(at + op);
        t.set_u32(g, 0x00a5_0000 + op);
    }
    let obj = SCRATCH + 0x1000;
    for bits in [0u32, 1] {
        t.set_u32(cell, bits);
        t.set_u32(gp, 0x7777);
        t.set_u32(lock, 0x6666);
        t.write(obj, &[0x5b; 0x80]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}
