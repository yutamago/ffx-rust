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
        // the scale matrix's diagonal it rolls
        m.w32(a[0], m.r32(a[1]) ^ m.r32(a[1] + 0x14) ^ m.r32(a[1] + 0x28));
        m.w32(a[0] + 0x38, a[2]);
        0x330
    });
    rec(&mut t, 229, 3, |m, a| {
        m.w32(a[0], m.r32(a[2]) ^ m.r32(a[2] + 4) ^ m.r32(a[2] + 8));
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
        rec(&mut t, s, if s == 775 { 2 } else { 5 }, move |m, _| {
            m.w32(SCRATCH + 0x9300 + 4 * (s - 774), m.r32(lock));
            s
        });
    }
    rec(&mut t, 776, 1, |_, a| a[0] << 4);
    rec(&mut t, 777, 1, |_, a| a[0] & 0x0f00);
    let k1 = f32::from_bits(t.u32(t.img.word(at + 0x199)));
    let hi = f32::from_bits(t.u32(t.img.word(at + 0x1e0)));
    let lo = f32::from_bits(t.u32(t.img.word(at + 0x1f0)));
    for (case, (paused, count)) in [(1u32, 2i32), (0, 0), (0, 6), (0, 6), (0, 6)]
        .into_iter()
        .enumerate()
    {
        t.set_u32(pause, paused);
        t.set_u32(t.img.word(at + 0x90), count.cast_unsigned());
        t.set_u32(lock, 0x77);
        for r in 0..6u32 {
            let rec0 = list + 0x74 * r;
            let b = rec0 + 0x28;
            let v = case as u32 * 6 + r;
            t.write(rec0, &[0x11; 0x74]);
            for j in 0..10 {
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
    for (n, rva) in [(18u16, 0x3a10u32), (446, 0x8090)] {
        let Some(mut t) = twin(n) else { return };
        let at = t.at(rva);
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
        let pa = t.img.rel32_target(at + 0x9d);
        let free = t.img.word(pa + 0x04);
        let tail = t.img.word(at + 0xa8);
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
            .with_xmm(&[
                Xmm::f32(0.0),
                Xmm::f32(0.0),
                Xmm::f32(0.0),
                Xmm::f32(scale),
            ]);
            let args = [k as u32 * 2 + 1, 0x0a00 + k as u32, 0xdead, 0x0b00 + k as u32];
            t.check(at, c, &args, Eax::Checked, |g, img, fr| {
                Ret::eax(s4_misc::start_object_at(
                    g,
                    img,
                    fr.at,
                    fr.edx,
                    fr.xmm_f32(3),
                    fr.arg(g, 0),
                    fr.arg(g, 1),
                    fr.arg(g, 3),
                ))
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
