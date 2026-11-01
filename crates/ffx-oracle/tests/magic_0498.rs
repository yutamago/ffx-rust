//! `magic_0498.dll` against its Rust port (`ffx_magic::m0498`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, recs, twin};

const N: u16 = 498;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001200: the start-up.
#[test]
fn start_up() {
    let Some(mut t) = twin(N) else { return };
    let d = SCRATCH + 0x400;
    for i in 0..8 {
        t.set_u32(d + 4 * i, 0x0d00_0000 + i);
    }
    rec(&mut t, 803, 2, |_, _| 0x2100_0000);
    rec(&mut t, 719, 3, move |_, _| d);
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    recs(&mut t, &[(718, 3), (222, 4), (711, 3), (710, 4)]);
    let at = t.at(0x1200);
    let blk = t.img.word(at + 0x127);
    t.write(blk, &[0x5e; 0x110]);
    t.check_entry(at, Call::default(), &[0x00a6_0000], Eax::Checked);
}
