//! `magic_0379.dll` against its Rust ports (`ffx_magic::m0379`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::twin;

const N: u16 = 379;

/// 0x10006a30 (internal, `ecx` = the node): under the static parent with
/// no children and after another child.
#[test]
fn node_under() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6a30);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let parent = w(&t, 0x18);
    let other = SCRATCH + 0x600;
    let n = SCRATCH + 0x100;
    for last in [parent, other] {
        t.write(n, &[0x5c; 0x40]);
        t.write(other, &[0x66; 0x10]);
        t.set_u32(w(&t, 0x24), last);
        t.set_u16(w(&t, 0x45), 3);
        t.check(
            at,
            Call::this(n),
            &[0x00a1_0000],
            Eax::Checked,
            ffx_magic::m0379::node_under,
        );
    }
}
