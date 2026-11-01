//! `magic_0478.dll` against its Rust port (`ffx_magic::m0478`): the
//! blinking object's task and set-up. Needs `FFX_GAME_DIR`.

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

use ffx_magic::{Ret, m0478};
use magic::{Call, Eax, SCRATCH};
use magic_hi::{f, konst};
use magic_owna::{rec, recs, twin};

const N: u16 = 478;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10006700: every state (both ways where it branches) and every mode.
#[test]
fn task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6700);
    let k1 = konst(&t, at, 0x7);
    let k3 = konst(&t, at, 0xf);
    let k4 = konst(&t, at, 0x17);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 411, 1, move |m, _| m.r32(cell));
    recs(&mut t, &[(228, 2), (239, 3)]);
    t.set_u32(t.img.word(at + 0x205), 0x00a2_0000);
    let (task, r, o, node) = (
        SCRATCH + 0x100,
        SCRATCH + 0x200,
        SCRATCH + 0x400,
        SCRATCH + 0x500,
    );
    let (prev, next) = (SCRATCH + 0x600, SCRATCH + 0x640);
    // (state, blinks, counter, half, scale, speed, mode, 411's answer)
    let cases: &[(u32, u32, u32, u32, f32, f32, u32, u32)] = &[
        (0, 0, 0, 0, 0.0, 0.0, 0, 0),
        (0, 0, 0xffff_fff0, 3, 0.0, 0.0, 0, 0),
        (10, 5, 0xffff_fff0, 3, 2.0, 0.0, 3, 0),
        (0x15, 2, 0, 0, 1.0, k4, 0, 0),
        (1, 3, 0, 1, 0.5, 0.0, 1, 1),
        (1, 0, 0, 2, 0.5, 0.0, 1, 0),
        (1, 1, 0xffff_fffe, 0, 0.5, 0.0, 10, 0),
        (10, 5, 4, 3, 2.0, 0.0, 3, 0),
        (11, 0, 0, 1, 2.0, 0.0, 0, 0),
        (11, 0, 0, 1, k1 - 4.0 * k3, 0.0, 0, 0),
        (0x14, 0, 0, 0, 1.0, 0.0, 2, 0),
        (0x15, 0, 0, 0, 1.0, 0.5, 0, 0),
        (0x15, 2, 0, 0, 1.0, 0.0, 0, 0),
        (0x1e, 0, 0, 0, 0.0, 0.0, 0, 0),
        (0x1f, 0, 0, 0, k1 - k3, 0.0, 0, 0),
        (0x1f, 0, 0, 0, k1 - 2.0 * k3, 0.0, 1, 1),
        (0x2710, 0, 0, 0, 1.0, 0.0, 10, 0),
        (5, 0, 0, 0, 1.0, 0.0, 11, 0),
        (0x4e20, 0, 0, 0, 1.0, 0.0, 0xffff_ffff, 0),
    ];
    for &(state, blinks, counter, half, scale, speed, mode, answer) in cases {
        t.write(r, &[0x3c; 0xa0]);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x1c, mode);
        t.set_u32(r + 0x20, blinks);
        t.set_u32(r + 0x24, counter);
        t.set_u32(r + 0x28, half);
        t.set_u32(r + 0x2c, f(1.75));
        t.set_u32(r + 0x30, f(scale));
        t.set_u32(r + 0x34, f(-6.5));
        t.set_u32(r + 0x3c, f(speed));
        t.set_u32(r + 0x40, o);
        t.set_u32(o + 0x50, 0x0050_0000 + state);
        t.set_u32(cell, answer);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// 0x100069a0: the set-up with the effect's flag bit set and clear, the
/// lists empty and not.
#[test]
fn init() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x69a0);
    let (obj, e) = (SCRATCH + 0x1000, SCRATCH + 0x2000);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4 {
            m.w32(a[i], f(i as f32 * 2.5));
        }
        0x262
    });
    rec(&mut t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, |_, _| 0.375);
    rec(&mut t, 601, 2, move |m, a| {
        let v = m.r32(cell);
        m.w32(a[1], v);
        0x601
    });
    for (slot, k) in [
        (15u32, 1.5f64),
        (16, -2.0),
        (17, 0.25),
        (277, 3.0),
        (664, 0.625),
    ] {
        rec(&mut t, slot, 1, move |_, _| slot);
        t.stub_st0(slot, move |_, _| k);
    }
    rec(&mut t, 290, 2, |m, a| {
        for i in 0..3 {
            m.w32(a[1] + 4 * i, f(i as f32 - 0.5));
        }
        0x290
    });
    recs(&mut t, &[(211, 2)]);
    let w = |t: &magic::Twin, op: u32| t.img.word(at + op);
    let ring = SCRATCH + 0x3000;
    let last = SCRATCH + 0x3100;
    for (flag, full, a) in [(1u8, false, 0.5f32), (0, true, 0.25), (0, false, -1.0e9)] {
        t.write(obj, &[0x4e; 0x240]);
        t.write(e, &[0x2d; 0xa0]);
        t.set_u8(e + 0x90, flag | 0x40);
        t.set_u32(e + 0x50, 0x00e5_0000);
        t.set_u32(e + 0x54, 0x00e5_0004);
        t.set_u32(cell, f(a));
        t.write(ring, &[0x1b; 0x20]);
        t.set_u32(w(&t, 0x24), ring);
        let sentinel = w(&t, 0x75);
        t.write(last, &[0x1c; 0x20]);
        t.set_u32(w(&t, 0x65), if full { last } else { sentinel });
        t.set_u16(w(&t, 0x89), 3);
        // the task ring's flags, bit 0 set (cleared by the set-up)
        t.set_u32(w(&t, 0x3f), 0xff);
        let c = Call {
            ecx: obj,
            edx: e,
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0478::init(g, img, fr))
        });
    }
}
