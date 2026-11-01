//! `magic_0477.dll` against its Rust port (`ffx_magic::m0477`): the
//! overlay table's part functions. Needs `FFX_GAME_DIR`.

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

use magic::{Call, Eax, SCRATCH, Twin};
use magic_hi::f;
use magic_owna::{rec, recs, twin};

const N: u16 = 477;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The DLL's state for the part functions: the point globals each copy
/// reads (filled with distinct floats), the slots' stubs, slot 712's
/// answer from `answer`.
fn setup(t: &mut Twin, answer: u32) {
    for (a, n) in [
        (0x1005_c600u32, 0x100u32),
        (0x1005_c7c0, 0x40),
        (0x1005_cb30, 0x40),
    ] {
        for i in 0..n / 4 {
            t.set_u32(a + 4 * i, f(0.75 * i as f32 - 3.0));
        }
    }
    let o = SCRATCH + 0x100;
    t.set_u32(0x1005_cb34, o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    let table = SCRATCH + 0x200;
    for i in 0..16 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    rec(t, 941, 1, move |_, _| table);
    for slot in [237u32, 268] {
        rec(t, slot, 3, move |m, a| {
            // 268 (a, n, out), 237 (a, n, 0, out)
            let at = if slot == 268 { a[2] } else { a[3] };
            for i in 0..4 {
                m.w32(at + 4 * i, f(i as f32 * 1.5 + a[1] as f32));
            }
            slot
        });
    }
    let cell = SCRATCH + 0x10;
    t.set_u32(cell, answer);
    rec(t, 712, 2, move |m, _| m.r32(cell));
    recs(
        t,
        &[
            (723, 1),
            (724, 2),
            (258, 1),
            (231, 3),
            (230, 2),
            (713, 1),
            (714, 5),
        ],
    );
}

/// Every part function, with slot 712 saying 0 and 1.
#[test]
fn part_functions() {
    for answer in [0u32, 1] {
        let Some(mut t) = twin(N) else { return };
        setup(&mut t, answer);
        for rva in [
            0x1070u32, 0x1250, 0x1430, 0x1580, 0x1670, 0x17b0, 0x1850, 0x1980, 0x1d50, 0x1de0,
        ] {
            let at = t.at(rva);
            t.check_entry(at, Call::default(), &[], Eax::Checked);
        }
    }
}

/// 0x10001f10: the start-up (the blocks dirty before).
#[test]
fn start_up() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1f10);
    let d = SCRATCH + 0x400;
    for i in 0..8 {
        t.set_u32(d + 4 * i, 0x0d00_0000 + i);
    }
    rec(&mut t, 803, 2, |_, _| 0x2100_0000);
    rec(&mut t, 719, 3, move |_, a| d + (a[1] & 0xc));
    rec(&mut t, 941, 3, |_, a| 0x0941_0000 ^ a[0]);
    rec(&mut t, 260, 1, |_, a| 0x0260_0000 ^ a[0]);
    recs(&mut t, &[(718, 3), (222, 4), (711, 3), (710, 4)]);
    for (a, n) in [
        (0x1004_a200u32, 0x30u32),
        (0x1004_a420, 0x110),
        (0x1005_c6e0, 0x110),
        (0x1005_c800, 0xd0),
        (0x1005_c900, 0x110),
        (0x1005_ca20, 0x110),
        (0x1005_cb80, 0x110),
    ] {
        t.write(a, &vec![0x5e; n as usize]);
    }
    t.check_entry(at, Call::default(), &[0x00a6_0000], Eax::Checked);
}
