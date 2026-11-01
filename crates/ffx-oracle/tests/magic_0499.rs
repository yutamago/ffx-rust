//! `magic_0499.dll` against its Rust port (`ffx_magic::m0499`). Needs
//! `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0499};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, recs, twin};

const N: u16 = 499;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100012b0: the start (the flags and state words set before).
#[test]
fn start() {
    let Some(mut t) = twin(N) else { return };
    rec(&mut t, 803, 2, |_, _| 0x2100_0000);
    rec(&mut t, 260, 1, |_, a| 0x0260_0000 ^ a[0]);
    recs(&mut t, &[(861, 3), (259, 3), (718, 1), (222, 4)]);
    let at = t.at(0x12b0);
    for op in [
        0x61u32, 0x67, 0x6d, 0x73, 0x79, 0x95, 0x9b, 0xa1, 0xa7, 0xad, 0xe6, 0xf0, 0xfa,
    ] {
        let a = t.img.word(at + op);
        t.set_u32(a, 0x7777_7777);
    }
    t.check_entry(at, Call::default(), &[0x00a6_0000], Eax::Checked);
}

/// 0x100010a0: the five parts' refresh, each part's flag on or off, slot
/// 712 saying 0 or not; every global it reads set to a distinct value.
#[test]
fn parts_refresh() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x10a0);
    let pr = t.img.rel32_target(at + 0x1e1);
    let cell = SCRATCH + 0x10;
    // slot 712 also keeps the game flag and the part pointer's cell it
    // runs under (both functions' cells)
    rec(&mut t, 712, 2, move |m, a| {
        let k = m.r32(magic_owna::row(712));
        for (i, c) in [
            SCRATCH + 0x20,
            SCRATCH + 0x30,
            SCRATCH + 0x40,
            SCRATCH + 0x50,
        ]
        .into_iter()
        .enumerate()
        {
            let v = m.r32(c);
            m.w32(SCRATCH + 0x2_0400 + 0x10 * (k % 0x40) + 4 * i as u32, v);
        }
        (m.r32(cell) >> (a[0] >> 7 & 7)) & 1
    });
    for (s, n) in [
        (236u32, 3),
        (243, 2),
        (231, 3),
        (233, 3),
        (237, 4),
        (230, 2),
        (258, 1),
    ] {
        rec(&mut t, s, n, move |m, a| {
            if s == 236 {
                for i in 0..16 {
                    m.w32(a[2] + 4 * i, 0x3f00_0000 + i);
                }
            }
            // slot 231 keeps the matrix it gets last (the local `A`)
            if s == 231 && a[2] >= 0x2000_0000 {
                let k = m.r32(magic_owna::row(231));
                magic_owna::dump(m, 0x100 + k % 0x40, a[2], 16);
            }
            (0x5a5a_0000 + s) ^ a[0]
        });
    }
    // the globals: matrix and the three block pointers, lock and part
    // pointers each to a cell of its own (both functions')
    let words = [
        (at, 0xb7u32, SCRATCH + 0x400),
        (at, 0x13e, 0x00c0_013e),
        (at, 0x14a, 0x00c0_014a),
        (at, 0x15d, 0x00c0_015d),
        (at, 0x16b, SCRATCH + 0x20),
        (at, 0x170, SCRATCH + 0x30),
    ];
    for (f, op, v) in words {
        t.set_u32(t.img.word(f + op), v);
    }
    // part_refresh_b's: four block words, the part and lock pointers
    for op in [0x1fu32, 0x2a, 0x36, 0x3e, 0x49] {
        let g = t.img.word(pr + op);
        if t.u32(g) == 0 {
            t.set_u32(g, 0x00d0_0000 + op);
        }
    }
    t.set_u32(t.img.word(pr + 0x56), SCRATCH + 0x50);
    t.set_u32(t.img.word(pr + 0x63), SCRATCH + 0x40);
    t.set_u32(t.img.word(pr + 0x7f), SCRATCH + 0x40);
    let block = SCRATCH + 0x1000;
    for (flags, bits) in [
        (0b1_1111u32, 0u32),
        (0b1_1111, 0xff),
        (0b0_0101, 0x0a),
        (0b1_0010, 0x35),
        (0, 0),
    ] {
        t.set_u32(cell, bits);
        let mut b = [0x5bu8; 0x2a0];
        b[..4].copy_from_slice(&0x00b1_0000u32.to_le_bytes());
        for i in 0..5usize {
            let o = 0x10 + 0x80 * i;
            let on = if flags >> i & 1 == 1 { 0x0101u16 } else { 0 };
            b[o + 0x1e..o + 0x20].copy_from_slice(&on.to_le_bytes());
            for k in 0..3usize {
                let x = (1.5 + i as f32 + 0.25 * k as f32).to_bits();
                b[o + 0x60 + 4 * k..o + 0x64 + 4 * k].copy_from_slice(&x.to_le_bytes());
            }
        }
        t.write(block, &b);
        t.set_u32(SCRATCH + 0x20, 0x1111);
        t.set_u32(SCRATCH + 0x40, 0x2222);
        t.check(at, Call::this(block), &[], Eax::Junk, |g, img, fr| {
            Ret::eax(m0499::parts_refresh(g, img, fr))
        });
    }
}

/// 0x10001400: the frame step through its one-time steps and the frames
/// 1, 30 and 100 (and others, and the stop flag), every global it reads
/// set to a distinct value.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1400);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 225, 1, move |m, _| m.r32(cell) & 1);
    rec(&mut t, 227, 1, move |m, _| m.r32(cell) >> 1 & 3);
    rec(&mut t, 261, 1, move |m, _| m.r32(cell) >> 3 & 1);
    let table = SCRATCH + 0x200;
    t.set_u32(table, 0x0f00_0000);
    t.set_u32(table + 4, 0x0f00_0004);
    // slot 941 keeps the part pointer's cell it sees, then clobbers it,
    // so each `*G_P = ITEM` before a part shows
    let part_cell = SCRATCH + 0x40;
    rec(&mut t, 941, 1, move |m, _| {
        let k = m.r32(magic_owna::row(941));
        let v = m.r32(part_cell);
        m.w32(SCRATCH + 0x2_0800 + 4 * (k % 0x100), v);
        m.w32(part_cell, 0);
        table
    });
    recs(&mut t, &[(226, 2), (711, 3), (710, 4), (714, 5), (214, 1)]);
    let w = |t: &magic::Twin, op: u32| t.img.word(at + op);
    let (lock, stop, part) = (SCRATCH + 0x20, SCRATCH + 0x30, SCRATCH + 0x40);
    for (op, v) in [
        (0x10u32, 0x00c0_0010u32),
        (0x91, lock),
        (0xcc, 0x00c0_00cc),
        (0xfd, stop),
        (0x151, part),
        (0x162, 0x00c0_0162),
    ] {
        let a = w(&t, op);
        t.set_u32(a, v);
    }
    // (once flags, frame count, slot answers, stop)
    for (once, n, bits, halt) in [
        (0u32, 0u32, 1u32, 0u32),
        (0, 0, 0, 0),
        (0, 0, 0b0_0010, 0),
        (1, 0, 0b0_1010, 0),
        (1, 0x1d, 0b0_1010, 0),
        (1, 0x64, 0b0_0010, 0),
        (1, 0x1e, 0b0_0010, 0),
        (1, 1, 0b0_0010, 0),
        (1, 7, 0b0_1010, 0),
        (1, 0, 0b0_1010, 1),
        (1, 1, 0b0_0010, 1),
    ] {
        // the rows first: the second row of frame 30's parts ends just
        // below the DLL's frame globals
        for first in [0x138u32, 0x34c] {
            let base = w(&t, first) - 0x40;
            t.write(base, &[0x5b; 0x380]);
        }
        for op in [0x2u32, 0x2f, 0x5a, 0x88] {
            let a = w(&t, op);
            t.set_u32(a, once * (op + 1));
        }
        let a = w(&t, 0x112);
        t.set_u32(a, n);
        t.set_u32(cell, bits);
        t.set_u32(stop, halt);
        t.set_u32(lock, 0x7777);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
