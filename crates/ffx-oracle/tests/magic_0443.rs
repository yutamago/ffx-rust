//! `magic_0443.dll` against its Rust ports (`ffx_magic::m0443`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 443;

/// 0x10001210: started (slot 411 at -1 and not), and before the start
/// the count and target pairs that start it and others.
#[test]
fn sound_step() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1210);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let answer = SCRATCH + 0x600;
    rec(&mut t, 411, 1, move |m, _| m.r32(answer));
    rec(&mut t, 228, 2, |_, a| 0x0228_0000 ^ a[0]);
    t.set_u32(w(&t, 0x4e), 0x00ab_004e);
    t.set_u32(w(&t, 0x2c), 0x00ab_002c);
    for (started, a, b, r) in [
        (1u32, 0u32, 0u32, u32::MAX),
        (2, 0, 0, 0),
        (1, 0, 0, 0xffff_fffe),
        (0, 1, 1, 0),
        (0, 1, 0, 0),
        (0, 1, 2, 0),
        (0, 5, 4, 0),
        (0, 5, 5, 0),
        (0, 0, u32::MAX, 0),
        (0, 2, 1, 0),
        (0, 2, 0, 0),
    ] {
        t.set_u32(w(&t, 0x02), started);
        t.set_u32(w(&t, 0x0a), a);
        t.set_u32(w(&t, 0x10), b);
        t.set_u32(answer, r);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
