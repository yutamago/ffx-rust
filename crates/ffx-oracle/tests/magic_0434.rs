//! `magic_0434.dll` against its Rust ports (`ffx_magic::m0434`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 434;

/// 0x10005d10 (internal): the colour reset and show for two actors.
#[test]
fn colour_show() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x5d10);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    for (s, n) in [(357u32, 3usize), (356, 3), (362, 4), (399, 2)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let b = SCRATCH + 0x700;
    t.set_u32(w(&t, 0x01), b);
    t.set_u32(w(&t, 0x4c), 0x00ab_004c);
    for i in 0..4 {
        t.set_u32(w(&t, 0x11) + 4 * i, 0x0a00_0000 + i);
    }
    for i in [0u8, 3] {
        t.set_u8(b, i);
        t.check(
            at,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::m0434::colour_show,
        );
    }
}
