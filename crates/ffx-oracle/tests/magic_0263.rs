//! `magic_0263.dll` against its Rust port (`ffx_magic::m0263`): a
//! node's set-up. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::twin;

const N: u16 = 263;

/// 0x10006dc0: a node set up into an empty list and after another.
#[test]
fn node_setup() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6dc0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let root = w(&t, 0x2b);
    let old = SCRATCH + 0x800;
    for empty in [true, false] {
        t.set_u32(w(&t, 0x1e), if empty { root } else { old });
        t.write(old, &[0x21; 0x30]);
        let n = SCRATCH + 0x100;
        t.write(n, &[0x7e; 0x40]);
        t.check_entry(at, Call::this(n), &[0x00a0_00a0], Eax::Checked);
    }
}
