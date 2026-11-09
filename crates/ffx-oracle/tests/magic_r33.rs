//! Round 33's ports (`ffx_magic::r33`) against each original. Needs
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

use ffx_magic::{Ret, r33};
use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use magic_owna::filler;
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

/// Every slot answering `first word ^ (slot << 20)`.
fn all_stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
}

/// Junk floats at the globals the operands `ops` of `f` name.
fn junk_floats(t: &mut Twin, f: u32, ops: &[u32], k: u32) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, (2.5f32 + o as f32 / 512.0 + k as f32).to_bits());
    }
}

/// The matrix and vector slots the summon tasks call, with results
/// written: 229 `out = row0 + row3 + 2 v`; 327 a matrix; 262 a place
/// from the object word; 366 a vector; 14 three floats; 601 a float; 290
/// four floats; 32 (the bones' function table) four floats.
fn summon_stubs(t: &mut Twin) {
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[1] + 0x30 + 4 * i) + 2.0 * rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
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
    rec(t, 32, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, 0.5 * i as f32 - 1.0);
        }
        0x32
    });
}

/// The float slots of the matrices: 235 a unit matrix; 330/328/329 `out =
/// in + angle (i + 1) / 8`; the scale slots 15/16/17/277 and the random
/// 404/314 floats in ST0 (as `magic_r31.rs`).
fn float_matrix_stubs(t: &mut Twin) {
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
    for (s, x) in [(15u32, 1.5f64), (16, -0.75), (17, 3.0), (277, 40.0)] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    for (s, x) in [(404u32, 0.0625f64), (314, -0.125)] {
        rec(t, s, 1, move |m, a| {
            let k = m.r32(SCRATCH + 0xa00);
            m.w32(SCRATCH + 0xa00, k + 1);
            (s << 16) ^ a[0]
        });
        t.stub_st0(s, move |m, a| {
            x * f64::from(m.r32(SCRATCH + 0xa00) % 5) + f64::from(f32::from_bits(a[0]))
        });
    }
}

/// The vector slots with their results written: 232 `out = v / 2`, 233
/// `out = v s`, 234 `out = a - b`, 266 the cross product, 230 `a += b`.
fn vector_stubs(t: &mut Twin) {
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
    rec(t, 234, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x234
    });
    rec(t, 266, 3, |m, a| {
        let u = [0, 1, 2].map(|i| rf(m, a[1] + 4 * i));
        let v = [0, 1, 2].map(|i| rf(m, a[2] + 4 * i));
        let c = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        for (i, x) in (0u32..).zip(c) {
            wf(m, a[0] + 4 * i, x);
        }
        0x266
    });
    rec(t, 230, 2, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[0] + 4 * i) + rf(m, a[1] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x230
    });
}

/// The child visit stub (slot `visit`): logs the cursor's node and mark.
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
    t.set_u32(class + 0x1c, magic::TRAPS + 16 * visit);
}

/// `magic_0609` 0x10009c80 (`r33::summon_task_0609`): every state, the
/// start with the effect's flag both ways, empty and non-empty static
/// lists, the actors' loop (skipped, found among the models, the three
/// kinds that skip, the rest; no models), the waits' bounds, states the
/// table leaves alone, above it and 10000.
#[test]
fn summon_task_0609() {
    let Some(mut t) = twin(609) else { return };
    all_stubs(&mut t);
    float_matrix_stubs(&mut t);
    vector_stubs(&mut t);
    summon_stubs(&mut t);
    let f = t.at(0x9c80);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let call = |t: &Twin, o: u32| t.img.rel32_target(f + o);
    for (s, n) in [
        (718u32, 1usize),
        (12, 1),
        (282, 1),
        (281, 1),
        (283, 2),
        (287, 1),
        (211, 2),
        (413, 1),
        (215, 1),
        (325, 2),
    ] {
        rec(&mut t, s, n, move |_, a| {
            let second = if n > 1 { a[1] } else { 0 };
            (s << 16) ^ a[0] ^ second.rotate_left(9)
        });
    }
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xf) * 0.5 - 1.0);
    t.stub_st0(282, |_, a| f64::from(a[0] & 0x7) * 0.25);
    t.stub_st0(790, |_, _| 0.375);
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xf) * 0.25 + 0.5);
    // slot 262 (four words) also writes its three floats
    rec(&mut t, 558, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 * i + (a[1] & 0xf));
        }
        0x558
    });
    rec(&mut t, 288, 2, |m, a| {
        let n = m.r32(SCRATCH + 0xa20);
        m.w32(a[1], n);
        SCRATCH + 0x700
    });
    t.write(SCRATCH + 0x700, &[3, 1, 4, 1, 5, 2, 6, 0]);
    rec(&mut t, 390, 1, |_, a| {
        [0x106a, 0x10f8, 0x10fa, 7, 9, 11, 3][(a[0] % 7) as usize]
    });
    rec(&mut t, 296, 2, |_, a| 0x0296_0000 ^ a[0] ^ (a[1] << 8));
    // the summon set-up's bones: visits and the function table to stubs
    let setup = call(&t, 0x459);
    let (visit, class) = (1000u32, SCRATCH + 0x3780);
    visit_stub(&mut t, visit, class);
    let fourth = t.img.word(setup + 0x3d7);
    for op in [
        0xdeu32, 0x1a6, 0x2bc, 0x2f6, 0x366, 0x4a9, 0x521, 0x566, 0x62d,
    ] {
        let vt = t.img.word(setup + op);
        t.set_u32(vt + 0x1c, magic::TRAPS + 16 * visit);
    }
    let bt = t.img.rel32_target(setup + 0x3ed);
    let table = t.img.word(bt + 0xed);
    for i in 0..4 {
        t.set_u32(table + 4 * i, magic::TRAPS + 16 * 32);
    }
    let (a, s) = (SCRATCH + 0x100, SCRATCH + 0x1_0000);
    // the static parents the start appends to (the common one, the
    // pair's and the colour node's)
    let parents = [w(&t, 0x21f), w(&t, 0x4a9), w(&t, 0x3b6)];
    // the task list's last (task start and link object) and the
    // static parent's (`static_link_word`)
    let lasts = [
        t.img.word(call(&t, 0x241) + 0x01),
        t.img.word(call(&t, 0x466) + 0x01),
    ];
    // (state, the counter `S + 0x38`, the flag, the actors' count, the
    // models' count, lists empty)
    let cases: [(u32, u32, u8, u32, u32, bool); 22] = [
        (0, 0, 0, 0, 0, true),
        (0x190, 0, 0, 0, 0, true),
        (0x191, 0, 1, 0, 0, true),
        (0x191, 0, 0, 0, 0, true),
        (0x192, 0, 0, 0, 0, true),
        (0x192, 0, 1, 0, 0, true),
        (0x19a, 0, 0, 7, 3, true),
        (0x19a, 7, 1, 8, 3, false),
        (0x19a, 3, 0, 5, 0, false),
        (0x19a, 0, 1, 0, 2, true),
        (0x19b, 0, 0, 0, 0, true),
        (0x19b, 9, 0, 0, 0, true),
        (0x19c, 5, 0, 0, 0, true),
        (0x19c, 0x30, 0, 0, 0, true),
        (0x19c, 0x27, 0, 0, 0, true),
        (0x19c, 0x28, 0, 0, 0, true),
        (0x2bc, 0, 0, 0, 0, true),
        (0x2710, 0, 0, 0, 0, true),
        (0x1a0, 0, 0, 0, 0, true),
        (0x150, 0, 0, 0, 0, true),
        (0x2bd, 0, 0, 0, 0, true),
        (0x3000, 0, 0, 0, 0, true),
    ];
    for (k, (state, n38, flag, actors, models, empty)) in (0u32..).zip(cases) {
        junk(&mut t, s, 0x60, k);
        t.set_u32(s + 0x18, state);
        t.set_u32(s + 0x38, n38);
        t.set_u32(a + 4, s);
        let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3700, SCRATCH + 0x3740);
        t.set_u32(a, node);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        for (i, &p) in (0u32..).zip(&parents) {
            let blk = SCRATCH + 0x7800 + 0x40 * i;
            junk(&mut t, blk, 0x20, k + i);
            t.set_u32(p + 0x10, if empty { p } else { blk });
            t.set_u32(p + 0xc, p);
            t.set_u16(p + 0x18, 1 + i as u16);
        }
        for (i, &last) in (0u32..).zip(&lasts) {
            let blk = SCRATCH + 0x7400 + 0x40 * i;
            junk(&mut t, blk, 0x20, k + i);
            t.set_u32(last, blk);
        }
        t.set_u32(w(&t, 0x1b1), u32::from(flag) | 0x0300);
        t.set_u32(fourth, class);
        // the globals the states store or read: junk first
        junk_floats(
            &mut t,
            f,
            &[
                0x5d, 0x78, 0x8a, 0x94, 0x9e, 0xa8, 0xb2, 0xbc, 0xc6, 0xd0, 0xf2, 0xfc, 0x106,
                0x110, 0x11a, 0x124, 0x12e, 0x138, 0x142, 0x14e, 0x154, 0x15c, 0x164, 0x16a, 0x16f,
                0x175, 0x190, 0x197, 0x1ba, 0x1db, 0x1fa, 0x6aa, 0x6c0, 0x6d0, 0x7df, 0x7e6, 0x804,
                0x931, 0x97a, 0x981, 0x9f5, 0xa22, 0xa2c, 0xa36,
            ],
            k,
        );
        // the effect whose time is left (its byte `+0x90`), the actors'
        // list and the models they are compared with (0x278 apart)
        let fx = w(&t, 0x372);
        junk(&mut t, fx, 0x100, k);
        t.set_u8(fx + 0x90, (k & 1) as u8);
        t.set_u32(SCRATCH + 0xa20, actors);
        t.set_u32(w(&t, 0x8b5), 4);
        let at = w(&t, 0x8c2);
        for (j, who) in (0u32..).zip([9u32, 1, 6]) {
            t.set_u32(at + 0x278 * j, who);
        }
        t.set_u32(w(&t, 0x8a3), models);
        ownb::dirty_stack(&mut t, 0x9c80 + k, &[]);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}

/// Every slot logging its first word and count, answering `first word ^
/// (slot << 20)` (as `magic_r30.rs`).
fn all_slots(t: &mut Twin) {
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

/// The slots the 0606 placement and its sparks call: the caster's place
/// (262), its turn (333, ST0 from `rot`), the actor's height (12), no
/// group for the others (293 0), the scale and matrix floats.
fn place_stubs(t: &mut Twin, rot: u32) {
    rec(t, 262, 4, |m, a| {
        for i in 0..3u32 {
            wf(
                m,
                a[1 + i as usize],
                (a[0] & 0xff) as f32 * 0.5 + i as f32 - 1.0,
            );
        }
        0x262
    });
    t.stub_st0(333, move |m, _| f64::from(rf(m, rot)));
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xf) * 0.25 - 1.0);
    rec(t, 293, 1, |_, _| 0);
    for (sl, v) in [(277u32, 40.0f64), (15, 1.5), (16, -0.75), (17, 3.0)] {
        t.stub_st0(sl, move |_, a| v + f64::from(a[0] & 3));
    }
}

/// `magic_0606` 0x10006a20 (`r33::place_0606`, internal): slot 520's
/// scene 0x18b0046 (the caster's height) and others (the point's actor:
/// the first selected or another), the yaw wrapped down, up and not, at
/// the wrap bounds, NaN.
#[test]
fn place_0606() {
    let Some(mut t) = twin(606) else { return };
    all_slots(&mut t);
    let f = t.at(0x6a20);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x14), s);
    let sparks = t.img.rel32_target(f + 0x1d5);
    t.set_u32(t.img.word(sparks + 0x1b), s);
    let sel = SCRATCH + 0x180;
    for i in 0..8u8 {
        t.set_u8(sel + u32::from(i), 7 - i);
    }
    t.set_u32(w(&t, 0x19d), sel);
    let scene = SCRATCH + 0x7e0;
    let rot = SCRATCH + 0x7e4;
    rec(&mut t, 520, 0, move |m, _| m.r32(scene));
    place_stubs(&mut t, rot);
    // the scene point: the selected actor 5 (index 2) is model 0x1084
    rec(&mut t, 390, 1, |_, a| {
        if a[0] == 5 { 0x1084 } else { 0x1000 + a[0] }
    });
    let sp = t.img.rel32_target(f + 0x167);
    let listed = t.u32(t.img.word(sp + 0x1a));
    t.set_u32(t.img.word(sp + 0x4a), 4);
    for b in 0..8u32 {
        t.set_u32(w(&t, 0x1a8) + 4 * b, 0x0077_0040 + 3 * b);
    }
    let k0 = f32::from_bits(t.u32(w(&t, 0x90)));
    let hi = f32::from_bits(t.u32(w(&t, 0x9f)));
    let lo = f32::from_bits(t.u32(w(&t, 0xb3)));
    // the turns giving a yaw `K - r` exactly at each bound
    let at = |b: f32| {
        let guess = (k0 - b).to_bits();
        (0..256u32)
            .flat_map(|d| [guess.wrapping_add(d), guess.wrapping_sub(d)])
            .map(f32::from_bits)
            .find(|&r| (k0 - r).to_bits() == b.to_bits())
            .unwrap_or(k0 - b)
    };
    // (scene, the caster's turn)
    let cases = [
        (0x0123_4567u32, 0.5f32),
        (0x018b_0046, -2.0),
        (0x0123_4567, 9.0),
        (0x018b_0046, 3.0),
        (0x0222_0000, -9.0),
        (0x0222_0000, at(hi)),
        (0x0222_0000, at(lo)),
        (0x0222_0000, f32::NAN),
        (listed, 1.25),
    ];
    for (k, (sc, r)) in (0u32..).zip(cases) {
        t.write(s, &magic_h6::varied(0x6000, k as u8));
        t.set_u32(scene, sc);
        t.set_u32(rot, r.to_bits());
        t.set_u32(w(&t, 0x1ff), 0x55 + k);
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(r33::place_0606(g, img, fr))
        });
    }
}

/// `magic_0606` 0x10006610 (`r33::load_0606`) with its start-up
/// 0x10008a90 (`r33::start_up_0606`, which runs the placement and the
/// sparks): slot 225 busy, the first and a later request, the data not
/// loaded and loaded (the task list empty and not).
#[test]
fn load_0606() {
    let Some(mut t) = twin(606) else { return };
    all_slots(&mut t);
    let at = t.at(0x6610);
    let w = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    let busy = SCRATCH + 0x100;
    let loaded = SCRATCH + 0x104;
    rec(&mut t, 225, 1, move |m, _| m.r32(busy));
    rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
    rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
    let su = t.img.rel32_target(at + 0x5b);
    let lock = SCRATCH + 0x7f0;
    let ps = t.img.rel32_target(su + 0x56);
    for o in [0x04u32, 0x2d] {
        t.set_u32(w(&t, ps, o), lock);
    }
    let place = t.img.rel32_target(su + 0x5b);
    let sel = SCRATCH + 0x180;
    for i in 0..8u8 {
        t.set_u8(sel + u32::from(i), i);
    }
    t.set_u32(w(&t, place, 0x19d), sel);
    let rot = SCRATCH + 0x7e4;
    t.set_u32(rot, 0.5f32.to_bits());
    place_stubs(&mut t, rot);
    rec(&mut t, 520, 0, |_, _| 0x0123_4567);
    magic_r11::pools(&mut t, su, 336);
    let ri = t.img.rel32_target(su + 0x102);
    let (head, last) = (w(&t, su, 0xca), w(&t, su, 0xde));
    let old = SCRATCH + 0x3400;
    for (k, (b, asked, l, listed)) in [
        (1u32, 0u32, 1u32, false),
        (0, 0, 0, false),
        (0, 1, 0xffff_ffff, false),
        (0, 0, 1, false),
        (0, 1, 2, true),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(busy, b);
        t.set_u32(loaded, l);
        t.set_u32(w(&t, at, 0x18), asked);
        for o in [0x61u32, 0x6b] {
            t.set_u32(w(&t, at, o), 0x0bad_0000 + o);
        }
        t.set_u32(lock, 0x77 + k);
        let node = |i: u32| SCRATCH + 0x5000 + 0x40 * i;
        for i in 0..3u32 {
            t.write(node(i), &[0x33; 0x40]);
            t.set_u32(node(i), node((i + 2) % 3));
            t.set_u32(node(i) + 4, node((i + 1) % 3));
        }
        t.set_u32(w(&t, ri, 0x3e), node(0));
        t.set_u16(w(&t, ri, 0x20), 3);
        t.set_u32(w(&t, ri, 0xa6), 0);
        t.write(old, &[0x34; 0x20]);
        t.set_u32(head, if listed { old } else { 0 });
        t.set_u32(last, old);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0610` 0x1000c7b0 (`r33::clips_0610`): the list `*G_L` filled
/// and not (`S + 0x1a80` 0), the clip mode 1 (each actor's model clipped
/// by `r30::clip_0434`: a special and a plain model, no parts), modes 0
/// and 2 (slots 296/257), party members and not (`*G_ALL` set: none), one
/// to four actors, the free ring with one node and more, the task list
/// empty and not.
#[test]
fn clips_0610() {
    let Some(mut t) = twin(610) else { return };
    all_stubs(&mut t);
    let f = t.at(0xc7b0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let clip = t.img.rel32_target(f + 0x361);
    let a = SCRATCH + 0x1_0000 * 0x10;
    // the clip's slots (as `magic_r30.rs`'s clip test, no parts)
    t.stub(562, move |_, _| a);
    t.stub(544, |m, a| {
        for i in 0..4 {
            m.w32(a[1] + 4 * i, (a[0] as f32 + 0.25 * i as f32).to_bits());
        }
        0x544
    });
    rec(&mut t, 553, 5, |_, _| 0x553);
    t.stub(545, |_, _| SCRATCH + 0x4f00);
    t.write(SCRATCH + 0x4f00, &magic_h6::varied(0x40, 1));
    rec(&mut t, 941, 1, |_, a| {
        SCRATCH + 0x5800 + (a[0] & 0xf) * 0x40
    });
    for (s, n) in [(357u32, 3), (356, 3), (362, 4), (327, 2), (843, 2)] {
        rec(&mut t, s, n, move |_, a| 0x5a5a_0000 + s + a[0]);
    }
    rec(&mut t, 363, 3, |_, _| 0x44);
    let boxes = |m: &mut dyn StubMem, hi: u32, lo: u32, z: f32| {
        for (at, s) in [(lo, -1.0f32), (hi, 1.0)] {
            wf(m, at, s / 1024.0);
            wf(m, at + 4, s / 1024.0);
            wf(m, at + 8, z * s);
            wf(m, at + 12, 1.0);
        }
    };
    rec(&mut t, 924, 3, move |m, a| {
        boxes(m, a[1], a[2], 0.375);
        0x924
    });
    rec(&mut t, 934, 5, move |m, a| {
        boxes(m, a[3], a[4], 0.625);
        for at in [a[1], a[2]] {
            for i in 0..4 {
                wf(m, at + 4 * i, 3.0 + i as f32);
            }
        }
        0x934
    });
    rec(&mut t, 404, 1, |_, a| 0x0404_0000 ^ a[0]);
    t.stub_st0(404, |_, a| -9.25 + f64::from(a[0] & 0xff) / 64.0);
    // the clips' own slots: the unit matrix, places, turns, matrices
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3u32 {
            wf(
                m,
                a[1 + i as usize],
                (a[0] & 0xff) as f32 * 0.5 + i as f32 - 1.0,
            );
        }
        0x262
    });
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xf) * 0.75 - 2.0);
    rec(&mut t, 329, 3, |m, a| {
        let ang = f32::from_bits(a[2]);
        for i in 0..16 {
            let v = rf(m, a[1] + 4 * i) + ang * (i + 1) as f32 / 8.0;
            wf(m, a[0] + 4 * i, v);
        }
        0x329
    });
    rec(&mut t, 230, 2, |m, a| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x230
    });
    rec(&mut t, 231, 3, |m, a| {
        for i in 0..16 {
            let v = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * (15 - i));
            wf(m, a[0] + 4 * i, v);
        }
        0x231
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[2] + 4 * i) * 0.5 + rf(m, a[1] + 4 * i) + i as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    rec(&mut t, 314, 1, |_, a| 0x0314_0000 ^ a[0]);
    t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * 3.0 - 0.5);
    for (s, n) in [
        (530u32, 2usize),
        (249, 2),
        (296, 2),
        (257, 2),
        (238, 2),
        (548, 1),
        (210, 2),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a[0] ^ (a.get(1).copied().unwrap_or(0) << 8)
        });
    }
    let s = SCRATCH + 0x4_0000;
    let sel = SCRATCH + 0x180;
    let list = SCRATCH + 0x1c0;
    let (vb, out) = (SCRATCH + 0x8_0000, SCRATCH + 0x9_0000);
    let actors = [SCRATCH + 0x1000, SCRATCH + 0x2044];
    t.set_u32(w(&t, 0x68), 0x0bad_0033);
    let table = w(&t, 0xbc);
    // (the mode `S + 0x1a80`, actors `N`, `*G_ALL`, free nodes, the task
    // list's head set)
    let cases: [(u32, u32, u32, u16, bool); 7] = [
        (1, 3, 0, 2, true),
        (0, 3, 0, 1, false),
        (2, 2, 1, 3, true),
        (1, 1, 0, 2, false),
        (1, 4, 1, 1, true),
        (0, 0, 0, 2, false),
        (3, 4, 0, 2, false),
    ];
    for (k, (mode, n, all, free_n, listed)) in (0u32..).zip(cases) {
        t.write(s, &magic_h6::varied(0x1b00, k as u8));
        t.set_u32(s + 0x1a80, mode);
        t.set_u32(w(&t, 0x2e), s);
        t.set_u32(w(&t, 0x75), n);
        t.set_u32(w(&t, 0xe2), all);
        t.set_u32(w(&t, 0xb1), sel);
        t.write(sel, &[0x30, 2, 0x25, 5, 0x21, 1]);
        for (b, ac) in [
            (2u32, actors[0]),
            (0x25, actors[1]),
            (5, actors[1]),
            (0x21, actors[0]),
        ] {
            t.set_u32(table + 4 * b, ac);
        }
        for (i, &ac) in (0u32..).zip(&actors) {
            t.write(ac, &magic_h6::varied(0x900, (k + i) as u8));
            t.set_u16(ac, if i == 0 { 0x10fb } else { 0x1234 });
            t.set_u32(ac + 0x1c0, SCRATCH + 0x600);
            t.set_u32(ac + 0x830, 0x0830_0000 + k);
        }
        t.write(SCRATCH + 0x600, &[0; 0x20]);
        t.write(a, &magic_h6::varied(0x200, k as u8 + 3));
        t.set_u32(a, SCRATCH + 0x3f0);
        t.set_u16(SCRATCH + 0x3f0, 0x2345);
        t.set_u16(a + 4, 0);
        for o in [0x3aeu32, 0x45e] {
            let at = t.img.word(clip + o);
            t.write(at, &magic_h6::varied(0x50, k as u8 + 0x31));
        }
        t.write(list, &[0x5c; 0x20]);
        t.set_u32(w(&t, 0x8f), list);
        t.write(vb, &[0x22; 0x400]);
        t.write(out, &magic_h6::varied(0x800, k as u8 + 7));
        t.write(a + 0x5_0000, &magic_h6::varied(0x400, k as u8 + 9));
        t.set_u32(w(&t, 0x22), vb);
        t.set_u32(w(&t, 0x1b), out);
        t.set_u32(w(&t, 0x35c), 0x0bad_0035);
        // the free ring and the task list
        let node = |i: u32| SCRATCH + 0x5000 + 0x40 * i;
        for i in 0..3u32 {
            t.write(node(i), &[0x33; 0x40]);
            t.set_u32(node(i), node((i + 2) % 3));
            t.set_u32(node(i) + 4, node((i + 1) % 3));
        }
        t.set_u16(w(&t, 0x587), free_n);
        t.set_u32(w(&t, 0x596), node(k % 3));
        let old = SCRATCH + 0x5400;
        t.write(old, &[0x34; 0x20]);
        t.set_u32(w(&t, 0x5fd), if listed { old } else { 0 });
        t.set_u32(w(&t, 0x612), old);
        magic_h6::scrub(&mut t);
        // the colours `parts_vertices` reads from its frame unwritten
        // (`magic_r30.rs`'s clip test), here under the clips' frame
        t.write(0x2fff_fa80, &[0; 0x30]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// The 0x74-byte record pool `take` searches (`sb_misc::pool_take`): `n`
/// records at `recs`, those in `busy` taken, the search from `from`.
fn pool_74(t: &mut Twin, take: u32, recs: u32, n: u32, busy: &[u32], from: u32, k: u32) {
    let w = |t: &Twin, o: u32| t.img.word(take + o);
    for o in [0x37u32, 0x61] {
        t.set_u32(w(t, o), recs);
    }
    for i in 0..n {
        junk(t, recs + 0x74 * i, 0x74, k + i);
        t.set_u32(recs + 0x74 * i + 0x6c, u32::from(busy.contains(&i)));
    }
    t.set_u32(w(t, 0x0f), n);
    t.set_u32(w(t, 0x97), n);
    t.set_u32(w(t, 0x08), from);
    t.set_u32(w(t, 0x2b), 0);
}

/// `magic_0165` 0x10003990 and `magic_0606` 0x10007bd0
/// (`r33::records_task_0165`): the countdown above 1, at 1 and out; the
/// 100 records delayed or not, waiting or fading (the fade at 0 and
/// not), scaling or not, each part kind (`+0xc0` 0..3), drawn or not
/// (`L28` 0), angles past either wrap bound; the particles' records
/// delayed, with small scale counts, the pool running out.
#[test]
fn records_task_0165() {
    for (n, addr, cnt) in [(165u16, 0x3990u32, 0x5f14u32), (606, 0x7bd0, 0x5ef4)] {
        let Some(mut t) = twin(n) else { return };
        all_stubs(&mut t);
        float_matrix_stubs(&mut t);
        let f = t.at(addr);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        for (s, k) in [(239u32, 3usize), (382, 2), (559, 3), (558, 1)] {
            rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
        }
        // slot 719: a two-word part handle per name and kind
        rec(&mut t, 719, 3, |m, a| {
            let at = SCRATCH + 0xb00 + 8 * ((a[1] * 8 + a[2]) % 0x40);
            m.w32(at, 0x7190_0000 ^ a[0] ^ (a[2] << 4));
            m.w32(at + 4, 0x7191_0000 ^ (a[1] << 8) ^ a[2]);
            at
        });
        // three_parts_draw's slots: handles (edx too), matrices, draws
        rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[0] ^ (a[1] << 4));
        t.stub_edx(772, |_, a| 0x0773_0000 ^ a[1]);
        filler(&mut t, 723, 1, 16);
        filler(&mut t, 724, 1, 16);
        // slot 231 reads both matrices (the drawn model's `N` among them)
        rec(&mut t, 231, 3, |m, a| {
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * (15 - i));
                wf(m, a[0] + 4 * i, v);
            }
            0x231
        });
        rec(&mut t, 867, 17, |_, a| 0x0867_0000 ^ a[16] ^ (a[4] << 8));
        rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
        let s = SCRATCH + 0x1_0000;
        let take = t.img.rel32_target(f + 0x6e4);
        let recs = SCRATCH + 0x9_0000;
        let hp = SCRATCH + 0xc00;
        for o in [0x70fu32, 0x959] {
            t.set_u32(w(&t, o), hp);
        }
        t.set_u32(hp, 0x0c00_1234);
        t.set_u32(hp + 4, 0x0c00_5678);
        t.set_u32(w(&t, 0x4a), 0x0239_0000);
        // (countdown, pool size, taken, search start)
        let cases: [(i32, u32, &[u32], u32); 4] = [
            (3, 8, &[1, 6], 5),
            (1, 3, &[0, 1, 2], 0),
            (0, 2, &[0], 0),
            (-2, 6, &[], 4),
        ];
        for (k, (count, pn, busy, from)) in (0u32..).zip(cases) {
            t.set_u32(w(&t, 0x1d), s);
            t.write(s, &magic_h6::varied(0x6000, k as u8));
            for i in 0..16u32 {
                t.set_u32(s + 0x9f0 + 4 * i, (0.5 * i as f32 - 1.25).to_bits());
            }
            t.set_u32(s + cnt, count.cast_unsigned());
            for i in 0..100u32 {
                let r = s + 0xa00 + 0xc8 * i;
                let v = i + 7 * k;
                // the floats: small values; a few angles past the bounds
                for j in 0..0x2cu32 {
                    let x = ((v * 13 + j * 7) % 41) as f32 / 8.0 - 2.5;
                    t.set_u32(r + 4 * j, x.to_bits());
                }
                if v % 7 == 3 {
                    t.set_u32(r + 0x10, 3.1f32.to_bits());
                    t.set_u32(r + 0x40, 0.25f32.to_bits());
                    t.set_u32(r + 0x64, (-3.0f32).to_bits());
                    t.set_u32(r + 0x74, (-0.5f32).to_bits());
                    t.set_u32(r + 0x90, 0.9f32.to_bits());
                    t.set_u32(r + 0x98, 0.25f32.to_bits());
                }
                t.set_u32(r + 0xa0, [5u32, 9, 1, 3][(v % 4) as usize]);
                t.set_u32(r + 0xa4, [0u32, 1, 4, 2, 7][(v % 5) as usize]);
                t.set_u32(r + 0xa8, [0u32, 0, 2][(v % 3) as usize]);
                t.set_u32(r + 0xac, [0u32, 0, 0, 1, 0, 3][(v % 6) as usize]);
                t.set_u32(r + 0xb0, [16u32, 30, 64][(v % 3) as usize]);
                t.set_u32(r + 0xb4, [0u32, 5, 61, 40, 51, 60][(v % 6) as usize]);
                t.set_u32(r + 0xb8, 0x40 + v);
                t.set_u32(r + 0xbc, (v * 3).wrapping_sub(20));
                t.set_u32(r + 0xc0, v % 4);
                t.set_u32(r + 0xc4, 0x0c40_0000 + v);
            }
            pool_74(&mut t, take, recs, pn, busy, from, k);
            magic_h6::scrub(&mut t);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `magic_0512` 0x10008fc0 (`r33::effect_task_0512`, r16's draft): every step, the wait running and
/// down, one node and two, the end flags.
#[test]
fn effect_task_0512() {
    let Some(mut t) = twin(512) else { return };
    all_stubs(&mut t);
    let at = t.at(0x8fc0);
    let pnw = t.img.rel32_target(at + 0xa7);
    let en = t.img.rel32_target(at + 0x79);
    let table = SCRATCH + 0x300;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x400;
    t.set_u32(t.img.word(pnw + 0x0d), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    for (slot, n) in [(714u32, 5), (325, 2), (239, 3)] {
        rec(&mut t, slot, n, move |_, a| (slot << 16) ^ a[0]);
    }
    for list in [t.img.word(pnw + 0x26), t.img.word(en + 0x18)] {
        t.set_u32(list + 0x10, list);
    }
    t.set_u32(t.img.word(at + 0x11d), 0x5a5a_5aa7);
    t.set_u32(t.img.word(at + 0x12b), 0x00a0_012b);
    let (task, r, node, prev, next) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x240,
        SCRATCH + 0x280,
    );
    for (state, wait, two, flags, d) in [
        (0u32, 0u32, 0u32, 0u8, 0.0f32),
        (0, 0, 1, 0, 0.0),
        (1, 3, 0, 0, 0.0),
        (1, 0, 0, 0, 0.0),
        (1, 0xffff_ffff, 1, 0, 0.5),
        (1, 0, 1, 0, 1.0e6),
        (1, 0, 1, 0, -3.0),
        (2, 0, 0, 0, 0.0),
        (2, 0, 0, 1, 0.0),
        (2, 0, 1, 1, 0.0),
        (2, 0, 1, 3, 0.0),
        (2, 0, 0, 2, 0.0),
        (0x2710, 0, 0, 0, 0.0),
        (7, 0, 0, 0, 0.0),
    ] {
        t.write(r, &[0x3c; 0x600]);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x1c, two);
        t.set_u32(r + 0x20, wait);
        t.set_u8(r + 0x488, (flags & 1) << 2);
        t.set_u8(r + 0x588, (flags & 2) << 1);
        t.set_u32(r + 0x2dc, (2.0 * d).to_bits());
        t.set_u32(r + 0x334, d.to_bits());
        let call = Call {
            ecx: 0x00ec_0000 + state,
            ..Call::default()
        };
        t.check_entry(at, call, &[task], Eax::Checked);
    }
}

/// The DLL globals the function at `f` and its callees name (relocated
/// operands outside the code) that hold 0 get blocks of their own whose
/// every word points at the block: an empty list ring, a node with itself
/// as neighbours (as `magic_pf_throws.rs`).
fn prime(t: &mut Twin, f: u32, mut next: u32) {
    let mut todo = vec![(f, 0)];
    let mut seen = Vec::new();
    while let Some((at, depth)) = todo.pop() {
        if seen.contains(&at) {
            continue;
        }
        seen.push(at);
        let Some(len) = ffx_magic::INTERNAL
            .iter()
            .chain(ffx_magic::PORTS)
            .find(|(_, sig)| t.img.matches(at, sig))
            .map(|(_, sig)| sig.len)
        else {
            continue;
        };
        for v in t.img.operands(at, len) {
            if t.img.in_text(v)
                || !(t.img.base..t.img.base + 0x0100_0000).contains(&v)
                || t.u32(v) != 0
            {
                continue;
            }
            for i in 0..0x40 {
                t.set_u32(next + 4 * i, next);
            }
            t.set_u32(v, next);
            next += 0x100;
        }
        if depth < 3 {
            for p in at..at + len.saturating_sub(4) {
                if t.img.word(p) as u8 == 0xe8 {
                    let c = t.img.rel32_target(p + 1);
                    if t.img.in_text(c) {
                        todo.push((c, depth + 1));
                    }
                }
            }
        }
    }
}

/// `magic_0512` 0x10009620 (`r33::summon_task_0512`): state 0 (the whole
/// set-up, then state 10 in the same call) with the model taking the
/// direct way and not; 10; 11 waiting and on into 0x1e; 0x1e (both
/// ways); 0x1f waiting and on through 0x3c and 0x3d into 10000 (the end
/// flags clear and set); 0x3c; 0x3d waiting; 10000 both ways; a table
/// state without code (5), one above the table (0x40).
#[test]
fn summon_task_0512() {
    let Some(mut t) = twin(512) else { return };
    all_stubs(&mut t);
    float_matrix_stubs(&mut t);
    vector_stubs(&mut t);
    summon_stubs(&mut t);
    let f = t.at(0x9620);
    rec(&mut t, 218, 2, |m, a| {
        m.w32(a[1], 2);
        SCRATCH + 0x700
    });
    t.write(SCRATCH + 0x700, &[3, 1, 4, 1, 5, 2, 6, 0]);
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 * i + 7);
        }
        0x558
    });
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            wf(m, a[2] + 4 * i, 0.5 * i as f32 + (a[1] & 3) as f32);
        }
        0x268
    });
    for (s, x) in [(251u32, 14.5f64), (333, 0.75), (12, -1.5)] {
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    // slot 790 takes no words
    t.stub_st0(790, |_, _| 0.375);
    // the direct way: slot 390's model outside the table, slot 534's
    // answer from a word the cases set
    rec(&mut t, 390, 1, |_, _| 0x2000);
    let dw = SCRATCH + 0x7f0;
    rec(&mut t, 534, 1, move |m, _| m.r32(dw));
    rec(&mut t, 719, 3, |_, a| {
        0x0719_0000 ^ a[0] ^ (a[1] << 8) ^ (a[2] << 12)
    });
    prime(&mut t, f, SCRATCH + 0x3_8000);
    let (a, r, node) = (SCRATCH + 0x100, SCRATCH + 0x1_0000, SCRATCH + 0x200);
    let (prev, next) = (SCRATCH + 0x240, SCRATCH + 0x280);
    // (state, the wait, the end flags `R + 0xc50`, the direct way)
    let cases: [(u32, i32, u32, u32); 16] = [
        (0, 0, 0x77, 0),
        (0, 0, 0x77, 1),
        (10, 0, 0x77, 0),
        (11, 3, 0, 0),
        (11, 0, 0, 0),
        (11, -2, 0, 1),
        (0x1e, 0, 0, 1),
        (0x1f, 2, 0, 0),
        (0x1f, 0, 0x4000, 0),
        (0x1f, 1, 0x2710, 0),
        (0x3c, 9, 0, 0),
        (0x3d, 4, 0, 0),
        (0x2710, 0, 0x0400, 0),
        (0x2710, 0, 0x0020, 0),
        (5, 0, 0, 0),
        (0x40, 0, 0, 0),
    ];
    for (k, (state, wait, end, direct)) in (0u32..).zip(cases) {
        for i in 0..0xa00u32 {
            let x = ((i * 7 + k * 3) % 37) as f32 * 0.25 - 4.0;
            t.set_u32(r + 4 * i, x.to_bits());
        }
        t.set_u32(r + 0x54c + 0x90, k & 1);
        t.set_u32(r + 0x78 + 0x90, (k >> 1) & 1);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x50, wait.cast_unsigned());
        t.set_u32(r + 0xc50, end);
        t.set_u32(r + 0x4c, 0x40 + k);
        t.set_u32(dw, direct);
        t.set_u32(a, node);
        t.set_u32(a + 4, r);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.write(prev, &[0x50; 0x10]);
        t.write(next, &[0x4f; 0x10]);
        // the static parents (the summon's, the colour node's), their
        // child lists empty or with a last child
        for (i, op) in (0u32..).zip([0x9eu32, 0x4fd]) {
            let p = t.img.word(f + op);
            let kid = SCRATCH + 0x3000 + 0x40 * i;
            t.write(kid, &[0x21; 0x40]);
            t.set_u32(p + 0x10, if k % 3 == 1 { kid } else { p });
            t.set_u32(p + 0xc, p);
            t.set_u16(p + 0x18, 2 + i as u16);
        }
        ownb::dirty_stack(&mut t, 0x9620 + k, &[]);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}
