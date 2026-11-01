//! `magic_0348.dll` against its Rust ports (`ffx_magic::m0348`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 348;

/// 0x10006ca0: the three tasks into an empty and a filled list.
#[test]
fn tasks() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6ca0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 619, 1, |_, a| 0x0619_0000 ^ a[0]);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x13), fx);
    let take = t.img.rel32_target(at + 0x80);
    let (count, head) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
    let node = |i: u32| SCRATCH + 0x1800 + 0x40 * i;
    let last = SCRATCH + 0x1a00;
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
    for first in [0u32, last] {
        for k in 0..4 {
            t.write(node(k), &[0x5a; 0x40]);
            t.set_u32(node(k), node((k + 3) % 4));
            t.set_u32(node(k) + 4, node((k + 1) % 4));
        }
        t.set_u32(head, node(0));
        t.set_u16(count, 4);
        t.write(last, &[0x66; 0x10]);
        t.set_u32(w(&t, 0x86), first);
        t.set_u32(w(&t, 0x9c), first);
        t.write(fx + 0xcd80, &[0x44; 0x20]);
        t.write(fx + 0xd170, &[0x44; 0x10]);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
