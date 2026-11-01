//! `magic_0341.dll` against its Rust ports (`ffx_magic::m0341`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 341;

/// 0x10001170: the start-up (its words junk first).
#[test]
fn start_up() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1170);
    for (s, n) in [
        (803u32, 2usize),
        (861, 3),
        (259, 0),
        (260, 1),
        (718, 1),
        (222, 4),
        (221, 4),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (0x0100_0000 + s) ^ a.first().copied().unwrap_or(0)
        });
    }
    for op in [
        0x07, 0x25, 0x59, 0x6d, 0xc4, 0xce, 0xd8, 0xe2, 0xec, 0xf6, 0x100,
    ] {
        t.set_u32(t.img.word(at + op), 0x5a5a_5a5b);
    }
    t.check_entry(at, Call::default(), &[0x00a1_0000], Eax::Checked);
}

/// 0x10001280: the six parts freed and the buffer.
#[test]
fn free_parts() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1280);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 713, 1, |m, a| {
        // the pointer it was handed, then the part
        let p = m.r32(SCRATCH + 0x640);
        0x0713_0000 ^ a[0] ^ p
    });
    rec(&mut t, 804, 1, |_, a| 0x0804_0000 ^ a[0]);
    t.set_u32(w(&t, 0x01), SCRATCH + 0x640);
    t.set_u32(w(&t, 0xb5), 0x00bf_0000);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}
