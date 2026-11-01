//! `magic_0564.dll` against its Rust port (`ffx_magic::m0564`). Needs
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

use ffx_magic::{Ret, m0564};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{recs, twin};

const N: u16 = 564;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001b40: the flag clear and set, the effect's actor and indexed
/// ones, numbers below and from 32.
#[test]
fn hide() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1b40);
    recs(&mut t, &[(296, 2), (257, 2)]);
    let b = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x9), b);
    t.set_u32(b, 0x2205_1f03);
    let table = t.img.word(at + 0x5d);
    for i in 0..0x40 {
        t.set_u32(table + 4 * i, 0x00ac_0000 + i);
    }
    t.set_u32(t.img.word(at + 0x3c), 0x00ae_0000);
    for (flag, own, i) in [
        (0u32, 3u32, u32::MAX),
        (0, 0x20, u32::MAX),
        (0, 0, 0),
        (0, 0, 2),
        (0, 0, 3),
        (1, 3, u32::MAX),
        (1, 0, 1),
    ] {
        t.set_u32(t.img.word(at + 0x2), flag);
        t.set_u32(t.img.word(at + 0x10), own);
        t.check(at, Call::this(i), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0564::hide(g, img, fr))
        });
    }
}

/// 0x10001990: the counts (above 1, at 1, at 0, negative), the motion
/// scripts and the objects each stopping the end, and the end.
#[test]
fn task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1990);
    let hd = t.img.rel32_target(at + 0x2c);
    let b = SCRATCH + 0x20;
    t.set_u32(t.img.word(hd + 0x9), b);
    t.set_u32(b, 0x0302_0100);
    t.set_u32(t.img.word(hd + 0x2), 0);
    recs(&mut t, &[(296, 2), (257, 2)]);
    let counts = t.img.word(at + 0x12);
    let actors = t.img.word(at + 0x27);
    for i in 0..3 {
        t.set_u32(actors + 4 * i, i);
    }
    let ms = [0x39u32, 0x43, 0x4f].map(|op| t.img.word(at + op));
    let ro = t.img.rel32_target(at + 0x5d);
    let (node, task) = (SCRATCH + 0x5000, SCRATCH + 0x5100);
    // (counts, scripts done, objects running)
    for (c, done, running) in [
        ([2u32, 1, 0], [true; 3], false),
        ([0, 0, 0], [true; 3], false),
        ([0xffff_fff0, 0, 1], [true; 3], false),
        ([0, 0, 0], [false, true, true], false),
        ([0, 0, 0], [true, false, true], false),
        ([0, 0, 0], [true, true, false], false),
        ([0, 0, 0], [true; 3], true),
    ] {
        for i in 0..3 {
            t.set_u32(counts + 4 * i as u32, c[i]);
            t.set_u32(ms[i] + 0x14, u32::from(done[i]));
            t.set_u16(ms[i] + 0x18, 3);
        }
        magic_hi::objects(&mut t, ro, running);
        t.write(node, &[0x4e; 0x20]);
        t.set_u32(node + 8, task);
        t.write(task, &[0x7a; 0x20]);
        t.check_entry(at, Call::default(), &[node], Eax::Checked);
    }
}
