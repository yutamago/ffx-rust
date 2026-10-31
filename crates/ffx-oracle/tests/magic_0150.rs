//! `magic_0150.dll` against its Rust port (`ffx_magic::m0150`): the
//! start-ups and the loading frame. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, recs, twin};

const N: u16 = 150;

/// 0x1000cb70: into an empty and a filled root list.
#[test]
fn start_up() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0xcb70);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    t.stub_st0(790, |_, _| 0.25);
    t.stub(558, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 3 + i);
        }
        0x558
    });
    recs(&mut t, &[(331, 1), (353, 1), (271, 1)]);
    let (tail, other) = (SCRATCH + 0x500, SCRATCH + 0x600);
    let root = w(&t, 0xa0);
    for filled in [false, true] {
        t.write(tail, &[0x55; 0x10]);
        t.write(other, &[0x66; 0x20]);
        t.set_u32(w(&t, 0x04), tail);
        t.set_u32(w(&t, 0x3d), if filled { other } else { root });
        t.set_u16(w(&t, 0xbc), 3);
        for op in [0x09, 0x23, 0x43, 0x57, 0xcf, 0xd6] {
            t.set_u32(w(&t, op), 0x5a5a_5a5b);
        }
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x1000de60: each loading step waiting and passing, then the frame
/// with and without children, with the flag set.
#[test]
fn load_frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0xde60);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let answers = SCRATCH + 0x700;
    for (k, s) in (0u32..).zip([225u32, 313, 549, 226, 673]) {
        rec(&mut t, s, 2, move |m, _| m.r32(answers + 4 * k));
    }
    let first = w(&t, 0xb0);
    let root = w(&t, 0xcc);
    t.stub(1000, move |m, a| {
        let k = m.r32(SCRATCH + 0x37fc).wrapping_add(1);
        m.w32(SCRATCH + 0x37fc, k);
        m.w32(SCRATCH + 0x3780 + 4 * (k % 16), m.r32(a[0]));
        m.w32(SCRATCH + 0x37c0 + 4 * (k % 16), m.r32(a[0] + 4));
        if m.r32(m.r32(a[0]) + 8) == root {
            m.w32(first, root);
        }
        0x1000
    });
    let vt = SCRATCH + 0x800;
    t.set_u32(magic::TABLE + 4 * 1000, magic::TRAPS + 16 * 1000);
    t.set_u32(vt + 0x20, magic::TRAPS + 16 * 1000);
    t.set_u32(w(&t, 0x99), SCRATCH + 0x400);
    let kid = |i: u32| SCRATCH + 0x1000 + 0x100 * i;
    let base = t.save();
    for (step, ans, flag, kids) in [
        (0u32, [1u32, 0, 0], 0u32, 0u32),
        (0, [0, 1, 0], 0, 0),
        (0, [0, 0, 1], 0, 0),
        (0, [0, 0, 0], 0, 2),
        (1, [0, 0, 0], 1, 2),
        (2, [0, 0, 0], 0, 1),
        (3, [9, 9, 9], 0, 0),
        (7, [9, 9, 9], 1, 0),
    ] {
        t.restore(&base);
        t.set_u32(w(&t, 0x04), step);
        for (k, v) in (0u32..).zip(ans) {
            t.set_u32(answers + 4 * k, v);
        }
        t.set_u32(w(&t, 0xb8), flag);
        t.set_u8(w(&t, 0x9f), 0x41);
        t.set_u32(w(&t, 0x71), 0x0071_0000);
        // the word copied to +0xc4 and the flags whose bit 0 it sets
        t.set_u32(w(&t, 0xaa), 0x00aa_0001);
        t.set_u32(w(&t, 0xc4), 0x5a5a_5a5b);
        t.set_u32(w(&t, 0x107), 0x10);
        for i in 0..kids {
            t.write(kid(i), &[0x22; 0x40]);
            t.set_u32(kid(i), vt);
            t.set_u32(kid(i) + 8, if i + 1 == kids { root } else { kid(i + 1) });
        }
        t.set_u32(first, if kids == 0 { root } else { kid(0) });
        for at in [w(&t, 0x113), w(&t, 0x12e)] {
            t.set_u32(at + 0x10, at);
            t.set_u8(at + 0x1a, 0);
            t.set_u8(at + 0x1b, 0);
        }
        // the cursor's junk word (the port's 0)
        t.write(magic::STACK_TOP - 0x2000, &[0; 0x1f00]);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x1000d070: slot 294 answering 0xff and another number, into an
/// empty and a filled root list.
#[test]
fn model_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0xd070);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let n = SCRATCH + 0x700;
    rec(&mut t, 294, 1, move |m, a| {
        let k = m.r32(SCRATCH + 0x704).wrapping_add(1);
        m.w32(SCRATCH + 0x704, k);
        if k == 1 { m.r32(n) } else { a[0] & 0xff }
    });
    recs(&mut t, &[(413, 1), (215, 1), (14, 2)]);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.5);
    let (tail, other) = (SCRATCH + 0x500, SCRATCH + 0x600);
    let root = w(&t, 0x93);
    t.set_u32(w(&t, 0xba), 0x00ba_0007);
    t.set_u32(w(&t, 0x176), 0x0176_0009);
    t.set_u8(w(&t, 0x1a1), 3);
    // the model nodes' parent: an empty list
    let parent = w(&t, 0x15b);
    t.set_u32(parent + 0x10, parent);
    for (num, filled) in [(0xffu32, false), (5, true), (0xff, true)] {
        t.set_u32(n, num);
        t.set_u32(SCRATCH + 0x704, 0);
        t.write(tail, &[0x55; 0x10]);
        t.write(other, &[0x66; 0x20]);
        t.set_u32(w(&t, 0x04), tail);
        t.set_u32(w(&t, 0x3d), if filled { other } else { root });
        t.set_u32(root + 0x10, if filled { other } else { root });
        // the words it stores hold junk (each store must show)
        for op in [
            0x43, 0x57, 0x5f, 0xde, 0xe8, 0xf3, 0xfb, 0x10c, 0x116, 0x120,
        ] {
            t.set_u32(w(&t, op), 0x5a5a_5a5b);
        }
        t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
