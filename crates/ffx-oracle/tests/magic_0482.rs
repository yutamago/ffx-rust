//! `magic_0482.dll` against its Rust port (`ffx_magic::m0482`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0482};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, recs, twin};

const N: u16 = 482;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10003550: no parent, a parent with no children and one with some.
#[test]
fn insert() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x3550);
    recs(&mut t, &[(235, 1), (230, 3), (325, 2)]);
    let (group, node, parent, last) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x2000,
        SCRATCH + 0x2100,
    );
    for case in 0..3 {
        t.write(node, &[0x47; 0x160]);
        t.write(parent, &[0x48; 0x20]);
        t.set_u32(parent + 0x10, if case == 1 { parent } else { last });
        t.set_u16(parent + 0x18, 5);
        t.write(last, &[0x49; 0x10]);
        for (i, v) in (0u32..).zip([node, if case == 0 { 0 } else { parent }, 0xa, 0xb, 0xc]) {
            t.set_u32(group + 4 * i, v);
        }
        t.check(at, Call::this(group), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0482::insert(g, img, fr.at, fr.ecx))
        });
    }
}

/// `r14::insert_0387` (`magic_0387` 0x10004b80): no parent, a parent with
/// no children and one with some; the node's words old before.
#[test]
fn insert_0387() {
    let Some(mut t) = twin(387) else { return };
    let at = t.at(0x4b80);
    recs(&mut t, &[(235, 1), (230, 3), (325, 2)]);
    let (group, node, parent, last) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x2000,
        SCRATCH + 0x2100,
    );
    for case in 0..3 {
        t.write(node, &[0x47; 0x160]);
        t.write(parent, &[0x48; 0x20]);
        t.set_u32(parent + 0x10, if case == 1 { parent } else { last });
        t.set_u16(parent + 0x18, 5);
        t.write(last, &[0x49; 0x10]);
        for (i, v) in (0u32..).zip([node, if case == 0 { 0 } else { parent }, 0xa, 0xb, 0xc]) {
            t.set_u32(group + 4 * i, v);
        }
        t.check(at, Call::this(group), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(ffx_magic::r14::insert_0387(g, img, fr.at, fr.ecx))
        });
    }
}

/// 0x10006d00: every state, both answers of slots 415 and 411.
#[test]
fn task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6d00);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 415, 1, move |m, _| m.r32(cell));
    rec(&mut t, 411, 1, move |m, _| m.r32(cell));
    recs(&mut t, &[(228, 2)]);
    t.set_u32(t.img.word(at + 0x57), 0x00a5_0000);
    let (task, r, node, prev, next) = (
        SCRATCH + 0x100,
        SCRATCH + 0x200,
        SCRATCH + 0x300,
        SCRATCH + 0x400,
        SCRATCH + 0x500,
    );
    for (state, wait, answer) in [
        (0u32, 0u32, 0u32),
        (1, 3, 0),
        (1, 0, 0x79),
        (1, 0xffff_fff0, 0x78),
        (10, 0, 0x79),
        (10, 0, 0),
        (11, 0, 0),
        (11, 1, 1),
        (5, 0, 0),
        (12, 0, 0),
        (0x4e20, 0, 0),
    ] {
        t.write(r, &[0x3c; 0x40]);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x34, wait);
        t.set_u32(cell, answer);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}
