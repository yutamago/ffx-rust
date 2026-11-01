//! `magic_0447.dll` against its Rust ports (`ffx_magic::m0447`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::twin;

const N: u16 = 447;

/// 0x10005430: the camera's quads copied.
#[test]
fn camera_copy() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x5430);
    let e = SCRATCH + 0x8000;
    t.set_u32(t.img.word(at + 0x02), e);
    let bytes: Vec<u8> = (0..0x240u32)
        .map(|i| (i * 7 + 3).to_le_bytes()[0])
        .collect();
    t.write(e + 0x5c0, &bytes);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}
