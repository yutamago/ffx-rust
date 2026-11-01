//! `magic_0573.dll` against its Rust port (`ffx_magic::m0573`). Needs
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

use ffx_magic::{Ret, m0573};
use magic::{Call, Eax, SCRATCH, Twin};
use magic_hi::{f, konst};
use magic_owna::{rec, recs, twin};

const N: u16 = 573;

/// The trap slot the node's children's vtable method answers through.
const METHOD: u32 = 900;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100035f0: the bit set, the parts missing and present.
#[test]
fn show() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x35f0);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 219, 2, move |m, a| m.r32(cell) * (a[1] + 1));
    recs(&mut t, &[(1, 1)]);
    let obj = SCRATCH + 0x1000;
    for (bits, parts) in [(1u8, 0x40u32), (0, 0), (0, 0x40)] {
        t.write(obj, &[0x22; 0x100]);
        t.set_u8(obj + 0x90, bits);
        t.set_u32(cell, parts);
        t.check(at, Call::this(obj), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0573::show(g, img, fr.at, fr.ecx))
        });
    }
}

/// 0x100070d0: every state, the size below and at its limit.
#[test]
fn task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x70d0);
    let (k, limit) = (konst(&t, at, 0x5f), konst(&t, at, 0x66));
    let (task, r, node, prev, next) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x240,
        SCRATCH + 0x280,
    );
    for (state, size, bits) in [
        (0u32, 0.0f32, 0u8),
        (1, limit - k - 1.0, 0),
        (1, limit - k, 0),
        (0x2710, 0.0, 0),
        (0x2710, 0.0, 4),
        (9, 0.0, 0),
    ] {
        t.write(r, &[0; 0x300]);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x20, f(2.5));
        t.set_u32(r + 0x24, f(size));
        t.set_u8(r + 0x288, bits);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// The node list's last node: the list itself (empty) or `last`.
fn list(t: &mut Twin, at: u32, (last_op, sentinel_op): (u32, u32), full: bool, last: u32) {
    let sentinel = t.img.word(at + sentinel_op);
    t.write(last, &[0x1c; 0x20]);
    t.set_u32(t.img.word(at + last_op), if full { last } else { sentinel });
}

/// 0x10007e00 and 0x10007170: the node list empty and not.
#[test]
fn inits() {
    let Some(mut t) = twin(N) else { return };
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5), (325, 2)]);
    let i7170 = t.at(0x7170);
    let o = SCRATCH + 0x300;
    t.set_u32(t.img.word(i7170 + 0xd8), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    let ring = SCRATCH + 0x3000;
    let (obj, node, last) = (SCRATCH + 0x1000, SCRATCH + 0x2000, SCRATCH + 0x3100);
    for full in [false, true] {
        let at = t.at(0x7e00);
        list(&mut t, at, (0x24, 0x31), full, last);
        t.write(node, &[0x4e; 0x40]);
        t.check(
            at,
            Call::this(node),
            &[0x00a0_0000],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0573::add_node(g, img, fr)),
        );
        list(&mut t, i7170, (0x62, 0x6f), full, last);
        let l2 = t.img.word(i7170 + 0xf1);
        t.write(l2, &[0; 0x20]);
        t.set_u32(l2 + 0xc, l2);
        t.set_u32(l2 + 0x10, l2);
        t.write(ring, &[0x1b; 0x20]);
        t.set_u32(t.img.word(i7170 + 0x21), ring);
        t.write(obj, &[0x4e; 0x300]);
        let c = Call {
            ecx: obj,
            edx: 0x00e5_0000,
            ..Call::default()
        };
        t.check(i7170, c, &[0x00a0_0001], Eax::Checked, |g, img, fr| {
            Ret::eax(m0573::init(g, img, fr))
        });
    }
}

/// 0x10002d60: the node refresh: the mode the node's already, a node
/// with no children and with two (their method a trap), the model flag
/// clear and set, the model and node with a parent and without.
#[test]
fn refresh() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2d60);
    rec(&mut t, METHOD, 1, |m, a| {
        let w1 = m.r32(a[0] + 4);
        m.w32(SCRATCH + 0x2_0004, w1 & 0xff00);
        0x900
    });
    let vt = SCRATCH + 0x600;
    t.set_u32(vt + 0x1c, magic::TRAPS + 16 * METHOD);
    recs(
        &mut t,
        &[
            (275, 1),
            (538, 2),
            (213, 2),
            (211, 2),
            (35, 5),
            (34, 2),
            (547, 1),
            (276, 2),
            (219, 2),
        ],
    );
    let cur = SCRATCH + 0x100;
    let (node, model, parent) = (SCRATCH + 0x1000, SCRATCH + 0x1400, SCRATCH + 0x1800);
    let kids = [SCRATCH + 0x1600, SCRATCH + 0x1680];
    for (same, n, bit, with_parent) in [
        (true, 0usize, 1u8, true),
        (false, 0, 0, false),
        (false, 2, 1, true),
        (false, 2, 1, false),
        (false, 0, 1, true),
    ] {
        t.write(node, &[0x2c; 0x100]);
        t.write(model, &[0x3d; 0x100]);
        t.write(parent, &[0x4e; 0x40]);
        t.set_u8(node + 0x1a, if same { 0x37 } else { 0x15 });
        let mut c = node;
        for k in kids.iter().take(n).rev() {
            t.set_u32(*k, vt);
            t.set_u32(k + 8, c);
            c = *k;
        }
        t.set_u32(node + 0x10, c);
        t.set_u8(node + 0x24, bit | 0x10);
        t.set_u32(node + 0x1c, model);
        t.set_u32(model + 0x90, 1);
        let p = if with_parent { parent } else { 0 };
        for x in [node, model] {
            t.set_u32(x + 0x14, p);
            t.set_u32(x + 4, parent);
            t.set_u32(x + 8, parent);
        }
        t.set_u32(cur, node);
        t.set_u32(cur + 4, 0x3700_5a5a);
        t.check(at, Call::this(cur), &[], Eax::Checked, |g, img, fr| {
            m0573::refresh(g, img, fr)
        });
    }
}

/// 0x100072a0: the summon's motion task through its states (the motion
/// done or not, the wait out or not), the count below and past 40 with
/// the size above, at and below its floor.
#[test]
fn motion_task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x72a0);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 411, 1, move |m, _| m.r32(cell) & 1);
    rec(&mut t, 228, 2, |_, a| 0x0228_0000 ^ a[1]);
    recs(&mut t, &[(213, 2), (211, 2), (255, 2)]);
    let tab = t.img.word(at + 0x6c);
    for i in 0..20u32 {
        t.set_u32(tab + 4 * i, 0x00b0 + i);
    }
    let (task, r, node, prev, next) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x240,
        SCRATCH + 0x280,
    );
    let o = SCRATCH + 0x1400;
    let mut cases = Vec::new();
    for s in [0u32, 1, 2, 5, 10, 11, 12, 13, 0x2710, 0x2711, 0x2712] {
        for (bits, wait, n) in [(0u32, 5u32, 3u32), (1, 0, 0x28), (1, 0xffff_fff0, 0x50)] {
            cases.push((s, bits, wait, n));
        }
    }
    for (k, (state, bits, wait, n)) in cases.into_iter().enumerate() {
        t.write(r, &[0x3c; 0x200]);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x1c, wait);
        t.set_u32(r + 0x20, n);
        t.set_u32(r + 0x24, k as u32 % 3);
        t.set_u32(r + 0x2c, o);
        t.set_u32(r + 0x28, 2.0f32.to_bits());
        t.set_u32(r + 0x19c, [9.0f32, 2.25, 1.0][k % 3].to_bits());
        t.write(o, &[0x4d; 0x100]);
        t.set_u32(o + 0x50, 0x00c0_0050);
        t.set_u8(o + 0x90, 0x10 | (k as u8 & 1));
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.set_u32(cell, bits);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}
