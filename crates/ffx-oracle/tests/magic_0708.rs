//! `magic_0708.dll` against its Rust port (`ffx_magic::m0708`). Needs
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

use ffx_magic::{Ret, m0708};
use magic::{Call, Eax};
use magic_owna::{recs, twin};

const N: u16 = 708;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001000: before, at and after 90; inside, after the end and past
/// its limit; negative times; a large product.
#[test]
fn fade() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1000);
    recs(&mut t, &[(352, 3)]);
    for (x, end) in [
        (0i32, 200i32),
        (45, 200),
        (90, 200),
        (91, 200),
        (200, 200),
        (201, 200),
        (289, 200),
        (290, 200),
        (-7, 200),
        (-0x0400_0000, 0x7fff_ff00),
        (150, 100),
        (100, 30),
        // 90 with the end below it: the rise's bound
        (90, 30),
    ] {
        let c = Call {
            ecx: x.cast_unsigned(),
            edx: end.cast_unsigned(),
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0708::fade(g, img, fr))
        });
    }
}

/// 0x10001070: negative, low, middle and last steps; past the end.
#[test]
fn rise() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1070);
    magic_owna::rec(&mut t, 776, 1, |_, a| a[0] ^ 0x0776_0000);
    recs(&mut t, &[(915, 5)]);
    for (x, n) in [
        (-3i32, 10i32),
        (0, 10),
        (1, 10),
        (2, 10),
        (7, 10),
        (10, 10),
        (11, 10),
        (5, 3),
    ] {
        let c = Call {
            ecx: x.cast_unsigned(),
            edx: n.cast_unsigned(),
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0708::rise(g, img, fr))
        });
    }
}

/// 0x10001560: no entries and three.
#[test]
fn bind_parts() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1560);
    let datas = magic::SCRATCH + 0x2000;
    for k in 0..3u32 {
        t.write(datas + 0x40 * k, &[0x2d; 0x40]);
        t.set_u16(datas + 0x40 * k + 0x12, [3u16, 0xfffe, 0x100][k as usize]);
    }
    magic_owna::rec(&mut t, 941, 1, move |_, a| datas + 0x40 * (a[0] & 3));
    magic_owna::rec(&mut t, 264, 1, |_, _| 0x0264_0000);
    recs(&mut t, &[(231, 4), (361, 4), (265, 1)]);
    let (rec, obj) = (magic::SCRATCH + 0x100, magic::SCRATCH + 0x1000);
    t.set_u32(rec, 0x00ec_0000);
    for k in 0..3u32 {
        t.set_u32(rec + 0x10 + 0x10 * k, 0x00e0_0000 + k);
    }
    for n in [0u16, 3] {
        t.set_u16(rec + 4, n);
        // the half-word after the count non-zero (a count read as u32 shows)
        t.set_u16(rec + 6, 0x5a5a);
        t.write(obj, &[0x4e; 0x40]);
        let c = Call {
            ecx: rec,
            edx: obj,
            ..Call::default()
        };
        t.check(at, c, &[0x00ba_0000], Eax::Checked, |g, img, fr| {
            Ret::eax(m0708::bind_parts(g, img, fr))
        });
    }
}

/// 0x100012d0: negative, zero, middle, last and too late times.
#[test]
fn shake() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x12d0);
    recs(&mut t, &[(348, 6)]);
    for tm in [-20i32, 0, 1, 75, 149, 150, 151, 1000] {
        let c = Call {
            eax: 0x00ea_0000,
            ecx: 0x00a0_0000,
            edx: 0x00b0_0000,
            ..Call::default()
        };
        t.check(at, c, &[tm.cast_unsigned()], Eax::Checked, |g, img, fr| {
            Ret::eax(m0708::shake(g, img, fr))
        });
    }
}

/// 0x10001180: two sets of slot answers.
#[test]
fn place() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1180);
    let cell = magic::SCRATCH + 0x10;
    magic_owna::rec(&mut t, 215, 1, |_, _| 0x0215_0000);
    for (s, n) in [(169u32, 4usize), (216, 11), (251, 12)] {
        magic_owna::rec(&mut t, s, n, move |_, a| s << 16 ^ a[0]);
        t.stub_st0(s, move |m, _| {
            f64::from(m.r32(cell) & 0xff) * 1.375 - f64::from(s & 7)
        });
    }
    recs(&mut t, &[(210, 3), (217, 6), (325, 8)]);
    t.set_u32(t.img.word(at + 0xb), 0x00ac_0000);
    // the bytes after the one place sets, non-zero
    let byte = t.img.word(at + 0x105);
    for k in [3u32, 200] {
        t.set_u32(byte, 0x6d6d_6d00);
        t.set_u32(cell, k);
        t.set_u32(t.img.word(at + 0xd7), (12.5f32).to_bits());
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0708::place(g, img, fr))
        });
    }
}
