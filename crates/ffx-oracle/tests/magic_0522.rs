//! `magic_0522.dll` against its Rust port (`ffx_magic::m0522`). Needs
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
use magic_owna::{recs, twin};

const N: u16 = 522;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The sequence steps: the count below and at 10, no target and three.
#[test]
fn steps() {
    let Some(mut t) = twin(N) else { return };
    let s = SCRATCH + 0x100;
    let at = t.at(0x1360);
    t.set_u32(t.img.word(at + 0x4), s);
    t.set_u32(s + 0x198, 0x00ac_7000);
    for i in 0..4 {
        t.set_u32(s + 0x1a0 + 4 * i, 0x00ac_7100 + i);
    }
    recs(
        &mut t,
        &[
            (434, 2),
            (450, 2),
            (451, 2),
            (442, 3),
            (443, 2),
            (440, 4),
            (446, 2),
            (449, 2),
        ],
    );
    let seq = SCRATCH + 0x1000;
    let count = t.img.word(t.at(0x1140) + 0x2f);
    for (n, targets) in [(0u32, 0u32), (9, 3), (0x7fff_fff0, 1)] {
        t.set_u32(count, n);
        t.set_u32(s + 0x19c, targets);
        t.write(seq, &[0x51; 0x140]);
        for rva in [0x1140u32, 0x1360, 0x1400] {
            t.check_entry(t.at(rva), Call::default(), &[seq], Eax::Checked);
        }
    }
}
