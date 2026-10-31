//! `magic_0481.dll` against its Rust port (`ffx_magic::m0481`): the
//! effect's start and end entries, the record search, the records' draw
//! and the facing matrix. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

mod magic;
mod magic_l3;
#[path = "magic/ownb.rs"]
mod ownb;

use ffx_magic::{Ret, m0481};
use magic::{Call, Eax, SCRATCH, Twin};
use ownb::{fill, floats, rec, rec_all};

const N: u16 = 481;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The records' base (`G_REC`).
fn records(t: &Twin) -> u32 {
    t.img.word(t.at(0x1040) + 0x06)
}

#[test]
fn free_record() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x1040);
    let base = records(&t);
    // none live, the first few live, all but the last, all live
    for live in [0u32, 1, 5, 127, 128] {
        for i in 0..128 {
            let n = if i < live { 1 + i as u16 } else { 0 };
            t.set_u16(base + 0x30 * i + 0x28, n);
        }
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0481::free_record(g, img, fr.at))
        });
    }
    // a gap after a live one; the high byte alone counts as live
    t.set_u16(base + 0x30 * 3 + 0x28, 0x100);
    t.set_u16(base + 0x30 * 9 + 0x28, 0);
    t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0481::free_record(g, img, fr.at))
    });
}

const SPR: u32 = SCRATCH + 0x1000;
const P941: u32 = SCRATCH + 0x2000;
const SHAPES: u32 = SCRATCH + 0x3000;

#[test]
fn draw_records() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x11c0);
    let w = |t: &Twin, off: u32| t.img.word(f + off);
    let base = records(&t);
    rec(&mut t, 264, 1, |_, _| SPR);
    rec(&mut t, 941, 2, |_, _| P941);
    t.set_u32(P941 + 4, SHAPES);
    for s in 0..8u32 {
        let off = (s * 0x40) as u16 ^ 0x8000;
        t.set_u16(SHAPES + 0x10 + 8 * s, off);
    }
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, 0x3f00_0000 + i);
        }
        0
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[2] + 4 * i) ^ m.r32(a[1]);
            m.w32(a[0] + 4 * i, v);
        }
        0
    });
    rec(&mut t, 230, 2, |m, a| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v.wrapping_add(i));
        }
        0
    });
    rec(&mut t, 242, 2, |m, a| {
        m.r32(a[1] + 0x14) + 0x40 + m.r32(a[0])
    });
    rec_all(&mut t, &[(265, 1)]);
    t.set_u32(w(&t, 0x28), 0x5038);
    t.set_u32(w(&t, 0x85), 0x1234_5678);
    let cursor = t.u32(w(&t, 0x43));
    let pause = t.u32(w(&t, 0x14b));
    fill(&mut t, SHAPES + 0x100, 0x80, 7);
    for (i, n) in [(0u32, 3u16), (5, 1), (6, 0), (126, 0xffff), (127, 2)] {
        let r = base + 0x30 * i;
        floats(
            &mut t,
            r,
            &[1.5, -2.0, 0.25, 0.0, 0.5, -0.75, 3.0, 0.0, 1.25, 0.0],
        );
        t.set_u16(r + 0x26, (i % 8) as u16);
        t.set_u16(r + 0x28, n);
    }
    for (p, c) in [(0u32, 0x100u32), (1, 0x200), (0, 0x300)] {
        t.set_u32(pause, p);
        t.set_u32(cursor, c);
        // the sprite's fields hold old values
        fill(&mut t, SPR, 0x30, c);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0481::draw_records(g, img, fr.at))
        });
    }
}

#[test]
fn start_and_end() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x1390);
    let base = records(&t);
    rec(&mut t, 803, 2, |_, _| SCRATCH + 0x8000);
    rec_all(
        &mut t,
        &[(861, 5), (259, 5), (260, 6), (718, 7), (222, 11), (221, 15)],
    );
    // every count, and the word after the last (the loop's bound)
    for i in 0..=128 {
        t.set_u16(base + 0x30 * i + 0x28, 0x1111);
    }
    for off in (0xc4..=0x14e).step_by(6) {
        let g = t.img.word(f + off);
        t.set_u32(g, 0xdead_0000 | off);
    }
    t.check_entry(f, Call::default(), &[0x77], Eax::Checked);
    let e = t.at(0x14f0);
    // pppAllFreePObject sees the environment pointer the entry set
    let ppp = t.u32(t.img.word(e + 0x01));
    t.set_u32(ppp, 0x0bad);
    rec(&mut t, 713, 1, move |m, _| {
        let k = m.r32(SCRATCH + 0x600);
        m.w32(SCRATCH + 0x600, k + 1);
        let v = m.r32(ppp);
        m.w32(SCRATCH + 0x604 + 4 * k, v);
        // the second call's environment is the entry's to set again
        m.w32(ppp, 0x0bad);
        0
    });
    rec_all(&mut t, &[(804, 3)]);
    t.check_entry(e, Call::default(), &[], Eax::Checked);
}

#[test]
fn facing() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x17f0);
    // "normalise": the source's bits changed into the destination
    rec(&mut t, 232, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ (0x0101_0000 << i);
            m.w32(a[0] + 4 * i, v);
        }
        0
    });
    // the matrix holds old values
    let out = t.u32(t.img.word(f + 0x13));
    fill(&mut t, out, 16, 0x17f0);
    rec(&mut t, 266, 3, |m, a| {
        let r = |m: &dyn magic::StubMem, p: u32, i: u32| f32::from_bits(m.r32(p + 4 * i));
        let (x, y) = (a[1], a[2]);
        let c = [
            r(m, x, 1) * r(m, y, 2) - r(m, x, 2) * r(m, y, 1),
            r(m, x, 2) * r(m, y, 0) - r(m, x, 0) * r(m, y, 2),
            r(m, x, 0) * r(m, y, 1) - r(m, x, 1) * r(m, y, 0),
        ];
        for (i, v) in (0u32..).zip(c) {
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0
    });
    let dir = SCRATCH + 0x500;
    for v in [[1.0f32, 2.0, 3.0, 4.0], [-0.5, 0.0, -8.0, 1.0], [0.0; 4]] {
        floats(&mut t, dir, &v);
        let c = Call {
            edx: dir,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Junk, |g, img, fr| {
            m0481::facing(g, img, fr.at, fr.edx, fr.esp);
            Ret::default()
        });
    }
}

#[test]
fn spawn() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x1070);
    let base = records(&t);
    t.set_u32(t.img.word(f + 0x0e), 0x0263_0000);
    rec(&mut t, 263, 1, |m, _| m.r32(SCRATCH + 0x600));
    rec(&mut t, 237, 5, |_, a| a[1] ^ 0x237);
    // the random draws: a sequence of floats from a counter
    rec(&mut t, 241, 1, |m, a| {
        let i = m.r32(SCRATCH + 0x604);
        m.w32(SCRATCH + 0x604, i + 1);
        let v = [0.25f32, 0.0, 0.999, 0.5, 0.125, 0.75, 0.3][(i % 7) as usize];
        m.w32(a[0], v.to_bits());
        0x2410 + i
    });
    // free records: all, two (the third spawn finds none), none
    for (n, free, k) in [(3u32, 128u32, 5u32), (3, 2, 7), (1, 0, 3), (4, 128, 0)] {
        for i in 0..128 {
            let c = if i < 128 - free { 9 } else { 0 };
            t.set_u16(base + 0x30 * i + 0x28, c);
            t.set_u16(base + 0x30 * i + 0x24, 0x1234);
        }
        t.set_u32(SCRATCH + 0x600, k);
        t.set_u32(SCRATCH + 0x604, n);
        let c = Call {
            edx: n,
            ..Call::default()
        }
        .with_xmm(&[
            ffx_magic::Xmm::f32(0.0),
            ffx_magic::Xmm::f32(0.0),
            ffx_magic::Xmm::f32(-1.5),
            ffx_magic::Xmm::f32(2.25),
        ]);
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            let (a, b) = (fr.xmm_f32(2), fr.xmm_f32(3));
            Ret::eax(m0481::spawn(g, img, fr.at, fr.esp, fr.edx, a, b))
        });
    }
}

// Mutation testing of `spawn` (M5 magic round 4, tools/mutants.py): 15
// mutants, all killed.
