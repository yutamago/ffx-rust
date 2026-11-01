//! `magic_0371.dll` against its Rust ports (`ffx_magic::m0371`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax};
use magic_owna::twin;

const N: u16 = 371;

/// 0x10002cb0 (internal): the block over junk.
#[test]
fn block_reset() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2cb0);
    let block = t.img.word(at + 0x08);
    t.write(block.wrapping_sub(0x10), &[0x6b; 0x130]);
    t.check(
        at,
        Call::default(),
        &[],
        Eax::Checked,
        ffx_magic::m0371::block_reset,
    );
}
