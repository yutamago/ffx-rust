//! `magic_0508.dll` against its Rust port (`ffx_magic::m0508`). Needs
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
use magic_hi::f;
use magic_owna::{rec, twin};

const N: u16 = 508;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001b40: two actors.
#[test]
fn e1b40() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1b40);
    let b = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x11), b);
    let table = t.img.word(at + 0x1d);
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x00ac_0000 + i);
    }
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4 {
            m.w32(a[i], f(i as f32 * 1.25));
        }
        0x262
    });
    rec(&mut t, 268, 3, |m, a| {
        let y = m.r32(a[2] + 4);
        m.w32(a[2] + 4, y ^ a[1] ^ a[0]);
        0x268
    });
    let obj = SCRATCH + 0x1000;
    for actor in [2u32, 5] {
        // the byte after the actor index non-zero (an index read as u16 shows)
        t.set_u32(b, 0x0017_3c00 | actor);
        t.write(obj, &[0x77; 0x110]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}
