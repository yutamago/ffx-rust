//! `magic_0019.dll` against its Rust port (`ffx_magic::m0019`): `DllMain`
//! against every bound port and the spinning debris step. Needs `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss)]

mod magic;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{f, rec, twin};

const N: u16 = 19;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 24 records over every branch (skipped, the delay, the countdown to
/// the list removal, the tilt, fast and slow rotations), stepped for
/// several frames.
#[test]
fn spin_step() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x43c0);
    let recs = SCRATCH + 0x1000;
    t.set_u32(t.img.word(at + 0x34), recs);
    let p = SCRATCH + 0x200;
    t.set_u32(t.img.word(at + 0x6f), p);
    rec(&mut t, 713, 1, |_, a| a[0] ^ 0x713);
    let obj = |i: u32| SCRATCH + 0x4000 + 0x200 * i;
    // the object list: objects 1 and 23 (removed when their count runs out)
    let list = t.img.word(at + 0x61);
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
        let r = recs + 0x34 * i;
        let x = i as f32;
        let count = match i % 6 {
            _ if i == 23 => 1,
            0 => 0,
            1 => 1,
            2 => 3,
            _ => 0x20 + i,
        };
        let delay = if i % 4 == 3 && i != 23 { 2 } else { 0 };
        let words: [(u32, u32); 11] = [
            (0x00, f(0.3 * x - 2.0)),
            (0x04, f(if i % 2 == 0 { 4.0 - x } else { 1e-4 })),
            (0x08, f(x * 0.05)),
            (0x10, f(1.5 - x * 0.1)),
            (0x14, f(-0.5 * x)),
            (0x18, f(0.25 * x)),
            (0x20, obj(i)),
            (0x24, f(-0.1 * x)),
            (0x28, f(0.9)),
            (0x2c, count),
            (0x30, delay),
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
    }
    for eax in [0u32, 0x1234, 7, 9, 11, 13] {
        let c = Call {
            eax,
            ..Call::default()
        };
        t.check_entry(at, c, &[SCRATCH + 0x500], Eax::Checked);
    }
}
