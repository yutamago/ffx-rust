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
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i)
                ^ m.r32(a[1] + 0x30 + 4 * i)
                ^ m.r32(a[2] + 4 * i).rotate_left(i + 1);
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
    let cases: [(f32, u32, u32); 6] = [
        (0.0, 3, 0),
        (10.0, 0x21, 0),
        (3.0, 2, 1),
        (f32::NAN, 0xffff_fff0, 0),
        (-20.0, 0x20, 0),
        (1.570_796_4, 0x1f, 0),
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
    t.stub(234, |m, a| {
        for i in 0..3 {
            let x = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x234
    });
    t.stub(248, |m, a| {
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
        let xyz = [
            k * 1.5 + c as f32,
            (k * 0.7).sin() * 2.0,
            k * 0.25 - 3.0,
            1.0,
        ];
        for (j, v) in (0u32..).zip(xyz) {
            t.set_u32(o + 0xc + 16 * i + 4 * j, v.to_bits());
        }
        t.set_u8(o + 0x828 + i, u8::from(i != gap && i != head));
    }
    t.write(o + 0x80c, &[0x20 + c as u8, 0x90, 0x10, alpha]);
    t.write(o + 0x810, &[0xe0, 0x30 + c as u8, 0x70, alpha]);
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
    let cases: [(i32, u32, u32, u32, u8, f32); 11] = [
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
