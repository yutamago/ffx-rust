//! `magic_0380.dll` against its Rust ports (`ffx_magic::m0380`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::twin;

const N: u16 = 380;

/// 0x10003810: the task into an empty and a filled list.
#[test]
fn task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x3810);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let take = t.img.rel32_target(at + 0x6a);
    let (count, head) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
    let node = |i: u32| SCRATCH + 0x1800 + 0x40 * i;
    let last = SCRATCH + 0x1a00;
    for op in [
        0x07, 0x14, 0x1e, 0x28, 0x32, 0x3c, 0x46, 0x50, 0x5a, 0x5f, 0x65,
    ] {
        t.set_u32(w(&t, op), 0x5a5a_5a5b);
    }
    for first in [0u32, last] {
        for k in 0..2 {
            t.write(node(k), &[0x5a; 0x40]);
            t.set_u32(node(k), node(1 - k));
            t.set_u32(node(k) + 4, node(1 - k));
        }
        t.set_u32(head, node(0));
        t.set_u16(count, 2);
        t.write(last, &[0x66; 0x10]);
        t.set_u32(w(&t, 0x70), first);
        t.set_u32(w(&t, 0x87), first);
        t.check_entry(at, Call::default(), &[0x00a1_0000], Eax::Checked);
    }
}
