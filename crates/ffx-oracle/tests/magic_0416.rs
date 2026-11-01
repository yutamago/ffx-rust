//! `magic_0416.dll` against its Rust ports (`ffx_magic::m0416`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 416;

/// 0x10001570: the count below, at and past 20.
#[test]
fn copies_step() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1570);
    for (s, n) in [(434u32, 2usize), (450, 2), (451, 2)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    t.set_u32(t.img.word(at + 0x82), 0x40);
    let obj = SCRATCH + 0x1000;
    for c in [0u32, 19, 20, 30] {
        t.set_u32(t.img.word(at + 0x05), c);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}
