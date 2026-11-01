//! `magic_0588.dll` against its Rust port (`ffx_magic::m0588`). Needs
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

use magic::{Call, Eax, SCRATCH, Twin};
use magic_hi::pool;
use magic_owna::{rec, recs, twin};

const N: u16 = 588;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The current actor `i` of the code at `at` (record operand `e`,
/// table operand `table`).
fn actor(t: &mut Twin, at: u32, (e, table): (u32, u32), i: u8) {
    let rec = SCRATCH + 0x80;
    t.set_u32(t.img.word(at + e), rec);
    t.set_u8(rec + 1, i);
    let table = t.img.word(at + table);
    for k in 0..8 {
        t.set_u32(table + 4 * k, 0x00ac_0000 + k);
    }
}

/// 0x10001e10: actors 0 and 3.
#[test]
fn sound() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1e10);
    rec(&mut t, 0, 1, |_, a| a[0] ^ 0x00f0_0000);
    recs(&mut t, &[(26, 3), (10, 5), (257, 7)]);
    for i in [0u8, 3] {
        actor(&mut t, at, (0x3e, 0x55), i);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10002510: the list empty and not.
#[test]
fn part_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2510);
    let r = SCRATCH + 0x4000;
    t.set_u32(t.img.word(at + 0x3), r);
    let take = t.img.rel32_target(at + 0x17);
    let head = SCRATCH + 0x300;
    for empty in [true, false] {
        pool(&mut t, take, SCRATCH + 0x1000, 3);
        t.write(r + 0xd00, &[0x3b; 0x200]);
        t.write(head, &[0x4d; 0x20]);
        t.set_u32(t.img.word(at + 0x1d), u32::from(!empty));
        t.set_u32(t.img.word(at + 0x31), head);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10002800: actors 1 and 5.
#[test]
fn colours() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2800);
    recs(&mut t, &[(357, 3), (356, 6), (355, 8), (354, 10)]);
    for i in [1u8, 5] {
        actor(&mut t, at, (0x1, 0x12), i);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10002860: one record.
#[test]
fn light() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2860);
    recs(&mut t, &[(235, 1), (328, 3), (329, 3), (229, 6), (248, 9)]);
    let r = SCRATCH + 0x4000;
    t.set_u32(t.img.word(at + 0x11), r);
    t.write(r + 0xb00, &[0x3e; 0x400]);
    t.set_u32(r + 0xdf4, 0x3fc9_0fdb);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}
