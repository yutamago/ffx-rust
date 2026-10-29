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
        m.w32(sa::node(30) + (a[0] & 0xfc), m.r32(a[1] + 0x30) ^ m.r32(a[1] + 0x3c));
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
                    let (h, step) = [(5.0f32, 1.0f32), (k - 0.5, 0.25), (k + 3.0, -2.0), (0.0, 0.0)]
                        [variant as usize];
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
