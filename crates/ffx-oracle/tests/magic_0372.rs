//! `magic_0372.dll` against its Rust ports (`ffx_magic::m0372`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 372;

/// 0x10001000: the part start for two heights.
#[test]
fn part_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1000);
    let list = SCRATCH + 0x700;
    for i in 0..4 {
        t.set_u32(list + 4 * i, 0x0900_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| list);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    rec(&mut t, 410, 2, |m, a| {
        let v = m.r32(a[1] + 4);
        m.w32(a[1] + 4, v ^ 0x0041_0000);
        0x0410_0000 ^ a[0]
    });
    let gp = SCRATCH + 0x680;
    t.set_u32(t.img.word(at + 0x10), gp);
    t.set_u32(gp + 0x10, 0x00de_0010);
    t.set_u32(t.img.word(at + 0x04), SCRATCH + 0x640);
    t.set_u32(t.img.word(at + 0x45), 0x00ab_0045);
    let sp = t.img.rel32_target(at + 0x36);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x6c0);
    let obj = SCRATCH + 0x1000;
    for v in [1.5f32, -0.25] {
        t.write(obj, &[0x5c; 0x80]);
        t.set_u32(obj + 0x44, v.to_bits());
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}
