//! The summon family's node handlers (`ffx_magic::sa_summon`) against
//! `magic_0003.dll`. Needs `FFX_GAME_DIR`.

#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

mod magic;
#[path = "magic/sa.rs"]
mod sa;

use magic::{Call, Eax, TRAPS, Twin};
use sa::{CURSOR, RINGS, VTABLE_2, check_this, check_this_eax, handler_stubs, node, rec, ring};

/// A /GS handler's `eax` is the cookie's junk on the first ring (no move).
fn gs_eax(k: usize) -> Eax {
    if k == 0 { Eax::Junk } else { Eax::Checked }
}

const N: u16 = 3;

fn twin() -> Option<Twin> {
    let mut t = sa::twin(N)?;
    handler_stubs(&mut t);
    Some(t)
}

#[test]
fn place_by_model_number() {
    let Some(mut t) = twin() else { return };
    rec(&mut t, 366, 2, |_, _| 0x366);
    rec(&mut t, 217, 2, |_, _| 0x217);
    let f = t.at(0x2e90);
    for (k, &(c, p, cp)) in RINGS.iter().enumerate() {
        for flag in [0u8, 1, 0xfe, 0xff] {
            ring(&mut t, c, p, cp, 0x40 + k as u8);
            t.set_u8(node(0) + 0x2d, flag);
            t.set_u8(node(0) + 0x2c, 0x80 | flag);
            check_this(&mut t, f, ffx_magic::sa_summon::place);
        }
    }
}

#[test]
fn model_setup_by_flags() {
    let Some(mut t) = twin() else { return };
    for s in [538, 213, 211] {
        rec(&mut t, s, 2, move |_, a| s << 16 | a[1] & 0xffff);
    }
    rec(&mut t, 547, 1, |_, a| 0x5470_0000 ^ a[0]);
    rec(&mut t, 35, 5, |_, _| 0x35);
    rec(&mut t, 34, 2, |_, _| 0x34);
    rec(&mut t, 276, 2, |_, a| 0x2760_0000 ^ a[0] ^ a[1]);
    // the parts: answers from the cells at node(5)
    rec(&mut t, 219, 2, |m, a| m.r32(node(5) + 4 * (a[1] & 1)));
    let f = t.at(0x24b0);
    for flags in [0u16, 1, 0x10, 0x11, 0x800, 0x801, 0x810, 0xffff, 0xf7fe] {
        for (motion, parts) in [
            (u32::MAX, [0u32, 0]),
            (7, [0x111, 0]),
            (0, [0, 0x222]),
            (9, [0x1, 0x2]),
        ] {
            ring(&mut t, 0, 1, 1, 0x3c);
            t.set_u16(node(0) + 0x90, flags);
            t.set_u32(node(0) + 0x64, motion);
            t.set_u32(node(5), parts[0]);
            t.set_u32(node(5) + 4, parts[1]);
            check_this(&mut t, f, ffx_magic::sa_summon::model_setup);
        }
    }
}

#[test]
fn sub_walk_runs_the_inner_ring() {
    let Some(mut t) = twin() else { return };
    rec(&mut t, 229, 4, |_, a| 0x2290_0000 ^ a[1]);
    let f = t.at(0x3fc0);
    for (k, &(c, p, cp)) in RINGS.iter().enumerate() {
        ring(&mut t, c, p, cp, 0x50 + k as u8);
        // the node's object: a node with the second handler table
        let obj = node(7);
        t.write(obj, &[0x77; 0x100]);
        t.set_u32(obj, VTABLE_2);
        t.set_u32(node(0) + 0x3c, obj);
        check_this(&mut t, f, ffx_magic::sa_summon::sub_walk);
    }
}

#[test]
fn scaled_place_scales() {
    let Some(mut t) = twin() else { return };
    rec(&mut t, 290, 2, |m, a| {
        // the callee moves the node: the scaling reads after it
        m.w32(a[1], 2.5f32.to_bits());
        0x290
    });
    let f = t.at(0x3100);
    for (k, &(c, p, cp)) in RINGS.iter().enumerate() {
        ring(&mut t, c, p, cp, 0x60 + k as u8);
        for (i, v) in [1.5f32, -0.0, 3.0e38, 0.1, 2.0, f32::NAN, 7.0]
            .iter()
            .enumerate()
        {
            t.set_u32(node(0) + 0x20 + 4 * i as u32, v.to_bits());
        }
        check_this(&mut t, f, ffx_magic::sa_summon::scaled_place);
    }
}

#[test]
fn aim_handler() {
    let Some(mut t) = twin() else { return };
    rec(&mut t, 237, 4, |_, _| 0x237);
    rec(&mut t, 248, 7, |_, a| 0x2480_0000 ^ a[2]);
    let f = t.at(0x2370);
    for (k, &(c, p, cp)) in RINGS.iter().enumerate() {
        ring(&mut t, c, p, cp, 0x70 + k as u8);
        t.check_entry(
            f,
            Call {
                eax: 0x1111_2222,
                ecx: 0x3333,
                edx: 0,
                ..Call::default()
            },
            &[CURSOR, 0x0bad_f00d],
            Eax::Checked,
        );
    }
}

#[test]
fn model_node_init_links_and_loads() {
    let Some(mut t) = twin() else { return };
    rec(&mut t, 413, 1, |_, a| 0x4130_0000 ^ a[0]);
    rec(&mut t, 215, 1, |_, a| 0x2150_0000 ^ a[0]);
    rec(&mut t, 14, 4, |m, a| {
        m.w32(a[1], 0x11);
        m.w32(a[2], 0x22);
        m.w32(a[3], 0x33);
        0x14
    });
    rec(&mut t, 333, 5, |_, _| 0x333);
    let f = t.at(0x23f0);
    let args = sa::node(9);
    for (k, (parent_kids, flags, st0)) in [
        (None, 0u8, Some(0.75f64)),
        (Some(0u32), 1, Some(-1.0e-3)),
        (Some(1), 0xfe, Some(1.0e300)),
        (Some(2), 0xff, Some(0.1)),
        (Some(0), 0, None),
    ]
    .into_iter()
    .enumerate()
    {
        match st0 {
            Some(v) => t.stub_st0(333, move |_, _| v),
            None => t.stub_st0(333, |_, _| f64::NAN),
        }
        let me = node(8);
        t.write(me, &[0x20 + k as u8; 0x100]);
        let parent = match parent_kids {
            None => 0,
            Some(kids) => {
                ring(&mut t, kids, 0, 0, 0x30);
                // ring() links children through +8 only; the parent's
                // last child is +0x10 here
                let last = if kids == 0 { node(0) } else { node(kids) };
                t.set_u32(node(0) + 0x10, last);
                t.set_u16(node(0) + 0x18, 0x7ffe);
                node(0)
            }
        };
        t.set_u32(args, me);
        t.set_u32(args + 4, parent);
        t.set_u32(args + 8, 0x00ab_0000 + k as u32);
        t.set_u32(args + 0xc, 0xeeee_ee00 | u32::from(flags));
        t.check(f, Call::this(args), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_summon::model_node_init(g, img, fr.at, fr.ecx))
        });
    }
}

#[test]
fn bind_models_sets_the_flag() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x6490);
    rec(&mut t, 723, 1, |_, _| 0x723);
    rec(&mut t, 724, 1, |_, _| 0x724);
    rec(&mut t, 230, 2, |_, a| 0x2300_0000 ^ a[0]);
    rec(&mut t, 253, 0, |m, _| m.r32(node(6)));
    let (d, e, flag) = (
        t.img.word(f + 0x47),
        t.img.word(f + 0x5f),
        t.img.word(f + 0x7e),
    );
    t.set_u32(d, 0x00dd_0000);
    t.set_u32(e, 0x00ee_0000);
    t.set_u32(flag, node(6) + 0x10);
    for answer in [0u32, 1, 0x8000_0000] {
        t.set_u32(node(6), answer);
        t.set_u32(node(6) + 0x10, 0x5555_5555);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_summon::bind_models(g, img, fr.at))
        });
    }
}

#[test]
fn vector_op_by_operation() {
    let Some(mut t) = twin() else { return };
    rec(&mut t, 262, 4, |_, _| 0x262);
    rec(&mut t, 29, 2, |_, _| 0x29);
    rec(&mut t, 268, 3, |_, _| 0x268);
    for s in [366, 665, 531] {
        rec(&mut t, s, 2, move |_, _| s);
    }
    rec(&mut t, 12, 1, |_, _| 0x12);
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * 0.3);
    let f = t.at(0x2f30);
    let owner = node(6);
    for (k, &(c, p, cp)) in RINGS.iter().enumerate() {
        for op in [0u8, 1, 2, 3, 4, 5, 6, 7, 0xff] {
            for oflags in [0u8, 1] {
                ring(&mut t, c, p, cp, 0x30 + k as u8);
                t.write(owner, &[0x44; 0x100]);
                t.set_u8(owner + 0x90, oflags);
                // the vector +0x1c..+0x24, its velocity +0x2c..+0x34
                for (i, v) in [1.0f32, -2.5, 1.0e-3].iter().enumerate() {
                    t.set_u32(node(0) + 0x1c + 4 * i as u32, v.to_bits());
                    t.set_u32(node(0) + 0x2c + 4 * i as u32, (v * 3.0).to_bits());
                }
                t.set_u32(node(0) + 0x38, owner);
                t.set_u32(node(0) + 0x3c, 0x0003_c3c3);
                t.set_u8(node(0) + 0x40, op);
                check_this(&mut t, f, ffx_magic::sa_summon::vector_op);
            }
        }
    }
}

#[test]
fn bounce_rebuilds_the_vector() {
    let Some(mut t) = twin() else { return };
    // slot 248 writes the local vector from the cells at node(6)
    rec(&mut t, 248, 3, |m, a| {
        for i in 0..3 {
            let v = m.r32(node(6) + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x248
    });
    rec(&mut t, 325, 2, |m, a| {
        // log the local vector it is given
        for i in 0..3 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(node(6) + 0x20 + 4 * i, v);
        }
        0x325
    });
    let f = t.at(0x5e80);
    for (k, &(c, p, cp)) in RINGS.iter().enumerate() {
        for (y, rate, count, limit) in [
            (5.0f32, 2.0f32, 3u16, 10.0f32),
            (-1.0, 2.0, 8, 10.0),
            (1.0e6, -0.5, 9, 10.0),
            (f32::NAN, 1.0, 0xffff, 1.0),
            (3.0, 0.0, 0, f32::NAN),
            (100.0, 4.0, 0xfffe, 0.5),
        ] {
            ring(&mut t, c, p, cp, 0x30 + k as u8);
            let parent = node(5);
            t.write(parent, &[0x41; 0x40]);
            t.set_u32(node(0) + 0x14, parent);
            t.set_u32(node(6), 1.5f32.to_bits());
            t.set_u32(node(6) + 4, y.to_bits());
            t.set_u32(node(6) + 8, (-2.25f32).to_bits());
            for (o, v) in [
                (0x1c, 1.25f32),
                (0x20, rate),
                (0x24, -3.5),
                (0x2c, 0.75),
                (0x30, 0.9),
                (0x34, -0.098),
                (0x38, 0.5),
                (0x3c, limit),
            ] {
                t.set_u32(node(0) + o, v.to_bits());
            }
            t.set_u16(node(0) + 0x40, 7);
            t.set_u16(node(0) + 0x42, count);
            check_this_eax(&mut t, f, ffx_magic::sa_summon::bounce, gs_eax(k));
        }
    }
}

#[test]
fn pair_aim_counts_down() {
    let Some(mut t) = twin() else { return };
    rec(&mut t, 229, 3, |_, a| 0x2290_0000 ^ a[1]);
    for s in [220, 393, 408] {
        rec(&mut t, s, 3, move |_, a| s << 16 ^ a[0]);
    }
    let f = t.at(0x40e0);
    for (k, &(c, p, cp)) in RINGS.iter().enumerate() {
        for flags in [0u32, 1, 2, 3, 0xffff_fffd] {
            for count in [0u32, 1, 5] {
                ring(&mut t, c, p, cp, 0x30 + k as u8);
                for (o, i) in [(0x20, 6), (0x1c, 7)] {
                    t.write(node(i), &[0x66; 0x100]);
                    t.set_u32(node(i), VTABLE_2);
                    t.set_u32(node(0) + o, node(i));
                }
                t.set_u32(node(0) + 0x2c, flags);
                t.set_u32(node(0) + 0x30, count);
                check_this_eax(&mut t, f, ffx_magic::sa_summon::pair_aim, gs_eax(k));
            }
        }
    }
}

/// A function slot no overlay uses (`Ch_SetPos`): the bones' table entries.
const TABLE_SLOT: u32 = 32;

fn bone_case(
    t: &mut Twin,
    f: u32,
    port: fn(&mut magic::PortGame, &ffx_magic::Image, &ffx_magic::Frame) -> ffx_magic::Ret,
) {
    rec(t, TABLE_SLOT, 2, |_, a| 0x0032_0000 ^ a[0]);
    rec(t, 233, 3, |m, a| {
        m.w32(a[0], a[2] ^ 0x8000_0000);
        0x233
    });
    rec(t, 325, 2, |_, _| 0x325);
    rec(t, 230, 2, |_, a| 0x2300_0000 ^ a[1]);
    rec(t, 668, 3, |_, _| 0x668);
    rec(t, 231, 3, |_, a| 0x2310_0000 ^ a[2]);
    rec(t, 229, 3, |_, _| 0x229);
    let table = t.img.word(f + 0xed);
    for i in 0..4 {
        t.set_u32(table + 4 * i, TRAPS + 16 * TABLE_SLOT);
    }
    for (k, &(c, p, cp)) in RINGS.iter().enumerate() {
        for (parent, which) in [(false, 0u8), (true, 1), (false, 2), (true, 3)] {
            ring(t, c, p, cp, 0x30 + k as u8);
            for j in 0..0x38 {
                let v = 1.0f32 / (j as f32 + 3.0);
                t.set_u32(node(0) + 0x5c + 4 * j, v.to_bits());
            }
            for (o, i) in [(0x13c, 5), (0x144, 6), (0x140, 7), (0x128, 8)] {
                t.write(node(i), &[0x50 + i as u8; 0x200]);
                t.set_u32(node(i), VTABLE_2);
                for j in 0..0x40 {
                    let v = (j as f32 + 1.0) * 0.25 * (i as f32);
                    t.set_u32(node(i) + 0x1c + 4 * j, v.to_bits());
                }
                if o != 0x128 || parent {
                    t.set_u32(node(0) + o, node(i));
                } else {
                    t.set_u32(node(0) + o, 0);
                }
            }
            t.set_u8(node(0) + 0x148, which);
            check_this_eax(t, f, port, gs_eax(k));
        }
    }
}

#[test]
fn bone_plain_updates() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x5820);
    bone_case(&mut t, f, ffx_magic::sa_summon::bone_plain);
}

#[test]
fn bone_scaled_updates() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x2b20);
    bone_case(&mut t, f, ffx_magic::sa_summon::bone_scaled);
}

#[test]
fn statics_init_inserts_three() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x6250);
    // the root holds a child already, and the statics hold old words
    let root = t.img.word(f + 0x10d);
    for op in [0x14, 0x64, 0xbc] {
        let a = t.img.word(f + op) & !0xf;
        t.write(a, &[0xa5; 0x40]);
    }
    for parent in [0u32, 1] {
        t.write(root, &[0x5a; 0x150]);
        t.set_u32(root + 4, if parent == 0 { 0 } else { node(3) });
        t.write(node(3), &[0x33; 0x40]);
        t.set_u32(node(3) + 0x10, node(3));
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_summon::statics_init(g, img, fr.at, fr.esp))
        });
    }
}
