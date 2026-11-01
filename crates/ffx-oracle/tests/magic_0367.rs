//! `magic_0367.dll` and `magic_0369.dll` against their Rust ports
//! (`ffx_magic::m0367`). Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

/// The slots the start-ups call and the part's set-up globals.
fn setup(t: &mut Twin, at: u32, part: u32) {
    for (s, n) in [(719u32, 3usize), (711, 3), (710, 4), (413, 1), (215, 1)] {
        rec(t, s, n, move |_, a| (0x0700_0000 + s) ^ a[0]);
    }
    let pi = t.img.rel32_target(at + part);
    t.set_u32(t.img.word(pi + 0x01), SCRATCH + 0x700);
    t.set_u32(t.img.word(pi + 0x07), SCRATCH + 0x740);
    t.set_u32(t.img.word(pi + 0x11), SCRATCH + 0x780);
}

/// `magic_0367` 0x100019b0: into an empty and a filled task list.
#[test]
fn start_up_0367() {
    let Some(mut t) = twin(367) else { return };
    let at = t.at(0x19b0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    setup(&mut t, at, 0x56);
    let take = t.img.rel32_target(at + 0x87);
    let (count, head) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
    let node = |i: u32| SCRATCH + 0x1800 + 0x40 * i;
    let last = SCRATCH + 0x1a00;
    for op in [0x02, 0x0c, 0x16, 0x20, 0x2a, 0x51, 0x64, 0x6e, 0x79, 0x81] {
        t.set_u32(w(&t, op), 0x5a5a_5a5b);
    }
    for head0 in [0u32, last, 0] {
        for i in 0..4 {
            t.write(node(i), &[0x5a; 0x40]);
            t.set_u32(node(i), node((i + 3) % 4));
            t.set_u32(node(i) + 4, node((i + 1) % 4));
        }
        t.set_u32(head, node(0));
        t.set_u16(count, 4);
        t.write(last, &[0x66; 0x10]);
        t.set_u32(w(&t, 0x8d), head0);
        t.set_u32(w(&t, 0xa1), head0);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0369` 0x10001450: into an empty and a filled task list.
#[test]
fn start_up_0369() {
    let Some(mut t) = twin(369) else { return };
    let at = t.at(0x1450);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    setup(&mut t, at, 0x4c);
    let pop = t.img.rel32_target(at + 0x78);
    let (count, head) = (t.img.word(pop + 0x04), t.img.word(pop + 0x13));
    let (n1, n2) = (SCRATCH + 0x1800, SCRATCH + 0x1840);
    let last = SCRATCH + 0x1900;
    for op in [0x02, 0x0c, 0x16, 0x20, 0x47, 0x55, 0x5f, 0x6a, 0x72] {
        t.set_u32(w(&t, op), 0x5a5a_5a5b);
    }
    for first in [0u32, last] {
        t.write(n1, &[0x5a; 0x80]);
        t.set_u32(n1, n2);
        t.set_u32(n1 + 4, n2);
        t.set_u32(n2, n1);
        t.set_u32(n2 + 4, n1);
        t.set_u32(head, n1);
        t.set_u16(count, 2);
        t.write(last, &[0x66; 0x10]);
        t.set_u32(w(&t, 0x7e), first);
        t.set_u32(w(&t, 0x8e), first);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
