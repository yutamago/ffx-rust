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
    recs(&mut t, &[(236, 3), (229, 6), (719, 9)]);
    ring(&mut t, at, 0x37);
    let (obj, src) = (SCRATCH + 0x1000, SCRATCH + 0x200);
    t.write(obj, &[0x4e; 0x1000]);
    t.write(src, &[0x5e; 0x80]);
    let c = Call {
        ecx: obj,
        edx: src,
        ..Call::default()
    };
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
    for (i, (state, count)) in (0u32..).zip(STATES) {
        let task = task(&mut t, state, count, i % 3);
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
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// 0x10007a20: indices 0 and 2.
#[test]
fn sound_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7a20);
    recs(&mut t, &[(709, 1)]);
    let obj = SCRATCH + 0x1000;
    for i in [0u32, 2] {
        ring(&mut t, at, 0x1d);
        t.write(obj, &[0x4e; 0x80]);
        let c = Call {
            ecx: obj,
            edx: i,
            ..Call::default()
        };
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
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0585::glow_clear(g, img, fr.at))
        });
    }
}
