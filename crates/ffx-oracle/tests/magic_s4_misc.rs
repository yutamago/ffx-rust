//! The shared-4 list's functions without a family (`ffx_magic::s4_misc`)
//! against the DLLs they were ported from. Needs `FFX_GAME_DIR`.

#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

mod magic;
#[path = "magic/sa.rs"]
mod sa;

use ffx_magic::{Ret, Xmm, s4_misc};
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
        a[3] & 1
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
            t.set_u32(b + 0xc, u32::from(v % 3 != 0));
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
    ] {
        let Some(mut t) = twin(n) else { return };
        let at = t.at(rva);
        let (pool_op, tail_op) = [(0x9d, 0xa8), (0x99, 0xa4), (0x9c, 0xa7), (0x96, 0xa1)][kind];
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
                Ret::eax(if kind == 1 {
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
        for (mx, my) in [(0.0f32, 0.0f32), (-1.0e6, 1.0e6), (f32::NAN, -0.01)] {
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
            t.set_u16(st + 0x2c, flags);
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

/// `magic_0052` 0x10002730: the follow task in each mode (no mode, on a
/// target with and without the turned offset, on the actors' mean), its
/// script waiting, ending with the give-back flag and without it.
#[test]
#[allow(clippy::too_many_lines)] // one set-up per callee
fn follow_task() {
    let Some(mut t) = twin(52) else { return };
    let f = t.at(0x2730);
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
        for i in 1..4 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32;
            m.w32(a[i as usize], v.to_bits());
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
            t.set_u16(st + 0x4c, 0x0123);
            t.set_u32(mean_n, n);
            t.check_entry(f, Call::default(), &[task], Eax::Checked);
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
    let map = SCRATCH + 0x5000;
    t.write(map, &[2, 0, 1, 3]);
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
    let states: [(u32, u16, u32, u32); 5] = [
        (1, 0, 0x5000, 0x0c0c_0003),
        (0, 3, 0x5000, 0x0c0c_0003),
        (0, 1, 0x5000, 0x0c0c_0003),
        (0, 0, 0x5001, 0x0c0c_0002),
        (0, 0, 0x5000, 0x0c0c_0003),
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

/// `magic_0018` 0x10002580: the trail with no sprites, a few, more than
/// the ring holds, zero-length and NaN segments, spacing below and past
/// the segments, drawn by known and new handles, alpha 0 and not.
#[test]
#[allow(clippy::too_many_lines)] // one set-up per field
fn trail_draw() {
    let Some(mut t) = twin(18) else { return };
    let f = t.at(0x2580);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let lock = SCRATCH + 0x9000;
    t.set_u32(w(&t, 0x46e), lock);
    let table = SCRATCH + 0x8000;
    rec(&mut t, 941, 1, move |_, _| table);
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
        rec(&mut t, s, 1, move |m, a| {
            for i in 0..16 {
                let v = (i as f32 + 1.0) * if s == 235 { 1.0 } else { 0.5 };
                m.w32(a[0] + 4 * i, v.to_bits());
            }
            s
        });
    }
    let fv = |m: &dyn magic::StubMem, a: u32| f32::from_bits(m.r32(a));
    rec(&mut t, 234, 3, move |m, a| {
        for i in 0..3 {
            let v = fv(m, a[1] + 4 * i) - fv(m, a[2] + 4 * i);
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x234
    });
    rec(&mut t, 248, 3, move |m, a| {
        for i in 0..3 {
            let v = fv(m, a[1] + 4 * i) + fv(m, a[2] + 4 * i);
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x248
    });
    rec(&mut t, 229, 3, |m, a| {
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
    rec(&mut t, 772, 2, |_, a| 0x7720_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    rec(&mut t, 773, 4, move |m, a| {
        m.w32(SCRATCH + 0x9200 + 4 * (a[3] & 7), m.r32(lock));
        a[3] & 1
    });
    for s in [774u32, 775, 778] {
        rec(&mut t, s, if s == 775 { 2 } else { 5 }, move |m, a| {
            m.w32(SCRATCH + 0x9300 + 4 * (s - 774), m.r32(lock));
            if s != 775 {
                let c = m.r32(a[3]);
                log(m, &[s, c]);
            }
            s
        });
    }
    rec(&mut t, 776, 1, |_, a| a[0] << 4);
    rec(&mut t, 777, 1, |_, a| 0x0ff0 ^ a[0]);
    let (obj, rows, point) = (SCRATCH + 0x1000, SCRATCH + 0x2000, SCRATCH + 0x2100);
    for i in 0..12u32 {
        set_f(&mut t, rows + 4 * i, 0.5 + i as f32 * 0.25);
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
                set_f(&mut t, obj + 0xc + 16 * k + 4 * i, x);
            }
        }
        t.write(obj + 0x20c, &[10, 200, 0, alpha, 250, 3, 128, alpha / 2]);
        set_f(&mut t, obj + 0x214, 1.5);
        set_f(&mut t, obj + 0x218, 0.25);
        set_f(&mut t, obj + 0x21c, spacing);
        t.set_u32(obj + 0x220, newest);
        t.set_u32(obj + 0x224, n);
        for (i, x) in [0.5f32, 9.0, -1.0, 7.0].into_iter().enumerate() {
            set_f(&mut t, point + 4 * i as u32, x);
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
