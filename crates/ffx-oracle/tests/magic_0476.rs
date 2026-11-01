//! `magic_0476.dll` against its Rust port (`ffx_magic::m0476`). Needs
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
use magic_hi::lone_node;
use magic_owna::{recs, twin};

const N: u16 = 476;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100017f0: counts running (above and below 120, negative products)
/// and the last step, which gives the node back.
#[test]
fn fade_step() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x17f0);
    let give = t.img.rel32_target(at + 0x97);
    recs(&mut t, &[(348, 6)]);
    t.set_u32(t.img.word(at + 0x72), 0x00a7_0000);
    t.set_u32(t.img.word(at + 0x67), 0x00a6_0000);
    let node = SCRATCH + 0x2000;
    for n in [0x200u32, 0x79, 0x78, 0x40, 2, 1, 0, 0x8000_0001] {
        t.set_u32(t.img.word(at + 0x7), n);
        lone_node(&mut t, give, node);
        t.check_entry(at, Call::default(), &[node], Eax::Checked);
    }
}
