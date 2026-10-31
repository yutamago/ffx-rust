//! `magic_0475.dll` against its Rust port (`ffx_magic::m0475`). Needs
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

use magic::{Call, Eax, SCRATCH};
use magic_hi::{f, motion, objects};
use magic_owna::{rec, recs, twin};

const N: u16 = 475;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001600: the motion running, an object running, the count above 0,
/// slot 917 not 0xc9, and the full path.
#[test]
fn task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1600);
    let ms = t.img.rel32_target(at + 0x5);
    let ro = t.img.rel32_target(at + 0xc);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 917, 0, move |m, _| m.r32(cell));
    rec(&mut t, 869, 1, |m, _| {
        let k = m.r32(SCRATCH + 0x14);
        m.w32(SCRATCH + 0x14, k + 1);
        0x0869_0000 + k
    });
    recs(&mut t, &[(916, 8), (284, 4)]);
    let p = t.img.word(at + 0x5e);
    let (task, record) = (SCRATCH + 0x100, SCRATCH + 0x180);
    for (done, running, count, answer) in [
        (false, false, 0u32, 0xc9u32),
        (true, true, 0, 0xc9),
        (true, false, 1, 0xc9),
        (true, false, 0, 0xc8),
        (true, false, 0, 0xc9),
        (true, false, 0xffff_ffff, 0xc9),
    ] {
        motion(&mut t, ms, done);
        objects(&mut t, ro, running);
        t.set_u32(t.img.word(at + 0x23), count);
        t.set_u32(cell, answer);
        t.write(task, &[0x11; 0x20]);
        t.set_u32(task + 8, record);
        t.write(record, &[0x22; 0x20]);
        for i in 0..3 {
            t.set_u32(p + 4 * i, f(1.5 * i as f32 - 2.0));
        }
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}
