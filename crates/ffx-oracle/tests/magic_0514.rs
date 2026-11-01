//! `magic_0514.dll` against its Rust port (`ffx_magic::m0514`). Needs
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

use ffx_magic::{Ret, m0514};
use magic::{Call, Eax, SCRATCH};
use magic_hi::f;
use magic_owna::{rec, recs, twin};

const N: u16 = 514;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10007d70 and 0x10006e30: the two set-ups, the node list empty and
/// not.
#[test]
fn inits() {
    let Some(mut t) = twin(N) else { return };
    let (obj, owner, model) = (SCRATCH + 0x1000, SCRATCH + 0x2000, SCRATCH + 0x2400);
    let ring = SCRATCH + 0x3000;
    let last = SCRATCH + 0x3100;
    t.set_u32(owner + 0x140, model);
    t.set_u32(model + 0x28, f(2.5));
    let at = t.at(0x7d70);
    t.set_u32(t.img.word(at + 0x1a), ring);
    t.set_u32(t.img.word(at + 0x32), 0xff);
    t.write(obj, &[0x4e; 0x200]);
    let c = Call {
        ecx: obj,
        edx: 0x00ed_0000,
        ..Call::default()
    };
    t.check(at, c, &[0xa, 0xb, 0xc], Eax::Checked, |g, img, fr| {
        Ret::eax(m0514::timed_init(g, img, fr))
    });
    let at = t.at(0x6e30);
    for full in [false, true] {
        t.write(obj, &[0x4e; 0x200]);
        t.write(ring, &[0x1b; 0x20]);
        t.set_u32(t.img.word(at + 0x1e), ring);
        let sentinel = t.img.word(at + 0x71);
        t.write(last, &[0x1c; 0x20]);
        t.set_u32(t.img.word(at + 0x64), if full { last } else { sentinel });
        t.set_u16(t.img.word(at + 0x85), 3);
        t.check(at, c, &[owner], Eax::Checked, |g, img, fr| {
            Ret::eax(m0514::scaled_init(g, img, fr))
        });
    }
}

/// 0x10007260 and 0x10007660: the tasks' states (the nodes without
/// parents).
#[test]
fn tasks() {
    let Some(mut t) = twin(N) else { return };
    let (task, r, node, prev, next) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x240,
        SCRATCH + 0x280,
    );
    for (rva, state, wait, byte) in [
        (0x7260u32, 0u32, 0u32, 0u8),
        (0x7260, 1, 1, 0),
        (0x7260, 1, 0, 0),
        (0x7260, 1, 0, 4),
        (0x7260, 0x2710, 0, 4),
        (0x7260, 9, 0, 0),
        (0x7660, 0, 0, 0x40),
        (0x7660, 1, 0, 0x10),
        (0x7660, 0x2710, 0, 0),
        (0x7660, 0x4e20, 0, 0),
    ] {
        t.write(r, &[0; 0x200]);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x1c, wait);
        t.set_u8(r + 0x2b8, byte);
        t.set_u8(r + 0x1eb, byte);
        t.check_entry(t.at(rva), Call::default(), &[task], Eax::Checked);
    }
}

/// The task's record and node for the task steps.
fn task_rec(t: &mut magic::Twin, state: u32, wait: u32) -> u32 {
    let (task, r, node, prev, next) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x240,
        SCRATCH + 0x280,
    );
    t.write(r, &[0x3c; 0x100]);
    t.set_u32(task, node);
    t.set_u32(task + 4, r);
    t.set_u32(node + 4, next);
    t.set_u32(node + 8, prev);
    t.set_u32(r + 0x18, state);
    t.set_u32(r + 0x1c, wait);
    task
}

/// 0x10006b40: the motion task through its states (the motion done or
/// not, the wait out or not) and states it ignores.
#[test]
fn motion_task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6b40);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 411, 1, move |m, _| m.r32(cell) & 1);
    rec(&mut t, 228, 2, |_, a| 0x0228_0000 ^ a[1]);
    recs(&mut t, &[(213, 2)]);
    let o = SCRATCH + 0x1400;
    for state in [0u32, 1, 4, 10, 11, 0x13, 0x14, 0x15, 0x16, 0x2710] {
        for (bits, wait) in [(0u32, 5u32), (1, 0), (1, 0xffff_fff0)] {
            let task = task_rec(&mut t, state, wait);
            let r = SCRATCH + 0x1000;
            t.set_u32(r + 0x20, o);
            t.write(o, &[0x4d; 0x60]);
            t.set_u32(o + 0x50, 0x00c0_0050);
            t.set_u32(cell, bits);
            t.check_entry(at, Call::default(), &[task], Eax::Checked);
        }
    }
}

/// 0x10006c90: the fade task through its states, the level bytes at and
/// off their limits, the wait and the count out or not.
#[test]
fn fade_task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6c90);
    let r = SCRATCH + 0x1000;
    for state in [
        0u32, 1, 5, 10, 11, 12, 13, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x2710,
    ] {
        for (wait, a8, a9, n) in [
            (5u32, 0x62u8, 0x20u8, 3u32),
            (0, 0x7e, 0x00, 1),
            (0xffff_fff0, 0x50, 0x40, 2),
            (0, 0x62, 0x20, 1),
        ] {
            let task = task_rec(&mut t, state, wait);
            t.set_u8(r + 0xa8, a8);
            t.set_u8(r + 0xa9, a9);
            t.set_u32(r + 0x20, n);
            t.check_entry(at, Call::default(), &[task], Eax::Checked);
        }
    }
}

/// 0x10006f40: the colour task: state 20, state 21 with the flag bit set
/// and clear (the node's bit 2 set or not, with a parent or none), and
/// other states.
#[test]
fn colour_task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6f40);
    let ftu = t.img.rel32_target(at + 0x10c);
    recs(&mut t, &[(325, 2), (713, 1)]);
    let (p1, p2) = (SCRATCH + 0x20, SCRATCH + 0x30);
    let g1 = t.img.word(ftu + 0x01);
    t.set_u32(g1, p1);
    let g2 = t.img.word(at + 0x11a);
    t.set_u32(g2, p2);
    let r = SCRATCH + 0x1000;
    let parent = SCRATCH + 0x1800;
    for (state, flags, e8, with_parent) in [
        (0x14u32, 0u32, 0u8, false),
        (0x15, 1, 0, false),
        (0x15, 0, 0, false),
        (0x15, 0, 4, true),
        (0x15, 2, 0x0b, true),
        (3, 0, 0, false),
        (0x2710, 0, 0, false),
    ] {
        let task = task_rec(&mut t, state, 9);
        t.write(r + 0x100, &[0x5e; 0x200]);
        for i in 0..4u32 {
            t.set_u16(r + 0x2a0 + 2 * i, (0x30 + 0x111 * i) as u16);
        }
        t.set_u32(r + 0x2b8, flags | 0x5a00);
        let node = r + 0x1d0;
        t.set_u8(node + 0xe8, e8);
        t.set_u32(node + 0x50, 0x00c0_0050);
        t.write(parent, &[0x6f; 0x40]);
        t.set_u32(node + 0x14, if with_parent { parent } else { 0 });
        t.set_u32(node + 4, parent);
        t.set_u32(node + 8, parent);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// 0x100070b0: the colour object's set-up, the node list empty and not.
#[test]
fn colour_setup() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x70b0);
    let list = t.img.word(at + 0x5c);
    let ring_last = t.img.word(at + 0x24);
    let ring_flags = t.img.word(at + 0x42);
    let table = SCRATCH + 0x300;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x400;
    t.set_u32(t.img.word(at + 0x161), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(325, 2), (714, 5)]);
    let list2 = t.img.word(at + 0x17a);
    t.set_u32(list2 + 0x10, list2);
    let obj = SCRATCH + 0x1000;
    let (old, prev) = (SCRATCH + 0x800, SCRATCH + 0x900);
    for (k, empty) in [(0u32, true), (1, false)] {
        t.write(obj, &[0x5b; 0x300]);
        t.write(old, &[0x6c; 0x40]);
        t.write(prev, &[0x7d; 0x40]);
        t.write(list, &[0x4e; 0x20]);
        t.set_u32(list + 0x10, if empty { list } else { prev });
        t.set_u32(list + 0xc, 0x00f1_0000);
        t.set_u16(list + 0x18, 0x0305);
        t.set_u32(ring_last, old);
        t.set_u32(ring_flags, 0x8000_0007);
        let c = Call {
            ecx: obj,
            edx: 0x00ed_0000 + k,
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0514::colour_setup(g, img, fr))
        });
    }
}

/// 0x100076f0: the glow object's set-up, both node lists empty and not.
#[test]
fn glow_setup() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x76f0);
    let list = t.img.word(at + 0x5c);
    let list2 = t.img.word(at + 0x18c);
    let ring_last = t.img.word(at + 0x22);
    let ring_flags = t.img.word(at + 0x3c);
    recs(&mut t, &[(719, 3)]);
    let obj = SCRATCH + 0x1000;
    let src = SCRATCH + 0x1400;
    let (old, prev, prev2) = (SCRATCH + 0x800, SCRATCH + 0x900, SCRATCH + 0xa00);
    for (k, empty) in [(0u32, true), (1, false)] {
        t.write(obj, &[0x5b; 0x300]);
        t.write(src, &[0x2a; 0x40]);
        t.set_u32(src + 0x1c, 0x3fc0_001c);
        t.set_u32(src + 0x24, 0x3fc0_0024);
        t.write(old, &[0x6c; 0x40]);
        t.write(prev, &[0x7d; 0x40]);
        t.write(prev2, &[0x7e; 0x40]);
        for (l, p) in [(list, prev), (list2, prev2)] {
            t.write(l, &[0x4e; 0x20]);
            t.set_u32(l + 0x10, if empty { l } else { p });
            t.set_u32(l + 0xc, 0x00f1_0000);
            t.set_u16(l + 0x18, 0x0305);
        }
        t.set_u32(ring_last, old);
        t.set_u32(ring_flags, 0x8000_0007);
        let c = Call {
            ecx: obj,
            edx: 0x00ed_0000 + k,
            ..Call::default()
        };
        t.check(at, c, &[src], Eax::Checked, |g, img, fr| {
            Ret::eax(m0514::glow_setup(g, img, fr))
        });
    }
}

/// 0x10007a90: the burst object's set-up, the node list empty and not.
#[test]
fn burst_setup() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7a90);
    let list = t.img.word(at + 0x5f);
    let ring_last = t.img.word(at + 0x27);
    let ring_flags = t.img.word(at + 0x45);
    let table = SCRATCH + 0x300;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x400;
    t.set_u32(t.img.word(at + 0x16b), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(325, 2), (714, 5)]);
    let list2 = t.img.word(at + 0x187);
    t.set_u32(list2 + 0x10, list2);
    let obj = SCRATCH + 0x4000;
    let (old, prev) = (SCRATCH + 0x800, SCRATCH + 0x900);
    for (k, empty) in [(0u32, true), (1, false)] {
        t.write(obj, &[0x5b; 0x1800]);
        t.write(old, &[0x6c; 0x40]);
        t.write(prev, &[0x7d; 0x40]);
        t.write(list, &[0x4e; 0x20]);
        t.set_u32(list + 0x10, if empty { list } else { prev });
        t.set_u32(list + 0xc, 0x00f1_0000);
        t.set_u16(list + 0x18, 0x0305);
        t.set_u32(ring_last, old);
        t.set_u32(ring_flags, 0x8000_0007);
        let c = Call {
            ecx: obj,
            edx: 0x00ed_0000 + k,
            ..Call::default()
        };
        t.check(
            at,
            c,
            &[0x00a0_0000 + k, 0x00b0_0000, 0x00c0_0000],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0514::burst_setup(g, img, fr)),
        );
    }
}

/// 0x100072f0: the spark object's set-up: the size below and above its
/// bound, both random picks, the node list empty and not; the DLL's
/// byte and globals distinct.
#[test]
fn spark_setup() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x72f0);
    let list = t.img.word(at + 0x95);
    let ring_last = t.img.word(at + 0x5d);
    let ring_flags = t.img.word(at + 0x7b);
    let table = SCRATCH + 0x300;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x400;
    t.set_u32(t.img.word(at + 0x295), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    recs(
        &mut t,
        &[
            (325, 2),
            (714, 5),
            (268, 3),
            (388, 3),
            (239, 3),
            (719, 3),
            (306, 3),
        ],
    );
    let cell = SCRATCH + 0x10;
    rec(&mut t, 251, 1, |_, _| 0x251);
    t.stub_st0(251, move |m, _| f64::from(f32::from_bits(m.r32(cell))));
    rec(&mut t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, move |m, _| {
        let k = m.r32(cell + 4);
        m.w32(cell + 4, k + 1);
        0.125 + f64::from(k) * 0.3125
    });
    rec(&mut t, 324, 0, move |m, _| m.r32(cell + 8));
    let list2 = t.img.word(at + 0x2ae);
    t.set_u32(list2 + 0x10, list2);
    let b = t.img.word(at + 0x9);
    t.set_u32(b, 0x5a5a_5aa7);
    let ga = t.img.word(at + 0x30d);
    t.set_u32(ga, 0x00a0_030d);
    let obj = SCRATCH + 0x1000;
    let a = SCRATCH + 0x1800;
    let (old, prev) = (SCRATCH + 0x800, SCRATCH + 0x900);
    for (size, pick, empty) in [(0.5f32, 0u32, true), (1000.0, 1, false), (-3.0, 2, true)] {
        t.write(obj, &[0x5b; 0x300]);
        for i in 0..4u32 {
            t.set_u32(obj + 0x18c + 4 * i, (1.5 + i as f32).to_bits());
        }
        t.write(a, &[0x2a; 0x60]);
        t.set_u32(a + 0x54, 0x00c0_0054);
        t.set_u32(cell, size.to_bits());
        t.set_u32(cell + 4, pick);
        t.set_u32(cell + 8, pick);
        t.write(old, &[0x6c; 0x40]);
        t.write(prev, &[0x7d; 0x40]);
        t.write(list, &[0x4e; 0x20]);
        t.set_u32(list + 0x10, if empty { list } else { prev });
        t.set_u32(list + 0xc, 0x00f1_0000);
        t.set_u16(list + 0x18, 0x0305);
        t.set_u32(ring_last, old);
        t.set_u32(ring_flags, 0x8000_0007);
        let c = Call {
            ecx: obj,
            edx: 0x00ed_0000 + pick,
            ..Call::default()
        };
        t.check(at, c, &[a, 0x00b0_0000], Eax::Checked, |g, img, fr| {
            Ret::eax(m0514::spark_setup(g, img, fr))
        });
    }
}
