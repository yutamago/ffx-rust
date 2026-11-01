//! `magic_0612.dll` against its Rust port (`ffx_magic::m0612`). Needs
//! `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 612;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10009a20: the object's start, its point copied back.
#[test]
fn object_5f0() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x9a20);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0xc), fx);
    let so = t.img.rel32_target(at + 0x20);
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    t.set_u32(t.img.word(so + 0x04), SCRATCH + 0x2f0);
    t.set_u32(t.img.word(so + 0x5e), SCRATCH + 0x280);
    let sp = t.img.rel32_target(so + 0x84);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x7f8);
    t.write(fx, &[0x3c; 0x3c00]);
    for i in 0..4 {
        t.set_u32(fx + 0x2b30 + 4 * i, magic_hi::f(1.5 + i as f32));
    }
    t.set_u32(fx + 0x3ba4, 0x3ba4_0001);
    magic_h6::scrub(&mut t);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}
