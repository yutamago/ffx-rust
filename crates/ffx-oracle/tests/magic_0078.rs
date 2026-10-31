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
#[allow(clippy::too_many_arguments)]
fn start_cases(
    n: u16,
    last: u32,
    a: u32,
    part: (u32, u32),
    stored: &[u32],
    shift: u32,
    kinds: &[(u32, u32)],
    rva: u32,
) {
    let Some(mut t) = Twin::load(n) else { return };
    t.init();
    let at = t.at(rva);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    // the colour node's slots (the 0083 family)
    t.stub_st0(790, |_, _| 0.25);
    t.stub(558, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 3 + i);
        }
        0x558
    });
    for (slot, k) in [(331u32, 8u32), (353, 9), (271, 10)] {
        t.stub(slot, move |m, a| {
            m.w32(SCRATCH + 0x3000 + 0x40 * k, a[0]);
            0x0100_0000 * (k + 1)
        });
    }
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
    // slot 390 answers the kind, 534 yes or no, 12 a float
    t.stub(390, |m, a| {
        m.w32(SCRATCH + 0x3300, a[0]);
        m.r32(SCRATCH + 0x2e4)
    });
    t.stub(534, |m, a| {
        m.w32(SCRATCH + 0x3304, a[0]);
        m.r32(SCRATCH + 0x2e8)
    });
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) + 0.75);
    t.stub(941, |_, _| SCRATCH + 0x400);
    for k in 0..4 {
        t.set_u32(SCRATCH + 0x400 + 4 * k, 0x0f00_0000 + k);
    }
    let pn = t.img.rel32_target(at + part.1);
    t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x700);
    let (tail, other) = (SCRATCH + 0x500, SCRATCH + 0x600);
    // the words it stores hold junk; junk under the entry too (the
    // records' bytes must be written)
    let head = [0x0c, 0x12, 0x1c, 0x26, 0x30, 0x41, 0x4a]
        .into_iter()
        .chain([0x5a, 0x64, 0x83].map(|o| o + shift));
    for op in head.chain(stored.iter().copied()) {
        t.set_u32(w(&t, op), 0x5a5a_5a5b);
    }
    // the floats the 0083 family reads: each its own value
    if rva != 0x64e0 {
        for (k, op) in (0u32..).zip([
            0x2f5, 0x32d, 0x368, 0x38f, 0x3a2, 0x3b5, 0x3bd, 0x3cd, 0x3d5,
        ]) {
            #[allow(clippy::cast_precision_loss)]
            t.set_u32(w(&t, op), (0.75 + k as f32 * 1.5).to_bits());
        }
    }
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
    let root = w(&t, 0x76 + shift);
    t.set_u32(w(&t, a), SCRATCH + 0x680);
    t.write(SCRATCH + 0x680, &[0x31; 0x40]);
    let plain = [(0x3002, 0)];
    let kinds = if kinds.is_empty() { &plain[..] } else { kinds };
    let base = t.save();
    for (flag, filled) in [(0u8, false), (1, true), (0, true), (3, false)] {
        for &(kind, yes) in kinds {
            t.restore(&base);
            t.set_u32(SCRATCH + 0x2e4, kind);
            t.set_u32(SCRATCH + 0x2e8, yes);
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
}

#[test]
fn start() {
    let s78 = [
        0x98, 0xa0, 0xaa, 0xb4, 0xbe, 0xc7, 0xd4, 0xf4, 0xff, 0x107, 0x111, 0x11b, 0x125, 0x12f,
        0x141, 0x14d, 0x15c, 0x16c, 0x176, 0x180, 0x18a, 0x194, 0x1f2, 0x222, 0x26f, 0x276, 0x2b8,
        0x2bf,
    ];
    start_cases(N, 0x91, 0x1e6, (0x204, 0x22b), &s78, 0, &[], 0x64e0);
    let s79 = [
        0xb4, 0xbf, 0xc7, 0xd1, 0xdb, 0xe5, 0xee, 0xfb, 0x11b, 0x126, 0x12e, 0x138, 0x142, 0x14c,
        0x156, 0x168, 0x174, 0x183, 0x193, 0x19d, 0x1a7, 0x1b1, 0x1bb, 0x222, 0x26f, 0x276, 0x2b8,
        0x2bf,
    ];
    start_cases(79, 0xab, 0x1f3, (0x204, 0x22b), &s79, 0, &[], 0x64e0);
    let s81 = [
        0x98, 0xa0, 0xaa, 0xb4, 0xbe, 0xc7, 0xd4, 0x100, 0x10a, 0x124, 0x12f, 0x137, 0x141, 0x14b,
        0x155, 0x15e, 0x16b, 0x188, 0x192, 0x19c, 0x1a6, 0x1b0, 0x1ba, 0x221, 0x26e, 0x275, 0x2b7,
        0x2be,
    ];
    start_cases(81, 0x91, 0x1f2, (0x203, 0x22a), &s81, 0, &[], 0x64e0);
    let mut s83: Vec<u32> = s81[..s81.len() - 2].to_vec();
    s83.extend([0x2ba, 0x2c1]);
    s83.extend([
        0x2d9, 0x2fb, 0x302, 0x30c, 0x316, 0x322, 0x333, 0x33d, 0x347, 0x351, 0x35d, 0x36e, 0x378,
        0x384, 0x397, 0x3aa, 0x3c5, 0x3dd, 0x3e5,
    ]);
    for (n, rva) in [(83, 0x6860), (86, 0x6860), (87, 0x6860), (97, 0x6870)] {
        start_cases(n, 0x91, 0x1f2, (0x203, 0x22a), &s83, 0, &[], rva);
    }
    let s80 = [
        0x99, 0xa1, 0xab, 0xb5, 0xbf, 0xc8, 0xd5, 0xf5, 0x100, 0x108, 0x112, 0x11c, 0x126, 0x130,
        0x142, 0x14e, 0x15d, 0x16d, 0x177, 0x181, 0x18b, 0x195, 0x230, 0x265, 0x27a, 0x2c9, 0x316,
        0x31d, 0x360, 0x367,
    ];
    let kinds = [
        (0x1071, 1),
        (0x1071, 0),
        (0x1072, 1),
        (0x1080, 0),
        (0x108c, 1),
        (0x1070, 1),
        (0x108d, 0),
        (0, 1),
    ];
    start_cases(80, 0x92, 0x2a1, (0x2b2, 0x2d2), &s80, 1, &kinds, 0x64e0);
}
