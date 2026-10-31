//! `magic_0645.dll` against its Rust port (`ffx_magic::m0645`). Needs
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

use ffx_magic::{Ret, m0645};
use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, recs, twin};

const N: u16 = 645;

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

/// 0x10006ec0: one object.
#[test]
fn setup() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6ec0);
    recs(&mut t, &[(719, 3)]);
    ring(&mut t, at, 0x2c);
    let (obj, src) = (SCRATCH + 0x1000, SCRATCH + 0x200);
    t.write(obj, &[0x4e; 0x1000]);
    let c = Call {
        ecx: obj,
        edx: src,
        ..Call::default()
    };
    t.check(at, c, &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0645::setup(g, img, fr))
    });
}

/// 0x10007b20: every state; the object not ready and ready, with bit 0
/// of `+0x90` clear and set.
#[test]
fn value_task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7b20);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 20, 1, move |m, _| m.r32(cell));
    rec(&mut t, 411, 1, move |m, _| m.r32(cell));
    recs(&mut t, &[(30, 1), (36, 3), (228, 2)]);
    let (task, r, node, prev, next, o) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x240,
        SCRATCH + 0x280,
        SCRATCH + 0x2000,
    );
    for (state, ready, bits) in [
        (0u32, 0u32, 0u16),
        (0, 1, 0),
        (0, 0, 1),
        (0, 1, 0x8001),
        (0x2710, 0, 0),
        (7, 0, 0),
    ] {
        t.write(r, &[0x2d; 0x40]);
        t.write(o, &[0x3d; 0x100]);
        t.write(prev, &[0x2e; 0x80]);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x1c, 5);
        t.set_u32(r + 0x20, o);
        t.set_u16(o + 0x90, bits);
        t.set_u32(cell, ready);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// 0x10007bb0: an object with bit 0 of `+0x90` clear and set.
#[test]
fn value_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7bb0);
    recs(&mut t, &[(30, 1), (36, 2), (228, 2)]);
    let (task, o) = (SCRATCH + 0x1000, SCRATCH + 0x2000);
    for bits in [0u16, 0x8001] {
        ring(&mut t, at, 0x1c);
        t.write(task, &[0x4e; 0x40]);
        t.write(o, &[0x3d; 0x100]);
        t.set_u16(o + 0x90, bits);
        let c = Call {
            ecx: task,
            edx: o,
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0645::value_start(g, img, fr))
        });
    }
}

/// 0x10007a40: one object.
#[test]
fn mover() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7a40);
    recs(&mut t, &[(210, 2), (229, 5)]);
    ring(&mut t, at, 0x30);
    let (obj, src) = (SCRATCH + 0x1000, SCRATCH + 0x200);
    t.write(obj, &[0x4e; 0x400]);
    t.write(src, &[0x5e; 0x80]);
    let c = Call {
        ecx: obj,
        edx: src,
        ..Call::default()
    };
    t.check(at, c, &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0645::mover(g, img, fr))
    });
}
