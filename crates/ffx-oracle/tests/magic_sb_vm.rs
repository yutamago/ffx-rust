//! The `magic_0162` family's effect-script opcodes and helpers
//! (`ffx_magic::sb_vm`) and `magic_0009`'s shared functions
//! (`ffx_magic::sb_common`) against the DLLs. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod magic;
mod magic_l3;

use ffx_magic::{Ret, sb_common, sb_vm};
use magic::{Call, Eax, SCRATCH, TRAPS, Twin};

const CTX: u32 = SCRATCH + 0x1000;
const IP: u32 = SCRATCH + 0x2000;
const LOG: u32 = SCRATCH + 0x3000;
const BYTES: u32 = SCRATCH + 0x3800;
const COUNT: u32 = SCRATCH + 0x3900;

fn trap(s: u32) -> u32 {
    TRAPS + 16 * s
}

/// Every slot the opcodes call logs its words (arity `n`) in its own row
/// and answers `0x100 + slot`; slot 288 hands out `BYTES` with the count
/// at `COUNT`, slot 549 answers the word at `COUNT + 4`; slot 293 the one
/// at `COUNT + 8`.
fn stubs(t: &mut Twin) {
    for (k, (s, n)) in [
        (543u32, 1usize),
        (371, 2),
        (222, 4),
        (550, 1),
        (271, 1),
        (267, 1),
        (719, 3),
        (306, 3),
        (257, 2),
        (296, 2),
        (256, 5),
        (215, 1),
        (413, 1),
        (1000, 1),
    ]
    .into_iter()
    .enumerate()
    {
        let row = LOG + 0x40 * u32::try_from(k).unwrap();
        t.stub(s, move |m, a| {
            let c = m.r32(row + 0x3c).wrapping_add(1);
            m.w32(row + 0x3c, c);
            for (i, w) in (0u32..).zip(&a[..n]) {
                m.w32(row + 4 * i, w.wrapping_add(c << 24));
            }
            0x100 + s
        });
    }
    t.set_u32(magic::TABLE + 4 * 1000, trap(1000));
    t.stub(288, |m, a| {
        m.w32(LOG + 0x400, a[0]);
        m.w32(a[1], m.r32(COUNT));
        BYTES
    });
    t.stub(549, |m, a| {
        m.w32(LOG + 0x408, a[0]);
        m.r32(COUNT + 4)
    });
    t.stub(293, |m, a| {
        m.w32(LOG + 0x40c, a[0]);
        m.r32(COUNT + 8)
    });
}

fn twin_0162() -> Option<Twin> {
    let mut t = magic_l3::twin(162)?;
    stubs(&mut t);
    t.write(BYTES, &[2, 5, 0, 9, 5, 0xff, 1, 3]);
    Some(t)
}

/// The instruction words the opcodes read: halfword `+2` = `h`, then
/// words from `+4`.
fn ip(t: &mut Twin, h: u16, words: &[u32]) {
    t.write(IP, &[0x5a; 0x40]);
    t.set_u16(IP + 2, h);
    for (i, w) in (0u32..).zip(words) {
        t.set_u32(IP + 4 + 4 * i, *w);
    }
}

/// The context: registers, a loop stack of `depth` with counters.
fn ctx(t: &mut Twin, depth: u8, counter: u16) {
    t.write(CTX, &[0xa5; 0x60]);
    for i in 0..8 {
        t.set_u32(CTX + 0x2c + 4 * i, 0x2c00 + i);
        t.set_u32(CTX + 4 + 4 * i, 0x0400 + i);
    }
    for i in 0..4 {
        t.set_u16(
            CTX + 0x4c + 2 * i,
            counter.wrapping_add(u16::try_from(i).unwrap()),
        );
    }
    t.set_u8(CTX + 0x57, depth);
}

fn run(t: &mut Twin, rva: u32) {
    t.check_entry(
        t.at(rva),
        Call {
            eax: 0x99,
            ecx: 0x1234,
            edx: 0x5678,
            ..Call::default()
        },
        &[CTX, IP],
        Eax::Checked,
    );
}

#[test]
fn dll_main_0162() {
    magic_l3::dll_main(162);
}

#[test]
fn opcodes() {
    let Some(mut t) = twin_0162() else { return };
    // the actor globals: the effect's own character and the byte table
    for rva in [0x6f80u32, 0x7340] {
        let f = t.at(rva);
        let (own, bytes) = if rva == 0x6f80 {
            (0x11, 0x18)
        } else {
            (0x3d, 0x44)
        };
        let g = t.img.word(f + own);
        t.set_u32(g, 0x0c0c_0001);
        let g = t.img.word(f + bytes);
        t.set_u32(g, BYTES);
    }
    let ops: [u32; 23] = [
        0x6c40, 0x6c50, 0x6d60, 0x6da0, 0x6d20, 0x6d80, 0x75b0, 0x76d0, 0x6c70, 0x6e10, 0x7570,
        0x78b0, 0x7010, 0x7920, 0x6ca0, 0x70d0, 0x6f80, 0x7960, 0x72d0, 0x7620, 0x77a0, 0x7340,
        0x7820,
    ];
    let words: [&[u32]; 4] = [
        &[trap(1000), 1, 3, 4, 5, 6],
        &[1, 0, 7, 0x10, 3, 0],
        &[
            0,
            SCRATCH + 0x2100,
            0xffff_fff0,
            0x8000_0000,
            2,
            0xffff_ffff,
        ],
        &[trap(1000), SCRATCH + 0x2100, 0x7fff_ffff, 5, 0, 7],
    ];
    t.set_u32(SCRATCH + 0x2100, 0x3f80_0000);
    t.set_u32(SCRATCH + 0x2104, 0xbf80_0000);
    t.set_u32(SCRATCH + 0x2108, 0x4000_0000);
    for rva in ops {
        for h in [0u16, 2, 3, 0xffff] {
            for (k, ws) in words.iter().enumerate() {
                // the call's target must be code (a trap)
                if rva == 0x6d20 && ws[0] != trap(1000) {
                    continue;
                }
                // the vector opcode reads through its first word
                let vector = [SCRATCH + 0x2100 + 4 * u32::try_from(k).unwrap(), 9];
                let ws: &[u32] = if rva == 0x70d0 { &vector } else { ws };
                // the loop stack matters to the loop opcodes only
                let stacks: &[(u8, u16)] = if rva == 0x6c70 || rva == 0x6ca0 {
                    &[(1, 1), (3, 2), (2, 0xffff), (1, 0)]
                } else {
                    &[(1, 1)]
                };
                for &(depth, counter) in stacks {
                    ip(&mut t, h, ws);
                    ctx(&mut t, depth, counter);
                    t.set_u32(COUNT, [0u32, 3, 8, 0xffff_ffff][k]);
                    t.set_u32(COUNT + 4, [0u32, 1, 0, 5][k]);
                    run(&mut t, rva);
                }
            }
        }
    }
}

#[test]
fn show_others() {
    let Some(mut t) = twin_0162() else { return };
    let f = t.at(0x6330);
    let me = t.img.word(f + 0x14);
    for (grouped, count, mine) in [
        (0u32, 3u32, 5u32),
        (1, 0, 5),
        (1, 8, 5),
        (1, 8, 0xff),
        (1, 0xffff_fffe, 2),
    ] {
        t.set_u32(COUNT, count);
        t.set_u32(COUNT + 8, grouped);
        t.set_u32(me, mine);
        t.write(LOG, &[0; 0x410]);
        t.check(f, Call::this(0x77), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(sb_vm::show_others(g, img, fr.at, fr.esp, fr.ecx))
        });
    }
}

#[test]
fn actor_record() {
    let Some(mut t) = twin_0162() else { return };
    let f = t.at(0x65c0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let (own, table, mode) = (w(&t, 0x14), w(&t, 0x1b), w(&t, 0x28));
    t.set_u32(own, 0x0000_0042);
    t.set_u32(table, BYTES);
    let rec = SCRATCH + 0x4000;
    for m in [0u32, 1] {
        for index in [u32::MAX, 0, 3, 5] {
            t.set_u32(mode, m);
            t.write(rec, &[0xee; 0x20]);
            t.check(
                f,
                Call {
                    eax: 1,
                    ecx: index,
                    edx: rec,
                    ..Call::default()
                },
                &[0x00ab_cdef],
                Eax::Checked,
                |g, img, fr| Ret::eax(sb_vm::actor_record(g, img, fr.at, fr.esp, fr.ecx, fr.edx)),
            );
        }
    }
}

#[test]
fn dll_main_0009() {
    magic_l3::dll_main(9);
}

#[test]
fn anim_step() {
    let Some(mut t) = magic_l3::twin(9) else {
        return;
    };
    let table = SCRATCH + 0x5000;
    t.stub(941, move |m, a| {
        m.w32(LOG, a[0]);
        table
    });
    // four frames: times, the last one's byte +4 (bit 7: loop)
    t.write(table, &[0; 0x60]);
    t.set_u16(table + 6, 4);
    for i in 0..4u32 {
        t.set_u16(
            table + 0x10 + 8 * i + 2,
            u16::try_from(0x300 + 0x100 * i).unwrap(),
        );
    }
    let this = SCRATCH + 0x100;
    for lp in [0u8, 0x80] {
        for playing in [0u32, 1] {
            for frame in [0u16, 2, 3, 4, 0xffff] {
                for time in [0x200u16, 0x201, 0x400, 0, 0x8000] {
                    t.set_u8(table + 0x10 + 8 * u32::from(frame & 3) + 4, lp);
                    t.set_u32(this, 0x7777);
                    t.set_u32(this + 4, playing);
                    t.set_u16(this + 8, frame);
                    t.set_u16(this + 0xa, time);
                    t.check(
                        t.at(0x2470),
                        Call::this(this),
                        &[],
                        Eax::Checked,
                        |g, img, fr| Ret::eax(sb_common::anim_step(g, img, fr.at, fr.ecx)),
                    );
                }
            }
        }
    }
}

#[test]
fn call_239() {
    let Some(mut t) = magic_l3::twin(9) else {
        return;
    };
    t.stub(239, |m, a| {
        for i in 0..3 {
            m.w32(LOG + 4 * i, a[i as usize]);
        }
        0x239
    });
    let f = t.at(0x38c0);
    let g = t.img.word(f + 0x15);
    t.set_u32(g, 0x6666);
    t.set_u32(CTX + 0x120, 0x0120);
    t.check_entry(f, Call::default(), &[CTX], Eax::Checked);
}
