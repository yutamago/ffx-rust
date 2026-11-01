//! `magic_0424.dll` against its Rust ports (`ffx_magic::m0424`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 424;

/// 0x100066b0 (internal, `ecx` = `[node]`): without a parent, as the
/// parent's only child, first, middle and last.
#[test]
fn release() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x66b0);
    rec(&mut t, 376, 1, |_, a| 0x0376_0000 ^ a[0]);
    let cell = SCRATCH + 0x100;
    let node = SCRATCH + 0x1000;
    let parent = SCRATCH + 0x2000;
    let (a, b) = (SCRATCH + 0x2400, SCRATCH + 0x2800);
    for (p, prev, next) in [
        (0u32, a, b),
        (parent, parent, parent),
        (parent, parent, b),
        (parent, a, b),
        (parent, a, parent),
    ] {
        t.write(node, &[0x5c; 0x100]);
        t.write(parent, &[0x3c; 0x20]);
        t.set_u16(parent + 0x18, 5);
        t.write(a, &[0x1c; 0x10]);
        t.write(b, &[0x2c; 0x10]);
        t.set_u32(node + 4, prev);
        t.set_u32(node + 8, next);
        t.set_u32(node + 0x14, p);
        t.set_u32(cell, node);
        t.check(
            at,
            Call::this(cell),
            &[],
            Eax::Checked,
            ffx_magic::m0424::release,
        );
    }
}

/// 0x100065a0 (internal, `ecx` = the record): the particle node's
/// set-up without a parent and under one with a child.
#[test]
fn part_record() {
    use magic::StubMem;
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x65a0);
    let list = SCRATCH + 0x700;
    for i in 0..4 {
        t.set_u32(list + 4 * i, 0x0900_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| list);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    rec(&mut t, 325, 2, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v ^ 0x0325);
        }
        0x325
    });
    rec(&mut t, 942, 1, |_, a| 0x0942_0000 ^ a[0]);
    rec(&mut t, 377, 4, |_, a| 0x0377_0000 ^ a[1]);
    let sp = t.img.rel32_target(at + 0x88);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x600);
    t.set_u32(t.img.word(at + 0x45), SCRATCH + 0x640);
    let r = SCRATCH + 0x100;
    let node = SCRATCH + 0x1000;
    let parent = SCRATCH + 0x2000;
    let kid = SCRATCH + 0x2800;
    let table = SCRATCH + 0x3000;
    for k in 0..0x20 {
        t.set_u32(table + 4 * k, 0x0e00_0000 + k);
    }
    for (with_parent, index) in [(false, 0u32), (true, 2)] {
        t.write(node, &[0x5c; 0x120]);
        t.write(parent, &[0x3c; 0x40]);
        t.set_u16(parent + 0x18, 5);
        t.set_u32(parent + 0x10, kid);
        t.write(kid, &[0x2c; 0x40]);
        for (i, v) in (0u32..).zip([
            node,
            if with_parent { parent } else { 0 },
            0x00a0_0008,
            table,
            0x00b0_0010,
            index,
        ]) {
            t.set_u32(r + 4 * i, v);
        }
        t.check(
            at,
            Call::this(r),
            &[],
            Eax::Checked,
            ffx_magic::m0424::part_record,
        );
    }
}
