//! Small own functions below `magic_0450` (agent magic-small-lo), batch
//! 6: `magic_0351`'s record parts. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, STACK_TOP, Twin};
use magic_owna::{rec, twin};

/// Junk under the entry on both sides.
fn dirty(t: &mut Twin) {
    t.write(STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
}

/// `m0351::part_restart` (0x10001c40) and `part_register` (0x10001d40):
/// a record past the count, one without an entry, one with (slot 712
/// answering 0 and not).
#[test]
fn record_parts_0351() {
    let Some(mut t) = twin(351) else { return };
    let (rs, rg) = (t.at(0x1c40), t.at(0x1d40));
    for (s, n) in [(239u32, 3usize), (258, 1), (231, 4), (230, 6), (941, 2)] {
        rec(&mut t, s, n, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    let table = SCRATCH + 0x400;
    for i in 0..4 {
        t.set_u32(table + 4 * i, 0x0900_0000 + i);
    }
    rec(&mut t, 941, 2, move |_, _| table);
    rec(&mut t, 215, 1, |_, a| 0x0215_0000 ^ a[0]);
    rec(&mut t, 268, 3, |_, a| a[0] ^ a[2]);
    let answer = SCRATCH + 0x20;
    rec(&mut t, 712, 2, move |m, _| m.r32(answer));
    rec(&mut t, 714, 5, |_, a| a[2] ^ 0x714);
    let sp = t.img.rel32_target(rs + 0x97);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(sp + 0x29), SCRATCH + 0x200);
    let lock = SCRATCH + 0x210;
    t.set_u32(t.img.word(rg + 0x9c), lock);
    t.set_u32(lock, 0x77);
    let h = SCRATCH + 0x300;
    t.set_u32(t.img.word(rs + 0x73), h);
    t.set_u32(h + 0x10, 0x00d0_0010);
    let bytes = SCRATCH + 0x600;
    t.write(bytes, &[3, 5, 7, 9]);
    t.set_u32(t.img.word(rs + 0x29), bytes);
    t.set_u32(t.img.word(rs + 0x4), 3);
    let base = t.img.word(rs + 0x10);
    // the part pointer global, and the entry table's neighbours empty
    t.set_u32(t.img.word(rg + 0x8f), SCRATCH + 0x240);
    for i in 0..8 {
        t.set_u32(t.img.word(rs + 0x23) + 4 * i, 0);
    }
    for (i, entry, ans) in [
        (5u32, 0x1_0000u32, 0u32),
        (1, 0, 0),
        (2, 0x1_0000, 0),
        (2, 0x1_0000, 7),
    ] {
        let a = base + 0x80 * i;
        t.write(a, &[0x44; 0x80]);
        t.set_u32(t.img.word(rs + 0x23) + 4 * i.min(3), entry);
        t.set_u32(answer, ans);
        for at in [rs, rg] {
            dirty(&mut t);
            t.check_entry(at, Call::default(), &[a], Eax::Checked);
        }
    }
}

/// `m0370::part_place` (0x10001930): slot 712 answering 0 and not.
#[test]
fn part_place_0370() {
    let Some(mut t) = twin(370) else { return };
    let at = t.at(0x1930);
    for (s, n) in [(723u32, 1usize), (724, 1), (258, 1), (231, 3), (230, 2)] {
        rec(&mut t, s, n, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            m.w32(a[3] + 4 * i, 0x3f00_0000 + 0x10_0000 * i);
        }
        0x237
    });
    let answer = SCRATCH + 0x20;
    rec(&mut t, 712, 2, move |m, _| m.r32(answer));
    let (p, lock) = (SCRATCH + 0x200, SCRATCH + 0x210);
    t.set_u32(t.img.word(at + 0x1), p);
    t.set_u32(t.img.word(at + 0xd6), lock);
    t.set_u32(lock, 0x77);
    for (k, o) in (0u32..).zip([0x8u32, 0x26, 0x48, 0x92, 0x9d, 0xa9, 0xbc]) {
        t.set_u32(t.img.word(at + o), 0x00e0_0000 + k);
    }
    for ans in [0u32, 5] {
        t.set_u32(answer, ans);
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// The object start's set-up (`s4_misc::start_object_anim` at `s`).
fn anim_setup(t: &mut Twin, s: u32, count: u16) {
    t.set_u32(t.img.word(s + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(s + 0x5e), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    rec(t, 941, 1, |_, _| SCRATCH + 0x400);
    rec(t, 714, 5, |_, _| 0x714);
    let pa = t.img.rel32_target(s + 0x93);
    let nodes = t.img.word(pa + 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    t.set_u16(t.img.word(pa + 0x04), count);
    t.set_u32(t.img.word(s + 0x9e), 0);
}

/// `n` bytes that differ from word to word.
fn varied(n: usize, seed: u8) -> Vec<u8> {
    (0..n)
        .map(|i| u8::try_from((i * 7 + usize::from(seed)) % 251).unwrap())
        .collect()
}

/// `m0430::three_start` (0x100024a0) and `m0434::main_restart`
/// (0x10006b00, the ninth object in the list and not): the pool free and
/// short.
#[test]
fn starts_0430_0434() {
    for (n, rva, sop, cop) in [
        (430u16, 0x24a0u32, 0xcu32, 0x20u32),
        (434, 0x6b00, 0x3, 0x58),
    ] {
        let Some(mut t) = twin(n) else { return };
        let at = t.at(rva);
        let s = t.img.rel32_target(at + cop);
        rec(&mut t, 713, 1, |_, a| 0x0713_0000 ^ a[0]);
        let blk = SCRATCH + 0x1_0000;
        t.set_u32(t.img.word(at + sop), blk);
        for (k, count) in [8u16, 2].into_iter().enumerate() {
            anim_setup(&mut t, s, count);
            t.write(blk, &varied(0x1b00, 0x5a));
            if n == 434 {
                let list = t.img.word(at + 0x11);
                let node = SCRATCH + 0x700;
                t.write(node, &[0; 0x10]);
                t.set_u32(node + 8, blk + 0x980);
                t.set_u32(list, if k == 0 { node } else { 0 });
                t.set_u32(list + 8, 0);
                t.set_u32(t.img.word(at + 0x1f), SCRATCH + 0x740);
            }
            dirty(&mut t);
            t.check_entry(at, Call::default(), &[], Eax::Checked);
        }
    }
}
