//! `magic_0445.dll` against its Rust ports (`ffx_magic::m0445`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 445;

/// 0x10006b00: state 0 before and from frame 90 (no and three model
/// nodes), state 1 with and without bit 2 (the nodes with and without a
/// parent), and another state.
#[test]
fn step() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6b00);
    rec(&mut t, 239, 3, |_, a| 0x0239_0000 ^ a[1]);
    t.set_u32(t.img.word(at + 0xb3), 0x00ab_00b3);
    let task = SCRATCH + 0x100;
    let fx = SCRATCH + 0x4000;
    let model = SCRATCH + 0x8000;
    let node = SCRATCH + 0x200;
    let (prev, next) = (SCRATCH + 0x280, SCRATCH + 0x2c0);
    let parent = SCRATCH + 0x300;
    for (state, frame, n, bits, with_parent) in [
        (0u32, 10u32, 3u32, 0u8, false),
        (0, 90, 0, 0, false),
        (0, 95, 3, 0, false),
        (1, 5, 0, 0, false),
        (1, 5, 0, 4, false),
        (1, 5, 0, 0xff, true),
        (7, 5, 0, 4, false),
    ] {
        t.write(fx, &[0x3c; 0x300]);
        t.set_u32(fx + 0x2d0, state);
        t.set_u32(fx + 0x2cc, frame);
        t.set_u32(fx + 0x2d4, model);
        t.set_u8(fx + 0x108, bits);
        for off in [0x120u32, 0x14c, 0x178, 0x2d8] {
            // the node's own links, and its parent (`+0x14`) or none
            t.set_u32(fx + off + 4, fx + off);
            t.set_u32(fx + off + 8, fx + off);
            t.set_u32(fx + off + 0x14, if with_parent { parent } else { 0 });
        }
        t.write(parent, &[0; 0x20]);
        t.set_u16(parent + 0x18, 4);
        t.set_u32(parent + 0x10, fx + 0x120);
        t.write(model, &[0x5d; 0x500]);
        t.set_u32(model + 0x4dc, n);
        t.set_u32(task, node);
        t.set_u32(task + 4, fx);
        t.set_u32(node + 4, prev);
        t.set_u32(node + 8, next);
        t.write(prev, &[0x11; 0x10]);
        t.write(next, &[0x22; 0x10]);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}
