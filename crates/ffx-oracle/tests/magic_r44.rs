//! Round 44's ports (`ffx_magic::r44`) against each original. Needs
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
#[path = "magic/sa.rs"]
mod sa;

use magic::{Call, Eax, SCRATCH, StubMem, TRAPS, Twin};
use ownb::rec;

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

fn rf(m: &dyn StubMem, at: u32) -> f32 {
    f32::from_bits(m.r32(at))
}

fn wf(m: &mut dyn StubMem, at: u32, v: f32) {
    m.w32(at, v.to_bits());
}

/// `n` bytes of junk at `at` (varied by `k`).
fn junk(t: &mut Twin, at: u32, n: u32, k: u32) {
    let b: Vec<u8> = (0..n)
        .map(|i| (i.wrapping_mul(0x3d) ^ (0x71 + k)).to_le_bytes()[0])
        .collect();
    t.write(at, &b);
}

/// `n` floats at `at`: `x0 + 0.25 i` (no NaNs).
fn floats(t: &mut Twin, at: u32, n: u32, x0: f32) {
    for i in 0..n {
        t.set_u32(at + 4 * i, (x0 + 0.25 * i as f32).to_bits());
    }
}

/// Every slot answering `first word ^ (slot << 20)`.
fn all_stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
}

/// The visit of a node's vtable (`+0x1c`): logs the cursor's node and its
/// mark byte; `class + 0x1c` routed to it.
fn visit_stub(t: &mut Twin, visit: u32, class: u32) {
    t.stub(visit, move |m, a| {
        let k = m.r32(SCRATCH + 0x3800);
        m.w32(SCRATCH + 0x3800, k + 1);
        let cur = m.r32(a[0]);
        m.w32(SCRATCH + 0x3810 + 8 * (k % 8), cur);
        m.w32(
            SCRATCH + 0x3814 + 8 * (k % 8),
            (m.r32(a[0] + 4) >> 8) & 0xff,
        );
        0x1000 + k
    });
    t.set_u32(class + 0x1c, TRAPS + 16 * visit);
}

/// The slots the summon set-ups and their bone call, with float results
/// and the vectors they fill (all logged).
fn summon_stubs(t: &mut Twin) {
    all_stubs(t);
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    for s in [330u32, 328, 329] {
        rec(t, s, 3, move |m, a| {
            let ang = f32::from_bits(a[2]);
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) + ang * (i + 1) as f32 / 8.0 + s as f32 / 1024.0;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[1] + 0x30 + 4 * i) + 2.0 * rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    for (s, x) in [(15u32, 1.5f64), (16, -0.75), (17, 3.0), (277, 40.0)] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    rec(t, 232, 2, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * 0.5;
            wf(m, a[0] + 4 * i, x);
        }
        0x232
    });
    rec(t, 233, 3, |m, a| {
        let s = f32::from_bits(a[2]);
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * s;
            wf(m, a[0] + 4 * i, x);
        }
        0x233
    });
    rec(t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x325
    });
    rec(t, 218, 2, |_, a| SCRATCH + 0x700 + (a[0] & 1));
    t.write(SCRATCH + 0x700, &[3, 1, 4, 1, 5, 2, 6, 0]);
    rec(t, 262, 4, |m, a| {
        let k = (a[0] & 0xff) as f32;
        wf(m, a[1], 2.0 * (k - 2.0));
        wf(m, a[2], 0.5 * k);
        wf(m, a[3], 3.0 - k);
        0x262
    });
    rec(t, 366, 2, |m, a| {
        for i in 0..3 {
            wf(m, a[1] + 4 * i, 0.75 * i as f32 - (a[0] & 7) as f32);
        }
        0x366
    });
    rec(t, 14, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], 0.25 * i as f32);
        }
        0x14
    });
    rec(t, 601, 2, |m, a| {
        wf(m, a[1], 0.25 * (a[0] & 0xf) as f32 - 1.0);
        0x601
    });
    rec(t, 290, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, 1.5 - i as f32);
        }
        0x290
    });
    // slot 390 answers 0x1084 at its call number `SCRATCH + 0x7814` (the
    // count at `+0x7810`; the models' `+0x50` words are the node set-up's)
    rec(t, 390, 1, |m, a| {
        let k = m.r32(SCRATCH + 0x7810);
        m.w32(SCRATCH + 0x7810, k + 1);
        if k == m.r32(SCRATCH + 0x7814) {
            0x1084
        } else {
            0x1083 ^ (a[0] & 0x10)
        }
    });
    rec(t, 268, 3, |m, a| {
        for i in 0..3 {
            wf(
                m,
                a[2] + 4 * i,
                0.5 * (a[0] & 0xf) as f32 - i as f32 + (a[1] & 0xff) as f32,
            );
        }
        0x268
    });
    for s in [333u32, 211, 413, 215, 12] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xf) * 0.25 + 0.5);
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xf) * 0.5 - 1.0);
    rec(t, 327, 2, |m, a| {
        for i in 0..16 {
            wf(
                m,
                a[0] + 4 * i,
                if i % 5 == 0 { 1.0 } else { 0.125 * i as f32 },
            );
        }
        0x327
    });
    rec(t, 32, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, 0.5 * i as f32 - 1.0);
        }
        0x32
    });
}

/// One summon set-up copy: its address, the operands of its node vtables,
/// the bone's object and call, the block's count and model layout.
struct Setup {
    dll: u16,
    at: u32,
    vts: [u32; 9],
    fourth: u32,
    bone: u32,
    count: u32,
    base: u32,
    stride: u32,
    matrix: u32,
}

const SETUP_0155: Setup = Setup {
    dll: 155,
    at: 0x75b0,
    vts: [0xeb, 0x1b5, 0x2cb, 0x305, 0x37c, 0x47c, 0x4f4, 0x539, 0x600],
    fourth: 0x3d9,
    bone: 0x3ef,
    count: 0x163c,
    base: 0xac,
    stride: 0x278,
    matrix: 0x1508,
};

const SETUP_0164: Setup = Setup {
    dll: 164,
    at: 0x75f0,
    vts: [0xd2, 0x142, 0x2e9, 0x323, 0x39b, 0x4bd, 0x535, 0x57a, 0x641],
    fourth: 0x425,
    bone: 0x435,
    count: 0x162c,
    base: 0xbc,
    stride: 0x274,
    matrix: 0x14f8,
};

/// The node vtables' visits and the bone's function table routed to
/// stubs; returns the bone's fourth object's global and the class.
fn setup_stubs(t: &mut Twin, f: u32, s: &Setup) -> (u32, u32) {
    summon_stubs(t);
    let (visit, class) = (1000u32, SCRATCH + 0x3700);
    visit_stub(t, visit, class);
    for op in s.vts {
        let vt = t.img.word(f + op);
        t.set_u32(vt + 0x1c, TRAPS + 16 * visit);
    }
    let bt = t.img.rel32_target(f + s.bone);
    let table = t.img.word(bt + 0xed);
    for i in 0..4 {
        t.set_u32(table + 4 * i, TRAPS + 16 * 32);
    }
    (t.img.word(f + s.fourth), class)
}

/// The block `blk` with `n` models (flag bit 0 alternating with `k`;
/// slot 390 answering 0x1084 for model `own`).
fn summon_block(t: &mut Twin, s: &Setup, blk: u32, n: u32, k: u32, own: Option<u32>) {
    t.set_u32(SCRATCH + 0x7810, 0);
    t.set_u32(SCRATCH + 0x7814, own.unwrap_or(0xffff));
    junk(t, blk, 0x1700, k);
    floats(t, blk + s.matrix, 16, 0.5 + k as f32);
    t.set_u32(blk + s.count, n);
    for i in 0..n {
        let o = blk + s.base + s.stride * i;
        t.set_u8(o + 0x90, u8::from((i + k) % 2 == 1));
        t.set_u32(o + 0x54, 0x0005_4000 + 3 * i + k);
    }
}

/// A summon set-up copy: models two, one, none and two (one flagged:
/// slot 333 / slot 366 skipped); with a parent and without; for 0164 a
/// model whose slot 390 answers 0x1084 (first and second).
fn setup_cases(
    s: &Setup,
    port: fn(&mut magic::PortGame, &ffx_magic::Image, &ffx_magic::Frame) -> ffx_magic::Ret,
) {
    let Some(mut t) = twin(s.dll) else { return };
    let f = t.at(s.at);
    let (fourth, class) = setup_stubs(&mut t, f, s);
    let (g, blk, p) = (SCRATCH + 0x100, SCRATCH + 0x1_0000, SCRATCH + 0x7000);
    // (models, parent, flags byte, the model answering 0x1084)
    let cases: [(u32, bool, u8, Option<u32>); 6] = [
        (2, true, 1, None),
        (1, false, 0, None),
        (0, true, 4, None),
        (2, false, 0x11, None),
        (2, true, 0, Some(0)),
        (3, false, 8, Some(1)),
    ];
    for (k, (n, with_parent, fl, own)) in (0u32..).zip(cases) {
        summon_block(&mut t, s, blk, n, k, own);
        junk(&mut t, p, 0x40, k + 1);
        t.set_u32(p + 0x10, p);
        t.set_u32(p + 0xc, p);
        t.set_u32(g, blk);
        t.set_u32(g + 4, if with_parent { p } else { 0 });
        t.set_u32(g + 8, 0x0008_0001 + k);
        t.set_u32(g + 0xc, 0x000c_0002 + k);
        t.set_u8(g + 0x10, fl);
        t.set_u32(fourth, class);
        ownb::dirty_stack(&mut t, s.at + k, &[]);
        t.check(f, Call::this(g), &[], Eax::Checked, port);
    }
}

/// `magic_0155` 0x100075b0 (`r44::summon_setup_0155`).
#[test]
fn summon_setup_0155() {
    setup_cases(&SETUP_0155, |g, img, fr| {
        ffx_magic::r44::summon_setup_0155(g, img, fr)
    });
}

/// `magic_0164` 0x100075f0 (`r44::summon_setup_0164`).
#[test]
fn summon_setup_0164() {
    setup_cases(&SETUP_0164, |g, img, fr| {
        ffx_magic::r44::summon_setup_0164(g, img, fr)
    });
}

/// The summon starts `magic_0155` 0x10008eb0 and `magic_0164` 0x10008e40
/// (`r44::summon_start_*`): the group on the frame, the block's vtable,
/// the set-up on the DLL's block (two models, one flagged; one; none).
#[test]
fn summon_starts() {
    type Port = fn(&mut magic::PortGame, &ffx_magic::Image, &ffx_magic::Frame) -> ffx_magic::Ret;
    let starts: [(&Setup, u32, Port); 2] = [
        (&SETUP_0155, 0x8eb0, |g, img, fr| {
            ffx_magic::r44::summon_start_0155(g, img, fr)
        }),
        (&SETUP_0164, 0x8e40, |g, img, fr| {
            ffx_magic::r44::summon_start_0164(g, img, fr)
        }),
    ];
    for (s, at, port) in starts {
        let Some(mut t) = twin(s.dll) else { return };
        let f = t.at(s.at);
        let (fourth, class) = setup_stubs(&mut t, f, s);
        let w = t.at(at);
        let blk = t.img.word(w + 0x11);
        let p = t.img.word(w + 0x18);
        let a = t.img.word(w + 0x7);
        for (k, n) in (0u32..).zip([2u32, 1, 0]) {
            summon_block(&mut t, s, blk, n, k, None);
            junk(&mut t, p, 0x40, k + 1);
            t.set_u32(p + 0x10, p);
            t.set_u32(p + 0xc, p);
            t.set_u32(a, 0x00a0_0001 + k);
            t.set_u32(fourth, class);
            ownb::dirty_stack(&mut t, at + k, &[]);
            t.check(w, Call::this(0x5eed_0000 + k), &[], Eax::Checked, port);
        }
    }
}

/// `magic_0155` 0x10009570 (`r44::throw_task_0155`, cdecl `(task)`): every
/// state (10, 11 and 12 below and at their counts, 13, 20 with the
/// rethrow, 21 with targets masked, shown and hidden, none, 30, 31 both
/// ways, 10000, the table's gaps, below and above it); the parent with
/// children and without.
#[test]
fn throw_task_0155() {
    let Some(mut t) = twin(155) else { return };
    all_stubs(&mut t);
    let bones = SCRATCH + 0x7a0;
    rec(&mut t, 262, 4, move |m, a| {
        for i in 0..3 {
            let v = rf(m, bones + 4 * i as u32) + (a[0] & 7) as f32;
            wf(m, a[1 + i], v);
        }
        0x262
    });
    for (s, x) in [(12u32, -1.5f64), (251, 0.75)] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    rec(&mut t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, |m, _| f64::from(m.r32(SCRATCH + 0x7f8) & 0xff) * 0.01);
    rec(&mut t, 719, 3, |_, a| {
        0x0719_0000 ^ a[0] ^ a[1] ^ (a[2] << 8)
    });
    rec(&mut t, 306, 3, |_, a| {
        0x0306_0000 ^ a[0] ^ (a[1] << 4) ^ a[2]
    });
    rec(&mut t, 324, 0, |m, _| {
        let k = m.r32(SCRATCH + 0x7f8);
        m.w32(
            SCRATCH + 0x7f8,
            k.wrapping_mul(0x0101_0101).wrapping_add(0x3779),
        );
        k
    });
    let f = t.at(0x9570);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let task = SCRATCH + 0x100;
    let node = SCRATCH + 0x180;
    let r = SCRATCH + 0x200;
    let s = SCRATCH + 0x1_0000;
    // the particle node's program table owner and table (slot 941)
    let owner = SCRATCH + 0x6200;
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    t.stub(941, |m, a| {
        m.w32(SCRATCH + 0x900, a[0]);
        SCRATCH + 0x400
    });
    let pw = t.img.rel32_target(f + 0x23d);
    let (l2, owner_at) = (t.img.word(pw + 0x26), t.img.word(pw + 0x0d));
    // (state, target count, mask, shown words, R + 0x30)
    let cases: [(u32, i32, u32, [u16; 3], u32); 18] = [
        (10, 2, 7, [1, 1, 1], 0x50),
        (11, 2, 7, [1, 1, 1], 0x27),
        (11, 2, 7, [1, 1, 1], 0x28),
        (12, 2, 7, [1, 1, 1], 0x81),
        (12, 2, 7, [1, 1, 1], 0x82),
        (13, 2, 7, [1, 1, 1], 0x90),
        (14, 2, 7, [1, 1, 1], 0x90),
        (5, 2, 7, [1, 1, 1], 0x90),
        (20, 3, 0, [1, 0, 1], 1),
        (21, 3, 5, [1, 1, 1], 2),
        (21, 2, 2, [0, 1, 1], 3),
        (21, 0, 3, [1, 1, 1], 4),
        (30, 2, 7, [1, 1, 1], 0x30),
        (31, 2, 7, [1, 1, 1], 0x13),
        (31, 2, 7, [1, 1, 1], 0x14),
        (10000, 2, 7, [1, 1, 1], 5),
        (25, 2, 7, [1, 1, 1], 6),
        (40, 2, 7, [1, 1, 1], 7),
    ];
    for (k, (st, n, mask, shown, count)) in (0u32..).zip(cases) {
        junk(&mut t, r, 0x200, k);
        t.set_u32(task + 4, r);
        t.set_u32(task, node);
        let (nx, pv) = (SCRATCH + 0x1c0, SCRATCH + 0x1e0);
        t.write(node, &[0x44; 0x20]);
        t.set_u32(node + 4, nx);
        t.set_u32(node + 8, pv);
        t.write(nx, &[0x45; 0x10]);
        t.write(pv, &[0x46; 0x10]);
        t.set_u32(r + 0x18, st);
        t.set_u32(r + 0x1c, s);
        t.set_u32(r + 0x30, count);
        t.set_u32(r + 0x34, mask);
        junk(&mut t, s, 0x1700, k + 1);
        floats(&mut t, s + 0x15d8, 1, 2.5 + k as f32);
        // the flag byte with bit 1 clear (the junk may have it set)
        t.set_u8(s + 0x1650, 0x41 ^ (k as u8 & 0xf0));
        t.set_u32(s + 0x163c, n.cast_unsigned());
        for i in 0..3u32 {
            let e = s + 0x100 + 0x278 * i;
            t.set_u16(e + 0x218, shown[i as usize]);
            t.set_u32(e, 0x00ac_0000 + i);
            t.set_u32(e.wrapping_sub(4), 0x00ad_0000 + i);
        }
        for i in 0..4u32 {
            t.set_u32(bones + 4 * i, (0.5 * i as f32 + k as f32).to_bits());
        }
        // the parents' children: none (the last child the parent) or one
        let kid = SCRATCH + 0x6000;
        t.write(kid, &[0x22; 0x20]);
        for (j, o) in [0x150u32, 0x1a6, 0x20c].into_iter().enumerate() {
            let parent = op(&t, o);
            let last = if (k + j as u32) % 2 == 1 { parent } else { kid };
            t.set_u32(parent + 0x10, last);
            t.set_u32(parent + 0xc, last);
            t.set_u16(parent + 0x18, 3 + k as u16);
        }
        // the particle node's list: a static node with no children
        t.write(l2, &[0; 0x20]);
        t.set_u32(l2 + 0xc, l2);
        t.set_u32(l2 + 0x10, l2);
        junk(&mut t, owner, 0x100, k);
        t.set_u32(owner_at, owner);
        ownb::dirty_stack(&mut t, 0x9570 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0184` 0x10002300 (`r44::frame_0184`): off; the frame counts
/// around each step (the sounds at 0x83 and 0xd3, nothing before 0x83,
/// the blocks from 0x83, 0xd8, 0xdd and 0xf1 with their flags on and off,
/// slot 712 answering 0 and not), the end at 0x11e with slot 240 both
/// ways; the sprites with a record used and all free.
#[test]
fn frame_0184() {
    let Some(mut t) = twin(184) else { return };
    all_stubs(&mut t);
    rec(&mut t, 233, 3, |m, a| {
        let s = f32::from_bits(a[2]);
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * s + i as f32;
            wf(m, a[0] + 4 * i, x);
        }
        0x233
    });
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    // 712 logs the lock word it sees and answers 0 every third call
    let lock = SCRATCH + 0x7e10;
    rec(&mut t, 712, 2, move |m, a| {
        let k = m.r32(SCRATCH + 0x7e00);
        m.w32(SCRATCH + 0x7e00, k + 1);
        m.w32(SCRATCH + 0x7e40 + 4 * (k % 8), m.r32(lock));
        if k % 3 == 1 { 0 } else { 0x0712_0000 ^ a[0] }
    });
    rec(&mut t, 240, 1, |m, a| {
        m.r32(SCRATCH + 0x7e20) ^ (a[0] & 0x10)
    });
    t.stub_edx(772, |m, _| {
        m.r32(SCRATCH + 0x7f74).wrapping_mul(0x0101_0101)
    });
    rec(&mut t, 772, 2, |m, a| {
        let k = m.r32(SCRATCH + 0x7f74).wrapping_add(1);
        m.w32(SCRATCH + 0x7f74, k);
        a[0] ^ k
    });
    let f = t.at(0x2300);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    // the sprites' globals (r40's test of 0x10001030)
    let sp = t.at(0x1030);
    let sw = |t: &Twin, o: u32| t.img.word(sp + o);
    let slock = SCRATCH + 0x7f00;
    for o in [0xe0u32, 0x10e, 0x141, 0x15e, 0x1a0, 0x1d1] {
        let at = sw(&t, o);
        t.set_u32(at, slock);
    }
    let at = sw(&t, 0xc1);
    t.set_u32(at, SCRATCH + 0x7f80);
    let (first, end) = (sw(&t, 0x1d), sw(&t, 0x226));
    // the pointer globals: the matrix, the screen words, the lock
    for (o, v) in [
        (0x178u32, SCRATCH + 0x7c00),
        (0x10c, SCRATCH + 0x7d00),
        (0x137, SCRATCH + 0x7d10),
        (0x272, lock),
    ] {
        let at = w(&t, o);
        t.set_u32(at, v);
    }
    // (on, frame count, the four flags, slot 240's answer, a sprite used)
    let cases: [(bool, u32, [u16; 4], u32, bool); 14] = [
        (false, 0x83, [1; 4], 1, false),
        (true, 0x50, [1; 4], 1, false),
        (true, 0x83, [1, 1, 1, 1], 1, false),
        (true, 0x84, [0, 1, 1, 1], 1, true),
        (true, 0xd3, [1, 1, 1, 1], 1, false),
        (true, 0xd8, [1, 1, 1, 1], 1, false),
        (true, 0xd7, [1, 1, 1, 1], 1, false),
        (true, 0xdd, [1, 0, 1, 1], 1, false),
        (true, 0xdc, [1, 1, 1, 1], 1, false),
        (true, 0xf1, [1, 1, 1, 1], 1, true),
        (true, 0xf0, [1, 1, 1, 1], 1, false),
        (true, 0xf2, [1, 1, 1, 0], 1, false),
        (true, 0x11e, [0, 0, 0, 0], 0, false),
        (true, 0x11d, [1, 1, 0, 1], 1, true),
    ];
    let more: [(bool, u32, [u16; 4], u32, bool); 2] = [
        (true, 0x11e, [1, 1, 1, 1], 0x40, false),
        (true, 0x130, [1, 0, 0, 1], 0x80, true),
    ];
    for (k, (on, fc, flags, ans, used)) in (0u32..).zip(cases.into_iter().chain(more)) {
        let on_at = w(&t, 0x2);
        t.set_u32(on_at, u32::from(on) * (0x10 + k));
        let fc_at = w(&t, 0xe);
        t.set_u32(fc_at, fc);
        for (i, o) in [0x14fu32, 0x2b3, 0x416, 0x579].into_iter().enumerate() {
            let at = w(&t, o);
            // the u16 flag and a non-zero word after it
            t.set_u32(at, u32::from(flags[i] * (3 + k as u16)) | 0x5a5a_0000);
        }
        for o in [0xd7u32, 0xef, 0x107] {
            let at = w(&t, o);
            t.set_u32(at, 0x5a5a_0000 ^ o ^ k);
        }
        junk(&mut t, SCRATCH + 0x7c00, 0x40, k);
        // the screen words the pointer globals name
        junk(&mut t, SCRATCH + 0x7d00, 0x20, k + 7);
        floats(&mut t, SCRATCH + 0x7c00, 4, 0.5 * k as f32);
        t.set_u32(lock, 0x40 + k);
        t.set_u32(SCRATCH + 0x7e20, ans);
        // the sprite records: all free, or the first used
        let mut at = first;
        let mut i = 0u32;
        while at < end {
            let rec = at - 0x16;
            for j in 0..4u32 {
                t.set_u32(rec + 4 * j, (0.5 * j as f32 + i as f32).to_bits());
            }
            t.set_u32(rec + 0x10, (1.0 + 0.25 * i as f32).to_bits());
            t.set_u16(rec + 0x14, 0x100);
            t.set_u16(at, if used && i == 0 { 0 } else { 0xffff });
            i += 1;
            at += 0x18;
        }
        t.set_u32(slock, 0x50 + k);
        ownb::dirty_stack(&mut t, 0x2300 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// The matrix and vector slots of `motion_0184` as data flow: 237 a bone's
/// place, 233 `a = b * s`, 243/244 a rotation by the angle, 231 `a = b +
/// c` (sixteen), 230 a copy, 234 `a = b - c`, 232 `a = 2 b`, 229 `a = b
/// row 0 + 2 c`, 235 a unit matrix, 241 a "random" float.
fn motion_stubs(t: &mut Twin) {
    rec(t, 237, 4, |m, a| {
        for i in 0..4 {
            wf(
                m,
                a[3] + 4 * i,
                (a[1] & 0xff) as f32 * 0.125 + i as f32 - (a[0] & 3) as f32,
            );
        }
        0x237
    });
    rec(t, 233, 3, |m, a| {
        let s = f32::from_bits(a[2]);
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * s + i as f32;
            wf(m, a[0] + 4 * i, x);
        }
        0x233
    });
    for s in [243u32, 244] {
        rec(t, s, 2, move |m, a| {
            let ang = (a[1] as i32) as f32 / 65536.0;
            for i in 0..16 {
                wf(m, a[0] + 4 * i, ang * (i + 1) as f32 + s as f32 / 1024.0);
            }
            s
        });
    }
    rec(t, 231, 3, |m, a| {
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x231
    });
    rec(t, 230, 2, |m, a| {
        for i in 0..16 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x230
    });
    rec(t, 234, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x234
    });
    rec(t, 232, 2, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * 2.0;
            wf(m, a[0] + 4 * i, x);
        }
        0x232
    });
    // 229 also logs the vector's words (tiny junk would round away)
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let raw = m.r32(a[2] + 4 * i);
            m.w32(SCRATCH + 0x7e60 + 4 * i, raw);
            let x = rf(m, a[1] + 4 * i) + 2.0 * rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    rec(t, 241, 1, |m, a| {
        let k = m.r32(SCRATCH + 0x7f70).wrapping_add(1);
        m.w32(SCRATCH + 0x7f70, k);
        wf(m, a[0], (k % 5) as f32 * 0.375 - 0.5);
        0x241
    });
    // the waits: 225 and 227 answer the words at `SCRATCH + 0x7e30`/`+0x7e34`
    rec(t, 225, 1, |m, a| m.r32(SCRATCH + 0x7e30) ^ (a[0] & 0x100));
    rec(t, 227, 1, |m, _| m.r32(SCRATCH + 0x7e34));
}

/// `magic_0184` 0x10001990 (`r44::motion_0184`): the waits (slot 225 not
/// done, slot 227 below 1, each gate open and closed), the motions at 0x47
/// and 0x10e, nothing below 0x83, the centre (both parts' flags on and
/// off), the start at 0xdd, the way 0xde..0xf0 (the motion at 0xf0), the
/// fall 0xf1..0x104 (the damage at 0xfb; table words of both signs), the
/// end at 0x105 and past it.
#[test]
fn motion_0184() {
    let Some(mut t) = twin(184) else { return };
    all_stubs(&mut t);
    motion_stubs(&mut t);
    let f = t.at(0x1990);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    // the table `*G_T`: 4096 floats of both signs
    let table = SCRATCH + 0x2_0000;
    for i in 0..0x1000u32 {
        let x = ((i * 37) % 101) as f32 / 50.0 - 1.0;
        t.set_u32(table + 4 * i, x.to_bits());
    }
    let at = w(&t, 0x750);
    t.set_u32(at, table);
    // the scatter's records (first_free's range): two free on odd cases
    let ff = t.img.rel32_target(t.at(0x1280) + 0x22);
    let (rec_start, rec_end) = (t.img.word(ff + 0x3), t.img.word(ff + 0x12));
    // (gate +0x15, slot 225, gate +0x42, gate +0x6d, slot 227, F, flags)
    let cases: [(u32, u32, u32, u32, u32, u32, [u16; 2]); 22] = [
        (0, 1, 1, 1, 1, 0x90, [1, 1]),
        (0, 0, 0, 0, 0, 0x90, [1, 1]),
        (0, 0, 1, 0, 1, 0x90, [1, 1]),
        (1, 1, 1, 1, 0, 0x47, [1, 1]),
        (1, 1, 1, 1, 0, 0x10e, [1, 1]),
        (1, 1, 1, 1, 0, 0x50, [1, 1]),
        (1, 1, 1, 1, 0, 0x83, [1, 1]),
        (1, 1, 1, 1, 0, 0x84, [0, 1]),
        (1, 1, 1, 1, 0, 0x85, [1, 0]),
        (1, 1, 1, 1, 0, 0x86, [0, 0]),
        (1, 1, 1, 1, 0, 0xdd, [1, 1]),
        (1, 1, 1, 1, 0, 0xde, [1, 0]),
        (1, 1, 1, 1, 0, 0xe8, [0, 1]),
        (1, 1, 1, 1, 0, 0xf0, [1, 1]),
        (1, 1, 1, 1, 0, 0xf1, [1, 1]),
        (1, 1, 1, 1, 0, 0xf6, [1, 1]),
        (1, 1, 1, 1, 0, 0xfb, [0, 0]),
        (1, 1, 1, 1, 0, 0x100, [1, 1]),
        (1, 1, 1, 1, 0, 0x104, [1, 1]),
        (1, 1, 1, 1, 0, 0x105, [1, 1]),
        (1, 1, 1, 1, 0, 0x106, [1, 1]),
        (1, 1, 1, 1, 0, 0x130, [1, 1]),
    ];
    for (k, (g15, a225, g42, g6d, a227, fc, flags)) in (0u32..).zip(cases) {
        for (o, v) in [(0x15u32, g15), (0x42, g42), (0x6d, g6d), (0x9b, fc)] {
            let at = w(&t, o);
            t.set_u32(at, v);
        }
        t.set_u32(SCRATCH + 0x7e30, a225);
        t.set_u32(SCRATCH + 0x7e34, a227);
        for (i, o) in [0x231u32, 0x327].into_iter().enumerate() {
            let at = w(&t, o);
            t.set_u32(at, u32::from(flags[i] * (2 + k as u16)) | 0x5a5a_0000);
        }
        // the globals it reads and writes: floats (no NaNs), the angle
        for o in [
            0x172u32, 0x182, 0x19f, 0x1bc, 0x2ed, 0x2fd, 0x30d, 0x320, 0x371, 0x381, 0x391, 0x3a4,
            0x3f1, 0x3f9, 0x40d, 0x419, 0x429, 0x43a, 0x442, 0x460, 0x468, 0x478, 0x480, 0x488,
            0x4f0, 0x500, 0x51e, 0x52a, 0x5aa, 0x5ce, 0x5ee, 0x612, 0x671, 0x6e1, 0x71c, 0x728,
            0x730, 0x857, 0x85f, 0x86b, 0x877, 0x884, 0x899,
        ] {
            let at = w(&t, o);
            t.set_u32(
                at,
                (0.25 * (o % 13) as f32 + k as f32 * 0.5 - 1.0).to_bits(),
            );
        }
        let at = w(&t, 0x636);
        t.set_u16(at, 0x1234 + 0x1111 * k as u16);
        let at = w(&t, 0x94d);
        t.set_u16(at, 0x0ab0 + k as u16);
        let mut p = rec_start;
        let mut i = 0u32;
        while p < rec_end {
            let free = k % 2 == 1 && (i == 1 || i == 3);
            t.set_u16(p, if free { 0xffff } else { i as u16 });
            p += 0x18;
            i += 1;
        }
        ownb::dirty_stack(&mut t, 0x1990 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// What `r44::start_up_0072` (`magic_0072` 0x10001460) needs: the part
/// set-up's buffers, the object starts, the free ring of the inlined
/// takes and the sprite pool (`free` nodes), the targets (`n`, their
/// bytes and handles), the task list (empty or not), the camera flags;
/// slot 262 writes a point from its first word, 268 a vector from its
/// first two, 215/413 answer their word.
fn start_up_0072_setup(
    t: &mut Twin,
    f: u32,
    k: u32,
    (n, free, listed, cam): (u32, u16, bool, u32),
) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 2201);
    magic_r11::object_slots(t);
    rec(t, 262, 4, move |m, a| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(t, 268, 3, |m, a| {
        for i in 0..4u32 {
            let v = (a[0] & 0xff) as f32 * -0.25 + i as f32 * 0.75 + a[1] as f32;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x268
    });
    for s in [215u32, 413] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    rec(t, 711, 3, |_, a| 0x0711_0000 ^ a[0]);
    rec(t, 710, 4, |_, a| 0x0710_0000 ^ a[0]);
    for call in [0x76u32, 0x2c5] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(t, start);
    }
    // the free ring (the takes and the pool's)
    magic_r11::ring(t, w(t, 0x426), w(t, 0x4b8), free, SCRATCH + 0xd800);
    // the targets: bytes and handles
    let bytes = SCRATCH + 0x1e0;
    for i in 0..8u8 {
        t.set_u8(bytes + u32::from(i), 2 + 3 * i);
    }
    for o in [0x1b6u32, 0x2da, 0x33c] {
        let at = w(t, o);
        t.set_u32(at, bytes);
    }
    for o in [0x1c1u32, 0x2e5] {
        let tab = w(t, o);
        for i in 0..0x20 {
            t.set_u32(tab + 4 * i, 0x0000_0080 + 7 * i + k);
        }
    }
    let at = w(t, 0x17c);
    t.set_u32(at, n);
    let at = w(t, 0x81);
    t.set_u32(at, 0x0000_00a0 + k);
    // the u16 the start-up clears, its neighbour junk
    let at = w(t, 0xc2);
    t.set_u32(at, 0x5a5a_5a5a ^ k);
    // the objects' vectors and records
    let at = w(t, 0x298);
    t.write(at - 0xd4, &[0x5e; 0x130 * 4]);
    let at = w(t, 0x3ca);
    t.write(at, &[0x5f; 0x34 * 4]);
    // the camera flags
    for (o, v) in [(0x68bu32, cam & 1), (0x6e4, cam & 2)] {
        let at = w(t, o);
        t.set_u32(at, v);
    }
    for o in [0x713u32, 0x71c] {
        let at = w(t, o);
        t.set_u32(at, 0x5a5a_5a5a);
    }
    // the task list
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    for o in [0x513u32, 0x79e, 0x84f] {
        let at = w(t, o);
        t.set_u32(at, if listed { SCRATCH + 0x600 } else { 0 });
    }
    let at = w(t, 0x52e);
    t.set_u32(at, SCRATCH + 0x600);
}

/// `magic_0072` 0x10001460 (`r44::start_up_0072`, internal) and its load
/// step 0x100012b0 (`r44::load_0072`): targets 0..3 (the farthest
/// first, last, in the middle), the ring with 5 and 7 free nodes, the
/// task list empty and not, the camera flags both ways; the load step's
/// waits.
#[test]
fn start_up_0072() {
    let Some(mut t) = twin(72) else { return };
    all_stubs(&mut t);
    let f = t.at(0x1460);
    for (k, case) in (0u32..).zip([
        (3u32, 5u16, false, 0u32),
        (2, 7, true, 1),
        (0, 5, true, 2),
        (1, 6, false, 3),
    ]) {
        start_up_0072_setup(&mut t, f, k, case);
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r44::start_up_0072(g, img, fr)
        });
    }
    let load = t.at(0x12b0);
    for (busy, asked, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        start_up_0072_setup(&mut t, f, 1, (2, 6, true, 1));
        rec(&mut t, 225, 1, move |_, _| busy);
        rec(&mut t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
        rec(&mut t, 227, 1, move |_, _| ready);
        let op = |t: &Twin, o: u32| t.img.word(load + o);
        let at = op(&t, 0x18);
        t.set_u32(at, asked);
        for o in [0x26u32, 0x46] {
            let at = op(&t, o);
            t.set_u32(at, 0x0b0b_0000 + o);
        }
        for o in [0x61u32, 0x6b] {
            let at = op(&t, o);
            t.set_u32(at, 0x5a5a_0000 ^ o);
        }
        magic_h6::scrub(&mut t);
        t.check_entry(load, Call::default(), &[], Eax::Checked);
    }
}

/// What `r44::flames_0072` (`magic_0072` 0x10001e80) needs: its waits
/// and counts, the summon's handle, the count word, the handle words
/// `G_H`, the sprite pool (`n` records, `busy` of them taken), the float
/// slots (314, 404, 277 in ST0) and the matrix slots as data flow.
fn flames_0072_setup(
    t: &mut Twin,
    f: u32,
    k: u32,
    (wait, count, wait2): (i32, u32, i32),
    (n, busy): (u32, u32),
) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    for (o, v) in [
        (0x14u32, wait.cast_unsigned()),
        (0x31, count),
        (0x3a, wait2.cast_unsigned()),
    ] {
        let at = w(t, o);
        t.set_u32(at, v);
    }
    let at = w(t, 0x182);
    t.set_u32(at, 0x00a0_0000 + k);
    let at = w(t, 0x2bc);
    t.set_u32(at, 5 + 3 * k);
    // the handle words
    let h = SCRATCH + 0x5f0;
    for i in 0..3 {
        t.set_u32(h + 4 * i, 0x0070_0000 + 0x10 * i + k);
    }
    for o in [0x4abu32, 0x622] {
        let at = w(t, o);
        t.set_u32(at, h);
    }
    // the pool: `n` records of 0x6c bytes, `busy` taken
    let take = t.img.rel32_target(f + 0x312);
    let buf = SCRATCH + 0x1_8000;
    for i in 0..n.max(1) {
        t.write(buf + 0x6c * i, &[0x3c; 0x6c]);
        t.set_u32(buf + 0x6c * i + 0x68, u32::from(i < busy));
    }
    for (o, v) in [(0x02u32, k % 2), (0x0a, n), (0x17, buf)] {
        let at = t.img.word(take + o);
        t.set_u32(at, v);
    }
    for s in [404u32, 277] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    // 314 counts its calls: the answers cycle through -9, 0.5 and 9 (both
    // wraps of the two angles)
    t.set_u32(SCRATCH + 0x7e70, k);
    rec(t, 314, 1, |m, a| {
        let c = m.r32(SCRATCH + 0x7e70) + 1;
        m.w32(SCRATCH + 0x7e70, c);
        (314 << 16) ^ a[0]
    });
    t.stub_st0(314, |m, a| {
        let c = m.r32(SCRATCH + 0x7e70);
        [-9.0, 0.5, 9.0][(c % 3) as usize] + f64::from(f32::from_bits(a[0])) * 0.75
    });
    t.stub_st0(404, |_, a| f64::from(f32::from_bits(a[0])) * 1.5 - 0.25);
    t.stub_st0(277, |_, a| f64::from(a[0] & 0xf) * 0.5 + 1.0);
    rec(t, 236, 3, |m, a| {
        for i in 0..16 {
            wf(
                m,
                a[2] + 4 * i,
                (a[0] & 0xff) as f32 * 0.03125 + i as f32 * 0.5 - 2.0,
            );
        }
        0x236
    });
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    for s in [328u32, 329] {
        rec(t, s, 3, move |m, a| {
            let ang = f32::from_bits(a[2]);
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) + ang * (i + 1) as f32 / 8.0 + s as f32 / 1024.0;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 231, 3, |m, a| {
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x231
    });
    rec(t, 230, 2, |m, a| {
        for i in 0..16 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x230
    });
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let raw = m.r32(a[2] + 4 * i);
            m.w32(SCRATCH + 0x7e60 + 4 * i, raw);
            let x = rf(m, a[1] + 4 * i) + 2.0 * rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0] ^ (a[1] << 8));
}

/// `magic_0072` 0x10001e80 (`r44::flames_0072`, internal): the first wait
/// running, the second running, done and at 0, a burst with the pool
/// full, running out after the first, second and fourth record and with
/// a record left; then its frame step 0x10001d30 (`r44::frame_0072`):
/// the motion and the objects running or not, the flag, 0..2 sounds.
#[test]
fn flames_0072() {
    let Some(mut t) = twin(72) else { return };
    all_stubs(&mut t);
    let f = t.at(0x1e80);
    // ((wait, count, wait2), (records, busy))
    let cases: [((i32, u32, i32), (u32, u32)); 9] = [
        ((3, 1, 0), (8, 0)),
        ((0, 0, 4), (8, 0)),
        ((0, 0, 0), (8, 0)),
        ((-2, 0, -3), (8, 0)),
        ((0, 2, 0), (8, 0)),
        ((-1, 1, 0), (1, 0)),
        ((0, 1, 0), (3, 1)),
        ((0, 1, 0), (5, 1)),
        ((0, 3, 0), (6, 1)),
    ];
    for (k, (waits, pool)) in (0u32..).zip(cases) {
        flames_0072_setup(&mut t, f, k, waits, pool);
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r44::flames_0072(g, img, fr)
        });
    }
    let step = t.at(0x1d30);
    let ro = t.img.rel32_target(step + 0xe);
    let ms = t.img.rel32_target(step + 0x15);
    rec(&mut t, 772, 2, |m, a| {
        let k = m.r32(SCRATCH + 0x7f74).wrapping_add(1);
        m.w32(SCRATCH + 0x7f74, k);
        a[0] ^ (a[1] << 12) ^ k
    });
    t.stub_edx(772, |m, a| m.r32(SCRATCH + 0x7f74) ^ a[1]);
    // 779 logs the lock word it sees
    let lock = SCRATCH + 0x7e10;
    rec(&mut t, 779, 2, move |m, a| {
        let k = m.r32(SCRATCH + 0x7e00);
        m.w32(SCRATCH + 0x7e00, k + 1);
        m.w32(SCRATCH + 0x7e40 + 4 * (k % 8), m.r32(lock));
        0x0779_0000 ^ a[0] ^ a[1]
    });
    // (motion done, objects running, the flag, sounds)
    for (k, (done, running, flag, n)) in (0u32..).zip([
        (false, false, 0u32, 2u32),
        (true, true, 0, 2),
        (true, false, 1, 2),
        (true, false, 0, 0),
        (true, false, 0, 1),
        (true, false, 0, 2),
    ]) {
        flames_0072_setup(&mut t, f, k, (1 + k as i32, 1, 0), (8, 0));
        magic_hi::objects(&mut t, ro, running);
        magic_hi::motion(&mut t, ms, done);
        let w = |t: &Twin, o: u32| t.img.word(step + o);
        for (o, v) in [(0x2cu32, flag), (0x3b, n), (0x7a, lock)] {
            let at = w(&t, o);
            t.set_u32(at, v);
        }
        t.set_u32(lock, 0x40 + k);
        let task = SCRATCH + 0x7c00;
        let rec_at = SCRATCH + 0x7c40;
        t.write(task, &[0x31; 0x20]);
        t.write(rec_at, &[0x32; 0x20]);
        t.set_u32(task + 8, rec_at);
        magic_h6::scrub(&mut t);
        t.check_entry(step, Call::default(), &[task], Eax::Checked);
    }
}

/// What `r44::start_up_0009` (`magic_0009` 0x10001340) needs: the pools
/// and part set-ups, the part objects' slots, the target (its byte and
/// table) and the summon's handle, the task list (empty or not); slot 262
/// writes a point from its first word, 268 a vector (the case's offset
/// `d` apart for the two), 229/248 combine theirs, the matrix slots fill
/// theirs.
fn start_up_0009_setup(t: &mut Twin, f: u32, k: u32, (d, listed): ([f32; 3], bool)) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 1464);
    magic_r11::object_slots(t);
    rec(t, 262, 4, move |m, a| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    // 268: the target (handle 0x80..) at `d`, the summon at 0
    rec(t, 268, 3, move |m, a| {
        let far = a[0] & 0xf00 == 0x100;
        for i in 0..4u32 {
            let base = i as f32 * 0.75 + (a[0] & 0xf) as f32 * 0.125;
            let v = if far && i < 3 {
                base + d[i as usize]
            } else {
                base
            };
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x268
    });
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    for s in [328u32, 329] {
        rec(t, s, 3, move |m, a| {
            let ang = f32::from_bits(a[2]);
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) + ang * (i + 1) as f32 / 8.0 + s as f32 / 1024.0;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let raw = m.r32(a[2] + 4 * i);
            m.w32(SCRATCH + 0x7e60 + 4 * i, raw);
            let x = rf(m, a[1] + 4 * i) + 2.0 * rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x248
    });
    // the part objects (m0009's test): the pointer and owner globals, the
    // program table, five free pool nodes
    let po = t.img.rel32_target(f + 0x2f3);
    let (owner, table) = (SCRATCH + 0x300, SCRATCH + 0x400);
    let at = t.img.word(po + 0x04);
    t.set_u32(at, SCRATCH + 0x200);
    let at = t.img.word(po + 0x5e);
    t.set_u32(at, owner);
    t.set_u32(owner + 0x10, 0x00a1_0010);
    for i in 0..4 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    rec(t, 941, 1, move |_, _| table);
    let alloc = t.img.rel32_target(po + 0x9f);
    let (free, nodes) = (t.img.word(alloc + 0x04), t.img.word(alloc + 0x27) - 8);
    t.set_u16(free, 5);
    t.write(nodes, &[0x5d; 0x60]);
    for i in 0..8 {
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    let at = t.img.word(po + 0xaa);
    t.set_u32(at, if k % 2 == 1 { SCRATCH + 0x500 } else { 0 });
    t.write(SCRATCH + 0x500, &[0x55; 0x10]);
    // the target: its byte and the table; the summon's handle
    let sel = SCRATCH + 0x1e0;
    t.set_u8(sel, 3 + k as u8);
    t.set_u8(sel + 1, 0x77);
    for o in [0x69u32, 0x369] {
        let at = w(t, o);
        t.set_u32(at, sel);
    }
    let tab = w(t, 0x73);
    for i in 0..0x20 {
        t.set_u32(tab + 4 * i, 0x0000_0180 + i);
    }
    let at = w(t, 0xb9);
    t.set_u32(at, 0x0000_0040 + k);
    let at = w(t, 0x39d);
    t.set_u32(at, 0x5a5a_0000 + k);
    let at = w(t, 0x3d4);
    t.set_u32(at, 0x5a5a_5a5a);
    // the task list
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    let at = w(t, 0x50c);
    t.set_u32(at, if listed { SCRATCH + 0x600 } else { 0 });
    let at = w(t, 0x523);
    t.set_u32(at, SCRATCH + 0x600);
}

/// `magic_0009` 0x10001340 (`r44::start_up_0009`, internal) and its load
/// step 0x10001210 (`r44::load_0009`, the 119-byte form): the target in
/// four directions (each yaw wrap), above and below, the task list empty
/// and not; the load step's waits.
#[test]
fn start_up_0009() {
    let Some(mut t) = twin(9) else { return };
    all_stubs(&mut t);
    let f = t.at(0x1340);
    let cases: [([f32; 3], bool); 5] = [
        ([3.0, 1.0, 4.0], false),
        ([-3.0, -2.0, 0.5], true),
        ([-1.0, 5.0, -4.0], false),
        ([2.5, 0.0, -0.25], true),
        ([0.0, -1.0, 0.0], true),
    ];
    for (k, case) in (0u32..).zip(cases) {
        start_up_0009_setup(&mut t, f, k, case);
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r44::start_up_0009(g, img, fr)
        });
    }
    let load = t.at(0x1210);
    for (busy, asked, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        start_up_0009_setup(&mut t, f, 1, ([1.0, 2.0, 3.0], true));
        rec(&mut t, 225, 1, move |_, _| busy);
        rec(&mut t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
        rec(&mut t, 227, 1, move |_, _| ready);
        let op = |t: &Twin, o: u32| t.img.word(load + o);
        let at = op(&t, 0x18);
        t.set_u32(at, asked);
        for o in [0x27u32, 0x47] {
            let at = op(&t, o);
            t.set_u32(at, 0x0b0b_0000 + o);
        }
        for o in [0x62u32, 0x6c] {
            let at = op(&t, o);
            t.set_u32(at, 0x5a5a_0000 ^ o);
        }
        magic_h6::scrub(&mut t);
        t.check_entry(load, Call::default(), &[], Eax::Checked);
    }
}

/// What `r44::trail_task_0009` (`magic_0009` 0x10001c70) needs: the
/// effect block `R` (matrix, ends, offset, four records with their trails
/// of `n` points, animation and spin; the count), the frame table of
/// slot 941 (as the trail draw's test has it), the lock globals of the
/// part link and the trail draw, the object (waiting, flag bit 15 per
/// case); the vector and matrix slots as data flow.
fn trail_task_0009_setup(
    t: &mut Twin,
    f: u32,
    k: u32,
    (count, n, anim): (u32, [u32; 4], [(u32, u16, u16); 4]),
) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let r = SCRATCH + 0x2_0000;
    let at = w(t, 0x20);
    t.set_u32(at, r);
    for i in 0..0x280u32 {
        let x = ((i * 29 + k * 7) % 97) as f32 * 0.0625 - 3.0;
        t.set_u32(r + 4 * i, x.to_bits());
    }
    t.set_u32(r + 0x9b8, count);
    // the ends: the second in one of six directions from the first
    let dirs = [
        (1.0f32, 0.0f32),
        (-1.0, 0.1),
        (-1.0, -0.1),
        (0.0, 1.0),
        (0.0, -1.0),
        (0.7, -0.7),
    ];
    let (dx, dz) = dirs[k as usize % 6];
    for (o, x) in [(0x40u32, 0.0f32), (0x44, 0.5), (0x48, 0.0), (0x4c, 1.0)] {
        t.set_u32(r + o, x.to_bits());
    }
    for (o, x) in [
        (0x50u32, 4.0 * dx),
        (0x54, 0.5),
        (0x58, 4.0 * dz),
        (0x5c, 1.0),
    ] {
        t.set_u32(r + o, x.to_bits());
    }
    // the screen pointer global, junk before each case
    let at = w(t, 0x3fb);
    t.set_u32(at, 0x5a5a_5a5a ^ k);
    for i in 0..4u32 {
        let rec = r + 0x70 + 0x250 * i;
        // the spins: both wraps
        let spin = [3.13f32, -3.6, 0.5, -0.5][((i + k) % 4) as usize];
        t.set_u32(rec + 0x24c, spin.to_bits());
        let (on, frame, timer) = anim[i as usize];
        t.set_u32(rec, 0x0a00 + i);
        t.set_u32(rec + 4, on);
        t.set_u16(rec + 8, frame);
        t.set_u16(rec + 0xa, timer);
        t.set_u32(rec + 0x1c, 0x0c0c_0000 + i);
        // the trail: colours, sizes, spacing, newest, the points
        t.write(rec + 0x24 + 0x20c, &[10, 200, 0, 0x80, 250, 3, 128, 0x40]);
        t.set_u32(rec + 0x24 + 0x214, 1.5f32.to_bits());
        t.set_u32(rec + 0x24 + 0x218, 0.25f32.to_bits());
        t.set_u32(rec + 0x24 + 0x21c, 0.75f32.to_bits());
        t.set_u32(rec + 0x24 + 0x220, 2 + i);
        t.set_u32(rec + 0x248, n[i as usize]);
    }
    // slot 941's table (frames, the entries the link and the draw read)
    let table = SCRATCH + 0x8000;
    rec(t, 941, 1, move |_, _| table);
    t.write(table, &[0; 0x100]);
    t.set_u16(table + 6, 3);
    for j in 0..3u32 {
        let e = table + 0x10 + 8 * j;
        t.set_u16(e, (0x80 + 0x10 * j) as u16);
        t.set_u16(e + 2, 0x300 + 0x40 * j as u16);
        t.set_u8(e + 4, if j == 2 && k % 2 == 1 { 0x80 } else { 0 });
        t.set_u8(e + 5, 0x80);
        let rr = table + 0x80 + 0x10 * j;
        t.set_u32(rr + 4, 4 * j);
        t.set_u8(rr + 4 * j + 4, 0x31 + j as u8);
    }
    // the lock globals (the part link's and the trail draw's)
    let lock = SCRATCH + 0x9000;
    let link = t.img.rel32_target(f + 0x700);
    for o in [0x4fu32, 0x7a, 0xa9, 0xc6, 0x115, 0x13b] {
        let at = t.img.word(link + o);
        t.set_u32(at, lock);
    }
    let draw = t.img.rel32_target(f + 0x713);
    let at = t.img.word(draw + 0x463);
    t.set_u32(at, lock);
    let at = t.img.word(draw + 0x520);
    t.set_u32(at, 0x0cc0_0cc0);
    t.set_u32(lock, 0x40 + k);
    // the object: waiting, its list empty, flag bit 15 by the case
    let obj = w(t, 0x75f);
    // (its block ends where the effect block's pointer `+0x34` sits)
    t.write(obj, &[0; 0x2c]);
    t.set_u32(obj + 0x2c, 5);
    t.set_u16(obj + 0x30, if k % 2 == 1 { 0x8001 } else { 0x0001 });
    // the task node for the give-back
    let give = t.img.rel32_target(f + 0x7dd);
    magic_hi::lone_node(t, give, SCRATCH + 0x7c00);
}

/// The stubs `r44::trail_task_0009` reaches, as data flow and logged.
fn trail_task_0009_stubs(t: &mut Twin) {
    all_stubs(t);
    let lock = SCRATCH + 0x9000;
    rec(t, 326, 4, |m, a| {
        let u = f32::from_bits(a[3]);
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) + (rf(m, a[2] + 4 * i) - rf(m, a[1] + 4 * i)) * u;
            wf(m, a[0] + 4 * i, x);
        }
        0x326
    });
    for s in [723u32, 724, 235] {
        rec(t, s, 1, move |m, a| {
            for i in 0..16 {
                let v = (i as f32 + 1.0) * if s == 235 { 1.0 } else { 0.5 };
                m.w32(a[0] + 4 * i, v.to_bits());
            }
            s
        });
    }
    for s in [330u32, 328, 329] {
        rec(t, s, 3, move |m, a| {
            let ang = f32::from_bits(a[2]);
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) + ang * (i + 1) as f32 / 8.0 + s as f32 / 1024.0;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    // 231 folds what it writes into a log word (the sprite matrix is only
    // seen here)
    rec(t, 231, 3, |m, a| {
        let mut h = m.r32(SCRATCH + 0x7e80);
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
            h = h.rotate_left(5) ^ x.to_bits();
        }
        m.w32(SCRATCH + 0x7e80, h);
        0x231
    });
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let raw = m.r32(a[2] + 4 * i);
            m.w32(SCRATCH + 0x7e60 + 4 * i, raw);
            let x = rf(m, a[1] + 4 * i) + 2.0 * rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    for s in [234u32, 248] {
        rec(t, s, 3, move |m, a| {
            for i in 0..3 {
                let v = if s == 234 {
                    rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i)
                } else {
                    rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i)
                };
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 772, 2, |_, a| 0x7720_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    rec(t, 773, 4, move |m, a| {
        m.w32(SCRATCH + 0x9200 + 4 * (a[3] & 7), m.r32(lock));
        (a[3] >> 1) & 1
    });
    for s in [774u32, 775, 778] {
        rec(t, s, if s == 775 { 2 } else { 5 }, move |m, _| {
            m.w32(SCRATCH + 0x9300 + 4 * (s - 774), m.r32(lock));
            s
        });
    }
    rec(t, 776, 1, |_, a| a[0] << 4);
    rec(t, 777, 1, |_, a| 0x0ff0 ^ a[0]);
}

/// `magic_0009` 0x10001c70 (`r44::trail_task_0009`, a task callback): the
/// count running (the records' frames: timers above and at 0, the last
/// frame looping and holding, records off) and ended (the trails halved;
/// the give-back).
#[test]
fn trail_task_0009() {
    let Some(mut t) = twin(9) else { return };
    trail_task_0009_stubs(&mut t);
    let f = t.at(0x1c70);
    let task = SCRATCH + 0x7c00;
    // (count, trail points, (on, frame, timer) per record)
    let cases: [(u32, [u32; 4], [(u32, u16, u16); 4]); 6] = [
        (
            5,
            [2, 0, 3, 1],
            [(1, 0, 0x300), (1, 1, 0x100), (0, 0, 0x100), (1, 2, 0x200)],
        ),
        (
            3,
            [1, 2, 0, 0],
            [(1, 2, 0x150), (1, 2, 0x150), (1, 0, 0x50), (0, 1, 0)],
        ),
        (0, [3, 2, 1, 4], [(1, 0, 0x300); 4]),
        (0, [0, 2, 1, 4], [(1, 0, 0x300); 4]),
        (
            9,
            [1, 1, 1, 1],
            [(1, 2, 0x10), (1, 1, 0x210), (1, 0, 0x200), (1, 2, 0x200)],
        ),
        (2, [0, 0, 0, 0], [(0, 1, 0x10); 4]),
    ];
    for (k, case) in (0u32..).zip(cases) {
        trail_task_0009_setup(&mut t, f, k, case);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// One case of `stage_task_0150`: the state, the timer before its step,
/// the counter `C`, `*G_Q`, the summon's kind, slot 535's / 520's / 294's
/// answers, the flag words, the move armed.
#[derive(Clone, Copy)]
struct Stage {
    state: u32,
    timer: f32,
    count: u32,
    q: u32,
    kind: u32,
    r535: u32,
    r520: u32,
    r294: u32,
    flags: [u16; 3],
    armed: u32,
}

const STAGE: Stage = Stage {
    state: 0,
    timer: 0.5,
    count: 3,
    q: 0,
    kind: 0x1000,
    r535: 0,
    r520: 0,
    r294: 5,
    flags: [0, 0, 0],
    armed: 0,
};

/// `magic_0150` 0x1000bac0 (`r44::stage_task_0150`, a task callback):
/// every state of the byte table and past it, each way through its
/// branches (the counter, `*G_Q`, the camera modes, slot 294's 0xff, the
/// flags, the summon's kinds and slot 535's answers, the timer below,
/// between and past the ending's bounds, the move armed or not).
#[test]
fn stage_task_0150() {
    let Some(mut t) = twin(150) else { return };
    all_stubs(&mut t);
    let f = t.at(0xbac0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    // the vector slots as data flow
    rec(&mut t, 232, 2, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * 0.5 + 0.25;
            wf(m, a[0] + 4 * i, x);
        }
        0x232
    });
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x325
    });
    rec(&mut t, 233, 3, |m, a| {
        let s = f32::from_bits(a[2]);
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * s + i as f32;
            wf(m, a[0] + 4 * i, x);
        }
        0x233
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) + 2.0 * rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    rec(&mut t, 248, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x248
    });
    rec(&mut t, 531, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, (a[0] & 0xf) as f32 + i as f32 * 0.5);
        }
        0x5310_0531
    });
    rec(&mut t, 255, 2, |m, a| {
        let mut h = 0x255u32;
        for i in 0..4 {
            h = h.rotate_left(7) ^ m.r32(a[1] + 4 * i);
        }
        m.w32(SCRATCH + 0x7e90, h);
        h
    });
    rec(&mut t, 289, 1, |_, a| 0x0289_0000 ^ a[0]);
    t.stub_st0(289, |_, a| 0.75 + f64::from(a[0] & 7));
    rec(&mut t, 307, 0, |_, _| 0x0307);
    t.stub_st0(307, |m, _| {
        f64::from(m.r32(SCRATCH + 0x7e3c) & 0xff) * 0.01 - 0.25
    });
    rec(&mut t, 520, 0, |m, _| m.r32(SCRATCH + 0x7e30));
    rec(&mut t, 294, 1, |m, a| {
        m.r32(SCRATCH + 0x7e34) ^ (a[0] & 0x100)
    });
    rec(&mut t, 535, 1, |m, _| m.r32(SCRATCH + 0x7e38));
    // the camera tree's slots
    rec(&mut t, 287, 3, |_, a| 0x0287_0000 ^ a[2]);
    let cases = [
        Stage {
            state: 0x70,
            ..STAGE
        },
        Stage { state: 7, ..STAGE },
        Stage {
            state: 5,
            count: 3,
            ..STAGE
        },
        Stage {
            state: 5,
            count: 12,
            q: 2,
            r520: 0x24a_0000,
            ..STAGE
        },
        Stage {
            state: 5,
            count: 13,
            q: 0,
            r520: 0x247_0000,
            ..STAGE
        },
        Stage {
            state: 5,
            count: 14,
            q: 2,
            r520: 7,
            ..STAGE
        },
        Stage {
            state: 5,
            count: 20,
            q: 1,
            kind: 0x1088,
            timer: 9.0,
            armed: 1,
            ..STAGE
        },
        Stage { state: 0, ..STAGE },
        Stage {
            state: 1,
            r294: 0xff,
            r520: 0xd4_0000,
            ..STAGE
        },
        Stage {
            state: 1,
            r294: 5,
            flags: [0, 0, 1],
            ..STAGE
        },
        Stage {
            state: 2,
            timer: 0.5,
            ..STAGE
        },
        Stage {
            state: 2,
            timer: 1.5,
            ..STAGE
        },
        Stage {
            state: 3,
            timer: 5.0,
            ..STAGE
        },
        Stage {
            state: 3,
            timer: 1.0,
            flags: [1, 0, 0],
            ..STAGE
        },
        Stage {
            state: 3,
            timer: 1.0,
            flags: [0, 1, 0],
            ..STAGE
        },
        Stage {
            state: 4,
            timer: 5.0,
            ..STAGE
        },
        Stage {
            state: 4,
            timer: 1.0,
            ..STAGE
        },
        Stage {
            state: 6,
            timer: 9.0,
            armed: 1,
            ..STAGE
        },
        Stage {
            state: 6,
            timer: 6.8,
            armed: 0,
            ..STAGE
        },
        Stage {
            state: 6,
            timer: 7.2,
            armed: 1,
            ..STAGE
        },
        Stage {
            state: 6,
            timer: 1.0,
            ..STAGE
        },
        Stage {
            state: 100,
            kind: 0x1089,
            timer: 6.8,
            ..STAGE
        },
        Stage {
            state: 100,
            kind: 0x108c,
            timer: 1.0,
            ..STAGE
        },
        Stage {
            state: 100,
            kind: 0x1000,
            r535: 0,
            timer: 7.2,
            ..STAGE
        },
        Stage {
            state: 100,
            kind: 0x1000,
            r535: 2,
            ..STAGE
        },
        Stage {
            state: 100,
            kind: 0x1000,
            r535: 5,
            timer: 9.0,
            armed: 1,
            ..STAGE
        },
        Stage {
            state: 101,
            timer: 6.8,
            armed: 1,
            ..STAGE
        },
        Stage {
            state: 101,
            timer: 1.0,
            ..STAGE
        },
    ];
    // the DLL's globals (its block at the preferred base)
    let block = w(&t, 0x18) & !0xfff;
    let task = SCRATCH + 0x7c00;
    let node = SCRATCH + 0x7c40;
    for (k, c) in (0u32..).zip(cases) {
        // the DLL's block: floats, then the words the steps read
        for i in 0..0x400u32 {
            // (odd low mantissa bits: byte and bit stores show)
            let x = ((i * 37 + k * 11) % 89) as f32 * 0.0625 - 2.5 + (i % 7 + 1) as f32 * 1.0e-4;
            t.set_u32(block + 4 * i, x.to_bits());
        }
        let set = |t: &mut Twin, o: u32, v: u32| {
            let at = t.img.word(f + o);
            t.set_u32(at, v);
        };
        set(&mut t, 0x18, (c.timer - 1.0 / 30.0).to_bits());
        set(&mut t, 0x29, c.state);
        set(&mut t, 0x50, 0x0400 + k);
        set(&mut t, 0x56, 0x1c78 - k);
        set(&mut t, 0x5c, c.count);
        set(&mut t, 0xae, c.q);
        set(&mut t, 0x3c0, 2.0f32.to_bits());
        set(&mut t, 0xaaf, c.kind);
        set(&mut t, 0x8d6, c.armed);
        set(&mut t, 0xe5e, c.armed);
        for (i, o) in [0x5dcu32, 0x740].into_iter().enumerate() {
            let at = w(&t, o);
            t.set_u16(at, c.flags[i] | 0x5a00);
        }
        let at = w(&t, 0x526);
        t.set_u8(at, (c.flags[2] as u8) | 0x40);
        let at = w(&t, 0xaf9);
        t.set_u8(at, 0x30 + k as u8);
        // the lists: the camera tree's parent and the node's list empty
        let list = w(&t, 0xdc);
        t.set_u32(list + 0x10, list);
        t.set_u32(list + 0xc, list);
        t.set_u16(list + 0x18, 1);
        // the leaving node without a parent
        // the leaving node: without a parent, or the list's only child
        let leaving = w(&t, 0xfda);
        if k % 2 == 1 {
            t.set_u32(leaving + 0x14, list);
            t.set_u32(leaving + 4, list);
            t.set_u32(leaving + 8, list);
            t.set_u32(list + 0x10, leaving);
            t.set_u32(list + 0xc, leaving);
        } else {
            t.set_u32(leaving + 0x14, 0);
        }
        t.set_u32(SCRATCH + 0x7e30, c.r520);
        t.set_u32(SCRATCH + 0x7e34, c.r294);
        t.set_u32(SCRATCH + 0x7e38, c.r535);
        // the task node in a ring of its own neighbours
        let (nx, pv) = (SCRATCH + 0x7c80, SCRATCH + 0x7cc0);
        t.write(node, &[0x44; 0x20]);
        t.set_u32(task, node);
        t.set_u32(node + 4, nx);
        t.set_u32(node + 8, pv);
        t.write(nx, &[0x45; 0x10]);
        t.write(pv, &[0x46; 0x10]);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// The call log of [`node_sparks_0071`]'s test: a count at `SPLOG`, then
/// rows of 0x40 bytes.
const SPLOG: u32 = SCRATCH + 0x6000;

/// Logs each call of `slot` into the shared log (in call order): the
/// slot, its first `n` words, and 4 words at each argument `ptrs` names.
/// Answers `ret(m, args, k)`, `k` the call's number.
fn sp_log(
    t: &mut Twin,
    slot: u32,
    n: usize,
    ptrs: &'static [usize],
    ret: impl Fn(&mut dyn StubMem, &[u32], u32) -> u32 + 'static,
) {
    t.stub(slot, move |m, a| {
        let k = m.r32(SPLOG);
        m.w32(SPLOG, k + 1);
        let row = SPLOG + 4 + 0x40 * (k % 128);
        m.w32(row, slot);
        for (i, w) in (1u32..).zip(&a[..n.min(a.len())]) {
            m.w32(row + 4 * i, *w);
        }
        for (j, &p) in (0u32..).zip(ptrs) {
            for i in 0..4 {
                m.w32(row + 0x20 + 0x10 * j + 4 * i, m.r32(a[p] + 4 * i));
            }
        }
        ret(m, a, k)
    });
}

/// `magic_0071` 0x10001430 (`r44::node_sparks_0071`) directly and through
/// its thunk 0x10001b40 (`r44::jmp_node_sparks_0071`), as the
/// `node_sparks` test runs `magic_0070`'s: the ring's cases, the timer
/// against `K0` (the wait running, at 0 the spawn), sparks counted or not
/// (the release), records live, dead, faded and near the cap.
#[test]
fn node_sparks_0071() {
    use sa::{RINGS, check_this_thunk, handler_stubs, node, ring};
    let Some(mut t) = sa::twin(71) else { return };
    handler_stubs(&mut t);
    let f = t.at(0x1430);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    // the sine table (two odd entries)
    let sines = SCRATCH + 0x1_0000;
    let at = w(&t, 0x38f);
    t.set_u32(at, sines);
    let b: Vec<u8> = (0..4096u32)
        .flat_map(|i| {
            let x = match i {
                7 => -0.0,
                9 => 1.0e30,
                _ => (f64::from(i) * std::f64::consts::TAU / 4096.0).sin() as f32,
            };
            x.to_bits().to_le_bytes()
        })
        .collect();
    t.write(sines, &b);
    let lock = SCRATCH + 0x7f0;
    let step = t.img.rel32_target(f + 0x535);
    let at = t.img.word(step + 0x24);
    t.set_u32(at, lock);
    let release = t.img.rel32_target(f + 0x619);
    for o in [0x6bu32, 0x90] {
        let at = t.img.word(release + o);
        t.set_u32(at, lock);
    }
    let loaded = SCRATCH + 0x7e0;
    sp_log(&mut t, 324, 0, &[], |_, _, k| {
        0x1235u32.wrapping_mul(k + 3) ^ 0x51
    });
    sp_log(&mut t, 307, 0, &[], |_, _, k| k);
    t.stub_st0(307, |m, _| f64::from(m.r32(SPLOG) % 7) * 0.375 - 0.75);
    sp_log(&mut t, 325, 2, &[1], |_, a, k| a[0] ^ k);
    sp_log(&mut t, 229, 3, &[], |m, a, k| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, (k as f32 * 0.25 + i as f32).to_bits());
        }
        0x229
    });
    sp_log(&mut t, 230, 2, &[], |m, a, k| {
        let mut x = k;
        for i in 0..16 {
            x = x.rotate_left(3) ^ m.r32(a[1] + 4 * i);
        }
        m.w32(SCRATCH + 0x7e90 + 4 * (k % 8), x);
        x
    });
    sp_log(&mut t, 773, 2, &[], move |m, _, k| m.r32(loaded) ^ (k & 1));
    for (s, n, ptrs) in [
        (774u32, 5usize, &[3usize][..]),
        (775, 2, &[]),
        (776, 1, &[]),
        (777, 1, &[]),
        (778, 6, &[3]),
        (779, 2, &[]),
    ] {
        sp_log(&mut t, s, n, ptrs, move |m, a, k| {
            (s << 16) ^ a[0] ^ k ^ m.r32(lock)
        });
    }
    let k0 = f32::from_bits(t.u32(w(&t, 0x76)));
    let k7 = f32::from_bits(t.u32(w(&t, 0x4f2)));
    let recs = SCRATCH + 0x9000;
    let q = SCRATCH + 0x8800;
    // (the timer against K0, the wait, sparks counted, the ring's head)
    let cases = [
        (-1.0f32, 0u32, 0u32, 0x18u16),
        (-1.0, 3, 0, 0x18),
        (0.5, 0, 3, 0),
        (0.5, 0, 0, 0),
        (-1.0, 0, 2, 2),
        (0.0, 1, 1, 0),
    ];
    for (k, &(c, p, cp)) in RINGS.iter().enumerate() {
        for (case, &(dt, wait, counted, head)) in cases.iter().enumerate() {
            let case = case as u32;
            ring(&mut t, c, p, cp, 0x30 + k as u8);
            let n = node(0);
            for j in 0..0x10u32 {
                let v = (j as f32 - 3.5) * 0.25 + case as f32;
                t.set_u32(n + 0x38 + 4 * j, v.to_bits());
            }
            // a ring that holds every request (up to 0x2f sparks)
            t.set_u16(n + 0x1c, 0x40);
            t.set_u16(n + 0x1e, 0x40);
            t.set_u32(n + 0x20, recs);
            t.set_u32(n + 0x24, recs + 0x40 * 3);
            t.set_u32(n + 0x28, recs);
            t.set_u16(n + 0x2c, 4);
            t.set_u16(n + 0x2e, 2);
            t.set_u16(n + 0x30, head);
            t.set_u16(n + 0x32, 0x1e);
            t.set_u16(n + 0x34, 1);
            t.set_u32(n + 0x48, q);
            t.set_u32(n + 0x4c, q + 0x100);
            t.set_u32(n + 0x50, (k0 + dt).to_bits());
            t.set_u32(n + 0x54, counted);
            t.set_u32(n + 0x58, wait);
            // the sequences (both kinds)
            for (qq, len) in [(q, 0x100i16), (q + 0x100, 0x180)] {
                let v: Vec<u8> = (0..0x100u32)
                    .map(|i| (i * 7 + (case + 1) * 13 + 1) as u8)
                    .collect();
                t.write(qq, &v);
                t.set_u16(qq + 6, 3);
                for i in 0..4u32 {
                    let r = qq + 0x10 + 8 * i;
                    t.set_u16(r, (0x80 + 0x20 * i) as u16);
                    t.set_u16(r + 2, len.cast_unsigned());
                    t.set_u32(qq + 0x80 + 0x20 * i + 4, 0x10);
                }
            }
            // the records: live or not, the fade below and near its cap
            for i in 0..0x40u32 {
                let r = recs + 0x40 * i;
                for j in 0..0x10u32 {
                    let x = ((j * 37 + (i + case) * 11) % 41) as f32 - 20.0;
                    t.set_u32(r + 4 * j, (x * 0.125).to_bits());
                }
                t.set_u8(r + 0x34, u8::from(i % 5 != 1));
                t.set_u8(r + 0x2b, if i % 7 == 3 { 0 } else { 0x40 });
                t.set_u32(r + 0x24, i % 2);
                t.set_u32(r + 0x30, 0x1234 * i);
                t.set_u32(r + 0x2c, [0.5f32, 3.9, 4.5, k7][(i % 4) as usize].to_bits());
                t.set_u16(r + 0x10, (i % 4) as u16);
                t.set_u16(r + 0x12, 0x80);
            }
            // the render state's word `*G_Z`
            let at = w(&t, 0x2b8);
            t.set_u32(at, 0x5a5a_0071 ^ case);
            t.set_u32(loaded, case % 2);
            t.set_u32(lock, 0x5a5a_0000 + case);
            t.set_u32(SPLOG, 0);
            check_this_thunk(
                &mut t,
                f,
                0x1b40,
                ffx_magic::r44::node_sparks_0071,
                Eax::Checked,
            );
        }
    }
}

/// `magic_0042` 0x1000a610 (`r44::scene_0042`, a table entry): the scene
/// built three times, the start-up's ring and roots as its own test
/// varies them, the handles and the lock word per case; the slots as
/// data flow and logged.
#[test]
fn scene_0042() {
    let Some(mut t) = twin(42) else { return };
    all_stubs(&mut t);
    let f = t.at(0xa610);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let su = t.img.rel32_target(f + 0xa3d);
    let sw = |t: &Twin, o: u32| t.img.word(su + o);
    // the start-up's slots (its test's answers)
    t.stub(557, |_, _| 0x0000_00ff);
    t.stub_st0(790, |_, _| 0.25);
    t.stub(558, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 3 + i);
        }
        0x558
    });
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x325
    });
    // slot 711 logs the lock word it sees
    let lock = SCRATCH + 0x7e10;
    rec(&mut t, 711, 3, move |m, a| {
        m.w32(SCRATCH + 0x7e14, m.r32(lock));
        0x0711_0000 ^ a[0]
    });
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0] ^ (a[1] << 8));
    // the model state's float slots (15, 16, 17, 277) and 290's vector
    for (s, x) in [(15u32, 1.5f64), (16, -0.75), (17, 3.0), (277, 40.0)] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    rec(&mut t, 290, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, 1.5 - i as f32);
        }
        0x290
    });
    // the time left's float slots
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xf) * 0.25 + 0.5);
    rec(&mut t, 601, 2, |m, a| {
        wf(m, a[1], 0.25 * (a[0] & 0xf) as f32 - 1.0);
        0x601
    });
    let tail = SCRATCH + 0x500;
    let other = SCRATCH + 0x600;
    let link = SCRATCH + 0x700;
    let sl = t.img.rel32_target(f + 0xa42);
    for (k, (root, second)) in (0u32..).zip([(false, false), (true, true), (false, true)]) {
        let at = w(&t, 0x63);
        t.set_u32(at, lock);
        t.set_u32(lock, 0x40 + k);
        for o in [0x1c5u32, 0x2f2, 0x38e, 0x5d6] {
            let at = w(&t, o);
            t.set_u32(at, 0x00a0_0000 + o + k);
        }
        for o in [0x269u32, 0x271, 0x8d1, 0x8d9] {
            let at = w(&t, o);
            t.set_u32(
                at,
                (0.75 + 0.5 * k as f32 + (o & 0xf) as f32 * 0.125).to_bits(),
            );
        }
        // the start-up's ring and roots
        t.write(tail, &[0x55; 0x10]);
        t.write(other, &[0x66; 0x20]);
        let at = sw(&t, 0x04);
        t.set_u32(at, tail);
        let r = sw(&t, 0x96);
        let at = sw(&t, 0x3d);
        t.set_u32(at, if root { other } else { r });
        let r2 = sw(&t, 0x338);
        let at = sw(&t, 0x2e9);
        t.set_u32(at, if second { other + 0x10 } else { r2 });
        let at = sw(&t, 0xb1);
        t.set_u16(at, 7 + k as u16);
        // the words the time is copied into
        for o in [0x6abu32, 0x6b5, 0x6bf] {
            let at = w(&t, o);
            t.set_u32(at, 0x5a5a_0000 ^ o ^ k);
        }
        // the static link's last node
        t.write(link, &[0x77; 0x20]);
        let at = t.img.word(sl + 0x1);
        t.set_u32(at, link);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[SCRATCH + 0x7c00 + k], Eax::Checked);
    }
}

/// `magic_0042` 0x100085b0 (`r44::camera_task_0042`, a task callback):
/// every state of the byte table (each wait below and at its bound, state
/// 92's fade on both sides, state 100's shake with the byte above and at
/// 2, state 101's count at and below 128), a gap of the table, 10000,
/// above it and a negative state.
#[test]
fn camera_task_0042() {
    let Some(mut t) = twin(42) else { return };
    all_stubs(&mut t);
    let f = t.at(0x85b0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    // the slots as data flow
    for s in [285u32, 286] {
        rec(&mut t, s, 2, move |_, a| (s << 16) ^ a[0] ^ (a[1] >> 3));
    }
    rec(&mut t, 307, 0, |m, _| m.r32(SCRATCH + 0x7e40));
    t.stub_st0(307, |m, _| {
        f64::from(m.r32(SCRATCH + 0x7e40) & 0xff) * 0.01 + 0.25
    });
    rec(&mut t, 250, 2, |_, a| 0x0250_0000 ^ a[0] ^ (a[1] >> 7));
    rec(&mut t, 557, 0, |_, _| 0x0557_0003);
    rec(&mut t, 560, 1, |_, a| 0x0560_0000 ^ a[0]);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0] ^ (a[1] << 8));
    rec(&mut t, 306, 3, |_, a| {
        0x0306_0000 ^ a[0] ^ (a[1] << 4) ^ a[2]
    });
    let task = SCRATCH + 0x100;
    let node = SCRATCH + 0x180;
    let r = SCRATCH + 0x1_0000;
    let st_at = w(&t, 0x23);
    // (state, R + 0x20 before the step, R + 0x1c, byte R + 0x390, R + 0x310)
    let cases: [(u32, u32, f32, u8, u32); 30] = [
        (0, 3, 0.0, 0, 0),
        (1, 3, 0.0, 0, 0),
        (1, 0x27, 0.0, 0, 0),
        (81, 3, 0.0, 0, 0),
        (2, 3, 0.0, 0, 0),
        (90, 3, 0.0, 0, 0),
        (91, 3, 0.0, 0, 0),
        (92, 3, -5.0, 0, 0),
        (92, 3, 20.0, 0, 0),
        (92, 3, 200.0, 0, 0),
        (10, 3, 0.0, 0, 0),
        (11, 3, 0.0, 0, 0),
        (100, 3, 0.0, 9, 4),
        (100, 3, 0.0, 2, 4),
        (100, 3, 0.0, 1, 0x7f),
        (101, 3, 0.0, 0, 0x7e),
        (101, 3, 0.0, 0, 0x7f),
        (20, 3, 0.0, 0, 0),
        (21, 3, 0.0, 0, 0),
        (60, 3, 0.0, 0, 0),
        (61, 3, 0.0, 0, 0),
        (62, 3, 0.0, 0, 0),
        (70, 3, 0.0, 0, 0),
        (80, 3, 0.0, 0, 0),
        (110, 3, 0.0, 0, 0),
        (111, 3, 0.0, 0, 0),
        (111, 0xe, 0.0, 0, 0),
        (7, 3, 0.0, 0, 0),
        (10000, 3, 0.0, 0, 0),
        (0x2711, 3, 0.0, 0, 0),
    ];
    let more: [(u32, u32, f32, u8, u32); 2] = [(0x70, 3, 0.0, 0, 0), (0xffff_fff0, 3, 0.0, 0, 0)];
    for (k, (st, cnt, x1c, b, c310)) in (0u32..).zip(cases.into_iter().chain(more)) {
        // the record: floats, then the words the case sets
        for i in 0..0x100u32 {
            let x = ((i * 29 + k * 7) % 97) as f32 * 0.0625 - 3.0 + (i % 5 + 1) as f32 * 1.0e-4;
            t.set_u32(r + 4 * i, x.to_bits());
        }
        t.set_u32(r + 0x20, cnt);
        t.set_u32(r + 0x1c, x1c.to_bits());
        t.set_u8(r + 0x390, b);
        t.set_u8(r + 0x391, 0x5a);
        t.set_u32(r + 0x310, c310);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        let (nx, pv) = (SCRATCH + 0x1c0, SCRATCH + 0x1e0);
        t.write(node, &[0x44; 0x20]);
        t.set_u32(node + 4, nx);
        t.set_u32(node + 8, pv);
        t.write(nx, &[0x45; 0x10]);
        t.write(pv, &[0x46; 0x10]);
        t.set_u32(st_at, st);
        t.set_u32(SCRATCH + 0x7e40, 0x31 + 7 * k);
        // the lists the nodes join: empty, the leaving node without a parent
        for o in [0x5dbu32, 0x691] {
            let list = w(&t, o);
            t.set_u32(list + 0x10, list);
            t.set_u32(list + 0xc, list);
            t.set_u16(list + 0x18, 1);
        }
        t.set_u32(r + 0x2c4 + 0x10, r + 0x2c4);
        t.set_u32(r + 0x2c4 + 0xc, r + 0x2c4);
        t.set_u32(r + 0x374 + 0x14, 0);
        // the flag words
        // (bit 0 clear where it is set, set where it is cleared)
        for (o, v) in [
            (0x17bu32, 0x5a5a_5a50u32),
            (0x5ef, 0x5a5a_5a40),
            (0x70d, 0x5a5a_5a44),
            (0x84e, 0x5a5a_5a51),
        ] {
            let at = w(&t, o);
            t.set_u32(at, v ^ (k << 8));
        }
        magic_h6::scrub(&mut t);
        t.check_entry(
            f,
            Call {
                eax: 0x0eee_0000 + k,
                ..Call::default()
            },
            &[task],
            Eax::Checked,
        );
    }
}

/// `magic_0164` 0x10009760 (`r44::summon_task_0164`, a task callback):
/// every state of the byte table (the waits below and at their bounds,
/// slot 520's modes), 400..402 (the flag both ways), 410..412, 700 (four
/// modes), 10000, gaps and states above.
#[test]
fn summon_task_0164() {
    let Some(mut t) = twin(164) else { return };
    let f = t.at(0x9760);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let call = |t: &Twin, o: u32| t.img.rel32_target(f + o);
    // the summon start's set-up: its stubs (and every slot answering)
    let su = t.at(SETUP_0164.at);
    let (fourth, class) = setup_stubs(&mut t, su, &SETUP_0164);
    for (s, n) in [
        (718u32, 1usize),
        (282, 1),
        (281, 1),
        (283, 2),
        (287, 1),
        (30, 1),
        (36, 2),
        (228, 2),
    ] {
        rec(&mut t, s, n, move |_, a| {
            let second = if n > 1 { a[1] } else { 0 };
            (s << 16) ^ a[0] ^ second.rotate_left(9)
        });
    }
    t.stub_st0(282, |_, a| f64::from(a[0] & 0x7) * 0.25);
    rec(&mut t, 520, 0, |m, _| {
        let k = m.r32(SCRATCH + 0xa14);
        m.w32(SCRATCH + 0xa14, k + 1);
        m.r32(SCRATCH + 0xa10 + 8 * (k & 1))
    });
    rec(&mut t, 288, 2, |m, a| {
        m.w32(a[1], 7);
        SCRATCH + 0x7a0
    });
    t.write(SCRATCH + 0x7a0, &[3, 1, 4, 1, 5, 2, 6, 0]);
    rec(&mut t, 390, 1, |_, a| {
        [0x106a, 0x10f8, 0x10fa, 7, 9, 11, 3][(a[0] % 7) as usize]
    });
    rec(&mut t, 307, 0, |m, _| m.r32(SCRATCH + 0x7e40));
    t.stub_st0(307, |m, _| {
        f64::from(m.r32(SCRATCH + 0x7e40) & 0xff) * 0.01 + 0.25
    });
    let task = SCRATCH + 0x100;
    let r = SCRATCH + 0x200;
    // the static parents and lists the states append to
    let parents = [
        w(&t, 0x63),
        w(&t, 0x7c),
        w(&t, 0x9a),
        w(&t, 0xe0),
        w(&t, 0x196),
        w(&t, 0x21a),
    ];
    let sl = call(&t, 0x77);
    let sl_last = t.img.word(sl + 0x1);
    let sn = call(&t, 0x3aa);
    // (state, the wait `R + 0x38`, slot 520's two answers, the flag)
    let cases: [(u32, u32, u32, u32, u8); 36] = [
        (0, 0, 0, 0, 0),
        (100, 5, 0x024f_000a, 0, 0),
        (101, 7, 7, 0, 0),
        (102, 3, 0, 0, 0),
        (102, 0x10, 0, 0, 0),
        (103, 3, 0, 0, 0),
        (103, 0x14, 0, 0, 0),
        (104, 3, 0, 0, 0),
        (104, 0xa, 0, 0, 0),
        (105, 0, 0, 0, 0),
        (106, 0, 0, 0, 0),
        (150, 0, 0, 0, 0),
        (151, 0, 0, 0, 0),
        (152, 0, 0, 0, 0),
        (160, 0, 0, 0, 0),
        (161, 0, 0, 0, 0),
        (180, 0, 0, 0, 0),
        (181, 0, 0, 0, 0),
        (200, 0, 0x024b_0005, 0, 0),
        (200, 0, 0x024f_000a, 0, 0),
        (200, 0, 3, 0, 0),
        (201, 0, 0, 0, 0),
        (0x190, 0, 0x024b_0005, 0, 0),
        (0x190, 0, 7, 0, 0),
        (0x191, 0, 0, 0, 1),
        (0x192, 0, 0, 0, 0),
        (0x192, 0, 0, 0, 1),
        (0x19a, 0, 0, 0, 0),
        (0x19b, 0, 0, 0, 0),
        (0x19c, 5, 0, 0, 0),
        (0x19c, 0x28, 0, 0, 0),
        (0x2bc, 0, 0x0237_0000, 0x0238_0000, 0),
        (0x2bc, 0, 0x0238_0000, 1, 0),
        (0x2bc, 0, 0x024f_000a, 1, 0),
        (0x2bc, 0, 3, 1, 0),
        (0x2710, 0, 0, 0, 0),
    ];
    let more: [(u32, u32, u32, u32, u8); 3] =
        [(7, 0, 0, 0, 0), (0x1a0, 0, 0, 0, 0), (0x3000, 0, 0, 0, 0)];
    // the globals the states store to (the block the code names): floats
    // first, the pointers and lists set after
    // (from the summon block's page up)
    let lo = t.img.word(t.at(0x8e40) + 0x11) & !0xfff;
    for (k, (state, n38, r1, r2, flag)) in (0u32..).zip(cases.into_iter().chain(more)) {
        for i in 0..0x1800u32 {
            let x = 1.25 + ((i * 13 + k) % 61) as f32 * 0.0625;
            // (low mantissa bits set: halfword stores of 0 show)
            t.set_u32(lo + 4 * i, x.to_bits() | (0x101 + (i & 0x7f)));
        }
        // pointers the paths follow: the leaving node, the current list node
        let at = w(&t, 0xd2a);
        t.set_u32(at + 0x14, 0);
        let lu = call(&t, 0xd34);
        let at = t.img.word(lu + 0x02);
        t.set_u32(at, 0);
        junk(&mut t, r, 0x60, k);
        for o in [0x1cu32, 0x20, 0x24, 0x28, 0x2c] {
            t.set_u32(r + o, (0.75 + 0.125 * (o + k) as f32).to_bits());
        }
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x38, n38);
        t.set_u32(r + 0x3c, 0x40 + k);
        t.set_u32(task + 4, r);
        let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3680, SCRATCH + 0x36c0);
        t.set_u32(task, node);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        for &p in &parents {
            t.set_u32(p + 0x10, p);
            t.set_u32(p + 0xc, p);
            t.set_u16(p + 0x18, 1);
        }
        for (last, blk) in [
            (sl_last, SCRATCH + 0x7480),
            (t.img.word(sn + 0x24), SCRATCH + 0x74c0),
        ] {
            junk(&mut t, blk, 0x20, k);
            t.set_u32(last, blk);
        }
        t.set_u32(SCRATCH + 0xa10, r1);
        t.set_u32(SCRATCH + 0xa18, r2);
        t.set_u32(SCRATCH + 0xa14, 0);
        t.set_u32(SCRATCH + 0x7e40, 0x31 + 7 * k);
        let fl = w(&t, 0xc7c);
        t.set_u32(fl, u32::from(flag) | 0x0300);
        t.set_u32(fourth, class);
        // the summon's block for the start (two models)
        let blk = t.img.word(t.at(0x8e40) + 0x11);
        summon_block(&mut t, &SETUP_0164, blk, 2, k, None);
        // the effect whose time is left, the models compared with
        let fx = w(&t, 0xba);
        junk(&mut t, fx, 0x100, k);
        t.set_u8(fx + 0x90, (k & 1) as u8);
        let models = w(&t, 0x142);
        for (j, who) in (0u32..).zip([9u32, 4, 8]) {
            t.set_u32(models + 0x274 * j, who);
        }
        let nm = w(&t, 0x12a);
        t.set_u32(nm, 3);
        // the floats the states move (no NaNs)
        for op in [
            0x32fu32, 0x6eb, 0x85f, 0x924, 0x939, 0x94e, 0x9a7, 0x9c4, 0x9d9, 0xa2c, 0xa52, 0xb6f,
            0xc85, 0xcdc, 0x245, 0x255, 0x268,
        ] {
            let at = w(&t, op);
            t.set_u32(at, (1.5f32 + op as f32 / 256.0).to_bits());
        }
        ownb::dirty_stack(&mut t, 0x9760 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}
