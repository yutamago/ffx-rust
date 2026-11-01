//! The summon start family (`ffx_magic::r9_summon`) against each copy's
//! original: two actors, the lock word and the DLL's start-up after it
//! (set up as `magic_0076.rs`/`magic_0078.rs` do). Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

/// What the start-up needs: its tail word (`+0x07`), the flag bytes
/// (`flag`), the root list (`last`), the particle list (`part`, `0` for
/// none) and the motion block (`a`, `0` for none).
#[derive(Clone, Copy)]
struct Start {
    flag: u32,
    last: u32,
    root: u32,
    part: (u32, u32),
    a: u32,
}

fn copy(n: u16, rva: u32, s: Start) {
    copy_at(n, rva, s, (0x195, 0xdb));
}

/// [`copy`] with the start-up's `rel32` and the lock's operand at `ops`.
fn copy_at(n: u16, rva: u32, s: Start, (call, lock_op): (u32, u32)) {
    let Some(mut t) = twin(n) else { return };
    let at = t.at(rva);
    let w = |t: &Twin, o: u32| t.img.word(at + o);
    // the summon start's own slots (711 copies the lock out)
    let lock = SCRATCH + 0x1f0;
    for (slot, k) in [
        (718u32, 1usize),
        (719, 3),
        (710, 4),
        (222, 4),
        (221, 4),
        (551, 3),
    ] {
        rec(&mut t, slot, k, move |_, a| (0x0700_0000 + slot) ^ a[0]);
    }
    rec(&mut t, 711, 3, move |m, a| {
        m.w32(SCRATCH + 0x1f4, m.r32(lock));
        0x0711_0000 ^ a[0]
    });
    // the targets' slots and the start-up's
    for (slot, k) in [
        (168u32, 1usize),
        (531, 2),
        (268, 3),
        (413, 1),
        (215, 1),
        (14, 2),
        (325, 1),
    ] {
        rec(&mut t, slot, k, move |_, a| (0x0100_0000 + slot) ^ a[0]);
    }
    t.stub_st0(289, |_, _| -1.25);
    rec(&mut t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.5);
    t.stub_st0(790, |_, _| 0.25);
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) + 0.75);
    t.stub(941, |_, _| SCRATCH + 0x400);
    t.stub(558, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 3 + i);
        }
        0x558
    });
    for (slot, k) in [(331u32, 1usize), (353, 1), (271, 1), (714, 1)] {
        rec(&mut t, slot, k, move |_, a| (0x0200_0000 + slot) ^ a[0]);
    }
    t.stub(390, |_, _| 0x1071);
    t.stub(534, |_, _| 1);
    for k in 0..4 {
        t.set_u32(SCRATCH + 0x400 + 4 * k, 0x0f00_0000 + k);
    }
    let start = t.img.rel32_target(at + call);
    let sw = |t: &Twin, o: u32| t.img.word(start + o);
    if s.part.0 != 0 {
        let pn = t.img.rel32_target(start + s.part.1);
        t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x700);
    }
    t.set_u32(w(&t, lock_op), lock);
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
    let (tail, other) = (SCRATCH + 0x500, SCRATCH + 0x600);
    let root = sw(&t, s.root);
    if s.a != 0 {
        t.set_u32(sw(&t, s.a), SCRATCH + 0x680);
        t.write(SCRATCH + 0x680, &[0x31; 0x40]);
    }
    for (actor, lk, filled) in [(3u32, 0u32, false), (0x21, 7, true)] {
        t.set_u32(lock, lk);
        t.write(tail, &[0x55; 0x10]);
        t.write(other, &[0x66; 0x20]);
        t.set_u32(sw(&t, 0x07), tail);
        t.write(sw(&t, s.flag), &[0, 0x40]);
        t.set_u32(sw(&t, s.last), if filled { other } else { root });
        t.set_u32(root + 0x10, if filled { other } else { root });
        if s.part.0 != 0 {
            let parts = sw(&t, s.part.0);
            t.set_u32(parts + 0x10, parts);
        }
        if call == 0x195 {
            for o in [
                0x17u32, 0x21, 0x2b, 0x35, 0x3f, 0x49, 0x54, 0x5c, 0x66, 0x70, 0x7a, 0x85, 0x8d,
                0x97, 0xa1, 0xb6, 0xe7,
            ] {
                t.set_u32(w(&t, o), 0x5a5a_5a5b);
            }
        } else {
            for o in [
                0x21u32, 0x2b, 0x35, 0x3f, 0x49, 0x53, 0x5d, 0x68, 0x70, 0x7a, 0x85, 0x8d, 0x97,
                0xa1, 0xac, 0xb4, 0xbe, 0xc8, 0xd2, 0xdc, 0xe6, 0xf0, 0xfa, 0x101, 0x10c, 0x114,
                0x11e, 0x128, 0x132, 0x13c, 0x14e, 0x15a, 0x16b, 0x175, 0x17f, 0x189, 0x193, 0x19e,
                0x1a6, 0x1b0, 0x1ba, 0x1c4, 0x1ce, 0x1e0, 0x1ec, 0x1f8, 0x207, 0x211, 0x21b, 0x225,
                0x22f, 0x239, 0x242, 0x247, 0x24c, 0x2be,
            ] {
                t.set_u32(w(&t, o), 0x5a5a_5a5b);
            }
            // the count and the flag byte
            t.set_u16(w(&t, 0x14), 0x0123);
            t.set_u8(w(&t, 0x1fd), 0x45 + u8::from(filled));
        }
        t.check_entry(at, Call::default(), &[actor], Eax::Checked);
    }
}

#[test]
fn start_0077() {
    let s = Start {
        flag: 0x42,
        last: 0x7f,
        root: 0xba,
        part: (0, 0),
        a: 0,
    };
    copy(77, 0x69d0, s);
}

#[test]
fn start_0078() {
    let s = Start {
        flag: 0x51,
        last: 0x91,
        root: 0x76,
        part: (0x204, 0x22b),
        a: 0x1e6,
    };
    copy(78, 0x67b0, s);
}

#[test]
fn start_0079() {
    let s = Start {
        flag: 0x51,
        last: 0xab,
        root: 0x76,
        part: (0x204, 0x22b),
        a: 0x1f3,
    };
    copy(79, 0x67b0, s);
}

#[test]
fn start_0080() {
    let s = Start {
        flag: 0x51,
        last: 0x92,
        root: 0x77,
        part: (0x2b2, 0x2d2),
        a: 0x2a1,
    };
    copy(80, 0x6880, s);
}

#[test]
fn start_0081() {
    let s = Start {
        flag: 0x51,
        last: 0x91,
        root: 0x76,
        part: (0x203, 0x22a),
        a: 0x1f2,
    };
    copy(81, 0x67b0, s);
}

/// The node-graph copies: `(dll, address, the start-up's Start)`.
fn nodes(n: u16, rva: u32, s: Start) {
    copy_at(n, rva, s, (0x36c, 0x2b2));
}

const S83: Start = Start {
    flag: 0x51,
    last: 0x91,
    root: 0x76,
    part: (0x203, 0x22a),
    a: 0x1f2,
};

#[test]
fn nodes_0083() {
    nodes(83, 0x6c50, S83);
}

#[test]
fn nodes_0097() {
    nodes(97, 0x6c60, S83);
}

#[test]
fn nodes_0091() {
    nodes(91, 0x6c70, S83);
}

#[test]
fn nodes_0094() {
    let s = Start {
        flag: 0x51,
        last: 0x92,
        root: 0x77,
        part: (0x368, 0x388),
        a: 0x357,
    };
    nodes(94, 0x7030, s);
}

#[test]
fn nodes_0095() {
    let s = Start {
        flag: 0x51,
        last: 0x91,
        root: 0x76,
        part: (0x1e9, 0x210),
        a: 0x1d8,
    };
    nodes(95, 0x6ec0, s);
}

#[test]
fn nodes_0096() {
    let s = Start {
        flag: 0x51,
        last: 0x92,
        root: 0x77,
        part: (0x2b2, 0x2d2),
        a: 0x2a1,
    };
    nodes(96, 0x6d30, s);
}
