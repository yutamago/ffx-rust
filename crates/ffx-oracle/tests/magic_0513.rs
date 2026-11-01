//! `magic_0513.dll` against its Rust port (`ffx_magic::m0513`). Needs
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

const N: u16 = 513;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100089a0: no node; a node that is its neighbours' first and last;
/// one with other neighbours.
#[test]
fn end() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x89a0);
    recs(&mut t, &[(250, 2)]);
    let w = |t: &magic::Twin, op: u32| t.img.word(at + op);
    let n = SCRATCH + 0x1000;
    let (a, b) = (SCRATCH + 0x2000, SCRATCH + 0x3000);
    for case in 0..3 {
        t.write(n, &[0x31; 0x20]);
        t.set_u16(n + 0x18, 5);
        t.write(a, &[0x32; 0x20]);
        t.write(b, &[0x33; 0x20]);
        t.set_u32(w(&t, 0x2), if case == 0 { 0 } else { n });
        let (next, prev) = if case == 1 { (n, n) } else { (a, b) };
        t.set_u32(w(&t, 0x15), next);
        t.set_u32(w(&t, 0x1a), prev);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
