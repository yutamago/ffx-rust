//! `magic_0474.dll` against its Rust port (`ffx_magic::m0474`). Needs
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

use ffx_magic::{Ret, m0474};
use magic::{Call, Eax, SCRATCH};
use magic_hi::{list, pool};
use magic_owna::twin;

const N: u16 = 474;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001f00: an empty list and one with a node, pools of 1 and 2.
#[test]
fn timed_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1f00);
    let take = t.img.rel32_target(at + 0x27);
    let obj = SCRATCH + 0x1000;
    for (n, full) in [(1u32, false), (2, true), (2, false)] {
        pool(&mut t, take, SCRATCH + 0x2000, n);
        let w = |op: u32| t.img.word(at + op);
        let (head, tail) = (w(0x41), w(0x2d));
        list(&mut t, head, tail, SCRATCH + 0x3000, full);
        t.write(obj, &[0x55; 0xa0]);
        t.check(at, Call::this(obj), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0474::timed_start(g, img, fr.at, fr.ecx))
        });
    }
}
