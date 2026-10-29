//! The shared-3 list's ports against the DLLs they were ported from
//! (`ffx_magic::s3_*`, the frameless slot thunks of `ffx_magic::runtime`).
//! The register thunks of `ffx_magic::s3_wrap` are checked with their
//! handlers' tests (`Twin::check_thunked`, `sa::check_this_thunk`).
//! Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod magic;

use magic::{Call, Eax, Twin};

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

/// `mov eax, [G]; mov eax, [eax + 4 * slot]; jmp eax` (13 bytes): the
/// slot's function with the caller's stack.
#[test]
fn bare_slot_thunks() {
    for (n, rvas) in [(297u16, &[0x8bf0u32, 0x8be0, 0x8bd0][..]), (275, &[0x3110])] {
        let Some(mut t) = twin(n) else { return };
        for rva in rvas {
            let slot = (t.img.word(t.at(*rva) + 7)) / 4;
            t.stub(slot, move |m, a| {
                m.w32(magic::SCRATCH + 4 * slot, a[0] ^ a[2]);
                a[1].wrapping_add(slot)
            });
            t.check_entry(
                t.at(*rva),
                Call {
                    eax: 1,
                    ecx: 0x5555,
                    edx: 7,
                    ..Call::default()
                },
                &[3, 5, 9, 11],
                Eax::Checked,
            );
        }
    }
}

use ffx_magic::{Ret, Xmm, s3_misc, s3_motion, s3_nodes, s3_summon};
use magic::SCRATCH;

const LOG: u32 = SCRATCH + 0x3_0000;

/// Slot `s` logs its first `n` words in row `k` (0x80 bytes from `LOG`),
/// the four words behind its first `deref` arguments too, and answers
/// `ret`.
fn log_slot(t: &mut Twin, s: u32, n: usize, deref: usize, k: u32, ret: u32) {
    let row = LOG + 0x80 * k;
    t.stub(s, move |m, a| {
        let c = m.r32(row + 0x7c).wrapping_add(1);
        m.w32(row + 0x7c, c);
        for (i, w) in (0u32..).zip(&a[..n]) {
            m.w32(row + 4 * i, w.wrapping_add(c << 28));
        }
        for (i, p) in (0u32..).zip(&a[..deref]) {
            for j in 0..4 {
                m.w32(row + 0x40 + 0x10 * i + 4 * j, m.r32(p + 4 * j));
            }
        }
        ret
    });
}

/// `magic_0003` 0x10001340: the node leaves its parent (every position in
/// the parent's list, or no parent), then the display resets.
#[test]
fn leave_and_reset() {
    let Some(mut t) = twin(3) else { return };
    let f = t.at(0x1340);
    let ratio = t.at(0x1870);
    for (o, x) in [(0x94u32, 1.0f32), (0x9c, 2.0), (0xb1, 4.0), (0xb9, 0.5)] {
        t.set_u32(t.img.word(ratio + o), x.to_bits());
    }
    t.set_u32(t.img.word(f + 0xfc), 0.75f32.to_bits());
    for (k, (s, n, d)) in [
        (332u32, 1usize, 1usize),
        (353, 2, 1),
        (267, 3, 0),
        (271, 4, 0),
        (669, 5, 1),
        (785, 1, 0),
        (801, 6, 0),
    ]
    .into_iter()
    .enumerate()
    {
        log_slot(&mut t, s, n, d, k as u32, 0x0100_0000 + s);
    }
    let (cur, node, parent, s1, s2) = (
        SCRATCH,
        SCRATCH + 0x1000,
        SCRATCH + 0x2000,
        SCRATCH + 0x3000,
        SCRATCH + 0x3400,
    );
    let floats: [[f32; 4]; 3] = [
        [1.5, -2.7, 3.0e9, f32::NAN],
        [0.0, -0.0, 65_535.9, -2_147_483_648.0],
        [100.25, -1.0e-3, 7.0, -2_147_483_904.0],
    ];
    for (k, (p, prev, next)) in [
        (0, node, node),
        (parent, parent, parent),
        (parent, parent, s1),
        (parent, s1, parent),
        (parent, s1, s2),
    ]
    .into_iter()
    .enumerate()
    {
        for v in &floats {
            for a in [node, parent, s1, s2] {
                t.write(a, &[0x33; 0x100]);
            }
            t.set_u32(cur, node);
            t.set_u32(node + 0x14, p);
            t.set_u32(node + 4, prev);
            t.set_u32(node + 8, next);
            t.set_u16(parent + 0x18, k as u16);
            for (i, x) in (0u32..).zip(v) {
                t.set_u32(node + 0x7c + 4 * i, x.to_bits());
            }
            t.check(f, Call::this(cur), &[], Eax::Checked, |g, img, fr| {
                Ret::eax(s3_nodes::leave_and_reset(g, img, fr))
            });
        }
    }
}

/// `magic_0042` 0x100083c0 and 0x10008570: a node's set-up record on the
/// frame for the particle and effect node builders.
#[test]
fn node_with_record() {
    let Some(mut t) = twin(42) else { return };
    let f = t.at(0x83c0);
    let pn = t.img.rel32_target(f + 0x35);
    let wrapper = t.img.rel32_target(pn + 0x88);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(t.img.word(wrapper + 0x04), lock);
    t.set_u32(lock, 0x77);
    t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x7f10);
    let owner = SCRATCH + 0x7a00;
    t.set_u32(t.img.word(f + 0x0d), owner);
    for i in 0..0x40 {
        t.set_u32(owner + 4 * i, 0x0a00_0000 + i);
    }
    let table = SCRATCH + 0x7c00;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x9000 + i);
    }
    log_slot(&mut t, 941, 1, 0, 0, table);
    t.stub(714, move |m, a| {
        for (i, w) in (0u32..).zip(&a[..5]) {
            m.w32(LOG + 0x400 + 4 * i, *w);
        }
        m.w32(LOG + 0x420, m.r32(lock));
        0x714
    });
    log_slot(&mut t, 325, 2, 2, 1, 0x325);
    log_slot(&mut t, 719, 3, 0, 2, 0x7190_0719);
    let node = SCRATCH + 0x1000;
    for (k, rva) in [0x83c0u32, 0x8570].into_iter().enumerate() {
        // the DLL's list: empty, or with a last child
        let list = t
            .img
            .word(t.at(rva) + if rva == 0x83c0 { 0x26 } else { 0x1e });
        let kid = SCRATCH + 0x1400;
        for (index, last) in [(0u32, list), (3, kid), (7, list)] {
            t.write(node, &[0xa5; 0x100]);
            t.write(kid, &[0x3c; 0x40]);
            t.set_u32(list + 0x10, last);
            t.set_u16(list + 0x18, 0xfffe);
            t.check(
                t.at(rva),
                Call::this(node),
                &[0x0808_0808, 0x0c0c_0c0c + k as u32, 0xdead, index],
                Eax::Checked,
                |g, img, fr| {
                    Ret::eax(if rva == 0x83c0 {
                        s3_nodes::particle_node_with(g, img, fr)
                    } else {
                        s3_nodes::effect_node_with(g, img, fr)
                    })
                },
            );
        }
    }
}

/// A pool node at rest (`+0x0`/`+0x4` itself, `+0x8` itself, the rest
/// filled), for `give_back`.
fn rest_node(t: &mut Twin, n: u32) {
    t.write(n, &[0x5a; 0x1c]);
    t.set_u32(n, n);
    t.set_u32(n + 4, n);
    t.set_u32(n + 8, n);
    t.set_u32(n + 0xc, 0);
}

/// `magic_0072` 0x10003500: the colour fade task waiting, fading (with
/// and without the byte colours) and ending (given back).
#[test]
fn colour_fade() {
    let Some(mut t) = twin(72) else { return };
    let f = t.at(0x3500);
    let clock = SCRATCH + 0x100;
    t.set_u32(t.img.word(f + 0x11), clock);
    let four = t.at(0x1000);
    t.set_u32(t.img.word(four + 0x87), 2.0f32.to_bits());
    for (k, (s, n, d)) in [
        (353u32, 1usize, 1usize),
        (397, 1, 1),
        (933, 4, 0),
        (785, 1, 0),
    ]
    .into_iter()
    .enumerate()
    {
        log_slot(&mut t, s, n, d, k as u32, 0x0200_0000 + s);
    }
    let (task, st) = (SCRATCH + 0x1000, SCRATCH + 0x2000);
    for (delay, left, total, dt, flag) in [
        (5i32, 10i32, 20i32, 2i32, 1u32),
        (1, 10, 20, 2, 1),
        (0, 10, 20, 2, 1),
        (0, 10, 20, 2, 0),
        (0, 3, 20, 4, 1),
        (-2, 1, 7, 1, 1),
        (0, 0x7fff_0000, 0x7fff_ffff, 3, 1),
        (0, 5, -3, 1, 1),
    ] {
        rest_node(&mut t, task);
        t.set_u32(task + 0x14, st);
        t.write(st, &[0x44; 0x40]);
        for i in 0..4u32 {
            t.set_u32(st + 4 * i, (0.25f32 * (i as f32 + 1.0)).to_bits());
            t.set_u32(st + 0x10 + 4 * i, (2.0f32 - i as f32).to_bits());
            t.set_u8(st + 0x20 + i, 0x10 * i as u8 + 3);
            t.set_u8(st + 0x24 + i, 0xff - 0x20 * i as u8);
        }
        t.set_u32(st + 0x28, delay as u32);
        t.set_u32(st + 0x2c, left as u32);
        t.set_u32(st + 0x30, total as u32);
        t.set_u32(st + 0x34, flag);
        t.set_u32(clock, dt as u32);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0162` 0x10007a90: the colour step task.
#[test]
fn colour_step() {
    let Some(mut t) = twin(162) else { return };
    let f = t.at(0x7a90);
    log_slot(&mut t, 776, 1, 0, 0, 0x776);
    log_slot(&mut t, 915, 5, 0, 1, 0x915);
    let (task, st) = (SCRATCH + 0x1000, SCRATCH + 0x2000);
    for (delay, left, total, hold) in [
        (3i32, 4u32, 8u32, 0u32),
        (0, 4, 8, 0),
        (0, 1, 8, 2),
        (0, 0, 8, 2),
        (0, 0, 8, 0),
        (-1, 9, 9, 0),
    ] {
        rest_node(&mut t, task);
        t.set_u32(task + 0x14, st);
        t.write(st, &[0x44; 0xa0]);
        for i in 0..4u32 {
            t.set_u8(st + 0x80 + i, 0x10 * i as u8 + 3);
            t.set_u8(st + 0x84 + i, 0xff - 0x30 * i as u8);
        }
        t.set_u32(st + 0x88, delay as u32);
        t.set_u32(st + 0x8c, left);
        t.set_u32(st + 0x90, total);
        t.set_u32(st + 0x94, hold);
        t.set_u8(st + 0x98, 7);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0042` 0x10002190: a colour node under no parent, an empty
/// parent and a parent with a child.
#[test]
fn colour_node() {
    let Some(mut t) = twin(42) else { return };
    let f = t.at(0x2190);
    t.stub_st0(790, |_, _| 0.375);
    log_slot(&mut t, 331, 1, 0, 0, 0x331);
    t.stub(558, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0xffff_fff0u32.wrapping_add(0x11 * i));
        }
        0x558
    });
    log_slot(&mut t, 353, 3, 1, 1, 0x353);
    log_slot(&mut t, 271, 4, 0, 2, 0x271);
    let (grp, node, parent, kid) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x2000,
        SCRATCH + 0x3000,
    );
    for (p, last) in [(0, 0), (parent, parent), (parent, kid)] {
        t.write(node, &[0x66; 0xb0]);
        t.write(parent, &[0x77; 0x40]);
        t.write(kid, &[0x88; 0x40]);
        t.set_u32(grp, node);
        t.set_u32(grp + 4, p);
        t.set_u32(parent + 0x10, last);
        t.check(f, Call::this(grp), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(s3_misc::colour_node(g, img, fr))
        });
    }
}

/// `magic_0084` 0x10001d60: a fade record on a pool node, appended to an
/// empty and a filled task list; the values clamped or kept.
#[test]
fn fade_task() {
    let Some(mut t) = twin(84) else { return };
    let f = t.at(0x1d60);
    let take = t.img.rel32_target(f + 0x75);
    let (count, free) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
    let head = t.img.word(f + 0x7b);
    let tail = t.img.word(f + 0x93);
    let rec = SCRATCH + 0x100;
    let pool = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    for (k, (a, b)) in [
        (1.5f32, 2.5f32),
        (0.0, -1.0),
        (-0.0, f32::NAN),
        (f32::NAN, 1.0e-30),
    ]
    .into_iter()
    .enumerate()
    {
        // a free ring of three
        for i in 1..=3u32 {
            t.write(pool(i), &[0x5a; 0x1c]);
            t.set_u32(pool(i), pool(if i == 1 { 3 } else { i - 1 }));
            t.set_u32(pool(i) + 4, pool(if i == 3 { 1 } else { i + 1 }));
        }
        t.set_u32(free, pool(1));
        t.set_u16(count, 3);
        t.write(rec, &[0x11; 0x40]);
        if k % 2 == 0 {
            t.set_u32(head, 0);
            t.set_u32(tail, 0);
        } else {
            t.write(pool(5), &[0x22; 0x1c]);
            t.set_u32(head, pool(5));
            t.set_u32(tail, pool(5));
        }
        t.check(
            f,
            Call {
                ecx: rec,
                edx: 0x00ed_0000 + k as u32,
                ..Call::default()
            }
            .with_xmm(&[Xmm::f32(9.0), Xmm::f32(a), Xmm::f32(b)]),
            &[0x0a0a_0000 + k as u32],
            Eax::Checked,
            |g, img, fr| Ret::eax(s3_misc::fade_task(g, img, fr)),
        );
    }
}

/// `magic_0162` 0x10005960 and 0x100059f0: key tracks stepping (eased,
/// plain and through the curve) and on their last step.
#[test]
fn key_steps() {
    let Some(mut t) = twin(162) else { return };
    let (track, keys, out, ctl) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x300,
    );
    for i in 0..30u32 {
        let x = (i as f32 * 0.7).sin() * 10.0 + i as f32;
        t.set_u32(keys + 4 * i, x.to_bits());
    }
    for (i, c) in [0.5f32, 1.0, -2.0, 3.5, 4.0].iter().enumerate() {
        t.set_u32(ctl + 4 * i as u32, c.to_bits());
    }
    for (rva, curved) in [(0x5960u32, false), (0x59f0, true)] {
        for (left, tm, step) in [
            (5u32, 0.0f32, 0.25f32),
            (5, 1.5, 0.3),
            (1, 2.0, 0.5),
            (9, 7.6, 0.5),
            (3, 0.2, 0.1),
        ] {
            t.write(track, &[0x33; 0x40]);
            t.write(out, &[0xee; 0x10]);
            t.set_u32(track + 0x20, keys);
            t.set_u32(track + 0x28, ctl);
            t.set_u32(track + 0x2c, tm.to_bits());
            t.set_u32(track + 0x30, step.to_bits());
            t.set_u32(track + 0x34, left);
            t.set_u32(track + 0x38, 9);
            // the eased path leaves ease_sample's scratch in eax
            let eax = if left == 1 { Eax::Checked } else { Eax::Junk };
            t.check(
                t.at(rva),
                Call {
                    ecx: track,
                    edx: out,
                    ..Call::default()
                },
                &[],
                eax,
                |g, img, fr| {
                    Ret::eax(if curved {
                        s3_misc::key_step_curved(g, img, fr)
                    } else {
                        s3_misc::key_step(g, img, fr)
                    })
                },
            );
        }
    }
}

/// `magic_0018` 0x10002c40: the motion script in every mode, through
/// each command and each end word, from a wait and from a finished
/// motion.
#[test]
#[allow(clippy::too_many_lines)]
fn motion_step() {
    let Some(mut t) = twin(18) else { return };
    let f = t.at(0x2c40);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let (done, wait, mode, chr, obj, pos, script, index) = (
        w(&t, 0x02),
        w(&t, 0x11),
        w(&t, 0x24),
        w(&t, 0x32),
        w(&t, 0x45),
        w(&t, 0x75),
        w(&t, 0x7a),
        w(&t, 0xa3),
    );
    let speed = t.img.rel32_target(f + 0xa8);
    let bytes = SCRATCH + 0x800;
    t.write(bytes, &[3, 0x25, 1, 0x40]);
    t.set_u32(t.img.word(speed + 0x0a), bytes);
    t.set_u32(w(&t, 0xd8), bytes);
    t.set_u32(w(&t, 0xd1), 0x00ee_0001);
    t.set_u32(w(&t, 0x13e), 0x0b0b_0b0b);
    // the character table the actor list's bytes index
    for i in 0..0x40 {
        t.set_u32(w(&t, 0xe3) + 4 * i, 0x00c0_0000 + i);
    }
    let answers = SCRATCH + 0x900;
    for (k, (s, n)) in [
        (411u32, 1usize),
        (20, 1),
        (23, 2),
        (3, 1),
        (529, 1),
        (228, 2),
        (30, 1),
        (36, 3),
        (37, 4),
        (18, 4),
        (213, 2),
        (530, 2),
        (249, 2),
    ]
    .into_iter()
    .enumerate()
    {
        let row = LOG + 0x80 * k as u32;
        let cell = answers + 4 * k as u32;
        t.stub(s, move |m, a| {
            let c = m.r32(row + 0x7c).wrapping_add(1);
            m.w32(row + 0x7c, c);
            for (i, x) in (0u32..).zip(&a[..n]) {
                m.w32(row + 4 * i, x.wrapping_add(c << 28));
            }
            // the motion calls move the object on: each read of it counts
            if matches!(s, 30 | 36 | 37) {
                m.w32(obj, m.r32(obj).wrapping_add(1));
            }
            m.r32(cell)
        });
    }
    let entries = SCRATCH + 0x1000;
    let scripts: [&[(i32, i32)]; 9] = [
        &[(-2, 0x80), (-3, 0), (-4, 0), (5, 3)],
        &[(-5, 0x11)],
        &[(-1, -1)],
        &[(7, -7)],
        &[(9, -2)],
        &[(-4, 0), (9, -4)],
        &[(9, -3), (9, -5)],
        &[(-6, -6)],
        &[(0x123, 0x0001_0007)],
    ];
    for m in [0u16, 1, 2, 3] {
        for (j, sc) in scripts.iter().enumerate() {
            for (w0, ended, ans) in [
                (0u16, 1u32, 4u32),
                (1, 0, 0),
                (2, 1, 0xffff_fff0),
                (0, 0, 2),
                (0, 1, 0xffff_fff0),
                // MsEffectSetMotion (the sixth answer) 0
                (0, 1, 0xffff_fffb),
            ] {
                for (i, (op, arg)) in (0u32..).zip(sc.iter()) {
                    t.set_u32(entries + 8 + 8 * i, *op as u32);
                    t.set_u32(entries + 12 + 8 * i, *arg as u32);
                }
                t.set_u32(entries + 8 + 8 * sc.len() as u32, 0x77);
                t.set_u32(entries + 12 + 8 * sc.len() as u32, 0x0f);
                t.set_u32(script, entries);
                t.set_u32(pos, 0);
                t.set_u32(done, u32::from(j == 8 && w0 == 2));
                t.set_u16(wait, w0);
                t.set_u16(wait + 2, 0x10);
                t.set_u16(mode, m);
                t.set_u32(chr, 0x00c4_0000);
                t.set_u32(obj, 0x00b0_0000);
                t.set_u32(index, [u32::MAX, 1, 2, 0][j % 4]);
                for k in 0..13 {
                    t.set_u32(answers + 4 * k, ans.wrapping_add(k));
                }
                // MsEffectCheckMotion / Ch_GetWaitMotion: ended or not
                t.set_u32(answers, ended);
                t.set_u32(answers + 4, ended ^ 1);
                t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
                    Ret::eax(s3_motion::motion_step(g, img, fr.at))
                });
            }
        }
    }
}

/// `magic_0003` 0x10005db0: model nodes for no, one and three models,
/// under no parent and under one.
#[test]
fn model_nodes() {
    let Some(mut t) = twin(3) else { return };
    let f = t.at(0x5db0);
    let (grp, node, parent, list) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x2000,
        SCRATCH + 0x200,
    );
    t.write(list, &[4, 0x81, 7, 0xff]);
    t.stub(218, move |m, a| {
        m.w32(a[1], m.r32(SCRATCH + 0x300));
        list
    });
    log_slot(&mut t, 413, 1, 0, 0, 0x4130);
    log_slot(&mut t, 215, 1, 0, 1, 0x2150);
    log_slot(&mut t, 14, 4, 0, 2, 0x14);
    t.stub_st0(333, |_, _| 0.625);
    for (n, p, flags) in [
        (0u32, 0u32, 0u8),
        (1, parent, 1),
        (3, parent, 0xfe),
        (3, 0, 3),
    ] {
        t.write(node, &[0x66; 0x400]);
        // junk under the entry: the record's flags byte must be written
        t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
        t.write(parent, &[0x77; 0x40]);
        t.set_u32(parent + 0x10, parent);
        t.set_u32(SCRATCH + 0x300, n);
        t.set_u32(grp, node);
        t.set_u32(grp + 4, p);
        t.set_u32(grp + 8, 0x00ac_0000);
        t.set_u32(grp + 0xc, 0x0c0c_0c0c);
        t.set_u32(grp + 0x10, 0x5a5a_5a00 | u32::from(flags));
        t.check(f, Call::this(grp), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(s3_summon::model_nodes(g, img, fr))
        });
    }
}

/// `magic_0003` 0x10006b70: the summon's frame before its phase, with
/// and without children under the root, with the flag set, and with the
/// done bit's slot call held back or made.
#[test]
fn summon_frame() {
    let Some(mut t) = twin(3) else { return };
    let f = t.at(0x6b70);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let bind = t.img.rel32_target(f + 0x2d);
    t.set_u32(t.img.word(bind + 0x7e), SCRATCH + 0x500);
    for (k, (s, n)) in [
        (723u32, 1usize),
        (724, 1),
        (230, 1),
        (253, 0),
        (214, 1),
        (1000, 1),
    ]
    .into_iter()
    .enumerate()
    {
        log_slot(&mut t, s, n, 0, k as u32, 0x0300_0000 + s);
    }
    // the children's method logs its cursor's node and, on the last
    // child, empties the root's list
    let first = w(&t, 0x38);
    let root0 = w(&t, 0x4e);
    t.stub(1000, move |m, a| {
        let k = m.r32(LOG + 0x7fc).wrapping_add(1);
        m.w32(LOG + 0x7fc, k);
        m.w32(LOG + 0x780 + 4 * (k % 16), m.r32(a[0]));
        if m.r32(m.r32(a[0]) + 8) == root0 {
            m.w32(first, root0);
        }
        0x1000
    });
    // the children's method +0x20: slot 1000 (a table word, so a slot)
    let vt = SCRATCH + 0x800;
    t.set_u32(magic::TABLE + 4 * 1000, magic::TRAPS + 16 * 1000);
    t.set_u32(vt + 0x20, magic::TRAPS + 16 * 1000);
    t.set_u32(w(&t, 0x18), SCRATCH + 0x400);
    let root = w(&t, 0x4e);
    let pass = w(&t, 0x32);
    let kid = |i: u32| SCRATCH + 0x1000 + 0x100 * i;
    for (phase, flag, kids, half) in [
        (3u32, 0u32, 0u32, 0u16),
        (4, 0, 2, 0),
        (5, 0, 0, 0),
        (5, 1, 2, 0),
        (5, 1, 0, 1),
        (6, 0, 1, 0),
    ] {
        t.set_u32(w(&t, 0x0b), phase);
        t.set_u32(w(&t, 0x40), flag);
        t.set_u32(w(&t, 0x89), 0x10 | (phase & 1));
        t.set_u32(w(&t, 0x23), 0x0023_0000 + phase);
        t.set_u16(w(&t, 0xef), half);
        t.set_u32(w(&t, 0xfd), 0x00fd_0000);
        t.set_u8(pass, 0x41);
        for i in 0..kids {
            t.write(kid(i), &[0x22; 0x40]);
            t.set_u32(kid(i), vt);
            t.set_u32(kid(i) + 8, if i + 1 == kids { root } else { kid(i + 1) });
        }
        t.set_u32(w(&t, 0x38), if kids == 0 { root } else { kid(0) });
        // the bone node holds still (its pass is the new one); the gated
        // ones move (no children)
        let bone = w(&t, 0x96);
        t.set_u32(bone + 0x10, bone);
        t.set_u8(bone + 0x1a, 0x42);
        for at in [w(&t, 0xb7), w(&t, 0xd7)] {
            t.set_u32(at + 0x10, at);
            t.set_u8(at + 0x1a, 0);
            t.set_u8(at + 0x1b, 0);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0162` 0x10007c10: the value step task waiting, stepping and
/// ending (with and without its callback, kept or given back).
#[test]
fn value_step() {
    let Some(mut t) = twin(162) else { return };
    let f = t.at(0x7c10);
    log_slot(&mut t, 543, 1, 0, 0, 0x543);
    t.set_u32(magic::TABLE + 4 * 1000, magic::TRAPS + 16 * 1000);
    log_slot(&mut t, 1000, 1, 1, 1, 0x1000);
    let (task, st) = (SCRATCH + 0x1000, SCRATCH + 0x2000);
    for (delay, left, total, cb, flags) in [
        (2i32, 3u32, 8u32, 0u32, 0u16),
        (0, 3, 8, 0, 0),
        (0, 0, 8, 0, 0),
        (0, 0, 8, magic::TRAPS + 16 * 1000, 1),
        (0, 0, 8, magic::TRAPS + 16 * 1000, 0x7ffe),
        (0, 0, 8, 0, 0xfffe),
        (-3, 9, 9, 0, 0xffff),
    ] {
        rest_node(&mut t, task);
        t.set_u32(task + 0x14, st);
        t.write(st, &[0x44; 0x20]);
        t.set_u32(st, cb);
        t.set_u32(st + 4, 0xffff_ff80);
        t.set_u32(st + 8, 0x7f);
        t.set_u32(st + 0xc, delay.cast_unsigned());
        t.set_u32(st + 0x10, left);
        t.set_u32(st + 0x14, total);
        t.set_u16(st + 0x18, flags);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0052` 0x10002670: the task started on an empty and a filled
/// task list.
#[test]
fn task_start() {
    let Some(mut t) = twin(52) else { return };
    let f = t.at(0x2670);
    let take = t.img.rel32_target(f + 0x6f);
    let (count, free) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
    let (head, tail) = (t.img.word(f + 0x75), t.img.word(f + 0x8c));
    let pool = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    for filled in [false, true, false] {
        for i in 1..=2u32 {
            t.write(pool(i), &[0x5a; 0x1c]);
            t.set_u32(pool(i), pool(3 - i));
            t.set_u32(pool(i) + 4, pool(3 - i));
        }
        t.set_u32(free, pool(1));
        t.set_u16(count, 2);
        // the state block's words hold old values the start clears
        for op in [
            0x14u32, 0x1e, 0x28, 0x32, 0x3c, 0x46, 0x50, 0x5a, 0x60, 0x6a,
        ] {
            t.set_u32(t.img.word(f + op), 0x5a5a_5a5a);
        }
        if filled {
            t.write(pool(5), &[0x22; 0x1c]);
            t.set_u32(head, pool(5));
            t.set_u32(tail, pool(5));
        } else {
            t.set_u32(head, 0);
            t.set_u32(tail, 0);
        }
        t.check(
            f,
            Call::default(),
            &[0x00a5_0000 + u32::from(filled)],
            Eax::Checked,
            |g, img, fr| Ret::eax(s3_misc::task_start(g, img, fr)),
        );
    }
}
