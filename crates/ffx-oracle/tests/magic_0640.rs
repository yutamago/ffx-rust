//! `magic_0640.dll` against its Rust port (`ffx_magic::m0640`). Needs
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

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, recs, twin};

const N: u16 = 640;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001730: no list; the data missing, found in a free slot and in a
/// used one.
#[test]
fn register() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1730);
    rec(&mut t, 941, 1, |_, a| a[0] ^ 0x0941_0000);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 719, 3, move |m, a| {
        if a[2] == m.r32(cell) {
            m.r32(cell + 4)
        } else {
            0x0719_0000 + a[2]
        }
    });
    recs(&mut t, &[(711, 3)]);
    let lock = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0xce), lock);
    t.set_u32(lock, 0x00a1_0000);
    let gp = SCRATCH + 0x24;
    t.set_u32(t.img.word(at + 0x4f), gp);
    let (obj, d, l, list) = (
        SCRATCH + 0x1000,
        SCRATCH + 0x1100,
        SCRATCH + 0x300,
        SCRATCH + 0x400,
    );
    t.write(obj, &[0; 0x100]);
    t.set_u32(obj + 0xd0, d);
    for k in 0..3 {
        t.set_u32(d + 0x14 + 4 * k, 0x00d0_0000 + k);
    }
    t.set_u32(t.img.word(at + 0x5d), l);
    t.set_u32(cell + 4, d);
    let used = t.img.word(at + 0xaf);
    let parts = t.img.word(at + 0xbc);
    for k in 0..4 {
        t.set_u32(parts + 4 * k, 0x00fa_0000 + k);
    }
    for (has_list, found, busy) in [
        (false, 0u32, false),
        (true, 9, false),
        (true, 2, false),
        (true, 2, true),
    ] {
        t.set_u32(l + 4, if has_list { list } else { 0 });
        t.set_u16(list + 0x50, 4);
        t.set_u32(cell, found);
        for k in 0..4 {
            t.set_u32(used + 4 * k, if busy { 0x00b5_0000 } else { 0 });
        }
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}
