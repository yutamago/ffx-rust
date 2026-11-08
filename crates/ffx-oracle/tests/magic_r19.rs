//! Round 19's ports (`ffx_magic::r19`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use magic_owna::{rec, row, twin};

/// Junk in the stack below the entry (the frames' locals start as it).
fn scrub(t: &mut Twin) {
    let entry = magic::STACK_TOP - 0x100;
    let junk: Vec<u8> = (0..0x1000u32)
        .map(|i| (i.wrapping_mul(0x2b) ^ 0x5b).to_le_bytes()[0])
        .collect();
    t.write(entry - 0x1000, &junk);
}

/// Junk words at the globals the operands `ops` of `f` name.
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, (0x5a5a_5a5a ^ o) | 1);
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

/// An empty list head at `l` (its child list itself).
fn empty_list(t: &mut Twin, l: u32) {
    t.write(l, &[0; 0x20]);
    t.set_u32(l + 0x10, l);
    t.set_u32(l + 0xc, l);
}

/// A task at `SCRATCH + 0x700` for `obj` (its node in a ring of three).
fn summon_task_at(t: &mut Twin, obj: u32) -> u32 {
    let (task, tn, a, b) = (
        SCRATCH + 0x700,
        SCRATCH + 0x740,
        SCRATCH + 0x7c0,
        SCRATCH + 0x7e0,
    );
    for (x, next, prev) in [(tn, a, b), (a, b, tn), (b, tn, a)] {
        t.write(x, &[0x44; 0x10]);
        t.set_u32(x + 4, next);
        t.set_u32(x + 8, prev);
    }
    t.set_u32(task, tn);
    t.set_u32(task + 4, obj);
    task
}

/// Where a test keeps an answer a stub gives.
const ANSWER_CELL: u32 = SCRATCH + 0x98;

/// The slots the summon tasks' callees make, as plain records: the model
/// slots (218 gives `count` model numbers), the float slots (12 the
/// ground, 333 an alpha), 237/262 fill vectors, 248 adds, 313 answers
/// [`ANSWER_CELL`], 941 a program table.
#[allow(clippy::too_many_lines)] // one stub per slot
fn summon_slots(t: &mut Twin, k: u32) {
    for (s, n) in [
        (413u32, 1usize),
        (215, 1),
        (14, 1),
        (211, 2),
        (228, 2),
        (222, 6),
        (36, 2),
        (257, 2),
        (296, 2),
        (38, 3),
        (541, 3),
        (538, 2),
        (213, 2),
        (35, 3),
        (34, 2),
        (547, 1),
        (276, 1),
        (219, 2),
        (331, 1),
        (353, 1),
        (271, 1),
    ] {
        rec(t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    let list = SCRATCH + 0x300;
    t.write(list, &[2, 0, 1, 3]);
    let count = SCRATCH + 0x9c;
    t.set_u32(count, 2);
    rec(t, 218, 2, move |m, a| {
        m.w32(a[1], m.r32(count));
        list
    });
    rec(t, 558, 2, |m, a| {
        for j in 0..4 {
            m.w32(a[0] + 4 * j, 0x20 + 3 * j);
        }
        0x558
    });
    rec(t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |_, _| 0.625);
    rec(t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, |_, _| 0.5);
    rec(t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, move |_, _| 0.5 + f64::from(k));
    rec(t, 262, 4, move |m, a| {
        for i in 1..4 {
            wf(m, a[i], 0.75 * i as f32 - k as f32);
        }
        0x262
    });
    rec(t, 237, 4, |m, a| {
        for i in 0..4 {
            wf(m, a[3] + 4 * i, 0.5 * i as f32 - 1.0);
        }
        0x237
    });
    rec(t, 248, 5, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
            fold(m, 248, i, v.to_bits());
        }
        0x248
    });
    rec(t, 313, 1, |m, _| m.r32(ANSWER_CELL));
    rec(t, 325, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x325
    });
    for s in [20u32, 411] {
        rec(t, s, 1, |m, _| m.r32(ANSWER_CELL));
    }
    rec(t, 217, 2, move |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, 1.5 * i as f32 - 2.0 + k as f32);
        }
        0x217
    });
    rec(t, 601, 2, move |m, a| {
        wf(m, a[1], 0.375 + k as f32);
        0x601
    });
    rec(t, 234, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
            fold(m, 234, i, v.to_bits());
        }
        0x234
    });
    rec(t, 232, 2, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) * 0.5;
            wf(m, a[0] + 4 * i, v);
            fold(m, 232, i, v.to_bits());
        }
        0x232
    });
    rec(t, 239, 3, |_, a| 0x0239_0000 ^ a[1]);
    rec(t, 30, 1, |_, a| 0x0030_0000 ^ a[0]);
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    let table = SCRATCH + 0x3400;
    for j in 0..0x10 {
        t.set_u32(table + 4 * j, 0x1111 * (j + 1));
    }
    rec(t, 941, 1, move |_, _| table);
}

/// The particle node a [`particle_node_with`] copy at `pnw` starts: its
/// list empty, its part start's lock, its owner's vectors.
fn particle_with(t: &mut Twin, pnw: u32, owner: u32) {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    empty_list(t, op(t, pnw, 0x26));
    let pn = t.img.rel32_target(pnw + 0x35);
    let wrapper = t.img.rel32_target(pn + 0x88);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(op(t, wrapper, 0x04), lock);
    t.set_u32(lock, 0x77);
    t.set_u32(op(t, pn, 0x45), SCRATCH + 0x7f10);
    t.set_u32(op(t, pnw, 0x0d), owner);
    for j in 0..0x40 {
        t.set_u32(owner + 4 * j, (0.25 * j as f32).to_bits());
    }
}

/// `magic_0343` 0x10006ad0 (`r19::summon_task_0343`): the record's
/// `(state, +0x28, +0x2c)`, slot 313's answer, the static model's and
/// the vector's flag bits (`bits`: 1 the flag step's, 2 the vector's), a
/// current list node or none.
fn summon_0343_setup(
    t: &mut Twin,
    f: u32,
    k: u32,
    (state, n28, n2c): (u32, u32, u32),
    (answer, bits, current): (u32, u8, bool),
) -> u32 {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    summon_slots(t, k);
    t.set_u32(ANSWER_CELL, answer);
    empty_list(t, op(t, f, 0xa8));
    for (call, owner) in [(0x247u32, 0x7a00u32), (0x55b, 0x7b00), (0x6e1, 0x7c00)] {
        let pnw = t.img.rel32_target(f + call);
        particle_with(t, pnw, SCRATCH + owner);
    }
    let enb = t.img.rel32_target(f + 0x25e);
    for o in [0x18u32, 0x1f] {
        empty_list(t, op(t, enb, o));
    }
    for o in [0x283u32, 0x2bc, 0x1b5] {
        t.set_u32(op(t, f, o), 0x0a0a_0000 + o + k);
    }
    let sfs = t.img.rel32_target(f + 0x310);
    t.set_u8(op(t, sfs, 0x02), 0x40 | (bits & 1));
    let vs = t.img.rel32_target(f + 0x35a);
    t.set_u8(op(t, vs, 0x06), 0x80 | (bits >> 1 & 1));
    // the summon's model node (0x100 bytes, up to the camera block)
    let node = op(t, f, 0x5a4);
    t.write(node, &[0x22; 0x100]);
    // the camera blocks' sources: real floats; their outputs junk
    for o in [
        0x91u32, 0x9c, 0xca, 0x105, 0x140, 0x153, 0x166, 0x16e, 0x17e, 0x186, 0x393, 0x39e, 0x3c1,
        0x3fc, 0x437, 0x459, 0x46c, 0x474, 0x484, 0x48c, 0x331, 0x4d2, 0x4da,
    ] {
        t.set_u32(op(t, f, o), (0.125 * o as f32 - 20.0 + k as f32).to_bits());
    }
    junk_ops(
        t,
        f,
        &[
            0xa2, 0xb3, 0xbf, 0xd0, 0xfa, 0x10b, 0x11f, 0x129, 0x135, 0x148, 0x15b, 0x176, 0x18e,
            0x196, 0x1ca, 0x1e1, 0x1eb, 0x1f5, 0x1ff, 0x241, 0x267, 0x26e, 0x3a3, 0x3b6, 0x3c7,
            0x3f1, 0x420, 0x42c, 0x442, 0x44e, 0x461, 0x47c, 0x494, 0x49c, 0x4ea, 0x4f6, 0x516,
            0x534, 0x550, 0x578, 0x582, 0x664, 0x67b, 0x699, 0x6db, 0x701, 0x708,
        ],
    );
    let lu = t.img.rel32_target(f + 0x5e4);
    let cur = op(t, lu, 0x02);
    if current {
        let n = SCRATCH + 0x7600;
        t.write(n, &[0x33; 0x40]);
        t.set_u32(n + 4, n);
        t.set_u32(n + 8, n);
        t.set_u32(cur, n);
    } else {
        t.set_u32(cur, 0);
    }
    let r = SCRATCH + 0x780;
    junk(t, r, 0x40, k);
    for (o, v) in [(0x18u32, state), (0x28, n28), (0x2c, n2c)] {
        t.set_u32(r + o, v);
    }
    summon_task_at(t, r)
}

/// `magic_0343` 0x10006ad0 (`r19::summon_task_0343`): every state, the
/// counts at their bounds, slot 313's two answers, both ways of the flag
/// steps, the unlink with and without a current node, states no case
/// takes.
#[test]
fn summon_task_0343() {
    let Some(mut t) = twin(343) else { return };
    let f = t.at(0x6ad0);
    for (k, (st, more)) in [
        ((0u32, 0u32, 5u32), (0u32, 0u8, false)),
        ((0, 0, 5), (0, 1, false)),
        ((10, 0, 5), (0, 0, false)),
        ((0x1e, 0, 0xa9), (0, 0, false)),
        ((0x1e, 0, 0xaa), (0, 0, false)),
        ((0x3c, 0, 0xffff_ffff), (0, 0, false)),
        ((0x3c, 0, 0), (1, 0, false)),
        ((0x3c, 0, 0), (0, 0, false)),
        ((0x32, 0x117, 3), (0, 0, false)),
        ((0x32, 0x118, 3), (0, 0, false)),
        ((0x32, 0x118, 3), (0, 2, false)),
        ((0x33, 0x149, 3), (0, 0, false)),
        ((0x33, 0x14a, 3), (0, 0, false)),
        ((0x34, 0, 0x59), (0, 0, false)),
        ((0x34, 0, 0x5a), (0, 0, false)),
        ((0x35, 0, 0x77), (0, 0, false)),
        ((0x35, 0, 0x78), (0, 0, false)),
        ((0x2710, 0, 0x1d), (0, 0, false)),
        ((0x2710, 0, 0x1e), (0, 0, false)),
        ((0x2710, 0, 0x1e), (0, 0, true)),
        ((5, 0, 0x1e), (0, 0, false)),
        ((0x31, 0, 0x1e), (0, 0, false)),
        ((0x36, 0, 0x1e), (0, 0, false)),
        ((0x3b, 0, 0x1e), (0, 0, false)),
        ((0x2711, 0, 0x1e), (0, 0, false)),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let task = summon_0343_setup(&mut t, f, k, st, more);
        scrub(&mut t);
        eprintln!("case {k}");
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    for (s, n) in [
        (218u32, 2u32),
        (237, 2),
        (313, 2),
        (222, 1),
        (248, 1),
        (262, 1),
    ] {
        assert!(t.u32(row(s)) >= n, "slot {s}: {} calls", t.u32(row(s)));
    }
}

/// Where `summon_0202_setup` keeps the distance base slot 394 adds its
/// call count to.
const DIST_CELL: u32 = SCRATCH + 0xa0;

/// `magic_0202` 0x10006f70 (`r19::summon_task_0202`): the record's
/// `(state, +0x2c)`, the answer of slots 20/411, the flag byte, `n`
/// targets with their flags, the distance base (slot 394 answers
/// `K - 1 + (base + call) % 3`).
fn summon_0202_setup(
    t: &mut Twin,
    f: u32,
    k: u32,
    (state, n2c): (u32, u32),
    (answer, flag, n, flags, base): (u32, u8, u32, [u32; 3], u32),
) -> u32 {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    summon_slots(t, k);
    t.set_u32(ANSWER_CELL, answer);
    let kk = f32::from_bits(t.u32(op(t, f, 0x396)));
    t.set_u32(DIST_CELL, base);
    rec(t, 394, 2, |_, _| 0x394);
    t.stub_st0(394, move |m, _| {
        let c = m.r32(row(394)) + m.r32(DIST_CELL);
        f64::from(kk) - 1.0 + f64::from(c % 3)
    });
    let list = op(t, f, 0x9f);
    empty_list(t, list);
    particle_with(t, t.img.rel32_target(f + 0x1a6), SCRATCH + 0x7a00);
    let pn = t.img.rel32_target(f + 0x5ee);
    let wrapper = t.img.rel32_target(pn + 0x88);
    t.set_u32(op(t, wrapper, 0x04), SCRATCH + 0x7f00);
    t.set_u32(SCRATCH + 0x7f00, 0x77);
    t.set_u32(op(t, pn, 0x45), SCRATCH + 0x7f10);
    let owner = SCRATCH + 0x7b00;
    t.set_u32(op(t, f, 0x5b4), owner);
    for j in 0..0x40 {
        t.set_u32(owner + 4 * j, (0.25 * j as f32 - 1.0).to_bits());
    }
    for o in [0x6ddu32, 0x716, 0x64, 0x6f, 0x608] {
        t.set_u32(op(t, f, o), 0x0a0a_0000 + o + k);
    }
    t.set_u8(op(t, f, 0x56), 0x20 | flag);
    t.set_u32(op(t, f, 0x2b8), n);
    // the object: its flags, model and object, a child of the list
    let obj = op(t, f, 0x689);
    t.write(obj, &[0x26; 0x100]);
    t.set_u8(obj + 0x90, 0x10 | flag);
    for (o, v) in [(4u32, list), (8, list), (0x14, list)] {
        t.set_u32(obj + o, v);
    }
    t.set_u32(list + 0xc, obj);
    t.set_u32(list + 0x10, obj);
    t.set_u16(list + 0x18, 1);
    // the targets: actor, id, the record before them
    for i in 0..3u32 {
        let tg = op(t, f, 0x2d7) + 0x98 * i;
        t.write(tg - 0x54, &[0x31; 0x98]);
        t.set_u8(tg - 0x54 + 0x90, (i as u8) & 1);
        t.set_u32(tg, 0x0b0b_0000 + 0x10 * i + k);
        t.set_u32(tg - 4, 0x0c0c_0000 + i);
    }
    for o in [0x32bu32, 0x359, 0x4db] {
        let at = op(t, f, o);
        for j in 0..4 {
            t.set_u32(at + 4 * j, (0.625 * j as f32 + 0.25 * o as f32).to_bits());
        }
    }
    junk_ops(
        t,
        f,
        &[
            0x10b, 0x115, 0x1a0, 0x29b, 0x248, 0x252, 0x25c, 0x66b, 0x77e, 0x785, 0x769,
        ],
    );
    let r = SCRATCH + 0x780;
    junk(t, r, 0x40, k);
    for (o, v) in [(0x18u32, state), (0x2c, n2c)] {
        t.set_u32(r + o, v);
    }
    for i in 0..3 {
        t.set_u32(r + 0x30 + 4 * i, flags[i as usize]);
    }
    summon_task_at(t, r)
}

/// `magic_0202` 0x10006f70 (`r19::summon_task_0202`): every state, the
/// count at its bound, both flag ways, targets flagged or not, near, at
/// and beyond the distance, none left and some left, no targets.
#[test]
fn summon_task_0202() {
    let Some(mut t) = twin(202) else { return };
    let f = t.at(0x6f70);
    for (k, (st, more)) in [
        ((0u32, 5u32), (0u32, 0u8, 2u32, [0u32; 3], 0u32)),
        ((10, 5), (0, 0, 2, [0; 3], 0)),
        ((10, 5), (0, 1, 2, [0; 3], 0)),
        ((11, 0x19), (0, 0, 2, [0x77; 3], 0)),
        ((11, 0x1a), (0, 0, 3, [0x77; 3], 0)),
        ((11, 0x1a), (0, 1, 0, [0x77; 3], 0)),
        ((12, 9), (0, 0, 3, [1, 0, 1], 0)),
        ((13, 9), (0, 0, 3, [1, 0, 1], 1)),
        ((13, 9), (0, 1, 3, [1, 1, 1], 2)),
        ((13, 9), (0, 0, 2, [0, 0, 1], 0)),
        ((13, 9), (0, 0, 0, [1, 1, 1], 0)),
        ((14, 9), (0, 0, 2, [0; 3], 0)),
        ((14, 9), (0, 1, 2, [0; 3], 0)),
        ((14, 9), (1, 0, 2, [0; 3], 0)),
        ((14, 9), (1, 1, 2, [0; 3], 0)),
        ((0x2710, 9), (0, 0, 2, [0; 3], 0)),
        ((5, 9), (0, 0, 2, [0; 3], 0)),
        ((15, 9), (0, 0, 2, [0; 3], 0)),
        ((0x270f, 9), (0, 0, 2, [0; 3], 0)),
        ((0x2711, 9), (0, 0, 2, [0; 3], 0)),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let task = summon_0202_setup(&mut t, f, k, st, more);
        scrub(&mut t);
        eprintln!("case {k}");
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    for (s, n) in [(394u32, 6u32), (239, 3), (262, 12), (20, 2), (411, 2)] {
        assert!(t.u32(row(s)) >= n, "slot {s}: {} calls", t.u32(row(s)));
    }
}

/// The particle node at `pn` started: its part start's lock and pointer.
fn part_start(t: &mut Twin, pn: u32) {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    let wrapper = t.img.rel32_target(pn + 0x88);
    t.set_u32(op(t, wrapper, 0x04), SCRATCH + 0x7f00);
    t.set_u32(SCRATCH + 0x7f00, 0x77);
    t.set_u32(op(t, pn, 0x45), SCRATCH + 0x7f10);
}

/// Floats at the globals the operands `ops` of `f` name (varied by `k`).
fn floats_at(t: &mut Twin, f: u32, k: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, (0.125 * o as f32 - 20.0 + k as f32).to_bits());
    }
}

/// `magic_0144` 0x10006780 (`r19::summon_task_0144`): the record's
/// `(state, +0x28, target)`.
fn summon_0144_setup(t: &mut Twin, f: u32, k: u32, (state, n28, target): (u32, u32, u32)) -> u32 {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    summon_slots(t, k);
    for o in [0x1f9u32, 0x4b7] {
        empty_list(t, op(t, f, o));
    }
    part_start(t, t.img.rel32_target(f + 0x4e2));
    let owner = SCRATCH + 0x7b00;
    t.set_u32(op(t, f, 0x46a), owner);
    for j in 0..0x40 {
        t.set_u32(owner + 4 * j, (0.25 * j as f32 - 1.0).to_bits());
    }
    t.set_u32(op(t, f, 0x1e5), 0x0a0a_0000 + k);
    floats_at(
        t,
        f,
        k,
        &[
            0x49, 0xc0, 0xe5, 0xed, 0x128, 0x151, 0x159, 0x16c, 0x17c, 0x184, 0x197, 0x1a7, 0x1af,
        ],
    );
    junk_ops(
        t,
        f,
        &[
            0xc5, 0xd1, 0xdd, 0x11d, 0x12e, 0x149, 0x161, 0x174, 0x18c, 0x19f, 0x1b7, 0x242, 0x2a3,
            0x328, 0x349, 0x353, 0x35d, 0x465, 0x4a6, 0x563, 0x59b, 0x5f4, 0x6d1, 0x75e, 0x83b,
            0x93a,
        ],
    );
    let r = SCRATCH + 0x780;
    junk(t, r, 0x40, k);
    for (o, v) in [(0x18u32, state), (0x28, n28), (0x24, target)] {
        t.set_u32(r + o, v);
    }
    summon_task_at(t, r)
}

/// `magic_0144` 0x10006780 (`r19::summon_task_0144`): every state, each
/// wait at its bound (the count is read after the entry's increment),
/// states the byte table sends to the default and states past it.
#[test]
fn summon_task_0144() {
    let Some(mut t) = twin(144) else { return };
    let f = t.at(0x6780);
    for (k, st) in [
        (0u32, 5u32, 1u32),
        (10, 5, 2),
        (11, 0x12, 0),
        (11, 0x13, 1),
        (20, 0x13, 2),
        (21, 0x17, 0),
        (21, 0x18, 1),
        (30, 0x18, 2),
        (31, 0x1c, 0),
        (31, 0x1d, 0),
        (40, 0x1d, 0),
        (41, 0x17, 0),
        (41, 0x18, 0),
        (42, 0x3f, 0),
        (42, 0x40, 0),
        (0x2710, 3, 0),
        (5, 3, 0),
        (12, 3, 0),
        (0x2a, 3, 0),
        (0x2b, 3, 0),
        (0x2711, 3, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let task = summon_0144_setup(&mut t, f, k, st);
        scrub(&mut t);
        eprintln!("case {k}");
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    for (s, n) in [(719u32, 2u32), (941, 6), (218, 1)] {
        assert!(t.u32(row(s)) >= n, "slot {s}: {} calls", t.u32(row(s)));
    }
}

/// `magic_0207` 0x10006b00 (`r19::summon_task_0207`): the record's
/// `(state, +0x2c)` and slot 411's answer.
fn summon_0207_setup(t: &mut Twin, f: u32, k: u32, (state, n2c): (u32, u32), answer: u32) -> u32 {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    summon_slots(t, k);
    rec(t, 701, 11, |_, a| 0x0701_0000 ^ a[4]);
    rec(t, 213, 2, |_, a| 0x0213_0000 ^ a[0]);
    t.set_u32(ANSWER_CELL, answer);
    empty_list(t, op(t, f, 0xdc));
    particle_with(t, t.img.rel32_target(f + 0x40e), SCRATCH + 0x7a00);
    let ena = t.img.rel32_target(f + 0x195);
    for o in [0x18u32, 0x1f] {
        empty_list(t, op(t, ena, o));
    }
    for o in [0xc8u32, 0x101, 0x44, 0x21b, 0x228, 0x96e] {
        t.set_u32(op(t, f, o), 0x0a0a_0000 + o + k);
    }
    t.set_u8(op(t, f, 0x73a), 0x5c + k as u8);
    floats_at(
        t,
        f,
        k,
        &[
            0x4a6, 0x4b1, 0x4df, 0x51a, 0x555, 0x568, 0x57b, 0x583, 0x593, 0x59b, 0x7ed, 0x7f8,
            0x81b, 0x848, 0x883, 0x8be, 0x8db, 0x8e3, 0x8f3, 0x8fb,
        ],
    );
    junk_ops(
        t,
        f,
        &[
            0x158, 0x15f, 0x1b1, 0x1b8, 0x245, 0x256, 0x26a, 0x27e, 0x2d1, 0x31b, 0x336, 0x35d,
            0x3a7, 0x3b8, 0x4b7, 0x4d4, 0x50f, 0x53e, 0x54a, 0x55d, 0x570, 0x58b, 0x5a3, 0x5ab,
            0x5ed, 0x5f7, 0x639, 0x67d, 0x687, 0x6bf, 0x749, 0x7a3, 0x7fd, 0x810, 0x83d, 0x878,
            0x8a7, 0x8b3, 0x8d0, 0x8eb, 0x903, 0x90b,
        ],
    );
    let r = SCRATCH + 0x780;
    junk(t, r, 0x40, k);
    for (o, v) in [(0x18u32, state), (0x2c, n2c)] {
        t.set_u32(r + o, v);
    }
    summon_task_at(t, r)
}

/// `magic_0207` 0x10006b00 (`r19::summon_task_0207`): every state, the
/// counts at their bounds, slot 411's two answers (the waits that run the
/// next state at once), states the byte table sends to the default and
/// states past it.
#[test]
fn summon_task_0207() {
    let Some(mut t) = twin(207) else { return };
    let f = t.at(0x6b00);
    for (k, (st, answer)) in [
        ((0u32, 5u32), 0u32),
        ((10, 5), 0),
        ((11, 5), 0),
        ((11, 5), 1),
        ((20, 5), 0),
        ((21, 2), 0),
        ((21, 3), 0),
        ((22, 5), 0),
        ((22, 5), 1),
        ((23, 3), 0),
        ((23, 4), 0),
        ((24, 0xf), 0),
        ((24, 0x10), 0),
        ((25, 5), 0),
        ((25, 5), 1),
        ((30, 5), 0),
        ((31, 5), 0),
        ((31, 5), 1),
        ((32, 5), 0),
        ((32, 5), 1),
        ((0x2710, 5), 0),
        ((5, 5), 0),
        ((0x20, 5), 0),
        ((0x21, 5), 0),
        ((0x2711, 5), 0),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let task = summon_0207_setup(&mut t, f, k, st, answer);
        scrub(&mut t);
        eprintln!("case {k}");
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    for (s, n) in [(701u32, 3u32), (411, 10), (239, 1), (213, 2), (215, 4)] {
        assert!(t.u32(row(s)) >= n, "slot {s}: {} calls", t.u32(row(s)));
    }
}

/// Where `summon_0106_setup` keeps how many target bytes slot 288 gives.
const IDS_CELL: u32 = SCRATCH + 0xa4;

/// `magic_0106` 0x10006c30 (`r19::summon_task_0106`): the record's
/// `(state, +0x2c)`, the flag `R + 0x74`, the kept ids `R + 0x30` (count
/// `R + 0x70`) and how many bytes slot 288 gives.
fn summon_0106_setup(
    t: &mut Twin,
    f: u32,
    k: u32,
    (state, n2c): (u32, u32),
    (flag, ids, n288): (u32, u16, u32),
) -> u32 {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    summon_slots(t, k);
    for (s, n) in [(296u32, 2usize), (878, 1), (684, 1)] {
        rec(t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0) ^ 1
        });
    }
    rec(t, 845, 0, |_, _| 0x845);
    rec(t, 531, 2, move |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, 0.75 * i as f32 + a[0] as f32 - k as f32);
        }
        0x531
    });
    let bytes = SCRATCH + 0x320;
    t.write(bytes, &[3, 9, 4, 11, 2, 0]);
    t.set_u32(IDS_CELL, n288);
    rec(t, 288, 2, move |m, a| {
        m.w32(a[1], m.r32(IDS_CELL));
        bytes
    });
    for o in [0x5au32, 0x109] {
        empty_list(t, op(t, f, o));
    }
    for (call, o) in [(0x3b3u32, 0x374u32), (0x650, 0x611), (0x79f, 0x753)] {
        part_start(t, t.img.rel32_target(f + call));
        let owner = SCRATCH + 0x7b00;
        t.set_u32(op(t, f, o), owner);
        for j in 0..0x40 {
            t.set_u32(owner + 4 * j, (0.25 * j as f32 - 1.0).to_bits());
        }
    }
    let lk = t.img.rel32_target(f + 0x199);
    t.set_u32(op(t, lk, 0x1), SCRATCH + 0x7f20);
    t.set_u32(SCRATCH + 0x7f20, 0x0123_4567);
    let ce = t.img.rel32_target(f + 0x98);
    t.set_u32(op(t, ce, 0x48), 2);
    t.write(op(t, ce, 0x55), &[5, 7]);
    // the targets: 3 and 4 are known
    t.set_u32(op(t, f, 0x4fe), 2);
    for (j, id) in [(0u32, 3u32), (1, 4)] {
        t.set_u32(op(t, f, 0x50c) + 0x98 * j, id);
    }
    t.set_u32(op(t, f, 0x46), 0x0a0a_0000 + k);
    t.set_u32(op(t, f, 0x213), 0x0b0b_0000 + k);
    floats_at(
        t,
        f,
        k,
        &[
            0x24, 0x239, 0x261, 0x29c, 0x2d7, 0x2fb, 0x30e, 0x31e, 0x326, 0x339, 0x3da, 0x40a,
            0x445, 0x476, 0x489, 0x49c, 0x4a4, 0x4b4, 0x4bc, 0x7a7,
        ],
    );
    junk_ops(
        t,
        f,
        &[
            0x23e, 0x256, 0x291, 0x2c0, 0x2cc, 0x2f0, 0x303, 0x316, 0x32e, 0x341, 0x3e0, 0x3e9,
            0x3f3, 0x3ff, 0x410, 0x43a, 0x45f, 0x46b, 0x47e, 0x491, 0x4ac, 0x4c4, 0x4cc, 0x596,
            0x5aa, 0x699, 0x6a0, 0x700, 0x707, 0x759, 0x760, 0xe4, 0xf0, 0x7da,
        ],
    );
    let r = SCRATCH + 0x780;
    junk(t, r, 0x80, k);
    for (o, v) in [(0x18u32, state), (0x2c, n2c), (0x74, flag)] {
        t.set_u32(r + o, v);
    }
    t.set_u16(r + 0x70, ids);
    summon_task_at(t, r)
}

/// `magic_0106` 0x10006c30 (`r19::summon_task_0106`): every state, the
/// counts at their bounds, ids known and new, none, the flag's two ways,
/// states the byte table sends to the default and states past it.
#[test]
fn summon_task_0106() {
    let Some(mut t) = twin(106) else { return };
    let f = t.at(0x6c30);
    for (k, (st, more)) in [
        ((0u32, 5u32), (1u32, 0u16, 0u32)),
        ((10, 5), (1, 0, 0)),
        ((20, 5), (0, 0, 0)),
        ((21, 0x79), (0, 0, 0)),
        ((21, 0x7a), (0, 0, 0)),
        ((30, 5), (0, 0, 0)),
        ((31, 0x59), (0, 0, 0)),
        ((31, 0x5a), (0, 0, 0)),
        ((40, 5), (0, 0, 0)),
        ((41, 9), (0, 0, 5)),
        ((41, 0xa), (0, 3, 5)),
        ((41, 0xa), (0, 3, 0)),
        ((42, 1), (1, 2, 0)),
        ((42, 0x27), (1, 2, 0)),
        ((42, 0x28), (1, 3, 0)),
        ((42, 0x28), (0, 0, 0)),
        ((50, 5), (1, 2, 0)),
        ((51, 0xe), (1, 2, 0)),
        ((51, 0xf), (1, 3, 0)),
        ((51, 0xf), (0, 0, 0)),
        ((52, 0xe), (1, 2, 0)),
        ((52, 0xf), (1, 2, 0)),
        ((0x2710, 5), (0, 0, 0)),
        ((0x2710, 5), (1, 0, 0)),
        ((5, 5), (1, 0, 0)),
        ((0x34, 5), (0, 0, 0)),
        ((0x35, 5), (1, 0, 0)),
        ((0x2711, 5), (0, 0, 0)),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let task = summon_0106_setup(&mut t, f, k, st, more);
        scrub(&mut t);
        eprintln!("case {k}");
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    for (s, n) in [
        (878u32, 2u32),
        (541, 4),
        (296, 5),
        (845, 6),
        (288, 2),
        (239, 1),
    ] {
        assert!(t.u32(row(s)) >= n, "slot {s}: {} calls", t.u32(row(s)));
    }
}
