//! `magic_0387.dll` against its Rust ports (`ffx_magic::m0387`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::twin;

const N: u16 = 387;

/// The node under no parent, a parent with no child and one with a child.
fn cases(
    rva: u32,
    port: fn(&mut magic::PortGame, &ffx_magic::Image, &ffx_magic::Frame) -> ffx_magic::Ret,
) {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(rva);
    let r = SCRATCH + 0x100;
    let node = SCRATCH + 0x1000;
    let parent = SCRATCH + 0x2000;
    let kid = SCRATCH + 0x2800;
    for case in 0..3 {
        t.write(node, &[0x5c; 0x400]);
        t.write(parent, &[0x3c; 0x40]);
        t.set_u16(parent + 0x18, 5);
        t.write(kid, &[0x2c; 0x40]);
        t.set_u32(parent + 0x10, if case == 2 { kid } else { parent });
        t.write(r, &[0x7a; 0x20]);
        t.set_u32(r, node);
        t.set_u32(r + 4, if case == 0 { 0 } else { parent });
        t.set_u32(r + 8, 0x00a0_0008);
        t.set_u32(r + 0xc, 0x00b0_000c);
        t.set_u32(r + 0x10, 0x00c0_0010);
        t.check(at, Call::this(r), &[], Eax::Checked, port);
    }
}

/// 0x10003c70.
#[test]
fn sprite_record() {
    cases(0x3c70, ffx_magic::m0387::sprite_record);
}

/// 0x100048b0.
#[test]
fn strip_record() {
    cases(0x48b0, ffx_magic::m0387::strip_record);
}
