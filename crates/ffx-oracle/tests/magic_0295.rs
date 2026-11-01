//! `magic_0295.dll` against its Rust ports (`ffx_magic::m0295`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::m0295;
use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::twin;

const N: u16 = 295;

/// 0x100074e0 (internal, `ecx` = the node): under the static parent
/// with no children and after another child.
#[test]
fn node_under() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x74e0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let parent = w(&t, 0x1b);
    let other = SCRATCH + 0x600;
    let n = SCRATCH + 0x100;
    t.set_u32(w(&t, 0x02), 0x00ab_0002);
    for last in [parent, other] {
        t.write(n, &[0x5c; 0x40]);
        t.write(other, &[0x66; 0x10]);
        t.set_u32(w(&t, 0x27), last);
        t.set_u16(w(&t, 0x48), 3);
        t.check(at, Call::this(n), &[], Eax::Checked, m0295::node_under);
    }
}
