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
    clippy::too_many_lines
)]

mod magic;
#[path = "magic/ownb.rs"]
mod ownb;

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
    // slot 390 answers the word its argument points at (0x1084 for a model
    // whose place slot 268 gives)
    rec(t, 390, 1, |m, a| m.r32(a[0]));
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

/// The block `blk` with `n` models (flag bit 0 alternating with `k`, the
/// `+0x50` words pointing at slot 390's answers: 0x1084 at model `own`).
fn summon_block(t: &mut Twin, s: &Setup, blk: u32, n: u32, k: u32, own: Option<u32>) {
    junk(t, blk, 0x1700, k);
    floats(t, blk + s.matrix, 16, 0.5 + k as f32);
    t.set_u32(blk + s.count, n);
    for i in 0..n {
        let o = blk + s.base + s.stride * i;
        t.set_u8(o + 0x90, u8::from((i + k) % 2 == 1));
        let ans = SCRATCH + 0x7800 + 0x10 * i;
        t.set_u32(o + 0x50, ans);
        t.set_u32(
            ans,
            if own == Some(i) {
                0x1084
            } else {
                0x1083 + 2 * i
            },
        );
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
