//! `magic_0370.dll` against its Rust ports (`ffx_magic::m0370`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 370;

/// 0x10001720: the part's start (its words junk first).
#[test]
fn start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1720);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    let list = SCRATCH + 0x700;
    t.set_u32(list, 0x0900_0000);
    rec(&mut t, 941, 1, move |_, _| list);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    let gp = SCRATCH + 0x680;
    t.set_u32(w(&t, 0x0c), gp);
    t.set_u32(gp + 0x10, 0x00de_0010);
    t.set_u32(w(&t, 0x01), SCRATCH + 0x640);
    let sp = t.img.rel32_target(at + 0x31);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x6c0);
    for (k, src) in (0u32..).zip([0x39u32, 0x48]) {
        for i in 0..4 {
            t.set_u32(w(&t, src) + 4 * i, 0x0a00_0000 + 0x10 * k + i);
        }
    }
    for dst in [0x41u32, 0x5c] {
        t.write(w(&t, dst), &[0x5a; 0x10]);
    }
    for op in [0x51, 0x62, 0x6c, 0x76, 0x80, 0x8a, 0x94, 0x9e] {
        t.set_u32(w(&t, op), 0x5a5a_5a5b);
    }
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}
