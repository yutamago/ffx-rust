//! `magic_0530.dll` against its Rust port (`ffx_magic::m0530`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::many_single_char_names
)]

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, recs, twin};

const N: u16 = 530;

/// The start's choice word and its two state blocks (operands).
const CHOICE: u32 = 0x254;
const BLOCKS: [u32; 2] = [0x268u32, 0x2b6];

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001140: the object's start with the light flag clear and set.
#[test]
fn start() {
    let Some(mut t) = twin(N) else { return };
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let at = t.at(0x1140);
    let o = SCRATCH + 0x100;
    t.set_u32(t.img.word(at + 0x10), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            m.w32(a[2] + 4 * i, 0x4000_0000 + i);
        }
        0x268
    });
    recs(&mut t, &[(714, 5), (393, 3)]);
    // the two globals the calls pass, distinct from their neighbours
    t.set_u32(t.img.word(at + 0x4d), 0x00a5_004d);
    t.set_u32(t.img.word(at + 0xaa), 0x00a5_00aa);
    let obj = SCRATCH + 0x1000;
    for flag in [0u32, 1] {
        t.set_u32(t.img.word(at + 0x58), flag);
        t.write(obj, &[0x5b; 0x80]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x10001200: the frame with the flag clear and set, slot 712 saying 0
/// and not; the globals distinct, the matrix stubs filling and keeping
/// the locals.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1200);
    let cell = SCRATCH + 0x10;
    let (lock, part) = (SCRATCH + 0x20, SCRATCH + 0x30);
    rec(&mut t, 712, 2, move |m, _| {
        let k = m.r32(magic_owna::row(712));
        let l = m.r32(lock);
        let p = m.r32(part);
        m.w32(SCRATCH + 0x2_0400 + 8 * (k % 0x40), l);
        m.w32(SCRATCH + 0x2_0404 + 8 * (k % 0x40), p);
        m.r32(cell) & 1
    });
    for (s, n) in [
        (210u32, 2usize),
        (235, 1),
        (231, 3),
        (268, 3),
        (325, 2),
        (230, 2),
    ] {
        rec(&mut t, s, n, move |m, a| {
            let k = m.r32(magic_owna::row(s));
            // the matrix slots fill their first argument, and keep the
            // second when it is a local
            if matches!(s, 210 | 235 | 268) {
                let dst = match s {
                    235 => a[0],
                    210 => a[1],
                    _ => a[2],
                };
                for i in 0..16 {
                    m.w32(dst + 4 * i, 0x3f00_0000 + 0x100 * s + i + k);
                }
            }
            if a.len() > 1 && a[1] >= 0x2000_0000 {
                magic_owna::dump(m, 0x100 + (s + k) % 0x40, a[1], 16);
            }
            if a.len() > 2 && a[2] >= 0x2000_0000 {
                magic_owna::dump(m, 0x180 + (s + k) % 0x40, a[2], 16);
            }
            (0x5a5a_0000 + s) ^ a[0]
        });
    }
    for (op, v) in [
        (0x40u32, 0x00a5_0040u32),
        (0xb4, 0x00a5_00b4),
        (0x122, 0x00a5_0122),
        (0x13e, 0x00a5_013e),
        (0x144, 0x00a5_0144),
        (0x14a, 0x00a5_014a),
        (0x15d, 0x00a5_015d),
        (0x16a, part),
        (0x177, lock),
    ] {
        let g = t.img.word(at + op);
        t.set_u32(g, v);
    }
    let obj = SCRATCH + 0x1000;
    for (flag, bits) in [(0u32, 0u32), (1, 0), (2, 1)] {
        t.set_u32(t.img.word(at + 0x15), flag);
        t.set_u32(cell, bits);
        t.set_u32(lock, 0x7777);
        t.set_u32(part, 0x6666);
        t.write(obj, &[0x5b; 0x80]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// The effect start's stubs: the buffer, the handles (the third one a
/// block with three program words), the actor's mode byte and kind, and
/// slot 367 writing the stack word; each records its words.
fn start_stubs(t: &mut magic::Twin, cell: u32) {
    let blk = SCRATCH + 0x600;
    for i in 0..8u32 {
        t.set_u32(blk + 4 * i, 0x0b00_0000 + 0x11 * i);
    }
    let modes = SCRATCH + 0x700;
    t.set_u32(modes, 0x5a5a_5a37);
    rec(t, 803, 2, |_, _| 0x2100_0000);
    rec(t, 719, 3, move |m, a| {
        let k = m.r32(magic_owna::row(719));
        if a[1] == 8 { blk } else { 0x0719_0000 + k }
    });
    rec(t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    rec(t, 168, 1, |_, a| 0x0168_0000 ^ a[0]);
    rec(t, 215, 1, |_, a| 0x0215_0000 ^ a[0]);
    rec(t, 218, 2, move |_, _| modes);
    rec(t, 415, 2, move |m, _| m.r32(cell));
    rec(t, 367, 8, move |m, a| {
        let v = m.r32(cell + 4);
        m.w32(a[2], v);
        0x367
    });
    // slot 711 keeps the game flag it runs under
    let lock = SCRATCH + 0x20;
    rec(t, 711, 3, move |m, _| {
        let l = m.r32(lock);
        m.w32(SCRATCH + 0x2_0400, l);
        0x711
    });
    recs(t, &[(718, 1), (222, 4), (221, 4), (710, 4)]);
}

/// 0x100016c0: the effect's start for the three kinds of actor, with the
/// choice word clear and set, slot 367's word with bits 0 and 2 apart.
#[test]
fn start_effect() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x16c0);
    let cell = SCRATCH + 0x10;
    start_stubs(&mut t, cell);
    let lock = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0xdd), lock);
    t.set_u32(t.img.word(at + 0x104), lock);
    for (op, v) in [
        (0x64u32, 0x00a5_0064u32),
        (0x83, 0x00a5_0083),
        (0x10f, 0x00a5_010f),
        (0x178, 0x00a5_0178),
    ] {
        let g = t.img.word(at + op);
        t.set_u32(g, v);
    }
    for kind in [0xb1u32, 0xb9, 0x10] {
        for (choice, word) in [(0u32, 0b001u32), (1, 0b100), (0, 0b101)] {
            t.set_u32(cell, kind);
            t.set_u32(cell + 4, word);
            t.set_u32(lock, 0x7777);
            t.set_u32(t.img.word(at + CHOICE), choice);
            for blk in BLOCKS {
                let a = t.img.word(at + blk);
                t.write(a, &[0x5b; 0x110]);
            }
            t.check_entry(at, Call::default(), &[0x00a6_0000], Eax::Checked);
        }
    }
}
