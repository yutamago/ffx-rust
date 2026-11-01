//! `magic_0269.dll` against its Rust ports (`ffx_magic::m0269`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax};
use magic_owna::{rec, twin};

const N: u16 = 269;

/// 0x100016c0: the sprite block (its words hold junk first).
#[test]
fn sprite_block() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x16c0);
    rec(&mut t, 562, 0, |_, _| 0x0562_0562);
    for op in [
        0x07, 0x18, 0x21, 0x29, 0x2f, 0x34, 0x39, 0x3f, 0x48, 0x4d, 0x52, 0x57, 0x5c, 0x62, 0x6b,
        0x70,
    ] {
        t.set_u32(t.img.word(at + op), 0x5a5a_5a5b);
    }
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}
