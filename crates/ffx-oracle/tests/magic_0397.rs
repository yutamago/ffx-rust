//! `magic_0397.dll` against its Rust port (`ffx_magic::m0397`): the
//! task's end. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 397;

/// 0x10004250: paused, the script or the motion still running, and the
/// end (the node alone given back to the pool).
#[test]
fn script_end() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x4250);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let paused = SCRATCH + 0x600;
    rec(&mut t, 561, 0, move |m, _| m.r32(paused));
    rec(&mut t, 628, 1, |_, a| a[0] ^ 0x628);
    let e = SCRATCH + 0x8000;
    t.set_u32(w(&t, 0x07), e);
    let run = t.img.rel32_target(at + 0x0c);
    let ms = t.img.rel32_target(at + 0x25);
    let give = t.img.rel32_target(at + 0x54);
    let s = t.img.word(run + 0x1c);
    let node = SCRATCH + 0x2000;
    for (p, mode, done) in [(1u32, 0u8, true), (0, 2, true), (0, 0, false), (0, 0, true)] {
        t.set_u32(paused, p);
        // the script ended (mode 0, returns 1) or waiting for ever (2)
        t.set_u8(t.img.word(run + 0x01), mode);
        t.set_u16(s + 0x54, 0xffff);
        t.set_u32(s + 0x24, 0);
        t.set_u32(s + 0x28, 0);
        t.set_u32(t.img.word(ms + 0x02), u32::from(done));
        t.set_u16(t.img.word(ms + 0x11), 5);
        t.set_u32(e + 0x2408, 0x2408);
        // the node alone (no siblings, nothing chained) for the pool
        t.write(node, &[0; 0x1c]);
        t.set_u32(node + 8, node);
        for op in [0x48, 0x55, 0x64, 0x74] {
            t.set_u32(t.img.word(give + op), SCRATCH + 0x3000 + op);
        }
        t.set_u32(SCRATCH + 0x3074, SCRATCH + 0x3074);
        t.set_u32(SCRATCH + 0x3078, SCRATCH + 0x3074);
        t.set_u16(t.img.word(give + 0x8e), 7);
        t.check_entry(at, Call::default(), &[node], Eax::Checked);
    }
}
