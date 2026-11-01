//! `magic_0368.dll` against its Rust ports (`ffx_magic::m0368`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 368;

/// 0x10001070: slot 712 answering 0 and not, the flag set and clear.
#[test]
fn draw() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1070);
    for (s, n) in [(258u32, 1usize), (231, 3)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    // slot 712 also copies the flag (`SCRATCH + 0x600`) it sees out
    rec(&mut t, 712, 2, |m, _| {
        let seen = m.r32(SCRATCH + 0x600);
        m.w32(SCRATCH + 0x6e0, seen);
        m.r32(SCRATCH + 0x6f0)
    });
    let lock = SCRATCH + 0x600;
    t.set_u32(t.img.word(at + 0x5f), lock);
    t.set_u32(t.img.word(at + 0x52), SCRATCH + 0x640);
    for (k, v) in (0u32..).zip([0x15u32, 0x20, 0x2c, 0x34, 0x3f, 0x4b]) {
        t.set_u32(t.img.word(at + v), 0x00c0_0000 + k);
    }
    for (drawn, old) in [(0u32, 0u32), (5, 0x77), (0, 1)] {
        t.set_u32(lock, old);
        t.set_u32(SCRATCH + 0x6f0, drawn);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
