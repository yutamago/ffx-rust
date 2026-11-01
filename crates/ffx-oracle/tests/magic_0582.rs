//! `magic_0582.dll` against its Rust port (`ffx_magic::m0582`). Needs
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
use magic_owna::twin;

const N: u16 = 582;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001690: the flag byte and the control bit each clear and set.
#[test]
fn start() {
    let Some(mut t) = twin(N) else { return };
    t.stub(803, |_, _| SCRATCH + 0x1_0000);
    t.stub(718, |_, a| a[0] ^ 0x718);
    t.stub(719, |_, a| a[0] + 0x10);
    // slot 715 sees every word the original leaves on the stack (11)
    t.stub(715, |m, a| {
        for (i, w) in (0u32..).zip(&a[..11]) {
            m.w32(SCRATCH + 0x3_0000 + 4 * i, *w);
        }
        SCRATCH
    });
    t.stub(717, |_, _| 0);
    t.stub(720, |_, a| a[0] ^ a[1]);
    let f = t.at(0x1690);
    let flags = t.img.word(f + 0x1b);
    let slots = t.img.word(f + 0x21);
    let ctrl = t.img.word(f + 0x3b);
    let data = t.img.word(f + 0x43);
    let setup = t.at(0x1000);
    for (op, len) in [(0x71, 0x100), (0x76, 0x2000), (0xa8, 0x100), (0xad, 0x30)] {
        let at = t.img.word(setup + op);
        t.write(at, &vec![0xa5; len]);
    }
    let fl = SCRATCH + 0x2_0000;
    let s = SCRATCH + 0x2_0100;
    t.set_u32(flags, fl);
    t.set_u32(slots, s);
    t.set_u32(data, 0x00da_7a00);
    for (flag, bits) in [(0u8, 0x10u8), (1, 0x10), (1, 0xef), (0, 0)] {
        t.set_u8(fl + 0x10, flag);
        // the byte after the flag non-zero (a wider read shows)
        t.set_u8(fl + 0x11, 0x5a);
        t.set_u8(ctrl, bits);
        t.set_u32(s + 4, 0);
        let c = Call {
            ecx: 0x77,
            ..Call::default()
        };
        t.check_entry(f, c, &[0x00ab_cdef], Eax::Checked);
    }
}
