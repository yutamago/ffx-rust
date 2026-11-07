//! `magic_0505.dll` against its Rust port (`ffx_magic::m0505`). Needs
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

use ffx_magic::{Ret, m0505};
use magic::{Call, Eax, SCRATCH};
use magic_hi::f;
use magic_owna::{rec, recs, twin};

const N: u16 = 505;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The table entries: the two starts, the draw, the registration (0, 1
/// and 3 records).
#[test]
fn entries() {
    let Some(mut t) = twin(N) else { return };
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let d = SCRATCH + 0x100;
    t.set_u32(0x1004_ac78, d);
    for i in 0..8 {
        t.set_u32(d + 4 * i, 0x00b0_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, a| table ^ (a[0] & 0xf0));
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 + a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    recs(
        &mut t,
        &[
            (714, 5),
            (723, 1),
            (724, 2),
            (258, 1),
            (231, 3),
            (230, 2),
            (712, 2),
            (779, 4),
            (804, 1),
        ],
    );
    for i in 0..4u32 {
        t.set_u32(0x1004_70a0 + 4 * i, f(i as f32 * 3.5 - 1.0));
    }
    for rva in [0x1180u32, 0x1380, 0x1210] {
        t.check_entry(t.at(rva), Call::default(), &[], Eax::Checked);
    }
    let reg = t.at(0x18d0);
    let recs_at = SCRATCH + 0x1000;
    t.set_u32(t.img.word(reg + 0xa), recs_at);
    for i in 0..3 {
        t.set_u32(recs_at + 0x30 + 0x94 * i, 0x00c0_0000 + i);
    }
    // slot 779 keeps the game flag it runs under; the word slot 804 gets
    // distinct
    let lock = magic_h6::cell(&t, reg, 0x43);
    t.set_u32(lock, 0x00a1_0000);
    magic_h6::rec_cells(&mut t, 779, 4, &[lock], |_, a| {
        0x5a5a_0779 ^ a.first().copied().unwrap_or(0)
    });
    magic_h6::distinct(&mut t, reg, &[0x7e]);
    for n in [0u32, 1, 3] {
        t.set_u32(t.img.word(reg + 0x14), n);
        t.check_entry(reg, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10002af0: the pool cleared (dirty before).
#[test]
fn clear() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2af0);
    let first = t.img.word(at + 0x10);
    t.write(first, &vec![0x3d; 0x94 * 0x64]);
    let head = t.img.word(at + 0xb);
    t.write(head, &[0x3e; 0x5c]);
    t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0505::clear(g, img, fr.at))
    });
}

/// 0x10001a20: the frame off (slot 253 0, the flag clear) and on with
/// slot 261's answers (the timelines live, so running or stopping one, or
/// the wrong one, shows).
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1a20);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 253, 0, move |m, _| (m.r32(cell) >> 4) & 1);
    // slot 261 (k) answers bit k of the cell
    rec(&mut t, 261, 1, move |m, a| (m.r32(cell) >> (a[0] & 3)) & 1);
    recs(&mut t, &[(718, 1), (222, 4), (378, 1), (214, 1)]);
    // the copied words distinct, the copy's target in scratch
    t.set_u32(t.img.word(at + 0x1f), SCRATCH + 0x30);
    t.set_u32(t.img.word(at + 0x24), 0x00a2_0024);
    t.set_u32(t.img.word(at + 0x2b), 0x00a2_002b);
    t.set_u32(t.img.word(at + 0xf2), 0x00a2_00f2);
    for (on, bits) in [
        (1u32, 0b0_0000u32),
        (0, 0b1_0000),
        (1, 0b1_0001),
        (1, 0b1_0010),
        (1, 0b1_0111),
    ] {
        t.set_u32(t.img.word(at + 0x16), on);
        t.set_u32(cell, bits);
        for (k, op) in (0u32..).zip([0x4b, 0x6b, 0x8b, 0x95]) {
            let tl = t.img.word(at + op);
            magic_h6::live_timeline(&mut t, tl, SCRATCH + 0x600 + 0x20 * k, 0x40 + k);
        }
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001580: the data, the block and the objects cleared.
#[test]
fn state_start() {
    let Some(mut t) = twin(505) else { return };
    let at = t.at(0x1580);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    t.set_u32(t.img.word(at + 0x11), 0x0011_0001);
    t.write(t.img.word(at + 0x1c), &[0x5c; 0x30]);
    for op in [0x4c, 0x56, 0x60, 0x6a] {
        t.set_u32(t.img.word(at + op), 0x00ab_0000 + op);
    }
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}

/// 0x10001690 (`r13_start::set_up_0505`): the set-up with its slots
/// answering distinct words, the lists' handle a block of words, the
/// cleared blocks dirty before.
#[test]
fn set_up() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1690);
    let w = |t: &magic::Twin, o: u32| t.img.word(at + o);
    let h = SCRATCH + 0x700;
    for i in 0..16 {
        t.set_u32(h + 4 * i, 0x00d0_0000 + i);
    }
    rec(&mut t, 803, 2, |_, a| 0x0080_0000 ^ a[1]);
    rec(&mut t, 719, 3, move |_, a| {
        if a[1] == 8 { h } else { 0x0719_0000 ^ a[1] }
    });
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    rec(&mut t, 260, 1, |_, a| 0x0260_0000 ^ a[0]);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(w(&t, 0x8d), lock);
    t.set_u32(lock, 0x00a1_0000);
    rec(&mut t, 711, 3, move |m, a| {
        m.w32(SCRATCH + 0x7f4, m.r32(lock));
        a[1]
    });
    recs(&mut t, &[(718, 1), (222, 4), (710, 4)]);
    for (o, n) in [(0x131u32, 0x110usize), (0x147, 0x110), (0x1da, 0x30)] {
        let b = w(&t, o);
        t.write(b, &vec![0x3c; n]);
    }
    let first = t.img.word(t.img.rel32_target(at + 0x202) + 0x10);
    t.write(first, &vec![0x3d; 0x94 * 0x64]);
    t.check_entry(at, Call::default(), &[0x0a00_0505], Eax::Checked);
}

/// 0x10001600 (`r13_start::sparks_frame_0505`): two free spark records
/// of six asked for (the rest find none), no sprites to step or draw.
#[test]
fn sparks_frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1600);
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4u32 {
            m.w32(a[3] + 4 * i, f(1.5 + i as f32 * 2.25));
        }
        0x237
    });
    let k = SCRATCH + 0x40;
    rec(&mut t, 324, 0, move |m, _| {
        let n = m.r32(k);
        m.w32(k, n + 1);
        n.wrapping_mul(0x9e37_79b9) >> 7
    });
    rec(&mut t, 403, 1, |_, a| a[0]);
    t.stub_st0(403, |_, a| f64::from(f32::from_bits(a[0])) * 0.5);
    recs(&mut t, &[(714, 5), (779, 2)]);
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
    let spark = t.img.rel32_target(at + 0x30);
    let take = t.img.rel32_target(spark + 0x0c);
    let recs_at = SCRATCH + 0x1000;
    t.set_u32(t.img.word(take + 0x2), 2);
    t.set_u32(t.img.word(take + 0xb), recs_at);
    t.write(recs_at, &vec![0x2a; 0x94 * 2]);
    t.set_u32(recs_at, 0);
    t.set_u32(recs_at + 0x94, 0);
    t.set_u32(t.img.word(at + 0x25), 0x00c0_0025);
    let frames = t.img.rel32_target(at + 0x6d);
    t.set_u32(t.img.word(frames + 0x1b), 0x00c0_001b);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}
