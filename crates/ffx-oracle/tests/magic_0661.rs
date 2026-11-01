//! `magic_0661.dll` against its Rust port (`ffx_magic::m0661`). Needs
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
use magic_owna::{recs, twin};

const N: u16 = 661;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100015e0: no record; the byte 1 and other.
#[test]
fn sound_stop() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x15e0);
    recs(&mut t, &[(932, 0), (931, 2), (847, 4), (922, 6)]);
    let r = SCRATCH + 0x1000;
    for (has, byte) in [(false, 1u8), (true, 1), (true, 2)] {
        t.set_u32(t.img.word(at + 0x3), if has { r } else { 0 });
        t.write(r, &[0x3c; 0x900]);
        t.set_u8(r + 0x180, byte);
        // the sound handle and the word after it distinct
        t.set_u32(r + 0x830, 0x0083_0000);
        t.set_u32(r + 0x834, 0x0083_4000);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001c20: a free slot first, later, and none.
#[test]
fn glow_part() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1c20);
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(t.img.word(at + 0x10), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    t.set_u32(t.img.word(at + 0x4), SCRATCH + 0x24);
    magic_owna::rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5), (325, 2)]);
    let slots = t.img.word(at + 0x3f);
    let obj = SCRATCH + 0x1000;
    for free in [Some(0u32), Some(9), None] {
        for i in 0..0xe {
            let on = u32::from(i % 2 == 1 || Some(i) == free);
            t.set_u32(slots + 8 * i - 4, on);
            t.set_u32(slots + 8 * i, u32::from(Some(i) != free));
        }
        t.write(obj, &[0x5b; 0x80]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x10001700: the first part and another; free slots 0, 3 and none.
#[test]
fn beam_part() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1700);
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(t.img.word(at + 0x43), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    t.set_u32(t.img.word(at + 0x38), SCRATCH + 0x24);
    magic_owna::rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5), (325, 2)]);
    let slots = t.img.word(at + 0x17);
    let base = t.img.word(at + 0xd);
    for (obj, free) in [(base, Some(0u32)), (base + 0x80, Some(3)), (base, None)] {
        for i in 0..0xe {
            let on = u32::from(i % 2 == 1 || Some(i) == free);
            t.set_u32(slots + 8 * i - 4, on);
            t.set_u32(slots + 8 * i, u32::from(Some(i) != free));
        }
        t.write(obj, &[0x5b; 0x80]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x10001a50: the start at an actor for the array's first object and
/// another, the free actor slot at 2, 12, 7 and none.
#[test]
fn start_at() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1a50);
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(t.img.word(at + 0x10), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    magic_owna::rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5), (325, 2)]);
    let slots = t.img.word(at + 0x4d);
    let base = t.img.word(at + 0x2b);
    for (k, free) in [(0u32, Some(2u32)), (1, Some(12)), (0, Some(7)), (1, None)] {
        for i in 0..14u32 {
            let p = slots + 8 * i;
            t.set_u32(p - 4, if i % 3 == 0 { 2 } else { 1 });
            t.set_u32(p, 0x0055_0000 + i);
        }
        if let Some(i) = free {
            t.set_u32(slots + 8 * i - 4, 1);
            t.set_u32(slots + 8 * i, 0);
        }
        let obj = base + 0x80 * k;
        t.write(obj, &[0x5b; 0x80]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}
