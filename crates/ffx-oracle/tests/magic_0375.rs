//! `magic_0375.dll` against its Rust ports (`ffx_magic::m0375`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 375;

/// 0x10001bc0: the objects in slots 0, 1 and 2 of the block.
#[test]
fn part_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1bc0);
    let list = SCRATCH + 0x700;
    t.set_u32(list, 0x0900_0000);
    rec(&mut t, 941, 1, move |_, _| list);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    let gp = SCRATCH + 0x680;
    t.set_u32(t.img.word(at + 0x10), gp);
    t.set_u32(gp + 0x10, 0x00de_0010);
    t.set_u32(t.img.word(at + 0x04), SCRATCH + 0x640);
    let sp = t.img.rel32_target(at + 0x35);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x6c0);
    let block = t.img.word(at + 0x3c);
    for i in 0..3 {
        let obj = block + 0x80 * i;
        t.write(obj, &[0x5c; 0x80]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x100010a0: slot 712 answering 0 and not, slot 261 answering 0 and
/// not; the lock seen during slot 712 copied out.
#[test]
fn part_draw() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x10a0);
    for (s, n) in [(258u32, 1usize), (231, 4), (230, 6)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let lock = SCRATCH + 0x600;
    let answers = SCRATCH + 0x6f0;
    let pp = SCRATCH + 0x640;
    // 712 copies the lock and the block pointer **P out
    rec(&mut t, 712, 2, move |m, _| {
        let seen = m.r32(lock);
        m.w32(lock + 0x40, seen);
        let v = m.r32(pp);
        m.w32(lock + 0x44, v);
        m.r32(answers)
    });
    rec(&mut t, 261, 1, move |m, _| m.r32(answers + 4));
    t.set_u32(t.img.word(at + 0x5d), lock);
    t.set_u32(t.img.word(at + 0x4d), SCRATCH + 0x640);
    for (k, op) in [0x16u32, 0x21, 0x2d, 0x40].into_iter().enumerate() {
        t.set_u32(t.img.word(at + op), 0x00ab_0000 + u32::try_from(k).unwrap());
    }
    let obj = SCRATCH + 0x1000;
    for (drawn, on, old) in [(0u32, 0u32, 0u32), (5, 0, 0x77), (0, 3, 1)] {
        t.set_u32(lock, old);
        t.set_u32(answers, drawn);
        t.set_u32(answers + 4, on);
        t.set_u32(pp, 0x6464_6464);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x100019a0: slot 712 answering 0 and not (the restart); the lock seen
/// during slot 712 copied out.
#[test]
fn part_restart() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x19a0);
    for (s, n) in [(258u32, 1usize), (231, 4), (230, 6)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let lock = SCRATCH + 0x600;
    let answer = SCRATCH + 0x6f0;
    let pp = SCRATCH + 0x640;
    // 712 and 713 copy the block pointer **P out; 713 leaves junk there
    // (the restart stores it again after the call)
    rec(&mut t, 712, 2, move |m, _| {
        let seen = m.r32(lock);
        m.w32(lock + 0x40, seen);
        let v = m.r32(pp);
        m.w32(lock + 0x44, v);
        m.r32(answer)
    });
    rec(&mut t, 713, 1, move |m, a| {
        let v = m.r32(pp);
        m.w32(lock + 0x48, v);
        m.w32(pp, 0x7171_3713);
        0x0100_0713 ^ a[0]
    });
    let list = SCRATCH + 0x700;
    for i in 0..12 {
        t.set_u32(list + 4 * i, 0x0900_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| list);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    let sp = t.img.rel32_target(at + 0xcd);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x680);
    t.set_u32(t.img.word(at + 0x5c), lock);
    t.set_u32(t.img.word(at + 0x4f), SCRATCH + 0x640);
    let s = SCRATCH + 0x6c0;
    t.set_u32(t.img.word(at + 0xaa), s);
    t.set_u32(s + 0x10, 0x00ab_0010);
    for (k, op) in [0x18u32, 0x23, 0x2f, 0x42].into_iter().enumerate() {
        t.set_u32(t.img.word(at + op), 0x00ab_0000 + u32::try_from(k).unwrap());
    }
    let part = SCRATCH + 0x1000;
    for (drawn, old) in [(0u32, 0u32), (5, 0x77)] {
        t.set_u32(lock, old);
        t.set_u32(answer, drawn);
        t.write(part, &[0x5c; 0x80]);
        t.set_u32(pp, 0x6464_6464);
        t.check_entry(at, Call::default(), &[part], Eax::Checked);
    }
}
