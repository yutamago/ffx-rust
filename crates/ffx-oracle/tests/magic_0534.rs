//! `magic_0534.dll` against its Rust port (`ffx_magic::m0534`). Needs
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

const N: u16 = 534;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The steps, the target count below and at the number of targets.
#[test]
fn steps() {
    let Some(mut t) = twin(N) else { return };
    let s = SCRATCH + 0x100;
    t.set_u32(0x1007_c2b0, s);
    t.set_u32(s + 0x198, 0x00ac_7000);
    t.set_u32(s + 0x19c, 3);
    let other = SCRATCH + 0x2000;
    rec(&mut t, 434, 2, move |_, _| other);
    recs(
        &mut t,
        &[
            (450, 2),
            (451, 2),
            (447, 2),
            (459, 3),
            (449, 2),
            (441, 3),
            (443, 2),
            (440, 4),
        ],
    );
    let seq = SCRATCH + 0x1000;
    let count = t.img.word(t.at(0x11a0) + 0x4);
    for (k, n) in [0u32, 1, 2, 7].into_iter().enumerate() {
        t.set_u32(count, n);
        t.write(seq, &[0; 0x140]);
        // the words past the colour non-zero, so the cleared `+0x13c`
        // shows (and a store one word on)
        t.set_u32(seq + 0x13c, 0x5a5a_5a5a);
        t.set_u32(seq + 0x140, 0x5a5a_5a5a);
        t.write(other, &[0; 0x100]);
        t.set_u32(seq + 0xac, k as u32);
        t.set_u32(other + 0xac, 3 - k as u32);
        for rva in [0x11a0u32, 0x12b0, 0x1390] {
            t.check_entry(t.at(rva), Call::default(), &[seq], Eax::Checked);
        }
    }
}
