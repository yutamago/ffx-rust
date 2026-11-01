//! `magic_0251.dll` against its Rust ports (`ffx_magic::m0251`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, row, twin};

const N: u16 = 251;

/// 0x100024b0: the parts' programs for no, one and three parts (slot
/// 772's 64-bit result handed on), the lock restored.
#[test]
fn part_programs() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x24b0);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
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
    t.set_u32(w(&t, 0x53), lock);
    let recs = SCRATCH + 0x1000;
    for k in 0..4 {
        t.set_u32(recs + 0x68 * k, 0x00e0_0000 + k);
    }
    t.set_u32(w(&t, 0x0f), recs);
    for (n, old) in [(0i32, 0u32), (1, 0x77), (3, 1), (-1, 0)] {
        t.set_u32(w(&t, 0x08), n.cast_unsigned());
        t.set_u32(lock, old);
        let c = Call {
            eax: 0x00ea_00ea,
            ..Call::default()
        };
        t.check_entry(at, c, &[], Eax::Checked);
    }
}
