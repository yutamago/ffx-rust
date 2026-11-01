//! The camera start-ups of the `magic_0078` family (`magic_0089`,
//! `0090`, `0091`, `0094`, `0095`, `0096`) against their Rust ports
//! (`ffx_magic::m0089`). Needs `FFX_GAME_DIR`.

mod magic;

use magic::{Call, Eax, SCRATCH, Twin};

/// One start-up: the DLL, its address, the operand of the root list's
/// last node, the particle owner's pointer global, the particle list's
/// operand and the particle node's `rel32`, the head's shift (1 after a
/// `push esi`), the globals it stores (filled with junk first), the
/// floats it reads (each its own value) and the kinds slot 390 answers
/// (with slot 534's answer).
struct Case {
    n: u16,
    rva: u32,
    last: u32,
    a: u32,
    part: (u32, u32),
    shift: u32,
    stored: &'static [u32],
    loads: &'static [u32],
    kinds: &'static [(u32, u32)],
}

/// The start-up into an empty and a filled root list, with the model
/// flag byte clear and set, the ring's tail anywhere, and each kind.
fn start_cases(c: &Case) {
    let Some(mut t) = Twin::load(c.n) else { return };
    t.init();
    let at = t.at(c.rva);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    // the colour node's slots
    t.stub_st0(790, |_, _| 0.25);
    t.stub(558, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 3 + i);
        }
        0x558
    });
    for (slot, k) in [
        (331u32, 8u32),
        (353, 9),
        (271, 10),
        (268, 0),
        (719, 1),
        (413, 2),
        (215, 3),
        (14, 4),
        (325, 5),
        (714, 6),
    ] {
        t.stub(slot, move |m, a| {
            // a count, then each call's first words
            let row = SCRATCH + 0x3000 + 0x40 * k;
            let n = m.r32(row);
            m.w32(row, n + 1);
            // slot 268 `(model, 0x11, x)`; the others' first word
            let words = if slot == 268 { 3 } else { 1 };
            for (i, v) in (0u32..).zip(&a[..words]) {
                m.w32(row + 4 + 4 * i, v ^ n);
            }
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
    let pn = t.img.rel32_target(at + c.part.1);
    t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x700);
    let (tail, other) = (SCRATCH + 0x500, SCRATCH + 0x600);
    // the words it stores hold junk; junk under the entry too (the
    // records' bytes must be written)
    let head = [0x0c, 0x12, 0x1c, 0x26, 0x30, 0x41, 0x4a]
        .into_iter()
        .chain([0x5a, 0x64, 0x83].map(|o| o + c.shift));
    for op in head.chain(c.stored.iter().copied()) {
        t.set_u32(w(&t, op), 0x5a5a_5a5b);
    }
    for (k, &op) in (0u32..).zip(c.loads) {
        #[allow(clippy::cast_precision_loss)]
        t.set_u32(w(&t, op), (0.75 + k as f32 * 1.5).to_bits());
    }
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
    let root = w(&t, 0x76 + c.shift);
    t.set_u32(w(&t, c.a), SCRATCH + 0x680);
    t.write(SCRATCH + 0x680, &[0x31; 0x40]);
    let plain = [(0x3002, 0)];
    let kinds = if c.kinds.is_empty() {
        &plain[..]
    } else {
        c.kinds
    };
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
            t.set_u32(w(&t, c.last), if filled { other } else { root });
            t.set_u32(root + 0x10, if filled { other } else { root });
            t.set_u16(w(&t, 0x4a), 9);
            // the particle list (record word 1) empty, its owner valid
            let parts = w(&t, c.part.0);
            t.set_u32(parts + 0x10, parts);
            t.check_entry(at, Call::default(), &[], Eax::Checked);
        }
    }
}

/// Kinds through both jump-table entries, outside the table and with
/// slot 534's answer either way.
const KINDS: [(u32, u32); 10] = [
    (0x1071, 1),
    (0x1071, 0),
    (0x1072, 1),
    (0x1072, 0),
    (0x1080, 0),
    (0x108c, 1),
    (0x1070, 1),
    (0x108d, 0),
    (0, 1),
    (0, 0),
];

#[test]
fn start_0089() {
    start_cases(&Case {
        n: 89,
        rva: 0x6860,
        last: 0x91,
        a: 0x1f2,
        part: (0x203, 0x22a),
        shift: 0,
        stored: &[
            0x98, 0xa0, 0xaa, 0xb4, 0xbe, 0xc7, 0xd4, 0xed, 0x100, 0x10a, 0x124, 0x12f, 0x137,
            0x141, 0x14b, 0x155, 0x15e, 0x16b, 0x17f, 0x188, 0x192, 0x19c, 0x1a6, 0x1b0, 0x1ba,
            0x221, 0x26e, 0x275, 0x28d, 0x2af, 0x2b6, 0x2c0, 0x2ca, 0x2d6, 0x2e7, 0x2f1, 0x2fb,
            0x305, 0x311, 0x322, 0x32c, 0x338, 0x34b, 0x35e, 0x379, 0x391, 0x399,
        ],
        loads: &[
            0x29e, 0x2a9, 0x2e1, 0x31c, 0x343, 0x356, 0x369, 0x371, 0x381, 0x389,
        ],
        kinds: &[],
    });
}

#[test]
fn start_0090() {
    start_cases(&Case {
        n: 90,
        rva: 0x6890,
        last: 0x91,
        a: 0x1f2,
        part: (0x203, 0x22a),
        shift: 0,
        stored: &[
            0x98, 0xa0, 0xaa, 0xb4, 0xbe, 0xc7, 0xd4, 0xed, 0x100, 0x10a, 0x124, 0x12f, 0x137,
            0x141, 0x14b, 0x155, 0x15e, 0x16b, 0x17f, 0x188, 0x192, 0x19c, 0x1a6, 0x1b0, 0x1ba,
            0x221, 0x2a9, 0x2b0, 0x2c8, 0x2fa, 0x301, 0x30b, 0x317, 0x328, 0x332, 0x33c, 0x346,
            0x352, 0x363, 0x36d, 0x377, 0x383, 0x38b, 0x39e, 0x3b1, 0x3d1, 0x3d9,
        ],
        loads: &[
            0x2d9, 0x2e1, 0x2ec, 0x2f4, 0x322, 0x35d, 0x396, 0x3a9, 0x3b9, 0x3c1, 0x3c9,
        ],
        kinds: &[],
    });
}

#[test]
fn start_0091() {
    start_cases(&Case {
        n: 91,
        rva: 0x6870,
        last: 0x91,
        a: 0x1f2,
        part: (0x203, 0x22a),
        shift: 0,
        stored: &[
            0x98, 0xa0, 0xaa, 0xb4, 0xbe, 0xc7, 0xd4, 0xed, 0x100, 0x10a, 0x124, 0x12f, 0x137,
            0x141, 0x14b, 0x155, 0x15e, 0x16b, 0x17f, 0x188, 0x192, 0x19c, 0x1a6, 0x1b0, 0x1ba,
            0x221, 0x26e, 0x275, 0x2ba, 0x2c1, 0x2d9, 0x30b, 0x312, 0x31c, 0x328, 0x339, 0x343,
            0x34d, 0x357, 0x363, 0x374, 0x37e, 0x388, 0x394, 0x39c, 0x3af, 0x3ca, 0x3e2, 0x3ea,
        ],
        loads: &[
            0x2ea, 0x2f2, 0x2fd, 0x305, 0x333, 0x36e, 0x3a7, 0x3ba, 0x3c2, 0x3d2, 0x3da,
        ],
        kinds: &[],
    });
}

#[test]
fn start_0094() {
    start_cases(&Case {
        n: 94,
        rva: 0x6b50,
        last: 0x92,
        a: 0x357,
        part: (0x368, 0x388),
        shift: 1,
        stored: &[
            0x99, 0xa1, 0xab, 0xb5, 0xbf, 0xc8, 0xd5, 0xf5, 0x100, 0x108, 0x112, 0x11c, 0x126,
            0x130, 0x142, 0x14e, 0x15d, 0x167, 0x16d, 0x177, 0x181, 0x18b, 0x202, 0x209, 0x257,
            0x25e, 0x2be, 0x2c8, 0x2d2, 0x307, 0x31c, 0x335, 0x33f, 0x352, 0x37f, 0x39f, 0x3c1,
            0x3c8, 0x3d2, 0x3dc, 0x3e8, 0x3f9, 0x403, 0x40d, 0x417, 0x423, 0x434, 0x43e, 0x44b,
            0x45e, 0x471, 0x48c, 0x4a4, 0x4ac,
        ],
        loads: &[
            0x32c, 0x3b0, 0x3bb, 0x3f3, 0x42e, 0x456, 0x469, 0x47c, 0x484, 0x494, 0x49c,
        ],
        kinds: &KINDS,
    });
}

#[test]
fn start_0095() {
    start_cases(&Case {
        n: 95,
        rva: 0x6ae0,
        last: 0x91,
        a: 0x1d8,
        part: (0x1e9, 0x210),
        shift: 0,
        stored: &[
            0x98, 0xa0, 0xaa, 0xb4, 0xbe, 0xc7, 0xd4, 0xed, 0x100, 0x10a, 0x124, 0x12f, 0x137,
            0x141, 0x14b, 0x155, 0x15e, 0x16b, 0x186, 0x18f, 0x199, 0x1a4, 0x207, 0x254, 0x25b,
            0x2a0, 0x2a7, 0x2bf, 0x2e1, 0x2e8, 0x2f2, 0x2fc, 0x308, 0x319, 0x323, 0x32d, 0x337,
            0x343, 0x354, 0x35e, 0x36a, 0x37d, 0x390, 0x3a3, 0x3c3, 0x3cb,
        ],
        loads: &[
            0x2d0, 0x2db, 0x313, 0x34e, 0x375, 0x388, 0x39b, 0x3ab, 0x3b3, 0x3bb,
        ],
        kinds: &[],
    });
}

#[test]
fn start_0096() {
    start_cases(&Case {
        n: 96,
        rva: 0x6870,
        last: 0x92,
        a: 0x2a1,
        part: (0x2b2, 0x2d2),
        shift: 1,
        stored: &[
            0x99, 0xa1, 0xab, 0xb5, 0xbf, 0xc8, 0xd5, 0xf5, 0x100, 0x108, 0x112, 0x11c, 0x126,
            0x130, 0x142, 0x14e, 0x15d, 0x167, 0x16d, 0x177, 0x181, 0x18b, 0x195, 0x230, 0x265,
            0x27a, 0x29c, 0x2c9, 0x316, 0x31d, 0x362, 0x369, 0x381, 0x3a3, 0x3aa, 0x3b4, 0x3be,
            0x3ca, 0x3db, 0x3e5, 0x3ef, 0x3f9, 0x405, 0x416, 0x420, 0x42d, 0x440, 0x453, 0x46e,
            0x486, 0x48e,
        ],
        loads: &[
            0x28a, 0x392, 0x39d, 0x3d5, 0x410, 0x438, 0x44b, 0x45e, 0x466, 0x476, 0x47e,
        ],
        kinds: &KINDS,
    });
}
