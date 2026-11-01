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
