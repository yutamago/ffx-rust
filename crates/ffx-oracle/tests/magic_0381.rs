//! `magic_0381.dll` and `magic_0383.dll` against their Rust ports
//! (`ffx_magic::m0381`). Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

/// `magic_0381` 0x10001430: in and out of a battle, party members and
/// others for both actors.
#[test]
fn poses() {
    let Some(mut t) = twin(381) else { return };
    let at = t.at(0x1430);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    for (s, n) in [(541u32, 3usize), (38, 3), (525, 1)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let tb = SCRATCH + 0x700;
    t.set_u32(w(&t, 0x49), tb);
    let table = w(&t, 0x89);
    for i in 0..0x30 {
        t.set_u32(table + 4 * i, 0x0b00_0000 + i);
    }
    t.set_u32(w(&t, 0x42), 0x00ab_0042);
    for (battle, caster, target) in [
        (0u32, 3u32, 5u8),
        (1, 3, 5),
        (0, 0x40, 0x21),
        (0, 0x1f, 0x20),
    ] {
        t.set_u32(w(&t, 0x02), battle);
        t.set_u32(w(&t, 0x0a), caster);
        // the byte's neighbours junk (the port reads 8 bits)
        t.write(tb, &[target, 0x5a, 0x5a, 0x5a]);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0383` 0x100013a0: into an empty and a filled task list.
#[test]
fn start_up_0383() {
    let Some(mut t) = twin(383) else { return };
    let at = t.at(0x13a0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    for (s, n) in [(719u32, 3usize), (711, 3), (710, 4)] {
        rec(&mut t, s, n, move |_, a| (0x0700_0000 + s) ^ a[0]);
    }
    let pi = t.img.rel32_target(at + 0x33);
    t.set_u32(t.img.word(pi + 0x01), SCRATCH + 0x700);
    t.set_u32(t.img.word(pi + 0x07), SCRATCH + 0x740);
    t.set_u32(t.img.word(pi + 0x11), SCRATCH + 0x780);
    let pop = t.img.rel32_target(at + 0x5f);
    let (count, head) = (t.img.word(pop + 0x04), t.img.word(pop + 0x13));
    let (n1, n2) = (SCRATCH + 0x1800, SCRATCH + 0x1840);
    let last = SCRATCH + 0x1900;
    for op in [0x16, 0x20, 0x2e, 0x3c, 0x46, 0x51, 0x59] {
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
        t.set_u32(w(&t, 0x65), first);
        t.set_u32(w(&t, 0x79), first);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
