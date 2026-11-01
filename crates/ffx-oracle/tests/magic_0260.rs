//! `magic_0260.dll` against its Rust ports (`ffx_magic::m0260`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::m0260;
use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, row, twin};

const N: u16 = 260;

/// 0x100021f0 (internal, `ecx` = the effect): the five blocks' programs
/// (slot 772's 64-bit result handed on), the lock restored.
#[test]
fn part_programs() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x21f0);
    rec(&mut t, 772, 2, |m, a| {
        let k = m.r32(row(772));
        0x0772_0000 ^ a[1] ^ (k << 8)
    });
    t.stub_edx(772, |_, a| 0x00ed_0000 ^ a[0]);
    let lock = SCRATCH + 0x600;
    // slot 779 copies the lock it sees out
    rec(&mut t, 779, 4, move |m, a| {
        let seen = m.r32(lock);
        m.w32(lock + 0x40, seen);
        0x0779_0000 ^ a[3]
    });
    t.set_u32(t.img.word(at + 0x3f), lock);
    let fx = SCRATCH + 0x1_0000;
    for old in [0u32, 0x77] {
        t.write(fx + 0x354, &[0x33; 0x10]);
        t.set_u32(lock, old);
        let c = Call {
            eax: 0x00ea_00ea,
            ecx: fx,
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, m0260::part_programs);
    }
}
