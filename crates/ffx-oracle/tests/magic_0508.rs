//! `magic_0508.dll` against its Rust port (`ffx_magic::m0508`). Needs
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

use magic::{Call, Eax, SCRATCH};
use magic_hi::f;
use magic_owna::{rec, twin};

const N: u16 = 508;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001b40: two actors.
#[test]
fn e1b40() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1b40);
    let b = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x11), b);
    let table = t.img.word(at + 0x1d);
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x00ac_0000 + i);
    }
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4 {
            m.w32(a[i], f(i as f32 * 1.25));
        }
        0x262
    });
    rec(&mut t, 268, 3, |m, a| {
        let y = m.r32(a[2] + 4);
        m.w32(a[2] + 4, y ^ a[1] ^ a[0]);
        0x268
    });
    let obj = SCRATCH + 0x1000;
    for actor in [2u32, 5] {
        // the byte after the actor index non-zero (an index read as u16 shows)
        t.set_u32(b, 0x0017_3c00 | actor);
        t.write(obj, &[0x77; 0x110]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x10001a20: the counts at 0, 1 and 2; the motion script, the objects,
/// `G_B`, the flag bit and the half-word each stopping it.
#[test]
fn task() {
    let Some(mut t) = twin(508) else { return };
    let at = t.at(0x1a20);
    let w = |t: &magic::Twin, op: u32| t.img.word(at + op);
    rec(&mut t, 531, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[1] + 4 * i, f(1.0 + i as f32));
        }
        0x531
    });
    rec(&mut t, 217, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[1] + 4 * i, f(0.5 * i as f32));
        }
        0x217
    });
    for s in [234u32, 233] {
        rec(&mut t, s, 3, move |m, a| {
            for i in 0..4 {
                m.w32(a[0] + 4 * i, f(0.25 + (s + i) as f32));
            }
            s
        });
    }
    magic_owna::recs(&mut t, &[(624, 1), (213, 2)]);
    for op in [0x31, 0x8f, 0xe0] {
        t.set_u32(w(&t, op), 0x00cc_0000 + op);
    }
    let fl = t.img.rel32_target(at + 0x66);
    let fw = |t: &magic::Twin, op: u32| t.img.word(fl + op);
    t.set_u32(fw(&t, 0x18), 0x0cc0_0007);
    let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    let ms = w(&t, 0x9f);
    let ro = t.img.rel32_target(at + 0xab);
    let (n, task) = (SCRATCH + 0x5000, SCRATCH + 0x5100);
    // (G_A, G_B, script done, objects running, flag, half)
    for (a, b, done, running, flag, half) in [
        (0u32, 0u32, true, false, 0x8000u32, 0u16),
        (1, 1, true, false, 0x8000, 0),
        (2, 2, true, false, 0x8000, 3),
        (0, 0, false, false, 0x8000, 0),
        (0, 0, true, true, 0x8000, 0),
        (0, 5, true, false, 0x8000, 0),
        (0, 0, true, false, 0x7fff, 0),
        (0, 0, true, false, 0x8000, 3),
        (0xffff_fff0, 0xffff_fff0, true, false, 0x8000, 0),
    ] {
        t.set_u32(w(&t, 0x11), a);
        t.set_u32(w(&t, 0x78), b);
        t.set_u32(w(&t, 0xc3), flag);
        t.set_u16(w(&t, 0xd0), half);
        t.set_u32(w(&t, 0x6f), 9);
        t.set_u32(w(&t, 0xf1), 9);
        t.set_u32(ms + 0x14, u32::from(done));
        t.set_u16(ms + 0x18, 3);
        magic_hi::objects(&mut t, ro, running);
        for i in 0..3u32 {
            t.write(node(i), &[0x40 + i as u8; 0x1c]);
            t.set_u32(node(i), node((i + 2) % 3));
            t.set_u32(node(i) + 4, node((i + 1) % 3));
        }
        t.set_u32(fw(&t, 0x1cf), node(0));
        t.set_u16(fw(&t, 0x158), 3);
        t.set_u32(fw(&t, 0x236), 0);
        t.set_u32(fw(&t, 0x24f), 0);
        t.write(w(&t, 0x51), &[0x51; 0xc0]);
        t.write(n, &[0x4e; 0x20]);
        t.set_u32(n + 8, task);
        t.write(task, &[0x7a; 0x20]);
        t.check_entry(at, Call::default(), &[n], Eax::Checked);
    }
}
