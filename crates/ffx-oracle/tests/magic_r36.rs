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
            m.r32(d + 0xc),
            m.r32(d + 0x10),
            m.r32(d + 0x1c),
            m.r32(d + 0x18),
            // the lock word while it draws
            m.r32(SCRATCH + 0x2_7400),
        ];
        for v in fields {
            put(m, v);
        }
        // the matrix buffer it draws with
        let buf = m.r32(d + 0x10);
        for i in 0..16 {
            let v = m.r32(buf + 4 * i);
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
        // a model word past the last record (the loop must stop before it)
        t.set_u32(end - 0x24, SCRATCH + 0x6000);
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
        // the globals it writes, junk first (u16 stores leave the upper half)
        let junk_at = |t: &mut Twin, op: u32, k: u32| {
            let at = t.img.word(f + op);
            t.set_u32(at, 0x5a5a_5a5a ^ op ^ k);
        };
        let blocks = t.img.word(f + 0x6df);
        for k in 0..6u32 {
            for op in (0x54fu32..=0x60f)
                .step_by(6)
                .chain((0x652..=0x6be).step_by(6))
                .chain([
                    0x13, 0x61a, 0x64a, 0x6c3, 0x6c8, 0x6ea, 0x6f4, 0x778, 0x77e, 0x788,
                ])
            {
                junk_at(&mut t, op, k);
            }
            for i in 0..3 {
                t.set_u32(blocks + 0x190 * i, 0x0b0b_0000 + i);
            }
            t.set_u32(t.img.word(f + 0x6d2), k % 3);
            for i in 0..0x100 {
                t.set_u32(t.img.word(f + 0x702) + 0x4c * i, 0x0c0c_0000 + i);
            }
            let (tr, tr_end) = (t.img.word(f + 0x71d), t.img.word(f + 0x734));
            let mut a = tr;
            while a < tr_end {
                t.set_u32(a - 0x24, 0x0d0d_0000 + a);
                t.set_u32(a, 0x0e0e_0000 + a);
                a += 0xa8;
            }
            for i in 0..0x200 {
                let a = t.img.word(f + 0x740) + 0x3c * i;
                t.set_u32(a - 0x18, 0x0f0f_0000 + i);
                t.set_u32(a, 0x0f1f_0000 + i);
            }
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
    // the draw struct (0x50 bytes), its colour and its sprite matrix
    t.stub(567, |m, a| {
        let mut at = m.r32(PLOG);
        let mut put = |m: &mut dyn StubMem, v: u32| {
            m.w32(PLOG + 4 + at % 0x3ff0, v);
            at += 4;
        };
        put(m, 0x5300_0567);
        for w in &a[..5] {
            put(m, *w);
        }
        let d = a[1];
        for i in 0..0x14 {
            let v = m.r32(d + 4 * i);
            put(m, v);
        }
        let (c, mx) = (m.r32(d + 0x24), m.r32(d + 0x28));
        for i in 0..4 {
            let v = m.r32(c + 4 * i);
            put(m, v);
        }
        for i in 0..16 {
            let v = m.r32(mx + 4 * i);
            put(m, v);
        }
        m.w32(PLOG, at);
        0x0567_0000 ^ a[0]
    });
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
    // the point turned: its words and every word of the matrix mixed in
    rec(t, 229, 3, |m, a| {
        let mix = (0..16).fold(0u32, |x, i| x.rotate_left(5) ^ m.r32(a[1] + 4 * i));
        for i in 0..4 {
            let v = rf(m, a[2] + 4 * i) + 0.25 + (mix.rotate_left(i) & 0xff) as f32;
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
    // a spread that changes per call (its count at SCRATCH + 0x2_7600)
    t.stub_st0(314, |m, a| {
        let k = m.r32(SCRATCH + 0x2_7600);
        m.w32(SCRATCH + 0x2_7600, k + 1);
        f64::from(f32::from_bits(a[0])) * 0.375 - 0.03125 + f64::from(k % 7) * 0.125
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
        // the model objects' flags: bit 0 set (cleared for the one at `O + 0x5f1c`)
        for j in 0..4 {
            t.set_u16(o + 0x124 + 0x130 * j, 0xffff);
        }
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

/// `magic_0098` 0x10002160 (`r36::fade_step_0098`, an entry) and its
/// texture (0x10001b00, `r36::texture_0098`): the scene running or not,
/// either step, the level held at 0 and at the top, ending on the
/// constant; the texture's widths, slot 885 answering 0 and 1, offsets of
/// either sign, colour sums below 0, inside and above their bound.
#[test]
fn fade_step_0098() {
    let Some(mut t) = twin(98) else { return };
    let f = t.at(0x2160);
    let x = t.img.rel32_target(f + 0x8a);
    let w = |t: &Twin, o: u32| t.img.word(x + o);
    all_stubs(&mut t);
    let tex = SCRATCH + 0x7000;
    rec(&mut t, 719, 3, move |_, _| tex);
    for (s, k) in [(235u32, 1usize), (825, 1), (38, 3), (275, 1)] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    rec(&mut t, 879, 10, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[4] + 4 * i);
            m.w32(SCRATCH + 0x7c00 + 4 * i, v);
        }
        0x879
    });
    let answer = SCRATCH + 0x718;
    rec(&mut t, 885, 7, move |m, a| {
        for i in 0..8 {
            m.w32(a[6] + 4 * i, 0x0885_0000 + i);
        }
        m.r32(answer)
    });
    let on = SCRATCH + 0x71c;
    t.stub(253, move |m, _| m.r32(on));
    let lock = SCRATCH + 0x7f0;
    t.set_u32(w(&t, 0x16b), lock);
    t.set_u32(lock, 0x00a1_0000);
    let colours = w(&t, 0x230);
    let level = t.img.word(f + 0x11);
    let (top, end) = (kf(&t, f, 0x48), kf(&t, f, 0x99));
    // (scene on, flag, level, texture lo/hi, x, y, colour bytes, answer)
    let cases: [(u32, u32, f32, u32, u32, i32, i32, [u8; 8], u32); 6] = [
        (0, 0, 0.5, 0, 0, 0, 0, [0; 8], 0),
        (
            1,
            1,
            0.75,
            0x1234_5678,
            3,
            0x0012_3456,
            -0x0003_2100,
            [0x10, 0x20, 0x30, 0x40, 0x80, 0xff, 0x7f, 0x01],
            1,
        ),
        (
            1,
            0,
            -2.0,
            0xbfff_ffff,
            1,
            -0x0abc_def0,
            0x0765_4321,
            [0xff, 0x00, 0x90, 0xfe, 0x81, 0x7f, 0x05, 0x00],
            1,
        ),
        (
            0,
            0,
            top + 1.0,
            0x0400_0000,
            0x10,
            0x100,
            -0x100,
            [1, 2, 3, 4, 5, 6, 7, 8],
            1,
        ),
        (0, 1, end, 0x0800_0000, 2, 5, 7, [9; 8], 1),
        (1, 0, f32::NAN, 0, 0, 0, 0, [0; 8], 0),
    ];
    for (k, (run, flag, lv, lo, hi, xo, yo, bytes, ans)) in (0u32..).zip(cases) {
        t.set_u32(on, run);
        t.set_u32(t.img.word(f + 0x1b), flag);
        setf(&mut t, level, lv);
        t.set_u32(tex, lo);
        t.set_u32(tex + 4, hi);
        t.set_u32(w(&t, 0x1d8), xo.cast_unsigned());
        t.set_u32(w(&t, 0x1ff), yo.cast_unsigned());
        t.write(colours, &bytes);
        t.set_u32(answer, ans);
        for o in [0x37u32, 0x7f, 0x85, 0x99, 0xa3, 0xad, 0xb3] {
            t.set_u32(w(&t, o), 0x00ab_0000 + o + k);
        }
        ownb::dirty_stack(&mut t, 0x2160 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0098` 0x10002ba0 (`r36::start_up_0098`, an entry): the summon
/// and the party's id in the same list (either) or not (the two follows),
/// slot 367's flag set or not (the two blocks).
#[test]
fn start_up_0098() {
    let Some(mut t) = twin(98) else { return };
    let f = t.at(0x2ba0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    all_stubs(&mut t);
    rec(&mut t, 803, 2, |_, a| 0x0400_0000 ^ a[1]);
    let tex = SCRATCH + 0x7000;
    rec(&mut t, 719, 3, move |_, a| tex + 0x40 * (a[2] & 7) + a[1]);
    let party = SCRATCH + 0x7400;
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 2);
        party
    });
    // the id lists: 3 bytes each
    let (l9, l11) = (SCRATCH + 0x7500, SCRATCH + 0x7540);
    rec(&mut t, 288, 2, move |m, a| {
        m.w32(a[1], 3);
        if a[0] == 9 { l9 } else { l11 }
    });
    let id = SCRATCH + 0x100;
    rec(&mut t, 390, 1, move |m, _| m.r32(id));
    rec(&mut t, 215, 1, |_, a| a[0] ^ 0x0215_0000);
    for s in [19u32, 364, 394] {
        rec(&mut t, s, 1, move |_, a| a[0] ^ (s << 16));
        t.stub_st0(s, move |_, a| {
            f64::from(a[0] & 0xff) * 0.37 + f64::from(s) * 0.001
        });
    }
    rec(&mut t, 210, 2, |m, a| {
        for i in 0..16 {
            wf(m, a[1] + 4 * i, 1.0 + i as f32 * 0.75);
        }
        0x210
    });
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x325
    });
    rec(&mut t, 232, 2, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) * 0.5;
            wf(m, a[0] + 4 * i, v);
        }
        0x232
    });
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(SCRATCH + 0x700 + 4 * i);
            m.w32(a[2] + 4 * i, v);
        }
        0x268
    });
    rec(&mut t, 248, 3, |m, a| {
        for i in 0..3 {
            let v = m.r32(a[2] + 4 * i);
            m.w32(SCRATCH + 0x3800 + 4 * i, v);
        }
        0x248
    });
    rec(&mut t, 217, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[1] + 4 * i, a[0] ^ (0x0300_0000 + i));
        }
        0x217
    });
    rec(&mut t, 234, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
            m.w32(a[0] + 4 * i, v & 0x3fff_ffff);
        }
        0x234
    });
    rec(&mut t, 266, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i) ^ 0x0100;
            m.w32(a[0] + 4 * i, v);
        }
        0x266
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[2] + 4 * i) ^ m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x229
    });
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    rec(&mut t, 231, 3, |m, a| {
        let v: Vec<u32> = (0..16)
            .map(|i| m.r32(a[2] + 4 * i) ^ m.r32(a[1] + 4 * i).rotate_left(7))
            .collect();
        for (i, x) in (0u32..).zip(v) {
            m.w32(a[0] + 4 * i, x);
        }
        0x231
    });
    rec(&mut t, 389, 2, |_, a| a[1] ^ a[0]);
    let flag = SCRATCH + 0x104;
    rec(&mut t, 367, 8, move |m, a| {
        let v = m.r32(flag);
        m.w32(a[2], v);
        0x367
    });
    let lock = SCRATCH + 0x200;
    t.set_u32(w(&t, 0x136), lock);
    t.set_u32(lock, 0x1c0c_0001);
    let fl = t.img.rel32_target(f + 0x331);
    t.set_u32(t.img.word(fl + 0x16), 0x00e0_0000);
    t.set_u32(t.img.word(t.at(0x11a0) + 0x0c), 0x44);
    let pf = t.img.rel32_target(f + 0x347);
    t.set_u32(t.img.word(pf + 0x16), 0x00e0_0000);
    t.set_u32(t.img.word(pf + 0x4f), 0x0044_0000);
    t.set_u32(t.img.word(pf + 0x66), 0x0066_0000);
    t.set_u32(t.img.word(pf + 0xa6), 1.25f32.to_bits());
    // (summon in list 9, id in list 9, flag)
    for (k, (s9, i9, fl_v)) in (0u32..).zip([
        (true, true, 1u32),
        (false, false, 0),
        (true, false, 3),
        (false, true, 0x10),
    ]) {
        let summon = 0x31 + k;
        rec(&mut t, 168, 1, move |_, _| summon);
        let pid = 0x51 + k as u8;
        t.write(party, &[pid, 0x22]);
        let mut a = [0x70u8, 0x71, 0x72];
        let mut b = [0x73u8, 0x74, 0x75];
        if s9 {
            a[1] = summon as u8;
        } else {
            b[0] = summon as u8;
        }
        if i9 {
            a[2] = pid;
        } else {
            b[2] = pid;
        }
        t.write(l9, &a);
        t.write(l11, &b);
        t.set_u32(flag, fl_v);
        t.set_u32(id, [0x1015u32, 0x1077, 0x1000, 0x1127][k as usize]);
        t.set_u32(SCRATCH + 0x704, (10.75f32 + k as f32).to_bits());
        t.set_u32(SCRATCH + 0x700, 0x4000_0000);
        for op in [
            0x284u32, 0x2a3, 0x2b7, 0x3ba, 0x3d9, 0x3f8, 0x417, 0x436, 0x455, 0x474, 0x493, 0x4b2,
            0x4d4, 0x4f0, 0x50f, 0x52e, 0x54d, 0x56c,
        ] {
            let at = w(&t, op);
            setf(&mut t, at, (op & 0xff) as f32 * 0.125 - 7.0 + k as f32);
        }
        // the blocks the flag picks, junk first
        for op in [0x5c7u32, 0x641] {
            let at = w(&t, op);
            t.write(at, &[0x3d; 0x110]);
        }
        ownb::dirty_stack(&mut t, 0x2ba0 + k, &[]);
        t.check_entry(f, Call::default(), &[0x0abc_0000 + k], Eax::Checked);
    }
}

/// `magic_0284` 0x10001340 (`r36::start_up_0284`, an entry) and its load
/// step (0x10001210, `r36::load_0284`): the facing wrapped either way or
/// not, the ring objects (all fourteen, or the pool running out), the
/// smoke objects per target (the actor's own and a character's point),
/// the model by either slot, the task list empty or not.
#[test]
fn start_up_0284() {
    let Some(mut t) = twin(284) else { return };
    let f = t.at(0x1340);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    all_stubs(&mut t);
    magic_r11::object_slots(&mut t);
    magic_r11::pools(&mut t, f, 2132);
    let start = t.img.rel32_target(f + 0x26d);
    magic_r11::object_start_auto(&mut t, start);
    // the actors' points and heights, the angles, the turns
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], (a[0] & 0xf) as f32 * 0.5 + i as f32);
        }
        0x262
    });
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            wf(
                m,
                a[2] + 4 * i,
                (a[0] & 0xf) as f32 + 0.25 * i as f32 + a[1] as f32,
            );
        }
        0x268
    });
    let facing = SCRATCH + 0x110;
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |m, _| f64::from(rf(m, facing)));
    rec(&mut t, 404, 1, |_, a| 0x0404_0000 ^ a[0]);
    // the angles: by call, either sign (the yaw wrapped both ways)
    t.stub_st0(404, |m, a| {
        let k = m.r32(SCRATCH + 0x2_7608);
        m.w32(SCRATCH + 0x2_7608, k + 1);
        f64::from(f32::from_bits(a[0])) * 0.25 + [0.6, -3.0, 3.0, -0.2][(k % 4) as usize]
    });
    rec(&mut t, 314, 1, |_, a| 0x0314_0000 ^ a[0]);
    t.stub_st0(314, |m, a| {
        let k = m.r32(SCRATCH + 0x2_7600);
        m.w32(SCRATCH + 0x2_7600, k + 1);
        f64::from(f32::from_bits(a[0])) * 0.375 + f64::from(k % 5) * 0.125
    });
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    for s in [328u32, 329] {
        rec(&mut t, s, 3, move |m, a| {
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) * 0.5 + (a[2] & 0xff) as f32 + s as f32 * 0.01;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    for s in [229u32, 248] {
        rec(&mut t, s, 3, move |m, a| {
            for i in 0..4 {
                let v = rf(m, a[2] + 4 * i) + rf(m, a[1] + 4 * i) * 0.25 + s as f32 * 0.001;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(&mut t, 324, 0, |m, _| {
        let s = m
            .r32(SCRATCH + 0x2_7604)
            .wrapping_mul(1_103_515_245)
            .wrapping_add(12345);
        m.w32(SCRATCH + 0x2_7604, s);
        s >> 8
    });
    // the ring objects' pool (0x140-byte records)
    let take = t.img.rel32_target(f + 0x249);
    let tw = |t: &Twin, o: u32| t.img.word(take + o);
    let recs = SCRATCH + 0x1_0000;
    t.set_u32(tw(&t, 0x16), recs);
    // the deal: three kinds of two each
    let deal = t.img.rel32_target(f + 0x428);
    t.set_u32(t.img.word(deal + 0x17), 3);
    for i in 0..3 {
        t.set_u32(t.img.word(deal + 0x46) + 0x1c * i, 2);
    }
    // the emitters' aim: its task pool
    let es = t.img.rel32_target(f + 0x3ed);
    let aim = t.img.rel32_target(es + 0xc1);
    let atake = t.img.rel32_target(aim + 0x1be);
    // the selection and the characters
    let sel = SCRATCH + 0x600;
    t.set_u32(w(&t, 0x4c3), sel);
    t.write(sel, &[3, 5, 7, 1, 2, 4]);
    for i in 0..8 {
        t.set_u32(w(&t, 0x4ce) + 4 * i, 0x00c0_0000 + i);
    }
    let load = t.at(0x1210);
    let (w225, r227) = (SCRATCH + 0x120, SCRATCH + 0x124);
    rec(&mut t, 225, 1, move |m, _| m.r32(w225));
    rec(&mut t, 226, 2, |_, a| 0x0226_0000 ^ a[1]);
    rec(&mut t, 227, 1, move |m, _| m.r32(r227));
    // the start-up itself (4 cases), then through the load step: waiting,
    // asked and not loaded, loaded (the start-up runs)
    for k in 0..11u32 {
        let (count, f_val) = [
            (18u32, 0.5f32),
            (9, 4.0),
            (16, -4.0),
            (14, 1.25),
            (15, 6.0),
            (17, -6.0),
            (16, 7.5),
            (16, 9.0),
        ][(k % 8) as usize];
        // the globals and records it writes, junk first
        for op in [
            0x29u32, 0x33, 0x3d, 0x4b, 0x59, 0xab, 0xb1, 0x619, 0x61e, 0x623, 0x62f, 0x639, 0x643,
            0x64d, 0x657, 0x661, 0x66b, 0x675, 0x67f, 0x689, 0x6b3, 0x6b9, 0x6c3, 0x6cd, 0x6d7,
            0x6e0,
        ] {
            let at = w(&t, op);
            t.set_u32(at, 0x5a5a_0000 ^ op ^ k);
        }
        let (ring_e, smoke_e) = (w(&t, 0x244), w(&t, 0x44b));
        floats(&mut t, ring_e - 0x10, 0x288 * 14 / 4, k);
        floats(&mut t, smoke_e - 0x250, 0x288 * 4 / 4, k + 1);
        setf(&mut t, facing, f_val);
        t.set_u32(tw(&t, 0x09), count);
        t.set_u32(tw(&t, 0x01), 0);
        for i in 0..count {
            let r = recs + 0x140 * i;
            t.write(r, &[0x2a; 0x140]);
            t.set_u32(r + 0x130, 0);
            t.set_u16(r + 0x124, 0x7fff);
        }
        // the targets: the count, their dealt order lives in the frame
        t.set_u32(w(&t, 0x440), [3u32, 0, 2, 1, 3, 2, 1, 2][(k % 8) as usize]);
        t.set_u32(w(&t, 0x4bc), 0x0abc_0000 + k);
        t.set_u32(w(&t, 0x612), k & 1);
        t.set_u32(w(&t, 0x6a8), (k >> 1) & 1);
        // the aim's task take (the same ring as the start-up's own takes
        // in this DLL, or another)
        let (ac, ah) = (t.img.word(atake + 0x04), t.img.word(atake + 0x17));
        magic_r11::ring(&mut t, ac, ah, 40, SCRATCH + 0xe000);
        // the task ring (shared by the two takes and the reset's take)
        let (tc, th) = (w(&t, 0x6e7), w(&t, 0x6fb));
        if (tc, th) != (ac, ah) {
            magic_r11::ring(&mut t, tc, th, 8, SCRATCH + 0x9000);
        }
        let (first, last) = (w(&t, 0x75d), w(&t, 0x770));
        let old = SCRATCH + 0x9400;
        t.write(old, &[0x41; 0x20]);
        t.set_u32(first, if k % 2 == 0 { old } else { 0 });
        t.set_u32(last, if k % 2 == 0 { old } else { 0 });
        ownb::dirty_stack(&mut t, 0x1340 + k, &[]);
        if k < 8 {
            t.check_entry(f, Call::default(), &[0x0dd0_0000 + k], Eax::Checked);
        } else {
            t.set_u32(w225, u32::from(k == 8));
            t.set_u32(r227, u32::from(k == 10) * 2);
            t.set_u32(t.img.word(load + 0x18), u32::from(k == 9));
            t.check_entry(load, Call::default(), &[0x0dd0_0000 + k], Eax::Checked);
        }
    }
}

/// `magic_0146` 0x10006d60 (`r36::summon_task_0146`, an entry): the start
/// (the object list empty or not), the step with the model's flag off and
/// on (a sound by slot 388, and the eighth time slots 213/239), the end
/// (the node unlinked), an unknown state.
#[test]
fn summon_task_0146() {
    let Some(mut t) = twin(146) else { return };
    let f = t.at(0x6d60);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    all_stubs(&mut t);
    magic_r11::object_slots(&mut t);
    let model = SCRATCH + 0x7000;
    let flag = SCRATCH + 0x130;
    // the model (its flag word set first), for every id
    rec(&mut t, 941, 1, move |m, _| {
        let v = m.r32(flag);
        m.w32(model + 0x3e, v);
        model
    });
    rec(&mut t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, |m, _| {
        let k = m.r32(SCRATCH + 0x2_7600);
        m.w32(SCRATCH + 0x2_7600, k + 1);
        f64::from(k % 5) * 0.25 + 0.125
    });
    rec(&mut t, 324, 0, |m, _| {
        let s = m
            .r32(SCRATCH + 0x2_7604)
            .wrapping_mul(1_103_515_245)
            .wrapping_add(12345);
        m.w32(SCRATCH + 0x2_7604, s);
        s >> 8
    });
    rec(&mut t, 685, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v ^ 0x10);
        }
        0x685
    });
    // the particle node's globals: its part pointer, the part start's,
    // the owner record
    let pn = t.img.rel32_target(f + 0x22c);
    let ps = t.img.rel32_target(pn + 0x88);
    let (pg, sg) = (t.img.word(pn + 0x45), t.img.word(ps + 0x04));
    t.set_u32(pg, SCRATCH + 0x7f0);
    t.set_u32(sg, SCRATCH + 0x7f8);
    let owner = SCRATCH + 0x3800;
    t.write(owner, &[0x31; 0x80]);
    t.set_u32(w(&t, 0x1ec), owner);
    let o = SCRATCH + 0x1_0000;
    let task = SCRATCH + 0x1000;
    // (state, model flag, sound count)
    let cases: [(u32, u32, u32); 7] = [
        (0, 0, 0),
        (0, 0, 0),
        (1, 0, 0),
        (1, 0x2000, 3),
        (1, 0x2fff, 7),
        (0x2710, 0, 0),
        (5, 0, 0),
    ];
    for (k, (state, fl, sounds)) in (0u32..).zip(cases) {
        floats(&mut t, o, 0x1c60 / 4, k);
        t.set_u32(o + 0x18, state);
        t.set_u32(o + 0x2c, 0x10 + k);
        t.set_u32(o + 0x1c40, 0x0bad_0000 + (k & 1));
        floats(&mut t, model, 0x80, k + 3);
        // the sound bytes and words the step reads: junk
        for op in [0x243u32, 0x25a, 0x268, 0x2a8] {
            let at = w(&t, op);
            t.set_u32(at, 0x5a5a_5a03 ^ (op << 8) ^ k);
        }
        t.set_u32(flag, fl);
        t.set_u32(task + 4, o);
        // the task's node in a ring of three
        let node = |i: u32| SCRATCH + 0x1100 + 0x20 * i;
        for i in 0..3 {
            t.write(node(i), &[0x5a; 0x20]);
            t.set_u32(node(i) + 4, node((i + 2) % 3));
            t.set_u32(node(i) + 8, node((i + 1) % 3));
        }
        t.set_u32(task, node(0));
        t.set_u32(w(&t, 0x231), sounds);
        // the parents' child lists: empty, or holding one child (k odd)
        for (j, op) in (0u32..).zip([0x97u32, 0x13f, 0x1b9, 0x205, 0x3c6]) {
            let p = w(&t, op);
            let kid = SCRATCH + 0x1400 + 0x40 * j;
            t.write(kid, &[0x66; 0x40]);
            let last = if k % 2 == 1 { kid } else { p };
            t.set_u32(p + 0xc, last);
            t.set_u32(p + 0x10, last);
            t.set_u16(p + 0x18, (k % 2) as u16);
        }
        // the object list: empty (its last is the list) or not
        let list = w(&t, 0x324);
        let other = SCRATCH + 0x1200;
        t.write(other, &[0x44; 0x20]);
        t.set_u32(w(&t, 0x31c), if k == 1 { other } else { list });
        t.set_u32(w(&t, 0x314), if k == 1 { other } else { list });
        ownb::dirty_stack(&mut t, 0x6d60 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// An empty child list on the node `p` (its first and last child itself,
/// no children).
fn empty_kids(t: &mut Twin, p: u32) {
    t.set_u32(p + 0xc, p);
    t.set_u32(p + 0x10, p);
    t.set_u16(p + 0x18, 0);
}

/// `magic_0146` 0x100071c0 (`r36::scene_task_0146`, an entry): every state
/// of the byte table (and the ones past it), each waiting and moving on:
/// the scene flag set or not, ids held or not, the kinds' sizes, the
/// object countdown, the end.
#[test]
fn scene_task_0146() {
    let Some(mut t) = twin(146) else { return };
    let f = t.at(0x71c0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    all_stubs(&mut t);
    magic_r11::object_slots(&mut t);
    let pick = SCRATCH + 0x280;
    let ids = SCRATCH + 0x200;
    t.write(ids, &[4, 0x81, 7, 0xff]);
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 0x5a5a_5a5a);
        pick
    });
    rec(&mut t, 368, 2, move |m, a| {
        m.w32(a[1], 2);
        ids
    });
    t.stub_st0(333, |_, _| 0.625);
    // the unit block's float, the colour node's four integers
    t.stub_st0(790, |_, _| 0.375);
    rec(&mut t, 558, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 + 0x11 * i);
        }
        0x558
    });
    let scene = SCRATCH + 0x134;
    rec(&mut t, 520, 0, move |m, _| m.r32(scene));
    let kind = SCRATCH + 0x138;
    rec(&mut t, 390, 1, move |m, _| m.r32(kind));
    let party = SCRATCH + 0x240;
    t.write(party, &[0x11, 0x22, 0x33]);
    let n_party = SCRATCH + 0x13c;
    rec(&mut t, 288, 2, move |m, a| {
        let n = m.r32(n_party);
        m.w32(a[1], n);
        party
    });
    rec(&mut t, 324, 0, |m, _| {
        let s = m
            .r32(SCRATCH + 0x2_7604)
            .wrapping_mul(1_103_515_245)
            .wrapping_add(12345);
        m.w32(SCRATCH + 0x2_7604, s);
        s >> 8
    });
    // the DLL's own records the party's ids are checked against (count, ids)
    let (rn, rec0) = (w(&t, 0x25e), w(&t, 0x26c));
    // the particle nodes' globals
    for call in [0x350u32, 0x53b, 0x692] {
        let pnw = t.img.rel32_target(f + call);
        let pn = t.img.rel32_target(pnw + 0x35);
        let ps = t.img.rel32_target(pn + 0x88);
        let (pg, sg) = (t.img.word(pn + 0x45), t.img.word(ps + 0x04));
        t.set_u32(pg, SCRATCH + 0x7f0);
        t.set_u32(sg, SCRATCH + 0x7f8);
        let owner = SCRATCH + 0x3800;
        t.write(owner, &[0x31; 0x80]);
        t.set_u32(t.img.word(pnw + 0x0d), owner);
    }
    // the push task's list head
    let pt = t.img.rel32_target(f + 0x3bb);
    let head = SCRATCH + 0x3900;
    let o = SCRATCH + 0x1_0000;
    let task = SCRATCH + 0x1000;
    let lock = SCRATCH + 0x7e0;
    t.set_u32(w(&t, 0x552), lock);
    t.set_u32(w(&t, 0x799), lock);
    // (state, count, countdown, objects, ids, flag, scene, kind)
    let cases: [(u32, u32, u32, u32, u16, u32, u32, u32); 24] = [
        (0, 0, 0, 0, 0, 0, 0x0161_000f, 0),
        (0, 3, 0, 0, 0, 1, 0x0160_000f, 0),
        (0xa, 0, 0, 0, 5, 0, 0, 0),
        (0x14, 7, 0, 0, 0, 1, 0, 0),
        (0x15, 14, 0, 0, 0, 0, 0, 0),
        (0x15, 15, 0, 0, 0, 0, 0, 0),
        (0x1e, 0, 0, 4, 0, 0, 0, 0),
        (0x1f, 0, 2, 0, 0, 0, 0, 0),
        (0x1f, 0, 0, 3, 0, 0, 0, 0),
        (0x1f, 0, 0, 7, 0, 1, 0, 0),
        (0x29, 29, 0, 0, 0, 0, 0, 0),
        (0x29, 30, 0, 0, 0, 0, 0, 0),
        (0x2a, 4, 0, 0, 0, 0, 0, 0),
        (0x2a, 5, 0, 0, 0, 0, 0, 0x1088),
        (0x2a, 6, 0, 0, 0, 0, 0, 0x108c),
        (0x2a, 7, 0, 0, 0, 1, 0, 0x108a),
        (0x2b, 10, 0, 0, 0, 0, 0x0161_000f, 0),
        (0x2b, 50, 0, 0, 0, 0, 0x0161_0000, 0),
        (0x33, 24, 0, 0, 2, 0, 0, 0),
        (0x33, 25, 0, 0, 2, 0, 0, 0),
        (0x33, 25, 0, 0, 0, 0, 0, 0),
        (0x34, 15, 0, 0, 0, 1, 0, 0),
        (0x2710, 0, 0, 0, 0, 0, 0, 0),
        (0x40, 0, 0, 0, 0, 1, 0, 0),
    ];
    for (k, (state, count, cd, objs, nid, flag, sc, kd)) in (0u32..).zip(cases) {
        floats(&mut t, o, 0x200 / 4, k);
        // the bytes and words it reads or writes: junk first
        for op in [0x219u32, 0x3ee, 0x4dc, 0x5eb, 0x699, 0x6f0] {
            let at = w(&t, op);
            t.set_u32(at, 0x5a5a_5a02 ^ (op << 8) ^ k);
        }
        t.set_u32(o + 0x18, state);
        t.set_u32(o + 0x2c, count);
        t.set_u32(o + 0x28, cd);
        t.set_u32(o + 0x30, objs);
        t.set_u16(o + 0x74, nid);
        t.set_u32(o + 0x78, flag);
        t.set_u32(scene, sc);
        t.set_u32(kind, kd);
        t.set_u32(n_party, [0u32, 2, 3][(k % 3) as usize]);
        // the DLL's records: hold 0x22 in the odd cases
        t.set_u32(rn, 2);
        t.set_u32(rec0, 0x44);
        t.set_u32(rec0 + 0x98, if k % 2 == 1 { 0x22 } else { 0x45 });
        t.set_u32(task + 4, o);
        let node = |i: u32| SCRATCH + 0x1100 + 0x20 * i;
        for i in 0..3 {
            t.write(node(i), &[0x5a; 0x20]);
            t.set_u32(node(i) + 4, node((i + 2) % 3));
            t.set_u32(node(i) + 8, node((i + 1) % 3));
        }
        t.set_u32(task, node(0));
        t.write(head, &[0x48; 0x20]);
        t.set_u32(t.img.word(pt + 0x17), head);
        // the parents: empty child lists
        for op in [0x62u32, 0x87, 0xf3, 0x2d9, 0x5da, 0x659] {
            let p = w(&t, op);
            empty_kids(&mut t, p);
        }
        for call in [0x350u32, 0x53b, 0x692] {
            let pnw = t.img.rel32_target(f + call);
            let p = t.img.word(pnw + 0x26);
            empty_kids(&mut t, p);
        }
        for call in [0xcbu32, 0x6ea] {
            let ena = t.img.rel32_target(f + call);
            let p = t.img.word(ena + 0x18);
            empty_kids(&mut t, p);
        }
        t.set_u32(lock, 0x1c0c_0000 + k);
        ownb::dirty_stack(&mut t, 0x71c0 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0155` 0x1000bf70 and `magic_0164` 0x1000bcf0
/// (`r36::start_up_0155`, `r36::start_up_0164`; `magic_r27.rs`'s
/// `start_up_0609` set-up): no targets and two; the statics' root with and
/// without a child; the rings' words junked first.
#[test]
fn start_up_0155_0164() {
    for (n, at, junked) in [
        (
            155u16,
            0xbf70u32,
            [
                0x188u32, 0x20d, 0x247, 0x24d, 0x25d, 0x267, 0x2a5, 0x2b6, 0x2c0,
            ],
        ),
        (
            164,
            0xbcf0,
            [
                0x188, 0x20d, 0x247, 0x250, 0x260, 0x26a, 0x2a3, 0x2b4, 0x2be,
            ],
        ),
    ] {
        let Some(mut t) = twin(n) else { return };
        all_stubs(&mut t);
        let f = t.at(at);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let ti = t.img.rel32_target(f + 0x19);
        let statics = t.img.rel32_target(ti + 0xe0);
        let root = t.img.word(statics + 0x10d);
        rec(&mut t, 168, 1, |_, a| 0x0168_0000 + a[0]);
        let list = SCRATCH + 0x6000;
        rec(&mut t, 218, 2, move |m, a| {
            m.w32(a[1], m.r32(list + 0x7c));
            list
        });
        rec(&mut t, 531, 2, |m, a| {
            for i in 0..4 {
                m.w32(a[1] + 4 * i, a[0].wrapping_mul(0x0101_0101) ^ i);
            }
            0x531
        });
        rec(&mut t, 289, 1, |_, _| 0x289);
        t.stub_st0(289, |_, a| f64::from(a[0] & 0xff) * -0.5);
        let lock = SCRATCH + 0x7f0;
        t.set_u32(w(&t, 0x17c), lock);
        rec(&mut t, 711, 3, move |m, a| {
            let l = m.r32(lock);
            m.w32(SCRATCH + 0x7e0, l);
            0x0711_0000 ^ a[0]
        });
        rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
        rec(&mut t, 710, 4, |_, a| a[2] ^ 0x710);
        for (s, k) in [(718u32, 1usize), (222, 4), (221, 4), (560, 1), (673, 0)] {
            rec(&mut t, s, k, move |_, a| {
                (s << 16) ^ a.first().copied().unwrap_or(0)
            });
        }
        let r557 = SCRATCH + 0x7f4;
        rec(&mut t, 557, 0, move |m, _| m.r32(r557));
        let child = SCRATCH + 0x6800;
        for (k, count) in [0u32, 2, 1].into_iter().enumerate() {
            let k = k as u32;
            t.write(list, &[0x80, 0x81, 0x83, 0x84]);
            t.set_u32(list + 0x7c, count);
            t.write(root, &[0x5a; 0x150]);
            t.set_u32(root + 4, if k.is_multiple_of(2) { 0 } else { child });
            t.write(child, &[0x33; 0x40]);
            t.set_u32(child + 0x10, child);
            t.set_u32(lock, 0x40 + k);
            t.set_u32(r557, 0x0557_00ff ^ (k << 2));
            for o in junked {
                let at = w(&t, o);
                t.set_u32(at, 0x5a5a_5a5a ^ o ^ k);
            }
            ownb::dirty_stack(&mut t, at + k, &[]);
            t.check_entry(f, Call::default(), &[0x00ac_0000 + k], Eax::Checked);
        }
    }
}

/// `magic_0353` 0x10001900 (`r36::script_task_0353`, an entry) with its
/// close (0x10001a70, `r36::close_0353`): the script ended (the close and
/// the give-back) or waiting.
#[test]
fn script_task_0353() {
    let Some(mut t) = twin(353) else { return };
    let f = t.at(0x1900);
    all_stubs(&mut t);
    let run = t.img.rel32_target(f + 0xc);
    let mode = t.img.word(run + 0x01);
    let wait = t.img.word(run + 0x59);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0x7), s);
    let close = t.img.rel32_target(f + 0x23);
    t.set_u32(t.img.word(close + 0x2), s);
    let ms = t.img.rel32_target(f + 0x13);
    let (done, mwait) = (t.img.word(ms + 0x02), t.img.word(ms + 0x11));
    for (k, m) in [(0u32, 0u8), (1, 2), (2, 0)] {
        // the motion: done, or waiting
        t.set_u32(done, u32::from(k != 1));
        t.set_u16(mwait, 3);
        floats(&mut t, s, 0x1500 / 4, k);
        t.set_u8(mode, m);
        t.set_u16(wait, 0xffff);
        // the effect's task node (none in case 2)
        t.set_u32(
            s + 0x1180 + 0x228,
            if k == 2 { 0 } else { SCRATCH + 0x1800 },
        );
        t.write(SCRATCH + 0x1800, &[0; 0x20]);
        t.set_u32(SCRATCH + 0x1808, SCRATCH + 0x1800);
        let task = SCRATCH + 0x1000;
        t.write(task, &[0; 0x20]);
        t.set_u32(task + 8, task);
        for op in [0x19u32, 0x2e, 0x41] {
            let at = t.img.word(close + op);
            t.set_u32(at, 0x0e0e_0000 + op + k);
        }
        ownb::dirty_stack(&mut t, 0x1900 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0353` 0x10001b70 (`r36::fade_task_0353`, an entry): fading in,
/// fading out, the fade over (the give-back); the angle wrapped or not.
#[test]
fn fade_task_0353() {
    let Some(mut t) = twin(353) else { return };
    let f = t.at(0x1b70);
    all_stubs(&mut t);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0x19), s);
    let desc = SCRATCH + 0x2000;
    rec(&mut t, 719, 3, move |_, _| desc);
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    rec(&mut t, 329, 3, |m, a| {
        for i in 0..16 {
            let v = rf(m, a[1] + 4 * i) * 0.5 + (a[2] & 0xff) as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x329
    });
    rec(&mut t, 210, 2, |m, a| {
        for i in 0..16 {
            wf(m, a[1] + 4 * i, (a[0] & 0xf) as f32 + i as f32 * 0.25);
        }
        0x210
    });
    let answer = SCRATCH + 0x718;
    rec(&mut t, 885, 7, move |m, a| {
        for i in 0..8 {
            m.w32(a[6] + 4 * i, 0x0885_0000 + i);
        }
        m.r32(answer)
    });
    logp(&mut t, 905, 9, Some((7, 4)), 0x0905_0000);
    let lock = SCRATCH + 0x7f0;
    // (fade-in count, its total, fade-out count, its total, angle)
    let cases: [(i32, i32, i32, i32, f32); 4] = [
        (3, 8, 5, 10, 0.5),
        (0, 8, 5, 10, 0.01),
        (0, 8, 1, 10, -0.5),
        (1, 4, 0, 6, 2.0),
    ];
    for (k, (c_in, n_in, c_out, n_out, a)) in (0u32..).zip(cases) {
        floats(&mut t, s + 0x1000, 0x400 / 4, k);
        t.set_u32(s + 0x13ec, c_in.cast_unsigned());
        t.set_u32(s + 0x13f0, n_in.cast_unsigned());
        t.set_u32(s + 0x13f4, c_out.cast_unsigned());
        t.set_u32(s + 0x13f8, n_out.cast_unsigned());
        setf(&mut t, s + 0x13e8, a);
        t.set_u32(desc, 0x1234_5678 ^ k);
        t.set_u32(desc + 4, 0x0000_0003 + k);
        t.set_u32(answer, u32::from(k != 2));
        t.set_u32(lock, 0x40 + k);
        let v = SCRATCH + 0x3000;
        t.write(v, &[0x44; 0x900]);
        t.set_u32(t.img.word(f + 0xa7), v);
        let task = SCRATCH + 0x1000;
        t.write(task, &[0; 0x20]);
        t.set_u32(task + 8, task);
        ownb::dirty_stack(&mut t, 0x1b70 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0353` 0x10001ce0 (`r36::effect_setup_0353`, an entry): the
/// busy list empty and not, the free ring with many nodes and few (the
/// count's last-node branch); the never-written frame words 0.
#[test]
fn effect_setup_0353() {
    let Some(mut t) = twin(353) else { return };
    let f = t.at(0x1ce0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    all_stubs(&mut t);
    let s = SCRATCH + 0x1_0000;
    for k in 0..3u32 {
        floats(&mut t, s + 0xe00, 0x600 / 4, k);
        t.set_u32(w(&t, 0x20), s);
        t.set_u32(w(&t, 0x141), s);
        t.set_u32(w(&t, 0x1ed), s);
        // the free ring (5 nodes, or exactly 5: the last take finds 1)
        let (count, head) = (w(&t, 0x19), w(&t, 0x2f));
        magic_r11::ring(
            &mut t,
            count,
            head,
            [8u16, 5, 6][k as usize],
            SCRATCH + 0x9000,
        );
        let (first, last) = (w(&t, 0xfd), w(&t, 0x11));
        let old = SCRATCH + 0x9400;
        t.write(old, &[0x41; 0x20]);
        t.set_u32(first, if k == 1 { 0 } else { old });
        t.set_u32(last, if k == 1 { 0 } else { old });
        ownb::dirty_stack(&mut t, 0x1ce0 + k, &[0xc, 0x10]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0353` 0x10002200 (`r36::spark_task_0353`, an entry) and
/// 0x10002460 (`r36::summon_start_0353`): the spark waiting, taken and
/// the pool empty; the summon start with the busy list empty and not.
#[test]
fn sparks_and_start_0353() {
    let Some(mut t) = twin(353) else { return };
    all_stubs(&mut t);
    magic_r11::object_slots(&mut t);
    rec(&mut t, 314, 1, |_, a| 0x0314_0000 ^ a[0]);
    t.stub_st0(314, |m, a| {
        let k = m.r32(SCRATCH + 0x2_7600);
        m.w32(SCRATCH + 0x2_7600, k + 1);
        f64::from(f32::from_bits(a[0])) * 0.375 + f64::from(k % 5) * 0.125
    });
    let s = SCRATCH + 0x1_0000;
    // the spark task
    let f = t.at(0x2200);
    let start = t.img.rel32_target(f + 0x50);
    magic_r11::object_start_auto(&mut t, start);
    let take = t.img.rel32_target(f + 0x30);
    let tw = |t: &Twin, o: u32| t.img.word(take + o);
    let recs = SCRATCH + 0x2_0000;
    t.set_u32(tw(&t, 0x16), recs);
    t.set_u32(t.img.word(f + 0x11), s);
    let task = SCRATCH + 0x1000;
    for (k, (wait, n)) in [(5i32, 4u32), (1, 4), (0, 0), (1, 3)]
        .into_iter()
        .enumerate()
    {
        let k = k as u32;
        floats(&mut t, s + 0xe00, 0x80 / 4, k);
        t.write(task, &[0x33; 0x40]);
        t.set_u32(task + 0x2c, wait.cast_unsigned());
        t.set_u32(tw(&t, 0x09), n);
        t.set_u32(tw(&t, 0x01), 0);
        for i in 0..n {
            let r = recs + 0x140 * i;
            t.write(r, &[0x2a; 0x140]);
            t.set_u32(r + 0x130, 0);
            t.set_u16(r + 0x124, 0x7fff);
        }
        ownb::dirty_stack(&mut t, 0x2200 + k, &[]);
        t.check_entry(
            f,
            Call {
                eax: 0x0eaa_0000 + k,
                ..Call::default()
            },
            &[task],
            Eax::Checked,
        );
    }
    // the summon start
    let f = t.at(0x2460);
    let start = t.img.rel32_target(f + 0x1f);
    magic_r11::object_start_auto(&mut t, start);
    magic_r11::pools(&mut t, f, 277);
    t.set_u32(t.img.word(f + 0xb), s);
    for k in 0..2u32 {
        floats(&mut t, s, 0x1000 / 4, k + 7);
        let (first, last) = (t.img.word(f + 0xdb), t.img.word(f + 0xef));
        let old = SCRATCH + 0x9400;
        t.write(old, &[0x41; 0x20]);
        t.set_u32(first, if k == 1 { 0 } else { old });
        t.set_u32(last, if k == 1 { 0 } else { old });
        ownb::dirty_stack(&mut t, 0x2460 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0353` 0x10002840 (`r36::lit_task_0353`, an entry) with its
/// flicker (0x100025d0), lit draw (0x10001640) and falloff (0x10001520):
/// the flicker counting down, starting anew, rising and held; parts with
/// points inside, at and outside the light's radius; no parts.
#[test]
fn lit_task_0353() {
    let Some(mut t) = twin(353) else { return };
    let f = t.at(0x2840);
    all_stubs(&mut t);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0x19), s);
    let fl = t.img.rel32_target(f + 0x26);
    // the flicker's and the task's passed-on globals, nonzero
    t.set_u32(t.img.word(fl + 0x13), 0x1357_9bdf);
    t.set_u32(t.img.word(f + 0x41), 0x2468_ace0);
    t.set_u32(t.img.word(fl + 0x1a), s);
    let ld = t.img.rel32_target(f + 0xad);
    let m = SCRATCH + 0x4000;
    t.set_u32(t.img.word(ld + 0x23), m);
    rec(&mut t, 404, 1, |_, a| 0x0404_0000 ^ a[0]);
    t.stub_st0(404, |_, a| f64::from(f32::from_bits(a[0])) * 0.5 + 0.25);
    // the light's colour words (four pointers)
    logp(&mut t, 857, 2, Some((2, 1)), 0x0857_0000);
    for (s9, n9) in [(559u32, 3usize), (558, 1)] {
        rec(&mut t, s9, n9, move |m, a| {
            m.w32(a[0], 0x0559_0000 ^ s9);
            0x559
        });
    }
    for (s9, n9) in [(723u32, 1usize), (724, 1), (230, 2)] {
        rec(&mut t, s9, n9, move |m, a| {
            for i in 0..16 {
                wf(m, a[0] + 4 * i, (i + s9 % 7) as f32 * 0.25);
            }
            s9
        });
    }
    rec(&mut t, 231, 3, |m, a| {
        let v: Vec<f32> = (0..16)
            .map(|i| rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i))
            .collect();
        for (i, x) in (0u32..).zip(v) {
            wf(m, a[0] + 4 * i, x);
        }
        0x231
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * (i + 12)) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    // the offset from the light
    rec(&mut t, 234, 3, |m, a| {
        for i in 0..3 {
            let v = rf(m, a[2] + 4 * i) - rf(m, a[1] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x234
    });
    rec(&mut t, 545, 0, |_, _| SCRATCH + 0x4800);
    let bufs = SCRATCH + 0x2_0000 - 0x6_0000;
    rec(&mut t, 562, 0, move |_, _| bufs);
    // the parts: records and matrices from slot 941
    let recs = SCRATCH + 0x5000;
    rec(&mut t, 941, 1, move |_, a| recs + 0x100 * (a[0] & 7));
    let pts = bufs.wrapping_add(0x6_0000);
    for k in 0..5u32 {
        floats(&mut t, s + 0x1300, 0x100 / 4, k);
        let (flag, count, level) = [
            (0u32, 3u32, 0.5f32),
            (0, 1, 0.5),
            (1, 0, 100.0),
            (1, 0, 200.0),
            (0, 2, 3.0),
        ][k as usize];
        t.set_u32(s + 0x13dc, flag);
        t.set_u32(s + 0x13d4, count);
        t.set_u32(s + 0x13d8, 4);
        setf(&mut t, s + 0x13cc, level);
        setf(&mut t, s + 0x13d0, 6.0);
        // the light at the origin, its radius the level
        for i in 0..4 {
            setf(&mut t, s + 0x13ac + 4 * i, 0.0);
            setf(&mut t, s + 0x13bc + 4 * i, 0.25 * i as f32);
        }
        // the model: k % 3 parts, each two ids
        t.write(m, &[0; 0x40]);
        t.set_u16(m + 4, (k % 3) as u16);
        for p in 0..2u32 {
            t.set_u32(m + 0x10 + 0x10 * p, 2 * p);
            t.set_u32(m + 0x14 + 0x10 * p, 2 * p + 1);
        }
        for r in 0..4u32 {
            let rr = recs + 0x100 * r;
            floats(&mut t, rr, 0x40, r + k);
            t.set_u32(rr + 4, 0x40);
            t.set_u16(rr + 0x10, 3);
            t.set_u16(rr + 0x12, 2 + r as u16);
            // groups: kind 0x10 with one extra, then plain ones
            t.set_u8(rr + 0x41, 0x10);
            t.set_u16(rr + 0x42, 1);
            t.set_u8(rr + 0x41 + 0x3c, 0x11);
            t.set_u16(rr + 0x42 + 0x3c, 2);
        }
        for i in 0..8u32 {
            let d = [0.5f32, 2.0, 0.1, 10.0][(i % 4) as usize];
            setf(&mut t, pts + 0x10 * i, d);
            setf(&mut t, pts + 0x10 * i + 4, 0.375);
            setf(&mut t, pts + 0x10 * i + 8, 0.25);
        }
        let task = SCRATCH + 0x1000;
        t.write(task, &[0; 0x20]);
        t.set_u32(task + 8, task);
        ownb::dirty_stack(&mut t, 0x2840 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0353` 0x10002950 (`r36::lit_start_0353`, an entry): the free
/// ring with several nodes and one, the busy list empty and not.
#[test]
fn lit_start_0353() {
    let Some(mut t) = twin(353) else { return };
    let f = t.at(0x2950);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    all_stubs(&mut t);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x13), s);
    t.set_u32(w(&t, 0x125), 0x0bad_f00d);
    for k in 0..3u32 {
        floats(&mut t, s + 0x1300, 0x100 / 4, k);
        let (count, head) = (w(&t, 0x36), w(&t, 0x7e));
        magic_r11::ring(
            &mut t,
            count,
            head,
            [4u16, 1, 2][k as usize],
            SCRATCH + 0x9000,
        );
        let (first, last) = (w(&t, 0xe5), w(&t, 0xfc));
        let old = SCRATCH + 0x9400;
        t.write(old, &[0x41; 0x20]);
        t.set_u32(first, if k == 1 { 0 } else { old });
        t.set_u32(last, if k == 1 { 0 } else { old });
        ownb::dirty_stack(&mut t, 0x2950 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0353` 0x10002aa0 (`r36::light_reset_0353`, an entry): the
/// records copied back, the scale task's pool full and not.
#[test]
fn light_reset_0353() {
    let Some(mut t) = twin(353) else { return };
    let f = t.at(0x2aa0);
    all_stubs(&mut t);
    magic_r11::pools(&mut t, f, 862);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0x3), s);
    for k in 0..2u32 {
        floats(&mut t, s + 0x1100, 0x300 / 4, k);
        ownb::dirty_stack(&mut t, 0x2aa0 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0353` 0x100012b0 (`r36::load_0353`, an entry) running the
/// start-up (0x10002e60, `r36::start_up_0353`): waiting, asked, loaded
/// (the start-up with the busy list empty and not).
#[test]
fn load_0353() {
    let Some(mut t) = twin(353) else { return };
    let f = t.at(0x12b0);
    all_stubs(&mut t);
    magic_r11::object_slots(&mut t);
    let su = t.img.rel32_target(f + 0x5b);
    magic_r11::pools(&mut t, su, 424);
    let (w225, r227) = (SCRATCH + 0x120, SCRATCH + 0x124);
    rec(&mut t, 225, 1, move |m, _| m.r32(w225));
    rec(&mut t, 226, 2, |_, a| 0x0226_0000 ^ a[1]);
    rec(&mut t, 227, 1, move |m, _| m.r32(r227));
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, |_, _| 0.625);
    rec(&mut t, 12, 1, |_, a| 0x12 ^ a[0]);
    t.stub_st0(12, |_, _| 1.75);
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], 0.5 * i as f32);
        }
        0x262
    });
    // the part set-up's lock
    let ps = t.img.rel32_target(su + 0x70);
    let lock = SCRATCH + 0x7e0;
    t.set_u32(t.img.word(ps + 0x04), lock);
    let lk = t.img.word(ps + 0x2d);
    if lk != t.img.word(ps + 0x04) {
        t.set_u32(lk, lock);
    }
    for k in 0..4u32 {
        t.set_u32(w225, u32::from(k == 0));
        t.set_u32(r227, u32::from(k >= 2) * 2);
        t.set_u32(t.img.word(f + 0x18), u32::from(k == 1));
        // the busy list: empty or one node
        let (first, last) = (t.img.word(su + 0x11b), t.img.word(su + 0x12f));
        let old = SCRATCH + 0x9400;
        t.write(old, &[0x41; 0x20]);
        t.set_u32(first, if k == 3 { 0 } else { old });
        t.set_u32(last, if k == 3 { 0 } else { old });
        t.set_u32(lock, 0x40 + k);
        // the words the start-up clears, dirty
        for i in 0..0x20u32 {
            t.set_u32(t.img.word(su + 0xb4) + 0x140 * i, 0x5a5a_5a5a);
        }
        t.set_u32(t.img.word(su + 0xe0), 0x5a5a_5a5a);
        t.set_u32(t.img.word(su + 0xe7), 0x5a5a_5a5a);
        ownb::dirty_stack(&mut t, 0x12b0 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// The slots of `magic_0351`'s particle callbacks: matrices, points,
/// the lock, slot 712's answer from `SCRATCH + 0x128`.
fn stubs_0351(t: &mut Twin) {
    all_stubs(t);
    for (s, n) in [
        (231u32, 3usize),
        (230, 2),
        (266, 3),
        (232, 2),
        (248, 3),
        (234, 3),
    ] {
        rec(t, s, n, move |m, a| {
            for i in 0..4 {
                let v = rf(m, a[1] + 4 * i) * 0.5 + if n == 3 { rf(m, a[2] + 4 * i) } else { 0.25 };
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    let r712 = SCRATCH + 0x128;
    rec(t, 712, 2, move |m, _| m.r32(r712));
    rec(t, 324, 0, |m, _| {
        let s = m
            .r32(SCRATCH + 0x2_7604)
            .wrapping_mul(1_103_515_245)
            .wrapping_add(12345);
        m.w32(SCRATCH + 0x2_7604, s);
        s >> 8
    });
    let table = SCRATCH + 0x3400;
    rec(t, 941, 1, move |_, _| table);
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
}

/// `magic_0351`'s particle callbacks: the spark step (0x10001690), the
/// glow start (0x10001820, with its glow points 0x10001310) and step
/// (0x100019c0), the scene start (0x10001e60): every early way out, the
/// lights below and at their limits, the glow counter's three ranges,
/// slot 712 answering 0 and not.
#[test]
fn particles_0351() {
    let Some(mut t) = twin(351) else { return };
    stubs_0351(&mut t);
    let r712 = SCRATCH + 0x128;
    let lock = SCRATCH + 0x7e0;
    // the spark step
    let f = t.at(0x1690);
    let w = |t: &Twin, f: u32, o: u32| t.img.word(f + o);
    let o = SCRATCH + 0x1000;
    t.set_u32(w(&t, f, 0x125), lock);
    t.set_u32(w(&t, f, 0x118), SCRATCH + 0x7f0);
    // (off, n, flags, lights, counter, glow, 712)
    let cases: [(u32, u32, [u32; 3], [f32; 3], i32, i32, u32); 6] = [
        (1, 3, [1, 1, 1], [1.0; 3], 0, 0, 0),
        (0, 3, [0, 1, 1], [0.0, 0.0, 1.0], 0, 0, 0),
        (0, 2, [1, 0, 1], [1.0, 0.0, 0.0], 0, 5, 1),
        (0, 3, [1, 1, 1], [1.0, 1.0, 1.0], 40, 20, 0),
        (0, 3, [0, 0, 0], [0.0; 3], 41, 200, 1),
        (0, 1, [1, 1, 1], [1.0, 0.0, 0.0], 45, 3, 0),
    ];
    for (k, (off, n, fl, li, c, glow, r)) in (0u32..).zip(cases) {
        t.set_u32(w(&t, f, 0x5), off);
        t.set_u32(w(&t, f, 0x3b), n);
        for i in 0..3u32 {
            t.set_u32(w(&t, f, 0x47) + 4 * i, fl[i as usize]);
            {
                let a = w(&t, f, 0x52);
                setf(&mut t, a + 4 * i, li[i as usize]);
            }
        }
        t.set_u32(w(&t, f, 0x5f), c.cast_unsigned());
        t.set_u32(w(&t, f, 0x95), glow.cast_unsigned());
        t.set_u32(r712, r);
        floats(&mut t, o, 0x20, k);
        ownb::dirty_stack(&mut t, 0x1690 + k, &[]);
        t.check_entry(f, Call::default(), &[o], Eax::Checked);
    }
    // the glow start and step
    for (rva, steps) in [(0x1820u32, false), (0x19c0, true)] {
        let f = t.at(rva);
        let base = w(&t, f, if steps { 0x11 } else { 0x14 });
        let sp = t.img.rel32_target(f + if steps { 0x1b7 } else { 0x8a });
        t.set_u32(t.img.word(sp + 0x04), lock);
        let pw = SCRATCH + 0x3800;
        t.write(pw, &[0x31; 0x40]);
        let ids = SCRATCH + 0x3900;
        t.write(ids, &[3, 5, 7, 9]);
        if steps {
            t.set_u32(w(&t, f, 0x194), pw);
            for op in [0x138u32, 0x170, 0x189] {
                t.set_u32(w(&t, f, op), SCRATCH + 0x7f0);
            }
            t.set_u32(w(&t, f, 0x145), lock);
        } else {
            t.set_u32(w(&t, f, 0x66), pw);
            t.set_u32(w(&t, f, 0x3f), ids);
            t.set_u32(w(&t, f, 0x58), SCRATCH + 0x7f0);
        }
        // (index, count, flag, level, dim, 712, the lights' levels)
        let cases: [(u32, u32, u32, f32, u32, u32, [f32; 3]); 6] = [
            (0, 3, 1, 0.25, 0, 0, [1.0, 1.0, 1.0]),
            (1, 3, 1, 0.99, 1, 1, [1.0, 0.5, 1.0]),
            (2, 3, 0, 0.5, 0, 0, [1.0; 3]),
            (3, 3, 1, 0.5, 0, 0, [1.0; 3]),
            (2, 3, 1, 2.0, 1, 1, [1.0, 1.0, f32::NAN]),
            (0, 1, 1, 0.5, 0, 1, [1.0, 0.0, 0.0]),
        ];
        for (k, (idx, n, fl, lv, dim, r, lights)) in (0u32..).zip(cases) {
            let o = base + 0x80 * idx;
            floats(&mut t, o, 0x20, k);
            t.set_u16(o + 0x76, 0x1234 + k as u16);
            let (n_op, fl_op) = if steps {
                (0x4u32, 0x24u32)
            } else {
                (0x7, 0x33)
            };
            t.set_u32(w(&t, f, n_op), n);
            for i in 0..4u32 {
                t.set_u32(w(&t, f, fl_op) + 4 * i, if i == idx { fl } else { 1 });
            }
            if steps {
                {
                    let a = w(&t, f, 0x34);
                    setf(&mut t, a + 4 * idx, 0.125);
                }
                {
                    let a = w(&t, f, 0x3d);
                    setf(&mut t, a + 4 * idx, lv);
                }
                t.set_u32(w(&t, f, 0xb3), dim);
                t.set_u32(w(&t, f, 0x1c0), n);
                for i in 0..3u32 {
                    t.set_u32(w(&t, f, 0x1d7) + 4 * i, 1);
                    {
                        let a = w(&t, f, 0x1e3);
                        setf(&mut t, a + 4 * i, lights[i as usize]);
                    }
                }
            }
            t.set_u32(r712, r);
            ownb::dirty_stack(&mut t, rva + k, &[]);
            t.check_entry(f, Call::default(), &[o], Eax::Checked);
        }
    }
    // the scene start
    let f = t.at(0x1e60);
    for k in 0..2u32 {
        t.set_u32(w(&t, f, 0x2), k * 0x0abc_0000);
        {
            let a = w(&t, f, 0x11f);
            floats(&mut t, a - 8, 8, k);
        }
        ownb::dirty_stack(&mut t, 0x1e60 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// The matrix slots of `magic_0351`'s draws: 235 the identity, 329 a
/// turn, 395 and 231 mixes, 325 a copy, 353 and 397 logged with the
/// words they read.
fn draw_stubs_0351(t: &mut Twin) {
    stubs_0351(t);
    rec(t, 235, 1, |m, a| {
        for i in 0..16u32 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    rec(t, 329, 3, |m, a| {
        let k = f32::from_bits(a[2]);
        for i in 0..16u32 {
            let v = rf(m, a[1] + 4 * i) * k + 0.125;
            wf(m, a[0] + 4 * i, v);
        }
        0x329
    });
    for s in [395u32, 231] {
        rec(t, s, 3, move |m, a| {
            let v: Vec<f32> = (0..16)
                .map(|i| rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * (i % 4)))
                .collect();
            for (i, x) in (0u32..).zip(v) {
                wf(m, a[0] + 4 * i, x);
            }
            s
        });
    }
    rec(t, 325, 2, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + 0.5;
            wf(m, a[0] + 4 * i, v);
        }
        0x325
    });
    logp(t, 353, 1, Some((0, 4)), 0x0353_0000);
    logp(t, 397, 1, Some((0, 4)), 0x0397_0000);
}

/// `magic_0351`'s camera draw (0x10001fc0) and glow draw (0x10002310):
/// the done word 0 and not, slot 712 answering 0 and not.
#[test]
fn draws_0351() {
    let Some(mut t) = twin(351) else { return };
    draw_stubs_0351(&mut t);
    let w = |t: &Twin, f: u32, o: u32| t.img.word(f + o);
    let f = t.at(0x1fc0);
    // the motion script: empty, then one motion playing
    let ms = t.img.rel32_target(f + 0x143);
    let tab = SCRATCH + 0x6000;
    t.set_u32(t.img.word(ms + 0xb), tab);
    t.set_u32(t.img.word(ms + 0x2), 0);
    t.set_u32(t.img.word(ms + 0x1e), 1);
    for k in 0..2u32 {
        t.write(tab, &[0; 0x20]);
        t.set_u32(tab, k * 5);
        let c = w(&t, f, 0x47);
        floats(&mut t, c, 16, k);
        let a = w(&t, f, 0xdd);
        floats(&mut t, a, 4, k + 3);
        t.set_u32(w(&t, f, 0x14e), k * 7);
        ownb::dirty_stack(&mut t, 0x1fc0 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    let f = t.at(0x2310);
    let lock = SCRATCH + 0x7e0;
    t.set_u32(w(&t, f, 0xfa), lock);
    if w(&t, f, 0x116) != w(&t, f, 0xfa) {
        t.set_u32(w(&t, f, 0x116), lock);
    }
    let state = SCRATCH + 0x7f0;
    t.set_u32(w(&t, f, 0xed), state);
    for (i, op) in [0xa8u32, 0xc1, 0xcd, 0xe0].into_iter().enumerate() {
        let m = SCRATCH + 0x4000 + 0x100 * i as u32;
        floats(&mut t, m, 16, i as u32);
        t.set_u32(w(&t, f, op), m);
    }
    for k in 0..2u32 {
        let p = w(&t, f, 0x1c);
        floats(&mut t, p, 4, k);
        t.set_u32(lock, 0x40 + k);
        t.set_u32(SCRATCH + 0x128, k);
        let o = SCRATCH + 0x1000;
        ownb::dirty_stack(&mut t, 0x2310 + k, &[]);
        t.check_entry(f, Call::default(), &[o], Eax::Checked);
    }
}

/// `magic_0351`'s fade (0x100024b0): rising (by the count, by the flag
/// and step) and falling; below the floor, above the top (rising: the
/// early 0), at the top exactly, between; the level above 0 and not.
#[test]
fn fade_0351() {
    let Some(mut t) = twin(351) else { return };
    draw_stubs_0351(&mut t);
    let f = t.at(0x24b0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let top = f32::from_bits(t.img.word(w(&t, 0x1b)));
    let up = f32::from_bits(t.img.word(w(&t, 0x71)));
    let down = f32::from_bits(t.img.word(w(&t, 0x42)));
    let floor = f32::from_bits(t.img.word(w(&t, 0x4a)));
    let mid = f32::midpoint(top, floor);
    // (count, flag, step, level)
    let cases: [(i32, u32, i32, f32); 10] = [
        (1, 0, 0, mid),
        (0, 1, 0xdd, mid),
        (0, 1, 0xdc, mid),
        (0, 0, 0x200, mid),
        (-1, 0, 0, floor - 1.0),
        (0, 0, 0, top + down + 1.0),
        (1, 0, 0, top),
        (1, 0, 0, top - up),
        (1, 0, 0, floor - up - 1.0),
        (0, 0, 0, f32::NAN),
    ];
    for (k, (count, flag, step, level)) in (0u32..).zip(cases) {
        t.set_u32(w(&t, 0x12), count.cast_unsigned());
        t.set_u32(w(&t, 0x23), flag);
        t.set_u32(w(&t, 0x2c), step.cast_unsigned());
        let a = w(&t, 0x3a);
        setf(&mut t, a, level);
        let a = w(&t, 0x14e);
        setf(&mut t, a, 0.5 + k as f32);
        let a = w(&t, 0x18e);
        setf(&mut t, a, 1.5);
        ownb::dirty_stack(&mut t, 0x24b0 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0351`'s start-up (0x100027d0): no ids, ids with their flags
/// all clear and one set.
#[test]
fn start_up_0351() {
    let Some(mut t) = twin(351) else { return };
    draw_stubs_0351(&mut t);
    let f = t.at(0x27d0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 803, 2, |_, _| SCRATCH + 0x2_0000);
    let parts = SCRATCH + 0x5000;
    rec(&mut t, 719, 3, move |_, a| parts + 0x40 * (a[2] + a[1]));
    for i in 0..16u32 {
        t.set_u32(parts + 0x40 * i + 0x14, 3 * i);
        t.set_u32(parts + 0x40 * i + 0x18, 3 * i + 1);
        t.set_u32(parts + 0x40 * i + 0x1c, 3 * i + 2);
    }
    let ids = SCRATCH + 0x3900;
    t.write(ids, &[3, 5, 7, 9]);
    let n_at = SCRATCH + 0x12c;
    rec(&mut t, 407, 3, move |m, a| {
        let n = m.r32(n_at);
        m.w32(a[1], n);
        ids
    });
    let flags = SCRATCH + 0x130;
    rec(&mut t, 367, 8, move |m, a| {
        let v = m.r32(flags) >> (a[1] & 7);
        m.w32(a[2], v ^ 0x10);
        0x367
    });
    let lock = SCRATCH + 0x7e0;
    t.set_u32(w(&t, 0x121), lock);
    if w(&t, 0x148) != w(&t, 0x121) {
        t.set_u32(w(&t, 0x148), lock);
    }
    for (k, (n, fl)) in (0u32..).zip([(0u32, 0u32), (3, 0), (4, 1 << 5), (2, 0xff)]) {
        t.set_u32(n_at, n);
        t.set_u32(flags, fl);
        t.set_u32(lock, 0x40 + k);
        t.write(w(&t, 0x2ae), &[0x5a; 0x110]);
        for i in 0..4u32 {
            t.set_u32(w(&t, 0x295) + 4 * i, 0x77);
        }
        ownb::dirty_stack(&mut t, 0x27d0 + k, &[]);
        t.check_entry(f, Call::default(), &[0x0abc_0000 + k], Eax::Checked);
    }
}
