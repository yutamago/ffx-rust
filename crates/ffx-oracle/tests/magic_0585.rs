//! `magic_0585.dll` against its Rust port (`ffx_magic::m0585`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::many_single_char_names
)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0585};
use magic::{Call, Eax, SCRATCH, Twin};
use magic_hi::models;
use magic_owna::{recs, twin};

const N: u16 = 585;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The task ring whose last word is the operand `last` of the code at
/// `at`: a dirty tail.
fn ring(t: &mut Twin, at: u32, last: u32) {
    let tail = SCRATCH + 0x3800;
    t.write(tail, &[0x1b; 0x20]);
    t.set_u32(t.img.word(at + last), tail);
}

/// 0x100078a0: one source.
#[test]
fn setup() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x78a0);
    recs(&mut t, &[(236, 3), (719, 9)]);
    // slot 229 keeps the vector it gets
    magic_owna::rec(&mut t, 229, 6, |m, a| {
        magic_owna::dump(m, 229, a[2], 4);
        0x5a5a_0229 ^ a[0]
    });
    ring(&mut t, at, 0x37);
    // the ring's flag word with bit 0 set, so its clear shows
    t.set_u32(t.img.word(at + 0x4f), 0x00f1_a6c3);
    let (obj, src) = (SCRATCH + 0x1000, SCRATCH + 0x200);
    t.write(obj, &[0x4e; 0x1000]);
    t.write(src, &magic_h6::varied(0x80, 0x5e));
    let c = Call {
        ecx: obj,
        edx: src,
        ..Call::default()
    };
    magic_h6::scrub(&mut t);
    t.check(at, c, &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0585::setup(g, img, fr))
    });
}

/// A task `[node, r]` with its node in a ring of three.
fn task(t: &mut Twin, state: u32, count: i32, index: u32) -> u32 {
    let (task, r, node, prev, next) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x240,
        SCRATCH + 0x280,
    );
    t.write(r, &[0x2d; 0x80]);
    t.write(prev, &[0x2e; 0x80]);
    t.set_u32(task, node);
    t.set_u32(task + 4, r);
    t.set_u32(node + 4, next);
    t.set_u32(node + 8, prev);
    t.set_u32(r + 0x18, state);
    t.set_u32(r + 0x34, count.cast_unsigned());
    t.set_u32(r + 0x3c, index);
    task
}

const STATES: [(u32, i32); 7] = [
    (0, 0),
    (1, 5),
    (1, 1),
    (1, 0),
    (1, -2),
    (10, 3),
    (0x2710, 3),
];

/// 0x10007990: every state, the counter above and at 0.
#[test]
fn sound() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7990);
    recs(&mut t, &[(709, 1)]);
    // the half-word after each sound id non-zero (an id read as u32 shows)
    let ids = t.img.word(at + 0x27);
    for i in 0..3 {
        t.set_u16(ids + 8 * i + 2, 0x5a5a);
    }
    for (i, (state, count)) in (0u32..).zip(STATES) {
        let task = task(&mut t, state, count, i % 3);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// 0x10007a90: as [`sound`].
#[test]
fn two_sounds() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7a90);
    recs(&mut t, &[(709, 2)]);
    for (state, count) in STATES {
        let task = task(&mut t, state, count, 0);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// 0x10007a20: indices 0 and 2.
#[test]
fn sound_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7a20);
    recs(&mut t, &[(709, 1)]);
    let ids = t.img.word(at + 0x37);
    for i in 0..3 {
        t.set_u16(ids + 8 * i + 2, 0x5a5a);
    }
    let obj = SCRATCH + 0x1000;
    for i in [0u32, 2] {
        ring(&mut t, at, 0x1d);
        t.write(obj, &[0x4e; 0x80]);
        let c = Call {
            ecx: obj,
            edx: i,
            ..Call::default()
        };
        magic_h6::scrub(&mut t);
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0585::sound_start(g, img, fr))
        });
    }
}

/// 0x10007b20: one task.
#[test]
fn two_sounds_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7b20);
    recs(&mut t, &[(709, 1), (719, 3)]);
    ring(&mut t, at, 0x23);
    let list = t.img.word(at + 0x62);
    t.write(list, &[0; 0x20]);
    t.set_u32(list + 0xc, list);
    t.set_u32(list + 0x10, list);
    let obj = SCRATCH + 0x1000;
    t.write(obj, &[0x4e; 0x100]);
    magic_h6::scrub(&mut t);
    t.check(at, Call::this(obj), &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0585::two_sounds_start(g, img, fr))
    });
}

/// 0x10008fd0: no models, empty and missing ones, other kinds.
#[test]
fn glow_clear() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x8fd0);
    let g_at = t.img.word(at + 0x8);
    let sets: [&[Option<&[(u32, u8, i16)]>]; 3] = [
        &[],
        &[None, Some(&[])],
        &[
            Some(&[(0x3700, 0x48, 3), (0x7700, 0x48, 1), (0x3580, 0x47, 2)]),
            None,
            Some(&[(0x3701, 0x48, 1), (0x3700, 0x48, 0), (0x3700, 0x48, -1)]),
        ],
    ];
    for set in sets {
        models(&mut t, g_at, set);
        magic_h6::scrub(&mut t);
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0585::glow_clear(g, img, fr.at))
        });
    }
}

/// 0x10007bd0: the drift task through every state, the level at and off
/// its limits, the wait out or not.
#[test]
fn drift_task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7bd0);
    let (task, r, node, prev, next) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x240,
        SCRATCH + 0x280,
    );
    for state in 0..14u32 {
        for (wait, level) in [(5u32, 0x4cu8), (0, 0x04), (0xffff_fff0, 0x30)] {
            t.write(r, &[0x3c; 0x100]);
            t.set_u32(r + 0x18, state);
            t.set_u32(r + 0x34, wait);
            t.set_u8(r + 0xc3, level);
            // the flag word and the one after it distinct
            t.set_u32(r + 0xec, 0x0000_0100);
            t.set_u32(r + 0xf0, 0x0055_0000);
            for (i, o) in [0x30u32, 0x38, 0x3c, 0x40, 0xe0, 0xe4]
                .into_iter()
                .enumerate()
            {
                t.set_u32(r + o, 0x0100_0000 * (i as u32 + 1) + 0x111 * state);
            }
            t.set_u32(task, node);
            t.set_u32(task + 4, r);
            t.set_u32(node + 4, next);
            t.set_u32(node + 8, prev);
            magic_h6::scrub(&mut t);
            t.check_entry(at, Call::default(), &[task], Eax::Checked);
        }
    }
}

/// 0x10008250: the drift object's set-up; slot 234 keeps the local
/// colour it gets.
#[test]
fn drift_setup() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x8250);
    let ring_last = t.img.word(at + 0x35);
    let ring_flags = t.img.word(at + 0x54);
    magic_owna::rec(&mut t, 234, 3, |m, a| {
        let k = m.r32(magic_owna::row(234));
        magic_owna::dump(m, 0x100 + k % 0x40, a[1], 4);
        0x234
    });
    recs(&mut t, &[(544, 2), (588, 2), (232, 2)]);
    let obj = SCRATCH + 0x1000;
    let old = SCRATCH + 0x800;
    t.write(obj, &[0x5b; 0x100]);
    t.write(old, &[0x6c; 0x40]);
    t.set_u32(ring_last, old);
    t.set_u32(ring_flags, 0x8000_0007);
    magic_h6::scrub(&mut t);
    t.check(at, Call::this(obj), &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0585::drift_setup(g, img, fr))
    });
}

/// 0x10007d40: the light object set up under a parent, the level placed
/// and not (slot 363 above and below the maximum).
#[test]
fn light_object() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7d40);
    let lvl = SCRATCH + 0x40;
    magic_owna::rec(&mut t, 363, 3, move |m, _| m.r32(lvl));
    magic_owna::rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    recs(&mut t, &[(235, 1), (357, 3), (356, 3), (704, 3), (362, 4)]);
    let (obj, parent, tail) = (SCRATCH + 0x1000, SCRATCH + 0x2000, SCRATCH + 0x2100);
    let g = t.img.word(at + 0x88);
    let gv = SCRATCH + 0x2200;
    t.set_u32(g, gv);
    t.set_u32(gv, 0x0bad_0001);
    for v in [5u32, 0xffff_ff00] {
        t.set_u32(lvl, v);
        t.write(obj, &[0x5b; 0x100]);
        t.write(parent, &[0x2c; 0x60]);
        t.set_u32(parent + 0x54, SCRATCH + 0x2300);
        // the light node's list head (the immediate +0x6a): empty
        let head = t.img.word(at + 0x6a);
        t.write(head, &[0x1d; 0x20]);
        t.set_u32(head + 0x10, head);
        t.write(tail, &[0x3d; 0x10]);
        t.set_u32(t.img.word(at + 0x21), tail);
        magic_h6::scrub(&mut t);
        let c = Call {
            ecx: obj,
            edx: parent,
            ..Call::default()
        };
        t.check(at, c, &[0x0077_0001], Eax::Checked, |g, img, fr| {
            m0585::light_object(g, img, fr)
        });
    }
}
