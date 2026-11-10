//! Round 42's ports (`ffx_magic::r42`) against each original. Needs
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

/// Every slot answering `first word ^ (slot << 20)` and logging its first
/// word (into `SCRATCH + 0xa00..`).
fn all_stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |m, a| {
            let k = m.r32(SCRATCH + 0xa00);
            if let Some(w) = a.first() {
                m.w32(SCRATCH + 0xa10 + 4 * (k % 0x100), *w ^ s);
            }
            m.w32(SCRATCH + 0xa00, k + 1);
            a.first().copied().unwrap_or(0) ^ (s << 20)
        });
    }
}

/// The vector and matrix slots as data flow (all logged): 325 copies four
/// words `b -> a`, 233 `a = b * s` (four lanes, `s` a float word), 229 `a =
/// b + c`, 232 `a = 2 b` (four lanes), 231 `a = b + c` (sixteen), 327 `a =
/// b ^ r` (sixteen words), 210 and 9 answer from their words.
fn matrix_stubs(t: &mut Twin) {
    rec(t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x325
    });
    rec(t, 233, 3, |m, a| {
        let s = f32::from_bits(a[2]);
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * s;
            wf(m, a[0] + 4 * i, x);
        }
        0x233
    });
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    rec(t, 232, 2, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * 2.0;
            wf(m, a[0] + 4 * i, x);
        }
        0x232
    });
    rec(t, 231, 3, |m, a| {
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x231
    });
    rec(t, 327, 2, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, a[1] ^ i.wrapping_mul(0x0101_0101));
        }
        0x327
    });
    rec(t, 210, 2, |_, a| a[0] ^ 0x0210_0000);
    rec(t, 9, 3, |_, a| a[0] ^ a[1].rotate_left(8));
    rec(t, 21, 3, |_, _| 0x21);
}

/// A ring like `sa::ring` with the owner at `root` (`children` nodes from
/// `sa::node(1)`), the cursor on it.
fn ring_at(t: &mut Twin, root: u32, children: u32, pass: u8, cursor_pass: u8, fill: u8) {
    for i in 1..=children {
        t.write(sa::node(i), &[fill; 0x200]);
        t.set_u32(sa::node(i), sa::VTABLE);
    }
    t.set_u8(root + 0x1a, pass);
    let first = if children == 0 { root } else { sa::node(1) };
    t.set_u32(root + 0x10, first);
    for i in 1..=children {
        let next = if i == children { root } else { sa::node(i + 1) };
        t.set_u32(sa::node(i) + 8, next);
    }
    t.set_u32(sa::CURSOR, root);
    t.set_u8(sa::CURSOR + 4, 0x7f);
    t.set_u8(sa::CURSOR + 5, cursor_pass);
}

/// `magic_0186` 0x10005390 (`r42::jmp_matrix_visit_0186` over
/// `matrix_visit_0186` 0x10004f00): the test rings; the fade off, running
/// (`i` below, at and past `n`, `n` 0) and the follow flag `+0x1f0` on
/// and off; every entry of the DLL's function table (routed to stubs on
/// both sides: the original would run them inline).
#[test]
fn matrix_visit_0186() {
    let Some(mut t) = twin(186) else { return };
    all_stubs(&mut t);
    sa::handler_stubs(&mut t);
    matrix_stubs(&mut t);
    let thunk = t.at(0x5390);
    let f = t.at(0x4f00);
    // the function table: five thunks, each routed to a stub of its own
    let table = t.img.word(f + 0x176);
    for i in 0..5u32 {
        let s = 1010 + i;
        rec(&mut t, s, 2, move |_, a| a[0] ^ (s << 16));
        t.set_u32(table + 4 * i, TRAPS + 16 * s);
    }
    let owner = SCRATCH + 0x8000;
    let objs = [SCRATCH + 0x8400, SCRATCH + 0x8480, SCRATCH + 0x8500];
    let mut k = 0u32;
    for &(c, p, cp) in sa::RINGS {
        for (fade, i, n, follow) in [
            (0u32, 0i32, 4i32, 0u32),
            (1, 1, 4, 1),
            (1, 4, 4, 0),
            (1, 6, 4, 1),
            (1, -2, 0, 0),
            (7, 3, -5, 1),
        ] {
            k += 1;
            ownb::dirty_stack(&mut t, 0x4f00 + k, &[]);
            t.write(owner, &[0x3c ^ k as u8; 0x220]);
            ring_at(&mut t, owner, c, p, cp, 0x30 + k as u8);
            for (j, &o) in objs.iter().enumerate() {
                t.write(o, &[0x41 + j as u8; 0x80]);
                t.set_u32(o, sa::VTABLE);
                ownb::floats(
                    &mut t,
                    o + 0x1c,
                    &[1.5 + j as f32, -2.0, 0.25 * k as f32, 0.75],
                );
            }
            t.set_u32(owner + 0x13c, objs[0]);
            t.set_u32(owner + 0x140, objs[1]);
            t.set_u32(owner + 0x144, objs[2]);
            t.set_u8(owner + 0x148, (k % 5) as u8);
            for (o, x) in [(0x5cu32, 0.5f32), (0x14c, -1.25), (0x1cc, 3.0)] {
                for q in 0..16u32 {
                    t.set_u32(owner + o + 4 * q, (x + q as f32 * 0.125).to_bits());
                }
            }
            ownb::floats(&mut t, owner + 0x1bc, &[2.0, -3.5, 0.1 * k as f32]);
            ownb::floats(&mut t, owner + 0x1dc, &[1.25, 0.5 + k as f32]);
            t.set_u32(owner + 0x1e8, 0x01e8_0000 + k);
            t.set_u32(owner + 0x1ec, 0x01ec_0000 + k);
            t.set_u32(owner + 0x1f0, follow);
            t.set_u32(owner + 0x1f4, 0x01f4_0000 + k);
            t.set_u32(owner + 0x1f8, fade);
            t.set_u32(owner + 0x1fc, i as u32);
            t.set_u32(owner + 0x200, n as u32);
            t.check_entry(
                thunk,
                Call::default(),
                &[sa::CURSOR, 0x0bad_f00e],
                Eax::Checked,
            );
        }
    }
    assert!(t.u32(ownb::LOG) > 0x100, "the slots' calls");
}

/// Junk words at the globals the operands `ops` of `f` name.
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, 0x5a5a_5a5a ^ o);
    }
}

/// The slots of `magic_0072`'s records task: 723/724 fill their matrices,
/// 235 the identity, 330 `(out, scale, angle)` and 230 `(out, in)` mix
/// what they read, 229 `(out, matrix, vector)`, 404 a float in ST0 from
/// its word, 941 a frame table, 772 a 64-bit handle; the part link's
/// 774/778 hash the colour and both frame matrices the owner record
/// (`G_REC`) points at; the link's lock global at a scratch word.
fn stubs_0072(t: &mut Twin, f: u32) {
    all_stubs(t);
    let table = SCRATCH + 0x3e00;
    t.stub(941, move |m, a| {
        let k = m.r32(SCRATCH + 0x3f80);
        m.w32(SCRATCH + 0x3f84 + 4 * (k % 8), a[0]);
        m.w32(SCRATCH + 0x3f80, k + 1);
        table
    });
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    for s in [723u32, 724] {
        rec(t, s, 1, move |m, a| {
            for i in 0..16 {
                wf(m, a[0] + 4 * i, 0.25 * i as f32 - 1.5 + (s & 1) as f32);
            }
            s
        });
    }
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    rec(t, 330, 3, |m, a| {
        let r = f32::from_bits(a[2]);
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i) * 0.5 + r * i as f32;
            wf(m, a[0] + 4 * i, x);
        }
        0x330
    });
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let mut x = rf(m, a[2] + 4 * i);
            for j in 0..4 {
                x += rf(m, a[1] + 16 * j + 4 * i) * 0.125;
            }
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    rec(t, 230, 2, |m, a| {
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i) - 0.5 * i as f32;
            wf(m, a[0] + 4 * i, x);
        }
        0x230
    });
    rec(t, 404, 1, |_, a| a[0] ^ 0x404);
    t.stub_st0(404, |m, a| {
        let k = m.r32(SCRATCH + 0x3fa0);
        m.w32(SCRATCH + 0x3fa0, k + 1);
        f64::from(f32::from_bits(a[0])) * 0.3 + f64::from(k) * 0.1
    });
    for s in [774u32, 778] {
        t.stub(s, move |m, a| {
            // (handle lo, handle hi, part, owner, ..): the owner record
            let rec = a[3];
            let mut x = s ^ a[2] ^ m.r32(rec + 4);
            for off in [0x10u32, 0x1c] {
                let mat = m.r32(rec + off);
                for i in 0..16 {
                    x = x.rotate_left(5) ^ m.r32(mat + 4 * i);
                }
            }
            let k = m.r32(SCRATCH + 0x3fc0);
            m.w32(SCRATCH + 0x3fc4 + 4 * (k % 15), x);
            m.w32(SCRATCH + 0x3fc0, k + 1);
            x
        });
    }
    let link = t.img.rel32_target(f + 0x345);
    for off in [0x4fu32, 0x7a, 0xa9, 0xc6, 0x115, 0x13b] {
        let g = t.img.word(link + off);
        t.set_u32(g, SCRATCH + 0x3f00);
    }
}

/// One record of `magic_0072`'s task at `r`: `kind` picks the way through
/// it (0 off, 1 delayed, 2 timed, 3 fading, 4 faded out, 5 below the
/// floor, 6 a NaN timer and height).
fn record_0072(t: &mut Twin, r: u32, kind: u32, c: u32) {
    t.write(r, &[0x27 ^ (kind + c) as u8; 0x6c]);
    t.set_u32(r, 0x0041_0000 + kind);
    t.set_u32(r + 4, (kind + c) & 1);
    t.set_u16(r + 8, 2);
    t.set_u16(r + 0xa, if kind == 3 { 0x100 } else { 0x300 });
    ownb::floats(t, r + 0xc, &[1.0 + c as f32, 5.0, -2.0, 1.0]);
    ownb::floats(t, r + 0x1c, &[0.5, -0.75 * kind as f32, 0.25, 0.0]);
    ownb::floats(t, r + 0x2c, &[0.125, -0.5, 0.0625, 0.0]);
    t.write(r + 0x3c, &[0x10, 0x80, 0xf0, 0x40 + kind as u8]);
    t.write(r + 0x40, &[0xe0, 0x20 + c as u8, 0x08, 0xc0]);
    ownb::floats(t, r + 0x48, &[0.0, 1.5 + kind as f32, -0.5]);
    // the floor: above it for kinds 5 and 6 (the height 5.0 - speed)
    let floor = if kind == 5 { -10.0 } else { 40.0 };
    ownb::floats(t, r + 0x54, &[floor]);
    t.set_u32(r + 0x58, if kind == 1 { 3 } else { (c & 1).wrapping_neg() });
    let (fade, at) = match (kind, c % 3) {
        (3, 0) => (4, 1),
        (3, 1) => (3, -3),
        (3, _) => (2, 2),
        (2 | 5, _) => (5, 0),
        _ => (0, 0),
    };
    t.set_u32(r + 0x5c, fade as u32);
    t.set_u32(r + 0x60, at as u32);
    let timer = match kind {
        2 => 2.0f32,
        6 => f32::NAN,
        _ if c == 2 => -1.0,
        _ => 0.0,
    };
    ownb::floats(t, r + 0x44, &[timer]);
    if kind == 6 {
        ownb::floats(t, r + 0x10, &[f32::NAN]);
    }
    t.set_u32(r + 0x64, u32::from(kind % 2 == 1));
    t.set_u32(r + 0x68, u32::from(kind != 0) * (0x10 + kind));
}

/// `magic_0072` 0x10002760 (`r42::records_task_0072`): no records (0 and
/// negative counts) and records of every kind, the hold global set and
/// not.
#[test]
fn records_task_0072() {
    let Some(mut t) = twin(72) else { return };
    let f = t.at(0x2760);
    stubs_0072(&mut t, f);
    let table = SCRATCH + 0x3e00;
    let recs = SCRATCH + 0x8000;
    let lists: [&[u32]; 5] = [
        &[],
        &[0, 1, 2, 3, 4, 5, 6],
        &[5, 3, 3, 2, 4, 1],
        &[3, 6, 5, 0, 2],
        &[2, 5, 3, 4],
    ];
    for (c, kinds) in (0u32..).zip(lists) {
        for hold in [0u32, 1] {
            let c = c * 2 + hold;
            junk_ops(
                &mut t,
                f,
                &[0x55, 0x61, 0x37b, 0x141, 0x178, 0x196, 0x1b4, 0x1d6, 0x1f2],
            );
            ownb::fill(&mut t, table, 0x20, 17 + c);
            t.set_u16(table + 6, 5);
            t.write(SCRATCH + 0x3f00, &[0x31; 0x40]);
            for (i, &k) in (0u32..).zip(kinds) {
                record_0072(&mut t, recs + 0x6c * i, k, c);
            }
            t.set_u32(t.img.word(f + 0x1f), recs);
            let n = if c == 1 {
                0xffff_fffe
            } else {
                kinds.len() as u32
            };
            t.set_u32(t.img.word(f + 0x7c), n);
            t.set_u32(t.img.word(f + 0xc0), hold);
            ownb::dirty_stack(&mut t, 0x2760 + c, &[]);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
    assert!(t.u32(ownb::LOG) > 0x100, "the task's calls");
}

/// `n` bytes of junk at `at` (varied by `k`).
fn junk(t: &mut Twin, at: u32, n: u32, k: u32) {
    let b: Vec<u8> = (0..n)
        .map(|i| (i.wrapping_mul(0x3d) ^ (0x71 + k)).to_le_bytes()[0])
        .collect();
    t.write(at, &b);
}

/// `magic_0152` 0x10008580 (`r42::screen_setup_0152`, `magic_r31.rs`'s
/// `screen_setup_0601` with the insert inlined): the parent's child list
/// empty and not; slot 687's answer varied.
#[test]
fn screen_setup_0152() {
    let Some(mut t) = twin(152) else { return };
    all_stubs(&mut t);
    for (s, n) in [(687u32, 3usize), (692, 3), (691, 7), (690, 5), (693, 2)] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a[0] ^ a[1].rotate_left(9)
        });
    }
    let f = t.at(0x8580);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let obj = SCRATCH + 0x6000;
    let parent = w(&t, 0x135);
    let tail = w(&t, 0x1c);
    for (k, empty) in (0u32..).zip([true, false, true]) {
        junk(&mut t, obj, 0x300, k);
        junk(&mut t, SCRATCH + 0x7400, 0x20, k);
        t.set_u32(tail, SCRATCH + 0x7400);
        let fl = w(&t, 0x3b);
        t.set_u32(fl, 0xffff_0003 + k);
        let kid = SCRATCH + 0x7500;
        junk(&mut t, kid, 0x20, k + 3);
        t.set_u32(parent + 0x10, if empty { parent } else { kid });
        t.set_u32(parent + 0xc, if empty { parent } else { kid });
        t.set_u16(parent + 0x18, 2 + k as u16);
        ownb::dirty_stack(&mut t, 0x8580 + k, &[]);
        t.check(f, Call::this(obj), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r42::screen_setup_0152(g, img, fr)
        });
    }
}

/// `magic_0042` 0x10001350 (`r42::light_visit_0042`) and its thunk
/// 0x100016b0: the node already marked; no children and two; the flags
/// 0..3 with `+0x28` above, at and below the light's bound (NaN once); the
/// step and fade before, across and at their end.
#[test]
fn light_visit_0042() {
    let Some(mut t) = twin(42) else { return };
    all_stubs(&mut t);
    for (s, n) in [
        (231u32, 3usize),
        (245, 1),
        (563, 4),
        (821, 1),
        (822, 4),
        (823, 2),
        (824, 4),
    ] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    let visit = 1000u32;
    rec(&mut t, visit, 1, |m, a| 0x1000 ^ m.r32(a[0]));
    let class = SCRATCH + 0x3700;
    t.set_u32(class + 0x1c, magic::TRAPS + 16 * visit);
    let f = t.at(0x1350);
    let c = SCRATCH + 0x3000;
    let (node, pa, pb) = (SCRATCH + 0x3100, SCRATCH + 0x3300, SCRATCH + 0x3400);
    let kids = [SCRATCH + 0x3200, SCRATCH + 0x3240];
    let bound = f32::from_bits(t.u32(t.img.word(f + 0x7b)));
    let counter = t.img.word(f + 0xb7);
    // (marked, children, flags, +0x28, speed, step, fade)
    let cases: [(bool, usize, u8, f32, f32, f32, f32); 11] = [
        (true, 0, 0, 1.0, 1.0, 0.0, 0.0),
        (false, 0, 0, 1.0, 1.0, 0.25, 0.5),
        (false, 2, 1, bound + 1.0, 2.0, 0.25, 0.5),
        (false, 2, 3, bound + 1.0, 2.0, 0.25, 0.5),
        (false, 0, 1, bound - 1.0, 2.0, 0.75, 0.5),
        (false, 0, 1, bound, 2.0, 0.75, 0.5),
        (false, 0, 2, 1.0, 2.0, 0.75, 0.5),
        (false, 0, 0, 1.0, 2.0, 0.75, 0.95),
        (false, 0, 0, 1.0, 2.0, 1.5, 0.25),
        (false, 0, 1, f32::NAN, 2.0, 0.25, 0.5),
        (false, 1, 1, bound + 2.0, 0.5, 0.25, 0.5),
    ];
    for (k, (marked, children, flags, x28, speed, step, fade)) in (0u32..).zip(cases) {
        let eax = if marked { Eax::Junk } else { Eax::Checked };
        for thunk in [false, true] {
            t.write(node, &[0x44; 0x80]);
            for i in 0..19u32 {
                let x = 0.25 * (k + i) as f32 - 1.5;
                t.set_u32(node + 0x1c + 4 * i, x.to_bits());
            }
            t.set_u32(node + 0x4c, 0.375f32.to_bits());
            t.set_u32(node + 0x54, 0.125f32.to_bits());
            t.set_u32(node + 0x50, step.to_bits());
            t.set_u32(node + 0x58, fade.to_bits());
            t.set_u32(node + 0x28, x28.to_bits());
            t.set_u32(node + 0x5c, speed.to_bits());
            t.set_u8(node + 0x68, flags);
            t.set_u8(node + 0x1a, 0xf0);
            t.set_u32(c, node);
            t.set_u8(c + 4, 0x55);
            t.set_u8(c + 5, if marked { 0xf0 } else { 3 + k as u8 });
            let mut prev = node;
            for &kid in kids.iter().take(children).rev() {
                t.write(kid, &[0x22; 0x10]);
                t.set_u32(kid, class);
                t.set_u32(kid + 8, prev);
                prev = kid;
            }
            t.set_u32(node + 0x10, prev);
            for part in [pa, pb] {
                t.write(part, &[0x11; 0xc0]);
                t.set_u32(part, class);
                for i in 0..3u32 {
                    t.set_u32(part + 0x1c + 4 * i, (0.5 * (i + k) as f32).to_bits());
                }
            }
            t.set_u32(node + 0x6c, pa);
            t.set_u32(node + 0x70, pb);
            t.write(SCRATCH + 0x5000, &[0x0c; 0x400]);
            t.set_u32(counter, SCRATCH + 0x5000 + 0x40 * k);
            // the sprite packet's frame words `[ebp - 0x10]`/`[ebp - 0xc]`
            // (its junk read: 0 on both sides), 0xb8/0xb4 below the entry
            ownb::dirty_stack(&mut t, 0x1350 + k, &[0xb8, 0xb4]);
            if thunk {
                t.check_entry(t.at(0x16b0), Call::default(), &[c], eax);
            } else {
                t.check(f, Call::this(c), &[], eax, |g, img, fr| {
                    ffx_magic::Ret::eax(ffx_magic::r42::light_visit_0042(
                        g, img, fr.at, fr.esp, fr.ecx,
                    ))
                });
            }
        }
    }
}
