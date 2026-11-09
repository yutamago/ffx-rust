//! Round 27's ports (`ffx_magic::r27`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    clippy::type_complexity
)]

mod magic;
mod magic_hi;
mod magic_owna;
mod magic_r11;
#[path = "magic/ownb.rs"]
mod ownb;

use ffx_magic::{Ret, r27};
use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use magic_owna::twin;
use ownb::rec;

/// Junk words at the globals the operands `ops` of `f` name.
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, 0x5a5a_5a5a ^ o);
    }
}

/// `n` bytes of junk at `at` (varied by `k`).
fn junk(t: &mut Twin, at: u32, n: u32, k: u32) {
    let b: Vec<u8> = (0..n)
        .map(|i| (i.wrapping_mul(0x3d) ^ (0x71 + k)).to_le_bytes()[0])
        .collect();
    t.write(at, &b);
}

/// Folds `v` into word `i` of `slot`'s dump row (the outputs a stub writes
/// to the stack, which the comparison skips).
fn fold(m: &mut dyn StubMem, slot: u32, i: u32, v: u32) {
    let at = SCRATCH + 0x2_0000 + 0x40 * (slot % 0x400) + 4 * i;
    let x = m.r32(at).rotate_left(5) ^ v;
    m.w32(at, x);
}

fn rf(m: &dyn StubMem, at: u32) -> f32 {
    f32::from_bits(m.r32(at))
}

fn wf(m: &mut dyn StubMem, at: u32, v: f32) {
    m.w32(at, v.to_bits());
}

/// Every slot answering `first word ^ (slot << 20)` and logging its first
/// word (into `SCRATCH + 0xa00..`), as `magic_o6_fam.rs`'s `stubs`.
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

/// `magic_0609` 0x100093d0 (`r27::motion_task_0609`): every state (the
/// waits answering 0 and not, with the model flag set and not, the model
/// id 0 and not), the unlink, and states the tables send to the end.
#[test]
fn motion_task_0609() {
    let Some(mut t) = twin(609) else { return };
    all_stubs(&mut t);
    let f = t.at(0x93d0);
    rec(&mut t, 20, 1, |m, a| {
        m.r32(SCRATCH + 0x10) ^ (a[0] & 0xff00)
    });
    rec(&mut t, 411, 1, |m, a| {
        m.r32(SCRATCH + 0x10) ^ (a[0] & 0xff00)
    });
    for (s, n) in [(30u32, 1usize), (36, 3), (228, 2)] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    let task = SCRATCH + 0x100;
    let r = SCRATCH + 0x1000;
    let obj = SCRATCH + 0x2000;
    let (node, prev, next) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    // (state, wait answer, model flag)
    let cases: [(u32, u32, u16); 26] = [
        (10, 0, 0),
        (10, 0, 1),
        (11, 0, 0),
        (11, 0, 1),
        (11, 1, 1),
        (11, 0x100, 0),
        (12, 0, 0),
        (20, 0, 1),
        (30, 0, 0),
        (31, 0, 1),
        (31, 1, 0),
        (0x1f, 0, 0),
        (0x20, 0, 0),
        (0x28, 0, 1),
        (0x29, 0, 0),
        (0x32, 0, 0),
        (0x3c, 0, 1),
        (0x3d, 0, 0),
        (0x3d, 0x77, 1),
        (0x3d, 0x77, 0),
        (0x3e, 0, 0),
        (0x2710, 0, 0),
        (5, 0, 0),
        (0x2711, 0, 0),
        (0x3b, 0, 0),
        (9, 0, 0),
    ];
    for (k, (state, wait, flag)) in (0u32..).zip(cases) {
        junk(&mut t, r, 0x40, k);
        junk(&mut t, obj, 0x100, k + 1);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x1c, obj);
        t.set_u16(obj + 0x90, flag);
        t.set_u32(SCRATCH + 0x10, wait);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.write(node, &[0x3c; 0x20]);
        t.set_u32(node + 4, prev);
        t.set_u32(node + 8, next);
        t.write(prev, &[0x4d; 0x20]);
        t.write(next, &[0x5e; 0x20]);
        ownb::dirty_stack(&mut t, 0x93d0 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// The slots `magic_0586`'s caster frames and start-up call, and one flat
/// mesh (one kind-3 group with two triangles) slot 719 answers with.
fn stubs_0586(t: &mut Twin) {
    all_stubs(t);
    rec(t, 225, 1, |m, _| m.r32(SCRATCH + 0x10));
    rec(t, 226, 2, |_, a| 0x226 ^ a[0]);
    rec(t, 227, 1, |m, _| m.r32(SCRATCH + 0x1c));
    rec(t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], 0.5 * i as f32 - 1.0 + (a[0] & 3) as f32);
        }
        0x262
    });
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, |m, _| f64::from(f32::from_bits(m.r32(SCRATCH + 0x24))));
    for (s, x) in [(15u32, 1.5f64), (16, -0.75), (17, 3.0), (277, 40.0)] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    for s in [330u32, 328, 329] {
        rec(t, s, 3, move |m, a| {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i);
                m.w32(a[0] + 4 * i, v ^ (a[2] & 0x00ff_0000) ^ (s << 8));
            }
            s
        });
    }
    // (out, matrix, vector) and (out, in, vector): every word read
    // (the vector's four words reach every output word, as a matrix
    // product's do)
    rec(t, 229, 3, |m, a| {
        let mix = (0..4).fold(0u32, |x, i| x.rotate_left(7) ^ m.r32(a[2] + 4 * i));
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i)
                ^ m.r32(a[1] + 0x30 + 4 * i)
                ^ m.r32(a[2] + 4 * i).rotate_left(i + 1)
                ^ mix.rotate_left(i);
            m.w32(a[0] + 4 * i, v);
        }
        0x229
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i).rotate_left(3);
            m.w32(a[0] + 4 * i, v);
        }
        0x248
    });
    rec(t, 217, 2, |m, a| {
        m.w32(a[1], 0x0217_0000 ^ a[0]);
        0x217
    });
    rec(t, 255, 2, |m, a| m.r32(a[1]) ^ a[0]);
    for (s, n) in [
        (718u32, 1usize),
        (711, 3),
        (710, 4),
        (238, 2),
        (296, 2),
        (257, 2),
        (271, 1),
        (211, 2),
        (210, 2),
        (8, 2),
    ] {
        rec(t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    // the flat coordinates' bounds
    rec(t, 232, 2, |m, a| {
        for i in 0..3u32 {
            let x = f32::from_bits(m.r32(a[0] + 4 * i)) * 0.125;
            m.w32(a[1] + 4 * i, x.to_bits());
        }
        0x232
    });
    rec(t, 564, 3, |_, a| a[2] ^ 0x564);
    let mesh = SCRATCH + 0x1000;
    rec(t, 719, 3, move |_, a| mesh ^ (a[1] & 0x8000_0000));
    let verts = mesh + 0x800;
    let grps = mesh + 0x40;
    for i in 0..32i32 {
        let (x, y, z) = ((i * 37) % 13 - 6, (i * 11 + 3) % 9 - 3, (i * 5 % 17) * -3);
        for (o, v) in [(0u32, x), (2, y), (4, z)] {
            t.set_u16(verts + 6 * i as u32 + o, (v as i16).cast_unsigned());
        }
    }
    t.write(mesh, &[0x4d; 0x40]);
    t.set_u32(mesh + 4, grps - mesh);
    t.set_u32(mesh + 8, verts - mesh);
    t.set_u16(mesh + 0x10, 2);
    t.write(grps, &[0x47; 0x10]);
    t.set_u8(grps + 1, 3);
    t.set_u16(grps + 2, 2);
    for j in 0..2u32 {
        let at = grps + 0x10 + 0x28 * j;
        t.write(at, &[0x52; 0x28]);
        for (o, v) in [(0xcu32, j * 3 + 1), (0xe, j * 5 + 2), (0x10, j * 7 + 4)] {
            t.set_u16(at + o, v as u16);
        }
    }
}

/// `magic_0586` 0x10002170 (`r27::caster_frames_0586`): the turn wrapped
/// both ways, kept and NaN; the caster index below 0x20 (negative too)
/// and not, the battle word set.
#[test]
fn caster_frames_0586() {
    let Some(mut t) = twin(586) else { return };
    stubs_0586(&mut t);
    let f = t.at(0x2170);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let s = SCRATCH + 0x1_0000;
    // (turn source, index, battle word)
    let cases: [(f32, u32, u32); 7] = [
        (0.0, 3, 0),
        (10.0, 0x21, 0),
        (3.0, 2, 1),
        (f32::NAN, 0xffff_fff0, 0),
        (-20.0, 0x20, 0),
        (1.570_796_4, 0x1f, 0),
        // `K - f` exactly pi: the upper bound itself
        (f32::from_bits(0x3fc9_0fda), 3, 0),
    ];
    for (k, (turn, idx, war)) in (0u32..).zip(cases) {
        t.set_u32(SCRATCH + 0x24, turn.to_bits());
        t.set_u32(op(&t, 0x18), s);
        junk(&mut t, s + 0x800, 0x200, k);
        junk(&mut t, s + 0x8560, 0x40, k + 1);
        junk(&mut t, s + 0x9d0, 0x600 * 20, k + 2);
        t.set_u32(op(&t, 0x2ff), idx);
        t.set_u32(op(&t, 0x2f7), war);
        junk_ops(&mut t, f, &[0x2f, 0x1bf, 0x2e6, 0x5ad, 0x5b7, 0x5c1]);
        ownb::dirty_stack(&mut t, 0x2170 + k, &[]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(r27::caster_frames_0586(g, img, fr.at, fr.esp))
        });
    }
}

/// `magic_0586` 0x10001340 (`r27::load_0586`, with 0x100043f0
/// `r27::start_up_0586`): the waits, the start-up with the task list
/// empty and not, the turn wrapped and kept, the caster index below 0x20
/// and not.
#[test]
fn load_0586() {
    let Some(mut t) = twin(586) else { return };
    stubs_0586(&mut t);
    let f = t.at(0x1340);
    let su = t.at(0x43f0);
    let w = |t: &Twin, at: u32, o: u32| t.img.word(at + o);
    let cf = t.at(0x2170);
    let lock = SCRATCH + 0x7f0;
    let pi = t.img.rel32_target(su + 0x60);
    t.set_u32(w(&t, pi, 0x04), lock);
    let take = t.img.rel32_target(su + 0xce);
    let ri = t.img.rel32_target(su + 0x10c);
    let turned = t.img.rel32_target(su + 0x6a);
    // (225, once, 227, task list, turn source, caster index)
    let cases: [(u32, u32, u32, bool, f32, u32); 6] = [
        (1, 0, 1, false, 0.0, 0),
        (0, 0, 0, false, 0.0, 0),
        (0, 0, 1, false, -10.0, 3),
        (0, 1, 2, true, 10.0, 0x21),
        (0, 0, 1, true, 3.0, 2),
        (0, 0, 1, false, 1.0, 0x20),
    ];
    for (k, (w225, once, r227, busy, turn, idx)) in (0u32..).zip(cases) {
        t.set_u32(SCRATCH + 0x10, w225);
        t.set_u32(SCRATCH + 0x1c, r227);
        t.set_u32(SCRATCH + 0x24, turn.to_bits());
        t.set_u32(w(&t, f, 0x18), once);
        t.set_u32(lock, 0x77 + k);
        // the state block the start-up names: junk first
        let sb = w(&t, su, 0x4c);
        junk(&mut t, sb + 0x800, 0x200, k);
        junk(&mut t, sb + 0x8560, 0x40, k + 1);
        t.set_u32(w(&t, cf, 0x2ff), idx);
        t.set_u32(w(&t, cf, 0x2f7), 0);
        t.set_u32(w(&t, turned, 0xa1), k % 2);
        junk_ops(&mut t, su, &[0x16, 0x56, 0x7f, 0xa3]);
        let node = |i: u32| SCRATCH + 0x5000 + 0x40 * i;
        for i in 0..3u32 {
            t.write(node(i), &[0x33; 0x40]);
            t.set_u32(node(i), node((i + 2) % 3));
            t.set_u32(node(i) + 4, node((i + 1) % 3));
        }
        t.set_u32(w(&t, ri, 0x3e), node(0));
        t.set_u16(w(&t, ri, 0x20), 3);
        t.set_u32(w(&t, ri, 0xa6), 0);
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
        let last = SCRATCH + 0x3400;
        t.write(last, &[0x34; 0x20]);
        t.set_u32(w(&t, su, 0xd4), if busy { last } else { 0 });
        t.set_u32(w(&t, su, 0xe8), last);
        ownb::dirty_stack(&mut t, 0x1340 + k, &[]);
        ownb::reset_counters(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x100, "the start-up's calls");
}

/// The slots `magic_0587`'s trails call: 234 the difference of its points,
/// 248 the sum, the matrix slots mixing what they read, slot 941 a frame
/// table, 772 a 64-bit handle; the part link's lock global at a scratch
/// word.
fn stubs_0587(t: &mut Twin, parts: u32) {
    all_stubs(t);
    let table = SCRATCH + 0x3e00;
    t.stub(941, move |m, a| {
        let k = m.r32(SCRATCH + 0x3f80);
        m.w32(SCRATCH + 0x3f84 + 4 * (k % 8), a[0]);
        m.w32(SCRATCH + 0x3f80, k + 1);
        table
    });
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    rec(t, 234, 3, |m, a| {
        for i in 0..3 {
            let x = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x234
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..3 {
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x248
    });
    for s in [723u32, 724] {
        t.stub(s, move |m, a| {
            for i in 0..16 {
                wf(m, a[0] + 4 * i, 0.25 * i as f32 - 1.5 + (s & 1) as f32);
            }
            s
        });
    }
    t.stub(235, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    // (out, matrix, vector): each word read
    t.stub(229, |m, a| {
        for i in 0..4 {
            let mut x = rf(m, a[2] + 4 * i);
            for j in 0..4 {
                x += rf(m, a[1] + 16 * j + 4 * i) * 0.125;
            }
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    rec(t, 237, 4, |m, a| {
        for i in 0..3 {
            wf(m, a[3] + 4 * i, 0.5 * i as f32 + (a[1] & 7) as f32);
        }
        0x237
    });
    // the part link's slots 774/778 hash the part's matrix and colour
    for s in [774u32, 778] {
        t.stub(s, move |m, a| {
            let mut x = s ^ a[2];
            let mat = m.r32(m.r32(SCRATCH + 0x3f40));
            for i in 0..16 {
                x = x.rotate_left(5) ^ m.r32(mat + 4 * i);
            }
            let k = m.r32(SCRATCH + 0x3fc0);
            m.w32(SCRATCH + 0x3fc4 + 4 * (k % 15), x);
            m.w32(SCRATCH + 0x3fc0, k + 1);
            x
        });
    }
    let link = t.img.rel32_target(parts + 0x465);
    for off in [0x4fu32, 0x7a, 0xa9, 0xc6, 0x115, 0x13b] {
        let g = t.img.word(link + off);
        t.set_u32(g, SCRATCH + 0x3f00);
    }
    // the part record's matrix word, for the hash above
    t.set_u32(SCRATCH + 0x3f40, t.img.word(parts + 0x70));
}

/// A trail at `o`: 0x80 points (`zero` repeats the point before it: a
/// zero-length segment), the marks (`gap` and the head unmarked), the
/// colours, the scales and the step, the head and the count, an
/// animation cursor.
fn trail_0587(
    t: &mut Twin,
    o: u32,
    c: u32,
    (count, head, gap, zero, alpha, step): (i32, u32, u32, u32, u8, f32),
) {
    t.write(o, &[0x13; 0x8a8]);
    t.set_u32(o, 0x0041_0000 + c);
    t.set_u32(o + 4, c & 1);
    t.set_u16(o + 8, 2);
    t.set_u16(o + 0xa, 0x100 + 0x80 * c as u16);
    for i in 0..0x80u32 {
        let k = if i + 1 == zero { zero } else { i } as f32;
        // a step of 4.0 walks a straight trail of 2.0-long segments: the
        // distance meets a segment's length exactly
        let xyz = if step.to_bits() == 4.0f32.to_bits() {
            [k * 2.0, 1.0, -3.0, 1.0]
        } else {
            [
                k * 1.5 + c as f32,
                (k * 0.7).sin() * 2.0,
                k * 0.25 - 3.0,
                1.0,
            ]
        };
        for (j, v) in (0u32..).zip(xyz) {
            t.set_u32(o + 0xc + 16 * i + 4 * j, v.to_bits());
        }
        t.set_u8(o + 0x828 + i, u8::from(i != gap && i != head));
    }
    t.write(o + 0x80c, &[0x20 + c as u8, 0x90, 0x10, alpha]);
    // the second alpha differs from the first (the fade reads only that)
    let alpha2 = if alpha == 0 { 0 } else { alpha ^ 0x5a };
    t.write(o + 0x810, &[0xe0, 0x30 + c as u8, 0x70, alpha2]);
    t.set_u32(o + 0x814, 1.5f32.to_bits());
    t.set_u32(o + 0x818, 0.5f32.to_bits());
    t.set_u32(o + 0x81c, step.to_bits());
    t.set_u32(o + 0x820, head);
    t.set_u32(o + 0x824, count.cast_unsigned());
}

/// `magic_0587` 0x10001d40 (`r27::trail_parts_0587`): no parts (0 and
/// negative); a short trail; an unmarked point on the way back; a
/// zero-length segment; the alpha 0 (nothing linked); a long step past
/// 0x80 segments; the head at the ring's last point and at its first; a
/// trail below the block (a negative index); a NaN step (the distance
/// unordered from then on).
#[test]
fn trail_parts_0587() {
    let Some(mut t) = twin(587) else { return };
    let f = t.at(0x1d40);
    stubs_0587(&mut t, f);
    let table = SCRATCH + 0x3e00;
    let block = SCRATCH + 0x8000;
    let (basis, point) = (SCRATCH + 0x4400, SCRATCH + 0x4480);
    let cases: [(i32, u32, u32, u32, u8, f32); 13] = [
        (0, 5, 99, 99, 0x80, 0.6),
        (-3, 0x7f, 99, 99, 0x80, 0.6),
        (6, 5, 99, 99, 0x80, 0.6),
        (12, 7, 3, 99, 0x40, 0.6),
        (9, 10, 99, 8, 0x80, 0.6),
        (5, 3, 99, 99, 0, 0.6),
        (60, 2, 99, 99, 0x7f, 5.0),
        (7, 0x7f, 0x7c, 99, 0x80, 0.9),
        (7, 0, 99, 99, 0x81, 0.9),
        (4, 0x40, 99, 99, 0x80, 0.0),
        (6, 9, 99, 99, 0x80, f32::NAN),
        (8, 20, 99, 99, 0x80, 4.0),
        // more parts than the walk's 0x80 segments
        (0x90, 2, 99, 99, 0x80, 5.0),
    ];
    for (c, case) in (0u32..).zip(cases) {
        // trails at the block's slots 0..2, and one below it
        let o = if c == 9 {
            block + 0xe48 - 0x8a8 * 2 - 0x40
        } else {
            block + 0xe48 + 0x8a8 * (c % 3)
        };
        t.set_u32(t.img.word(f + 0x438), block);
        ownb::fill(&mut t, table, 0x20, 17 + c);
        t.set_u16(table + 6, 5);
        t.write(SCRATCH + 0x3f00, &[0x31; 0x40]);
        trail_0587(&mut t, o, c, case);
        for i in 0..12u32 {
            t.set_u32(
                basis + 4 * i,
                (0.5 + 0.25 * i as f32 - c as f32 * 0.1).to_bits(),
            );
        }
        for (i, v) in [3.0f32 + c as f32, -1.0, 2.0, 1.0].into_iter().enumerate() {
            t.set_u32(point + 4 * i as u32, v.to_bits());
        }
        junk_ops(&mut t, f, &[0x5f, 0x70, 0x20b, 0x233, 0x25b, 0x286]);
        ownb::dirty_stack(&mut t, 0x1d40 + c, &[]);
        let regs = Call {
            ecx: o,
            edx: basis,
            ..Call::default()
        };
        t.check(f, regs, &[point], Eax::Checked, |g, img, fr| {
            Ret::eax(r27::trail_parts_0587(g, img, fr.at, fr))
        });
    }
}

/// `magic_0587` 0x10002440 (`r27::trails_task_0587`): the four trails
/// with the stop word set and not, the alpha bytes 0, 1, 2 and 0x80.
#[test]
fn trails_task_0587() {
    let Some(mut t) = twin(587) else { return };
    let f = t.at(0x2440);
    let parts = t.at(0x1d40);
    stubs_0587(&mut t, parts);
    let table = SCRATCH + 0x3e00;
    let block = SCRATCH + 0x8000;
    for c in 0..2u32 {
        t.set_u32(t.img.word(f + 0x17), block);
        t.set_u32(t.img.word(parts + 0x438), block);
        ownb::fill(&mut t, table, 0x20, 23 + c);
        t.set_u16(table + 6, 5);
        t.write(SCRATCH + 0x3f00, &[0x31; 0x40]);
        t.set_u32(block + 0x30fc, c);
        for (i, alpha) in (0u32..).zip([0u8, 1, 2, 0x80]) {
            let o = block + 0xe48 + 0x8a8 * i;
            trail_0587(
                &mut t,
                o,
                c + i,
                (3 + i.cast_signed(), 5 + i, 99, 99, alpha, 0.6),
            );
        }
        junk_ops(&mut t, f, &[0x6b]);
        junk_ops(&mut t, parts, &[0x5f, 0x70, 0x20b, 0x233, 0x25b, 0x286]);
        ownb::dirty_stack(&mut t, 0x2440 + c, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0649` 0x10007da0 (`r27::light_task_0649`): the start with slot
/// 390 picking either model node, the id found in slot 288's list, not
/// found and an empty list, each group's alpha from slot 333 and from slot
/// 601, the static lists empty and not; the waits running and out in
/// states 1, 11, 12 and 13; state 10; the end with the objects' words
/// sharing a bit of 0x4e20 and not; states outside the machine.
#[test]
fn light_task_0649() {
    let Some(mut t) = twin(649) else { return };
    all_stubs(&mut t);
    let f = t.at(0x7da0);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let fs = t.img.rel32_target(f + 0xc1a);
    let lo = t.img.rel32_target(f + 0xbd4);
    let ops = |t: &Twin, at: u32, o: u32| t.img.word(at + o);
    // slot 390: the answer for a byte from the table at `ans`
    let ans = SCRATCH + 0x7c0;
    rec(&mut t, 390, 1, move |m, a| m.r32(ans + 4 * (a[0] & 7)));
    let ids = SCRATCH + 0x7e0;
    let cnt = SCRATCH + 0x7f0;
    rec(&mut t, 288, 2, move |m, a| {
        let n = m.r32(cnt);
        m.w32(a[1], n);
        ids
    });
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], 0.5 * i as f32 - 1.0 + (a[0] & 3) as f32);
        }
        0x262
    });
    for (s, x) in [
        (333u32, -0.75f64),
        (251, 0.625),
        (15, 0.5),
        (16, 0.75),
        (17, 1.5),
        (277, 2.0),
    ] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    rec(&mut t, 601, 2, |m, a| {
        wf(m, a[1], 1.75 + (a[0] & 1) as f32);
        0x601
    });
    rec(&mut t, 290, 2, |m, a| {
        for i in 0..3 {
            wf(m, a[1] + 4 * i, 0.25 * i as f32 + 3.0);
        }
        0x290
    });
    for (s, n) in [
        (234u32, 3usize),
        (719, 3),
        (676, 3),
        (678, 1),
        (941, 1),
        (413, 1),
        (215, 1),
        (14, 2),
        (213, 2),
        (211, 2),
        (357, 3),
        (356, 6),
        (362, 4),
        (704, 4),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    let level = SCRATCH + 0x90;
    rec(&mut t, 363, 3, move |m, _| m.r32(level));
    // the colour node's helper keeps slot 790's float; slot 558 writes 4
    rec(&mut t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |_, _| 0.375);
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 + 0x11 * i);
        }
        0x558
    });
    let list = op(&t, 0x194);
    let list2 = op(&t, 0x66);
    let old = SCRATCH + 0x800;
    let ring_end = SCRATCH + 0x880;
    let task = SCRATCH + 0x100;
    let (tn, tprev, tnext) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let r = SCRATCH + 0x1_0000;
    let model = SCRATCH + 0x3000;
    let ob = SCRATCH + 0x7a00;
    let ls = SCRATCH + 0x700;
    // (state, count, slot 390's answer for the id source, the list's
    // count and where the id is (99: nowhere), the groups' flag bits,
    // the end words share 0x4e20)
    let cases: [(u32, u32, u32, u32, u32, u8, bool); 20] = [
        (0, 0, 0x10ad, 4, 2, 0b000, false),
        (0, 0, 0x10ad, 4, 99, 0b111, false),
        (0, 0, 0x10ae, 3, 0, 0b101, false),
        (0, 0, 0x1234, 0, 99, 0b010, false),
        (1, 3, 0, 0, 0, 0, false),
        (1, 0, 0, 0, 0, 0, false),
        (1, 0, 0, 0, 0, 0, true),
        (5, 0, 0, 0, 0, 0, false),
        (10, 4, 0, 0, 0, 0, false),
        (11, 2, 0, 0, 0, 0, false),
        (11, 0, 0, 0, 0, 0b001, false),
        (11, 0, 0, 0, 0, 0b110, true),
        (12, 5, 0, 0, 0, 0, false),
        (12, 0, 0, 0, 0, 0, false),
        (13, 1, 0, 0, 0, 0, false),
        (13, 0, 0, 0, 0, 0, false),
        (13, 0, 0, 0, 0, 0, true),
        (0x2710, 0, 0, 0, 0, 0, true),
        (0x2710, 0, 0, 0, 0, 0, false),
        (0x4e20, 0, 0, 0, 0, 0, false),
    ];
    for (k, (state, count, a0, n, at, flags, share)) in (0u32..).zip(cases) {
        junk(&mut t, r, 0x9d00, k);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x20, count);
        for (i, o) in [0xb4u32, 0x14c, 0x1e4].into_iter().enumerate() {
            t.set_u8(r + o, (flags >> i) & 1);
        }
        // the model nodes have no parent; their objects' models
        for n in [0x24u32, 0xbc, 0x154] {
            t.set_u32(r + n + 0x14, 0);
            t.set_u32(r + n + 0x54, model);
        }
        junk(&mut t, model, 0x100, k + 1);
        t.set_u32(model + 0x28, (2.0 + k as f32).to_bits());
        t.set_u16(model + 0x90, (k & 1) as u16);
        // the end words: all four share 0x4e20, or one has none of it
        let (x, y) = if share {
            (0x4e20, 0xffff)
        } else {
            (0x4e20, 0xb1df)
        };
        for (o, v) in [
            (0x5798u32, x),
            (0x13c8, y),
            (0xdf8, 0xffff),
            (0x828, 0xffff),
        ] {
            t.set_u32(r + o, v);
        }
        // slot 390: the id source answers `a0`, the list's bytes 1..
        t.set_u32(op(&t, 0x177), 0);
        t.set_u32(ans, a0);
        let id = if a0 == 0x10ad { 0x10ae } else { 0x10ad };
        for i in 1..8u32 {
            t.set_u32(ans + 4 * i, if i == at + 1 { id } else { 0x1000 + i });
        }
        t.write(ids, &[1, 2, 3, 4, 5, 6, 7, 0]);
        t.set_u32(cnt, n);
        // the static lists: empty, or with a last node
        for (l, i) in [(list, 0u32), (list2, 1)] {
            junk(&mut t, l, 0x20, k + 2 + i);
            t.set_u32(l + 0x10, if (k + i) % 2 == 0 { l } else { old });
            t.set_u16(l + 0x18, 0x0305);
        }
        t.write(old, &[0x6c; 0x40]);
        // the task ring's last and flags
        t.write(ring_end, &[0x5d; 0x40]);
        t.set_u32(op(&t, 0xa8a), ring_end);
        t.set_u32(op(&t, 0xaaa), 0x0101_0101 + k);
        // the fade set-up's other lists and its particle owner
        for (l, i) in [(ops(&t, fs, 0x2a0), 2u32), (ops(&t, fs, 0x3de), 3)] {
            junk(&mut t, l, 0x20, k + 2 + i);
            t.set_u32(l + 0x10, if (k + i) % 2 == 0 { l } else { old });
            t.set_u16(l + 0x18, 0x0305);
        }
        t.set_u32(ops(&t, fs, 0x27d), ob);
        junk(&mut t, ob, 0x40, k + 6);
        t.set_u32(ob + 0x10, ob);
        // the light object: its level answer, scale word and parent list
        t.set_u32(level, if k % 2 == 0 { 0x7_0000 } else { 5 });
        t.set_u32(ops(&t, lo, 0x88), ls);
        t.set_u32(ls, 0x0055_0000 + k);
        let parent = ops(&t, lo, 0x6a);
        t.set_u32(parent + 0x10, parent);
        t.set_u32(parent + 0xc, parent);
        // (`+0x18c` names the id source again: it stays 0)
        junk_ops(&mut t, f, &[0x1ea]);
        junk(&mut t, task, 0x10, k + 3);
        t.set_u32(task, tn);
        t.set_u32(task + 4, r);
        t.write(tn, &[0x3c; 0x20]);
        t.set_u32(tn + 4, tprev);
        t.set_u32(tn + 8, tnext);
        t.write(tprev, &[0x4d; 0x20]);
        t.write(tnext, &[0x5e; 0x20]);
        ownb::dirty_stack(&mut t, 0x7da0 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x200, "the task's calls");
}

/// The globals `start_object_plain` at `so` reads (as
/// `magic_s4_misc.rs`'s `start_object_at`): its item and model words,
/// slot 941's part table, the pool (`count` free nodes) and the object
/// list's tail `tl`.
fn start_env(t: &mut Twin, so: u32, count: u16, tl: u32) {
    start_env_at(t, so, (0x96, 0xa1), count, tl);
}

/// `magic_0586` 0x10002ce0 and `magic_0162` 0x100030d0
/// (`r27::point_object_0586`): the pool with nodes and empty, the list's
/// tail set and not, the selected actors varied.
#[test]
fn point_object_0586() {
    for (n, rva) in [(586u16, 0x2ce0u32), (162, 0x30d0)] {
        let Some(mut t) = twin(n) else { return };
        stubs_0586(&mut t);
        let f = t.at(rva);
        let op = |t: &Twin, o: u32| t.img.word(f + o);
        let so = t.img.rel32_target(f + 0x2f);
        let s = SCRATCH + 0x1_0000;
        let sel = SCRATCH + 0x600;
        for (k, (count, tl, who)) in [(3u16, 0u32, 1u8), (0, SCRATCH + 0x680, 4), (2, 0, 0x31)]
            .into_iter()
            .enumerate()
        {
            let k = k as u32;
            start_env(&mut t, so, count, tl);
            t.set_u32(op(&t, 0x1d), s);
            junk(&mut t, s, 0x8600, k);
            for i in 0..0x60u32 {
                t.set_u32(
                    s + 0x8c0 + 4 * i,
                    (0.25 * i as f32 - 3.0 + k as f32).to_bits(),
                );
            }
            t.set_u32(op(&t, 0xcd), sel);
            t.write(sel, &[7, who, 9, 9]);
            for i in 0..0x40u32 {
                t.set_u32(op(&t, 0xd8) + 4 * i, 0x00ac_0000 + 0x101 * i + k);
            }
            junk_ops(&mut t, f, &[0xf9]);
            ownb::dirty_stack(&mut t, rva + k, &[]);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `magic_0586` 0x10002fd0 and `magic_0162` 0x100033c0
/// (`r27::glow_effects_0586`): each fade state with its count running and
/// at its end, a state past them, the spin wrapped both ways and kept,
/// the effect lookups failing and found, `*G_OFF` set and not.
#[test]
fn glow_effects_0586() {
    for (n, rva) in [(586u16, 0x2fd0u32), (162, 0x33c0)] {
        let Some(mut t) = twin(n) else { return };
        stubs_0586(&mut t);
        let f = t.at(rva);
        let op = |t: &Twin, o: u32| t.img.word(f + o);
        let mep = t.img.rel32_target(f + 0x30b);
        let s = SCRATCH + 0x1_0000;
        let sel = SCRATCH + 0x600;
        let lock = SCRATCH + 0x5f80;
        t.set_u32(t.img.word(mep + 0x60), lock);
        t.set_u32(t.img.word(mep + 0xd5), lock);
        let key = SCRATCH + 0x5e00;
        let cam = SCRATCH + 0x5e40;
        let found = SCRATCH + 0x5f90;
        rec(&mut t, 719, 3, move |_, _| key);
        rec(&mut t, 885, 7, move |m, a| {
            for i in 0..8u32 {
                m.w32(a[6] + 4 * i, a[0] ^ (i << 8));
            }
            m.r32(found)
        });
        rec(&mut t, 545, 0, move |_, _| cam);
        rec(&mut t, 230, 2, |m, a| {
            for i in 0..16u32 {
                m.w32(a[0] + 4 * i, m.r32(a[1] + 4 * i));
            }
            0x230
        });
        rec(&mut t, 231, 3, |m, a| {
            for i in 0..16u32 {
                m.w32(a[0] + 4 * i, m.r32(a[1] + 4 * i) ^ a[2] ^ i);
            }
            0x231
        });
        rec(&mut t, 912, 12, |_, a| a[0] ^ 0x912);
        rec(&mut t, 833, 3, |_, a| a[2] ^ 0x833);
        rec(&mut t, 544, 2, |m, a| {
            for i in 0..4 {
                wf(m, a[1] + 4 * i, 0.5 * i as f32 + a[0] as f32 - 1.0);
            }
            0x544
        });
        for (s_, n_) in [
            (553u32, 5usize),
            (357, 3),
            (356, 3),
            (355, 2),
            (354, 2),
            (938, 1),
            (922, 2),
            (931, 2),
            (776, 1),
            (834, 1),
            (723, 1),
            (724, 1),
        ] {
            rec(&mut t, s_, n_, move |_, a| {
                (s_ << 16) ^ a.first().copied().unwrap_or(0)
            });
        }
        for i in 0..16 {
            t.set_u32(cam + 4 * i, (0.125 * i as f32 - 0.5).to_bits());
        }
        let (caster, actors) = (SCRATCH + 0x7000, SCRATCH + 0x7400);
        let d = |t: &Twin, o: u32| t.img.word(f + o);
        let (ry, st, cnt) = (d(&t, 0x2b), d(&t, 0x111), d(&t, 0x12d));
        // (state, count, spin, lookups found, *G_OFF)
        let cases: [(u32, u32, f32, u32, u32); 10] = [
            (0, 1, 0.0, 1, 0),
            (0, 5, 3.1, 0, 0),
            (1, 1, -3.1, 1, 1),
            (1, 3, 1.0, 1, 0),
            (2, 1, 0.5, 0, 0),
            (2, 10, 2.0, 1, 0),
            (2, 0x40, -1.0, 1, 0),
            (3, 7, 0.25, 1, 1),
            (0, 0xa5, f32::NAN, 1, 0),
            (7, 0, 0.0, 0, 0),
        ];
        for (k, (state, count, spin, fnd, off)) in (0u32..).zip(cases) {
            t.set_u32(op(&t, 0x1f), s);
            junk(&mut t, s, 0x8600, k);
            for i in 0..0x60u32 {
                t.set_u32(
                    s + 0x8c0 + 4 * i,
                    (0.25 * i as f32 - 3.0 + k as f32).to_bits(),
                );
            }
            t.set_u32(s + ry, spin.to_bits());
            t.set_u32(s + st, state);
            t.set_u32(s + cnt, count);
            t.set_u32(found, fnd);
            t.set_u32(lock, 0x1100 + k);
            t.set_u32(key, 0x1234_5678 ^ k);
            t.set_u32(key + 4, 0x9abc_def0u32.rotate_left(k));
            for o in [caster, actors] {
                junk(&mut t, o, 0x840, k + 4);
                t.set_u32(o + 0x830, SCRATCH + 0x7c00);
            }
            t.set_u32(op(&t, 0x21a), caster);
            t.set_u32(op(&t, 0x310), sel);
            t.write(sel, &[7, (k % 3) as u8, 9, 9]);
            for i in 0..4u32 {
                t.set_u32(op(&t, 0x323) + 4 * i, actors);
            }
            t.set_u32(op(&t, 0x426), off);
            ownb::dirty_stack(&mut t, rva + k, &[]);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `magic_0586` 0x10003780 and `magic_0162` 0x10003b70
/// (`r27::models_place_0586`): the object found in the list (first of two)
/// and not, slot 324's bit set and clear, the turns wrapped both ways and
/// kept, the task list empty and not.
#[test]
fn models_place_0586() {
    for (n, rva) in [(586u16, 0x3780u32), (162, 0x3b70)] {
        let Some(mut t) = twin(n) else { return };
        stubs_0586(&mut t);
        let f = t.at(rva);
        let op = |t: &Twin, o: u32| t.img.word(f + o);
        let answer = SCRATCH + 0x5f90;
        rec(&mut t, 324, 0, move |m, _| m.r32(answer));
        for (s_, n_) in [(713u32, 1usize), (371, 2)] {
            rec(&mut t, s_, n_, move |_, a| (s_ << 16) ^ a[0]);
        }
        let take = t.img.rel32_target(f + 0x55a);
        let s = SCRATCH + 0x1_0000;
        let sel = SCRATCH + 0x600;
        let objs = op(&t, 0x21);
        let (n1, n2) = (SCRATCH + 0x6100, SCRATCH + 0x6140);
        let last = SCRATCH + 0x3400;
        // (in the list, slot 324, turn y, task list busy)
        let cases: [(bool, u32, f32, bool); 6] = [
            (true, 1, 0.0, false),
            (false, 0, 1.0, true),
            (true, 3, 7.5, true),
            (false, 1, -7.5, false),
            (true, 0x10, 2.5, false),
            (false, 5, f32::NAN, true),
        ];
        for (k, (found, bit, ry, busy)) in (0u32..).zip(cases) {
            t.set_u32(op(&t, 0x13), s);
            junk(&mut t, s, 0x8600, k);
            for i in 0..0x60u32 {
                t.set_u32(
                    s + 0x8c0 + 4 * i,
                    (0.25 * i as f32 - 3.0 + k as f32).to_bits(),
                );
            }
            let obj = s + op(&t, 0x1a);
            t.set_u32(s + op(&t, 0x1e8), ry.to_bits());
            t.set_u32(answer, bit);
            // the list: [n1, n2] with n1 holding the object, or empty
            t.write(objs, &[0; 0x10]);
            t.write(n1, &[0x31; 0x40]);
            t.write(n2, &[0x32; 0x40]);
            if found {
                t.set_u32(objs, n1);
                t.set_u32(objs + 4, n2);
                t.set_u32(objs + 8, n2);
                t.set_u32(objs + 0xc, 2);
                t.set_u32(n1, 0);
                t.set_u32(n1 + 4, n2);
                t.set_u32(n1 + 8, obj);
                t.set_u32(n2, n1);
                t.set_u32(n2 + 4, 0);
            }
            t.set_u32(op(&t, 0x2f), SCRATCH + 0x5fa0);
            t.set_u32(op(&t, 0x247), SCRATCH + 0x7000 + k);
            t.set_u32(op(&t, 0x278), SCRATCH + 0x7100 + k);
            t.set_u32(op(&t, 0x335), sel);
            t.set_u32(op(&t, 0x370), sel);
            t.write(sel, &[7, (k % 3) as u8, 9, 9]);
            for i in 0..4u32 {
                t.set_u32(op(&t, 0x340) + 4 * i, 0x00ac_0000 + 0x10 * i + k);
                t.set_u32(op(&t, 0x37b) + 4 * i, 0x00ad_0000 + 0x10 * i + k);
            }
            magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
            t.write(last, &[0x34; 0x20]);
            t.set_u32(op(&t, 0x560), if busy { last } else { 0 });
            t.set_u32(op(&t, 0x570), last);
            junk_ops(&mut t, f, &[0x59c]);
            ownb::dirty_stack(&mut t, rva + k, &[]);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `magic_0586` 0x10003d60 and `magic_0162` 0x10004150
/// (`r27::point_objects_0586`): the caster's matrix on and off, the turns
/// wrapped both ways and kept, the pool with nodes and empty, the task
/// list empty and not.
#[test]
fn point_objects_0586() {
    for (n, rva) in [(586u16, 0x3d60u32), (162, 0x4150)] {
        let Some(mut t) = twin(n) else { return };
        stubs_0586(&mut t);
        let f = t.at(rva);
        let op = |t: &Twin, o: u32| t.img.word(f + o);
        for (s_, n_) in [(399u32, 2usize), (371, 2)] {
            rec(&mut t, s_, n_, move |_, a| (s_ << 16) ^ a[0] ^ a[1]);
        }
        let so = t.img.rel32_target(f + 0x2e);
        let take = t.img.rel32_target(f + 0x5c3);
        let s = SCRATCH + 0x1_0000;
        let sel = SCRATCH + 0x600;
        let last = SCRATCH + 0x3400;
        // (S.on, the turn S.a, the pool's free count, the task list busy)
        let cases: [(u32, f32, u16, bool); 6] = [
            (1, 0.0, 3, false),
            (0, 3.0, 0, true),
            (1, -3.0, 2, true),
            (2, 1.0, 1, false),
            (1, -1.5, 3, false),
            (0, 1.5, 2, true),
        ];
        for (k, (on, turn, count, busy)) in (0u32..).zip(cases) {
            start_env(&mut t, so, count, if busy { SCRATCH + 0x680 } else { 0 });
            t.set_u32(op(&t, 0x1b), s);
            junk(&mut t, s, 0x8600, k);
            for i in 0..0x60u32 {
                t.set_u32(
                    s + 0x8c0 + 4 * i,
                    (0.25 * i as f32 - 3.0 + k as f32).to_bits(),
                );
            }
            t.set_u32(s + op(&t, 0x7c), turn.to_bits());
            t.set_u32(s + op(&t, 0x1f5), on);
            t.set_u32(op(&t, 0x21b), SCRATCH + 0x7000 + k);
            t.set_u32(op(&t, 0x24c), SCRATCH + 0x7100 + k);
            for o in [0x331u32, 0x417, 0x452, 0x544] {
                t.set_u32(op(&t, o), sel);
            }
            t.write(sel, &[7, 9, (k % 3) as u8, 9]);
            for o in [0x342u32, 0x422, 0x45d, 0x551] {
                for i in 0..4u32 {
                    t.set_u32(op(&t, o) + 4 * i, 0x00ac_0000 + 0x10 * i + o + k);
                }
            }
            magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
            t.write(last, &[0x34; 0x20]);
            t.set_u32(op(&t, 0x5c9), if busy { last } else { 0 });
            t.set_u32(op(&t, 0x5dd), last);
            junk_ops(&mut t, f, &[0x363]);
            ownb::dirty_stack(&mut t, rva + k, &[]);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `magic_0587` 0x10003770 (`r27::spin_step_0587`): the count running
/// and running out (a new aim), past 0, the spin wrapped both ways and
/// kept.
#[test]
fn spin_step_0587() {
    let Some(mut t) = twin(587) else { return };
    stubs_0586(&mut t);
    for (s, x) in [(314u32, 0.375f64), (404, 2.5)] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 0x30_0000) / 1_048_576.0);
    }
    rec(&mut t, 371, 2, |_, a| 0x0371_0000 ^ a[0] ^ a[1]);
    // slot 229 as a matrix product (floats: the sums the port adds)
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let mut x = 0.0f32;
            for j in 0..4 {
                x += rf(m, a[1] + 16 * j + 4 * i) * rf(m, a[2] + 4 * j);
            }
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    let f = t.at(0x3770);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let s = SCRATCH + 0x8000;
    // (count, spin)
    let cases: [(u32, f32); 6] = [
        (1, 0.0),
        (5, 3.1),
        (0, -3.1),
        (1, 2.0),
        (0x28, -1.0),
        (2, f32::NAN),
    ];
    for (k, (count, spin)) in (0u32..).zip(cases) {
        t.set_u32(op(&t, 0x13), s);
        junk(&mut t, s, 0x3100, k);
        for i in 0..0xc0u32 {
            t.set_u32(
                s + 0x7c0 + 4 * i,
                (0.25 * i as f32 - 3.0 + k as f32).to_bits(),
            );
        }
        t.set_u32(s + 0xa44, spin.to_bits());
        t.set_u32(s + 0x30f4, count);
        ownb::dirty_stack(&mut t, 0x3770 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// [`start_env`] for a start whose pool call and list tail sit at other
/// operands (`start_object_anim`: `+0x99`, `+0xa4`).
fn start_env_at(t: &mut Twin, so: u32, (pool_op, tail_op): (u32, u32), count: u16, tl: u32) {
    t.set_u32(t.img.word(so + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(so + 0x5e), SCRATCH + 0x300);
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
    let pa = t.img.rel32_target(so + pool_op);
    let free = t.img.word(pa + 0x04);
    let nodes = t.img.word(pa + 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    t.set_u16(free, count);
    t.set_u32(t.img.word(so + tail_op), tl);
}

/// `magic_0587` 0x10003b20 (`r27::second_start_0587`): the two objects
/// in the list and not, the object pool with nodes and empty, the task
/// list empty and not.
#[test]
fn second_start_0587() {
    let Some(mut t) = twin(587) else { return };
    stubs_0586(&mut t);
    for (s_, n_) in [(371u32, 2usize), (713, 1), (560, 1)] {
        rec(&mut t, s_, n_, move |_, a| (s_ << 16) ^ a[0]);
    }
    let r557 = SCRATCH + 0x5f90;
    rec(&mut t, 557, 0, move |m, _| m.r32(r557));
    let f = t.at(0x3b20);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let so = t.img.rel32_target(f + 0xeb);
    let take = t.img.rel32_target(f + 0x1eb);
    let s = SCRATCH + 0x8000;
    let objs = op(&t, 0x3d);
    let (n1, n2, n3) = (SCRATCH + 0x6100, SCRATCH + 0x6140, SCRATCH + 0x6180);
    let last = SCRATCH + 0x3400;
    // (which objects are in the list, the pool's count, the task list busy)
    let cases: [(u8, u16, bool); 4] = [(3, 3, false), (0, 0, true), (1, 2, true), (2, 1, false)];
    for (k, (inl, count, busy)) in (0u32..).zip(cases) {
        start_env_at(
            &mut t,
            so,
            (0x99, 0xa4),
            count,
            if busy { SCRATCH + 0x680 } else { 0 },
        );
        t.set_u32(op(&t, 0x2d), s);
        junk(&mut t, s, 0x3100, k);
        for i in 0..0xc0u32 {
            t.set_u32(
                s + 0x7c0 + 4 * i,
                (0.25 * i as f32 - 3.0 + k as f32).to_bits(),
            );
        }
        // the list: n1 (S + 0x720 or not), n2 (S + 0x850 or not), n3
        t.write(objs, &[0; 0x10]);
        for n in [n1, n2, n3] {
            t.write(n, &[0x31; 0x40]);
        }
        t.set_u32(objs, n1);
        t.set_u32(objs + 4, n3);
        t.set_u32(objs + 8, n3);
        t.set_u32(objs + 0xc, 3);
        t.set_u32(n1, 0);
        t.set_u32(n1 + 4, n2);
        t.set_u32(n1 + 8, if inl & 1 != 0 { s + 0x720 } else { 0x1234 });
        t.set_u32(n2, n1);
        t.set_u32(n2 + 4, n3);
        t.set_u32(n2 + 8, if inl & 2 != 0 { s + 0x850 } else { 0x5678 });
        t.set_u32(n3, n2);
        t.set_u32(n3 + 4, 0);
        t.set_u32(n3 + 8, 0x9abc);
        t.set_u32(op(&t, 0x50), SCRATCH + 0x5fa0);
        t.set_u32(op(&t, 0x89), SCRATCH + 0x5fa4);
        t.set_u32(SCRATCH + 0x5fa0, 0x5fa0_0000 + k);
        t.set_u32(SCRATCH + 0x5fa4, 0x5fa4_0000 + k);
        for o in [0x720u32, 0x850] {
            t.set_u32(s + o + 0x124, 0xc3a5_96f1 ^ (k << 8));
        }
        t.set_u32(r557, 0x0557_0000 + k);
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
        t.write(last, &[0x34; 0x20]);
        t.set_u32(op(&t, 0x1f1), if busy { last } else { 0 });
        t.set_u32(op(&t, 0x208), last);
        ownb::dirty_stack(&mut t, 0x3b20 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0587` 0x10002bc0 (`r27::trails_start_0587`): the object in the
/// list and not, the pool's last node and one of several, the task list
/// empty and not.
#[test]
fn trails_start_0587() {
    let Some(mut t) = twin(587) else { return };
    stubs_0586(&mut t);
    for (s_, n_) in [(713u32, 1usize), (256, 5)] {
        rec(&mut t, s_, n_, move |_, a| (s_ << 16) ^ a[0]);
    }
    let f = t.at(0x2bc0);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let so = t.img.rel32_target(f + 0x68);
    let s = SCRATCH + 0x8000;
    let objs = op(&t, 0x18);
    let (n1, n2) = (SCRATCH + 0x6100, SCRATCH + 0x6140);
    let last = SCRATCH + 0x3400;
    let pool = SCRATCH + 0x4000;
    // (the object in the list, the pool's count, the task list busy)
    let cases: [(bool, u16, bool); 4] = [
        (true, 3, false),
        (false, 1, true),
        (true, 1, false),
        (false, 2, true),
    ];
    for (k, (inl, count, busy)) in (0u32..).zip(cases) {
        start_env_at(&mut t, so, (0x99, 0xa4), 2, 0);
        t.set_u32(op(&t, 0x9), s);
        junk(&mut t, s, 0x3100, k);
        for i in 0..0xc0u32 {
            t.set_u32(
                s + 0x7c0 + 4 * i,
                (0.25 * i as f32 - 3.0 + k as f32).to_bits(),
            );
        }
        t.write(objs, &[0; 0x10]);
        for n in [n1, n2] {
            t.write(n, &[0x31; 0x40]);
        }
        t.set_u32(objs, n1);
        t.set_u32(objs + 4, n2);
        t.set_u32(objs + 8, n1);
        t.set_u32(objs + 0xc, 2);
        t.set_u32(n1, 0);
        t.set_u32(n1 + 4, n2);
        t.set_u32(n1 + 8, if inl { s + 0x5f0 } else { 0x1234 });
        t.set_u32(n2, n1);
        t.set_u32(n2 + 4, 0);
        t.set_u32(n2 + 8, 0x5678);
        t.set_u32(op(&t, 0x29), SCRATCH + 0x5fa0);
        t.set_u32(SCRATCH + 0x5fa0, 0x5fa0_0000 + k);
        t.set_u32(s + 0x5f0 + 0x124, 0xc3a5_96f1 ^ (k << 8));
        let a = SCRATCH + 0x5fc0;
        t.set_u32(op(&t, 0x127), a);
        t.set_u32(a + 0x20, 0x00a2_0000 + k);
        // the pool: a ring of `count` free nodes
        t.write(pool - 0x20, &[0x6d; 0x20 + 0x1c * 4]);
        for i in 0..u32::from(count) {
            let node = pool + 0x1c * i;
            let n = u32::from(count);
            t.set_u32(node, pool + 0x1c * ((i + n - 1) % n));
            t.set_u32(node + 4, pool + 0x1c * ((i + 1) % n));
        }
        t.set_u16(op(&t, 0x205), count);
        t.set_u32(op(&t, 0x24c), pool);
        t.write(last, &[0x34; 0x20]);
        t.set_u32(op(&t, 0x2b3), if busy { last } else { 0 });
        t.set_u32(op(&t, 0x2cc), last);
        ownb::dirty_stack(&mut t, 0x2bc0 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0587` 0x100032f0 (`r27::third_start_0587`): the object in the
/// list and not, the spin wrapped both ways and kept, the pool's last
/// node and one of several, the task list empty and not.
#[test]
fn third_start_0587() {
    let Some(mut t) = twin(587) else { return };
    stubs_0586(&mut t);
    rec(&mut t, 713, 1, |_, a| 0x0713_0000 ^ a[0]);
    let f = t.at(0x32f0);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let so = t.img.rel32_target(f + 0x117);
    let s = SCRATCH + 0x8000;
    let objs = op(&t, 0xb7);
    let (n1, n2) = (SCRATCH + 0x6100, SCRATCH + 0x6140);
    let last = SCRATCH + 0x3400;
    let pool = SCRATCH + 0x4000;
    // (the object in the list, the turn, the pool's count, the task list busy)
    let cases: [(bool, f32, u16, bool); 4] = [
        (true, 0.0, 3, false),
        (false, 3.1, 1, true),
        (true, -3.1, 1, false),
        (false, 1.0, 2, true),
    ];
    for (k, (inl, turn, count, busy)) in (0u32..).zip(cases) {
        start_env_at(&mut t, so, (0x99, 0xa4), 2, 0);
        t.set_u32(op(&t, 0xe), s);
        junk(&mut t, s, 0x3100, k);
        for i in 0..0xc0u32 {
            t.set_u32(
                s + 0x7c0 + 4 * i,
                (0.25 * i as f32 - 3.0 + k as f32).to_bits(),
            );
        }
        t.set_u32(s + 0x30f0, turn.to_bits());
        t.write(objs, &[0; 0x10]);
        for n in [n1, n2] {
            t.write(n, &[0x31; 0x40]);
        }
        t.set_u32(objs, n1);
        t.set_u32(objs + 4, n2);
        t.set_u32(objs + 8, n1);
        t.set_u32(objs + 0xc, 2);
        t.set_u32(n1, 0);
        t.set_u32(n1 + 4, n2);
        t.set_u32(n1 + 8, if inl { s + 0x130 } else { 0x1234 });
        t.set_u32(n2, n1);
        t.set_u32(n2 + 4, 0);
        t.set_u32(n2 + 8, 0x5678);
        t.set_u32(op(&t, 0xd8), SCRATCH + 0x5fa0);
        t.set_u32(SCRATCH + 0x5fa0, 0x5fa0_0000 + k);
        t.set_u32(s + 0x130 + 0x124, 0xc3a5_96f1 ^ (k << 8));
        let a = SCRATCH + 0x5fc0;
        t.set_u32(op(&t, 0x7), a);
        t.set_u32(a + 0x20, 0x00a2_0000 + k);
        t.write(pool - 0x20, &[0x6d; 0x20 + 0x1c * 4]);
        for i in 0..u32::from(count) {
            let node = pool + 0x1c * i;
            let n = u32::from(count);
            t.set_u32(node, pool + 0x1c * ((i + n - 1) % n));
            t.set_u32(node + 4, pool + 0x1c * ((i + 1) % n));
        }
        t.set_u16(op(&t, 0x261), count);
        t.set_u32(op(&t, 0x3c7), pool);
        t.write(last, &[0x34; 0x20]);
        t.set_u32(op(&t, 0x42e), if busy { last } else { 0 });
        t.set_u32(op(&t, 0x445), last);
        ownb::dirty_stack(&mut t, 0x32f0 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0587` 0x10002520 (`r27::first_start_0587`): the turns wrapped
/// both ways and kept, a task node already there and not, the pool's last
/// node and one of several, the task list empty and not.
#[test]
fn first_start_0587() {
    let Some(mut t) = twin(587) else { return };
    stubs_0586(&mut t);
    rec(&mut t, 256, 5, |_, a| 0x0256_0000 ^ a[0]);
    // slot 229 as a matrix product (floats: the sums the port adds)
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let mut x = 0.0f32;
            for j in 0..4 {
                x += rf(m, a[1] + 16 * j + 4 * i) * rf(m, a[2] + 4 * j);
            }
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    let f = t.at(0x2520);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let so = t.img.rel32_target(f + 0x33);
    let s = SCRATCH + 0x8000;
    let last = SCRATCH + 0x3400;
    let pool = SCRATCH + 0x4000;
    // (turn x, turn z, node there, the pool's count, the task list busy)
    let cases: [(f32, f32, bool, u16, bool); 5] = [
        (0.0, 0.0, false, 3, false),
        (-3.0, 3.1, false, 1, true),
        (3.1, -3.5, true, 2, false),
        (1.0, 1.0, false, 2, true),
        (f32::NAN, 2.0, false, 1, false),
    ];
    for (k, (ax, az, have, count, busy)) in (0u32..).zip(cases) {
        start_env_at(&mut t, so, (0x99, 0xa4), 2, 0);
        t.set_u32(op(&t, 0x1b), s);
        junk(&mut t, s, 0x3100, k);
        for i in 0..0xc0u32 {
            t.set_u32(
                s + 0x7c0 + 4 * i,
                (0.25 * i as f32 - 3.0 + k as f32).to_bits(),
            );
        }
        t.set_u32(s + 0xa40, ax.to_bits());
        t.set_u32(s + 0xa48, az.to_bits());
        t.set_u32(s + 0x30e8, if have { SCRATCH + 0x6200 } else { 0 });
        let a = SCRATCH + 0x5fc0;
        t.set_u32(op(&t, 0x308), a);
        t.set_u32(a + 0x20, 0x00a2_0000 + k);
        t.write(pool - 0x20, &[0x6d; 0x20 + 0x1c * 4]);
        for i in 0..u32::from(count) {
            let node = pool + 0x1c * i;
            let n = u32::from(count);
            t.set_u32(node, pool + 0x1c * ((i + n - 1) % n));
            t.set_u32(node + 4, pool + 0x1c * ((i + 1) % n));
        }
        t.set_u16(op(&t, 0x3ce), count);
        t.set_u32(op(&t, 0x3dd), pool);
        t.write(last, &[0x34; 0x20]);
        t.set_u32(op(&t, 0x444), if busy { last } else { 0 });
        t.set_u32(op(&t, 0x453), last);
        ownb::dirty_stack(&mut t, 0x2520 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0570` 0x10009830, `magic_0597` 0x10009060 and `magic_0070`
/// 0x10009b40 (`r27::spark_step_0570`): every state, the wait below and at
/// its bound, a state past the table, the random words varied (the top
/// bits' shift of the colour steps too).
#[test]
fn spark_step_0570() {
    for (n, rva) in [(570u16, 0x9830u32), (597, 0x9060), (70, 0x9b40)] {
        let Some(mut t) = twin(n) else { return };
        all_stubs(&mut t);
        let f = t.at(rva);
        let op = |t: &Twin, o: u32| t.img.word(f + o);
        rec(&mut t, 325, 2, |m, a| {
            for i in 0..4 {
                let v = m.r32(a[1] + 4 * i);
                m.w32(a[0] + 4 * i, v);
            }
            0x325
        });
        let rnd = SCRATCH + 0x5f00;
        rec(&mut t, 324, 0, move |m, _| {
            let k = m.r32(rnd);
            m.w32(rnd, k + 1);
            k.wrapping_mul(0x9e37_79b9) ^ m.r32(rnd + 4)
        });
        let wait = t.img.word(f + 0x40) >> 24;
        let sparks = op(&t, 0x27);
        let obj = SCRATCH + 0x1000;
        // (index, state, age, random seed)
        let cases: [(u32, u8, u16, u32); 8] = [
            (0, 0, 5, 0),
            (1, 1, 0, 0x1234_5678),
            (2, 1, 3, 0xc000_0000),
            (3, 2, (wait - 1) as u16, 0),
            (4, 2, wait as u16, 0),
            (5, 3, 9, 0),
            (6, 7, 1, 0),
            (7, 1, 0, 0x4000_0000),
        ];
        for (k, (idx, state, age, seed)) in (0u32..).zip(cases) {
            let r = sparks + 0x30 * idx;
            for i in 0..0xcu32 {
                t.set_u32(r + 4 * i, (0.5 * i as f32 - 1.0 + k as f32).to_bits());
            }
            t.set_u16(r + 0x20, age);
            t.set_u8(r + 0x22, state);
            t.set_u8(r + 0x23, 0x5a);
            t.set_u32(op(&t, 0x16), idx);
            junk(&mut t, obj, 0x150, k);
            for (i, v) in (0u32..).zip([1.5f32, -2.0, 0.75 + k as f32]) {
                t.set_u32(obj + 0xb0 + 4 * i, v.to_bits());
            }
            for i in 0..4u32 {
                t.set_u32(obj + 0xa0 + 4 * i, (2.0 * i as f32 + 0.5).to_bits());
            }
            t.set_u32(rnd, k);
            t.set_u32(rnd + 4, seed);
            ownb::dirty_stack(&mut t, rva + k, &[]);
            t.check_entry(f, Call::default(), &[obj], Eax::Checked);
        }
    }
}

/// `magic_0589` 0x10002000 and `magic_0511` 0x10002240
/// (`r27::summon_models_0589`): the selected actors and their places
/// varied.
#[test]
fn summon_models_0589() {
    for (n, rva) in [(589u16, 0x2000u32), (511, 0x2240)] {
        let Some(mut t) = twin(n) else { return };
        stubs_0586(&mut t);
        rec(&mut t, 0, 1, |_, a| 0x00a0_0000 ^ a[0]);
        for (s_, n_) in [(26u32, 2usize), (10, 2), (31, 2)] {
            rec(&mut t, s_, n_, move |_, a| (s_ << 16) ^ a[0] ^ a[1]);
        }
        let f = t.at(rva);
        let op = |t: &Twin, o: u32| t.img.word(f + o);
        let s = SCRATCH + 0x8000;
        let sel = SCRATCH + 0x600;
        for k in 0..3u32 {
            t.set_u32(op(&t, 0x1c), s);
            junk(&mut t, s, 0x1b00, k);
            for i in 0..0x90u32 {
                t.set_u32(
                    s + 0x1800 + 4 * i,
                    (0.25 * i as f32 - 3.0 + k as f32).to_bits(),
                );
            }
            t.set_u32(op(&t, 0xce), sel);
            t.write(sel, &[7, 9, 9, (k % 3) as u8, ((k + 1) % 3) as u8, 2, 9, 9]);
            for i in 0..4u32 {
                t.set_u32(op(&t, 0xe6) + 4 * i, 0x00ac_0000 + 0x10 * i + k);
            }
            junk_ops(&mut t, f, &[0xd4, 0x49, 0x85, 0xc1]);
            ownb::dirty_stack(&mut t, rva + k, &[]);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `magic_0578` 0x10007320 (`r27::fade_task_0578`): every state with its
/// count running and out, the entry's float below and past its bound
/// (NaN once), the repeat word, the fade above 0 and past it (the nodes
/// with parents and without, first and not), states outside the tables.
#[test]
fn fade_task_0578() {
    let Some(mut t) = twin(578) else { return };
    all_stubs(&mut t);
    rec(&mut t, 771, 1, |_, a| 0x0771_0000 ^ a[0]);
    t.stub_edx(771, |_, a| 0x00ed_0000 ^ a[0]);
    let lock = SCRATCH + 0x7d0;
    rec(&mut t, 779, 2, move |m, a| {
        let l = m.r32(lock);
        fold(m, 779, 0, l ^ a[1]);
        0x779
    });
    let f = t.at(0x7320);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let task = SCRATCH + 0x100;
    let (tn, tprev, tnext) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let r = SCRATCH + 0x1_0000;
    let parent = SCRATCH + 0x3000;
    let sib = SCRATCH + 0x3100;
    let tbl = op(&t, 0x24e);
    // (state, count, entry float, repeat, fade, parents, entry index)
    let cases: [(u32, i32, f32, u32, f32, u8, u32); 18] = [
        (0, 0, 0.0, 0, 0.0, 0, 0),
        (1, 2, 0.0, 0, 0.0, 0, 1),
        (1, 0, 0.0, 0, 0.0, 0, 1),
        (1, 0, 100.0, 0, 0.0, 0, 2),
        (1, 0, f32::NAN, 0, 0.0, 0, 0),
        (10, 0, 0.0, 0, 0.0, 0, 0),
        (11, 3, 0.0, 1, 0.0, 0, 0),
        (11, 0, 0.0, 2, 0.0, 0, 0),
        (11, 0, 0.0, 0, 0.0, 0, 0),
        (0x28, 0, 0.0, 0, 0.0, 0, 0),
        (0x29, 4, 0.0, 0, 0.0, 0, 0),
        (0x29, 0, 0.0, 0, 0.0, 0, 1),
        (0x2a, 0, 0.0, 0, 0.75, 0, 2),
        (0x2a, 0, 0.0, 0, 0.01, 3, 1),
        (0x2a, 0, 0.0, 0, 0.01, 1, 0),
        (0x14, 0, 0.0, 0, 0.0, 0, 0),
        (5, 0, 0.0, 0, 0.0, 0, 0),
        (0x4e20, 0, 0.0, 0, 0.0, 0, 0),
    ];
    for (k, (state, count, x, rep, lv, par, idx)) in (0u32..).zip(cases) {
        junk(&mut t, r, 0x800, k);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x20, count.cast_unsigned());
        t.set_u32(r + 0x24, rep);
        t.set_u32(r + 0x28, idx);
        t.set_u32(r + 0x1c, lv.to_bits());
        junk(&mut t, tbl, 0x50 * 3, k + 1);
        for i in 0..3 {
            t.set_u32(tbl + 0x24 + 0x50 * i, (0.25 * i as f32).to_bits());
        }
        t.set_u32(tbl + 0x24 + 0x50 * idx, x.to_bits());
        junk(&mut t, parent, 0x40, k + 2);
        junk(&mut t, sib, 0x40, k + 3);
        for (bit, n) in [(1u8, 0x750u32), (2, 0x7b0)] {
            let node = r + n;
            if par & bit != 0 {
                // a parent with the node between a sibling and the parent
                t.set_u32(node + 0x14, parent);
                t.set_u32(node + 4, if bit == 1 { parent } else { sib });
                t.set_u32(node + 8, if bit == 1 { sib } else { parent });
            } else {
                t.set_u32(node + 0x14, 0);
            }
        }
        t.set_u32(op(&t, 0x15b), lock);
        t.set_u32(lock, 0x40 + k);
        junk(&mut t, task, 0x10, k + 3);
        t.set_u32(task, tn);
        t.set_u32(task + 4, r);
        t.write(tn, &[0x3c; 0x20]);
        t.set_u32(tn + 4, tprev);
        t.set_u32(tn + 8, tnext);
        t.write(tprev, &[0x4d; 0x20]);
        t.write(tnext, &[0x5e; 0x20]);
        ownb::dirty_stack(&mut t, 0x7320 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// The particle node's globals behind `particle_node_with` at `pnw` (as
/// `magic_s3.rs`'s test): the lock, the pool word and the owner block.
fn particle_globals(t: &mut Twin, pnw: u32) {
    let pn = t.img.rel32_target(pnw + 0x35);
    let wrapper = t.img.rel32_target(pn + 0x88);
    t.set_u32(t.img.word(wrapper + 0x04), SCRATCH + 0x7f00);
    t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x7f10);
    let owner = SCRATCH + 0x7a00;
    t.set_u32(t.img.word(pnw + 0x0d), owner);
    for i in 0..0x40 {
        t.set_u32(owner + 4 * i, 0x0a00_0000 + i);
    }
}

/// `magic_0578` 0x10006fc0 (`r27::summon_task_0578`): every state, slot
/// 411 answering and not, the counts running and out, each kind, the
/// static list empty and not, states past the table.
#[test]
fn summon_task_0578() {
    let Some(mut t) = twin(578) else { return };
    all_stubs(&mut t);
    let f = t.at(0x6fc0);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let cell = SCRATCH + 0x7c0;
    rec(&mut t, 411, 1, move |m, a| m.r32(cell) ^ (a[0] & 0x100));
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    for (s, n) in [(228u32, 2usize), (213, 2), (941, 1)] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], 0.5 * i as f32 - 1.0 + (a[0] & 3) as f32);
        }
        0x262
    });
    let pnw = t.img.rel32_target(f + 0x1b7);
    particle_globals(&mut t, pnw);
    let plist = t.img.word(pnw + 0x26);
    let pl2 = op(&t, 0x26a);
    let list = op(&t, 0xc6);
    let old = SCRATCH + 0x800;
    let task = SCRATCH + 0x100;
    let (tn, tprev, tnext) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let r = SCRATCH + 0x1_0000;
    let o = SCRATCH + 0x3000;
    let owner = op(&t, 0x240);
    let ob = SCRATCH + 0x7a00;
    // (state, count, slot 411, kind)
    let cases: [(u32, u32, u32, u32); 16] = [
        (0, 3, 0, 0),
        (1, 3, 0, 0),
        (1, 3, 1, 0),
        (1, 3, 1, 2),
        (2, 4, 0, 1),
        (2, 0, 0, 1),
        (10, 0, 0, 2),
        (11, 4, 0, 0),
        (11, 0, 0, 0),
        (20, 2, 0, 1),
        (21, 0, 0, 0),
        (21, 0, 5, 0),
        (5, 0, 0, 0),
        (0x14, 0, 0, 0),
        (0x16, 0, 0, 0),
        (0x4e20, 0, 0, 0),
    ];
    for (k, (state, count, ans, kind)) in (0u32..).zip(cases) {
        junk(&mut t, r, 0x600, k);
        junk(&mut t, o, 0x100, k + 1);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x1c, count);
        t.set_u32(r + 0x20, kind);
        t.set_u32(r + 0x24, o);
        t.set_u32(cell, ans);
        for n in [0xb0u32, 0x1fc, 0x350, 0x450] {
            t.set_u32(r + n + 0x14, 0);
        }
        for (l, i) in [(list, 0u32), (plist, 1), (pl2, 2)] {
            junk(&mut t, l, 0x20, k + 2 + i);
            t.set_u32(l + 0x10, if (k + i) % 2 == 0 { l } else { old });
            t.set_u16(l + 0x18, 0x0305);
        }
        t.write(old, &[0x6c; 0x40]);
        t.set_u32(owner, ob);
        junk(&mut t, ob, 0x40, k + 6);
        t.set_u32(ob + 0x10, ob);
        junk(&mut t, task, 0x10, k + 3);
        t.set_u32(task, tn);
        t.set_u32(task + 4, r);
        t.write(tn, &[0x3c; 0x20]);
        t.set_u32(tn + 4, tprev);
        t.set_u32(tn + 8, tnext);
        t.write(tprev, &[0x4d; 0x20]);
        t.write(tnext, &[0x5e; 0x20]);
        ownb::dirty_stack(&mut t, 0x6fc0 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x40, "the task's calls");
}

/// `magic_0578` 0x10007730 (`r27::fade_setup_0578`): the kinds 0, 1 and
/// 2, the source's alpha from slot 333 and from slot 601, the static list
/// empty and not, the ring's last varied.
#[test]
fn fade_setup_0578() {
    let Some(mut t) = twin(578) else { return };
    all_stubs(&mut t);
    let f = t.at(0x7730);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    for (s, x) in [(333u32, -0.75f64), (251, 0.625)] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    rec(&mut t, 601, 2, |m, a| {
        wf(m, a[1], 1.75 + (a[0] & 1) as f32);
        0x601
    });
    for (s, n) in [(676u32, 3usize), (678, 1), (719, 3), (941, 1)] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    // the colour node's helper keeps slot 790's float; slot 558 writes 4
    rec(&mut t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |_, _| 0.375);
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 + 0x11 * i);
        }
        0x558
    });
    let list = op(&t, 0x9d);
    let pl2 = op(&t, 0x2dd);
    let pl3 = op(&t, 0x652);
    let owner = op(&t, 0x2b0);
    let ob = SCRATCH + 0x7a00;
    let old = SCRATCH + 0x800;
    let o = SCRATCH + 0x1_0000;
    let src = SCRATCH + 0x3000;
    for (k, (kind, flag)) in (0u32..).zip([(0u32, 0u16), (1, 1), (2, 0), (1, 0)]) {
        junk(&mut t, o, 0x2200, k);
        junk(&mut t, src, 0x100, k + 1);
        t.set_u16(src + 0x90, flag);
        t.set_u32(o + 0x164 + 0x14, 0);
        for (l, i) in [(list, 0u32), (pl2, 1), (pl3, 2)] {
            junk(&mut t, l, 0x20, k + 2 + i);
            t.set_u32(l + 0x10, if (k + i) % 2 == 0 { l } else { old });
            t.set_u16(l + 0x18, 0x0305);
        }
        t.write(old, &[0x6c; 0x40]);
        let ring_end = SCRATCH + 0x880;
        t.write(ring_end, &[0x5d; 0x40]);
        t.set_u32(op(&t, 0x21), ring_end);
        t.set_u32(owner, ob);
        junk(&mut t, ob, 0x40, k + 6);
        t.set_u32(ob + 0x10, ob);
        t.set_u32(op(&t, 0x3f), 0x0101_0101 + k);
        ownb::dirty_stack(&mut t, 0x7730 + k, &[]);
        let c = Call {
            ecx: o,
            edx: src,
            ..Call::default()
        };
        t.check(
            f,
            c,
            &[0x00a0_0000 + k, kind, 0x00b0_0000 + k],
            Eax::Checked,
            |g, img, fr| Ret::eax(r27::fade_setup_0578(g, img, fr)),
        );
    }
}

/// `magic_0578` 0x10008110 (`r27::light_task_0578`): the start with slot
/// 390 matching each model, none and some, slot 288's list with and
/// without the skipped ids, the alpha and model state from slots 333/15-17
/// and from 601/290, the static lists empty and not; the waits in states
/// 1, 11 (slot 415 0x10 and not), 12 and 13 (with kept ids); state 10;
/// the end with the objects' words sharing a bit of 0x4e20 and not (the
/// model nodes with parents and without); states outside the machine.
#[test]
fn light_task_0578() {
    let Some(mut t) = twin(578) else { return };
    all_stubs(&mut t);
    let f = t.at(0x8110);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let fs = t.img.rel32_target(f + 0x909);
    let lo = t.img.rel32_target(f + 0x88f);
    let ops = |t: &Twin, at: u32, o: u32| t.img.word(at + o);
    // slot 390: the model for an id from the table at `ans`
    let ans = SCRATCH + 0x7c0;
    rec(&mut t, 390, 1, move |m, a| m.r32(ans + 4 * (a[0] & 7)));
    let ids = SCRATCH + 0x7e0;
    let cnt = SCRATCH + 0x7f0;
    rec(&mut t, 288, 2, move |m, a| {
        let n = m.r32(cnt);
        m.w32(a[1], n);
        ids
    });
    let r415 = SCRATCH + 0x7f4;
    rec(&mut t, 415, 1, move |m, a| {
        m.r32(r415) ^ (a[0] & 0x1000_0000)
    });
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], 0.5 * i as f32 - 1.0 + (a[0] & 3) as f32);
        }
        0x262
    });
    for (s, x) in [
        (333u32, -0.75f64),
        (251, 0.625),
        (15, 0.5),
        (16, 0.75),
        (17, 1.5),
        (277, 2.0),
    ] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    rec(&mut t, 601, 2, |m, a| {
        wf(m, a[1], 1.75 + (a[0] & 1) as f32);
        0x601
    });
    rec(&mut t, 290, 2, |m, a| {
        for i in 0..3 {
            wf(m, a[1] + 4 * i, 0.25 * i as f32 + 3.0);
        }
        0x290
    });
    let wait = SCRATCH + 0x7f8;
    rec(&mut t, 411, 1, move |m, a| m.r32(wait) ^ (a[0] & 0x100));
    for (s, n) in [
        (234u32, 3usize),
        (719, 3),
        (676, 3),
        (678, 1),
        (941, 1),
        (413, 1),
        (215, 1),
        (14, 2),
        (213, 2),
        (211, 2),
        (228, 2),
        (296, 2),
        (357, 3),
        (356, 6),
        (362, 4),
        (704, 4),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    let level = SCRATCH + 0x90;
    rec(&mut t, 363, 3, move |m, _| m.r32(level));
    rec(&mut t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |_, _| 0.375);
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 + 0x11 * i);
        }
        0x558
    });
    let list = op(&t, 0x20c);
    let old = SCRATCH + 0x800;
    let ring_end = SCRATCH + 0x880;
    let task = SCRATCH + 0x100;
    let (tn, tprev, tnext) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0);
    let r = SCRATCH + 0x1_0000;
    let model = SCRATCH + 0x3000;
    let ob = SCRATCH + 0x7a00;
    let ls = SCRATCH + 0x700;
    let parent = SCRATCH + 0x3400;
    // (state, count, matches (bit per node), the list's count, flag
    // byte, end words share 0x4e20, slot 415 / 411 answer, parents)
    let cases: [(u32, u32, u8, u32, u8, bool, u32, bool); 20] = [
        (0, 0, 0b111, 4, 1, false, 0, false),
        (0, 0, 0b010, 0, 0, false, 0, false),
        (0, 0, 0b000, 6, 1, false, 0, false),
        (1, 3, 0, 0, 0, false, 0, false),
        (1, 0, 0, 0, 0, false, 0, false),
        (5, 0, 0, 0, 0, false, 0, false),
        (10, 4, 0, 0, 0, false, 0, false),
        (11, 2, 0, 0, 0, false, 0, false),
        (11, 0, 0, 0, 0, false, 0x10, false),
        (11, 0, 0, 0, 1, false, 3, false),
        (12, 5, 0, 0, 0, false, 0, false),
        (12, 0, 0, 0, 0, false, 0, false),
        (13, 0, 0, 0, 0, false, 0, false),
        (13, 0, 0, 3, 0, false, 1, false),
        (13, 0, 0, 0, 0, true, 1, true),
        (0x2710, 0, 0, 0, 0, true, 0, true),
        (0x2710, 0, 0, 0, 0, true, 0, false),
        (0x2710, 0, 0, 0, 0, false, 0, false),
        (0x4e20, 0, 0, 0, 0, false, 0, false),
        (0xd, 3, 0, 2, 0, false, 0, false),
    ];
    for (k, (state, count, matches, n, flags, share, answer, par)) in (0u32..).zip(cases) {
        junk(&mut t, r, 0x9b00, k);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x20, count);
        t.set_u8(r + 0xb4, flags);
        t.set_u32(r + 0x74, 0x0074_0000 + k);
        t.set_u32(r + 0x78, 0x0078_0000 + k);
        for nd in [0x24u32, 0xbc, 0x154] {
            t.set_u32(r + nd + 0x54, model);
        }
        junk(&mut t, model, 0x100, k + 1);
        t.set_u32(model + 0x28, (2.0 + k as f32).to_bits());
        t.set_u16(model + 0x90, (k & 1) as u16);
        // the model nodes' parents: none, or a parent with them inside
        junk(&mut t, parent, 0x40, k + 9);
        for nd in [0x24u32, 0xbc, 0x154] {
            let node = r + nd;
            if par {
                t.set_u32(node + 0x14, parent);
                t.set_u32(node + 4, parent);
                t.set_u32(node + 8, parent);
            } else {
                t.set_u32(node + 0x14, 0);
            }
        }
        let (x, y) = if share {
            (0x4e20, 0xffff)
        } else {
            (0x4e20, 0xb1df)
        };
        for (o, v) in [(0x5428u32, x), (0xf28, y), (0x9d8, 0xffff), (0x488, 0xffff)] {
            t.set_u32(r + o, v);
        }
        // slot 390: id i+0x14 answers node i's model when its bit is set
        for i in 0..8u32 {
            t.set_u32(ans + 4 * i, 0x2000 + i);
        }
        for (i, m) in [0x1084u32, 0x10ad, 0x10ae].into_iter().enumerate() {
            if matches & (1 << i) != 0 {
                t.set_u32(ans + 4 * ((0x14 + i as u32) & 7), m);
            }
        }
        t.write(ids, &[0x14, 3, 0x15, 7, 0x16, 9, 0, 0]);
        t.set_u32(cnt, n);
        t.set_u16(r + 0x9a48, (n & 3) as u16);
        t.set_u32(r415, answer);
        t.set_u32(wait, answer);
        junk(&mut t, list, 0x20, k + 2);
        t.set_u32(list + 0x10, if k % 2 == 0 { list } else { old });
        t.set_u16(list + 0x18, 0x0305);
        t.write(old, &[0x6c; 0x40]);
        t.write(ring_end, &[0x5d; 0x40]);
        t.set_u32(op(&t, 0x686), ring_end);
        t.set_u32(op(&t, 0x69e), 0x0101_0101 + k);
        // the fade set-up's other lists and its particle owner
        for (l, i) in [(ops(&t, fs, 0x2dd), 2u32), (ops(&t, fs, 0x652), 3)] {
            junk(&mut t, l, 0x20, k + 2 + i);
            t.set_u32(l + 0x10, if (k + i) % 2 == 0 { l } else { old });
            t.set_u16(l + 0x18, 0x0305);
        }
        t.set_u32(ops(&t, fs, 0x2b0), ob);
        junk(&mut t, ob, 0x40, k + 6);
        t.set_u32(ob + 0x10, ob);
        t.set_u32(r + 0xf10 + 0x164 + 0x14, 0);
        t.set_u32(r + 0x5410 + 0x164 + 0x14, 0);
        // the light object: its level answer, scale word and parent list
        t.set_u32(level, if k % 2 == 0 { 0x7_0000 } else { 5 });
        t.set_u32(ops(&t, lo, 0x88), ls);
        t.set_u32(ls, 0x0055_0000 + k);
        let lp = ops(&t, lo, 0x6a);
        t.set_u32(lp + 0x10, lp);
        t.set_u32(lp + 0xc, lp);
        junk(&mut t, task, 0x10, k + 3);
        t.set_u32(task, tn);
        t.set_u32(task + 4, r);
        t.write(tn, &[0x3c; 0x20]);
        t.set_u32(tn + 4, tprev);
        t.set_u32(tn + 8, tnext);
        t.write(tprev, &[0x4d; 0x20]);
        t.write(tnext, &[0x5e; 0x20]);
        ownb::dirty_stack(&mut t, 0x8110 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x200, "the task's calls");
}

/// `magic_0577` 0x10005660 (`r27::thunk_bone_0577`, with 0x10005270
/// `r27::bone_0577`; `magic_r23.rs`'s `bone_0573` set-up): the walk stopping at once and moving (children and
/// none), the new-parent byte set and not. The DLL's function table and
/// the objects' `+0x1c` methods answer from logging stubs on both sides
/// (as `magic_0342`'s test does).
#[test]
fn bone_0577() {
    let Some(mut t) = twin(577) else { return };
    all_stubs(&mut t);
    let f = t.at(0x5660);
    let b = t.img.rel32_target(f + 8);
    let table = t.img.word(b + 0x103);
    rec(&mut t, 32, 2, |m, a| {
        for i in 0..16 {
            let x = 0.25 * i as f32 + rf(m, a[1]);
            wf(m, a[0] + 4 * i, x);
        }
        0x0032_0000 ^ a[1]
    });
    // each entry its own trap (slots 32..35)
    for i in 0..4 {
        t.set_u32(table + 4 * i, magic::TRAPS + 16 * (32 + i));
    }
    for i in 33..36u32 {
        rec(&mut t, i, 2, move |m, a| {
            for j in 0..16 {
                let x = 0.125 * j as f32 - rf(m, a[1]) + i as f32;
                wf(m, a[0] + 4 * j, x);
            }
            (i << 16) ^ a[1]
        });
    }
    t.set_u32(magic::TABLE + 4 * 1001, magic::TRAPS + 16 * 1001);
    rec(&mut t, 1001, 1, |m, a| {
        let x = m.r32(a[0]) ^ (m.r32(a[0] + 4) & 0xff00);
        fold(m, 1001, 0, x);
        0x1001_0000 ^ x
    });
    let vt = SCRATCH + 0x880;
    t.set_u32(vt + 0x1c, magic::TRAPS + 16 * 1001);
    rec(&mut t, 233, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) * f32::from_bits(a[2]) + 0.5;
            wf(m, a[0] + 4 * i, v);
        }
        0x233
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) * 0.5 + 0.25 * i as f32;
            wf(m, a[0] + 4 * i, v);
            fold(m, 229, i, v.to_bits());
        }
        0x229
    });
    for (s, n) in [(231u32, 3usize), (668, 3)] {
        rec(&mut t, s, n, move |m, a| {
            for i in 0..4 {
                let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i).rotate_left(3) ^ s;
                m.w32(a[0] + 4 * i, v);
                fold(m, s, i, v);
            }
            s
        });
    }
    for (s, n) in [(325u32, 2usize), (327, 2), (232, 2)] {
        rec(&mut t, s, n, move |m, a| {
            for i in 0..4 {
                let v = m.r32(a[1] + 4 * i) ^ (s << 4);
                m.w32(a[0] + 4 * i, v);
                fold(m, s, i, v);
            }
            s
        });
    }
    rec(&mut t, 1, 1, |_, a| 0x0001_0000 ^ a[0]);
    rec(&mut t, 219, 2, |_, a| a[1] * 0x40);
    let cursor = SCRATCH + 0x100;
    let n = SCRATCH + 0x2000;
    let (parent, q, show) = (SCRATCH + 0x3000, SCRATCH + 0x3400, SCRATCH + 0x3800);
    let objs = [
        SCRATCH + 0x4000,
        SCRATCH + 0x4100,
        SCRATCH + 0x4200,
        SCRATCH + 0x4300,
    ];
    // (stop, children, new parent, function index)
    let cases: [(bool, bool, u8, u8); 5] = [
        (true, true, 0, 0),
        (false, true, 0, 1),
        (false, false, 1, 2),
        (false, true, 1, 3),
        (false, false, 0, 0),
    ];
    for (k, (stop, kids, newp, which)) in (0u32..).zip(cases) {
        junk(&mut t, n, 0x1c0, k);
        for (i, o) in (0u32..).zip(objs) {
            junk(&mut t, o, 0x40, k + i);
            t.set_u32(o, vt);
            t.set_u32(o + 0x28, (0.25 + 0.125 * i as f32).to_bits());
        }
        for o in [parent, q] {
            junk(&mut t, o, 0x200, k + 7);
            t.set_u32(o, vt);
            // moderate floats where the node adds the parent's
            for i in 0..3 {
                t.set_u32(
                    o + 0xcc + 4 * i,
                    (0.75 * i as f32 - 1.5 + k as f32).to_bits(),
                );
            }
            // and slot 229's source (the parent's `+0xdc` matrix)
            for i in 0..16 {
                t.set_u32(o + 0xdc + 4 * i, (0.5 * i as f32 - 3.0).to_bits());
            }
        }
        junk(&mut t, show, 0x100, k + 9);
        t.set_u16(show + 0x90, (k & 1) as u16);
        t.set_u32(cursor, n);
        t.set_u8(cursor + 4, 0x77);
        t.set_u8(cursor + 5, 0x30 + k as u8);
        t.set_u8(n + 0x1a, if stop { 0x30 + k as u8 } else { 0x11 });
        if kids {
            t.set_u32(n + 0x10, objs[0]);
            t.set_u32(objs[0] + 8, objs[1]);
            t.set_u32(objs[1] + 8, n);
        } else {
            t.set_u32(n + 0x10, n);
        }
        t.set_u32(n + 0x128, parent);
        t.set_u32(n + 0x140, objs[2]);
        t.set_u32(n + 0x144, objs[3]);
        t.set_u8(n + 0x148, which);
        t.set_u8(n + 0x149, newp);
        t.set_u32(n + 0x14c, q);
        t.set_u32(n + 0x1a4, (1.5 + k as f32).to_bits());
        t.set_u32(n + 0x1a8, (0.75 * k as f32).to_bits());
        t.set_u32(n + 0x1ac, show);
        ownb::dirty_stack(&mut t, 0x5270 + k, &[]);
        let c = Call {
            ecx: 0x0ecc_0000 + k,
            ..Call::default()
        };
        t.check_entry(f, c, &[cursor], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x100, "the bones' calls");
}

/// `magic_0609` 0x1000bcd0 (`r27::start_up_0609`; `magic_r23.rs`'s
/// `start_up_0613` set-up): no targets and two;
/// the statics' root with and without a child; the rings' old last
/// words varied.
#[test]
fn start_up_0609() {
    let Some(mut t) = twin(609) else { return };
    all_stubs(&mut t);
    let f = t.at(0xbcd0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let ti = t.img.rel32_target(f + 0xc);
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
    t.set_u32(w(&t, 0x16f), lock);
    rec(&mut t, 711, 3, move |m, a| {
        let l = m.r32(lock);
        fold(m, 711, 0, l);
        0x0711_0000 ^ a[0]
    });
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(&mut t, 710, 4, |_, a| a[2] ^ 0x710);
    for (s, n) in [(718u32, 1usize), (222, 4), (221, 4), (560, 1)] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
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
        // the words the start clears or sets, junk first (the rings'
        // last words are the stores' own: the sentinels)
        junk_ops(
            &mut t,
            f,
            &[0x2b3, 0x2a9, 0x250, 0x25a, 0x17b, 0x298, 0x23a],
        );
        ownb::dirty_stack(&mut t, 0xbcd0 + k, &[]);
        t.check_entry(f, Call::default(), &[0x00ac_0000 + k], Eax::Checked);
    }
}
