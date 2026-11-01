//! `magic_0430.dll` against its Rust ports (`ffx_magic::m0430`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 430;

/// A pool of four nodes in a ring for `take` (called at `call`), the
/// task list's first and last words (operands `first`, `last`) set to
/// `head`.
fn pool(t: &mut Twin, call: u32, first: u32, last: u32, head: u32) {
    let take = t.img.rel32_target(call);
    let (count, ring) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
    let node = |i: u32| SCRATCH + 0x1800 + 0x40 * i;
    for k in 0..4 {
        t.write(node(k), &[0x5a; 0x40]);
        t.set_u32(node(k), node((k + 3) % 4));
        t.set_u32(node(k) + 4, node((k + 1) % 4));
    }
    t.set_u32(ring, node(0));
    t.set_u16(count, 4);
    t.set_u32(first, head);
    t.set_u32(last, head);
}

/// 0x100030d0: the start into an empty and a filled task list.
#[test]
fn start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x30d0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let e = SCRATCH + 0x8000;
    t.set_u32(w(&t, 0x03), e);
    let last = SCRATCH + 0x1a00;
    for head in [0u32, last] {
        t.write(e + 0xbe0, &[0x44; 0x300]);
        t.write(last, &[0x66; 0x10]);
        let (first, tail) = (w(&t, 0x6d), w(&t, 0x7d));
        pool(&mut t, at + 0x67, first, tail, head);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10003cf0: the count below, at and past 7 and 20 (the node given
/// back from 19 on).
#[test]
fn fade_step() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x3cf0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 38, 3, |_, a| 0x0038_0000 ^ a[1]);
    t.set_u32(w(&t, 0x2a), 0x00ab_002a);
    let give = t.img.rel32_target(at + 0x73);
    let node = SCRATCH + 0x2000;
    for n in [0u32, 7, 8, 12, 18, 19, 25, 0x8000_0000] {
        t.set_u32(w(&t, 0x04), n);
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

/// 0x10003d70: the fade's start into an empty and a filled task list.
#[test]
fn fade_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x3d70);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 38, 3, |_, a| 0x0038_0000 ^ a[1]);
    t.set_u32(w(&t, 0x17), 0x00ab_0017);
    let last = SCRATCH + 0x1a00;
    for head in [0u32, last] {
        t.write(last, &[0x66; 0x10]);
        let (first, tail) = (w(&t, 0x2c), w(&t, 0x40));
        pool(&mut t, at + 0x26, first, tail, head);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
