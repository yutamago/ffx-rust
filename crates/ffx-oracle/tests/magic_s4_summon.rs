//! The shared-4 list's summon handlers (`ffx_magic::s4_summon`) and their
//! thunks (`ffx_magic::s4_wrap`) against `magic_0003.dll` and
//! `magic_0087.dll`. Needs `FFX_GAME_DIR`.

#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

mod magic;
#[path = "magic/sa.rs"]
mod sa;

use ffx_magic::s4_summon;
use magic::{Eax, Twin};
use sa::{RINGS, VTABLE_2, check_this_thunk, handler_stubs, node, rec, ring};

/// A /GS handler's `eax` is the cookie's junk on the first ring (no move).
fn gs_eax(k: usize) -> Eax {
    if k == 0 { Eax::Junk } else { Eax::Checked }
}

fn twin(n: u16) -> Option<Twin> {
    let mut t = sa::twin(n)?;
    handler_stubs(&mut t);
    Some(t)
}

fn set_f(t: &mut Twin, a: u32, x: f32) {
    t.set_u32(a, x.to_bits());
}

/// Object `i` hung at the node's `off`: its own ring (one child, or
/// none), its pass byte equal to the cursor's or not.
fn object(t: &mut Twin, off: u32, i: u32, child: bool, pass: u8) {
    let o = node(i);
    t.write(o, &[0x50 + i as u8; 0x200]);
    t.set_u32(o, VTABLE_2);
    t.set_u8(o + 0x1a, pass);
    if child {
        let c = node(i + 8);
        t.write(c, &[0x70 + i as u8; 0x40]);
        t.set_u32(c, VTABLE_2);
        t.set_u32(c + 8, o);
        t.set_u32(o + 0x10, c);
    } else {
        t.set_u32(o + 0x10, o);
    }
    for j in 0..0x40 {
        let v = (j as f32 + 1.0) * 0.25 * (i as f32);
        t.set_u32(o + 0x1c + 4 * j, v.to_bits());
    }
    t.set_u32(node(0) + off, o);
}

fn elem_bone_cases(t: &mut Twin, f: u32, thunk: u32) {
    rec(t, 236, 3, |m, a| {
        for i in 0..16 {
            let v = (i as f32 - 7.5) * 0.125 + (a[0] & 0xf) as f32;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x236
    });
    rec(t, 233, 3, |m, a| {
        m.w32(a[0], a[2] ^ 0x8000_0000);
        m.w32(a[0] + 4, a[2].rotate_left(3));
        m.w32(a[0] + 8, a[1]);
        0x233
    });
    rec(t, 325, 2, |_, a| 0x0325_0000 ^ a[1]);
    rec(t, 277, 1, |_, _| 0x277);
    t.stub_st0(277, |_, a| f64::from(a[0] & 0xff) * 0.37 + 0.1);
    rec(t, 232, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v ^ 0x0040_0000);
        }
        0x232
    });
    rec(t, 231, 3, |m, a| {
        // the unit matrix's last row and the matrix it is given
        m.w32(
            sa::node(30) + (a[0] & 0xfc),
            m.r32(a[1] + 0x30) ^ m.r32(a[1] + 0x3c),
        );
        0x2310_0000 ^ a[2]
    });
    rec(t, 229, 3, |m, a| m.r32(a[1] + 4));
    for (k, &(c, p, cp)) in RINGS.iter().enumerate() {
        for (case, (child, opass)) in [(false, 0u8), (true, 1), (true, 0x30), (false, 9)]
            .into_iter()
            .enumerate()
        {
            ring(t, c, p, cp, 0x30 + k as u8);
            for j in 0..0x40 {
                let v = 1.0f32 / (j as f32 + 3.0) - case as f32;
                t.set_u32(node(0) + 0x5c + 4 * j, v.to_bits());
            }
            // the objects' passes: equal to the cursor's on the last case
            let op = if case == 3 { cp } else { opass };
            object(t, 0x13c, 5, child, op);
            object(t, 0x144, 6, !child, op);
            object(t, 0x140, 7, child, op.wrapping_add(1));
            t.set_u32(node(0) + 0x14c, 0x14c0 + case as u32);
            t.set_u32(node(0) + 0x150, 0x1500 + k as u32);
            set_f(t, node(0) + 0x138, 1.5 - case as f32);
            check_this_thunk(t, f, thunk, s4_summon::elem_bone, gs_eax(k));
        }
    }
}

#[test]
fn elem_bone_places() {
    for (n, f, thunk) in [(3u16, 0x1f60u32, 0x2360u32), (87, 0x21a0, 0x25a0)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(f);
        elem_bone_cases(&mut t, f, thunk);
    }
}

/// The DLL's saved ratios [`ffx_magic::sb_nodes::scale_ratio_801`] reads,
/// the stubs of it and of `leave_and_reset`'s slots.
fn fade_stubs(t: &mut Twin, f: u32) {
    let ratio = t.img.rel32_target(f + 0x341);
    for (o, x) in [(0x94u32, 1.0f32), (0x9c, 2.0), (0xb1, 4.0), (0xb9, 0.5)] {
        let a = t.img.word(ratio + o);
        set_f(t, a, x);
    }
    let leave = t.img.rel32_target(f + 0x3e1);
    let k = t.img.word(leave + 0xfc);
    set_f(t, k, 0.75);
    for (s, n) in [
        (353u32, 1usize),
        (332, 1),
        (669, 1),
        (801, 6),
        (267, 1),
        (271, 1),
        (785, 1),
    ] {
        rec(t, s, n, move |m, a| {
            // the vectors and integers behind the first argument
            let row = sa::node(31) + 0x20 * (s % 8);
            for i in 0..4 {
                m.w32(row + 4 * i, m.r32(a[0].wrapping_add(4 * i)));
            }
            0x0100_0000 + s
        });
    }
}

#[test]
fn colour_node_fades() {
    for (n, f, thunk) in [(3u16, 0x1470u32, 0x19c0u32), (87, 0x1620, 0x1c00)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(f);
        fade_stubs(&mut t, f);
        let k = f32::from_bits(t.u32(t.img.word(f + 0x157)));
        for (kr, &(c, p, cp)) in RINGS.iter().enumerate() {
            for flags in 0u32..8 {
                for (rate, progress, end, limit) in [
                    (0.25f32, 0.5f32, 0.6f32, 0.4f32),
                    (0.5, k - 0.25, 0.1, 200.0),
                    (0.75, k, 2.0, -1.0),
                    (0.125, -3.0, -4.0, 1.25),
                    (1.0e-3, k + 1.0, f32::NAN, 0.75),
                    (f32::NAN, 0.5, 0.25, f32::NAN),
                    // no step: the channels 1, 2, 3 (4 after them) against
                    // limits on, between and past them
                    (0.0, 0.5, 9.0, 3.0),
                    (0.0, 0.5, 9.0, 3.5),
                    (0.0, 0.5, 9.0, 1.5),
                    (0.0, 0.5, 9.0, 1.0),
                ] {
                    ring(&mut t, c, p, cp, 0x30 + kr as u8);
                    let n0 = node(0);
                    // the colours, targets and speeds +0x1c..+0x7c; the
                    // integer scales +0x8c
                    for j in 0..0x1c {
                        let v = (j as f32 - 9.0) * 0.375 + flags as f32 * 0.0625;
                        set_f(&mut t, n0 + 0x1c + 4 * j, v);
                    }
                    for j in 0..4 {
                        t.set_u32(n0 + 0x8c + 4 * j, [255, 128, 7, 0x8000_0000][j as usize]);
                    }
                    if rate == 0.0 {
                        for j in 0..4 {
                            set_f(&mut t, n0 + 0x1c + 4 * j, 1.0 + j as f32);
                        }
                    }
                    set_f(&mut t, n0 + 0x9c, rate);
                    set_f(&mut t, n0 + 0xa0, progress);
                    set_f(&mut t, n0 + 0xa4, end);
                    set_f(&mut t, n0 + 0xa8, limit);
                    t.set_u32(n0 + 0xac, flags | 0x5a00);
                    // leave_and_reset: alone, or under a parent
                    if flags == 7 {
                        let parent = node(5);
                        t.write(parent, &[0x45; 0x40]);
                        t.set_u32(n0 + 0x14, parent);
                        t.set_u32(n0 + 4, parent);
                        t.set_u32(n0 + 8, parent);
                        t.set_u16(parent + 0x18, 3);
                    } else {
                        t.set_u32(n0 + 0x14, 0);
                    }
                    check_this_thunk(&mut t, f, thunk, s4_summon::colour_node_fade, gs_eax(kr));
                }
            }
        }
    }
}

/// `leave_and_reset` through its own 12-byte thunk.
#[test]
fn leave_and_reset_thunk() {
    for (n, thunk) in [(3u16, 0x19b0u32), (87, 0x1bf0)] {
        let Some(mut t) = twin(n) else { return };
        let fade = t.at(if n == 3 { 0x1470 } else { 0x1620 });
        fade_stubs(&mut t, fade);
        let f = t.img.rel32_target(fade + 0x3e1);
        for parent in [false, true] {
            ring(&mut t, 0, 1, 2, 0x31);
            let n0 = node(0);
            for j in 0..4 {
                set_f(&mut t, n0 + 0x7c + 4 * j, 100.0 * j as f32 - 150.5);
            }
            if parent {
                let p = node(5);
                t.write(p, &[0x45; 0x40]);
                t.set_u32(n0 + 0x14, p);
                t.set_u32(n0 + 4, node(6));
                t.set_u32(n0 + 8, p);
                t.set_u16(p + 0x18, 0);
            } else {
                t.set_u32(n0 + 0x14, 0);
            }
            check_this_thunk(
                &mut t,
                f,
                thunk,
                |g, img, fr| ffx_magic::Ret::eax(ffx_magic::s3_nodes::leave_and_reset(g, img, fr)),
                Eax::Checked,
            );
        }
    }
}

fn motion_stubs(t: &mut Twin) {
    for s in [20u32, 411] {
        rec(t, s, 1, |_, a| a[0] & 1);
    }
    for s in [36u32, 228, 257, 296, 538, 255] {
        rec(t, s, 2, move |m, a| {
            // the vector behind the second argument
            let row = sa::node(31) + 0x10 * (s % 16);
            for i in 0..3 {
                m.w32(row + 4 * i, m.r32(a[1].wrapping_add(4 * i)));
            }
            0x0200_0000 + s
        });
    }
    rec(t, 217, 2, |m, a| {
        for (i, v) in [1.5f32, 3.25, -0.5].iter().enumerate() {
            m.w32(a[1] + 4 * i as u32, v.to_bits() ^ (a[0] & 0x10));
        }
        0x217
    });
    rec(t, 234, 3, |m, a| {
        for i in 0..3 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) - f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x234
    });
    for s in [667u32, 233] {
        rec(t, s, 3, move |m, a| {
            for i in 0..3 {
                let v = f32::from_bits(m.r32(a[1] + 4 * i)) * f32::from_bits(a[2]);
                m.w32(a[0] + 4 * i, v.to_bits());
            }
            s
        });
    }
    rec(t, 248, 3, |m, a| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x248
    });
    rec(t, 289, 1, |_, _| 0x289);
    t.stub_st0(289, |_, a| f64::from(a[0] & 0xf) * 0.5 - 1.0);
    rec(t, 534, 1, |_, a| a[0] & 2);
    rec(t, 664, 1, |_, _| 0x664);
    t.stub_st0(664, |_, _| 0.75);
}

#[test]
fn motion_state_steps() {
    for (n, f, thunk) in [(3u16, 0x2600u32, 0x2a70u32), (87, 0x2840, 0x2cb0)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(f);
        motion_stubs(&mut t);
        let k = f32::from_bits(t.u32(t.img.word(f + 0x322)));
        let flag_sets: &[u16] = &[
            0, 1, 2, 3, 0x10, 0x100, 0x101, 0x120, 0x121, 0x4, 0x5, 0x8, 0x9, 0x40, 0x41, 0x42,
            0x116, 0x1c, 0x48, 0x60, 0x140,
        ];
        for (kr, &(c, p, cp)) in RINGS.iter().enumerate() {
            for (case, &flags) in flag_sets.iter().enumerate() {
                for variant in 0u32..4 {
                    ring(&mut t, c, p, cp, 0x30 + kr as u8);
                    let n0 = node(0);
                    for j in 0..0x20 {
                        let v = (j as f32 - 5.0) * 0.5 + variant as f32;
                        set_f(&mut t, n0 + 0x1c + 4 * j, v);
                    }
                    t.set_u16(n0 + 0x90, flags | 0x8000);
                    // the model and the character (their low bits pick
                    // the stubs' answers)
                    t.set_u32(n0 + 0x50, 0x5000 + variant);
                    t.set_u32(n0 + 0x54, 0x5400 + (variant ^ 1));
                    t.set_u32(n0 + 0x58, [0, 3, 0, 1][variant as usize]);
                    t.set_u32(n0 + 0x5c, [0, 1, 0x8000_0001, 7][variant as usize]);
                    t.set_u32(n0 + 0x60, 0x60 + case as u32);
                    t.set_u32(n0 + 0x64, 0x64);
                    set_f(&mut t, n0 + 0x70, 0.5);
                    set_f(&mut t, n0 + 0x74, -2.0);
                    // the fall: height, its step, the damping, the bounce
                    let (h, step) = [
                        (5.0f32, 1.0f32),
                        (k - 0.5, 0.25),
                        (k + 3.0, -2.0),
                        (0.0, 0.0),
                    ][variant as usize];
                    set_f(&mut t, n0 + 0x78, h);
                    set_f(&mut t, n0 + 0x7c, step);
                    set_f(&mut t, n0 + 0x80, 0.5);
                    set_f(&mut t, n0 + 0x84, 3.0);
                    set_f(&mut t, n0 + 0x88, 0.125);
                    t.set_u16(n0 + 0x92, [1, 3, 0, 1][variant as usize]);
                    t.set_u16(n0 + 0x94, [0, 2, 0, 0][variant as usize]);
                    object(&mut t, 0x4c, 5, variant & 1 != 0, 0x22);
                    check_this_thunk(&mut t, f, thunk, s4_summon::motion_state, gs_eax(kr));
                }
            }
        }
    }
}

/// The summon's start (a table entry, `(actor)`): the targets, the lists,
/// the part and its program, with the lock word as the game left it.
#[test]
fn summon_start() {
    for (n, rva) in [(3u16, 0x68d0u32), (202, 0x7760)] {
        summon_start_in(n, rva);
    }
}

fn summon_start_in(n: u16, rva: u32) {
    let Some(mut t) = twin(n) else { return };
    let at = t.at(rva);
    let targets = t.img.rel32_target(at + 0x08);
    let statics = t.img.rel32_target(targets + 0xe0);
    let root = t.img.word(statics + 0x10d);
    let lock = sa::node(21);
    t.set_u32(t.img.word(at + 0xd4), lock);
    let list = sa::node(20);
    rec(&mut t, 168, 1, |_, a| 0x0168_0000 + a[0]);
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 2);
        list
    });
    rec(&mut t, 531, 2, |m, a| {
        m.w32(a[1], a[0]);
        0x531
    });
    rec(&mut t, 289, 1, |_, _| 0x289);
    t.stub_st0(289, |_, _| 2.5);
    rec(&mut t, 718, 1, |_, _| 0x718);
    for s in [222u32, 221] {
        rec(&mut t, s, 4, move |_, _| s);
    }
    rec(&mut t, 719, 3, |_, a| 0x7190_0000 ^ a[1]);
    rec(&mut t, 711, 3, move |m, a| {
        // the lock word while the program starts
        m.w32(list + 0x40, m.r32(lock));
        a[0] ^ 0x711
    });
    rec(&mut t, 710, 4, |_, _| 0x710);
    for (k, old) in [0u32, 7].into_iter().enumerate() {
        t.write(list, &[0x80, 0x81]);
        t.set_u32(lock, old);
        t.write(root, &[0x5a; 0x150]);
        t.set_u32(root + 4, 0);
        let flags = t.img.word(at + 0x197);
        t.set_u32(flags, 0x5555_5555 + k as u32);
        t.check_entry(at, magic::Call::default(), &[0x30 + k as u32], Eax::Checked);
    }
}

/// The summon's targets (internal, `ecx` = the actor): no targets, one,
/// three, and a list whose count grows while it is read; then the
/// statics' reset under a root with and without a child.
#[test]
fn targets_init() {
    for (n, rva) in [(3u16, 0x6390u32), (76, 0x62c0), (70, 0x9010)] {
        targets_init_in(n, rva);
    }
}

fn targets_init_in(n: u16, rva: u32) {
    let Some(mut t) = twin(n) else { return };
    let at = t.at(rva);
    let b = n != 3;
    let statics = t.img.rel32_target(at + 0xe0);
    let root = t.img.word(statics + 0x10d);
    let count_at = t.img.word(at + if b { 0x35 } else { 0x32 });
    rec(&mut t, 168, 1, |_, a| 0x0168_0000 + a[0]);
    let list = sa::node(20);
    rec(&mut t, 218, 2, move |m, a| {
        // the count the test left in the list's last word
        m.w32(a[1], m.r32(list + 0x7c));
        list
    });
    rec(&mut t, 531, 2, move |m, a| {
        for i in 0..4 {
            m.w32(a[1] + 4 * i, a[0].wrapping_mul(0x0101_0101) ^ i);
        }
        // the third target makes the list longer
        if a[0] == 0x83 {
            m.w32(count_at, 4);
        }
        0x531
    });
    rec(&mut t, 289, 1, |_, _| 0x289);
    t.stub_st0(289, |_, a| f64::from(a[0] & 0xff) * -0.5);
    for (k, count) in [0u32, 1, 3, 0xffff_fffe].into_iter().enumerate() {
        t.write(list, &[0x80, 0x81, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88]);
        t.set_u32(list + 0x7c, count);
        t.write(root, &[0x5a; 0x150]);
        t.set_u32(root + 4, if k % 2 == 0 { 0 } else { node(3) });
        t.write(node(3), &[0x33; 0x40]);
        t.set_u32(node(3) + 0x10, node(3));
        t.check(
            at,
            magic::Call::this(0x40 + k as u32),
            &[],
            Eax::Checked,
            |g, img, fr| {
                ffx_magic::Ret::eax(if n == 70 {
                    s4_summon::targets_init_c(g, img, fr.at, fr.esp, fr.ecx)
                } else if b {
                    s4_summon::targets_init_b(g, img, fr.at, fr.esp, fr.ecx)
                } else {
                    s4_summon::targets_init(g, img, fr.at, fr.esp, fr.ecx)
                })
            },
        );
    }
}

/// `magic_0411` 0x10006000: the node for every target (none, one, three)
/// under no parent, an empty parent and a parent with a child.
#[test]
fn target_nodes() {
    let Some(mut t) = twin(411) else { return };
    let f = t.at(0x6000);
    let list = magic::SCRATCH + 0x8000;
    t.write(list, &[5, 7, 9, 11]);
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], m.r32(magic::SCRATCH + 0x8100));
        list
    });
    for s in [413u32, 215] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    rec(&mut t, 14, 4, |m, a| {
        for i in 1..4 {
            m.w32(a[i], a[0].wrapping_mul(i as u32));
        }
        0x14
    });
    rec(&mut t, 333, 5, |_, _| 0x333);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.125);
    let (r, node, parent, kid) = (
        magic::SCRATCH + 0x100,
        magic::SCRATCH + 0x1000,
        magic::SCRATCH + 0x2000,
        magic::SCRATCH + 0x3000,
    );
    for (count, p, last) in [
        (0u32, 0, 0),
        (1, parent, parent),
        (3, parent, kid),
        (0xffff_ffff, parent, kid),
    ] {
        t.set_u32(magic::SCRATCH + 0x8100, count);
        t.write(node, &[0x66; 0x500]);
        t.write(parent, &[0x77; 0x40]);
        t.write(kid, &[0x88; 0x40]);
        t.set_u32(parent + 0x10, last);
        for (i, x) in [node, p, 0x0c0c_0003, 0x1234_5678, 0x0000_00f1]
            .into_iter()
            .enumerate()
        {
            t.set_u32(r + 4 * i as u32, x);
        }
        t.check(f, magic::Call::this(r), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(s4_summon::target_nodes(g, img, fr))
        });
    }
}

/// `magic_0070` 0x100057e0 (and its thunk): the stepping node with steps
/// (the time reaching `K3` or not), without (leaving its parent or not).
#[test]
fn step_node_steps() {
    let Some(mut t) = twin(70) else { return };
    let f = t.at(0x57e0);
    rec(&mut t, 776, 1, |_, a| 0x7760_0000 ^ a[0]);
    rec(&mut t, 915, 5, |_, _| 0x915);
    rec(&mut t, 234, 3, |m, a| {
        for i in 0..3 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x234
    });
    let k3 = f32::from_bits(t.u32(t.img.word(f + 0x113)));
    for (kr, &(c, p, cp)) in RINGS.iter().enumerate() {
        for (steps, time, rate, flags) in [
            (2u32, 0.0f32, 0.25f32, 0u8),
            (1, k3 - 0.125, 0.25, 0),
            (3, k3, 0.0, 1),
            (0, 0.0, 0.5, 0),
            (0, 0.0, 0.5, 1),
            (0, 0.0, 0.5, 3),
        ] {
            ring(&mut t, c, p, cp, 0x30 + kr as u8);
            let n0 = node(0);
            for j in 0..12 {
                set_f(&mut t, n0 + 0x1c + 4 * j, j as f32 * 0.75 - 3.0);
            }
            set_f(&mut t, n0 + 0x4c, time);
            set_f(&mut t, n0 + 0x50, rate);
            t.set_u32(n0 + 0x54, steps);
            t.set_u8(n0 + 0x58, flags);
            t.set_u16(n0 + 0x5a, 0x1234);
            // the parent to leave, when there is one
            if flags & 2 == 0 {
                t.set_u32(n0 + 0x14, 0);
            } else {
                let parent = node(5);
                t.write(parent, &[0x45; 0x40]);
                t.set_u32(n0 + 0x14, parent);
                t.set_u32(n0 + 4, parent);
                t.set_u32(n0 + 8, parent);
                t.set_u16(parent + 0x18, 3);
            }
            // the walk's `eax` is the cookie's junk when nothing is called
            let eax = if kr == 0 || steps == 0 {
                Eax::Junk
            } else {
                Eax::Checked
            };
            check_this_thunk(&mut t, f, 0x5950, s4_summon::step_node, eax);
        }
    }
}

/// `magic_0070` 0x10005f60 (and its thunk): the particle node done,
/// delayed (paused or not), fading, drawn (its part finished or not) and
/// calling back.
#[test]
#[allow(clippy::too_many_lines)] // one set-up per global
fn particle_node_draws() {
    let Some(mut t) = twin(70) else { return };
    let f = t.at(0x5f60);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let pause = magic::SCRATCH + 0x8000;
    t.set_u32(w(&t, 0x75), pause);
    t.set_u32(w(&t, 0x17e), magic::SCRATCH + 0x8100);
    let matrix = magic::SCRATCH + 0x8200;
    t.set_u32(w(&t, 0x1c2), matrix);
    t.set_u32(w(&t, 0x2b3), magic::SCRATCH + 0x8300);
    t.set_u32(w(&t, 0x2bf), magic::SCRATCH + 0x8400);
    t.set_u32(w(&t, 0x2d2), magic::SCRATCH + 0x8500);
    t.set_u32(w(&t, 0x2df), 0x00d0_00d0);
    t.set_u32(w(&t, 0x2e5), magic::SCRATCH + 0x8600);
    t.set_u32(w(&t, 0x2ee), magic::SCRATCH + 0x8610);
    t.set_u32(w(&t, 0x2f8), magic::SCRATCH + 0x8620);
    let fu = t.img.rel32_target(f + 0x328);
    t.set_u32(t.img.word(fu + 0x01), magic::SCRATCH + 0x8630);
    for s in [325u32, 230, 668, 231] {
        rec(&mut t, s, 2, move |m, a| {
            let x = m.r32(a[1]) ^ s;
            m.w32(a[0], x);
            s
        });
    }
    rec(&mut t, 233, 3, |m, a| {
        m.w32(a[0], a[2]);
        0x233
    });
    let done = magic::SCRATCH + 0x8640;
    rec(&mut t, 712, 2, move |m, _| {
        let lock = m.r32(magic::SCRATCH + 0x8620);
        m.w32(magic::SCRATCH + 0x8644, lock);
        m.r32(done)
    });
    rec(&mut t, 713, 1, |_, _| 0x713);
    rec(&mut t, 160, 3, |_, _| 0x160);
    for (kr, &(c, p, cp)) in RINGS.iter().enumerate() {
        for (flags, delay, paused, fin, callback) in [
            (2u32, 0u32, 0u32, 0u32, 0u32),
            (0, 3, 0, 0, 0),
            (0, 3, 1, 0, 0),
            (1, 0, 0, 0, 0x0cb0),
            (1, 0, 1, 1, 0x0cb0),
            (0, 0, 0, 1, 0),
            (1, 0xffff_fff0, 0, 0, 0x0cb1),
        ] {
            ring(&mut t, c, p, cp, 0x30 + kr as u8);
            let n0 = node(0);
            for j in 0..8 {
                set_f(&mut t, n0 + 0x2c + 4 * j, 200.0 - 23.5 * j as f32);
            }
            set_f(&mut t, n0 + 0x1c, 1.5);
            set_f(&mut t, n0 + 0x20, -2.0);
            set_f(&mut t, n0 + 0x24, 0.25);
            let parent = node(6);
            t.write(parent, &[0x46; 0x200]);
            set_f(&mut t, parent + 0x138, 2.0);
            t.set_u32(n0 + 0x4c, parent);
            t.set_u32(n0 + 0x50, 0x5050);
            t.set_u32(n0 + 0x14, 0);
            t.set_u32(n0 + 0xe0, callback);
            t.set_u16(n0 + 0xe4, (kr as u16) * 3);
            t.set_u16(n0 + 0xe6, 8);
            t.set_u32(n0 + 0xe8, flags);
            t.set_u32(n0 + 0xec, delay);
            t.set_u32(n0 + 0xf4, 0xf4f4);
            t.set_u32(pause, paused);
            t.set_u32(done, fin);
            for i in 0..16 {
                set_f(&mut t, matrix + 4 * i, i as f32);
            }
            let eax = if kr == 0 || flags & 2 != 0 {
                Eax::Junk
            } else {
                Eax::Checked
            };
            check_this_thunk(&mut t, f, 0x6320, s4_summon::particle_node, eax);
        }
    }
}
