//! `magic_0477.dll` against its Rust port (`ffx_magic::m0477`): the
//! overlay table's part functions. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

mod magic;
mod magic_h6;
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

/// The part pointer's cell and the game flag's (the same globals in
/// every copy).
fn cells(t: &Twin) -> (u32, u32) {
    let at = t.at(0x1070);
    (magic_h6::cell(t, at, 0x2d), magic_h6::cell(t, at, 0xed))
}

/// The DLL's state for the part functions: the point globals each copy
/// reads (filled with distinct floats), the slots' stubs, slot 712's
/// answer from `answer`. Slots 258, 712 and 713 keep the part pointer's
/// cell and the game flag as they find them, and 230 and 712 move them on
/// (as the game's part code may), so a store left out before or after a
/// call shows.
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
    let (gp, lock) = cells(t);
    magic_h6::rec_cells(t, 712, 2, &[gp, lock], move |m, _| {
        m.w32(gp, 0x6666_0712);
        m.w32(lock, 0x0712_0712);
        m.r32(cell)
    });
    for slot in [258u32, 713] {
        magic_h6::rec_cells(t, slot, 1, &[gp, lock], move |_, a| {
            (0x5a5a_0000 + slot) ^ a.first().copied().unwrap_or(0)
        });
    }
    rec(t, 230, 2, move |m, _| {
        m.w32(gp, 0x6666_0230);
        0x5a5a_0230
    });
    recs(t, &[(723, 1), (724, 2), (231, 3), (714, 5)]);
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
            let (gp, lock) = cells(&t);
            t.set_u32(gp, 0x7777_7777);
            t.set_u32(lock, 0x00a1_0000);
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
    // slot 711 keeps the game flag
    let lock = magic_h6::cell(&t, at, 0x8f);
    t.set_u32(lock, 0x00a1_0000);
    magic_h6::rec_cells(&mut t, 711, 3, &[lock], |_, a| {
        0x5a5a_0000 ^ a.first().copied().unwrap_or(0)
    });
    recs(&mut t, &[(718, 3), (222, 4), (710, 4)]);
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
    // the word below the first record (a cleared word one record early
    // shows)
    let rec0 = t.img.word(at + 0x2e1);
    t.set_u32(rec0 - 0x10, 0x5e5e_5e5f);
    t.check_entry(at, Call::default(), &[0x00a6_0000], Eax::Checked);
}

/// 0x10002350 (`r13_start::frame_0477`): off (slot 253 0, the flag 0);
/// on with slot 261's answers (bit `k` for timeline `k`, bit 6 the stop):
/// none, all with the fifth live and dead, the stop asked and not.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2350);
    let w = |t: &Twin, o: u32| t.img.word(at + o);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 253, 0, move |m, _| (m.r32(cell) >> 8) & 1);
    rec(&mut t, 261, 1, move |m, a| (m.r32(cell) >> (a[0] & 7)) & 1);
    recs(&mut t, &[(214, 1)]);
    let p = SCRATCH + 0x30;
    t.set_u32(w(&t, 0x2b), p);
    t.set_u32(w(&t, 0x30), 0x00a2_0030);
    t.set_u32(w(&t, 0x37), 0x00a2_0037);
    t.set_u32(w(&t, 0x106), 0x00a2_0106);
    let stop = t.img.rel32_target(at + 0xfb);
    let on = w(&t, 0x22);
    for blk in [0x1005_c800u32, 0x1005_c85c] {
        t.set_u32(blk, 0);
    }
    for (flag, bits, fifth_live) in [
        (1u32, 0x000u32, true),
        (0, 0x100, true),
        (1, 0x100, true),
        (1, 0x13e, true),
        (1, 0x13e, false),
        (1, 0x17e, true),
        (1, 0x140, true),
        (1, 0x102, false),
    ] {
        t.set_u32(on, flag);
        t.set_u32(cell, bits);
        for (k, op) in (0u32..).zip([0x57u32, 0x77, 0x97, 0xb7, 0xd7]) {
            let tl = w(&t, op);
            if k < 4 || fifth_live {
                magic_h6::live_timeline(&mut t, tl, SCRATCH + 0x600 + 0x20 * k, 0x40 + k);
            } else {
                t.write(tl, &[0; 0x110]);
            }
        }
        t.set_u32(t.img.word(stop + 0x02), 1);
        t.set_u32(t.img.word(stop + 0x0d), 1);
        t.set_u32(t.img.word(stop + 0x18), SCRATCH + 0x700);
        t.write(SCRATCH + 0x700, &[0; 0x18]);
        t.set_u32(SCRATCH + 0x704, 1);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001b60 (`r13_start::sparks_0477`): three free sprites of the five
/// asked for.
#[test]
fn sparks() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1b60);
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4u32 {
            m.w32(a[2] + 4 * i, f(1.5 + i as f32 * 2.25));
        }
        0x268
    });
    let k = SCRATCH + 0x40;
    rec(&mut t, 324, 0, move |m, _| {
        let n = m.r32(k);
        m.w32(k, n + 1);
        n.wrapping_mul(0x9e37_79b9) >> 7
    });
    rec(&mut t, 403, 1, |_, a| a[0]);
    t.stub_st0(403, |_, a| f64::from(f32::from_bits(a[0])) * 0.5);
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, a| table ^ (a[0] & 0xf0));
    rec(&mut t, 942, 1, |_, a| 0x0942_0000 ^ a[0]);
    let pool = t.img.word(at + 0x47);
    let sprites = SCRATCH + 0x1000;
    t.write(sprites, &vec![0x2a; 0x94 * 4]);
    for i in 0..4u32 {
        t.set_u32(sprites + 0x94 * i, u32::from(i == 2));
    }
    t.set_u32(pool, 4);
    t.set_u32(pool + 4, sprites);
    t.set_u32(t.img.word(at + 0x1f), 0x00c0_001f);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}
