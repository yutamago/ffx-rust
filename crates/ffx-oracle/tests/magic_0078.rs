//! `magic_0078.dll` against its Rust port (`ffx_magic::m0078`). Needs
//! `FFX_GAME_DIR`.

mod magic;

use magic::{Call, Eax, SCRATCH, Twin};

const N: u16 = 78;

#[test]
fn clock_unlinks_the_task_when_due() {
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    let f = t.at(0x6470);
    let k0 = f32::from_bits(t.u32(t.img.word(f + 0x1e)));
    let k1 = f32::from_bits(t.u32(t.img.word(f + 0x2f)));
    eprintln!("K0 {k0} K1 {k1}");
    t.set_u32(t.img.word(f + 0x46), 0x00aa_0046);
    t.stub(239, |m, a| {
        for (i, w) in (0u32..).zip(&a[..3]) {
            m.w32(SCRATCH + 0x800 + 4 * i, *w);
        }
        0x0239
    });
    let task = SCRATCH;
    let obj = SCRATCH + 0x1000;
    let node = SCRATCH + 0x100;
    let (prev, next) = (SCRATCH + 0x200, SCRATCH + 0x300);
    for (held, clock) in [
        (0u16, k1 - k0),
        (0, k1 - k0 - 1.0),
        (1, k1),
        (0, k1 + 5.0),
        (0, f32::NAN),
        (0, k1 - 2.0 * k0),
        (0x100, -1.0),
        (0x100, k1 + 1.0),
    ] {
        t.set_u32(task, node);
        t.set_u32(task + 4, obj);
        t.set_u16(obj + 0x2c8, held);
        t.set_u32(obj + 0x2c4, clock.to_bits());
        t.set_u32(node + 4, prev);
        t.set_u32(node + 8, next);
        t.write(prev, &[0x11; 0x10]);
        t.write(next, &[0x22; 0x10]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// A start-up (0x100064e0) into an empty and a filled root list, with
/// the model flag byte clear and set and the ring's tail anywhere.
/// `last`: the operand of the root list's last node; `a`: the particle
/// owner's pointer global; `part`: the particle list's operand and the
/// particle node's `rel32`.
fn start_cases(n: u16, last: u32, a: u32, part: (u32, u32), stored: &[u32]) {
    let Some(mut t) = Twin::load(n) else { return };
    t.init();
    let at = t.at(0x64e0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    for (slot, k) in [
        (268u32, 0u32),
        (719, 1),
        (413, 2),
        (215, 3),
        (14, 4),
        (325, 5),
        (714, 6),
    ] {
        t.stub(slot, move |m, a| {
            m.w32(SCRATCH + 0x3000 + 0x40 * k, a[0]);
            0x0100_0000 * (k + 1)
        });
    }
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.5);
    t.stub(941, |_, _| SCRATCH + 0x400);
    for k in 0..4 {
        t.set_u32(SCRATCH + 0x400 + 4 * k, 0x0f00_0000 + k);
    }
    let pn = t.img.rel32_target(at + part.1);
    t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x700);
    let (tail, other) = (SCRATCH + 0x500, SCRATCH + 0x600);
    // the words it stores hold junk; junk under the entry too (the
    // records' bytes must be written)
    let head = [0x0c, 0x12, 0x1c, 0x26, 0x30, 0x41, 0x4a, 0x5a, 0x64, 0x83];
    for &op in head.iter().chain(stored) {
        t.set_u32(w(&t, op), 0x5a5a_5a5b);
    }
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
    let root = w(&t, 0x76);
    t.set_u32(w(&t, a), SCRATCH + 0x680);
    t.write(SCRATCH + 0x680, &[0x31; 0x40]);
    for (flag, filled) in [(0u8, false), (1, true), (0, true), (3, false)] {
        t.write(tail, &[0x55; 0x10]);
        t.write(other, &[0x66; 0x20]);
        t.set_u32(w(&t, 0x07), tail);
        t.write(w(&t, 0x51), &[flag, 0x40]);
        t.set_u32(w(&t, last), if filled { other } else { root });
        t.set_u32(root + 0x10, if filled { other } else { root });
        t.set_u16(w(&t, 0x4a), 9);
        // the particle list (record word 1) empty, its owner valid
        let parts = w(&t, part.0);
        t.set_u32(parts + 0x10, parts);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn start() {
    let s78 = [
        0x98, 0xa0, 0xaa, 0xb4, 0xbe, 0xc7, 0xd4, 0xf4, 0xff, 0x107, 0x111, 0x11b, 0x125, 0x12f,
        0x141, 0x14d, 0x15c, 0x16c, 0x176, 0x180, 0x18a, 0x194, 0x1f2, 0x222, 0x26f, 0x276, 0x2b8,
        0x2bf,
    ];
    start_cases(N, 0x91, 0x1e6, (0x204, 0x22b), &s78);
    let s79 = [
        0xb4, 0xbf, 0xc7, 0xd1, 0xdb, 0xe5, 0xee, 0xfb, 0x11b, 0x126, 0x12e, 0x138, 0x142, 0x14c,
        0x156, 0x168, 0x174, 0x183, 0x193, 0x19d, 0x1a7, 0x1b1, 0x1bb, 0x222, 0x26f, 0x276, 0x2b8,
        0x2bf,
    ];
    start_cases(79, 0xab, 0x1f3, (0x204, 0x22b), &s79);
    let s81 = [
        0x98, 0xa0, 0xaa, 0xb4, 0xbe, 0xc7, 0xd4, 0x100, 0x10a, 0x124, 0x12f, 0x137, 0x141, 0x14b,
        0x155, 0x15e, 0x16b, 0x188, 0x192, 0x19c, 0x1a6, 0x1b0, 0x1ba, 0x221, 0x26e, 0x275, 0x2b7,
        0x2be,
    ];
    start_cases(81, 0x91, 0x1f2, (0x203, 0x22a), &s81);
}
