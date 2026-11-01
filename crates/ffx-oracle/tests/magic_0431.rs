//! `magic_0431.dll` against its Rust ports (`ffx_magic::m0431`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 431;

/// 0x10001620: each of the four answers not 0 in turn, and all 0.
#[test]
fn sounds_ready() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1620);
    let answers = SCRATCH + 0x600;
    for (s, n) in [(5u32, 1usize), (28, 2)] {
        rec(&mut t, s, n, move |m, _| {
            // the k-th call (both slots counted together) answers word k
            let k = m.r32(answers + 0x20);
            m.w32(answers + 0x20, k + 1);
            m.r32(answers + 4 * k)
        });
    }
    for bad in [0u32, 1, 2, 3, 4] {
        for k in 0..4 {
            t.set_u32(answers + 4 * k, u32::from(k == bad) * (0x10 + k));
        }
        t.set_u32(answers + 0x20, 0);
        t.check(
            at,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::m0431::sounds_ready,
        );
    }
}

/// 0x10001a10: the colour reset for two actors.
#[test]
fn colour_reset() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1a10);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    for (s, n) in [(357u32, 3usize), (356, 3), (362, 4)] {
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
            ffx_magic::m0431::colour_reset,
        );
    }
}
