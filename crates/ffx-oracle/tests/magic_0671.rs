//! `magic_0671.dll` against its Rust port (`ffx_magic::m0671`). Needs
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

use ffx_magic::{Ret, m0671};
use magic::{Call, Eax, SCRATCH};
use magic_hi::{list, pop_pool};
use magic_owna::{rec, recs, twin};

const N: u16 = 671;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001520: the list empty and not.
#[test]
fn start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1520);
    let pop = t.img.rel32_target(at + 0x73);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    recs(&mut t, &[(711, 3), (710, 4)]);
    for (n, full) in [(1u32, false), (3, true)] {
        pop_pool(&mut t, pop, SCRATCH + 0x2000, n);
        let w = |op: u32| t.img.word(at + op);
        let (head, tail) = (w(0x8d), w(0x79));
        list(&mut t, head, tail, SCRATCH + 0x3000, full);
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0671::start(g, img, fr))
        });
    }
}
