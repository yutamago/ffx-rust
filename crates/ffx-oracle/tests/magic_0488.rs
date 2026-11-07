//! `magic_0488.dll` against its Rust port (`ffx_magic::m0488`). Needs
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

use ffx_magic::{Ret, m0488};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, recs, twin};

const N: u16 = 488;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001350: slot 712 saying 0 and 2.
#[test]
fn draw() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1350);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 712, 2, move |m, _| m.r32(cell));
    recs(&mut t, &[(258, 1), (231, 3)]);
    for answer in [0u32, 2] {
        t.set_u32(cell, answer);
        let c = Call {
            ecx: SCRATCH + 0x1000,
            edx: SCRATCH + 0x2000,
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0488::draw(g, img, fr.at, fr.ecx, fr.edx))
        });
    }
}

/// 0x100014e0: the start-up.
#[test]
fn start_up() {
    let Some(mut t) = twin(N) else { return };
    let d = SCRATCH + 0x400;
    for i in 0..8 {
        t.set_u32(d + 4 * i, 0x0d00_0000 + i);
    }
    rec(&mut t, 803, 2, |_, _| 0x2100_0000);
    rec(&mut t, 719, 3, move |_, _| d);
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    rec(&mut t, 260, 1, |_, a| 0x0260_0000 ^ a[0]);
    recs(&mut t, &[(718, 3), (222, 4), (711, 3), (710, 4)]);
    let at = t.at(0x14e0);
    let blk = t.img.word(at + 0x140);
    t.write(blk, &[0x5e; 0x110]);
    t.check_entry(at, Call::default(), &[0x00a6_0000], Eax::Checked);
}

/// 0x10001170: the state, the block and the records cleared.
#[test]
fn state_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1170);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    t.set_u32(t.img.word(at + 0x11), 0x0011_0001);
    t.write(t.img.word(at + 0x1c), &[0x5c; 0x30]);
    for op in [0x4c, 0x56, 0x60] {
        t.set_u32(t.img.word(at + op), 0x00ab_0000 + op);
    }
    let rc = t.img.rel32_target(at + 0x43);
    let recs_at = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(rc + 0x7), recs_at);
    t.write(recs_at, &vec![0x99; 0x94 * 1000]);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}

/// 0x10001410: parts 0 and 2, drawn and removed.
#[test]
fn draw_at_actor() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1410);
    let base = t.img.word(at + 0x21);
    let table = t.img.word(at + 0x2b);
    for i in 0..4 {
        t.set_u32(table + 4 * i, 0x00ac_0000 + i);
    }
    rec(&mut t, 210, 2, |m, a| {
        for i in 0..16 {
            m.w32(a[1] + 4 * i, 0x3f80_0000 + i);
        }
        0x210
    });
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, 0x4000_0000 + i);
        }
        0x235
    });
    let ret712 = SCRATCH + 0x40;
    rec(&mut t, 712, 2, move |m, _| m.r32(ret712));
    recs(&mut t, &[(231, 3), (258, 1)]);
    for (i, r) in [(0u32, 0u32), (2, 1)] {
        t.set_u32(ret712, r);
        let part = base + 0x80 * i;
        t.write(part, &[0x19; 0x80]);
        t.check_entry(at, Call::default(), &[part], Eax::Checked);
    }
}

/// 0x100011e0 (`r13_start::sparks_frame_0488`): the pool with three free
/// sprites (the rest of the 50 sparks find none), live records counted
/// with their colour words looked up, nothing to step or draw.
#[test]
fn sparks_frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x11e0);
    rec(&mut t, 210, 2, |m, a| {
        for i in 0..4u32 {
            m.w32(a[1] + 4 * i, (1.5 + i as f32 * 2.25).to_bits());
        }
        0x0210_0000 ^ a[0]
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
    recs(&mut t, &[(942, 1), (714, 5), (779, 2)]);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    let keys = t.img.word(at + 0x13);
    for i in 0..5 {
        t.set_u32(keys + 4 * i, 0x00e0_0000 + i);
    }
    let pool = t.img.word(at + 0x37);
    let sprites = SCRATCH + 0x1000;
    t.write(sprites, &vec![0x2a; 0x94 * 6]);
    for i in 0..6u32 {
        t.set_u32(sprites + 0x94 * i, u32::from(i % 2 == 1));
        t.set_u32(sprites + 0x94 * i + 0x28, 0x20 + i);
    }
    t.set_u32(pool, 6);
    t.set_u32(pool + 4, sprites);
    let table = t.img.word(at + 0x6c);
    for i in 0..0x40u32 {
        t.set_u32(table.wrapping_sub(4 * i), 0x00c0_1000 + i);
    }
    // the step and the draw run on the same pool (`ecx` the pool's
    // immediate): their slots
    for (s, k) in [
        (325u32, 2usize),
        (229, 3),
        (773, 4),
        (774, 5),
        (775, 2),
        (776, 1),
        (777, 1),
        (778, 6),
    ] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}
