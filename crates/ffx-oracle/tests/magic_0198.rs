//! `magic_0198.dll` against its Rust port (`ffx_magic::m0198`): the
//! effect's load. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, Twin};
use magic_owna::{rec, twin};

const N: u16 = 198;

/// 0x10001000: no, one and three of each kind.
#[test]
fn load() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1000);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 718, 1, |_, a| a[0] ^ 0x718);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 + 0x10 * a[1] + a[2]);
    rec(&mut t, 209, 2, |_, a| a[0] ^ 0x209);
    rec(&mut t, 208, 2, |_, a| a[0] ^ 0x208);
    rec(&mut t, 221, 4, |_, a| a[0] ^ 0x221);
    for (n1, n2) in [(0u16, 0u16), (1, 3), (3, 0)] {
        t.set_u16(w(&t, 0x1d), n1);
        t.set_u16(w(&t, 0x5e), n2);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
