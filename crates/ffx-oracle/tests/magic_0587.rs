//! `magic_0587.dll` against its Rust port (`ffx_magic::m0587`). Needs
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

use ffx_magic::m0587;
use magic::{Call, Eax, SCRATCH};
use magic_hi::{f, konst, pool};
use magic_owna::{rec, recs, twin};

const N: u16 = 587;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001a90: one camera.
#[test]
fn camera_reset() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1a90);
    recs(&mut t, &[(357, 3), (356, 3), (362, 4)]);
    rec(&mut t, 562, 1, |_, _| 0x0056_2000);
    t.set_u32(t.img.word(at + 0x8), 0x00ca_0000);
    t.set_u32(t.img.word(at + 0x4f), 0x00a0_0000);
    magic_h6::scrub(&mut t);
    t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
        m0587::camera_reset(g, img, fr)
    });
}

/// 0x100021f0: groups with no, one and several entries.
#[test]
fn load_groups() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x21f0);
    rec(&mut t, 941, 1, |_, a| a[0] ^ 0x0941_0000);
    t.stub(772, |m, a| {
        let k = m.r32(SCRATCH + 0x3_0000);
        m.w32(SCRATCH + 0x3_0000, k + 1);
        m.w32(SCRATCH + 0x3_0004 + 4 * (k % 16), a[0] ^ a[1]);
        a[0] ^ a[1]
    });
    t.stub_edx(772, |_, a| a[1] + 0x77);
    let flag = SCRATCH + 0x10;
    t.set_u32(t.img.word(at + 0x5b), flag);
    t.set_u32(flag, 0x00f1_0000);
    // slot 779 keeps the game flag it runs under
    magic_h6::rec_cells(&mut t, 779, 4, &[flag], |_, a| {
        0x5a5a_0779 ^ a.first().copied().unwrap_or(0)
    });
    let r = SCRATCH + 0x4000;
    t.set_u32(t.img.word(at + 0x8), r);
    for (k, n) in (0u32..).zip([0i32, 1, 3, -1]) {
        t.set_u32(r + 0x166c + 0x8a8 * k, n.cast_unsigned());
        t.set_u32(r + 0xe48 + 0x8a8 * k, 0x00e0_0000 + k);
    }
    magic_h6::scrub(&mut t);
    t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
        m0587::load_groups(g, img, fr)
    });
}

/// 0x100022b0: the list empty and not.
#[test]
fn glow_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x22b0);
    recs(&mut t, &[(371, 4)]);
    let r = SCRATCH + 0x4000;
    t.set_u32(t.img.word(at + 0xe), r);
    let take = t.img.rel32_target(at + 0x83);
    let head = SCRATCH + 0x300;
    for empty in [true, false] {
        pool(&mut t, take, SCRATCH + 0x1000, 3);
        t.write(r + 0xaa0, &[0x3b; 0x100]);
        t.write(head, &[0x4d; 0x20]);
        t.set_u32(t.img.word(at + 0x89), u32::from(!empty));
        t.set_u32(t.img.word(at + 0x9d), head);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10002380: one source.
#[test]
fn glow_colour() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2380);
    let r = SCRATCH + 0x4000;
    t.set_u32(t.img.word(at + 0x33), r);
    for i in 0..0x10 {
        t.set_u32(r + 0xe10 + 4 * i, 0x00e1_0000 + i);
    }
    let src = SCRATCH + 0x200;
    for i in 0..0x10 {
        t.set_u32(src + 4 * i, f(i as f32 * 0.75 - 2.0));
    }
    t.set_u32(src + 0x30, 0x7fc0_1234);
    magic_h6::scrub(&mut t);
    t.check_entry(at, Call::default(), &[src], Eax::Checked);
}

/// 0x100031f0: the angle above, inside and below its range, NaN; the
/// counters below and at 24.
#[test]
fn glow_turn() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x31f0);
    let sr = t.img.rel32_target(at + 0x69);
    t.set_u32(t.img.word(sr + 0x1d), 0x00ab_0000);
    rec(&mut t, 235, 1, |_, _| 0x235);
    for (s, k) in [(277u32, 2.0f64), (15, 0.5), (16, -1.5), (17, 3.0)] {
        rec(&mut t, s, 1, move |_, a| s << 16 ^ a[0]);
        t.stub_st0(s, move |_, _| k);
    }
    for s in [330u32, 328, 329] {
        rec(&mut t, s, 3, move |_, a| s << 16 ^ a[2]);
    }
    let (hi, lo) = (konst(&t, at, 0x2e), konst(&t, at, 0x42));
    let r = SCRATCH + 0x4000;
    t.set_u32(t.img.word(at + 0x3), r);
    for (angle, counter) in [
        (hi + 1.0, 0u8),
        (hi, 0x17),
        (f32::midpoint(hi, lo), 0x18),
        (lo, 0xff),
        (lo - 1.0, 3),
        (f32::NAN, 0),
    ] {
        t.write(r + 0x980, &[0x3c; 0x100]);
        // the step x = 0.5 - 0.25 non-zero (an add for the sub shows), the
        // angle after the step as listed
        t.set_u32(r + 0xa84, f(0.5));
        t.set_u32(r + 0xa94, f(0.25));
        let a = if angle.is_nan() {
            angle
        } else {
            magic_h6::hits(angle, 0.25)
        };
        t.set_u32(r + 0xa44, f(a));
        t.set_u32(r + 0xa54, f(1.25));
        for k in 0..4 {
            t.set_u8(
                r + 0x1657 + 0x8a8 * k,
                counter.wrapping_add(u8::try_from(k).unwrap()),
            );
        }
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x100029b0: the sway frame with the angle above, between and below
/// its bounds; the float slots answering in ST0, the matrix slots
/// keeping what they get.
#[test]
fn sway_frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x29b0);
    let s = SCRATCH + 0x4000;
    t.set_u32(t.img.word(at + 0x18), s);
    for (slot, n) in [(314u32, 1usize), (277, 2), (15, 1), (16, 1), (17, 1)] {
        rec(&mut t, slot, n, move |_, _| slot);
        t.stub_st0(slot, move |_, _| 0.375 + f64::from(slot) / 64.0);
    }
    for (slot, n) in [(235u32, 1usize), (328, 3), (329, 3), (330, 3), (231, 3)] {
        rec(&mut t, slot, n, move |m, a| {
            if a[0] >= 0x2000_0000 {
                magic_owna::dump(m, 0x100 + slot % 0x40, a[0], 16);
            }
            slot
        });
    }
    let sr = t.img.rel32_target(at + 0xb1);
    let h = t.img.word(sr + 0x1d);
    t.set_u32(h, 0x00a5_001d);
    // the step, the bounds: also exactly at each bound after the step
    let step = magic_hi::konst(&t, at, 0xc1);
    let (hi, lo) = (magic_hi::konst(&t, at, 0xcb), magic_hi::konst(&t, at, 0xdf));
    for angle in [
        10.0f32,
        0.5,
        -10.0,
        magic_h6::hits(hi, step),
        magic_h6::hits(lo, step),
    ] {
        t.write(s, &[0x2b; 0xb00]);
        for (i, o) in [0xa80u32, 0xa88, 0xa90, 0xa98, 0xa94, 0xa9c, 0x6c0, 0x6c8]
            .into_iter()
            .enumerate()
        {
            t.set_u32(s + o, (1.25 * i as f32 - 3.0).to_bits());
        }
        t.set_u32(s + 0xa84, angle.to_bits());
        t.set_u16(s + 0x664, 0x0008);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
