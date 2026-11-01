//! `magic_0680.dll` against its Rust port (`ffx_magic::m0680`). Needs
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

use ffx_magic::m0680;
use magic::{Call, Eax, SCRATCH};
use magic_hi::{f, konst, objects};
use magic_owna::twin;

const N: u16 = 680;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001af0: the angle above, inside and below its range; an object
/// running, the count above 0, and the full path.
#[test]
fn task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1af0);
    let ro = t.img.rel32_target(at + 0x53);
    let (s, h, l) = (konst(&t, at, 0xf), konst(&t, at, 0x16), konst(&t, at, 0x2a));
    let (task, record) = (SCRATCH + 0x100, SCRATCH + 0x180);
    // the word the task copies twice, distinct
    magic_h6::distinct(&mut t, at, &[0x3c]);
    for (angle, running, count) in [
        (h - s + 1.0, false, 0u32),
        (l - s - 1.0, true, 0),
        (f32::midpoint(h, l) - s, false, 1),
        (f32::NAN, false, 0xffff_ffff),
        // exactly at each bound after the step
        (magic_h6::hits(h, s), false, 1),
        (magic_h6::hits(l, s), false, 1),
    ] {
        objects(&mut t, ro, running);
        t.set_u32(t.img.word(at + 0x7), f(angle));
        t.set_u32(t.img.word(at + 0x5e), count);
        t.write(task, &[0x11; 0x20]);
        t.set_u32(task + 8, record);
        t.write(record, &[0x22; 0x20]);
        t.check(at, Call::default(), &[task], Eax::Checked, |g, img, fr| {
            m0680::task(g, img, fr)
        });
    }
}
