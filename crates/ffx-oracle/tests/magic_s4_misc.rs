//! The shared-4 list's functions without a family (`ffx_magic::s4_misc`)
//! against the DLLs they were ported from. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

mod magic;
#[path = "magic/sa.rs"]
mod sa;

use ffx_magic::{Ret, Xmm, s4_fam446, s4_misc};
use magic::{Call, Eax, SCRATCH, Twin};
use sa::rec;

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

fn set_f(t: &mut Twin, a: u32, x: f32) {
    t.set_u32(a, x.to_bits());
}

/// A log in scratch memory (compared like the rest): its next free word
/// is kept at `LOG`.
const LOG: u32 = SCRATCH + 0x3_8000;

/// Appends `words` to the log.
fn log(m: &mut dyn magic::StubMem, words: &[u32]) {
    let mut at = m.r32(LOG);
    if at == 0 {
        at = LOG + 4;
    }
    for &x in words {
        m.w32(at, x);
        at += 4;
    }
    m.w32(LOG, at);
}

/// The sprites' frame: paused, no records, and records in every state
/// (dead, waiting, damped, scaled over frames, rolled past both limits,
/// copied, colour faded or plain, a held life, a dying one), drawn by
/// handle (known or not).
#[test]
#[allow(clippy::too_many_lines)] // one set-up per record field
fn sprites_run() {
    let Some(mut t) = twin(53) else { return };
    let at = t.at(0x2520);
    let list = SCRATCH + 0x4000;
    let table = SCRATCH + 0x8000;
    let lock = SCRATCH + 0x9000;
    t.set_u32(t.img.word(at + 0x34), list);
    t.set_u32(t.img.word(at + 0x3a5), lock);
    let pause = SCRATCH + 0x9100;
    rec(&mut t, 561, 0, move |m, _| m.r32(pause));
    for s in [723u32, 724, 235] {
        rec(&mut t, s, 1, move |m, a| {
            for i in 0..16 {
                let v = (i as f32 + 1.0) * if s == 235 { 1.0 } else { 0.5 };
                m.w32(a[0] + 4 * i, v.to_bits());
            }
            s
        });
    }
    rec(&mut t, 330, 3, |m, a| {
        // the scale matrix's diagonal it rolls, and the roll
        let d = [m.r32(a[1]), m.r32(a[1] + 0x14), m.r32(a[1] + 0x28), a[2]];
        log(m, &d);
        m.w32(a[0], d[0] ^ d[1] ^ d[2]);
        m.w32(a[0] + 0x38, a[2]);
        0x330
    });
    rec(&mut t, 229, 3, |m, a| {
        let p = [m.r32(a[2]), m.r32(a[2] + 4), m.r32(a[2] + 8)];
        log(m, &p);
        m.w32(a[0], p[0] ^ p[1] ^ p[2]);
        0x229
    });
    rec(&mut t, 230, 2, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, m.r32(a[1] + 4 * i));
        }
        0x230
    });
    // the frame table: count 3, frames of 8 bytes from +0x10 (record
    // offset, time, flags), the records behind it
    rec(&mut t, 941, 1, move |_, _| table);
    t.write(table, &[0; 0x100]);
    t.set_u16(table + 6, 3);
    for k in 0..3u32 {
        let e = table + 0x10 + 8 * k;
        t.set_u16(e, (0x80 + 0x10 * k) as u16);
        t.set_u16(e + 2, 0x300);
        t.set_u8(e + 4, if k == 2 { 0x80 } else { 0 });
        let r = table + 0x80 + 0x10 * k;
        t.set_u32(r + 4, 4 * k);
        t.set_u8(r + 4 * k + 4, 0x31 + k as u8);
    }
    rec(&mut t, 772, 2, |_, a| 0x7720_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    rec(&mut t, 773, 4, move |m, a| {
        m.w32(SCRATCH + 0x9200 + 4 * (a[3] & 7), m.r32(lock));
        (a[3] >> 1) & 1
    });
    for s in [774u32, 775, 778] {
        rec(&mut t, s, if s == 775 { 2 } else { 5 }, move |m, a| {
            m.w32(SCRATCH + 0x9300 + 4 * (s - 774), m.r32(lock));
            // the colour behind the pointer
            if s != 775 {
                let c = m.r32(a[3]);
                log(m, &[s, c]);
            }
            s
        });
    }
    rec(&mut t, 776, 1, |_, a| a[0] << 4);
    rec(&mut t, 777, 1, |m, a| {
        // the copied matrix (its `z` moved), when there is one
        if a[0] != 0 {
            let z = [m.r32(a[0] + 0x38), m.r32(a[0] + 0x30)];
            log(m, &z);
        }
        0x0ff0 ^ a[0]
    });
    let k1 = f32::from_bits(t.u32(t.img.word(at + 0x199)));
    let hi = f32::from_bits(t.u32(t.img.word(at + 0x1e0)));
    let lo = f32::from_bits(t.u32(t.img.word(at + 0x1f0)));
    for (case, (paused, count)) in [(1u32, 2i32), (0, 0), (0, 6), (0, 6), (0, 6)]
        .into_iter()
        .enumerate()
    {
        t.set_u32(pause, paused);
        t.set_u32(LOG, 0);
        t.set_u32(t.img.word(at + 0x90), count.cast_unsigned());
        t.set_u32(lock, 0x77);
        for r in 0..6u32 {
            let rec0 = list + 0x74 * r;
            let b = rec0 + 0x28;
            let v = case as u32 * 6 + r;
            t.write(rec0, &[0x11; 0x74]);
            for j in 0..11 {
                set_f(&mut t, rec0 + 4 * j, (j as f32 - 4.5) * 0.75 + v as f32);
            }
            t.set_u32(b + 0x44, u32::from(r != 1));
            t.set_u32(b + 0x38, if r == 2 && case == 2 { 3 } else { 0 });
            set_f(&mut t, b + 4, 0.5);
            t.set_u8(b + 0x48, (v % 4) as u8);
            set_f(&mut t, b + 0x14, 2.0 + r as f32);
            set_f(&mut t, b + 0x18, -1.5);
            t.set_u32(b + 0x30, [0, 4, 4, 1, 0, 2][r as usize]);
            t.set_u32(b + 0x34, [0, 0, 4, 1, 0, 1][r as usize]);
            let roll = [0.5, hi + 0.25, lo - 0.25, f32::NAN, hi, lo][r as usize];
            set_f(&mut t, b - 0xc, roll);
            set_f(&mut t, b + 0x20, if case == 4 { 0.0 } else { 0.125 * k1 });
            set_f(&mut t, b + 0x1c, 3.0);
            t.set_u32(b + 0x28, [0, 5, 1, 2, 0, 3][r as usize]);
            t.set_u32(b + 0x2c, [0, 2, 0, 0xffff_fffe, 0, 3][r as usize]);
            t.write(b + 0x3c, &[10, 200, 0, 255, 250, 3, 128, 1]);
            // the animation cursor at +0x8
            t.set_u32(b + 8, 0x0a00 + r);
            t.set_u32(b + 0xc, u32::from(!v.is_multiple_of(3)));
            t.set_u16(b + 0x10, (v % 3) as u16);
            t.set_u16(b + 0x12, if r == 3 { 0x100 } else { 0x500 });
            t.set_u32(b + 0x24, [0xffff_ffff, 2, 1, 0, 0xffff_ffff, 5][r as usize]);
        }
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            s4_misc::sprites_run(g, img, fr)
        });
    }
}

/// The object start with a part index (internal: `edx` = the object,
/// `xmm3` the scale, the stack `(index, owner, -, owner_b)`): appended to
/// an empty list, to a list with a tail, and with the pool empty.
#[test]
fn start_object_at() {
    for (n, rva, kind) in [
        (18u16, 0x3a10u32, 0usize),
        (446, 0x8090, 0),
        (54, 0x1b80, 1),
        (92, 0x27f0, 2),
        (162, 0x96a0, 3),
        (55, 0x3680, 4),
    ] {
        let Some(mut t) = twin(n) else { return };
        let at = t.at(rva);
        let (pool_op, tail_op) = [
            (0x9d, 0xa8),
            (0x99, 0xa4),
            (0x9c, 0xa7),
            (0x96, 0xa1),
            (0x95, 0xa0),
        ][kind];
        t.set_u32(t.img.word(at + 0x04), SCRATCH + 0x200);
        t.set_u32(t.img.word(at + 0x5e), SCRATCH + 0x300);
        t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
        for i in 0..8 {
            t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
        }
        t.stub(941, |m, a| {
            m.w32(SCRATCH + 0x900, a[0]);
            SCRATCH + 0x400
        });
        t.stub(714, |m, a| {
            for (i, w) in (0u32..).zip(&a[..5]) {
                m.w32(SCRATCH + 0x910 + 4 * i, *w);
            }
            0x714
        });
        let pa = t.img.rel32_target(at + pool_op);
        let free = t.img.word(pa + 0x04);
        // the pool's nodes: free (payload 0), their links old words
        let nodes = t.img.word(pa + 0x27) - 8;
        for i in 0..8 {
            t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
            t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
            t.set_u32(nodes + 12 * i + 8, 0);
        }
        let tail = t.img.word(at + tail_op);
        let obj = SCRATCH + 0x1000;
        for (k, (count, scale, tl)) in [
            (3u16, 0.5f32, 0u32),
            (2, -2.0, SCRATCH + 0x600),
            (0, 1.0, 0),
            (1, f32::NAN, SCRATCH + 0x700),
        ]
        .into_iter()
        .enumerate()
        {
            t.write(obj, &[0x5a; 0x130]);
            t.set_u16(free, count);
            t.set_u32(tail, tl);
            let c = Call {
                edx: obj,
                ecx: 0x0bad_0ecc,
                ..Call::default()
            }
            .with_xmm(&[Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(scale)]);
            let args = [
                k as u32 * 2 + 1,
                0x0a00 + k as u32,
                0xdead,
                0x0b00 + k as u32,
            ];
            t.check(at, c, &args, Eax::Checked, |g, img, fr| {
                let (obj, scale, index) = (fr.edx, fr.xmm_f32(3), fr.arg(g, 0));
                Ret::eax(if kind == 4 {
                    s4_misc::start_object_anim_first(g, img, fr.at, obj, scale, fr.arg(g, 1))
                } else if kind == 1 {
                    s4_misc::start_object_anim(g, img, fr.at, obj, scale, index, fr.arg(g, 1))
                } else if kind == 3 {
                    s4_misc::start_object_plain(g, img, fr.at, obj, scale, index)
                } else if kind == 2 {
                    let owners = (fr.arg(g, 2), fr.arg(g, 3));
                    let owner = fr.arg(g, 1);
                    s4_misc::start_object_owners(g, img, fr.at, obj, scale, index, owner, owners)
                } else {
                    s4_misc::start_object_at(
                        g,
                        img,
                        fr.at,
                        obj,
                        scale,
                        index,
                        fr.arg(g, 1),
                        fr.arg(g, 3),
                    )
                })
            });
        }
    }
}

/// A node at rest for the pool's give-back: its own links, no chain.
fn rest_node(t: &mut Twin, n: u32) {
    t.write(n, &[0x5a; 0x1c]);
    t.set_u32(n, n);
    t.set_u32(n + 4, n);
    t.set_u32(n + 8, n);
    t.set_u32(n + 0xc, 0);
}

/// `magic_0162` 0x10007d10: the shake task waiting, running (each flag,
/// odd and even time left, the global maxima beaten or not) and ending
/// (with and without its callback, kept or given back).
#[test]
fn shake_task() {
    let Some(mut t) = twin(162) else { return };
    let f = t.at(0x7d10);
    rec(&mut t, 314, 1, |_, a| 0x0314_0000 ^ (a[0] >> 16));
    t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * -1.25 + 0.1);
    rec(&mut t, 0, 1, |m, a| {
        m.w32(SCRATCH + 0x3000, a[0]);
        0x0c0b
    });
    let (task, st) = (SCRATCH + 0x1000, SCRATCH + 0x2000);
    let gx = t.img.word(f + 0x13c);
    let gy = t.img.word(f + 0x168);
    for (delay, left, flags, callback, step) in [
        (3i32, 4u32, 0u16, 0u32, 0.1f32),
        (0, 4, 0, 0, 0.1),
        (0, 5, 1, 0, 0.2),
        (0, 6, 2, 0, -0.3),
        (0, 3, 3, 0, 0.05),
        (0, 9, 3, 0, f32::NAN),
        (0, 1, 0, 0, 0.1),
        (0, 0, 4, magic::TRAPS, 0.1),
        (0, 1, 0, magic::TRAPS, 0.1),
        (-1, 0xffff_fff0, 5, 0, 0.1),
    ] {
        for (mx, my) in [
            (0.0f32, 0.0f32),
            (-1.0e6, 1.0e6),
            (f32::NAN, -0.01),
            (2.0, -2.0),
        ] {
            rest_node(&mut t, task);
            t.set_u32(task + 0x14, st);
            t.write(st, &[0x44; 0x30]);
            t.set_u32(st, callback);
            for (i, v) in [2.0f32, -3.5, 0.75, 1.5, 4.0, -0.25].iter().enumerate() {
                set_f(&mut t, st + 4 + 4 * i as u32, *v);
            }
            set_f(&mut t, st + 0x1c, 0.25);
            set_f(&mut t, st + 0x20, step);
            t.set_u32(st + 0x24, delay.cast_unsigned());
            t.set_u32(st + 0x28, left);
            t.set_u16(
                st + 0x2c,
                flags
                    | if mx.to_bits() == 2.0f32.to_bits() {
                        0x8000
                    } else {
                        0
                    },
            );
            set_f(&mut t, gx, mx);
            set_f(&mut t, gy, my);
            let c = Call {
                ecx: 0x0bad_0ecc,
                ..Call::default()
            }
            .with_xmm(&[Xmm::f32(1.0), Xmm::f32(2.0), Xmm::f32(0.0), Xmm::f32(4.0)]);
            t.check_entry(f, c, &[task], Eax::Checked);
        }
    }
}

/// Every copy of [`s4_misc::follow_task`] (census2, 2026-11-01: the
/// copies calling `s4_small::turn_point` are the same function).
const FOLLOW_TASKS: &[(u16, u32)] = &[
    (52, 0x2730),
    (53, 0x2cd0),
    (55, 0x2ef0),
    (64, 0x1b50),
    (65, 0x2310),
    (67, 0x1ef0),
    (149, 0x1960),
    (163, 0x28f0),
    (199, 0x2740),
    (200, 0x26f0),
    (243, 0x2dc0),
    (245, 0x2010),
    (247, 0x20f0),
    (248, 0x1f30),
    (249, 0x34f0),
    (250, 0x28e0),
    (251, 0x2af0),
    (269, 0x35c0),
    (275, 0x1990),
    (299, 0x28d0),
    (303, 0x4a80),
    (318, 0x1e10),
    (325, 0x2590),
    (354, 0x26b0),
    (380, 0x38c0),
    (385, 0x22b0),
    (390, 0x4080),
    (394, 0x35f0),
    (447, 0x2e20),
    (472, 0x1960),
    (475, 0x22e0),
    (508, 0x2eb0),
    (562, 0x3650),
    (659, 0x20f0),
    (660, 0x18b0),
];

/// `magic_0052` 0x10002730: the follow task in each mode (no mode, on a
/// target with and without the turned offset, on the actors' mean), its
/// script waiting, ending with the give-back flag and without it.
#[test]
#[allow(clippy::too_many_lines)] // one set-up per callee
fn follow_task() {
    for &(n, rva) in FOLLOW_TASKS {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        assert_eq!(
            ffx_magic::bind(&t.img, f).map(|i| ffx_magic::PORTS[i].0),
            Some("follow task"),
            "magic_{n:04}"
        );
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let map = SCRATCH + 0x5000;
        let chrs = w(&t, 0x4a);
        t.write(map, &[2, 0, 1, 3]);
        t.set_u32(w(&t, 0x3f), map);
        t.set_u32(w(&t, 0x38), 0x0c0c_0001);
        for i in 0..4 {
            t.set_u32(chrs + 4 * i, 0x0c0c_0100 + i);
        }
        let angle = t.img.rel32_target(f + 0x83);
        t.set_u32(t.img.word(angle + 0x11), map);
        let mean = t.img.rel32_target(f + 0x29);
        t.set_u32(t.img.word(mean + 0x4d), map);
        let mean_n = t.img.word(mean + 0x11);
        rec(&mut t, 262, 4, |m, a| {
            for i in 1..4usize {
                let v = (a[0] & 0xff) as f32 * 0.5 + i as f32;
                m.w32(a[i], v.to_bits());
            }
            0x262
        });
        rec(&mut t, 333, 1, |_, _| 0x333);
        t.stub_st0(333, |_, a| f64::from(a[0] & 0xf) * 0.25);
        for s in [235u32, 328, 329, 229, 248] {
            rec(&mut t, s, 1, move |m, a| {
                m.w32(a[0], m.r32(a[0]) ^ s);
                s
            });
        }
        let (task, st) = (SCRATCH + 0x1000, SCRATCH + 0x2000);
        let cmds = SCRATCH + 0x3000;
        t.set_u32(cmds, 3);
        t.set_u32(cmds + 4, 9);
        t.set_u32(cmds + 12, 5);
        for (mode, target, flags, cmd, count) in [
            (0u32, 0u32, 0u16, 0u32, 0u32),
            (7, 0, 0, 0, 0),
            (1, 0xffff_ffff, 0, 0, 0),
            (1, 2, 1, 1, 0),
            (1, 1, 0x4001, 1, 0),
            (1, 3, 0x4001, 1, 2),
            (2, 0, 0x8000, 0, 0),
            (2, 0, 0, 1, 3),
            (2, 0, 0x4000, 1, 1),
        ] {
            for n in [1u32, 3] {
                rest_node(&mut t, task);
                t.set_u32(task + 0x14, st);
                t.write(st, &[0x22; 0x50]);
                for i in 0..4 {
                    set_f(&mut t, st + 4 * i, 1.5 * i as f32 - 2.0);
                }
                // the script object at +0x10: its list empty (no current
                // node; `count` items), the commands, the index, no wait
                t.write(st + 0x20, &[0; 0x10]);
                t.set_u32(st + 0x2c, count);
                t.set_u32(st + 0x34, cmds);
                t.set_u32(st + 0x38, cmd);
                t.set_u32(st + 0x3c, 0);
                t.set_u16(st + 0x40, flags);
                t.set_u32(st + 0x44, mode);
                t.set_u32(st + 0x48, target);
                t.set_u16(st + 0x4c, if n == 3 { 0x8123 } else { 0x0123 });
                t.set_u32(mean_n, n);
                t.check_entry(f, Call::default(), &[task], Eax::Checked);
            }
        }
    }
}

/// `magic_0162` 0x10005b00 and 0x10005c90: angle tracks stepping (plain
/// and through the curve, angles past a turn) and on their last step.
#[test]
fn angle_steps() {
    let Some(mut t) = twin(162) else { return };
    let (track, keys, out, ctl, base) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x300,
        SCRATCH + 0x400,
    );
    for i in 0..30u32 {
        let x = (i as f32 * 0.7).sin() * 4.0 + i as f32 * 0.9;
        set_f(&mut t, keys + 4 * i, x);
    }
    for (i, c) in [0.5f32, 1.0, -2.0, 3.5, 4.0].iter().enumerate() {
        set_f(&mut t, ctl + 4 * i as u32, *c);
    }
    for (i, c) in [10.0f32, -20.0, 0.125, 7.0].iter().enumerate() {
        set_f(&mut t, base + 4 * i as u32, *c);
    }
    for s in [235u32, 330, 328, 329] {
        rec(&mut t, s, 1, move |m, a| {
            let v = m.r32(a[0] + 4 * (s % 16)) ^ s;
            m.w32(a[0] + 4 * (s % 16), v);
            s
        });
    }
    rec(&mut t, 229, 4, |m, a| {
        // the vector it is given, turned
        let (x, z) = (m.r32(a[2]), m.r32(a[2] + 8));
        m.w32(a[0], z ^ m.r32(a[1] + 0x2c));
        m.w32(a[0] + 8, x.wrapping_add(m.r32(a[1] + 0x1c)));
        0x229
    });
    for (rva, curved) in [(0x5b00u32, false), (0x5c90, true)] {
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
            t.set_u32(track + 0x24, base);
            t.set_u32(track + 0x28, ctl);
            set_f(&mut t, track + 0x2c, tm);
            set_f(&mut t, track + 0x30, step);
            t.set_u32(track + 0x34, left);
            t.set_u32(track + 0x38, 9);
            t.check(
                t.at(rva),
                Call {
                    ecx: track,
                    edx: out,
                    ..Call::default()
                },
                &[],
                Eax::Checked,
                |g, img, fr| {
                    Ret::eax(if curved {
                        s4_misc::angle_step_curved(g, img, fr)
                    } else {
                        s4_misc::angle_step(g, img, fr)
                    })
                },
            );
        }
    }
}

/// `magic_0162` 0x100066e0: the motion script done, waiting, waiting for
/// the motion (model and character), and running each command and each
/// following argument in every mode.
#[test]
#[allow(clippy::too_many_lines)] // one set-up per command
fn motion_script() {
    let Some(mut t) = twin(162) else { return };
    let f = t.at(0x66e0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let map = SCRATCH + 0x5004;
    t.write(map - 4, &[1, 3, 2, 1, 2, 3, 1, 0, 1, 2, 3]);
    t.set_u32(w(&t, 0xb8), map);
    t.set_u32(w(&t, 0xb1), 0x0c0c_0001);
    for i in 0..4 {
        t.set_u32(w(&t, 0xc3) + 4 * i, 0x0c0c_0100 + i);
    }
    let speed = t.img.rel32_target(f + 0x8e);
    t.set_u32(t.img.word(speed + 0x0a), map);
    t.set_u32(t.img.word(f + 0x10e), 0x0529_0529);
    rec(&mut t, 411, 1, |_, a| a[0] & 1);
    rec(&mut t, 20, 1, |_, a| a[0] & 1);
    rec(&mut t, 228, 2, |_, a| {
        if a[1] == 0x10 {
            (a[0] & 2) * 3
        } else {
            a[1].wrapping_sub(0x55)
        }
    });
    for (s, n) in [
        (23u32, 2usize),
        (3, 1),
        (529, 1),
        (30, 1),
        (36, 3),
        (37, 4),
        (213, 2),
    ] {
        rec(&mut t, s, n, move |_, _| s);
    }
    rec(&mut t, 18, 1, |_, a| a[0].wrapping_sub(0x0c0c_0002));
    for s in [530u32, 249] {
        rec(&mut t, s, 2, move |_, _| s);
    }
    let (sc, cmds) = (SCRATCH + 0x100, SCRATCH + 0x1000);
    // (done, wait, model, chr)
    let states: [(u32, u16, u32, u32); 6] = [
        (1, 0, 0x5000, 0x0c0c_0003),
        (0, 3, 0x5000, 0x0c0c_0003),
        (0, 1, 0x5000, 0x0c0c_0003),
        (0, 0, 0x5001, 0x0c0c_0002),
        (0, 0, 0x5000, 0x0c0c_0003),
        (2, 0, 0x5001, 0x0c0c_0002),
    ];
    let lists: [&[(i32, i32)]; 6] = [
        &[(-2, 0x40), (-3, 0), (-4, 0), (0x55, -7)],
        &[(-4, 0), (0x56, -7)],
        &[(-5, -6), (0, 0)],
        &[(-1, -5)],
        &[(0x57, -3), (0, 0)],
        &[(0x55, -7)],
    ];
    for mode in [0u16, 1, 2, 3] {
        for (k, &(done, wait, model, chr)) in states.iter().enumerate() {
            for (li, list) in lists.iter().enumerate() {
                for arg in [
                    None,
                    Some(-4),
                    Some(-2),
                    Some(-1),
                    Some(5),
                    Some(-8),
                    Some(-6),
                ] {
                    if k < 3 && (li > 0 || arg.is_some()) {
                        continue;
                    }
                    t.write(cmds, &[0x77; 0x40]);
                    for (i, &(op, a)) in list.iter().enumerate() {
                        let a = if i + 1 == list.len() {
                            arg.unwrap_or(a)
                        } else {
                            a
                        };
                        t.set_u32(cmds + 8 * i as u32, op.cast_unsigned());
                        t.set_u32(cmds + 8 * i as u32 + 4, a.cast_unsigned());
                    }
                    t.write(sc, &[0x5a; 0x20]);
                    t.set_u32(sc, cmds);
                    t.set_u32(sc + 4, [u32::MAX, 1, 2, 3][li % 4]);
                    t.set_u32(sc + 8, model);
                    t.set_u32(sc + 0xc, chr);
                    t.set_u32(sc + 0x10, u32::MAX);
                    t.set_u32(sc + 0x14, done);
                    t.set_u16(sc + 0x18, wait);
                    t.set_u16(sc + 0x1c, mode);
                    t.check(f, Call::this(sc), &[], Eax::Checked, |g, img, fr| {
                        Ret::eax(s4_misc::motion_script(g, img, fr.at, fr.ecx))
                    });
                }
            }
        }
    }
}

/// `magic_0018` 0x10002580, and `magic_0009`'s copy (0x100024f0, its
/// operands earlier) in two of its DLLs: the trail with no sprites, a
/// few, more than the ring holds, zero-length and NaN segments, spacing
/// below and past the segments, drawn by known and new handles, alpha 0
/// and not.
#[test]
fn trail_draw() {
    for (dll, rva, lock_op, x_op) in [
        (18u16, 0x2580u32, 0x46eu32, 0x52bu32),
        (9, 0x24f0, 0x463, 0x520),
        (491, 0x1100, 0x463, 0x520),
    ] {
        let Some(mut t) = twin(dll) else { return };
        trail_draw_cases(&mut t, rva, lock_op, x_op);
    }
}

#[allow(clippy::too_many_lines)] // one set-up per field
fn trail_draw_cases(t: &mut Twin, rva: u32, lock_op: u32, x_op: u32) {
    let f = t.at(rva);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let lock = SCRATCH + 0x9000;
    t.set_u32(w(t, lock_op), lock);
    t.set_u32(w(t, x_op), 0x0cc0_0cc0);
    let table = SCRATCH + 0x8000;
    rec(t, 941, 1, move |_, _| table);
    t.write(table, &[0; 0x100]);
    t.set_u16(table + 6, 3);
    for k in 0..3u32 {
        let e = table + 0x10 + 8 * k;
        t.set_u16(e, (0x80 + 0x10 * k) as u16);
        t.set_u16(e + 2, 0x300);
        let r = table + 0x80 + 0x10 * k;
        t.set_u32(r + 4, 4 * k);
        t.set_u8(r + 4 * k + 4, 0x31 + k as u8);
    }
    for s in [723u32, 724, 235] {
        rec(t, s, 1, move |m, a| {
            for i in 0..16 {
                let v = (i as f32 + 1.0) * if s == 235 { 1.0 } else { 0.5 };
                m.w32(a[0] + 4 * i, v.to_bits());
            }
            s
        });
    }
    let fv = |m: &dyn magic::StubMem, a: u32| f32::from_bits(m.r32(a));
    rec(t, 234, 3, move |m, a| {
        for i in 0..3 {
            let v = fv(m, a[1] + 4 * i) - fv(m, a[2] + 4 * i);
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x234
    });
    rec(t, 248, 3, move |m, a| {
        for i in 0..3 {
            let v = fv(m, a[1] + 4 * i) + fv(m, a[2] + 4 * i);
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x248
    });
    rec(t, 229, 3, |m, a| {
        // the scaled matrix and the point it places
        let mut v = [0u32; 13];
        for (i, x) in v.iter_mut().enumerate().take(12) {
            *x = m.r32(a[0] - 0x30 + 4 * i as u32);
        }
        v[12] = m.r32(a[2] + 0xc);
        log(m, &v);
        let p = [m.r32(a[2]), m.r32(a[2] + 4), m.r32(a[2] + 8)];
        log(m, &p);
        0x229
    });
    rec(t, 772, 2, |_, a| 0x7720_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    rec(t, 773, 4, move |m, a| {
        m.w32(SCRATCH + 0x9200 + 4 * (a[3] & 7), m.r32(lock));
        (a[3] >> 1) & 1
    });
    for s in [774u32, 775, 778] {
        rec(t, s, if s == 775 { 2 } else { 5 }, move |m, a| {
            m.w32(SCRATCH + 0x9300 + 4 * (s - 774), m.r32(lock));
            if s != 775 {
                let c = m.r32(a[3]);
                log(m, &[s, c]);
            }
            s
        });
    }
    rec(t, 776, 1, |_, a| a[0] << 4);
    rec(t, 777, 1, |_, a| 0x0ff0 ^ a[0]);
    let (obj, rows, point) = (SCRATCH + 0x1000, SCRATCH + 0x2000, SCRATCH + 0x2100);
    for i in 0..12u32 {
        set_f(t, rows + 4 * i, 0.5 + i as f32 * 0.25);
    }
    for (case, (n, spacing, newest, alpha)) in [
        (0u32, 1.0f32, 3u32, 0x80u8),
        (1, 0.5, 0, 0x80),
        (5, 0.75, 31, 0x80),
        (6, 2.5, 7, 0),
        (40, 0.1, 12, 0x40),
        (3, f32::NAN, 2, 0x10),
    ]
    .into_iter()
    .enumerate()
    {
        t.set_u32(LOG, 0);
        t.write(obj, &[0x33; 0x230]);
        t.set_u32(obj, 0x0a00 + case as u32);
        t.set_u32(obj + 4, 1);
        t.set_u16(obj + 8, case as u16 % 3);
        t.set_u16(obj + 0xa, 0x100);
        for k in 0..32u32 {
            // a spiral, with a repeated point (zero length) and a NaN
            let r = 1.0 + k as f32 * 0.3;
            let a = k as f32 * 0.7;
            let pt = if k == 5 {
                [1.0, 2.0, f32::NAN, 1.0]
            } else if k == 9 || k == 10 {
                [4.0, 4.0, 4.0, 1.0]
            } else {
                [r * a.cos(), k as f32 * 0.2, r * a.sin(), 1.0]
            };
            for (i, x) in (0u32..).zip(pt) {
                set_f(t, obj + 0xc + 16 * k + 4 * i, x);
            }
        }
        t.write(obj + 0x20c, &[10, 200, 0, alpha, 250, 3, 128, alpha / 2]);
        set_f(t, obj + 0x214, 1.5);
        set_f(t, obj + 0x218, 0.25);
        set_f(t, obj + 0x21c, spacing);
        t.set_u32(obj + 0x220, newest);
        t.set_u32(obj + 0x224, n);
        for (i, x) in [0.5f32, 9.0, -1.0, 7.0].into_iter().enumerate() {
            set_f(t, point + 4 * i as u32, x);
        }
        let c = Call {
            ecx: obj,
            edx: rows,
            ..Call::default()
        };
        t.check(f, c, &[point], Eax::Checked, |g, img, fr| {
            Ret::eax(s4_misc::trail_draw(g, img, fr))
        });
    }
}

/// The `magic_0446` family's effect starts against `magic_0446` (and
/// `magic_0565`'s copies where it has them): each start with the pool
/// free and empty, and the kinds of the table ones.
#[test]
fn fam446_starts() {
    let Some(mut t) = twin(446) else { return };
    let block = SCRATCH + 0x1_0000;
    let soa = t.img.rel32_target(t.at(0x2980) + 0x39);
    t.set_u32(t.img.word(soa + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(soa + 0x5e), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    for i in 0..0x30 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    t.stub(941, |m, a| {
        m.w32(SCRATCH + 0x900, a[0]);
        SCRATCH + 0x400
    });
    t.stub(714, |m, a| {
        let k = m.r32(SCRATCH + 0x90c);
        m.w32(SCRATCH + 0x90c, k + 1);
        for (i, w) in (0u32..).zip(&a[..5]) {
            m.w32(SCRATCH + 0x910 + 4 * i, *w ^ k);
        }
        0x714
    });
    rec(&mut t, 219, 2, |_, a| 0x0219_0000 ^ a[0]);
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..3 {
            m.w32(a[3] + 4 * i, a[0].wrapping_add(i));
        }
        0x237
    });
    t.set_u32(t.img.word(t.at(0x41f0) + 0x2c), 0x0c0c_0007);
    let pa = t.img.rel32_target(soa + 0x9d);
    let free = t.img.word(pa + 0x04);
    let nodes = t.img.word(pa + 0x27) - 8;
    let tail = t.img.word(soa + 0xa8);
    let task = SCRATCH + 0x800;
    for (rva, kinds) in [
        (0x2980u32, &[0u32, 1, 2][..]),
        (0x1530, &[0, 1, 2]),
        (0x2910, &[0]),
        (0x3050, &[0]),
        (0x4180, &[0]),
        (0x40d0, &[0]),
        (0x41f0, &[0]),
    ] {
        for &kind in kinds {
            for (count, tl) in [(3u16, 0u32), (0, SCRATCH + 0x600), (5, SCRATCH + 0x700)] {
                t.write(block, &vec![0x3c; 0xec00]);
                for i in 0..4 {
                    set_f(&mut t, block + 0x2f80 + 4 * i, 1.5 * i as f32);
                }
                t.set_u32(block + 0xebf0, 0x00c1_0c00 + kind);
                t.set_u32(block + 0x5c04, 0x77);
                for i in 0..8 {
                    t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
                    t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
                    t.set_u32(nodes + 12 * i + 8, 0);
                }
                t.set_u16(free, count);
                t.set_u32(tail, tl);
                t.set_u32(task + 0x2c, kind);
                t.set_u32(t.img.word(t.at(0x2980) + 0x15), block);
                let c = Call {
                    ecx: 0x0bad_0ecc,
                    ..Call::default()
                };
                t.check_entry(t.at(rva), c, &[task], Eax::Checked);
            }
        }
    }
}

/// The `magic_0446` family's child starts (a container in `ecx`, two
/// words at `edx`): each kind, the container's index -1 and not, the pool
/// free and empty.
#[test]
fn fam446_children() {
    let Some(mut t) = twin(446) else { return };
    let soa = t.img.rel32_target(t.at(0x21d0) + 0x2d);
    t.set_u32(t.img.word(soa + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(soa + 0x5e), SCRATCH + 0x300);
    for i in 0..0x30 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    t.stub(941, |_, _| SCRATCH + 0x400);
    t.stub(714, |m, a| {
        for (i, w) in (0u32..).zip(&a[..5]) {
            m.w32(SCRATCH + 0x910 + 4 * i, *w);
        }
        0x714
    });
    let kind_at = SCRATCH + 0x880;
    rec(&mut t, 641, 1, move |m, _| m.r32(kind_at));
    let map = SCRATCH + 0x5000;
    t.write(map, &[0x21, 0x22, 0x23, 0x24]);
    let pa = t.img.rel32_target(soa + 0x9d);
    let free = t.img.word(pa + 0x04);
    let (this, pair) = (SCRATCH + 0x1000, SCRATCH + 0x900);
    t.set_u32(pair, 0x0aaa_0001);
    t.set_u32(pair + 4, 0x0bbb_0002);
    for (rva, ga, gb) in [
        (0x21d0u32, 0x81u32, 0x91u32),
        (0x29e0, 0x82, 0x92),
        (0x30c0, 0x92, 0xa1),
    ] {
        let f = t.at(rva);
        t.set_u32(t.img.word(f + ga), 0x00a0_0000u32.wrapping_add(rva));
        t.set_u32(t.img.word(f + gb), map);
        for kind in [0u32, 1, 2] {
            for (index, count) in [(u32::MAX, 3u16), (2, 0)] {
                t.write(this, &[0x3c; 0x520]);
                for i in 0..4 {
                    set_f(&mut t, this + 0x270 + 4 * i, 2.5 * i as f32);
                }
                t.set_u32(this + 0x514, index);
                t.set_u16(free, count);
                t.set_u32(kind_at, kind);
                let c = Call {
                    ecx: this,
                    edx: pair,
                    ..Call::default()
                };
                t.check_entry(f, c, &[kind], Eax::Checked);
            }
        }
    }
}

/// A free ring of `n` nodes for the pool's take at `take` (its count and
/// head operands), 0x40 bytes apart from `at`.
fn free_ring(t: &mut Twin, take: u32, at: u32, n: u32) {
    let node = |i: u32| at + 0x40 * i;
    for i in 1..=n {
        t.write(node(i), &[0x5a; 0x1c]);
        t.set_u32(node(i), node(if i == 1 { n } else { i - 1 }));
        t.set_u32(node(i) + 4, node(if i == n { 1 } else { i + 1 }));
    }
    t.set_u32(t.img.word(take + 0x17), node(1));
    t.set_u16(t.img.word(take + 0x04), n as u16);
}

/// The `magic_0446` family's start-up and mode set-up: the task list empty
/// and not, each mode.
#[test]
fn fam446_start_up() {
    let Some(mut t) = twin(446) else { return };
    let su = t.at(0x49c0);
    let ms = t.at(0x2010);
    let take = t.img.rel32_target(su + 0x7c);
    let ps = t.img.rel32_target(su + 0x56);
    t.set_u32(t.img.word(ps + 0x04), SCRATCH + 0x800);
    rec(&mut t, 719, 3, |_, a| 0x7190_0000 ^ a[1]);
    for (s, n) in [
        (711u32, 3usize),
        (710, 4),
        (271, 1),
        (219, 2),
        (357, 3),
        (356, 3),
        (362, 4),
        (262, 4),
    ] {
        rec(&mut t, s, n, move |_, _| s);
    }
    rec(&mut t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, |_, _| 1.25);
    let head = t.img.word(su + 0x82);
    let tail = t.img.word(su + 0x96);
    let owner_res = SCRATCH + 0x880;
    t.set_u32(t.img.word(ms + 0x12), owner_res);
    t.set_u32(owner_res + 0x24, 0x0e5e_0001);
    for k in 0..2u32 {
        free_ring(&mut t, take, SCRATCH + 0x1000, 4);
        if k == 0 {
            t.set_u32(head, 0);
            t.set_u32(tail, 0);
        } else {
            t.write(SCRATCH + 0x1400, &[0x22; 0x1c]);
            t.set_u32(head, SCRATCH + 0x1400);
            t.set_u32(tail, SCRATCH + 0x1400);
        }
        t.check_entry(su, Call::default(), &[], Eax::Checked);
    }
    let block = t.u32(t.img.word(ms + 0x3));
    for mode in [0u32, 1, 2] {
        for k in 0..2u32 {
            free_ring(&mut t, take, SCRATCH + 0x1000, 2);
            if k == 0 {
                t.set_u32(head, 0);
                t.set_u32(tail, 0);
            } else {
                t.write(SCRATCH + 0x1400, &[0x22; 0x1c]);
                t.set_u32(head, SCRATCH + 0x1400);
                t.set_u32(tail, SCRATCH + 0x1400);
            }
            t.set_u32(block + 0x5c04, mode);
            let c = Call {
                eax: 0x00ea_0000 + mode,
                ..Call::default()
            };
            t.check_entry(ms, c, &[], Eax::Checked);
        }
    }
}

/// The `magic_0446` family's load step (busy, asking, waiting, loaded)
/// and mode object (no mode; the previous object listed or not).
#[test]
fn fam446_load_and_mode_object() {
    let Some(mut t) = twin(446) else { return };
    let ls = t.at(0x1240);
    let su = t.img.rel32_target(ls + 0x5b);
    let take = t.img.rel32_target(su + 0x7c);
    let ps = t.img.rel32_target(su + 0x56);
    t.set_u32(t.img.word(ps + 0x04), SCRATCH + 0x800);
    rec(&mut t, 719, 3, |_, a| 0x7190_0000 ^ a[1]);
    for (s, n) in [
        (711u32, 3usize),
        (710, 4),
        (271, 1),
        (219, 2),
        (357, 3),
        (356, 3),
        (362, 4),
        (262, 4),
        (226, 2),
        (713, 1),
    ] {
        rec(&mut t, s, n, move |_, _| s);
    }
    rec(&mut t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, |_, _| 1.25);
    let answers = SCRATCH + 0x8c0;
    rec(&mut t, 225, 1, move |m, _| m.r32(answers));
    rec(&mut t, 227, 1, move |m, _| m.r32(answers + 4));
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..3 {
            m.w32(a[3] + 4 * i, a[0].wrapping_add(i));
        }
        0x237
    });
    let asked = t.img.word(ls + 0x18);
    // the resource words the load step passes on
    t.set_u32(t.img.word(ls + 0x26), 0x0e5e_0026);
    t.set_u32(t.img.word(ls + 0x46), 0x0e5e_0046);
    // the start-up's first cleared word holds something first
    t.set_u32(t.img.word(su + 0x16), 0x1616);
    for (busy, state, there) in [
        (1u32, 0u32, 0u32),
        (0, 0, 0),
        (0, 1, 0xffff_ffff),
        (0, 1, 1),
        (0, 0, 2),
    ] {
        free_ring(&mut t, take, SCRATCH + 0x1000, 4);
        t.set_u32(answers, busy);
        t.set_u32(answers + 4, there);
        t.set_u32(asked, state);
        t.check_entry(ls, Call::default(), &[], Eax::Checked);
    }
    // the mode object, with the block the start-up set
    let mo = t.at(0x20c0);
    let soa = t.img.rel32_target(mo + 0x78);
    t.set_u32(t.img.word(soa + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(soa + 0x5e), SCRATCH + 0x300);
    t.stub(941, |_, _| SCRATCH + 0x400);
    t.stub(714, |m, a| {
        for (i, w) in (0u32..).zip(&a[..5]) {
            m.w32(SCRATCH + 0x910 + 4 * i, *w);
        }
        0x714
    });
    t.set_u32(t.img.word(mo + 0x29), SCRATCH + 0x8e0);
    t.set_u32(t.img.word(mo + 0x85), 0x0a0a_0085);
    let block = t.u32(t.img.word(mo + 0x3));
    let list = t.img.word(mo + 0x1b);
    let pa = t.img.rel32_target(soa + 0x9d);
    let free = t.img.word(pa + 0x04);
    let old = block + 0x130 * 3;
    for (mode, listed) in [(0u32, false), (1, false), (1, true), (2, true)] {
        t.set_u16(free, 2);
        t.set_u32(block + 0x5c04, mode);
        t.set_u32(block + 0x5bfc, old);
        t.set_u16(old + 0x124, 0xffff);
        let node = SCRATCH + 0x1800;
        t.write(node, &[0; 0x10]);
        t.set_u32(node + 8, old);
        t.write(list, &[0; 0x10]);
        if listed {
            t.set_u32(list, node);
            t.set_u32(list + 4, node);
            t.set_u32(list + 0xc, 1);
        }
        t.check_entry(mo, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0565`'s child starts with their sound: each kind, the sound on
/// and off, the container's index -1 and not.
#[test]
fn fam446_children_sound() {
    let Some(mut t) = twin(565) else { return };
    let soa = t.img.rel32_target(t.at(0x3d30) + 0x2d);
    t.set_u32(t.img.word(soa + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(soa + 0x5e), SCRATCH + 0x300);
    for i in 0..0x30 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    t.stub(941, |_, _| SCRATCH + 0x400);
    t.stub(714, |m, a| {
        for (i, w) in (0u32..).zip(&a[..5]) {
            m.w32(SCRATCH + 0x910 + 4 * i, *w);
        }
        0x714
    });
    rec(&mut t, 719, 3, |_, a| 0x7190_0000 ^ a[1] ^ a[2]);
    rec(&mut t, 393, 3, |m, a| {
        let x = m.r32(a[1]);
        m.w32(SCRATCH + 0x8f0, x);
        0x393
    });
    rec(&mut t, 306, 3, |_, a| 0x3060_0000 ^ a[2]);
    let map = SCRATCH + 0x5000;
    t.write(map, &[0x21, 0x22, 0x23, 0x24]);
    let pa = t.img.rel32_target(soa + 0x9d);
    let free = t.img.word(pa + 0x04);
    let (this, pair) = (SCRATCH + 0x1000, SCRATCH + 0x900);
    t.set_u32(pair, 0x0aaa_0001);
    t.set_u32(pair + 4, 0x0bbb_0002);
    for (rva, ga, gb) in [(0x3d30u32, 0x81u32, 0x88u32), (0x4640, 0x82, 0x89)] {
        let f = t.at(rva);
        t.set_u32(t.img.word(f + ga), 0x00a0_0000u32.wrapping_add(rva));
        t.set_u32(t.img.word(f + gb), map);
        for kind in [0u32, 1, 2] {
            for (index, count, quiet) in [(u32::MAX, 3u16, 0u32), (2, 0, 1), (1, 2, 0)] {
                t.write(this, &[0x3c; 0x520]);
                for i in 0..4 {
                    set_f(&mut t, this + 0x270 + 4 * i, 2.5 * i as f32);
                }
                t.set_u32(this + 0x514, index);
                t.set_u32(this + 0x518, quiet);
                t.set_u16(free, count);
                let c = Call {
                    ecx: this,
                    edx: pair,
                    ..Call::default()
                };
                t.check_entry(f, c, &[kind], Eax::Checked);
            }
        }
    }
}

/// `magic_0010` 0x10001cd0: the object start with the first part, the
/// pool free and empty.
#[test]
fn start_object_first() {
    let Some(mut t) = twin(10) else { return };
    let at = t.at(0x1cd0);
    t.set_u32(t.img.word(at + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(at + 0x5e), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x400, 0x1111);
    t.stub(941, |m, a| {
        m.w32(SCRATCH + 0x900, a[0]);
        SCRATCH + 0x400
    });
    t.stub(714, |m, a| {
        for (i, w) in (0u32..).zip(&a[..5]) {
            m.w32(SCRATCH + 0x910 + 4 * i, *w);
        }
        0x714
    });
    let pa = t.img.rel32_target(at + 0x9b);
    let free = t.img.word(pa + 0x04);
    let tail = t.img.word(at + 0xa6);
    let obj = SCRATCH + 0x1000;
    for (k, (count, tl)) in [(3u16, 0u32), (2, SCRATCH + 0x600), (0, 0)]
        .into_iter()
        .enumerate()
    {
        t.write(obj, &[0x5a; 0x130]);
        t.set_u16(free, count);
        t.set_u32(tail, tl);
        let c = Call {
            edx: obj,
            ecx: 0x0bad_0ecc,
            ..Call::default()
        }
        .with_xmm(&[Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(1.5)]);
        t.check(
            at,
            c,
            &[0xdead, 0x0a00 + k as u32],
            Eax::Checked,
            |g, img, fr| {
                Ret::eax(s4_misc::start_object_first(
                    g,
                    img,
                    fr.at,
                    fr.edx,
                    fr.xmm_f32(3),
                    fr.arg(g, 1),
                ))
            },
        );
    }
}

/// `magic_0162` 0x10006030: an actor's point in every mode, for the
/// caster and a listed actor.
#[test]
fn actor_point() {
    let Some(mut t) = twin(162) else { return };
    let f = t.at(0x6030);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let map = SCRATCH + 0x5000;
    t.write(map, &[2, 0, 1, 3]);
    t.set_u32(w(&t, 0x26), map);
    t.set_u32(w(&t, 0x1f), 0x0c0c_0001);
    for i in 0..4 {
        t.set_u32(w(&t, 0x31) + 4 * i, 0x0c0c_0100 + i);
    }
    let scale = t.img.rel32_target(f + 0x8c);
    t.set_u32(t.img.word(scale + 0x15), map);
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4usize {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32;
            m.w32(a[i], v.to_bits());
        }
        0x262
    });
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..3 {
            let v = (a[1] as f32) * 0.25 - i as f32;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x268
    });
    rec(&mut t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * -0.75);
    for s in [16u32, 364] {
        rec(&mut t, s, 1, move |_, _| s);
        t.stub_st0(s, move |_, a| f64::from(a[0] & 0xf) + f64::from(s) * 0.001);
    }
    let out = SCRATCH + 0x200;
    for index in [u32::MAX, 2] {
        for mode in 0u32..9 {
            t.write(out, &[0x44; 0x10]);
            let c = Call {
                ecx: index,
                edx: out,
                ..Call::default()
            };
            t.check(f, c, &[mode], Eax::Checked, |g, img, fr| {
                s4_misc::actor_point(g, img, fr)
            });
        }
    }
}

/// `magic_0171` 0x100035f0: the second sprites' frame, with records in
/// every state (dead, held, dying, copied, faded or plain, animated past
/// the end) drawn by known and new handles.
#[test]
#[allow(clippy::too_many_lines)] // one set-up per record field
fn sprites_run_b() {
    let Some(mut t) = twin(171) else { return };
    let at = t.at(0x35f0);
    let list = SCRATCH + 0x4000;
    let table = SCRATCH + 0x8000;
    let lock = SCRATCH + 0x9000;
    t.set_u32(t.img.word(at + 0x1f), list);
    let lp = t.img.rel32_target(at + 0x2b7);
    t.set_u32(t.img.word(lp + 0x4f), lock);
    for s in [723u32, 724, 235] {
        rec(&mut t, s, 1, move |m, a| {
            for i in 0..16 {
                let v = (i as f32 + 1.0) * if s == 235 { 1.0 } else { 0.5 };
                m.w32(a[0] + 4 * i, v.to_bits());
            }
            s
        });
    }
    rec(&mut t, 330, 3, |m, a| {
        let d = [m.r32(a[1]), m.r32(a[1] + 0x14), m.r32(a[1] + 0x28), a[2]];
        log(m, &d);
        m.w32(a[0] + 0x38, a[2]);
        0x330
    });
    rec(&mut t, 229, 3, |m, a| {
        let p = [m.r32(a[2]), m.r32(a[2] + 4), m.r32(a[2] + 8)];
        log(m, &p);
        0x229
    });
    rec(&mut t, 230, 2, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, m.r32(a[1] + 4 * i));
        }
        0x230
    });
    rec(&mut t, 941, 1, move |_, _| table);
    t.write(table, &[0; 0x100]);
    t.set_u16(table + 6, 3);
    for k in 0..3u32 {
        let e = table + 0x10 + 8 * k;
        t.set_u16(e, (0x80 + 0x10 * k) as u16);
        t.set_u16(e + 2, 0x300);
        t.set_u8(e + 4, if k == 2 { 0x80 } else { 0 });
        let r = table + 0x80 + 0x10 * k;
        t.set_u32(r + 4, 4 * k);
        t.set_u8(r + 4 * k + 4, 0x31 + k as u8);
    }
    rec(&mut t, 772, 2, |_, a| 0x7720_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    rec(&mut t, 773, 4, move |m, a| {
        m.w32(SCRATCH + 0x9200 + 4 * (a[3] & 7), m.r32(lock));
        (a[3] >> 1) & 1
    });
    for s in [774u32, 775, 778] {
        rec(&mut t, s, if s == 775 { 2 } else { 4 }, move |m, a| {
            m.w32(SCRATCH + 0x9300 + 4 * (s - 774), m.r32(lock));
            if s != 775 {
                // the colour and the copy behind the owner block
                let v = [s, m.r32(a[3] + 4), m.r32(a[3] + 0x1c)];
                log(m, &v);
            }
            s
        });
    }
    rec(&mut t, 776, 1, |_, a| a[0] << 4);
    rec(&mut t, 777, 1, |_, a| 0x0ff0 ^ a[0]);
    for (case, count) in [0i32, 6, 6, 6].into_iter().enumerate() {
        t.set_u32(LOG, 0);
        t.set_u32(t.img.word(at + 0x7b), count.cast_unsigned());
        t.set_u32(lock, 0x77);
        for r in 0..6u32 {
            let rec0 = list + 0x50 * r;
            let b = rec0 + 0x14;
            let v = case as u32 * 6 + r;
            t.write(rec0, &[0x11; 0x50]);
            for j in 0..13 {
                set_f(&mut t, b - 8 + 4 * j, (j as f32 - 4.5) * 0.75 + v as f32);
            }
            t.set_u32(b + 0x34, u32::from(r != 1));
            t.set_u8(b + 0x38, (v % 4) as u8);
            t.set_u32(b + 0x24, [0, 5, 1, 2, 0, 3][r as usize]);
            t.set_u32(b + 0x28, [0, 2, 0, 0xffff_fffe, 0, 3][r as usize]);
            t.write(b + 0x2c, &[10, 200, 0, 255, 250, 3, 128, 1]);
            // the animation cursor at -0x14
            t.set_u32(b - 0x14, 0x0a00 + r);
            t.set_u32(b - 0x10, u32::from(!v.is_multiple_of(3)));
            t.set_u16(b - 0xc, (v % 3) as u16);
            t.set_u16(b - 0xa, if r == 3 { 0x100 } else { 0x500 });
            t.set_u32(b + 0x20, [0xffff_ffff, 2, 1, 0, 0xffff_ffff, 5][r as usize]);
        }
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            s4_misc::sprites_run_b(g, img, fr)
        });
    }
}

/// `magic_0162` 0x10008e70: an object's screen place in every mode (the
/// record placed and not, the callback).
#[test]
#[allow(clippy::too_many_lines)] // one set-up per mode
fn screen_place() {
    let Some(mut t) = twin(162) else { return };
    let f = t.at(0x8e70);
    let ap = t.img.rel32_target(f + 0x3e);
    let map = SCRATCH + 0x5000;
    t.write(map, &[2, 0, 1, 3]);
    t.set_u32(t.img.word(ap + 0x26), map);
    for i in 0..4 {
        t.set_u32(t.img.word(ap + 0x31) + 4 * i, 0x30 + 8 * i);
    }
    t.set_u32(t.img.word(t.img.rel32_target(ap + 0x8c) + 0x15), map);
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4usize {
            m.w32(a[i], (a[0] as f32 * 0.5 + i as f32).to_bits());
        }
        0x262
    });
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..3 {
            m.w32(a[3] + 4 * i, a[1].wrapping_add(i) ^ a[2]);
        }
        0x237
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..3 {
            m.w32(a[0] + 4 * i, m.r32(a[2] + 4 * i) ^ 0x0100_0000);
        }
        0x229
    });
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x325
    });
    for s in [723u32, 724, 231] {
        rec(&mut t, s, 1, move |m, a| {
            m.w32(a[0], s);
            s
        });
    }
    rec(&mut t, 565, 4, |m, a| {
        let x = m.r32(a[2]);
        m.w32(a[0], x >> 8);
        m.w32(a[0] + 4, x.rotate_left(5) >> 3);
        0x565
    });
    rec(&mut t, 0, 1, |_, a| 0x0cb0_0000 ^ a[0]);
    for s in [16u32, 364] {
        rec(&mut t, s, 1, move |_, _| s);
        t.stub_st0(s, move |_, a| f64::from(a[0] & 0xf) + f64::from(s) * 0.001);
    }
    let recs = t.img.word(f + 0xa2);
    let obj = SCRATCH + 0x1000;
    for (mode, index, placed) in [
        (1u16, 2u32, 0u8),
        (2, 3, 0),
        (3, 1, 0),
        (4, 1, 0),
        (4, u32::MAX, 1),
        (5, 0, 0),
        (0, 0, 0),
        (9, 0, 0),
    ] {
        t.write(obj, &[0x5a; 0x130]);
        t.set_u16(obj + 0x124, mode);
        t.set_u32(obj + 0x11c, 1);
        t.set_u32(obj + 0x120, index);
        t.set_u32(obj + 0xf4, magic::TRAPS);
        t.set_u32(obj + 0x114, 0xffff_f123);
        t.set_u32(obj + 0x118, 0x0000_0456);
        t.set_u32(t.img.word(f + 0x96), 2);
        for k in [1u32, 2] {
            let r = recs + 0x148 * k;
            for i in 0..4 {
                set_f(&mut t, r + 0x60 + 4 * i, 3.5 * i as f32 + k as f32);
            }
            t.set_u8(r + 0x146, placed);
        }
        t.check(f, Call::this(obj), &[], Eax::Checked, |g, img, fr| {
            s4_misc::screen_place(g, img, fr)
        });
    }
}

/// `magic_0255` 0x10001540: the frame step not ready, then with each of
/// slot 261's answers.
#[test]
fn frame_step() {
    let Some(mut t) = twin(255) else { return };
    let f = t.at(0x1540);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    t.set_u32(w(&t, 0xd), SCRATCH + 0x800);
    t.set_u32(w(&t, 0x13), 0x0a0a_0001);
    t.set_u32(w(&t, 0x31), 0x0b0b_0002);
    t.set_u32(w(&t, 0x3f), SCRATCH + 0x810);
    t.set_u32(w(&t, 0x87), 0x0e0e_0003);
    let answers = SCRATCH + 0x820;
    rec(&mut t, 261, 1, move |m, a| m.r32(answers + 4 * (a[0] & 1)));
    for s in [723u32, 724, 214] {
        rec(&mut t, s, 1, move |_, _| s);
    }
    for (ready, a0, a1) in [
        (0u32, 0u32, 0u32),
        (1, 0, 0),
        (1, 0, 1),
        (1, 1, 0),
        (1, 1, 1),
    ] {
        t.set_u32(w(&t, 0x2), ready);
        t.set_u32(answers, a0);
        t.set_u32(answers + 4, a1);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0414` 0x100069d0: the timed task in each mode, before and after
/// its time, leaving with flag bit 2.
#[test]
fn timed_task() {
    let Some(mut t) = twin(414) else { return };
    let f = t.at(0x69d0);
    let k2 = f32::from_bits(t.u32(t.img.word(f + 0x99)));
    let k = f32::from_bits(t.u32(t.img.word(f + 0x17)));
    rec(&mut t, 239, 3, |_, _| 0x239);
    t.set_u32(t.img.word(f + 0xb1), 0x00a0_0001);
    let (task, s, node) = (SCRATCH + 0x100, SCRATCH + 0x1000, SCRATCH + 0x200);
    for (mode, flags, time) in [
        (0u32, 0u8, k2 - k - 1.0),
        (0, 0, k2 - k),
        (1, 4, 0.0),
        (1, 0, k2),
        (3, 4, k2 + 1.0),
        (2, 0, f32::NAN),
    ] {
        t.write(s, &[0x33; 0x300]);
        // the four nodes: no parents
        for off in [0x120u32, 0x14c, 0x178, 0x2d8] {
            t.set_u32(s + off + 0x14, 0);
        }
        set_f(&mut t, s + 0x2c4, time);
        t.set_u32(s + 0x2d0, mode);
        t.set_u8(s + 0x108, flags);
        t.set_u32(s + 0x2d4, SCRATCH + 0x400);
        t.set_u32(SCRATCH + 0x450, 0x50);
        // the task in a ring of two
        let other = SCRATCH + 0x300;
        t.set_u32(task, node);
        t.set_u32(task + 4, s);
        t.set_u32(node + 4, other);
        t.set_u32(node + 8, other);
        t.set_u32(other + 4, node);
        t.set_u32(other + 8, node);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0565` 0x10001870 and 0x10001000: a record reset for each kind,
/// and the family's start-up.
#[test]
fn fam565_reset_and_start() {
    let Some(mut t) = twin(565) else { return };
    let f = t.at(0x1870);
    let i_at = t.img.word(f + 0x1);
    let kinds = t.img.word(f + 0xf);
    for (i, kind) in [(0u32, 0u32), (1, 1), (2, 2), (3, 3), (1, 7)] {
        t.set_u32(i_at, i);
        t.set_u32(kinds + 4 * i, kind);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    let hs = t.at(0x1000);
    let block = SCRATCH + 0x2000;
    rec(&mut t, 168, 1, move |_, _| block);
    t.write(block, &[0; 0x200]);
    for s in [
        215u32, 413, 218, 291, 367, 556, 555, 331, 720, 718, 222, 221,
    ] {
        rec(&mut t, s, 1, move |_, a| s ^ (a[0] << 8));
    }
    rec(&mut t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |_, _| 0.625);
    for arg in [0x0a0a_0001u32, 0x0a0a_0002] {
        t.check_entry(hs, Call::default(), &[arg], Eax::Checked);
    }
}

/// `magic_0565` 0x100013f0: the screen bars for several counts, the first
/// time and later, with and without the last two slots.
#[test]
fn fam565_screen_bars() {
    let Some(mut t) = twin(565) else { return };
    let f = t.at(0x13f0);
    for (s, n) in [
        (628u32, 1usize),
        (821, 1),
        (811, 5),
        (809, 3),
        (812, 4),
        (650, 0),
        (382, 2),
    ] {
        rec(&mut t, s, 0, move |m, a| {
            let words: Vec<u32> = a.iter().take(n).copied().collect();
            log(m, &words);
            s
        });
    }
    for (init, n, m) in [
        (0u32, 0u32, 1u32),
        (1, 3, 0),
        (1, 10, 1),
        (1, 0xffff_fff0, 2),
    ] {
        t.set_u32(t.img.word(f + 0x8), init);
        t.set_u32(t.img.word(f + 0x45), n);
        t.set_u32(t.img.word(f + 0x22f), m);
        t.set_u32(LOG, 0);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// The family's two sparks (`magic_0446` 0x100015d0, `magic_0565`
/// 0x10003170): both made, the second held back by the flags, and the
/// pool full.
#[test]
fn fam446_sparks() {
    for (n, rva) in [(446u16, 0x15d0u32), (565, 0x3170)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        let take = t.img.rel32_target(f + 0x9);
        let recs = SCRATCH + 0x4000;
        t.set_u32(t.img.word(take + 0x37), recs);
        t.set_u32(t.img.word(take + 0x61), recs);
        let gf = SCRATCH + 0x800;
        t.set_u32(t.img.word(f + 0x150), gf);
        let r2 = SCRATCH + 0x880;
        t.set_u32(t.img.word(f + 0x190), r2);
        for i in 0..8 {
            t.set_u32(r2 + 4 * i, 0x0e00_0000 + i);
        }
        rec(&mut t, 719, 3, |_, a| 0x7190_0000 ^ a[1]);
        rec(&mut t, 941, 1, |_, a| 0x9410_0000 ^ a[0]);
        for s in [314u32, 404] {
            rec(&mut t, s, 1, move |_, a| s ^ a[0]);
            t.stub_st0(s, move |_, a| {
                f64::from(f32::from_bits(a[0])) * if s == 404 { 0.75 } else { 0.5 }
            });
        }
        let pos = SCRATCH + 0x900;
        for (i, x) in [1.5f32, -2.0, 3.25, 1.0].into_iter().enumerate() {
            set_f(&mut t, pos + 4 * i as u32, x);
        }
        for (flags, count, taken) in [(0u8, 8u32, 0u32), (2, 8, 0), (0, 2, 1), (4, 8, 0)] {
            t.set_u8(gf, flags);
            t.set_u32(t.img.word(take + 0x08), 0);
            t.set_u32(t.img.word(take + 0x0f), count);
            t.set_u32(t.img.word(take + 0x97), count);
            t.set_u32(t.img.word(take + 0x2b), 0);
            for i in 0..8 {
                t.write(recs + 0x74 * i, &[0x22; 0x74]);
                t.set_u32(recs + 0x74 * i + 0x6c, taken);
            }
            t.check(f, Call::this(pos), &[], Eax::Checked, |g, img, fr| {
                if n == 446 {
                    s4_fam446::sparks_a(g, img, fr)
                } else {
                    s4_fam446::sparks_b(g, img, fr)
                }
            });
        }
    }
}

/// The family's other two sparks (`magic_0446` 0x100018b0 and
/// 0x10001b90): both made, held back by the flags, and the pool full.
#[test]
fn fam446_sparks_cd() {
    for (n, rva) in [(446u16, 0x18b0u32), (446, 0x1b90)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        let take = t
            .img
            .rel32_target(f + if rva == 0x18b0 { 0x13 } else { 0x17 });
        let recs = SCRATCH + 0x4000;
        t.set_u32(t.img.word(take + 0x37), recs);
        t.set_u32(t.img.word(take + 0x61), recs);
        let gf = SCRATCH + 0x800;
        t.set_u32(t.img.word(f + if rva == 0x18b0 { 0x14c } else { 0x5 }), gf);
        let r2 = SCRATCH + 0x880;
        for op in if rva == 0x18b0 {
            [0x5u32, 0x18f]
        } else {
            [0x27, 0x18c]
        } {
            t.set_u32(t.img.word(f + op), r2);
        }
        for i in 0..8 {
            t.set_u32(r2 + 4 * i, 0x0e00_0000 + i);
        }
        rec(&mut t, 719, 3, |_, a| 0x7190_0000 ^ a[1]);
        rec(&mut t, 941, 1, |_, a| 0x9410_0000 ^ a[0]);
        for s in [314u32, 404] {
            rec(&mut t, s, 1, move |_, a| s ^ a[0]);
            t.stub_st0(s, move |_, a| {
                f64::from(f32::from_bits(a[0])) * if s == 404 { 0.75 } else { 0.5 }
            });
        }
        let pos = SCRATCH + 0x900;
        for (i, x) in [1.5f32, -2.0, 3.25, 1.0].into_iter().enumerate() {
            set_f(&mut t, pos + 4 * i as u32, x);
        }
        for (flags, count, taken) in [(0u8, 8u32, 0u32), (2, 8, 0), (0, 2, 1), (4, 8, 0)] {
            t.set_u8(gf, flags);
            t.set_u32(t.img.word(take + 0x08), 0);
            t.set_u32(t.img.word(take + 0x0f), count);
            t.set_u32(t.img.word(take + 0x97), count);
            t.set_u32(t.img.word(take + 0x2b), 0);
            for i in 0..8 {
                t.write(recs + 0x74 * i, &[0x22; 0x74]);
                t.set_u32(recs + 0x74 * i + 0x6c, taken);
            }
            t.check(f, Call::this(pos), &[], Eax::Checked, |g, img, fr| {
                if rva == 0x18b0 {
                    s4_fam446::sparks_c(g, img, fr)
                } else {
                    s4_fam446::sparks_d(g, img, fr)
                }
            });
        }
    }
}

/// `magic_0543`'s small entries: the caster shown again (with and
/// without the first slot), the counter reset, and the per-frame task
/// with no objects (ending or not).
#[test]
fn fam543_small() {
    let Some(mut t) = twin(543) else { return };
    let sc = t.at(0x4c30);
    rec(&mut t, 213, 2, |_, _| 0x213);
    rec(&mut t, 219, 2, |_, a| 0x0219_0000 ^ a[0]);
    rec(&mut t, 257, 2, |_, _| 0x257);
    t.set_u32(t.img.word(sc + 0x13), 0x0aaa_0001);
    t.set_u32(t.img.word(sc + 0x2b), 0x0bbb_0002);
    for word in [0u16, 1] {
        t.set_u16(t.img.word(sc + 0x3), word);
        t.check_entry(sc, Call::default(), &[], Eax::Checked);
    }
    let cr = t.at(0x3ad0);
    let block = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(cr + 0x2), block);
    for v in [0u32, 5] {
        t.write(block + 0xd740, &[0x77; 0x14]);
        t.set_u32(block + 0xd748, v);
        t.check_entry(cr, Call::default(), &[], Eax::Checked);
    }
    let ot = t.at(0x4a90);
    t.set_u32(t.img.word(ot + 0xa), block);
    for s in [
        941u32, 772, 779, 552, 230, 554, 229, 231, 712, 713, 330, 328, 329,
    ] {
        rec(&mut t, s, 1, move |_, _| s);
    }
    let task = SCRATCH + 0x800;
    let owner = SCRATCH + 0x900;
    for end in [0u32, 1] {
        t.set_u32(block + 0xebfc, end);
        t.set_u32(task + 8, owner);
        t.set_u32(task + 0x14, 0x55);
        t.write(owner, &[0x44; 0x20]);
        t.check_entry(ot, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0146` 0x10006d20 and `magic_0186` 0x10007780: an effect node set
/// up from a record on the frame, its list empty or with a last child.
#[test]
fn effect_node_records() {
    for (n, rva) in [(146u16, 0x6d20u32), (186, 0x7780)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        rec(&mut t, 719, 3, |_, a| 0x7190_0000 ^ a[0] ^ a[2]);
        let list = t.img.word(f + 0x18);
        let node = SCRATCH + 0x1000;
        let kid = SCRATCH + 0x1400;
        for last in [list, kid] {
            t.write(node, &[0xa5; 0x100]);
            t.write(kid, &[0x3c; 0x40]);
            t.set_u32(list + 0x10, last);
            t.set_u16(list + 0x18, 0xfffe);
            let c = Call::this(node);
            t.check_entry(
                f,
                c,
                &[0x0808_0808, 0x0c0c_0c0c, 0xdead, 0x0303],
                Eax::Checked,
            );
        }
    }
}

/// `magic_0162` 0x10007ef0 and `magic_0053` 0x10002c30: a state's task
/// started on an empty task list and on one with a last node (the scales
/// above 0, at 0, below and NaN).
#[test]
fn state_tasks() {
    for (n, rva) in [(162u16, 0x7ef0u32), (53, 0x2c30)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        let (call, head, tail) = if n == 162 {
            (0x7d, 0x83, 0x9b)
        } else {
            (0x54, 0x5a, 0x6e)
        };
        let take = t.img.rel32_target(f + call);
        let (head, tail) = (t.img.word(f + head), t.img.word(f + tail));
        let s = SCRATCH + 0x100;
        for (k, (a, b)) in [(1.5f32, 2.0f32), (0.0, -1.0), (f32::NAN, 0.5), (-0.0, 3.0)]
            .into_iter()
            .enumerate()
        {
            free_ring(&mut t, take, SCRATCH + 0x1000, 3);
            if k % 2 == 0 {
                t.set_u32(head, 0);
                t.set_u32(tail, 0);
            } else {
                t.write(SCRATCH + 0x1400, &[0x22; 0x1c]);
                t.set_u32(head, SCRATCH + 0x1400);
                t.set_u32(tail, SCRATCH + 0x1400);
            }
            t.write(s, &[0x11; 0x50]);
            let c = Call::this(s).with_xmm(&[Xmm::f32(9.0), Xmm::f32(a), Xmm::f32(b)]);
            t.check(f, c, &[0x0a00 + k as u32], Eax::Checked, |g, img, fr| {
                if n == 162 {
                    s4_misc::scale_task(g, img, fr)
                } else {
                    s4_misc::state_task(g, img, fr)
                }
            });
        }
    }
}

/// `magic_0565` 0x100020c0: glyph quads for a few glyphs, rows and
/// columns.
#[test]
fn glyph_quad() {
    let Some(mut t) = twin(565) else { return };
    let f = t.at(0x20c0);
    let tex = SCRATCH + 0x800;
    t.set_u32(tex, 0x0123_4567);
    t.set_u32(tex + 4, 0x89ab_cdef);
    rec(&mut t, 719, 3, move |m, a| {
        m.w32(SCRATCH + 0x810, a[2]);
        tex
    });
    for (s, n) in [(382u32, 2usize), (810, 3), (811, 5), (809, 3), (812, 4)] {
        rec(&mut t, s, 0, move |m, a| {
            let words: Vec<u32> = a.iter().take(n).copied().collect();
            log(m, &words);
            s
        });
    }
    for (glyph, row, args) in [
        (0u32, 0u32, [0u32, 1, 1, 0x80, 0x40]),
        (3, 2, [5, 2, 3, 0xff, 0x10]),
        (7, 0xffff_fff0, [0xffff_ff00, 4, 0x10, 1, 0]),
    ] {
        t.set_u32(LOG, 0);
        let c = Call {
            ecx: glyph,
            edx: row,
            ..Call::default()
        };
        t.check(f, c, &args, Eax::Checked, |g, img, fr| {
            s4_fam446::glyph_quad(g, img, fr)
        });
    }
}

/// The small entries: slot 307's float, a task's motion, the callback
/// registration, the unlisting, the key registration, the state list,
/// the part freed and the two counters.
#[test]
#[allow(clippy::too_many_lines)] // one set-up per DLL
fn small_entries() {
    if let Some(mut t) = twin(42) {
        let f = t.at(0xbe30);
        rec(&mut t, 307, 0, |_, _| 0x307);
        t.stub_st0(307, |_, _| 1.0 / 3.0);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            s4_misc::float_307(g, img, fr)
        });
    }
    if let Some(mut t) = twin(200) {
        let f = t.at(0x3210);
        rec(&mut t, 239, 3, |_, _| 0x239);
        t.set_u32(t.img.word(f + 0x15), 0x0aaa_0001);
        let task = SCRATCH + 0x100;
        t.set_u32(task + 0x120, 0x120);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    if let Some(mut t) = twin(393) {
        let f = t.at(0x14b0);
        rec(&mut t, 312, 4, |_, _| 0x0312_0312);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    if let Some(mut t) = twin(18) {
        let f = t.at(0x4020);
        rec(&mut t, 713, 1, |_, _| 0x713);
        t.set_u32(t.img.word(f + 0x16), SCRATCH + 0x80);
        let (owner, obj) = (SCRATCH + 0x100, SCRATCH + 0x400);
        let node = SCRATCH + 0x300;
        for listed in [false, true] {
            t.write(owner, &[0; 0x50]);
            t.write(node, &[0; 0x10]);
            t.set_u32(node + 8, obj);
            if listed {
                t.set_u32(owner + 0x30, node);
                t.set_u32(owner + 0x34, node);
                t.set_u32(owner + 0x3c, 1);
            }
            t.set_u16(obj + 0x124, 0xffff);
            let c = Call {
                ecx: owner,
                edx: obj,
                ..Call::default()
            };
            t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                s4_misc::unlist_free(g, img, fr)
            });
        }
    }
    if let Some(mut t) = twin(155) {
        let f = t.at(0x82f0);
        let lock = SCRATCH + 0x80;
        t.set_u32(t.img.word(f + 0x1c), lock);
        t.set_u32(lock, 7);
        rec(&mut t, 771, 1, |_, a| 0x7710_0000 ^ a[0]);
        t.stub_edx(771, |_, a| 0x0edd_0000 ^ a[0]);
        rec(&mut t, 779, 2, move |m, _| {
            let l = m.r32(lock);
            m.w32(SCRATCH + 0x84, l);
            0x779
        });
        let key = SCRATCH + 0x100;
        t.set_u32(key, 0x4242);
        t.check_entry(f, Call::default(), &[key], Eax::Checked);
    }
    if let Some(mut t) = twin(84) {
        let f = t.at(0x1a40);
        let actor = SCRATCH + 0x80;
        t.set_u32(t.img.word(f + 0x1), actor);
        rec(&mut t, 390, 1, |m, _| m.r32(SCRATCH + 0x90));
        rec(&mut t, 534, 1, |_, a| 0x5340 + a[0]);
        let list = t.img.word(f + 0x20);
        for state in [t.u32(list), t.u32(list + 4), 0x7777_7777] {
            t.set_u8(actor, 3);
            t.set_u32(SCRATCH + 0x90, state);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
    if let Some(mut t) = twin(198) {
        let f = t.at(0x1320);
        t.set_u32(t.img.word(f + 0x1), SCRATCH + 0x80);
        rec(&mut t, 713, 1, |_, _| 0x713);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    if let Some(mut t) = twin(644) {
        let block = SCRATCH + 0x1_0000;
        let task = SCRATCH + 0x100;
        for (rva, a) in [(0x9f10u32, 0xd61cu32), (0x8560, 0x5ad8)] {
            let f = t.at(rva);
            t.set_u32(t.img.word(f + 0x5), block);
            for (n, end) in [(3u32, 3u32), (2, 3)] {
                t.set_u32(block + a, n);
                t.set_u32(block + a + 4, end);
                t.set_u32(block + a + 8, 0);
                t.set_u8(task + 0x56, 0);
                t.check_entry(f, Call::default(), &[task], Eax::Checked);
            }
        }
    }
}

/// The grid frame (`magic_0242`), the block object freed (`magic_0565`)
/// and the objects' end frames (`magic_0644`, `magic_0565`), with no
/// objects to run.
#[test]
fn object_frames() {
    let stubs = |t: &mut Twin| {
        for s in [
            941u32, 772, 779, 552, 230, 554, 229, 231, 712, 713, 330, 328, 329, 219, 257, 214,
        ] {
            rec(t, s, 1, move |_, _| s);
        }
    };
    if let Some(mut t) = twin(242) {
        stubs(&mut t);
        let f = t.at(0x3a60);
        let task = SCRATCH + 0x800;
        let owner = SCRATCH + 0x900;
        for _ in 0..2 {
            t.set_u32(task + 8, owner);
            t.check_entry(f, Call::default(), &[task], Eax::Checked);
        }
    }
    if let Some(mut t) = twin(565) {
        stubs(&mut t);
        let f = t.at(0x6140);
        let block = SCRATCH + 0x1_0000;
        t.set_u32(t.img.word(f + 0x3), block);
        t.set_u32(t.img.word(f + 0x1e), SCRATCH + 0x80);
        let list = t.img.word(f + 0x10);
        let obj = block + 0xf70;
        let node = SCRATCH + 0x300;
        for listed in [false, true] {
            t.write(list, &[0; 0x10]);
            t.write(node, &[0; 0x10]);
            t.set_u32(node + 8, obj);
            if listed {
                t.set_u32(list, node);
                t.set_u32(list + 4, node);
                t.set_u32(list + 0xc, 1);
            }
            t.set_u16(obj + 0x124, 0xffff);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
        let f = t.at(0x6b40);
        t.set_u32(t.img.word(f + 0x3), block);
        for end in [0u32, 1] {
            t.set_u32(block + 0xeca4, end);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
    if let Some(mut t) = twin(644) {
        stubs(&mut t);
        let f = t.at(0xa970);
        let block = SCRATCH + 0x1_0000;
        t.set_u32(t.img.word(f + 0x3), block);
        for end in [0u32, 1] {
            t.set_u32(block + 0xeb74, end);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// The sprite take (`magic_0370`), the counter set-up (`magic_0644`) and
/// the part start (`magic_0700`).
#[test]
fn takes_and_starts() {
    if let Some(mut t) = twin(370) {
        let f = t.at(0x2f00);
        for s in [942u32, 941] {
            rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        }
        let pool = SCRATCH + 0x100;
        let sprites = SCRATCH + 0x1000;
        for (n, busy) in [(0u32, 0usize), (3, 0), (3, 2), (2, 2)] {
            t.write(sprites, &[0x5a; 0x94 * 3]);
            for i in 0..3u32 {
                t.set_u32(sprites + 0x94 * i, u32::from((i as usize) < busy));
            }
            t.set_u32(pool, n);
            t.set_u32(pool + 4, sprites);
            let c = Call {
                ecx: pool,
                edx: 0x0e5e_0001,
                ..Call::default()
            };
            t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                s4_misc::sprite_take(g, img, fr)
            });
        }
    }
    if let Some(mut t) = twin(644) {
        let f = t.at(0x9940);
        let block = SCRATCH + 0x1_0000;
        t.set_u32(t.img.word(f + 0x3), block);
        rec(&mut t, 219, 2, |_, a| 0x0219_0000 ^ a[0]);
        rec(&mut t, 257, 2, |_, _| 0x257);
        let rec4 = SCRATCH + 0x200;
        t.set_u32(block + 0xeb64, rec4);
        t.set_u32(rec4 + 4, 0x44);
        for on in [0u32, 1] {
            t.set_u32(block + 0xd618, on);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
    if let Some(mut t) = twin(700) {
        let f = t.at(0x1000);
        for s in [718u32, 222, 221] {
            rec(&mut t, s, 1, move |_, _| s);
        }
        rec(&mut t, 719, 3, |_, a| 0x7190_0000 ^ a[1]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// The part toggle (`magic_0165`) with and without a part, and the model
/// nodes reset (`magic_0201`) with no nodes, one under a parent and two.
#[test]
fn part_toggle_and_nodes_reset() {
    if let Some(mut t) = twin(165) {
        let f = t.at(0x6ad0);
        t.set_u32(t.img.word(f + 0x13), 0x0c0c_0001);
        for s in [210u32, 211, 255] {
            rec(&mut t, s, 2, move |_, _| s);
        }
        rec(&mut t, 217, 2, |m, a| {
            m.w32(a[1], 0x3f80_0000 ^ a[0]);
            0x217
        });
        for (part, skip, actor) in [
            (0x0e00_0001u32, 0u32, 3u32),
            (0, 0, 3),
            (0, 1, 3),
            (0, 0, 0x20),
        ] {
            t.set_u32(t.img.word(f + 0x34), skip);
            t.set_u32(t.img.word(f + 0x3e), actor);
            let c = Call {
                edx: part,
                ..Call::default()
            };
            t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                s4_misc::part_toggle(g, img, fr)
            });
        }
    }
    if let Some(mut t) = twin(201) {
        let f = t.at(0x3720);
        for s in [538u32, 213, 211, 35, 34, 547, 276, 219] {
            rec(&mut t, s, 1, move |_, _| s);
        }
        let (cursor, owner, parent) = (SCRATCH + 0x80, SCRATCH + 0x1000, SCRATCH + 0x3000);
        for n in [0u32, 1, 2] {
            t.write(owner, &[0; 0x500]);
            t.set_u32(owner + 0x4dc, n);
            t.write(parent, &[0x45; 0x40]);
            for i in 0..2u32 {
                let node = owner + 0x1c + 0x98 * i;
                t.set_u32(node + 0x14, if i == 0 { parent } else { 0 });
                t.set_u32(node + 4, parent);
                t.set_u32(node + 8, parent);
                t.set_u32(node + 0x64, u32::MAX);
            }
            t.set_u16(parent + 0x18, 2);
            t.set_u32(cursor, owner);
            let c = Call {
                eax: 0x00ea_0000,
                ecx: cursor,
                ..Call::default()
            };
            let saved = t.save();
            t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                s4_misc::nodes_reset(g, img, fr)
            });
            // through its thunk: the cursor as the first stack word
            t.restore(&saved);
            let c = Call {
                eax: 0x00ea_0000,
                ecx: 0x0bad_0ecc,
                ..Call::default()
            };
            t.check_entry(t.at(0x37b0), c, &[cursor, 0x0bad_f00e], Eax::Checked);
        }
    }
}

/// The small `magic_0515` functions, the float of slot 664, the model
/// node with its record, the effect frames and the range stop.
#[test]
#[allow(clippy::too_many_lines)] // one set-up per function
fn small_internals() {
    if let Some(mut t) = twin(515) {
        rec(&mut t, 220, 3, |_, _| 0x220);
        rec(&mut t, 239, 3, |_, _| 0x239);
        rec(&mut t, 276, 2, |_, a| 0x0276_0000 ^ a[0]);
        rec(&mut t, 310, 1, |_, a| 0x0310_0000 ^ a[0]);
        let f = t.at(0x9d20);
        t.check(f, Call::this(0x1234), &[], Eax::Checked, |g, img, fr| {
            s4_misc::call_220(g, img, fr)
        });
        let f = t.at(0x9d40);
        t.set_u32(t.img.word(f + 0x10), 0x0aaa_0001);
        let c = Call {
            edx: 0x5678,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            s4_misc::call_239(g, img, fr)
        });
        let f = t.at(0x3d70);
        let recs = t.img.word(f + 0x6);
        for free in [0u32, 7, 0x100] {
            for i in 0..0x100 {
                t.set_u32(recs + 0x4c * i + 0x28, u32::from(i != free));
            }
            t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
                s4_misc::free_record(g, img, fr)
            });
        }
        let f = t.at(0x2d70);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            s4_misc::records_clear(g, img, fr)
        });
        let f = t.at(0x2960);
        let n = t.img.word(f + 0x5);
        for count in [0u32, 3] {
            t.set_u32(n, count);
            let c = Call {
                eax: 0x00ea,
                ..Call::default()
            };
            t.check_entry(f, c, &[], Eax::Checked);
        }
        let f = t.at(0x24d0);
        for (count, state) in [(0u32, 0u32), (2, 0), (3, 0xff)] {
            t.set_u32(t.img.word(f + 0x7), count);
            t.set_u32(t.img.word(f + 0x13), state);
            t.set_u32(t.img.word(f + 0x24), 0x0bbb_0002);
            let c = Call {
                eax: 0x00eb,
                ..Call::default()
            };
            t.check_entry(f, c, &[], Eax::Checked);
        }
    }
    if let Some(mut t) = twin(164) {
        let f = t.at(0xd070);
        rec(&mut t, 664, 1, |_, _| 0x664);
        t.stub_st0(664, |_, a| f64::from(a[0]) * 0.5);
        t.check(f, Call::this(7), &[], Eax::Checked, |g, img, fr| {
            s4_misc::float_664(g, img, fr)
        });
    }
    if let Some(mut t) = twin(70) {
        let f = t.at(0x94d0);
        for s in [413u32, 215, 14, 333] {
            rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        }
        t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.25);
        let (node, parent) = (SCRATCH + 0x1000, SCRATCH + 0x2000);
        for p in [0u32, parent] {
            t.write(node, &[0x66; 0x100]);
            t.write(parent, &[0x77; 0x40]);
            t.set_u32(parent + 0x10, parent);
            let c = Call {
                ecx: node,
                edx: p,
                ..Call::default()
            };
            t.check(f, c, &[0x0b0b], Eax::Checked, |g, img, fr| {
                s4_misc::model_node_with(g, img, fr)
            });
        }
    }
    for n in [248u16, 177] {
        if let Some(mut t) = twin(n) {
            for s in [
                941u32, 772, 779, 552, 230, 554, 229, 231, 712, 713, 330, 328, 329,
            ] {
                rec(&mut t, s, 1, move |_, _| s);
            }
            let f = t.at(if n == 248 { 0x1650 } else { 0x3460 });
            let task = SCRATCH + 0x800;
            t.set_u32(task + 8, SCRATCH + 0x900);
            t.check_entry(f, Call::default(), &[task], Eax::Checked);
        }
    }
    if let Some(mut t) = twin(284) {
        let f = t.at(0x21e0);
        let k = |t: &Twin, o: u32| f32::from_bits(t.u32(t.img.word(f + o)));
        let (k1, k2, k3) = (k(&t, 0xa), k(&t, 0x1b), k(&t, 0x25));
        let p = SCRATCH + 0x100;
        for (x, y) in [
            (k1, 0.0),
            (k2, k3 - 1.0),
            (f32::midpoint(k1, k2), k3),
            (f32::midpoint(k1, k2), k3 + 1.0),
            (f32::NAN, f32::NAN),
        ] {
            t.write(p, &[0x11; 0x70]);
            set_f(&mut t, p + 0xc, x);
            set_f(&mut t, p + 0x14, y);
            t.check_entry(f, Call::default(), &[p], Eax::Checked);
        }
    }
}

/// More small functions: the actors' and models' slot loops, the ring
/// run, the handle registrations, the fade and the object hook.
#[test]
#[allow(clippy::too_many_lines)] // one set-up per function
fn small_loops() {
    if let Some(mut t) = twin(515) {
        rec(&mut t, 296, 2, |_, a| 0x0296_0000 ^ a[0]);
        rec(&mut t, 295, 2, |_, a| 0x0295_0000 ^ a[0]);
        rec(&mut t, 772, 2, |_, a| 0x7720_0000 ^ a[1]);
        t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
        let lock = SCRATCH + 0x80;
        rec(&mut t, 779, 2, move |m, _| m.r32(lock));
        let f = t.at(0x2510);
        let n = t.img.word(f + 0x2);
        let actors = t.img.word(f + 0x12);
        for (count, skip) in [(0u32, 9u32), (3, 1), (2, 1)] {
            t.set_u32(n, count);
            for i in 0..3 {
                t.set_u32(actors + 0xcc * i, if i == skip { 0xff } else { 0x10 + i });
            }
            let c = Call {
                eax: 0x00ea,
                ecx: 0x0f,
                ..Call::default()
            };
            t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                s4_misc::actors_296(g, img, fr)
            });
        }
        let f = t.at(0x25d0);
        for count in [0u32, 2] {
            t.set_u32(t.img.word(f + 0x5), count);
            t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
                s4_misc::models_295(g, img, fr)
            });
        }
        let f = t.at(0x38e0);
        t.set_u32(t.img.word(f + 0x22), lock);
        t.set_u32(lock, 3);
        let parts = t.img.word(f + 0x4);
        for i in 0..0x200 {
            t.set_u32(parts + 0x3c * i, [u32::MAX, 0, 1][(i % 3) as usize]);
        }
        let c = Call {
            eax: 0x00ec,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            s4_misc::register_parts_b(g, img, fr)
        });
    }
    if let Some(mut t) = twin(577) {
        let f = t.at(0x87a0);
        let vt = SCRATCH + 0x100;
        t.set_u32(vt + 0x20, magic::TRAPS);
        rec(&mut t, 0, 1, |m, a| 0x77 ^ m.r32(a[0]));
        let obj = SCRATCH + 0x200;
        for (skip, n) in [(0u32, 0u32), (0, 2), (1, 2)] {
            t.write(obj, &[0x33; 0x20]);
            t.set_u32(t.img.word(f + 0x8), skip);
            let mut prev = obj + 4;
            for i in 0..n {
                let nd = SCRATCH + 0x400 + 0x40 * i;
                t.set_u32(nd, vt);
                if i == 0 {
                    t.set_u32(obj + 0xc, nd);
                } else {
                    t.set_u32(prev + 8, nd);
                }
                prev = nd;
            }
            if n == 0 {
                t.set_u32(obj + 0xc, obj + 4);
            } else {
                t.set_u32(prev + 8, obj + 4);
            }
            let c = Call {
                eax: 0x00ed,
                ecx: obj,
                ..Call::default()
            };
            t.check(f, c, &[], Eax::Checked, |g, img, fr| {
                s4_misc::ring_run(g, img, fr)
            });
        }
    }
    if let Some(mut t) = twin(132) {
        let f = t.at(0x2c80);
        let lock = SCRATCH + 0x80;
        t.set_u32(t.img.word(f + 0x21), lock);
        t.set_u32(lock, 5);
        rec(&mut t, 772, 2, |_, a| 0x7720_0000 ^ a[1]);
        t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
        rec(&mut t, 779, 2, move |m, _| m.r32(lock));
        t.check(f, Call::this(0x4242), &[], Eax::Checked, |g, img, fr| {
            s4_misc::register_80(g, img, fr)
        });
    }
    if let Some(mut t) = twin(155) {
        let f = t.at(0x9180);
        rec(&mut t, 234, 3, |_, _| 0x234);
        let c = Call::default().with_xmm(&[Xmm::f32(0.0), Xmm::f32(0.75)]);
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            s4_misc::fade_set(g, img, fr)
        });
    }
    if let Some(mut t) = twin(301) {
        let f = t.at(0x1c10);
        let a = SCRATCH + 0x100;
        t.set_u32(t.img.word(f + 0x4), a);
        t.set_u32(a + 0x198, 0x0c0c_0001);
        for s in [447u32, 450, 449] {
            rec(&mut t, s, 2, move |_, _| s);
        }
        let obj = SCRATCH + 0x400;
        for i in [0u32, 3] {
            t.write(obj, &[0x22; 0xb0]);
            t.set_u32(obj + 0xac, i);
            t.check_entry(f, Call::default(), &[obj], Eax::Checked);
        }
    }
}

/// The view matrix (`magic_0307`), the static node (`magic_0070`, an
/// empty list and one with a last node) and the part block start
/// (`magic_0477`).
#[test]
fn view_node_part() {
    if let Some(mut t) = twin(307) {
        let f = t.at(0x63e0);
        for s in [723u32, 724, 327] {
            rec(&mut t, s, 1, move |m, a| {
                m.w32(a[0], s);
                s
            });
        }
        t.check_entry(f, Call::default(), &[SCRATCH + 0x100], Eax::Checked);
    }
    if let Some(mut t) = twin(70) {
        let f = t.at(0x92a0);
        let list = t.img.word(f + 0x1f);
        let node = SCRATCH + 0x1000;
        for last in [list, SCRATCH + 0x2000] {
            t.write(node, &[0x66; 0x80]);
            t.write(SCRATCH + 0x2000, &[0x22; 0x20]);
            t.set_u32(list + 0x10, last);
            t.set_u16(list + 0x18, 4);
            t.check(
                f,
                Call::this(node),
                &[0x0b0b],
                Eax::Checked,
                s4_misc::static_node,
            );
        }
    }
    if let Some(mut t) = twin(477) {
        let f = t.at(0x1000);
        t.set_u32(t.img.word(f + 0x1), SCRATCH + 0x200);
        t.set_u32(t.img.word(f + 0xc), SCRATCH + 0x300);
        t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
        t.set_u32(SCRATCH + 0x400, 0x1111);
        t.stub(941, |_, _| SCRATCH + 0x400);
        t.stub(714, |m, a| {
            for (i, w) in (0u32..).zip(&a[..5]) {
                m.w32(SCRATCH + 0x910 + 4 * i, *w);
            }
            0x714
        });
        let start = t.img.rel32_target(f + 0x31);
        t.set_u32(t.img.word(start + 0x04), SCRATCH + 0x980);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// The family's starts in `magic_0565` and `magic_0644` (their own block
/// layouts): each start with the pool free and empty, the kinds of the
/// table one.
#[test]
fn fam446_starts_565_644() {
    for (n, starts) in [
        (
            565u16,
            &[(0x4570u32, 0xcu32), (0x4d60, 0xc), (0x6240, 0xb)][..],
        ),
        (644, &[(0x8600, 0x15), (0x8590, 0xc), (0x8d80, 0xc)]),
    ] {
        let Some(mut t) = twin(n) else { return };
        let block = SCRATCH + 0x1_0000;
        let call = if starts[0].0 == 0x8600 { 0x39 } else { 0x20 };
        let soa = t.img.rel32_target(t.at(starts[0].0) + call);
        t.set_u32(t.img.word(soa + 0x04), SCRATCH + 0x200);
        t.set_u32(t.img.word(soa + 0x5e), SCRATCH + 0x300);
        for i in 0..0x30 {
            t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
        }
        t.stub(941, |_, _| SCRATCH + 0x400);
        t.stub(714, |m, a| {
            let k = m.r32(SCRATCH + 0x90c);
            m.w32(SCRATCH + 0x90c, k + 1);
            for (i, w) in (0u32..).zip(&a[..5]) {
                m.w32(SCRATCH + 0x910 + 4 * i, *w ^ k);
            }
            0x714
        });
        let pa = t.img.rel32_target(soa + 0x9d);
        let free = t.img.word(pa + 0x04);
        let nodes = t.img.word(pa + 0x27) - 8;
        let task = SCRATCH + 0x800;
        for &(rva, block_op) in starts {
            let f = t.at(rva);
            t.set_u32(t.img.word(f + block_op), block);
            for kind in [0u32, 1, 2] {
                for count in [3u16, 0] {
                    t.write(block, &vec![0x3c; 0xec00]);
                    for i in 0..4 {
                        set_f(&mut t, block + 0x2f80 + 4 * i, 1.5 * i as f32);
                        set_f(&mut t, block + 0x2e50 + 4 * i, -2.5 * i as f32);
                    }
                    t.set_u32(block + 0xebf0, 0x00c1_0c00 + kind);
                    t.set_u32(block + 0xeac0, 0x00c2_0c00 + kind);
                    for i in 0..8 {
                        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
                        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
                        t.set_u32(nodes + 12 * i + 8, 0);
                    }
                    t.set_u16(free, count);
                    t.set_u32(task + 0x2c, kind);
                    let c = Call {
                        ecx: 0x0bad_0ecc,
                        ..Call::default()
                    };
                    t.check_entry(f, c, &[task], Eax::Checked);
                }
            }
        }
    }
}

/// `magic_0565`'s camera target: every fold branch (above, below, inside).
#[test]
fn camera_target_c_folds() {
    let Some(mut t) = twin(565) else { return };
    let f = t.at(0x6d00);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0x08), fx);
    t.set_u32(t.img.word(f + 0x22), 0x00ab_0000);
    rec(&mut t, 262, 4, |m, a| {
        m.w32(a[1], 0x3f00_0000);
        0x262
    });
    rec(&mut t, 333, 1, |_, _| 0x333);
    for angle in [0.0f64, 1.0, -3.0, 7.0, 100.0, -100.0] {
        t.stub_st0(333, move |_, _| angle);
        t.write(fx + 0x2f80, &[0x5a; 0x10]);
        t.write(fx + 0xebf0, &[0x5a; 0xc0]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(ffx_magic::sa_misc::camera_target_c(g, img, fr.at))
        });
    }
}

/// The longer start-ups of `magic_0565` and `magic_0644` through their
/// load steps (busy, asking, waiting, loaded with the task list empty and
/// not).
#[test]
fn fam446_load_steps_565_644() {
    for (n, rva) in [(565u16, 0x1270u32), (644, 0x5480)] {
        let Some(mut t) = twin(n) else { return };
        let ls = t.at(rva);
        let su = t.img.rel32_target(ls + 0x5b);
        let take = t.img.rel32_target(su + 0x9e);
        let ps = t.img.rel32_target(su + 0x60);
        t.set_u32(t.img.word(ps + 0x04), SCRATCH + 0x800);
        rec(&mut t, 719, 3, |_, a| 0x7190_0000 ^ a[1]);
        for (s, k) in [
            (711u32, 3usize),
            (710, 4),
            (271, 3),
            (221, 7),
            (219, 2),
            (357, 3),
            (356, 3),
            (262, 4),
            (226, 2),
        ] {
            rec(&mut t, s, k, move |_, _| s);
        }
        rec(&mut t, 333, 1, |_, _| 0x333);
        let answers = SCRATCH + 0x8c0;
        rec(&mut t, 225, 1, move |m, _| m.r32(answers));
        rec(&mut t, 227, 1, move |m, _| m.r32(answers + 4));
        t.set_u32(t.img.word(ls + 0x26), 0x0e5e_0026);
        t.set_u32(t.img.word(ls + 0x46), 0x0e5e_0046);
        t.set_u32(t.img.word(su + 0x16), 0x1616);
        let asked = t.img.word(ls + 0x18);
        let head = t.img.word(su + 0xa4);
        let tail = t.img.word(su + 0xb8);
        let cases = [
            (1u32, 0u32, 0u32, false, 1.25f64),
            (0, 0, 0, false, 1.25),
            (0, 1, 0xffff_ffff, false, 1.25),
            (0, 1, 1, false, 100.0),
            (0, 0, 2, true, -100.0),
            (0, 1, 1, true, 1.25),
        ];
        for (busy, state, there, listed, angle) in cases {
            t.stub_st0(333, move |_, _| angle);
            free_ring(&mut t, take, SCRATCH + 0x1000, 4);
            if listed {
                t.write(SCRATCH + 0x1400, &[0x22; 0x1c]);
                t.set_u32(head, SCRATCH + 0x1400);
                t.set_u32(tail, SCRATCH + 0x1400);
            } else {
                t.set_u32(head, 0);
                t.set_u32(tail, 0);
            }
            t.set_u32(answers, busy);
            t.set_u32(answers + 4, there);
            t.set_u32(asked, state);
            t.check_entry(ls, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `magic_0644`'s object `G + 0x260` started and placed at a part.
#[test]
fn fam446_start_260_placed() {
    let Some(mut t) = twin(644) else { return };
    let f = t.at(0xa100);
    let soa = t.img.rel32_target(f + 0x1f);
    t.set_u32(t.img.word(soa + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(soa + 0x5e), SCRATCH + 0x300);
    t.stub(941, |_, _| SCRATCH + 0x400);
    t.stub(714, |_, _| 0x714);
    rec(&mut t, 219, 2, |_, a| 0x2190_0000 ^ a[0]);
    rec(&mut t, 237, 4, |m, a| {
        m.w32(a[3], 0x3f00_0000);
        0x237
    });
    let pa = t.img.rel32_target(soa + 0x9d);
    let free = t.img.word(pa + 0x04);
    let nodes = t.img.word(pa + 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    let block = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0xd), block);
    t.write(block, &vec![0x3c; 0xec00]);
    t.set_u32(block + 0xeac0, 0x0c10_c000);
    t.set_u32(t.img.word(f + 0x2c), 0x0e5e_0001);
    for count in [3u16, 0] {
        t.set_u16(free, count);
        let c = Call {
            ecx: 0x0bad_0ecc,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, s4_fam446::start_260_placed);
    }
}

/// `magic_0644`'s raised start of `G + 0x1300`.
#[test]
fn fam446_start_1300_raised() {
    let Some(mut t) = twin(644) else { return };
    let f = t.at(0xa070);
    let soa = t.img.rel32_target(f + 0x20);
    t.set_u32(t.img.word(soa + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(soa + 0x5e), SCRATCH + 0x300);
    t.stub(941, |_, _| SCRATCH + 0x400);
    t.stub(714, |_, _| 0x714);
    let pa = t.img.rel32_target(soa + 0x9d);
    let free = t.img.word(pa + 0x04);
    let block = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0xb), block);
    t.write(block, &vec![0x3c; 0xec00]);
    for i in 0..4 {
        set_f(&mut t, block + 0x2e50 + 4 * i, 1.5 * i as f32);
    }
    for count in [3u16, 0] {
        t.set_u16(free, count);
        let c = Call {
            ecx: 0x0bad_0ecc,
            ..Call::default()
        };
        t.check_entry(f, c, &[], Eax::Checked);
    }
}

/// `magic_0565`'s start by kind (kinds 0, 1 and 2).
#[test]
fn fam446_start_kind_d() {
    let Some(mut t) = twin(565) else { return };
    let f = t.at(0x30d0);
    let soa = t.img.rel32_target(f + 0x39);
    t.set_u32(t.img.word(soa + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(soa + 0x5e), SCRATCH + 0x300);
    t.stub(941, |_, _| SCRATCH + 0x400);
    t.stub(714, |_, _| 0x714);
    let pa = t.img.rel32_target(soa + 0x9d);
    let free = t.img.word(pa + 0x04);
    let block = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0x15), block);
    let task = SCRATCH + 0x800;
    for kind in [0u32, 1, 2] {
        t.write(block, &vec![0x3c; 0xec00]);
        t.set_u16(free, 3);
        t.set_u32(task + 0x2c, kind);
        let c = Call {
            ecx: 0x0bad_0ecc,
            ..Call::default()
        };
        t.check_entry(f, c, &[task], Eax::Checked);
    }
}

/// `magic_0644`'s start by kind (kinds 0, 1 and 2).
#[test]
fn fam446_start_kind_e() {
    let Some(mut t) = twin(644) else { return };
    let f = t.at(0x7260);
    let soa = t.img.rel32_target(f + 0x39);
    t.set_u32(t.img.word(soa + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(soa + 0x5e), SCRATCH + 0x300);
    t.stub(941, |_, _| SCRATCH + 0x400);
    t.stub(714, |_, _| 0x714);
    let pa = t.img.rel32_target(soa + 0x9d);
    let free = t.img.word(pa + 0x04);
    let block = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0x15), block);
    let task = SCRATCH + 0x800;
    for kind in [0u32, 1, 2] {
        t.write(block, &vec![0x3c; 0xec00]);
        t.set_u16(free, 3);
        t.set_u32(task + 0x2c, kind);
        let c = Call {
            ecx: 0x0bad_0ecc,
            ..Call::default()
        };
        t.check_entry(f, c, &[task], Eax::Checked);
    }
}

/// The mode set-ups of `magic_0565` and `magic_0644`: each mode, the task
/// list empty and not.
#[test]
fn fam446_mode_setups_565_644() {
    for (n, rva) in [(565u16, 0x3bb0u32), (644, 0x7bd0)] {
        let Some(mut t) = twin(n) else { return };
        let ms = t.at(rva);
        let take = t.img.rel32_target(ms + 0x64);
        let owner_res = SCRATCH + 0x880;
        t.set_u32(t.img.word(ms + 0x12), owner_res);
        t.write(owner_res, &[0; 0x40]);
        t.set_u32(owner_res + 0x28, 0x0e5e_0028);
        t.set_u32(owner_res + 0x2c, 0x0e5e_002c);
        let head = t.img.word(ms + 0x6a);
        let block = SCRATCH + 0x1_0000;
        t.set_u32(t.img.word(ms + 0x3), block);
        let mode_at = t.img.word(ms + 0x9);
        for mode in [0u32, 1, 2] {
            for listed in [false, true] {
                free_ring(&mut t, take, SCRATCH + 0x1000, 2);
                t.write(block + 0x5800, &[0x3c; 0x400]);
                if listed {
                    t.write(SCRATCH + 0x1400, &[0x22; 0x1c]);
                    t.set_u32(head, SCRATCH + 0x1400);
                    t.set_u32(t.img.word(ms + 0x7a), SCRATCH + 0x1400);
                } else {
                    t.set_u32(head, 0);
                }
                t.set_u32(block + mode_at, mode);
                let c = Call {
                    eax: 0x00ea_0000 + mode,
                    ..Call::default()
                };
                t.check_entry(ms, c, &[], Eax::Checked);
            }
        }
    }
}

/// The two-object starts of `magic_0565` and `magic_0644`.
#[test]
fn fam446_start_pairs() {
    for (n, rva) in [(565u16, 0x6190u32), (644, 0x9fc0)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        let soa = t.img.rel32_target(f + 0x20);
        t.set_u32(t.img.word(soa + 0x04), SCRATCH + 0x200);
        t.set_u32(t.img.word(soa + 0x5e), SCRATCH + 0x300);
        t.stub(941, |_, _| SCRATCH + 0x400);
        t.stub(714, |_, _| 0x714);
        let pa = t.img.rel32_target(soa + 0x9d);
        let free = t.img.word(pa + 0x04);
        let block = SCRATCH + 0x1_0000;
        t.set_u32(t.img.word(f + 0xc), block);
        for count in [3u16, 1, 0] {
            t.write(block, &vec![0x3c; 0xec00]);
            t.set_u16(free, count);
            let c = Call {
                ecx: 0x0bad_0ecc,
                ..Call::default()
            };
            t.check_entry(f, c, &[], Eax::Checked);
        }
    }
}

/// The mode objects of `magic_0565` and `magic_0644` (no mode test): the
/// previous object listed or not, each mode.
#[test]
fn fam446_mode_objects_565_644() {
    for (n, rva, (mode_at, old_at)) in [
        (565u16, 0x3c60u32, (0x5c04u32, 0x5bfcu32)),
        (644, 0x7c80, (0x5ad4, 0x5acc)),
    ] {
        let Some(mut t) = twin(n) else { return };
        let mo = t.at(rva);
        let soa = t.img.rel32_target(mo + 0x6f);
        t.set_u32(t.img.word(soa + 0x04), SCRATCH + 0x200);
        t.set_u32(t.img.word(soa + 0x5e), SCRATCH + 0x300);
        t.stub(941, |_, _| SCRATCH + 0x400);
        t.stub(714, |_, _| 0x714);
        rec(&mut t, 713, 1, |_, a| 0x7130_0000 ^ a[0]);
        rec(&mut t, 219, 2, |_, a| 0x2190_0000 ^ a[0]);
        rec(&mut t, 237, 4, |_, _| 0x237);
        t.set_u32(t.img.word(mo + 0x20), SCRATCH + 0x8e0);
        t.set_u32(t.img.word(mo + 0x7c), 0x0a0a_007c);
        let block = SCRATCH + 0x1_0000;
        t.set_u32(t.img.word(mo + 0x3), block);
        let list = t.img.word(mo + 0x12);
        let pa = t.img.rel32_target(soa + 0x9d);
        let free = t.img.word(pa + 0x04);
        let old = block + 0x130 * 3;
        for (mode, listed) in [(0u32, false), (0, true), (1, true), (2, false)] {
            t.write(block, &vec![0x3c; 0xec00]);
            t.set_u16(free, 2);
            t.set_u32(block + mode_at, mode);
            t.set_u32(block + old_at, old);
            t.set_u16(old + 0x124, 0xffff);
            let node = SCRATCH + 0x1800;
            t.write(node, &[0; 0x10]);
            t.set_u32(node + 8, old);
            t.write(list, &[0; 0x10]);
            if listed {
                t.set_u32(list, node);
                t.set_u32(list + 4, node);
                t.set_u32(list + 0xc, 1);
            }
            t.check_entry(mo, Call::default(), &[], Eax::Checked);
        }
    }
}
