//! `magic_0375.dll` against its Rust ports (`ffx_magic::m0375`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 375;

/// 0x10001bc0: the objects in slots 0, 1 and 2 of the block.
#[test]
fn part_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1bc0);
    let list = SCRATCH + 0x700;
    t.set_u32(list, 0x0900_0000);
    rec(&mut t, 941, 1, move |_, _| list);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    let gp = SCRATCH + 0x680;
    t.set_u32(t.img.word(at + 0x10), gp);
    t.set_u32(gp + 0x10, 0x00de_0010);
    t.set_u32(t.img.word(at + 0x04), SCRATCH + 0x640);
    let sp = t.img.rel32_target(at + 0x35);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x6c0);
    let block = t.img.word(at + 0x3c);
    for i in 0..3 {
        let obj = block + 0x80 * i;
        t.write(obj, &[0x5c; 0x80]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}
