//! `magic_0307.dll` against its Rust ports (`ffx_magic::m0307`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, row, twin};

const N: u16 = 307;

/// 0x10006e50: with and without the pool node, the part item on the
/// object list and not.
#[test]
fn end() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6e50);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 772, 2, |m, a| {
        let k = m.r32(row(772));
        0x0772_0000 ^ a[0] ^ (k << 8)
    });
    t.stub_edx(772, |_, a| 0x00ed_0000 ^ a[0]);
    rec(&mut t, 779, 4, |_, a| 0x0779_0000 ^ a[2]);
    rec(&mut t, 713, 1, |_, a| 0x0713_0000 ^ a[0]);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x09), fx);
    t.set_u32(w(&t, 0x31), SCRATCH + 0x2000);
    let lock = SCRATCH + 0x600;
    t.set_u32(w(&t, 0x49), lock);
    t.set_u32(w(&t, 0x8d), SCRATCH + 0x640);
    let list = w(&t, 0x7f);
    let item = w(&t, 0x7a);
    let node = SCRATCH + 0x3000;
    let lnode = SCRATCH + 0x3100;
    for (with_node, listed) in [(true, true), (false, true), (true, false), (false, false)] {
        t.write(node, &[0x4c; 0x20]);
        t.set_u32(fx + 0x2864, if with_node { node } else { 0 });
        t.set_u32(lock, 0x77);
        // the object list: one node holding the item, or none
        t.write(lnode, &[0x3c; 0x20]);
        t.set_u32(lnode, 0);
        t.set_u32(lnode + 4, 0);
        t.set_u32(lnode + 8, if listed { item } else { 0x1234 });
        t.set_u32(list, lnode);
        t.set_u32(list + 4, lnode);
        t.set_u16(w(&t, 0xb4), 0xffff);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10006f10: an actor index and -1.
#[test]
fn part_model() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6f10);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    for (s, n) in [
        (239u32, 3usize),
        (388, 3),
        (528, 2),
        (719, 3),
        (393, 3),
        (210, 2),
        (211, 2),
        (217, 2),
        (255, 2),
    ] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let bytes = SCRATCH + 0x700;
    t.write(bytes, &[0x31, 0x32, 0x33, 0x34]);
    for op in [0x2c, 0x5e, 0x88] {
        t.set_u32(w(&t, op), bytes);
    }
    t.set_u32(w(&t, 0x08), 0x00ab_0008);
    let recs = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x1e), recs);
    let obj = SCRATCH + 0x1000;
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
    for i in [2u32, 0xffff_ffff] {
        t.write(obj, &[0x5c; 0x130]);
        t.set_u32(obj + 0x120, i);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}
