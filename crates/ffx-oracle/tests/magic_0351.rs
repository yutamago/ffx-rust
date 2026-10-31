//! `magic_0351.dll` against its Rust port (`ffx_magic::m0351`): the
//! sounds. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 351;

/// 0x10002770: stopped, early, and the sounds for counts 0, 2 and 5.
#[test]
fn sounds_3() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2770);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 239, 3, |_, a| a[1] ^ 0x239);
    let bytes = SCRATCH + 0x300;
    for i in 0u8..4 {
        t.set_u8(bytes + u32::from(i), 0x60 + i);
    }
    t.set_u32(w(&t, 0x35), bytes);
    t.set_u32(w(&t, 0x47), 0x0047_0047);
    for (k, f) in (0u32..).zip([1u32, 0, 1, 1]) {
        t.set_u32(w(&t, 0x2d) + 4 * k, f);
    }
    for (stop, clock, count) in [
        (1u32, 0x40u32, 3u32),
        (0, 0x27, 3),
        (0, 0x28, 0),
        (0, 0x30, 2),
        (0, 0x50, 5),
    ] {
        t.set_u32(w(&t, 0x02), stop);
        t.set_u32(w(&t, 0x0b), clock);
        t.set_u32(w(&t, 0x21), count);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
