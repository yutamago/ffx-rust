//! `magic_0641.dll` against its Rust port (`ffx_magic::m0641`). Needs
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

use ffx_magic::{Ret, m0641};
use magic::{Call, Eax, SCRATCH};
use magic_hi::{f, konst};
use magic_owna::{rec, recs, twin};

const N: u16 = 641;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10002e60: a dirty header and pool.
#[test]
fn clear_pool() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2e60);
    let pool = SCRATCH + 0x1000;
    t.set_u32(t.img.word(at + 0x7), pool);
    t.write(pool, &vec![0x6c; 100 * 0x94 + 0x10]);
    t.write(t.img.word(at + 0x11), &[0x6d; 0x60]);
    t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0641::clear_pool(g, img, fr))
    });
}

/// 0x10001490: distinct request words.
#[test]
fn take_requests() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1490);
    for (i, op) in (0u32..).zip([0x01, 0x0b, 0x15, 0x1f, 0x29, 0x33, 0x3d, 0x47, 0x51]) {
        t.set_u32(t.img.word(at + op), 0x00c0_0000 + i);
    }
    t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0641::take_requests(g, img, fr))
    });
}

/// 0x10001550: every request/state mix, the hold at and off its value,
/// the level above, inside, below its range and reaching 0, the timer
/// running out and not; with and without `ecx`.
#[test]
fn step_flashes() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1550);
    let h = konst(&t, at, 0x4);
    let s = konst(&t, at, 0xc);
    let arr = |t: &magic::Twin, op: u32| t.img.word(at + op);
    // (request, on, current, hold, level, timer on, timer)
    let cases: [(u32, u32, u32, f32, f32, u32, u32); 9] = [
        (1, 0, 0, 0.0, 0.0, 0, 0),
        (1, 1, 1, h, s, 1, 1),
        // the timer off, so the flash's going off (ecx set) starts it
        (0, 0, 1, 0.5, 0.0, 0, 5),
        (0, 1, 0, h, h + s + 1.0, 0, 0),
        (2, 0, 1, h, s * 0.5, 1, 2),
        (0, 0, 0, h, 0.25 * h + s, 0, 0),
        (1, 0, 1, f32::NAN, 0.0, 0, 0),
        (0, 0, 1, h, f32::NAN, 1, 0),
        (3, 1, 1, -0.0, -1.0, 0, 0),
    ];
    for ecx in [0u32, 1] {
        for (k, c) in (0u32..).zip(cases) {
            let k4 = 4 * k;
            t.set_u32(arr(&t, 0x17) + k4, c.0);
            t.set_u32(arr(&t, 0x22) + k4, c.1);
            t.set_u32(arr(&t, 0x41) + k4, c.2);
            t.set_u32(arr(&t, 0x6e) + k4, f(c.3));
            t.set_u32(arr(&t, 0x7f) + k4, f(c.4));
            t.set_u32(arr(&t, 0xc8) + k4, c.5);
            t.set_u32(arr(&t, 0xd1) + k4, c.6);
        }
        t.check(at, Call::this(ecx), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0641::step_flashes(g, img, fr))
        });
    }
}

/// 0x10001640: one object.
#[test]
fn part_defaults() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1640);
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(t.img.word(at + 0xd), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    t.set_u32(t.img.word(at + 0x1), SCRATCH + 0x24);
    magic_owna::rec(&mut t, 941, 1, move |_, _| table);
    magic_owna::recs(&mut t, &[(714, 5)]);
    let obj = SCRATCH + 0x1000;
    t.write(obj, &[0x5b; 0x80]);
    t.check(at, Call::this(obj), &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0641::part_defaults(g, img, fr))
    });
}

/// 0x10001870 and 0x100019b0: nine dirty parts.
#[test]
fn nine_parts() {
    let Some(mut t) = twin(N) else { return };
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    magic_owna::rec(&mut t, 941, 1, move |_, _| table);
    magic_owna::recs(&mut t, &[(714, 5)]);
    let base = SCRATCH + 0x1000;
    for rva in [0x1870u32, 0x19b0] {
        let at = t.at(rva);
        let o = SCRATCH + 0x100;
        t.set_u32(t.img.word(at + 0x1c), o);
        t.set_u32(o + 0x10, 0x00b0_0010);
        t.set_u32(t.img.word(at + 0x11), SCRATCH + 0x24);
        t.write(base, &[0x5b; 9 * 0x80]);
        t.check_entry(at, Call::default(), &[base], Eax::Checked);
    }
}

/// 0x10001f20: off, and on with slot 261's answers (the timelines live,
/// so running or stopping one, or the wrong one, shows); the words it
/// copies and passes distinct.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1f20);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 261, 1, move |m, a| (m.r32(cell) >> (a[0] & 3)) & 1);
    recs(&mut t, &[(723, 1), (724, 2), (376, 1), (381, 1), (214, 1)]);
    rec(&mut t, 253, 0, |_, _| 1);
    for op in [0x44u32, 0x4b, 0x7b, 0x9d] {
        let g = t.img.word(at + op);
        t.set_u32(g, 0x00a5_0000 + op);
    }
    for (on, bits) in [
        (0u32, 0u32),
        (1, 0b0010),
        (1, 0b1000),
        (1, 0b0001),
        (1, 0b1001),
        (1, 0),
    ] {
        t.set_u32(t.img.word(at + 0x2), on);
        t.set_u32(cell, bits);
        for (k, op) in (0u32..).zip([0x6b, 0xc9, 0xe9, 0xf3]) {
            let tl = t.img.word(at + op);
            magic_h6::live_timeline(&mut t, tl, SCRATCH + 0x600 + 0x20 * k, 0x40 + k);
        }
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
