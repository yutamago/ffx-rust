//! `magic_0606.dll` against its Rust port (`ffx_magic::m0606`). Needs
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

use ffx_magic::{Ret, m0606};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{recs, twin};

const N: u16 = 606;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10007a10: no, negative and four targets, in and out of battle.
#[test]
fn show_targets() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7a10);
    recs(&mut t, &[(530, 2), (249, 2)]);
    let list = SCRATCH + 0x300;
    t.set_u32(t.img.word(at + 0x19), list);
    t.write(list, &[3, 0x25, 0x1f, 0x20]);
    let table = t.img.word(at + 0x55);
    for k in 0..0x40 {
        t.set_u32(table + 4 * k, 0x00ab_0000 + k);
    }
    for (n, battle) in [(0i32, 0u32), (-1, 0), (4, 0), (4, 1)] {
        t.set_u32(t.img.word(at + 0x5), n.cast_unsigned());
        t.set_u32(t.img.word(at + 0x12), battle);
        let c = Call {
            eax: 0x00ea_0000,
            ..Call::default()
        };
        t.check_entry(at, c, &[], Eax::Checked);
    }
}

/// 0x10008990: slot 520 saying 0x1590000 and something else; the point
/// with `x` and `z` -0.0 too (the eye's `v - 0` keeps the sign, `v + 0`
/// would not).
#[test]
fn camera_points() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x8990);
    recs(&mut t, &[(327, 2)]);
    let zeros = SCRATCH + 0x14;
    magic_owna::rec(&mut t, 229, 5, move |m, a| {
        let neg = m.r32(zeros) != 0;
        for i in 0..4 {
            let v = if neg && i % 2 == 0 {
                -0.0f32
            } else {
                0.5 + i as f32 * -1.75
            };
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x0229_0000
    });
    let cell = SCRATCH + 0x10;
    magic_owna::rec(&mut t, 520, 0, move |m, _| m.r32(cell));
    t.set_u32(t.img.word(at + 0x13), SCRATCH + 0x4000);
    for (answer, neg) in [
        (0x0159_0000u32, 0u32),
        (0x0158_0000, 0),
        (0x0159_0000, 1),
        (0x0158_0000, 1),
    ] {
        t.set_u32(cell, answer);
        t.set_u32(zeros, neg);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10008720: the three tasks started with the task list empty and
/// not.
#[test]
fn start_tasks() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x8720);
    let s = SCRATCH + 0x8000;
    t.set_u32(t.img.word(at + 0x5), s);
    let take = t.img.rel32_target(at + 0x67);
    let first = t.img.word(at + 0x6d);
    let head = t.img.word(at + 0x82);
    let old = SCRATCH + 0x900;
    for full in [false, true] {
        magic_hi::pool(&mut t, take, SCRATCH + 0x1000, 5);
        t.write(s + 0x5800, &[0x2b; 0x800]);
        t.write(old, &[0x6c; 0x20]);
        t.set_u32(first, if full { old } else { 0 });
        t.set_u32(head, if full { old } else { 0 });
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0606::start_tasks(g, img, fr))
        });
    }
}
