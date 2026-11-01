//! `magic_0673.dll` against its Rust port (`ffx_magic::m0673`). Needs
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
use magic_hi::{list, pool};
use magic_owna::twin;

const N: u16 = 673;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001dd0: entries 0 and 2, the list empty and not.
#[test]
fn start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1dd0);
    let r = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x23), r);
    let take = t.img.rel32_target(at + 0x68);
    let ctx = SCRATCH + 0x100;
    for (k, full) in [(0u32, false), (2, true)] {
        pool(&mut t, take, SCRATCH + 0x1000, 3);
        let (head, tail) = (t.img.word(at + 0x85), t.img.word(at + 0x6e));
        list(&mut t, head, tail, SCRATCH + 0x3000, full);
        t.write(r + 0x59f8, &[0x3b; 0x100]);
        t.write(ctx, &[0; 0x40]);
        t.set_u32(ctx + 0x38, k);
        // the template's last word non-zero (the record's `+0x24` takes it)
        let e = t.img.word(at + 0x1d) + 32 * k;
        t.set_u32(e + 0x1c, 0x00e1_c000 + k);
        t.check_entry(at, Call::default(), &[ctx], Eax::Checked);
    }
}
