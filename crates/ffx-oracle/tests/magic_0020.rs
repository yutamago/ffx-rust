//! `magic_0020.dll` against its Rust port (`ffx_magic::m0020`): `DllMain`
//! against every bound port, the step table's entries, the object
//! callbacks, the start-up and per-frame steps and the bars' set-up
//! (internal). Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::too_many_lines
)]

mod magic;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, Xmm, m0020};
use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use magic_owna::{dump, f, rec, recs, twin};

const N: u16 = 20;
const OBJ: u32 = SCRATCH + 0x2000;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// A slot that fills `n` words at its first argument (a matrix or a
/// vector the caller passes) and records its words.
fn filler(t: &mut Twin, slot: u32, words: usize, n: u32) {
    rec(t, slot, words, move |m: &mut dyn StubMem, a: &[u32]| {
        let k = m.r32(magic_owna::row(slot));
        for i in 0..n {
            m.w32(
                a[0] + 4 * i,
                f(0.5 + i as f32 * 0.25 + (slot + k) as f32 * 0.125),
            );
        }
        slot
    });
}

#[test]
fn steps() {
    let Some(mut t) = twin(N) else { return };
    recs(
        &mut t,
        &[(374, 0), (375, 1), (372, 0), (933, 4), (846, 0), (804, 5)],
    );
    // 1000: the byte at *G_B
    let gb = t.img.word(t.at(0x1000) + 0x0e);
    t.set_u32(gb, SCRATCH + 0x100);
    t.set_u8(SCRATCH + 0x100, 0xa7);
    t.set_u8(SCRATCH + 0x101, 0x5c);
    t.check_entry(t.at(0x1000), Call::default(), &[], Eax::Checked);
    // 1030: slot 373's answer
    let ans = SCRATCH + 0x104;
    rec(&mut t, 373, 0, move |m, _| m.r32(ans));
    for r in [0u32, 5, 4, 0xffff_ffff] {
        t.set_u32(ans, r);
        t.check_entry(t.at(0x1030), Call::default(), &[], Eax::Checked);
    }
    t.check_entry(t.at(0x1060), Call::default(), &[], Eax::Checked);
    t.check_entry(t.at(0x1d90), Call::default(), &[], Eax::Checked);
    // 1080: the frame counter and the counter
    let e = t.at(0x1080);
    let (frame, n) = (t.img.word(e + 0x02), t.img.word(e + 0x1c));
    for (fr, c) in [
        (0u32, 0u32),
        (5, 6),
        (7, 6),
        (10, 7),
        (3, 9),
        (0xffff_fffb, 2),
        (4, 0xffff_ffff),
        (0, 0xffff_ffff),
        (0, 0xffff_fffe),
        (0x8000_0000, 1),
    ] {
        t.set_u32(frame, fr);
        t.set_u32(n, c);
        t.check_entry(e, Call::default(), &[], Eax::Checked);
    }
    // 10d0: the fade up and down, clamped
    let e = t.at(0x10d0);
    let (done, v) = (t.img.word(e + 0x02), t.img.word(e + 0x13));
    let end = f32::from_bits(t.u32(t.img.word(e + 0x99)));
    let max = f32::from_bits(t.u32(t.img.word(e + 0x0b)));
    for (d, x) in [
        (0u32, 0.0f32),
        (0, max),
        (0, max - 1e-3),
        (0, end),
        (1, 0.0),
        (1, 1e-3),
        (1, max),
        (2, 0.5),
        (1, end),
    ] {
        t.set_u32(done, d);
        t.set_u32(v, f(x));
        t.check_entry(e, Call::default(), &[], Eax::Checked);
    }
}

/// The object callbacks' slots.
fn object_stubs(t: &mut Twin) -> u32 {
    let gp = t.img.word(t.at(0x13d0) + 0x04);
    t.set_u32(gp, SCRATCH + 0x200);
    let lock = t.img.word(t.at(0x1470) + 0x1c8);
    t.set_u32(lock, SCRATCH + 0x204);
    t.set_u32(SCRATCH + 0x204, 9);
    let gd = t.img.word(t.at(0x13d0) + 0x10);
    t.set_u32(gd, SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    let list = SCRATCH + 0x400;
    t.set_u32(list, 0x1111);
    t.set_u32(list + 4, 0x2222);
    rec(t, 941, 1, move |_, _| list);
    // pppStartPart: what it sees, and the lock during the call
    rec(t, 714, 5, |m, _| m.r32(SCRATCH + 0x204) ^ 0x714);
    recs(t, &[(723, 1), (724, 2), (713, 1)]);
    filler(t, 327, 4, 16);
    // 230 fills its first argument and keeps its second (C in the end
    // part), each call in a row of its own
    rec(t, 230, 5, |m, a| {
        let k = m.r32(magic_owna::row(230));
        if a[1] >= 0x2000_0000 {
            dump(m, 0x280 + k % 0x40, a[1], 16);
        }
        for i in 0..16 {
            m.w32(a[0] + 4 * i, f(0.75 + i as f32 * 0.5 + k as f32));
        }
        0x230
    });
    filler(t, 235, 7, 16);
    rec(t, 231, 8, |m, a| {
        let k = m.r32(magic_owna::row(231));
        dump(m, 0x200 + k % 0x40, a[2], 16);
        0x231
    });
    rec(t, 229, 3, |m, a| {
        dump(m, 229, a[0], 4);
        for i in 0..4 {
            m.w32(a[0] + 4 * i, f(1.5 - i as f32));
        }
        0x229
    });
    let ans = SCRATCH + 0x208;
    rec(t, 712, 12, move |m, _| m.r32(ans) ^ m.r32(SCRATCH + 0x204));
    ans
}

#[test]
fn objects() {
    let Some(mut t) = twin(N) else { return };
    let ans = object_stubs(&mut t);
    let base = t.img.word(t.at(0x16e0) + 0x0a);
    for obj in [OBJ, base, base + 0x80] {
        t.set_u16(obj + 0x76, 0x1234);
        t.check_entry(t.at(0x13d0), Call::default(), &[obj], Eax::Checked);
        t.check_entry(t.at(0x16e0), Call::default(), &[obj], Eax::Checked);
    }
    // the camera callbacks: before and after the drift starts, the part
    // done or not
    let e = t.at(0x1470);
    let gt = t.img.word(e + 0xc1);
    let gt2 = t.img.word(t.at(0x17b0) + 0xe0);
    for (frame, done) in [(0u32, 0u32), (0x4b, 1), (0x4c, 0), (0x74, 1), (0x73, 0)] {
        t.set_u32(gt, frame);
        t.set_u32(gt2, frame);
        t.set_u32(ans, done ^ 9);
        for obj in [OBJ, base, base + 0x80] {
            t.set_u16(obj + 0x76, 0xf000);
            t.check_entry(e, Call::default(), &[obj], Eax::Checked);
            if obj != OBJ {
                t.check_entry(t.at(0x17b0), Call::default(), &[obj], Eax::Checked);
            }
        }
    }
}

/// `17b0` for an object past the second (the vector left unwritten): the
/// port's zeros against the original's stack junk, so only the calls'
/// pointers are compared (slot 229 records none of the vector).
#[test]
fn object_17b0_other_index() {
    let Some(mut t) = twin(N) else { return };
    let ans = object_stubs(&mut t);
    rec(&mut t, 229, 3, |_, _| 0x229);
    rec(&mut t, 230, 5, |_, _| 0x230);
    rec(&mut t, 231, 8, |_, _| 0x231);
    for done in [0u32, 1] {
        t.set_u32(ans, done ^ 9);
        t.check_entry(t.at(0x17b0), Call::default(), &[OBJ], Eax::Checked);
    }
}

#[test]
fn start_up() {
    let Some(mut t) = twin(N) else { return };
    let e = t.at(0x1a10);
    let lock = t.img.word(e + 0xa9);
    t.set_u32(lock, SCRATCH + 0x204);
    t.set_u32(SCRATCH + 0x204, 5);
    let bytes = SCRATCH + 0x500;
    for i in 0..8u32 {
        t.set_u8(bytes + i, u8::try_from(0x30 + i).unwrap());
    }
    let count = t.img.word(e + 0x223);
    let cfg = SCRATCH + 0x210;
    rec(&mut t, 803, 2, |_, _| SCRATCH + 0x1_0000);
    // the entries: blocks whose part words differ
    for i in 0..8u32 {
        t.set_u32(SCRATCH + 0x600 + i * 4, 0x0600_0000 + i);
    }
    rec(&mut t, 719, 10, |_, a| SCRATCH + 0x600 + a[1]);
    rec(&mut t, 711, 16, |m, _| {
        let v = m.r32(SCRATCH + 0x204);
        m.w32(SCRATCH + 0x220, v);
        0x711
    });
    rec(&mut t, 941, 5, |_, a| a[0] ^ 0x941);
    rec(&mut t, 368, 5, move |m, a| {
        let n = m.r32(cfg);
        m.w32(a[1], n);
        bytes
    });
    rec(&mut t, 215, 1, |_, a| 0x0215_0000 + a[0]);
    rec(&mut t, 367, 14, move |m, a| {
        let v = m.r32(cfg + 4);
        if v != 0 {
            m.w32(a[4], v);
        }
        0x367
    });
    rec(&mut t, 369, 15, move |m, _| m.r32(cfg + 8));
    recs(&mut t, &[(718, 3), (221, 7), (710, 4), (219, 2), (370, 5)]);
    // the blocks dirty before
    for o in [0x14d, 0x16d, 0x19c, 0x1e2] {
        let b = t.img.word(e + o);
        t.write(b, &[0xa5; 0x110]);
    }
    for (n, rewrite, r369, arg) in [
        (0u32, 0u32, 0u32, 0u32),
        (3, 0, 1, 0),
        (1, 7, 0, 0),
        (5, 0, 2, 1),
        (2, 0, 2, 0),
        (0xffff_ffff, 0, 0, 3),
    ] {
        t.set_u32(cfg, n);
        t.set_u32(cfg + 4, rewrite);
        t.set_u32(cfg + 8, r369);
        t.set_u32(count, 0x99);
        t.check_entry(e, Call::default(), &[arg], Eax::Checked);
    }
}

#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let e = t.at(0x1e00);
    let cfg = SCRATCH + 0x210;
    let list = SCRATCH + 0x500;
    for i in 0..8u32 {
        t.set_u8(list + i, u8::try_from(0x40 + i).unwrap());
    }
    t.set_u32(t.img.word(e + 0x15c), list);
    rec(&mut t, 253, 0, move |m, _| m.r32(cfg));
    rec(&mut t, 225, 1, move |m, _| m.r32(cfg + 4));
    rec(&mut t, 227, 1, move |m, _| m.r32(cfg + 8));
    rec(&mut t, 377, 4, |_, _| 0x377);
    rec(&mut t, 288, 12, move |m, a| {
        m.w32(a[1], 3);
        list + 2
    });
    rec(&mut t, 368, 2, move |m, a| {
        m.w32(a[1], m.r32(cfg + 0xc));
        list + 4
    });
    rec(&mut t, 210, 6, |m, a| {
        m.w32(a[1], 0x210);
        0
    });
    rec(&mut t, 325, 8, |m, a| {
        m.w32(a[1], 0x325);
        0
    });
    t.stub_st0(365, |_, a| f64::from(a[0]) * 0.5 + 0.25);
    t.stub_st0(364, |_, a| -f64::from(a[0] & 0xfff) * 0.75 - 0.5);
    recs(
        &mut t,
        &[
            (226, 2),
            (381, 5),
            (276, 2),
            (366, 2),
            (365, 3),
            (364, 4),
            (232, 10),
            (296, 2),
        ],
    );
    let count = t.img.word(e + 0x10a);
    let handles = t.img.word(e + 0x11a);
    for i in 0..4u32 {
        t.set_u32(handles + 4 * i, 0x0100 + i);
    }
    let pair = t.img.word(e + 0x133);
    t.set_u32(pair, 0x77);
    t.set_u32(pair + 4, 0x78);
    t.set_u32(t.img.word(e + 0x2ca), f(3.0));
    t.set_u32(t.img.word(e + 0x2d2), f(-4.5));
    let vec = t.img.word(e + 0x1f2);
    for i in 0..3u32 {
        t.set_u32(vec + 4 * i, f(2.0 + i as f32));
    }
    let flags = [0x3a, 0x66, 0x91, 0xbf, 0xcc].map(|o| t.img.word(e + o));
    // (253, 225, 227, n0, flags, count)
    for (r253, r225, r227, n0, fl, n) in [
        (0u32, 0u32, 1u32, 0u32, [0u32; 5], 0u32),
        (1, 1, 1, 0, [0, 0, 0, 1, 0], 0),
        (1, 0, 0, 0, [0, 0, 0, 1, 0], 0),
        (1, 0, 1, 0, [0, 0, 0, 0, 0], 0),
        (1, 0, 1, 2, [0, 0, 0, 1, 0], 3),
        (1, 0, 1, 0, [1, 1, 1, 1, 0xffff_ffff], 0),
        (1, 0, 0xffff_ffff, 0, [1, 1, 0, 1, 5], 1),
        (1, 0, 1, 0, [1, 1, 1, 1, 2], 0),
        (1, 0, 1, 0, [1, 1, 1, 1, 0], 1),
    ] {
        t.set_u32(cfg, r253);
        t.set_u32(cfg + 4, r225);
        t.set_u32(cfg + 8, r227);
        t.set_u32(cfg + 0xc, n0);
        for (g, v) in flags.iter().zip(fl) {
            t.set_u32(*g, v);
        }
        t.set_u32(count, n);
        t.check_entry(e, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn bars() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2b00);
    recs(&mut t, &[(821, 1), (822, 4), (823, 2), (824, 4)]);
    let rec_ = SCRATCH + 0x800;
    for (a, b, c, s0, s1) in [
        (1.0f32, 2.0f32, 3.0f32, 4.0f32, 5.0f32),
        (-0.5, 100.0, -7.25, 0.0, 1e-3),
        (1e12, -1e12, 0.125, 3e9, -3e9),
        (f32::NAN, 0.0, 1.0, -1.0, 2.0),
    ] {
        t.write(rec_, &[0x5a; 0x100]);
        let c_ = Call::this(rec_).with_xmm(&[Xmm::f32(0.0), Xmm::f32(a), Xmm::f32(b), Xmm::f32(c)]);
        t.check(at, c_, &[f(s0), f(s1)], Eax::Junk, |g, img, fr| {
            let s0 = f32::from_bits(fr.arg(g, 0));
            let s1 = f32::from_bits(fr.arg(g, 1));
            m0020::bars(
                g,
                img,
                fr.at,
                fr.ecx,
                fr.xmm_f32(1),
                fr.xmm_f32(2),
                fr.xmm_f32(3),
                s0,
                s1,
            );
            Ret::default()
        });
    }
}

/// A timeline with one live event and no callbacks (`timeline_run`'s
/// layout), its events at `events`.
fn live_timeline(t: &mut Twin, tl: u32, events: u32, live: bool) {
    t.set_u32(tl, 5);
    t.set_u32(tl + 4, 1);
    t.set_u32(tl + 8, u32::from(live));
    t.set_u32(tl + 0xc, u32::from(live));
    t.set_u32(tl + 0x10c, events);
    t.write(events, &[0; 0x18]);
    t.set_u32(events + 4, 9);
}

/// 0x10001180: the level falling and rising (to and past its limits,
/// held below the start time), state 7 and another, the release off and
/// with no, two and three handles.
#[test]
fn fade_frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1180);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    recs(
        &mut t,
        &[(821, 1), (822, 4), (823, 2), (824, 2), (845, 0), (276, 2)],
    );
    let k = f32::from_bits(t.u32(w(&t, 0x0b)));
    let d = f32::from_bits(t.u32(w(&t, 0x26)));
    for (i, (c, time, level, state, n)) in [
        (0u32, 5u32, 0.5f32, 0u32, 0u32),
        (0, 6, 0.5, 7, 0),
        (0, 6, k - d * 0.5, 0, 0),
        (0, 6, k, 7, 2),
        (1, 6, d * 0.5, 7, 0),
        (1, 0, 0.75, 7, 3),
        (1, 9, k, 3, 2),
        (0, 4, f32::NAN, 3, 0),
        (1, 4, f32::NAN, 6, 1),
    ]
    .into_iter()
    .enumerate()
    {
        t.set_u32(w(&t, 0x02), c);
        t.set_u32(w(&t, 0x1b), time);
        t.set_u32(w(&t, 0x13), f(level));
        t.set_u32(w(&t, 0x8e), state);
        t.set_u32(w(&t, 0xf0), n);
        for j in 0..3 {
            t.set_u32(w(&t, 0x10a) + 4 * j, 0x00ab_0000 + j);
        }
        for (j, a) in (0u32..).zip([w(&t, 0x123), w(&t, 0x123) + 4, w(&t, 0x145)]) {
            t.set_u32(
                a,
                if (i as u32 + j).is_multiple_of(2) {
                    0
                } else {
                    0x00cd_0000 + j
                },
            );
        }
        for (j, off) in (0u32..).zip([0x83, 0x7b, 0xab, 0x73]) {
            t.set_u32(w(&t, off), f(1.5 + j as f32 - i as f32 * 0.25));
        }
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x100012e0: the fade's end with the level below, at and above its
/// top (and NaN), with a live timeline event and none.
#[test]
fn fade_end() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x12e0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    recs(
        &mut t,
        &[
            (382, 2),
            (371, 2),
            (315, 1),
            (222, 4),
            (378, 1),
            (380, 1),
            (379, 1),
            (253, 0),
        ],
    );
    t.set_u32(w(&t, 0x02), SCRATCH + 0x300);
    t.set_u32(w(&t, 0x8e), 0x008e_0000);
    let k = f32::from_bits(t.u32(w(&t, 0x58)));
    for (i, (level, live)) in [(k, 0u32), (k * 0.5, 0), (k, 1), (f32::NAN, 0), (k * 2.0, 1)]
        .into_iter()
        .enumerate()
    {
        t.set_u32(w(&t, 0x51), f(level));
        t.set_u32(w(&t, 0x07), 0x0e0e_0000 + i as u32);
        for (k, tl) in [0x44, 0xce, 0xd8, 0xe2].into_iter().enumerate() {
            let tl = w(&t, tl);
            live_timeline(&mut t, tl, SCRATCH + 0x5000 + 0x20 * k as u32, live != 0);
        }
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10002120: off, the flag clear, the counter's first and later frames,
/// and the release with no, two and three bytes from slot 288 (and with
/// the slot leaving the count as the caller's `ecx` had it).
#[test]
fn close_frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2120);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    recs(
        &mut t,
        &[
            (723, 1),
            (724, 2),
            (239, 3),
            (214, 1),
            (376, 1),
            (381, 2),
            (276, 2),
            (296, 2),
        ],
    );
    // slot 220 moves the counter on (the original reads it again after)
    let counter = w(&t, 0x1b1);
    rec(&mut t, 220, 3, move |m, _| {
        m.w32(counter, 7);
        0x220
    });
    let bytes = SCRATCH + 0x400;
    t.write(bytes, &[9, 8, 7, 6]);
    rec(&mut t, 288, 2, move |m, a| {
        let n = m.r32(SCRATCH + 0x410);
        if n != 0xffff {
            m.w32(a[1], n);
        }
        bytes
    });
    t.set_u32(w(&t, 0x14), SCRATCH + 0x300);
    t.set_u32(w(&t, 0x46), SCRATCH + 0x304);
    // every global read holds its own value
    for (k, op) in [0x1a, 0x38, 0x4b, 0x6e, 0x7b, 0x1cd, 0xd1, 0x196]
        .into_iter()
        .enumerate()
    {
        t.set_u32(w(&t, op), 0x0a00_0000 + k as u32);
    }
    for j in 0..3 {
        t.set_u32(w(&t, 0x10a) + 4 * j, 0x00ab_0000 + j);
    }
    for op in [w(&t, 0x123), w(&t, 0x123) + 4, w(&t, 0x145)] {
        t.set_u32(op, 0x00cd_0000 + op % 0x100);
    }
    for (k, op) in [0xa3, 0xad, 0xb7, 0xc1].into_iter().enumerate() {
        let tl = w(&t, op);
        live_timeline(&mut t, tl, SCRATCH + 0x5000 + 0x20 * k as u32, true);
    }
    let base = t.save();
    for (on, flag, e, counter, n, ecx) in [
        (0u32, 0u32, 0u32, 0u32, 0u32, 0u32),
        (1, 0, 0, 0, 0, 0),
        (1, 1, 0, 0, 0, 0),
        (1, 1, 0, 3, 0, 0),
        (1, 1, 1, 0, 0, 0),
        (1, 1, 1, 0, 2, 0),
        (1, 1, 1, 0, 0xffff, 3),
    ] {
        t.restore(&base);
        t.set_u32(w(&t, 0x06), on);
        t.set_u32(w(&t, 0x54), flag);
        t.set_u32(w(&t, 0x95), e);
        t.set_u32(w(&t, 0x1b1), counter);
        t.set_u32(SCRATCH + 0x410, n);
        t.set_u32(w(&t, 0xf3), n & 3);
        let c = Call::this(ecx);
        t.check_entry(at, c, &[], Eax::Checked);
    }
}
