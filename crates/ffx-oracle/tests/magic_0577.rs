//! `magic_0577.dll` against its Rust port (`ffx_magic::m0577`). Needs
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

use ffx_magic::{Ret, m0577};
use magic::{Call, Eax, SCRATCH, Twin};
use magic_hi::{f, konst, models};
use magic_owna::{rec, recs, twin};

const N: u16 = 577;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100090a0: halfwords small and large.
#[test]
fn colour() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x90a0);
    recs(&mut t, &[(325, 2)]);
    let node = SCRATCH + 0x1000;
    for (h, fl) in [
        ([0u16, 1, 0x80, 0xff], 0u32),
        ([0x7fff, 0xffff, 3, 0x1234], 6),
    ] {
        t.write(node, &[0x3d; 0x100]);
        for (i, v) in (0u32..).zip(h) {
            t.set_u16(node + 0xd0 + 2 * i, v);
        }
        t.set_u32(node + 0xe8, fl);
        let c = Call {
            ecx: node,
            edx: 0x00e5_0005,
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0577::colour(g, img, fr))
        });
    }
}

const SETS: [&[Option<&[(u32, u8, i16)]>]; 4] = [
    &[],
    &[None, Some(&[])],
    &[
        Some(&[(0x3580, 0x48, 3), (0x7580, 0x48, 3), (0x3580, 0x47, 2)]),
        None,
        Some(&[(0x3700, 0x48, 4), (0x3701, 0x48, 1), (0x3700, 0x48, 0)]),
    ],
    &[Some(&[(0xc000_3700, 0x48, 2), (0x3580, 0x48, -1)])],
];

/// 0x1000de80: no models, empty and missing ones, every kind and byte.
#[test]
fn flash_off() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0xde80);
    let g_at = t.img.word(at + 0x5);
    for set in SETS {
        models(&mut t, g_at, set);
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0577::flash_off(g, img, fr.at))
        });
    }
}

/// 0x1000e050: as [`flash_off`].
#[test]
fn glow_off() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0xe050);
    let g_at = t.img.word(at + 0x8);
    for set in SETS {
        models(&mut t, g_at, set);
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0577::glow_off(g, img, fr.at))
        });
    }
}

/// 0x100025f0: with and without a mode.
#[test]
fn move_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x25f0);
    let (rec, node) = (SCRATCH + 0x100, SCRATCH + 0x1000);
    for mode in [0u32, 1, 0x8000_0000] {
        t.write(node, &[0x00; 0x100]);
        for i in 0..0x18 {
            t.set_u32(node + 0x1c + 4 * i, f(1.5 * i as f32 - 7.25));
        }
        t.set_u32(node + 0xe4, 0x0f0f_0f0b);
        for (i, v) in (0u32..).zip([node, mode, 0x00ab_0000, 0x00cd_0000]) {
            t.set_u32(rec + 4 * i, v);
        }
        t.check(at, Call::this(rec), &[], Eax::Checked, |g, _, fr| {
            Ret::eax(m0577::move_start(g, fr.ecx))
        });
    }
}

/// 0x10001ed0: the node in a list (first, middle, last) and in none.
#[test]
fn draw_detached() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1ed0);
    recs(&mut t, &[(332, 1), (353, 2), (267, 3), (271, 4), (669, 5)]);
    let (holder, node, list, a, b) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x300,
        SCRATCH + 0x340,
    );
    for (inlist, next, prev) in [
        (false, a, b),
        (true, list, b),
        (true, a, list),
        (true, a, b),
    ] {
        t.write(node, &[0; 0x100]);
        t.write(list, &[0x2c; 0x20]);
        t.write(a, &[0x2a; 0x80]);
        for i in 0..4 {
            t.set_u32(node + 0x7c + 4 * i, f(-2.75 + 1.6 * i as f32));
        }
        t.set_u32(holder, node);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.set_u32(node + 0x14, if inlist { list } else { 0 });
        t.check(at, Call::this(holder), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0577::draw_detached(g, img, fr))
        });
    }
}

/// 0x10002480: bit 3 clear and set, with and without a mode.
#[test]
fn place() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2480);
    recs(&mut t, &[(325, 2), (248, 3), (234, 3)]);
    let (h, node) = (SCRATCH + 0x100, SCRATCH + 0x1000);
    for (fl, mode) in [(0u32, 0u32), (0x0f0f_0ff7, 1), (8, 0), (0xffff_ffff, 2)] {
        t.write(node, &[0; 0x100]);
        for i in 0..4u32 {
            t.set_u32(node + 0xbc + 4 * i, (i * 0x1234_5679).wrapping_sub(0x10));
            t.set_u32(node + 0x1c + 4 * i, f(0.25 + 1.5 * i as f32));
        }
        t.set_u32(node + 0xe4, fl);
        for (i, v) in (0u32..).zip([node, 0x00ce_0000, 0x00cf_0000, mode]) {
            t.set_u32(h + 4 * i, v);
        }
        t.check(at, Call::this(h), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0577::place(g, img, fr))
        });
    }
}

/// 0x1000df10: no models, both kinds with both strides, other kinds.
#[test]
fn halve() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0xdf10);
    let g_at = t.img.word(at + 0x7);
    let sets: [&[Option<&[(u32, u8, i16)]>]; 4] = [
        &[],
        &[None, Some(&[])],
        &[
            Some(&[(0x34c0, 0x48, 3), (0x7cc0, 0x48, 3), (0x3cc0, 0x47, 2)]),
            None,
            Some(&[(0x34c0, 0x07, 4), (0x34c1, 0x48, 1), (0x3cc0, 0x08, 0)]),
        ],
        &[Some(&[(0xc000_3cc0, 0x48, 2), (0x34c0, 0x48, -1)])],
    ];
    for set in sets {
        models(&mut t, g_at, set);
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0577::halve(g, img, fr.at))
        });
    }
}

/// 0x1000dbb0: no current nodes; nodes that are their neighbours' first
/// and last; others; the list object's model with flags 1 and 0x811.
#[test]
fn end() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0xdbb0);
    recs(
        &mut t,
        &[
            (250, 2),
            (24, 4),
            (25, 6),
            (538, 3),
            (213, 3),
            (211, 3),
            (35, 3),
            (34, 2),
            (547, 2),
            (276, 3),
            (219, 3),
        ],
    );
    let un = t.img.rel32_target(at + 0x73);
    let w = |t: &Twin, f: u32, op: u32| t.img.word(f + op);
    let obj = t.img.word(at + 0x69);
    let (n, a, b) = (SCRATCH + 0x1000, SCRATCH + 0x2000, SCRATCH + 0x3000);
    for case in 0..4 {
        t.write(n, &[0x31; 0x20]);
        t.write(a, &[0x32; 0x20]);
        t.write(b, &[0x33; 0x20]);
        t.set_u32(obj + 0x90, if case == 3 { 0x811 } else { 1 });
        t.set_u32(obj + 0x64, u32::MAX);
        for f in [at + 4, un] {
            t.set_u32(w(&t, f, 0x2), if case == 0 { 0 } else { n });
            let (next, prev) = if case == 1 { (n, n) } else { (a, b) };
            t.set_u32(w(&t, f, 0x15), next);
            t.set_u32(w(&t, f, 0x1a), prev);
        }
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10008c10: no targets, none accepted, some accepted.
#[test]
fn centre() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x8c10);
    let cell = SCRATCH + 0x10;
    let base = SCRATCH + 0x400;
    for i in 0..4 {
        t.set_u32(base + 4 * i, f(10.5 - 3.25 * i as f32));
    }
    rec(&mut t, 325, 2, move |m, a| {
        for i in 0..4 {
            let v = m.r32(base + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x0032_5000
    });
    rec(&mut t, 684, 1, move |m, a| m.r32(cell) >> (a[0] & 7) & 1);
    rec(&mut t, 531, 2, move |m, a| {
        for i in 0..3 {
            m.w32(a[1] + 4 * i, f(a[0] as f32 * 1.75 - i as f32));
        }
        0x0053_1000 + a[0]
    });
    let n_at = t.img.word(at + 0x48);
    let table = t.img.word(at + 0x55);
    for (i, b) in (0u32..).zip([3u8, 5, 0, 7, 1, 6]) {
        t.set_u8(table + i, b);
    }
    let out = SCRATCH + 0x1000;
    for (n, mask) in [(0i32, 0xffu32), (-1, 0xff), (4, 0), (6, 0xaa), (6, 0xff)] {
        t.write(out, &[0x55; 0x20]);
        t.set_u32(n_at, n.cast_unsigned());
        t.set_u32(cell, mask);
        let c = Call {
            ecx: out,
            edx: SCRATCH + 0x500,
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0577::centre(g, img, fr))
        });
    }
}

/// 0x1000a6c0: every state, the timer above and at 0, the size above
/// and at its limit, slot 313 saying 1 and 0, the bits clear and set.
#[test]
fn task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0xa6c0);
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 313, 1, move |m, _| m.r32(cell));
    recs(&mut t, &[(714, 5), (325, 2), (222, 4)]);
    let o = SCRATCH + 0x300;
    t.set_u32(t.img.word(at + 0x161), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    let (k, l) = (konst(&t, at, 0x95), konst(&t, at, 0x9d));
    let (task, r, node, prev, next) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x140,
        SCRATCH + 0x180,
        SCRATCH + 0x1c0,
    );
    let parent = t.img.word(at + 0x11c);
    let l2 = t.img.word(at + 0x17d);
    t.write(l2, &[0; 0x20]);
    t.set_u32(l2 + 0xc, l2);
    t.set_u32(l2 + 0x10, l2);
    for (state, timer, size, answer, bits) in [
        (0u32, 5i32, 0.0f32, 0u32, 0u32),
        (1, 1, 0.0, 0, 0),
        (1, 0, 0.0, 0, 0),
        (1, -3, 0.0, 0, 0),
        (2, 3, l / k + 1.0, 0, 0),
        (2, 3, l / k, 0, 0),
        (2, 3, f32::NAN, 0, 0),
        (5, 3, 0.0, 0, 0),
        (10, 3, 0.0, 1, 0),
        (10, 3, 0.0, 0, 0),
        (11, 3, 0.0, 0, 0),
        (12, 3, 0.0, 0, 0),
        (12, 3, 0.0, 0, 4),
        (0x2710, 3, 0.0, 0, 3),
        (0x2710, 3, 0.0, 0, 0xff),
        (0x4e20, 3, 0.0, 0, 0),
    ] {
        t.write(r, &[0x3c; 0x600]);
        t.write(parent, &[0; 0x20]);
        t.set_u32(parent + 0x10, parent);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.set_u32(cell, answer);
        t.set_u32(r + 0x18, timer.cast_unsigned());
        t.set_u32(r + 0x1c, state);
        t.set_u32(r + 0x48, f(size));
        t.set_u32(r + 0x4f8, bits | 0x10);
        t.set_u32(r + 0x3f8, 0xffff_fff3 | bits);
        t.set_u32(r + 0x1a0, 0x00a1_0000);
        t.set_u32(r + 0x1b4, 0x00a2_0000);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}
