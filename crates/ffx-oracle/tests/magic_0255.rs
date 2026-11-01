//! `magic_0255.dll` against its Rust ports (`ffx_magic::m0255`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 255;

/// Slot 712 answers the case's word; the others record their words.
fn stubs(t: &mut Twin) {
    for (s, n) in [(258u32, 1usize), (231, 3), (230, 2), (237, 4)] {
        rec(t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    rec(t, 712, 2, |m, _| m.r32(SCRATCH + 0x6f0));
    rec(t, 261, 1, |m, _| m.r32(SCRATCH + 0x6f4));
}

/// 0x100010a0 and 0x10001230: with slot 261 / 712 answering 0 and not,
/// the lock set and clear before.
#[test]
fn part_draws() {
    let Some(mut t) = twin(N) else { return };
    stubs(&mut t);
    let obj = SCRATCH + 0x1000;
    for (rva, end) in [(0x10a0u32, 0x3bu32), (0x1230, 0x24)] {
        let at = t.at(rva);
        let lock = SCRATCH + 0x600;
        t.set_u32(t.img.word(at + end + 0x53), lock);
        t.set_u32(t.img.word(at + end + 0x46), SCRATCH + 0x640);
        for (k, (on, drawn, old)) in [(0u32, 0u32, 0u32), (1, 5, 0x77), (7, 0, 1)]
            .into_iter()
            .enumerate()
        {
            t.write(obj, &[0x5c; 0x60]);
            t.set_u32(lock, old);
            t.set_u32(SCRATCH + 0x6f0, drawn);
            t.set_u32(SCRATCH + 0x6f4, on);
            let c = Call {
                eax: 0x00ea_0000 + u32::try_from(k).unwrap(),
                ..Call::default()
            };
            t.check_entry(at, c, &[obj], Eax::Checked);
        }
    }
}
