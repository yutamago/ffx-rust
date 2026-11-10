//! Round 36's ports (`ffx_magic::r36`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    clippy::type_complexity
)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_owna;
mod magic_r11;
#[path = "magic/ownb.rs"]
mod ownb;

use ffx_magic::{Ret, r36};
use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use ownb::rec;

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    magic_h6::scrub(&mut t);
    Some(t)
}

fn rf(m: &dyn StubMem, at: u32) -> f32 {
    f32::from_bits(m.r32(at))
}

fn wf(m: &mut dyn StubMem, at: u32, v: f32) {
    m.w32(at, v.to_bits());
}

fn kf(t: &Twin, f: u32, o: u32) -> f32 {
    f32::from_bits(t.u32(t.img.word(f + o)))
}

fn setf(t: &mut Twin, at: u32, v: f32) {
    t.set_u32(at, v.to_bits());
}

/// `n` floats at `at`, varied by `k` (small, never NaN).
fn floats(t: &mut Twin, at: u32, n: u32, k: u32) {
    for i in 0..n {
        let v = ((i * 37 + k * 11) % 41) as f32 * 0.125 - 2.5;
        setf(t, at + 4 * i, v);
    }
}

/// Every slot answering `first word ^ (slot << 20)`.
fn all_stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
}

/// A call log of its own: the slot, `n` argument words and `deref` words
/// at the argument `deref_arg`.
const PLOG: u32 = SCRATCH + 0x2_8000;

fn logp(t: &mut Twin, slot: u32, n: usize, deref: Option<(usize, u32)>, ret: u32) {
    t.stub(slot, move |m, a| {
        let mut at = m.r32(PLOG);
        let mut put = |m: &mut dyn StubMem, v: u32| {
            m.w32(PLOG + 4 + at % 0x3ff0, v);
            at += 4;
        };
        put(m, 0x5300_0000 | slot);
        for w in &a[..n] {
            put(m, *w);
        }
        if let Some((i, k)) = deref {
            for j in 0..k {
                let v = m.r32(a[i] + 4 * j);
                put(m, v);
            }
        }
        m.w32(PLOG, at);
        ret ^ a[0]
    });
}

/// The matrix and point slots the turning followers and the cameras
/// call (each writes what it makes from every word it reads).
fn camera_stubs(t: &mut Twin) {
    all_stubs(t);
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    for s in [330u32, 328, 329] {
        rec(t, s, 3, move |m, a| {
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) * 0.5 + (s as f32) * 0.001 + i as f32 * 0.25;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    // (out, matrix, point): four floats
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[2] + 4 * i) + rf(m, a[1] + 4 * (i + 4)) * 0.5 + i as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    // (point out, end, start, t)
    rec(t, 326, 4, |m, a| {
        let x = f32::from_bits(a[3]);
        for i in 0..4 {
            let v = rf(m, a[2] + 4 * i) * (1.0 - x) + rf(m, a[1] + 4 * i) * x;
            wf(m, a[0] + 4 * i, v);
        }
        0x326
    });
    logp(t, 284, 3, Some((2, 4)), 0x0284_0000);
    logp(t, 285, 2, None, 0x0285_0000);
    logp(t, 286, 2, None, 0x0286_0000);
    // the jitter's random spread
    rec(t, 314, 1, |_, a| 0x0314_0000 ^ a[0]);
    t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * 0.375 - 0.0625);
}

/// The answers of slot 253 (one per call, in turn).
const ANS: u32 = SCRATCH + 0x2_7000;

/// `magic_0515` 0x10001560 (`r36::cameras_0515`): every state of the
/// eye and aim tracks (eased, steady, the four followers, ended or
/// not), of the roll (key track, blend wrapped either way or not, ended)
/// and of the field of view (time, key track, quartic held or not,
/// ended); the jitter on and off; drawn as they are, turned or not
/// drawn; slot 253 saying 0.
#[test]
fn cameras_0515() {
    for n in [515u16, 519, 611, 667, 668] {
        cameras_in(n);
    }
}

fn cameras_in(n: u16) {
    let Some(mut t) = twin(n) else { return };
    let f = t.at(if n == 519 { 0x1690 } else { 0x1560 });
    camera_stubs(&mut t);
    t.stub(253, |m, _| {
        let k = m.r32(ANS);
        m.w32(ANS, k + 1);
        m.r32(ANS + 4 + 4 * (k % 3))
    });
    let c0 = t.img.word(f + 0x14);
    let (k2, k4) = (kf(&t, f, 0x3dc), kf(&t, f, 0x3f8));
    for k in 0..12u32 {
        t.set_u32(ANS, 0);
        for j in 0..3u32 {
            let i = 3 * k + j;
            let c = c0 + 0x190 * j;
            floats(&mut t, c - 0x108, 0x190 / 4, i);
            let sc = SCRATCH + 0x1000 + 0x400 * j;
            // the eye and aim tracks
            for (tf, state, alt, keys) in [
                (c - 0x18, i % 8, (i / 8) % 3, sc),
                (c - 0x58, (3 * i + 1) % 8, (i / 4) % 2, sc + 0x100),
            ] {
                floats(&mut t, keys, 0x40, i + 5);
                t.set_u32(tf + 0x20, keys);
                t.set_u32(tf + 0x24, keys + 0xc0);
                t.set_u32(tf + 0x28, keys + 0xe0);
                for (w, x) in (0u32..).zip([0.0f32, 0.75, 1.5, 2.25, 3.0]) {
                    setf(&mut t, keys + 0xe0 + 4 * w, x);
                }
                t.set_u32(tf + 0x38, 5);
                t.set_u32(tf + 0x3c, state);
                let (time, step, left) = match (state, alt) {
                    (1, 0) => (0.25, 0.25, 3),
                    (1, 2) => (0.75, 0.25, 3),
                    (1, _) => (0.9, 0.25, 3),
                    (_, 0) => (0.5, 0.125, 3),
                    _ => (0.375, 0.25, 1),
                };
                setf(&mut t, tf + 0x2c, time);
                setf(&mut t, tf + 0x30, step);
                t.set_u32(tf + 0x34, left);
            }
            // the roll
            let (state, alt) = (i % 3, (i / 3) % 6);
            let rk = sc + 0x200;
            floats(&mut t, rk, 8, i + 9);
            t.set_u32(c + 0x28, rk);
            // a single frame (alts 4, 5): the blend is its end exactly
            t.set_u32(c + 0x3c, if alt >= 4 { 1 } else { 5 });
            t.set_u32(c + 0x40, state);
            t.set_u32(c + 0x38, [1, 3, 3, 3, 2, 2][alt as usize]);
            let (from, to) = match (state, alt) {
                (1, 1) => (k2 + 1.5, k2 + 0.5),
                (1, 2) => (k4 - 1.25, k4 - 0.5),
                (1, 4) => (0.5, k2),
                (1, 5) => (0.5, k4),
                (_, 4) => (0.5, k2 + 0.001),
                (1, _) => (0.5, -1.0),
                _ => (1.25, 0.5),
            };
            setf(&mut t, c + 0x2c, from);
            setf(&mut t, c + 0x30, to);
            // the field of view
            let (state, alt) = ((i + 1) % 4, (i / 4) % 3);
            let fk = sc + 0x240;
            floats(&mut t, fk, 8, i + 13);
            let wt = sc + 0x280;
            let big = if alt == 1 { 3.0 } else { 0.25 };
            for w in 0..5 {
                setf(&mut t, wt + 4 * w, big * (w + 1) as f32);
            }
            t.set_u32(c + 0x44, fk);
            t.set_u32(c + 0x48, wt);
            setf(&mut t, c + 0x4c, 0.25);
            setf(&mut t, c + 0x50, 0.125);
            t.set_u32(c + 0x58, if alt == 0 { 1 } else { 4 });
            t.set_u32(c + 0x5c, 4);
            t.set_u32(c + 0x60, state);
            // the jitter, the draw flags
            t.set_u32(c + 0x6c, i % 3);
            t.set_u32(c + 0x70, (i + 1) % 2);
            t.set_u8(c + 0x80, [1u8, 0, 5][(i % 3) as usize]);
            t.set_u8(c + 0x81, 0x5a);
            t.set_u32(
                c + 0x7c,
                u32::from(i % 4 != 3) * (1 + u32::from(i % 5 == 4)),
            );
            t.set_u8(c + 0x82, ((i / 2) % 2) as u8);
            t.set_u32(ANS + 4 + 4 * j, u32::from(i % 7 != 5) * (0x40 + i));
        }
        t.set_u32(ANS, 0);
        t.set_u32(t.img.word(f + 0x529), 0x0c0a_0000 + k);
        ownb::dirty_stack(&mut t, 0x1560 + k, &[]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(r36::cameras_0515(g, img, fr))
        });
    }
    assert!(t.u32(PLOG) > 0x100, "magic_{n:04}: the cameras' draws");
}

/// Answers of [`draw_stubs`]' slots 773 and 886 (one word each, in turn
/// per call: `ANS2 + 4 + 4 * (k % 4)` for 773, `ANS2 + 0x20 + ...` for
/// 886; counters at `ANS2` and `ANS2 + 0x1c`).
const ANS2: u32 = SCRATCH + 0x2_7100;

/// The draw slots of the trails and streaks: matrices made and copied,
/// points placed, pairs and buffers, the draws logged with their
/// structs.
fn draw_stubs(t: &mut Twin) {
    all_stubs(t);
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    rec(t, 298, 1, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, i as f32 * 0.5 - 1.0);
        }
        0x298
    });
    // (V, x, R, R + 0x40, byte): the point
    rec(t, 270, 5, |m, a| {
        let x = f32::from_bits(a[1]);
        for i in 0..3 {
            let v = rf(m, a[2] + 4 * i) * x + rf(m, a[3] + 4 * i) + (a[4] & 7) as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x270
    });
    // (out row, camera, V)
    rec(t, 297, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[2] + 4 * i) * 2.0 + (a[1] & 0xf) as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x297
    });
    // (out, M): the matrix copied, mixed
    rec(t, 230, 2, |m, a| {
        for i in 0..16 {
            let v = rf(m, a[1] + 4 * i) + i as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x230
    });
    rec(t, 772, 2, |_, a| 0x0772_0000 ^ a[0] ^ a[1]);
    t.stub_edx(772, |_, a| 0x0e72_0000 ^ a[0]);
    t.stub(773, |m, a| {
        let k = m.r32(ANS2);
        m.w32(ANS2, k + 1);
        let r = m.r32(ANS2 + 4 + 4 * (k % 4));
        m.w32(ANS2 + 0x40 + 4 * (k % 8), a[0] ^ a[1]);
        r
    });
    t.stub(886, |m, a| {
        let k = m.r32(ANS2 + 0x1c);
        m.w32(ANS2 + 0x1c, k + 1);
        m.w32(ANS2 + 0x60 + 4 * (k % 8), a[0] ^ a[2] ^ a[3].rotate_left(5));
        m.r32(ANS2 + 0x20 + 4 * (k % 4))
    });
    rec(t, 937, 2, |_, a| 0x0937_0000 ^ a[1]);
    rec(t, 870, 2, |_, a| 0x0870_0000 ^ a[0]);
    rec(t, 315, 1, |_, a| 0x0315_0000 ^ a[0]);
    rec(t, 776, 1, |_, a| 0x0776_0000 ^ a[0]);
    // the draws: the pair, the model, the draw struct's fields
    for (s, n) in [(908u32, 4usize), (875, 4), (874, 5)] {
        logd(t, s, n);
    }
}

/// Logs a draw call's `n` words and the fields its draw struct (the
/// fourth word) gets: the u16 `+2`, the words `+4`, `+8`, `+0x10`,
/// `+0x1c` and the byte `+0x18` (the others are never written: the
/// callees don't read them).
fn logd(t: &mut Twin, slot: u32, n: usize) {
    t.stub(slot, move |m, a| {
        let mut at = m.r32(PLOG);
        let mut put = |m: &mut dyn StubMem, v: u32| {
            m.w32(PLOG + 4 + at % 0x3ff0, v);
            at += 4;
        };
        put(m, 0x5400_0000 | slot);
        for w in &a[..n] {
            put(m, *w);
        }
        let d = a[3];
        let fields = [
            m.r32(d) >> 16,
            m.r32(d + 4),
            m.r32(d + 8),
            m.r32(d + 0x10),
            m.r32(d + 0x1c),
            m.r32(d + 0x18) & 0xff,
        ];
        for v in fields {
            put(m, v);
        }
        m.w32(PLOG, at);
        (slot << 16) ^ a[0]
    });
}

/// A model for the streaks and trails: the count `u16 +2`, the parts'
/// offset `+4` and the kind byte there.
fn model(t: &mut Twin, at: u32, n: u16, kind: u8) {
    t.write(at, &[0x33; 0x40]);
    t.set_u16(at + 2, n);
    t.set_u32(at + 4, 0x20);
    t.set_u8(at + 0x24, kind);
}

/// `magic_0515` 0x10002fa0 (`r36::trails_0515`): plain and stepped
/// trails, the limit passed or not at the start, the colour running
/// out, a shrinking scale reaching 0 or growing, more than 0x40 copies,
/// no buffer, slot 886 refusing (the early end), no record at all.
#[test]
fn trails_0515() {
    for (n, at) in [
        (515u16, 0x2fa0u32),
        (611, 0x2fd0),
        (667, 0x2e50),
        (668, 0x2e70),
    ] {
        trails_in(n, at);
    }
}

fn trails_in(n: u16, at: u32) {
    let Some(mut t) = twin(n) else { return };
    let f = t.at(at);
    draw_stubs(&mut t);
    let lock = SCRATCH + 0x2_7400;
    t.set_u32(t.img.word(f + 0xc7), lock);
    let (e0, end) = (t.img.word(f + 0x5b), t.img.word(f + 0x4f1));
    let recs = (end - e0) / 0xa8;
    // (stepped, num, den, count, colour, colour step, scale, d, dd, step2)
    let kinds: [(bool, i16, i16, u8, u32, u32, f32, f32, f32, f32); 8] = [
        (
            false,
            3,
            4,
            2,
            0x60ff_ffff,
            0x2000_0000,
            1.5,
            -0.5,
            0.125,
            0.0,
        ),
        (
            true,
            3,
            4,
            2,
            0x60ff_ffff,
            0x1800_0000,
            1.5,
            0.25,
            -0.125,
            0.25,
        ),
        (false, 9, 2, 1, 0x40ff_ffff, 0x1000_0000, 1.0, 0.5, 0.0, 0.0),
        (false, 1, 1, 1, 0x20ff_ffff, 0, 2.0, -0.75, -0.5, 0.0),
        (false, 1, 1, 1, 0x80ff_ffff, 0, 1.0, 0.0, 0.0, 0.0),
        (
            true,
            5,
            4,
            3,
            0x30ff_ffff,
            0x1000_0000,
            0.5,
            0.5,
            0.5,
            0.375,
        ),
        (true, 1, 1, 0, 0x40ff_ffff, 0, 1.0, 0.0, 0.0, 0.0125),
        (
            false,
            -1,
            2,
            2,
            0x40ff_ffff,
            0x0100_0000,
            1.0,
            0.0,
            0.0,
            0.0,
        ),
    ];
    for k in 0..7u32 {
        for r in 0..recs {
            let e = e0 + 0xa8 * r;
            floats(&mut t, e - 0xa4, 0x2a, k + r);
            t.set_u32(e - 0x24, 0);
        }
        let active: &[u32] = match k {
            0 => &[0, 1, 2],
            1 => &[3, 4],
            2 => &[5, 6, 7, 8],
            3 => &[],
            4 => &[2, 9],
            5 => &[recs - 1, 10],
            _ => &[1, 11, 12],
        };
        for (j, &r) in (0u32..).zip(active) {
            let e = e0 + 0xa8 * r;
            let (stepped, num, den, count, colour, cstep, scale, d, dd, step2) =
                kinds[((k + j * 3) % 8) as usize];
            let mdl = SCRATCH + 0x6000 + 0x80 * r;
            model(&mut t, mdl, 3 + j as u16, 0x41 + j as u8);
            t.set_u32(e - 0x24, mdl);
            t.set_u8(e - 0x20, 0x11 + j as u8);
            t.set_u8(e - 0x1f, count);
            t.set_u8(e - 0x1e, u8::from(stepped) * (1 + j as u8));
            t.set_u16(e - 0x1c, num.cast_unsigned());
            t.set_u16(e - 0x1a, den.cast_unsigned());
            t.set_u32(e - 0x18, colour);
            t.set_u32(e - 0x14, cstep);
            setf(&mut t, e - 0x10, scale);
            setf(&mut t, e - 0xc, d);
            setf(&mut t, e - 0x8, dd);
            setf(&mut t, e - 0x4, step2);
            t.set_u32(e, 3 * j);
        }
        // slot 773's buffers (one 0 in turn), slot 886's answers (0 in case 6)
        t.set_u32(ANS2, 0);
        t.set_u32(ANS2 + 0x1c, 0);
        for i in 0..4 {
            t.set_u32(
                ANS2 + 4 + 4 * i,
                if (i + k) % 3 == 2 { 0 } else { 0xb0f0_0000 + i },
            );
            t.set_u32(ANS2 + 0x20 + 4 * i, u32::from(k != 6 || i != 1));
        }
        t.set_u32(lock, 0x1c0c_0000 + k);
        t.set_u32(t.img.word(f + 0x1f5), 0x0ca0_0000 + k);
        ownb::dirty_stack(&mut t, 0x2fa0 + k, &[]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(r36::trails_0515(g, img, fr))
        });
    }
    assert!(t.u32(PLOG) > 0x200, "magic_{n:04}: the trails' draws");
}

/// `magic_0515` 0x10003520 (`r36::streaks_0515`): records with and
/// without a buffer, the time wrapping past 1, no copies, the first
/// record's slot 315, the last record.
#[test]
fn streaks_0515() {
    for (n, at) in [
        (515u16, 0x3520u32),
        (611, 0x3550),
        (667, 0x33d0),
        (668, 0x33f0),
    ] {
        streaks_in(n, at);
    }
}

fn streaks_in(n: u16, at: u32) {
    let Some(mut t) = twin(n) else { return };
    let f = t.at(at);
    draw_stubs(&mut t);
    let lock = SCRATCH + 0x2_7400;
    t.set_u32(t.img.word(f + 0x231), lock);
    let e0 = t.img.word(f + 0xa6);
    // (count, num, den)
    let kinds: [(u32, u32, u32); 5] = [(3, 1, 4), (2, 3, 4), (0, 1, 2), (4, 7, 8), (1, 9, 8)];
    for k in 0..4u32 {
        t.write(e0 - 0x18, &vec![0u8; 0x200 * 0x3c]);
        let active: &[u32] = match k {
            0 => &[0, 5, 0x1ff],
            1 => &[2],
            2 => &[],
            _ => &[7, 8, 9, 0x100],
        };
        for (j, &r) in (0u32..).zip(active) {
            let e = e0 + 0x3c * r;
            floats(&mut t, e - 0x18, 0xf, k * 7 + j);
            let (cnt, num, den) = kinds[((k + j) % 5) as usize];
            let mdl = SCRATCH + 0x6000 + 0x80 * j;
            model(&mut t, mdl, 2, 0x51 + j as u8);
            t.set_u32(e + 8, mdl);
            t.set_u32(e + 0xc, cnt);
            t.set_u32(e + 0x10, num);
            t.set_u32(e + 0x14, den);
            t.set_u32(e + 0x18, 0x7f40_2010 + j);
            setf(&mut t, e + 0x1c, 0.5 + j as f32);
            t.set_u32(e + 0x20, j);
        }
        t.set_u32(ANS2, 0);
        for i in 0..4 {
            t.set_u32(
                ANS2 + 4 + 4 * i,
                if (i + k) % 2 == 1 { 0 } else { 0xb0f0_0000 + i },
            );
        }
        t.set_u32(lock, 0x1c0c_0000 + k);
        t.set_u32(t.img.word(f + 0x16f), 0x0ca0_0000 + k);
        ownb::dirty_stack(&mut t, 0x3520 + k, &[]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(r36::streaks_0515(g, img, fr))
        });
    }
    assert!(t.u32(PLOG) > 0x100, "magic_{n:04}: the streaks' draws");
}

/// `magic_0515` 0x100041f0 (`r36::draw_0515`, an entry): paused, not
/// (no pause word, a zero one), the part refresh on and off.
#[test]
fn draw_0515() {
    for (n, at) in [
        (515u16, 0x41f0u32),
        (611, 0x4220),
        (667, 0x40a0),
        (668, 0x40c0),
    ] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(at);
        draw_stubs(&mut t);
        let tr = t.img.rel32_target(f + 0x2c);
        let st = t.img.rel32_target(f + 0x31);
        let lock = SCRATCH + 0x2_7400;
        t.set_u32(t.img.word(tr + 0xc7), lock);
        t.set_u32(t.img.word(st + 0x231), lock);
        let (e0, s0) = (t.img.word(tr + 0x5b), t.img.word(st + 0xa6));
        let to = SCRATCH + 0x2_7500;
        t.set_u32(t.img.word(f + 0x8), to);
        let obj = t.img.word(f + 0x14);
        let pause = t.img.word(f + 0x1e);
        for k in 0..4u32 {
            t.set_u32(t.img.word(f + 0xd), 0x0d0d_0000 + k);
            t.set_u16(obj + 0x1e, u16::from(k == 3));
            let word = SCRATCH + 0x2_7510;
            t.set_u32(word, u32::from(k == 1) * 7);
            t.set_u32(pause, if k == 0 { 0 } else { word });
            // one trail and one streak record
            t.write(s0 - 0x18, &vec![0u8; 0x200 * 0x3c]);
            for r in 0..((t.img.word(tr + 0x4f1) - e0) / 0xa8) {
                t.set_u32(e0 + 0xa8 * r - 0x24, 0);
            }
            let e = e0 + 0xa8 * 2;
            floats(&mut t, e - 0xa4, 0x24, k);
            model(&mut t, SCRATCH + 0x6000, 2, 3);
            t.set_u32(e - 0x24, SCRATCH + 0x6000);
            t.set_u8(e - 0x1f, 1);
            t.set_u8(e - 0x1e, 0);
            t.set_u16(e - 0x1c, 1);
            t.set_u16(e - 0x1a, 2);
            t.set_u32(e - 0x18, 0x10ff_ffff);
            t.set_u32(e - 0x14, 0x0800_0000);
            let s = s0 + 0x3c * 3;
            floats(&mut t, s - 0x18, 0xf, k);
            t.set_u32(s + 8, SCRATCH + 0x6000);
            t.set_u32(s + 0xc, 2);
            t.set_u32(s + 0x10, 1);
            t.set_u32(s + 0x14, 3);
            t.set_u32(ANS2, 0);
            t.set_u32(ANS2 + 0x1c, 0);
            for i in 0..4 {
                t.set_u32(ANS2 + 4 + 4 * i, 0xb0f0_0000 + i);
                t.set_u32(ANS2 + 0x20 + 4 * i, 1);
            }
            ownb::dirty_stack(&mut t, 0x41f0 + k, &[]);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// Answers of the start-up's slots in turn: a counter at `SEQ + 0x100 *
/// i` and its words after it.
const SEQ: u32 = SCRATCH + 0x2_7800;

fn seq(t: &mut Twin, slot: u32, i: u32, n: usize) {
    let base = SEQ + 0x100 * i;
    rec(t, slot, n, move |m, _| {
        let k = m.r32(base);
        m.w32(base, k + 1);
        m.r32(base + 4 + 4 * (k % 16))
    });
}

fn set_seq(t: &mut Twin, i: u32, v: &[u32]) {
    let base = SEQ + 0x100 * i;
    t.set_u32(base, 0);
    for (j, x) in (0u32..).zip(v) {
        t.set_u32(base + 4 + 4 * j, *x);
    }
}

/// `magic_0515` 0x10004230 (`r36::start_up_0515`, an entry): every
/// scene flag on and off, the neighbours either way, no party and a
/// party of three, the leader or 0xff, targets already in a record and
/// new ones, none at all.
#[test]
fn start_up_0515() {
    for (n, at) in [(515u16, 0x4230u32), (611, 0x4260)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(at);
        all_stubs(&mut t);
        seq(&mut t, 280, 0, 0);
        seq(&mut t, 293, 1, 1);
        rec(&mut t, 168, 1, |_, a| 0x40 ^ (a[0] & 0xf));
        rec(&mut t, 294, 1, |_, a| a[0] + 0x10);
        rec(&mut t, 289, 1, |_, a| a[0]);
        t.stub_st0(289, |_, a| f64::from(a[0]) * 0.25 + 0.125);
        let party = SCRATCH + 0x2_7c00;
        let list = SCRATCH + 0x2_7c40;
        rec(&mut t, 218, 2, move |m, a| {
            let n = m.r32(SEQ + 0x100 * 2);
            m.w32(a[1], n);
            party
        });
        rec(&mut t, 288, 2, move |m, a| {
            let n = m.r32(SEQ + 0x100 * 2 + 4);
            m.w32(a[1], n);
            list
        });
        rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ (a[1] << 8) ^ a[2]);
        rec(&mut t, 312, 4, |_, a| 0x0312_0000 ^ a[0]);
        // the measure's slots: points, matrices, sizes
        rec(&mut t, 217, 2, |m, a| {
            for i in 0..4 {
                wf(m, a[1] + 4 * i, (a[0] & 0xff) as f32 * 0.5 + i as f32);
            }
            0x217
        });
        rec(&mut t, 210, 2, |m, a| {
            for i in 0..16 {
                wf(m, a[1] + 4 * i, (a[0] & 0xff) as f32 + i as f32 * 0.25);
            }
            0x210
        });
        rec(&mut t, 230, 2, |m, a| {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i);
                m.w32(a[0] + 4 * i, v);
            }
            0x230
        });
        rec(&mut t, 290, 2, |m, a| {
            for i in 0..4 {
                wf(m, a[1] + 4 * i, 0.5 + i as f32);
            }
            0x290
        });
        for (s, x) in [(169u32, 0.75f64), (251, 2.0), (216, 1.5), (277, 1.25)] {
            rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
            t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
        }
        let scenes = [
            0x0235_0000u32,
            0x0236_0000,
            0x0237_0000,
            0x0247_0000,
            0x024a_0000,
            0x018b_0046,
        ];
        for k in 0..6u32 {
            let answers: Vec<u32> = (0..6u32)
                .map(|j| {
                    if (j + k) % 2 == 0 {
                        scenes[j as usize]
                    } else {
                        scenes[j as usize] ^ 1
                    }
                })
                .collect();
            set_seq(&mut t, 0, &answers);
            set_seq(&mut t, 1, &[k % 2, (k / 2) % 2]);
            // the party count and the target count
            let (np, nt) = [(3u32, 5u32), (0, 0), (2, 4), (1, 1), (3, 6), (0, 3)][k as usize];
            t.set_u32(SEQ + 0x200, np);
            t.set_u32(SEQ + 0x204, nt);
            t.write(party, &[0x21, 0x22, 0x23, 0x24]);
            // known: the summon's records 0x40.., the party, the leader; new ones
            t.write(
                list,
                &[0x41 + (k as u8 & 1), 0x22, 0x33, 0x34, 0x33, 0x50 + k as u8],
            );
            for o in [0x10cu32, 0x115, 0x138] {
                let tm = t.img.rel32_target(f + 0x124);
                t.set_u32(t.img.word(tm + o), 0);
            }
            ownb::dirty_stack(&mut t, 0x4230 + k, &[]);
            t.check_entry(f, Call::default(), &[0x0abc_0000 + k], Eax::Checked);
        }
    }
}

/// A free ring of `n` nodes (0x20 bytes) at `at` for the task pool take
/// at `take` (its head and count globals).
fn ring(t: &mut Twin, take: u32, at: u32, n: u32) {
    for i in 0..n {
        let node = at + 0x20 * i;
        t.write(node, &[0x5a; 0x20]);
        t.set_u32(node, at + 0x20 * ((i + n - 1) % n));
        t.set_u32(node + 4, at + 0x20 * ((i + 1) % n));
    }
    t.set_u32(t.img.word(take + 0x17), at);
    t.set_u16(t.img.word(take + 0x04), n as u16);
}

/// The matrix and draw slots of `magic_0165`'s glow and sparks.
fn stubs_0165(t: &mut Twin) {
    all_stubs(t);
    for s in [723u32, 724, 235] {
        rec(t, s, 1, move |m, a| {
            for i in 0..16 {
                let v = (i as f32 + 1.0) * if s == 235 { 1.0 } else { 0.5 };
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 230, 2, |m, a| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v ^ 0x10);
        }
        0x230
    });
    rec(t, 231, 3, |m, a| {
        for i in 0..16 {
            let v = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x231
    });
    // the bone point
    rec(t, 237, 4, |m, a| {
        for i in 0..4 {
            wf(m, a[3] + 4 * i, (a[1] & 0xff) as f32 * 0.125 + i as f32);
        }
        0x237
    });
    // the sprite draw: its five words and the draw struct's first words
    logp(t, 567, 5, Some((1, 12)), 0x0567_0000);
    for s in [234u32, 248] {
        rec(t, s, 3, move |m, a| {
            for i in 0..3 {
                let v =
                    rf(m, a[1] + 4 * i) + if s == 248 { 1.0 } else { -0.5 } * rf(m, a[2] + 4 * i);
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[2] + 4 * i) + 0.25;
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    rec(t, 772, 2, |_, a| 0x7720_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    rec(t, 773, 4, |_, a| (a[3] >> 1) & 1);
    rec(t, 776, 1, |_, a| a[0] << 4);
    rec(t, 777, 1, |_, a| 0x0ff0 ^ a[0]);
    rec(t, 314, 1, |_, a| 0x0314_0000 ^ a[0]);
    t.stub_st0(314, |_, a| {
        f64::from(f32::from_bits(a[0])) * 0.375 - 0.03125
    });
}

/// `magic_0165` 0x100025b0 (`r36::glow_0165`, an entry): the fade count
/// positive (counted down), 0 and negative.
#[test]
fn glow_0165() {
    let Some(mut t) = twin(165) else { return };
    let f = t.at(0x25b0);
    stubs_0165(&mut t);
    let o = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0x1e), o);
    for (k, n) in [(0u32, 7i32), (1, 0), (2, -45), (3, 1), (4, 21)] {
        floats(&mut t, o + 0x980, 16, k);
        t.set_u32(o + 0x5f18, n.cast_unsigned());
        t.set_u32(t.img.word(f + 0x3a), 0x0e0e_0000 + k);
        t.set_u32(t.img.word(f + 0x2f9), 0x0cc0_0000 + k);
        ownb::dirty_stack(&mut t, 0x25b0 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0165` 0x10002a00 (`r36::spark_task_0165`, an entry): waiting,
/// the trail shrinking with sparks taken (and none free, none left), the
/// task's end.
#[test]
fn spark_task_0165() {
    let Some(mut t) = twin(165) else { return };
    let f = t.at(0x2a00);
    stubs_0165(&mut t);
    let o = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0x17), o);
    let tr = t.img.rel32_target(f + 0xb5);
    t.set_u32(t.img.word(tr + 0x463), SCRATCH + 0x9000);
    t.set_u32(t.img.word(tr + 0x520), 0x0cc0_0cc0);
    let table = SCRATCH + 0x8000;
    rec(&mut t, 941, 1, move |_, _| table);
    t.write(table, &[0; 0x100]);
    t.set_u16(table + 6, 3);
    let take = t.img.rel32_target(f + 0xd2);
    let pool = SCRATCH + 0xa000;
    let x = SCRATCH + 0x9100;
    t.set_u32(t.img.word(f + 0xe2), x);
    t.set_u32(x + 4, 0x4a4d_0001);
    // the end's pairs, parts and give-back
    let give = t.img.rel32_target(f + 0x2a2);
    let _ = give;
    // (wait, steps, sparks left, a free pool record)
    for (k, (wait, steps, left, free)) in [
        (5i32, 4i32, 3i32, true),
        (1, 4, 2, true),
        (0, 3, 1, false),
        (0, 6, 0, true),
        (0, 2, -1, true),
        (0, 1, 2, true),
        (1, 0, 2, true),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let tobj = o + 0x5cb8;
        t.write(tobj, &[0x33; 0x230]);
        t.set_u32(tobj, 0x0a00 + k);
        t.set_u32(tobj + 4, 1);
        t.set_u16(tobj + 8, (k % 3) as u16);
        t.set_u16(tobj + 0xa, 0x100);
        for j in 0..32u32 {
            for (i, v) in (0u32..).zip([j as f32 * 0.5, 1.0, -(j as f32), 1.0]) {
                setf(&mut t, tobj + 0xc + 16 * j + 4 * i, v);
            }
        }
        t.write(tobj + 0x20c, &[10, 200, 0, 0x80, 250, 3, 128, 0x40]);
        setf(&mut t, tobj + 0x214, 1.5);
        setf(&mut t, tobj + 0x218, 0.25);
        setf(&mut t, tobj + 0x21c, 0.75);
        t.set_u32(tobj + 0x220, k * 5);
        t.set_u32(o + 0x5f0c, wait.cast_unsigned());
        t.set_u32(o + 0x5edc, steps.cast_unsigned());
        t.set_u32(o + 0x5f10, left.cast_unsigned());
        // the spark pool: 3 records, the first free or none
        t.set_u32(t.img.word(take + 0x08), 0);
        t.set_u32(t.img.word(take + 0x0f), 3);
        t.set_u32(t.img.word(take + 0x37), pool);
        for r in 0..3u32 {
            floats(&mut t, pool + 0x74 * r, 0x1d, k + r);
            t.set_u32(pool + 0x74 * r + 0x6c, u32::from(!free || r > 0));
        }
        // the task: a lone node
        let task = SCRATCH + 0x1000 + 0x40 * k;
        t.write(task, &[0; 0x20]);
        t.set_u32(task + 8, task);
        ownb::dirty_stack(&mut t, 0x2a00 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0165` 0x10003080 (`r36::objects_0165`, an entry): the first
/// object in the list or not, the task list empty or not.
#[test]
fn objects_0165() {
    let Some(mut t) = twin(165) else { return };
    let f = t.at(0x3080);
    all_stubs(&mut t);
    magic_r11::pools(&mut t, f, 656);
    magic_r11::object_slots(&mut t);
    for call in [0x5du32, 0xd5] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(&mut t, start);
    }
    let o = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0x3), o);
    let p = SCRATCH + 0x9100;
    t.set_u32(t.img.word(f + 0x20), p);
    let list = t.img.word(f + 0x12);
    let (head, tail) = (t.img.word(f + 0x1cc), t.img.word(f + 0x1b7));
    for k in 0..4u32 {
        floats(&mut t, o, 0x1800, k);
        t.set_u32(o + 0x5f1c, k % 3);
        // the list: its node holds the object or another one
        let node = SCRATCH + 0x9200;
        t.write(node, &[0; 0x20]);
        t.set_u32(list, node);
        t.set_u32(node + 8, if k % 2 == 0 { o + 0x260 } else { o + 0x100 });
        magic_r11::pools(&mut t, f, 656);
        let old = SCRATCH + 0x9300;
        t.write(old, &[0x41; 0x20]);
        t.set_u32(head, if k < 2 { old } else { 0 });
        t.set_u32(tail, if k < 2 { old } else { 0 });
        ownb::dirty_stack(&mut t, 0x3080 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}
