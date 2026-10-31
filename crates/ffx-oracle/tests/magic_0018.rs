//! `magic_0018.dll` against its Rust port (`ffx_magic::m0018`): `DllMain`
//! against every bound port and the debris step. Needs `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss)]

mod magic;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{f, rec, twin};

const N: u16 = 18;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 24 records over every branch (skipped, the countdown to the list
/// removal, the tilt, falling, bouncing with and without spin, stopping),
/// stepped for several frames.
#[test]
fn bounce_step() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1ed0);
    let recs = SCRATCH + 0x1000;
    t.set_u32(t.img.word(at + 0x24), recs);
    let p = SCRATCH + 0x200;
    t.set_u32(t.img.word(at + 0x61), p);
    rec(&mut t, 713, 1, |_, a| a[0] ^ 0x713);
    rec(&mut t, 314, 1, |_, a| a[0] ^ 0x314);
    t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * 0.75 + 0.0625);
    let obj = |i: u32| SCRATCH + 0x4000 + 0x200 * i;
    // the object list: objects 1 and 23 (removed when their count runs out)
    let list = t.img.word(at + 0x53);
    let (na, nb) = (SCRATCH + 0x300, SCRATCH + 0x310);
    t.set_u32(list, na);
    t.set_u32(list + 4, nb);
    t.set_u32(list + 8, na);
    t.set_u32(list + 0xc, 2);
    t.set_u32(na, 0);
    t.set_u32(na + 4, nb);
    t.set_u32(na + 8, obj(1));
    t.set_u32(nb, na);
    t.set_u32(nb + 4, 0);
    t.set_u32(nb + 8, obj(23));
    for i in 0..24u32 {
        let r = recs + 0x44 * i;
        let x = i as f32;
        let count = match i % 6 {
            _ if i == 23 => 1,
            0 => 0,
            1 => 1,
            2 => 3,
            _ => 0x20 + i,
        };
        let mode = match i % 8 {
            _ if i == 23 => 1,
            3 | 7 => 0,
            5 => 2,
            _ => 1,
        };
        let words: [(u32, u32); 15] = [
            (0x00, f(0.3 * x - 2.0)),
            (0x04, f(4.0 - x)),
            (0x08, f(x * 0.05)),
            (0x10, f(1.5 - x * 0.1)),
            (0x14, f(if i % 5 == 0 { 0.0 } else { -0.5 * x })),
            (0x18, f(0.25 * x)),
            (0x20, obj(i)),
            (0x24, f(if i % 7 == 0 { 0.0 } else { -0.1 * x })),
            (0x28, f(if i % 8 == 4 { 50.0 } else { -0.2 })),
            (0x2c, f(if i % 3 == 0 { 100.0 } else { -1.0 - x })),
            (0x30, f(0.9)),
            (0x34, f(0.5 + x * 0.01)),
            (0x38, f(if i % 2 == 0 { 0.0 } else { x * 0.02 - 0.2 })),
            (0x3c, count),
            (0x40, mode),
        ];
        for (o, v) in words {
            t.set_u32(r + o, v);
        }
        for k in 0..3u32 {
            t.set_u32(obj(i) + 0xc0 + 4 * k, f(3.0 - x * 0.27 + k as f32));
            t.set_u32(obj(i) + 0xd0 + 4 * k, f(x - 3.0 * k as f32));
        }
        t.set_u16(obj(i) + 0x124, 0xc000 | (0x111 * u16::try_from(i).unwrap()));
        if i == 4 {
            // rotations landing on the wrap limits (pi, -pi)
            t.set_u32(r, 0);
            t.set_u32(r + 4, 0);
            t.set_u32(obj(i) + 0xc0, f(std::f32::consts::PI));
            t.set_u32(obj(i) + 0xc4, f(-std::f32::consts::PI));
        }
        if i == 10 {
            // a fall that cancels the y velocity (dy 0, vy not)
            t.set_u32(r + 0x14, f(-1.0));
            t.set_u32(r + 0x24, 0);
            t.set_u32(r + 0x28, f(1.0));
        }
    }
    for eax in [0u32, 0x1234, 7, 9, 11, 13] {
        let c = Call {
            eax,
            ..Call::default()
        };
        t.check_entry(at, c, &[SCRATCH + 0x500], Eax::Checked);
    }
}

/// The start-up (`magic_0018` 0x100022a0, `magic_0019` 0x10004620) with
/// the motion flag clear and set, two or three free pool nodes, into an
/// empty and a filled task list.
fn start_cases(n: u16, rva: u32) {
    let Some(mut t) = twin(n) else { return };
    let at = t.at(rva);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[1]);
    rec(&mut t, 215, 1, |_, a| 0x0215_0000 ^ a[0]);
    rec(&mut t, 413, 1, |_, a| 0x0413_0000 ^ a[0]);
    // the record table: the pointer the start-up sets first (+0x3c)
    let recs = w(&t, 0x3c);
    t.write(recs, &[0x6d; 0x700]);
    let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    let table = w(&t, 0x20d);
    for (flag, free, last) in [(0u32, 2u16, 0u32), (1, 3, SCRATCH + 0x600), (1, 2, 0)] {
        t.set_u32(w(&t, 0x92), flag);
        for op in [0xb8, 0xbe, 0xc8, 0xd2, 0xdc] {
            t.set_u32(w(&t, op), 0xa5a5_a5a5);
        }
        t.set_u32(w(&t, 0x6d), 1 + flag);
        t.set_u32(table + 4 + 4 * flag, 0x0bad_0000 + flag);
        for i in 1..=3 {
            t.write(node(i), &[0x5a; 0x1c]);
            t.set_u32(node(i), node(if i == 1 { 3 } else { i - 1 }));
            t.set_u32(node(i) + 4, node(if i == 3 { 1 } else { i + 1 }));
        }
        t.set_u32(w(&t, 0x10e), node(1));
        t.set_u16(w(&t, 0xf4), free);
        t.write(SCRATCH + 0x600, &[0x44; 0x1c]);
        t.set_u32(w(&t, 0x169), last);
        t.set_u32(w(&t, 0x179), last);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn start() {
    start_cases(N, 0x22a0);
    start_cases(19, 0x4620);
}
