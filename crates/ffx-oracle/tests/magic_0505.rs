//! `magic_0505.dll` against its Rust port (`ffx_magic::m0505`). Needs
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

use ffx_magic::{Ret, m0505};
use magic::{Call, Eax, SCRATCH};
use magic_hi::f;
use magic_owna::{rec, recs, twin};

const N: u16 = 505;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The table entries: the two starts, the draw, the registration (0, 1
/// and 3 records).
#[test]
fn entries() {
    let Some(mut t) = twin(N) else { return };
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let d = SCRATCH + 0x100;
    t.set_u32(0x1004_ac78, d);
    for i in 0..8 {
        t.set_u32(d + 4 * i, 0x00b0_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, a| table ^ (a[0] & 0xf0));
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 + a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    recs(
        &mut t,
        &[
            (714, 5),
            (723, 1),
            (724, 2),
            (258, 1),
            (231, 3),
            (230, 2),
            (712, 2),
            (779, 4),
            (804, 1),
        ],
    );
    for i in 0..4u32 {
        t.set_u32(0x1004_70a0 + 4 * i, f(i as f32 * 3.5 - 1.0));
    }
    for rva in [0x1180u32, 0x1380, 0x1210] {
        t.check_entry(t.at(rva), Call::default(), &[], Eax::Checked);
    }
    let reg = t.at(0x18d0);
    let recs_at = SCRATCH + 0x1000;
    t.set_u32(t.img.word(reg + 0xa), recs_at);
    for i in 0..3 {
        t.set_u32(recs_at + 0x30 + 0x94 * i, 0x00c0_0000 + i);
    }
    for n in [0u32, 1, 3] {
        t.set_u32(t.img.word(reg + 0x14), n);
        t.check_entry(reg, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10002af0: the pool cleared (dirty before).
#[test]
fn clear() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2af0);
    let first = t.img.word(at + 0x10);
    t.write(first, &vec![0x3d; 0x94 * 0x64]);
    let head = t.img.word(at + 0xb);
    t.write(head, &[0x3e; 0x5c]);
    t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0505::clear(g, img, fr.at))
    });
}

/// 0x10001a20: the frame off (slot 253 0, the flag clear) and on with
/// slot 261's answers (the timelines idle).
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1a20);
    for op in [0x4b, 0x6b] {
        let tl = t.img.word(at + op);
        t.set_u32(tl + 8, 0);
    }
    let cell = SCRATCH + 0x10;
    rec(&mut t, 253, 0, move |m, _| (m.r32(cell) >> 4) & 1);
    // slot 261 (k) answers bit k of the cell
    rec(&mut t, 261, 1, move |m, a| (m.r32(cell) >> (a[0] & 3)) & 1);
    recs(&mut t, &[(718, 1), (222, 4), (378, 1), (214, 1)]);
    for (on, bits) in [
        (1u32, 0b0_0000u32),
        (0, 0b1_0000),
        (1, 0b1_0001),
        (1, 0b1_0010),
        (1, 0b1_0111),
    ] {
        t.set_u32(t.img.word(at + 0x16), on);
        t.set_u32(cell, bits);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
