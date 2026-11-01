//! `magic_0417.dll` against its Rust ports (`ffx_magic::m0417`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::twin;

const N: u16 = 417;

/// 0x10004430: a free record at the rover, one further on (wrapping),
/// none free, and no records.
#[test]
fn take_record() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x4430);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let base = SCRATCH + 0x1000;
    t.set_u32(w(&t, 0x17), base);
    // two free records (`free`, `free2`; 9 none), so where the search
    // starts shows
    for (n, rover, free, free2) in [
        (4u32, 1u32, 1u32, 9u32),
        (4, 2, 0, 9),
        (4, 0, 9, 9),
        (0, 0, 0, 9),
        (3, 2, 2, 9),
        (4, 2, 0, 2),
        (4, 3, 1, 3),
    ] {
        for i in 0..4 {
            t.write(base + 0x7c * i, &[0x5a; 0x7c]);
            t.set_u32(base + 0x7c * i + 0x78, u32::from(i != free && i != free2));
        }
        t.set_u32(w(&t, 0x0a), n);
        t.set_u32(w(&t, 0x02), rover);
        t.check(
            at,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::m0417::take_record,
        );
    }
}
