//! `magic_0528.dll` against its Rust port (`ffx_magic::m0528`). Needs
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

const N: u16 = 528;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001000 and 0x100014f0.
#[test]
fn steps() {
    let Some(mut t) = twin(N) else { return };
    let s = SCRATCH + 0x100;
    t.set_u32(0x1008_0110, s);
    t.set_u32(s + 0x198, 0x00ac_7000);
    rec(&mut t, 481, 1, |_, a| 0x0481_0000 ^ a[0]);
    recs(
        &mut t,
        &[(21, 3), (278, 3), (450, 2), (449, 2), (441, 3), (440, 4)],
    );
    let seq = SCRATCH + 0x1000;
    // the flags word's bit 4 clear first (`|= 0x10` shows), and the
    // word the start clears non-zero
    t.write(seq, &[0x41; 0x140]);
    t.set_u32(seq + 0xc0, 0x2c2c_2c2c);
    let cleared = t.img.word(t.at(0x1000) + 0x5b);
    t.set_u32(cleared, 0x5a5a_5a5a);
    for rva in [0x1000u32, 0x14f0] {
        t.check_entry(t.at(rva), Call::default(), &[seq], Eax::Checked);
    }
}
