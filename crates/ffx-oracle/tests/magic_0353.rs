//! `magic_0353.dll` against its Rust ports (`ffx_magic::m0353`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 353;

/// 0x10001ae0: the model's motion reset.
#[test]
fn motion_reset() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1ae0);
    for (s, n) in [(357u32, 3usize), (356, 3), (362, 4), (399, 2)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    t.set_u32(t.img.word(at + 0x08), 0x00ab_0008);
    t.set_u32(t.img.word(at + 0x43), 0x00cd_0043);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}

/// 0x10002300: the countdown above, at and below 1 (the pool node given
/// back at the end), and values that round differently.
#[test]
fn fade_step() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2300);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 348, 6, |_, a| 0x0348_0000 ^ a[3] ^ (a[4] << 8));
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x07), fx);
    t.set_u32(w(&t, 0x6b), 0x00ab_006b);
    t.set_u32(w(&t, 0x5f), 0x00cd_005f);
    let node = SCRATCH + 0x3000;
    for n in [0x60i32, 2, 1, 0, -7, 0x51, 0x4f] {
        t.set_u32(fx + 0x13e4, n.cast_unsigned());
        t.write(node, &[0x4c; 0x20]);
        t.check_entry(at, Call::default(), &[node], Eax::Checked);
    }
}
