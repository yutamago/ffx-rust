//! `magic_0526.dll` against its Rust port (`ffx_magic::m0526`). Needs
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

const N: u16 = 526;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The sequence steps with one and two targets.
#[test]
fn steps() {
    let Some(mut t) = twin(N) else { return };
    let s = SCRATCH + 0x100;
    t.set_u32(0x1007_d3d0, s);
    t.set_u32(s + 0x198, 0x00ac_7000);
    for i in 0..4 {
        t.set_u32(s + 0x1a0 + 4 * i, 0x00ac_7100 + i);
    }
    recs(
        &mut t,
        &[
            (434, 2),
            (450, 2),
            (449, 2),
            (442, 3),
            (443, 2),
            (451, 2),
            (446, 2),
            (440, 4),
        ],
    );
    let seq = SCRATCH + 0x1000;
    for (k, targets) in [(0u32, 1u32), (1, 2)] {
        t.set_u32(s + 0x19c, targets);
        t.write(seq, &[0; 0x140]);
        t.set_u32(seq + 0xac, k);
        t.set_u32(seq + 0x2c + 16 * k, 2 - k);
        for rva in [0x1650u32, 0x16e0, 0x1140] {
            t.check_entry(t.at(rva), Call::default(), &[seq], Eax::Checked);
        }
    }
}
