//! `magic_0672.dll` against its Rust port (`ffx_magic::m0672`). Needs
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

use ffx_magic::{Ret, m0672};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{recs, twin};

const N: u16 = 672;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100016a0: one camera.
#[test]
fn camera_reset() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x16a0);
    recs(&mut t, &[(357, 3), (356, 6), (399, 14)]);
    t.set_u32(t.img.word(at + 0x8), 0x00ca_0000);
    t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0672::camera_reset(g, img, fr))
    });
}

/// 0x100015f0: no tracks, a negative count, and two tracks with no and
/// three keys.
#[test]
fn copy_keys() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x15f0);
    let cell = magic::SCRATCH + 0x10;
    let (src, dst) = (magic::SCRATCH + 0x1000, magic::SCRATCH + 0x2000);
    magic_owna::rec(&mut t, 586, 1, move |m, _| m.r32(cell));
    magic_owna::rec(&mut t, 584, 2, move |_, a| src + 0x100 * a[1]);
    magic_owna::rec(&mut t, 585, 2, move |m, a| {
        m.w32(a[2], 0x30 * a[1]);
        dst + 0x100 * a[1]
    });
    recs(&mut t, &[(346, 1), (399, 3)]);
    t.set_u32(t.img.word(at + 0x15), 0x00ca_0000);
    let pattern: Vec<u8> = (0..0x200u32)
        .map(|i| u8::try_from(i & 0xff).unwrap())
        .collect();
    for n in [0i32, -1, 2] {
        t.set_u32(cell, n.cast_unsigned());
        t.write(src, &pattern);
        t.write(dst, &[0xdd; 0x200]);
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0672::copy_keys(g, img, fr))
        });
    }
}

/// 0x10001b80 and 0x10001c30: the fade task with the countdown above 1,
/// at 1 and out (the give-back), and the fade start with the list empty
/// and not.
#[test]
fn fade() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1b80);
    let st = t.at(0x1c30);
    let s = SCRATCH + 0x4000;
    t.set_u32(t.img.word(at + 0x6), s);
    magic_owna::recs(&mut t, &[(348, 6), (587, 2)]);
    for (op, v) in [(0x73u32, 0x00a5_0073u32), (0x7e, 0x00a5_007e)] {
        let g = t.img.word(at + op);
        t.set_u32(g, v);
    }
    let take = t.img.rel32_target(st + 0x65);
    magic_hi::pool(&mut t, take, SCRATCH + 0x8000, 4);
    // the task node: no children (`+8` itself, its `+0xc` 0), unlinked
    let task = SCRATCH + 0x700;
    for n in [40u32, 1, 0, 0xffff_fff0] {
        t.write(s, &[0x2b; 0x600]);
        t.set_u32(s + 0x5b8, n);
        t.write(task, &[0; 0x40]);
        t.set_u32(task + 8, task);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
    for head in [0u32, SCRATCH + 0x900] {
        t.write(s + 0x280, &[0x3c; 0x100]);
        // the pool afresh (the give-backs above took its ring apart)
        magic_hi::pool(&mut t, take, SCRATCH + 0x8000, 4);
        let empty = t.img.word(st + 0x6b);
        t.set_u32(empty, head);
        let h = t.img.word(st + 0x7f);
        t.set_u32(h, head);
        t.check_entry(st, Call::default(), &[], Eax::Checked);
    }
}
