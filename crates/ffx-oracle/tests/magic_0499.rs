//! `magic_0499.dll` against its Rust port (`ffx_magic::m0499`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax};
use magic_owna::{rec, recs, twin};

const N: u16 = 499;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100012b0: the start (the flags and state words set before).
#[test]
fn start() {
    let Some(mut t) = twin(N) else { return };
    rec(&mut t, 803, 2, |_, _| 0x2100_0000);
    rec(&mut t, 260, 1, |_, a| 0x0260_0000 ^ a[0]);
    recs(&mut t, &[(861, 3), (259, 3), (718, 1), (222, 4)]);
    let at = t.at(0x12b0);
    for op in [
        0x61u32, 0x67, 0x6d, 0x73, 0x79, 0x95, 0x9b, 0xa1, 0xa7, 0xad, 0xe6, 0xf0, 0xfa,
    ] {
        let a = t.img.word(at + op);
        t.set_u32(a, 0x7777_7777);
    }
    t.check_entry(at, Call::default(), &[0x00a6_0000], Eax::Checked);
}
