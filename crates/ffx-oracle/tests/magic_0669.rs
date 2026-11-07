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
mod magic_h6;
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
        // -0.0: not below 0, so it stays -0.0
        (f(-0.0), false, e),
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
    // slots 712 and 713 keep the part cell and the game flag; 712 moves
    // the part cell on (as the game's part code may)
    let gp = magic::SCRATCH + 0x24;
    magic_h6::rec_cells(&mut t, 712, 2, &[gp, lock], move |m, _| {
        m.w32(gp, 0x6666_0712);
        m.r32(cell)
    });
    magic_h6::rec_cells(&mut t, 713, 1, &[gp, lock], |_, a| {
        0x5a5a_0713 ^ a.first().copied().unwrap_or(0)
    });
    recs(&mut t, &[(714, 5), (258, 1), (231, 4), (230, 6)]);
    let obj = magic::SCRATCH + 0x1000;
    for answer in [0u32, 1] {
        t.set_u32(cell, answer);
        t.set_u32(gp, 0x7777_7777);
        t.set_u32(lock, 0x00a1_0000);
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
        // the part cell moved on, as the game's part code may
        m.w32(gp, 0x6666_0712);
        m.r32(cell) & 1
    });
    magic_h6::rec_cells(&mut t, 713, 1, &[gp, lock], |_, a| {
        0x5a5a_0713 ^ a.first().copied().unwrap_or(0)
    });
    recs(&mut t, &[(325, 2), (258, 1), (231, 3), (230, 2), (714, 5)]);
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

/// 0x10001640 (`r13_start::set_up_tail_0669`): the slots answering
/// distinct words, the cleared blocks dirty before.
#[test]
fn set_up_tail() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1640);
    t.set_u32(t.img.word(at + 0x11), 0x00c0_0011);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0] ^ (a[2] << 8));
    t.write(t.img.word(at + 0x38), &[0x3c; 0x30]);
    let clear = t.img.rel32_target(at + 0x5f);
    let recs_at = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(clear + 0x7), recs_at);
    t.write(t.img.word(clear + 0x11), &[0x3e; 0x5c]);
    t.write(recs_at, &vec![0x3d; 0x94 * 500]);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}

/// 0x100016d0 (`r13_start::sparks_frame_0669`): the counts below, inside
/// and above the range and NaN; four free records (the rest find none),
/// live records counted (none, some), no sprites to step or draw.
#[test]
fn sparks_frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x16d0);
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4u32 {
            m.w32(a[0] + 4 * i, f(1.5 + i as f32 * 2.25));
        }
        0x325
    });
    let k = SCRATCH + 0x40;
    rec(&mut t, 324, 0, move |m, _| {
        let n = m.r32(k);
        m.w32(k, n + 1);
        n.wrapping_mul(0x9e37_79b9) >> 7
    });
    rec(&mut t, 403, 1, |_, a| a[0]);
    t.stub_st0(403, |_, a| f64::from(f32::from_bits(a[0])) * 0.5);
    recs(&mut t, &[(714, 5), (382, 2), (779, 2)]);
    // the sprites stepped and drawn (the pool's count is the system's):
    // their slots answer distinct words, slot 941 a table
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, a| table ^ (a[0] & 0xf0));
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    for (s, k) in [
        (325u32, 2usize),
        (229, 3),
        (773, 4),
        (774, 5),
        (775, 2),
        (776, 1),
        (777, 1),
        (778, 6),
        (942, 1),
    ] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    let (lo, hi) = (konst(&t, at, 0x2a), konst(&t, at, 0x3a));
    let (ga, gb) = (t.img.word(at + 0x32), t.img.word(at + 0x5f));
    let spark = t.img.rel32_target(at + 0x95);
    let take = t.img.rel32_target(spark + 0x0c);
    let recs_at = SCRATCH + 0x1000;
    t.set_u32(t.img.word(take + 0x2), 8);
    t.set_u32(t.img.word(take + 0xb), recs_at);
    let short = t.img.rel32_target(at + 0xc5);
    let take2 = t.img.rel32_target(short + 0x0c);
    t.set_u32(t.img.word(take2 + 0x2), 8);
    t.set_u32(t.img.word(take2 + 0xb), recs_at);
    let frames = t.img.rel32_target(at + 0x109);
    t.set_u32(t.img.word(frames + 0x1b), 0x00c0_001b);
    let live = SCRATCH + 0x3000;
    t.set_u32(t.img.word(at + 0xe5), live);
    for (a, b, n) in [
        (lo - 1.0, 2.0, 0i32),
        (lo, 3.0, 2),
        (3.0, hi + 7.0, 3),
        (f32::NAN, 1.0, -1),
        (2.0, f32::NAN, 5),
        (hi + 0.5, lo - 0.5, 1),
    ] {
        t.write(recs_at, &vec![0x2a; 0x94 * 8]);
        for i in 0..8 {
            t.set_u32(recs_at + 0x94 * i, 0);
        }
        t.set_u32(ga, f(a));
        t.set_u32(gb, f(b));
        t.set_u32(t.img.word(at + 0xe0), n.cast_unsigned());
        for i in 0..6u32 {
            t.set_u32(live + 0x94 * i, i % 2);
        }
        t.set_u32(k, 0);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
